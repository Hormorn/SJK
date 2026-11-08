//! Floating conversation overlay. Chat owns presentation/history only; player
//! identity and stock command semantics come from the client adapter.

mod card;
mod editing;
mod editor;
pub(crate) mod emoji;
mod feed_gif;
pub(crate) mod find_blade;
pub(crate) mod incoming_color;

pub(crate) mod client_icons;
mod interaction;
mod layout;
mod links;
pub(crate) mod marks;
pub(crate) mod options;
pub(crate) mod pm_only;
pub(crate) mod quiet;
pub(crate) mod reply;
mod row_tools;
pub(crate) mod server_copy;
pub(crate) mod sjk;
mod substitutions;
pub(crate) mod tabs;
pub(crate) mod text_sizes;

mod view;

pub(crate) use view::{encrypted_pink, star};

pub(crate) mod combat;

use crate::game_font::{GameFonts, RetailFont};
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextVertex, UiFont};
use editor::Editor;
use sjk_client::{
    ChatRoster, ChatTarget, ServerEventKind, chat_body, chat_display_text, chat_unescape,
};
use sjk_ui::{DrawList, Tween};
use std::collections::VecDeque;
use std::time::Instant;

const HISTORY_LIMIT: usize = 64;
/// Most messages the feed shows together, `cg_chatBoxLines`' top: EternalJK's
/// `MAX_CHATBOX_ITEMS` (24). It was 8 before 23/10/2026, so a larger value did nothing
/// (Creyon's report #129).
const MAX_VISIBLE: usize = 24;
const HOLD_MS: u64 = 9_000;
const FADE_MS: u64 = 2_000;
const ENTER_MS: u32 = 220;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Channel {
    Global,
    Team,
    Whisper,
    /// The SJK chat, through the SJK hub (`chat/sjk.rs`).
    Sjk,
    /// The server's admins, `amsay` (`chat/tabs.rs`): jaPRO's `<Admin>(name)` lines.
    Admin,
    /// JoF's clan chat, through the SJK hub (`chat/sjk.rs`, `sjk_identity::channels`).
    Jof,
    /// The Sith clan's chat (`{{S}}`), through the SJK hub, shown as S.
    Sith,
}

impl Channel {
    /// The hub's clan chat this channel is, if it is one.
    pub(crate) fn clan(self) -> Option<sjk_identity::HubChannel> {
        match self {
            Self::Jof => Some(sjk_identity::HubChannel::Jof),
            Self::Sith => Some(sjk_identity::HubChannel::Sith),
            _ => None,
        }
    }

    /// The feed's channel for a hub message said in `channel`.
    pub(super) fn of_hub(channel: sjk_identity::HubChannel) -> Self {
        match channel {
            sjk_identity::HubChannel::Jof => Self::Jof,
            sjk_identity::HubChannel::Sith => Self::Sith,
            _ => Self::Sjk,
        }
    }

    /// Whether lines of this channel come from the SJK hub (SJK chat or a clan chat).
    fn is_hub(self) -> bool {
        matches!(self, Self::Sjk | Self::Jof | Self::Sith)
    }
}

struct ChatLine {
    name: String,
    body: String,
    sender: Option<ChatTarget>,
    channel: Channel,
    server_private: bool,
    received_ms: u64,
    muted: bool,
    /// The emojis the body's marks stand for, in order ([`emoji::Emojis::markup`]).
    emojis: Vec<u16>,
    wrap: layout::Wrapped,
    y: Option<Tween>,
    /// What the hub said of an SJK chat line.
    hub: Option<HubLine>,
    /// The GIF an SJK chat line links to, shown under it (`chat/feed_gif.rs`).
    gif: Option<Box<feed_gif::LineGif>>,
    /// An end-to-end encrypted whisper through the SJK hub (`e2e_whispers.rs`): in its
    /// own pink, labelled ENCRYPTED. One the player sent has no sender and is named
    /// "To <recipient>".
    encrypted: bool,
    /// An encrypted whisper with a player checked with the safety code: labelled
    /// VERIFIED.
    verified: bool,
    /// The slot the player's own encrypted whisper went to: its line wears the
    /// recipient's marks (emblem, verified tick or trust mark, name emoji).
    recipient: Option<u16>,
    /// The line passes quiet mode (`chat/quiet.rs`), decided when it arrived.
    loud: bool,
}

