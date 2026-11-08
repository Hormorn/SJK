//! The classic nameplate (the plates before 28/10/2026; `cg_nameplateStyle 2` since
//! 31/10/2026): the name centred over the player, the duel badge before it and the mark
//! after it, and under it, as they come close, a dark plate of rounded bars (shield,
//! health, Force) framed in gold for a sharer, in the team's colour otherwise. A player
//! who does not share has grey bars, so they read as hidden rather than empty. No
//! picture, and no weapon icon (gone since 28/10/2026 in every style).

use super::{
    EMPTY_SHARE, Entry, Meter, NEUTRAL_ACCENT, Place, SHARED_GOLD, State, UNSURE_WIDTH, math,
    name_opacity, name_tint,
};
use crate::UiFont;
use crate::hud::estimate::Range;
use math::Rows;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign, TextId, TextOverflow, TextureId};

/// The plate's fill and its corners, in the HUD's pixels.
const PLATE_FILL: Color = Color::new(0.03, 0.04, 0.06, 0.62);
const PLATE_CORNER: f32 = 4.0;
/// A grey bar of a player who does not share.
pub(super) const WITHHELD_BAR: Color = Color::new(0.42, 0.44, 0.48, 0.55);
/// The dark backdrop behind a bar, and the faint edge of a grey or empty one.
const BAR_BACKDROP: Color = Color::new(0.0, 0.0, 0.0, 0.6);
const FAINT_EDGE: Color = Color::new(1.0, 1.0, 1.0, 0.15);

/// How a rounded bar is drawn: on the classic plate (a dark backdrop and an outline) or
/// on the capsule's tray (a faint track, no outline).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BarLook {
    Classic,
    Tray,
}

/// The plate under a name: its frame and the bars inside (shield on top, then
/// health, then Force), in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stack {
    pub(crate) frame: Rect,
    pub(crate) health: Option<Rect>,
    pub(crate) shield: Option<Rect>,
    pub(crate) force: Option<Rect>,
}

impl Stack {
    /// Bar heights and spacing in units of `u` pixels (the scaled pixel unit).
    const HEALTH: f32 = 7.0;
    const SHIELD: f32 = 5.0;
    const FORCE: f32 = 4.0;
    const GAP: f32 = 2.5;
    const PAD: f32 = 4.0;
    /// Plate width in units of `u`.
    pub(crate) const WIDTH: f32 = 112.0;

    /// Total height of a stack holding `rows`, in pixels.
    pub(crate) fn height(rows: Rows, u: f32) -> f32 {
        let bars = [
            (rows.shield, Self::SHIELD),
            (rows.health, Self::HEALTH),
            (rows.force, Self::FORCE),
        ];
        let heights: f32 = bars.iter().filter(|b| b.0).map(|b| b.1).sum();
        let count = bars.iter().filter(|b| b.0).count();
        if count == 0 {
            return 0.0;
        }
        (Self::PAD * 2.0 + heights + Self::GAP * (count - 1) as f32) * u
    }

