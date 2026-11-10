//! Tag copies (Blep's idea, 03/11/2026; `docs/client.md`, "Tag"): on a hub whose economy
//! says `tag_copies`, a duplicate tag copy can be broken down into kyber shards, traded,
//! or given with a second copy of the same tier and finish for a tag the player does not
//! hold yet. The player always keeps one copy of every tag, and the free tags never take
//! part. The hub decides; this is what the client offers and says beforehand, from the
//! hub's tables (`economy.tags`).

use crate::tags::{CATALOGUE, Held, Tag};
use sjk_identity::{TagCopy, TagTables, Unlock};

/// Whether the hub of `economy` breaks down, crafts and trades tag copies.
pub(crate) fn on(economy: Option<&sjk_identity::EconomyTables>) -> bool {
    economy.is_some_and(|tables| tables.can(sjk_identity::TAG_COPIES_ABILITY))
}

/// The copies of `held` in finish `finish` (`normal`, `holo`, `solar`).
pub(crate) fn in_finish(held: Held, finish: &str) -> u32 {
    match finish {
        "holo" => held.holo,
        "solar" => held.solar,
        _ => held.copies.saturating_sub(held.holo + held.solar),
    }
}

/// How many copies of `held` in `finish` may go (broken down, crafted, traded): the
/// player keeps one copy of the tag, of any finish.
pub(crate) fn spare(held: Held, finish: &str) -> u32 {
    in_finish(held, finish).min(held.copies.saturating_sub(1))
}

/// Whether `tag` takes part: one that drops (the free ones never do).
pub(crate) fn takes_part(tag: &Tag) -> bool {
    !tag.free
}

/// The shards a copy of `tag` in `finish` breaks into, by the hub's `tables`.
pub(crate) fn shatter_yield(tables: &TagTables, tag: &Tag, finish: &str) -> Option<u64> {
    tables.yield_of(tag.tier as usize, finish)
}

/// What the player holds of each tag, from the profile's `unlocks`.
fn held_of(unlocks: &[Unlock], tag: &Tag) -> Held {
    Held::of_parts(tag.id, Some(unlocks), None)
}

/// Whether a craft in `finish` of tier `tag.tier` could bring something new: a tag of
/// that tier the player lacks altogether, or lacks in that finish (as the hub picks).
pub(crate) fn something_new(unlocks: &[Unlock], tag: &Tag, finish: &str) -> bool {
    CATALOGUE
        .iter()
        .filter(|other| takes_part(other) && other.tier == tag.tier)
        .any(|other| in_finish(held_of(unlocks, other), finish) == 0)
}

/// The two copies a craft from `tag` in `finish` gives: this tag's spare copies first,
/// then the other tags of its tier with the most spare in that finish. `None` when two
/// are not to be had.
pub(crate) fn craft_inputs(unlocks: &[Unlock], tag: &Tag, finish: &str) -> Option<[TagCopy; 2]> {
    let mut sources: Vec<(&Tag, u32)> = CATALOGUE
        .iter()
        .filter(|other| takes_part(other) && other.tier == tag.tier)
        .map(|other| (other, spare(held_of(unlocks, other), finish)))
        .filter(|(_, spare)| *spare > 0)
        .collect();
    sources.sort_by_key(|(other, spare)| (other.id != tag.id, std::cmp::Reverse(*spare)));
    let mut picked = Vec::with_capacity(2);
    for (other, spare) in sources {
        for _ in 0..spare.min(2) {
            if picked.len() < 2 {
                picked.push(TagCopy::new(other.id, finish));
            }
        }
    }
    match picked.as_slice() {
        [first, second] if first.id == tag.id || second.id == tag.id => {
            Some([first.clone(), second.clone()])
        }
        _ => None,
    }
}

/// A copy's name as players read it: "Holo Gonk tag", "Kyber tag".
pub(crate) fn copy_name(copy: &TagCopy) -> String {
    let name = crate::tags::get(&copy.id).map_or(copy.id.as_str(), |tag| tag.name);
    match crate::cards::Finish::from_id(&copy.finish).filter(|finish| finish.index > 0) {
        Some(finish) => format!("{} {name}", finish.name),
        None => name.to_owned(),
    }
}

