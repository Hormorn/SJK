//! Shown faces: sides of the world's brushes that the map draws nothing on (caulk,
//! nodraw), or draws as sky, drawn by a map edit with a shader staff choose
//! (`Kind::ShowFace`).
//!
//! q3map2 makes no drawn surface for a caulk or nodraw side, and a BSP keeps no texture
//! projection for brush sides, so the face is rebuilt from its brush's planes (as
//! `r_showBrushes` does) and textured the way Radiant textures a new brush: the axial
//! projection at scale 0.5, no shift, no turn. It is drawn as a small model with entity
//! lighting. A sky side (Sol, 04/11/2026: "remap skybox faces") is drawn the same way,
//! over the sky: every program that draws on the sky's own faces pushes their depth back
//! (`SKY_FACE_PUSH`), so the face drawn on their plane covers them.

use super::document::Align;
use crate::brush_view::kinds::Kind as BrushKind;
use crate::scene_flatten::{ActorDraw, FlattenedScene, ViewerMaterial};
use glam::Vec3;
use sjk_bsp::Bsp;

/// A side smaller than this (square units) is a bevel or a sliver: not offered.
const MIN_AREA: f32 = 0.5;
/// What a texture of unknown size is taken to measure.
const DEFAULT_TEXTURE_SIZE: [f32; 2] = [256.0; 2];
/// `SURF_SKY` (`codemp/game/surfaceflags.h`): the side is drawn as the sky.
pub(crate) const SURF_SKY: u32 = 0x0000_2000;
/// Brush contents whose sides are no sky an edit covers: lava, water, fog and slime.
const VOLUME_CONTENTS: u32 = 0x0000_0002 | 0x0000_0004 | 0x0000_0008 | 0x0002_0000;

/// A world brush side the ray met that edits may show.
#[derive(Clone, Debug)]
pub(crate) struct Face {
    pub(crate) brush: u32,
    pub(crate) side: u32,
    /// The side's own shader (`textures/system/caulk`).
    pub(crate) shader: String,
    /// The side is drawn as the sky (its shader is a sky's): an edit may cover it with a
    /// texture, or swap the map's sky for another.
    pub(crate) sky: bool,
    pub(crate) distance: f32,
    pub(crate) normal: Vec3,
    /// Its corners in order around it.
    pub(crate) polygon: Vec<[f32; 3]>,
}

/// `CONTENTS_SOLID` and `CONTENTS_TRANSLUCENT` (`codemp/game/surfaceflags.h`): q3map2
/// sets the second for `surfaceparm trans`, on tool brushes (`physics_clip`,
/// `nodraw_solid`, hint, clip, trigger) and never on caulk.
pub(crate) const CONTENTS_SOLID: u32 = 0x0000_0001;
pub(crate) const CONTENTS_TRANSLUCENT: u32 = 0x8000_0000;
/// `SURF_NODRAW`: the side draws nothing.
const SURF_NODRAW: u32 = 0x0020_0000;

/// Whether a side of a brush of `contents`, of a shader with `side_contents` and
/// `surface` flags, is one edits may draw, and whether it is a sky side: a sky side, or a
/// structural brush's caulk side (nodraw, solid, opaque, not a clip, trigger, liquid or
/// invisible wall). Told by the compiled flags only: an invisible wall (`physics_clip`,
/// `nodraw_solid`) is `trans`, caulk never is (duel6, 10/11/2026: a click on a physics
/// clip selected it as a hidden face).
pub(crate) fn offered(
    contents: u32,
    side_contents: u32,
    surface: u32,
    name: &[u8],
) -> Option<bool> {
    if is_sky_side(contents, surface) {
        return Some(true);
    }
    let structural = (contents | side_contents) & CONTENTS_TRANSLUCENT == 0;
    (structural
        && matches!(
            BrushKind::of_face(contents, surface, name),
            Some(BrushKind::Caulk | BrushKind::Solid)
        ))
    .then_some(false)
}

