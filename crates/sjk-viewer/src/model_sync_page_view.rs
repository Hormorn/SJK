//! The Model sync page in the SJK UI (`docs/sjk-ui.md`), over the darkened map as the
//! model viewer: the title and what the page is for, the search with Refresh beside it,
//! the list (each model's name, a gold "Worn" on the one the player wears, where it
//! stands, and its buttons on the right), then what model sync said last, and the keys
//! along the foot.

use super::*;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{FontWeight, TextAlign};

/// The page's column: its left edge and width (frame pixels).
const LEFT_X: f32 = 96.0;
const WIDTH: f32 = 1_728.0;
const RIGHT_X: f32 = LEFT_X + WIDTH;
/// The search pill and Refresh beside it.
const SEARCH_Y: f32 = 196.0;
const SEARCH_WIDTH: f32 = 860.0;
const REFRESH_WIDTH: f32 = 160.0;
/// The list's top and row pitch.
const LIST_Y: f32 = 276.0;
const ROW_HEIGHT: f32 = 52.0;
/// The row's columns: the name, the worn mark, the status.
const NAME_X: f32 = LEFT_X + 24.0;
const NAME_WIDTH: f32 = 470.0;
const WORN_X: f32 = NAME_X + NAME_WIDTH + 10.0;
const STATUS_X: f32 = WORN_X + 100.0;
/// The row's buttons, right-aligned.
const BUTTON_WIDTH: f32 = 150.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_GAP: f32 = 12.0;
const STATUS_WIDTH: f32 =
    RIGHT_X - 12.0 - ROW_BUTTONS as f32 * (BUTTON_WIDTH + BUTTON_GAP) - STATUS_X;
/// What model sync said: its heading and lines.
const SAID_Y: f32 = LIST_Y + ROWS as f32 * ROW_HEIGHT + 44.0;
const SAID_LINE: f32 = 23.0;
/// The keys' line.
const KEYS_Y: f32 = 1_030.0;

/// `line` without its `^N` colour codes, and whether it is a warning (`^3` first).
pub(super) fn plain(line: &str) -> (bool, String) {
    let warning = line.starts_with("^3") || line.starts_with("^1");
    let mut out = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '^' && characters.peek().is_some_and(char::is_ascii_digit) {
            characters.next();
            continue;
        }
        out.push(character);
    }
    (warning, out)
}

impl Page {
    /// The token of the focused stop, for the canvas's focus.
    fn focus_token(&self) -> u16 {
        match self.focus {
            Focus::Search => SEARCH_TOKEN,
            Focus::Row(button) => {
                let slot = self.selected.saturating_sub(self.first);
                (slot * ROW_BUTTONS + button) as u16
            }
            Focus::Refresh => REFRESH_TOKEN,
            Focus::Close => BACK_TOKEN,
        }
    }

    /// Draw the page in the SJK UI.
    pub(crate) fn append_sjk(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        let s = frame.s;
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Model sync"),
            frame.rect(LEFT_X, 60.0, 700.0, 56.0),
            40.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        kit::button(
            &mut self.ui,
            &frame,
            [RIGHT_X - 140.0, 66.0, 140.0, 46.0],
            "Close",
            false,
            true,
            self.focus == Focus::Close,
            BACK_TOKEN,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Share the models you have through the Sith cloud. JoF and Sith verified \
                 players can share, everyone can download; downloads show from the next map."
            ),
            frame.rect(LEFT_X, 124.0, WIDTH, 28.0),
            18.0 * s,
            color::alpha(color::TEXT, 0.86),
            FontWeight::Regular,
            TextAlign::Start,
        );
        let shared = self.rows.iter().filter(|row| row.cloud.is_some()).count();
        if self.can_share {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{} models · {shared} on the Sith cloud · you can share",
                    self.rows.len()
                ),
                frame.rect(LEFT_X, 154.0, WIDTH, 26.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        } else {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{} models · {shared} on the Sith cloud · sharing needs a JoF or Sith \
                     tick on your SJK identity",
                    self.rows.len()
                ),
                frame.rect(LEFT_X, 154.0, WIDTH, 26.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        kit::search(
            &mut self.ui,
            &frame,
            [LEFT_X, SEARCH_Y, SEARCH_WIDTH, 56.0],
            &self.query,
            self.focus == Focus::Search,
            "Type to search models",
            (!self.query.is_empty()).then_some(self.shown.len()),
            SEARCH_TOKEN,
        );
        kit::button(
            &mut self.ui,
            &frame,
            [
                LEFT_X + SEARCH_WIDTH + 20.0,
                SEARCH_Y + 5.0,
                REFRESH_WIDTH,
                46.0,
            ],
            "Refresh",
            false,
            !self.busy,
            self.focus == Focus::Refresh,
            REFRESH_TOKEN,
        );
        let auto_x = LEFT_X + SEARCH_WIDTH + REFRESH_WIDTH + 40.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Download shared models automatically: {} (Settings > Mods)",
                if self.auto_on { "on" } else { "off" }
            ),
            frame.rect(auto_x, SEARCH_Y + 16.0, RIGHT_X - auto_x, 24.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
        self.sjk_rows(&frame);
        self.sjk_said(&frame);
        self.sjk_keys(&frame);
        let focused = self.focus_token();
        self.ui.finish(focused);
        target.append(&self.ui, viewport);
    }

