//! Keyboard and pointer actions for the floating conversation layer.

use super::*;
use sjk_client::{ChatDestination, chat_command_full};
use sjk_ui::{InputEvent, PointerButton, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, KeyCode, PhysicalKey};

pub(super) const GLOBAL: u16 = 100;
pub(super) const TEAM: u16 = 101;
pub(super) const LATEST: u16 = 102;
pub(super) const SJK: u16 = 103;
pub(super) const ADMIN: u16 = 104;
pub(super) const PM: u16 = 105;
pub(super) const JOF: u16 = 106;
pub(super) const SITH: u16 = 107;
/// The whisper recipient's `<`: the previous player who whispered you (`view/recipient.rs`).
pub(super) const PM_PREV: u16 = 108;
/// The whisper recipient's `>`: the next player who whispered you.
pub(super) const PM_NEXT: u16 = 109;
/// The whisper recipient's name: opens the list of the players who whispered you.
pub(super) const PM_NAME: u16 = 110;
/// The list's first player; the others follow, up to `PM_LIST + LIST_MAX - 1` (118).
pub(super) const PM_LIST: u16 = 111;
/// The list's tokens, newest whisperer first.
const PM_LIST_TOKENS: std::ops::Range<u16> = PM_LIST..PM_LIST + super::view::PM_LIST_MAX as u16;

/// Keys the composer acts on itself rather than typing their text.
fn handled_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::Escape
            | KeyCode::Enter
            | KeyCode::NumpadEnter
            | KeyCode::Tab
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Backspace
            | KeyCode::Delete
    )
}

impl ChatOverlay {
    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> ChatInputResult {
        if event.state != ElementState::Pressed {
            return ChatInputResult::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return ChatInputResult::None;
        };
        if self.edit_shortcut(event) {
            if let Some(input) = &mut self.input {
                input.dead.other_key(event.text.as_deref());
            }
            return ChatInputResult::None;
        }
        self.edit_key(key, &event.logical_key, event.text.as_deref())
    }

