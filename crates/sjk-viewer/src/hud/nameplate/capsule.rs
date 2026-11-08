//! The capsule nameplate (design E, 28/10/2026; `cg_nameplateStyle 1` since 31/10/2026):
//! a pill holding the player's picture (round), their name with its mark after it and,
//! at its right end in a duel, the duel badge. A sharer's bars stand on a tray tucked
//! under it; a player who does not share has the capsule alone. Team games tint the
//! pill in the team's colour made deep, a sharer's edge is gold, and far away the pill
//! fades leaving the picture and the name.

use super::{
    BLANK_PICTURE, CHIP_FILL, CHIP_INSET, Entry, INITIAL_TEXT, Meter, NEUTRAL_ACCENT, NPC_ACCENT,
    Picture, Place, SHARED_GOLD, State, classic::BarLook, math, name_opacity,
};
use crate::UiFont;
use math::Rows;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign, TextId, TextOverflow, TextureId};

/// The capsule's measures are its design's (28/10/2026, design E), in that design's
/// pixels, whose name is this tall: a plate scales them by its name's size over it.
const DESIGN_NAME: f32 = 13.0;
/// The capsule's height, its edge's width (a duellist's is wider), and the picture's
/// inset from the outer edge (the edge and the padding round the picture).
const PILL_HEIGHT: f32 = 26.0;
const EDGE: f32 = 1.0;
const DUEL_EDGE: f32 = 1.5;
const PICTURE_INSET: f32 = 3.0;
/// The room before the name without a picture (an NPC's), and after the last item: a
/// duel badge's end is tighter.
const BARE_START: f32 = 11.0;
const END_PAD: f32 = 11.0;
const DUEL_END_PAD: f32 = 5.0;
/// The gap before the duel badge, and between the name and the mark after it.
const GAP: f32 = 6.0;
const MARK_GAP: f32 = 4.0;
/// The verified seal's side.
const MARK: f32 = 11.0;
/// The side of another client's icon after the mark (Sol, 08/11/2026: "in small").
const CLIENT: f32 = 9.0;
/// The right end: the padding before the duel badge, and the badge.
const END_LEAD: f32 = 2.0;
const BADGE: f32 = 18.0;
/// The widest the name runs before its ellipsis.
const NAME_MAX: f32 = 150.0;
/// The picture's diameter as a share of the name's size, near and far: far away the
/// capsule fades and a smaller picture stays beside the name.
const NEAR_PICTURE: f32 = 20.0 / 13.0;
const FAR_PICTURE: f32 = 13.0 / 11.0;
/// The gap after the picture, near and far, as a share of the name's size.
const NEAR_GAP: f32 = 6.0 / 13.0;
const FAR_GAP: f32 = 4.0 / 11.0;
/// The tray under the capsule: its width, how far it tucks under, the room round its
/// bars (top, sides, bottom), its bottom corners, and the room the capsule keeps
/// either side of it.
const TRAY_WIDTH: f32 = 104.0;
const TRAY_TUCK: f32 = 1.0;
const TRAY_TOP: f32 = 3.0;
const TRAY_SIDE: f32 = 6.0;
const TRAY_BOTTOM: f32 = 5.0;
const TRAY_CORNER: f32 = 6.0;
const TRAY_MARGIN: f32 = 10.0;
/// The tray's bars, top to bottom, and the gap between them.
const TRAY_SHIELD: f32 = 2.0;
const TRAY_HEALTH: f32 = 3.0;
const TRAY_FORCE: f32 = 2.0;
const TRAY_GAP: f32 = 1.5;
/// The power icons over the capsule: their square cells' side (1.5 times the round
/// chips' 15, Sol, 28/10/2026), their corners, the gap between them and the room under
/// the row.
const CHIP: f32 = 22.5;
const CHIP_CORNER: f32 = 3.0;
const CHIP_GAP: f32 = 2.0;
const CHIP_LIFT: f32 = 3.0;
/// The edge of a player who does not share.
pub(crate) const NEUTRAL_EDGE: Color = Color::new(0.78, 0.82, 0.9, 0.45);
/// The capsule's and the tray's fill out of team games (a team's is its own colour
/// made deep, [`team_fill`]).
pub(crate) const PILL_FILL: Color = Color::new(0.03, 0.04, 0.06, 0.55);
/// The track a bar fills on the tray.
pub(crate) const TRACK: Color = Color::new(1.0, 1.0, 1.0, 0.09);

