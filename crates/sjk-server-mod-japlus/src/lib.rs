//! JA+ for SJK's dedicated server: `g_mod japlus` runs JA+ 2.4 Build 7's rules in place
//! of the base game's, so that the server plays as a JA+ server and every JA+ client
//! (SJK's, EternalJK, the JA+ plugin) predicts it.
//!
//! JA+ is closed source. The reference is VisionJK (github.com/Blackwolf1337/visionjk,
//! GPL-2.0), Blackwolf's rebuild of the JA+ 2.4 B7 server module from its Linux binary,
//! compared with the original in random games; its behaviour is ported here, not its
//! code. What runs so far, and what does not, is in `docs/server.md` (JA+ mode).
//!
//! - [`cvars`]: JA+'s settings, at JA+'s defaults.
//! - [`cinfo`]: `jp_cinfo`, the rules the server announces, only those it plays.
//! - [`JaPlus::movement`]: the JA+ movement of the shared `Pmove`
//!   (`sjk_game_jka::pmove_japlus`), selected from the same bits the clients read, and
//!   `jp_gripSpeedScale`, which the clients read from the server info too.
//! - [`combat`]: JA+'s saber damage and `kill` rules, which only the server plays.
//! - [`JaPlus::force`]: JA+'s Force rules (`sjk_game_jka::force_rules`).
//! - [`JaPlus::duels`]: JA+'s private duels (`sjk_game_jka::duel::DuelRules`), and its
//!   `engage_FullForceDuel` ([`JaPlus::command`]).
//! - [`commands`]: its emotes and player commands, and the server console's own.
//! - [`JaPlus::hook`]: JA+'s grappling hook (`sjk_game_jka::japlus_hook`), whose pull every
//!   JA+ client predicts ([`ModMovement::grapple`]).
//! - [`admin`]: JA+'s admin system, its ranks' commands and texts, which the server runs
//!   (`sjk_server_mod::admin`).

pub mod admin;
pub mod cinfo;
pub mod combat;
pub mod commands;
pub mod cvars;

use sjk_game_jka::RollRules;
use sjk_game_jka::duel::{DuelRules, WinnerReport};
use sjk_game_jka::force_rules::{ForceRules, GripOpenSaber};
use sjk_game_jka::japlus_hook::{DF_NO_HOOK_FUEL, HookRules};
use sjk_game_jka::pmove::grapple::GrappleRules;
use sjk_game_jka::pmove_japlus::JaPlusRules;
use sjk_game_jka::pmove_japlus_emotes::EmoteFreeze;
use sjk_protocol::{JaPlusCapabilities, ServerDialect};
use sjk_server_mod::{ModCombat, ModCommand, ModMovement, PlayerCommandRules, ServerMod, Settings};

/// `gamename` of JA+ 2.4 B7 (VisionJK `g_japlus.h` `JP_GAMENAME`): clients and server
/// browsers tell a JA+ server by it (`sjk_protocol::is_ja_plus_game_name`).
pub const GAMENAME: &str = "JA+ Mod v2.4 B7";
/// Its server info `V` (`JP_SERVER_VERSION`), `<version>B<build>`, which the JA+ plugin
/// reads.
pub const VERSION: &str = "2.4B7";

/// The `dmflags` bit with which a JA+ server keeps the base game's collision boxes
/// (`JP_DF_OLD_COLLISION`), which clients read as `JaPlusRules::old_collision`.
const DF_OLD_COLLISION: i32 = 256;

/// The JA+ server mod.
#[derive(Clone, Copy, Debug, Default)]
pub struct JaPlus;

