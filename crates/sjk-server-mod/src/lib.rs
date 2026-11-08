//! Server mods: rule sets SJK's dedicated server can run in place of the base game's,
//! as a JKA server runs a mod's game module (JA+ in `japlus`, for example).
//!
//! A mod is one [`ServerMod`]. The server (`sjk-dedicated`) keeps the table of the mods
//! it offers and runs the one its `g_mod` setting names; with none, it is the base game.
//! What a mod may change goes through the hooks of this crate, so the server's own code
//! never asks which mod runs: it asks the running mod.
//!
//! The rule a mod follows for movement: the server runs what its server info tells the
//! clients to predict. A JA+ server announces its rules in `jp_cinfo`, every JA+ client
//! (SJK's, EternalJK, the JA+ plugin) predicts from it, and the server's own moves follow
//! the same bits ([`ServerMod::movement`]), so no client is pulled back for a rule it was
//! told about.

use sjk_game_jka::duel::DuelRules;
use sjk_game_jka::force_rules::ForceRules;
use sjk_game_jka::pmove::MovementConfig;
use sjk_game_jka::pmove_japlus::JaPlusRules;
use sjk_game_jka::saber_damage_japlus::JaPlusSaberDamage;

/// The cvar flags a mod's settings use, the bits of the engine's `cvar.cpp`
/// (`q_shared.h`), which the server's cvar table keeps as they are.
pub mod flags {
    /// `CVAR_ARCHIVE`: saved to the server's configuration file.
    pub const ARCHIVE: u32 = 0x1;
    /// `CVAR_SERVERINFO`: part of the server info string clients and browsers read.
    pub const SERVERINFO: u32 = 0x4;
    /// `CVAR_SYSTEMINFO`: copied to every client.
    pub const SYSTEMINFO: u32 = 0x8;
    /// `CVAR_LATCH`: a change waits for the next map.
    pub const LATCH: u32 = 0x20;
    /// `CVAR_ROM`: set by the server only.
    pub const ROM: u32 = 0x40;
    /// `CVAR_INTERNAL`: never shown in an info string (passwords).
    pub const INTERNAL: u32 = 0x800;
}

/// One setting a mod registers when it runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModCvar {
    pub name: &'static str,
    /// The value it starts with, the mod's own default.
    pub default: &'static str,
    /// [`flags`] bits.
    pub flags: u32,
    /// Every change is announced to the players (`Server: <name> changed to <value>`).
    pub announced: bool,
}

/// The server's settings as a mod reads them.
pub trait Settings {
    /// `Cvar_VariableIntegerValue`: 0 for a setting that does not exist.
    fn integer(&self, name: &str) -> i32;
    /// `Cvar_VariableValue`: 0 for a setting that does not exist.
    fn value(&self, name: &str) -> f32;
}

/// The movement rules a mod runs, which the server puts into every player's moves.
/// [`Default`] is the base game's, as SJK's server runs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModMovement {
    /// JA+'s rules (`MovementConfig::ja_plus`); off on the base game.
    pub ja_plus: JaPlusRules,
    /// The roll fix in force (`RollRules::fix_roll`, 0 to 3); 0 on the base game.
    pub fix_roll: u8,
    /// Which of OpenJK's animation fixes run (`MovementConfig::legacy_fixes`, published
    /// in `CS_LEGACY_FIXES`, which clients predict from).
    pub legacy_fixes: u32,
}

impl Default for ModMovement {
    fn default() -> Self {
        let base = MovementConfig::default();
        Self {
            ja_plus: base.ja_plus,
            fix_roll: base.roll_rules.fix_roll,
            legacy_fixes: base.legacy_fixes,
        }
    }
}

impl ModMovement {
    /// `config` with these rules, the rest of it (a player's sabers, its replay state's
    /// policy) left alone.
    pub fn apply(self, config: &mut MovementConfig) {
        config.ja_plus = self.ja_plus;
        config.roll_rules.fix_roll = self.fix_roll;
        config.legacy_fixes = self.legacy_fixes;
    }
}