/// What [`Capsule::new`] lays out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CapsuleSpec {
    /// The point the plate is centred on, and the bottom of the whole plate (the
    /// tray's while it shows, else the capsule's).
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
    /// The duel badge at the right end. The held weapon's icon stood there until
    /// 28/10/2026.
    pub(crate) duel: bool,
    /// The bars on the tray; none, no tray.
    pub(crate) rows: Rows,
    /// How much of the tray shows, 0 to 1 ([`math::detail`]): the capsule rises over it
    /// and widens to hold it as it fades in.
    pub(crate) detail: f32,
    /// How close the player is, 0 (far) to 1 ([`math::detail`]'s ramp, bars or not).
    pub(crate) proximity: f32,
    /// The thinnest a bar may be, in pixels.
    pub(crate) least_bar: f32,
    pub(crate) viewport: [f32; 2],
}

/// The tray under a capsule: its frame and the bars inside (shield on top, then
/// health, then Force), in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tray {
    pub(crate) frame: Rect,
    pub(crate) shield: Option<Rect>,
    pub(crate) health: Option<Rect>,
    pub(crate) force: Option<Rect>,
}

/// A nameplate as a capsule: a pill holding the picture, the name with its mark, and
/// the duel badge at its right end in a duel, with the power icons over it and, for a
/// player whose bars show, the tray under it. Physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Capsule {
    /// The design's pixel in physical ones.
    pub(crate) k: f32,
    pub(crate) pill: Rect,
    /// The pill's vertical middle: the name's capitals, the picture and the icons are
    /// centred on it.
    pub(crate) middle: f32,
    pub(crate) picture: Option<Rect>,
    /// Where the name runs: its left edge and the most it may take.
    pub(crate) name: Rect,
    pub(crate) mark: Option<Rect>,
    pub(crate) client: Option<Rect>,
    pub(crate) badge: Option<Rect>,
    pub(crate) tray: Option<Tray>,
}

