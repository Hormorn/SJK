//! The map editor's panel (`docs/map-edits.md`, "The panel"): the editor's commands as an
//! SJK UI column down the window's right edge, as Camera control's, over the live map. The
//! world-note key opens it on what is under the crosshair while the editor is on; a click
//! on the scene left of the column selects what is under the pointer; Escape (or Aim)
//! hides it to aim again, with a strip at the top saying how to come back.
//!
//! Three pages: Selected (hide it, remap its shader to one found by a search), Place (an
//! MD3 from the game data, then turn, lift, nudge, scale or grab the placed model) and
//! Edits (the map's edits; delete one, undo and redo); under them the scope of new edits,
//! the last command's answer (the hub's to Publish too), Publish, Aim and Editor off.
//! Every button runs the editor's own command ([`super::editor`]), so the console and the
//! panel stay one editor. Shift held makes the placed model's steps fine.
//!
//! It is a page of the in-game menu ([`crate::ingame_menu::Page::MapEdit`]), which frees
//! the pointer and keeps clicks from the game; its state lives with the map's edits.

use super::document::{Kind, Scope};
use crate::GpuState;
use crate::ingame_menu::Page;
use crate::menu_widgets::MenuCanvas;
use crate::world_notes::EditTarget;
use sjk_ui::{AbstractAction, InputEvent, PointerButton, Rect, UiEventKind, Vec2};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// The pages, as tabs.
pub(crate) const TAB_SELECTED: u16 = 1;
pub(crate) const TAB_PLACE: u16 = 2;
pub(crate) const TAB_EDITS: u16 = 3;
/// Selected: hide it (or remove a placed model), its whole compiled model, a copy of it.
pub(crate) const HIDE: u16 = 10;
pub(crate) const HIDE_GROUP: u16 = 11;
pub(crate) const COPY: u16 = 12;
/// Selected: the remap's search field (it narrows the shader list) and buttons.
pub(crate) const REMAP_FIELD: u16 = 13;
pub(crate) const REMAP: u16 = 14;
pub(crate) const UNREMAP: u16 = 15;
/// Selected, on a sky: swap every sky of the map for the chosen one.
pub(crate) const SKY_ALL: u16 = 16;
/// Place: the search, placing the chosen model, the placed model's controls.
pub(crate) const SEARCH: u16 = 20;
pub(crate) const PLACE: u16 = 21;
pub(crate) const TURN_LEFT: u16 = 22;
pub(crate) const TURN_RIGHT: u16 = 23;
pub(crate) const UP: u16 = 24;
pub(crate) const DOWN: u16 = 25;
pub(crate) const SMALLER: u16 = 26;
pub(crate) const BIGGER: u16 = 27;
pub(crate) const GRAB: u16 = 28;
pub(crate) const REMOVE: u16 = 29;
/// Place: the placed model nudged along the world axis nearest the view's left, right,
/// away and back.
pub(crate) const MOVE_LEFT: u16 = 34;
pub(crate) const MOVE_RIGHT: u16 = 35;
pub(crate) const FARTHER: u16 = 36;
pub(crate) const NEARER: u16 = 37;
/// Place, SJK's creator only: a mystery box where you aim, the nearest one removed
/// (`crate::mystery_box`).
pub(crate) const BOX_PLACE: u16 = 32;
pub(crate) const BOX_REMOVE: u16 = 33;
/// Edits: delete the chosen edit, undo the last change, redo it.
pub(crate) const DELETE: u16 = 30;
pub(crate) const UNDO: u16 = 31;
pub(crate) const REDO: u16 = 38;
/// The foot: the scope's two segments, then the closing buttons.
pub(crate) const SCOPE_ALL: u16 = 40;
pub(crate) const SCOPE_SERVER: u16 = 41;
pub(crate) const PUBLISH: u16 = 42;
pub(crate) const AIM: u16 = 43;
pub(crate) const EDITOR_OFF: u16 = 44;
/// The model list's rows on show are `MODEL_ROW..MODEL_ROW + MODEL_ROWS`, the edits'
/// `EDIT_ROW..`, the shaders' `SHADER_ROW..`.
pub(crate) const MODEL_ROW: u16 = 100;
pub(crate) const MODEL_ROWS: usize = 6;
pub(crate) const EDIT_ROW: u16 = 200;
pub(crate) const EDIT_ROWS: usize = 12;
pub(crate) const SHADER_ROW: u16 = 300;
pub(crate) const SHADER_ROWS: usize = 5;

/// Longest text the fields take.
const FIELD_MAX: usize = 96;
/// What Turn, Up and Down (and the nudges), Smaller and Bigger change by; with Shift held
/// a turn is a degree, a lift or a nudge a unit, a growth a tenth of [`GROW`]'s.
const TURN: f32 = 15.0;
const LIFT: f32 = 8.0;
const GROW: f32 = 1.1;

/// The page on show.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Tab {
    #[default]
    Selected,
    Place,
    Edits,
}

impl Tab {
    pub(crate) fn token(self) -> u16 {
        match self {
            Self::Selected => TAB_SELECTED,
            Self::Place => TAB_PLACE,
            Self::Edits => TAB_EDITS,
        }
    }
}

/// What a click or key asks the editor for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Hide,
    HideGroup,
    Copy,
    Remap,
    Unremap,
    /// Swap every sky of the map for the chosen sky shader.
    SkyAll,
    Place,
    Turn(f32),
    Lift(f32),
    /// Along the floor: to the view's right and away from it, on the nearest world axes.
    Nudge {
        right: f32,
        away: f32,
    },
    Scale(f32),
    Grab,
    Remove,
    BoxPlace,
    BoxRemove,
    /// An edit chosen in the list: a placed model becomes the one the controls move.
    Choose,
    Delete,
    Undo,
    Redo,
    Scope(Scope),
    Publish,
    Aim,
    EditorOff,
}

/// What the selection is, as the panel names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Picked {
    /// A `misc_model_static` of the map.
    Prop,
    /// A model an edit placed.
    Placed,
    /// A BSP surface; `soup` when the compiler merged a model into it (Hide whole model).
    Surface { soup: bool },
    /// An entity the game needs (a door, a breakable): remapped, never hidden.
    Entity,
    /// A caulk or nodraw brush side the map draws nothing on: an edit may draw it.
    Face,
    /// A sky brush side: an edit may cover it with a texture, or swap the sky.
    Sky,
    /// Nothing an edit changes.
    Other,
}

/// The selection as shown.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Selected {
    pub(crate) picked: Picked,
    /// Its name line: the model's path, or the surface's number.
    pub(crate) name: String,
    /// Its detail line: the shader, the class, the distance.
    pub(crate) detail: String,
    /// The shader it draws, which a remap replaces (none: it cannot be remapped).
    pub(crate) shader: Option<String>,
}

/// One edit in the Edits list.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EditRow {
    pub(crate) id: u32,
    pub(crate) text: String,
    /// It applies on this server.
    pub(crate) here: bool,
}

