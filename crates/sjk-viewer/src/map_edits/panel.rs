//! The map editor's panel (`docs/map-edits.md`, "The panel"): the editor's commands as an
//! SJK UI column down the window's right edge, as Camera control's, over the live map. It
//! shows whenever the editor is on: a click on the scene left of the column selects what
//! is under the pointer (Shift+click adds it to the selection, or takes it out), the right
//! mouse button held flies the camera ([`super::fly`]), Escape backs out one step at a
//! time ([`super::editor::escape_step`]) and the key bound to `mapedit` leaves, whatever
//! has the keyboard.
//!
//! Four pages: Selected (hide it, remap its shader to one found by a search; on a caulk,
//! nodraw or sky face: Same plane, Connected and Fill), Place (an MD3 from the game data,
//! then turn, lift, nudge, scale or grab the placed model), Edits (the map's edits;
//! delete one, undo and redo; the Recent rows' switch) and Weather (rain, a storm, snow,
//! fog and wind kept for the map, [`weather_choice`]); under them the scope of new edits,
//! the last command's answer (the hub's to Publish too), Publish and Editor off. The
//! shader and model lists start with the Recent rows the browser editor shares
//! ([`super::recent`]). Every button runs the editor's own command ([`super::editor`]),
//! so the console, the panel and the browser editor stay one editor. Shift held makes the
//! placed model's steps fine.
//!
//! It is a page of the in-game menu ([`crate::ingame_menu::Page::MapEdit`]), which frees
//! the pointer and keeps clicks from the game; its state lives with the map's edits.

use super::document::{Kind, Scope};
use super::linked::Grow;
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
pub(crate) const TAB_WEATHER: u16 = 4;
pub(crate) const TAB_LIGHTS: u16 = 5;
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
/// Selected, on a caulk, nodraw or sky face: the faces joined to it on its plane, at any
/// angle, and the nearby texture drawn over the selected faces.
pub(crate) const LINK_PLANE: u16 = 17;
pub(crate) const LINK_SURFACE: u16 = 18;
pub(crate) const FILL: u16 = 19;
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
/// Select every surface and face of the selection's shader, every placed model; copy
/// the selected placed models (Ctrl+D).
pub(crate) const SELECT_SHADER: u16 = 32;
pub(crate) const SELECT_PLACED: u16 = 33;
pub(crate) const DUPLICATE: u16 = 39;
pub(crate) const MOVE_LEFT: u16 = 34;
pub(crate) const MOVE_RIGHT: u16 = 35;
pub(crate) const FARTHER: u16 = 36;
pub(crate) const NEARER: u16 = 37;
/// Place: the switch between the models and the things standing on the map (the
/// boomboxes, the mystery boxes and the Toy Master's toys, as the browser editor's World
/// page): place one where the next click meets the floor, the toy's kind and escort,
/// remove the chosen one.
pub(crate) const PLACE_THINGS: u16 = 82;
/// Place: models placed and grabbed stand along the surface (`mapedit_surface`).
pub(crate) const PLACE_SURFACE: u16 = 81;
/// Selected, on surfaces: select one ring more, or one less (`mapedit_select grow`,
/// `shrink`).
pub(crate) const SELECT_MORE: u16 = 48;
pub(crate) const SELECT_LESS: u16 = 49;
pub(crate) const PLACE_MODELS: u16 = 83;
pub(crate) const THING_BOOMBOX: u16 = 84;
pub(crate) const THING_BOX: u16 = 85;
pub(crate) const THING_TOY: u16 = 86;
pub(crate) const THING_KIND: u16 = 87;
pub(crate) const THING_ESCORT: u16 = 88;
pub(crate) const THING_REMOVE: u16 = 89;
/// Edits: delete the chosen edit, undo the last change, redo it.
pub(crate) const DELETE: u16 = 30;
pub(crate) const UNDO: u16 = 31;
pub(crate) const REDO: u16 = 38;
/// The foot: the scope's two segments, then the closing buttons.
pub(crate) const SCOPE_ALL: u16 = 40;
pub(crate) const SCOPE_SERVER: u16 = 41;
pub(crate) const PUBLISH: u16 = 42;
pub(crate) const EDITOR_OFF: u16 = 44;
/// Edits: the Recent rows' switch, and forgetting them.
pub(crate) const REMEMBER: u16 = 45;
pub(crate) const FORGET: u16 = 46;
/// Weather: the four presets (as `r_weatherForce` 1 to 4), fog and wind on or off, what
/// is seen now, the map's own.
pub(crate) const WEATHER_DRIZZLE: u16 = 50;
pub(crate) const WEATHER_RAIN: u16 = 51;
pub(crate) const WEATHER_STORM: u16 = 52;
pub(crate) const WEATHER_SNOW: u16 = 53;
pub(crate) const WEATHER_FOG: u16 = 54;
pub(crate) const WEATHER_WIND: u16 = 55;
pub(crate) const WEATHER_SEEN: u16 = 56;
pub(crate) const WEATHER_OWN: u16 = 57;
/// Lights: add one where the next click lands, or at the view's centre; delete the
/// chosen one; its reach and strength; the chosen lamp off or on, and back as the map
/// has it.
pub(crate) const LIGHT_ADD: u16 = 60;
pub(crate) const LIGHT_AT_VIEW: u16 = 61;
pub(crate) const LIGHT_DELETE: u16 = 62;
pub(crate) const LIGHT_SMALLER: u16 = 63;
pub(crate) const LIGHT_BIGGER: u16 = 64;
pub(crate) const LIGHT_DIMMER: u16 = 65;
pub(crate) const LIGHT_BRIGHTER: u16 = 66;
pub(crate) const LAMP_TOGGLE: u16 = 67;
pub(crate) const LAMP_RESET: u16 = 68;
/// Lights: the colour swatches, `LIGHT_SWATCH..LIGHT_SWATCH + 8`.
pub(crate) const LIGHT_SWATCH: u16 = 70;
/// Edits: the invisible brushes' view (`r_showBrushes`), `BRUSHES..BRUSHES + 4` for its
/// presets ([`BrushPreset`]).
pub(crate) const BRUSHES: u16 = 90;

/// Selected, on a caulk, nodraw or sky face: the switch between the shader list and the
/// texture's alignment, and the alignment's buttons (`ALIGN_STEPS`, in their order).
pub(crate) const ALIGN: u16 = 600;
pub(crate) const ALIGN_STEP: u16 = 601;

/// The alignment's buttons, three rows of four: what each says and the step it takes
/// ([`super::align`]), before Shift makes it fine.
pub(crate) const ALIGN_STEPS: [(&str, super::align::Step); 12] = {
    use super::align::{SCALE_STEP, SHIFT_STEP, Step, TURN_STEP};
    [
        ("Left", Step::Shift([SHIFT_STEP, 0.0])),
        ("Right", Step::Shift([-SHIFT_STEP, 0.0])),
        ("Up", Step::Shift([0.0, SHIFT_STEP])),
        ("Down", Step::Shift([0.0, -SHIFT_STEP])),
        ("Turn left", Step::Rotate(TURN_STEP)),
        ("Turn right", Step::Rotate(-TURN_STEP)),
        ("Smaller", Step::Scale([1.0 / SCALE_STEP; 2])),
        ("Bigger", Step::Scale([SCALE_STEP; 2])),
        ("Flip across", Step::Scale([-1.0, 1.0])),
        ("Flip down", Step::Scale([1.0, -1.0])),
        ("Fit", Step::Fit),
        ("Reset", Step::Reset),
    ]
};

/// The browser editor's brush-view presets, as `r_showBrushes` and `r_showBrushesTypes`
/// set them: none, what play feels (`r_showBrushes 1`), the faces a caulk fix is about
/// (caulk, nodraw and invisible walls), every kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BrushPreset {
    Off,
    Play,
    Caulk,
    All,
}

/// The kinds the Caulk preset shows, as `r_showBrushesTypes` names them.
pub(crate) const CAULK_KINDS: &str = "caulk nodraw solid";

impl BrushPreset {
    pub(crate) const ALL: [Self; 4] = [Self::Off, Self::Play, Self::Caulk, Self::All];

    /// The cvars' values for it: `r_showBrushes`, `r_showBrushesTypes`.
    pub(crate) fn cvars(self) -> (&'static str, &'static str) {
        match self {
            Self::Off => ("0", ""),
            Self::Play => ("1", ""),
            Self::Caulk => ("2", CAULK_KINDS),
            Self::All => ("2", ""),
        }
    }

    /// The preset the cvars amount to, if one.
    pub(crate) fn of(mode: i64, types: &str) -> Option<Self> {
        let types = types.trim();
        match (mode, types) {
            (mode, _) if mode <= 0 => Some(Self::Off),
            (1, "") => Some(Self::Play),
            (_, "") => Some(Self::All),
            (_, types) => {
                let mut listed: Vec<String> = types
                    .split(|c: char| c.is_ascii_whitespace() || c == ',' || c == ';')
                    .filter(|name| !name.is_empty())
                    .map(str::to_ascii_lowercase)
                    .collect();
                listed.sort();
                let mut caulk: Vec<&str> = CAULK_KINDS.split(' ').collect();
                caulk.sort_unstable();
                listed.iter().eq(caulk.iter()).then_some(Self::Caulk)
            }
        }
    }

