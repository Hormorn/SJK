//! A mod's admin system: ranks, the commands each rank may use, who an admin may use
//! them on, and the anti-abuse counter, as data the mod gives ([`AdminSystem`]) and
//! rules the server runs with it.
//!
//! The server keeps every player's rank and runs the commands; it never asks which mod
//! runs, only what [`crate::ServerMod::admin`] gives. The names of the settings an
//! admin system reads are the mod's ([`AdminSettings`]), and so are its fixed texts
//! ([`AdminTexts`]). The base game has none.
//!
//! Ranks are JA+'s (VisionJK `codemp/game/japlus/g_japlus.h:38-44`, the rebuild of the
//! JA+ 2.4 B7 server module): a level number each, lower is more, which the "usable on"
//! rule compares.

/// An admin's rank. The numbers are JA+'s levels (`jpAdminLevel_t`), which the "usable
/// on" setting compares; no rank is 0.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Rank {
    /// Every right, as the server console has them; only a key or a one-time code gives it.
    Rcon = 1,
    Council = 2,
    Knight = 3,
    Instructor = 4,
}

impl Rank {
    /// The three ranks a setting describes (council, knight, instructor), in that order.
    pub const NAMED: [Rank; 3] = [Rank::Council, Rank::Knight, Rank::Instructor];

    /// Its level (`jpAdminLevel_t`).
    pub fn level(self) -> i32 {
        self as i32
    }

    /// Its name in files and commands (`council`, `rcon`...).
    pub fn name(self) -> &'static str {
        match self {
            Rank::Rcon => "rcon",
            Rank::Council => "council",
            Rank::Knight => "knight",
            Rank::Instructor => "instructor",
        }
    }

    /// The rank named `name`, any letter case.
    pub fn from_name(name: &[u8]) -> Option<Rank> {
        [Rank::Rcon, Rank::Council, Rank::Knight, Rank::Instructor]
            .into_iter()
            .find(|rank| rank.name().as_bytes().eq_ignore_ascii_case(name))
    }

    /// Its place in [`Self::NAMED`], `None` for rcon.
    pub fn named_index(self) -> Option<usize> {
        Self::NAMED.iter().position(|named| *named == self)
    }
}

/// What an admin command does, which the server carries out. Grouped: the login, the
/// commands on players, the server's own, and the server console's.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AdminCommand {
    // The login.
    /// `amlogin <password or code>`.
    Login,
    /// `amlogout`.
    Logout,
    /// `aminfo`: the commands the rank may use.
    Info,
    /// SJK's key login: the client's half of the exchange (`sjk_amkey`).
    KeyLogin,
    // Players.
    Kick,
    Ban,
    /// `amShowMotd <player|-1>`: the message of the day shown to them.
    ShowMotd,
    /// `ammotd`: the message of the day shown to oneself (any player).
    Motd,
    /// `amorigin [player]`: where a player is.
    Origin,
    Slap,
    Sleep,
    Wake,
    DenyVote,
    Protect,
    Silence,
    Unsilence,
    Tele,
    /// `amtelemark`: where `amtele` with no argument goes.
    TeleMark,
    /// `refuseTele`: a player refuses the admins' teleports (any player).
    RefuseTele,
    Status,
    Whois,
    Rename,
    ForceTeam,
    // The server.
    Map,
    /// `ampsay <player|-1> <text>`: a centre print.
    PSay,
    /// `amsay <text>`: chat only admins read (any player).
    Say,
    LockTeam,
    KillVote,
    // The server console only.
    /// `admGrant <player> <rank|no>`.
    Grant,
    /// `admToggle <bit> <rank>`.
    Toggle,
    /// `admBan <player>`: no login from its address any more (or again).
    AdminBan,
    /// `admAddIP <mask>`, `admRemoveIP <mask|all>`, `admListIP`.
    AddIp,
    RemoveIp,
    ListIp,
    /// SJK's `amcode <rank>`: a one-time code.
    Code,
    /// SJK's `amkeys`: the key file read again and listed.
    Keys,
}

