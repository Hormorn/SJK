//! The holocron economy on this client (`docs/cards.md`, "Shards and the shop"): kyber
//! shards beside credits, what shattering a duplicate gives and crafting costs, the shop's
//! prices (in shards since 17/10/2026, Sol's decision), what a holocron costs in credits,
//! and the requests that trade. The hub decides everything; this keeps its tables
//! ([`current`]) and says what a screen may offer.
//!
//! Only shaders of the *trade* take part: the ones holocrons drop, less any a medal brings
//! ([`in_trade`], read from the catalogue and the medals, as the hub's
//! `economy::in_trade`). Cards are luck only, hats and capes free for everyone: neither is
//! ever sold, crafted or shattered.
//!
//! The Collection's Forge tab (21/10/2026, `crate::console::collection_panel::forge`)
//! trades through these: [`in_trade`], [`shatter_yield`], [`craft_cost`],
//! [`craft_price`] (a held blade card's copy crafts its shader for less, [`blade_cards`]),
//! [`may_act`],
//! [`shatter`], [`shatter_last`], [`shatter_all`], [`craft`], [`buy_holocron`] and
//! [`outcome_words`]; the shop's offers are read and bought through
//! `crate::player_identity`. Before, Shatter and Craft stood on the
//! Shaders tab's cards and the shop over the Holocrons tab.

use crate::unlockables::{Rarity, Unlockable};
use std::sync::Mutex;

/// The shader tiers, Common to Mythical (Unique is never traded).
pub(crate) const TIERS: usize = 5;

/// The economy's tables by shader tier, Common to Mythical.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Tables {
    /// The kyber shards shattering one duplicate gives.
    pub(crate) shatter: [u64; TIERS],
    /// The kyber shards crafting one costs.
    pub(crate) craft: [u64; TIERS],
    /// What one costs in the shop, in shards ([`Tables::shop_in_shards`]) or credits.
    pub(crate) shop: [u64; TIERS],
    /// Whether the shop sells for kyber shards (hubs since 17/10/2026) or credits (before).
    pub(crate) shop_in_shards: bool,
    /// What an Uncommon and a Rare holocron cost in credits; 0 when not for sale.
    pub(crate) holocrons: [u64; 2],
    /// What crafting a shader costs while the player holds a copy of its blade card, in
    /// per cent of the craft cost, by the best copy's finish ([`crate::cards::FINISHES`]'
    /// order); 100 is no discount, as hubs before 21/10/2026 take.
    pub(crate) blade_card_craft: [u64; 3],
    /// The duplicates one fusion takes by the input shaders' tier, 0 for a tier that does
    /// not fuse (10/11/2026; all 0 on a hub that does not say `fuse`).
    pub(crate) fuse: [u32; TIERS],
}

/// This client's copy of the hub's tables (`src/economy.rs` there, provisional
/// 17/10/2026), shown until the hub sends its own with the progress.
pub(crate) const BUILT_IN: Tables = Tables {
    shatter: [5, 10, 25, 60, 150],
    craft: [40, 100, 300, 800, 2_000],
    shop: [30, 75, 225, 600, 1_500],
    shop_in_shards: true,
    holocrons: [150, 400],
    blade_card_craft: [75, 50, 50],
    // No fusion until the hub says it fuses.
    fuse: [0; TIERS],
};

/// The holocron tiers credits may buy, by the hub's ids, in [`Tables::holocrons`]' order.
pub(crate) const HOLOCRONS_FOR_SALE: [&str; 2] = ["uncommon", "rare"];

static HUB: Mutex<Option<Tables>> = Mutex::new(None);

/// The tables to show: the hub's last that fit, else [`BUILT_IN`].
pub(crate) fn current() -> Tables {
    HUB.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .unwrap_or(BUILT_IN)
}

/// Take the tables the hub sent last (`Snapshot::economy`); nothing changes when it sent
/// none or tables this client cannot show (not five tiers, or a zero).
pub(crate) fn follow(economy: Option<&sjk_identity::EconomyTables>) {
    if let Some(table) = economy.and_then(of_hub) {
        *HUB.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(table);
    }
}

