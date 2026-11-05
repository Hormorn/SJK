//! Finishers (`docs/unlockables.md`, "Finishers"; Sol, 04/11/2026): what a player's saber
//! kill does to the body, a category of its own beside the blade skins. A player wears one
//! finisher at a time with any blade (`cg_saberFinisher`, shown on Collection > Shaders);
//! it goes to the SJK hub with the look (`finisher`), so every SJK client in the match
//! plays it on the victims of that player's saber kills (`GpuState::observe_saber_kills`).
//!
//! The first is Disintegrate: the body burns away, as a disruptor's kill does, for every SJK
//! client (`crate::disintegration::Forced`). Milamber made it in the Saber Lab, where it was
//! a blade skin's `disintegrate` section; it became the first finisher so any blade can wear
//! it. Finishers are not droppable yet: the hub grants them by hand.

use crate::unlockables::Rarity;

/// Archived: the finisher the local player wears, an id of [`CATALOGUE`]; empty for none.
/// It is worn (and sent to the hub) only while the own hub profile lists it.
pub(crate) const CVAR: &str = "cg_saberFinisher";

/// One finisher of the catalogue.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Finisher {
    /// Its unlock id at the hub (`finisher_disintegrate`).
    pub(crate) id: &'static str,
    /// Its name as players read it.
    pub(crate) name: &'static str,
    pub(crate) tier: Rarity,
    /// What it does, for its tile's hover card.
    pub(crate) description: &'static str,
    /// How a player gets it.
    pub(crate) how_to_get: &'static str,
    /// Who made it.
    pub(crate) smith: &'static str,
    /// What it does to the body.
    pub(crate) effect: Effect,
}

/// What a finisher does to the body of a saber kill's victim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Effect {
    /// The body burns away as a disruptor's kill does, starting `delay_ms` after the blow.
    Disintegrate { delay_ms: i64 },
}

/// Every finisher this client knows, in the order Collection > Shaders lists them. The hub
/// keeps the same ids (`crate::unlocks::CATALOGUE` there, kind `finisher`).
pub(crate) const CATALOGUE: &[Finisher] = &[Finisher {
    id: "finisher_disintegrate",
    name: "Disintegrate",
    tier: Rarity::Mythical,
    description: "Your saber kills burn away: the body glows, chars and crumbles into embers, as a disruptor's kill does, for every SJK player in the match. Worn with any blade.",
    how_to_get: "Not found in holocrons yet: given by the SJK team.",
    smith: "Milamber",
    // Milamber's tuned blade burnt its kills at once (`"delay": 0.0`).
    effect: Effect::Disintegrate { delay_ms: 0 },
}];

/// The finisher `id` names (spaces round it and case ignored), if this client knows it.
pub(crate) fn get(id: &str) -> Option<&'static Finisher> {
    let id = id.trim();
    CATALOGUE
        .iter()
        .find(|finisher| finisher.id.eq_ignore_ascii_case(id))
}

/// The finisher `setting` ([`CVAR`]) chooses, when the own profile lists it (`owned`).
pub(crate) fn own(setting: &str, owned: impl Fn(&str) -> bool) -> Option<&'static str> {
    get(setting)
        .map(|finisher| finisher.id)
        .filter(|id| owned(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finishers_are_found_by_id_and_worn_only_when_held() {
        assert_eq!(
            get(" FINISHER_disintegrate ").map(|f| f.name),
            Some("Disintegrate")
        );
        assert_eq!(get("saber_sun"), None);
        assert_eq!(get(""), None);
        assert_eq!(
            own("finisher_disintegrate", |_| true),
            Some("finisher_disintegrate")
        );
        assert_eq!(own("finisher_disintegrate", |_| false), None);
        assert_eq!(own("", |_| true), None);
    }

    /// The hub's catalogue lists the same finishers, after its tags; none is a blade
    /// skin of this client.
    #[test]
    fn finishers_are_not_blade_skins() {
        for finisher in CATALOGUE {
            assert!(finisher.id.starts_with("finisher_"), "{}", finisher.id);
            assert!(
                crate::unlockables::find(finisher.id).is_none(),
                "{}",
                finisher.id
            );
        }
    }
}
