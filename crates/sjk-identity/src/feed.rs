//! The hub's feed (`PROTOCOL.md`, "The feed"): a thread of its own that holds a long
//! poll open at the hub and keeps the SJK chat's last messages and online count, and
//! the emotes, pings and looks received, so the identity worker's claims and reports
//! never wait behind it.
//!
//! The worker tells it, through [`FeedShared`], which hub to read (only while the
//! identity is on and the hub answered the registration, and then while the player
//! is on a game server or the chat is on), which game server's emotes and looks to ask
//! for, and whether the chat shows. With the chat off the feed still reads on a game
//! server, for the looks and emotes, but keeps no message: [`ChatState`] stays as an
//! idle feed leaves it. With no hub to read it makes no request.
//!
//! A holocron drop (`PROTOCOL.md`, "The feed") joins the chat's messages in the feed's
//! order as a [`ChatMessage`] whose [`ChatMessage::holocron`] is set, so the game's
//! chat, the docked chat and the chat page show it where it was said. A drop of the
//! player's own key also tells the identity worker ([`OwnDrop`]), which reads the own
//! profile and holocron progress again soon.
//!
//! An announcement or a reward round (`PROTOCOL.md`, "Announcements", "Reward rounds")
//! joins the messages the same way, as a [`ChatMessage`] whose [`ChatMessage::official`]
//! is set, and the worker is told of it ([`FeedNews`]): it keeps the announcements for
//! the viewer and reads the rewards again. Announcements read from the hub rather than
//! the feed (made while the player was away) join the chat through
//! [`post_announcements`].
//!
//! The clan chats ([`crate::channels`], `PROTOCOL.md` "Clan chats") come through the
//! same poll for the ones the worker says the player joined ([`FeedShared::channels`]);
//! their messages are kept apart ([`ChatState::clan`]), so the docked chat and the chat
//! page, which show the SJK chat, never show them. Joining or leaving one starts the
//! reading again from `after` 0 under a new epoch ([`ChatState::loaded`]), which
//! brings its backlog.
//!
//! Another game server starts the reading again from `after` 0, whose answer carries
//! that server's looks of the last minute. The looks queued for the viewer carry the
//! server they were read for and the reading's generation ([`FeedShared::generation`],
//! which changes with the hub, the server or the identity going off or on), and
//! [`take_looks`] hands over only those of the current generation and the viewer's
//! server, so a poll still under way when either changed cannot bring old looks back.

use crate::hub::{Hub, HubError};
use crate::keys::Identity;
use crate::news::{Announcement, Official};
use crate::service::{HubFactory, ReportOutcome};
use crate::wire::{
    ChatMessage, DropEvent, DropMark, Emote, Feed, FindEvent, FindMark, LookEvent, People, Ping,
    ToyEvent,
};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Messages kept, as many as the hub keeps.
pub const MESSAGES_KEPT: usize = 200;
/// How long the hub may hold a poll when nothing is new, in seconds.
pub const WAIT: u64 = 25;
/// The feed's HTTP timeout: the hub's wait and room for a slow answer.
pub const TIMEOUT: Duration = Duration::from_secs(40);
/// Shortest time between the starts of two polls: a ping, emote or toy event that comes
/// just after another answer waits at most this long (2 seconds before 27/10/2026).
const POLL_GAP: Duration = Duration::from_millis(500);
/// The gap a hub from before 27/10/2026 wants: it takes 40 polls a minute from a key and
/// answers more with 429 `feed_quota`, after which the feed keeps to this.
const OLD_POLL_GAP: Duration = Duration::from_secs(2);
/// First wait after a failure, doubled up to [`RETRY_MAX`].
const RETRY_MIN: Duration = Duration::from_secs(2);
const RETRY_MAX: Duration = Duration::from_secs(60);
/// How often an idle feed looks at what it is told.
pub(crate) const IDLE: Duration = Duration::from_secs(1);
/// Emotes waiting for the viewer, the newest kept.
const EMOTES_WAITING: usize = 64;
/// Looks waiting for the viewer, the newest kept.
const LOOKS_WAITING: usize = 64;
/// Pings waiting for the viewer, the newest kept.
const PINGS_WAITING: usize = 64;

/// What the feed thread calls when a holocron dropped for the player's own key.
pub(crate) type OwnDrop = Box<dyn Fn() + Send>;

/// What the feed tells the identity worker of the SJK team's news.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FeedNews {
    /// Announcements relayed, oldest first.
    Announcements(Vec<Announcement>),
    /// A reward round was made: the rewards are read again.
    Round,
}

/// What the feed thread calls with the SJK team's news.
pub(crate) type NewsSink = Box<dyn Fn(FeedNews) + Send>;

/// What the chat shows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChatState {
    /// The last messages, oldest first.
    pub messages: VecDeque<ChatMessage>,
    /// The last messages of the clan chats read ([`crate::channels`]), oldest first,
    /// [`MESSAGES_KEPT`] at most, each with its [`ChatMessage::channel`].
    pub clan: VecDeque<ChatMessage>,
    /// The clan chats the hub lets the player read, as it last listed them; empty for a
    /// player who may read none (and from hubs before clan chats).
    pub channels: Vec<crate::ListedChannel>,
    /// Counts changes to `messages`, `online`, `people` and `live`, so a reader that derives from
    /// them knows when to derive again.
    pub revision: u64,
    /// Keys that read the chat in the last minute, as the hub last said.
    pub online: u32,
    /// Who reads the chat now and who read it last, as the hub last listed them;
    /// `None` until it does (hubs before the list never do).
    pub people: Option<People>,
    /// The last poll reached the hub.
    pub live: bool,
    /// What became of the last message or emote sent.
    pub outcome: Option<ReportOutcome>,
    /// Result of the most recent reaction request, separate from sending text.
    pub reaction: Option<ReportOutcome>,
    /// What became of the last translations asked for ([`crate::Service::translate`]),
    /// oldest first, [`crate::translate::OUTCOMES_KEPT`] at most; a reader picks its
    /// own out by serial.
    pub translations: VecDeque<crate::TranslateOutcome>,
    /// The registered canonical person id, copied without allocating for UI snapshots.
    pub own_key: Option<[u8; 16]>,
    /// What became of the last chat ban asked for ([`crate::Service::chat_ban`]).
    pub ban: Option<crate::BanOutcome>,
    /// `None` until the hub being read answered once; then a number that changes with
    /// each hub (or each time the reading starts again), so a reader can tell the
    /// backlog that came with that first answer from what is said afterwards.
    pub loaded: Option<u64>,
}

/// What the identity worker tells the feed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct FeedShared {
    /// The hub to read; `None` while there is none to read.
    pub(crate) url: Option<String>,
    /// The game server whose emotes and looks to ask for.
    pub(crate) server: Option<String>,
    /// Whether the SJK chat shows: without it no message is kept.
    pub(crate) chat: bool,
    /// The clan chats the player joined ([`crate::channels`]), asked for with the chat.
    pub(crate) channels: Vec<crate::HubChannel>,
    /// The player's own id at the hub ([`crate::Snapshot::key_id`]), to tell their
    /// drops from others'; empty for this PC's key.
    pub(crate) person: String,
    /// Changes whenever `url` or `server` does: looks read before belong to another
    /// reading (another hub or server, or the identity since turned off).
    pub(crate) generation: u64,
}

/// A look event queued for the viewer, with what it was read for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueuedLook {
    /// [`FeedShared::generation`] when the poll that brought it began.
    generation: u64,
    /// The game server that poll asked for.
    server: Option<SocketAddr>,
    event: LookEvent,
}

/// The look events the feed received for the viewer's server under the current
/// reading ([`crate::Service::take_looks`]).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReceivedLooks {
    /// The reading's generation: when it changes (the identity went off or on, another
    /// hub or server), every look received before is out of date, the roster's too.
    pub generation: u64,
    /// The events, oldest first.
    pub events: Vec<LookEvent>,
}

