//! The HUD picker: Enter or a click on the HUD settings' "HUD" row opens a
//! list of every HUD that can be used (see [`crate::menu_hud::choices`])
//! beside a picture of the highlighted one, drawn for a sample player (see
//! [`crate::menu_hud::preview`]). Arrows, the wheel or the pointer move the
//! highlight; Enter or a click uses the HUD and closes; Escape (or the ESC
//! hint) closes without changing it. Left and right on the row itself step
//! through the same list without opening it.
//!
//! It is the SJK UI's pop-up card over the darkened map in every menu style
//! (`docs/sjk-ui.md`, Settings), as the resolution list is: the list on the
//! card's left, the picture on its right, the keys under the card. A
//! game-data HUD's picture is composited on a worker; SJK's own layouts are
//! drawn over the same backdrop by the live HUD's code, so the picture is
//! what a match shows. Positions are pixels of the SJK UI's 16:9 frame
//! ([`Frame`]).
//!
//! The list needs the mounted files and the previews the shader catalogue
//! too, which the menu does not hold: [`SettingsMenu::service_hud_picker`]
//! is given both every frame the settings are up. Previews render on a
//! worker, one at a time, and stay cached until the picker closes.

use super::*;
use crate::hud::{HudLook, HudOverlay, HudVisibility};
use crate::menu::levelshot::LevelshotImage;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text};
use crate::menu_hud::choices::{self, HudChoice};
use crate::menu_hud::preview::{self, PreviewText};
use crate::menu_hud::{FILES_CVAR, HudStyle, PACK_CVAR};
use crate::menu_widgets::{BACK_TOKEN, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, InputEvent, TextAlign, UiEventKind};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

/// Wheel target over the list.
const LIST_SCROLL_TOKEN: u16 = 913;
/// The card, its inner margin and its text's column.
const CARD: [f32; 4] = [240.0, 80.0, 1440.0, 780.0];
const MARGIN: f32 = 48.0;
const TEXT_X: f32 = CARD[0] + MARGIN;
const TEXT_WIDTH: f32 = CARD[2] - MARGIN * 2.0;
/// The list: its first row's top, a row's height, the rows shown at once
/// and their width, with room for the in-use dot.
const ROWS_TOP: f32 = CARD[1] + 150.0;
const ROW: f32 = 48.0;
const LIST_ROWS: usize = 12;
const LIST_WIDTH: f32 = 420.0;
const DOT_ROOM: f32 = 26.0;
/// The picture (16:9), right of the list to the card's margin.
const PREVIEW_X: f32 = TEXT_X + LIST_WIDTH + 64.0;
const PREVIEW_WIDTH: f32 = CARD[0] + CARD[2] - MARGIN - PREVIEW_X;
const PREVIEW: [f32; 4] = [
    PREVIEW_X,
    ROWS_TOP,
    PREVIEW_WIDTH,
    PREVIEW_WIDTH * 9.0 / 16.0,
];
/// The keys' line under the card, and the room between its hints.
const KEYS: [(&[&str], &str); 3] = [
    (&["Up", "Down"], "Choose"),
    (&["Enter"], "Use"),
    (&["Esc"], "Back"),
];
const KEY_GAP: f32 = 22.0;

// The rows and the picture with its name and origin inside the card, the
// keys under it inside the frame.
const _: () = assert!(ROWS_TOP + LIST_ROWS as f32 * ROW <= CARD[1] + CARD[3] - MARGIN * 0.5);
const _: () = assert!(PREVIEW[1] + PREVIEW[3] + 84.0 <= CARD[1] + CARD[3] - MARGIN * 0.5);
const _: () = assert!(CARD[1] + CARD[3] + 28.0 + 24.0 <= 1080.0);

/// The player SJK's layouts are drawn for: the game-data pictures' wounded
/// saberist (`menu_hud::preview`), medium style.
const SAMPLE: sjk_client::HudDataSource = sjk_client::HudDataSource {
    health: 74,
    armor: 46,
    force: 63,
    weapon: 3,
    ammo: None,
    saber_style: Some(2),
};
const SAMPLE_RATIOS: [f32; 4] = [0.74, 0.46, 0.63, 0.0];
/// Long after the last weapon change, so the weapon's name has faded.
const SAMPLE_TIME_MS: u64 = 60_000;
/// The status and weapon widgets alone: no crosshair (the backdrop has one),
/// timer, lagometer or selection rows.
const SAMPLE_VISIBILITY: HudVisibility = HudVisibility {
    hud: true,
    elements: true,
    status: true,
    vehicle: false,
    weapon: true,
    selection: false,
    crosshair: false,
    crosshair_names: false,
    timer: false,
    lagometer: false,
    ground_hud: false,
    menu_hud: false,
};

