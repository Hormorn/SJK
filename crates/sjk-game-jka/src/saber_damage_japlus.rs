//! JA+ 2.4 B7's saber damage rules, which a JA+ server mod puts into the shared blade
//! sweep ([`crate::saber_damage`]); [`JaPlusSaberDamage::default`] is the base game's,
//! every rule off.
//!
//! JA+ is closed source. The reference is VisionJK (Blackwolf's GPL-2.0 rebuild of the
//! JA+ server module, `codemp/game/w_saber.c`, the changes marked `JA+`); its behaviour is
//! ported here, not its code. SJK's server always runs `d_saberSPStyleDamage 1`, so of
//! JA+'s two damage trees (`jpTree = !(d_saberSPStyleDamage && !jp_alterDMG)`,
//! `w_saber.c:4219-4220`) the setting `jp_alterDMG` alone chooses: 0 keeps the base
//! game's SP-style damage, anything else gives each saber move the damage of JA+'s own
//! table ([`MoveDamage`], `JP_SaberAttackDamage`, `w_saber.c:2302-2394`).
//!
//! Only the server deals damage (clients do not predict it), so nothing here changes
//! what clients predict.

use crate::saber_move_data::movement as moves;

/// `SABER_HITDAMAGE`: a hit's damage where JA+'s table has no entry (the Desann and
/// Tavion styles), and the level 1 hit that a blow at a saber wielder cuts to 70 %.
pub const SABER_HITDAMAGE: i32 = 35;
/// `SS_FAST`, `SS_MEDIUM`, `SS_STRONG`, `SS_DUAL`, `SS_STAFF` (`ps.fd.saberAnimLevel`).
const SS_FAST: i32 = 1;
const SS_MEDIUM: i32 = 2;
const SS_STRONG: i32 = 3;
const SS_DUAL: i32 = 6;
const SS_STAFF: i32 = 7;

/// The lowest and the highest damage of a move whose damage ramps with its animation
/// (`G_GetAttackDamage`): the highest at the animation's peak, the lowest far from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageRange {
    pub min: i32,
    pub max: i32,
}

const fn range(min: i32, max: i32) -> DamageRange {
    DamageRange { min, max }
}

/// The damage of each saber move under `jp_alterDMG`, one entry per `jp_*Attack`
/// setting. [`Default`] is the table JA+ builds in (`JP_SaberAttackDamage`'s `defMin`,
/// `defMax` and flat values, VisionJK `w_saber.c:2302-2394`), which are also the
/// settings' defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveDamage {
    /// `jp_StabDownAttack*`: the stab down at a player lying on the ground.
    pub stab_down: DamageRange,
    /// `jp_RollStabAttack*`.
    pub roll_stab: DamageRange,
    /// `jp_StaffBackAttack*`, `jp_DualBackAttack*`.
    pub staff_back: DamageRange,
    pub dual_back: DamageRange,
    /// `jp_StaffDualNormalAttack*`: the staff's and the pair's plain attacks.
    pub staff_dual_normal: DamageRange,
    /// `jp_StaffDualSpinAttack`: their spin attacks, flat.
    pub staff_dual_spin: i32,
    /// `jp_NewSpinDfaAttack*`: Alora's spin (a quarter of it) and the strong style's
    /// butterfly.
    pub new_spin_dfa: DamageRange,
    /// `jp_StaffDualOtherAttack*`: their other specials.
    pub staff_dual_other: DamageRange,
    /// `jp_DualKataAttack` (flat), `jp_StaffKataAttack*`.
    pub dual_kata: i32,
    pub staff_kata: DamageRange,
    /// `jp_NewPullAttack*`: the pull attack's swing, and a fifth of it for its stab.
    pub new_pull: DamageRange,
    /// `jp_SingleKataAttack*`: the three single styles' katas.
    pub single_kata: DamageRange,
    /// `jp_SingleStrong*`: the strong style's plain attacks, its death from above, its
    /// back attack and its other specials (flat).
    pub strong_normal: DamageRange,
    pub strong_dfa: DamageRange,
    pub strong_back: DamageRange,
    pub strong_other: i32,
    /// `jp_SingleMedium*`: the medium style's flip attack, back attack, the rest (flat).
    pub medium_dfa: DamageRange,
    pub medium_back: DamageRange,
    pub medium_other: i32,
    /// `jp_SingleFast*`: the fast style's lunge, back attack, the rest (flat).
    pub fast_lunge: DamageRange,
    pub fast_back: DamageRange,
    pub fast_other: i32,
}

