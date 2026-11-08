//! The Collection's Tags tab (27/10/2026): the tags a player sticks on walls with
//! `toy_tag` (`tags.rs`). Every tag of the catalogue stands in a wall of tiles on the
//! left, its picture over its name and its tier's colour, the ones the player lacks
//! dimmed and the one `cg_tag` names marked; the tag chosen stands large on a card on the
//! right, with how it is got and its Choose button, which sets `cg_tag`.
//!
//! Since 31/10/2026 (Sol's decisions) the wall scrolls ([`Line`]: the wheel, the arrows
//! keeping the choice in view, a bar at its side), the credits cards the hub lets be tags
//! follow the tags under their own heading ([`crate::tags::tag_cards`]), and the card
//! offers the finishes, Normal, Holo and Solar: one the player holds a copy in can be
//! chosen, and `cg_tagFinish` keeps it. A tile shows the Holo and Solar copies held.
//!
//! Since 08/11/2026 (Sol) the tags stand under a heading for each of their collections
//! ([`crate::tags::COLLECTIONS`]: Alpha, then Community), the card naming the chosen
//! tag's.
//!
//! Since 03/11/2026 (Blep's idea, `crate::tag_copies`), on a hub that says `tag_copies`,
//! the card of a dropped tag held twice or more offers Break down (one copy of the finish
//! shown into kyber shards) and Craft (two spare copies of its tier and finish for a tag
//! not held yet), each on a second press, with the hub's answer under them.

use super::view::LEFT_X;
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::tags::Held;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The wall: its columns, a tile's size and the room between tiles, its top and foot,
/// and a heading's height.
const COLUMNS: usize = 5;
const TILE_WIDTH: f32 = 168.0;
const TILE_HEIGHT: f32 = 196.0;
const TILE_GAP: f32 = 18.0;
const WALL_Y: f32 = 304.0;
const WALL_BOTTOM: f32 = 990.0;
const HEAD_HEIGHT: f32 = 44.0;
/// The chosen tag's card: its left edge, width, top and foot, and the room inside it.
const CARD_X: f32 = 1_180.0;
const CARD_WIDTH: f32 = super::view::RIGHT_X - CARD_X;
const CARD_TOP: f32 = 304.0;
const CARD_BOTTOM: f32 = 990.0;
const CARD_PAD: f32 = 28.0;
/// Where new tags come from: proposals on SJK's Discord, which Sol picks from
/// (`docs/client.md`, "Tag").
const PROPOSE: &str = "New tag ideas: SJK's Discord, #tag-proposals";
/// Most tags and cards the wall lists: their tokens' room.
pub(super) const MAX_ENTRIES: usize = 100;
const _: () = assert!(TAG_BASE as usize + MAX_ENTRIES <= 8_200);
const _: () = assert!(crate::tags::CATALOGUE.len() < MAX_ENTRIES);
/// The heading of the credits cards' part of the wall (the tags' are their collections').
const CARDS_HEAD: &str = "Credits cards";

/// A tag copies button of the card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Tool {
    Break,
    Craft,
}

/// A button pressed once, waiting for its second press: for this tag and finish.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Confirm {
    tool: Tool,
    id: String,
    finish: &'static str,
}

/// What the card's tag copies buttons do for the entry and finish shown, as the last
/// frame worked out (`None`: the button is not offered).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Tools {
    /// The hub takes tag copies requests and the player may act.
    pub(super) on: bool,
    /// The copy Break down gives and the shards it brings.
    pub(super) shatter: Option<(sjk_identity::TagCopy, u64)>,
    /// The two copies Craft gives, when a craft could bring a new tag.
    pub(super) craft: Option<[sjk_identity::TagCopy; 2]>,
    /// Why Craft is not offered though the tag takes part.
    pub(super) craft_why: Option<&'static str>,
    /// The last tag copies answer, in words, and whether it worked.
    pub(super) said: Option<(String, bool)>,
}

impl Tools {
    /// The tools for tag `tag` in `finish`, from the snapshot.
    fn of(
        tag: &crate::tags::Tag,
        finish: &'static str,
        unlocks: Option<&[sjk_identity::Unlock]>,
        snapshot: Option<&sjk_identity::Snapshot>,
    ) -> Self {
        let mut tools = Self {
            said: snapshot
                .and_then(|snapshot| snapshot.tag_trade.as_ref())
                .map(|outcome| (crate::tag_copies::outcome_words(outcome), outcome.done())),
            ..Self::default()
        };
        let (Some(unlocks), Some(snapshot)) = (unlocks, snapshot) else {
            return tools;
        };
        let economy = snapshot.economy.as_ref();
        if !crate::tag_copies::on(economy)
            || !crate::economy::may_act(snapshot)
            || !crate::tag_copies::takes_part(tag)
        {
            return tools;
        }
        tools.on = true;
        let held = Held::of_parts(tag.id, Some(unlocks), None);
        if crate::tag_copies::spare(held, finish) > 0 {
            let shards = economy
                .and_then(|economy| economy.tags.as_ref())
                .and_then(|tables| crate::tag_copies::shatter_yield(tables, tag, finish))
                .unwrap_or(0);
            tools.shatter = Some((sjk_identity::TagCopy::new(tag.id, finish), shards));
        }
        match crate::tag_copies::craft_inputs(unlocks, tag, finish) {
            Some(_) if !crate::tag_copies::something_new(unlocks, tag, finish) => {
                tools.craft_why = Some("You hold every tag of this tier in this finish.");
            }
            Some(inputs) => tools.craft = Some(inputs),
            None if held.copies >= 2 => {
                tools.craft_why = Some("Craft needs two spare copies of this tier in this finish.");
            }
            None => {}
        }
        tools
    }
}

