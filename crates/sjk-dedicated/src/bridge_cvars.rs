//! The server's console variables at work: what a changed value does, checked after
//! every console line and every frame, as `G_UpdateCvars` and `SV_Frame`'s serverinfo
//! check do it.

use super::{NativeGame, Told};
use crate::cvars::{CVAR_INTERNAL, CVAR_SERVERINFO, Cvars};
use sjk_network::{LegacyOobRates, LegacySessionSettings};

impl NativeGame {
    /// The console variables, for the process to read what belongs to the endpoint
    /// (passwords, timeouts, rates, `sv_fps`).
    pub fn cvars(&self) -> &Cvars {
        &self.cvars
    }

    /// `duel_fraglimit`: the duels a player must win to end a tournament, 0 for none.
    pub fn set_duel_fraglimit(&mut self, duel_fraglimit: i32) {
        self.set_cvar("duel_fraglimit", &duel_fraglimit.to_string());
    }

    /// The limits that end a match — `fraglimit`, `timelimit`, `capturelimit` — set as
    /// the console variables they are.
    pub fn set_limits(&mut self, fraglimit: i32, timelimit: f32, capturelimit: i32) {
        self.set_cvar("fraglimit", &fraglimit.to_string());
        self.set_cvar("timelimit", &timelimit.to_string());
        self.set_cvar("capturelimit", &capturelimit.to_string());
    }

    /// `Cvar_Set`: the process's or the code's own change, past every protection, put
    /// into effect at once. How the command line's options reach the server.
    pub fn set_cvar(&mut self, name: &str, value: &str) {
        self.cvars.set(name.as_bytes(), value.as_bytes());
        self.apply_cvars();
    }

    /// Put every variable changed since the last look into effect: the game's own
    /// settings, the server-info string everyone is sent, and the announcement of a
    /// tracked variable (`print "Server: %s changed to %s\n"`).
    pub(super) fn apply_cvars(&mut self) {
        self.update_mod_info();
        let mut seen = std::mem::take(&mut self.cvars_seen);
        let changed = self.cvars.changed_since(&mut seen);
        self.cvars_seen = seen;
        if !changed.is_empty() {
            self.cvars_generation += 1;
            self.update_mod_movement();
            self.mod_combat = crate::server_mods::combat(self.server_mod, &self.cvars);
            self.mod_force = crate::server_mods::force(self.server_mod, &self.cvars);
        }
        for name in changed {
            let Some(var) = self.cvars.var(&name) else {
                continue;
            };
            let (value, flags, announced, integer) =
                (var.string.clone(), var.flags, var.announced, var.integer());
            let text = String::from_utf8_lossy(&value).into_owned();
            let key = String::from_utf8_lossy(&name).into_owned();
            match key.to_ascii_lowercase().as_str() {
                "sv_hostname" => self.identity.hostname = value.clone(),
                "fraglimit" => self.limits.fraglimit = integer,
                "timelimit" => self.limits.timelimit = var.value(),
                "capturelimit" => self.limits.capturelimit = integer,
                "duel_fraglimit" => self.limits.duel_fraglimit = integer,
                "g_allowvote" => self.vote_rules.0 = integer & super::bridge_votes::SUPPORTED_VOTES,
                "g_votedelay" => self.vote_rules.1 = integer,
                "g_teamautojoin" => self.settings.team_auto_join = integer != 0,
                "sv_cheats" => self.settings.cheats = integer != 0,
                "g_gametype" if integer != self.gametype => self.set_gametype(integer),
                _ => {}
            }
            // `SV_SetConfigstring(CS_SERVERINFO, Cvar_InfoString(CVAR_SERVERINFO))`, one
            // key at a time into the string this server keeps in its own order; the
            // registration's own values are the level's first string, told to nobody.
            if flags & CVAR_SERVERINFO != 0 && flags & CVAR_INTERNAL == 0 {
                self.set_server_info(&key, &text, self.cvars_primed);
            }
            // The first look is the registration's own, which announces nothing.
            if announced && self.cvars_primed {
                self.told.push(Told::Everyone(
                    format!("print \"Server: {key} changed to {text}\n\"").into_bytes(),
                ));
            }
        }
        self.cvars_primed = true;
    }

    /// The movement a player of this server moves by: the bare `Pmove`, with the rules of
    /// the running server mod (`crate::server_mods`).
    pub(super) fn movement_config(&self) -> sjk_game_jka::pmove::MovementConfig {
        let mut config = super::bridge_maps::authoritative();
        self.mod_movement.apply(&mut config);
        config
    }

