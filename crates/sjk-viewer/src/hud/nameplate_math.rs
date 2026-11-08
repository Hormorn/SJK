//! Nameplate maths: how large, how opaque and where a plate sits, the holo frame it is
//! drawn as ([`Holo`]), and the colours and rectangles of its bars. Pure functions,
//! so the layout is testable without a window; [`super::nameplate`] collects the
//! players and draws with these.
use sjk_ui::{Color, Rect};

/// Distance, in world units, out to which a plate keeps its full size.
const FULL_SIZE_DISTANCE: f32 = 300.0;
/// Share of the range, from the camera, over which a plate is fully opaque.
const OPAQUE_SHARE: f32 = 0.75;
/// Units above the top of the player's box at which the plate is anchored.
pub(crate) const HEAD_CLEARANCE: f32 = 8.0;
/// Standing box top above the origin, used when the entity sends none.
const STANDING_TOP: f32 = 40.0;
/// Milliseconds a plate takes to fade between hidden and shown.
pub(crate) const FADE_MILLIS: f32 = 120.0;
/// Share of the detail distance over which the bars fade in on approach.
const DETAIL_BLEND: f32 = 0.25;

/// Size multiplier for a plate `distance` away: full size up close, shrinking
/// linearly to `minimum` at `range`.
pub(crate) fn distance_scale(distance: f32, range: f32, minimum: f32) -> f32 {
    if range <= FULL_SIZE_DISTANCE {
        return 1.0;
    }
    let t = ((distance - FULL_SIZE_DISTANCE) / (range - FULL_SIZE_DISTANCE)).clamp(0.0, 1.0);
    1.0 + (minimum.clamp(0.0, 1.0) - 1.0) * t
}

/// Opacity for a plate `distance` away: opaque for the first part of the range,
/// then fading to nothing at `range`.
pub(crate) fn distance_fade(distance: f32, range: f32) -> f32 {
    let start = range * OPAQUE_SHARE;
    if distance <= start || range <= start {
        return 1.0;
    }
    (1.0 - (distance - start) / (range - start)).clamp(0.0, 1.0)
}

/// How much of the plate (the bars) shows at `distance`: none beyond `near`,
/// all of it within the inner part of `near`, blending between.
pub(crate) fn detail(distance: f32, near: f32) -> f32 {
    if near <= 0.0 {
        return 0.0;
    }
    ((near - distance) / (near * DETAIL_BLEND)).clamp(0.0, 1.0)
}

/// Height of the top of an entity's box above its origin, from the packed
/// `entityState_t::solid` (`SV_LinkEntity`: bits 16-23 hold top + 32). A
/// player who sends no box is taken as standing.
pub(crate) fn head_height(solid: u32) -> f32 {
    let packed = (solid >> 16) & 255;
    if solid == 0 || packed == 0 {
        STANDING_TOP
    } else {
        packed as f32 - 32.0
    }
}

/// Where a player's plate hangs: the world point it is traced from and the
/// height of the box top above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Anchor {
    pub(crate) origin: [f32; 3],
    pub(crate) head: f32,
}

/// A presented body: its interpolated origin and packed `entityState_t::solid`.
pub(crate) type Body = ([f32; 3], u32);

/// The anchor of a player whose own body is `own` and whose vehicle's body is
/// `vehicle`, each `None` when the world does not present it.
///
/// A pilot inside a `hideRider` vehicle carries `EF_NODRAW`, so the client never
/// presents his body (`CG_AddCEntity` skips it); his plate hangs over the
/// vehicle he sits in instead. A rider whose body is presented keeps his own.
pub(crate) fn anchor(own: Option<Body>, vehicle: Option<Body>) -> Option<Anchor> {
    let (origin, solid) = own.or(vehicle)?;
    Some(Anchor {
        origin,
        head: head_height(solid),
    })
}

/// `CLASS_VEHICLE` (`teams.h`).
const CLASS_VEHICLE: u8 = 53;
/// `ET_NPC` (`entityType_t`).
const ET_NPC: u8 = 13;
/// `MAX_CLIENTS`.
const MAX_CLIENTS: u16 = 32;

/// The client sealed in a vehicle whose snapshot state is given, if his own entity
/// is not in the snapshot: stock cgame names the pilot of an `ET_NPC` /
/// `CLASS_VEHICLE` entity from its `owner` while that is below `MAX_CLIENTS`
/// (`CG_DrawCrosshair`). The viewer is not named to himself, and a pilot whose
/// entity is sent (`present`) keeps the ordinary plate.
pub(crate) fn hidden_pilot(
    entity_type: u8,
    npc_class: u8,
    owner: u16,
    local: u16,
    present: impl FnOnce(u16) -> bool,
) -> Option<u16> {
    (entity_type == ET_NPC
        && npc_class == CLASS_VEHICLE
        && owner < MAX_CLIENTS
        && owner != local
        && !present(owner))
    .then_some(owner)
}

