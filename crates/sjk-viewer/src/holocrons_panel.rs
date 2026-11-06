//! The Holocrons page (`docs/holocrons.md`, "The Holocrons tab"): the player's holocrons
//! on the SJK UI's Profile screen as its Holocrons tab ([`crate::profile_hub`]), and, with
//! the classic menus, opened by the Profile page's See holocrons or the `holocrons`
//! command. Like the Unlockables page it lives in the console and has the SJK UI's look
//! in every menu style ([`view`]).
//!
//! Left, the page leaves the world clear for one 3D holocron
//! ([`crate::holocrons::stage`]) wearing the look of the tier chosen; right, the four
//! tiers in a list (picture, name in the tier's colour, how many are held, the odds of a
//! drop), what the chosen tier is, how long to the next holocron, and the newest ten. A
//! tier held none of shows dimmed, never hidden. Under the holocron, Open opens the
//! oldest of the tier chosen and Open all up to twenty, in a ceremony over the page
//! ([`opening`]): each brings a sealed card and credits (`docs/cards.md`). Loot table (or
//! L) shows the odds of everything a holocron brings over the page ([`loot`]); Shop (or S)
//! shows the Collection's Forge on its Shop (since 21/10/2026: the shop was an overlay of
//! this page before). The player's credits and kyber shards stand at the panel's top right.
//!
//! Up and Down (or Left and Right, or the mouse wheel) choose a tier, Tab and Shift+Tab
//! walk them round, 1 to 4 and Home and End jump, Enter opens one, A opens all, L shows
//! the loot table, Escape goes back; a click on a tier chooses it. Open all asks first
//! when more than one would open ([`ask_all`]). The hub's counts and list are read twice a
//! second ([`Data::read`]), never per frame; while the ceremony waits for the hub its
//! answer is looked for every frame.

use crate::holocrons::{self, COUNT, SLOTS, Tier};
use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::{Duration, Instant};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "holocrons_panel_ask_all.rs"]
mod ask_all;
#[path = "holocrons_panel_batch.rs"]
mod batch;
#[path = "holocrons_panel_loot.rs"]
mod loot;
#[path = "holocrons_panel_opening.rs"]
pub(crate) mod opening;
#[path = "holocrons_panel_crate.rs"]
mod strip;
#[path = "holocrons_panel_unseal.rs"]
mod unseal;
#[path = "holocrons_panel_view.rs"]
mod view;

/// The tiers' rows, one token each: clear of the Unlockables page's cards (to 1260), the
/// Profile page's (to 1030) and the Profile screen's row of tabs (from 2000).
pub(crate) const ROW_BASE: u16 = 1_300;
/// Where the holocron floats on the 16:9 frame: the middle of the clear left of the page,
/// in frame pixels (not moved with the page under the Profile screen's tabs).
pub(crate) const STAGE_AT: [f32; 2] = [540.0, 480.0];
/// How many recent holocrons the page lists (eight since the kyber crystals took a row,
/// 21/10/2026).
pub(crate) const RECENT_SHOWN: usize = 8;
/// How often the hub's counts and list are read again while the page is open.
const REFRESH: Duration = Duration::from_millis(500);
/// The buttons under the holocron: Open (one of the tier chosen) and Open all.
pub(crate) const OPEN_TOKEN: u16 = 1_320;
pub(crate) const OPEN_ALL_TOKEN: u16 = 1_321;
/// Loot table, beside what the chosen tier holds, and the loot table's Back.
pub(crate) const LOOT_TOKEN: u16 = 1_323;
pub(crate) const LOOT_BACK_TOKEN: u16 = 1_324;
/// Shop, beside Open all: the Collection's Forge on its Shop.
pub(crate) const SHOP_TOKEN: u16 = 1_325;

/// What the page knows of the player's holocrons: whether the hub could say, and what.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum State {
    /// The identity is off: holocrons need it.
    IdentityOff,
    /// No hub is set (`cl_hubUrl`).
    NoHub,
    /// The hub has not answered yet, or cannot be reached.
    Waiting,
    /// The own profile's counts and list are known.
    Known,
}

/// One of the recent holocrons as the page lists it, its words made once per read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Recent {
    /// Position in [`holocrons::TIERS`].
    pub(crate) tier: usize,
    /// `dd/mm/yyyy HH:MM` (UTC), empty when the hub did not say.
    pub(crate) when: String,
    /// Staff gave it rather than play.
    pub(crate) gift: bool,
    /// The team's note, plain text; empty without one.
    pub(crate) note: String,
    /// It was opened.
    pub(crate) opened: bool,
    /// How it came in a trade ([`holocrons::Entry::trade_line`]), colour codes kept;
    /// `None` when it did not.
    pub(crate) trade: Option<String>,
    /// Its edition's word ("Alpha", [`crate::editions::word`]).
    pub(crate) edition: String,
}

/// What the page shows of the identity and the hub, read by [`Data::read`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Data {
    pub(crate) state: State,
    /// How many of each tier the player holds, once known.
    pub(crate) counts: Option<[u32; COUNT]>,
    /// The newest holocrons first, at most [`RECENT_SHOWN`].
    pub(crate) recent: Vec<Recent>,
    /// "Next holocron in about 20 minutes of play", empty when the hub did not say.
    pub(crate) next: String,
    /// The catch-up and the pace of earning ([`holocrons::pace_text`]), empty for none.
    pub(crate) pace: String,
    /// Holocrons found by play today (the hub's day, from 09:00 in Paris) and the most a
    /// day brings.
    pub(crate) today: Option<(u32, u32)>,
    /// How far along the next one is, 0 to 1.
    pub(crate) fraction: Option<f32>,
    /// The ids of the holocrons of each tier the profile lists that can still be
    /// opened, the oldest first.
    pub(crate) sealed: [Vec<u64>; COUNT],
    /// How many kyber crystals of each tier the player holds unopened, once known
    /// ([`crate::kyber`]): counted from the profile's list, which lists sealed rows first.
    pub(crate) crystals: Option<[u32; crate::kyber::COUNT]>,
    /// The ids of the kyber crystals of each tier that can still be opened, the oldest
    /// first.
    pub(crate) crystal_sealed: [Vec<u64>; crate::kyber::COUNT],
    /// The edition of every holocron and crystal that can still be opened, by id
    /// ([`crate::editions`], 06/11/2026): what opening it may bring.
    pub(crate) editions: Vec<(u64, String)>,
    /// The person's pity counter, once the hub said.
    pub(crate) pity: Option<crate::kyber::Pity>,
    /// The player's credits, once known.
    pub(crate) credits: Option<u64>,
    /// The player's kyber shards, once known.
    pub(crate) shards: Option<u64>,
    /// Seconds of play a holocron takes, once the hub said.
    pub(crate) every: Option<u64>,
    /// Whether the hub lets the player open holocrons now: only staff while opening is
    /// tested (`docs/holocrons.md`, "Opening"). False until the hub said. The Shop button
    /// follows it.
    pub(crate) can_open: bool,
    /// The SJK release the hub wants for opening and trading when this build is older
    /// (the hub's minimum client): `can_open` is false then and the page says to update.
    pub(crate) update: Option<String>,
}

impl Data {
    /// Nothing known: the page of `state`.
    pub(crate) fn nothing(state: State) -> Self {
        Self {
            state,
            counts: None,
            recent: Vec::new(),
            next: String::new(),
            pace: String::new(),
            today: None,
            fraction: None,
            sealed: Default::default(),
            crystals: None,
            crystal_sealed: Default::default(),
            editions: Vec::new(),
            pity: None,
            credits: None,
            shards: None,
            every: None,
            can_open: false,
            update: None,
        }
    }

