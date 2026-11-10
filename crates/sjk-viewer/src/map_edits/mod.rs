//! Map edits: SJK's own fixes to a map, drawn on the client only. Staff hide the map's
//! props and surfaces, place models and remap shaders in game ([`editor`]); every SJK
//! player gets the edits from the hub (the identity worker's
//! [`sjk_identity::map_edits`] keeps them in [`FOLDER`]) and sees them while
//! `cg_mapEdits` is on (the default).
//!
//! Edits change what is drawn, never collision: the server still has a hidden prop's
//! clip brushes, and nothing stops a player walking through a placed model. An edit
//! applies on every server, or on one server only (its address), and only to the map
//! file whose checksum its document carries ([`document`]).
//!
//! The world applies the edits that hold for its server as a desired state, compared
//! with what it applied last, when the document, the server or the setting changes:
//! - hidden props drop out of [`crate::static_models::StaticModels`];
//! - hidden surfaces get degenerate triangles in the world's index buffer (every pass
//!   draws nothing there) and leave the triangles picks and marks use;
//! - placed models load on demand ([`crate::model_demand`]) and are drawn each frame;
//! - remaps join the world's remaps as source `edit`, over the map's and the server's;
//! - shown faces (caulk and nodraw brush sides) are built as small models once and drawn
//!   each frame ([`faces`]);
//! - a weather edit's commands replace the map's own weather ([`crate::weather`]);
//! - added lights are drawn as dynamic lights each frame, and lamp edits turn the map's
//!   own real-time lamps off or move them when its world is built ([`lights`]).
//!
//! Staff edit in game ([`editor`], [`panel`]) or in a web browser ([`web`]).

pub(crate) mod clip;
pub(crate) mod document;
pub(crate) mod editor;
pub(crate) mod faces;
pub(crate) mod fly;
pub(crate) mod fly_to;
pub(crate) mod gizmo;
pub(crate) mod lights;
pub(crate) mod linked;
pub(crate) mod nearby;
pub(crate) mod panel;
pub(crate) mod panel_lights;
mod panel_view;
pub(crate) mod pictures;
pub(crate) mod preview;
pub(crate) mod recent;
pub(crate) mod select;
pub(crate) mod suggest;
pub(crate) mod web;

use crate::{ActorInstance, GpuState};
use document::{Document, Kind};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

/// `cg_mapEdits`: draw SJK's map edits (on by default).
pub(crate) const CVAR: &str = "cg_mapEdits";
/// The command that asks the hub for the map edits now, for any player.
pub(crate) const SYNC_COMMAND: &str = "mapedits_sync";
pub(crate) const SYNC_HELP: &str =
    "Ask the SJK hub for the latest map edits now (they also come by themselves)";
/// The folder of the documents, in the SJK config folder.
pub(crate) const FOLDER: &str = "map_edits";
/// The class picks give a placed model, so they can tell it from the map's entities.
pub(crate) const PLACED_CLASS: &str = "sjk_map_edit";
/// The model name of a shown face's mesh; its variant names the face and its shader, so
/// no model of the game data is ever taken for one.
const FACE_MODEL: &str = "*sjk_face";

