//! JA+'s settings: the 187 `jp_` cvars of the JA+ 2.4 Build 7 server module, with its
//! names, defaults and flags, in the order of its cvar table.
//!
//! The list is VisionJK's (`codemp/game/japlus/g_japlus_xcvar.h`, Blackwolf's port of
//! the module, GPL-2.0), which read it from `jampgamei386.so`; VisionJK's own `vs_`
//! additions are left out. Every setting is registered so that a JA+ server's
//! `server.cfg` loads as it is, but only those listed in `docs/server.md` (JA+ mode)
//! do something yet. The defaults are JA+'s, not the base game's: an operator who runs
//! this mod has chosen JA+.
//!
//! SJK's own settings for JA+ servers follow them, named `japlus.<name>` as everything
//! SJK adds for one server family ([`SJK_SETTINGS`]).

use sjk_server_mod::ModCvar;
use sjk_server_mod::flags::{ARCHIVE, INTERNAL, LATCH, ROM, SERVERINFO, SYSTEMINFO};

const fn cvar(name: &'static str, default: &'static str, flags: u32, announced: bool) -> ModCvar {
    ModCvar {
        name,
        default,
        flags,
        announced,
    }
}

/// `japlus.adminPasswords`: 1 lets JA+'s three rank passwords log admins in; 0 (the
/// default) leaves the key login and the one-time codes only (Sol, 06/11/2026; VisionJK
/// has the same switch, `vs_adminPasswords`).
pub const ADMIN_PASSWORDS: &str = "japlus.adminPasswords";
/// `japlus.adminHosts`: the addresses this server is reached by
/// (`"1.2.3.4:29070 192.168.0.5:29070"`); when set, a key login signed for another
/// address is refused (VisionJK's `vs_authHosts`).
pub const ADMIN_HOSTS: &str = "japlus.adminHosts";

/// How many of [`CVARS`] are SJK's, at its end.
pub const SJK_SETTINGS: usize = 2;