impl Default for MoveDamage {
    fn default() -> Self {
        Self {
            stab_down: range(2, 50),
            roll_stab: range(2, 12),
            staff_back: range(2, 30),
            dual_back: range(2, 25),
            staff_dual_normal: range(2, 60),
            staff_dual_spin: 10,
            new_spin_dfa: range(2, 60),
            staff_dual_other: range(2, 45),
            dual_kata: 50,
            staff_kata: range(30, 50),
            new_pull: range(2, 50),
            single_kata: range(30, 50),
            strong_normal: range(2, 110),
            strong_dfa: range(2, 180),
            strong_back: range(2, 50),
            strong_other: 100,
            medium_dfa: range(2, 70),
            medium_back: range(2, 35),
            medium_other: 60,
            fast_lunge: range(2, 30),
            fast_back: range(2, 30),
            fast_other: 35,
        }
    }
}

/// JA+'s saber damage rules as the running server mod sets them; [`Default`] is the base
/// game's, every rule off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JaPlusSaberDamage {
    /// A JA+ server: the rules every JA+ server plays — a swinger under the chat bubble
    /// cuts nobody, a blade meeting a saber may pass through it ([`Self::reduce_block`]),
    /// an idle touch hurts by [`Self::touch_damage`].
    pub enabled: bool,
    /// `jp_alterDMG` other than 0: JA+'s move table ([`Self::moves`]) in place of the
    /// SP-style damage, and the specials' extra weights.
    pub move_damage: bool,
    /// The table, `jp_*Attack` with `jp_alterDMG 1`, JA+'s built-in one otherwise.
    pub moves: MoveDamage,
    /// `jp_SaberThrowScaleDMG` with `jp_alterDMG 1`: the thrown saber's damage scaled.
    pub throw_scale: Option<f32>,
    /// `jp_alterDMG 1`: a blow at a player holding a saber is halved (70 % for a level 1
    /// hit), outside siege, unless it is unblockable or thrown.
    pub halved_on_sabers: bool,
    /// `jp_reduceSaberBlock`: the percent chance that a blade passes through the saber it
    /// meets.
    pub reduce_block: i32,
    /// `jp_allowSaberTouchDMG`: an idle blade's touch hurts (1), not only a returning one.
    pub touch_damage: bool,
}

/// What JA+'s damage reads of the swinger's torso animation: `ps.torsoAnim`, its
/// `torsoTimer`, the animation's length (`numFrames * |frameLerp|`) and what
/// `BG_SaberStartTransAnim` speeds it by.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwingAnimation {
    pub torso: u16,
    /// `ps.legsAnim`: a kick played by the legs keeps a special's blade at 2.
    pub legs: u16,
    pub timer: i32,
    pub length: i32,
    /// `ps.fd.saberAnimLevel`, `ps.weapon`, `ps.brokenLimbs` and the hilts'
    /// `animSpeedScale`, for `BG_SaberStartTransAnim`.
    pub style: i32,
    pub weapon: u8,
    pub broken_limbs: u8,
    pub hilt_scales: [f32; 2],
}

impl SwingAnimation {
    /// `BG_SaberStartTransAnim`'s factor, from 1.
    fn speed(&self) -> f32 {
        crate::pmove_anim::saber_start_trans_anim(
            1.0,
            self.torso,
            self.weapon,
            self.style,
            self.broken_limbs,
            self.hilt_scales,
        )
    }