    /// The Force rules every Force update and throw runs by: the running server mod's
    /// (`crate::server_mods`), the base game's without one.
    pub(super) fn force_rules(&self) -> sjk_game_jka::force_rules::ForceRules {
        self.mod_force
    }

    /// The server info the running mod works out from its settings (JA+'s `jp_cinfo`),
    /// written into those settings before the changes are looked at, so that it goes out
    /// with the change that caused it.
    fn update_mod_info(&mut self) {
        let Some(server_mod) = self.server_mod else {
            return;
        };
        for (name, value) in server_mod.derived_info(&self.cvars) {
            if self.cvars.string(name.as_bytes()) != value.as_bytes() {
                self.cvars.set(name.as_bytes(), value.as_bytes());
            }
        }
    }

    /// The movement rules the mod's settings select now, put into every player's moves
    /// at once, as the clients predict them from the server info just changed. The
    /// animation fixes are the mod's for the server's whole life, so the level's
    /// `CS_LEGACY_FIXES` stays as it is. Its private-duel rules follow too.
    fn update_mod_movement(&mut self) {
        self.duel_rules = crate::server_mods::duels(self.server_mod, &self.cvars);
        let movement = crate::server_mods::movement(self.server_mod, &self.cvars);
        if movement == self.mod_movement {
            return;
        }
        debug_assert_eq!(movement.legacy_fixes, self.mod_movement.legacy_fixes);
        self.mod_movement = movement;
        if let Some(world) = self.server.world_mut(self.world) {
            for (_, peer) in world.entities_mut() {
                let mut config = peer.movement.config();
                movement.apply(&mut config);
                peer.movement.set_config(config);
            }
        }
    }

    /// `Cvar_Get` of a latched variable when a level is built (`SV_Map_f`'s
    /// `g_gametype`, and the running mod's latched settings, which its game module
    /// registers again as every level starts): a change waiting for the next map takes
    /// effect.
    pub(super) fn take_latched_cvars(&mut self) {
        self.cvars.get(
            b"g_gametype",
            b"0",
            CVAR_SERVERINFO | crate::cvars::CVAR_LATCH,
            None,
        );
        if let Some(server_mod) = self.server_mod {
            for cvar in server_mod.cvars() {
                if cvar.flags & sjk_server_mod::flags::LATCH != 0 {
                    self.cvars.get(
                        cvar.name.as_bytes(),
                        cvar.default.as_bytes(),
                        cvar.flags | crate::cvars::CVAR_VM_CREATED,
                        None,
                    );
                }
            }
        }
        self.apply_cvars();
    }

    /// How many times the variables have changed; the process compares it to know when
    /// to read [`Self::endpoint_settings`] again.
    pub fn cvars_generation(&self) -> u64 {
        self.cvars_generation
    }

    /// The endpoint's share of the variables: the remote console's password, the
    /// private slots, the timeouts, flood protection, downloads, automatic demos and the
    /// rate and snapshot limits.
    pub fn endpoint_settings(&self, settings: &mut LegacySessionSettings) {
        let cvars = &self.cvars;
        settings.console.rcon_password = cvars.string(b"rconPassword").to_vec();
        // `status` says whether this is a LAN or a public server.
        settings.console.dedicated = cvars.integer(b"dedicated");
        settings.private_clients = cvars.integer(b"sv_privateClients");
        settings.private_password = cvars.string(b"sv_privatePassword").to_vec();
        settings.timeout_seconds = cvars.integer(b"sv_timeout");
        settings.zombie_seconds = cvars.integer(b"sv_zombietime");
        settings.reconnect_limit_seconds = cvars.integer(b"sv_reconnectlimit");
        settings.flood_protect = cvars.integer(b"sv_floodProtect");
        settings.legacy_fixes = cvars.integer(b"sv_legacyFixes") != 0;
        settings.allow_download = cvars.integer(b"sv_allowDownload") != 0;
        settings.pure = cvars.integer(b"sv_pure") != 0;
        settings.auto_whitelist = cvars.integer(b"sv_autoWhitelist") != 0;
        settings.auto_demo = sjk_network::LegacyAutoDemoSettings {
            enabled: cvars.integer(b"sv_autoDemo") != 0,
            max_maps: cvars.integer(b"sv_autoDemoMaxMaps"),
        };
        settings.oob_rates = LegacyOobRates {
            per_address: cvars.integer(b"sv_maxOOBRateIP"),
            global: cvars.integer(b"sv_maxOOBRate"),
        };
        let rates = &mut settings.rates;
        rates.lan_force_rate = cvars.integer(b"sv_lanForceRate") != 0;
        rates.rate_policy = cvars.integer(b"sv_ratePolicy");
        rates.client_rate = cvars.integer(b"sv_clientRate");
        rates.min_rate = cvars.integer(b"sv_minRate");
        rates.max_rate = cvars.integer(b"sv_maxRate");
        rates.snaps_policy = cvars.integer(b"sv_snapsPolicy");
        rates.snaps_min = cvars.integer(b"sv_snapsMin");
        rates.snaps_max = cvars.integer(b"sv_snapsMax");
        // Simulation/snapshot time now advances in complete frames, so LAN peers
        // can use the actual server cadence without skipping an early wall-clock frame.
        rates.fps = cvars.integer(b"sv_fps").max(1);
    }

