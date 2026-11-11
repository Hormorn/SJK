//! The playstyle badges over a plate ([`crate::playstyle`], 11/11/2026): a row of round
//! dark cells, one per playstyle the player picked, each with its icon drawn in its
//! colour (a lit saber, lightning, a burst, a smile, a book). With the crosshair on
//! the player, each cell widens into a pill that also says its words. The row stands
//! over the plate (and over its power icons), centred on it; it fades with the plate
//! when far and stays away from duels.

use super::{CHIP_FILL, Entry, PLAYSTYLE_TEXT, State, name_opacity};
use crate::UiFont;
use crate::playstyle::{Picks, Playstyle};
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign, TextId, TextOverflow};

/// The cell's diameter, the gap between cells and the room under the row, as shares of
/// the name's size.
const CELL: f32 = 1.3;
const GAP: f32 = 0.28;
const LIFT: f32 = 0.3;
/// A focused pill's words: their size and the room either side of them.
const WORDS: f32 = 0.72;
const WORDS_PAD: f32 = 0.45;
/// How long the words stay after the crosshair leaves the player, in ms.
const WORDS_LINGER: i64 = 1_500;

/// The text id of `style`'s words.
pub(super) fn text_id(style: Playstyle) -> TextId {
    TextId(PLAYSTYLE_TEXT + style as u32)
}

/// The words of text id `id`, if it is a playstyle's.
pub(super) fn words(id: TextId) -> Option<&'static str> {
    let index = id.0.checked_sub(PLAYSTYLE_TEXT)?;
    Playstyle::ALL
        .get(index as usize)
        .map(|style| style.label())
}

/// The cells of a row of `picks` centred on `centre`, standing on `top`: each cell's
/// rectangle (a pill when `widths` gives its words' width).
pub(super) fn cells(
    picks: Picks,
    [centre, top]: [f32; 2],
    size: f32,
    widths: impl Fn(Playstyle) -> Option<f32>,
    viewport: [f32; 2],
) -> Vec<(Playstyle, Rect)> {
    let side = size * CELL;
    let gap = size * GAP;
    let width_of =
        |style: Playstyle| widths(style).map_or(side, |words| side + words + size * WORDS_PAD);
    let total =
        picks.iter().map(width_of).sum::<f32>() + gap * picks.count().saturating_sub(1) as f32;
    let mut x = (centre - total * 0.5).clamp(0.0, (viewport[0] - total).max(0.0));
    let y = top - size * LIFT - side;
    picks
        .iter()
        .map(|style| {
            let width = width_of(style);
            let rect = Rect::new(x, y, width, side);
            x += width + gap;
            (style, rect)
        })
        .collect()
}

