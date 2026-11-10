//! GIFs in the in-play chat box (`docs/hub-chat.md`, "GIFs from GIPHY"): an SJK chat
//! line whose message links to a GIPHY GIF reads "GIF" in the link's place and shows the
//! GIF under its text, small ([`gif_draw::FEED`], three rows high), with the line's alpha
//! and place, so it slides in, scrolls and fades out with it. The cache, fetching and
//! decoding are [`crate::chat_gifs`]'s, as on the SJK chat page and the docks.
//!
//! Only SJK chat lines: a game server's chat lines never have a GIF. A muted sender's
//! GIF is never asked for (the line shows "Messages hidden on this client" and only
//! [`crate::chat_gifs::show`] fetches), nor an unverified sender's (only verified
//! players, staff and the creator play GIFs: [`crate::chat_gifs::plays_from`]; the link
//! stays as typed), with `cl_sjkChatGifs 0` the lines show their
//! links as text, and a GIF the player hid reads "GIF hidden": each line keeps its text
//! as typed, so a change takes effect on the lines already in the box.

use super::*;
use crate::chat_gifs::{self, GifId, Shown, draw as gif_draw};

/// How the chat box shows a GIF.
pub(super) const LOOK: gif_draw::Look = gif_draw::FEED;

/// How an SJK line words its GIF link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Look {
    /// "GIF", the GIF under the line.
    Gif,
    /// [`chat_gifs::link::HIDDEN_LABEL`]: the player hid it ([`chat_gifs::hidden`]).
    Hidden,
    /// The link as typed: GIFs off, or a source the player does not trust.
    Link,
}

/// An SJK line's GIF, its text as typed and how it is worded now.
pub(super) struct LineGif {
    pub(super) id: GifId,
    typed: String,
    look: Look,
}

/// `typed` with its first GIF link worded as `look` asks.
fn worded(typed: &str, look: Look) -> String {
    let label = match look {
        Look::Gif => chat_gifs::link::LABEL,
        Look::Hidden => chat_gifs::link::HIDDEN_LABEL,
        Look::Link => return typed.to_owned(),
    };
    match chat_gifs::link::find(typed) {
        Some((_, range)) => {
            let mut text = typed.to_owned();
            text.replace_range(range, label);
            text
        }
        None => typed.to_owned(),
    }
}

impl ChatOverlay {
    /// How GIF `id` is worded while GIFs show or not (`gifs_shown`).
    fn look(gifs_shown: bool, id: &GifId) -> Look {
        if !gifs_shown || !chat_gifs::sources::trusted(id) {
            Look::Link
        } else if chat_gifs::hidden::contains(id) {
            Look::Hidden
        } else {
            Look::Gif
        }
    }

    /// `text` with the emojis its marks stand for, while emojis show.
    fn marked(emojis: &emoji::Emojis, on: bool, text: String) -> (String, Vec<u16>) {
        if on {
            emojis.markup(&text)
        } else {
            (text, Vec::new())
        }
    }

    /// An SJK message's `text` as its line shows it, the emojis its marks stand for and
    /// its GIF: the first GIF link worded "GIF" while GIFs show, "GIF hidden" for one the
    /// player hid, else as typed. A sender whose GIFs do not play (`plays` false,
    /// [`chat_gifs::plays_from`]) keeps the link as typed, its GIF never asked for.
    pub(super) fn with_gif(
        &self,
        text: String,
        plays: bool,
    ) -> (String, Vec<u16>, Option<Box<LineGif>>) {
        let Some((id, _)) = chat_gifs::link::find(&text).filter(|_| plays) else {
            let (body, emojis) = Self::marked(&self.emojis, self.options.emojis, text);
            return (body, emojis, None);
        };
        let look = Self::look(self.gifs_shown, &id);
        let (body, emojis) = Self::marked(&self.emojis, self.options.emojis, worded(&text, look));
        let gif = LineGif {
            id,
            typed: text,
            look,
        };
        (body, emojis, Some(Box::new(gif)))
    }

