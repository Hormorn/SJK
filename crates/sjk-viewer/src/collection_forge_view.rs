//! How the Forge tab looks ([`super::forge`]): its section chips and the player's
//! balances under the row of tabs, then the section on show. The Shop lays the day's four
//! offers out as large cards over the whole width with the holocrons credits buy under
//! them; Craft and Break down list their shaders on the left (a grid of cards or a list,
//! as `ui_shaderList` says) and show the one chosen big on the right. A card over the
//! page asks before the last copy, every duplicate or the shaders picked go.

use super::forge::*;
use super::view::{LEFT_X, RIGHT_X, left_lines};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::rarity_fx;
use crate::symbols;
use crate::unlockables::Unlockable;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The section chips and the line under them.
const SECTION_Y: f32 = 206.0;
const SECTION_HEIGHT: f32 = 48.0;
const INTRO_Y: f32 = 268.0;
/// The balances at the right of the chips.
const BALANCE_Y: f32 = 194.0;
/// The Shop: the day's heading, the offers' cards and the holocrons for credits.
const DAY_Y: f32 = 322.0;
const OFFERS_Y: f32 = 350.0;
const OFFER: [f32; 2] = [408.0, 444.0];
const OFFER_GAP: f32 = 32.0;
const HOLOCRONS_HEAD_Y: f32 = 826.0;
const HOLOCRONS_Y: f32 = 852.0;
const HOLOCRON_HEIGHT: f32 = 112.0;
/// Craft and Break down: the filter's chips, the list's edges and top, a grid card and a
/// list row.
const CHIPS_Y: f32 = 318.0;
const CHIP_HEIGHT: f32 = 30.0;
const LIST_LEFT: f32 = LEFT_X - 12.0;
const LIST_RIGHT: f32 = LIST_LEFT + 860.0;
const LIST_TOP: f32 = 364.0;
const TILE: [f32; 2] = [164.0, 196.0];
const TILE_GAP: [f32; 2] = [10.0, 14.0];
const ROW_STEP: f32 = 100.0;
const ROW_HEIGHT: f32 = 90.0;
/// The search field, at the right of the line under the chips.
const SEARCH_WIDTH: f32 = 320.0;
/// The view switches' side.
const SWITCH: f32 = 30.0;
/// The chosen shader up close on the right.
const DETAIL: [f32; 4] = [1_000.0, 304.0, RIGHT_X - 1_000.0, 686.0];
const PAD: f32 = 30.0;
const RADIUS: f32 = 16.0;
/// The card asking before the last copy or every duplicate.
/// Its left, top (unused: it centres on its height) and width.
const ASK: [f32; 4] = [520.0, 0.0, 880.0, 0.0];
/// The chosen thing's outline: white, apart from every tier's colour.
const CHOSEN: Color = Color::new(1.0, 1.0, 1.0, 1.0);

/// A section's grid card or list row: where it is, which thing, how it shows.
struct Item<'a> {
    index: usize,
    skin: &'static Unlockable,
    /// The line under its tier: its cost, or what a copy gives.
    price: String,
    /// Whether the price is within reach (lit) or not (quiet).
    reach: bool,
    /// Copies held, for a badge ("x3"); 0 or 1 for none.
    count: u32,
    look: Option<&'a crate::saber_skins::LoadedSkin>,
    /// Its collection marks (29/10/2026): the star, the padlock, the New sticker and the
    /// saber's mark for one worn.
    favourite: bool,
    locked: bool,
    new: bool,
    worn: bool,
    /// The copies picked on Break down while Select is on (05/11/2026), 0 for none.
    picked: u32,
}

impl Panel {
    /// The Forge tab, swatches alive at `seconds`.
    pub(super) fn forge(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        seconds: f32,
        viewport: [f32; 2],
    ) {
        self.forge_lists(inputs);
        if let Some(ask) = self.forge.ask {
            self.forge_ask(frame, ask, inputs, seconds, viewport);
            return;
        }
        self.forge_sections(frame);
        self.forge_balances(frame);
        let reason = match (self.forge.data.closed.clone(), inputs.holdings.reason()) {
            (Closed::IdentityOff, _) => Some((
                "The Forge needs the SJK identity",
                "Switch it on in Settings, Network: your shaders and kyber shards are kept on \
                 the SJK hub.",
            )),
            (_, Some(_)) | (Closed::Waiting, _) => Some((
                "Contacting the SJK hub...",
                "The Forge shows once the hub says what you hold.",
            )),
            (Closed::NoEconomy, _) => Some((
                "The Forge comes with the hub's shop",
                "This SJK hub has no kyber shards or shop yet: crafting, breaking down and \
                 the daily offers come with them.",
            )),
            (Closed::Open, None) => None,
        };
        if let Some((title, line)) = reason {
            self.forge_intro(frame, "");
            self.forge_empty(
                frame,
                [LEFT_X, OFFERS_Y, RIGHT_X - LEFT_X, 420.0],
                title,
                line,
            );
            return;
        }
        match self.forge.section {
            Section::Shop => self.forge_shop(frame, seconds),
            Section::Craft => self.forge_craft(frame, inputs, seconds),
            Section::Break => self.forge_break(frame, inputs, seconds),
        }
    }

    /// Read the sections' lists for this frame from what the player holds and the filters;
    /// keep each choice in range and in view.
    fn forge_lists(&mut self, inputs: &Inputs<'_>) {
        self.follow_marks(inputs);
        if !self.marks.kept {
            // No favourites on a hub that keeps no marks.
            self.forge.break_filter.shaders.favourite = false;
        }
        let shards = self.forge.data.shards.unwrap_or(0);
        let craft = &self.forge.craft_filter;
        self.forge.craft_list = craftable(&inputs.holdings, self.held_cards(inputs), &self.marks)
            .into_iter()
            .filter(|item| craft.shaders.passes(item.skin, false, item.favourite))
            .filter(|item| !craft.only || item.cost <= shards)
            .collect();
        let all = owned(&inputs.holdings, self.forge.data.last, &self.marks);
        self.forge.spare = duplicates(&all);
        self.forge.break_all = all.clone();
        self.forge.prune_picks();
        let breaks = &self.forge.break_filter;
        self.forge.break_list = all
            .into_iter()
            .filter(|item| breaks.shaders.passes(item.skin, true, item.favourite))
            .filter(|item| !breaks.only || item.count > 1)
            .collect();
        if let Some(id) = self.forge.want.take() {
            let at = match self.forge.section {
                Section::Craft => self
                    .forge
                    .craft_list
                    .iter()
                    .position(|item| item.skin.id == id),
                Section::Break => self
                    .forge
                    .break_list
                    .iter()
                    .position(|item| item.skin.id == id),
                Section::Shop => None,
            };
            if let Some(at) = at {
                match self.forge.section {
                    Section::Craft => self.forge.craft = at,
                    _ => self.forge.breaks = at,
                }
            }
        }
        let forge = &mut self.forge;
        forge.craft = forge.craft.min(forge.craft_list.len().saturating_sub(1));
        forge.breaks = forge.breaks.min(forge.break_list.len().saturating_sub(1));
        forge.shop_selected = forge
            .shop_selected
            .min(forge.shop_choices().saturating_sub(1));
        let (per, shown) = if self.grid {
            (COLUMNS, LINES)
        } else {
            (1, ROWS)
        };
        forge.craft_first = shaders::window(
            forge.craft_first,
            forge.craft / per,
            forge.craft_list.len().div_ceil(per),
            shown,
        );
        forge.break_first = shaders::window(
            forge.break_first,
            forge.breaks / per,
            forge.break_list.len().div_ceil(per),
            shown,
        );
        // A New one chosen is seen once the choice leaves it.
        let chosen = self.forge.chosen_shader().map(|(id, _)| id);
        self.view(chosen);
    }