/// What the panel shows of the editor and the map, read again when they change.
#[derive(Clone, Debug, Default)]
pub(crate) struct Facts {
    /// The map's name, or empty while it loads.
    pub(crate) map: String,
    /// The map's checksum is known (edits can be made).
    pub(crate) ready: bool,
    pub(crate) edit_count: usize,
    pub(crate) unpublished: bool,
    pub(crate) revision: u64,
    pub(crate) selection: Option<Selected>,
    /// The current placed model's line, when there is one.
    pub(crate) placed: Option<String>,
    /// The map's edits, newest first.
    pub(crate) rows: Vec<EditRow>,
    pub(crate) scope: Scope,
    /// The server new edits for this server would name, when on one.
    pub(crate) server: Option<String>,
    pub(crate) grabbing: bool,
    /// The first key bound to the world note (the editor's select key).
    pub(crate) select_key: String,
    /// Whether there is a change to undo, and one to redo.
    pub(crate) can_undo: bool,
    pub(crate) can_redo: bool,
    /// Why the last look at the hub's map edits fell short, when it did.
    pub(crate) hub_problem: Option<String>,
}

/// The panel's state.
#[derive(Default)]
pub(crate) struct Panel {
    pub(crate) tab: Tab,
    /// The control the keyboard is on.
    pub(crate) focus: u16,
    pub(crate) remap: String,
    pub(crate) search: String,
    pub(crate) facts: Facts,
    /// Every MD3 the game data holds outside the player models, read once per game data.
    models: Vec<String>,
    /// The game data [`Self::models`] was read from (its address).
    models_from: usize,
    /// The indices of [`Self::models`] the search keeps.
    pub(crate) shown: Vec<u32>,
    pub(crate) model_row: usize,
    pub(crate) model_first: usize,
    pub(crate) edit_row: usize,
    pub(crate) edit_first: usize,
    /// Every shader a remap may draw ([`listed_shader`]): defined ones and images, read
    /// once per game data, with the game data's address.
    shaders: Vec<String>,
    shaders_from: usize,
    /// The sky shaders (their scripts have `skyParms`), lower case and sorted: on a sky
    /// they come first in the list, and a sky swaps only for one of them.
    skies: Vec<String>,
    /// The indices of [`Self::shaders`] the remap's search keeps.
    pub(crate) shader_shown: Vec<u32>,
    pub(crate) shader_row: usize,
    pub(crate) shader_first: usize,
    /// The search the panel filled in for the selection (its shader's folder), and the
    /// shader it was for: a selection of another shader fills it in again unless the
    /// search was typed since.
    prefill: String,
    prefilled_for: Option<String>,
    /// The last command's answer, and whether it was a refusal.
    pub(crate) status: String,
    pub(crate) status_error: bool,
    /// Built by the view: where the scene ends (window pixels; a press left of it selects),
    /// and the lists' areas for the wheel.
    pub(crate) scene_right: f32,
    pub(crate) model_list: Option<Rect>,
    pub(crate) edit_list: Option<Rect>,
    pub(crate) shader_list: Option<Rect>,
    /// What [`Facts`] were read for: the document's revision, the current placed model,
    /// grabbing, and whether the map's checksum was known.
    seen: Option<(u64, Option<u32>, bool, bool)>,
    /// The gizmo transforms the placed model: the keys line names its keys.
    pub(crate) transforming: bool,
}

/// Whether `path` (the game data's spelling) is a model a map edit can place: an MD3
/// outside the player models.
fn placeable(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md3") && lower.starts_with("models/") && !lower.starts_with("models/players/")
}

/// Every model a map edit can place ([`placeable`]), sorted: the Place page's list, and
/// the browser editor's.
pub(super) fn placeable_models(vfs: &sjk_vfs::VirtualFileSystem) -> Vec<String> {
    let mut models: Vec<String> = vfs
        .paths()
        .into_iter()
        .map(|path| path.as_str().to_owned())
        .filter(|path| placeable(path))
        .collect();
    models.sort();
    models.dedup();
    models
}

/// Every shader a remap may draw ([`listed_shader`]): defined ones and images, sorted.
/// The Selected page's list, and the browser editor's.
pub(super) fn remap_shaders(
    vfs: &sjk_vfs::VirtualFileSystem,
    catalog: &sjk_shader::ShaderCatalog,
) -> Vec<String> {
    let mut shaders: Vec<String> = catalog
        .names()
        .filter(|name| listed_shader(name))
        .map(shader_stem)
        .chain(
            vfs.paths()
                .into_iter()
                .filter_map(|path| texture_shader(path.as_str())),
        )
        .collect();
    shaders.sort();
    shaders.dedup();
    shaders
}

/// Whether `path` matches every word of `query` (lower case).
/// The sky shaders of `catalog` (their scripts have `skyParms`), as named there: what a
/// sky swaps for.
pub(crate) fn sky_shaders(catalog: &sjk_shader::ShaderCatalog) -> Vec<String> {
    let mut skies: Vec<String> = catalog
        .names()
        .filter(|name| !name.contains(':'))
        .filter(|name| catalog.get(name).is_some_and(|d| d.sky.is_some()))
        .map(str::to_owned)
        .collect();
    skies.sort();
    skies.dedup();
    skies
}

fn matches(path: &str, query: &str) -> bool {
    let path = path.to_ascii_lowercase();
    query.split_whitespace().all(|word| path.contains(word))
}

/// Whether a remap's list offers shader `name` (lower case): the world's and the map
/// objects' shaders, never the player models' nor the compiler's own (`name:q3map`).
fn listed_shader(name: &str) -> bool {
    (name.starts_with("textures/")
        || (name.starts_with("models/") && !name.starts_with("models/players/")))
        && !name.contains(':')
}

/// A shader name without its image extension (`.jpg`, `.tga` or `.png`, any case).
pub(super) fn without_image_extension(name: &str) -> &str {
    let cut = name.len().saturating_sub(4);
    match name.get(cut..) {
        Some(extension)
            if [".jpg", ".tga", ".png"]
                .iter()
                .any(|image| extension.eq_ignore_ascii_case(image)) =>
        {
            &name[..cut]
        }
        _ => name,
    }
}

/// A shader name as the list holds it: lower case, without its image extension.
fn shader_stem(name: &str) -> String {
    without_image_extension(name).to_ascii_lowercase()
}

/// The shader name an image of the game data stands for, when a remap may draw it: a
/// listed path without its extension.
fn texture_shader(path: &str) -> Option<String> {
    let stem = without_image_extension(path);
    if stem.len() == path.len() {
        return None;
    }
    let stem = stem.to_ascii_lowercase();
    listed_shader(&stem).then_some(stem)
}

/// The folder part of a shader's name: `textures/a/floor` is `textures/a/`.
fn folder_of(shader: &str) -> &str {
    shader.rfind('/').map_or("", |slash| &shader[..=slash])
}

impl Panel {
    /// Lay the panel out in the SJK UI's look.
    pub(crate) fn build(&mut self, canvas: &mut MenuCanvas, viewport: [f32; 2]) {
        super::panel_view::build(self, canvas, viewport);
    }