/// One thing the wall lists: a tag of the catalogue, or a card the hub lets be a tag.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Entry {
    Tag(&'static crate::tags::Tag),
    Card(String),
}

impl Entry {
    /// Its id, as `cg_tag` and a look name it.
    pub(super) fn id(&self) -> &str {
        match self {
            Self::Tag(tag) => tag.id,
            Self::Card(id) => id,
        }
    }
}

/// One line of the wall: a heading, or a row of the entries `start..end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Line {
    Head(&'static str),
    Row(usize, usize),
}

impl Line {
    fn height(self) -> f32 {
        match self {
            Self::Head(_) => HEAD_HEIGHT,
            Self::Row(..) => TILE_HEIGHT + TILE_GAP,
        }
    }
}

/// The wall's lines for its parts in order, each a heading and how many entries follow
/// it (a part of none is left out).
pub(super) fn lines(parts: &[(&'static str, usize)]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    for &(head, count) in parts {
        if count == 0 {
            continue;
        }
        lines.push(Line::Head(head));
        let mut at = start;
        start += count;
        while at < start {
            let end = (at + COLUMNS).min(start);
            lines.push(Line::Row(at, end));
            at = end;
        }
    }
    lines
}

/// How many lines from `first` fit the wall (at least one).
fn shown_from(lines: &[Line], first: usize) -> usize {
    let mut y = WALL_Y;
    let mut shown = 0;
    for line in lines.iter().skip(first) {
        // A row's gap below it may hang past the foot.
        let needs = match line {
            Line::Head(_) => HEAD_HEIGHT,
            Line::Row(..) => TILE_HEIGHT,
        };
        if y + needs > WALL_BOTTOM && shown > 0 {
            break;
        }
        y += line.height();
        shown += 1;
    }
    shown
}

/// The last line the wall may start from: from it, every line to the end fits.
fn last_first(lines: &[Line]) -> usize {
    (0..lines.len())
        .find(|first| first + shown_from(lines, *first) >= lines.len())
        .unwrap_or(0)
}

/// The line holding entry `index`.
fn line_of(lines: &[Line], index: usize) -> Option<usize> {
    lines
        .iter()
        .position(|line| matches!(line, Line::Row(start, end) if (*start..*end).contains(&index)))
}

/// The first line that keeps line `at` in view, scrolling as little as possible from
/// `first`: a row right under a heading brings the heading too.
fn first_showing(lines: &[Line], first: usize, at: usize) -> usize {
    let top = if at > 0 && matches!(lines[at - 1], Line::Head(_)) {
        at - 1
    } else {
        at
    };
    if top < first {
        return top;
    }
    let mut first = first;
    while first < at && first + shown_from(lines, first) <= at {
        first += 1;
    }
    first.min(last_first(lines))
}

/// The entry Up (`down` false) or Down takes `index` to: the line above or below, the
/// same column or the nearest, past the headings; `None` at the ends.
fn vertical(lines: &[Line], index: usize, down: bool) -> Option<usize> {
    let at = line_of(lines, index)?;
    let Line::Row(start, _) = lines[at] else {
        return None;
    };
    let column = index - start;
    let mut next = at;
    loop {
        next = if down { next + 1 } else { next.checked_sub(1)? };
        match lines.get(next)? {
            Line::Row(start, end) => return Some((start + column).min(end - 1)),
            Line::Head(_) => {}
        }
    }
}

impl Panel {
    /// The entry chosen and what the player holds of it, as the last frame showed them.
    fn tag_chosen(&self) -> Option<(&Entry, Held)> {
        let entry = self.tag_entries.get(self.tag)?;
        Some((
            entry,
            self.tag_held.get(self.tag).copied().unwrap_or_default(),
        ))
    }

    /// The finish the card shows for the chosen entry: the one chosen on it, when held.
    fn tag_finish_shown(&self, held: Held) -> &'static crate::cards::Finish {
        crate::cards::FINISHES
            .get(self.tag_finish)
            .filter(|finish| held.has(finish))
            .unwrap_or(&crate::cards::FINISHES[0])
    }

    /// Whether Enter (or Choose) picks the entry chosen: one the player holds, not
    /// already the one `cg_tag` names in the finish shown.
    pub(super) fn tag_choosable(&self, inputs: &Inputs<'_>) -> bool {
        self.tag_chosen().is_some_and(|(entry, held)| {
            let finish = self.tag_finish_shown(held);
            held.copies > 0
                && !(entry.id() == picked(inputs) && finish.id == picked_finish(inputs, held))
        })
    }

    /// Enter, or the card's Choose button: pick the entry chosen, in the finish shown, for
    /// `toy_tag`. One the player lacks is not picked; `toy_tag` would refuse it.
    pub(super) fn choose_tag(&self) -> PanelAction {
        match self.tag_chosen() {
            Some((entry, held)) if held.copies > 0 => {
                PanelAction::ChooseTag(entry.id().to_owned(), self.tag_finish_shown(held).id)
            }
            _ => PanelAction::None,
        }
    }

    /// A finish chip of the card: shown on it when held, and picked at once when the
    /// entry is the one in use (Choose picks it otherwise).
    pub(super) fn choose_tag_finish(&mut self, finish: usize) -> PanelAction {
        let Some((entry, held)) = self.tag_chosen() else {
            return PanelAction::None;
        };
        let Some(wanted) = crate::cards::FINISHES.get(finish).filter(|f| held.has(f)) else {
            return PanelAction::None;
        };
        let in_use = self.tag_in_use.as_deref() == Some(entry.id());
        let id = entry.id().to_owned();
        self.tag_finish = finish;
        if in_use {
            PanelAction::ChooseTag(id, wanted.id)
        } else {
            PanelAction::None
        }
    }

    /// Choose entry `id` with finish `finish` shown, the wall brought to it at the next
    /// frame, for a world shot.
    #[cfg(test)]
    pub(crate) fn choose_tag_for_shot(&mut self, id: &str, finish: usize) {
        if let Some(index) = self.tag_entries.iter().position(|entry| entry.id() == id) {
            self.tag = index;
            self.tag_finish = finish;
            self.tag_finish_for = Some(id.to_owned());
            self.tag_follow = true;
        }
    }

    /// Break down or Craft on the card: the first press asks, the second sends.
    pub(super) fn press_tag_tool(&mut self, tool: Tool) -> PanelAction {
        let Some((entry, held)) = self.tag_chosen() else {
            return PanelAction::None;
        };
        let id = entry.id().to_owned();
        let finish = self.tag_finish_shown(held).id;
        let wanted = Confirm { tool, id, finish };
        let action = match tool {
            Tool::Break => self
                .tag_tools
                .shatter
                .as_ref()
                .map(|(copy, _)| PanelAction::ShatterTag(copy.clone())),
            Tool::Craft => self.tag_tools.craft.clone().map(PanelAction::CraftTags),
        };
        let Some(action) = action else {
            return PanelAction::None;
        };
        if self.tag_confirm.as_ref() == Some(&wanted) {
            self.tag_confirm = None;
            action
        } else {
            self.tag_confirm = Some(wanted);
            PanelAction::None
        }
    }

    /// The wheel over the Tags tab: the wall a line up or down.
    pub(super) fn scroll_tags(&mut self, up: bool) {
        let last = last_first(&self.tag_lines);
        self.tag_first = if up {
            self.tag_first.saturating_sub(1)
        } else {
            (self.tag_first + 1).min(last)
        };
    }

    /// The arrows, Page Up and Down, Home and End on the wall; false for other keys.
    /// The choice is brought into view at the next frame.
    pub(super) fn tag_key(&mut self, key: KeyCode) -> bool {
        let count = self.tag_entries.len();
        if count == 0 {
            return false;
        }
        let at = self.tag.min(count - 1);
        let page = COLUMNS * 3;
        self.tag = match key {
            KeyCode::ArrowLeft => at.saturating_sub(1),
            KeyCode::ArrowRight => (at + 1).min(count - 1),
            KeyCode::ArrowUp => vertical(&self.tag_lines, at, false).unwrap_or(at),
            KeyCode::ArrowDown => vertical(&self.tag_lines, at, true).unwrap_or(at),
            KeyCode::PageUp => at.saturating_sub(page),
            KeyCode::PageDown => (at + page).min(count - 1),
            KeyCode::Home => 0,
            KeyCode::End => count - 1,
            _ => return false,
        };
        self.tag_follow = true;
        true
    }

    /// The Tags tab.
    pub(super) fn tags(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let (unlocks, me) = match inputs.holdings {
            Holdings::Known(unlocks) => (
                Some(unlocks),
                inputs.snapshot.and_then(|snapshot| snapshot.me.as_ref()),
            ),
            _ => (None, None),
        };
        let cards = crate::tags::tag_cards(me);
        let room = MAX_ENTRIES - crate::tags::CATALOGUE.len();
        let parts = wall_parts(cards.len().min(room));
        self.tag_entries = crate::tags::COLLECTIONS
            .iter()
            .flat_map(|collection| {
                crate::tags::CATALOGUE
                    .iter()
                    .filter(|tag| tag.collection == collection.id)
            })
            .map(Entry::Tag)
            .chain(cards.into_iter().take(room).map(Entry::Card))
            .collect();
        self.tag_held = self
            .tag_entries
            .iter()
            .map(|entry| Held::of_parts(entry.id(), unlocks, me))
            .collect();
        let total = self.tag_entries.len();
        let tags = crate::tags::CATALOGUE.len();
        self.tag_lines = lines(&parts);
        let owned = |range: std::ops::Range<usize>| {
            self.tag_held[range]
                .iter()
                .filter(|held| held.copies > 0)
                .count()
        };
        let (tags_owned, cards_owned) = (owned(0..tags), owned(tags..total));
        let held = if total > tags {
            format!(
                "{tags_owned} of {tags} tags and {cards_owned} of {} credits cards held",
                total - tags
            )
        } else {
            format!("{tags_owned} of {tags} held")
        };
        let line = match inputs.holdings.reason() {
            Some(reason) => reason.to_owned(),
            None => format!(
                "{held}. Holocrons drop more, now and then Holo or Solar; stick yours with \
                 toy_tag (the quick wheel's Toys, or a key)."
            ),
        };
        let heading = if total > tags {
            format!("{tags} tags, {} cards", total - tags)
        } else {
            format!("{total} tags")
        };
        self.lead(frame, &heading, &line);
        self.tag = self.tag.min(total - 1);
        let in_use = picked(inputs).to_owned();
        let in_use_finish = self
            .tag_entries
            .iter()
            .position(|entry| entry.id() == in_use)
            .map_or("normal", |index| {
                picked_finish(inputs, self.tag_held[index])
            });
        // A new choice shows its finish in use, or Normal.
        let chosen_id = self.tag_entries[self.tag].id().to_owned();
        if self.tag_finish_for.as_deref() != Some(chosen_id.as_str()) {
            self.tag_finish = if chosen_id == in_use {
                crate::cards::Finish::from_id(in_use_finish).map_or(0, |finish| finish.index)
            } else {
                0
            };
            self.tag_finish_for = Some(chosen_id);
        }
        self.tag_in_use = Some(in_use.clone());
        // The wall.
        let last = last_first(&self.tag_lines);
        self.tag_first = self.tag_first.min(last);
        if std::mem::take(&mut self.tag_follow)
            && let Some(at) = line_of(&self.tag_lines, self.tag)
        {
            self.tag_first = first_showing(&self.tag_lines, self.tag_first, at);
        }
        let shown = shown_from(&self.tag_lines, self.tag_first);
        let mut y = WALL_Y;
        for at in self.tag_first..self.tag_first + shown {
            let line = self.tag_lines[at];
            match line {
                Line::Head(words) => self.tag_heading(frame, words, y),
                Line::Row(start, end) => {
                    for index in start..end {
                        let x = LEFT_X + (index - start) as f32 * (TILE_WIDTH + TILE_GAP);
                        let entry = self.tag_entries[index].clone();
                        let held = self.tag_held[index];
                        let use_finish = (entry.id() == in_use).then_some(in_use_finish);
                        self.tag_tile(frame, index, &entry, [x, y], held, use_finish);
                    }
                }
            }
            y += line.height();
        }
        self.tag_scrollbar(frame, shown);
        let held = self.tag_held[self.tag];
        let entry = self.tag_entries[self.tag].clone();
        let in_use_here = (entry.id() == in_use).then_some(in_use_finish);
        let finish = self.tag_finish_shown(held).id;
        self.tag_tools = match &entry {
            Entry::Tag(tag) => Tools::of(tag, finish, unlocks, inputs.snapshot),
            Entry::Card(_) => Tools::default(),
        };
        // A press waits only for the tag and finish it was for.
        if self
            .tag_confirm
            .as_ref()
            .is_some_and(|confirm| confirm.id != entry.id() || confirm.finish != finish)
        {
            self.tag_confirm = None;
        }
        self.tag_card(frame, &entry, held, in_use_here);
    }

    /// The card's tag copies row at `y` (03/11/2026): Break down and Craft, the second
    /// press asked for, and the hub's last answer or why Craft waits.
    fn tag_tools_row(&mut self, frame: &Frame, x: f32, y: f32, width: f32) {
        let s = frame.s;
        let tools = self.tag_tools.clone();
        if !tools.on {
            return;
        }
        let confirm = self.tag_confirm.as_ref().map(|confirm| confirm.tool);
        let button = (width - 12.0) * 0.5;
        let break_label = match (&tools.shatter, confirm) {
            (Some((_, shards)), Some(Tool::Break)) => format!("Again: +{shards} shards"),
            (Some((_, shards)), _) => format!("Break down  +{shards}"),
            (None, _) => "Break down".to_owned(),
        };
        kit::button(
            &mut self.ui,
            frame,
            [x, y, button, 40.0],
            &break_label,
            false,
            tools.shatter.is_some(),
            confirm == Some(Tool::Break),
            TAG_BREAK_TOKEN,
        );
        let craft_label = if confirm == Some(Tool::Craft) {
            "Again to craft"
        } else {
            "Craft: 2 for 1 new"
        };
        kit::button(
            &mut self.ui,
            frame,
            [x + button + 12.0, y, button, 40.0],
            craft_label,
            false,
            tools.craft.is_some(),
            confirm == Some(Tool::Craft),
            TAG_CRAFT_TOKEN,
        );
        let (words, colour) = match (confirm, &tools.craft, &tools.said) {
            (Some(Tool::Craft), Some(copies), _) => (
                format!(
                    "Gives {} and {}: a tag of this tier you lack, in this finish.",
                    crate::tag_copies::copy_name(&copies[0]),
                    crate::tag_copies::copy_name(&copies[1])
                ),
                color::GOLD_BRIGHT,
            ),
            (Some(Tool::Break), _, _) => (
                "One copy goes; you keep at least one of every tag.".to_owned(),
                color::GOLD_BRIGHT,
            ),
            (_, _, Some((said, true))) => (said.clone(), color::TEXT),
            (_, _, Some((said, false))) => (said.clone(), color::alpha(color::GOLD, 0.9)),
            _ => match tools.craft_why {
                Some(why) => (why.to_owned(), color::QUIET),
                None if tools.shatter.is_none() => (
                    "Duplicates break down into kyber shards or craft into a new tag.".to_owned(),
                    color::QUIET,
                ),
                None => (String::new(), color::QUIET),
            },
        };
        if !words.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(x, y + 46.0, width, 22.0),
                14.5 * s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// A heading of the wall at `y`, with a rule after it.
    fn tag_heading(&mut self, frame: &Frame, words: &str, y: f32) {
        let width = COLUMNS as f32 * (TILE_WIDTH + TILE_GAP) - TILE_GAP;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{words}"),
            frame.rect(LEFT_X, y + 6.0, 260.0, 26.0),
            20.0 * frame.s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(LEFT_X + 260.0, y + 19.0, width - 260.0, 1.0),
            radius: 0.5 * frame.s,
            color: color::alpha(color::HOLO, 0.18),
        });
    }

    /// The bar at the wall's side, when it scrolls: its track and how much of the wall
    /// shows, where.
    fn tag_scrollbar(&mut self, frame: &Frame, shown: usize) {
        let lines = &self.tag_lines;
        if shown >= lines.len() {
            return;
        }
        let height = |range: std::ops::Range<usize>| -> f32 {
            lines[range].iter().map(|line| line.height()).sum()
        };
        let whole = height(0..lines.len()).max(1.0);
        let track = WALL_BOTTOM - WALL_Y;
        let top = WALL_Y + track * height(0..self.tag_first) / whole;
        let size = (track * height(self.tag_first..self.tag_first + shown) / whole).max(40.0);
        let x = LEFT_X + COLUMNS as f32 * (TILE_WIDTH + TILE_GAP) - TILE_GAP + 10.0;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, WALL_Y, 4.0, track),
            radius: 2.0 * frame.s,
            color: color::alpha(color::HOLO, 0.1),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top.min(WALL_BOTTOM - size), 4.0, size),
            radius: 2.0 * frame.s,
            color: color::alpha(color::HOLO, 0.45),
        });
    }

    /// Entry `index`'s tile at `[x, y]`: its picture (its name while the pack is not
    /// here), its name and tier (a card's colour and kind); dimmed when not held, "x2" over
    /// a duplicate, framed gold when chosen here, and marked when it is the one `toy_tag`
    /// sticks (`in_use`, with its finish). Its Holo and Solar copies show as badges.
    fn tag_tile(
        &mut self,
        frame: &Frame,
        index: usize,
        entry: &Entry,
        [x, y]: [f32; 2],
        held: Held,
        in_use: Option<&str>,
    ) {
        let s = frame.s;
        let token = TAG_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        let chosen = index == self.tag;
        let count = held.copies;
        let (name, tone_name, tone) = entry_words(entry);
        let area = frame.rect(x, y, TILE_WIDTH, TILE_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 12.0 * s,
            color: if chosen {
                color::alpha(color::GOLD, 0.09)
            } else {
                color::alpha(color::SPACE, if hovered { 0.78 } else { 0.6 })
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 16.0, y, TILE_WIDTH - 32.0, 3.0),
            radius: 1.5 * s,
            color: Color::new(tone.r, tone.g, tone.b, if count > 0 { 1.0 } else { 0.4 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 12.0 * s,
            width: if chosen { 1.6 } else { 1.0 } * s,
            color: if chosen {
                color::GOLD_BRIGHT
            } else {
                color::alpha(color::HOLO, if hovered { 0.35 } else { 0.16 })
            },
        });
        let art = 120.0;
        let art_rect = [x + (TILE_WIDTH - art) * 0.5, y + 16.0, art, art];
        let alpha = if count > 0 { 1.0 } else { 0.32 };
        self.tag_picture(frame, entry, art_rect, alpha, 15.0);
        // The finished copies held: a badge for each finish, top right.
        let mut badge_y = y + 10.0;
        for (copies, finish) in [(held.holo, 1), (held.solar, 2)] {
            if copies == 0 {
                continue;
            }
            let label = crate::cards::FINISHES[finish].name;
            finish_badge(&mut self.ui, frame, [x + TILE_WIDTH - 62.0, badge_y], label);
            badge_y += 24.0;
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x + 8.0, y + 142.0, TILE_WIDTH - 16.0, 24.0),
            17.0 * s,
            if chosen {
                color::GOLD_BRIGHT
            } else if count > 0 {
                color::TEXT
            } else {
                color::QUIET
            },
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let status = match in_use {
            Some("normal") => "In use".to_owned(),
            Some(finish) => format!(
                "In use  ·  {}",
                crate::cards::Finish::from_id(finish).map_or("Normal", |finish| finish.name)
            ),
            None if count == 0 => "Locked".to_owned(),
            None if count > 1 => format!("{tone_name}  ·  x{count}"),
            None => tone_name.to_owned(),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(x + 8.0, y + 168.0, TILE_WIDTH - 16.0, 20.0),
            14.0 * s,
            if in_use.is_some() {
                color::HOLO
            } else {
                Color::new(tone.r, tone.g, tone.b, if count > 0 { 0.9 } else { 0.5 })
            },
            FontWeight::Regular,
            TextAlign::Center,
        );
        self.ui
            .hit_region(token, frame.rect(x, y, TILE_WIDTH, TILE_HEIGHT));
    }

    /// Entry `entry`'s picture over `[x, y, width, height]` at `alpha`: a tag's from the
    /// tags' pack, a card's from its card pack (in the 6:5 window, centred); its name in
    /// its colour (`size`) while the pack has not brought it.
    fn tag_picture(
        &mut self,
        frame: &Frame,
        entry: &Entry,
        [x, y, width, height]: [f32; 4],
        alpha: f32,
        size: f32,
    ) {
        let texture = match entry {
            Entry::Tag(tag) => crate::tag_icons::texture(tag.id).map(|texture| (texture, 1.0)),
            Entry::Card(id) => crate::cards::faces::current().find(id).map(|(index, _)| {
                (
                    crate::cards::faces::texture(index),
                    crate::cards::faces::ART_ASPECT,
                )
            }),
        };
        match texture {
            Some((texture, aspect)) => {
                // A wide picture keeps its shape, centred in the square.
                let picture_height = height / aspect;
                let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: frame.rect(
                        x,
                        y + (height - picture_height) * 0.5,
                        width,
                        picture_height,
                    ),
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, alpha),
                });
            }
            None => {
                let (name, _, tone) = entry_words(entry);
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(x, y, width, height),
                    radius: 10.0 * frame.s,
                    width: 1.2 * frame.s,
                    color: Color::new(tone.r, tone.g, tone.b, 0.5 * alpha),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{name}"),
                    frame.rect(x, y + height * 0.5 - size * 0.7, width, size * 1.4),
                    size * frame.s,
                    Color::new(tone.r, tone.g, tone.b, alpha),
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
    }

    /// The chosen entry on its card: large, in the finish shown, its name, tier (or a
    /// card's kind and colour) and how it is got, its finishes and Choose (`held`: what the
    /// player holds of it; `in_use`: the finish `toy_tag` sticks it in, when it is the
    /// one in use).
    fn tag_card(&mut self, frame: &Frame, entry: &Entry, held: Held, in_use: Option<&str>) {
        let s = frame.s;
        let count = held.copies;
        let (name, tone_name, tone) = entry_words(entry);
        let finish = self.tag_finish_shown(held);
        let height = CARD_BOTTOM - CARD_TOP;
        let area = frame.rect(CARD_X, CARD_TOP, CARD_WIDTH, height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 16.0 * s,
            color: color::alpha(color::SPACE, 0.88),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(CARD_X + 24.0, CARD_TOP, CARD_WIDTH - 48.0, 3.0),
            radius: 1.5 * s,
            color: tone,
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 16.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::HOLO, 0.3),
        });
        let x = CARD_X + CARD_PAD;
        let width = CARD_WIDTH - CARD_PAD * 2.0;
        let mut y = CARD_TOP + CARD_PAD;
        let kind = match entry {
            Entry::Tag(tag) => format!("{} tag  ·  {tone_name}", tag.collection().name),
            Entry::Card(_) => format!("Card tag  ·  {tone_name}"),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{kind}"),
            frame.rect(x, y, width, 22.0),
            16.0 * s,
            tone,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 26.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x, y, width, 48.0),
            if name.chars().count() > 22 {
                30.0
            } else {
                38.0
            } * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 60.0;
        // A tag's picture is a 128-pixel atlas cell: no larger, or it blurs. A card's is
        // its full picture.
        let art = match entry {
            Entry::Tag(_) => 224.0_f32.min(width),
            Entry::Card(_) => 300.0_f32.min(width),
        };
        let art_height = match entry {
            Entry::Tag(_) => art,
            Entry::Card(_) => art / crate::cards::faces::ART_ASPECT,
        };
        super::view::glow(
            &mut self.ui,
            frame,
            [CARD_X + CARD_WIDTH * 0.5, y + art_height * 0.5],
            art * 0.55,
            tone,
            0.22,
        );
        let alpha = if count > 0 { 1.0 } else { 0.4 };
        let art_rect = [CARD_X + (CARD_WIDTH - art) * 0.5, y, art, art];
        let shown = [art_rect[0], y, art, art_height];
        self.tag_picture(frame, entry, [art_rect[0], y, art, art_height], alpha, 30.0);
        // The finish shown, over the picture as it lies on a wall.
        let kind = match finish.index {
            1 => Some(sjk_ui::ShineKind::Holo),
            2 => Some(sjk_ui::ShineKind::Solar),
            _ => None,
        };
        if kind.is_some() && count > 0 {
            let seconds = self.epoch.elapsed().as_secs_f32();
            crate::tile_shine::push(
                self.ui.draw_list_mut(),
                frame.rect(shown[0], shown[1], shown[2], shown[3]),
                10.0 * s,
                kind,
                None,
                seconds,
            );
        }
        y += art_height + 18.0;
        // The finishes: a chip each, the one shown ringed, the ones not held dimmed.
        self.tag_finish_chips(frame, [x, y, width, 40.0], held, finish);
        y += 52.0;
        let held_line = match count {
            0 => "You do not hold it yet.".to_owned(),
            1 => "Yours.".to_owned(),
            count => format!("Yours, {count} copies."),
        };
        let how = match entry {
            Entry::Tag(tag) => format!("{} {}", tag.how_to_get(), tag.collection().about),
            Entry::Card(_) => {
                "A credits card: stick the copies you hold, in their finishes.".to_owned()
            }
        };
        for (words, colour) in [
            (how, color::TEXT),
            (held_line, color::MUTED),
            (PROPOSE.to_owned(), color::QUIET),
        ] {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(x, y, width, 24.0),
                17.0 * s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 30.0;
        }
        self.tag_tools_row(frame, x, y + 4.0, width);
        let in_use_here = in_use == Some(finish.id);
        let label = if in_use_here {
            "In use"
        } else if count > 0 {
            "Choose"
        } else {
            "Locked"
        };
        let button = [x, CARD_BOTTOM - CARD_PAD - 48.0, 200.0, 48.0];
        kit::button(
            &mut self.ui,
            frame,
            button,
            label,
            true,
            count > 0 && !in_use_here,
            false,
            TAG_CHOOSE_TOKEN,
        );
        let [bx, by, bw, _] = button;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("toy_tag sticks it where you look"),
            frame.rect(bx + bw + 18.0, by + 12.0, width - bw - 18.0, 24.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The card's finish chips over `[x, y, width, height]`: Normal, Holo and Solar, each
    /// with the copies held in it; `shown` ringed gold, one not held dimmed and inert.
    fn tag_finish_chips(
        &mut self,
        frame: &Frame,
        [x, y, width, height]: [f32; 4],
        held: Held,
        shown: &crate::cards::Finish,
    ) {
        let s = frame.s;
        let gap = 10.0;
        let chip = (width - gap * 2.0) / 3.0;
        for finish in &crate::cards::FINISHES {
            let left = x + finish.index as f32 * (chip + gap);
            let has = held.has(finish);
            let token = TAG_FINISH_BASE + finish.index as u16;
            let hovered = has && self.ui.token_hovered(token);
            let rect = frame.rect(left, y, chip, height);
            let active = finish.id == shown.id && has;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: height * 0.5 * s,
                color: color::alpha(
                    if active { color::GOLD } else { color::HOLO },
                    if active {
                        0.16
                    } else if hovered {
                        0.12
                    } else {
                        0.05
                    },
                ),
            });
            if finish.index > 0 && has {
                let kind = if finish.index == 1 {
                    sjk_ui::ShineKind::Holo
                } else {
                    sjk_ui::ShineKind::Solar
                };
                crate::tile_shine::push(
                    self.ui.draw_list_mut(),
                    rect,
                    height * 0.5 * s,
                    Some(kind),
                    None,
                    self.epoch.elapsed().as_secs_f32(),
                );
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: height * 0.5 * s,
                width: if active { 1.6 } else { 1.0 } * s,
                color: if active {
                    color::GOLD_BRIGHT
                } else {
                    color::alpha(color::HOLO, if has { 0.35 } else { 0.12 })
                },
            });
            let copies = match finish.index {
                1 => held.holo,
                2 => held.solar,
                _ => held.copies,
            };
            let label = if finish.index > 0 && copies > 0 {
                format!("{}  x{copies}", finish.name)
            } else {
                finish.name.to_owned()
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(left, y + (height - 22.0) * 0.5, chip, 22.0),
                16.0 * s,
                if active {
                    color::GOLD_BRIGHT
                } else if has {
                    color::TEXT
                } else {
                    color::QUIET
                },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            if has {
                self.ui.hit_region(token, rect);
            }
        }
    }
}