/// A rendered preview, ready to upload.
struct Ready {
    image: LevelshotImage,
    texts: Vec<PreviewText>,
}

/// The cvars that select a HUD, as last read.
#[derive(Clone, Debug, Default, PartialEq)]
struct Selected {
    style: Option<HudStyle>,
    files: String,
    pack: String,
}

#[derive(Default)]
pub(super) struct HudPicker {
    open: bool,
    /// The cvars that select the HUD in use.
    selected: Selected,
    choices: Vec<HudChoice>,
    /// The list is out of date (the settings or the picker just opened).
    stale: bool,
    /// Highlighted choice and the first one the list shows.
    highlight: usize,
    first: usize,
    /// Previews made since the picker opened, by choice.
    previews: Vec<Option<Arc<Ready>>>,
    /// Choices whose preview could not be made.
    failed: Vec<bool>,
    /// The preview being rendered, and for which choice.
    rendering: Option<(usize, Receiver<Option<Ready>>)>,
    /// The choice whose preview is in the texture.
    shown: Option<usize>,
    /// The live HUD that draws SJK's layouts on their pictures, made on the
    /// first one highlighted.
    sample: Option<Box<HudOverlay>>,
}

impl HudPicker {
    pub(super) fn is_open(&self) -> bool {
        self.open
    }

    /// Read the list again when the files are next available.
    pub(super) fn mark_stale(&mut self) {
        self.stale = true;
    }

    /// Take the cvars that select the HUD.
    pub(super) fn read(&mut self, console: &ViewerConsole) {
        self.selected = Selected {
            style: Some(HudStyle::read(Some(console))),
            files: console
                .text_value(FILES_CVAR)
                .unwrap_or_default()
                .to_owned(),
            pack: console.text_value(PACK_CVAR).unwrap_or_default().to_owned(),
        };
    }

    /// Index of the choice in use.
    fn current(&self) -> Option<usize> {
        choices::current(
            &self.choices,
            self.selected.style?,
            &self.selected.files,
            &self.selected.pack,
        )
    }

    /// What the HUD row shows: the HUD in use, by name once the list is
    /// read, else by its style.
    pub(super) fn label(&self) -> String {
        if let Some(index) = self.current() {
            return self.choices[index].label.clone();
        }
        match self.selected.style {
            Some(HudStyle::Classic) => "SJK classic",
            Some(HudStyle::Radial) => "SJK radial",
            _ => "Game HUD",
        }
        .to_owned()
    }

    fn open(&mut self) {
        self.open = true;
        self.stale = true;
        self.clear_previews();
    }

    fn close(&mut self) {
        self.open = false;
        self.sample = None;
        self.clear_previews();
    }

    fn clear_previews(&mut self) {
        self.previews.clear();
        self.failed.clear();
        self.rendering = None;
        self.shown = None;
    }

    fn highlight(&mut self, index: usize) {
        if self.choices.is_empty() {
            return;
        }
        self.highlight = index.min(self.choices.len() - 1);
        if self.highlight < self.first {
            self.first = self.highlight;
        } else if self.highlight >= self.first + LIST_ROWS {
            self.first = self.highlight + 1 - LIST_ROWS;
        }
    }

    fn scroll(&mut self, rows: isize) {
        let last = self.choices.len().saturating_sub(LIST_ROWS);
        self.first = self.first.saturating_add_signed(rows).min(last);
    }
}

impl SettingsMenu {
    /// Open the picker on the HUD in use.
    pub(super) fn open_hud_picker(&mut self, console: &ViewerConsole) {
        self.hud.read(console);
        self.hud.open();
        if let Some(current) = self.hud.current() {
            self.hud.highlight(current);
        }
    }

    /// Use choice `index`: write its cvars and show it on the HUD row.
    fn use_hud(&mut self, console: &mut ViewerConsole, index: usize) {
        if let Some(choice) = self.hud.choices.get(index) {
            choice.apply(console);
        }
        self.refresh(console);
    }

    /// Left or right on the HUD row: the previous or next HUD of the list.
    pub(super) fn step_hud(&mut self, console: &mut ViewerConsole, direction: i32) {
        let count = self.hud.choices.len();
        if count == 0 {
            return;
        }
        let next = match self.hud.current() {
            Some(index) => (index as i32 + direction).rem_euclid(count as i32) as usize,
            None => 0,
        };
        self.use_hud(console, next);
    }

