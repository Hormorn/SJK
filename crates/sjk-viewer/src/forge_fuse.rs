//! Fusion in the Forge (10/11/2026, Sol: "make crafting and the shop more relevant, people
//! barely use it"; `docs/cards.md`, "Fusion"): a number of duplicates of one tier, the
//! hub's `economy.fuse` table, makes one saber shader of the next tier the player does not
//! hold yet, chosen by the hub. Only on a hub whose economy says `fuse`
//! ([`crate::economy::fuse_need`]); Mythical (the last tier) and Unique never fuse.
//!
//! This holds the rules the Forge's Fuse section and the badges follow, apart from any
//! drawing: which copies may go ([`spares`]: of the trade, never a locked shader's, never
//! the last copy, a medal's or a card's copy not counted), what the Forge proposes
//! ([`propose`]: the most duplicated first), whether a choice may be sent ([`check`]) and
//! the request's ids ([`ids`]). The hub checks everything again.

use crate::unlockables::{Rarity, Unlockable};

/// The tiers that may fuse, in order: each into the one after it.
pub(crate) const TIERS: [Rarity; 4] = [
    Rarity::Common,
    Rarity::Uncommon,
    Rarity::Rare,
    Rarity::Legendary,
];

/// The tier a fusion of `tier` gives; `None` for Mythical and Unique.
pub(crate) fn next_tier(tier: Rarity) -> Option<Rarity> {
    match tier {
        Rarity::Common => Some(Rarity::Uncommon),
        Rarity::Uncommon => Some(Rarity::Rare),
        Rarity::Rare => Some(Rarity::Legendary),
        Rarity::Legendary => Some(Rarity::Mythical),
        Rarity::Mythical | Rarity::Unique => None,
    }
}

/// A shader of the trade the player holds more than once: its copies (a medal's or a
/// card's is never counted).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Spare {
    pub(crate) skin: &'static Unlockable,
    /// The copies held.
    pub(crate) count: u32,
}

impl Spare {
    /// The copies that may go: all but the last.
    pub(crate) fn spare(self) -> u32 {
        self.count.saturating_sub(1)
    }
}

/// Every shader of `tier` with copies to spare, the most duplicated first (the
/// catalogue's order between equals): of the trade, held (`held` gives the copies the
/// player's own, without a medal's or a card's) more than once, and not locked.
pub(crate) fn spares(
    tier: Rarity,
    held: impl Fn(&str) -> u32,
    locked: impl Fn(&str) -> bool,
) -> Vec<Spare> {
    let mut list: Vec<Spare> = crate::unlockables::blade_skins_by_tier()
        .iter()
        .copied()
        .filter(|skin| skin.tier == tier && crate::economy::in_trade(skin) && !locked(skin.id))
        .filter_map(|skin| {
            let count = held(skin.id);
            (count > 1).then_some(Spare { skin, count })
        })
        .collect();
    list.sort_by_key(|spare| std::cmp::Reverse(spare.count));
    list
}

/// The copies `spares` may give in all.
pub(crate) fn spare_total(spares: &[Spare]) -> u32 {
    spares.iter().map(|spare| spare.spare()).sum()
}

/// How many shaders of the tier after `tier` the player lacks (`owns` says whether they
/// hold any copy): what a fusion of `tier` can give.
pub(crate) fn targets(tier: Rarity, owns: impl Fn(&str) -> bool) -> usize {
    let Some(next) = next_tier(tier) else {
        return 0;
    };
    crate::unlockables::blade_skins_by_tier()
        .iter()
        .filter(|skin| skin.tier == next && crate::economy::in_trade(skin) && !owns(skin.id))
        .count()
}

/// The copies the Forge proposes for a fusion taking `need` of them: one at a time from
/// the shader with the most copies left, so the biggest piles shrink first and no last
/// copy ever goes; empty when `spares` cannot give `need`.
pub(crate) fn propose(spares: &[Spare], need: u32) -> Vec<(&'static str, u32)> {
    if need == 0 || spare_total(spares) < need {
        return Vec::new();
    }
    let mut taken = vec![0u32; spares.len()];
    for _ in 0..need {
        // The first of the fullest piles left (ties keep the list's order).
        let Some(at) = spares
            .iter()
            .enumerate()
            .filter(|(at, spare)| taken[*at] < spare.spare())
            .max_by(|(a, x), (b, y)| {
                (x.count - taken[*a])
                    .cmp(&(y.count - taken[*b]))
                    .then(b.cmp(a))
            })
            .map(|(at, _)| at)
        else {
            return Vec::new();
        };
        taken[at] += 1;
    }
    spares
        .iter()
        .zip(taken)
        .filter(|(_, copies)| *copies > 0)
        .map(|(spare, copies)| (spare.skin.id, copies))
        .collect()
}

