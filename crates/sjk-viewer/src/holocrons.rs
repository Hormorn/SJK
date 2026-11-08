//! Holocrons (`docs/holocrons.md`): loot a player earns by playing. The SJK hub rolls and
//! stores them; the client shows them. Opening one gives a sealed card and credits
//! ([`crate::cards`]).
//!
//! This is the one place that says which tiers the client knows and how each looks: its
//! wire id, name, colour and odds ([`TIERS`]), and how a profile's entries and the hub's
//! progress read ([`entries`], [`counts_of`], [`next_text`]). Everything that draws a
//! holocron reads these: the drop pop-up ([`crate::holocron_popup`]), the chat lines
//! ([`line`]), the staff page and the Profile screen's Holocrons tab (which reads the
//! counts, the list and the progress from the identity's snapshot once, in
//! [`crate::console::holocrons_panel::Data::of`]). An id the client does not know is left
//! out.
//!
//! | piece | module |
//! | --- | --- |
//! | what counts as actively playing, and the input clock | [`activity`] |
//! | the tier gem, drawn when a tier has no icon | [`gem`] |
//! | the tiers' icons in the UI atlas | [`icons`] |
//! | the chat lines' words | [`line`] |
//! | `debug_holocron` | [`rehearsal`] |
//! | `holocrons_seen.txt` | [`seen`] |
//! | the Holocrons tab's 3D holocron | [`stage`] |
//!
//! Adding a tier is one entry in [`TIERS`] (the hub's catalogue has the same ids, names
//! and order) and its icon `gfx/sjk/holocron_<id>.png`.

pub(crate) mod activity;
pub(crate) mod gem;
pub(crate) mod icons;
pub(crate) mod line;
pub(crate) mod rehearsal;
pub(crate) mod seen;
pub(crate) mod signals;
pub(crate) mod stage;

use sjk_identity::{Holocron, HolocronCounts, HolocronProgress};
use sjk_ui::Color;

/// How many tiers the client knows.
pub(crate) const COUNT: usize = 4;
/// The most holocrons the pop-up shows at once when a profile first lists many (a
/// reinstalled PC): the newest of them.
pub(crate) const FIRST_READ_MAX: usize = 20;

/// `0xRRGGBB` as an opaque colour.
pub(crate) const fn rgb(hex: u32) -> Color {
    Color::new(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    )
}

/// One tier of holocron, in the hub's catalogue order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tier {
    /// The hub's id (`rare`), also the end of its icon's name.
    pub(crate) id: &'static str,
    /// The name players read ("Rare Holocron").
    pub(crate) name: &'static str,
    /// The tier alone ("Rare"), as a saber shader's tier reads (`unlockables::Rarity`).
    pub(crate) label: &'static str,
    /// The tier's colour: its pop-up, chat lines, gem and counts. None of the game's
    /// `^n` colours, and four colours that stay apart for players who tell colours
    /// apart poorly.
    pub(crate) colour: Color,
    /// The odds a drop is this tier, in parts per thousand; the four sum to 1000. This
    /// client's copy: what players read is the hub's ([`Tier::odds`]).
    pub(crate) per_mille: u16,
    /// Position in [`TIERS`] (a crystal's in [`crate::kyber::TIERS`]): the rank, which the
    /// odds tables and the rarity effects read.
    pub(crate) index: usize,
    /// Position among every tier the Holocrons tab lists ([`slot`]): a holocron's is its
    /// index, a kyber crystal's comes after the holocrons' ([`COUNT`] + its index). The
    /// tab's rows, its 3D stage ([`stage::Request::tier`]) and its opening read it.
    pub(crate) slot: usize,
    /// The colour of the point light the Profile screen's holocron casts, linear RGB on
    /// the scale of Illuminate's `[1.5, 1.3, 1.0]` and not clamped: the one in
    /// `scripts/holocron_assets.py` (`Tier.light`) and `assets/holocron/README.md`.
    pub(crate) light: [f32; 3],
}