/// Whether the face pick goes on through a brush of `contents` whose entered side has
/// `surface` flags and is no face edits draw: a tool volume (clip, trigger, hint, water,
/// an invisible wall), not a drawn wall.
fn passes_through(contents: u32, side_contents: u32, surface: u32) -> bool {
    contents & CONTENTS_SOLID == 0
        || ((contents | side_contents) & CONTENTS_TRANSLUCENT != 0 && surface & SURF_NODRAW != 0)
}

/// Whether side `side` of brush `brush` is one edits may draw ([`offered`]); its shader,
/// and whether it is a sky side.
fn showable(bsp: &Bsp, brush: usize, side: usize) -> Option<(String, bool)> {
    let brush = bsp.brushes().get(brush)?;
    let side = bsp.brush_sides().get(side)?;
    let shader = bsp.shaders().get(side.shader)?;
    let sky = offered(
        brush.content_flags,
        shader.content_flags,
        shader.surface_flags,
        shader.name(),
    )?;
    Some((shader.name_lossy().into_owned(), sky))
}

/// Whether a side of `surface` flags on a brush of `contents` is a sky side an edit may
/// cover: drawn as sky, not the side of a fog or liquid volume.
pub(crate) fn is_sky_side(contents: u32, surface: u32) -> bool {
    surface & SURF_SKY != 0 && contents & VOLUME_CONTENTS == 0
}