    /// `G_GetAttackDamage` (OpenJK `w_saber.c`, VisionJK `w_saber.c:2243-2280`): `max`
    /// at `peak` of the animation (as a share of it from its start), falling off either
    /// side, never below `min`.
    ///
    /// In `float`, as the source computes it with SSE. VisionJK computes it in `long
    /// double` to match the original Linux module's x87 FPU, which truncates one point
    /// lower where the factor is exactly a value like 0.7 (110 at 0.7 is 76 there, 77
    /// here); SJK follows the SSE results (VisionJK `docs/japlus/differences.md`, "Where
    /// the original itself depends on the platform").
    pub fn attack_damage(&self, min: i32, max: i32, peak: f32) -> i32 {
        let mut length = self.length as f32;
        let speed_dif = (length - length * self.speed()) as i32;
        length += speed_dif as f32;
        let peak_point = length - length * peak;
        let mut factor = self.timer as f32 / peak_point;
        if factor > 1.0 {
            factor = 2.0 - factor;
        }
        let total = (max as f32 * factor) as i32;
        total.max(min).min(max)
    }

    /// `G_GetAnimPoint` (`w_saber.c:2398-2419`): how much of the animation is left, from 1
    /// at its start to 0 at its end, in `float` as the reference computes it.
    pub fn anim_point(&self) -> f32 {
        let mut length = self.length as f32;
        let speed_dif = (length - length * self.speed()) as i32;
        length += speed_dif as f32;
        self.timer as f32 / length
    }
}

impl JaPlusSaberDamage {
    /// `JP_AttackDamage`: the move's ramp over `range`.
    fn ramp(&self, animation: &SwingAnimation, range: DamageRange, peak: f32) -> i32 {
        animation.attack_damage(range.min, range.max, peak)
    }

    /// `JP_SaberAttackDamage` (VisionJK `w_saber.c:2302-2394`): an attacking blade's
    /// damage under [`Self::move_damage`], by the swinger's style and saber move. JA+'s
    /// own moves (`LS_JAPLUS_SPINATTACK`, `LS_NEWANIM_*`), which need its extra
    /// animations, are not played on SJK's server and have no entry.
    pub fn attack(&self, saber_move: u32, animation: &SwingAnimation) -> i32 {
        let table = &self.moves;
        let style = animation.style;
        let special = crate::saber_rules::in_special(saber_move);
        let back = back_attack(saber_move);
        let kata = crate::saber_rules::in_kata(saber_move);
        let is = |name: u16| saber_move == u32::from(name);
        if crate::saber_rules::stab_down(animation.torso) {
            return self.ramp(animation, table.stab_down, 0.5);
        }
        if is(moves::LS_ROLL_STAB) {
            return self.ramp(animation, table.roll_stab, 0.5);
        }
        if style == SS_STAFF || style == SS_DUAL {
            if back {
                let range = if style == SS_STAFF {
                    table.staff_back
                } else {
                    table.dual_back
                };
                return self.ramp(animation, range, 0.5);
            }
            if !special {
                return self.ramp(animation, table.staff_dual_normal, 0.5);
            }
            if is(moves::LS_SPINATTACK) || is(moves::LS_SPINATTACK_DUAL) {
                return table.staff_dual_spin;
            }
            if is(LS_SPINATTACK_ALORA) {
                // A quarter, truncated (`w_saber.c:2332`).
                let damage = self.ramp(animation, table.new_spin_dfa, 0.5);
                return (damage as f32 * 0.25) as i32;
            }
            if animation.kicking() && !is(moves::LS_A_BACKFLIP_ATK) {
                // "saber shouldn't do more than min dmg during kicks"
                return 2;
            }
            if !kata {
                return self.ramp(animation, table.staff_dual_other, 0.5);
            }
            if style == SS_DUAL {
                return table.dual_kata;
            }
            return self.ramp(animation, table.staff_kata, 0.5);
        }
        if is(moves::LS_PULL_ATTACK_SWING) || is(moves::LS_PULL_ATTACK_STAB) {
            let damage = self.ramp(animation, table.new_pull, 0.5);
            return if is(moves::LS_PULL_ATTACK_STAB) {
                (damage as f32 * 0.2) as i32
            } else {
                damage
            };
        }
        match style {
            SS_STRONG => {
                if !special {
                    let range = if back {
                        table.strong_back
                    } else {
                        table.strong_normal
                    };
                    return self.ramp(animation, range, 0.5);
                }
                if is(moves::LS_A_JUMP_T__B_) {
                    return self.ramp(animation, table.strong_dfa, 0.65);
                }
                if is(moves::LS_BUTTERFLY_LEFT) {
                    return self.ramp(animation, table.new_spin_dfa, 0.5);
                }
                if back {
                    return self.ramp(animation, table.strong_back, 0.5);
                }
                if !kata {
                    return table.strong_other;
                }
                self.ramp(animation, table.single_kata, 0.5)
            }
            SS_MEDIUM => {
                if special && (is(moves::LS_A_FLIP_STAB) || is(moves::LS_A_FLIP_SLASH)) {
                    return self.ramp(animation, table.medium_dfa, 0.5);
                }
                if back {
                    return self.ramp(animation, table.medium_back, 0.5);
                }
                if !kata {
                    return table.medium_other;
                }
                self.ramp(animation, table.single_kata, 0.5)
            }
            SS_FAST => {
                if special && is(moves::LS_A_LUNGE) {
                    return self.ramp(animation, table.fast_lunge, 0.3);
                }
                if back {
                    return self.ramp(animation, table.fast_back, 0.5);
                }
                if !kata {
                    return table.fast_other;
                }
                self.ramp(animation, table.single_kata, 0.5)
            }
            _ => SABER_HITDAMAGE,
        }
    }