    pub(super) fn edit_key(
        &mut self,
        key: KeyCode,
        logical: &Key,
        text: Option<&str>,
    ) -> ChatInputResult {
        let Some(input) = &mut self.input else {
            return ChatInputResult::None;
        };
        // Keys the composer acts on itself end a pending composition when the
        // platform reported text for them (see `input::dead_key`).
        if handled_key(key) {
            input.dead.other_key(text);
        }
        match key {
            // A pinned sender card goes first, the composer with the next Escape.
            KeyCode::Escape if self.unpin_card() => {}
            // An open + or cog menu goes before the composer (`chat/row_tools.rs`).
            KeyCode::Escape
                if self
                    .input
                    .as_ref()
                    .is_some_and(|input| input.menu.is_some()) =>
            {
                if let Some(input) = self.input.as_mut() {
                    input.menu = None;
                }
            }
            // An open list of whisperers goes before the composer too.
            KeyCode::Escape if self.input.as_ref().is_some_and(|input| input.pm_list) => {
                if let Some(input) = self.input.as_mut() {
                    input.pm_list = false;
                }
            }
            KeyCode::Escape => {
                self.input = None;
                self.scroll = 0;
                self.unread = 0;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => return self.submit(),
            // Alt+Left and Alt+Right step between the players who whispered you, as the
            // recipient's `<` and `>` do.
            KeyCode::ArrowLeft | KeyCode::ArrowRight
                if self.modifiers.alt_key()
                    && self
                        .input
                        .as_ref()
                        .is_some_and(|input| input.channel == Channel::Whisper) =>
            {
                self.step_pm(if key == KeyCode::ArrowLeft { -1 } else { 1 });
            }
            KeyCode::PageUp | KeyCode::ArrowUp => self.scroll_by(3),
            KeyCode::PageDown | KeyCode::ArrowDown => self.scroll_by(-3),
            KeyCode::Tab => {
                let channel = self.input.as_ref().expect("active input").channel;
                // The row's channels in turn (`chat/tabs.rs`), then the last whisperer
                // (`chat/reply.rs`), then All.
                match self.next_channel(channel) {
                    Some(next) => self.choose(next),
                    None => {
                        let target = self.reply_target().ok();
                        let input = self.input.as_mut().expect("active input");
                        input.channel = Channel::Whisper;
                        input.recipient = target;
                        self.notice = "";
                    }
                }
            }
            _ => {
                let input = self.input.as_mut().expect("active input");
                if !input.key(
                    key,
                    self.modifiers.control_key(),
                    self.modifiers.shift_key(),
                ) && (!self.modifiers.control_key() || self.modifiers.alt_key())
                {
                    input.type_key(logical, text);
                }
            }
        }
        ChatInputResult::None
    }

    pub(super) fn submit(&mut self) -> ChatInputResult {
        let input = self.input.as_ref().expect("active input");
        // The hub's chats: the SJK chat and the clan chats.
        if input.channel.is_hub() {
            let channel = input.channel;
            let text = input.text.trim().to_owned();
            self.input = None;
            self.scroll = 0;
            self.unread = 0;
            if text.is_empty() {
                return ChatInputResult::None;
            }
            self.sent_on(channel, None);
            return match channel.clan() {
                Some(clan) => ChatInputResult::Clan(text, clan),
                None => ChatInputResult::Sjk(text),
            };
        }
        let destination = match input.channel {
            Channel::Sjk | Channel::Jof | Channel::Sith => unreachable!("sent to the hub above"),
            Channel::Global => ChatDestination::Global,
            Channel::Team => ChatDestination::Team,
            Channel::Admin => ChatDestination::Admin,
            Channel::Whisper => {
                let Some(target) = input
                    .recipient
                    .filter(|target| self.roster.name(*target).is_some())
                else {
                    self.notice = "Player left or changed identity. Choose a name again.";
                    return ChatInputResult::None;
                };
                ChatDestination::Player(target.slot())
            }
        };
        let result = chat_command_full(destination, &input.text);
        if result.is_some() {
            let (channel, recipient) = (input.channel, input.recipient);
            self.sent_on(channel, recipient);
        }
        self.submitted = self.input.take();
        self.scroll = 0;
        self.unread = 0;
        result.map_or(ChatInputResult::None, ChatInputResult::Submit)
    }

    pub(crate) fn pointer(&mut self, event: InputEvent) {
        if !self.is_typing() {
            return;
        }
        if let InputEvent::PointerMove(position)
        | InputEvent::PointerPress { position, .. }
        | InputEvent::PointerRelease { position, .. } = event
        {
            self.pointer_at = Some(position);
        }
        // The profile card lies over the feed and may reach the composer: a press on
        // it is the card's.
        let on_card = matches!(event, InputEvent::PointerPress { position, .. }
            if self.card.is_some()
                && self.ui.rect_for(card::CARD).is_some_and(|rect| rect.contains(position)));
        if !on_card && self.edit_pointer(event) {
            return;
        }
        // A right click on PM steps back to the previous whisperer (`chat/tabs.rs`).
        if let InputEvent::PointerRelease {
            position,
            button: PointerButton::Secondary,
        } = event
            && self
                .ui
                .rect_for(PM)
                .is_some_and(|rect| rect.contains(position))
        {
            self.step_pm(-1);
            return;
        }
        if let InputEvent::PointerPress {
            position,
            button: PointerButton::Primary,
        } = event
        {
            // Match the canvas's reverse paint order: the card covers the
            // composer, which covers the feed. Otherwise release activates a
            // different widget than the one recorded here and is discarded.
            // The list of whisperers lies over the feed, under the card.
            let list_open = self.input.as_ref().is_some_and(|input| input.pm_list);
            let list = if list_open {
                PM_LIST_TOKENS
            } else {
                PM_LIST..PM_LIST
            };
            let menu_open = self
                .input
                .as_ref()
                .is_some_and(|input| input.menu.is_some());
            let items = if menu_open {
                row_tools::ITEMS
            } else {
                row_tools::ITEM..row_tools::ITEM
            };
            self.pressed_action = card::BUTTONS
                .into_iter()
                .chain([card::CARD])
                .chain(list)
                .chain(items)
                .chain(row_tools::CLOSES)
                .chain([row_tools::ADD, row_tools::SETTINGS])
                .chain([
                    LATEST, PM_PREV, PM_NEXT, PM_NAME, PM, ADMIN, SITH, JOF, SJK, TEAM, GLOBAL,
                ])
                .chain(0..MAX_VISIBLE as u16)
                .find(|token| {
                    self.ui
                        .rect_for(*token)
                        .is_some_and(|rect| rect.contains(position))
                });
            // A sender's name: whose it is, so a release on another never pins theirs.
            self.pressed_who = self
                .pressed_action
                .and_then(|token| self.visible_people.get(usize::from(token)).copied())
                .flatten();
            // A press off the open list and off the name that opened it closes it.
            if list_open
                && !self
                    .pressed_action
                    .is_some_and(|token| token == PM_NAME || PM_LIST_TOKENS.contains(&token))
                && let Some(input) = self.input.as_mut()
            {
                input.pm_list = false;
            }
            // A press off the open menu and off the button that opened it closes it.
            if menu_open
                && !self.pressed_action.is_some_and(|token| {
                    row_tools::ITEMS.contains(&token)
                        || token == row_tools::ADD
                        || token == row_tools::SETTINGS
                })
                && let Some(input) = self.input.as_mut()
            {
                input.menu = None;
            }
            // A press off a pinned card and off every name lets the card go.
            if !self
                .pressed_action
                .is_some_and(|token| usize::from(token) < MAX_VISIBLE || token >= card::CARD)
            {
                self.unpin_card();
            }
        }
        if let InputEvent::PointerWheel { delta, .. } = event {
            if delta.y != 0.0 {
                self.scroll_by(if delta.y > 0.0 { 1 } else { -1 });
            }
            return;
        }
        let result = self.ui.pointer(event);
        if let Some(result) = result
            && result.kind == UiEventKind::Activate
            && let Some(token) = result.token
            && self.pressed_action.take() == Some(token)
        {
            self.activate(token);
        }
    }

    fn scroll_by(&mut self, delta: isize) {
        self.scroll = self.scroll.saturating_add_signed(delta).min(
            self.lines
                .iter()
                .filter(|line| self.shown(line))
                .count()
                .saturating_sub(1),
        );
        if self.scroll == 0 {
            self.unread = 0;
        }
        for line in &mut self.lines {
            line.y = None;
        }
    }

    /// Put the composer on `channel`, away from any whisper.
    pub(super) fn choose(&mut self, channel: Channel) {
        if let Some(input) = &mut self.input {
            input.channel = channel;
            input.recipient = None;
            input.pm_list = false;
            self.notice = "";
        }
    }

    pub(super) fn activate(&mut self, token: u16) {
        match token {
            GLOBAL => self.choose(Channel::Global),
            TEAM => self.choose(Channel::Team),
            SJK => self.choose(Channel::Sjk),
            ADMIN => self.choose(Channel::Admin),
            JOF => self.choose(Channel::Jof),
            SITH => self.choose(Channel::Sith),
            PM => self.step_pm(1),
            PM_PREV => self.step_pm(-1),
            PM_NEXT => self.step_pm(1),
            PM_NAME => self.toggle_pm_list(),
            row_tools::ADD => self.toggle_menu(row_tools::RowMenu::Add),
            row_tools::SETTINGS => self.toggle_menu(row_tools::RowMenu::Settings),
            token if row_tools::ITEMS.contains(&token) => {
                self.pick_item(usize::from(token - row_tools::ITEM));
            }
            token if row_tools::CLOSES.contains(&token) => {
                self.close_tab(super::tabs::ROW[usize::from(token - row_tools::CLOSE)]);
            }
            token if PM_LIST_TOKENS.contains(&token) => self.pick_pm(usize::from(token - PM_LIST)),
            LATEST => {
                self.scroll = 0;
                self.unread = 0;
            }
            card::CARD_MUTE => self.toggle_card_mute(),
            card::CARD_BAN => self.press_card_ban(),
            card::CARD_SAFETY => self.press_card_safety(),
            card::CARD_REPORT => self.press_card_report(),
            card::CARD_WHISPER => self.press_card_whisper(),
            card::CARD_TRADE => self.press_card_trade(),
            card::CARD_MAP_BLOCK => self.press_card_map_block(false),
            card::CARD_STAFF_BLOCK => self.press_card_map_block(true),
            token if usize::from(token) < MAX_VISIBLE => {
                let pressed = self.pressed_who.take();
                self.click_name(usize::from(token), pressed);
            }
            _ => {}
        }
    }
}

impl crate::GpuState {
    /// Refresh recipient identity at submission time and use the existing reliable
    /// channel. No pointer action sends traffic or enters the gameplay bind path.
    pub(crate) fn chat_key(&mut self, event: &KeyEvent) {
        if let Some(session) = self
            .resident
            .session
            .as_ref()
            .or(self.live_session.as_ref())
        {
            self.chat.update_roster(session.game_state());
        }
        match self.chat.handle_key(event) {
            ChatInputResult::Submit(command) => {
                let sent = self.send_chat_command(&command, None);
                self.chat.settle_submitted(sent);
            }
            ChatInputResult::Sjk(text) => {
                // Staff's Shift+Enter keeps it out of the server's All chat.
                let only = self.chat.modifiers.shift_key()
                    && self.chat.said_in_all.is_some()
                    && crate::player_identity::is_staff();
                self.send_sjk_chat(text, only);
            }
            ChatInputResult::Clan(text, channel) => self.send_clan_chat(text, channel),
            ChatInputResult::None => {}
        }
        self.sync_cursor_policy();
    }