/// A small badge naming a finish at `[x, y]` (a tile's Holo or Solar copies).
fn finish_badge(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    label: &str,
) {
    let s = frame.s;
    let rect = frame.rect(x, y, 52.0, 20.0);
    let solar = label == crate::cards::FINISHES[2].name;
    let ink = if solar {
        Color::new(1.0, 0.8, 0.32, 1.0)
    } else {
        Color::new(0.55, 0.85, 1.0, 1.0)
    };
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect,
        radius: 10.0 * s,
        color: color::alpha(color::SPACE, 0.85),
    });
    let _ = canvas.draw_list_mut().push(DrawCommand::Border {
        rect,
        radius: 10.0 * s,
        width: 1.0 * s,
        color: Color::new(ink.r, ink.g, ink.b, 0.8),
    });
    text(
        canvas,
        TextFamily::Body,
        format_args!("{label}"),
        frame.rect(x, y + 2.0, 52.0, 16.0),
        12.0 * s,
        ink,
        FontWeight::Semibold,
        TextAlign::Center,
    );
}

/// The wall's parts: each collection's heading and how many tags it holds, then the
/// `cards` credits cards'.
fn wall_parts(cards: usize) -> Vec<(&'static str, usize)> {
    crate::tags::COLLECTIONS
        .iter()
        .map(|collection| {
            let held = crate::tags::CATALOGUE
                .iter()
                .filter(|tag| tag.collection == collection.id)
                .count();
            (collection.heading, held)
        })
        .chain([(CARDS_HEAD, cards)])
        .collect()
}

