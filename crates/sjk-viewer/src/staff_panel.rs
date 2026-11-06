//! The Staff page: for a player the hub's operator made staff, the SJK team's tools
//! in the game (`docs/identity.md`, "Staff"): find a player, then, on the chosen
//! player's tabs, verify them and moderate their picture and SJK chat (Overview), give
//! or take back their medals and unlockables (Medals and unlocks), their holocrons,
//! credits and kyber shards (Holocrons and credits), clear their achievements, list
//! their keys and unlink one or merge another player into them (a player who reset
//! their key), and ask them for their game's logs; two more tabs read every player's
//! crash reports and the SJK chat bans in force, with Lift. Key ids never show on the
//! page (nor anywhere in the game): players go by name, a pasted key id shows as dots,
//! and linked keys are numbered. Every action is a
//! request signed by the player's own key (`sjk_identity::StaffRequest`,
//! `sjk_identity::StaffQuery`); the hub refuses it from any other key, and the page
//! only opens for a key whose profile says staff.
//!
//! Opened by the Profile page's Staff tools button or the `staff` command. Like the
//! Profile page it lives in the console and has the SJK UI's look ([`view`]). Tab
//! and Shift+Tab walk every control; Enter or Space works the one focused; the
//! search pill, the note and the merge field take typing; Up and Down move through the
//! players found; Escape goes back (or lets go of a merge being confirmed or picked).

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_identity::{Profile, StaffQuery, StaffRequest, StaffState};
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
/// Longest amount of credits or shards, in digits.
const AMOUNT_MAX: usize = 6;
/// Longest text the merge field takes (a pasted key id with spaces round it).
const MERGE_FIELD_MAX: usize = 40;
/// Longest search, as the hub takes it.
const QUERY_MAX: usize = 64;
/// Longest note with a medal or an unlock, as the hub takes it.
const NOTE_MAX: usize = 200;
/// How long the buttons that take something away (Clear all, Unlink, Remove, Take
/// credits, Take shards, Make or Remove staff) wait for their second press.
const CONFIRM_FOR: Duration = Duration::from_secs(3);

/// Whether `token` is one of the buttons only SJK's creator may use (the hub's
/// `PROTOCOL.md`, "Creator", since 23/10/2026): what gives, takes back, clears or merges.
/// Other staff see them dimmed; Tab skips them and a press does nothing.
fn owner_only(token: u16) -> bool {
    let within = |base: u16, count: usize| (base..base + count as u16).contains(&token);
    matches!(
        token,
        PICTURE_DOWN_TOKEN
            | CLEAR_ALL_TOKEN
            | HOLOCRON_GIVE_TOKEN
            | AMOUNT_TOKEN
            | CREDITS_GIVE_TOKEN
            | CREDITS_TAKE_TOKEN
            | SHARDS_GIVE_TOKEN
            | SHARDS_TAKE_TOKEN
            | MERGE_FIELD_TOKEN
            | MERGE_PICK_TOKEN
            | MERGE_TOKEN
            | MERGE_CONFIRM_TOKEN
    ) || within(RANK_BASE, crate::medals::Medal::COUNT)
        || within(GIVE_BASE, crate::medals::Medal::COUNT)
        || within(TAKE_BASE, crate::medals::Medal::COUNT)
        || within(UNLOCK_BASE, crate::unlockables::ALL.len())
        || within(RELOCK_BASE, crate::unlockables::ALL.len())
        || within(HOLOCRON_TIER_BASE, crate::holocrons::SLOTS)
        || within(HOLOCRON_REMOVE_BASE, HOLOCRONS_SHOWN)
        || within(CLEAR_BASE, crate::achievements::ALL.len())
        || within(UNLINK_BASE, LINKED_SHOWN)
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
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// The player's own profile, once the hub sent it.
    pub(crate) me: Option<&'a Profile>,
    pub(crate) staff: &'a StaffState,
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
    /// The players' keys in the list, in order.
    players: Vec<String>,
    me: Option<String>,
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
            SEARCH_TOKEN => {
                PanelAction::Request(StaffRequest::Search(self.query.trim().to_owned()))
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
            REWARDS_TAB_TOKEN => {
                self.view = View::Rewards;
                PanelAction::None
            }
            ECONOMY_TAB_TOKEN => {
                self.view = View::Economy;
                PanelAction::None
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
                PanelAction::Request(StaffRequest::Search(String::new()))
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
                match target {
                    Some(key_id) if !self.shown.unlocks.get(index).copied().unwrap_or(false) => {
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
        };
        if tab == ShotTab::Economy {
            self.amount = "50".to_owned();
            // A rare kyber crystal chosen, from the chips' second row.
            self.holocron_tier = crate::holocrons::COUNT + 1;
        }
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
            SEARCH_TOKEN | NOTE_TOKEN | MERGE_FIELD_TOKEN | AMOUNT_TOKEN
        );
        let in_list = (PLAYER_BASE..PLAYER_BASE + PLAYERS_SHOWN as u16).contains(&self.focus);
        if typing && !event.repeat {
            let field = match self.focus {
                SEARCH_TOKEN => &mut self.query,
                MERGE_FIELD_TOKEN => &mut self.merge_from,
                AMOUNT_TOKEN => &mut self.amount,
                _ => &mut self.note,
            };
            if crate::text_entry::before_edit(field, key, event.text.as_deref(), shift, control) {
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
                let count = self.shown.players.len().min(PLAYERS_SHOWN);
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
                    _ => &mut self.note,
                };
                field.pop();
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
            Some(token @ (SEARCH_TOKEN | NOTE_TOKEN | MERGE_FIELD_TOKEN | AMOUNT_TOKEN)) => {
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
        assert!(owner_only(RANK_BASE));
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
    fn staff_read_what_only_the_creator_gives_takes_back_or_merges() {
        let other = profile("bbbbbbbbbbbbbbbb", "Fox");
        let (mut panel, me, _) = drawn(vec![other.clone()]);
        let me = Profile {
            creator: false,
            ..me
        };
        let _ = panel.activate(PLAYER_BASE);
        panel.amount = "50".to_owned();
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &me, vec![other.clone()]);
        assert!(!panel.order.iter().any(|token| owner_only(*token)));
        assert_eq!(panel.activate(CREDITS_GIVE_TOKEN), PanelAction::None);
        assert_eq!(panel.activate(HOLOCRON_GIVE_TOKEN), PanelAction::None);
        assert_eq!(panel.activate(GIVE_BASE), PanelAction::None);
        // The day-to-day moderation stays theirs.
        assert_eq!(
            panel.activate(HUB_MUTE_TOKEN),
            PanelAction::Request(StaffRequest::ChatMute {
                key_id: other.key_id.clone(),
                muted: true
            })
        );
        let creator = Profile {
            staff: true,
            creator: true,
            ..me
        };
        let _ = panel.activate(ECONOMY_TAB_TOKEN);
        redraw(&mut panel, &creator, vec![other.clone()]);
        assert!(panel.order.contains(&CREDITS_GIVE_TOKEN));
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
}