    /// A key while the picker is open.
    pub(super) fn hud_picker_key(
        &mut self,
        key: KeyCode,
        repeat: bool,
        console: &mut ViewerConsole,
    ) {
        let count = self.hud.choices.len();
        let highlight = self.hud.highlight;
        match key {
            // A held Enter or Escape that opened the picker must not act on it.
            KeyCode::Escape | KeyCode::Backspace if !repeat => self.hud.close(),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space if !repeat => {
                if highlight < count {
                    self.use_hud(console, highlight);
                }
                self.hud.close();
            }
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.hud
                    .highlight(highlight.checked_sub(1).unwrap_or(count.saturating_sub(1)));
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.hud.highlight(if highlight + 1 >= count {
                    0
                } else {
                    highlight + 1
                });
            }
            KeyCode::PageUp => self.hud.highlight(highlight.saturating_sub(LIST_ROWS)),
            KeyCode::PageDown => self.hud.highlight(highlight + LIST_ROWS),
            KeyCode::Home => self.hud.highlight(0),
            KeyCode::End => self.hud.highlight(count.saturating_sub(1)),
            _ => {}
        }
    }

    /// Pointer while the picker is open: hover highlights (and previews), a
    /// click uses the HUD, the wheel scrolls the list, the ESC hint closes.
    pub(super) fn hud_picker_pointer(&mut self, event: InputEvent, console: &mut ViewerConsole) {
        let Some(event) = self.ui.pointer(event) else {
            return;
        };
        let Some(token) = event.token else {
            return;
        };
        let slot = usize::from(token);
        let on_row = slot < LIST_ROWS && self.hud.first + slot < self.hud.choices.len();
        match event.kind {
            UiEventKind::Wheel => {
                let notches = event.delta.map_or(0, |delta| -delta.y.signum() as isize);
                self.hud.scroll(notches);
            }
            UiEventKind::HoverEnter | UiEventKind::Hover if on_row => {
                self.hud.highlight(self.hud.first + slot);
            }
            UiEventKind::Activate if token == BACK_TOKEN => self.hud.close(),
            UiEventKind::Activate if on_row => {
                self.use_hud(console, self.hud.first + slot);
                self.hud.close();
            }
            _ => {}
        }
    }

    /// Upload the highlighted HUD's preview again, into another world's
    /// texture.
    pub(crate) fn forget_hud_preview_upload(&mut self) {
        self.hud.shown = None;
    }

    /// Per frame while the settings are up: read the HUD list when it is
    /// stale, collect a finished preview, start the highlighted one's and
    /// hand `upload` the preview to show when it changes.
    pub(crate) fn service_hud_picker(
        &mut self,
        vfs: Option<&Arc<sjk_vfs::VirtualFileSystem>>,
        shaders: &sjk_shader::ShaderCatalog,
        mut upload: impl FnMut(&LevelshotImage),
    ) {
        let Some(vfs) = vfs else {
            return;
        };
        let picker = &mut self.hud;
        if picker.stale {
            picker.stale = false;
            picker.choices = choices::list(vfs);
            picker.clear_previews();
            if picker.open
                && let Some(current) = picker.current()
            {
                picker.highlight(current);
            }
            self.refresh_hud_row();
        }
        let picker = &mut self.hud;
        if !picker.open {
            return;
        }
        let count = picker.choices.len();
        picker.previews.resize_with(count, || None);
        picker.failed.resize(count, false);
        let finished = match &picker.rendering {
            Some((index, receiver)) => match receiver.try_recv() {
                Ok(ready) => Some((*index, ready)),
                Err(mpsc::TryRecvError::Disconnected) => Some((*index, None)),
                Err(mpsc::TryRecvError::Empty) => None,
            },
            None => None,
        };
        if let Some((index, ready)) = finished {
            picker.rendering = None;
            match ready {
                Some(ready) => picker.previews[index] = Some(Arc::new(ready)),
                None => picker.failed[index] = true,
            }
        }
        let wanted = picker.highlight;
        let Some(choice) = picker.choices.get(wanted) else {
            return;
        };
        if let Some(ready) = &picker.previews[wanted] {
            if picker.shown != Some(wanted) {
                upload(&ready.image);
                picker.shown = Some(wanted);
            }
            return;
        }
        if picker.rendering.is_some() || picker.failed[wanted] {
            return;
        }
        let Some(plan) = preview::plan(vfs, shaders, choice) else {
            picker.failed[wanted] = true;
            return;
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let spawned = std::thread::Builder::new()
            .name("HUD preview".to_owned())
            .spawn(move || {
                let preview = preview::render(&plan);
                let _ = sender.send(Some(Ready {
                    image: LevelshotImage::from_rgba(preview.image),
                    texts: preview.texts,
                }));
            });
        match spawned {
            Ok(_) => picker.rendering = Some((wanted, receiver)),
            Err(_) => picker.failed[wanted] = true,
        }
    }

    /// Show the HUD in use on the HUD row.
    fn refresh_hud_row(&mut self) {
        let label = self.hud.label();
        for (row, setting) in self.rows().iter().enumerate() {
            if matches!(setting.kind(), ValueKind::HudPicker)
                && let Some(value) = self.values.get_mut(row)
            {
                value.clone_from(&label);
            }
        }
    }

    /// Draw the open picker as the SJK UI's pop-up card over the darkened
    /// map, whatever the menu style.
    pub(crate) fn append_hud_picker_sjk(
        &mut self,
        target: TextTarget<'_>,
        viewport: [f32; 2],
        reveal: f32,
    ) {
        self.sjk_controls = None;
        self.append_hud_picker(target, viewport, reveal);
    }

    /// Draw the open picker at `reveal` opacity: the list on the card's left,
    /// the highlighted HUD's picture on its right, the keys under the card.
    pub(super) fn append_hud_picker(
        &mut self,
        target: TextTarget<'_>,
        viewport: [f32; 2],
        reveal: f32,
    ) {
        // SJK's layouts are laid out by the live HUD's code, which wants a
        // font to measure with.
        let font = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        let frame = Frame::new(viewport);
        let s = frame.s;
        self.ui.begin_transparent(viewport);
        self.ui.push_opacity(reveal);
        kit::scrim(&mut self.ui, viewport);
        kit::card(&mut self.ui, &frame, CARD);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("HUD look"),
            frame.rect(TEXT_X, CARD[1] + 30.0, TEXT_WIDTH, 52.0),
            40.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "How a match shows your health, armor, Force and ammunition. Hover a name to see it."
            ),
            frame.rect(TEXT_X, CARD[1] + 88.0, TEXT_WIDTH, 26.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        self.hud_list(&frame);
        self.hud_preview(&frame, font);
        self.hud_keys(&frame, CARD[1] + CARD[3] + 28.0);
        self.ui.pop_opacity();
        let focus = self.hud.highlight.saturating_sub(self.hud.first);
        self.ui.finish(focus as u16);
        target.append(&self.ui, viewport);
    }

    /// The visible rows: the highlighted one on the band, the one in use in
    /// gold with its dot; row token `i` is list position `first + i`.
    fn hud_list(&mut self, frame: &Frame) {
        let s = frame.s;
        let picker = &self.hud;
        let (first, highlight, current) = (picker.first, picker.highlight, picker.current());
        let count = picker.choices.len();
        let list = frame.rect(
            TEXT_X - 16.0,
            ROWS_TOP,
            LIST_WIDTH + 32.0,
            ROW * LIST_ROWS as f32,
        );
        self.ui.scroll_region(LIST_SCROLL_TOKEN, list);
        if count == 0 {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Reading the HUDs..."),
                frame.rect(TEXT_X + 8.0, ROWS_TOP, LIST_WIDTH, ROW),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            return;
        }
        let right = TEXT_X + LIST_WIDTH;
        for slot in 0..LIST_ROWS.min(count - first) {
            let index = first + slot;
            let top = ROWS_TOP + slot as f32 * ROW;
            let middle = top + ROW * 0.5;
            let lit = index == highlight;
            let in_use = Some(index) == current;
            let band = [TEXT_X - 16.0, top, LIST_WIDTH + 32.0, ROW];
            if lit {
                kit::band(&mut self.ui, frame, band);
            }
            let label = self.hud.choices[index].label.clone();
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(
                    TEXT_X + 8.0,
                    middle - 15.0,
                    LIST_WIDTH - 8.0 - DOT_ROOM,
                    30.0,
                ),
                23.0 * s,
                match (lit, in_use) {
                    (true, _) => color::TEXT,
                    (false, true) => color::GOLD_BRIGHT,
                    (false, false) => color::alpha(color::TEXT, 0.88),
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            if in_use {
                self.hud_shape(
                    [right - 10.0, middle - 4.0, 8.0, 8.0],
                    color::GOLD_BRIGHT,
                    frame,
                );
            }
            let [x, y, width, height] = band;
            self.ui
                .hit_region(slot as u16, frame.rect(x, y, width, height));
        }
        if count > LIST_ROWS {
            // Where the view lies in the list: a track right of the rows and
            // its thumb.
            let track = [right + 24.0, ROWS_TOP, 4.0, LIST_ROWS as f32 * ROW];
            let thumb = (LIST_ROWS as f32 / count as f32).clamp(0.08, 1.0) * track[3];
            let travel = (track[3] - thumb) * first as f32 / (count - LIST_ROWS) as f32;
            self.hud_shape(track, color::alpha(color::HOLO, 0.12), frame);
            self.hud_shape(
                [track[0], track[1] + travel, track[2], thumb],
                color::alpha(color::HOLO, 0.55),
                frame,
            );
        }
    }

    /// The highlighted HUD's picture, its name and where it comes from. A
    /// game-data HUD is the worker's picture with its text; SJK's layouts are
    /// drawn over the picture's backdrop by the live HUD's code.
    fn hud_preview(&mut self, frame: &Frame, font: &UiFont) {
        let s = frame.s;
        let [x, y, width, height] = PREVIEW;
        let rect = frame.rect(x, y, width, height);
        let radius = 12.0 * s;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius,
            color: color::alpha(color::SPACE, 0.92),
        });
        let picker = &self.hud;
        let Some(choice) = picker.choices.get(picker.highlight).cloned() else {
            self.hud_preview_edge(rect, radius, s);
            return;
        };
        let index = picker.highlight;
        let ready = picker.previews.get(index).cloned().flatten();
        let shown = ready.filter(|_| picker.shown == Some(index));
        let failed = picker.failed.get(index).copied().unwrap_or(false);
        let _ = self
            .ui
            .draw_list_mut()
            .push(DrawCommand::PushRoundedClip { rect, radius });
        if let Some(ready) = &shown {
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect,
                texture: crate::ui_renderer::HUD_PREVIEW_TEXTURE,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
            // The HUD's text in the menu font, where the HUD puts it.
            let zoom = rect.width / preview::SIZE[0] as f32;
            self.ui.set_family(TextFamily::Display);
            for run in &ready.texts {
                let size = run.size * zoom;
                let [r, g, b, a] = run.color;
                let (x, align) = if run.centred {
                    (rect.x + run.x * zoom - rect.width, TextAlign::Center)
                } else {
                    (rect.x + run.x * zoom, TextAlign::Start)
                };
                let width = if run.centred {
                    rect.width * 2.0
                } else {
                    rect.right() - x
                };
                self.ui.text_aligned(
                    &run.text,
                    Rect::new(x, rect.y + run.y * zoom, width, size * 1.25),
                    size * 0.8,
                    Color::new(r, g, b, a),
                    FontWeight::Semibold,
                    0.0,
                    align,
                );
            }
            self.ui.set_family(TextFamily::Body);
        }
        let note = match choice.style {
            HudStyle::Game => match (&shown, failed) {
                (Some(_), _) => None,
                (None, true) => Some("No preview: these files describe no HUD SJK can draw."),
                (None, false) => Some("Drawing the preview..."),
            },
            style => {
                self.sjk_hud_picture(font, rect, style);
                None
            }
        };
        let _ = self.ui.draw_list_mut().push(DrawCommand::PopClip);
        if let Some(note) = note {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{note}"),
                frame.rect(x, y + height * 0.5 - 13.0, width, 26.0),
                17.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        self.hud_preview_edge(rect, radius, s);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", choice.label),
            frame.rect(x, y + height + 18.0, width, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let in_use = self.hud.current() == Some(self.hud.highlight);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}{}",
                choice.origin,
                if in_use { "   \u{b7}   In use" } else { "" }
            ),
            frame.rect(x, y + height + 60.0, width, 24.0),
            17.0 * s,
            if in_use {
                color::GOLD_BRIGHT
            } else {
                color::MUTED
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The picture's thin holo edge.
    fn hud_preview_edge(&mut self, rect: Rect, radius: f32, s: f32) {
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.3),
        });
    }

    /// SJK's layout `style` as a match draws it for the sample player: laid
    /// out by the live HUD for a screen the picture's size, then moved onto
    /// the picture. Its pictures (crosshair, icons) are left out: the
    /// backdrop has its own crosshair, and the HUD's textures are not the
    /// menu's.
    fn sjk_hud_picture(&mut self, font: &UiFont, rect: Rect, style: HudStyle) {
        let look = match style {
            HudStyle::Radial => HudLook::Radial,
            _ => HudLook::Classic,
        };
        let hud = self.hud.sample.get_or_insert_with(|| {
            let mut hud = Box::new(HudOverlay::new());
            hud.preview_values(SAMPLE, SAMPLE_RATIOS);
            hud
        });
        let _ = hud.layout(
            font,
            look,
            [rect.width, rect.height],
            1.0,
            SAMPLE_VISIBILITY,
            SAMPLE_TIME_MS,
        );
        let hud = &**hud;
        self.ui.set_family(TextFamily::Display);
        for command in hud.draw_list().commands() {
            if let DrawCommand::Text {
                rect: line,
                text,
                size,
                color,
                align,
                weight,
                letter_spacing,
                ..
            } = *command
            {
                self.ui.text_aligned(
                    hud.resolve_text(text),
                    Rect::new(rect.x + line.x, rect.y + line.y, line.width, line.height),
                    size,
                    color,
                    weight,
                    letter_spacing,
                    align,
                );
            } else if let Some(moved) = moved(*command, [rect.x, rect.y]) {
                let _ = self.ui.draw_list_mut().push(moved);
            }
        }
        self.ui.set_family(TextFamily::Body);
    }

    /// A small rounded shape over frame rectangle `rect`: the in-use dot, the
    /// scroll track and its thumb.
    fn hud_shape(&mut self, rect: [f32; 4], colour: Color, frame: &Frame) {
        let [x, y, width, height] = rect;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: width.min(height) * 0.5 * frame.s,
            color: colour,
        });
    }

    /// The keys' line under the card, centred, its top at `y`; Esc's cap is
    /// the pointer's way back.
    fn hud_keys(&mut self, frame: &Frame, y: f32) {
        let s = frame.s;
        let width = KEYS
            .iter()
            .map(|(keys, action)| key_hint_width(keys, action, s) + KEY_GAP * s)
            .sum::<f32>()
            - KEY_GAP * s;
        let [centre, top] = frame.point(CARD[0] + CARD[2] * 0.5, y);
        let mut x = centre - width * 0.5;
        for (keys, action) in KEYS {
            let start = x;
            x = key_hint(&mut self.ui, keys, action, x, top, s);
            if keys == ["Esc"] {
                self.ui
                    .hit_region(BACK_TOKEN, Rect::new(start, top, x - start, 24.0 * s));
            }
            x += KEY_GAP * s;
        }
    }
}

