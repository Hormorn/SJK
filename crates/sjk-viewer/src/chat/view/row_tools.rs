//! The composer row's ×, + and cog, and their menus (`chat/row_tools.rs`), drawn as
//! strokes rather than glyphs so they stay sharp at any scale.

use super::*;
use crate::chat::row_tools::{ADD, CLOSE, ITEM, Item, RowMenu, SETTINGS};

/// The tools' icon box, in 1080p pixels at the chat's scale.
const ICON: f32 = 16.0;
/// The room between the row's last word and the +, and between the + and the cog.
const ICON_GAP: f32 = 4.0;
/// The row's words' size (`chat/tabs.rs`), which the icons are set level with.
const LABEL_SIZE: f32 = 14.0;
/// A menu row's height and text size.
const MENU_ROW: f32 = 24.0;
const MENU_SIZE: f32 = 13.0;
/// The words' colour when nothing points at them, the row's own.
const IDLE: Color = Color::new(0.786, 0.827, 0.865, 1.0);

impl ChatOverlay {
    /// The × on the hovered button, then the + (when a chat is off the row) and the cog
    /// after the row's words. `row` is the row as laid out, `y` its top. Returns where
    /// the tools end, in 1080p pixels at the chat's scale, for the notes after them.
    pub(in crate::chat) fn build_row_tools(
        &mut self,
        font: &UiFont,
        g: &Geometry,
        row: &[(Channel, f32, f32)],
        row_end: f32,
        y: f32,
    ) -> f32 {
        let pointer = self.pointer_at;
        // Level with the middle of the words' capitals and as tall as them (Sol,
        // 06/11/2026: the first ones sat low and drew twice the words' height).
        let em = LABEL_SIZE * g.scale / font.height.max(1.0);
        let middle = y + font.capital_middle(em);
        let cap = font.capital_height(em).max(1.0);
        let stroke = (cap * 0.14).max(1.0);
        let half_gap = crate::chat::tabs::ROW_GAP * 0.5 * g.scale;
        for &(channel, x, width) in row {
            if !Self::closable(channel) {
                continue;
            }
            let place = crate::chat::tabs::ROW
                .iter()
                .position(|row| *row == channel)
                .unwrap_or(0) as u16;
            let token = CLOSE + place;
            let word_end = g.left + (x + width) * g.scale;
            let button = Rect::new(
                g.left + x * g.scale - half_gap,
                y,
                width * g.scale + half_gap * 2.0,
                26.0 * g.scale,
            );
            let close = Rect::new(
                word_end + 1.0 * g.scale,
                middle - 6.0 * g.scale,
                12.0 * g.scale,
                12.0 * g.scale,
            );
            // Only while the pointer is on the button or its ×, so the row stays quiet.
            if !pointer.is_some_and(|at| button.contains(at) || close.contains(at)) {
                continue;
            }
            self.ui.hit_region(token, close);
            let on = pointer.is_some_and(|at| close.contains(at));
            if on {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: close,
                    radius: 6.0 * g.scale,
                    color: Color::new(1.0, 0.36, 0.32, 0.28),
                });
            }
            let ink = if on {
                Color::new(1.0, 0.86, 0.84, 1.0)
            } else {
                Color::new(0.786, 0.827, 0.865, 0.75)
            };
            cross(&mut self.ui, close, cap * 0.3, stroke, ink);
        }
        let open = self.input.as_ref().and_then(|input| input.menu);
        let mut x = g.left + row_end * g.scale - half_gap + ICON_GAP * g.scale;
        let mut tools = Vec::with_capacity(2);
        if self.hidden().next().is_some() {
            tools.push((ADD, RowMenu::Add));
        }
        tools.push((SETTINGS, RowMenu::Settings));
        for (token, menu) in tools {
            let rect = Rect::new(
                x,
                middle - ICON * 0.5 * g.scale,
                ICON * g.scale,
                ICON * g.scale,
            );
            self.ui.hit_region(token, rect);
            let hovered = pointer.is_some_and(|at| rect.contains(at));
            let picked = open == Some(menu);
            if hovered || picked {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 4.0 * g.scale,
                    color: Color::new(1.0, 1.0, 1.0, if picked { 0.14 } else { 0.08 }),
                });
            }
            let ink = if hovered || picked {
                Color::new(1.0, 1.0, 1.0, 1.0)
            } else {
                IDLE
            };
            match menu {
                RowMenu::Add => plus(&mut self.ui, rect, cap * 0.45, stroke, ink),
                RowMenu::Settings => cog(&mut self.ui, rect, cap, ink),
            }
            if picked {
                self.menu_left = rect.x;
            }
            x += (ICON + ICON_GAP) * g.scale;
        }
        (x - g.left) / g.scale
    }

    /// The open + or cog menu, over the feed above the row whose top is `y`.
    pub(in crate::chat) fn build_row_menu(&mut self, font: &UiFont, g: &Geometry, y: f32) {
        let items = self.menu_items();
        if items.is_empty() {
            return;
        }
        let row = MENU_ROW * g.scale;
        let pad = 6.0 * g.scale;
        let size = MENU_SIZE * g.scale;
        let em = size / font.height.max(1.0);
        let words: Vec<(String, Option<&str>)> = items
            .iter()
            .map(|item| (item_words(*item), self.item_state(*item)))
            .collect();
        let state_room = 34.0 * g.scale;
        let widest = words
            .iter()
            .map(|(words, state)| {
                visible_text_width_face(font, words, em, TextFace::Semibold)
                    + if state.is_some() { state_room } else { 0.0 }
            })
            .fold(0.0_f32, f32::max);
        let width = (widest + 28.0 * g.scale + pad * 2.0).min(g.width);
        let height = items.len() as f32 * row + pad * 2.0;
        let panel = Rect::new(
            self.menu_left.min(g.left + g.width - width).max(g.left),
            y - height - 4.0 * g.scale,
            width,
            height,
        );
        // Text draws over every shape: what lies under the panel is left out.
        self.ui.cover(panel);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: panel,
            radius: 6.0 * g.scale,
            color: Color::new(0.03, 0.035, 0.05, 0.86),
        });
        let pointer = self.pointer_at;
        for (index, (item, (words, state))) in items.iter().zip(&words).enumerate() {
            let token = ITEM + index as u16;
            let rect = Rect::new(
                panel.x + pad,
                panel.y + pad + index as f32 * row,
                panel.width - pad * 2.0,
                row,
            );
            self.ui.hit_region(token, rect);
            let picked = matches!(item, Item::Filter(_)) && self.item_on(*item);
            let hovered = pointer.is_some_and(|at| rect.contains(at));
            if picked || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 4.0 * g.scale,
                    color: Color::new(1.0, 1.0, 1.0, if picked { 0.12 } else { 0.07 }),
                });
            }
            let ink = match item {
                Item::Add(channel) => tint(*channel, 1.0),
                _ if picked || hovered => Color::new(1.0, 1.0, 1.0, 1.0),
                _ => IDLE,
            };
            let text = Rect::new(
                rect.x + 8.0 * g.scale,
                rect.y + (row - size) * 0.5,
                rect.width - 16.0 * g.scale,
                size,
            );
            self.ui.text_fmt_aligned(
                format_args!("{words}"),
                text,
                size,
                ink,
                FontWeight::Semibold,
                0.0,
                TextAlign::Start,
            );
            if let Some(state) = state {
                let on = self.item_on(*item);
                self.ui.text_fmt_aligned(
                    format_args!("{state}"),
                    text,
                    size,
                    if on {
                        tint(Channel::Sjk, 1.0)
                    } else {
                        Color::new(0.786, 0.827, 0.865, 0.6)
                    },
                    FontWeight::Semibold,
                    0.0,
                    TextAlign::End,
                );
            }
        }
    }

    /// The On or Off a switch row shows at its right; none for the other rows.
    fn item_state(&self, item: Item) -> Option<&'static str> {
        match item {
            Item::TabSkips(_) | Item::Remember => {
                Some(if self.item_on(item) { "On" } else { "Off" })
            }
            Item::Add(_) | Item::Filter(_) => None,
        }
    }
}

