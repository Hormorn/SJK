//! Kyber crystals (`docs/holocrons.md`, "Kyber crystals", 21/10/2026, Sol's decisions): a
//! loot of their own beside the holocrons. A person earns one per achievement, once
//! ([`REWARDS`]); opening one always gives a saber shader, its tier rolled by the crystal's
//! odds ([`Odds`]), with a pity counter that makes the tenth crystal in a row without a
//! Legendary or Mythical shader one of them ([`Pity`]).
//!
//! At the hub a crystal is a holocron row whose tier is one of [`TIERS`]' ids
//! (`kyber_uncommon` to `kyber_mythical`, source `achievement`); it is listed, opened
//! (`POST /v1/holocrons/open`) and given by staff as holocrons are. It is never dropped by
//! play, never sold and never counted in the holocrons' caps or drop odds, so everything
//! about holocrons found by play reads [`crate::holocrons::TIERS`] only and leaves crystals
//! out. The tiers reuse [`crate::holocrons::Tier`] (same names, colours and lights as the
//! holocron tier of the same rank) with a slot after the holocrons'
//! ([`crate::holocrons::slot`]), which the Holocrons tab, its stage and its opening use to
//! tell them apart.
//!
//! [`BUILT_IN`] and [`REWARDS`] are this client's copies of the hub's, shown until the hub
//! sends its own (`kyber` in `GET /v1/holocrons`, `reward` on each achievement) and for
//! hubs older than crystals.

use crate::holocrons::{self, Tier};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

/// How many crystal tiers there are.
pub(crate) const COUNT: usize = 4;
/// The saber shader tiers a crystal rolls among, common to mythical.
pub(crate) const SHADER_TIERS: usize = crate::loot_odds::SHADER_TIERS;

/// A crystal tier of the holocron tier at `index`: its colour, label and light.
const fn tier(index: usize, id: &'static str, name: &'static str) -> Tier {
    let of = &holocrons::TIERS[index];
    Tier {
        id,
        name,
        label: of.label,
        colour: of.colour,
        per_mille: 0,
        index,
        slot: holocrons::COUNT + index,
        light: of.light,
    }
}

/// Every crystal tier, the commonest first, in the hub's order.
pub(crate) const TIERS: [Tier; COUNT] = [
    tier(0, "kyber_uncommon", "Uncommon Kyber Crystal"),
    tier(1, "kyber_rare", "Rare Kyber Crystal"),
    tier(2, "kyber_legendary", "Legendary Kyber Crystal"),
    tier(3, "kyber_mythical", "Mythical Kyber Crystal"),
];

/// The crystal tier the hub's `id` names, if the client knows it.
pub(crate) fn from_id(id: &str) -> Option<&'static Tier> {
    TIERS.iter().find(|tier| tier.id == id)
}

/// What the Holocrons tab and hover cards say of every crystal.
pub(crate) const ABOUT: &str = "Earned from achievements. Always a saber shader.";

/// The odds a crystal rolls its shader's tier with, and its pity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Odds {
    /// Per crystal tier, the shader's tier per mille (common to mythical; each sums to 1000).
    pub(crate) shader_tier: [[u32; SHADER_TIERS]; COUNT],
    /// The `every`-th crystal in a row without a Legendary or Mythical shader gives one.
    pub(crate) pity_every: u32,
}

/// This client's copy of the hub's odds (provisional, Sol's to tune).
pub(crate) const BUILT_IN: Odds = Odds {
    shader_tier: [
        [450, 400, 120, 25, 5],
        [200, 400, 300, 85, 15],
        [0, 250, 450, 250, 50],
        [0, 0, 300, 500, 200],
    ],
    pity_every: 10,
};

static HUB: Mutex<Option<Odds>> = Mutex::new(None);

/// The odds to show: the hub's last that fit, else [`BUILT_IN`].
pub(crate) fn current() -> Odds {
    HUB.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .unwrap_or(BUILT_IN)
}

/// Take the odds the hub sent last (`Snapshot::kyber`); nothing changes when it sent none
/// or odds this client cannot show.
pub(crate) fn follow(kyber: Option<&sjk_identity::KyberTables>) {
    let Some(odds) = kyber.and_then(of_hub) else {
        return;
    };
    *HUB.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(odds);
}

