//! JA+'s combat rules from its settings: the saber's damage (`jp_alterDMG` and the
//! per-move `jp_*Attack` table, `jp_SaberThrowScaleDMG`, `jp_reduceSaberBlock`,
//! `jp_allowSaberTouchDMG`) and the `kill` command (`jp_allowTeamKill`, the frag of a
//! `/kill` going to the last attacker).
//!
//! The evidence is VisionJK (`codemp/game/w_saber.c`, `g_cmds.c`, `g_combat.c`, the
//! changes marked `JA+`); the rules themselves run in the shared saber damage
//! (`sjk_game_jka::saber_damage_japlus`) and the server's `kill`.

use sjk_game_jka::saber_damage_japlus::{DamageRange, JaPlusSaberDamage, MoveDamage};
use sjk_server_mod::{KillRules, ModCombat, Settings};

/// The rules the settings select now.
pub fn combat(settings: &dyn Settings) -> ModCombat {
    ModCombat {
        saber: saber_damage(settings),
        kill: KillRules {
            // `player_die` takes `MOD_SUICIDE` with the falls (VisionJK `g_combat.c:2432-2438`).
            suicide_credits_last_attacker: true,
            // `Cmd_Kill_f` (VisionJK `g_cmds.c:548-559`): no `/kill` in a duel, nor in a team
            // game without `jp_allowTeamKill`.
            refused_in_duel: true,
            refused_in_team_games: settings.integer("jp_allowTeamKill") == 0,
        },
    }
}

/// `jp_alterDMG` and the settings it reads (VisionJK `w_saber.c:2284-2300, 4219-4330,
/// 4363, 4386, 4726, 4818`): 1 reads the table from the `jp_*Attack` settings, any other
/// value but 0 keeps JA+'s built-in table; only 1 scales the thrown saber and halves
/// blows at saber wielders.
fn saber_damage(settings: &dyn Settings) -> JaPlusSaberDamage {
    let alter = settings.integer("jp_alterDMG");
    let exact = alter == 1;
    JaPlusSaberDamage {
        enabled: true,
        move_damage: alter != 0,
        moves: if exact {
            moves(settings)
        } else {
            MoveDamage::default()
        },
        throw_scale: exact.then(|| settings.value("jp_SaberThrowScaleDMG")),
        halved_on_sabers: exact,
        reduce_block: settings.integer("jp_reduceSaberBlock"),
        touch_damage: settings.integer("jp_allowSaberTouchDMG") != 0,
    }
}

