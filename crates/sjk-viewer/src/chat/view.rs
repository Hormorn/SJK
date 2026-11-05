//! Floating message typography and deterministic motion on the shared canvas.

mod backdrop;
mod composer;
mod draft;
mod name;
mod recipient;

pub(super) use recipient::LIST_MAX as PM_LIST_MAX;
mod sjk_line;

/// The GIF picker's favourite mark.
pub(crate) use name::star;

use super::*;
use crate::menu_widgets::MenuCanvas;
use crate::text::{Carry, TextFace, visible_text_width_face, visible_text_width_style};
use layout::Geometry;
use sjk_ui::{Color, DrawCommand, Easing, FontWeight, Rect, TextAlign};

pub(super) fn tint(channel: Channel, alpha: f32) -> Color {
    match channel {
        Channel::Global => Color::new(0.70, 0.88, 0.98, alpha),
        Channel::Team => Color::new(0.40, 0.86, 0.86, alpha),
        Channel::Whisper => Color::new(0.84, 0.68, 1.0, alpha),
        // The SJK UI's gold (`#E8B84A`), apart from the game's blues.
        Channel::Sjk => Color::new(0.91, 0.72, 0.29, alpha),
        // jaPRO's `<Admin>` yellow, softened.
        Channel::Admin => Color::new(0.98, 0.88, 0.40, alpha),
        Channel::Jof => clan_colour(sjk_identity::HubChannel::Jof, alpha),
        Channel::Sith => clan_colour(sjk_identity::HubChannel::Sith, alpha),
    }
}

/// A clan chat's colour (`docs/hub-chat.md`, "Clan chats"): its messages, label and
/// button, as the SJK chat's gold is SJK's. JoF's light cyan (`#8FDDF5`) after its
/// tick's, the Sith clan's rose red (`#FA8C9E`) after theirs, each far from the game's
/// colour codes, the SJK gold and the encrypted whispers' pink (a test keeps them so).
pub(crate) fn clan_colour(channel: sjk_identity::HubChannel, alpha: f32) -> Color {
    match channel {
        sjk_identity::HubChannel::Jof => Color::new(0.56, 0.867, 0.96, alpha),
        sjk_identity::HubChannel::Sith => Color::new(0.98, 0.55, 0.62, alpha),
        _ => crate::sjk_chat_look::gold(alpha),
    }
}

/// An end-to-end encrypted whisper's pink (`#F5ADD1`): softer on the eye than the
/// game's whisper magenta, and apart from the plain whisper's lilac.
pub(crate) fn encrypted_pink(alpha: f32) -> Color {
    Color::new(0.96, 0.68, 0.82, alpha)
}

pub(super) fn visibility(age: u64, active: bool) -> f32 {
    let entering = Tween::new(0.0, 1.0, 0, ENTER_MS, Easing::EaseOutCubic).sample(age);
    let leaving = if active {
        1.0
    } else {
        Tween::new(1.0, 0.0, HOLD_MS, FADE_MS as u32, Easing::SmoothStep).sample(age)
    };
    entering * leaving
}

/// Put chat beside the SJK scoreboard while preserving the player's full chat
/// text scale. The column gets narrower than the normal feed, so larger text
/// wraps sooner and its rows grow just as they do outside the scoreboard.
fn place_beside_scoreboard(g: &mut Geometry, viewport: [f32; 2], font_scale: f32) {
    let layout = crate::scoreboard::layout::Layout::new(viewport);
    g.scale = layout.scale * font_scale;
    g.left = layout.chat_left;
    g.width = layout.chat_width;
    g.top = 150.0 * layout.scale;
    g.bottom = viewport[1] - 200.0 * g.scale;
    g.font = layout::FONT_1080 * g.scale;
    g.row = g.font * layout::ROW_PITCH;
}