/// Who may type a command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Callers {
    /// Players only.
    Players,
    /// Players, and the server console (and rcon) with every right.
    Both,
    /// The server console only.
    Console,
}

/// One command of an admin system's table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminEntry {
    /// As typed (any letter case).
    pub name: &'static str,
    /// Its bit in the ranks' allowed masks (`jp_*AllowedCMD`), counted from 1; `None`
    /// for a command every player may type, its own rules deciding.
    pub bit: Option<u8>,
    /// Every logged in admin may use it, whatever its mask says.
    pub always: bool,
    /// What the server does; `None` for a command of the table the server does not
    /// carry out yet, which keeps its bit for the masks and `aminfo`.
    pub command: Option<AdminCommand>,
    pub callers: Callers,
}

/// The names of the settings an admin system reads, the mod's own.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSettings {
    /// Per [`Rank::NAMED`] rank: its allowed mask, its login message (`%s` the name)
    /// and its password.
    pub allowed: [&'static str; 3],
    pub login_message: [&'static str; 3],
    pub password: [&'static str; 3],
    /// The value a password setting has when none was set (JA+'s `NOT SET`), which is
    /// never a password.
    pub unset_password: &'static str,
    /// Whether the rank passwords count at all (SJK's switch, off by default).
    pub passwords_allowed: &'static str,
    /// On whom an admin may use a command (`jp_admUsableOn`, see [`usable_on`]).
    pub usable_on: &'static str,
    /// The anti-abuse counter: seconds of its window (0: off), commands in it, abuses
    /// before the admin is admin-banned (0: never).
    pub abuse_timer: &'static str,
    pub abuse_commands: &'static str,
    pub abuse_bans: &'static str,
    /// The addresses that may not log in, as `addip` masks with a space after each.
    pub banned_ips: &'static str,
    /// The message of the day (`\n` a line break) and how many seconds it shows.
    pub motd: &'static str,
    pub motd_time: &'static str,
    /// Locked teams, a bit each (`jp_teamLock`: 1 spectators, 2 free, 3 blue, 4 red).
    pub team_lock: &'static str,
    /// A teleport cloaks the player for a few seconds first (1).
    pub star_trek: &'static str,
}

/// The texts of one effect command, as the names of the settings that hold them (`""`
/// for none): what its target is shown, what everyone else is shown (`%s` the target's
/// name), the same when it is undone, and what everyone is shown when it was for all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectTexts {
    pub command: AdminCommand,
    pub target: &'static str,
    pub broadcast: &'static str,
    pub target_off: &'static str,
    pub broadcast_off: &'static str,
    pub all: &'static str,
}

/// An admin system's fixed texts, without the `print "..."` around them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminTexts {
    pub not_allowed: &'static str,
    pub not_allowed_on: &'static str,
    pub bad_password: &'static str,
    /// Answer to a password while passwords are off and no code fits.
    pub no_such_code: &'static str,
    pub unset_password_typed: &'static str,
    pub admin_banned_try: &'static str,
    pub too_many_tries: &'static str,
    pub logged_out: &'static str,
    /// Told to the player logged in with every right (no rank message exists for it).
    pub rcon_login: &'static str,
    pub admin_banned: &'static str,
    pub admin_unbanned: &'static str,
    /// The anti-abuse wait: the seconds left and the commands allowed, in that order.
    pub abuse_wait: &'static str,
    /// The abuses left before the ban.
    pub abuse_countdown: &'static str,
    /// `aminfo`'s heading per rank: rcon and council, knight, instructor.
    pub info_heading: [&'static str; 3],
    /// What follows the admin commands in `aminfo`, for every player.
    pub info_player: &'static [&'static str],
    /// A command of an effect refused in this game type (`%s` the command).
    pub disallowed_gametype: &'static str,
    /// On a player in the alternate dimension.
    pub in_alt_dim: &'static str,
    pub slap_flood: &'static str,
    pub refuses_tele: &'static str,
    pub refuse_tele_hint: &'static str,
    pub refuse_on: &'static str,
    pub refuse_off: &'static str,
    pub lava: &'static str,
    pub lava_console: &'static str,
    pub hurt_below: &'static str,
    pub tele_mark: &'static str,
    pub vote_killed: &'static str,
    pub team_lock_usage: &'static str,
    pub force_team_usage: &'static str,
    pub map_usage: &'static str,
    pub map_gametypes: &'static str,
    pub map_gametypes_console: &'static str,
    pub map_bad_gametype: &'static str,
    pub invalid_string: &'static str,
    /// `%s` the team: locked, unlocked.
    pub team_locked: &'static str,
    pub team_unlocked: &'static str,
    /// Refusals of a locked team.
    pub spectators_locked: &'static str,
    pub free_locked: &'static str,
    pub all_teams_locked: &'static str,
    pub red_locked: &'static str,
    pub blue_locked: &'static str,
    /// The tag before a centre print of `ampsay`, and `amsay`'s line around the name.
    pub say_prefix: &'static str,
    /// The motd's title lines and the line of seconds left (`%i`).
    pub motd_title: &'static str,
    pub motd_time_left: &'static str,
    /// `amwhois`' title of each [`Rank::NAMED`] rank.
    pub whois_titles: [&'static str; 3],
    /// `amstatus`' line when a player has no JA+ plugin.
    pub no_plugin: &'static str,
}

/// A mod's admin system, all of it static: its command table, the names of its
/// settings and its texts.
#[derive(Clone, Copy, Debug)]
pub struct AdminSystem {
    pub table: &'static [AdminEntry],
    pub settings: AdminSettings,
    pub effects: &'static [EffectTexts],
    pub texts: &'static AdminTexts,
}

impl AdminSystem {
    /// The entry typed as `name` (any letter case) for `console` (the server console) or
    /// a player.
    pub fn entry(&self, name: &[u8], console: bool) -> Option<&'static AdminEntry> {
        self.table.iter().find(|entry| {
            entry.name.as_bytes().eq_ignore_ascii_case(name)
                && match entry.callers {
                    Callers::Players => !console,
                    Callers::Console => console,
                    Callers::Both => true,
                }
        })
    }

    /// The entry of `command`.
    pub fn entry_of(&self, command: AdminCommand) -> Option<&'static AdminEntry> {
        self.table
            .iter()
            .find(|entry| entry.command == Some(command))
    }

    /// The texts of an effect command.
    pub fn effect(&self, command: AdminCommand) -> Option<&'static EffectTexts> {
        self.effects.iter().find(|texts| texts.command == command)
    }

    /// The commands with a bit, in the bit order, for `aminfo` and `admToggle`.
    pub fn masked(&self) -> impl Iterator<Item = &'static AdminEntry> {
        let table = self.table;
        (1..=31u8).filter_map(move |bit| table.iter().find(|entry| entry.bit == Some(bit)))
    }
}