/// The table from the settings (`JP_AttackDamage`, `JP_FlatDamage`): a range whose
/// lowest is above its highest keeps JA+'s built-in one.
fn moves(settings: &dyn Settings) -> MoveDamage {
    let built_in = MoveDamage::default();
    let range = |name: &str, built_in: DamageRange| {
        let min = settings.integer(&format!("jp_{name}AttackMin"));
        let max = settings.integer(&format!("jp_{name}AttackMax"));
        if min <= max {
            DamageRange { min, max }
        } else {
            built_in
        }
    };
    let flat = |name: &str| settings.integer(&format!("jp_{name}Attack"));
    MoveDamage {
        stab_down: range("StabDown", built_in.stab_down),
        roll_stab: range("RollStab", built_in.roll_stab),
        staff_back: range("StaffBack", built_in.staff_back),
        dual_back: range("DualBack", built_in.dual_back),
        staff_dual_normal: range("StaffDualNormal", built_in.staff_dual_normal),
        staff_dual_spin: flat("StaffDualSpin"),
        new_spin_dfa: range("NewSpinDfa", built_in.new_spin_dfa),
        staff_dual_other: range("StaffDualOther", built_in.staff_dual_other),
        dual_kata: flat("DualKata"),
        staff_kata: range("StaffKata", built_in.staff_kata),
        new_pull: range("NewPull", built_in.new_pull),
        single_kata: range("SingleKata", built_in.single_kata),
        strong_normal: range("SingleStrongNormal", built_in.strong_normal),
        strong_dfa: range("SingleStrongDfa", built_in.strong_dfa),
        strong_back: range("SingleStrongBack", built_in.strong_back),
        strong_other: flat("SingleStrongOther"),
        medium_dfa: range("SingleMediumDfa", built_in.medium_dfa),
        medium_back: range("SingleMediumBack", built_in.medium_back),
        medium_other: flat("SingleMediumOther"),
        fast_lunge: range("SingleFastLunge", built_in.fast_lunge),
        fast_back: range("SingleFastBack", built_in.fast_back),
        fast_other: flat("SingleFastOther"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cvars::CVARS;
    use std::collections::HashMap;

    struct Table(HashMap<&'static str, &'static str>);

    impl Table {
        fn read(&self, name: &str) -> &str {
            self.0.get(name).copied().unwrap_or_else(|| {
                CVARS
                    .iter()
                    .find(|cvar| cvar.name == name)
                    .map_or("0", |cvar| cvar.default)
            })
        }
    }

    impl Settings for Table {
        fn integer(&self, name: &str) -> i32 {
            let text = self.read(name);
            text.parse()
                .unwrap_or_else(|_| text.parse::<f32>().map_or(0, |value| value as i32))
        }
        fn value(&self, name: &str) -> f32 {
            self.read(name).parse().unwrap_or(0.0)
        }
    }

    #[test]
    fn every_setting_the_table_reads_exists() {
        // The names the table builds must be JA+'s, or a typo would read 0.
        let names = [
            "StabDown",
            "RollStab",
            "StaffBack",
            "DualBack",
            "StaffDualNormal",
            "NewSpinDfa",
            "StaffDualOther",
            "StaffKata",
            "NewPull",
            "SingleKata",
            "SingleStrongNormal",
            "SingleStrongDfa",
            "SingleStrongBack",
            "SingleMediumDfa",
            "SingleMediumBack",
            "SingleFastLunge",
            "SingleFastBack",
        ];
        let known = |name: String| CVARS.iter().any(|cvar| cvar.name == name);
        for name in names {
            assert!(known(format!("jp_{name}AttackMin")), "{name}");
            assert!(known(format!("jp_{name}AttackMax")), "{name}");
        }
        for name in [
            "StaffDualSpin",
            "DualKata",
            "SingleStrongOther",
            "SingleMediumOther",
            "SingleFastOther",
        ] {
            assert!(known(format!("jp_{name}Attack")), "{name}");
        }
    }

    #[test]
    fn the_settings_defaults_are_the_built_in_table() {
        let settings = Table(HashMap::from([("jp_alterDMG", "1")]));
        assert_eq!(moves(&settings), MoveDamage::default());
    }

    #[test]
    fn jp_alter_dmg_0_keeps_the_sp_style() {
        let rules = combat(&Table(HashMap::new())).saber;
        assert!(rules.enabled && !rules.move_damage);
        assert_eq!(rules.throw_scale, None);
        assert!(!rules.halved_on_sabers && !rules.touch_damage);
        assert_eq!(rules.reduce_block, 0);
    }

    #[test]
    fn jp_alter_dmg_1_reads_the_settings_and_2_the_built_in_table() {
        let mut settings = Table(HashMap::from([
            ("jp_alterDMG", "1"),
            ("jp_SingleStrongNormalAttackMax", "80"),
            ("jp_SingleFastOtherAttack", "20"),
            // A lowest above the highest falls back to the built-in range.
            ("jp_RollStabAttackMin", "20"),
            ("jp_SaberThrowScaleDMG", "0.5"),
        ]));
        let rules = combat(&settings).saber;
        assert!(rules.move_damage && rules.halved_on_sabers);
        assert_eq!(rules.moves.strong_normal, DamageRange { min: 2, max: 80 });
        assert_eq!(rules.moves.fast_other, 20);
        assert_eq!(rules.moves.roll_stab, DamageRange { min: 2, max: 12 });
        assert_eq!(rules.throw_scale, Some(0.5));
        settings.0.insert("jp_alterDMG", "2");
        let rules = combat(&settings).saber;
        assert!(rules.move_damage && !rules.halved_on_sabers);
        assert_eq!(rules.moves, MoveDamage::default());
        assert_eq!(rules.throw_scale, None);
    }

    #[test]
    fn kill_follows_jp_allow_team_kill() {
        let mut settings = Table(HashMap::new());
        let kill = combat(&settings).kill;
        assert!(kill.suicide_credits_last_attacker && kill.refused_in_duel);
        assert!(kill.refused_in_team_games);
        settings.0.insert("jp_allowTeamKill", "1");
        assert!(!combat(&settings).kill.refused_in_team_games);
    }

    #[test]
    fn touch_and_block_settings_reach_the_rules() {
        let settings = Table(HashMap::from([
            ("jp_allowSaberTouchDMG", "1"),
            ("jp_reduceSaberBlock", "30"),
        ]));
        let rules = combat(&settings).saber;
        assert!(rules.touch_damage);
        assert_eq!(rules.reduce_block, 30);
    }
}
