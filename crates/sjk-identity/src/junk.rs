//! Selling fished junk to the hub for credits (`PROTOCOL.md` "Fishing junk", since 03/11/2026,
//! Sol's yes on SJK #278): the Fishing toy's junk (an old boot, a rusted blaster...) sells
//! for a few credits a piece, capped by the hub at a small sum a day per person and per
//! address, behind the operator's switch. The client counts its own catches, so the cap
//! is what keeps a faked log from minting credits.
//!
//! `GET /v1/fishing/junk` says whether this key may sell, the prices, the cap and what is
//! still payable; `POST /v1/fishing/junk/sell` sells pieces, the most valuable first while
//! they fit in what is left, and answers what it took.

use crate::hub::HubError;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Most kinds of junk one sale names (the hub refuses more).
pub const KINDS_MAX: usize = 6;
/// Most pieces of one kind one sale names.
pub const COUNT_MAX: u32 = 1_000;

/// `GET /v1/fishing/junk`'s answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct JunkStatus {
    /// Whether the hub lets this key sell now (the operator's switch).
    #[serde(default)]
    pub open: bool,
    /// Credits a piece, by junk id.
    #[serde(default)]
    pub prices: BTreeMap<String, u32>,
    /// The most credits junk pays in 24 hours.
    #[serde(default)]
    pub daily: u32,
    /// What it still pays this player in the rolling 24 hours.
    #[serde(default)]
    pub left: u32,
    /// The player's credits.
    #[serde(default)]
    pub balance: i64,
}

/// `POST /v1/fishing/junk/sell`'s answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct JunkSold {
    /// The pieces the hub took, by junk id; the rest stay with the player.
    #[serde(default)]
    pub sold: BTreeMap<String, u32>,
    /// What it paid.
    pub credits: u32,
    /// What it still pays in the rolling 24 hours.
    pub left: u32,
    /// The player's credits after.
    pub balance: i64,
}

impl JunkStatus {
    /// What selling `held` (pieces by junk id) would pay now, as the hub picks: the most
    /// valuable first while each fits in [`Self::left`].
    pub fn payable(&self, held: &BTreeMap<String, u32>) -> u32 {
        let mut pieces: Vec<(u32, u32)> = held
            .iter()
            .filter_map(|(id, count)| self.prices.get(id).map(|price| (*price, *count)))
            .filter(|(price, count)| *price > 0 && *count > 0)
            .collect();
        pieces.sort_by_key(|(price, _)| std::cmp::Reverse(*price));
        let mut left = self.left;
        let mut paid = 0;
        for (price, count) in pieces {
            let taken = count.min(left / price);
            paid += taken * price;
            left -= taken * price;
        }
        paid
    }
}

/// The body of `POST /v1/fishing/junk/sell` for `held`: the kinds with pieces, each at
/// most [`COUNT_MAX`], the [`KINDS_MAX`] first; `None` when nothing is held.
pub fn sale_body(held: &BTreeMap<String, u32>) -> Option<serde_json::Value> {
    let junk: serde_json::Map<String, serde_json::Value> = held
        .iter()
        .filter(|(_, count)| **count > 0)
        .take(KINDS_MAX)
        .map(|(id, count)| (id.clone(), serde_json::json!((*count).min(COUNT_MAX))))
        .collect();
    (!junk.is_empty()).then(|| serde_json::json!({ "junk": junk }))
}

/// Why a junk request failed, in the player's words.
pub fn refusal(error: &HubError) -> String {
    let words = match error {
        HubError::Rejected { code, message, .. } => match code.as_str() {
            "junk_cap" => "The hub buys no more junk from you today: come back tomorrow.",
            "junk_selling_off" => "The hub does not buy junk yet.",
            "bad_junk" => "The hub did not take that junk.",
            "client_update_required" => "Update SJK to sell junk.",
            _ => return message.clone(),
        },
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub does not buy junk yet."
        }
        other => return other.to_string(),
    };
    words.to_owned()
}

/// Whether `error` means the hub has no junk selling (an older hub): the client then
/// shows no Sell button.
pub fn unsupported(error: &HubError) -> bool {
    matches!(error, HubError::Protocol(why)
        if why.starts_with("status 404") || why.starts_with("status 405"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs.iter().map(|(id, n)| ((*id).to_owned(), *n)).collect()
    }

    #[test]
    fn answers_read_as_the_protocol_writes_them() {
        let status: JunkStatus = serde_json::from_str(
            r#"{"open":true,"prices":{"boot":1,"weed":1,"blaster":2,"holopad":3,"droid_head":4,"hilt":5},"daily":15,"left":7,"balance":158}"#,
        )
        .unwrap();
        assert!(status.open);
        assert_eq!(status.prices["hilt"], 5);
        assert_eq!((status.daily, status.left, status.balance), (15, 7, 158));
        let closed: JunkStatus = serde_json::from_str(r#"{"open":false}"#).unwrap();
        assert!(!closed.open && closed.prices.is_empty());
        let sold: JunkSold = serde_json::from_str(
            r#"{"sold":{"hilt":1,"boot":3},"credits":8,"left":7,"balance":158}"#,
        )
        .unwrap();
        assert_eq!(sold.sold, held(&[("boot", 3), ("hilt", 1)]));
        assert_eq!((sold.credits, sold.left, sold.balance), (8, 7, 158));
    }

    #[test]
    fn payable_takes_the_most_valuable_first_within_what_is_left() {
        let status = JunkStatus {
            open: true,
            prices: held(&[("boot", 1), ("blaster", 2), ("hilt", 5)]),
            daily: 15,
            left: 12,
            balance: 0,
        };
        // Two hilts (10), one blaster (2), then nothing left for the boots.
        assert_eq!(
            status.payable(&held(&[("hilt", 3), ("blaster", 4), ("boot", 9)])),
            12
        );
        assert_eq!(status.payable(&held(&[("boot", 3)])), 3);
        assert_eq!(status.payable(&held(&[("unknown", 3)])), 0);
        let spent = JunkStatus { left: 0, ..status };
        assert_eq!(spent.payable(&held(&[("hilt", 1)])), 0);
    }

    #[test]
    fn a_sale_names_what_is_held() {
        assert_eq!(sale_body(&held(&[("boot", 0)])), None);
        assert_eq!(
            sale_body(&held(&[("boot", 2), ("weed", 0), ("hilt", 5_000)])),
            Some(serde_json::json!({"junk": {"boot": 2, "hilt": COUNT_MAX}}))
        );
    }

    #[test]
    fn refusals_read_in_the_players_words() {
        let rejected = |code: &str| HubError::Rejected {
            status: 409,
            code: code.to_owned(),
            message: "hub words".to_owned(),
        };
        assert!(refusal(&rejected("junk_cap")).contains("tomorrow"));
        assert_eq!(refusal(&rejected("other")), "hub words");
        let old = HubError::Protocol("status 404 Not Found".to_owned());
        assert!(unsupported(&old));
        assert!(refusal(&old).contains("does not buy"));
        assert!(!unsupported(&rejected("junk_cap")));
    }
}
