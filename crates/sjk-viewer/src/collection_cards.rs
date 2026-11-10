//! The Collection's Cards tab (`docs/cards.md`, "The Cards tab"): a binder of every card
//! the SJK packs have a face of ([`crate::cards::faces`]), found or not (the player's
//! credits and kyber shards are on the Forge tab). On the left the tiles ([`crate::cards::binder`]), six to a
//! line and three lines shown, scrolling a line at a time, each naming its card: the
//! sealed copies first (a tile per rarity), then every card by set and number, one tile
//! whatever the copies held, in the best finish's light with how many; a card not found
//! yet greyed. Above them the filter's chips (All, Owned or Missing; a rarity; a kind; a
//! finish held) and how many cards it shows. On the right the tile chosen up close, big
//! enough to read, with what it is; Enter or a click shows it large over the tab, the
//! arrows turning to the next (`card_inspect_view`). When the packs have faces of a sealed copy's rarity, Reveal (or R)
//! asks the hub to reveal them, and a line beside it says what came of it. The cards
//! come from the player's own hub profile ([`crate::cards::stacks`]).

use super::view::{LEFT_X, SHOWCASE_WIDTH, SHOWCASE_X, centred_lines};
use super::*;
use crate::cards::binder::{self, Show, Slot, Tile};
use crate::cards::{self, FINISHES, Stack, look};
use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The tiles on the left: their width, the room each takes across and down, how many to
/// a line and the lines shown (the rest scroll into view).
/// Six to a line at 140 (20/10/2026, Sol's request: seven at 122 were too small to
/// tell apart), each naming its card under its picture ([`look::Card::label`]).
const CARD_WIDTH: f32 = 140.0;
const STEP_X: f32 = 158.0;
const STEP_Y: f32 = 206.0;
pub(super) const COLUMNS: usize = 6;
pub(super) const LINES: usize = 3;
/// The first line's top.
const GRID_TOP: f32 = 340.0;
/// The filter's chips: their line and height.
const FILTER_Y: f32 = 294.0;
const FILTER_HEIGHT: f32 = 28.0;
/// The grid's right edge, where the filter's count ends and the scroll bar stands.
const GRID_RIGHT: f32 = LEFT_X + (COLUMNS - 1) as f32 * STEP_X + CARD_WIDTH;
/// The card up close: wide enough that a face's lore reads at 1080p, its top under the
/// row of tabs ([`crate::profile_hub::ROW_BOTTOM`]). It stood in the row, which a large
/// text style stretches under it, and took the Forge tab's clicks (Kushina's report
/// #124); its bottom stays where it was.
const BIG_WIDTH: f32 = 437.0;
const BIG_TOP: f32 = 196.0;
/// Reveal, under the grid, and its height; what it brought reads beside it.
const REVEAL_Y: f32 = 958.0;
const REVEAL_HEIGHT: f32 = 40.0;
/// The card looked at closely ([`Panel::card_inspect_view`]): its width (its height
/// fills most of the frame's), its left and the words' column at its right.
const INSPECT_WIDTH: f32 = 660.0;
const INSPECT_X: f32 = 300.0;
const INSPECT_WORDS_X: f32 = 1_030.0;
const INSPECT_WORDS_WIDTH: f32 = 640.0;
const REVEAL_WIDTH: f32 = 330.0;
/// Room kept at the right of Reveal's line for where the grid has scrolled to.
const ROWS_WIDTH: f32 = 180.0;

/// How many cards the player holds, once the hub answered with the identity on.
pub(super) fn held(inputs: &Inputs<'_>) -> Option<usize> {
    me(inputs).map(|me| cards::total(&cards::stacks(&me.cards)) as usize)
}

/// The own profile, once known with the identity on.
fn me<'a>(inputs: &Inputs<'a>) -> Option<&'a sjk_identity::Profile> {
    inputs
        .snapshot
        .filter(|_| inputs.enabled)
        .and_then(|snapshot| snapshot.me.as_ref())
}

/// What the tab says at its top: how many of the packs' cards are found and, under it,
/// each set's progress ("SJK Alpha 4 / 13").
fn headline(
    inputs: &Inputs<'_>,
    stacks: Option<&[Stack]>,
    progress: &[binder::Progress],
) -> (String, String) {
    let Some(stacks) = stacks.filter(|_| inputs.enabled) else {
        return if inputs.enabled {
            (
                "Contacting the hub...".to_owned(),
                "Your cards show once the SJK hub has answered.".to_owned(),
            )
        } else {
            (
                "Identity is off".to_owned(),
                "Your cards are kept on the SJK hub: switch the identity on in Settings, Network, to see them."
                    .to_owned(),
            )
        };
    };
    let sets: Vec<String> = progress
        .iter()
        .take(4)
        .map(|set| format!("{} {} / {}", set.name, set.found, set.of))
        .collect();
    let mut line = sets.join("   ·   ");
    if progress.len() > 4 {
        line.push_str(&format!("   ·   {} more sets", progress.len() - 4));
    }
    if stacks.is_empty() {
        let open = crate::terms::say(
            "Open a holocron on the Holocrons tab: each holds a card and credits.",
        );
        line = if line.is_empty() {
            open.into_owned()
        } else {
            format!("{line}.   {open}")
        };
    }
    let total = cards::total(stacks);
    let of: usize = progress.iter().map(|set| set.of).sum();
    let found: usize = progress.iter().map(|set| set.found).sum();
    let headline = match (of, total) {
        (0, 0) => "No cards yet".to_owned(),
        (0, 1) => "1 card".to_owned(),
        (0, total) => format!("{total} cards"),
        (of, _) => format!("{found} of {of} cards found"),
    };
    (headline, line)
}

