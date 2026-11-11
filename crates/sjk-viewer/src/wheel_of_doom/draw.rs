//! What the WHEEL OF DOOM draws on the screen: the words over each wheel (hidden behind
//! walls and far away, as the mystery boxes' are), the effect running and its seconds on
//! a small line of the HUD, and the Blind effect's darkness with its pinhole. The words are kept in a fixed set of strings
//! written in place, so a frame allocates nothing once they have grown.

use super::{AXLE, MOST, RADIUS};
use crate::GpuState;
use glam::Vec3;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};
use std::fmt::Write as _;
use std::time::Instant;

/// How high over the floor a wheel's words stand.
const RISE: f32 = AXLE + RADIUS + 28.0;
/// Farthest a wheel's words are read from, and where they start to fade.
const RANGE: f32 = 1_600.0;
const FADE_FROM: f32 = 1_100.0;
/// The words' slots: two spare (the price card's, before 11/11/2026), the HUD's line,
/// then two a wheel.
const HUD_LINE: usize = 2;
const WHEEL_BASE: usize = 3;
const TEXTS: usize = WHEEL_BASE + MOST * 2;
/// The HUD line's top, in pixels at 1080 lines (under the box hunt's card).
const HUD_TOP: f32 = 132.0;
/// The Blind pinhole's radius and its soft edge, in pixels at 1080 lines.
const PINHOLE: f32 = 46.0;
const PINHOLE_EDGE: f32 = 26.0;
/// The wheel's red, the gold and the darkness.
const RED: Color = Color::new(0.95, 0.22, 0.18, 1.0);
const GOLD: Color = Color::new(1.0, 0.8, 0.32, 1.0);

/// The draw list and its words.
pub(crate) struct Draw {
    pub(crate) list: DrawList,
    texts: [String; TEXTS],
}

impl Default for Draw {
    fn default() -> Self {
        Self {
            // The darkness's five shapes, the card's four, the HUD line's two, and each
            // wheel's two lines over their shadows.
            list: DrawList::new(16 + MOST * 4),
            texts: std::array::from_fn(|_| String::with_capacity(48)),
        }
    }
}

impl Draw {
    fn set(&mut self, slot: usize, words: std::fmt::Arguments<'_>) -> TextId {
        let text = &mut self.texts[slot];
        text.clear();
        let _ = text.write_fmt(words);
        TextId(slot as u32)
    }

    #[allow(clippy::too_many_arguments)]
    fn line(
        &mut self,
        rect: Rect,
        text: TextId,
        size: f32,
        color: Color,
        unit: f32,
        weight: FontWeight,
    ) {
        for (offset, color) in [
            (1.5 * unit, Color::new(0.0, 0.0, 0.0, 0.75 * color.a)),
            (0.0, color),
        ] {
            let _ = self.list.push(DrawCommand::Text {
                rect: Rect::new(rect.x + offset, rect.y + offset, rect.width, rect.height),
                text,
                size,
                color,
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight,
                letter_spacing: 0.0,
            });
        }
    }
}

/// The Blind darkness at `dark` (0 to 1) round a pinhole at `centre`: a square of black
/// round the hole, a thick ring filling the square's corners, and a fainter ring as the
/// hole's soft edge.
fn push_blind(list: &mut DrawList, viewport: [f32; 2], centre: [f32; 2], dark: f32, unit: f32) {
    let black = Color::new(0.0, 0.0, 0.0, dark);
    let hole = PINHOLE * unit;
    let edge = PINHOLE_EDGE * unit;
    let half = hole + edge * 2.0;
    let [x, y] = centre;
    for rect in [
        Rect::new(0.0, 0.0, viewport[0], (y - half).max(0.0)),
        Rect::new(
            0.0,
            y + half,
            viewport[0],
            (viewport[1] - y - half).max(0.0),
        ),
        Rect::new(0.0, y - half, (x - half).max(0.0), half * 2.0),
        Rect::new(
            x + half,
            y - half,
            (viewport[0] - x - half).max(0.0),
            half * 2.0,
        ),
    ] {
        let _ = list.push(DrawCommand::SolidRect { rect, color: black });
    }
    // From the hole's edge out past the square's corners.
    let inner = hole + edge;
    let outer = half * std::f32::consts::SQRT_2 + 2.0;
    let _ = list.push(DrawCommand::Arc {
        center: centre,
        radius: (inner + outer) * 0.5,
        width: outer - inner,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color: black,
        knockout: None,
    });
    let _ = list.push(DrawCommand::Arc {
        center: centre,
        radius: hole + edge * 0.5,
        width: edge,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color: Color::new(0.0, 0.0, 0.0, dark * 0.6),
        knockout: None,
    });
}

