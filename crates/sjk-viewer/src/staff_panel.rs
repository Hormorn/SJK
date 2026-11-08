//! The Staff page: for a player the hub's operator made staff, the SJK team's tools
//! in the game (`docs/identity.md`, "Staff"): find a player (by name or key id, among
//! the SJK players on this server, those playing now anywhere or the SJK team; a search
//! that finds one player chooses them), then, on the chosen player's tabs, verify them
//! and moderate their picture and SJK chat (Overview), give or take back their medals
//! and saber shaders (Medals and shaders, the shaders found by name, number or tier),
//! their holocrons, kyber crystals, credits and kyber shards (Holocrons and credits),
//! clear their achievements, list their keys and unlink one or merge another player into
//! them (a player who reset their key), and ask them for their game's logs; three more
//! tabs read every player's crash reports, the SJK chat bans in force (with Lift) and the
//! staff member's own gifts of the day (with Undo). Since 08/11/2026 every staff member
//! gives medals, ranks, shaders, holocrons and crystals, a number a day and never to
//! themselves; taking back stays the creator's. Key ids never show on the
//! page (nor anywhere in the game): players go by name, a pasted key id shows as dots,
//! and linked keys are numbered. Every action is a
//! request signed by the player's own key (`sjk_identity::StaffRequest`,
//! `sjk_identity::StaffQuery`); the hub refuses it from any other key, and the page
//! only opens for a key whose profile says staff.
//!
//! Opened by the Profile page's Staff tools button or the `staff` command. Like the
//! Profile page it lives in the console and has the SJK UI's look ([`view`]). Tab
//! and Shift+Tab walk every control; Enter or Space works the one focused; the
//! search pill, the note, the shader filter and the merge field take typing; Up and Down
//! move through the players found; Escape goes back (or lets go of a merge being
//! confirmed or picked).

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_identity::{Profile, SearchScope, StaffQuery, StaffRequest, StaffState};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::{Duration, Instant};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "staff_panel_view.rs"]
mod view;

const SEARCH_TOKEN: u16 = 1_100;
const ME_TOKEN: u16 = 1_101;
const RECENT_TOKEN: u16 = 1_102;
const NOTE_TOKEN: u16 = 1_103;
const CLEAR_ALL_TOKEN: u16 = 1_104;
const PICTURE_DOWN_TOKEN: u16 = 1_105;
/// Players found, one token each.
const PLAYER_BASE: u16 = 1_110;
const PLAYERS_SHOWN: usize = 15;
/// A held medal's rank button, one token per medal of the catalogue: each press sets the
/// next rank, V going back to I (`medals/rank.rs`).
const RANK_BASE: u16 = 1_140;
/// Give and Take back, one token per medal of the catalogue.
const GIVE_BASE: u16 = 1_150;
const TAKE_BASE: u16 = 1_160;
/// Clear, one token per achievement of the catalogue.
const CLEAR_BASE: u16 = 1_170;
/// Unlock and Relock, one token per unlockable of the catalogue.
/// The unlockables' pager: previous page, next page (the list outgrew the page).
const UNLOCK_PREV_TOKEN: u16 = 7_590;
const UNLOCK_NEXT_TOKEN: u16 = 7_591;
const UNLOCK_BASE: u16 = 7_600;
const RELOCK_BASE: u16 = 8_000;
/// The tier chips (one token per holocron tier, then one per kyber crystal tier, as
/// `crate::holocrons::slot` counts them), Give, and Remove (one token per row).
const HOLOCRON_TIER_BASE: u16 = 1_300;
const HOLOCRON_GIVE_TOKEN: u16 = 1_310;
const HOLOCRON_REMOVE_BASE: u16 = 1_320;
/// The chosen player's recent holocrons and kyber crystals listed with Remove.
const HOLOCRONS_SHOWN: usize = 12;
/// Verify or Unverify the chosen player.
const VERIFY_TOKEN: u16 = 1_400;
/// Make staff or Remove staff, the creator's own button (`PROTOCOL.md` of the hub,
/// "Creator").
const SET_STAFF_TOKEN: u16 = 1_408;
/// The tick buttons, one per group of [`sjk_identity::Tick::GROUPS`] (`PROTOCOL.md` of
/// the hub, "Ticks"): verify the chosen player with that tick, or change theirs.
const TICK_BASE: u16 = 1_540;
/// The tabs: the chosen player's views, then every player's crashes and chat bans.
const ACHIEVEMENTS_TAB_TOKEN: u16 = 1_401;
const KEYS_TAB_TOKEN: u16 = 1_402;
/// The key id of the player to merge away, Pick in list, Merge, and the confirmation's
/// two buttons.
const MERGE_FIELD_TOKEN: u16 = 1_403;
const MERGE_PICK_TOKEN: u16 = 1_404;
const MERGE_TOKEN: u16 = 1_405;
const MERGE_CONFIRM_TOKEN: u16 = 1_406;
const MERGE_CANCEL_TOKEN: u16 = 1_407;
/// Unlink, one token per linked key listed.
const UNLINK_BASE: u16 = 1_410;
const LINKED_SHOWN: usize = 6;
/// The right column's Logs and Crashes views.
const LOGS_TAB_TOKEN: u16 = 1_420;
const CRASHES_TAB_TOKEN: u16 = 1_421;
/// Request logs of the chosen player, and the list read again.
const REQUEST_LOGS_TOKEN: u16 = 1_422;
const LOGS_REFRESH_TOKEN: u16 = 1_423;
/// Open, one token per log request listed (only sent ones have it).
const LOG_OPEN_BASE: u16 = 1_430;
const LOG_ROWS_SHOWN: usize = 12;
/// The crash groups read again, and back from one group's reports to the groups.
const CRASHES_REFRESH_TOKEN: u16 = 1_450;
const CRASHES_BACK_TOKEN: u16 = 1_451;
/// A crash group, one token each, and Open, one token per report of a group.
const CRASH_GROUP_BASE: u16 = 1_460;
const CRASH_GROUPS_SHOWN: usize = 9;
const CRASH_OPEN_BASE: u16 = 1_480;
const CRASH_ROWS_SHOWN: usize = 12;
/// The Overview, Medals and unlocks, Holocrons and credits, and Chat bans tabs.
const OVERVIEW_TAB_TOKEN: u16 = 1_500;
const REWARDS_TAB_TOKEN: u16 = 1_501;
const ECONOMY_TAB_TOKEN: u16 = 1_502;
const CHAT_BANS_TAB_TOKEN: u16 = 1_503;
/// The Overview's moderation: stop or let again the player's pictures, mute or unmute
/// them in the SJK chat, lift their chat bans.
const AVATAR_BLOCK_TOKEN: u16 = 1_504;
const AVATAR_ALLOW_TOKEN: u16 = 1_505;
const HUB_MUTE_TOKEN: u16 = 1_506;
const HUB_UNMUTE_TOKEN: u16 = 1_507;
const CHAT_UNBAN_TOKEN: u16 = 1_508;
/// The amount field, then Give and Take for credits and for kyber shards.
const AMOUNT_TOKEN: u16 = 1_509;
const CREDITS_GIVE_TOKEN: u16 = 1_510;
const CREDITS_TAKE_TOKEN: u16 = 1_511;
const SHARDS_GIVE_TOKEN: u16 = 1_512;
const SHARDS_TAKE_TOKEN: u16 = 1_513;
/// The chat bans read again, and Lift, one token per ban listed.
const CHAT_BANS_REFRESH_TOKEN: u16 = 1_514;
const BAN_LIFT_BASE: u16 = 1_520;
const BAN_ROWS_SHOWN: usize = 14;
/// The scope chips over the players (`sjk_identity::SearchScope`): the SJK players on this
/// server, those playing now on any server, the SJK team.
const HERE_TOKEN: u16 = 9_000;
const ONLINE_TOKEN: u16 = 9_001;
const TEAM_TOKEN: u16 = 9_002;
/// The players found, a page back and a page on (the hub answers up to 30).
const PLAYERS_PREV_TOKEN: u16 = 9_003;
const PLAYERS_NEXT_TOKEN: u16 = 9_004;
/// The saber shaders' filter (a name, a number or a tier) and Held only.
const UNLOCK_FILTER_TOKEN: u16 = 9_005;
const UNLOCK_HELD_TOKEN: u16 = 9_006;
/// The My gifts tab, its Refresh, and Undo, one token per gift listed.
const GIFTS_TAB_TOKEN: u16 = 9_007;
const GIFTS_REFRESH_TOKEN: u16 = 9_008;
const GIFT_UNDO_BASE: u16 = 9_010;
const GIFT_ROWS_SHOWN: usize = 16;
/// Longest shader filter.
const FILTER_MAX: usize = 32;
/// Longest amount of credits or shards, in digits.
const AMOUNT_MAX: usize = 6;
/// Longest text the merge field takes (a pasted key id with spaces round it).
const MERGE_FIELD_MAX: usize = 40;
/// Longest search, as the hub takes it.
const QUERY_MAX: usize = 64;
/// Longest note with a medal or an unlock, as the hub takes it.
const NOTE_MAX: usize = 200;
/// How long the buttons that take something away (Clear all, Unlink, Remove, Take
/// credits, Take shards, Make or Remove staff, Undo a gift) wait for their second press.
const CONFIRM_FOR: Duration = Duration::from_secs(3);

/// Whether `token` is one of the buttons only SJK's creator may use (the hub's
/// `PROTOCOL.md`, "Creator", since 23/10/2026): what takes back, clears, merges or makes
/// value out of nothing (credits and shards). Other staff see them dimmed; Tab skips them
/// and a press does nothing. Giving is every staff member's since 08/11/2026 ([`gift`]).
fn owner_only(token: u16) -> bool {
    let within = |base: u16, count: usize| (base..base + count as u16).contains(&token);
    matches!(
        token,
        PICTURE_DOWN_TOKEN
            | CLEAR_ALL_TOKEN
            | AMOUNT_TOKEN
            | CREDITS_GIVE_TOKEN
            | CREDITS_TAKE_TOKEN
            | SHARDS_GIVE_TOKEN
            | SHARDS_TAKE_TOKEN
            | MERGE_FIELD_TOKEN
            | MERGE_PICK_TOKEN
            | MERGE_TOKEN
            | MERGE_CONFIRM_TOKEN
    ) || within(TAKE_BASE, crate::medals::Medal::COUNT)
        || within(RELOCK_BASE, crate::unlockables::ALL.len())
        || within(HOLOCRON_REMOVE_BASE, HOLOCRONS_SHOWN)
        || within(CLEAR_BASE, crate::achievements::ALL.len())
        || within(UNLINK_BASE, LINKED_SHOWN)
}

/// Whether `token` gives something: a medal's Give and Rank, a shader's Unlock and the
/// holocrons' Give (the hub's `PROTOCOL.md`, "Staff", since 08/11/2026). Every staff
/// member may, except to themselves and past the day's number of gifts
/// ([`Shown::gift_block`]); the creator always may.
fn gift(token: u16) -> bool {
    let within = |base: u16, count: usize| (base..base + count as u16).contains(&token);
    token == HOLOCRON_GIVE_TOKEN
        || within(RANK_BASE, crate::medals::Medal::COUNT)
        || within(GIVE_BASE, crate::medals::Medal::COUNT)
        || within(UNLOCK_BASE, crate::unlockables::ALL.len())
}