impl Panel {
    /// The Cards tab.
    pub(super) fn cards(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        let stacks = self.holdings(inputs);
        let faces = cards::faces::current();
        self.card_blade = None;
        self.card_blade_craft = false;
        let all = stacks
            .as_deref()
            .map(|stacks| binder::tiles(stacks, &faces))
            .unwrap_or_default();
        let progress = binder::progress(&all, &faces);
        let (headline, line) = headline(inputs, stacks.as_deref(), &progress);
        self.lead(frame, &headline, &line);
        let stacks = stacks.unwrap_or_default();
        let can_reveal = inputs.snapshot.is_some_and(crate::economy::may_act);
        self.revealable = if can_reveal {
            cards::revealable(&stacks, &faces)
        } else {
            0
        };
        let filter = self.card_filter;
        self.card_tiles.clear();
        self.card_tiles.extend(
            all.iter()
                .filter(|tile| filter.passes(tile, &faces))
                .take(CARD_TILES_MAX)
                .cloned(),
        );
        self.cards_shown = self.card_tiles.len();
        self.card = self.card.min(self.cards_shown.saturating_sub(1));
        if all.is_empty() {
            // A dark outline where the first card will lie.
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(LEFT_X, GRID_TOP, CARD_WIDTH, CARD_WIDTH * look::ASPECT),
                radius: CARD_WIDTH * 0.07 * frame.s,
                width: 1.5 * frame.s,
                color: color::alpha(color::HOLO, 0.22),
            });
            return;
        }
        let cards_in = |tiles: &[Tile]| {
            tiles
                .iter()
                .filter(|tile| matches!(tile.slot, Slot::Card(_)))
                .count()
        };
        if self.card_inspect && self.cards_shown > 0 {
            let tiles = std::mem::take(&mut self.card_tiles);
            if let Some(chosen) = tiles.get(self.card) {
                self.card_inspect_view(frame, chosen, &faces, seconds);
            }
            self.card_tiles = tiles;
            return;
        }
        self.card_inspect = false;
        let shown_cards = cards_in(&self.card_tiles);
        self.card_filter_bar(frame, shown_cards, cards_in(&all));
        // The grid scrolls just enough to keep the chosen tile in view.
        let lines = self.cards_shown.div_ceil(COLUMNS);
        self.card_first =
            super::shaders::window(self.card_first, self.card / COLUMNS, lines, LINES);
        let first = self.card_first * COLUMNS;
        let tiles = std::mem::take(&mut self.card_tiles);
        for (index, tile) in tiles.iter().enumerate().skip(first).take(COLUMNS * LINES) {
            self.small_card(frame, tile, &faces, [index, index - first], seconds);
        }
        if tiles.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("No card matches these filters."),
                frame.rect(LEFT_X, GRID_TOP + 10.0, 900.0, 24.0),
                16.0 * frame.s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        self.card_scroll_bar(frame, self.card_first, lines);
        self.reveal_button(frame);
        if let Some(chosen) = tiles.get(self.card) {
            self.card_up_close(frame, chosen, &faces, &inputs.holdings, seconds);
        }
        self.card_tiles = tiles;
    }

    /// The wheel over the Cards tab: a line a notch, the chosen tile kept in view.
    pub(super) fn scroll_cards(&mut self, delta: f32) {
        let lines = self.cards_shown.div_ceil(COLUMNS);
        let last = lines.saturating_sub(LINES);
        self.card_first = if delta > 0.0 {
            self.card_first.saturating_sub(1)
        } else if delta < 0.0 {
            (self.card_first + 1).min(last)
        } else {
            self.card_first
        };
        if self.cards_shown > 0 {
            let line = (self.card / COLUMNS).clamp(self.card_first, self.card_first + LINES - 1);
            self.card = (line * COLUMNS + self.card % COLUMNS).min(self.cards_shown - 1);
        }
    }

    /// The filter over the grid: All, Owned and Missing, then a chip each for the rarity,
    /// the kind and the finish held (a click goes on to the next), and at the right how
    /// many cards it shows of the packs' (`shown` of `of`).
    fn card_filter_bar(&mut self, frame: &Frame, shown: usize, of: usize) {
        let s = frame.s;
        let filter = self.card_filter;
        let mut chips: Vec<(u16, String, sjk_ui::Color, bool, f32)> = Show::ALL
            .iter()
            .enumerate()
            .map(|(index, show)| {
                (
                    CARD_SHOW_BASE + index as u16,
                    show.name().to_owned(),
                    if *show == Show::Owned {
                        color::GOLD
                    } else {
                        color::HOLO
                    },
                    filter.show == *show,
                    [56.0, 80.0, 90.0][index],
                )
            })
            .collect();
        chips.push((
            CARD_RARITY_TOKEN,
            format!(
                "Rarity: {}",
                filter.rarity.map_or("Any", |rarity| rarity.name)
            ),
            filter.rarity.map_or(color::HOLO, |rarity| rarity.colour),
            filter.rarity.is_some(),
            158.0,
        ));
        chips.push((
            CARD_KIND_TOKEN,
            format!("Kind: {}", filter.kind.map_or("Any", |kind| kind.name())),
            color::HOLO,
            filter.kind.is_some(),
            172.0,
        ));
        chips.push((
            CARD_FINISH_TOKEN,
            format!(
                "Finish: {}",
                filter.finish.map_or("Any", |finish| finish.name)
            ),
            if filter.finish.is_some_and(|finish| finish.index == 2) {
                color::GOLD_BRIGHT
            } else {
                color::HOLO
            },
            filter.finish.is_some(),
            128.0,
        ));
        let mut left = LEFT_X;
        for (index, (token, label, colour, lit, width)) in chips.into_iter().enumerate() {
            if index == Show::ALL.len() {
                left += 12.0;
            }
            let rect = frame.rect(left, FILTER_Y, width, FILTER_HEIGHT);
            let hovered = self.ui.token_hovered(token);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: FILTER_HEIGHT * 0.5 * s,
                    color: color::alpha(colour, if lit { 0.2 } else { 0.09 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: FILTER_HEIGHT * 0.5 * s,
                width: 1.2 * s,
                color: color::alpha(colour, if lit { 1.0 } else { 0.4 }),
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(left, FILTER_Y + 4.0, width, FILTER_HEIGHT - 8.0),
                14.0 * s,
                if lit { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
            left += width + 8.0;
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{shown} of {of} cards"),
            frame.rect(left, FILTER_Y + 4.0, GRID_RIGHT - left, FILTER_HEIGHT - 8.0),
            15.0 * s,
            if filter.active() {
                color::TEXT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::End,
        );
    }

    /// The grid's scroll bar at its right, when it holds more lines than it shows.
    fn card_scroll_bar(&mut self, frame: &Frame, first: usize, lines: usize) {
        if lines <= LINES {
            return;
        }
        let s = frame.s;
        let x = GRID_RIGHT + 14.0;
        let height = LINES as f32 * STEP_Y - (STEP_Y - CARD_WIDTH * look::ASPECT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, GRID_TOP, 4.0, height),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let thumb = height * LINES as f32 / lines as f32;
        let top = GRID_TOP + (height - thumb) * first as f32 / (lines - LINES) as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top, 4.0, thumb),
            radius: 2.0 * s,
            color: color::GOLD,
        });
        // Where the window stands, under the grid's right end: "Rows 4-6 of 9".
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Rows {}-{} of {lines}", first + 1, first + LINES),
            frame.rect(GRID_RIGHT - ROWS_WIDTH, REVEAL_Y + 8.0, ROWS_WIDTH, 24.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::End,
        );
    }

    /// The player's cards, once known (a world shot's own in their place).
    pub(super) fn holdings(&self, inputs: &Inputs<'_>) -> Option<Vec<Stack>> {
        #[cfg(test)]
        if let Some((list, _, _)) = &self.preview_cards {
            return Some(cards::stacks(list));
        }
        me(inputs).map(|me| cards::stacks(&me.cards))
    }

    /// How card `key` (0 the card up close, 1 and up the grid's) at `[x, y, width]` (frame
    /// pixels) answers the pointer at `seconds`: eased in while the pointer rests on it,
    /// eased back out when it leaves ([`crate::tile_shine::HoverEase`]).
    fn card_tilt(
        &mut self,
        frame: &Frame,
        [x, y, width]: [f32; 3],
        key: usize,
        seconds: f32,
    ) -> Option<look::Tilt> {
        let rect = frame.rect(x, y, width, width * look::ASPECT);
        self.tilt.update(key, rect, self.pointer, seconds)
    }

    /// Reveal under the grid, when sealed copies can be revealed, and beside it what the
    /// last reveal asked here brought.
    fn reveal_button(&mut self, frame: &Frame) {
        let outcome = self.reveal_asked.and_then(|asked| {
            crate::player_identity::reveal_outcome().filter(|outcome| outcome.serial > asked)
        });
        let mut words_x = LEFT_X;
        if self.revealable > 0 {
            let waiting = self.reveal_asked.is_some() && outcome.is_none();
            let label = if waiting {
                "Revealing...".to_owned()
            } else if self.revealable == 1 {
                "Reveal 1 sealed card".to_owned()
            } else {
                format!("Reveal {} sealed cards", self.revealable)
            };
            crate::menu::sjk::kit::button(
                &mut self.ui,
                frame,
                [LEFT_X, REVEAL_Y, REVEAL_WIDTH, REVEAL_HEIGHT],
                &label,
                true,
                !waiting,
                false,
                REVEAL_TOKEN,
            );
            words_x += REVEAL_WIDTH + 24.0;
        }
        let Some(outcome) = outcome else {
            return;
        };
        let words = reveal_words(&outcome);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{words}"),
            frame.rect(
                words_x,
                REVEAL_Y + 8.0,
                GRID_RIGHT - ROWS_WIDTH - 12.0 - words_x,
                24.0,
            ),
            16.0 * frame.s,
            if outcome.message.is_empty() {
                color::GOLD_BRIGHT
            } else {
                color::EMBER
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// Tile `index` of the list shown, at place `slot` of the lines in view: its card in
    /// the best finish held (greyed when not found yet) and how many when more than one.
    fn small_card(
        &mut self,
        frame: &Frame,
        tile: &Tile,
        faces: &cards::faces::Faces,
        [index, slot]: [usize; 2],
        seconds: f32,
    ) {
        let s = frame.s;
        let x = LEFT_X + (slot % COLUMNS) as f32 * STEP_X;
        let y = GRID_TOP + (slot / COLUMNS) as f32 * STEP_Y;
        let chosen = index == self.card;
        let token = CARD_BASE + index as u16;
        let height = CARD_WIDTH * look::ASPECT;
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(x - 5.0, y - 5.0, CARD_WIDTH + 10.0, height + 10.0),
                radius: (CARD_WIDTH * 0.07 + 5.0) * s,
                width: 2.0 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        let tilt = self.card_tilt(frame, [x, y, CARD_WIDTH], index + 1, seconds);
        let best = tile.best();
        look::draw(
            &mut self.ui,
            frame,
            [x, y, CARD_WIDTH],
            look::Card {
                rarity: tile.rarity,
                finish: tile.finish(),
                edition: "",
                face: shown_face(tile, faces),
                tilt,
                dim: !tile.owned(),
                label: true,
            },
            seconds,
            1.0,
            1.0,
        );
        // Its perks, found or not: a mark each on round badges over the top left corner.
        if let Some(face) = tile.face(faces) {
            let ink = if tile.owned() { 0.9 } else { 0.6 };
            for (place, perk) in face.perks().iter().enumerate() {
                let left = x - 7.0 + place as f32 * 28.0;
                let rect = frame.rect(left, y - 6.0, 25.0, 25.0);
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 12.5 * s,
                    color: color::alpha(color::SPACE, 0.94),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 12.5 * s,
                    width: 1.2 * s,
                    color: color::alpha(tile.tone(faces).colour, ink),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{}", perk.mark),
                    frame.rect(left, y - 5.0, 25.0, 23.0),
                    18.0 * s,
                    color::alpha(tile.tone(faces).colour, ink),
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
        let count = tile.count();
        if count > 1 {
            // The count on a pill over the tile's top right corner.
            let words = format!("x{count}");
            let width = 16.0 + 9.0 * words.chars().count() as f32;
            let rect = frame.rect(x + CARD_WIDTH - width + 6.0, y - 4.0, width, 22.0);
            let solar = best.is_some_and(|stack| stack.finish.index == 2);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 11.0 * s,
                color: color::alpha(color::SPACE, 0.94),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 11.0 * s,
                width: 1.2 * s,
                color: if solar {
                    color::GOLD_BRIGHT
                } else {
                    color::alpha(tile.tone(faces).colour, 0.9)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x + CARD_WIDTH - width + 6.0, y - 2.0, width, 18.0),
                14.0 * s,
                if solar {
                    color::GOLD_BRIGHT
                } else {
                    color::TEXT
                },
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        self.ui.hit_region(
            token,
            frame.rect(x - 5.0, y - 5.0, CARD_WIDTH + 10.0, height + 10.0),
        );
    }

    /// The tile chosen, up close on the right: its card, its name, the copies held (or
    /// that it is not found yet), what it is and when the first was found (or how it is
    /// obtained).
    fn card_up_close(
        &mut self,
        frame: &Frame,
        tile: &Tile,
        faces: &cards::faces::Faces,
        holdings: &crate::unlockables::Holdings<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let middle = SHOWCASE_X + SHOWCASE_WIDTH * 0.5;
        let face = shown_face(tile, faces);
        let tilt = self.card_tilt(
            frame,
            [middle - BIG_WIDTH * 0.5, BIG_TOP, BIG_WIDTH],
            0,
            seconds,
        );
        let shown = tile.in_finish(self.card_look_for());
        let finish = shown.map_or(&FINISHES[0], |stack| stack.finish);
        let edition = shown.map_or("", |stack| stack.edition.as_str());
        look::draw(
            &mut self.ui,
            frame,
            [middle - BIG_WIDTH * 0.5, BIG_TOP, BIG_WIDTH],
            look::Card {
                rarity: tile.rarity,
                finish,
                edition,
                face,
                tilt,
                dim: !tile.owned(),
                label: false,
            },
            seconds,
            1.0,
            1.0,
        );
        // A click on it looks at it closely.
        self.ui.hit_region(
            CARD_CLOSE_UP_TOKEN,
            frame.rect(
                middle - BIG_WIDTH * 0.5,
                BIG_TOP,
                BIG_WIDTH,
                BIG_WIDTH * look::ASPECT,
            ),
        );
        let words = card_words(tile, faces);
        let mut y = BIG_TOP + BIG_WIDTH * look::ASPECT + 18.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", words.name),
            frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 40.0),
            32.0 * s,
            words.colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        y += 46.0;
        if tile.finishes_held().len() > 1 {
            // More than one finish held: their chips stand for the copies' line.
            self.card_look_chips(
                frame,
                [SHOWCASE_X + 30.0, y - 2.0, SHOWCASE_WIDTH - 60.0, 30.0],
                tile,
                finish,
                seconds,
            );
        } else {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", words.held),
                frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 26.0),
                18.0 * s,
                if tile.owned() {
                    color::TEXT
                } else {
                    color::MUTED
                },
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        y += 34.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", words.about),
            frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 24.0),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
        y += 30.0;
        // A blade card held: a button to its shader, where the copy crafts it for less,
        // under one line of words (the hover card has them all) and above the keys.
        let blade = tile
            .owned()
            .then(|| tile.face(faces))
            .flatten()
            .and_then(|face| crate::unlockables::find(face.blade.as_deref()?));
        let next = centred_lines(
            &mut self.ui,
            frame,
            words.last.trim_end(),
            [SHOWCASE_X + 40.0, y, SHOWCASE_WIDTH - 80.0],
            72,
            if blade.is_some() { 1 } else { words.lines },
            15.0,
            color::QUIET,
        );
        if let Some(skin) = blade {
            // A shader of the trade the player lacks is crafted in the Forge.
            let craft = self.trade_known
                && crate::economy::in_trade(skin)
                && holdings.unlock(skin.id).is_none();
            self.card_blade = Some(skin);
            self.card_blade_craft = craft;
            let label = if craft {
                format!("Craft the {} in the Forge", skin.name)
            } else {
                format!("Go to the {}", skin.name)
            };
            let width = crate::text::display_width(&label, 18.0) + 48.0;
            crate::menu::sjk::kit::button(
                &mut self.ui,
                frame,
                [
                    SHOWCASE_X + (SHOWCASE_WIDTH - width) * 0.5,
                    (next + 6.0).min(super::view::KEYS_Y - 52.0),
                    width,
                    36.0,
                ],
                &label,
                false,
                true,
                false,
                CARD_SHADER_TOKEN,
            );
        }
    }

    /// The chips over `[x, y, width, height]` that show the chosen tile in each finish held
    /// (01/11/2026, Sol's request: every copy can be looked at, not only the best): Normal,
    /// Holo and Solar with how many are held in it, `shown` ringed gold, one not held
    /// dimmed and inert. A click (or V) shows the card in it.
    fn card_look_chips(
        &mut self,
        frame: &Frame,
        [x, y, width, height]: [f32; 4],
        tile: &Tile,
        shown: &cards::Finish,
        seconds: f32,
    ) {
        let s = frame.s;
        let gap = 10.0;
        let chip = (width - gap * 2.0) / 3.0;
        for finish in &FINISHES {
            let left = x + finish.index as f32 * (chip + gap);
            let copies: u32 = tile
                .copies
                .iter()
                .filter(|stack| stack.finish.index == finish.index)
                .map(|stack| stack.count)
                .sum();
            let has = copies > 0;
            let token = CARD_LOOK_BASE + finish.index as u16;
            let hovered = has && self.ui.token_hovered(token);
            let rect = frame.rect(left, y, chip, height);
            let active = finish.index == shown.index && has;
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
                    seconds,
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
            let label = if has {
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

    /// The chosen tile looked at closely (20/10/2026, Sol's request: the cards should be
    /// easier to see): over the darkened page, its card as tall as the frame allows
    /// ([`INSPECT_WIDTH`] wide), tilting under the pointer in its finish's light, and at
    /// its right where it stands among the tiles shown, its name, the copies held, what
    /// it is and when the first was found or how it is obtained. The page is not drawn
    /// under it, and the glyphs drawn before it (the header, the headline) are covered.
    /// A click anywhere, Escape, Enter or Space close it; Left and Right, Up and Down or
    /// the wheel go to the previous or next tile.
    fn card_inspect_view(
        &mut self,
        frame: &Frame,
        tile: &Tile,
        faces: &cards::faces::Faces,
        seconds: f32,
    ) {
        let s = frame.s;
        // The whole window, beyond the frame's sides on a wide one.
        let [left, top] = frame.point(0.0, 0.0);
        let window = sjk_ui::Rect::new(0.0, 0.0, left * 2.0 + 1920.0 * s, top * 2.0 + 1080.0 * s);
        self.ui.cover(window);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: window,
            radius: 0.0,
            color: color::alpha(color::SPACE, 0.97),
        });
        self.ui.hit_region(CARD_INSPECT_TOKEN, window);
        let height = INSPECT_WIDTH * look::ASPECT;
        let place = [INSPECT_X, (1_080.0 - height) * 0.5 - 14.0, INSPECT_WIDTH];
        let tilt = self.card_tilt(frame, place, 0, seconds);
        let shown = tile.in_finish(self.card_look_for());
        let finish = shown.map_or(&FINISHES[0], |stack| stack.finish);
        look::draw(
            &mut self.ui,
            frame,
            place,
            look::Card {
                rarity: tile.rarity,
                finish,
                edition: shown.map_or("", |stack| stack.edition.as_str()),
                face: shown_face(tile, faces),
                tilt,
                dim: !tile.owned(),
                label: false,
            },
            seconds,
            1.0,
            1.0,
        );
        let words = card_words(tile, faces);
        let (x, width) = (INSPECT_WORDS_X, INSPECT_WORDS_WIDTH);
        let mut y = 380.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Card {} of {}", self.card + 1, self.cards_shown),
            frame.rect(x, y, width, 24.0),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 34.0;
        let lines = crate::menu::sjk::wrap(&words.name, 30).count().clamp(1, 2);
        for (index, line) in crate::menu::sjk::wrap(&words.name, 30).take(2).enumerate() {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{line}"),
                frame.rect(x, y + index as f32 * 50.0, width, 50.0),
                42.0 * s,
                words.colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
        y += 50.0 * lines as f32 + 14.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", words.held),
            frame.rect(x, y, width, 28.0),
            20.0 * s,
            if tile.owned() {
                color::TEXT
            } else {
                color::MUTED
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 38.0;
        if tile.finishes_held().len() > 1 {
            self.card_look_chips(frame, [x, y, 420.0, 34.0], tile, finish, seconds);
            y += 50.0;
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", words.about),
            frame.rect(x, y, width, 26.0),
            18.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 44.0;
        super::view::left_lines(
            &mut self.ui,
            frame,
            words.last.trim_end(),
            [x, y, width],
            (62, 4),
            17.0,
            color::QUIET,
            TextAlign::Start,
        );
    }
}

/// What the Cards tab says of a tile, up close and looked at closely: its name and the
/// colour it reads in, the copies held (or "Not found yet"), what it is, and last when
/// the first was found or how it is obtained (after what a card's blade brings), with
/// the lines that last takes at most up close.
struct CardWords {
    name: String,
    colour: sjk_ui::Color,
    held: String,
    about: String,
    last: String,
    lines: usize,
}

/// [`CardWords`] for `tile`.
fn card_words(tile: &Tile, faces: &cards::faces::Faces) -> CardWords {
    let face = shown_face(tile, faces);
    let colour = if !tile.owned() {
        color::MUTED
    } else if tile.finish().index == 2 {
        color::GOLD_BRIGHT
    } else {
        tile.tone(faces).colour
    };
    let held = if tile.owned() {
        tile.breakdown()
    } else {
        "Not found yet".to_owned()
    };
    let about = match (&tile.slot, face) {
        (_, Some(face)) => {
            let mut about = format!(
                "{}   ·   {}   ·   {}",
                face.face.tone(),
                face.face.kind.name(),
                face.face.set_name,
            );
            // Its perks' marks, as on its tile.
            let marks: Vec<String> = face
                .face
                .perks()
                .iter()
                .map(|perk| perk.mark.to_string())
                .collect();
            if !marks.is_empty() {
                about = format!("{about}   ·   {}", marks.join(" "));
            }
            about
        }
        (Slot::Sealed, _) => "Sealed: a card is picked for each when it is revealed.".to_owned(),
        _ => "Its face is not in your card pack yet.".to_owned(),
    };
    let last = if tile.owned() {
        match crate::medals::date_text(tile.first()) {
            date if date.is_empty() => String::new(),
            date => format!("First found {date}"),
        }
    } else {
        match face {
            Some(shown) if shown.face.rank.is_some() => card_odds(shown.face, faces),
            _ => rarity_odds(tile.rarity, faces),
        }
    };
    // A card that brings a blade (or whose Solar copies drop one) says so first, a line
    // more before the keys.
    let extra = face.and_then(|face| face.face.brings().or_else(|| face.face.solar_drop()));
    let (last, lines) = match extra {
        Some(brings) => (format!("{brings} {last}"), 3),
        None => (last, 2),
    };
    CardWords {
        name: tile.name(faces),
        colour,
        held,
        about,
        last,
        lines,
    }
}

/// What a reveal brought, beside Reveal: how many cards, and the blade (or how many) their
/// Solar blade cards dropped with them (20/10/2026), or why it failed.
fn reveal_words(outcome: &sjk_identity::RevealOutcome) -> String {
    let count = outcome.revealed.len();
    let cards = match (count, outcome.message.is_empty()) {
        (_, false) => return outcome.message.clone(),
        (0, true) => return "Nothing to reveal yet: no card of their rarity is out.".to_owned(),
        (1, true) => "1 card revealed".to_owned(),
        (count, true) => format!("{count} cards revealed"),
    };
    let blades: Vec<String> = outcome
        .revealed
        .iter()
        .filter_map(|revealed| revealed.unlock.as_ref())
        .map(|unlock| match unlock.name.trim() {
            "" => crate::unlockables::find(&unlock.id).map_or_else(
                || "new saber shader".to_owned(),
                |known| known.name.to_owned(),
            ),
            name => name.to_owned(),
        })
        .collect();
    let with = if count == 1 { "it" } else { "them" };
    match blades.as_slice() {
        [] => format!("{cards}."),
        [one] => format!("{cards}, and the {one} drops with {with}!"),
        // Their names would not fit beside Reveal; the Shaders tab has them.
        many => format!("{cards}, and {} blades drop with them!", many.len()),
    }
}

/// How tile `tile` shows its face, when it is a card the packs have.
fn shown_face<'a>(tile: &Tile, faces: &'a cards::faces::Faces) -> Option<look::Shown<'a>> {
    let Slot::Card(index) = tile.slot else {
        return None;
    };
    let face = faces.list.get(index)?;
    Some(look::Shown { face, index })
}

/// How a card of `rarity` is obtained, with the odds: "From holocrons: about 1 in 1 912
/// holocrons found holds a Legendary card (0.05%), one of the 9 Legendary cards."
pub(super) fn rarity_odds(rarity: &cards::Rarity, faces: &cards::faces::Faces) -> String {
    use crate::cards::loot;
    let chance: f64 = FINISHES
        .iter()
        .map(|finish| loot::kind_any(rarity.index, finish.index))
        .sum();
    let odds = crate::terms::say_owned(match loot::one_in(chance) {
        rare if rare.is_empty() => format!(
            "From holocrons: {} of holocrons found hold a {} card",
            loot::percent(chance),
            rarity.name
        ),
        rare => format!(
            "From holocrons: about {rare} holocrons found holds a {} card ({})",
            rarity.name,
            loot::percent(chance)
        ),
    });
    match faces
        .list
        .iter()
        .filter(|face| face.rarity.id == rarity.id)
        .count()
    {
        0 | 1 => format!("{odds}."),
        pool => format!("{odds}, one of the {pool} {} cards.", rarity.name),
    }
}

/// How a ranked card is obtained, with its own odds (its rarity is not named: a rank
/// stands in its place): "From holocrons: about 1 in 9 560 holocrons found holds this
/// card (0.01%)." The hub picks a set among those with cards of the rarity rolled, then
/// a card of it in that set (`docs/cards.md`, "SJK Blades").
pub(super) fn card_odds(face: &cards::faces::Face, faces: &cards::faces::Faces) -> String {
    use crate::cards::loot;
    let rarity = face.rarity;
    let of_rarity = || {
        faces
            .list
            .iter()
            .filter(|other| other.rarity.id == rarity.id)
    };
    let mut sets: Vec<&str> = of_rarity().map(|other| other.set.as_str()).collect();
    sets.sort_unstable();
    sets.dedup();
    let in_set = of_rarity().filter(|other| other.set == face.set).count();
    let chance: f64 = FINISHES
        .iter()
        .map(|finish| loot::kind_any(rarity.index, finish.index))
        .sum::<f64>()
        / (sets.len().max(1) * in_set.max(1)) as f64;
    crate::terms::say_owned(match loot::one_in(chance) {
        rare if rare.is_empty() => format!(
            "From holocrons: {} of holocrons found hold this card.",
            loot::percent(chance)
        ),
        rare => format!(
            "From holocrons: about {rare} holocrons found holds this card ({}).",
            loot::percent(chance)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::inputs;
    use super::*;
    use crate::unlockables::Holdings;

    /// Every kind of sealed card: each rarity in each finish, the counts rising.
    pub(crate) fn every_kind() -> Vec<sjk_identity::CardStack> {
        let mut list = Vec::new();
        for finish in &cards::FINISHES {
            for rarity in &cards::RARITIES {
                list.push(sjk_identity::CardStack {
                    card: None,
                    rarity: rarity.id.to_owned(),
                    finish: finish.id.to_owned(),
                    edition: "alpha".to_owned(),
                    count: (list.len() as u32 % 4) + 1,
                    first: 1_791_641_100,
                    last: 1_791_641_100,
                });
            }
        }
        list
    }

    /// A reveal says how many cards it turned and the blades Solar blade cards dropped
    /// with them (20/10/2026); a refusal says why.
    #[test]
    fn a_reveal_names_the_blade_it_dropped() {
        let copy = |unlock: Option<(&str, &str)>| sjk_identity::Revealed {
            id: 1,
            card: "sjk_blades_001".to_owned(),
            rarity: "rare".to_owned(),
            finish: "solar".to_owned(),
            edition: "alpha".to_owned(),
            unlock: unlock.map(|(id, name)| sjk_identity::OpenedUnlock {
                finish: None,
                id: id.to_owned(),
                name: name.to_owned(),
                count: 1,
            }),
        };
        let outcome = |revealed| sjk_identity::RevealOutcome {
            serial: 1,
            revealed,
            message: String::new(),
        };
        assert_eq!(reveal_words(&outcome(vec![copy(None)])), "1 card revealed.");
        assert_eq!(
            reveal_words(&outcome(vec![copy(Some(("saber_sun", "Sun blade")))])),
            "1 card revealed, and the Sun blade drops with it!"
        );
        // A nameless one is named from the catalogue.
        assert_eq!(
            reveal_words(&outcome(vec![copy(None), copy(Some(("saber_sun", "")))])),
            "2 cards revealed, and the Sun blade drops with them!"
        );
        assert_eq!(
            reveal_words(&outcome(vec![
                copy(Some(("saber_sun", "Sun blade"))),
                copy(Some(("saber_tidal", "Tidal blade"))),
                copy(None),
            ])),
            "3 cards revealed, and 2 blades drop with them!"
        );
        assert_eq!(
            reveal_words(&outcome(Vec::new())),
            "Nothing to reveal yet: no card of their rarity is out."
        );
        let refused = sjk_identity::RevealOutcome {
            message: "No.".to_owned(),
            ..outcome(Vec::new())
        };
        assert_eq!(reveal_words(&refused), "No.");
    }

    /// A held blade card's button turns to the Shaders tab on its shader, the filter there
    /// cleared so it shows, or to the Forge's Craft on it when the player lacks it; with
    /// no blade card up close it does nothing.
    #[test]
    fn a_blade_cards_button_goes_to_its_shader() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = inputs(Holdings::Known(&[]), "");
        let mut panel = Panel::new();
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        assert_eq!(panel.go_to_card_shader(), PanelAction::None);
        assert_eq!(panel.tab, Tab::Cards);
        panel.shader_filter.tier = Some(crate::unlockables::Rarity::Common);
        panel.card_blade = crate::unlockables::find("saber_sun");
        let _ = panel.go_to_card_shader();
        assert_eq!(panel.tab, Tab::Shaders);
        assert!(!panel.shader_filter.active());
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            panel.skin_at(panel.shader).map(|skin| skin.id),
            Some("saber_sun")
        );
        assert!(panel.shader_focus.is_none());
        assert!(!panel.overflowed());
        // One of the trade the player lacks: the Forge's Craft on it, where the copy
        // crafts it for less.
        panel.show(Tab::Cards);
        panel.card_blade_craft = true;
        let _ = panel.go_to_card_shader();
        assert_eq!(
            (panel.tab, panel.forge_section()),
            (Tab::Forge, super::super::forge::Section::Craft)
        );
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            panel
                .forge
                .craft_list
                .get(panel.forge.craft)
                .map(|item| item.skin.id),
            Some("saber_sun")
        );
    }

    /// Copies of `count` cards `card_000`, `card_001`... in one finish each.
    fn revealed(count: usize) -> Vec<sjk_identity::CardStack> {
        (0..count)
            .map(|index| sjk_identity::CardStack {
                card: Some(format!("card_{index:03}")),
                rarity: "legendary".to_owned(),
                finish: cards::FINISHES[index % 3].id.to_owned(),
                edition: "alpha".to_owned(),
                count: 1 + index as u32 % 3,
                first: 1_791_641_100,
                last: 1_791_641_100,
            })
            .collect()
    }

    /// Sol's request (01/11/2026): every finish held can be looked at. A card held in
    /// Normal and Solar shows Solar first, with a chip for each finish held; V (and a
    /// chip) shows it in the other, up close and looked at closely; another tile starts
    /// on its best again.
    #[test]
    fn every_finish_held_can_be_looked_at() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = inputs(Holdings::Known(&[]), "");
        let stack = |finish: &str, count| sjk_identity::CardStack {
            card: Some("card_000".to_owned()),
            rarity: "legendary".to_owned(),
            finish: finish.to_owned(),
            edition: "alpha".to_owned(),
            count,
            first: 1_791_641_100,
            last: 1_791_641_100,
        };
        let mut held = vec![stack("normal", 2), stack("solar", 1)];
        held.extend(revealed(3).into_iter().skip(1));
        let mut panel = Panel::new();
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        panel.preview_cards = Some((held, 0, 0));
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.card, 0);
        assert_eq!(panel.chosen_finishes_held(), 2);
        let tile = panel.card_tiles[0].clone();
        assert_eq!(
            tile.in_finish(panel.card_look_for()).unwrap().finish.id,
            "solar"
        );
        assert!(panel.ui.rect_for(CARD_LOOK_BASE).is_some(), "Normal's chip");
        assert!(
            panel.ui.rect_for(CARD_LOOK_BASE + 1).is_none(),
            "no Holo held"
        );
        assert!(
            panel.ui.rect_for(CARD_LOOK_BASE + 2).is_some(),
            "Solar's chip"
        );
        panel.cycle_card_look(false);
        assert_eq!(
            tile.in_finish(panel.card_look_for()).unwrap().finish.id,
            "normal"
        );
        panel.cycle_card_look(false);
        assert_eq!(
            tile.in_finish(panel.card_look_for()).unwrap().finish.id,
            "solar"
        );
        // Looked at closely, V goes on and the chips stand by the words.
        panel.card_inspect = true;
        let _ = panel.key(KeyCode::KeyV, false, true);
        assert_eq!(
            tile.in_finish(panel.card_look_for()).unwrap().finish.id,
            "normal"
        );
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert!(panel.ui.rect_for(CARD_LOOK_BASE + 2).is_some());
        assert!(!panel.overflowed());
        // Another tile: its best again, and one finish held offers no chips.
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        assert_eq!(panel.card_look_for(), None);
        panel.card_inspect = false;
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.chosen_finishes_held(), 1);
        assert!(panel.ui.rect_for(CARD_LOOK_BASE).is_none());
    }

    /// Kushina's report #124: the card up close stands under the row of tabs, so the
    /// Forge tab beside Cards takes its clicks. The row reaches the card only with a
    /// large text style (`ui_textScale`, which stretches it towards the frame's right),
    /// so the card's area must start under the whole row's, in each window shape.
    #[test]
    fn the_card_up_close_leaves_the_tabs_their_clicks() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = inputs(Holdings::Known(&[]), "");
        let mut panel = Panel::new();
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        panel.preview_cards = Some((every_kind(), 123_456, 98_765));
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
            panel.build(&shown, &fonts.font, viewport);
            let card = panel.ui.rect_for(CARD_CLOSE_UP_TOKEN).expect("the card");
            let frame = Frame::new(viewport);
            let row = frame.rect(0.0, 0.0, 1_920.0, crate::profile_hub::ROW_BOTTOM);
            assert!(!card.overlaps(row), "{viewport:?}: {card:?} under {row:?}");
            let forge = crate::profile_hub::TOKEN + Tab::Forge.index() as u16;
            let forge = panel.ui.rect_for(forge).expect("the Forge tab");
            assert!(forge.bottom() <= row.bottom() + 0.01, "{viewport:?}");
        }
    }

    /// Every state fits the canvas: no identity, waiting, none, one kind, many sealed
    /// kinds and more tiles than the grid shows; the arrows walk the tiles, the wheel
    /// scrolls and the filter's keys narrow the list.
    #[test]
    fn the_tab_shows_the_tiles_and_fits() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = inputs(Holdings::Known(&[]), "");
        let mut many = every_kind();
        many.extend(revealed(60));
        let states = [
            None,
            Some((Vec::new(), 0, 0)),
            Some((many[..1].to_vec(), 35, 5)),
            Some((every_kind(), 35, 5)),
            Some((many.clone(), 123_456, 98_765)),
        ];
        for state in &states {
            let mut panel = Panel::new();
            panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
            panel.preview_cards = state.clone();
            for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
                panel.build(&shown, &fonts.font, viewport);
                assert!(!panel.overflowed(), "{viewport:?}");
            }
        }
        let mut panel = Panel::new();
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        panel.preview_cards = states[4].clone();
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        // Five sealed rarities, then the sixty unknown cards (no faces in a test).
        assert_eq!(panel.cards_shown, 65);
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        assert_eq!(panel.card, 2);
        let _ = panel.key(KeyCode::ArrowDown, false, true);
        assert_eq!(panel.card, 2 + COLUMNS);
        for _ in 0..40 {
            let _ = panel.key(KeyCode::ArrowDown, false, true);
        }
        // Down keeps the column: on the last line (tiles 60 to 64) it is tile 62.
        assert_eq!(
            panel.card,
            64 / COLUMNS * COLUMNS + 2,
            "Down stops at the last line"
        );
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            panel.card_first,
            65usize.div_ceil(COLUMNS) - LINES,
            "scrolled to it"
        );
        assert!(
            panel.ui.rect_for(CARD_BASE + 64).is_some(),
            "the chosen tile answers the pointer"
        );
        assert!(
            panel.ui.rect_for(CARD_BASE).is_none(),
            "the first is out of view"
        );
        for _ in 0..20 {
            panel.scroll_cards(1.0);
        }
        assert_eq!(panel.card_first, 0);
        assert!(
            panel.card < COLUMNS * LINES,
            "the chosen tile stays in view"
        );
        // Page Down a screen of lines, keeping the column; End and Home to the ends.
        panel.card = 3;
        let _ = panel.key(KeyCode::PageDown, false, true);
        assert_eq!(panel.card, 3 + COLUMNS * LINES);
        let _ = panel.key(KeyCode::PageUp, false, true);
        assert_eq!(panel.card, 3);
        let _ = panel.key(KeyCode::PageUp, false, true);
        assert_eq!(panel.card, 3, "Page Up stops at the first line");
        let _ = panel.key(KeyCode::End, false, true);
        assert_eq!(panel.card, 64);
        let _ = panel.key(KeyCode::PageDown, false, true);
        assert_eq!(panel.card, 64, "Page Down stops at the last line");
        let _ = panel.key(KeyCode::Home, false, true);
        assert_eq!(panel.card, 0);
        // Where the window stands reads under the grid.
        panel.card = 30;
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        let rows = panel
            .ui
            .text_runs()
            .find(|run| run.starts_with("Rows "))
            .map(str::to_owned);
        let first = panel.card_first;
        assert_eq!(
            rows.as_deref(),
            Some(
                format!(
                    "Rows {}-{} of {}",
                    first + 1,
                    first + LINES,
                    65usize.div_ceil(COLUMNS)
                )
                .as_str()
            )
        );
        // Enter looks at the chosen tile closely: the arrows go to the next and back,
        // stopping at the ends, Escape puts it back without closing the Collection,
        // and the page drawn is the card's alone, every state fitting.
        panel.card = 3;
        let _ = panel.key(KeyCode::Enter, false, true);
        assert!(panel.card_inspect);
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
            panel.build(&shown, &fonts.font, viewport);
            assert!(!panel.overflowed(), "{viewport:?}");
        }
        assert!(panel.ui.rect_for(CARD_INSPECT_TOKEN).is_some());
        assert!(
            panel.ui.rect_for(CARD_BASE + 3).is_none(),
            "no tile under it"
        );
        assert!(
            panel.ui.text_runs().any(|run| run == "Card 4 of 65"),
            "where it stands"
        );
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        assert_eq!(panel.card, 4);
        for _ in 0..80 {
            let _ = panel.key(KeyCode::ArrowDown, false, true);
        }
        assert_eq!(panel.card, 64, "the last tile");
        assert!(matches!(
            panel.key(KeyCode::Escape, false, true),
            PanelAction::None
        ));
        assert!(!panel.card_inspect, "Escape puts it back");
        panel.card = 0;
        // T for one rarity: the sealed Mythical tile alone.
        for _ in 0..5 {
            panel.card_filter.cycle_rarity(false);
        }
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.cards_shown, 1);
        assert_eq!(panel.card, 0);
    }
}