    /// The section chips: Shop, Craft and Break down, the one on show gold; Break down
    /// says how many duplicates wait.
    fn forge_sections(&mut self, frame: &Frame) {
        let s = frame.s;
        let mut x = LEFT_X;
        for (index, section) in Section::ALL.into_iter().enumerate() {
            let token = SECTION_BASE + index as u16;
            let lit = section == self.forge.section;
            let hovered = self.ui.token_hovered(token);
            let badge = match section {
                Section::Break if self.forge.spare.0 > 0 => Some(self.forge.spare.0.to_string()),
                _ => None,
            };
            let label_width = crate::text::display_width(section.label(), 24.0)
                * crate::text::style::current().scale
                * self.widen;
            let badge_width = badge.as_ref().map_or(0.0, |badge| {
                crate::text::display_width(badge, 15.0) * self.widen + 22.0
            });
            let width = label_width + badge_width + 56.0;
            let rect = frame.rect(x, SECTION_Y, width, SECTION_HEIGHT);
            let radius = SECTION_HEIGHT * 0.5 * s;
            if lit {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color: color::GOLD,
                });
            } else {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color: color::alpha(color::HOLO, if hovered { 0.14 } else { 0.05 }),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius,
                    width: 1.4 * s,
                    color: color::alpha(color::HOLO, if hovered { 0.7 } else { 0.35 }),
                });
            }
            let ink = if lit {
                Color::new(0.078, 0.063, 0.02, 1.0)
            } else if hovered {
                color::TEXT
            } else {
                color::MUTED
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", section.label()),
                frame.rect(x + 28.0, SECTION_Y + 10.0, label_width + 4.0, 28.0),
                24.0 * s,
                ink,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if let Some(badge) = badge {
                let bx = x + 28.0 + label_width + 10.0;
                let bw = badge_width - 10.0;
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(bx, SECTION_Y + 13.0, bw, 22.0),
                    radius: 11.0 * s,
                    color: if lit {
                        color::alpha(color::SPACE, 0.85)
                    } else {
                        color::alpha(crate::cards::shard::KYBER, 0.22)
                    },
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{badge}"),
                    frame.rect(bx, SECTION_Y + 14.0, bw, 20.0),
                    15.0 * s,
                    crate::cards::shard::KYBER,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
            self.ui.hit_region(token, rect);
            x += width + 12.0;
        }
    }

    /// The player's kyber shards and credits at the top right, large, each mark and sum
    /// over its word; the pointer resting on one says how it is earned.
    fn forge_balances(&mut self, frame: &Frame) {
        let (Some(shards), Some(credits)) = (self.forge.data.shards, self.forge.data.credits)
        else {
            return;
        };
        let s = frame.s;
        let size = 40.0;
        let scale = crate::text::style::current().scale * self.widen;
        let mark_height = 34.0;
        let mut right = RIGHT_X;
        let columns = [
            (
                CREDITS_TOKEN,
                crate::cards::credits_text(credits),
                "credits",
                color::GOLD_BRIGHT,
                false,
            ),
            (
                SHARDS_TOKEN,
                crate::economy::shards_text(shards),
                "kyber shards",
                crate::cards::shard::KYBER,
                true,
            ),
        ];
        for (token, words, label, colour, shard) in columns {
            let width = crate::text::display_width(&words, size) * scale;
            let mark_width = if shard {
                crate::cards::shard::width(mark_height)
            } else {
                crate::cards::mark::width(mark_height)
            };
            let left = right - width - 12.0 - mark_width;
            let hovered = self.ui.token_hovered(token);
            if hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(left - 14.0, BALANCE_Y - 6.0, right - left + 28.0, 82.0),
                    radius: 12.0 * s,
                    color: color::alpha(colour, 0.08),
                });
            }
            let [mx, my] = frame.point(left, BALANCE_Y + 6.0);
            if shard {
                let _ = crate::cards::shard::draw(
                    self.ui.draw_list_mut(),
                    [mx, my],
                    mark_height * s,
                    crate::cards::shard::KYBER,
                );
            } else {
                let _ = crate::cards::mark::draw(
                    self.ui.draw_list_mut(),
                    [mx, my],
                    mark_height * s,
                    color::GOLD,
                );
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(right - width - 4.0, BALANCE_Y - 2.0, width + 4.0, 50.0),
                size * s,
                colour,
                FontWeight::Semibold,
                TextAlign::End,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(right - 220.0, BALANCE_Y + 50.0, 220.0, 22.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            self.ui.hit_region(
                token,
                frame.rect(left - 14.0, BALANCE_Y - 6.0, right - left + 28.0, 82.0),
            );
            right = left - 56.0;
        }
    }

    /// The line under the chips: what the section is, or `line` when given.
    fn forge_intro(&mut self, frame: &Frame, line: &str) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X, INTRO_Y, 900.0, 24.0),
            17.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// A quiet place in `rect` saying why it is empty: a dim shard, `title` and `line`.
    fn forge_empty(&mut self, frame: &Frame, rect: [f32; 4], title: &str, line: &str) {
        let s = frame.s;
        let [x, y, width, height] = rect;
        let middle = y + height * 0.5;
        let mark = 56.0;
        let [mx, my] = frame.point(
            x + width * 0.5 - crate::cards::shard::width(mark) * 0.5,
            middle - 110.0,
        );
        let _ = crate::cards::shard::draw(
            self.ui.draw_list_mut(),
            [mx, my],
            mark * s,
            color::alpha(crate::cards::shard::KYBER, 0.45),
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(x, middle - 34.0, width, 40.0),
            32.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let _ = super::view::centred_lines(
            &mut self.ui,
            frame,
            line,
            [x + width * 0.5 - 380.0, middle + 18.0, 760.0],
            84,
            3,
            18.0,
            color::MUTED,
        );
    }

    /// The Shop: the day's heading and countdown, its four offers, the holocrons for
    /// credits, and the hub's answer to the last purchase.
    fn forge_shop(&mut self, frame: &Frame, seconds: f32) {
        let s = frame.s;
        let now = Instant::now();
        self.forge_intro(
            frame,
            if crate::economy::shop_in_shards() {
                "Four saber shaders a day, picked for you from those you lack, for kyber shards."
            } else {
                "Four saber shaders a day, the same for every player, for credits."
            },
        );
        let (day, countdown) = match &self.forge.data.shop {
            Some(state) if state.message.is_empty() => (
                format!("Your offers of {}", day_text(&state.day.day)),
                match state.seconds_left(now) {
                    0 => "New offers are here: reading them...".to_owned(),
                    left => format!(
                        "New offers in {}, at midnight in Paris",
                        countdown_text(left)
                    ),
                },
            ),
            _ => ("Your offers today".to_owned(), String::new()),
        };
        kit::heading(&mut self.ui, frame, LEFT_X, DAY_Y, 1_040.0, &day);
        if !countdown.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{countdown}"),
                frame.rect(RIGHT_X - 640.0, DAY_Y - 13.0, 640.0, 26.0),
                17.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        let message = match &self.forge.data.shop {
            None if self.forge.data.can_trade => Some("Reading the shop...".to_owned()),
            None => self.forge.data.why_not.clone(),
            Some(state) if !state.message.is_empty() => Some(state.message.clone()),
            Some(state) if state.day.offers.is_empty() => {
                Some("The SJK hub offers nothing today.".to_owned())
            }
            Some(_) => None,
        };
        if let Some(message) = message {
            let title = if self.forge.data.can_trade {
                "The shop"
            } else {
                "Not open to you yet"
            };
            self.forge_empty(
                frame,
                [LEFT_X, OFFERS_Y, RIGHT_X - LEFT_X, 440.0],
                title,
                &message,
            );
            return;
        }
        let count = self.forge.offers().len().min(4);
        for index in 0..count {
            self.forge_offer(frame, index, now, seconds);
        }
        self.forge_holocrons(frame, now);
    }

    /// Offer `index`'s card: its tier's frame, wash and effects, the tier, its live blade,
    /// its name, its price, its state and Buy.
    fn forge_offer(&mut self, frame: &Frame, index: usize, now: Instant, seconds: f32) {
        let s = frame.s;
        let Some(offer) = self.forge.offers().get(index).cloned() else {
            return;
        };
        let tier = tier_of(&offer);
        let colour = tier.colour();
        let count = self.forge.offers().len().min(4) as f32;
        let total = count * OFFER[0] + (count - 1.0) * OFFER_GAP;
        let x = 960.0 - total * 0.5 + index as f32 * (OFFER[0] + OFFER_GAP);
        let y = OFFERS_Y;
        let [width, height] = OFFER;
        let rect = frame.rect(x, y, width, height);
        let chosen = index == self.forge.shop_selected;
        let token = OFFER_BASE + index as u16;
        let state = can(&offer, self.forge.purse());
        let held = matches!(state, Can::Owned | Can::Bought);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius: RADIUS * s,
            color: color::alpha(color::SPACE, 0.9),
        });
        let _ = list.push(DrawCommand::GradientRect {
            rect,
            radius: RADIUS * s,
            gradient: sjk_ui::Gradient {
                start: color::alpha(colour, 0.02),
                end: color::alpha(colour, if held { 0.08 } else { 0.2 }),
                vertical: true,
            },
        });
        rarity_fx::draw(
            self.ui.draw_list_mut(),
            rarity_fx::Item::of(tier, !held, rarity_fx::Size::Full),
            rect,
            RADIUS * s,
            s,
            seconds,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 28.0, y, width - 56.0, 4.0),
            radius: 2.0 * s,
            color: colour,
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: RADIUS * s,
            width: if chosen { 2.6 } else { 1.6 } * s,
            color: if chosen {
                color::alpha(CHOSEN, 0.9)
            } else {
                color::alpha(colour, 0.7)
            },
        });
        self.ui.hit_region(token, rect);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}  {}", tier.symbol(), tier.label()),
            frame.rect(x, y + 20.0, width, 28.0),
            21.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let look = self.skins.get(&offer.id);
        let _ = swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [x + 22.0, y + 62.0, width - 44.0, 150.0],
            look,
            true,
            seconds,
            0.0,
        );
        let name = if offer.name.trim().is_empty() {
            unlockables::find(&offer.id).map_or("A saber shader", |skin| skin.name)
        } else {
            offer.name.trim()
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x + 14.0, y + 230.0, width - 28.0, 38.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Saber shader"),
            frame.rect(x, y + 270.0, width, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
        if crate::economy::shop_in_shards() {
            crate::console::holocrons_panel::opening::shards_line(
                &mut self.ui,
                frame,
                x + width * 0.5,
                y + 304.0,
                &crate::economy::shards_text(offer.price),
                34.0,
                if held { 0.5 } else { 1.0 },
            );
        } else {
            crate::console::holocrons_panel::opening::credits_line(
                &mut self.ui,
                frame,
                x + width * 0.5,
                y + 304.0,
                &crate::cards::credits_text(offer.price),
                34.0,
                if held { 0.5 } else { 1.0 },
            );
        }
        let (status, status_colour) = match state {
            Can::Buy => (String::new(), color::MUTED),
            Can::Bought => (
                format!("{}  Bought today: it is yours", symbols::TICK),
                color::GOLD,
            ),
            Can::Owned => (
                format!("{}  You hold it already", symbols::TICK),
                color::GOLD,
            ),
            Can::Short(missing) => (format!("{} more needed", priced(missing)), color::MUTED),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(x, y + 352.0, width, 22.0),
            16.0 * s,
            status_colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let armed = self.forge.is_armed(Press::Offer(index), now);
        let waiting = self.forge.waiting();
        let buying = waiting
            && self
                .forge
                .sent
                .is_some_and(|(_, press)| press == Press::Offer(index));
        let label = match state {
            _ if buying => "Buying...".to_owned(),
            Can::Buy if armed => format!("Confirm: {}", priced(offer.price)),
            Can::Buy | Can::Short(_) => "Buy".to_owned(),
            Can::Bought => "Bought".to_owned(),
            Can::Owned => "Owned".to_owned(),
        };
        let open = state == Can::Buy && self.forge.data.can_trade;
        kit::button(
            &mut self.ui,
            frame,
            [x + 36.0, y + height - 70.0, width - 72.0, 48.0],
            &label,
            open,
            open && !waiting,
            chosen && armed,
            OFFER_BUY_BASE + index as u16,
        );
        // The hub's answer under the card it was asked from.
        if let Some((outcome, Some(Press::Offer(asked)))) = self.forge.answer()
            && asked == index
        {
            let words = crate::economy::outcome_words(outcome, name);
            let colour = if outcome.done() {
                color::GOLD_BRIGHT
            } else {
                color::EMBER
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(x - 16.0, y + height + 10.0, width + 32.0, 22.0),
                15.0 * s,
                colour,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
    }

    /// The holocrons credits buy, a card each under the offers: the holocron's picture, its
    /// name, what it holds, its price and Buy.
    fn forge_holocrons(&mut self, frame: &Frame, now: Instant) {
        let sale = crate::economy::holocrons_for_sale();
        if sale.is_empty() || !self.forge.shop_read() {
            return;
        }
        let s = frame.s;
        let width_all = RIGHT_X - LEFT_X;
        kit::heading(
            &mut self.ui,
            frame,
            LEFT_X,
            HOLOCRONS_HEAD_Y,
            width_all - 620.0,
            &crate::terms::say("Holocrons for credits"),
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Yours at once, outside the daily limit; they open like any other."),
            frame.rect(RIGHT_X - 600.0, HOLOCRONS_HEAD_Y - 12.0, 600.0, 24.0),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::End,
        );
        let credits = self.forge.data.credits.unwrap_or(0);
        let offers = self.forge.offers().len().min(4);
        let count = sale.len() as f32;
        let width = (width_all - OFFER_GAP * (count - 1.0)) / count;
        let waiting = self.forge.waiting();
        for (index, (tier_id, price)) in sale.into_iter().enumerate() {
            let x = LEFT_X + index as f32 * (width + OFFER_GAP);
            let y = HOLOCRONS_Y;
            let choice = offers + index;
            let chosen = self.forge.shop_selected == choice;
            let tier = holocron_tier(tier_id);
            let colour = tier.map_or(color::HOLO, |tier| tier.colour);
            let name = crate::terms::say(tier.map_or(tier_id, |tier| tier.name));
            let rect = frame.rect(x, y, width, HOLOCRON_HEIGHT);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: RADIUS * s,
                color: color::alpha(color::SPACE, 0.88),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::GradientRect {
                rect,
                radius: RADIUS * s,
                gradient: sjk_ui::Gradient {
                    start: color::alpha(colour, 0.16),
                    end: color::alpha(colour, 0.02),
                    vertical: false,
                },
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: RADIUS * s,
                width: if chosen { 2.6 } else { 1.5 } * s,
                color: if chosen {
                    color::alpha(CHOSEN, 0.9)
                } else {
                    color::alpha(colour, 0.6)
                },
            });
            self.ui.hit_region(HOLOCRON_BASE + index as u16, rect);
            if let Some(at) = crate::holocrons::TIERS.iter().position(|t| t.id == tier_id) {
                super::view::glow(
                    &mut self.ui,
                    frame,
                    [x + 70.0, y + HOLOCRON_HEIGHT * 0.5],
                    52.0,
                    colour,
                    0.35,
                );
                holocron_picture(
                    &mut self.ui,
                    frame,
                    at,
                    x + 70.0,
                    y + HOLOCRON_HEIGHT * 0.5,
                    40.0,
                );
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(x + 134.0, y + 20.0, width - 520.0, 34.0),
                28.0 * s,
                colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let short = credits < price;
            let (line, line_colour) = if short {
                (
                    format!(
                        "{} more credits needed",
                        crate::cards::credits_text(price - credits)
                    ),
                    color::MUTED,
                )
            } else {
                (
                    "A sealed card and credits, a saber shader or kyber shards".to_owned(),
                    color::MUTED,
                )
            };
            let _ = left_lines(
                &mut self.ui,
                frame,
                &line,
                [x + 134.0, y + 58.0, width - 520.0],
                (38, 2),
                15.0,
                line_colour,
                TextAlign::Start,
            );
            crate::console::holocrons_panel::opening::credits_line(
                &mut self.ui,
                frame,
                x + width - 330.0,
                y + 36.0,
                &crate::cards::credits_text(price),
                32.0,
                if short { 0.5 } else { 1.0 },
            );
            let armed = self.forge.is_armed(Press::Holocron(index), now);
            let buying = waiting
                && self
                    .forge
                    .sent
                    .is_some_and(|(_, press)| press == Press::Holocron(index));
            let label = if buying {
                "Buying...".to_owned()
            } else if armed && !short {
                format!("Confirm: {} credits", crate::cards::credits_text(price))
            } else {
                "Buy".to_owned()
            };
            let open = !short && self.forge.data.can_trade;
            kit::button(
                &mut self.ui,
                frame,
                [x + width - 230.0, y + 32.0, 200.0, 48.0],
                &label,
                open,
                open && !waiting,
                chosen && armed,
                HOLOCRON_BUY_BASE + index as u16,
            );
            if let Some((outcome, Some(Press::Holocron(asked)))) = self.forge.answer()
                && asked == index
            {
                let words = crate::economy::outcome_words(outcome, &name);
                let colour = if outcome.done() {
                    color::GOLD_BRIGHT
                } else {
                    color::EMBER
                };
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{words}"),
                    frame.rect(x + 134.0, y + 84.0, width - 380.0, 20.0),
                    14.0 * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
            }
        }
    }

    /// Craft: the filter, the shaders the player lacks, the one chosen up close.
    fn forge_craft(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        let all = craftable(&inputs.holdings, self.held_cards(inputs), &self.marks);
        let total = all.len();
        let shards = self.forge.data.shards.unwrap_or(0);
        let affordable = all.iter().filter(|item| item.cost <= shards).count();
        let line = if total == 0 {
            crate::terms::say("Every shader holocrons drop is yours already.").into_owned()
        } else {
            format!(
                "{total} saber shaders you lack, made with kyber shards: {affordable} within reach."
            )
        };
        self.forge_intro(frame, &line);
        self.forge_filter(frame, "Affordable", false);
        let list = self.forge.craft_list.clone();
        if list.is_empty() {
            let held = if total == 0 {
                Vec::new()
            } else {
                held_matches(
                    &inputs.holdings,
                    &self.forge.craft_filter.shaders,
                    &self.marks,
                )
            };
            let words: std::borrow::Cow<'_, str> = if total == 0 {
                "You hold every saber shader holocrons drop. New ones join the Forge as they                  come."
                    .into()
            } else if let [skin] = held.as_slice() {
                // Hub report #305: crafting refuses a held shader, so say why it is missing.
                format!(
                    "You hold the {} already. The Forge crafts only shaders you lack; another                      copy comes from holocrons.",
                    skin.name
                )
                .into()
            } else if !held.is_empty() {
                format!(
                    "You hold the {} shaders that match already. The Forge crafts only shaders                      you lack; other copies come from holocrons.",
                    held.len()
                )
                .into()
            } else {
                "T steps the tier, A shows only those within reach, / searches.".into()
            };
            let title = if total == 0 {
                "Nothing left to craft"
            } else if held.is_empty() {
                "No shader matches"
            } else {
                "Yours already"
            };
            self.forge_empty(
                frame,
                [LIST_LEFT, LIST_TOP, LIST_RIGHT - LIST_LEFT, 560.0],
                title,
                &crate::terms::say(&words),
            );
        }
        let first = self.forge.craft_first;
        let skins = std::sync::Arc::clone(&self.skins);
        for (index, item) in list.iter().enumerate() {
            let shown = Item {
                index,
                skin: item.skin,
                price: crate::economy::shards_text(item.cost),
                reach: item.cost <= shards,
                count: 0,
                look: skins.get(item.skin.id),
                favourite: item.favourite,
                locked: false,
                new: false,
                worn: false,
                picked: 0,
            };
            self.forge_item(frame, &shown, first, self.forge.craft, inputs, seconds);
        }
        self.forge_scroll(frame, first, list.len());
        if let Some(item) = list.get(self.forge.craft).copied() {
            self.forge_detail_craft(frame, item, inputs, seconds);
        }
    }

    /// Break down: the filter, Break down all duplicates, the player's own copies, the
    /// one chosen up close.
    fn forge_break(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        let (copies, shaders, gives) = self.forge.spare;
        let line = if self.forge.selecting {
            "Picking: click shaders to pick them, + and - for copies.".to_owned()
        } else if self.forge.data.last {
            "Turn saber shaders you hold into kyber shards: duplicates first.".to_owned()
        } else {
            "Turn duplicate saber shaders into kyber shards: you keep your last copy.".to_owned()
        };
        self.forge_intro(frame, &line);
        self.forge_filter(frame, "Duplicates", self.marks.kept);
        let list = self.forge.break_list.clone();
        if list.is_empty() {
            let any = !owned(&inputs.holdings, true, &self.marks).is_empty();
            let (title, words) = if any && !self.forge.data.last && copies == 0 {
                (
                    "No duplicates yet",
                    "A shader holocrons bring you again lands here, to break down into kyber \
                     shards. This SJK hub breaks down duplicates only.",
                )
            } else if any || self.forge.break_filter.only {
                (
                    "No shader matches",
                    "T steps the tier, D shows only duplicates, Shift+F only favourites, / \
                     searches.",
                )
            } else {
                (
                    "Nothing to break down yet",
                    "Saber shaders holocrons bring you, craft or buy can be broken down into \
                     kyber shards here. A medal's or a card's stays with it.",
                )
            };
            self.forge_empty(
                frame,
                [LIST_LEFT, LIST_TOP, LIST_RIGHT - LIST_LEFT, 560.0],
                title,
                &crate::terms::say(words),
            );
        }
        let first = self.forge.break_first;
        let skins = std::sync::Arc::clone(&self.skins);
        for (index, item) in list.iter().enumerate() {
            let shown = Item {
                index,
                skin: item.skin,
                price: format!("+{}", crate::economy::shards_text(item.gives)),
                reach: !item.locked,
                count: item.count,
                look: skins.get(item.skin.id),
                favourite: item.favourite,
                locked: item.locked,
                new: item.new,
                worn: item.worn,
                picked: self.forge.picked_copies(item.skin.id),
            };
            self.forge_item(frame, &shown, first, self.forge.breaks, inputs, seconds);
        }
        self.forge_scroll(frame, first, list.len());
        if let Some(item) = list.get(self.forge.breaks).copied() {
            self.forge_detail_break(frame, item, inputs, seconds);
        } else if copies > 0 {
            // Nothing chosen under the filter, but the duplicates may still go.
            let _ = (shaders, gives);
        }
    }

    /// The filter under the line: All and a chip a tier (in its colour), the section's own
    /// chip (`only`), Favourites (`favourites`: Break down, on a hub that keeps marks), the
    /// search field at the line's right and the view switches.
    fn forge_filter(&mut self, frame: &Frame, only: &str, favourites: bool) {
        let s = frame.s;
        let filter = self.forge.filter().clone();
        let mut left = LEFT_X;
        let mut chips: Vec<(u16, String, Color, bool)> = vec![(
            TIER_BASE,
            "All".to_owned(),
            color::HOLO,
            filter.shaders.tier.is_none(),
        )];
        for (index, tier) in FILTER_TIERS.iter().enumerate() {
            chips.push((
                TIER_BASE + 1 + index as u16,
                tier.label().to_owned(),
                tier.colour(),
                filter.shaders.tier == Some(*tier),
            ));
        }
        chips.push((ONLY_TOKEN, only.to_owned(), color::GOLD, filter.only));
        // Break down's Select, on a hub that breaks several shaders at once (05/11/2026).
        if self.forge.section == Section::Break && self.forge.data.many {
            chips.push((
                SELECT_TOKEN,
                format!("{}  Select", symbols::TICK),
                color::GOLD,
                self.forge.selecting,
            ));
        }
        if favourites {
            chips.push((
                FAVOURITES_TOKEN,
                format!("{}  Favourites", symbols::EDITION),
                color::GOLD,
                filter.shaders.favourite,
            ));
        }
        for (token, label, colour, lit) in chips {
            let width = body_width(&label, 15.0) * self.widen + 30.0;
            let rect = frame.rect(left, CHIPS_Y, width, CHIP_HEIGHT);
            let hovered = self.ui.token_hovered(token);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: CHIP_HEIGHT * 0.5 * s,
                    color: color::alpha(colour, if lit { 0.22 } else { 0.09 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: CHIP_HEIGHT * 0.5 * s,
                width: 1.2 * s,
                color: color::alpha(colour, if lit { 1.0 } else { 0.4 }),
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(left, CHIPS_Y + 5.0, width, CHIP_HEIGHT - 10.0),
                15.0 * s,
                if lit { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
            left += width + 8.0;
        }
        // The views: the list (three bars) and the grid (four squares).
        for (index, (token, grid)) in [(LIST_TOKEN, false), (GRID_TOKEN, true)]
            .into_iter()
            .enumerate()
        {
            let x = LIST_RIGHT - 2.0 * SWITCH - 8.0 + index as f32 * (SWITCH + 8.0);
            let lit = self.grid == grid;
            let hovered = self.ui.token_hovered(token);
            let rect = frame.rect(x, CHIPS_Y, SWITCH, SWITCH);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 8.0 * s,
                    color: color::alpha(color::GOLD, if lit { 0.16 } else { 0.08 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 8.0 * s,
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
                    [8.0, 8.0, 6.0, 6.0],
                    [16.0, 8.0, 6.0, 6.0],
                    [8.0, 16.0, 6.0, 6.0],
                    [16.0, 16.0, 6.0, 6.0],
                ]
            } else {
                &[
                    [8.0, 8.5, 14.0, 2.6],
                    [8.0, 13.7, 14.0, 2.6],
                    [8.0, 18.9, 14.0, 2.6],
                ]
            };
            for [mx, my, width, height] in marks {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x + mx, CHIPS_Y + my, *width, *height),
                    radius: 1.4 * s,
                    color: ink,
                });
            }
            self.ui.hit_region(token, rect);
        }
        let found = (!filter.shaders.query.trim().is_empty()).then(|| self.forge.listed());
        kit::search(
            &mut self.ui,
            frame,
            [LIST_RIGHT - SEARCH_WIDTH, INTRO_Y - 8.0, SEARCH_WIDTH, 38.0],
            &filter.shaders.query,
            self.forge.search_focus,
            "Search shaders",
            found,
            forge::SEARCH_TOKEN,
        );
    }

    /// `item` of a section's list: a grid card or a list row, as the view says, when it is
    /// in the window from `first`; `chosen` is the section's choice.
    fn forge_item(
        &mut self,
        frame: &Frame,
        item: &Item<'_>,
        first: usize,
        chosen: usize,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        if self.grid {
            let line = item.index / COLUMNS;
            if line < first || line >= first + LINES {
                return;
            }
            let column = (item.index % COLUMNS) as f32;
            let rect = [
                LIST_LEFT + column * (TILE[0] + TILE_GAP[0]),
                LIST_TOP + (line - first) as f32 * (TILE[1] + TILE_GAP[1]),
                TILE[0],
                TILE[1],
            ];
            self.forge_tile(frame, item, rect, item.index == chosen, inputs, seconds);
            if item.picked > 0 {
                // Over the card, as the chosen one stands raised.
                let [x, y, width, height] = rect;
                let y = if item.index == chosen { y - 5.0 } else { y };
                self.forge_pick_mark(frame, [x, y, width, height], item.picked);
            }
        } else {
            if item.index < first || item.index >= first + ROWS {
                return;
            }
            let y = LIST_TOP + (item.index - first) as f32 * ROW_STEP;
            self.forge_row(frame, item, y, item.index == chosen, inputs, seconds);
            if item.picked > 0 {
                self.forge_pick_mark(
                    frame,
                    [LIST_LEFT, y, LIST_RIGHT - LIST_LEFT, ROW_HEIGHT],
                    item.picked,
                );
            }
        }
    }

    /// The sign of a picked copies' stepper button at `[x, y]` (56 by 56), large; quiet
    /// while the button cannot be pressed.
    fn forge_step_sign(&mut self, frame: &Frame, [x, y]: [f32; 2], sign: &str, lit: bool) {
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{sign}"),
            frame.rect(x, y + 8.0, 56.0, 40.0),
            32.0 * frame.s,
            if lit { color::TEXT } else { color::QUIET },
            FontWeight::Semibold,
            TextAlign::Center,
        );
    }

    /// A picked shader's mark over its card or row in `rect` (05/11/2026): a gold frame and
    /// a gold tick at its top left, the copies picked in its place when more than one.
    fn forge_pick_mark(&mut self, frame: &Frame, [x, y, width, height]: [f32; 4], copies: u32) {
        let s = frame.s;
        // A wash and a thick bright frame just inside the card (the chosen one's white
        // outline stays outside it), apart from a Mythical card's own gold edge.
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 12.0 * s,
            color: color::alpha(color::GOLD, 0.16),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(x + 1.0, y + 1.0, width - 2.0, height - 2.0),
            radius: 11.0 * s,
            width: 3.2 * s,
            color: color::GOLD_BRIGHT,
        });
        let mark = if copies > 1 {
            format!("-{copies}")
        } else {
            symbols::TICK.to_string()
        };
        let side = (crate::text::display_width(&mark, 16.0) * self.widen + 12.0).max(26.0);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 6.0, y + 6.0, side, 26.0),
            radius: 13.0 * s,
            color: color::GOLD,
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{mark}"),
            frame.rect(x + 6.0, y + 9.0, side, 20.0),
            16.0 * s,
            Color::new(0.078, 0.063, 0.02, 1.0),
            FontWeight::Semibold,
            TextAlign::Center,
        );
    }

    /// A grid card in `rect`: the tier's band and frame, the live blade, the name, the tier
    /// and the price (or what a copy gives); a count badge for copies held.
    fn forge_tile(
        &mut self,
        frame: &Frame,
        item: &Item<'_>,
        rect: [f32; 4],
        chosen: bool,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let [x, y, width, height] = rect;
        let y = if chosen { y - 5.0 } else { y };
        let token = TILE_BASE + item.index as u16;
        let hovered = self.ui.token_hovered(token);
        let tier = item.skin.tier;
        let area = frame.rect(x, y, width, height);
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 2.0, y + 9.0, width - 4.0, height),
                radius: 12.0 * s,
                color: color::alpha(color::SPACE, 0.55),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 12.0 * s,
            color: color::alpha(color::SPACE, if chosen || hovered { 0.9 } else { 0.82 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 12.0, y, width - 24.0, 4.0),
            radius: 2.0 * s,
            color: tier.colour(),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 4.0, y + 4.0, width - 8.0, 30.0),
            radius: 10.0 * s,
            color: color::alpha(tier.colour(), 0.1),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 12.0 * s,
            width: if chosen { 1.8 } else { 1.2 } * s,
            color: color::alpha(tier.colour(), 0.85),
        });
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(x - 3.0, y - 3.0, width + 6.0, height + 6.0),
                radius: 15.0 * s,
                width: 2.0 * s,
                color: color::alpha(CHOSEN, 0.95),
            });
        }
        if chosen || hovered {
            crate::blade_pictures::focus(item.skin.id);
        }
        let _ = swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [x + 8.0, y + 22.0, width - 16.0, 52.0],
            item.look,
            true,
            seconds,
            swatch::chroma_turn(item.look, swatch::rgb_bytes(inputs.stock)),
        );
        if item.count == 1 {
            let badge = "Last";
            let badge_width = crate::text::display_width(badge, 14.0) * self.widen + 16.0;
            let rect = frame.rect(x + width - badge_width - 6.0, y + 8.0, badge_width, 22.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 11.0 * s,
                width: 1.2 * s,
                color: color::alpha(color::EMBER, 0.85),
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{badge}"),
                frame.rect(x + width - badge_width - 6.0, y + 9.0, badge_width, 20.0),
                14.0 * s,
                color::EMBER,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        if item.count > 1 {
            let badge = format!("x{}", item.count);
            let badge_width = crate::text::display_width(&badge, 15.0) * self.widen + 16.0;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + width - badge_width - 6.0, y + 8.0, badge_width, 22.0),
                radius: 11.0 * s,
                color: color::GOLD,
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{badge}"),
                frame.rect(x + width - badge_width - 6.0, y + 9.0, badge_width, 20.0),
                15.0 * s,
                Color::new(0.078, 0.063, 0.02, 1.0),
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        // The marks: the New sticker on the top edge, the star and the padlock at the
        // blade's foot on the right, worn by the tier.
        if item.new {
            let _ = marks_view::new_sticker(&mut self.ui, frame, [x + 8.0, y - 8.0], 18.0);
        }
        let _ = marks_view::glyph_pill(
            &mut self.ui,
            frame,
            [x + width - 6.0, y + 58.0],
            24.0,
            item.favourite,
            item.locked,
        );
        let name = item
            .skin
            .name
            .strip_suffix(" blade")
            .unwrap_or(item.skin.name);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x + 10.0, y + 86.0, width - 20.0, 26.0),
            21.0 * s,
            if chosen { CHOSEN } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}  {}", tier.symbol(), tier.label()),
            frame.rect(x + 10.0, y + 118.0, width - 20.0, 20.0),
            14.0 * s,
            tier.colour(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if item.worn {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}  Worn", symbols::SABER),
                frame.rect(x + 10.0, y + 118.0, width - 20.0, 20.0),
                14.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Semibold,
                TextAlign::End,
            );
        }
        self.forge_price(frame, item, [x + 10.0, y + 152.0], 24.0);
        rarity_fx::draw(
            self.ui.draw_list_mut(),
            rarity_fx::Item::of(tier, true, rarity_fx::Size::Full),
            area,
            12.0 * s,
            s,
            seconds,
        );
        self.ui.hit_region(token, area);
    }

    /// A list row at `y`: the live blade, the name, the tier, and at the right the copies
    /// and the price.
    fn forge_row(
        &mut self,
        frame: &Frame,
        item: &Item<'_>,
        y: f32,
        chosen: bool,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let token = TILE_BASE + item.index as u16;
        let hovered = self.ui.token_hovered(token);
        let tier = item.skin.tier;
        let width = LIST_RIGHT - LIST_LEFT;
        let area = frame.rect(LIST_LEFT, y, width, ROW_HEIGHT);
        if chosen || hovered {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: area,
                radius: 12.0 * s,
                color: color::alpha(color::HOLO, if chosen { 0.1 } else { 0.05 }),
            });
        }
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(LIST_LEFT - 12.0, y + 14.0, 4.0, ROW_HEIGHT - 28.0),
                radius: 2.0 * s,
                color: color::GOLD,
            });
        }
        let swatch_rect = [LIST_LEFT + 10.0, y + 9.0, 330.0, ROW_HEIGHT - 18.0];
        let [sx, sy, sw, sh] = swatch_rect;
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(sx, sy, sw, sh),
            radius: 10.0 * s,
            width: 1.2 * s,
            color: color::alpha(tier.colour(), 0.85),
        });
        if chosen || hovered {
            crate::blade_pictures::focus(item.skin.id);
        }
        let _ = swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            swatch_rect,
            item.look,
            true,
            seconds,
            swatch::chroma_turn(item.look, swatch::rgb_bytes(inputs.stock)),
        );
        let x = LIST_LEFT + 366.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", item.skin.name),
            frame.rect(x, y + 14.0, 300.0, 32.0),
            26.0 * s,
            if chosen {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}  {}", tier.symbol(), tier.label()),
            frame.rect(x, y + 54.0, 300.0, 20.0),
            15.0 * s,
            tier.colour(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        // The marks: worn after the tier, the star and the padlock and the New sticker on
        // the blade's left end.
        if item.worn {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}  Worn", symbols::SABER),
                frame.rect(x + 150.0, y + 54.0, 150.0, 20.0),
                15.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
        let _ = marks_view::glyph_pill(
            &mut self.ui,
            frame,
            [sx + sw - 6.0, sy + 6.0],
            24.0,
            item.favourite,
            item.locked,
        );
        if item.new {
            let _ = marks_view::new_sticker(&mut self.ui, frame, [sx + 6.0, sy + 6.0], 20.0);
        }
        if item.count > 1 {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("x{}", item.count),
                frame.rect(LIST_RIGHT - 300.0, y + 30.0, 120.0, 30.0),
                24.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Semibold,
                TextAlign::End,
            );
        }
        self.forge_price(frame, item, [LIST_RIGHT - 150.0, y + 30.0], 28.0);
        self.ui.hit_region(token, area);
    }

    /// The price at `[x, y]`: the shard mark and the number, lit when within reach.
    fn forge_price(&mut self, frame: &Frame, item: &Item<'_>, [x, y]: [f32; 2], size: f32) {
        let s = frame.s;
        let colour = if item.reach {
            crate::cards::shard::KYBER
        } else {
            color::QUIET
        };
        let mark = size * 0.85;
        let [mx, my] = frame.point(x, y + (size - mark) * 0.5);
        let _ = crate::cards::shard::draw(
            self.ui.draw_list_mut(),
            [mx, my],
            mark * s,
            color::alpha(colour, if item.reach { 1.0 } else { 0.6 }),
        );
        let after = x + crate::cards::shard::width(mark) + 8.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", item.price),
            frame.rect(after, y - 2.0, 140.0, size + 4.0),
            size * 0.82 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
    }

    /// The scroll bar at the list's right, when it holds more than it shows.
    fn forge_scroll(&mut self, frame: &Frame, first: usize, count: usize) {
        let s = frame.s;
        let (lines, shown, height) = if self.grid {
            (
                count.div_ceil(COLUMNS),
                LINES,
                LINES as f32 * (TILE[1] + TILE_GAP[1]) - TILE_GAP[1],
            )
        } else {
            (
                count,
                ROWS,
                ROWS as f32 * ROW_STEP - (ROW_STEP - ROW_HEIGHT),
            )
        };
        if lines <= shown {
            return;
        }
        let x = LIST_RIGHT + 10.0;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, LIST_TOP, 4.0, height),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let thumb = height * shown as f32 / lines as f32;
        let top = LIST_TOP + (height - thumb) * first as f32 / (lines - shown) as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top, 4.0, thumb),
            radius: 2.0 * s,
            color: color::GOLD,
        });
    }

    /// The chosen shader's card on the right: its tier, its blade big, its name (the star
    /// and the padlock at its right, on a hub that keeps them; the padlock while `owned`),
    /// its facts, what it is and its parts; returns where the words end (frame pixels).
    fn forge_detail_head(
        &mut self,
        frame: &Frame,
        skin: &'static Unlockable,
        facts: &[String],
        inputs: &Inputs<'_>,
        seconds: f32,
        owned: bool,
    ) -> f32 {
        let s = frame.s;
        let [x0, y0, width0, height0] = DETAIL;
        let area = frame.rect(x0, y0, width0, height0);
        let tier = skin.tier;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: RADIUS * s,
            color: color::alpha(color::SPACE, 0.84),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::GradientRect {
            rect: frame.rect(x0, y0, width0, 260.0),
            radius: RADIUS * s,
            gradient: sjk_ui::Gradient {
                start: color::alpha(tier.colour(), 0.14),
                end: color::alpha(tier.colour(), 0.0),
                vertical: true,
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x0 + 24.0, y0, width0 - 48.0, 4.0),
            radius: 2.0 * s,
            color: tier.colour(),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: RADIUS * s,
            width: 1.4 * s,
            color: color::alpha(tier.colour(), 0.8),
        });
        rarity_fx::draw(
            self.ui.draw_list_mut(),
            rarity_fx::Item::of(tier, true, rarity_fx::Size::Full),
            area,
            RADIUS * s,
            s,
            seconds,
        );
        let x = x0 + PAD;
        let width = width0 - PAD * 2.0;
        let mut y = y0 + 22.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}  {}", tier.symbol(), tier.label()),
            frame.rect(x, y, width, 28.0),
            21.0 * s,
            tier.colour(),
            FontWeight::Semibold,
            TextAlign::Start,
        );
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
        y += 40.0;
        // The chosen one, large.
        crate::blade_pictures::focus(skin.id);
        let look = self.skins.get(skin.id);
        let _ = swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [x, y, width, 150.0],
            look,
            true,
            seconds,
            swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
        );
        y += 168.0;
        let mut name_width = width;
        if self.marks.kept {
            let toggles_x = x + width - marks_view::TOGGLES;
            marks_view::toggle(
                &mut self.ui,
                frame,
                [toggles_x, y + 6.0],
                symbols::EDITION,
                self.marks.favourite(skin.id),
                true,
                STAR_TOKEN,
            );
            marks_view::toggle(
                &mut self.ui,
                frame,
                [
                    toggles_x + marks_view::TOGGLE + marks_view::TOGGLE_GAP,
                    y + 6.0,
                ],
                symbols::LOCK,
                self.marks.locked(skin.id),
                owned || self.marks.locked(skin.id),
                PADLOCK_TOKEN,
            );
            name_width = toggles_x - 16.0 - x;
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", skin.name),
            frame.rect(x, y, name_width, 58.0),
            46.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 66.0;
        let mut left = x;
        for fact in facts {
            left = self.forge_fact(frame, [left, y], fact);
        }
        if !facts.is_empty() {
            y += 48.0;
        }
        y = left_lines(
            &mut self.ui,
            frame,
            skin.description,
            [x, y, width],
            (80, 2),
            17.0,
            color::TEXT,
            TextAlign::Start,
        );
        if let Some(parts) = self.skins.get(skin.id).map(|look| {
            look.options()
                .iter()
                .map(|option| option.name.clone())
                .collect::<Vec<_>>()
        }) && !parts.is_empty()
        {
            y += 8.0;
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Parts: {}", parts.join(", ")),
                frame.rect(x, y, width, 22.0),
                15.0 * s,
                color::HOLO,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            y += 24.0;
        }
        y
    }

    /// A fact's chip at `[x, y]`; returns where the next starts.
    fn forge_fact(&mut self, frame: &Frame, [x, y]: [f32; 2], words: &str) -> f32 {
        let s = frame.s;
        let width = body_width(words, 15.0) * self.widen + 30.0;
        let rect = frame.rect(x, y, width, 32.0);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 16.0 * s,
            color: color::alpha(color::HOLO, 0.06),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 16.0 * s,
            width: 1.1 * s,
            color: color::alpha(color::HOLO, 0.3),
        });
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{words}"),
            frame.rect(x, y + 6.0, width, 20.0),
            15.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        x + width + 10.0
    }

    /// The two figures at the card's foot (frame pixels, from `y`): what the trade costs
    /// or gives, and the player's shards before and after it.
    #[allow(clippy::too_many_arguments)]
    fn forge_sums(
        &mut self,
        frame: &Frame,
        y: f32,
        left: (&str, String, Color),
        right: (&str, String, Color),
        note: (&str, Color),
    ) {
        let s = frame.s;
        let [x0, _, width0, _] = DETAIL;
        let x = x0 + PAD;
        let width = (width0 - PAD * 2.0 - 16.0) * 0.5;
        for (index, (label, words, colour)) in [left, right].into_iter().enumerate() {
            let bx = x + index as f32 * (width + 16.0);
            let rect = frame.rect(bx, y, width, 92.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 12.0 * s,
                color: color::alpha(color::HOLO, 0.05),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 12.0 * s,
                width: 1.0 * s,
                color: color::alpha(color::HOLO, 0.2),
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(bx + 18.0, y + 12.0, width - 36.0, 20.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let mark = 28.0;
            let [mx, my] = frame.point(bx + 18.0, y + 44.0);
            let _ = crate::cards::shard::draw(self.ui.draw_list_mut(), [mx, my], mark * s, colour);
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(
                    bx + 26.0 + crate::cards::shard::width(mark),
                    y + 38.0,
                    width - 60.0,
                    42.0,
                ),
                34.0 * s,
                colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", note.0),
            frame.rect(x, y + 100.0, width0 - PAD * 2.0, 22.0),
            15.0 * s,
            note.1,
            FontWeight::Semibold,
            TextAlign::Start,
        );
    }

    /// Craft's card: the shader up close, what it costs against the shards held, Craft.
    fn forge_detail_craft(
        &mut self,
        frame: &Frame,
        item: Craftable,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let now = Instant::now();
        let skin = item.skin;
        let mut facts = Vec::new();
        if skin.chroma {
            facts.push("Takes your colour".to_owned());
        }
        facts.push(format!("Smith: {}", skin.smith));
        let _ = self.forge_detail_head(frame, skin, &facts, inputs, seconds, false);
        let shards = self.forge.data.shards.unwrap_or(0);
        let reach = shards >= item.cost;
        let [x0, y0, width0, height0] = DETAIL;
        let bottom = y0 + height0;
        let sums_y = bottom - PAD - 56.0 - 26.0 - 122.0;
        let after = if reach {
            (
                format!(
                    "{} kyber shards left after crafting",
                    crate::economy::shards_text(shards - item.cost)
                ),
                color::MUTED,
            )
        } else {
            (
                crate::terms::say_owned(format!(
                    "{} kyber shards more needed: break down a duplicate, or open holocrons",
                    crate::economy::shards_text(item.cost - shards)
                )),
                color::EMBER,
            )
        };
        // A copy of its blade card lowers the cost, and the card and its cut are named
        // (21/10/2026).
        let costs = item.off.map_or_else(
            || "Crafting costs".to_owned(),
            |(finish, off)| format!("Crafting costs ({} card -{off} %)", finish.name),
        );
        self.forge_sums(
            frame,
            sums_y,
            (
                &costs,
                crate::economy::shards_text(item.cost),
                crate::cards::shard::KYBER,
            ),
            (
                "You hold",
                crate::economy::shards_text(shards),
                if reach { color::TEXT } else { color::QUIET },
            ),
            (&after.0, after.1),
        );
        let press = Press::Craft(skin.id);
        let armed = self.forge.is_armed(press, now);
        let waiting = self.forge.waiting();
        let label = if waiting && self.forge.sent.is_some_and(|(_, sent)| sent == press) {
            "Crafting...".to_owned()
        } else if armed && reach {
            format!(
                "Confirm: craft for {}",
                crate::economy::shards_text(item.cost)
            )
        } else if let Some((finish, off)) = item.off {
            format!(
                "Craft  {}  {}  {} card -{off} %",
                symbols::SHARD,
                crate::economy::shards_text(item.cost),
                finish.name
            )
        } else {
            format!(
                "Craft  {}  {}",
                symbols::SHARD,
                crate::economy::shards_text(item.cost)
            )
        };
        let open = reach && self.forge.data.can_trade;
        let button_y = bottom - PAD - 56.0;
        let button = (crate::text::display_width(&label, 18.0) + 48.0).max(330.0);
        kit::button(
            &mut self.ui,
            frame,
            [x0 + PAD, button_y, button, 56.0],
            &label,
            open,
            open && !waiting,
            armed,
            ACTION_TOKEN,
        );
        self.forge_answer_line(
            frame,
            press,
            [
                x0 + PAD + button + 20.0,
                button_y,
                width0 - PAD * 2.0 - button - 20.0,
            ],
        );
    }

    /// Break down's card: the shader up close, the copies held, what a copy gives, and
    /// Break down a copy (the last copy asks with a card first).
    fn forge_detail_break(
        &mut self,
        frame: &Frame,
        item: Owned,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let now = Instant::now();
        let skin = item.skin;
        let mut facts = vec![if item.count > 1 {
            format!("{} copies", item.count)
        } else {
            "Your only copy".to_owned()
        }];
        if item.worn {
            facts.push(format!("{}  Worn", symbols::SABER));
        }
        if item.new {
            facts.push("New".to_owned());
        }
        if skin.chroma {
            facts.push("Takes your colour".to_owned());
        }
        let _ = self.forge_detail_head(frame, skin, &facts, inputs, seconds, true);
        let shards = self.forge.data.shards.unwrap_or(0);
        let [x0, y0, width0, height0] = DETAIL;
        let bottom = y0 + height0;
        let sums_y = bottom - PAD - 56.0 - 26.0 - 122.0;
        // While picking, the sums follow the copies picked of it (05/11/2026), else one.
        let taken = match self.forge.picked_copies(skin.id) {
            picked if self.forge.selecting && picked > 0 => picked,
            _ => 1,
        };
        let keep = item.count.saturating_sub(taken);
        let note = if item.locked {
            (
                format!(
                    "{}  Locked: you keep it until you unlock it (L).",
                    symbols::LOCK
                ),
                color::GOLD_BRIGHT,
            )
        } else if keep == 0 && item.worn {
            (
                format!(
                    "{}, and you wear it: it leaves your collection and your saber.",
                    if item.last() {
                        "Your only copy"
                    } else {
                        "Every copy"
                    }
                ),
                color::EMBER,
            )
        } else if keep == 0 {
            (
                format!(
                    "{}: it leaves your collection.",
                    if item.last() {
                        "Your only copy"
                    } else {
                        "Every copy"
                    }
                ),
                color::EMBER,
            )
        } else {
            (
                format!(
                    "You keep {keep} {}",
                    if keep == 1 { "copy" } else { "copies" }
                ),
                color::MUTED,
            )
        };
        let gives = u64::from(taken) * item.gives;
        let gives_label = if taken == 1 {
            "A copy gives".to_owned()
        } else {
            format!("{taken} copies give")
        };
        self.forge_sums(
            frame,
            sums_y,
            (
                &gives_label,
                format!("+{}", crate::economy::shards_text(gives)),
                crate::cards::shard::KYBER,
            ),
            (
                "You will hold",
                crate::economy::shards_text(shards + gives),
                color::TEXT,
            ),
            (&note.0, note.1),
        );
        let press = Press::Break(skin.id);
        let armed = self.forge.is_armed(press, now);
        let waiting = self.forge.waiting();
        let label = if item.locked {
            format!("{}  Locked", symbols::LOCK)
        } else if waiting && self.forge.sent.is_some_and(|(_, sent)| sent == press) {
            "Breaking down...".to_owned()
        } else if item.last() {
            "Break down your last copy".to_owned()
        } else if armed {
            format!(
                "Confirm: +{} kyber shards",
                crate::economy::shards_text(item.gives)
            )
        } else {
            format!(
                "Break down a copy  +{}",
                crate::economy::shards_text(item.gives)
            )
        };
        let open = self.forge.data.can_trade;
        let button_y = bottom - PAD - 56.0;
        if self.forge.selecting {
            // Picking: the button breaks every pick down (after the card asks).
            let (picked, copies, gives, _) = self.forge.picked_sum();
            let label = if waiting && self.forge.sent_many.is_some() {
                "Breaking down...".to_owned()
            } else if picked == 0 {
                "Pick shaders to break down".to_owned()
            } else {
                format!(
                    "Break down selected ({copies})  +{}",
                    crate::economy::shards_text(gives)
                )
            };
            kit::button(
                &mut self.ui,
                frame,
                [x0 + PAD, button_y, 330.0, 56.0],
                &label,
                open && picked > 0,
                open && !waiting && picked > 0,
                false,
                ACTION_TOKEN,
            );
        } else {
            kit::button(
                &mut self.ui,
                frame,
                [x0 + PAD, button_y, 330.0, 56.0],
                &label,
                open && !item.last() && !item.locked,
                open && !waiting && !item.locked,
                armed,
                ACTION_TOKEN,
            );
        }
        // Picking: the chosen shader's picked copies, fewer and more, beside the button.
        if self.forge.selecting {
            let picked = self.forge.picked_copies(skin.id);
            let most = self.forge.pick_max(item);
            let left = x0 + PAD + 346.0;
            let fewer = open && !waiting && picked > 1;
            let more = open && !waiting && picked < most;
            kit::button(
                &mut self.ui,
                frame,
                [left, button_y, 56.0, 56.0],
                "",
                false,
                fewer,
                false,
                PICK_LESS_TOKEN,
            );
            self.forge_step_sign(frame, [left, button_y], "-", fewer);
            let words = if most == 0 {
                "Cannot be picked".to_owned()
            } else if picked == 0 {
                "Not picked".to_owned()
            } else {
                format!("{picked} of {most} picked")
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(left + 64.0, button_y + 17.0, 150.0, 24.0),
                17.0 * frame.s,
                if picked > 0 {
                    color::GOLD_BRIGHT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            kit::button(
                &mut self.ui,
                frame,
                [left + 222.0, button_y, 56.0, 56.0],
                "",
                false,
                more,
                false,
                PICK_MORE_TOKEN,
            );
            self.forge_step_sign(frame, [left + 222.0, button_y], "+", more);
            self.forge_answer_line(
                frame,
                press,
                [x0 + PAD, button_y - 30.0, width0 - PAD * 2.0],
            );
        } else if self.forge.data.all && self.forge.spare.0 > 0 {
            // Every duplicate at once, beside it, on a hub that takes it.
            let (copies, _, gives) = self.forge.spare;
            let label = format!(
                "Break down all duplicates ({copies})  +{}",
                crate::economy::shards_text(gives)
            );
            kit::button(
                &mut self.ui,
                frame,
                [x0 + PAD + 346.0, button_y, width0 - PAD * 2.0 - 346.0, 56.0],
                &label,
                false,
                open && !waiting,
                false,
                ALL_TOKEN,
            );
            self.forge_answer_line(
                frame,
                press,
                [x0 + PAD, button_y - 30.0, width0 - PAD * 2.0],
            );
        } else {
            self.forge_answer_line(
                frame,
                press,
                [x0 + PAD + 350.0, button_y, width0 - PAD * 2.0 - 350.0],
            );
        }
    }

    /// The hub's answer to the last trade of `press`'s shader (or all duplicates), the
    /// wait, or why the Forge cannot trade, in the row from `[x, y, width]` beside the
    /// button.
    fn forge_answer_line(&mut self, frame: &Frame, press: Press, [x, y, width]: [f32; 3]) {
        let s = frame.s;
        let (words, colour) = match self.forge.answer() {
            Some((outcome, asked)) if asked.is_none() || asked == Some(press) => {
                let name = match press {
                    Press::Craft(id) | Press::Break(id) => {
                        unlockables::find(id).map_or(id, |skin| skin.name)
                    }
                    _ => "",
                };
                (
                    crate::economy::outcome_words(outcome, name),
                    if outcome.done() {
                        color::GOLD_BRIGHT
                    } else {
                        color::EMBER
                    },
                )
            }
            _ if self.forge.waiting() => ("Asking the SJK hub...".to_owned(), color::MUTED),
            _ => match &self.forge.data.why_not {
                Some(why) => (why.clone(), color::MUTED),
                None => (String::new(), color::MUTED),
            },
        };
        let _ = left_lines(
            &mut self.ui,
            frame,
            &words,
            [x, y + 6.0, width],
            (
                // As many letters as the row's width holds (46 beside the button), so a
                // line above it stays on one row instead of falling onto the button.
                ((width * 46.0 / 280.0) as usize).clamp(46, 120),
                2,
            ),
            15.0,
            colour,
            TextAlign::Start,
        );
        let _ = s;
    }

    /// The card asking before the last copy or every duplicate goes, over the darkened
    /// tab (the page under it is left out): what goes, shown; what it gives; and the
    /// quieter choice (Keep it, Cancel) beside the stronger one.
    fn forge_ask(
        &mut self,
        frame: &Frame,
        ask: Ask,
        inputs: &Inputs<'_>,
        seconds: f32,
        viewport: [f32; 2],
    ) {
        let s = frame.s;
        kit::scrim(&mut self.ui, viewport);
        self.ui
            .cover(sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]));
        let [x, _, width, _] = ASK;
        let inner = width - 96.0;
        let left = x + 48.0;
        // What goes, as blades: the one shader, or each with copies to spare.
        let (title, lines, gives, yes_label, no_label, yes, danger, shown) = match ask {
            Ask::Last { skin, yes } => {
                let gives = crate::economy::shatter_yield(skin.tier);
                let mut lines = Vec::with_capacity(3);
                // Worn on either saber (or staff half) is said first and plainly (Sol,
                // 29/10/2026).
                if self.marks.worn(skin.id) {
                    lines.push(format!(
                        "You wear this shader now: breaking it down puts your saber back to \
                         the stock blade{}.",
                        if inputs.pair == crate::saber_skins::Pair::None {
                            ""
                        } else {
                            " (each saber or half that wears it)"
                        }
                    ));
                }
                lines.push(format!(
                    "Breaking down your {} gives you {} kyber shards, and the blade leaves your \
                     collection.",
                    skin.name,
                    crate::economy::shards_text(gives)
                ));
                // Crafting it back costs less with a copy of its blade card.
                let card = crate::economy::blade_card(
                    skin,
                    self.held_cards(inputs),
                    &crate::cards::faces::current(),
                );
                let again = crate::economy::craft_price(skin, card).0;
                lines.push(crate::terms::say_owned(format!(
                    "To have it again you would craft it for {} kyber shards, or find it in a \
                     holocron.",
                    crate::economy::shards_text(again)
                )));
                (
                    "This is your only copy",
                    lines,
                    gives,
                    "Break it down",
                    "Keep it",
                    yes,
                    true,
                    vec![(skin, 0)],
                )
            }
            Ask::All { yes } => {
                let (copies, shaders, gives) = self.forge.spare;
                let lines = vec![
                    format!(
                        "{copies} {} of {shaders} saber {} become kyber shards. You keep one \
                         of each: no shader leaves your collection.",
                        if copies == 1 { "copy" } else { "copies" },
                        if shaders == 1 { "shader" } else { "shaders" }
                    ),
                    crate::terms::say("Copies a medal or a card brings are never touched.")
                        .into_owned(),
                ];
                let all = owned(&inputs.holdings, false, &self.marks);
                let kept = all.iter().filter(|item| item.locked).count();
                let mut lines = lines;
                if kept > 0 {
                    lines.push(format!(
                        "{}  Locked shaders are kept whole: {kept} {} skipped.",
                        symbols::LOCK,
                        if kept == 1 { "is" } else { "are" }
                    ));
                }
                let shown: Vec<_> = all
                    .into_iter()
                    .filter(|item| !item.locked)
                    .map(|item| (item.skin, item.count - 1))
                    .collect();
                (
                    "Break down all duplicates?",
                    lines,
                    gives,
                    "Break them down",
                    "Cancel",
                    yes,
                    false,
                    shown,
                )
            }
            Ask::Picked { yes } => {
                let items = self.forge.picked_items();
                let (picked, copies, gives, lasts) = self.forge.picked_sum();
                let mut lines = Vec::with_capacity(3);
                if items
                    .iter()
                    .any(|(item, copies)| *copies >= item.count && item.worn)
                {
                    lines.push(
                        "You wear one of them now: breaking it down puts your saber back to \
                         the stock blade."
                            .to_owned(),
                    );
                }
                lines.push(format!(
                    "{copies} {} of the {picked} saber {} you picked {} kyber shards.",
                    if copies == 1 { "copy" } else { "copies" },
                    if picked == 1 { "shader" } else { "shaders" },
                    if copies == 1 { "becomes" } else { "become" }
                ));
                if lasts > 0 {
                    lines.push(format!(
                        "You give up every copy of {lasts} of them: {} your collection.",
                        if lasts == 1 {
                            "it leaves"
                        } else {
                            "they leave"
                        }
                    ));
                }
                let shown: Vec<_> = items
                    .iter()
                    .map(|(item, copies)| (item.skin, *copies))
                    .collect();
                (
                    if lasts > 0 {
                        "Break down the picked shaders?"
                    } else {
                        "Break down the picked copies?"
                    },
                    lines,
                    gives,
                    "Break them down",
                    "Cancel",
                    yes,
                    lasts > 0,
                    shown,
                )
            }
        };
        // Worn: a pill beside the title, so it cannot be missed.
        let worn_pill = match ask {
            Ask::Last { skin, .. } => self.marks.worn(skin.id),
            Ask::Picked { .. } => self
                .forge
                .picked_items()
                .iter()
                .any(|(item, copies)| *copies >= item.count && item.worn),
            Ask::All { .. } => false,
        };
        // A last copy among the picks reddens the title and the button, as for one.
        let big = matches!(ask, Ask::Last { .. });
        // The card's height follows what it holds; it stands centred on the page.
        let wrapped: usize = lines
            .iter()
            .map(|line| crate::menu::sjk::wrap(line, 86).count().min(2))
            .sum();
        let blades = if big { 130.0 } else { 96.0 };
        let height = 112.0 + blades + 18.0 + wrapped as f32 * 25.6 + 30.0 + 40.0 + 52.0 + 44.0;
        let y = 545.0 - height * 0.5;
        kit::card(&mut self.ui, frame, [x, y, width, height]);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(left, y + 40.0, inner, 50.0),
            40.0 * s,
            if danger { color::EMBER } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if worn_pill {
            let words = format!("{}  You wear it", symbols::SABER);
            let pill = crate::text::display_width(&words, 18.0) * self.widen + 36.0;
            let rect = frame.rect(left + inner - pill, y + 48.0, pill, 34.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 17.0 * s,
                color: color::alpha(color::EMBER, 0.16),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 17.0 * s,
                width: 1.4 * s,
                color: color::EMBER,
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(left + inner - pill, y + 54.0, pill, 24.0),
                18.0 * s,
                color::EMBER,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        if big {
            let look = shown.first().and_then(|(skin, _)| self.skins.get(skin.id));
            let _ = swatch::small_swatch(
                self.ui.draw_list_mut(),
                frame,
                [left, y + 112.0, inner, 112.0],
                look,
                true,
                seconds,
                swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
            );
        } else {
            // Up to five shaders with copies to spare, each with how many go.
            let gap = 12.0;
            let each = (inner - gap * 4.0) / 5.0;
            for (index, (skin, spare)) in shown.iter().take(5).enumerate() {
                let bx = left + index as f32 * (each + gap);
                let look = self.skins.get(skin.id);
                let _ = swatch::small_swatch(
                    self.ui.draw_list_mut(),
                    frame,
                    [bx, y + 112.0, each, 48.0],
                    look,
                    true,
                    seconds,
                    swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                );
                let name = skin.name.strip_suffix(" blade").unwrap_or(skin.name);
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{name}  -{spare}"),
                    frame.rect(bx, y + 168.0, each, 20.0),
                    14.0 * s,
                    skin.tier.colour(),
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
            if shown.len() > 5 {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("and {} more", shown.len() - 5),
                    frame.rect(left, y + 190.0, inner, 20.0),
                    14.0 * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::End,
                );
            }
        }
        let mut line_y = y + 112.0 + blades + 18.0;
        for (index, line) in lines.iter().enumerate() {
            // The worn line (the first) in ember, as the pill.
            let colour = if worn_pill && index == 0 {
                color::EMBER
            } else {
                color::MUTED
            };
            line_y = left_lines(
                &mut self.ui,
                frame,
                line,
                [left, line_y, inner],
                (86, 2),
                18.0,
                colour,
                TextAlign::Start,
            );
        }
        // The gain at the left of the buttons' row, the choices at its right.
        let buttons = y + height - 44.0 - 52.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("You get"),
            frame.rect(left, buttons - 26.0, 300.0, 20.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let gain = format!("+{}", crate::economy::shards_text(gives));
        let mark = 34.0;
        let [mx, my] = frame.point(left, buttons + 8.0);
        let _ = crate::cards::shard::draw(
            self.ui.draw_list_mut(),
            [mx, my],
            mark * s,
            crate::cards::shard::KYBER,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{gain}"),
            frame.rect(
                left + crate::cards::shard::width(mark) + 12.0,
                buttons,
                300.0,
                50.0,
            ),
            42.0 * s,
            crate::cards::shard::KYBER,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let no_width = 180.0;
        let yes_width = 230.0;
        kit::button(
            &mut self.ui,
            frame,
            [
                x + width - 48.0 - yes_width - 16.0 - no_width,
                buttons,
                no_width,
                52.0,
            ],
            no_label,
            false,
            true,
            !yes,
            ASK_NO_TOKEN,
        );
        let yes_rect = [x + width - 48.0 - yes_width, buttons, yes_width, 52.0];
        if danger {
            danger_button(&mut self.ui, frame, yes_rect, yes_label, yes, ASK_YES_TOKEN);
        } else {
            kit::button(
                &mut self.ui,
                frame,
                yes_rect,
                yes_label,
                true,
                true,
                yes,
                ASK_YES_TOKEN,
            );
        }
    }

    /// The hover card of the Forge's `token`: a balance (how it is earned), or a listed
    /// shader (how it is obtained).
    pub(super) fn forge_card(
        &self,
        token: u16,
        inputs: &Inputs<'_>,
    ) -> Option<crate::menu::sjk::hover::Card> {
        use crate::menu::sjk::hover::{Card, Line};
        match token {
            SHARDS_TOKEN => Some(Card {
                title: "Kyber shards".to_owned(),
                colour: crate::cards::shard::KYBER,
                lines: vec![
                    Line::how(crate::terms::say(
                        "Found in holocrons as a pile, a few with every card, and from \
                         breaking saber shaders down here.",
                    )),
                    Line::status("They craft any shader of the trade and buy the shop's offers."),
                    Line::held(format!(
                        "You hold {}.",
                        crate::economy::shards_text(self.forge.data.shards.unwrap_or(0))
                    )),
                ],
            }),
            CREDITS_TOKEN => Some(Card {
                title: "Credits".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(crate::terms::say(
                        "Every card a holocron brings comes with credits.",
                    )),
                    Line::status(crate::terms::say(
                        "They buy Uncommon and Rare holocrons in the shop.",
                    )),
                    Line::held(format!(
                        "You hold {}.",
                        crate::cards::credits_text(self.forge.data.credits.unwrap_or(0))
                    )),
                ],
            }),
            STAR_TOKEN | PADLOCK_TOKEN | FAVOURITES_TOKEN => {
                let chosen = self.forge.chosen_shader();
                hover::mark_card(token_mark(token), &self.marks, chosen)
            }
            token => {
                let index = usize::from(token.checked_sub(TILE_BASE)?);
                let skin = match self.forge.section {
                    Section::Craft => self.forge.craft_list.get(index)?.skin,
                    Section::Break => self.forge.break_list.get(index)?.skin,
                    Section::Shop => return None,
                };
                Some(hover::shader_card(
                    Some(skin),
                    &inputs.holdings,
                    &self.marks,
                ))
            }
        }
    }
}

/// Which of the marks' hover cards the Forge's `token` asks for.
fn token_mark(token: u16) -> hover::MarkToken {
    match token {
        STAR_TOKEN => hover::MarkToken::Favourite,
        PADLOCK_TOKEN => hover::MarkToken::Lock,
        _ => hover::MarkToken::Favourites,
    }
}

/// How wide `words` are in the body family at `size` (frame pixels), near enough for a
/// chip's room: Exo 2 runs about a seventh wider than the display family.
fn body_width(words: &str, size: f32) -> f32 {
    crate::text::display_width(words, size) * crate::text::style::current().scale * 1.15
}

/// Holocron tier `index`'s picture over the square of side `2 * half` frame pixels centred
/// on (`x`, `y`): its icon, or its gem where the icon did not load.
fn holocron_picture(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    index: usize,
    x: f32,
    y: f32,
    half: f32,
) {
    use crate::holocrons::{gem, icons};
    if icons::is_loaded(index) {
        let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: frame.rect(x - half, y - half, half * 2.0, half * 2.0),
            texture: icons::texture(index),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
    } else {
        let [cx, cy] = frame.point(x, y);
        gem::draw(
            canvas.draw_list_mut(),
            [cx, cy],
            half * frame.s,
            crate::holocrons::TIERS[index].colour,
            1.0,
            gem::ROWS,
        );
    }
}

/// A button for a choice that cannot be undone, over `rect`: outlined in ember, filled
/// ember with dark words while it has the focus or the pointer.
fn danger_button(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    label: &str,
    focused: bool,
    token: u16,
) {
    let s = frame.s;
    let [x, y, width, height] = rect;
    let area = frame.rect(x, y, width, height);
    let lit = focused || canvas.token_hovered(token);
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: area,
        radius: height * 0.5 * s,
        color: if lit {
            color::EMBER
        } else {
            color::alpha(color::EMBER, 0.1)
        },
    });
    let _ = canvas.draw_list_mut().push(DrawCommand::Border {
        rect: area,
        radius: height * 0.5 * s,
        width: 1.6 * s,
        color: color::EMBER,
    });
    text(
        canvas,
        TextFamily::Display,
        format_args!("{label}"),
        frame.rect(x, y + (height - 26.0) * 0.5, width, 26.0),
        20.0 * s,
        if lit {
            Color::new(0.08, 0.03, 0.01, 1.0)
        } else {
            color::EMBER
        },
        FontWeight::Regular,
        TextAlign::Center,
    );
    canvas.hit_region(token, area);
}
