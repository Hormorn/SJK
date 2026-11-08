//! The words in the world over the mystery boxes (and the box hunt's mark, and the
//! players of a quest, `crate::quests`): what a box is and costs, that it rolls, then what
//! came out, for every SJK player who sees it. Drawn as the drone's pilot names are:
//! projected over the point, hidden behind walls and far away. The mark has an arrow
//! besides, seen through walls: over them when they are in view, on a ring round the
//! crosshair pointing their way when not; so have a quest's players, in gold, and the
//! quest has a card of its own under the hunt's.

use super::hunt::Card;
use super::{Here, Opening, ROLL, wire};
use crate::GpuState;
use glam::Vec3;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};
use std::time::Instant;

/// How high over a box's base its words stand (over what floats out of it).
const RISE: f32 = 70.0;
/// Farthest a label is read from, and where it starts to fade.
const RANGE: f32 = 1_400.0;
const FADE_FROM: f32 = 1_000.0;
/// Most labels a frame: the boxes in view and the mark.
const MOST: usize = 16;
/// The box hunt's card at the top of the screen, in pixels at 1080 lines: its size and
/// top, and how long it pulses as a new mark arrives.
const CARD_WIDTH: f32 = 380.0;
const CARD_HEIGHT: f32 = 66.0;
const CARD_TOP: f32 = 56.0;
const CARD_PULSE: f32 = 2.0;
/// The arrow to the mark, in pixels at 1080 lines: its size and the ring round the
/// crosshair it runs on (when the mark is out of view); how far into the view the mark
/// must be for the arrow over them instead (1.0 is the screen's edge).
const ARROW_SIZE: f32 = 15.0;
const ARROW_RING: f32 = 230.0;
const ARROW_IN_VIEW: f32 = 0.9;
/// Most arrows a quest draws: its two targets (each arrow two polygons, its shadow
/// and itself).
const QUEST_ARROWS: usize = 2;
/// The room between the hunt's card and the quest's under it, in pixels at 1080 lines.
const CARD_GAP: f32 = 10.0;

/// How a label's headline is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    /// A box waiting, or rolling.
    Box,
    /// What came out paid.
    Win,
    /// The jackpot.
    Jackpot,
    /// Nothing came out.
    Empty,
    /// The box hunt's mark.
    Mark,
    /// A quest's player or target.
    Quest,
}

impl Tone {
    fn colour(self, alpha: f32) -> Color {
        match self {
            Self::Box => Color::new(1.0, 0.84, 0.42, alpha),
            Self::Win => Color::new(0.55, 0.95, 0.6, alpha),
            Self::Jackpot => Color::new(1.0, 0.45, 0.3, alpha),
            Self::Empty => Color::new(0.72, 0.72, 0.72, alpha),
            Self::Mark => Color::new(1.0, 0.3, 0.25, alpha),
            Self::Quest => Color::new(1.0, 0.76, 0.2, alpha),
        }
    }
}

/// One label: two lines over a point.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Label {
    pub(super) headline: String,
    pub(super) line: String,
    pub(super) tone: Tone,
}

/// This frame's labels and their draw list.
pub(crate) struct Labels {
    pub(crate) list: DrawList,
    lines: Vec<String>,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            // Each label two lines over their shadows, the hunt's card and the arrow to
            // the mark, the quest's card and its arrows.
            list: DrawList::new(MOST * 4 + 12 + 6 + 2 * QUEST_ARROWS),
            lines: Vec::new(),
        }
    }
}

/// What a box's label says: its name and price while it waits, that it rolls, then what
/// came out (the player's own winnings, or what that outcome pays by `pays`).
pub(super) fn box_label(
    here: &Here,
    price: u64,
    free: bool,
    pays: impl Fn(&str) -> Option<u64>,
    now: Instant,
) -> Label {
    let Some(opening) = &here.opening else {
        return Label {
            headline: "MYSTERY BOX".to_owned(),
            line: if free {
                "Free roll".to_owned()
            } else {
                format!("{price} credits")
            },
            tone: Tone::Box,
        };
    };
    if now.saturating_duration_since(opening.started) < ROLL {
        return Label {
            headline: "MYSTERY BOX".to_owned(),
            line: "Rolling...".to_owned(),
            tone: Tone::Box,
        };
    }
    came_out(opening, pays)
}

