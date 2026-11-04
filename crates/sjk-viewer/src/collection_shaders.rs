//! The Collection's Shaders tab: the saber's looks as a wall of tiles, a heading to each
//! tier, or as a list whose columns sort it ([`SHADER_VIEW_CVAR`], switched by V or the
//! two marks by the kinds; Sol's pick of 25/10/2026, the Collection revamp's canvas): the
//! stock blade first in the player's colour, then every blade skin
//! (`docs/unlockables.md`), the rarest tier first, each framed in its tier's colour, alive
//! when owned, a still ghost of its own colours when not. A strip of the tiers over them
//! says how many of each the player holds and filters by one. The one chosen is the blade
//! the stage shows on its own beside the page ([`Backstage`]), a locked one too, as a
//! preview only this screen draws; its card under it says what it is, since when it is
//! the player's or how to get it, and Equip or Unequip, and grows upward to hold all its
//! words. Body shaders, for the whole character, are to come: their kind is shown, not
//! yet offered.
//!
//! Since 29/10/2026 every shader carries its marks (`crate::shader_marks`): a New sticker
//! for one gained since the player last looked, and, on a hub that keeps them, a star for
//! a favourite (listed first, under a heading of their own by tier) and a padlock for one
//! the player locked against breaking down and trading; the card's two toggles, F and L set
//! them, Shift+F shows the favourites alone and N marks every New one seen.

use super::view::{LEAD_Y, LEFT_X, RIGHT_X, SABER_AREA};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::rarity_fx;
use crate::symbols;
use crate::unlockables::{Rarity, Unlockable};
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The wall and the list: their left edge and width, the top and the foot of the room
/// they scroll in, and the scroll bar's place at their right.
const LIST_X: f32 = LEFT_X - 12.0;
const LIST_WIDTH: f32 = 860.0;
const LIST_TOP: f32 = 400.0;
const LIST_BOTTOM: f32 = 990.0;
const SCROLL_X: f32 = LIST_X + LIST_WIDTH + 8.0;
/// The list's column heads stand at its top; its rows under them.
const HEADS_HEIGHT: f32 = 28.0;
const ROWS_TOP: f32 = LIST_TOP + HEADS_HEIGHT + 6.0;
/// The wall's tiles a line, the room between them and a tile's width.
pub(super) const WALL_COLUMNS: usize = 6;
const TILE_GAP: f32 = 10.0;
const TILE_WIDTH: f32 = (LIST_WIDTH - TILE_GAP * (WALL_COLUMNS - 1) as f32) / WALL_COLUMNS as f32;
/// A section's heading on the wall and in the list.
const WALL_HEAD: f32 = 34.0;
const LIST_HEAD: f32 = 30.0;
/// How far the chosen tile lifts (frame pixels), and its outline's colour: white, so it
/// never reads as a tier (Mythical's frame is gold).
const CHOSEN_LIFT: f32 = 4.0;
const CHOSEN_OUTLINE: sjk_ui::Color = sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0);
/// The chroma mark's side on a tile and a row.
const MARK: f32 = 16.0;
/// The view switches' side and where the first stands, left of the kinds.
const SWITCH: f32 = 36.0;
const SWITCH_X: f32 = LEFT_X + 466.0;
/// The filters' line under the headline (the Owned and Chroma chips and the search field)
/// and the tiers' strip under it.
const TOOLBAR_Y: f32 = 292.0;
const TOOLBAR_HEIGHT: f32 = 30.0;
const STRIP_Y: f32 = 334.0;
const STRIP_HEIGHT: f32 = 52.0;
const STRIP_GAP: f32 = 8.0;

/// The detail's state pill: its height, its least width and its gap from the name.
const TAG_HEIGHT: f32 = 26.0;
const TAG_MIN_WIDTH: f32 = 80.0;
const TAG_GAP: f32 = 18.0;
/// The chosen shader's card, under the lone saber (frame pixels): its left edge and
/// width, its foot, and the highest its top rises to make room for its words; the room
/// inside its edge and its corners' radius.
const CARD_X: f32 = 1_000.0;
const CARD_WIDTH: f32 = RIGHT_X - CARD_X;
const CARD_BOTTOM: f32 = 990.0;
const CARD_TOP_MIN: f32 = 436.0;
const CARD_PAD: f32 = 28.0;
const CARD_RADIUS: f32 = 16.0;
/// The card's facts: a chip's height, its icon's side and the room between chips.
const FACT_HEIGHT: f32 = 34.0;
const FACT_ICON: f32 = 20.0;
const FACT_GAP: f32 = 10.0;
/// The card's words: the description's size, the other lines', and the characters a
/// line of each holds at the ordinary text size (fewer at a larger one).
const ABOUT_SIZE: f32 = 18.0;
const LINE_SIZE: f32 = 16.0;
const ABOUT_CHARS: f32 = 78.0;
const LINE_CHARS: f32 = 88.0;

/// The player's text size (`ui_textScale`): rows, tiles and lines grow with it.
fn text_scale() -> f32 {
    crate::text::style::current().scale.max(0.5)
}

/// A wall tile's height: its strip, its name on two lines and its state, at the player's
/// text size.
fn tile_height() -> f32 {
    48.0 + 52.0 * text_scale()
}

/// A list row's height.
fn row_height() -> f32 {
    30.0 + 10.0 * text_scale()
}

/// The group a row stands in: under a heading of its own on the wall and in the list by
/// tier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Group {
    /// The stock blade.
    Stock,
    /// The player's favourites (29/10/2026), first after the stock blade.
    Favourites,
    /// A tier's.
    Tier(Rarity),
}

/// One line of the Shaders tab, as the wall or the list lays it out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Line {
    /// A section's heading: its group.
    Head(Group),
    /// Rows `start..end` side by side: a line of the wall's tiles, or one row of the
    /// list.
    Things(usize, usize),
}

/// How tall `line` stands on the wall (`grid`) or in the list.
fn line_height(line: &Line, grid: bool) -> f32 {
    match (line, grid) {
        (Line::Head(_), true) => WALL_HEAD,
        (Line::Head(_), false) => LIST_HEAD,
        (Line::Things(..), true) => tile_height() + TILE_GAP,
        (Line::Things(..), false) => row_height() + 2.0,
    }
}

/// The room the lines scroll in.
fn room(grid: bool) -> f32 {
    LIST_BOTTOM - if grid { LIST_TOP } else { ROWS_TOP }
}

/// The lines of `rows`: on the wall a heading to each tier (the stock blade's too) and
/// its tiles [`WALL_COLUMNS`] a line; in the list a row to each, under a heading to each
/// tier while it stands by tier (the stock blade, first, without one).
fn lines_of(rows: &[Row], grid: bool, by_tier: bool) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut index = 0;
    while index < rows.len() {
        let group = rows[index].group();
        let end = if grid || by_tier {
            rows[index..]
                .iter()
                .position(|row| row.group() != group)
                .map_or(rows.len(), |count| index + count)
        } else {
            rows.len()
        };
        if grid || (by_tier && group != Group::Stock) {
            lines.push(Line::Head(group));
        }
        let step = if grid { WALL_COLUMNS } else { 1 };
        let mut start = index;
        while start < end {
            let stop = (start + step).min(end);
            lines.push(Line::Things(start, stop));
            start = stop;
        }
        index = end;
    }
    lines
}

/// The line holding row `row`, and its column in it.
fn line_of(lines: &[Line], row: usize) -> Option<(usize, usize)> {
    lines.iter().enumerate().find_map(|(at, line)| match *line {
        Line::Things(start, end) if (start..end).contains(&row) => Some((at, row - start)),
        _ => None,
    })
}

/// How many of `lines` from `first` stand in the room, one at least.
fn shown_from(lines: &[Line], first: usize, grid: bool) -> usize {
    let room = room(grid);
    let mut used = 0.0;
    let mut count = 0;
    for line in lines.iter().skip(first) {
        let height = line_height(line, grid);
        if count > 0 && used + height > room + 0.5 {
            break;
        }
        used += height;
        count += 1;
    }
    count.max(1)
}

/// The last line the room may start at: from it the lines left fill the room to their
/// end.
fn last_first(lines: &[Line], grid: bool) -> usize {
    let room = room(grid);
    let mut used = 0.0;
    let mut first = lines.len();
    while first > 0 {
        let height = line_height(&lines[first - 1], grid);
        if used + height > room + 0.5 {
            break;
        }
        used += height;
        first -= 1;
    }
    first.min(lines.len().saturating_sub(1))
}

/// The first line shown, from the one shown first: moved just enough that the line of
/// row `chosen` shows, with its tier's heading when it is that tier's first line, never
/// past the end.
fn line_window(lines: &[Line], first: usize, chosen: usize, grid: bool) -> usize {
    let last = last_first(lines, grid);
    let mut first = first.min(last);
    let Some((at, _)) = line_of(lines, chosen) else {
        return first;
    };
    let top = if at > 0 && matches!(lines[at - 1], Line::Head(_)) {
        at - 1
    } else {
        at
    };
    if top < first {
        first = top;
    }
    while at >= first + shown_from(lines, first, grid) && first < last {
        first += 1;
    }
    first
}

/// The row Up (`down` false) or Down moves to on the wall from row `chosen`: the line of
/// tiles above or under it, past headings, in the same column or its nearest; `chosen`
/// itself at the first or the last line.
pub(super) fn vertical(lines: &[Line], chosen: usize, down: bool) -> usize {
    let Some((at, column)) = line_of(lines, chosen) else {
        return chosen;
    };
    let mut index = at;
    loop {
        if down {
            index += 1;
            if index >= lines.len() {
                return chosen;
            }
        } else {
            if index == 0 {
                return chosen;
            }
            index -= 1;
        }
        if let Line::Things(start, end) = lines[index] {
            return start + column.min(end - start - 1);
        }
    }
}

/// The first of `rows` a window of `shown` shows, from the one it showed first: moved
/// just enough that `chosen` is in view, never past either end (the Cards tab's binder and
/// the Forge's lists).
pub(super) fn window(first: usize, chosen: usize, rows: usize, shown: usize) -> usize {
    let first = if chosen < first {
        chosen
    } else if chosen >= first + shown {
        chosen + 1 - shown
    } else {
        first
    };
    first.min(rows.saturating_sub(shown))
}

/// Where `skin` comes from, as facts' icons and words: the medal that brings it, or a
/// holocron (it drops) and the card that also brings it, or the SJK team.
fn sources(skin: &Unlockable) -> Vec<(char, String)> {
    if let Some(medal) = crate::medals::Medal::ALL
        .iter()
        .find(|medal| medal.blade() == Some(skin.id))
    {
        return vec![(
            symbols::MEDAL,
            crate::terms::say_owned(format!("{} medal", medal.name())),
        )];
    }
    let mut sources = Vec::new();
    if skin.droppable {
        sources.push((
            symbols::HOLOCRON,
            crate::terms::say("Holocron drop").into_owned(),
        ));
    }
    if let Some(card) = unlockables::card_bringing(skin.id) {
        let name = crate::cards::faces::name_of(card);
        sources.push((symbols::FINISH[0], format!("{name} card")));
    }
    if sources.is_empty() {
        sources.push((symbols::GIFT, "SJK team gift".to_owned()));
    }
    sources
}

/// The detail's name and state pill on the line at `y` of the column from `x`, `width`
/// wide: the pill at the column's right end, the name ellipsised short of it.
fn title_rects(x: f32, y: f32, width: f32, tag_width: f32) -> ([f32; 4], [f32; 4]) {
    let tag_width = tag_width.max(TAG_MIN_WIDTH);
    let tag = [x + width - tag_width, y + 18.0, tag_width, TAG_HEIGHT];
    let name = [x, y, (tag[0] - TAG_GAP - x).max(0.0), 62.0];
    (name, tag)
}

/// A shader's name on a tile or a row: without the " blade" every one ends in.
fn card_name(name: &str) -> &str {
    name.strip_suffix(" blade").unwrap_or(name)
}

/// `name` on at most two lines `width` wide at `size` (display family, the player's text
/// size): its words as they fit, the second line ellipsised when they do not.
fn name_lines(name: &str, width: f32, size: f32) -> (String, String) {
    let scale = text_scale();
    let fits = |words: &str| crate::text::display_width(words, size) * scale <= width;
    let words: Vec<&str> = name.split_whitespace().collect();
    let mut first = String::new();
    let mut taken = 0;
    for word in &words {
        let tried = if first.is_empty() {
            (*word).to_owned()
        } else {
            format!("{first} {word}")
        };
        if !first.is_empty() && !fits(&tried) {
            break;
        }
        first = tried;
        taken += 1;
    }
    let mut second = words[taken..].join(" ");
    if !fits(&second) {
        while !second.is_empty() && !fits(&format!("{second}...")) {
            second.pop();
        }
        second = format!("{}...", second.trim_end());
    }
    (first, second)
}

/// The owned line of a shader the player holds: since when, and from the team or the
/// medal or card that brings it (a medal the client does not know reads as the team, a
/// card without a face in the packs by its id).
fn owned_line(grant: &sjk_identity::Unlock) -> String {
    if crate::debug_collectibles::added(grant) {
        return crate::debug_collectibles::NOTE.to_owned();
    }
    let date = crate::medals::date_text(grant.granted);
    let medal = grant
        .medal
        .as_deref()
        .and_then(crate::medals::Medal::from_id)
        .map(|medal| crate::terms::say_owned(format!("with your {} medal", medal.name())));
    let card = || {
        grant
            .card
            .as_deref()
            .map(|card| format!("with your {} card", crate::cards::faces::name_of(card)))
    };
    let from = medal
        .or_else(card)
        .unwrap_or_else(|| "from the SJK team".to_owned());
    let held = if grant.count > 1 {
        format!(", {} held", grant.count)
    } else {
        String::new()
    };
    if date.is_empty() {
        format!("Yours, {from}{held}")
    } else {
        format!("Yours since {date}, {from}{held}")
    }
}