/// `G_CmdBitAllowed`: bit `bit` of `mask`, none outside 0 to 31.
pub fn bit_allowed(mask: i32, bit: u8) -> bool {
    bit <= 31 && (mask >> bit) & 1 != 0
}

/// `G_ClientAdminCmdAllowed`: whether an admin of `rank` whose rank's mask is `mask`
/// (ignored for rcon) may use `entry`. A command without a bit is no admin command:
/// any player may type it.
pub fn allows(rank: Option<Rank>, mask: i32, entry: &AdminEntry) -> bool {
    let Some(bit) = entry.bit else {
        return true;
    };
    match rank {
        None => false,
        Some(Rank::Rcon) => true,
        Some(_) => entry.always || bit_allowed(mask, bit),
    }
}

/// `G_ClientAdminCmdAllowedOn`'s rule (VisionJK `g_japlus_admin.c:243-268`): whether an
/// admin of `admin` may use a command on a player of `target`, by `usable_on`
/// (`jp_admUsableOn`): -1 on everybody, -2 on non-admins only, 0 or more on non-admins
/// and on an admin whose level is at least `usable_on` above the admin's own (0: the
/// same rank or a lower one); on oneself always.
pub fn usable_on(admin: Option<Rank>, target: Option<Rank>, oneself: bool, setting: i32) -> bool {
    if oneself {
        return true;
    }
    let Some(admin) = admin else {
        return false;
    };
    let target_level = target.map_or(0, Rank::level);
    match setting {
        -1 => true,
        -2 => target_level < Rank::Council.level(),
        _ if target_level < Rank::Council.level() => true,
        setting if setting >= 0 => target_level >= setting + admin.level(),
        _ => false,
    }
}

