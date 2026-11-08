//! `jp_cinfo`: the bits with which a JA+ server tells its clients the rules they
//! must predict, and which of them SJK's server runs.
//!
//! JA+ builds the value every frame from its settings (VisionJK
//! `codemp/game/japlus/g_japlus_misc.c:824-855`, from the 2.4 B7 module); the clients
//! read it from the server info (EternalJK, the JA+ plugin, SJK's
//! `sjk_protocol::JaPlusCapabilities`). A bit SJK's server does not run yet is left out
//! whatever its setting says: a client told about a rule the server does not play would
//! predict it and be pulled back. [`CINFO_BITS`] is the table; a rule the server learns
//! is a `runs: true` there.

use sjk_protocol::JaPlusCapabilities as Caps;
use sjk_server_mod::Settings;

/// When a setting turns its bit on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum When {
    /// The setting is exactly this value (JA+ tests `== 1` for its switches).
    Is(i32),
    /// The setting is above this value (`jp_fixRoll`'s three levels).
    Above(i32),
    /// The setting is anything but 0.
    NotZero,
}

impl When {
    fn holds(self, value: i32) -> bool {
        match self {
            Self::Is(wanted) => value == wanted,
            Self::Above(floor) => value > floor,
            Self::NotZero => value != 0,
        }
    }
}

/// One bit of `jp_cinfo`.
#[derive(Clone, Copy, Debug)]
pub struct CinfoBit {
    /// The bit (`JaPlusCapabilities`).
    pub bit: u32,
    /// The setting it follows.
    pub setting: &'static str,
    pub when: When,
    /// SJK's server plays the rule (or, for a request to clients only, passes it on).
    pub runs: bool,
    /// What the bit is and why it runs or not.
    pub note: &'static str,
}

/// The bits in JA+'s order.
pub const CINFO_BITS: &[CinfoBit] = &[
    CinfoBit {
        bit: Caps::FLIP_KICK,
        setting: "jp_allowflipkick",
        when: When::Is(1),
        runs: true,
        note: "wall flips and runs off players: the shared movement's `JaPlusRules::flip_kick`",
    },
    CinfoBit {
        bit: Caps::FIX_ROLL_1,
        setting: "jp_fixRoll",
        when: When::Above(0),
        runs: true,
        note: "roll fix 1: the shared movement's `RollRules::fix_roll`, TaystJK's reading of \
               the levels, which clients already predict on JA+ servers",
    },
    CinfoBit {
        bit: Caps::FIX_ROLL_2,
        setting: "jp_fixRoll",
        when: When::Above(1),
        runs: true,
        note: "roll fix 2, as fix 1",
    },
    CinfoBit {
        bit: Caps::FIX_ROLL_3,
        setting: "jp_fixRoll",
        when: When::Above(2),
        runs: true,
        note: "roll fix 3, as fix 1",
    },
    CinfoBit {
        bit: Caps::YELLOW_DFA,
        setting: "jp_improveYellowDFA",
        when: When::Is(1),
        runs: true,
        note: "the medium style's shorter flip over: `JaPlusRules::yellow_dfa`",
    },
    CinfoBit {
        bit: Caps::HEAD_SLIDE,
        setting: "jp_slideOnPlayer",
        when: When::Is(1),
        runs: true,
        note: "standing on a player slides as in the base game: `JaPlusRules::head_slide`",
    },
    CinfoBit {
        bit: Caps::SINGLE_PLAYER_ATTACKS,
        setting: "jp_allowSPattacks",
        when: When::Is(1),
        runs: false,
        note: "single player saber attacks: not in the shared movement yet",
    },
    CinfoBit {
        bit: Caps::NEW_DFA,
        setting: "jp_allowNewDFA",
        when: When::Is(1),
        runs: false,
        note: "JA+'s new death from above: not in the shared movement yet",
    },
    CinfoBit {
        bit: Caps::MODEL_SCALE,
        setting: "jp_allowModelScale",
        when: When::Is(1),
        runs: false,
        note: "model scale from modelscale.cfg: not built",
    },
    CinfoBit {
        bit: Caps::DAMAGE_SPEED_SCALE,
        setting: "jp_allowDmgSpeedScale",
        when: When::Is(1),
        runs: false,
        note: "saber damage and speed follow the model scale: not built",
    },
    CinfoBit {
        bit: Caps::MACRO_SCAN_1,
        setting: "jp_macroScan",
        when: When::Is(1),
        runs: false,
        note: "asks the JA+ plugin to scan its binds for macros; the server does nothing with \
               what it finds",
    },
    CinfoBit {
        bit: Caps::MACRO_SCAN_2,
        setting: "jp_macroScan",
        when: When::NotZero,
        runs: false,
        note: "as the first macro bit; JA+ sets it for 2 only once every player has the plugin",
    },
    CinfoBit {
        bit: Caps::JK2_DFA,
        setting: "jp_jk2RedDFA",
        when: When::Is(1),
        runs: false,
        note: "Jedi Outcast's red death from above: not in the shared movement",
    },
    CinfoBit {
        bit: Caps::NO_KATA,
        setting: "jp_noKATA",
        when: When::Is(1),
        runs: false,
        note: "no katas: the server does not refuse them yet",
    },
    CinfoBit {
        bit: Caps::NO_AUTO_REPLIER,
        setting: "jp_noAutoReplier",
        when: When::Is(1),
        runs: true,
        note: "asks the JA+ plugin to keep its automatic replies off; nothing for the server \
               to play",
    },
    CinfoBit {
        bit: Caps::GLA_ANIMATIONS,
        setting: "jp_newGLAAnims",
        when: When::NotZero,
        runs: false,
        note: "JA+'s extra animations need its own animation files, which SJK does not ship",
    },
    CinfoBit {
        bit: Caps::LEDGE_GRAB,
        setting: "jp_ledgeGrab",
        when: When::NotZero,
        runs: false,
        note: "ledge grab: not in the shared movement yet",
    },
    CinfoBit {
        bit: Caps::ALTERNATE_DIMENSION,
        setting: "jp_altDim",
        when: When::NotZero,
        runs: false,
        note: "the alternate dimension: not built",
    },
];