/// What a row shows: the stock blade, or a blade skin.
#[derive(Clone, Copy)]
struct Row {
    skin: Option<&'static Unlockable>,
    owned: bool,
    worn: bool,
    /// The second saber wears it (`cg_saberSkin2`) and not the first.
    second: bool,
    /// How many the player holds (holocrons may drop the same shader again): 1 for the
    /// stock blade and a shader held once, 0 when locked.
    count: u32,
    /// Its blade number shows: only while `debug_collectibles` is on (Sol, 21/10/2026).
    numbered: bool,
    /// Its collection marks (29/10/2026, `crate::shader_marks`): a favourite, locked, New.
    favourite: bool,
    locked: bool,
    new: bool,
}

/// `word` with how many are held after it when there are several: "Yours x2".
fn held_words(word: &str, count: u32) -> String {
    if count > 1 {
        format!("{word} x{count}")
    } else {
        word.to_owned()
    }
}

impl Row {
    fn name(self) -> &'static str {
        self.skin.map_or("Stock blade", |skin| skin.name)
    }

    /// Its blade number after its tier ("  ·  042"), nothing for the stock blade or
    /// when numbers are hidden ([`Self::numbered`]).
    fn number_words(self) -> String {
        self.skin
            .filter(|_| self.numbered)
            .map(|skin| format!("  ·  {}", skin.number_text()))
            .unwrap_or_default()
    }

    /// Its tier, `None` for the stock blade.
    fn tier(self) -> Option<Rarity> {
        self.skin.map(|skin| skin.tier)
    }

    /// The group it stands in: the stock blade's, the favourites' or its tier's.
    fn group(self) -> Group {
        match self.skin {
            None => Group::Stock,
            Some(_) if self.favourite => Group::Favourites,
            Some(skin) => Group::Tier(skin.tier),
        }
    }

    /// Its frame's colour: its tier's, quieter when locked; the stock blade's is the
    /// page's own.
    fn frame_colour(self) -> sjk_ui::Color {
        match self.tier() {
            Some(tier) => color::alpha(tier.colour(), if self.owned { 0.9 } else { 0.4 }),
            None => color::alpha(color::HOLO, 0.3),
        }
    }

    /// Its state in a word after its icon (a lightsaber worn, a tick held; "Not owned"
    /// without one, since 29/10/2026 the padlock is the player's own lock), with how many
    /// are held: a tile's line and a row's Yours.
    fn status(self) -> String {
        match (self.skin, self.worn, self.owned) {
            (None, true, _) => format!("{}  Worn", symbols::SABER),
            (None, false, _) if self.second => {
                format!("{}  2nd blade", symbols::SABER)
            }
            (None, false, _) => "Your colour".to_owned(),
            (Some(_), false, true) if self.second => {
                format!(
                    "{}  {}",
                    symbols::SABER,
                    held_words("2nd blade", self.count)
                )
            }
            (Some(_), true, _) => format!("{}  {}", symbols::SABER, held_words("Worn", self.count)),
            (Some(_), false, true) => {
                format!("{}  {}", symbols::TICK, held_words("Yours", self.count))
            }
            (Some(_), false, false) => "Not owned".to_owned(),
        }
    }

    /// Its state on its card's pill (the copies held are a fact of their own there):
    /// `None` for the stock blade.
    fn state(self) -> Option<String> {
        match (self.skin, self.worn, self.owned) {
            (None, _, _) => None,
            (Some(_), true, _) => Some(format!("{}  Worn", symbols::SABER)),
            (Some(_), false, true) if self.second => Some(format!("{}  2nd blade", symbols::SABER)),
            (Some(_), false, true) => Some(format!("{}  Yours", symbols::TICK)),
            (Some(_), false, false) => Some("Not owned".to_owned()),
        }
    }
}