impl Capsule {
    pub(crate) fn new(spec: &CapsuleSpec) -> Self {
        let k = spec.size / DESIGN_NAME;
        let near = spec.proximity.clamp(0.0, 1.0);
        let detail = spec.detail.clamp(0.0, 1.0);
        let lerp = |far: f32, close: f32| far + (close - far) * near;
        let diameter = spec.size * lerp(FAR_PICTURE, NEAR_PICTURE);
        let height = PILL_HEIGHT * k;
        let start = if spec.picture {
            PICTURE_INSET * k + diameter + spec.size * lerp(FAR_GAP, NEAR_GAP)
        } else {
            BARE_START * k
        };
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
        let (end_width, end_pad) = if spec.duel {
            (GAP + END_LEAD + BADGE, DUEL_END_PAD)
        } else {
            (0.0, END_PAD)
        };
        let natural = start + name_width + tail_width + (end_width + end_pad) * k;
        let tray = spec.rows.any();
        let wanted = if tray {
            natural.max((TRAY_WIDTH + TRAY_MARGIN * 2.0) * k)
        } else {
            natural
        };
        let width = (natural + (wanted - natural) * detail).min(spec.viewport[0]);
        let bar = |height: f32| (height * k).max(spec.least_bar);
        let bars = [
            (spec.rows.shield, bar(TRAY_SHIELD)),
            (spec.rows.health, bar(TRAY_HEALTH)),
            (spec.rows.force, bar(TRAY_FORCE)),
        ];
        let count = bars.iter().filter(|(wanted, _)| *wanted).count();
        let tray_height = if count == 0 {
            0.0
        } else {
            (TRAY_TOP + TRAY_BOTTOM + TRAY_GAP * (count - 1) as f32) * k
                + bars
                    .iter()
                    .filter(|(wanted, _)| *wanted)
                    .map(|(_, height)| height)
                    .sum::<f32>()
        };
        let pill_bottom = spec.bottom - (tray_height - TRAY_TUCK * k).max(0.0) * detail;
        // Far away only the picture, the name and its mark (and a duel badge) stay:
        // they, not the faded pill, are centred over the player.
        let shown_from = if spec.picture {
            PICTURE_INSET * k
        } else {
            BARE_START * k
        };
        let hidden_right = if spec.duel {
            end_pad * k
        } else {
            (end_width + end_pad) * k
        };
        let lean = (hidden_right - shown_from) * 0.5 * (1.0 - near);
        let x =
            (spec.centre_x - width * 0.5 + lean).clamp(0.0, (spec.viewport[0] - width).max(0.0));
        let pill = Rect::new(x, pill_bottom - height, width, height);
        let middle = pill.y + height * 0.5;
        let square = |left: f32, side: f32| Rect::new(left, middle - side * 0.5, side, side);
        let picture = spec
            .picture
            .then(|| square(pill.x + PICTURE_INSET * k, diameter));
        let name = Rect::new(pill.x + start, pill.y, name_width, height);
        let mark = spec
            .mark
            .then(|| square(name.right() + MARK_GAP * k, MARK * k));
        let client = spec
            .client
            .then(|| square(name.right() + mark_width + MARK_GAP * k, CLIENT * k));
        let right = pill.right() - end_pad * k;
        let badge = spec.duel.then(|| square(right - BADGE * k, BADGE * k));
        let tray = tray.then(|| {
            let tray_width = (TRAY_WIDTH * k).min(pill.width);
            let frame = Rect::new(
                pill.x + (pill.width - tray_width) * 0.5,
                pill.bottom() - TRAY_TUCK * k,
                tray_width,
                tray_height,
            );
            let inner_x = frame.x + TRAY_SIDE * k;
            let inner_width = (frame.width - TRAY_SIDE * 2.0 * k).max(0.0);
            let mut cursor = frame.y + TRAY_TOP * k;
            let mut next = |(wanted, height): (bool, f32)| {
                wanted.then(|| {
                    let rect = Rect::new(inner_x, cursor, inner_width, height);
                    cursor += height + TRAY_GAP * k;
                    rect
                })
            };
            Tray {
                frame,
                shield: next(bars[0]),
                health: next(bars[1]),
                force: next(bars[2]),
            }
        });
        Self {
            k,
            pill,
            middle,
            picture,
            name,
            mark,
            client,
            badge,
            tray,
        }
    }

