//! The Collection page's frame, in the SJK UI's look: the map darkened behind Medals
//! and Achievements, or dark on the left and clear over the player's model behind
//! Shaders, Toys and Nameplates; the way back, the title and the row of tabs, each
//! with how many of its things the player holds; at the top right how many of all of
//! them, as a tick per thing, lit for the ones held; the tab; the keys.

use super::*;
use crate::menu::sjk::{
    Frame, TextTarget, color, fade, fade_across, key_hint, key_hint_width, text,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The page's left edge, where a tab's things start, and the right edge.
pub(super) const LEFT_X: f32 = 96.0;
pub(super) const RIGHT_X: f32 = 1_824.0;
/// Where a tab's headline stands, under the row of tabs.
pub(super) const LEAD_Y: f32 = 222.0;
/// The keys' line.
pub(super) const KEYS_Y: f32 = 1_010.0;
/// The showcase on the right of Medals and Achievements: its left edge and width.
pub(super) const SHOWCASE_X: f32 = 1_120.0;
pub(super) const SHOWCASE_WIDTH: f32 = RIGHT_X - SHOWCASE_X;
/// Where the live preview of the player's model shows over a match (frame pixels):
/// where the model stands on the menu map's stage.
pub(crate) const MODEL_AREA: [f32; 4] = [760.0, 110.0, 820.0, 840.0];
/// Where the Shaders tab's saber lies on its own (frame pixels), over the clear right of
/// the screen above the shader's card: on the menu map's stage before the backdrop
/// camera, and over a match in the live preview drawn there.
pub(crate) const SABER_AREA: [f32; 4] = [1_000.0, 196.0, RIGHT_X - 1_000.0, 300.0];

/// The tick bar at the top right: a tick's size, the gaps between ticks and groups.
/// The tally's bar at the top right: as wide as its words, a stretch to a tab.
const TALLY_WIDTH: f32 = 500.0;
const TALLY_HEIGHT: f32 = 6.0;
const GROUP_GAP: f32 = 10.0;

/// How many of a tab's things the player holds, and how many there are; `None` while
/// that is not known (no identity, no answer from the hub).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Count {
    pub(super) held: Option<usize>,
    pub(super) total: usize,
}

impl Count {
    /// What the tab's name says after it: "2/4", how many for a tab with no set total
    /// (the holocrons held), empty when unknown or none.
    fn label(self) -> String {
        match self.held {
            Some(held) if self.total > 0 => format!("{held}/{}", self.total),
            Some(held) if held > 0 => crate::cards::grouped(held as u64),
            _ => String::new(),
        }
    }
}

/// Each tab's count, in the row's order.
pub(super) fn counts(inputs: &Inputs<'_>) -> [Count; 11] {
    let medals = medals::held(inputs);
    // Out of the ones the player sees: a retired one not held is not counted.
    let (achievements, achievements_total) = crate::achievements::tally(inputs.standings);
    let shaders = (inputs.holdings.reason().is_none()).then(|| {
        unlockables::ALL
            .iter()
            .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
            .count()
    });
    // Out of the shaders the player sees: an unheld Unique one is not counted.
    let shaders_shown = inputs.holdings.listed_count();
    [
        Count {
            held: medals,
            total: crate::medals::Medal::COUNT,
        },
        Count {
            held: Some(achievements),
            total: achievements_total,
        },
        Count {
            held: shaders,
            total: shaders_shown,
        },
        // Every toy is everyone's (Illuminate, the cardboard box, Order 66).
        Count {
            held: Some(TOY_COUNT),
            total: TOY_COUNT,
        },
        // The tags held, the free ones with them.
        Count {
            held: (inputs.holdings.reason().is_none()).then(|| {
                crate::tags::CATALOGUE
                    .iter()
                    .filter(|tag| tag.free || inputs.holdings.unlock(tag.id).is_some())
                    .count()
            }),
            total: crate::tags::CATALOGUE.len(),
        },
        Count {
            held: Some(0),
            total: 0,
        },
        // The holocrons are opened, not kept: how many, and no total.
        Count {
            held: inputs
                .snapshot
                .filter(|_| inputs.enabled)
                .and_then(|snapshot| snapshot.me.as_ref())
                .map(|me| {
                    crate::holocrons::counts_of(&me.holocron_counts)
                        .iter()
                        .sum::<u32>() as usize
                }),
            total: 0,
        },
        // The Forge: the kyber shards the player holds, what it trades with.
        Count {
            held: inputs
                .snapshot
                .filter(|_| inputs.enabled)
                .and_then(|snapshot| snapshot.me.as_ref())
                .filter(|me| me.shards > 0)
                .map(|me| me.shards as usize),
            total: 0,
        },
        // The cards held, and no total until a set is out.
        Count {
            held: cards::held(inputs),
            total: 0,
        },
        Count {
            held: inputs
                .snapshot
                .filter(|_| inputs.enabled)
                .and_then(|snapshot| snapshot.me.as_ref())
                .map(|profile| {
                    crate::profile_card::banner::ALL
                        .iter()
                        .filter(|banner| {
                            crate::profile_card::banner::count(Some(profile), banner.id) > 0
                        })
                        .count()
                }),
            total: crate::profile_card::banner::ALL.len(),
        },
        // The history counts nothing.
        Count {
            held: None,
            total: 0,
        },
    ]
}

