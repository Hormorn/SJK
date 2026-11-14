//! A line with a sender in the feed (`docs/hub-chat.md`, "How a line looks"): an SJK
//! chat line, or a game chat line whose sender is known. One flowing line, never a
//! name row with the message under it. Its first row starts with SJK's emblem, small, for a sender playing on SJK (the logo, in its own
//! colours, Sol's request 11/10/2026: every SJK chat sender, and since 16/10/2026 a
//! game line's sender the hub knows in that slot), the clan emblem for a sender with a
//! known clan's tag ([`crate::jof_tag`]), the sender's name in its own colour codes,
//! the verified tick alone for a verified sender and, on a game line, its channel
//! (TEAM, WHISPER); the message follows on the same row and wraps only when it is too
//! long ([`layout::Wrapped::update_indented`]). The two kinds differ only in the
//! message's colour (Sol's request, 16/10/2026): the SJK chat's gold
//! ([`crate::sjk_chat_look::GOLD`]), or the game line's own colour codes over white.
//!
//! A holocron drop or grand find starts with the holocron's or crystal's picture in
//! place of the emblem; a find of a blade skin then draws that blade, small and live
//! ([`BLADE_LENGTH`]), and its prize's words in the prize's own rarity colour
//! (01/11/2026, Sol's request; `chat/find_blade.rs` shows the blade large).

use super::*;
use crate::holocrons::Tier;
use crate::sjk_chat_look;

/// The SJK emblem's side before an SJK line's name, against the text size: the
/// starburst's spikes reach the box's edges, its medallion is about half of it.
const LOGO_SIDE: f32 = 1.35;
/// The channel label's size against the text size (10 pixels beside 16, as the name
/// rows' labels were).
const LABEL_SIZE: f32 = 0.625;
/// A find's small blade against the text size: its length and its height, and the gap
/// after it.
const BLADE_LENGTH: f32 = 3.6;
const BLADE_HEIGHT: f32 = 0.95;
const BLADE_GAP: f32 = 0.35;

/// What a muted sender's line says in place of their message.
pub(in crate::chat) const MUTED_NOTE: &str = "Messages hidden on this client";

/// The size [`MUTED_NOTE`] is drawn at in geometry `g`.
pub(in crate::chat) fn muted_size(g: &Geometry) -> f32 {
    15.0 * g.scale
}

/// What the first row shows of a line's sender.
#[derive(Clone, Copy, Default)]
pub(in crate::chat) struct Marks<'a> {
    /// The sender plays on SJK: SJK's emblem comes first.
    pub(in crate::chat) sjk: bool,
    /// The message was sent from another client with its own icon
    /// (`chat/client_icons.rs`), which takes the emblem's place.
    pub(in crate::chat) icon: Option<&'static crate::chat::client_icons::ClientIcon>,
    /// The mark after the name: the verified seal or a trust mark
    /// (`ui_renderer::mark_texture`).
    pub(in crate::chat) mark: Option<sjk_ui::TextureId>,
    /// The line is a holocron drop of this tier: its mark is the tier's gem.
    pub(in crate::chat) tier: Option<&'static Tier>,
    /// The channel written after the name, empty for All and the SJK chat.
    pub(in crate::chat) label: &'static str,
    /// The sender's name emoji (`sjk_identity::name_emoji`), right after the name;
    /// empty for none.
    pub(in crate::chat) emoji: &'a str,
    /// The SJK team's line: the name in the SJK UI's accent gold.
    pub(in crate::chat) accent: bool,
    /// A grand find of this blade skin: it is drawn after the holocron's picture.
    pub(in crate::chat) blade: Option<&'static crate::unlockables::Unlockable>,
}

/// What a line's first row holds before its message, measured in the feed's font.
/// Nothing is stored: it is measured where it is drawn.
pub(in crate::chat) struct Prefix<'a> {
    /// The body's text size, which the name and the colon share.
    size: f32,
    /// The SJK emblem's side, 0 without one.
    logo: f32,
    /// The emblem drawn: SJK's, or JoF EternalJK's icon for a message sent from it.
    logo_texture: sjk_ui::TextureId,
    /// Where the shown name ends in the line's name (cut to half the feed).
    pub(in crate::chat) name_end: usize,
    name_width: f32,
    mark: Option<sjk_ui::TextureId>,
    /// The name emoji after the name, and its room with its gap.
    emoji: &'a str,
    emoji_width: f32,
    /// The name carries a known clan's tag: the clan's emblem comes before it.
    jof: Option<crate::jof_tag::Clan>,
    label: &'static str,
    /// The label's room, its gap before it included.
    label_width: f32,
    colon_width: f32,
    /// The line is a holocron drop: its mark is the tier's gem, not the verified tick.
    tier: Option<&'static Tier>,
    /// The name in the SJK UI's accent gold ([`Marks::accent`]).
    accent: bool,
    /// A find's blade ([`Marks::blade`]) and the room it takes, its gap included.
    pub(in crate::chat) blade: Option<&'static crate::unlockables::Unlockable>,
    blade_width: f32,
}