/// The anti-abuse counter of one admin (`G_ClientCheckAdminAbuse`, VisionJK
/// `g_japlus_admin.c:178-231`): at most a number of "sensible" commands in a window of
/// seconds, and after some abuses an admin-ban.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbuseCounter {
    /// When the window ends (`abuseTimer`).
    pub window_end: i32,
    /// Commands left in it (`abuseCmdsLeft`).
    pub left: i32,
    /// Abuses left before the ban (`abuseBansLeft`); JA+ starts it at 0.
    pub bans_left: i32,
}

/// What one more sensible command makes of the counter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Abuse {
    /// Within the limits: the command goes on.
    Fine,
    /// Too many: refused, the wait told (and `countdown` the abuses left before the
    /// ban); `first` on the abuse that crossed the limit, which the log tells.
    Refused { first: bool, countdown: bool },
    /// Too many, and the abuses are used up: the admin loses its rank and its address
    /// goes into the admin-ban list.
    Banned,
}

/// The counter's settings, as read from the mod's settings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbuseRules {
    /// Seconds of the window; 0 switches the counter off.
    pub seconds: i32,
    /// Commands allowed in one window.
    pub commands: i32,
    /// Abuses before the ban; 0 never bans.
    pub bans_before: i32,
}

impl AbuseCounter {
    /// One more sensible command at `now` by an admin of `rank` (rcon is never counted).
    pub fn count(&mut self, rules: AbuseRules, rank: Option<Rank>, now: i32) -> Abuse {
        if rules.seconds == 0 || rank == Some(Rank::Rcon) {
            return Abuse::Fine;
        }
        if self.window_end <= now {
            self.window_end = now + rules.seconds * 1000;
            self.left = rules.commands + 1;
        }
        self.left -= 1;
        if self.left >= 1 {
            return Abuse::Fine;
        }
        let first = self.left == 0;
        let mut countdown = false;
        if first && rules.bans_before > 0 {
            countdown = true;
            if self.bans_left < 1 {
                return Abuse::Banned;
            }
            self.bans_left -= 1;
        }
        Abuse::Refused { first, countdown }
    }

    /// Seconds left in the window at `now`, as the wait message tells them.
    pub fn seconds_left(&self, now: i32) -> i32 {
        (self.window_end - now) / 1000
    }
}

/// `JP_FormatName`: the first `%s` of `format` replaced by `name`, nothing else of it
/// read as a format.
pub fn format_name(format: &[u8], name: &[u8]) -> Vec<u8> {
    match format.windows(2).position(|pair| pair == b"%s") {
        Some(at) => [&format[..at], name, &format[at + 2..]].concat(),
        None => format.to_vec(),
    }
}

/// `JP_NewLines`: every typed `\n` (a backslash and an `n`) made a line break.
pub fn new_lines(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut bytes = text.iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        if byte == b'\\' && bytes.peek() == Some(&b'n') {
            bytes.next();
            out.push(b'\n');
        } else {
            out.push(byte);
        }
    }
    out
}

