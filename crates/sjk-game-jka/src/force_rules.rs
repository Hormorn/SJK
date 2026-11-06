//! The Force rules a server mod changes, read by the Force code the server runs
//! ([`crate::force_powers`], [`crate::force_dark`], [`crate::force_throw`],
//! [`crate::holdables::batteries`]). [`ForceRules::default`] is the base game's; a mod's
//! settings select others (JA+'s `jp_pushPullKnockDown`, `jp_fixAbsorb`...), which the
//! server passes into every Force update and throw, its NPCs' too, as `w_force.c` treats
//! every client alike.
//!
//! These are the server's rules: what they change reaches the clients through the player
//! states and events the server sends, so no client predicts them (the movement's own
//! JA+ rules are [`crate::pmove_japlus::JaPlusRules`]).

/// What `jp_gripOpenSaber` does to a gripped player's lit saber (VisionJK
/// `codemp/game/w_force.c:6029-6051`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GripOpenSaber {
    /// The base game: it goes off (`Cmd_ToggleSaber_f`).
    #[default]
    Off,
    /// Only a thrown saber goes off; one in the hand stays lit (JA+ 1).
    ThrownOnly,
    /// A thrown saber, or that of a player with more than 35 % of its Force, goes off; a
    /// player with less loses it (`saberKnockOutOfHand`) (JA+ 2).
    LowForceDrops,
}

/// The Force rules in force on a server.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceRules {
    /// Push and pull reach a player lying knocked down (`jp_pushPullKnockDown` 1, VisionJK
    /// `w_force.c:628-634`); the base game passes over one.
    pub throw_knocked_down: bool,
    /// Push and pull reach teammates (JA+ with `g_friendlyFire`, VisionJK
    /// `w_force.c:3729-3732`); the base game never throws a teammate.
    pub throw_teammates: bool,
    /// `jp_fixAbsorb`: above 0, absorbing a grip, lightning or drain gains
    /// `spent * (absorb level / 3) * this` Force (VisionJK `w_force.c:870-875`) in place
    /// of the base game's `(spent / 3) * absorb level`, which loses everything under three
    /// points a shot. 0 is the base game's.
    pub absorb_gain_scale: f32,
    /// `jp_pushAll`: 1 push and pull fling the items they reach, 2 dropped flags too
    /// ([`crate::item_push`]); 0, the base game, leaves them alone.
    pub push_items: u8,
    /// A gripped player's lit saber ([`GripOpenSaber`]).
    pub grip_open_saber: GripOpenSaber,
    /// A grip may start while the gripper rolls or back flips, though its weapon is busy
    /// (`jp_fixRoll` above 0, VisionJK `w_force.c:1474-1478`).
    pub grip_while_rolling: bool,
    /// The cloak drains its battery while on (`jp_cloakFuel` 1, VisionJK
    /// `codemp/game/g_main.c:3348-3360`); off, it lasts for ever. The base game drains it.
    pub cloak_drains: bool,
}

impl Default for ForceRules {
    fn default() -> Self {
        Self {
            throw_knocked_down: false,
            throw_teammates: false,
            absorb_gain_scale: 0.0,
            push_items: 0,
            grip_open_saber: GripOpenSaber::Off,
            grip_while_rolling: false,
            cloak_drains: true,
        }
    }
}

impl ForceRules {
    /// `WP_AbsorbConversion`'s gain for a power that cost `spent`, absorbed at `absorb`
    /// (1 to 3): the base game's for push and pull (`continuous` false) and everywhere
    /// without [`Self::absorb_gain_scale`]. At least one point once anything was spent.
    pub fn absorb_gain(self, spent: i32, absorb: i32, continuous: bool) -> i32 {
        let gain = if continuous && self.absorb_gain_scale > 0.0 {
            // `(int)((long double)spent * (level / 3.0f) * jp_fixAbsorb.value)`: the
            // level's third a float, the product wider, cut off.
            (f64::from(spent) * f64::from(absorb as f32 / 3.0) * f64::from(self.absorb_gain_scale))
                as i32
        } else {
            (spent / 3) * absorb
        };
        if gain < 1 && spent >= 1 { 1 } else { gain }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_base_game() {
        let base = ForceRules::default();
        assert!(!base.throw_knocked_down && !base.throw_teammates);
        assert!(!base.grip_while_rolling && base.cloak_drains);
        assert_eq!(base.push_items, 0);
        assert_eq!(base.grip_open_saber, GripOpenSaber::Off);
        // `(spent / 3) * level`, one at least.
        for continuous in [false, true] {
            assert_eq!(base.absorb_gain(1, 3, continuous), 1);
            assert_eq!(base.absorb_gain(2, 1, continuous), 1);
            assert_eq!(base.absorb_gain(10, 2, continuous), 6);
            assert_eq!(base.absorb_gain(0, 3, continuous), 0);
        }
    }

    #[test]
    fn fix_absorb_scales_the_continuous_powers_only() {
        let rules = ForceRules {
            absorb_gain_scale: 2.0,
            ..ForceRules::default()
        };
        // Lightning's single point at absorb 3: 1 * 1.0 * 2 = 2 (the base game gives 1).
        assert_eq!(rules.absorb_gain(1, 3, true), 2);
        // 10 at absorb 1: 10 * 0.333.. * 2 = 6.66.., cut to 6.
        assert_eq!(rules.absorb_gain(10, 1, true), 6);
        // 3 at absorb 2: the float 2/3 is a little above, so 3 * 0.6666667 * 2 is 4.
        assert_eq!(rules.absorb_gain(3, 2, true), 4);
        // A push keeps the base game's.
        assert_eq!(rules.absorb_gain(10, 2, false), 6);
    }
}