/// Bar fills until the HUD in use names its own: the retail HUD's red health,
/// green shield (armour) and light blue Force.
pub(crate) const HEALTH_COLOR: Color = Color::new(1.0, 0.25, 0.2, 1.0);
pub(crate) const SHIELD_COLOR: Color = Color::new(0.3, 0.9, 0.35, 1.0);
pub(crate) const FORCE_COLOR: Color = Color::new(0.36, 0.7, 1.0, 1.0);
/// The grey of an empty shield's broken bar.
pub(crate) const EMPTY_SHIELD: Color = Color::new(0.55, 0.57, 0.6, 0.7);
/// The "?" over a bar the estimate cannot fill.
pub(crate) const UNKNOWN_MARK: Color = Color::new(1.0, 0.85, 0.15, 1.0);

/// A deeper, more saturated `color`: the second layer of a bar over its maximum
/// (overheal, overshield). Each channel moves away from the mean and the whole
/// darkens, so it reads as the same colour, deeper, even for a green that is
/// already near full.
pub(crate) fn saturated(color: Color) -> Color {
    let mean = (color.r + color.g + color.b) / 3.0;
    let push = |channel: f32| ((mean + (channel - mean) * 1.8) * 0.72).clamp(0.0, 1.0);
    Color::new(push(color.r), push(color.g), push(color.b), color.a)
}

/// Where a bar's second layer (over the maximum) is drawn in `bar`: an inner band,
/// so the full bar shows round it. The nameplates and the HUD's meters share it.
pub(crate) fn overflow_band(bar: Rect) -> Rect {
    let inset = bar.height * 0.22;
    Rect::new(bar.x, bar.y + inset, bar.width, bar.height - inset * 2.0)
}

/// The pieces of a dashed run `width` pixels long from `x`: `on` pixels drawn, `off`
/// left out, as `(start, end)` pixels, the last piece cut to the run. The bars'
/// segments, an empty shield's broken bar and the dotted line of a plate without bars.
pub(crate) fn dashes(x: f32, width: f32, on: f32, off: f32) -> impl Iterator<Item = (f32, f32)> {
    let step = (on + off).max(0.5);
    let count = if width > 0.0 && on > 0.0 {
        (width / step).ceil() as usize
    } else {
        0
    };
    (0..count).filter_map(move |index| {
        let start = x + index as f32 * step;
        let end = (start + on).min(x + width);
        (end > start).then_some((start, end))
    })
}

/// A bar `width` pixels long from `x` cut into whole segments about `on` pixels long
/// with `off` between them, stretched to fill it, as `(start, end)` pixels.
pub(crate) fn segments(x: f32, width: f32, on: f32, off: f32) -> impl Iterator<Item = (f32, f32)> {
    let count = ((width + off) / (on + off).max(0.5)).round().max(1.0);
    let on = ((width - off * (count - 1.0)) / count).max(0.0);
    dashes(x, width, on, off)
}

/// Which bars a plate holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rows {
    pub(crate) health: bool,
    pub(crate) shield: bool,
    pub(crate) force: bool,
}

impl Rows {
    pub(crate) fn any(self) -> bool {
        self.health || self.shield || self.force
    }
}

/// The holo frame's measures are its design's (28/10/2026, design C), in that design's
/// pixels, whose name is this tall: a plate scales them by its name's size over it.
const DESIGN_NAME: f32 = 15.0;
/// The plate's least width, its cut top-right corner and its edge's width.
const PLATE_WIDTH: f32 = 150.0;
const CUT: f32 = 8.0;
pub(crate) const EDGE: f32 = 1.0;
/// The room after the name, and before it on a plate without a picture (an NPC's).
const PAD_RIGHT: f32 = 9.0;
const BARE_LEFT: f32 = 9.0;
/// The name's row.
const NAME_ROW: f32 = 15.0;
/// The verified seal's side, and the gap before it.
const MARK: f32 = 11.0;
const MARK_GAP: f32 = 4.0;
/// The side of another client's icon after the mark (Sol, 08/11/2026: "in small").
const CLIENT: f32 = 9.0;
/// The widest the name runs before its ellipsis.
const NAME_MAX: f32 = 160.0;
/// The bars, top to bottom (shield, health, Force), the gap over each and the room
/// between the name and the first.
const BAR_SHIELD: f32 = 3.0;
const BAR_HEALTH: f32 = 5.0;
const BAR_FORCE: f32 = 2.0;
const BAR_GAP: f32 = 2.0;
const NAME_GAP: f32 = 1.0;
/// The shield's and health's segments, and the gaps between them.
pub(crate) const SEGMENT: f32 = 9.0;
pub(crate) const SEGMENT_GAP: f32 = 2.0;
/// The dotted line under the name of a plate without bars: its dots, the gaps
/// between them, its thickness and the room over it.
pub(crate) const DOT: f32 = 3.0;
pub(crate) const DOT_GAP: f32 = 4.0;
const DOTTED: f32 = 2.0;
const DOTTED_GAP: f32 = 3.0;
/// An empty shield's dashes and the gaps between them.
pub(crate) const EMPTY_DASH: f32 = 4.0;
pub(crate) const EMPTY_GAP: f32 = 3.0;
/// The gold corner of a player who shares: how far along the top it starts from the
/// right end, and how far down the right side it runs; its width.
const GOLD_ALONG: f32 = 34.0;
const GOLD_DOWN: f32 = 22.0;
pub(crate) const GOLD_WIDTH: f32 = 2.0;
/// The scan lines' spacing over the plate.
pub(crate) const SCAN: f32 = 3.0;
/// Far away: the hex beside the name, the gap between them, and the hairline under
/// them (its width and the room over it).
const FAR_HEX: [f32; 2] = [17.0, 15.0];
const FAR_GAP: f32 = 4.0;
const HAIRLINE: f32 = 46.0;
const HAIRLINE_GAP: f32 = 2.0;
/// The duel badge on the hex's bottom-right corner, near and far, and how far out it
/// stands.
const BADGE: f32 = 17.0;
const FAR_BADGE: f32 = 11.0;
const BADGE_OUT: f32 = 5.0;
/// The power icons over the plate's right end: their square cells' side (1.5 times the
/// design's 15, Sol, 28/10/2026), the gap between them, the room from the plate's right
/// end and under the row.
const CHIP: f32 = 22.5;
const CHIP_GAP: f32 = 2.0;
const CHIP_RIGHT: f32 = 2.0;
const CHIP_LIFT: f32 = 4.0;

