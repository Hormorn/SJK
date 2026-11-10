//! `cg_defaultModelRandom`: the stock model that stands in for a player whose model
//! this client lacks, after JoF EternalJK (`CG_RegisterClientModelname_RandomCheck`,
//! `codemp/cgame/cg_players.c`).
//!
//! Base JKA shows Kyle for every missing model (`CG_LoadClientInfo` registers
//! `DEFAULT_MODEL`), so a server full of custom models looks like a server of Kyles.
//! With the setting on, the stand-in is a retail model picked from the missing
//! model's name (the sum of its lower-cased bytes), so one missing model always
//! looks the same, and two different ones most likely differ. A female player
//! (`ds` `f` in their clientinfo) gets a female model. A non-Jedi stand-in keeps
//! the player's `default`/`red`/`blue` skin or takes one of those three; a Jedi
//! (`jedi_*`) stand-in gets a head, torso and legs from the parts the retail
//! model has. In team games a non-Jedi stand-in wears the team's skin, as
//! `BG_ValidateSkinForTeam` gives it.
//!
//! Off by default: Kyle stays the stock stand-in. A missing skin is not a missing
//! model: the model's default skin draws, as in JKA, and this does not apply.

use sjk_protocol::GameState;
use sjk_runtime::Appearance;

/// The console variable that turns random stand-ins on.
pub const CVAR: &str = "cg_defaultModelRandom";

/// Male stand-ins (EternalJK's `defaultModelTable`).
const MALE_MODELS: [&str; 10] = [
    "luke",
    "jedi_rm",
    "jedi_kdm",
    "jedi_hm",
    "rodian",
    "jedi",
    "prisoner",
    "imperial",
    "bespin_cop",
    "rebel",
];

/// Female stand-ins (EternalJK's `defaultFemaleModelTable`; it leaves `monmothma`
/// out, as viewers found her confusing to fight).
const FEMALE_MODELS: [&str; 7] = [
    "alora",
    "alora2",
    "jedi_hf",
    "jedi_tf",
    "jedi_zf",
    "tavion_new",
    "tavion",
];

/// The skins every non-Jedi stand-in has.
const SKINS: [&str; 3] = ["default", "red", "blue"];

/// Twi'lek heads (`jedi_tf`); `head_a3` is left out, as in EternalJK.
const TWILEK_FEMALE_HEADS: [&str; 7] = [
    "head_a1", "head_a2", "head_a4", "head_b1", "head_b2", "head_b3", "head_b4",
];
const FEMALE_HEADS: [&str; 3] = ["head_a1", "head_b1", "head_c1"];
/// `torso_g1` and `lower_e1` are left out, as in EternalJK.
const FEMALE_TORSOS: [&str; 6] = [
    "torso_a1", "torso_b1", "torso_c1", "torso_d1", "torso_e1", "torso_f1",
];
const FEMALE_LOWERS: [&str; 4] = ["lower_a1", "lower_b1", "lower_c1", "lower_d1"];
/// The human male (`jedi_hm`) has two more heads and a second `torso_d`.
const HUMAN_MALE_HEADS: [&str; 5] = ["head_a1", "head_a2", "head_b1", "head_b2", "head_c1"];
const HUMAN_MALE_TORSOS: [&str; 7] = [
    "torso_a1", "torso_b1", "torso_c1", "torso_d1", "torso_d2", "torso_e1", "torso_f1",
];
/// The other male species (`jedi_rm`, `jedi_kdm`).
const MALE_HEADS: [&str; 3] = ["head_a1", "head_b1", "head_c1"];
const MALE_TORSOS: [&str; 6] = [
    "torso_a1", "torso_b1", "torso_c1", "torso_d1", "torso_e1", "torso_f1",
];
const MALE_LOWERS: [&str; 4] = ["lower_a1", "lower_b1", "lower_c1", "lower_d1"];

/// The Kyle stand-in of base JKA.
pub fn kyle() -> Appearance {
    Appearance {
        model: "models/players/kyle".to_owned(),
        variant: "default".to_owned(),
    }
}

/// The stand-in for `client_num`'s `requested` appearance when it cannot be loaded:
/// Kyle when `random` is off, else [`random_stand_in`] with the player's gender and,
/// in team games, team.
pub fn legacy_client_stand_in(
    game_state: &GameState,
    client_num: u16,
    requested: &Appearance,
    random: bool,
) -> Appearance {
    if !random {
        return kyle();
    }
    let config = game_state
        .config_string(crate::player_identity::CS_PLAYERS + usize::from(client_num))
        .unwrap_or(&[]);
    let info = crate::LegacyClientInfo::new(config);
    let female = info.bytes("ds").is_some_and(|ds| ds.first() == Some(&b'f'));
    let team = crate::TeamColorPolicy::from_game_state(game_state)
        .team_override_active
        .then(|| info.integer("t"))
        .flatten();
    random_stand_in(requested, female, team)
}