/// Every tier, in the hub's catalogue order (`uncommon`, `rare`, `legendary`,
/// `mythical`: the commonest first).
pub(crate) const TIERS: [Tier; COUNT] = [
    Tier {
        id: "uncommon",
        name: "Uncommon Holocron",
        label: "Uncommon",
        colour: rgb(0x2F_C77A),
        per_mille: 600,
        index: 0,
        slot: 0,
        light: [0.55, 1.50, 0.60],
    },
    Tier {
        id: "rare",
        name: "Rare Holocron",
        label: "Rare",
        colour: rgb(0x2E_7BFF),
        per_mille: 280,
        index: 1,
        slot: 1,
        light: [0.50, 1.00, 1.75],
    },
    Tier {
        id: "legendary",
        name: "Legendary Holocron",
        label: "Legendary",
        colour: rgb(0xA6_4DFF),
        per_mille: 105,
        index: 2,
        slot: 2,
        light: [1.25, 0.45, 1.75],
    },
    Tier {
        id: "mythical",
        name: "Mythical Holocron",
        label: "Mythical",
        colour: rgb(0xFF_C933),
        per_mille: 15,
        index: 3,
        slot: 3,
        light: [1.85, 1.30, 0.40],
    },
];

/// How many tiers the Holocrons tab lists: the holocrons', then the kyber crystals'
/// ([`crate::kyber::TIERS`]).
pub(crate) const SLOTS: usize = COUNT + crate::kyber::COUNT;

/// The tier at `slot` ([`Tier::slot`]), kept to the tiers there are.
pub(crate) fn slot(slot: usize) -> &'static Tier {
    match slot {
        slot if slot < COUNT => &TIERS[slot],
        slot => &crate::kyber::TIERS[(slot - COUNT).min(crate::kyber::COUNT - 1)],
    }
}

/// The holocron or kyber crystal tier the hub's `id` names, if the client knows it. What
/// is about holocrons found by play reads [`Tier::from_id`], which knows holocrons only.
pub(crate) fn any(id: &str) -> Option<&'static Tier> {
    Tier::from_id(id).or_else(|| crate::kyber::from_id(id))
}

impl Tier {
    /// Whether this is a kyber crystal's tier ([`crate::kyber`]), not a holocron's.
    pub(crate) const fn is_kyber(&self) -> bool {
        self.slot >= COUNT
    }

    /// How often a drop is this tier, as players read it ("10.5%"): the hub's odds.
    pub(crate) fn odds(&self) -> String {
        crate::cards::loot::percent(crate::cards::loot::drop(self.index))
    }

    /// The tier the hub's `id` names, if the client knows it.
    pub(crate) fn from_id(id: &str) -> Option<&'static Tier> {
        TIERS.iter().find(|tier| tier.id == id)
    }

    /// The game path of the tier's icon, without its extension
    /// (`gfx/sjk/holocron_rare`). It is a bundled file mounted below all game data
    /// ([`crate::illuminate`]'s list); where it is missing the tier draws as a gem
    /// ([`gem`]).
    pub(crate) fn icon_path(&self) -> String {
        format!("gfx/sjk/holocron_{}", self.id)
    }

    /// What the Profile screen says of the tier under its list: how often it drops and who
    /// hears of a drop (what opening one gives is [`crate::cards::inside`]).
    pub(crate) const fn about(&self) -> &'static str {
        if self.is_kyber() {
            return crate::kyber::ABOUT;
        }
        match self.index {
            0 => "The commonest holocron: about 6 in 10 drops.",
            1 => "About 3 in 10 drops.",
            2 => "About 1 in 10 drops. Every SJK player sees it in chat when one drops.",
            _ => {
                "About 1 in 67 drops, at most 1 a day for a player. Every SJK player sees it in chat when one drops."
            }
        }
    }

    /// The name as players read it ([`crate::terms`]: "Rare Echo" with JoF's names).
    pub(crate) fn title(&self) -> std::borrow::Cow<'static, str> {
        crate::terms::say(self.name)
    }

    /// "a" or "an" for the name: "an Uncommon Holocron", "a Rare Holocron".
    pub(crate) fn article(&self) -> &'static str {
        if self.name.starts_with(['A', 'E', 'I', 'O', 'U']) {
            "an"
        } else {
            "a"
        }
    }

    /// The tier's colour at `alpha`.
    pub(crate) fn colour_alpha(&self, alpha: f32) -> Color {
        Color::new(self.colour.r, self.colour.g, self.colour.b, alpha)
    }

    /// Whether a drop of this tier is told to every SJK player (legendary and mythical)
    /// and not just to its owner.
    pub(crate) fn is_announced(&self) -> bool {
        !self.is_kyber() && self.index >= 2
    }
}