    /// The models to place: read from `vfs` the first time the Place page needs them.
    fn read_models(&mut self, vfs: Option<&sjk_vfs::VirtualFileSystem>) {
        let Some(vfs) = vfs else {
            return;
        };
        let from = std::ptr::from_ref(vfs) as usize;
        if self.models_from == from {
            return;
        }
        self.models_from = from;
        self.models = placeable_models(vfs);
        self.refilter();
    }

    /// The shaders a remap may draw ([`listed_shader`]): read from `vfs` and `catalog` the
    /// first time the Selected page needs them.
    fn read_shaders(
        &mut self,
        vfs: Option<&sjk_vfs::VirtualFileSystem>,
        catalog: &sjk_shader::ShaderCatalog,
    ) {
        let Some(vfs) = vfs else {
            return;
        };
        let from = std::ptr::from_ref(vfs) as usize;
        if self.shaders_from == from {
            return;
        }
        self.shaders_from = from;
        self.shaders = remap_shaders(vfs, catalog);
        self.skies = sky_shaders(catalog)
            .into_iter()
            .map(|sky| sky.to_ascii_lowercase())
            .collect();
        self.skies.sort();
        self.skies.dedup();
        // A sky outside textures/ (a pack's own folder) is still offered on a sky.
        for sky in &self.skies {
            if let Err(at) = self.shaders.binary_search(sky) {
                self.shaders.insert(at, sky.clone());
            }
        }
        self.refilter_shaders();
    }

    /// Whether `shader` (any case) is a sky shader.
    pub(crate) fn is_sky(&self, shader: &str) -> bool {
        self.skies.binary_search(&shader_stem(shader)).is_ok()
    }

    /// Whether the chosen shader is a sky: on a sky, the remap swaps the sky.
    pub(crate) fn chosen_is_sky(&self) -> bool {
        self.chosen_shader()
            .is_some_and(|shader| self.is_sky(shader))
    }

    #[cfg(test)]
    pub(crate) fn set_models(&mut self, models: &[&str]) {
        self.models = models.iter().map(|model| (*model).to_owned()).collect();
        self.refilter();
    }

    #[cfg(test)]
    pub(crate) fn set_shaders(&mut self, shaders: &[&str]) {
        self.shaders = shaders.iter().map(|shader| (*shader).to_owned()).collect();
        self.skies = self
            .shaders
            .iter()
            .filter(|shader| shader.contains("skies/"))
            .cloned()
            .collect();
        self.skies.sort();
        self.refilter_shaders();
    }

    /// Narrow the shaders to the remap's search; the chosen row is the one named exactly,
    /// else the selection's own shader, else the first.
    pub(crate) fn refilter_shaders(&mut self) {
        let query = self.remap.trim().to_ascii_lowercase();
        self.shader_shown.clear();
        self.shader_shown.extend(
            self.shaders
                .iter()
                .enumerate()
                .filter(|(_, shader)| matches(shader, &query))
                .map(|(index, _)| index as u32),
        );
        let on_sky = self
            .facts
            .selection
            .as_ref()
            .is_some_and(|selection| selection.picked == Picked::Sky);
        if on_sky {
            // The skies first: what a sky is most often swapped for.
            let skies = &self.skies;
            let shaders = &self.shaders;
            self.shader_shown
                .sort_by_key(|&index| skies.binary_search(&shaders[index as usize]).is_err());
        }
        let own = self
            .facts
            .selection
            .as_ref()
            .and_then(|selection| selection.shader.as_deref())
            .map(shader_stem);
        let named = |name: &str| {
            self.shader_shown
                .iter()
                .position(|&index| self.shaders[index as usize] == name)
        };
        self.shader_row = named(&query)
            .or_else(|| own.as_deref().and_then(named))
            .unwrap_or(0);
        self.shader_first = 0;
        Self::settle(
            &mut self.shader_row,
            &mut self.shader_first,
            self.shader_shown.len(),
            SHADER_ROWS,
        );
    }

    /// Fill the remap's search with the selected shader's folder, so its neighbours show,
    /// unless the search was typed since the last fill.
    fn prefill_remap(&mut self) {
        let shader = self
            .facts
            .selection
            .as_ref()
            .and_then(|selection| selection.shader.clone());
        if shader == self.prefilled_for {
            return;
        }
        if self.remap.is_empty() || self.remap == self.prefill {
            let picked = self
                .facts
                .selection
                .as_ref()
                .map(|selection| selection.picked);
            // A face draws a wall or floor texture, not another tool shader; on a sky the
            // skies come first in the whole list.
            self.prefill = if picked == Some(Picked::Sky) {
                String::new()
            } else if picked == Some(Picked::Face) {
                "textures/".to_owned()
            } else {
                shader
                    .as_deref()
                    .map(|shader| folder_of(&shader.to_ascii_lowercase()).to_owned())
                    .unwrap_or_default()
            };
            self.remap = self.prefill.clone();
        }
        self.prefilled_for = shader;
        self.refilter_shaders();
    }

    /// The shader chosen in the remap's list.
    pub(crate) fn chosen_shader(&self) -> Option<&str> {
        self.shown_shader(self.shader_row)
    }

    /// The shader list's row `index` on show, for the view.
    pub(crate) fn shown_shader(&self, index: usize) -> Option<&str> {
        self.shader_shown
            .get(index)
            .and_then(|&index| self.shaders.get(index as usize))
            .map(String::as_str)
    }

    /// What a remap would draw: the shader chosen in the list, else the search as typed
    /// (a shader the list does not know).
    pub(crate) fn remap_target(&self) -> Option<String> {
        self.chosen_shader().map(str::to_owned).or_else(|| {
            let typed = self.remap.trim();
            (!typed.is_empty() && !typed.ends_with('/')).then(|| typed.to_owned())
        })
    }

    /// Whether there is a [`Panel::remap_target`], without making it (each frame).
    pub(crate) fn has_remap_target(&self) -> bool {
        let typed = self.remap.trim();
        self.chosen_shader().is_some() || (!typed.is_empty() && !typed.ends_with('/'))
    }

    fn refilter(&mut self) {
        let query = self.search.to_ascii_lowercase();
        self.shown.clear();
        self.shown.extend(
            self.models
                .iter()
                .enumerate()
                .filter(|(_, model)| matches(model, &query))
                .map(|(index, _)| index as u32),
        );
        self.model_row = 0;
        self.model_first = 0;
    }

    /// The model chosen in the list.
    pub(crate) fn chosen_model(&self) -> Option<&str> {
        self.shown
            .get(self.model_row)
            .and_then(|&index| self.models.get(index as usize))
            .map(String::as_str)
    }

    /// The model list's row `index` on show, for the view.
    pub(crate) fn shown_model(&self, index: usize) -> Option<&str> {
        self.shown
            .get(index)
            .and_then(|&index| self.models.get(index as usize))
            .map(String::as_str)
    }

    /// The edit chosen in the list.
    pub(crate) fn chosen_edit(&self) -> Option<u32> {
        self.facts.rows.get(self.edit_row).map(|row| row.id)
    }

    pub(super) fn show(&mut self, tab: Tab) {
        self.tab = tab;
        self.focus = tab.token();
    }

