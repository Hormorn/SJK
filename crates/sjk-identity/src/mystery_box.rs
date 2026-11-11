//! Mystery boxes (the hub's `PROTOCOL.md`, "Mystery boxes", since 27/10/2026): boxes
//! SJK's creator places on maps, which players open for credits. The hub keeps where
//! they are, takes the price and rolls what is inside; this crate reads the list, asks
//! to open one and carries the creator's placing. The viewer draws them
//! (`sjk-viewer`'s `mystery_box.rs`).

use crate::hub::HubError;
use crate::wire::Profile;
use serde::{Deserialize, Serialize};

/// The toy id of the feed event the hub makes when a box is opened.
pub const TOY: &str = "mystery_box";
/// The outcome that blows the opener up and hands what they paid to a random hunter on
/// their server (since 28/10/2026).
pub const EXPLODE: &str = "explode";
/// The outcome that gives a holocron (since 28/10/2026).
pub const HOLOCRON: &str = "holocron";
/// The outcome that gives a pile of kyber shards (since 29/10/2026).
pub const SHARD_PILE: &str = "shards";
/// The outcome that gave a free opening, spent on the next one (29/10/2026 to
/// 11/11/2026: no box holds a box since).
pub const AGAIN: &str = "again";
/// The outcome that gives a kyber crystal (since 11/11/2026, very rare).
pub const KYBER: &str = "kyber";
/// The outcome that hands the opener the Ray Gun for [`Opened::raygun`] seconds (since
/// 11/11/2026).
pub const RAYGUN: &str = "raygun";
/// The small pile of gold coins (since 11/11/2026: half the price).
pub const COINS: &str = "coins";
/// The toy id of the Ray Gun's bolts (the toy relay) and of the hub's hit event.
pub const RAYGUN_TOY: &str = "raygun";
/// The outcome that gives the boxes' own saber shader, [`BLADE_ID`], or shards for a
/// player who holds it (since 29/10/2026).
pub const BLADE: &str = "blade";
/// The boxes' own saber shader, found nowhere else.
pub const BLADE_ID: &str = "saber_mystery";

/// One placed box.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MysteryBox {
    /// The hub's id, never given again.
    pub id: i64,
    /// The map's name as the game gives it (`mp/ffa3`).
    pub map: String,
    /// The map's BSP checksum: a box stands in one exact file.
    pub checksum: i32,
    /// `ip:port` for one server only, empty for every server.
    #[serde(default)]
    pub server: String,
    /// Where it stands, in game units.
    pub origin: [f32; 3],
    /// Facing, in degrees.
    pub yaw: f32,
}

impl MysteryBox {
    /// Whether the box stands on map `map` (any case) of checksum `checksum`, on `server`.
    pub fn stands_on(&self, map: &str, checksum: i32, server: Option<&str>) -> bool {
        self.checksum == checksum
            && self.map.eq_ignore_ascii_case(map)
            && (self.server.is_empty() || Some(self.server.as_str()) == server)
    }
}

/// One thing a box may hold, as the hub rolls it.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Outcome {
    /// Its id (`empty`, `jackpot`, ...).
    pub id: String,
    /// How likely it is, per mille.
    pub per_mille: u32,
    /// The credits it pays.
    pub credits: u64,
}

/// `GET /v1/mystery-boxes`: every placed box, the price and the odds.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Boxes {
    /// Every placed box.
    pub boxes: Vec<MysteryBox>,
    /// What opening one costs, in credits.
    pub price: u64,
    /// Shortest time between two openings by one person, in seconds.
    #[serde(default)]
    pub cooldown: u64,
    /// What a box may hold.
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
}

impl Boxes {
    /// Whether any box stands on a map called `map` (any checksum), so the viewer knows
    /// to read the loaded map's checksum.
    pub fn on_map(&self, map: &str) -> bool {
        self.boxes
            .iter()
            .any(|placed| placed.map.eq_ignore_ascii_case(map))
    }
}

