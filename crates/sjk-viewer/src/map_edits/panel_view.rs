//! The map editor's panel in the SJK UI's look (`docs/sjk-ui.md`, Map editor): Camera
//! control's column down the window's right edge over its dark fade, the scene left clear
//! (a click there selects), and the strip at the top of the screen while the panel is
//! hidden to aim. The state, tokens, keys and pointer are [`super::panel`]'s.
//!
//! Positions are pixels of a 1080-line frame whose right edge is the window's, as Camera
//! control's.

use super::document::Scope;
use super::panel::*;
use crate::menu::sjk::{Frame, color, fade_across, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};

/// The column's left edge and width.
const COLUMN_X: f32 = 1440.0;
const COLUMN_WIDTH: f32 = 416.0;
/// Where the fade starts, clear, and where it is dark under the column.
const FADE_START: f32 = 1200.0;
const FADE_DARK: f32 = 1400.0;
/// A press left of this (frame pixels) is on the scene.
const SCENE_RIGHT: f32 = 1400.0;
const TITLE_Y: f32 = 80.0;
const SUBTITLE_Y: f32 = 120.0;
const TABS_Y: f32 = 148.0;
const PAGE_Y: f32 = 226.0;
/// The foot: the scope's line, the answer, the closing buttons, the keys.
const SCOPE_Y: f32 = 800.0;
const STATUS_Y: f32 = 840.0;
const CLOSE_Y: f32 = 898.0;
const KEYS_Y: f32 = 964.0;
const KEYS_PITCH: f32 = 32.0;
const BUTTON: f32 = 40.0;
const GAP: f32 = 8.0;
const FIELD: f32 = 44.0;
const LIST_ROW: f32 = 34.0;
/// Characters of Exo 2 at 16 the column holds on a line.
const LINE_CHARS: usize = 44;

/// The SJK UI's 1080-line frame with its right edge on the window's.
fn frame(viewport: [f32; 2]) -> Frame {
    let s = crate::menu::sjk::scale(viewport);
    Frame {
        s,
        origin: [viewport[0] - 1920.0 * s, (viewport[1] - 1080.0 * s) * 0.5],
    }
}

fn push(ui: &mut MenuCanvas, command: DrawCommand) {
    let _ = ui.draw_list_mut().push(command);
}

/// Draw `panel` into `ui`, and keep where its scene and lists lie for the pointer.
pub(super) fn build(panel: &mut Panel, ui: &mut MenuCanvas, viewport: [f32; 2]) {
    let frame = frame(viewport);
    ui.begin_transparent(viewport);
    scrim(ui, &frame, viewport);
    header(ui, &frame, panel);
    panel.model_list = None;
    panel.edit_list = None;
    panel.shader_list = None;
    match panel.tab {
        Tab::Selected => selected(ui, &frame, panel),
        Tab::Place => place(ui, &frame, panel),
        Tab::Edits => edits(ui, &frame, panel),
    }
    foot(ui, &frame, panel);
    keys(ui, &frame, panel);
    panel.scene_right = frame.point(SCENE_RIGHT, 0.0)[0];
    ui.finish(panel.focus);
}

/// The fade from clear to dark under the column, full height.
fn scrim(ui: &mut MenuCanvas, frame: &Frame, viewport: [f32; 2]) {
    let [width, height] = viewport;
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let space = |alpha| color::alpha(color::SPACE, alpha);
    let (start, dark) = (x(FADE_START).max(0.0), x(FADE_DARK));
    if dark > start {
        fade_across(
            ui,
            Rect::new(start, 0.0, dark - start, height),
            space(0.0),
            space(0.74),
        );
    }
    fade_across(
        ui,
        Rect::new(dark, 0.0, width - dark, height),
        space(0.74),
        space(0.88),
    );
}

/// Body text on a line of the column from `y`.
fn line(ui: &mut MenuCanvas, frame: &Frame, y: f32, value: &str, colour: Color) {
    text(
        ui,
        TextFamily::Body,
        format_args!("{value}"),
        frame.rect(COLUMN_X, y, COLUMN_WIDTH, 24.0),
        16.0 * frame.s,
        colour,
        FontWeight::Regular,
        TextAlign::Start,
    );
}