/// The map edits of one world.
#[derive(Default)]
pub(crate) struct State {
    lookup: Lookup,
    /// A map whose checksum is read even with no edits for it ([`State::want_checksum`]).
    checksum_wanted: Option<String>,
    /// The loaded map's document, when it has one (or staff began one).
    pub(super) document: Option<Document>,
    /// Bumped on every change of [`Self::document`].
    revision: u64,
    /// What [`GpuState::refresh_map_edits`] last applied, and for what.
    applied: Applied,
    applied_for: Option<(u64, Option<std::net::SocketAddr>, bool)>,
    /// The placed models drawn now.
    placed: Vec<Placed>,
    /// The added lights drawn now ([`lights`]).
    pub(crate) lights: Vec<lights::EditLight>,
    /// The shown faces drawn now: each one's mesh in `object_meshes` and where it is lit.
    faces: Vec<(usize, ActorInstance)>,
    /// The remap edits applied now (from, to): what a remap preview stands over and goes
    /// back to ([`preview`]).
    pub(crate) remaps: Vec<(String, String)>,
    /// `object_meshes.len()` when [`Self::placed`] last looked their meshes up.
    meshes_seen: usize,
    /// The hub copies' revision last read, and when the hub's state was last read.
    hub_revision: u64,
    polled: Option<std::time::Instant>,
    /// What the identity worker's last look at the hub's map edits did: its summary, or
    /// why it fell short (logged when it changes; the panel shows a failure).
    pub(crate) hub_note: Option<Result<String, String>>,
    /// The editor's changes to this map's edits, to undo and redo ([`editor`]).
    pub(crate) history: editor::History,
    pub(crate) editor: editor::Editor,
    /// The editor's panel ([`panel`]).
    pub(crate) panel: panel::Panel,
    /// The editor's camera: the free camera, flown with the right mouse button ([`fly`]).
    pub(crate) fly: fly::Fly,
    /// The browser editor's server while it runs (`mapedit_web`, [`web`]).
    pub(crate) web: Option<web::http::Server>,
    /// The shaders and models used lately, once read ([`recent`]).
    pub(crate) recent: Option<recent::Recent>,
    /// The browser editor's brush faces, linked caulk and nearby shaders, per map.
    pub(crate) web_cache: web::Cache,
    /// The in-game transform gizmo ([`gizmo`]).
    pub(crate) gizmo: gizmo::Gizmo,
}

/// Finding the loaded map's document.
#[derive(Default)]
enum Lookup {
    #[default]
    Idle,
    /// Reading the map's checksum on a worker.
    Checksum {
        map: String,
        receiver: Receiver<Option<i32>>,
    },
    /// The map, its checksum (`None` when it could not be read) and its document's file.
    Done { map: String, checksum: Option<i32> },
}

#[derive(Default)]
struct Applied {
    props: BTreeSet<u32>,
    surfaces: BTreeSet<u32>,
}

/// A placed model as drawn.
struct Placed {
    id: u32,
    model: String,
    instance: ActorInstance,
    origin: [f32; 3],
    angles: [f32; 3],
    scale: [f32; 3],
    /// Its rigid mesh, once loaded.
    mesh: Option<usize>,
}

impl State {
    /// Read the checksum of map `map` (any case) even with no edits for it, when it is
    /// the loaded one: a mystery box may stand on it (`mystery_box`). Allocation-free
    /// once asked.
    pub(crate) fn want_checksum(&mut self, map: &str) {
        if self
            .checksum_wanted
            .as_deref()
            .is_some_and(|wanted| wanted.eq_ignore_ascii_case(map))
        {
            return;
        }
        let map = map.to_ascii_lowercase();
        if matches!(&self.lookup, Lookup::Done { map: done, checksum: None } if *done == map) {
            self.lookup = Lookup::Idle;
        }
        self.checksum_wanted = Some(map);
    }

    /// The loaded map's name and checksum, once known.
    pub(crate) fn map(&self) -> Option<(&str, i32)> {
        match &self.lookup {
            Lookup::Done {
                map,
                checksum: Some(checksum),
            } => Some((map, *checksum)),
            _ => None,
        }
    }

    /// The document changed: save it and apply it again.
    pub(super) fn changed(&mut self, directory: Option<&Path>) -> Result<(), String> {
        self.revision += 1;
        let (Some(document), Some(directory)) = (&self.document, directory) else {
            return Ok(());
        };
        save(directory, document)
    }

    /// Models placed edits name that no mesh is loaded for yet, for the demand loader.
    pub(crate) fn wanted_models(&self) -> impl Iterator<Item = &str> {
        self.placed
            .iter()
            .filter(|placed| placed.mesh.is_none())
            .map(|placed| placed.model.as_str())
    }

