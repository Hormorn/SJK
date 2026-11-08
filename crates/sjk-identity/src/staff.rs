//! Staff requests (`PROTOCOL.md`, "Staff"): what a key the hub's operator made staff
//! may do from inside the game: find players, verify them, give and take back medals and
//! unlockables, give and take back holocrons, clear achievements, moderate the SJK chat
//! and players' pictures, merge two players into one and unlink a key from a player. The
//! hub refuses every one from a key that is not staff; the client only offers them to
//! a player whose own profile says `staff`.

use crate::logs::{CrashDetail, CrashGroup, CrashSummary, LogRequestRow};
use crate::wire::Profile;
use serde::Deserialize;

/// One staff request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaffRequest {
    /// Find players: 16 hex digits for one key, a part of a name, or empty for the
    /// players seen most recently.
    Search(String),
    /// Give a medal.
    Award {
        /// The key that gets it.
        key_id: String,
        /// The medal's id.
        medal: String,
        /// Why, shown with the medal; may be empty.
        note: String,
        /// The rank it starts at, 1 (Issued) to 5 (Solar) (since 28/10/2026).
        rank: u8,
    },
    /// Take one award of a medal back.
    Unaward {
        /// The key that holds it.
        key_id: String,
        /// The medal's id.
        medal: String,
    },
    /// Set the rank of a medal the key holds, 1 (Issued) to 5 (Solar), by hand
    /// (`PROTOCOL.md`, "Staff", since 27/10/2026). Quiet, like every staff change.
    MedalRank {
        /// The key that holds it.
        key_id: String,
        /// The medal's id.
        medal: String,
        /// The new rank.
        rank: u8,
    },
    /// Grant an unlockable (`PROTOCOL.md`, "Unlocks").
    Unlock {
        /// The key that gets it.
        key_id: String,
        /// The unlockable's id.
        unlock: String,
        /// Shown with it; may be empty.
        note: String,
    },
    /// Take an unlockable back.
    Relock {
        /// The key that holds it.
        key_id: String,
        /// The unlockable's id.
        unlock: String,
    },
    /// Clear achievements.
    ClearAchievements {
        /// The key whose achievements go.
        key_id: String,
        /// The achievement's id, or empty for every one.
        id: String,
    },
    /// Delete an SJK chat message for everyone.
    ChatDelete {
        /// The message's id.
        id: u64,
    },
    /// Mute or unmute a key in the SJK chat.
    ChatMute {
        /// The key.
        key_id: String,
        /// Mute (true) or unmute.
        muted: bool,
    },
    /// Lift every chat ban on a key's player ([`crate::ban`]), however many verified
    /// players banned them.
    ChatUnban {
        /// The key.
        key_id: String,
    },
    /// Take a key's picture down.
    AvatarRemove {
        /// The key.
        key_id: String,
    },
    /// Stop a key from uploading pictures (taking its picture down), or let it again.
    AvatarBlock {
        /// The key.
        key_id: String,
        /// Stop (true) or let again.
        blocked: bool,
    },
    /// Give a holocron of a tier (`PROTOCOL.md`, "Holocrons"). A gift counts toward no
    /// cap and is not announced.
    HolocronGive {
        /// The key that gets it.
        key_id: String,
        /// The tier's wire id (`uncommon`, `rare`, `legendary`, `mythical`).
        tier: String,
        /// Shown with it; may be empty.
        note: String,
    },
    /// Take one holocron back, by its number at the hub.
    HolocronRemove {
        /// The key that holds it.
        key_id: String,
        /// The holocron's number ([`crate::Holocron::id`]).
        id: u64,
    },
    /// Say whether the SJK team vouches for a player (the verified mark).
    Verify {
        /// The player's key.
        key_id: String,
        /// Vouch (true) or no longer.
        verified: bool,
        /// The group their tick shows ([`crate::Tick::GROUPS`]); `None` leaves it as it
        /// was.
        tick: Option<crate::Tick>,
    },
    /// Make a player staff or take it away; only SJK's creator may (the hub refuses
    /// anyone else), and never from the creator.
    SetStaff {
        /// The player's key.
        key_id: String,
        /// Staff (true) or no longer.
        staff: bool,
    },
    /// Merge one player into another, for good: everything `from` holds goes to the
    /// player of `key_id`, and `from`'s keys become linked keys of that player. A staff
    /// player cannot be merged away.
    Merge {
        /// The player who is kept.
        key_id: String,
        /// The player merged away.
        from: String,
    },
    /// Detach a linked key from its player. The key starts afresh the next time it is
    /// used; nothing it brought moves back.
    Unlink {
        /// The linked key (never a player's main key).
        key_id: String,
    },
    /// Change a player's credits by `delta` (`PROTOCOL.md`, "Holocrons"); the hub
    /// refuses one that would go below 0.
    Credits {
        /// The player's key.
        key_id: String,
        /// Credits given (above 0) or taken (below 0).
        delta: i64,
        /// Why, shown with it; may be empty.
        note: String,
    },
    /// Block a player of the game server being played for every SJK player there who
    /// follows staff blocks, until its map changes (`PROTOCOL.md`, "Map blocks"), or
    /// lift that block.
    MapBlock {
        /// The game server, `ip:port`.
        server: String,
        /// The map, as the game names it.
        map: String,
        /// The server's `sv_serverid` for this map load.
        serverid: i64,
        /// The player's slot.
        slot: u8,
        /// What the game shows in that slot.
        name: String,
        /// Block (true) or lift the block.
        blocked: bool,
    },
    /// SJK's creator only: ignore servers' shader remaps on `map` in every SJK client
    /// that follows the shared list (`PROTOCOL.md`, "Remap blocks"), or stop.
    RemapBlock {
        /// The map, as `mp/ffa4`.
        map: String,
        /// Block (true) or take the map off the list.
        blocked: bool,
    },
    /// Change a player's kyber shards by `delta` (`PROTOCOL.md`, "Shards and the
    /// shop"), as [`Self::Credits`] does credits.
    Shards {
        /// The player's key.
        key_id: String,
        /// Shards given (above 0) or taken (below 0).
        delta: i64,
        /// Why, shown with it; may be empty.
        note: String,
    },
}