/// Whether staff who are not the creator may give `unlockable`: only what holocrons can
/// bring anyway (`droppable`). A Unique one, a medal's blade or a blade not opened to
/// drops yet stays the creator's to give (the hub refuses it, `creator_only`).
fn staff_may_unlock(unlockable: &crate::unlockables::Unlockable) -> bool {
    unlockable.droppable
}

/// Where the list of players looks: everyone (a name or key id, or the players seen
/// lately), or a scope ([`SearchScope`]).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Scope {
    #[default]
    Everyone,
    /// The SJK players on the server this player is on.
    Here,
    /// The players the hub sees in a game now, on any server.
    Online,
    /// The SJK team.
    Team,
}

/// Whether `unlockable` is one the shader filter `filter` asks for: its name or id
/// containing it, its number ("42", "042" or "#42") or its tier's name ("mythical").
fn filter_matches(unlockable: &crate::unlockables::Unlockable, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    let digits = filter.strip_prefix('#').unwrap_or(filter);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return digits.parse::<u16>().ok() == Some(unlockable.number);
    }
    let normal = sjk_identity::normal_form;
    let wanted = normal(filter);
    !wanted.is_empty()
        && (normal(unlockable.name).contains(&wanted)
            || normal(unlockable.id).contains(&wanted)
            || normal(unlockable.tier.label()) == wanted)
}

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Send this to the hub.
    Request(StaffRequest),
    /// And this, about logs and crash reports.
    Query(StaffQuery),
}

/// A tab a world shot shows.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShotTab {
    Overview,
    Rewards,
    Economy,
    Achievements,
    ChatBans,
    Gifts,
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// The player's own profile, once the hub sent it.
    pub(crate) me: Option<&'a Profile>,
    pub(crate) staff: &'a StaffState,
    /// The game server the hub sees this player on (`ip:port`), for Here.
    pub(crate) server: Option<String>,
}

/// What the tabs show.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum View {
    /// The chosen player's account, flags and moderation.
    #[default]
    Overview,
    /// Their medals and unlockables.
    Rewards,
    /// Their holocrons, credits and kyber shards.
    Economy,
    Achievements,
    Keys,
    /// The log requests, with Request logs for the chosen player.
    Logs,
    /// The crash reports of every player: the groups, or one group's reports.
    Crashes,
    /// The SJK chat bans in force, of every player, with Lift.
    ChatBans,
    /// The staff member's own gifts of the last day, with Undo.
    Gifts,
}

/// A merge waiting for its confirmation: who is kept and who goes.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingMerge {
    kept: String,
    from: String,
}

/// Why the merge field's key cannot be merged into the chosen player yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MergeProblem {
    /// Nothing typed or picked.
    Empty,
    /// Not 16 hex digits.
    NotKey,
    /// The chosen player's own key (main or linked).
    Same,
    /// A staff player, which the hub never merges away.
    Staff,
}

impl MergeProblem {
    fn words(self) -> &'static str {
        match self {
            Self::Empty => "Pick the other player in the list, or paste their key id",
            Self::NotKey => "A key id is 16 hex digits",
            Self::Same => "That key is already this player's",
            Self::Staff => "A staff player cannot be merged away",
        }
    }
}