impl GpuState {
    /// Lay this frame's WHEEL OF DOOM out and add its words to the HUD's text: the
    /// darkness of Blind always, the rest only while `shown` (not in the menus, the
    /// console, the scoreboard or the intermission).
    pub(crate) fn append_wheel_of_doom_draw(&mut self, viewport: [f32; 2], shown: bool) {
        let now = Instant::now();
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let [cx, cy] = self.hud.targeting.center(viewport);
        let crosshair = [cx * viewport[0], cy * viewport[1]];
        let toy = &mut self.wheel_of_doom;
        toy.draw.list.clear();
        if let Some(dark) = toy.effects.blind(now) {
            push_blind(&mut toy.draw.list, viewport, crosshair, dark, unit);
        }
        if !shown {
            return;
        }
        // The effect running and its seconds, or the curse waiting.
        let hud = match toy.effects.current(now) {
            Some((effect, left)) => Some(toy.draw.set(
                HUD_LINE,
                format_args!("WHEEL OF DOOM \u{b7} {} \u{b7} {left} s", effect.hud()),
            )),
            None if toy.effects.cursed() => Some(
                toy.draw
                    .set(HUD_LINE, format_args!("WHEEL OF DOOM \u{b7} CURSED...")),
            ),
            None => None,
        };
        if let Some(text) = hud {
            let size = 20.0 * unit;
            let width = (520.0 * unit).min(viewport[0]);
            let rect = Rect::new(
                (viewport[0] - width) * 0.5,
                HUD_TOP * unit,
                width,
                size * 1.3,
            );
            toy.draw
                .line(rect, text, size, RED, unit, FontWeight::Semibold);
        }
        self.append_wheel_of_doom_words(viewport, unit);
        let draw = &self.wheel_of_doom.draw;
        if draw.list.commands().is_empty() {
            return;
        }
        let (vertices, font) = self.game_fonts.target(
            crate::game_font::RetailFont::Headline,
            &mut self.text_vertices,
            &self.ui_font,
        );
        crate::ui_renderer::append_text_commands_where(
            &draw.list,
            |id| draw.texts.get(id.0 as usize).map_or("", String::as_str),
            |_, _| true,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
            crate::text::CodePalette::Legible,
        );
    }

    /// The words over each wheel seen: "WHEEL OF DOOM" and what a spin costs, "Spinning..."
    /// while it spins, then what it landed on.
    fn append_wheel_of_doom_words(&mut self, viewport: [f32; 2], unit: f32) {
        let Some(camera) = self.frame_view else {
            return;
        };
        let width = (440.0 * unit).min(viewport[0]);
        for index in 0..self.wheel_of_doom.placed.len().min(MOST) {
            let placed = &self.wheel_of_doom.placed[index];
            let point = placed.at + Vec3::Z * RISE;
            let sight = placed.at + Vec3::Z * AXLE;
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
                sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).expect("a point is a box"),
                1,
            );
            if trace.fraction < 1.0 || trace.start_solid || trace.all_solid {
                continue;
            }
            let alpha = (1.0 - (distance - FADE_FROM) / (RANGE - FADE_FROM)).clamp(0.0, 1.0);
            let near = (1.0 - distance / RANGE).clamp(0.45, 1.0);
            let head = 28.0 * unit * near;
            let small = 18.0 * unit * near;
            let x = (at[0] - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
            let slot = WHEEL_BASE + index * 2;
            let placed = &self.wheel_of_doom.placed[index];
            let stopped = placed.spin.as_ref().map(|spin| spin.stopped);
            let draw = &mut self.wheel_of_doom.draw;
            let (first, second) = match (stopped, &placed.spin) {
                (Some(true), Some(spin)) => {
                    draw.texts[slot].clear();
                    draw.texts[slot].push_str(&spin.title);
                    draw.texts[slot + 1].clear();
                    draw.texts[slot + 1].push_str(&spin.line);
                    (TextId(slot as u32), TextId(slot as u32 + 1))
                }
                (Some(false), _) => (
                    draw.set(slot, format_args!("WHEEL OF DOOM")),
                    draw.set(slot + 1, format_args!("Spinning...")),
                ),
                _ => (
                    draw.set(slot, format_args!("WHEEL OF DOOM")),
                    draw.set(slot + 1, format_args!("A DOOM coin a spin: press Use")),
                ),
            };
            let headline = if stopped == Some(true) { GOLD } else { RED };
            draw.line(
                Rect::new(x, at[1] - head * 1.3 - small * 1.4, width, head * 1.3),
                first,
                head,
                Color::new(headline.r, headline.g, headline.b, alpha),
                unit,
                FontWeight::Semibold,
            );
            draw.line(
                Rect::new(x, at[1] - small * 1.4, width, small * 1.3),
                second,
                small,
                Color::new(1.0, 1.0, 1.0, alpha),
                unit,
                FontWeight::Semibold,
            );
        }
    }
}