    /// Hand a message to the SJK chat (`only`: never said in the server's All chat, even
    /// where a mod says SJK chat there); say why when it cannot go.
    pub(crate) fn send_sjk_chat(&mut self, text: String, only: bool) {
        let off = self
            .console
            .as_ref()
            .is_some_and(|console| !console.sjk_chat_on());
        let why = if off {
            Some("^3SJK chat is off (cl_sjkChat 1 turns it on)")
        } else if !if only {
            crate::player_identity::chat_sjk_only(text)
        } else {
            crate::player_identity::chat(text)
        } {
            Some("^3SJK chat needs the SJK identity (cl_identity 1)")
        } else {
            None
        };
        if let Some(why) = why {
            self.chat.receive(
                sjk_client::ServerEventKind::Chat,
                why.to_owned(),
                None,
                Instant::now(),
            );
        }
    }

    /// Hand a message to a clan chat (`chat/tabs.rs`); say why when it cannot go. It
    /// never goes to the game server.
    pub(crate) fn send_clan_chat(&mut self, text: String, channel: sjk_identity::HubChannel) {
        let off = self
            .console
            .as_ref()
            .is_some_and(|console| console.bool_cvar("cl_sjkChat") == Some(false));
        let why = if off {
            Some("^3The clan chats come with SJK chat, which is off (cl_sjkChat 1 turns it on)")
        } else if !crate::player_identity::chat_clan(text, channel) {
            Some("^3The clan chats need the SJK identity (cl_identity 1)")
        } else {
            None
        };
        if let Some(why) = why {
            self.chat.receive(
                sjk_client::ServerEventKind::Chat,
                why.to_owned(),
                None,
                Instant::now(),
            );
        }
    }

