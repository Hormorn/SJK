//! Drawing of the spawn list in the SJK UI's look, in every menu style (as the debug
//! panel): the way back, "Spawn list" and the filter's pill at the top, the tabs under
//! them; the names down the left under their "Jedi Academy" and "Community" headings,
//! the chosen one on a band; on the right the chosen one's kind, its name, the line it
//! runs and the Spawn button; the last line run bottom left and the keys bottom right.
//! Laid out in frame pixels ([`Frame`]), so it holds at every window size. Text goes
//! into the canvas' retained slots, so a steady frame allocates nothing.

use super::catalogue::CHEATS;
use super::pointer::{
    BACK_TOKEN, PANE_WHEEL_TOKEN, ROW_BASE, ROW_LIMIT, RUN_TOKEN, SCROLLBAR_TOKEN, SEARCH_TOKEN,
    TAB_BASE,
};
use super::{CHEAT_TAB, COMMUNITY_HEADING, NPCS, Panel, Row, TABS, VEHICLES, item};
use crate::menu::sjk::top_bar;
use crate::menu::sjk::{Frame, SearchPill, TextTarget, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use crate::sjk_chat_look::Measure;
use crate::text::TextFace;
use sjk_ui::{DrawCommand, FontWeight, Rect, TextAlign};

/// The list (frame pixels): its left edge, width, first row and where it stops, and the
/// step from row to row.
const LIST_X: f32 = 96.0;
const LIST_WIDTH: f32 = 600.0;
const LIST_TOP: f32 = 214.0;
const LIST_BOTTOM: f32 = 950.0;
const ROW: f32 = 44.0;
/// A row's band, a little shorter than its step.
const ROW_BAND: f32 = 40.0;
/// The reading column.
const DETAIL_X: f32 = 790.0;
const DETAIL_WIDTH: f32 = 1_034.0;
const DETAIL_TOP: f32 = 210.0;
const DETAIL_BOTTOM: f32 = 950.0;
/// Sizes: a row's name, the chosen one's name, the column's text.
const ROW_SIZE: f32 = 19.0;
const TITLE_SIZE: f32 = 40.0;
const BODY_SIZE: f32 = 18.0;
/// The keys' line and the status's.
const KEYS_Y: f32 = 992.0;
/// The right edge of the frame's content.
const RIGHT: f32 = 1_824.0;
/// The Spawn button.
const BUTTON_WIDTH: f32 = 220.0;
const BUTTON_HEIGHT: f32 = 52.0;

impl Panel {
    /// Draw the panel over the whole frame, its text to `target`.
    pub(crate) fn append_sjk(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let measure = target.body_measure();
        self.build(&measure, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out on the canvas, its text measured as `measure` draws.
    fn build(&mut self, measure: &Measure<'_>, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.rows = (((LIST_BOTTOM - LIST_TOP) / ROW).floor().max(1.0) as usize).min(ROW_LIMIT);
        self.first = self.first.min(self.visible.len().saturating_sub(self.rows));

        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        let query = self.query[self.tab].as_str();
        let found = self
            .visible
            .iter()
            .filter(|row| matches!(row, Row::Item(_)))
            .count();
        top_bar(
            &mut self.ui,
            &frame,
            "Back",
            BACK_TOKEN,
            "Spawn list",
            Some(SearchPill {
                query,
                active: true,
                prompt: "Type to filter",
                found: (!query.is_empty()).then_some(found),
                token: SEARCH_TOKEN,
            }),
        );
        crate::profile_hub::tabs(&mut self.ui, &frame, &TABS, self.tab, TAB_BASE);
        self.list_view(&frame, measure);
        self.ui.scroll_region(
            PANE_WHEEL_TOKEN,
            frame.rect(
                DETAIL_X,
                DETAIL_TOP,
                DETAIL_WIDTH,
                DETAIL_BOTTOM - DETAIL_TOP,
            ),
        );
        if let Some(index) = self.selected_item() {
            self.detail_view(&frame, measure, index);
        }
        let keys_x = self.keys_view(&frame);
        self.status_view(&frame, measure, keys_x);
        let selected = self.selected.saturating_sub(self.first);
        self.ui
            .finish(ROW_BASE + selected.min(ROW_LIMIT - 1) as u16);
    }

    /// The rows the filter keeps, with the scrollbar beside them; or why there are none.
    fn list_view(&mut self, frame: &Frame, measure: &Measure<'_>) {
        let s = frame.s;
        if self.visible.is_empty() {
            let message = if !self.query[self.tab].is_empty() {
                "Nothing matches. Esc clears the filter."
            } else if self.tab == VEHICLES {
                "The game data defines no vehicles."
            } else {
                "The game data defines no NPCs."
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{message}"),
                frame.rect(LIST_X, LIST_TOP + 8.0, LIST_WIDTH, 28.0),
                BODY_SIZE * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            return;
        }
        let shown = self.first..self.visible.len().min(self.first + self.rows);
        for (slot, position) in shown.enumerate() {
            self.row_view(frame, measure, slot, position);
        }
        if self.visible.len() > self.rows {
            self.ui.scrollbar(
                SCROLLBAR_TOKEN,
                frame.rect(
                    LIST_X + LIST_WIDTH + 16.0,
                    LIST_TOP,
                    4.0,
                    self.rows as f32 * ROW - (ROW - ROW_BAND),
                ),
                self.first,
                self.rows,
                self.visible.len(),
            );
        }
    }

    /// One row: a heading with its rule, or a name (cut with an ellipsis when too long),
    /// the chosen one on a band in gold.
    fn row_view(&mut self, frame: &Frame, measure: &Measure<'_>, slot: usize, position: usize) {
        let s = frame.s;
        let top = LIST_TOP + slot as f32 * ROW;
        let index = match self.visible[position] {
            Row::Heading(label) => {
                kit::heading_in(
                    &mut self.ui,
                    frame,
                    LIST_X + 4.0,
                    top + ROW_BAND * 0.5 + 4.0,
                    LIST_WIDTH - 8.0,
                    label,
                    if label == COMMUNITY_HEADING {
                        color::HOLO
                    } else {
                        color::GOLD
                    },
                );
                return;
            }
            Row::Item(index) => index,
        };
        let token = ROW_BASE + slot.min(ROW_LIMIT - 1) as u16;
        let chosen = position == self.selected;
        let hovered = self.ui.token_hovered(token);
        if chosen {
            kit::band(&mut self.ui, frame, [LIST_X, top, LIST_WIDTH, ROW_BAND]);
        } else if hovered {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(LIST_X, top, LIST_WIDTH, ROW_BAND),
                radius: 10.0 * s,
                color: color::alpha(color::HOLO, 0.045),
            });
        }
        self.ui
            .hit_region(token, frame.rect(LIST_X, top, LIST_WIDTH, ROW_BAND));
        let colour = match (chosen, hovered) {
            (true, _) => color::GOLD_BRIGHT,
            (false, true) => color::TEXT,
            (false, false) => color::alpha(color::TEXT, 0.9),
        };
        let room = LIST_WIDTH - 40.0;
        let rect = frame.rect(LIST_X + 20.0, top + (ROW_BAND - 26.0) * 0.5, room, 26.0);
        let name = item(&self.catalogue, self.tab, index).title;
        let cut = measure.fitting(name, room * s, ROW_SIZE * s, TextFace::Regular);
        let ellipsis = if cut < name.len() { "..." } else { "" };
        let cut = if ellipsis.is_empty() {
            cut
        } else {
            let dots = measure.width("...", ROW_SIZE * s, TextFace::Regular);
            measure.fitting(name, room * s - dots, ROW_SIZE * s, TextFace::Regular)
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}{ellipsis}", &name[..cut]),
            rect,
            ROW_SIZE * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The chosen item in the reading column: its kind, its name, the line it runs (and
    /// what a cheat does), the Spawn button, and that the server decides.
    fn detail_view(&mut self, frame: &Frame, measure: &Measure<'_>, index: usize) {
        let s = frame.s;
        let mut y = DETAIL_TOP;
        let kind = match (self.tab, self.group()) {
            (CHEAT_TAB, _) | (_, None) => "Cheat",
            (NPCS, Some(group)) if index < group.retail => "NPC  -  Jedi Academy",
            (NPCS, Some(_)) => "NPC  -  Community",
            (_, Some(group)) if index < group.retail => "Vehicle  -  Jedi Academy",
            (_, Some(_)) => "Vehicle  -  Community",
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{kind}"),
            frame.rect(DETAIL_X, y, DETAIL_WIDTH, 24.0),
            19.0 * s,
            color::GOLD,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 34.0;
        let shown = item(&self.catalogue, self.tab, index);
        let title = shown.title;
        let title_cut = measure.fitting(
            title,
            DETAIL_WIDTH * s * 0.8,
            TITLE_SIZE * s,
            TextFace::Regular,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!(
                "{}{}",
                &title[..title_cut],
                if title_cut < title.len() { "..." } else { "" }
            ),
            frame.rect(DETAIL_X, y, DETAIL_WIDTH, 52.0),
            TITLE_SIZE * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 92.0;

        if self.tab == CHEAT_TAB {
            kit::heading(
                &mut self.ui,
                frame,
                DETAIL_X,
                y,
                DETAIL_WIDTH,
                "What it does",
            );
            y += 28.0;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", CHEATS[index].about),
                frame.rect(DETAIL_X, y, DETAIL_WIDTH, 28.0),
                BODY_SIZE * s,
                color::alpha(color::TEXT, 0.88),
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 64.0;
        }

        kit::heading(&mut self.ui, frame, DETAIL_X, y, DETAIL_WIDTH, "Runs");
        y += 28.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("/{}{}", shown.prefix, shown.rest),
            frame.rect(DETAIL_X, y, DETAIL_WIDTH, 28.0),
            BODY_SIZE * s,
            color::GOLD_BRIGHT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 64.0;

        let label = if self.tab == CHEAT_TAB {
            "Run"
        } else {
            "Spawn"
        };
        kit::button(
            &mut self.ui,
            frame,
            [DETAIL_X, y, BUTTON_WIDTH, BUTTON_HEIGHT],
            label,
            true,
            true,
            false,
            RUN_TOKEN,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Enter, or click a row, does the same. The list stays open."),
            frame.rect(
                DETAIL_X + BUTTON_WIDTH + 24.0,
                y + (BUTTON_HEIGHT - 24.0) * 0.5,
                DETAIL_WIDTH - BUTTON_WIDTH - 24.0,
                24.0,
            ),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );

        let note_y = DETAIL_BOTTOM - 56.0;
        for (number, note) in [
            "Needs a game with cheats on, such as one started with devmap mp/ffa3.",
            "Elsewhere the server refuses it and says so in the console.",
        ]
        .into_iter()
        .enumerate()
        {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{note}"),
                frame.rect(DETAIL_X, note_y + number as f32 * 26.0, DETAIL_WIDTH, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The keys, right-aligned at the bottom. Returns where they start (window pixels).
    fn keys_view(&mut self, frame: &Frame) -> f32 {
        let s = frame.s;
        let run = if self.tab == CHEAT_TAB {
            "run"
        } else {
            "spawn"
        };
        let back = if self.query[self.tab].is_empty() {
            "back"
        } else {
            "clear"
        };
        let keys: [(&[&str], &str); 4] = [
            (&["Enter"], run),
            (&["Tab"], "list"),
            (&["Up", "Down"], "choose"),
            (&["Esc"], back),
        ];
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(RIGHT, KEYS_Y);
        let start = right - width;
        let mut x = start;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
        start
    }

    /// The last line run, at the bottom left up to the keys at `keys_x`.
    fn status_view(&mut self, frame: &Frame, measure: &Measure<'_>, keys_x: f32) {
        if self.status.is_empty() {
            return;
        }
        let s = frame.s;
        let [x, y] = frame.point(LIST_X, KEYS_Y);
        let room = (keys_x - x - 30.0 * s).max(0.0);
        let status = self.status.as_str();
        let cut = measure.fitting(status, room, 15.0 * s, TextFace::Regular);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}{}",
                &status[..cut],
                if cut < status.len() { "..." } else { "" }
            ),
            Rect::new(x, y, room, 24.0 * s),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }
}
