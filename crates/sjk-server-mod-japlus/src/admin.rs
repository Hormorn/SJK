//! JA+'s admin system: its command table with each command's bit in the ranks' masks,
//! the names of the settings it reads and its texts, which the server runs
//! (`sjk_server_mod::admin`).
//!
//! From VisionJK, Blackwolf's rebuild of the JA+ 2.4 B7 server module: the bit order
//! and the always-allowed commands (`codemp/game/japlus/g_japlus_admin.c:29-44`), which
//! commands the server console has too (`g_japlus_admin.c:1135-1174`, the `rcon`
//! column), the server console's own (`g_japlus_admin.c:1386-1391`), the texts of
//! `g_japlus_admin.c`, `g_japlus_admin2.c`, `g_japlus_fx.c` and `g_japlus_tele.c`.
//! Two names are SJK's: the key login's `sjk_amkey` and the console's `amcode` and
//! `amkeys`, after VisionJK's own login without a password (`codemp/game/auth/`).

use sjk_server_mod::admin::{
    AdminCommand as C, AdminEntry, AdminSettings, AdminSystem, AdminTexts, Callers, EffectTexts,
};

const fn bit(name: &'static str, bit: u8, command: Option<C>, callers: Callers) -> AdminEntry {
    AdminEntry {
        name,
        bit: Some(bit),
        always: false,
        command,
        callers,
    }
}

const fn always(name: &'static str, bit: u8, command: Option<C>) -> AdminEntry {
    AdminEntry {
        name,
        bit: Some(bit),
        always: true,
        command,
        callers: Callers::Players,
    }
}

const fn free(name: &'static str, command: C, callers: Callers) -> AdminEntry {
    AdminEntry {
        name,
        bit: None,
        always: false,
        command: Some(command),
        callers,
    }
}

use Callers::{Both, Console, Players};

/// The table. A command whose `command` is `None` is not carried out yet (see
/// `docs/server.md`, JA+ mode): it keeps its bit for the masks and `aminfo`.
#[rustfmt::skip]
pub const TABLE: &[AdminEntry] = &[
    // `jp_adminCmdsA`, bits 1 to 17.
    bit("amkick", 1, Some(C::Kick), Both),
    bit("amban", 2, Some(C::Ban), Both),
    bit("amShowMotd", 3, Some(C::ShowMotd), Both),
    always("amorigin", 4, Some(C::Origin)),
    bit("amslap", 5, Some(C::Slap), Both),
    bit("amsleep", 6, Some(C::Sleep), Both),
    bit("amwake", 7, Some(C::Wake), Both),
    bit("amghost", 8, None, Both),
    bit("amdenyvote", 9, Some(C::DenyVote), Both),
    bit("amprotect", 10, Some(C::Protect), Both),
    bit("amsilence", 11, Some(C::Silence), Both),
    bit("amunsilence", 12, Some(C::Unsilence), Both),
    bit("amUnForceAltDim", 13, None, Both),
    bit("amForceAltDim", 14, None, Both),
    bit("amempower", 15, None, Both),
    bit("ammerc", 16, None, Both),
    bit("ammindtrick", 17, None, Both),
    // `jp_adminCmdsB`, bits 18 to 38 (a mask holds 31: those above are always allowed or
    // never).
    bit("amtele", 18, Some(C::Tele), Both),
    bit("amstatus", 19, Some(C::Status), Both),
    bit("amwhois", 20, Some(C::Whois), Both),
    bit("ammap", 21, Some(C::Map), Both),
    bit("ampsay", 22, Some(C::PSay), Both),
    bit("npc", 23, None, Players),
    bit("amrename", 24, Some(C::Rename), Both),
    bit("amlockteam", 25, Some(C::LockTeam), Both),
    bit("amforceteam", 26, Some(C::ForceTeam), Both),
    bit("amkillvote", 27, Some(C::KillVote), Both),
    bit("amvstr", 28, None, Both),
    bit("amweather", 29, None, Players),
    always("amlogout", 30, Some(C::Logout)),
    always("amtelemark", 31, Some(C::TeleMark)),
    always("ampoll", 32, None),
    always("amseeGhost", 33, None),
    always("admCmdPrev", 34, None),
    always("admCmdNext", 35, None),
    always("playerPrev", 36, None),
    always("playerNext", 37, None),
    always("admCmdEXE", 38, None),
    // Every player's.
    free("amlogin", C::Login, Players),
    free("aminfo", C::Info, Players),
    free("ammotd", C::Motd, Players),
    free("amsay", C::Say, Players),
    free("refuseTele", C::RefuseTele, Players),
    // SJK's key login (the client's half).
    free("sjk_amkey", C::KeyLogin, Players),
    // The server console's.
    free("admGrant", C::Grant, Console),
    free("admToggle", C::Toggle, Console),
    free("admBan", C::AdminBan, Console),
    free("admAddIP", C::AddIp, Console),
    free("admRemoveIP", C::RemoveIp, Console),
    free("admListIP", C::ListIp, Console),
    free("amcode", C::Code, Console),
    free("amkeys", C::Keys, Console),
];