    /// Composer and console messages share the real server's reliable channel.
    /// False when the message was refused (more than four pieces); nothing was sent.
    pub(crate) fn send_chat_command(
        &mut self,
        command: &str,
        console_colors: Option<(u16, u32)>,
    ) -> bool {
        let values = self.chat_substitution_values();
        let command = super::substitutions::command(
            command,
            values.as_ref(),
            sjk_shell::local_time::LocalTime::now(),
        );
        let command = self.console.as_ref().map_or_else(
            || command.clone(),
            |console| console.color_chat_command(&command, console_colors),
        );
        // A whisper to a player who takes encrypted ones goes through the SJK hub
        // instead (`e2e_whispers.rs`).
        if self.route_encrypted_whisper(&command) {
            return true;
        }
        let Some(commands) = self.outgoing_chat_parts(&command) else {
            return false;
        };
        let origin = if commands.len() > 1 {
            sjk_client::ReliableCommandOrigin::ChatPart
        } else {
            sjk_client::ReliableCommandOrigin::Chat
        };
        for command in commands {
            // A full encrypted whisper may exceed the game server's chat limit;
            // once divided, route each piece through the hub as well.
            if self.route_encrypted_whisper(&command) {
                continue;
            }
            let recipient = whisper_destination(&command).map(|slot| {
                self.chat
                    .player_name(slot)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("client {slot}"))
            });
            self.send_chat_to_server_from(&command, recipient, origin);
        }
        true
    }

    /// Send a chat command through the game server even when it could have been an
    /// encrypted whisper. The failure card uses this only after the player chooses
    /// "Send unencrypted".
    pub(crate) fn send_plain_chat_command(&mut self, command: &str, recipient: Option<String>) {
        let Some(commands) = self.outgoing_chat_parts(command) else {
            return;
        };
        let origin = if commands.len() > 1 {
            sjk_client::ReliableCommandOrigin::ChatPart
        } else {
            sjk_client::ReliableCommandOrigin::Chat
        };
        for command in commands {
            self.send_chat_to_server_from(&command, recipient.clone(), origin);
        }
    }