/// The hub's tables as this client keeps them, if they fit.
fn of_hub(economy: &sjk_identity::EconomyTables) -> Option<Tables> {
    let row = |values: &[u64]| -> Option<[u64; TIERS]> {
        let row: [u64; TIERS] = values.try_into().ok()?;
        row.iter().all(|value| *value > 0).then_some(row)
    };
    Some(Tables {
        shatter: row(&economy.shatter)?,
        craft: row(&economy.craft)?,
        shop: row(&economy.shop)?,
        shop_in_shards: economy.shop_currency == "shards",
        holocrons: HOLOCRONS_FOR_SALE.map(|tier| economy.holocrons.get(tier).copied().unwrap_or(0)),
        blade_card_craft: crate::cards::FINISHES.map(|finish| {
            economy
                .blade_card_craft
                .get(finish.id)
                .copied()
                .filter(|share| (1..=100).contains(share))
                .unwrap_or(100)
        }),
        fuse: std::array::from_fn(|tier| economy.fuse_count(tier).unwrap_or(0)),
    })
}

/// Whether the shop sells for kyber shards (else credits, as hubs before 17/10/2026).
pub(crate) fn shop_in_shards() -> bool {
    current().shop_in_shards
}

/// The holocrons credits may buy now: each tier's id and its price in credits.
pub(crate) fn holocrons_for_sale() -> Vec<(&'static str, u64)> {
    HOLOCRONS_FOR_SALE
        .into_iter()
        .zip(current().holocrons)
        .filter(|(_, price)| *price > 0)
        .collect()
}

/// Whether `skin` takes part in the trade: holocrons drop it and no medal brings it.
pub(crate) fn in_trade(skin: &Unlockable) -> bool {
    skin.droppable
        && !crate::medals::Medal::ALL
            .iter()
            .any(|medal| medal.blade() == Some(skin.id))
}

/// Tier `tier`'s entry of a table row: 0 for Unique, which the tables leave out (no Unique
/// shader is ever traded, [`in_trade`]).
fn of_tier(row: [u64; TIERS], tier: Rarity) -> u64 {
    row.get(tier as usize).copied().unwrap_or(0)
}

/// The kyber shards shattering one duplicate of tier `tier` gives.
pub(crate) fn shatter_yield(tier: Rarity) -> u64 {
    of_tier(current().shatter, tier)
}

/// The kyber shards crafting a shader of tier `tier` costs.
pub(crate) fn craft_cost(tier: Rarity) -> u64 {
    of_tier(current().craft, tier)
}

/// The duplicates of tier `tier` one fusion takes, when the hub fuses that tier into the
/// next (10/11/2026); `None` otherwise (Mythical and Unique never fuse).
pub(crate) fn fuse_need(tier: Rarity) -> Option<u32> {
    let need = current().fuse.get(tier as usize).copied().unwrap_or(0);
    (need > 0 && crate::forge_fuse::next_tier(tier).is_some()).then_some(need)
}

/// The shop's table price of a shader of tier `tier`, what a deal of the day halves.
pub(crate) fn table_price(tier: Rarity) -> u64 {
    of_tier(current().shop, tier)
}

/// What a shader of tier `tier` costs in the shop (the shop itself shows the prices the
/// hub sends with its offers; this is the table's).
#[cfg(test)]
pub(crate) fn shop_price(tier: Rarity) -> u64 {
    of_tier(current().shop, tier)
}

/// A count of shards as players read it: "1 250".
pub(crate) fn shards_text(shards: u64) -> String {
    crate::cards::grouped(shards)
}

/// The finish of the best revealed copy of a blade card the player holds, by the shader
/// it is the blade card of: each of `cards` (the player's) whose face in `faces` names a
/// `blade` this client knows, the best finish kept for each shader.
pub(crate) fn blade_cards(
    cards: &[sjk_identity::CardStack],
    faces: &crate::cards::faces::Faces,
) -> Vec<(&'static str, &'static crate::cards::Finish)> {
    let mut best: Vec<(&'static str, &'static crate::cards::Finish)> = Vec::new();
    for stack in crate::cards::stacks(cards) {
        let Some(skin) = stack
            .card
            .as_deref()
            .and_then(|id| faces.find(id))
            .and_then(|(_, face)| crate::unlockables::find(face.blade.as_deref()?))
        else {
            continue;
        };
        match best.iter_mut().find(|(id, _)| *id == skin.id) {
            Some(kept) if kept.1.index < stack.finish.index => kept.1 = stack.finish,
            Some(_) => {}
            None => best.push((skin.id, stack.finish)),
        }
    }
    best
}