/// An SJK chat line's id at the hub and its sender.
struct HubLine {
    id: u64,
    verified: bool,
    /// The sender's tick when they sent it, as the hub sent it.
    tick: sjk_identity::Tick,
    staff: bool,
    /// SJK's creator (staff too).
    creator: bool,
    /// The sender's trust when they sent it (`ui_renderer::mark_texture`).
    trust: Option<sjk_identity::Trust>,
    /// The sender's name emoji when they sent it, shown at the right of their name.
    emoji: String,
    /// Sent from another client with its own icon (`chat/client_icons.rs`), which takes
    /// the emblem's place.
    client: Option<&'static client_icons::ClientIcon>,
    /// Said in the game server's All chat too (`chat/server_copy.rs`): labelled ALL/SJK
    /// instead of SJK.
    in_all: bool,
    key_id: String,
    /// The line is a holocron drop of this tier (`holocrons/line.rs`): in the tier's
    /// colour, a gem before it instead of the verified tick.
    tier: Option<&'static crate::holocrons::Tier>,
    /// A grand find's prize (`holocrons/line.rs`): its words in its own colour, and its
    /// blade drawn small after the holocron's picture (`chat/find_blade.rs`).
    prize: Option<crate::holocrons::line::Prize>,
    /// The line is the SJK team's (an announcement or a reward round, `official.rs`):
    /// "SJK" in the SJK UI's accent gold, no label.
    official: bool,
}