/// Take the queued looks: those read under the current generation for `server`, the
/// game server the viewer is on (it may be ahead of the worker); the rest are dropped.
pub(crate) fn take_looks(
    shared: &Mutex<FeedShared>,
    queue: &Mutex<VecDeque<QueuedLook>>,
    server: Option<SocketAddr>,
) -> ReceivedLooks {
    // Under the shared lock, so no new generation's looks are dropped as old ones.
    let shared = lock(shared);
    let mut queue = lock(queue);
    let events = queue
        .drain(..)
        .filter(|queued| {
            queued.generation == shared.generation && server.is_some() && queued.server == server
        })
        .map(|queued| queued.event)
        .collect();
    ReceivedLooks {
        generation: shared.generation,
        events,
    }
}

/// A ping or toy event queued for the viewer, with the game server the poll that
/// brought it asked about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ForServer<T> {
    server: Option<SocketAddr>,
    item: T,
}

impl<T> ForServer<T> {
    /// `item`, read for `server`.
    pub(crate) fn new(server: Option<SocketAddr>, item: T) -> Self {
        Self { server, item }
    }
}

/// A ping queued for the viewer.
pub(crate) type QueuedPing = ForServer<Ping>;
/// A toy event queued for the viewer.
pub(crate) type QueuedToyEvent = ForServer<ToyEvent>;

/// Take the queued pings read for `server`, the game server the viewer is on; those read
/// for another server (a poll under way when the player changed servers) are dropped.
pub(crate) fn take_pings(
    queue: &Mutex<VecDeque<QueuedPing>>,
    server: Option<SocketAddr>,
) -> Vec<Ping> {
    take_for_server(queue, server)
}

/// Take the queued toy events read for `server`, as [`take_pings`].
pub(crate) fn take_toy_events(
    queue: &Mutex<VecDeque<QueuedToyEvent>>,
    server: Option<SocketAddr>,
) -> Vec<ToyEvent> {
    take_for_server(queue, server)
}