impl<'a> Prefix<'a> {
    pub(in crate::chat) fn new(name: &str, marks: Marks<'a>, font: &UiFont, g: &Geometry) -> Self {
        let size = g.font;
        let scale = size / font.height.max(1.0);
        let name_end = layout::fitting_end(name, font, g.width * 0.5, size);
        let label_size = size * LABEL_SIZE;
        Self {
            size,
            logo: if marks.sjk { size * LOGO_SIDE } else { 0.0 },
            logo_texture: crate::chat::client_icons::texture(marks.icon),
            name_end,
            name_width: visible_text_width_face(font, &name[..name_end], scale, TextFace::Semibold),
            mark: marks.mark,
            emoji: marks.emoji,
            emoji_width: sjk_chat_look::emoji_room(font, marks.emoji, size),
            tier: marks.tier,
            accent: marks.accent,
            blade: marks.blade,
            blade_width: if marks.blade.is_some() {
                size * (BLADE_LENGTH + BLADE_GAP)
            } else {
                0.0
            },
            jof: crate::jof_tag::clan(name),
            label: marks.label,
            label_width: if marks.label.is_empty() {
                0.0
            } else {
                size * 0.375
                    + visible_text_width_style(
                        font,
                        marks.label,
                        label_size / font.height.max(1.0),
                        TextFace::Semibold,
                        label_size * 0.1,
                    )
            },
            colon_width: if name.is_empty() {
                0.0
            } else {
                visible_text_width_face(font, ": ", scale, TextFace::Regular)
            },
        }
    }

    /// Where the clan emblem's ink starts after the SJK emblem.
    fn jof_x(&self) -> f32 {
        if self.logo > 0.0 {
            self.logo + self.size * 0.2
        } else {
            0.0
        }
    }

    /// Where the name starts after the emblems.
    fn name_x(&self) -> f32 {
        self.jof_x()
            + if let Some(clan) = self.jof {
                crate::jof_tag::room(clan, crate::jof_tag::side(self.size))
            } else {
                0.0
            }
    }

    /// Where the name ends with its emoji, where the tick goes.
    fn tick_x(&self) -> f32 {
        self.name_x() + self.name_width + self.emoji_width
    }

    /// Where a find's blade starts after the name, its emoji and its tick.
    fn blade_x(&self) -> f32 {
        self.tick_x()
            + if self.mark.is_some() || self.tier.is_some() {
                sjk_chat_look::tick_room(self.size)
            } else {
                0.0
            }
    }

    /// Where the channel's label (its gap first) starts after the name, its emoji, its
    /// tick and a find's blade.
    fn label_x(&self) -> f32 {
        self.blade_x() + self.blade_width
    }

    /// Where a find's blade is drawn on a line whose first row's top-left is `origin`.
    pub(in crate::chat) fn blade_rect(&self, font: &UiFont, origin: [f32; 2]) -> Option<Rect> {
        self.blade?;
        let middle = origin[1] + font.capital_middle(self.size / font.height.max(1.0));
        let height = self.size * BLADE_HEIGHT;
        Some(Rect::new(
            origin[0] + self.blade_x() + self.size * 0.1,
            middle - height * 0.5,
            self.size * BLADE_LENGTH,
            height,
        ))
    }

    /// Where the colon starts after the name, its tick and the label.
    fn colon_x(&self) -> f32 {
        self.label_x() + self.label_width
    }

    /// The room before the message on the first row.
    pub(in crate::chat) fn width(&self) -> f32 {
        self.colon_x() + self.colon_width
    }
}