fn came_out(opening: &Opening, pays: impl Fn(&str) -> Option<u64>) -> Label {
    let won = opening
        .mine
        .as_ref()
        .map(|prize| prize.won)
        .or_else(|| pays(&opening.outcome))
        .unwrap_or(0);
    let (headline, tone) = match opening.outcome.as_str() {
        "jackpot" => ("JACKPOT!", Tone::Jackpot),
        "double" => ("DOUBLE!", Tone::Win),
        "refund" => ("CREDITS BACK", Tone::Win),
        "empty" => ("EMPTY", Tone::Empty),
        wire::EXPLODE => ("BOOM!", Tone::Jackpot),
        wire::HOLOCRON => ("HOLOCRON!", Tone::Jackpot),
        wire::SHARD_PILE => ("KYBER SHARDS", Tone::Win),
        wire::AGAIN => ("FREE BOX!", Tone::Win),
        wire::BLADE => ("MYSTERY BLADE!", Tone::Jackpot),
        _ if won > 0 => ("PRIZE", Tone::Win),
        _ => ("EMPTY", Tone::Empty),
    };
    let tier = opening
        .mine
        .as_ref()
        .and_then(|prize| prize.holocron.as_deref())
        .and_then(crate::holocrons::name);
    let shards = opening.mine.as_ref().and_then(|prize| prize.shards);
    let line = match opening.outcome.as_str() {
        wire::EXPLODE => "It blew up".to_owned(),
        wire::HOLOCRON => tier.unwrap_or("A holocron").to_owned(),
        wire::SHARD_PILE | wire::BLADE if shards.is_some() => {
            format!("+{} kyber shards", shards.unwrap_or(0))
        }
        wire::SHARD_PILE => "A pile of kyber shards".to_owned(),
        wire::AGAIN => "The next roll is free".to_owned(),
        wire::BLADE => "The box's own saber shader".to_owned(),
        _ if won > 0 => format!("+{won} credits"),
        _ => "Nothing inside".to_owned(),
    };
    Label {
        headline: headline.to_owned(),
        line,
        tone,
    }
}

