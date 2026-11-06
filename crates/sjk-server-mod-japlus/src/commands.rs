//! JA+'s player commands: the table of names ([`command`]), what its settings allow
//! ([`rules`]), the answers word for word ([`TEXTS`], the typos are JA+'s own), the
//! printed lists of `ammove` and `amDMGs` ([`prints`]), the plugin a client says it runs
//! ([`has_plugin`]) and the server console's `serverTime` and `modVersion` ([`console`]).
//!
//! From VisionJK `codemp/game/japlus/g_japlus_cmds.c`, `g_japlus_info.c`,
//! `g_japlus_tele.c:297-302` and `g_japlus_admin.c:1249-1366`; the server carries the
//! commands out with its shared code (`sjk-dedicated`'s `bridge_mod_commands.rs`).

use sjk_server_mod::{CommandTexts, EmoteKind, ModCommand, ModEmote, PlayerCommandRules, Settings};

/// The emotes (VisionJK `g_japlus_cmds.c:120-148`): name, pose, `jp_emotDisallow` bit.
/// Every pose is a retail `_humanoid` animation.
const EMOTES: &[(&str, &str, u8)] = &[
    ("amkneel", "BOTH_CROUCH3", 1),
    ("amdie", "BOTH_HIT1", 2),
    ("amdie2", "BOTH_DEATH15", 3),
    ("amfinishinghim", "BOTH_SABERKILLER1", 4),
    ("ampower", "BOTH_FORCE_GETUP_F2", 5),
    ("amnod", "BOTH_HEADNOD", 6),
    ("amshake", "BOTH_HEADSHAKE", 7),
    ("amhiltThrow1", "BOTH_SABERTHROW1START", 8),
    ("amhiltThrow2", "BOTH_SABERTHROW2START", 9),
    ("amcomeon", "BOTH_FORCEGRIP3THROW", 10),
    ("amwon", "TORSO_HANDSIGNAL1", 11),
    ("amhello", "BOTH_SILENCEGESTURE1", 12),
    ("sleep", "BOTH_SLEEP1", 14),
    ("amatease", "BOTH_STAND4", 15),
    ("amsurrender", "TORSO_SURRENDER_START", 16),
    ("amsit", "BOTH_SLEEP6START", 17),
    ("amsit2", "BOTH_STAND5TOSIT3", 18),
    ("amsit3", "BOTH_SIT6", 19),
    ("amsit4", "BOTH_SIT7", 20),
    ("ambeg", "BOTH_KNEES2", 21),
    ("amwait", "BOTH_STAND10", 22),
    ("amhips", "BOTH_STAND5TOSTAND8", 23),
    ("amvictory", "BOTH_TAVION_SWORDPOWER", 24),
    ("amneo", "BOTH_FORCE_GETUP_B4", 25),
    ("ambreakdance", "BOTH_FORCE_GETUP_B5", 26),
    ("ambreakdance2", "BOTH_FORCE_GETUP_B6", 27),
    ("amnoisy", "BOTH_SONICPAIN_HOLD", 28),
];

/// The other player commands (`g_japlus_cmds.c:845-849`, `g_japlus_admin.c:1275-1278`);
/// JA+'s `amAltDim`, `toggleFlame`, `immortal` and the admin commands are not here yet.
const COMMANDS: &[(&str, ModCommand)] = &[
    ("engage_FullForceDuel", ModCommand::FullForceDuel),
    ("amhug", ModCommand::Hug),
    ("amkiss", ModCommand::Kiss),
    ("amflip", ModCommand::Flip),
    ("amknockmedown", ModCommand::KnockMeDown),
    ("amdropsaber", ModCommand::DropSaber),
    ("jetpack", ModCommand::Jetpack),
    ("amcolor", ModCommand::SkinColor),
    ("sabercolor", ModCommand::SaberColor),
    ("saber", ModCommand::SaberSwitch),
    ("ammove", ModCommand::MoveList),
    ("amDMGs", ModCommand::DamageList),
    ("duelWhois", ModCommand::DuelList),
    ("refuseTele", ModCommand::RefuseTeleport),
];