/// The combat rules a mod runs: what only the server decides (damage, frags), which
/// no client predicts. [`Default`] is the base game's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModCombat {
    /// The saber's damage (`sjk_game_jka::saber_damage_japlus`), for players' and NPCs'
    /// blades alike.
    pub saber: JaPlusSaberDamage,
    /// The `kill` command and whom a suicide is put down to.
    pub kill: KillRules,
}

/// How the `kill` command and suicides go. [`Default`] is the base game's: `kill` always
/// works and a suicide is the player's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KillRules {
    /// A suicide (`MOD_SUICIDE`: `kill`, `killother`) is the frag of whoever last pushed
    /// or hit the player (`ps.otherKiller`, credited for five seconds), as a fall to death
    /// is in the base game.
    pub suicide_credits_last_attacker: bool,
    /// `kill` does nothing during a private duel.
    pub refused_in_duel: bool,
    /// `kill` does nothing in the team games (`GT_TEAM` and after).
    pub refused_in_team_games: bool,
}

/// `MOD_SUICIDE`, `GT_TEAM`.
const MOD_SUICIDE: u32 = sjk_game_jka::means_of_death::MOD_SUICIDE;
const GT_TEAM: i32 = 6;

impl KillRules {
    /// Whether `player_die` puts the death of `client` by `means` down to whoever last
    /// pushed or hit it: under these rules a suicide the player itself or the world dealt
    /// (`self == attacker || !attacker->client`), never one an NPC or another player did.
    pub fn credits_last_attacker(
        &self,
        client: u16,
        means: u32,
        attacker: Option<u16>,
        npc_attacker: bool,
    ) -> bool {
        self.suicide_credits_last_attacker
            && means == MOD_SUICIDE
            && !npc_attacker
            && attacker.is_none_or(|attacker| attacker == client)
    }

    /// Whether the `kill` command does nothing, for a player `duelling` or not, in game
    /// type `gametype`.
    pub fn refuses_kill(&self, duelling: bool, gametype: i32) -> bool {
        (self.refused_in_duel && duelling) || (self.refused_in_team_games && gametype >= GT_TEAM)
    }
}

/// A player command a mod adds to the game's own (`ClientCommand`'s table), which the
/// server carries out with its shared code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModCommand {
    /// The duel key for a full-force duel, where the Force works between the two
    /// (JA+'s `engage_FullForceDuel`, [`sjk_game_jka::duel::Duellist::full_force`]); not
    /// at the intermission (`CMD_NOINTERMISSION`).
    FullForceDuel,
}

