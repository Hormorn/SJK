//! Quests (the hub's `POST /v1/quests`, since 04/11/2026; wonner67's idea): players
//! wearing look toy [`TOY`] take part. Now and then the hub gives one of them on the
//! server a quest with a minute to finish it; the first kind, `social`, asks them to hug
//! (`amhug`) or kiss (`amkiss`) each of the players it names. The hub never sees an
//! emote: each named player's own client says who hugged or kissed it
//! (`POST /v1/quests/done`), so nobody finishes a quest alone. A finished quest wins an
//! Echo (a holocron, as one earned by playing: within the day's cap), a few times a day.

use serde::Deserialize;

/// The look toy of quests: a player wearing it takes part.
pub const TOY: &str = "quests";

/// What a social quest asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emote {
    /// JA+ `amhug`.
    Hug,
    /// JA+ `amkiss`.
    Kiss,
}

impl Emote {
    /// The hub's word for it.
    pub fn of(word: &str) -> Option<Self> {
        match word {
            "hug" => Some(Self::Hug),
            "kiss" => Some(Self::Kiss),
            _ => None,
        }
    }

    /// The server command that does it.
    pub fn command(self) -> &'static str {
        match self {
            Self::Hug => "amhug",
            Self::Kiss => "amkiss",
        }
    }

    /// The verb, for the player's eyes.
    pub fn verb(self) -> &'static str {
        match self {
            Self::Hug => "hug",
            Self::Kiss => "kiss",
        }
    }
}

/// One player a quest names.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct QuestTarget {
    /// Their slot.
    pub slot: u8,
    /// Their worn name.
    #[serde(default)]
    pub name: String,
    /// Their client said the quest's player did it.
    #[serde(default)]
    pub done: bool,
}

/// A server's quest as the hub tells the players taking part.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Quest {
    /// Its kind: `social`; a kind this client does not know is not shown.
    #[serde(default)]
    pub kind: String,
    /// The slot of the player given it.
    pub slot: u8,
    /// Their worn name.
    #[serde(default)]
    pub name: String,
    /// What they do to each target (`hug`, `kiss`).
    #[serde(default)]
    pub emote: String,
    /// The players named.
    #[serde(default)]
    pub targets: Vec<QuestTarget>,
    /// When it ends, Unix seconds.
    #[serde(default)]
    pub until: i64,
    /// The seconds from the read to then, by the hub's clock.
    #[serde(default)]
    pub left: u64,
    /// What finishing it wins: `echo`.
    #[serde(default)]
    pub reward: String,
}

impl Quest {
    /// What a social quest asks for; None for another kind, or an emote not known.
    pub fn social(&self) -> Option<Emote> {
        (self.kind == "social").then(|| Emote::of(&self.emote))?
    }

    /// Whether the player on `slot` is one of its targets still to do.
    pub fn awaits(&self, slot: u8) -> bool {
        self.targets
            .iter()
            .any(|target| target.slot == slot && !target.done)
    }
}

/// `POST /v1/quests`: the quests on a server as the player reads them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct QuestState {
    /// The player wears the toy there: only then is the quest told.
    #[serde(default)]
    pub questing: bool,
    /// The quest under way, if any.
    #[serde(default)]
    pub quest: Option<Quest>,
    /// How many more quests the player may be paid for today.
    #[serde(default)]
    pub wins_left: u64,
    /// How long a quest lasts, in seconds.
    #[serde(default)]
    pub every: u64,
}

/// What a feed event of [`TOY`] says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestEvent {
    /// The player on `slot` was given a quest: `emote` each of `targets`
    /// (`quest_<slot>_<emote>_<target>-<target>`).
    Given {
        /// The player's slot.
        slot: u8,
        /// The hub's word for the emote.
        emote: String,
        /// The targets' slots.
        targets: Vec<u8>,
    },
    /// The target on `target` said the player on `slot` did it (`done_<slot>_<target>`).
    Done {
        /// The quest's player.
        slot: u8,
        /// The target.
        target: u8,
    },
    /// The player on `slot` finished the quest and won an Echo of `tier`
    /// (`won_<slot>_<tier>`; `won_<slot>_none`, None, past the day's cap).
    Won {
        /// The quest's player.
        slot: u8,
        /// The Echo's tier (`uncommon` .. `mythical`), None for none.
        tier: Option<String>,
    },
    /// The quest of the player on `slot` ran out unfinished (`failed_<slot>`).
    Failed(u8),
}

/// Read a feed event of [`TOY`].
pub fn parse_event(event: &str) -> Option<QuestEvent> {
    if let Some(rest) = event.strip_prefix("quest_") {
        let mut parts = rest.split('_');
        let slot = parts.next()?.parse().ok()?;
        let emote = parts.next()?.to_owned();
        let targets = parts
            .next()?
            .split('-')
            .map(str::parse)
            .collect::<Result<Vec<u8>, _>>()
            .ok()?;
        return (parts.next().is_none() && !emote.is_empty()).then_some(QuestEvent::Given {
            slot,
            emote,
            targets,
        });
    }
    if let Some(rest) = event.strip_prefix("done_") {
        let (slot, target) = rest.split_once('_')?;
        return Some(QuestEvent::Done {
            slot: slot.parse().ok()?,
            target: target.parse().ok()?,
        });
    }
    if let Some(rest) = event.strip_prefix("won_") {
        let (slot, tier) = rest.split_once('_')?;
        if tier.is_empty() {
            return None;
        }
        return Some(QuestEvent::Won {
            slot: slot.parse().ok()?,
            tier: (tier != "none").then(|| tier.to_owned()),
        });
    }
    if let Some(slot) = event.strip_prefix("failed_") {
        return Some(QuestEvent::Failed(slot.parse().ok()?));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_read_as_the_hub_writes_them() {
        assert_eq!(
            parse_event("quest_4_hug_2-7"),
            Some(QuestEvent::Given {
                slot: 4,
                emote: "hug".to_owned(),
                targets: vec![2, 7],
            })
        );
        assert_eq!(
            parse_event("done_4_7"),
            Some(QuestEvent::Done { slot: 4, target: 7 })
        );
        assert_eq!(
            parse_event("won_4_rare"),
            Some(QuestEvent::Won {
                slot: 4,
                tier: Some("rare".to_owned())
            })
        );
        assert_eq!(
            parse_event("won_4_none"),
            Some(QuestEvent::Won {
                slot: 4,
                tier: None
            })
        );
        assert_eq!(parse_event("failed_4"), Some(QuestEvent::Failed(4)));
        for bad in ["quest_4_hug_2-x", "quest_4__2", "done_4", "won_x_1", "nope"] {
            assert_eq!(parse_event(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_read_parses_and_names_its_emote() {
        let state: QuestState = serde_json::from_str(
            r#"{"questing":true,"quest":{"kind":"social","slot":4,"name":"p4","emote":"kiss",
                "targets":[{"slot":2,"name":"p2","done":false},{"slot":7,"name":"p7","done":true}],
                "until":100,"left":42,"reward":"echo"},"wins_left":3,"every":60}"#,
        )
        .unwrap();
        let quest = state.quest.unwrap();
        assert_eq!(quest.social(), Some(Emote::Kiss));
        assert!(quest.awaits(2) && !quest.awaits(7) && !quest.awaits(4));
        let other = Quest {
            kind: "hunt".to_owned(),
            ..quest
        };
        assert_eq!(other.social(), None);
    }
}
