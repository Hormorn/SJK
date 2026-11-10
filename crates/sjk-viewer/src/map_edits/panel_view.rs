//! The map editor's panel in the SJK UI's look (`docs/sjk-ui.md`, Map editor): Camera
//! control's column down the window's right edge over its dark fade, the scene left clear
//! (a click there selects, the right button held flies). The state, tokens, keys and
//! pointer are [`super::panel`]'s.
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
const KEYS_Y: f32 = 954.0;
const KEYS_PITCH: f32 = 30.0;
const BUTTON: f32 = 40.0;
const GAP: f32 = 8.0;
const FIELD: f32 = 44.0;
const LIST_ROW: f32 = 34.0;
/// Between two tabs: five of them fill the column.
const TAB_GAP: f32 = 18.0;
/// The tabs' Rajdhani size, and about how wide a character of it is.
const TAB_SIZE: f32 = 22.0;
const TAB_CHAR: f32 = 10.2;
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
    panel.light_list = None;
    panel.lamp_list = None;
    panel.thing_list = None;
    panel.preview_rect = None;
    panel.hover_shader = None;
    panel.pictures.begin();
    match panel.tab {
        Tab::Selected => selected(ui, &frame, panel),
        Tab::Place => place(ui, &frame, panel),
        Tab::Edits => edits(ui, &frame, panel),
        Tab::Weather => weather(ui, &frame, panel),
        Tab::Lights => lights(ui, &frame, panel),
    }
    foot(ui, &frame, panel);
    keys(ui, &frame, panel);
    panel.scene_right = frame.point(SCENE_RIGHT, 0.0)[0];
    scene_marks(ui, &frame, panel);
    ui.finish(panel.focus);
}

/// Over the scene: the box being drawn (holo, faintly filled) and the hover's tip, one
/// line on navy glass under and right of the pointer, kept on the scene.
fn scene_marks(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel) {
    let s = frame.s;
    if let Some([x0, y0, x1, y1]) = panel.scene.marquee {
        let rect = Rect::new(x0, y0, x1 - x0, y1 - y0);
        push(
            ui,
            DrawCommand::RoundedRect {
                rect,
                radius: 2.0 * s,
                color: color::alpha(color::HOLO, 0.08),
            },
        );
        push(
            ui,
            DrawCommand::Border {
                rect,
                radius: 2.0 * s,
                width: 1.5 * s,
                color: color::alpha(color::HOLO, 0.8),
            },
        );
        return;
    }
    let (Some(tip), Some([x, y])) = (panel.scene.tip.as_deref(), panel.scene.at) else {
        return;
    };
    let tip = wrap(tip, 56).next().unwrap_or(tip);
    let width = (tip.chars().count() as f32 * 6.8 + 26.0) * s;
    let height = 28.0 * s;
    let left = (x + 18.0 * s)
        .min(panel.scene_right - width - 8.0 * s)
        .max(0.0);
    // Under the pointer, or over it near the window's foot.
    let bottom = frame.point(0.0, 1080.0)[1];
    let top = if y + (22.0 + 28.0 + 8.0) * s > bottom {
        y - (14.0 + 28.0) * s
    } else {
        y + 22.0 * s
    };
    let rect = Rect::new(left, top, width, height);
    push(
        ui,
        DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * s,
            color: color::alpha(color::SPACE, 0.86),
        },
    );
    push(
        ui,
        DrawCommand::Border {
            rect,
            radius: 8.0 * s,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.35),
        },
    );
    text(
        ui,
        TextFamily::Body,
        format_args!("{tip}"),
        Rect::new(left + 12.0 * s, top + 4.0 * s, width - 16.0 * s, 20.0 * s),
        14.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
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
        (Tab::Lights, "Lights"),
        (Tab::Edits, "Edits"),
        (Tab::Weather, "Weather"),
    ] {
        let token = tab.token();
        let width = TAB_CHAR * label.len() as f32 + 4.0;
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
            frame.rect(x, TABS_Y + 3.0, width + 20.0, 30.0),
            TAB_SIZE * s,
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
        x += width + TAB_GAP;
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
        line(
            ui,
            frame,
            y + 96.0,
            "Hold the right button to fly (W A S D).",
            color::MUTED,
        );
        return;
    };
    let many = panel.facts.count > 1;
    let models = panel.facts.models;
    if many && models == panel.facts.count {
        placed_set(ui, frame, panel, y, models);
        return;
    }
    let kind = match selection.picked {
        Picked::Prop => "Map prop",
        Picked::Placed => "Placed model",
        Picked::Surface { .. } => "Surface",
        Picked::Entity => "Game entity",
        Picked::Face => "Hidden face",
        Picked::Sky => "Sky",
        Picked::Other => "Not editable",
    };
    if many {
        text(
            ui,
            TextFamily::Display,
            format_args!("{kind} and {} more", panel.facts.count - 1),
            frame.rect(COLUMN_X, y, COLUMN_WIDTH, 34.0),
            26.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
    } else {
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
    }
    line(ui, frame, y + 40.0, &selection.name, color::TEXT);
    // The fill's suggestion takes the second detail line.
    let chosen_fill = panel
        .fill_choice()
        .zip(panel.candidate_share(panel.shader_row));
    let fill = chosen_fill
        .or_else(|| {
            panel
                .facts
                .fill
                .as_ref()
                .map(|(shader, share)| (shader.as_str(), *share))
        })
        .map(|(shader, share)| {
            let shown = shader
                .strip_prefix("textures/")
                .or_else(|| shader.strip_prefix("models/"))
                .unwrap_or(shader);
            format!("Fill: {shown} ({:.0} % nearby)", share * 100.0)
        });
    // A set mixing placed models with surfaces or faces: the gizmo moves the models.
    let set = (many && models > 0).then(|| match models {
        1 => "1 placed model among them: G R T move it".to_owned(),
        n => format!("{n} placed models among them: G R T move them"),
    });
    let extras: Vec<(&str, Color)> = [
        set.as_deref().map(|set| (set, color::TEXT)),
        fill.as_deref().map(|fill| (fill, color::GOLD_BRIGHT)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let detail_lines = 2 - extras.len();
    for (index, part) in wrap(&selection.detail, LINE_CHARS)
        .take(detail_lines)
        .enumerate()
    {
        line(
            ui,
            frame,
            y + 64.0 + index as f32 * 24.0,
            part,
            color::MUTED,
        );
    }
    for (index, (extra, colour)) in extras.iter().enumerate() {
        let slot = detail_lines + index;
        line(ui, frame, y + 64.0 + slot as f32 * 24.0, extra, *colour);
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
                &[
                    (HIDE, if many { "Hide all" } else { "Remove" }, true),
                    (COPY, "Place a copy", !many),
                    (DUPLICATE, "Duplicate", true),
                ],
            );
        }
        Picked::Surface { soup } => buttons(
            ui,
            frame,
            panel,
            y,
            &[
                (HIDE, if many { "Hide all" } else { "Hide" }, true),
                (HIDE_GROUP, "Hide whole model", soup && !many),
            ],
        ),
        Picked::Entity => line(
            ui,
            frame,
            y + 8.0,
            "The game needs it: it can be remapped, not hidden.",
            color::MUTED,
        ),
        Picked::Face | Picked::Sky => buttons(
            ui,
            frame,
            panel,
            y,
            &[
                (LINK_PLANE, "Same plane", true),
                (LINK_SURFACE, "Connected", true),
                (FILL, "Fill", panel.facts.fill.is_some()),
            ],
        ),
        Picked::Other => {}
    }
    if selection.shader.is_none() {
        return;
    }
    y += BUTTON + 34.0;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Draw another shader");
    let card_top = y - 10.0;
    y += 18.0;
    let switch = 92.0;
    kit::search(
        ui,
        frame,
        [COLUMN_X, y, COLUMN_WIDTH - switch - GAP, FIELD],
        &panel.remap,
        panel.focus == REMAP_FIELD,
        "Search the shaders",
        (!panel.remap.is_empty()).then_some(panel.shader_shown.len()),
        REMAP_FIELD,
    );
    kit::button(
        ui,
        frame,
        [COLUMN_X + COLUMN_WIDTH - switch, y + 2.0, switch, BUTTON],
        if panel.tiles { "Rows" } else { "Tiles" },
        false,
        true,
        panel.focus == TILES,
        TILES,
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
    let own = without_image_extension(selection.shader.as_deref().unwrap_or("")).to_owned();
    let picked = selection.picked;
    if panel.tiles {
        shader_tiles(ui, frame, panel, y, &own);
    } else {
        shader_rows(ui, frame, panel, y, &own);
    }
    preview_card(ui, frame, panel, card_top, &own);
    y += height + 8.0;
    if picked == Picked::Sky {
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
        &if picked == Picked::Face {
            [
                (REMAP, "Draw", panel.has_remap_target()),
                (UNREMAP, "Draw nothing", true),
                (SELECT_SHADER, "Same shader", true),
            ]
        } else {
            [
                (REMAP, "Remap", panel.has_remap_target()),
                (UNREMAP, "Remove remap", true),
                (
                    SELECT_SHADER,
                    "Same shader",
                    matches!(picked, Picked::Surface { .. }),
                ),
            ]
        },
    );
}

/// The Selected page on a multiple selection of placed models only: their count, Remove
/// all, and the Place page's turns, lifts, nudges and growths acting on all of them about
/// their middle (the editor's `mapedit_turn`, `_move` and `_scale` on a set). Grab takes
/// one model at a time.
fn placed_set(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel, mut y: f32, models: usize) {
    let s = frame.s;
    text(
        ui,
        TextFamily::Display,
        format_args!("{models} models selected"),
        frame.rect(COLUMN_X, y, COLUMN_WIDTH, 34.0),
        26.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    let newest = panel
        .facts
        .selection
        .as_ref()
        .map_or("", |selection| selection.name.as_str());
    line(ui, frame, y + 40.0, newest, color::TEXT);
    line(
        ui,
        frame,
        y + 64.0,
        "G R T or these move them all together;",
        color::MUTED,
    );
    line(
        ui,
        frame,
        y + 88.0,
        "Grab takes one model at a time.",
        color::MUTED,
    );
    y += 124.0;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (HIDE, "Remove all", true),
            (DUPLICATE, "Duplicate", true),
            (GRAB, "Grab", false),
        ],
    );
    y += BUTTON + GAP;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (TURN_LEFT, "Turn left", true),
            (TURN_RIGHT, "Turn right", true),
            (UP, "Up", true),
            (DOWN, "Down", true),
        ],
    );
    y += BUTTON + GAP;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (MOVE_LEFT, "Left", true),
            (MOVE_RIGHT, "Right", true),
            (FARTHER, "Away", true),
            (NEARER, "Closer", true),
        ],
    );
    y += BUTTON + GAP;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[(SMALLER, "Smaller", true), (BIGGER, "Bigger", true)],
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
    list_row_from(ui, frame, top, token, value, [chosen, focused, muted], 0.0);
}