    /// The next one, for the B key.
    pub(crate) fn next(this: Option<Self>) -> Self {
        match this {
            Some(Self::Off) | None => Self::Play,
            Some(Self::Play) => Self::Caulk,
            Some(Self::Caulk) => Self::All,
            Some(Self::All) => Self::Off,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Play => "Play",
            Self::Caulk => "Caulk",
            Self::All => "All",
        }
    }
}
/// The model list's rows on show are `MODEL_ROW..MODEL_ROW + MODEL_ROWS`, the edits'
/// `EDIT_ROW..`, the shaders' `SHADER_ROW..`.
pub(crate) const MODEL_ROW: u16 = 100;
pub(crate) const MODEL_ROWS: usize = 6;
pub(crate) const EDIT_ROW: u16 = 200;
pub(crate) const EDIT_ROWS: usize = 10;
pub(crate) const SHADER_ROW: u16 = 300;
pub(crate) const SHADER_ROWS: usize = 5;
/// The Lights page's lists: the added lights' rows, the lamps'.
pub(crate) const LIGHT_ROW: u16 = 400;
pub(crate) const LAMP_ROW: u16 = 450;
pub(crate) const LIGHT_ROWS: usize = 3;
/// Place's things list.
pub(crate) const THING_ROW: u16 = 500;
pub(crate) const THING_ROWS: usize = 6;
/// The shader list as tiles: seven a line, three lines.
pub(crate) const TILE_COLUMNS: usize = 7;
pub(crate) const SHADER_TILES: usize = TILE_COLUMNS * 3;
/// Selected: the shader list's switch between rows and tiles.
pub(crate) const TILES: u16 = 47;
/// How many nearby shaders a face's list starts with (Fill's candidates).
pub(crate) const FILL_CANDIDATES: usize = 6;

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
    Weather,
    Lights,
}

impl Tab {
    /// Every page, in the tabs' order.
    pub(crate) const ALL: [Self; 5] = [
        Self::Selected,
        Self::Place,
        Self::Lights,
        Self::Edits,
        Self::Weather,
    ];

    pub(crate) fn token(self) -> u16 {
        match self {
            Self::Selected => TAB_SELECTED,
            Self::Place => TAB_PLACE,
            Self::Edits => TAB_EDITS,
            Self::Weather => TAB_WEATHER,
            Self::Lights => TAB_LIGHTS,
        }
    }
}

/// A button of the Weather page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WeatherChoice {
    /// What `r_weatherForce` 1 to 4 make: drizzle, rain, a storm, snow.
    Preset(u32),
    /// The light ground fog on or off.
    Fog,
    /// The wind on or off.
    Wind,
    /// Keep the weather seen now.
    Seen,
    /// Give the map its own weather back.
    Own,
}

/// The first word of a weather command, lower case.
fn weather_word(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

const FALLING: &[&str] = &[
    "lightrain",
    "rain",
    "acidrain",
    "heavyrain",
    "snow",
    "sand",
    "spacedust",
];
const FOGS: &[&str] = &["mist", "fog", "heavyrainfog", "light_fog"];
const WINDS: &[&str] = &["wind", "constantwind", "gustingwind"];

/// Whether the weather `kept` has a command of `words`.
pub(crate) fn weather_has(kept: &[String], words: &[&str]) -> bool {
    kept.iter()
        .any(|command| words.contains(&weather_word(command).as_str()))
}

/// Whether the weather `kept` has fog, and wind.
pub(crate) fn weather_fog_and_wind(kept: &[String]) -> (bool, bool) {
    (weather_has(kept, FOGS), weather_has(kept, WINDS))
}

/// The weather commands a Preset, Fog or Wind button keeps, from those kept now (`kept`,
/// none for the map's own): a preset is `r_weatherForce`'s weather with the fog kept;
/// Fog adds `mist` or takes every fog out; Wind adds `wind` or takes every wind out.
/// Empty: the map's own weather again.
pub(crate) fn weather_choice(kept: &[String], choice: WeatherChoice) -> Vec<String> {
    let keep_others = |words: &[&str]| -> Vec<String> {
        kept.iter()
            .filter(|command| !words.contains(&weather_word(command).as_str()))
            .cloned()
            .collect()
    };
    match choice {
        WeatherChoice::Preset(level) => {
            let mut commands: Vec<String> = crate::weather::settings::forced_commands(level)
                .iter()
                .map(|command| (*command).to_owned())
                .collect();
            commands.extend(
                kept.iter()
                    .filter(|command| FOGS.contains(&weather_word(command).as_str()))
                    .cloned(),
            );
            commands
        }
        WeatherChoice::Fog if weather_has(kept, FOGS) => keep_others(FOGS),
        WeatherChoice::Fog => {
            let mut commands = kept.to_vec();
            commands.push(crate::weather::effects::MIST.to_owned());
            commands
        }
        WeatherChoice::Wind if weather_has(kept, WINDS) => keep_others(WINDS),
        WeatherChoice::Wind => {
            let mut commands = kept.to_vec();
            commands.push("wind".to_owned());
            commands
        }
        WeatherChoice::Seen | WeatherChoice::Own => Vec::new(),
    }
}

/// The preset `kept` amounts to (1 drizzle to 4 snow), if it is one: its falling weather.
pub(crate) fn weather_preset(kept: &[String]) -> Option<u32> {
    let falling: Vec<String> = kept
        .iter()
        .map(|command| weather_word(command))
        .filter(|word| FALLING.contains(&word.as_str()))
        .collect();
    (1..=crate::weather::settings::FORCE_MAX as u32).find(|&level| {
        crate::weather::settings::forced_commands(level)
            .iter()
            .filter(|command| FALLING.contains(command))
            .eq(falling.iter())
    })
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
    /// Select the faces joined to the selected one.
    Link(Grow),
    /// Draw the selected faces with the nearby texture.
    Fill,
    /// `mapedit_select shader` and `mapedit_select placed` (Shift: added).
    SelectShader,
    SelectPlaced,
    /// `mapedit_duplicate`: copies of the selected placed models (Ctrl+D).
    Duplicate,
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
    /// Place: stand a thing where the next click meets the floor (again: stop).
    PlaceThing(super::web::http::Thing),
    /// Place: take the chosen thing away; the toy kind placed next; a toy's escort.
    ThingRemove,
    ThingKind,
    ThingEscort,
    /// Ctrl+C and Ctrl+V outside a field ([`super::clip`]).
    ClipCopy,
    ClipPaste,
    /// The invisible brushes' view (none: the next one, the B key).
    Brushes(Option<BrushPreset>),
    /// An edit chosen in the list: a placed model becomes the one the controls move.
    Choose,
    Delete,
    Undo,
    Redo,
    Scope(Scope),
    Publish,
    /// Escape: back out one step ([`super::editor::escape_step`]).
    Escape,
    EditorOff,
    /// The Recent rows' switch, and forgetting them.
    Remember,
    Forget,
    /// A button of the Weather page.
    Weather(WeatherChoice),
    /// Lights: add one where the next click lands (again: stop), or at the view's centre.
    LightAdd,
    LightAtView,
    /// Lights: delete the chosen light, or reset the chosen lamp.
    LightDelete,
    /// Lights: the chosen light's reach or strength times the factor, or a swatch's colour.
    LightRadius(f32),
    LightIntensity(f32),
    LightColour(usize),
    /// Lights: the chosen lamp off or on, and back as the map has it.
    LampToggle,
    LampReset,
    /// F: glide the camera to frame the selection (`mapedit_frame`).
    Frame,
    /// A step of the selected faces' texture alignment (`mapedit_align`).
    Align(super::align::Step),
    /// Place's Surface switch (`mapedit_surface`).
    OnSurface,
    /// Grow, shrink or invert the selection (`mapedit_select grow|shrink|invert`).
    Ring(super::select_grow::Ring),
    /// M: the measure tool on or off (`mapedit_measure`).
    Measure,
    /// A double click on a row: glide the camera to what it names.
    FlyTo(FlyTarget),
}

/// What a double click on a row flies to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FlyTarget {
    /// The chosen edit of the Edits list.
    Edit,
    /// The chosen thing of Place's things.
    Thing,
    /// The chosen added light, or lamp, of Lights.
    Light,
    Lamp,
}

/// A second click on the same row this soon after the first is a double click.
const DOUBLE_CLICK: std::time::Duration = std::time::Duration::from_millis(450);

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
    /// How many the selection holds (a multiple selection's count).
    pub(crate) count: usize,
    /// How many placed models a multiple selection holds: the gizmo and the placed
    /// model's buttons move them all.
    pub(crate) models: usize,
    /// What Fill would draw on the selected faces: the nearby vote's first shader and its
    /// share.
    pub(crate) fill: Option<(String, f32)>,
    /// The nearby vote's first [`FILL_CANDIDATES`], with their shares: a face's shader
    /// list starts with them.
    pub(crate) fill_candidates: Vec<(String, f32)>,
    /// The nearby vote round the selected drawn surfaces, and the map's most used
    /// shaders: with the family of the selection's shader, the Suggested rows
    /// ([`super::suggest`]).
    pub(crate) nearby: Vec<(String, f32)>,
    pub(crate) map_usage: Vec<(String, u32)>,
    /// Why the editor has the player's own view, not the free camera, when it has.
    pub(crate) camera: Option<String>,
    /// The editor flies (the right mouse button held).
    pub(crate) flying: bool,
    /// The first key bound to `mapedit` (the one that leaves), or empty.
    pub(crate) exit_key: String,
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
    /// The weather an edit keeps here (its commands), and whether that edit is for this
    /// server only; `None`: the map's own.
    pub(crate) weather: Option<(Vec<String>, bool)>,
    /// The map's own weather, as the server (or the map) names it.
    pub(crate) map_weather: Vec<String>,
    /// The player forces weather (`r_weatherForce`), which replaces the kept one here.
    pub(crate) forced: bool,
    /// The added lights here, and the map's lamp fixtures nearest first (read while the
    /// Lights page shows), and whether the real-time lighting finds lamps.
    pub(crate) lights: Vec<super::panel_lights::LightRow>,
    pub(crate) lamps: Vec<super::panel_lights::LampRow>,
    pub(crate) realtime: bool,
    /// The things standing on the map (read while Place shows them), the hub's toy kinds,
    /// whether the game is on a server, and whether the player is SJK's creator (boxes and
    /// toys are the creator's).
    pub(crate) things: Vec<super::web::things::ThingRow>,
    pub(crate) toy_kinds: Vec<String>,
    pub(crate) on_server: bool,
    pub(crate) creator: bool,
    /// The invisible brushes' view in force, when it is one of the presets.
    pub(crate) brushes: Option<BrushPreset>,
    /// How the newest selected face's texture lies, when an edit draws it.
    pub(crate) align: Option<super::document::Align>,
    /// Models placed and grabbed stand along the surface (`mapedit_surface`).
    pub(crate) on_surface: bool,
}