impl ServerMod for JaPlus {
    fn id(&self) -> &'static str {
        "japlus"
    }

    fn name(&self) -> &'static str {
        "JA+"
    }

    fn cvars(&self) -> &'static [sjk_server_mod::ModCvar] {
        cvars::CVARS
    }

    fn identity(&self) -> &'static [(&'static str, &'static str)] {
        &[("gamename", GAMENAME), ("V", VERSION)]
    }

    fn game_directory(&self) -> &'static str {
        // JA+ servers run in `japlus` (its `server.cfg`, `+set fs_game japlus`).
        "japlus"
    }

    fn derived_info(&self, settings: &dyn Settings) -> Vec<(&'static str, String)> {
        vec![("jp_cinfo", cinfo::cinfo(settings).to_string())]
    }

    fn movement(&self, settings: &dyn Settings) -> ModMovement {
        // What a client makes of this server's info (`JaPlusRules::from_game_state`,
        // `RollRules::from_dialect`), from the same bits.
        let dialect = ServerDialect::JaPlus {
            capabilities: JaPlusCapabilities(cinfo::cinfo(settings)),
        };
        ModMovement {
            ja_plus: JaPlusRules {
                old_collision: settings.integer("dmflags") & DF_OLD_COLLISION != 0,
                // A serverinfo setting (`CVAR_SERVERINFO`), which the clients read from
                // there (`JaPlusRules::from_game_state`).
                grip_speed: settings.value("jp_gripSpeedScale"),
                // VisionJK `bg_pmove.c:11186-11193`: `jp_fixEmot` freezes the held
                // emotes only while a player has no plugin (the server then freezes them
                // all, [`ServerMod::client_has_plugin`]); `amatease` always.
                emote_freeze: if settings.integer("jp_fixEmot") != 0 {
                    EmoteFreeze::AtEase
                } else {
                    EmoteFreeze::Held
                },
                ..JaPlusRules::from_dialect(&dialect)
            },
            fix_roll: RollRules::from_dialect(&dialect).fix_roll,
            // JA+ is built on the 1.01 game code, which has none of OpenJK's animation
            // fixes (VisionJK `docs/japlus/differences.md`: `g_fixRunWalkAnims`,
            // `g_fixWeaponAttackAnim` and `g_fixSaberMoveData` are 0 there).
            legacy_fixes: 0,
            // The hook's pull, as every JA+ client predicts it on any JA+ server
            // (`GrappleRules::from_dialect`); without `jp_allowHook` no player is ever
            // flagged for it.
            grapple: GrappleRules::from_dialect(&dialect),
        }
    }

    fn hook(&self, settings: &dyn Settings) -> HookRules {
        // VisionJK `japlus/g_japlus_hook.c`: `jp_allowHook` (latched), `jp_hookSpeed`,
        // `jp_hookFloodProtect`, and `dmflags` 512 for a hook without fuel.
        HookRules {
            allow: settings.integer("jp_allowHook"),
            speed: settings.integer("jp_hookSpeed"),
            flood_protect: settings.integer("jp_hookFloodProtect"),
            fuel_free: settings.integer("dmflags") & DF_NO_HOOK_FUEL != 0,
        }
    }

    fn combat(&self, settings: &dyn Settings) -> ModCombat {
        combat::combat(settings)
    }

    fn force(&self, settings: &dyn Settings) -> ForceRules {
        let dialect = ServerDialect::JaPlus {
            capabilities: JaPlusCapabilities(cinfo::cinfo(settings)),
        };
        ForceRules {
            // VisionJK `codemp/game/w_force.c:628-634`: only 0 keeps them out.
            throw_knocked_down: settings.integer("jp_pushPullKnockDown") != 0,
            // `w_force.c:3729-3732`: `OnSameTeam && !g_friendlyFire` passes over.
            throw_teammates: settings.integer("g_friendlyFire") != 0,
            // `w_force.c:870-875`: above 0 the continuous powers' gain is scaled.
            absorb_gain_scale: settings.value("jp_fixAbsorb").max(0.0),
            // `w_force.c:3146-3184`, `3736-3739`: items flung from 1, dropped flags at 2
            // (latched: a change waits for the next map).
            push_items: settings.integer("jp_pushAll").clamp(0, 255) as u8,
            // `w_force.c:6029-6051`.
            grip_open_saber: match settings.integer("jp_gripOpenSaber") {
                1 => GripOpenSaber::ThrownOnly,
                2 => GripOpenSaber::LowForceDrops,
                _ => GripOpenSaber::Off,
            },
            // `w_force.c:1474-1478`: `JP_FIXROLL()`, the level announced in `jp_cinfo`.
            grip_while_rolling: RollRules::from_dialect(&dialect).fix_roll > 0,
            // `codemp/game/g_main.c:3348-3360`: only 1 drains.
            cloak_drains: settings.integer("jp_cloakFuel") == 1,
        }
    }

    fn duels(&self, settings: &dyn Settings) -> DuelRules {
        // VisionJK `g_cmds.c:3262-3452`, `g_active.c:2439-2572`, `japlus/g_japlus_dim.c`,
        // `w_force.c:6256-6257`; the settings' meanings are its web panel's
        // (`webadmin/wa_settings_table.h:130,241-246`).
        DuelRules {
            ja_plus: true,
            // `jp_DuelAlpha` -1 switches the isolation off, as its clients read it
            // (`cgs.jpDuelAlpha >= 0`, VisionJK `cgame/cg_predict.c:478`).
            isolation: settings.integer("jp_DuelAlpha") >= 0,
            team_duels: settings.integer("jp_allowTeamDuel") != 0,
            distance_limit: settings.integer("jp_removeDistanceDuelLimit") == 0,
            bow: settings.integer("jp_autoDuelBOW") == 1,
            start_armor: settings.integer("jp_duelStartArmor"),
            winner_report: WinnerReport::from_setting(settings.integer("jp_endDuelBroadcastMSG")),
            force_regen_time: Some(settings.integer("jp_privateDuelForceRegen")),
        }
    }

    fn command(&self, name: &[u8]) -> Option<ModCommand> {
        // JA+'s own commands (`engage_FullForceDuel` among them), then the admin system's
        // a player types (`JP_ClientCommand`); those not carried out yet stay unknown.
        commands::command(name).or_else(|| {
            admin::SYSTEM
                .entry(name, false)
                .and_then(|entry| entry.command)
                .map(ModCommand::Admin)
        })
    }

    fn player_commands(&self, settings: &dyn Settings) -> PlayerCommandRules {
        commands::rules(settings)
    }

    fn command_prints(&self, command: ModCommand, settings: &dyn Settings) -> Vec<String> {
        commands::prints(command, settings)
    }

    fn client_has_plugin(&self, userinfo: &[u8]) -> bool {
        commands::has_plugin(userinfo)
    }

    fn console_command(&self, name: &[u8], level_time: i32) -> Option<String> {
        commands::console(name, level_time)
    }

    fn admin(&self) -> Option<&'static sjk_server_mod::admin::AdminSystem> {
        Some(&admin::SYSTEM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::{InfoString, ServerProfile};
    use std::collections::HashMap;

    struct Table(HashMap<&'static str, i32>);

    impl Settings for Table {
        fn integer(&self, name: &str) -> i32 {
            self.0.get(name).copied().unwrap_or_else(|| {
                cvars::CVARS
                    .iter()
                    .find(|cvar| cvar.name == name)
                    .map_or(0, |cvar| cvar.default.parse().unwrap_or(0))
            })
        }
        fn value(&self, name: &str) -> f32 {
            match self.0.get(name) {
                Some(value) => *value as f32,
                None => cvars::CVARS
                    .iter()
                    .find(|cvar| cvar.name == name)
                    .map_or(0.0, |cvar| cvar.default.parse().unwrap_or(0.0)),
            }
        }
    }

    #[test]
    fn clients_read_the_identity_as_ja_plus() {
        let info = format!(r"\gamename\{GAMENAME}\V\{VERSION}\jp_cinfo\19");
        let profile = ServerProfile::from_server_info(&InfoString::parse(&info).unwrap());
        assert_eq!(
            profile.dialect,
            ServerDialect::JaPlus {
                capabilities: JaPlusCapabilities(19)
            }
        );
    }

    #[test]
    fn the_server_moves_as_its_clients_predict() {
        let settings = Table(HashMap::new());
        let derived = JaPlus.derived_info(&settings);
        let info = format!(
            r"\gamename\{GAMENAME}\V\{VERSION}\jp_cinfo\{}",
            derived[0].1
        );
        let profile = ServerProfile::from_server_info(&InfoString::parse(&info).unwrap());
        let movement = JaPlus.movement(&settings);
        // The emotes' freeze is the server's own, for the clients that do not predict.
        assert_eq!(
            JaPlusRules {
                emote_freeze: EmoteFreeze::Off,
                ..movement.ja_plus
            },
            JaPlusRules::from_dialect(&profile.dialect)
        );
        assert_eq!(
            movement.fix_roll,
            RollRules::from_dialect(&profile.dialect).fix_roll
        );
        assert!(movement.ja_plus.enabled && movement.ja_plus.flip_kick);
        assert!(movement.ja_plus.yellow_dfa && !movement.ja_plus.head_slide);
        assert_eq!(movement.fix_roll, 1);
        assert_eq!(movement.legacy_fixes, 0);
    }

    #[test]
    fn jp_fix_emot_picks_what_the_server_freezes() {
        let mut settings = Table(HashMap::new());
        assert_eq!(
            JaPlus.movement(&settings).ja_plus.emote_freeze,
            EmoteFreeze::AtEase
        );
        settings.0.insert("jp_fixEmot", 0);
        assert_eq!(
            JaPlus.movement(&settings).ja_plus.emote_freeze,
            EmoteFreeze::Held
        );
    }

    #[test]
    fn dmflags_256_keeps_the_old_collision() {
        let mut settings = Table(HashMap::new());
        assert!(!JaPlus.movement(&settings).ja_plus.old_collision);
        settings.0.insert("dmflags", 256 | 8);
        assert!(JaPlus.movement(&settings).ja_plus.old_collision);
    }

    #[test]
    fn the_duels_follow_the_settings() {
        let mut settings = Table(HashMap::new());
        let rules = JaPlus.duels(&settings);
        // JA+'s defaults: isolated, team duels, no distance limit, the bow, 100 armour,
        // everyone told, 200 ms a Force point.
        assert!(rules.ja_plus && rules.isolation && rules.team_duels && rules.bow);
        assert!(!rules.distance_limit);
        assert_eq!(rules.start_armor, 100);
        assert_eq!(rules.winner_report, WinnerReport::Everyone);
        assert_eq!(rules.force_regen_time, Some(200));
        settings.0.insert("jp_DuelAlpha", -1);
        settings.0.insert("jp_allowTeamDuel", 0);
        settings.0.insert("jp_removeDistanceDuelLimit", 0);
        settings.0.insert("jp_autoDuelBOW", 0);
        settings.0.insert("jp_endDuelBroadcastMSG", 2);
        let rules = JaPlus.duels(&settings);
        assert!(!rules.isolation && !rules.team_duels && !rules.bow);
        assert!(rules.distance_limit);
        assert_eq!(rules.winner_report, WinnerReport::Duellists);
        settings.0.insert("jp_DuelAlpha", 0);
        settings.0.insert("jp_endDuelBroadcastMSG", 0);
        let rules = JaPlus.duels(&settings);
        assert!(rules.isolation);
        assert_eq!(rules.winner_report, WinnerReport::Silent);
    }

    #[test]
    fn the_hook_follows_its_settings_and_is_predicted() {
        let mut settings = Table(HashMap::new());
        let hook = JaPlus.hook(&settings);
        assert_eq!(
            hook,
            HookRules {
                allow: 1,
                speed: 800,
                flood_protect: 750,
                fuel_free: false
            }
        );
        assert_eq!(
            JaPlus.movement(&settings).grapple,
            Some(GrappleRules { pull_speed: 800.0 })
        );
        settings.0.insert("jp_allowHook", 0);
        settings.0.insert("dmflags", 512);
        settings.0.insert("jp_hookSpeed", 1_600);
        let hook = JaPlus.hook(&settings);
        assert!(!hook.enabled() && hook.fuel_free);
        assert_eq!(hook.speed, 1_600);
    }

    #[test]
    fn the_full_force_duel_is_a_command() {
        assert_eq!(
            JaPlus.command(b"engage_fullforceduel"),
            Some(ModCommand::FullForceDuel)
        );
        assert_eq!(JaPlus.command(b"engage_duel"), None);
    }

    #[test]
    fn admin_commands_are_commands_and_the_unbuilt_ones_are_not() {
        use sjk_server_mod::admin::AdminCommand;
        assert_eq!(
            JaPlus.command(b"AMLOGIN"),
            Some(ModCommand::Admin(AdminCommand::Login))
        );
        assert_eq!(JaPlus.command(b"amghost"), None);
        assert_eq!(JaPlus.command(b"admGrant"), None);
        assert!(JaPlus.admin().is_some());
    }

    #[test]
    fn a_rule_turned_off_is_off_for_the_server_too() {
        let mut settings = Table(HashMap::new());
        settings.0.insert("jp_allowflipkick", 0);
        settings.0.insert("jp_fixRoll", 0);
        let movement = JaPlus.movement(&settings);
        assert!(movement.ja_plus.enabled && !movement.ja_plus.flip_kick);
        assert_eq!(movement.fix_roll, 0);
    }

    #[test]
    fn the_grip_speed_is_the_setting_the_clients_read() {
        let mut settings = Table(HashMap::new());
        assert_eq!(JaPlus.movement(&settings).ja_plus.grip_speed, 0.8);
        settings.0.insert("jp_gripSpeedScale", 1);
        assert_eq!(JaPlus.movement(&settings).ja_plus.grip_speed, 1.0);
    }

    #[test]
    fn ja_plus_force_rules_at_its_defaults() {
        let settings = Table(HashMap::new());
        let rules = JaPlus.force(&settings);
        assert!(rules.throw_knocked_down);
        assert!(!rules.throw_teammates);
        assert_eq!(rules.absorb_gain_scale, 2.0);
        assert_eq!(rules.push_items, 1);
        assert_eq!(rules.grip_open_saber, GripOpenSaber::Off);
        assert!(rules.grip_while_rolling);
        assert!(rules.cloak_drains);
    }

    #[test]
    fn ja_plus_force_rules_follow_their_settings() {
        let mut settings = Table(HashMap::new());
        settings.0.insert("jp_pushPullKnockDown", 0);
        settings.0.insert("g_friendlyFire", 1);
        settings.0.insert("jp_fixAbsorb", 0);
        settings.0.insert("jp_gripOpenSaber", 2);
        settings.0.insert("jp_fixRoll", 0);
        settings.0.insert("jp_cloakFuel", 0);
        settings.0.insert("jp_pushAll", 2);
        let rules = JaPlus.force(&settings);
        assert_eq!(rules.push_items, 2);
        assert!(!rules.throw_knocked_down && rules.throw_teammates);
        assert_eq!(rules.absorb_gain_scale, 0.0);
        assert_eq!(rules.grip_open_saber, GripOpenSaber::LowForceDrops);
        assert!(!rules.grip_while_rolling && !rules.cloak_drains);
        settings.0.insert("jp_gripOpenSaber", 1);
        settings.0.insert("jp_cloakFuel", 2);
        let rules = JaPlus.force(&settings);
        assert_eq!(rules.grip_open_saber, GripOpenSaber::ThrownOnly);
        assert!(!rules.cloak_drains);
    }
}