/// One opening, as the opener is answered.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Opened {
    /// The box opened.
    #[serde(rename = "box")]
    pub box_id: i64,
    /// What was inside (`empty`, `refund`, `double`, `jackpot`, or a newer hub's).
    pub outcome: String,
    /// What it cost.
    pub price: u64,
    /// The credits it paid.
    pub won: u64,
    /// The opener's credits after.
    pub balance: u64,
    /// A box that blew up: the hunter who got what the opener paid, when one was there.
    #[serde(default)]
    pub gift: Option<Gift>,
    /// The holocron the box gave.
    #[serde(default)]
    pub holocron: Option<BoxHolocron>,
    /// The kyber shards the box gave (since 29/10/2026).
    #[serde(default)]
    pub shards: Option<u64>,
    /// The saber shader the box gave, its unlock id (since 29/10/2026).
    #[serde(default)]
    pub blade: Option<String>,
    /// The free openings left after this one (since 29/10/2026; an older hub leaves it
    /// out).
    #[serde(default)]
    pub free: Option<u64>,
    /// The kyber crystal the box gave: its id and tier (`kyber_rare`; since 11/11/2026).
    #[serde(default)]
    pub kyber: Option<BoxHolocron>,
    /// The Ray Gun the box gave: the seconds it lasts (since 11/11/2026).
    #[serde(default)]
    pub raygun: Option<u64>,
    /// The DOOM coins the opener holds after (since 11/11/2026).
    #[serde(default)]
    pub doom_coins: Option<u64>,
}

/// Where a blown up box's price went.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Gift {
    /// The hunter's slot.
    pub slot: u8,
    /// The hunter's worn name.
    pub name: String,
    /// What they got (0 for a free opening).
    pub credits: u64,
}

/// The holocron a box gave: its id and tier.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BoxHolocron {
    /// The holocron's id among the player's.
    pub id: i64,
    /// Its tier (`rare`, ...).
    pub tier: String,
}

/// `POST /v1/mystery-box/open`'s answer.
#[derive(Clone, Debug, Deserialize)]
pub struct OpenAnswer {
    /// What came out.
    pub opened: Opened,
    /// The opener's profile after.
    pub profile: Profile,
}

/// What a feed event of [`TOY`] says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoxEvent<'a> {
    /// The box opened.
    pub id: i64,
    /// What came out (`holocron`, `kyber`, `double`...).
    pub outcome: &'a str,
    /// For a holocron or a kyber crystal, its tier as the Collection names it (`rare`,
    /// `kyber_rare`), since 11/11/2026; none from an older hub.
    pub tier: Option<String>,
}

/// Read a feed event of [`TOY`]: box `box` gave `outcome` (`open_<box>_<outcome>`), and
/// for a holocron or a kyber crystal its tier after (`open_<box>_holocron_rare`,
/// `open_<box>_kyber_rare`; since 11/11/2026).
pub fn parse_event(event: &str) -> Option<BoxEvent<'_>> {
    let rest = event.strip_prefix("open_")?;
    let (id, outcome) = rest.split_once('_')?;
    let id = id.parse().ok()?;
    for (kind, prefix) in [(HOLOCRON, ""), (KYBER, "kyber_")] {
        if let Some(tier) = outcome
            .strip_prefix(kind)
            .and_then(|rest| rest.strip_prefix('_'))
            .filter(|tier| !tier.is_empty())
        {
            return Some(BoxEvent {
                id,
                outcome: kind,
                tier: Some(format!("{prefix}{tier}")),
            });
        }
    }
    (!outcome.is_empty()).then_some(BoxEvent {
        id,
        outcome,
        tier: None,
    })
}

/// What a feed event of [`RAYGUN_TOY`] says: the Ray Gun of the event's player struck the
/// player on `victim`, paying `credits` (`hit_<victim>_<credits>`).
pub fn parse_raygun_hit(event: &str) -> Option<(u8, u64)> {
    let (victim, credits) = event.strip_prefix("hit_")?.split_once('_')?;
    Some((victim.parse().ok()?, credits.parse().ok()?))
}

/// The look toy of the box hunt (`POST /v1/box-hunt`): a player wearing it takes part.
pub const HUNT_TOY: &str = "box_hunt";

/// The box hunt's mark as the hub tells hunters.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct HuntMark {
    /// The mark's slot.
    pub slot: u8,
    /// The mark's worn name.
    pub name: String,
    /// When the hub picks another, Unix seconds.
    pub until: i64,
    /// Seconds from the read to `until`, by the hub's clock (a hub before 31/10/2026
    /// does not say).
    #[serde(default)]
    pub left: Option<u64>,
    /// The hunters after the mark (a hub before 01/11/2026 does not say).
    #[serde(default)]
    pub hunters: Option<u32>,
    /// How many of them are still in the chase.
    #[serde(default)]
    pub alive: Option<u32>,
}