/// The settings it reads: JA+'s, and SJK's switch for the passwords.
pub const SETTINGS: AdminSettings = AdminSettings {
    allowed: [
        "jp_councilAllowedCMD",
        "jp_knightAllowedCMD",
        "jp_instructorAllowedCMD",
    ],
    login_message: [
        "jp_councilLoginMSG",
        "jp_knightLoginMSG",
        "jp_instructorLoginMSG",
    ],
    password: ["jp_councilPass", "jp_knightPass", "jp_instructorPass"],
    unset_password: "NOT SET",
    passwords_allowed: crate::cvars::ADMIN_PASSWORDS,
    key_hosts: crate::cvars::ADMIN_HOSTS,
    usable_on: "jp_admUsableOn",
    abuse_timer: "jp_antiAdmAbuseTimer",
    abuse_commands: "jp_antiAdmAbuseCmdsNumber",
    abuse_bans: "jp_antiAdmAbuseNumberB4ban",
    banned_ips: "jp_banAdminIPs",
    motd: "jp_motd",
    motd_time: "jp_motdtime",
    team_lock: "jp_teamLock",
    star_trek: "jp_starTrekTeleport",
};

const fn texts(
    command: C,
    target: &'static str,
    broadcast: &'static str,
    target_off: &'static str,
    broadcast_off: &'static str,
    all: &'static str,
) -> EffectTexts {
    EffectTexts {
        command,
        target,
        broadcast,
        target_off,
        broadcast_off,
        all,
    }
}

/// The effects' messages (`jp_msg*`).
#[rustfmt::skip]
pub const EFFECTS: &[EffectTexts] = &[
    texts(C::Slap, "jp_msgTargetSlap", "jp_msgBroadcastSlap", "", "", ""),
    texts(C::Sleep, "jp_msgTargetSleep", "jp_msgBroadcastSleep", "", "", "jp_msgAllSleep"),
    texts(C::Wake, "jp_msgTargetWake", "jp_msgBroadcastWake", "", "", "jp_msgAllWake"),
    texts(C::Silence, "jp_msgTargetSilence", "jp_msgBroadcastSilence", "", "", "jp_msgAllSilence"),
    texts(C::Unsilence, "jp_msgTargetUnsilence", "jp_msgBroadcastUnsilence", "", "", "jp_msgAllUnsilence"),
    texts(C::Protect, "jp_msgTargetProtect", "jp_msgBroadcastProtect", "jp_msgTargetUnprotect", "jp_msgBroadcastUnprotect", ""),
    texts(C::DenyVote, "jp_msgTargetDenyvote", "jp_msgBroadcastDenyvote", "jp_msgTargetUndenyvote", "jp_msgBroadcastUndenyvote", ""),
    texts(C::Tele, "jp_msgTargetTeleport", "jp_msgBroadcastTeleport", "", "", ""),
];

