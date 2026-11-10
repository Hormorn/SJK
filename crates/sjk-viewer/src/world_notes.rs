//! World notes: in-game annotations of surfaces and objects for whoever fixes them.
//!
//! `worldnote` selects the world surface under the crosshair, or the mover whose bounds
//! the view ray meets first, and names it in a centre print. A second `worldnote` within
//! [`CONFIRM_MS`] confirms the selection and opens a note line in the chat composer.
//! Enter appends the note with the selection to `notes.jsonl` (one JSON object per
//! line) and a readable line to `notes.txt` in the config directory, and takes a
//! JPEG screenshot of the same view named after the note. With the SJK identity on,
//! the note also goes to the hub for the SJK team (`PROTOCOL.md` in the hub, "World
//! notes"), followed by a smaller copy of the screenshot. Escape drops the note.
//!
//! While a selection waits or its note is written, its outline is drawn over the view
//! as flickering green dots along the surface's triangle edges (a mover's bounds),
//! projected each frame, at most [`HIGHLIGHT_DOTS`] of them, over a green scanline shade
//! of its triangles (thin horizontal strips with a wave running down them, at most
//! [`FILL_STRIPS`]; the HUD has no polygon, so the fill is rasterised into rectangles).
//!
//! A selection records what a fix needs: the map, the camera pose and a `setviewpos`
//! back to it, the hit point and normal, the BSP draw surface (index, shader, kind,
//! lightmap or vertex lighting, BSP material), the collision trace's surface flags, and
//! the nearest map entity (an inline model whose bounds hold the hit, or the closest
//! origin within [`ENTITY_RADIUS`]). When a remap (the map's, the server's or a local
//! one) draws another shader over the surface, the note names that shader too: what
//! the player saw is the remap's target, not the map's own shader. Every remap active
//! when the note is taken goes with it ([`Remap`]), with its source.
//!
//! A model drawn from the map's entities (an MD3 such as a `misc_model_static` prop,
//! which is not in the world's triangles) is selected when the view ray meets one of its
//! triangles before the world ([`pick_model`]): the note then names the model's path, its
//! entity and placement, and every surface's shader with its remap, and the outline
//! traces the model. While the note is written, Clear model in the dialog drops the model
//! for the surface behind it, and a click on the scene round the dialog selects what is
//! under the pointer instead ([`Notes::pick_at`]).

use crate::camera_uniform::CameraUniform;
use glam::Vec3;
use models::{ModelCache, pick_model};
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use sjk_vfs::VirtualFileSystem;
use std::borrow::Cow;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[path = "world_notes_models.rs"]
mod models;

/// How long a selection waits for its confirming press.
pub(crate) const CONFIRM_MS: u128 = 20_000;
/// Farthest a selection reaches.
const REACH: f32 = 16_384.0;
/// Entities whose origin lies this close to the hit are named when no inline model
/// holds it.
const ENTITY_RADIUS: f32 = 160.0;
/// `CONTENTS_SOLID | CONTENTS_PLAYERCLIP`-style mask of the collision trace: anything a
/// player could stand against.
const COLLISION_MASK: u32 = 1 | 0x10000;
/// `SURF_NODRAW`: the side the collision trace met draws nothing.
const SURF_NODRAW: u32 = 0x0020_0000;
/// How much nearer than the drawn surface a nodraw side must be to be selected instead.
const FACE_MARGIN: f32 = 1.0;
/// How far from the sky surface the ray met its brush's sky side may lie and still be
/// the side that surface was made from.
const SKY_MARGIN: f32 = 2.0;
/// JKA's standing view height: `setviewpos` places the origin, the eye sits above it.
const VIEW_HEIGHT: f32 = 36.0;
/// Most edges an outline keeps, and most dots it draws a frame (the HUD's shapes share
/// 16,384 vertices, six a dot).
const HIGHLIGHT_EDGES: usize = 512;
pub(crate) const HIGHLIGHT_DOTS: usize = 600;
/// Most dots the map editor's hover outline draws a frame (in the outline's list, before
/// the selection's).
pub(crate) const HOVER_DOTS: usize = 240;
/// Most triangles a shade keeps, and most strips it draws a frame.
const FILL_TRIANGLES: usize = 512;
pub(crate) const FILL_STRIPS: usize = 800;
/// Clip-space `w` a shaded triangle is cut at: what lies nearer the eye is not drawn.
const NEAR_W: f32 = 1.0;

/// What `worldnote` selected.
#[derive(Clone, Debug)]
pub(crate) struct Selection {
    map: String,
    pose: String,
    eye: Vec3,
    forward: Vec3,
    hit: Vec3,
    normal: Vec3,
    distance: f32,
    surface: Option<Surface>,
    entity: Option<Entity>,
    collision: Option<Collision>,
    /// The caulk or nodraw brush side the ray met before any drawn surface: edits may
    /// draw it ([`crate::map_edits::faces`]).
    face: Option<crate::map_edits::faces::Face>,
    /// The outline drawn while selected (`Notes::draw_highlight`).
    edges: Vec<[Vec3; 2]>,
    /// The triangles shaded while selected.
    triangles: Vec<[Vec3; 3]>,
    selected: Instant,
    /// The model the view ray met before the world, if any: the note is about it.
    model: Option<Model>,
    /// The shader remaps active when the selection was made.
    remaps: Vec<Remap>,
}

/// A shader remap active when a note was taken.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Remap {
    /// `map` (a worldspawn key), `server` or `local` (the console's `remapShader`).
    pub(crate) source: &'static str,
    /// The shader the map or model names.
    pub(crate) from: String,
    /// The shader drawn instead.
    pub(crate) to: String,
}

/// The shader a remap in `remaps` draws instead of `name`, when one draws another.
fn remap_of(remaps: &[Remap], name: &str) -> Option<String> {
    let name = sjk_client::shader_name(name)?;
    remaps
        .iter()
        .find(|remap| remap.from == name && remap.to != name)
        .map(|remap| remap.to.clone())
}

/// A model selected for a note: a map entity drawing an MD3, met on its triangles.
#[derive(Clone, Debug)]
struct Model {
    /// The entity, its `model` key the model's path.
    entity: Entity,
    /// Where it stands, pitch, yaw and roll, and the scale on each axis: as its outline,
    /// hit and normal were made ([`Model::move_to`] moves them along).
    origin: Vec3,
    angles: [f32; 3],
    scale: [f32; 3],
    /// The entity's skin key, or empty.
    skin: String,
    shaders: Vec<ModelShader>,
    hit: Vec3,
    normal: Vec3,
    distance: f32,
    edges: Vec<[Vec3; 2]>,
    triangles: Vec<[Vec3; 3]>,
}

impl Model {
    /// Its outline, shade and hit carried to `origin`, `angles` and `scale` (a placed
    /// model the map editor moved, turned or scaled while selected).
    fn move_to(&mut self, origin: Vec3, angles: [f32; 3], scale: [f32; 3]) {
        if self.origin == origin && self.angles == angles && self.scale == scale {
            return;
        }
        let rotation = |angles: [f32; 3]| {
            glam::Quat::from_array(sjk_client::legacy_angles_to_quaternion(angles))
        };
        let back = rotation(self.angles).inverse();
        let turn = rotation(angles);
        let (from, to) = (self.origin, origin);
        let from_scale = Vec3::from_array(self.scale).max(Vec3::splat(1e-6));
        let to_scale = Vec3::from_array(scale);
        let place = |point: Vec3| to + turn * (to_scale * (back * (point - from) / from_scale));
        for edge in &mut self.edges {
            *edge = edge.map(place);
        }
        for triangle in &mut self.triangles {
            *triangle = triangle.map(place);
        }
        self.hit = place(self.hit);
        self.normal = (turn * (back * self.normal)).normalize_or_zero();
        self.origin = origin;
        self.angles = angles;
        self.scale = scale;
    }
}

/// One surface of a selected model and the shader it draws.
#[derive(Clone, Debug)]
struct ModelShader {
    surface: String,
    shader: String,
    /// The shader a remap draws instead.
    remapped: Option<String>,
}

#[derive(Clone, Debug)]
struct Surface {
    index: usize,
    shader: String,
    /// The shader a remap draws instead of [`Surface::shader`].
    remapped: Option<String>,
    kind: &'static str,
    lightmap: i32,
    vertices: usize,
    material: u32,
}

#[derive(Clone, Debug)]
struct Entity {
    number: usize,
    classname: String,
    targetname: String,
    model: String,
    origin: String,
    /// The inline model's shaders, for a mover the ray met.
    shaders: Vec<String>,
}

#[derive(Clone, Debug)]
struct Collision {
    distance: f32,
    surface_flags: u32,
    content_flags: u32,
}

impl Selection {
    /// One line naming the selection, for the centre print and the note's subject.
    pub(crate) fn summary(&self) -> String {
        if let Some(model) = &self.model {
            let first = model.shaders.first();
            let more = model.shaders.len().saturating_sub(1);
            return format!(
                "{} {} ({}{}{}), {:.0} units",
                model.entity.classname,
                model.entity.model,
                first.map_or("no shader", |shader| shader.shader.as_str()),
                first
                    .and_then(|shader| shader.remapped.as_ref())
                    .map_or_else(String::new, |target| format!(" remapped to {target}")),
                match more {
                    0 => String::new(),
                    1 => ", 1 more shader".to_owned(),
                    more => format!(", {more} more shaders"),
                },
                model.distance
            );
        }
        if let Some(face) = &self.face {
            return format!(
                "{} (brush {} side {}, {}), {:.0} units",
                face.shader,
                face.brush,
                face.side,
                if face.sky {
                    "sky"
                } else {
                    "drawn by no surface"
                },
                self.distance
            );
        }
        let what = match (&self.surface, &self.entity) {
            (_, Some(entity)) if !entity.shaders.is_empty() => format!(
                "{} {} ({})",
                entity.classname,
                entity.model,
                entity.shaders.first().map_or("", String::as_str)
            ),
            (Some(surface), _) => format!(
                "{}{} (surface {}, {})",
                surface.shader,
                surface
                    .remapped
                    .as_ref()
                    .map_or_else(String::new, |target| format!(" remapped to {target}")),
                surface.index,
                lighting(surface.lightmap)
            ),
            (None, Some(entity)) => {
                let mut what = entity.classname.clone();
                if !entity.targetname.is_empty() {
                    what.push_str(&format!(" \"{}\"", entity.targetname));
                }
                if !entity.origin.is_empty() {
                    what.push_str(&format!(" at {}", entity.origin));
                }
                what
            }
            (None, None) => "nothing".to_owned(),
        };
        format!("{what}, {:.0} units", self.distance)
    }

    /// The placed edit's id when this is a model a map edit placed (the pick's entity
    /// for it is named `edit <id>`, `crate::map_edits::State::pick_entities`).
    pub(crate) fn placed_id(&self) -> Option<u32> {
        let model = self.model.as_ref()?;
        if model.entity.classname != crate::map_edits::PLACED_CLASS {
            return None;
        }
        model.entity.targetname.strip_prefix("edit ")?.parse().ok()
    }

    /// What it is in the map editor's multiple selection, when it may be in one: a
    /// placed model, a surface or a hidden face.
    pub(crate) fn member(&self) -> Option<Member> {
        if self.model.is_some() {
            return self.placed_id().map(Member::Placed);
        }
        self.edit_target().member()
    }

    /// What a map edit would act on.
    pub(crate) fn edit_target(&self) -> EditTarget {
        if let Some(model) = &self.model {
            return EditTarget::Model {
                number: model.entity.number,
                classname: model.entity.classname.clone(),
                model: model.entity.model.clone(),
                shader: model.shaders.first().map(|shader| shader.shader.clone()),
            };
        }
        if let Some(face) = &self.face {
            return EditTarget::Face {
                brush: face.brush,
                side: face.side,
                shader: face.shader.clone(),
                sky: face.sky,
            };
        }
        match (&self.surface, &self.entity) {
            (_, Some(entity)) if !entity.shaders.is_empty() => EditTarget::Other,
            (Some(surface), _) => EditTarget::Surface {
                index: surface.index,
                shader: surface.shader.clone(),
                kind: surface.kind,
            },
            _ => EditTarget::Other,
        }
    }

    /// The view's forward direction when the selection was made.
    pub(crate) fn forward(&self) -> Vec3 {
        self.forward
    }

    /// The point aimed at and its normal.
    pub(crate) fn aim(&self) -> (Vec3, Vec3) {
        let (hit, normal, _) = self.target();
        (hit, normal)
    }