    /// The edit whose placed model has entity number `number` in a pick made over
    /// [`State::pick_entities`] of a lump of `lump` entities.
    pub(crate) fn placed_edit(&self, number: usize, lump: usize) -> Option<u32> {
        number
            .checked_sub(lump)
            .and_then(|index| self.placed.get(index))
            .map(|placed| placed.id)
    }

    /// The entities picks look at while edits are drawn: the map's, without the props
    /// hidden, then one per placed model. `None` when no edit changes them.
    pub(crate) fn pick_entities(&self, bsp: &sjk_bsp::Bsp) -> Option<Vec<sjk_entity::Entity>> {
        if self.applied.props.is_empty() && self.placed.is_empty() {
            return None;
        }
        let mut entities = sjk_entity::parse_entity_lump(bsp.entities()).ok()?;
        for &number in &self.applied.props {
            if let Some(entity) = entities.get_mut(number as usize) {
                let fields = entity
                    .fields()
                    .iter()
                    .filter(|(key, _)| key != "model")
                    .cloned()
                    .collect();
                *entity = sjk_entity::Entity::from_fields(fields);
            }
        }
        let vector = |v: [f32; 3]| format!("{} {} {}", v[0], v[1], v[2]);
        entities.extend(self.placed.iter().map(|placed| {
            sjk_entity::Entity::from_fields(vec![
                ("classname".into(), PLACED_CLASS.into()),
                ("model".into(), placed.model.clone()),
                ("origin".into(), vector(placed.origin)),
                ("angles".into(), vector(placed.angles)),
                ("modelscale_vec".into(), vector(placed.scale)),
                ("targetname".into(), format!("edit {}", placed.id)),
            ])
        }));
        Some(entities)
    }

    /// Move placed edit `id` where it is drawn, without saving (`mapedit_grab`).
    pub(super) fn preview_placement(&mut self, id: u32, origin: [f32; 3], angles: [f32; 3]) {
        if let Some(placed) = self.placed.iter_mut().find(|placed| placed.id == id) {
            placed.origin = origin;
            placed.angles = angles;
            placed.instance = instance(origin, angles, placed.scale);
        }
    }
}

fn instance(origin: [f32; 3], angles: [f32; 3], scale: [f32; 3]) -> ActorInstance {
    ActorInstance::new(
        origin,
        sjk_client::legacy_angles_to_quaternion(angles),
        scale,
    )
}

/// The documents' folder in `config` (the SJK config folder).
pub(crate) fn folder(config: &Path) -> PathBuf {
    config.join(FOLDER)
}

/// Write `document` to its file in `directory`.
pub(crate) fn save(directory: &Path, document: &Document) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = directory.join(document::file_name(&document.map, document.checksum));
    let text = serde_json::to_string_pretty(document).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text)
        .and_then(|()| std::fs::rename(&temporary, &path))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Whether `directory` holds a document of `map`, whatever its checksum: only then is
/// the map's checksum worth reading.
fn has_documents(directory: &Path, map: &str) -> bool {
    let prefix = document::file_prefix(map);
    std::fs::read_dir(directory).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.strip_prefix(&prefix)
                    .and_then(|rest| rest.strip_suffix(".json"))
                    .is_some_and(|checksum| checksum.parse::<i32>().is_ok())
            })
        })
    })
}

/// `map`'s document for `checksum` in `directory`, if there is a readable one.
fn load(directory: &Path, map: &str, checksum: i32) -> Option<Document> {
    let path = directory.join(document::file_name(map, checksum));
    let text = std::fs::read_to_string(&path).ok()?;
    match Document::parse(&text, map, checksum) {
        Ok(document) => Some(document),
        Err(error) => {
            crate::log::progress(format_args!(
                "map edits {} ignored: {error}",
                path.display()
            ));
            None
        }
    }
}

impl GpuState {
    /// The map edits' folder, when there is a console (and so a config folder).
    pub(crate) fn map_edits_folder(&self) -> Option<PathBuf> {
        self.console
            .as_ref()
            .map(|console| folder(console.config_directory()))
    }