/// The sky shaders the map's world draws (`SURF_SKY` on a surface of the world model),
/// each once, in the BSP's shader order: what "all skies of this map" swaps.
pub(crate) fn map_skies(bsp: &Bsp) -> Vec<String> {
    let render = bsp.render();
    let Some(world) = render.models().first() else {
        return Vec::new();
    };
    let mut used: Vec<usize> = world
        .surfaces
        .clone()
        .filter_map(|index| render.surfaces().get(index))
        .map(|surface| surface.shader)
        .filter(|&shader| {
            bsp.shaders()
                .get(shader)
                .is_some_and(|shader| shader.surface_flags & SURF_SKY != 0)
        })
        .collect();
    used.sort_unstable();
    used.dedup();
    let mut names: Vec<String> = Vec::with_capacity(used.len());
    for shader in used {
        let name = bsp.shaders()[shader].name_lossy().into_owned();
        if !names.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
    names
}

/// The world brush side the ray from `eye` along `direction` (normalized) enters first
/// within `reach`, when it is one edits may show. Tool volumes (clip, trigger, hint,
/// water, invisible walls: [`passes_through`]) are no faces: the ray goes on through them
/// to what is behind; a drawn brush stops it.
pub(crate) fn pick(bsp: &Bsp, eye: Vec3, direction: Vec3, reach: f32) -> Option<Face> {
    let world = bsp.render().models().first()?;
    let mut entries: Vec<(f32, usize, usize)> = Vec::new();
    for brush_index in world.brushes.clone() {
        let Some(brush) = bsp.brushes().get(brush_index) else {
            continue;
        };
        let (mut enter, mut exit, mut entered) = (f32::NEG_INFINITY, reach, None);
        let mut missed = false;
        for side_index in brush.sides.clone() {
            let Some(plane) = bsp
                .brush_sides()
                .get(side_index)
                .and_then(|side| bsp.planes().get(side.plane))
            else {
                missed = true;
                break;
            };
            let normal = Vec3::from_array(plane.normal);
            let start = normal.dot(eye) - plane.distance;
            let along = normal.dot(direction);
            if along.abs() < 1e-6 {
                if start > 0.0 {
                    missed = true;
                    break;
                }
                continue;
            }
            let at = -start / along;
            if along < 0.0 {
                if at > enter {
                    enter = at;
                    entered = Some(side_index);
                }
            } else if at < exit {
                exit = at;
            }
            if enter > exit {
                missed = true;
                break;
            }
        }
        // Behind the eye, or the eye inside the brush: not a face it sees.
        let Some(side) = entered.filter(|_| !missed && enter >= 0.0) else {
            continue;
        };
        entries.push((enter, brush_index, side));
    }
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (distance, brush, side, shader, sky) = entries.into_iter().find_map(|(d, b, s)| {
        if let Some((shader, sky)) = showable(bsp, b, s) {
            return Some(Some((d, b, s, shader, sky)));
        }
        let brush = &bsp.brushes()[b];
        let shader = &bsp.shaders()[bsp.brush_sides()[s].shader];
        // a tool volume: on through it; a drawn wall: nothing behind it is seen
        (!passes_through(
            brush.content_flags,
            shader.content_flags,
            shader.surface_flags,
        ))
        .then_some(None)
    })??;
    let polygon = bsp.brush_side_polygon(brush, side);
    if area(&polygon) < MIN_AREA {
        return None;
    }
    let plane = bsp.planes()[bsp.brush_sides()[side].plane];
    Some(Face {
        brush: brush as u32,
        side: side as u32,
        shader,
        sky,
        distance,
        normal: Vec3::from_array(plane.normal),
        polygon,
    })
}

/// Side `side` of brush `brush` as an edit names it, when it is still one edits may show
/// and still carries `shader`: its corners and its plane's normal.
pub(crate) fn polygon(
    bsp: &Bsp,
    brush: u32,
    side: u32,
    shader: &str,
) -> Option<(Vec<[f32; 3]>, [f32; 3])> {
    let (brush, side) = (brush as usize, side as usize);
    let model = bsp.render().models().first()?;
    if !model.brushes.contains(&brush) {
        return None;
    }
    if !showable(bsp, brush, side)?.0.eq_ignore_ascii_case(shader) {
        return None;
    }
    let polygon = bsp.brush_side_polygon(brush, side);
    if area(&polygon) < MIN_AREA {
        return None;
    }
    let normal = bsp.planes()[bsp.brush_sides()[side].plane].normal;
    Some((polygon, normal))
}

/// The shader of side `side` of brush `brush` when it is a world brush's side that edits
/// may show (caulk, nodraw, sky) and not a sliver, and whether it is a sky side: what a
/// face of the browser editor's multiple selection is checked with.
pub(crate) fn world_side_shader(bsp: &Bsp, brush: u32, side: u32) -> Option<(String, bool)> {
    let (brush, side) = (brush as usize, side as usize);
    let model = bsp.render().models().first()?;
    if !model.brushes.contains(&brush) || !bsp.brushes().get(brush)?.sides.contains(&side) {
        return None;
    }
    let shown = showable(bsp, brush, side)?;
    (area(&bsp.brush_side_polygon(brush, side)) >= MIN_AREA).then_some(shown)
}

/// Side `side` of brush `brush` as the ray from `eye` along `direction` (normalized) meets
/// its plane from the front, when edits may show it: the face the browser editor's brush
/// view was clicked on.
pub(crate) fn met(bsp: &Bsp, brush: u32, side: u32, eye: Vec3, direction: Vec3) -> Option<Face> {
    let (shader, sky) = world_side_shader(bsp, brush, side)?;
    let plane = bsp.planes()[bsp.brush_sides()[side as usize].plane];
    let normal = Vec3::from_array(plane.normal);
    let along = normal.dot(direction);
    if along >= -1e-6 {
        return None;
    }
    let distance = (plane.distance - normal.dot(eye)) / along;
    (distance >= 0.0).then(|| Face {
        brush,
        side,
        shader,
        sky,
        distance,
        normal,
        polygon: bsp.brush_side_polygon(brush as usize, side as usize),
    })
}

/// The middle of `polygon`'s corners.
pub(crate) fn centre(polygon: &[[f32; 3]]) -> [f32; 3] {
    let sum = polygon
        .iter()
        .fold(Vec3::ZERO, |sum, point| sum + Vec3::from_array(*point));
    (sum / polygon.len().max(1) as f32).to_array()
}

/// The corners of the face `polygon` (`normal` out of its brush) as drawn: positions
/// relative to `origin` (where it is lit from), its texture laid as `align` lays it on a
/// `size` (width, height) texture. What [`append`] draws, and what a new alignment
/// writes over a face's mesh in place.
pub(crate) fn vertices(
    polygon: &[[f32; 3]],
    normal: [f32; 3],
    origin: [f32; 3],
    size: Option<[u32; 2]>,
    align: &Align,
) -> Vec<crate::GpuVertex> {
    polygon
        .iter()
        .zip(texture_coordinates(polygon, normal, size, align))
        .map(|(point, texture_coordinates)| crate::GpuVertex {
            position: (Vec3::from_array(*point) - Vec3::from_array(origin)).to_array(),
            normal,
            color: [1.0; 4],
            texture_coordinates,
            lightmap_coordinates: [0.0; 2],
        })
        .collect()
}

/// The triangles of the face `polygon` (corners in order, `normal` out of its brush) as a
/// fan of indices from `base`, wound as the map's drawn faces are (clockwise seen from the
/// front), so a shader culling back faces shows it on the side its normal points to.
pub(crate) fn fan_indices(polygon: &[[f32; 3]], normal: [f32; 3], base: u32) -> Vec<u32> {
    if polygon.len() < 3 {
        return Vec::new();
    }
    let corner = |i: usize| Vec3::from_array(polygon[i]);
    let reversed = (corner(1) - corner(0))
        .cross(corner(2) - corner(0))
        .dot(Vec3::from_array(normal))
        > 0.0;
    (2..polygon.len() as u32)
        .flat_map(|corner| {
            let (b, c) = if reversed {
                (corner, corner - 1)
            } else {
                (corner - 1, corner)
            };
            [base, base + b, base + c]
        })
        .collect()
}

/// Append the face `polygon` (corners in order, `normal` out of its brush) drawn with
/// `shader` to `scene`, its positions relative to `origin` (where it is lit from), its
/// texture laid as `align` lays it on a `size` (width, height) texture.
pub(crate) fn append(
    scene: &mut FlattenedScene,
    polygon: &[[f32; 3]],
    normal: [f32; 3],
    origin: [f32; 3],
    shader: &str,
    size: Option<[u32; 2]>,
    align: &Align,
) -> Result<Vec<ActorDraw>, Box<dyn std::error::Error>> {
    if polygon.len() < 3 {
        return Err("a face has three corners at least".into());
    }
    let vertex_base: u32 = scene.vertices.len().try_into()?;
    let index_start: u32 = scene.indices.len().try_into()?;
    scene
        .vertices
        .extend(vertices(polygon, normal, origin, size, align));
    scene
        .indices
        .extend(fan_indices(polygon, normal, vertex_base));
    let material = scene.materials.len();
    scene.materials.push(ViewerMaterial {
        shader: shader.to_owned(),
        lightmap: -1,
    });
    Ok(vec![ActorDraw {
        indices: index_start..scene.indices.len().try_into()?,
        material,
    }])
}

/// The size a texture of `size` is taken to have: 256 square when unknown.
fn texture_size(size: Option<[u32; 2]>) -> [f32; 2] {
    size.map_or(DEFAULT_TEXTURE_SIZE, |[w, h]| {
        [w.max(1) as f32, h.max(1) as f32]
    })
}

/// The texture coordinates of `polygon`'s corners as Radiant lays a brush face's
/// texture (`Face_TextureVectors`): the axial projection ([`texture_axes`]) turned by
/// `align.rotate` within its plane, each axis divided by its `align.scale`, shifted by
/// `align.shift` texels, over an image of `size` (256 square when unknown). At the
/// default (scale 0.5, no shift, no turn) it is how Radiant textures a new brush. What
/// [`append`] draws, and the browser editor's page too.
pub(crate) fn texture_coordinates(
    polygon: &[[f32; 3]],
    normal: [f32; 3],
    size: Option<[u32; 2]>,
    align: &Align,
) -> Vec<[f32; 2]> {
    let size = texture_size(size);
    let (s, t) = aligned_axes(normal, align.rotate);
    polygon
        .iter()
        .map(|point| {
            let world = Vec3::from_array(*point);
            [
                (world.dot(s) / align.scale[0] + align.shift[0]) / size[0],
                (world.dot(t) / align.scale[1] + align.shift[1]) / size[1],
            ]
        })
        .collect()
}

/// The alignment that fits the texture once across `polygon` and once down it (Radiant's
/// Fit 1 x 1): each scale set so the face's extent along the texture's axis is the
/// image's size, each shift so its low edge starts the image; the turn and each scale's
/// sign (a flip) are kept. A face with no extent along an axis keeps that axis.
pub(crate) fn fit(
    polygon: &[[f32; 3]],
    normal: [f32; 3],
    size: Option<[u32; 2]>,
    align: &Align,
) -> Align {
    let size = texture_size(size);
    let (s, t) = aligned_axes(normal, align.rotate);
    let mut fitted = *align;
    for (axis, along) in [s, t].into_iter().enumerate() {
        let (low, high) = polygon
            .iter()
            .map(|point| Vec3::from_array(*point).dot(along))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| {
                (low.min(value), high.max(value))
            });
        let extent = high - low;
        // (A NaN extent, from a broken polygon, is no extent either.)
        if extent.is_nan() || extent <= 1e-3 {
            continue;
        }
        let sign = if align.scale[axis] < 0.0 { -1.0 } else { 1.0 };
        let scale = (extent / size[axis]).clamp(
            *super::document::ALIGN_SCALE.start(),
            *super::document::ALIGN_SCALE.end(),
        ) * sign;
        // The low edge at 0 (the high one when flipped), the other at 1.
        let start = if sign > 0.0 { low } else { high };
        fitted.scale[axis] = scale;
        fitted.shift[axis] = -start / scale;
    }
    fitted
}