    /// Follow `cl_sjkChatGifs` ([`options::Options::gifs`]), the trusted sources and the
    /// hidden GIFs: the lines whose GIF is worded another way now are worded again and
    /// wrapped again.
    pub(super) fn follow_gif_switch(&mut self) {
        let revision = chat_gifs::shown_revision();
        if self.gifs_shown == self.options.gifs && self.gif_sources_revision == revision {
            return;
        }
        self.gifs_shown = self.options.gifs;
        self.gif_sources_revision = revision;
        let (emojis, on) = (&self.emojis, self.options.emojis);
        for line in &mut self.lines {
            if let Some(gif) = &mut line.gif {
                let look = Self::look(self.gifs_shown, &gif.id);
                if look == gif.look {
                    continue;
                }
                gif.look = look;
                (line.body, line.emojis) = Self::marked(emojis, on, worded(&gif.typed, look));
                line.wrap = layout::Wrapped::default();
            }
        }
    }

    /// What the feed shows under the line at `index` this frame: its GIF while GIFs show,
    /// it is not hidden and its sender is not muted, else nothing. Asks for the GIF the
    /// first time.
    pub(super) fn gif_shown(&self, index: usize) -> Option<Shown> {
        let line = &self.lines[index];
        let gif = line
            .gif
            .as_ref()
            .filter(|gif| !line.muted && Self::look(self.gifs_shown, &gif.id) == Look::Gif)?;
        Some(chat_gifs::show(&gif.id))
    }
}

/// The room (pixels) a GIF block takes under its line in geometry `g`.
pub(super) fn room(g: &layout::Geometry, shown: Option<Shown>) -> f32 {
    shown.map_or(0.0, |shown| gif_draw::room(LOOK, shown) * scale(g))
}

/// The scale [`LOOK`] is drawn at in geometry `g`: its sizes are for rows 18 pixels
/// apart, so the GIF stays three of the box's rows high at any text size.
pub(super) fn scale(g: &layout::Geometry) -> f32 {
    g.row / 18.0
}

#[cfg(test)]
mod tests {
    use crate::chat::ChatOverlay;
    use crate::chat_gifs::{self, GifId};
    use sjk_client::ServerEventKind;
    use sjk_identity::{ChatMessage, ChatState};
    use sjk_ui::{Color, DrawCommand, Rect};
    use std::time::Instant;

    fn message(id: u64, name: &str, text: &str) -> ChatMessage {
        ChatMessage {
            reactions: Default::default(),
            id,
            at: 0,
            key_id: format!("{id:016x}"),
            name: name.to_owned(),
            // Only a verified sender's GIF plays (`chat_gifs::plays_from`).
            verified: true,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: text.to_owned(),
            holocron: None,
            official: None,
            channel: Default::default(),
        }
    }