/// The command `name` (any letter case) of JA+'s table.
pub fn command(name: &[u8]) -> Option<ModCommand> {
    if let Some(&(_, animation, bit)) = EMOTES
        .iter()
        .find(|(emote, _, _)| emote.as_bytes().eq_ignore_ascii_case(name))
    {
        let kind = match animation {
            // `g_japlus_cmds.c:875-890`.
            "BOTH_SABERKILLER1" => EmoteKind::LightsSaber,
            "BOTH_HIT1" | "BOTH_DEATH15" => EmoteKind::FreeForAllOnly,
            "BOTH_TAVION_SWORDPOWER" => EmoteKind::Victory,
            _ => EmoteKind::Plain,
        };
        return Some(ModCommand::Emote(ModEmote {
            animation,
            bit,
            // `JP_EmoteNotForTeams` (`g_japlus_cmds.c:72-80`): the held poses.
            free_for_all_pose: sjk_game_jka::pmove_japlus_emotes::full_body_pose(animation),
            kind,
        }));
    }
    COMMANDS
        .iter()
        .find(|(command, _)| command.as_bytes().eq_ignore_ascii_case(name))
        .map(|&(_, command)| command)
}

/// What the settings allow (`g_japlus_cmds.c:94,434,471,509,604`).
pub fn rules(settings: &dyn Settings) -> PlayerCommandRules {
    PlayerCommandRules {
        emotes_refused: settings.integer("jp_emotDisallow") as u32,
        knock_me_down: settings.integer("jp_allowAmKnockmedown") != 0,
        drop_saber: settings.integer("jp_allowAmDropSaber") != 0,
        jetpack: settings.integer("jp_allowJetpack") != 0,
        saber_switch_interval: settings.integer("jp_allowSaberSwitch"),
        texts: &TEXTS,
    }
}

/// JA+'s answers, word for word (VisionJK `g_japlus_cmds.c`, `g_japlus_info.c:129-140`,
/// `g_japlus_tele.c:301`, `g_japlus_admin.c:1269`). `amknockmedown` and `amdropsaber`
/// send their refusal as a command of its own, not a print, as JA+ does.
pub const TEXTS: CommandTexts = CommandTexts {
    intermission: ("print \"@@@CANNOT_TASK_INTERMISSION (", ") \n\""),
    emote_refused: "print \"this Emot is not allowed on this server!.\n\"",
    emote_gametype: "print \"this Emot is not compatible with this gametype!.\n\"",
    gametype_refused: "print \"this is not compatible with this gametype!.\n\"",
    kiss_refused: "print \"the new GLA animations are not loaded on this server or the admin disallowed them!.\n\"",
    knock_me_down_refused: "amknockmedown is not allowed!\n",
    drop_saber_refused: "amdropsaber is not allowed!\n",
    jetpack_refused: "print \"Cmd jetpack not allowed\n\"",
    jetpack_jedi: "print \"Cmd jetpack not allowed in JEdiVsMerc mod if you are a jedi\n\"",
    jetpack_team: "print \"Cmd jetpack not allowed in TEAM FFA if you are a JEdi. You have to be a merc.\n\"",
    saber_switch_refused: "print \"/saber ^1cmd is not allowed on this server!.\n\"",
    saber_color_plugin: "print \"/sabercolor is only usable with the JA+ client PLugin greater or equal to 1.1.\n\"",
    saber_color_usage: "print \"/sabercolor <saber_num 1 or 2> <R> <G> <B> where (R,G,B) are red, green, blue colors between 0 and 255\n\"",
    skin_color_usage: "print \"Usage: amcolor <R> <G> <B> where R,G,B are (Red,Green,Blue) integers between 0 and 255\n\"",
    duel_list: "print \"^5All Duelers :\n\"",
    duel_pair: ("print \"\t--->", " ^3VS ^7", "\n\""),
    refuse_teleport_on: "print \"You refuse to be teleported by admins.\n\"",
    refuse_teleport_off: "print \"You accept to be teleported by admins.\n\"",
};