/// A plate's measures with no bars (names only, a player who does not share, a duel)
/// and with them: the hex's size, how far it overlaps the plate, and the plate's room
/// over the name, before it and under the last row.
#[derive(Clone, Copy)]
struct Measures {
    hex: [f32; 2],
    overlap: f32,
    top: f32,
    left: f32,
    bottom: f32,
}

const SHORT: Measures = Measures {
    hex: [38.0, 34.0],
    overlap: 10.0,
    top: 5.0,
    left: 16.0,
    bottom: 5.0,
};
const TALL: Measures = Measures {
    hex: [44.0, 40.0],
    overlap: 12.0,
    top: 4.0,
    left: 18.0,
    bottom: 4.0,
};

/// The points of a hexagon pointed left and right in `rect`, its top and bottom edges
/// running over the middle half, clockwise from the left point.
pub(crate) fn hexagon(rect: Rect) -> [[f32; 2]; 6] {
    let middle = rect.y + rect.height * 0.5;
    let quarter = rect.width * 0.25;
    [
        [rect.x, middle],
        [rect.x + quarter, rect.y],
        [rect.right() - quarter, rect.y],
        [rect.right(), middle],
        [rect.right() - quarter, rect.bottom()],
        [rect.x + quarter, rect.bottom()],
    ]
}

/// `rect` shrunk by `inset` on every side.
pub(crate) fn inset(rect: Rect, inset: f32) -> Rect {
    Rect::new(
        rect.x + inset,
        rect.y + inset,
        (rect.width - inset * 2.0).max(0.0),
        (rect.height - inset * 2.0).max(0.0),
    )
}

/// The texture coordinates (top-left, bottom-right) a round-cut picture is spread over
/// a [`hexagon`] with: the hexagon's corners land just inside the picture's disc, so
/// none of the clear outside shows.
pub(crate) const HEX_UV: [[f32; 2]; 2] = {
    const REACH: f32 = 0.49;
    // A regular hexagon's corners: half its width across, sin 60° of it up and down.
    const RISE: f32 = REACH * 0.866;
    [[0.5 - REACH, 0.5 - RISE], [0.5 + REACH, 0.5 + RISE]]
};

/// What [`Holo::new`] lays out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HoloSpec {
    /// The point the plate is centred on, and its bottom.
    pub(crate) centre_x: f32,
    pub(crate) bottom: f32,
    /// The name's text size and drawn width, in pixels.
    pub(crate) size: f32,
    pub(crate) name_width: f32,
    pub(crate) picture: bool,
    /// The mark after the name (the verified seal or a trust mark).
    pub(crate) mark: bool,
    /// Another client's icon after the mark (`chat::client_icons`).
    pub(crate) client: bool,
    /// The duel badge on the picture.
    pub(crate) duel: bool,
    /// The bars; none, a dotted line under the name instead.
    pub(crate) rows: Rows,
    /// How much of the bars shows, 0 to 1 ([`detail`]): the plate grows to hold them
    /// as they fade in.
    pub(crate) detail: f32,
    /// How close the player is, 0 (far) to 1 ([`detail`]'s ramp, bars or not): far away
    /// the plate fades and a small hex and the name stay, over a hairline.
    pub(crate) proximity: f32,
    /// The thinnest a bar or line may be, in pixels.
    pub(crate) least_bar: f32,
    pub(crate) viewport: [f32; 2],
}

/// The bars inside a plate, top to bottom, in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Bars {
    pub(crate) shield: Option<Rect>,
    pub(crate) health: Option<Rect>,
    pub(crate) force: Option<Rect>,
}

