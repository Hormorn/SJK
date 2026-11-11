//! The SJK channel (`docs/hub-chat.md`): the chat every SJK player shares through the
//! SJK hub, shown in the feed beside the game server's and typed in the same composer.
//! Its messages come from the identity service's chat state, never from the game
//! server, and what is typed on it goes to the hub, never to the game server.

use super::*;
use sjk_identity::{ChatMessage, ChatState, ReportOutcome};

/// Messages the identity service keeps (`sjk_identity::feed::MESSAGES_KEPT`): while it
/// holds fewer, none has dropped off its end.
const KEPT: usize = sjk_identity::feed::MESSAGES_KEPT;

impl ChatOverlay {
    /// Open the composer on the SJK channel.
    pub(crate) fn open_sjk(&mut self) {
        self.open(false);
        if let Some(input) = &mut self.input {
            input.channel = Channel::Sjk;
        }
    }

    /// A message for the SJK channel: its name and text through the chat rules, as
    /// anything a hub sends is shown, the text without its colour codes, as it is
    /// drawn in the SJK chat's gold ([`crate::sjk_chat_look::message_text`]).
    pub(super) fn push_sjk(&mut self, message: &ChatMessage, muted: bool, now: Instant) {
        // A holocron drop is one sentence with no sender column, in its tier's colour.
        let drop = crate::holocrons::line::words(message);
        // The SJK team's line: "SJK", then its words; the game's chat cannot be clicked,
        // so it names the command that opens it.
        let official = crate::official::words::game_words(message);
        let name = if drop.is_some() {
            String::new()
        } else if official.is_some() {
            crate::official::words::SENDER.to_owned()
        } else {
            sjk_identity::chat::name_for_display(&message.name)
        };
        let body = match (&drop, &official) {
            (Some(words), _) => words.text.clone(),
            (None, Some(words)) => words.clone(),
            (None, None) => crate::sjk_chat_look::message_text(&message.text),
        };
        // A message's GIPHY link reads "GIF", its GIF under it (`chat/feed_gif.rs`).
        // Only a verified sender's (or staff's) GIF plays (`chat_gifs::plays_from`).
        let plays = crate::chat_gifs::plays_from(message.verified, message.staff, message.creator);
        let (body, emojis, gif) = if drop.is_none() && official.is_none() {
            self.with_gif(body, plays)
        } else {
            (body, Vec::new(), None)
        };
        let loud = self.sjk_line_loud(&message.key_id, official.is_some());
        if self.lines.len() == HISTORY_LIMIT {
            self.lines.pop_front();
        }
        self.lines.push_back(ChatLine {
            name,
            body,
            sender: None,
            channel: Channel::of_hub(message.channel),
            server_private: false,
            received_ms: self.millis(now),
            muted,
            emojis,
            wrap: layout::Wrapped::default(),
            y: None,
            hub: Some(HubLine {
                id: message.id,
                verified: message.verified && drop.is_none(),
                tick: if drop.is_none() {
                    message.tick
                } else {
                    sjk_identity::Tick::None
                },
                staff: message.staff,
                creator: message.creator,
                trust: message.trust.filter(|_| drop.is_none()),
                emoji: sjk_identity::name_emoji::for_display(&message.emoji).to_owned(),
                client: super::client_icons::find(&message.client),
                in_all: false,
                key_id: message.key_id.clone(),
                prize: drop.as_ref().and_then(|words| words.prize),
                tier: drop.map(|words| words.tier),
                official: official.is_some(),
            }),
            gif,
            encrypted: false,
            verified: false,
            recipient: None,
            loud,
            local: false,
            asked: false,
        });
        if self.is_typing() && self.scroll > 0 && (loud || !self.options.quiet) {
            self.scroll = (self.scroll + 1).min(self.lines.len().saturating_sub(1));
            self.unread = (self.unread + 1).min(HISTORY_LIMIT);
        }
    }