/// JA+'s fixed texts, as VisionJK has them (its typos too).
pub const TEXTS: AdminTexts = AdminTexts {
    not_allowed: "^7 You are not allowed to execute this command! \n",
    not_allowed_on: "^7 You are not allowed to execute this command on this player! \n",
    bad_password: "^3 Bad Password ! \n",
    no_such_code: "^3 No such code. This server takes no admin passwords: log in with your SJK key, or with a one-time code from the server's console. \n",
    unset_password_typed: "^3 Hey! try to hack me? :) ! \n",
    admin_banned_try: "^3 You are admin-banned ! \n",
    too_many_tries: "^3 Too many wrong tries, wait a moment. \n",
    logged_out: "^7 You are not an admin any more! \n",
    rcon_login: "^7You are logged in as an ^1admin with every right^7.\n",
    admin_banned: "You have been ^5admin-banned!\n",
    admin_unbanned: "You have been ^5admin-UNbanned!\n",
    abuse_wait: "^7Detected as ^1Admin Abuse^7 => Wait for ^5%is ^7to have your ^5%i ^7Number of sensible Usable cmds \n",
    abuse_countdown: "Again ^5%i Admin abuses ^7and you will be admbanned!\n",
    info_heading: [
        "^5Admin COUNCIL Commands \n",
        "^5Admin KNIGHT Commands \n",
        "^5Admin INSTRUCTOR Commands \n",
    ],
    info_player: &[
        "\n\n^5Player Commands \n",
        "\t^3Total New MOVES LIST Display: ^7ammove",
        "\n\t^3Damages list: ^7amDMGs",
        "\n\t^3Speak Mod Cmd: ^7say_team_mod, clanpass, clanwhois, amsay, clansay, ignore, tell",
        "\n\t^3Vote Cmds: ^7callvote",
        "\n\t^3Action Cmds: ^7engage_FullForceDuel, saber, jetpack, amknockmedown, amdropsaber, toggleFlame, +button12 ^2(grapple hook)^7",
        "\n\t^3Emot Cmds: ^7taunt ,meditate, bow, gloat, flourish, amkneel, amdie, amdie2, amfinishinghim, ampower, amflip, amnod, amshake, amhiltThrow1, amhiltThrow2, amcomeon, amwon, amhello, amhug, sleep, amatease, amsurrender, amsit, amsit2, amsit3, amsit4, ambeg, amwait, amhips, amvictory, amneo, ambreakdance, ambreakdance2, amnoisy, amkiss, ampoop",
        "\n\t^3Other Cmds: ^7refuseTele, amAltDim, ammotd, amcolor, sabercolor, immortal, duelWhois\n",
        "\n^5---------------------\n^5JA+ Mod v2.4 Build 7 by Slider\n---------------------\n",
    ],
    disallowed_gametype: "^5%s ^7is disallowed in this gametype!\n",
    in_alt_dim: "^1You cannot use this cmd on a player inside the Alternate dimension.!\n",
    slap_flood: "^1Admin Flood Protect: you can't execute this cmd on this player during ^5%is!\n",
    refuses_tele: "^1this player refuses to be teleported!\n",
    refuse_tele_hint: "^5You can refuse to be teleported typing ^1/refuseTele ^5into the console!\n^5retype it to accept again\n",
    refuse_on: "You refuse to be teleported by admins.\n",
    refuse_off: "You accept to be teleported by admins.\n",
    lava: "^5It seems there is lava or Slime under the teleport Point\n",
    lava_console: "^5It seems there is lava under the teleport Point\n",
    hurt_below: "^5It seems there is a trigger_hurt under the teleport Point\n",
    tele_mark: "TELEPORT MARKER :^3 %s\n",
    vote_killed: "^1Vote has been killed!\n",
    team_lock_usage: "usage ^7==> amlockteam <team>\n\t<team> = s (spectator), f (free), b (blue), r (red)\n",
    force_team_usage: "usage ^7==> amforceteam <ClientNUM> <team>\n\t<ClientNUM> can be -1 for all players\n\t<team> = s (spectator), f (free), b (blue), r (red)\n",
    map_usage: "Usage : ammap <gametype> <map_name>\n\t <gametype> \t",
    map_gametypes: "[^5HL = ^7HighLander] [^50 = ^7FFA] [^53 = ^7DUEL one on one tournament] [^54 = ^7POWER DUEL] [^56 = ^7TEAM DEATHMATCH] [^57 = ^7SIEGE] [^58 = ^7CTF]\n",
    map_gametypes_console: "[HL = HighLander] [0 = FFA] [3 = DUEL one on one tournament] [4 = POWER DUEL] [6 = TEAM DEATHMATCH] [7 = SIEGE] [8 = CTF]\n",
    map_bad_gametype: "^5Invalid GAMEType ^7==> valid are \n\t\t",
    invalid_string: "Invalid string.\n",
    team_locked: "^5%s ^7team ^6locked!\n",
    team_unlocked: "^5%s ^7team ^6Unlocked!\n",
    spectators_locked: "^7Sorry, ^5Spectator team^7 is locked!\n",
    free_locked: "^7Sorry, ^5You can't join^7 the game!\n",
    all_teams_locked: "^7Sorry, ^5All teams^7 are locked!\n",
    red_locked: "^7Sorry, the ^1Red team^7 is locked!\n",
    blue_locked: "^7Sorry, the ^4Blue team^7 is locked!\n",
    say_prefix: "^3<Admin>^7",
    motd_title: "^5SJK\n^5JA+ Mod v2.4 Build 7\n\n^7",
    motd_time_left: "\nTimeLeft : ^2%i s",
    whois_titles: ["^3Admin Council", "^5Admin Knight", "^2Admin Instructor"],
    no_plugin: "^1No Plugin",
    // VisionJK `g_cmds.c:2237-2240`.
    vote_denied: "You are not allowed to callVote\n",
};