/// Why the player is out of the box hunt's chase (`POST /v1/box-hunt/out`, since
/// 01/11/2026): each hunter has one life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HuntOut {
    /// Killed by the mark, on slot `by`.
    Dead {
        /// The mark's slot.
        by: u8,
    },
    /// Teleported (not by a map teleporter or a spawn) or in noclip.
    Cheat,
    /// The mark typed in the chat too long (since 02/11/2026): it blows up.
    Chat,
}

/// `POST /v1/box-hunt`: the hunt on a server as the player reads it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct HuntState {
    /// The player wears the hunt's toy there: only then is the mark told.
    #[serde(default)]
    pub hunting: bool,
    /// The mark, told while the player hunts and a mark is picked.
    #[serde(default)]
    pub mark: Option<HuntMark>,
    /// Free box openings won and not spent.
    #[serde(default)]
    pub free: u64,
    /// Why the player is out of the current mark's chase (`dead`, `cheat`), none while
    /// in it.
    #[serde(default)]
    pub out: Option<String>,
    /// For the mark: the slots of the hunters still in the chase (since 02/11/2026).
    #[serde(default)]
    pub chasers: Vec<u8>,
    /// How long a mark lasts, in seconds.
    #[serde(default)]
    pub every: u64,
}

/// What a feed event of the hunt says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HuntEvent {
    /// The player on this slot is the new mark (`mark_<slot>`).
    Mark(u8),
    /// The player on `slot` killed the mark, and whether it won them a free opening
    /// (`hunted_<slot>_<won>`).
    Hunted {
        /// The hunter's slot.
        slot: u8,
        /// Whether it won them a free opening.
        won: bool,
    },
    /// The hunter on `slot` is out of the chase: killed by the mark, or `cheat`ing
    /// (`out_<slot>_dead`, `out_<slot>_cheat`).
    Out {
        /// The hunter's slot.
        slot: u8,
        /// Caught teleporting or in noclip, not killed.
        cheat: bool,
    },
    /// The mark on `slot` blew up, caught cheating or typing in the chat too long, and
    /// the hunter on `to` (`None`: nobody else hunted there) was picked for a free mystery
    /// box roll, which they `won` unless they held or won the most already
    /// (`blown_<slot>_<cheat|chat>_<to>_<won>`, `to` 255 for nobody; since 02/11/2026).
    Blown {
        /// The mark's slot.
        slot: u8,
        /// Typing in the chat, not teleporting or in noclip.
        chat: bool,
        /// The slot of the hunter picked.
        to: Option<u8>,
        /// Whether it won them a free opening.
        won: bool,
    },
    /// The mark on `slot` outlasted every hunter, `hunters` of them, and won `credits`
    /// (`survived_<slot>_<credits>_<hunters>`; 0 credits past the day's wins).
    Survived {
        /// The mark's slot.
        slot: u8,
        /// The credits won.
        credits: u64,
        /// The hunters it counted.
        hunters: u32,
    },
    /// The box the player on `opener` opened blew up, and `credits` went to the hunter
    /// on `to` (`None`: nobody else hunted there, they were lost)
    /// (`laugh_<opener>_<to>_<credits>`, `to` 255 for nobody).
    Laugh {
        /// The opener's slot.
        opener: u8,
        /// The slot of the hunter who got the credits.
        to: Option<u8>,
        /// What they got.
        credits: u64,
    },
}

/// Read a feed event of [`HUNT_TOY`].
pub fn parse_hunt_event(event: &str) -> Option<HuntEvent> {
    if let Some(slot) = event.strip_prefix("mark_") {
        return Some(HuntEvent::Mark(slot.parse().ok()?));
    }
    if let Some(rest) = event.strip_prefix("blown_") {
        let mut parts = rest.split('_');
        let slot = parts.next()?.parse().ok()?;
        let chat = match parts.next()? {
            "cheat" => false,
            "chat" => true,
            _ => return None,
        };
        let to: u8 = parts.next()?.parse().ok()?;
        let won = match parts.next()? {
            "1" => true,
            "0" => false,
            _ => return None,
        };
        return parts.next().is_none().then_some(HuntEvent::Blown {
            slot,
            chat,
            to: (to != 255).then_some(to),
            won,
        });
    }
    if let Some(rest) = event.strip_prefix("out_") {
        let (slot, why) = rest.split_once('_')?;
        return Some(HuntEvent::Out {
            slot: slot.parse().ok()?,
            cheat: match why {
                "dead" => false,
                "cheat" => true,
                _ => return None,
            },
        });
    }
    if let Some(rest) = event.strip_prefix("survived_") {
        let mut parts = rest.split('_');
        let slot = parts.next()?.parse().ok()?;
        let credits = parts.next()?.parse().ok()?;
        let hunters = parts.next()?.parse().ok()?;
        return parts.next().is_none().then_some(HuntEvent::Survived {
            slot,
            credits,
            hunters,
        });
    }
    if let Some(rest) = event.strip_prefix("laugh_") {
        let mut parts = rest.split('_');
        let opener = parts.next()?.parse().ok()?;
        let to: u8 = parts.next()?.parse().ok()?;
        let credits = parts.next()?.parse().ok()?;
        return parts.next().is_none().then_some(HuntEvent::Laugh {
            opener,
            to: (to != 255).then_some(to),
            credits,
        });
    }
    let (slot, won) = event.strip_prefix("hunted_")?.split_once('_')?;
    Some(HuntEvent::Hunted {
        slot: slot.parse().ok()?,
        won: match won {
            "1" => true,
            "0" => false,
            _ => return None,
        },
    })
}

