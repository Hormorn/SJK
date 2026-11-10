//! The SJK chat page's drawing, in the SJK UI's look: the messages down the left,
//! newest at the bottom over the field and Send, and the chosen message with what
//! can be done about it on the right. A message flows as one line, as SJK chat does
//! everywhere ([`crate::sjk_chat_look`]): the JoF emblem for the clan's tag
//! ([`crate::jof_tag`]), the name, the verified tick for a verified
//! sender, then the text in the SJK chat's gold, wrapping only when it is too long. Over
//! the chosen message, a small window shows who is online and who was seen last as
//! faces, their pictures or initials ([`super::people`]); its List shows every online
//! player as a row instead, with the clients they read the chat with, over the chosen
//! message. A name in the chat or a face
//! in the window under the pointer shows that player's sender card over the page.
//! While the GIF picker is open it takes the messages' place
//! ([`super::gif_picker`]), and its key card takes the whole page's.

use super::*;
use crate::menu::sjk::recent::ago;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar};
use crate::menu_widgets::TextFamily;
use crate::sjk_chat_look::{self, Measure};
use crate::text::TextFace;
use sjk_identity::ChatMessage;
use sjk_identity::chat::name_for_display;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};
use std::ops::Range;

/// The columns (frame pixels).
pub(super) const LIST_X: f32 = 96.0;
pub(super) const LIST_WIDTH: f32 = 1_180.0;
const SIDE_X: f32 = 1_340.0;
const SIDE_WIDTH: f32 = 484.0;
const TOP: f32 = 170.0;
/// The messages' area.
const LIST_TOP: f32 = 236.0;
const LIST_BOTTOM: f32 = 880.0;
/// A message: its first row (the name's), each row after it and the gap after it.
const NAME_ROW: f32 = 28.0;
const BODY_ROW: f32 = 24.0;
const GAP: f32 = 12.0;
/// The sizes of a message's name, its text and what is said of it (frame pixels).
const NAME_SIZE: f32 = 19.0;
const TEXT_SIZE: f32 = 17.0;
const META_SIZE: f32 = 15.0;
/// The field, GIF and Send.
const FIELD_Y: f32 = 900.0;
const FIELD_WIDTH: f32 = 936.0;
const GIF_X: f32 = LIST_X + FIELD_WIDTH + 16.0;
const SEND_X: f32 = LIST_X + 1_060.0;
/// Send only in SJK, under Send and as big; Retry then goes left of it.
const ONLY_Y: f32 = FIELD_Y + 52.0;
const RETRY_BESIDE_X: f32 = SEND_X - 132.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;
/// The who-is-online window: its left, top and width (its height follows what it
/// shows), its first heading, from a heading to the faces under it, from the faces to
/// the next heading, a quiet line's row and the room under the last part (frame pixels).
const PEOPLE_X: f32 = SIDE_X - 18.0;
const PEOPLE_TOP: f32 = 204.0;
const PEOPLE_WIDTH: f32 = SIDE_WIDTH + 36.0;
const ONLINE_Y: f32 = 230.0;
const UNDER_HEADING: f32 = 22.0;
const PART_GAP: f32 = 34.0;
const NOTE_ROW: f32 = 22.0;
const PEOPLE_PAD: f32 = 18.0;
/// The chosen message's Translate, at the right of its sender's name.
const TRANSLATE_WIDTH: f32 = 150.0;
/// The chosen message's heading, this far under the window.
const CHOSEN_GAP: f32 = 34.0;
/// The window's List / Faces button, at the right end of the Online heading.
const LIST_BUTTON_WIDTH: f32 = 120.0;
/// The window as a list: its bottom, a row, a row's face, the name's size and a
/// client's icon with the gap before it (frame pixels).
const LIST_BOTTOM_Y: f32 = 984.0;
const LIST_ROW: f32 = 34.0;
const LIST_FACE: f32 = 13.0;
const LIST_NAME_SIZE: f32 = 17.0;
const CLIENT_SIDE: f32 = 22.0;
const CLIENT_GAP: f32 = 6.0;
/// A client's name after its icon, and the room between two clients.
const CLIENT_LABEL_SIZE: f32 = 14.0;
const CLIENT_SPACING: f32 = 14.0;

/// A name under the pointer: whose card it shows, who, and the name's rectangle.
type Shows = (CardSource, crate::sender_card::Person, sjk_ui::Rect);

/// A message's height on the page, its text in `rows` rows.
fn height(rows: usize) -> f32 {
    NAME_ROW + rows.saturating_sub(1) as f32 * BODY_ROW + GAP
}

/// A message laid out on the page, in frame pixels.
struct Laid {
    /// The name as shown, with its colour codes.
    name: String,
    /// The room the JoF emblem takes before the name ([`crate::jof_tag`]), or 0.
    jof: f32,
    name_width: f32,
    /// The sender's name emoji after the name, and its room with its gap.
    emoji: String,
    emoji_width: f32,
    /// The mark after the name (`ui_renderer::mark_texture`), none for a drop.
    mark: Option<sjk_ui::TextureId>,
    /// Where the text starts on the first row, after the name, the tick and the colon.
    indent: f32,
    /// Staff, and how long ago it came, on the right of the first row.
    meta: String,
    staff: bool,
    /// The text (or that its sender is muted here) and its rows.
    text: String,
    rows: Vec<Range<usize>>,
    muted: bool,
    /// The message is a holocron drop of this tier (or of one this client does not
    /// know, `None` with a text that says so): the tier's gem stands where the tick
    /// does, there is no colon, and the text is in the tier's colour.
    tier: Option<&'static crate::holocrons::Tier>,
    drop: bool,
    /// The SJK team's line (`official.rs`): "SJK" in gold, its mark where the tick goes,
    /// the title in bold; a click opens it.
    official: Option<crate::official::words::Look>,
    /// The message's GIF ([`crate::chat_gifs`]) and what it shows this frame.
    gif: Option<crate::chat_gifs::Shown>,
    /// The translation the player asked for (`chat/translate.rs`), or what it came to,
    /// and its rows, under the text and the GIF.
    translation: Option<(String, Vec<Range<usize>>)>,
}

impl Laid {
    /// The message's height on the page, its GIF and translation under its text
    /// included.
    fn height(&self) -> f32 {
        height(self.rows.len()) + self.gif_room() + self.translation_rows() as f32 * BODY_ROW
    }

    fn gif_room(&self) -> f32 {
        self.gif
            .map_or(0.0, |shown| crate::chat_gifs::draw::room(GIF, shown))
    }

    fn translation_rows(&self) -> usize {
        self.translation.as_ref().map_or(0, |(_, rows)| rows.len())
    }
}

/// How the page shows a message's GIF.
const GIF: crate::chat_gifs::draw::Look = crate::chat_gifs::draw::PAGE;

/// Lay `message` out in the page's column, measured as `measure` draws at the frame
/// scale `s`.
fn lay(
    message: &ChatMessage,
    inputs: &Inputs<'_>,
    measure: &Measure<'_>,
    s: f32,
    translation: Option<&str>,
) -> Laid {
    let s = s.max(0.001);
    let width = |value: &str, size: f32, face| measure.width(value, size * s, face) / s;
    let name = name_for_display(&message.name);
    let name = if name.is_empty() {
        "(no name)".to_owned()
    } else {
        name
    };
    let jof = if let Some(clan) = crate::jof_tag::clan(&message.name) {
        crate::jof_tag::room(clan, crate::jof_tag::side(NAME_SIZE))
    } else {
        0.0
    };
    let name_width = width(&name, NAME_SIZE, TextFace::Semibold).min(LIST_WIDTH * 0.4);
    // A holocron drop reads `Sol <gem> found a Rare Holocron.` on the page: the name, the
    // tier's gem for the tick, no colon.
    let drop = message.holocron.is_some();
    let words = crate::holocrons::line::words(message);
    let official = crate::official::words::words(message, "Read more");
    let mark = crate::ui_renderer::mark_texture(message.verified, message.tick, message.trust)
        .filter(|_| !drop);
    let tick = if mark.is_some() || drop || official.is_some() {
        sjk_chat_look::tick_room(NAME_SIZE)
    } else {
        0.0
    };
    let colon = if drop {
        0.0
    } else {
        width(": ", TEXT_SIZE, TextFace::Regular)
    };
    let emoji = if drop {
        String::new()
    } else {
        sjk_identity::name_emoji::for_display(&message.emoji).to_owned()
    };
    let emoji_width = if emoji.is_empty() {
        0.0
    } else {
        sjk_chat_look::emoji_gap(NAME_SIZE) + width(&emoji, NAME_SIZE, TextFace::Regular)
    };
    let indent = jof + name_width + emoji_width + tick + colon;
    let staff = if message.creator {
        "Creator  ·  "
    } else if message.staff {
        "Staff  ·  "
    } else {
        ""
    };
    let when = ago(u64::try_from(message.at).unwrap_or(0), inputs.now);
    let muted = inputs.muted.contains(&message.key_id) && official.is_none();
    let counts = if muted {
        [0; 6]
    } else {
        sjk_identity::reactions::counts(&message.reactions, None).0
    };
    // One allocation, as before: reserve for the bounded reaction suffix too.
    let mut meta = String::with_capacity(128);
    use std::fmt::Write as _;
    let _ = write!(
        meta,
        "{staff}{when}{}",
        sjk_identity::reactions::Summary(counts)
    );
    let meta_room = width(&meta, META_SIZE, TextFace::Regular) + 24.0;
    let mut gif = None;
    let text = if let Some(official) = &official {
        official.text.clone()
    } else if muted {
        "Blocked on this PC".to_owned()
    } else if let Some(words) = &words {
        words.rest.clone()
    } else if drop {
        crate::terms::say("found a holocron.").into_owned()
    } else {
        // A muted sender's GIF is never asked for: the text above says so instead. An
        // unverified sender's link stays as typed.
        let silent = muted
            || !crate::chat_gifs::plays_from(message.verified, message.staff, message.creator);
        let (text, id) =
            crate::chat_gifs::for_message(&sjk_chat_look::message_text(&message.text), silent);
        gif = id.as_ref().map(crate::chat_gifs::show);
        text
    };
    let rows = sjk_chat_look::flow(
        &text,
        (LIST_WIDTH - indent - meta_room).max(0.0),
        LIST_WIDTH,
        |value, room| measure.fitting(value, room * s, TEXT_SIZE * s, TextFace::Regular),
    );
    let translation = translation.filter(|_| !muted).map(|line| {
        let rows = sjk_chat_look::flow(line, LIST_WIDTH, LIST_WIDTH, |value, room| {
            measure.fitting(value, room * s, TEXT_SIZE * s, TextFace::Regular)
        });
        (line.to_owned(), rows)
    });
    Laid {
        name,
        jof,
        name_width,
        emoji,
        emoji_width,
        mark,
        indent,
        meta,
        staff: message.staff,
        text,
        rows,
        muted,
        tier: words.map(|words| words.tier),
        drop,
        official: official.map(|official| official.look),
        gif,
        translation,
    }
}