/// A nameplate as a holo frame (design C, Sol's pick of 28/10/2026): a hexagon holding
/// the picture, overlapping the left end of a plate with a cut top-right corner, which
/// holds the name with its mark and, under it, the bars (or a dotted line without
/// them); the power icons stand over the plate's right end. Far away the plate fades
/// and a small hexagon and the name stay over a hairline. Physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Holo {
    /// The design's pixel in physical ones.
    pub(crate) k: f32,
    /// The plate's box, and its cut corner's run along either side.
    pub(crate) plate: Rect,
    pub(crate) cut: f32,
    /// The hexagon's box.
    pub(crate) hex: Option<Rect>,
    /// Where the name runs: its left edge and the most it may take.
    pub(crate) name: Rect,
    /// The name's capitals are centred on this height.
    pub(crate) middle: f32,
    pub(crate) mark: Option<Rect>,
    pub(crate) client: Option<Rect>,
    pub(crate) badge: Option<Rect>,
    pub(crate) bars: Bars,
    /// The dotted line under the name of a plate without bars.
    pub(crate) dotted: Rect,
    /// The hairline under a far plate's name.
    pub(crate) hairline: Rect,
}

impl Holo {
    pub(crate) fn new(spec: &HoloSpec) -> Self {
        let k = spec.size / DESIGN_NAME;
        let near = spec.proximity.clamp(0.0, 1.0);
        let detail = if spec.rows.any() {
            spec.detail.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let lerp = |from: f32, to: f32, t: f32| from + (to - from) * t;
        let measure = |pick: fn(&Measures) -> f32| lerp(pick(&SHORT), pick(&TALL), detail) * k;
        let line = |height: f32| (height * k).max(spec.least_bar);
        let name_width = spec.name_width.clamp(0.0, NAME_MAX * k);
        let mark_width = if spec.mark {
            (MARK_GAP + MARK) * k
        } else {
            0.0
        };
        // What follows the name: the mark, then a client's icon.
        let tail_width = mark_width
            + if spec.client {
                (MARK_GAP + CLIENT) * k
            } else {
                0.0
            };
        let left = if spec.picture {
            measure(|m| m.left)
        } else {
            BARE_LEFT * k
        };
        let width = (PLATE_WIDTH * k)
            .max(left + name_width + tail_width + PAD_RIGHT * k)
            .min(spec.viewport[0]);
        let bars = [
            (spec.rows.shield, line(BAR_SHIELD)),
            (spec.rows.health, line(BAR_HEALTH)),
            (spec.rows.force, line(BAR_FORCE)),
        ];
        let short_height = (SHORT.top + NAME_ROW + DOTTED_GAP + DOTTED + SHORT.bottom) * k;
        let tall_height = if spec.rows.any() {
            (TALL.top + NAME_ROW + NAME_GAP + TALL.bottom) * k
                + bars
                    .iter()
                    .filter(|(wanted, _)| *wanted)
                    .map(|(_, height)| BAR_GAP * k + height)
                    .sum::<f32>()
        } else {
            short_height
        };
        let height = lerp(short_height, tall_height, detail);
        let near_hex = [
            lerp(SHORT.hex[0], TALL.hex[0], detail) * k,
            lerp(SHORT.hex[1], TALL.hex[1], detail) * k,
        ];
        let overlap = measure(|m| m.overlap);
        // Near, the hexagon and the plate are centred together; far, the small hexagon
        // and the name.
        let fits = |left: f32, width: f32| left.clamp(0.0, (spec.viewport[0] - width).max(0.0));
        let (hex_width, joined) = if spec.picture {
            (near_hex[0], near_hex[0] - overlap + width)
        } else {
            (0.0, width)
        };
        let near_left = fits(spec.centre_x - joined * 0.5, joined);
        let plate_x = near_left
            + if spec.picture {
                hex_width - overlap
            } else {
                0.0
            };
        let far_hex = if spec.picture {
            FAR_HEX[0] * k + FAR_GAP * k
        } else {
            0.0
        };
        let far_width = far_hex + name_width + tail_width;
        let far_left = fits(spec.centre_x - far_width * 0.5, far_width);
        let plate = Rect::new(plate_x, spec.bottom - height, width, height);
        let near_middle = plate.y + measure(|m| m.top) + NAME_ROW * 0.5 * k;
        let hairline_y = spec.bottom - line(EDGE);
        let far_middle = hairline_y - HAIRLINE_GAP * k - NAME_ROW * 0.5 * k;
        let middle = lerp(far_middle, near_middle, near);
        let name_x = lerp(far_left + far_hex, plate.x + left, near);
        let hex = spec.picture.then(|| {
            let size = [
                lerp(FAR_HEX[0] * k, near_hex[0], near),
                lerp(FAR_HEX[1] * k, near_hex[1], near),
            ];
            let centre = lerp(far_middle, plate.y + height * 0.5, near);
            Rect::new(
                lerp(far_left, near_left, near),
                centre - size[1] * 0.5,
                size[0],
                size[1],
            )
        });
        let name = Rect::new(
            name_x,
            middle - NAME_ROW * 0.5 * k,
            name_width,
            NAME_ROW * k,
        );
        let square = |left: f32, side: f32| Rect::new(left, middle - side * 0.5, side, side);
        let mark = spec
            .mark
            .then(|| square(name.right() + MARK_GAP * k, MARK * k));
        let client = spec
            .client
            .then(|| square(name.right() + mark_width + MARK_GAP * k, CLIENT * k));
        let badge = spec.duel.then(|| match hex {
            Some(hex) => {
                let side = lerp(FAR_BADGE, BADGE, near) * k;
                let out = BADGE_OUT * k * side / (BADGE * k);
                Rect::new(
                    hex.right() + out - side,
                    hex.bottom() + out - side,
                    side,
                    side,
                )
            }
            None => square(
                name.right() + tail_width + MARK_GAP * k,
                lerp(FAR_BADGE, BADGE, near) * k,
            ),
        });
        let inner_x = plate.x + left;
        let inner_width = (plate.right() - PAD_RIGHT * k - inner_x).max(0.0);
        // The bars stand on the plate's bottom, so they keep their places as it grows.
        let mut cursor = plate.bottom() - measure(|m| m.bottom);
        let mut stack = [None; 3];
        for (slot, (wanted, height)) in stack.iter_mut().zip(bars).rev() {
            if wanted {
                cursor -= height;
                *slot = Some(Rect::new(inner_x, cursor, inner_width, height));
                cursor -= BAR_GAP * k;
            }
        }
        let dotted = Rect::new(
            inner_x,
            near_middle + NAME_ROW * 0.5 * k + DOTTED_GAP * k,
            inner_width,
            line(DOTTED),
        );
        let hairline_width = HAIRLINE * k;
        let hairline = Rect::new(
            fits(
                far_left + far_width * 0.5 - hairline_width * 0.5,
                hairline_width,
            ),
            hairline_y,
            hairline_width,
            line(EDGE),
        );
        Self {
            k,
            plate,
            cut: CUT * k,
            hex,
            name,
            middle,
            mark,
            client,
            badge,
            bars: Bars {
                shield: stack[0],
                health: stack[1],
                force: stack[2],
            },
            dotted,
            hairline,
        }
    }

    /// The plate's corners, clockwise from the top-left: its top-right one is cut.
    pub(crate) fn outline(&self) -> [[f32; 2]; 5] {
        let plate = self.plate;
        let cut = self.cut.min(plate.height).min(plate.width);
        [
            [plate.x, plate.y],
            [plate.right() - cut, plate.y],
            [plate.right(), plate.y + cut],
            [plate.right(), plate.bottom()],
            [plate.x, plate.bottom()],
        ]
    }

    /// The gold corner of a player who shares, along the top to the cut corner and
    /// down the right side: its three points.
    pub(crate) fn gold(&self) -> [[f32; 2]; 3] {
        let plate = self.plate;
        let cut = self.cut.min(plate.height);
        let half = GOLD_WIDTH * self.k * 0.25;
        [
            [plate.right() - GOLD_ALONG * self.k, plate.y + half],
            [plate.right() - cut, plate.y + half],
            [
                plate.right() - half,
                plate.y + (GOLD_DOWN * self.k).min(plate.height),
            ],
        ]
    }

    /// The power icons' cells, `count` of them, over the plate's right end: the
    /// first's rectangle and the step to the next.
    pub(crate) fn chips(&self, count: usize, viewport: [f32; 2]) -> (Rect, f32) {
        let side = CHIP * self.k;
        let step = side + CHIP_GAP * self.k;
        let width = count as f32 * step - CHIP_GAP * self.k;
        let right = self.plate.right() - CHIP_RIGHT * self.k;
        let left = (right - width).clamp(0.0, (viewport[0] - width).max(0.0));
        (
            Rect::new(left, self.plate.y - CHIP_LIFT * self.k - side, side, side),
            step,
        )
    }
}

/// The plate's fill under a frame of colour `frame`: the frame's colour made dark, so
/// a team's or a duel's plate is tinted with it as the design's are.
pub(crate) fn plate_fill(frame: Color, alpha: f32) -> Color {
    let dark = |channel: f32| channel * 0.1;
    Color::new(dark(frame.r), dark(frame.g), dark(frame.b), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plates_shrink_with_distance_to_the_minimum() {
        assert_eq!(distance_scale(100.0, 3000.0, 0.5), 1.0);
        assert_eq!(distance_scale(300.0, 3000.0, 0.5), 1.0);
        assert!((distance_scale(1650.0, 3000.0, 0.5) - 0.75).abs() < 1e-5);
        assert!((distance_scale(3000.0, 3000.0, 0.5) - 0.5).abs() < 1e-5);
        assert!((distance_scale(9000.0, 3000.0, 0.5) - 0.5).abs() < 1e-5);
        assert_eq!(distance_scale(1000.0, 200.0, 0.5), 1.0);
    }

    #[test]
    fn plates_fade_out_over_the_last_quarter_of_the_range() {
        assert_eq!(distance_fade(0.0, 4000.0), 1.0);
        assert_eq!(distance_fade(3000.0, 4000.0), 1.0);
        assert!((distance_fade(3500.0, 4000.0) - 0.5).abs() < 1e-5);
        assert_eq!(distance_fade(4000.0, 4000.0), 0.0);
        assert_eq!(distance_fade(9000.0, 4000.0), 0.0);
    }

    #[test]
    fn bars_fade_in_between_far_and_close() {
        assert_eq!(detail(1500.0, 1000.0), 0.0);
        assert_eq!(detail(1000.0, 1000.0), 0.0);
        assert!((detail(875.0, 1000.0) - 0.5).abs() < 1e-5);
        assert_eq!(detail(750.0, 1000.0), 1.0);
        assert_eq!(detail(10.0, 1000.0), 1.0);
        assert_eq!(detail(10.0, 0.0), 0.0);
    }

    #[test]
    fn head_height_unpacks_the_encoded_box() {
        // Standing player: x 15, mins z -24 (24), maxs z 40 -> 72.
        let standing = (72 << 16) | (24 << 8) | 15;
        assert_eq!(head_height(standing), 40.0);
        // Crouching: maxs z 16 -> 48.
        let crouching = (48 << 16) | (24 << 8) | 15;
        assert_eq!(head_height(crouching), 16.0);
        assert_eq!(head_height(0), 40.0);
    }

    #[test]
    fn a_hidden_pilot_is_anchored_over_his_vehicle() {
        let ship = ([100.0, 50.0, 400.0], 96 << 16);
        let body = ([1.0, 2.0, 3.0], 72 << 16);
        // Presented body wins, even inside a presented vehicle.
        let own = anchor(Some(body), Some(ship)).unwrap();
        assert_eq!((own.origin, own.head), (body.0, 40.0));
        // No body (EF_NODRAW): the vehicle's origin and box top.
        let hidden = anchor(None, Some(ship)).unwrap();
        assert_eq!((hidden.origin, hidden.head), (ship.0, 64.0));
        // A vehicle that sends no box falls back to the standing height.
        let bare = anchor(None, Some((ship.0, 0))).unwrap();
        assert_eq!(bare.head, 40.0);
        // Nothing presented, nothing to anchor to.
        assert_eq!(anchor(None, None), None);
    }

    #[test]
    fn a_vehicles_owner_below_max_clients_is_its_hidden_pilot() {
        let never = |_| false;
        assert_eq!(hidden_pilot(13, 53, 5, 0, never), Some(5));
        // Not a vehicle, an empty one (ENTITYNUM_NONE), an NPC pilot, or myself.
        assert_eq!(hidden_pilot(13, 20, 5, 0, never), None);
        assert_eq!(hidden_pilot(1, 53, 5, 0, never), None);
        assert_eq!(hidden_pilot(13, 53, 1023, 0, never), None);
        assert_eq!(hidden_pilot(13, 53, 40, 0, never), None);
        assert_eq!(hidden_pilot(13, 53, 5, 5, never), None);
        // A pilot whose own entity is in the snapshot is drawn from it.
        assert_eq!(hidden_pilot(13, 53, 5, 0, |pilot| pilot == 5), None);
    }

    #[test]
    fn the_overflow_layer_is_the_same_hue_stronger() {
        for color in [HEALTH_COLOR, SHIELD_COLOR] {
            let deep = saturated(color);
            let spread = |c: Color| c.r.max(c.g).max(c.b) - c.r.min(c.g).min(c.b);
            let strongest = |c: Color| {
                [c.r, c.g, c.b]
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(index, _)| index)
            };
            assert!(spread(deep) >= spread(color) - 0.2, "{deep:?}");
            assert_eq!(strongest(deep), strongest(color), "same hue");
            let sum = |c: Color| c.r + c.g + c.b;
            assert!(sum(deep) < sum(color), "darker");
        }
    }

    #[test]
    fn the_overflow_band_is_inside_the_bar_and_centred() {
        let bar = Rect::new(10.0, 20.0, 100.0, 10.0);
        let band = overflow_band(bar);
        assert!(band.y > bar.y && band.bottom() < bar.bottom());
        assert!((band.y + band.height * 0.5 - 25.0).abs() < 1e-4);
        assert_eq!((band.x, band.width), (bar.x, bar.width));
    }

    #[test]
    fn dashes_have_gaps_and_stay_in_their_run() {
        let pieces: Vec<_> = dashes(10.0, 100.0, 9.0, 2.0).collect();
        assert_eq!(pieces.len(), 10);
        assert_eq!(pieces[0], (10.0, 19.0));
        assert_eq!(pieces[1], (21.0, 30.0));
        for pair in pieces.windows(2) {
            assert!(pair[0].1 < pair[1].0, "a gap between dashes");
        }
        assert_eq!(pieces.last().unwrap().1, 110.0, "the last cut to the run");
        assert_eq!(dashes(0.0, 0.0, 3.0, 4.0).count(), 0);
        // A bar's segments are whole: the last ends at the bar's end, as long as the rest.
        let bar: Vec<_> = segments(0.0, 123.0, 9.0, 2.0).collect();
        assert_eq!(bar.len(), 11);
        let first = bar[0].1 - bar[0].0;
        let last = bar[10].1 - bar[10].0;
        assert!((first - last).abs() < 1e-3 && (bar[10].1 - 123.0).abs() < 1e-3);
    }

    /// A plate near enough for its bars: 18-pixel names (`cg_nameplateScale` 0.5 at
    /// 1080 lines), a picture and the seal.
    fn spec(rows: Rows) -> HoloSpec {
        HoloSpec {
            centre_x: 960.0,
            bottom: 500.0,
            size: 18.0,
            name_width: 60.0,
            picture: true,
            mark: true,
            client: false,
            duel: false,
            rows,
            detail: 1.0,
            proximity: 1.0,
            least_bar: 2.0,
            viewport: [1920.0, 1080.0],
        }
    }

    const ALL: Rows = Rows {
        health: true,
        shield: true,
        force: true,
    };

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x - 1e-3
            && inner.y >= outer.y - 1e-3
            && inner.right() <= outer.right() + 1e-3
            && inner.bottom() <= outer.bottom() + 1e-3
    }