/// Radiant's texture axes for a face with `normal`, turned `degrees` within the plane
/// of their two world axes (`Face_TextureVectors`: each axis's components on the pair
/// turned as a 2D vector).
fn aligned_axes(normal: [f32; 3], degrees: f32) -> (Vec3, Vec3) {
    let (s, t) = texture_axes(normal);
    if degrees == 0.0 {
        return (s, t);
    }
    let component = |axis: Vec3| {
        axis.to_array()
            .iter()
            .position(|value| *value != 0.0)
            .unwrap_or(0)
    };
    let (sv, tv) = (component(s), component(t));
    let (sin, cos) = degrees.to_radians().sin_cos();
    let turn = |axis: Vec3| {
        let mut turned = axis.to_array();
        let (a, b) = (turned[sv], turned[tv]);
        turned[sv] = cos * a - sin * b;
        turned[tv] = sin * a + cos * b;
        Vec3::from_array(turned)
    };
    (turn(s), turn(t))
}

/// Radiant's texture axes for a face with `normal` (`TextureAxisFromPlane`): the world
/// axis nearest the normal picks the pair the texture is projected along.
fn texture_axes(normal: [f32; 3]) -> (Vec3, Vec3) {
    const BASE: [[[f32; 3]; 3]; 6] = [
        [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
        [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
        [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
        [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
        [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
    ];
    let normal = Vec3::from_array(normal);
    let mut best = (0, 0.0);
    for (index, [axis, _, _]) in BASE.iter().enumerate() {
        let dot = normal.dot(Vec3::from_array(*axis));
        if dot > best.1 {
            best = (index, dot);
        }
    }
    let [_, s, t] = BASE[best.0];
    (Vec3::from_array(s), Vec3::from_array(t))
}

fn area(points: &[[f32; 3]]) -> f32 {
    let Some(first) = points.first().map(|p| Vec3::from_array(*p)) else {
        return 0.0;
    };
    let mut sum = Vec3::ZERO;
    for pair in points[1..].windows(2) {
        sum += (Vec3::from_array(pair[0]) - first).cross(Vec3::from_array(pair[1]) - first);
    }
    sum.length() * 0.5
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A room: a stone floor, a caulk wall, a sky lid and a water pool's brush.
    pub(crate) fn sky_room() -> Bsp {
        use sjk_bsp::{CollisionShader, box_brush, write_collision_map};
        let shader = |name: &str, surface_flags, content_flags| CollisionShader {
            name: name.into(),
            surface_flags,
            content_flags,
        };
        const SOLID: u32 = 1;
        const NODRAW: u32 = 0x0020_0000;
        let shaders = [
            shader("textures/a/stone", 0, SOLID),
            shader("textures/skies/night", SURF_SKY, SOLID),
            shader("textures/system/caulk", NODRAW, SOLID),
            shader("textures/a/water_sky", SURF_SKY, 0x4),
        ];
        let brushes = [
            box_brush([-256.0, -256.0, -16.0], [256.0, 256.0, 0.0], 0),
            box_brush([-256.0, -256.0, 512.0], [256.0, 256.0, 528.0], 1),
            box_brush([256.0, -256.0, 0.0], [272.0, 256.0, 512.0], 2),
            box_brush([-256.0, -256.0, 0.0], [-128.0, -128.0, 32.0], 3),
        ];
        Bsp::parse(&write_collision_map(
            "{\n\"classname\" \"worldspawn\"\n}\n",
            &shaders,
            &brushes,
        ))
        .expect("synthetic map parses")
    }

    #[test]
    fn a_sky_side_is_a_face_edits_may_cover() {
        let bsp = sky_room();
        let eye = Vec3::new(0.0, 0.0, 64.0);
        // up: the sky lid's underside, a sky face
        let sky = pick(&bsp, eye, Vec3::Z, 4096.0).expect("the sky lid");
        assert!(sky.sky);
        assert_eq!(sky.shader, "textures/skies/night");
        assert!((sky.distance - 448.0).abs() < 1e-3);
        assert_eq!(sky.normal, Vec3::NEG_Z);
        // the caulk wall is a face, not a sky; the stone floor is neither
        let wall = pick(&bsp, eye, Vec3::X, 4096.0).expect("the caulk wall");
        assert!(!wall.sky && wall.shader.ends_with("caulk"));
        assert!(pick(&bsp, eye, Vec3::NEG_Z, 4096.0).is_none());
        // the checks the edits and the page make agree
        let (brush, side) = (sky.brush, sky.side);
        assert_eq!(
            world_side_shader(&bsp, brush, side),
            Some(("textures/skies/night".to_owned(), true))
        );
        let (corners, normal) = polygon(&bsp, brush, side, "TEXTURES/SKIES/NIGHT").expect("shown");
        assert_eq!(corners.len(), 4);
        assert_eq!(normal, [0.0, 0.0, -1.0]);
        assert!(polygon(&bsp, brush, side, "textures/system/caulk").is_none());
        let below = met(&bsp, brush, side, eye, Vec3::Z).expect("met from below");
        assert!(below.sky && (below.distance - 448.0).abs() < 1e-3);
        assert!(met(&bsp, brush, side, Vec3::new(0.0, 0.0, 600.0), Vec3::NEG_Z).is_none());
        // a sky-flagged liquid's side is no sky an edit covers
        assert!(!is_sky_side(0x4, SURF_SKY));
        assert!(is_sky_side(1, SURF_SKY) && !is_sky_side(1, 0));
    }

    /// Clips, triggers and invisible walls are no faces: the pick goes through them to the
    /// caulk behind, and a physics clip alone is never offered (duel6, 10/11/2026). The
    /// flags are retail duel6's and ffa3's own.
    #[test]
    fn the_pick_goes_through_clips_and_triggers_to_the_caulk_behind() {
        use sjk_bsp::{CollisionShader, box_brush, write_collision_map};
        let shader = |name: &str, surface_flags: u32, content_flags: u32| CollisionShader {
            name: name.into(),
            surface_flags,
            content_flags,
        };
        let shaders = [
            shader("textures/system/caulk", 0x30_0000, 0x8001),
            shader("textures/system/physics_clip", 0x20_0000, 0x8000_8001),
            shader("textures/system/trigger", 0x20_0000, 0x8000_0400),
            shader("textures/system/nodraw_solid", 0x30_0000, 0x8000_0001),
            shader("textures/system/clip", 0x38_0000, 0x8000_0030),
            shader("textures/a/stone", 0, 0x8001),
        ];
        let brushes = [
            // the caulk wall at x 256
            box_brush([256.0, -256.0, 0.0], [272.0, 256.0, 256.0], 0),
            // a physics clip, a trigger and a player clip before it
            box_brush([64.0, -256.0, 0.0], [80.0, 256.0, 256.0], 1),
            box_brush([128.0, -256.0, 0.0], [144.0, 256.0, 256.0], 2),
            box_brush([192.0, -256.0, 0.0], [200.0, 256.0, 256.0], 4),
            // a nodraw_solid wall at -x, a stone wall at +y with caulk behind it
            box_brush([-80.0, -256.0, 0.0], [-64.0, 256.0, 256.0], 3),
            box_brush([-256.0, 64.0, 0.0], [256.0, 80.0, 256.0], 5),
            box_brush([-256.0, 96.0, 0.0], [256.0, 112.0, 256.0], 0),
        ];
        let bsp = Bsp::parse(&write_collision_map(
            "{
\"classname\" \"worldspawn\"
}
",
            &shaders,
            &brushes,
        ))
        .expect("synthetic map parses");
        let eye = Vec3::new(0.0, 0.0, 64.0);
        let wall = pick(&bsp, eye, Vec3::X, 4096.0).expect("the caulk behind the clips");
        assert_eq!(wall.shader, "textures/system/caulk");
        assert!((wall.distance - 256.0).abs() < 1e-3, "{}", wall.distance);
        // an invisible wall alone is not a face
        assert!(pick(&bsp, eye, Vec3::NEG_X, 4096.0).is_none());
        // a drawn wall stops the pick: the caulk behind it is not seen
        assert!(pick(&bsp, eye, Vec3::Y, 4096.0).is_none());
        // the flags alone tell them apart
        assert_eq!(
            offered(0x8001, 0x8001, 0x30_0000, b"textures/system/caulk"),
            Some(false)
        );
        assert_eq!(
            offered(0x8000_8001, 0x8000_8001, 0x20_0000, b"x/physics_clip"),
            None
        );
        assert_eq!(
            offered(0x8000_0001, 0x8000_0001, 0x30_0000, b"x/nodraw_solid"),
            None
        );
        assert_eq!(
            offered(0x8000_0400, 0x8000_0400, 0x20_0000, b"x/trigger"),
            None
        );
    }

    #[test]
    fn faces_take_radiants_axial_projection() {
        let (s, t) = texture_axes([0.0, 0.0, 1.0]);
        assert_eq!((s, t), (Vec3::X, Vec3::NEG_Y));
        let (s, t) = texture_axes([0.0, -1.0, 0.0]);
        assert_eq!((s, t), (Vec3::X, Vec3::NEG_Z));
        // A slope nearer a wall than the floor takes the wall's axes.
        let (s, t) = texture_axes([0.9, 0.0, 0.43]);
        assert_eq!((s, t), (Vec3::Y, Vec3::NEG_Z));
    }

    #[test]
    fn a_face_is_lit_from_its_middle_and_tiles_at_half_scale() {
        let square = [
            [0.0, 0.0, 0.0],
            [128.0, 0.0, 0.0],
            [128.0, 128.0, 0.0],
            [0.0, 128.0, 0.0],
        ];
        let middle = centre(&square);
        assert_eq!(middle, [64.0, 64.0, 0.0]);
        let mut scene = FlattenedScene::default();
        let draws = append(
            &mut scene,
            &square,
            [0.0, 0.0, 1.0],
            middle,
            "textures/a/wall",
            Some([256, 256]),
            &Align::default(),
        )
        .unwrap();
        assert_eq!(draws.len(), 1);
        // Counter-clockwise seen from above: turned over to the map's winding.
        assert_eq!(scene.indices, [0, 2, 1, 0, 3, 2]);
        assert_eq!(scene.vertices[0].position, [-64.0, -64.0, 0.0]);
        // 128 units of a 256 texel texture at scale 0.5: exactly one tile.
        assert_eq!(scene.vertices[1].texture_coordinates, [1.0, 0.0]);
        assert_eq!(scene.vertices[3].texture_coordinates, [0.0, -1.0]);
        assert_eq!(scene.materials[0].shader, "textures/a/wall");
        assert_eq!(area(&square), 128.0 * 128.0);
    }

    /// Radiant's alignment: a shift moves the texture by texels, a scale by units a texel
    /// covers (negative flips), a turn turns its axes in their plane, and Fit lays the
    /// image once across the face, whatever the turn or a flip.
    #[test]
    fn a_faces_texture_shifts_scales_turns_and_fits_as_radiants() {
        let floor = [
            [0.0, 0.0, 0.0],
            [128.0, 0.0, 0.0],
            [128.0, 64.0, 0.0],
            [0.0, 64.0, 0.0],
        ];
        let up = [0.0, 0.0, 1.0];
        let size = Some([256, 128]);
        let at = |align: &Align| texture_coordinates(&floor, up, size, align);
        let plain = at(&Align::default());
        // 128 units of a 256-wide image at 0.5: one image across; y runs down the t axis.
        assert_eq!(plain[1], [1.0, 0.0]);
        assert_eq!(plain[3], [0.0, -1.0]);
        let shifted = at(&Align {
            shift: [64.0, 32.0],
            ..Align::default()
        });
        assert_eq!(shifted[0], [0.25, 0.25]);
        let doubled = at(&Align {
            scale: [1.0, 1.0],
            ..Align::default()
        });
        assert_eq!(doubled[1], [0.5, 0.0]);
        let flipped = at(&Align {
            scale: [-0.5, 0.5],
            ..Align::default()
        });
        assert_eq!(flipped[1], [-1.0, 0.0]);
        // A quarter turn: what ran along x now runs along y.
        let turned = at(&Align {
            rotate: 90.0,
            ..Align::default()
        });
        assert!((turned[1][0]).abs() < 1e-5, "{:?}", turned[1]);
        assert!((turned[1][1] - 2.0).abs() < 1e-5, "{:?}", turned[1]);
        // Fit: the image once across and down, its corner on the face's.
        for align in [
            Align::default(),
            Align {
                rotate: 30.0,
                ..Align::default()
            },
            Align {
                scale: [-0.5, 0.25],
                shift: [17.0, 3.0],
                ..Align::default()
            },
        ] {
            let fitted = fit(&floor, up, size, &align);
            assert_eq!(fitted.rotate, align.rotate);
            assert_eq!(
                fitted.scale.map(f32::signum),
                align.scale.map(f32::signum),
                "a flip is kept"
            );
            let uv = texture_coordinates(&floor, up, size, &fitted);
            for axis in 0..2 {
                let (low, high) = uv.iter().fold((f32::MAX, f32::MIN), |(l, h), c| {
                    (l.min(c[axis]), h.max(c[axis]))
                });
                assert!(
                    low.abs() < 1e-4 && (high - 1.0).abs() < 1e-4,
                    "{align:?} {uv:?}"
                );
            }
        }
        assert!(fit(&floor, up, size, &Align::default()).valid());
    }
}