/// What the panel's facts were read for ([`Panel`]'s `seen`): the document's revision,
/// the current placed model, grabbing, the map's checksum known, the free camera, the
/// flight, the page, the things standing.
type Seen = (u64, Option<u32>, bool, bool, bool, bool, Tab, usize);

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
    /// grabbing, whether the map's checksum was known, the free camera, the flight, the
    /// page.
    seen: Option<Seen>,
    /// The Recent rows (newest first, [`super::recent`]) and their switch.
    recent_models: Vec<String>,
    recent_shaders: Vec<String>,
    pub(crate) remember: bool,
    /// How many rows at the head of each list are Recent ones.
    pub(crate) recent_model_rows: usize,
    pub(crate) recent_shader_rows: usize,
    /// The faces the last Fill suggestion was voted for.
    fill_for: Vec<[u32; 2]>,
    /// The gizmo transforms the placed model: the keys line names its keys.
    pub(crate) transforming: bool,
    /// The shader list drawn as tiles (pictures only, more of them) instead of rows.
    pub(crate) tiles: bool,
    /// How many rows at the head of the shader list are Suggested ones (before the
    /// Recent ones), and why each is ([`super::suggest`]).
    pub(crate) candidate_rows: usize,
    pub(crate) suggested: Vec<super::suggest::Reason>,
    /// The drawn surfaces the last surface vote was asked for.
    surfaces_for: Option<(String, bool, Vec<usize>)>,
    /// The shader row or tile under the pointer (its index in the list), for the preview.
    pub(crate) hover_shader: Option<usize>,
    /// Where the large preview card lies (window pixels): a press there is the panel's.
    pub(crate) preview_rect: Option<Rect>,
    /// The shaders' pictures ([`super::pictures`]).
    pub(crate) pictures: super::pictures::Pictures,
    /// The Lights page: its lists' chosen rows, which list the buttons act on, adding a
    /// light where the next click lands, and choosing the newest light once it is read.
    pub(crate) light_row: usize,
    pub(crate) light_first: usize,
    pub(crate) lamp_row: usize,
    pub(crate) lamp_first: usize,
    pub(crate) light_choice: super::panel_lights::LightChoice,
    pub(crate) adding_light: bool,
    pub(crate) choose_newest_light: bool,
    pub(crate) light_list: Option<Rect>,
    pub(crate) lamp_list: Option<Rect>,
    /// Place shows the things on the map instead of the models; their list's chosen row,
    /// the thing the next click stands, the toy kind and its escort.
    pub(crate) things: bool,
    pub(crate) thing_row: usize,
    pub(crate) thing_first: usize,
    pub(crate) placing_thing: Option<super::web::http::Thing>,
    pub(crate) toy_kind: usize,
    pub(crate) escort: bool,
    pub(crate) thing_list: Option<Rect>,
    /// The last copy (Ctrl+C), with its angles and scale ([`super::clip`]).
    pub(crate) clip: Option<super::clip::Clip>,
    /// The pointer over the scene: the hover, a box being drawn ([`super::select`]).
    pub(crate) scene: super::select::ScenePointer,
    /// The remap previewed from the shader list ([`super::preview`]).
    pub(crate) preview: super::preview::Preview,
    /// The last click on a list's row (its token) and when: a second soon after is a
    /// double click ([`DOUBLE_CLICK`]).
    last_click: Option<(u16, std::time::Instant)>,
    /// Selected shows a face's texture alignment instead of the shader list.
    pub(crate) aligning: bool,
    /// The measure tool ([`super::measure`]).
    pub(crate) measure: super::measure::Measure,
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

    /// Take the Recent rows (newest first) and their switch, narrowing the lists again
    /// when they changed.
    pub(crate) fn set_recent(&mut self, recent: &super::recent::Recent) {
        self.remember = recent.remember;
        if self.recent_models != recent.models {
            self.recent_models.clone_from(&recent.models);
            self.refilter();
        }
        if self.recent_shaders != recent.shaders {
            self.recent_shaders.clone_from(&recent.shaders);
            self.refilter_shaders();
        }
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
        // The Suggested rows first (whatever the search: on faces Fill's candidates lead
        // them), then the Recent rows (those the search keeps), then every match.
        let own_shader = self
            .facts
            .selection
            .as_ref()
            .and_then(|selection| selection.shader.as_deref());
        let nearby = if self.facts.fill_candidates.is_empty() {
            &self.facts.nearby
        } else {
            &self.facts.fill_candidates
        };
        // A caulk or nodraw face's own shader is a tool's: its family (caulk_water,
        // caulk_lava...) is nothing to draw on it.
        let on_face = self
            .facts
            .selection
            .as_ref()
            .is_some_and(|selection| selection.picked == Picked::Face);
        let kin = own_shader
            .filter(|_| !on_face)
            .map_or_else(Vec::new, |own| super::suggest::family(own, &self.shaders));
        let shaders = &self.shaders;
        let suggestions =
            super::suggest::merge(own_shader, nearby, &kin, &self.facts.map_usage, |shader| {
                shaders
                    .binary_search_by(|known| known.as_str().cmp(shader))
                    .is_ok()
            });
        self.suggested.clear();
        for (name, reason) in suggestions {
            if let Ok(index) = self.shaders.binary_search(&name) {
                self.shader_shown.push(index as u32);
                self.suggested.push(reason);
            }
        }
        self.candidate_rows = self.shader_shown.len();
        for name in &self.recent_shaders {
            let stem = shader_stem(name);
            if matches(&stem, &query)
                && let Ok(index) = self.shaders.binary_search(&stem)
                && !self.shader_shown.contains(&(index as u32))
            {
                self.shader_shown.push(index as u32);
            }
        }
        self.recent_shader_rows = self.shader_shown.len() - self.candidate_rows;
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
            self.shader_shown[self.candidate_rows + self.recent_shader_rows..]
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
        // The name typed, else the first Suggested row (the list opens on them), else the
        // selection's own shader among its neighbours.
        self.shader_row = named(&query)
            .or_else(|| (self.candidate_rows > 0).then_some(0))
            .or_else(|| own.as_deref().and_then(named))
            .unwrap_or(0);
        self.shader_first = 0;
        self.settle_shaders();
    }

    /// How many shaders the list shows at once, and how many a line: rows or tiles.
    pub(crate) fn shader_slots(&self) -> (usize, usize) {
        if self.tiles {
            (SHADER_TILES, TILE_COLUMNS)
        } else {
            (SHADER_ROWS, 1)
        }
    }

    /// Keep the chosen shader on show; a tile list scrolls by whole lines.
    fn settle_shaders(&mut self) {
        let (slots, per_line) = self.shader_slots();
        let count = self.shader_shown.len();
        self.shader_row = self.shader_row.min(count.saturating_sub(1));
        let line = self.shader_row / per_line;
        let lines = slots / per_line;
        let mut first = self.shader_first / per_line;
        if line < first {
            first = line;
        } else if line >= first + lines {
            first = line + 1 - lines;
        }
        let last = count.div_ceil(per_line).saturating_sub(lines);
        self.shader_first = first.min(last) * per_line;
    }

    /// Switch the shader list between rows and tiles.
    pub(crate) fn toggle_tiles(&mut self) {
        self.tiles = !self.tiles;
        self.shader_first = 0;
        self.settle_shaders();
        if self.is_shader_row(self.focus) {
            self.focus = SHADER_ROW + (self.shader_row - self.shader_first) as u16;
        }
    }

    /// The picture of the shader list's row `index`, asked for when not held (see
    /// [`super::pictures::Pictures::thumbnail`]).
    pub(crate) fn shader_picture(&mut self, index: usize) -> Option<Option<sjk_ui::TextureId>> {
        let shader = self
            .shader_shown
            .get(index)
            .and_then(|&index| self.shaders.get(index as usize))?;
        self.pictures.thumbnail(shader)
    }

    /// The share of the vote of the shader list's row `index`, when it is suggested for
    /// being nearby.
    pub(crate) fn candidate_share(&self, index: usize) -> Option<f32> {
        match self.suggestion(index)? {
            super::suggest::Reason::Nearby(share) => Some(share),
            _ => None,
        }
    }

    /// Why the shader list's row `index` is suggested, when it is.
    pub(crate) fn suggestion(&self, index: usize) -> Option<super::suggest::Reason> {
        self.suggested.get(index).copied()
    }

    /// What Fill draws: the chosen row when it is one of Fill's candidates (suggested for
    /// being nearby on faces), else the vote's first.
    pub(crate) fn fill_choice(&self) -> Option<&str> {
        (!self.facts.fill_candidates.is_empty() && self.candidate_share(self.shader_row).is_some())
            .then(|| self.chosen_shader())
            .flatten()
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
        // The Recent rows first (those the search keeps), then every match.
        for name in &self.recent_models {
            if matches(name, &query)
                && let Ok(index) = self.models.binary_search(name)
                && !self.shown.contains(&(index as u32))
            {
                self.shown.push(index as u32);
            }
        }
        self.recent_model_rows = self.shown.len();
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
        if tab != Tab::Lights {
            self.adding_light = false;
        }
        if tab != Tab::Place {
            self.placing_thing = None;
        }
    }

    fn is_thing_row(&self, token: u16) -> bool {
        (THING_ROW..THING_ROW + THING_ROWS as u16).contains(&token)
    }

    /// The thing chosen in Place's things list.
    pub(crate) fn chosen_thing(&self) -> Option<&super::web::things::ThingRow> {
        self.facts.things.get(self.thing_row)
    }

    /// The toy kind placed next, of the hub's.
    pub(crate) fn toy_kind(&self) -> Option<&str> {
        let kinds = &self.facts.toy_kinds;
        (!kinds.is_empty()).then(|| kinds[self.toy_kind % kinds.len()].as_str())
    }

    /// Keep the Lights page's chosen rows on show.
    pub(crate) fn settle_light_lists(&mut self) {
        Self::settle(
            &mut self.light_row,
            &mut self.light_first,
            self.facts.lights.len(),
            LIGHT_ROWS,
        );
        Self::settle(
            &mut self.lamp_row,
            &mut self.lamp_first,
            self.facts.lamps.len(),
            LIGHT_ROWS,
        );
    }

    fn is_light_row(&self, token: u16) -> bool {
        (LIGHT_ROW..LIGHT_ROW + LIGHT_ROWS as u16).contains(&token)
    }

    fn is_lamp_row(&self, token: u16) -> bool {
        (LAMP_ROW..LAMP_ROW + LIGHT_ROWS as u16).contains(&token)
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
                return self.move_in_shaders(by, typing);
            } else if self.is_light_row(self.focus) {
                (
                    &mut self.light_row,
                    &mut self.light_first,
                    self.facts.lights.len(),
                    LIGHT_ROWS,
                    LIGHT_ROW,
                )
            } else if self.is_lamp_row(self.focus) {
                (
                    &mut self.lamp_row,
                    &mut self.lamp_first,
                    self.facts.lamps.len(),
                    LIGHT_ROWS,
                    LAMP_ROW,
                )
            } else if self.is_thing_row(self.focus) {
                (
                    &mut self.thing_row,
                    &mut self.thing_first,
                    self.facts.things.len(),
                    THING_ROWS,
                    THING_ROW,
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

    /// [`Panel::move_in_list`] in the shader list: `by` shaders.
    pub(crate) fn move_in_shaders(&mut self, by: isize, typing: bool) -> bool {
        let count = self.shader_shown.len();
        let wanted = self.shader_row.saturating_add_signed(by);
        if count == 0 || wanted >= count || (by < 0 && self.shader_row < by.unsigned_abs()) {
            return typing;
        }
        self.shader_row = wanted;
        self.settle_shaders();
        if !typing {
            self.focus = SHADER_ROW + (self.shader_row - self.shader_first) as u16;
        }
        true
    }

    /// Up or Down: a line of the list the keyboard is on (a tile line is several).
    pub(crate) fn move_line(&mut self, forward: bool) -> bool {
        let per_line = if self.is_shader_row(self.focus) || self.focus == REMAP_FIELD {
            self.shader_slots().1 as isize
        } else {
            1
        };
        self.move_in_list(if forward { per_line } else { -per_line })
    }

    fn is_model_row(&self, token: u16) -> bool {
        (MODEL_ROW..MODEL_ROW + MODEL_ROWS as u16).contains(&token)
    }

    fn is_edit_row(&self, token: u16) -> bool {
        (EDIT_ROW..EDIT_ROW + EDIT_ROWS as u16).contains(&token)
    }

    fn is_shader_row(&self, token: u16) -> bool {
        (SHADER_ROW..SHADER_ROW + self.shader_slots().0 as u16).contains(&token)
    }

    /// Up, Down or Tab: along a list, else the next control in the drawn order.
    pub(crate) fn step(&mut self, canvas: &mut MenuCanvas, forward: bool) {
        self.step_by(canvas, forward, true);
    }

    /// [`Panel::step`]; `along` false (Tab in a search) leaves at once.
    fn step_by(&mut self, canvas: &mut MenuCanvas, forward: bool, along: bool) {
        if along && self.move_line(forward) {
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
            } else if self.is_light_row(token) {
                self.focus = LIGHT_ROW + (self.light_row.saturating_sub(self.light_first)) as u16;
            } else if self.is_lamp_row(token) {
                self.focus = LAMP_ROW + (self.lamp_row.saturating_sub(self.lamp_first)) as u16;
            } else if self.is_thing_row(token) {
                self.focus = THING_ROW + (self.thing_row.saturating_sub(self.thing_first)) as u16;
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
        if inside(self.shader_list) {
            let (slots, per_line) = self.shader_slots();
            let step = if per_line > 1 { per_line as isize } else { 3 };
            let step = if delta.y > 0.0 { -step } else { step };
            let count = self.shader_shown.len();
            let last = count.div_ceil(per_line).saturating_sub(slots / per_line) * per_line;
            self.shader_first = self.shader_first.saturating_add_signed(step).min(last);
            return true;
        }
        let (first, count, rows) = if inside(self.model_list) {
            (&mut self.model_first, self.shown.len(), MODEL_ROWS)
        } else if inside(self.light_list) {
            (&mut self.light_first, self.facts.lights.len(), LIGHT_ROWS)
        } else if inside(self.lamp_list) {
            (&mut self.lamp_first, self.facts.lamps.len(), LIGHT_ROWS)
        } else if inside(self.thing_list) {
            (&mut self.thing_first, self.facts.things.len(), THING_ROWS)
        } else if inside(self.edit_list) {
            (&mut self.edit_first, self.facts.rows.len(), EDIT_ROWS)
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
            if !self.is_model_row(token)
                && !self.is_edit_row(token)
                && !self.is_shader_row(token)
                && !self.is_light_row(token)
                && !self.is_lamp_row(token)
                && !self.is_thing_row(token)
            {
                self.focus = token;
            }
            return None;
        }
        if event.kind != UiEventKind::Activate {
            return None;
        }
        self.focus = token;
        let double = self.double_click(token, std::time::Instant::now());
        let action = self.activate(token);
        match double {
            Some(target) => Some(Action::FlyTo(target)),
            None => action,
        }
    }

    /// A click on `token` at `now`: what it flies to when it is the second click of a
    /// double click on a row of the edits, things, lights or lamps.
    pub(crate) fn double_click(
        &mut self,
        token: u16,
        now: std::time::Instant,
    ) -> Option<FlyTarget> {
        let target = if self.is_edit_row(token) {
            FlyTarget::Edit
        } else if self.is_thing_row(token) {
            FlyTarget::Thing
        } else if self.is_light_row(token) {
            FlyTarget::Light
        } else if self.is_lamp_row(token) {
            FlyTarget::Lamp
        } else {
            self.last_click = None;
            return None;
        };
        let second = self
            .last_click
            .is_some_and(|(last, when)| last == token && now.duration_since(when) <= DOUBLE_CLICK);
        self.last_click = (!second).then_some((token, now));
        second.then_some(target)
    }

    /// Enter, Space or a click on `token`.
    pub(crate) fn activate(&mut self, token: u16) -> Option<Action> {
        match token {
            TAB_SELECTED => self.show(Tab::Selected),
            TAB_PLACE => self.show(Tab::Place),
            TAB_EDITS => self.show(Tab::Edits),
            TAB_WEATHER => self.show(Tab::Weather),
            TAB_LIGHTS => self.show(Tab::Lights),
            brushes if (BRUSHES..BRUSHES + 4).contains(&brushes) => {
                return Some(Action::Brushes(Some(
                    BrushPreset::ALL[usize::from(brushes - BRUSHES)],
                )));
            }
            LIGHT_ADD => return Some(Action::LightAdd),
            LIGHT_AT_VIEW => return Some(Action::LightAtView),
            LIGHT_DELETE => return Some(Action::LightDelete),
            LIGHT_SMALLER => {
                return Some(Action::LightRadius(1.0 / super::panel_lights::STEP));
            }
            LIGHT_BIGGER => return Some(Action::LightRadius(super::panel_lights::STEP)),
            LIGHT_DIMMER => {
                return Some(Action::LightIntensity(1.0 / super::panel_lights::STEP));
            }
            LIGHT_BRIGHTER => return Some(Action::LightIntensity(super::panel_lights::STEP)),
            LAMP_TOGGLE => return Some(Action::LampToggle),
            LAMP_RESET => return Some(Action::LampReset),
            swatch if (LIGHT_SWATCH..LIGHT_SWATCH + 8).contains(&swatch) => {
                return Some(Action::LightColour(usize::from(swatch - LIGHT_SWATCH)));
            }
            row if self.is_light_row(row) => {
                let index = self.light_first + usize::from(row - LIGHT_ROW);
                if index < self.facts.lights.len() {
                    self.light_row = index;
                    self.light_choice = super::panel_lights::LightChoice::Light;
                }
            }
            row if self.is_lamp_row(row) => {
                let index = self.lamp_first + usize::from(row - LAMP_ROW);
                if index < self.facts.lamps.len() {
                    self.lamp_row = index;
                    self.light_choice = super::panel_lights::LightChoice::Lamp;
                }
            }
            WEATHER_DRIZZLE => return Some(Action::Weather(WeatherChoice::Preset(1))),
            WEATHER_RAIN => return Some(Action::Weather(WeatherChoice::Preset(2))),
            WEATHER_STORM => return Some(Action::Weather(WeatherChoice::Preset(3))),
            WEATHER_SNOW => return Some(Action::Weather(WeatherChoice::Preset(4))),
            WEATHER_FOG => return Some(Action::Weather(WeatherChoice::Fog)),
            WEATHER_WIND => return Some(Action::Weather(WeatherChoice::Wind)),
            WEATHER_SEEN => return Some(Action::Weather(WeatherChoice::Seen)),
            WEATHER_OWN => return Some(Action::Weather(WeatherChoice::Own)),
            HIDE => return Some(Action::Hide),
            HIDE_GROUP => return Some(Action::HideGroup),
            COPY => return Some(Action::Copy),
            REMAP | REMAP_FIELD => return Some(Action::Remap),
            UNREMAP => return Some(Action::Unremap),
            SKY_ALL => return Some(Action::SkyAll),
            LINK_PLANE => return Some(Action::Link(Grow::Plane)),
            LINK_SURFACE => return Some(Action::Link(Grow::Surface)),
            FILL => return Some(Action::Fill),
            SELECT_SHADER => return Some(Action::SelectShader),
            SELECT_PLACED => return Some(Action::SelectPlaced),
            DUPLICATE => return Some(Action::Duplicate),
            TILES => self.toggle_tiles(),
            PLACE_SURFACE => return Some(Action::OnSurface),
            SELECT_MORE => return Some(Action::Ring(super::select_grow::Ring::Grow)),
            SELECT_LESS => return Some(Action::Ring(super::select_grow::Ring::Shrink)),
            ALIGN => self.aligning = !self.aligning,
            step if (ALIGN_STEP..ALIGN_STEP + ALIGN_STEPS.len() as u16).contains(&step) => {
                return Some(Action::Align(ALIGN_STEPS[usize::from(step - ALIGN_STEP)].1));
            }
            REMEMBER => return Some(Action::Remember),
            FORGET => return Some(Action::Forget),
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
            PLACE_THINGS | PLACE_MODELS => {
                self.things = token == PLACE_THINGS;
                self.placing_thing = None;
                self.focus = if self.things {
                    PLACE_MODELS
                } else {
                    PLACE_THINGS
                };
                self.seen = None;
            }
            THING_BOOMBOX => return Some(Action::PlaceThing(super::web::http::Thing::Boombox)),
            THING_BOX => return Some(Action::PlaceThing(super::web::http::Thing::MysteryBox)),
            THING_TOY => return Some(Action::PlaceThing(super::web::http::Thing::Toy)),
            THING_KIND => return Some(Action::ThingKind),
            THING_ESCORT => return Some(Action::ThingEscort),
            THING_REMOVE => return Some(Action::ThingRemove),
            row if self.is_thing_row(row) => {
                let index = self.thing_first + usize::from(row - THING_ROW);
                if index < self.facts.things.len() {
                    self.thing_row = index;
                }
            }
            DELETE => return Some(Action::Delete),
            UNDO => return Some(Action::Undo),
            REDO => return Some(Action::Redo),
            SCOPE_ALL => return Some(Action::Scope(Scope::AllServers)),
            SCOPE_SERVER => return Some(Action::Scope(Scope::ThisServer)),
            PUBLISH => return Some(Action::Publish),
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
            KeyCode::Escape if !event.repeat => return Some(Action::Escape),
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
                    TAB_SELECTED | TAB_PLACE | TAB_EDITS | TAB_WEATHER | TAB_LIGHTS => {
                        let tabs = Tab::ALL;
                        let count = tabs.len();
                        let at = tabs.iter().position(|tab| *tab == self.tab).unwrap_or(0);
                        let next = if right {
                            (at + 1) % count
                        } else {
                            (at + count - 1) % count
                        };
                        self.show(tabs[next]);
                    }
                    focus if self.tiles && (self.is_shader_row(focus) || focus == REMAP_FIELD) => {
                        let typing = focus == REMAP_FIELD;
                        let _ = self.move_in_shaders(if right { 1 } else { -1 }, typing);
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
            KeyCode::KeyC | KeyCode::KeyV
                if control && !event.repeat && !matches!(self.focus, SEARCH | REMAP_FIELD) =>
            {
                return Some(if key == KeyCode::KeyC {
                    Action::ClipCopy
                } else {
                    Action::ClipPaste
                });
            }
            KeyCode::KeyD
                if control && !event.repeat && !matches!(self.focus, SEARCH | REMAP_FIELD) =>
            {
                return Some(Action::Duplicate);
            }
            KeyCode::KeyB
                if !control && !event.repeat && !matches!(self.focus, SEARCH | REMAP_FIELD) =>
            {
                return Some(Action::Brushes(None));
            }
            KeyCode::KeyF
                if !control && !event.repeat && !matches!(self.focus, SEARCH | REMAP_FIELD) =>
            {
                return Some(Action::Frame);
            }
            KeyCode::KeyM
                if !control && !event.repeat && !matches!(self.focus, SEARCH | REMAP_FIELD) =>
            {
                return Some(Action::Measure);
            }
            // Blender's Select More, Less and Invert: Ctrl with + (or =), - and I.
            KeyCode::Equal
            | KeyCode::NumpadAdd
            | KeyCode::Minus
            | KeyCode::NumpadSubtract
            | KeyCode::KeyI
                if control && !event.repeat && !matches!(self.focus, SEARCH | REMAP_FIELD) =>
            {
                use super::select_grow::Ring;
                return Some(Action::Ring(match key {
                    KeyCode::Equal | KeyCode::NumpadAdd => Ring::Grow,
                    KeyCode::KeyI => Ring::Invert,
                    _ => Ring::Shrink,
                }));
            }
            KeyCode::Delete if self.is_edit_row(self.focus) && !event.repeat => {
                return Some(Action::Delete);
            }
            KeyCode::Delete
                if (self.is_light_row(self.focus) || self.is_lamp_row(self.focus))
                    && !event.repeat =>
            {
                return Some(Action::LightDelete);
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
    pub(crate) fn answer(&mut self, result: Result<Vec<String>, String>) {
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
    pub(crate) fn stale(&mut self) {
        self.seen = None;
    }
}

impl GpuState {
    /// Whether the editor's panel is on show with the pointer.
    pub(crate) fn map_edit_panel_open(&self) -> bool {
        self.game_menu && self.game_menu_page == Page::MapEdit
    }

    /// Whether the editor's panel is drawn: with the pointer, or while the editor flies.
    pub(crate) fn map_edit_panel_shown(&self) -> bool {
        self.map_edit_panel_open()
            || (self.map_edit_flying() && self.game_menu_page == Page::MapEdit)
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

    /// Close the panel (the editor going off, or a world shot).
    pub(crate) fn close_map_edit_panel(&mut self) {
        self.world_notes.set_hover(None);
        self.map_edits.panel.scene = super::select::ScenePointer::default();
        self.map_edit_preview_off();
        if self.map_edit_panel_open() {
            self.game_menu = false;
            self.game_menu_page = Page::Main;
            self.capture_pointer();
        }
    }

    /// A key while the panel is open: all of them are its own; while the editor flies,
    /// the movement keys go to the game ([`super::fly`]). The key bound to `mapedit`
    /// leaves the editor either way. False when the editor does not take keys.
    pub(crate) fn map_edit_key(&mut self, event: &KeyEvent) -> bool {
        let flying = self.map_edit_flying();
        if !self.map_edit_panel_open() && !flying {
            return false;
        }
        let pressed = event.state == ElementState::Pressed;
        // A letter typed in a search stays the search's, even when it is the bound key.
        let typing = !flying
            && matches!(self.map_edits.panel.focus, SEARCH | REMAP_FIELD)
            && event
                .text
                .as_deref()
                .is_some_and(|text| text.chars().any(|c| !c.is_control()));
        if pressed && !event.repeat && !typing && self.map_edit_bound_key(event) {
            let result = self.map_edit_command("mapedit", &["off".to_owned()]);
            if result.is_err() {
                self.map_edits.panel.answer(result);
            }
            return true;
        }
        if flying {
            if pressed && !event.repeat && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
            {
                self.map_edit_escape();
                return true;
            }
            return pressed && self.map_edit_fly_holds_key(event);
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

    /// Whether key `event` is bound to `mapedit` (the toggle that leaves).
    fn map_edit_bound_key(&self, event: &KeyEvent) -> bool {
        let Some(name) = crate::input::keys::key_name(event) else {
            return false;
        };
        self.console
            .as_ref()
            .and_then(|console| console.key_script(name.as_str()))
            .is_some_and(|script| script.trim().eq_ignore_ascii_case("mapedit"))
    }

    /// A pointer event while the panel is open: the column's, or one on the scene: a
    /// click selects what is under the pointer (Shift: adds it, or takes it out; a
    /// grabbed model: puts it down), the right button held flies, the wheel turns a
    /// grabbed model.
    pub(crate) fn map_edit_pointer(&mut self, event: InputEvent) {
        let scene_right = self.map_edits.panel.scene_right;
        // A press held on the scene: its moves draw a box, its release selects
        // ([`super::select`]); the gizmo never saw that press.
        match event {
            InputEvent::PointerMove(at) => {
                if self.map_edit_scene_move([at.x, at.y]) {
                    return;
                }
            }
            InputEvent::PointerRelease {
                button: PointerButton::Primary,
                ..
            } => {
                if self.map_edit_scene_release() {
                    return;
                }
            }
            InputEvent::PointerLeave => self.map_edits.panel.scene.at = None,
            _ => {}
        }
        // The preview card stands over the scene's edge: a press on it is the panel's.
        if let InputEvent::PointerPress { position, .. } = event
            && self
                .map_edits
                .panel
                .preview_rect
                .is_some_and(|rect| rect.contains(position))
        {
            return;
        }
        // A grabbed model under the pointer: a click puts it down, before its gizmo.
        if let InputEvent::PointerPress {
            position,
            button: PointerButton::Primary,
        } = event
            && position.x < scene_right
            && self.map_edits.editor.grabbing()
        {
            let result = self.map_edit_command("mapedit_grab", &[]);
            let panel = &mut self.map_edits.panel;
            panel.answer(result);
            panel.stale();
            return;
        }
        // A press on the gizmo's handles, and its drags, are the gizmo's.
        if self.map_edit_gizmo_pointer(event) {
            return;
        }
        let (shift, control) = self.console.as_ref().map_or((false, false), |console| {
            (console.shift_held(), console.control_held())
        });
        match event {
            InputEvent::PointerPress {
                position,
                button: PointerButton::Secondary,
            } if position.x < scene_right => {
                self.map_edit_fly_start([position.x, position.y]);
                return;
            }
            InputEvent::PointerPress {
                position,
                button: PointerButton::Primary,
            } if position.x < scene_right => {
                let point = [position.x, position.y];
                if self.map_edits.panel.adding_light {
                    self.map_edit_light_add_at(point);
                    return;
                }
                if self.map_edits.panel.measure.on {
                    self.map_edit_measure_at(point);
                    return;
                }
                if let Some(thing) = self.map_edits.panel.placing_thing {
                    self.map_edit_thing_at(thing, point);
                    return;
                }
                if self.map_edits.panel.tab == Tab::Lights
                    && let Some((choice, index)) =
                        self.map_edit_marker_at(glam::Vec2::new(position.x, position.y))
                {
                    self.map_edit_choose_light(choice, index);
                    return;
                }
                // A click selects at the release; a drag draws a box.
                let combine = if control {
                    crate::world_notes::Combine::Remove
                } else if shift {
                    crate::world_notes::Combine::Add
                } else {
                    crate::world_notes::Combine::Replace
                };
                self.map_edit_scene_press(point, combine);
                return;
            }
            InputEvent::PointerWheel { position, delta }
                if position.x < scene_right && self.map_edits.editor.grabbing() =>
            {
                if delta.y != 0.0 {
                    let _ = self.map_edit_wheel(delta.y > 0.0, shift);
                }
                return;
            }
            _ => {}
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
            Action::Align(step) if fine => Action::Align(super::align::fine(step)),
            action => action,
        };
        let (name, args): (&str, Vec<String>) = match action {
            Action::Escape if self.map_edits.panel.measure.on => {
                ("mapedit_measure", text(&["off"]))
            }
            Action::Escape if self.map_edits.panel.adding_light => {
                let panel = &mut self.map_edits.panel;
                panel.adding_light = false;
                panel.answer(Ok(vec!["Stopped adding a light".into()]));
                return;
            }
            Action::Escape if self.map_edits.panel.placing_thing.is_some() => {
                let panel = &mut self.map_edits.panel;
                panel.placing_thing = None;
                panel.answer(Ok(vec!["Stopped placing".into()]));
                return;
            }
            Action::PlaceThing(thing) => {
                let panel = &mut self.map_edits.panel;
                let again = panel.placing_thing == Some(thing);
                panel.placing_thing = (!again).then_some(thing);
                panel.answer(Ok(vec![if again {
                    "Stopped placing".to_owned()
                } else {
                    "Click the floor where it should stand (Esc stops)".to_owned()
                }]));
                return;
            }
            Action::ThingKind => {
                let panel = &mut self.map_edits.panel;
                panel.toy_kind = panel.toy_kind.wrapping_add(1);
                let line = match panel.toy_kind() {
                    Some(kind) => format!("Toy placed next: {kind}"),
                    None => "The hub names no toy kind yet".to_owned(),
                };
                panel.answer(Ok(vec![line]));
                return;
            }
            Action::ThingEscort => {
                let panel = &mut self.map_edits.panel;
                panel.escort = !panel.escort;
                panel.answer(Ok(vec![if panel.escort {
                    "Toys placed next are shown only where you are".to_owned()
                } else {
                    "Toys placed next are shown to everyone".to_owned()
                }]));
                return;
            }
            Action::ClipCopy | Action::ClipPaste => {
                let result = if action == Action::ClipCopy {
                    self.map_edit_copy()
                } else {
                    self.map_edit_paste()
                };
                let panel = &mut self.map_edits.panel;
                panel.answer(result);
                panel.stale();
                return;
            }
            Action::Brushes(preset) => {
                let preset =
                    preset.unwrap_or_else(|| BrushPreset::next(self.map_edits.panel.facts.brushes));
                let (mode, types) = preset.cvars();
                if let Some(console) = self.console.as_mut() {
                    console.set_cvar(crate::brush_view::CVAR, mode);
                    console.set_cvar(crate::brush_view::TYPES_CVAR, types);
                }
                let panel = &mut self.map_edits.panel;
                panel.facts.brushes = Some(preset);
                panel.answer(Ok(vec![match preset {
                    BrushPreset::Off => "Invisible brushes hidden (r_showBrushes 0)".to_owned(),
                    BrushPreset::Play => {
                        "Invisible brushes: what play feels (r_showBrushes 1)".to_owned()
                    }
                    BrushPreset::Caulk => {
                        format!("Invisible brushes: {CAULK_KINDS} (B for the next view)")
                    }
                    BrushPreset::All => {
                        "Invisible brushes: every kind (r_showBrushes 2)".to_owned()
                    }
                }]));
                return;
            }
            Action::ThingRemove => {
                let Some(row) = self.map_edits.panel.chosen_thing().cloned() else {
                    self.map_edits
                        .panel
                        .answer(Err("Choose a thing in the list first".into()));
                    return;
                };
                let result = self.map_edit_web_remove_thing(row.thing, row.id);
                let panel = &mut self.map_edits.panel;
                panel.answer(result);
                panel.stale();
                return;
            }
            Action::Escape => {
                self.map_edit_escape();
                return;
            }
            Action::LightAdd
            | Action::LightAtView
            | Action::LightDelete
            | Action::LightRadius(_)
            | Action::LightIntensity(_)
            | Action::LightColour(_)
            | Action::LampToggle
            | Action::LampReset => match self.map_edit_light_action(action) {
                Some(command) => command,
                None => return,
            },
            Action::Remember | Action::Forget => {
                let remember =
                    (action == Action::Remember).then_some(!self.map_edits.panel.remember);
                self.map_edit_set_recent(remember);
                self.map_edits.panel.answer(Ok(vec![match remember {
                    Some(true) => "Recent textures and models are remembered".to_owned(),
                    Some(false) => "Recent textures and models are not remembered".to_owned(),
                    None => "Recent textures and models forgotten".to_owned(),
                }]));
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
            Action::Link(Grow::Plane) => ("mapedit_link", text(&["plane"])),
            Action::Link(Grow::Surface) => ("mapedit_link", text(&["surface"])),
            Action::SelectShader | Action::SelectPlaced => {
                let what = if action == Action::SelectShader {
                    "shader"
                } else {
                    "placed"
                };
                let mut words = vec![what.to_owned()];
                if fine {
                    words.push("add".to_owned());
                }
                ("mapedit_select", words)
            }
            Action::Duplicate => ("mapedit_duplicate", vec![]),
            Action::Fill => match self.map_edits.panel.fill_choice() {
                Some(shader) => ("mapedit_fill", vec![shader.to_owned()]),
                None => ("mapedit_fill", vec![]),
            },
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
            // A set of placed models only: all of them, one step.
            Action::Remove if self.map_edits.panel.facts.models > 1 => ("mapedit_hide", vec![]),
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
            Action::Frame => ("mapedit_frame", vec![]),
            Action::Measure => ("mapedit_measure", vec![]),
            Action::Ring(ring) => (
                "mapedit_select",
                text(&[match ring {
                    super::select_grow::Ring::Grow => "grow",
                    super::select_grow::Ring::Shrink => "shrink",
                    super::select_grow::Ring::Invert => "invert",
                }]),
            ),
            Action::OnSurface => {
                let on = !self.map_edits.editor.on_surface();
                ("mapedit_surface", text(&[if on { "on" } else { "off" }]))
            }
            Action::Align(step) => ("mapedit_align", super::align::words(step)),
            Action::FlyTo(target) => match self.map_edit_fly_target(target) {
                Ok((centre, radius)) => {
                    ("mapedit_frame", super::fly_to::frame_words(centre, radius))
                }
                Err(why) => {
                    self.map_edits.panel.answer(Err(why));
                    return;
                }
            },
            Action::Undo => ("mapedit_undo", vec![]),
            Action::Redo => ("mapedit_redo", vec![]),
            Action::Scope(Scope::AllServers) => ("mapedit_scope", text(&["all"])),
            Action::Scope(Scope::ThisServer) => ("mapedit_scope", text(&["server"])),
            Action::Publish => ("mapedit_publish", vec![]),
            Action::EditorOff => ("mapedit", text(&["off"])),
            Action::Weather(WeatherChoice::Seen) => ("mapedit_weather", vec![]),
            Action::Weather(choice) => {
                let kept = self
                    .map_edits
                    .panel
                    .facts
                    .weather
                    .as_ref()
                    .map_or(&[][..], |(commands, _)| commands.as_slice());
                let commands = weather_choice(kept, choice);
                if commands.is_empty() && kept.is_empty() {
                    self.map_edits
                        .panel
                        .answer(Ok(vec!["The map has its own weather already".into()]));
                    return;
                }
                let words = if commands.is_empty() {
                    "-".to_owned()
                } else {
                    commands.join(", ")
                };
                ("mapedit_weather", vec![words])
            }
        };
        let result = self.map_edit_command(name, &args);
        let ok = result.is_ok();
        let panel = &mut self.map_edits.panel;
        panel.answer(result);
        panel.stale();
        match action {
            Action::Remap if ok && self.world_notes.edit_selection_len() <= 1 => {
                panel.remap.clear();
            }
            Action::Link(_) | Action::SelectShader | Action::Ring(_) if ok => {
                panel.show(Tab::Selected);
            }
            _ => {}
        }
    }

    /// Where a double-clicked row's thing stands, and how big it is.
    fn map_edit_fly_target(&self, target: FlyTarget) -> Result<(glam::Vec3, f32), String> {
        use super::fly_to::POINT_RADIUS;
        let panel = &self.map_edits.panel;
        match target {
            FlyTarget::Edit => {
                let id = panel
                    .chosen_edit()
                    .ok_or("Choose an edit in the list first")?;
                self.map_edit_edit_sphere(id)
            }
            FlyTarget::Thing => panel
                .chosen_thing()
                .map(|thing| (thing.origin, POINT_RADIUS))
                .ok_or_else(|| "Choose a thing in the list first".to_owned()),
            FlyTarget::Light => {
                let row = panel
                    .facts
                    .lights
                    .get(panel.light_row)
                    .ok_or("Choose a light in the list first")?;
                Ok((glam::Vec3::from_array(row.origin), POINT_RADIUS))
            }
            FlyTarget::Lamp => panel
                .facts
                .lamps
                .get(panel.lamp_row)
                .map(|lamp| (lamp.at(), POINT_RADIUS))
                .ok_or_else(|| "Choose a lamp in the list first".to_owned()),
        }
    }

    /// What a Shift+click (or Shift and the world-note key) did, on the answer line.
    pub(super) fn map_edit_toggled(&mut self, toggled: crate::world_notes::Toggled) {
        use crate::world_notes::Toggled;
        let panel = &mut self.map_edits.panel;
        panel.answer(match toggled {
            Toggled::Added(1) => Ok(vec!["Selected (Shift+click adds more)".to_owned()]),
            Toggled::Added(count) => Ok(vec![format!("Added: {count} selected")]),
            Toggled::Removed(0) => Ok(vec!["Taken out: nothing selected".to_owned()]),
            Toggled::Removed(count) => Ok(vec![format!("Taken out: {count} selected")]),
            Toggled::NotAMember => Err(
                "Shift adds surfaces, faces and placed models; a map prop is selected alone"
                    .to_owned(),
            ),
            Toggled::Full(limit) if limit == crate::world_notes::PLACED_LIMIT => Err(format!(
                "The selection holds {limit} placed models at most (one step to undo)"
            )),
            Toggled::Full(limit) => Err(format!("The selection holds {limit} at most")),
            Toggled::Nothing => Err("Nothing there to edit".to_owned()),
        });
        if matches!(toggled, Toggled::Added(_) | Toggled::Removed(_)) && panel.tab != Tab::Place {
            panel.show(Tab::Selected);
        }
        panel.stale();
    }

    /// How many things stand on the map while Place shows them (0 otherwise): the facts
    /// are read again when it changes.
    fn map_edit_things_stamp(&self) -> usize {
        let panel = &self.map_edits.panel;
        if panel.tab != Tab::Place || !panel.things {
            return 0;
        }
        1 + self.boombox.standing().len()
            + self.mystery_boxes.standing().len()
            + self.toy_master.hidden().count()
    }

    /// A click on the scene while placing a thing: stand it where the click's ray meets
    /// the floor, facing the view, through its own code as the page does.
    fn map_edit_thing_at(&mut self, thing: super::web::http::Thing, point: [f32; 2]) {
        let Some(view) = self.map_edit_view() else {
            return;
        };
        let ray = view.ray(glam::Vec2::from_array(point));
        let panel = &self.map_edits.panel;
        let kind = panel.toy_kind().unwrap_or_default().to_owned();
        let one_server = self.map_edits.editor.scope() == Scope::ThisServer;
        let escort = panel.escort;
        let result = self.map_edit_web_place_thing(
            thing,
            [
                ray.origin.to_array(),
                ray.dir.to_array(),
                view.forward.to_array(),
            ],
            &kind,
            one_server,
            escort,
        );
        let panel = &mut self.map_edits.panel;
        if result.is_ok() {
            panel.placing_thing = None;
        }
        panel.answer(result);
        panel.stale();
    }

    /// The world offset of a nudge `right` of the view and `away` from it, along the
    /// world axes nearest those directions (a map's walls mostly follow them), so steps
    /// add up straight.
    pub(super) fn map_edit_nudge(&self, right: f32, away: f32) -> Option<[f32; 3]> {
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
    pub(super) fn map_edit_pick_at(&mut self, point: [f32; 2]) -> bool {
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
        self.map_edit_hover_tick();
        self.map_edit_preview_tick();
        let editor = &self.map_edits.editor;
        let key = (
            self.map_edits.revision,
            editor.current(),
            editor.grabbing(),
            self.map_edits.map().is_some(),
            self.free_camera_active(),
            self.map_edit_flying(),
            self.map_edits.panel.tab,
            self.map_edit_things_stamp(),
        );
        if self.map_edits.panel.seen == Some(key) {
            return;
        }
        self.map_edits.panel.seen = Some(key);
        self.world_notes.recheck_hover();
        let selection = self.map_edit_selected();
        let server = self.map_edits_server();
        let first_key = |command: &str| {
            self.console.as_ref().map_or_else(String::new, |console| {
                console
                    .keys_for_command(command)
                    .first()
                    .map(|key| {
                        let mut name = String::new();
                        sjk_shell::key_names::push_display_key(&mut name, key);
                        name
                    })
                    .unwrap_or_default()
            })
        };
        let select_key = first_key("worldnote");
        let exit_key = first_key("mapedit");
        // What Fill would draw, voted again only when the selected faces change.
        let faces = self.map_edit_selected_faces();
        if faces != self.map_edits.panel.fill_for {
            let candidates: Vec<(String, f32)> = if faces.is_empty() {
                Vec::new()
            } else {
                self.map_edit_vote(&faces)
                    .into_iter()
                    .take(FILL_CANDIDATES)
                    .map(|candidate| (candidate.shader, candidate.share))
                    .collect()
            };
            let panel = &mut self.map_edits.panel;
            panel.facts.fill = candidates.first().cloned();
            panel.facts.fill_candidates = candidates;
            panel.fill_for = faces;
        }
        // The vote round the selected drawn surfaces, again only when they change.
        let surfaces = (
            self.world_load_map.clone(),
            self.map_edits.map().is_some(),
            self.map_edit_selected_surfaces(),
        );
        if self.map_edits.panel.surfaces_for.as_ref() != Some(&surfaces) {
            let (nearby, usage) = if surfaces.1 {
                self.map_edit_surface_vote(&surfaces.2)
            } else {
                (Vec::new(), Vec::new())
            };
            let panel = &mut self.map_edits.panel;
            panel.facts.nearby = nearby;
            panel.facts.map_usage = usage;
            panel.surfaces_for = Some(surfaces);
        }
        let nearby = self.map_edits.panel.facts.nearby.clone();
        let map_usage = self.map_edits.panel.facts.map_usage.clone();
        let fill = self.map_edits.panel.facts.fill.clone();
        let fill_candidates = self.map_edits.panel.facts.fill_candidates.clone();
        let count = self.world_notes.edit_selection_len();
        let models = self.map_edit_set_ids().len();
        let camera = if self.live_session.is_some() && !self.free_camera_active() {
            Some(self.map_edits.fly.refused.clone().unwrap_or_else(|| {
                "Own view: the free camera is off (alive, on foot, no duel)".to_owned()
            }))
        } else {
            None
        };
        let flying = self.map_edit_flying();
        let recent = self.map_edit_recent().clone();
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
        let weather = document.and_then(|document| {
            let kept = document.weather(server.as_deref())?;
            let own = document
                .active(server.as_deref())
                .any(|edit| edit.server.is_some() && matches!(edit.kind, Kind::Weather { .. }));
            Some((kept.to_vec(), own))
        });
        let map_weather = self.weather.map_commands().to_vec();
        let lights =
            super::panel_lights::light_rows(self.map_edits.document.as_ref(), server.as_deref());
        let lamps = if self.map_edits.panel.tab == Tab::Lights {
            let view = self.world_notes.view().map(|(eye, _)| eye);
            self.map_edit_lamp_rows(view)
        } else {
            Vec::new()
        };
        let realtime = self.context.sun_shadows.day.enabled;
        let place_things = self.map_edits.panel.tab == Tab::Place && self.map_edits.panel.things;
        let things = if place_things {
            self.map_edit_thing_rows()
        } else {
            Vec::new()
        };
        let toy_kinds = self.toy_master.kinds().to_vec();
        let on_server = self.map_edits_server().is_some();
        let creator = crate::player_identity::is_creator();
        let brushes = self.console.as_ref().and_then(|console| {
            BrushPreset::of(
                console.integer_cvar(crate::brush_view::CVAR).unwrap_or(0),
                console
                    .text_value(crate::brush_view::TYPES_CVAR)
                    .unwrap_or(""),
            )
        });
        let forced = self.context.weather.force() != 0;
        let align = self.map_edit_selected_align();
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
            count,
            models,
            fill,
            fill_candidates,
            nearby,
            map_usage,
            camera,
            flying,
            exit_key,
            placed,
            rows,
            scope: state.editor.scope(),
            server,
            grabbing: state.editor.grabbing(),
            select_key,
            can_undo,
            can_redo,
            hub_problem,
            weather,
            map_weather,
            forced,
            lights,
            lamps,
            realtime,
            things,
            toy_kinds,
            on_server,
            creator,
            brushes,
            align,
            on_surface: state.editor.on_surface(),
        };
        let vfs = self.vfs.clone();
        let panel = &mut self.map_edits.panel;
        panel.facts = facts;
        if std::mem::take(&mut panel.choose_newest_light)
            && let Some(newest) =
                (0..panel.facts.lights.len()).max_by_key(|&i| panel.facts.lights[i].id)
        {
            panel.light_row = newest;
            panel.light_choice = super::panel_lights::LightChoice::Light;
        }
        panel.settle_light_lists();
        Panel::settle(
            &mut panel.thing_row,
            &mut panel.thing_first,
            panel.facts.things.len(),
            THING_ROWS,
        );
        panel.set_recent(&recent);
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
                let placed = self.map_edits.placed_edit(number, lump);
                let picked = if placed.is_some() {
                    Picked::Placed
                } else if self.static_models.is_prop(number as u32) {
                    Picked::Prop
                } else {
                    Picked::Entity
                };
                // The page's summary: the edit and where it stands, or the entity.
                let class = match placed {
                    Some(id) => {
                        let at = self
                            .map_edits
                            .placements(&[id])
                            .first()
                            .map(|placed| placed.origin.round().to_array());
                        match at {
                            Some([x, y, z]) => format!("edit {id}, at {x} {y} {z}"),
                            None => format!("edit {id}"),
                        }
                    }
                    None => format!("{classname}, entity {number}"),
                };
                Selected {
                    picked,
                    name: model,
                    detail: match &shader {
                        Some(shader) => format!("{shader}, {class}"),
                        None => class,
                    },
                    shader,
                }
            }
            EditTarget::Surface {
                index,
                shader,
                kind,
            } => {
                let soup = self
                    .bsp
                    .render()
                    .surfaces()
                    .get(index)
                    .is_some_and(|surface| surface.kind == sjk_bsp::SurfaceKind::TriangleSoup);
                // The page's summary: the remap it is drawn with, the whole model's size.
                let server = self.map_edits_server();
                let remapped = self.map_edits.document.as_ref().and_then(|document| {
                    document
                        .active(server.as_deref())
                        .find_map(|edit| match &edit.kind {
                            Kind::Remap { from, to } if from.eq_ignore_ascii_case(&shader) => {
                                Some(to.clone())
                            }
                            _ => None,
                        })
                });
                let mut detail = format!("surface {index}, {kind}");
                if soup {
                    let group = self.map_edit_hide_group(index).len();
                    detail.push_str(&format!(", whole model {group} surfaces"));
                }
                if let Some(to) = remapped {
                    detail.push_str(&format!("; drawn as {to}"));
                }
                Selected {
                    picked: Picked::Surface { soup },
                    name: shader.clone(),
                    detail,
                    shader: Some(shader),
                }
            }
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

    /// The world shots: where the gizmo stands on the Lights page's chosen light or lamp.
    pub(crate) fn shot_light_target(&self) -> Option<glam::Vec3> {
        self.map_edit_light_target()
            .map(|(_, placement)| placement.origin)
    }

    /// The world shots: a Shift+click at `point` (physical pixels).
    pub(crate) fn shot_map_edit_toggle(&mut self, point: [f32; 2]) -> crate::world_notes::Toggled {
        let toggled = self.map_edit_toggle_pick(Some(point));
        self.map_edit_toggled(toggled);
        toggled
    }

    /// The world shots: type `query` in the Selected page's shader search.
    pub(crate) fn shot_map_edit_remap_search(&mut self, query: &str) {
        let panel = &mut self.map_edits.panel;
        panel.remap = query.to_owned();
        panel.refilter_shaders();
        panel.shader_row = 2;
        panel.focus = REMAP_FIELD;
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

    fn words(commands: &[&str]) -> Vec<String> {
        commands
            .iter()
            .map(|command| (*command).to_owned())
            .collect()
    }

    /// The Weather page's buttons: a preset keeps the fog, Fog and Wind switch theirs,
    /// and taking everything away gives the map its own weather back.
    #[test]
    fn weather_buttons_build_the_commands_kept() {
        let storm = weather_choice(&[], WeatherChoice::Preset(3));
        assert_eq!(storm, words(&["heavyrain", "gustingwind"]));
        assert_eq!(weather_preset(&storm), Some(3));
        let foggy = weather_choice(&storm, WeatherChoice::Fog);
        assert_eq!(foggy, words(&["heavyrain", "gustingwind", "mist"]));
        assert_eq!(weather_fog_and_wind(&foggy), (true, true));
        let snow = weather_choice(&foggy, WeatherChoice::Preset(4));
        assert_eq!(snow, words(&["snow", "wind", "mist"]), "the fog stays");
        assert_eq!(weather_preset(&snow), Some(4));
        let calm = weather_choice(&snow, WeatherChoice::Wind);
        assert_eq!(calm, words(&["snow", "mist"]));
        assert_eq!(
            weather_choice(&calm, WeatherChoice::Wind),
            words(&["snow", "mist", "wind"])
        );
        let clear = weather_choice(&words(&["mist"]), WeatherChoice::Fog);
        assert!(clear.is_empty(), "nothing left: the map's own");
        assert_eq!(
            weather_preset(&words(&["constantwind ( 0 9 0 )", "fog"])),
            None
        );
        assert_eq!(weather_preset(&words(&["lightrain", "light_fog"])), Some(1));
        assert!(weather_choice(&storm, WeatherChoice::Own).is_empty());
    }

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
        // floor2 is suggested as its family, then the folder's three
        assert_eq!(panel.shader_shown.len(), 4);
        assert_eq!(panel.candidate_rows, 1);
        assert_eq!(panel.shown_shader(0), Some("textures/yavin/floor2"));
        assert_eq!(
            panel.suggestion(0),
            Some(super::super::suggest::Reason::Family)
        );
        assert_eq!(
            panel.chosen_shader(),
            Some("textures/yavin/floor2"),
            "the list opens on its suggestions"
        );
        // Without any, its own shader is chosen among its neighbours.
        panel.set_shaders(&[
            "textures/korriban/wall",
            "textures/yavin/floor",
            "textures/yavin/stone",
        ]);
        assert_eq!(panel.candidate_rows, 0);
        assert_eq!(panel.chosen_shader(), Some("textures/yavin/floor"));
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
    fn the_recent_rows_come_first_in_both_lists_and_follow_the_search() {
        let mut panel = Panel::default();
        panel.set_models(&["models/a.md3", "models/b.md3", "models/c.md3"]);
        panel.set_shaders(&["textures/x/floor", "textures/x/wall", "textures/y/wall"]);
        let recent = super::super::recent::Recent {
            remember: true,
            shaders: vec!["TEXTURES/Y/WALL.jpg".to_owned(), "textures/gone".to_owned()],
            models: vec!["models/c.md3".to_owned()],
        };
        panel.set_recent(&recent);
        assert_eq!(panel.recent_model_rows, 1);
        assert_eq!(panel.shown_model(0), Some("models/c.md3"));
        assert_eq!(panel.shown.len(), 4, "the recent one, then the whole list");
        // A shader the game data lacks is left out; one of another case is found.
        assert_eq!(panel.recent_shader_rows, 1);
        assert_eq!(panel.shown_shader(0), Some("textures/y/wall"));
        panel.remap = "x/".to_owned();
        panel.refilter_shaders();
        assert_eq!(
            panel.recent_shader_rows, 0,
            "the search keeps no recent one"
        );
        panel.search = "b".to_owned();
        panel.refilter();
        assert_eq!((panel.recent_model_rows, panel.shown.len()), (0, 1));
        // Off: no Recent rows.
        panel.set_recent(&super::super::recent::Recent {
            remember: false,
            ..Default::default()
        });
        panel.search.clear();
        panel.refilter();
        assert!(!panel.remember);
        assert_eq!(panel.recent_model_rows, 0);
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
        assert_eq!(
            panel.activate(LINK_SURFACE),
            Some(Action::Link(Grow::Surface))
        );
        assert_eq!(panel.activate(FILL), Some(Action::Fill));
        assert_eq!(panel.activate(REMEMBER), Some(Action::Remember));
        // A model row chooses, then a second click places.
        panel.set_models(&["models/a.md3", "models/b.md3"]);
        assert_eq!(panel.activate(MODEL_ROW + 1), None);
        assert_eq!(panel.chosen_model(), Some("models/b.md3"));
        assert_eq!(panel.activate(MODEL_ROW + 1), Some(Action::Place));
    }

    /// Two quick clicks on one row of the edits, things, lights or lamps fly to it; a
    /// slow second click, another row or a button does not.
    #[test]
    fn a_double_click_on_a_row_flies_to_it() {
        let mut panel = Panel::default();
        let now = std::time::Instant::now();
        let soon = now + DOUBLE_CLICK / 2;
        assert_eq!(panel.double_click(EDIT_ROW + 2, now), None);
        assert_eq!(
            panel.double_click(EDIT_ROW + 2, soon),
            Some(FlyTarget::Edit)
        );
        // a third click starts over
        assert_eq!(panel.double_click(EDIT_ROW + 2, soon), None);
        assert_eq!(panel.double_click(LIGHT_ROW, now), None);
        assert_eq!(panel.double_click(LIGHT_ROW + 1, soon), None, "another row");
        let late = soon + DOUBLE_CLICK * 2;
        assert_eq!(panel.double_click(LIGHT_ROW + 1, late), None, "too slow");
        assert_eq!(panel.double_click(THING_ROW, now), None);
        assert_eq!(panel.double_click(THING_ROW, soon), Some(FlyTarget::Thing));
        assert_eq!(panel.double_click(LAMP_ROW, now), None);
        assert_eq!(panel.double_click(LAMP_ROW, soon), Some(FlyTarget::Lamp));
        assert_eq!(panel.double_click(HIDE, now), None);
        assert_eq!(panel.double_click(HIDE, soon), None, "a button is no row");
    }
}