/// What came of a tag copies request, in the player's words.
pub(crate) fn outcome_words(outcome: &sjk_identity::TagOutcome) -> String {
    if !outcome.done() {
        return crate::terms::say_owned(outcome.message.clone());
    }
    match (&outcome.trade, &outcome.crafted) {
        (sjk_identity::TagTrade::Shatter(copy), _) => format!(
            "{} broken down: +{} kyber shards.",
            copy_name(copy),
            crate::economy::shards_text(outcome.shards)
        ),
        (sjk_identity::TagTrade::Craft(_), Some(crafted)) => format!(
            "Crafted: {}. It is yours.",
            copy_name(&TagCopy::new(&crafted.id, &crafted.finish))
        ),
        (sjk_identity::TagTrade::Craft(_), None) => "Crafted a new tag.".to_owned(),
        (sjk_identity::TagTrade::Gif(_), _) => {
            "Made: your GIF tag. It is yours for good.".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlock(id: &str, count: u32, holo: u32, solar: u32) -> Unlock {
        Unlock {
            id: id.to_owned(),
            granted: 0,
            note: String::new(),
            count,
            medal: None,
            card: None,
            holo,
            solar,
        }
    }

    fn tag(id: &str) -> &'static Tag {
        crate::tags::get(id).unwrap()
    }

    #[test]
    fn outcomes_read_in_the_players_words() {
        let copy = TagCopy::new("tag_gonk", "holo");
        let shattered = sjk_identity::TagOutcome {
            serial: 1,
            trade: sjk_identity::TagTrade::Shatter(copy.clone()),
            shards: 25,
            crafted: None,
            message: String::new(),
        };
        assert_eq!(
            outcome_words(&shattered),
            "Holo Gonk tag broken down: +25 kyber shards."
        );
        let crafted = sjk_identity::TagOutcome {
            serial: 2,
            trade: sjk_identity::TagTrade::Craft([copy.clone(), copy]),
            shards: 0,
            crafted: Some(sjk_identity::TagCrafted {
                id: "tag_kyber".to_owned(),
                name: "Kyber tag".to_owned(),
                tier: "uncommon".to_owned(),
                finish: "normal".to_owned(),
                gave: Vec::new(),
            }),
            message: String::new(),
        };
        assert_eq!(outcome_words(&crafted), "Crafted: Kyber tag. It is yours.");
    }

    #[test]
    fn one_copy_of_each_tag_is_always_kept() {
        let held = Held {
            copies: 3,
            holo: 1,
            solar: 0,
        };
        assert_eq!(in_finish(held, "normal"), 2);
        assert_eq!(spare(held, "normal"), 2);
        assert_eq!(spare(held, "holo"), 1);
        let one = Held {
            copies: 1,
            holo: 1,
            solar: 0,
        };
        assert_eq!(spare(one, "holo"), 0, "the last copy stays");
        assert!(!takes_part(tag("tag_sjk")));
        assert!(takes_part(tag("tag_gonk")));
    }

    #[test]
    fn a_yield_is_the_tier_times_the_finish() {
        let tables: TagTables = serde_json::from_str(
            r#"{"shatter":[2,5,12,30,75],"finish":{"normal":1,"holo":5,"solar":25}}"#,
        )
        .unwrap();
        assert_eq!(shatter_yield(&tables, tag("tag_gonk"), "normal"), Some(5));
        assert_eq!(shatter_yield(&tables, tag("tag_gonk"), "holo"), Some(25));
        assert_eq!(
            shatter_yield(&tables, tag("tag_solar_fox"), "solar"),
            Some(1_875)
        );
    }

    #[test]
    fn a_craft_takes_two_spare_copies_of_one_tier_and_finish() {
        // Two spare Gonks: both from it.
        let unlocks = [unlock("tag_gonk", 3, 0, 0)];
        let inputs = craft_inputs(&unlocks, tag("tag_gonk"), "normal").unwrap();
        assert!(inputs.iter().all(|copy| copy.id == "tag_gonk"));
        assert!(something_new(&unlocks, tag("tag_gonk"), "normal"));
        // One spare Gonk and one spare Kyber (both Uncommon).
        let unlocks = [unlock("tag_gonk", 2, 0, 0), unlock("tag_kyber", 2, 0, 0)];
        let inputs = craft_inputs(&unlocks, tag("tag_gonk"), "normal").unwrap();
        assert_eq!(inputs[0].id, "tag_gonk");
        assert_eq!(inputs[1].id, "tag_kyber");
        // Not two of that finish, or another tier only.
        let unlocks = [unlock("tag_gonk", 2, 1, 0), unlock("tag_holocron", 3, 0, 0)];
        assert_eq!(craft_inputs(&unlocks, tag("tag_gonk"), "holo"), None);
        assert_eq!(craft_inputs(&unlocks, tag("tag_gonk"), "normal"), None);
        // Every Legendary held in Normal: nothing new.
        let unlocks = [
            unlock("tag_force_push", 3, 0, 0),
            unlock("tag_dark_mask", 1, 0, 0),
        ];
        assert!(!something_new(&unlocks, tag("tag_force_push"), "normal"));
        assert!(something_new(&unlocks, tag("tag_force_push"), "holo"));
    }
}