    /// The selected model's path, for the dialog's Clear model.
    pub(crate) fn model_path(&self) -> Option<String> {
        self.model.as_ref().map(|model| model.entity.model.clone())
    }

    /// The aimed point, its normal and its distance: the model's when there is one.
    fn target(&self) -> (Vec3, Vec3, f32) {
        self.model
            .as_ref()
            .map_or((self.hit, self.normal, self.distance), |model| {
                (model.hit, model.normal, model.distance)
            })
    }

    /// The corners of what is selected (its outline and shade, and the point aimed at):
    /// what the map editor frames (`mapedit_frame`).
    pub(crate) fn extent_points(&self) -> impl Iterator<Item = Vec3> + '_ {
        let (edges, triangles) = self.outline();
        let (hit, _, _) = self.target();
        edges
            .iter()
            .flatten()
            .chain(triangles.iter().flatten())
            .copied()
            .chain(std::iter::once(hit))
    }

    /// The outline and the shade drawn while selected: the model's when there is one.
    fn outline(&self) -> (&[[Vec3; 2]], &[[Vec3; 3]]) {
        self.model.as_ref().map_or(
            (self.edges.as_slice(), self.triangles.as_slice()),
            |model| (model.edges.as_slice(), model.triangles.as_slice()),
        )
    }

    /// The shaders the note is about, as remaps name them: the surface's, or the
    /// model's surfaces'.
    fn noted_shaders(&self) -> Vec<String> {
        if let Some(face) = self.face.as_ref().filter(|_| self.model.is_none()) {
            return sjk_client::shader_name(&face.shader).into_iter().collect();
        }
        let names: Vec<&str> = match (&self.model, &self.surface, &self.entity) {
            (Some(model), _, _) => model.shaders.iter().map(|s| s.shader.as_str()).collect(),
            (None, Some(surface), _) => vec![surface.shader.as_str()],
            (None, None, Some(entity)) => entity.shaders.iter().map(String::as_str).collect(),
            (None, None, None) => Vec::new(),
        };
        names
            .into_iter()
            .filter_map(sjk_client::shader_name)
            .collect()
    }

    /// The note `text` about this selection as the hub takes it; the build and the
    /// server are the caller's to fill.
    fn world_note(&self, text: String) -> sjk_identity::WorldNote {
        use sjk_identity::report::{
            NOTE_MODEL_PATH_MAX, NOTE_MODEL_SHADERS_MAX, NOTE_REMAPS_MAX, NOTE_SHADER_MAX, field,
        };
        let face = self.face.as_ref().filter(|_| self.model.is_none());
        let (shader, remapped) = match (&self.model, &self.surface, &self.entity) {
            _ if face.is_some() => (face.map_or("", |face| face.shader.as_str()), None),
            (Some(model), _, _) => model.shaders.first().map_or(("", None), |shader| {
                (shader.shader.as_str(), shader.remapped.as_deref())
            }),
            (None, Some(surface), _) => (surface.shader.as_str(), surface.remapped.as_deref()),
            (None, None, Some(entity)) => (entity.shaders.first().map_or("", String::as_str), None),
            (None, None, None) => ("", None),
        };
        let vector = |v: Vec3| [v.x, v.y, v.z];
        let (hit, normal, distance) = self.target();
        // The remaps of the noted shaders first, then the others, as many as the hub keeps.
        let noted = self.noted_shaders();
        let (mut remaps, others): (Vec<&Remap>, Vec<&Remap>) = self
            .remaps
            .iter()
            .partition(|remap| noted.contains(&remap.from));
        remaps.extend(others);
        let remaps = remaps
            .into_iter()
            .map(|remap| sjk_identity::NoteRemap {
                source: remap.source.to_owned(),
                from: field(&remap.from, NOTE_SHADER_MAX),
                to: field(&remap.to, NOTE_SHADER_MAX),
            })
            .filter(|remap| !remap.from.is_empty() && !remap.to.is_empty())
            .take(NOTE_REMAPS_MAX)
            .collect();
        let model = self.model.as_ref().map(|model| sjk_identity::NoteModel {
            path: field(&model.entity.model, NOTE_MODEL_PATH_MAX),
            classname: field(&model.entity.classname, 64),
            entity: u32::try_from(model.entity.number).ok(),
            origin: parse_vector(Some(&model.entity.origin)).map(vector),
            skin: field(&model.skin, NOTE_MODEL_PATH_MAX),
            shaders: model
                .shaders
                .iter()
                .take(NOTE_MODEL_SHADERS_MAX)
                .map(|shader| sjk_identity::NoteModelShader {
                    surface: field(&shader.surface, NOTE_SHADER_MAX),
                    shader: field(&shader.shader, NOTE_SHADER_MAX),
                    remapped: shader
                        .remapped
                        .as_deref()
                        .map_or_else(String::new, |name| field(name, NOTE_SHADER_MAX)),
                })
                .collect(),
        });
        let surface = self.surface.as_ref().filter(|_| self.model.is_none());
        sjk_identity::WorldNote {
            text,
            map: field(&self.map, 64),
            view: Some([
                self.eye.x.round(),
                self.eye.y.round(),
                (self.eye.z - VIEW_HEIGHT).round(),
                self.yaw().round(),
            ]),
            hit: Some(vector(hit)),
            normal: Some(vector(normal)),
            shader: field(shader, NOTE_SHADER_MAX),
            remapped: remapped.map_or_else(String::new, |name| field(name, NOTE_SHADER_MAX)),
            surface: surface.and_then(|s| u32::try_from(s.index).ok()),
            lighting: surface.map_or_else(String::new, |s| lighting(s.lightmap).to_owned()),
            distance: Some(distance.round()),
            entity: self
                .model
                .as_ref()
                .map(|model| &model.entity)
                .or(self.entity.as_ref())
                .map_or_else(String::new, |e| field(&e.classname, 64)),
            remaps,
            model,
            ..sjk_identity::WorldNote::default()
        }
    }

    fn yaw(&self) -> f32 {
        self.forward
            .y
            .atan2(self.forward.x)
            .to_degrees()
            .rem_euclid(360.0)
    }

    fn fresh(&self, now: Instant) -> bool {
        now.duration_since(self.selected).as_millis() <= CONFIRM_MS
    }

    /// `setviewpos` that puts the eye back where the selection was made.
    fn setviewpos(&self) -> String {
        let yaw = self.yaw();
        format!(
            "setviewpos {:.0} {:.0} {:.0} {:.0}",
            self.eye.x,
            self.eye.y,
            self.eye.z - VIEW_HEIGHT,
            yaw
        )
    }

    /// The note's line of `notes.jsonl`. With a model, `surface` is left out (the note is
    /// not about the world behind it) and `entity` and `model` name the model's entity.
    fn json(&self, note: &str, time: &str, screenshot: &str) -> serde_json::Value {
        let vector = |v: Vec3| serde_json::json!([round(v.x), round(v.y), round(v.z)]);
        let entity_json = |e: &Entity| {
            serde_json::json!({
                "number": e.number,
                "classname": e.classname,
                "targetname": e.targetname,
                "model": e.model,
                "origin": e.origin,
                "shaders": e.shaders,
            })
        };
        let (hit, normal, distance) = self.target();
        let surface = self.surface.as_ref().filter(|_| self.model.is_none());
        serde_json::json!({
            "time": time,
            "map": self.map,
            "note": note,
            "selection": self.summary(),
            "setviewpos": self.setviewpos(),
            "pose": self.pose,
            "hit": vector(hit),
            "normal": vector(normal),
            "distance": round(distance),
            "surface": surface.map(|s| serde_json::json!({
                "index": s.index,
                "shader": s.shader,
                "remapped_to": s.remapped,
                "kind": s.kind,
                "lightmap": s.lightmap,
                "lighting": lighting(s.lightmap),
                "vertices": s.vertices,
                "bsp_material": s.material,
            })),
            "entity": self
                .model
                .as_ref()
                .map(|model| &model.entity)
                .or(self.entity.as_ref())
                .map(entity_json),
            "model": self.model.as_ref().map(|model| serde_json::json!({
                "path": model.entity.model,
                "classname": model.entity.classname,
                "entity": model.entity.number,
                "targetname": model.entity.targetname,
                "origin": model.entity.origin,
                "angles": model.angles,
                "scale": model.scale,
                "skin": model.skin,
                "shaders": model.shaders.iter().map(|shader| serde_json::json!({
                    "surface": shader.surface,
                    "shader": shader.shader,
                    "remapped_to": shader.remapped,
                })).collect::<Vec<_>>(),
            })),
            "remaps": self.remaps.iter().map(|remap| serde_json::json!({
                "source": remap.source,
                "from": remap.from,
                "to": remap.to,
            })).collect::<Vec<_>>(),
            "collision": self.collision.as_ref().map(|c| serde_json::json!({
                "distance": round(c.distance),
                "surface_flags": format!("{:#x}", c.surface_flags),
                "content_flags": format!("{:#x}", c.content_flags),
            })),
            "screenshot": screenshot,
        })
    }
}

fn lighting(lightmap: i32) -> &'static str {
    match lightmap {
        0.. => "lightmapped",
        -3 => "vertex-lit",
        -2 => "white image",
        _ => "unlit",
    }
}

fn round(value: f32) -> f64 {
    (f64::from(value) * 10.0).round() / 10.0
}