/// The title, the map's state, and the pages as tabs.
fn header(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel) {
    let s = frame.s;
    let facts = &panel.facts;
    text(
        ui,
        TextFamily::Display,
        format_args!("Map editor"),
        frame.rect(COLUMN_X, TITLE_Y - 26.0, COLUMN_WIDTH, 52.0),
        42.0 * s,
        color::TEXT,
        FontWeight::Semibold,
        TextAlign::Start,
    );
    let map = if facts.map.is_empty() {
        "This map"
    } else {
        facts.map.as_str()
    };
    let rect = frame.rect(COLUMN_X, SUBTITLE_Y - 12.0, COLUMN_WIDTH, 24.0);
    let (size, colour) = (17.0 * s, color::MUTED);
    let body = |ui: &mut MenuCanvas, value: std::fmt::Arguments<'_>| {
        text(
            ui,
            TextFamily::Body,
            value,
            rect,
            size,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    };
    match (facts.ready, facts.edit_count, facts.unpublished) {
        (false, _, _) => body(ui, format_args!("{map}: reading the map...")),
        (true, 0, _) => body(ui, format_args!("{map}: no edits yet")),
        (true, count, true) => body(
            ui,
            format_args!(
                "{map}: {count} edit{}, not published",
                if count == 1 { "" } else { "s" }
            ),
        ),
        (true, count, false) => body(
            ui,
            format_args!(
                "{map}: {count} edit{}, published (revision {})",
                if count == 1 { "" } else { "s" },
                facts.revision
            ),
        ),
    }
    let mut x = COLUMN_X;
    for (tab, label) in [
        (Tab::Selected, "Selected"),
        (Tab::Place, "Place"),
        (Tab::Edits, "Edits"),
    ] {
        let token = tab.token();
        // Rajdhani at 26 is about 12 pixels a character.
        let width = 12.0 * label.len() as f32 + 4.0;
        let current = tab == panel.tab;
        let focused = panel.focus == token;
        if focused {
            kit::band(ui, frame, [x - 16.0, TABS_Y - 6.0, width + 32.0, 50.0]);
        }
        let lit = focused || ui.token_hovered(token);
        text(
            ui,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(x, TABS_Y, width + 20.0, 34.0),
            26.0 * s,
            match (current, lit) {
                (true, _) => color::GOLD_BRIGHT,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        if current {
            push(
                ui,
                DrawCommand::RoundedRect {
                    rect: frame.rect(x, TABS_Y + 38.0, width, 3.0),
                    radius: 1.5 * s,
                    color: color::GOLD_BRIGHT,
                },
            );
        }
        ui.hit_region(token, frame.rect(x - 8.0, TABS_Y - 4.0, width + 16.0, 46.0));
        x += width + 40.0;
    }
}

/// A row of buttons across the column from `y`: (token, label, enabled).
fn buttons(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel, y: f32, row: &[(u16, &str, bool)]) {
    let count = row.len() as f32;
    let width = (COLUMN_WIDTH - GAP * (count - 1.0)) / count;
    for (index, (token, label, enabled)) in row.iter().enumerate() {
        kit::button(
            ui,
            frame,
            [COLUMN_X + index as f32 * (width + GAP), y, width, BUTTON],
            label,
            false,
            *enabled,
            panel.focus == *token,
            *token,
        );
    }
}

/// The Selected page: what the key or a click selected, what can be done to it, and its
/// shader's remap: a search over the shaders, opened on its own shader's folder.
fn selected(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel) {
    let s = frame.s;
    let mut y = PAGE_Y;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Selected");
    y += 18.0;
    let Some(selection) = &panel.facts.selection else {
        text(
            ui,
            TextFamily::Display,
            format_args!("Nothing selected"),
            frame.rect(COLUMN_X, y, COLUMN_WIDTH, 34.0),
            26.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let key = if panel.facts.select_key.is_empty() {
            "the world-note key"
        } else {
            panel.facts.select_key.as_str()
        };
        line(
            ui,
            frame,
            y + 44.0,
            "Click a prop or a surface in the scene,",
            color::MUTED,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!("or aim at it and press {key}."),
            frame.rect(COLUMN_X, y + 70.0, COLUMN_WIDTH, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        return;
    };
    let kind = match selection.picked {
        Picked::Prop => "Map prop",
        Picked::Placed => "Placed model",
        Picked::Surface { .. } => "Surface",
        Picked::Entity => "Game entity",
        Picked::Face => "Hidden face",
        Picked::Sky => "Sky",
        Picked::Other => "Not editable",
    };
    text(
        ui,
        TextFamily::Display,
        format_args!("{kind}"),
        frame.rect(COLUMN_X, y, COLUMN_WIDTH, 34.0),
        26.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    line(ui, frame, y + 40.0, &selection.name, color::TEXT);
    for (index, part) in wrap(&selection.detail, LINE_CHARS).take(2).enumerate() {
        line(
            ui,
            frame,
            y + 64.0 + index as f32 * 24.0,
            part,
            color::MUTED,
        );
    }
    y += 124.0;
    match selection.picked {
        Picked::Prop => buttons(
            ui,
            frame,
            panel,
            y,
            &[(HIDE, "Hide", true), (COPY, "Place a copy", true)],
        ),
        Picked::Placed => {
            buttons(
                ui,
                frame,
                panel,
                y,
                &[(HIDE, "Remove", true), (COPY, "Place a copy", true)],
            );
        }
        Picked::Surface { soup } => buttons(
            ui,
            frame,
            panel,
            y,
            &[(HIDE, "Hide", true), (HIDE_GROUP, "Hide whole model", soup)],
        ),
        Picked::Entity => line(
            ui,
            frame,
            y + 8.0,
            "The game needs it: it can be remapped, not hidden.",
            color::MUTED,
        ),
        Picked::Face => line(
            ui,
            frame,
            y + 8.0,
            "The map draws nothing here: pick a shader to draw it.",
            color::MUTED,
        ),
        Picked::Sky => line(
            ui,
            frame,
            y + 8.0,
            "Pick a sky to swap it, or a texture to cover this side.",
            color::MUTED,
        ),
        Picked::Other => {}
    }
    if selection.shader.is_none() {
        return;
    }
    y += BUTTON + 34.0;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Draw another shader");
    y += 18.0;
    kit::search(
        ui,
        frame,
        [COLUMN_X, y, COLUMN_WIDTH, FIELD],
        &panel.remap,
        panel.focus == REMAP_FIELD,
        "Search the shaders",
        (!panel.remap.is_empty()).then_some(panel.shader_shown.len()),
        REMAP_FIELD,
    );
    y += FIELD + 8.0;
    let height = LIST_ROW * SHADER_ROWS as f32 + 16.0;
    list_ground(ui, frame, [COLUMN_X, y, COLUMN_WIDTH, height]);
    panel.shader_list = Some(frame.rect(COLUMN_X, y, COLUMN_WIDTH, height));
    if panel.shader_shown.is_empty() {
        let typed = panel.remap.trim();
        let line_text = if typed.is_empty() || typed.ends_with('/') {
            "No shader matches"
        } else {
            "None known: Remap tries the name as typed"
        };
        line(ui, frame, y + 16.0, line_text, color::MUTED);
    }
    let own = without_image_extension(selection.shader.as_deref().unwrap_or(""));
    for slot in 0..SHADER_ROWS {
        let index = panel.shader_first + slot;
        let Some(shader) = panel.shown_shader(index) else {
            break;
        };
        // The name without its textures/ or models/ prefix; its own shader quiet.
        let shown = shader
            .strip_prefix("textures/")
            .or_else(|| shader.strip_prefix("models/"))
            .unwrap_or(shader);
        let token = SHADER_ROW + slot as u16;
        list_row(
            ui,
            frame,
            y + 8.0 + slot as f32 * LIST_ROW,
            token,
            shown,
            index == panel.shader_row,
            panel.focus == token,
            shader.eq_ignore_ascii_case(own),
        );
    }
    y += height + 8.0;
    if selection.picked == Picked::Sky {
        let sky = panel.chosen_is_sky();
        buttons(
            ui,
            frame,
            panel,
            y,
            &[
                (
                    REMAP,
                    if sky { "Swap sky" } else { "Cover side" },
                    panel.has_remap_target(),
                ),
                (SKY_ALL, "Swap all skies", sky),
                (UNREMAP, "Take back", true),
            ],
        );
        return;
    }
    buttons(
        ui,
        frame,
        panel,
        y,
        &if selection.picked == Picked::Face {
            [
                (REMAP, "Draw", panel.has_remap_target()),
                (UNREMAP, "Draw nothing", true),
            ]
        } else {
            [
                (REMAP, "Remap", panel.has_remap_target()),
                (UNREMAP, "Remove remap", true),
            ]
        },
    );
}

/// The rows of a list over `rect` (frame pixels), from the list's `first`: each row's
/// text, whether it is chosen, and whether it is muted.
fn list_ground(ui: &mut MenuCanvas, frame: &Frame, rect: [f32; 4]) {
    let [x, y, width, height] = rect;
    let s = frame.s;
    push(
        ui,
        DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 12.0 * s,
            color: color::alpha(color::SPACE, 0.6),
        },
    );
    push(
        ui,
        DrawCommand::Border {
            rect: frame.rect(x, y, width, height),
            radius: 12.0 * s,
            width: 1.5 * s,
            color: color::alpha(color::HOLO, 0.35),
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn list_row(
    ui: &mut MenuCanvas,
    frame: &Frame,
    top: f32,
    token: u16,
    value: &str,
    chosen: bool,
    focused: bool,
    muted: bool,
) {
    let s = frame.s;
    let hovered = ui.token_hovered(token);
    if chosen || hovered || focused {
        push(
            ui,
            DrawCommand::RoundedRect {
                rect: frame.rect(COLUMN_X + 6.0, top, COLUMN_WIDTH - 12.0, LIST_ROW),
                radius: 8.0 * s,
                color: color::alpha(color::HOLO, if focused { 0.2 } else { 0.1 }),
            },
        );
    }
    if chosen {
        push(
            ui,
            DrawCommand::RoundedRect {
                rect: frame.rect(COLUMN_X + 6.0, top + 6.0, 3.0, LIST_ROW - 12.0),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            },
        );
    }
    text(
        ui,
        TextFamily::Body,
        format_args!("{value}"),
        frame.rect(
            COLUMN_X + 18.0,
            top + (LIST_ROW - 22.0) * 0.5,
            COLUMN_WIDTH - 32.0,
            22.0,
        ),
        15.0 * s,
        match (chosen, muted) {
            (true, _) => color::GOLD_BRIGHT,
            (false, true) => color::QUIET,
            (false, false) => color::TEXT,
        },
        FontWeight::Regular,
        TextAlign::Start,
    );
    ui.hit_region(
        token,
        frame.rect(COLUMN_X + 6.0, top, COLUMN_WIDTH - 12.0, LIST_ROW),
    );
}

/// The Place page: the game data's models, a search narrowing them, placing the chosen
/// one, and the placed model's controls.
fn place(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel) {
    let mut y = PAGE_Y;
    let searching = panel.focus == SEARCH;
    kit::search(
        ui,
        frame,
        [COLUMN_X, y, COLUMN_WIDTH, FIELD],
        &panel.search,
        searching,
        "Search the models",
        (!panel.search.is_empty()).then_some(panel.shown.len()),
        SEARCH,
    );
    y += FIELD + 12.0;
    let height = LIST_ROW * MODEL_ROWS as f32 + 16.0;
    list_ground(ui, frame, [COLUMN_X, y, COLUMN_WIDTH, height]);
    panel.model_list = Some(frame.rect(COLUMN_X, y, COLUMN_WIDTH, height));
    if panel.shown.is_empty() {
        line(ui, frame, y + 16.0, "No model matches", color::MUTED);
    }
    for slot in 0..MODEL_ROWS {
        let index = panel.model_first + slot;
        let Some(model) = panel.shown_model(index) else {
            break;
        };
        // The path without its models/ prefix.
        let shown = model.get(7..).unwrap_or(model);
        let token = MODEL_ROW + slot as u16;
        list_row(
            ui,
            frame,
            y + 8.0 + slot as f32 * LIST_ROW,
            token,
            shown,
            index == panel.model_row,
            panel.focus == token,
            false,
        );
    }
    y += height + 12.0;
    // SJK's creator also places mystery boxes here (`crate::mystery_box`), with the scope
    // below; the page has no room for a section of their own.
    if crate::player_identity::is_creator() {
        buttons(
            ui,
            frame,
            panel,
            y,
            &[
                (PLACE, "Place model", panel.chosen_model().is_some()),
                (BOX_PLACE, "Mystery box", true),
                (BOX_REMOVE, "Remove box", true),
            ],
        );
    } else {
        kit::button(
            ui,
            frame,
            [COLUMN_X, y, COLUMN_WIDTH, BUTTON],
            "Place where you aimed",
            false,
            panel.chosen_model().is_some(),
            panel.focus == PLACE,
            PLACE,
        );
    }
    y += BUTTON + 30.0;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Placed model");
    y += 14.0;
    let placed = panel.facts.placed.is_some();
    match &panel.facts.placed {
        Some(description) => line(
            ui,
            frame,
            y,
            wrap(description, LINE_CHARS).next().unwrap_or(""),
            color::MUTED,
        ),
        None => line(
            ui,
            frame,
            y,
            "Place one, or select one, to move it",
            color::QUIET,
        ),
    }
    y += 32.0;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (TURN_LEFT, "Turn left", placed),
            (TURN_RIGHT, "Turn right", placed),
            (UP, "Up", placed),
            (DOWN, "Down", placed),
        ],
    );
    y += BUTTON + GAP;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (MOVE_LEFT, "Left", placed),
            (MOVE_RIGHT, "Right", placed),
            (FARTHER, "Away", placed),
            (NEARER, "Closer", placed),
        ],
    );
    y += BUTTON + GAP;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (SMALLER, "Smaller", placed),
            (BIGGER, "Bigger", placed),
            (GRAB, "Grab", placed),
            (REMOVE, "Remove", placed),
        ],
    );
}

/// The Edits page: the map's edits, newest first, each with its scope; delete the chosen
/// one, or undo the newest.
fn edits(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel) {
    let mut y = PAGE_Y;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "This map's edits");
    y += 16.0;
    let height = LIST_ROW * EDIT_ROWS as f32 + 16.0;
    list_ground(ui, frame, [COLUMN_X, y, COLUMN_WIDTH, height]);
    panel.edit_list = Some(frame.rect(COLUMN_X, y, COLUMN_WIDTH, height));
    if panel.facts.rows.is_empty() {
        line(ui, frame, y + 16.0, "No edits yet", color::MUTED);
    }
    for slot in 0..EDIT_ROWS {
        let index = panel.edit_first + slot;
        let Some(row) = panel.facts.rows.get(index) else {
            break;
        };
        let token = EDIT_ROW + slot as u16;
        list_row(
            ui,
            frame,
            y + 8.0 + slot as f32 * LIST_ROW,
            token,
            &row.text,
            index == panel.edit_row,
            panel.focus == token,
            !row.here,
        );
    }
    y += height + 12.0;
    let any = !panel.facts.rows.is_empty();
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (DELETE, "Delete", any),
            (UNDO, "Undo", panel.facts.can_undo),
            (REDO, "Redo", panel.facts.can_redo),
        ],
    );
}