/// The parts of one line to draw.
pub(in crate::chat) struct Line<'a> {
    pub(in crate::chat) name: &'a str,
    pub(in crate::chat) body: &'a str,
    pub(in crate::chat) muted: bool,
    pub(in crate::chat) wrap: &'a layout::Wrapped,
    pub(in crate::chat) marks: &'a [u16],
    /// The emoji pictures, while `cg_chatBoxEmojis` is on.
    pub(in crate::chat) emojis: Option<&'a emoji::Emojis>,
    /// The pointer rests on the name (its profile card shows): it is lit.
    pub(in crate::chat) hovered: bool,
    /// The message's colour: the SJK chat's gold, a holocron tier's, or a game line's
    /// white under its own colour codes.
    pub(in crate::chat) colour: Color,
    pub(in crate::chat) forced: Option<Color>,
    /// The channel label's colour (`view::tint`).
    pub(in crate::chat) label_colour: Color,
    /// A find's prize: where its words are in the body, drawn in its own colour.
    pub(in crate::chat) prize: Option<(std::ops::Range<usize>, Color)>,
}

/// Draw `line` with its first row's top-left at `origin`, at `alpha`; returns the
/// shown name's rectangle.
pub(in crate::chat) fn draw(
    ui: &mut MenuCanvas,
    line: Line<'_>,
    prefix: &Prefix<'_>,
    font: &UiFont,
    g: &Geometry,
    origin: [f32; 2],
    alpha: f32,
) -> Rect {
    let [x, y] = origin;
    let size = prefix.size;
    let middle = y + font.capital_middle(size / font.height.max(1.0));
    if prefix.logo > 0.0 {
        let _ = ui.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: Rect::new(x, middle - prefix.logo * 0.5, prefix.logo, prefix.logo),
            texture: prefix.logo_texture,
            color: Color::new(1.0, 1.0, 1.0, alpha),
        });
    }
    if let Some(clan) = prefix.jof {
        crate::jof_tag::draw(
            clan,
            x + prefix.jof_x(),
            middle,
            crate::jof_tag::side(size),
            alpha,
            |command| {
                let _ = ui.draw_list_mut().push(command);
            },
        );
    }
    let ink = Color::new(0.982, 0.987, 0.996, alpha);
    let name = &line.name[..prefix.name_end];
    let name_x = x + prefix.name_x();
    let name_rect = name::ink_bounds(font, name, [name_x, y], size);
    if line.hovered {
        // The name under the pointer, whose card shows.
        ui.accent_bar(name_rect, Color::new(0.70, 0.88, 0.98, 0.16));
    }
    if !name.is_empty() {
        let gold = crate::menu::sjk::color::GOLD;
        ui.text(
            name,
            Rect::new(name_x, y, prefix.name_width + 1.0, row_box(g)),
            size,
            if prefix.accent {
                Color::new(gold.r, gold.g, gold.b, alpha)
            } else {
                ink
            },
            FontWeight::Semibold,
            0.0,
        );
    }
    if !prefix.emoji.is_empty() {
        let left = name_x + prefix.name_width + sjk_chat_look::emoji_gap(size);
        ui.text(
            prefix.emoji,
            Rect::new(left, y, prefix.emoji_width + 1.0, row_box(g)),
            size,
            Color::new(1.0, 1.0, 1.0, alpha),
            FontWeight::Regular,
            0.0,
        );
    }
    let tick_x = x + prefix.tick_x();
    if let Some(tier) = prefix.tier {
        sjk_chat_look::gem_mark(ui.draw_list_mut(), tick_x, middle, size, tier, alpha);
    } else if let Some(mark) = prefix.mark {
        sjk_chat_look::tick(ui.draw_list_mut(), mark, tick_x, middle, size, alpha);
    }
    if let (Some(skin), Some(rect)) = (prefix.blade, prefix.blade_rect(font, origin)) {
        super::super::find_blade::blade(ui.draw_list_mut(), skin, rect, alpha);
    }
    if !prefix.label.is_empty() {
        let label_size = size * LABEL_SIZE;
        ui.text(
            prefix.label,
            Rect::new(
                x + prefix.label_x() + size * 0.375,
                middle - font.capital_middle(label_size / font.height.max(1.0)),
                prefix.label_width + 1.0,
                row_box(g),
            ),
            label_size,
            line.label_colour,
            FontWeight::Semibold,
            label_size * 0.1,
        );
    }
    if prefix.colon_width > 0.0 {
        ui.text(
            ":",
            Rect::new(x + prefix.colon_x(), y, prefix.colon_width, row_box(g)),
            size,
            ink,
            FontWeight::Regular,
            0.0,
        );
    }
    let indent = prefix.width();
    if line.muted {
        ui.text(
            MUTED_NOTE,
            Rect::new(x + indent, y, (g.width - indent).max(1.0), row_box(g)),
            muted_size(g),
            Color::new(0.821, 0.854, 0.886, alpha * 0.853),
            FontWeight::Regular,
            0.0,
        );
        return name_rect;
    }
    for (row, range) in line.wrap.rows[..line.wrap.len].iter().enumerate() {
        if range.is_empty() {
            continue;
        }
        let left = if row == 0 { x + indent } else { x };
        let truncated = row + 1 == line.wrap.len && range.end < line.body.len();
        let rect = Rect::new(
            left,
            y + row as f32 * g.row,
            (x + g.width - left).max(1.0),
            row_box(g),
        );
        if let Some((prize, colour)) = &line.prize {
            prize_row(
                ui,
                line.body,
                range.clone(),
                (prize.clone(), *colour),
                truncated,
                font,
                g,
                rect,
                line.colour,
            );
            continue;
        }
        body_row(
            ui,
            BodyRow {
                forced: line.forced,
                text: &line.body[range.clone()],
                carry: line.wrap.carry[row],
                marks: line.marks,
                emojis: line.emojis,
                truncated,
                spacing: g.spacing,
            },
            font,
            Rect::new(
                left,
                y + row as f32 * g.row,
                (x + g.width - left).max(1.0),
                row_box(g),
            ),
            g.font,
            line.colour,
        );
    }
    name_rect
}