/// The tier the hub's `id` names, if the client knows it.
#[allow(dead_code)] // for screens that know a tier by the hub's id
pub(crate) fn by_id(id: &str) -> Option<&'static Tier> {
    Tier::from_id(id)
}

/// The colour of the tier `id` names; the chat's gold for an id the client does not know.
#[allow(dead_code)] // for screens that know a tier by the hub's id
pub(crate) fn colour(id: &str) -> Color {
    by_id(id).map_or(crate::sjk_chat_look::GOLD, |tier| tier.colour)
}

/// The name of the tier `id` names, if the client knows it.
pub(crate) fn name(id: &str) -> Option<&'static str> {
    by_id(id).map(|tier| tier.name)
}

/// One of the player's own holocrons as their profile lists it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    /// Its number at the hub, which only rises.
    pub(crate) id: u64,
    pub(crate) tier: &'static Tier,
    /// When it dropped, unix seconds; 0 when the hub did not say.
    pub(crate) dropped: i64,
    /// Staff gave it rather than play.
    pub(crate) gift: bool,
    /// An achievement earned it (a kyber crystal, source `achievement`).
    pub(crate) earned: bool,
    /// Play dropped it (source `play`) and it is still the finder's: the Fishing
    /// toy's Echoes are these only, not bought, gifted, earned or traded ones.
    pub(crate) played: bool,
    /// The team's note as plain text; only on a gift, often empty.
    pub(crate) note: String,
    /// It came in a trade: when (unix seconds), and from whom (colour codes kept), when
    /// the hub knows.
    pub(crate) traded: Option<(i64, Option<String>)>,
    /// The edition the hub stamped on it ([`crate::editions`]): what opening it may bring.
    pub(crate) edition: String,
}

impl Entry {
    /// Read a hub's entry, a holocron or a kyber crystal; `None` for a tier the client
    /// does not know.
    pub(crate) fn from_wire(holocron: &Holocron) -> Option<Self> {
        Some(Self {
            id: holocron.id,
            tier: any(&holocron.tier)?,
            dropped: holocron.dropped,
            gift: holocron.source == "staff",
            earned: holocron.source == "achievement",
            played: holocron.source == "play" && holocron.traded.is_none(),
            note: crate::medals::plain_note(&holocron.note),
            traded: holocron
                .traded
                .filter(|_| holocron.traded_from_key.is_some())
                .map(|at| (at, holocron.traded_from.clone())),
            edition: holocron.edition.clone(),
        })
    }

    /// How it came in a trade, for the list: "Received from Nib in a trade", and while
    /// the hub's resale lock (`lock` seconds, `None` before the hub said) holds at `now`,
    /// how long until it may trade again. `None` when it did not come in a trade.
    pub(crate) fn trade_line(&self, now: i64, lock: Option<i64>) -> Option<String> {
        let (at, from) = self.traded.as_ref()?;
        let from = from
            .as_deref()
            .map(|name| format!("{}^7", name))
            .unwrap_or_else(|| "another player".to_owned());
        let left = lock.map_or(0, |lock| at + lock - now);
        Some(if left > 0 {
            let wait = crate::trade_window::countdown(left);
            format!("Received from {from} in a trade - tradable again in {wait}")
        } else {
            format!("Received from {from} in a trade")
        })
    }

    /// "Found 10/10/2026 14:05" (UTC, day first, 24 hours), or "A gift from the SJK
    /// team 10/10/2026 14:05" for a gift; no date when the hub did not say when.
    pub(crate) fn when(&self) -> String {
        let date = when_text(self.dropped);
        if self.earned {
            return if date.is_empty() {
                "Earned from an achievement".to_owned()
            } else {
                format!("Earned {date}")
            };
        }
        match (self.gift, date.is_empty()) {
            (true, true) => "A gift from the SJK team".to_owned(),
            (true, false) => format!("A gift from the SJK team, {date}"),
            (false, true) => String::new(),
            (false, false) => format!("Found {date}"),
        }
    }
}

/// The known holocrons of a profile's list, in the hub's order (newest first): kyber
/// crystals left out. (The Staff page lists crystals too, so only tests ask now.)
#[cfg(test)]
pub(crate) fn entries(list: &[Holocron]) -> Vec<Entry> {
    list.iter()
        .filter_map(Entry::from_wire)
        .filter(|entry| !entry.tier.is_kyber())
        .collect()
}

