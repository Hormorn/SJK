//! The Collection's Achievements tab: two pages, General and Fun (the toys, Sol,
//! 30/10/2026), switched at the wall's top right or with F. Each is a wall of medallions
//! in bands, each category's run under its name and count (short categories share a
//! band), each medallion
//! ringed by the count in the category's colour, filled and gold once unlocked, the
//! goal inside and the name under it; the one chosen is shown up close on the right with
//! what to do and how far the player is, and under it the three nearest to unlocking.
//! A hidden achievement not unlocked yet stands as a secret (`?`, no count); a retired
//! one stands only once held, with "No longer obtainable".

use super::view::{LEAD_Y, LEFT_X, SHOWCASE_WIDTH, SHOWCASE_X, centred_lines, glow};
use super::*;
use crate::achievements::medallion::{self, Medallion, tint};
use crate::achievements::{Category, Page};
use crate::menu::sjk::{Frame, color, kit, text, wrap};
use crate::menu_widgets::TextFamily;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The wall: where its first band starts, the room a band takes, the medallions a band
/// holds, a medallion's cell and its radius, and how many characters a line of a name
/// under one holds.
const WALL_TOP: f32 = 286.0;
const BAND_STEP: f32 = 172.0;
const COLUMNS: usize = 12;
const CELL: f32 = 85.0;
const RADIUS: f32 = 28.0;
const NAME_CHARS: usize = 13;
/// The medallion up close.
const BIG_RADIUS: f32 = 104.0;
/// How many the Next up list shows.
const NEXT_UP: usize = 3;

/// One category's run of medallions on a band of the wall: the achievements' indices,
/// the band, its first column, and whether it is the category's first run (a category
/// longer than a band goes on in the next).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Group {
    pub(super) category: Category,
    pub(super) members: Vec<usize>,
    pub(super) band: usize,
    pub(super) column: usize,
    pub(super) first: bool,
}

/// A medallion's place on the wall: its achievement's index, band and column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Slot {
    pub(super) index: usize,
    pub(super) band: usize,
    pub(super) column: usize,
}

/// Whether `standing` is on `page` of the board.
fn on_page(standing: &Standing, page: Page) -> bool {
    standing.kind.category.page() == page
}

/// The wall's runs on `page`, of the achievements listed ([`Standing::listed`]): the
/// categories in their own order ([`Category`]), each's achievements in the catalogue's,
/// each after the one before on its band when it fits there with a column between
/// them, else on the next band.
pub(super) fn groups(standings: &[Standing], page: Page) -> Vec<Group> {
    let mut categories: Vec<(Category, Vec<usize>)> = Vec::with_capacity(8);
    for (index, standing) in standings.iter().enumerate().take(WALL_MAX) {
        if !standing.listed() || !on_page(standing, page) {
            continue;
        }
        let category = standing.kind.category;
        match categories.iter_mut().find(|(other, _)| *other == category) {
            Some((_, members)) => members.push(index),
            None => categories.push((category, vec![index])),
        }
    }
    categories.sort_by_key(|(category, _)| *category);
    let mut groups = Vec::with_capacity(categories.len() + 1);
    let (mut band, mut column) = (0, 0);
    // Where each band's last run ends.
    let mut ends: Vec<usize> = Vec::with_capacity(8);
    for (category, members) in categories {
        for (run, chunk) in members.chunks(COLUMNS).enumerate() {
            // The Toy Master's (`achievements::secret_kind`), found one by one, take the
            // first band with room for them rather than one of their own below the rest.
            let earlier = (category == Category::ToyMaster)
                .then(|| {
                    ends.iter().enumerate().find_map(|(at, end)| {
                        let start = if *end == 0 { 0 } else { end + 1 };
                        (start + chunk.len() <= COLUMNS).then_some((at, start))
                    })
                })
                .flatten();
            let (at, start) = match earlier {
                Some(place) => place,
                None => {
                    let mut start = if column == 0 { 0 } else { column + 1 };
                    if start + chunk.len() > COLUMNS {
                        band += 1;
                        start = 0;
                    }
                    column = start + chunk.len();
                    (band, start)
                }
            };
            if ends.len() <= at {
                ends.resize(at + 1, 0);
            }
            ends[at] = start + chunk.len();
            groups.push(Group {
                category,
                members: chunk.to_vec(),
                band: at,
                column: start,
                first: run == 0,
            });
        }
    }
    // Reading order: band by band, left to right.
    groups.sort_by_key(|group| (group.band, group.column));
    groups
}