pub(crate) fn take_for_server<T>(
    queue: &Mutex<VecDeque<ForServer<T>>>,
    server: Option<SocketAddr>,
) -> Vec<T> {
    let mut queue = lock(queue);
    if server.is_none() {
        queue.clear();
        return Vec::new();
    }
    queue
        .drain(..)
        .filter(|queued| queued.server == server)
        .map(|queued| queued.item)
        .collect()
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The feed's logic, advanced by [`FeedWorker::step`] with an explicit clock.
pub(crate) struct FeedWorker {
    identity: Identity,
    make_hub: HubFactory,
    shared: Arc<Mutex<FeedShared>>,
    state: Arc<Mutex<ChatState>>,
    emotes: Arc<Mutex<VecDeque<Emote>>>,
    looks: Arc<Mutex<VecDeque<QueuedLook>>>,
    /// The pings received, for [`take_pings`].
    pings: Arc<Mutex<VecDeque<QueuedPing>>>,
    /// The toy events received, for [`take_toy_events`].
    toys: Arc<Mutex<VecDeque<QueuedToyEvent>>>,
    hub: Option<Box<dyn Hub>>,
    url: Option<String>,
    /// The player's own id at the hub, to tell their drops from others': this PC's key
    /// id, or the person's id the worker tells ([`FeedShared::person`]).
    key_id: String,
    /// This PC's key id.
    local_key_id: String,
    /// Called when a drop of the player's own key arrives.
    own_drop: OwnDrop,
    /// The newest own drop told of, so a replay of the feed's last minute is not news.
    own_drop_told: u64,
    /// Called when one of the player's trades changed ([`FeedWorker::on_trade`]).
    own_trade: OwnDrop,
    /// Where the SJK team's news goes ([`FeedNews`]), and the newest feed ids of
    /// announcements and rounds told of.
    news: Option<NewsSink>,
    news_told: (u64, u64),
    /// The game server read for, as last told.
    server: Option<String>,
    /// Whether the chat shows, as last told.
    chat: bool,
    /// The clan chats asked for, as last told.
    channels: Vec<crate::HubChannel>,
    after: u64,
    due: Instant,
    backoff: Duration,
    /// [`POLL_GAP`], or [`OLD_POLL_GAP`] once this hub refused a poll as too soon.
    gap: Duration,
    /// Counts the hubs read (`ChatState::loaded`).
    epoch: u64,
}

impl FeedWorker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity: Identity,
        make_hub: HubFactory,
        shared: Arc<Mutex<FeedShared>>,
        state: Arc<Mutex<ChatState>>,
        emotes: Arc<Mutex<VecDeque<Emote>>>,
        looks: Arc<Mutex<VecDeque<QueuedLook>>>,
        own_drop: OwnDrop,
        now: Instant,
    ) -> Self {
        Self {
            key_id: identity.key_id(),
            local_key_id: identity.key_id(),
            own_drop,
            own_drop_told: 0,
            own_trade: Box::new(|| {}),
            news: None,
            news_told: (0, 0),
            identity,
            make_hub,
            shared,
            state,
            emotes,
            looks,
            pings: Arc::default(),
            toys: Arc::default(),
            hub: None,
            url: None,
            server: None,
            chat: false,
            channels: Vec::new(),
            after: 0,
            due: now,
            backoff: RETRY_MIN,
            gap: POLL_GAP,
            epoch: 0,
        }
    }

    /// Call `told` whenever the feed tells of a change to one of the player's trades.
    pub(crate) fn on_trade(mut self, told: OwnDrop) -> Self {
        self.own_trade = told;
        self
    }

    /// Tell `sink` of the announcements and reward rounds the feed relays.
    pub(crate) fn set_news(&mut self, sink: NewsSink) {
        self.news = Some(sink);
    }

    /// The queue the pings received go to ([`take_pings`]).
    pub(crate) fn pings(&self) -> Arc<Mutex<VecDeque<QueuedPing>>> {
        Arc::clone(&self.pings)
    }

    /// The queue the toy events received go to ([`take_toy_events`]).
    pub(crate) fn toys(&self) -> Arc<Mutex<VecDeque<QueuedToyEvent>>> {
        Arc::clone(&self.toys)
    }

    /// Poll the hub if it is time to; return how long to wait before the next step.
    /// A poll blocks for as long as the hub holds it (at most [`WAIT`] seconds).
    pub(crate) fn step(&mut self, now: Instant) -> Duration {
        let shared = lock(&self.shared).clone();
        let person = if shared.person.is_empty() {
            &self.local_key_id
        } else {
            &shared.person
        };
        if *person != self.key_id {
            self.key_id.clone_from(person);
        }
        if shared.url != self.url {
            self.chat = shared.chat;
            self.channels.clone_from(&shared.channels);
            self.switch(shared.url.clone(), now);
        } else if shared.chat != self.chat {
            self.channels.clone_from(&shared.channels);
            self.show_chat(shared.chat, now);
        } else if shared.channels != self.channels {
            // A clan chat joined or left: read again from the backlog, under a new epoch,
            // as when the chat is turned on, so the joined chat's history is marked as
            // backlog rather than shown as news.
            self.channels.clone_from(&shared.channels);
            self.show_chat(self.chat, now);
        }
        // Another server: its looks of the last minute come with `after` 0, and the
        // old server's ids say nothing of it.
        if shared.server != self.server {
            self.server.clone_from(&shared.server);
            self.after = 0;
        }
        if self.url.is_none() {
            return IDLE;
        }
        if now < self.due {
            return self.due - now;
        }
        let Some(hub) = self.hub.as_mut() else {
            return IDLE;
        };
        // Clan chats only with the chat on: off, the feed keeps no message.
        let channels: &[crate::HubChannel] = if self.chat { &self.channels } else { &[] };
        match hub.feed(
            &self.identity,
            self.after,
            shared.server.as_deref(),
            channels,
            WAIT,
        ) {
            Ok(feed) => {
                self.apply(feed, &shared);
                self.backoff = RETRY_MIN;
                self.due = now + self.gap;
            }
            // An older hub's quota: the poll did not count, the feed is still live.
            Err(HubError::Rejected {
                status: 429, code, ..
            }) if code == "feed_quota" => {
                self.gap = OLD_POLL_GAP;
                self.due = now + OLD_POLL_GAP;
            }
            Err(_) => {
                self.set_live(false);
                self.due = now + self.backoff;
                self.backoff = (self.backoff * 2).min(RETRY_MAX);
            }
        }
        self.due.saturating_duration_since(now)
    }

    /// Read another hub, or none: start again from its backlog.
    fn switch(&mut self, url: Option<String>, now: Instant) {
        self.hub = url.as_deref().and_then(|url| (self.make_hub)(url).ok());
        self.url = url;
        self.after = 0;
        self.own_drop_told = 0;
        self.news_told = (0, 0);
        self.due = now;
        self.backoff = RETRY_MIN;
        self.gap = POLL_GAP;
        self.clear_chat();
    }

    /// The chat was turned on or off while the same hub is read (on a game server).
    /// Off, the messages go; on, the reading starts again from the hub's backlog, as
    /// it would have had the feed been idle.
    fn show_chat(&mut self, on: bool, now: Instant) {
        self.chat = on;
        if on {
            self.after = 0;
            self.due = now;
        }
        self.clear_chat();
    }

    /// Forget the messages: a new backlog, under a new epoch, is due.
    fn clear_chat(&mut self) {
        self.epoch += 1;
        let mut state = lock(&self.state);
        state.loaded = None;
        state.messages.clear();
        state.clan.clear();
        state.channels.clear();
        state.live = false;
        state.online = 0;
        state.people = None;
        state.revision += 1;
    }

    fn set_live(&self, live: bool) {
        if !self.chat {
            return;
        }
        let mut state = lock(&self.state);
        if state.live != live {
            state.live = live;
            state.revision += 1;
        }
    }

    /// Take an answer to a poll made as `polled` says into the state: its messages
    /// only while the chat shows, its looks marked with the reading they belong to.
    fn apply(&mut self, mut feed: Feed, polled: &FeedShared) {
        self.tell_own_drops(&feed.drops);
        // Only the trade's two players are told; a replay after `after` 0 reads the list
        // once more, which is harmless.
        if !feed.trades.is_empty() {
            (self.own_trade)();
        }
        self.tell_news(&feed);
        if self.chat {
            self.apply_chat(&mut feed, polled.person.as_bytes().try_into().ok());
        }
        queue(&self.emotes, feed.emotes, EMOTES_WAITING);
        let server = polled
            .server
            .as_deref()
            .and_then(|server| server.parse().ok());
        let pings = feed
            .pings
            .into_iter()
            .map(|item| QueuedPing { server, item });
        queue(&self.pings, pings, PINGS_WAITING);
        let toys = feed
            .toys
            .into_iter()
            .map(|item| QueuedToyEvent { server, item });
        queue(&self.toys, toys, PINGS_WAITING);
        let looks = feed.looks.into_iter().map(|event| QueuedLook {
            generation: polled.generation,
            server,
            event,
        });
        queue(&self.looks, looks, LOOKS_WAITING);
        self.after = feed.next;
    }

    /// Tell the worker of the newest drop for the player's own key not told of yet.
    fn tell_own_drops(&mut self, drops: &[DropEvent]) {
        let newest = drops
            .iter()
            .filter(|drop| drop.key_id == self.key_id)
            .map(|drop| drop.id)
            .max();
        if let Some(id) = newest.filter(|id| *id > self.own_drop_told) {
            self.own_drop_told = id;
            (self.own_drop)();
        }
    }

    /// Tell the worker of the announcements and rounds not told of yet.
    fn tell_news(&mut self, feed: &Feed) {
        let Some(sink) = &self.news else {
            return;
        };
        let (announced, rounds) = self.news_told;
        let new: Vec<Announcement> = feed
            .announcements
            .iter()
            .filter(|relayed| relayed.id > announced)
            .map(|relayed| relayed.announcement.clone())
            .collect();
        if let Some(newest) = feed.announcements.iter().map(|relayed| relayed.id).max() {
            self.news_told.0 = announced.max(newest);
        }
        if !new.is_empty() {
            sink(FeedNews::Announcements(new));
        }
        if let Some(newest) = feed.rounds.iter().map(|round| round.id).max()
            && newest > rounds
        {
            self.news_told.1 = newest;
            sink(FeedNews::Round);
        }
    }

    /// Take an answer's messages, deletions, online count and people into the chat.
    fn apply_chat(&self, feed: &mut Feed, own_key: Option<[u8; 16]>) {
        let mut state = lock(&self.state);
        let mut changed =
            !state.live || state.online != feed.online || state.loaded != Some(self.epoch);
        changed |= state.own_key != own_key;
        state.own_key = own_key;
        state.live = true;
        state.online = feed.online;
        // The hub lists them now and then: an answer without the list keeps the last.
        if let Some(people) = feed.people.take()
            && state.people.as_ref() != Some(&people)
        {
            state.people = Some(people);
            changed = true;
        }
        if state.channels != feed.channels {
            state.channels = std::mem::take(&mut feed.channels);
            changed = true;
        }
        // Ids that go backwards mean the hub restarted: its backlog starts afresh.
        if feed.next < self.after {
            changed |= !state.messages.is_empty() || !state.clan.is_empty();
            state.messages.clear();
            state.clan.clear();
        }
        if !feed.deleted.is_empty() {
            let before = state.messages.len() + state.clan.len();
            // The SJK team's lines are not the hub's messages, whatever id they hold.
            state.messages.retain(|message| {
                message.official.is_some() || !feed.deleted.contains(&message.id)
            });
            state
                .clan
                .retain(|message| !feed.deleted.contains(&message.id));
            changed |= state.messages.len() + state.clan.len() != before;
        }
        // Drops, finds, announcements and rounds share the messages' ids: merged, the
        // chat keeps the order they came in.
        let mut incoming = std::mem::take(&mut feed.chat);
        if !feed.drops.is_empty()
            || !feed.finds.is_empty()
            || !feed.announcements.is_empty()
            || !feed.rounds.is_empty()
        {
            let own = &self.key_id;
            incoming.extend(feed.drops.drain(..).map(|drop| drop_message(drop, own)));
            incoming.extend(feed.finds.drain(..).map(|find| find_message(find, own)));
            incoming.extend(feed.announcements.drain(..).map(|relayed| {
                let at = relayed.announcement.at;
                official_message(relayed.id, at, Official::Announcement(relayed.announcement))
            }));
            incoming.extend(
                feed.rounds
                    .drain(..)
                    .map(|round| official_message(round.id, round.at, Official::Round(round))),
            );
            incoming.sort_by_key(|message| message.id);
        }
        for message in incoming {
            let held = match message.channel {
                crate::HubChannel::Sjk => &mut state.messages,
                // Only the clan chats asked for; a channel this client does not know
                // is left out.
                channel if self.channels.contains(&channel) => &mut state.clan,
                _ => continue,
            };
            changed |= keep(held, message);
        }
        for update in feed.reaction_updates.drain(..) {
            let ChatState { messages, clan, .. } = &mut *state;
            if let Some(message) = messages
                .iter_mut()
                .chain(clan.iter_mut())
                .find(|m| m.id == update.message && m.holocron.is_none() && m.official.is_none())
            {
                if message.reactions != update.reactions {
                    message.reactions = update.reactions;
                    changed = true;
                }
            }
        }
        state.loaded = Some(self.epoch);
        if changed {
            state.revision += 1;
        }
    }
}

/// Put `message` at the end of `held` (one channel's messages), keeping
/// [`MESSAGES_KEPT`]; a message `held` has already only refreshes its reactions (an
/// `after` 0 reading replays the backlog). Whether `held` changed.
fn keep(held: &mut VecDeque<ChatMessage>, message: ChatMessage) -> bool {
    if held.back().is_some_and(|last| message.id <= last.id) {
        if let Some(known) = held.iter_mut().find(|known| {
            known.id == message.id
                && known.official.is_none()
                && known.holocron.is_none()
                && known.key_id == message.key_id
                && known.text == message.text
        }) && known.reactions != message.reactions
        {
            known.reactions = message.reactions;
            return true;
        }
        return false;
    }
    if held.len() == MESSAGES_KEPT {
        held.pop_front();
    }
    held.push_back(message);
    true
}