    /// Put the line a holocron drop of `tier` makes into the feed, for `debug_holocron`
    /// (as the player's own drop). The real lines come from the hub's feed
    /// ([`Self::sync_sjk`]).
    pub(crate) fn rehearse_holocron(&mut self, tier: &str, now: Instant) {
        let message = ChatMessage {
            reactions: Default::default(),
            id: u64::MAX,
            at: 0,
            key_id: String::new(),
            name: String::new(),
            verified: false,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: String::new(),
            holocron: Some(sjk_identity::DropMark {
                tier: tier.to_owned(),
                own: true,
                find: None,
            }),
            official: None,
            channel: Default::default(),
        };
        self.push_sjk(&message, false, now);
    }

    /// Put a made-up grand find of `skin` from a `tier` holocron or crystal into the feed,
    /// as another player's, for `debug_find` (`chat/find_blade.rs`).
    pub(crate) fn rehearse_find(
        &mut self,
        tier: &'static crate::holocrons::Tier,
        skin: &'static crate::unlockables::Unlockable,
        now: Instant,
    ) {
        let message = ChatMessage {
            reactions: Default::default(),
            id: u64::MAX,
            at: 0,
            key_id: String::new(),
            name: "^1Warrior ^7SmoKeY".to_owned(),
            verified: true,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: String::new(),
            holocron: Some(sjk_identity::DropMark {
                tier: tier.id.to_owned(),
                own: false,
                find: Some(sjk_identity::FindMark {
                    shader: true,
                    name: skin.name.to_owned(),
                    rarity: skin.tier.label().to_lowercase(),
                    finish: String::new(),
                    card: String::new(),
                    skin: skin.id.to_owned(),
                }),
            }),
            official: None,
            channel: Default::default(),
        };
        self.push_sjk(&message, false, now);
    }

    /// Put the SJK team's line `official` into the feed, for `debug_announce` and
    /// `debug_reward`. The real lines come from the hub's feed ([`Self::sync_sjk`]).
    pub(crate) fn rehearse_official(&mut self, official: sjk_identity::Official, now: Instant) {
        let message = ChatMessage {
            reactions: Default::default(),
            id: u64::MAX,
            at: 0,
            key_id: String::new(),
            name: crate::official::words::SENDER.to_owned(),
            verified: false,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: String::new(),
            holocron: None,
            official: Some(official),
            channel: Default::default(),
        };
        self.push_sjk(&message, false, now);
    }

    /// Whether the SJK chat's `mark` (its revision, outcome serial and the mutes'
    /// revision) differs from the one the feed last followed; it is the one followed
    /// from now on.
    pub(crate) fn sjk_changed(&mut self, mark: (u64, u64, u64)) -> bool {
        let changed = self.sjk_mark != Some(mark);
        self.sjk_mark = Some(mark);
        changed
    }

    /// Follow `cl_sjkChatFeed`. Turning it off removes SJK messages already in the
    /// game chat immediately; repeated frame calls do no further history scan.
    pub(crate) fn set_sjk_feed_visible(&mut self, visible: bool) {
        if self.sjk_feed_visible == visible {
            return;
        }
        self.sjk_feed_visible = visible;
        if !visible {
            // The clan chats show only here, so they stay.
            self.lines.retain(|line| line.channel != Channel::Sjk);
        }
    }

    /// Say in the feed why the hub refused what was sent, once per outcome. Shown even
    /// with the SJK feed off: the in-game composer has no other place to say it.
    pub(crate) fn sjk_outcome(&mut self, outcome: Option<&ReportOutcome>, now: Instant) {
        let Some(outcome) = outcome else { return };
        if self.sjk_outcome == outcome.serial {
            return;
        }
        self.sjk_outcome = outcome.serial;
        if !outcome.sent {
            self.notice(format!("^3SJK chat: {}", outcome.message), now);
        }
    }