    /// Another client's icon follows the seal, smaller, level with it and inside the
    /// plate, which widens for it once the name is long.
    #[test]
    fn a_client_icon_follows_the_seal() {
        let holo = Holo::new(&HoloSpec {
            client: true,
            ..spec(ALL)
        });
        let (mark, client) = (holo.mark.unwrap(), holo.client.unwrap());
        assert!(client.x > mark.right() && client.width < mark.width);
        assert!(inside(client, holo.plate));
        let centre = |rect: Rect| rect.y + rect.height * 0.5;
        assert!((centre(client) - centre(mark)).abs() < 1e-3);
        assert!(Holo::new(&spec(ALL)).client.is_none());
        // Without a seal it takes the seal's place.
        let alone = Holo::new(&HoloSpec {
            mark: false,
            client: true,
            ..spec(ALL)
        });
        assert!((alone.client.unwrap().x - mark.x).abs() < 1e-3);
        let long = |client: bool| {
            Holo::new(&HoloSpec {
                name_width: 150.0,
                client,
                ..spec(ALL)
            })
        };
        let grown = long(true).plate.width - long(false).plate.width;
        assert!((grown - (MARK_GAP + CLIENT) * holo.k).abs() < 1e-3, "{grown}");
        assert!(inside(long(true).client.unwrap(), long(true).plate));
    }