/// A profile's counts as an array in [`TIERS`] order.
pub(crate) fn counts_of(counts: &HolocronCounts) -> [u32; COUNT] {
    std::array::from_fn(|index| counts.of(TIERS[index].id))
}

/// `dd/mm/yyyy HH:MM` (UTC) of unix `seconds`; empty when it is not after 1970.
pub(crate) fn when_text(seconds: i64) -> String {
    let date = crate::medals::date_text(seconds);
    if date.is_empty() {
        return date;
    }
    let of_day = seconds.rem_euclid(86_400);
    format!("{date} {:02}:{:02}", of_day / 3_600, of_day % 3_600 / 60)
}

/// Ask the hub for fresh progress (a page that shows it, as it opens); at most once
/// every 30 seconds.
pub(crate) fn refresh() {
    crate::player_identity::refresh_holocrons();
}

/// Seconds from `now` until the hub's caps' day ends and the cap lifts (`resets_at`:
/// 09:00 in Paris, since 29/10/2026; `docs/holocrons.md`), when the progress says it is
/// reached; `None` otherwise, or from a hub before that does not say. 0 once past it,
/// until the next reading.
pub(crate) fn cap_lifts_in(progress: &HolocronProgress, now: std::time::Instant) -> Option<u64> {
    if !progress.capped() {
        return None;
    }
    let hub_now = progress.server_time.saturating_add(
        i64::try_from(now.saturating_duration_since(progress.read_at).as_secs())
            .unwrap_or(i64::MAX),
    );
    u64::try_from(progress.resets_at?.saturating_sub(hub_now).max(0)).ok()
}

/// What the progress says in a line: how long to the next holocron, or that the day's
/// limit is reached and, when `lifts_in` says, how long until it resets
/// ([`cap_lifts_in`]). The play time does not count down on its own (it is as the hub
/// last said, and counts only while the player plays), so it is rounded up to whole
/// minutes.
pub(crate) fn next_text(progress: &HolocronProgress, lifts_in: Option<u64>) -> String {
    if progress.capped() {
        return match lifts_in {
            Some(secs) => {
                let minutes = secs.div_ceil(60).max(1);
                match (minutes / 60, minutes % 60) {
                    (0, minutes) => format!("Limit reached: resets in about {minutes} min"),
                    (hours, 0) => format!("Limit reached: resets in about {hours} h"),
                    (hours, minutes) => {
                        format!("Limit reached: resets in about {hours} h {minutes} min")
                    }
                }
            }
            None => "Limit reached for today".to_owned(),
        };
    }
    let minutes = progress.remaining_secs().div_ceil(60).max(1);
    // While catch-up drops are left today, the next comes twice as fast (03/11/2026).
    let catchup = if progress.catchup_left() > 0 {
        " (catch-up x2)"
    } else {
        ""
    };
    if minutes == 1 {
        format!("Next holocron in about 1 minute of play{catchup}")
    } else {
        format!("Next holocron in about {minutes} minutes of play{catchup}")
    }
}

