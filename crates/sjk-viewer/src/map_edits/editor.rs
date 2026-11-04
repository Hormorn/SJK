//! The in-game map editor, for staff: console commands that act on what the world-note
//! key selected (or, without a selection, on what is under the crosshair), saved to the
//! map's document at once and sent to the hub with `mapedit_publish`.
//!
//! While `mapedit` is on, the world-note key only selects (its outline shows what an
//! edit will touch); it opens no note. Every command that changes the map's edits is one
//! step of [`History`], which `mapedit_undo` and `mapedit_redo` walk.

use super::document::{Edit, Kind, Scope};
use crate::GpuState;
use crate::world_notes::{EditTarget, Scene};
use glam::Vec3;

/// How many surfaces `mapedit_hide group` takes at most.
const GROUP_LIMIT: usize = 256;
/// Degrees `mapedit_turn` turns by without a number, and a notch of the wheel turns a
/// grabbed model by.
const TURN_STEP: f32 = 15.0;
/// Steps [`History`] keeps each way.
const HISTORY_STEPS: usize = 100;
/// The commands that may change the map's edits: each one that does is a step to undo.
const CHANGES: &[&str] = &[
    "mapedit_hide",
    "mapedit_place",
    "mapedit_turn",
    "mapedit_move",
    "mapedit_scale",
    "mapedit_grab",
    "mapedit_remap",
    "mapedit_sky",
    "mapedit_delete",
    super::gizmo::COMMAND,
];

/// The editor's steps on one map: the edits as they were before each change, to undo,
/// and as they were before each undo, to redo. Cleared when the map changes or the hub
/// brings another staff member's version.
#[derive(Default)]
pub(crate) struct History {
    undo: Vec<Vec<Edit>>,
    redo: Vec<Vec<Edit>>,
}

impl History {
    /// A change made the edits differ from `before`: a new step, and nothing to redo.
    fn changed(&mut self, before: Vec<Edit>) {
        if self.undo.len() == HISTORY_STEPS {
            self.undo.remove(0);
        }
        self.undo.push(before);
        self.redo.clear();
    }

    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Whether there is a step to undo, and one to redo.
    pub(crate) fn can(&self) -> (bool, bool) {
        (!self.undo.is_empty(), !self.redo.is_empty())
    }
}

/// The edit a step changes between `from` and `to`, as `to` has it when it is there: what
/// an undo or a redo names.
fn stepped<'a>(from: &'a [Edit], to: &'a [Edit]) -> Option<&'a Edit> {
    to.iter()
        .find(|edit| !from.contains(edit))
        .or_else(|| from.iter().find(|edit| !to.contains(edit)))
}

/// A path's last part: `models/a/door.md3` is `door.md3`.
pub(crate) fn short_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The editor's state in one world.
pub(crate) struct Editor {
    pub(crate) active: bool,
    /// Where new edits apply.
    scope: Scope,
    /// The placed model `mapedit_turn`, `_move`, `_scale` and `_grab` act on.
    current: Option<u32>,
    /// `mapedit_grab`: the current placed model follows the crosshair.
    grabbing: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            active: false,
            scope: Scope::AllServers,
            current: None,
            grabbing: false,
        }
    }
}

impl Editor {
    /// The placed model the panel's controls act on.
    pub(crate) fn current(&self) -> Option<u32> {
        self.current
    }

    /// Whether the current placed model follows the aim.
    pub(crate) fn grabbing(&self) -> bool {
        self.grabbing
    }

    /// Where new edits apply.
    pub(crate) fn scope(&self) -> Scope {
        self.scope
    }

    /// Make placed edit `id` the one the placed model's controls act on.
    pub(crate) fn set_current(&mut self, id: u32) {
        if self.current != Some(id) {
            self.current = Some(id);
            self.grabbing = false;
        }
    }
}

/// The console commands, with their help.
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    ("mapedit", "Staff: turn the map editor on or off"),
    (
        "mapedit_panel",
        "Map editor: open its panel (the world-note key does, on what you aim at)",
    ),
    (
        "mapedit_web",
        "Map editor: edit this map in your web browser (creator only for now; a page served on this PC only)",
    ),
    (
        "mapedit_hide",
        "Map editor: hide the selected prop or surface ('group' takes the whole compiled model), or remove a placed model",
    ),
    (
        "mapedit_place",
        "Map editor: place a model (a path, or a copy of the selected one) where you aim",
    ),
    (
        "mapedit_turn",
        "Map editor: turn the current placed model (degrees, 15 by default)",
    ),
    (
        "mapedit_move",
        "Map editor: move the current placed model (x y z, or one number up)",
    ),
    (
        "mapedit_scale",
        "Map editor: scale the current placed model by a factor",
    ),
    (
        "mapedit_grab",
        "Map editor: the current placed model follows your aim until mapedit_grab again",
    ),
    (super::gizmo::COMMAND, super::gizmo::HELP),
    (
        "mapedit_remap",
        "Map editor: draw a shader instead of the selected one ('-' removes the remap)",
    ),
    (
        "mapedit_sky",
        "Map editor: swap the selected sky for another sky shader ('all': every sky of this map; '-' puts the map's back)",
    ),
    (
        "mapedit_scope",
        "Map editor: new edits for all servers or only this server",
    ),
    (
        "mapedit_undo",
        "Map editor: undo the last change to this map's edits",
    ),
    (
        "mapedit_redo",
        "Map editor: redo the change mapedit_undo took back",
    ),
    ("mapedit_list", "Map editor: list this map's edits"),
    ("mapedit_delete", "Map editor: remove the edit of an id"),
    (
        "mapedit_publish",
        "Map editor: send this map's edits to the hub for every SJK player",
    ),
];

/// Whether `name` (lower case) is a map editor command.
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|(command, _)| *command == name)
}

/// What an edit is aimed at.
struct Aim {
    target: EditTarget,
    hit: Vec3,
    /// The view's forward when the selection was made: a placed model faces it.
    forward: Vec3,
    summary: String,
}

fn yaw_of(forward: Vec3) -> f32 {
    forward.y.atan2(forward.x).to_degrees()
}

fn number(text: &str) -> Result<f32, String> {
    text.parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("not a number: {text}"))
}