/// Bounded incoming history and the currently active unboxed composer.
pub(crate) struct ChatOverlay {
    lines: VecDeque<ChatLine>,
    center: Option<(String, u64)>,
    /// Text ids of this frame's centre-print rows, which retail drew with the
    /// medium font rather than the chat box's small one.
    center_text: (u32, u32),
    /// Text ids of this frame's feed and composer, which `cg_chatBoxOutline` edges in a
    /// dark outline ([`Self::palette_of`]); the centre prints and the sender card keep
    /// their shadow.
    chat_text: [(u32, u32); 2],
    combat: combat::Combat,
    input: Option<Editor>,
    /// The draft just submitted to the game server, put back by
    /// [`Self::settle_submitted`] when sending refuses it (too many pieces once
    /// `%W%` and the chat prefix and suffix are expanded), so the text is not lost.
    submitted: Option<Editor>,
    modifiers: winit::keyboard::ModifiersState,
    roster: ChatRoster,
    scroll: usize,
    unread: usize,
    clock: Instant,
    opened_ms: u64,
    notice: &'static str,
    ui: MenuCanvas,
    /// The feed's name or card target a press began on, until its release.
    pressed_action: Option<u16>,
    /// Whose name that press began on, when it began on a name (`chat/card.rs`).
    pressed_who: Option<card::Who>,
    /// Whose name each feed token is, for the profile card (`chat/card.rs`).
    visible_people: [Option<card::Who>; MAX_VISIBLE],
    /// The finds' blades the feed shows, by line token, while the pointer is free: the
    /// skin, where it is drawn and its prize's colour (`chat/find_blade.rs`).
    visible_blades: [Option<find_blade::Shown>; MAX_VISIBLE],
    /// The profile card on show while the composer is open.
    card: Option<card::Card>,
    /// Mute or Unmute asked from the card, until the frame takes it.
    mute_request: Option<card::MuteRequest>,
    /// Settings the composer row's tools changed, until the frame saves them
    /// (`chat/row_tools.rs`).
    settings: Vec<(&'static str, &'static str)>,
    /// Where this frame drew the button of the open + or cog menu, which the menu
    /// lines up with.
    menu_left: f32,
    /// A block for this map a card asked for (`chat/card.rs`).
    map_block_request: Option<card::MapBlockRequest>,
    /// The map blocks as the frame last said: the player's own slots, staff's, and
    /// whether the player is staff (`ChatOverlay::set_map_blocks`).
    map_blocks: (u32, u32, bool),
    /// Client slots muted on this PC (bits, `muted_players.rs`): their lines are hidden.
    muted_slots: u32,
    /// What SJK knows of each slot's player, for their game lines (`chat/marks.rs`).
    slot_marks: Vec<marks::SlotMark>,
    /// Chat lines each client slot has sent, for the scoreboard's away marks.
    spoken: [u32; 32],
    /// What the marks were worked out from.
    marks_key: Option<marks::MarksKey>,
    layout_viewport: [f32; 2],
    scoreboard_layout: crate::scoreboard::behavior::ChatLayout,
    options: options::Options,
    emojis: emoji::Emojis,
    /// The newest SJK chat id the feed has seen; `None` before the first sync.
    sjk_seen: Option<u64>,
    /// The reading of the hub (`ChatState::loaded`) whose backlog was marked.
    sjk_epoch: Option<u64>,
    /// The announcements whose lines the feed has shown (`chat/sjk.rs`): one the service
    /// put at the chat's end takes the newest message's id, so it is followed by its own.
    sjk_officials: Vec<u64>,
    /// The serial of the last SJK chat outcome shown.
    sjk_outcome: u64,
    /// The SJK chat's revision, outcome serial and mutes revision last followed.
    sjk_mark: Option<(u64, u64, u64)>,
    /// When the SJK chat's sound last played (`chat/sjk.rs`, [`sjk::SOUND_GAP`]).
    sjk_sound_at: Option<Instant>,
    /// SJK chat said on the game server too (`chat/server_copy.rs`).
    server_copies: server_copy::ServerCopies,
    /// While a mod says SJK chat in this server's All chat (`sjk_chat_frame.rs`), the
    /// red note above the field when the SJK channel is picked.
    said_in_all: Option<server_copy::SaidInAll>,
    /// Whether SJK messages join the in-game general chat feed (`cl_sjkChatFeed`).
    sjk_feed_visible: bool,
    /// Whether the lines with a GIF read "GIF" and show it (`cl_sjkChatGifs`), or keep
    /// their links as text (`chat/feed_gif.rs`).
    gifs_shown: bool,
    gif_sources_revision: u64,
    /// Our own client slot, from the gamestate: our whispers echoed back are not
    /// whispers to reply to (`chat/reply.rs`).
    own_slot: Option<u16>,
    /// The last player who whispered to us, for `reply`.
    last_whisperer: Option<ChatTarget>,
    /// The channel (and whisper recipient) the composer last sent on, which the chat
    /// key opens on while `cg_chatRememberChannel` is on.
    last_channel: (Channel, Option<ChatTarget>),
    /// The server being played takes `amsay` (`chat/tabs.rs`).
    admin_server: bool,
    /// The clan chats the player joined (`chat/tabs.rs`), JoF's then S's: their buttons
    /// show only then.
    clans: [bool; 2],
    /// Where the pointer last was over the chat, for hovers checked against this frame's
    /// layout ([`card`]): the canvas's hovered widget is the one under the pointer when it
    /// last moved, and a feed that changed under a still pointer (a tab chosen) would
    /// otherwise hand that place to another widget.
    pointer_at: Option<sjk_ui::Vec2>,
    /// Where this frame drew the links of the shown lines, while the composer is open,
    /// for Ctrl+C on a link (`chat/links.rs`).
    links: links::Spots,
    /// When a link was last copied, for the "Link copied" note under the field.
    link_copied_ms: Option<u64>,
    /// The client slots (bits) whose player takes encrypted whispers now
    /// (`e2e_whispers.rs`): a whisper to them goes encrypted, which the composer says,
    /// and their card offers the safety code.
    encrypted_slots: u64,
    /// Safety code asked from a card, for this slot, until the frame takes it.
    safety_request: Option<u16>,
    /// The client slots (bits) whose player's last encrypted whisper to us can be
    /// reported (`e2e_whispers.rs`): their card offers Report.
    reportable_slots: u64,
    /// Report asked from a card, for this slot, until the frame takes it.
    report_request: Option<u16>,
    /// The slot whose card asked to trade, for the frame (`crate::player_trades`).
    trade_request: Option<u16>,
    /// The private duel partner, kept a while after the duel, for quiet mode
    /// (`chat/quiet.rs`).
    duel_partner: sjk_client::DuelPartner,
    /// The player's own SJK identity key, for quiet mode's SJK lines.
    sjk_own_key: Option<String>,
    /// The feed draws during a world shot, which has no session.
    #[cfg(test)]
    pub(crate) for_shot: bool,
}

impl ChatOverlay {
    /// Borrow an existing coloured roster name for the overhead HUD; never copy it.
    pub(crate) fn player_label(&self, slot: u16) -> &str {
        self.roster
            .target(Some(slot))
            .and_then(|target| self.roster.display_name(target))
            .unwrap_or("")
    }

    /// Current plain roster name for console metadata recorded at send time.
    pub(crate) fn player_name(&self, slot: u16) -> Option<&str> {
        self.roster
            .target(Some(slot))
            .and_then(|target| self.roster.name(target))
    }