    fn outgoing_chat_parts(&mut self, command: &str) -> Option<Vec<String>> {
        let mod_chat = self
            .console
            .as_ref()
            .map(|console| console.mod_chat_commands())
            .unwrap_or_default();
        match split_outgoing_chat(command, mod_chat) {
            Ok(commands) => Some(commands),
            Err(_) => {
                const TOO_LONG: &str = "Message is too long (four chat messages maximum).";
                self.chat.notice = TOO_LONG;
                if let Some(console) = self.console.as_mut() {
                    console.push_log(format!("^3{TOO_LONG}"));
                }
                None
            }
        }
    }

    fn send_chat_to_server_from(
        &mut self,
        command: &str,
        recipient: Option<String>,
        origin: sjk_client::ReliableCommandOrigin,
    ) {
        let sent = if let Some(session) = self.communication_session_mut() {
            if let Err(error) = session.send_reliable_command_from(command.as_bytes(), origin) {
                eprintln!("failed to send chat: {error}");
                false
            } else {
                true
            }
        } else {
            false
        };
        if sent
            && let Some(name) = recipient
            && let Some(console) = &mut self.console
        {
            console.note_sent_whisper(command, name);
        }
    }
}

fn split_outgoing_chat(
    command: &str,
    mod_chat: &[&str],
) -> Result<Vec<String>, sjk_client::ChatDraftTooLong> {
    let Ok(tokens) = sjk_shell::tokenize(command) else {
        return Ok(vec![command.to_owned()]);
    };
    let Some(verb) = tokens.first() else {
        return Ok(vec![command.to_owned()]);
    };
    let (destination, text) = if verb.eq_ignore_ascii_case("say") {
        (Some(ChatDestination::Global), tokens[1..].join(" "))
    } else if verb.eq_ignore_ascii_case("say_team") {
        (Some(ChatDestination::Team), tokens[1..].join(" "))
    } else if verb.eq_ignore_ascii_case("amsay") {
        (Some(ChatDestination::Admin), tokens[1..].join(" "))
    } else if verb.eq_ignore_ascii_case("tell") && tokens.len() >= 3 {
        let Ok(slot) = tokens[1].parse() else {
            return Ok(vec![command.to_owned()]);
        };
        (Some(ChatDestination::Player(slot)), tokens[2..].join(" "))
    } else if mod_chat.iter().any(|name| verb.eq_ignore_ascii_case(name)) {
        (None, tokens[1..].join(" "))
    } else {
        return Ok(vec![command.to_owned()]);
    };
    let text = sjk_client::chat_unescape(&text);
    if let Some(destination) = destination {
        return sjk_client::chat_commands(destination, &text);
    }
    sjk_client::chat_commands(ChatDestination::Global, &text).map(|commands| {
        commands
            .into_iter()
            .map(|piece| format!("{verb}{}", &piece[3..]))
            .collect()
    })
}

fn whisper_destination(command: &str) -> Option<u16> {
    let tokens = sjk_shell::tokenize(command).ok()?;
    let [verb, slot, body @ ..] = tokens.as_slice() else {
        return None;
    };
    (verb.eq_ignore_ascii_case("tell") && !body.is_empty())
        .then(|| slot.parse().ok())
        .flatten()
}

#[cfg(test)]
mod whisper_destination_tests {
    use super::{split_outgoing_chat, whisper_destination};

    #[test]
    fn reads_only_a_complete_tell_command() {
        assert_eq!(whisper_destination("tell 7 \"hello there\""), Some(7));
        assert_eq!(whisper_destination("tell 7 hello there"), Some(7));
        assert_eq!(whisper_destination("TELL 12 hello"), Some(12));
        assert_eq!(whisper_destination("say hello"), None);
        assert_eq!(whisper_destination("tell nope hello"), None);
        assert_eq!(whisper_destination("tell 7"), None);
    }

    #[test]
    fn divides_stock_and_declared_mod_chat_without_losing_escapes() {
        let text = format!("{} % \"end\"", "many words ".repeat(40));
        let full = sjk_client::chat_command_full(sjk_client::ChatDestination::Global, &text)
            .expect("chat");
        let pieces = split_outgoing_chat(&full, &[]).expect("four messages");
        assert!(pieces.len() >= 3);
        assert!(pieces.iter().all(|piece| {
            let tokens = sjk_shell::tokenize(piece).expect("safe command");
            sjk_protocol::encode_legacy_text(&tokens[1]).len() <= sjk_client::CHAT_INPUT_BYTES
        }));

        let pieces = split_outgoing_chat(
            &format!("clansay \"{}\"", "clan words ".repeat(30)),
            &["clansay"],
        )
        .expect("four messages");
        assert!(pieces.len() > 1);
        assert!(pieces.iter().all(|piece| piece.starts_with("clansay \"")));

        // The encrypted-whisper failure card's Send unencrypted choice uses this
        // same splitter instead of handing its full stored command to the server.
        let pieces =
            split_outgoing_chat(&format!("tell 7 \"{}\"", "private words ".repeat(30)), &[])
                .expect("four-message whisper");
        assert!(pieces.len() > 1);
        assert!(pieces.iter().all(|piece| piece.starts_with("tell 7 \"")));
    }