impl ChatOverlay {
    pub(super) fn build(&mut self, draw_feed: bool, font: &UiFont, viewport: [f32; 2], ms: u64) {
        if self.layout_viewport != viewport {
            for line in &mut self.lines {
                line.y = None;
            }
            self.layout_viewport = viewport;
            self.pressed_action = None;
        }
        self.ui.begin_transparent(viewport);
        self.visible_people.fill(None);
        self.visible_blades.fill(None);
        let mut g = self.options.geometry(viewport);
        if self.scoreboard_layout == crate::scoreboard::behavior::ChatLayout::SideColumn {
            place_beside_scoreboard(&mut g, viewport, self.options.font);
        }
        if let crate::scoreboard::behavior::ChatLayout::Classic { .. } = self.scoreboard_layout {
            // CG_ChatBox_DrawStrings overrides only x/y; retain the player's
            // font, width, expiry and line-count choices.
            [g.left, g.bottom] = crate::scoreboard::behavior::ChatLayout::classic_origin(viewport);
        }
        let hide_feed = matches!(
            self.scoreboard_layout,
            crate::scoreboard::behavior::ChatLayout::Classic { hide_feed: true }
        );
        let feed_start = self.ui.next_text_id();
        if !hide_feed && (self.is_typing() || (draw_feed && self.options.lifetime != 0)) {
            self.build_feed(font, &g, ms);
        }
        let feed_text = (feed_start, self.ui.next_text_id());
        let center_start = self.ui.next_text_id();
        if let Some((text, received)) = &self.center {
            let age = ms.saturating_sub(*received);
            if age < self.options.center_time && !self.scoreboard_layout.active() {
                // CG_DrawCenterString (cg_draw.c:4488): every `\n` row is its own
                // centred draw with the base colour, so colour codes never bleed
                // into the next row; the block sits around cg.centerPrintY (0.30).
                let base = self.ui.theme().foreground;
                let fade = self.options.center_time.saturating_sub(age).min(200) as f32 / 200.0;
                let color = Color::new(base.r, base.g, base.b, base.a * fade);
                let size = self.options.center_size;
                let center_scale = crate::ui_scale::height_scale(viewport[1]);
                let row = center_row(size) * center_scale;
                let rows = center_rows(text).count() as f32;
                let height = if self.options.center_height == 0.0 {
                    self.combat.height.unwrap_or(0.0)
                } else {
                    self.options.center_height
                };
                let mut y = viewport[1] * if height == 0.0 { 0.30 } else { height / 480.0 }
                    - rows * row * 0.5;
                for line in center_rows(text) {
                    self.ui.text_aligned(
                        line,
                        Rect::new(32.0, y, viewport[0] - 64.0, row),
                        CENTER_FONT_1080 * crate::game_font::HEADLINE_GROWTH * center_scale * size,
                        color,
                        FontWeight::Semibold,
                        0.0,
                        TextAlign::Center,
                    );
                    y += row;
                }
            } else if age >= self.options.center_time {
                self.combat.spare = self.center.take().unwrap().0;
                self.combat.height = None;
            }
        }
        self.center_text = (center_start, self.ui.next_text_id());
        if !self.scoreboard_layout.active() {
            self.draw_plums();
        }
        let composer_start = self.ui.next_text_id();
        if self.is_typing() {
            // JoF's message input is independent of its chat feed. Keep SJK's
            // composer inside the window when the feed sits at virtual y=475.
            // The composer keeps its pre-19/10/2026 proportion to the feed's text,
            // which grew to EternalJK's size then.
            let composer_scale = g.scale * crate::chat::text_sizes::CHAT_FACTOR;
            if matches!(
                self.scoreboard_layout,
                crate::scoreboard::behavior::ChatLayout::Classic { .. }
            ) {
                g.bottom = g.bottom.min(viewport[1] - 200.0 * composer_scale);
            }
            let feed_scale = std::mem::replace(&mut g.scale, composer_scale);
            self.build_composer(font, &g, ms);
            g.scale = feed_scale;
        }
        self.chat_text = [feed_text, (composer_start, self.ui.next_text_id())];
        // A find's blade under the pointer shows large, over the feed and the composer.
        self.build_find_blade(&g, viewport);
        // A name under the pointer shows its profile card, over everything.
        self.follow_card(self.is_typing(), ms);
        self.build_card(font, &g, viewport);
        self.ui.finish(u16::MAX);
    }

