//! The small-card notice (`docs/sjk-ui.md`, "Small graphics cards"): when SJK gave a
//! card under 3.5 GiB lighter graphics at start ([`crate::video_memory::lightened`]), an
//! SJK pop-up card over the darkened main menu says what changed and why, and offers
//! Keep (gold, Enter) or Back to SJK graphics (Backspace), which puts the SJK
//! (recommended) preset, the stock blades and the menu cap back as they were
//! ([`crate::video_memory::undo`]); the graphics reload card then offers to apply them.
//! It shows once a start, after the cards before it, in the SJK UI's look in every menu
//! style; the menus under it are not drawn and take no input, as under the other cards.

use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::video_memory::Lightened;
use sjk_ui::{FontWeight, InputEvent, TextAlign, UiEventKind};
use std::sync::atomic::{AtomicBool, Ordering};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Keep and Back to SJK graphics.
const KEEP_TOKEN: u16 = 1;
const BACK_TOKEN: u16 = 2;

/// The card's left edge, top and width; it is as tall as what it holds.
const CARD_X: f32 = 560.0;
const CARD_TOP: f32 = 340.0;
const CARD_WIDTH: f32 = 800.0;
const MARGIN: f32 = 44.0;
const TEXT_X: f32 = CARD_X + MARGIN;
const TEXT_WIDTH: f32 = CARD_WIDTH - MARGIN * 2.0;
/// The explanation's first line, its step and most lines, and characters a line.
const BODY_TOP: f32 = CARD_TOP + 134.0;
const LINE: f32 = 28.0;
const LINES: usize = 5;
const LINE_CHARS: usize = 88;
/// The buttons along the card's foot, this far under the explanation.
const BUTTONS_GAP: f32 = 30.0;
const BUTTON_HEIGHT: f32 = 46.0;
const BUTTON_GAP: f32 = 14.0;
const KEEP_WIDTH: f32 = 150.0;
const BACK_WIDTH: f32 = 270.0;
/// The keys' line, as on the other screens.
const KEYS_Y: f32 = 992.0;
const KEYS_RIGHT: f32 = 1824.0;

/// The card was answered at this start: it does not show again in a later world.
static ANSWERED: AtomicBool = AtomicBool::new(false);

/// What the player chose on the card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Choice {
    Keep,
    Back,
}

pub(crate) struct Card {
    /// Shown this frame.
    open: bool,
    ui: MenuCanvas,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            open: false,
            ui: MenuCanvas::with_capacities(16, 200, 64),
        }
    }
}

/// What the lighter graphics changed, if the card is still to be answered.
fn pending() -> Option<Lightened> {
    crate::video_memory::lightened()
        .filter(|lightened| lightened.any() && !ANSWERED.load(Ordering::Relaxed))
}

