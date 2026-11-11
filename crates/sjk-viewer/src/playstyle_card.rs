//! The playstyle card (`docs/sjk-ui.md`, "Playstyle card"; Sol's request, 11/11/2026):
//! an SJK pop-up card over the darkened menus asking every player how they like to play
//! ([`crate::playstyle`]). A tick per playstyle, with its badge's icon and what it means,
//! filled from the player's own switches; for a player whose nameplates are off, a tick
//! to turn them on (the badges show only on nameplates, and seeing them means sharing,
//! `cg_nameplate`); and "Don't show this again". Save (gold, Enter) writes the ticks,
//! Not now (Escape) leaves them; either way the card is gone for this start, and for good
//! with "Don't show this again" ticked (`ui_playstyleCard` 1).
//!
//! It shows once a start, on the main menu or the game menu, after the cards before it,
//! in the SJK UI's look in every menu style; the menus under it are not drawn and take
//! no input, as under the other cards.

use crate::hud::nameplate::playstyle_badges::draw_icon;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::playstyle::Playstyle;
use sjk_ui::{Color, DrawCommand, FontWeight, InputEvent, TextAlign, UiEventKind};
use std::sync::atomic::{AtomicBool, Ordering};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// 1: the player ticked "Don't show this again".
pub(crate) const CVAR: &str = "ui_playstyleCard";
/// The nameplates' switch (see and share, `hud::nameplate::see_and_share`).
const NAMEPLATES_CVAR: &str = "cg_nameplate";

const SAVE_TOKEN: u16 = 1;
const LATER_TOKEN: u16 = 2;
/// The ticks: `TICK_TOKEN + n` for playstyle `n`, then the nameplates' and Never's.
const TICK_TOKEN: u16 = 10;
const NAMEPLATES_TOKEN: u16 = 30;
const NEVER_TOKEN: u16 = 31;

/// The card's left edge and width; it is as tall as what it holds, centred.
const CARD_X: f32 = 510.0;
const CARD_WIDTH: f32 = 900.0;
const MARGIN: f32 = 44.0;
const TEXT_X: f32 = CARD_X + MARGIN;
const TEXT_WIDTH: f32 = CARD_WIDTH - MARGIN * 2.0;
/// The explanation's first line under the card's top, its step, and characters a line.
const BODY: f32 = 134.0;
const LINE: f32 = 28.0;
const LINE_CHARS: usize = 98;
/// A playstyle's row: its height, the badge's side, and where the meaning starts.
const ROW: f32 = 50.0;
const BADGE: f32 = 36.0;
const MEANING_X: f32 = 300.0;
/// The gaps before the rows, before the extra ticks and before the buttons.
const ROWS_GAP: f32 = 18.0;
const EXTRA_GAP: f32 = 14.0;
const EXTRA_CHARS: usize = 92;
const FOOT_GAP: f32 = 24.0;
const BUTTON_HEIGHT: f32 = 46.0;
const BUTTON_GAP: f32 = 14.0;
const SAVE_WIDTH: f32 = 200.0;
const LATER_WIDTH: f32 = 150.0;
const KEYS_Y: f32 = 1020.0;
const KEYS_RIGHT: f32 = 1824.0;

const INTRO: &str = "Tick what you enjoy. SJK players see it as small badges over your \
     nameplate, and you see theirs: aim at a player to read their badges. It only says \
     what you are up for and blocks nothing. Change it any time in Settings, by the \
     nameplate rows.";
const NAMEPLATES: &str = "Turn nameplates on: the badges show on them. You then see \
     players' health over their heads, and share yours with SJK players.";
const NEVER: &str = "Don't show this again";

/// What a badge means, under its name.
const fn meaning(style: Playstyle) -> &'static str {
    match style {
        Playstyle::SaberDuels => "Clean 1v1s with the saber",
        Playstyle::ForceDuels => "Full Force duels, every power",
        Playstyle::Ffa => "Hit me any time, the more the merrier",
        Playstyle::Goofing => "Emotes, toys and hanging out, not fights",
        Playstyle::Teaching => "Happy to show moves (or to learn them)",
    }
}

/// Asked this start: the card shows once. The tests' other shots start asked, so it
/// does not cover them ([`reset_for_shot`]).
static ASKED: AtomicBool = AtomicBool::new(cfg!(test));

/// What the player chose on the card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Choice {
    /// Flip playstyle `n`.
    Tick(usize),
    Nameplates,
    Never,
    /// Write the ticks; the card goes.
    Save,
    /// Leave everything; the card goes.
    Later,
}

