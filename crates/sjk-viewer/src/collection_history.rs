//! The Collection's History tab (31/10/2026, Sol: "a general inventory history"): what came
//! to the player, what they opened and what it held, the credits and kyber shards they
//! gained and spent, the mystery boxes they opened and the trades they finished, newest
//! first, as the hub keeps it (`GET /v1/history`, [`sjk_identity::history`]).
//!
//! A row is one line in a sentence, a chip of its kind in its tier's colour, what it
//! changed in credits and shards (gains green, spends red) and when, in this PC's time.
//! Consecutive openings of one second (an "Open all") make one row, and so do holocrons
//! that came in one second the same way: Enter or a click opens it on its lines. A row of
//! chips filters what was read, client-side. The tab reads the newest page each time it is
//! shown; Load older, at the end of the list, the next while the hub has more.
//!
//! The rows' words are made once for what was read, the filter and the rows opened
//! ([`History::follow`]), not each frame.

use super::*;
use sjk_identity::history::{
    BoxLine, Came, HistoryLine, HistoryState, Ledger, OpenedLine, TradeLine,
};
use sjk_ui::Color;
use std::collections::BTreeSet;
use std::sync::Arc;

/// The filter's chips, Load older and the rows shown (one token to each place on screen),
/// clear of the Tags tab's (to 8 199) and the Forge's (9 000 on).
pub(super) const FILTER_BASE: u16 = 8_300;
pub(super) const MORE_TOKEN: u16 = 8_310;
pub(super) const ROW_BASE: u16 = 8_320;
/// The most rows the list shows at once.
pub(super) const ROWS_SHOWN: usize = 15;
const _: () = assert!(FILTER_BASE as usize + Filter::ALL.len() <= MORE_TOKEN as usize);
const _: () = assert!(ROW_BASE as usize + ROWS_SHOWN <= 8_400);

/// What the list shows of the lines read.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Filter {
    #[default]
    All,
    /// What came: holocrons and kyber crystals.
    Drops,
    /// What was opened.
    Opened,
    /// Whatever moved credits: openings that held some, purchases, gifts, boxes.
    Credits,
    /// Whatever moved kyber shards.
    Shards,
    Trades,
    /// Mystery boxes.
    Boxes,
}

impl Filter {
    pub(super) const ALL: [Self; 7] = [
        Self::All,
        Self::Drops,
        Self::Opened,
        Self::Credits,
        Self::Shards,
        Self::Trades,
        Self::Boxes,
    ];

    /// Its chip's word.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Drops => "Drops",
            Self::Opened => "Opened",
            Self::Credits => "Credits",
            Self::Shards => "Shards",
            Self::Trades => "Trades",
            Self::Boxes => "Boxes",
        }
    }

    /// Its place in [`Filter::ALL`].
    pub(super) fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|filter| *filter == self)
            .unwrap_or(0)
    }

    /// Whether `line` stays in the list.
    pub(super) fn passes(self, line: &HistoryLine) -> bool {
        match (self, line) {
            (Self::All, _)
            | (Self::Drops, HistoryLine::Came(_))
            | (Self::Opened, HistoryLine::Opened(_))
            | (Self::Credits, HistoryLine::Credits(_))
            | (Self::Shards, HistoryLine::Shards(_))
            | (Self::Trades, HistoryLine::Trade(_))
            | (Self::Boxes, HistoryLine::Box(_)) => true,
            (Self::Credits, HistoryLine::Opened(line)) => line.credits != 0,
            (Self::Credits, HistoryLine::Came(line)) => line.price != 0,
            (Self::Credits, HistoryLine::Box(line)) => line.won != line.price,
            (Self::Shards, HistoryLine::Opened(line)) => line.shards != 0,
            (Self::Shards, HistoryLine::Box(line)) => line.shards != 0,
            _ => false,
        }
    }
}

/// A change in credits or kyber shards a row shows, signed as players read it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Amount {
    /// "+120 credits", "-400 credits", "+15 shards".
    pub(super) text: String,
    /// A gain (green), else a spend (red).
    pub(super) gain: bool,
}

impl Amount {
    fn of(delta: i64, unit: &str) -> Option<Self> {
        (delta != 0).then(|| Self {
            text: format!(
                "{}{} {unit}",
                if delta > 0 { '+' } else { '-' },
                crate::cards::grouped(delta.unsigned_abs())
            ),
            gain: delta > 0,
        })
    }

    /// The credits and shards of a line, those that moved.
    fn pair(credits: i64, shards: i64) -> Vec<Self> {
        Self::of(credits, "credits")
            .into_iter()
            .chain(Self::of(shards, "shards"))
            .collect()
    }
}

/// One row of the list, its words made.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Row {
    /// The kind's word on the chip ("Drop", "Opened") and the chip's colour.
    pub(super) chip: &'static str,
    pub(super) colour: Color,
    /// The sentence.
    pub(super) text: String,
    pub(super) amounts: Vec<Amount>,
    /// When, `dd/mm/yyyy HH:MM` in this PC's time; empty on a group's lines.
    pub(super) when: String,
    /// A group's row: its key, whether it is open, and how many lines it holds.
    pub(super) group: Option<(GroupKey, bool, usize)>,
    /// One of an open group's lines, drawn under it.
    pub(super) inner: bool,
}

/// What tells a group from the others, kept while what is read changes: its second, its
/// kind and how its things came.
pub(super) type GroupKey = (i64, u8, String);