/// The selection under the ray from the eye of `camera` along `direction` (its view, or
/// through a point of the screen): the nearest draw surface of the world, unless a
/// mover's bounds come first. Models are [`pick_model`]'s.
pub(crate) fn pick(
    bsp: &Bsp,
    surfaces: &crate::decal_marks::DecalSurfaces,
    scratch: &mut TraceScratch,
    camera: &CameraUniform,
    map: &str,
    direction: Vec3,
) -> Option<Selection> {
    let eye = Vec3::from_array(camera.camera_position);
    let view = Vec3::from_array(camera.view_forward).normalize_or_zero();
    // `setviewpos` keeps the view's yaw; the ray may go elsewhere.
    let forward = if view == Vec3::ZERO { direction } else { view };
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return None;
    }
    let world = surfaces.ray_hit(eye, direction, REACH);
    let entities = sjk_entity::parse_entity_lump(bsp.entities()).unwrap_or_default();
    // Movers are not in the world's triangles: the first inline model box the ray meets.
    let mut mover = None;
    for (number, entity) in entities.iter().enumerate() {
        let Some(index) = entity
            .get("model")
            .and_then(|model| model.strip_prefix('*'))
            .and_then(|index| index.parse::<usize>().ok())
        else {
            continue;
        };
        let Some(model) = bsp.render().inline_model(index) else {
            continue;
        };
        let origin = parse_vector(entity.get("origin")).unwrap_or(Vec3::ZERO);
        let low = Vec3::from_array(model.minimums) + origin;
        let high = Vec3::from_array(model.maximums) + origin;
        let Some(distance) = ray_box(eye, direction, low, high) else {
            continue;
        };
        if world.as_ref().is_some_and(|hit| hit.distance < distance)
            || mover
                .as_ref()
                .is_some_and(|(_, _, nearest)| *nearest <= distance)
        {
            continue;
        }
        mover = Some((number, index, distance));
    }
    let trace = bsp.trace_box_with(
        scratch,
        eye.to_array(),
        (eye + direction * REACH).to_array(),
        Aabb::new([0.0; 3], [0.0; 3]).expect("point bounds are valid"),
        COLLISION_MASK,
    );
    let collision = (trace.fraction < 1.0).then_some(Collision {
        distance: trace.fraction * REACH,
        surface_flags: trace.surface_flags,
        content_flags: trace.content_flags,
    });
    let render = bsp.render();
    let shader_name = |index: usize| {
        bsp.shaders().get(index).map_or_else(
            || format!("shader {index}"),
            |s| s.name_lossy().into_owned(),
        )
    };
    let entity_of = |number: usize, shaders: Vec<String>| {
        let entity = &entities[number];
        let text = |key: &str| entity.get(key).unwrap_or("").to_owned();
        Entity {
            number,
            classname: text("classname"),
            targetname: text("targetname"),
            model: text("model"),
            origin: text("origin"),
            shaders,
        }
    };
    let selection =
        |hit: Vec3,
         normal: Vec3,
         distance: f32,
         surface,
         entity,
         (edges, triangles): (Vec<[Vec3; 2]>, Vec<[Vec3; 3]>)| Selection {
            map: map.to_owned(),
            pose: camera.viewpos(),
            eye,
            forward,
            hit,
            normal,
            distance,
            surface,
            entity,
            collision: collision.clone(),
            face: None,
            edges,
            triangles,
            selected: Instant::now(),
            model: None,
            remaps: Vec::new(),
        };
    if let Some((number, index, distance)) = mover {
        let shaders = render
            .inline_model(index)
            .map(|model| {
                let mut names: Vec<String> = model
                    .surfaces
                    .clone()
                    .filter_map(|surface| render.surfaces().get(surface))
                    .map(|surface| shader_name(surface.shader))
                    .collect();
                names.sort();
                names.dedup();
                names
            })
            .unwrap_or_default();
        let hit = eye + direction * distance;
        let outline = render
            .inline_model(index)
            .map_or_else(Default::default, |model| {
                let origin = parse_vector(entities[number].get("origin")).unwrap_or(Vec3::ZERO);
                let (low, high) = (
                    Vec3::from_array(model.minimums) + origin,
                    Vec3::from_array(model.maximums) + origin,
                );
                (box_edges(low, high), box_triangles(low, high))
            });
        return Some(selection(
            hit,
            -direction,
            distance,
            None,
            Some(entity_of(number, shaders)),
            outline,
        ));
    }
    // A caulk or nodraw side nearer than any drawn surface: the map draws nothing there
    // (a hole in the world, or a face edits drew), only the brush is.
    let nodraw_first = collision.as_ref().is_some_and(|collision| {
        collision.surface_flags & SURF_NODRAW != 0
            && world
                .as_ref()
                .is_none_or(|hit| collision.distance + FACE_MARGIN < hit.distance)
    });
    // The sky drawn first: the sky side of the brush behind it, which edits may cover
    // with a texture or whose sky they may swap (the sky's surface itself is no edit's).
    let sky_first = world.as_ref().is_some_and(|hit| {
        render
            .surfaces()
            .get(hit.surface)
            .and_then(|surface| bsp.shaders().get(surface.shader))
            .is_some_and(|shader| shader.surface_flags & crate::map_edits::faces::SURF_SKY != 0)
    });
    let face = if nodraw_first {
        // The pick goes through clips and triggers; what it finds behind them must still
        // be nearer than the drawn surface (a merged model has no brush to stop it).
        crate::map_edits::faces::pick(bsp, eye, direction, REACH).filter(|face| {
            world
                .as_ref()
                .is_none_or(|hit| face.distance + FACE_MARGIN < hit.distance)
        })
    } else if sky_first {
        crate::map_edits::faces::pick(bsp, eye, direction, REACH).filter(|face| {
            face.sky
                && world
                    .as_ref()
                    .is_some_and(|hit| (hit.distance - face.distance).abs() <= SKY_MARGIN)
        })
    } else {
        None
    };
    if let Some(face) = face {
        let point = eye + direction * face.distance;
        let corners: Vec<Vec3> = face.polygon.iter().map(|p| Vec3::from_array(*p)).collect();
        let edges = (0..corners.len())
            .map(|i| [corners[i], corners[(i + 1) % corners.len()]])
            .collect();
        let triangles = (2..corners.len())
            .map(|i| [corners[0], corners[i - 1], corners[i]])
            .collect();
        let entity = nearest_entity(&entities, point).map(|number| entity_of(number, Vec::new()));
        let mut selected = selection(
            point,
            face.normal,
            face.distance,
            None,
            entity,
            (edges, triangles),
        );
        selected.face = Some(face);
        return Some(selected);
    }
    let hit = world?;
    let point = eye + direction * hit.distance;
    let surface = render.surfaces().get(hit.surface).map(|surface| {
        let shader = bsp.shaders().get(surface.shader);
        Surface {
            index: hit.surface,
            shader: shader_name(surface.shader),
            remapped: None,
            kind: surface_kind(surface.kind),
            lightmap: surface.lightmaps[0],
            vertices: surface.vertices.len(),
            material: shader.map_or(0, |s| s.surface_flags & 0x1f),
        }
    });
    let entity = nearest_entity(&entities, point).map(|number| entity_of(number, Vec::new()));
    let outline = (
        surfaces.surface_edges(hit.surface, HIGHLIGHT_EDGES),
        surfaces.surface_triangles(hit.surface, FILL_TRIANGLES),
    );
    Some(selection(
        point,
        hit.normal,
        hit.distance,
        surface,
        entity,
        outline,
    ))
}

/// The entity, other than the world, whose origin is nearest `point` within
/// [`ENTITY_RADIUS`].
fn nearest_entity(entities: &[sjk_entity::Entity], point: Vec3) -> Option<usize> {
    entities
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(number, entity)| {
            let origin = parse_vector(entity.get("origin"))?;
            let distance = origin.distance(point);
            (distance <= ENTITY_RADIUS).then_some((number, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(number, _)| number)
}

fn parse_vector(text: Option<&str>) -> Option<Vec3> {
    let mut parts = text?
        .split_whitespace()
        .map(|part| part.parse::<f32>().ok());
    Some(Vec3::new(parts.next()??, parts.next()??, parts.next()??))
}

/// The twelve edges of a box.
fn box_edges(low: Vec3, high: Vec3) -> Vec<[Vec3; 2]> {
    let corner = |i: usize| {
        Vec3::new(
            if i & 1 == 0 { low.x } else { high.x },
            if i & 2 == 0 { low.y } else { high.y },
            if i & 4 == 0 { low.z } else { high.z },
        )
    };
    (0..8)
        .flat_map(|i| [1, 2, 4].map(|bit| (i, i | bit)))
        .filter(|(from, to)| from != to)
        .map(|(from, to)| [corner(from), corner(to)])
        .collect()
}

/// The twelve triangles of a box's faces.
fn box_triangles(low: Vec3, high: Vec3) -> Vec<[Vec3; 3]> {
    let corner = |x: usize, y: usize, z: usize| {
        Vec3::new(
            if x == 0 { low.x } else { high.x },
            if y == 0 { low.y } else { high.y },
            if z == 0 { low.z } else { high.z },
        )
    };
    let mut triangles = Vec::with_capacity(12);
    for axis in 0..3 {
        for side in 0..2 {
            let at = |u: usize, v: usize| {
                let mut c = [0; 3];
                c[axis] = side;
                c[(axis + 1) % 3] = u;
                c[(axis + 2) % 3] = v;
                corner(c[0], c[1], c[2])
            };
            triangles.push([at(0, 0), at(1, 0), at(1, 1)]);
            triangles.push([at(0, 0), at(1, 1), at(0, 1)]);
        }
    }
    triangles
}

/// The part of `triangle` in front of the eye, on the screen of `view_projection`
/// (physical pixels): the triangle cut at clip-space `w` [`NEAR_W`] (Sutherland and
/// Hodgman against one plane), so a floor under the player keeps its visible part.
fn screen_polygon(
    view_projection: &glam::Mat4,
    viewport: [f32; 2],
    triangle: &[Vec3; 3],
) -> Vec<[f32; 2]> {
    let clip = triangle.map(|point| *view_projection * point.extend(1.0));
    let mut kept: Vec<glam::Vec4> = Vec::with_capacity(4);
    for index in 0..3 {
        let (a, b) = (clip[index], clip[(index + 1) % 3]);
        let (inside_a, inside_b) = (a.w >= NEAR_W, b.w >= NEAR_W);
        if inside_a {
            kept.push(a);
        }
        if inside_a != inside_b {
            let t = (NEAR_W - a.w) / (b.w - a.w);
            kept.push(a.lerp(b, t));
        }
    }
    kept.iter()
        .map(|clip| {
            [
                (clip.x / clip.w + 1.0) * 0.5 * viewport[0],
                (1.0 - clip.y / clip.w) * 0.5 * viewport[1],
            ]
        })
        .collect()
}

/// Where the convex `polygon` crosses the row at `y`: its left and right x.
fn row_span(polygon: &[[f32; 2]], y: f32) -> Option<(f32, f32)> {
    let mut span: Option<(f32, f32)> = None;
    for index in 0..polygon.len() {
        let (a, b) = (polygon[index], polygon[(index + 1) % polygon.len()]);
        if (a[1] <= y) == (b[1] <= y) {
            continue;
        }
        let x = a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
        span = Some(span.map_or((x, x), |(low, high)| (low.min(x), high.max(x))));
    }
    span.filter(|(low, high)| high > low)
}

/// `point` on the screen of `view_projection` (physical pixels), if in front of the eye.
fn project(view_projection: &glam::Mat4, viewport: [f32; 2], point: Vec3) -> Option<[f32; 2]> {
    let clip = *view_projection * point.extend(1.0);
    if clip.w <= 0.5 {
        return None;
    }
    let (x, y) = (clip.x / clip.w, clip.y / clip.w);
    (x.abs() <= 1.2 && y.abs() <= 1.2)
        .then(|| [(x + 1.0) * 0.5 * viewport[0], (1.0 - y) * 0.5 * viewport[1]])
}

/// A selection's outline and shade, borrowed or gathered from several.
type Outline<'a> = (Cow<'a, [[Vec3; 2]]>, Cow<'a, [[Vec3; 3]]>);

/// A selection of drawn surface `index` of `bsp` met at `point` with `normal` from the
/// eye of `camera` (the map editor's box and Same shader, which pick no ray of their
/// own): named, outlined and shaded as a click's.
fn surface_selection(
    scene: &Scene<'_>,
    camera: &CameraUniform,
    index: usize,
    point: Vec3,
    normal: Vec3,
) -> Option<Selection> {
    let render = scene.bsp.render();
    let surface = render.surfaces().get(index)?;
    let shader = scene.bsp.shaders().get(surface.shader);
    let eye = Vec3::from_array(camera.camera_position);
    Some(Selection {
        map: scene.map.to_owned(),
        pose: camera.viewpos(),
        eye,
        forward: Vec3::from_array(camera.view_forward),
        hit: point,
        normal,
        distance: point.distance(eye),
        surface: Some(Surface {
            index,
            shader: shader.map_or_else(
                || format!("shader {}", surface.shader),
                |s| s.name_lossy().into_owned(),
            ),
            remapped: None,
            kind: surface_kind(surface.kind),
            lightmap: surface.lightmaps[0],
            vertices: surface.vertices.len(),
            material: shader.map_or(0, |s| s.surface_flags & 0x1f),
        }),
        entity: None,
        collision: None,
        face: None,
        edges: scene.surfaces.surface_edges(index, HIGHLIGHT_EDGES),
        triangles: scene.surfaces.surface_triangles(index, FILL_TRIANGLES),
        selected: Instant::now(),
        model: None,
        remaps: Vec::new(),
    })
}

/// A BSP surface kind as a selection names it.
fn surface_kind(kind: sjk_bsp::SurfaceKind) -> &'static str {
    match kind {
        sjk_bsp::SurfaceKind::Planar => "planar",
        sjk_bsp::SurfaceKind::Patch => "patch",
        sjk_bsp::SurfaceKind::TriangleSoup => "triangle soup",
        sjk_bsp::SurfaceKind::Flare => "flare",
    }
}

/// A selection of caulk, nodraw or sky face `face` as seen from `eye` looking along
/// `forward`: met at its middle, outlined and shaded whole.
fn face_selection(
    map: &str,
    eye: Vec3,
    forward: Vec3,
    face: crate::map_edits::faces::Face,
) -> Selection {
    let camera = CameraUniform {
        camera_position: eye.to_array(),
        view_forward: forward.normalize_or_zero().to_array(),
        ..bytemuck::Zeroable::zeroed()
    };
    let corners: Vec<Vec3> = face.polygon.iter().map(|p| Vec3::from_array(*p)).collect();
    let middle = corners.iter().copied().sum::<Vec3>() / corners.len().max(1) as f32;
    Selection {
        map: map.to_owned(),
        pose: camera.viewpos(),
        eye,
        forward: forward.normalize_or_zero(),
        hit: middle,
        normal: face.normal,
        distance: face.distance,
        surface: None,
        entity: None,
        collision: None,
        edges: (0..corners.len())
            .map(|i| [corners[i], corners[(i + 1) % corners.len()]])
            .collect(),
        triangles: (2..corners.len())
            .map(|i| [corners[0], corners[i - 1], corners[i]])
            .collect(),
        face: Some(face),
        selected: Instant::now(),
        model: None,
        remaps: Vec::new(),
    }
}

/// The direction from the eye of `camera` through `point` of a `viewport`-sized screen
/// (physical pixels), or `None` before the first frame.
fn screen_ray(camera: &CameraUniform, viewport: [f32; 2], point: [f32; 2]) -> Option<Vec3> {
    if viewport[0] <= 0.0 || viewport[1] <= 0.0 {
        return None;
    }
    let inverse = glam::Mat4::from_cols_array_2d(&camera.view_projection).inverse();
    let x = point[0] / viewport[0] * 2.0 - 1.0;
    let y = 1.0 - point[1] / viewport[1] * 2.0;
    // A point inside the frustum, whatever its depth convention.
    let far = inverse.project_point3(Vec3::new(x, y, 0.5));
    let eye = Vec3::from_array(camera.camera_position);
    let mut direction = (far - eye).normalize_or_zero();
    if direction.dot(Vec3::from_array(camera.view_forward)) < 0.0 {
        direction = -direction;
    }
    (direction != Vec3::ZERO && direction.is_finite()).then_some(direction)
}

/// Entry distance of the ray into the box, if it meets it ahead.
fn ray_box(origin: Vec3, direction: Vec3, low: Vec3, high: Vec3) -> Option<f32> {
    let inverse = direction.recip();
    let a = (low - origin) * inverse;
    let b = (high - origin) * inverse;
    let near = a.min(b).max_element();
    let far = a.max(b).min_element();
    (far >= near.max(0.0)).then_some(near.max(0.0))
}

/// The selection waiting for its confirming press, and the note being written.
pub(crate) struct Notes {
    selection: Option<Selection>,
    /// The map editor's multiple selection: the surfaces and faces Shift added before
    /// [`Self::selection`] (the newest), oldest first ([`Member`]).
    more: Vec<Selection>,
    /// The confirmed selection the composer's note belongs to.
    writing: Option<Selection>,
    /// The last rendered main view.
    camera: Option<CameraUniform>,
    /// The selection's outline for this frame.
    highlight: sjk_ui::DrawList,
    /// The selection's shade for this frame, drawn under the outline.
    fill: sjk_ui::DrawList,
    started: Instant,
    /// A note went to the identity service and its outcome is not shown yet.
    sent_waiting: bool,
    /// The serial of the last outcome shown.
    sent_serial: u64,
    /// The models read for selections on this map.
    models: ModelCache,
    /// The last frame's size in physical pixels, for a click's ray.
    viewport: [f32; 2],
    /// The map editor is on: the selection waits until it is replaced or dropped.
    held: bool,
    /// What the map editor's pointer is over, outlined faintly, and whether it is
    /// selected ([`Self::recheck_hover`]).
    hover: Option<Selection>,
    hover_selected: bool,
}

impl Default for Notes {
    fn default() -> Self {
        Self {
            selection: None,
            more: Vec::new(),
            writing: None,
            camera: None,
            highlight: sjk_ui::DrawList::new(HIGHLIGHT_DOTS + HOVER_DOTS),
            fill: sjk_ui::DrawList::new(FILL_STRIPS),
            started: Instant::now(),
            sent_waiting: false,
            sent_serial: 0,
            models: ModelCache::default(),
            viewport: [0.0; 2],
            held: false,
            hover: None,
            hover_selected: false,
        }
    }
}

/// What a selection is made from: the world, its models and the remaps drawn over them.
pub(crate) struct Scene<'a> {
    pub(crate) bsp: &'a Bsp,
    pub(crate) surfaces: &'a crate::decal_marks::DecalSurfaces,
    pub(crate) scratch: &'a mut TraceScratch,
    pub(crate) vfs: Option<&'a VirtualFileSystem>,
    pub(crate) map: &'a str,
    /// The remaps active now ([`Remap`]), in application order.
    pub(crate) remaps: Vec<Remap>,
    /// The entity markers on show (`r_showEntities`): they are picked before the world.
    pub(crate) markers: Vec<crate::entity_view::Marker>,
    /// The entities models are picked from, when map edits change the map's
    /// ([`crate::map_edits::State::pick_entities`]); the map's own otherwise.
    pub(crate) entities: Option<Vec<sjk_entity::Entity>>,
}