impl Card {
    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Enter, Space or Escape keep the lighter graphics, Backspace goes back; every other
    /// key is swallowed while the card shows.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> Option<Choice> {
        if event.state != ElementState::Pressed || event.repeat {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };
        key(code)
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> Option<Choice> {
        let event = self.ui.pointer(event)?;
        if event.kind != UiEventKind::Activate {
            return None;
        }
        match event.token {
            Some(KEEP_TOKEN) => Some(Choice::Keep),
            Some(BACK_TOKEN) => Some(Choice::Back),
            _ => None,
        }
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Draw the card for `lightened` over the whole frame.
    fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2], lightened: Lightened) {
        let frame = Frame::new(viewport);
        let s = frame.s;
        let explanation = words(lightened);
        let lines: Vec<&str> = wrap(&explanation, LINE_CHARS).take(LINES).collect();
        let body_bottom = BODY_TOP + lines.len() as f32 * LINE;
        let buttons_y = body_bottom + BUTTONS_GAP;
        let height = buttons_y + BUTTON_HEIGHT + MARGIN - CARD_TOP;
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        kit::scrim(ui, viewport);
        kit::card(ui, &frame, [CARD_X, CARD_TOP, CARD_WIDTH, height]);
        text(
            ui,
            TextFamily::Display,
            format_args!("Graphics"),
            frame.rect(TEXT_X, CARD_TOP + 30.0, TEXT_WIDTH, 30.0),
            22.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Display,
            format_args!("Lighter graphics for your card"),
            frame.rect(TEXT_X, CARD_TOP + 66.0, TEXT_WIDTH, 48.0),
            36.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        for (index, line) in lines.iter().enumerate() {
            text(
                ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(TEXT_X, BODY_TOP + index as f32 * LINE, TEXT_WIDTH, LINE),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let right = CARD_X + CARD_WIDTH - MARGIN;
        kit::button(
            ui,
            &frame,
            [right - KEEP_WIDTH, buttons_y, KEEP_WIDTH, BUTTON_HEIGHT],
            "Keep",
            true,
            true,
            false,
            KEEP_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [
                right - KEEP_WIDTH - BUTTON_GAP - BACK_WIDTH,
                buttons_y,
                BACK_WIDTH,
                BUTTON_HEIGHT,
            ],
            "Back to SJK graphics",
            false,
            true,
            false,
            BACK_TOKEN,
        );
        let [right, keys_y] = frame.point(KEYS_RIGHT, KEYS_Y);
        let back = key_hint_width(&["Backspace"], "back to SJK graphics", s);
        let keep = key_hint_width(&["Enter"], "keep", s);
        let x = right - back - keep - 18.0 * s;
        let x = key_hint(ui, &["Enter"], "keep", x, keys_y, s);
        key_hint(
            ui,
            &["Backspace"],
            "back to SJK graphics",
            x + 18.0 * s,
            keys_y,
            s,
        );
        ui.finish(KEEP_TOKEN);
        target.append(&self.ui, viewport);
    }
}

fn key(code: KeyCode) -> Option<Choice> {
    match code {
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::Escape => {
            Some(Choice::Keep)
        }
        KeyCode::Backspace => Some(Choice::Back),
        _ => None,
    }
}

/// What the card says changed, in Settings' words.
fn words(lightened: Lightened) -> String {
    let mut changes = Vec::new();
    if lightened.vanilla {
        changes.push("Vanilla graphics");
    }
    if lightened.stock_blades {
        changes.push("other players' blades drawn stock");
    }
    if lightened.menu_cap {
        changes.push("a 60 FPS cap in the menus");
    }
    let list = match changes.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    format!(
        "Your graphics card has {} MiB of video memory, so SJK starts on lighter settings: \
         {list}{}. Keep them for a steadier game, or go back to Vanilla Plus graphics. \
         Each can be changed in Settings at any time.",
        lightened.mib,
        if lightened.smaller_textures {
            ", with smaller textures"
        } else {
            ""
        }
    )
}

impl crate::GpuState {
    /// Whether the card shows this frame: lighter graphics to tell about, the player on
    /// the main menu with nothing else over it. `covered` is another card or the console.
    pub(crate) fn prepare_small_card_notice(&mut self, covered: bool) -> bool {
        let main_menu = self
            .client_menu
            .as_ref()
            .is_some_and(|menu| menu.is_visible() && menu.on_main_menu());
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        let open = pending().is_some()
            && main_menu
            && !covered
            && !console_open
            && !self.text_dialog.is_open()
            && self.world_load_task.is_none()
            && self.world_install_task.is_none();
        self.small_card_notice.set_open(open);
        open
    }

    /// Draw the card. It darkens everything, so every font batch appended before it is
    /// dropped (text draws above all shapes).
    pub(crate) fn append_small_card_notice(&mut self, viewport: [f32; 2]) {
        let Some(lightened) = pending() else {
            return;
        };
        self.text_vertices.clear();
        self.classic_text_vertices.clear();
        self.game_fonts.clear_text();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.small_card_notice.append(target, viewport, lightened);
    }

    /// Act on the player's choice on the card.
    pub(crate) fn small_card_notice_choice(&mut self, choice: Option<Choice>) {
        let Some(choice) = choice else {
            return;
        };
        let Some(lightened) = pending() else {
            return;
        };
        ANSWERED.store(true, Ordering::Relaxed);
        if choice == Choice::Back
            && let Some(console) = &mut self.console
        {
            crate::video_memory::undo(console, lightened);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_and_escape_keep_backspace_goes_back() {
        assert_eq!(key(KeyCode::Enter), Some(Choice::Keep));
        assert_eq!(key(KeyCode::Escape), Some(Choice::Keep));
        assert_eq!(key(KeyCode::Backspace), Some(Choice::Back));
        assert_eq!(key(KeyCode::KeyA), None);
    }

    #[test]
    fn the_words_name_what_changed() {
        let all = Lightened {
            mib: 3072,
            vanilla: true,
            stock_blades: true,
            menu_cap: true,
            smaller_textures: true,
        };
        let said = words(all);
        assert!(
            said.starts_with("Your graphics card has 3072 MiB"),
            "{said}"
        );
        assert!(
            said.contains("Vanilla graphics, other players' blades drawn stock and a 60 FPS cap"),
            "{said}"
        );
        let blades = words(Lightened {
            mib: 2048,
            stock_blades: true,
            ..Lightened::default()
        });
        assert!(
            blades.contains("lighter settings: other players' blades drawn stock. Keep"),
            "{blades}"
        );
        assert!(said.is_ascii(), "the menu font draws bytes");
    }
}