    /// Keep `row` of `count` rows on show in a list of `rows` from `first`.
    fn settle(row: &mut usize, first: &mut usize, count: usize, rows: usize) {
        *row = (*row).min(count.saturating_sub(1));
        if *row < *first {
            *first = *row;
        } else if *row >= *first + rows {
            *first = *row + 1 - rows;
        }
        *first = (*first).min(count.saturating_sub(rows));
    }

    /// Move the chosen row of the list the keyboard is on by `by`; whether it moved. In a
    /// search the keyboard stays there, choosing in the list under it.
    fn move_in_list(&mut self, by: isize) -> bool {
        let typing = matches!(self.focus, SEARCH | REMAP_FIELD);
        let (row, first, count, rows, base) =
            if self.is_model_row(self.focus) || self.focus == SEARCH {
                (
                    &mut self.model_row,
                    &mut self.model_first,
                    self.shown.len(),
                    MODEL_ROWS,
                    MODEL_ROW,
                )
            } else if self.is_edit_row(self.focus) {
                (
                    &mut self.edit_row,
                    &mut self.edit_first,
                    self.facts.rows.len(),
                    EDIT_ROWS,
                    EDIT_ROW,
                )
            } else if self.is_shader_row(self.focus) || self.focus == REMAP_FIELD {
                (
                    &mut self.shader_row,
                    &mut self.shader_first,
                    self.shader_shown.len(),
                    SHADER_ROWS,
                    SHADER_ROW,
                )
            } else {
                return false;
            };
        let wanted = row.saturating_add_signed(by);
        if count == 0 || wanted >= count || (by < 0 && *row == 0) {
            return typing;
        }
        *row = wanted;
        Self::settle(row, first, count, rows);
        if !typing {
            self.focus = base + (*row - *first) as u16;
        }
        true
    }

    fn is_model_row(&self, token: u16) -> bool {
        (MODEL_ROW..MODEL_ROW + MODEL_ROWS as u16).contains(&token)
    }

    fn is_edit_row(&self, token: u16) -> bool {
        (EDIT_ROW..EDIT_ROW + EDIT_ROWS as u16).contains(&token)
    }

    fn is_shader_row(&self, token: u16) -> bool {
        (SHADER_ROW..SHADER_ROW + SHADER_ROWS as u16).contains(&token)
    }

    /// Up, Down or Tab: along a list, else the next control in the drawn order.
    pub(crate) fn step(&mut self, canvas: &mut MenuCanvas, forward: bool) {
        self.step_by(canvas, forward, true);
    }

    /// [`Panel::step`]; `along` false (Tab in a search) leaves at once.
    fn step_by(&mut self, canvas: &mut MenuCanvas, forward: bool, along: bool) {
        if along && self.move_in_list(if forward { 1 } else { -1 }) {
            return;
        }
        let direction = if forward {
            AbstractAction::Next
        } else {
            AbstractAction::Previous
        };
        if let Some(token) = canvas.action(direction) {
            self.focus = token;
            // Into a list at its chosen row.
            if self.is_model_row(token) {
                self.focus = MODEL_ROW + (self.model_row.saturating_sub(self.model_first)) as u16;
            } else if self.is_edit_row(token) {
                self.focus = EDIT_ROW + (self.edit_row.saturating_sub(self.edit_first)) as u16;
            } else if self.is_shader_row(token) {
                self.focus =
                    SHADER_ROW + (self.shader_row.saturating_sub(self.shader_first)) as u16;
            }
        }
    }

    /// The wheel over a list scrolls it; whether it was over one.
    fn scroll(&mut self, position: Vec2, delta: Vec2) -> bool {
        let inside = |rect: Option<Rect>| {
            rect.is_some_and(|rect| {
                position.x >= rect.x
                    && position.x <= rect.right()
                    && position.y >= rect.y
                    && position.y <= rect.bottom()
            })
        };
        let lines = if delta.y > 0.0 { -3 } else { 3 };
        let (first, count, rows) = if inside(self.model_list) {
            (&mut self.model_first, self.shown.len(), MODEL_ROWS)
        } else if inside(self.edit_list) {
            (&mut self.edit_first, self.facts.rows.len(), EDIT_ROWS)
        } else if inside(self.shader_list) {
            (&mut self.shader_first, self.shader_shown.len(), SHADER_ROWS)
        } else {
            return false;
        };
        *first = first
            .saturating_add_signed(lines)
            .min(count.saturating_sub(rows));
        true
    }

    /// A pointer event on the column.
    pub(crate) fn pointer(&mut self, canvas: &mut MenuCanvas, event: InputEvent) -> Option<Action> {
        if let InputEvent::PointerWheel { position, delta } = event
            && self.scroll(position, delta)
        {
            return None;
        }
        let event = canvas.pointer(event)?;
        let token = event.token?;
        if matches!(event.kind, UiEventKind::Hover | UiEventKind::HoverEnter) {
            // Hovering a list's row does not choose it; a click does.
            if !self.is_model_row(token) && !self.is_edit_row(token) && !self.is_shader_row(token) {
                self.focus = token;
            }
            return None;
        }
        if event.kind != UiEventKind::Activate {
            return None;
        }
        self.focus = token;
        self.activate(token)
    }

    /// Enter, Space or a click on `token`.
    pub(crate) fn activate(&mut self, token: u16) -> Option<Action> {
        match token {
            TAB_SELECTED => self.show(Tab::Selected),
            TAB_PLACE => self.show(Tab::Place),
            TAB_EDITS => self.show(Tab::Edits),
            HIDE => return Some(Action::Hide),
            HIDE_GROUP => return Some(Action::HideGroup),
            COPY => return Some(Action::Copy),
            REMAP | REMAP_FIELD => return Some(Action::Remap),
            UNREMAP => return Some(Action::Unremap),
            SKY_ALL => return Some(Action::SkyAll),
            PLACE | SEARCH => return Some(Action::Place),
            TURN_LEFT => return Some(Action::Turn(TURN)),
            TURN_RIGHT => return Some(Action::Turn(-TURN)),
            UP => return Some(Action::Lift(LIFT)),
            DOWN => return Some(Action::Lift(-LIFT)),
            MOVE_LEFT => {
                return Some(Action::Nudge {
                    right: -LIFT,
                    away: 0.0,
                });
            }
            MOVE_RIGHT => {
                return Some(Action::Nudge {
                    right: LIFT,
                    away: 0.0,
                });
            }
            FARTHER => {
                return Some(Action::Nudge {
                    right: 0.0,
                    away: LIFT,
                });
            }
            NEARER => {
                return Some(Action::Nudge {
                    right: 0.0,
                    away: -LIFT,
                });
            }
            SMALLER => return Some(Action::Scale(1.0 / GROW)),
            BIGGER => return Some(Action::Scale(GROW)),
            GRAB => return Some(Action::Grab),
            REMOVE => return Some(Action::Remove),
            BOX_PLACE => return Some(Action::BoxPlace),
            BOX_REMOVE => return Some(Action::BoxRemove),
            DELETE => return Some(Action::Delete),
            UNDO => return Some(Action::Undo),
            REDO => return Some(Action::Redo),
            SCOPE_ALL => return Some(Action::Scope(Scope::AllServers)),
            SCOPE_SERVER => return Some(Action::Scope(Scope::ThisServer)),
            PUBLISH => return Some(Action::Publish),
            AIM => return Some(Action::Aim),
            EDITOR_OFF => return Some(Action::EditorOff),
            row if self.is_model_row(row) => {
                let index = self.model_first + usize::from(row - MODEL_ROW);
                if index < self.shown.len() {
                    // A click on the chosen row places it.
                    if index == self.model_row {
                        return Some(Action::Place);
                    }
                    self.model_row = index;
                }
            }
            row if self.is_edit_row(row) => {
                let index = self.edit_first + usize::from(row - EDIT_ROW);
                if index < self.facts.rows.len() {
                    self.edit_row = index;
                    return Some(Action::Choose);
                }
            }
            row if self.is_shader_row(row) => {
                let index = self.shader_first + usize::from(row - SHADER_ROW);
                if index < self.shader_shown.len() {
                    // A click on the chosen row remaps to it.
                    if index == self.shader_row {
                        return Some(Action::Remap);
                    }
                    self.shader_row = index;
                }
            }
            _ => {}
        }
        None
    }