/// The chat entry for `drop`, as `own` (the player's key id) sees it. Its text is left
/// empty: the viewer words the line from its tier.
fn drop_message(drop: DropEvent, own: &str) -> ChatMessage {
    ChatMessage {
        reactions: Default::default(),
        id: drop.id,
        at: drop.at,
        holocron: Some(DropMark {
            own: drop.key_id == own,
            tier: drop.tier,
            find: None,
        }),
        official: None,
        key_id: drop.key_id,
        name: drop.name,
        verified: drop.verified,
        tick: crate::Tick::None,
        staff: false,
        creator: false,
        trust: None,
        emoji: String::new(),
        client: String::new(),
        text: String::new(),
        channel: Default::default(),
    }
}

/// The chat entry for the grand find `find`, as `own` (the player's key id) sees it: a
/// drop line marked with what the holocron gave, worded by the viewer.
fn find_message(find: FindEvent, own: &str) -> ChatMessage {
    ChatMessage {
        reactions: Default::default(),
        id: find.id,
        at: find.at,
        holocron: Some(DropMark {
            own: find.key_id == own,
            tier: find.tier,
            find: Some(FindMark {
                shader: find.prize == "shader",
                name: find.item_name,
                rarity: find.rarity,
                finish: find.finish,
                card: if find.prize == "card" {
                    find.item.clone()
                } else {
                    String::new()
                },
                skin: if find.prize == "shader" {
                    find.item
                } else {
                    String::new()
                },
            }),
        }),
        official: None,
        key_id: find.key_id,
        name: find.name,
        verified: find.verified,
        tick: crate::Tick::None,
        staff: false,
        creator: false,
        trust: None,
        emoji: String::new(),
        client: String::new(),
        text: String::new(),
        channel: Default::default(),
    }
}

/// The chat line of the SJK team's `official` news, `id` in the feed's sequence. Its
/// text is left empty: the viewer words the line.
fn official_message(id: u64, at: i64, official: Official) -> ChatMessage {
    ChatMessage {
        reactions: Default::default(),
        id,
        at,
        key_id: String::new(),
        name: "SJK".to_owned(),
        verified: false,
        tick: crate::Tick::None,
        staff: false,
        creator: false,
        trust: None,
        emoji: String::new(),
        client: String::new(),
        text: String::new(),
        holocron: None,
        official: Some(official),
        channel: Default::default(),
    }
}

/// Whether `state`'s messages hold announcement `id`.
fn holds_announcement(state: &ChatState, id: u64) -> bool {
    state.messages.iter().any(|message| {
        matches!(&message.official, Some(Official::Announcement(shown)) if shown.id == id)
    })
}

/// Put `announcements` (oldest first) at the end of the chat, read from the hub rather
/// than the feed (made while the player was away), each once: one the chat holds already
/// (the feed relayed it) is not added again. Only once the hub being read has answered
/// (`loaded`): before, its backlog is still to come. Each takes the id of the newest
/// message (0 in an empty chat), so the feed's next message still follows it; the viewer
/// tells these lines by their announcement. Returns the ids of the announcements given
/// that the chat now holds; empty when it could not take them (not loaded: the chat off,
/// or its first answer still to come).
pub(crate) fn post_announcements(
    state: &Mutex<ChatState>,
    announcements: &[Announcement],
) -> Vec<u64> {
    let mut state = lock(state);
    if state.loaded.is_none() {
        return Vec::new();
    }
    let mut held = Vec::with_capacity(announcements.len());
    let mut changed = false;
    for announcement in announcements {
        if !holds_announcement(&state, announcement.id) {
            let id = state.messages.back().map_or(0, |last| last.id);
            if state.messages.len() == MESSAGES_KEPT {
                state.messages.pop_front();
            }
            state.messages.push_back(official_message(
                id,
                announcement.at,
                Official::Announcement(announcement.clone()),
            ));
            changed = true;
        }
        held.push(announcement.id);
    }
    if changed {
        state.revision += 1;
    }
    held
}

/// Add `received` to `waiting`, keeping the newest `kept`.
fn queue<T>(waiting: &Mutex<VecDeque<T>>, received: impl IntoIterator<Item = T>, kept: usize) {
    let mut received = received.into_iter().peekable();
    if received.peek().is_none() {
        return;
    }
    let mut waiting = lock(waiting);
    for item in received {
        if waiting.len() == kept {
            waiting.pop_front();
        }
        waiting.push_back(item);
    }
}