impl GpuState {
    /// A `mapedit*` command.
    pub(crate) fn map_edit_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if !crate::player_identity::is_staff() {
            return Err("The map editor is for SJK staff".into());
        }
        if name == "mapedit" {
            return self.map_edit_toggle(args);
        }
        if name == "mapedit_web" {
            return self.map_edit_web();
        }
        if name == "mapedit_panel" {
            if !self.map_edits.editor.active {
                self.map_edit_toggle(&[])?;
            }
            self.open_map_edit_panel();
            return Ok(Vec::new());
        }
        if !self.map_edits.editor.active {
            return Err("Turn the map editor on first: mapedit".into());
        }
        let before = CHANGES.contains(&name).then(|| self.map_edit_edits());
        let result = self.map_edit_change(name, args);
        if let Some(before) = before
            && result.is_ok()
            && before != self.map_edit_edits()
        {
            self.map_edits.history.changed(before);
        }
        if result.is_ok() {
            // The shaders and models used lately, for the browser editor's Recent rows.
            self.map_edit_web_remember(name, args);
        }
        result
    }

    /// This map's edits as they stand (none without a document).
    fn map_edit_edits(&self) -> Vec<Edit> {
        self.map_edits
            .document
            .as_ref()
            .map_or_else(Vec::new, |document| document.edits.clone())
    }

    /// A command of the editor while it is on.
    fn map_edit_change(&mut self, name: &str, args: &[String]) -> Result<Vec<String>, String> {
        match name {
            "mapedit_hide" => self.map_edit_hide(args),
            "mapedit_place" => self.map_edit_place(args),
            "mapedit_turn" => {
                let degrees = match args {
                    [] => TURN_STEP,
                    [degrees] => number(degrees)?,
                    _ => return Err("usage: mapedit_turn [degrees]".into()),
                };
                self.map_edit_transform(|_, angles, _| angles[1] = (angles[1] + degrees) % 360.0)
            }
            "mapedit_move" => {
                let offset = match args {
                    [up] => [0.0, 0.0, number(up)?],
                    [x, y, z] => [number(x)?, number(y)?, number(z)?],
                    _ => return Err("usage: mapedit_move <up> | <x> <y> <z>".into()),
                };
                self.map_edit_transform(|origin, _, _| {
                    for axis in 0..3 {
                        origin[axis] += offset[axis];
                    }
                })
            }
            "mapedit_scale" => {
                let [factor] = args else {
                    return Err("usage: mapedit_scale <factor>".into());
                };
                let factor = number(factor)?;
                if factor <= 0.0 {
                    return Err("the factor must be above 0".into());
                }
                self.map_edit_transform(|_, _, scale| {
                    for axis in scale.iter_mut() {
                        *axis = (*axis * factor).clamp(0.01, 64.0);
                    }
                })
            }
            "mapedit_grab" => self.map_edit_grab(),
            super::gizmo::COMMAND => {
                // A gizmo's drag or a G, R or T transform: every model at once, one step.
                let sets = super::gizmo::parse_set(args)?;
                let line = super::gizmo::set_placed(self.map_edit_document()?, &sets)?;
                if let [set] = sets.as_slice() {
                    self.map_edits.editor.current = Some(set.id);
                }
                self.map_edit_saved(line)
            }
            "mapedit_remap" => self.map_edit_remap(args),
            "mapedit_sky" => self.map_edit_sky(args),
            "mapedit_scope" => self.map_edit_scope(args),
            "mapedit_undo" => self.map_edit_step(true),
            "mapedit_redo" => self.map_edit_step(false),
            "mapedit_delete" => {
                let [id] = args else {
                    return Err("usage: mapedit_delete <id> (mapedit_list shows them)".into());
                };
                let id = id.parse().map_err(|_| format!("not an id: {id}"))?;
                self.map_edit_delete(id)
            }
            "mapedit_list" => Ok(self.map_edit_list()),
            "mapedit_publish" => self.map_edit_publish(),
            _ => Err(format!("unknown map editor command {name}")),
        }
    }

    fn map_edit_toggle(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let active = match args {
            [] => !self.map_edits.editor.active,
            [value] if matches!(value.as_str(), "1" | "on") => true,
            [value] if matches!(value.as_str(), "0" | "off") => false,
            _ => return Err("usage: mapedit [on|off]".into()),
        };
        let editor = &mut self.map_edits.editor;
        editor.active = active;
        editor.grabbing = false;
        self.world_notes.hold_selection(active);
        if !active {
            self.world_notes.cancel_selection();
            self.close_map_edit_panel();
            // The browser editor stops with the editor.
            self.map_edits.web = None;
            return Ok(vec!["Map editor off".into()]);
        }
        // Start from the hub's latest copies.
        crate::player_identity::refresh_map_edits();
        // A map without a document has no checksum read yet: read it now.
        if self.map_edits.map().is_none() {
            self.map_edits.lookup = super::Lookup::Idle;
        }
        // In a world, its panel opens on what is under the crosshair.
        if self.live_session.is_some() || self.resident.exploring() {
            self.map_edit_pick();
            self.open_map_edit_panel();
        }
        Ok(vec![
            format!(
                "Map editor on ({}). The world-note key selects and opens its panel; or:",
                self.map_edit_scope_text()
            ),
            "  mapedit_hide [group], mapedit_place [model], mapedit_remap <shader|->".into(),
            "  mapedit_turn [deg], mapedit_move <up|x y z>, mapedit_scale <f>, mapedit_grab".into(),
            "  mapedit_scope [all|server], mapedit_list, mapedit_delete <id>".into(),
            "  mapedit_undo, mapedit_redo step through this map's changes".into(),
            "  mapedit_publish sends the map's edits to every SJK player".into(),
        ])
    }

    fn map_edit_scope_text(&self) -> String {
        match (self.map_edits.editor.scope, self.map_edits_server()) {
            (Scope::AllServers, _) => "new edits for all servers".into(),
            (Scope::ThisServer, Some(server)) => format!("new edits for {server} only"),
            (Scope::ThisServer, None) => "new edits for this server, but not on one".into(),
        }
    }

    fn map_edit_scope(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let editor = &mut self.map_edits.editor;
        editor.scope = match args {
            [] => match editor.scope {
                Scope::AllServers => Scope::ThisServer,
                Scope::ThisServer => Scope::AllServers,
            },
            [value] if value == "all" => Scope::AllServers,
            [value] if value == "server" => Scope::ThisServer,
            _ => return Err("usage: mapedit_scope [all|server]".into()),
        };
        Ok(vec![self.map_edit_scope_text()])
    }

    /// The server new edits are for, by the scope.
    fn map_edit_server(&self) -> Result<Option<String>, String> {
        match self.map_edits.editor.scope {
            Scope::AllServers => Ok(None),
            Scope::ThisServer => self
                .map_edits_server()
                .map(Some)
                .ok_or_else(|| "Not on a server: mapedit_scope all".into()),
        }
    }

    /// The world-note key while the editor is on: a grabbed model is put down where it is
    /// drawn; else what is under the crosshair is selected. Either way the panel opens.
    pub(crate) fn map_edit_select(&mut self) {
        if self.map_edits.editor.grabbing {
            let result = self.map_edit_command("mapedit_grab", &[]);
            self.map_edits.panel.answer(result);
        } else {
            let picked = self.map_edit_pick().is_some();
            self.map_edit_picked(picked, "Nothing under the crosshair");
        }
        self.open_map_edit_panel();
    }

    pub(super) fn map_edit_pick(&mut self) -> Option<String> {
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let remaps = Vec::new();
        self.world_notes.select_for_edit(Scene {
            bsp: &self.bsp,
            surfaces: &self.decal_surfaces,
            scratch: &mut self.trace_scratch,
            vfs: self.vfs.as_deref(),
            map: &map,
            remaps,
            markers: Vec::new(),
            entities: self.map_edits.pick_entities(&self.bsp),
        })
    }

    /// The selection the world-note key made, or what is under the crosshair now.
    fn map_edit_aim(&mut self) -> Result<Aim, String> {
        if self.world_notes.waiting_selection().is_none() {
            self.map_edit_pick();
        }
        let selection = self
            .world_notes
            .waiting_selection()
            .ok_or("Nothing under the crosshair")?;
        let (hit, _) = selection.aim();
        Ok(Aim {
            target: selection.edit_target(),
            hit,
            forward: selection.forward(),
            summary: selection.summary(),
        })
    }

    /// The loaded map's document, begun when it has none.
    fn map_edit_document(&mut self) -> Result<&mut super::Document, String> {
        let (map, checksum) = self
            .map_edits
            .map()
            .map(|(map, checksum)| (map.to_owned(), checksum))
            .ok_or("The map's checksum is still being read: try again in a moment")?;
        Ok(self
            .map_edits
            .document
            .get_or_insert_with(|| super::Document::new(&map, checksum)))
    }

    /// Save the document after an edit and answer `line`.
    fn map_edit_saved(&mut self, line: String) -> Result<Vec<String>, String> {
        let folder = self.map_edits_folder();
        self.map_edits.changed(folder.as_deref())?;
        Ok(vec![line])
    }

    fn lump_len(&self) -> usize {
        sjk_entity::parse_entity_lump(self.bsp.entities()).map_or(0, |entities| entities.len())
    }

    fn map_edit_hide(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let group = match args {
            [] => false,
            [word] if word == "group" => true,
            _ => return Err("usage: mapedit_hide [group]".into()),
        };
        let aim = self.map_edit_aim()?;
        let server = self.map_edit_server()?;
        self.world_notes.cancel_selection();
        match aim.target {
            EditTarget::Model { number, model, .. } => {
                if let Some(id) = self.map_edits.placed_edit(number, self.lump_len()) {
                    return self.map_edit_delete(id);
                }
                if !self.static_models.is_prop(number as u32) {
                    return Err(format!(
                        "{model} is a server entity: the game needs it, it is not hidden"
                    ));
                }
                let id = self.map_edit_document()?.add(
                    Kind::HideProp {
                        entity: number as u32,
                        model: model.clone(),
                    },
                    server,
                );
                self.map_edit_saved(format!("Edit {id}: {model} hidden"))
            }
            EditTarget::Surface { index, shader, .. } => {
                let surfaces = if group {
                    self.map_edit_hide_group(index)
                } else {
                    vec![index]
                };
                let shader_of = |surface: usize| {
                    self.bsp
                        .render()
                        .surfaces()
                        .get(surface)
                        .and_then(|surface| self.bsp.shaders().get(surface.shader))
                        .map_or_else(|| shader.clone(), |s| s.name_lossy().into_owned())
                };
                let named: Vec<(u32, String)> = surfaces
                    .iter()
                    .map(|&surface| (surface as u32, shader_of(surface)))
                    .collect();
                let document = self.map_edit_document()?;
                let ids: Vec<u32> = named
                    .into_iter()
                    .map(|(surface, shader)| {
                        document.add(Kind::HideSurface { surface, shader }, server.clone())
                    })
                    .collect();
                self.map_edit_saved(match ids.as_slice() {
                    [id] => format!("Edit {id}: surface {index} ({shader}) hidden"),
                    _ => format!(
                        "Edits {}-{}: {} surfaces hidden from surface {index}",
                        ids.iter().min().unwrap_or(&0),
                        ids.iter().max().unwrap_or(&0),
                        ids.len()
                    ),
                })
            }
            EditTarget::Face {
                brush,
                side,
                shader,
                ..
            } => self.map_edit_unshow_face(brush, side, &shader, server),
            EditTarget::Other => Err(format!("{}: not something edits hide", aim.summary)),
        }
    }

    /// The surfaces `mapedit_hide group` takes from surface `index`: those of the model
    /// the map compiler merged into the world that it is part of.
    pub(super) fn map_edit_hide_group(&self, index: usize) -> Vec<usize> {
        let render = self.bsp.render();
        let soup = |other: usize| {
            render
                .surfaces()
                .get(other)
                .is_some_and(|surface| surface.kind == sjk_bsp::SurfaceKind::TriangleSoup)
        };
        self.decal_surfaces.connected(index, soup, GROUP_LIMIT)
    }

    fn map_edit_place(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let aim = self.map_edit_aim()?;
        let server = self.map_edit_server()?;
        // A pasted copy (the browser editor's) brings its own angles and scale; else
        // the model faces the view at its own size.
        let (model, angles, scale) = match (args, &aim.target) {
            ([model], _) => (model.replace('\\', "/"), None, [1.0; 3]),
            ([model, rest @ ..], _) if rest.len() == 3 || rest.len() == 6 => {
                let numbers = rest
                    .iter()
                    .map(|text| number(text))
                    .collect::<Result<Vec<f32>, String>>()?;
                let scale = match numbers.get(3..6) {
                    Some(&[x, y, z]) => [x, y, z].map(|v| v.clamp(0.01, 64.0)),
                    _ => [1.0; 3],
                };
                (
                    model.replace('\\', "/"),
                    Some([numbers[0], numbers[1], numbers[2]]),
                    scale,
                )
            }
            ([], EditTarget::Model { model, .. }) => (model.clone(), None, [1.0; 3]),
            ([], _) => {
                return Err("usage: mapedit_place <model.md3>, or select a model to copy".into());
            }
            _ => {
                return Err("usage: mapedit_place [model.md3 [pitch yaw roll [sx sy sz]]]".into());
            }
        };
        if !model.to_ascii_lowercase().ends_with(".md3") {
            return Err(format!("{model}: only MD3 models can be placed"));
        }
        let found = self
            .vfs
            .as_ref()
            .and_then(|vfs| vfs.read(&model).ok().flatten())
            .is_some();
        if !found {
            return Err(format!("{model} is not in the game data"));
        }
        // Facing the view the selection was made from (the browser editor's when the
        // page selected).
        let yaw = yaw_of(aim.forward) + 180.0;
        let angles = angles
            .unwrap_or([0.0, yaw, 0.0])
            .map(|a| a.rem_euclid(360.0));
        self.world_notes.cancel_selection();
        let kind = Kind::Place {
            model: model.clone(),
            origin: aim.hit.to_array(),
            angles,
            scale,
        };
        let id = self.map_edit_document()?.add(kind, server);
        self.map_edits.editor.current = Some(id);
        self.map_edit_saved(format!(
            "Edit {id}: {} placed at {:.0} {:.0} {:.0} (mapedit_turn, _move, _grab move it)",
            short_name(&model),
            aim.hit.x,
            aim.hit.y,
            aim.hit.z
        ))
    }

    /// The placed model edits act on: the selected one, else the last placed.
    fn map_edit_current(&mut self) -> Result<u32, String> {
        if let Some(selection) = self.world_notes.waiting_selection()
            && let EditTarget::Model { number, .. } = selection.edit_target()
            && let Some(id) = self.map_edits.placed_edit(number, self.lump_len())
        {
            self.map_edits.editor.current = Some(id);
        }
        let id = self
            .map_edits
            .editor
            .current
            .ok_or("No placed model: mapedit_place one, or select one")?;
        let placed = self
            .map_edits
            .document
            .as_ref()
            .and_then(|document| document.edits.iter().find(|edit| edit.id == id))
            .is_some_and(|edit| matches!(edit.kind, Kind::Place { .. }));
        if !placed {
            self.map_edits.editor.current = None;
            return Err("The placed model is gone".into());
        }
        Ok(id)
    }

    fn map_edit_transform(
        &mut self,
        change: impl FnOnce(&mut [f32; 3], &mut [f32; 3], &mut [f32; 3]),
    ) -> Result<Vec<String>, String> {
        let id = self.map_edit_current()?;
        let Some(edit) = self.map_edit_document()?.get_mut(id) else {
            return Err("The placed model is gone".into());
        };
        let Kind::Place {
            origin,
            angles,
            scale,
            ..
        } = &mut edit.kind
        else {
            return Err("Not a placed model".into());
        };
        change(origin, angles, scale);
        let line = edit.describe();
        if let Some(document) = &mut self.map_edits.document {
            document.unpublished = true;
        }
        self.map_edit_saved(line)
    }

    fn map_edit_grab(&mut self) -> Result<Vec<String>, String> {
        let id = self.map_edit_current()?;
        let editor = &mut self.map_edits.editor;
        editor.grabbing = !editor.grabbing;
        if editor.grabbing {
            return Ok(vec![format!(
                "Edit {id} follows your aim: mapedit_grab again to put it down"
            )]);
        }
        // Put down where it was last drawn.
        let Some((origin, angles)) = self
            .map_edits
            .placed
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| (placed.origin, placed.angles))
        else {
            return Err("The placed model is gone".into());
        };
        self.map_edit_transform(|o, a, _| {
            *o = origin;
            *a = angles;
        })
    }

    /// Once a frame while the editor is on: a grabbed model follows the aim.
    pub(super) fn editor_frame(&mut self) {
        let editor = &self.map_edits.editor;
        if !editor.active || !editor.grabbing {
            return;
        }
        let (Some(id), Some((eye, forward))) = (editor.current, self.world_notes.view()) else {
            return;
        };
        let Some(hit) = self.decal_surfaces.ray_hit(eye, forward, 4096.0) else {
            return;
        };
        let point = eye + forward * hit.distance;
        let angles = self
            .map_edits
            .placed
            .iter()
            .find(|placed| placed.id == id)
            .map_or([0.0; 3], |placed| placed.angles);
        self.map_edits
            .preview_placement(id, point.to_array(), angles);
    }

    fn map_edit_remap(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let [target] = args else {
            return Err("usage: mapedit_remap <shader> | - (removes the remap)".into());
        };
        let aim = self.map_edit_aim()?;
        let server = self.map_edit_server()?;
        if let EditTarget::Face {
            brush,
            side,
            shader,
            sky,
        } = &aim.target
        {
            if *sky {
                // A sky shader swaps the sky; a texture covers this side; `-` takes back
                // the cover, else the swap.
                let shown = self.map_edits.document.as_ref().is_some_and(|document| {
                    document.edits.iter().any(|edit| {
                        edit.server == server
                            && matches!(edit.kind, Kind::ShowFace { brush: b, side: s, .. }
                                if b == *brush && s == *side)
                    })
                });
                if (target == "-" && !shown) || is_sky_shader(&self.shaders, target) {
                    self.world_notes.cancel_selection();
                    let skies = [shader.clone()];
                    return self.map_edit_sky_swap(&skies, target, server);
                }
            }
            return self.map_edit_show_face(*brush, *side, shader.clone(), target, server);
        }
        let from = match &aim.target {
            EditTarget::Surface { shader, .. } => shader.clone(),
            EditTarget::Model {
                shader: Some(shader),
                ..
            } => shader.clone(),
            _ => return Err(format!("{}: no shader to remap", aim.summary)),
        };
        self.world_notes.cancel_selection();
        if target == "-" {
            let document = self.map_edit_document()?;
            let ids: Vec<u32> = document
                .edits
                .iter()
                .filter(|edit| {
                    edit.server == server
                        && matches!(&edit.kind, Kind::Remap { from: f, .. } if f.eq_ignore_ascii_case(&from))
                })
                .map(|edit| edit.id)
                .collect();
            if ids.is_empty() {
                return Err(format!("No remap edit of {from} for these servers"));
            }
            for id in &ids {
                document.remove(*id);
            }
            return self.map_edit_saved(format!("{from}: remap edit removed"));
        }
        let (vfs, shaders) = (
            self.vfs.clone().ok_or("No loaded map")?,
            self.shaders.clone(),
        );
        let (from, to) =
            crate::world_materials::map_remaps::checked(&vfs, &shaders, &from, target)?;
        let id = self.map_edit_document()?.add(
            Kind::Remap {
                from: from.clone(),
                to: to.clone(),
            },
            server,
        );
        self.map_edit_saved(format!("Edit {id}: {from} drawn as {to}"))
    }

    /// Draw caulk or nodraw side `side` of brush `brush` (its shader `shader`) with
    /// shader `target`, or as nothing again with `-`.
    fn map_edit_show_face(
        &mut self,
        brush: u32,
        side: u32,
        shader: String,
        target: &str,
        server: Option<String>,
    ) -> Result<Vec<String>, String> {
        self.world_notes.cancel_selection();
        if target == "-" {
            return self.map_edit_unshow_face(brush, side, &shader, server);
        }
        let (vfs, shaders) = (
            self.vfs.clone().ok_or("No loaded map")?,
            self.shaders.clone(),
        );
        // The same checks a remap's target passes: a shader or image the game has.
        let (_, to) = crate::world_materials::map_remaps::checked(&vfs, &shaders, &shader, target)?;
        if is_sky_shader(&shaders, &to) {
            return Err(format!(
                "{to} is a sky: a face takes a texture (mapedit_sky swaps the map's sky)"
            ));
        }
        let id = self.map_edit_document()?.add(
            Kind::ShowFace {
                brush,
                side,
                shader: shader.clone(),
                to: to.clone(),
            },
            server,
        );
        self.map_edit_saved(format!(
            "Edit {id}: {shader} face (brush {brush} side {side}) drawn as {to}"
        ))
    }

    /// Remove the edit drawing side `side` of brush `brush` for `server`'s scope.
    fn map_edit_unshow_face(
        &mut self,
        brush: u32,
        side: u32,
        shader: &str,
        server: Option<String>,
    ) -> Result<Vec<String>, String> {
        let document = self.map_edit_document()?;
        let shown = document.edits.iter().find(|edit| {
            edit.server == server
                && matches!(
                    edit.kind,
                    Kind::ShowFace { brush: b, side: s, .. } if b == brush && s == side
                )
        });
        let Some(id) = shown.map(|edit| edit.id) else {
            return Err(format!(
                "{shader}: no edit draws this face for these servers"
            ));
        };
        document.remove(id);
        self.map_edit_saved(format!(
            "Edit {id} removed: the face is drawn as nothing again"
        ))
    }

    /// `mapedit_sky <shader> [all]`: draw sky shader `shader` instead of the selected
    /// sky, or of every sky of the map with `all`, as remap edits (one step to undo);
    /// `-` takes those remaps away, putting the map's sky back.
    fn map_edit_sky(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let (target, all) = match args {
            [target] => (target.as_str(), false),
            [target, all] if all == "all" => (target.as_str(), true),
            _ => {
                return Err(
                    "usage: mapedit_sky <sky shader> | - [all] (all: every sky of this map)".into(),
                );
            }
        };
        let skies = if all {
            let skies = super::faces::map_skies(&self.bsp);
            if skies.is_empty() {
                return Err("This map has no sky".into());
            }
            skies
        } else {
            let aim = self.map_edit_aim()?;
            match aim.target {
                EditTarget::Face {
                    shader, sky: true, ..
                } => vec![shader],
                _ => {
                    return Err(format!(
                        "{}: not the sky (aim at it, or mapedit_sky {target} all)",
                        aim.summary
                    ));
                }
            }
        };
        let server = self.map_edit_server()?;
        self.world_notes.cancel_selection();
        self.map_edit_sky_swap(&skies, target, server)
    }

    /// Swap each of `skies` for sky shader `target` (`-`: put them back) for `server`'s
    /// scope, after the checks a remap passes.
    fn map_edit_sky_swap(
        &mut self,
        skies: &[String],
        target: &str,
        server: Option<String>,
    ) -> Result<Vec<String>, String> {
        let to = if target == "-" {
            None
        } else {
            let vfs = self.vfs.clone().ok_or("No loaded map")?;
            let mut checked = String::new();
            for from in skies {
                checked =
                    crate::world_materials::map_remaps::checked(&vfs, &self.shaders, from, target)?
                        .1;
            }
            if !is_sky_shader(&self.shaders, &checked) {
                return Err(format!(
                    "{checked} is not a sky (no skyParms): a sky swaps only for a sky"
                ));
            }
            Some(checked)
        };
        let line = swap_skies(self.map_edit_document()?, server, skies, to.as_deref())?;
        self.map_edit_saved(line)
    }

    /// The browser editor's multiple selection: draw caulk or nodraw faces `faces`
    /// (brush, side) with `shader` and remap the shaders of surfaces `surfaces` to it
    /// (`-` takes those edits away), or, with no `shader`, hide the surfaces and draw
    /// the faces as nothing again. One step to undo, whatever the count.
    pub(crate) fn map_edit_many(
        &mut self,
        faces: &[[u32; 2]],
        surfaces: &[u32],
        shader: Option<&str>,
    ) -> Result<Vec<String>, String> {
        if !crate::player_identity::is_staff() {
            return Err("The map editor is for SJK staff".into());
        }
        if !self.map_edits.editor.active {
            return Err("Turn the map editor on first: mapedit".into());
        }
        if faces.is_empty() && surfaces.is_empty() {
            return Err("Nothing selected".into());
        }
        let before = self.map_edit_edits();
        let result = self.map_edit_many_change(faces, surfaces, shader);
        if result.is_ok() && before != self.map_edit_edits() {
            self.map_edits.history.changed(before);
        }
        if result.is_ok()
            && let Some(shader) = shader.filter(|shader| *shader != "-")
        {
            self.map_edit_web_remember("mapedit_remap", &[shader.to_owned()]);
        }
        result
    }

    fn map_edit_many_change(
        &mut self,
        faces: &[[u32; 2]],
        surfaces: &[u32],
        shader: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let server = self.map_edit_server()?;
        let faces: Vec<(u32, u32, String)> = faces
            .iter()
            .map(|&[brush, side]| {
                super::faces::world_side_shader(&self.bsp, brush, side)
                    .map(|(shader, _)| (brush, side, shader))
                    .ok_or_else(|| {
                        format!(
                            "Brush {brush} side {side} is not a caulk, nodraw or sky face of this map"
                        )
                    })
            })
            .collect::<Result<_, _>>()?;
        let mut surface_shaders: Vec<(u32, String)> = Vec::with_capacity(surfaces.len());
        for &surface in surfaces {
            let shader = self
                .bsp
                .render()
                .surfaces()
                .get(surface as usize)
                .and_then(|s| self.bsp.shaders().get(s.shader))
                .ok_or_else(|| format!("No surface {surface} on this map"))?;
            surface_shaders.push((surface, shader.name_lossy().into_owned()));
        }
        let step = match shader {
            Some("-") => Many::Remove {
                faces: &faces,
                shaders: &surface_shaders,
            },
            Some(target) => {
                let (vfs, shaders) = (
                    self.vfs.clone().ok_or("No loaded map")?,
                    self.shaders.clone(),
                );
                let checked = |from: &str| {
                    crate::world_materials::map_remaps::checked(&vfs, &shaders, from, target)
                };
                // The target is checked as a remap's is, once per shader it replaces.
                let to = match (faces.first(), surface_shaders.first()) {
                    (Some((_, _, from)), _) | (None, Some((_, from))) => checked(from)?.1,
                    (None, None) => return Err("Nothing selected".into()),
                };
                for (_, from) in &surface_shaders {
                    checked(from)?;
                }
                if !faces.is_empty() && is_sky_shader(&shaders, &to) {
                    return Err(format!(
                        "{to} is a sky: faces take a texture (Swap sky swaps the map's sky)"
                    ));
                }
                Many::Draw {
                    faces: &faces,
                    shaders: &surface_shaders,
                    to,
                }
            }
            None => {
                self.world_notes.cancel_selection();
                Many::Hide {
                    faces: &faces,
                    surfaces: &surface_shaders,
                }
            }
        };
        let line = apply_many(self.map_edit_document()?, server, step)?;
        self.map_edit_saved(line)
    }

    /// Undo the last change to this map's edits (`undo`), or redo the last undone.
    fn map_edit_step(&mut self, undo: bool) -> Result<Vec<String>, String> {
        // The map's checksum first: a step is taken only when it can be saved.
        self.map_edit_document()?;
        let history = &mut self.map_edits.history;
        let from = if undo {
            &mut history.undo
        } else {
            &mut history.redo
        };
        let Some(edits) = from.pop() else {
            return Err(if undo {
                "Nothing to undo".into()
            } else {
                "Nothing to redo".into()
            });
        };
        let document = self.map_edit_document()?;
        let now = std::mem::replace(&mut document.edits, edits);
        document.unpublished = true;
        let line = stepped(&now, &document.edits).map_or_else(
            || "the edits".to_owned(),
            |edit| match document.edits.iter().find(|kept| kept.id == edit.id) {
                Some(kept) => kept.label(),
                None => format!("{} taken away", edit.label()),
            },
        );
        let present = document
            .edits
            .iter()
            .map(|edit| edit.id)
            .collect::<Vec<_>>();
        let history = &mut self.map_edits.history;
        if undo {
            history.redo.push(now);
        } else {
            history.undo.push(now);
        }
        // The placed model the controls act on may be gone.
        let editor = &mut self.map_edits.editor;
        if editor.current.is_some_and(|id| !present.contains(&id)) {
            editor.current = None;
            editor.grabbing = false;
        }
        self.map_edit_saved(format!(
            "{}: {line}",
            if undo { "Undone" } else { "Redone" }
        ))
    }

    fn map_edit_delete(&mut self, id: u32) -> Result<Vec<String>, String> {
        let removed = self
            .map_edit_document()?
            .remove(id)
            .ok_or_else(|| format!("No edit {id}"))?;
        if self.map_edits.editor.current == Some(id) {
            self.map_edits.editor.current = None;
            self.map_edits.editor.grabbing = false;
        }
        self.map_edit_saved(format!("Removed {}", removed.describe()))
    }

    fn map_edit_list(&self) -> Vec<String> {
        let Some(document) = &self.map_edits.document else {
            return vec!["No edits on this map".into()];
        };
        let server = self.map_edits_server();
        let mut lines = vec![format!(
            "{} edits on {} (hub revision {}{}):",
            document.edits.len(),
            document.map,
            document.revision,
            if document.unpublished {
                ", changed here, not published"
            } else {
                ""
            }
        )];
        lines.extend(document.edits.iter().map(|edit| {
            let shown = if edit.applies_on(server.as_deref()) {
                ""
            } else {
                " (not on this server)"
            };
            format!("  {}{shown}", edit.describe())
        }));
        lines
    }

    fn map_edit_publish(&mut self) -> Result<Vec<String>, String> {
        let document = self
            .map_edits
            .document
            .as_ref()
            .ok_or("No edits on this map")?;
        document.check()?;
        let edits = serde_json::to_value(&document.edits).map_err(|error| error.to_string())?;
        crate::player_identity::publish_map_edits(document.map.clone(), document.checksum, edits)?;
        Ok(vec![format!(
            "Sending {} edits of {} to the hub...",
            document.edits.len(),
            document.map
        )])
    }

    /// Once a frame: the hub's answer to `mapedit_publish`.
    pub(super) fn poll_map_edit_publish(&mut self) {
        let Some(outcome) = crate::player_identity::map_edits_published() else {
            return;
        };
        let result = match outcome {
            Ok((map, checksum, revision)) => {
                let folder = self.map_edits_folder();
                if let Some(document) = self
                    .map_edits
                    .document
                    .as_mut()
                    .filter(|document| document.map == map && document.checksum == checksum)
                {
                    document.revision = revision;
                    document.unpublished = false;
                    if let Some(folder) = folder
                        && let Err(error) = super::save(&folder, document)
                    {
                        crate::log::progress(format_args!("map edits not saved: {error}"));
                    }
                }
                Ok(format!(
                    "Published: every SJK player gets {map}'s edits (hub revision {revision})"
                ))
            }
            Err(error) => Err(format!("Not published: {error}")),
        };
        let line = match &result {
            Ok(line) => line.clone(),
            Err(error) => format!("^1Map edits {}", error.replacen("Not", "not", 1)),
        };
        crate::log::progress(format_args!("map edits: {line}"));
        self.chat.receive(
            sjk_client::ServerEventKind::Print,
            line,
            None,
            std::time::Instant::now(),
        );
        let panel = &mut self.map_edits.panel;
        panel.answer(result.map(|line| vec![line]));
        panel.stale();
    }

    /// The mouse wheel while a placed model is grabbed: it turns the model (by a degree
    /// with `fine`, Shift held) instead of changing weapon. Whether it took the wheel.
    pub(crate) fn map_edit_wheel(&mut self, up: bool, fine: bool) -> bool {
        let editor = &self.map_edits.editor;
        if !editor.active || !editor.grabbing {
            return false;
        }
        let Some(id) = editor.current else {
            return false;
        };
        let Some((origin, mut angles)) = self
            .map_edits
            .placed
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| (placed.origin, placed.angles))
        else {
            return false;
        };
        let step = if fine { 1.0 } else { TURN_STEP };
        angles[1] = (angles[1] + if up { step } else { -step }).rem_euclid(360.0);
        self.map_edits.preview_placement(id, origin, angles);
        true
    }
}