/// The hub's odds as this client shows them, if they fit: the four tiers in order, five
/// shares each summing to a thousand. A hub without pity keeps the built-in pace.
fn of_hub(kyber: &sjk_identity::KyberTables) -> Option<Odds> {
    if kyber.tiers.len() != COUNT {
        return None;
    }
    let mut odds = BUILT_IN;
    for (index, tier) in kyber.tiers.iter().enumerate() {
        if tier.tier != TIERS[index].id
            || tier.shader_tier.len() != SHADER_TIERS
            || tier.shader_tier.iter().sum::<u32>() != 1_000
        {
            return None;
        }
        odds.shader_tier[index].copy_from_slice(&tier.shader_tier);
    }
    if let Some(pity) = &kyber.pity {
        odds.pity_every = pity.every;
    }
    Some(odds)
}

/// The saber shader tiers a crystal rolls among, in [`Odds::shader_tier`]'s order.
pub(crate) const SHADER_RARITIES: [crate::unlockables::Rarity; SHADER_TIERS] = [
    crate::unlockables::Rarity::Common,
    crate::unlockables::Rarity::Uncommon,
    crate::unlockables::Rarity::Rare,
    crate::unlockables::Rarity::Legendary,
    crate::unlockables::Rarity::Mythical,
];

/// What the Holocrons tab says opening a crystal of tier `index` gives: "Opening it gives
/// a saber shader: Common 45%, Uncommon 40%, ..." (the tiers it cannot give left out).
pub(crate) fn inside(index: usize) -> String {
    let shares = current().shader_tier[index.min(COUNT - 1)];
    let odds: Vec<String> = SHADER_RARITIES
        .iter()
        .zip(shares)
        .filter(|(_, share)| *share > 0)
        .map(|(rarity, share)| {
            format!(
                "{} {}",
                rarity.label(),
                crate::cards::loot::percent(f64::from(share) / 1_000.0)
            )
        })
        .collect();
    format!("Opening it gives a saber shader: {}.", odds.join(", "))
}

/// The person's pity counter as the hub last said: crystals opened since the last one
/// that gave Legendary or better, and how many in a row make one sure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pity {
    pub(crate) every: u32,
    pub(crate) since: u32,
}

impl Pity {
    /// The hub's, when it said.
    pub(crate) fn of(kyber: Option<&sjk_identity::KyberTables>) -> Option<Self> {
        let pity = kyber?.pity.as_ref()?;
        let last = pity.every.checked_sub(1)?;
        Some(Self {
            every: pity.every,
            since: pity.since.min(last),
        })
    }

    /// How many crystals at most until one is Legendary or better: 1 when the next is.
    pub(crate) fn within(self) -> u32 {
        self.every.saturating_sub(self.since).max(1)
    }
}

/// The pity line the Holocrons tab shows: how soon a Legendary or better is sure, or how
/// often when the hub has not said where the person stands.
pub(crate) fn pity_line(pity: Option<Pity>) -> String {
    match pity {
        Some(pity) if pity.within() == 1 => {
            "Your next crystal is Legendary or better, guaranteed".to_owned()
        }
        Some(pity) => format!(
            "Legendary or better guaranteed within {} crystals",
            pity.within()
        ),
        None => format!(
            "Legendary or better guaranteed at least every {} crystals",
            current().pity_every
        ),
    }
}