/// [`list_row`] with its text `indent` further right (a picture before it).
fn list_row_from(
    ui: &mut MenuCanvas,
    frame: &Frame,
    top: f32,
    token: u16,
    value: &str,
    [chosen, focused, muted]: [bool; 3],
    indent: f32,
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
            COLUMN_X + 18.0 + indent,
            top + (LIST_ROW - 22.0) * 0.5,
            COLUMN_WIDTH - 32.0 - indent,
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

/// A shader name as the lists show it: without `textures/` or `models/`.
fn short_shader(shader: &str) -> &str {
    shader
        .strip_prefix("textures/")
        .or_else(|| shader.strip_prefix("models/"))
        .unwrap_or(shader)
}

/// A picture over `rect` (frame pixels), on a dark ground: the thumbnail when held, the
/// shader's short name when it has none, a faint ground while it loads.
fn picture(
    ui: &mut MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    picture: Option<Option<sjk_ui::TextureId>>,
    name: &str,
) {
    let [x, y, width, height] = rect;
    let s = frame.s;
    push(
        ui,
        DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 4.0 * s,
            color: color::alpha(color::SPACE, 0.9),
        },
    );
    match picture {
        Some(Some(texture)) => push(
            ui,
            DrawCommand::TexturedQuad {
                rect: frame.rect(x + 1.0, y + 1.0, width - 2.0, height - 2.0),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            },
        ),
        Some(None) if height >= 40.0 => {
            let short = name.rsplit('/').next().unwrap_or(name);
            text(
                ui,
                TextFamily::Body,
                format_args!("{short}"),
                frame.rect(x + 3.0, y + height * 0.5 - 8.0, width - 6.0, 16.0),
                11.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        _ => {}
    }
}

/// The shader list as rows: each with its picture, the chosen one gold, the selection's
/// own quiet, Fill's candidates with their share and the Recent rows marked.
fn shader_rows(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel, y: f32, own: &str) {
    const SIDE: f32 = 28.0;
    for slot in 0..SHADER_ROWS {
        let index = panel.shader_first + slot;
        let thumbnail = panel.shader_picture(index);
        let Some(shader) = panel.shown_shader(index) else {
            break;
        };
        let token = SHADER_ROW + slot as u16;
        let top = y + 8.0 + slot as f32 * LIST_ROW;
        let flags = [
            index == panel.shader_row,
            panel.focus == token,
            shader.eq_ignore_ascii_case(own),
        ];
        list_row_from(
            ui,
            frame,
            top,
            token,
            short_shader(shader),
            flags,
            SIDE + 6.0,
        );
        picture(
            ui,
            frame,
            [COLUMN_X + 14.0, top + (LIST_ROW - SIDE) * 0.5, SIDE, SIDE],
            thumbnail,
            shader,
        );
        if ui.token_hovered(token) {
            panel.hover_shader = Some(index);
        }
        head_mark(ui, frame, top, index, panel);
    }
}

/// The shader list as tiles: pictures only, seven a line, the chosen one framed in gold.
fn shader_tiles(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel, y: f32, own: &str) {
    const TILE: f32 = 52.0;
    const TILE_GAP: f32 = 6.0;
    let s = frame.s;
    for slot in 0..SHADER_TILES {
        let index = panel.shader_first + slot;
        let thumbnail = panel.shader_picture(index);
        let reason = panel.suggestion(index);
        let recent = (panel.candidate_rows..panel.candidate_rows + panel.recent_shader_rows)
            .contains(&index);
        let Some(shader) = panel.shown_shader(index) else {
            break;
        };
        let (column, line) = (slot % TILE_COLUMNS, slot / TILE_COLUMNS);
        let x = COLUMN_X + 8.0 + column as f32 * (TILE + TILE_GAP);
        let top = y + 8.0 + line as f32 * (TILE + TILE_GAP);
        let token = SHADER_ROW + slot as u16;
        let chosen = index == panel.shader_row;
        let own = shader.eq_ignore_ascii_case(own);
        picture(ui, frame, [x, top, TILE, TILE], thumbnail, shader);
        let hovered = ui.token_hovered(token);
        if hovered {
            panel.hover_shader = Some(index);
        }
        let border = if chosen {
            Some((color::GOLD_BRIGHT, 2.5))
        } else if hovered || panel.focus == token {
            Some((color::HOLO, 2.0))
        } else if own {
            Some((color::QUIET, 1.5))
        } else {
            None
        };
        if let Some((colour, width)) = border {
            push(
                ui,
                DrawCommand::Border {
                    rect: frame.rect(x - 1.0, top - 1.0, TILE + 2.0, TILE + 2.0),
                    radius: 5.0 * s,
                    width: width * s,
                    color: colour,
                },
            );
        }
        if let Some(reason) = reason {
            push(
                ui,
                DrawCommand::RoundedRect {
                    rect: frame.rect(x + 1.0, top + TILE - 15.0, TILE - 2.0, 14.0),
                    radius: 3.0 * s,
                    color: color::alpha(color::SPACE, 0.8),
                },
            );
            text(
                ui,
                TextFamily::Body,
                format_args!("{}", reason.band()),
                frame.rect(x, top + TILE - 16.0, TILE, 16.0),
                11.0 * s,
                reason_colour(reason),
                FontWeight::Regular,
                TextAlign::Center,
            );
        } else if recent {
            push(
                ui,
                DrawCommand::RoundedRect {
                    rect: frame.rect(x + TILE - 9.0, top + 3.0, 6.0, 6.0),
                    radius: 3.0 * s,
                    color: color::HOLO,
                },
            );
        }
        ui.hit_region(token, frame.rect(x, top, TILE, TILE));
    }
}

/// The large preview left of the column, over the scene's edge: the shader under the
/// pointer (else the chosen one) at its picture's own shape, its name and size, and the
/// selection's own shader small under it, so a texture is judged before it is drawn.
fn preview_card(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel, top: f32, own: &str) {
    const X: f32 = 1164.0;
    const WIDTH: f32 = 224.0;
    const BOX: f32 = 200.0;
    const HEIGHT: f32 = 372.0;
    let s = frame.s;
    let hovered = panel.hover_shader;
    let index = hovered.unwrap_or(panel.shader_row);
    let Some(shader) = panel.shown_shader(index).map(str::to_owned) else {
        return;
    };
    let rect = frame.rect(X, top, WIDTH, HEIGHT);
    panel.preview_rect = Some(rect);
    push(
        ui,
        DrawCommand::RoundedRect {
            rect,
            radius: 12.0 * s,
            color: color::alpha(color::SPACE, 0.88),
        },
    );
    push(
        ui,
        DrawCommand::Border {
            rect,
            radius: 12.0 * s,
            width: 1.5 * s,
            color: color::alpha(color::HOLO, 0.35),
        },
    );
    let small = |ui: &mut MenuCanvas, y: f32, value: std::fmt::Arguments<'_>, colour: Color| {
        text(
            ui,
            TextFamily::Body,
            value,
            frame.rect(X + 12.0, y - 1.0, WIDTH - 24.0, 21.0),
            13.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    };
    small(
        ui,
        top + 8.0,
        format_args!(
            "{}",
            if hovered.is_some() {
                "Under the pointer"
            } else {
                "Chosen"
            }
        ),
        color::QUIET,
    );
    let (box_x, box_y) = (X + 12.0, top + 30.0);
    push(
        ui,
        DrawCommand::RoundedRect {
            rect: frame.rect(box_x, box_y, BOX, BOX),
            radius: 6.0 * s,
            color: color::alpha(color::SPACE, 0.95),
        },
    );
    let mut size_line = None;
    match panel.pictures.preview(&shader) {
        Some(Some((texture, [width, height]))) => {
            // The picture's own shape, as large as the box holds.
            let scale = BOX / width.max(height).max(1) as f32;
            let (w, h) = (width as f32 * scale, height as f32 * scale);
            push(
                ui,
                DrawCommand::TexturedQuad {
                    rect: frame.rect(box_x + (BOX - w) * 0.5, box_y + (BOX - h) * 0.5, w, h),
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, 1.0),
                },
            );
            size_line = Some((width, height));
        }
        Some(None) => small(
            ui,
            box_y + BOX * 0.5 - 9.0,
            format_args!("No picture: drawn by its script only"),
            color::MUTED,
        ),
        None => small(
            ui,
            box_y + BOX * 0.5 - 9.0,
            format_args!("Loading the picture..."),
            color::QUIET,
        ),
    }
    let mut y = box_y + BOX + 8.0;
    for part in wrap(short_shader(&shader), 28).take(2) {
        small(ui, y, format_args!("{part}"), color::TEXT);
        y += 18.0;
    }
    match size_line {
        Some((width, height)) => small(
            ui,
            y,
            format_args!("Picture {width} x {height}"),
            color::QUIET,
        ),
        None => small(ui, y, format_args!(" "), color::QUIET),
    }
    let y = top + HEIGHT - 66.0;
    small(ui, y, format_args!("Drawn there now"), color::QUIET);
    let thumbnail = panel.pictures.thumbnail(own);
    picture(ui, frame, [X + 12.0, y + 20.0, 40.0, 40.0], thumbnail, own);
    text(
        ui,
        TextFamily::Body,
        format_args!("{}", wrap(short_shader(own), 20).next().unwrap_or("")),
        frame.rect(X + 60.0, y + 29.0, WIDTH - 72.0, 21.0),
        13.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::Start,
    );
}

/// What marks a shader row at its right: why it is suggested (a nearby share in gold,
/// "same family" or "140 on map" in holo), else "Recent";
/// a line under the last of each group.
fn head_mark(ui: &mut MenuCanvas, frame: &Frame, top: f32, index: usize, panel: &Panel) {
    let s = frame.s;
    let rect = frame.rect(
        COLUMN_X + COLUMN_WIDTH - 116.0,
        top + (LIST_ROW - 20.0) * 0.5,
        100.0,
        20.0,
    );
    if let Some(reason) = panel.suggestion(index) {
        text(
            ui,
            TextFamily::Body,
            format_args!("{}", reason.mark()),
            rect,
            13.0 * s,
            reason_colour(reason),
            FontWeight::Regular,
            TextAlign::End,
        );
        if index + 1 == panel.candidate_rows {
            under_line(ui, frame, top);
        }
        return;
    }
    if let Some(index) = index.checked_sub(panel.candidate_rows) {
        recent_mark(ui, frame, top, index, panel.recent_shader_rows);
    }
}

/// A suggestion's mark: gold for the nearby vote (Fill draws it on faces), holo else.
fn reason_colour(reason: super::suggest::Reason) -> Color {
    match reason {
        super::suggest::Reason::Nearby(_) => color::GOLD_BRIGHT,
        _ => color::HOLO,
    }
}

/// The line under the last row of a group at the list's head.
fn under_line(ui: &mut MenuCanvas, frame: &Frame, top: f32) {
    push(
        ui,
        DrawCommand::RoundedRect {
            rect: frame.rect(
                COLUMN_X + 18.0,
                top + LIST_ROW - 1.0,
                COLUMN_WIDTH - 36.0,
                1.0,
            ),
            radius: 0.5 * frame.s,
            color: color::alpha(color::HOLO, 0.35),
        },
    );
}

/// "Recent" at the right of a list's row `index` (its top at `top`) among the first
/// `recent` rows, and a line under the last of them.
fn recent_mark(ui: &mut MenuCanvas, frame: &Frame, top: f32, index: usize, recent: usize) {
    if index >= recent {
        return;
    }
    let s = frame.s;
    text(
        ui,
        TextFamily::Body,
        format_args!("Recent"),
        frame.rect(
            COLUMN_X + COLUMN_WIDTH - 96.0,
            top + (LIST_ROW - 20.0) * 0.5,
            80.0,
            20.0,
        ),
        13.0 * s,
        color::QUIET,
        FontWeight::Regular,
        TextAlign::End,
    );
    if index + 1 == recent {
        push(
            ui,
            DrawCommand::RoundedRect {
                rect: frame.rect(
                    COLUMN_X + 18.0,
                    top + LIST_ROW - 1.0,
                    COLUMN_WIDTH - 36.0,
                    1.0,
                ),
                radius: 0.5 * s,
                color: color::alpha(color::HOLO, 0.35),
            },
        );
    }
}

/// The Place page: the game data's models, a search narrowing them, placing the chosen
/// one, and the placed model's controls.
fn place(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel) {
    if panel.things {
        things(ui, frame, panel);
        return;
    }
    let mut y = PAGE_Y;
    let searching = panel.focus == SEARCH;
    let switch = 92.0;
    kit::search(
        ui,
        frame,
        [COLUMN_X, y, COLUMN_WIDTH - switch - GAP, FIELD],
        &panel.search,
        searching,
        "Search the models",
        (!panel.search.is_empty()).then_some(panel.shown.len()),
        SEARCH,
    );
    kit::button(
        ui,
        frame,
        [COLUMN_X + COLUMN_WIDTH - switch, y + 2.0, switch, BUTTON],
        "Things",
        false,
        true,
        panel.focus == PLACE_THINGS,
        PLACE_THINGS,
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
        let top = y + 8.0 + slot as f32 * LIST_ROW;
        list_row(
            ui,
            frame,
            top,
            token,
            shown,
            index == panel.model_row,
            panel.focus == token,
            false,
        );
        recent_mark(ui, frame, top, index, panel.recent_model_rows);
    }
    y += height + 12.0;
    // Place, then copies of the selected placed models and every placed model selected.
    let small = 100.0;
    kit::button(
        ui,
        frame,
        [COLUMN_X, y, COLUMN_WIDTH - 2.0 * (small + GAP), BUTTON],
        "Place where you aimed",
        false,
        panel.chosen_model().is_some(),
        panel.focus == PLACE,
        PLACE,
    );
    kit::button(
        ui,
        frame,
        [
            COLUMN_X + COLUMN_WIDTH - 2.0 * small - GAP,
            y,
            small,
            BUTTON,
        ],
        "Duplicate",
        false,
        panel.facts.placed.is_some() || panel.facts.models > 0,
        panel.focus == DUPLICATE,
        DUPLICATE,
    );
    kit::button(
        ui,
        frame,
        [COLUMN_X + COLUMN_WIDTH - small, y, small, BUTTON],
        "Select all",
        false,
        true,
        panel.focus == SELECT_PLACED,
        SELECT_PLACED,
    );
    y += BUTTON + 30.0;
    let set = panel.facts.models > 1;
    kit::heading(
        ui,
        frame,
        COLUMN_X,
        y,
        COLUMN_WIDTH,
        if set { "Placed models" } else { "Placed model" },
    );
    y += 14.0;
    let placed = panel.facts.placed.is_some() || set;
    let whole_set = set && panel.facts.models == panel.facts.count;
    match &panel.facts.placed {
        _ if set => line(
            ui,
            frame,
            y,
            &format!(
                "{} selected: these move them all; Grab one",
                panel.facts.models
            ),
            color::MUTED,
        ),
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
            (
                GRAB,
                if panel.facts.grabbing {
                    "Put down"
                } else {
                    "Grab"
                },
                placed && !set,
            ),
            (
                REMOVE,
                if set { "Remove all" } else { "Remove" },
                placed && (!set || whole_set),
            ),
        ],
    );
}