/// Run the feed on this thread until `stop` is set.
pub(crate) fn run(mut worker: FeedWorker, stop: &std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering;
    while !stop.load(Ordering::Relaxed) {
        let wait = worker.step(Instant::now());
        // Short sleeps, so a change of hub or a stop is seen within a second.
        std::thread::sleep(wait.min(IDLE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::HubError;
    use crate::wire::Profile;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Something that happens while a poll is under way.
    type Meanwhile = Box<dyn FnOnce() + Send>;

    /// A hub whose feed answers come from a script.
    #[derive(Clone, Default)]
    struct Scripted {
        answers: Arc<Mutex<VecDeque<Result<Feed, HubError>>>>,
        asked: Arc<Mutex<Vec<String>>>,
        down: Arc<AtomicBool>,
        meanwhile: Arc<Mutex<Option<Meanwhile>>>,
    }

    impl Hub for Scripted {
        fn register(&mut self, _: &Identity, _: Option<&str>) -> Result<Profile, HubError> {
            unreachable!()
        }
        fn set_bio(&mut self, _: &Identity, _: &str) -> Result<Profile, HubError> {
            unreachable!()
        }
        fn profile(&mut self, _: &str) -> Result<Profile, HubError> {
            unreachable!()
        }
        fn claim(
            &mut self,
            _: &Identity,
            _: &str,
            _: u8,
            _: &str,
            _: bool,
        ) -> Result<(), HubError> {
            unreachable!()
        }
        fn release(&mut self, _: &Identity, _: &str) -> Result<(), HubError> {
            unreachable!()
        }
        fn presence(&mut self, _: &str) -> Result<Vec<crate::Presence>, HubError> {
            unreachable!()
        }
        fn feed(
            &mut self,
            _: &Identity,
            after: u64,
            server: Option<&str>,
            channels: &[crate::HubChannel],
            wait: u64,
        ) -> Result<Feed, HubError> {
            let asked = if channels.is_empty() {
                format!("after={after} server={server:?} wait={wait}")
            } else {
                format!("after={after} server={server:?} wait={wait} channels={channels:?}")
            };
            self.asked.lock().unwrap().push(asked);
            if let Some(meanwhile) = self.meanwhile.lock().unwrap().take() {
                meanwhile();
            }
            if self.down.load(Ordering::SeqCst) {
                return Err(HubError::Network("down".to_owned()));
            }
            self.answers
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(Feed::default()))
        }
    }

    fn message(id: u64, text: &str) -> ChatMessage {
        ChatMessage {
            reactions: Default::default(),
            id,
            at: 0,
            key_id: "aa".to_owned(),
            name: "Sol".to_owned(),
            verified: false,
            tick: crate::Tick::None,
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

    fn answer(next: u64, chat: Vec<ChatMessage>) -> Result<Feed, HubError> {
        Ok(Feed {
            next,
            chat,
            online: 3,
            ..Feed::default()
        })
    }

    struct Rig {
        hub: Scripted,
        shared: Arc<Mutex<FeedShared>>,
        state: Arc<Mutex<ChatState>>,
        emotes: Arc<Mutex<VecDeque<Emote>>>,
        looks: Arc<Mutex<VecDeque<QueuedLook>>>,
        worker: FeedWorker,
        made: Arc<Mutex<Vec<String>>>,
        /// How many times the worker was told of a drop for the player's own key.
        told: Arc<std::sync::atomic::AtomicUsize>,
    }

    fn rig(now: Instant) -> Rig {
        let hub = Scripted::default();
        let shared = Arc::new(Mutex::new(FeedShared::default()));
        let state = Arc::new(Mutex::new(ChatState::default()));
        let emotes = Arc::new(Mutex::new(VecDeque::new()));
        let looks = Arc::new(Mutex::new(VecDeque::new()));
        let made = Arc::new(Mutex::new(Vec::new()));
        let told = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (factory, log) = (hub.clone(), Arc::clone(&made));
        let counter = Arc::clone(&told);
        let make: HubFactory = Box::new(move |url| {
            log.lock().unwrap().push(url.to_owned());
            Ok(Box::new(factory.clone()))
        });
        let worker = FeedWorker::new(
            Identity::from_seed([1; 32]),
            make,
            Arc::clone(&shared),
            Arc::clone(&state),
            Arc::clone(&emotes),
            Arc::clone(&looks),
            Box::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
            now,
        );
        Rig {
            hub,
            shared,
            state,
            emotes,
            looks,
            worker,
            made,
            told,
        }
    }

    impl Rig {
        fn script(&self, answer: Result<Feed, HubError>) {
            self.hub.answers.lock().unwrap().push_back(answer);
        }
        fn asked(&self) -> Vec<String> {
            self.hub.asked.lock().unwrap().clone()
        }
        fn texts(&self) -> Vec<String> {
            lock(&self.state)
                .messages
                .iter()
                .map(|m| m.text.clone())
                .collect()
        }
        fn read(&self, url: &str, server: Option<&str>) {
            let mut shared = lock(&self.shared);
            *shared = FeedShared {
                url: Some(url.to_owned()),
                server: server.map(str::to_owned),
                chat: true,
                channels: Vec::new(),
                person: String::new(),
                generation: shared.generation + 1,
            };
        }
        /// The looks waiting for a viewer on `server`.
        fn take_looks(&self, server: &str) -> ReceivedLooks {
            take_looks(&self.shared, &self.looks, server.parse().ok())
        }
        fn show_chat(&self, on: bool) {
            lock(&self.shared).chat = on;
        }
    }

    fn drop_event(id: u64, key_id: &str, tier: &str) -> DropEvent {
        DropEvent {
            id,
            at: 0,
            key_id: key_id.to_owned(),
            name: "^2Sol".to_owned(),
            tier: tier.to_owned(),
            verified: true,
        }
    }

    #[test]
    fn reconnect_backlog_refreshes_reactions_on_a_known_message() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let mut line = message(1, "hello");
        line.reactions = vec![crate::reactions::Reaction {
            key_id: "0123456789abcdef".into(),
            emoji: "like".into(),
        }]
        .into();
        rig.script(answer(2, vec![line]));
        rig.worker.step(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        rig.script(answer(3, vec![message(1, "hello")]));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(rig.texts(), ["hello"]);
        assert!(lock(&rig.state).messages[0].reactions.is_empty());
    }

    #[test]
    fn reaction_updates_replace_state_without_new_messages_or_resurrection() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(1, vec![message(1, "hello")]));
        rig.worker.step(t0);
        let reactions: crate::reactions::State = vec![crate::reactions::Reaction {
            key_id: "0123456789abcdef".into(),
            emoji: "heart".into(),
        }]
        .into();
        rig.script(Ok(Feed {
            next: 2,
            online: 3,
            reaction_updates: vec![crate::reactions::Update {
                id: 2,
                message: 1,
                reactions: reactions.clone(),
            }],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(rig.texts(), ["hello"]);
        assert_eq!(lock(&rig.state).messages[0].reactions, reactions);
        rig.script(Ok(Feed {
            next: 3,
            online: 3,
            deleted: vec![1],
            reaction_updates: vec![crate::reactions::Update {
                id: 3,
                message: 1,
                reactions,
            }],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 2);
        assert!(rig.texts().is_empty());
    }

    #[test]
    fn drops_join_the_chat_in_the_feeds_order() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let own = Identity::from_seed([1; 32]).key_id();
        rig.script(Ok(Feed {
            next: 5,
            chat: vec![message(3, "gg"), message(5, "wp")],
            drops: vec![
                drop_event(4, "ff", "legendary"),
                drop_event(2, &own, "rare"),
            ],
            online: 1,
            ..Feed::default()
        }));
        rig.worker.step(t0);
        let state = lock(&rig.state).clone();
        let ids: Vec<u64> = state.messages.iter().map(|m| m.id).collect();
        assert_eq!(ids, [2, 3, 4, 5], "one sequence, drops among messages");
        let marks: Vec<Option<(&str, bool)>> = state
            .messages
            .iter()
            .map(|m| m.holocron.as_ref().map(|d| (d.tier.as_str(), d.own)))
            .collect();
        assert_eq!(
            marks,
            [Some(("rare", true)), None, Some(("legendary", false)), None]
        );
        let first = &state.messages[0];
        assert_eq!(
            (first.name.as_str(), first.text.as_str(), first.verified),
            ("^2Sol", "", true)
        );
        // A replay of what was shown is skipped like any message.
        rig.script(Ok(Feed {
            next: 5,
            drops: vec![drop_event(4, "ff", "legendary")],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(lock(&rig.state).messages.len(), 4);
    }

    fn announcement(id: u64, title: &str) -> Announcement {
        Announcement {
            id,
            at: 100 + id as i64,
            kind: "news".to_owned(),
            title: title.to_owned(),
            text: "Hello all".to_owned(),
            version: String::new(),
            images: std::collections::BTreeMap::new(),
        }
    }

    /// Announcements and rounds join the chat in the feed's order as the SJK team's
    /// lines, the worker is told of each once (a replay of the last minute is not news),
    /// and older hubs send neither.
    #[test]
    fn announcements_and_rounds_join_the_chat_and_tell_the_worker() {
        let parsed: Feed = serde_json::from_str(
            r#"{"next":9,"announcements":[{"id":8,"announcement":{"id":3,"at":5,
                "kind":"news","title":"Hi","text":"Line"}}],
                "rounds":[{"id":9,"round":2,"at":6,"title":"Contributor rewards #1","count":7}]}"#,
        )
        .unwrap();
        let old: Feed = serde_json::from_str(r#"{"next":7}"#).unwrap();
        assert!(old.announcements.is_empty() && old.rounds.is_empty());
        let t0 = Instant::now();
        let mut rig = rig(t0);
        let told = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&told);
        rig.worker
            .set_news(Box::new(move |news| sink.lock().unwrap().push(news)));
        rig.read("https://hub", None);
        rig.script(Ok(Feed {
            chat: vec![message(7, "gg")],
            ..parsed.clone()
        }));
        rig.worker.step(t0);
        let state = lock(&rig.state).clone();
        let ids: Vec<u64> = state.messages.iter().map(|m| m.id).collect();
        assert_eq!(ids, [7, 8, 9]);
        match &state.messages[1].official {
            Some(Official::Announcement(shown)) => assert_eq!(shown.title, "Hi"),
            other => panic!("an announcement line, not {other:?}"),
        }
        match &state.messages[2].official {
            Some(Official::Round(round)) => assert_eq!((round.round, round.count), (2, 7)),
            other => panic!("a round line, not {other:?}"),
        }
        assert!(state.messages[1].holocron.is_none());
        assert_eq!(
            *told.lock().unwrap(),
            [
                FeedNews::Announcements(vec![announcement_of(&parsed)]),
                FeedNews::Round
            ]
        );
        // The same again (a backlog read anew): the chat skips it, the worker hears nothing.
        rig.script(Ok(parsed));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(lock(&rig.state).messages.len(), 3);
        assert_eq!(told.lock().unwrap().len(), 2);
        // A deletion of the id an SJK line holds leaves the line.
        rig.script(Ok(Feed {
            next: 9,
            deleted: vec![8],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 2);
        assert_eq!(lock(&rig.state).messages.len(), 3);
    }

    fn announcement_of(feed: &Feed) -> Announcement {
        feed.announcements[0].announcement.clone()
    }

    /// Announcements read from the hub join the end of a loaded chat once each, after the
    /// newest message's id, so the feed's next message still follows them; before the
    /// first answer they wait.
    #[test]
    fn posted_announcements_join_a_loaded_chat_once() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let list = [announcement(1, "Old"), announcement(2, "New")];
        assert!(
            post_announcements(&rig.state, &list).is_empty(),
            "not loaded"
        );
        rig.script(answer(5, vec![message(4, "a"), message(5, "b")]));
        rig.worker.step(t0);
        assert_eq!(post_announcements(&rig.state, &list), [1, 2]);
        let state = lock(&rig.state).clone();
        let ids: Vec<u64> = state.messages.iter().map(|m| m.id).collect();
        assert_eq!(ids, [4, 5, 5, 5]);
        assert!(state.messages[2].official.is_some());
        // Again: nothing added, both still held.
        let revision = state.revision;
        assert_eq!(post_announcements(&rig.state, &list), [1, 2]);
        assert_eq!(lock(&rig.state).revision, revision);
        // The feed's next message follows them.
        rig.script(answer(6, vec![message(6, "c")]));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(rig.texts().last().map(String::as_str), Some("c"));
        assert_eq!(lock(&rig.state).messages.len(), 5);
    }

    /// A grand find joins the chat in the feed's order as a drop line marked with what
    /// the holocron gave; the wire's field is parsed, and absent from older hubs.
    #[test]
    fn grand_finds_join_the_chat_marked_with_their_prize() {
        let parsed: Feed = serde_json::from_str(
            r#"{"next":7,"finds":[{"id":7,"at":9,"key_id":"ff","name":"Fox","verified":true,
                "tier":"mythical","prize":"shader","item":"saber_phoenix",
                "item_name":"Phoenix blade","rarity":"mythical","finish":""}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.finds.len(), 1);
        let old: Feed = serde_json::from_str(r#"{"next":7}"#).unwrap();
        assert!(old.finds.is_empty(), "an older hub sends no finds");
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(Ok(Feed {
            chat: vec![message(6, "gg")],
            ..parsed
        }));
        rig.worker.step(t0);
        let state = lock(&rig.state).clone();
        let ids: Vec<u64> = state.messages.iter().map(|m| m.id).collect();
        assert_eq!(ids, [6, 7]);
        let mark = state.messages[1].holocron.as_ref().expect("a find line");
        assert_eq!((mark.tier.as_str(), mark.own), ("mythical", false));
        let find = mark.find.as_ref().expect("its prize");
        assert!(find.shader);
        assert_eq!(
            (find.skin.as_str(), find.card.as_str()),
            ("saber_phoenix", "")
        );
        assert_eq!(
            (find.name.as_str(), find.rarity.as_str()),
            ("Phoenix blade", "mythical")
        );
        assert_eq!(state.messages[1].name, "Fox");
    }

    #[test]
    fn a_drop_for_the_own_key_tells_the_worker_once_even_with_the_chat_off() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.show_chat(false);
        let own = Identity::from_seed([1; 32]).key_id();
        rig.script(Ok(Feed {
            next: 8,
            drops: vec![
                drop_event(7, "ff", "mythical"),
                drop_event(8, &own, "uncommon"),
            ],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        assert_eq!(rig.told.load(Ordering::SeqCst), 1, "the own key's only");
        assert!(lock(&rig.state).messages.is_empty(), "the chat is off");
        // The same drop again (a reader that started over) is not news; a newer one is.
        rig.script(Ok(Feed {
            next: 8,
            drops: vec![drop_event(8, &own, "uncommon")],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(rig.told.load(Ordering::SeqCst), 1);
        rig.script(Ok(Feed {
            next: 9,
            drops: vec![drop_event(9, &own, "rare")],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 2);
        assert_eq!(rig.told.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn nothing_is_asked_while_there_is_no_hub_to_read() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        assert_eq!(rig.worker.step(t0), IDLE);
        assert!(rig.asked().is_empty());
        assert!(rig.made.lock().unwrap().is_empty());
        assert!(!lock(&rig.state).live);
    }

    #[test]
    fn the_people_listed_stay_until_the_hub_lists_others_or_the_chat_goes_off() {
        use crate::wire::Person;
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let people = |names: &[&str]| People {
            online: names
                .iter()
                .map(|name| Person {
                    key_id: format!("{name:0>16}"),
                    name: (*name).to_owned(),
                    ..Person::default()
                })
                .collect(),
            recent: Vec::new(),
        };
        let with = |next, listed: Option<People>| {
            Ok(Feed {
                next,
                online: 3,
                people: listed,
                ..Feed::default()
            })
        };
        rig.script(answer(1, vec![message(1, "a")]));
        rig.worker.step(t0);
        assert_eq!(lock(&rig.state).people, None, "a hub that lists nobody");
        rig.script(with(1, Some(people(&["Sol", "Fox"]))));
        rig.worker.step(t0 + POLL_GAP);
        let state = lock(&rig.state).clone();
        assert_eq!(state.people, Some(people(&["Sol", "Fox"])));
        // An answer without the list keeps it; the same list changes nothing.
        rig.script(with(1, None));
        rig.script(with(1, Some(people(&["Sol", "Fox"]))));
        rig.worker.step(t0 + POLL_GAP * 2);
        rig.worker.step(t0 + POLL_GAP * 3);
        assert_eq!(*lock(&rig.state), state);
        rig.script(with(1, Some(people(&["Fox"]))));
        rig.worker.step(t0 + POLL_GAP * 4);
        let state = lock(&rig.state).clone();
        assert_eq!(state.people, Some(people(&["Fox"])));
        // The chat off: nobody shows.
        rig.show_chat(false);
        rig.worker.step(t0 + POLL_GAP * 5);
        assert_eq!(lock(&rig.state).people, None);
    }

    #[test]
    fn the_feed_appends_in_order_and_skips_repeats() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        rig.script(answer(2, vec![message(1, "a"), message(2, "b")]));
        rig.script(answer(3, vec![message(2, "b"), message(3, "c")]));
        rig.worker.step(t0);
        let revision = lock(&rig.state).revision;
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.texts(), ["a", "b", "c"]);
        assert_eq!(
            rig.asked(),
            [
                "after=0 server=Some(\"1.2.3.4:29070\") wait=25",
                "after=2 server=Some(\"1.2.3.4:29070\") wait=25"
            ]
        );
        let state = lock(&rig.state).clone();
        assert!(state.live);
        assert_eq!(state.online, 3);
        assert!(state.revision > revision);
        // An answer with nothing new changes nothing.
        rig.script(answer(3, Vec::new()));
        rig.worker.step(t0 + Duration::from_secs(4));
        assert_eq!(lock(&rig.state).revision, state.revision);
    }

    #[test]
    fn clan_chats_joined_are_asked_for_and_kept_apart() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let clan = |id, text, channel| ChatMessage {
            channel,
            ..message(id, text)
        };
        let listed = vec![crate::ListedChannel {
            id: crate::HubChannel::Jof,
            home: true,
        }];
        rig.script(Ok(Feed {
            channels: listed.clone(),
            ..answer(1, vec![message(1, "sjk")]).unwrap()
        }));
        rig.worker.step(t0);
        assert_eq!(lock(&rig.state).channels, listed, "what the hub lists");
        // Joining asks again from the start, which brings the clan's backlog.
        lock(&rig.shared).channels = vec![crate::HubChannel::Jof];
        rig.script(answer(
            3,
            vec![
                message(1, "sjk"),
                clan(2, "clan", crate::HubChannel::Jof),
                // Not joined, or unknown here: left out.
                clan(3, "other", crate::HubChannel::Sith),
            ],
        ));
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.asked()[1], "after=0 server=None wait=25 channels=[Jof]");
        let state = lock(&rig.state).clone();
        assert_eq!(rig.texts(), ["sjk"], "the SJK chat's messages alone");
        let clan_texts: Vec<&str> = state.clan.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(clan_texts, ["clan"]);
        // Deleted by staff: gone from the clan's too.
        rig.script(Ok(Feed {
            deleted: vec![2],
            ..answer(4, Vec::new()).unwrap()
        }));
        rig.worker.step(t0 + Duration::from_secs(4));
        assert!(lock(&rig.state).clan.is_empty());
        // Leaving forgets its lines and stops asking.
        rig.script(answer(4, vec![clan(5, "late", crate::HubChannel::Jof)]));
        let epoch = lock(&rig.state).loaded;
        lock(&rig.shared).channels.clear();
        rig.worker.step(t0 + Duration::from_secs(6));
        assert_eq!(rig.asked()[3], "after=0 server=None wait=25");
        let state = lock(&rig.state).clone();
        assert!(state.clan.is_empty());
        assert_ne!(state.loaded, epoch, "a new reading");
    }

    #[test]
    fn the_last_200_messages_are_kept() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        let batch: Vec<ChatMessage> = (1..=250).map(|id| message(id, &id.to_string())).collect();
        rig.script(answer(250, batch));
        rig.worker.step(t0);
        let texts = rig.texts();
        assert_eq!(texts.len(), MESSAGES_KEPT);
        assert_eq!(texts[0], "51");
    }

    #[test]
    fn deleted_ids_leave_the_state() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(2, vec![message(1, "spam"), message(2, "hi")]));
        rig.script(Ok(Feed {
            next: 3,
            deleted: vec![1],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.texts(), ["hi"]);
    }

    #[test]
    fn pings_wait_for_the_viewer_of_their_server() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub.example", Some("1.2.3.4:29070"));
        let mark = |id| Ping {
            id,
            at: 0,
            slot: 3,
            claimed_name: "^2Sol".to_owned(),
            key_id: "aa".to_owned(),
            pos: [1, 2, 3],
            scope: "all".to_owned(),
        };
        rig.script(Ok(Feed {
            next: 2,
            pings: vec![mark(1), mark(2)],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        let queue = rig.worker.pings();
        let here = Some("1.2.3.4:29070".parse().unwrap());
        // Another server takes none and drops them: they were not read for it.
        assert!(take_pings(&queue, Some("5.6.7.8:29070".parse().unwrap())).is_empty());
        assert!(take_pings(&queue, here).is_empty());
        rig.script(Ok(Feed {
            next: 3,
            pings: vec![mark(3)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        let taken = take_pings(&queue, here);
        assert_eq!(taken.iter().map(|ping| ping.id).collect::<Vec<_>>(), [3]);
        assert!(take_pings(&queue, here).is_empty(), "taken once");
        // Out of a game, the queue empties.
        rig.script(Ok(Feed {
            next: 4,
            pings: vec![mark(4)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 2);
        assert!(take_pings(&queue, None).is_empty());
        assert!(take_pings(&queue, here).is_empty());
    }

    #[test]
    fn toy_events_wait_for_the_viewer_of_their_server() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub.example", Some("1.2.3.4:29070"));
        let spotted = |id| ToyEvent {
            id,
            at: 0,
            slot: 3,
            claimed_name: "^2Sol".to_owned(),
            key_id: "aa".to_owned(),
            toy: "cardboard_box".to_owned(),
            event: "spotted".to_owned(),
            target: 5,
        };
        rig.script(Ok(Feed {
            next: 2,
            toys: vec![spotted(1), spotted(2)],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        let queue = rig.worker.toys();
        let here = Some("1.2.3.4:29070".parse().unwrap());
        assert!(take_toy_events(&queue, Some("5.6.7.8:29070".parse().unwrap())).is_empty());
        rig.script(Ok(Feed {
            next: 3,
            toys: vec![spotted(3)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        let taken = take_toy_events(&queue, here);
        assert_eq!(taken.iter().map(|event| event.id).collect::<Vec<_>>(), [3]);
        assert!(take_toy_events(&queue, here).is_empty(), "taken once");
    }

    #[test]
    fn emotes_wait_for_the_viewer() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        let wave = Emote {
            id: 4,
            at: 0,
            slot: 3,
            claimed_name: "Sol".to_owned(),
            key_id: "aa".to_owned(),
            emote: "wave".to_owned(),
            sync: None,
            quiet: false,
        };
        rig.script(Ok(Feed {
            next: 4,
            emotes: vec![wave.clone()],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        assert_eq!(
            lock(&rig.emotes).iter().cloned().collect::<Vec<_>>(),
            [wave]
        );
    }

    fn look_event(id: u64, illuminate: bool) -> LookEvent {
        LookEvent {
            id,
            at: 0,
            slot: 3,
            claimed_name: "Sol".to_owned(),
            key_id: "aa".to_owned(),
            saber: "saber_sun".to_owned(),
            illuminate,
            saber_off: Vec::new(),
            toys: Vec::new(),
            saber2: None,
            saber2_off: Vec::new(),
            saber_on: Vec::new(),
            saber2_on: Vec::new(),
            saber_sounds: String::new(),
            tag: None,
            finisher: String::new(),
        }
    }

    #[test]
    fn looks_wait_for_the_viewer_and_the_newest_are_kept() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        let batch: Vec<LookEvent> = (1..=70).map(|id| look_event(id, id % 2 == 0)).collect();
        rig.script(Ok(Feed {
            next: 70,
            looks: batch,
            ..Feed::default()
        }));
        rig.worker.step(t0);
        let waiting = rig.take_looks("1.2.3.4:29070").events;
        assert_eq!(waiting.len(), LOOKS_WAITING);
        assert_eq!(waiting.first().map(|look| look.id), Some(7));
        assert_eq!(waiting.last(), Some(&look_event(70, true)));
        assert!(lock(&rig.looks).is_empty(), "taken");
    }

    #[test]
    fn another_server_is_read_from_the_start_for_its_looks() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        rig.script(answer(9, vec![message(9, "hi")]));
        rig.worker.step(t0);
        rig.read("https://hub", Some("5.6.7.8:29070"));
        rig.script(Ok(Feed {
            next: 9,
            chat: vec![message(9, "hi")],
            looks: vec![look_event(8, true)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(
            rig.asked()[1],
            "after=0 server=Some(\"5.6.7.8:29070\") wait=25",
            "the hub's answer to 0 carries the server's looks of the last minute"
        );
        assert_eq!(rig.texts(), ["hi"], "the same hub: the chat goes on");
        assert_eq!(
            rig.take_looks("5.6.7.8:29070").events,
            [look_event(8, true)]
        );
        rig.worker.step(t0 + POLL_GAP * 2);
        assert_eq!(
            rig.asked()[2],
            "after=9 server=Some(\"5.6.7.8:29070\") wait=25"
        );
    }

    #[test]
    fn looks_of_a_poll_under_way_when_the_reading_changed_are_dropped() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        let old = "1.2.3.4:29070";
        rig.read("https://hub", Some(old));
        // The player moves to another server while the poll waits at the hub.
        let shared = Arc::clone(&rig.shared);
        *rig.hub.meanwhile.lock().unwrap() = Some(Box::new(move || {
            let mut shared = lock(&shared);
            shared.server = Some("5.6.7.8:29070".to_owned());
            shared.generation += 1;
        }));
        rig.script(Ok(Feed {
            next: 5,
            looks: vec![look_event(5, true)],
            ..Feed::default()
        }));
        rig.worker.step(t0);
        let taken = rig.take_looks("5.6.7.8:29070");
        assert_eq!(taken.generation, lock(&rig.shared).generation);
        assert!(taken.events.is_empty(), "read for the old server");
        // The viewer is still on the old server, but the reading changed: dropped too.
        rig.read("https://hub", Some(old));
        rig.script(Ok(Feed {
            next: 6,
            looks: vec![look_event(6, true)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP);
        let shared = Arc::clone(&rig.shared);
        *rig.hub.meanwhile.lock().unwrap() = Some(Box::new(move || {
            // The identity is turned off.
            let mut shared = lock(&shared);
            *shared = FeedShared {
                generation: shared.generation + 1,
                ..FeedShared::default()
            };
        }));
        rig.script(Ok(Feed {
            next: 7,
            looks: vec![look_event(7, false)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 2);
        let taken = rig.take_looks(old);
        assert!(taken.events.is_empty(), "{:?}", taken.events);
        // Without a change, the same server's looks are handed over.
        rig.read("https://hub", Some(old));
        rig.script(Ok(Feed {
            next: 8,
            looks: vec![look_event(8, true)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 3);
        assert!(rig.take_looks("5.6.7.8:29070").events.is_empty());
        rig.script(Ok(Feed {
            next: 9,
            looks: vec![look_event(9, true)],
            ..Feed::default()
        }));
        rig.worker.step(t0 + POLL_GAP * 4);
        assert_eq!(rig.take_looks(old).events, [look_event(9, true)]);
    }

    #[test]
    fn with_the_chat_off_the_feed_keeps_looks_and_emotes_but_no_message() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", Some("1.2.3.4:29070"));
        rig.show_chat(false);
        rig.script(Ok(Feed {
            next: 5,
            chat: vec![message(4, "hidden")],
            looks: vec![look_event(5, true)],
            online: 9,
            ..Feed::default()
        }));
        rig.worker.step(t0);
        assert_eq!(
            rig.asked(),
            ["after=0 server=Some(\"1.2.3.4:29070\") wait=25"]
        );
        assert_eq!(lock(&rig.looks).len(), 1);
        // The chat is as an idle feed leaves it: nothing the page or the game shows.
        let idle = |state: &ChatState| ChatState {
            revision: state.revision,
            ..ChatState::default()
        };
        let state = lock(&rig.state).clone();
        assert_eq!(state, idle(&state));
        // A failure says nothing either.
        rig.hub.down.store(true, Ordering::SeqCst);
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(*lock(&rig.state), state);
        rig.hub.down.store(false, Ordering::SeqCst);
        // On again: the backlog is read from the start, under a new epoch.
        rig.show_chat(true);
        rig.script(answer(6, vec![message(4, "hidden"), message(6, "hello")]));
        rig.worker.step(t0 + POLL_GAP * 2);
        assert_eq!(
            rig.asked()[2],
            "after=0 server=Some(\"1.2.3.4:29070\") wait=25"
        );
        assert_eq!(rig.texts(), ["hidden", "hello"]);
        let first = lock(&rig.state).loaded.expect("loaded");
        // Off once more: the messages go, and the reading goes on from where it was.
        rig.show_chat(false);
        rig.worker.step(t0 + POLL_GAP * 3);
        assert!(rig.texts().is_empty());
        assert_eq!(lock(&rig.state).loaded, None);
        assert_eq!(
            rig.asked()[3],
            "after=6 server=Some(\"1.2.3.4:29070\") wait=25"
        );
        rig.show_chat(true);
        rig.worker.step(t0 + POLL_GAP * 4);
        assert_ne!(lock(&rig.state).loaded, Some(first));
    }

    #[test]
    fn a_new_hub_starts_from_the_backlog() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://one", None);
        rig.script(answer(9, vec![message(9, "old hub")]));
        rig.worker.step(t0);
        rig.read("https://two", None);
        rig.script(answer(1, vec![message(1, "new hub")]));
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(*rig.made.lock().unwrap(), ["https://one", "https://two"]);
        assert_eq!(rig.asked()[1], "after=0 server=None wait=25");
        assert_eq!(rig.texts(), ["new hub"]);
    }

    #[test]
    fn the_backlog_is_marked_loaded_once_the_hub_answered() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        assert_eq!(lock(&rig.state).loaded, None);
        rig.read("https://one", None);
        rig.hub.down.store(true, Ordering::SeqCst);
        rig.worker.step(t0);
        assert_eq!(lock(&rig.state).loaded, None, "no answer yet");
        rig.hub.down.store(false, Ordering::SeqCst);
        rig.script(answer(2, vec![message(1, "a"), message(2, "b")]));
        rig.worker.step(t0 + RETRY_MIN);
        let first = lock(&rig.state).loaded.expect("loaded");
        // Another hub: not loaded until it answers, then under another epoch.
        rig.read("https://two", None);
        rig.hub.down.store(true, Ordering::SeqCst);
        rig.worker.step(t0 + RETRY_MIN * 2);
        assert_eq!(lock(&rig.state).loaded, None);
        rig.hub.down.store(false, Ordering::SeqCst);
        rig.worker.step(t0 + RETRY_MIN * 4);
        let second = lock(&rig.state).loaded.expect("loaded again");
        assert_ne!(first, second);
    }

    #[test]
    fn a_hub_that_restarted_starts_from_its_backlog() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(90, vec![message(90, "before")]));
        // The hub restarted: its ids begin again, below what was asked.
        rig.script(answer(
            2,
            vec![message(1, "after a"), message(2, "after b")],
        ));
        rig.worker.step(t0);
        rig.worker.step(t0 + Duration::from_secs(2));
        assert_eq!(rig.texts(), ["after a", "after b"]);
        rig.worker.step(t0 + Duration::from_secs(4));
        assert_eq!(rig.asked()[2], "after=2 server=None wait=25");
    }

    #[test]
    fn feed_stops_when_chat_turns_off() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.script(answer(1, vec![message(1, "a")]));
        rig.worker.step(t0);
        *lock(&rig.shared) = FeedShared::default();
        assert_eq!(rig.worker.step(t0 + Duration::from_secs(2)), IDLE);
        assert_eq!(rig.asked().len(), 1);
        let state = lock(&rig.state).clone();
        assert!(!state.live);
        assert!(
            state.messages.is_empty(),
            "the chat is off: nothing to show"
        );
    }

    #[test]
    fn polls_are_half_a_second_apart() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        assert_eq!(rig.worker.step(t0), POLL_GAP);
        assert_eq!(
            rig.worker.step(t0 + Duration::from_millis(200)),
            Duration::from_millis(300)
        );
        assert_eq!(rig.asked().len(), 1);
        rig.worker.step(t0 + POLL_GAP);
        assert_eq!(rig.asked().len(), 2);
    }

    /// A hub from before 27/10/2026 takes a poll every 2 seconds: its `feed_quota`
    /// slows the feed to that, without counting as a failure, until another hub is read.
    #[test]
    fn an_older_hubs_feed_quota_slows_polls_to_two_seconds() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.worker.step(t0);
        assert!(lock(&rig.state).live);
        rig.script(Err(HubError::Rejected {
            status: 429,
            code: "feed_quota".to_owned(),
            message: "slow down".to_owned(),
        }));
        assert_eq!(rig.worker.step(t0 + POLL_GAP), OLD_POLL_GAP);
        assert!(lock(&rig.state).live);
        let later = t0 + POLL_GAP + OLD_POLL_GAP;
        assert_eq!(rig.worker.step(later), OLD_POLL_GAP);
        assert_eq!(rig.asked().len(), 3);
        rig.read("https://other", None);
        assert_eq!(rig.worker.step(later + OLD_POLL_GAP), POLL_GAP);
    }

    #[test]
    fn failures_back_off() {
        let t0 = Instant::now();
        let mut rig = rig(t0);
        rig.read("https://hub", None);
        rig.hub.down.store(true, Ordering::SeqCst);
        assert_eq!(rig.worker.step(t0), RETRY_MIN);
        assert!(!lock(&rig.state).live);
        assert_eq!(rig.worker.step(t0 + RETRY_MIN), RETRY_MIN * 2);
        rig.hub.down.store(false, Ordering::SeqCst);
        rig.worker.step(t0 + RETRY_MIN * 3);
        assert!(lock(&rig.state).live);
        assert_eq!(rig.asked().len(), 3);
    }
}