    /// The server the world shows, for edits made for one server: its address.
    pub(crate) fn map_edits_server(&self) -> Option<String> {
        self.live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(|session| session.server().to_string())
    }

    /// `mapedits_sync`: ask the hub for its map edits now; the map's edits change when
    /// newer ones arrive (the log says what the look did).
    pub(crate) fn map_edits_sync_command(&mut self) -> Result<Vec<String>, String> {
        if self
            .console
            .as_ref()
            .is_some_and(|console| !console.map_edits_on())
        {
            return Err(format!(
                "SJK map edits are off (Settings > Game, {CVAR} 1 turns them on)"
            ));
        }
        if !crate::player_identity::hub_on() {
            return Err("Map edits come through the SJK hub, which is off on this PC".into());
        }
        crate::player_identity::refresh_map_edits();
        Ok(vec!["Asking the SJK hub for the latest map edits".into()])
    }

    /// Once a frame: find the map's document, then apply what changed.
    pub(crate) fn refresh_map_edits(&mut self) {
        self.map_edit_web_frame();
        if self.is_menu_world || crate::menu_pictures::pictured(&self.world_load_map).is_some() {
            return;
        }
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map);
        if map.is_empty() {
            return;
        }
        // Allocation-free unless the map changed or its checksum is being read.
        let looked_up = match &self.map_edits.lookup {
            Lookup::Done { map: done, .. } => done.eq_ignore_ascii_case(map),
            _ => false,
        };
        if !looked_up {
            let map = map.to_ascii_lowercase();
            self.lookup_map_edits(&map);
        }
        let now = std::time::Instant::now();
        if self
            .map_edits
            .polled
            .is_none_or(|polled| now.duration_since(polled).as_millis() >= 500)
        {
            self.map_edits.polled = Some(now);
            self.follow_hub_map_edits();
            self.poll_map_edit_publish();
        }
        self.editor_frame();
        let enabled = self
            .console
            .as_ref()
            .is_none_or(|console| console.map_edits_on());
        let server = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(|session| session.server());
        let key = (self.map_edits.revision, server, enabled);
        if self.map_edits.applied_for != Some(key) {
            let server = server.map(|server| server.to_string());
            self.apply_map_edits(server.as_deref(), enabled);
            self.map_edits.applied_for = Some(key);
        }
        if self.map_edits.meshes_seen != self.object_meshes.len() {
            self.map_edits.meshes_seen = self.object_meshes.len();
            for placed in &mut self.map_edits.placed {
                placed.mesh = self.object_meshes.iter().position(|mesh| {
                    mesh.appearance.variant.is_empty()
                        && mesh.appearance.model.eq_ignore_ascii_case(&placed.model)
                });
            }
        }
    }

    /// Start or finish the document's lookup for `map`.
    fn lookup_map_edits(&mut self, map: &str) {
        // Another map: the editor's selection was on the last one.
        let moved = matches!(
            &self.map_edits.lookup,
            Lookup::Done { map: done, .. } | Lookup::Checksum { map: done, .. } if done != map
        );
        if moved {
            self.world_notes.cancel_selection();
            self.map_edits.panel.stale();
            self.map_edits.history.clear();
        }
        let folder = self.map_edits_folder();
        let state = &mut self.map_edits;
        match &state.lookup {
            Lookup::Done { map: done, .. } | Lookup::Checksum { map: done, .. } if done == map => {}
            _ => {
                let wanted = folder
                    .as_deref()
                    .is_some_and(|directory| has_documents(directory, map))
                    || state.editor.active
                    || state.checksum_wanted.as_deref() == Some(map);
                let Some(vfs) = self.vfs.clone().filter(|_| wanted) else {
                    state.lookup = Lookup::Done {
                        map: map.to_owned(),
                        checksum: None,
                    };
                    return;
                };
                let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                let path = format!("maps/{map}.bsp");
                let spawned = std::thread::Builder::new()
                    .name("sjk-map-edits".into())
                    .spawn(move || {
                        let checksum =
                            vfs.read(&path).ok().flatten().map(|asset| {
                                crate::assets::session_content::checksum(&asset.bytes)
                            });
                        let _ = sender.send(checksum);
                    });
                state.lookup = match spawned {
                    Ok(_) => Lookup::Checksum {
                        map: map.to_owned(),
                        receiver,
                    },
                    Err(_) => Lookup::Done {
                        map: map.to_owned(),
                        checksum: None,
                    },
                };
                return;
            }
        }
        let Lookup::Checksum { map, receiver } = &state.lookup else {
            return;
        };
        let checksum = match receiver.try_recv() {
            Ok(checksum) => checksum,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => None,
        };
        let map = map.clone();
        state.document = checksum
            .and_then(|checksum| folder.and_then(|directory| load(&directory, &map, checksum)));
        if let Some(document) = &state.document {
            crate::log::progress(format_args!(
                "map edits: {} edits for {map} (checksum {}, hub revision {})",
                document.edits.len(),
                document.checksum,
                document.revision
            ));
        }
        state.lookup = Lookup::Done { map, checksum };
        state.revision += 1;
    }

    /// Read the map's document again when the hub brought a newer copy, and log what the
    /// last look at the hub did when it changed.
    fn follow_hub_map_edits(&mut self) {
        if let Some(note) = crate::player_identity::map_edits_note(self.map_edits.hub_note.as_ref())
        {
            match &note {
                Ok(summary) => crate::log::progress(format_args!("map edits: {summary}")),
                Err(problem) => crate::log::progress(format_args!(
                    "map edits: not in step with the hub: {problem}"
                )),
            }
            self.map_edits.hub_note = Some(note);
            self.map_edits.panel.stale();
        }
        let revision = crate::player_identity::map_edits_revision();
        if revision == self.map_edits.hub_revision {
            return;
        }
        self.map_edits.hub_revision = revision;
        let Some((map, checksum)) = self.map_edits.map().map(|(m, c)| (m.to_owned(), c)) else {
            // Not looked up yet, or no checksum: look again for a document now there.
            if matches!(self.map_edits.lookup, Lookup::Done { checksum: None, .. }) {
                self.map_edits.lookup = Lookup::Idle;
            }
            return;
        };
        // Staff's unpublished work stays as it is.
        if self
            .map_edits
            .document
            .as_ref()
            .is_some_and(|document| document.unpublished)
        {
            return;
        }
        let fresh = self
            .map_edits_folder()
            .and_then(|directory| load(&directory, &map, checksum));
        if fresh != self.map_edits.document {
            // Another staff member's version: this PC's steps no longer apply to it.
            self.map_edits.document = fresh;
            self.map_edits.revision += 1;
            self.map_edits.history.clear();
        }
    }

    /// Apply the document's edits for `server` (none when `enabled` is off).
    fn apply_map_edits(&mut self, server: Option<&str>, enabled: bool) {
        let mut props = BTreeSet::new();
        let mut surfaces = BTreeSet::new();
        let mut remaps = Vec::new();
        let mut placed = Vec::new();
        let mut shown_faces = Vec::new();
        let edits = self
            .map_edits
            .document
            .as_ref()
            .filter(|_| enabled)
            .into_iter()
            .flat_map(|document| document.active(server));
        let surface_count = self.bsp.render().surfaces().len();
        let shader_of = |surface: usize| {
            let render = self.bsp.render();
            render
                .surfaces()
                .get(surface)
                .and_then(|surface| self.bsp.shaders().get(surface.shader))
                .map(|shader| shader.name_lossy().into_owned())
        };
        for edit in edits {
            match &edit.kind {
                Kind::HideProp { entity, .. } => {
                    if self.static_models.is_prop(*entity) {
                        props.insert(*entity);
                    }
                }
                Kind::HideSurface { surface, shader } => {
                    let index = *surface as usize;
                    // The checksum pins the map; the shader is a last check.
                    if index < surface_count
                        && shader_of(index).is_some_and(|name| name.eq_ignore_ascii_case(shader))
                    {
                        surfaces.insert(*surface);
                    }
                }
                Kind::Place {
                    model,
                    origin,
                    angles,
                    scale,
                } => placed.push(Placed {
                    id: edit.id,
                    model: model.clone(),
                    instance: instance(*origin, *angles, *scale),
                    origin: *origin,
                    angles: *angles,
                    scale: *scale,
                    mesh: self.object_meshes.iter().position(|mesh| {
                        mesh.appearance.variant.is_empty()
                            && mesh.appearance.model.eq_ignore_ascii_case(model)
                    }),
                }),
                Kind::Remap { from, to } => remaps.push((from.clone(), to.clone())),
                Kind::ShowFace {
                    brush,
                    side,
                    shader,
                    to,
                } => {
                    // The checksum pins the map; the side's shader is a last check.
                    if let Some((polygon, normal)) =
                        faces::polygon(&self.bsp, *brush, *side, shader)
                    {
                        shown_faces.push((
                            format!("{brush}:{side}:{to}"),
                            polygon,
                            normal,
                            to.clone(),
                        ));
                    }
                }
                // One for the map, read below; lights below; lamps when the world is
                // built (lights.rs).
                Kind::Weather { .. } | Kind::Light { .. } | Kind::Lamp { .. } => {}
            }
        }
        self.map_edits.lights = self
            .map_edits
            .document
            .as_ref()
            .filter(|_| enabled)
            .map(|document| lights::lights(document, server))
            .unwrap_or_default();
        let weather = self
            .map_edits
            .document
            .as_ref()
            .filter(|_| enabled)
            .and_then(|document| document.weather(server))
            .map(<[String]>::to_vec)
            .unwrap_or_default();
        self.weather.set_edit(&weather);
        self.map_edits.faces = shown_faces
            .into_iter()
            .filter_map(|(variant, polygon, normal, to)| {
                let origin = faces::centre(&polygon);
                let mesh = self.face_mesh(variant, &polygon, normal, origin, &to)?;
                Some((
                    mesh,
                    ActorInstance::new(origin, [0.0, 0.0, 0.0, 1.0], [1.0; 3]),
                ))
            })
            .collect();
        self.static_models.set_hidden(props.clone());
        let before = std::mem::take(&mut self.map_edits.applied.surfaces);
        for &surface in before.difference(&surfaces) {
            self.show_surface(surface as usize, true);
        }
        for &surface in surfaces.difference(&before) {
            self.show_surface(surface as usize, false);
        }
        self.map_edits.applied = Applied { props, surfaces };
        self.map_edits.placed = placed;
        // A remap preview the panel drew is gone with these.
        self.map_edits.panel.preview.dropped();
        self.map_edits.remaps.clone_from(&remaps);
        if let Some(vfs) = self.vfs.clone() {
            let refused = self.world_materials.set_edit_remaps(
                &vfs,
                &self.shaders,
                remaps.iter().map(|(a, b)| (a.as_str(), b.as_str())),
            );
            for line in refused {
                crate::log::progress(format_args!("map edit remap ignored: {line}"));
            }
        }
    }

    /// Apply `document` for no server at once (the world shots of map edits).
    #[cfg(test)]
    pub(crate) fn shot_map_edits(&mut self, document: Option<Document>) {
        self.map_edits.lookup = Lookup::Done {
            map: document
                .as_ref()
                .map_or_else(String::new, |d| d.map.clone()),
            checksum: document.as_ref().map(|d| d.checksum),
        };
        self.map_edits.document = document;
        self.apply_map_edits(None, true);
    }

    /// Draw surface `surface` as loaded, or as nothing.
    fn show_surface(&mut self, surface: usize, shown: bool) {
        let Some((range, indices)) = self.decal_surfaces.surface_indices(surface) else {
            return;
        };
        let written: Vec<u32> = if shown {
            indices.to_vec()
        } else {
            // Every corner on the surface's first vertex: no triangle has an area.
            vec![indices[0]; indices.len()]
        };
        self.queue.write_buffer(
            &self.geometry.index_buffer,
            u64::from(range.start) * 4,
            bytemuck::cast_slice(&written),
        );
        self.decal_surfaces.set_hidden(surface, !shown);
    }

    /// The mesh of a shown face (`variant` names it and its shader), built and installed
    /// the first time it is asked for: its index in `object_meshes`.
    fn face_mesh(
        &mut self,
        variant: String,
        polygon: &[[f32; 3]],
        normal: [f32; 3],
        origin: [f32; 3],
        shader: &str,
    ) -> Option<usize> {
        let appearance = sjk_runtime::Appearance {
            model: FACE_MODEL.to_owned(),
            variant,
        };
        let find = |meshes: &[crate::object_meshes::StaticModelMesh]| {
            meshes.iter().position(|mesh| mesh.appearance == appearance)
        };
        if let Some(index) = find(&self.object_meshes) {
            return Some(index);
        }
        let size = self
            .vfs
            .as_ref()
            .and_then(|vfs| texture_size(vfs, &self.shaders, shader));
        let mut scene = crate::scene_flatten::FlattenedScene::default();
        let built =
            faces::append(&mut scene, polygon, normal, origin, shader, size).and_then(|draws| {
                self.install_config_model(
                    crate::object_meshes::StaticModelMesh {
                        appearance: appearance.clone(),
                        draws,
                        center: [0.0; 3],
                        flash_bolt: None,
                        turning_bone: None,
                        eweb_part: None,
                    },
                    scene,
                )
            });
        if let Err(error) = built {
            crate::log::progress(format_args!(
                "map edit face {}: {error}",
                appearance.variant
            ));
            return None;
        }
        find(&self.object_meshes)
    }

    /// Submit the placed models for this frame.
    pub(crate) fn append_map_edit_instances(&mut self) {
        for placed in &self.map_edits.placed {
            if let Some(group) = placed
                .mesh
                .and_then(|mesh| self.object_groups.get_mut(mesh))
            {
                group.push(placed.instance);
            }
        }
        for (mesh, instance) in &self.map_edits.faces {
            if let Some(group) = self.object_groups.get_mut(*mesh) {
                group.push(*instance);
            }
        }
    }
}