    /// `SV_Frame`'s frame length: a thousand milliseconds over `sv_fps`, never below
    /// one; an `sv_fps` below 1 is set back to 10 first, as the reference repairs it.
    pub fn frame_msec(&mut self) -> i32 {
        if self.cvars.integer(b"sv_fps") < 1 {
            self.set_cvar("sv_fps", "10");
        }
        (1000 / self.cvars.integer(b"sv_fps")).max(1)
    }

    /// One `CVAR_SERVERINFO` value rewritten in `CS_SERVERINFO` and the status reply;
    /// `tell` when clients already hold the old one.
    pub(super) fn set_server_info(&mut self, key: &str, value: &str, tell: bool) {
        self.status_info = server_info_with(&self.status_info, key, value);
        let info = self.status_info.clone();
        if tell {
            self.publish_config_string(0, &info);
        } else if let Some(slot) = self
            .config_strings
            .iter_mut()
            .find(|(index, _)| *index == 0)
        {
            slot.1 = info;
        }
    }

    /// A console line for the cvar table: a cvar command or a variable by name.
    pub(super) fn cvar_line(&mut self, line: &[u8], print: &mut dyn FnMut(&[u8])) -> bool {
        let taken = self.cvars.command(line, print);
        if taken {
            self.apply_cvars();
        }
        taken
    }
}

/// `CS_SERVERINFO` with one key's value replaced — `mapname` on a map change, a limit
/// when a vote sets one — and the rest of that string, the engine's and the game's own,
/// left alone. A key the string does not hold yet is added at its end, if the string
/// stays within `MAX_INFO_STRING`.
pub(super) fn server_info_with(server_info: &[u8], key: &str, value: &str) -> Vec<u8> {
    let fields: Vec<&[u8]> = server_info.split(|byte| *byte == b'\\').collect();
    let mut rebuilt: Vec<u8> = Vec::with_capacity(server_info.len() + value.len());
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            rebuilt.push(b'\\');
        }
        if index >= 2 && index % 2 == 0 && fields[index - 1] == key.as_bytes() {
            rebuilt.extend_from_slice(value.as_bytes());
        } else {
            rebuilt.extend_from_slice(field);
        }
    }
    // `Info_SetValueForKey` refuses a pair that would reach `MAX_INFO_STRING`.
    let held = fields
        .iter()
        .skip(1)
        .step_by(2)
        .any(|known| *known == key.as_bytes());
    if !held && !value.is_empty() && rebuilt.len() + key.len() + value.len() + 2 < 1024 {
        rebuilt.extend_from_slice(format!("\\{key}\\{value}").as_bytes());
    }
    rebuilt
}

/// The `CVAR_SERVERINFO` variables a legacy cgame reads from `CS_SERVERINFO`, in the
/// order this server has always sent them; their values are the variables' own, filled
/// in when the table is first read.
pub(super) const SERVERINFO_DEFAULTS: [(&str, &str); 20] = [
    ("g_gametype", "0"),
    ("sv_fps", "20"),
    ("timelimit", "0"),
    ("fraglimit", "20"),
    ("capturelimit", "8"),
    ("duel_fraglimit", "10"),
    ("dmflags", "0"),
    ("g_needpass", "0"),
    ("g_jediVmerc", "0"),
    ("g_forcePowerDisable", "0"),
    ("g_weaponDisable", "0"),
    ("g_stepSlideFix", "1"),
    ("g_noSpecMove", "0"),
    ("g_debugMelee", "0"),
    ("g_showDuelHealths", "0"),
    ("g_siegeTeam1", "none"),
    ("g_siegeTeam2", "none"),
    ("g_siegeTeamSwitch", "1"),
    ("g_redTeam", "Empire"),
    ("g_blueTeam", "Rebellion"),
];