/// Where new edits apply, the last answer, then Publish, Aim and Editor off.
fn foot(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel) {
    let s = frame.s;
    let facts = &panel.facts;
    let focused = matches!(panel.focus, SCOPE_ALL | SCOPE_SERVER);
    text(
        ui,
        TextFamily::Body,
        format_args!("New edits for"),
        frame.rect(COLUMN_X, SCOPE_Y - 14.0, 160.0, 28.0),
        19.0 * s,
        if focused {
            color::TEXT
        } else {
            color::alpha(color::TEXT, 0.88)
        },
        FontWeight::Regular,
        TextAlign::Start,
    );
    kit::segments(
        ui,
        frame,
        COLUMN_X + COLUMN_WIDTH,
        SCOPE_Y,
        &["All servers", "This server"],
        usize::from(facts.scope == Scope::ThisServer),
        focused,
        SCOPE_ALL,
    );
    let status = if !panel.status.is_empty() {
        Some((
            panel.status.as_str(),
            if panel.status_error {
                color::EMBER
            } else {
                color::TEXT
            },
        ))
    } else if let Some(problem) = &facts.hub_problem {
        Some((problem.as_str(), color::EMBER))
    } else if facts.scope == Scope::ThisServer {
        Some((
            facts.server.as_deref().unwrap_or("Not on a server"),
            color::MUTED,
        ))
    } else {
        None
    };
    if let Some((status, colour)) = status {
        for (index, part) in wrap(status, LINE_CHARS).take(2).enumerate() {
            line(ui, frame, STATUS_Y + index as f32 * 22.0, part, colour);
        }
    }
    let width = (COLUMN_WIDTH - GAP * 2.0) / 3.0;
    kit::button(
        ui,
        frame,
        [COLUMN_X, CLOSE_Y, width, 44.0],
        "Publish",
        true,
        facts.unpublished,
        panel.focus == PUBLISH,
        PUBLISH,
    );
    kit::button(
        ui,
        frame,
        [COLUMN_X + width + GAP, CLOSE_Y, width, 44.0],
        "Aim",
        false,
        true,
        panel.focus == AIM,
        AIM,
    );
    kit::button(
        ui,
        frame,
        [COLUMN_X + (width + GAP) * 2.0, CLOSE_Y, width, 44.0],
        "Editor off",
        false,
        true,
        panel.focus == EDITOR_OFF,
        EDITOR_OFF,
    );
}