/// `JP_VERSION` (VisionJK `g_japlus.h:28`).
const VERSION: &str = "JA+ Mod v2.4 Build 7";

fn print(text: &str) -> String {
    format!("print \"{text}\"")
}

/// `JP_ByLine` (`g_japlus_info.c:33-35`).
fn by_line() -> String {
    print(&format!(
        "\n^5---------------------\n^5{VERSION} by Slider\n---------------------\n"
    ))
}

/// `ammove` (`g_japlus_info.c:37-62`) and `amDMGs` (`:73-123`): JA+'s list of its moves
/// and of its saber damage settings, as JA+ prints them. The moves list is JA+'s own,
/// moves SJK does not play included.
pub fn prints(command: ModCommand, settings: &dyn Settings) -> Vec<String> {
    let int = |name: &str| settings.integer(name);
    let mut out = Vec::new();
    match command {
        ModCommand::MoveList => {
            out.push(print(&format!(
                "^5Normal g_forceRegenTime:^2{}  ^1-  ^5jp_privateDuelForceRegen:^2{}\n ",
                int("g_forceRegenTime"),
                int("jp_privateDuelForceRegen")
            )));
            out.push(print("\n^5Player Total NEW MOVES LIST: \n"));
            for (setting, value, text) in [
                ("jp_noKATA", 1, "^5Saber kata^2 are desactivated\n "),
                ("jp_jk2RedDFA", 1, "^5JK2 Single RED DFA^2 is activated\n "),
                ("jp_allowflipkick", 0, "^2FlipKicks are desactivated\n "),
                ("jp_allowSPattacks", 0, "^2All SP Attacks desactivated\n "),
            ] {
                if int(setting) == value {
                    out.push(print(text));
                }
            }
            out.push(print("^3STAFF:\n        ^5DOUBLE KICK: ^2 Alt Attack ^72 ennemis to your left and right or back and front\n        ^5SPIN KICK: ^2 Alt Attack ^7when enemis are close to you\n        ^5KICK FORWARD: ^2forward + alt attack ^7if enemy very close to you in front of you : \n"));
            out.push(print("^3SINGLE + first stance of STAFF and DUAL:\n        ^5PULL/SLASH ATTACK : ^2pull + attack+ back\n"));
            out.push(print(
                "^3LONG JUMP :  ^2: forward +jump+ speed ^7 you can also add attack action\n",
            ));
            let sp_forces = int("jp_allowSPForces") == 2;
            if !sp_forces {
                out.push(print("^2 desactivated :"));
            }
            out.push(print("^3FORCE DRAIN GRAB (sith kiss):  ^2: force drain with melee weapon^7 in front of your ennemy \n"));
            if !sp_forces {
                out.push(print("^2 desactivated :"));
            }
            out.push(print("^3FORCE PUSH GRAB (jedi tornado):  ^2: force push with melee weapon ^7 in front of your ennemy (only for Jedi)\n"));
            if int("jp_allowNewDFA") == 0 {
                out.push(print("^2All new DFA Attacks Desactivated by admin :\n "));
            }
            out.push(print("^3NEW DFA MOVES:  ^2Forward + Jump + attack ^7when you are close to an enemy..... It works with the following sabers:\n        ^5SINGLE SABER  : ^2Red STANCE\n        ^5DUAL  : ^2Yellow second STANCE\n        ^5STAFF : ^2Yellow second STANCE\n"));
            out.push(print("^3NEW DFA JUMP BACK SPIN KICK: ^2 Alt Attack + forward + Jump using STAFF stance, Dual fast Stance\n"));
            out.push(print("^3NEW DFA SABER FLIP STAB: ^2 Alt Attack + forward + Jump using single strong stance\n"));
            out.push(print("^3NEW DFA HIGH JUMP FLIP BACK KICK: ^2 Alt Attack + forward + Jump using Dual Stance or single medium Stance in front of an enemy\n"));
            out.push(print("^3NEW DFA MELEE/STAFF MOVES:  ^5JA+ SPIN KICK: ^2 Alt Attack + forward + left\n        ^5JA+ BACK KICK: ^2 Alt Attack + forward + right\n"));
            out.push(print("^3NEW DFA Pull impale MOVES:  ^2 While pulling press Attack ^7in front of an enemy\n"));
            out.push(by_line());
        }
        ModCommand::DamageList => damage_list(settings, &mut out),
        _ => {}
    }
    out
}