impl StaffRequest {
    /// The key the request changes, if it changes one.
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Search(_)
            | Self::ChatDelete { .. }
            | Self::MapBlock { .. }
            | Self::RemapBlock { .. } => None,
            Self::Award { key_id, .. }
            | Self::Unaward { key_id, .. }
            | Self::MedalRank { key_id, .. }
            | Self::Unlock { key_id, .. }
            | Self::Relock { key_id, .. }
            | Self::ClearAchievements { key_id, .. }
            | Self::ChatMute { key_id, .. }
            | Self::ChatUnban { key_id }
            | Self::AvatarRemove { key_id }
            | Self::AvatarBlock { key_id, .. }
            | Self::HolocronGive { key_id, .. }
            | Self::HolocronRemove { key_id, .. }
            | Self::Verify { key_id, .. }
            | Self::SetStaff { key_id, .. }
            | Self::Merge { key_id, .. }
            | Self::Unlink { key_id }
            | Self::Credits { key_id, .. }
            | Self::Shards { key_id, .. } => Some(key_id),
        }
    }
}

/// A staff request about players' logs and crash reports (`PROTOCOL.md`, "Log
/// requests"): asking a player for their logs, and reading what players sent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaffQuery {
    /// Ask a player for their game's logs; their game asks them whether to send.
    RequestLogs {
        /// The player's key.
        key_id: String,
        /// Shown to the player with the request; may be empty.
        note: String,
    },
    /// The log requests, newest first.
    LogRequests,
    /// One request with the logs sent for it, by its number.
    Log(i64),
    /// The crash reports, grouped by the hub's signature.
    CrashGroups,
    /// The newest crash reports of one signature, or of all with an empty one.
    Crashes(String),
    /// One whole crash report, by its number.
    Crash(i64),
    /// The SJK chat bans in force, newest first (`PROTOCOL.md`, "Chat bans").
    ChatBans,
}

/// What the hub answered a [`StaffQuery`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaffAnswer {
    /// The request made, or one request with its logs.
    LogRequest(Box<LogRequestRow>),
    /// The log requests.
    LogRequests(Vec<LogRequestRow>),
    /// The crash groups.
    CrashGroups(Vec<CrashGroup>),
    /// The crash reports of a signature.
    Crashes(Vec<CrashSummary>),
    /// One crash report.
    Crash(Box<CrashDetail>),
    /// The chat bans in force.
    ChatBans(Vec<ChatBanRow>),
}

/// A chat ban in force, as the hub lists them to staff (`PROTOCOL.md`, "Chat bans",
/// ChatBan).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ChatBanRow {
    /// The ban's number at the hub.
    #[serde(default)]
    pub id: i64,
    /// The banned player's key, empty for a ban that holds only their address.
    #[serde(default)]
    pub key_id: String,
    /// Their display name.
    #[serde(default)]
    pub name: String,
    /// The address the hub last saw them use is banned too.
    #[serde(default)]
    pub address: bool,
    /// The key of the player who banned them.
    #[serde(default)]
    pub by_key_id: String,
    /// That player's display name.
    #[serde(default)]
    pub by_name: String,
    /// When the ban was made, unix seconds.
    #[serde(default)]
    pub created: i64,
    /// When it ends, unix seconds.
    #[serde(default)]
    pub until: i64,
}