    fn build_feed(&mut self, font: &UiFont, g: &Geometry, ms: u64) {
        let active = self.is_typing() || self.options.history;
        let limit = self.options.lines;
        self.follow_gif_switch();
        // The links' places, for Ctrl+C on one while the pointer is free (`links.rs`).
        self.links.clear();
        let follow_links = self.is_typing();
        let mut visible = [(0_usize, 0.0_f32, 0.0_f32, None); MAX_VISIBLE];
        let mut count = 0;
        let mut bottom = g.bottom;
        let mut skipped = 0;
        for index in (0..self.lines.len()).rev() {
            let line = &self.lines[index];
            if !self.shown(line) {
                continue;
            }
            if skipped < self.scroll {
                skipped += 1;
                continue;
            }
            let age = ms.saturating_sub(line.received_ms);
            let alpha = if active {
                visibility(age, true)
            } else {
                visibility(age, true)
                    * (self.options.lifetime.saturating_sub(age).min(2000) as f32 / 2000.0)
            };
            if (!active && line.muted) || alpha <= 0.0 {
                continue;
            }
            // A line with a sender flows from its name's row ([`sjk_line`]).
            let indent = line_marks(line, &self.slot_marks).map_or(0.0, |(name, marks)| {
                sjk_line::Prefix::new(name, marks, font, g).width()
            });
            let wrap_width = g.width - 12.0 * g.scale;
            // An SJK line's GIF under its text takes room of its own (`feed_gif.rs`).
            let gif = self.gif_shown(index);
            let line = &mut self.lines[index];
            line.wrap
                .update_spaced(&line.body, font, wrap_width, g.font, indent, g.spacing);
            let rows = if line.muted { 1 } else { line.wrap.len };
            let text_height = rows as f32 * g.row + g.gap;
            let mut gif = gif;
            let mut top = bottom - text_height - feed_gif::room(g, gif);
            if top < g.top && count == 0 && gif.is_some() {
                // Too tall for the box with its GIF: its text alone.
                gif = None;
                top = bottom - text_height;
            }
            if top < g.top || count == limit {
                break;
            }
            visible[count] = (index, top, alpha, gif);
            count += 1;
            bottom = top;
        }
        // Where each shown line is drawn this frame, as it slides in and moves up.
        let mut origins = [[0.0_f32; 2]; MAX_VISIBLE];
        for (origin, &(index, top, ..)) in origins.iter_mut().zip(&visible[..count]) {
            let line = &mut self.lines[index];
            let age = ms.saturating_sub(line.received_ms);
            let y = line.y.get_or_insert_with(|| Tween::settled(top));
            if (y.target() - top).abs() > 0.1 {
                y.retarget(top, ms, ENTER_MS, Easing::EaseOutCubic);
            }
            let slide =
                Tween::new(14.0 * g.scale, 0.0, 0, ENTER_MS, Easing::EaseOutCubic).sample(age);
            *origin = [g.left + slide, y.sample(ms)];
        }
        // Every band before any line, under the pictures of the rows next to it.
        if self.options.backdrop > 0.0 {
            for (&origin, &(index, _, alpha, _)) in origins.iter().zip(&visible[..count]) {
                let line = &self.lines[index];
                let prefix = line_marks(line, &self.slot_marks)
                    .map(|(name, marks)| sjk_line::Prefix::new(name, marks, font, g));
                backdrop::draw(
                    &mut self.ui,
                    line,
                    prefix.as_ref(),
                    font,
                    g,
                    origin,
                    self.options.backdrop * alpha,
                );
            }
        }
        for (token, (index, _, alpha, gif)) in visible[..count].iter().copied().enumerate().rev() {
            let [x, y] = origins[token];
            let line = &self.lines[index];
            let Some((name, marks)) = line_marks(line, &self.slot_marks) else {
                // No sender: the server's own words, as one text.
                for (row, range) in line.wrap.rows[..line.wrap.len].iter().enumerate() {
                    let truncated = row + 1 == line.wrap.len && range.end < line.body.len();
                    body_row(
                        &mut self.ui,
                        BodyRow {
                            forced: line.incoming_colour(self.options.incoming),
                            text: &line.body[range.clone()],
                            carry: line.wrap.carry[row],
                            marks: &line.emojis,
                            emojis: self.options.emojis.then_some(&self.emojis),
                            truncated,
                            spacing: g.spacing,
                        },
                        font,
                        Rect::new(x, y + row as f32 * g.row, g.width, row_box(g)),
                        g.font,
                        Color::new(0.982, 0.987, 0.996, alpha),
                    );
                }
                if follow_links && !line.muted {
                    let place = links::RowPlace {
                        origin: [x, y],
                        indent: 0.0,
                        size: g.font,
                        spacing: g.spacing,
                        row: g.row,
                    };
                    let rows = &line.wrap.rows[..line.wrap.len];
                    let key = (index, line.received_ms);
                    self.links.add_line(font, key, &line.body, rows, place);
                }
                continue;
            };
            let prefix = sjk_line::Prefix::new(name, marks, font, g);
            let hovered = active && self.ui.token_hovered(token as u16);
            // The only difference between the two chats: the SJK chat's gold (or a
            // holocron tier's colour), or the game line's own codes over white.
            let colour = match &line.hub {
                Some(hub) => hub.tier.map_or_else(
                    || match line.channel.clan() {
                        Some(clan) => clan_colour(clan, alpha),
                        None => crate::sjk_chat_look::gold(alpha),
                    },
                    |tier| tier.colour_alpha(alpha),
                ),
                None if line.encrypted => encrypted_pink(alpha),
                None => Color::new(0.982, 0.987, 0.996, alpha),
            };
            let name = sjk_line::draw(
                &mut self.ui,
                sjk_line::Line {
                    name,
                    body: &line.body,
                    muted: line.muted,
                    wrap: &line.wrap,
                    marks: &line.emojis,
                    emojis: self.options.emojis.then_some(&self.emojis),
                    hovered,
                    colour,
                    forced: line.incoming_colour(self.options.incoming),
                    label_colour: if line.encrypted {
                        encrypted_pink(alpha * 0.85)
                    } else {
                        tint(line.channel, alpha * 0.75)
                    },
                    prize: line
                        .hub
                        .as_ref()
                        .and_then(|hub| hub.prize)
                        .map(|prize| (prize.range(&line.body), prize.colour)),
                },
                &prefix,
                font,
                g,
                [x, y],
                alpha,
            );
            // A muted line shows a note, and a find's words hold no link.
            if follow_links
                && !line.muted
                && line.hub.as_ref().is_none_or(|hub| hub.prize.is_none())
            {
                let place = links::RowPlace {
                    origin: [x, y],
                    indent: prefix.width(),
                    size: g.font,
                    spacing: g.spacing,
                    row: g.row,
                };
                let rows = &line.wrap.rows[..line.wrap.len];
                let key = (index, line.received_ms);
                self.links.add_line(font, key, &line.body, rows, place);
            }
            if let Some(shown) = gif {
                let rows = if line.muted { 1 } else { line.wrap.len };
                crate::chat_gifs::draw::paint(
                    &mut self.ui,
                    feed_gif::LOOK,
                    shown,
                    [x, y + rows as f32 * g.row],
                    g.width - 12.0 * g.scale,
                    feed_gif::scale(g),
                    alpha,
                );
            }
            // Resting the pointer on the sender's name shows their profile card.
            if !active {
                continue;
            }
            // Resting it on a find's blade shows the blade large (`find_blade.rs`).
            if let (Some(skin), Some(rect)) = (prefix.blade, prefix.blade_rect(font, [x, y])) {
                let colour = line
                    .hub
                    .as_ref()
                    .and_then(|hub| hub.prize)
                    .map_or(skin.tier.colour(), |prize| prize.colour);
                self.visible_blades[token] = Some((skin, rect, colour));
                self.ui.hit_region(find_blade::BLADE + token as u16, rect);
            }
            if let Some(hub) = &line.hub {
                self.visible_people[token] = Some(card::Who::Hub(hub.id));
                self.ui.hit_region(token as u16, name);
            } else if let Some(target) = line
                .sender
                .filter(|target| self.roster.name(*target).is_some())
            {
                self.visible_people[token] = Some(card::Who::Player(target));
                self.ui.hit_region(token as u16, name);
            }
        }
        // The link under the pointer is lit as a name under it is, on every row it is
        // drawn on: Ctrl+C copies it (`links.rs`).
        if let Some(spot) = self.link_under_pointer() {
            for rect in self.links.pieces(spot) {
                self.ui.accent_bar(rect, Color::new(0.70, 0.88, 0.98, 0.16));
            }
        }
    }
}