    /// The specials' extra weights (VisionJK `w_saber.c:4380-4446`), which the base game
    /// gives only with `d_saberSPStyleDamage 0` and JA+ also under [`Self::move_damage`]:
    /// `damage` as it stands, changed for a special that is not a back attack.
    pub fn special_weight(&self, damage: i32, saber_move: u32, animation: &SwingAnimation) -> i32 {
        if !self.move_damage || back_attack(saber_move) {
            return damage;
        }
        let is = |name: u16| saber_move == u32::from(name);
        if is(moves::LS_A_JUMP_T__B_) {
            // "do extra damage for special unblockables"
            damage + 5
        } else if is(moves::LS_A_FLIP_STAB) || is(moves::LS_A_FLIP_SLASH) {
            // "don't want it doing big damage in the off points of the anim"
            if animation.anim_point() <= 0.2 {
                2
            } else {
                damage + 5
            }
        } else if is(moves::LS_A_LUNGE) {
            damage + 2
        } else if is(moves::LS_SPINATTACK) || is(moves::LS_SPINATTACK_DUAL) {
            animation.attack_damage(0, damage + 3, 0.5) + 10
        } else if animation.kicking() && !is(moves::LS_A_BACKFLIP_ATK) {
            2
        } else {
            animation.attack_damage(5, damage + 5, 0.5)
        }
    }

    /// `jp_alterDMG 1`'s cut of a blow at a player holding a saber (VisionJK
    /// `w_saber.c:4726-4745`), after the base game's 1.5 times for a client.
    pub fn versus_saber(&self, damage: i32, siege: bool, unblockable: bool, thrown: bool) -> i32 {
        if !self.halved_on_sabers || thrown || siege || damage <= 1 || unblockable {
            return damage;
        }
        if damage == SABER_HITDAMAGE {
            (f64::from(damage) * 0.7) as i32
        } else {
            (f64::from(damage) * 0.5) as i32
        }
    }
}

impl SwingAnimation {
    /// `BG_KickingAnim(legsAnim) || BG_KickingAnim(torsoAnim)`.
    fn kicking(&self) -> bool {
        crate::saber_rules::kicking(self.legs) || crate::saber_rules::kicking(self.torso)
    }
}

/// `LS_SPINATTACK_ALORA`.
const LS_SPINATTACK_ALORA: u16 = 58;

/// `G_SaberInBackAttack`: `LS_A_BACK`, `LS_A_BACK_CR`, `LS_A_BACKSTAB`.
fn back_attack(saber_move: u32) -> bool {
    [moves::LS_A_BACK, moves::LS_A_BACK_CR, moves::LS_A_BACKSTAB]
        .iter()
        .any(|back| u32::from(*back) == saber_move)
}

#[cfg(test)]
mod tests {
    //! The numbers follow VisionJK's `JP_SaberAttackDamage` and `G_GetAttackDamage`
    //! (`w_saber.c:2243-2394`) and the specials' weights (`w_saber.c:4386-4446`), worked
    //! by hand for an animation 1000 ms long.
    use super::*;