/// The merge field's text as a key id: trimmed and lower case.
fn merge_key(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

/// What the last frame showed, for keys and clicks.
#[derive(Debug, Default)]
struct Shown {
    /// The players' keys in the list, in order (the page shown).
    players: Vec<String>,
    /// How many players were found, on every page.
    players_found: usize,
    me: Option<String>,
    /// The server the hub sees this player on, which Here lists.
    server: Option<String>,
    /// Why gifts cannot be given to the chosen player now, `None` when they can: the
    /// staff member themselves, or no gift left today. Never for the creator.
    gift_block: Option<&'static str>,
    /// How many gifts the staff member has left today, once the hub said.
    gifts_left: Option<u32>,
    /// The gifts of the day were asked for since the page opened: the tabs that give ask
    /// once.
    gifts_read: bool,
    /// Which unlockables of the catalogue the filter shows, in its order, and for what
    /// (the filter, Held only, the player and how many they hold).
    unlock_rows: Vec<usize>,
    unlock_rows_for: Option<(String, bool, String, usize)>,
    /// The gifts listed with Undo, by number, with who got them.
    gift_rows: Vec<(i64, String)>,
    /// The chosen player's key, how many of each medal they hold and which
    /// unlockables.
    target: Option<String>,
    medals: [u32; crate::medals::Medal::COUNT],
    /// The rank of each medal they hold.
    ranks: [crate::medals::Rank; crate::medals::Medal::COUNT],
    /// The chosen player has a picture.
    picture: bool,
    /// Which unlockables of the catalogue they hold, in its order.
    unlocks: Vec<bool>,
    /// How many unlockables a page of the list shows.
    unlock_per_page: usize,
    /// The numbers of the holocrons listed with Remove, newest first, and how many.
    holocrons: [u64; HOLOCRONS_SHOWN],
    holocron_rows: usize,
    /// The chosen player is verified.
    verified: bool,
    /// The chosen player's tick as drawn ([`sjk_identity::Tick::shown`]).
    tick: sjk_identity::Tick,
    /// The chosen player is staff.
    staff: bool,
    /// The player's own is SJK's creator, who alone makes staff and uses the
    /// [`owner_only`] buttons.
    creator: bool,
    /// The chosen player's linked keys listed with Unlink.
    linked: Vec<String>,
    /// Why the merge field's key cannot be merged in, or `None` when it can.
    merge_problem: Option<MergeProblem>,
    /// The log requests listed, by number, and whether each has logs to open.
    log_rows: Vec<(i64, bool)>,
    /// The crash groups listed, by signature, and one group's reports, by number.
    crash_groups: Vec<String>,
    crash_rows: Vec<i64>,
    /// The chat bans listed with Lift, by the banned player's key (empty: the ban holds
    /// only an address, which Lift cannot name).
    ban_rows: Vec<String>,
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    ui: MenuCanvas,
    query: String,
    note: String,
    /// The rank each medal is given at, chosen with its Rank button while the player does
    /// not hold it; back to I once given.
    give_ranks: [crate::medals::Rank; crate::medals::Medal::COUNT],
    /// The credits or shards Give and Take change, as typed (digits only).
    amount: String,
    /// The chosen player's key; the player's own until another is chosen.
    selected: Option<String>,
    /// What Give gives: a holocron tier or a kyber crystal tier, as
    /// `crate::holocrons::slot` counts them.
    holocron_tier: usize,
    /// The unlockables list's page, from 0.
    unlock_page: usize,
    /// The control the keyboard is on, by its token.
    focus: u16,
    /// The controls in the order Tab visits them, laid out by the last frame.
    order: Vec<u16>,
    /// The button pressed once (Clear all, an Unlink) waiting for its second press,
    /// and when.
    confirm: Option<(u16, Instant)>,
    /// The tab shown.
    view: View,
    /// The merge field: the key id of the player to merge away.
    merge_from: String,
    /// The next player chosen in the list fills the merge field instead.
    picking: bool,
    /// A merge waiting for Merge for good or Cancel.
    pending_merge: Option<PendingMerge>,
    /// The crash group whose reports the Crashes view lists, or none for the groups.
    crash_group: Option<String>,
    /// Where the list of players looks.
    scope: Scope,
    /// The page of players shown, from 0.
    player_page: usize,
    /// The saber shaders' filter, as typed, and whether only those held show.
    unlock_filter: String,
    unlock_held: bool,
    /// A search sent: when the hub's answer (a later [`StaffState::searches`] than this)
    /// finds exactly one player, they are chosen.
    awaiting_search: Option<u64>,
    /// [`StaffState::searches`] as the last frame read it.
    searches_seen: u64,
    shown: Shown,
    epoch: Instant,
    /// What a world shot shows in place of the live profile and answers.
    #[cfg(test)]
    pub(crate) preview: Option<(Profile, StaffState)>,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

/// Add `text` to `field` up to `limit` characters, control characters left out;
/// whether it changed.
fn type_into(field: &mut String, text: &str, limit: usize) -> bool {
    let mut changed = false;
    for c in text.chars().filter(|c| !c.is_control()) {
        if field.chars().count() >= limit {
            break;
        }
        field.push(c);
        changed = true;
    }
    changed
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            ui: MenuCanvas::with_capacities(260, 160, 1_200),
            query: String::new(),
            note: String::new(),
            give_ranks: [crate::medals::Rank::Issued; crate::medals::Medal::COUNT],
            amount: String::new(),
            selected: None,
            holocron_tier: 0,
            unlock_page: 0,
            focus: SEARCH_TOKEN,
            order: Vec::with_capacity(64),
            confirm: None,
            view: View::Overview,
            merge_from: String::new(),
            picking: false,
            pending_merge: None,
            crash_group: None,
            scope: Scope::Everyone,
            player_page: 0,
            unlock_filter: String::new(),
            unlock_held: false,
            awaiting_search: None,
            searches_seen: 0,
            shown: Shown::default(),
            epoch: Instant::now(),
            #[cfg(test)]
            preview: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page; `owns_console` when the console was closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.focus = SEARCH_TOKEN;
        self.confirm = None;
        self.picking = false;
        self.pending_merge = None;
        self.shown.gifts_read = false;
    }

    /// The search the page sends as it opens: `query` among everyone (a name or a key id;
    /// empty for the players seen lately). A search that finds one player chooses them.
    pub(crate) fn opening_search(&mut self, query: &str) -> StaffRequest {
        self.scope = Scope::Everyone;
        self.query.clear();
        type_into(&mut self.query, query, QUERY_MAX);
        self.search()
    }

    /// The search the list's scope and the search pill ask for, from the first page.
    fn search(&mut self) -> StaffRequest {
        self.player_page = 0;
        self.awaiting_search = Some(self.searches_seen);
        let query = self.query.trim().to_owned();
        let scope = match self.scope {
            Scope::Everyone => None,
            Scope::Here => match self.shown.server.clone() {
                Some(server) => Some(SearchScope::Server(server)),
                // No longer on a server the hub knows: everyone.
                None => {
                    self.scope = Scope::Everyone;
                    None
                }
            },
            Scope::Online => Some(SearchScope::Online),
            Scope::Team => Some(SearchScope::Staff),
        };
        match scope {
            Some(scope) => StaffRequest::SearchIn { query, scope },
            None => StaffRequest::Search(query),
        }
    }

    /// A scope chip: its scope (another press goes back to everyone), and the search.
    fn choose_scope(&mut self, scope: Scope) -> PanelAction {
        self.scope = if self.scope == scope {
            Scope::Everyone
        } else {
            scope
        };
        PanelAction::Request(self.search())
    }

    /// How many pages of players there are.
    fn player_pages(&self) -> usize {
        self.shown.players_found.div_ceil(PLAYERS_SHOWN).max(1)
    }

    /// Closing the page closes the console too.
    pub(crate) fn own_console(&mut self) {
        self.owns_console = true;
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// The chosen player's profile: from the players found, or the player's own.
    fn target<'a>(&self, inputs: &Inputs<'a>) -> Option<&'a Profile> {
        let me = inputs.me;
        let key = self
            .selected
            .as_deref()
            .or(me.map(|me| me.key_id.as_str()))?;
        inputs
            .staff
            .players
            .iter()
            .find(|player| player.key_id == key)
            .or(me.filter(|me| me.key_id == key))
    }

    /// The amount field's number, when it is one above 0.
    fn amount(&self) -> Option<i64> {
        self.amount.parse::<i64>().ok().filter(|amount| *amount > 0)
    }

    /// Whether the button `token` is waiting for its second press.
    fn confirming(&self, token: u16) -> bool {
        self.confirm
            .is_some_and(|(pressed, at)| pressed == token && at.elapsed() < CONFIRM_FOR)
    }

    /// A press of a button that needs two: whether this one is the second.
    fn second_press(&mut self, token: u16) -> bool {
        if self.confirming(token) {
            self.confirm = None;
            true
        } else {
            self.confirm = Some((token, Instant::now()));
            false
        }
    }

    /// Why `from`, the merge field's key, cannot be merged into `target` yet, knowing
    /// the players found; `None` when it can.
    fn merge_problem(
        from: &str,
        target: Option<&Profile>,
        players: &[Profile],
    ) -> Option<MergeProblem> {
        if from.is_empty() {
            return Some(MergeProblem::Empty);
        }
        if from.len() != 16 || !from.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Some(MergeProblem::NotKey);
        }
        if target.is_some_and(|target| target.has_key(from)) {
            return Some(MergeProblem::Same);
        }
        if players
            .iter()
            .any(|player| player.has_key(from) && player.staff)
        {
            return Some(MergeProblem::Staff);
        }
        None
    }

    /// What Enter (or a click) on `token` does.
    fn activate(&mut self, token: u16) -> PanelAction {
        if owner_only(token) && !self.shown.creator {
            return PanelAction::None;
        }
        if gift(token) && self.shown.gift_block.is_some() {
            return PanelAction::None;
        }
        self.focus = token;
        if self.confirm.is_some_and(|(pressed, _)| pressed != token) {
            self.confirm = None;
        }
        if token != MERGE_PICK_TOKEN
            && !(PLAYER_BASE..PLAYER_BASE + PLAYERS_SHOWN as u16).contains(&token)
        {
            self.picking = false;
        }
        let target = self.shown.target.clone();
        match token {
            SEARCH_TOKEN => PanelAction::Request(self.search()),
            HERE_TOKEN if self.shown.server.is_some() => self.choose_scope(Scope::Here),
            HERE_TOKEN => PanelAction::None,
            ONLINE_TOKEN => self.choose_scope(Scope::Online),
            TEAM_TOKEN => self.choose_scope(Scope::Team),
            PLAYERS_PREV_TOKEN => {
                self.player_page = self.player_page.saturating_sub(1);
                PanelAction::None
            }
            PLAYERS_NEXT_TOKEN => {
                self.player_page = (self.player_page + 1).min(self.player_pages() - 1);
                PanelAction::None
            }
            UNLOCK_FILTER_TOKEN => PanelAction::None,
            UNLOCK_HELD_TOKEN => {
                self.unlock_held = !self.unlock_held;
                self.unlock_page = 0;
                PanelAction::None
            }
            GIFTS_TAB_TOKEN | GIFTS_REFRESH_TOKEN => {
                self.view = View::Gifts;
                self.shown.gifts_read = true;
                PanelAction::Query(StaffQuery::Gifts)
            }
            token if (GIFT_UNDO_BASE..GIFT_UNDO_BASE + GIFT_ROWS_SHOWN as u16).contains(&token) => {
                match self
                    .shown
                    .gift_rows
                    .get(usize::from(token - GIFT_UNDO_BASE))
                    .cloned()
                {
                    Some((id, key_id)) if self.second_press(token) => {
                        PanelAction::Request(StaffRequest::GiftUndo { id, key_id })
                    }
                    _ => PanelAction::None,
                }
            }
            ME_TOKEN => {
                self.selected.clone_from(&self.shown.me);
                PanelAction::None
            }
            VERIFY_TOKEN => match target {
                Some(key_id) => PanelAction::Request(StaffRequest::Verify {
                    key_id,
                    verified: !self.shown.verified,
                    tick: None,
                }),
                None => PanelAction::None,
            },
            token
                if (TICK_BASE..TICK_BASE + sjk_identity::Tick::GROUPS.len() as u16)
                    .contains(&token) =>
            {
                match target {
                    Some(key_id) => PanelAction::Request(StaffRequest::Verify {
                        key_id,
                        verified: true,
                        tick: Some(sjk_identity::Tick::GROUPS[usize::from(token - TICK_BASE)]),
                    }),
                    None => PanelAction::None,
                }
            }
            SET_STAFF_TOKEN => match target {
                Some(key_id) if self.shown.creator && self.second_press(SET_STAFF_TOKEN) => {
                    PanelAction::Request(StaffRequest::SetStaff {
                        key_id,
                        staff: !self.shown.staff,
                    })
                }
                _ => PanelAction::None,
            },
            OVERVIEW_TAB_TOKEN => {
                self.view = View::Overview;
                PanelAction::None
            }
            // The tabs that give read how many gifts are left today, once, for staff who
            // have a limit (the creator has none).
            REWARDS_TAB_TOKEN | ECONOMY_TAB_TOKEN => {
                self.view = if token == REWARDS_TAB_TOKEN {
                    View::Rewards
                } else {
                    View::Economy
                };
                if self.shown.creator || self.shown.gifts_read {
                    PanelAction::None
                } else {
                    self.shown.gifts_read = true;
                    PanelAction::Query(StaffQuery::Gifts)
                }
            }
            CHAT_BANS_TAB_TOKEN | CHAT_BANS_REFRESH_TOKEN => {
                self.view = View::ChatBans;
                PanelAction::Query(StaffQuery::ChatBans)
            }
            token if (BAN_LIFT_BASE..BAN_LIFT_BASE + BAN_ROWS_SHOWN as u16).contains(&token) => {
                match self.shown.ban_rows.get(usize::from(token - BAN_LIFT_BASE)) {
                    Some(key_id) if !key_id.is_empty() => {
                        PanelAction::Request(StaffRequest::ChatUnban {
                            key_id: key_id.clone(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            AVATAR_BLOCK_TOKEN | AVATAR_ALLOW_TOKEN => match target {
                Some(key_id) => PanelAction::Request(StaffRequest::AvatarBlock {
                    key_id,
                    blocked: token == AVATAR_BLOCK_TOKEN,
                }),
                None => PanelAction::None,
            },
            HUB_MUTE_TOKEN | HUB_UNMUTE_TOKEN => match target {
                Some(key_id) => PanelAction::Request(StaffRequest::ChatMute {
                    key_id,
                    muted: token == HUB_MUTE_TOKEN,
                }),
                None => PanelAction::None,
            },
            CHAT_UNBAN_TOKEN => match target {
                Some(key_id) => PanelAction::Request(StaffRequest::ChatUnban { key_id }),
                None => PanelAction::None,
            },
            AMOUNT_TOKEN => PanelAction::None,
            CREDITS_GIVE_TOKEN | CREDITS_TAKE_TOKEN | SHARDS_GIVE_TOKEN | SHARDS_TAKE_TOKEN => {
                let (Some(key_id), Some(amount)) = (target, self.amount()) else {
                    return PanelAction::None;
                };
                let take = matches!(token, CREDITS_TAKE_TOKEN | SHARDS_TAKE_TOKEN);
                // Taking away waits for a second press.
                if take && !self.second_press(token) {
                    return PanelAction::None;
                }
                let delta = if take { -amount } else { amount };
                let note = self.note.trim().to_owned();
                PanelAction::Request(
                    if matches!(token, CREDITS_GIVE_TOKEN | CREDITS_TAKE_TOKEN) {
                        StaffRequest::Credits {
                            key_id,
                            delta,
                            note,
                        }
                    } else {
                        StaffRequest::Shards {
                            key_id,
                            delta,
                            note,
                        }
                    },
                )
            }
            ACHIEVEMENTS_TAB_TOKEN => {
                self.view = View::Achievements;
                PanelAction::None
            }
            KEYS_TAB_TOKEN => {
                self.view = View::Keys;
                PanelAction::None
            }
            // The Logs and Crashes views read their lists from the hub as they open.
            LOGS_TAB_TOKEN | LOGS_REFRESH_TOKEN => {
                self.view = View::Logs;
                PanelAction::Query(StaffQuery::LogRequests)
            }
            CRASHES_TAB_TOKEN | CRASHES_REFRESH_TOKEN | CRASHES_BACK_TOKEN => {
                self.view = View::Crashes;
                self.crash_group = None;
                PanelAction::Query(StaffQuery::CrashGroups)
            }
            REQUEST_LOGS_TOKEN => match target {
                Some(key_id) => PanelAction::Query(StaffQuery::RequestLogs {
                    key_id,
                    note: self.note.trim().to_owned(),
                }),
                None => PanelAction::None,
            },
            token if (LOG_OPEN_BASE..LOG_OPEN_BASE + LOG_ROWS_SHOWN as u16).contains(&token) => {
                match self.shown.log_rows.get(usize::from(token - LOG_OPEN_BASE)) {
                    Some(&(id, true)) => PanelAction::Query(StaffQuery::Log(id)),
                    _ => PanelAction::None,
                }
            }
            token
                if (CRASH_GROUP_BASE..CRASH_GROUP_BASE + CRASH_GROUPS_SHOWN as u16)
                    .contains(&token) =>
            {
                match self
                    .shown
                    .crash_groups
                    .get(usize::from(token - CRASH_GROUP_BASE))
                {
                    Some(signature) => {
                        self.crash_group = Some(signature.clone());
                        self.focus = CRASHES_BACK_TOKEN;
                        PanelAction::Query(StaffQuery::Crashes(signature.clone()))
                    }
                    None => PanelAction::None,
                }
            }
            token
                if (CRASH_OPEN_BASE..CRASH_OPEN_BASE + CRASH_ROWS_SHOWN as u16)
                    .contains(&token) =>
            {
                match self
                    .shown
                    .crash_rows
                    .get(usize::from(token - CRASH_OPEN_BASE))
                {
                    Some(&id) => PanelAction::Query(StaffQuery::Crash(id)),
                    None => PanelAction::None,
                }
            }
            MERGE_FIELD_TOKEN => PanelAction::None,
            MERGE_PICK_TOKEN => {
                self.picking = !self.picking;
                PanelAction::None
            }
            MERGE_TOKEN => {
                if let Some(kept) = target
                    && self.shown.merge_problem.is_none()
                {
                    self.pending_merge = Some(PendingMerge {
                        kept,
                        from: merge_key(&self.merge_from),
                    });
                    // Cancel first, so a hurried Enter does not merge.
                    self.focus = MERGE_CANCEL_TOKEN;
                }
                PanelAction::None
            }
            MERGE_CANCEL_TOKEN => {
                self.pending_merge = None;
                self.focus = MERGE_TOKEN;
                PanelAction::None
            }
            MERGE_CONFIRM_TOKEN => match self.pending_merge.take() {
                // Only the merge confirmed for the player still chosen.
                Some(merge) if target.as_deref() == Some(merge.kept.as_str()) => {
                    self.merge_from.clear();
                    self.focus = MERGE_FIELD_TOKEN;
                    PanelAction::Request(StaffRequest::Merge {
                        key_id: merge.kept,
                        from: merge.from,
                    })
                }
                _ => PanelAction::None,
            },
            token if (UNLINK_BASE..UNLINK_BASE + LINKED_SHOWN as u16).contains(&token) => {
                let Some(key_id) = self
                    .shown
                    .linked
                    .get(usize::from(token - UNLINK_BASE))
                    .cloned()
                else {
                    return PanelAction::None;
                };
                if self.second_press(token) {
                    PanelAction::Request(StaffRequest::Unlink { key_id })
                } else {
                    PanelAction::None
                }
            }
            RECENT_TOKEN => {
                self.query.clear();
                self.scope = Scope::Everyone;
                PanelAction::Request(self.search())
            }
            NOTE_TOKEN => PanelAction::None,
            PICTURE_DOWN_TOKEN => match target {
                Some(key_id) if self.shown.picture => {
                    PanelAction::Request(StaffRequest::AvatarRemove { key_id })
                }
                _ => PanelAction::None,
            },
            CLEAR_ALL_TOKEN => {
                let Some(key_id) = target else {
                    return PanelAction::None;
                };
                if self.second_press(CLEAR_ALL_TOKEN) {
                    PanelAction::Request(StaffRequest::ClearAchievements {
                        key_id,
                        id: String::new(),
                    })
                } else {
                    PanelAction::None
                }
            }
            token if (PLAYER_BASE..PLAYER_BASE + PLAYERS_SHOWN as u16).contains(&token) => {
                if let Some(key) = self.shown.players.get(usize::from(token - PLAYER_BASE)) {
                    if self.picking {
                        // Picking the player to merge away: the chosen one stays.
                        self.picking = false;
                        self.merge_from.clone_from(key);
                    } else {
                        self.selected = Some(key.clone());
                    }
                }
                PanelAction::None
            }
            token
                if (GIVE_BASE..GIVE_BASE + crate::medals::Medal::COUNT as u16).contains(&token) =>
            {
                let medal = crate::medals::Medal::ALL[usize::from(token - GIVE_BASE)];
                let held = self.shown.medals[medal.index()];
                match target {
                    Some(key_id) if held == 0 || medal.repeatable() => {
                        let rank = std::mem::take(&mut self.give_ranks[medal.index()]);
                        PanelAction::Request(StaffRequest::Award {
                            key_id,
                            medal: medal.id().to_owned(),
                            note: self.note.trim().to_owned(),
                            rank: rank.number(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            token
                if (RANK_BASE..RANK_BASE + crate::medals::Medal::COUNT as u16).contains(&token) =>
            {
                let medal = crate::medals::Medal::ALL[usize::from(token - RANK_BASE)];
                let rank = self.shown.ranks[medal.index()];
                match target {
                    Some(key_id) if self.shown.medals[medal.index()] > 0 => {
                        // Quiet, as every staff change: the player sees the promotion
                        // on their own pop-up, nobody else is told.
                        PanelAction::Request(StaffRequest::MedalRank {
                            key_id,
                            medal: medal.id().to_owned(),
                            rank: rank.next().unwrap_or_default().number(),
                        })
                    }
                    // Not held yet: the rank Give will give it at, nothing sent.
                    Some(_) => {
                        let give = &mut self.give_ranks[medal.index()];
                        *give = give.next().unwrap_or_default();
                        PanelAction::None
                    }
                    None => PanelAction::None,
                }
            }
            token
                if (TAKE_BASE..TAKE_BASE + crate::medals::Medal::COUNT as u16).contains(&token) =>
            {
                let medal = crate::medals::Medal::ALL[usize::from(token - TAKE_BASE)];
                match target {
                    Some(key_id) if self.shown.medals[medal.index()] > 0 => {
                        PanelAction::Request(StaffRequest::Unaward {
                            key_id,
                            medal: medal.id().to_owned(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            UNLOCK_PREV_TOKEN => {
                self.unlock_page = self.unlock_page.saturating_sub(1);
                PanelAction::None
            }
            UNLOCK_NEXT_TOKEN => {
                let per_page = self.shown.unlock_per_page.max(1);
                let last = crate::unlockables::ALL.len().div_ceil(per_page) - 1;
                self.unlock_page = (self.unlock_page + 1).min(last);
                PanelAction::None
            }
            token
                if (UNLOCK_BASE..UNLOCK_BASE + crate::unlockables::ALL.len() as u16)
                    .contains(&token) =>
            {
                let index = usize::from(token - UNLOCK_BASE);
                let allowed =
                    self.shown.creator || staff_may_unlock(&crate::unlockables::ALL[index]);
                match target {
                    Some(key_id)
                        if allowed && !self.shown.unlocks.get(index).copied().unwrap_or(false) =>
                    {
                        PanelAction::Request(StaffRequest::Unlock {
                            key_id,
                            unlock: crate::unlockables::ALL[index].id.to_owned(),
                            note: self.note.trim().to_owned(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            token
                if (RELOCK_BASE..RELOCK_BASE + crate::unlockables::ALL.len() as u16)
                    .contains(&token) =>
            {
                let index = usize::from(token - RELOCK_BASE);
                match target {
                    Some(key_id) if self.shown.unlocks.get(index).copied().unwrap_or(false) => {
                        PanelAction::Request(StaffRequest::Relock {
                            key_id,
                            unlock: crate::unlockables::ALL[index].id.to_owned(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            token
                if (HOLOCRON_TIER_BASE..HOLOCRON_TIER_BASE + crate::holocrons::SLOTS as u16)
                    .contains(&token) =>
            {
                self.holocron_tier = usize::from(token - HOLOCRON_TIER_BASE);
                PanelAction::None
            }
            HOLOCRON_GIVE_TOKEN => match target {
                Some(key_id) => PanelAction::Request(StaffRequest::HolocronGive {
                    key_id,
                    tier: crate::holocrons::slot(self.holocron_tier).id.to_owned(),
                    note: self.note.trim().to_owned(),
                }),
                None => PanelAction::None,
            },
            token
                if (HOLOCRON_REMOVE_BASE..HOLOCRON_REMOVE_BASE + HOLOCRONS_SHOWN as u16)
                    .contains(&token) =>
            {
                let row = usize::from(token - HOLOCRON_REMOVE_BASE);
                match target {
                    Some(key_id) if row < self.shown.holocron_rows && self.second_press(token) => {
                        PanelAction::Request(StaffRequest::HolocronRemove {
                            key_id,
                            id: self.shown.holocrons[row],
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            token
                if (CLEAR_BASE..CLEAR_BASE + crate::achievements::ALL.len() as u16)
                    .contains(&token) =>
            {
                let kind = &crate::achievements::ALL[usize::from(token - CLEAR_BASE)];
                match target {
                    Some(key_id) => PanelAction::Request(StaffRequest::ClearAchievements {
                        key_id,
                        id: kind.id.to_owned(),
                    }),
                    None => PanelAction::None,
                }
            }
            _ => PanelAction::None,
        }
    }

    /// Show `tab`, for a world shot.
    #[cfg(test)]
    pub(crate) fn tab_for_shot(&mut self, tab: ShotTab) {
        self.view = match tab {
            ShotTab::Overview => View::Overview,
            ShotTab::Rewards => View::Rewards,
            ShotTab::Economy => View::Economy,
            ShotTab::Achievements => View::Achievements,
            ShotTab::ChatBans => View::ChatBans,
            ShotTab::Gifts => View::Gifts,
        };
        if tab == ShotTab::Economy {
            self.amount = "50".to_owned();
            // A rare kyber crystal chosen, from the chips' second row.
            self.holocron_tier = crate::holocrons::COUNT + 1;
        }
    }

    /// Type `filter` in the saber shader filter, for a world shot.
    #[cfg(test)]
    pub(crate) fn shader_filter_for_shot(&mut self, filter: &str) {
        self.unlock_filter = filter.to_owned();
        self.unlock_page = 0;
    }

    /// Choose the player with `key_id`, for a world shot.
    #[cfg(test)]
    pub(crate) fn choose_for_shot(&mut self, key_id: &str) {
        self.selected = Some(key_id.to_owned());
    }

    /// Show the Keys and merge view with `from` in the merge field, and its confirmation
    /// for the chosen player when `confirm`, for a world shot.
    #[cfg(test)]
    pub(crate) fn merge_for_shot(&mut self, from: &str, confirm: bool) {
        self.view = View::Keys;
        self.merge_from = from.to_owned();
        self.pending_merge = None;
        if confirm && let Some(kept) = self.shown.target.clone() {
            self.pending_merge = Some(PendingMerge {
                kept,
                from: merge_key(from),
            });
            self.focus = MERGE_CANCEL_TOKEN;
        }
    }

    /// Show the Logs view, or the Crashes view (one kind's reports with `group`), for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn view_for_shot(&mut self, logs: bool, group: Option<&str>) {
        self.view = if logs { View::Logs } else { View::Crashes };
        self.crash_group = group.map(str::to_owned);
    }

    /// Move the keyboard `by` controls along the order the last frame laid out.
    fn step(&mut self, forward: bool) {
        if self.order.is_empty() {
            return;
        }
        let at = self.order.iter().position(|token| *token == self.focus);
        let next = match (at, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % self.order.len(),
            (Some(at), false) => (at + self.order.len() - 1) % self.order.len(),
        };
        self.focus = self.order[next];
    }

    /// A key, with Shift and Ctrl held or not.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        shift: bool,
        control: bool,
    ) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        let typing = matches!(
            self.focus,
            SEARCH_TOKEN | NOTE_TOKEN | MERGE_FIELD_TOKEN | AMOUNT_TOKEN | UNLOCK_FILTER_TOKEN
        );
        let in_list = (PLAYER_BASE..PLAYER_BASE + PLAYERS_SHOWN as u16).contains(&self.focus);
        if typing && !event.repeat {
            let field = match self.focus {
                SEARCH_TOKEN => &mut self.query,
                MERGE_FIELD_TOKEN => &mut self.merge_from,
                AMOUNT_TOKEN => &mut self.amount,
                UNLOCK_FILTER_TOKEN => &mut self.unlock_filter,
                _ => &mut self.note,
            };
            if crate::text_entry::before_edit(field, key, event.text.as_deref(), shift, control) {
                if self.focus == UNLOCK_FILTER_TOKEN {
                    self.unlock_page = 0;
                }
                return PanelAction::None;
            }
        }
        match key {
            // A merge being confirmed or picked is let go first.
            KeyCode::Escape if self.pending_merge.is_some() => {
                return self.activate(MERGE_CANCEL_TOKEN);
            }
            KeyCode::Escape if self.picking => self.picking = false,
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(!shift),
            KeyCode::ArrowDown | KeyCode::ArrowUp if in_list => {
                let count = self.shown.players.len();
                if count > 0 {
                    let at = usize::from(self.focus - PLAYER_BASE);
                    let next = if key == KeyCode::ArrowDown {
                        (at + 1) % count
                    } else {
                        (at + count - 1) % count
                    };
                    return self.activate(PLAYER_BASE + next as u16);
                }
            }
            KeyCode::ArrowDown if !typing => self.step(true),
            KeyCode::ArrowUp if !typing => self.step(false),
            KeyCode::Enter | KeyCode::NumpadEnter => return self.activate(self.focus),
            KeyCode::Space if !typing => return self.activate(self.focus),
            KeyCode::Backspace if typing => {
                let field = match self.focus {
                    SEARCH_TOKEN => &mut self.query,
                    MERGE_FIELD_TOKEN => &mut self.merge_from,
                    AMOUNT_TOKEN => &mut self.amount,
                    UNLOCK_FILTER_TOKEN => &mut self.unlock_filter,
                    _ => &mut self.note,
                };
                field.pop();
                if self.focus == UNLOCK_FILTER_TOKEN {
                    self.unlock_page = 0;
                }
            }
            _ if typing && !event.repeat => {
                let pasted;
                let text = match event.text.as_deref() {
                    // Ctrl+V (Windows hands the letter without Ctrl) or Shift+Insert.
                    text if crate::sjk_chat_field::is_paste(key, text, shift, control) => {
                        pasted = crate::console::clipboard::paste().unwrap_or_default();
                        pasted.as_str()
                    }
                    Some(text) => text,
                    None => return PanelAction::None,
                };
                match self.focus {
                    SEARCH_TOKEN => type_into(&mut self.query, text, QUERY_MAX),
                    MERGE_FIELD_TOKEN => type_into(&mut self.merge_from, text, MERGE_FIELD_MAX),
                    AMOUNT_TOKEN => {
                        let digits: String = text.chars().filter(char::is_ascii_digit).collect();
                        type_into(&mut self.amount, &digits, AMOUNT_MAX)
                    }
                    UNLOCK_FILTER_TOKEN => {
                        self.unlock_page = 0;
                        type_into(&mut self.unlock_filter, text, FILTER_MAX)
                    }
                    _ => type_into(&mut self.note, text, NOTE_MAX),
                };
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            // A click in a field puts the keyboard there; it searches on Enter.
            Some(
                token @ (SEARCH_TOKEN | NOTE_TOKEN | MERGE_FIELD_TOKEN | AMOUNT_TOKEN
                | UNLOCK_FILTER_TOKEN),
            ) => {
                self.focus = token;
                PanelAction::None
            }
            Some(token) => self.activate(token),
            None => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{Achievement, Medal};

    pub(super) fn profile(key_id: &str, name: &str) -> Profile {
        Profile {
            tag_cards: Vec::new(),
            credits: 0,
            cards: Vec::new(),
            keys: Vec::new(),
            key_id: key_id.to_owned(),
            key: String::new(),
            name: name.to_owned(),
            banner: "".into(),
            bio: String::new(),
            verified: false,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            jkr_creator: false,
            trust: None,
            emoji: String::new(),
            created: 1_791_250_000,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            avatar: String::new(),
            unlocks: Vec::new(),
            holocron_counts: sjk_identity::HolocronCounts::default(),
            holocrons: Vec::new(),
            shards: 0,
        }
    }

    /// The player's own: SJK's creator, who may use every button.
    fn me() -> Profile {
        Profile {
            staff: true,
            creator: true,
            medals: vec![Medal {
                id: "early_tester".into(),
                count: 1,
                awarded: 1,
                note: String::new(),
                rank: 1,
            }],
            achievements: vec![Achievement {
                id: "first_blood".into(),
                progress: 1,
                goal: 1,
                unlocked: 5,
                reward: None,
                name: None,
                description: None,
                group: None,
            }],
            ..profile("aaaaaaaaaaaaaaaa", "^1Sol")
        }
    }

    /// The panel after one frame with `players` found, so its controls are known.
    fn drawn(players: Vec<Profile>) -> (Panel, Profile, StaffState) {
        let me = me();
        let staff = StaffState {
            players,
            ..StaffState::default()
        };
        let mut panel = Panel::new();
        panel.open(true);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(&me),
                staff: &staff,
                server: None,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        (panel, me, staff)
    }

    #[test]
    fn logs_and_crashes_are_asked_for_and_opened() {
        use sjk_identity::logs::{CrashGroup, CrashSummary, LogRequestRow};
        let (mut panel, me, _) = drawn(Vec::new());
        // Opening the views reads their lists.
        assert_eq!(
            panel.activate(LOGS_TAB_TOKEN),
            PanelAction::Query(StaffQuery::LogRequests)
        );
        panel.note = " the freeze ".into();
        assert_eq!(
            panel.activate(REQUEST_LOGS_TOKEN),
            PanelAction::Query(StaffQuery::RequestLogs {
                key_id: me.key_id.clone(),
                note: "the freeze".into(),
            })
        );
        let row = |id, key_id: &str, state: &str| LogRequestRow {
            id,
            key_id: key_id.into(),
            state: state.into(),
            ..LogRequestRow::default()
        };
        let staff = StaffState {
            log_requests: vec![
                row(9, &me.key_id, "open"),
                row(8, "bbbbbbbbbbbbbbbb", "sent"),
            ],
            crash_groups: vec![CrashGroup {
                signature: "00ff00ff00ff00ff".into(),
                ..CrashGroup::default()
            }],
            crashes: vec![CrashSummary {
                id: 77,
                ..CrashSummary::default()
            }],
            crashes_of: "00ff00ff00ff00ff".into(),
            ..StaffState::default()
        };
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let inputs = Inputs {
            me: Some(&me),
            staff: &staff,
            server: None,
        };
        panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        // One request open for this player: no second one; only sent logs open.
        assert!(!panel.order.contains(&REQUEST_LOGS_TOKEN));
        assert!(!panel.order.contains(&LOG_OPEN_BASE));
        assert_eq!(
            panel.activate(LOG_OPEN_BASE + 1),
            PanelAction::Query(StaffQuery::Log(8))
        );
        assert_eq!(panel.activate(LOG_OPEN_BASE), PanelAction::None);
        // A kind of crash lists its reports, whose Open reads one whole.
        assert_eq!(
            panel.activate(CRASHES_TAB_TOKEN),
            PanelAction::Query(StaffQuery::CrashGroups)
        );
        panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            panel.activate(CRASH_GROUP_BASE),
            PanelAction::Query(StaffQuery::Crashes("00ff00ff00ff00ff".into()))
        );
        panel.build(&inputs, &fonts.font, [1920.0, 1080.0]);
        assert!(panel.order.contains(&CRASHES_BACK_TOKEN));
        assert_eq!(
            panel.activate(CRASH_OPEN_BASE),
            PanelAction::Query(StaffQuery::Crash(77))
        );
        assert_eq!(
            panel.activate(CRASHES_BACK_TOKEN),
            PanelAction::Query(StaffQuery::CrashGroups)
        );
        assert_eq!(panel.crash_group, None);
    }

    #[test]
    fn the_player_is_their_own_target_until_another_is_chosen() {
        let (mut panel, me, staff) = drawn(vec![profile("bbbbbbbbbbbbbbbb", "Fox")]);
        let inputs = Inputs {
            me: Some(&me),
            staff: &staff,
            server: None,
        };
        assert_eq!(panel.target(&inputs).unwrap().key_id, me.key_id);
        assert_eq!(panel.activate(PLAYER_BASE), PanelAction::None);
        assert_eq!(panel.target(&inputs).unwrap().name, "Fox");
        let _ = panel.activate(ME_TOKEN);
        assert_eq!(panel.target(&inputs).unwrap().key_id, me.key_id);
    }

    #[test]
    fn a_picture_is_taken_down_only_when_there_is_one() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert!(panel.order.contains(&PICTURE_DOWN_TOKEN));
        assert_eq!(panel.activate(PICTURE_DOWN_TOKEN), PanelAction::None);
        let pictured = Profile {
            avatar: "0123456789abcdef".into(),
            ..profile("bbbbbbbbbbbbbbbb", "Fox")
        };
        let (mut panel, _, _) = drawn(vec![pictured]);
        assert_eq!(panel.activate(PLAYER_BASE), PanelAction::None);
        // The next frame shows the chosen player.
        let staff = StaffState {
            players: vec![Profile {
                avatar: "0123456789abcdef".into(),
                ..profile("bbbbbbbbbbbbbbbb", "Fox")
            }],
            ..StaffState::default()
        };
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(&me),
                staff: &staff,
                server: None,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(
            panel.activate(PICTURE_DOWN_TOKEN),
            PanelAction::Request(StaffRequest::AvatarRemove {
                key_id: "bbbbbbbbbbbbbbbb".into()
            })
        );
    }

    #[test]
    fn medals_are_given_and_taken_back_as_the_catalogue_allows() {
        let (mut panel, me, _) = drawn(Vec::new());
        panel.note = "  found the fog bug ".into();
        // Tester is held and not repeatable: no second one.
        assert_eq!(panel.activate(GIVE_BASE), PanelAction::None);
        assert_eq!(
            panel.activate(TAKE_BASE),
            PanelAction::Request(StaffRequest::Unaward {
                key_id: me.key_id.clone(),
                medal: "early_tester".into()
            })
        );
        assert_eq!(
            panel.activate(GIVE_BASE + 2),
            PanelAction::Request(StaffRequest::Award {
                key_id: me.key_id.clone(),
                medal: "bug_hunter".into(),
                note: "found the fog bug".into(),
                rank: 1,
            })
        );
        // Not held: nothing to take back.
        assert_eq!(panel.activate(TAKE_BASE + 2), PanelAction::None);
    }

    #[test]
    fn a_held_medals_rank_steps_up_and_round() {
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(REWARDS_TAB_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        assert!(panel.order.contains(&RANK_BASE), "Tester is held");
        assert!(
            panel.order.contains(&(RANK_BASE + 2)),
            "Bug Hunter picks its rank"
        );
        assert_eq!(
            panel.activate(RANK_BASE),
            PanelAction::Request(StaffRequest::MedalRank {
                key_id: me.key_id.clone(),
                medal: "early_tester".into(),
                rank: 2,
            })
        );
        // Not held: the Rank button picks the rank Give uses (IV here), then back to I.
        for _ in 0..3 {
            assert_eq!(panel.activate(RANK_BASE + 2), PanelAction::None);
        }
        assert_eq!(
            panel.activate(GIVE_BASE + 2),
            PanelAction::Request(StaffRequest::Award {
                key_id: me.key_id.clone(),
                medal: "bug_hunter".into(),
                note: String::new(),
                rank: 4,
            })
        );
        assert_eq!(
            panel.activate(GIVE_BASE + 2),
            PanelAction::Request(StaffRequest::Award {
                key_id: me.key_id.clone(),
                medal: "bug_hunter".into(),
                note: String::new(),
                rank: 1,
            })
        );
        // At rank V the next press goes back to I.
        let mut solar = me.clone();
        solar.medals[0].rank = 5;
        redraw(&mut panel, &solar, Vec::new());
        assert_eq!(
            panel.activate(RANK_BASE),
            PanelAction::Request(StaffRequest::MedalRank {
                key_id: me.key_id.clone(),
                medal: "early_tester".into(),
                rank: 1,
            })
        );
        assert!(gift(RANK_BASE) && !owner_only(RANK_BASE));
    }

    #[test]
    fn unlockables_are_unlocked_and_relocked_as_held() {
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(REWARDS_TAB_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        assert!(panel.order.contains(&UNLOCK_BASE), "Unlock is offered");
        assert!(!panel.order.contains(&RELOCK_BASE), "nothing to relock");
        panel.note = " for testing ".into();
        assert_eq!(panel.activate(RELOCK_BASE), PanelAction::None);
        assert_eq!(
            panel.activate(UNLOCK_BASE),
            PanelAction::Request(StaffRequest::Unlock {
                key_id: me.key_id.clone(),
                unlock: "saber_sun".into(),
                note: "for testing".into()
            })
        );
        // Once the profile lists it: Relock, and no second Unlock.
        let held = Profile {
            unlocks: vec![sjk_identity::Unlock {
                holo: 0,
                solar: 0,
                id: "saber_sun".into(),
                granted: 1,
                note: String::new(),
                count: 1,
                medal: None,
                card: None,
            }],
            ..me.clone()
        };
        let staff = StaffState::default();
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(&held),
                staff: &staff,
                server: None,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(panel.order.contains(&RELOCK_BASE));
        assert!(!panel.order.contains(&UNLOCK_BASE));
        assert_eq!(panel.activate(UNLOCK_BASE), PanelAction::None);
        assert_eq!(
            panel.activate(RELOCK_BASE),
            PanelAction::Request(StaffRequest::Relock {
                key_id: me.key_id,
                unlock: "saber_sun".into()
            })
        );
    }

    fn wire_holocron(id: u64, tier: &str) -> sjk_identity::Holocron {
        sjk_identity::Holocron {
            opened: None,
            traded: None,
            traded_from_key: None,
            traded_from: None,
            edition: sjk_identity::wire::EDITION_FIRST.to_owned(),
            id,
            tier: tier.to_owned(),
            dropped: 1_791_641_100,
            source: "play".to_owned(),
            note: String::new(),
        }
    }

    #[test]
    fn a_holocron_is_given_in_the_chosen_tier_with_the_note() {
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        panel.note = "  for the fog bug ".into();
        // Uncommon is chosen to begin with.
        assert_eq!(
            panel.activate(HOLOCRON_GIVE_TOKEN),
            PanelAction::Request(StaffRequest::HolocronGive {
                key_id: me.key_id.clone(),
                tier: "uncommon".into(),
                note: "for the fog bug".into(),
            })
        );
        assert_eq!(
            panel.activate(HOLOCRON_TIER_BASE + 3),
            PanelAction::None,
            "choosing a tier sends nothing"
        );
        assert_eq!(
            panel.activate(HOLOCRON_GIVE_TOKEN),
            PanelAction::Request(StaffRequest::HolocronGive {
                key_id: me.key_id,
                tier: "mythical".into(),
                note: "for the fog bug".into(),
            })
        );
        assert!(panel.order.contains(&HOLOCRON_GIVE_TOKEN));
        for tier in 0..crate::holocrons::SLOTS as u16 {
            assert!(panel.order.contains(&(HOLOCRON_TIER_BASE + tier)));
        }
    }

    #[test]
    fn a_kyber_crystal_is_given_from_the_second_row_of_chips() {
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        for (slot, tier) in crate::kyber::TIERS.iter().enumerate() {
            let chip = HOLOCRON_TIER_BASE + (crate::holocrons::COUNT + slot) as u16;
            assert_eq!(panel.activate(chip), PanelAction::None);
            assert_eq!(
                panel.activate(HOLOCRON_GIVE_TOKEN),
                PanelAction::Request(StaffRequest::HolocronGive {
                    key_id: me.key_id.clone(),
                    tier: tier.id.into(),
                    note: String::new(),
                })
            );
        }
    }

    #[test]
    fn a_holocron_the_player_holds_is_taken_back_by_its_number() {
        let (mut panel, me, staff) = drawn(Vec::new());
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        // Nothing listed yet: nothing to remove.
        assert_eq!(panel.activate(HOLOCRON_REMOVE_BASE), PanelAction::None);
        assert!(!panel.order.contains(&HOLOCRON_REMOVE_BASE));
        let held = Profile {
            staff: true,
            creator: true,
            holocrons: vec![
                wire_holocron(42, "kyber_rare"),
                wire_holocron(41, "legendary"),
                wire_holocron(40, "from_the_future"),
                wire_holocron(39, "rare"),
            ],
            ..profile("aaaaaaaaaaaaaaaa", "^1Sol")
        };
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(&held),
                staff: &staff,
                server: None,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        // Rows are the known tiers, kyber crystals too, newest first; each Remove
        // waits for a second press.
        for (row, id) in [42, 41, 39].into_iter().enumerate() {
            let token = HOLOCRON_REMOVE_BASE + row as u16;
            assert_eq!(panel.activate(token), PanelAction::None, "first press");
            assert_eq!(
                panel.activate(token),
                PanelAction::Request(StaffRequest::HolocronRemove {
                    key_id: "aaaaaaaaaaaaaaaa".into(),
                    id
                })
            );
        }
        // A first press elsewhere lets go of the one waiting.
        let _ = panel.activate(HOLOCRON_REMOVE_BASE);
        let _ = panel.activate(HOLOCRON_REMOVE_BASE + 1);
        assert_eq!(panel.activate(HOLOCRON_REMOVE_BASE), PanelAction::None);
        assert_eq!(panel.activate(HOLOCRON_REMOVE_BASE + 3), PanelAction::None);
        assert!(panel.order.contains(&(HOLOCRON_REMOVE_BASE + 2)));
        assert!(!panel.order.contains(&(HOLOCRON_REMOVE_BASE + 3)));
    }

    #[test]
    fn clear_all_needs_a_second_press() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert_eq!(panel.activate(CLEAR_ALL_TOKEN), PanelAction::None);
        assert!(panel.confirming(CLEAR_ALL_TOKEN));
        assert_eq!(
            panel.activate(CLEAR_ALL_TOKEN),
            PanelAction::Request(StaffRequest::ClearAchievements {
                key_id: me.key_id.clone(),
                id: String::new()
            })
        );
        // Anything else in between starts again.
        let _ = panel.activate(CLEAR_ALL_TOKEN);
        let _ = panel.activate(ME_TOKEN);
        assert_eq!(panel.activate(CLEAR_ALL_TOKEN), PanelAction::None);
        assert_eq!(
            panel.activate(CLEAR_BASE),
            PanelAction::Request(StaffRequest::ClearAchievements {
                key_id: me.key_id,
                id: "first_blood".into()
            })
        );
    }

    /// `panel` laid out again with `players` found and `me` as the player's own.
    fn redraw(panel: &mut Panel, me: &Profile, players: Vec<Profile>) {
        let staff = StaffState {
            players,
            ..StaffState::default()
        };
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(me),
                staff: &staff,
                server: None,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
    }

    #[test]
    fn verify_shows_the_state_and_sends_the_other() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert!(panel.order.contains(&VERIFY_TOKEN));
        assert_eq!(
            panel.activate(VERIFY_TOKEN),
            PanelAction::Request(StaffRequest::Verify {
                key_id: me.key_id.clone(),
                verified: true,
                tick: None,
            })
        );
        let verified = Profile {
            verified: true,
            ..me.clone()
        };
        redraw(&mut panel, &verified, Vec::new());
        assert_eq!(
            panel.activate(VERIFY_TOKEN),
            PanelAction::Request(StaffRequest::Verify {
                key_id: me.key_id.clone(),
                verified: false,
                tick: None,
            })
        );
        // A tick button verifies with that group, or changes it.
        assert!(panel.order.contains(&(TICK_BASE + 2)));
        assert_eq!(
            panel.activate(TICK_BASE + 2),
            PanelAction::Request(StaffRequest::Verify {
                key_id: me.key_id,
                verified: true,
                tick: Some(sjk_identity::Tick::Sith),
            })
        );
    }

    #[test]
    fn only_the_creator_makes_staff_on_a_second_press_and_never_unstaffs_themselves() {
        let other = profile("bbbbbbbbbbbbbbbb", "Fox");
        let (mut panel, me, _) = drawn(vec![other.clone()]);
        let me = Profile {
            creator: false,
            ..me
        };
        let _ = panel.activate(PLAYER_BASE);
        redraw(&mut panel, &me, vec![other.clone()]);
        assert!(
            !panel.order.contains(&SET_STAFF_TOKEN),
            "staff do not get it"
        );
        assert_eq!(panel.activate(SET_STAFF_TOKEN), PanelAction::None);
        let creator = Profile {
            staff: true,
            creator: true,
            ..me.clone()
        };
        redraw(&mut panel, &creator, vec![other.clone()]);
        assert!(panel.order.contains(&SET_STAFF_TOKEN));
        assert_eq!(panel.activate(SET_STAFF_TOKEN), PanelAction::None);
        assert!(panel.confirming(SET_STAFF_TOKEN));
        assert_eq!(
            panel.activate(SET_STAFF_TOKEN),
            PanelAction::Request(StaffRequest::SetStaff {
                key_id: other.key_id.clone(),
                staff: true
            })
        );
        let staffed = Profile {
            staff: true,
            ..other.clone()
        };
        redraw(&mut panel, &creator, vec![staffed]);
        let _ = panel.activate(SET_STAFF_TOKEN);
        assert_eq!(
            panel.activate(SET_STAFF_TOKEN),
            PanelAction::Request(StaffRequest::SetStaff {
                key_id: other.key_id,
                staff: false
            })
        );
        // On their own player the creator has no such button.
        let _ = panel.activate(ME_TOKEN);
        redraw(&mut panel, &creator, Vec::new());
        assert!(!panel.order.contains(&SET_STAFF_TOKEN));
    }

    #[test]
    fn a_linked_key_is_unlinked_on_a_second_press_and_the_main_key_never() {
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(KEYS_TAB_TOKEN);
        let linked = Profile {
            keys: vec![me.key_id.clone(), "cccccccccccccccc".into()],
            ..me.clone()
        };
        redraw(&mut panel, &linked, Vec::new());
        assert!(panel.order.contains(&UNLINK_BASE));
        assert!(
            !panel.order.contains(&(UNLINK_BASE + 1)),
            "the main key has none"
        );
        assert_eq!(panel.activate(UNLINK_BASE), PanelAction::None);
        assert!(panel.confirming(UNLINK_BASE));
        assert_eq!(
            panel.activate(UNLINK_BASE),
            PanelAction::Request(StaffRequest::Unlink {
                key_id: "cccccccccccccccc".into()
            })
        );
        // Anything else in between starts again.
        let _ = panel.activate(UNLINK_BASE);
        let _ = panel.activate(KEYS_TAB_TOKEN);
        assert_eq!(panel.activate(UNLINK_BASE), PanelAction::None);
        assert_eq!(panel.activate(UNLINK_BASE + 1), PanelAction::None);
    }

    #[test]
    fn a_merge_keeps_the_chosen_player_and_asks_before_it_goes() {
        let reset = profile("bbbbbbbbbbbbbbbb", "Fox");
        let (mut panel, me, _) = drawn(vec![reset.clone()]);
        let _ = panel.activate(KEYS_TAB_TOKEN);
        let players = vec![reset.clone()];
        redraw(&mut panel, &me, players.clone());
        assert_eq!(panel.shown.merge_problem, Some(MergeProblem::Empty));
        assert!(!panel.order.contains(&MERGE_TOKEN), "nothing to merge yet");
        // Typed: checked as a key id, never the chosen player's own.
        for (typed, problem) in [
            ("bbbb", Some(MergeProblem::NotKey)),
            ("AAAAAAAAAAAAAAAA", Some(MergeProblem::Same)),
            (" BBBBBBBBBBBBBBBB ", None),
        ] {
            panel.merge_from = typed.into();
            redraw(&mut panel, &me, players.clone());
            assert_eq!(panel.shown.merge_problem, problem, "{typed}");
        }
        // Merge only asks; Merge for good sends, keeping the chosen player.
        assert_eq!(panel.activate(MERGE_TOKEN), PanelAction::None);
        assert_eq!(panel.focus, MERGE_CANCEL_TOKEN, "Cancel has the keyboard");
        redraw(&mut panel, &me, players.clone());
        assert!(panel.order.contains(&MERGE_CONFIRM_TOKEN));
        assert_eq!(
            panel.activate(MERGE_CONFIRM_TOKEN),
            PanelAction::Request(StaffRequest::Merge {
                key_id: me.key_id.clone(),
                from: "bbbbbbbbbbbbbbbb".into()
            })
        );
        assert!(panel.merge_from.is_empty());
        assert_eq!(panel.activate(MERGE_CONFIRM_TOKEN), PanelAction::None);
        // Picked in the list: the field fills, the chosen player stays.
        let _ = panel.activate(MERGE_PICK_TOKEN);
        assert!(panel.picking);
        assert_eq!(panel.activate(PLAYER_BASE), PanelAction::None);
        assert!(!panel.picking);
        assert_eq!(panel.merge_from, "bbbbbbbbbbbbbbbb");
        redraw(&mut panel, &me, players.clone());
        assert_eq!(panel.shown.target.as_deref(), Some(me.key_id.as_str()));
        // Choosing another player lets a merge waiting for confirmation go.
        let _ = panel.activate(MERGE_TOKEN);
        assert!(panel.pending_merge.is_some());
        panel.selected = Some("bbbbbbbbbbbbbbbb".into());
        redraw(&mut panel, &me, players.clone());
        assert!(panel.pending_merge.is_none());
        assert_eq!(panel.activate(MERGE_CONFIRM_TOKEN), PanelAction::None);
        // Escape cancels a waiting merge before it closes the page.
        panel.selected = None;
        redraw(&mut panel, &me, players);
        let _ = panel.activate(MERGE_TOKEN);
        assert!(panel.pending_merge.is_some());
        assert_eq!(panel.activate(MERGE_CANCEL_TOKEN), PanelAction::None);
        assert!(panel.pending_merge.is_none());
    }

    #[test]
    fn a_staff_player_is_never_offered_to_merge_away() {
        let other_staff = Profile {
            staff: true,
            keys: vec!["bbbbbbbbbbbbbbbb".into(), "cccccccccccccccc".into()],
            ..profile("bbbbbbbbbbbbbbbb", "Mod")
        };
        let (mut panel, me, _) = drawn(vec![other_staff.clone()]);
        let _ = panel.activate(KEYS_TAB_TOKEN);
        // By a linked key too.
        panel.merge_from = "cccccccccccccccc".into();
        redraw(&mut panel, &me, vec![other_staff]);
        assert_eq!(panel.shown.merge_problem, Some(MergeProblem::Staff));
        assert_eq!(panel.activate(MERGE_TOKEN), PanelAction::None);
        assert!(panel.pending_merge.is_none());
    }

    #[test]
    fn credits_and_shards_need_an_amount_and_carry_the_note() {
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        assert!(panel.order.contains(&AMOUNT_TOKEN));
        assert!(!panel.order.contains(&CREDITS_GIVE_TOKEN), "no amount yet");
        assert_eq!(panel.activate(CREDITS_GIVE_TOKEN), PanelAction::None);
        // The field takes digits only.
        panel.focus = AMOUNT_TOKEN;
        let _ = type_into(&mut panel.amount, "4x2", AMOUNT_MAX);
        assert_eq!(
            panel.amount, "4x2",
            "type_into itself keeps what it is given"
        );
        panel.amount.clear();
        let digits: String = "4x2".chars().filter(char::is_ascii_digit).collect();
        let _ = type_into(&mut panel.amount, &digits, AMOUNT_MAX);
        assert_eq!(panel.amount, "42");
        panel.note = " refund ".into();
        redraw(&mut panel, &me, Vec::new());
        for token in [
            CREDITS_GIVE_TOKEN,
            CREDITS_TAKE_TOKEN,
            SHARDS_GIVE_TOKEN,
            SHARDS_TAKE_TOKEN,
        ] {
            assert!(panel.order.contains(&token), "{token}");
        }
        assert_eq!(
            panel.activate(CREDITS_TAKE_TOKEN),
            PanelAction::None,
            "taking waits for a second press"
        );
        assert_eq!(
            panel.activate(CREDITS_TAKE_TOKEN),
            PanelAction::Request(StaffRequest::Credits {
                key_id: me.key_id.clone(),
                delta: -42,
                note: "refund".into(),
            })
        );
        assert_eq!(
            panel.activate(SHARDS_GIVE_TOKEN),
            PanelAction::Request(StaffRequest::Shards {
                key_id: me.key_id.clone(),
                delta: 42,
                note: "refund".into(),
            })
        );
        panel.amount = "0".into();
        assert_eq!(
            panel.activate(SHARDS_GIVE_TOKEN),
            PanelAction::None,
            "0 is no amount"
        );
    }

    #[test]
    fn the_overview_moderates_pictures_and_the_chat() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert_eq!(panel.view, View::Overview, "the page opens on the Overview");
        let key_id = me.key_id.clone();
        assert_eq!(
            panel.activate(AVATAR_BLOCK_TOKEN),
            PanelAction::Request(StaffRequest::AvatarBlock {
                key_id: key_id.clone(),
                blocked: true,
            })
        );
        assert_eq!(
            panel.activate(AVATAR_ALLOW_TOKEN),
            PanelAction::Request(StaffRequest::AvatarBlock {
                key_id: key_id.clone(),
                blocked: false,
            })
        );
        assert_eq!(
            panel.activate(HUB_MUTE_TOKEN),
            PanelAction::Request(StaffRequest::ChatMute {
                key_id: key_id.clone(),
                muted: true,
            })
        );
        assert_eq!(
            panel.activate(HUB_UNMUTE_TOKEN),
            PanelAction::Request(StaffRequest::ChatMute {
                key_id: key_id.clone(),
                muted: false,
            })
        );
        assert_eq!(
            panel.activate(CHAT_UNBAN_TOKEN),
            PanelAction::Request(StaffRequest::ChatUnban { key_id })
        );
    }

    #[test]
    fn chat_bans_are_listed_and_lifted_by_key() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert_eq!(
            panel.activate(CHAT_BANS_TAB_TOKEN),
            PanelAction::Query(StaffQuery::ChatBans)
        );
        let staff = StaffState {
            chat_bans: vec![
                sjk_identity::ChatBanRow {
                    key_id: "bbbbbbbbbbbbbbbb".into(),
                    name: "Fox".into(),
                    ..Default::default()
                },
                // Only an address: nothing Lift can name.
                sjk_identity::ChatBanRow::default(),
            ],
            ..StaffState::default()
        };
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(&me),
                staff: &staff,
                server: None,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(panel.order.contains(&BAN_LIFT_BASE));
        assert!(!panel.order.contains(&(BAN_LIFT_BASE + 1)));
        assert_eq!(
            panel.activate(BAN_LIFT_BASE),
            PanelAction::Request(StaffRequest::ChatUnban {
                key_id: "bbbbbbbbbbbbbbbb".into(),
            })
        );
        assert_eq!(panel.activate(BAN_LIFT_BASE + 1), PanelAction::None);
    }

    #[test]
    fn tab_walks_every_control_and_typing_goes_to_the_field() {
        let (mut panel, me, _) = drawn(vec![profile("bbbbbbbbbbbbbbbb", "Fox")]);
        let _ = panel.activate(ACHIEVEMENTS_TAB_TOKEN);
        redraw(&mut panel, &me, vec![profile("bbbbbbbbbbbbbbbb", "Fox")]);
        assert!(panel.order.contains(&SEARCH_TOKEN));
        assert!(panel.order.contains(&PLAYER_BASE));
        assert!(panel.order.contains(&NOTE_TOKEN));
        assert!(panel.order.contains(&CLEAR_BASE), "first_blood is listed");
        let start = panel.focus;
        for _ in 0..panel.order.len() {
            panel.step(true);
        }
        assert_eq!(panel.focus, start, "Tab comes round");
        assert!(type_into(&mut panel.query, "So\u{7}l", QUERY_MAX));
        assert_eq!(panel.query, "Sol");
        panel.focus = SEARCH_TOKEN;
        assert_eq!(
            panel.activate(SEARCH_TOKEN),
            PanelAction::Request(StaffRequest::Search("Sol".into()))
        );
        let mut long = String::new();
        type_into(&mut long, &"x".repeat(300), NOTE_MAX);
        assert_eq!(long.len(), NOTE_MAX);
    }

    /// `panel` laid out again with `staff` and `me`, on `server` when given.
    fn rebuild(panel: &mut Panel, me: &Profile, staff: &StaffState, server: Option<&str>) {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(me),
                staff,
                server: server.map(str::to_owned),
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
    }

    /// The player's own as SJK's creator ([`me`]), where a test's `me` is someone else.
    fn the_creator() -> Profile {
        me()
    }

    /// The player's own, staff but not the creator.
    fn helper() -> Profile {
        Profile {
            creator: false,
            ..me()
        }
    }

    #[test]
    fn staff_give_but_only_the_creator_takes_back_or_makes_credits() {
        let other = profile("bbbbbbbbbbbbbbbb", "Fox");
        let (mut panel, _, _) = drawn(vec![other.clone()]);
        let me = helper();
        let _ = panel.activate(PLAYER_BASE);
        panel.amount = "50".to_owned();
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &me, vec![other.clone()]);
        assert!(!panel.order.iter().any(|token| owner_only(*token)));
        assert!(panel.order.contains(&HOLOCRON_GIVE_TOKEN));
        assert_eq!(panel.activate(CREDITS_GIVE_TOKEN), PanelAction::None);
        assert_eq!(
            panel.activate(HOLOCRON_GIVE_TOKEN),
            PanelAction::Request(StaffRequest::HolocronGive {
                key_id: other.key_id.clone(),
                tier: "uncommon".into(),
                note: String::new(),
            })
        );
        assert_eq!(
            panel.activate(GIVE_BASE),
            PanelAction::Request(StaffRequest::Award {
                key_id: other.key_id.clone(),
                medal: "early_tester".into(),
                note: String::new(),
                rank: 1,
            })
        );
        // The day-to-day moderation stays theirs; taking back does not.
        assert_eq!(
            panel.activate(HUB_MUTE_TOKEN),
            PanelAction::Request(StaffRequest::ChatMute {
                key_id: other.key_id.clone(),
                muted: true
            })
        );
        assert!(owner_only(TAKE_BASE) && owner_only(RELOCK_BASE));
        assert!(owner_only(HOLOCRON_REMOVE_BASE) && !owner_only(HOLOCRON_GIVE_TOKEN));
        let creator = the_creator();
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &creator, vec![other.clone()]);
        assert_eq!(
            panel.activate(CREDITS_GIVE_TOKEN),
            PanelAction::Request(StaffRequest::Credits {
                key_id: other.key_id,
                delta: 50,
                note: String::new()
            })
        );
    }

    #[test]
    fn the_tabs_that_give_read_the_days_gifts_once_for_staff() {
        let (mut panel, _, _) = drawn(Vec::new());
        let me = helper();
        redraw(&mut panel, &me, Vec::new());
        assert_eq!(
            panel.activate(REWARDS_TAB_TOKEN),
            PanelAction::Query(StaffQuery::Gifts)
        );
        assert_eq!(panel.activate(ECONOMY_TAB_TOKEN), PanelAction::None);
        // Again after the page opens again.
        panel.open(true);
        assert_eq!(
            panel.activate(ECONOMY_TAB_TOKEN),
            PanelAction::Query(StaffQuery::Gifts)
        );
        // The creator has no limit to read.
        let (mut panel, _, _) = drawn(Vec::new());
        assert_eq!(panel.activate(REWARDS_TAB_TOKEN), PanelAction::None);
    }

    #[test]
    fn staff_never_give_to_themselves_nor_past_the_days_gifts() {
        let other = profile("bbbbbbbbbbbbbbbb", "Fox");
        let (mut panel, _, _) = drawn(vec![other.clone()]);
        let me = helper();
        // Their own player, by their id or a linked key: nothing to give.
        let _ = panel.activate(REWARDS_TAB_TOKEN);
        redraw(&mut panel, &me, vec![other.clone()]);
        assert_eq!(
            panel.shown.gift_block,
            Some("Staff cannot give to themselves")
        );
        assert!(!panel.order.iter().any(|token| gift(*token)));
        assert_eq!(panel.activate(GIVE_BASE + 2), PanelAction::None);
        assert_eq!(panel.activate(UNLOCK_BASE), PanelAction::None);
        // Another player, but no gift left today.
        let _ = panel.activate(PLAYER_BASE);
        let spent = StaffState {
            players: vec![other.clone()],
            gifts: Some(sjk_identity::GiftLog {
                gifts: Vec::new(),
                per_day: Some(20),
                left: Some(0),
            }),
            ..StaffState::default()
        };
        rebuild(&mut panel, &me, &spent, None);
        assert!(panel.shown.gift_block.is_some());
        assert_eq!(panel.activate(GIVE_BASE + 2), PanelAction::None);
        // Gifts left: given.
        let mut left = spent.clone();
        left.gifts.as_mut().unwrap().left = Some(5);
        rebuild(&mut panel, &me, &left, None);
        assert_eq!(panel.shown.gift_block, None);
        assert!(matches!(
            panel.activate(GIVE_BASE + 2),
            PanelAction::Request(StaffRequest::Award { .. })
        ));
        // The creator gives to themselves, without a limit.
        let _ = panel.activate(ME_TOKEN);
        rebuild(&mut panel, &the_creator(), &spent, None);
        assert_eq!(panel.shown.gift_block, None);
    }

    #[test]
    fn staff_unlock_only_the_shaders_holocrons_drop() {
        let other = profile("bbbbbbbbbbbbbbbb", "Fox");
        let (mut panel, _, _) = drawn(vec![other.clone()]);
        let me = helper();
        let _ = panel.activate(PLAYER_BASE);
        let _ = panel.activate(REWARDS_TAB_TOKEN);
        let kept = crate::unlockables::ALL
            .iter()
            .position(|unlockable| !unlockable.droppable)
            .expect("a shader holocrons do not drop");
        // Found with the filter, by its number, so it is on the page.
        panel.unlock_filter = crate::unlockables::ALL[kept].number_text();
        redraw(&mut panel, &me, vec![other.clone()]);
        let token = UNLOCK_BASE + kept as u16;
        assert!(!panel.order.contains(&token), "dimmed for staff");
        assert_eq!(panel.activate(token), PanelAction::None);
        // The creator gives it.
        redraw(&mut panel, &the_creator(), vec![other.clone()]);
        assert!(panel.order.contains(&token));
        assert!(matches!(
            panel.activate(token),
            PanelAction::Request(StaffRequest::Unlock { .. })
        ));
        // A shader holocrons drop is staff's to give.
        panel.unlock_filter = "sun blade".into();
        redraw(&mut panel, &me, vec![other.clone()]);
        assert!(crate::unlockables::ALL[0].droppable);
        assert_eq!(
            panel.activate(UNLOCK_BASE),
            PanelAction::Request(StaffRequest::Unlock {
                key_id: other.key_id,
                unlock: "saber_sun".into(),
                note: String::new(),
            })
        );
    }

    #[test]
    fn the_shader_filter_finds_by_name_number_and_tier_and_held_only() {
        let sun = &crate::unlockables::ALL[0];
        for filter in [
            "",
            "sun",
            "SUN BLADE",
            "1",
            "001",
            "#1",
            "mythical",
            "saber_sun",
        ] {
            assert!(filter_matches(sun, filter), "{filter}");
        }
        for filter in ["2", "moon", "rare", "^1"] {
            assert!(!filter_matches(sun, filter), "{filter}");
        }
        let (mut panel, me, _) = drawn(Vec::new());
        let _ = panel.activate(REWARDS_TAB_TOKEN);
        panel.unlock_filter = "mythical".into();
        redraw(&mut panel, &me, Vec::new());
        assert!(!panel.shown.unlock_rows.is_empty());
        assert!(panel.shown.unlock_rows.iter().all(|index| {
            crate::unlockables::ALL[*index].tier == crate::unlockables::Rarity::Mythical
        }));
        // Held only: the player holds none.
        panel.unlock_filter.clear();
        let _ = panel.activate(UNLOCK_HELD_TOKEN);
        redraw(&mut panel, &me, Vec::new());
        assert!(panel.shown.unlock_rows.is_empty());
        assert!(
            !panel
                .order
                .iter()
                .any(|token| (UNLOCK_BASE..UNLOCK_BASE + 299).contains(token))
        );
    }

    #[test]
    fn scope_chips_search_within_and_a_second_press_goes_back() {
        let (mut panel, me, staff) = drawn(Vec::new());
        // Not on a server the hub knows: no Here.
        assert!(!panel.order.contains(&HERE_TOKEN));
        assert_eq!(panel.activate(HERE_TOKEN), PanelAction::None);
        rebuild(&mut panel, &me, &staff, Some("1.2.3.4:29070"));
        assert!(panel.order.contains(&HERE_TOKEN));
        assert_eq!(
            panel.activate(HERE_TOKEN),
            PanelAction::Request(StaffRequest::SearchIn {
                query: String::new(),
                scope: SearchScope::Server("1.2.3.4:29070".into()),
            })
        );
        panel.query = " fox ".into();
        assert_eq!(
            panel.activate(ONLINE_TOKEN),
            PanelAction::Request(StaffRequest::SearchIn {
                query: "fox".into(),
                scope: SearchScope::Online,
            })
        );
        // The search pill searches within the scope chosen.
        assert_eq!(
            panel.activate(SEARCH_TOKEN),
            PanelAction::Request(StaffRequest::SearchIn {
                query: "fox".into(),
                scope: SearchScope::Online,
            })
        );
        // Another press on the lit chip: everyone again.
        assert_eq!(
            panel.activate(ONLINE_TOKEN),
            PanelAction::Request(StaffRequest::Search("fox".into()))
        );
        assert_eq!(
            panel.activate(TEAM_TOKEN),
            PanelAction::Request(StaffRequest::SearchIn {
                query: "fox".into(),
                scope: SearchScope::Staff,
            })
        );
        // Seen lately clears the search and the scope.
        assert_eq!(
            panel.activate(RECENT_TOKEN),
            PanelAction::Request(StaffRequest::Search(String::new()))
        );
        assert_eq!(panel.scope, Scope::Everyone);
    }

    #[test]
    fn a_search_that_finds_one_player_chooses_them() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert_eq!(
            panel.opening_search("fox"),
            StaffRequest::Search("fox".into())
        );
        assert_eq!(panel.query, "fox");
        let fox = profile("bbbbbbbbbbbbbbbb", "Fox");
        // Still on its way: nothing chosen yet.
        let waiting = StaffState {
            busy: true,
            ..StaffState::default()
        };
        rebuild(&mut panel, &me, &waiting, None);
        assert_eq!(panel.selected, None);
        let answered = StaffState {
            players: vec![fox.clone()],
            searches: 1,
            ..StaffState::default()
        };
        rebuild(&mut panel, &me, &answered, None);
        assert_eq!(panel.selected.as_deref(), Some(fox.key_id.as_str()));
        // Only once: choosing someone else afterwards sticks.
        let _ = panel.activate(ME_TOKEN);
        rebuild(&mut panel, &me, &answered, None);
        assert_eq!(panel.selected.as_deref(), Some(me.key_id.as_str()));
        // Two found: nobody is chosen for the staff member.
        let _ = panel.activate(SEARCH_TOKEN);
        let two = StaffState {
            players: vec![fox, profile("cccccccccccccccc", "Foxy")],
            searches: 2,
            ..StaffState::default()
        };
        rebuild(&mut panel, &me, &two, None);
        assert_eq!(panel.selected.as_deref(), Some(me.key_id.as_str()));
    }

    #[test]
    fn thirty_players_found_show_fifteen_a_page() {
        let players: Vec<Profile> = (0..30)
            .map(|index| profile(&format!("{index:016x}"), &format!("Player {index}")))
            .collect();
        let (mut panel, me, _) = drawn(players.clone());
        assert_eq!(panel.shown.players.len(), PLAYERS_SHOWN);
        assert!(panel.order.contains(&PLAYERS_NEXT_TOKEN));
        assert!(!panel.order.contains(&PLAYERS_PREV_TOKEN));
        let _ = panel.activate(PLAYERS_NEXT_TOKEN);
        redraw(&mut panel, &me, players.clone());
        assert_eq!(panel.shown.players[0], players[15].key_id);
        assert_eq!(panel.activate(PLAYER_BASE), PanelAction::None);
        assert_eq!(panel.selected.as_deref(), Some(players[15].key_id.as_str()));
        let _ = panel.activate(PLAYERS_NEXT_TOKEN);
        assert_eq!(panel.player_page, 1, "stops at the last page");
        let _ = panel.activate(PLAYERS_PREV_TOKEN);
        redraw(&mut panel, &me, players.clone());
        assert_eq!(panel.shown.players[0], players[0].key_id);
        // A new search starts from the first page.
        let _ = panel.activate(PLAYERS_NEXT_TOKEN);
        let _ = panel.activate(SEARCH_TOKEN);
        assert_eq!(panel.player_page, 0);
    }

    #[test]
    fn my_gifts_are_read_and_undone_on_a_second_press() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert_eq!(
            panel.activate(GIFTS_TAB_TOKEN),
            PanelAction::Query(StaffQuery::Gifts)
        );
        let row = |id, undone| sjk_identity::GiftRow {
            id,
            action: "holocron_give".into(),
            key_id: "bbbbbbbbbbbbbbbb".into(),
            name: "Fox".into(),
            what: "rare".into(),
            created: 1_791_720_000,
            undone,
            ..Default::default()
        };
        let staff = StaffState {
            gifts: Some(sjk_identity::GiftLog {
                gifts: vec![row(12, false), row(11, true)],
                per_day: Some(20),
                left: Some(19),
            }),
            ..StaffState::default()
        };
        rebuild(&mut panel, &me, &staff, None);
        assert!(panel.order.contains(&GIFT_UNDO_BASE));
        assert!(
            !panel.order.contains(&(GIFT_UNDO_BASE + 1)),
            "already undone"
        );
        assert_eq!(panel.activate(GIFT_UNDO_BASE), PanelAction::None);
        assert_eq!(
            panel.activate(GIFT_UNDO_BASE),
            PanelAction::Request(StaffRequest::GiftUndo {
                id: 12,
                key_id: "bbbbbbbbbbbbbbbb".into(),
            })
        );
        assert_eq!(
            panel.activate(GIFTS_REFRESH_TOKEN),
            PanelAction::Query(StaffQuery::Gifts)
        );
    }
}