    /// From `cl_identity` (`enabled`) and the identity service's snapshot, by the rule the
    /// Collection's holdings follow.
    pub(crate) fn of(enabled: bool, snapshot: &sjk_identity::Snapshot) -> Self {
        use sjk_identity::Status;
        let state = match (enabled, &snapshot.status, &snapshot.me) {
            (false, _, _) | (_, Status::Disabled, _) => State::IdentityOff,
            (_, Status::NoHub, _) => State::NoHub,
            (_, _, Some(_)) => State::Known,
            (_, _, None) => State::Waiting,
        };
        let (Some(me), State::Known) = (&snapshot.me, state) else {
            return Self::nothing(state);
        };
        let known: Vec<(holocrons::Entry, bool)> = me
            .holocrons
            .iter()
            .filter_map(|wire| {
                holocrons::Entry::from_wire(wire).map(|entry| (entry, !wire.is_sealed()))
            })
            .collect();
        let mut sealed: [Vec<u64>; COUNT] = Default::default();
        let mut crystal_sealed: [Vec<u64>; crate::kyber::COUNT] = Default::default();
        // The list is the newest first: the oldest go first when opening.
        for (entry, opened) in known.iter().rev() {
            if *opened {
                continue;
            }
            if entry.tier.is_kyber() {
                crystal_sealed[entry.tier.index].push(entry.id);
            } else {
                sealed[entry.tier.index].push(entry.id);
            }
        }
        let crystals = std::array::from_fn(|index| crystal_sealed[index].len() as u32);
        let editions = known
            .iter()
            .filter(|(_, opened)| !opened)
            .map(|(entry, _)| (entry.id, entry.edition.clone()))
            .collect();
        // The recent list is the holocrons found: crystals are counted on their own.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() as i64);
        let lock = snapshot
            .trades
            .as_ref()
            .map(|state| state.list.rules.resale_lock)
            // The hub's lock; none said yet, no countdown (the hub owns the number).
            .filter(|lock| *lock > 0);
        let recent = known
            .into_iter()
            .filter(|(entry, _)| !entry.tier.is_kyber())
            .take(RECENT_SHOWN)
            .map(|(entry, opened)| Recent {
                tier: entry.tier.index,
                when: holocrons::when_text(entry.dropped),
                gift: entry.gift,
                trade: entry.trade_line(now, lock),
                note: entry.note,
                opened,
                edition: crate::editions::word(&entry.edition),
            })
            .collect();
        let progress = snapshot.holocrons;
        Self {
            state,
            counts: Some(holocrons::counts_of(&me.holocron_counts)),
            recent,
            next: progress
                .as_ref()
                .map(|progress| {
                    let lifts_in = holocrons::cap_lifts_in(progress, Instant::now());
                    holocrons::next_text(progress, lifts_in)
                })
                .unwrap_or_default(),
            pace: progress
                .as_ref()
                .map(holocrons::pace_text)
                .unwrap_or_default(),
            today: progress.map(|progress| (progress.today, progress.daily_cap)),
            fraction: progress
                .filter(|progress| progress.next_every() > 0)
                .map(|progress| {
                    (progress.progress_secs as f32 / progress.next_every() as f32).clamp(0.0, 1.0)
                }),
            sealed,
            crystals: Some(crystals),
            crystal_sealed,
            editions,
            pity: crate::kyber::Pity::of(snapshot.kyber.as_ref()),
            credits: Some(me.credits),
            shards: Some(me.shards),
            every: progress
                .map(|progress| progress.every_secs)
                .filter(|&every| every > 0),
            can_open: crate::economy::may_act(snapshot),
            update: crate::economy::update_to(snapshot).map(str::to_owned),
        }
    }

    /// The live identity's: `enabled` is `cl_identity`.
    pub(crate) fn read(enabled: bool) -> Self {
        crate::player_identity::with_snapshot(|snapshot| Self::of(enabled, snapshot))
            .unwrap_or_else(|| Self::nothing(State::IdentityOff))
    }

    /// How many holocrons the player holds in all, once known.
    pub(crate) fn total(&self) -> Option<u32> {
        self.counts.map(|counts| counts.iter().sum())
    }

    /// How many of the tier at `slot` they hold, once known: a holocron tier's, or past
    /// them a kyber crystal tier's ([`holocrons::slot`]).
    pub(crate) fn count(&self, slot: usize) -> Option<u32> {
        match slot.checked_sub(COUNT) {
            None => self.counts.and_then(|counts| counts.get(slot).copied()),
            Some(crystal) => self
                .crystals
                .and_then(|counts| counts.get(crystal).copied()),
        }
    }

    /// What editions the tier at `slot`'s unopened ones are of, for its hover card:
    /// "Alpha Edition" when they are all of one, else how many of each ("2 Alpha, 1
    /// Beta", the oldest edition first); `None` when none is held.
    pub(crate) fn editions_line(&self, slot: usize) -> Option<String> {
        let sealed = self.sealed_of(slot);
        let mut counts: Vec<(&str, u32)> = Vec::new();
        for (_, edition) in self.editions.iter().filter(|(id, _)| sealed.contains(id)) {
            match counts
                .iter_mut()
                .find(|(seen, _)| *seen == edition.as_str())
            {
                Some((_, count)) => *count += 1,
                None => counts.push((edition, 1)),
            }
        }
        let rank = |id: &str| {
            crate::editions::ALL
                .iter()
                .position(|edition| edition.id == id)
                .unwrap_or(usize::MAX)
        };
        counts.sort_by_key(|(id, _)| rank(id));
        match counts.as_slice() {
            [] => None,
            [(only, _)] => Some(crate::editions::name(only)),
            _ => Some(
                counts
                    .iter()
                    .map(|(id, count)| format!("{count} {}", crate::editions::word(id)))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        }
    }

    /// The ids of the tier at `slot` that can still be opened, the oldest first.
    pub(crate) fn sealed_of(&self, slot: usize) -> &[u64] {
        match slot.checked_sub(COUNT) {
            None => self.sealed.get(slot),
            Some(crystal) => self.crystal_sealed.get(crystal),
        }
        .map_or(&[], Vec::as_slice)
    }

    /// The sealed lists of the kind `slot` is of: the holocrons', or the kyber crystals'.
    fn kind_of(&self, slot: usize) -> &[Vec<u64>] {
        if slot < COUNT {
            &self.sealed
        } else {
            &self.crystal_sealed
        }
    }

    /// How many of the kind of `slot` (holocrons, or kyber crystals) the page can open
    /// now (those the profile lists, none while the hub keeps opening to staff).
    pub(crate) fn openable(&self, slot: usize) -> usize {
        if !self.can_open {
            return 0;
        }
        self.kind_of(slot).iter().map(Vec::len).sum()
    }

    /// The ids of every one of the kind of `slot` that can be opened, the oldest first.
    pub(crate) fn sealed_kind(&self, slot: usize) -> Vec<u64> {
        let mut ids: Vec<u64> = self.kind_of(slot).iter().flatten().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// The slot of the tier sealed holocron or crystal `id` is of.
    pub(crate) fn slot_of(&self, id: u64) -> Option<usize> {
        (0..SLOTS).find(|&slot| self.sealed_of(slot).contains(&id))
    }

    /// Whether the hub said opening is not for the player yet (so the page says so in
    /// place of Open and Open all).
    pub(crate) fn opening_closed(&self) -> bool {
        self.state == State::Known
            && self.every.is_some()
            && !self.can_open
            && self.update.is_none()
    }

    /// What the page says in place of Open and Open all when this build is older than the
    /// hub's minimum client.
    pub(crate) fn update_line(&self) -> Option<String> {
        (self.state == State::Known)
            .then_some(self.update.as_deref())
            .flatten()
            .map(crate::economy::update_line)
    }
}

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Show another of the Collection screen's tabs (a click on the row).
    Hub(crate::profile_hub::Tab),
    /// Ask the hub to open these holocrons ([`opening`]).
    Open(Vec<u64>),
    /// Show the Collection's Forge on its Shop (Shop, or S).
    Shop,
}

pub(crate) struct Panel {
    /// The tier row the pointer rests on, for its hover card.
    hover: crate::menu::sjk::hover::Hover,
    open: bool,
    owns_console: bool,
    /// Shown as the Profile screen's Holocrons tab: its title and row of tabs at the top,
    /// the page moved down under them.
    hub: bool,
    /// What the Profile screen's tabs put at the top.
    hub_header: crate::profile_hub::Header,
    /// The Collection row's counts after each tab's name, in the row's order.
    row: Vec<String>,
    ui: MenuCanvas,
    /// The tier chosen, an index into [`holocrons::TIERS`].
    selected: usize,
    /// What the hub last said, and when it was read.
    data: Data,
    refreshed: Option<Instant>,
    /// The opening's ceremony, or the wait for the hub, while it shows.
    ceremony: Option<opening::Ceremony>,
    /// Bonus covers reveal after the ordinary prizes, within the same opening.
    bonus_prizes: Vec<opening::Prize>,
    /// The loot table shows over the page.
    loot: bool,
    /// Open all's question shows, the focus on its Open all or Cancel ([`ask_all`]).
    ask_all: Option<ask_all::AskAll>,
    /// The serial of the identity's last opening the page saw, so a new one is its
    /// answer.
    opening_serial: u64,
    /// The page's clock, for the finishes' motion.
    epoch: Instant,
    /// The blade skins the hub's packs brought, the crate tiles' blades.
    skins: std::sync::Arc<crate::saber_skins::LoadedSkins>,
    /// The cues the holocron (or round) on show has played, and its strips' tiles under
    /// the marker last frame (one tick a tile).
    played: opening::Played,
    /// When the 3D holocron set off for the middle of the window (the opening was
    /// asked for), and how long the hub then took to answer, so its glide and turn
    /// carry on into the unseal.
    glide_from: Option<Instant>,
    waited: f32,
    /// The data is a world shot's own, never read again.
    #[cfg(test)]
    pinned: bool,
    /// The time a world shot draws the ceremony's finishes at.
    #[cfg(test)]
    pub(crate) shot_seconds: Option<f32>,
    /// How far into its entrance a world shot draws the ceremony's card.
    #[cfg(test)]
    pub(crate) shot_elapsed: Option<f32>,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            hover: Default::default(),
            open: false,
            owns_console: false,
            hub: false,
            hub_header: crate::profile_hub::Header::default(),
            row: Vec::new(),
            // An opening of twenty: five strips of tiles at once, then all twenty found.
            ui: MenuCanvas::with_capacities(224, 160, 2_400),
            selected: 0,
            data: Data::nothing(State::Waiting),
            refreshed: None,
            ceremony: None,
            bonus_prizes: Vec::new(),
            loot: false,
            ask_all: None,
            opening_serial: 0,
            epoch: Instant::now(),
            #[cfg(test)]
            pinned: false,
            skins: Default::default(),
            played: opening::Played::default(),
            glide_from: None,
            waited: 0.0,
            #[cfg(test)]
            shot_seconds: None,
            #[cfg(test)]
            shot_elapsed: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page on its own, on the first tier; `owns_console` when the console was
    /// closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.hub = false;
        self.selected = 0;
        self.refreshed = None;
        self.ceremony = None;
        self.loot = false;
        self.ask_all = None;
    }