/// What the rows were made from, to make them again only when it changes.
#[derive(Clone, Debug, PartialEq)]
struct BuiltFrom {
    state: usize,
    serial: u64,
    filter: Filter,
    opened: u64,
    renamed: bool,
    faces: u64,
}

/// The History tab's state.
#[derive(Debug, Default)]
pub(super) struct History {
    pub(super) filter: Filter,
    /// The rows the filter shows, groups' lines under open groups; then, while the hub has
    /// more, the list ends on Load older ([`History::more_row`]).
    pub(super) rows: Vec<Row>,
    /// Whether the hub has older lines (Load older shows), and what to ask them with.
    pub(super) more: Option<i64>,
    /// The row chosen (`rows.len()` for Load older) and the first one shown.
    pub(super) chosen: usize,
    pub(super) first: usize,
    /// The groups opened.
    open: BTreeSet<GroupKey>,
    open_changes: u64,
    built: Option<BuiltFrom>,
    /// The tab was on show last frame: showing it again reads the newest page.
    pub(super) shown: bool,
    /// A read the console is to send (the page before this time, or the newest), and the
    /// read count the answer will move on from, while it is awaited.
    ask: Option<Option<i64>>,
    pub(super) waiting: Option<u64>,
    /// What was read, kept for the words under the list ([`HistoryState::message`]).
    pub(super) state: Option<Arc<HistoryState>>,
    /// What a world shot or a test shows in place of the identity's.
    #[cfg(test)]
    pub(super) preview: Option<Arc<HistoryState>>,
}

impl History {
    /// How many places the list offers to choose: its rows and Load older.
    pub(super) fn places(&self) -> usize {
        self.rows.len() + usize::from(self.more.is_some())
    }

    /// Whether place `index` is Load older.
    pub(super) fn more_row(&self, index: usize) -> bool {
        self.more.is_some() && index == self.rows.len()
    }

    /// Whether a read is awaited.
    pub(super) fn loading(&self) -> bool {
        self.waiting.is_some() || self.ask.is_some()
    }

    /// Take what the identity read this frame (`state`, `None` before the service
    /// started); `on_show` is whether the tab shows. Showing it again asks for the newest
    /// page; the rows are made again only when what they show changed.
    pub(super) fn follow(&mut self, state: Option<Arc<HistoryState>>, on_show: bool) {
        #[cfg(test)]
        let state = self.preview.clone().or(state);
        let serial = state.as_ref().map_or(0, |state| state.serial);
        if on_show && !self.shown {
            self.ask = Some(None);
            self.waiting = Some(serial);
        }
        self.shown = on_show;
        if self.waiting.is_some_and(|asked| asked != serial) && self.ask.is_none() {
            self.waiting = None;
        }
        let from = BuiltFrom {
            state: state
                .as_ref()
                .map_or(0, |state| Arc::as_ptr(state) as usize),
            serial,
            filter: self.filter,
            opened: self.open_changes,
            renamed: crate::terms::renamed(),
            faces: crate::cards::faces::current().generation,
        };
        if self.built.as_ref() != Some(&from) {
            let lines = state.as_ref().map_or(&[][..], |state| &state.lines[..]);
            self.rows = rows(lines, self.filter, &self.open);
            self.more = state
                .as_ref()
                .filter(|state| state.more)
                .map(|state| state.before);
            self.built = Some(from);
            self.chosen = self.chosen.min(self.places().saturating_sub(1));
        }
        self.state = state;
    }

    /// The read the console is to send now, once: the newest page (`Some(None)`) or the
    /// one before a time.
    pub(super) fn take_ask(&mut self) -> Option<Option<i64>> {
        self.ask.take()
    }

    /// The read could not go (the identity has not started): stop waiting for it.
    pub(super) fn not_sent(&mut self) {
        self.waiting = None;
    }

    /// Ask for the page of older lines, when the hub has more and nothing is awaited.
    pub(super) fn load_older(&mut self) {
        if let Some(before) = self.more
            && !self.loading()
        {
            self.ask = Some(Some(before));
            self.waiting = Some(self.state.as_ref().map_or(0, |state| state.serial));
        }
    }

    /// Show `filter`'s lines, from the top.
    pub(super) fn set_filter(&mut self, filter: Filter) {
        if self.filter != filter {
            self.filter = filter;
            self.chosen = 0;
            self.first = 0;
        }
    }

    /// Enter or a click on place `index`: open or close a group, or load older lines.
    pub(super) fn activate(&mut self, index: usize) {
        self.chosen = index;
        if self.more_row(index) {
            self.load_older();
            return;
        }
        let Some((key, open, _)) = self.rows.get(index).and_then(|row| row.group.clone()) else {
            return;
        };
        if open {
            self.open.remove(&key);
        } else {
            self.open.insert(key);
        }
        self.open_changes += 1;
    }

    /// Move the choice `by` places, stopping at the ends, the list scrolling with it.
    pub(super) fn step(&mut self, by: isize) {
        let last = self.places().saturating_sub(1) as isize;
        self.chosen = (self.chosen as isize + by).clamp(0, last) as usize;
        self.keep_in_view();
    }

    /// Scroll the list `by` rows (the wheel), the choice kept within what shows.
    pub(super) fn scroll(&mut self, by: isize) {
        let most = self.places().saturating_sub(ROWS_SHOWN) as isize;
        self.first = (self.first as isize + by).clamp(0, most) as usize;
        self.chosen = self
            .chosen
            .clamp(self.first, self.first + ROWS_SHOWN.saturating_sub(1))
            .min(self.places().saturating_sub(1));
    }