/// The finish of the best revealed copy among `cards` (the player's) of a blade card of
/// `skin` (`faces`' cards whose `blade` it is), `None` without one.
pub(crate) fn blade_card(
    skin: &Unlockable,
    cards: &[sjk_identity::CardStack],
    faces: &crate::cards::faces::Faces,
) -> Option<&'static crate::cards::Finish> {
    blade_cards(cards, faces)
        .into_iter()
        .find(|(id, _)| *id == skin.id)
        .map(|(_, finish)| finish)
}

/// The per cent of the craft cost paid with a copy of finish `finish` of the shader's
/// blade card (100: none, or a hub that gives no discount).
pub(crate) fn blade_card_share(finish: &crate::cards::Finish) -> u64 {
    current().blade_card_craft[finish.index]
}

/// What crafting `skin` costs in kyber shards while the player holds, at best, a copy of
/// its blade card of finish `card` ([`blade_card`]; 21/10/2026), and that copy's finish
/// with how much it took off in per cent when it lowered the cost (`None` at full cost).
pub(crate) fn craft_price(
    skin: &Unlockable,
    card: Option<&'static crate::cards::Finish>,
) -> (u64, Option<(&'static crate::cards::Finish, u64)>) {
    let full = craft_cost(skin.tier);
    let off = card
        .map(|finish| (finish, 100 - blade_card_share(finish)))
        .filter(|(_, off)| *off > 0);
    (off.map_or(full, |(_, off)| full * (100 - off) / 100), off)
}

/// Whether `snapshot` lets the player open holocrons and trade: the hub said they may
/// (`can_open`) and this build is not older than the hub's minimum client.
pub(crate) fn may_act(snapshot: &sjk_identity::Snapshot) -> bool {
    snapshot.holocrons.is_some_and(|progress| progress.can_open) && update_to(snapshot).is_none()
}

/// The SJK release to update to when this build is older than the hub's minimum client
/// (the hub refuses it opening and trading then), else `None`.
pub(crate) fn update_to(snapshot: &sjk_identity::Snapshot) -> Option<&str> {
    snapshot
        .min_client
        .as_deref()
        .filter(|min| !crate::build_info::meets_min(min))
}

/// What the Holocrons tab says in place of Open and Open all when this build is older
/// than `min`, the hub's minimum client.
pub(crate) fn update_line(min: &str) -> String {
    crate::terms::say_owned(format!(
        "Update SJK to open holocrons: this needs SJK {min} or newer."
    ))
}

/// Shatter one duplicate of shader `id`; false when the identity has not started. What
/// came of it arrives in the snapshot's `trade`.
pub(crate) fn shatter(id: &str) -> bool {
    crate::player_identity::with_service(|service| service.shatter(id)).is_some()
}

/// Break shader `id` down, its last copy too (a hub whose economy says `shatter_last`);
/// false when the identity has not started. What came of it arrives in the snapshot's `trade`.
pub(crate) fn shatter_last(id: &str) -> bool {
    crate::player_identity::with_service(|service| service.shatter_last(id)).is_some()
}

/// Break every duplicate down, one copy of each kept (a hub whose economy says
/// `shatter_duplicates`); false when the identity has not started. What came of it
/// arrives in the snapshot's `trade`.
pub(crate) fn shatter_all() -> bool {
    crate::player_identity::with_service(|service| service.shatter_duplicates()).is_some()
}

/// Break the copies `items` names down at once (shader, copies), last copies too when
/// `last` (a hub whose economy says `shatter_many`, 05/11/2026); false when the identity
/// has not started. What came of it arrives in the snapshot's `trade`.
pub(crate) fn shatter_many(items: &[(&'static str, u32)], last: bool) -> bool {
    let items: Vec<(String, u32)> = items
        .iter()
        .map(|(id, copies)| ((*id).to_owned(), *copies))
        .collect();
    crate::player_identity::with_service(|service| service.shatter_many(items, last)).is_some()
}

/// Craft shader `id` with kyber shards; false when the identity has not started. What
/// came of it arrives in the snapshot's `trade`.
pub(crate) fn craft(id: &str) -> bool {
    crate::player_identity::with_service(|service| service.craft(id)).is_some()
}

/// Fuse the duplicates `ids` names (each shader once per copy, all of one tier) into a
/// shader of the next tier the player lacks (a hub whose economy says `fuse`, 10/11/2026);
/// false when the identity has not started. What came of it arrives in the snapshot's
/// `trade`.
pub(crate) fn fuse(ids: &[&'static str]) -> bool {
    let ids: Vec<String> = ids.iter().map(|id| (*id).to_owned()).collect();
    crate::player_identity::with_service(|service| service.fuse(ids)).is_some()
}

/// Buy a holocron of tier `tier` with credits; false when the identity has not started.
/// What came of it arrives in the snapshot's `trade`, the holocron in the player's list.
pub(crate) fn buy_holocron(tier: &str) -> bool {
    crate::player_identity::with_service(|service| service.buy_holocron(tier)).is_some()
}

/// What a trade's outcome says, in a line: "Kamino blade broken down: +5 kyber shards.",
/// or why it failed. `name` is the shader's name (a holocron's tier name for one bought).
pub(crate) fn outcome_words(outcome: &sjk_identity::TradeOutcome, name: &str) -> String {
    use sjk_identity::Trade;
    if !outcome.done() {
        // Several shaders at once: the one the hub stopped at, named (05/11/2026).
        if outcome.trade == sjk_identity::Trade::ShatterMany
            && let Some(skin) = crate::unlockables::find(&outcome.id)
        {
            return crate::terms::say_owned(format!("{}: {}", skin.name, outcome.message));
        }
        return crate::terms::say_owned(outcome.message.clone());
    }
    let amount = shards_text(outcome.amount);
    match outcome.trade {
        Trade::Shatter => format!("{name} broken down: +{amount} kyber shards."),
        Trade::ShatterLast => {
            format!("{name} broken down: +{amount} kyber shards. It has left your collection.")
        }
        Trade::ShatterAll => format!(
            "{} {} broken down: +{amount} kyber shards.",
            outcome.copies,
            if outcome.copies == 1 {
                "duplicate"
            } else {
                "duplicates"
            }
        ),
        Trade::ShatterMany => format!(
            "{} {} broken down: +{amount} kyber shards.",
            outcome.copies,
            if outcome.copies == 1 {
                "copy"
            } else {
                "copies"
            }
        ),
        Trade::Craft => format!("{name} crafted for {amount} kyber shards. It is yours."),
        // The shader gained is the outcome's: the catalogue names it.
        Trade::Fuse => format!(
            "{} {} fused into {}. It is yours.",
            outcome.copies,
            if outcome.copies == 1 {
                "duplicate"
            } else {
                "duplicates"
            },
            crate::unlockables::find(&outcome.id).map_or(name, |skin| skin.name)
        ),
        Trade::Buy if shop_in_shards() => {
            format!("{name} bought for {amount} kyber shards. It is yours.")
        }
        Trade::Buy => format!("{name} bought for {amount} credits. It is yours."),
        Trade::Holocron => crate::terms::say_owned(format!(
            "{name} bought for {} credits. It waits in your holocrons.",
            crate::cards::credits_text(outcome.amount)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unlockables;

    /// The copy matches the hub's (`src/economy.rs`): the same numbers there, pinned by
    /// both sides' tests.
    #[test]
    fn the_built_in_tables_are_the_hubs() {
        assert_eq!(BUILT_IN.shatter, [5, 10, 25, 60, 150]);
        assert_eq!(BUILT_IN.craft, [40, 100, 300, 800, 2_000]);
        assert_eq!(BUILT_IN.shop, [30, 75, 225, 600, 1_500]);
        const { assert!(BUILT_IN.shop_in_shards) };
        assert_eq!(BUILT_IN.holocrons, [150, 400]);
        assert_eq!(BUILT_IN.blade_card_craft, [75, 50, 50]);
        for table in [BUILT_IN.shatter, BUILT_IN.craft, BUILT_IN.shop] {
            assert!(table.windows(2).all(|pair| pair[0] < pair[1]));
        }
        for tier in HOLOCRONS_FOR_SALE {
            assert!(crate::holocrons::TIERS.iter().any(|known| known.id == tier));
        }
    }

    #[test]
    fn the_hubs_tables_replace_the_copy_only_when_they_fit() {
        let wire = |shatter: Vec<u64>| sjk_identity::EconomyTables {
            shatter,
            craft: vec![1, 2, 3, 4, 5],
            shop: vec![10, 20, 30, 40, 50],
            ..Default::default()
        };
        assert_eq!(
            of_hub(&wire(vec![9, 8, 7, 6, 5])).map(|t| t.shatter),
            Some([9, 8, 7, 6, 5])
        );
        // A hub before the shop sold for shards: credits, no holocrons for sale.
        let older = of_hub(&wire(vec![1, 2, 3, 4, 5])).unwrap();
        assert_eq!((older.shop_in_shards, older.holocrons), (false, [0, 0]));
        let newer = of_hub(&sjk_identity::EconomyTables {
            shop_currency: "shards".to_owned(),
            holocrons: [("rare".to_owned(), 400)].into_iter().collect(),
            ..wire(vec![1, 2, 3, 4, 5])
        })
        .unwrap();
        assert_eq!((newer.shop_in_shards, newer.holocrons), (true, [0, 400]));
        // A hub before blade cards crafted for less: the full cost with every finish.
        assert_eq!(newer.blade_card_craft, [100, 100, 100]);
        let cards = of_hub(&sjk_identity::EconomyTables {
            blade_card_craft: [("normal".to_owned(), 75), ("holo".to_owned(), 0)]
                .into_iter()
                .collect(),
            ..wire(vec![1, 2, 3, 4, 5])
        })
        .unwrap();
        assert_eq!(
            cards.blade_card_craft,
            [75, 100, 100],
            "a zero is no discount"
        );
        assert_eq!(of_hub(&wire(vec![1, 2, 3, 4])), None, "four tiers");
        assert_eq!(of_hub(&wire(vec![0, 2, 3, 4, 5])), None, "a zero");
    }

    /// Medal blades never take part; every shader holocrons drop does (every one but the
    /// medals', the Sun too: a dropped copy trades, the one Sol's card brings does not).
    #[test]
    fn only_what_holocrons_drop_and_no_medal_brings_is_traded() {
        for medal in crate::medals::Medal::ALL {
            if let Some(id) = medal.blade() {
                assert!(!in_trade(unlockables::find(id).unwrap()), "{id}");
            }
        }
        assert!(in_trade(unlockables::find("saber_sun").unwrap()));
        assert!(in_trade(unlockables::find("saber_tidal").unwrap()));
        assert!(
            unlockables::blade_skins()
                .filter(|skin| in_trade(skin))
                .all(|skin| skin.droppable && skin.tier != Rarity::Unique)
        );
        // Sol's thank-you gifts are never broken down, crafted or sold (25/10/2026).
        assert!(!in_trade(
            unlockables::find("saber_thousand_cranes").unwrap()
        ));
    }

    #[test]
    fn the_tables_price_each_tier() {
        assert_eq!(shatter_yield(Rarity::Common), 5);
        assert_eq!(shatter_yield(Rarity::Mythical), 150);
        assert_eq!(craft_cost(Rarity::Rare), 300);
        assert_eq!(shop_price(Rarity::Legendary), 600);
        // No table has Unique: nothing of it is priced.
        assert_eq!(shatter_yield(Rarity::Unique), 0);
        assert_eq!(craft_cost(Rarity::Unique), 0);
        assert_eq!(shop_price(Rarity::Unique), 0);
    }

    /// A blade card's copy crafts its shader at the shop's price (Normal) or half (Holo,
    /// Solar), and says by how much; without a copy it costs the full price.
    #[test]
    fn a_blade_card_crafts_its_shader_for_less() {
        let [normal, holo, solar] = &crate::cards::FINISHES;
        let tidal = unlockables::find("saber_tidal").unwrap();
        assert_eq!(craft_price(tidal, None), (40, None));
        let (cost, off) = craft_price(tidal, Some(normal));
        assert_eq!(
            (cost, off.map(|(finish, off)| (finish.id, off))),
            (30, Some(("normal", 25)))
        );
        assert_eq!(craft_price(tidal, Some(holo)).0, 20);
        assert_eq!(craft_price(tidal, Some(solar)).0, 20);
        let mythical = unlockables::blade_skins()
            .find(|skin| in_trade(skin) && skin.tier == Rarity::Mythical)
            .unwrap();
        assert_eq!(craft_price(mythical, Some(normal)).0, 1_500);
        assert_eq!(craft_price(mythical, Some(holo)).0, 1_000);
    }

    /// The best revealed copy of each blade card is kept, by its shader; a sealed copy, a
    /// card that is no blade card or one of an unknown shader gives nothing.
    #[test]
    fn the_best_copy_of_each_blade_card_is_kept() {
        let face = |id: &str, blade: Option<&str>| crate::cards::faces::Face {
            id: id.to_owned(),
            name: id.to_owned(),
            set: "sjk_blades".to_owned(),
            set_name: "SJK Blades".to_owned(),
            number: 1,
            kind: crate::cards::faces::Kind::Blade,
            rarity: &crate::cards::RARITIES[2],
            lore: String::new(),
            artist: String::new(),
            art: String::new(),
            grants: None,
            blade: blade.map(str::to_owned),
            scene: None,
            rank: None,
        };
        let faces = crate::cards::faces::Faces::of_list(vec![
            face("sun", Some("saber_sun")),
            face("tidal", Some("saber_tidal")),
            face("plain", None),
            face("later", Some("saber_later")),
        ]);
        let copy = |card: Option<&str>, finish: &str| sjk_identity::CardStack {
            card: card.map(str::to_owned),
            rarity: "rare".to_owned(),
            finish: finish.to_owned(),
            edition: "alpha".to_owned(),
            count: 1,
            first: 0,
            last: 0,
        };
        let cards = [
            copy(Some("sun"), "normal"),
            copy(Some("sun"), "holo"),
            copy(Some("tidal"), "normal"),
            copy(Some("plain"), "solar"),
            copy(Some("later"), "solar"),
            copy(None, "solar"),
        ];
        let mut found: Vec<_> = blade_cards(&cards, &faces)
            .into_iter()
            .map(|(id, finish)| (id, finish.id))
            .collect();
        found.sort_unstable();
        assert_eq!(found, [("saber_sun", "holo"), ("saber_tidal", "normal")]);
        let sun = unlockables::find("saber_sun").unwrap();
        assert_eq!(blade_card(sun, &cards, &faces).map(|f| f.id), Some("holo"));
        let frost = unlockables::find("saber_frost").unwrap();
        assert_eq!(blade_card(frost, &cards, &faces), None);
    }

    #[test]
    fn an_outcome_reads_as_a_line() {
        use sjk_identity::{Trade, TradeOutcome};
        let done = |trade, amount| TradeOutcome {
            serial: 1,
            trade,
            id: "saber_tidal".to_owned(),
            amount,
            copies: 1,
            message: String::new(),
        };
        assert_eq!(
            outcome_words(&done(Trade::Shatter, 5), "Kamino blade"),
            "Kamino blade broken down: +5 kyber shards."
        );
        assert_eq!(
            outcome_words(&done(Trade::ShatterLast, 5), "Kamino blade"),
            "Kamino blade broken down: +5 kyber shards. It has left your collection."
        );
        // Several at once: the totals, or the shader a refusal stopped at, named.
        let many = TradeOutcome {
            copies: 3,
            ..done(Trade::ShatterMany, 15)
        };
        assert_eq!(
            outcome_words(&many, ""),
            "3 copies broken down: +15 kyber shards."
        );
        let refused = TradeOutcome {
            message: "That is your last one: only duplicates can be shattered.".to_owned(),
            ..done(Trade::ShatterMany, 0)
        };
        let tidal = unlockables::find("saber_tidal").unwrap().name;
        assert_eq!(
            outcome_words(&refused, ""),
            format!("{tidal}: That is your last one: only duplicates can be shattered.")
        );
        let all = TradeOutcome {
            copies: 12,
            ..done(Trade::ShatterAll, 180)
        };
        assert_eq!(
            outcome_words(&all, ""),
            "12 duplicates broken down: +180 kyber shards."
        );
        assert_eq!(
            outcome_words(&done(Trade::Buy, 1_500), "Lava Lamp blade"),
            "Lava Lamp blade bought for 1 500 kyber shards. It is yours."
        );
        assert_eq!(
            outcome_words(&done(Trade::Holocron, 400), "Rare Holocron"),
            "Rare Holocron bought for 400 credits. It waits in your holocrons."
        );
        let refused = TradeOutcome {
            message: "Not enough kyber shards.".to_owned(),
            ..done(Trade::Craft, 0)
        };
        assert_eq!(outcome_words(&refused, "x"), "Not enough kyber shards.");
        assert_eq!(shards_text(12_345), "12 345");
    }
}