/// What the browser editor's multiple selection does to a map's edits in one step: its
/// faces (brush, side, shader) and its surfaces (number, shader).
enum Many<'a> {
    /// Draw the faces with shader `to` and remap the surfaces' shaders to it.
    Draw {
        faces: &'a [(u32, u32, String)],
        shaders: &'a [(u32, String)],
        to: String,
    },
    /// Take away the edits that draw the faces and remap the surfaces' shaders.
    Remove {
        faces: &'a [(u32, u32, String)],
        shaders: &'a [(u32, String)],
    },
    /// Hide the surfaces and draw the faces as nothing again.
    Hide {
        faces: &'a [(u32, u32, String)],
        surfaces: &'a [(u32, String)],
    },
}

/// The surfaces' shaders, each once: a remap acts on every surface of its shader.
/// Whether `name` is a sky shader (its script has `skyParms`): what a sky swaps for.
pub(crate) fn is_sky_shader(shaders: &sjk_shader::ShaderCatalog, name: &str) -> bool {
    shaders
        .get(name)
        .is_some_and(|definition| definition.sky.is_some())
}

/// Swap each sky shader of `skies` for `to` in `document`'s edits for `server`, as remap
/// edits; with `to` `None`, or a sky's own name, take that sky's remap edit away (the
/// map's sky again). Its answer, or why nothing changed.
fn swap_skies(
    document: &mut super::Document,
    server: Option<String>,
    skies: &[String],
    to: Option<&str>,
) -> Result<String, String> {
    let mut swapped = Vec::new();
    let mut restored = 0;
    for from in skies {
        if let Some(to) = to.filter(|to| !to.eq_ignore_ascii_case(from)) {
            swapped.push(document.add(
                Kind::Remap {
                    from: from.clone(),
                    to: to.to_owned(),
                },
                server.clone(),
            ));
            continue;
        }
        let ids: Vec<u32> = document
            .edits
            .iter()
            .filter(|edit| {
                edit.server == server
                    && matches!(&edit.kind, Kind::Remap { from: f, .. } if f.eq_ignore_ascii_case(from))
            })
            .map(|edit| edit.id)
            .collect();
        for id in &ids {
            document.remove(*id);
        }
        restored += ids.len();
    }
    match (swapped.as_slice(), restored, to) {
        ([], 0, _) => Err("No sky swap to take back for these servers".into()),
        ([], _, _) => Ok(match skies {
            [sky] => format!("{sky}: the map's sky again"),
            _ => format!("{restored} of {} skies: the map's again", skies.len()),
        }),
        ([id], 0, Some(to)) => Ok(format!("Edit {id}: {} swapped for {to}", skies[0])),
        (ids, _, Some(to)) => Ok(format!("{} skies swapped for {to}", ids.len())),
        (_, _, None) => unreachable!("nothing is swapped without a target"),
    }
}