/// `jp_cinfo` as SJK's server announces it: the bits whose settings are on and whose
/// rules it runs.
pub fn cinfo(settings: &dyn Settings) -> u32 {
    CINFO_BITS
        .iter()
        .filter(|entry| entry.runs && entry.when.holds(settings.integer(entry.setting)))
        .fold(0, |bits, entry| bits | entry.bit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Table(HashMap<&'static str, i32>);

    impl Settings for Table {
        fn integer(&self, name: &str) -> i32 {
            self.0.get(name).copied().unwrap_or_default()
        }
        fn value(&self, name: &str) -> f32 {
            self.integer(name) as f32
        }
    }

    /// JA+'s defaults for the bits' settings (`crate::cvars::CVARS`).
    fn defaults() -> Table {
        Table(
            CINFO_BITS
                .iter()
                .map(|entry| {
                    let cvar = crate::cvars::CVARS
                        .iter()
                        .find(|cvar| cvar.name == entry.setting)
                        .unwrap_or_else(|| panic!("{} is not a JA+ setting", entry.setting));
                    (entry.setting, cvar.default.parse().unwrap())
                })
                .collect(),
        )
    }

    #[test]
    fn every_bit_once_and_every_setting_is_jas() {
        let mut seen = 0;
        for entry in CINFO_BITS {
            assert_eq!(entry.bit.count_ones(), 1);
            assert_eq!(seen & entry.bit, 0, "{} twice", entry.setting);
            seen |= entry.bit;
        }
        // Bits 0 to 17, as JA+ 2.4 B7 has them.
        assert_eq!(seen, (1 << 18) - 1);
        let _ = defaults();
    }

    #[test]
    fn ja_plus_defaults_announce_only_what_the_server_runs() {
        let bits = cinfo(&defaults());
        // jp_allowflipkick 1, jp_fixRoll 1, jp_improveYellowDFA 1.
        assert_eq!(bits, Caps::FLIP_KICK | Caps::FIX_ROLL_1 | Caps::YELLOW_DFA);
        // JA+'s own default would also hold SP attacks, the new DFA, the GLA animations,
        // ledge grab and the alternate dimension, which are not run yet.
        assert_eq!(bits & Caps::NEW_DFA, 0);
        assert_eq!(bits & Caps::LEDGE_GRAB, 0);
    }

    #[test]
    fn roll_fix_levels_add_up() {
        let mut settings = defaults();
        settings.0.insert("jp_fixRoll", 3);
        let bits = cinfo(&settings);
        assert_eq!(
            bits & (Caps::FIX_ROLL_1 | Caps::FIX_ROLL_2 | Caps::FIX_ROLL_3),
            Caps::FIX_ROLL_1 | Caps::FIX_ROLL_2 | Caps::FIX_ROLL_3
        );
        settings.0.insert("jp_fixRoll", 0);
        assert_eq!(cinfo(&settings) & Caps::FIX_ROLL_1, 0);
    }

    #[test]
    fn a_switch_counts_only_at_one() {
        let mut settings = defaults();
        settings.0.insert("jp_allowflipkick", 2);
        assert_eq!(cinfo(&settings) & Caps::FLIP_KICK, 0);
        settings.0.insert("jp_slideOnPlayer", 1);
        settings.0.insert("jp_noAutoReplier", 1);
        let bits = cinfo(&settings);
        assert_ne!(bits & Caps::HEAD_SLIDE, 0);
        assert_ne!(bits & Caps::NO_AUTO_REPLIER, 0);
    }
}