    /// Normalized body when this server event is our own echoed whisper.
    pub(crate) fn own_whisper_body(
        &self,
        kind: &ServerEventKind,
        text: &str,
        sender: Option<u16>,
    ) -> Option<String> {
        if *kind != ServerEventKind::Chat {
            return None;
        }
        let display = chat_display_text(&chat_unescape(text));
        let target = self
            .roster
            .target(sender)
            .or_else(|| self.line_sender(&display))?;
        if Some(target.slot()) != self.own_slot {
            return None;
        }
        let name = self.roster.display_name(target)?;
        private_whisper_body(&display, name)
    }
    pub(crate) fn new() -> Self {
        Self {
            lines: VecDeque::with_capacity(HISTORY_LIMIT),
            center: None,
            center_text: (0, 0),
            chat_text: [(0, 0); 2],
            combat: combat::Combat::default(),
            input: None,
            submitted: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            roster: ChatRoster::default(),
            scroll: 0,
            unread: 0,
            clock: Instant::now(),
            opened_ms: 0,
            notice: "",
            ui: MenuCanvas::with_text_capacity(512),
            pressed_action: None,
            pressed_who: None,
            visible_people: [None; MAX_VISIBLE],
            visible_blades: [None; MAX_VISIBLE],
            card: None,
            mute_request: None,
            settings: Vec::new(),
            menu_left: 0.0,
            map_block_request: None,
            map_blocks: (0, 0, false),
            muted_slots: 0,
            slot_marks: Vec::new(),
            spoken: [0; 32],
            marks_key: None,
            layout_viewport: [0.0; 2],
            scoreboard_layout: crate::scoreboard::behavior::ChatLayout::Normal,
            options: options::Options::default(),
            emojis: emoji::Emojis::default(),
            sjk_seen: None,
            sjk_epoch: None,
            sjk_officials: Vec::new(),
            sjk_outcome: 0,
            sjk_mark: None,
            sjk_sound_at: None,
            server_copies: server_copy::ServerCopies::default(),
            said_in_all: None,
            sjk_feed_visible: true,
            gifs_shown: true,
            gif_sources_revision: 0,
            own_slot: None,
            last_whisperer: None,
            last_channel: (Channel::Global, None),
            admin_server: false,
            clans: [false; 2],
            pointer_at: None,
            links: links::Spots::default(),
            link_copied_ms: None,
            encrypted_slots: 0,
            safety_request: None,
            reportable_slots: 0,
            report_request: None,
            trade_request: None,
            duel_partner: sjk_client::DuelPartner::default(),
            sjk_own_key: None,
            #[cfg(test)]
            for_shot: false,
        }
    }

    /// The client slots (bits) whose player takes encrypted whispers now.
    pub(crate) fn set_encrypted_slots(&mut self, slots: u64) {
        self.encrypted_slots = slots;
    }

    /// Whether the player in `slot` takes encrypted whispers now.
    fn takes_encrypted(&self, slot: u16) -> bool {
        slot < 64 && self.encrypted_slots & (1 << slot) != 0
    }

    /// The slot whose safety code a card asked for, once.
    pub(crate) fn take_safety_request(&mut self) -> Option<u16> {
        self.safety_request.take()
    }

    /// The client slots (bits) whose player's last encrypted whisper can be reported.
    pub(crate) fn set_reportable_slots(&mut self, slots: u64) {
        self.reportable_slots = slots;
    }

    /// Whether the last encrypted whisper from the player in `slot` can be reported.
    fn reportable(&self, slot: u16) -> bool {
        slot < 64 && self.reportable_slots & (1 << slot) != 0
    }

    /// The slot whose player a card asked to report, once.
    pub(crate) fn take_report_request(&mut self) -> Option<u16> {
        self.report_request.take()
    }

    /// The slot a card asked to trade with, once.
    pub(crate) fn take_trade_request(&mut self) -> Option<u16> {
        self.trade_request.take()
    }

    /// An overlay showing the installed world's emoji pictures.
    pub(crate) fn with_emojis(emojis: emoji::Emojis) -> Self {
        Self {
            emojis,
            ..Self::new()
        }
    }