/// `POST /v1/staff/chat-bans`'s answer.
#[derive(Deserialize)]
pub(crate) struct ChatBanRows {
    pub(crate) bans: Vec<ChatBanRow>,
}

/// A text a staff member opened (logs or a crash report), for the page to save and show.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OpenedText {
    /// A file name for it, without extension: ASCII letters, digits, `-` and `_` only.
    pub name: String,
    /// The text.
    pub text: String,
}

/// What the staff requests brought back.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaffState {
    /// Counts answers, so a reader can tell a new one.
    pub serial: u64,
    /// The players the last search found, updated by later answers about them.
    pub players: Vec<Profile>,
    /// What the last request came to: done, or why not.
    pub message: String,
    /// Whether the last request failed.
    pub failed: bool,
    /// A request is on its way.
    pub busy: bool,
    /// The log requests, as the hub last listed them (newest first).
    pub log_requests: Vec<LogRequestRow>,
    /// The crash groups, as the hub last listed them.
    pub crash_groups: Vec<CrashGroup>,
    /// The crash reports of the signature last asked about.
    pub crashes: Vec<CrashSummary>,
    /// That signature (empty: the newest of all).
    pub crashes_of: String,
    /// The chat bans in force, as the hub last listed them (newest first).
    pub chat_bans: Vec<ChatBanRow>,
    /// The logs or crash report opened last, until the viewer takes it
    /// (`Service::take_opened`).
    pub opened: Option<OpenedText>,
}

impl StaffState {
    /// Put `profile`, the hub's answer about a key, in place of that key's entry.
    pub fn replace(&mut self, profile: &Profile) {
        if let Some(entry) = self
            .players
            .iter_mut()
            .find(|entry| entry.key_id == profile.key_id)
        {
            *entry = profile.clone();
        } else {
            self.players.insert(0, profile.clone());
        }
    }

    /// Take the player who owns `key_id` (their main key or a linked one) out of the
    /// list, as a merge does with the player merged away; `kept` is never taken out.
    pub fn remove(&mut self, key_id: &str, kept: &str) {
        self.players
            .retain(|entry| entry.key_id == kept || !entry.has_key(key_id));
    }
}

impl StaffState {
    /// Take in the hub's answer to `query`.
    pub fn answered(&mut self, query: &StaffQuery, answer: StaffAnswer) {
        self.message = match answer {
            StaffAnswer::LogRequest(mut row) if matches!(query, StaffQuery::Log(_)) => {
                let message = format!("Opened the logs of {}", plain(&row.name));
                let name = format!("sjk-logs-{}-{}", row.id, file_part(&row.name));
                self.open(name, std::mem::take(&mut row.logs));
                self.put_request(*row);
                message
            }
            StaffAnswer::LogRequest(row) => {
                let message = format!("Asked {} for their logs", plain(&row.name));
                self.put_request(*row);
                message
            }
            StaffAnswer::LogRequests(rows) => {
                self.log_requests = rows;
                format!("{} log requests", self.log_requests.len())
            }
            StaffAnswer::CrashGroups(groups) => {
                self.crash_groups = groups;
                format!("{} kinds of crash", self.crash_groups.len())
            }
            StaffAnswer::Crashes(crashes) => {
                self.crashes = crashes;
                self.crashes_of = match query {
                    StaffQuery::Crashes(signature) => signature.clone(),
                    _ => String::new(),
                };
                format!("{} crash reports", self.crashes.len())
            }
            StaffAnswer::Crash(detail) => {
                let name = format!(
                    "sjk-crash-{}-{}",
                    detail.summary.id,
                    file_part(&detail.summary.name)
                );
                self.open(name, detail.readable());
                format!("Opened crash report #{}", detail.summary.id)
            }
            StaffAnswer::ChatBans(bans) => {
                self.chat_bans = bans;
                match self.chat_bans.len() {
                    1 => "1 chat ban in force".to_owned(),
                    count => format!("{count} chat bans in force"),
                }
            }
        };
    }

    fn open(&mut self, name: String, text: String) {
        self.opened = Some(OpenedText { name, text });
    }