/// Why a fusion cannot be sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Block {
    /// The hub does not fuse this tier.
    NoFusion,
    /// The player holds every shader of the next tier: nothing new to give.
    NothingNew,
    /// Fewer duplicates to spare than a fusion takes.
    Short { have: u32, need: u32 },
    /// The copies picked are not the count a fusion takes (or one picked is not spare).
    Picked { picked: u32, need: u32 },
}

/// Whether `picks` (shader, copies) may be sent as a fusion of `tier` taking `need`
/// (`None`: the hub does not fuse it), from `spares` of that tier, with `targets` shaders
/// of the next tier left to give.
pub(crate) fn check(
    tier: Rarity,
    picks: &[(&'static str, u32)],
    spares: &[Spare],
    need: Option<u32>,
    targets: usize,
) -> Result<(), Block> {
    let Some(need) = need.filter(|_| next_tier(tier).is_some()) else {
        return Err(Block::NoFusion);
    };
    if targets == 0 {
        return Err(Block::NothingNew);
    }
    let have = spare_total(spares);
    if have < need {
        return Err(Block::Short { have, need });
    }
    let picked: u32 = picks.iter().map(|(_, copies)| copies).sum();
    let fits = picks.iter().all(|(id, copies)| {
        spares
            .iter()
            .find(|spare| spare.skin.id == *id)
            .is_some_and(|spare| *copies <= spare.spare())
    });
    if picked != need || !fits {
        return Err(Block::Picked { picked, need });
    }
    Ok(())
}

/// What the player reads for `block` on a fusion of `tier`.
pub(crate) fn block_words(tier: Rarity, block: Block) -> String {
    let next = next_tier(tier).map_or("next", Rarity::label);
    match block {
        Block::NoFusion => format!("The SJK hub does not fuse {} shaders.", tier.label()),
        Block::NothingNew => {
            format!("You hold every {next} shader already: nothing new to fuse into.")
        }
        Block::Short { have, need } => format!(
            "{have} of {need} {} duplicates: {} more needed.",
            tier.label(),
            need - have
        ),
        Block::Picked { picked, need } => {
            format!("Pick {need} duplicates: {picked} picked.")
        }
    }
}

/// The fusion in a line: "3 duplicates of Common -> 1 Uncommon shader you do not own yet".
pub(crate) fn summary(tier: Rarity, need: u32) -> String {
    let next = next_tier(tier).map_or("new", Rarity::label);
    format!(
        "{need} duplicates of {} -> 1 {next} shader you do not own yet",
        tier.label()
    )
}

/// The request's ids: each picked shader once per copy.
pub(crate) fn ids(picks: &[(&'static str, u32)]) -> Vec<&'static str> {
    picks
        .iter()
        .flat_map(|(id, copies)| std::iter::repeat_n(*id, *copies as usize))
        .collect()
}

/// Whether some tier may fuse now: the hub fuses it (`need`, the duplicates a fusion of a
/// tier takes, [`crate::economy::fuse_need`] live), the player holds enough of its
/// duplicates (`held`, `locked` as [`spares`]) and lacks a shader of the next (`owns`).
pub(crate) fn any_possible(
    need: impl Fn(Rarity) -> Option<u32>,
    held: impl Fn(&str) -> u32,
    owns: impl Fn(&str) -> bool,
    locked: impl Fn(&str) -> bool,
) -> bool {
    TIERS.into_iter().any(|tier| {
        need(tier).is_some_and(|need| {
            spare_total(&spares(tier, &held, &locked)) >= need && targets(tier, &owns) > 0
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unlockables;

    /// The first `n` shaders of the trade of `tier`.
    fn of_tier(tier: Rarity, n: usize) -> Vec<&'static Unlockable> {
        unlockables::blade_skins_by_tier()
            .iter()
            .copied()
            .filter(|skin| skin.tier == tier && crate::economy::in_trade(skin))
            .take(n)
            .collect()
    }

    fn spare(skin: &'static Unlockable, count: u32) -> Spare {
        Spare { skin, count }
    }

    #[test]
    fn only_spare_copies_of_the_tier_are_listed_most_duplicated_first() {
        let commons = of_tier(Rarity::Common, 3);
        let rare = of_tier(Rarity::Rare, 1)[0];
        let counts = [
            (commons[0].id, 2),
            (commons[1].id, 5),
            (commons[2].id, 1),
            (rare.id, 4),
        ];
        let held = |id: &str| {
            counts
                .iter()
                .find(|(known, _)| *known == id)
                .map_or(0, |(_, count)| *count)
        };
        let list = spares(Rarity::Common, held, |_| false);
        assert_eq!(
            list.iter()
                .map(|s| (s.skin.id, s.count))
                .collect::<Vec<_>>(),
            [(commons[1].id, 5), (commons[0].id, 2)],
            "the last copy and other tiers left out"
        );
        assert_eq!(spare_total(&list), 5);
        // A locked shader is never listed.
        let locked = spares(Rarity::Common, held, |id| id == commons[1].id);
        assert_eq!(locked.len(), 1);
        // A medal's blade is never of the trade.
        for medal in crate::medals::Medal::ALL {
            if let Some(id) = medal.blade() {
                let skin = unlockables::find(id).unwrap();
                assert!(
                    spares(skin.tier, |_| 9, |_| false)
                        .iter()
                        .all(|s| s.skin.id != id)
                );
            }
        }
    }

    #[test]
    fn the_proposal_takes_the_exact_count_from_the_biggest_piles_never_the_last_copy() {
        let commons = of_tier(Rarity::Common, 3);
        let list = [
            spare(commons[0], 5),
            spare(commons[1], 3),
            spare(commons[2], 2),
        ];
        let picks = propose(&list, 3);
        assert_eq!(picks.iter().map(|(_, n)| n).sum::<u32>(), 3);
        // 5, 3, 2 copies: two from the five, then the five (now three) is level with the
        // three and comes first in the list.
        assert_eq!(picks, [(commons[0].id, 3)]);
        // Everything spare, and never more.
        let all = propose(&list, 7);
        assert_eq!(
            all,
            [(commons[0].id, 4), (commons[1].id, 2), (commons[2].id, 1)]
        );
        assert_eq!(propose(&list, 6), [(commons[0].id, 4), (commons[1].id, 2)]);
        for (id, copies) in &all {
            let held = list.iter().find(|s| s.skin.id == *id).unwrap();
            assert!(*copies < held.count, "the last copy stays");
        }
        assert!(propose(&list, 8).is_empty(), "not enough");
        assert!(propose(&list, 0).is_empty());
        assert!(propose(&[], 3).is_empty());
        assert_eq!(ids(&picks).len(), 3);
        assert_eq!(ids(&picks)[..2], [commons[0].id, commons[0].id]);
    }

    #[test]
    fn a_fusion_is_checked_by_count_tier_and_what_is_left_to_gain() {
        let commons = of_tier(Rarity::Common, 2);
        let list = [spare(commons[0], 3), spare(commons[1], 2)];
        let picks = propose(&list, 3);
        assert_eq!(check(Rarity::Common, &picks, &list, Some(3), 4), Ok(()));
        assert_eq!(
            check(Rarity::Common, &picks, &list, None, 4),
            Err(Block::NoFusion)
        );
        assert_eq!(
            check(Rarity::Mythical, &picks, &list, Some(3), 4),
            Err(Block::NoFusion),
            "the last tier never fuses"
        );
        assert_eq!(
            check(Rarity::Common, &picks, &list, Some(3), 0),
            Err(Block::NothingNew)
        );
        assert_eq!(
            check(Rarity::Common, &picks, &list, Some(5), 4),
            Err(Block::Short { have: 3, need: 5 })
        );
        let short = [(commons[0].id, 2)];
        assert_eq!(
            check(Rarity::Common, &short, &list, Some(3), 4),
            Err(Block::Picked { picked: 2, need: 3 })
        );
        // The last copy of one shader is never sent, even at the right count.
        let last = [(commons[0].id, 1), (commons[1].id, 2)];
        assert!(check(Rarity::Common, &last, &list, Some(3), 4).is_err());
        assert_eq!(
            summary(Rarity::Common, 3),
            "3 duplicates of Common -> 1 Uncommon shader you do not own yet"
        );
        assert_eq!(
            block_words(Rarity::Rare, Block::Short { have: 1, need: 3 }),
            "1 of 3 Rare duplicates: 2 more needed."
        );
    }

    #[test]
    fn the_targets_are_the_next_tiers_shaders_not_held() {
        let uncommons = of_tier(Rarity::Uncommon, 200);
        assert_eq!(targets(Rarity::Common, |_| false), uncommons.len());
        assert_eq!(targets(Rarity::Common, |_| true), 0);
        assert_eq!(targets(Rarity::Mythical, |_| false), 0);
        assert_eq!(next_tier(Rarity::Legendary), Some(Rarity::Mythical));
        assert_eq!(next_tier(Rarity::Unique), None);
    }
}