    /// Bring the feed in line with the SJK chat `state`: new messages join it, ones
    /// staff deleted leave it, and lines follow the mutes (`muted` says whether the
    /// player muted a key). The backlog a hub sends when the reading starts is only
    /// marked, never replayed over the game: nothing is marked before the hub's first
    /// answer, and a new reading (`ChatState::loaded`) marks again.
    ///
    /// Returns the id the feed had seen when messages newer than it joined the feed now
    /// (those with a higher id in `state`), `None` when nothing new joined: a marked
    /// backlog is not news ([`worth_a_sound`] picks the ones worth a sound).
    #[cfg(test)]
    pub(crate) fn sync_sjk(
        &mut self,
        state: &ChatState,
        muted: impl Fn(&str) -> bool,
        now: Instant,
    ) -> Option<u64> {
        self.sync_sjk_in_feed(state, muted, now, true)
    }

    /// [`Self::sync_sjk`], with live messages omitted from the game chat while
    /// `feed` is false. Their ids are still consumed, so turning the feed back on
    /// never replays messages received while it was hidden.
    pub(crate) fn sync_sjk_in_feed(
        &mut self,
        state: &ChatState,
        muted: impl Fn(&str) -> bool,
        now: Instant,
        feed: bool,
    ) -> Option<u64> {
        for line in &mut self.lines {
            if let Some(hub) = &line.hub {
                line.muted = muted(&hub.key_id);
            }
        }
        let Some(epoch) = state.loaded else {
            return None;
        };
        let messages = &state.messages;
        let newest = messages
            .back()
            .map_or(0, |message| message.id)
            .max(state.clan.back().map_or(0, |message| message.id));
        if self.sjk_epoch != Some(epoch) {
            self.sjk_epoch = Some(epoch);
            self.sjk_seen = Some(newest);
            return None;
        }
        let seen = match self.sjk_seen {
            // A first look, or a hub whose ids began again: mark, show nothing.
            None => {
                self.sjk_seen = Some(newest);
                return None;
            }
            Some(seen) if newest < seen => {
                self.sjk_seen = Some(newest);
                return None;
            }
            Some(seen) => seen,
        };
        // A shown message the chat should hold but does not was deleted by staff (or
        // its clan chat was left).
        let lowest = |held: &std::collections::VecDeque<ChatMessage>| {
            if held.len() >= KEPT {
                held.front().map_or(0, |message| message.id)
            } else {
                0
            }
        };
        let (sjk_lowest, clan_lowest) = (lowest(messages), lowest(&state.clan));
        self.lines.retain(|line| match &line.hub {
            Some(hub)
                if line.channel == Channel::Sjk && hub.id >= sjk_lowest && hub.id <= newest =>
            {
                messages.iter().any(|message| message.id == hub.id)
            }
            Some(hub) if hub.id >= clan_lowest && hub.id <= newest => {
                state.clan.iter().any(|message| message.id == hub.id)
            }
            _ => true,
        });
        for message in merged(state).filter(|message| message.id > seen) {
            let muted = muted(&message.key_id);
            // A muted player's drops are not shown, rather than hidden behind a note;
            // a drop of a tier this client does not know is left out.
            if message.holocron.is_some() && (muted || crate::holocrons::line::unknown(message)) {
                continue;
            }
            if feed {
                self.push_sjk(message, muted, now);
            }
            self.note_official(message);
        }
        // An announcement the service put at the chat's end holds the id of the message
        // before it, which the feed had seen: it joins once, by its own number.
        for message in messages.iter().filter(|message| message.id <= seen) {
            if let Some(sjk_identity::Official::Announcement(shown)) = &message.official
                && !self.sjk_officials.contains(&shown.id)
            {
                if feed {
                    self.push_sjk(message, false, now);
                }
                self.note_official(message);
            }
        }
        self.sjk_seen = Some(newest);
        (newest > seen).then_some(seen)
    }

    /// Remember that the feed showed `message`'s announcement, if it is one.
    fn note_official(&mut self, message: &ChatMessage) {
        if let Some(sjk_identity::Official::Announcement(shown)) = &message.official
            && !self.sjk_officials.contains(&shown.id)
        {
            if self.sjk_officials.len() >= 64 {
                self.sjk_officials.remove(0);
            }
            self.sjk_officials.push(shown.id);
        }
    }