/// The retail model standing in for `requested` (EternalJK's random default model):
/// picked by the sum of the model name's lower-cased bytes, female for `female`, in
/// team `team`'s skin (1 red, 2 blue) when it is not a Jedi.
pub fn random_stand_in(requested: &Appearance, female: bool, team: Option<i32>) -> Appearance {
    let name = requested
        .model
        .strip_prefix("models/players/")
        .unwrap_or(&requested.model);
    let sum = name_sum(name);
    let pick = |table: &[&'static str]| table[sum % table.len()];
    let model = if female {
        pick(&FEMALE_MODELS)
    } else {
        pick(&MALE_MODELS)
    };
    let variant = if model.starts_with("jedi_") {
        let (heads, torsos, lowers): (&[&str], &[&str], &[&str]) = if female {
            let heads: &[&str] = if model == "jedi_tf" {
                &TWILEK_FEMALE_HEADS
            } else {
                &FEMALE_HEADS
            };
            (heads, &FEMALE_TORSOS, &FEMALE_LOWERS)
        } else if model == "jedi_hm" {
            (&HUMAN_MALE_HEADS, &HUMAN_MALE_TORSOS, &MALE_LOWERS)
        } else {
            (&MALE_HEADS, &MALE_TORSOS, &MALE_LOWERS)
        };
        format!("{}|{}|{}", pick(heads), pick(torsos), pick(lowers))
    } else {
        match team {
            Some(1) => "red".to_owned(),
            Some(2) => "blue".to_owned(),
            _ => SKINS
                .iter()
                .find(|skin| requested.variant.eq_ignore_ascii_case(skin))
                .copied()
                .unwrap_or_else(|| pick(&SKINS))
                .to_owned(),
        }
    };
    Appearance {
        model: format!("models/players/{model}"),
        variant,
    }
}

/// EternalJK's hash: the sum of the name's bytes, lower-cased, read as C's signed
/// `char` (a byte past 127 counts negative), and 0 if the sum is negative.
fn name_sum(name: &str) -> usize {
    let sum: i64 = name
        .bytes()
        .map(|byte| i64::from(byte.to_ascii_lowercase() as i8))
        .sum();
    usize::try_from(sum).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wanted(model: &str, variant: &str) -> Appearance {
        Appearance {
            model: format!("models/players/{model}"),
            variant: variant.to_owned(),
        }
    }

    fn got(appearance: &Appearance) -> (&str, &str) {
        (
            appearance.model.strip_prefix("models/players/").unwrap(),
            appearance.variant.as_str(),
        )
    }

    #[test]
    fn the_name_sum_picks_the_model_and_keeps_a_stock_skin() {
        // "a" = 97: 97 % 10 = 7 (imperial), 97 % 3 = 1.
        let male = random_stand_in(&wanted("a", "blue"), false, None);
        assert_eq!(got(&male), ("imperial", "blue"));
        let other_skin = random_stand_in(&wanted("A", "fancy"), false, None);
        assert_eq!(got(&other_skin), ("imperial", "red"));
        // 97 % 7 = 6 (tavion).
        let female = random_stand_in(&wanted("a", "default"), true, None);
        assert_eq!(got(&female), ("tavion", "default"));
    }

    #[test]
    fn the_same_missing_model_always_gets_the_same_stand_in() {
        let first = random_stand_in(&wanted("zzzpiza", "original"), false, None);
        let again = random_stand_in(&wanted("ZZZPIZA", "other"), false, None);
        assert_eq!(first.model, again.model);
    }

    #[test]
    fn a_jedi_stand_in_wears_parts_its_model_has() {
        // A name landing on each model in turn.
        for (index, model) in MALE_MODELS.iter().enumerate() {
            let name = (b'a'..=b'z')
                .map(|byte| (byte as char).to_string().repeat(3))
                .find(|name| name_sum(name) % MALE_MODELS.len() == index)
                .unwrap();
            let stand_in = random_stand_in(&wanted(&name, "default"), false, Some(1));
            assert_eq!(got(&stand_in).0, *model);
            if model.starts_with("jedi_") {
                let parts: Vec<_> = stand_in.variant.split('|').collect();
                assert_eq!(parts.len(), 3);
                let heads: &[&str] = if *model == "jedi_hm" {
                    &HUMAN_MALE_HEADS
                } else {
                    &MALE_HEADS
                };
                assert!(heads.contains(&parts[0]), "{model}: {}", parts[0]);
            } else {
                // A team game gives a non-Jedi the team's skin.
                assert_eq!(stand_in.variant, "red");
            }
        }
    }

    #[test]
    fn a_twilek_stand_in_takes_a_twilek_head() {
        let name = (b'a'..=b'z')
            .map(|byte| (byte as char).to_string().repeat(2))
            .find(|name| FEMALE_MODELS[name_sum(name) % FEMALE_MODELS.len()] == "jedi_tf")
            .unwrap();
        let stand_in = random_stand_in(&wanted(&name, "default"), true, None);
        let head = stand_in.variant.split('|').next().unwrap();
        assert!(TWILEK_FEMALE_HEADS.contains(&head));
    }

    #[test]
    fn bytes_past_127_count_negative_as_in_c() {
        assert_eq!(name_sum("\u{e9}"), 0);
        assert_eq!(name_sum("a"), 97);
    }
}
