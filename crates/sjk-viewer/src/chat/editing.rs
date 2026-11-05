//! Clipboard and pointer editing. No action submits the draft or enters binds.

use super::*;
use crate::console::{
    clipboard,
    line_edit::{Motion, token_at},
};
use sjk_ui::{InputEvent, PointerButton, Rect};
use winit::{
    event::KeyEvent,
    keyboard::{Key, KeyCode, ModifiersState, PhysicalKey},
};

/// Cached glyph positions from the last drawn draft; bounded by the compose budget.
pub(super) struct DraftLayout {
    pub(super) rect: Rect,
    pub(super) stops: [(usize, f32); sjk_client::CHAT_DRAFT_CHARS + 1],
    pub(super) len: usize,
    dragging: bool,
    last_click: Option<(Instant, usize)>,
}

impl Default for DraftLayout {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            stops: [(0, 0.0); sjk_client::CHAT_DRAFT_CHARS + 1],
            len: 0,
            dragging: false,
            last_click: None,
        }
    }
}

impl DraftLayout {
    fn byte_at(&self, x: f32) -> usize {
        let stops = &self.stops[..self.len];
        for pair in stops.windows(2) {
            if x < (pair[0].1 + pair[1].1) * 0.5 {
                return pair[0].0;
            }
        }
        stops.last().map_or(0, |stop| stop.0)
    }
}

/// What Ctrl+C copies in the composer ([`ChatOverlay::copy_choice`]).
#[derive(Debug, PartialEq)]
pub(super) enum Copied<'a> {
    /// The link under the pointer, without its colour codes (`chat/links.rs`).
    Link(String),
    /// The draft's selection, else the whole draft.
    Draft(&'a str),
}