/// Place's things: the boomboxes, mystery boxes and toys standing on the map, as the
/// browser editor's World page; stand one where the next click meets the floor (boxes and
/// toys SJK's creator's, on a game server), the toy's kind and escort, remove the chosen
/// one. No move for a box or a toy: the hub places and removes them only.
fn things(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel) {
    use super::web::http::Thing;
    let mut y = PAGE_Y;
    let switch = 92.0;
    kit::heading(
        ui,
        frame,
        COLUMN_X,
        y + 8.0,
        COLUMN_WIDTH - switch - GAP,
        "On this map",
    );
    kit::button(
        ui,
        frame,
        [COLUMN_X + COLUMN_WIDTH - switch, y, switch, BUTTON],
        "Models",
        false,
        true,
        panel.focus == PLACE_MODELS,
        PLACE_MODELS,
    );
    y += BUTTON + 8.0;
    let height = LIST_ROW * THING_ROWS as f32 + 16.0;
    list_ground(ui, frame, [COLUMN_X, y, COLUMN_WIDTH, height]);
    panel.thing_list = Some(frame.rect(COLUMN_X, y, COLUMN_WIDTH, height));
    if panel.facts.things.is_empty() {
        line(
            ui,
            frame,
            y + 16.0,
            "No boombox, box or toy stands here",
            color::MUTED,
        );
    }
    for slot in 0..THING_ROWS {
        let index = panel.thing_first + slot;
        let Some(row) = panel.facts.things.get(index) else {
            break;
        };
        let token = THING_ROW + slot as u16;
        list_row(
            ui,
            frame,
            y + 8.0 + slot as f32 * LIST_ROW,
            token,
            &row.label,
            index == panel.thing_row,
            panel.focus == token,
            row.thing == Thing::Boombox && !row.mine,
        );
    }
    y += height + 8.0;
    let facts = &panel.facts;
    let hub = facts.creator && facts.on_server;
    let placing = panel.placing_thing;
    marked_buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (
                THING_BOOMBOX,
                "Your boombox",
                true,
                placing == Some(Thing::Boombox),
            ),
            (
                THING_BOX,
                "Mystery box",
                hub,
                placing == Some(Thing::MysteryBox),
            ),
            (
                THING_TOY,
                "Toy",
                hub && !facts.toy_kinds.is_empty(),
                placing == Some(Thing::Toy),
            ),
        ],
    );
    y += BUTTON + GAP;
    let kind = panel.toy_kind().unwrap_or("none").to_owned();
    let removable = panel.chosen_thing().is_some_and(|row| match row.thing {
        Thing::Boombox => row.mine,
        _ => panel.facts.creator,
    });
    marked_buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (THING_KIND, &format!("Toy: {kind}"), hub, false),
            (
                THING_ESCORT,
                if panel.escort {
                    "Escort: on"
                } else {
                    "Escort: off"
                },
                hub,
                panel.escort,
            ),
            (THING_REMOVE, "Remove", removable, false),
        ],
    );
    y += BUTTON + 16.0;
    let note = if !panel.facts.creator {
        "A button, then a click on the floor stands it there. Mystery boxes and toys are SJK's creator's."
    } else if !panel.facts.on_server {
        "A button, then a click on the floor stands it there. Boxes and toys need a game server."
    } else {
        "A button, then a click on the floor stands it there. The hub has no move for a box or a toy: remove it and place it again."
    };
    for part in wrap(note, LINE_CHARS).take(4) {
        line(ui, frame, y, part, color::MUTED);
        y += 24.0;
    }
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
    y += BUTTON + GAP;
    // The Recent rows of the lists, shared with the browser editor; then the invisible
    // brushes' view (B steps through it).
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (
                REMEMBER,
                if panel.remember {
                    "Recent: remembered"
                } else {
                    "Recent: off"
                },
                true,
            ),
            (FORGET, "Forget recent", panel.remember),
        ],
    );
    y += BUTTON + 16.0;
    kit::heading(
        ui,
        frame,
        COLUMN_X,
        y,
        COLUMN_WIDTH,
        "Invisible brushes (B)",
    );
    y += 18.0;
    let row: Vec<(u16, &str, bool, bool)> = BrushPreset::ALL
        .iter()
        .enumerate()
        .map(|(index, preset)| {
            (
                BRUSHES + index as u16,
                preset.label(),
                true,
                panel.facts.brushes == Some(*preset),
            )
        })
        .collect();
    marked_buttons(ui, frame, panel, y, &row);
}

