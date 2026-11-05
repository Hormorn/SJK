//! The whisper composer's recipient (Creo's idea, 04/11/2026). Until then "To <name>"
//! shared the channel row and got what the row's channels, the "Latest" link and the
//! "1/2" count left: with All, JoF and Admin in the row, about six letters of the name,
//! cut without a mark, so a player could not tell whom they were writing to. Now the
//! recipient has a line of its own under the channels, the whole chat's width: the name
//! in its colours, "..." where even that is too narrow. With other players who whispered
//! you, `<` before it and `>` after its count ("1/2") step between them (as the PM
//! button does, and Alt+Left and Alt+Right), and a click on the name lists them all,
//! newest first, in a dark see-through panel right above the name, to pick one.

use super::*;
use crate::chat::interaction::{PM_LIST, PM_NAME, PM_NEXT, PM_PREV};

/// The recipient's line, in 1080p pixels at the composer's scale.
pub(super) const ROW: f32 = 26.0;
/// The most players the list shows, the newest whisperers.
pub(in crate::chat) const LIST_MAX: usize = 8;
/// The line's and the list's text size, in 1080p pixels at the composer's scale.
const SIZE: f32 = 14.0;
/// An arrow's button, in 1080p pixels at the composer's scale.
const ARROW: f32 = 22.0;
/// One of the list's players, in 1080p pixels at the composer's scale.
const LIST_ROW: f32 = 26.0;
/// The list's widest, in 1080p pixels at the composer's scale.
const LIST_WIDTH: f32 = 340.0;
/// Where a name too long for its room is cut.
const CUT: &str = "...";

/// `name` cut to `room` pixels at `size`, with whether it was cut (and so needs
/// [`CUT`] after it, which the room leaves space for).
fn fitted<'a>(name: &'a str, font: &UiFont, room: f32, size: f32) -> (&'a str, bool) {
    let end = layout::fitting_end(name, font, room.max(0.0), size);
    if end == name.len() {
        return (name, false);
    }
    let dots = visible_text_width_face(font, CUT, size / font.height.max(1.0), TextFace::Semibold);
    let end = layout::fitting_end(name, font, (room - dots).max(0.0), size);
    (&name[..end], true)
}

impl ChatOverlay {
    /// Whether there is someone to step to from the composer's recipient: a player who
    /// whispered you other than them.
    pub(in crate::chat) fn pm_steps(
        whisperers: &[ChatTarget],
        recipient: Option<ChatTarget>,
    ) -> bool {
        whisperers.iter().any(|target| Some(*target) != recipient)
    }

