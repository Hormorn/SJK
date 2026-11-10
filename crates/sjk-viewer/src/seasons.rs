//! Holocron seasons (`docs/cards.md`, "Seasons"; the hub's `PROTOCOL.md`, "Seasons",
//! 10/11/2026): what the client says about a Beta holocron's guaranteed prizes, bonus
//! rolls and the pity, from the tables the hub sends with the progress
//! ([`sjk_identity::SeasonTables`]). A holocron's season is its edition
//! ([`crate::editions`]); an Alpha one opens as it always did, so nothing here shows
//! until the player holds a Beta holocron or the loot table's Beta page is asked for.

use crate::cards::{FINISHES, RARITIES, loot};
use sjk_identity::{SeasonBonus, SeasonPity, SeasonTables, SeasonTier};

/// How Beta holocrons differ, in a line.
pub(crate) const ABOUT: &str = "Beta holocrons of Rare and above give guaranteed prizes and bonus rolls on top, and count towards a pity.";

/// The odds `odds` (per mille) as "Name 70% · Name 25%", leaving out the zeros.
fn shares(names: &[&str], odds: &[u32], whole: u32) -> String {
    names
        .iter()
        .zip(odds)
        .filter(|(_, odd)| **odd > 0)
        .map(|(name, odd)| {
            format!(
                "{name} {}",
                loot::percent(f64::from(*odd) / f64::from(whole))
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The display name of shader tier `id` (`rare`), as the cards' rarities read.
fn tier_name(id: &str) -> &'static str {
    RARITIES
        .iter()
        .find(|rarity| rarity.id == id)
        .map_or("", |rarity| rarity.name)
}

/// What a bonus roll may bring, as a line.
fn bonus_text(bonus: &SeasonBonus) -> String {
    let whole = bonus.scale.max(1);
    let nothing =
        whole.saturating_sub(bonus.shader + bonus.tag + bonus.shards + bonus.banner + bonus.card);
    shares(
        &[
            "shader",
            "tag",
            "shard pile",
            "banner",
            "second card",
            "nothing",
        ],
        &[
            bonus.shader,
            bonus.tag,
            bonus.shards,
            bonus.banner,
            bonus.card,
            nothing,
        ],
        whole,
    )
}

/// The lines a Beta tier's page says: its card, its shader, its bonus rolls and faces.
pub(crate) fn tier_lines(tier: &SeasonTier) -> Vec<String> {
    let rarities: Vec<&str> = RARITIES.iter().map(|rarity| rarity.name).collect();
    let finishes: Vec<&str> = FINISHES.iter().map(|finish| finish.name).collect();
    let mut lines = vec![format!(
        "Card, guaranteed: {}; {}; with {} to {} credits.",
        shares(&rarities, &tier.rarity, 1_000),
        shares(&finishes, &tier.finish, 1_000),
        tier.credits[0],
        tier.credits[1]
    )];
    if let Some(floor) = &tier.shader_floor {
        let floor_at = RARITIES
            .iter()
            .position(|rarity| rarity.id == floor)
            .unwrap_or(0);
        let mut line = format!("Saber shader, guaranteed: {} or better", tier_name(floor));
        if let Some(odds) = &tier.shader_tier {
            let kept: Vec<u32> = odds
                .iter()
                .enumerate()
                .map(|(at, odd)| if at < floor_at { 0 } else { *odd })
                .collect();
            let whole: u32 = kept.iter().sum();
            if whole > 0 {
                line.push_str(&format!(" ({})", shares(&rarities, &kept, whole)));
            }
        }
        line.push_str(", one you do not hold first.");
        lines.push(line);
    }
    match tier.bonus.as_slice() {
        [] => {}
        [one] => lines.push(format!("Bonus roll on top: {}.", bonus_text(one))),
        many => lines.push(format!(
            "{} bonus rolls on top, each: {}.",
            many.len(),
            bonus_text(&many[0])
        )),
    }
    if tier.fresh_faces {
        lines.push("Its cards are ones you do not hold yet, while any is left.".to_owned());
    }
    lines
}

/// The pity, as a line ("Legendary or better within 12 Beta holocrons; Mythical within 70").
pub(crate) fn pity_line(pity: &SeasonPity) -> String {
    let within = |count: sjk_identity::PityCount| count.every.saturating_sub(count.since).max(1);
    let grand = within(pity.grand);
    let mythical = within(pity.mythical);
    format!(
        "Pity: Legendary or better within {grand} Beta holocron{}, Mythical within {mythical}",
        if grand == 1 { "" } else { "s" }
    )
}

/// The notes under the Beta page.
pub(crate) fn notes(tables: Option<&SeasonTables>) -> Vec<String> {
    let pity = tables.map_or_else(
        || "Pity: a prize of Legendary or better at the latest every 15 Beta holocrons, a Mythical one every 80.".to_owned(),
        |tables| {
            format!(
                "Pity: a prize of Legendary or better at the latest every {} Beta holocrons, a Mythical one every {}. {}.",
                tables.pity.grand.every, tables.pity.mythical.every, pity_line(&tables.pity)
            )
        },
    );
    vec![
        "A holocron keeps the season it dropped in: an Alpha one opens by the Alpha tables, whenever you open it.".to_owned(),
        "A Beta Uncommon holocron opens as an Alpha one, and counts towards the pity.".to_owned(),
        pity,
        "The pity raises the guaranteed shader when there is one, else the card. Tags, banners and shards do not count.".to_owned(),
        "Opening a Beta holocron needs the SJK release that brought Beta, or newer.".to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mythical() -> SeasonTier {
        serde_json::from_value(serde_json::json!({
            "tier":"mythical","rarity":[0,0,500,380,120],"finish":[0,850,150],
            "credits":[250,350],"shader_floor":"legendary","shader_tier":[50,150,350,330,120],
            "bonus":[{"shader":0,"tag":250,"shards":250,"banner":0,"card":250,"scale":750},
                     {"shader":0,"tag":250,"shards":250,"banner":0,"card":250,"scale":750}],
            "fresh_faces":true
        }))
        .unwrap()
    }

    #[test]
    fn a_beta_tier_says_its_card_shader_bonus_rolls_and_faces() {
        let lines = tier_lines(&mythical());
        assert_eq!(lines.len(), 4, "{lines:?}");
        assert!(lines[0].contains("Rare 50%"), "{}", lines[0]);
        assert!(!lines[0].contains("Common"), "{}", lines[0]);
        assert!(lines[0].contains("250 to 350 credits"), "{}", lines[0]);
        assert!(lines[1].contains("Legendary or better"), "{}", lines[1]);
        assert!(!lines[1].contains("Rare "), "{}", lines[1]);
        assert!(lines[2].starts_with("2 bonus rolls"), "{}", lines[2]);
        assert!(!lines[2].contains("banner"), "banners off: {}", lines[2]);
        assert!(!lines[2].contains("nothing"), "{}", lines[2]);
    }

    #[test]
    fn the_pity_line_counts_down() {
        let pity: SeasonPity = serde_json::from_value(serde_json::json!({
            "grand":{"every":15,"since":14},"mythical":{"every":80,"since":10}
        }))
        .unwrap();
        assert_eq!(
            pity_line(&pity),
            "Pity: Legendary or better within 1 Beta holocron, Mythical within 70"
        );
        assert_eq!(notes(None).len(), 5);
    }
}