    /// `BOTH_A3_T__B_`: a strong attack that is no transition, so it plays at speed 1.
    const ATTACK: u16 = 280;

    fn swing(style: i32, timer: i32) -> SwingAnimation {
        SwingAnimation {
            torso: ATTACK,
            legs: 0,
            timer,
            length: 1000,
            style,
            weapon: 3,
            broken_limbs: 0,
            hilt_scales: [1.0; 2],
        }
    }

    fn rules() -> JaPlusSaberDamage {
        JaPlusSaberDamage {
            enabled: true,
            move_damage: true,
            ..JaPlusSaberDamage::default()
        }
    }

    #[test]
    fn the_ramp_peaks_where_it_is_asked_to() {
        let at = |timer| swing(SS_STRONG, timer).attack_damage(2, 110, 0.5);
        // Peak at 500 of the 1000 left; half the damage half-way on either side.
        assert_eq!(at(500), 110);
        assert_eq!(at(250), 55);
        assert_eq!(at(750), 55);
        // Its end: nothing, held at the lowest.
        assert_eq!(at(0), 2);
        assert_eq!(at(1000), 2);
        // A factor of exactly 0.7: 77 in float (SSE), 76 on the original's x87.
        assert_eq!(at(350), 77);
        // The lunge peaks later (0.3 of it done: 700 left).
        assert_eq!(swing(SS_FAST, 700).attack_damage(2, 30, 0.3), 30);
    }

    #[test]
    fn the_ramp_follows_the_animation_speed() {
        // `BOTH_T3_BR__R` (287), a strong transition: three quarters of the speed, so the
        // animation lasts 1250 and peaks at 625.
        let slow = SwingAnimation {
            torso: 287,
            ..swing(SS_STRONG, 625)
        };
        assert_eq!(slow.attack_damage(2, 110, 0.5), 110);
        assert_eq!(swing(SS_STRONG, 625).attack_damage(2, 110, 0.5), 82);
    }

    #[test]
    fn each_style_and_move_takes_its_entry() {
        let rules = rules();
        let peak = |style| swing(style, 500);
        let attack = |saber_move: u16, style| rules.attack(u32::from(saber_move), &peak(style));
        assert!(crate::saber_rules::in_special(u32::from(moves::LS_A_LUNGE)));
        assert!(crate::saber_rules::in_kata(u32::from(
            moves::LS_STAFF_SOULCAL
        )));
        // The single styles' plain attacks.
        assert_eq!(attack(moves::LS_A_TL2BR, SS_STRONG), 110);
        assert_eq!(attack(moves::LS_A_TL2BR, SS_MEDIUM), 60);
        assert_eq!(attack(moves::LS_A_TL2BR, SS_FAST), 35);
        // Their specials.
        // Death from above peaks at 0.65: 350 left. At 500 left, 2 - 500/350 of 180.
        assert_eq!(attack(moves::LS_A_JUMP_T__B_, SS_STRONG), 102);
        assert_eq!(attack(moves::LS_A_BACK, SS_STRONG), 50);
        assert_eq!(attack(moves::LS_A_FLIP_STAB, SS_MEDIUM), 70);
        assert_eq!(attack(moves::LS_A_BACK_CR, SS_MEDIUM), 35);
        assert_eq!(
            rules.attack(u32::from(moves::LS_A_LUNGE), &swing(SS_FAST, 700)),
            30
        );
        assert_eq!(attack(moves::LS_A_BACKSTAB, SS_FAST), 30);
        assert_eq!(attack(moves::LS_A1_SPECIAL, SS_FAST), 50);
        // Strong's butterfly takes the spin entry.
        assert_eq!(attack(moves::LS_BUTTERFLY_LEFT, SS_STRONG), 60);
        // The staff and the pair.
        assert_eq!(attack(moves::LS_A_TL2BR, SS_STAFF), 60);
        assert_eq!(attack(moves::LS_A_BACK, SS_STAFF), 30);
        assert_eq!(attack(moves::LS_A_BACK, SS_DUAL), 25);
        assert_eq!(attack(moves::LS_SPINATTACK, SS_STAFF), 10);
        assert_eq!(attack(moves::LS_SPINATTACK_DUAL, SS_DUAL), 10);
        assert_eq!(attack(LS_SPINATTACK_ALORA, SS_DUAL), 15);
        assert_eq!(attack(moves::LS_STAFF_SOULCAL, SS_STAFF), 50);
        assert_eq!(attack(moves::LS_DUAL_SPIN_PROTECT, SS_DUAL), 50);
        assert_eq!(attack(moves::LS_JUMPATTACK_STAFF_LEFT, SS_STAFF), 45);
        // Every style's.
        assert_eq!(attack(moves::LS_ROLL_STAB, SS_MEDIUM), 12);
        assert_eq!(attack(moves::LS_PULL_ATTACK_SWING, SS_MEDIUM), 50);
        assert_eq!(attack(moves::LS_PULL_ATTACK_STAB, SS_MEDIUM), 10);
        // Desann's and Tavion's styles have no entry.
        assert_eq!(attack(moves::LS_A_TL2BR, 4), SABER_HITDAMAGE);
        // The staff's kata at its end keeps its lowest, 30.
        let end = swing(SS_STAFF, 0);
        assert_eq!(rules.attack(u32::from(moves::LS_STAFF_SOULCAL), &end), 30);
    }