    /// The power icons' cells, `count` of them, centred over the capsule: the first's
    /// rectangle and the step to the next.
    pub(crate) fn chips(&self, count: usize, viewport: [f32; 2]) -> (Rect, f32) {
        let side = CHIP * self.k;
        let step = side + CHIP_GAP * self.k;
        let width = count as f32 * step - CHIP_GAP * self.k;
        let centre = self.pill.x + self.pill.width * 0.5;
        let left = (centre - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
        (
            Rect::new(left, self.pill.y - CHIP_LIFT * self.k - side, side, side),
            step,
        )
    }

    /// The tray's bottom corners' radius, at most what fits.
    pub(crate) fn tray_corner(&self, tray: &Tray) -> f32 {
        (TRAY_CORNER * self.k)
            .min(tray.frame.width * 0.5)
            .min((tray.frame.bottom() - self.pill.bottom()).max(0.0))
    }
}

/// The colour a team tints a capsule with: its own colour made deep, so red reads as
/// dark as the design's blue (`rgba(22, 52, 110, 0.86)`) does.
pub(crate) fn team_fill(accent: Color) -> Color {
    let deep = |channel: f32| 0.04 + 0.43 * channel * channel;
    Color::new(deep(accent.r), deep(accent.g), deep(accent.b), 0.62)
}

impl State {
    /// One capsule plate.
    pub(super) fn capsule<'a>(
        &mut self,
        entry: &Entry,
        place: Place,
        label: &dyn Fn(TextId) -> &'a str,
        icons: &[Option<TextureId>],
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let Place {
            index,
            id,
            x,
            bottom,
            size,
            unit,
            ..
        } = place;
        let scale = size / font.height.max(1.0);
        // The duel badge stands at the capsule's right end.
        let duel_badge = entry.duelling && self.settings.duel_badge;
        // The tray holds a sharer's bars: a player who does not share has the capsule
        // alone, and a duel shows none.
        let rows = if entry.withheld || duel_badge {
            Rows::default()
        } else {
            entry.rows()
        };
        let capsule = Capsule::new(&CapsuleSpec {
            centre_x: x,
            bottom,
            size,
            name_width: crate::text::visible_text_width(font, label(id), scale),
            picture: entry.picture != Picture::Nothing,
            mark: entry.mark.is_some(),
            client: entry.client.is_some(),
            duel: duel_badge,
            rows,
            detail: entry.detail,
            proximity: entry.proximity,
            least_bar: 2.0 * unit,
            viewport,
        });
        let opacity = name_opacity(entry);
        // The pill (and the tray under it) fades out with distance: far away the
        // picture and the name stay.
        if entry.proximity > 0.01 {
            let _ = self.list.push(DrawCommand::PushOpacity(entry.proximity));
            self.pill(entry, &capsule, duel_badge);
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
        self.round_picture(entry, capsule.picture, index, font, opacity);
        self.name_line(
            entry,
            id,
            [capsule.name.x, capsule.middle],
            capsule.name.width,
            size,
            font,
        );
        self.badges(entry, [capsule.mark, capsule.client], capsule.badge, opacity);
        if entry.power_count > 0 && entry.proximity > 0.01 {
            let shown = entry.powers[..usize::from(entry.power_count)]
                .iter()
                .filter(|power| icons.get(usize::from(**power)).copied().flatten().is_some())
                .count();
            if shown > 0 {
                let (chip, step) = capsule.chips(shown, viewport);
                let k = capsule.k;
                let inset = (CHIP_INSET * k).max(1.0);
                self.chip_row(entry, icons, (chip, step), CHIP_CORNER * k, inset);
            }
        }
    }

    /// The capsule's pill, its fill (a team's colour made deep in team games) and its
    /// edge (gold for a sharer, the pair's colour in a duel), with the tray of bars
    /// tucked under it while they fade in.
    fn pill(&mut self, entry: &Entry, capsule: &Capsule, duel_badge: bool) {
        let k = capsule.k;
        let fill = if entry.npc_class == 0 && entry.accent != NEUTRAL_ACCENT {
            team_fill(entry.accent)
        } else {
            PILL_FILL
        };
        let edge = if duel_badge {
            let [r, g, b] = crate::ui_renderer::duel_badge::COLOURS
                [usize::from(entry.duel_colour) % crate::ui_renderer::duel_badge::COLOURS.len()];
            Color::new(r, g, b, 1.0)
        } else if entry.npc_class != 0 {
            NPC_ACCENT
        } else if entry.shared {
            SHARED_GOLD
        } else {
            NEUTRAL_EDGE
        };
        let width = (if duel_badge { DUEL_EDGE } else { EDGE } * k).max(1.0);
        if let Some(tray) = capsule.tray
            && entry.detail > 0.01
        {
            let _ = self.list.push(DrawCommand::PushOpacity(entry.detail));
            self.tray(entry, &tray, capsule, fill, (edge, width));
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
        let pill = capsule.pill;
        let radius = pill.height * 0.5;
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect: pill,
            radius,
            color: fill,
        });
        let _ = self.list.push(DrawCommand::Border {
            rect: pill,
            radius,
            width,
            color: edge,
        });
    }