fn damage_list(settings: &dyn Settings, out: &mut Vec<String>) {
    let int = |name: &str| settings.integer(name);
    out.push(print("\n\n^5Damages LIST: \n"));
    out.push(print(&format!(
        "^2g_saberDamageScale: ^5{:.6}.\n ",
        settings.value("g_saberDamageScale")
    )));
    if int("d_saberSPStyleDamage") == 1 && int("jp_alterDMG") == 0 {
        out.push(print(
            "^2Single Player Saber Damages (default in BaseJKA) are ^5Activated.\n ",
        ));
    } else {
        out.push(print("^2Multiplayer Saber Damages are ^5Activated. (the new JA+ damage system looking like the JK2 one.)\n "));
    }
    let flip_scale = settings.value("jp_flipkickScaleDMG");
    if flip_scale != 1.0 {
        out.push(print(&format!(
            "^2FlipKick scale factor altered ^5{flip_scale:.6} ^2default 1\n "
        )));
    }
    if int("jp_allowModelScale") == 1 && int("jp_allowDmgSpeedScale") == 1 {
        out.push(print(
            "^2Damages and speed are scaled according to your model size.\n ",
        ));
    }
    if int("jp_alterDMG") == 0 {
        if int("d_saberSPStyleDamage") == 0 {
            out.push(print(
                "^2The Saber Damages of the MP Style are ^5NOT ^7altered.\n ",
            ));
        }
        return;
    }
    // `JP_DmgRange` and `JP_DmgFlat` (`g_japlus_info.c:65-71`): a range or a flat value,
    // then the module's own default.
    let range = |out: &mut Vec<String>, name: &str, setting: &str, low: i32, high: i32, lead| {
        out.push(print(&format!(
            "{}     - {name} : [{},{}] ^5Default : ^2[{low},{high}]\n ",
            if lead { "\n" } else { "" },
            int(&format!("{setting}Min")),
            int(&format!("{setting}Max")),
        )));
    };
    let flat = |out: &mut Vec<String>, name: &str, setting: &str, default: i32| {
        out.push(print(&format!(
            "     - {name} : {} ^5Default : ^2{default}\n ",
            int(setting)
        )));
    };
    out.push(print(&format!(
        "     - Saber Throw Scale Factor DMG : {:.6} ^5Default : ^2{:.6}\n ",
        settings.value("jp_SaberThrowScaleDMG"),
        1.0
    )));
    range(out, "StabDown Attack", "jp_StabDownAttack", 2, 50, false);
    range(out, "RollStab Attack", "jp_RollStabAttack", 2, 12, false);
    range(
        out,
        "NewSpin DfaAttack",
        "jp_NewSpinDfaAttack",
        2,
        60,
        false,
    );
    range(out, "NewPull Attack", "jp_NewPullAttack", 2, 50, false);
    out.push(print("\n^3DUAL & STAFF DMGs:\n "));
    flat(out, "StaffDual SpinAttack", "jp_StaffDualSpinAttack", 10);
    flat(out, "Dual KataAttack", "jp_DualKataAttack", 50);
    range(out, "Staff KataAttack", "jp_StaffKataAttack", 30, 50, false);
    range(out, "Staff BackAttack", "jp_StaffBackAttack", 2, 30, false);
    range(out, "Dual BackAttack", "jp_DualBackAttack", 2, 25, false);
    range(
        out,
        "StaffDual OtherAttack",
        "jp_StaffDualOtherAttack",
        2,
        45,
        false,
    );
    range(
        out,
        "StaffDual NormalAttack",
        "jp_StaffDualNormalAttack",
        2,
        60,
        false,
    );
    out.push(print("^3SINGLE DMGs:\n "));
    range(
        out,
        "Single KataAttack",
        "jp_SingleKataAttack",
        30,
        50,
        false,
    );
    range(
        out,
        "Strong NormalAttack",
        "jp_SingleStrongNormalAttack",
        2,
        110,
        true,
    );
    range(
        out,
        "Strong DfaAttack",
        "jp_SingleStrongDfaAttack",
        2,
        180,
        false,
    );
    range(
        out,
        "Strong BackAttack",
        "jp_SingleStrongBackAttack",
        2,
        50,
        false,
    );
    flat(out, "Strong OtherAttack", "jp_SingleStrongOtherAttack", 100);
    range(
        out,
        "Medium DfaAttack",
        "jp_SingleMediumDfaAttack",
        2,
        70,
        true,
    );
    range(
        out,
        "Medium BackAttack",
        "jp_SingleMediumBackAttack",
        2,
        35,
        false,
    );
    flat(out, "Medium OtherAttack", "jp_SingleMediumOtherAttack", 60);
    range(
        out,
        "Fast LungeAttack",
        "jp_SingleFastLungeAttack",
        2,
        30,
        true,
    );
    range(
        out,
        "Fast BackAttack",
        "jp_SingleFastBackAttack",
        2,
        30,
        false,
    );
    flat(out, "Fast OtherAttack", "jp_SingleFastOtherAttack", 35);
    out.push(by_line());
}