    /// Scroll so the choice shows.
    pub(super) fn keep_in_view(&mut self) {
        if self.chosen < self.first {
            self.first = self.chosen;
        } else if self.chosen >= self.first + ROWS_SHOWN {
            self.first = self.chosen + 1 - ROWS_SHOWN;
        }
        let most = self.places().saturating_sub(ROWS_SHOWN);
        self.first = self.first.min(most);
    }
}

/// The rows of `lines` that pass `filter`, consecutive openings of one second and things
/// that came the same way in one second gathered, the groups in `open` with their lines
/// under them.
pub(super) fn rows(lines: &[HistoryLine], filter: Filter, open: &BTreeSet<GroupKey>) -> Vec<Row> {
    let kept: Vec<&HistoryLine> = lines.iter().filter(|line| filter.passes(line)).collect();
    let mut rows = Vec::with_capacity(kept.len());
    let mut at = 0;
    while at < kept.len() {
        let key = group_key(kept[at]);
        let mut end = at + 1;
        if let Some(key) = &key {
            while end < kept.len() && group_key(kept[end]).as_ref() == Some(key) {
                end += 1;
            }
        }
        let run = &kept[at..end];
        match key {
            Some(key) if run.len() > 1 => {
                let opened = open.contains(&key);
                let mut head = group_row(run);
                head.group = Some((key, opened, run.len()));
                rows.push(head);
                if opened {
                    rows.extend(run.iter().map(|line| Row {
                        when: String::new(),
                        inner: true,
                        ..row(line)
                    }));
                }
            }
            _ => rows.push(row(kept[at])),
        }
        at = end;
    }
    rows
}

/// What gathers `line` with its neighbours: openings by their second, things that came by
/// their second and how they came; `None` for the other kinds.
fn group_key(line: &HistoryLine) -> Option<GroupKey> {
    match line {
        HistoryLine::Opened(line) => Some((line.at, 0, String::new())),
        HistoryLine::Came(line) => Some((
            line.at,
            1,
            format!("{}{}", line.source, if line.traded { "+" } else { "" }),
        )),
        _ => None,
    }
}

/// The colour of tier `id`: a holocron's or crystal's, a card's rarity's (Common's
/// silver), else gold.
pub(super) fn tier_colour(id: &str) -> Color {
    crate::holocrons::any(id)
        .map(|tier| tier.colour)
        .or_else(|| crate::cards::Rarity::from_id(id).map(|rarity| rarity.colour))
        .unwrap_or(crate::menu::sjk::color::GOLD)
}