    #[test]
    fn a_changed_table_is_read() {
        let mut rules = rules();
        rules.moves.strong_normal = range(2, 80);
        rules.moves.fast_other = 20;
        assert_eq!(
            rules.attack(u32::from(moves::LS_A_TL2BR), &swing(SS_STRONG, 500)),
            80
        );
        assert_eq!(
            rules.attack(u32::from(moves::LS_A_TL2BR), &swing(SS_FAST, 500)),
            20
        );
    }

    #[test]
    fn the_specials_weigh_only_with_the_move_table() {
        let rules = rules();
        let at = |saber_move: u16, damage, timer| {
            rules.special_weight(damage, u32::from(saber_move), &swing(SS_STRONG, timer))
        };
        assert_eq!(at(moves::LS_A_JUMP_T__B_, 100, 500), 105);
        assert_eq!(at(moves::LS_A_LUNGE, 30, 500), 32);
        assert_eq!(at(moves::LS_A_FLIP_STAB, 70, 500), 75);
        // Early in the flip, where 0.2 or less of it is left: 2.
        assert_eq!(at(moves::LS_A_FLIP_SLASH, 70, 100), 2);
        // The spin attacks: ramped to 13 at the peak, 10 more.
        assert_eq!(at(moves::LS_SPINATTACK, 10, 500), 23);
        // Any other special: ramped from 5 to its damage and 5.
        assert_eq!(at(moves::LS_A_BACKFLIP_ATK, 20, 500), 25);
        assert_eq!(at(moves::LS_A_BACKFLIP_ATK, 20, 0), 5);
        // A back attack keeps its damage.
        assert_eq!(at(moves::LS_A_BACK, 50, 500), 50);
        // The SP style (`jp_alterDMG 0`) adds nothing.
        let sp = JaPlusSaberDamage {
            move_damage: false,
            ..rules
        };
        let animation = swing(SS_STRONG, 500);
        assert_eq!(
            sp.special_weight(100, u32::from(moves::LS_A_JUMP_T__B_), &animation),
            100
        );
    }

    #[test]
    fn a_blow_at_a_saber_is_halved_with_jp_alter_dmg_1() {
        let off = rules();
        assert_eq!(off.versus_saber(60, false, false, false), 60);
        let on = JaPlusSaberDamage {
            halved_on_sabers: true,
            ..rules()
        };
        assert_eq!(on.versus_saber(60, false, false, false), 30);
        assert_eq!(on.versus_saber(61, false, false, false), 30);
        // A level 1 hit: 70 %.
        assert_eq!(on.versus_saber(SABER_HITDAMAGE, false, false, false), 24);
        // Not in siege, not unblockable, not thrown, not a touch.
        assert_eq!(on.versus_saber(60, true, false, false), 60);
        assert_eq!(on.versus_saber(60, false, true, false), 60);
        assert_eq!(on.versus_saber(60, false, false, true), 60);
        assert_eq!(on.versus_saber(1, false, false, false), 1);
    }
}