/// A row of buttons as [`buttons`], each also marked (the choice in force) or not:
/// (token, label, enabled, marked).
fn marked_buttons(
    ui: &mut MenuCanvas,
    frame: &Frame,
    panel: &Panel,
    y: f32,
    row: &[(u16, &str, bool, bool)],
) {
    let count = row.len() as f32;
    let width = (COLUMN_WIDTH - GAP * (count - 1.0)) / count;
    for (index, (token, label, enabled, marked)) in row.iter().enumerate() {
        kit::button(
            ui,
            frame,
            [COLUMN_X + index as f32 * (width + GAP), y, width, BUTTON],
            label,
            *marked,
            *enabled,
            panel.focus == *token,
            *token,
        );
    }
}

/// The Weather page: the map's own weather, what an edit keeps here, and the buttons that
/// choose it (each runs `mapedit_weather`).
fn weather(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel) {
    let facts = &panel.facts;
    let mut y = PAGE_Y;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "This map's weather");
    y += 40.0;
    let own = if facts.map_weather.is_empty() {
        "none".to_owned()
    } else {
        facts.map_weather.join(", ")
    };
    for part in wrap(&format!("Map's own: {own}"), LINE_CHARS).take(2) {
        line(ui, frame, y, part, color::MUTED);
        y += 26.0;
    }
    let kept = facts
        .weather
        .as_ref()
        .map_or(&[][..], |(commands, _)| commands.as_slice());
    let words = match &facts.weather {
        Some((commands, true)) => format!("Kept for this server: {}", commands.join(", ")),
        Some((commands, false)) => format!("Kept: {}", commands.join(", ")),
        None => "Kept: nothing, the map's own shows".to_owned(),
    };
    let colour = if kept.is_empty() {
        color::TEXT
    } else {
        color::GOLD_BRIGHT
    };
    for part in wrap(&words, LINE_CHARS).take(3) {
        line(ui, frame, y, part, colour);
        y += 26.0;
    }
    if facts.forced {
        for part in wrap(
            "Your Force weather setting replaces it on your screen (r_weatherForce 0 shows what players see)",
            LINE_CHARS,
        )
        .take(2)
        {
            line(ui, frame, y, part, color::EMBER);
            y += 26.0;
        }
    }
    y += 12.0;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Choose");
    y += 24.0;
    let preset = (!kept.is_empty()).then(|| weather_preset(kept)).flatten();
    marked_buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (WEATHER_DRIZZLE, "Drizzle", true, preset == Some(1)),
            (WEATHER_RAIN, "Rain", true, preset == Some(2)),
            (WEATHER_STORM, "Storm", true, preset == Some(3)),
            (WEATHER_SNOW, "Snow", true, preset == Some(4)),
        ],
    );
    y += BUTTON + GAP;
    let (fog, wind) = weather_fog_and_wind(kept);
    marked_buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (
                WEATHER_FOG,
                if fog { "Fog: on" } else { "Fog: off" },
                true,
                fog,
            ),
            (
                WEATHER_WIND,
                if wind { "Wind: on" } else { "Wind: off" },
                true,
                wind,
            ),
        ],
    );
    y += BUTTON + GAP;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (WEATHER_SEEN, "Keep what I see", true),
            (WEATHER_OWN, "Map's own", !kept.is_empty()),
        ],
    );
    y += BUTTON + 16.0;
    for part in wrap(
        "Kept weather replaces the map's own for every SJK player once published, on maps with sky; their own Force weather and Weather off still win.",
        LINE_CHARS,
    )
    .take(4)
    {
        line(ui, frame, y, part, color::MUTED);
        y += 24.0;
    }
}