/// Leave out of `canvas` the text runs whose ink reaches `card` (measured with
/// `measure`): text draws over every shape, so a run under a card drawn later would
/// show through it. A run's rectangle can be far wider than its ink (a message's is
/// the whole list's width), so the ink is measured, else short lines beside the card
/// would vanish too.
fn leave_out_text_under(canvas: &mut MenuCanvas, measure: &Measure<'_>, card: sjk_ui::Rect) {
    let mut covered = Vec::new();
    for command in canvas.draw_list().commands() {
        let &DrawCommand::Text {
            rect,
            text,
            size,
            align,
            weight,
            ..
        } = command
        else {
            continue;
        };
        if !rect.overlaps(card) {
            continue;
        }
        let face = if matches!(weight, FontWeight::Regular) {
            TextFace::Regular
        } else {
            TextFace::Semibold
        };
        // A little room for the face's own spacing and weight.
        let ink =
            (measure.width(canvas.stored_text(text), size, face) * 1.05 + 2.0).min(rect.width);
        let x = match align {
            TextAlign::Start => rect.x,
            TextAlign::End => rect.right() - ink,
            _ => rect.x + (rect.width - ink) * 0.5,
        };
        if sjk_ui::Rect::new(x, rect.y, ink, rect.height).overlaps(card) {
            covered.push(text);
        }
    }
    if covered.is_empty() {
        return;
    }
    canvas.draw_list_mut().retain(
        |command| !matches!(command, DrawCommand::Text { text, .. } if covered.contains(text)),
    );
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        let measure = target.body_measure();
        self.build(inputs, &measure, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out, its messages measured in the body family (`measure`).
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, measure: &Measure<'_>, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.order.clear();
        self.follow_translations(inputs.chat);
        std::mem::swap(&mut self.shown.people, &mut self.previous_people);
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        top_bar(&mut self.ui, &frame, "Back", BACK_TOKEN, "SJK chat", None);
        inputs.giphy_key.clone_into(&mut self.giphy_key);
        let has_key = crate::chat_gifs::search::key(inputs.giphy_key).is_some();
        if self.gifs.key_card_due(has_key) {
            self.focus = super::gif_picker::KEY_FIELD_TOKEN;
        }
        if self.gifs.key_card.is_some() {
            // The card alone: text draws over every shape, so nothing goes under it.
            self.card = None;
            self.key_card(&frame);
            self.keys(&frame);
            if !self.order.contains(&self.focus) {
                self.focus = super::gif_picker::KEY_FIELD_TOKEN;
            }
            self.ui.finish(self.focus);
            return;
        }
        self.status(&frame, inputs);
        let named = if self.gifs.open {
            self.shown.messages.clear();
            self.shown.total = inputs.chat.map_or(0, |chat| chat.messages.len());
            self.gif_picker(&frame, inputs);
            None
        } else {
            self.messages(&frame, inputs, measure)
        };
        let (listed, bottom) = self.people(&frame, inputs, measure);
        self.follow_card(named.or(listed));
        self.composer(&frame, inputs);
        // The window's list stands where the chosen message goes.
        if self.shown.list_rect.is_none() {
            self.chosen(&frame, inputs, measure, bottom + CHOSEN_GAP);
        }
        self.keys(&frame);
        self.card(&frame, inputs, measure, viewport);
        self.only_hover(&frame, inputs);
        if !self.order.contains(&self.focus) {
            self.focus = FIELD_TOKEN;
        }
        self.ui.finish(self.focus);
        // The faces changed under a resting pointer (the hub's list came, or changed):
        // the targets after them moved in the frame's list, so find what it rests on.
        if self.shown.people != self.previous_people
            && let Some(at) = self.pointer
        {
            let _ = self.ui.pointer(sjk_ui::InputEvent::PointerMove(at));
        }
    }

    /// Who reads the chat, or why it cannot be read, and what it is.
    fn status(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let reaction_error = self.reaction_notice.message(
            inputs.chat.and_then(|chat| chat.reaction.as_ref()),
            Instant::now(),
        );
        let (line, colour) = match inputs.chat {
            _ if !inputs.enabled => (
                "SJK chat is off (Settings > Network > SJK chat)".to_owned(),
                color::EMBER,
            ),
            None => (
                "The SJK identity is off: turn it on to chat (Settings > Network)".to_owned(),
                color::EMBER,
            ),
            Some(_) if reaction_error.is_some() => {
                (reaction_error.unwrap().to_owned(), color::EMBER)
            }
            Some(chat) if !chat.live => ("Connecting to the SJK hub...".to_owned(), color::MUTED),
            Some(chat) if chat.online == 1 => {
                ("1 SJK player reading".to_owned(), color::GOLD_BRIGHT)
            }
            Some(chat) => (
                format!("{} SJK players reading", chat.online),
                color::GOLD_BRIGHT,
            ),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{line}"),
            frame.rect(LIST_X, TOP - 4.0, LIST_WIDTH, 28.0),
            22.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Every SJK player reads this, in game and in the menus. The hub keeps the last 200 messages, in memory only."
            ),
            frame.rect(LIST_X, TOP + 26.0, LIST_WIDTH, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The messages that fit, newest at the bottom, `scroll` messages up from it; returns
    /// the sender whose name is under the pointer, for the card.
    fn messages(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        measure: &Measure<'_>,
    ) -> Option<Shows> {
        let s = frame.s;
        self.shown.messages.clear();
        self.shown.officials.clear();
        let empty = std::collections::VecDeque::new();
        let messages = inputs.chat.map_or(&empty, |chat| &chat.messages);
        self.shown.total = messages.len();
        self.scroll = self.scroll.min(messages.len().saturating_sub(1));
        // From the newest on show upwards, while they fit.
        let mut room = LIST_BOTTOM - LIST_TOP;
        let mut first = messages.len().saturating_sub(self.scroll);
        let end = first;
        let mut laid = Vec::with_capacity(MESSAGES_SHOWN);
        while first > 0 && end - first < MESSAGES_SHOWN {
            let message = &messages[first - 1];
            let layout = lay(message, inputs, measure, s, self.translation_of(message.id));
            let needed = layout.height();
            if needed > room {
                break;
            }
            room -= needed;
            first -= 1;
            laid.push(layout);
        }
        laid.reverse();
        if first == end {
            let line = if inputs.chat.is_some() && inputs.enabled {
                "Nobody has said anything yet. Say hello."
            } else {
                ""
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(LIST_X, LIST_BOTTOM - 40.0, LIST_WIDTH, 24.0),
                17.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = LIST_TOP + room;
        let mut hovered = None;
        for (index, (message, laid)) in messages.range(first..end).zip(&laid).enumerate() {
            let token = MESSAGE_BASE + index as u16;
            let tall = laid.height();
            let row = [LIST_X - 18.0, y - 4.0, LIST_WIDTH + 18.0, tall - 4.0];
            if self.focus == token || self.selected == Some(message.id) {
                kit::band(&mut self.ui, frame, row);
            }
            self.message(frame, laid, y, measure);
            self.ui
                .hit_region(token, frame.rect(row[0], row[1], row[2], row[3]));
            // The name over its row: resting the pointer on it shows the card.
            let name = frame.rect(LIST_X + laid.jof, y, laid.name_width + 2.0, NAME_ROW - 2.0);
            self.ui.hit_region(NAME_BASE + index as u16, name);
            if self.ui.token_hovered(NAME_BASE + index as u16) && laid.official.is_none() {
                let person = crate::sender_card::Person {
                    name: laid.name.clone(),
                    key_id: Some(message.key_id.clone()),
                    hub_name: None,
                    verified: message.verified,
                    tick: message.tick,
                    staff: message.staff,
                    creator: message.creator,
                    trust: message.trust,
                    emoji: laid.emoji.clone(),
                    medals: crate::medals::Medals::default(),
                    place: crate::sender_card::Place::Unknown,
                    banner: "",
                    avatar: None,
                    note: None,
                    safety: false,
                    report: false,
                    trade: offers_trade(&message.key_id, &laid.name),
                    map_block: None,
                };
                hovered = Some((CardSource::Message(message.id), person, name));
            }
            self.order.push(token);
            self.shown
                .messages
                .push((message.id, message.key_id.clone()));
            self.shown
                .officials
                .push(laid.official.map(|look| look.target));
            y += tall;
        }
        if first > 0 {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{first} older: Page Up"),
                frame.rect(LIST_X, LIST_TOP - 26.0, LIST_WIDTH, 20.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        if self.scroll > 0 {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{} newer: Page Down", self.scroll),
                frame.rect(LIST_X, LIST_BOTTOM - 2.0, LIST_WIDTH, 20.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        hovered
    }

    /// Draw a laid out message whose first row's top is `y`: the JoF emblem, the
    /// name, the tick, the
    /// colon and the text going on after them, then its other rows from the left
    /// edge, and what is said of it on the right of the first row.
    fn message(&mut self, frame: &Frame, laid: &Laid, y: f32, measure: &Measure<'_>) {
        let s = frame.s;
        let first = frame.rect(LIST_X, y, LIST_WIDTH, NAME_ROW - 2.0);
        if let Some(clan) = crate::jof_tag::clan(&laid.name) {
            crate::jof_tag::draw(
                clan,
                first.x,
                first.y + first.height * 0.5,
                crate::jof_tag::side(NAME_SIZE * s),
                1.0,
                |command| {
                    let _ = self.ui.draw_list_mut().push(command);
                },
            );
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", laid.name),
            frame.rect(LIST_X + laid.jof, y, laid.name_width + 2.0, NAME_ROW - 2.0),
            NAME_SIZE * s,
            if laid.official.is_some() {
                sjk_chat_look::ACCENT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut x = LIST_X + laid.jof + laid.name_width;
        if !laid.emoji.is_empty() {
            let gap = sjk_chat_look::emoji_gap(NAME_SIZE);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", laid.emoji),
                frame.rect(x + gap, y, laid.emoji_width - gap + 2.0, NAME_ROW - 2.0),
                NAME_SIZE * s,
                sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0),
                FontWeight::Regular,
                TextAlign::Start,
            );
            x += laid.emoji_width;
        }
        if let Some(look) = laid.official {
            let [left, _] = frame.point(x, y);
            let mut gift = None;
            sjk_chat_look::official_mark(
                self.ui.draw_list_mut(),
                look,
                left,
                first.y + first.height * 0.5,
                NAME_SIZE * s,
                1.0,
                |c, rect, colour| gift = Some((c, rect, colour)),
            );
            if let Some((c, rect, colour)) = gift {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{c}"),
                    rect,
                    NAME_SIZE * s * 1.1,
                    colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            x += sjk_chat_look::tick_room(NAME_SIZE);
        } else if let Some(tier) = laid.tier {
            let [left, _] = frame.point(x, y);
            sjk_chat_look::gem_mark(
                self.ui.draw_list_mut(),
                left,
                first.y + first.height * 0.5,
                NAME_SIZE * s,
                tier,
                1.0,
            );
            x += sjk_chat_look::tick_room(NAME_SIZE);
        } else if let Some(mark) = laid.mark {
            let [left, _] = frame.point(x, y);
            sjk_chat_look::tick(
                self.ui.draw_list_mut(),
                mark,
                left,
                first.y + first.height * 0.5,
                NAME_SIZE * s,
                1.0,
            );
            x += sjk_chat_look::tick_room(NAME_SIZE);
        }
        if !laid.drop {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(":"),
                frame.rect(x, y, 12.0, NAME_ROW - 2.0),
                TEXT_SIZE * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", laid.meta),
            frame.rect(LIST_X + LIST_WIDTH - 420.0, y + 2.0, 420.0, NAME_ROW - 6.0),
            META_SIZE * s,
            if laid.staff {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::End,
        );
        let colour = if laid.muted {
            color::QUIET
        } else {
            sjk_chat_look::text_colour(laid.tier)
        };
        for (row, range) in laid.rows.iter().enumerate() {
            if range.is_empty() {
                continue;
            }
            let rect = if row == 0 {
                frame.rect(
                    LIST_X + laid.indent,
                    y,
                    (LIST_WIDTH - laid.indent).max(1.0),
                    NAME_ROW - 2.0,
                )
            } else {
                frame.rect(
                    LIST_X,
                    y + NAME_ROW + (row - 1) as f32 * BODY_ROW,
                    LIST_WIDTH,
                    BODY_ROW - 2.0,
                )
            };
            let shown = &laid.text[range.clone()];
            // The SJK team's title, in bold where it starts the first row.
            let bold = laid
                .official
                .map_or(0, |look| look.bold)
                .saturating_sub(range.start)
                .min(shown.len());
            if bold > 0 && shown.is_char_boundary(bold) {
                let (title, rest) = shown.split_at(bold);
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{title}"),
                    rect,
                    TEXT_SIZE * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
                let after = measure.width(title, TEXT_SIZE * s, TextFace::Semibold);
                let rest_rect = sjk_ui::Rect::new(
                    rect.x + after,
                    rect.y,
                    (rect.width - after).max(1.0),
                    rect.height,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{rest}"),
                    rest_rect,
                    TEXT_SIZE * s,
                    colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            } else {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{shown}"),
                    rect,
                    TEXT_SIZE * s,
                    colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
        }
        let under = y + NAME_ROW + laid.rows.len().saturating_sub(1) as f32 * BODY_ROW;
        if let Some(shown) = laid.gif {
            crate::chat_gifs::draw::draw(
                &mut self.ui,
                frame,
                GIF,
                shown,
                [LIST_X, under],
                LIST_WIDTH,
            );
        }
        // The translation under it all, dim (`chat/translate.rs`).
        if let Some((line, rows)) = &laid.translation {
            let top = under + laid.gif_room();
            for (row, range) in rows.iter().enumerate() {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{}", &line[range.clone()]),
                    frame.rect(
                        LIST_X,
                        top + row as f32 * BODY_ROW,
                        LIST_WIDTH,
                        BODY_ROW - 2.0,
                    ),
                    TEXT_SIZE * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
        }
    }

    /// The who-is-online window over the chosen message: who reads the chat now
    /// (playing first) and who was seen most recently, as faces, or, from a hub that
    /// does not list them, the count and the chat's last senders. Returns the player
    /// whose face is under the pointer, for the card, and the window's bottom (frame
    /// pixels), which follows what it shows.
    fn people(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        measure: &Measure<'_>,
    ) -> (Option<Shows>, f32) {
        use super::people::{
            FACES_WIDTH, ONLINE_RADIUS, RECENT_RADIUS, centre, columns, grid_height,
        };
        let s = frame.s;
        self.shown.people.clear();
        self.shown.unpictured.clear();
        self.shown.list_rect = None;
        let chat = inputs.chat.filter(|chat| inputs.enabled && chat.live);
        let listed = chat.map_or_else(Default::default, |chat| {
            super::people::derive(chat, inputs.muted)
        });
        // List and Faces, once the hub says who is online and someone is.
        let can_list = chat.is_some() && listed.listed && !listed.online.is_empty();
        if self.people_list && can_list && !self.gifs.open {
            return self.people_rows(frame, &listed, measure);
        }
        let (faces, more) = listed.faces();
        // Where each part goes, so the window can be drawn round them first.
        let online_note = match chat {
            None => Some("Shows once the SJK hub answers."),
            Some(_) if !listed.listed => Some("This hub counts them but does not say who."),
            Some(_) if listed.online.is_empty() => Some("Nobody else."),
            Some(_) => None,
        };
        let online_columns = columns(FACES_WIDTH, ONLINE_RADIUS);
        let recent_columns = columns(FACES_WIDTH, RECENT_RADIUS);
        let online_faces = faces.len() + usize::from(more > 0);
        let online_top = ONLINE_Y + UNDER_HEADING;
        let mut online_bottom =
            online_top + grid_height(online_faces, online_columns, ONLINE_RADIUS);
        if online_note.is_some() {
            online_bottom = online_bottom.max(online_top + NOTE_ROW);
        }
        // The second part, once the hub has answered.
        let recent_y = online_bottom + PART_GAP;
        let recent_top = recent_y + UNDER_HEADING;
        let bottom = match chat {
            None => online_bottom,
            Some(_) if listed.recent.is_empty() => recent_top + NOTE_ROW,
            Some(_) => recent_top + grid_height(listed.recent.len(), recent_columns, RECENT_RADIUS),
        } + PEOPLE_PAD;
        let rect = frame.rect(PEOPLE_X, PEOPLE_TOP, PEOPLE_WIDTH, bottom - PEOPLE_TOP);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 14.0 * s,
            color: color::alpha(color::HOLO, 0.05),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 14.0 * s,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.25),
        });
        let online = match chat {
            Some(_) => format!("Online  \u{b7}  {}", listed.count),
            None => "Online".to_owned(),
        };
        self.online_heading(frame, &online, can_list.then_some("List"));
        let mut hovered = None;
        if let Some(note) = online_note {
            self.quiet_row(frame, online_top, note);
        }
        for (index, row) in faces.iter().enumerate() {
            let at = centre(index, online_columns, ONLINE_RADIUS, [SIDE_X, online_top]);
            let note = super::people::online_note(row);
            hovered = self
                .face(frame, row, at, ONLINE_RADIUS, true, note)
                .or(hovered);
        }
        if more > 0 {
            let at = centre(
                faces.len(),
                online_columns,
                ONLINE_RADIUS,
                [SIDE_X, online_top],
            );
            self.more_face(frame, at, ONLINE_RADIUS, more);
        }
        if chat.is_none() {
            return (hovered, bottom);
        }
        let recent = if listed.listed {
            "Recently active"
        } else {
            "Recently in chat"
        };
        kit::heading(&mut self.ui, frame, SIDE_X, recent_y, SIDE_WIDTH, recent);
        if listed.recent.is_empty() {
            self.quiet_row(frame, recent_top, "Nobody yet.");
        }
        for (index, row) in listed.recent.iter().enumerate() {
            let at = centre(index, recent_columns, RECENT_RADIUS, [SIDE_X, recent_top]);
            let when = ago(row.seen, inputs.now);
            let note = if listed.listed {
                format!("Seen {when}")
            } else {
                format!("Last spoke {when}")
            };
            hovered = self
                .face(frame, row, at, RECENT_RADIUS, false, note)
                .or(hovered);
        }
        (hovered, bottom)
    }

    /// The Online heading, with `button` (List or Faces) at its right end when given.
    fn online_heading(&mut self, frame: &Frame, label: &str, button: Option<&str>) {
        let room = if button.is_some() {
            SIDE_WIDTH - LIST_BUTTON_WIDTH - 12.0
        } else {
            SIDE_WIDTH
        };
        kit::heading(&mut self.ui, frame, SIDE_X, ONLINE_Y, room, label);
        if let Some(button) = button {
            kit::button(
                &mut self.ui,
                frame,
                [
                    SIDE_X + SIDE_WIDTH - LIST_BUTTON_WIDTH,
                    ONLINE_Y - 14.0,
                    LIST_BUTTON_WIDTH,
                    28.0,
                ],
                button,
                false,
                true,
                self.focus == LIST_TOKEN,
                LIST_TOKEN,
            );
            self.order.push(LIST_TOKEN);
        }
    }

    /// The window as a list of every online player (its List), down to the keys, over
    /// the chosen message: a row each, playing first, with their face, name, tick and
    /// the icons of the clients they read the chat with on the right; the wheel scrolls
    /// the rows when they do not all fit. Returns the player whose row is under the
    /// pointer and the window's bottom.
    fn people_rows(
        &mut self,
        frame: &Frame,
        listed: &super::people::Listed,
        measure: &Measure<'_>,
    ) -> (Option<Shows>, f32) {
        let s = frame.s;
        let top = ONLINE_Y + UNDER_HEADING;
        let bottom = LIST_BOTTOM_Y;
        let rect = frame.rect(PEOPLE_X, PEOPLE_TOP, PEOPLE_WIDTH, bottom - PEOPLE_TOP);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 14.0 * s,
            color: color::alpha(color::HOLO, 0.05),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 14.0 * s,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.25),
        });
        let label = format!("Online  \u{b7}  {}", listed.count);
        self.online_heading(frame, &label, Some("Faces"));
        let fits = (((bottom - PEOPLE_PAD - top) / LIST_ROW) as usize).min(PEOPLE_SHOWN);
        let total = listed.online.len();
        // A list too long for the window keeps its last row to say where it is.
        let rows = if total > fits {
            fits.saturating_sub(1)
        } else {
            total
        };
        self.people_scroll = self.people_scroll.min(total - rows);
        let first = self.people_scroll;
        self.shown.list_rect = Some(frame.rect(SIDE_X, top, SIDE_WIDTH, rows as f32 * LIST_ROW));
        let mut hovered = None;
        for (index, row) in listed.online[first..first + rows].iter().enumerate() {
            let y = top + index as f32 * LIST_ROW;
            let token = PERSON_BASE + self.shown.people.len() as u16;
            let band = frame.rect(SIDE_X - 8.0, y, SIDE_WIDTH + 16.0, LIST_ROW);
            if self.ui.token_hovered(token) {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: band,
                    radius: 8.0 * s,
                    color: color::alpha(color::HOLO, 0.10),
                });
            }
            let middle = y + LIST_ROW * 0.5;
            let note = super::people::online_note(row);
            hovered = self
                .face(
                    frame,
                    row,
                    [SIDE_X + LIST_FACE, middle],
                    LIST_FACE,
                    true,
                    note,
                )
                .or(hovered);
            // The whole row answers to the pointer, as its face does.
            self.ui.hit_region(token, band);
            // Each client its icon and its name, from the right edge leftwards.
            let mut x = SIDE_X + SIDE_WIDTH;
            for client in row.clients.iter().rev() {
                let (Some(icon), Some(label)) = (
                    super::people::client_texture(client),
                    super::people::client_label(client),
                ) else {
                    continue;
                };
                let label_width =
                    measure.width(label, CLIENT_LABEL_SIZE * s, TextFace::Regular) / s.max(0.001);
                x -= label_width;
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{label}"),
                    frame.rect(x, middle - 10.0, label_width + 2.0, 20.0),
                    CLIENT_LABEL_SIZE * s,
                    if row.muted {
                        color::QUIET
                    } else {
                        color::MUTED
                    },
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                x -= CLIENT_GAP + CLIENT_SIDE;
                let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: frame.rect(x, middle - CLIENT_SIDE * 0.5, CLIENT_SIDE, CLIENT_SIDE),
                    texture: icon,
                    color: if row.muted {
                        sjk_ui::Color::new(0.6, 0.62, 0.68, 0.5)
                    } else {
                        sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0)
                    },
                });
                x -= CLIENT_SPACING;
            }
            let icons_width = SIDE_X + SIDE_WIDTH - x;
            let name = name_for_display(&row.name);
            let name = if name.is_empty() {
                "(no name)".to_owned()
            } else {
                name
            };
            let left = SIDE_X + LIST_FACE * 2.0 + 12.0;
            let tick_room = sjk_chat_look::tick_room(LIST_NAME_SIZE);
            let room = (SIDE_X + SIDE_WIDTH - icons_width - 8.0 - left - tick_room).max(0.0);
            let name_width = (measure.width(&name, LIST_NAME_SIZE * s, TextFace::Semibold)
                / s.max(0.001))
            .min(room);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{name}"),
                frame.rect(left, middle - 12.0, name_width + 2.0, 24.0),
                LIST_NAME_SIZE * s,
                if row.muted { color::QUIET } else { color::TEXT },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if let Some(mark) = crate::ui_renderer::mark_texture(row.verified, row.tick, row.trust)
            {
                let [at, centre] = frame.point(left + name_width, middle);
                sjk_chat_look::tick(
                    self.ui.draw_list_mut(),
                    mark,
                    at,
                    centre,
                    LIST_NAME_SIZE * s,
                    1.0,
                );
            }
        }
        if rows < total {
            let line = format!(
                "{} to {} of {}  \u{b7}  the wheel scrolls",
                first + 1,
                first + rows,
                total
            );
            self.quiet_row(frame, top + rows as f32 * LIST_ROW + 6.0, &line);
        }
        (hovered, bottom)
    }

    /// A quiet line in the who-is-online window, its top at `y`.
    fn quiet_row(&mut self, frame: &Frame, y: f32, line: &str) {
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(SIDE_X, y, SIDE_WIDTH, NOTE_ROW),
            15.0 * frame.s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// A player's face in the who-is-online window, a disc of `radius` round `at`
    /// (frame pixels): their picture once it is loaded, else their initial on their
    /// colour, as their card draws it; smaller and dimmed for someone not `online`,
    /// greyed for someone muted here, ringed in gold under the pointer, with a gold dot
    /// for someone in a match. Returns them, with `note` for their card, when the
    /// pointer is on it.
    fn face(
        &mut self,
        frame: &Frame,
        row: &super::people::Row,
        at: [f32; 2],
        radius: f32,
        online: bool,
        note: String,
    ) -> Option<Shows> {
        let s = frame.s;
        let token = PERSON_BASE + self.shown.people.len() as u16;
        self.shown.people.push(row.key_id.clone());
        let centre = frame.point(at[0], at[1]);
        let r = radius * s;
        let square = sjk_ui::Rect::new(centre[0] - r, centre[1] - r, r * 2.0, r * 2.0);
        let lit = self.ui.token_hovered(token);
        // The version the hub listed, else one found for a sender (asked after the
        // frame, outside the chat's lock).
        let version = match &row.avatar {
            Some(version) => Some(version.clone()),
            None => {
                let found = self.picture_of(&row.key_id).map(str::to_owned);
                if found.is_none() {
                    self.shown.unpictured.push(row.key_id.clone());
                }
                found
            }
        };
        let texture = version
            .as_deref()
            .and_then(|version| crate::avatars::texture(&row.key_id, version));
        let name = name_for_display(&row.name);
        match texture {
            Some(texture) => {
                let tint = if row.muted {
                    sjk_ui::Color::new(0.5, 0.52, 0.58, 0.45)
                } else if online {
                    sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0)
                } else {
                    sjk_ui::Color::new(0.86, 0.88, 0.92, 0.82)
                };
                let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: square,
                    texture,
                    color: tint,
                });
            }
            None => {
                let ground = if row.muted {
                    color::alpha(color::QUIET, 0.22)
                } else {
                    let own = crate::profile_card::stand_in_colour(&row.key_id, &name);
                    if online { own } else { color::alpha(own, 0.72) }
                };
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: square,
                    radius: r,
                    color: ground,
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{}", super::people::letter(&name)),
                    square,
                    r * 1.1,
                    if row.muted {
                        color::QUIET
                    } else if online {
                        color::TEXT
                    } else {
                        color::MUTED
                    },
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
        let (ring, width) = if lit {
            (color::GOLD_BRIGHT, 2.0 * s)
        } else if row.muted {
            (color::alpha(color::QUIET, 0.35), 1.5 * s)
        } else if online {
            (color::alpha(color::HOLO, 0.5), 1.5 * s)
        } else {
            (color::alpha(color::HOLO, 0.25), 1.5 * s)
        };
        let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
            center: centre,
            radius: r + width,
            width,
            start: 0.0,
            sweep: std::f32::consts::TAU,
            color: ring,
            knockout: None,
        });
        if online && row.playing {
            // In a match: a gold dot at the face's foot, right, rimmed by the ground.
            let foot = [centre[0] + r * 0.72, centre[1] + r * 0.72];
            let dot = if row.muted {
                color::QUIET
            } else {
                color::GOLD_BRIGHT
            };
            for (size, colour) in [(7.0 * s, color::alpha(color::SPACE, 0.95)), (5.0 * s, dot)] {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: sjk_ui::Rect::new(foot[0] - size, foot[1] - size, size * 2.0, size * 2.0),
                    radius: size,
                    color: colour,
                });
            }
        }
        self.ui.hit_region(token, square);
        lit.then(|| {
            let person = crate::sender_card::Person {
                name: if name.is_empty() {
                    "(no name)".to_owned()
                } else {
                    name
                },
                key_id: Some(row.key_id.clone()),
                hub_name: None,
                verified: row.verified,
                tick: row.tick,
                staff: row.staff,
                creator: row.creator,
                trust: row.trust,
                emoji: row.emoji.clone(),
                medals: crate::medals::Medals::default(),
                place: crate::sender_card::Place::Unknown,
                banner: "",
                avatar: version,
                note: Some(note),
                safety: false,
                report: false,
                trade: offers_trade(&row.key_id, &row.name),
                map_block: None,
            };
            (CardSource::Person(row.key_id.clone()), person, square)
        })
    }

    /// The last online face of a full window: how many more players read the chat. A
    /// click on it opens the list of them all.
    fn more_face(&mut self, frame: &Frame, at: [f32; 2], radius: f32, more: usize) {
        let s = frame.s;
        let centre = frame.point(at[0], at[1]);
        let r = radius * s;
        let square = sjk_ui::Rect::new(centre[0] - r, centre[1] - r, r * 2.0, r * 2.0);
        let lit = self.ui.token_hovered(LIST_TOKEN);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: square,
            radius: r,
            color: color::alpha(color::HOLO, if lit { 0.22 } else { 0.12 }),
        });
        let width = 1.5 * s;
        let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
            center: centre,
            radius: r + width,
            width,
            start: 0.0,
            sweep: std::f32::consts::TAU,
            color: color::alpha(color::HOLO, 0.35),
            knockout: None,
        });
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("+{more}"),
            square,
            if more > 99 { 0.62 } else { 0.78 } * r,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        self.ui.hit_region(LIST_TOKEN, square);
    }

    /// Send only in SJK's hover card, over the page: why it should stay rare.
    fn only_hover(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let now = Instant::now();
        self.hover.follow(self.ui.hovered(), now);
        let Some(words) = inputs.said_in_all else {
            return;
        };
        let Some((ONLY_TOKEN, alpha)) = self.hover.showing(now) else {
            return;
        };
        let Some(anchor) = self.ui.rect_for(ONLY_TOKEN) else {
            return;
        };
        let card = crate::menu::sjk::hover::Card {
            title: words.only_title.to_owned(),
            colour: color::EMBER,
            lines: words
                .only_why
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    if index == 1 {
                        crate::menu::sjk::hover::Line::how(*line)
                    } else {
                        crate::menu::sjk::hover::Line::status(*line)
                    }
                })
                .collect(),
        };
        crate::menu::sjk::hover::draw(&mut self.ui, frame, anchor, &card, alpha);
    }

    /// The field, Send, and what became of the last message.
    fn composer(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let focused = self.focus == FIELD_TOKEN;
        let blink = (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let at = self.caret.cursor(&self.draft);
        // The caret blinks at the end; inside the text it stays, so the text after it
        // does not shift every half second.
        let mark = if focused && (blink || at < self.draft.len()) {
            "|"
        } else {
            ""
        };
        let shown = if self.draft.is_empty() && !focused {
            "Say something to every SJK player".to_owned()
        } else {
            crate::sjk_chat_field::with_caret(&self.draft, at, mark, 96)
        };
        let field = [LIST_X, FIELD_Y, FIELD_WIDTH, 46.0];
        kit::field(
            &mut self.ui,
            frame,
            field,
            format_args!("{shown}"),
            focused,
            false,
        );
        self.ui.hit_region(
            FIELD_TOKEN,
            frame.rect(field[0], field[1], field[2], field[3]),
        );
        self.order.push(FIELD_TOKEN);
        let can_send = !self.draft.trim().is_empty() && inputs.enabled && inputs.chat.is_some();
        // GIF opens the picker (closes it when open); off with the chat's GIFs.
        let gifs_on = crate::chat_gifs::enabled();
        kit::button(
            &mut self.ui,
            frame,
            [GIF_X, FIELD_Y + 2.0, 88.0, 42.0],
            "GIF",
            false,
            gifs_on,
            self.focus == GIF_TOKEN || self.gifs.open,
            GIF_TOKEN,
        );
        if gifs_on {
            self.order.push(GIF_TOKEN);
        }
        kit::button(
            &mut self.ui,
            frame,
            [SEND_X, FIELD_Y + 2.0, 120.0, 42.0],
            "Send",
            true,
            can_send,
            self.focus == SEND_TOKEN,
            SEND_TOKEN,
        );
        self.order.push(SEND_TOKEN);
        // Where a mod says SJK chat in this server's All chat: Send only in SJK, under
        // Send and as big, for every player; its hover card says to keep it rare.
        self.only_for_staff = inputs.said_in_all.is_some() && inputs.staff;
        if let Some(words) = inputs.said_in_all {
            kit::button(
                &mut self.ui,
                frame,
                [SEND_X, ONLY_Y, 120.0, 42.0],
                words.only,
                false,
                can_send,
                self.focus == ONLY_TOKEN,
                ONLY_TOKEN,
            );
            self.order.push(ONLY_TOKEN);
        }
        let notice = inputs
            .chat
            .and_then(|chat| chat.outcome.as_ref())
            .filter(|outcome| !outcome.sent)
            .map(|outcome| outcome.message.as_str());
        let line = match notice {
            Some(why) => format!("Not sent: {why}"),
            None => format!("{} of {DRAFT_MAX}", self.draft.chars().count()),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LIST_X + 4.0, FIELD_Y + 54.0, FIELD_WIDTH, 20.0),
            14.0 * s,
            if notice.is_some() {
                color::EMBER
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        // A refusal (the hub's too many requests, mostly) can be tried again.
        if notice.is_some() && !self.last_sent.is_empty() && inputs.enabled {
            kit::button(
                &mut self.ui,
                frame,
                [
                    if inputs.said_in_all.is_some() {
                        RETRY_BESIDE_X
                    } else {
                        SEND_X
                    },
                    FIELD_Y + 52.0,
                    120.0,
                    34.0,
                ],
                "Retry",
                false,
                true,
                self.focus == RETRY_TOKEN,
                RETRY_TOKEN,
            );
            self.order.push(RETRY_TOKEN);
        }
    }

    /// The players blocked on this PC, each with Unblock, while no message is chosen:
    /// as many as there is room for above the keys' line, the console's `block` listing
    /// the rest.
    fn blocked_list(&mut self, frame: &Frame, inputs: &Inputs<'_>, top: f32) {
        const ROW: f32 = 44.0;
        if inputs.blocked.is_empty() {
            return;
        }
        let s = frame.s;
        let room = ((KEYS_Y - 40.0 - top - 30.0) / ROW).max(0.0) as usize;
        let shown = inputs.blocked.len().min(room).min(UNBLOCK_SHOWN);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Blocked on this PC ({})", inputs.blocked.len()),
            frame.rect(SIDE_X, top, SIDE_WIDTH, 24.0),
            17.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        for (index, blocked) in inputs.blocked.iter().take(shown).enumerate() {
            let y = top + 32.0 + index as f32 * ROW;
            let token = UNBLOCK_BASE + index as u16;
            let name = name_for_display(&blocked.name);
            let name = if name.trim().is_empty() {
                "(no name)".into()
            } else {
                name
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{name}"),
                frame.rect(SIDE_X, y + 8.0, SIDE_WIDTH - 160.0, 24.0),
                16.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            kit::button(
                &mut self.ui,
                frame,
                [SIDE_X + SIDE_WIDTH - 140.0, y, 140.0, 36.0],
                "Unblock",
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
            self.shown.blocked.push(blocked.clone());
        }
        let more = inputs.blocked.len() - shown;
        if more > 0 {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{more} more: block in the console lists them all"),
                frame.rect(
                    SIDE_X,
                    top + 32.0 + shown as f32 * ROW + 4.0,
                    SIDE_WIDTH,
                    22.0,
                ),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The chosen message's sender and what can be done: Block here, Ban 24 h for a
    /// sender the hub does not vouch for when it vouches for the player, and for staff
    /// Delete, Lift chat ban and Mute at the hub.
    fn chosen(&mut self, frame: &Frame, inputs: &Inputs<'_>, measure: &Measure<'_>, top: f32) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            SIDE_X,
            top,
            SIDE_WIDTH,
            "Chosen message",
        );
        self.shown.chosen_bannable = false;
        self.shown.chosen_translate = false;
        let message = self.selected.and_then(|id| {
            inputs.chat.and_then(|chat| {
                // The SJK team's lines have no sender to act on.
                chat.messages
                    .iter()
                    .find(|message| message.id == id && message.official.is_none())
            })
        });
        self.shown.chosen_gif = None;
        self.shown.chosen_source = None;
        self.shown.blocked.clear();
        let Some(message) = message else {
            self.shown.chosen = None;
            for (row, line) in [
                "Choose a message (Up from the field, or a click)",
                "to block its sender on this PC.",
            ]
            .into_iter()
            .enumerate()
            {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{line}"),
                    frame.rect(SIDE_X, top + 30.0 + row as f32 * 24.0, SIDE_WIDTH, 22.0),
                    16.0 * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            self.blocked_list(frame, inputs, top + 98.0);
            return;
        };
        let muted = inputs.muted.contains(&message.key_id);
        // The name in the body family, which the page measures, and the tick alone
        // after it for a verified sender.
        let name = name_for_display(&message.name);
        self.shown.chosen = Some((message.id, message.key_id.clone(), name.clone(), muted));
        // Translate at the right of the name, for another's message that may be foreign
        // (`chat/translate.rs`): not one surely in English or a skipped language.
        let own = inputs
            .chat
            .and_then(|chat| chat.own_key.as_ref())
            .and_then(|key| std::str::from_utf8(key).ok())
            .is_some_and(|key| key == message.key_id);
        let offered = match inputs.translate {
            Some(skip) if !muted && !own && message.holocron.is_none() => {
                // Guessed once per message (and skip list), not every frame.
                let known = matches!(&self.translate_guess,
                    Some((id, kept, _)) if *id == message.id && kept == skip);
                if !known {
                    let plain =
                        crate::chat::translate::plain(&sjk_chat_look::message_text(&message.text));
                    let offer = crate::chat::translate::offered(&plain, skip).then_some(plain);
                    self.translate_guess = Some((message.id, skip.clone(), offer));
                }
                self.translate_guess
                    .as_ref()
                    .is_some_and(|(_, _, offer)| offer.is_some())
            }
            _ => false,
        };
        if offered {
            kit::button(
                &mut self.ui,
                frame,
                [
                    SIDE_X + SIDE_WIDTH - TRANSLATE_WIDTH,
                    top + 24.0,
                    TRANSLATE_WIDTH,
                    40.0,
                ],
                "Translate",
                false,
                true,
                self.focus == TRANSLATE_TOKEN,
                TRANSLATE_TOKEN,
            );
            self.order.push(TRANSLATE_TOKEN);
            self.shown.chosen_translate = true;
        }
        let size = 26.0;
        let room = SIDE_WIDTH
            - sjk_chat_look::tick_room(size)
            - if self.shown.chosen_translate {
                TRANSLATE_WIDTH + 12.0
            } else {
                0.0
            };
        let width = (measure.width(&name, size * s, TextFace::Semibold) / s.max(0.001)).min(room);
        let rect = frame.rect(SIDE_X, top + 28.0, width + 2.0, 34.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{name}"),
            rect,
            size * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some(mark) =
            crate::ui_renderer::mark_texture(message.verified, message.tick, message.trust)
        {
            sjk_chat_look::tick(
                self.ui.draw_list_mut(),
                mark,
                frame.point(SIDE_X + width, 0.0)[0],
                rect.y + rect.height * 0.5,
                size * s,
                1.0,
            );
        }
        // No key id here: key ids never show in the game.
        super::reactions::draw(
            self,
            frame,
            message,
            [SIDE_X, top + 68.0, SIDE_WIDTH, 36.0],
            !muted && inputs.enabled,
            inputs
                .chat
                .and_then(|chat| chat.own_key.as_ref())
                .and_then(|key| std::str::from_utf8(key).ok()),
        );
        let mut y = top + 110.0;
        kit::button(
            &mut self.ui,
            frame,
            [SIDE_X, y, 240.0, 42.0],
            if muted {
                "Unblock on this PC"
            } else {
                "Block on this PC"
            },
            false,
            true,
            self.focus == MUTE_TOKEN,
            MUTE_TOKEN,
        );
        self.order.push(MUTE_TOKEN);
        let quiet = "Hides their chat, model, sabers, sounds and emotes on this PC.";
        let under = frame.rect(SIDE_X, y + 50.0, SIDE_WIDTH, 22.0);
        let bannable = crate::chat_ban::offered(
            Some(&message.key_id),
            message.verified,
            message.staff,
            message.trust,
        );
        self.shown.chosen_bannable = bannable;
        if bannable {
            // Beside Mute; what it came to (or the armed button's warning) takes the
            // quiet line.
            let (ui, focused) = (&mut self.ui, self.focus == BAN_TOKEN);
            crate::chat_ban::with_shown(&message.key_id, |shown| {
                kit::button(
                    ui,
                    frame,
                    [SIDE_X + 252.0, y, 232.0, 42.0],
                    shown.label,
                    shown.armed,
                    true,
                    focused,
                    BAN_TOKEN,
                );
                let (said, colour) = match shown.line {
                    Some((said, true)) => (said, color::GOLD_BRIGHT),
                    Some((said, false)) => (said, color::EMBER),
                    None => (quiet, color::QUIET),
                };
                text(
                    ui,
                    TextFamily::Body,
                    format_args!("{said}"),
                    under,
                    14.0 * s,
                    colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            });
            self.order.push(BAN_TOKEN);
        } else {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{quiet}"),
                under,
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // The message's GIF (none for a muted sender), on one row as Staff's buttons are:
        // one that shows can be made a favourite for the GIF picker and hidden on this PC;
        // a hidden one shown again.
        if let Some(gif) = crate::chat_gifs::chosen_gif(
            &message.text,
            muted
                || !crate::chat_gifs::plays_from(message.verified, message.staff, message.creator),
        ) {
            let hidden = crate::chat_gifs::hidden::contains(&gif);
            let at = y + 84.0;
            let (hide_x, hide_width) = if hidden {
                (SIDE_X, 300.0)
            } else {
                let favourite = crate::chat_gifs::favourites::contains(&gif);
                kit::button(
                    &mut self.ui,
                    frame,
                    [SIDE_X, at, 240.0, 42.0],
                    if favourite {
                        "Remove GIF favourite"
                    } else {
                        "Add GIF to favourites"
                    },
                    false,
                    true,
                    self.focus == FAVOURITE_TOKEN,
                    FAVOURITE_TOKEN,
                );
                self.order.push(FAVOURITE_TOKEN);
                (SIDE_X + 252.0, 232.0)
            };
            kit::button(
                &mut self.ui,
                frame,
                [hide_x, at, hide_width, 42.0],
                if hidden {
                    "Show this GIF again"
                } else {
                    "Hide this GIF"
                },
                false,
                true,
                self.focus == HIDE_GIF_TOKEN,
                HIDE_GIF_TOKEN,
            );
            self.order.push(HIDE_GIF_TOKEN);
            self.shown.chosen_gif = Some(gif);
            // Manage GIF source (or Staff's part) under the row, rows 54 apart as Staff's.
            y = at - 50.0;
        }
        if !muted && let Some((id, _)) = crate::chat_gifs::link::find(&message.text) {
            let at = y + 104.0;
            let token = super::gif_sources::OPEN;
            kit::button(
                &mut self.ui,
                frame,
                [SIDE_X, at, 300.0, 42.0],
                "Manage GIF source...",
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
            self.shown.chosen_source = Some(id);
            y = at - 30.0;
        }
        if !inputs.staff {
            return;
        }
        y += 104.0;
        kit::heading(&mut self.ui, frame, SIDE_X, y, SIDE_WIDTH, "Staff");
        y += 24.0;
        // Lift chat ban beside Delete and Unmute beside Mute: two rows, so a message with a
        // GIF and its two rows of tools still fits above the keys.
        for (row, column, label, token) in [
            (0_u8, 0, "Delete for everyone", DELETE_TOKEN),
            (0, 1, "Lift chat ban", LIFT_TOKEN),
            (1, 0, "Mute at the hub", HUB_MUTE_TOKEN),
            (1, 1, "Unmute at the hub", HUB_UNMUTE_TOKEN),
        ] {
            let (x, width) = if column == 0 {
                (SIDE_X, 240.0)
            } else {
                (SIDE_X + 252.0, 232.0)
            };
            kit::button(
                &mut self.ui,
                frame,
                [x, y + f32::from(row) * 54.0, width, 42.0],
                label,
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        // What the page's last staff request came to, else what staff requests are.
        let answer = inputs
            .staff_state
            .zip(self.staff_after)
            .and_then(|(state, after)| {
                if state.serial > after {
                    Some(if state.failed {
                        (format!("Not done: {}", state.message), color::EMBER)
                    } else {
                        (format!("Done: {}", state.message), color::GOLD_BRIGHT)
                    })
                } else {
                    state
                        .busy
                        .then(|| ("Sending to the hub...".to_owned(), color::MUTED))
                }
            });
        let (line, colour) =
            answer.unwrap_or_else(|| ("The hub logs what staff do.".to_owned(), color::QUIET));
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(SIDE_X, y + 2.0 * 54.0, SIDE_WIDTH, 22.0),
            14.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(SIDE_X - 30.0, LIST_TOP - 20.0, 1.0, LIST_BOTTOM - LIST_TOP),
            color: color::alpha(color::HOLO, 0.25),
        });
    }

    /// Show the sender card of the player whose name or face is under the pointer
    /// (`hovered`), keep it while the pointer is on the card, else hide it.
    fn follow_card(&mut self, hovered: Option<Shows>) {
        match hovered {
            Some((source, person, anchor)) => match &mut self.card {
                Some(card) if card.source == source => {
                    card.anchor = anchor;
                    card.left = None;
                }
                _ => {
                    self.card = Some(Hovered {
                        source,
                        person,
                        anchor,
                        placed: false,
                        left: None,
                    });
                }
            },
            None if [
                CARD_TOKEN,
                CARD_MUTE_TOKEN,
                CARD_BAN_TOKEN,
                CARD_TRADE_TOKEN,
            ]
            .into_iter()
            .any(|token| self.ui.token_hovered(token)) =>
            {
                if let Some(card) = &mut self.card {
                    card.left = None;
                }
            }
            // Off both: it waits a moment for the pointer on its way to the card.
            None => {
                if let Some(card) = &mut self.card {
                    let left = *card.left.get_or_insert_with(std::time::Instant::now);
                    if left.elapsed() >= CARD_GRACE {
                        self.card = None;
                    }
                }
            }
        }
    }

    /// The sender card on show, beside its name and over the page; its targets last.
    fn card(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        measure: &Measure<'_>,
        viewport: [f32; 2],
    ) {
        let Some(card) = &self.card else {
            self.shown.card_muted = false;
            return;
        };
        let muted = card
            .person
            .key_id
            .as_ref()
            .is_some_and(|key| inputs.muted.contains(key));
        self.shown.card_muted = muted;
        let s = frame.s;
        let size = crate::sender_card::size(&card.person, false, s);
        // A player of the who-is-online window has their card left of the window, over
        // the chat's quiet top, rather than over the window's other faces (text draws
        // over every shape, so a card over them would show their initials through it).
        let origin = match card.source {
            CardSource::Person(_) => [
                (frame.point(PEOPLE_X, 0.0)[0] - 12.0 * s - size[0]).max(0.0),
                card.anchor.y.min(viewport[1] - size[1]).max(0.0),
            ],
            CardSource::Message(_) => {
                crate::sender_card::beside(card.anchor, size, viewport, 12.0 * s)
            }
        };
        // Text draws over every shape: the page's text under the card is left out, or
        // the times and messages showed through it (Sol, 11/10/2026).
        leave_out_text_under(
            &mut self.ui,
            measure,
            sjk_ui::Rect::new(origin[0], origin[1], size[0], size[1]),
        );
        crate::sender_card::draw(
            &mut self.ui,
            &crate::sender_card::Card {
                person: &card.person,
                muted,
                measure: Some(measure),
            },
            origin,
            s,
            crate::sender_card::Tokens {
                card: CARD_TOKEN,
                mute: CARD_MUTE_TOKEN,
                ban: CARD_BAN_TOKEN,
                // The safety code, Report and Whisper are only offered in the game's chat.
                safety: CARD_TOKEN,
                report: CARD_TOKEN,
                whisper: None,
                trade: CARD_TRADE_TOKEN,
                map_block: CARD_TOKEN,
                staff_block: CARD_TOKEN,
            },
        );
    }

    /// The keys of the page, bottom right: the picker's while it is open.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let keys: [(&[&str], &str); 4] = if self.gifs.key_card.is_some() {
            [
                (&["Ctrl", "V"], "paste"),
                (&["Enter"], "save"),
                (&["Tab"], "next"),
                (&["Esc"], "not now"),
            ]
        } else if self.gifs.open && self.gifs.tab == super::gif_picker::Tab::Sources {
            [
                (&["Ctrl", "V"], "paste"),
                (&["Enter"], "activate"),
                (&["Tab"], "next"),
                (&["Esc"], "close GIFs"),
            ]
        } else if self.gifs.open {
            [
                (&["Enter"], "pick"),
                (&["F"], "favourite"),
                (&["PgUp", "PgDn"], "scroll"),
                (&["Esc"], "close GIFs"),
            ]
        } else {
            [
                (&["Enter"], "send"),
                (&["Up", "Down"], "messages"),
                (&["PgUp", "PgDn"], "scroll"),
                (&["Esc"], "back"),
            ]
        };
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(1_824.0, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// Whether the card of the player with `key_id` (wearing `name`) offers Trade: another
/// player's, not blocked here (Sol's decision 1), while the SJK hub lets this player
/// trade.
fn offers_trade(key_id: &str, name: &str) -> bool {
    crate::player_trades::trading_open()
        && !crate::player_trades::blocked(key_id, name)
        && crate::player_identity::own_key_id().as_deref() != Some(key_id)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{chat, inputs, message};
    use super::*;
    use crate::text::{TextStyle, UiFont};
    use sjk_ui::{Color, Rect};

    impl Panel {
        /// [`Panel::build`] measuring in `font` without a text style.
        pub(in crate::console) fn build_with(
            &mut self,
            inputs: &Inputs<'_>,
            font: &UiFont,
            viewport: [f32; 2],
        ) {
            self.build(inputs, &Measure::new(font, TextStyle::NEUTRAL), viewport);
        }
    }

    /// Every text the page drew, with its rectangle and colour.
    fn texts(panel: &Panel) -> Vec<(String, Rect, Color)> {
        panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    rect, text, color, ..
                } => Some((panel.ui.stored_text(*text).to_owned(), *rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// The verified ticks in the messages' column (at 1920 by 1080), the who-is-online
    /// window's left out.
    fn ticks(panel: &Panel) -> usize {
        panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command, DrawCommand::TexturedQuad { texture, rect, .. }
                    if *texture == crate::ui_renderer::VERIFIED_TEXTURE && rect.x < SIDE_X - 30.0)
            })
            .count()
    }

    fn found<'a>(texts: &'a [(String, Rect, Color)], text: &str) -> &'a (String, Rect, Color) {
        texts
            .iter()
            .find(|(drawn, ..)| drawn == text)
            .unwrap_or_else(|| panic!("{text:?} not in {texts:?}"))
    }

    #[test]
    fn a_message_flows_after_its_name_in_gold() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = chat(0);
        let mut short = message(1, "aaaaaaaaaaaaaaaa", "gg ^1all");
        short.name = "^2Sol".to_owned();
        short.verified = true;
        // As long as the hub takes, in wide letters: more than the column's row.
        let long = message(
            2,
            "bbbbbbbbbbbbbbbb",
            &"WMWMWMWM ".repeat(34)[..sjk_identity::chat::TEXT_MAX],
        );
        state.messages.extend([short, long.clone()]);
        let mut panel = Panel::new();
        panel.open(true);
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        let texts = texts(&panel);
        let (_, name, _) = found(&texts, "^2Sol");
        let (_, body, colour) = found(&texts, "gg all");
        // Centred on the same line (the name's text is larger): the name's row.
        assert!((name.y - body.y).abs() < 4.0, "on the name's row");
        assert!(body.x > name.right());
        assert_eq!(*colour, sjk_chat_look::GOLD);
        // The name keeps its codes and is not gold.
        assert_ne!(found(&texts, "^2Sol").2, sjk_chat_look::GOLD);
        // The verified sender has the tick, and no word says so.
        assert_eq!(ticks(&panel), 1);
        assert!(
            texts
                .iter()
                .all(|(text, ..)| !text.to_ascii_lowercase().contains("verified")),
            "{texts:?}"
        );
        // The long one wraps: its rows after the first start at the column's edge.
        let (_, long_name, _) = found(&texts, &long.name);
        let rows: Vec<_> = texts
            .iter()
            .filter(|(_, rect, colour)| *colour == sjk_chat_look::GOLD && rect.y >= long_name.y)
            .collect();
        assert!(rows.len() >= 2, "{rows:?}");
        assert!((rows[0].1.y - long_name.y).abs() < 4.0);
        assert!(rows[1].1.y > rows[0].1.y + 10.0);
        assert!(rows[1].1.x <= long_name.x + 0.5);
    }

    /// A holocron drop reads `name gem sentence` in its tier's colour: no colon, no tick.
    #[test]
    fn a_holocron_drop_is_the_name_a_gem_and_the_sentence_in_the_tiers_colour() {
        use crate::holocrons::TIERS;
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = chat(0);
        let mut drop = message(1, "aaaaaaaaaaaaaaaa", "");
        drop.name = "^2Sol".to_owned();
        drop.verified = true;
        drop.holocron = Some(sjk_identity::DropMark {
            tier: "legendary".to_owned(),
            own: false,
            find: None,
        });
        let mut future = message(2, "bbbbbbbbbbbbbbbb", "");
        future.holocron = Some(sjk_identity::DropMark {
            tier: "from_the_future".to_owned(),
            own: false,
            find: None,
        });
        state.messages.extend([drop, future]);
        let mut panel = Panel::new();
        panel.open(true);
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        let texts = texts(&panel);
        let (_, name, _) = found(&texts, "^2Sol");
        let (_, body, colour) = found(&texts, "found a Legendary Holocron!");
        assert_eq!(*colour, TIERS[2].colour);
        assert!((name.y - body.y).abs() < 4.0 && body.x > name.right());
        assert!(!texts.iter().any(|(text, ..)| text == ":"), "{texts:?}");
        assert_eq!(ticks(&panel), 0, "the gem stands for the tick");
        // A tier this client does not know is still a line, worded plainly.
        found(&texts, "found a holocron.");
    }

    /// A GIPHY link shows "GIF" in the line and the GIF's block under it, inside the
    /// messages' column and over the next message, at 1080p and 4K; a muted sender's
    /// GIF is never asked for; one that could not be had is a quiet line.
    #[test]
    fn a_gif_shows_under_its_line_and_fits_the_column() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = "https://giphy.com/gifs/wave-PageGif01Loading";
        let failed = "https://media.giphy.com/media/PageGif02Failed/giphy.gif";
        let muted_link = "https://i.giphy.com/PageGif03Muted.gif";
        crate::chat_gifs::fail_for_shot(&crate::chat_gifs::GifId::new("PageGif02Failed").unwrap());
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0]] {
            let s = Frame::new(viewport).s;
            let mut state = chat(0);
            state.messages.extend([
                message(1, "cccccccccccccccc", muted_link),
                message(2, "aaaaaaaaaaaaaaaa", &format!("hi {shown}")),
                message(3, "bbbbbbbbbbbbbbbb", failed),
                message(4, "aaaaaaaaaaaaaaaa", "gg"),
                message(
                    7,
                    "dddddddddddddddd",
                    "https://giphy.com/gifs/PageGif04Unverified",
                ),
            ]);
            // Only verified senders' GIFs play: the last stays a link.
            for message in state.messages.iter_mut().take(3) {
                message.verified = true;
            }
            let muted = ["cccccccccccccccc".to_owned()];
            let mut panel = Panel::new();
            panel.open(true);
            panel.build_with(&inputs(&state, &muted, false), &fonts.font, viewport);
            assert!(!panel.ui.overflowed());
            let texts = texts(&panel);
            let (_, label, _) = found(&texts, "hi GIF");
            let (_, loading, _) = found(&texts, "Loading GIF");
            let (_, unavailable, _) = found(&texts, "GIF unavailable");
            let (_, gg, _) = found(&texts, "gg");
            assert!(
                texts
                    .iter()
                    .any(|(text, ..)| text.contains("PageGif04Unverified"))
            );
            // Under its line, in the column, above the next message.
            assert!(loading.y > label.bottom(), "{loading:?} {label:?}");
            assert!(loading.x >= LIST_X * s - 0.5);
            assert!(loading.right() <= (LIST_X + LIST_WIDTH) * s + 0.5);
            assert!(
                loading.bottom() < unavailable.y,
                "{loading:?} {unavailable:?}"
            );
            assert!(unavailable.bottom() < gg.y);
            assert!(gg.bottom() <= LIST_BOTTOM * s + 0.5);
            // The placeholder is the GIF's height.
            let boxes: Vec<Rect> = panel
                .ui
                .draw_list()
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::RoundedRect { rect, .. }
                        if (rect.height - GIF.height * s).abs() < 0.5 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(boxes.len(), 1, "{boxes:?}");
            // The muted sender's line says so, and their GIF was never asked for.
            found(&texts, "Blocked on this PC");
            assert!(!crate::chat_gifs::asked_for(
                &crate::chat_gifs::GifId::new("PageGif03Muted").unwrap()
            ));
            assert!(crate::chat_gifs::asked_for(
                &crate::chat_gifs::GifId::new("PageGif01Loading").unwrap()
            ));
        }
    }

    /// Every focus, long messages, a muted sender, a refusal and staff fit the canvas
    /// at 1080p, 4K, 4:3 and 21:9, in the families and in Inter.
    #[test]
    fn every_state_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let mut crowded = chat(120);
        for id in 121..140 {
            crowded.messages.push_back(message(
                id,
                "aaaaaaaaaaaaaaaa",
                &"long words make many rows here ".repeat(5),
            ));
        }
        crowded.outcome = Some(sjk_identity::ReportOutcome {
            serial: 1,
            sent: false,
            message: "x".repeat(200),
        });
        let muted = ["aaaaaaaaaaaaaaaa".to_owned()];
        let empty = sjk_identity::ChatState::default();
        for (state, staff) in [(&crowded, true), (&empty, false)] {
            let mut panel = Panel::new();
            panel.open(true);
            panel.draft = "d".repeat(sjk_identity::chat::TEXT_MAX);
            panel.selected = Some(130);
            let inputs = inputs(state, &muted, staff);
            panel.build_with(&inputs, &inter.font, [1920.0, 1080.0]);
            for focus in panel.order.clone() {
                for font in [&body.font, &inter.font] {
                    for viewport in [
                        [1920.0, 1080.0],
                        [3840.0, 2160.0],
                        [1440.0, 1080.0],
                        [2560.0, 1080.0],
                    ] {
                        panel.focus = focus;
                        panel.build_with(&inputs, font, viewport);
                        assert!(!panel.ui.overflowed(), "{focus} at {viewport:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_staff_request_says_what_it_came_to() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(3);
        let runs =
            |panel: &Panel| -> Vec<String> { panel.ui.text_runs().map(str::to_owned).collect() };
        // Nothing sent from the page: no line, whatever the staff state holds.
        let old = StaffShown {
            serial: 4,
            message: "Gave bug_hunter",
            failed: false,
            busy: false,
        };
        let quiet = Inputs {
            staff_state: Some(old),
            ..inputs(&state, &[], true)
        };
        panel.build_with(&quiet, &fonts.font, [1920.0, 1080.0]);
        assert!(!runs(&panel).iter().any(|run| run == "Gave bug_hunter"));
        // Sent: waiting, then the hub's answer.
        panel.staff_sent(4);
        let waiting = StaffShown { busy: true, ..old };
        panel.build_with(
            &Inputs {
                staff_state: Some(waiting),
                ..inputs(&state, &[], true)
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(
            runs(&panel)
                .iter()
                .any(|run| run == "Sending to the hub...")
        );
        let refused = StaffShown {
            serial: 5,
            message: "you sent as many staff requests as you may for now; try again later",
            failed: true,
            busy: false,
        };
        panel.build_with(
            &Inputs {
                staff_state: Some(refused),
                ..inputs(&state, &[], true)
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(
            runs(&panel)
                .iter()
                .any(|run| run.starts_with("Not done: you sent as many staff")),
            "{:?}",
            runs(&panel)
        );
    }

    #[test]
    fn the_status_says_why_the_chat_is_silent() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut panel = Panel::new();
        panel.open(true);
        let state = chat(0);
        let mut off = inputs(&state, &[], false);
        off.chat = None;
        panel.build_with(&off, &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("The SJK identity is off"))
        );
        off.enabled = false;
        panel.build_with(&off, &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run.starts_with("SJK chat is off"))
        );
        panel.build_with(&inputs(&state, &[], false), &fonts.font, [1920.0, 1080.0]);
        assert!(
            panel
                .ui
                .text_runs()
                .any(|run| run == "9 SJK players reading")
        );
    }

    fn listed_person(key: &str, name: &str, seen: i64, playing: bool) -> sjk_identity::Person {
        sjk_identity::Person {
            key_id: key.to_owned(),
            name: name.to_owned(),
            seen,
            playing,
            ..sjk_identity::Person::default()
        }
    }

    /// The who-is-online window's rectangle as drawn (its border).
    fn window(panel: &Panel, s: f32) -> Rect {
        panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::Border {
                    rect,
                    radius,
                    width,
                    color,
                } if (*radius - 14.0 * s).abs() < 0.01
                    && (*width - s).abs() < 0.01
                    && *color == color::alpha(color::HOLO, 0.25)
                    && rect.y > 150.0 * s =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the window")
    }

    /// The faces' squares, by token, as the last frame laid them out.
    fn faces(panel: &Panel) -> Vec<Rect> {
        (0..panel.shown.people.len())
            .map(|index| {
                panel
                    .ui
                    .rect_for(PERSON_BASE + index as u16)
                    .expect("a face")
            })
            .collect()
    }

    fn inside(rect: Rect, of: Rect) -> bool {
        rect.x >= of.x
            && rect.right() <= of.right()
            && rect.y >= of.y
            && rect.bottom() <= of.bottom()
    }

    /// Point the pointer at `at` and lay the page out again.
    fn point_at(panel: &mut Panel, inputs: &Inputs<'_>, font: &UiFont, at: [f32; 2]) {
        let _ = panel.handle_pointer(sjk_ui::InputEvent::PointerMove(sjk_ui::Vec2::new(
            at[0], at[1],
        )));
        panel.build_with(inputs, font, [1920.0, 1080.0]);
    }

    /// The online players as a crowd of `count`, Sol first (verified, with a picture,
    /// playing), then Zed (playing) and the others; two seen recently.
    fn crowd(count: usize) -> sjk_identity::ChatState {
        let mut state = chat(3);
        let now = 1_791_400_000_i64;
        let mut online = vec![
            sjk_identity::Person {
                verified: true,
                avatar: "0123456789abcdef".to_owned(),
                ..listed_person(&format!("{:016x}", 2), "^2Sol", now, true)
            },
            listed_person("aaaaaaaaaaaaaaaa", "^1Zed", now, true),
        ];
        for n in 2..count {
            online.push(listed_person(
                &format!("{:016x}", 0x100 + n),
                &format!("Player {n:02}"),
                now,
                false,
            ));
        }
        online.truncate(count);
        state.people = Some(sjk_identity::People {
            online,
            recent: vec![
                listed_person("ffffffffffffffff", "Bastila", now - 180, false),
                listed_person("1111111111111111", "^5jan", now - 7_200, false),
            ],
        });
        state
    }

    /// The window shows who is online as faces in a grid (playing first, the last face
    /// "+N" for the rest) and who was seen last as smaller ones, inside the window and
    /// with no time written; each face shows its initial until its picture loads, and
    /// a gold dot for a player in a match.
    #[test]
    fn the_window_shows_faces_in_a_grid_with_a_last_one_for_the_rest() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = crowd(29);
        let mut panel = Panel::new();
        panel.open(true);
        let inputs = inputs(&state, &[], false);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Online  \u{b7}  9");
        found(&drawn, "Recently active");
        // Twenty-nine online in twenty-seven faces: twenty-six, then "+3".
        let online = super::people::ONLINE_FACES - 1;
        assert_eq!(panel.shown.people.len(), online + 2, "and two recent");
        let (_, more, _) = found(&drawn, "+3");
        let shown = faces(&panel);
        let window = window(&panel, 1.0);
        for face in shown.iter().chain([more]) {
            assert!(inside(*face, window), "{face:?} in {window:?}");
        }
        assert!(inside(*more, window));
        // Nine across, the tenth under the first; the "+3" in the last cell.
        let (first, tenth) = (shown[0], shown[9]);
        assert_eq!((first.width, first.height), (40.0, 40.0));
        assert!((tenth.x - first.x).abs() < 0.01 && (tenth.y - first.y - 50.0).abs() < 0.01);
        assert!((shown[1].x - first.x - 50.0).abs() < 0.01);
        assert!((more.x - shown[8].x).abs() < 0.01);
        let third_row = shown[18];
        assert!(
            more.y >= third_row.y - 1.0 && more.y < third_row.bottom(),
            "{more:?}"
        );
        // No two faces touch.
        for (index, a) in shown.iter().enumerate() {
            for b in &shown[index + 1..] {
                assert!(
                    a.right() < b.x || b.right() < a.x || a.bottom() < b.y || b.bottom() < a.y,
                    "{a:?} and {b:?}"
                );
            }
        }
        // Recent faces are smaller, under their heading, under the online ones.
        let recent = shown[online];
        assert_eq!((recent.width, recent.height), (30.0, 30.0));
        let (_, heading, _) = found(&drawn, "Recently active");
        assert!(heading.y > tenth.bottom() && recent.y > heading.bottom() - 8.0);
        // Their initials, colour codes aside, centred on their faces.
        let (_, s_letter, _) = drawn
            .iter()
            .find(|(text, rect, _)| text == "S" && inside(*rect, window))
            .expect("Sol's initial");
        assert!((s_letter.x + s_letter.width * 0.5 - (first.x + 20.0)).abs() < 0.5);
        assert!(
            drawn
                .iter()
                .any(|(text, rect, _)| text == "J" && inside(*rect, window))
        );
        // Gold dots for the two in a match, and nothing written of when anyone was seen.
        let dots = panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command, DrawCommand::RoundedRect { color, rect, .. }
                    if *color == color::GOLD_BRIGHT && inside(*rect, window))
            })
            .count();
        assert_eq!(dots, 2);
        assert!(
            !drawn.iter().any(|(text, rect, _)| inside(*rect, window)
                && (text.contains("ago") || text == "playing")),
            "{drawn:?}"
        );
        // The chosen message's heading under the window.
        let (_, chosen, _) = found(&drawn, "Chosen message");
        assert!(chosen.y > window.bottom());
    }

    /// The window is as tall as what it shows, so Chosen message comes up under a short
    /// one; every face stays in the window and on screen at 1080p, 4K, 4:3 and 21:9.
    #[test]
    fn the_window_shrinks_to_its_faces_and_fits_every_screen() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let chosen_at = |state: &sjk_identity::ChatState| {
            let mut panel = Panel::new();
            panel.open(true);
            panel.build_with(&inputs(state, &[], false), &fonts.font, [1920.0, 1080.0]);
            let (_, chosen, _) = found(&texts(&panel), "Chosen message").clone();
            (window(&panel, 1.0), chosen)
        };
        let (full, full_chosen) = chosen_at(&crowd(30));
        let (one_row, one_row_chosen) = chosen_at(&crowd(5));
        assert!(
            (full.height - one_row.height - 100.0).abs() < 0.01,
            "two rows less"
        );
        assert!((full_chosen.y - one_row_chosen.y - 100.0).abs() < 0.01);
        // Three rows since 08/11/2026: 50 taller than the two before (bottom 522), Chosen
        // under one row still higher than under the window of names (at 556).
        assert!(full.bottom() < 572.0 && one_row_chosen.y < 556.0 - 15.0 - 100.0);
        let state = crowd(30);
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(3);
        let staff = inputs(&state, &[], true);
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
            [2560.0, 1440.0],
            [1024.0, 768.0],
        ] {
            panel.build_with(&staff, &fonts.font, viewport);
            assert!(!panel.ui.overflowed(), "{viewport:?}");
            let s = Frame::new(viewport).s;
            let window = window(&panel, s);
            let screen = Rect::new(0.0, 0.0, viewport[0], viewport[1]);
            assert!(inside(window, screen), "{viewport:?}");
            for face in faces(&panel) {
                assert!(
                    inside(face, window),
                    "{face:?} in {window:?} at {viewport:?}"
                );
                // A whole number of pixels across at 1080p and 4K: crisp discs.
                if viewport == [3840.0, 2160.0] {
                    assert!(face.width == 80.0 || face.width == 60.0, "{face:?}");
                }
            }
            // The chosen message's tools under the window and above the keys.
            let mute = panel.ui.rect_for(MUTE_TOKEN).expect("Mute");
            let unmute = panel
                .ui
                .rect_for(HUB_UNMUTE_TOKEN)
                .expect("Unmute at the hub");
            let keys = Frame::new(viewport).point(0.0, KEYS_Y - 20.0)[1];
            assert!(
                mute.y > window.bottom() && unmute.bottom() < keys,
                "{viewport:?}"
            );
        }
    }

    /// List turns the window into a row per online player, over the chosen message and
    /// down to the keys: their name and, on the right, the icons of the clients they
    /// read the chat with; the wheel scrolls the rows that do not fit; Faces, Escape or
    /// choosing a message brings the faces back.
    #[test]
    fn the_list_shows_every_online_player_with_their_clients() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = crowd(30);
        if let Some(people) = state.people.as_mut() {
            people.online[0].clients = vec!["sjk".to_owned(), "launcher".to_owned()];
            people.online[5].clients = vec!["ejk".to_owned()];
        }
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(3);
        let inputs = inputs(&state, &[], false);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        assert!(panel.order.contains(&LIST_TOKEN), "List on the heading");
        found(&texts(&panel), "List");
        assert_eq!(panel.activate(LIST_TOKEN), PanelAction::None);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Faces");
        assert!(
            !drawn.iter().any(|(text, ..)| text == "Chosen message"),
            "the list stands where the chosen message goes"
        );
        found(&drawn, "^2Sol");
        // Thirty online, twenty rows and the line that says where the list is.
        let rows = panel.shown.people.len();
        assert_eq!(rows, 20);
        found(&drawn, "1 to 20 of 30  \u{b7}  the wheel scrolls");
        let textures: Vec<sjk_ui::TextureId> = panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::TexturedQuad { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        for icon in [
            crate::ui_renderer::LOGO_TEXTURE,
            crate::ui_renderer::LAUNCHER_TEXTURE,
            crate::ui_renderer::EJK_TEXTURE,
        ] {
            assert!(textures.contains(&icon), "{icon:?}");
        }
        // The rows inside the window, at every screen shape, above the keys.
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1024.0, 768.0],
            [2560.0, 1080.0],
        ] {
            panel.build_with(&inputs, &fonts.font, viewport);
            assert!(!panel.ui.overflowed(), "{viewport:?}");
            let s = Frame::new(viewport).s;
            let window = window(&panel, s);
            let keys = Frame::new(viewport).point(0.0, KEYS_Y - 20.0)[1];
            assert!(window.bottom() < keys, "{viewport:?}");
            for row in faces(&panel) {
                assert!(inside(row, window), "{row:?} in {window:?} at {viewport:?}");
            }
        }
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        // The wheel over the rows scrolls them, no further than the last.
        let rows_rect = panel.shown.list_rect.expect("the rows");
        let over = sjk_ui::Vec2::new(rows_rect.x + 40.0, rows_rect.y + 40.0);
        let _ = panel.handle_pointer(sjk_ui::InputEvent::PointerMove(over));
        let wheel = |down: bool| sjk_ui::InputEvent::PointerWheel {
            position: over,
            delta: sjk_ui::Vec2::new(0.0, if down { -1.0 } else { 1.0 }),
        };
        let scroll = panel.scroll;
        let _ = panel.handle_pointer(wheel(true));
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        found(&texts(&panel), "2 to 21 of 30  \u{b7}  the wheel scrolls");
        assert_eq!(panel.scroll, scroll, "the messages stay");
        for _ in 0..40 {
            let _ = panel.handle_pointer(wheel(true));
        }
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        found(&texts(&panel), "11 to 30 of 30  \u{b7}  the wheel scrolls");
        // A click on a row leaves the list; Escape closes it, then the page.
        assert_eq!(panel.activate(PERSON_BASE), PanelAction::None);
        assert!(panel.people_list);
        assert_eq!(
            panel.edit(KeyCode::Escape, None, false, false),
            PanelAction::None
        );
        assert!(!panel.people_list);
        // Choosing a message closes it too.
        let _ = panel.activate(LIST_TOKEN);
        assert!(panel.people_list);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let _ = panel.activate(MESSAGE_BASE);
        assert!(!panel.people_list);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        found(&texts(&panel), "Chosen message");
    }

    /// A chosen message with a GIF, as staff, on a full window: the GIF row (favourites
    /// and Hide this GIF), Manage GIF source and Staff's buttons all stand above the keys,
    /// none over another, at every screen shape.
    #[test]
    fn a_gif_message_and_staff_fit_above_the_keys() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut state = crowd(30);
        state.messages[2].text = "look https://giphy.com/gifs/wave-PanelFit01xyz".to_owned();
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(3);
        let staff = inputs(&state, &[], true);
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
            [1024.0, 768.0],
        ] {
            panel.build_with(&staff, &fonts.font, viewport);
            let keys = Frame::new(viewport).point(0.0, KEYS_Y - 20.0)[1];
            let tokens = [
                MUTE_TOKEN,
                FAVOURITE_TOKEN,
                HIDE_GIF_TOKEN,
                super::gif_sources::OPEN,
                DELETE_TOKEN,
                HUB_MUTE_TOKEN,
                HUB_UNMUTE_TOKEN,
            ];
            let rects: Vec<Rect> = tokens
                .iter()
                .map(|token| {
                    panel
                        .ui
                        .rect_for(*token)
                        .unwrap_or_else(|| panic!("{token}"))
                })
                .collect();
            for (index, rect) in rects.iter().enumerate() {
                assert!(
                    rect.bottom() < keys,
                    "{} under the keys at {viewport:?}",
                    tokens[index]
                );
                for (other, second) in rects.iter().enumerate().skip(index + 1) {
                    let apart = rect.right() <= second.x
                        || second.right() <= rect.x
                        || rect.bottom() <= second.y
                        || second.bottom() <= rect.y;
                    assert!(
                        apart,
                        "{} over {} at {viewport:?}",
                        tokens[index], tokens[other]
                    );
                }
            }
        }
    }

    /// A face under the pointer shows that player's card, left of the window, with when
    /// they were about; the gap between two faces shows none; a click chooses their
    /// newest message; a muted player's face is greyed.
    #[test]
    fn a_face_under_the_pointer_shows_its_card() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = crowd(5);
        let muted = ["aaaaaaaaaaaaaaaa".to_owned()];
        let mut panel = Panel::new();
        panel.open(true);
        let inputs = inputs(&state, &muted, false);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let shown = faces(&panel);
        let centre = |rect: Rect| [rect.x + rect.width * 0.5, rect.y + rect.height * 0.5];
        // Sol's face: their card, with the picture the hub listed and that they play.
        point_at(&mut panel, &inputs, &fonts.font, centre(shown[0]));
        let card = panel.card.as_ref().expect("the card");
        assert_eq!(card.source, CardSource::Person(format!("{:016x}", 2)));
        assert_eq!(card.person.avatar.as_deref(), Some("0123456789abcdef"));
        assert_eq!(card.person.note.as_deref(), Some("Online now, in a match"));
        assert!(card.person.verified);
        let target = panel.ui.rect_for(CARD_TOKEN).expect("the card's target");
        assert!(
            target.right() <= window(&panel, 1.0).x,
            "left of the window: {target:?}"
        );
        assert!(found(&texts(&panel), "Online now, in a match").1.right() <= target.right());
        // Lit: a gold ring round it.
        assert!(panel.ui.draw_list().commands().iter().any(|command| matches!(
            command,
            DrawCommand::Arc { color, center, .. }
                if *color == color::GOLD_BRIGHT && (center[0] - centre(shown[0])[0]).abs() < 0.01
        )));
        point_at(
            &mut panel,
            &inputs,
            &fonts.font,
            [shown[0].right() + 5.0, centre(shown[0])[1]],
        );
        // Between two faces: the card waits a moment for the pointer, then goes.
        assert!(panel.card.is_some(), "kept on the way");
        if let Some(card) = panel.card.as_mut() {
            card.left = card.left.map(|at| at - super::CARD_GRACE);
        }
        point_at(
            &mut panel,
            &inputs,
            &fonts.font,
            [shown[0].right() + 5.0, centre(shown[0])[1]],
        );
        assert!(panel.card.is_none());
        // A recent face: when they were seen, on the card only.
        let recent = shown[5];
        point_at(&mut panel, &inputs, &fonts.font, centre(recent));
        let card = panel.card.as_ref().expect("the recent card");
        assert_eq!(card.person.name, "Bastila");
        assert_eq!(card.person.note.as_deref(), Some("Seen 3 minutes ago"));
        found(&texts(&panel), "Seen 3 minutes ago");
        // Zed is muted here: a grey disc, and Unmute on the card.
        let zed = shown[1];
        assert!(
            panel
                .ui
                .draw_list()
                .commands()
                .iter()
                .any(|command| matches!(
                    command,
                    DrawCommand::RoundedRect { color, rect, .. }
                        if *color == color::alpha(color::QUIET, 0.22) && *rect == zed
                ))
        );
        // A click chooses their newest message on show; someone who said nothing, none.
        assert_eq!(panel.activate(PERSON_BASE), PanelAction::None);
        assert_eq!(panel.selected, Some(2));
        panel.selected = None;
        let _ = panel.activate(PERSON_BASE + 1);
        assert_eq!(panel.selected, None);
    }

    /// A sender's card over the history leaves out the text it covers (text draws over
    /// every shape, so the times and messages showed through it, Sol 11/10/2026); the
    /// rest of the page keeps its text.
    #[test]
    fn a_senders_card_leaves_out_the_text_it_covers() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = chat(12);
        let mut panel = Panel::new();
        panel.open(true);
        let inputs = inputs(&state, &[], false);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let name = panel
            .ui
            .rect_for(NAME_BASE + 6)
            .expect("a name in the middle of the history");
        let before = texts(&panel).len();
        point_at(
            &mut panel,
            &inputs,
            &fonts.font,
            [name.x + 4.0, name.y + name.height * 0.5],
        );
        assert!(matches!(
            panel.card.as_ref().map(|card| &card.source),
            Some(CardSource::Message(_))
        ));
        let card = panel.ui.rect_for(CARD_TOKEN).expect("the card's target");
        let shown = texts(&panel);
        let history: Vec<_> = (1..=12).map(|id| format!("message {id}")).collect();
        let page_text = |text: &str| {
            history.iter().any(|line| line == text) || text.ends_with(" ago") || text == "just now"
        };
        let under: Vec<_> = shown
            .iter()
            .filter(|(text, rect, _)| rect.overlaps(card) && page_text(text))
            .collect();
        assert!(under.is_empty(), "page text under the card: {under:?}");
        assert!(
            shown.iter().any(|(text, ..)| history.contains(text)),
            "the history away from the card still shows"
        );
        assert!(shown.len() < before + 20, "nothing doubled");
    }

    /// When the faces change under a pointer that does not move, what it rests on is
    /// looked for again: the targets after the faces moved in the frame's list, and
    /// without it the field's place would light a face and show its card.
    #[test]
    fn faces_changing_under_a_resting_pointer_do_not_light_another_target() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let (few, many) = (crowd(5), crowd(20));
        let mut panel = Panel::new();
        panel.open(true);
        panel.build_with(&inputs(&few, &[], false), &fonts.font, [1920.0, 1080.0]);
        let field = panel.ui.rect_for(FIELD_TOKEN).expect("the field");
        let at = [field.x + 10.0, field.y + field.height * 0.5];
        point_at(&mut panel, &inputs(&few, &[], false), &fonts.font, at);
        assert!(panel.card.is_none());
        for _ in 0..3 {
            panel.build_with(&inputs(&many, &[], false), &fonts.font, [1920.0, 1080.0]);
            assert!(
                panel.card.is_none(),
                "{:?}",
                panel.card.as_ref().map(|card| &card.source)
            );
        }
        // Resting on a face while the list grows: the face now under it.
        let first = faces(&panel)[0];
        point_at(
            &mut panel,
            &inputs(&many, &[], false),
            &fonts.font,
            [first.x + 20.0, first.y + 20.0],
        );
        assert!(panel.card.is_some());
        for _ in 0..2 {
            panel.build_with(&inputs(&few, &[], false), &fonts.font, [1920.0, 1080.0]);
        }
        let card = panel.card.as_ref().expect("still a card");
        assert_eq!(card.source, CardSource::Person(format!("{:016x}", 2)));
    }

    /// A hub that does not list who is online: the count, and the last senders' faces,
    /// newest first; their pictures are looked up after the frame, and their card says
    /// when they last spoke.
    #[test]
    fn without_a_list_the_window_shows_the_count_and_the_last_senders() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let state = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        let inputs = inputs(&state, &[], false);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Online  \u{b7}  9");
        found(&drawn, "This hub counts them but does not say who.");
        found(&drawn, "Recently in chat");
        let keys: Vec<String> = [3, 2, 1].iter().map(|id| format!("{id:016x}")).collect();
        assert_eq!(panel.shown.people, keys, "newest first");
        let shown = faces(&panel);
        assert!(shown.windows(2).all(|pair| pair[0].x < pair[1].x));
        assert_eq!(shown[0].width, 30.0, "recent faces");
        // Their pictures are not known: asked after the frame, kept once found.
        assert_eq!(panel.shown.unpictured, keys);
        let asked = std::cell::RefCell::new(Vec::new());
        panel.find_pictures(|key| {
            asked.borrow_mut().push(key.to_owned());
            (key == keys[1]).then(|| "0123456789abcdef".to_owned())
        });
        assert_eq!(*asked.borrow(), keys);
        panel.build_with(&inputs, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.shown.unpictured, [keys[0].clone(), keys[2].clone()]);
        let centre = |rect: Rect| [rect.x + rect.width * 0.5, rect.y + rect.height * 0.5];
        point_at(&mut panel, &inputs, &fonts.font, centre(shown[1]));
        let card = panel.card.as_ref().expect("the card");
        assert_eq!(card.person.avatar.as_deref(), Some("0123456789abcdef"));
        // Player 2 spoke at 1_791_300_120, the page's now is 1_791_400_000.
        assert_eq!(card.person.note.as_deref(), Some("Last spoke yesterday"));
        // Not reading: the window says why it is empty, and is only that tall.
        let mut quiet = state.clone();
        quiet.live = false;
        panel.build_with(&inputs_for(&quiet), &fonts.font, [1920.0, 1080.0]);
        let drawn = texts(&panel);
        found(&drawn, "Shows once the SJK hub answers.");
        found(&drawn, "Online");
        assert!(!drawn.iter().any(|(text, ..)| text == "Recently active"));
        assert!(panel.shown.people.is_empty());
    }

    fn inputs_for(chat: &sjk_identity::ChatState) -> Inputs<'_> {
        inputs(chat, &[], false)
    }
}