    /// The field the keyboard types in, if it is on one.
    fn field(&mut self) -> Option<&mut String> {
        match self.focus {
            REMAP_FIELD => Some(&mut self.remap),
            SEARCH => Some(&mut self.search),
            _ => None,
        }
    }

    /// A key while the panel is open (the caller has the modifiers): an action, or none.
    pub(crate) fn key(
        &mut self,
        canvas: &mut MenuCanvas,
        event: &KeyEvent,
        shift: bool,
        control: bool,
    ) -> Option<Action> {
        if event.state != ElementState::Pressed {
            return None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return None;
        };
        match key {
            KeyCode::Escape if !event.repeat => return Some(Action::Aim),
            KeyCode::ArrowUp => self.step(canvas, false),
            KeyCode::ArrowDown => self.step(canvas, true),
            KeyCode::Tab if !event.repeat => {
                let typing = matches!(self.focus, SEARCH | REMAP_FIELD);
                self.step_by(canvas, !shift, !typing);
            }
            KeyCode::PageUp => {
                let _ = self.move_in_list(-(MODEL_ROWS as isize));
            }
            KeyCode::PageDown => {
                let _ = self.move_in_list(MODEL_ROWS as isize);
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight if !event.repeat => {
                let right = key == KeyCode::ArrowRight;
                match self.focus {
                    TAB_SELECTED | TAB_PLACE | TAB_EDITS => {
                        let tabs = [Tab::Selected, Tab::Place, Tab::Edits];
                        let at = tabs.iter().position(|tab| *tab == self.tab).unwrap_or(0);
                        let next = if right { (at + 1) % 3 } else { (at + 2) % 3 };
                        self.show(tabs[next]);
                    }
                    SCOPE_ALL | SCOPE_SERVER => {
                        self.focus = if right { SCOPE_SERVER } else { SCOPE_ALL };
                        return Some(Action::Scope(if right {
                            Scope::ThisServer
                        } else {
                            Scope::AllServers
                        }));
                    }
                    _ => {}
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter if !event.repeat => {
                return self.activate(self.focus);
            }
            KeyCode::KeyZ if control && shift && !event.repeat => return Some(Action::Redo),
            KeyCode::KeyZ if control && !event.repeat => return Some(Action::Undo),
            KeyCode::KeyY if control && !event.repeat => return Some(Action::Redo),
            KeyCode::Delete if self.is_edit_row(self.focus) && !event.repeat => {
                return Some(Action::Delete);
            }
            _ => {
                let search = self.focus == SEARCH;
                if let Some(field) = self.field() {
                    let text = event.text.as_deref();
                    if crate::text_entry::edit(field, key, text, shift, control, FIELD_MAX, true) {
                        if search {
                            self.refilter();
                        } else {
                            self.refilter_shaders();
                        }
                    }
                } else if key == KeyCode::Space && !event.repeat {
                    return self.activate(self.focus);
                }
            }
        }
        None
    }

    /// Show what a command answered: its last line, or why it was refused.
    pub(super) fn answer(&mut self, result: Result<Vec<String>, String>) {
        match result {
            Ok(lines) => {
                self.status = lines.last().cloned().unwrap_or_default();
                self.status_error = false;
            }
            Err(error) => {
                self.status = error;
                self.status_error = true;
            }
        }
    }

    /// Read the facts again on the next frame.
    pub(super) fn stale(&mut self) {
        self.seen = None;
    }
}

impl GpuState {
    /// Whether the editor's panel is on show.
    pub(crate) fn map_edit_panel_open(&self) -> bool {
        self.game_menu && self.game_menu_page == Page::MapEdit
    }

    /// Open the panel over the live map, the pointer freed for it.
    pub(crate) fn open_map_edit_panel(&mut self) {
        if !self.map_edits.editor.active {
            return;
        }
        let panel = &mut self.map_edits.panel;
        panel.stale();
        if panel.focus == 0 {
            panel.focus = panel.tab.token();
        }
        self.gameplay_input.release_keys();
        self.game_menu = true;
        self.game_menu_page = Page::MapEdit;
        self.release_pointer();
    }

    /// Hide the panel to aim again; the editor stays on.
    pub(crate) fn close_map_edit_panel(&mut self) {
        if self.map_edit_panel_open() {
            self.game_menu = false;
            self.game_menu_page = Page::Main;
            self.capture_pointer();
        }
    }

    /// A key while the panel is open: all of them are its own. False when it is not open.
    pub(crate) fn map_edit_key(&mut self, event: &KeyEvent) -> bool {
        if !self.map_edit_panel_open() {
            return false;
        }
        // The gizmo's keys (1, 2, 3, L, G, R, T; all of them while it transforms).
        if self.map_edit_gizmo_key(event) {
            return true;
        }
        let (shift, control) = self.console.as_ref().map_or((false, false), |console| {
            (console.shift_held(), console.control_held())
        });
        if let Some(action) =
            self.map_edits
                .panel
                .key(self.in_game_menu.canvas_mut(), event, shift, control)
        {
            self.apply_map_edit_action(action);
        }
        true
    }

    /// A pointer event while the panel is open: the column's, or a press on the scene,
    /// which selects what is under the pointer.
    pub(crate) fn map_edit_pointer(&mut self, event: InputEvent) {
        // A press on the gizmo's handles, and its drags, are the gizmo's.
        if self.map_edit_gizmo_pointer(event) {
            return;
        }
        if let InputEvent::PointerPress {
            position,
            button: PointerButton::Primary,
        } = event
            && position.x < self.map_edits.panel.scene_right
        {
            let picked = self.map_edit_pick_at([position.x, position.y]);
            self.map_edit_picked(picked, "Nothing there to edit");
            return;
        }
        if let Some(action) = self
            .map_edits
            .panel
            .pointer(self.in_game_menu.canvas_mut(), event)
        {
            self.apply_map_edit_action(action);
        }
    }

    /// Run what `action` asks with the editor's commands, and show their answer.
    fn apply_map_edit_action(&mut self, action: Action) {
        let text = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
        let fine = self
            .console
            .as_ref()
            .is_some_and(|console| console.shift_held());
        let action = match action {
            Action::Turn(degrees) if fine => Action::Turn(degrees.signum()),
            Action::Lift(units) if fine => Action::Lift(units.signum()),
            Action::Nudge { right, away } if fine => Action::Nudge {
                right: right.signum() * f32::from(right != 0.0),
                away: away.signum() * f32::from(away != 0.0),
            },
            Action::Scale(factor) if fine => Action::Scale(factor.powf(0.1)),
            action => action,
        };
        let (name, args): (&str, Vec<String>) = match action {
            Action::Aim => {
                self.close_map_edit_panel();
                return;
            }
            Action::BoxPlace | Action::BoxRemove => {
                let (name, args) = match action {
                    Action::BoxPlace => match self.map_edits.editor.scope() {
                        Scope::ThisServer => ("mysterybox_place", text(&["server"])),
                        Scope::AllServers => ("mysterybox_place", vec![]),
                    },
                    _ => ("mysterybox_remove", vec![]),
                };
                let result = self.mystery_box_command(name, &args);
                self.map_edits.panel.answer(result);
                return;
            }
            Action::Hide => ("mapedit_hide", vec![]),
            Action::HideGroup => ("mapedit_hide", text(&["group"])),
            Action::Copy => ("mapedit_place", vec![]),
            Action::Remap => {
                let panel = &mut self.map_edits.panel;
                let Some(shader) = panel.remap_target() else {
                    panel.answer(Err("Search for the shader to draw instead first".into()));
                    panel.focus = REMAP_FIELD;
                    return;
                };
                let own = panel
                    .facts
                    .selection
                    .as_ref()
                    .and_then(|selection| selection.shader.as_deref());
                if own.is_some_and(|own| shader_stem(own) == shader_stem(&shader)) {
                    panel.answer(Err("It draws that shader already: choose another".into()));
                    return;
                }
                ("mapedit_remap", vec![shader])
            }
            Action::Unremap => ("mapedit_remap", text(&["-"])),
            Action::SkyAll => {
                let panel = &mut self.map_edits.panel;
                let Some(sky) = panel.chosen_shader().filter(|_| panel.chosen_is_sky()) else {
                    panel.answer(Err("Choose a sky in the list first".into()));
                    return;
                };
                ("mapedit_sky", vec![sky.to_owned(), "all".to_owned()])
            }
            Action::Place => {
                let panel = &mut self.map_edits.panel;
                let Some(model) = panel.chosen_model().map(str::to_owned) else {
                    panel.answer(Err("Choose a model to place first".into()));
                    return;
                };
                ("mapedit_place", vec![model])
            }
            Action::Turn(degrees) => ("mapedit_turn", vec![degrees.to_string()]),
            Action::Lift(units) => ("mapedit_move", vec![units.to_string()]),
            Action::Nudge { right, away } => {
                let Some(offset) = self.map_edit_nudge(right, away) else {
                    self.map_edits
                        .panel
                        .answer(Err("No view to nudge along".into()));
                    return;
                };
                ("mapedit_move", offset.iter().map(f32::to_string).collect())
            }
            Action::Scale(factor) => ("mapedit_scale", vec![factor.to_string()]),
            Action::Grab => ("mapedit_grab", vec![]),
            Action::Remove => match self.map_edits.editor.current() {
                Some(id) => ("mapedit_delete", vec![id.to_string()]),
                None => {
                    self.map_edits
                        .panel
                        .answer(Err("No placed model: place one first".into()));
                    return;
                }
            },
            Action::Choose => {
                let Some(id) = self.map_edits.panel.chosen_edit() else {
                    return;
                };
                let placed = self.map_edits.document.as_ref().and_then(|document| {
                    document
                        .edits
                        .iter()
                        .find(|edit| edit.id == id && matches!(edit.kind, Kind::Place { .. }))
                        .map(super::document::Edit::label)
                });
                if let Some(label) = placed {
                    self.map_edits.editor.set_current(id);
                    let panel = &mut self.map_edits.panel;
                    panel.answer(Ok(vec![format!("{label}: Place's controls move it now")]));
                    panel.stale();
                }
                return;
            }
            Action::Delete => match self.map_edits.panel.chosen_edit() {
                Some(id) => ("mapedit_delete", vec![id.to_string()]),
                None => {
                    self.map_edits
                        .panel
                        .answer(Err("Choose an edit in the list first".into()));
                    return;
                }
            },
            Action::Undo => ("mapedit_undo", vec![]),
            Action::Redo => ("mapedit_redo", vec![]),
            Action::Scope(Scope::AllServers) => ("mapedit_scope", text(&["all"])),
            Action::Scope(Scope::ThisServer) => ("mapedit_scope", text(&["server"])),
            Action::Publish => ("mapedit_publish", vec![]),
            Action::EditorOff => ("mapedit", text(&["off"])),
        };
        let result = self.map_edit_command(name, &args);
        let ok = result.is_ok();
        let panel = &mut self.map_edits.panel;
        panel.answer(result);
        panel.stale();
        match action {
            Action::Remap if ok => panel.remap.clear(),
            // A grabbed model follows the aim: back to aiming.
            Action::Grab if ok && self.map_edits.editor.grabbing() => self.close_map_edit_panel(),
            Action::EditorOff if ok => self.close_map_edit_panel(),
            _ => {}
        }
    }

    /// The strip over play while the editor is on and its panel hidden; whether it is
    /// drawn this frame.
    pub(crate) fn append_map_edit_strip(&mut self, viewport: [f32; 2]) -> bool {
        if !self.map_edits.editor.active || self.map_edit_panel_open() {
            return false;
        }
        self.refresh_map_edit_panel();
        let state = &mut self.map_edits;
        super::panel_view::build_strip(&state.panel.facts, &mut state.strip.0, viewport);
        crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        )
        .append(&self.map_edits.strip.0, viewport);
        true
    }

    /// The world offset of a nudge `right` of the view and `away` from it, along the
    /// world axes nearest those directions (a map's walls mostly follow them), so steps
    /// add up straight.
    fn map_edit_nudge(&self, right: f32, away: f32) -> Option<[f32; 3]> {
        let (_, forward) = self.world_notes.view()?;
        // The view's heading on the floor, snapped to an axis.
        let ahead = if forward.x.abs() >= forward.y.abs() {
            [forward.x.signum(), 0.0]
        } else {
            [0.0, forward.y.signum()]
        };
        // Quake axes: x forward, y left, z up; right of (x, y) is (y, -x).
        let to_right = [ahead[1], -ahead[0]];
        Some([
            to_right[0] * right + ahead[0] * away,
            to_right[1] * right + ahead[1] * away,
            0.0,
        ])
    }

    /// A selection was made (`picked`), or nothing was there (`nothing` says so): placing,
    /// it chooses where the model goes; else the Selected page shows it.
    pub(super) fn map_edit_picked(&mut self, picked: bool, nothing: &str) {
        let panel = &mut self.map_edits.panel;
        if !picked {
            panel.answer(Err(nothing.to_owned()));
        } else if panel.tab == Tab::Place {
            let line = match panel.chosen_model() {
                Some(model) => format!(
                    "Spot chosen: Place puts {} there",
                    super::editor::short_name(model)
                ),
                None => "Spot chosen: choose a model, then Place".to_owned(),
            };
            panel.answer(Ok(vec![line]));
        } else {
            panel.show(Tab::Selected);
        }
        panel.stale();
    }

    /// Select what is under `point` (physical pixels) for the editor; whether something
    /// was.
    fn map_edit_pick_at(&mut self, point: [f32; 2]) -> bool {
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let picked = self.world_notes.select_for_edit_at(
            crate::world_notes::Scene {
                bsp: &self.bsp,
                surfaces: &self.decal_surfaces,
                scratch: &mut self.trace_scratch,
                vfs: self.vfs.as_deref(),
                map: &map,
                remaps: Vec::new(),
                markers: Vec::new(),
                entities: self.map_edits.pick_entities(&self.bsp),
            },
            point,
        );
        picked.is_some()
    }

    /// Once a frame while the panel is open: read the facts again when the edits, the
    /// placed model or the grab changed (allocation-free otherwise).
    pub(crate) fn refresh_map_edit_panel(&mut self) {
        let editor = &self.map_edits.editor;
        let key = (
            self.map_edits.revision,
            editor.current(),
            editor.grabbing(),
            self.map_edits.map().is_some(),
        );
        if self.map_edits.panel.seen == Some(key) {
            return;
        }
        self.map_edits.panel.seen = Some(key);
        let selection = self.map_edit_selected();
        let server = self.map_edits_server();
        let select_key = self.console.as_ref().map_or_else(String::new, |console| {
            console
                .keys_for_command("worldnote")
                .first()
                .map(|key| {
                    let mut name = String::new();
                    sjk_shell::key_names::push_display_key(&mut name, key);
                    name
                })
                .unwrap_or_default()
        });
        let state = &self.map_edits;
        let document = state.document.as_ref();
        let current = state.editor.current();
        let placed = current.and_then(|id| {
            document
                .and_then(|document| document.edits.iter().find(|edit| edit.id == id))
                .filter(|edit| matches!(edit.kind, Kind::Place { .. }))
                .map(super::document::Edit::describe)
        });
        let rows = document.map_or_else(Vec::new, |document| {
            document
                .edits
                .iter()
                .rev()
                .map(|edit| EditRow {
                    id: edit.id,
                    text: format!("{}: {}", edit.id, edit.label()),
                    here: edit.applies_on(server.as_deref()),
                })
                .collect()
        });
        let (can_undo, can_redo) = state.history.can();
        let hub_problem = match &state.hub_note {
            Some(Err(problem)) => Some(problem.clone()),
            _ => None,
        };
        let facts = Facts {
            map: state
                .map()
                .map(|(map, _)| map.to_owned())
                .or_else(|| document.map(|document| document.map.clone()))
                .unwrap_or_default(),
            ready: state.map().is_some(),
            edit_count: document.map_or(0, |document| document.edits.len()),
            unpublished: document.is_some_and(|document| document.unpublished),
            revision: document.map_or(0, |document| document.revision),
            selection,
            placed,
            rows,
            scope: state.editor.scope(),
            server,
            grabbing: state.editor.grabbing(),
            select_key,
            can_undo,
            can_redo,
            hub_problem,
        };
        let vfs = self.vfs.clone();
        let panel = &mut self.map_edits.panel;
        panel.facts = facts;
        Panel::settle(
            &mut panel.edit_row,
            &mut panel.edit_first,
            panel.facts.rows.len(),
            EDIT_ROWS,
        );
        panel.read_models(vfs.as_deref());
        if panel
            .facts
            .selection
            .as_ref()
            .is_some_and(|s| s.shader.is_some())
        {
            panel.read_shaders(vfs.as_deref(), &self.shaders);
        }
        panel.prefill_remap();
    }

    /// The selection, as the panel shows it.
    fn map_edit_selected(&self) -> Option<Selected> {
        let selection = self.world_notes.waiting_selection()?;
        let summary = selection.summary();
        let lump =
            sjk_entity::parse_entity_lump(self.bsp.entities()).map_or(0, |entities| entities.len());
        Some(match selection.edit_target() {
            EditTarget::Model {
                number,
                classname,
                model,
                shader,
            } => {
                let picked = if self.map_edits.placed_edit(number, lump).is_some() {
                    Picked::Placed
                } else if self.static_models.is_prop(number as u32) {
                    Picked::Prop
                } else {
                    Picked::Entity
                };
                let class = if picked == Picked::Placed {
                    "placed by an edit"
                } else {
                    classname.as_str()
                };
                Selected {
                    picked,
                    name: model,
                    detail: match &shader {
                        Some(shader) => format!("{shader}, {class}"),
                        None => class.to_owned(),
                    },
                    shader,
                }
            }
            EditTarget::Surface {
                index,
                shader,
                kind,
            } => Selected {
                picked: Picked::Surface {
                    soup: self
                        .bsp
                        .render()
                        .surfaces()
                        .get(index)
                        .is_some_and(|surface| surface.kind == sjk_bsp::SurfaceKind::TriangleSoup),
                },
                name: shader.clone(),
                detail: format!("surface {index}, {kind}"),
                shader: Some(shader),
            },
            EditTarget::Face {
                brush,
                side,
                shader,
                sky,
            } => {
                let server = self.map_edits_server();
                let drawn = self.map_edits.document.as_ref().and_then(|document| {
                    document
                        .active(server.as_deref())
                        .find_map(|edit| match &edit.kind {
                            Kind::ShowFace {
                                brush: b,
                                side: s,
                                to,
                                ..
                            } if *b == brush && *s == side => Some(to.clone()),
                            _ => None,
                        })
                });
                if sky {
                    let swapped = self.map_edits.document.as_ref().and_then(|document| {
                        document
                            .active(server.as_deref())
                            .find_map(|edit| match &edit.kind {
                                Kind::Remap { from, to } if from.eq_ignore_ascii_case(&shader) => {
                                    Some(to.clone())
                                }
                                _ => None,
                            })
                    });
                    let sky = match swapped {
                        Some(to) => format!("swapped for {to}"),
                        None => "the map's sky".to_owned(),
                    };
                    return Some(Selected {
                        picked: Picked::Sky,
                        name: shader.clone(),
                        detail: match drawn {
                            Some(to) => {
                                format!("brush {brush} side {side} covered with {to}; {sky}")
                            }
                            None => format!("brush {brush} side {side}, {sky}"),
                        },
                        shader: Some(shader),
                    });
                }
                Selected {
                    picked: Picked::Face,
                    name: shader.clone(),
                    detail: match drawn {
                        Some(to) => format!("brush {brush} side {side}, drawn as {to}"),
                        None => format!("brush {brush} side {side}, drawn by no surface"),
                    },
                    shader: Some(shader),
                }
            }
            EditTarget::Other => Selected {
                picked: Picked::Other,
                name: "Not something edits change".to_owned(),
                detail: summary,
                shader: None,
            },
        })
    }
}

#[cfg(test)]
impl GpuState {
    /// The world shots: the editor on (no staff check), what is under the crosshair
    /// selected, or under `point` (physical pixels) when given, and the panel open on `tab`.
    pub(crate) fn shot_map_edit_panel(&mut self, tab: Tab, point: Option<[f32; 2]>) {
        self.map_edits.editor.active = true;
        self.world_notes.hold_selection(true);
        match point {
            Some(point) => {
                self.map_edit_pick_at(point);
            }
            None => {
                let _ = self.map_edit_pick();
            }
        }
        self.open_map_edit_panel();
        self.map_edits.panel.show(tab);
        self.map_edits.panel.stale();
    }

    /// The world shots: type `query` in the Place page's search.
    pub(crate) fn shot_map_edit_search(&mut self, query: &str) {
        let panel = &mut self.map_edits.panel;
        panel.search = query.to_owned();
        panel.refilter();
        panel.focus = SEARCH;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_md3s_outside_the_player_models_are_placed() {
        assert!(placeable("models/map_objects/yavin/tree01.md3"));
        assert!(placeable("Models/Items/Shield.MD3"));
        assert!(!placeable("models/players/kyle/model.glm"));
        assert!(!placeable("models/players/kyle/head.md3"));
        assert!(!placeable("textures/yavin/tree.md3"));
        assert!(!placeable("models/map_objects/tree.glm"));
    }

    #[test]
    fn the_search_keeps_paths_with_every_word() {
        let mut panel = Panel::default();
        panel.set_models(&[
            "models/map_objects/yavin/tree01.md3",
            "models/map_objects/yavin/rock.md3",
            "models/map_objects/korriban/tree.md3",
        ]);
        assert_eq!(panel.shown.len(), 3);
        panel.search = "Yavin TREE".to_owned();
        panel.refilter();
        assert_eq!(
            panel.chosen_model(),
            Some("models/map_objects/yavin/tree01.md3")
        );
        assert_eq!(panel.shown.len(), 1);
        panel.search = "nothing".to_owned();
        panel.refilter();
        assert_eq!(panel.chosen_model(), None);
    }

    #[test]
    fn a_list_keeps_its_chosen_row_on_show() {
        let (mut row, mut first) = (11, 0);
        Panel::settle(&mut row, &mut first, 20, 8);
        assert_eq!((row, first), (11, 4));
        Panel::settle(&mut row, &mut first, 5, 8);
        assert_eq!((row, first), (4, 0));
        let (mut row, mut first) = (0, 0);
        Panel::settle(&mut row, &mut first, 0, 8);
        assert_eq!((row, first), (0, 0));
    }

    #[test]
    fn the_shader_search_keeps_the_selections_neighbours_and_chooses_its_own() {
        let mut panel = Panel::default();
        panel.facts.selection = Some(Selected {
            picked: Picked::Surface { soup: false },
            name: "textures/yavin/floor".to_owned(),
            detail: String::new(),
            shader: Some("textures/yavin/Floor".to_owned()),
        });
        panel.set_shaders(&[
            "textures/korriban/wall",
            "textures/yavin/floor",
            "textures/yavin/floor2",
            "textures/yavin/stone",
        ]);
        panel.prefill_remap();
        assert_eq!(panel.remap, "textures/yavin/");
        assert_eq!(panel.shader_shown.len(), 3);
        assert_eq!(
            panel.chosen_shader(),
            Some("textures/yavin/floor"),
            "its own shader is chosen first"
        );
        // Typed since: another selection leaves the search alone.
        panel.remap = "stone".to_owned();
        panel.refilter_shaders();
        panel.facts.selection.as_mut().unwrap().shader = Some("textures/korriban/wall".into());
        panel.prefill_remap();
        assert_eq!(panel.remap, "stone");
        assert_eq!(
            panel.remap_target().as_deref(),
            Some("textures/yavin/stone")
        );
        // A name the list does not know is taken as typed; a bare folder is not.
        panel.remap = "textures/other/thing".to_owned();
        panel.refilter_shaders();
        assert_eq!(
            panel.remap_target().as_deref(),
            Some("textures/other/thing")
        );
        panel.remap = "textures/other/".to_owned();
        panel.refilter_shaders();
        assert_eq!(panel.remap_target(), None);
    }

    #[test]
    fn in_a_search_up_and_down_choose_and_the_keyboard_stays() {
        let mut panel = Panel::default();
        panel.set_models(&["models/a.md3", "models/b.md3", "models/c.md3"]);
        panel.focus = SEARCH;
        assert!(panel.move_in_list(1));
        assert_eq!(
            (panel.focus, panel.chosen_model()),
            (SEARCH, Some("models/b.md3"))
        );
        assert!(panel.move_in_list(5), "at the end it stays in the search");
        assert_eq!(panel.chosen_model(), Some("models/b.md3"));
        assert!(panel.move_in_list(-1));
        assert_eq!(panel.chosen_model(), Some("models/a.md3"));
        // On a row, the keyboard follows the row.
        panel.focus = MODEL_ROW;
        assert!(panel.move_in_list(1));
        assert_eq!(panel.focus, MODEL_ROW + 1);
    }

    #[test]
    fn textures_stand_for_shaders_by_their_path() {
        assert_eq!(
            texture_shader("textures/Yavin/Floor.JPG").as_deref(),
            Some("textures/yavin/floor")
        );
        assert_eq!(texture_shader("textures/yavin/floor.md3"), None);
        assert_eq!(texture_shader("gfx/2d/crosshaira.tga"), None);
        assert_eq!(
            texture_shader("models/map_objects/yavin/fern3_b.tga").as_deref(),
            Some("models/map_objects/yavin/fern3_b")
        );
        assert_eq!(texture_shader("models/players/kyle/body.jpg"), None);
        assert!(!listed_shader("textures/yavin/floor:q3map"));
        assert_eq!(shader_stem("models/a/B.TGA"), "models/a/b");
        assert_eq!(folder_of("textures/a/floor"), "textures/a/");
        assert_eq!(folder_of("floor"), "");
    }

    #[test]
    fn buttons_ask_for_their_command() {
        let mut panel = Panel::default();
        assert_eq!(panel.activate(HIDE), Some(Action::Hide));
        assert_eq!(panel.activate(TURN_RIGHT), Some(Action::Turn(-TURN)));
        assert_eq!(
            panel.activate(SCOPE_SERVER),
            Some(Action::Scope(Scope::ThisServer))
        );
        assert_eq!(panel.activate(TAB_EDITS), None);
        assert_eq!(panel.tab, Tab::Edits);
        assert_eq!(
            panel.activate(FARTHER),
            Some(Action::Nudge {
                right: 0.0,
                away: LIFT
            })
        );
        assert_eq!(panel.activate(REDO), Some(Action::Redo));
        // A model row chooses, then a second click places.
        panel.set_models(&["models/a.md3", "models/b.md3"]);
        assert_eq!(panel.activate(MODEL_ROW + 1), None);
        assert_eq!(panel.chosen_model(), Some("models/b.md3"));
        assert_eq!(panel.activate(MODEL_ROW + 1), Some(Action::Place));
    }
}