    /// The hexagon overlaps the plate's left end and is centred on it; the name and its
    /// seal run in the plate's top row, the bars under them (shield on top, health the
    /// thickest), and the whole stands centred on its bottom.
    #[test]
    fn a_holo_frame_holds_its_parts_in_order() {
        let holo = Holo::new(&spec(ALL));
        let (plate, hex) = (holo.plate, holo.hex.unwrap());
        assert!((plate.bottom() - 500.0).abs() < 1e-3, "on the bottom");
        assert!(
            (hex.x + (plate.right() - hex.x) * 0.5 - 960.0).abs() < 1e-3,
            "centred"
        );
        assert!(
            hex.x < plate.x && hex.right() > plate.x,
            "over the plate's end"
        );
        let centre = |rect: Rect| rect.y + rect.height * 0.5;
        assert!((centre(hex) - centre(plate)).abs() < 1e-3);
        // At the design's size the frame is the design's 40 high, 150 wide.
        let k = holo.k;
        assert!((plate.height - 40.0 * k).abs() < 1e-3, "{plate:?}");
        assert!((plate.width - 150.0 * k).abs() < 1e-3);
        assert!((hex.width - 44.0 * k).abs() < 1e-3);
        let mark = holo.mark.unwrap();
        assert!(holo.name.x > hex.right() && holo.name.right() < mark.x);
        assert!(inside(mark, plate));
        assert!((centre(mark) - holo.middle).abs() < 1e-3);
        assert!(holo.badge.is_none());
        let Bars {
            shield,
            health,
            force,
        } = holo.bars;
        let (shield, health, force) = (shield.unwrap(), health.unwrap(), force.unwrap());
        assert!(holo.name.bottom() <= shield.y);
        assert!(shield.bottom() < health.y && health.bottom() < force.y);
        for bar in [shield, health, force] {
            assert!(inside(bar, plate), "{bar:?}");
            assert!(bar.x > hex.right(), "clear of the hexagon");
        }
        assert!(health.height > shield.height, "health is the thickest");
        // The plate's corners: its top-right one cut.
        let outline = holo.outline();
        assert_eq!(outline[0], [plate.x, plate.y]);
        assert!(outline[1][0] < plate.right() && outline[2][1] > plate.y);
    }