/// A shape of the HUD's draw list moved by `offset` window pixels, or `None`
/// for what the picture leaves out: text (drawn in the menu's type), pictures,
/// shaded effects (the strafe aura, a card's finish) and transforms.
fn moved(command: DrawCommand, offset: [f32; 2]) -> Option<DrawCommand> {
    let [dx, dy] = offset;
    let shift = |rect: Rect| Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
    let point = |[x, y]: [f32; 2]| [x + dx, y + dy];
    Some(match command {
        DrawCommand::SolidRect { rect, color } => DrawCommand::SolidRect {
            rect: shift(rect),
            color,
        },
        DrawCommand::RoundedRect {
            rect,
            radius,
            color,
        } => DrawCommand::RoundedRect {
            rect: shift(rect),
            radius,
            color,
        },
        DrawCommand::GradientRect {
            rect,
            radius,
            gradient,
        } => DrawCommand::GradientRect {
            rect: shift(rect),
            radius,
            gradient,
        },
        DrawCommand::Border {
            rect,
            radius,
            width,
            color,
        } => DrawCommand::Border {
            rect: shift(rect),
            radius,
            width,
            color,
        },
        DrawCommand::Arc {
            center,
            radius,
            width,
            start,
            sweep,
            color,
            knockout,
        } => DrawCommand::Arc {
            center: point(center),
            radius,
            width,
            start,
            sweep,
            color,
            knockout: knockout.map(shift),
        },
        DrawCommand::Line {
            from,
            to,
            width,
            color,
        } => DrawCommand::Line {
            from: point(from),
            to: point(to),
            width,
            color,
        },
        DrawCommand::Polygon {
            points,
            count,
            color,
            texture: None,
        } => DrawCommand::Polygon {
            points: points.map(point),
            count,
            color,
            texture: None,
        },
        DrawCommand::PushClip(rect) => DrawCommand::PushClip(shift(rect)),
        DrawCommand::PushRoundedClip { rect, radius } => DrawCommand::PushRoundedClip {
            rect: shift(rect),
            radius,
        },
        DrawCommand::PopClip => DrawCommand::PopClip,
        DrawCommand::PushOpacity(opacity) => DrawCommand::PushOpacity(opacity),
        DrawCommand::PopOpacity => DrawCommand::PopOpacity,
        DrawCommand::Text { .. }
        | DrawCommand::TexturedQuad { .. }
        | DrawCommand::TexturedQuadUv { .. }
        | DrawCommand::TexturedQuadTurned { .. }
        | DrawCommand::Polygon { .. }
        | DrawCommand::StrafeAura { .. }
        | DrawCommand::CardShine { .. }
        | DrawCommand::PushTransform(_)
        | DrawCommand::PopTransform => return None,
    })
}