/// Every `jp_` setting, in JA+'s order, one line each, then SJK's own.
#[rustfmt::skip]
pub const CVARS: &[ModCvar] = &[
    cvar("jp_admLoginEffect", "1", ARCHIVE, true),
    cvar("jp_admScript", "1", ARCHIVE, true),
    cvar("jp_admUsableOn", "0", ARCHIVE, true),
    cvar("jp_allowAmDropSaber", "0", ARCHIVE, true),
    cvar("jp_allowAmKnockmedown", "1", ARCHIVE, true),
    cvar("jp_allowBlackName", "0", ARCHIVE, true),
    cvar("jp_allowCloakItem", "0", ARCHIVE | LATCH, true),
    cvar("jp_allowDmgSpeedScale", "0", ARCHIVE | LATCH, true),
    cvar("jp_allowFlameThrower", "1", ARCHIVE | LATCH, true),
    cvar("jp_allowflipkick", "1", ARCHIVE, true),
    cvar("jp_allowGodTalk", "1", ARCHIVE | LATCH, true),
    cvar("jp_allowHook", "1", ARCHIVE | LATCH, true),
    cvar("jp_allowJetpack", "0", ARCHIVE | LATCH, true),
    cvar("jp_allowModelScale", "0", ARCHIVE | LATCH, true),
    cvar("jp_allowNewDFA", "1", ARCHIVE, true),
    cvar("jp_allowRGBsaber", "1", ARCHIVE, true),
    cvar("jp_allowSaberSwitch", "1500", ARCHIVE, true),
    cvar("jp_allowSaberTouchDMG", "0", ARCHIVE, true),
    cvar("jp_allowSamePlayerNames", "0", ARCHIVE, true),
    cvar("jp_allowSPattacks", "1", ARCHIVE, true),
    cvar("jp_allowSPForces", "2", ARCHIVE, true),
    cvar("jp_allowTeamDuel", "1", ARCHIVE, true),
    cvar("jp_allowTeamKill", "0", ARCHIVE, true),
    cvar("jp_altDim", "2", ARCHIVE | LATCH, true),
    cvar("jp_altDimInactivityTimer", "8", ARCHIVE, true),
    cvar("jp_altDimName", "Alternate Dimension (real ffa, no rules)", ARCHIVE, true),
    cvar("jp_altDimTimer", "30", ARCHIVE, true),
    cvar("jp_alterDMG", "0", ARCHIVE, true),
    cvar("jp_antiAdmAbuseCmdsNumber", "10", ARCHIVE, true),
    cvar("jp_antiAdmAbuseNumberB4ban", "0", ARCHIVE, true),
    cvar("jp_antiAdmAbuseTimer", "0", ARCHIVE, true),
    cvar("jp_antiHackModel", "1", ARCHIVE, true),
    cvar("jp_autoDuelBOW", "1", ARCHIVE, true),
    cvar("jp_autoQUIT", "10-06:00", ARCHIVE, true),
    cvar("jp_banAdminIPs", "", ARCHIVE, false),
    cvar("jp_cinfo", "0", SERVERINFO | ROM, true),
    cvar("jp_clanPass", "", INTERNAL, false),
    cvar("jp_clanTag", "", ARCHIVE, true),
    cvar("jp_cloakFuel", "1", ARCHIVE, true),
    cvar("jp_councilAllowedCMD", "528351183", ARCHIVE, true),
    cvar("jp_councilLoginMSG", "%s ^7is logged as an ^3admin council.", ARCHIVE, false),
    cvar("jp_councilPass", "NOT SET", INTERNAL, false),
    cvar("jp_debugClientCMD", "0", INTERNAL, false),
    cvar("jp_DlBaseURL", "", ARCHIVE | SYSTEMINFO, false),
    cvar("jp_DualBackAttackMax", "25", ARCHIVE, true),
    cvar("jp_DualBackAttackMin", "2", ARCHIVE, true),
    cvar("jp_DualKataAttack", "50", ARCHIVE, true),
    cvar("jp_DuelAlpha", "100", ARCHIVE | SERVERINFO, true),
    cvar("jp_duelStartArmor", "100", ARCHIVE, true),
    cvar("jp_emotDisallow", "0", ARCHIVE, true),
    cvar("jp_endDuelBroadcastMSG", "1", ARCHIVE, true),
    cvar("jp_fakeClients", "", ARCHIVE, false),
    cvar("jp_fixAbsorb", "2.0", ARCHIVE, true),
    cvar("jp_fixEmot", "1", ARCHIVE, true),
    cvar("jp_fixRoll", "1", ARCHIVE, true),
    cvar("jp_flipkickScaleDMG", "1", ARCHIVE, true),
    cvar("jp_forceCombo", "0", ARCHIVE | SERVERINFO, true),
    cvar("jp_giveWP_MELEE", "1", ARCHIVE | LATCH, true),
    cvar("jp_gripOpenSaber", "0", ARCHIVE, true),
    cvar("jp_gripSpeedScale", "0.8", ARCHIVE | SERVERINFO, true),
    cvar("jp_highlander", "0", ARCHIVE | SERVERINFO | LATCH, true),
    cvar("jp_highlanderFinalDualTimeLimit", "2", ARCHIVE, true),
    cvar("jp_highlanderMaxForceRank", "5", ARCHIVE | LATCH, true),
    cvar("jp_highlanderStartingLives", "2", ARCHIVE | LATCH, true),
    cvar("jp_hookFloodProtect", "750", ARCHIVE, true),
    cvar("jp_hookSpeed", "800", ARCHIVE, true),
    cvar("jp_improveYellowDFA", "1", ARCHIVE, true),
    cvar("jp_inactivityProtectTimer", "120", ARCHIVE | LATCH, true),
    cvar("jp_instructorAllowedCMD", "140016139", ARCHIVE, true),
    cvar("jp_instructorLoginMSG", "%s ^7is logged as an ^5admin instructor.", ARCHIVE, false),
    cvar("jp_instructorPass", "NOT SET", INTERNAL, false),
    cvar("jp_jk2RedDFA", "0", ARCHIVE, true),
    cvar("jp_knightAllowedCMD", "207387355", ARCHIVE, true),
    cvar("jp_knightLoginMSG", "%s ^7is logged as an ^5admin knight.", ARCHIVE, false),
    cvar("jp_knightPass", "NOT SET", INTERNAL, false),
    cvar("jp_ledgeGrab", "1", ARCHIVE, true),
    cvar("jp_macroScan", "0", ARCHIVE, true),
    cvar("jp_maxConnFromIP", "2", ARCHIVE, true),
    cvar("jp_motd", "Put your MOTD here\\n URL : ^5www.clan.com", ARCHIVE, false),
    cvar("jp_motdtime", "10", ARCHIVE, true),
    cvar("jp_msgAllForceAltDim", "You are all sent \\nin ^5alternate dimension", ARCHIVE, false),
    cvar("jp_msgAllSilence", "You are all ^5silenced!", ARCHIVE, false),
    cvar("jp_msgAllSleep", "You are all sleeping!", ARCHIVE, false),
    cvar("jp_msgAllUnForceAltDim", "You aren't forced to\\nstay in ^5alternate dimension!", ARCHIVE, false),
    cvar("jp_msgAllUnsilence", "You are all ^5unsilenced!", ARCHIVE, false),
    cvar("jp_msgAllWake", "You are all waking up!", ARCHIVE, false),
    cvar("jp_msgBroadcastDenyvote", "%s\\n cannot callvote any more", ARCHIVE, false),
    cvar("jp_msgBroadcastEmpower", "%s\\n has been ^5empowered", ARCHIVE, false),
    cvar("jp_msgBroadcastForceAltDim", "%s\\n has been sent \\nin ^5alternate dimension", ARCHIVE, false),
    cvar("jp_msgBroadcastGhost", "%s\\n is a Ghost", ARCHIVE, false),
    cvar("jp_msgBroadcastMerc", "%s\\n is a Super Merc", ARCHIVE, false),
    cvar("jp_msgBroadcastMindtrick", "%s\\n has been Mindtricked", ARCHIVE, false),
    cvar("jp_msgBroadcastProtect", "%s\\n is being ^5protected", ARCHIVE, false),
    cvar("jp_msgBroadcastSilence", "%s\\n has been ^5silenced", ARCHIVE, false),
    cvar("jp_msgBroadcastSlap", "%s\\n has been slapped", ARCHIVE, false),
    cvar("jp_msgBroadcastSleep", "%s\\n is falling asleep", ARCHIVE, false),
    cvar("jp_msgBroadcastTeleport", "%s\\n has been teleported", ARCHIVE, false),
    cvar("jp_msgBroadcastUndenyvote", "%s\\nis allowed to callvote", ARCHIVE, false),
    cvar("jp_msgBroadcastUnempower", "%s\\n has lost the ^5power", ARCHIVE, false),
    cvar("jp_msgBroadcastUnForceAltDim", "%s\\n isn't forced to\\nstay in ^5alternate dimension", ARCHIVE, false),
    cvar("jp_msgBroadcastUnghost", "%s\\n has been Unghosted", ARCHIVE, false),
    cvar("jp_msgBroadcastUnmerc", "%s\\n lost Weapons", ARCHIVE, false),
    cvar("jp_msgBroadcastUnmindtrick", "%s\\n has been Unmindtricked", ARCHIVE, false),
    cvar("jp_msgBroadcastUnprotect", "%s\\n is loosing ^5protection", ARCHIVE, false),
    cvar("jp_msgBroadcastUnsilence", "%s\\n has been ^5unsilenced", ARCHIVE, false),
    cvar("jp_msgBroadcastWake", "%s\\n is waking up", ARCHIVE, false),
    cvar("jp_msgTargetDenyvote", "You lost the vote right", ARCHIVE, false),
    cvar("jp_msgTargetEmpower", "You are ^5empowered!", ARCHIVE, false),
    cvar("jp_msgTargetForceAltDim", "You are sent \\nin ^5alternate dimension", ARCHIVE, false),
    cvar("jp_msgTargetGhost", "You become a Ghost", ARCHIVE, false),
    cvar("jp_msgTargetMerc", "You become a Super Merc", ARCHIVE, false),
    cvar("jp_msgTargetMindtrick", "You have been Mindtricked", ARCHIVE, false),
    cvar("jp_msgTargetProtect", "You get protection!", ARCHIVE, false),
    cvar("jp_msgTargetSilence", "You are silenced!\\n Stop distrub...", ARCHIVE, false),
    cvar("jp_msgTargetSlap", "You Fly", ARCHIVE, false),
    cvar("jp_msgTargetSleep", "You are sleeping!", ARCHIVE, false),
    cvar("jp_msgTargetTeleport", "You have been teleported", ARCHIVE, false),
    cvar("jp_msgTargetUndenyvote", "You are allowed to vote", ARCHIVE, false),
    cvar("jp_msgTargetUnempower", "You lost the ^5power!", ARCHIVE, false),
    cvar("jp_msgTargetUnForceAltDim", "You aren't forced to\\nstay in ^5alternate dimension!", ARCHIVE, false),
    cvar("jp_msgTargetUnghost", "You have been Unghosted", ARCHIVE, false),
    cvar("jp_msgTargetUnmerc", "You lost Weapons", ARCHIVE, false),
    cvar("jp_msgTargetUnmindtrick", "You have been Unmindtricked", ARCHIVE, false),
    cvar("jp_msgTargetUnprotect", "You lost ^5protection!", ARCHIVE, false),
    cvar("jp_msgTargetUnsilence", "You can speak!", ARCHIVE, false),
    cvar("jp_msgTargetWake", "You awaken!\\n Stop distrub...", ARCHIVE, false),
    cvar("jp_newGLAAnims", "1", ARCHIVE | LATCH, true),
    cvar("jp_NewPullAttackMax", "50", ARCHIVE, true),
    cvar("jp_NewPullAttackMin", "2", ARCHIVE, true),
    cvar("jp_NewSpinDfaAttackMax", "60", ARCHIVE, true),
    cvar("jp_NewSpinDfaAttackMin", "2", ARCHIVE, true),
    cvar("jp_noAutoReplier", "0", ARCHIVE, true),
    cvar("jp_noKATA", "0", ARCHIVE, true),
    cvar("jp_oldPlayerName", "0", ARCHIVE | LATCH, true),
    cvar("jp_onlyVotingClients", "1", ARCHIVE, true),
    cvar("jp_primDimName", "Primary Dimension", ARCHIVE, true),
    cvar("jp_privateDuelForceRegen", "200", ARCHIVE, true),
    cvar("jp_pushAll", "1", ARCHIVE | LATCH, true),
    cvar("jp_pushPullKnockDown", "1", ARCHIVE, true),
    cvar("jp_reduceSaberBlock", "0", ARCHIVE, true),
    cvar("jp_removeDistanceDuelLimit", "1", ARCHIVE, true),
    cvar("jp_RollStabAttackMax", "12", ARCHIVE, true),
    cvar("jp_RollStabAttackMin", "2", ARCHIVE, true),
    cvar("jp_saberNoForceDrain", "0", ARCHIVE, true),
    cvar("jp_SaberThrowScaleDMG", "1", ARCHIVE, true),
    cvar("jp_siegeItemTime", "3", ARCHIVE, true),
    cvar("jp_SingleFastBackAttackMax", "30", ARCHIVE, true),
    cvar("jp_SingleFastBackAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleFastLungeAttackMax", "30", ARCHIVE, true),
    cvar("jp_SingleFastLungeAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleFastOtherAttack", "35", ARCHIVE, true),
    cvar("jp_SingleKataAttackMax", "50", ARCHIVE, true),
    cvar("jp_SingleKataAttackMin", "30", ARCHIVE, true),
    cvar("jp_SingleMediumBackAttackMax", "35", ARCHIVE, true),
    cvar("jp_SingleMediumBackAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleMediumDfaAttackMax", "70", ARCHIVE, true),
    cvar("jp_SingleMediumDfaAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleMediumOtherAttack", "60", ARCHIVE, true),
    cvar("jp_SingleStrongBackAttackMax", "50", ARCHIVE, true),
    cvar("jp_SingleStrongBackAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleStrongDfaAttackMax", "180", ARCHIVE, true),
    cvar("jp_SingleStrongDfaAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleStrongNormalAttackMax", "110", ARCHIVE, true),
    cvar("jp_SingleStrongNormalAttackMin", "2", ARCHIVE, true),
    cvar("jp_SingleStrongOtherAttack", "100", ARCHIVE, true),
    cvar("jp_slideOnPlayer", "0", ARCHIVE, true),
    cvar("jp_StabDownAttackMax", "50", ARCHIVE, true),
    cvar("jp_StabDownAttackMin", "2", ARCHIVE, true),
    cvar("jp_StaffBackAttackMax", "30", ARCHIVE, true),
    cvar("jp_StaffBackAttackMin", "2", ARCHIVE, true),
    cvar("jp_StaffDualNormalAttackMax", "60", ARCHIVE, true),
    cvar("jp_StaffDualNormalAttackMin", "2", ARCHIVE, true),
    cvar("jp_StaffDualOtherAttackMax", "45", ARCHIVE, true),
    cvar("jp_StaffDualOtherAttackMin", "2", ARCHIVE, true),
    cvar("jp_StaffDualSpinAttack", "10", ARCHIVE, true),
    cvar("jp_StaffKataAttackMax", "50", ARCHIVE, true),
    cvar("jp_StaffKataAttackMin", "30", ARCHIVE, true),
    cvar("jp_startMapVoteTimer", "60", ARCHIVE, true),
    cvar("jp_starTrekTeleport", "1", ARCHIVE, true),
    cvar("jp_teamLock", "0", ARCHIVE, true),
    cvar("jp_userInfoAntiSpam", "3", ARCHIVE, true),
    cvar("jp_votesDisable", "0", ARCHIVE, true),
    cvar("jp_voteTimer", "10", ARCHIVE, true),
    cvar("jp_welcomeMSG", "Welcome to the server!", ARCHIVE, false),
    cvar("jp_wordFilter", "0", ARCHIVE, true),
    cvar("jp_wordFilterBan", "", ARCHIVE, true),
    cvar("jp_wordFilterNumberB4punish", "0", ARCHIVE, true),
    // SJK's.
    cvar(ADMIN_PASSWORDS, "0", ARCHIVE, true),
    cvar(ADMIN_HOSTS, "", ARCHIVE, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_is_jas_and_named_once() {
        assert_eq!(CVARS.len(), 187 + SJK_SETTINGS);
        for (index, cvar) in CVARS.iter().enumerate() {
            let prefix = if index < 187 { "jp_" } else { "japlus." };
            assert!(cvar.name.starts_with(prefix), "{}", cvar.name);
            assert!(
                CVARS[..index]
                    .iter()
                    .all(|other| !other.name.eq_ignore_ascii_case(cvar.name)),
                "{} twice",
                cvar.name
            );
        }
    }

    #[test]
    fn passwords_never_reach_an_info_string() {
        for name in [
            "jp_councilPass",
            "jp_knightPass",
            "jp_instructorPass",
            "jp_clanPass",
        ] {
            let cvar = CVARS.iter().find(|cvar| cvar.name == name).unwrap();
            assert_eq!(cvar.flags, INTERNAL, "{name}");
        }
    }
}
