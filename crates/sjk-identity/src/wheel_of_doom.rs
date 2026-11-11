//! The WHEEL OF DOOM (the hub's `PROTOCOL.md`, "WHEEL OF DOOM", since 07/11/2026): since
//! 11/11/2026 wheels SJK's creator places on maps, which any SJK player spins for a DOOM
//! coin (a mystery box that blows up brings one). The hub spends the coin, keeps the
//! cooldown and rolls the outcome; this crate reads the wheels, the coins and the odds and
//! asks for a spin. The viewer draws the wheels and plays what they land on
//! (`sjk-viewer`'s `wheel_of_doom.rs`, `docs/wheel-of-doom.md`).

use crate::hub::HubError;
use crate::mystery_box::BoxHolocron;
use crate::wire::Profile;
use serde::Deserialize;

/// The toy id of the feed event the hub makes when a player spins: `land_<wheel>_<outcome>`
/// from the spinner's slot.
pub const TOY: &str = "wheel_of_doom";

/// One segment of the wheel, as the hub rolls it.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Outcome {
    /// Its id (`mythical`, `nothing`, `mirror`, ...).
    pub id: String,
    /// How likely it is, per mille.
    #[serde(default)]
    pub per_mille: u32,
}

/// `GET /v1/wheel-of-doom`: the wheels placed, the player's DOOM coins, the cooldown, a
/// spin's length, the segments in order and whether this player may spin now.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Wheel {
    /// Every placed wheel (since 11/11/2026), shaped as a mystery box.
    #[serde(default)]
    pub wheels: Vec<crate::mystery_box::MysteryBox>,
    /// What a spin costs, in credits (0 since 11/11/2026).
    #[serde(default)]
    pub price: u64,
    /// What a spin costs, in DOOM coins.
    #[serde(default)]
    pub coins_price: u64,
    /// The DOOM coins the player holds.
    #[serde(default)]
    pub coins: u64,
    /// How long a spin lasts, in seconds (its music).
    #[serde(default)]
    pub spin: u64,
    /// Shortest time between two spins by one person, in seconds.
    #[serde(default)]
    pub cooldown: u64,
    /// The segments, in the wheel's order (segment 0 first).
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
    /// Whether the hub lets this player spin now (since 09/11/2026; false while the
    /// operator has the wheel off). A hub before it sends none: open.
    #[serde(default = "open")]
    pub open: bool,
}

fn open() -> bool {
    true
}

impl Wheel {
    /// The segment `outcome` stands on, by the hub's order.
    pub fn segment_of(&self, outcome: &str) -> Option<usize> {
        self.outcomes.iter().position(|known| known.id == outcome)
    }
}

impl Wheel {
    /// Whether any wheel stands on a map called `map` (any checksum), so the viewer knows
    /// to read the loaded map's checksum.
    pub fn on_map(&self, map: &str) -> bool {
        self.wheels
            .iter()
            .any(|placed| placed.map.eq_ignore_ascii_case(map))
    }
}

/// One spin, as the spinner is answered.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Spin {
    /// The placed wheel spun (since 11/11/2026).
    #[serde(default)]
    pub wheel: i64,
    /// The DOOM coins the spinner holds after (since 11/11/2026).
    #[serde(default)]
    pub coins: Option<u64>,
    /// What the wheel lands on (`mirror`, or a newer hub's).
    pub outcome: String,
    /// The segment it stops on, 0 to 19.
    #[serde(default)]
    pub segment: u32,
    /// The holocron a holocron segment gave.
    #[serde(default)]
    pub holocron: Option<BoxHolocron>,
}

/// `POST /v1/wheel-of-doom/spin`'s answer.
#[derive(Clone, Debug, Deserialize)]
pub struct SpinAnswer {
    /// What the wheel landed on.
    pub spin: Spin,
    /// The spinner's profile after (the credits paid, a holocron won).
    pub profile: Profile,
}

/// What a feed event of [`TOY`] says: placed wheel `wheel` landed on `outcome`
/// (`land_<wheel>_<outcome>`; the toy's `spin_<outcome>` before 11/11/2026 is not read).
pub fn parse_event(event: &str) -> Option<(i64, &str)> {
    let (wheel, outcome) = event.strip_prefix("land_")?.split_once('_')?;
    Some((wheel.parse().ok()?, outcome)).filter(|(_, outcome)| !outcome.is_empty())
}

/// What a player reads while the hub has the wheel off (`wheel_off`, or `open` false).
pub const OUT_OF_ORDER: &str = "The WHEEL OF DOOM is out of order for now.";
/// What a player without a DOOM coin reads (`no_doom_coin`).
pub const NO_COIN: &str =
    "The WHEEL OF DOOM spins for a DOOM coin: the teddy bear in a mystery box brings one.";

/// A refused spin: why, in the player's words, and for the cooldown, the whole seconds
/// left before the next spin (the hub's `left`, read from its message, which says it as
/// "spins again in N s").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refused {
    /// Why, in the player's words.
    pub words: String,
    /// The seconds left of the cooldown (`wheel_cooldown`).
    pub left: Option<u64>,
}