#[cfg(test)]
mod tests {
    use super::NativeGame;
    use crate::bridge::Identity;
    use crate::cvars::Cvars;
    use sjk_network::LegacyGameHost;
    use sjk_protocol::InfoString;

    /// A server without a map whose start had `g_mod` set to `server_mod`, as a
    /// `+set g_mod japlus` gives it.
    fn started(server_mod: &str) -> NativeGame {
        let mut cvars = Cvars::new();
        cvars.set(b"g_mod", server_mod.as_bytes());
        cvars.register_server();
        let identity = Identity {
            hostname: b"test".to_vec(),
            mapname: b"mp/ffa3".to_vec(),
        };
        NativeGame::with_cvars(identity, None, 4, 4, cvars).unwrap()
    }

    fn info(game: &NativeGame, key: &str) -> Option<String> {
        let text = String::from_utf8(game.status_info.clone()).unwrap();
        InfoString::parse(&text)
            .unwrap()
            .get(key)
            .map(str::to_owned)
    }

    fn legacy_fixes(game: &NativeGame) -> Vec<u8> {
        let index = sjk_game_jka::pmove_rules::CS_LEGACY_FIXES;
        let (_, value) = game
            .config_strings
            .iter()
            .find(|(known, _)| *known == index)
            .unwrap();
        value.clone()
    }

    #[test]
    fn the_base_game_announces_and_moves_as_before() {
        let game = started("base");
        assert_eq!(info(&game, "gamename").as_deref(), Some("basejka"));
        assert_eq!(info(&game, "jp_cinfo"), None);
        assert!(!game.movement_config().ja_plus.enabled);
        assert_eq!(legacy_fixes(&game), b"7");
        assert!(game.cvars.var(b"jp_allowflipkick").is_none());
        assert_eq!(game.server_info().game_directory, b"");
    }

    #[test]
    fn ja_plus_announces_itself_and_moves_by_what_it_announces() {
        let game = started("japlus");
        assert_eq!(info(&game, "gamename").as_deref(), Some("JA+ Mod v2.4 B7"));
        assert_eq!(info(&game, "V").as_deref(), Some("2.4B7"));
        assert_eq!(info(&game, "sjk").as_deref(), Some("1"));
        assert_eq!(game.server_info().game_directory, b"japlus");
        // Flip kick, roll fix 1 and the improved yellow DFA: JA+'s defaults that run.
        assert_eq!(info(&game, "jp_cinfo").as_deref(), Some("19"));
        assert_eq!(info(&game, "jp_gripSpeedScale").as_deref(), Some("0.8"));
        let config = game.movement_config();
        assert!(config.ja_plus.enabled && config.ja_plus.flip_kick && config.ja_plus.yellow_dfa);
        assert_eq!(config.roll_rules.fix_roll, 1);
        assert!(config.authoritative);
        // JA+ runs none of OpenJK's animation fixes, and says so.
        assert_eq!(config.legacy_fixes, 0);
        assert_eq!(legacy_fixes(&game), b"0");
    }

    #[test]
    fn a_ja_plus_setting_changed_in_game_reaches_the_info_and_the_moves() {
        let mut game = started("JAPLUS");
        game.set_cvar("jp_allowflipkick", "0");
        game.set_cvar("jp_fixRoll", "3");
        assert_eq!(info(&game, "jp_cinfo").as_deref(), Some("30"));
        let config = game.movement_config();
        assert!(!config.ja_plus.flip_kick);
        assert_eq!(config.roll_rules.fix_roll, 3);
    }

    #[test]
    fn the_combat_rules_follow_the_mod_and_its_settings() {
        let game = started("base");
        assert_eq!(game.mod_combat, sjk_server_mod::ModCombat::default());
        let mut game = started("japlus");
        let saber = game.mod_combat.saber;
        assert!(saber.enabled && !saber.move_damage && !saber.touch_damage);
        assert!(game.mod_combat.kill.suicide_credits_last_attacker);
        assert!(game.mod_combat.kill.refused_in_team_games);
        game.set_cvar("jp_alterDMG", "1");
        game.set_cvar("jp_SingleFastOtherAttack", "20");
        game.set_cvar("jp_allowTeamKill", "1");
        let saber = game.mod_combat.saber;
        assert!(saber.move_damage && saber.halved_on_sabers);
        assert_eq!(saber.moves.fast_other, 20);
        assert_eq!(saber.throw_scale, Some(1.0));
        assert!(!game.mod_combat.kill.refused_in_team_games);
    }