/// One key hint: its caps and what they do.
type Hint<'a> = (&'a [&'a str], &'a str);

/// Hints right-aligned on the line whose top is `y` (frame pixels), ending at `right`.
fn hint_line(ui: &mut MenuCanvas, frame: &Frame, right: f32, y: f32, hints: &[Hint<'_>]) {
    let s = frame.s;
    let gap = 20.0 * s;
    let total = hints
        .iter()
        .map(|(caps, action)| key_hint_width(caps, action, s))
        .sum::<f32>()
        + gap * hints.len().saturating_sub(1) as f32;
    let [right, y] = frame.point(right, y);
    let mut x = right - total;
    for (caps, action) in hints {
        x = key_hint(ui, caps, action, x, y, s) + gap;
    }
}

/// The keys of what has the keyboard, two lines at the column's foot.
fn keys(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel) {
    let right = COLUMN_X + COLUMN_WIDTH;
    let typing = matches!(panel.focus, SEARCH | REMAP_FIELD);
    let first: &[Hint<'_>] = if typing {
        &[
            (&["Up", "Down"], "choose"),
            (
                &["Enter"],
                if panel.focus == SEARCH {
                    "place"
                } else {
                    "remap"
                },
            ),
        ]
    } else {
        &[
            (&["Enter"], "act"),
            (&["Ctrl", "Z"], "undo"),
            (&["Ctrl", "Y"], "redo"),
        ]
    };
    hint_line(ui, frame, right, KEYS_Y, first);
    hint_line(ui, frame, right, KEYS_Y + KEYS_PITCH, second_keys(panel));
}

/// The second line of keys: the gizmo's while it transforms ([`super::gizmo`]).
fn second_keys(panel: &Panel) -> &'static [Hint<'static>] {
    if panel.transforming {
        &[
            (&["X", "Y", "Z"], "axis"),
            (&["Enter"], "done"),
            (&["Esc"], "cancel"),
        ]
    } else if panel.tab == Tab::Place {
        &[
            (&["Shift"], "fine"),
            (&["G", "R", "T"], "gizmo"),
            (&["Esc"], "aim"),
        ]
    } else {
        &[
            (&["Click"], "selects"),
            (&["G", "R", "T"], "gizmo"),
            (&["Esc"], "aim"),
        ]
    }
}