/// What a map edit acts on ([`Selection::edit_target`]).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EditTarget {
    /// A model drawn from an entity: its number in the pick's entities, its class, its
    /// path, and the shader of the surface aimed at.
    Model {
        number: usize,
        classname: String,
        model: String,
        shader: Option<String>,
    },
    /// A surface of the world.
    Surface {
        index: usize,
        shader: String,
        kind: &'static str,
    },
    /// A side of a world brush the map draws nothing on (caulk, nodraw), or draws as sky
    /// (`sky`): its BSP brush and side numbers and its shader.
    Face {
        brush: u32,
        side: u32,
        shader: String,
        sky: bool,
    },

    /// Something edits do not change (a mover, an entity marker).
    Other,
}

/// The selection's subject and its model's path, for the note dialog.
pub(crate) type Subject = (String, Option<String>);

/// Most selections the map editor's multiple selection holds: the browser editor's batch
/// limit (`apply_many`, `hide_many`).
pub(crate) const SET_LIMIT: usize = 1024;
/// Most placed models it holds: what one `mapedit_set` (one step to undo) sets.
pub(crate) const PLACED_LIMIT: usize = crate::map_edits::gizmo::MAX_SET;

/// What the map editor's multiple selection holds: a world surface, a hidden face or a
/// model a map edit placed (by its edit's id). Map props and game entities are selected
/// one at a time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Member {
    Surface(usize),
    Face { brush: u32, side: u32 },
    Placed(u32),
}

impl EditTarget {
    /// What it is in a multiple selection, when it may be in one: a surface or a face
    /// (a placed model's member comes from its selection, [`Selection::member`]).
    pub(crate) fn member(&self) -> Option<Member> {
        match self {
            Self::Surface { index, .. } => Some(Member::Surface(*index)),
            Self::Face { brush, side, .. } => Some(Member::Face {
                brush: *brush,
                side: *side,
            }),
            Self::Model { .. } | Self::Other => None,
        }
    }
}

/// What a Shift+click (or Shift and the world-note key) did to the multiple selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Toggled {
    /// Added: the selection now holds this many.
    Added(usize),
    /// Taken out: this many are left.
    Removed(usize),
    /// A map prop, a game entity or something edits do not change: not added.
    NotAMember,
    /// A limit reached: [`SET_LIMIT`] in all, or [`PLACED_LIMIT`] placed models.
    Full(usize),
    /// Nothing there.
    Nothing,
}

/// How a box or a Select command meets the selection there is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Combine {
    /// They become the selection.
    Replace,
    /// They join it (Shift).
    Add,
    /// Those in it leave it (Ctrl).
    Remove,
}

/// Meet the selection `primary` (the newest) and `more` with `found` (the first the
/// newest) as `combine` says, each thing once (by `key`), within [`SET_LIMIT`] and
/// [`PLACED_LIMIT`]: how many it holds after, and whether a limit left some out. A map
/// prop or another thing selected alone gives way, as with Shift+click.
pub(crate) fn combine_in<T>(
    primary: &mut Option<T>,
    more: &mut Vec<T>,
    found: Vec<T>,
    combine: Combine,
    key: impl Fn(&T) -> Option<Member>,
) -> (usize, bool) {
    let count = |primary: &Option<T>, more: &Vec<T>| usize::from(primary.is_some()) + more.len();
    if combine == Combine::Remove {
        let gone: Vec<Member> = found.iter().filter_map(&key).collect();
        more.retain(|kept| key(kept).is_none_or(|member| !gone.contains(&member)));
        if primary
            .as_ref()
            .and_then(&key)
            .is_some_and(|member| gone.contains(&member))
        {
            *primary = more.pop();
        }
        return (count(primary, more), false);
    }
    if combine == Combine::Replace || primary.as_ref().is_some_and(|p| key(p).is_none()) {
        *primary = None;
        more.clear();
    }
    let mut members: Vec<Member> = primary.iter().chain(more.iter()).filter_map(&key).collect();
    let mut placed = members
        .iter()
        .filter(|member| matches!(member, Member::Placed(_)))
        .count();
    let mut capped = false;
    // Added oldest first, so the first found ends the newest.
    for candidate in found.into_iter().rev() {
        let Some(member) = key(&candidate) else {
            continue;
        };
        if members.contains(&member) {
            continue;
        }
        let is_placed = matches!(member, Member::Placed(_));
        if members.len() >= SET_LIMIT || (is_placed && placed >= PLACED_LIMIT) {
            capped = true;
            continue;
        }
        placed += usize::from(is_placed);
        members.push(member);
        if let Some(previous) = primary.replace(candidate) {
            more.push(previous);
        }
    }
    (count(primary, more), capped)
}

/// Add `candidate` to the multiple selection `primary` (the newest) and `more`, or take
/// it out when it is there already (by `key`). A map prop or another thing selected alone
/// gives way to it. Placed models mix with surfaces and faces, as the browser editor's
/// set does: the gizmo moves the placed ones, Hide takes them all, a remap the others.
fn toggle_in<T>(
    primary: &mut Option<T>,
    more: &mut Vec<T>,
    candidate: T,
    key: impl Fn(&T) -> Option<Member>,
) -> Toggled {
    let Some(wanted) = key(&candidate) else {
        return Toggled::NotAMember;
    };
    let count = |primary: &Option<T>, more: &Vec<T>| usize::from(primary.is_some()) + more.len();
    match primary.as_ref().map(&key) {
        None | Some(None) => {
            more.clear();
            *primary = Some(candidate);
            return Toggled::Added(1);
        }
        Some(Some(member)) if member == wanted => {
            *primary = more.pop();
            return Toggled::Removed(count(primary, more));
        }
        Some(Some(_)) => {}
    }
    if let Some(at) = more.iter().position(|kept| key(kept) == Some(wanted)) {
        more.remove(at);
        return Toggled::Removed(count(primary, more));
    }
    if count(primary, more) >= SET_LIMIT {
        return Toggled::Full(SET_LIMIT);
    }
    if matches!(wanted, Member::Placed(_)) {
        let placed = primary
            .iter()
            .chain(more.iter())
            .filter(|kept| matches!(key(kept), Some(Member::Placed(_))))
            .count();
        if placed >= PLACED_LIMIT {
            return Toggled::Full(PLACED_LIMIT);
        }
    }
    if let Some(previous) = primary.replace(candidate) {
        more.push(previous);
    }
    Toggled::Added(count(primary, more))
}

/// What a `worldnote` press did.
pub(crate) enum Press {
    /// Selected (or reselected) something: show its summary.
    Selected(String),
    /// Confirmed the selection: open the composer for a note about it (its subject and
    /// its model's path).
    Confirmed(Subject),
    /// Nothing under the crosshair.
    Nothing,
}

impl Notes {
    /// Keep the final main view (`upload_scene_camera`).
    pub(crate) fn set_camera(&mut self, camera: CameraUniform) {
        self.camera = Some(camera);
    }

    /// A `worldnote` press.
    pub(crate) fn press(&mut self, scene: Scene<'_>) -> Press {
        let now = Instant::now();
        if let Some(selection) = self.selection.take().filter(|s| s.fresh(now)) {
            let subject = (selection.summary(), selection.model_path());
            self.writing = Some(selection);
            return Press::Confirmed(subject);
        }
        let Some(camera) = self.camera else {
            return Press::Nothing;
        };
        let forward = Vec3::from_array(camera.view_forward);
        match self.select(scene, &camera, forward) {
            Some(selection) => {
                let summary = selection.summary();
                self.selection = Some(selection);
                Press::Selected(summary)
            }
            None => Press::Nothing,
        }
    }

    /// The selection along `direction` from the eye of `camera`: the world's surface or
    /// mover, unless a model comes first, with the remaps drawn over their shaders.
    fn select(
        &mut self,
        scene: Scene<'_>,
        camera: &CameraUniform,
        direction: Vec3,
    ) -> Option<Selection> {
        if let Some(selection) = self.select_marker(&scene, camera, direction) {
            return Some(selection);
        }
        let world = pick(
            scene.bsp,
            scene.surfaces,
            scene.scratch,
            camera,
            scene.map,
            direction,
        );
        let eye = Vec3::from_array(camera.camera_position);
        let direction = direction.normalize_or_zero();
        let model = scene.vfs.and_then(|vfs| {
            let parsed;
            let entities = match &scene.entities {
                Some(entities) => entities,
                None => {
                    parsed = sjk_entity::parse_entity_lump(scene.bsp.entities()).ok()?;
                    &parsed
                }
            };
            pick_model(entities, &mut self.models, vfs, scene.map, eye, direction)
        });
        let model = model.filter(|model| {
            world
                .as_ref()
                .is_none_or(|world| model.distance <= world.distance)
        });
        let mut selection = match (world, model) {
            (Some(mut world), model) => {
                world.model = model;
                world
            }
            (None, Some(model)) => Selection {
                map: scene.map.to_owned(),
                pose: camera.viewpos(),
                eye,
                forward: Vec3::from_array(camera.view_forward),
                hit: model.hit,
                normal: model.normal,
                distance: model.distance,
                surface: None,
                entity: None,
                collision: None,
                face: None,
                edges: Vec::new(),
                triangles: Vec::new(),
                selected: Instant::now(),
                model: Some(model),
                remaps: Vec::new(),
            },
            (None, None) => return None,
        };
        if let Some(surface) = &mut selection.surface {
            surface.remapped = remap_of(&scene.remaps, &surface.shader);
        }
        if let Some(model) = &mut selection.model {
            for shader in &mut model.shaders {
                shader.remapped = remap_of(&scene.remaps, &shader.shader);
            }
        }
        selection.remaps = scene.remaps;
        Some(selection)
    }