#[cfg(test)]
impl SettingsMenu {
    /// Open the HUD picker with choice `label` highlighted and wait for its
    /// picture, which it returns as the texture's pixels (snapshots).
    pub(crate) fn hud_picker_for_shot(
        &mut self,
        console: &ViewerConsole,
        vfs: &Arc<sjk_vfs::VirtualFileSystem>,
        label: &str,
    ) -> Option<image::RgbaImage> {
        let shaders = sjk_shader::ShaderCatalog::default();
        self.open_tab(console, Self::tab_index("HUD")?);
        self.service_hud_picker(Some(vfs), &shaders, |_| {});
        self.open_hud_picker(console);
        self.service_hud_picker(Some(vfs), &shaders, |_| {});
        let index = self
            .hud
            .choices
            .iter()
            .position(|choice| choice.label == label)?;
        self.hud.highlight(index);
        let mut picture = None;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while picture.is_none() && std::time::Instant::now() < deadline {
            self.service_hud_picker(Some(vfs), &shaders, |image| {
                let [width, height] = image.size;
                picture = image::RgbaImage::from_raw(width, height, image.levels[0].clone());
            });
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        picture
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu_hud::STYLE_CVAR;
    use crate::menu_hud::choices::tests::installed;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn hud_tab(menu: &mut SettingsMenu, console: &ViewerConsole) -> usize {
        menu.open_tab(console, SettingsMenu::tab_index("HUD").unwrap());
        menu.rows()
            .iter()
            .position(|setting| matches!(setting.kind(), ValueKind::HudPicker))
            .unwrap()
    }

    #[test]
    fn the_hud_row_names_the_hud_and_steps_through_the_list() {
        let (_directory, mut console) = console();
        let vfs = Arc::new(installed());
        let shaders = sjk_shader::ShaderCatalog::default();
        let mut menu = SettingsMenu::new();
        let row = hud_tab(&mut menu, &console);
        // A fresh profile uses the game HUD: the pack installed last.
        menu.service_hud_picker(Some(&vfs), &shaders, |_| {});
        assert_eq!(menu.values[row], "TheRisqe Radial HUD");
        menu.selected = row;
        menu.adjust(&mut console, 1);
        assert_eq!(menu.values[row], "Elegance HUD");
        assert_eq!(console.text_value(FILES_CVAR), Some("ui/elegance_hud.txt"));
        assert_eq!(console.text_value(PACK_CVAR), Some(""));
        // Jedi Knight II, Text only, then SJK's radial HUD, which needs no pack.
        for _ in 0..3 {
            menu.adjust(&mut console, 1);
        }
        assert_eq!(menu.values[row], "SJK radial");
        assert_eq!(console.text_value(STYLE_CVAR), Some("radial"));
        // SJK's classic layout, then round to the first.
        for _ in 0..2 {
            menu.adjust(&mut console, 1);
        }
        assert_eq!(menu.values[row], "Jedi Academy");
        assert_eq!(console.text_value(STYLE_CVAR), Some("game"));
        assert_eq!(console.text_value(PACK_CVAR), Some("assets1.pk3"));
        assert_eq!(console.text_value(FILES_CVAR), Some("ui/jahud.txt"));
        menu.adjust(&mut console, -1);
        assert_eq!(menu.values[row], "SJK classic");
        assert_eq!(console.text_value(STYLE_CVAR), Some("classic"));
    }

    #[test]
    fn the_picker_opens_on_the_hud_in_use_and_previews_what_is_highlighted() {
        let (_directory, mut console) = console();
        console.set_cvar(PACK_CVAR, "JoF_AssetsExtra.pk3");
        let vfs = Arc::new(installed());
        let shaders = sjk_shader::ShaderCatalog::default();
        let mut menu = SettingsMenu::new();
        let row = hud_tab(&mut menu, &console);
        menu.service_hud_picker(Some(&vfs), &shaders, |_| {});
        menu.selected = row;
        menu.open_hud_picker(&console);
        assert!(menu.hud.is_open());
        assert_eq!(menu.hud.highlight, 1, "JoF AssetsExtra is in use");
        // The highlighted HUD's preview renders and is uploaded once.
        let mut uploads = 0;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while menu.hud.shown != Some(1) && std::time::Instant::now() < deadline {
            menu.service_hud_picker(Some(&vfs), &shaders, |image| {
                assert_eq!(image.size, preview::SIZE);
                uploads += 1;
            });
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        menu.service_hud_picker(Some(&vfs), &shaders, |_| uploads += 1);
        assert_eq!(uploads, 1);
        // SJK's layouts get the backdrop, which the live HUD draws them over.
        menu.hud_picker_key(KeyCode::End, false, &mut console);
        let last = menu.hud.highlight;
        assert_eq!(menu.hud.choices[last].style, HudStyle::Classic);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while menu.hud.shown != Some(last) && std::time::Instant::now() < deadline {
            menu.service_hud_picker(Some(&vfs), &shaders, |image| {
                assert_eq!(image.size, preview::SIZE);
            });
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(menu.hud.shown, Some(last));
        // Escape leaves the HUD as it was; Enter uses the highlighted one.
        menu.hud_picker_key(KeyCode::Escape, false, &mut console);
        assert!(!menu.hud.is_open());
        assert_eq!(console.text_value(STYLE_CVAR), Some("game"));
        menu.open_hud_picker(&console);
        menu.service_hud_picker(Some(&vfs), &shaders, |_| {});
        menu.hud_picker_key(KeyCode::ArrowUp, false, &mut console);
        menu.hud_picker_key(KeyCode::Enter, false, &mut console);
        assert!(!menu.hud.is_open());
        assert_eq!(console.text_value(PACK_CVAR), Some("assets1.pk3"));
        assert_eq!(menu.values[row], "Jedi Academy");
    }

    /// Highlighting SJK's radial or classic layout draws it on the picture
    /// with the live HUD's shapes: arcs or bars, inside the picture.
    #[test]
    fn sjk_layouts_are_drawn_on_their_picture() {
        let (_directory, mut console) = console();
        let vfs = Arc::new(installed());
        let shaders = sjk_shader::ShaderCatalog::default();
        let font = crate::text::test_font();
        let viewport = [1920.0, 1080.0];
        let mut menu = SettingsMenu::new();
        hud_tab(&mut menu, &console);
        menu.service_hud_picker(Some(&vfs), &shaders, |_| {});
        menu.open_hud_picker(&console);
        menu.service_hud_picker(Some(&vfs), &shaders, |_| {});
        let picture = Frame::new(viewport).rect(PREVIEW[0], PREVIEW[1], PREVIEW[2], PREVIEW[3]);
        let inside = |point: [f32; 2]| {
            (picture.x..=picture.right()).contains(&point[0])
                && (picture.y..=picture.bottom()).contains(&point[1])
        };
        for (key, style) in [
            (KeyCode::End, HudStyle::Classic),
            (KeyCode::ArrowUp, HudStyle::Radial),
        ] {
            menu.hud_picker_key(key, false, &mut console);
            assert_eq!(menu.hud.choices[menu.hud.highlight].style, style);
            let mut vertices = Vec::new();
            menu.append_hud_picker(TextTarget::Inter(&mut vertices, &font), viewport, 1.0);
            let commands = menu.ui.draw_list().commands();
            let arcs: Vec<_> = commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Arc { center, .. } => Some(*center),
                    _ => None,
                })
                .collect();
            match style {
                HudStyle::Radial => {
                    assert!(!arcs.is_empty());
                    assert!(arcs.iter().all(|&center| inside(center)));
                }
                _ => {
                    assert!(arcs.is_empty());
                    let bars = commands.iter().filter(|command| {
                        matches!(command, DrawCommand::RoundedRect { rect, .. }
                            if inside([rect.x, rect.y]) && rect.width > rect.height * 4.0)
                    });
                    assert!(bars.count() >= 3, "health, armor and Force bars");
                }
            }
        }
        menu.hud_picker_key(KeyCode::Escape, false, &mut console);
        assert!(menu.hud.sample.is_none());
    }
}