/// The crystal each achievement brings, by its id: this client's copy of the hub's
/// catalogue (`reward` there), a position in [`TIERS`].
pub(crate) const REWARDS: [(&str, usize); 78] = [
    ("first_blood", 0),
    ("kills_100", 1),
    ("kills_1000", 3),
    ("saber_kills_100", 1),
    ("streak_5", 0),
    ("streak_10", 1),
    ("dark_side_25", 1),
    ("ledge_10", 0),
    ("arsenal", 1),
    ("duel_wins_10", 0),
    ("duel_wins_100", 2),
    ("captures_10", 0),
    ("maps_10", 0),
    ("maps_25", 1),
    ("servers_5", 0),
    ("hours_10", 0),
    ("hours_100", 2),
    ("storyteller", 0),
    ("bug_reporter", 0),
    ("surveyor", 1),
    ("decorated", 1),
    ("holocrons_10", 0),
    ("holocrons_50", 1),
    ("holocrons_200", 2),
    ("mythical_find", 1),
    ("craft_blade", 0),
    ("craft_finishes", 1),
    ("full_set", 2),
    ("order66_kill", 0),
    ("box_minutes_10", 0),
    ("duel_flawless", 1),
    ("duel_close_call", 0),
    ("duel_partners_25", 1),
    ("saber_throw_far", 0),
    ("thermal_return", 1),
    ("style_blue_50", 1),
    ("style_yellow_50", 1),
    ("style_red_50", 1),
    ("style_dual_50", 1),
    ("style_staff_50", 1),
    ("monitor_payback", 0),
    ("monitor_surge", 1),
    ("box_minutes_30", 0),
    ("box_minutes_60", 1),
    ("box_ambush", 0),
    ("box_spotted_10", 0),
    ("order66_purge", 1),
    ("order66_victims_25", 1),
    ("hax_puppet", 0),
    ("hax_double", 1),
    ("hax_triple", 2),
    ("hax_fall", 0),
    ("hax_minutes_5", 0),
    ("hax_minutes_15", 1),
    ("hax_traps_5", 0),
    ("drone_strike", 0),
    ("drone_monitor", 0),
    ("boombox_minutes_30", 0),
    ("boombox_crowd", 1),
    ("boombox_tough_crowd", 1),
    ("monitor_victims_25", 1),
    ("monitor_double", 0),
    ("monitor_triple", 2),
    ("monitor_far", 1),
    ("box_hunt_mark", 0),
    ("box_hunt_marks_10", 1),
    ("box_hunt_survive", 0),
    ("box_hunt_survive_5", 1),
    ("fish_first", 0),
    ("fish_50", 1),
    ("fish_kinds", 1),
    ("echo_angler", 1),
    ("old_whiskers", 1),
    ("mine_first", 0),
    ("mine_500", 1),
    ("mine_clean_250", 1),
    ("mine_kinds", 1),
    ("heart_of_the_mountain", 1),
];

/// What the hub said each achievement brings, in [`crate::achievements::ALL`]'s order: 0
/// while it has not said, else the tier's position plus one. Atomics, so the boards read it
/// every frame without a lock.
static HUB_REWARDS: [AtomicU8; crate::achievements::ALL.len()] =
    [const { AtomicU8::new(0) }; crate::achievements::ALL.len()];

/// Take the rewards a hub's achievement list carries (absent from hubs older than
/// crystals, which keep the built-in ones).
pub(crate) fn follow_rewards(held: &[sjk_identity::Achievement]) {
    for (at, said) in held.iter().filter_map(said_of) {
        HUB_REWARDS[at].store(said, Ordering::Relaxed);
    }
}

/// What a hub's achievement says it brings, as [`HUB_REWARDS`] keeps it: the position in
/// [`crate::achievements::ALL`] and the tier's position plus one; `None` when it says
/// nothing this client knows.
fn said_of(achievement: &sjk_identity::Achievement) -> Option<(usize, u8)> {
    let at = crate::achievements::ALL
        .iter()
        .position(|kind| kind.id == achievement.id)?;
    let tier = from_id(achievement.reward.as_deref()?)?;
    Some((at, tier.index as u8 + 1))
}

/// The crystal achievement `id` brings: the hub's word for it, else this client's copy.
pub(crate) fn reward(id: &str) -> Option<&'static Tier> {
    reward_with(id, |at| HUB_REWARDS[at].load(Ordering::Relaxed))
}

/// [`reward`], with what the hub said of the achievement at each position (0: nothing).
fn reward_with(id: &str, said: impl Fn(usize) -> u8) -> Option<&'static Tier> {
    let hub = crate::achievements::ALL
        .iter()
        .position(|kind| kind.id == id)
        .map(said)
        .filter(|&said| said > 0);
    match hub {
        Some(said) => TIERS.get(usize::from(said) - 1),
        None => REWARDS
            .iter()
            .find(|(achievement, _)| *achievement == id)
            .map(|(_, index)| &TIERS[*index]),
    }
}

/// "Reward: Rare Kyber Crystal", for a board, a hover card and the unlock toast.
pub(crate) const fn reward_line(tier: &Tier) -> &'static str {
    match tier.index {
        0 => "Reward: Uncommon Kyber Crystal",
        1 => "Reward: Rare Kyber Crystal",
        2 => "Reward: Legendary Kyber Crystal",
        _ => "Reward: Mythical Kyber Crystal",
    }
}