/// The Collection row's counts, for the Holocrons page, which draws the row without the
/// Collection page's inputs.
pub(crate) fn row_labels(
    enabled: bool,
    snapshot: Option<&sjk_identity::Snapshot>,
    standings: &[Standing],
) -> Vec<String> {
    let inputs = Inputs {
        enabled,
        snapshot,
        holdings: unlockables::Holdings::of(enabled, snapshot),
        setting: "",
        standings,
        illuminate: true,
        cardboard_box: false,
        order66: false,
        hax: false,
        ragdoll: false,
        drone: false,
        boombox: false,
        monitor: false,
        box_hunt: false,
        banana: false,
        fishing: false,
        mining: false,
        name: "",
        stock: color::HOLO,
        grid: false,
        options: "",
        second: "",
        pair: crate::saber_skins::Pair::None,
        share: false,
        nameplate_style: crate::hud::nameplate::Style::Holo,
        toy_keys: &[],
        toys_unseen: 0,
        toy_options: 0,
        tag: "",
        tag_finish: "",
    };
    counts(&inputs).iter().map(|count| count.label()).collect()
}

/// A soft round glow of `colour` at (`x`, `y`) (frame pixels), `radius` across its
/// half, `strength` its alpha at the middle: discs stacked from the rim inwards.
pub(super) fn glow(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    centre: [f32; 2],
    radius: f32,
    colour: Color,
    strength: f32,
) {
    const LAYERS: usize = 8;
    let [x, y] = centre;
    for layer in 0..LAYERS {
        let r = radius * (1.0 - layer as f32 / LAYERS as f32);
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x - r, y - r, r * 2.0, r * 2.0),
            radius: r * frame.s,
            color: color::alpha(colour, strength / LAYERS as f32),
        });
    }
}

/// Lines of `body`, centred in the column from `x` `width` wide, from `y`, `chars` to a
/// line and at most `lines` of them; returns where the next line would stand.
#[allow(clippy::too_many_arguments)]
pub(super) fn centred_lines(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    body: &str,
    [x, y, width]: [f32; 3],
    chars: usize,
    lines: usize,
    size: f32,
    colour: Color,
) -> f32 {
    left_lines(
        canvas,
        frame,
        body,
        [x, y, width],
        (chars, lines),
        size,
        colour,
        TextAlign::Center,
    )
}