    /// The entity marker the view ray passes closest to, if the markers are on show:
    /// they draw through walls, so the one aimed at wins over whatever is behind it.
    fn select_marker(
        &self,
        scene: &Scene<'_>,
        camera: &CameraUniform,
        direction: Vec3,
    ) -> Option<Selection> {
        let eye = Vec3::from_array(camera.camera_position);
        let direction = direction.normalize_or_zero();
        let (marker, distance) = crate::entity_view::Marker::aim(&scene.markers, eye, direction)?;
        let entities = sjk_entity::parse_entity_lump(scene.bsp.entities()).ok()?;
        let source = entities.get(marker.number)?;
        let text = |key: &str| source.get(key).unwrap_or("").to_owned();
        let (low, high) = marker.bounds();
        Some(Selection {
            map: scene.map.to_owned(),
            pose: camera.viewpos(),
            eye,
            forward: Vec3::from_array(camera.view_forward),
            hit: marker.origin,
            normal: -direction,
            distance,
            surface: None,
            entity: Some(Entity {
                number: marker.number,
                classname: text("classname"),
                targetname: text("targetname"),
                model: text("model"),
                origin: text("origin"),
                shaders: Vec::new(),
            }),
            collision: None,
            face: None,
            edges: box_edges(low, high),
            triangles: box_triangles(low, high),
            selected: Instant::now(),
            model: None,
            remaps: scene.remaps.clone(),
        })
    }

    /// Select what is under the crosshair for a map edit, never opening the composer:
    /// its summary, or `None` when nothing is there (the selection is dropped).
    pub(crate) fn select_for_edit(&mut self, scene: Scene<'_>) -> Option<String> {
        let camera = self.camera?;
        let forward = Vec3::from_array(camera.view_forward);
        self.more.clear();
        self.selection = self.select(scene, &camera, forward);
        self.selection.as_ref().map(Selection::summary)
    }

    /// Select what is under `point` (physical pixels) for a map edit, as
    /// [`Self::select_for_edit`] does under the crosshair.
    pub(crate) fn select_for_edit_at(
        &mut self,
        scene: Scene<'_>,
        point: [f32; 2],
    ) -> Option<String> {
        let camera = self.camera?;
        let direction = screen_ray(&camera, self.viewport, point)?;
        self.more.clear();
        self.selection = self.select(scene, &camera, direction);
        self.selection.as_ref().map(Selection::summary)
    }

    /// Shift and the world-note key in the map editor: add what is under the crosshair
    /// to the multiple selection, or take it out.
    pub(crate) fn toggle_for_edit(&mut self, scene: Scene<'_>) -> Toggled {
        let Some(camera) = self.camera else {
            return Toggled::Nothing;
        };
        let forward = Vec3::from_array(camera.view_forward);
        match self.select(scene, &camera, forward) {
            Some(candidate) => self.toggle(candidate),
            None => Toggled::Nothing,
        }
    }

    /// Shift+click in the map editor: add what is under `point` (physical pixels) to the
    /// multiple selection, or take it out.
    pub(crate) fn toggle_for_edit_at(&mut self, scene: Scene<'_>, point: [f32; 2]) -> Toggled {
        let Some(camera) = self.camera else {
            return Toggled::Nothing;
        };
        let Some(direction) = screen_ray(&camera, self.viewport, point) else {
            return Toggled::Nothing;
        };
        match self.select(scene, &camera, direction) {
            Some(candidate) => self.toggle(candidate),
            None => Toggled::Nothing,
        }
    }

    fn toggle(&mut self, candidate: Selection) -> Toggled {
        if !self.held
            && self
                .selection
                .as_ref()
                .is_some_and(|s| !s.fresh(Instant::now()))
        {
            self.selection = None;
        }
        toggle_in(
            &mut self.selection,
            &mut self.more,
            candidate,
            Selection::member,
        )
    }

    /// The map editor's box, Same shader and Select all placed: `found` (the first the
    /// newest) meets the selection as `combine` says ([`combine_in`]): how many it holds
    /// after, and whether a limit left some out.
    pub(crate) fn combine_for_edit(
        &mut self,
        found: Vec<Selection>,
        combine: Combine,
    ) -> (usize, bool) {
        if self.waiting_selection().is_none() {
            self.selection = None;
            self.more.clear();
        }
        combine_in(
            &mut self.selection,
            &mut self.more,
            found,
            combine,
            Selection::member,
        )
    }

    /// Whether the map editor's selection holds `member`.
    pub(crate) fn holds_member(&self, member: Member) -> bool {
        self.edit_selection()
            .any(|selection| selection.member() == Some(member))
    }

    /// What is under `point` (physical pixels) as a click would select it, without
    /// selecting it: Ctrl+click's candidate, and the hover's.
    pub(crate) fn candidate_at(&mut self, scene: Scene<'_>, point: [f32; 2]) -> Option<Selection> {
        let camera = self.camera?;
        let direction = screen_ray(&camera, self.viewport, point)?;
        self.select(scene, &camera, direction)
    }

    /// Drawn surface `index` met at `point` with `normal`, as a selection from the last
    /// rendered view (the map editor's box and Same shader).
    pub(crate) fn surface_for_edit(
        &self,
        scene: &Scene<'_>,
        index: usize,
        point: Vec3,
        normal: Vec3,
    ) -> Option<Selection> {
        surface_selection(scene, self.camera.as_ref()?, index, point, normal)
    }

    /// Caulk, nodraw or sky face `face` as a selection from the last rendered view.
    pub(crate) fn face_for_edit(
        &self,
        map: &str,
        face: crate::map_edits::faces::Face,
    ) -> Option<Selection> {
        let camera = self.camera?;
        Some(face_selection(
            map,
            Vec3::from_array(camera.camera_position),
            Vec3::from_array(camera.view_forward),
            face,
        ))
    }

    /// The model of entity `number` of `scene`'s entities (a placed model's) as a
    /// selection from the last rendered view, met at its first triangle.
    pub(crate) fn model_for_edit(&mut self, scene: &Scene<'_>, number: usize) -> Option<Selection> {
        let camera = self.camera?;
        let vfs = scene.vfs?;
        let entities = scene.entities.as_deref()?;
        let eye = Vec3::from_array(camera.camera_position);
        let model = models::model_of(entities, &mut self.models, vfs, scene.map, number, eye)?;
        Some(Selection {
            map: scene.map.to_owned(),
            pose: camera.viewpos(),
            eye,
            forward: Vec3::from_array(camera.view_forward),
            hit: model.hit,
            normal: model.normal,
            distance: model.distance,
            surface: None,
            entity: None,
            collision: None,
            face: None,
            edges: Vec::new(),
            triangles: Vec::new(),
            selected: Instant::now(),
            model: Some(model),
            remaps: Vec::new(),
        })
    }

    /// The last rendered main view's eye, view-projection and frame size (physical
    /// pixels), for the map editor's box.
    pub(crate) fn view_for_box(&self) -> Option<(Vec3, glam::Mat4, [f32; 2])> {
        let camera = self.camera?;
        Some((
            Vec3::from_array(camera.camera_position),
            glam::Mat4::from_cols_array_2d(&camera.view_projection),
            self.viewport,
        ))
    }

    /// The map editor's hover: what is under the pointer, outlined faintly over the
    /// scene ([`Self::draw_highlight`]); `None` drops it.
    pub(crate) fn set_hover(&mut self, hover: Option<Selection>) {
        self.hover = hover;
        self.recheck_hover();
    }

    /// Note again whether the hover is selected (it is then not outlined): when it or the
    /// selection changes, never per frame.
    pub(crate) fn recheck_hover(&mut self) {
        self.hover_selected = self
            .hover
            .as_ref()
            .and_then(Selection::member)
            .is_some_and(|member| self.holds_member(member));
    }

    /// The map editor's hover, when there is one.
    pub(crate) fn hover(&self) -> Option<&Selection> {
        self.hover.as_ref()
    }

    /// The map editor's placed models drawn elsewhere than their selection's outline (a
    /// gizmo's preview, a commit, an undo, a grab): the outline follows. `at` gives a
    /// placed edit's origin, angles and scale as drawn. Once a frame while the editor is
    /// on; nothing is allocated.
    pub(crate) fn follow_placed(
        &mut self,
        at: impl Fn(u32) -> Option<([f32; 3], [f32; 3], [f32; 3])>,
    ) {
        for selection in self.selection.iter_mut().chain(self.more.iter_mut()) {
            let Some((origin, angles, scale)) = selection.placed_id().and_then(&at) else {
                continue;
            };
            if let Some(model) = &mut selection.model {
                model.move_to(Vec3::from_array(origin), angles, scale);
            }
        }
    }

    /// The map editor's selection as edits act on it: the newest first, then the others
    /// Shift added, newest first. Empty without a selection.
    pub(crate) fn edit_selection(&self) -> impl Iterator<Item = &Selection> {
        let primary = self.waiting_selection();
        let more = if primary.is_some() {
            self.more.as_slice()
        } else {
            &[]
        };
        primary.into_iter().chain(more.iter().rev())
    }

    /// How many the map editor's selection holds.
    pub(crate) fn edit_selection_len(&self) -> usize {
        self.waiting_selection().map_or(0, |_| 1 + self.more.len())
    }

    /// The map editor's Same plane and Connected surface: faces `faces` (the first one the
    /// newest) become its selection, seen from `eye` looking along `forward`.
    pub(crate) fn select_faces_for_edit(
        &mut self,
        map: &str,
        [eye, forward]: [Vec3; 2],
        faces: Vec<crate::map_edits::faces::Face>,
    ) {
        let mut faces = faces.into_iter();
        let Some(first) = faces.next() else {
            return;
        };
        let rest: Vec<Selection> = faces
            .take(SET_LIMIT - 1)
            .map(|face| face_selection(map, eye, forward, face))
            .collect();
        self.selection = Some(face_selection(map, eye, forward, first));
        // Kept oldest first, the first face being the newest.
        self.more = rest.into_iter().rev().collect();
    }

    /// The ray from the last rendered main view's eye through `point` (physical pixels):
    /// the eye and the direction.
    pub(crate) fn ray_at(&self, point: [f32; 2]) -> Option<(Vec3, Vec3)> {
        let camera = self.camera?;
        let direction = screen_ray(&camera, self.viewport, point)?;
        Some((Vec3::from_array(camera.camera_position), direction))
    }

    /// Select what the ray from `eye` along `direction` meets for a map edit, a view at
    /// `eye` looking along `forward` (the browser editor's view: [`crate::map_edits`]'s
    /// `web`), as [`Self::select_for_edit`] does under the crosshair.
    pub(crate) fn select_for_edit_ray(
        &mut self,
        scene: Scene<'_>,
        eye: Vec3,
        forward: Vec3,
        direction: Vec3,
    ) -> Option<String> {
        let camera = CameraUniform {
            camera_position: eye.to_array(),
            view_forward: forward.normalize_or_zero().to_array(),
            ..bytemuck::Zeroable::zeroed()
        };
        self.more.clear();
        self.selection = self.select(scene, &camera, direction);
        self.selection.as_ref().map(Selection::summary)
    }

    /// Select caulk or nodraw face `face` for a map edit, met at `hit`, as seen from
    /// `eye` looking along `forward`: the browser editor's click on a face its brush
    /// view draws.
    pub(crate) fn select_face_for_edit(
        &mut self,
        map: &str,
        [eye, forward, hit]: [Vec3; 3],
        face: crate::map_edits::faces::Face,
    ) -> String {
        let mut selection = face_selection(map, eye, forward, face);
        selection.hit = hit;
        let summary = selection.summary();
        self.more.clear();
        self.selection = Some(selection);
        summary
    }