    /// Put `row` in place of the request with its number, or first.
    fn put_request(&mut self, row: LogRequestRow) {
        match self
            .log_requests
            .iter_mut()
            .find(|entry| entry.id == row.id)
        {
            Some(entry) => *entry = row,
            None => self.log_requests.insert(0, row),
        }
    }
}

/// `name` without colour codes.
fn plain(name: &str) -> String {
    let mut out = String::new();
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '^' && chars.peek().is_some() {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// `name` as part of a file name: ASCII letters and digits, the rest `_`, at most 24.
fn file_part(name: &str) -> String {
    let part: String = plain(name)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(24)
        .collect();
    let part = part.trim_matches('_');
    if part.is_empty() {
        "player".to_owned()
    } else {
        part.to_owned()
    }
}

/// What a request's success is called on the page: never a key id, which the game never
/// shows.
pub(crate) fn done(request: &StaffRequest, count: usize) -> String {
    match request {
        StaffRequest::Search(query) if query.is_empty() => {
            format!("{count} players seen lately")
        }
        StaffRequest::Search(_) => match count {
            0 => "Nobody found".to_owned(),
            1 => "1 player found".to_owned(),
            count => format!("{count} players found"),
        },
        StaffRequest::Award { medal, rank: 1, .. } => format!("Gave {medal}"),
        StaffRequest::Award { medal, rank, .. } => format!("Gave {medal} at rank {rank}"),
        StaffRequest::Unaward { medal, .. } => format!("Took back {medal}"),
        StaffRequest::MedalRank { medal, rank, .. } => format!("Set {medal} to rank {rank}"),
        StaffRequest::Unlock { unlock, .. } => format!("Unlocked {unlock}"),
        StaffRequest::Relock { unlock, .. } => format!("Took back {unlock}"),
        StaffRequest::ClearAchievements { id, .. } if id.is_empty() => {
            "Cleared every achievement".to_owned()
        }
        StaffRequest::ClearAchievements { id, .. } => format!("Cleared {id}"),
        StaffRequest::ChatDelete { .. } => "Deleted the message".to_owned(),
        StaffRequest::ChatMute { muted: true, .. } => "Muted in the chat".to_owned(),
        StaffRequest::ChatMute { muted: false, .. } => "Unmuted in the chat".to_owned(),
        StaffRequest::ChatUnban { .. } => "Lifted their chat bans".to_owned(),
        StaffRequest::AvatarRemove { .. } => "Took the picture down".to_owned(),
        StaffRequest::AvatarBlock { blocked: true, .. } => {
            "Stopped their pictures and took theirs down".to_owned()
        }
        StaffRequest::AvatarBlock { blocked: false, .. } => {
            "They may upload pictures again".to_owned()
        }
        StaffRequest::HolocronGive { tier, .. } => match tier.strip_prefix("kyber_") {
            Some(crystal) => format!("Gave {} {crystal} kyber crystal", article(crystal)),
            None => format!("Gave {} {tier} holocron", article(tier)),
        },
        StaffRequest::HolocronRemove { id, .. } => format!("Took back holocron #{id}"),
        StaffRequest::Verify {
            verified: true,
            tick: Some(tick),
            ..
        } => format!("Verified, {} tick", tick.label()),
        StaffRequest::Verify { verified: true, .. } => "Verified".to_owned(),
        StaffRequest::Verify {
            verified: false, ..
        } => "No longer verified".to_owned(),
        StaffRequest::SetStaff { staff: true, .. } => "Made staff".to_owned(),
        StaffRequest::SetStaff { staff: false, .. } => "No longer staff".to_owned(),
        StaffRequest::Merge { .. } => "Merged the other player into this one".to_owned(),
        StaffRequest::Unlink { .. } => "Unlinked the key: it starts afresh".to_owned(),
        StaffRequest::Credits { delta, .. } if *delta < 0 => {
            format!("Took {} credits", delta.unsigned_abs())
        }
        StaffRequest::Credits { delta, .. } => format!("Gave {delta} credits"),
        StaffRequest::Shards { delta, .. } if *delta < 0 => {
            format!("Took {} kyber shards", delta.unsigned_abs())
        }
        StaffRequest::Shards { delta, .. } => format!("Gave {delta} kyber shards"),
        StaffRequest::MapBlock { blocked: true, .. } => {
            "Blocked for everyone on this map".to_owned()
        }
        StaffRequest::MapBlock { blocked: false, .. } => {
            "No longer blocked for everyone".to_owned()
        }
        StaffRequest::RemapBlock { map, blocked: true } => {
            format!("Server remaps ignored on {map} for every SJK player")
        }
        StaffRequest::RemapBlock {
            map,
            blocked: false,
        } => {
            format!("{map} is off the shared remap block list")
        }
    }
}

/// "an" before a tier that starts with a vowel ("an uncommon"), else "a".
fn article(word: &str) -> &'static str {
    if word.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_logs_become_a_text_with_a_safe_file_name() {
        let mut state = StaffState::default();
        state.answered(
            &StaffQuery::LogRequests,
            StaffAnswer::LogRequests(vec![LogRequestRow {
                id: 4,
                name: "^1Kush^7ina".into(),
                state: "open".into(),
                ..LogRequestRow::default()
            }]),
        );
        assert_eq!(state.message, "1 log requests");
        state.answered(
            &StaffQuery::Log(4),
            StaffAnswer::LogRequest(Box::new(LogRequestRow {
                id: 4,
                name: "^1Kush^7ina/..".into(),
                state: "sent".into(),
                logs: "lines".into(),
                ..LogRequestRow::default()
            })),
        );
        let opened = state.opened.take().unwrap();
        assert_eq!(opened.name, "sjk-logs-4-Kushina");
        assert_eq!(opened.text, "lines");
        assert_eq!(state.message, "Opened the logs of Kushina/..");
        // The list's row is updated, without the text.
        assert_eq!(state.log_requests.len(), 1);
        assert_eq!(state.log_requests[0].state, "sent");
        assert!(state.log_requests[0].logs.is_empty());
    }

    #[test]
    fn a_new_request_goes_first_and_crash_reports_open_as_text() {
        let mut state = StaffState::default();
        state.answered(
            &StaffQuery::RequestLogs {
                key_id: "0123456789abcdef".into(),
                note: String::new(),
            },
            StaffAnswer::LogRequest(Box::new(LogRequestRow {
                id: 9,
                name: "***".into(),
                ..LogRequestRow::default()
            })),
        );
        assert_eq!(state.log_requests[0].id, 9);
        assert_eq!(state.message, "Asked *** for their logs");
        let mut detail = CrashDetail::default();
        detail.summary.id = 3;
        detail.summary.name = "***".into();
        state.answered(&StaffQuery::Crash(3), StaffAnswer::Crash(Box::new(detail)));
        let opened = state.opened.take().unwrap();
        assert_eq!(opened.name, "sjk-crash-3-player");
        assert!(opened.text.starts_with("SJK crash report #3"));
        state.answered(
            &StaffQuery::Crashes("00ff00ff00ff00ff".into()),
            StaffAnswer::Crashes(vec![CrashSummary::default()]),
        );
        assert_eq!(state.crashes_of, "00ff00ff00ff00ff");
        state.answered(
            &StaffQuery::ChatBans,
            StaffAnswer::ChatBans(vec![ChatBanRow::default(), ChatBanRow::default()]),
        );
        assert_eq!(state.chat_bans.len(), 2);
        assert_eq!(state.message, "2 chat bans in force");
    }

    #[test]
    fn credits_and_shards_say_what_was_given_or_taken() {
        let credits = |delta| StaffRequest::Credits {
            key_id: "0123456789abcdef".into(),
            delta,
            note: String::new(),
        };
        assert_eq!(done(&credits(50), 1), "Gave 50 credits");
        assert_eq!(done(&credits(-20), 1), "Took 20 credits");
        let shards = StaffRequest::Shards {
            key_id: "0123456789abcdef".into(),
            delta: -5,
            note: String::new(),
        };
        assert_eq!(done(&shards, 1), "Took 5 kyber shards");
        assert_eq!(shards.target(), Some("0123456789abcdef"));
    }

    #[test]
    fn gifts_say_what_they_were_and_no_line_names_a_key() {
        let give = |tier: &str| StaffRequest::HolocronGive {
            key_id: "0123456789abcdef".into(),
            tier: tier.into(),
            note: String::new(),
        };
        assert_eq!(done(&give("rare"), 1), "Gave a rare holocron");
        assert_eq!(done(&give("uncommon"), 1), "Gave an uncommon holocron");
        assert_eq!(
            done(&give("kyber_legendary"), 1),
            "Gave a legendary kyber crystal"
        );
        for request in [
            StaffRequest::Merge {
                key_id: "0123456789abcdef".into(),
                from: "fedcba9876543210".into(),
            },
            StaffRequest::Unlink {
                key_id: "fedcba9876543210".into(),
            },
        ] {
            let line = done(&request, 1);
            assert!(!line.contains("fedcba9876543210"), "{line}");
        }
    }
}