/// The width and height of the image `shader` draws first, read from its header.
fn texture_size(
    vfs: &sjk_vfs::VirtualFileSystem,
    shaders: &sjk_shader::ShaderCatalog,
    shader: &str,
) -> Option<[u32; 2]> {
    let path = shaders.resolve_image(vfs, shader).ok()??;
    let asset = vfs.read(path.as_str()).ok()??;
    let format = image::ImageFormat::from_path(path.as_str()).ok()?;
    let (width, height) =
        image::ImageReader::with_format(std::io::Cursor::new(&asset.bytes[..]), format)
            .into_dimensions()
            .ok()?;
    Some([width, height])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_are_found_by_map_whatever_their_checksum() {
        let directory = std::env::temp_dir().join(format!(
            "sjk-map-edits-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        assert!(!has_documents(&directory, "mp/ffa3"));
        let mut document = Document::new("mp/ffa3", -5);
        document.add(
            Kind::Remap {
                from: "textures/a".into(),
                to: "textures/b".into(),
            },
            None,
        );
        save(&directory, &document).unwrap();
        assert!(has_documents(&directory, "mp/ffa3"));
        assert!(
            !has_documents(&directory, "mp/ffa"),
            "a map whose name starts the same has none"
        );
        assert!(!has_documents(&directory, "mp/ffa3_night"));
        assert_eq!(load(&directory, "mp/ffa3", -5), Some(document));
        assert_eq!(load(&directory, "mp/ffa3", 6), None);
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