    /// No bars (names only, a player who does not share, a duel): the plate is the
    /// design's 30 high with a smaller hexagon and a dotted line under the name, and it
    /// grows to hold the bars only as they fade in.
    #[test]
    fn without_bars_the_plate_is_short_and_dotted() {
        let short = Holo::new(&spec(Rows::default()));
        let k = short.k;
        assert!((short.plate.height - 30.0 * k).abs() < 1e-3);
        assert!((short.hex.unwrap().height - 34.0 * k).abs() < 1e-3);
        assert_eq!(short.bars, Bars::default());
        assert!(short.dotted.y > short.name.bottom() && inside(short.dotted, short.plate));
        let fading = Holo::new(&HoloSpec {
            detail: 0.5,
            ..spec(ALL)
        });
        assert!(fading.plate.height > short.plate.height);
        assert!(fading.plate.height < Holo::new(&spec(ALL)).plate.height);
        assert!((fading.plate.bottom() - 500.0).abs() < 1e-3);
    }

    /// At 1080 lines with the default text size no bar is thinner than two pixels,
    /// even at the far edge of the bars' range, where the plate has shrunk.
    #[test]
    fn the_bars_are_never_under_two_pixels() {
        let shrunk = distance_scale(1000.0, 3000.0, 0.6);
        let holo = Holo::new(&HoloSpec {
            size: 36.0 * 0.5 * shrunk,
            ..spec(ALL)
        });
        for bar in [holo.bars.shield, holo.bars.health, holo.bars.force] {
            assert!(bar.unwrap().height >= 2.0 - 1e-4, "{bar:?}");
        }
    }