impl GpuState {
    /// Lay this frame's labels out over the view and add their words to the HUD's text;
    /// nothing while `shown` is false (menus, console, scoreboard, intermission).
    pub(crate) fn append_mystery_box_labels(&mut self, viewport: [f32; 2], shown: bool) {
        self.mystery_boxes.labels.list.clear();
        self.mystery_boxes.labels.lines.clear();
        let Some(camera) = self.frame_view.filter(|_| shown) else {
            return;
        };
        let now = Instant::now();
        let boxes = &self.mystery_boxes;
        let pays = |outcome: &str| boxes.pays(outcome);
        let mut wanted: Vec<(Vec3, Vec3, Label)> = boxes
            .here
            .iter()
            .map(|here| {
                (
                    here.origin + Vec3::Z * RISE,
                    here.origin + Vec3::Z * (super::HEIGHT + 4.0),
                    box_label(here, boxes.price, boxes.free > 0, pays, now),
                )
            })
            .collect();
        let mark = self.box_hunt_mark_origin();
        wanted.extend(mark.map(GpuState::box_hunt_mark_label));
        // A quest's players this one should find, where the game server sends them.
        let quest_points: Vec<Vec3> = self
            .quest_marked()
            .into_iter()
            .filter_map(|marked| {
                let origin = self.box_hunt_player_origin(marked.slot)?;
                wanted.push((
                    origin + Vec3::Z * super::hunt::MARK_RISE,
                    origin + Vec3::Z * super::hunt::MARK_SIGHT,
                    Label {
                        headline: marked.headline,
                        line: marked.line,
                        tone: Tone::Quest,
                    },
                ));
                Some(origin + Vec3::Z * super::hunt::MARK_RISE)
            })
            .take(QUEST_ARROWS)
            .collect();
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let width = (420.0 * unit).min(viewport[0]);
        for (point, sight, label) in wanted.into_iter().take(MOST) {
            let distance = point.distance(camera.eye);
            if distance > RANGE {
                continue;
            }
            let Some(at) = camera.project(point) else {
                continue;
            };
            let trace = self.bsp.trace_box_with(
                &mut self.trace_scratch,
                camera.eye.to_array(),
                sight.to_array(),
                sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                1,
            );
            if trace.fraction < 1.0 || trace.start_solid || trace.all_solid {
                continue;
            }
            let alpha = (1.0 - (distance - FADE_FROM) / (RANGE - FADE_FROM)).clamp(0.0, 1.0);
            let near = (1.0 - distance / RANGE).clamp(0.4, 1.0);
            let head = 26.0 * unit * near;
            let small = 17.0 * unit * near;
            let x = (at[0] - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
            let labels = &mut self.mystery_boxes.labels;
            let id = labels.lines.len() as u32;
            labels.lines.push(label.headline);
            labels.lines.push(label.line);
            let rows = [
                (
                    at[1] - head * 1.3 - small * 1.4,
                    head,
                    id,
                    label.tone.colour(alpha),
                ),
                (
                    at[1] - small * 1.4,
                    small,
                    id + 1,
                    Color::new(1.0, 1.0, 1.0, alpha),
                ),
            ];
            for (y, size, text, colour) in rows {
                // A dark shadow under each line so it reads on any wall.
                for (offset, color) in [
                    (1.5 * unit, Color::new(0.0, 0.0, 0.0, 0.75 * alpha)),
                    (0.0, colour),
                ] {
                    let _ = labels.list.push(DrawCommand::Text {
                        rect: Rect::new(x + offset, y + offset, width, size * 1.3),
                        text: TextId(text),
                        size,
                        color,
                        align: TextAlign::Center,
                        overflow: TextOverflow::Ellipsis,
                        weight: FontWeight::Semibold,
                        letter_spacing: 0.0,
                    });
                }
            }
        }
        if let Some(origin) = mark {
            let point = GpuState::box_hunt_mark_label(origin).0;
            push_arrow(
                &mut self.mystery_boxes.labels,
                camera,
                point,
                unit,
                Tone::Mark,
            );
        }
        for point in quest_points {
            push_arrow(
                &mut self.mystery_boxes.labels,
                camera,
                point,
                unit,
                Tone::Quest,
            );
        }
        let mut top = CARD_TOP;
        if let Some(card) = self.mystery_boxes.hunt.card() {
            let since = now.saturating_duration_since(card.since).as_secs_f32();
            push_card(&mut self.mystery_boxes.labels, card, since, viewport, unit);
            top += CARD_HEIGHT + CARD_GAP;
        }
        if let Some(card) = self.quest_card(now) {
            let since = now.saturating_duration_since(card.since).as_secs_f32();
            let fill = if card.own {
                Color::new(0.36, 0.25, 0.03, 0.88)
            } else {
                Color::new(0.06, 0.05, 0.03, 0.82)
            };
            let plate = Plate {
                caption: card.caption,
                line: card.line,
                fill,
                edge: Tone::Quest,
                top,
            };
            push_plate(&mut self.mystery_boxes.labels, plate, since, viewport, unit);
        }
        if self.mystery_boxes.labels.list.commands().is_empty() {
            return;
        }
        let labels = &self.mystery_boxes.labels;
        // In the headline font, as centre prints are (Inter until it loads).
        let (vertices, font) = self.game_fonts.target(
            crate::game_font::RetailFont::Headline,
            &mut self.text_vertices,
            &self.ui_font,
        );
        // The menus' lifted colours, so a black name reads on the dark card and plates.
        crate::ui_renderer::append_text_commands_where(
            &labels.list,
            |id| labels.lines.get(id.0 as usize).map_or("", String::as_str),
            |_, _| true,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
            crate::text::CodePalette::Legible,
        );
    }
}

/// Push the arrow to the mark standing at `point` (over their head), seen through walls:
/// pointing down at them when they are in view, else on a ring round the crosshair
/// pointing their way.
fn push_arrow(
    labels: &mut Labels,
    camera: crate::hud::identification::Camera,
    point: Vec3,
    unit: f32,
    tone: Tone,
) {
    let viewport = camera.viewport;
    let size = ARROW_SIZE * unit;
    let (tip, dir) = match camera.project_within(point, ARROW_IN_VIEW) {
        // Over the mark, under their label's words, pointing down at them.
        Some(at) => ([at[0], at[1] + size], [0.0, 1.0]),
        None => {
            let dir = screen_direction(camera, point);
            let centre = [viewport[0] * 0.5, viewport[1] * 0.5];
            let ring = ARROW_RING * unit;
            ([centre[0] + dir[0] * ring, centre[1] + dir[1] * ring], dir)
        }
    };
    let across = [-dir[1], dir[0]];
    let back = [tip[0] - dir[0] * size * 1.6, tip[1] - dir[1] * size * 1.6];
    let corners = [
        tip,
        [back[0] + across[0] * size, back[1] + across[1] * size],
        [back[0] - across[0] * size, back[1] - across[1] * size],
    ];
    for (offset, color) in [
        (2.0 * unit, Color::new(0.0, 0.0, 0.0, 0.6)),
        (0.0, tone.colour(0.95)),
    ] {
        let mut points = [[0.0; 2]; sjk_ui::POLYGON_POINTS];
        for (to, from) in points.iter_mut().zip(corners) {
            *to = [from[0] + offset, from[1] + offset];
        }
        let _ = labels.list.push(DrawCommand::Polygon {
            points,
            count: corners.len() as u8,
            color,
            texture: None,
        });
    }
}

/// Which way on the screen (x right, y down, unit length) `point` lies from the
/// crosshair, behind the camera too (straight down when right behind it).
fn screen_direction(camera: crate::hud::identification::Camera, point: Vec3) -> [f32; 2] {
    let forward = (camera.target - camera.eye).normalize_or_zero();
    let right = forward.cross(camera.up).normalize_or_zero();
    let up = right.cross(forward);
    let delta = point - camera.eye;
    let (x, y) = (delta.dot(right), -delta.dot(up));
    let length = x.hypot(y);
    if length < 1e-3 {
        return [0.0, 1.0];
    }
    [x / length, y / length]
}

/// What the box hunt's card says: a caption (with the hunters still in the chase, when
/// the hub says, or that the player is out of it), then the mark's name, or that the
/// player is the mark.
fn card_words(card: &Card) -> (String, String) {
    let left = card
        .hunters
        .map(|(alive, all)| format!(" \u{b7} {alive} OF {all} HUNTERS LEFT"))
        .unwrap_or_default();
    if card.own {
        (format!("BOX HUNT{left}"), "YOU ARE THE MARK".to_owned())
    } else if card.out {
        (
            "BOX HUNT \u{b7} YOU'RE OUT".to_owned(),
            format!("{}^7", card.name),
        )
    } else {
        (
            format!("BOX HUNT \u{b7} THE MARK{left}"),
            format!("{}^7", card.name),
        )
    }
}

/// Push the box hunt's card, `since` seconds after it arrived: a dark plate edged in red
/// at the top centre (red itself when the player is the mark), its edge flashing for
/// [`CARD_PULSE`] seconds.
fn push_card(labels: &mut Labels, card: &Card, since: f32, viewport: [f32; 2], unit: f32) {
    let (caption, line) = card_words(card);
    let fill = if card.own {
        Color::new(0.42, 0.05, 0.04, 0.88)
    } else {
        Color::new(0.06, 0.04, 0.04, 0.82)
    };
    let plate = Plate {
        caption,
        line,
        fill,
        edge: Tone::Mark,
        top: CARD_TOP,
    };
    push_plate(labels, plate, since, viewport, unit);
}

/// A card at the top centre: a small caption over a line (colour codes kept), on a plate
/// of `fill` edged in `edge`'s colour, its top `top` pixels down at 1080 lines.
struct Plate {
    caption: String,
    line: String,
    fill: Color,
    edge: Tone,
    top: f32,
}

/// Push `plate`, `since` seconds after it arrived, its edge flashing for [`CARD_PULSE`]
/// seconds.
fn push_plate(labels: &mut Labels, plate: Plate, since: f32, viewport: [f32; 2], unit: f32) {
    let Plate {
        caption,
        line,
        fill,
        edge,
        top,
    } = plate;
    let width = (CARD_WIDTH * unit).min(viewport[0]);
    let height = CARD_HEIGHT * unit;
    let rect = Rect::new((viewport[0] - width) * 0.5, top * unit, width, height);
    let pulse = (1.0 - since / CARD_PULSE).clamp(0.0, 1.0);
    let flash = pulse * (0.5 + 0.5 * (since * std::f32::consts::TAU * 2.0).cos());
    let radius = 10.0 * unit;
    let _ = labels.list.push(DrawCommand::RoundedRect {
        rect,
        radius,
        color: fill,
    });
    let _ = labels.list.push(DrawCommand::Border {
        rect,
        radius,
        width: (2.0 + 3.0 * flash) * unit,
        color: edge.colour(0.55 + 0.45 * flash),
    });
    let id = labels.lines.len() as u32;
    // Each line's shadow is the line without its colour codes, all of it dark.
    let plain = crate::text::Plain(&line).to_string();
    labels.lines.push(caption);
    labels.lines.push(line);
    labels.lines.push(plain);
    let pad = 14.0 * unit;
    let small = 15.0 * unit;
    let big = 28.0 * unit;
    let top = rect.y + 7.0 * unit;
    let rows = [
        (top, small, id, id, Tone::Box.colour(1.0)),
        (
            top + small * 1.3,
            big,
            id + 1,
            id + 2,
            Color::new(1.0, 1.0, 1.0, 1.0),
        ),
    ];
    for (y, size, text, shadow, colour) in rows {
        for (offset, text, color) in [
            (1.5 * unit, shadow, Color::new(0.0, 0.0, 0.0, 0.7)),
            (0.0, text, colour),
        ] {
            let _ = labels.list.push(DrawCommand::Text {
                rect: Rect::new(
                    rect.x + pad + offset,
                    y + offset,
                    width - 2.0 * pad,
                    size * 1.3,
                ),
                text: TextId(text),
                size,
                color,
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight: FontWeight::Semibold,
                letter_spacing: 0.0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Prize, prize};
    use super::*;
    use std::time::Duration;

    fn here(opening: Option<Opening>) -> Here {
        Here {
            id: 1,
            origin: Vec3::ZERO,
            yaw: 0.0,
            opening,
            lid: Default::default(),
        }
    }

    fn opening(outcome: &str, mine: Option<Prize>, started: Instant) -> Opening {
        Opening {
            started,
            outcome: outcome.to_owned(),
            prize: prize(outcome, true, None),
            seed: 0,
            mine,
            opener: None,
            revealed: false,
            blasted: false,
        }
    }

    #[test]
    fn a_label_tells_the_box_the_roll_and_what_came_out() {
        let now = Instant::now();
        let pays = |outcome: &str| (outcome == "jackpot").then_some(500);
        let waiting = box_label(&here(None), 100, false, pays, now);
        assert_eq!(
            (waiting.headline.as_str(), waiting.line.as_str()),
            ("MYSTERY BOX", "100 credits")
        );
        assert_eq!(
            box_label(&here(None), 100, true, pays, now).line,
            "Free roll"
        );
        let rolling = here(Some(opening("jackpot", None, now)));
        assert_eq!(
            box_label(&rolling, 100, false, pays, now).line,
            "Rolling..."
        );
        // Another player's jackpot: what that outcome pays.
        let later = now + ROLL + Duration::from_millis(10);
        let theirs = box_label(&rolling, 100, false, pays, later);
        assert_eq!(
            (theirs.headline.as_str(), theirs.line.as_str()),
            ("JACKPOT!", "+500 credits")
        );
        assert_eq!(theirs.tone, Tone::Jackpot);
        // The player's own: their winnings, and how to take them.
        let mine = Prize {
            price: 100,
            won: 200,
            balance: 300,
            ..Prize::default()
        };
        let own = here(Some(opening("double", Some(mine), now)));
        assert_eq!(
            box_label(&own, 100, false, pays, later).line,
            "+200 credits"
        );
        let empty = here(Some(opening("empty", None, now)));
        let empty = box_label(&empty, 100, false, pays, later);
        assert_eq!(
            (empty.headline.as_str(), empty.tone),
            ("EMPTY", Tone::Empty)
        );
        // A box that blew up, and a holocron: the opener's names its tier.
        let blown = here(Some(opening(wire::EXPLODE, None, now)));
        let blown = box_label(&blown, 100, false, pays, later);
        assert_eq!(
            (blown.headline.as_str(), blown.line.as_str()),
            ("BOOM!", "It blew up")
        );
        let found = Prize {
            holocron: Some("rare".to_owned()),
            ..Prize::default()
        };
        let found = here(Some(opening(wire::HOLOCRON, Some(found), now)));
        let found = box_label(&found, 100, false, pays, later);
        assert_eq!(found.headline, "HOLOCRON!");
        assert!(found.line == "Rare Holocron", "{}", found.line);
    }

    #[test]
    fn the_hunt_card_names_the_mark_or_tells_the_player_they_are_it() {
        let card = |own| Card {
            slot: 3,
            name: "^1Ky^7le".to_owned(),
            own,
            since: Instant::now(),
            hunters: None,
            out: false,
        };
        let (caption, line) = card_words(&card(false));
        assert!(caption.ends_with("THE MARK"));
        // The hunters left, and a player out of the chase.
        let counted = Card {
            hunters: Some((2, 3)),
            ..card(false)
        };
        assert!(
            card_words(&counted)
                .0
                .ends_with("THE MARK \u{b7} 2 OF 3 HUNTERS LEFT")
        );
        let out = Card {
            out: true,
            ..counted.clone()
        };
        assert!(card_words(&out).0.ends_with("YOU'RE OUT"));
        // The name's colours do not run past it.
        assert_eq!(line, "^1Ky^7le^7");
        assert_eq!(card_words(&card(true)).1, "YOU ARE THE MARK");
        // A plate, its edge, then two lines over their shadows, at the top centre.
        let mut labels = Labels::default();
        push_card(&mut labels, &card(false), 0.0, [1920.0, 1080.0], 1.0);
        let commands = labels.list.commands();
        assert_eq!(commands.len(), 6);
        let DrawCommand::RoundedRect { rect, .. } = commands[0] else {
            panic!("{:?}", commands[0]);
        };
        assert_eq!((rect.x + rect.width * 0.5, rect.y), (960.0, CARD_TOP));
        // The edge is widest as the card arrives and settles once it has pulsed.
        let edge = |since| {
            let mut labels = Labels::default();
            push_card(&mut labels, &card(false), since, [1920.0, 1080.0], 1.0);
            match labels.list.commands()[1] {
                DrawCommand::Border { width, .. } => width,
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(edge(0.0), 5.0);
        assert_eq!(edge(CARD_PULSE + 1.0), 2.0);
    }
}