/// The Lights page: the added lights (add one, its colour, reach and strength, delete
/// it) and the map's lamps nearest first (off or on, back as the map has it); the chosen
/// one ringed in gold over the scene, where the gizmo moves it.
fn lights(ui: &mut MenuCanvas, frame: &Frame, panel: &mut Panel) {
    use super::panel_lights::{LightChoice, SWATCHES};
    let s = frame.s;
    let mut y = PAGE_Y;
    let facts = &panel.facts;
    let drawn = facts.lights.len().min(super::lights::BUDGET);
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Added lights");
    y += 18.0;
    line(
        ui,
        frame,
        y,
        &format!(
            "Every SJK player sees them; {drawn} of {} drawn (8 nearest)",
            facts.lights.len()
        ),
        color::MUTED,
    );
    y += 32.0;
    let light_chosen = panel.light_choice == LightChoice::Light && !facts.lights.is_empty();
    let lamp_chosen = panel.light_choice == LightChoice::Lamp && !facts.lamps.is_empty();
    marked_buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (
                LIGHT_ADD,
                if panel.adding_light {
                    "Click the scene"
                } else {
                    "Add: click"
                },
                true,
                panel.adding_light,
            ),
            (LIGHT_AT_VIEW, "At view centre", true, false),
            (LIGHT_DELETE, "Delete", light_chosen || lamp_chosen, false),
        ],
    );
    y += BUTTON + 8.0;
    let height = LIST_ROW * LIGHT_ROWS as f32 + 16.0;
    list_ground(ui, frame, [COLUMN_X, y, COLUMN_WIDTH, height]);
    panel.light_list = Some(frame.rect(COLUMN_X, y, COLUMN_WIDTH, height));
    if panel.facts.lights.is_empty() {
        line(
            ui,
            frame,
            y + 16.0,
            "No light added: Add, then click a wall",
            color::MUTED,
        );
    }
    for slot in 0..LIGHT_ROWS {
        let index = panel.light_first + slot;
        let Some(row) = panel.facts.lights.get(index) else {
            break;
        };
        let token = LIGHT_ROW + slot as u16;
        let top = y + 8.0 + slot as f32 * LIST_ROW;
        let chosen = index == panel.light_row && panel.light_choice == LightChoice::Light;
        list_row_from(
            ui,
            frame,
            top,
            token,
            &row.label(),
            [chosen, panel.focus == token, false],
            22.0,
        );
        swatch_dot(ui, frame, top, row.color);
    }
    y += height + 8.0;
    // The colours, then the reach and the strength, of the chosen light.
    let count = SWATCHES.len() as f32;
    let width = (COLUMN_WIDTH - 6.0 * (count - 1.0)) / count;
    let current = panel.facts.lights.get(panel.light_row).map(|row| row.color);
    for (index, (_, rgb)) in SWATCHES.iter().enumerate() {
        let token = LIGHT_SWATCH + index as u16;
        let x = COLUMN_X + index as f32 * (width + 6.0);
        let rect = frame.rect(x, y, width, 30.0);
        push(
            ui,
            DrawCommand::RoundedRect {
                rect,
                radius: 6.0 * s,
                color: Color::new(
                    rgb[0],
                    rgb[1],
                    rgb[2],
                    if light_chosen { 0.95 } else { 0.35 },
                ),
            },
        );
        let lit = current == Some(*rgb) && light_chosen;
        if lit || panel.focus == token || ui.token_hovered(token) {
            push(
                ui,
                DrawCommand::Border {
                    rect,
                    radius: 6.0 * s,
                    width: 2.0 * s,
                    color: if lit { color::GOLD_BRIGHT } else { color::TEXT },
                },
            );
        }
        ui.hit_region(token, rect);
    }
    y += 38.0;
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (LIGHT_SMALLER, "Smaller", light_chosen),
            (LIGHT_BIGGER, "Bigger", light_chosen),
            (LIGHT_DIMMER, "Dimmer", light_chosen),
            (LIGHT_BRIGHTER, "Brighter", light_chosen),
        ],
    );
    y += BUTTON + 26.0;
    kit::heading(ui, frame, COLUMN_X, y, COLUMN_WIDTH, "Map lamps");
    y += 18.0;
    line(
        ui,
        frame,
        y,
        if panel.facts.realtime {
            "Nearest first; an edit applies on the map's next load"
        } else {
            "Found on the Experimental preset only"
        },
        color::MUTED,
    );
    y += 28.0;
    let height = LIST_ROW * LIGHT_ROWS as f32 + 16.0;
    list_ground(ui, frame, [COLUMN_X, y, COLUMN_WIDTH, height]);
    panel.lamp_list = Some(frame.rect(COLUMN_X, y, COLUMN_WIDTH, height));
    if panel.facts.lamps.is_empty() {
        line(ui, frame, y + 16.0, "No lamp in this world", color::MUTED);
    }
    for slot in 0..LIGHT_ROWS {
        let index = panel.lamp_first + slot;
        let Some(row) = panel.facts.lamps.get(index) else {
            break;
        };
        let token = LAMP_ROW + slot as u16;
        let top = y + 8.0 + slot as f32 * LIST_ROW;
        let chosen = index == panel.lamp_row && panel.light_choice == LightChoice::Lamp;
        list_row_from(
            ui,
            frame,
            top,
            token,
            &row.label(),
            [chosen, panel.focus == token, row.off],
            22.0,
        );
        swatch_dot(ui, frame, top, if row.off { [0.45; 3] } else { row.color });
    }
    y += height + 8.0;
    let off = panel
        .facts
        .lamps
        .get(panel.lamp_row)
        .is_some_and(|row| row.off);
    buttons(
        ui,
        frame,
        panel,
        y,
        &[
            (
                LAMP_TOGGLE,
                if off { "Turn on" } else { "Turn off" },
                lamp_chosen,
            ),
            (LAMP_RESET, "As the map has it", lamp_chosen),
        ],
    );
}

