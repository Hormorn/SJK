//! Unboxed composer, channel choices, and a compact player-action popover.

use super::*;
use crate::chat::interaction::LATEST;

impl ChatOverlay {
    pub(in crate::chat) fn build_composer(&mut self, font: &UiFont, g: &Geometry, ms: u64) {
        let Some(input) = &self.input else {
            return;
        };
        let alpha = Tween::new(0.0, 1.0, self.opened_ms, ENTER_MS, Easing::EaseOutCubic).sample(ms);
        let y = g.bottom + 18.0 * g.scale;
        self.ui.push_opacity(alpha);
        // The row's channels (`chat/tabs.rs`), each as wide as its word, an even gap
        // between them; the hit area reaches half the gap either side.
        let (row, count, row_end) = self.row_layout(font);
        let pointer = self.pointer_at;
        for &(channel, x, width) in &row[..count] {
            let token = super::super::tabs::token(channel);
            let text = Rect::new(
                g.left + x * g.scale,
                y,
                (width + 4.0) * g.scale,
                26.0 * g.scale,
            );
            let pad = super::super::tabs::ROW_GAP * 0.5 * g.scale;
            let hit = Rect::new(
                text.x - pad,
                y,
                (width * g.scale) + pad * 2.0,
                26.0 * g.scale,
            );
            let selected = input.channel == channel;
            self.ui.hit_region(token, hit);
            // Hovered as this frame lays the row out (`pointer_at`).
            let hovered = self.ui.token_hovered(token) && pointer.is_none_or(|at| hit.contains(at));
            self.ui.text(
                super::super::tabs::label(channel),
                text,
                14.0 * g.scale,
                if selected || hovered {
                    tint(channel, 1.0)
                } else {
                    Color::new(0.786, 0.827, 0.865, 1.0)
                },
                FontWeight::Semibold,
                0.0,
            );
            if selected {
                self.ui.accent_bar(
                    Rect::new(text.x, text.bottom(), width * g.scale, 2.0 * g.scale),
                    tint(channel, 1.0),
                );
            }
        }
        // A whisper's recipient on a line of its own under the row (`recipient.rs`),
        // which moves the field and its notes down.
        let whisper = input.channel == Channel::Whisper;
        let sjk = input.channel == Channel::Sjk;
        // The ×, + and cog (`chat/row_tools.rs`); the notes start after them.
        let row_end = self.build_row_tools(font, g, &row[..count], row_end, y);
        let below = if whisper {
            recipient::ROW * g.scale
        } else {
            0.0
        };
        if whisper {
            self.build_recipient(font, g, y + 30.0 * g.scale);
        }
        // SJK chat said in this server's All chat too: a small red note says so.
        if sjk && let Some(note) = self.said_in_all.map(|words| words.note) {
            red_note(
                &mut self.ui,
                note,
                [g.left + row_end * g.scale, y + 1.0 * g.scale],
                g.width - (row_end + 120.0) * g.scale,
                font,
                g,
                alpha,
            );
        }
        if self.scroll > 0 {
            let rect = Rect::new(
                g.left + g.width - 116.0 * g.scale,
                y,
                116.0 * g.scale,
                26.0 * g.scale,
            );
            self.ui.hit_region(LATEST, rect);
            self.ui.text_fmt_aligned(
                format_args!("Latest +{}", self.unread),
                rect,
                12.0 * g.scale,
                tint(Channel::Global, 1.0),
                FontWeight::Semibold,
                0.0,
                TextAlign::End,
            );
        }
        // A whisper that goes through the game server, not encrypted (the recipient does
        // not take encrypted whispers, or `cl_e2eWhispers` is off): a red note under the
        // field says so for as long as the composer is on it (Sol, 22/10/2026).
        let plain_whisper = whisper
            && !self
                .input
                .as_ref()
                .and_then(|input| input.recipient)
                .is_some_and(|target| self.takes_encrypted(target.slot()));
        self.build_draft(font, g, y + 42.0 * g.scale + below, ms);
        if plain_whisper {
            red_note(
                &mut self.ui,
                PLAIN_WHISPER_NOTE,
                [g.left, y + 94.0 * g.scale + below],
                g.width,
                font,
                g,
                alpha,
            );
        }
        // A link just copied with Ctrl+C says so a moment (`chat/links.rs`).
        let copied = self
            .link_copied_ms
            .is_some_and(|at| ms.saturating_sub(at) < super::super::links::COPIED_MS);
        let notice = match self.notice {
            "" if copied => super::super::links::COPIED_NOTE,
            notice => notice,
        };
        if !notice.is_empty() {
            let notice_y = y + if plain_whisper { 124.0 } else { 94.0 } * g.scale + below;
            self.ui.text(
                notice,
                Rect::new(g.left, notice_y, g.width, 20.0 * g.scale),
                11.0 * g.scale,
                Color::new(0.832, 0.87, 0.901, 0.93),
                FontWeight::Regular,
                0.0,
            );
        }
        // The + or cog menu lies over the feed, so it goes last.
        self.build_row_menu(font, g, y);
        self.ui.pop_opacity();
    }
}

/// A small red note in a pill at `at` (its top-left), as wide as its words and at most
/// `room`, the words centred in it.
fn red_note(
    ui: &mut MenuCanvas,
    note: &str,
    at: [f32; 2],
    room: f32,
    font: &UiFont,
    g: &Geometry,
    alpha: f32,
) {
    let size = 12.0 * g.scale;
    let ink = visible_text_width_face(font, note, size / font.height.max(1.0), TextFace::Semibold);
    // A few pixels more than the words: measured to the pixel, the last letter was cut
    // by rounding ("...can read i").
    let width = (ink + 24.0 * g.scale).min(room);
    let rect = Rect::new(at[0], at[1], width, 24.0 * g.scale);
    let _ = ui.draw_list_mut().push(DrawCommand::RoundedRect {
        rect,
        radius: 5.0 * g.scale,
        color: Color::new(0.55, 0.06, 0.06, 0.82 * alpha),
    });
    let _ = ui.draw_list_mut().push(DrawCommand::Border {
        rect,
        radius: 5.0 * g.scale,
        width: 1.0 * g.scale,
        color: Color::new(1.0, 0.36, 0.32, 0.9 * alpha),
    });
    let end = layout::fitting_end(note, font, (width - 20.0 * g.scale).max(0.0), size);
    // Text draws from the top of its box: a box of the text's own size, set in the
    // pill's middle, centres the words.
    ui.text_fmt_aligned(
        format_args!("{}", &note[..end]),
        Rect::new(
            rect.x + 10.0 * g.scale,
            rect.y + (rect.height - size) * 0.5,
            width - 20.0 * g.scale,
            size,
        ),
        size,
        Color::new(1.0, 0.86, 0.84, 1.0),
        FontWeight::Semibold,
        0.0,
        TextAlign::Center,
    );
}

/// What the red note under the composer says of a whisper that is not encrypted.
pub(in crate::chat) const PLAIN_WHISPER_NOTE: &str =
    "Not private: the server logs this whisper and its owner can read it";