/// Lines of `body` from (`x`, `y`) in a column `width` wide, aligned `align`; `fit` is
/// the characters to a line and the most lines. Returns where the next line would
/// stand.
#[allow(clippy::too_many_arguments)]
pub(super) fn left_lines(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    body: &str,
    [x, mut y, width]: [f32; 3],
    (chars, lines): (usize, usize),
    size: f32,
    colour: Color,
    align: TextAlign,
) -> f32 {
    let step = size * 1.42;
    let parts: Vec<&str> = crate::menu::sjk::wrap(body, chars).collect();
    for (index, part) in parts.iter().take(lines).enumerate() {
        let cut = index + 1 == lines && parts.len() > lines;
        text(
            canvas,
            TextFamily::Body,
            format_args!("{part}{}", if cut { "..." } else { "" }),
            frame.rect(x, y, width, step),
            size * frame.s,
            colour,
            FontWeight::Regular,
            align,
        );
        y += step;
    }
    y
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        let body = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        // Inter, where the families are not loaded, is wider than the display family.
        self.widen = if matches!(target, TextTarget::Inter(..)) {
            1.3
        } else {
            1.0
        };
        self.build(inputs, body, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out; `_body` is the body family, kept for measured layouts.
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, _body: &UiFont, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.box_on = inputs.cardboard_box;
        self.order66_on = inputs.order66;
        self.hax_on = inputs.hax;
        self.ragdoll_on = inputs.ragdoll;
        self.drone_on = inputs.drone;
        self.boombox_on = inputs.boombox;
        self.monitor_on = inputs.monitor;
        self.box_hunt_on = inputs.box_hunt;
        self.banana_on = inputs.banana;
        self.fishing_on = inputs.fishing;
        self.mining_on = inputs.mining;
        self.share_on = inputs.share;
        self.toys_unseen = inputs.toys_unseen;
        self.toy_options = inputs.toy_options;
        let seconds = self.epoch.elapsed().as_secs_f32();
        #[cfg(test)]
        let seconds = self.shot_seconds.unwrap_or(seconds);
        self.ui.begin_transparent(viewport);
        let kit = matches!(self.tab, Tab::Shaders | Tab::Toys | Tab::Nameplates);
        if kit && self.backstage != Backstage::None {
            stage_scrims(&mut self.ui, viewport, &frame);
        } else {
            crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        }
        if let Backstage::Preview { ready: true } = self.backstage
            && kit
        {
            // The Shaders tab's preview is its saber on its own.
            let [x, y, width, height] = if self.tab == Tab::Shaders {
                SABER_AREA
            } else {
                MODEL_AREA
            };
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(x, y, width, height),
                texture: crate::ui_renderer::PREVIEW_TEXTURE,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        if self.tab != Tab::History {
            // Not on show: shown again, the History tab reads the newest lines.
            self.history.shown = false;
        }
        let counts = counts(inputs);
        let mut labels: Vec<String> = counts.iter().map(|count| count.label()).collect();
        // The Forge says the shards it trades with, as it read them this frame.
        if let Some(shards) = self.forge.data.shards.filter(|shards| *shards > 0) {
            labels[Tab::Forge.index()] = crate::cards::grouped(shards);
        }
        // The Loadouts card covers the page: text draws over every shape, so the page is
        // left out under it.
        if self.loadouts.is_open() {
            self.loadouts
                .draw(&mut self.ui, &frame, viewport, &self.loadout_slots);
            self.ui.finish(u16::MAX);
            return;
        }
        let asking = self.tab == Tab::Forge && self.forge.ask.is_some();
        let back = if self.hub {
            crate::profile_hub::back_words(self.back)
        } else {
            "Back"
        };
        // A card asking on the Forge covers the page: the header is left out under it.
        if !asking {
            crate::profile_hub::collection_header(
                &mut self.ui,
                &frame,
                back,
                BACK_TOKEN,
                self.tab,
                (&labels, self.widen),
            );
            self.tally(&frame, &counts);
        }
        match self.tab {
            Tab::Medals => self.medals(&frame, inputs, seconds),
            Tab::Achievements => self.achievements(&frame, inputs),
            Tab::Shaders => self.shaders(&frame, inputs, seconds),
            Tab::Toys => self.toys(&frame, inputs),
            Tab::Tags => self.tags(&frame, inputs),
            Tab::Banners => self.banners(&frame, inputs),
            Tab::Cards => self.cards(&frame, inputs, seconds),
            Tab::History => self.history_tab(&frame, inputs),
            Tab::Forge => {
                self.grid = inputs.grid;
                self.forge(&frame, inputs, seconds, viewport);
            }
            _ => self.nameplates(&frame, inputs),
        }
        if self.offers_loadouts() {
            // Under the tally, clear of each tab's lead line and card.
            crate::menu::sjk::kit::button(
                &mut self.ui,
                &frame,
                [
                    RIGHT_X - 200.0,
                    206.0,
                    200.0,
                    crate::menu::sjk::kit::CONTROL_HEIGHT,
                ],
                "Loadouts  (P)",
                false,
                true,
                false,
                super::loadouts::OPEN_TOKEN,
            );
        }
        self.keys(&frame, inputs);
        self.hover_card(&frame, inputs);
        self.ui.finish(self.focus_token());
    }

    /// How many of all the things the player holds, at the top right: the words, and a
    /// bar under them, a stretch to a tab in the tabs' order, each filled as far as its
    /// share held (one tick a thing no longer fits two hundred things, 21/10/2026).
    fn tally(&mut self, frame: &Frame, counts: &[Count; 11]) {
        let s = frame.s;
        // Only the things with a set total: the holocrons are opened, not collected.
        let total: usize = counts.iter().map(|count| count.total).sum();
        let held: usize = counts
            .iter()
            .filter(|count| count.total > 0)
            .filter_map(|count| count.held)
            .sum();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{held} of {total} collected"),
            frame.rect(RIGHT_X - TALLY_WIDTH, 58.0, TALLY_WIDTH, 30.0),
            22.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::End,
        );
        let groups: Vec<Count> = counts.iter().copied().filter(|c| c.total > 0).collect();
        if groups.is_empty() {
            return;
        }
        let stretch = (TALLY_WIDTH - GROUP_GAP * (groups.len() - 1) as f32) / groups.len() as f32;
        let radius = TALLY_HEIGHT / 2.0 * s;
        let mut x = RIGHT_X - TALLY_WIDTH;
        for count in groups {
            let share = count.held.unwrap_or(0).min(count.total) as f32 / count.total as f32;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x, 98.0, stretch, TALLY_HEIGHT),
                radius,
                color: color::alpha(color::HOLO, 0.24),
            });
            if share > 0.0 {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x, 98.0, (stretch * share).max(TALLY_HEIGHT), TALLY_HEIGHT),
                    radius,
                    color: color::GOLD_BRIGHT,
                });
            }
            x += stretch + GROUP_GAP;
        }
    }

    /// A tab's headline at the top left and the line under it.
    pub(super) fn lead(&mut self, frame: &Frame, headline: &str, line: &str) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(LEFT_X, LEAD_Y, 900.0, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if !line.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(LEFT_X, LEAD_Y + 42.0, 940.0, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The keys of the tab on show, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let next_tab = crate::terms::say(self.tab.next(true).label());
        let mut keys: Vec<(&[&str], &str)> = Vec::with_capacity(10);
        match self.tab {
            Tab::Medals => keys.push((&["Left", "Right"], "choose")),
            Tab::History => {
                keys.push((&["Left", "Right"], "filter"));
                keys.push((&["Up", "Down"], "choose"));
                let chosen = self.history.chosen;
                if self.history.more_row(chosen) {
                    keys.push((&["Enter"], "load older"));
                } else if self
                    .history
                    .rows
                    .get(chosen)
                    .is_some_and(|row| row.group.is_some())
                {
                    keys.push((&["Enter"], "open, close"));
                }
                keys.push((&["R"], "refresh"));
            }
            Tab::Cards if self.card_inspect => {
                if self.cards_shown > 1 {
                    keys.push((&["Left", "Right"], "previous, next"));
                }
                if self.chosen_finishes_held() > 1 {
                    keys.push((&["V"], "finish"));
                }
                keys.push((&["Esc"], "put back"));
            }
            Tab::Cards => {
                if self.cards_shown > 0 {
                    keys.push((&["Enter"], "look closer"));
                }
                if self.cards_shown > 1 {
                    keys.push((&["Arrows"], "choose"));
                }
                if self.chosen_finishes_held() > 1 {
                    keys.push((&["V"], "finish"));
                }
                if self.cards_shown > cards::COLUMNS * cards::LINES {
                    keys.push((&["PgUp", "PgDn"], "page"));
                }
                keys.push((&["O", "T", "K", "F"], "filter"));
                if self.revealable > 0 {
                    keys.push((&["R"], "reveal"));
                }
            }
            Tab::Achievements => {
                keys.push((&["Arrows"], "choose"));
                keys.push((&["F"], "General or Fun"));
            }
            Tab::Forge if self.forge.ask.is_some() => {
                keys.push((&["Tab"], "switch"));
                keys.push((&["Enter"], "answer"));
                keys.push((&["Esc"], "cancel"));
            }
            Tab::Forge => {
                keys.push((&["Tab"], self.forge.section.next(true).label()));
                match self.forge.section {
                    forge::Section::Shop => {
                        keys.push((&["Arrows"], "choose"));
                        keys.push((&["Enter"], "buy"));
                    }
                    section => {
                        keys.push((&["Arrows"], "choose"));
                        keys.push((&["V"], if self.grid { "list" } else { "grid" }));
                        keys.push((&["/"], "search"));
                        // The marks (29/10/2026): the star, and the padlock on what is held.
                        if self.marks.kept && section == forge::Section::Break {
                            keys.push((&["F", "L"], "star, lock"));
                        } else if self.marks.kept {
                            keys.push((&["F"], "star"));
                        }
                        if section == forge::Section::Break && self.forge.data.all {
                            keys.push((&["B"], "all duplicates"));
                        }
                        keys.push((
                            &["Enter"],
                            if section == forge::Section::Craft {
                                "craft"
                            } else {
                                "break down"
                            },
                        ));
                    }
                }
            }
            Tab::Shaders => {
                if self.grid {
                    keys.push((&["Arrows"], "choose"));
                    keys.push((&["V"], "list"));
                } else {
                    keys.push((&["Up", "Down"], "choose"));
                    keys.push((&["S"], "sort"));
                    keys.push((&["V"], "tiles"));
                }
                keys.push((&["T"], "tier"));
                keys.push((&["/"], "search"));
                // The marks (29/10/2026): the star and the padlock, and every New one seen.
                if self.marks.kept && self.shader > 0 {
                    keys.push((&["F", "L"], "star, lock"));
                }
                if self.marks.new_count() > 0 {
                    keys.push((&["N"], "all seen"));
                }
                let worn = self.shader_rows.get(self.shader).and_then(|row| row.wear);
                match worn {
                    Some("") if self.shader > 0 => keys.push((&["Enter"], "unequip")),
                    Some(_) => keys.push((&["Enter"], "equip")),
                    None => {}
                }
            }
            Tab::Banners => {
                keys.push((&["Arrows"], "choose"));
                keys.push((&["Enter"], "equip / unequip"));
            }
            Tab::Tags => {
                keys.push((&["Arrows"], "choose"));
                if self.tag_choosable(inputs) {
                    keys.push((&["Enter"], "choose for toy_tag"));
                }
            }
            Tab::Toys if self.bind_wait.is_some() => {
                keys.push((&["Esc"], "cancel"));
                keys.push((&["Backspace"], "clear the key"));
            }
            Tab::Toys => keys.push((
                &["Enter"],
                match (self.toy, inputs.illuminate, inputs.cardboard_box) {
                    (1, _, true) => "come out",
                    (1, _, false) => "hide",
                    (2, _, _) if inputs.order66 => "turn off",
                    (2, _, _) => "turn on",
                    (3, _, _) if inputs.hax => "turn off",
                    (3, _, _) => "turn on",
                    (4, _, _) if inputs.ragdoll => "get up",
                    (4, _, _) => "go limp",
                    (5, _, _) if inputs.drone => "turn off",
                    (5, _, _) => "turn on",
                    (6, _, _) if inputs.boombox => "put away",
                    (6, _, _) => "play",
                    (7, _, _) if inputs.monitor => "turn off",
                    (7, _, _) => "turn on",
                    (8, _, _) if inputs.box_hunt => "turn off",
                    (8, _, _) => "turn on",
                    (9, _, _) if inputs.banana => "turn off",
                    (9, _, _) => "turn on",
                    (10, _, _) if inputs.fishing => "put away",
                    (10, _, _) => "get the rod out",
                    (11, _, _) if inputs.mining => "put away",
                    (11, _, _) => "take the pickaxe",
                    (_, true, _) => "put out",
                    _ => "light",
                },
            )),
            _ => {}
        }
        if self.tab == Tab::Toys && self.bind_wait.is_none() {
            keys.push((&["K"], "bind a key"));
        }
        let covered = (self.tab == Tab::Cards && self.card_inspect)
            || (self.tab == Tab::Forge && self.forge.ask.is_some())
            || (self.tab == Tab::Toys && self.bind_wait.is_some());
        if !covered {
            keys.push((&["Ctrl", "Tab"], &*next_tab));
            keys.push((&["Esc"], "back"));
        }
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(RIGHT_X, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// Where the band behind the title and the tabs holds solid, and where it has faded out,
/// in frame pixels (25/10/2026, Sol: the tabs on top must read over any map).
const HEADER_SOLID: f32 = 200.0;
const HEADER_CLEAR: f32 = 290.0;

/// Dark behind the things on the left, fading out at the stage before the model; a band
/// behind the title and the tabs, solid under them and fading out above the stage, as
/// the Holocrons page's; dark at the bottom behind the keys. The words beside the model
/// stand on cards of their own.
fn stage_scrims(canvas: &mut crate::menu_widgets::MenuCanvas, viewport: [f32; 2], frame: &Frame) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::SPACE, alpha);
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let y = |frame_y: f32| frame.point(0.0, frame_y)[1];
    fade_across(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, x(900.0), height),
        space(0.94),
        space(0.88),
    );
    fade_across(
        canvas,
        sjk_ui::Rect::new(x(900.0), 0.0, x(1_060.0) - x(900.0), height),
        space(0.88),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, width, y(HEADER_SOLID)),
        space(0.86),
        space(0.86),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(
            0.0,
            y(HEADER_SOLID),
            width,
            y(HEADER_CLEAR) - y(HEADER_SOLID),
        ),
        space(0.86),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, y(860.0), width, height - y(860.0)),
        space(0.0),
        space(0.86),
    );
}