    /// While the map editor is on, a selection never goes stale.
    pub(crate) fn hold_selection(&mut self, held: bool) {
        self.held = held;
    }

    /// The selection waiting for a second press, while it is fresh.
    pub(crate) fn waiting_selection(&self) -> Option<&Selection> {
        self.selection
            .as_ref()
            .filter(|selection| self.held || selection.fresh(Instant::now()))
    }

    /// The last rendered main view's eye and forward direction.
    pub(crate) fn view(&self) -> Option<(Vec3, Vec3)> {
        self.camera.map(|camera| {
            (
                Vec3::from_array(camera.camera_position),
                Vec3::from_array(camera.view_forward).normalize_or_zero(),
            )
        })
    }

    /// A click at `point` (physical pixels) on the scene round the note dialog: what is
    /// under the pointer becomes the note's selection. `None` when nothing is there, and
    /// the selection stays.
    pub(crate) fn pick_at(&mut self, scene: Scene<'_>, point: [f32; 2]) -> Option<Subject> {
        let camera = self.camera?;
        self.writing.as_ref()?;
        let direction = screen_ray(&camera, self.viewport, point)?;
        let selection = self.select(scene, &camera, direction)?;
        let subject = (selection.summary(), selection.model_path());
        self.writing = Some(selection);
        Some(subject)
    }

    /// Clear model in the note dialog: the note is about what is behind the model.
    pub(crate) fn clear_model(&mut self) -> Option<Subject> {
        let selection = self.writing.as_mut()?;
        selection.model.take()?;
        Some((selection.summary(), None))
    }

    /// The waiting selection's summary, for the world shots.
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> Option<String> {
        self.selection.as_ref().map(Selection::summary)
    }

    /// Escape before the confirming press: forget the waiting selection. True when there
    /// was one to forget.
    pub(crate) fn cancel_selection(&mut self) -> bool {
        let more = !self.more.is_empty();
        self.more.clear();
        self.selection.take().is_some() || more
    }

    /// The composer closed without saving (Escape): forget the confirmed selection.
    pub(crate) fn composer_closed(&mut self) {
        self.writing = None;
    }

    /// Rebuild the outline of the waiting or confirmed selection for this frame.
    pub(crate) fn draw_highlight(&mut self, viewport: [f32; 2]) {
        self.viewport = viewport;
        self.highlight.clear();
        self.fill.clear();
        let now = Instant::now();
        if !self.held && self.selection.as_ref().is_some_and(|s| !s.fresh(now)) {
            self.selection = None;
            self.more.clear();
        }
        if !self.held {
            // the hover is the map editor's alone
            self.hover = None;
        }
        if let (Some(hover), Some(camera), None) = (&self.hover, self.camera, &self.writing) {
            // Faint and steady, under the selection's dots; not on what is selected.
            if !self.hover_selected {
                let view_projection = glam::Mat4::from_cols_array_2d(&camera.view_projection);
                Self::draw_hover(
                    &mut self.highlight,
                    hover.outline().0,
                    &view_projection,
                    viewport,
                );
            }
        }
        let (Some(selection), Some(camera)) = (
            self.writing.as_ref().or(self.selection.as_ref()),
            self.camera,
        ) else {
            return;
        };
        let view_projection = glam::Mat4::from_cols_array_2d(&camera.view_projection);
        // The map editor's multiple selection is drawn whole (a note's never has one).
        let more: &[Selection] = if self.writing.is_some() {
            &[]
        } else {
            &self.more
        };
        let (edges, triangles) = selection.outline();
        let (edges, triangles): Outline<'_> = if more.is_empty() {
            (Cow::Borrowed(edges), Cow::Borrowed(triangles))
        } else {
            let mut all_edges = edges.to_vec();
            let mut all_triangles = triangles.to_vec();
            for kept in more {
                let (edges, triangles) = kept.outline();
                all_edges.extend_from_slice(edges);
                all_triangles.extend_from_slice(triangles);
            }
            (Cow::Owned(all_edges), Cow::Owned(all_triangles))
        };
        let (edges, triangles) = (edges.as_ref(), triangles.as_ref());
        // Space the dots by the outline's length on screen, so a big surface keeps
        // within the budget and a small one still reads as a line.
        let lengths: Vec<f32> = edges
            .iter()
            .map(|[a, b]| {
                match (
                    project(&view_projection, viewport, *a),
                    project(&view_projection, viewport, *b),
                ) {
                    (Some(a), Some(b)) => ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt(),
                    _ => 120.0,
                }
            })
            .collect();
        let total: f32 = lengths.iter().sum();
        let spacing = (total / HIGHLIGHT_DOTS as f32).max(7.0);
        let time = now.duration_since(self.started).as_secs_f32();
        Self::draw_fill(&mut self.fill, triangles, &view_projection, viewport, time);
        let size = (viewport[1] / 540.0).clamp(2.0, 4.0);
        let mut index = 0u32;
        for ([a, b], length) in edges.iter().zip(&lengths) {
            let steps = ((length / spacing).ceil() as usize).clamp(1, 64);
            for step in 0..=steps {
                let point = a.lerp(*b, step as f32 / steps as f32);
                index = index.wrapping_add(1);
                let Some([x, y]) = project(&view_projection, viewport, point) else {
                    continue;
                };
                // Each dot flickers on its own beat, brighter as a wave runs along.
                let seed = (index as f32 * 12.9898 + (time * 9.0).floor() * 78.233).sin();
                let flicker = (seed * 43_758.547).fract().abs();
                let wave = ((index as f32 * 0.21 - time * 6.0).sin() * 0.5 + 0.5).powi(4);
                let alpha = (0.35 + 0.4 * flicker + 0.25 * wave).min(1.0);
                if !self.highlight.push(sjk_ui::DrawCommand::SolidRect {
                    rect: sjk_ui::Rect::new(x - size * 0.5, y - size * 0.5, size, size),
                    color: sjk_ui::Color::new(0.25, 1.0, 0.45, alpha),
                }) {
                    return;
                }
            }
        }
    }

    /// The map editor's hover outline: `edges` as faint white dots, at most
    /// [`HOVER_DOTS`], spaced by their length on screen.
    fn draw_hover(
        list: &mut sjk_ui::DrawList,
        edges: &[[Vec3; 2]],
        view_projection: &glam::Mat4,
        viewport: [f32; 2],
    ) {
        let on_screen = |point: Vec3| project(view_projection, viewport, point);
        let length = |[a, b]: &[Vec3; 2]| match (on_screen(*a), on_screen(*b)) {
            (Some(a), Some(b)) => ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt(),
            _ => 120.0,
        };
        let total: f32 = edges.iter().map(length).sum();
        let spacing = (total / HOVER_DOTS as f32).max(9.0);
        let size = (viewport[1] / 540.0).clamp(2.0, 3.5);
        let mut drawn = 0;
        for edge in edges {
            let steps = ((length(edge) / spacing).ceil() as usize).clamp(1, 48);
            for step in 0..=steps {
                if drawn == HOVER_DOTS {
                    return;
                }
                let Some([x, y]) = on_screen(edge[0].lerp(edge[1], step as f32 / steps as f32))
                else {
                    continue;
                };
                drawn += 1;
                if !list.push(sjk_ui::DrawCommand::SolidRect {
                    rect: sjk_ui::Rect::new(x - size * 0.5, y - size * 0.5, size, size),
                    color: sjk_ui::Color::new(0.85, 0.95, 1.0, 0.65),
                }) {
                    return;
                }
            }
        }
    }

    /// Shade the selection's triangles in horizontal strips, spaced so the whole of it
    /// keeps within [`FILL_STRIPS`].
    fn draw_fill(
        fill: &mut sjk_ui::DrawList,
        triangles: &[[Vec3; 3]],
        view_projection: &glam::Mat4,
        viewport: [f32; 2],
        time: f32,
    ) {
        let polygons: Vec<Vec<[f32; 2]>> = triangles
            .iter()
            .map(|triangle| screen_polygon(view_projection, viewport, triangle))
            .filter(|polygon| polygon.len() >= 3)
            .collect();
        let rows = |polygon: &Vec<[f32; 2]>| {
            let (low, high) = polygon.iter().fold((f32::MAX, f32::MIN), |(low, high), p| {
                (low.min(p[1]), high.max(p[1]))
            });
            (low.max(0.0), high.min(viewport[1]))
        };
        let covered: f32 = polygons
            .iter()
            .map(|polygon| {
                let (low, high) = rows(polygon);
                (high - low).max(0.0)
            })
            .sum();
        let step = (covered / FILL_STRIPS as f32).max((viewport[1] / 360.0).max(3.0));
        // Thin lines however far apart: a box's or Same shader's big set stays a shade.
        let thickness = (step * 0.55).clamp(1.0, (viewport[1] / 360.0).max(3.0));
        for polygon in &polygons {
            let (low, high) = rows(polygon);
            // Rows on one grid for the whole selection, so the strips line up.
            let mut y = (low / step).floor() * step;
            while y < high {
                if let Some((left, right)) = row_span(polygon, y + step * 0.5) {
                    let (left, right) = (left.max(0.0), right.min(viewport[0]));
                    if right > left {
                        let wave = ((y * 0.035 - time * 5.0).sin() * 0.5 + 0.5).powi(3);
                        if !fill.push(sjk_ui::DrawCommand::SolidRect {
                            rect: sjk_ui::Rect::new(left, y, right - left, thickness),
                            color: sjk_ui::Color::new(0.2, 1.0, 0.45, 0.13 + 0.17 * wave),
                        }) {
                            return;
                        }
                    }
                }
                y += step;
            }
        }
    }

    /// The shade to draw this frame, under [`Self::highlight`], if any.
    pub(crate) fn fill(&self) -> Option<&sjk_ui::DrawList> {
        (!self.fill.is_empty()).then_some(&self.fill)
    }

    /// The outline to draw this frame, if any.
    pub(crate) fn highlight(&self) -> Option<&sjk_ui::DrawList> {
        (!self.highlight.is_empty()).then_some(&self.highlight)
    }

    /// The last rendered main view for the map editor's gizmo
    /// ([`crate::map_edits::gizmo`]): its view-projection, eye, forward and the frame's
    /// size in physical pixels.
    pub(crate) fn camera_for_gizmo(&self) -> Option<(glam::Mat4, Vec3, Vec3, [f32; 2])> {
        self.camera.map(|camera| {
            (
                glam::Mat4::from_cols_array_2d(&camera.view_projection),
                Vec3::from_array(camera.camera_position),
                Vec3::from_array(camera.view_forward).normalize_or_zero(),
                self.viewport,
            )
        })
    }

    /// Save `note` about the confirmed selection into `directory`, returning the
    /// screenshot name to take (without extension; it lands in `screenshots`), a line
    /// for the console and the note for the hub (`None` when the hub would refuse its
    /// text).
    pub(crate) fn save(
        &mut self,
        note: &str,
        directory: &Path,
        screenshots: &Path,
    ) -> Option<(String, String, Option<sjk_identity::WorldNote>)> {
        let selection = self.writing.take()?;
        let note = note.trim();
        if note.is_empty() {
            return None;
        }
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let time = utc_text(seconds);
        let shot = format!("note_{seconds}");
        let screenshot = screenshots
            .join(format!("{shot}.jpg"))
            .display()
            .to_string();
        let json = selection.json(note, &time, &screenshot);
        let line = format!(
            "{time} | {} | {} | {} | {note}",
            selection.map,
            selection.setviewpos(),
            selection.summary()
        );
        let written = append(&directory.join("notes.jsonl"), &json.to_string())
            .and_then(|()| append(&directory.join("notes.txt"), &line));
        let hub = sjk_identity::report::note_text(note)
            .ok()
            .map(|text| selection.world_note(text));
        Some((
            shot,
            match written {
                Ok(()) => format!("note kept: {line}"),
                Err(error) => format!("^1note not kept in {}: {error}", directory.display()),
            },
            hub,
        ))
    }

    /// Once a frame: show the outcome of a note sent to the hub.
    fn poll_sent(&mut self) -> Option<String> {
        if !self.sent_waiting {
            return None;
        }
        let outcome = crate::player_identity::note_outcome()
            .filter(|outcome| outcome.serial != self.sent_serial)?;
        self.sent_serial = outcome.serial;
        self.sent_waiting = false;
        Some(if outcome.sent {
            format!(
                "Note sent to the SJK team ({}). Thank you!",
                outcome.message
            )
        } else {
            format!("Note kept on this PC, not sent: {}", outcome.message)
        })
    }
}