/// The strip's canvas, kept for the session.
pub(crate) struct Strip(pub(crate) MenuCanvas);

impl Default for Strip {
    fn default() -> Self {
        Self(MenuCanvas::new())
    }
}

/// The strip at the top of the screen while the editor is on and its panel hidden: what
/// it is, and the key that selects and opens the panel (or puts a grabbed model down).
pub(crate) fn build_strip(facts: &Facts, ui: &mut MenuCanvas, viewport: [f32; 2]) {
    let frame = Frame::new(viewport);
    let s = frame.s;
    ui.begin_transparent(viewport);
    let (x, y, width, height) = (660.0, 28.0, 600.0, 64.0);
    push(
        ui,
        DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 14.0 * s,
            color: color::alpha(color::SPACE, 0.78),
        },
    );
    push(
        ui,
        DrawCommand::Border {
            rect: frame.rect(x, y, width, height),
            radius: 14.0 * s,
            width: 1.5 * s,
            color: color::alpha(color::HOLO, 0.4),
        },
    );
    text(
        ui,
        TextFamily::Display,
        format_args!("Map editor"),
        frame.rect(x + 20.0, y + 8.0, 200.0, 24.0),
        20.0 * s,
        color::GOLD_BRIGHT,
        FontWeight::Semibold,
        TextAlign::Start,
    );
    text(
        ui,
        TextFamily::Body,
        format_args!(
            "{}",
            if facts.grabbing {
                "The placed model follows your aim"
            } else {
                "Aim at a prop, a surface or a placed model"
            }
        ),
        frame.rect(x + 20.0, y + 34.0, width - 40.0, 22.0),
        15.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::Start,
    );
    let key = if facts.select_key.is_empty() {
        "World note"
    } else {
        facts.select_key.as_str()
    };
    let action = if facts.grabbing {
        "put it down"
    } else {
        "edit"
    };
    let caps = [key];
    let right = frame.point(x + width - 20.0, y + 8.0);
    let start = right[0] - key_hint_width(&caps, action, s);
    key_hint(ui, &caps, action, start, right[1], s);
    if facts.grabbing {
        let (caps, action) = (["Wheel"], "turn");
        let below = frame.point(x + width - 20.0, y + 34.0);
        let start = below[0] - key_hint_width(&caps, action, s);
        key_hint(ui, &caps, action, start, below[1], s);
    }
    ui.finish(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::{InputEvent, PointerButton, Vec2};

    const VIEWPORTS: [[f32; 2]; 4] = [
        [1_920.0, 1_080.0],
        [3_840.0, 2_160.0],
        [2_560.0, 1_080.0],
        [1_024.0, 768.0],
    ];

    fn panel(tab: Tab, selection: Option<Picked>) -> Panel {
        let mut panel = Panel::default();
        panel.tab = tab;
        panel.focus = tab.token();
        panel.set_models(&[
            "models/map_objects/yavin/tree01.md3",
            "models/map_objects/yavin/tree02.md3",
            "models/map_objects/yavin/rock.md3",
            "models/items/shield.md3",
            "models/map_objects/korriban/pillar.md3",
            "models/map_objects/korriban/statue.md3",
            "models/map_objects/korriban/brazier.md3",
            "models/map_objects/desert/bones.md3",
            "models/map_objects/desert/crate.md3",
            "models/map_objects/desert/wreck.md3",
        ]);
        panel.facts = Facts {
            map: "mp/duel6".to_owned(),
            ready: true,
            edit_count: 14,
            unpublished: true,
            selection: selection.map(|picked| Selected {
                picked,
                name: "models/map_objects/yavin/tree01.md3".to_owned(),
                detail:
                    "models/map_objects/yavin/tree, misc_model_static, a long detail that wraps"
                        .to_owned(),
                shader: Some("models/map_objects/yavin/tree".to_owned()),
            }),
            placed: Some(
                "3: place models/items/shield.md3 at 1 2 3, yaw 90 [all servers]".to_owned(),
            ),
            rows: (0..14)
                .map(|id| EditRow {
                    id,
                    text: format!("{id}: hide surface {id} (textures/yavin/floor) [all servers]"),
                    here: id % 3 != 0,
                })
                .collect(),
            select_key: "O".to_owned(),
            can_undo: true,
            ..Facts::default()
        };
        panel.set_shaders(&[
            "textures/yavin/floor",
            "textures/yavin/floor2",
            "textures/yavin/stone",
            "textures/yavin/wall",
            "textures/yavin/wall2",
            "textures/yavin/wall3",
            "textures/korriban/wall",
        ]);
        panel.remap = "yavin".to_owned();
        panel.refilter_shaders();
        panel.status =
            "Edit 14: surface 1043 (textures/yavin/floor) hidden, and a long answer that wraps"
                .to_owned();
        panel
    }

    fn states() -> Vec<(&'static str, Panel)> {
        vec![
            ("nothing", panel(Tab::Selected, None)),
            ("prop", panel(Tab::Selected, Some(Picked::Prop))),
            ("placed", panel(Tab::Selected, Some(Picked::Placed))),
            (
                "surface",
                panel(Tab::Selected, Some(Picked::Surface { soup: true })),
            ),
            ("entity", panel(Tab::Selected, Some(Picked::Entity))),
            ("place", panel(Tab::Place, None)),
            ("edits", panel(Tab::Edits, None)),
            ("face", panel(Tab::Selected, Some(Picked::Face))),
            ("sky", panel(Tab::Selected, Some(Picked::Sky))),
        ]
    }

    #[test]
    fn every_page_fits_its_canvas_with_its_controls_on_the_window() {
        for viewport in VIEWPORTS {
            for (state, mut panel) in states() {
                let mut canvas = MenuCanvas::new();
                build(&mut panel, &mut canvas, viewport);
                assert!(!canvas.overflowed(), "{state} {viewport:?}");
                for token in [
                    TAB_SELECTED,
                    TAB_PLACE,
                    TAB_EDITS,
                    SCOPE_ALL,
                    SCOPE_SERVER,
                    PUBLISH,
                    AIM,
                    EDITOR_OFF,
                ] {
                    assert!(
                        canvas.rect_for(token).is_some(),
                        "{state} {viewport:?}: {token}"
                    );
                }
                // A page's controls end above the foot.
                let foot = frame(viewport).point(0.0, SCOPE_Y - 22.0)[1];
                for token in canvas.widget_tokens().to_vec() {
                    let rect = canvas.rect_for(token).expect("a registered token");
                    if !(SCOPE_ALL..MODEL_ROW).contains(&token) {
                        assert!(
                            rect.bottom() <= foot + 0.01,
                            "{state} {viewport:?}: {token}"
                        );
                    }
                    assert!(
                        rect.x >= panel.scene_right
                            && rect.y >= 0.0
                            && rect.right() <= viewport[0] + 0.01
                            && rect.bottom() <= viewport[1] + 0.01,
                        "{state} {viewport:?}: token {token} at {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn each_page_offers_what_its_selection_allows() {
        let tokens = |state: &str| {
            let (_, mut panel) = states()
                .into_iter()
                .find(|(name, _)| *name == state)
                .unwrap();
            let mut canvas = MenuCanvas::new();
            build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
            canvas.widget_tokens().to_vec()
        };
        assert!(!tokens("nothing").contains(&HIDE));
        assert!(tokens("prop").contains(&HIDE) && tokens("prop").contains(&COPY));
        assert!(tokens("surface").contains(&HIDE_GROUP));
        assert!(!tokens("entity").contains(&HIDE) && tokens("entity").contains(&REMAP_FIELD));
        assert!(!tokens("face").contains(&HIDE) && tokens("face").contains(&REMAP));
        let sky = tokens("sky");
        assert!(!sky.contains(&HIDE) && sky.contains(&REMAP) && sky.contains(&SKY_ALL));
        assert!(!tokens("face").contains(&SKY_ALL) && !tokens("prop").contains(&SKY_ALL));

        let prop = tokens("prop");
        assert!(prop.contains(&(SHADER_ROW + SHADER_ROWS as u16 - 1)) && prop.contains(&REMAP));
        assert!(!prop.contains(&(SHADER_ROW + SHADER_ROWS as u16)));
        let place = tokens("place");
        assert!(place.contains(&SEARCH) && place.contains(&PLACE) && place.contains(&GRAB));
        assert!(place.contains(&MOVE_LEFT) && place.contains(&NEARER));
        assert!(place.contains(&(MODEL_ROW + MODEL_ROWS as u16 - 1)));
        assert!(!place.contains(&(MODEL_ROW + MODEL_ROWS as u16)));
        let edits = tokens("edits");
        assert!(edits.contains(&(EDIT_ROW + EDIT_ROWS as u16 - 1)) && edits.contains(&DELETE));
        assert!(edits.contains(&UNDO) && edits.contains(&REDO));
    }

    #[test]
    fn a_click_on_the_scene_is_left_of_the_column_and_the_wheel_scrolls_a_list() {
        let (_, mut panel) = states().remove(5);
        let mut canvas = MenuCanvas::new();
        build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
        assert!((panel.scene_right - 1_400.0).abs() < 0.01);
        let list = panel.model_list.expect("the model list");
        let at = Vec2::new(list.x + 20.0, list.y + 20.0);
        let wheel = InputEvent::PointerWheel {
            position: at,
            delta: Vec2::new(0.0, -1.0),
        };
        assert_eq!(panel.pointer(&mut canvas, wheel), None);
        assert_eq!(panel.model_first, 3, "ten models, six rows");
        // A click on a row chooses it; a second places it.
        build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
        let row = canvas.rect_for(MODEL_ROW).expect("a row");
        let at = Vec2::new(row.x + 10.0, row.y + row.height * 0.5);
        let button = PointerButton::Primary;
        let click = |panel: &mut Panel, canvas: &mut MenuCanvas| {
            let _ = panel.pointer(
                canvas,
                InputEvent::PointerPress {
                    position: at,
                    button,
                },
            );
            panel.pointer(
                canvas,
                InputEvent::PointerRelease {
                    position: at,
                    button,
                },
            )
        };
        assert_eq!(click(&mut panel, &mut canvas), None);
        assert_eq!(panel.model_row, 3);
        build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
        assert_eq!(click(&mut panel, &mut canvas), Some(Action::Place));
    }

    #[test]
    fn down_walks_the_page_and_its_list() {
        let (_, mut panel) = states().remove(6);
        let mut canvas = MenuCanvas::new();
        let mut visited = Vec::new();
        for _ in 0..40 {
            build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
            panel.step(&mut canvas, true);
            visited.push(panel.focus);
            if panel.focus == EDITOR_OFF {
                break;
            }
        }
        // Into the list, down its 14 edits (scrolling), then the buttons and the foot.
        assert_eq!(panel.edit_row, 13);
        assert_eq!(panel.edit_first, 2);
        assert!(visited.contains(&DELETE) && visited.contains(&PUBLISH));
        assert_eq!(visited.last(), Some(&EDITOR_OFF));
    }

    #[test]
    fn the_key_lines_fit_the_column() {
        let width = |hints: &[Hint<'_>]| {
            hints
                .iter()
                .map(|(caps, action)| key_hint_width(caps, action, 1.0))
                .sum::<f32>()
                + 20.0 * hints.len().saturating_sub(1) as f32
        };
        for tab in [Tab::Selected, Tab::Place, Tab::Edits] {
            for transforming in [false, true] {
                let mut panel = Panel::default();
                panel.tab = tab;
                panel.transforming = transforming;
                let line = width(second_keys(&panel));
                assert!(line <= COLUMN_WIDTH, "{tab:?} {transforming}: {line}");
            }
        }
    }

    #[test]
    fn the_strip_fits_every_window() {
        for viewport in VIEWPORTS {
            for grabbing in [false, true] {
                let facts = Facts {
                    grabbing,
                    select_key: "O".to_owned(),
                    ..Facts::default()
                };
                let mut canvas = MenuCanvas::new();
                build_strip(&facts, &mut canvas, viewport);
                assert!(!canvas.overflowed());
                let runs: Vec<&str> = canvas.text_runs().collect();
                assert_eq!(runs[0], "Map editor");
                assert!(runs.contains(&if grabbing { "put it down" } else { "edit" }));
                assert_eq!(
                    runs.contains(&"turn"),
                    grabbing,
                    "the wheel turns a grabbed model"
                );
            }
        }
    }
}