    /// The recipient's line at `y`, and the list of whisperers above it while open.
    pub(super) fn build_recipient(&mut self, font: &UiFont, g: &Geometry, y: f32) {
        let Some(input) = &self.input else {
            return;
        };
        let recipient = input.recipient;
        let list_open = input.pm_list;
        let whisperers = self.whisperers();
        let steps = Self::pm_steps(&whisperers, recipient);
        let place = self.pm_place_among(&whisperers);
        let size = SIZE * g.scale;
        let em = size / font.height.max(1.0);
        let measure = |text: &str| visible_text_width_face(font, text, em, TextFace::Semibold);
        let height = ROW * g.scale;
        let arrow = ARROW * g.scale;
        let idle = Color::new(0.786, 0.827, 0.865, 1.0);
        let pointer = self.pointer_at;
        let hovered = |ui: &MenuCanvas, token: u16, rect: Rect| {
            ui.token_hovered(token) && pointer.is_none_or(|at| rect.contains(at))
        };

        // "<", "To ", the name in its own colours, " (encrypted)" in pink, then "1/2" and
        // ">" right after it: at the chat's right edge they were too far away to see.
        let mut count = String::new();
        if let Some((at, of)) = place {
            use std::fmt::Write as _;
            let _ = write!(count, "{at}/{of}");
        }
        let count_width = if count.is_empty() {
            0.0
        } else {
            measure(&count) + 10.0 * g.scale
        };
        let lead = if steps { arrow + 6.0 * g.scale } else { 0.0 };
        let tail = count_width + if steps { arrow + 4.0 * g.scale } else { 0.0 };
        let left = g.left + lead;
        let encrypted = recipient.is_some_and(|target| self.takes_encrypted(target.slot()));
        let (suffix, colour) = if encrypted {
            (" (encrypted)", super::encrypted_pink(1.0))
        } else {
            ("", tint(Channel::Whisper, 1.0))
        };
        let name = recipient
            .and_then(|target| {
                self.roster
                    .display_name(target)
                    .or_else(|| self.roster.name(target))
            })
            .unwrap_or("Player unavailable");
        let to = "To ";
        let to_width = measure(to);
        let suffix_width = if suffix.is_empty() {
            0.0
        } else {
            measure(suffix)
        };
        let room = g.width - lead - tail - to_width - suffix_width;
        let (shown, cut) = fitted(name, font, room, size);
        let name_width = measure(shown) + if cut { measure(CUT) } else { 0.0 };
        let text_y = y + (height - size) * 0.5;
        let name_rect = Rect::new(left, y, to_width + name_width, height);
        // A click on the name lists the players who whispered you.
        let listable = !whisperers.is_empty();
        let name_hovered = listable && {
            self.ui.hit_region(PM_NAME, name_rect);
            hovered(&self.ui, PM_NAME, name_rect)
        };
        self.ui.text_aligned(
            to,
            Rect::new(left, text_y, to_width + 2.0 * g.scale, size),
            size,
            colour,
            FontWeight::Semibold,
            0.0,
            TextAlign::Start,
        );
        self.ui.text_fmt_aligned(
            format_args!("{shown}{}", if cut { CUT } else { "" }),
            Rect::new(left + to_width, text_y, name_width + 2.0 * g.scale, size),
            size,
            colour,
            FontWeight::Semibold,
            0.0,
            TextAlign::Start,
        );
        if !suffix.is_empty() {
            self.ui.text_aligned(
                suffix,
                Rect::new(
                    left + to_width + name_width,
                    text_y,
                    suffix_width + 2.0 * g.scale,
                    size,
                ),
                size,
                colour,
                FontWeight::Semibold,
                0.0,
                TextAlign::Start,
            );
        }
        if name_hovered || list_open {
            self.ui.accent_bar(
                Rect::new(
                    left + to_width,
                    text_y + size + 2.0 * g.scale,
                    name_width,
                    1.0 * g.scale,
                ),
                colour,
            );
        }
        let mut end = left + to_width + name_width + suffix_width;
        if !count.is_empty() {
            let width = count_width - 10.0 * g.scale;
            self.ui.text_aligned(
                &count,
                Rect::new(end + 10.0 * g.scale, text_y, width + 2.0 * g.scale, size),
                size,
                idle,
                FontWeight::Semibold,
                0.0,
                TextAlign::Start,
            );
            end += count_width;
        }
        if steps {
            for (token, label, rect) in [
                (PM_PREV, "<", Rect::new(g.left, y, arrow, height)),
                (
                    PM_NEXT,
                    ">",
                    Rect::new(end + 4.0 * g.scale, y, arrow, height),
                ),
            ] {
                self.ui.hit_region(token, rect);
                let colour = if hovered(&self.ui, token, rect) {
                    tint(Channel::Whisper, 1.0)
                } else {
                    idle
                };
                self.ui.text_aligned(
                    label,
                    Rect::new(rect.x, text_y, rect.width, size),
                    size,
                    colour,
                    FontWeight::Semibold,
                    0.0,
                    TextAlign::Center,
                );
            }
        }
        if list_open && listable {
            // Right above the name, growing up over the channel row and the feed.
            self.build_pm_list(font, g, &whisperers, recipient, left, y);
        }
    }