/// A small dot of `rgb` at the left of a list row.
fn swatch_dot(ui: &mut MenuCanvas, frame: &Frame, top: f32, rgb: [f32; 3]) {
    push(
        ui,
        DrawCommand::RoundedRect {
            rect: frame.rect(COLUMN_X + 18.0, top + LIST_ROW * 0.5 - 7.0, 14.0, 14.0),
            radius: 7.0 * frame.s,
            color: Color::new(rgb[0], rgb[1], rgb[2], 1.0),
        },
    );
}

/// Where new edits apply, the last answer, then Publish and Editor off.
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
    let width = (COLUMN_WIDTH - GAP) / 2.0;
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

/// The keys of what has the keyboard, up to three lines at the column's foot, then why
/// the editor has the player's own view when the free camera was refused.
fn keys(ui: &mut MenuCanvas, frame: &Frame, panel: &Panel) {
    let right = COLUMN_X + COLUMN_WIDTH;
    let exit = [exit_cap(panel)];
    let lines = key_lines(panel, &exit);
    for (index, hints) in lines.iter().enumerate() {
        hint_line(ui, frame, right, KEYS_Y + index as f32 * KEYS_PITCH, hints);
    }
    if let Some(camera) = &panel.facts.camera {
        text(
            ui,
            TextFamily::Body,
            format_args!("{camera}"),
            frame.rect(
                COLUMN_X - 60.0,
                KEYS_Y + 3.0 * KEYS_PITCH,
                COLUMN_WIDTH + 60.0,
                24.0,
            ),
            15.0 * frame.s,
            color::EMBER,
            FontWeight::Regular,
            TextAlign::End,
        );
    }
}