    fn millis(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.clock)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }

    pub(crate) fn update_roster(&mut self, game: &sjk_protocol::GameState) {
        self.roster.update(game);
        self.own_slot = u16::try_from(game.client_num).ok();
    }

    /// Who is in each slot, as roster observations: one changes when its player
    /// leaves, another takes the slot or they rename.
    pub(crate) fn roster_targets(&self) -> [Option<ChatTarget>; crate::chat_mutes::SLOTS] {
        std::array::from_fn(|slot| self.roster.target(Some(slot as u16)))
    }

    /// The client slots muted on this PC changed (or not: this is called every frame):
    /// hide their lines, and show again those of players no longer muted.
    pub(crate) fn set_muted_slots(&mut self, slots: u32) {
        if self.muted_slots == slots {
            return;
        }
        self.muted_slots = slots;
        for index in 0..self.lines.len() {
            let line = &self.lines[index];
            // The player's own encrypted whispers are nobody else's lines.
            if line.hub.is_some() || (line.encrypted && line.sender.is_none()) {
                continue;
            }
            let muted = match line.sender {
                Some(target) => self.slot_muted(target),
                None => self.prefix_muted(&line.body),
            };
            self.lines[index].muted = muted;
        }
    }

    /// Whether `target`'s slot is muted on this PC.
    fn slot_muted(&self, target: ChatTarget) -> bool {
        target.slot() < 32 && self.muted_slots & (1 << target.slot()) != 0
    }

    /// Whether a line the server did not attribute starts with the name the game shows
    /// in a muted slot, then `: ` (colour codes ignored), as JA+ servers send chat.
    fn prefix_muted(&self, text: &str) -> bool {
        self.muted_slots != 0
            && (0..32_u16)
                .filter(|slot| self.muted_slots & (1 << slot) != 0)
                .filter_map(|slot| self.roster.target(Some(slot)))
                .filter_map(|target| self.roster.display_name(target))
                .any(|name| chat_body(text, name).0.len() < text.len())
    }

    /// Whether server chat `text` (from `sender`, when the server named one) comes from
    /// a player blocked on this PC, decided as the chat box decides it: the console
    /// leaves the line out too (hub report #191, 28/10/2026: a blocked player's name
    /// and messages still showed there).
    pub(crate) fn sent_by_muted(&self, text: &str, sender: Option<u16>) -> bool {
        if self.muted_slots == 0 {
            return false;
        }
        let display = chat_display_text(&chat_unescape(text));
        match self
            .roster
            .target(sender)
            .or_else(|| self.line_sender(&display))
        {
            Some(target) => self.slot_muted(target),
            None => self.prefix_muted(&display),
        }
    }

    /// Chat lines each client slot has sent: a counter that only grows (and wraps).
    pub(crate) fn spoken(&self) -> &[u32; 32] {
        &self.spoken
    }

    pub(crate) fn receive(
        &mut self,
        kind: ServerEventKind,
        text: String,
        sender: Option<u16>,
        now: Instant,
    ) {
        let private = kind == ServerEventKind::Chat && text.starts_with("\u{19}[");
        self.receive_classified(kind, text, sender, private, now);
    }

    pub(crate) fn receive_classified(
        &mut self,
        kind: ServerEventKind,
        mut text: String,
        sender: Option<u16>,
        server_private: bool,
        now: Instant,
    ) {
        // Console output never enters conversation history, including direct callers.
        if kind == ServerEventKind::Print {
            return;
        }
        let ms = self.millis(now);
        if kind == ServerEventKind::CenterPrint {
            text.reserve(1024usize.saturating_sub(text.len()));
            self.combat.height = None;
            self.center = Some((text, ms));
            return;
        }
        if kind == ServerEventKind::Chat && self.options.clean != 0 {
            let plain = options::clean_body(&text, 1).to_ascii_lowercase();
            if plain.contains("media - currently playing: ") || plain.contains("hi everybody!") {
                return;
            }
        }
        // A public line of a player whose name starts with the tell marker is no tell.
        let server_private =
            server_private && !sjk_client::forged_tell(&text, sender, &self.roster);
        // EternalJK's escapes for `%` and `"` show as what was typed, in the
        // chat box only, as in EternalJK.
        let display = chat_display_text(&chat_unescape(&text));
        // A line the server did not attribute (JA+) is its sender's when it starts
        // with one player's name.
        let target = self.roster.target(sender).or_else(|| {
            matches!(kind, ServerEventKind::Chat | ServerEventKind::TeamChat)
                .then(|| self.line_sender(&display))
                .flatten()
        });
        if let Some(target) = target
            && let Some(count) = self.spoken.get_mut(usize::from(target.slot()))
        {
            *count = count.wrapping_add(1);
        }
        // Display keeps `^n`; identity, whisper destinations and muting keep
        // using the plain roster name.
        let name = target
            .and_then(|target| self.roster.display_name(target))
            .unwrap_or("")
            .to_owned();
        let (body, private) = if name.is_empty() {
            (display.as_str(), false)
        } else {
            chat_body(&display, &name)
        };
        match target {
            Some(_) => self.note_whisper(private, target),
            // The line stays unattributed; only `reply` reads its sender from it.
            None if kind == ServerEventKind::Chat => {
                let sender = self.whisper_sender(&display);
                self.note_whisper(sender.is_some(), sender);
            }
            None => {}
        }
        let body = options::clean_body(body, self.options.clean);
        let (body, emojis) = if self.options.emojis {
            self.emojis.markup(&body)
        } else {
            (body, Vec::new())
        };
        if self.options.clean != 0
            && self.lines.back().is_some_and(|line| {
                line.sender == target && line.body == body && line.emojis == emojis
            })
        {
            return;
        }
        let line = ChatLine {
            name,
            body,
            sender: target,
            channel: if private {
                Channel::Whisper
            } else if tabs::is_admin_line(&display) {
                Channel::Admin
            } else if kind == ServerEventKind::TeamChat {
                Channel::Team
            } else {
                Channel::Global
            },
            server_private,
            received_ms: ms,
            // Muted on this PC (`muted_players.rs`).
            muted: match target {
                Some(target) => self.slot_muted(target),
                None => self.prefix_muted(&display),
            },
            emojis,
            wrap: layout::Wrapped::default(),
            y: None,
            hub: None,
            gif: None,
            encrypted: false,
            verified: false,
            recipient: None,
            loud: sjk_client::quiet_passes(
                private || server_private,
                sjk_client::line_sender(&display, sender, &self.roster),
                self.own_slot,
                self.duel_partner.target(),
            ),
        };
        self.push_line(line, server_private);
    }

    /// Whether the newest line is a server's private message from a player not muted
    /// here: what flashes the taskbar (`chat_sounds.rs`).
    pub(crate) fn newest_is_open_private(&self) -> bool {
        self.lines
            .back()
            .is_some_and(|line| line.server_private && !line.muted)
    }

    fn push_line(&mut self, line: ChatLine, private: bool) {
        if self.lines.len() == HISTORY_LIMIT {
            self.lines.pop_front();
        }
        let shown = self.quiet_visible(&line);
        self.lines.push_back(line);
        if self.is_typing() && self.scroll > 0 && (!self.options.pm_only || private) && shown {
            self.scroll = (self.scroll + 1).min(self.lines.len().saturating_sub(1));
            self.unread = (self.unread + 1).min(HISTORY_LIMIT);
        }
    }

    /// Show an end-to-end encrypted whisper (`e2e_whispers.rs`): from the player in
    /// `from` to us, or ours to the player named `to` in `slot` (`from` `None`);
    /// `verified` when they were checked with the safety code. Its sender becomes the
    /// one `reply` answers. False when the slot holds nobody.
    pub(crate) fn receive_encrypted(
        &mut self,
        from: Option<u16>,
        (slot, to): (u16, &str),
        text: &str,
        verified: bool,
        now: Instant,
    ) -> bool {
        let recipient = from.is_none().then_some(slot);
        let (name, sender, muted) = match from {
            Some(slot) => {
                let Some(target) = self.roster.target(Some(slot)) else {
                    return false;
                };
                let name = self.roster.display_name(target).unwrap_or("").to_owned();
                if let Some(count) = self.spoken.get_mut(usize::from(slot)) {
                    *count = count.wrapping_add(1);
                }
                self.note_whisper(true, Some(target));
                (name, Some(target), self.slot_muted(target))
            }
            None => (format!("To {to}"), None, false),
        };
        let body = options::clean_body(text, self.options.clean);
        let (body, emojis) = if self.options.emojis {
            self.emojis.markup(&body)
        } else {
            (body, Vec::new())
        };
        let line = ChatLine {
            name,
            body,
            sender,
            channel: Channel::Whisper,
            server_private: true,
            received_ms: self.millis(now),
            muted,
            emojis,
            wrap: layout::Wrapped::default(),
            y: None,
            hub: None,
            gif: None,
            encrypted: true,
            verified,
            recipient,
            loud: true,
        };
        self.push_line(line, true);
        true
    }

    pub(crate) fn open(&mut self, team: bool) {
        self.input = Some(Editor::new(if team {
            Channel::Team
        } else {
            Channel::Global
        }));
        self.pressed_action = None;
        self.scroll = 0;
        self.unread = 0;
        self.notice = "";
        self.opened_ms = self.millis(Instant::now());
    }

    /// Open the field and type `text` into it, for world shots.
    #[cfg(test)]
    pub(crate) fn draft_for_shot(&mut self, text: &str) {
        self.open(false);
        if let Some(input) = &mut self.input {
            input.insert(text);
        }
    }

    pub(crate) fn is_typing(&self) -> bool {
        self.input.is_some()
    }

    /// After a submitted draft went to the sender: dropped when it was sent, the
    /// composer reopened on it when it was refused.
    pub(crate) fn settle_submitted(&mut self, sent: bool) {
        let draft = self.submitted.take();
        if !sent && self.input.is_none() {
            self.input = draft;
        }
    }

    /// Hand the open composer, its channel and draft, to the next world's chat, so a
    /// field open over a map change stays open after it, as EternalJK keeps its message
    /// field (a remote map change resets no key catcher there) and with it the talk
    /// balloon (`BUTTON_TALK`; report #134). The feed's lines stay behind, as a new
    /// map's cgame starts its chat box empty.
    pub(crate) fn hand_composer_to(&mut self, to: &mut ChatOverlay) {
        let Some(input) = self.input.take() else {
            return;
        };
        to.input = Some(input);
        to.modifiers = self.modifiers;
        to.scroll = 0;
        to.unread = 0;
        // Already open: no fade-in on the new world's clock.
        to.opened_ms = 0;
    }

    /// What SJK knows of each slot's player, as the frame sets it (world shots).
    #[cfg(test)]
    pub(crate) fn set_slot_marks(&mut self, marks: Vec<marks::SlotMark>) {
        self.slot_marks = marks;
    }

    /// Close the composer, its draft dropped, as Escape does.
    pub(crate) fn close_composer(&mut self) {
        self.input = None;
        self.scroll = 0;
        self.unread = 0;
    }

    /// Toggle a whisper composer only for a currently occupied roster slot.
    pub(crate) fn whisper_to(&mut self, slot: Option<u16>) -> bool {
        let Some(target) = self.roster.target(slot) else {
            return false;
        };
        if self.is_typing() {
            self.input = None;
            return true;
        }
        self.open(false);
        let input = self.input.as_mut().expect("opened composer");
        input.channel = Channel::Whisper;
        input.recipient = Some(target);
        true
    }

    /// Whether a world shot asked for the feed (never outside the tests).
    fn shown_for_shot(&self) -> bool {
        #[cfg(test)]
        return self.for_shot;
        #[cfg(not(test))]
        false
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        self.ui.draw_list()
    }

    pub(crate) fn set_scoreboard_layout(
        &mut self,
        enabled: crate::scoreboard::behavior::ChatLayout,
    ) {
        if self.scoreboard_layout != enabled {
            self.scoreboard_layout = enabled;
            for line in &mut self.lines {
                line.y = None;
            }
            self.pressed_action = None;
        }
    }

    /// Lay out and append chat text: the chat box in the small game font when
    /// `ui_gameFont` has it loaded (`CG_ChatBox_DrawStrings` paints with
    /// `FONT_SMALL`), centre prints in the headline font (where retail's
    /// `CG_DrawCenterString` drew `FONT_MEDIUM`), otherwise in `font`.
    pub(crate) fn append(
        &mut self,
        draw_feed: bool,
        fonts: &mut GameFonts,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
        _scale: f32,
    ) {
        let ms = self.millis(Instant::now());
        self.build(
            draw_feed,
            fonts.font(chat_font(fonts)).unwrap_or(font),
            viewport,
            ms,
        );
        self.append_built(fonts, vertices, font, viewport);
    }

    /// Append the text [`Self::build`] laid out, each run in its font and palette.
    fn append_built(
        &self,
        fonts: &mut GameFonts,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let (start, end) = self.center_text;
        let palette_of = self.palette_of();
        let chat = chat_font(fonts);
        self.ui.append_text_routed(
            fonts,
            |id, _| {
                Some(if (start..end).contains(&id.0) {
                    RetailFont::Headline
                } else {
                    chat
                })
            },
            |id| palette_of(id.0),
            vertices,
            font,
            viewport,
        );
    }

    /// The palette each text run of this frame draws in, by its id: with
    /// `cg_chatBoxOutline` on, the feed's and the composer's in a dark outline
    /// ([`crate::text::CodePalette::DarkOutlined`]); with `cg_darkTextOutline` on, their
    /// dark letters in a white one; the rest as the game draws it.
    fn palette_of(&self) -> impl Fn(u32) -> crate::text::CodePalette + Copy + use<> {
        use crate::text::CodePalette;
        let feed = match (self.options.outline, self.options.white) {
            (true, white) => CodePalette::DarkOutlined.white_outlined(white),
            (false, true) => CodePalette::WhiteEdged,
            (false, false) => CodePalette::Game,
        };
        let chat = self.chat_text;
        move |id| {
            if chat.iter().any(|(start, end)| (*start..*end).contains(&id)) {
                feed
            } else {
                CodePalette::Game
            }
        }
    }
}