/// JA+'s admin system.
pub const SYSTEM: AdminSystem = AdminSystem {
    table: TABLE,
    settings: SETTINGS,
    effects: EFFECTS,
    texts: &TEXTS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_server_mod::admin::{Rank, allows};

    #[test]
    fn the_bits_are_ja_pluses_order() {
        let masked: Vec<&str> = SYSTEM.masked().map(|entry| entry.name).collect();
        assert_eq!(masked.len(), 31);
        assert_eq!(&masked[..4], ["amkick", "amban", "amShowMotd", "amorigin"]);
        assert_eq!(masked[17], "amtele");
        assert_eq!(masked[29], "amlogout");
        assert_eq!(masked[30], "amtelemark");
        // Every bit once.
        for bit in 1..=38u8 {
            assert_eq!(
                TABLE.iter().filter(|entry| entry.bit == Some(bit)).count(),
                1,
                "{bit}"
            );
        }
    }

    #[test]
    fn the_default_masks_give_ja_pluses_rights() {
        let kick = SYSTEM.entry(b"AMKICK", false).unwrap();
        let ban = SYSTEM.entry(b"amban", false).unwrap();
        let map = SYSTEM.entry(b"ammap", false).unwrap();
        // jp_councilAllowedCMD 528351183, jp_knightAllowedCMD 207387355,
        // jp_instructorAllowedCMD 140016139.
        assert!(allows(Some(Rank::Council), 528_351_183, kick));
        assert!(allows(Some(Rank::Council), 528_351_183, ban));
        assert!(allows(Some(Rank::Council), 528_351_183, map));
        assert!(allows(Some(Rank::Knight), 207_387_355, kick));
        assert!(!allows(Some(Rank::Knight), 207_387_355, ban));
        assert!(allows(Some(Rank::Instructor), 140_016_139, kick));
        assert!(!allows(Some(Rank::Instructor), 140_016_139, map));
        // The always-allowed ones for every rank.
        let logout = SYSTEM.entry(b"amlogout", false).unwrap();
        assert!(allows(Some(Rank::Instructor), 0, logout));
    }

    #[test]
    fn players_and_the_console_have_their_own_commands() {
        assert!(SYSTEM.entry(b"amlogin", false).is_some());
        assert!(SYSTEM.entry(b"amlogin", true).is_none());
        assert!(SYSTEM.entry(b"amkick", true).is_some());
        assert!(SYSTEM.entry(b"amorigin", true).is_none());
        assert!(SYSTEM.entry(b"admgrant", true).is_some());
        assert!(SYSTEM.entry(b"admgrant", false).is_none());
        assert_eq!(
            SYSTEM.entry_of(C::Tele).map(|entry| entry.name),
            Some("amtele")
        );
        assert!(SYSTEM.effect(C::Slap).is_some());
    }

    #[test]
    fn every_setting_it_reads_is_registered() {
        let registered = |name: &str| crate::cvars::CVARS.iter().any(|cvar| cvar.name == name);
        let settings = SETTINGS;
        for name in settings
            .allowed
            .iter()
            .chain(&settings.login_message)
            .chain(&settings.password)
            .chain(&[
                settings.passwords_allowed,
                settings.key_hosts,
                settings.usable_on,
                settings.abuse_timer,
                settings.abuse_commands,
                settings.abuse_bans,
                settings.banned_ips,
                settings.motd,
                settings.motd_time,
                settings.team_lock,
                settings.star_trek,
            ])
        {
            assert!(registered(name), "{name}");
        }
        for effect in EFFECTS {
            for name in [
                effect.target,
                effect.broadcast,
                effect.target_off,
                effect.broadcast_off,
                effect.all,
            ] {
                assert!(name.is_empty() || registered(name), "{name}");
            }
        }
    }
}