/// Every medallion's place, in reading order.
pub(super) fn slots(groups: &[Group]) -> impl Iterator<Item = Slot> + '_ {
    groups.iter().flat_map(|group| {
        group
            .members
            .iter()
            .enumerate()
            .map(|(offset, index)| Slot {
                index: *index,
                band: group.band,
                column: group.column + offset,
            })
    })
}

/// The achievement Up (`down` false) or Down from `index` lands on: the next band's, as
/// near its column as it has. An index not on the wall goes to its first.
pub(super) fn vertical(slots: &[Slot], index: usize, down: bool) -> usize {
    let Some(at) = slots.iter().find(|slot| slot.index == index) else {
        return slots.first().map_or(index, |slot| slot.index);
    };
    let band = if down {
        at.band + 1
    } else {
        at.band.saturating_sub(1)
    };
    slots
        .iter()
        .filter(|slot| slot.band == band)
        .min_by_key(|slot| slot.column.abs_diff(at.column))
        .map_or(index, |slot| slot.index)
}

/// The achievement `by` medallions on from `index` in reading order: wrapping round when
/// `wrap`, else stopping at the ends. An index not on the wall goes to its first.
pub(super) fn sideways(slots: &[Slot], index: usize, by: isize, wrap: bool) -> usize {
    let Some(at) = slots.iter().position(|slot| slot.index == index) else {
        return slots.first().map_or(index, |slot| slot.index);
    };
    let count = slots.len() as isize;
    let next = if wrap {
        (at as isize + by).rem_euclid(count)
    } else {
        (at as isize + by).clamp(0, count - 1)
    };
    slots[next as usize].index
}

/// Whether the board may offer `standing` as one to work towards: listed, locked, and
/// no secret (a secret says nothing of how far it is).
fn open(standing: &Standing) -> bool {
    standing.listed() && standing.unlocked.is_none() && !standing.secret()
}

/// The one of `page` to show first: the open one nearest to unlocking, else the first
/// listed.
fn first_choice(standings: &[Standing], page: Page) -> usize {
    standings
        .iter()
        .enumerate()
        .filter(|(_, standing)| open(standing) && on_page(standing, page))
        .max_by(|a, b| a.1.fraction().total_cmp(&b.1.fraction()))
        .or_else(|| {
            standings
                .iter()
                .enumerate()
                .find(|(_, standing)| standing.listed() && on_page(standing, page))
        })
        .map_or(0, |(index, _)| index)
}

/// The open ones of `page` nearest to unlocking but `chosen`, nearest first.
fn next_up(standings: &[Standing], chosen: usize, page: Page) -> Vec<usize> {
    let mut locked: Vec<usize> = (0..standings.len())
        .filter(|index| {
            *index != chosen && open(&standings[*index]) && on_page(&standings[*index], page)
        })
        .collect();
    locked.sort_by(|a, b| {
        standings[*b]
            .fraction()
            .total_cmp(&standings[*a].fraction())
    });
    locked.truncate(NEXT_UP);
    locked
}