    /// The players who whispered you, newest first, in a see-through panel from `left`
    /// (kept inside the chat), its bottom at `bottom`; the composer's recipient is marked.
    fn build_pm_list(
        &mut self,
        font: &UiFont,
        g: &Geometry,
        whisperers: &[ChatTarget],
        recipient: Option<ChatTarget>,
        left: f32,
        bottom: f32,
    ) {
        let shown = whisperers.len().min(LIST_MAX);
        let row = LIST_ROW * g.scale;
        let pad = 6.0 * g.scale;
        let size = SIZE * g.scale;
        // As wide as its longest name, dark, see-through and without a frame (Slash,
        // 04/11/2026).
        let em = size / font.height.max(1.0);
        let widest = whisperers
            .iter()
            .take(shown)
            .filter_map(|target| {
                self.roster
                    .display_name(*target)
                    .or_else(|| self.roster.name(*target))
            })
            .map(|name| visible_text_width_face(font, name, em, TextFace::Semibold))
            .fold(0.0_f32, f32::max);
        // A little more than the longest name: measured to the pixel, its last letter
        // could be cut by rounding.
        let width = (widest + 24.0 * g.scale + pad * 2.0)
            .min(LIST_WIDTH * g.scale)
            .min(g.width);
        let panel = Rect::new(
            left.min(g.left + g.width - width).max(g.left),
            bottom - shown as f32 * row - pad * 2.0 - 4.0 * g.scale,
            width,
            shown as f32 * row + pad * 2.0,
        );
        // Text draws over every shape: the channel row's and the feed's words under the
        // panel are left out, or they showed through it (as the sender card does).
        self.ui.cover(panel);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: panel,
            radius: 6.0 * g.scale,
            color: Color::new(0.03, 0.035, 0.05, 0.78),
        });
        let pointer = self.pointer_at;
        for (index, &target) in whisperers.iter().take(shown).enumerate() {
            let token = PM_LIST + index as u16;
            let rect = Rect::new(
                panel.x + pad,
                panel.y + pad + index as f32 * row,
                panel.width - pad * 2.0,
                row,
            );
            self.ui.hit_region(token, rect);
            let current = Some(target) == recipient;
            let hovered =
                self.ui.token_hovered(token) && pointer.is_none_or(|at| rect.contains(at));
            if current || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 4.0 * g.scale,
                    color: Color::new(1.0, 1.0, 1.0, if current { 0.12 } else { 0.07 }),
                });
            }
            let name = self
                .roster
                .display_name(target)
                .or_else(|| self.roster.name(target))
                .unwrap_or("");
            let room = rect.width - 16.0 * g.scale;
            let (name, cut) = fitted(name, font, room, size);
            self.ui.text_fmt_aligned(
                format_args!("{name}{}", if cut { CUT } else { "" }),
                Rect::new(
                    rect.x + 8.0 * g.scale,
                    rect.y + (row - size) * 0.5,
                    room,
                    size,
                ),
                size,
                tint(Channel::Whisper, 1.0),
                FontWeight::Semibold,
                0.0,
                TextAlign::Start,
            );
        }
    }

    /// Open or close the list of the players who whispered you.
    pub(in crate::chat) fn toggle_pm_list(&mut self) {
        if let Some(input) = &mut self.input {
            input.pm_list = !input.pm_list;
        }
    }

    /// Whisper the list's `index`th player and close the list.
    pub(in crate::chat) fn pick_pm(&mut self, index: usize) {
        let Some(target) = self.whisperers().get(index).copied() else {
            return;
        };
        if let Some(input) = &mut self.input {
            input.channel = Channel::Whisper;
            input.recipient = Some(target);
            input.pm_list = false;
            self.notice = "";
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::{InputEvent, PointerButton};
    use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey};

    const VIEW: [f32; 2] = [1920.0, 1080.0];

    /// A chat with All, Team, SJK and Admin in its row and three players who whispered,
    /// the newest (`names`' last) first, its composer open on the newest.
    fn whispered(names: [&str; 3]) -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        chat.admin_server = true;
        let mut game = sjk_protocol::GameState::empty_local(0);
        for (slot, name) in (3usize..).zip(names) {
            game.replace_config_string(1_131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                .expect("a player");
        }
        chat.update_roster(&game);
        let now = std::time::Instant::now();
        for (slot, name) in (3u16..).zip(names) {
            chat.receive_classified(
                sjk_client::ServerEventKind::Chat,
                format!("\u{19}[{name}\u{19}]: hi"),
                Some(slot),
                true,
                now,
            );
        }
        chat.open(false);
        chat.step_pm(1);
        chat
    }

    fn frame(chat: &mut ChatOverlay) {
        chat.build(true, &crate::text::test_font(), VIEW, 1_000);
    }

    fn texts(chat: &mut ChatOverlay) -> Vec<String> {
        frame(chat);
        chat.ui.text_runs().map(str::to_owned).collect()
    }

    fn recipient(chat: &ChatOverlay) -> String {
        let target = chat.input.as_ref().unwrap().recipient.unwrap();
        chat.roster.name(target).unwrap().to_owned()
    }

    fn click(chat: &mut ChatOverlay, token: u16) {
        frame(chat);
        let rect = chat.ui.rect_for(token).expect("a button");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        chat.pointer(InputEvent::PointerMove(at));
        for event in [
            InputEvent::PointerPress {
                position: at,
                button: PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: PointerButton::Primary,
            },
        ] {
            chat.pointer(event);
        }
    }

    #[test]
    fn the_whole_name_shows_on_its_own_line_whatever_the_row_holds() {
        // Creo's screenshot: All, JoF and Admin left "To jof.sl  1/2".
        let mut chat = whispered(["Fox", "Wolf", "jof.slashgibson"]);
        let texts = texts(&mut chat);
        assert!(
            texts.iter().any(|text| text == "jof.slashgibson"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text == "To "));
        assert!(texts.iter().any(|text| text == "1/3"));
    }

    #[test]
    fn a_name_too_long_for_the_line_ends_in_dots() {
        let long = "W".repeat(32);
        let mut chat = whispered(["Fox", "Wolf", &long]);
        chat.options.width = 160.0;
        let texts = texts(&mut chat);
        // The recipient's name is drawn right after its "To ".
        let to = texts.iter().position(|text| text == "To ").expect("To");
        let shown = &texts[to + 1];
        assert!(shown.ends_with(CUT) && shown.len() < long.len(), "{shown}");
    }

    #[test]
    fn the_arrows_and_alt_arrows_step_between_whisperers() {
        let mut chat = whispered(["Fox", "Wolf", "Bear"]);
        assert_eq!(recipient(&chat), "Bear");
        click(&mut chat, PM_NEXT);
        assert_eq!(recipient(&chat), "Wolf");
        click(&mut chat, PM_PREV);
        assert_eq!(recipient(&chat), "Bear");
        chat.modifiers = ModifiersState::ALT;
        chat.edit_key(KeyCode::ArrowLeft, &Key::Named(NamedKey::ArrowLeft), None);
        assert_eq!(recipient(&chat), "Fox", "round to the last");
        chat.edit_key(KeyCode::ArrowRight, &Key::Named(NamedKey::ArrowRight), None);
        assert_eq!(recipient(&chat), "Bear");
        // Without Alt the arrows still move the caret, not the recipient.
        chat.modifiers = ModifiersState::empty();
        chat.edit_key(KeyCode::ArrowRight, &Key::Named(NamedKey::ArrowRight), None);
        assert_eq!(recipient(&chat), "Bear");
    }

    #[test]
    fn a_click_on_the_name_lists_the_whisperers_to_pick_one() {
        let mut chat = whispered(["Fox", "Wolf", "Bear"]);
        click(&mut chat, PM_NAME);
        assert!(chat.input.as_ref().unwrap().pm_list);
        let texts = texts(&mut chat);
        for name in ["Fox", "Wolf", "Bear"] {
            assert!(texts.iter().any(|text| text == name), "{name}: {texts:?}");
        }
        click(&mut chat, PM_LIST + 2);
        assert_eq!(recipient(&chat), "Fox");
        assert!(!chat.input.as_ref().unwrap().pm_list, "picking closes it");
        // Escape closes an open list, and only the next one the composer.
        click(&mut chat, PM_NAME);
        chat.edit_key(KeyCode::Escape, &Key::Named(NamedKey::Escape), None);
        assert!(chat.input.as_ref().is_some_and(|input| !input.pm_list));
        chat.edit_key(KeyCode::Escape, &Key::Named(NamedKey::Escape), None);
        assert!(chat.input.is_none());
    }

    #[test]
    fn one_whisperer_already_chosen_has_no_arrows_nor_list_marks() {
        let mut chat = ChatOverlay::new();
        chat.open(false);
        if let Some(input) = chat.input.as_mut() {
            input.channel = Channel::Whisper;
        }
        frame(&mut chat);
        for token in [PM_PREV, PM_NEXT, PM_NAME] {
            assert!(chat.ui.rect_for(token).is_none(), "{token}");
        }
    }
}