/// The key lines: flying, a gizmo's transform, typing in a search, or the panel's own
/// (with the key that leaves on the third).
fn key_lines<'a>(panel: &'a Panel, exit: &'a [&'a str]) -> Vec<Vec<Hint<'a>>> {
    let leave: Hint<'a> = (exit, "leave");
    if panel.facts.flying {
        return vec![
            vec![
                (&["W", "A", "S", "D"], "fly"),
                (&["Space", "C"], "up, down"),
            ],
            vec![
                (&["Shift"], "faster"),
                (&["Wheel"], "speed"),
                (&["Esc"], "back"),
            ],
        ];
    }
    if panel.transforming {
        return vec![
            vec![(&["Click"], "done"), (&["Right"], "cancel")],
            vec![
                (&["X", "Y", "Z"], "axis"),
                (&["Enter"], "done"),
                (&["Esc"], "cancel"),
            ],
        ];
    }
    let typing = matches!(panel.focus, SEARCH | REMAP_FIELD);
    let first: Vec<Hint<'a>> = if panel.facts.grabbing {
        vec![
            (&["Click"], "put down"),
            (&["Wheel"], "turn"),
            (&["Esc"], "back"),
        ]
    } else if typing {
        vec![
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
        vec![
            (&["RMB"], "fly"),
            (&["Click"], "select"),
            (&["Shift+Click"], "add"),
        ]
    };
    let second: Vec<Hint<'a>> = if panel.tab == Tab::Place {
        vec![
            (&["Shift"], "fine"),
            (&["G", "R", "T"], "gizmo"),
            (&["Esc"], "back"),
        ]
    } else {
        vec![
            (&["G", "R", "T"], "gizmo"),
            (&["Drag"], "box"),
            (&["Esc"], "back"),
        ]
    };
    vec![first, second, vec![(&["Ctrl", "Z"], "undo"), leave]]
}