    /// The duel badge sits on the hexagon's bottom-right corner; with no picture (an
    /// NPC) the name starts further in; far away a small hexagon and the name are
    /// centred over a hairline.
    #[test]
    fn the_badge_and_the_far_plate_follow_the_hexagon() {
        let duel = Holo::new(&HoloSpec {
            duel: true,
            rows: Rows::default(),
            ..spec(ALL)
        });
        let (badge, hex) = (duel.badge.unwrap(), duel.hex.unwrap());
        assert!(badge.right() > hex.right() && badge.bottom() > hex.bottom());
        assert!(
            badge.x < hex.right() && badge.y < hex.bottom(),
            "on its corner"
        );
        let npc = Holo::new(&HoloSpec {
            picture: false,
            ..spec(ALL)
        });
        assert!(npc.hex.is_none());
        assert!(npc.name.x > npc.plate.x);
        let far = Holo::new(&HoloSpec {
            proximity: 0.0,
            ..spec(Rows::default())
        });
        let hex = far.hex.unwrap();
        assert!((hex.height - 15.0 * far.k).abs() < 1e-3, "small");
        assert!(far.name.x > hex.right());
        let shown = (hex.x + far.mark.unwrap().right()) * 0.5;
        assert!((shown - 960.0).abs() < 0.5, "{shown}");
        let hairline = far.hairline;
        assert!((hairline.x + hairline.width * 0.5 - 960.0).abs() < 0.5);
        assert!(hairline.y > hex.bottom() && hairline.y > far.name.bottom());
        assert!((hairline.bottom() - 500.0).abs() < 1e-3);
    }

    /// A long name widens the plate up to its widest, and a plate at the screen's edge
    /// stays on it, its power icons too.
    #[test]
    fn plates_stay_on_screen() {
        let long = Holo::new(&HoloSpec {
            name_width: 5_000.0,
            ..spec(ALL)
        });
        assert!(long.name.width < 300.0);
        assert!(long.plate.width > 150.0 * long.k);
        let left = Holo::new(&HoloSpec {
            centre_x: 5.0,
            ..spec(ALL)
        });
        assert_eq!(left.hex.unwrap().x, 0.0);
        let right = Holo::new(&HoloSpec {
            centre_x: 1915.0,
            ..spec(ALL)
        });
        assert!((right.plate.right() - 1920.0).abs() < 1e-3);
        // The power icons stand clear over the plate's right end.
        let (chip, step) = right.chips(3, [1920.0, 1080.0]);
        assert!(chip.bottom() < right.plate.y && step > chip.width);
        assert!(chip.x + 2.0 * step + chip.width <= right.plate.right() + 1e-3);
    }

    /// A round-cut picture spread over a hexagon shows none of its clear outside: each
    /// corner's texture coordinate is inside the disc.
    #[test]
    fn the_hexagon_stays_inside_a_round_picture() {
        let box_ = Rect::new(0.0, 0.0, 40.0, 36.0);
        let [low, high] = HEX_UV;
        for [x, y] in hexagon(box_) {
            let u = low[0] + (high[0] - low[0]) * x / box_.width;
            let v = low[1] + (high[1] - low[1]) * y / box_.height;
            let reach = (u - 0.5).hypot(v - 0.5);
            assert!(reach <= 0.5 && reach > 0.45, "{reach}");
        }
    }

    /// A frame's plate is its colour made dark: the design's blue team plate.
    #[test]
    fn a_plate_is_tinted_by_its_frame() {
        let blue = plate_fill(Color::new(77.0 / 255.0, 153.0 / 255.0, 1.0, 1.0), 0.55);
        assert!((blue.r * 255.0 - 8.0).abs() < 2.0 && (blue.b * 255.0 - 26.0).abs() < 2.0);
        assert_eq!(blue.a, 0.55);
    }
}