/// `JP_SanitizeString` with `JP_NAME_CHARS` (VisionJK `g_japlus_chat.c:38-61`): a name
/// as JA+ compares it, lower case, without colours nor control characters, its first 31
/// characters.
pub fn plain_name(name: &[u8]) -> Vec<u8> {
    let mut lower = Vec::with_capacity(name.len());
    let mut at = 0;
    while at < name.len() {
        match name[at] {
            27 => at += if at + 1 < name.len() { 2 } else { 1 },
            // A `signed char` below a space: the control characters and every byte
            // from 0x80 up.
            byte if !(b' '..0x80).contains(&byte) => at += 1,
            byte => {
                lower.push(byte.to_ascii_lowercase());
                at += 1;
            }
        }
    }
    let mut out = Vec::with_capacity(lower.len());
    let mut at = 0;
    while at < lower.len() && at < 31 {
        if lower[at] == b'^' {
            at += if lower.get(at + 1).is_some_and(u8::is_ascii_digit) {
                2
            } else {
                1
            };
        } else {
            out.push(lower[at]);
            at += 1;
        }
    }
    out
}

/// `G_getClientNumberForString`: how an admin names a player. `-1` is everybody, a word
/// starting with a digit is a client number (`atoi`), any other the first player whose
/// [`plain_name`] holds it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Named {
    Everybody,
    Number(i32),
    /// A part of a name; the server looks for it with [`name_holds`].
    Name,
}

/// What `word` names.
pub fn named(word: &[u8]) -> Named {
    if word == b"-1" {
        Named::Everybody
    } else if word.first().is_some_and(u8::is_ascii_digit) {
        let digits = word.iter().take_while(|byte| byte.is_ascii_digit()).count();
        Named::Number(
            std::str::from_utf8(&word[..digits])
                .ok()
                .and_then(|text| text.parse::<i64>().ok())
                .map_or(i32::MAX, |number| number.min(i64::from(i32::MAX)) as i32),
        )
    } else {
        Named::Name
    }
}