/// A refused spin in the player's words, with the cooldown's seconds left when it says.
pub fn refusal(error: &HubError) -> Refused {
    let left = match error {
        HubError::Rejected { code, message, .. }
            if code == "wheel_cooldown" || code == "wheel_busy" =>
        {
            message
                .split(|c: char| !c.is_ascii_digit())
                .rfind(|digits| !digits.is_empty())
                .and_then(|digits| digits.parse().ok())
        }
        _ => None,
    };
    let busy = matches!(error, HubError::Rejected { code, .. } if code == "wheel_busy");
    let words = match (error, left) {
        (_, Some(left)) if busy => {
            return Refused {
                words: format!("Someone is spinning this WHEEL OF DOOM: {left} s left"),
                left: None,
            };
        }
        (_, Some(left)) => {
            return Refused {
                words: format!("Your next spin in {left} s"),
                left: Some(left),
            };
        }
        (HubError::Rejected { code, message, .. }, None) => match code.as_str() {
            "not_enough_credits" => "Not enough credits to spin the WHEEL OF DOOM.",
            "no_doom_coin" => NO_COIN,
            "wheel_busy" => "Someone is spinning this WHEEL OF DOOM.",
            "wheel_moved" => "Update SJK: the WHEEL OF DOOM stands on maps now.",
            "no_such_box" => "That WHEEL OF DOOM is not here any more.",
            "wheel_off" => OUT_OF_ORDER,
            "wheel_cooldown" => "The WHEEL OF DOOM needs a rest: spin again later.",
            "not_on_server" => "The hub does not see you on this server yet.",
            "client_update_required" => "Update SJK to spin the WHEEL OF DOOM.",
            _ if message.is_empty() => {
                return Refused {
                    words: error.to_string(),
                    left: None,
                };
            }
            _ => {
                return Refused {
                    words: message.clone(),
                    left: None,
                };
            }
        },
        (HubError::Protocol(why), None)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub has no WHEEL OF DOOM yet."
        }
        (other, None) => {
            return Refused {
                words: other.to_string(),
                left: None,
            };
        }
    };
    Refused {
        words: words.to_owned(),
        left: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_reads_as_the_hub_sends_it() {
        let wheel: Wheel = serde_json::from_str(
            r#"{"price":100,"cooldown":300,"outcomes":[{"id":"mythical","per_mille":10},
                {"id":"nothing","per_mille":90},{"id":"mirror","per_mille":60}]}"#,
        )
        .unwrap();
        assert_eq!((wheel.price, wheel.cooldown), (100, 300));
        assert_eq!(wheel.segment_of("mirror"), Some(2));
        assert_eq!(wheel.segment_of("bunny"), None);
        assert!(wheel.open, "a hub without `open` lets the wheel spin");
        let off: Wheel =
            serde_json::from_str(r#"{"price":100,"cooldown":300,"open":false}"#).unwrap();
        assert!(!off.open);
        // Since 11/11/2026: placed wheels, DOOM coins and a spin's length.
        let placed: Wheel = serde_json::from_str(
            r#"{"wheels":[{"id":2,"map":"mp/ffa3","checksum":-1234,"server":"",
                "origin":[1,2,3],"yaw":90,"placed":5}],"price":0,"coins_price":1,"coins":3,
                "cooldown":40,"spin":35,"open":true,"outcomes":[]}"#,
        )
        .unwrap();
        assert_eq!((placed.coins, placed.coins_price, placed.spin), (3, 1, 35));
        assert!(placed.on_map("MP/FFA3") && !placed.on_map("mp/ffa2"));
        assert_eq!(placed.wheels[0].id, 2);
    }

    #[test]
    fn a_spin_reads_its_segment_and_holocron() {
        let plain: Spin =
            serde_json::from_str(r#"{"id":7,"outcome":"walk","segment":12,"holocron":null}"#)
                .unwrap();
        assert_eq!((plain.outcome.as_str(), plain.segment), ("walk", 12));
        assert_eq!(plain.holocron, None);
        let jackpot: Spin = serde_json::from_str(
            r#"{"id":"8","outcome":"mythical","segment":0,"holocron":{"id":91,"tier":"mythical"}}"#,
        )
        .unwrap();
        assert_eq!(
            jackpot.holocron.map(|h| h.tier),
            Some("mythical".to_owned())
        );
    }

    #[test]
    fn feed_events_name_the_wheel_and_the_outcome() {
        assert_eq!(parse_event("land_2_mirror"), Some((2, "mirror")));
        assert_eq!(
            parse_event("land_2_reverse_keys"),
            Some((2, "reverse_keys"))
        );
        assert_eq!(parse_event("land_2_"), None);
        assert_eq!(parse_event("land_x_mirror"), None);
        // The toy's events of before 11/11/2026 stand no wheel.
        assert_eq!(parse_event("spin_mirror"), None);
        assert_eq!(parse_event("open_1_empty"), None);
    }

    #[test]
    fn refusals_read_in_the_players_words() {
        let rejected = |code: &str| HubError::Rejected {
            status: 409,
            code: code.to_owned(),
            message: "raw".to_owned(),
        };
        assert!(refusal(&rejected("wheel_cooldown")).words.contains("rest"));
        assert!(
            refusal(&rejected("not_enough_credits"))
                .words
                .contains("credits")
        );
        assert_eq!(refusal(&rejected("opening_staff_only")).words, "raw");
        assert_eq!(refusal(&rejected("wheel_off")).words, OUT_OF_ORDER);
        let cooling = HubError::Rejected {
            status: 429,
            code: "wheel_cooldown".to_owned(),
            message: "the WHEEL OF DOOM spins again in 245 s".to_owned(),
        };
        assert_eq!(
            refusal(&cooling),
            Refused {
                words: "Your next spin in 245 s".to_owned(),
                left: Some(245)
            }
        );
        assert_eq!(refusal(&rejected("no_doom_coin")).words, NO_COIN);
        let busy = HubError::Rejected {
            status: 409,
            code: "wheel_busy".to_owned(),
            message: "someone is spinning this WHEEL OF DOOM: 12 s left".to_owned(),
        };
        assert_eq!(
            refusal(&busy),
            Refused {
                words: "Someone is spinning this WHEEL OF DOOM: 12 s left".to_owned(),
                left: None
            }
        );
    }
}