    #[test]
    fn refuses_more_than_four_pieces() {
        assert_eq!(
            split_outgoing_chat(&format!("say \"{}\"", "%".repeat(1_200)), &[]),
            Err(sjk_client::ChatDraftTooLong)
        );
    }
}

#[cfg(test)]
mod dead_key_tests {
    use super::*;
    use winit::keyboard::{NamedKey, SmolStr};

    fn press(chat: &mut ChatOverlay, key: KeyCode, logical: Key, text: Option<&str>) {
        chat.edit_key(key, &logical, text);
    }

    fn typed(chat: &mut ChatOverlay, key: KeyCode, text: &str) {
        press(chat, key, Key::Character(SmolStr::new(text)), Some(text));
    }

    fn dead_caret(chat: &mut ChatOverlay) {
        press(chat, KeyCode::BracketLeft, Key::Dead(Some('^')), None);
    }

    fn draft(chat: &ChatOverlay) -> &str {
        &chat.input.as_ref().expect("composer open").text
    }

    fn composer() -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        chat.open(false);
        chat
    }

    #[test]
    fn azerty_dead_caret_and_digit_type_a_colour_code() {
        let mut chat = composer();
        dead_caret(&mut chat);
        assert_eq!(draft(&chat), "^");
        press(
            &mut chat,
            KeyCode::ShiftLeft,
            Key::Named(NamedKey::Shift),
            None,
        );
        typed(&mut chat, KeyCode::Digit1, "^1");
        typed(&mut chat, KeyCode::KeyH, "h");
        assert_eq!(draft(&chat), "^1h");
    }

    #[test]
    fn dead_caret_and_a_vowel_type_the_circumflex_letter() {
        let mut chat = composer();
        typed(&mut chat, KeyCode::KeyF, "f");
        dead_caret(&mut chat);
        typed(&mut chat, KeyCode::KeyE, "ê");
        typed(&mut chat, KeyCode::KeyT, "t");
        typed(&mut chat, KeyCode::KeyE, "e");
        assert_eq!(draft(&chat), "fête");
    }

    #[test]
    fn dead_caret_then_space_types_the_caret_alone() {
        let mut chat = composer();
        dead_caret(&mut chat);
        typed(&mut chat, KeyCode::Space, "^");
        assert_eq!(draft(&chat), "^");
    }

    #[test]
    fn dead_caret_is_replaced_at_the_caret_inside_the_draft() {
        let mut chat = composer();
        typed(&mut chat, KeyCode::KeyA, "a");
        typed(&mut chat, KeyCode::KeyB, "b");
        press(
            &mut chat,
            KeyCode::ArrowLeft,
            Key::Named(NamedKey::ArrowLeft),
            None,
        );
        dead_caret(&mut chat);
        assert_eq!(draft(&chat), "a^b");
        typed(&mut chat, KeyCode::Digit2, "^2");
        assert_eq!(draft(&chat), "a^2b");
        let input = chat.input.as_ref().unwrap();
        assert_eq!(input.edit.cursor(&input.text), 3);
    }

    #[test]
    fn backspace_after_a_dead_caret_erases_only_the_caret() {
        let mut chat = composer();
        typed(&mut chat, KeyCode::KeyA, "a");
        dead_caret(&mut chat);
        press(
            &mut chat,
            KeyCode::Backspace,
            Key::Named(NamedKey::Backspace),
            Some("^\u{8}"),
        );
        assert_eq!(draft(&chat), "a");
        typed(&mut chat, KeyCode::Digit1, "1");
        assert_eq!(draft(&chat), "a1");
    }

    #[test]
    fn enter_after_a_dead_caret_sends_it() {
        let mut chat = composer();
        typed(&mut chat, KeyCode::KeyA, "a");
        dead_caret(&mut chat);
        let result = chat.edit_key(KeyCode::Enter, &Key::Named(NamedKey::Enter), Some("^\r"));
        assert!(matches!(result, ChatInputResult::Submit(command) if command == "say \"a^\""));
    }
}