/// What the second saber (or a staff's other half) wears by its own choice
/// (`cg_saberSkin2`): `None` when it has none and wears the first's, `Some(None)` for the
/// stock blade, `Some(Some(id))` for a skin the player holds or the Saber Lab gives on this
/// PC (one not held is no choice, as `looks::SecondSkin::own` reads it).
fn second_own(inputs: &Inputs<'_>) -> Option<Option<&'static str>> {
    match inputs.second.trim() {
        "" => None,
        crate::looks::STOCK_SECOND => Some(None),
        id => unlockables::blade_skin(id)
            .filter(|skin| {
                inputs.holdings.unlock(skin.id).is_some() || crate::saber_lab::owns(skin.id)
            })
            .map(|skin| Some(skin.id)),
    }
}

/// The list's columns, left edges and widths (frame pixels): the blade's number (while
/// numbers show), its look, its name, its tier, its smith, what the player holds and, while
/// the hub's economy is known, what the Forge asks or gives.
#[derive(Clone, Copy, Debug)]
struct Columns {
    number: Option<[f32; 2]>,
    look: [f32; 2],
    name: [f32; 2],
    tier: [f32; 2],
    smith: [f32; 2],
    yours: [f32; 2],
    forge: Option<[f32; 2]>,
}

impl Columns {
    fn of(numbers: bool, forge: bool) -> Self {
        const GAP: f32 = 12.0;
        let left = LIST_X + 16.0;
        let right = LIST_X + LIST_WIDTH - 14.0;
        let mut x = left;
        let mut take = |width: f32| {
            let column = [x, width];
            x += width + GAP;
            column
        };
        let number = numbers.then(|| take(40.0));
        let look = take(150.0);
        let name_x = x;
        let forge = forge.then_some([right - 80.0, 80.0]);
        let yours_right = forge.map_or(right, |[x, _]| x - GAP);
        let yours = [yours_right - 82.0, 82.0];
        let smith = [yours[0] - GAP - 84.0, 84.0];
        let tier = [smith[0] - GAP - 112.0, 112.0];
        let name = [name_x, (tier[0] - GAP - name_x).max(60.0)];
        Self {
            number,
            look,
            name,
            tier,
            smith,
            yours,
            forge,
        }
    }
}

impl Panel {
    /// The rows: the stock blade, then, while the lab is on (`lab`), the Saber Lab's blade
    /// ([`crate::saber_lab::active`]), then the shaders the player may see
    /// ([`unlockables::Holdings::listed`]: a Unique one only while held) that `filter` keeps,
    /// by tier (the rarest first), or in the list's order `sort` when not `by_tier`; the
    /// favourites among `marks` first, each part in that order.
    fn rows(
        inputs: &Inputs<'_>,
        filter: &ShaderFilter,
        sort: ShaderSort,
        by_tier: bool,
        marks: &crate::shader_marks::Marks,
        lab: bool,
    ) -> Vec<Row> {
        // A Saber Lab skin is worn on this PC while the lab is on (`saber_lab::owns`).
        let wearable = |skin: &unlockables::Unlockable| {
            inputs.holdings.can_wear(skin) || crate::saber_lab::owns(skin.id)
        };
        let worn = unlockables::blade_skin(inputs.setting)
            .filter(|skin| wearable(skin))
            .map(|skin| skin.id);
        // "2nd blade" only while there is a second saber or a staff's other half to wear it,
        // on a row the first does not wear.
        let pair = inputs.pair != crate::saber_skins::Pair::None;
        let second = second_own(inputs);
        let mut skins: Vec<Row> = inputs
            .holdings
            .listed()
            .filter(|skin| {
                filter.passes(
                    skin,
                    inputs.holdings.unlock(skin.id).is_some(),
                    marks.favourite(skin.id),
                )
            })
            .map(|skin| Row {
                skin: Some(skin),
                owned: wearable(skin),
                worn: worn == Some(skin.id),
                second: pair && second == Some(Some(skin.id)) && worn != Some(skin.id),
                count: inputs
                    .holdings
                    .unlock(skin.id)
                    .map_or(0, |unlock| unlock.count),
                numbered: inputs.holdings.debug(),
                favourite: marks.favourite(skin.id),
                locked: marks.locked(skin.id),
                new: marks.is_new(skin.id),
            })
            .collect();
        if !by_tier {
            match sort {
                ShaderSort::Tier => {}
                ShaderSort::Name => skins.sort_by_key(|row| row.name().to_lowercase()),
                // Stable: by tier within the held and the rest, as listed.
                ShaderSort::Yours => skins.sort_by_key(|row| !row.owned),
                ShaderSort::Number => {
                    skins.sort_by_key(|row| row.skin.map_or(0, |skin| skin.number))
                }
            }
        }
        // The favourites first, in whatever order stands (29/10/2026).
        crate::shader_marks::favourites_first(&mut skins, marks, |row| {
            row.skin.map_or("", |skin| skin.id)
        });
        std::iter::once(Row {
            skin: None,
            owned: true,
            worn: worn.is_none(),
            second: pair && second == Some(None) && worn.is_some(),
            count: 1,
            numbered: false,
            favourite: false,
            locked: false,
            new: false,
        })
        // The Saber Lab's blade, first after the stock one while the lab is on, whatever
        // the filter or the order: only ever previewed.
        .chain(lab.then_some(Row {
            skin: Some(&crate::saber_lab::BLADE),
            owned: true,
            worn: worn == Some(crate::saber_lab::ID),
            second: pair
                && second == Some(Some(crate::saber_lab::ID))
                && worn != Some(crate::saber_lab::ID),
            count: 1,
            // No catalogue number to show.
            numbered: false,
            favourite: false,
            locked: false,
            new: false,
        }))
        .chain(skins)
        .collect()
    }

    /// The Shaders tab, swatches alive at `seconds`.
    pub(super) fn shaders(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        self.grid = inputs.grid;
        self.pair = inputs.pair;
        if self.pair == crate::saber_skins::Pair::None {
            self.second_target = false;
        }
        self.shader_numbers = inputs.holdings.debug();
        if self.shader_sort == ShaderSort::Number && !self.shader_numbers {
            self.shader_sort = ShaderSort::Tier;
        }
        // Unique's segment only for a player who holds a Unique shader; one taken back
        // takes its segment, and the filter on it, with it.
        self.unique_chip = inputs
            .holdings
            .listed()
            .any(|skin| skin.hidden_unless_held());
        if self
            .shader_filter
            .tier
            .is_some_and(|tier| !self.tier_chips().contains(&tier))
        {
            self.shader_filter.tier = None;
            let _ = self.filter_changed();
        }
        let by_tier = self.grid || self.shader_sort == ShaderSort::Tier;
        // The shader chosen last frame, in last frame's list: a list that changes under it
        // (a Unique shader the hub's answer brings first, one given or taken back) keeps it
        // chosen rather than whatever now stands at its place.
        let chosen = self.skin_at(self.shader).map(|skin| skin.id);
        self.follow_marks(inputs);
        if !self.marks.kept && self.shader_filter.favourite {
            // A hub that keeps no marks (another one, or the identity off) has no favourites.
            self.shader_filter.favourite = false;
        }
        let rows = Self::rows(
            inputs,
            &self.shader_filter,
            self.shader_sort,
            by_tier,
            &self.marks,
            crate::saber_lab::active(),
        );
        self.shader_list = rows.iter().filter_map(|row| row.skin).collect();
        self.shader_owned.clear();
        self.shader_owned.extend(
            rows.iter()
                .filter(|row| row.owned)
                .filter_map(|row| row.skin.map(|skin| skin.id)),
        );
        self.shader_count = rows.len();
        // A blade card's button (or a change of view or order) asked for its shader.
        let wanted = self.shader_focus.take().or(chosen);
        if let Some(id) = wanted
            && let Some(row) = rows
                .iter()
                .position(|row| row.skin.is_some_and(|skin| skin.id == id))
        {
            self.shader = row;
        }
        if self.shader >= rows.len() {
            // Opening on what is worn: from the top, scrolled only as far as it needs.
            self.shader = rows.iter().position(|row| row.worn).unwrap_or(0);
            self.shader_first = 0;
        }
        // A New one chosen is seen once the choice leaves it.
        self.view(
            rows.get(self.shader)
                .and_then(|row| row.skin)
                .map(|skin| skin.id),
        );
        let owned = unlockables::blade_skins()
            .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
            .count();
        // Out of the shaders this player sees: an unheld Unique one is not counted.
        let total = inputs.holdings.listed_count();
        let headline = if inputs.holdings.reason().is_none() {
            format!("{owned} of {total} owned")
        } else {
            "Saber shaders".to_owned()
        };
        let shown_words;
        let line = match inputs.holdings {
            unlockables::Holdings::Known(_) if self.shader_filter.active() => {
                shown_words = format!(
                    "{} of {total} shaders shown: T tier, O owned, C chroma, / search.",
                    rows.len() - 1,
                );
                shown_words.as_str()
            }
            unlockables::Holdings::IdentityOff => {
                "Shaders are kept on the SJK hub: switch the identity on in Settings, Network."
            }
            unlockables::Holdings::NoHub => {
                "Shaders are kept on the SJK hub: no hub is set (cl_hubUrl)."
            }
            unlockables::Holdings::Waiting => "Waiting for the SJK hub to say which are yours.",
            holdings @ unlockables::Holdings::Known(_) if holdings.debug() => {
                "Debug collectibles is on: every shader is shown as yours, on this PC only."
            }
            unlockables::Holdings::Known(_) => {
                "How your blade looks and sounds, for every SJK player on your server."
            }
        };
        // The Saber Lab's folder and what it last gave, while the lab is on.
        let lab = crate::saber_lab::status();
        self.lead(frame, &headline, lab.as_deref().unwrap_or(line));
        self.new_count(frame, &headline);
        self.kinds(frame, owned, total);
        self.view_switches(frame);
        self.toolbar(frame);
        self.tier_strip(frame, inputs);
        let unlocks = matches!(inputs.holdings, unlockables::Holdings::Known(_));
        // A side just chosen: show the row it wears (the second's own, else the first's).
        if std::mem::take(&mut self.jump_to_worn) {
            let own = second_own(inputs);
            let worn_by_target = rows
                .iter()
                .position(|row| {
                    self.second_target && own.is_some_and(|own| own == row.skin.map(|s| s.id))
                })
                .or_else(|| rows.iter().position(|row| row.worn));
            if let Some(index) = worn_by_target {
                self.shader = index;
            }
        }
        // While the second has no choice of its own it wears the first's.
        let second = second_own(inputs);
        for (index, row) in rows.iter().enumerate() {
            // What a second saber wears now: its own choice, else the first's.
            let row_id = row.skin.map(|skin| skin.id);
            let second_id = row_id.unwrap_or(crate::looks::STOCK_SECOND);
            let second_worn = second.map_or(row.worn, |own| own == row_id);
            // A Saber Lab skin is worn on this PC as one held is (`saber_lab::owns`).
            let wearable = row.owned
                && (unlocks || row.skin.is_some_and(|skin| crate::saber_lab::owns(skin.id)));
            self.shader_rows[index] = ShaderRow {
                wear: match row.skin {
                    // The stock blade, worn by taking the skin off.
                    None => (!row.worn).then_some(""),
                    Some(skin) if wearable => Some(if row.worn { "" } else { skin.id }),
                    Some(_) => None,
                },
                // Offered while the player holds two sabers or a staff: Equip puts it on the
                // second, Unequip leaves the second the stock blade (the stock row has none).
                second: if inputs.pair == crate::saber_skins::Pair::None
                    || !(row.skin.is_none() || wearable)
                {
                    None
                } else if second_worn {
                    row.skin.map(|_| crate::looks::STOCK_SECOND)
                } else {
                    Some(second_id)
                },
                second_worn,
                // Both sides at once ("Both" box): they wear it, the second following the
                // first; Unequip leaves both the stock blade.
                both: if inputs.pair == crate::saber_skins::Pair::None
                    || !(row.skin.is_none() || wearable)
                    || (row.skin.is_none() && row.worn && second_worn)
                {
                    None
                } else if row.worn && second_worn {
                    Some(("", ""))
                } else {
                    Some((row_id.unwrap_or(""), ""))
                },
                both_worn: row.worn && second_worn,
            };
        }
        // The wall or the list scrolls just enough to keep the chosen one in view.
        self.shader_lines = lines_of(&rows, self.grid, by_tier);
        self.shader_first = line_window(
            &self.shader_lines,
            self.shader_first,
            self.shader,
            self.grid,
        );
        let first = self.shader_first;
        let lines = std::mem::take(&mut self.shader_lines);
        let shown = shown_from(&lines, first, self.grid);
        if !self.grid {
            self.column_heads(frame);
        }
        let columns = Columns::of(self.shader_numbers, self.trade_known);
        let mut y = if self.grid { LIST_TOP } else { ROWS_TOP };
        for line in lines.iter().skip(first).take(shown) {
            let height = line_height(line, self.grid);
            match *line {
                Line::Head(group) => self.heading(frame, inputs, group, y, height),
                Line::Things(start, end) => {
                    for (index, row) in rows.iter().enumerate().take(end).skip(start) {
                        if self.grid {
                            let column = (index - start) as f32;
                            let rect = [
                                LIST_X + column * (TILE_WIDTH + TILE_GAP),
                                y,
                                TILE_WIDTH,
                                tile_height(),
                            ];
                            self.wall_tile(frame, *row, index, rect, inputs, seconds);
                        } else {
                            self.list_row(frame, *row, index, y, inputs, columns);
                        }
                    }
                }
            }
            y += height;
        }
        self.scroll_bar(frame, &lines, first, shown);
        self.shader_shown = shown;
        self.shader_lines = lines;
        if let Some(row) = rows.get(self.shader).copied() {
            self.shader_card(frame, row, inputs, seconds);
        }
    }

    /// How many shaders are New, after the headline (`headline`'s words), and Mark all
    /// seen beside it (N), while there is one.
    fn new_count(&mut self, frame: &Frame, headline: &str) {
        let count = self.marks.new_count();
        if count == 0 {
            return;
        }
        let s = frame.s;
        let k = text_scale();
        let x = LEFT_X + crate::text::display_width(headline, 30.0) * k * self.widen + 18.0;
        let y = LEAD_Y + 5.0;
        let sticker = marks_view::new_sticker(&mut self.ui, frame, [x, y], 26.0);
        let words = format!("{count} new");
        let words_x = x + sticker + 10.0;
        let words_width = crate::text::display_width(&words, 20.0) * k * self.widen + 8.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{words}"),
            frame.rect(words_x, y - 1.0, words_width, 28.0),
            20.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let label = "Mark all seen";
        let width = crate::text::display_width(label, 15.0) * k * self.widen + 30.0;
        let rect = frame.rect(words_x + words_width + 8.0, y - 1.0, width, 28.0);
        let hovered = self.ui.token_hovered(SEEN_ALL_TOKEN);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 14.0 * s,
            color: color::alpha(color::GOLD, if hovered { 0.18 } else { 0.06 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 14.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::GOLD, if hovered { 1.0 } else { 0.55 }),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(words_x + words_width + 8.0, y + 3.0, width, 22.0),
            15.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        self.ui.hit_region(SEEN_ALL_TOKEN, rect);
    }

    /// Scroll the wall or the list a line up (`up`) or down; the shader chosen stays in
    /// view, on the nearest line in view in the same column when it would leave it.
    pub(super) fn scroll_shaders(&mut self, up: bool) {
        let lines = &self.shader_lines;
        if lines.is_empty() {
            return;
        }
        let last = last_first(lines, self.grid);
        self.shader_first = if up {
            self.shader_first.saturating_sub(1)
        } else {
            (self.shader_first + 1).min(last)
        };
        let first = self.shader_first;
        let shown = shown_from(lines, first, self.grid);
        let Some((at, column)) = line_of(lines, self.shader) else {
            return;
        };
        // A line may keep the choice when it is in view, but not as the top line with its
        // tier's heading scrolled off: the next frame's [`line_window`] would bring the
        // heading back and undo the notch, and the wheel would stop at every tier
        // (hub report #201).
        let keeps = |line: usize| {
            (first..first + shown).contains(&line)
                && !(line == first && line > 0 && matches!(lines[line - 1], Line::Head(_)))
        };
        if keeps(at) {
            return;
        }
        let mut in_view = (first..first + shown)
            .filter(|index| keeps(*index))
            .filter_map(|index| match lines.get(index) {
                Some(Line::Things(start, end)) => Some((*start, *end)),
                _ => None,
            });
        let target = if at <= first {
            in_view.clone().next()
        } else {
            in_view.next_back()
        };
        if let Some((start, end)) = target {
            self.shader = start + column.min(end - start - 1);
        }
    }

    /// A section's heading at `y`: its tier's name in its colour, how many of it the
    /// player holds, and a line after them in its colour; the stock blade's "Stock", the
    /// favourites' "Favourites" in gold.
    fn heading(&mut self, frame: &Frame, inputs: &Inputs<'_>, group: Group, y: f32, height: f32) {
        let s = frame.s;
        let (name, colour) = match group {
            Group::Tier(tier) => (
                format!("{}  {}", tier.symbol(), tier.label()),
                tier.colour(),
            ),
            Group::Favourites => (format!("{}  Favourites", symbols::EDITION), color::GOLD),
            Group::Stock => (format!("{}  Stock", symbols::SABER), color::HOLO),
        };
        let tier = match group {
            Group::Tier(tier) => Some(tier),
            _ => None,
        };
        let words_y = y + height - 30.0;
        let width = crate::text::display_width(&name, 20.0) * text_scale() + 16.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(LIST_X + 4.0, words_y, width, 26.0),
            20.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut rule_x = LIST_X + 4.0 + width + 4.0;
        if let Some(tier) = tier.filter(|_| inputs.holdings.reason().is_none()) {
            let (held, total) = tier_count(inputs, Some(tier));
            let count = format!("{held} of {total}");
            let count_width = crate::text::display_width(&count, 15.0) * text_scale() + 12.0;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{count}"),
                frame.rect(rule_x, words_y + 3.0, count_width, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            rule_x += count_width + 4.0;
        }
        let end = LIST_X + LIST_WIDTH;
        if rule_x < end {
            let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                rect: frame.rect(rule_x, words_y + 14.0, end - rule_x, 1.0),
                color: color::alpha(colour, 0.3),
            });
        }
    }

    /// The filters' line under the headline: Owned (O), Chroma (C), Favourites (Shift+F,
    /// while the hub keeps marks), and the search field to the list's right edge (/).
    fn toolbar(&mut self, frame: &Frame) {
        let s = frame.s;
        let y = TOOLBAR_Y;
        let mut left = LEFT_X;
        let favourites = format!("{}  Favourites", symbols::EDITION);
        let chips = [
            (FILTER_OWNED_TOKEN, "Owned", self.shader_filter.owned, false),
            (
                FILTER_CHROMA_TOKEN,
                "Chroma",
                self.shader_filter.chroma,
                true,
            ),
            (
                FILTER_FAVOURITE_TOKEN,
                favourites.as_str(),
                self.shader_filter.favourite,
                false,
            ),
        ];
        let shown = if self.marks.kept { 3 } else { 2 };
        for (token, label, lit, chroma) in chips.into_iter().take(shown) {
            let mark = if chroma { MARK + 8.0 } else { 0.0 };
            let width = crate::text::display_width(label, 15.0) * text_scale() + 36.0 + mark;
            let rect = frame.rect(left, y, width, TOOLBAR_HEIGHT);
            let hovered = self.ui.token_hovered(token);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: TOOLBAR_HEIGHT * 0.5 * s,
                    color: color::alpha(color::GOLD, if lit { 0.18 } else { 0.08 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: TOOLBAR_HEIGHT * 0.5 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, 0.4)
                },
            });
            if chroma {
                swatch::chroma_mark(
                    self.ui.draw_list_mut(),
                    frame,
                    left + 14.0,
                    y + (TOOLBAR_HEIGHT - MARK) * 0.5,
                    MARK,
                    1.0,
                );
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(left + mark, y + 4.0, width - mark, TOOLBAR_HEIGHT - 8.0),
                15.0 * s,
                if lit { color::GOLD_BRIGHT } else { color::TEXT },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
            left += width + 8.0;
        }
        let width = (LIST_X + LIST_WIDTH - left).max(120.0);
        let rect = frame.rect(left, y, width, TOOLBAR_HEIGHT);
        let focused = self.search_focus;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: TOOLBAR_HEIGHT * 0.5 * s,
            color: color::alpha(color::SPACE, if focused { 0.8 } else { 0.5 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: TOOLBAR_HEIGHT * 0.5 * s,
            width: 1.2 * s,
            color: if focused {
                color::GOLD
            } else {
                color::alpha(color::HOLO, 0.35)
            },
        });
        let caret = (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let shown = if focused {
            format!(
                "{}{}",
                self.shader_filter.query,
                if caret { "|" } else { "" }
            )
        } else if self.shader_filter.query.is_empty() {
            "Search shaders, smiths, numbers  /".to_owned()
        } else {
            self.shader_filter.query.clone()
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{shown}"),
            frame.rect(left + 16.0, y + 4.0, width - 32.0, TOOLBAR_HEIGHT - 8.0),
            14.0 * s,
            if focused || !self.shader_filter.query.is_empty() {
                color::TEXT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        self.ui.hit_region(SEARCH_TOKEN, rect);
    }

    /// The tiers' strip: All, then each tier the player may see (the rarest first), each
    /// its name in its colour, how many of it the player holds and a bar of that share;
    /// the one shown lit gold. A click (or T) shows that tier alone.
    fn tier_strip(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let tiers = self.tier_chips();
        let count = tiers.len() + 1;
        let width = (LIST_WIDTH - STRIP_GAP * (count - 1) as f32) / count as f32;
        let known = inputs.holdings.reason().is_none();
        let segments = std::iter::once(None).chain(tiers.iter().copied().map(Some));
        for (index, tier) in segments.enumerate() {
            let token = FILTER_TIER_BASE + index as u16;
            let x = LIST_X + index as f32 * (width + STRIP_GAP);
            let rect = frame.rect(x, STRIP_Y, width, STRIP_HEIGHT);
            let lit = self.shader_filter.tier == tier;
            let hovered = self.ui.token_hovered(token);
            let colour = tier.map_or(color::GOLD, Rarity::colour);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 10.0 * s,
                color: if lit {
                    color::alpha(color::GOLD, 0.12)
                } else {
                    color::alpha(color::SPACE, if hovered { 0.75 } else { 0.5 })
                },
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 10.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, if hovered { 0.4 } else { 0.2 })
                },
            });
            let label = tier.map_or_else(
                || "All".to_owned(),
                |tier| format!("{}  {}", tier.symbol(), tier.label()),
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(x + 10.0, STRIP_Y + 5.0, width - 20.0, 24.0),
                16.0 * s,
                match tier {
                    Some(tier) => tier.colour(),
                    None if lit => color::GOLD_BRIGHT,
                    None => color::TEXT,
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let (held, total) = tier_count(inputs, tier);
            let words = if known {
                format!("{held}/{total}")
            } else {
                total.to_string()
            };
            let words_width = crate::text::display_width(&words, 14.0) * text_scale() + 6.0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x + 10.0, STRIP_Y + 30.0, words_width, 18.0),
                14.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let bar_x = x + 14.0 + words_width;
            let bar_width = x + width - 10.0 - bar_x;
            if bar_width > 8.0 {
                let bar = |share: f32| {
                    frame.rect(bar_x, STRIP_Y + 37.0, (bar_width * share).max(4.0), 4.0)
                };
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: bar(1.0),
                    radius: 2.0 * s,
                    color: color::alpha(color::HOLO, 0.16),
                });
                if known && held > 0 && total > 0 {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: bar(held as f32 / total as f32),
                        radius: 2.0 * s,
                        color: colour,
                    });
                }
            }
            self.ui.hit_region(token, rect);
        }
    }

    /// The list's column heads: the ones that sort it (the number's while numbers show,
    /// the name's, the tier's and Yours) answer a click, the order on show lit gold with
    /// its mark.
    fn column_heads(&mut self, frame: &Frame) {
        let s = frame.s;
        let columns = Columns::of(self.shader_numbers, self.trade_known);
        let mut heads: Vec<([f32; 2], &str, Option<ShaderSort>)> = Vec::with_capacity(7);
        if let Some(number) = columns.number {
            heads.push((number, "#", Some(ShaderSort::Number)));
        }
        heads.push((columns.look, "Look", None));
        heads.push((columns.name, "Shader", Some(ShaderSort::Name)));
        heads.push((columns.tier, "Tier", Some(ShaderSort::Tier)));
        heads.push((columns.smith, "Smith", None));
        heads.push((columns.yours, "Yours", Some(ShaderSort::Yours)));
        if let Some(forge) = columns.forge {
            heads.push((forge, "Forge", None));
        }
        let y = LIST_TOP;
        for ([x, width], label, sort) in heads {
            let lit = sort == Some(self.shader_sort);
            let token = sort.map(|sort| {
                SORT_BASE
                    + ShaderSort::ALL
                        .iter()
                        .position(|&other| other == sort)
                        .unwrap_or(0) as u16
            });
            let hovered = token.is_some_and(|token| self.ui.token_hovered(token));
            let end = label == "Forge";
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(x, y + 2.0, width, 22.0),
                15.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else if hovered {
                    color::TEXT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                if end {
                    TextAlign::End
                } else {
                    TextAlign::Start
                },
            );
            if lit {
                // The order's mark: three bars, shortening.
                let mark_x = x + crate::text::display_width(label, 15.0) * text_scale() + 8.0;
                for (bar, length) in [12.0, 8.0, 4.0].into_iter().enumerate() {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: frame.rect(mark_x, y + 7.0 + bar as f32 * 4.0, length, 2.0),
                        radius: 1.0 * s,
                        color: color::GOLD,
                    });
                }
            }
            if let Some(token) = token {
                self.ui
                    .hit_region(token, frame.rect(x - 4.0, y, width + 8.0, HEADS_HEIGHT));
            }
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(LIST_X, y + HEADS_HEIGHT, LIST_WIDTH, 1.0),
            color: color::alpha(color::HOLO, 0.25),
        });
    }

    /// The scroll bar at the right of the wall or the list, when it holds more than it
    /// shows: where the lines shown are among all of them.
    fn scroll_bar(&mut self, frame: &Frame, lines: &[Line], first: usize, shown: usize) {
        if shown >= lines.len() {
            return;
        }
        let s = frame.s;
        let top = if self.grid { LIST_TOP } else { ROWS_TOP };
        let height = LIST_BOTTOM - top;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(SCROLL_X, top, 4.0, height),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let total = lines.len() as f32;
        let thumb = (height * shown as f32 / total).max(24.0);
        let last = last_first(lines, self.grid).max(1) as f32;
        let thumb_top = top + (height - thumb) * (first as f32 / last).min(1.0);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(SCROLL_X, thumb_top, 4.0, thumb),
            radius: 2.0 * s,
            color: color::GOLD,
        });
    }

    /// The two view switches left of the kinds: the list (three bars) and the wall
    /// (four squares), the one on show lit.
    fn view_switches(&mut self, frame: &Frame) {
        let s = frame.s;
        let y = LEAD_Y - 2.0;
        for (index, (token, grid)) in [(LIST_VIEW_TOKEN, false), (GRID_VIEW_TOKEN, true)]
            .into_iter()
            .enumerate()
        {
            let x = SWITCH_X + index as f32 * (SWITCH + 8.0);
            let lit = self.grid == grid;
            let hovered = self.ui.token_hovered(token);
            let rect = frame.rect(x, y, SWITCH, SWITCH);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 9.0 * s,
                    color: color::alpha(color::GOLD, if lit { 0.16 } else { 0.08 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 9.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, 0.3)
                },
            });
            let ink = if lit {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            };
            let marks: &[[f32; 4]] = if grid {
                &[
                    [10.0, 10.0, 7.0, 7.0],
                    [19.0, 10.0, 7.0, 7.0],
                    [10.0, 19.0, 7.0, 7.0],
                    [19.0, 19.0, 7.0, 7.0],
                ]
            } else {
                &[
                    [10.0, 10.0, 16.0, 3.0],
                    [10.0, 16.5, 16.0, 3.0],
                    [10.0, 23.0, 16.0, 3.0],
                ]
            };
            for [mx, my, width, height] in marks {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x + mx, y + my, *width, *height),
                    radius: 1.5 * s,
                    color: ink,
                });
            }
            self.ui.hit_region(token, rect);
        }
    }

    /// Tile `index` of the wall in `rect` (frame pixels): its tier's frame and a band of
    /// its colour along the top, its blade as a strip (alive when owned, a ghost of its
    /// colours when not), its name on two lines, its state and the chroma's mark; the
    /// chosen one lifted over a shadow in a white outline.
    fn wall_tile(
        &mut self,
        frame: &Frame,
        row: Row,
        index: usize,
        rect: [f32; 4],
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let k = text_scale();
        let [x, y, width, height] = rect;
        let chosen = index == self.shader;
        let y = if chosen { y - CHOSEN_LIFT } else { y };
        let token = SHADER_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        let area = frame.rect(x, y, width, height);
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 2.0, y + CHOSEN_LIFT + 4.0, width - 4.0, height),
                radius: 10.0 * s,
                color: color::alpha(color::SPACE, 0.55),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 10.0 * s,
            color: color::alpha(color::SPACE, if chosen || hovered { 0.82 } else { 0.64 }),
        });
        if let Some(tier) = row.tier() {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 10.0, y, width - 20.0, 3.0),
                radius: 1.5 * s,
                color: color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.45 }),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 10.0 * s,
            width: if chosen { 1.6 } else { 1.2 } * s,
            color: row.frame_colour(),
        });
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(x - 3.0, y - 3.0, width + 6.0, height + 6.0),
                radius: 13.0 * s,
                width: 2.0 * s,
                color: color::alpha(CHOSEN_OUTLINE, 0.95),
            });
        } else if row.new {
            // A New one not seen yet stands out in gold, with its sticker (Creyon,
            // 31/10/2026); chosen, the white outline says which is chosen.
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(x - 3.0, y - 3.0, width + 6.0, height + 6.0),
                radius: 13.0 * s,
                width: 1.6 * s,
                color: color::alpha(color::GOLD_BRIGHT, 0.85),
            });
        }
        let strip = [x + 10.0, y + 10.0, width - 20.0, 22.0];
        match row.skin {
            None => swatch::stock_strip(self.ui.draw_list_mut(), frame, strip, inputs.stock),
            Some(skin) => {
                if chosen || hovered {
                    crate::blade_pictures::focus(skin.id);
                }
                let look = self.skins.get(skin.id);
                let _ = swatch::strip(
                    self.ui.draw_list_mut(),
                    frame,
                    strip,
                    look,
                    row.owned,
                    swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                );
            }
        }
        let name_size = 16.0;
        let line = 18.0 * k;
        let (first, second) = name_lines(card_name(row.name()), width - 20.0, name_size);
        let name_colour = match (chosen, row.owned) {
            (true, _) => CHOSEN_OUTLINE,
            (false, true) => color::TEXT,
            (false, false) => color::MUTED,
        };
        for (at, words) in [first, second].iter().enumerate() {
            if words.is_empty() {
                continue;
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x + 10.0, y + 38.0 + at as f32 * line, width - 20.0, line),
                name_size * s,
                name_colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
        let state_y = y + 38.0 + 2.0 * line + 2.0;
        let chroma = row.skin.is_some_and(|skin| skin.chroma);
        let state_width = width - 20.0 - if chroma { MARK + 4.0 } else { 0.0 };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", row.status()),
            frame.rect(x + 10.0, state_y, state_width, 16.0 * k),
            12.5 * s,
            if row.worn {
                color::GOLD_BRIGHT
            } else if row.owned && row.skin.is_some() {
                color::GOLD
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        if chroma {
            swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                x + width - 10.0 - MARK,
                state_y + (16.0 * k - MARK) * 0.5,
                MARK,
                if row.owned { 1.0 } else { 0.55 },
            );
        }
        if let Some(tier) = row.tier() {
            rarity_fx::draw(
                self.ui.draw_list_mut(),
                rarity_fx::Item::of(tier, row.owned, rarity_fx::Size::Small),
                area,
                10.0 * s,
                s,
                seconds,
            );
        }
        // The marks over the blade's right end, the New sticker on the top edge.
        let _ = marks_view::glyph_pill(
            &mut self.ui,
            frame,
            [x + width - 6.0, y + 9.0],
            24.0,
            row.favourite,
            row.locked,
        );
        if row.new {
            let _ = marks_view::new_sticker(&mut self.ui, frame, [x + 8.0, y - 8.0], 17.0);
        }
        self.ui.hit_region(token, area);
    }

    /// Row `index` of the list at `y`: in its columns, the blade's number (while numbers
    /// show), its look as a strip, the chroma's mark and its name, its tier, its smith,
    /// what the player holds and what the Forge asks or gives; the chosen one banded with
    /// a gold mark.
    fn list_row(
        &mut self,
        frame: &Frame,
        row: Row,
        index: usize,
        y: f32,
        inputs: &Inputs<'_>,
        columns: Columns,
    ) {
        let s = frame.s;
        let height = row_height();
        let chosen = index == self.shader;
        let token = SHADER_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        if chosen || hovered {
            kit::band(&mut self.ui, frame, [LIST_X, y, LIST_WIDTH, height]);
        }
        if row.new && !chosen {
            // A New one not seen yet: a gold edge down the row's left, with its sticker
            // (Creyon, 31/10/2026).
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(LIST_X, y + 4.0, 3.0, height - 8.0),
                radius: 1.5 * s,
                color: color::alpha(color::GOLD_BRIGHT, 0.9),
            });
        }
        let middle = y + height * 0.5;
        let words = |rect_x: f32, width: f32, size: f32| {
            frame.rect(rect_x, middle - size * 0.7, width, size * 1.4)
        };
        if let (Some([x, width]), Some(skin)) = (columns.number, row.skin) {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", skin.number_text()),
                words(x, width, 15.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
        let [look_x, look_width] = columns.look;
        let strip = [look_x, middle - 9.0, look_width, 18.0];
        match row.skin {
            None => swatch::stock_strip(self.ui.draw_list_mut(), frame, strip, inputs.stock),
            Some(skin) => {
                if chosen || hovered {
                    crate::blade_pictures::focus(skin.id);
                }
                let look = self.skins.get(skin.id);
                let _ = swatch::strip(
                    self.ui.draw_list_mut(),
                    frame,
                    strip,
                    look,
                    row.owned,
                    swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                );
            }
        }
        let [name_x, mut name_width] = columns.name;
        // The marks at the name column's end: the New sticker last, the star and padlock
        // before it.
        let mut marks_right = name_x + name_width;
        if row.new {
            let width = marks_view::new_sticker(
                &mut self.ui,
                frame,
                [marks_right - 40.0, middle - 9.0],
                18.0,
            );
            marks_right -= width + 6.0 + (40.0 - width).max(0.0);
        }
        let marks_left = marks_view::glyph_pill(
            &mut self.ui,
            frame,
            [marks_right, middle - 11.0],
            22.0,
            row.favourite,
            row.locked,
        );
        if marks_left < name_x + name_width {
            name_width = (marks_left - 6.0 - name_x).max(40.0);
        }
        let chroma = row.skin.is_some_and(|skin| skin.chroma);
        let name_x = if chroma {
            swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                name_x,
                middle - MARK * 0.5,
                MARK,
                if row.owned { 1.0 } else { 0.55 },
            );
            name_x + MARK + 8.0
        } else {
            name_x
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", card_name(row.name())),
            words(name_x, columns.name[0] + name_width - name_x, 18.0),
            18.0 * s,
            match (chosen, row.owned) {
                (true, _) => color::GOLD_BRIGHT,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let [tier_x, tier_width] = columns.tier;
        let (tier_words, tier_colour) = match row.tier() {
            Some(tier) => (
                format!("{}  {}", tier.symbol(), tier.label()),
                color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.7 }),
            ),
            None => ("Your colour".to_owned(), color::QUIET),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{tier_words}"),
            words(tier_x, tier_width, 14.0),
            14.0 * s,
            tier_colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let [smith_x, smith_width] = columns.smith;
        if let Some(skin) = row.skin {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", skin.smith),
                words(smith_x, smith_width, 14.0),
                14.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let [yours_x, yours_width] = columns.yours;
        let (yours, yours_colour) = match (row.skin, row.worn, row.owned) {
            (_, true, _) => (format!("{}  Worn", symbols::SABER), color::GOLD_BRIGHT),
            (None, false, _) => (String::new(), color::QUIET),
            (Some(_), false, true) if row.count > 1 => (format!("x{}", row.count), color::GOLD),
            (Some(_), false, true) => (format!("{}  Yours", symbols::TICK), color::GOLD),
            (Some(_), false, false) => ("-".to_owned(), color::QUIET),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{yours}"),
            words(yours_x, yours_width, 14.0),
            14.0 * s,
            yours_colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let (Some([forge_x, forge_width]), Some(skin)) = (columns.forge, row.skin) {
            let (words_shown, colour) = self.forge_words(skin, row, inputs);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words_shown}"),
                words(forge_x, forge_width, 14.0),
                14.0 * s,
                colour,
                FontWeight::Semibold,
                TextAlign::End,
            );
        }
        self.ui
            .hit_region(token, frame.rect(LIST_X, y, LIST_WIDTH, height));
    }

    /// What the list's Forge column says of `skin`: what a copy breaks down for while the
    /// player holds copies of their own, what crafting it costs while they hold none (a
    /// blade card's lower price, quiet past their kyber shards), nothing for a shader no
    /// one trades.
    fn forge_words(
        &self,
        skin: &'static Unlockable,
        row: Row,
        inputs: &Inputs<'_>,
    ) -> (String, sjk_ui::Color) {
        if !crate::economy::in_trade(skin) {
            return ("-".to_owned(), color::QUIET);
        }
        let unlock = inputs.holdings.unlock(skin.id);
        let own = unlock
            .filter(|unlock| unlock.medal.is_none() && unlock.card.is_none())
            .map_or(0, |unlock| unlock.count);
        if row.owned {
            return if own > 0 {
                (
                    format!(
                        "+{} {}",
                        crate::economy::shards_text(crate::economy::shatter_yield(skin.tier)),
                        symbols::SHARD
                    ),
                    color::HOLO,
                )
            } else {
                ("-".to_owned(), color::QUIET)
            };
        }
        let card = crate::economy::blade_card(
            skin,
            self.held_cards(inputs),
            &crate::cards::faces::current(),
        );
        let (cost, _) = crate::economy::craft_price(skin, card);
        let shards = inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.me.as_ref())
            .map_or(0, |me| me.shards);
        (
            format!("{} {}", crate::economy::shards_text(cost), symbols::SHARD),
            if cost <= shards {
                color::TEXT
            } else {
                color::QUIET
            },
        )
    }

    /// The kinds of shader along the lead: Saber (`owned` of the `total` the player sees),
    /// and Body to come.
    fn kinds(&mut self, frame: &Frame, owned: usize, total: usize) {
        let s = frame.s;
        let x = LEFT_X + 560.0;
        let y = LEAD_Y - 2.0;
        let pills: [(&str, String, bool); 2] = [
            ("Saber", format!("{owned}/{total}"), true),
            ("Body", "soon".to_owned(), false),
        ];
        let mut left = x;
        for (name, count, lit) in pills {
            let width = 132.0;
            let rect = frame.rect(left, y, width, 36.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 18.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, 0.25)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 18.0, y + 5.0, 70.0, 26.0),
                19.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else {
                    color::QUIET
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{count}"),
                frame.rect(left + 70.0, y + 7.0, width - 84.0, 22.0),
                14.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Regular,
                TextAlign::End,
            );
            left += width + 12.0;
        }
    }

    /// How tall the chosen shader's card stands for `row`: its lines as [`Self::shader_card`]
    /// lays them out, the description and the other words wrapped in full.
    fn card_height(&self, row: Row, inputs: &Inputs<'_>) -> f32 {
        let k = text_scale();
        let mut height = 20.0 + 36.0 + 72.0 + FACT_HEIGHT + 16.0;
        height += paragraph_lines(description(row), ABOUT_CHARS) as f32 * ABOUT_SIZE * 1.42 * k;
        height += 6.0;
        height += paragraph_lines(&self.how_line(row, inputs).0, LINE_CHARS) as f32
            * LINE_SIZE
            * 1.42
            * k;
        if let Some(note) = note_of(row, inputs) {
            height += 2.0 + paragraph_lines(&note, LINE_CHARS).min(2) as f32 * LINE_SIZE * 1.42 * k;
        }
        if row.skin.is_some_and(|skin| !row.owned && skin.droppable) {
            height += 12.0 + 30.0 + FACT_HEIGHT;
        }
        if let Some(skin) = row.skin.filter(|_| row.owned)
            && let Some(look) = self.skins.get(skin.id)
        {
            let extras = crate::blade_extras::of(skin.id).map_or(0, |extras| extras.list.len());
            let options = look.options().len() + extras;
            if options > 0 {
                let columns = if options > 4 { 3 } else { 2 };
                height += 10.0 + 28.0 + options.div_ceil(columns) as f32 * 34.0 * k.max(1.0);
            }
            // The line saying what unlocks the extras.
            if extras > 0 && !extras_unlocked(skin.id, inputs) {
                height += LINE_SIZE * 1.42 * k + 4.0;
            }
        }
        height + 12.0 + 44.0 + CARD_PAD
    }

    /// The card's line under the description: since when the shader is the player's, or
    /// how to get it (the stock blade's own words), with its colour.
    fn how_line(&self, row: Row, inputs: &Inputs<'_>) -> (String, sjk_ui::Color) {
        let grant = row.skin.and_then(|skin| inputs.holdings.unlock(skin.id));
        match (row.skin, grant) {
            (None, _) if row.worn => (
                format!("{}  What you hold now: no shader on.", symbols::SABER),
                color::MUTED,
            ),
            (None, _) => ("Wearing it takes your shader off.".to_owned(), color::MUTED),
            (Some(_), Some(grant)) => (
                format!("{}  {}", symbols::TICK, owned_line(grant)),
                color::GOLD,
            ),
            (Some(skin), None) if crate::saber_lab::owns(skin.id) => (
                format!(
                    "{}  Yours in the Saber Lab, on this PC only: other players see your usual blade.",
                    symbols::TICK
                ),
                color::GOLD,
            ),
            (Some(skin), None) => (
                format!("How to get it: {}", crate::terms::say(skin.how_to_get)),
                color::MUTED,
            ),
        }
    }

    /// The chosen shader over the right of the screen: its card at the foot, framed and
    /// banded in its tier's colour and alive as its tier is, as tall as its words need
    /// (it grows upward, never cutting the description: Sol's fit rule of 25/10/2026), and
    /// over it the saber on its own (the stage draws it in [`SABER_AREA`], turned by a
    /// drag; with no stage, its swatch, larger). The card: the tier's mark and name and
    /// the edition, the name and its state, its facts as icons (a chroma, how many are
    /// held, where it comes from, its smith), what it is, since when it is the player's
    /// (with the team's note) or how to get it, its parts and Equip or Unequip.
    fn shader_card(&mut self, frame: &Frame, row: Row, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        self.scale = s;
        self.spin.advance(seconds);
        let card_height = self
            .card_height(row, inputs)
            .min(CARD_BOTTOM - CARD_TOP_MIN);
        let card_y = CARD_BOTTOM - card_height;
        self.saber_area(frame, row, inputs, seconds, card_y);
        let (card_x, card_width) = (CARD_X, CARD_WIDTH);
        let area = frame.rect(card_x, card_y, card_width, card_height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: CARD_RADIUS * s,
            color: color::alpha(color::SPACE, 0.86),
        });
        if let Some(tier) = row.tier() {
            // The tier's band along the top, and its glow under it, as on the wall's tiles.
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(card_x + 24.0, card_y, card_width - 48.0, 4.0),
                radius: 2.0 * s,
                color: color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.5 }),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(card_x + 6.0, card_y + 6.0, card_width - 12.0, 56.0),
                radius: (CARD_RADIUS - 4.0) * s,
                color: color::alpha(tier.colour(), if row.owned { 0.09 } else { 0.04 }),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: CARD_RADIUS * s,
            width: 1.4 * s,
            color: row.frame_colour(),
        });
        if let Some(tier) = row.tier() {
            rarity_fx::draw(
                self.ui.draw_list_mut(),
                rarity_fx::Item::of(tier, row.owned, rarity_fx::Size::Full),
                area,
                CARD_RADIUS * s,
                s,
                seconds,
            );
        }
        let x = card_x + CARD_PAD;
        let width = card_width - CARD_PAD * 2.0;
        let mut y = card_y + 20.0;
        // Its tier's mark and name, or what the stock blade is; its edition at the right.
        let (kind, kind_colour) = match row.tier() {
            Some(tier) => (
                format!("{}  {}{}", tier.symbol(), tier.label(), row.number_words()),
                color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.7 }),
            ),
            None => (format!("{}  Your saber", symbols::SABER), color::HOLO),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{kind}"),
            frame.rect(x, y, width, 28.0),
            21.0 * s,
            kind_colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some(skin) = row.skin {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}  {}",
                    symbols::EDITION,
                    crate::editions::name(skin.edition)
                ),
                frame.rect(x, y + 3.0, width, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::End,
            );
            if row.new {
                let kind_width =
                    crate::text::display_width(&kind, 21.0) * text_scale() * self.widen;
                let _ = marks_view::new_sticker(
                    &mut self.ui,
                    frame,
                    [x + kind_width + 14.0, y + 3.0],
                    22.0,
                );
            }
        }
        y += 36.0;
        let lit = row.owned && row.skin.is_some();
        // The name never runs under the state: it ends short of the pill, ellipsised,
        // whatever face and size the text is drawn in.
        let tag = row.state();
        let (name_rect, tag_rect) = match &tag {
            Some(tag) => {
                let (name, pill) = title_rects(
                    x,
                    y,
                    width,
                    crate::text::display_width(tag, 15.0) * text_scale() + 40.0,
                );
                (name, Some(pill))
            }
            None => ([x, y, width, 62.0], None),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", row.name()),
            frame.rect(name_rect[0], name_rect[1], name_rect[2], name_rect[3]),
            50.0 * s,
            if lit { color::GOLD_BRIGHT } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let (Some(tag), Some([tag_x, tag_y, tag_width, tag_height])) = (&tag, tag_rect) {
            let rect = frame.rect(tag_x, tag_y, tag_width, tag_height);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 13.0 * s,
                width: 1.2 * s,
                color: if lit { color::GOLD } else { color::QUIET },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{tag}"),
                frame.rect(tag_x, tag_y + 2.0, tag_width, 22.0),
                15.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        y += 72.0;
        // Its facts, a chip each: a chroma, the copies held, where it comes from, its
        // smith.
        let mut left = x;
        if row.skin.is_some_and(|skin| skin.chroma) {
            left = self.fact(
                frame,
                [left, y],
                Fact::Chroma,
                "Takes your colour",
                color::HOLO,
            );
        }
        if row.count > 1 {
            let copies = format!("{} copies", row.count);
            left = self.fact(
                frame,
                [left, y],
                Fact::Icon(symbols::COPIES),
                &copies,
                color::HOLO,
            );
        }
        if let Some(skin) = row.skin {
            for (icon, from) in sources(skin) {
                left = self.fact(frame, [left, y], Fact::Icon(icon), &from, color::HOLO);
            }
            let smith = format!("Smith: {}", skin.smith);
            let _ = self.fact(
                frame,
                [left, y],
                Fact::Icon(symbols::SMITH),
                &smith,
                color::HOLO,
            );
        } else {
            let _ = self.fact(
                frame,
                [left, y],
                Fact::Icon(symbols::GIFT),
                "Every player's",
                color::HOLO,
            );
        }
        y += FACT_HEIGHT + 16.0;
        y = paragraph(
            &mut self.ui,
            frame,
            description(row),
            [x, y, width],
            ABOUT_CHARS,
            usize::MAX,
            ABOUT_SIZE,
            color::TEXT,
            FontWeight::Regular,
        );
        y += 6.0;
        let (line, colour) = self.how_line(row, inputs);
        y = paragraph(
            &mut self.ui,
            frame,
            &line,
            [x, y, width],
            LINE_CHARS,
            usize::MAX,
            LINE_SIZE,
            colour,
            FontWeight::Semibold,
        );
        if let Some(note) = note_of(row, inputs) {
            y = paragraph(
                &mut self.ui,
                frame,
                &note,
                [x, y + 2.0, width],
                LINE_CHARS,
                2,
                LINE_SIZE,
                color::MUTED,
                FontWeight::Regular,
            );
        }
        // A locked shader a holocron may drop says which, and how likely.
        if let Some(skin) = row.skin.filter(|skin| !row.owned && skin.droppable) {
            self.drops(frame, skin, [x, y + 12.0]);
        }
        // Equip or Unequip at the foot of the card, then the second saber's, its parts
        // between.
        let button_y = card_y + card_height - CARD_PAD - 44.0;
        let shown_row = self.shader_rows.get(self.shader).copied();
        self.shader_options.clear();
        if let Some(skin) = row.skin.filter(|_| row.owned) {
            self.shader_parts(
                frame,
                skin.id,
                [x, y + 10.0, width, button_y - 12.0],
                inputs,
            );
        }
        let mut left = x;
        // Equip follows the target the switch over the saber chose (the gold pill says
        // which): the first saber, or the second (the staff's other half).
        let label = shown_row.and_then(|shown| {
            if self.both_sides() {
                shown.both.map(|_| {
                    if row.skin.is_none() {
                        ("Stock on both", true)
                    } else if shown.both_worn {
                        ("Unequip both", false)
                    } else {
                        ("Equip both", true)
                    }
                })
            } else if self.second_target {
                shown.second.map(|_| {
                    (
                        if shown.second_worn {
                            "Unequip"
                        } else {
                            "Equip"
                        },
                        !shown.second_worn,
                    )
                })
            } else {
                shown.wear.map(|wear| {
                    let label = if wear.is_empty() && row.skin.is_some() {
                        "Unequip"
                    } else if row.skin.is_none() {
                        "Wear the stock blade"
                    } else {
                        "Equip"
                    };
                    (label, !(wear.is_empty() && row.skin.is_some()))
                })
            }
        });
        if let Some((label, primary)) = label {
            let button_width = (crate::text::display_width(label, 18.0) + 56.0).max(180.0);
            kit::button(
                &mut self.ui,
                frame,
                [x, button_y, button_width, 44.0],
                label,
                primary,
                true,
                false,
                WEAR_TOKEN,
            );
            left += button_width + 12.0;
        }
        // The star and the padlock at the foot's right end, on a hub that keeps them; the
        // padlock only for a shader the player holds.
        let mut right = x + width;
        if let Some(skin) = row.skin.filter(|_| self.marks.kept) {
            let toggles_x = right - marks_view::TOGGLES;
            marks_view::toggle(
                &mut self.ui,
                frame,
                [toggles_x, button_y],
                symbols::EDITION,
                self.marks.favourite(skin.id),
                true,
                FAVOURITE_TOKEN,
            );
            marks_view::toggle(
                &mut self.ui,
                frame,
                [
                    toggles_x + marks_view::TOGGLE + marks_view::TOGGLE_GAP,
                    button_y,
                ],
                symbols::LOCK,
                self.marks.locked(skin.id),
                row.owned || self.marks.locked(skin.id),
                LOCK_TOKEN,
            );
            right = toggles_x - 12.0;
        }
        self.trade_buttons(frame, row, inputs, [left, button_y], right);
    }

    /// The way to the Forge at `[x, y]` on the card's foot, while the hub's economy is
    /// known (`docs/cards.md`, "The Forge"): "Craft in the Forge" for a shader of the
    /// trade not held, "Break down in the Forge" for one of the player's own copies (a
    /// medal's or a card's is not theirs to trade), with what it costs or gives. A click
    /// shows the Forge on that section with the shader chosen.
    fn trade_buttons(
        &mut self,
        frame: &Frame,
        row: Row,
        inputs: &Inputs<'_>,
        [x, y]: [f32; 2],
        right: f32,
    ) {
        self.forge_link = None;
        let Some(skin) = row
            .skin
            .filter(|skin| self.trade_known && crate::economy::in_trade(skin))
        else {
            return;
        };
        let s = frame.s;
        let unlock = inputs.holdings.unlock(skin.id);
        let own = unlock
            .filter(|unlock| unlock.medal.is_none() && unlock.card.is_none())
            .map_or(0, |unlock| unlock.count);
        let (section, label, line) = match (unlock, own) {
            (None, _) => (
                forge::Section::Craft,
                "Craft in the Forge".to_owned(),
                craft_words(skin, self.held_cards(inputs)),
            ),
            (Some(_), 0) => return,
            (Some(_), _) => (
                forge::Section::Break,
                "Break down in the Forge".to_owned(),
                format!(
                    "{}  +{} kyber shards a copy",
                    symbols::SHARD,
                    crate::economy::shards_text(crate::economy::shatter_yield(skin.tier))
                ),
            ),
        };
        let width = (crate::text::display_width(&label, 20.0) * text_scale() + 56.0).max(200.0);
        kit::button(
            &mut self.ui,
            frame,
            [x, y, width, 44.0],
            &label,
            false,
            true,
            false,
            FORGE_LINK_TOKEN,
        );
        self.forge_link = Some((section, skin.id));
        let left = x + width + 12.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(left, y + 11.0, (right - left).max(0.0), 22.0),
            15.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::End,
        );
    }

    /// The room above the card: the stage's lone saber, which a drag turns (a hint says
    /// so, just over the card at `card_y`, and a locked one says it is a preview); with no
    /// stage, the swatch, larger.
    fn saber_area(
        &mut self,
        frame: &Frame,
        row: Row,
        inputs: &Inputs<'_>,
        seconds: f32,
        card_y: f32,
    ) {
        let s = frame.s;
        let [x, y, width, height] = SABER_AREA;
        // The sabers take a drag (and a click) anywhere in their room, but the switch over
        // them is registered after, so a click on a pill reaches the pill: the later of two
        // overlapping regions is the one hit.
        // Above the card only: the card grows upward over the room (main's fit rule).
        let bottom = card_y.min(y + height);
        if self.backstage != Backstage::None {
            self.ui
                .hit_region(SABER_TOKEN, frame.rect(x, y, width, (bottom - y).max(40.0)));
        }
        let switch_bottom = self.target_switch(frame, inputs, [x, y]);
        if self.backstage == Backstage::None {
            let middle = (y + bottom) * 0.5;
            let rect = [x + 40.0, middle - 59.0, width - 80.0, 118.0];
            match row.skin {
                None => swatch::small_stock(self.ui.draw_list_mut(), frame, rect, inputs.stock),
                Some(skin) => {
                    // The chosen one, large.
                    crate::blade_pictures::focus(skin.id);
                    let look = self.skins.get(skin.id);
                    let _ = swatch::small_swatch(
                        self.ui.draw_list_mut(),
                        frame,
                        rect,
                        look,
                        row.owned,
                        seconds,
                        swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                    );
                }
            }
            return;
        }
        let hint_colour = if self.spin.held() {
            color::HOLO
        } else {
            color::QUIET
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}  Drag to turn{}",
                symbols::TURN,
                match self.pair {
                    crate::saber_skins::Pair::TwoSabers => {
                        ",  pick a saber above (or click it), choose a shader, Equip"
                    }
                    crate::saber_skins::Pair::Staff => {
                        ",  pick a half above (or click the staff), choose a shader, Equip"
                    }
                    crate::saber_skins::Pair::None => "",
                }
            ),
            frame.rect(x, bottom - 26.0, width, 22.0),
            15.0 * s,
            hint_colour,
            FontWeight::Semibold,
            TextAlign::End,
        );
        if !row.owned {
            let words = "Preview: not yours yet";
            let pill = crate::text::display_width(words, 16.0) * text_scale() + 40.0;
            let rect = frame.rect(x, switch_bottom, pill, 34.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 17.0 * s,
                color: color::alpha(color::SPACE, 0.6),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 17.0 * s,
                width: 1.2 * s,
                color: color::alpha(color::HOLO, 0.6),
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x, switch_bottom + 6.0, pill, 22.0),
                16.0 * s,
                color::HOLO,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
    }

    /// The target switch at `[x, y]`, over the lone saber, while the player holds two
    /// sabers or a staff: the first saber (or half) and the second, each with what it
    /// wears, the one Equip dresses lit in gold. Answers where what follows it may start
    /// (`y` without a switch).
    fn target_switch(&mut self, frame: &Frame, inputs: &Inputs<'_>, [x, y]: [f32; 2]) -> f32 {
        let s = frame.s;
        let (first_word, second_word) = match self.pair {
            // One saber of one blade: a second skin chosen before waits, and the page says
            // how to show it, in place of the switch.
            crate::saber_skins::Pair::None if !inputs.second.trim().is_empty() => {
                let words = format!(
                    "{}  Your 2nd blade shows with two sabers or a staff: Profile, Saber",
                    symbols::SABER
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{words}"),
                    frame.rect(x, y + 4.0, 700.0, 22.0),
                    15.0 * s,
                    color::QUIET,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
                return y + 34.0;
            }
            crate::saber_skins::Pair::None => return y,
            crate::saber_skins::Pair::TwoSabers => ("1st saber", "2nd saber"),
            crate::saber_skins::Pair::Staff => ("1st half", "Other half"),
        };
        let first = inputs.holdings.worn_blade_skin(inputs.setting);
        let first_name = first.map_or("Stock blade", |skin| skin.name);
        let second_name = match inputs.second.trim() {
            "" => "Same as the 1st",
            crate::looks::STOCK_SECOND => "Stock blade",
            id => unlockables::blade_skin(id)
                .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
                .map_or("Same as the 1st", |skin| skin.name),
        };
        let (width, height) = (236.0, 52.0);
        for (index, (word, name, token)) in [
            (first_word, first_name, TARGET_FIRST_TOKEN),
            (second_word, second_name, TARGET_SECOND_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            let left = x + index as f32 * (width + 10.0);
            let lit = self.both_sides() || self.second_target == (index == 1);
            let hovered = self.ui.token_hovered(token);
            let rect = frame.rect(left, y, width, height);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 14.0 * s,
                color: if lit {
                    color::alpha(color::GOLD, 0.14)
                } else {
                    color::alpha(color::SPACE, 0.62)
                },
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 14.0 * s,
                width: if lit { 1.6 * s } else { 1.2 * s },
                color: if lit {
                    color::GOLD
                } else if hovered {
                    color::alpha(color::HOLO, 0.6)
                } else {
                    color::alpha(color::HOLO, 0.25)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}  {word}", symbols::SABER),
                frame.rect(left + 16.0, y + 6.0, width - 32.0, 18.0),
                13.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 16.0, y + 24.0, width - 32.0, 24.0),
                18.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            self.ui.hit_region(token, rect);
        }
        // The "Both" box, beside the pills: Equip dresses both sides at once.
        let left = x + 2.0 * (width + 10.0) + 8.0;
        let middle = y + height * 0.5;
        let both_word = match self.pair {
            crate::saber_skins::Pair::Staff => "Both halves",
            _ => "Both sabers",
        };
        // Drawn in the face it is measured in, with room to spare (Creyon, 21/10/2026: in
        // the body face it was wider than measured and cut to "Both sab...").
        let label_width = crate::text::display_width(both_word, 16.0) + 16.0;
        let hovered = self.ui.token_hovered(BOTH_TOKEN);
        let after = kit::tick(&mut self.ui, frame, left, middle, self.both, hovered);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{both_word}"),
            frame.rect(after + 10.0, middle - 11.0, label_width, 22.0),
            16.0 * s,
            if self.both { color::GOLD } else { color::MUTED },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        self.ui.hit_region(
            BOTH_TOKEN,
            frame.rect(left - 6.0, y, after - left + label_width + 22.0, height),
        );
        y + height + 10.0
    }

    /// Where a locked `skin` drops, from `[x, y]`: a chip for each holocron tier that may
    /// bring it, in the tier's colour, with the chance an opening of one does (the hub's
    /// odds, [`crate::loot_odds`], split evenly among the droppable shaders of its tier).
    fn drops(&mut self, frame: &Frame, skin: &Unlockable, [x, y]: [f32; 2]) {
        let s = frame.s;
        let rarity = skin.tier as usize;
        let pool = unlockables::blade_skins()
            .filter(|other| other.droppable && other.tier == skin.tier)
            .count();
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}  Where it drops, a chance per opening",
                symbols::HOLOCRON
            ),
            frame.rect(x, y, CARD_WIDTH - CARD_PAD * 2.0, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut left = x;
        for tier in &crate::holocrons::TIERS {
            let chance = crate::cards::loot::shader_in(tier.index, rarity, pool);
            if chance <= 0.0 {
                continue;
            }
            let words = format!("{} {}", tier.label, crate::cards::loot::percent(chance));
            left = self.fact(
                frame,
                [left, y + 30.0],
                Fact::Icon(symbols::RARITY[tier.index + 1]),
                &words,
                tier.colour,
            );
        }
    }

    /// One of the card's facts at `[x, y]`: a chip with its icon (or the chroma's colour
    /// wheel) in `tint` and its words. Returns where the next one starts.
    fn fact(
        &mut self,
        frame: &Frame,
        [x, y]: [f32; 2],
        fact: Fact,
        words: &str,
        tint: sjk_ui::Color,
    ) -> f32 {
        let s = frame.s;
        let width =
            14.0 + FACT_ICON + 8.0 + crate::text::display_width(words, 16.0) * text_scale() + 16.0;
        let rect = frame.rect(x, y, width, FACT_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: FACT_HEIGHT * 0.5 * s,
            color: color::alpha(color::SPACE, 0.55),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: FACT_HEIGHT * 0.5 * s,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.35),
        });
        let icon_x = x + 14.0;
        match fact {
            Fact::Chroma => swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                icon_x,
                y + (FACT_HEIGHT - FACT_ICON) * 0.5,
                FACT_ICON,
                1.0,
            ),
            Fact::Icon(icon) => text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{icon}"),
                frame.rect(icon_x - 2.0, y + 6.0, FACT_ICON + 4.0, 22.0),
                17.0 * s,
                tint,
                FontWeight::Semibold,
                TextAlign::Center,
            ),
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{words}"),
            frame.rect(icon_x + FACT_ICON + 8.0, y + 6.0, width, 22.0),
            16.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        x + width + FACT_GAP
    }

    /// The chosen shader's parts its wearer may switch off, under its words from `y`:
    /// a tick each in two columns (three past four), named as its file names them,
    /// ticked while on; a click or its number key switches it, on the model at once and,
    /// worn, for every SJK player. Its extras follow (`blade_extras.rs`), off until
    /// switched on; while the player lacks the card that unlocks them each is greyed
    /// behind a padlock, takes no click, and a line under them says what unlocks them.
    fn shader_parts(
        &mut self,
        frame: &Frame,
        skin: &'static str,
        [x, y, width, bottom]: [f32; 4],
        inputs: &Inputs<'_>,
    ) {
        let Some(look) = self.skins.get(skin) else {
            return;
        };
        let extras = crate::blade_extras::of(skin);
        let unlocked = extras_unlocked(skin, inputs);
        // Each part, then each extra: id, name and whether it is locked.
        let options: Vec<(&str, &str, bool)> = look
            .options()
            .iter()
            .map(|option| (option.id.as_str(), option.name.as_str(), false))
            .chain(extras.into_iter().flat_map(|extras| {
                extras
                    .list
                    .iter()
                    .map(move |extra| (extra.id.as_str(), extra.name.as_str(), !unlocked))
            }))
            .collect();
        if options.is_empty() {
            return;
        }
        let s = frame.s;
        let columns = if options.len() > 4 { 3 } else { 2 };
        let column = width / columns as f32;
        // The line saying what unlocks the extras takes the foot while they are locked.
        let note = extras.filter(|_| !unlocked).map(|extras| {
            format!(
                "{}  Extras: {}.",
                symbols::LOCK,
                capitalised(&extras.gate.unlocks())
            )
        });
        let bottom = if note.is_some() {
            bottom - LINE_SIZE * 1.42 * text_scale() - 4.0
        } else {
            bottom
        };
        // Never under `bottom` (the card's button): the rows close up when room is short.
        let rows = options.len().div_ceil(columns);
        let step = ((bottom - (y + 28.0)) / rows as f32).clamp(26.0, 36.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Parts"),
            frame.rect(x, y, width, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let shown: Vec<(&'static str, String, bool, String, bool)> = options
            .iter()
            .map(|(id, name, locked)| {
                let on = !locked && crate::saber_skin_options::is_on(inputs.options, skin, id);
                (skin, (*id).to_owned(), on, (*name).to_owned(), *locked)
            })
            .collect();
        let mut last_middle = y + 28.0;
        for (index, (_, _, on, name, locked)) in shown.iter().enumerate() {
            let token = OPTION_BASE + index as u16;
            let left = x + (index % columns) as f32 * column;
            let middle = y + 28.0 + (index / columns) as f32 * step + step * 0.5;
            last_middle = middle;
            let hovered = !locked && self.ui.token_hovered(token);
            let after = kit::tick(&mut self.ui, frame, left, middle, *on, hovered);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}{name}",
                    if *locked {
                        format!("{}  ", symbols::LOCK)
                    } else {
                        String::new()
                    }
                ),
                frame.rect(
                    after + 10.0,
                    middle - 11.0,
                    column - kit::TICK_BOX - 18.0,
                    22.0,
                ),
                16.0 * s,
                if *on { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if !locked && index < OPTION_TOKENS {
                self.ui.hit_region(
                    token,
                    frame.rect(left - 4.0, middle - step * 0.5, column - 8.0, step),
                );
            }
        }
        if let Some(note) = note {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{note}"),
                frame.rect(x, last_middle + step * 0.5 + 4.0, width, LINE_SIZE * 1.42),
                LINE_SIZE * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // A locked extra is listed (for the number keys' order) but never switched.
        self.shader_options = shown
            .into_iter()
            .map(|(skin, option, on, _, locked)| {
                (skin, if locked { String::new() } else { option }, on)
            })
            .collect();
    }
}

/// Whether the player holds the card that unlocks blade skin `skin`'s extras
/// (`blade_extras.rs`), as their own hub profile lists their cards.
fn extras_unlocked(skin: &str, inputs: &Inputs<'_>) -> bool {
    crate::blade_extras::of(skin).is_some_and(|extras| {
        inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.me.as_ref())
            .is_some_and(|me| crate::blade_extras::holds(&me.cards, &extras.gate))
    })
}

/// `text` with its first letter in capitals.
fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// What a fact's chip shows before its words.
#[derive(Clone, Copy)]
enum Fact {
    /// The chroma's colour wheel.
    Chroma,
    /// One of SJK Symbols' icons.
    Icon(char),
}

/// How many shaders of `tier` (`None`: all of them) the player holds and sees.
fn tier_count(inputs: &Inputs<'_>, tier: Option<Rarity>) -> (usize, usize) {
    let mut held = 0;
    let mut total = 0;
    for skin in inputs.holdings.listed() {
        if tier.is_some_and(|tier| tier != skin.tier) {
            continue;
        }
        total += 1;
        if inputs.holdings.unlock(skin.id).is_some() {
            held += 1;
        }
    }
    (held, total)
}

/// What the card says the shader of `row` is (the stock blade's own words).
fn description(row: Row) -> &'static str {
    row.skin.map_or(
        "Your blade in the colour you pick on Profile, Saber.",
        |skin| skin.description,
    )
}