/// What an entry reads as: its name ("Gonk", a card's title), its tier's or kind's name
/// ("Rare", "Credits") and colour (a card's rank's or rarity's).
fn entry_words(entry: &Entry) -> (String, String, Color) {
    match entry {
        Entry::Tag(tag) => (
            tag.name.trim_end_matches(" tag").to_owned(),
            tag.tier.label().to_owned(),
            tag.tier.colour(),
        ),
        Entry::Card(id) => match crate::cards::faces::current().find(id) {
            Some((_, face)) => (
                face.name.clone(),
                face.kind.name().to_owned(),
                face.tone().colour,
            ),
            None => (id.clone(), "Card".to_owned(), color::QUIET),
        },
    }
}

/// The tag `toy_tag` sticks: `cg_tag`, or the default when it is empty.
fn picked<'a>(inputs: &Inputs<'a>) -> &'a str {
    let chosen = inputs.tag.trim();
    if chosen.is_empty() {
        crate::tags::DEFAULT
    } else {
        chosen
    }
}

/// The finish `toy_tag` sticks the tag in use in, holding `held` of it: `cg_tagFinish`
/// when held, Normal otherwise.
fn picked_finish(inputs: &Inputs<'_>, held: Held) -> &'static str {
    held.finish(inputs.tag_finish.trim()).id
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAGS_HEAD: &str = "Alpha collection";

    /// The tags stand under their collections, Alpha then Community, then the cards.
    #[test]
    fn the_tags_stand_under_their_collections() {
        let parts = wall_parts(2);
        assert_eq!(
            parts,
            [
                ("Alpha collection", crate::tags::CATALOGUE.len() - 3),
                ("Community collection", 3),
                (CARDS_HEAD, 2),
            ]
        );
        let heads: Vec<_> = lines(&parts)
            .into_iter()
            .filter_map(|line| match line {
                Line::Head(words) => Some(words),
                Line::Row(..) => None,
            })
            .collect();
        assert_eq!(
            heads,
            ["Alpha collection", "Community collection", CARDS_HEAD]
        );
    }

    /// Each part's rows under its heading, a part of none left out.
    #[test]
    fn the_wall_has_a_heading_for_each_part() {
        assert_eq!(
            lines(&[(TAGS_HEAD, 7), (CARDS_HEAD, 3)]),
            [
                Line::Head(TAGS_HEAD),
                Line::Row(0, 5),
                Line::Row(5, 7),
                Line::Head(CARDS_HEAD),
                Line::Row(7, 10),
            ]
        );
        assert_eq!(
            lines(&[(TAGS_HEAD, 3), (CARDS_HEAD, 0)]),
            [Line::Head(TAGS_HEAD), Line::Row(0, 3)]
        );
    }

    /// The wall scrolls: from the last first line everything fits, and the arrows bring
    /// the choice into view with its heading.
    #[test]
    fn the_wall_scrolls_and_follows_the_choice() {
        let wall = lines(&[(TAGS_HEAD, 17), (CARDS_HEAD, 13)]);
        let shown = shown_from(&wall, 0);
        assert!(shown < wall.len(), "{shown} of {}", wall.len());
        let last = last_first(&wall);
        assert_eq!(last + shown_from(&wall, last), wall.len());
        assert!(last == 0 || last - 1 + shown_from(&wall, last - 1) < wall.len());
        // The last card brings the wall down; the first tag back up, with its heading.
        let at = line_of(&wall, 29).unwrap();
        let first = first_showing(&wall, 0, at);
        assert!(first > 0 && first + shown_from(&wall, first) > at);
        assert_eq!(first_showing(&wall, first, line_of(&wall, 0).unwrap()), 0);
        // The first card's row, right under its heading, brings the heading too.
        let cards = line_of(&wall, 17).unwrap();
        assert_eq!(wall[cards - 1], Line::Head(CARDS_HEAD));
        assert!(first_showing(&wall, last, cards) < cards);
    }

    /// Up and Down go a row, past the headings, to the same column or the nearest.
    #[test]
    fn up_and_down_cross_the_headings() {
        let wall = lines(&[(TAGS_HEAD, 7), (CARDS_HEAD, 3)]);
        assert_eq!(vertical(&wall, 1, true), Some(6));
        assert_eq!(vertical(&wall, 3, true), Some(6), "the short row's last");
        assert_eq!(vertical(&wall, 6, true), Some(8), "into the cards");
        assert_eq!(vertical(&wall, 9, false), Some(6), "back over the heading");
        assert_eq!(vertical(&wall, 9, true), None);
        assert_eq!(vertical(&wall, 2, false), None);
    }
}