/// `id` with its first letter in capitals, `_` made spaces ("kyber_rare" "Kyber rare").
fn capitalised(id: &str) -> String {
    let words = id.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The name of tier `id`'s thing, as players read it ("Rare Holocron", "Rare Kyber
/// Crystal"); a tier this client does not know is a holocron of that name.
fn thing(id: &str) -> String {
    match crate::holocrons::any(id) {
        Some(tier) => tier.title().into_owned(),
        None if id.is_empty() => crate::terms::say("Holocron").into_owned(),
        None => crate::terms::say_owned(format!("{} Holocron", capitalised(id))),
    }
}

/// [`thing`] for `count` of them ("3 Rare Holocrons").
fn things(id: &str, count: usize) -> String {
    if count == 1 {
        return format!("1 {}", thing(id));
    }
    let name = match crate::holocrons::any(id) {
        Some(tier) => format!("{}s", tier.name),
        None if id.is_empty() => "Holocrons".to_owned(),
        None => format!("{} Holocrons", capitalised(id)),
    };
    crate::terms::say_owned(format!("{count} {name}"))
}

/// "a" or "an" before `word`.
fn article(word: &str) -> &'static str {
    if word.starts_with(['A', 'E', 'I', 'O', 'U', 'a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

/// The tier's short name ("Rare"), for a group's counts.
fn tier_label(id: &str) -> String {
    match crate::holocrons::any(id) {
        Some(tier) if tier.is_kyber() => format!("{} crystal", tier.label),
        Some(tier) => tier.label.to_owned(),
        None => capitalised(id),
    }
}

/// The name of unlock `id` as players read it: a shader's or a tag's, else its id.
pub(super) fn unlock_name(id: &str) -> String {
    crate::unlockables::find(id)
        .map(|unlock| unlock.name)
        .or_else(|| {
            crate::tags::CATALOGUE
                .iter()
                .find(|tag| tag.id.eq_ignore_ascii_case(id.trim()))
                .map(|tag| tag.name)
        })
        .map_or_else(|| id.trim().to_owned(), str::to_owned)
}

/// A team's note as one plain line, after " - "; nothing for none.
fn with_note(text: String, note: &str) -> String {
    let note = crate::medals::plain_note(note);
    if note.trim().is_empty() {
        text
    } else {
        format!("{text} - {}", note.trim())
    }
}

/// The row of one line.
pub(super) fn row(line: &HistoryLine) -> Row {
    use crate::menu::sjk::color;
    let at = line.at();
    let base = |chip, colour, text: String, amounts| Row {
        chip,
        colour,
        text,
        amounts,
        when: sjk_identity::history::local_date_time(at),
        group: None,
        inner: false,
    };
    match line {
        HistoryLine::Came(came) => base(
            "Drop",
            tier_colour(&came.tier),
            came_text(came),
            Amount::of(-came.price, "credits").into_iter().collect(),
        ),
        HistoryLine::Opened(opened) => base(
            "Opened",
            tier_colour(&opened.tier),
            opened_text(opened),
            Amount::pair(opened.credits, opened.shards),
        ),
        HistoryLine::Credits(ledger) => base(
            "Credits",
            color::GOLD,
            credits_text(ledger),
            Amount::of(ledger.delta, "credits").into_iter().collect(),
        ),
        HistoryLine::Shards(ledger) => base(
            "Shards",
            crate::holocrons::rgb(0x5F_E3F0),
            shards_text(ledger),
            Amount::of(ledger.delta, "shards").into_iter().collect(),
        ),
        HistoryLine::Box(opened) => base(
            "Box",
            color::EMBER,
            box_text(opened),
            Amount::pair(opened.won - opened.price, opened.shards),
        ),
        HistoryLine::Trade(trade) => base("Trade", color::HOLO, trade_text(trade), vec![]),
    }
}

/// How a holocron or crystal came, in a sentence.
pub(super) fn came_text(came: &Came) -> String {
    let name = thing(&came.tier);
    let a = article(&name);
    if came.traded {
        return format!("Received {a} {name} in a trade");
    }
    match came.source.as_str() {
        "play" => format!("Found {a} {name}"),
        "staff" => with_note(format!("A gift from the SJK team: {a} {name}"), &came.note),
        "achievement" => format!("Earned {a} {name} from an achievement"),
        "bought" => format!("Bought {a} {name}"),
        "box" => format!("{} {name} from a mystery box", capitalised(a)),
        "wheel" => format!("{} {name} from the WHEEL OF DOOM", capitalised(a)),
        "toy" => format!("{} {name} a Toy Master's toy threw out", capitalised(a)),
        "reward" => with_note(format!("Reward: {a} {name}"), &came.note),
        _ => with_note(format!("Received {a} {name}"), &came.note),
    }
}

/// What an opening held, after "Opened a Rare Holocron": its card and its unlock (the
/// credits and shards are the row's amounts).
fn contents(opened: &OpenedLine) -> Vec<String> {
    let mut parts = Vec::with_capacity(2);
    if let Some(card) = &opened.card {
        let rarity = crate::cards::Rarity::from_id(&card.rarity).map_or_else(
            || capitalised(&card.rarity),
            |rarity| rarity.name.to_owned(),
        );
        let finish = crate::cards::FINISHES
            .iter()
            .find(|finish| finish.id == card.finish && finish.index > 0)
            .map(|finish| format!(" ({})", finish.name))
            .unwrap_or_default();
        parts.push(match card.card.as_deref() {
            Some(id) => format!("card {}{finish}", crate::cards::faces::name_of(id)),
            None => format!("{} {rarity} card, sealed{finish}", article(&rarity)),
        });
    }
    if !opened.unlock.trim().is_empty() {
        parts.push(unlock_name(&opened.unlock));
    }
    parts
}

/// An opening in a sentence: "Opened a Rare Holocron: card Sol (Holo), Tidal blade".
pub(super) fn opened_text(opened: &OpenedLine) -> String {
    let name = thing(&opened.tier);
    let head = format!("Opened {} {name}", article(&name));
    let parts = contents(opened);
    if parts.is_empty() {
        head
    } else {
        format!("{head}: {}", parts.join(", "))
    }
}

/// A line of the credits ledger in a sentence.
pub(super) fn credits_text(ledger: &Ledger) -> String {
    match ledger.reason.as_str() {
        "holocron_buy" => crate::terms::say("Bought a holocron").into_owned(),
        "staff" if ledger.delta < 0 => {
            with_note("Corrected by the SJK team".to_owned(), &ledger.note)
        }
        "staff" => with_note("A gift from the SJK team".to_owned(), &ledger.note),
        "reward" => with_note("Contributor reward".to_owned(), &ledger.note),
        "mystery_box" if ledger.note == "explode_gift" => {
            "Another player's mystery box blew up: what they paid came to you".to_owned()
        }
        "mystery_box" => "Mystery box".to_owned(),
        "wheel_of_doom" => "Spun the WHEEL OF DOOM".to_owned(),
        "toy" => "Taken from a Toy Master's toy".to_owned(),
        "box_hunt" if ledger.note == "survived" => {
            "Box hunt: you outlasted every hunter".to_owned()
        }
        reason => with_note(format!("Credits: {}", capitalised(reason)), &ledger.note),
    }
}

/// A line of the kyber shard ledger in a sentence.
pub(super) fn shards_text(ledger: &Ledger) -> String {
    match ledger.reason.as_str() {
        "shatter" => format!("Shattered {}", unlock_name(&ledger.note)),
        "craft" => format!("Crafted {}", unlock_name(&ledger.note)),
        "shop" => format!("Bought {} in the shop", unlock_name(&ledger.note)),
        "staff" if ledger.delta < 0 => {
            with_note("Corrected by the SJK team".to_owned(), &ledger.note)
        }
        "staff" => with_note("A gift from the SJK team".to_owned(), &ledger.note),
        "reward" => with_note("Contributor reward".to_owned(), &ledger.note),
        "toy" => "Taken from a Toy Master's toy".to_owned(),
        reason => with_note(
            format!("Kyber shards: {}", capitalised(reason)),
            &ledger.note,
        ),
    }
}

/// What came out of a mystery box, in a few words (`docs/mystery-boxes.md`'s outcomes).
fn outcome_words(outcome: &str) -> String {
    use sjk_identity::mystery_box as wire;
    match outcome {
        "empty" => "nothing inside".to_owned(),
        "refund" => "your credits back".to_owned(),
        "double" => "double your credits".to_owned(),
        "jackpot" => "the jackpot".to_owned(),
        wire::EXPLODE => "it blew up".to_owned(),
        wire::HOLOCRON => crate::terms::say("a holocron").into_owned(),
        wire::SHARD_PILE => "kyber shards".to_owned(),
        wire::AGAIN => "a free roll".to_owned(),
        wire::BLADE => "the Mystery blade".to_owned(),
        other => other.replace('_', " "),
    }
}

/// A mystery box in a sentence: "Mystery box: double your credits, paid 100, won 200".
pub(super) fn box_text(opened: &BoxLine) -> String {
    let mut text = format!("Mystery box: {}", outcome_words(&opened.outcome));
    if opened.price > 0 {
        text.push_str(&format!(
            ", paid {}",
            crate::cards::grouped(opened.price.unsigned_abs())
        ));
    } else {
        text.push_str(", free roll");
    }
    if opened.won > 0 {
        text.push_str(&format!(
            ", won {}",
            crate::cards::grouped(opened.won.unsigned_abs())
        ));
    }
    text
}

/// What one side of a trade held, in words.
fn items_text(items: &[sjk_identity::TradeItem]) -> String {
    if items.is_empty() {
        return "nothing".to_owned();
    }
    let names: Vec<String> = items
        .iter()
        .map(|item| {
            if item.is_holocron() || (item.id.is_none() && item.name.is_none()) {
                let name = thing(&item.tier);
                format!("{} {name}", article(&name))
            } else {
                match (&item.name, &item.id) {
                    (Some(name), _) if !name.trim().is_empty() => crate::medals::plain_note(name),
                    (_, Some(id)) => unlock_name(id),
                    _ => thing(&item.tier),
                }
            }
        })
        .collect();
    names.join(", ")
}

/// A finished trade in a sentence: "Traded with Nib: gave a Rare Holocron, got Sun". The
/// name keeps its colour codes, as other players' names show.
pub(super) fn trade_text(trade: &TradeLine) -> String {
    let name = sjk_identity::chat::name_for_display(&trade.with.name);
    let name = if name.trim().is_empty() {
        "another player".to_owned()
    } else {
        format!("{name}^7")
    };
    format!(
        "Traded with {name}: gave {}, got {}",
        items_text(&trade.gave),
        items_text(&trade.got)
    )
}

/// How many of each tier `tiers` holds, the rarest first: "3 Rare, 2 Uncommon".
fn tier_counts<'a>(tiers: impl Iterator<Item = &'a str>) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for tier in tiers {
        match counts.iter_mut().find(|(id, _)| *id == tier) {
            Some((_, count)) => *count += 1,
            None => counts.push((tier, 1)),
        }
    }
    let rank = |id: &str| crate::holocrons::any(id).map_or(0, |tier| tier.slot + 1);
    counts.sort_by_key(|(id, _)| std::cmp::Reverse(rank(id)));
    counts
        .iter()
        .map(|(id, count)| format!("{count} {}", tier_label(id)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The row of a group of lines (two or more, of one kind and second).
pub(super) fn group_row(run: &[&HistoryLine]) -> Row {
    let first = row(run[0]);
    let count = run.len();
    let tiers: Vec<&str> = run
        .iter()
        .map(|line| match line {
            HistoryLine::Came(came) => came.tier.as_str(),
            HistoryLine::Opened(opened) => opened.tier.as_str(),
            _ => "",
        })
        .collect();
    let same = tiers.iter().all(|tier| *tier == tiers[0]);
    // The rarest tier's colour for the chip.
    let rarest = tiers
        .iter()
        .max_by_key(|id| crate::holocrons::any(id).map_or(0, |tier| tier.slot + 1))
        .copied()
        .unwrap_or("");
    let noun = if same {
        things(tiers[0], count)
    } else if tiers
        .iter()
        .all(|id| crate::holocrons::any(id).is_some_and(|tier| tier.is_kyber()))
    {
        format!("{count} Kyber Crystals")
    } else {
        crate::terms::say_owned(format!("{count} Holocrons"))
    };
    let breakdown = if same {
        String::new()
    } else {
        format!(" ({})", tier_counts(tiers.iter().copied()))
    };
    let (text, amounts) = match run[0] {
        HistoryLine::Came(came) => {
            let text = if came.traded {
                format!("Received {noun}{breakdown} in a trade")
            } else {
                match came.source.as_str() {
                    "play" => format!("Found {noun}{breakdown}"),
                    "staff" => with_note(
                        format!("A gift from the SJK team: {noun}{breakdown}"),
                        &came.note,
                    ),
                    "achievement" => format!("Earned {noun}{breakdown} from achievements"),
                    "bought" => format!("Bought {noun}{breakdown}"),
                    "box" => format!("{noun}{breakdown} from mystery boxes"),
                    "wheel" => format!("{noun}{breakdown} from the WHEEL OF DOOM"),
                    "toy" => format!("{noun}{breakdown} the Toy Master's toys threw out"),
                    "reward" => with_note(format!("Reward: {noun}{breakdown}"), &came.note),
                    _ => format!("Received {noun}{breakdown}"),
                }
            };
            let price: i64 = run
                .iter()
                .map(|line| match line {
                    HistoryLine::Came(came) => came.price,
                    _ => 0,
                })
                .sum();
            (text, Amount::of(-price, "credits").into_iter().collect())
        }
        _ => {
            let (mut credits, mut shards, mut cards, mut unlocks) = (0, 0, 0, 0);
            for line in run {
                if let HistoryLine::Opened(opened) = line {
                    credits += opened.credits;
                    shards += opened.shards;
                    cards += usize::from(opened.card.is_some());
                    unlocks += usize::from(!opened.unlock.trim().is_empty());
                }
            }
            let mut parts = Vec::with_capacity(2);
            if cards > 0 {
                parts.push(format!("{cards} card{}", if cards == 1 { "" } else { "s" }));
            }
            if unlocks > 0 {
                parts.push(format!(
                    "{unlocks} unlock{}",
                    if unlocks == 1 { "" } else { "s" }
                ));
            }
            let mut text = format!("Opened {noun}{breakdown}");
            if !parts.is_empty() {
                text.push_str(": ");
                text.push_str(&parts.join(", "));
            }
            (text, Amount::pair(credits, shards))
        }
    };
    Row {
        colour: tier_colour(rarest),
        text,
        amounts,
        ..first
    }
}

impl Panel {
    /// What the History tab reads this frame, and whether it shows; the console sends the
    /// read it asks for ([`Panel::take_history_ask`]).
    pub(super) fn follow_history(&mut self, inputs: &Inputs<'_>) {
        let state = inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.history.clone());
        self.history
            .follow(state, self.open && self.tab == Tab::History);
    }

    /// The read of the history the console should send now: the newest page
    /// (`Some(None)`) or the one before a time.
    pub(crate) fn take_history_ask(&mut self) -> Option<Option<i64>> {
        self.history.take_ask()
    }

    /// The read asked could not go: the identity has not started.
    pub(crate) fn history_not_sent(&mut self) {
        self.history.not_sent();
    }

    /// Show `state` on the History tab in place of the identity's (`None` lets go), for
    /// a world shot or a test.
    #[cfg(test)]
    pub(crate) fn preview_history(&mut self, state: Option<Arc<HistoryState>>) {
        self.history.preview = state;
    }

    /// Open or close row `index` and choose it, for a world shot.
    #[cfg(test)]
    pub(crate) fn history_activate_for_shot(&mut self, index: usize) {
        self.history.activate(index);
    }

    /// A key on the History tab; `None` for one it leaves to the page.
    pub(super) fn history_key(&mut self, key: KeyCode, shift: bool) -> Option<PanelAction> {
        let history = &mut self.history;
        match key {
            KeyCode::ArrowUp => history.step(-1),
            KeyCode::ArrowDown => history.step(1),
            KeyCode::PageUp => history.step(-(ROWS_SHOWN as isize)),
            KeyCode::PageDown => history.step(ROWS_SHOWN as isize),
            KeyCode::Home => history.step(isize::MIN / 2),
            KeyCode::End => history.step(isize::MAX / 2),
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Tab => {
                let back = key == KeyCode::ArrowLeft || (key == KeyCode::Tab && shift);
                let count = Filter::ALL.len();
                let at = history.filter.index();
                let next = if back {
                    (at + count - 1) % count
                } else {
                    (at + 1) % count
                };
                history.set_filter(Filter::ALL[next]);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                let chosen = history.chosen;
                history.activate(chosen);
                history.keep_in_view();
            }
            KeyCode::KeyR => {
                // Read the newest page again.
                history.shown = false;
            }
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// A press (`activate`) or a rest (`resting`) of the pointer on `token` of the History tab; `None` for a token
    /// that is not the tab's.
    pub(super) fn history_token(
        &mut self,
        token: u16,
        activate: bool,
        resting: bool,
    ) -> Option<PanelAction> {
        let history = &mut self.history;
        if let Some(filter) = token
            .checked_sub(FILTER_BASE)
            .and_then(|index| Filter::ALL.get(usize::from(index)))
        {
            if activate {
                history.set_filter(*filter);
            }
            return Some(PanelAction::None);
        }
        if token == MORE_TOKEN {
            if activate {
                history.chosen = history.rows.len();
                history.load_older();
            }
            return Some(PanelAction::None);
        }
        let slot = token
            .checked_sub(ROW_BASE)
            .map(usize::from)
            .filter(|slot| *slot < ROWS_SHOWN)?;
        let index = history.first + slot;
        if index < history.places() {
            if activate {
                history.activate(index);
            } else if resting {
                history.chosen = index;
            }
        }
        Some(PanelAction::None)
    }

    /// The token of the History tab's chosen place, while it shows.
    pub(super) fn history_focus(&self) -> u16 {
        let history = &self.history;
        match history.chosen.checked_sub(history.first) {
            Some(slot) if slot < ROWS_SHOWN && history.chosen < history.places() => {
                if history.more_row(history.chosen) {
                    MORE_TOKEN
                } else {
                    ROW_BASE + slot as u16
                }
            }
            _ => BACK_TOKEN,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use sjk_identity::history::{HistoryCard, HistoryPartner};

    pub(crate) fn opened(at: i64, tier: &str, credits: i64, shards: i64) -> HistoryLine {
        HistoryLine::Opened(OpenedLine {
            at,
            holocron: Some(1),
            tier: tier.into(),
            credits,
            shards,
            card: None,
            unlock: String::new(),
        })
    }

    pub(crate) fn came(at: i64, tier: &str, source: &str) -> HistoryLine {
        HistoryLine::Came(Came {
            at,
            holocron: Some(1),
            tier: tier.into(),
            source: source.into(),
            note: String::new(),
            price: 0,
            traded: false,
        })
    }

    pub(crate) fn ledger(at: i64, delta: i64, reason: &str, note: &str) -> Ledger {
        Ledger {
            at,
            delta,
            reason: reason.into(),
            note: note.into(),
        }
    }

    #[test]
    fn a_bought_holocron_shows_its_price() {
        let HistoryLine::Came(mut bought) = came(0, "rare", "bought") else {
            unreachable!()
        };
        bought.price = 400;
        let line = HistoryLine::Came(bought);
        let shown = row(&line);
        assert_eq!(shown.text, "Bought a Rare Holocron");
        assert_eq!(shown.amounts[0].text, "-400 credits");
        assert!(!shown.amounts[0].gain);
        assert!(Filter::Credits.passes(&line));
    }

    #[test]
    fn sentences_say_what_happened() {
        assert_eq!(
            came_text(&Came {
                tier: "rare".into(),
                source: "play".into(),
                ..Came::default()
            }),
            "Found a Rare Holocron"
        );
        assert_eq!(
            came_text(&Came {
                tier: "uncommon".into(),
                source: "staff".into(),
                note: "^1Thanks^7 for testing".into(),
                ..Came::default()
            }),
            "A gift from the SJK team: an Uncommon Holocron - Thanks for testing"
        );
        assert_eq!(
            came_text(&Came {
                tier: "kyber_rare".into(),
                source: "achievement".into(),
                ..Came::default()
            }),
            "Earned a Rare Kyber Crystal from an achievement"
        );
        assert_eq!(
            came_text(&Came {
                tier: "legendary".into(),
                source: "box".into(),
                ..Came::default()
            }),
            "A Legendary Holocron from a mystery box"
        );
        assert_eq!(
            came_text(&Came {
                tier: "mythical".into(),
                source: "wheel".into(),
                ..Came::default()
            }),
            "A Mythical Holocron from the WHEEL OF DOOM"
        );
        // A Toy Master's drop names no toy, whatever the hub noted.
        assert_eq!(
            came_text(&Came {
                tier: "uncommon".into(),
                source: "toy".into(),
                note: "kind".into(),
                ..Came::default()
            }),
            "An Uncommon Holocron a Toy Master's toy threw out"
        );
        assert_eq!(
            credits_text(&ledger(0, 25, "toy", "kind")),
            "Taken from a Toy Master's toy"
        );
        assert_eq!(
            shards_text(&ledger(0, 5, "toy", "kind")),
            "Taken from a Toy Master's toy"
        );
        assert_eq!(
            came_text(&Came {
                tier: "common".into(),
                source: "someday".into(),
                ..Came::default()
            }),
            "Received a Common Holocron"
        );
        let mut open = OpenedLine {
            tier: "rare".into(),
            credits: 120,
            shards: 8,
            card: Some(HistoryCard {
                card: None,
                rarity: "uncommon".into(),
                finish: "holo".into(),
            }),
            unlock: "saber_sun".into(),
            ..OpenedLine::default()
        };
        let sun = crate::unlockables::find("saber_sun").unwrap().name;
        assert_eq!(
            opened_text(&open),
            format!("Opened a Rare Holocron: an Uncommon card, sealed (Holo), {sun}")
        );
        open.card = None;
        open.unlock.clear();
        assert_eq!(opened_text(&open), "Opened a Rare Holocron");
        assert_eq!(
            row(&HistoryLine::Opened(open)).amounts,
            [
                Amount {
                    text: "+120 credits".into(),
                    gain: true
                },
                Amount {
                    text: "+8 shards".into(),
                    gain: true
                }
            ]
        );
        assert_eq!(
            credits_text(&ledger(0, -400, "holocron_buy", "")),
            "Bought a holocron"
        );
        let bought = row(&HistoryLine::Credits(ledger(0, -1_400, "holocron_buy", "")));
        assert_eq!(bought.amounts[0].text, "-1 400 credits");
        assert!(!bought.amounts[0].gain);
        assert_eq!(
            credits_text(&ledger(0, 50, "reward", "Bug hunting")),
            "Contributor reward - Bug hunting"
        );
        assert_eq!(
            credits_text(&ledger(0, 100, "mystery_box", "explode_gift")),
            "Another player's mystery box blew up: what they paid came to you"
        );
        assert_eq!(
            credits_text(&ledger(0, -100, "wheel_of_doom", "")),
            "Spun the WHEEL OF DOOM"
        );
        assert_eq!(
            credits_text(&ledger(0, 300, "box_hunt", "survived")),
            "Box hunt: you outlasted every hunter"
        );
        assert_eq!(
            shards_text(&ledger(0, 15, "shatter", "saber_sun")),
            format!("Shattered {sun}")
        );
        assert_eq!(
            shards_text(&ledger(0, -60, "craft", "saber_unknown_one")),
            "Crafted saber_unknown_one"
        );
        assert_eq!(
            box_text(&BoxLine {
                outcome: "double".into(),
                price: 100,
                won: 200,
                ..BoxLine::default()
            }),
            "Mystery box: double your credits, paid 100, won 200"
        );
        assert_eq!(
            box_text(&BoxLine {
                outcome: "explode".into(),
                ..BoxLine::default()
            }),
            "Mystery box: it blew up, free roll"
        );
        let trade = TradeLine {
            with: HistoryPartner {
                name: "^1Nib".into(),
                verified: true,
            },
            gave: vec![sjk_identity::TradeItem {
                kind: "holocron".into(),
                holocron: Some(4),
                tier: "rare".into(),
                ..Default::default()
            }],
            got: vec![sjk_identity::TradeItem {
                kind: "shader".into(),
                id: Some("saber_sun".into()),
                tier: "mythical".into(),
                ..Default::default()
            }],
            ..TradeLine::default()
        };
        assert_eq!(
            trade_text(&trade),
            format!("Traded with ^1Nib^7: gave a Rare Holocron, got {sun}")
        );
    }

    #[test]
    fn an_open_all_is_one_row_that_opens_on_its_lines() {
        let lines = vec![
            opened(50, "rare", 10, 0),
            opened(50, "rare", 20, 4),
            opened(50, "uncommon", 5, 0),
            opened(40, "rare", 1, 0),
            came(30, "rare", "play"),
            came(30, "uncommon", "play"),
            came(30, "rare", "staff"),
            HistoryLine::Credits(ledger(20, -400, "holocron_buy", "")),
        ];
        let rows = rows(&lines, Filter::All, &BTreeSet::new());
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].text, "Opened 3 Holocrons (2 Rare, 1 Uncommon)");
        assert_eq!(rows[0].amounts[0].text, "+35 credits");
        assert_eq!(rows[0].amounts[1].text, "+4 shards");
        assert_eq!(rows[0].group.as_ref().map(|group| group.2), Some(3));
        assert_eq!(rows[1].text, "Opened a Rare Holocron");
        assert!(rows[1].group.is_none());
        assert_eq!(rows[2].text, "Found 2 Holocrons (1 Rare, 1 Uncommon)");
        assert_eq!(rows[3].text, "A gift from the SJK team: a Rare Holocron");
        // Opened, the group's lines follow it.
        let key = rows[0].group.clone().unwrap().0;
        let open = BTreeSet::from([key]);
        let rows = super::rows(&lines, Filter::All, &open);
        assert_eq!(rows.len(), 8);
        assert!(rows[1].inner && rows[1].when.is_empty());
        assert_eq!(rows[1].amounts[0].text, "+10 credits");
        // Of one tier, the group says it.
        let same = vec![opened(9, "legendary", 0, 0), opened(9, "legendary", 0, 0)];
        assert_eq!(
            super::rows(&same, Filter::All, &BTreeSet::new())[0].text,
            "Opened 2 Legendary Holocrons"
        );
    }

    #[test]
    fn the_filter_keeps_what_it_names() {
        let lines = vec![
            opened(50, "rare", 10, 0),
            opened(49, "rare", 0, 3),
            came(30, "rare", "play"),
            HistoryLine::Credits(ledger(20, -400, "holocron_buy", "")),
            HistoryLine::Shards(ledger(19, 15, "shatter", "saber_sun")),
            HistoryLine::Box(BoxLine {
                at: 18,
                outcome: "empty".into(),
                price: 100,
                ..BoxLine::default()
            }),
            HistoryLine::Trade(TradeLine::default()),
        ];
        let count = |filter| rows(&lines, filter, &BTreeSet::new()).len();
        assert_eq!(count(Filter::All), 7);
        assert_eq!(count(Filter::Drops), 1);
        assert_eq!(count(Filter::Opened), 2);
        assert_eq!(count(Filter::Credits), 3);
        assert_eq!(count(Filter::Shards), 2);
        assert_eq!(count(Filter::Trades), 1);
        assert_eq!(count(Filter::Boxes), 1);
    }

    #[test]
    fn showing_the_tab_reads_the_newest_page_and_load_older_the_next() {
        let mut history = History::default();
        let state = Arc::new(HistoryState {
            serial: 1,
            lines: (0..20).map(|at| opened(100 - at, "rare", 1, 0)).collect(),
            more: true,
            before: 81,
            message: String::new(),
        });
        history.follow(Some(state.clone()), true);
        assert_eq!(history.take_ask(), Some(None));
        assert!(history.loading());
        history.follow(Some(state.clone()), true);
        assert_eq!(history.take_ask(), None, "asked once");
        assert!(history.loading(), "until the answer");
        let answered = Arc::new(HistoryState {
            serial: 2,
            ..(*state).clone()
        });
        history.follow(Some(answered.clone()), true);
        assert!(!history.loading());
        assert_eq!(history.rows.len(), 20);
        assert_eq!(history.places(), 21);
        // Load older, from the end of the list.
        history.step(100);
        assert!(history.more_row(history.chosen));
        assert_eq!(history.first, 21 - ROWS_SHOWN);
        history.activate(history.chosen);
        assert_eq!(history.take_ask(), Some(Some(81)));
        // Hidden and shown again, the newest page is read again.
        history.follow(Some(answered.clone()), false);
        history.follow(Some(answered), true);
        assert_eq!(history.take_ask(), Some(None));
    }
}