/// The ticks as the card holds them, filled from the settings when it opens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Ticks {
    pub(crate) styles: [bool; Playstyle::ALL.len()],
    /// The nameplates were off when the card opened: it offers to turn them on.
    pub(crate) offer_nameplates: bool,
    pub(crate) nameplates: bool,
    pub(crate) never: bool,
}

pub(crate) struct Card {
    /// The ticks, once the card has opened this start.
    ticks: Option<Ticks>,
    /// Shown this frame.
    open: bool,
    ui: MenuCanvas,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            ticks: None,
            open: false,
            ui: MenuCanvas::with_capacities(96, 2048, 256),
        }
    }
}

impl Card {
    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// The ticks the card shows, from the settings the first time it opens.
    fn fill(&mut self, read: impl FnOnce() -> Ticks) {
        if self.ticks.is_none() {
            self.ticks = Some(read());
        }
    }

    /// 1 to 5 flip a playstyle, 6 the nameplates (when offered), 0 "Don't show this
    /// again", Enter saves, Escape is Not now; every other key is swallowed while the
    /// card shows.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> Option<(Choice, Ticks)> {
        if event.state != ElementState::Pressed || event.repeat {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };
        self.key(code)
    }

    /// [`Self::handle_key`] for a key code, for the world shots.
    #[cfg(test)]
    pub(crate) fn handle_key_code_for_test(&mut self, code: KeyCode) -> Option<(Choice, Ticks)> {
        self.key(code)
    }

    fn key(&mut self, code: KeyCode) -> Option<(Choice, Ticks)> {
        let choice = match code {
            KeyCode::Digit1 | KeyCode::Numpad1 => Choice::Tick(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => Choice::Tick(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => Choice::Tick(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => Choice::Tick(3),
            KeyCode::Digit5 | KeyCode::Numpad5 => Choice::Tick(4),
            KeyCode::Digit6 | KeyCode::Numpad6 => Choice::Nameplates,
            KeyCode::Digit0 | KeyCode::Numpad0 => Choice::Never,
            KeyCode::Enter | KeyCode::NumpadEnter => Choice::Save,
            KeyCode::Escape => Choice::Later,
            _ => return None,
        };
        self.choose(choice)
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> Option<(Choice, Ticks)> {
        let event = self.ui.pointer(event)?;
        if event.kind != UiEventKind::Activate {
            return None;
        }
        let choice = match event.token {
            Some(SAVE_TOKEN) => Choice::Save,
            Some(LATER_TOKEN) => Choice::Later,
            Some(NAMEPLATES_TOKEN) => Choice::Nameplates,
            Some(NEVER_TOKEN) => Choice::Never,
            Some(token)
                if (TICK_TOKEN..TICK_TOKEN + Playstyle::ALL.len() as u16).contains(&token) =>
            {
                Choice::Tick(usize::from(token - TICK_TOKEN))
            }
            _ => return None,
        };
        self.choose(choice)
    }

    /// A tick flips; Save and Not now hand back the ticks to act on.
    fn choose(&mut self, choice: Choice) -> Option<(Choice, Ticks)> {
        let ticks = self.ticks.as_mut()?;
        match choice {
            Choice::Tick(index) => ticks.styles[index] = !ticks.styles[index],
            Choice::Nameplates if !ticks.offer_nameplates => return None,
            Choice::Nameplates => ticks.nameplates = !ticks.nameplates,
            Choice::Never => ticks.never = !ticks.never,
            Choice::Save | Choice::Later => {}
        }
        Some((choice, *ticks))
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Draw the card over the whole frame.
    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let Some(ticks) = self.ticks else {
            return;
        };
        let frame = Frame::new(viewport);
        let s = frame.s;
        // Lay out from the card's top, then centre it.
        let mut y = BODY;
        let mut lines: Vec<(f32, &str)> = Vec::new();
        for line in wrap(INTRO, LINE_CHARS) {
            lines.push((y, line));
            y += LINE;
        }
        y += ROWS_GAP;
        let rows = y;
        y += ROW * Playstyle::ALL.len() as f32 + EXTRA_GAP;
        // The extra ticks: the nameplates' (when offered) and Never's, each its lines.
        let mut extras: Vec<(f32, u16, bool, Vec<&str>)> = Vec::new();
        if ticks.offer_nameplates {
            let extra: Vec<&str> = wrap(NAMEPLATES, EXTRA_CHARS).collect();
            let next = y + extra.len() as f32 * LINE + EXTRA_GAP;
            extras.push((y, NAMEPLATES_TOKEN, ticks.nameplates, extra));
            y = next;
        }
        extras.push((y, NEVER_TOKEN, ticks.never, vec![NEVER]));
        y += LINE;
        let foot = y + FOOT_GAP;
        let height = foot + BUTTON_HEIGHT + MARGIN;
        let top = ((1080.0 - height) * 0.5).max(20.0);
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        kit::scrim(ui, viewport);
        kit::card(ui, &frame, [CARD_X, top, CARD_WIDTH, height]);
        text(
            ui,
            TextFamily::Display,
            format_args!("New: playstyle badges"),
            frame.rect(TEXT_X, top + 30.0, TEXT_WIDTH, 30.0),
            22.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Display,
            format_args!("How do you like to play?"),
            frame.rect(TEXT_X, top + 66.0, TEXT_WIDTH, 48.0),
            36.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        for (line_y, line) in &lines {
            text(
                ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(TEXT_X, top + line_y, TEXT_WIDTH, LINE),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // A row per playstyle: the tick, the badge as nameplates draw it, its name and
        // what it means.
        for (index, style) in Playstyle::ALL.into_iter().enumerate() {
            let row = top + rows + index as f32 * ROW;
            let middle = row + ROW * 0.5;
            let ticked = ticks.styles[index];
            let after = kit::tick(ui, &frame, TEXT_X, middle, ticked, false);
            let badge = frame.rect(after + 16.0, middle - BADGE * 0.5, BADGE, BADGE);
            let [r, g, b] = style.colour();
            let list = ui.draw_list_mut();
            let _ = list.push(DrawCommand::RoundedRect {
                rect: badge,
                radius: badge.height * 0.5,
                color: Color::new(0.04, 0.05, 0.07, 0.85),
            });
            let _ = list.push(DrawCommand::Border {
                rect: badge,
                radius: badge.height * 0.5,
                width: (2.0 * s).max(1.0),
                color: Color::new(r, g, b, 0.75),
            });
            draw_icon(list, style, badge, Color::new(r, g, b, 1.0));
            let name_x = after + 16.0 + BADGE + 14.0;
            text(
                ui,
                TextFamily::Display,
                format_args!("{}", style.label()),
                frame.rect(name_x, middle - 14.0, MEANING_X - (name_x - TEXT_X), 28.0),
                20.0 * s,
                if ticked {
                    color::GOLD_BRIGHT
                } else {
                    color::TEXT
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                ui,
                TextFamily::Body,
                format_args!("{}", meaning(style)),
                frame.rect(
                    TEXT_X + MEANING_X,
                    middle - 13.0,
                    TEXT_WIDTH - MEANING_X,
                    26.0,
                ),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            ui.hit_region(
                TICK_TOKEN + index as u16,
                frame.rect(TEXT_X - 8.0, row + 2.0, TEXT_WIDTH + 16.0, ROW - 4.0),
            );
        }
        for (extra_top, token, ticked, extra_lines) in &extras {
            let extra_top = top + extra_top;
            let after = kit::tick(ui, &frame, TEXT_X, extra_top + LINE * 0.5, *ticked, false);
            for (line_index, line) in extra_lines.iter().enumerate() {
                text(
                    ui,
                    TextFamily::Body,
                    format_args!("{line}"),
                    frame.rect(
                        after + 14.0,
                        extra_top + line_index as f32 * LINE,
                        TEXT_WIDTH,
                        LINE,
                    ),
                    18.0 * s,
                    if *ticked { color::TEXT } else { color::MUTED },
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            ui.hit_region(
                *token,
                frame.rect(
                    TEXT_X - 8.0,
                    extra_top - 6.0,
                    TEXT_WIDTH + 16.0,
                    extra_lines.len() as f32 * LINE + 12.0,
                ),
            );
        }
        let right = CARD_X + CARD_WIDTH - MARGIN;
        let foot = top + foot;
        kit::button(
            ui,
            &frame,
            [right - SAVE_WIDTH, foot, SAVE_WIDTH, BUTTON_HEIGHT],
            "Save",
            true,
            true,
            false,
            SAVE_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [
                right - SAVE_WIDTH - BUTTON_GAP - LATER_WIDTH,
                foot,
                LATER_WIDTH,
                BUTTON_HEIGHT,
            ],
            "Not now",
            false,
            true,
            false,
            LATER_TOKEN,
        );
        let [right, keys_y] = frame.point(KEYS_RIGHT, KEYS_Y);
        let hints: [(&[&str], &str); 3] = [
            (&["1", "5"], "tick"),
            (&["Enter"], "save"),
            (&["Esc"], "not now"),
        ];
        let width: f32 = hints
            .iter()
            .map(|(keys, label)| key_hint_width(keys, label, s) + 18.0 * s)
            .sum();
        let mut x = right - width;
        for (keys, label) in hints {
            x = key_hint(ui, keys, label, x, keys_y, s) + 18.0 * s;
        }
        ui.finish(SAVE_TOKEN);
        target.append(&self.ui, viewport);
    }
}

impl crate::GpuState {
    /// Whether the card shows this frame: not asked this start and not turned off for
    /// good, the player on a menu or the game menu with nothing else over it. `covered`
    /// is another card or the console.
    pub(crate) fn prepare_playstyle_card(&mut self, covered: bool) -> bool {
        let Some(console) = self.console.as_ref() else {
            self.playstyle_card.set_open(false);
            return false;
        };
        let never = console.integer_cvar(CVAR).unwrap_or(0) != 0;
        let menu_visible = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_visible);
        let open = !never
            && !ASKED.load(Ordering::Relaxed)
            && (menu_visible || self.game_menu)
            && !covered
            && !console.is_open()
            && !self.text_dialog.is_open()
            && self.world_load_task.is_none()
            && self.world_install_task.is_none()
            && !self.pending_map_reload;
        if open {
            self.playstyle_card.fill(|| {
                let nameplates = console.bool_cvar(NAMEPLATES_CVAR).unwrap_or(false);
                Ticks {
                    styles: Playstyle::ALL
                        .map(|style| console.bool_cvar(style.cvar()).unwrap_or(false)),
                    offer_nameplates: !nameplates,
                    nameplates: false,
                    never: false,
                }
            });
        }
        self.playstyle_card.set_open(open);
        open
    }

    /// Draw the card. It darkens everything, so every font batch appended before it is
    /// dropped (text draws above all shapes).
    pub(crate) fn append_playstyle_card(&mut self, viewport: [f32; 2]) {
        self.text_vertices.clear();
        self.classic_text_vertices.clear();
        self.game_fonts.clear_text();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.playstyle_card.append(target, viewport);
    }

    /// Act on the player's choice on the card: Save writes the ticks; Save and Not now
    /// put the card away for this start, and for good with "Don't show this again".
    pub(crate) fn playstyle_card_choice(&mut self, choice: Option<(Choice, Ticks)>) {
        let Some((choice, ticks)) = choice else {
            return;
        };
        if !matches!(choice, Choice::Save | Choice::Later) {
            return;
        }
        ASKED.store(true, Ordering::Relaxed);
        self.playstyle_card.set_open(false);
        let Some(console) = &mut self.console else {
            return;
        };
        if ticks.never {
            console.set_cvar(CVAR, "1");
        }
        if choice == Choice::Save {
            for (style, on) in Playstyle::ALL.into_iter().zip(ticks.styles) {
                console.set_cvar(style.cvar(), if on { "1" } else { "0" });
            }
            if ticks.offer_nameplates && ticks.nameplates {
                console.set_cvar(NAMEPLATES_CVAR, "1");
            }
        }
        crate::log::progress(format_args!(
            "playstyle card: {}",
            if choice == Choice::Save {
                "saved"
            } else {
                "not now"
            }
        ));
    }
}

/// Ask again this start, for the world shots.
#[cfg(test)]
pub(crate) fn reset_for_shot() {
    ASKED.store(false, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Card {
        let mut card = Card::default();
        card.fill(|| Ticks {
            styles: [false, true, false, false, false],
            offer_nameplates: true,
            ..Ticks::default()
        });
        card
    }

    #[test]
    fn keys_flip_the_ticks_and_save_hands_them_back() {
        let mut card = card();
        assert_eq!(card.key(KeyCode::KeyA), None);
        let _ = card.key(KeyCode::Digit1);
        let _ = card.key(KeyCode::Digit2);
        let _ = card.key(KeyCode::Digit6);
        let _ = card.key(KeyCode::Digit0);
        let (choice, ticks) = card.key(KeyCode::Enter).unwrap();
        assert_eq!(choice, Choice::Save);
        assert_eq!(ticks.styles, [true, false, false, false, false]);
        assert!(ticks.nameplates && ticks.never);
        assert_eq!(card.key(KeyCode::Escape).unwrap().0, Choice::Later);
    }

    #[test]
    fn the_nameplates_tick_needs_them_off() {
        let mut card = Card::default();
        card.fill(Ticks::default);
        assert_eq!(card.key(KeyCode::Digit6), None);
    }

    #[test]
    fn the_ticks_are_read_once() {
        let mut card = card();
        card.fill(|| Ticks {
            never: true,
            ..Ticks::default()
        });
        assert!(!card.ticks.unwrap().never, "the first fill stays");
    }

    #[test]
    fn the_cvar_is_defined_and_starts_asking() {
        let directory = tempfile::tempdir().unwrap();
        let console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.integer_cvar(CVAR), Some(0));
    }

    #[test]
    fn every_meaning_fits_its_row() {
        for style in Playstyle::ALL {
            assert!(meaning(style).len() <= 44, "{style:?}");
        }
    }
}