    /// The list's rows on screen, or why there are none.
    fn sjk_rows(&mut self, frame: &Frame) {
        let s = frame.s;
        self.ui.scroll_region(
            WHEEL_TOKEN,
            frame.rect(LEFT_X, LIST_Y, WIDTH, ROWS as f32 * ROW_HEIGHT),
        );
        let row_focus = match self.focus {
            Focus::Row(button) => Some(button),
            _ => None,
        };
        for slot in 0..ROWS {
            let top = LIST_Y + slot as f32 * ROW_HEIGHT;
            // Every slot answers, so a click on an empty one is not the page's backdrop.
            self.ui.hit_region(
                ROW_TOKEN + slot as u16,
                frame.rect(LEFT_X, top, WIDTH, ROW_HEIGHT - 4.0),
            );
            let Some(&index) = self.shown.get(self.first + slot) else {
                continue;
            };
            let chosen = self.first + slot == self.selected;
            if chosen {
                kit::band(&mut self.ui, frame, [LEFT_X, top, WIDTH, ROW_HEIGHT - 4.0]);
            }
            let row = &self.rows[index];
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", row.name),
                frame.rect(NAME_X, top + 9.0, NAME_WIDTH, 28.0),
                19.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            if row.worn {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("Worn"),
                    frame.rect(WORN_X, top + 11.0, 90.0, 24.0),
                    15.0 * s,
                    color::GOLD_BRIGHT,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
            }
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", self.statuses[index]),
                frame.rect(STATUS_X, top + 11.0, STATUS_WIDTH, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let mut buttons = [RowAction::Share; ROW_BUTTONS];
            let mut count = 0;
            for action in self.buttons_of(index) {
                buttons[count] = action;
                count += 1;
            }
            // Download in gold: the one thing a row offers everyone.
            let mut x = RIGHT_X - 12.0 - BUTTON_WIDTH;
            for (button, action) in buttons[..count].iter().enumerate().rev() {
                let primary = *action == RowAction::Download;
                kit::button(
                    &mut self.ui,
                    frame,
                    [
                        x,
                        top + (ROW_HEIGHT - 4.0 - BUTTON_HEIGHT) * 0.5,
                        BUTTON_WIDTH,
                        BUTTON_HEIGHT,
                    ],
                    action.label(),
                    primary,
                    !self.busy,
                    chosen && row_focus == Some(button),
                    (slot * ROW_BUTTONS + button) as u16,
                );
                x -= BUTTON_WIDTH + BUTTON_GAP;
            }
        }
        let empty = if self.rows.is_empty() {
            Some(if self.has_index {
                "No models here: none installed besides the game's own, and nobody shared one yet."
                    .to_owned()
            } else {
                "Asking the Sith cloud for its list...".to_owned()
            })
        } else if self.shown.is_empty() {
            Some(format!("No model matches \"{}\".", self.query))
        } else {
            None
        };
        if let Some(empty) = empty {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{empty}"),
                frame.rect(NAME_X, LIST_Y + 8.0, WIDTH - 48.0, 28.0),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        } else if self.shown.len() > ROWS {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}-{} of {}",
                    self.first + 1,
                    (self.first + ROWS).min(self.shown.len()),
                    self.shown.len()
                ),
                frame.rect(
                    RIGHT_X - 300.0,
                    LIST_Y + ROWS as f32 * ROW_HEIGHT + 2.0,
                    300.0,
                    22.0,
                ),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
    }

    /// What model sync said last, under its heading, with "Working..." while busy.
    fn sjk_said(&mut self, frame: &Frame) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            LEFT_X,
            SAID_Y,
            WIDTH,
            "Model sync says",
        );
        if self.busy {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Working..."),
                frame.rect(RIGHT_X - 200.0, SAID_Y - 12.0, 200.0, 24.0),
                16.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        if self.lines.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Nothing yet. Share, Update and Download say here how they went."),
                frame.rect(LEFT_X, SAID_Y + 18.0, WIDTH, 22.0),
                16.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        for (at, line) in self.lines.iter().enumerate() {
            let (warning, words) = plain(line);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(LEFT_X, SAID_Y + 18.0 + at as f32 * SAID_LINE, WIDTH, 22.0),
                16.0 * s,
                if warning {
                    color::EMBER
                } else {
                    color::alpha(color::TEXT, 0.86)
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The page's keys, right-aligned at the bottom.
    fn sjk_keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let keys: [(&[&str], &str); 5] = [
            (&["Type"], "search"),
            (&["Up", "Down"], "choose"),
            (&["Tab"], "next button"),
            (&["Enter"], "press"),
            (&["Esc"], "close"),
        ];
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(RIGHT_X, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

// The said lines end above the keys, and the status column has room.
const _: () = assert!(SAID_Y + 18.0 + 6.0 * SAID_LINE < KEYS_Y - 6.0);
const _: () = assert!(STATUS_WIDTH >= 400.0);
const _: () = assert!(LEFT_X + SEARCH_WIDTH + REFRESH_WIDTH + 40.0 < RIGHT_X - 400.0);