/// The font the chat box draws in: the player's own (`cg_chatFont`) once it is
/// loaded, else the small game font (`FONT_SMALL`), which falls back to Inter.
fn chat_font(fonts: &GameFonts) -> RetailFont {
    if fonts.active(RetailFont::Chat) {
        RetailFont::Chat
    } else {
        RetailFont::Small
    }
}

/// Parse the private grammar rather than requiring the optional 0x19 marker:
/// compatible servers do not all include that byte in outgoing whisper echoes.
fn private_whisper_body(display: &str, name: &str) -> Option<String> {
    let (body, private) = chat_body(display, name);
    private.then(|| sjk_client::chat_plain_text(&chat_unescape(body)))
}

pub(crate) enum ChatInputResult {
    None,
    Submit(String),
    /// A message for the SJK chat, to send through the hub.
    Sjk(String),
    /// A message for a clan chat, to send through the hub.
    Clan(String, sjk_identity::HubChannel),
}

#[cfg(test)]
mod escape_tests {
    use super::*;

    // Report #134 (Nib): a chat field open over a map change closed with the old
    // world's chat, and the talk balloon with it.
    #[test]
    fn the_open_field_and_its_draft_go_to_the_next_world() {
        let mut old = ChatOverlay::new();
        old.draft_for_shot("gg");
        old.input.as_mut().expect("composer open").channel = Channel::Team;
        let mut next = ChatOverlay::new();
        old.hand_composer_to(&mut next);
        assert!(!old.is_typing() && next.is_typing());
        let input = next.input.as_ref().expect("composer open");
        assert_eq!((input.text.as_str(), input.channel), ("gg", Channel::Team));
        // A closed field opens nothing.
        let mut closed = ChatOverlay::new();
        old.hand_composer_to(&mut closed);
        assert!(!closed.is_typing());
    }