/// Draw the row `range` of a find's `body` (plain words: no colour codes, no emoji) in
/// `colour`, with the part of the prize's words it holds in the prize's own colour.
#[allow(clippy::too_many_arguments)]
fn prize_row(
    ui: &mut MenuCanvas,
    body: &str,
    range: std::ops::Range<usize>,
    (prize, prize_colour): (std::ops::Range<usize>, Color),
    truncated: bool,
    font: &UiFont,
    g: &Geometry,
    rect: Rect,
    colour: Color,
) {
    let start = prize.start.clamp(range.start, range.end);
    let end = prize.end.clamp(range.start, range.end);
    let prize_colour = Color::new(prize_colour.r, prize_colour.g, prize_colour.b, colour.a);
    let mut x = rect.x;
    let parts = [
        (range.start..start, colour),
        (start..end, prize_colour),
        (end..range.end, colour),
    ];
    let last = parts.iter().rposition(|(part, _)| !part.is_empty());
    for (index, (part, ink)) in parts.into_iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let words = &body[part];
        let ellipsis = if truncated && Some(index) == last {
            "..."
        } else {
            ""
        };
        ui.text_fmt_aligned(
            format_args!("{words}{ellipsis}"),
            Rect::new(x, rect.y, (rect.right() - x).max(1.0), rect.height),
            g.font,
            ink,
            FontWeight::Regular,
            g.spacing,
            TextAlign::Start,
        );
        x += visible_text_width_style(
            font,
            words,
            g.font / font.height.max(1.0),
            TextFace::Regular,
            g.spacing,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::chat::ChatOverlay;
    use crate::sjk_chat_look::GOLD;
    use sjk_client::ServerEventKind;
    use sjk_identity::{ChatMessage, ChatState};
    use sjk_ui::{Color, DrawCommand, Rect};
    use std::time::Instant;

    const VIEWPORT: [f32; 2] = [1920.0, 1080.0];

    fn message(id: u64, name: &str, text: &str, verified: bool) -> ChatMessage {
        ChatMessage {
            reactions: Default::default(),
            id,
            at: 0,
            key_id: format!("{id:016x}"),
            name: name.to_owned(),
            verified,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: text.to_owned(),
            holocron: None,
            official: None,
            channel: Default::default(),
        }
    }

    /// A feed holding `messages` from the hub after `server` lines from the game.
    fn feed(server: &[&str], messages: Vec<ChatMessage>) -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let state = |messages| ChatState {
            messages,
            loaded: Some(1),
            ..ChatState::default()
        };
        chat.sync_sjk(&state(Default::default()), |_| false, now);
        for line in server {
            chat.receive(ServerEventKind::Chat, (*line).to_owned(), None, now);
        }
        chat.sync_sjk(&state(messages.into_iter().collect()), |_| false, now);
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        chat
    }

    /// Every text drawn, with its rectangle and colour.
    fn texts(chat: &ChatOverlay) -> Vec<(String, Rect, Color)> {
        chat.ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    rect, text, color, ..
                } => Some((chat.ui.stored_text(*text).to_owned(), *rect, *color)),
                _ => None,
            })
            .collect()
    }

    fn drawn<'a>(texts: &'a [(String, Rect, Color)], text: &str) -> &'a (String, Rect, Color) {
        texts
            .iter()
            .find(|(drawn, ..)| drawn == text)
            .unwrap_or_else(|| panic!("{text:?} not in {texts:?}"))
    }

    fn ticks(chat: &ChatOverlay) -> Vec<Rect> {
        quads(chat, crate::ui_renderer::VERIFIED_TEXTURE)
    }

    /// The SJK emblems before the lines.
    fn logos(chat: &ChatOverlay) -> Vec<Rect> {
        quads(chat, crate::ui_renderer::LOGO_TEXTURE)
    }

    fn quads(chat: &ChatOverlay, wanted: sjk_ui::TextureId) -> Vec<Rect> {
        chat.ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::TexturedQuad { rect, texture, .. } if *texture == wanted => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect()
    }

    fn rgb(color: Color) -> [f32; 3] {
        [color.r, color.g, color.b]
    }

    #[test]
    fn incoming_colour_changes_displayed_history_but_not_names_or_stored_text() {
        let mut chat = feed(
            &["^1server text"],
            vec![message(1, "^3Sol", "hello", false)],
        );
        let stored = chat
            .lines
            .iter()
            .map(|l| l.body.clone())
            .collect::<Vec<_>>();
        chat.options.incoming = Some(Color::new(0.0, 1.0, 0.0, 1.0));
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        let shown = texts(&chat);
        drawn(&shown, "^3Sol");
        let (_, _, colour) = drawn(&shown, "server text");
        assert_eq!(rgb(*colour), [0.0, 1.0, 0.0]);
        assert_eq!(
            chat.lines
                .iter()
                .map(|l| l.body.clone())
                .collect::<Vec<_>>(),
            stored
        );
        chat.options.incoming = None;
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        assert_eq!(rgb(drawn(&texts(&chat), "hello").2), rgb(GOLD));
    }

    #[test]
    fn incoming_colour_preserves_special_channels_and_notice_colours() {
        use crate::chat::Channel;
        let mut chat = feed(
            &["^5public"],
            (1..=8)
                .map(|id| message(id, "Fox", &format!("^5body{id}"), false))
                .collect(),
        );
        chat.options.lines = crate::chat::MAX_VISIBLE;
        for (index, line) in chat.lines.iter_mut().skip(1).enumerate() {
            line.body = format!("^5body{}", index + 1);
            line.wrap = Default::default();
        }
        chat.lines[1].channel = Channel::Team;
        chat.lines[2].channel = Channel::Whisper;
        chat.lines[3].channel = Channel::Admin;
        chat.lines[4].channel = Channel::Global;
        chat.lines[4].encrypted = true;
        chat.lines[5].channel = Channel::Global;
        chat.lines[5].server_private = true;
        chat.lines[6].hub.as_mut().unwrap().official = true;
        chat.lines[7].hub.as_mut().unwrap().tier = Some(&crate::holocrons::TIERS[0]);
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        let before = texts(&chat);
        chat.options.incoming = Some(Color::new(0.0, 1.0, 0.0, 1.0));
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        let after = texts(&chat);
        assert_eq!(rgb(drawn(&after, "public").2), [0.0, 1.0, 0.0]);
        for id in 1..=8 {
            let body = format!("^5body{id}");
            assert_eq!(
                rgb(drawn(&before, &body).2),
                rgb(drawn(&after, &body).2),
                "{body}"
            );
        }
    }

    #[test]
    fn an_unverified_sender_shows_their_trust_mark_where_the_tick_goes() {
        use crate::ui_renderer::{VERIFIED_TEXTURE, trust_texture};
        use sjk_identity::Trust;
        let trust = |id, trust| ChatMessage {
            reactions: Default::default(),
            trust: Some(trust),
            ..message(id, "^2Fox", "hi", false)
        };
        let chat = feed(
            &[],
            vec![
                trust(1, Trust::New),
                trust(2, Trust::Trusted),
                trust(3, Trust::Banned),
                message(4, "^2Sol", "hello", true),
                message(5, "^2Old", "from an older hub", false),
            ],
        );
        for (texture, what) in [
            (trust_texture(Trust::New), "new"),
            (trust_texture(Trust::Trusted), "trusted"),
            (trust_texture(Trust::Banned), "banned"),
            (Some(VERIFIED_TEXTURE), "verified"),
        ] {
            assert_eq!(quads(&chat, texture.unwrap()).len(), 1, "{what}");
        }
        assert_eq!(trust_texture(Trust::Unknown), None);
    }

    #[test]
    fn only_sjk_messages_are_gold() {
        let chat = feed(
            &["^7Fox^7: ^2a game line", "plain game words"],
            vec![message(1, "^2Sol", "hello ^1there", false)],
        );
        let texts = texts(&chat);
        let (_, _, colour) = drawn(&texts, "hello there");
        assert_eq!(rgb(*colour), rgb(GOLD));
        for line in ["^7Fox^7: ^2a game line", "plain game words"] {
            let (_, _, colour) = drawn(&texts, line);
            assert_ne!(rgb(*colour), rgb(GOLD), "{line}");
        }
        // The name keeps its own colour codes, over the game's white, not gold.
        let (_, _, colour) = drawn(&texts, "^2Sol");
        assert_ne!(rgb(*colour), rgb(GOLD));
    }

    #[test]
    fn a_short_sjk_line_is_one_row_with_its_name() {
        let chat = feed(&[], vec![message(1, "^2Sol", "gg all", true)]);
        assert_eq!(chat.lines[0].wrap.len, 1);
        let texts = texts(&chat);
        let tag = logos(&chat)[0];
        let (_, name, _) = drawn(&texts, "^2Sol");
        let (_, colon, _) = drawn(&texts, ":");
        let (_, body, _) = drawn(&texts, "gg all");
        let (_, label, _) = drawn(&texts, "SJK");
        // Emblem, name, tick, label, colon and message, left to right on the name's row.
        assert_eq!(name.y, body.y, "the message stays on the name's row");
        assert!(tag.right() < name.x && name.right() <= colon.x && colon.x < body.x);
        assert!(
            name.right() < label.x && label.x < colon.x,
            "the label after the name"
        );
        let tick = ticks(&chat);
        assert_eq!(tick.len(), 1);
        assert!(tick[0].x >= name.x && tick[0].right() <= colon.x);
    }

    #[test]
    fn a_long_sjk_line_wraps_like_a_game_line() {
        // 131 characters, well within the hub's limit (300) and more than a row holds.
        let words = "the quick brown fox jumps over the lazy dog ".repeat(3);
        let text = words.trim_end();
        let chat = feed(
            &[&format!("Sol: {text}")],
            vec![message(1, "Sol", text, false)],
        );
        let game = &chat.lines[0].wrap;
        let sjk = &chat.lines[1].wrap;
        assert!(sjk.len >= 2, "it wraps");
        let last = sjk.rows[sjk.len - 1].end;
        assert_eq!(last, text.len(), "nothing cut");
        // The game line holds the name on its first row too: as many rows.
        assert_eq!(sjk.len, game.len);
        let texts = texts(&chat);
        let (_, name, _) = drawn(&texts, "Sol");
        // The game line sits above: look for the SJK line's rows from its name down.
        let row = |index: usize| {
            let wanted = &text[sjk.rows[index].clone()];
            texts
                .iter()
                .find(|(drawn, rect, _)| drawn == wanted && rect.y >= name.y)
                .unwrap_or_else(|| panic!("{wanted:?} not in {texts:?}"))
                .1
        };
        assert_eq!(row(0).y, name.y, "the first row is the name's");
        for index in 1..sjk.len {
            assert!(row(index).y > row(index - 1).y);
            assert!(row(index).x < name.x, "later rows start at the edge");
        }
    }

    #[test]
    fn a_game_line_from_an_sjk_player_looks_like_an_sjk_line_but_white() {
        // A JA+ line: the server names nobody, so the line is read from the name. The
        // server cut the fox from the name; the hub's claim carries it.
        let mut game = sjk_protocol::GameState::empty_local(0);
        for (slot, name) in [(0, "Me"), (3, "^5JoF^7 Sol ")] {
            game.replace_config_string(1131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                .unwrap();
        }
        let mut chat = ChatOverlay::new();
        chat.update_roster(&game);
        chat.slot_marks = vec![crate::chat::marks::SlotMark::default(); 32];
        chat.slot_marks[3] = crate::chat::marks::SlotMark {
            sjk: true,
            client: None,
            verified: true,
            tick: sjk_identity::Tick::None,
            trust: None,
            emoji: String::new(),
            name: Some("^5JoF^7 Sol 🦊".to_owned()),
            key_id: String::new(),
        };
        chat.receive(
            ServerEventKind::Chat,
            "^5JoF^7 Sol ^7: ^2gg all".to_owned(),
            None,
            Instant::now(),
        );
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        assert_eq!(chat.lines[0].sender.map(|target| target.slot()), Some(3));
        let shown = texts(&chat);
        let (_, name, _) = drawn(&shown, "^5JoF^7 Sol 🦊");
        let (_, colon, _) = drawn(&shown, ":");
        let (_, body, colour) = drawn(&shown, "^2gg all");
        assert_eq!(name.y, body.y, "one flowing line");
        assert_ne!(rgb(*colour), rgb(GOLD), "a game line keeps its colours");
        let logo = logos(&chat);
        assert_eq!(logo.len(), 1, "SJK's emblem");
        let tick = ticks(&chat);
        assert_eq!(tick.len(), 1, "the verified tick");
        assert!(logo[0].right() < name.x && tick[0].x >= name.x && tick[0].right() <= colon.x);
        // A player the hub does not know: no emblem, no tick.
        chat.slot_marks[3] = crate::chat::marks::SlotMark::default();
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
        let again = texts(&chat);
        drawn(&again, "^5JoF^7 Sol ");
        assert!(logos(&chat).is_empty() && ticks(&chat).is_empty());
    }

    #[test]
    fn a_message_sent_from_jof_eternaljk_starts_with_its_icon() {
        let ejk = ChatMessage {
            reactions: Default::default(),
            client: "ejk".to_owned(),
            ..message(2, "Milamber", "hello from JoF", false)
        };
        let chat = feed(&[], vec![message(1, "Sol", "hello", false), ejk]);
        let logos = logos(&chat);
        let icons = quads(&chat, crate::ui_renderer::EJK_TEXTURE);
        assert_eq!(logos.len(), 1, "SJK's emblem before SJK's message only");
        assert_eq!(icons.len(), 1, "JoF EternalJK's icon before its message");
        assert_eq!(
            (icons[0].width, icons[0].height),
            (logos[0].width, logos[0].height),
            "the same size"
        );
        assert!(icons[0].y > logos[0].y, "on the second line");
    }

    fn drop(id: u64, name: &str, tier: &str, own: bool) -> ChatMessage {
        ChatMessage {
            reactions: Default::default(),
            name: name.to_owned(),
            verified: true,
            holocron: Some(sjk_identity::DropMark {
                tier: tier.to_owned(),
                own,
                find: None,
            }),
            ..message(id, name, "", true)
        }
    }

    /// A grand find of a blade (01/11/2026, Sol's request): the holocron's mark, the
    /// blade's room after it, and the prize's words in the blade's own tier colour, the
    /// rest of the sentence in the crystal's.
    #[test]
    fn a_blade_find_colours_its_prize_and_makes_room_for_its_blade() {
        let find = |id: u64, skin: &str| {
            let mut message = drop(id, "Fox", "kyber_rare", false);
            message.holocron.as_mut().unwrap().find = Some(sjk_identity::FindMark {
                shader: true,
                name: "Phoenix blade".to_owned(),
                rarity: "mythical".to_owned(),
                finish: String::new(),
                card: String::new(),
                skin: skin.to_owned(),
            });
            message
        };
        let chat = feed(&[], vec![find(1, "saber_phoenix"), find(2, "")]);
        let texts = texts(&chat);
        let phoenix = crate::unlockables::blade_skin("saber_phoenix").unwrap();
        let runs: Vec<_> = texts
            .iter()
            .filter(|(text, ..)| text.contains("Fox") || text.contains("Phoenix") || text == "!")
            .collect();
        let starts: Vec<_> = runs
            .iter()
            .filter(|(text, ..)| text.starts_with("Fox opened"))
            .map(|(_, rect, colour)| (rect.x, rgb(*colour)))
            .collect();
        assert_eq!(starts.len(), 2, "{runs:?}");
        let crystal = rgb(crate::kyber::from_id("kyber_rare").unwrap().colour);
        assert!(starts.iter().all(|(_, colour)| *colour == crystal));
        // The line with its blade starts its words further right, after the blade.
        assert!(starts[0].0 > starts[1].0 + 20.0, "{starts:?}");
        let prizes: Vec<_> = runs
            .iter()
            .filter(|(text, ..)| text == "Phoenix blade")
            .collect();
        assert_eq!(prizes.len(), 2);
        assert_eq!(rgb(prizes[0].2), rgb(phoenix.tier.colour()));
        assert_eq!(rgb(prizes[1].2), rgb(crate::holocrons::TIERS[3].colour));
    }

    /// A holocron drop is one sentence in its tier's colour (not the SJK chat's gold),
    /// with the tier's gem where the tick goes: no name column, no colon, no tick.
    #[test]
    fn a_holocron_drop_is_one_sentence_in_its_tiers_colour_with_a_gem() {
        use crate::holocrons::TIERS;
        let chat = feed(
            &[],
            vec![
                drop(1, "^2Sol", "legendary", false),
                drop(2, "^2Sol", "uncommon", true),
                drop(3, "^2Sol", "from_the_future", false),
                message(4, "Fox", "gg", false),
            ],
        );
        assert_eq!(chat.lines.len(), 3, "the unknown tier is left out");
        let texts = texts(&chat);
        let (_, grand, colour) = drawn(&texts, "Sol found a Legendary Holocron!");
        assert_eq!(rgb(*colour), rgb(TIERS[2].colour));
        let (_, own, colour) = drawn(&texts, "You found an Uncommon Holocron.");
        assert_eq!(rgb(*colour), rgb(TIERS[0].colour));
        assert!(own.y > grand.y, "in the order they came");
        assert_eq!(
            texts.iter().filter(|(text, ..)| text == ":").count(),
            1,
            "Fox's colon only: {texts:?}"
        );
        assert!(
            !texts.iter().any(|(text, ..)| text.is_empty()),
            "a drop has no empty name run"
        );
        assert!(ticks(&chat).is_empty(), "the gem stands for the tick");
        // Each drop draws a gem (its picture is not loaded in a test); SJK's emblem is on
        // Fox's line only: a drop starts with its holocron (01/11/2026).
        let bars = chat
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::SolidRect { .. }))
            .count();
        assert!(bars >= 2 * crate::holocrons::gem::MARK_ROWS, "{bars}");
        assert_eq!(logos(&chat).len(), 1);
        // The sentence follows the gem on its row.
        let gem = chat
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::SolidRect { rect, .. }
                    if rect.y >= grand.y && rect.y < grand.y + grand.height =>
                {
                    Some(rect.right())
                }
                _ => None,
            })
            .fold(f32::MIN, f32::max);
        assert!(gem > f32::MIN && grand.x > gem, "{gem} {grand:?}");
    }

    /// A muted player's drops are not shown at all, not hidden behind a note.
    #[test]
    fn a_muted_players_drops_are_left_out() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let state = |messages| ChatState {
            messages,
            loaded: Some(1),
            ..ChatState::default()
        };
        chat.sync_sjk(&state(Default::default()), |_| false, now);
        let troll = drop(2, "Troll", "mythical", false);
        let friend = drop(3, "Fox", "mythical", false);
        let key = troll.key_id.clone();
        chat.sync_sjk(
            &state([troll, friend].into_iter().collect()),
            |candidate| candidate == key,
            now,
        );
        let bodies: Vec<&str> = chat.lines.iter().map(|line| line.body.as_str()).collect();
        assert_eq!(bodies, ["Fox found a Mythical Holocron!"]);
    }

    #[test]
    fn a_verified_sender_has_the_tick_alone() {
        let chat = feed(
            &[],
            vec![
                message(1, "Sol", "first words", true),
                message(2, "Fox", "second words", false),
            ],
        );
        assert_eq!(ticks(&chat).len(), 1, "one tick, for the verified sender");
        let texts = texts(&chat);
        assert!(
            texts
                .iter()
                .all(|(text, ..)| !text.to_ascii_lowercase().contains("verified")),
            "{texts:?}"
        );
        assert_eq!(logos(&chat).len(), 2, "each line has its emblem");
    }
}