/// The cap of the key that leaves: the one bound to `mapedit`.
fn exit_cap(panel: &Panel) -> &str {
    if panel.facts.exit_key.is_empty() {
        "mapedit"
    } else {
        panel.facts.exit_key.as_str()
    }
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
            ("weather", panel(Tab::Weather, None)),
            ("face", panel(Tab::Selected, Some(Picked::Face))),
            ("sky", panel(Tab::Selected, Some(Picked::Sky))),
            ("models", models_panel(Tab::Selected, 3, 3)),
            ("mixed", models_panel(Tab::Selected, 2, 5)),
            ("place set", models_panel(Tab::Place, 3, 3)),
            ("tiles", {
                let mut panel = panel(Tab::Selected, Some(Picked::Surface { soup: false }));
                panel.toggle_tiles();
                panel
            }),
            ("fill", fill_panel()),
            ("lights", lights_panel()),
            ("things", things_panel()),
        ]
    }

    /// A hidden face with Fill's candidates at the head of its list.
    fn fill_panel() -> Panel {
        let mut panel = panel(Tab::Selected, Some(Picked::Face));
        panel.facts.fill_candidates = vec![
            ("textures/yavin/wall".to_owned(), 0.62),
            ("textures/yavin/stone".to_owned(), 0.2),
            ("textures/unknown/gone".to_owned(), 0.1),
        ];
        panel.facts.fill = panel.facts.fill_candidates.first().cloned();
        panel.refilter_shaders();
        panel
    }

    /// The Lights page with three lights and five lamps.
    fn lights_panel() -> Panel {
        use super::super::panel_lights::{LampRow, LightRow};
        let mut panel = panel(Tab::Lights, None);
        panel.facts.lights = (0..3)
            .map(|id| LightRow {
                id: 20 + id,
                origin: [id as f32 * 64.0, 0.0, 32.0],
                color: [1.0, 0.85, 0.65],
                radius: 320.0,
                intensity: 1.0,
            })
            .collect();
        panel.facts.lamps = (0..5)
            .map(|n| LampRow {
                fixture: format!("surface:{}", 1000 + n),
                name: "textures/kejim/light_blue".to_owned(),
                position: [0.0, n as f32 * 32.0, 128.0],
                color: [0.5, 0.7, 1.0],
                off: n == 1,
                offset: [0.0; 3],
                pending: n == 1,
            })
            .collect();
        panel.facts.realtime = true;
        panel
    }

    /// Place's things: a boombox, a mystery box and a toy, as the creator on a server.
    fn things_panel() -> Panel {
        use super::super::web::http::Thing;
        use super::super::web::things::ThingRow;
        let mut panel = panel(Tab::Place, None);
        panel.things = true;
        let row = |thing, id, label: &str, mine| ThingRow {
            thing,
            id,
            label: label.to_owned(),
            mine,
            origin: glam::Vec3::ZERO,
            parts: Vec::new(),
        };
        panel.facts.things = vec![
            row(Thing::Boombox, 0, "Your boombox", true),
            row(Thing::MysteryBox, 12, "Mystery box 12", false),
            row(Thing::Toy, 3, "Toy 3: cat", false),
        ];
        panel.facts.toy_kinds = vec!["cat".to_owned(), "drone".to_owned()];
        panel.facts.creator = true;
        panel.facts.on_server = true;
        panel
    }

    /// The shader list as tiles shows more of them, Fill's candidates lead a face's list
    /// with their shares, the Lights page offers its lists and buttons, and Place's things
    /// their placing.
    #[test]
    fn the_new_pages_offer_their_controls() {
        let build_state = |state: &str| {
            let (_, mut panel) = states()
                .into_iter()
                .find(|(name, _)| *name == state)
                .unwrap();
            let mut canvas = MenuCanvas::new();
            build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
            (panel, canvas)
        };
        let (panel, canvas) = build_state("tiles");
        let tokens = canvas.widget_tokens().to_vec();
        assert!(tokens.contains(&TILES) && tokens.contains(&(SHADER_ROW + 5)));
        assert!(panel.preview_rect.is_some(), "the large preview card");
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(
            runs.contains(&"Rows") && runs.contains(&"Loading the picture..."),
            "{runs:?}"
        );

        let (panel, canvas) = build_state("fill");
        assert_eq!(
            panel.candidate_rows, 2,
            "a candidate the list lacks is left out"
        );
        assert_eq!(panel.shown_shader(0), Some("textures/yavin/wall"));
        assert_eq!(panel.candidate_share(1), Some(0.2));
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"62 % nearby"), "{runs:?}");

        let (_, canvas) = build_state("lights");
        let tokens = canvas.widget_tokens().to_vec();
        for token in [
            TAB_LIGHTS,
            LIGHT_ADD,
            LIGHT_AT_VIEW,
            LIGHT_DELETE,
            LIGHT_SWATCH,
            LIGHT_SWATCH + 7,
            LIGHT_BRIGHTER,
            LIGHT_ROW + 2,
            LAMP_ROW + 2,
            LAMP_TOGGLE,
            LAMP_RESET,
        ] {
            assert!(tokens.contains(&token), "{token}");
        }
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"20: #ffd9a6 r320 x1.00"), "{runs:?}");
        assert!(
            runs.contains(&"light_blue (surface:1001) off (on reload)"),
            "{runs:?}"
        );

        let (_, canvas) = build_state("things");
        let tokens = canvas.widget_tokens().to_vec();
        for token in [
            PLACE_MODELS,
            THING_BOOMBOX,
            THING_BOX,
            THING_TOY,
            THING_KIND,
            THING_REMOVE,
            THING_ROW + 2,
        ] {
            assert!(tokens.contains(&token), "{token}");
        }
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(
            runs.contains(&"Toy: cat") && runs.contains(&"Mystery box 12"),
            "{runs:?}"
        );
    }

    /// In tiles, Up and Down go a line and Left and Right a tile; a list's wheel goes a
    /// line of tiles.
    #[test]
    fn tiles_step_by_lines_and_tiles() {
        let mut panel = panel(Tab::Selected, Some(Picked::Surface { soup: false }));
        let many: Vec<String> = (0..40).map(|n| format!("textures/t/s{n:02}")).collect();
        let names: Vec<&str> = many.iter().map(String::as_str).collect();
        panel.set_shaders(&names);
        panel.remap.clear();
        panel.refilter_shaders();
        panel.toggle_tiles();
        panel.focus = SHADER_ROW;
        assert!(panel.move_line(true));
        assert_eq!(panel.shader_row, TILE_COLUMNS);
        assert!(panel.move_in_shaders(1, false));
        assert_eq!(panel.shader_row, TILE_COLUMNS + 1);
        // Down past the third line scrolls a whole line.
        for _ in 0..3 {
            assert!(panel.move_line(true));
        }
        assert_eq!(
            panel.shader_first,
            2 * TILE_COLUMNS,
            "line 4 shown last of three"
        );
        assert_eq!(
            panel.focus,
            SHADER_ROW + (panel.shader_row - panel.shader_first) as u16
        );
        panel.toggle_tiles();
        assert_eq!(panel.shader_slots(), (SHADER_ROWS, 1));
        assert!(panel.shader_row >= panel.shader_first);
        assert!(panel.shader_row < panel.shader_first + SHADER_ROWS);
    }

    /// The B key and the Edits row read and set the brush view's presets.
    #[test]
    fn brush_presets_follow_the_cvars() {
        for preset in BrushPreset::ALL {
            let (mode, types) = preset.cvars();
            assert_eq!(BrushPreset::of(mode.parse().unwrap(), types), Some(preset));
        }
        assert_eq!(
            BrushPreset::of(2, "Solid, CAULK nodraw"),
            Some(BrushPreset::Caulk)
        );
        assert_eq!(BrushPreset::of(1, "clip"), None, "a hand-picked set");
        assert_eq!(BrushPreset::next(None), BrushPreset::Play);
        assert_eq!(BrushPreset::next(Some(BrushPreset::All)), BrushPreset::Off);
        let mut panel = Panel::default();
        assert_eq!(
            panel.activate(BRUSHES + 2),
            Some(Action::Brushes(Some(BrushPreset::Caulk)))
        );
    }

    /// `models` placed models selected out of `count`, the newest a placed one.
    fn models_panel(tab: Tab, models: usize, count: usize) -> Panel {
        let mut panel = panel(tab, Some(Picked::Placed));
        panel.facts.models = models;
        panel.facts.count = count;
        panel
    }

    #[test]
    fn a_set_of_placed_models_says_its_count_and_moves_them_all() {
        let build_state = |state: &str| {
            let (_, mut panel) = states()
                .into_iter()
                .find(|(name, _)| *name == state)
                .unwrap();
            let mut canvas = MenuCanvas::new();
            build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
            canvas
        };
        let canvas = build_state("models");
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"3 models selected"), "{runs:?}");
        assert!(runs.contains(&"Remove all"), "{runs:?}");
        let tokens = canvas.widget_tokens().to_vec();
        for token in [
            HIDE, TURN_LEFT, TURN_RIGHT, UP, DOWN, MOVE_LEFT, NEARER, SMALLER, BIGGER,
        ] {
            assert!(tokens.contains(&token), "{token}");
        }
        assert!(!tokens.contains(&REMAP), "a set of models takes no remap");
        // Mixed with surfaces or faces: the newest's page, the models counted.
        let canvas = build_state("mixed");
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"Placed model and 4 more"), "{runs:?}");
        assert!(
            runs.contains(&"2 placed models among them: G R T move them"),
            "{runs:?}"
        );
        assert!(runs.contains(&"Hide all"), "{runs:?}");
        // Place: the buttons move the set; Grab one at a time.
        let canvas = build_state("place set");
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"Placed models"), "{runs:?}");
        assert!(
            runs.contains(&"3 selected: these move them all; Grab one"),
            "{runs:?}"
        );
        assert!(runs.contains(&"Remove all"), "{runs:?}");
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
        assert_eq!(panel.edit_first, 4);
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
        for tab in Tab::ALL {
            for (transforming, flying, focus) in [
                (false, false, tab.token()),
                (true, false, tab.token()),
                (false, true, tab.token()),
                (false, false, SEARCH),
            ] {
                for (exit_key, grabbing) in [("", false), ("F7", true), ("Backspace", false)] {
                    let mut panel = Panel::default();
                    panel.facts.grabbing = grabbing;
                    panel.tab = tab;
                    panel.focus = focus;
                    panel.transforming = transforming;
                    panel.facts.flying = flying;
                    panel.facts.exit_key = exit_key.to_owned();
                    let exit = [exit_cap(&panel)];
                    let lines = key_lines(&panel, &exit);
                    assert!((2..=3).contains(&lines.len()));
                    for hints in &lines {
                        let line = width(hints);
                        assert!(
                            line <= COLUMN_WIDTH,
                            "{tab:?} {transforming} {flying} {exit_key}: {line}"
                        );
                    }
                }
            }
        }
        // The panel's own lines name the right button, Shift+click and the key that leaves.
        let mut panel = Panel::default();
        panel.facts.exit_key = "F7".to_owned();
        let exit = [exit_cap(&panel)];
        let lines = key_lines(&panel, &exit);
        let actions: Vec<&str> = lines.iter().flatten().map(|(_, action)| *action).collect();
        for wanted in ["fly", "select", "add", "gizmo", "back", "leave", "undo"] {
            assert!(actions.contains(&wanted), "{wanted}: {actions:?}");
        }
        assert!(lines.iter().flatten().any(|(caps, _)| caps == &["F7"]));
    }

    #[test]
    fn a_multiple_selection_says_its_count_and_a_face_offers_link_and_fill() {
        let (_, mut panel) = states()
            .into_iter()
            .find(|(name, _)| *name == "face")
            .unwrap();
        panel.facts.count = 12;
        panel.facts.fill = Some(("textures/yavin/stone_walk".to_owned(), 0.62));
        let mut canvas = MenuCanvas::new();
        build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
        assert!(!canvas.overflowed());
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"Hidden face and 11 more"), "{runs:?}");
        assert!(
            runs.contains(&"Fill: yavin/stone_walk (62 % nearby)"),
            "{runs:?}"
        );
        let tokens = canvas.widget_tokens().to_vec();
        for token in [LINK_PLANE, LINK_SURFACE, FILL, REMAP, UNREMAP] {
            assert!(tokens.contains(&token), "{token}");
        }
        // A surface among many hides them all, never a whole model.
        let (_, mut panel) = states()
            .into_iter()
            .find(|(name, _)| *name == "surface")
            .unwrap();
        panel.facts.count = 3;
        build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
        let runs: Vec<&str> = canvas.text_runs().collect();
        assert!(runs.contains(&"Hide all"));
        // The Edits page carries the Recent rows' switch.
        let (_, mut panel) = states()
            .into_iter()
            .find(|(name, _)| *name == "edits")
            .unwrap();
        build(&mut panel, &mut canvas, [1_920.0, 1_080.0]);
        let tokens = canvas.widget_tokens().to_vec();
        assert!(tokens.contains(&REMEMBER) && tokens.contains(&FORGET));
    }
}