/// Why an opening, a placing or a removal failed, in the player's words.
pub fn refusal(error: &HubError) -> String {
    let words = match error {
        HubError::Rejected { code, message, .. } => match code.as_str() {
            "not_enough_credits" => "Not enough credits.",
            "box_cooldown" => "You opened a box a moment ago: let someone else roll.",
            "no_such_box" => "That box is not here any more.",
            "not_on_server" => "The hub does not see you on this server yet.",
            "opening_staff_only" => "Mystery boxes open with holocrons: soon.",
            "too_many_boxes" => "As many boxes are placed as the hub keeps: remove one first.",
            "creator_only" => "Only SJK's creator places boxes.",
            _ => return message.clone(),
        },
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub has no mystery boxes yet."
        }
        other => return other.to_string(),
    };
    words.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunt_events_read_back() {
        assert_eq!(parse_hunt_event("mark_7"), Some(HuntEvent::Mark(7)));
        assert_eq!(
            parse_hunt_event("hunted_3_1"),
            Some(HuntEvent::Hunted { slot: 3, won: true })
        );
        assert_eq!(
            parse_hunt_event("hunted_3_0"),
            Some(HuntEvent::Hunted {
                slot: 3,
                won: false
            })
        );
        assert_eq!(
            parse_hunt_event("laugh_2_4_100"),
            Some(HuntEvent::Laugh {
                opener: 2,
                to: Some(4),
                credits: 100
            })
        );
        assert_eq!(
            parse_hunt_event("laugh_2_255_0"),
            Some(HuntEvent::Laugh {
                opener: 2,
                to: None,
                credits: 0
            })
        );
        assert_eq!(
            parse_hunt_event("out_5_dead"),
            Some(HuntEvent::Out {
                slot: 5,
                cheat: false
            })
        );
        assert_eq!(
            parse_hunt_event("out_5_cheat"),
            Some(HuntEvent::Out {
                slot: 5,
                cheat: true
            })
        );
        assert_eq!(
            parse_hunt_event("blown_1_cheat_4_1"),
            Some(HuntEvent::Blown {
                slot: 1,
                chat: false,
                to: Some(4),
                won: true
            })
        );
        assert_eq!(
            parse_hunt_event("blown_1_chat_255_0"),
            Some(HuntEvent::Blown {
                slot: 1,
                chat: true,
                to: None,
                won: false
            })
        );
        assert_eq!(
            parse_hunt_event("survived_1_300_3"),
            Some(HuntEvent::Survived {
                slot: 1,
                credits: 300,
                hunters: 3
            })
        );
        for bad in [
            "out_5_away",
            "survived_1_300",
            "survived_1_300_3_4",
            "cheat_1",
            "blown_1_away_4_1",
            "blown_1_cheat_4",
            "blown_1_cheat_4_300",
            "blown_1_cheat_4_1_9",
        ] {
            assert_eq!(parse_hunt_event(bad), None, "{bad}");
        }
        for bad in [
            "mark_",
            "mark_x",
            "hunted_3",
            "hunted_3_2",
            "open_1_empty",
            "laugh_2_4",
            "laugh_2_4_100_1",
            "laugh_x_4_100",
        ] {
            assert_eq!(parse_hunt_event(bad), None, "{bad}");
        }
        let state: HuntState = serde_json::from_str(
            r#"{"hunting":true,"mark":{"slot":4,"name":"Kyle","until":99},"free":1,"every":180}"#,
        )
        .unwrap();
        assert_eq!(state.mark.map(|mark| mark.slot), Some(4));
        assert_eq!(state.free, 1);
    }

    #[test]
    fn the_list_reads_as_the_hub_sends_it() {
        let boxes: Boxes = serde_json::from_str(
            r#"{"boxes":[{"id":3,"map":"mp/ffa3","checksum":-1234,"server":"","origin":[1,2,3],
                "yaw":90,"placed":5}],"price":100,"cooldown":3,
                "outcomes":[{"id":"empty","per_mille":500,"credits":0}]}"#,
        )
        .unwrap();
        assert_eq!(boxes.price, 100);
        let placed = &boxes.boxes[0];
        assert!(placed.stands_on("MP/FFA3", -1234, Some("1.2.3.4:29070")));
        assert!(!placed.stands_on("mp/ffa3", 7, None));
        assert!(boxes.on_map("mp/ffa3") && !boxes.on_map("mp/ffa2"));
        let one = MysteryBox {
            server: "1.2.3.4:29070".to_owned(),
            ..placed.clone()
        };
        assert!(one.stands_on("mp/ffa3", -1234, Some("1.2.3.4:29070")));
        assert!(!one.stands_on("mp/ffa3", -1234, Some("5.6.7.8:29070")));
        assert!(!one.stands_on("mp/ffa3", -1234, None));
    }

    #[test]
    fn an_opening_reads_its_gift_and_holocron_when_the_hub_sends_them() {
        let plain: Opened = serde_json::from_str(
            r#"{"id":1,"box":3,"outcome":"double","price":100,"won":200,"balance":5}"#,
        )
        .unwrap();
        assert_eq!((plain.gift, plain.holocron), (None, None));
        let blown: Opened = serde_json::from_str(
            r#"{"id":2,"box":3,"outcome":"explode","price":100,"won":0,"balance":5,
                "gift":{"slot":4,"name":"Kyle","credits":100}}"#,
        )
        .unwrap();
        assert_eq!(
            blown.gift.map(|gift| (gift.slot, gift.credits)),
            Some((4, 100))
        );
        let found: Opened = serde_json::from_str(
            r#"{"id":3,"box":3,"outcome":"holocron","price":100,"won":0,"balance":5,
                "holocron":{"id":77,"tier":"rare"}}"#,
        )
        .unwrap();
        assert_eq!(found.holocron.map(|h| h.tier), Some("rare".to_owned()));
    }

    #[test]
    fn feed_events_name_the_box_and_what_came_out() {
        let read = |event| parse_event(event).map(|e| (e.id, e.outcome.to_owned(), e.tier));
        assert_eq!(read("open_12_jackpot"), Some((12, "jackpot".into(), None)));
        assert_eq!(read("open_12_big_win"), Some((12, "big_win".into(), None)));
        // Since 11/11/2026 a holocron's and a crystal's tier follow.
        assert_eq!(
            read("open_12_holocron_rare"),
            Some((12, HOLOCRON.into(), Some("rare".into())))
        );
        assert_eq!(
            read("open_12_kyber_legendary"),
            Some((12, KYBER.into(), Some("kyber_legendary".into())))
        );
        // An older hub's holocron names no tier.
        assert_eq!(read("open_12_holocron"), Some((12, HOLOCRON.into(), None)));
        assert_eq!(read("open_x_empty"), None);
        assert_eq!(read("open_12_"), None);
        assert_eq!(read("spotted"), None);
    }

    #[test]
    fn ray_gun_hits_and_new_prizes_read_back() {
        assert_eq!(parse_raygun_hit("hit_5_25"), Some((5, 25)));
        assert_eq!(parse_raygun_hit("hit_5_0"), Some((5, 0)));
        assert_eq!(parse_raygun_hit("hit_x_25"), None);
        assert_eq!(parse_raygun_hit("shot_1"), None);
        let gun: Opened = serde_json::from_str(
            r#"{"id":4,"box":3,"outcome":"raygun","price":100,"won":0,"balance":5,
                "free":0,"doom_coins":2,"raygun":30}"#,
        )
        .unwrap();
        assert_eq!((gun.raygun, gun.doom_coins), (Some(30), Some(2)));
        let crystal: Opened = serde_json::from_str(
            r#"{"id":5,"box":3,"outcome":"kyber","price":100,"won":0,"balance":5,
                "kyber":{"id":9,"tier":"kyber_rare"}}"#,
        )
        .unwrap();
        assert_eq!(crystal.kyber.map(|k| k.tier), Some("kyber_rare".to_owned()));
    }
}