/// The name a line with a sender shows and what its first row marks, `None` for a line
/// with no sender (a server's words). A game line's sender the hub knows in that slot
/// gets SJK's emblem, the verified tick and the name as they wear it
/// (`chat/marks.rs`).
fn line_marks<'a>(
    line: &'a ChatLine,
    slots: &'a [super::marks::SlotMark],
) -> Option<(&'a str, sjk_line::Marks<'a>)> {
    if let Some(hub) = &line.hub {
        let marks = sjk_line::Marks {
            // A drop starts with its holocron's picture, not SJK's emblem.
            sjk: hub.tier.is_none(),
            icon: hub.client,
            mark: crate::ui_renderer::mark_texture(hub.verified, hub.tick, hub.trust),
            tier: hub.tier,
            // Which chats it was said in; a holocron drop and the SJK team's lines are
            // the hub's news, unlabelled.
            label: match hub.tier {
                _ if hub.official => "",
                Some(_) => "",
                None if hub.in_all => "ALL/SJK",
                None => super::tabs::label(line.channel),
            },
            emoji: &hub.emoji,
            accent: hub.official,
            blade: hub.prize.and_then(|prize| prize.skin),
        };
        return Some((&line.name, marks));
    }
    if line.name.is_empty() {
        return None;
    }
    // The player's own encrypted whisper: "To <recipient>", no sender, with the
    // recipient's marks as the SJK chat shows a sender's.
    if line.encrypted && line.sender.is_none() {
        let mark = line
            .recipient
            .and_then(|slot| slots.get(usize::from(slot)))
            .filter(|mark| mark.sjk);
        let marks = sjk_line::Marks {
            sjk: true,
            mark: mark.and_then(|mark| {
                crate::ui_renderer::mark_texture(mark.verified, mark.tick, mark.trust)
            }),
            label: if line.verified {
                "VERIFIED"
            } else {
                "ENCRYPTED"
            },
            emoji: mark.map_or("", |mark| mark.emoji.as_str()),
            ..sjk_line::Marks::default()
        };
        return Some((&line.name, marks));
    }
    let target = line.sender?;
    let mark = slots
        .get(usize::from(target.slot()))
        .filter(|mark| mark.sjk);
    let marks = sjk_line::Marks {
        sjk: mark.is_some(),
        icon: None,
        mark: mark.and_then(|mark| {
            crate::ui_renderer::mark_texture(mark.verified, mark.tick, mark.trust)
        }),
        tier: None,
        label: match line.channel {
            _ if line.muted => "",
            _ if line.verified => "VERIFIED",
            _ if line.encrypted => "ENCRYPTED",
            Channel::Team => "TEAM",
            Channel::Whisper => "WHISPER",
            Channel::Global | Channel::Sjk | Channel::Admin | Channel::Jof | Channel::Sith => "",
        },
        emoji: mark.map_or("", |mark| mark.emoji.as_str()),
        accent: false,
        blade: None,
    };
    let name = mark
        .and_then(|mark| mark.name.as_deref())
        .unwrap_or(&line.name);
    Some((name, marks))
}

/// One wrapped row of a message body and the emojis its marks stand for.
struct BodyRow<'a> {
    forced: Option<Color>,
    text: &'a str,
    /// The colour code in force where the row starts: a wrapped message goes on in
    /// the colour the row before it ended in, not in the base colour.
    carry: Carry,
    marks: &'a [u16],
    /// The pictures, while `cg_chatBoxEmojis` is on; off, a mark is a blank.
    emojis: Option<&'a emoji::Emojis>,
    truncated: bool,
    /// Extra room after each character (`cg_chatBoxLetterSpacing`), in pixels.
    spacing: f32,
}