    /// The tray under the pill: the pill's fill and edge carried down round the bars,
    /// with a square top, which the pill's own edge closes, and round bottom corners.
    /// The fill is one stroke cut to the tray's rounded clip and the edge strokes and
    /// arcs, so nothing is drawn twice where the shapes meet (each is translucent).
    fn tray(
        &mut self,
        entry: &Entry,
        tray: &Tray,
        capsule: &Capsule,
        fill: Color,
        (edge, width): (Color, f32),
    ) {
        let frame = tray.frame;
        let top = capsule.pill.bottom();
        let corner = capsule.tray_corner(tray);
        if frame.bottom() - top > 0.5 {
            let _ = self.list.push(DrawCommand::PushRoundedClip {
                rect: Rect::new(
                    frame.x,
                    top - corner,
                    frame.width,
                    frame.bottom() - top + corner,
                ),
                radius: corner,
            });
            let middle = (top + frame.bottom()) * 0.5;
            let _ = self.list.push(DrawCommand::Line {
                from: [frame.x, middle],
                to: [frame.right(), middle],
                width: frame.bottom() - top,
                color: fill,
            });
            let _ = self.list.push(DrawCommand::PopClip);
        }
        let half = width * 0.5;
        let low = frame.bottom() - corner;
        for side in [frame.x + half, frame.right() - half] {
            let _ = self.list.push(DrawCommand::Line {
                from: [side, top],
                to: [side, low],
                width,
                color: edge,
            });
        }
        let _ = self.list.push(DrawCommand::Line {
            from: [frame.x + corner, frame.bottom() - half],
            to: [frame.right() - corner, frame.bottom() - half],
            width,
            color: edge,
        });
        for (centre, start) in [
            ([frame.x + corner, low], std::f32::consts::FRAC_PI_2),
            ([frame.right() - corner, low], 0.0),
        ] {
            let _ = self.list.push(DrawCommand::Arc {
                center: centre,
                radius: (corner - half).max(0.0),
                width,
                start,
                sweep: std::f32::consts::FRAC_PI_2,
                color: edge,
                knockout: None,
            });
        }
        let colors = self.colors;
        for (bar, range, color, meter) in [
            (tray.shield, entry.shield, colors.shield, Meter::Shield),
            (tray.health, entry.health, colors.health, Meter::Health),
            (tray.force, entry.force, colors.force, Meter::Force),
        ] {
            if let (Some(rect), Some(range)) = (bar, range) {
                self.rounded_meter(rect, range, color, meter, capsule.k, BarLook::Tray);
            }
        }
    }