impl Panel {
    /// The Achievements tab.
    pub(super) fn achievements(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let standings = inputs.standings;
        if standings.is_empty() {
            self.lead(frame, "No achievements", "");
            return;
        }
        let page = self.achievement_page;
        let groups = groups(standings, page);
        self.achievement_slots.clear();
        self.achievement_slots.extend(slots(&groups));
        if !self
            .achievement_slots
            .iter()
            .any(|slot| slot.index == self.achievement)
        {
            self.achievement = first_choice(standings, page);
        }
        let s = frame.s;
        let (unlocked, total) = crate::achievements::tally(standings);
        self.lead(frame, &format!("{unlocked} of {total} unlocked"), "");
        let fraction = unlocked as f32 / total.max(1) as f32;
        bar(
            &mut self.ui,
            frame,
            [LEFT_X + 300.0, LEAD_Y + 16.0, 300.0, 6.0],
            fraction,
            color::GOLD,
        );
        // The pages' switch, at the wall's top right.
        let labels = Page::ALL.map(Page::name);
        kit::segments(
            &mut self.ui,
            frame,
            LEFT_X + COLUMNS as f32 * CELL - 8.0,
            LEAD_Y + 19.0,
            &labels,
            Page::ALL
                .iter()
                .position(|other| *other == page)
                .unwrap_or(0),
            false,
            ACHIEVEMENT_PAGE_BASE,
        );
        for group in &groups {
            let top = WALL_TOP + group.band as f32 * BAND_STEP;
            let left = LEFT_X + group.column as f32 * CELL;
            let right = left + group.members.len() as f32 * CELL - 8.0;
            let hue = tint(group.category);
            if group.first {
                let members: Vec<&Standing> = groups
                    .iter()
                    .filter(|other| other.category == group.category)
                    .flat_map(|other| other.members.iter())
                    .filter_map(|index| standings.get(*index))
                    .collect();
                let held = members.iter().filter(|s| s.unlocked.is_some()).count();
                // The heading may take the band's room after a short run, up to the next.
                let next = groups
                    .iter()
                    .filter(|other| other.band == group.band && other.column > group.column)
                    .map(|other| other.column - 1)
                    .min()
                    .unwrap_or(COLUMNS);
                let heading_right = (LEFT_X + next as f32 * CELL - 8.0).max(right);
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{}", group.category.name()),
                    frame.rect(left, top, heading_right - left, 28.0),
                    22.0 * s,
                    hue,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
                let name_width = crate::text::display_width(group.category.name(), 22.0);
                // The Toy Master's are listed only once found: their count is all held.
                let count = if group.category == Category::ToyMaster {
                    format!("{held} found")
                } else {
                    format!("{held} of {}", members.len())
                };
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{count}"),
                    frame.rect(left + name_width + 12.0, top + 2.0, 90.0, 24.0),
                    15.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                let rule_x = left + name_width + 24.0 + crate::text::display_width(&count, 15.0);
                if right - rule_x > 12.0 {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                        rect: frame.rect(rule_x, top + 14.0, right - rule_x, 1.0),
                        color: color::alpha(hue, 0.3),
                    });
                }
            }
            for (offset, index) in group.members.iter().enumerate() {
                if let Some(standing) = standings.get(*index) {
                    let x = left + offset as f32 * CELL;
                    self.wall_medallion(frame, standing, *index, x, top + 40.0);
                }
            }
        }
        if let Some(standing) = standings.get(self.achievement) {
            self.achievement_up_close(frame, standing);
            self.next_up(frame, standings);
        }
        centred_lines(
            &mut self.ui,
            frame,
            "Counted on this PC in your matches on servers, and kept on the SJK hub with your identity on.",
            [SHOWCASE_X, 950.0, SHOWCASE_WIDTH],
            96,
            1,
            15.0,
            color::QUIET,
        );
    }

    /// Achievement `index`'s medallion in its cell from (`x`, `top`), its name under it.
    fn wall_medallion(
        &mut self,
        frame: &Frame,
        standing: &Standing,
        index: usize,
        x: f32,
        top: f32,
    ) {
        let s = frame.s;
        let chosen = index == self.achievement;
        let done = standing.unlocked.is_some();
        let centre = [x + CELL * 0.5 - 4.0, top + RADIUS + 4.0];
        if done {
            glow(&mut self.ui, frame, centre, RADIUS * 1.5, color::GOLD, 0.16);
        }
        medallion::draw(
            &mut self.ui,
            Medallion {
                kind: standing.kind,
                centre: frame.point(centre[0], centre[1]),
                radius: RADIUS * s,
                fraction: standing.fraction(),
                done,
            },
        );
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                center: frame.point(centre[0], centre[1]),
                radius: (RADIUS + 9.0) * s,
                width: 2.0 * s,
                start: 0.0,
                sweep: std::f32::consts::TAU,
                color: color::GOLD_BRIGHT,
                knockout: None,
            });
        }
        let mut y = top + RADIUS * 2.0 + 14.0;
        let name = if standing.secret() {
            std::borrow::Cow::Borrowed("Secret")
        } else {
            crate::terms::say(standing.kind.name)
        };
        for part in wrap(&name, NAME_CHARS).take(2) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(centre[0] - CELL * 0.5 - 6.0, y, CELL + 12.0, 20.0),
                14.0 * s,
                match (chosen, done) {
                    (true, _) => color::GOLD_BRIGHT,
                    (false, true) => color::TEXT,
                    (false, false) => color::MUTED,
                },
                FontWeight::Regular,
                TextAlign::Center,
            );
            y += 17.0;
        }
        self.ui.hit_region(
            ACHIEVEMENT_BASE + index as u16,
            frame.rect(centre[0] - CELL * 0.5, top, CELL - 4.0, RADIUS * 2.0 + 52.0),
        );
    }

    /// The achievement chosen, up close on the right.
    fn achievement_up_close(&mut self, frame: &Frame, standing: &Standing) {
        let s = frame.s;
        let kind = standing.kind;
        let done = standing.unlocked.is_some();
        let hue = tint(kind.category);
        let middle = SHOWCASE_X + SHOWCASE_WIDTH * 0.5;
        let centre = [middle, 226.0 + BIG_RADIUS + 8.0];
        glow(
            &mut self.ui,
            frame,
            centre,
            BIG_RADIUS * 2.0,
            if done { color::GOLD } else { hue },
            if done { 0.32 } else { 0.16 },
        );
        medallion::draw(
            &mut self.ui,
            Medallion {
                kind,
                centre: frame.point(centre[0], centre[1]),
                radius: BIG_RADIUS * s,
                fraction: standing.fraction(),
                done,
            },
        );
        let mut y = centre[1] + BIG_RADIUS + 28.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", standing.name()),
            frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 56.0),
            50.0 * s,
            if done {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Center,
        );
        y += 60.0;
        let retired = if kind.retired {
            crate::achievements::RETIRED_NOTE.trim_end_matches('.')
        } else {
            ""
        };
        let dot = if kind.retired { " \u{b7} " } else { "" };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}{dot}{retired}", kind.category.name()),
            frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 22.0),
            16.0 * s,
            hue,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        y += 30.0;
        y = centred_lines(
            &mut self.ui,
            frame,
            &standing.description(),
            [SHOWCASE_X, y, SHOWCASE_WIDTH],
            56,
            2,
            20.0,
            color::TEXT,
        );
        y += 12.0;
        let width = 520.0;
        bar(
            &mut self.ui,
            frame,
            [middle - width * 0.5, y, width, 6.0],
            standing.fraction(),
            if done { color::GOLD } else { hue },
        );
        // A secret says nothing of its goal or how far the player is.
        let progress = if standing.secret() {
            String::new()
        } else {
            format!(
                "{} / {}",
                kind.amount(standing.progress.min(kind.goal)),
                kind.amount(kind.goal)
            )
        };
        let status = match standing.unlocked {
            Some(at) if at > 0 => format!("Unlocked {}", crate::medals::date_text(at)),
            Some(_) => "Unlocked".to_owned(),
            None if standing.secret() => "Not unlocked yet".to_owned(),
            None if standing.waiting => crate::achievements::WAITING_NOTE.to_owned(),
            None => {
                let left = kind.goal.saturating_sub(standing.progress);
                format!("{} more to go", kind.amount(left))
            }
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{progress}"),
            frame.rect(middle - width * 0.5, y + 14.0, width * 0.5, 22.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(middle, y + 14.0, width * 0.5, 22.0),
            16.0 * s,
            if done {
                color::GOLD_BRIGHT
            } else {
                color::MUTED
            },
            FontWeight::Regular,
            TextAlign::End,
        ); // The kyber crystal it brings (21/10/2026), its gem before it in its colour.
        if let Some(tier) = crate::kyber::reward(kind.id) {
            let words = crate::kyber::reward_line(tier);
            let size = 20.0;
            let wide =
                crate::text::display_width(words, size) * crate::text::style::current().scale;
            let left = middle - (wide + 26.0) * 0.5;
            let [gx, gy] = frame.point(left + 9.0, y + 56.0);
            crate::holocrons::gem::draw(
                self.ui.draw_list_mut(),
                [gx, gy],
                9.0 * s,
                tier.colour,
                if done { 1.0 } else { 0.8 },
                crate::holocrons::gem::ROWS,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(left + 26.0, y + 43.0, wide + 20.0, 26.0),
                size * s,
                tier.colour_alpha(if done { 1.0 } else { 0.85 }),
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
    }

    /// The locked achievements nearest to unlocking, under the one up close.
    fn next_up(&mut self, frame: &Frame, standings: &[Standing]) {
        let s = frame.s;
        let width = 560.0;
        let x = SHOWCASE_X + (SHOWCASE_WIDTH - width) * 0.5;
        let near = next_up(standings, self.achievement, self.achievement_page);
        if near.is_empty() {
            return;
        }
        let top = 762.0;
        kit::heading(&mut self.ui, frame, x, top, width, "Next up");
        for (row, index) in near.into_iter().enumerate() {
            let standing = &standings[index];
            let kind = standing.kind;
            let hue = tint(kind.category);
            let y = top + 34.0 + row as f32 * 50.0;
            medallion::draw(
                &mut self.ui,
                Medallion {
                    kind,
                    centre: frame.point(x + 20.0, y + 20.0),
                    radius: 19.0 * s,
                    fraction: standing.fraction(),
                    done: false,
                },
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", crate::terms::say(kind.name)),
                frame.rect(x + 54.0, y + 6.0, 260.0, 28.0),
                22.0 * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            bar(
                &mut self.ui,
                frame,
                [x + 320.0, y + 18.0, 150.0, 4.0],
                standing.fraction(),
                hue,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{} / {}",
                    kind.amount(standing.progress),
                    kind.amount(kind.goal)
                ),
                frame.rect(x + 470.0, y + 8.0, 90.0, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
    }
}

/// A progress bar over `rect` (frame pixels), filled to `fraction` in `fill`.
fn bar(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    fraction: f32,
    fill: sjk_ui::Color,
) {
    let [x, y, width, height] = rect;
    let s = frame.s;
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: frame.rect(x, y, width, height),
        radius: height * 0.5 * s,
        color: color::alpha(color::HOLO, 0.14),
    });
    let filled = width * fraction.clamp(0.0, 1.0);
    if filled > 0.5 {
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, filled, height),
            radius: height * 0.5 * s,
            color: fill,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(id: &str) -> usize {
        crate::achievements::ALL
            .iter()
            .position(|kind| kind.id == id)
            .expect("an achievement")
    }

    /// Standings with `held` unlocked and nothing else.
    fn standings_holding(held: &[&str]) -> Vec<Standing> {
        let held: Vec<sjk_identity::Achievement> = held
            .iter()
            .map(|id| sjk_identity::Achievement {
                id: (*id).to_owned(),
                progress: 1,
                goal: 1,
                unlocked: 1_791_336_225,
                reward: None,
                name: None,
                description: None,
                group: None,
            })
            .collect();
        crate::achievements::Record::default().standings(&held)
    }

    /// The Toy Master's found ones are on the Fun page and take its first band with room
    /// (beside Order 66), not a fifth band below the wall, and the wall still reads band
    /// by band.
    #[test]
    fn the_toy_masters_take_the_first_band_with_room() {
        let found = |id: &str| sjk_identity::Achievement {
            id: id.to_owned(),
            progress: 1,
            goal: 1,
            unlocked: 1_791_336_225,
            reward: None,
            name: Some(id.to_owned()),
            description: Some("Found.".to_owned()),
            group: Some(sjk_identity::wire::TOY_MASTER.to_owned()),
        };
        let standings = crate::achievements::Record::default()
            .standings(&[found("secret_wall_a"), found("secret_wall_b")]);
        let groups = groups(&standings, Page::Fun);
        let master = groups
            .iter()
            .find(|group| group.category == Category::ToyMaster)
            .expect("on the wall");
        let order66 = groups
            .iter()
            .find(|group| group.category == Category::Order66)
            .unwrap();
        assert_eq!(master.band, order66.band);
        assert_eq!(master.column, order66.column + order66.members.len() + 1);
        assert_eq!(master.members.len(), 2);
        assert!(groups.iter().all(|group| group.band <= 3));
        assert!(
            groups
                .windows(2)
                .all(|pair| (pair[0].band, pair[0].column) < (pair[1].band, pair[1].column))
        );
    }

    /// General's five categories fit four bands of twelve: Combat alone, Duels and flags
    /// alone (with the five style masters), Journeys beside Community, Collection alone;
    /// a retired achievement not held is left out and one held takes its place. Fun's
    /// seven toys fit four too, two to a band but the last; Pastimes' two share one. Every listed achievement is on one page.
    #[test]
    fn the_wall_packs_the_categories_into_bands() {
        let standings = standings_holding(&[]);
        let layout = |page: Page| -> Vec<(&str, usize, usize, usize)> {
            groups(&standings, page)
                .iter()
                .map(|g| (g.category.name(), g.band, g.column, g.members.len()))
                .collect()
        };
        assert_eq!(
            layout(Page::General),
            [
                ("Combat", 0, 0, 11),
                ("Duels and flags", 1, 0, 11),
                ("Journeys", 2, 0, 5),
                ("Community", 2, 6, 3),
                ("Collection", 3, 0, 7),
            ]
        );
        assert_eq!(
            layout(Page::Fun),
            [
                ("Cardboard box", 0, 0, 5),
                ("Order 66", 0, 6, 3),
                ("Hax", 1, 0, 7),
                ("Drone", 1, 8, 2),
                ("Boombox", 2, 0, 3),
                ("Monitor", 2, 4, 6),
                ("Box hunt", 3, 0, 4),
            ]
        );
        assert_eq!(
            layout(Page::Pastimes),
            [("Fishing", 0, 0, 5), ("Mining", 0, 6, 5)]
        );
        let all = |standings: &[Standing]| -> Vec<Slot> {
            Page::ALL
                .iter()
                .flat_map(|page| slots(&groups(standings, *page)).collect::<Vec<_>>())
                .collect()
        };
        let slots = all(&standings);
        assert_eq!(slots.len(), crate::achievements::ALL.len() - 1);
        assert!(!slots.iter().any(|slot| slot.index == index_of("surveyor")));
        assert!(
            slots
                .iter()
                .all(|slot| slot.column < COLUMNS && slot.band < 4)
        );
        let holder = standings_holding(&["surveyor"]);
        let slots = all(&holder);
        assert_eq!(slots.len(), crate::achievements::ALL.len());
        assert!(slots.iter().any(|slot| slot.index == index_of("surveyor")));
    }

    fn slots_of(standings: &[Standing]) -> Vec<Slot> {
        slots(&groups(standings, Page::General)).collect()
    }

    #[test]
    fn up_and_down_move_between_the_bands_near_the_column() {
        let slots = slots_of(&standings_holding(&[]));
        let at = |index: usize| *slots.iter().find(|slot| slot.index == index).unwrap();
        // Down from Arsenal (Combat's ninth) lands on Journeys's below it.
        let arsenal = at(index_of("arsenal"));
        let below = at(vertical(&slots, arsenal.index, true));
        assert_eq!((below.band, below.column), (1, arsenal.column));
        assert_eq!(vertical(&slots, below.index, false), arsenal.index);
        // Up from the first band and Down from the last stay.
        assert_eq!(vertical(&slots, arsenal.index, false), arsenal.index);
        let last_band = index_of("full_set");
        assert_eq!(vertical(&slots, last_band, true), last_band);
        // Left and right read the wall in order, Tab wraps round.
        let first = slots[0].index;
        let last = slots.last().unwrap().index;
        assert_eq!(sideways(&slots, first, -1, false), first);
        assert_eq!(sideways(&slots, first, -1, true), last);
        assert_eq!(sideways(&slots, last, 1, true), first);
        assert_eq!(
            sideways(&slots, index_of("arsenal"), 1, false),
            index_of("saber_throw_far"),
            "Combat's newest follow on its band"
        );
        // An index not on the wall (a retired one) goes to the first.
        assert_eq!(vertical(&slots, index_of("surveyor"), true), first);
        assert_eq!(sideways(&slots, index_of("surveyor"), 1, false), first);
    }

    /// A secret is never offered as the next to work towards, nor a retired one.
    #[test]
    fn secrets_and_retired_ones_are_not_next_up() {
        let standings = standings_holding(&[]);
        let secret = index_of("mythical_find");
        assert!(standings[secret].secret());
        assert_ne!(first_choice(&standings, Page::General), secret);
        for page in Page::ALL {
            for chosen in 0..standings.len() {
                let near = next_up(&standings, chosen, page);
                assert!(!near.contains(&secret));
                assert!(!near.contains(&index_of("monitor_triple")));
                assert!(!near.contains(&index_of("surveyor")));
                assert!(near.iter().all(|index| on_page(&standings[*index], page)));
            }
        }
        let fun = first_choice(&standings, Page::Fun);
        assert_eq!(standings[fun].kind.category.page(), Page::Fun);
    }

    #[test]
    fn the_board_opens_on_the_nearest_to_unlocking_and_lists_the_next() {
        let held = [sjk_identity::Achievement {
            id: "first_blood".into(),
            progress: 1,
            goal: 1,
            unlocked: 1_791_336_225,
            reward: None,
            name: None,
            description: None,
            group: None,
        }];
        let standings = crate::achievements::standings(&held);
        let chosen = first_choice(&standings, Page::General);
        assert!(standings[chosen].unlocked.is_none());
        let near = next_up(&standings, chosen, Page::General);
        assert!(near.len() <= NEXT_UP && !near.contains(&chosen));
        assert!(
            near.iter()
                .all(|index| standings[*index].unlocked.is_none())
        );
    }

    /// Every medallion listed on each page stands in the canvas over the keys and
    /// answers the pointer, at 1080 lines, 4K, 4:3 and 21:9, and a retired one not held
    /// has no place; the pages' switch answers too.
    #[test]
    fn the_wall_fits_and_answers_the_pointer() {
        let standings = crate::achievements::standings(&[]);
        let inputs = Inputs {
            standings: &standings,
            ..super::super::tests::inputs(Holdings::Known(&[]), "")
        };
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            for page in Page::ALL {
                let mut panel = Panel::new();
                panel.open(Tab::Achievements, true, true, ReturnTarget::InGame);
                panel.achievement_page = page;
                panel.build(&inputs, &font, viewport);
                assert!(!panel.ui.overflowed(), "{viewport:?}");
                let frame = Frame::new(viewport);
                let keys = frame.point(0.0, super::super::view::KEYS_Y)[1];
                for token in ACHIEVEMENT_PAGE_BASE..ACHIEVEMENT_PAGE_BASE + Page::ALL.len() as u16 {
                    let rect = panel.ui.rect_for(token).expect("a page's switch");
                    assert!(rect.right() <= frame.point(SHOWCASE_X, 0.0)[0]);
                }
                for index in (0..standings.len())
                    .filter(|index| standings[*index].listed() && on_page(&standings[*index], page))
                {
                    let rect = panel
                        .ui
                        .rect_for(ACHIEVEMENT_BASE + index as u16)
                        .expect("a medallion's area");
                    assert!(rect.bottom() <= keys, "{index} {viewport:?}");
                    assert!(rect.right() <= frame.point(SHOWCASE_X, 0.0)[0]);
                }
                let surveyor = index_of("surveyor") as u16;
                assert!(panel.ui.rect_for(ACHIEVEMENT_BASE + surveyor).is_none());
                for command in panel.ui.draw_list().commands() {
                    if let DrawCommand::Text { rect, .. } = command {
                        assert!(rect.bottom() <= keys || rect.y >= keys, "{rect:?}");
                    }
                }
            }
        }
    }
}