/// Draw a body row: as one text when it holds no emoji, else as its text runs
/// with each emoji picture between them, a run after a picture keeping the colour
/// the row had reached. Every run is written after the colour in force where it
/// starts ([`BodyRow::carry`] for the first), since each is drawn as its own text,
/// which restarts in the base colour.
fn body_row(
    ui: &mut MenuCanvas,
    row: BodyRow<'_>,
    font: &UiFont,
    rect: Rect,
    size: f32,
    color: Color,
) {
    let color = super::incoming_color::colour(color, row.forced);
    let carry = if row.forced.is_some() {
        Carry::NONE
    } else {
        row.carry
    };
    let ellipsis = if row.truncated { "..." } else { "" };
    let is_mark = |character: char| emoji::mark_index(character).is_some();
    if !row.text.contains(is_mark) {
        ui.text_fmt_aligned(
            format_args!(
                "{}{}{ellipsis}",
                carry,
                super::incoming_color::Body(row.text, row.forced.is_some())
            ),
            rect,
            size,
            color,
            FontWeight::Regular,
            row.spacing,
            TextAlign::Start,
        );
        return;
    }
    let mut x = rect.x;
    let mut colour = carry;
    let mut rest = row.text;
    loop {
        let (run, after) = rest.split_at(rest.find(is_mark).unwrap_or(rest.len()));
        let tail = if after.is_empty() { ellipsis } else { "" };
        if !run.is_empty() || !tail.is_empty() {
            ui.text_fmt_aligned(
                format_args!(
                    "{colour}{}{tail}",
                    super::incoming_color::Body(run, row.forced.is_some())
                ),
                Rect::new(x, rect.y, (rect.right() - x).max(1.0), rect.height),
                size,
                color,
                FontWeight::Regular,
                row.spacing,
                TextAlign::Start,
            );
            x += visible_text_width_style(
                font,
                run,
                size / font.height,
                TextFace::Regular,
                row.spacing,
            );
            if row.forced.is_none() {
                colour = colour.after(run);
            }
        }
        let Some(mark) = after.chars().next() else {
            break;
        };
        let picture = emoji::mark_index(mark)
            .and_then(|index| row.marks.get(index))
            .zip(row.emojis)
            .and_then(|(&emoji, emojis)| emojis.picture(emoji));
        if let Some((texture, uv)) = picture {
            let side = emoji::side(size);
            let _ = ui.draw_list_mut().push(DrawCommand::TexturedQuadUv {
                rect: Rect::new(x, rect.y + (size - side) * 0.5, side, side),
                texture,
                color: Color::new(1.0, 1.0, 1.0, color.a),
                uv,
            });
        }
        x += emoji::advance(size);
        rest = &after[mark.len_utf8()..];
    }
}

/// Height of the rectangle a body row is drawn and clipped in: its whole line box
/// and 1 px glyph shadow, whatever the row pitch.
fn row_box(g: &Geometry) -> f32 {
    g.row.max(g.font + 1.0)
}

/// Line box of centre-print text in 1080p pixels at `cg_centerSize` 1, the size
/// EternalJK draws it: `CG_DrawCenterString` (JoF EJK `cg_draw.c:6985`) draws
/// `ergoec` (`FONT_MEDIUM`) at scale `cg_centerSize`, and `ergoec`'s line is 22
/// retail pixels (`mHeight`), which SJK Menu keeps (`text::retail_font::MENU`): 49.5
/// px at 1080 lines, capitals about 29 px. Before 19/10/2026 it was 22 px. The
/// headline font draws it grown by [`crate::game_font::HEADLINE_GROWTH`], to the
/// same capitals.
pub(super) const CENTER_FONT_1080: f32 = 22.0 * 1080.0 / 480.0;

/// Pitch of centre-print rows in 1080p pixels for `size`: EternalJK advances
/// `CG_Text_Height` (`ergoec`'s point size 20 times the scale) plus 6 virtual pixels.
pub(super) fn center_row(size: f32) -> f32 {
    (20.0 * size + 6.0) * 1080.0 / 480.0
}

/// CG_DrawCenterString wraps rows longer than this at their last space ([BugFix19];
/// `BG_IsWhiteSpace` counts only the space, so a tab or vertical tab in a name
/// does not break the row).
/// Retail counts bytes of its one-byte code page, so the limit counts characters:
/// `×` or `é` in a name is one byte there but two in UTF-8.
const CENTER_WRAP_CHARS: usize = 50;

/// Rows of a centre print: explicit `\n` breaks, then whitespace wrapping of
/// over-long rows, without allocating.
pub(super) fn center_rows(text: &str) -> impl Iterator<Item = &str> {
    text.trim_end_matches('\n')
        .split('\n')
        .flat_map(|row| WrapRow { rest: row })
}

struct WrapRow<'a> {
    rest: &'a str,
}

impl<'a> Iterator for WrapRow<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        // Byte offset of the first character past the limit, if the row is longer.
        // Slicing at a fixed byte count panicked when it fell inside a character.
        let cut = match self.rest.char_indices().nth(CENTER_WRAP_CHARS) {
            Some((limit, _)) => self.rest[..limit]
                .char_indices()
                .rev()
                .find(|&(_, character)| character == ' ')
                .map_or(limit, |(index, _)| index + 1),
            None => self.rest.len(),
        };
        let (head, tail) = self.rest.split_at(cut);
        self.rest = tail.trim_start_matches(' ');
        Some(head.trim_end_matches(' '))
    }
}

#[cfg(test)]
mod tests {
    use super::center_rows;
    use super::{BodyRow, Carry, body_row, emoji};
    use crate::chat::ChatOverlay;
    use crate::menu_widgets::MenuCanvas;
    use crate::text::TextStyle;
    use sjk_client::ServerEventKind;
    use sjk_ui::{Color, DrawCommand, Rect};
    use std::time::Instant;