    #[test]
    fn eternaljk_escapes_show_as_what_was_typed() {
        let mut chat = ChatOverlay::new();
        chat.receive(
            ServerEventKind::Chat,
            "^7Creyon: ^2100\u{b0}/. ''sure''".to_owned(),
            None,
            Instant::now(),
        );
        assert_eq!(chat.lines[0].body, "^7Creyon: ^2100% \"sure\"");
    }

    #[test]
    fn a_servers_chat_bytes_show_the_percent() {
        // As a JA+ server sends it: a Latin-1 name, 0x19 separators and the
        // escape's degree sign as byte 0xB0, which is not UTF-8.
        let bytes = b"jof\xbbdmg\xabcreyon^7\x19: ^2\xb0/.\x19";
        let text = sjk_client::decode_legacy(bytes).into_owned();
        for emojis in [false, true] {
            let mut chat = ChatOverlay::new();
            chat.options.emojis = emojis;
            chat.receive(ServerEventKind::Chat, text.clone(), None, Instant::now());
            assert!(
                chat.lines[0].body.ends_with("^2%"),
                "{:?}",
                chat.lines[0].body
            );
        }
    }
}

#[cfg(test)]
mod whisper_echo_tests {
    use super::*;

    #[test]
    fn private_echo_does_not_need_the_optional_control_marker() {
        assert_eq!(
            private_whisper_body("[Sol]: hello ^1there", "Sol"),
            Some("hello there".to_owned())
        );
    }

    #[test]
    fn public_chat_is_not_an_outgoing_whisper_echo() {
        assert_eq!(private_whisper_body("Sol: hello", "Sol"), None);
    }
}