    /// A chat box that took `messages` from the hub, as they arrive in play.
    fn chat_with(messages: Vec<ChatMessage>, muted: &str, now: Instant) -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        let state = |messages| ChatState {
            messages,
            loaded: Some(1),
            ..ChatState::default()
        };
        chat.sync_sjk(&state(Default::default()), |_| false, now);
        chat.sync_sjk(
            &state(messages.into_iter().collect()),
            |key| key == muted,
            now,
        );
        chat
    }

    /// Every text drawn and its rectangle and colour.
    fn texts(chat: &ChatOverlay) -> Vec<(String, Rect, Color)> {
        chat.ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    rect, text, color, ..
                } => Some((chat.ui.stored_text(*text).to_owned(), *rect, *color)),
                _ => None,
            })
            .collect()
    }

    fn find(texts: &[(String, Rect, Color)], wanted: &str) -> Option<(Rect, Color)> {
        texts
            .iter()
            .find(|(text, ..)| text == wanted)
            .map(|(_, rect, color)| (*rect, *color))
    }

    /// The loading GIF's box (the only rounded box the feed draws).
    fn placeholder(chat: &ChatOverlay) -> Option<(Rect, Color)> {
        chat.ui
            .draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
    }

    /// An unverified sender's GIF link stays as typed and its GIF is never asked for;
    /// staff's plays (Sol, 10/11/2026).
    #[test]
    fn only_verified_senders_gifs_play() {
        let now = Instant::now();
        let unverified = ChatMessage {
            verified: false,
            ..message(1, "Fox", "see https://giphy.com/gifs/FeedGif09Unverified")
        };
        let staff = ChatMessage {
            verified: false,
            staff: true,
            ..message(2, "Sol", "and https://giphy.com/gifs/FeedGif10Staff")
        };
        let mut chat = chat_with(vec![unverified, staff], "", now);
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        let texts = texts(&chat);
        let words: String = texts.iter().map(|(text, ..)| text.as_str()).collect();
        assert!(
            words.contains("giphy.com/gifs/FeedGif09Unverified"),
            "{words}"
        );
        assert!(!chat_gifs::asked_for(
            &GifId::new("FeedGif09Unverified").unwrap()
        ));
        assert!(find(&texts, "and GIF").is_some(), "{words}");
    }

    /// At 1080p and 4K, a GIF line shows "GIF" and its block under the text, three rows
    /// high, inside the chat box's column; the line after it starts below the block.
    #[test]
    fn a_gif_takes_the_room_of_three_rows_under_its_line() {
        for (viewport, size) in [([1920.0, 1080.0], 1.0), ([3840.0, 2160.0], 1.5)] {
            let now = Instant::now();
            let mut chat = chat_with(
                vec![
                    message(1, "Sol", "look https://giphy.com/gifs/wave-FeedGif01Room"),
                    message(2, "Fox", "after"),
                ],
                "",
                now,
            );
            chat.options.font = size;
            chat.build(true, &crate::text::test_font(), viewport, 1_000);
            let g = chat.options.geometry(viewport);
            let texts = texts(&chat);
            let (label, _) = find(&texts, "look GIF").expect("the link reads GIF");
            assert!(texts.iter().all(|(text, ..)| !text.contains("giphy")));
            let (block, _) = placeholder(&chat).expect("the GIF's box");
            let (after, _) = find(&texts, "after").expect("the next line");
            assert!(block.y >= label.y + g.row - 0.5, "{block:?} {label:?}");
            assert!((block.height - 3.0 * g.row).abs() < 0.5, "{block:?}");
            assert!(block.x >= g.left - 0.5 && block.right() <= g.left + g.width + 0.5);
            assert!(after.y >= block.bottom() - 0.5, "{after:?} {block:?}");
            assert!(find(&texts, "Loading GIF").is_some());
            assert!(chat_gifs::asked_for(&GifId::new("FeedGif01Room").unwrap()));
        }
    }

    /// The GIF fades with its line and goes when it does; a new line moves both up.
    #[test]
    fn a_gif_lives_and_fades_with_its_line() {
        let now = Instant::now();
        let mut chat = chat_with(
            vec![message(
                1,
                "Sol",
                "https://media.giphy.com/media/FeedGif02Fade/giphy.gif",
            )],
            "",
            now,
        );
        let font = crate::text::test_font();
        let viewport = [1920.0, 1080.0];
        let received = chat.lines[0].received_ms;
        // Half way through the last two seconds of its ten.
        chat.build(true, &font, viewport, received + 9_000);
        let texts = texts(&chat);
        let (_, line) = find(&texts, "GIF").expect("the line");
        let (_, block) = placeholder(&chat).expect("the GIF's box");
        assert!(line.a > 0.3 && line.a < 0.7, "{line:?}");
        assert!((block.a - 0.07 * line.a).abs() < 1e-4, "{block:?} {line:?}");
        let (_, words) = find(&texts, "Loading GIF").expect("its words");
        assert!((words.a - line.a).abs() < 1e-4, "{words:?} {line:?}");
        // Gone with the line.
        chat.build(true, &font, viewport, received + 10_500);
        assert!(placeholder(&chat).is_none());
        assert!(find(&self::texts(&chat), "GIF").is_none());
        // A newer line pushes the line and its GIF up together.
        let mut chat = chat_with(
            vec![message(1, "Sol", "https://giphy.com/gifs/FeedGif03Move")],
            "",
            now,
        );
        chat.build(true, &font, viewport, received + 1_000);
        let (before, _) = placeholder(&chat).unwrap();
        let (line_before, _) = find(&self::texts(&chat), "GIF").unwrap();
        chat.receive(ServerEventKind::Chat, "a game line".to_owned(), None, now);
        // The move begins with the frame that sees the new line; long after, it has
        // settled.
        chat.build(true, &font, viewport, received + 1_000);
        chat.build(true, &font, viewport, received + 3_000);
        let (after, _) = placeholder(&chat).unwrap();
        let (line_after, _) = find(&self::texts(&chat), "GIF").unwrap();
        assert!(after.y < before.y - 1.0, "{before:?} {after:?}");
        let (game, _) = find(&self::texts(&chat), "a game line").unwrap();
        assert!(game.y >= after.bottom() - 0.5, "{game:?} {after:?}");
        assert!(((before.y - line_before.y) - (after.y - line_after.y)).abs() < 0.01);
    }

    /// A muted sender's GIF is never asked for; GIFs off keep the link as text and ask
    /// for nothing, and the lines follow the switch both ways.
    #[test]
    fn muted_senders_and_gifs_off_show_no_gif() {
        let now = Instant::now();
        let muted = message(1, "Troll", "https://giphy.com/gifs/FeedGif04Muted");
        let key = muted.key_id.clone();
        let mut chat = chat_with(vec![muted], &key, now);
        chat.open_sjk();
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        assert!(!chat_gifs::asked_for(
            &GifId::new("FeedGif04Muted").unwrap()
        ));
        assert!(placeholder(&chat).is_none());

        let link = "see https://giphy.com/gifs/FeedGif05Off";
        let mut chat = ChatOverlay::new();
        chat.options.gifs = false;
        let state = |messages| ChatState {
            messages,
            loaded: Some(1),
            ..ChatState::default()
        };
        chat.sync_sjk(&state(Default::default()), |_| false, now);
        chat.sync_sjk(
            &state([message(1, "Sol", link)].into_iter().collect()),
            |_| false,
            now,
        );
        let font = crate::text::test_font();
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        let words: String = texts(&chat)
            .iter()
            .map(|(text, ..)| text.as_str())
            .collect();
        assert!(words.contains("giphy.com/gifs/FeedGif05Off"), "{words}");
        assert!(placeholder(&chat).is_none());
        assert!(!chat_gifs::asked_for(&GifId::new("FeedGif05Off").unwrap()));
        // Switched on: the same line shows "GIF" and its GIF.
        chat.options.gifs = true;
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        assert!(find(&texts(&chat), "see GIF").is_some());
        assert!(placeholder(&chat).is_some());
        // And off again: the link is back.
        chat.options.gifs = false;
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        let words: String = texts(&chat)
            .iter()
            .map(|(text, ..)| text.as_str())
            .collect();
        assert!(words.contains("giphy.com"), "{words}");
        assert!(placeholder(&chat).is_none());
    }

    /// A game server's chat line keeps its GIPHY link as text and fetches nothing.
    #[test]
    fn a_game_line_keeps_its_link() {
        let mut chat = ChatOverlay::new();
        chat.receive(
            ServerEventKind::Chat,
            "^7Fox^7: ^2https://giphy.com/gifs/FeedGif06Server".to_owned(),
            None,
            Instant::now(),
        );
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        assert!(placeholder(&chat).is_none());
        assert!(!chat_gifs::asked_for(
            &GifId::new("FeedGif06Server").unwrap()
        ));
    }

    /// With the composer open, the history (lines long faded) shows its GIFs too.
    #[test]
    fn the_history_shows_gifs_while_typing() {
        let now = Instant::now();
        let mut chat = chat_with(
            vec![message(1, "Sol", "https://giphy.com/gifs/FeedGif07Typing")],
            "",
            now,
        );
        let font = crate::text::test_font();
        let received = chat.lines[0].received_ms;
        chat.build(true, &font, [1920.0, 1080.0], received + 60_000);
        assert!(placeholder(&chat).is_none(), "faded in play");
        chat.open_sjk();
        chat.build(true, &font, [1920.0, 1080.0], received + 60_000);
        let (block, colour) = placeholder(&chat).expect("in the history");
        assert!((colour.a - 0.07).abs() < 1e-4);
        assert!(block.height > 0.0);
    }

    /// A GIF line too tall for the box with its GIF shows its text without it.
    #[test]
    fn a_line_too_tall_with_its_gif_shows_its_text() {
        let now = Instant::now();
        let mut chat = chat_with(
            vec![message(1, "Sol", "hi https://giphy.com/gifs/FeedGif08Tall")],
            "",
            now,
        );
        // The box's bottom 40 pixels from the top: room for the row, not the GIF.
        chat.options.height = 40.0 / 1080.0 * 480.0;
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
        assert!(find(&texts(&chat), "hi GIF").is_some());
        assert!(placeholder(&chat).is_none());
    }
}