    /// Show the page as the Profile screen's Holocrons tab (`true`), or on its own;
    /// [`Self::open`] puts it back on its own.
    pub(crate) fn set_hub(&mut self, hub: bool) {
        self.hub = hub;
    }

    /// Whether the page is the Profile screen's Holocrons tab.
    pub(crate) fn is_hub(&self) -> bool {
        self.hub
    }

    /// What the Profile screen's tabs put at the top.
    /// Put `row`'s counts after the Collection's tabs' names.
    pub(crate) fn set_collection_row(&mut self, row: Vec<String>) {
        self.row = row;
    }

    pub(crate) fn hub_header(&self) -> &crate::profile_hub::Header {
        &self.hub_header
    }

    /// Put `header` at the top on the Profile screen.
    pub(crate) fn set_hub_header(&mut self, header: crate::profile_hub::Header) {
        self.hub_header = header;
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.ceremony = None;
        self.loot = false;
        self.ask_all = None;
        self.owns_console = false;
        owned
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Whether the last frame ran out of room on the canvas.
    #[cfg(test)]
    pub(crate) fn overflowed(&self) -> bool {
        self.ui.overflowed()
    }

    /// Read the hub's counts and list again when they are half a second old; `read`
    /// makes them (the live identity's, [`Data::read`]). A world shot's own are kept.
    pub(crate) fn refresh(&mut self, now: Instant, read: impl FnOnce() -> Data) {
        #[cfg(test)]
        if self.pinned {
            return;
        }
        if self
            .refreshed
            .is_none_or(|at| now.saturating_duration_since(at) >= REFRESH)
        {
            self.data = read();
            self.refreshed = Some(now);
        }
    }

    /// Show `data` in place of the live ones, for a world shot.
    #[cfg(test)]
    pub(crate) fn pin(&mut self, data: Data) {
        self.data = data;
        self.pinned = true;
    }

    /// What the page last showed.
    #[cfg(test)]
    pub(crate) fn data(&self) -> &Data {
        &self.data
    }

    /// The tier chosen: a holocron's, or a kyber crystal's.
    pub(crate) fn selected(&self) -> &'static Tier {
        holocrons::slot(self.selected)
    }

    /// Show the hover card of tier row `index` (`None` for none), for a world shot.
    #[cfg(test)]
    pub(crate) fn hover_for_shot(&mut self, index: Option<usize>) {
        self.hover.pin(index.map(|index| ROW_BASE + index as u16));
    }

    /// Choose the tier at slot `index` (kept to the tiers there are: the holocrons', then
    /// the kyber crystals'), for a world shot and the keys.
    pub(crate) fn select(&mut self, index: usize) {
        self.selected = index.min(SLOTS - 1);
    }

    /// What the 3D holocron shows: the chosen tier, in its own look while the player
    /// holds one of it, else the locked look; while one is opened, that one.
    pub(crate) fn stage_request(&self) -> holocrons::stage::Request {
        if let Some(tier) = self
            .ceremony
            .as_ref()
            .and_then(opening::Ceremony::stage_tier)
        {
            return holocrons::stage::Request { tier, owned: true };
        }
        holocrons::stage::Request {
            tier: self.selected,
            owned: self
                .data
                .count(self.selected)
                .is_some_and(|count| count > 0),
        }
    }

    /// Step the choice `by` tiers: round the list for Tab, stopping at its ends for the
    /// arrows and the wheel.
    fn step(&mut self, by: isize, wrap: bool) {
        let count = SLOTS as isize;
        let next = self.selected as isize + by;
        self.selected = if wrap {
            next.rem_euclid(count)
        } else {
            next.clamp(0, count - 1)
        } as usize;
    }