impl State {
    /// The row of playstyle badges over `entry`'s plate, whose top is at `top` and
    /// middle at `centre`; `size` is its name's.
    pub(super) fn playstyle_row<'a>(
        &mut self,
        entry: &Entry,
        [centre, top]: [f32; 2],
        size: f32,
        label: &dyn Fn(TextId) -> &'a str,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        if !self.settings.playstyle
            || entry.npc_class != 0
            || entry.duelling
            || entry.proximity <= 0.01
        {
            return;
        }
        let Some(picks) = self.playstyles.get(usize::from(entry.number)).copied() else {
            return;
        };
        if !picks.any() {
            return;
        }
        // The crosshair on them lately: their badges say their words.
        let focused = self.focus.is_some_and(|(client, seen)| {
            client == entry.number && (0..=WORDS_LINGER).contains(&(self.last_update - seen))
        });
        let words_size = size * WORDS;
        let scale = words_size / font.height.max(1.0);
        let widths = |style: Playstyle| {
            focused.then(|| crate::text::visible_text_width(font, label(text_id(style)), scale))
        };
        let cells = cells(picks, [centre, top], size, widths, viewport);
        let opacity = name_opacity(entry) * entry.proximity;
        let _ = self.list.push(DrawCommand::PushOpacity(opacity));
        for (style, rect) in cells {
            let [r, g, b] = style.colour();
            let colour = Color::new(r, g, b, 1.0);
            let radius = rect.height * 0.5;
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect,
                radius,
                color: CHIP_FILL,
            });
            let _ = self.list.push(DrawCommand::Border {
                rect,
                radius,
                width: (size * 0.07).max(1.0),
                color: Color::new(r, g, b, 0.75),
            });
            let icon = Rect::new(rect.x, rect.y, rect.height, rect.height);
            self.playstyle_icon(style, icon, colour);
            if focused {
                let _ = self.list.push(DrawCommand::Text {
                    rect: Rect::new(
                        icon.right() - size * 0.05,
                        rect.y + radius - font.capital_middle(scale),
                        rect.width - icon.width,
                        words_size * 1.15,
                    ),
                    text: text_id(style),
                    size: words_size,
                    color: Color::new(1.0, 1.0, 1.0, 1.0),
                    align: TextAlign::Start,
                    overflow: TextOverflow::Clip,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
    }

    /// `style`'s icon in the round `cell`, drawn in strokes of `colour`.
    fn playstyle_icon(&mut self, style: Playstyle, cell: Rect, colour: Color) {
        let r = cell.width * 0.5;
        let [cx, cy] = [cell.x + r, cell.y + r];
        let at = |x: f32, y: f32| [cx + x * r, cy + y * r];
        let stroke = (r * 0.16).max(1.0);
        let line = |list: &mut sjk_ui::DrawList, from: [f32; 2], to: [f32; 2]| {
            let _ = list.push(DrawCommand::Line {
                from,
                to,
                width: stroke,
                color: colour,
            });
        };
        let list = &mut self.list;
        match style {
            // A lit saber, slanted: a thick hilt and a thin blade (not the duel
            // badge's crossed sabers).
            Playstyle::SaberDuels => {
                line(list, at(-0.3, 0.3), at(0.52, -0.52));
                let _ = list.push(DrawCommand::Line {
                    from: at(-0.58, 0.58),
                    to: at(-0.26, 0.26),
                    width: stroke * 2.0,
                    color: Color::new(0.85, 0.87, 0.9, colour.a),
                });
            }
            // A lightning bolt.
            Playstyle::ForceDuels => {
                line(list, at(0.18, -0.6), at(-0.22, 0.04));
                line(list, at(-0.22, 0.04), at(0.22, 0.0));
                line(list, at(0.22, 0.0), at(-0.16, 0.6));
            }
            // A burst: eight rays out of an empty middle.
            Playstyle::Ffa => {
                for step in 0..8 {
                    let angle = step as f32 * std::f32::consts::FRAC_PI_4;
                    let (sin, cos) = angle.sin_cos();
                    let reach = if step % 2 == 0 { 0.6 } else { 0.42 };
                    line(list, at(cos * 0.2, sin * 0.2), at(cos * reach, sin * reach));
                }
            }
            // A smile and two eyes.
            Playstyle::Goofing => {
                let _ = list.push(DrawCommand::Arc {
                    center: at(0.0, 0.02),
                    radius: r * 0.4,
                    width: stroke,
                    start: std::f32::consts::PI * 0.15,
                    sweep: std::f32::consts::PI * 0.7,
                    color: colour,
                    knockout: None,
                });
                for x in [-0.25, 0.25] {
                    let eye = r * 0.2;
                    let [ex, ey] = at(x, -0.22);
                    let _ = list.push(DrawCommand::RoundedRect {
                        rect: Rect::new(ex - eye * 0.5, ey - eye * 0.5, eye, eye),
                        radius: eye * 0.5,
                        color: colour,
                    });
                }
            }
            // An open book.
            Playstyle::Teaching => {
                let (top, bottom, dip) = (-0.32, 0.36, 0.12);
                line(list, at(-0.58, top), at(0.0, top + dip));
                line(list, at(0.0, top + dip), at(0.58, top));
                line(list, at(-0.58, bottom), at(0.0, bottom + dip));
                line(list, at(0.0, bottom + dip), at(0.58, bottom));
                line(list, at(-0.58, top), at(-0.58, bottom));
                line(list, at(0.58, top), at(0.58, bottom));
                line(list, at(0.0, top + dip), at(0.0, bottom + dip));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Picks {
        Playstyle::ALL
            .into_iter()
            .fold(Picks::default(), |picks, style| picks.with(style, true))
    }

    /// The row stands over the plate, centred on it, its cells apart and on screen.
    #[test]
    fn the_row_stands_over_the_plate_centred() {
        let cells = cells(all(), [960.0, 400.0], 18.0, |_| None, [1920.0, 1080.0]);
        assert_eq!(cells.len(), 5);
        let left = cells[0].1.x;
        let right = cells[4].1.right();
        assert!(((left + right) * 0.5 - 960.0).abs() < 1e-3);
        for pair in cells.windows(2) {
            assert!(pair[0].1.right() < pair[1].1.x);
        }
        assert!(cells.iter().all(|(_, rect)| rect.bottom() < 400.0));
        // At the screen's edge it stays on it.
        let edge = super::cells(all(), [2.0, 400.0], 18.0, |_| None, [1920.0, 1080.0]);
        assert_eq!(edge[0].1.x, 0.0);
    }

    /// Focused, a cell widens by its words.
    #[test]
    fn focused_cells_widen_for_their_words() {
        let one = Picks::default().with(Playstyle::Goofing, true);
        let plain = cells(one, [960.0, 400.0], 18.0, |_| None, [1920.0, 1080.0]);
        let wide = cells(one, [960.0, 400.0], 18.0, |_| Some(80.0), [1920.0, 1080.0]);
        assert!(wide[0].1.width > plain[0].1.width + 79.0);
        assert_eq!(plain[0].1.width, plain[0].1.height, "round alone");
    }

    #[test]
    fn every_playstyle_has_words() {
        for style in Playstyle::ALL {
            assert_eq!(words(text_id(style)), Some(style.label()));
        }
        assert_eq!(words(TextId(PLAYSTYLE_TEXT + 5)), None);
        assert_eq!(words(TextId(3)), None);
    }
}