    /// Lay a stack out centred on `centre_x` with its bottom edge on `bottom_y`, kept
    /// on screen.
    pub(crate) fn new(
        centre_x: f32,
        bottom_y: f32,
        rows: Rows,
        u: f32,
        viewport: [f32; 2],
    ) -> Self {
        let width = Self::WIDTH * u;
        let height = Self::height(rows, u);
        let x = (centre_x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
        let y = bottom_y - height;
        let inner_x = x + Self::PAD * u;
        let inner_width = width - Self::PAD * 2.0 * u;
        let mut cursor = y + Self::PAD * u;
        let mut bar = |wanted: bool, h: f32| {
            wanted.then(|| {
                let rect = Rect::new(inner_x, cursor, inner_width, h * u);
                cursor += (h + Self::GAP) * u;
                rect
            })
        };
        let shield = bar(rows.shield, Self::SHIELD);
        let health = bar(rows.health, Self::HEALTH);
        let force = bar(rows.force, Self::FORCE);
        Self {
            frame: Rect::new(x, y, width, height),
            health,
            shield,
            force,
        }
    }
}

/// The pieces of an empty shield's broken bar, as `(start, end)` shares of its width:
/// seven dashes with gaps between, a crack's look.
fn broken_dashes() -> impl Iterator<Item = (f32, f32)> {
    const DASHES: usize = 7;
    const SOLID: f32 = 0.7;
    (0..DASHES).map(|index| {
        let start = index as f32 / DASHES as f32;
        (start, start + SOLID / DASHES as f32)
    })
}

/// `color` mixed `share` of the way to white, at opacity `alpha`: a bar's outline.
fn lighter(color: Color, share: f32, alpha: f32) -> Color {
    let mix = |channel: f32| channel + (1.0 - channel) * share.clamp(0.0, 1.0);
    Color::new(mix(color.r), mix(color.g), mix(color.b), alpha)
}

impl State {
    /// One classic plate.
    pub(super) fn classic<'a>(
        &mut self,
        entry: &Entry,
        place: Place,
        label: &dyn Fn(TextId) -> &'a str,
        icons: &[Option<TextureId>],
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let Place {
            id,
            x,
            bottom,
            size,
            u,
            unit,
            ..
        } = place;
        let rows = entry.rows();
        // The name rises as the plate fades in beneath it.
        let line = size * 1.15;
        let top = bottom - Stack::height(rows, u) * entry.detail - line;
        if !label(id).is_empty() {
            let width = (320.0 * unit).min(viewport[0]);
            let left = (x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
            let opacity = name_opacity(entry);
            let _ = self.list.push(DrawCommand::Text {
                rect: Rect::new(left, top, width, line),
                text: id,
                size,
                color: name_tint(entry),
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight: FontWeight::Regular,
                letter_spacing: 0.0,
            });
            let scale = size / font.height.max(1.0);
            let duel_badge = entry.duelling && self.settings.duel_badge;
            // Measured only for a badge beside the name.
            let text = if duel_badge || entry.mark.is_some() || entry.client.is_some() {
                crate::text::visible_text_width(font, label(id), scale).min(width)
            } else {
                0.0
            };
            // Level with the name's capitals: the text hangs from the top of its line by
            // the font's own baseline, not from the line's middle.
            let capitals = top + font.capital_middle(scale);
            // The duel badge before the name, mirroring the mark after it.
            let badge = duel_badge.then(|| {
                let side = size * 1.05;
                Rect::new(
                    (left + (width - text) * 0.5 - size * 0.2 - side).max(0.0),
                    capitals - side * 0.5,
                    side,
                    side,
                )
            });
            let after = left + (width + text) * 0.5 + size * 0.15;
            let mark = entry.mark.map(|_| {
                let side = size * 0.95;
                Rect::new(
                    after.min(viewport[0] - side),
                    capitals - side * 0.5,
                    side,
                    side,
                )
            });
            // Another client's icon after the mark, smaller.
            let client = entry.client.map(|_| {
                let side = size * 0.78;
                let x = mark.map_or(after, |mark| mark.right() + size * 0.15);
                Rect::new(x.min(viewport[0] - side), capitals - side * 0.5, side, side)
            });
            self.badges(entry, [mark, client], badge, opacity);
        }
        if entry.power_count > 0 && entry.proximity > 0.01 {
            // Centred over the name, as tall as it.
            let side = size;
            let gap = 2.0 * u;
            let shown = entry.powers[..usize::from(entry.power_count)]
                .iter()
                .filter(|power| icons.get(usize::from(**power)).copied().flatten().is_some())
                .count();
            if shown > 0 {
                let width = shown as f32 * side + (shown - 1) as f32 * gap;
                let left = (x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
                let chip = Rect::new(left, top - gap - side, side, side);
                self.chip_row(entry, icons, (chip, side + gap), 3.0 * u, u.max(1.0));
            }
        }
        if rows.any() && entry.detail > 0.01 {
            let stack = Stack::new(x, bottom, rows, u, viewport);
            let _ = self.list.push(DrawCommand::PushOpacity(entry.detail));
            self.stack(entry, &stack, u);
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
    }

    /// The framed plate and its bars: gold round a sharer's (a team's colour as a thin
    /// line inside it), the team's colour round the others'.
    fn stack(&mut self, entry: &Entry, stack: &Stack, u: f32) {
        let radius = PLATE_CORNER * u;
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect: stack.frame,
            radius,
            color: PLATE_FILL,
        });
        if entry.shared {
            let _ = self.list.push(DrawCommand::Border {
                rect: stack.frame,
                radius,
                width: (1.6 * u).max(1.0),
                color: SHARED_GOLD,
            });
            if entry.accent != NEUTRAL_ACCENT {
                let inset = (1.6 * u).max(1.0);
                let _ = self.list.push(DrawCommand::Border {
                    rect: math::inset(stack.frame, inset),
                    radius: (radius - inset).max(0.0),
                    width: (0.8 * u).max(1.0),
                    color: entry.accent,
                });
            }
        } else {
            let _ = self.list.push(DrawCommand::Border {
                rect: stack.frame,
                radius,
                width: (1.2 * u).max(1.0),
                color: entry.accent,
            });
        }
        let colors = self.colors;
        for (bar, range, color, meter) in [
            (stack.shield, entry.shield, colors.shield, Meter::Shield),
            (stack.health, entry.health, colors.health, Meter::Health),
            (stack.force, entry.force, colors.force, Meter::Force),
        ] {
            match (bar, range) {
                (Some(rect), Some(_)) if entry.withheld => self.withheld_meter(rect, u),
                (Some(rect), Some(range)) => {
                    self.rounded_meter(rect, range, color, meter, u, BarLook::Classic);
                }
                _ => {}
            }
        }
    }

