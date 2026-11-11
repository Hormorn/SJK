//! How the Collection draws a shader's marks (`crate::shader_marks`, 29/10/2026): the New
//! sticker, the star and padlock on a tile or a row, and the two toggles on the chosen
//! shader's card, in the SJK UI kit's look. Shared by the Shaders tab and the Forge.

use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::symbols;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The dark ink on a gold fill.
const INK: Color = Color::new(0.078, 0.063, 0.02, 1.0);
/// A toggle's side on a card.
pub(super) const TOGGLE: f32 = 44.0;
/// The room between the two toggles.
pub(super) const TOGGLE_GAP: f32 = 8.0;

/// The New sticker, `height` tall, its left at `[x, y]` (frame pixels): a gold pill with
/// "New" in dark ink. Returns its width.
pub(super) fn new_sticker(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    at: [f32; 2],
    height: f32,
) -> f32 {
    new_sticker_lit(canvas, frame, at, height, true)
}

/// [`new_sticker`], or quiet (`lit` false: a dim pill, muted words) when nothing is new.
pub(super) fn new_sticker_lit(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    height: f32,
    lit: bool,
) -> f32 {
    let s = frame.s;
    let size = height * 0.68;
    let width = crate::text::display_width("New", size) * crate::text::style::current().scale
        + height * 0.9;
    let rect = frame.rect(x, y, width, height);
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect,
        radius: height * 0.5 * s,
        color: if lit {
            color::GOLD_BRIGHT
        } else {
            color::alpha(color::HOLO, 0.16)
        },
    });
    text(
        canvas,
        TextFamily::Display,
        format_args!("New"),
        frame.rect(x, y + (height - size * 1.25) * 0.5, width, size * 1.25),
        size * s,
        if lit { INK } else { color::MUTED },
        FontWeight::Semibold,
        TextAlign::Center,
    );
    width
}

/// The star and the padlock a shader carries, as one run of SJK Symbols' glyphs ("" for
/// neither).
pub(super) fn glyphs(favourite: bool, locked: bool) -> String {
    match (favourite, locked) {
        (true, true) => format!("{} {}", symbols::EDITION, symbols::LOCK),
        (true, false) => symbols::EDITION.to_string(),
        (false, true) => symbols::LOCK.to_string(),
        (false, false) => String::new(),
    }
}

/// The star and the padlock on a dark pill `height` tall whose right edge is at `right`
/// (frame pixels, from `y`), the glyphs gold; nothing for neither. Returns the pill's left
/// edge (`right` for none).
pub(super) fn glyph_pill(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    [right, y]: [f32; 2],
    height: f32,
    favourite: bool,
    locked: bool,
) -> f32 {
    let words = glyphs(favourite, locked);
    if words.is_empty() {
        return right;
    }
    let s = frame.s;
    let size = height * 0.72;
    let count = if favourite && locked { 2.0 } else { 1.0 };
    let width = size * (1.15 * count + 0.3 * (count - 1.0)) + height * 0.6;
    let x = right - width;
    let rect = frame.rect(x, y, width, height);
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect,
        radius: height * 0.5 * s,
        color: color::alpha(color::SPACE, 0.82),
    });
    let _ = canvas.draw_list_mut().push(DrawCommand::Border {
        rect,
        radius: height * 0.5 * s,
        width: 1.0 * s,
        color: color::alpha(color::GOLD, 0.6),
    });
    text(
        canvas,
        TextFamily::Body,
        format_args!("{words}"),
        frame.rect(x, y + (height - size * 1.3) * 0.5, width, size * 1.3),
        size * s,
        color::GOLD_BRIGHT,
        FontWeight::Semibold,
        TextAlign::Center,
    );
    x
}

/// One toggle on the chosen shader's card at `[x, y]` ([`TOGGLE`] square): `glyph` gold on
/// a gold-ringed dark square while `on`, else muted on an outlined one; dimmed and quiet
/// while not `enabled` (a lock on a shader not held). It answers `token` either way, so
/// its hover card can say why.
pub(super) fn toggle(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    glyph: char,
    on: bool,
    enabled: bool,
    token: u16,
) {
    let s = frame.s;
    let rect = frame.rect(x, y, TOGGLE, TOGGLE);
    let hovered = canvas.token_hovered(token) && enabled;
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect,
        radius: 12.0 * s,
        color: if on {
            color::alpha(color::GOLD, if hovered { 0.34 } else { 0.24 })
        } else {
            color::alpha(color::HOLO, if hovered { 0.14 } else { 0.05 })
        },
    });
    let _ = canvas.draw_list_mut().push(DrawCommand::Border {
        rect,
        radius: 12.0 * s,
        width: if on { 1.6 } else { 1.2 } * s,
        color: match (on, enabled) {
            (true, _) => color::GOLD,
            (false, true) => color::alpha(color::HOLO, if hovered { 0.7 } else { 0.4 }),
            (false, false) => color::alpha(color::HOLO, 0.16),
        },
    });
    text(
        canvas,
        TextFamily::Body,
        format_args!("{glyph}"),
        frame.rect(x, y + 9.0, TOGGLE, 26.0),
        21.0 * s,
        match (on, enabled) {
            (true, _) => color::GOLD_BRIGHT,
            (false, true) => color::TEXT,
            (false, false) => color::alpha(color::QUIET, 0.6),
        },
        FontWeight::Semibold,
        TextAlign::Center,
    );
    canvas.hit_region(token, rect);
}

/// How wide the two toggles stand side by side.
pub(super) const TOGGLES: f32 = TOGGLE * 2.0 + TOGGLE_GAP;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_glyphs_say_the_star_then_the_padlock() {
        assert_eq!(glyphs(false, false), "");
        assert_eq!(glyphs(true, false), symbols::EDITION.to_string());
        assert_eq!(glyphs(false, true), symbols::LOCK.to_string());
        assert_eq!(
            glyphs(true, true),
            format!("{} {}", symbols::EDITION, symbols::LOCK)
        );
    }
}