impl ChatOverlay {
    /// What Ctrl+C copies now: the draft's selection when there is one, as in every
    /// field; else the link under the pointer, when it rests on one in the feed; else
    /// the whole draft.
    pub(super) fn copy_choice(&self) -> Copied<'_> {
        let input = self.input.as_ref().expect("composer open");
        if let Some(range) = input.edit.selection(&input.text) {
            return Copied::Draft(&input.text[range]);
        }
        let link = self.link_under_pointer().and_then(|spot| {
            // The line the frame laid out, still in its place.
            let line = self
                .lines
                .get(spot.line)
                .filter(|line| line.received_ms == spot.received_ms)?;
            line.body.get(spot.start..spot.end)
        });
        match link {
            Some(link) => Copied::Link(crate::text::Plain(link).to_string()),
            None => Copied::Draft(&input.text),
        }
    }

    pub(crate) fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub(super) fn edit_shortcut(&mut self, event: &KeyEvent) -> bool {
        if self.input.is_none() {
            return false;
        }
        let control = self.modifiers.control_key() && !self.modifiers.alt_key();
        let letter = match event.text.as_deref() {
            Some("\u{1}") => Some('a'),
            Some("\u{3}") => Some('c'),
            Some("\u{16}") => Some('v'),
            Some("\u{18}") => Some('x'),
            _ if control => match &event.logical_key {
                Key::Character(text) => text.chars().next().map(|c| c.to_ascii_lowercase()),
                _ => None,
            },
            _ => None,
        };
        let letter = if event.physical_key == PhysicalKey::Code(KeyCode::Insert) {
            if self.modifiers.shift_key() {
                Some('v')
            } else if control {
                Some('c')
            } else {
                letter
            }
        } else {
            letter
        };
        if letter == Some('c') {
            match self.copy_choice() {
                Copied::Link(link) => {
                    if clipboard::copy(&link) {
                        self.link_copied_ms = Some(self.millis(Instant::now()));
                    }
                }
                Copied::Draft(text) => {
                    clipboard::copy(text);
                }
            }
            return true;
        }
        let Some(input) = &mut self.input else {
            return false;
        };
        match letter {
            Some('a') => input.edit.select_all(&input.text),
            Some('x') => {
                if let Some(range) = input.edit.selection(&input.text)
                    && clipboard::copy(&input.text[range])
                {
                    input.edit.delete(&mut input.text, Motion::Left);
                }
            }
            Some('v') => {
                if let Some(text) = clipboard::paste() {
                    input.insert(&text);
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn edit_pointer(&mut self, event: InputEvent) -> bool {
        let Some(input) = &mut self.input else {
            return false;
        };
        match event {
            InputEvent::PointerPress {
                position,
                button: PointerButton::Primary,
            } if input.layout.rect.contains(position) && input.layout.len > 0 => {
                let at = input.layout.byte_at(position.x);
                let now = Instant::now();
                let double = input.layout.last_click.is_some_and(|(last, byte)| {
                    byte == at && now.duration_since(last).as_millis() < 350
                });
                if double {
                    input.edit.select(&input.text, token_at(&input.text, at));
                } else {
                    input
                        .edit
                        .place(&input.text, at, self.modifiers.shift_key());
                }
                input.layout.last_click = (!double).then_some((now, at));
                input.layout.dragging = !double;
                self.pressed_action = None;
                true
            }
            InputEvent::PointerMove(position) if input.layout.dragging => {
                input
                    .edit
                    .place(&input.text, input.layout.byte_at(position.x), true);
                true
            }
            InputEvent::PointerRelease {
                button: PointerButton::Primary,
                ..
            } if input.layout.dragging => {
                input.layout.dragging = false;
                true
            }
            InputEvent::PointerLeave => {
                input.layout.dragging = false;
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod link_copy_tests {
    use super::*;
    use sjk_client::ServerEventKind;
    use sjk_ui::Vec2;

    /// The chat box with `text` from the server and the composer open on `draft`, laid
    /// out in a 1080p frame.
    fn chat(text: &str, draft: &str) -> ChatOverlay {
        let font = crate::text::test_font();
        let mut chat = ChatOverlay::new();
        chat.receive(ServerEventKind::Chat, text.to_owned(), None, Instant::now());
        chat.open(false);
        chat.input.as_mut().unwrap().insert(draft);
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        chat
    }

    fn middle(rect: Rect) -> Vec2 {
        Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }

    #[test]
    fn ctrl_c_on_a_link_copies_the_link_without_its_colours() {
        let mut chat = chat("^2look ^5https://sjk.example/^3page^7 now", "my draft");
        let spot = chat.links.first().expect("the link is laid out");
        chat.pointer_at = Some(middle(spot.rect));
        assert_eq!(
            chat.copy_choice(),
            Copied::Link("https://sjk.example/page".to_owned())
        );
        // Off the link, Ctrl+C copies the draft as before.
        chat.pointer_at = Some(Vec2::new(spot.rect.x - 4.0, middle(spot.rect).y));
        assert_eq!(chat.copy_choice(), Copied::Draft("my draft"));
        chat.pointer_at = None;
        assert_eq!(chat.copy_choice(), Copied::Draft("my draft"));
    }

    #[test]
    fn a_selection_in_the_draft_goes_before_the_link() {
        let mut chat = chat("www.sjk.example", "words");
        let spot = chat.links.first().expect("the link is laid out");
        chat.pointer_at = Some(middle(spot.rect));
        let input = chat.input.as_mut().unwrap();
        input.edit.select_all(&input.text);
        assert_eq!(chat.copy_choice(), Copied::Draft("words"));
    }

    #[test]
    fn a_line_gone_from_its_place_copies_nothing_of_it() {
        let mut chat = chat("https://sjk.example", "");
        let spot = chat.links.first().expect("the link is laid out");
        chat.pointer_at = Some(middle(spot.rect));
        // Another line took the index before the next frame.
        chat.lines[spot.line].received_ms += 1;
        assert_eq!(chat.copy_choice(), Copied::Draft(""));
    }

    #[test]
    fn links_are_laid_out_only_while_the_composer_is_open() {
        let font = crate::text::test_font();
        let mut chat = ChatOverlay::new();
        chat.receive(
            ServerEventKind::Chat,
            "https://sjk.example".to_owned(),
            None,
            Instant::now(),
        );
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        assert_eq!(chat.links.first(), None);
    }

    #[test]
    fn a_copied_link_says_so_under_the_field_a_moment() {
        let font = crate::text::test_font();
        let mut chat = chat("https://sjk.example", "");
        chat.link_copied_ms = Some(1_000);
        let notes = |chat: &mut ChatOverlay, ms| {
            chat.build(true, &font, [1920.0, 1080.0], ms);
            chat.ui
                .text_runs()
                .filter(|text| *text == super::super::links::COPIED_NOTE)
                .count()
        };
        assert_eq!(notes(&mut chat, 1_200), 1);
        assert_eq!(notes(&mut chat, 1_000 + super::super::links::COPIED_MS), 0);
    }

    /// The chat box with the composer open and the pointer on a link, lit, "Link
    /// copied" under the field, rendered off screen at 1080p.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn link_copy_shot() {
        crate::world_shot::on_big_stack(|| {
            let Some((mut gpu, _profile)) =
                crate::world_shot::open("maps/mp/duel6.bsp", [1920, 1080], None, &[])
            else {
                return;
            };
            gpu.chat.for_shot = true;
            let now = Instant::now();
            for text in [
                "^7Padawan^7: ^2gg",
                "^7Padawan^7: ^7the map pack is at ^5https://sjk.example/maps/pack-03^7, grab it",
                "^7Padawan^7: also www.sjk.example",
            ] {
                gpu.chat
                    .receive(ServerEventKind::Chat, text.to_owned(), None, now);
            }
            gpu.chat.open(false);
            gpu.chat.input.as_mut().unwrap().insert("thanks");
            let _ = crate::world_shot::frame(&mut gpu, 4);
            let spot = gpu.chat.links.first().expect("the link is laid out");
            gpu.chat.pointer_at = Some(middle(spot.rect));
            assert!(matches!(gpu.chat.copy_choice(), Copied::Link(_)));
            gpu.chat.link_copied_ms = Some(gpu.chat.millis(Instant::now()));
            let path = crate::world_shot::shoot(&mut gpu, 4, "chat-link-copy");
            println!("{}", path.display());
        });
    }
}