    #[test]
    fn the_clan_colours_are_none_of_the_colour_codes_nor_the_sjk_gold() {
        use crate::text::CodePalette;
        let distance = |a: Color, b: [f32; 3]| {
            ((a.r - b[0]).powi(2) + (a.g - b[1]).powi(2) + (a.b - b[2]).powi(2)).sqrt()
        };
        let gold = crate::sjk_chat_look::GOLD;
        for clan in [
            sjk_identity::HubChannel::Jof,
            sjk_identity::HubChannel::Sith,
        ] {
            let colour = super::clan_colour(clan, 1.0);
            for index in 0..=9 {
                for palette in [CodePalette::Game, CodePalette::Legible] {
                    let code = palette.colour(index);
                    assert!(
                        distance(colour, [code[0], code[1], code[2]]) > 0.25,
                        "{clan:?} ^{index} {palette:?}"
                    );
                }
            }
            assert!(distance(colour, [gold.r, gold.g, gold.b]) > 0.3, "{clan:?}");
            let pink = super::encrypted_pink(1.0);
            assert!(distance(colour, [pink.r, pink.g, pink.b]) > 0.2, "{clan:?}");
        }
    }

    #[test]
    fn scoreboard_chat_keeps_the_full_font_scale() {
        let viewport = [1920.0, 1080.0];
        let mut geometry = crate::chat::options::Options::default().geometry(viewport);
        super::place_beside_scoreboard(&mut geometry, viewport, 3.0);
        assert_eq!(geometry.scale, 3.0);
        assert_eq!(geometry.font, 3.0 * crate::chat::layout::FONT_1080);
        assert_eq!(geometry.row, geometry.font * crate::chat::layout::ROW_PITCH);
    }

    /// The texts and emoji rectangles drawn for `body` with `names` loaded.
    fn drawn(names: &[&str], body: &str, truncated: bool) -> (Vec<(f32, String)>, Vec<Rect>) {
        drawn_shown(names, body, truncated, true)
    }

    fn drawn_shown(
        names: &[&str],
        body: &str,
        truncated: bool,
        shown: bool,
    ) -> (Vec<(f32, String)>, Vec<Rect>) {
        drawn_after(names, body, truncated, shown, Carry::NONE)
    }