    /// Whether the SJK chat's sound may play at `now`: at most once in
    /// [`SOUND_GAP`], so a burst of messages plays it once. It counts as played.
    pub(crate) fn sjk_sound_due(&mut self, now: Instant) -> bool {
        if self
            .sjk_sound_at
            .is_some_and(|at| now.saturating_duration_since(at) < SOUND_GAP)
        {
            return false;
        }
        self.sjk_sound_at = Some(now);
        true
    }
}

/// The SJK chat's messages and the clan chats' (`ChatState::clan`), in the feed's order:
/// both are in it already, so they merge without sorting or allocating.
pub(crate) fn merged(state: &ChatState) -> impl Iterator<Item = &ChatMessage> {
    let mut sjk = state.messages.iter().peekable();
    let mut clan = state.clan.iter().peekable();
    std::iter::from_fn(move || match (sjk.peek(), clan.peek()) {
        (Some(a), Some(b)) if b.id < a.id => clan.next(),
        (Some(_), _) => sjk.next(),
        (None, _) => clan.next(),
    })
}

/// The shortest time between two of the SJK chat's sounds.
pub(crate) const SOUND_GAP: std::time::Duration = std::time::Duration::from_secs(1);

/// Whether `messages` hold one newer than `after` worth the SJK chat's sound: a message
/// (not a holocron drop) not muted on this PC (`muted`). The player's own messages
/// count (Sol's request, 11/10/2026): one joins the feed once the hub took it, so its
/// sound says it went out; a refused one never joins and plays nothing.
pub(crate) fn worth_a_sound<'a>(
    messages: impl IntoIterator<Item = &'a ChatMessage>,
    after: u64,
    muted: impl Fn(&str) -> bool,
) -> bool {
    messages.into_iter().any(|message| {
        message.id > after
            && message.holocron.is_none()
            && message.official.is_none()
            && !muted(&message.key_id)
    })
}