impl crate::GpuState {
    /// `worldnote`: select the world under the crosshair, or confirm the selection and
    /// open the note composer.
    pub(crate) fn world_note_press(&mut self) {
        if self.map_edits.editor.active {
            self.map_edit_select();
            return;
        }
        let map = self.note_map();
        let remaps = self.note_remaps();
        let press = self.world_notes.press(Scene {
            bsp: &self.bsp,
            surfaces: &self.decal_surfaces,
            scratch: &mut self.trace_scratch,
            vfs: self.vfs.as_deref(),
            map: &map,
            remaps,
            markers: self.entity_view.pickable(),
            entities: self.map_edits.pick_entities(&self.bsp),
        });
        let centre = |text: String| (sjk_client::ServerEventKind::CenterPrint, text);
        let (kind, text) = match press {
            Press::Selected(summary) => {
                crate::log::progress(format_args!("note selection: {summary}"));
                centre(format!(
                    "{summary}
Press World note again to write a note, Escape to drop it"
                ))
            }
            Press::Confirmed((subject, model)) => {
                self.gameplay_input.release_keys();
                self.text_dialog
                    .open(crate::text_dialog::Kind::Note { subject, model });
                self.sync_cursor_policy();
                return;
            }
            Press::Nothing => centre("Nothing under the crosshair to note".to_owned()),
        };
        self.chat.receive(kind, text, None, Instant::now());
    }

    /// A click on the scene round the note dialog at `point` (physical pixels): note
    /// what is under the pointer instead.
    pub(crate) fn world_note_pick_at(&mut self, point: [f32; 2]) {
        let map = self.note_map();
        let remaps = self.note_remaps();
        let picked = self.world_notes.pick_at(
            Scene {
                bsp: &self.bsp,
                surfaces: &self.decal_surfaces,
                scratch: &mut self.trace_scratch,
                vfs: self.vfs.as_deref(),
                map: &map,
                remaps,
                markers: self.entity_view.pickable(),
                entities: self.map_edits.pick_entities(&self.bsp),
            },
            point,
        );
        if let Some((subject, model)) = picked {
            crate::log::progress(format_args!("note selection: {subject}"));
            self.text_dialog.set_note(subject, model);
        }
    }

    /// Clear model in the note dialog: the note is about what is behind the model.
    pub(crate) fn world_note_clear_model(&mut self) {
        if let Some((subject, model)) = self.world_notes.clear_model() {
            crate::log::progress(format_args!("note selection: {subject}"));
            self.text_dialog.set_note(subject, model);
        }
    }

    /// The map a note is about: the one explored, or the one loaded.
    fn note_map(&self) -> String {
        if self.resident.exploring() {
            self.resident.map.clone()
        } else {
            self.world_load_map.clone()
        }
    }

    /// The shader remaps drawn now, as `listRemaps` lists them, without those a later
    /// remap of the same shader hides or that restore a shader to itself.
    fn note_remaps(&mut self) -> Vec<Remap> {
        let mode = self.remap_mode();
        let server = self
            .live_session
            .as_ref()
            .map(|session| session.shader_remaps())
            .or_else(|| self.demo_session.as_ref().map(|demo| demo.shader_remaps()))
            .and_then(|remaps| remaps.table(mode));
        self.world_materials
            .remap_entries(server)
            .into_iter()
            .filter(|entry| entry.active && entry.from != entry.to)
            .map(|entry| Remap {
                source: entry.source,
                from: entry.from.to_owned(),
                to: entry.to.to_owned(),
            })
            .collect()
    }

    /// The note composer's Enter: keep the note with its selection and take the
    /// screenshot of the same view.
    pub(crate) fn save_world_note(&mut self, note: &str) {
        let Some(directory) = self
            .console
            .as_ref()
            .map(|console| console.config_directory().to_path_buf())
        else {
            return;
        };
        let Some((shot, line, hub)) =
            self.world_notes
                .save(note, &directory, self.screenshots.directory())
        else {
            return;
        };
        let tag = hub.and_then(|mut hub| {
            hub.build = crate::build_info::VERSION.to_owned();
            hub.server = self
                .live_session
                .as_ref()
                .filter(|session| !session.is_local())
                .map(|session| session.server().to_string())
                .unwrap_or_default();
            crate::player_identity::note(hub)
        });
        self.world_notes.sent_waiting |= tag.is_some();
        self.screenshots
            .request(crate::screenshot::Request::jpeg_named(&shot).for_note(tag));
        crate::log::progress(format_args!("{line}"));
        if let Some(console) = &mut self.console {
            console.push_log(line);
        }
    }
}

impl crate::GpuState {
    /// Once a frame: show the outcome of a note sent to the hub.
    pub(crate) fn poll_world_note(&mut self) {
        if let Some(message) = self.world_notes.poll_sent() {
            crate::log::progress(format_args!("{message}"));
            if let Some(console) = &mut self.console {
                console.push_log(message.clone());
            }
            self.chat.receive(
                sjk_client::ServerEventKind::CenterPrint,
                message,
                None,
                Instant::now(),
            );
        }
    }
}