/// What a menu row says.
fn item_words(item: Item) -> String {
    match item {
        Item::Add(channel) => crate::chat::tabs::label(channel).to_owned(),
        Item::Filter(true) => "Show only this chat".to_owned(),
        Item::Filter(false) => "Show every chat".to_owned(),
        Item::TabSkips(channel) => format!("Tab skips {}", crate::chat::tabs::label(channel)),
        Item::Remember => "Open on the last chat".to_owned(),
    }
}

fn line(ui: &mut MenuCanvas, from: [f32; 2], to: [f32; 2], width: f32, color: Color) {
    let _ = ui.draw_list_mut().push(DrawCommand::Line {
        from,
        to,
        width,
        color,
    });
}

fn centre_of(rect: Rect) -> [f32; 2] {
    [rect.x + rect.width * 0.5, rect.y + rect.height * 0.5]
}

/// An × of arms `arm` long from the centre of `rect`.
fn cross(ui: &mut MenuCanvas, rect: Rect, arm: f32, width: f32, color: Color) {
    let [x, y] = centre_of(rect);
    line(ui, [x - arm, y - arm], [x + arm, y + arm], width, color);
    line(ui, [x - arm, y + arm], [x + arm, y - arm], width, color);
}

/// A + of arms `arm` long from the centre of `rect`.
fn plus(ui: &mut MenuCanvas, rect: Rect, arm: f32, width: f32, color: Color) {
    let [x, y] = centre_of(rect);
    line(ui, [x - arm, y], [x + arm, y], width, color);
    line(ui, [x, y - arm], [x, y + arm], width, color);
}

/// A cog as tall as capitals `cap` high: a ring with eight teeth, centred in `rect`.
fn cog(ui: &mut MenuCanvas, rect: Rect, cap: f32, color: Color) {
    let centre = centre_of(rect);
    let _ = ui.draw_list_mut().push(DrawCommand::Arc {
        center: centre,
        radius: cap * 0.3,
        width: cap * 0.17,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color,
        knockout: None,
    });
    for tooth in 0..8 {
        let angle = tooth as f32 * std::f32::consts::FRAC_PI_4;
        let (sin, cos) = angle.sin_cos();
        line(
            ui,
            [centre[0] + cos * cap * 0.34, centre[1] + sin * cap * 0.34],
            [centre[0] + cos * cap * 0.56, centre[1] + sin * cap * 0.56],
            cap * 0.18,
            color,
        );
    }
}