    /// The round picture in `rect`, at `opacity` (the name's): the player's picture or
    /// head icon, already round, or their initial on their colour.
    fn round_picture(
        &mut self,
        entry: &Entry,
        rect: Option<Rect>,
        index: usize,
        font: &UiFont,
        opacity: f32,
    ) {
        let Some(rect) = rect else {
            return;
        };
        let radius = rect.width * 0.5;
        match entry.picture {
            Picture::Nothing => {}
            Picture::Texture(texture) => {
                let _ = self.list.push(DrawCommand::TexturedQuad {
                    rect,
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, opacity),
                });
            }
            Picture::StandIn(colour) => {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color: Color::new(colour.r, colour.g, colour.b, opacity),
                });
                let size = rect.height * 0.55;
                let scale = size / font.height.max(1.0);
                let _ = self.list.push(DrawCommand::Text {
                    rect: Rect::new(
                        rect.x,
                        rect.y + radius - font.capital_middle(scale),
                        rect.width,
                        size * 1.15,
                    ),
                    text: TextId(INITIAL_TEXT + index as u32),
                    size,
                    color: Color::new(1.0, 1.0, 1.0, opacity),
                    align: TextAlign::Center,
                    overflow: TextOverflow::Clip,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
            Picture::Blank => {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color: Color::new(
                        BLANK_PICTURE.r,
                        BLANK_PICTURE.g,
                        BLANK_PICTURE.b,
                        BLANK_PICTURE.a * opacity,
                    ),
                });
            }
        }
    }

    /// A row of power icons from `chip`, `step` apart, each on a square dark cell with
    /// `corner` round corners and its icon `inset` in: the capsule's and the classic
    /// plate's.
    pub(super) fn chip_row(
        &mut self,
        entry: &Entry,
        icons: &[Option<TextureId>],
        (mut chip, step): (Rect, f32),
        corner: f32,
        inset: f32,
    ) {
        let _ = self.list.push(DrawCommand::PushOpacity(entry.proximity));
        for power in &entry.powers[..usize::from(entry.power_count)] {
            let Some(texture) = icons.get(usize::from(*power)).copied().flatten() else {
                continue;
            };
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect: chip,
                radius: corner,
                color: CHIP_FILL,
            });
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect: math::inset(chip, inset),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
            chip.x += step;
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A capsule near enough for its tray: 18-pixel names (`cg_nameplateScale` 0.5 at
    /// 1080 lines), a picture and the seal.
    fn spec(rows: Rows) -> CapsuleSpec {
        CapsuleSpec {
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

    /// Another client's icon follows the seal in the pill, smaller and level with it,
    /// and the pill grows by its room.
    #[test]
    fn a_client_icon_follows_the_seal_in_the_pill() {
        let with = Capsule::new(&CapsuleSpec {
            client: true,
            ..spec(Rows::default())
        });
        let without = Capsule::new(&spec(Rows::default()));
        let (mark, client) = (with.mark.unwrap(), with.client.unwrap());
        assert!(client.x > mark.right() && client.width < mark.width);
        assert!(inside(client, with.pill));
        assert!((client.y + client.height * 0.5 - with.middle).abs() < 1e-3);
        let grown = with.pill.width - without.pill.width;
        assert!((grown - (MARK_GAP + CLIENT) * with.k).abs() < 1e-3, "{grown}");
        assert!(without.client.is_none());
    }

    /// Left to right in the pill: the picture, the name and the seal, all centred on its
    /// middle, and no weapon after them; the tray under it holds the bars, shield
    /// on top, and the whole plate stands on its bottom.
    #[test]
    fn a_capsule_holds_its_parts_in_order_with_the_tray_under_it() {
        let capsule = Capsule::new(&spec(ALL));
        let pill = capsule.pill;
        assert!((pill.x + pill.width * 0.5 - 960.0).abs() < 1e-3, "centred");
        assert!((capsule.middle - (pill.y + pill.height * 0.5)).abs() < 1e-4);
        let picture = capsule.picture.unwrap();
        let mark = capsule.mark.unwrap();
        for part in [picture, mark] {
            assert!(inside(part, pill), "{part:?} in {pill:?}");
            assert!((part.y + part.height * 0.5 - capsule.middle).abs() < 1e-3);
        }
        assert!(picture.right() < capsule.name.x);
        assert!(capsule.name.right() < mark.x);
        assert!(capsule.badge.is_none());
        let tray = capsule.tray.unwrap();
        assert!((tray.frame.bottom() - 500.0).abs() < 1e-3, "on the bottom");
        assert!(tray.frame.y < pill.bottom(), "tucked under the pill");
        assert!(tray.frame.width <= pill.width);
        let (shield, health, force) = (
            tray.shield.unwrap(),
            tray.health.unwrap(),
            tray.force.unwrap(),
        );
        assert!(shield.bottom() < health.y && health.bottom() < force.y);
        for bar in [shield, health, force] {
            assert!(inside(bar, tray.frame), "{bar:?}");
            assert!(bar.y >= pill.bottom(), "under the pill");
        }
        assert!(health.height > shield.height, "health is the thickest");
    }

    /// No bars (sharing off, names only, a duel): no tray, and the pill stands on the
    /// bottom itself; the pill widens to hold a tray only as it fades in.
    #[test]
    fn without_bars_the_capsule_stands_alone() {
        let bare = Capsule::new(&spec(Rows::default()));
        assert!(bare.tray.is_none());
        // No weapon after the seal: the pill ends a padding after it.
        let mark = bare.mark.unwrap();
        let after = bare.pill.right() - mark.right();
        assert!((after - END_PAD * bare.k).abs() < 1e-3, "{after}");
        assert!((bare.pill.bottom() - 500.0).abs() < 1e-3);
        let short = CapsuleSpec {
            name_width: 10.0,
            ..spec(ALL)
        };
        let hidden = Capsule::new(&CapsuleSpec {
            detail: 0.0,
            ..short
        });
        let shown = Capsule::new(&short);
        assert!((hidden.pill.bottom() - 500.0).abs() < 1e-3, "no room kept");
        assert!(shown.pill.width > hidden.pill.width, "widened for the tray");
        let tray = shown.tray.unwrap().frame;
        assert!(tray.x > shown.pill.x && tray.right() < shown.pill.right());
    }

    /// At 1080 lines with the default text size no bar is thinner than two pixels,
    /// even at the far edge of the bars' range, where the plate has shrunk.
    #[test]
    fn the_bars_are_never_under_two_pixels() {
        let shrunk = math::distance_scale(1000.0, 3000.0, 0.6);
        let capsule = Capsule::new(&CapsuleSpec {
            size: 36.0 * 0.5 * shrunk,
            ..spec(ALL)
        });
        let tray = capsule.tray.unwrap();
        for bar in [tray.shield, tray.health, tray.force] {
            assert!(bar.unwrap().height >= 2.0 - 1e-4, "{bar:?}");
        }
    }

    /// The duel badge stands at the right end; with no picture (an NPC) the name
    /// starts further in; far away the picture is smaller.
    #[test]
    fn the_right_end_and_the_picture_follow_the_plate() {
        let duel = Capsule::new(&CapsuleSpec {
            duel: true,
            rows: Rows::default(),
            ..spec(ALL)
        });
        let badge = duel.badge.unwrap();
        assert!(inside(badge, duel.pill) && badge.x > duel.name.right());
        let npc = Capsule::new(&CapsuleSpec {
            picture: false,
            ..spec(ALL)
        });
        assert!(npc.picture.is_none());
        assert!(npc.name.x > npc.pill.x);
        let near = Capsule::new(&spec(Rows::default()));
        let far = Capsule::new(&CapsuleSpec {
            proximity: 0.0,
            ..spec(Rows::default())
        });
        assert!(far.picture.unwrap().width < near.picture.unwrap().width);
        assert!(
            far.picture.unwrap().width > 18.0,
            "still taller than the name"
        );
        // Far, what stays (the picture to the mark) is centred, not the faded pill.
        let shown = (far.picture.unwrap().x + far.mark.unwrap().right()) * 0.5;
        assert!((shown - 960.0).abs() < 0.5, "{shown}");
    }

    /// A long name stops at its widest, and a plate at the screen's edge stays on it.
    #[test]
    fn capsules_stay_on_screen() {
        let long = Capsule::new(&CapsuleSpec {
            name_width: 5_000.0,
            ..spec(ALL)
        });
        assert!(long.name.width < 300.0);
        let left = Capsule::new(&CapsuleSpec {
            centre_x: 5.0,
            ..spec(ALL)
        });
        assert_eq!(left.pill.x, 0.0);
        let right = Capsule::new(&CapsuleSpec {
            centre_x: 1915.0,
            ..spec(ALL)
        });
        assert!((right.pill.right() - 1920.0).abs() < 1e-3);
        // The power icons centre over it and stand clear of it.
        let (chip, step) = right.chips(3, [1920.0, 1080.0]);
        assert!(chip.bottom() < right.pill.y && step > chip.width);
        assert!(chip.x + 2.0 * step + chip.width <= 1920.0 + 1e-3);
    }

    /// A team's tint is its colour made deep: the blue one is the design's, and red is as
    /// dark.
    #[test]
    fn team_tints_are_deep_and_alike() {
        let blue = team_fill(Color::new(0.3, 0.6, 1.0, 0.95));
        assert!((blue.r - 22.0 / 255.0).abs() < 0.02);
        assert!((blue.g - 52.0 / 255.0).abs() < 0.02);
        assert!((blue.b - 110.0 / 255.0).abs() < 0.05);
        let red = team_fill(Color::new(1.0, 0.3, 0.25, 0.95));
        assert!(red.r > red.g && red.r > red.b, "still red");
        let light = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
        assert!((light(red) - light(blue)).abs() < 0.06);
    }
}