fn append(path: &PathBuf, line: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

/// `dd/mm/yyyy hh:mm:ss UTC` of Unix `seconds`.
fn utc_text(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{day:02}/{month:02}/{year} {:02}:{:02}:{:02} UTC",
        rest / 3_600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_adds_surfaces_and_faces_and_takes_them_out_again() {
        // Members stand for selections: a surface, a face, or a model (no member).
        let key = |member: &Option<Member>| *member;
        let (mut primary, mut more) = (None, Vec::new());
        let surface = Some(Member::Surface(4));
        let face = Some(Member::Face { brush: 2, side: 9 });
        let other = Some(Member::Surface(5));
        assert_eq!(
            toggle_in(&mut primary, &mut more, surface, key),
            Toggled::Added(1)
        );
        assert_eq!(
            toggle_in(&mut primary, &mut more, face, key),
            Toggled::Added(2)
        );
        assert_eq!(
            toggle_in(&mut primary, &mut more, other, key),
            Toggled::Added(3)
        );
        assert_eq!(
            (primary, more.as_slice()),
            (Some(other), &[surface, face][..])
        );
        // A model is selected alone, never added.
        assert_eq!(
            toggle_in(&mut primary, &mut more, None, key),
            Toggled::NotAMember
        );
        // Taking out an older one keeps the newest; taking out the newest promotes the
        // last added before it.
        assert_eq!(
            toggle_in(&mut primary, &mut more, face, key),
            Toggled::Removed(2)
        );
        assert_eq!(
            toggle_in(&mut primary, &mut more, other, key),
            Toggled::Removed(1)
        );
        assert_eq!((primary, more.len()), (Some(surface), 0));
        assert_eq!(
            toggle_in(&mut primary, &mut more, surface, key),
            Toggled::Removed(0)
        );
        assert_eq!(primary, None);
        // A model selected alone gives way to a surface.
        let mut model = Some(None);
        let mut kept = vec![Some(Member::Surface(1))];
        assert_eq!(
            toggle_in(&mut model, &mut kept, face, key),
            Toggled::Added(1)
        );
        assert_eq!((model, kept.len()), (Some(face), 0));
        // The limit.
        let (mut primary, mut more) = (Some(Some(Member::Surface(0))), Vec::new());
        more.extend((1..SET_LIMIT).map(|i| Some(Member::Surface(i))));
        let next = Some(Member::Surface(SET_LIMIT));
        assert_eq!(
            toggle_in(&mut primary, &mut more, next, key),
            Toggled::Full(SET_LIMIT)
        );
    }

    #[test]
    fn placed_models_join_the_set_mixed_with_faces_up_to_one_set_command() {
        let key = |member: &Option<Member>| *member;
        let (mut primary, mut more) = (None, Vec::new());
        let a = Some(Member::Placed(3));
        let b = Some(Member::Placed(9));
        let face = Some(Member::Face { brush: 2, side: 9 });
        assert_eq!(
            toggle_in(&mut primary, &mut more, a, key),
            Toggled::Added(1)
        );
        assert_eq!(
            toggle_in(&mut primary, &mut more, b, key),
            Toggled::Added(2)
        );
        // The browser editor's rule: placed models and faces may share a set.
        assert_eq!(
            toggle_in(&mut primary, &mut more, face, key),
            Toggled::Added(3)
        );
        // The newest is the active one (its axes are the gizmo's local ones).
        assert_eq!((primary, more.as_slice()), (Some(face), &[a, b][..]));
        assert_eq!(
            toggle_in(&mut primary, &mut more, a, key),
            Toggled::Removed(2)
        );
        // A map prop (no member) is still selected alone.
        assert_eq!(
            toggle_in(&mut primary, &mut more, None, key),
            Toggled::NotAMember
        );
        // No more placed models than one mapedit_set sets; faces may still join.
        let (mut primary, mut more) = (Some(Some(Member::Placed(0))), Vec::new());
        more.extend((1..PLACED_LIMIT as u32).map(|i| Some(Member::Placed(i))));
        assert_eq!(
            toggle_in(&mut primary, &mut more, Some(Member::Placed(9999)), key),
            Toggled::Full(PLACED_LIMIT)
        );
        assert_eq!(
            toggle_in(&mut primary, &mut more, face, key),
            Toggled::Added(PLACED_LIMIT + 1)
        );
    }

    #[test]
    fn utc_dates_are_european() {
        assert_eq!(utc_text(0), "01/01/1970 00:00:00 UTC");
        // 07/10/2026 01:23:45 UTC.
        assert_eq!(utc_text(1_791_336_225), "07/10/2026 01:23:45 UTC");
    }

    #[test]
    fn boxes_have_twelve_edges_and_points_project_in_front_only() {
        let edges = box_edges(Vec3::ZERO, Vec3::ONE);
        assert_eq!(edges.len(), 12);
        assert!(edges.iter().all(|[a, b]| (*a - *b).length() == 1.0));
        let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z);
        let projection =
            glam::camera::rh::proj::directx::perspective(1.2, 16.0 / 9.0, 2.0, 8_192.0);
        let view_projection = projection * view;
        let centre = project(
            &view_projection,
            [1920.0, 1080.0],
            Vec3::new(100.0, 0.0, 0.0),
        )
        .expect("in front");
        assert!((centre[0] - 960.0).abs() < 0.5 && (centre[1] - 540.0).abs() < 0.5);
        assert!(
            project(
                &view_projection,
                [1920.0, 1080.0],
                Vec3::new(-100.0, 0.0, 0.0)
            )
            .is_none()
        );
    }

    #[test]
    fn shades_are_cut_at_the_eye_and_rows_cross_polygons() {
        assert_eq!(box_triangles(Vec3::ZERO, Vec3::ONE).len(), 12);
        let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z);
        let projection =
            glam::camera::rh::proj::directx::perspective(1.2, 16.0 / 9.0, 2.0, 8_192.0);
        let view_projection = projection * view;
        let viewport = [1920.0, 1080.0];
        // A floor triangle reaching behind the eye keeps its part in front.
        let floor = [
            Vec3::new(-100.0, -50.0, -40.0),
            Vec3::new(200.0, -50.0, -40.0),
            Vec3::new(200.0, 50.0, -40.0),
        ];
        let polygon = screen_polygon(&view_projection, viewport, &floor);
        assert_eq!(polygon.len(), 4, "cut into a quad: {polygon:?}");
        assert!(polygon.iter().all(|p| p[0].is_finite() && p[1].is_finite()));
        // Wholly behind: nothing.
        let behind = floor.map(|p| Vec3::new(-p.x.abs() - 10.0, p.y, p.z));
        assert!(screen_polygon(&view_projection, viewport, &behind).is_empty());
        // A row through a square spans its width; outside, nothing.
        let square = [[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]];
        assert_eq!(row_span(&square, 30.0), Some((10.0, 50.0)));
        assert_eq!(row_span(&square, 60.0), None);
    }

    #[test]
    fn rays_enter_boxes_ahead_only() {
        let low = Vec3::new(10.0, -1.0, -1.0);
        let high = Vec3::new(12.0, 1.0, 1.0);
        assert_eq!(ray_box(Vec3::ZERO, Vec3::X, low, high), Some(10.0));
        assert_eq!(ray_box(Vec3::ZERO, -Vec3::X, low, high), None);
        assert_eq!(ray_box(Vec3::ZERO, Vec3::Y, low, high), None);
        // From inside, the box is met at once.
        assert_eq!(
            ray_box(Vec3::new(11.0, 0.0, 0.0), Vec3::X, low, high),
            Some(0.0)
        );
    }

    #[test]
    fn origins_parse_and_the_nearest_entity_is_named() {
        assert_eq!(
            parse_vector(Some("1 2.5 -3")),
            Some(Vec3::new(1.0, 2.5, -3.0))
        );
        assert_eq!(parse_vector(Some("1 2")), None);
        let entities = sjk_entity::parse_entity_lump(
            b"{\n\"classname\" \"worldspawn\"\n}\n\
              {\n\"classname\" \"light\"\n\"origin\" \"0 0 100\"\n}\n\
              {\n\"classname\" \"misc_model\"\n\"origin\" \"0 0 20\"\n}\n",
        )
        .expect("parses");
        assert_eq!(nearest_entity(&entities, Vec3::ZERO), Some(2));
        assert_eq!(
            nearest_entity(&entities, Vec3::new(0.0, 0.0, 1_000.0)),
            None
        );
    }

    #[test]
    fn a_note_needs_a_confirmed_selection_and_text() {
        let mut notes = Notes::default();
        let directory = std::env::temp_dir().join(format!("sjk-notes-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("directory");
        assert!(notes.save("hello", &directory, &directory).is_none());
        notes.writing = Some(Selection {
            map: "maps/mp/ffa3.bsp".into(),
            pose: "(0 0 0) : 90 0".into(),
            eye: Vec3::new(10.0, 20.0, 76.0),
            forward: Vec3::Y,
            hit: Vec3::new(10.0, 120.0, 76.0),
            normal: -Vec3::Y,
            distance: 100.0,
            surface: Some(Surface {
                index: 7,
                shader: "textures/imperial/basic_floor".into(),
                remapped: Some("textures/video/raven".into()),
                kind: "planar",
                lightmap: 2,
                vertices: 4,
                material: 3,
            }),
            entity: None,
            collision: None,
            face: None,
            edges: Vec::new(),
            triangles: Vec::new(),
            selected: Instant::now(),
            model: None,
            remaps: vec![
                Remap {
                    source: "local",
                    from: "textures/other/wall".into(),
                    to: "textures/other/wall_red".into(),
                },
                Remap {
                    source: "server",
                    from: "textures/imperial/basic_floor".into(),
                    to: "textures/video/raven".into(),
                },
            ],
        });
        let (shot, line, hub) = notes
            .save("  too   shiny  ", &directory, &directory)
            .expect("saved");
        let hub = hub.expect("the hub takes it");
        assert_eq!(hub.text, "too shiny");
        assert_eq!(hub.view, Some([10.0, 20.0, 40.0, 90.0]));
        assert_eq!(hub.shader, "textures/imperial/basic_floor");
        assert_eq!(
            (hub.surface, hub.lighting.as_str()),
            (Some(7), "lightmapped")
        );
        assert_eq!(hub.map, "maps/mp/ffa3.bsp");
        assert!(shot.starts_with("note_"));
        assert!(line.contains("setviewpos 10 20 40 90"), "{line}");
        // The remap's target is what the player saw; the hub has both, and the remaps,
        // the noted shader's first.
        assert_eq!(hub.remapped, "textures/video/raven");
        assert_eq!(
            hub.remaps
                .iter()
                .map(|r| (r.source.as_str(), r.from.as_str()))
                .collect::<Vec<_>>(),
            [
                ("server", "textures/imperial/basic_floor"),
                ("local", "textures/other/wall")
            ]
        );
        assert!(hub.model.is_none());
        assert!(line.contains(
            "textures/imperial/basic_floor remapped to textures/video/raven (surface 7, lightmapped)"
        ));
        let json = std::fs::read_to_string(directory.join("notes.jsonl")).expect("jsonl");
        let value: serde_json::Value = serde_json::from_str(json.trim()).expect("json");
        assert_eq!(value["note"], "too   shiny");
        assert_eq!(value["surface"]["shader"], "textures/imperial/basic_floor");
        assert_eq!(value["surface"]["remapped_to"], "textures/video/raven");
        assert_eq!(value["remaps"][1]["source"], "server");
        assert_eq!(value["remaps"][1]["to"], "textures/video/raven");
        assert!(value["model"].is_null());
        let shot_path = directory.join(format!("{shot}.jpg")).display().to_string();
        assert_eq!(value["screenshot"], shot_path);
        // Saving consumes the selection.
        assert!(notes.save("again", &directory, &directory).is_none());
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A note about a model names it, its shaders with their remaps, and leaves the
    /// surface behind it out; Clear model notes that surface again.
    #[test]
    fn a_placed_models_outline_follows_it_and_others_stay() {
        // The crate at 100 0 0 (yaw 45, scale 2) as a placed edit's pick entity.
        let lump = concat!(
            "{
\"classname\" \"worldspawn\"
}
",
            "{
\"classname\" \"sjk_map_edit\"
\"model\" \"models/test/crate.md3\"
",
            "\"origin\" \"100 0 0\"
\"angles\" \"0 45 0\"
",
            "\"modelscale_vec\" \"2 2 2\"
\"targetname\" \"edit 5\"
}
",
        );
        let entities = sjk_entity::parse_entity_lump(lump.as_bytes()).expect("parses");
        let (_, mut cache) = models::tests::cube_map();
        let vfs = VirtualFileSystem::new();
        let model = pick_model(
            &entities,
            &mut cache,
            &vfs,
            "maps/test.bsp",
            Vec3::ZERO,
            Vec3::X,
        )
        .expect("the crate");
        let selection = |model: Option<Model>| Selection {
            map: "maps/test.bsp".into(),
            pose: String::new(),
            eye: Vec3::ZERO,
            forward: Vec3::X,
            hit: Vec3::X,
            normal: -Vec3::X,
            distance: 1.0,
            surface: None,
            entity: None,
            collision: None,
            face: None,
            edges: vec![[Vec3::ZERO, Vec3::X]],
            triangles: Vec::new(),
            selected: Instant::now(),
            model,
            remaps: Vec::new(),
        };
        let placed = selection(Some(model.clone()));
        assert_eq!(placed.placed_id(), Some(5));
        assert_eq!(placed.member(), Some(Member::Placed(5)));
        let mut prop = model;
        prop.entity.classname = "misc_model_static".into();
        assert_eq!(selection(Some(prop)).member(), None, "a map prop: alone");
        let corners = |selection: &Selection| selection.outline().1.to_vec();
        let before = corners(&placed);
        let mut notes = Notes {
            selection: Some(placed),
            more: vec![selection(None)],
            held: true,
            ..Notes::default()
        };
        // Moved 50 along Y, turned to yaw 135 and halved: every corner goes with it.
        notes.follow_placed(|id| {
            (id == 5).then_some(([100.0, 50.0, 0.0], [0.0, 135.0, 0.0], [1.0; 3]))
        });
        let after = corners(notes.selection.as_ref().expect("kept"));
        let back = glam::Quat::from_rotation_z(-45f32.to_radians());
        let turn = glam::Quat::from_rotation_z(135f32.to_radians());
        for (from, to) in before.iter().flatten().zip(after.iter().flatten()) {
            let local = back * (*from - Vec3::new(100.0, 0.0, 0.0)) / 2.0;
            let expected = Vec3::new(100.0, 50.0, 0.0) + turn * local;
            assert!(to.distance(expected) < 1e-3, "{to} != {expected}");
        }
        // The surface beside it stays, and a second call with the same place changes nothing.
        assert_eq!(notes.more[0].edges, vec![[Vec3::ZERO, Vec3::X]]);
        notes.follow_placed(|_| Some(([100.0, 50.0, 0.0], [0.0, 135.0, 0.0], [1.0; 3])));
        assert_eq!(corners(notes.selection.as_ref().expect("kept")), after);
    }

    #[test]
    fn a_note_about_a_model_names_it_and_clear_model_drops_it() {
        let (entities, mut cache) = models::tests::cube_map();
        let vfs = VirtualFileSystem::new();
        let mut model = pick_model(
            &entities,
            &mut cache,
            &vfs,
            "maps/test.bsp",
            Vec3::ZERO,
            Vec3::X,
        )
        .expect("the crate");
        let remaps = vec![Remap {
            source: "server",
            from: "models/test/crate".into(),
            to: "textures/video/raven".into(),
        }];
        model.shaders[0].remapped = remap_of(&remaps, &model.shaders[0].shader);
        assert_eq!(
            model.shaders[0].remapped.as_deref(),
            Some("textures/video/raven")
        );
        let mut notes = Notes {
            writing: Some(Selection {
                map: "maps/test.bsp".into(),
                pose: "(0 0 -36) : 0 0".into(),
                eye: Vec3::ZERO,
                forward: Vec3::X,
                hit: Vec3::new(400.0, 0.0, 0.0),
                normal: -Vec3::X,
                distance: 400.0,
                surface: Some(Surface {
                    index: 3,
                    shader: "textures/test/wall".into(),
                    remapped: None,
                    kind: "planar",
                    lightmap: 0,
                    vertices: 4,
                    material: 0,
                }),
                entity: None,
                collision: None,
                face: None,
                edges: Vec::new(),
                triangles: Vec::new(),
                selected: Instant::now(),
                model: Some(model),
                remaps,
            }),
            ..Notes::default()
        };
        let selection = notes.writing.as_ref().expect("writing");
        let summary = selection.summary();
        assert!(
            summary.starts_with(
                "misc_model_static models/test/crate.md3 \
                 (models/test/crate remapped to textures/video/raven)"
            ),
            "{summary}"
        );
        assert_eq!(
            selection.model_path().as_deref(),
            Some("models/test/crate.md3")
        );
        let hub = selection.world_note("crate too dark".into());
        assert_eq!(hub.shader, "models/test/crate");
        assert_eq!(hub.remapped, "textures/video/raven");
        assert_eq!((hub.surface, hub.lighting.as_str()), (None, ""));
        assert_eq!(hub.entity, "misc_model_static");
        let noted = hub.model.expect("the model");
        assert_eq!(noted.path, "models/test/crate.md3");
        assert_eq!(noted.entity, Some(1));
        assert_eq!(noted.origin, Some([100.0, 0.0, 0.0]));
        assert_eq!(noted.shaders[0].remapped, "textures/video/raven");
        let json = selection.json("crate too dark", "now", "shot.jpg");
        assert!(json["surface"].is_null());
        assert_eq!(json["entity"]["classname"], "misc_model_static");
        assert_eq!(json["model"]["path"], "models/test/crate.md3");
        assert_eq!(json["model"]["angles"], serde_json::json!([0.0, 45.0, 0.0]));
        assert_eq!(json["model"]["scale"], serde_json::json!([2.0, 2.0, 2.0]));
        assert_eq!(
            json["model"]["shaders"][0]["remapped_to"],
            "textures/video/raven"
        );
        assert!(json["distance"].as_f64().expect("distance") < 100.0);
        // Clear model: the surface behind it again, and no model to clear twice.
        let (subject, model) = notes.clear_model().expect("cleared");
        assert!(
            subject.starts_with("textures/test/wall (surface 3"),
            "{subject}"
        );
        assert_eq!(model, None);
        assert!(notes.clear_model().is_none());
    }

    /// A click's ray leaves the eye through the clicked pixel: the centre is the view.
    #[test]
    fn a_click_casts_a_ray_through_its_pixel() {
        let eye = Vec3::new(0.0, 0.0, 50.0);
        let view = glam::camera::rh::view::look_at_mat4(eye, eye + Vec3::X, Vec3::Z);
        let projection =
            glam::camera::rh::proj::directx::perspective(1.2, 16.0 / 9.0, 2.0, 8_192.0);
        let camera = CameraUniform {
            view_projection: (projection * view).to_cols_array_2d(),
            camera_position: eye.to_array(),
            view_forward: Vec3::X.to_array(),
            ..bytemuck::Zeroable::zeroed()
        };
        let viewport = [1920.0, 1080.0];
        let centre = screen_ray(&camera, viewport, [960.0, 540.0]).expect("a ray");
        assert!(centre.dot(Vec3::X) > 0.9999, "{centre:?}");
        let left = screen_ray(&camera, viewport, [100.0, 540.0]).expect("a ray");
        assert!(left.y > 0.1 && left.x > 0.0, "{left:?}");
        let low = screen_ray(&camera, viewport, [960.0, 1000.0]).expect("a ray");
        assert!(low.z < -0.1, "{low:?}");
        assert!(screen_ray(&camera, [0.0, 0.0], [1.0, 1.0]).is_none());
    }
}