/// The client plugin version from which JA+ counts a player as having it (VisionJK
/// `g_japlus_misc.c:814`, `g_japlus_cmds.c:570`).
const PLUGIN_VERSION: f32 = 1.1;

/// Whether the userinfo's `cjp_client` names a plugin of 1.1 or later
/// (`G_ClientUsingJAPLUSClient`, read with `sscanf("%f")` from its first digit:
/// `g_japlus_misc.c:766-773`). SJK's client sends `1.4 SJK`, EternalJK `1.4JAPRO`.
pub fn has_plugin(userinfo: &[u8]) -> bool {
    let value = sjk_protocol::info_value(userinfo, b"cjp_client").unwrap_or_default();
    if !value.first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    let number: Vec<u8> = value
        .iter()
        .copied()
        .take_while(|byte| byte.is_ascii_digit() || *byte == b'.')
        .collect();
    std::str::from_utf8(&number)
        .ok()
        .and_then(|text| text.trim_end_matches('.').parse::<f32>().ok())
        .is_some_and(|version| version >= PLUGIN_VERSION)
}

/// `serverTime` and `modVersion` of the server console (`g_japlus_admin.c:1355-1366`).
pub fn console(name: &[u8], level_time: i32) -> Option<String> {
    if name.eq_ignore_ascii_case(b"serverTime") {
        let seconds = level_time / 1_000;
        return Some(format!(
            "server TIme {}:{}:{}\n",
            seconds / 3_600,
            (seconds / 60) % 60,
            seconds % 60
        ));
    }
    name.eq_ignore_ascii_case(b"modVersion")
        .then(|| format!("JA+ Version {VERSION}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Table(HashMap<&'static str, i32>);

    impl Settings for Table {
        fn integer(&self, name: &str) -> i32 {
            self.0.get(name).copied().unwrap_or_else(|| {
                crate::cvars::CVARS
                    .iter()
                    .find(|cvar| cvar.name == name)
                    .map_or(0, |cvar| cvar.default.parse().unwrap_or(0))
            })
        }
        fn value(&self, name: &str) -> f32 {
            self.integer(name) as f32
        }
    }

    #[test]
    fn the_27_emotes_play_retail_poses() {
        assert_eq!(EMOTES.len(), 27);
        for (name, animation, bit) in EMOTES {
            assert!(
                sjk_game_jka::legacy_animation_index(animation).is_some(),
                "{name}"
            );
            let Some(ModCommand::Emote(emote)) = command(name.to_ascii_uppercase().as_bytes())
            else {
                panic!("{name}");
            };
            assert_eq!((emote.animation, emote.bit), (*animation, *bit));
        }
        let emote = |name: &[u8]| match command(name) {
            Some(ModCommand::Emote(emote)) => emote,
            other => panic!("{other:?}"),
        };
        assert_eq!(emote(b"amdie").kind, EmoteKind::FreeForAllOnly);
        assert_eq!(emote(b"amvictory").kind, EmoteKind::Victory);
        assert_eq!(emote(b"amfinishinghim").kind, EmoteKind::LightsSaber);
        assert!(emote(b"amsit").free_for_all_pose && emote(b"amatease").free_for_all_pose);
        assert!(!emote(b"amnod").free_for_all_pose);
        assert_eq!(command(b"amkiss"), Some(ModCommand::Kiss));
        assert_eq!(command(b"duelwhois"), Some(ModCommand::DuelList));
        // Not JA+'s, or not built.
        assert_eq!(command(b"amtaunt"), None);
        assert_eq!(command(b"toggleFlame"), None);
    }

    #[test]
    fn the_rules_follow_the_settings() {
        let mut settings = Table(HashMap::new());
        let rules = rules(&settings);
        // JA+'s defaults: knock-me-down on, the saber drop and the jetpack off, a saber
        // change every 1.5 s.
        assert_eq!(rules.emotes_refused, 0);
        assert!(rules.knock_me_down && !rules.drop_saber && !rules.jetpack);
        assert_eq!(rules.saber_switch_interval, 1_500);
        settings.0.insert("jp_emotDisallow", 1 << 17);
        settings.0.insert("jp_allowJetpack", 1);
        let rules = super::rules(&settings);
        assert_eq!(rules.emotes_refused, 1 << 17);
        assert!(rules.jetpack);
    }

    #[test]
    fn a_plugin_from_1_1_counts() {
        assert!(has_plugin(b"\\name\\a\\cjp_client\\1.4 SJK"));
        assert!(has_plugin(b"\\cjp_client\\1.4JAPRO"));
        assert!(has_plugin(b"\\cjp_client\\1.1"));
        assert!(!has_plugin(b"\\cjp_client\\1.0"));
        assert!(!has_plugin(b"\\cjp_client\\JAPRO"));
        assert!(!has_plugin(b"\\name\\a"));
    }

    #[test]
    fn the_console_says_the_time_and_version() {
        assert_eq!(
            console(b"servertime", 3_723_000).as_deref(),
            Some("server TIme 1:2:3\n")
        );
        assert_eq!(
            console(b"modVersion", 0).as_deref(),
            Some("JA+ Version JA+ Mod v2.4 Build 7\n")
        );
        assert_eq!(console(b"status", 0), None);
    }

    #[test]
    fn the_damage_list_follows_jp_alter_dmg() {
        let mut settings = Table(HashMap::new());
        settings.0.insert("d_saberSPStyleDamage", 1);
        let short = prints(ModCommand::DamageList, &settings);
        assert!(short[2].contains("Single Player Saber Damages"));
        assert!(!short.iter().any(|line| line.contains("StabDown")));
        settings.0.insert("jp_alterDMG", 1);
        let long = prints(ModCommand::DamageList, &settings);
        assert!(long.iter().any(|line| line
            == "print \"\n     - Strong NormalAttack : [2,110] ^5Default : ^2[2,110]\n \""));
        assert!(long.last().unwrap().contains("by Slider"));
        let moves = prints(ModCommand::MoveList, &settings);
        assert!(moves[0].contains("g_forceRegenTime:^20"));
        assert!(prints(ModCommand::Hug, &settings).is_empty());
    }
}