/// A rule set the server can run in place of the base game.
pub trait ServerMod: Sync {
    /// Its `g_mod` value (`japlus`): letters, digits and `_` only.
    fn id(&self) -> &'static str;
    /// The name operators and players know it by (`JA+`).
    fn name(&self) -> &'static str;
    /// The settings it registers when it runs, at its own defaults.
    fn cvars(&self) -> &'static [ModCvar];
    /// The read-only server info it announces itself with (`gamename`, a version key),
    /// which clients and server browsers tell the mod by.
    fn identity(&self) -> &'static [(&'static str, &'static str)];
    /// The mod folder its servers report to server browsers (`getinfo`'s `game`, which
    /// is a JKA server's `fs_game`): browsers list and filter mods by it. Only the
    /// report; clients load no files of that folder for it.
    fn game_directory(&self) -> &'static str;
    /// Server info worked out from its settings (JA+'s `jp_cinfo`), as `(setting,
    /// value)`. The server asks again whenever a setting changes and writes the values
    /// into those settings, which the mod registers read-only ([`Self::cvars`]).
    fn derived_info(&self, settings: &dyn Settings) -> Vec<(&'static str, String)>;
    /// The movement rules its settings select: those its server info tells clients to
    /// predict, never more.
    fn movement(&self, settings: &dyn Settings) -> ModMovement;
    /// The combat rules its settings select (saber damage, `kill`): the base game's unless
    /// the mod changes them.
    fn combat(&self, settings: &dyn Settings) -> ModCombat {
        let _ = settings;
        ModCombat::default()
    }

    /// The Force rules its settings select (who a push reaches, what absorbing gains, a
    /// gripped player's saber, the cloak's battery...), which the server puts into every
    /// Force update and throw, its NPCs' too. The base game's by default. The server asks
    /// again whenever a setting changes.
    fn force(&self, _settings: &dyn Settings) -> ForceRules {
        ForceRules::default()
    }

    /// The private-duel rules its settings select; the base game's by default.
    fn duels(&self, _settings: &dyn Settings) -> DuelRules {
        DuelRules::default()
    }
    /// The player command `name` (any letter case) is in its table, if it adds one;
    /// none by default.
    fn command(&self, _name: &[u8]) -> Option<ModCommand> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Plain;

    impl ServerMod for Plain {
        fn id(&self) -> &'static str {
            "plain"
        }
        fn name(&self) -> &'static str {
            "Plain"
        }
        fn cvars(&self) -> &'static [ModCvar] {
            &[]
        }
        fn identity(&self) -> &'static [(&'static str, &'static str)] {
            &[]
        }
        fn game_directory(&self) -> &'static str {
            ""
        }
        fn derived_info(&self, _: &dyn Settings) -> Vec<(&'static str, String)> {
            Vec::new()
        }
        fn movement(&self, _: &dyn Settings) -> ModMovement {
            ModMovement::default()
        }
    }

    struct NoSettings;

    impl Settings for NoSettings {
        fn integer(&self, _: &str) -> i32 {
            0
        }
        fn value(&self, _: &str) -> f32 {
            0.0
        }
    }

    #[test]
    fn a_mod_without_force_rules_plays_the_base_games() {
        assert_eq!(Plain.force(&NoSettings), ForceRules::default());
    }

    #[test]
    fn the_base_game_kills_as_before() {
        let base = ModCombat::default();
        assert_eq!(base.saber, JaPlusSaberDamage::default());
        assert!(!base.saber.enabled);
        assert!(
            !base
                .kill
                .credits_last_attacker(3, MOD_SUICIDE, Some(3), false)
        );
        assert!(!base.kill.refuses_kill(true, GT_TEAM));
    }

    #[test]
    fn only_a_suicide_of_ones_own_or_the_worlds_is_credited() {
        let rules = KillRules {
            suicide_credits_last_attacker: true,
            ..KillRules::default()
        };
        assert!(rules.credits_last_attacker(3, MOD_SUICIDE, Some(3), false));
        assert!(rules.credits_last_attacker(3, MOD_SUICIDE, None, false));
        // Somebody else's kill, an NPC's, or another means stays as it is.
        assert!(!rules.credits_last_attacker(3, MOD_SUICIDE, Some(4), false));
        assert!(!rules.credits_last_attacker(3, MOD_SUICIDE, Some(40), true));
        assert!(!rules.credits_last_attacker(3, MOD_SUICIDE + 1, Some(3), false));
    }

    #[test]
    fn kill_is_refused_where_the_rules_say() {
        let rules = KillRules {
            refused_in_duel: true,
            refused_in_team_games: true,
            ..KillRules::default()
        };
        assert!(rules.refuses_kill(true, 0));
        assert!(!rules.refuses_kill(false, 0));
        assert!(!rules.refuses_kill(false, 3));
        assert!(rules.refuses_kill(false, GT_TEAM));
        assert!(rules.refuses_kill(false, 8));
    }

    #[test]
    fn the_default_movement_is_the_base_servers() {
        let mut config = MovementConfig::default();
        let before = config;
        ModMovement::default().apply(&mut config);
        assert_eq!(config, before);
        assert!(!config.ja_plus.enabled);
    }

    #[test]
    fn applying_rules_keeps_the_rest_of_a_players_config() {
        let mut config = MovementConfig {
            saber_speed_scales: [0.5, 1.0],
            ..MovementConfig::default()
        };
        config.roll_rules.saber_forbids_rolls = true;
        ModMovement {
            ja_plus: JaPlusRules {
                enabled: true,
                ..JaPlusRules::default()
            },
            fix_roll: 2,
            legacy_fixes: 0,
        }
        .apply(&mut config);
        assert!(config.ja_plus.enabled);
        assert_eq!(config.roll_rules.fix_roll, 2);
        assert!(config.roll_rules.saber_forbids_rolls);
        assert_eq!(config.legacy_fixes, 0);
        assert_eq!(config.saber_speed_scales, [0.5, 1.0]);
    }
}