/// Where a crystal came from, for its arrival: the achievement that earned it when the
/// hub said (its id, or its name, in the row's note), else just an achievement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Earned(pub(crate) Option<&'static crate::achievements::Kind>);

impl Earned {
    /// From a crystal's note at the hub.
    pub(crate) fn of(note: &str) -> Self {
        let note = note.trim();
        Self(
            crate::achievements::ALL
                .iter()
                .find(|kind| kind.id == note || kind.name.eq_ignore_ascii_case(note)),
        )
    }
}

impl std::fmt::Display for Earned {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(kind) => write!(formatter, "Earned: {}", crate::terms::say(kind.name)),
            None => formatter.write_str("Earned from an achievement"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tiers are the holocrons' names and colours with "Kyber Crystal", their slots
    /// after the holocrons'; the hub's `src/kyber.rs` carries the same ids.
    #[test]
    fn the_crystal_tiers_are_the_holocron_tiers_as_crystals() {
        let ids: Vec<&str> = TIERS.iter().map(|tier| tier.id).collect();
        assert_eq!(
            ids,
            [
                "kyber_uncommon",
                "kyber_rare",
                "kyber_legendary",
                "kyber_mythical"
            ]
        );
        for (index, tier) in TIERS.iter().enumerate() {
            let holocron = &holocrons::TIERS[index];
            assert_eq!(tier.name, format!("{} Kyber Crystal", holocron.label));
            assert_eq!((tier.colour, tier.light), (holocron.colour, holocron.light));
            assert_eq!((tier.index, tier.slot), (index, holocrons::COUNT + index));
            assert!(tier.is_kyber() && !holocron.is_kyber());
            assert_eq!(from_id(tier.id), Some(tier));
            assert_eq!(holocrons::Tier::from_id(tier.id), None, "not a holocron");
            assert_eq!(holocrons::any(tier.id), Some(tier));
            assert_eq!(holocrons::slot(tier.slot), tier);
            assert!(!tier.is_announced(), "grants are not announced");
            assert_eq!(reward_line(tier), format!("Reward: {}", tier.name));
        }
        assert_eq!(from_id("rare"), None);
        assert_eq!(TIERS[0].article(), "an");
    }

    /// The built-in odds are the spec's, every tier keeps a small Mythical chance, and the
    /// hub's replace them only when they fit.
    #[test]
    fn the_odds_are_the_hubs_and_fit() {
        assert_eq!(BUILT_IN.pity_every, 10);
        for shares in BUILT_IN.shader_tier {
            assert_eq!(shares.iter().sum::<u32>(), 1_000);
            assert!(shares[4] > 0, "a small chance of a Mythical shader");
        }
        assert_eq!(BUILT_IN.shader_tier[0], [450, 400, 120, 25, 5]);
        assert_eq!(BUILT_IN.shader_tier[3], [0, 0, 300, 500, 200]);
        let wire = |odds: &Odds, pity: Option<(u32, u32)>| sjk_identity::KyberTables {
            tiers: TIERS
                .iter()
                .zip(odds.shader_tier)
                .map(|(tier, shares)| sjk_identity::KyberTier {
                    tier: tier.id.to_owned(),
                    shader_tier: shares.to_vec(),
                })
                .collect(),
            pity: pity.map(|(every, since)| sjk_identity::KyberPity { every, since }),
        };
        assert_eq!(of_hub(&wire(&BUILT_IN, Some((10, 3)))), Some(BUILT_IN));
        let mut changed = BUILT_IN;
        changed.shader_tier[1] = [100, 400, 400, 85, 15];
        changed.pity_every = 8;
        assert_eq!(of_hub(&wire(&changed, Some((8, 0)))), Some(changed));
        assert_eq!(
            of_hub(&wire(&changed, None)).map(|odds| odds.pity_every),
            Some(10),
            "a hub without pity keeps the built-in pace"
        );
        let mut uneven = wire(&BUILT_IN, None);
        uneven.tiers[2].shader_tier[0] += 1;
        assert_eq!(of_hub(&uneven), None);
        let mut swapped = wire(&BUILT_IN, None);
        swapped.tiers.swap(0, 1);
        assert_eq!(of_hub(&swapped), None);
        let mut short = wire(&BUILT_IN, None);
        short.tiers.pop();
        assert_eq!(of_hub(&short), None);
    }

    #[test]
    fn what_a_crystal_gives_is_said_from_its_odds() {
        assert_eq!(
            inside(0),
            "Opening it gives a saber shader: Common 45%, Uncommon 40%, Rare 12%, Legendary 2.5%, Mythical 0.5%."
        );
        assert_eq!(
            inside(3),
            "Opening it gives a saber shader: Rare 30%, Legendary 50%, Mythical 20%."
        );
        for index in 0..COUNT {
            assert!(inside(index).len() <= 100, "one line of the tab");
        }
    }

    #[test]
    fn the_pity_line_says_how_soon_a_legendary_is_sure() {
        let pity = |every, since| {
            Pity::of(Some(&sjk_identity::KyberTables {
                tiers: Vec::new(),
                pity: Some(sjk_identity::KyberPity { every, since }),
            }))
        };
        assert_eq!(
            pity_line(pity(10, 3)),
            "Legendary or better guaranteed within 7 crystals"
        );
        assert_eq!(
            pity_line(pity(10, 0)),
            "Legendary or better guaranteed within 10 crystals"
        );
        assert_eq!(
            pity_line(pity(10, 9)),
            "Your next crystal is Legendary or better, guaranteed"
        );
        assert_eq!(pity(10, 40).map(Pity::within), Some(1), "kept in range");
        assert_eq!(pity(0, 0), None, "no pity at that hub");
        assert_eq!(Pity::of(None), None);
        assert_eq!(
            pity_line(None),
            "Legendary or better guaranteed at least every 10 crystals"
        );
    }

    /// Every achievement brings a crystal, by the spec's tables (21/10, 26/10 and 29/10/2026):
    /// seventeen Uncommon, twenty Rare, four Legendary, one Mythical. The hub's word
    /// replaces the copy.
    #[test]
    fn every_achievement_brings_its_crystal() {
        for kind in &crate::achievements::ALL {
            assert!(
                REWARDS.iter().any(|(id, _)| *id == kind.id),
                "{} has no reward",
                kind.id
            );
        }
        let mut per_tier = [0; COUNT];
        for (id, index) in REWARDS {
            assert!(crate::achievements::find(id).is_some(), "{id}");
            per_tier[index] += 1;
        }
        assert_eq!(per_tier, [32, 39, 6, 1]);
        assert_eq!(reward("kills_1000").map(|t| t.id), Some("kyber_mythical"));
        assert_eq!(reward("hours_100").map(|t| t.id), Some("kyber_legendary"));
        assert_eq!(reward("nope"), None);
        let said = |id: &str, reward: Option<&str>| sjk_identity::Achievement {
            id: id.to_owned(),
            progress: 1,
            goal: 1,
            unlocked: 0,
            reward: reward.map(str::to_owned),
            name: None,
            description: None,
            group: None,
        };
        // The hub says another tier for one, nothing for another, an unknown one (worked
        // out without the shared table, which the other tests read).
        let bug = crate::achievements::ALL
            .iter()
            .position(|kind| kind.id == "bug_reporter")
            .unwrap();
        assert_eq!(
            said_of(&said("bug_reporter", Some("kyber_rare"))),
            Some((bug, 2))
        );
        assert_eq!(said_of(&said("first_blood", None)), None);
        assert_eq!(
            said_of(&said("storyteller", Some("kyber_from_the_future"))),
            None
        );
        assert_eq!(said_of(&said("nope", Some("kyber_rare"))), None);
        let hub = |at: usize| if at == bug { 2 } else { 0 };
        assert_eq!(
            reward_with("bug_reporter", hub).map(|t| t.id),
            Some("kyber_rare")
        );
        assert_eq!(
            reward_with("first_blood", hub).map(|t| t.id),
            Some("kyber_uncommon")
        );
    }

    #[test]
    fn an_arrival_says_which_achievement_earned_it_when_the_hub_said() {
        assert_eq!(Earned::of("").to_string(), "Earned from an achievement");
        assert_eq!(Earned::of("first_blood").to_string(), "Earned: First Blood");
        assert_eq!(Earned::of("Centurion").to_string(), "Earned: Centurion");
        assert_eq!(
            Earned::of("something else").to_string(),
            "Earned from an achievement"
        );
    }
}