    /// A key.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent, shift: bool) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        self.hover.keyed();
        self.key(key, shift)
    }

    /// A pressed key, by its code.
    fn key(&mut self, key: KeyCode, shift: bool) -> PanelAction {
        let now = Instant::now();
        if self.ceremony.is_some() {
            return self.ceremony_key(key, now);
        }
        if self.loot {
            return self.loot_key(key);
        }
        if self.ask_all.is_some() {
            return self.ask_all_key(key, now);
        }
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(if shift { -1 } else { 1 }, true),
            KeyCode::ArrowUp | KeyCode::ArrowLeft => self.step(-1, false),
            KeyCode::ArrowDown | KeyCode::ArrowRight => self.step(1, false),
            KeyCode::Home => self.select(0),
            KeyCode::End => self.select(SLOTS - 1),
            KeyCode::Digit1 | KeyCode::Numpad1 => self.select(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => self.select(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => self.select(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => self.select(3),
            // The kyber crystals.
            KeyCode::Digit5 | KeyCode::Numpad5 => self.select(4),
            KeyCode::Digit6 | KeyCode::Numpad6 => self.select(5),
            KeyCode::Digit7 | KeyCode::Numpad7 => self.select(6),
            KeyCode::Digit8 | KeyCode::Numpad8 => self.select(7),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(ids) = self.open_one() {
                    return self.ask(ids, now);
                }
            }
            KeyCode::KeyA => return self.open_all(now),
            KeyCode::KeyL => self.show_loot(true),
            KeyCode::KeyS => return PanelAction::Shop,
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let now = Instant::now();
        self.hover.moved();
        if self.ceremony.is_some() {
            // Any click moves the ceremony on, its button or not.
            if let InputEvent::PointerRelease { .. } = event {
                let _ = self.ui.pointer(event);
                self.advance(now);
            } else if !matches!(event, InputEvent::PointerWheel { .. }) {
                let _ = self.ui.pointer(event);
            }
            return PanelAction::None;
        }
        if self.loot {
            return self.loot_pointer(event);
        }
        if self.ask_all.is_some() {
            return self.ask_all_pointer(event, now);
        }
        if let InputEvent::PointerWheel { delta, .. } = event {
            // Up (positive) goes to the tier above.
            if delta.y > 0.0 {
                self.step(-1, false);
            } else if delta.y < 0.0 {
                self.step(1, false);
            }
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(token) if (ROW_BASE..ROW_BASE + SLOTS as u16).contains(&token) => {
                self.select(usize::from(token - ROW_BASE));
                PanelAction::None
            }
            Some(OPEN_TOKEN) => self
                .open_one()
                .map_or(PanelAction::None, |ids| self.ask(ids, now)),
            Some(OPEN_ALL_TOKEN) => self.open_all(now),
            Some(LOOT_TOKEN) => {
                self.show_loot(true);
                PanelAction::None
            }
            Some(SHOP_TOKEN) => PanelAction::Shop,
            Some(token) if self.hub => match crate::profile_hub::collection_tab_of(token) {
                Some(tab) if tab != crate::profile_hub::Tab::Holocrons => PanelAction::Hub(tab),
                _ => PanelAction::None,
            },
            _ => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(counts: [u32; COUNT]) -> Data {
        Data {
            counts: Some(counts),
            ..Data::nothing(State::Known)
        }
    }

    #[test]
    fn the_keys_choose_a_tier_and_stop_at_the_ends_while_tab_comes_round() {
        let mut panel = Panel::new();
        panel.open(true);
        assert_eq!(
            panel.selected().id,
            "uncommon",
            "it opens on the first tier"
        );
        let _ = panel.key(KeyCode::ArrowUp, false);
        assert_eq!(panel.selected, 0, "the arrows stop at the ends");
        let _ = panel.key(KeyCode::ArrowDown, false);
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.selected().id, "legendary");
        let _ = panel.key(KeyCode::ArrowLeft, false);
        assert_eq!(panel.selected().id, "rare");
        let _ = panel.key(KeyCode::Digit4, false);
        assert_eq!(panel.selected().id, "mythical");
        let _ = panel.key(KeyCode::ArrowDown, false);
        assert_eq!(
            panel.selected().id,
            "kyber_uncommon",
            "the kyber crystals come after the holocrons"
        );
        let _ = panel.key(KeyCode::End, false);
        assert_eq!(panel.selected().id, "kyber_mythical");
        let _ = panel.key(KeyCode::ArrowDown, false);
        assert_eq!(panel.selected, SLOTS - 1, "nothing after the last crystal");
        let _ = panel.key(KeyCode::Tab, false);
        assert_eq!(panel.selected, 0, "Tab comes round");
        let _ = panel.key(KeyCode::Tab, true);
        assert_eq!(panel.selected, SLOTS - 1, "and Shift+Tab back");
        let _ = panel.key(KeyCode::Home, false);
        assert_eq!(panel.selected, 0);
        for (key, id) in [
            (KeyCode::Digit2, "rare"),
            (KeyCode::Digit3, "legendary"),
            (KeyCode::Numpad4, "mythical"),
            (KeyCode::Digit5, "kyber_uncommon"),
            (KeyCode::Numpad7, "kyber_legendary"),
            (KeyCode::Digit8, "kyber_mythical"),
            (KeyCode::Digit1, "uncommon"),
        ] {
            let _ = panel.key(key, false);
            assert_eq!(panel.selected().id, id);
        }
        // Nothing opens: Enter and Space change nothing.
        let _ = panel.key(KeyCode::Digit2, false);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(panel.key(KeyCode::Space, false), PanelAction::None);
        assert_eq!(panel.selected().id, "rare");
        assert_eq!(panel.key(KeyCode::Escape, false), PanelAction::Close);
        // Opening again starts at the top.
        panel.open(true);
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn the_wheel_steps_the_choice() {
        let mut panel = Panel::new();
        panel.open(true);
        let wheel = |panel: &mut Panel, y: f32| {
            panel.handle_pointer(InputEvent::PointerWheel {
                position: sjk_ui::Vec2::new(1_400.0, 500.0),
                delta: sjk_ui::Vec2::new(0.0, y),
            })
        };
        assert_eq!(wheel(&mut panel, -1.0), PanelAction::None);
        assert_eq!(wheel(&mut panel, -1.0), PanelAction::None);
        assert_eq!(panel.selected, 2, "down goes to the next tier");
        let _ = wheel(&mut panel, 1.0);
        assert_eq!(panel.selected, 1);
        for _ in 0..9 {
            let _ = wheel(&mut panel, 1.0);
        }
        assert_eq!(panel.selected, 0, "it stops at the first");
    }

    #[test]
    fn the_stage_shows_the_tier_chosen_and_dims_one_held_none_of() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = held([3, 0, 1, 0]);
        let request = |panel: &Panel| panel.stage_request();
        assert_eq!(request(&panel).tier, 0);
        assert!(request(&panel).owned);
        panel.select(1);
        assert!(!request(&panel).owned, "none held: the locked look");
        assert_eq!(
            request(&panel).model(),
            "models/sjk/holocron_rare_locked.md3"
        );
        panel.select(2);
        assert_eq!(
            request(&panel).model(),
            "models/sjk/holocron_legendary.md3",
            "the chosen tier's own model"
        );
        panel.select(3);
        assert!(!request(&panel).owned);
        // Not known (no identity, no answer): shown dimmed, never hidden.
        panel.data = Data::nothing(State::IdentityOff);
        panel.select(0);
        assert!(!request(&panel).owned);
        panel.select(99);
        assert_eq!(panel.selected, SLOTS - 1, "kept to the tiers there are");
        // A kyber crystal is drawn by its slot (the stage falls back to the Mythical
        // holocron's model until the crystals' own come), its own look while one is held.
        let mut data = held([3, 0, 1, 0]);
        data.crystals = Some([0, 2, 0, 0]);
        panel.data = data;
        let rare = crate::kyber::TIERS[1].slot;
        panel.select(rare);
        assert_eq!(
            request(&panel),
            holocrons::stage::Request {
                tier: rare,
                owned: true
            }
        );
        assert!(
            !request(&panel).parts().is_empty(),
            "a model, never a panic"
        );
        panel.select(crate::kyber::TIERS[0].slot);
        assert!(!request(&panel).owned, "none held: locked");
    }

    /// The kyber crystals are read from the profile's list (holocron rows of their tiers),
    /// kept out of the holocrons' counts and recent finds, and opened on their own: Open
    /// takes the oldest of the chosen crystal tier, Open all every crystal.
    #[test]
    fn crystals_are_counted_apart_and_open_on_their_own() {
        use sjk_identity::Status;
        let mut list = vec![
            holocron(30, "kyber_rare", "achievement", ""),
            holocron(29, "rare", "play", ""),
            holocron(28, "kyber_rare", "achievement", ""),
            holocron(27, "kyber_mythical", "achievement", ""),
            holocron(26, "kyber_uncommon", "achievement", ""),
        ];
        list[4].opened = Some(1);
        let mut snapshot = snapshot(Status::Online, Some(profile(list)));
        snapshot.kyber = Some(sjk_identity::KyberTables {
            tiers: Vec::new(),
            pity: Some(sjk_identity::KyberPity {
                every: 10,
                since: 3,
            }),
        });
        let mut data = Data::of(true, &snapshot);
        assert_eq!(
            data.crystals,
            Some([0, 2, 0, 1]),
            "the opened one is not held"
        );
        assert_eq!(data.counts, Some([5, 3, 1, 0]), "the hub's holocron counts");
        assert_eq!(data.total(), Some(9), "crystals are not holocrons");
        assert_eq!(data.recent.len(), 1, "the recent finds are holocrons");
        assert_eq!(data.recent[0].tier, 1);
        assert_eq!(data.pity.map(|pity| pity.within()), Some(7));
        let rare = crate::kyber::TIERS[1].slot;
        assert_eq!(data.count(rare), Some(2));
        assert_eq!(data.sealed_of(rare), [28, 30], "the oldest first");
        assert_eq!(data.slot_of(27), Some(crate::kyber::TIERS[3].slot));
        data.can_open = true;
        assert_eq!(data.openable(rare), 3);
        assert_eq!(data.openable(1), 1);
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = data;
        panel.select(rare);
        assert_eq!(panel.open_one(), Some(vec![28]));
        assert_eq!(panel.open_every(), Some(vec![27, 28, 30]), "every crystal");
        panel.select(1);
        assert_eq!(panel.open_every(), Some(vec![29]), "every holocron");
        let _ = panel.ask(vec![27], Instant::now());
        assert_eq!(
            panel.stage_request().tier,
            crate::kyber::TIERS[3].slot,
            "the crystal asked for"
        );
    }

    #[test]
    fn the_hubs_counts_and_list_are_read_twice_a_second_not_every_frame() {
        let mut panel = Panel::new();
        panel.open(true);
        let start = Instant::now();
        let mut reads = 0;
        for frame in 0..30 {
            panel.refresh(start + Duration::from_millis(frame * 16), || {
                reads += 1;
                held([reads, 0, 0, 0])
            });
        }
        assert_eq!(reads, 1, "30 frames in 0.5 s read once");
        panel.refresh(start + Duration::from_millis(520), || {
            reads += 1;
            held([reads, 0, 0, 0])
        });
        assert_eq!(panel.data().count(0), Some(2));
        // A reopened page reads at once.
        panel.open(true);
        panel.refresh(start + Duration::from_millis(530), || {
            reads += 1;
            held([reads, 0, 0, 0])
        });
        assert_eq!(panel.data().count(0), Some(3));
    }

    #[test]
    fn nothing_is_known_before_the_identity_starts() {
        let data = Data::read(true);
        assert_eq!(data, Data::nothing(State::IdentityOff));
        assert_eq!((data.total(), data.count(0)), (None, None));
    }

    fn snapshot(
        status: sjk_identity::Status,
        me: Option<sjk_identity::Profile>,
    ) -> sjk_identity::Snapshot {
        sjk_identity::Snapshot {
            loot: Vec::new(),
            min_client: None,
            opening: None,
            reveal: None,
            local_key_id: "0123456789abcdef".into(),
            status,
            key_id: "0123456789abcdef".into(),
            me,
            server: None,
            map_blocks: Vec::new(),
            remap_blocks: None,
            remap_blocks_revision: 0,
            players: Vec::new(),
            profiles: std::collections::HashMap::new(),
            model_checks: std::collections::HashMap::new(),
            profiles_revision: 0,
            me_revision: 0,
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            avatar: None,
            look_outcome: None,
            packs_revision: 0,
            map_edits_revision: 0,
            map_edits_note: None,
            map_edits_published: None,
            mystery_boxes: None,
            mystery_boxes_revision: 0,
            mystery_box: None,
            junk: None,
            junk_sale: None,
            box_hunt: None,
            secrets: None,
            secrets_revision: 0,
            secret_shows: None,
            secret_shows_revision: 0,
            secret: None,
            assets_note: None,
            crashes_sent: 0,
            crash_note: None,
            holocrons: None,
            log_requests: Vec::new(),
            log_answer: None,
            economy: None,
            kyber: None,
            shop: None,
            trade: None,
            tag_trade: None,
            marks: None,
            marks_revision: 0,
            mark_refusal: None,
            trades: None,
            history: None,
            announcements: Vec::new(),
            announcements_revision: 0,
            rewards: None,
            rewards_revision: 0,
            claim: None,
            announce: None,
            news_image: None,
            public_key: String::new(),
        }
    }

    fn profile(holocrons: Vec<sjk_identity::Holocron>) -> sjk_identity::Profile {
        sjk_identity::Profile {
            tag_cards: Vec::new(),
            credits: 0,
            cards: Vec::new(),
            keys: Vec::new(),
            key_id: "0123456789abcdef".into(),
            key: String::new(),
            name: "Sol".into(),
            banner: "".into(),
            bio: String::new(),
            verified: false,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            jkr_creator: false,
            trust: None,
            emoji: String::new(),
            created: 0,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            unlocks: Vec::new(),
            avatar: String::new(),
            holocron_counts: sjk_identity::HolocronCounts {
                uncommon: 5,
                rare: 3,
                legendary: 1,
                mythical: 0,
            },
            holocrons,
            shards: 0,
        }
    }

    fn holocron(id: u64, tier: &str, source: &str, note: &str) -> sjk_identity::Holocron {
        sjk_identity::Holocron {
            opened: None,
            traded: None,
            traded_from_key: None,
            traded_from: None,
            edition: sjk_identity::wire::EDITION_FIRST.to_owned(),
            id,
            tier: tier.to_owned(),
            dropped: 1_791_641_100,
            source: source.to_owned(),
            note: note.to_owned(),
        }
    }

    /// Each unopened holocron and crystal keeps its edition (06/11/2026): a tier's hover
    /// says the one edition they are all of, else how many of each, the oldest first; an
    /// opened one does not count, and the recent list says each one's.
    #[test]
    fn the_unopened_ones_say_their_editions() {
        use sjk_identity::Status;
        let beta = |id, tier| sjk_identity::Holocron {
            edition: crate::editions::BETA.to_owned(),
            ..holocron(id, tier, "play", "")
        };
        let mut opened = beta(5, "rare");
        opened.opened = Some(1);
        let list = vec![
            opened,
            beta(4, "rare"),
            holocron(3, "rare", "play", ""),
            holocron(2, "rare", "play", ""),
            holocron(1, "legendary", "play", ""),
            beta(6, "kyber_uncommon"),
        ];
        let data = Data::of(true, &snapshot(Status::Online, Some(profile(list))));
        assert_eq!(data.editions_line(1).as_deref(), Some("2 Alpha, 1 Beta"));
        assert_eq!(data.editions_line(2).as_deref(), Some("Alpha Edition"));
        assert_eq!(data.editions_line(3), None, "none held");
        let crystal = crate::kyber::TIERS[0].slot;
        assert_eq!(data.editions_line(crystal).as_deref(), Some("Beta Edition"));
        let editions: Vec<&str> = data
            .recent
            .iter()
            .map(|recent| recent.edition.as_str())
            .collect();
        assert_eq!(editions, ["Beta", "Beta", "Alpha", "Alpha", "Alpha"]);
    }

    /// The page follows the identity and the hub like the Collection does: off, no hub,
    /// waiting, then the own profile's counts, its newest ten (the known tiers) and the
    /// progress as the hub last said.
    #[test]
    fn the_data_follows_the_identity_and_the_own_profile() {
        use sjk_identity::Status;
        let online = snapshot(Status::Online, Some(profile(Vec::new())));
        assert_eq!(Data::of(false, &online).state, State::IdentityOff);
        assert_eq!(
            Data::of(true, &snapshot(Status::Disabled, None)).state,
            State::IdentityOff
        );
        assert_eq!(
            Data::of(true, &snapshot(Status::NoHub, None)).state,
            State::NoHub
        );
        assert_eq!(
            Data::of(true, &snapshot(Status::Registering, None)).state,
            State::Waiting
        );
        assert_eq!(
            Data::of(true, &snapshot(Status::Registering, None)).counts,
            None
        );
        let known = Data::of(true, &online);
        assert_eq!(known.state, State::Known);
        assert_eq!(known.counts, Some([5, 3, 1, 0]));
        assert_eq!(known.total(), Some(9));
        assert_eq!(known.count(1), Some(3));
        assert!(
            known.next.is_empty() && known.today.is_none(),
            "no progress yet"
        );
        // Twelve holocrons, one of an unknown tier, a gift: the newest ten known ones.
        let mut list: Vec<sjk_identity::Holocron> = (0..12)
            .map(|id| holocron(100 - id, "rare", "play", ""))
            .collect();
        list[1] = holocron(99, "from_the_future", "play", "");
        list[0] = holocron(100, "legendary", "staff", "For the ^1fog^7 bug");
        let mut snapshot = snapshot(Status::Online, Some(profile(list)));
        snapshot.holocrons = Some(sjk_identity::HolocronProgress {
            progress_secs: 600,
            every_secs: 1_800,
            today: 3,
            daily_cap: 8,
            resets_at: None,
            server_time: 0,
            can_open: false,
            catchup: None,
            activity: None,
            read_at: Instant::now(),
        });
        let data = Data::of(true, &snapshot);
        assert!(data.opening_closed(), "a player while opening is for staff");
        assert_eq!(data.openable(0), 0);
        assert_eq!(data.recent.len(), RECENT_SHOWN);
        assert_eq!(data.recent[0].tier, 2);
        assert!(data.recent[0].gift);
        assert_eq!(data.recent[0].note, "For the fog bug");
        assert_eq!(data.recent[0].when, "10/10/2026 14:05");
        assert_eq!(data.recent[1].tier, 1, "the unknown tier is left out");
        assert!(!data.recent[1].gift);
        assert_eq!(data.next, "Next holocron in about 20 minutes of play");
        assert_eq!(data.today, Some((3, 8)));
        assert!((data.fraction.unwrap() - 1.0 / 3.0).abs() < 1e-6);
    }

    /// A hub minimum client: a dev build (as tests are) still opens; a build older than it
    /// opens nothing and says to update in place of the buttons, not that opening is closed.
    #[test]
    fn an_old_build_says_to_update() {
        let mut snapshot = snapshot(sjk_identity::Status::Online, Some(profile(Vec::new())));
        snapshot.holocrons = Some(sjk_identity::HolocronProgress {
            progress_secs: 600,
            every_secs: 1_800,
            today: 0,
            daily_cap: 8,
            resets_at: None,
            server_time: 0,
            can_open: true,
            catchup: None,
            activity: None,
            read_at: Instant::now(),
        });
        snapshot.min_client = Some("2026.1022.1".into());
        let data = Data::of(true, &snapshot);
        assert!(crate::build_info::VERSION == "dev" || crate::build_info::meets_min("2026.1022.1"));
        assert_eq!((data.can_open, data.update.as_deref()), (true, None));
        let mut old = openable();
        old.can_open = false;
        old.update = Some("2026.1022.1".into());
        assert_eq!(old.openable(0), 0);
        assert!(!old.opening_closed());
        assert_eq!(
            old.update_line().as_deref(),
            Some("Update SJK to open holocrons: this needs SJK 2026.1022.1 or newer.")
        );
        assert_eq!(openable().update_line(), None);
    }

    fn openable() -> Data {
        let mut data = held([2, 1, 0, 0]);
        data.sealed = [vec![3, 8], vec![5], Vec::new(), Vec::new()];
        data.credits = Some(120);
        data.every = Some(1_800);
        data.can_open = true;
        data
    }

    fn opened(tier: &str, finish: &str) -> sjk_identity::Opened {
        sjk_identity::Opened {
            banner: None,
            holocron: 3,
            tier: tier.to_owned(),
            card: Some(sjk_identity::OpenedCard {
                id: 1,
                card: None,
                rarity: "rare".to_owned(),
                finish: finish.to_owned(),
                edition: "alpha".to_owned(),
            }),
            credits: 42,
            unlock: None,
            shards: 0,
        }
    }

    /// A card with shards beside its credits is a pack (17/10/2026): still the card, its
    /// shards kept for the tiles; shards without a card are a pile.
    #[test]
    fn a_card_with_shards_is_a_pack_and_shards_alone_a_pile() {
        let pack = sjk_identity::Opened {
            shards: 4,
            ..opened("rare", "normal")
        };
        let Some(opening::Prize::Card(shown)) = opening::Prize::of(&pack) else {
            panic!("a card prize");
        };
        assert_eq!((shown.credits, shown.shards), (42, 4));
        let plain = opening::Prize::of(&opened("rare", "normal"));
        assert!(matches!(plain, Some(opening::Prize::Card(shown)) if shown.shards == 0));
        let pile = sjk_identity::Opened {
            card: None,
            credits: 0,
            ..pack
        };
        assert!(matches!(
            opening::Prize::of(&pile),
            Some(opening::Prize::Shards(pile)) if pile.shards == 4
        ));
    }

    /// A shader newer than this client is still a prize: shown by the hub's name, ending
    /// the spin on the "new shader" tile; a known one keeps its catalogue entry.
    #[test]
    fn a_shader_this_client_does_not_know_still_shows() {
        let shader = |id: &str, name: &str| sjk_identity::Opened {
            card: None,
            credits: 0,
            unlock: Some(sjk_identity::OpenedUnlock {
                finish: None,
                id: id.to_owned(),
                name: name.to_owned(),
                count: 1,
            }),
            ..opened("rare", "normal")
        };
        let Some(opening::Prize::Shader(found)) =
            opening::Prize::of(&shader("saber_from_the_future", "Comet blade"))
        else {
            panic!("a shader prize");
        };
        assert_eq!((found.skin, found.name.as_str()), (None, "Comet blade"));
        let Some(opening::Prize::Shader(nameless)) =
            opening::Prize::of(&shader("saber_from_the_future", " "))
        else {
            panic!("a shader prize");
        };
        assert_eq!(nameless.name, "A new saber shader");
        let Some(opening::Prize::Shader(tidal)) =
            opening::Prize::of(&shader("saber_tidal", "Tidal blade"))
        else {
            panic!("a shader prize");
        };
        assert_eq!(tidal.skin.map(|skin| skin.id), Some("saber_tidal"));
        assert!(!tidal.is_unknown());
        assert!(found.is_unknown());
    }

    /// Any prize newer than this client shows as a fallback of its kind (Kushina,
    /// 30/10/2026: "again new shader bug"): a tag, toy or banner it does not know says
    /// what it is, and a card of a rarity it does not know still shows, not dropped.
    #[test]
    fn any_prize_this_client_does_not_know_shows_by_its_kind() {
        let unlock = |id: &str| sjk_identity::Opened {
            card: None,
            credits: 0,
            unlock: Some(sjk_identity::OpenedUnlock {
                finish: None,
                id: id.to_owned(),
                name: String::new(),
                count: 1,
            }),
            ..opened("rare", "normal")
        };
        for (id, kind) in [
            ("tag_from_the_future", "tag"),
            ("toy_from_the_future", "toy"),
            ("banner_from_the_future", "profile banner"),
            ("saber_from_the_future", "saber shader"),
            ("something_else", "prize"),
        ] {
            let Some(opening::Prize::Shader(found)) = opening::Prize::of(&unlock(id)) else {
                panic!("{id}: a prize");
            };
            assert!(found.is_unknown(), "{id}");
            assert_eq!(found.kind(), kind, "{id}");
            assert_eq!(found.name, format!("A new {kind}"), "{id}");
            assert_eq!(
                opening::Prize::Shader(found).item(),
                super::strip::Item::New(kind)
            );
        }
        let mut future = opened("rare", "normal");
        future.card.as_mut().unwrap().rarity = "beyond_mythical".to_owned();
        let Some(opening::Prize::Shader(card)) = opening::Prize::of(&future) else {
            panic!("an unknown card still shows");
        };
        assert_eq!((card.kind(), card.name.as_str()), ("card", "A new card"));
    }

    /// A Solar blade card brings its blade beside it (20/10/2026): the card shows, with
    /// the blade as its bonus and a line saying so; a card this client cannot show (a
    /// rarity it does not know) leaves the blade alone.
    #[test]
    fn a_solar_blade_card_shows_with_its_blade() {
        let sun = |count: u32| sjk_identity::OpenedUnlock {
            finish: None,
            id: "saber_sun".to_owned(),
            name: "Sun blade".to_owned(),
            count,
        };
        let solar = sjk_identity::Opened {
            unlock: Some(sun(1)),
            ..opened("mythical", "solar")
        };
        let Some(opening::Prize::Card(shown)) = opening::Prize::of(&solar) else {
            panic!("a card prize");
        };
        assert_eq!(shown.credits, 42);
        let bonus = shown.bonus.as_ref().expect("its blade");
        assert_eq!(
            (bonus.skin.map(|skin| skin.id), bonus.name.as_str()),
            (Some("saber_sun"), "Sun blade")
        );
        assert_eq!(
            opening::bonus_line(bonus),
            "...and the Sun blade drops with it! Yours now: wear it in the Collection."
        );
        let again = sjk_identity::Opened {
            unlock: Some(sun(3)),
            ..opened("mythical", "solar")
        };
        let Some(opening::Prize::Card(again)) = opening::Prize::of(&again) else {
            panic!("a card prize");
        };
        assert_eq!(
            opening::bonus_line(again.bonus.as_ref().unwrap()),
            "...and the Sun blade drops with it! You hold 3 of it now."
        );
        assert!(matches!(
            opening::Prize::of(&opened("rare", "solar")),
            Some(opening::Prize::Card(shown)) if shown.bonus.is_none()
        ));
        let mut unknown = solar.clone();
        unknown.card.as_mut().unwrap().rarity = "exotic".to_owned();
        assert!(matches!(
            opening::Prize::of(&unknown),
            Some(opening::Prize::Shader(found)) if found.id == "saber_sun"
        ));
    }

    /// The 3D holocron is the one opened while the ceremony shows: it sets off for the
    /// middle when the hub is asked, cracks open, is gone in the flash and is back in its
    /// place after.
    #[test]
    fn the_holocron_opened_leaves_the_page_and_comes_back() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        panel.select(1);
        let start = Instant::now();
        assert!(panel.stage_motion(start).is_none(), "in its place");
        let _ = panel.ask(vec![3], start);
        let asked = panel.stage_motion(start).expect("moving");
        assert!(asked.shown);
        assert_eq!(asked.at, STAGE_AT, "from its place");
        assert_eq!(
            panel.stage_request(),
            holocrons::stage::Request {
                tier: 0,
                owned: true
            },
            "the holocron asked for"
        );
        let answer = start + Duration::from_millis(400);
        panel.show(
            vec![opening::Prize::of(&opened("legendary", "holo")).unwrap()],
            answer,
        );
        assert_eq!(panel.stage_request().tier, 2, "the one opened");
        let cracking = panel
            .stage_motion(answer + Duration::from_millis(1_000))
            .unwrap();
        assert!(cracking.shown && cracking.glow > 1.5);
        assert!(
            (cracking.at[0] - unseal::MIDDLE[0]).abs() < 8.0,
            "in the middle, shaking"
        );
        let gone = panel
            .stage_motion(answer + Duration::from_millis(1_800))
            .unwrap();
        assert!(!gone.shown, "gone in the flash");
        panel.end_ceremony();
        assert!(panel.stage_motion(answer).is_none(), "back in its place");
        assert_eq!(panel.stage_request().tier, 1);
    }

    /// More than three at once run as a batch: Enter skips to the stops, on through the
    /// rounds to the finds, and Done ends it.
    #[test]
    fn open_all_runs_strips_together_and_lays_out_the_finds() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        let now = Instant::now();
        let shown: Vec<_> = (0..7)
            .map(|_| opening::Prize::of(&opened("rare", "normal")).unwrap())
            .collect();
        panel.show(shown, now);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Batch { .. })
        ));
        // To round 1's stops, round 2, its stops, the finds, all of them in; then Done.
        for _ in 0..5 {
            panel.advance(now);
            assert!(panel.opening());
        }
        panel.advance(now);
        assert!(!panel.opening(), "Done after the finds");
        // Three or fewer still open one by one.
        let three: Vec<_> = (0..3)
            .map(|_| opening::Prize::of(&opened("rare", "normal")).unwrap())
            .collect();
        panel.show(three, now);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Showing { .. })
        ));
    }

    /// While the hub keeps opening to staff, neither Enter nor A asks it anything and the
    /// page says why.
    #[test]
    fn a_player_cannot_open_while_opening_is_for_staff() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        panel.data.can_open = false;
        assert!(panel.data.opening_closed());
        assert_eq!(panel.data.openable(0), 0);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(panel.key(KeyCode::KeyA, false), PanelAction::None);
        assert_eq!(panel.ask(vec![3], Instant::now()), PanelAction::None);
        assert!(!panel.opening());
    }

    /// Enter opens the oldest of the tier chosen and A every one, the oldest first; the
    /// page then waits for the hub, whose new answer starts the ceremony, which Enter
    /// moves on card by card and ends.
    #[test]
    fn opening_asks_the_hub_and_plays_its_answer() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        panel.select(2);
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::None,
            "none held"
        );
        panel.select(0);
        assert_eq!(panel.open_every(), Some(vec![3, 5, 8]));
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::Open(vec![3]));
        assert!(panel.opening());
        assert_eq!(
            panel.key(KeyCode::KeyA, false),
            PanelAction::None,
            "one at a time"
        );
        let now = Instant::now();
        // An old outcome (the serial seen before) is not this page's answer.
        panel.opening_serial = 4;
        let old = sjk_identity::OpenOutcome {
            serial: 4,
            opened: vec![opened("rare", "holo")],
            message: String::new(),
        };
        panel.follow_opening(now, Some(&old));
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Waiting { .. })
        ));
        let answer = sjk_identity::OpenOutcome {
            serial: 5,
            opened: vec![
                opened("uncommon", "holo"),
                opened("rare", "solar"),
                opened("uncommon", "glitter"),
            ],
            message: String::new(),
        };
        panel.follow_opening(now, Some(&answer));
        let Some(opening::Ceremony::Showing { shown, index, .. }) = &panel.ceremony else {
            panic!("the ceremony shows");
        };
        assert_eq!(
            shown.len(),
            3,
            "a finish the client does not know still shows, as a new card"
        );
        assert!(
            matches!(&shown[2], opening::Prize::Shader(card) if card.kind() == "card"),
            "{:?}",
            shown[2]
        );
        let (opening::Prize::Card(first), opening::Prize::Card(second)) = (&shown[0], &shown[1])
        else {
            panic!("two cards");
        };
        assert_eq!((*index, second.finish.id, second.credits), (0, "solar", 42));
        assert_eq!(first.edition, "Alpha Edition");
        // The first press skips the crate spin, the next finishes the entrance, the next
        // goes on, the last ends it.
        panel.advance(now);
        panel.advance(now);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Showing { index: 0, .. })
        ));
        panel.advance(now);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Showing { index: 1, .. })
        ));
        panel.advance(now);
        panel.advance(now);
        panel.advance(now);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Showing { index: 2, .. })
        ));
        panel.advance(now);
        panel.advance(now);
        panel.advance(now);
        assert!(!panel.opening(), "Done after the last");
        assert!(panel.refreshed.is_none(), "the page reads the hub again");
    }

    /// A refusal or no answer says why; Escape ends the ceremony at any card.
    #[test]
    fn a_refusal_or_no_answer_says_why() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        let now = Instant::now();
        let _ = panel.ask(vec![3], now);
        let refused = sjk_identity::OpenOutcome {
            serial: 1,
            opened: Vec::new(),
            message: "That holocron was already opened.".to_owned(),
        };
        panel.follow_opening(now, Some(&refused));
        assert!(matches!(
            &panel.ceremony,
            Some(opening::Ceremony::Failed { message, .. }) if message == "That holocron was already opened."
        ));
        panel.advance(now);
        assert!(!panel.opening());
        let _ = panel.ask(vec![3], now);
        panel.follow_opening(now + Duration::from_secs(5), None);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Waiting { .. })
        ));
        panel.follow_opening(now + Duration::from_secs(16), None);
        assert!(matches!(
            panel.ceremony,
            Some(opening::Ceremony::Failed { .. })
        ));
        panel.show(
            vec![opening::Prize::of(&opened("rare", "holo")).unwrap()],
            now,
        );
        assert_eq!(panel.key(KeyCode::Escape, false), PanelAction::None);
        assert!(!panel.opening(), "Escape ends it, the page stays");
        assert!(panel.is_open());
    }

    /// Every moment of the ceremony fits the canvas, in every finish and window.
    #[test]
    fn the_ceremony_fits_the_canvas() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        let start = Instant::now();
        let shader = |tier: &str, id: &str| sjk_identity::Opened {
            card: None,
            credits: 0,
            unlock: Some(sjk_identity::OpenedUnlock {
                finish: None,
                id: id.to_owned(),
                name: "A blade".to_owned(),
                count: 2,
            }),
            ..opened(tier, "normal")
        };
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
            // One by one: the unseal, the spin, the entrance and the words, every tenth.
            for finish in ["normal", "holo", "solar"] {
                // The second a pack, its shards' tile sliding out too.
                let pack = sjk_identity::Opened {
                    shards: 3,
                    ..opened("rare", finish)
                };
                // The third a blade card with its blade beside its shards (20/10/2026).
                let blade = sjk_identity::Opened {
                    unlock: Some(sjk_identity::OpenedUnlock {
                        finish: None,
                        id: "saber_sun".to_owned(),
                        name: "Sun blade".to_owned(),
                        count: 1,
                    }),
                    ..pack.clone()
                };
                let shown = vec![
                    opening::Prize::of(&opened("mythical", finish)).unwrap(),
                    opening::Prize::of(&pack).unwrap(),
                    opening::Prize::of(&blade).unwrap(),
                ];
                panel.show(shown, start);
                for tenth in 0..110 {
                    panel.build_at(viewport, start + Duration::from_millis(tenth * 100));
                    assert!(!panel.overflowed(), "{finish} at {tenth} in {viewport:?}");
                }
            }
            // Twenty at once, cards of every finish and shaders: every round and the
            // finds laid out, every quarter second.
            let twenty: Vec<opening::Prize> = (0..20)
                .map(|at| {
                    let tier = ["uncommon", "rare", "legendary", "mythical"][at % 4];
                    let opened = match at % 5 {
                        0 => shader(tier, "saber_tidal"),
                        1 => shader(tier, "saber_phoenix"),
                        2 => opened(tier, "holo"),
                        3 => sjk_identity::Opened {
                            unlock: Some(sjk_identity::OpenedUnlock {
                                finish: None,
                                id: "saber_sun".to_owned(),
                                name: "Sun blade".to_owned(),
                                count: 1,
                            }),
                            ..opened(tier, "solar")
                        },
                        _ => opened(tier, "normal"),
                    };
                    opening::Prize::of(&opened).unwrap()
                })
                .collect();
            panel.show(twenty, start);
            assert!(matches!(
                panel.ceremony,
                Some(opening::Ceremony::Batch { .. })
            ));
            for quarter in 0..120 {
                panel.build_at(viewport, start + Duration::from_millis(quarter * 250));
                assert!(!panel.overflowed(), "batch at {quarter} in {viewport:?}");
            }
            // Twenty Holo cards, the most shapes a light tile takes (its credits, its card
            // tucked in its corner, the rainbow across it).
            let holo: Vec<opening::Prize> = (0..20)
                .map(|at| {
                    let tier = ["uncommon", "rare", "legendary", "mythical"][at % 4];
                    opening::Prize::of(&opened(tier, "holo")).unwrap()
                })
                .collect();
            panel.show(holo, start);
            for quarter in 0..60 {
                panel.build_at(viewport, start + Duration::from_millis(quarter * 250));
                assert!(
                    !panel.overflowed(),
                    "holo batch at {quarter} in {viewport:?}"
                );
            }
            let _ = panel.ask(vec![3], start);
            panel.ceremony = Some(opening::Ceremony::Waiting {
                tier: 3,
                since: start,
            });
            panel.build_at(viewport, start);
            panel.refused("No.", start);
            panel.build_at(viewport, start);
            assert!(!panel.overflowed());
            panel.end_ceremony();
        }
    }

    /// Open all asks first when more than one would open: Enter (or Open all) asks the
    /// hub, Escape, Cancel or Enter on Cancel do not; one holocron opens at once.
    #[test]
    fn open_all_asks_before_opening_more_than_one() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        // A asks; the keys under it are the question's.
        assert_eq!(panel.key(KeyCode::KeyA, false), PanelAction::None);
        assert!(panel.asking_all() && !panel.opening());
        assert_eq!(panel.key(KeyCode::ArrowDown, false), PanelAction::None);
        assert_eq!(panel.selected, 0, "the tiers do not move under it");
        assert_eq!(panel.key(KeyCode::Escape, false), PanelAction::None);
        assert!(!panel.asking_all() && !panel.opening(), "Escape cancels");
        assert!(panel.is_open(), "and the page stays");
        // Tab to Cancel, Enter: nothing opens.
        let _ = panel.key(KeyCode::KeyA, false);
        let _ = panel.key(KeyCode::Tab, false);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert!(!panel.asking_all() && !panel.opening());
        // Enter on Open all opens every one, the oldest first.
        let _ = panel.key(KeyCode::KeyA, false);
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::Open(vec![3, 5, 8])
        );
        assert!(panel.opening() && !panel.asking_all());
        panel.end_ceremony();
        // One holocron: A opens it at once.
        panel.data.sealed = [vec![8], Vec::new(), Vec::new(), Vec::new()];
        assert_eq!(panel.key(KeyCode::KeyA, false), PanelAction::Open(vec![8]));
        assert!(!panel.asking_all());
    }

    /// The question's buttons answer the pointer: Cancel closes it, Open all asks the hub.
    #[test]
    fn open_all_question_answers_the_pointer() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = openable();
        let viewport = [1920.0, 1080.0];
        let click = |panel: &mut Panel, token: u16| {
            panel.build(viewport);
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                action = panel.handle_pointer(event);
            }
            action
        };
        assert_eq!(click(&mut panel, OPEN_ALL_TOKEN), PanelAction::None);
        assert!(panel.asking_all());
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
            panel.build(viewport);
            assert!(!panel.overflowed(), "{viewport:?}");
        }
        assert_eq!(
            click(&mut panel, ask_all::ASK_ALL_CANCEL_TOKEN),
            PanelAction::None
        );
        assert!(!panel.asking_all() && !panel.opening());
        let _ = click(&mut panel, OPEN_ALL_TOKEN);
        assert_eq!(
            click(&mut panel, ask_all::ASK_ALL_OPEN_TOKEN),
            PanelAction::Open(vec![3, 5, 8])
        );
        assert!(panel.opening());
    }

    /// The question says the batch's animation is simpler only when a batch would run, and
    /// what stays for later past twenty.
    #[test]
    fn open_all_question_words() {
        let (title, lines) = ask_all::words(3, 3, false);
        assert_eq!(title, "Open all 3 holocrons?");
        assert!(lines.join(" ").contains("full opening"));
        let (title, lines) = ask_all::words(7, 7, false);
        assert_eq!(title, "Open all 7 holocrons?");
        let all = lines.join(" ");
        assert!(all.contains("quicker and simpler animation"), "{all}");
        let (title, lines) = ask_all::words(20, 34, false);
        assert_eq!(title, "Open 20 of your 34 holocrons?");
        assert!(lines.join(" ").contains("the other 14 wait"));
        let (title, lines) = ask_all::words(7, 7, true);
        assert_eq!(title, "Open all 7 kyber crystals?");
        assert!(lines.join(" ").contains("every shader found"));
        for (opening, held) in [(2, 2), (7, 7), (20, 34), (20, 999)] {
            let (title, lines) = ask_all::words(opening, held, false);
            assert!(lines.len() <= 4);
            assert!(title.is_ascii() && lines.iter().all(|line| line.is_ascii()));
        }
    }

    #[test]
    fn a_click_chooses_a_tier_and_the_way_back_closes() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.build([1920.0, 1080.0]);
        let click = |panel: &mut Panel, token: u16| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + 4.0, rect.y + 4.0);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                action = panel.handle_pointer(event);
            }
            action
        };
        for (index, tier) in holocrons::TIERS.iter().enumerate() {
            assert_eq!(
                click(&mut panel, ROW_BASE + index as u16),
                PanelAction::None
            );
            assert_eq!(panel.selected().id, tier.id);
            panel.build([1920.0, 1080.0]);
        }
        assert_eq!(click(&mut panel, BACK_TOKEN), PanelAction::Close);
    }
}
