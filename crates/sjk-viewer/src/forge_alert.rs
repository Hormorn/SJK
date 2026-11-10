//! The Forge's dot (10/11/2026; `docs/cards.md`, "Fusion, deal of the day, wishlist"): a
//! gold dot on the Collection's Forge tab and the Holocrons tab's Shop button while
//! something in the Forge waits for the player: an offer of today's shop is on their
//! wishlist, they can afford the deal of the day, or a fusion is possible. Read from the
//! identity's snapshot twice a second (`wishlist::follow`), kept here for the pages to
//! draw ([`shown`]).

use std::sync::atomic::{AtomicBool, Ordering};

static SHOWN: AtomicBool = AtomicBool::new(false);

/// Whether the dot shows.
pub(crate) fn shown() -> bool {
    SHOWN.load(Ordering::Relaxed)
}

/// Show the dot or not.
pub(crate) fn set(on: bool) {
    SHOWN.store(on, Ordering::Relaxed);
}

/// What the dot is for.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Reasons {
    /// An offer of today's on the wishlist, not held or bought.
    pub(crate) wished: bool,
    /// The deal of the day within reach, not held or bought.
    pub(crate) deal: bool,
    /// Some tier's duplicates make a fusion.
    pub(crate) fuse: bool,
}

impl Reasons {
    /// Whether any reason holds.
    pub(crate) fn any(self) -> bool {
        self.wished || self.deal || self.fuse
    }
}

/// Why the dot shows for `snapshot`, a fusion of a tier taking `need` duplicates
/// ([`crate::economy::fuse_need`] live). Nothing while the player may not trade.
pub(crate) fn reasons(
    snapshot: &sjk_identity::Snapshot,
    need: impl Fn(crate::unlockables::Rarity) -> Option<u32>,
) -> Reasons {
    if !crate::economy::may_act(snapshot) {
        return Reasons::default();
    }
    let Some(me) = snapshot.me.as_ref() else {
        return Reasons::default();
    };
    let purse = if crate::economy::shop_in_shards() {
        me.shards
    } else {
        me.credits
    };
    let mut reasons = Reasons::default();
    if let Some(shop) = snapshot
        .shop
        .as_ref()
        .filter(|shop| shop.message.is_empty())
    {
        let open = shop
            .day
            .offers
            .iter()
            .filter(|offer| !offer.owned && !offer.bought);
        for offer in open {
            reasons.wished |= offer.wished;
            reasons.deal |= offer.deal && purse >= offer.price;
        }
    }
    let held = |id: &str| {
        me.unlocks
            .iter()
            .find(|unlock| unlock.id == id && unlock.medal.is_none() && unlock.card.is_none())
            .map_or(0, |unlock| unlock.count)
    };
    let owns = |id: &str| me.unlocks.iter().any(|unlock| unlock.id == id);
    let locked = |id: &str| {
        snapshot
            .marks
            .as_ref()
            .is_some_and(|marks| marks.locked(id))
    };
    reasons.fuse = crate::forge_fuse::any_possible(need, held, owns, locked);
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unlockables::Rarity;

    /// A snapshot of a player who may trade, holding `unlocks` and `shards`, with `offers`.
    fn snapshot(
        unlocks: Vec<sjk_identity::Unlock>,
        shards: u64,
        offers: Vec<sjk_identity::Offer>,
    ) -> sjk_identity::Snapshot {
        let mut snapshot = crate::console::collection_panel::forge::sample_snapshot(unlocks);
        if let Some(me) = snapshot.me.as_mut() {
            me.shards = shards;
        }
        let mut shop = crate::console::collection_panel::forge::sample_day(3_600);
        shop.day.offers = offers;
        snapshot.shop = Some(shop);
        snapshot
    }

    fn offer(id: &str, price: u64, deal: bool, wished: bool) -> sjk_identity::Offer {
        sjk_identity::Offer {
            deal,
            wished,
            ..crate::console::collection_panel::forge::sample_offer(id, "common", price)
        }
    }

    fn copy(id: &str, count: u32) -> sjk_identity::Unlock {
        sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: id.to_owned(),
            granted: 1,
            note: String::new(),
            count,
            medal: None,
            card: None,
        }
    }

    #[test]
    fn the_dot_shows_for_a_wished_offer_an_affordable_deal_or_a_fusion() {
        let none = |_: Rarity| None;
        let quiet = snapshot(
            Vec::new(),
            100,
            vec![offer("saber_aurora", 30, false, false)],
        );
        assert!(!reasons(&quiet, none).any());
        let wished = snapshot(Vec::new(), 0, vec![offer("saber_aurora", 30, false, true)]);
        assert_eq!(
            reasons(&wished, none),
            Reasons {
                wished: true,
                ..Reasons::default()
            }
        );
        // The deal only when the purse reaches it.
        let deal = |shards| {
            snapshot(
                Vec::new(),
                shards,
                vec![offer("saber_aurora", 300, true, false)],
            )
        };
        assert!(reasons(&deal(300), none).deal);
        assert!(!reasons(&deal(299), none).deal);
        // Held or bought already: nothing to say.
        let mut bought = deal(900);
        bought.shop.as_mut().unwrap().day.offers[0].bought = true;
        assert!(!reasons(&bought, none).any());
        // Three spare Commons with a hub that fuses three.
        let commons: Vec<_> = crate::unlockables::blade_skins_by_tier()
            .iter()
            .filter(|skin| skin.tier == Rarity::Common && crate::economy::in_trade(skin))
            .take(2)
            .collect();
        let held = vec![copy(commons[0].id, 3), copy(commons[1].id, 2)];
        let three = |tier: Rarity| (tier == Rarity::Common).then_some(3);
        let fusing = snapshot(held.clone(), 0, Vec::new());
        assert!(reasons(&fusing, three).fuse);
        assert!(!reasons(&fusing, none).fuse, "a hub that does not fuse");
        let four = |tier: Rarity| (tier == Rarity::Common).then_some(4);
        assert!(!reasons(&fusing, four).fuse, "three spare, four needed");
        // Nothing while the player may not trade.
        let mut closed = fusing;
        if let Some(progress) = closed.holocrons.as_mut() {
            progress.can_open = false;
        }
        assert!(!reasons(&closed, three).any());
    }
}