/// What the hub says of the player's earning, in a line (03/11/2026, Rollembones' idea;
/// `docs/holocrons.md`, "Earning"): the catch-up left today, and the pace the last claims
/// earned at when it is not full. Empty when there is nothing to say, and from hubs
/// before.
pub(crate) fn pace_text(progress: &HolocronProgress) -> String {
    let mut parts: Vec<String> = Vec::new();
    let left = progress.catchup_left();
    if left > 0 && !progress.capped() {
        parts.push(if left == 1 {
            "Catch-up x2: 1 more holocron at double speed today".to_owned()
        } else {
            format!("Catch-up x2: {left} more holocrons at double speed today")
        });
    }
    if let Some(activity) = progress.activity {
        let minutes = (activity.combat_window / 60).max(1);
        match activity.pace {
            Some(0) if activity.fishing_window > 0 => {
                parts.push("Not earning: move around, fight or fish to earn holocrons".to_owned())
            }
            Some(0) => parts.push("Not earning: move around or fight to earn holocrons".to_owned()),
            Some(pace) if pace < 100 => parts.push(format!(
                "{} pace: no fight in the last {minutes} minutes. Fight, get hit or duel for \
                 full pace",
                if pace == 50 {
                    "Half".to_owned()
                } else {
                    format!("{pace}%")
                }
            )),
            _ => {}
        }
    }
    parts.join(". ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::CodePalette;

    fn distance(a: Color, b: [f32; 4]) -> f32 {
        ((a.r - b[0]).powi(2) + (a.g - b[1]).powi(2) + (a.b - b[2]).powi(2)).sqrt()
    }

    /// The list is pinned, in the hub's catalogue order: `src/holocrons.rs` in
    /// Sol-Vulpes/SJK-hub carries the same ids, names and odds, and its test pins them.
    #[test]
    fn the_catalogue_is_the_hubs() {
        let list: Vec<(&str, &str, u16, String)> = TIERS
            .iter()
            .map(|tier| (tier.id, tier.name, tier.per_mille, tier.odds()))
            .collect();
        assert_eq!(
            list,
            [
                ("uncommon", "Uncommon Holocron", 600, "60%".to_owned()),
                ("rare", "Rare Holocron", 280, "28%".to_owned()),
                ("legendary", "Legendary Holocron", 105, "10.5%".to_owned()),
                ("mythical", "Mythical Holocron", 15, "1.5%".to_owned()),
            ]
        );
        assert_eq!(
            TIERS.iter().map(|t| u32::from(t.per_mille)).sum::<u32>(),
            1000
        );
        for (index, tier) in TIERS.iter().enumerate() {
            assert_eq!(tier.index, index);
            assert_eq!(Tier::from_id(tier.id), Some(tier));
            assert_eq!(by_id(tier.id).map(|t| t.name), Some(tier.name));
            assert!(tier.name.ends_with(" Holocron"));
            assert_eq!(tier.name, format!("{} Holocron", tier.label));
        }
        assert_eq!(by_id("from_the_future"), None);
        assert_eq!(name("rare"), Some("Rare Holocron"));
        assert_eq!(name("nope"), None);
    }

    /// The tier colours are none of the game's `^n` codes (in the game's palette and in
    /// the SJK UI's lifted one), as the SJK chat's gold is none of them
    /// (`sjk_chat_look.rs`): a drop line is never mistaken for a coloured player name.
    /// And the four are apart from one another, for players who tell hues apart poorly.
    #[test]
    fn the_colours_are_none_of_the_colour_codes_and_apart_from_each_other() {
        for tier in &TIERS {
            for index in 0..=9 {
                for palette in [CodePalette::Game, CodePalette::Legible] {
                    let code = palette.colour(index);
                    assert!(
                        distance(tier.colour, code) > 0.28,
                        "{} is too near ^{index} {palette:?}: {code:?}",
                        tier.id
                    );
                }
            }
            assert_eq!(tier.colour.a, 1.0);
        }
        for (a, first) in TIERS.iter().enumerate() {
            for second in &TIERS[a + 1..] {
                let apart = distance(
                    first.colour,
                    [second.colour.r, second.colour.g, second.colour.b, 1.0],
                );
                assert!(apart > 0.4, "{} and {}: {apart}", first.id, second.id);
            }
        }
    }

    #[test]
    fn names_articles_icons_and_who_hears_of_a_drop() {
        assert_eq!(TIERS[0].article(), "an");
        assert_eq!(TIERS[1].article(), "a");
        assert_eq!(TIERS[1].icon_path(), "gfx/sjk/holocron_rare");
        let announced: Vec<&str> = TIERS
            .iter()
            .filter(|tier| tier.is_announced())
            .map(|tier| tier.id)
            .collect();
        assert_eq!(announced, ["legendary", "mythical"], "as the hub delivers");
        assert_eq!(TIERS[2].colour_alpha(0.5).a, 0.5);
        assert_eq!(colour("legendary"), TIERS[2].colour);
        assert_eq!(colour("nope"), crate::sjk_chat_look::GOLD);
    }

    fn wire(id: u64, tier: &str, source: &str, note: &str) -> Holocron {
        Holocron {
            opened: None,
            traded: None,
            traded_from_key: None,
            traded_from: None,
            edition: sjk_identity::wire::EDITION_FIRST.to_owned(),
            id,
            tier: tier.to_owned(),
            dropped: 1_791_641_100,
            source: source.to_owned(),
            note: note.to_owned(),
        }
    }

    #[test]
    fn entries_keep_the_known_tiers_in_the_hubs_order_with_plain_notes() {
        let list = [
            wire(9, "legendary", "play", ""),
            wire(8, "from_the_future", "play", ""),
            wire(7, "rare", "staff", "For the ^1fog^7 bug"),
        ];
        let entries = entries(&list);
        let ids: Vec<u64> = entries.iter().map(|entry| entry.id).collect();
        assert_eq!(ids, [9, 7]);
        assert!(!entries[0].gift && entries[1].gift);
        assert_eq!(entries[1].note, "For the fog bug");
        assert_eq!(entries[0].when(), "Found 10/10/2026 14:05");
        assert_eq!(
            entries[1].when(),
            "A gift from the SJK team, 10/10/2026 14:05"
        );
        let undated = Entry {
            dropped: 0,
            ..entries[0].clone()
        };
        assert_eq!(undated.when(), "");
        let undated_gift = Entry {
            dropped: 0,
            ..entries[1].clone()
        };
        assert_eq!(undated_gift.when(), "A gift from the SJK team");
    }

    /// A kyber crystal reads as an entry of its own tier for the arrival, but is not one
    /// of the holocrons a profile's list gives the screens about play.
    #[test]
    fn crystals_are_entries_of_their_own_but_not_holocrons() {
        let list = [
            wire(9, "kyber_rare", "achievement", ""),
            wire(8, "rare", "play", ""),
        ];
        let crystal = Entry::from_wire(&list[0]).expect("a crystal");
        assert_eq!(crystal.tier.id, "kyber_rare");
        assert!(crystal.earned && !crystal.gift);
        assert_eq!(crystal.when(), "Earned 10/10/2026 14:05");
        let ids: Vec<u64> = entries(&list).iter().map(|entry| entry.id).collect();
        assert_eq!(ids, [8], "crystals left out");
        assert_eq!(slot(1).id, "rare");
        assert_eq!(slot(COUNT + 2).id, "kyber_legendary");
        assert_eq!(slot(99).id, "kyber_mythical", "kept to the tiers there are");
        assert_eq!(any("kyber_uncommon").map(|t| t.slot), Some(COUNT));
        assert_eq!(
            TIERS[2].about(),
            Tier::from_id("legendary").unwrap().about()
        );
        assert_eq!(slot(COUNT).about(), crate::kyber::ABOUT);
    }

    #[test]
    fn counts_are_in_the_catalogues_order() {
        let counts = HolocronCounts {
            uncommon: 5,
            rare: 3,
            legendary: 1,
            mythical: 0,
        };
        assert_eq!(counts_of(&counts), [5, 3, 1, 0]);
        assert_eq!(when_text(0), "");
        assert_eq!(when_text(1_791_641_100), "10/10/2026 14:05");
        assert_eq!(when_text(86_399), "01/01/1970 23:59");
    }

    fn progress(progress_secs: u64, today: u32) -> HolocronProgress {
        HolocronProgress {
            progress_secs,
            every_secs: 1_800,
            today,
            daily_cap: 8,
            resets_at: None,
            server_time: 0,
            can_open: true,
            catchup: None,
            activity: None,
            read_at: std::time::Instant::now(),
        }
    }

    #[test]
    fn the_next_holocron_is_said_in_whole_minutes_or_the_limit() {
        assert_eq!(
            next_text(&progress(0, 0), None),
            "Next holocron in about 30 minutes of play"
        );
        assert_eq!(
            next_text(&progress(600, 3), None),
            "Next holocron in about 20 minutes of play"
        );
        assert_eq!(
            next_text(&progress(1_741, 3), None),
            "Next holocron in about 1 minute of play",
            "rounded up"
        );
        assert_eq!(
            next_text(&progress(1_800, 3), None),
            "Next holocron in about 1 minute of play"
        );
        assert_eq!(
            next_text(&progress(300, 8), None),
            "Limit reached for today"
        );
        assert_eq!(
            next_text(&progress(300, 8), Some(5 * 3_600 + 19 * 60 + 1)),
            "Limit reached: resets in about 5 h 20 min"
        );
        assert_eq!(
            next_text(&progress(300, 8), Some(2 * 3_600)),
            "Limit reached: resets in about 2 h"
        );
        assert_eq!(
            next_text(&progress(300, 8), Some(0)),
            "Limit reached: resets in about 1 min"
        );
        assert_eq!(
            next_text(&progress(600, 3), Some(600)),
            "Next holocron in about 20 minutes of play",
            "not capped: the play time"
        );
    }

    /// Since 29/10/2026 the caps' day ends at 09:00 in Paris for everyone (Sol), and the
    /// hub says when (`resets_at`): the page counts down to it. Before, the page worked
    /// the end of a rolling 24 hours out of the profile's list, which a traded holocron
    /// put off.
    #[test]
    fn the_cap_lifts_when_the_hubs_day_ends() {
        let now = std::time::Instant::now();
        // 29/10/2026 05:00 UTC (06:00 in Paris); the day ends at 08:00 UTC.
        let capped = HolocronProgress {
            server_time: 1_793_250_000,
            resets_at: Some(1_793_260_800),
            catchup: None,
            activity: None,
            read_at: now,
            ..progress(1_800, 8)
        };
        assert_eq!(cap_lifts_in(&capped, now), Some(3 * 3_600));
        assert_eq!(
            cap_lifts_in(&capped, now + std::time::Duration::from_secs(600)),
            Some(3 * 3_600 - 600),
            "counts down from when the hub answered"
        );
        assert_eq!(
            cap_lifts_in(&capped, now + std::time::Duration::from_secs(4 * 3_600)),
            Some(0),
            "past it until the next reading"
        );
        let old_hub = HolocronProgress {
            resets_at: None,
            ..capped
        };
        assert_eq!(
            cap_lifts_in(&old_hub, now),
            None,
            "a hub before does not say"
        );
        let open = HolocronProgress { today: 3, ..capped };
        assert_eq!(cap_lifts_in(&open, now), None, "not capped");
    }

    /// While catch-up drops are left, the next holocron needs the catch-up's interval and
    /// the line says so; the pace line tells half and no pace.
    #[test]
    fn catch_up_halves_the_wait_and_the_pace_is_told() {
        let catchup = sjk_identity::Catchup {
            pool: 4,
            left: 3,
            every_secs: 900,
            max: 4,
        };
        let fast = HolocronProgress {
            catchup: Some(catchup),
            ..progress(600, 1)
        };
        assert_eq!(fast.remaining_secs(), 300);
        assert_eq!(
            next_text(&fast, None),
            "Next holocron in about 5 minutes of play (catch-up x2)"
        );
        assert_eq!(
            pace_text(&fast),
            "Catch-up x2: 3 more holocrons at double speed today"
        );
        let spent = HolocronProgress {
            catchup: Some(sjk_identity::Catchup { left: 0, ..catchup }),
            ..progress(600, 5)
        };
        assert_eq!(spent.remaining_secs(), 1_200);
        assert_eq!(pace_text(&spent), "");
        let activity = sjk_identity::ActivityState {
            pace: Some(50),
            last_combat: 0,
            combat_window: 600,
            peaceful: 50,
            move_min: 200,
            last_bite: 0,
            fishing_window: 0,
            fishing: 0,
        };
        let half = HolocronProgress {
            activity: Some(activity),
            ..progress(600, 1)
        };
        assert_eq!(
            pace_text(&half),
            "Half pace: no fight in the last 10 minutes. Fight, get hit or duel for full pace"
        );
        let idle = HolocronProgress {
            activity: Some(sjk_identity::ActivityState {
                pace: Some(0),
                ..activity
            }),
            ..progress(600, 1)
        };
        assert_eq!(
            pace_text(&idle),
            "Not earning: move around or fight to earn holocrons"
        );
        // A hub that counts fishing says so.
        let idle_by_water = HolocronProgress {
            activity: Some(sjk_identity::ActivityState {
                pace: Some(0),
                fishing_window: 300,
                fishing: 100,
                ..activity
            }),
            ..progress(600, 1)
        };
        assert_eq!(
            pace_text(&idle_by_water),
            "Not earning: move around, fight or fish to earn holocrons"
        );
        let full = HolocronProgress {
            activity: Some(sjk_identity::ActivityState {
                pace: Some(100),
                ..activity
            }),
            ..progress(600, 1)
        };
        assert_eq!(pace_text(&full), "");
    }
}