    /// As [`drawn_shown`], for a row that starts in the colour `carry` stands for.
    fn drawn_after(
        names: &[&str],
        body: &str,
        truncated: bool,
        shown: bool,
        carry: Carry,
    ) -> (Vec<(f32, String)>, Vec<Rect>) {
        let emojis = emoji::Emojis::from_names(names);
        let (text, marks) = emojis.markup(body);
        let mut ui = MenuCanvas::with_text_capacity(512);
        ui.begin_transparent([640.0, 480.0]);
        let font = crate::text::test_font();
        body_row(
            &mut ui,
            BodyRow {
                forced: None,
                text: &text,
                carry,
                marks: &marks,
                emojis: shown.then_some(&emojis),
                truncated,
                spacing: 0.0,
            },
            &font,
            Rect::new(10.0, 20.0, 400.0, 13.0),
            12.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        let mut texts = Vec::new();
        let mut pictures = Vec::new();
        for command in ui.draw_list().commands() {
            match command {
                DrawCommand::Text { rect, text, .. } => {
                    texts.push((rect.x, ui.stored_text(*text).to_owned()));
                }
                DrawCommand::TexturedQuadUv { rect, .. } => pictures.push(*rect),
                _ => {}
            }
        }
        (texts, pictures)
    }

    #[test]
    fn a_row_without_emojis_is_one_text() {
        let (texts, pictures) = drawn(&[":x:"], "^2plain", true);
        assert_eq!(texts, [(10.0, "^2plain...".to_owned())]);
        assert!(pictures.is_empty());
    }

    #[test]
    fn an_emoji_sits_between_its_text_runs_and_the_colour_carries_on() {
        // Test glyphs advance 8 units at height 12, so "^1hi " is 24 wide.
        let (texts, pictures) = drawn(&[":x:"], "^1hi :x: there", false);
        let after = 10.0 + 24.0 + emoji::advance(12.0);
        assert_eq!(
            texts,
            [(10.0, "^1hi ".to_owned()), (after, "^1 there".to_owned())]
        );
        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0].x, 34.0);
        assert_eq!(pictures[0].width, emoji::side(12.0));
    }

    #[test]
    fn a_row_after_a_break_starts_in_the_colour_the_row_before_ended_in() {
        let green = Carry::NONE.after("^2first row");
        let (texts, _) = drawn_after(&[], "second row", false, true, green);
        assert_eq!(texts, [(10.0, "^2second row".to_owned())]);
        // The ellipsis of a cut last row is in that colour too.
        let (texts, _) = drawn_after(&[], "last", true, true, green);
        assert_eq!(texts, [(10.0, "^2last...".to_owned())]);
        // A row with no code before it is drawn as it is.
        let (texts, _) = drawn_after(&[], "second row", false, true, Carry::NONE);
        assert_eq!(texts, [(10.0, "second row".to_owned())]);
    }

    #[test]
    fn a_wrapped_row_with_an_emoji_goes_on_in_the_carried_colour() {
        let red = Carry::NONE.after("^1");
        let (texts, pictures) = drawn_after(&[":x:"], "hi :x: ^3there :x: you", false, true, red);
        assert_eq!(pictures.len(), 2);
        // The first run starts in the colour the row before ended in, and each run
        // after a picture in the colour the run before it reached.
        let runs: Vec<_> = texts.iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(runs, ["^1hi ", "^1 ^3there ", "^3 you"]);
    }

    /// The rows the feed draws for chat `text` from an unknown sender, in a
    /// 1920 x 1080 frame, and the colours of the glyphs they put on screen.
    fn feed(text: &str) -> (Vec<String>, Vec<[f32; 3]>) {
        let font = crate::text::test_font();
        let viewport = [1920.0, 1080.0];
        let mut chat = ChatOverlay::new();
        chat.receive(ServerEventKind::Chat, text.to_owned(), None, Instant::now());
        chat.build(true, &font, viewport, 1_000);
        let rows = chat.ui.text_runs().map(str::to_owned).collect();
        let mut vertices = Vec::new();
        chat.ui
            .append_text_styled(&mut vertices, &font, viewport, TextStyle::NEUTRAL);
        // Each glyph is a shadow quad then its own, six vertices each; the shadows
        // are black.
        let glyphs = vertices
            .iter()
            .map(|vertex| vertex.colour())
            .filter(|colour| colour[..3] != [0.0, 0.0, 0.0])
            .map(|[red, green, blue, _]| [red, green, blue])
            .collect();
        (rows, glyphs)
    }

    #[test]
    fn a_long_message_is_green_on_every_row_it_is_drawn_on() {
        // `^2` leads the text of a stock `say`; this is long enough for three rows.
        let words = "the quick brown fox jumps over the lazy dog ".repeat(5);
        let (rows, glyphs) = feed(&format!("^2{}", words.trim_end()));
        assert!(rows.len() >= 2, "{rows:?}");
        for row in &rows {
            assert!(row.starts_with("^2"), "{row:?}");
        }
        // Six vertices a glyph quad; every glyph drawn is pure green.
        let visible: usize = rows
            .iter()
            .map(|row| crate::text::Plain(row).to_string().len())
            .sum();
        assert_eq!(glyphs.len(), visible * 6);
        assert!(glyphs.iter().all(|colour| *colour == [0.0, 1.0, 0.0]));
    }

    #[test]
    fn a_message_without_codes_is_in_one_colour_on_every_row() {
        let words = "no codes here at all but a rather long message ".repeat(5);
        let (rows, glyphs) = feed(words.trim_end());
        assert!(rows.len() >= 2, "{rows:?}");
        assert!(rows.iter().all(|row| !row.contains('^')), "{rows:?}");
        let first = glyphs[0];
        assert_ne!(first, [0.0, 1.0, 0.0]);
        assert!(glyphs.iter().all(|colour| *colour == first));
    }

    /// `cg_chatBoxOutline` edges the feed's and the field's letters in a dark outline,
    /// eight copies round each in place of its shadow; the centre print keeps its
    /// shadow, and with the setting off (the default) every letter keeps it.
    #[test]
    fn the_outline_replaces_the_shadow_in_the_feed_and_the_field_only() {
        use crate::text::CodePalette;
        let font = crate::text::test_font();
        let viewport = [1920.0, 1080.0];
        let built = |outline: bool| {
            let mut chat = ChatOverlay::new();
            chat.options.outline = outline;
            let now = Instant::now();
            chat.receive(ServerEventKind::Chat, "plain words".to_owned(), None, now);
            chat.receive(ServerEventKind::CenterPrint, "centre".to_owned(), None, now);
            chat.draft_for_shot("hi");
            chat.build(true, &font, viewport, 1_000);
            let palette_of = chat.palette_of();
            let palettes: Vec<(String, CodePalette)> = chat
                .ui
                .draw_list()
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => {
                        Some((chat.ui.stored_text(*text).to_owned(), palette_of(text.0)))
                    }
                    _ => None,
                })
                .collect();
            let mut vertices = Vec::new();
            chat.append_built(
                &mut crate::game_font::GameFonts::default(),
                &mut vertices,
                &font,
                viewport,
            );
            // Quads of each colour: six vertices a quad.
            let quads = |rgb: [f32; 3]| {
                vertices
                    .iter()
                    .filter(|vertex| vertex.colour()[..3] == rgb)
                    .count()
                    / 6
            };
            let shadows = quads([0.0, 0.0, 0.0]);
            let rings = quads([0.04, 0.04, 0.06]);
            (palettes, vertices.len() / 6, shadows, rings)
        };
        let palette = |palettes: &[(String, CodePalette)], text: &str| {
            palettes
                .iter()
                .find(|(run, _)| run == text)
                .unwrap_or_else(|| panic!("{text:?} in {palettes:?}"))
                .1
        };
        let (palettes, all, shadows, rings) = built(false);
        assert!(palettes.iter().all(|(_, p)| *p == CodePalette::Game));
        assert_eq!((shadows * 2, rings), (all, 0), "a shadow a letter");
        let (palettes, _, outlined_shadows, rings) = built(true);
        for run in ["plain words", "hi"] {
            assert_eq!(palette(&palettes, run), CodePalette::DarkOutlined, "{run}");
        }
        assert_eq!(palette(&palettes, "centre"), CodePalette::Game);
        // Each outlined letter traded its shadow for eight ring copies.
        assert_eq!(rings % 8, 0);
        let outlined = rings / 8;
        assert!(outlined >= "plainwordshi".len(), "{outlined}");
        assert_eq!(outlined_shadows + outlined, shadows);
        assert!(outlined_shadows >= "centre".len(), "the centre print's");
    }

    #[test]
    fn switched_off_the_pictures_leave_their_blanks() {
        let (texts, pictures) = drawn_shown(&[":x:"], "hi :x: there", false, false);
        assert!(pictures.is_empty());
        assert_eq!(texts[1].0, 10.0 + 24.0 + emoji::advance(12.0));
    }

    #[test]
    fn a_row_ending_in_an_emoji_still_shows_the_ellipsis() {
        let (texts, pictures) = drawn(&[":x:"], "a:x:", true);
        assert_eq!(pictures.len(), 1);
        assert_eq!(texts.last().map(|(_, text)| text.as_str()), Some("..."));
    }

    #[test]
    fn center_rows_wrap_long_rows_at_a_space() {
        let text = format!("{} {}", "a".repeat(30), "b".repeat(30));
        let rows: Vec<_> = center_rows(&text).collect();
        assert_eq!(rows, ["a".repeat(30), "b".repeat(30)]);
    }

    #[test]
    fn center_rows_do_not_split_a_character_at_the_limit() {
        // 49 ASCII bytes put `×` across UTF-8 bytes 49..51: a byte limit of 50
        // sliced inside it and crashed on a duel challenge from such a name.
        let name = format!("{}×", "x".repeat(49));
        let text = format!("{name} has challenged you to a duel!");
        let rows: Vec<_> = center_rows(&text).collect();
        assert_eq!(rows, [name.as_str(), "has challenged you to a duel!"]);
    }

    #[test]
    fn center_rows_break_only_at_a_space() {
        // A name with vertical tabs (0x0B) stays on the row after the last space.
        let text = "You have challenged ^6{^0JoF^6}^7\u{b}Toxiee\u{b}^6{^0C^6}.ak";
        assert_eq!(
            center_rows(text).collect::<Vec<_>>(),
            [
                "You have challenged",
                "^6{^0JoF^6}^7\u{b}Toxiee\u{b}^6{^0C^6}.ak"
            ]
        );
    }

    #[test]
    fn center_rows_count_characters_as_retail_counts_bytes() {
        // Fifty two-byte characters fit one retail row of fifty bytes.
        let row = "é".repeat(50);
        assert_eq!(center_rows(&row).collect::<Vec<_>>(), [row.as_str()]);
        let longer = "é".repeat(51);
        assert_eq!(
            center_rows(&longer).collect::<Vec<_>>(),
            ["é".repeat(50), "é".to_owned()]
        );
    }

    #[test]
    fn a_whisper_that_is_not_encrypted_says_so_under_the_field() {
        use super::composer::PLAIN_WHISPER_NOTE;
        let mut chat = ChatOverlay::new();
        let mut game = sjk_protocol::GameState::empty_local(0);
        for slot in [3_usize, 4] {
            game.replace_config_string(1_131 + slot, format!("n\\P{slot}\\t\\0").into_bytes())
                .expect("a player");
        }
        chat.update_roster(&game);
        chat.set_encrypted_slots(1 << 4);
        let shown = |chat: &mut ChatOverlay| {
            chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
            chat.ui.text_runs().any(|text| text == PLAIN_WHISPER_NOTE)
        };
        assert!(chat.whisper_to(Some(3)));
        assert!(shown(&mut chat), "to a player without encrypted whispers");
        chat.close_composer();
        assert!(chat.whisper_to(Some(4)));
        assert!(!shown(&mut chat), "encrypted");
        chat.close_composer();
        chat.open(false);
        assert!(!shown(&mut chat), "not a whisper");
    }

    #[test]
    fn our_encrypted_whisper_wears_the_recipients_marks() {
        use crate::chat::marks::SlotMark;
        let mut chat = ChatOverlay::new();
        let now = std::time::Instant::now();
        assert!(chat.receive_encrypted(None, (4, "Creyon"), "hi", false, now));
        let mut slots = vec![SlotMark::default(); crate::chat_mutes::SLOTS];
        let line = chat.lines.back().expect("the line");
        let (name, marks) = super::line_marks(line, &slots).expect("marks");
        assert_eq!((name, marks.label), ("To Creyon", "ENCRYPTED"));
        assert!(
            marks.mark.is_none() && marks.emoji.is_empty(),
            "not known yet"
        );
        slots[4] = SlotMark {
            sjk: true,
            verified: true,
            emoji: "x".to_owned(),
            ..Default::default()
        };
        let (_, marks) = super::line_marks(line, &slots).expect("marks");
        assert!(marks.sjk && marks.mark.is_some());
        assert_eq!(marks.emoji, "x");
    }

    /// A centre print in the headline font (Fira Sans Condensed), rendered off screen
    /// at 1080p and 4K, with `ui_gameFont` on and off (it draws the same either way).
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn centre_print_shot() {
        for (size, suffix) in [([1920, 1080], "1080p"), ([3840, 2160], "4k")] {
            for game_font in ["1", "0"] {
                crate::world_shot::on_big_stack(move || {
                    let Some((mut gpu, _profile)) = crate::world_shot::open(
                        "maps/mp/duel6.bsp",
                        size,
                        None,
                        &[(crate::game_font::CVAR, game_font), ("cg_centertime", "60")],
                    ) else {
                        return;
                    };
                    gpu.chat.for_shot = true;
                    gpu.chat.receive(
                        ServerEventKind::CenterPrint,
                        "^1You are the mark!\n^7Hunters: Solol, dart_mario 0123456789".to_owned(),
                        None,
                        Instant::now(),
                    );
                    let name = format!("centre-print-{suffix}-gamefont{game_font}");
                    let path = crate::world_shot::shoot(&mut gpu, 10, &name);
                    println!("{}", path.display());
                });
            }
        }
    }
}