/// Console text for the live SJK messages newer than `after`. The initial hub
/// backlog never supplies an `after` ([`ChatOverlay::sync_sjk`]), and a locally
/// muted sender stays out of the console just as they stay out of the visible feed.
/// The console cannot draw the feed's emblem, tick, emoji or exact gold, so its
/// plain-text substitute starts with a yellow `SJK` tag.
pub(crate) fn console_lines<'a>(
    messages: impl IntoIterator<Item = &'a ChatMessage>,
    after: u64,
    muted: impl Fn(&str) -> bool,
) -> Vec<String> {
    messages
        .into_iter()
        .filter(|message| message.id > after && !muted(&message.key_id))
        .filter_map(|message| {
            let shown = crate::sjk_chat_look::shown(message)?;
            // A clan chat's line says which, in its own colour code.
            let tag = match message.channel {
                sjk_identity::HubChannel::Jof => "^5JoF",
                sjk_identity::HubChannel::Sith => "^1S",
                _ => "^3SJK",
            };
            Some(if shown.name.is_empty() {
                format!("* {tag}^7: {}", shown.text)
            } else {
                format!("* {tag}^7 {}^7: {}", shown.name, shown.text)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::ChatState;
    use std::collections::VecDeque;
    use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, SmolStr};

    fn message(id: u64, text: &str) -> ChatMessage {
        ChatMessage {
            reactions: Default::default(),
            id,
            at: 0,
            key_id: format!("{id:016x}"),
            name: "^2Sol".to_owned(),
            verified: id.is_multiple_of(2),
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

    /// The chat as the service holds it, the hub's first answer in.
    fn hub(messages: &[(u64, &str)]) -> ChatState {
        loaded(
            1,
            messages
                .iter()
                .map(|(id, text)| message(*id, text))
                .collect(),
        )
    }

    fn loaded(epoch: u64, messages: VecDeque<ChatMessage>) -> ChatState {
        ChatState {
            messages,
            loaded: Some(epoch),
            ..ChatState::default()
        }
    }

    fn type_text(chat: &mut ChatOverlay, text: &str) {
        for c in text.chars() {
            let s = c.to_string();
            chat.edit_key(KeyCode::KeyA, &Key::Character(SmolStr::new(&s)), Some(&s));
        }
    }

    fn enter(chat: &mut ChatOverlay) -> ChatInputResult {
        chat.edit_key(KeyCode::Enter, &Key::Named(NamedKey::Enter), Some("\r"))
    }

    fn tab(chat: &mut ChatOverlay) {
        chat.edit_key(KeyCode::Tab, &Key::Named(NamedKey::Tab), Some("\t"));
    }

    fn channel(chat: &ChatOverlay) -> Channel {
        chat.input.as_ref().expect("composer open").channel
    }

    fn bodies(chat: &ChatOverlay) -> Vec<&str> {
        chat.lines.iter().map(|line| line.body.as_str()).collect()
    }

    #[test]
    fn clan_chat_messages_join_the_feed_labelled_and_in_order() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let mut state = hub(&[(1, "old")]);
        state.clan.push_back(ChatMessage {
            channel: sjk_identity::HubChannel::Jof,
            ..message(2, "old clan")
        });
        // The backlog is marked, not replayed.
        assert_eq!(chat.sync_sjk(&state, |_| false, now), None);
        state.messages.push_back(message(4, "sjk news"));
        state.clan.push_back(ChatMessage {
            channel: sjk_identity::HubChannel::Sith,
            ..message(3, "sith news")
        });
        assert_eq!(chat.sync_sjk(&state, |_| false, now), Some(2));
        assert_eq!(bodies(&chat), ["sith news", "sjk news"]);
        assert!(chat.lines[0].channel == Channel::Sith);
        assert_eq!(super::super::tabs::label(chat.lines[0].channel), "S");
        assert!(chat.lines[1].channel == Channel::Sjk);
        // Console lines say which chat.
        let console = console_lines(merged(&state), 2, |_| false);
        assert!(console[0].starts_with("* ^1S^7 "), "{console:?}");
        assert!(console[1].starts_with("* ^3SJK^7 "), "{console:?}");
        // Hiding SJK chat in the game's feed leaves the clan chats there.
        chat.set_sjk_feed_visible(false);
        assert_eq!(bodies(&chat), ["sith news"]);
        // Deleted by staff: the clan line leaves too.
        state.clan.pop_back();
        state.messages.push_back(message(5, "later"));
        chat.set_sjk_feed_visible(true);
        chat.sync_sjk(&state, |_| false, now);
        assert_eq!(bodies(&chat), ["later"]);
    }

    #[test]
    fn tab_cycles_global_team_sjk() {
        let mut chat = ChatOverlay::new();
        chat.open(false);
        tab(&mut chat);
        assert!(channel(&chat) == Channel::Team);
        tab(&mut chat);
        assert!(channel(&chat) == Channel::Sjk);
        tab(&mut chat);
        assert!(channel(&chat) == Channel::Global);
        chat.activate(interaction::SJK);
        assert!(channel(&chat) == Channel::Sjk);
    }

    fn arrow(chat: &mut ChatOverlay, key: KeyCode, named: NamedKey) {
        chat.edit_key(key, &Key::Named(named), None);
    }

    #[test]
    fn the_arrows_walk_the_chat_buttons_while_nothing_is_typed() {
        let mut chat = ChatOverlay::new();
        chat.open(false);
        arrow(&mut chat, KeyCode::ArrowRight, NamedKey::ArrowRight);
        assert!(channel(&chat) == Channel::Team);
        arrow(&mut chat, KeyCode::ArrowLeft, NamedKey::ArrowLeft);
        assert!(channel(&chat) == Channel::Global);
        // Round to the last button and back.
        arrow(&mut chat, KeyCode::ArrowLeft, NamedKey::ArrowLeft);
        assert!(channel(&chat) == Channel::Sjk);
        arrow(&mut chat, KeyCode::ArrowRight, NamedKey::ArrowRight);
        assert!(channel(&chat) == Channel::Global);
        // A held modifier leaves the buttons alone.
        chat.modifiers = ModifiersState::SHIFT;
        arrow(&mut chat, KeyCode::ArrowRight, NamedKey::ArrowRight);
        assert!(channel(&chat) == Channel::Global);
        chat.modifiers = ModifiersState::empty();
        // With a draft the arrows move the caret and the button stays.
        chat.input.as_mut().unwrap().insert("hi");
        arrow(&mut chat, KeyCode::ArrowLeft, NamedKey::ArrowLeft);
        assert!(channel(&chat) == Channel::Global);
        let input = chat.input.as_ref().unwrap();
        assert_eq!(input.edit.cursor(&input.text), 1);
    }

    #[test]
    fn enter_on_sjk_hands_the_text_to_the_hub_not_the_server() {
        let mut chat = ChatOverlay::new();
        chat.open_sjk();
        assert!(channel(&chat) == Channel::Sjk);
        type_text(&mut chat, "gg all");
        assert!(matches!(enter(&mut chat), ChatInputResult::Sjk(text) if text == "gg all"));
        assert!(!chat.is_typing());
    }

    #[test]
    fn an_empty_sjk_draft_sends_nothing() {
        let mut chat = ChatOverlay::new();
        chat.open_sjk();
        type_text(&mut chat, "   ");
        assert!(matches!(enter(&mut chat), ChatInputResult::None));
    }

    #[test]
    fn the_first_sync_marks_the_backlog_without_showing_it() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[(1, "old"), (2, "older")]), |_| false, now);
        assert!(chat.lines.is_empty());
        chat.sync_sjk(
            &hub(&[(1, "old"), (2, "older"), (3, "new")]),
            |_| false,
            now,
        );
        assert_eq!(bodies(&chat), ["new"]);
    }

    #[test]
    fn hub_lines_join_the_feed_once_with_their_tag() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        let messages = hub(&[(1, "hello ^1there"), (2, "bad \u{221E}x")]);
        chat.sync_sjk(&messages, |_| false, now);
        chat.sync_sjk(&messages, |_| false, now);
        // The text is drawn in the SJK chat's gold, without its colour codes; the
        // name keeps them.
        assert_eq!(bodies(&chat), ["hello there", "bad x"]);
        let line = &chat.lines[0];
        assert!(line.channel == Channel::Sjk);
        assert_eq!(line.name, "^2Sol");
        assert!(line.sender.is_none(), "no game slot: no player menu");
        let font = crate::text::test_font();
        chat.build(true, &font, [1920.0, 1080.0], 1_000);
        let runs: Vec<&str> = chat.ui.text_runs().collect();
        // The emblem before the name; SJK after it is the channel's label.
        assert_eq!(
            runs.iter().filter(|run| **run == "SJK").count(),
            2,
            "one label a line: {runs:?}"
        );
        assert!(!runs.contains(&"SJK VERIFIED"), "the tick alone: {runs:?}");
        let logos = chat
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command, sjk_ui::DrawCommand::TexturedQuad { texture, .. }
                    if *texture == crate::ui_renderer::LOGO_TEXTURE)
            })
            .count();
        assert_eq!(logos, 2, "each line has the SJK emblem");
    }

    #[test]
    fn deleted_hub_messages_leave_the_feed() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        chat.receive(ServerEventKind::Chat, "server line".to_owned(), None, now);
        chat.sync_sjk(&hub(&[(4, "spam"), (5, "fine")]), |_| false, now);
        chat.sync_sjk(&hub(&[(5, "fine")]), |_| false, now);
        assert_eq!(bodies(&chat), ["server line", "fine"]);
        // A hub that restarted begins its ids again: what was shown stays, and its
        // backlog is not replayed.
        chat.sync_sjk(&hub(&[(1, "after a restart")]), |_| false, now);
        assert_eq!(bodies(&chat), ["server line", "fine"]);
        chat.sync_sjk(&hub(&[(1, "after a restart"), (2, "new")]), |_| false, now);
        assert_eq!(bodies(&chat), ["server line", "fine", "new"]);
    }

    #[test]
    fn a_full_chat_does_not_take_old_lines_for_deleted() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        chat.sync_sjk(&hub(&[(1, "first")]), |_| false, now);
        let full = loaded(1, (2..2 + KEPT as u64).map(|id| message(id, "x")).collect());
        chat.sync_sjk(&full, |_| false, now);
        assert_eq!(
            bodies(&chat)[..HISTORY_LIMIT - 1],
            vec!["x"; HISTORY_LIMIT - 1][..]
        );
        let mut chat = ChatOverlay::new();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        chat.sync_sjk(&hub(&[(1, "first")]), |_| false, now);
        // The service dropped message 1 off its end: still shown.
        let mut later = full.clone();
        later.messages.pop_back();
        chat.sync_sjk(&later, |_| false, now);
        assert_eq!(chat.lines.front().map(|line| line.body.as_str()), Some("x"));
    }

    #[test]
    fn only_messages_newer_than_the_feed_had_seen_are_news() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        // The hub not answered yet, then its backlog at start or a reconnection: no news.
        assert_eq!(chat.sync_sjk(&ChatState::default(), |_| false, now), None);
        assert_eq!(
            chat.sync_sjk(&hub(&[(1, "old"), (2, "older")]), |_| false, now),
            None
        );
        assert_eq!(
            chat.sync_sjk(&hub(&[(1, "old"), (2, "older")]), |_| false, now),
            None
        );
        let state = hub(&[(1, "old"), (2, "older"), (3, "new"), (4, "newer")]);
        assert_eq!(chat.sync_sjk(&state, |_| false, now), Some(2));
        assert_eq!(chat.sync_sjk(&state, |_| false, now), None, "once");
        // A new reading of the hub marks its backlog again.
        let again = loaded(2, state.messages.clone());
        assert_eq!(chat.sync_sjk(&again, |_| false, now), None);
    }

    #[test]
    fn a_sound_is_for_messages_own_included_not_muted_or_drops() {
        let mut messages: Vec<ChatMessage> = (1..=3).map(|id| message(id, "hi")).collect();
        messages[2].holocron = Some(sjk_identity::DropMark {
            tier: "legendary".to_owned(),
            own: false,
            find: None,
        });
        let troll = message(2, "").key_id;
        let news = |after| worth_a_sound(&messages, after, |key| key == troll);
        assert!(news(0), "message 1, the player's own or another's");
        assert!(!news(1), "2 is muted, 3 a holocron drop");
        assert!(!news(3), "nothing newer");
        assert!(worth_a_sound(&messages, 1, |_| false));
    }

    #[test]
    fn console_gets_only_new_visible_sjk_lines() {
        let mut messages: Vec<ChatMessage> = (1..=4).map(|id| message(id, "hi ^1there")).collect();
        let muted = messages[3].key_id.clone();
        messages[1].holocron = Some(sjk_identity::DropMark {
            tier: "legendary".to_owned(),
            own: false,
            find: None,
        });
        assert_eq!(
            console_lines(&messages, 1, |key| key == muted),
            [
                "* ^3SJK^7: Sol found a Legendary Holocron!",
                "* ^3SJK^7 ^2Sol^7: hi there",
            ]
        );
        assert!(console_lines(&messages, 4, |_| false).is_empty());
    }

    #[test]
    fn a_burst_plays_the_sound_once_a_second() {
        let mut chat = ChatOverlay::new();
        let start = Instant::now();
        assert!(chat.sjk_sound_due(start));
        assert!(!chat.sjk_sound_due(start + std::time::Duration::from_millis(300)));
        assert!(!chat.sjk_sound_due(start + std::time::Duration::from_millis(999)));
        assert!(chat.sjk_sound_due(start + SOUND_GAP));
    }

    #[test]
    fn the_feed_follows_the_chat_only_when_it_changed() {
        let mut chat = ChatOverlay::new();
        assert!(chat.sjk_changed((0, 0, 0)));
        assert!(!chat.sjk_changed((0, 0, 0)));
        assert!(chat.sjk_changed((1, 0, 0)));
        assert!(chat.sjk_changed((1, 1, 0)));
        assert!(chat.sjk_changed((1, 1, 1)), "a mute changed");
    }

    #[test]
    fn refused_messages_say_why_once() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let refused = ReportOutcome {
            serial: 1,
            sent: false,
            message: "you send messages too fast; wait a moment".to_owned(),
        };
        chat.sjk_outcome(Some(&refused), now);
        chat.sjk_outcome(Some(&refused), now);
        assert_eq!(
            bodies(&chat),
            ["^3SJK chat: you send messages too fast; wait a moment"]
        );
        let sent = ReportOutcome {
            serial: 2,
            sent: true,
            message: String::new(),
        };
        chat.sjk_outcome(Some(&sent), now);
        assert_eq!(chat.lines.len(), 1);
    }

    #[test]
    fn a_muted_key_stays_muted() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        let troll = message(3, "spam");
        chat.sync_sjk(
            &loaded(1, VecDeque::from([troll.clone()])),
            |key| key == troll.key_id,
            now,
        );
        assert!(chat.lines[0].muted);
    }

    #[test]
    fn a_muted_key_hides_lines_already_shown() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        let state = hub(&[(1, "spam"), (2, "fine")]);
        chat.sync_sjk(&state, |_| false, now);
        assert!(!chat.lines[0].muted);
        let troll = message(1, "").key_id;
        chat.sync_sjk(&state, |key| key == troll, now);
        assert!(chat.lines[0].muted && !chat.lines[1].muted);
        chat.sync_sjk(&state, |_| false, now);
        assert!(!chat.lines[0].muted, "unmuted again");
    }

    #[test]
    fn the_backlog_is_not_replayed_while_or_after_it_loads() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        // The service has started but the hub has not answered: nothing is marked.
        chat.sync_sjk(&ChatState::default(), |_| false, now);
        // Its first answer brings the backlog: marked, not shown.
        chat.sync_sjk(&hub(&[(1, "old"), (2, "older")]), |_| false, now);
        assert!(chat.lines.is_empty());
        chat.sync_sjk(
            &hub(&[(1, "old"), (2, "older"), (3, "new")]),
            |_| false,
            now,
        );
        assert_eq!(bodies(&chat), ["new"]);
        // Reading starts again (another hub, the chat turned off and on): its backlog
        // is marked again rather than replayed.
        chat.sync_sjk(&ChatState::default(), |_| false, now);
        let mut again = hub(&[(1, "old"), (2, "older"), (3, "new"), (4, "newer")]);
        again.loaded = Some(2);
        chat.sync_sjk(&again, |_| false, now);
        assert_eq!(bodies(&chat), ["new"]);
    }

    #[test]
    fn a_hidden_feed_consumes_messages_without_touching_other_chat() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        chat.receive(ServerEventKind::Chat, "server line".to_owned(), None, now);
        assert_eq!(
            chat.sync_sjk_in_feed(&hub(&[(1, "hidden")]), |_| false, now, false),
            Some(0)
        );
        assert_eq!(bodies(&chat), ["server line"]);
        chat.sync_sjk_in_feed(&hub(&[(1, "hidden"), (2, "shown")]), |_| false, now, true);
        assert_eq!(bodies(&chat), ["server line", "shown"]);
    }

    #[test]
    fn turning_the_sjk_feed_off_removes_only_its_existing_lines() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        chat.sync_sjk(&hub(&[]), |_| false, now);
        chat.receive(ServerEventKind::Chat, "server line".to_owned(), None, now);
        chat.sync_sjk(&hub(&[(1, "hub line")]), |_| false, now);
        chat.set_sjk_feed_visible(false);
        assert_eq!(bodies(&chat), ["server line"]);
        chat.set_sjk_feed_visible(false);
        assert_eq!(bodies(&chat), ["server line"]);
    }
}