fn distinct_shaders(surfaces: &[(u32, String)]) -> Vec<&str> {
    let mut shaders: Vec<&str> = Vec::new();
    for (_, shader) in surfaces {
        if !shaders.iter().any(|seen| seen.eq_ignore_ascii_case(shader)) {
            shaders.push(shader);
        }
    }
    shaders
}

/// Make `step` in `document`'s edits for `server`: its answer, or why not.
fn apply_many(
    document: &mut super::Document,
    server: Option<String>,
    step: Many<'_>,
) -> Result<String, String> {
    let shows = |edit: &Edit, faces: &[(u32, u32, String)]| {
        edit.server == server
            && matches!(&edit.kind, Kind::ShowFace { brush, side, .. }
                if faces.iter().any(|(b, s, _)| b == brush && s == side))
    };
    let remove = |document: &mut super::Document, ids: &[u32]| {
        for id in ids {
            document.remove(*id);
        }
    };
    match step {
        Many::Draw { faces, shaders, to } => {
            for (brush, side, shader) in faces {
                document.add(
                    Kind::ShowFace {
                        brush: *brush,
                        side: *side,
                        shader: shader.clone(),
                        to: to.clone(),
                    },
                    server.clone(),
                );
            }
            let remapped = distinct_shaders(shaders);
            for from in &remapped {
                document.add(
                    Kind::Remap {
                        from: (*from).to_owned(),
                        to: to.clone(),
                    },
                    server.clone(),
                );
            }
            Ok(match (faces.len(), remapped.len()) {
                (f, 0) => format!("{f} faces drawn as {to}"),
                (0, r) => format!("{r} shaders remapped to {to} (every surface with them)"),
                (f, r) => format!("{f} faces drawn as {to}, {r} shaders remapped to it"),
            })
        }
        Many::Remove { faces, shaders } => {
            let remapped = distinct_shaders(shaders);
            let ids: Vec<u32> = document
                .edits
                .iter()
                .filter(|edit| {
                    shows(edit, faces)
                        || (edit.server == server
                            && matches!(&edit.kind, Kind::Remap { from, .. }
                                if remapped.iter().any(|r| r.eq_ignore_ascii_case(from))))
                })
                .map(|edit| edit.id)
                .collect();
            if ids.is_empty() {
                return Err(
                    "No edit draws these faces or remaps these shaders for these servers".into(),
                );
            }
            remove(document, &ids);
            Ok(format!("{} edits taken away", ids.len()))
        }
        Many::Hide { faces, surfaces } => {
            let shown: Vec<u32> = document
                .edits
                .iter()
                .filter(|edit| shows(edit, faces))
                .map(|edit| edit.id)
                .collect();
            if shown.is_empty() && surfaces.is_empty() {
                return Err("These faces are drawn as nothing already".into());
            }
            remove(document, &shown);
            for (surface, shader) in surfaces {
                document.add(
                    Kind::HideSurface {
                        surface: *surface,
                        shader: shader.clone(),
                    },
                    server.clone(),
                );
            }
            Ok(match (surfaces.len(), shown.len()) {
                (s, 0) => format!("{s} surfaces hidden"),
                (0, f) => format!("{f} faces drawn as nothing again"),
                (s, f) => format!("{s} surfaces hidden, {f} faces drawn as nothing again"),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faces() -> Vec<(u32, u32, String)> {
        (0..3)
            .map(|i| (10 + i, 60 + i, "textures/system/caulk".to_owned()))
            .collect()
    }

    #[test]
    fn a_fill_of_many_faces_is_one_step_to_undo() {
        let mut document = super::super::Document::new("mp/test", 7);
        document.add(
            Kind::HideSurface {
                surface: 1,
                shader: "textures/a/b".into(),
            },
            None,
        );
        let mut history = History::default();
        let before = document.edits.clone();
        let surfaces = [
            (4, "textures/a/wall".to_owned()),
            (5, "textures/a/wall".to_owned()),
            (6, "textures/a/trim".to_owned()),
        ];
        let line = apply_many(
            &mut document,
            None,
            Many::Draw {
                faces: &faces(),
                shaders: &surfaces,
                to: "textures/a/stone".into(),
            },
        )
        .unwrap();
        assert_eq!(
            line,
            "3 faces drawn as textures/a/stone, 2 shaders remapped to it"
        );
        // three faces and two remaps (the wall's once): five edits, one step
        assert_eq!(document.edits.len(), 1 + 5);
        history.changed(before.clone());
        assert_eq!(history.can(), (true, false));
        assert_eq!(history.undo.len(), 1);
        // the undo puts back the edits as they were before the fill
        assert_eq!(history.undo.pop().unwrap(), before);
        // a second fill of the same faces replaces their shader, adding nothing
        apply_many(
            &mut document,
            None,
            Many::Draw {
                faces: &faces(),
                shaders: &[],
                to: "textures/a/brick".into(),
            },
        )
        .unwrap();
        assert_eq!(document.edits.len(), 6);
    }

    #[test]
    fn a_gizmo_set_of_several_models_is_one_step_to_undo() {
        use super::super::gizmo::{parse_set, set_placed};
        let words =
            |text: String| -> Vec<String> { text.split_whitespace().map(str::to_owned).collect() };
        let mut document = super::super::Document::new("mp/test", 7);
        let place = |x: f32| Kind::Place {
            model: "models/map_objects/a.md3".into(),
            origin: [x, 0.0, 0.0],
            angles: [0.0; 3],
            scale: [1.0; 3],
        };
        let a = document.add(place(0.0), None);
        let b = document.add(place(100.0), None);
        let hidden = document.add(
            Kind::HideSurface {
                surface: 1,
                shader: "textures/a/b".into(),
            },
            None,
        );
        let mut history = History::default();
        let before = document.edits.clone();
        let sets = parse_set(&words(format!(
            "{a} 0 0 8 0 90 0 1 1 2 {b} 100 0 8 0 90 0 1 1 2"
        )))
        .unwrap();
        assert_eq!(
            set_placed(&mut document, &sets).unwrap(),
            "2 placed models set"
        );
        history.changed(before.clone());
        assert_eq!(history.undo.len(), 1, "one step for both");
        for id in [a, b] {
            let edit = document.edits.iter().find(|e| e.id == id).unwrap();
            let Kind::Place {
                origin,
                angles,
                scale,
                ..
            } = &edit.kind
            else {
                panic!("not placed")
            };
            assert_eq!(origin[2], 8.0);
            assert_eq!(angles[1], 90.0);
            assert_eq!(*scale, [1.0, 1.0, 2.0]);
        }
        assert!(document.unpublished);
        assert_eq!(history.undo.pop().unwrap(), before);
        // All or none: a hidden surface is not a placed model, so nothing moves.
        let now = document.edits.clone();
        let wrong = parse_set(&words(format!(
            "{a} 5 5 5 0 0 0 1 1 1 {hidden} 1 1 1 0 0 0 1 1 1"
        )))
        .unwrap();
        assert!(set_placed(&mut document, &wrong).is_err());
        assert_eq!(document.edits, now);
    }

    #[test]
    fn a_sky_swaps_as_remap_edits_and_comes_back() {
        let mut document = super::super::Document::new("mp/test", 7);
        let skies = ["textures/skies/a".to_owned(), "textures/skies/b".to_owned()];
        let line = swap_skies(
            &mut document,
            None,
            &skies[..1],
            Some("textures/skies/night"),
        )
        .unwrap();
        assert_eq!(
            line,
            "Edit 1: textures/skies/a swapped for textures/skies/night"
        );
        assert_eq!(
            document.edits[0].kind,
            Kind::Remap {
                from: "textures/skies/a".into(),
                to: "textures/skies/night".into(),
            }
        );
        // all skies of the map: the first's swap replaced, the second added, in one call
        // (one step of the history)
        let line = swap_skies(&mut document, None, &skies, Some("textures/skies/day")).unwrap();
        assert_eq!(line, "2 skies swapped for textures/skies/day");
        assert_eq!(document.edits.len(), 2);
        // a sky swapped for itself is its own again; for another server, nothing to take
        assert!(
            swap_skies(
                &mut document.clone(),
                Some("1.2.3.4:1".into()),
                &skies,
                None
            )
            .is_err()
        );
        let line = swap_skies(&mut document, None, &skies[1..], Some("TEXTURES/SKIES/B")).unwrap();
        assert_eq!(line, "textures/skies/b: the map's sky again");
        let line = swap_skies(&mut document, None, &skies, None).unwrap();
        assert_eq!(line, "1 of 2 skies: the map's again");
        assert!(document.edits.is_empty());
        assert!(swap_skies(&mut document, None, &skies, None).is_err());
    }

    #[test]
    fn many_faces_and_surfaces_are_hidden_or_taken_back_at_once() {
        let mut document = super::super::Document::new("mp/test", 7);
        let faces = faces();
        apply_many(
            &mut document,
            None,
            Many::Draw {
                faces: &faces,
                shaders: &[(4, "textures/a/wall".to_owned())],
                to: "textures/a/stone".into(),
            },
        )
        .unwrap();
        // Remove takes back the faces' and the remap's edits
        let mut removed = document.clone();
        let line = apply_many(
            &mut removed,
            None,
            Many::Remove {
                faces: &faces[..2],
                shaders: &[(9, "TEXTURES/A/WALL".to_owned())],
            },
        )
        .unwrap();
        assert_eq!(line, "3 edits taken away");
        assert_eq!(removed.edits.len(), 1);
        // for another server's scope, nothing is theirs to take
        assert!(
            apply_many(
                &mut document.clone(),
                Some("1.2.3.4:29070".into()),
                Many::Remove {
                    faces: &faces,
                    shaders: &[],
                },
            )
            .is_err()
        );
        // Hide: the surfaces hidden, the shown faces drawn as nothing again
        let line = apply_many(
            &mut document,
            None,
            Many::Hide {
                faces: &faces,
                surfaces: &[
                    (7, "textures/a/x".to_owned()),
                    (8, "textures/a/y".to_owned()),
                ],
            },
        )
        .unwrap();
        assert_eq!(line, "2 surfaces hidden, 3 faces drawn as nothing again");
        assert_eq!(document.edits.len(), 1 + 2);
        assert!(
            apply_many(
                &mut document,
                None,
                Many::Hide {
                    faces: &faces,
                    surfaces: &[],
                },
            )
            .is_err()
        );
    }
}