/// The team's note on the grant of `row`'s shader, quoted, when it has one.
fn note_of(row: Row, inputs: &Inputs<'_>) -> Option<String> {
    row.skin
        .and_then(|skin| inputs.holdings.unlock(skin.id))
        .map(|grant| grant.note.as_str())
        .filter(|note| !note.is_empty() && *note != crate::debug_collectibles::NOTE)
        .map(|note| format!("\u{201c}{note}\u{201d}"))
}

/// How many lines `body` takes at `chars` to a line at the ordinary text size (fewer at a
/// larger one).
fn paragraph_lines(body: &str, chars: f32) -> usize {
    let chars = (chars / text_scale()).max(12.0) as usize;
    crate::menu::sjk::wrap(body, chars).count().max(1)
}

/// `body` from (`x`, `y`) in a column `width` wide, wrapped at `chars` to a line (fewer at
/// a larger text size), at most `most` lines (the last ellipsised past them); its lines
/// step with the text size. Returns where the next line would stand.
#[allow(clippy::too_many_arguments)]
fn paragraph(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    body: &str,
    [x, mut y, width]: [f32; 3],
    chars: f32,
    most: usize,
    size: f32,
    colour: sjk_ui::Color,
    weight: FontWeight,
) -> f32 {
    let k = text_scale();
    let step = size * 1.42 * k;
    let chars = (chars / k).max(12.0) as usize;
    let parts: Vec<&str> = crate::menu::sjk::wrap(body, chars).collect();
    for (index, part) in parts.iter().take(most).enumerate() {
        let cut = index + 1 == most && parts.len() > most;
        text(
            canvas,
            TextFamily::Body,
            format_args!("{part}{}", if cut { "..." } else { "" }),
            frame.rect(x, y, width, step),
            size * frame.s,
            colour,
            weight,
            TextAlign::Start,
        );
        y += step;
    }
    y
}