    #[test]
    fn the_duel_rules_are_the_base_games_or_follow_the_ja_plus_settings() {
        let game = started("base");
        assert_eq!(game.duel_rules, sjk_game_jka::duel::DuelRules::default());
        let mut game = started("japlus");
        assert!(game.duel_rules.ja_plus && game.duel_rules.isolation);
        assert_eq!(info(&game, "jp_DuelAlpha").as_deref(), Some("100"));
        game.set_cvar("jp_DuelAlpha", "-1");
        game.set_cvar("jp_removeDistanceDuelLimit", "0");
        assert!(!game.duel_rules.isolation && game.duel_rules.distance_limit);
        assert_eq!(info(&game, "jp_DuelAlpha").as_deref(), Some("-1"));
    }

    #[test]
    fn g_mod_waits_for_the_next_start_and_an_unknown_one_is_the_base_game() {
        let mut game = started("japlus");
        game.cvar_line(b"g_mod base", &mut |_| {});
        assert_eq!(info(&game, "gamename").as_deref(), Some("JA+ Mod v2.4 B7"));
        assert!(game.movement_config().ja_plus.enabled);
        let game = started("nonesuch");
        assert_eq!(info(&game, "gamename").as_deref(), Some("basejka"));
        assert!(!game.movement_config().ja_plus.enabled);
    }

    #[test]
    fn the_base_game_keeps_its_force_rules_whatever_ja_plus_settings_say() {
        let mut game = started("base");
        let base = sjk_game_jka::force_rules::ForceRules::default();
        assert_eq!(game.force_rules(), base);
        game.set_cvar("g_friendlyFire", "1");
        game.set_cvar("jp_pushPullKnockDown", "1");
        game.set_cvar("jp_cloakFuel", "0");
        assert_eq!(game.force_rules(), base);
        // Gripping keeps the base game's 0.4.
        assert_eq!(info(&game, "jp_gripSpeedScale"), None);
        assert!(!game.movement_config().ja_plus.enabled);
    }

    #[test]
    fn ja_plus_force_rules_change_with_their_settings_at_once() {
        use sjk_game_jka::force_rules::GripOpenSaber;
        let mut game = started("japlus");
        let rules = game.force_rules();
        assert!(rules.throw_knocked_down && !rules.throw_teammates);
        assert_eq!(rules.absorb_gain_scale, 2.0);
        assert!(rules.grip_while_rolling && rules.cloak_drains);
        game.set_cvar("g_friendlyFire", "1");
        game.set_cvar("jp_gripOpenSaber", "2");
        game.set_cvar("jp_fixRoll", "0");
        game.set_cvar("jp_cloakFuel", "0");
        let rules = game.force_rules();
        assert!(rules.throw_teammates && !rules.grip_while_rolling && !rules.cloak_drains);
        assert_eq!(rules.grip_open_saber, GripOpenSaber::LowForceDrops);
    }

    #[test]
    fn ja_plus_grip_speed_reaches_the_info_and_the_moves() {
        let mut game = started("japlus");
        assert_eq!(game.movement_config().ja_plus.grip_speed, 0.8);
        game.set_cvar("jp_gripSpeedScale", "1");
        assert_eq!(info(&game, "jp_gripSpeedScale").as_deref(), Some("1"));
        assert_eq!(game.movement_config().ja_plus.grip_speed, 1.0);
        // What a client reads from that info is what the server moves by (but the held
        // emotes' freeze, the server's own for the clients that do not predict them).
        let mut state = sjk_protocol::GameState::empty_local(0);
        state
            .replace_config_string(0, game.status_info.clone())
            .unwrap();
        assert_eq!(
            sjk_game_jka::pmove_japlus::JaPlusRules::from_game_state(&state),
            sjk_game_jka::pmove_japlus::JaPlusRules {
                emote_freeze: sjk_game_jka::pmove_japlus_emotes::EmoteFreeze::Off,
                ..game.movement_config().ja_plus
            }
        );
    }

    #[test]
    fn a_latched_ja_plus_setting_waits_for_the_next_map() {
        let mut game = started("japlus");
        assert_eq!(game.force_rules().push_items, 1);
        game.cvar_line(b"jp_pushAll 0", &mut |_| {});
        assert_eq!(game.force_rules().push_items, 1);
        game.take_latched_cvars();
        assert_eq!(game.force_rules().push_items, 0);
    }
}