/// `JP_ClientByName`'s test, not exact: `name`, as JA+ compares names, holds `part`.
pub fn name_holds(name: &[u8], part: &[u8]) -> bool {
    let (name, part) = (plain_name(name), plain_name(part));
    !name.is_empty()
        && (part.is_empty()
            || name
                .windows(part.len())
                .any(|window| window == part.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn entry(name: &'static str, bit: u8, always: bool) -> AdminEntry {
        AdminEntry {
            name,
            bit: Some(bit),
            always,
            command: None,
            callers: Callers::Both,
        }
    }

    #[test]
    fn ranks_are_ja_plus_levels_and_names() {
        assert_eq!(Rank::Rcon.level(), 1);
        assert_eq!(Rank::Instructor.level(), 4);
        assert_eq!(Rank::from_name(b"Council"), Some(Rank::Council));
        assert_eq!(Rank::from_name(b"no"), None);
        assert_eq!(Rank::Knight.named_index(), Some(1));
        assert_eq!(Rank::Rcon.named_index(), None);
    }

    #[test]
    fn a_rank_may_use_what_its_mask_allows() {
        let kick = entry("amkick", 1, false);
        let logout = entry("amlogout", 30, true);
        assert!(!allows(None, -1, &kick));
        assert!(allows(Some(Rank::Rcon), 0, &kick));
        assert!(allows(Some(Rank::Knight), 0b10, &kick));
        assert!(!allows(Some(Rank::Knight), 0b01, &kick));
        // Always allowed for an admin, never for a player.
        assert!(allows(Some(Rank::Instructor), 0, &logout));
        assert!(!allows(None, 0, &logout));
        // JA+'s default masks: the council may kick, the instructor may not ban.
        assert!(allows(Some(Rank::Council), 528_351_183, &kick));
        assert!(!allows(
            Some(Rank::Instructor),
            140_016_139,
            &entry("amban", 2, false)
        ));
        assert!(!bit_allowed(-1, 32));
    }

    #[test]
    fn usable_on_follows_ja_plus() {
        let (council, knight) = (Some(Rank::Council), Some(Rank::Knight));
        // On oneself always, never without a rank.
        assert!(usable_on(None, None, true, 0));
        assert!(!usable_on(None, None, false, -1));
        // On players always; on admins by the setting.
        assert!(usable_on(knight, None, false, 0));
        assert!(usable_on(knight, council, false, -1));
        assert!(!usable_on(knight, council, false, -2));
        assert!(usable_on(council, None, false, -2));
        // 0: the same rank or a lower one (a higher level).
        assert!(usable_on(council, knight, false, 0));
        assert!(usable_on(knight, knight, false, 0));
        assert!(!usable_on(knight, council, false, 0));
        // 1: a rank strictly lower.
        assert!(!usable_on(knight, knight, false, 1));
        assert!(usable_on(council, knight, false, 1));
        // JA+ compares levels: rcon's (1) is below council's, so a player of the rcon
        // rank is reached as a player is.
        assert!(usable_on(knight, Some(Rank::Rcon), false, 0));
        assert!(!usable_on(knight, council, false, -3));
    }

    #[test]
    fn the_abuse_counter_waits_then_bans() {
        let rules = AbuseRules {
            seconds: 10,
            commands: 2,
            bans_before: 0,
        };
        let mut counter = AbuseCounter::default();
        let rank = Some(Rank::Knight);
        assert_eq!(counter.count(rules, rank, 0), Abuse::Fine);
        assert_eq!(counter.count(rules, rank, 1), Abuse::Fine);
        assert_eq!(
            counter.count(rules, rank, 2),
            Abuse::Refused {
                first: true,
                countdown: false
            }
        );
        assert_eq!(counter.seconds_left(2000), 8);
        assert_eq!(
            counter.count(rules, rank, 3),
            Abuse::Refused {
                first: false,
                countdown: false
            }
        );
        // A new window.
        assert_eq!(counter.count(rules, rank, 10_000), Abuse::Fine);
        // Off, and never for rcon.
        let mut off = AbuseCounter::default();
        for now in 0..20 {
            assert_eq!(off.count(AbuseRules::default(), rank, now), Abuse::Fine);
            assert_eq!(off.count(rules, Some(Rank::Rcon), now), Abuse::Fine);
        }
        // With bans: JA+ starts with no abuse left, so the first abuse bans.
        let rules = AbuseRules {
            bans_before: 3,
            ..rules
        };
        let mut counter = AbuseCounter::default();
        counter.count(rules, rank, 0);
        counter.count(rules, rank, 0);
        assert_eq!(counter.count(rules, rank, 0), Abuse::Banned);
        let mut counter = AbuseCounter {
            bans_left: 1,
            ..AbuseCounter::default()
        };
        counter.count(rules, rank, 0);
        counter.count(rules, rank, 0);
        assert_eq!(
            counter.count(rules, rank, 0),
            Abuse::Refused {
                first: true,
                countdown: true
            }
        );
        assert_eq!(counter.bans_left, 0);
    }

    #[test]
    fn texts_take_the_name_and_line_breaks() {
        assert_eq!(
            format_name(b"%s ^7is logged as %s", b"Sol"),
            b"Sol ^7is logged as %s"
        );
        assert_eq!(format_name(b"no name", b"Sol"), b"no name");
        assert_eq!(new_lines(b"a\\nb\\\\c\\"), b"a\nb\\\\c\\");
    }

    #[test]
    fn players_are_named_as_ja_plus_names_them() {
        assert_eq!(named(b"-1"), Named::Everybody);
        assert_eq!(named(b"12"), Named::Number(12));
        assert_eq!(named(b"3abc"), Named::Number(3));
        assert_eq!(named(b"sol"), Named::Name);
        assert_eq!(plain_name(b"^1S^7ol\x1bxFox"), b"solfox");
        assert!(name_holds(b"^1Sol^7Fox", b"FOX"));
        assert!(!name_holds(b"Padawan", b"sol"));
    }
}