/// What crafting `skin` costs, said beside its "Craft in the Forge": lowered, and the
/// blade card named, while `cards` (the player's) hold a copy of its blade card
/// (21/10/2026).
fn craft_words(skin: &'static Unlockable, cards: &[sjk_identity::CardStack]) -> String {
    let card = crate::economy::blade_card(skin, cards, &crate::cards::faces::current());
    let (cost, off) = crate::economy::craft_price(skin, card);
    let cost = crate::economy::shards_text(cost);
    match off {
        Some((finish, off)) => format!(
            "{}  {cost} kyber shards to craft, {} card -{off} %",
            symbols::SHARD,
            finish.name
        ),
        None => format!("{}  {cost} kyber shards to craft", symbols::SHARD),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::sjk::Frame;

    fn unlock(id: &str) -> sjk_identity::Unlock {
        sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: id.into(),
            granted: 1_791_336_225,
            note: String::new(),
            count: 1,
            medal: None,
            card: None,
        }
    }

    /// The Saber Lab's blade comes right after the stock one, whatever the filter, can be
    /// equipped (on this PC only), and fits the rows' table with every catalogue shader.
    #[test]
    fn the_lab_blade_is_the_row_after_the_stock_blade() {
        let inputs = super::super::tests::inputs(Holdings::Known(&[]), "");
        let plain = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Tier,
            true,
            &crate::shader_marks::Marks::default(),
            false,
        );
        let lab = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Tier,
            true,
            &crate::shader_marks::Marks::default(),
            true,
        );
        assert_eq!(lab.len(), plain.len() + 1);
        assert!(lab.len() <= SHADER_SLOTS);
        assert_eq!(lab[1].skin.map(|skin| skin.id), Some(crate::saber_lab::ID));
        assert!(lab[1].owned && !lab[1].worn);
        let owned_only = ShaderFilter {
            owned: true,
            ..ShaderFilter::default()
        };
        let filtered = Panel::rows(
            &inputs,
            &owned_only,
            ShaderSort::Tier,
            true,
            &crate::shader_marks::Marks::default(),
            true,
        );
        assert_eq!(filtered.len(), 2, "the stock blade and the lab blade");
    }

    #[test]
    fn the_owned_line_names_the_medal_that_brings_a_shader() {
        let grant = |medal: Option<&str>| sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: "saber_glitch".to_owned(),
            granted: 0,
            note: String::new(),
            count: 1,
            medal: medal.map(str::to_owned),
            card: None,
        };
        assert_eq!(owned_line(&grant(None)), "Yours, from the SJK team");
        assert_eq!(
            owned_line(&grant(Some("bug_hunter"))),
            "Yours, with your Bug Hunter medal"
        );
        assert_eq!(
            owned_line(&grant(Some("from_the_future"))),
            "Yours, from the SJK team"
        );
        let dated = sjk_identity::Unlock {
            granted: 1_791_000_000,
            ..grant(Some("early_tester"))
        };
        assert!(owned_line(&dated).starts_with("Yours since "));
        let twice = sjk_identity::Unlock {
            count: 3,
            ..grant(None)
        };
        assert_eq!(owned_line(&twice), "Yours, from the SJK team, 3 held");
        assert_eq!(held_words("Yours", 1), "Yours");
        assert_eq!(held_words("Worn", 2), "Worn x2");
        assert!(owned_line(&dated).ends_with(", with your Tester medal"));
        let card = sjk_identity::Unlock {
            id: "saber_sun".to_owned(),
            card: Some("sjk_unknown_999".to_owned()),
            ..grant(None)
        };
        assert_eq!(owned_line(&card), "Yours, with your sjk_unknown_999 card");
    }

    /// Every shader's name stands on a tile in two lines at most, each inside the tile,
    /// at the ordinary text size.
    #[test]
    fn every_name_fits_a_tile_on_two_lines() {
        for skin in unlockables::blade_skins() {
            let (first, second) = name_lines(card_name(skin.name), TILE_WIDTH - 20.0, 16.0);
            assert!(!first.is_empty(), "{}", skin.name);
            for line in [&first, &second] {
                assert!(
                    crate::text::display_width(line, 16.0) <= TILE_WIDTH - 20.0 + 0.5,
                    "{}: {line}",
                    skin.name
                );
            }
            assert!(!second.ends_with("..."), "{} is cut", skin.name);
        }
    }

    /// The wall lays out a heading to each tier and its tiles six a line; the list a row
    /// to each, headed by tier only while it stands by tier, the stock blade first.
    #[test]
    fn the_lines_head_each_tier_and_hold_the_rows() {
        let held = [unlock("saber_sun")];
        let inputs = super::super::tests::inputs(unlockables::Holdings::Known(&held), "");
        let none = crate::shader_marks::Marks::default();
        let rows = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Tier,
            true,
            &none,
            false,
        );
        let wall = lines_of(&rows, true, true);
        assert_eq!(
            wall[0],
            Line::Head(Group::Stock),
            "the stock blade's heading"
        );
        assert_eq!(wall[1], Line::Things(0, 1));
        assert_eq!(wall[2], Line::Head(Group::Tier(Rarity::Mythical)));
        let things: usize = wall
            .iter()
            .map(|line| match line {
                Line::Things(start, end) => end - start,
                Line::Head(_) => 0,
            })
            .sum();
        assert_eq!(things, rows.len());
        assert!(wall.iter().all(|line| match line {
            Line::Things(start, end) => end - start <= WALL_COLUMNS,
            Line::Head(_) => true,
        }));
        let list = lines_of(&rows, false, true);
        assert_eq!(
            list[0],
            Line::Things(0, 1),
            "no heading over the stock blade"
        );
        assert_eq!(list[1], Line::Head(Group::Tier(Rarity::Mythical)));
        let by_name = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Name,
            false,
            &none,
            false,
        );
        let list = lines_of(&by_name, false, false);
        assert!(list.iter().all(|line| matches!(line, Line::Things(..))));
        assert_eq!(list.len(), by_name.len());
        let names: Vec<_> = by_name[1..]
            .iter()
            .map(|row| row.name().to_lowercase())
            .collect();
        assert!(names.windows(2).all(|pair| pair[0] <= pair[1]), "{names:?}");
        let yours = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Yours,
            false,
            &none,
            false,
        );
        assert_eq!(
            yours[1].skin.map(|skin| skin.id),
            Some("saber_sun"),
            "held first"
        );
    }

    /// Favourites (29/10/2026) stand first after the stock blade: under a heading of their
    /// own on the wall and in the list by tier, simply first in another order; the
    /// favourites chip keeps them alone. A shader's marks reach its row.
    #[test]
    fn favourites_are_listed_first_under_their_heading() {
        let held = [unlock("saber_sun"), unlock("saber_frost")];
        let inputs = super::super::tests::inputs(unlockables::Holdings::Known(&held), "");
        let marks = crate::shader_marks::Marks::shown(
            &["saber_frost", "saber_storm"],
            &["saber_sun"],
            &["saber_frost"],
            None,
        );
        let rows = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Tier,
            true,
            &marks,
            false,
        );
        let ids: Vec<_> = rows
            .iter()
            .map(|row| row.skin.map(|skin| skin.id))
            .collect();
        // Storm is the rarer of the two, so it stands first among them.
        let storm = unlockables::find("saber_storm").unwrap().tier;
        let frost = unlockables::find("saber_frost").unwrap().tier;
        let favourites = if storm >= frost {
            [Some("saber_storm"), Some("saber_frost")]
        } else {
            [Some("saber_frost"), Some("saber_storm")]
        };
        assert_eq!(ids[0], None, "the stock blade first");
        assert_eq!(ids[1..3], favourites);
        let wall = lines_of(&rows, true, true);
        assert_eq!(wall[2], Line::Head(Group::Favourites));
        assert_eq!(wall[3], Line::Things(1, 3));
        assert!(!wall[4..].contains(&Line::Head(Group::Favourites)));
        let frost_row = rows
            .iter()
            .find(|row| row.skin.is_some_and(|skin| skin.id == "saber_frost"))
            .unwrap();
        assert!(frost_row.favourite && frost_row.new && !frost_row.locked);
        let sun_row = rows
            .iter()
            .find(|row| row.skin.is_some_and(|skin| skin.id == "saber_sun"))
            .unwrap();
        assert!(sun_row.locked && !sun_row.favourite);
        // By name: first, without a heading.
        let by_name = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Name,
            false,
            &marks,
            false,
        );
        assert!(by_name[1].favourite && by_name[2].favourite && !by_name[3].favourite);
        assert!(
            lines_of(&by_name, false, false)
                .iter()
                .all(|line| matches!(line, Line::Things(..)))
        );
        // The chip: the favourites alone.
        let only = ShaderFilter {
            favourite: true,
            ..ShaderFilter::default()
        };
        let alone = Panel::rows(&inputs, &only, ShaderSort::Tier, true, &marks, false);
        assert_eq!(alone.len(), 3, "the stock blade and the two favourites");
        // Not held: no padlock in the state, the words say it.
        let storm_row = alone
            .iter()
            .find(|row| row.skin.is_some_and(|skin| skin.id == "saber_storm"))
            .unwrap();
        assert_eq!(storm_row.status(), "Not owned");
        assert_eq!(storm_row.state().as_deref(), Some("Not owned"));
    }

    /// Up and Down on the wall go to the line above or under, past headings, keeping the
    /// column or the nearest one; a scroll window always shows the line of the one chosen.
    #[test]
    fn the_wall_moves_by_lines_and_keeps_the_chosen_in_view() {
        let inputs = super::super::tests::inputs(unlockables::Holdings::Known(&[]), "");
        let rows = Panel::rows(
            &inputs,
            &ShaderFilter::default(),
            ShaderSort::Tier,
            true,
            &crate::shader_marks::Marks::default(),
            false,
        );
        let lines = lines_of(&rows, true, true);
        // The stock blade alone, then Mythical's twelve: rows 1 to 6 and 7 to 12.
        assert_eq!(vertical(&lines, 0, false), 0, "the top");
        assert_eq!(vertical(&lines, 0, true), 1);
        assert_eq!(vertical(&lines, 3, true), 9);
        assert_eq!(vertical(&lines, 9, false), 3);
        assert_eq!(vertical(&lines, 4, false), 0, "to the stock blade's line");
        let last = rows.len() - 1;
        assert_eq!(vertical(&lines, last, true), last, "the bottom");
        for grid in [true, false] {
            let lines = lines_of(&rows, grid, true);
            let mut first = 0;
            for chosen in [0, 40, last, 3, 90, 1] {
                first = line_window(&lines, first, chosen, grid);
                let (at, _) = line_of(&lines, chosen).unwrap();
                let shown = shown_from(&lines, first, grid);
                assert!((first..first + shown).contains(&at), "{grid} {chosen}");
                assert!(first <= last_first(&lines, grid));
            }
        }
    }

    /// The lone saber's room and the card under it keep to the right of the wall and
    /// the list, inside the frame and over the keys; a shader's facts fit along the card.
    #[test]
    fn the_saber_and_its_card_keep_to_the_right() {
        const {
            let [sx, _, sw, _] = SABER_AREA;
            // The card above the keys.
            assert!(CARD_BOTTOM < view::KEYS_Y);
            let list_right = SCROLL_X + 4.0;
            assert!(sx > list_right + 30.0 && CARD_X > list_right + 30.0);
            assert!(sx + sw <= RIGHT_X && CARD_X + CARD_WIDTH <= RIGHT_X);
        }
        let facts = [
            "Takes your colour",
            "2 copies",
            "Contributor medal",
            "Smith: Creyon",
        ]
        .iter()
        .map(|words| 14.0 + FACT_ICON + 8.0 + crate::text::display_width(words, 16.0) + 16.0)
        .sum::<f32>()
            + FACT_GAP * 3.0;
        assert!(facts <= CARD_WIDTH - CARD_PAD * 2.0, "{facts}");
    }

    /// The card holds every shader's whole description, its how-to line and a long note:
    /// it grows upward and never past its highest top, at the ordinary text size.
    #[test]
    fn the_card_holds_every_description_in_full() {
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for skin in unlockables::blade_skins() {
            for owned in [false, true] {
                let held = [sjk_identity::Unlock {
                    note: "a long note from the team ".repeat(4),
                    ..unlock(skin.id)
                }];
                let holdings = if owned {
                    unlockables::Holdings::Known(&held)
                } else {
                    unlockables::Holdings::Known(&[])
                };
                let inputs = super::super::tests::inputs(holdings, "");
                let mut panel = Panel::new();
                panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
                panel.set_backstage(Backstage::World { head: None });
                panel.build(&inputs, &font, [1_920.0, 1_080.0]);
                let Some(row) = panel
                    .shader_list
                    .iter()
                    .position(|other| other.id == skin.id)
                else {
                    // A Unique shader the player does not hold is not listed.
                    assert!(skin.hidden_unless_held() && !owned);
                    continue;
                };
                panel.shader = row + 1;
                panel.build(&inputs, &font, [1_920.0, 1_080.0]);
                let row = Panel::rows(
                    &inputs,
                    &ShaderFilter::default(),
                    ShaderSort::Tier,
                    true,
                    &crate::shader_marks::Marks::default(),
                    false,
                )[panel.shader];
                let height = panel.card_height(row, &inputs);
                assert!(
                    height <= CARD_BOTTOM - CARD_TOP_MIN,
                    "{} ({owned}): {height}",
                    skin.name
                );
                let runs: Vec<&str> = panel.ui.text_runs().collect();
                assert!(
                    runs.iter().all(|run| !run.ends_with("...")),
                    "{} {runs:?}",
                    skin.name
                );
            }
        }
    }

    /// The wall and the list, their tiles, rows and words, fit the canvas over the keys
    /// and left of the saber at 1080 lines, 4K, 4:3 and 21:9, wherever they scroll (the
    /// larger menu text sizes are a world shot's: the style is the whole process's).
    #[test]
    fn the_wall_and_the_list_fit_over_the_keys_at_every_shape() {
        let held = [unlock("saber_sun")];
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            for grid in [true, false] {
                for chosen in [0, 1, 60, super::super::tests::rows_shown() - 1] {
                    let inputs = Inputs {
                        grid,
                        ..super::super::tests::inputs(
                            unlockables::Holdings::Known(&held),
                            "saber_sun",
                        )
                    };
                    let mut panel = Panel::new();
                    panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
                    panel.set_backstage(Backstage::World { head: None });
                    panel.shader = chosen;
                    panel.build(&inputs, &font, viewport);
                    assert!(!panel.ui.overflowed(), "{viewport:?} {grid}");
                    let frame = Frame::new(viewport);
                    let keys = frame.point(0.0, view::KEYS_Y)[1];
                    let right = frame.rect(SCROLL_X + 6.0, 0.0, 0.0, 0.0).x;
                    let (at, _) = line_of(&panel.shader_lines, chosen).unwrap();
                    let first = panel.shader_first;
                    assert!(
                        (first..first + panel.shader_shown).contains(&at),
                        "{chosen}"
                    );
                    let mut shown = 0;
                    for index in 0..SHADER_ROWS {
                        if let Some(area) = panel.ui.rect_for(SHADER_BASE + index as u16) {
                            shown += 1;
                            assert!(area.bottom() <= keys, "{index} {viewport:?}");
                            assert!(area.right() <= right, "{index} {viewport:?}");
                        }
                    }
                    assert!(shown >= if grid { 18 } else { 12 }, "{shown} {grid}");
                    for command in panel.ui.draw_list().commands() {
                        if let DrawCommand::Text { rect, .. } = command {
                            assert!(rect.bottom() <= keys || rect.y >= keys, "{rect:?}");
                            assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                        }
                    }
                }
            }
        }
    }

    /// Sol's report (26/10/2026): reopened after scrolling, the wall started at the worn
    /// shader's tier and hid what stood above it (the stock blade, his Unique shaders)
    /// until a filter put it back at the top. Reopening starts from the top again, the worn
    /// shader chosen and shown; and the hub's answer bringing Unique shaders first keeps
    /// the shader chosen.
    #[test]
    fn reopening_starts_from_the_top_and_the_chosen_stays_chosen() {
        let held = [
            unlock("saber_sun"),
            unlock("saber_thousand_cranes"),
            unlock("saber_hearth"),
        ];
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        let inputs = Inputs {
            grid: true,
            ..super::super::tests::inputs(unlockables::Holdings::Known(&held), "saber_sun")
        };
        let mut panel = Panel::new();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        panel.build(&inputs, &font, [3_840.0, 2_160.0]);
        for _ in 0..8 {
            panel.scroll_shaders(false);
            panel.build(&inputs, &font, [3_840.0, 2_160.0]);
        }
        assert!(panel.shader_first > 0, "scrolled down");
        let _ = panel.close();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        panel.build(&inputs, &font, [3_840.0, 2_160.0]);
        assert_eq!(panel.shader_first, 0, "from the top");
        assert_eq!(
            panel.skin_at(panel.shader).map(|skin| skin.id),
            Some("saber_sun")
        );
        for row in 0..3 {
            assert!(
                panel.ui.rect_for(SHADER_BASE + row).is_some(),
                "row {row} (the stock blade, the Unique ones) shown"
            );
        }
        // The Void chosen while the hub has not answered; its answer brings the Unique
        // shaders first, and the Void stays chosen.
        let only_sun = [unlock("saber_sun")];
        let before = Inputs {
            grid: true,
            ..super::super::tests::inputs(unlockables::Holdings::Known(&only_sun), "saber_sun")
        };
        let mut panel = Panel::new();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        panel.build(&before, &font, [1_920.0, 1_080.0]);
        panel.shader = 1 + panel
            .shader_list
            .iter()
            .position(|skin| skin.id == "saber_void")
            .expect("the Void");
        panel.build(&inputs, &font, [1_920.0, 1_080.0]);
        assert_eq!(
            panel.skin_at(panel.shader).map(|skin| skin.id),
            Some("saber_void")
        );
    }

    /// With Unique's segment (a player who holds a Unique shader) the strip still stands in
    /// a row within the list's width, none over another, and the toolbar's chips and field
    /// too.
    #[test]
    fn the_strip_and_the_toolbar_fit_the_list_with_unique_s_segment() {
        let held = [unlock("saber_thousand_cranes")];
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [[1_920.0, 1_080.0], [3_840.0, 2_160.0], [1_440.0, 1_080.0]] {
            let inputs = super::super::tests::inputs(unlockables::Holdings::Known(&held), "");
            let mut panel = Panel::new();
            panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
            panel.build(&inputs, &font, viewport);
            let frame = Frame::new(viewport);
            let right = frame.rect(LIST_X + LIST_WIDTH, 0.0, 0.0, 0.0).x;
            for tokens in [
                (0..=STRIP_TIERS_WITH_UNIQUE.len() as u16)
                    .map(|index| FILTER_TIER_BASE + index)
                    .collect::<Vec<_>>(),
                vec![FILTER_OWNED_TOKEN, FILTER_CHROMA_TOKEN, SEARCH_TOKEN],
            ] {
                let rects: Vec<_> = tokens
                    .iter()
                    .map(|token| panel.ui.rect_for(*token).expect("a control"))
                    .collect();
                for pair in rects.windows(2) {
                    assert!(pair[0].right() <= pair[1].x, "{viewport:?} {pair:?}");
                }
                assert!(
                    rects[rects.len() - 1].right() <= right + 0.01,
                    "{viewport:?}"
                );
            }
        }
    }
}