    /// The bar of a player who does not share: a flat grey bar the whole way, so it
    /// reads as hidden rather than empty.
    fn withheld_meter(&mut self, rect: Rect, u: f32) {
        let radius = rect.height * 0.5;
        for color in [BAR_BACKDROP, WITHHELD_BAR] {
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect,
                radius,
                color,
            });
        }
        let _ = self.list.push(DrawCommand::Border {
            rect,
            radius,
            width: u.max(1.0),
            color: FAINT_EDGE,
        });
    }

    /// One rounded bar, the classic plate's or the capsule's tray's: its track, the fill
    /// to the guess with the haze over the uncertain stretch, and over a full bar
    /// (overheal, overshield) a second layer in a deeper shade from the left. An empty
    /// shield is a broken grey bar; health or shield too unsure to show dims under a
    /// yellow "?". `u` is the plate's pixel.
    pub(super) fn rounded_meter(
        &mut self,
        rect: Rect,
        range: Range,
        color: Color,
        meter: Meter,
        u: f32,
        look: BarLook,
    ) {
        let radius = rect.height * 0.5;
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect,
            radius,
            color: match look {
                BarLook::Classic => BAR_BACKDROP,
                BarLook::Tray => super::capsule::TRACK,
            },
        });
        if meter == Meter::Shield && range.high < EMPTY_SHARE {
            // On the classic plate the dashes are thinner than the bar and edged.
            let (inset, height) = match look {
                BarLook::Classic => (0.2, 0.6),
                BarLook::Tray => (0.0, 1.0),
            };
            for (start, end) in broken_dashes() {
                let _ = self.list.push(DrawCommand::SolidRect {
                    rect: Rect::new(
                        rect.x + rect.width * start,
                        rect.y + rect.height * inset,
                        rect.width * (end - start),
                        rect.height * height,
                    ),
                    color: math::EMPTY_SHIELD,
                });
            }
            if look == BarLook::Classic {
                let _ = self.list.push(DrawCommand::Border {
                    rect,
                    radius,
                    width: u.max(1.0),
                    color: FAINT_EDGE,
                });
            }
            return;
        }
        let unsure = meter != Meter::Force && range.width() >= UNSURE_WIDTH;
        let _ = self
            .list
            .push(DrawCommand::PushOpacity(if unsure { 0.35 } else { 1.0 }));
        let layers = [
            (range.clamp(0.0, 1.0), color),
            (
                range.map(|share| (share - 1.0).clamp(0.0, 1.0)),
                math::saturated(color),
            ),
        ];
        for (index, (layer, shade)) in layers.into_iter().enumerate() {
            if index == 1 && range.high <= 1.0 {
                break;
            }
            // The second layer is an inner band, so the full bar shows round it.
            let band = if index == 0 {
                rect
            } else {
                math::overflow_band(rect)
            };
            if layer.best > 0.01 {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect: Rect::new(
                        band.x,
                        band.y,
                        (band.width * layer.best).max(band.height),
                        band.height,
                    ),
                    radius: band.height * 0.5,
                    color: shade,
                });
            }
            self.haze(band, layer, (band.x, band.right()));
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
        if look == BarLook::Classic {
            let _ = self.list.push(DrawCommand::Border {
                rect,
                radius,
                width: u.max(1.0),
                color: match meter {
                    Meter::Force => lighter(color, 0.4, 0.75),
                    _ => Color::new(1.0, 1.0, 1.0, 0.3),
                },
            });
        }
        if unsure {
            self.unknown_mark(rect, u);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Rows = Rows {
        health: true,
        shield: true,
        force: true,
    };

    /// The bars stack shield, health, Force inside the plate, which stands on its
    /// bottom, centred, and stays on screen.
    #[test]
    fn a_stack_holds_its_bars_in_order() {
        let stack = Stack::new(960.0, 500.0, ALL, 1.0, [1920.0, 1080.0]);
        assert!((stack.frame.bottom() - 500.0).abs() < 1e-3);
        assert!((stack.frame.x + stack.frame.width * 0.5 - 960.0).abs() < 1e-3);
        let (shield, health, force) = (
            stack.shield.unwrap(),
            stack.health.unwrap(),
            stack.force.unwrap(),
        );
        assert!(shield.bottom() < health.y && health.bottom() < force.y);
        assert!(force.bottom() < stack.frame.bottom());
        assert!(health.height > shield.height && shield.height > force.height);
        assert_eq!(Stack::height(Rows::default(), 1.0), 0.0);
        let edge = Stack::new(5.0, 500.0, ALL, 1.0, [1920.0, 1080.0]);
        assert_eq!(edge.frame.x, 0.0);
    }

    /// An empty shield's broken bar has gaps and stays inside the bar.
    #[test]
    fn a_broken_bar_has_gaps_and_stays_inside() {
        let pieces: Vec<_> = broken_dashes().collect();
        assert!(pieces.len() > 3);
        for window in pieces.windows(2) {
            assert!(window[0].1 < window[1].0, "a gap between dashes");
        }
        assert!(pieces[0].0 >= 0.0 && pieces.last().unwrap().1 <= 1.0);
    }
}
