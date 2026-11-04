//! The Profile page's drawing, in the SJK UI's look over the map whatever the menu
//! style: the top bar, or on the Profile screen its title and row of tabs
//! ([`crate::profile_hub::header`]) with the page moved down under them; the player's
//! picture, who they are (their name emoji after their name) and their record on the
//! left, the bio (or the picture or emoji panel) in the middle and their medals (on its own only), achievements and collection on the
//! right, each with the way to the Collection.

use super::*;
use crate::menu::sjk::{
    Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar, wrap,
};
use crate::menu_widgets::TextFamily;
use crate::text::{TextStyle, UiFont};
use crate::text_dialog::field::Face;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The columns of the Profile tab (frame pixels).
const LEFT_X: f32 = 96.0;
const LEFT_WIDTH: f32 = 520.0;
const MIDDLE_X: f32 = 680.0;
const MIDDLE_WIDTH: f32 = 620.0;
const RIGHT_X: f32 = 1_364.0;
const RIGHT_WIDTH: f32 = 460.0;
/// Where the columns start, under the top bar.
const TOP: f32 = 170.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;
/// The bio's box: its top, height, inner padding and line.
const BIO_TOP: f32 = 216.0;
const BIO_HEIGHT: f32 = 300.0;
const BIO_PAD: f32 = 20.0;
const BIO_LINE: f32 = 26.0;
const BIO_SIZE: f32 = 18.0;
/// Most medals the Profile tab lists.
const MEDALS_SHOWN: usize = 3;
/// Most unlocks the Profile tab lists.
const UNLOCKS_SHOWN: usize = 4;
/// The player's picture at the top of the left column, and the text beside it.
const PICTURE_SIZE: f32 = 104.0;
const WHO_X: f32 = LEFT_X + PICTURE_SIZE + 24.0;
const WHO_WIDTH: f32 = LEFT_WIDTH - PICTURE_SIZE - 24.0;
/// The picture panel's large picture.
const LARGE_PICTURE: f32 = 220.0;
/// The emoji panel's grid: a cell's side and the gap between cells.
const EMOJI_CELL: f32 = 56.0;
const EMOJI_GAP: f32 = 6.0;

/// What the page says when the player holds no medal yet.
const NO_MEDALS: &str =
    "No medals yet. The SJK team gives medals for testing, contributing and more.";

/// Who the player is, as the Profile tab's left column says it.
struct Who {
    headline: String,
    lines: Vec<(String, Color)>,
    /// Their key id and picture's version, empty when the hub has not answered.
    key_id: String,
    avatar: String,
    verified: bool,
    /// Their tick, as the hub sent it.
    tick: sjk_identity::Tick,
    /// Their trust mark (`ui_renderer::mark_texture`), none when verified.
    trust: Option<sjk_identity::Trust>,
}

fn who(inputs: &Inputs<'_>) -> Who {
    let plain = |headline: &str, lines: &[&str]| Who {
        headline: headline.to_owned(),
        lines: lines
            .iter()
            .map(|line| ((*line).to_owned(), color::MUTED))
            .collect(),
        key_id: String::new(),
        avatar: String::new(),
        verified: false,
        tick: sjk_identity::Tick::None,
        trust: None,
    };
    if !inputs.enabled {
        return plain(
            "Identity is off",
            &[
                "Your profile lives on the SJK hub. Switch the identity on in Settings, Network, to show it to other players.",
                "Your achievements are counted on this PC meanwhile.",
            ],
        );
    }
    let Some(snapshot) = inputs.snapshot else {
        return plain("Starting...", &["Preparing your identity."]);
    };
    match &snapshot.status {
        Status::Disabled => plain("Identity is off", &["The settings were just changed."]),
        Status::NoHub => plain(
            "No hub is set",
            &["Set cl_hubUrl, or use the official hub: Settings, Network, SJK identity key."],
        ),
        Status::Registering => plain("Contacting the hub...", &[]),
        Status::Failed(error) => Who {
            headline: "Cannot reach the hub".to_owned(),
            lines: vec![
                (error.clone(), color::MUTED),
                ("Retrying automatically.".to_owned(), color::QUIET),
            ],
            key_id: snapshot.key_id.clone(),
            avatar: snapshot
                .me
                .as_ref()
                .map(|me| me.avatar.clone())
                .unwrap_or_default(),
            verified: false,
            tick: sjk_identity::Tick::None,
            trust: None,
        },
        Status::Online => {
            let Some(me) = &snapshot.me else {
                return plain("Registered", &[]);
            };
            let mut lines = Vec::new();
            let since = crate::medals::date_text(me.created);
            use sjk_identity::Trust;
            let badge = match (me.verified, me.trust) {
                (true, _) => ("Verified by the SJK team".to_owned(), color::GOLD_BRIGHT),
                (false, Some(Trust::New)) => (
                    "New: green after 30 days and 10 hours of play".to_owned(),
                    color::QUIET,
                ),
                (false, Some(Trust::Trusted)) => (
                    "Trusted: the SJK team may verify you".to_owned(),
                    color::TRUSTED,
                ),
                (false, Some(Trust::Banned)) => (
                    "Banned from the SJK chat: your 30 days start again".to_owned(),
                    color::EMBER,
                ),
                (false, _) => ("Not verified yet".to_owned(), color::QUIET),
            };
            lines.push(badge);
            if !since.is_empty() {
                lines.push((format!("Member since {since}"), color::MUTED));
            }
            let earlier: Vec<&str> = me
                .names
                .iter()
                .map(|worn| worn.name.as_str())
                .filter(|name| {
                    sjk_identity::normal_form(name) != sjk_identity::normal_form(&me.name)
                })
                .take(3)
                .collect();
            if !earlier.is_empty() {
                lines.push((
                    format!("Also known as {}", earlier.join("^7, ")),
                    color::MUTED,
                ));
            }
            Who {
                headline: if me.name.is_empty() {
                    "Registered".to_owned()
                } else {
                    me.name.clone()
                },
                lines,
                key_id: me.key_id.clone(),
                avatar: me.avatar.clone(),
                verified: me.verified,
                tick: me.tick,
                trust: me.trust,
            }
        }
    }
}

/// `text` cut to `chars` characters with an ellipsis, colour codes counted as text.
fn cut(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_owned();
    }
    let kept: String = text.chars().take(chars.saturating_sub(3)).collect();
    format!("{kept}...")
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        // The fonts outlive the target's vertex lists: measure with them, then append.
        // The bio is drawn in the player's menu text style, which its caret measures with.
        let (body, style) = match &target {
            TextTarget::Families(fonts, style) => (fonts.body.1, *style),
            TextTarget::Inter(_, font) => (*font, font.style()),
        };
        self.build(inputs, body, style, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out, measuring the bio's lines in `body` drawn in `style`.
    pub(super) fn build(
        &mut self,
        inputs: &Inputs<'_>,
        body: &UiFont,
        style: TextStyle,
        viewport: [f32; 2],
    ) {
        self.sync(inputs);
        let frame = Frame::new(viewport);
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        match self.mode {
            Mode::Pages => top_bar(&mut self.ui, &frame, "Back", BACK_TOKEN, "Profile", None),
            Mode::Hub => crate::profile_hub::header(
                &mut self.ui,
                &frame,
                &self.hub_header,
                BACK_TOKEN,
                HubTab::Profile,
            ),
        }
        // Under the Profile screen's row of tabs the page moves down.
        let page = frame.shifted(0.0, self.shift());
        let who = who(inputs);
        self.left_column(&page, inputs, &who);
        match self.middle {
            Middle::Bio => self.bio_column(&page, body, style),
            Middle::Picture => self.picture_column(&page, &who),
            Middle::Emoji => self.emoji_column(&page, &who),
        }
        self.right_column(&page, inputs);
        self.keys(&frame);
        self.ui.finish(self.focus_token());
    }

    /// How far the page is moved down: under the Profile screen's row of tabs.
    fn shift(&self) -> f32 {
        if self.mode == Mode::Hub {
            crate::profile_hub::SHIFT
        } else {
            0.0
        }
    }

    /// The last frame line the page's own parts may reach, in their moved frame: over
    /// the keys.
    pub(super) fn bottom(&self) -> f32 {
        KEYS_Y - 20.0 - self.shift()
    }

    fn left_column(&mut self, frame: &Frame, inputs: &Inputs<'_>, who: &Who) {
        let profile = inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.me.as_ref());
        let s = frame.s;
        // The neutral cover draws nothing behind the head; a worn banner, round corners.
        let cover = crate::profile_card::banner::equipped(profile);
        if !crate::profile_card::banner::plain(cover) {
            crate::profile_card::banner::draw_rounded(
                &mut self.ui,
                frame.rect(LEFT_X - 12.0, TOP - 12.0, LEFT_WIDTH + 24.0, 148.0),
                cover,
                12.0 * s,
            );
        }
        // The player's picture: a click (or Enter) opens the picture panel.
        let lit = self.focus == Focus::Picture || self.ui.token_hovered(PICTURE_TOKEN);
        let radius = PICTURE_SIZE * 0.5;
        crate::profile_card::avatar(
            &mut self.ui,
            frame.point(LEFT_X + radius, TOP + radius),
            radius * s,
            &crate::profile_card::Avatar {
                key_id: &who.key_id,
                version: &who.avatar,
                name: &who.headline,
                verified: who.verified,
                tick: who.tick,
                trust: who.trust,
                preview: false,
                lit,
            },
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}",
                if self.has_picture {
                    "Change picture"
                } else {
                    "Add a picture"
                }
            ),
            frame.rect(
                LEFT_X - 8.0,
                TOP + PICTURE_SIZE + 8.0,
                PICTURE_SIZE + 16.0,
                22.0,
            ),
            14.0 * s,
            if lit {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Center,
        );
        self.ui.hit_region(
            PICTURE_TOKEN,
            frame.rect(
                LEFT_X - 8.0,
                TOP - 4.0,
                PICTURE_SIZE + 16.0,
                PICTURE_SIZE + 38.0,
            ),
        );
        // The name emoji after the name, as everyone sees it.
        let emoji = if self.writable {
            self.emoji.as_str()
        } else {
            ""
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!(
                "{}{}{emoji}",
                cut(&who.headline, 28),
                if emoji.is_empty() { "" } else { "^7 " }
            ),
            frame.rect(WHO_X, TOP, WHO_WIDTH, 50.0),
            38.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut y = TOP + 56.0;
        for (line, colour) in &who.lines {
            for part in wrap(line, 44).take(3) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(WHO_X, y, WHO_WIDTH, 26.0),
                    17.0 * s,
                    *colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 26.0;
            }
        }
        y = y.max(TOP + PICTURE_SIZE + 38.0) + 8.0;
        let emoji_label = if self.emoji.is_empty() {
            "Add a name emoji".to_owned()
        } else {
            format!("Name emoji {}", self.emoji)
        };
        // On the Profile screen, Change name first: it opens the Character tab's Name
        // field, the name the hub shows being the one worn in game.
        let mut x = LEFT_X;
        if self.mode == Mode::Hub {
            kit::button(
                &mut self.ui,
                frame,
                [x, y, 150.0, 42.0],
                "Change name",
                false,
                true,
                self.focus == Focus::Name,
                NAME_TOKEN,
            );
            x += 162.0;
        }
        let emoji_width = if self.mode == Mode::Hub { 210.0 } else { 230.0 };
        kit::button(
            &mut self.ui,
            frame,
            [x, y, emoji_width, 42.0],
            &emoji_label,
            false,
            self.writable,
            self.focus == Focus::Emoji || self.middle == Middle::Emoji,
            EMOJI_TOKEN,
        );
        x += emoji_width + 16.0;
        if self.staff {
            let width = (LEFT_X + LEFT_WIDTH - x).min(180.0);
            kit::button(
                &mut self.ui,
                frame,
                [x, y, width, 42.0],
                "Staff tools",
                true,
                true,
                self.focus == Focus::Staff,
                STAFF_TOKEN,
            );
        }
        y += 42.0;
        y += 34.0;
        kit::heading(&mut self.ui, frame, LEFT_X, y, LEFT_WIDTH, "Your record");
        y += 30.0;
        for (index, (label, value)) in inputs.record.iter().take(8).enumerate() {
            let x = LEFT_X + (index % 2) as f32 * 260.0;
            let row_y = y + (index / 2) as f32 * 74.0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{value}"),
                frame.rect(x, row_y, 250.0, 40.0),
                34.0 * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(x, row_y + 40.0, 250.0, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = y + 4.0 * 74.0 + 14.0;
        let mut unlocked: Vec<&Standing> = inputs
            .standings
            .iter()
            .filter(|standing| standing.unlocked.is_some())
            .collect();
        unlocked.sort_by_key(|standing| std::cmp::Reverse(standing.unlocked.unwrap_or(0)));
        let bottom = self.bottom();
        if unlocked.is_empty() || y + 60.0 > bottom {
            return;
        }
        kit::heading(
            &mut self.ui,
            frame,
            LEFT_X,
            y,
            LEFT_WIDTH,
            "Unlocked lately",
        );
        y += 26.0;
        for standing in unlocked.into_iter().take(UNLOCKS_SHOWN) {
            if y + 28.0 > bottom {
                break;
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", crate::terms::say(standing.kind.name)),
                frame.rect(LEFT_X, y, LEFT_WIDTH - 140.0, 28.0),
                21.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}",
                    crate::medals::date_text(standing.unlocked.unwrap_or(0))
                ),
                frame.rect(LEFT_X + LEFT_WIDTH - 140.0, y, 140.0, 28.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            y += 30.0;
        }
    }

    /// The bio's lines in its box, scrolled to keep the caret in view, with the selection
    /// lit and the caret (`caret`: lit this frame) between two glyphs. It lays the bio
    /// out for the keys and clicks of the next frame ([`FieldLayout`]).
    fn bio_text(&mut self, frame: &Frame, body: &UiFont, style: TextStyle, caret: bool) {
        let s = frame.s;
        let room = MIDDLE_WIDTH - BIO_PAD * 2.0;
        let most = ((BIO_HEIGHT - BIO_PAD * 2.0) / BIO_LINE) as usize;
        let row = |line: usize| {
            frame.rect(
                MIDDLE_X + BIO_PAD,
                BIO_TOP + BIO_PAD + line as f32 * BIO_LINE,
                room,
                BIO_LINE,
            )
        };
        let face = Face::new(body, style, BIO_SIZE * s, 0.0);
        self.bio_layout.lay_out(&self.bio, &face, room * s);
        let at = self.bio_edit.cursor(&self.bio);
        let (caret_line, caret_x) = self.bio_layout.locate(&self.bio, at);
        let top = row(0);
        self.bio_layout
            .show(caret_line, most, [top.x, top.y], row(1).y - top.y);
        let (first, shown) = self.bio_layout.visible();
        let ink = if self.writable {
            color::TEXT
        } else {
            color::MUTED
        };
        let selection = self.bio_edit.selection(&self.bio);
        for (index, range) in shown.iter().enumerate() {
            let rect = row(index);
            if let Some(selected) = &selection {
                let start = selected.start.max(range.start);
                let end = selected.end.min(range.end);
                if start < end {
                    let x0 = self.bio_layout.locate(&self.bio, start).1;
                    let x1 = self.bio_layout.locate(&self.bio, end).1;
                    let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                        rect: sjk_ui::Rect::new(rect.x + x0, rect.y, x1 - x0, rect.height),
                        color: color::alpha(color::HOLO, 0.35),
                    });
                }
            }
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", &self.bio[range.clone()]),
                rect,
                BIO_SIZE * s,
                ink,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        if caret && caret_line >= first && caret_line < first + shown.len() {
            let line = row(caret_line - first);
            let height = (BIO_SIZE * 1.3 * style.scale).min(BIO_LINE - 2.0) * s;
            let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                rect: sjk_ui::Rect::new(
                    line.x + caret_x - 1.0 * s,
                    line.y + (line.height - height) * 0.5,
                    2.0 * s,
                    height,
                ),
                color: color::TEXT,
            });
        }
    }

    /// The bio's box, its counts, buttons and the rules, measured in `body` drawn in
    /// `style`.
    fn bio_column(&mut self, frame: &Frame, body: &UiFont, style: TextStyle) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            TOP + 20.0,
            MIDDLE_WIDTH,
            "About you",
        );
        let focused = self.focus == Focus::Bio;
        let rect = frame.rect(MIDDLE_X, BIO_TOP, MIDDLE_WIDTH, BIO_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 16.0 * s,
            color: color::alpha(color::SPACE, 0.6),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 16.0 * s,
            width: 1.5 * s,
            color: if focused {
                Color::new(1.0, 1.0, 1.0, 0.7)
            } else {
                color::alpha(color::HOLO, 0.4)
            },
        });
        self.ui.hit_region(BIO_TOKEN, rect);
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let room = MIDDLE_WIDTH - BIO_PAD * 2.0;
        if !self.bio.is_empty() || focused {
            self.bio_text(frame, body, style, caret);
        } else {
            let prompt = if self.writable {
                "Say something about yourself: your clan, your style, where you play."
            } else {
                "Your bio is kept on the SJK hub: switch the identity on to write one."
            };
            for (index, part) in wrap(prompt, 60).enumerate() {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(
                        MIDDLE_X + BIO_PAD,
                        BIO_TOP + BIO_PAD + index as f32 * BIO_LINE,
                        room,
                        BIO_LINE,
                    ),
                    BIO_SIZE * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
        }
        let below = BIO_TOP + BIO_HEIGHT + 18.0;
        let characters = self.bio.chars().count();
        let lines = if self.bio.is_empty() {
            0
        } else {
            self.bio.split('\n').count()
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{characters} / {} characters, {lines} / {} lines",
                bio::BIO_MAX,
                bio::LINES_MAX
            ),
            frame.rect(MIDDLE_X, below, 330.0, 26.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let mut x = MIDDLE_X + MIDDLE_WIDTH;
        for (label, primary, token, focus) in [
            ("Save", true, SAVE_TOKEN, Focus::Save),
            ("Revert", false, REVERT_TOKEN, Focus::Revert),
        ] {
            let width = 130.0;
            x -= width;
            kit::button(
                &mut self.ui,
                frame,
                [x, below - 10.0, width, 46.0],
                label,
                primary,
                self.writable && (primary || self.edited),
                self.focus == focus,
                token,
            );
            x -= 14.0;
        }
        let mut y = below + 56.0;
        if !self.message.is_empty() {
            let message = cut(&self.message, 80);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{message}"),
                frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 26.0),
                17.0 * s,
                if self.message == "Saved" {
                    color::GOLD_BRIGHT
                } else {
                    color::EMBER
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 34.0;
        }
        for part in wrap(
            "Letters, digits, spaces and simple punctuation, up to 6 lines. No emoji or symbols. A caret and a digit colour the text. Everyone can read it.",
            92,
        ) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 24.0;
        }
    }

    /// The picture panel, in the bio's place: the picture large (the one about to be
    /// sent, else the player's), what is happening, Browse... and how else to choose
    /// one, what a picture may be, and Use this picture, Remove picture and Done.
    fn picture_column(&mut self, frame: &Frame, who: &Who) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            TOP + 20.0,
            MIDDLE_WIDTH,
            "Your picture",
        );
        let preview = self.ready.is_some();
        let radius = LARGE_PICTURE * 0.5;
        crate::profile_card::avatar(
            &mut self.ui,
            frame.point(MIDDLE_X + radius, BIO_TOP + radius),
            radius * s,
            &crate::profile_card::Avatar {
                key_id: &who.key_id,
                version: &who.avatar,
                name: &who.headline,
                verified: who.verified,
                tick: who.tick,
                trust: who.trust,
                preview,
                lit: preview,
            },
        );
        let side_x = MIDDLE_X + LARGE_PICTURE + 30.0;
        let side_width = MIDDLE_WIDTH - LARGE_PICTURE - 30.0;
        let headline = match (&self.reading, &self.ready, self.changing) {
            (_, _, Some(changing)) if changing.removing => "Taking it down...",
            (_, _, Some(_)) => "Sending...",
            (Some(_), _, _) => "Reading the picture...",
            (_, Some(_), _) => "Your new picture",
            _ if self.has_picture => "Your picture",
            _ => "No picture yet",
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(side_x, BIO_TOP + 8.0, side_width, 36.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let file = self
            .reading
            .as_ref()
            .map(|reading| reading.name.as_str())
            .or_else(|| self.ready.as_ref().map(|(name, _)| name.as_str()));
        let mut y = BIO_TOP + 50.0;
        let detail = match file {
            Some(name) => cut(name, 40),
            None => "Everyone sees it beside your name".to_owned(),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{detail}"),
            frame.rect(side_x, y, side_width, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 36.0;
        let message = match &self.picture_message {
            Some(message) => Some(message.clone()),
            None if !self.writable => Some((
                "Pictures need the SJK identity on and the SJK hub answering".to_owned(),
                false,
            )),
            None => None,
        };
        if let Some((message, good)) = message {
            for part in wrap(&message, 44).take(4) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(side_x, y, side_width, 24.0),
                    16.0 * s,
                    if good {
                        color::GOLD_BRIGHT
                    } else {
                        color::EMBER
                    },
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 24.0;
            }
        }
        let browse_y = BIO_TOP + LARGE_PICTURE + 24.0;
        let browsing = self.is_browsing();
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, browse_y, 200.0, 46.0],
            if browsing {
                "Stop waiting"
            } else {
                "Browse..."
            },
            false,
            true,
            self.focus == Focus::Browse,
            BROWSE_TOKEN,
        );
        for (index, part) in wrap(
            "Choose a picture file on this PC, or drop one on this window.",
            46,
        )
        .take(2)
        .enumerate()
        {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(
                    MIDDLE_X + 220.0,
                    browse_y + index as f32 * 22.0,
                    MIDDLE_WIDTH - 220.0,
                    22.0,
                ),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = browse_y + 46.0 + 20.0;
        for line in [
            "A PNG, JPEG or TGA of up to 16 MB, 64 to 8192 pixels a side, at most four times as long as it is wide.",
            "It is cropped to a square from its middle and made 128 pixels across. Everyone can see it, so keep it friendly: the SJK team takes down pictures that are not.",
        ] {
            for part in wrap(line, 80) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 24.0),
                    16.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 24.0;
            }
            y += 10.0;
        }
        let buttons_y = y + 14.0;
        let busy = self.changing.is_some() || self.reading.is_some();
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, buttons_y, 220.0, 46.0],
            "Use this picture",
            true,
            self.writable && self.ready.is_some() && !busy,
            self.focus == Focus::UsePicture,
            USE_TOKEN,
        );
        let armed = self
            .remove_armed
            .is_some_and(|at| at.elapsed() < REMOVE_CONFIRM);
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + 236.0, buttons_y, 220.0, 46.0],
            if armed {
                "Press again"
            } else {
                "Remove picture"
            },
            false,
            self.writable && self.has_picture && !busy,
            self.focus == Focus::RemovePicture,
            REMOVE_TOKEN,
        );
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + MIDDLE_WIDTH - 130.0, buttons_y, 130.0, 46.0],
            "Done",
            false,
            true,
            self.focus == Focus::BioBack,
            BIO_BACK_TOKEN,
        );
    }

    /// The emoji panel: the name with its emoji as others see it, the grid of
    /// [`EMOJI_CHOICES`], what the hub answered, Remove emoji and Done.
    fn emoji_column(&mut self, frame: &Frame, who: &Who) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            TOP + 20.0,
            MIDDLE_WIDTH,
            "Your name emoji",
        );
        let shown = if self.emoji.is_empty() {
            cut(&who.headline, 24)
        } else {
            format!("{}^7 {}", cut(&who.headline, 24), self.emoji)
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{shown}"),
            frame.rect(MIDDLE_X, BIO_TOP - 4.0, MIDDLE_WIDTH, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Everyone sees it at the right of your name: in chat, on cards and on the scoreboard."
            ),
            frame.rect(MIDDLE_X, BIO_TOP + 40.0, MIDDLE_WIDTH, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let grid_y = BIO_TOP + 78.0;
        let grid_focused = self.focus == Focus::EmojiGrid;
        for (index, choice) in EMOJI_CHOICES.iter().enumerate() {
            let column = (index % EMOJI_COLUMNS) as f32;
            let row = (index / EMOJI_COLUMNS) as f32;
            let cell = [
                MIDDLE_X + column * (EMOJI_CELL + EMOJI_GAP),
                grid_y + row * (EMOJI_CELL + EMOJI_GAP),
                EMOJI_CELL,
                EMOJI_CELL,
            ];
            let token = EMOJI_CELL_TOKEN + index as u16;
            let worn = *choice == self.emoji;
            let lit = self.ui.token_hovered(token) || (grid_focused && index == self.emoji_cursor);
            let rect = frame.rect(cell[0], cell[1], cell[2], cell[3]);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 10.0 * s,
                color: if worn {
                    Color::new(0.91, 0.72, 0.29, 0.30)
                } else if lit {
                    Color::new(1.0, 1.0, 1.0, 0.14)
                } else {
                    Color::new(1.0, 1.0, 1.0, 0.05)
                },
            });
            if worn || lit {
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 10.0 * s,
                    width: 1.5 * s,
                    color: if worn {
                        color::GOLD_BRIGHT
                    } else {
                        color::TEXT
                    },
                });
            }
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{choice}"),
                frame.rect(cell[0], cell[1] + 11.0, cell[2], 36.0),
                30.0 * s,
                Color::new(1.0, 1.0, 1.0, 1.0),
                FontWeight::Regular,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
        }
        let rows = EMOJI_CHOICES.len().div_ceil(EMOJI_COLUMNS) as f32;
        let mut y = grid_y + rows * (EMOJI_CELL + EMOJI_GAP) + 10.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Another emoji? Type or paste it here (Win+. opens the emoji panel)."),
            frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 24.0),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 30.0;
        let message = match &self.emoji_message {
            Some(message) => Some(message.clone()),
            None if !self.writable => Some((
                "The name emoji needs the SJK identity on and the SJK hub answering".to_owned(),
                false,
            )),
            None => None,
        };
        if let Some((message, good)) = message {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", cut(&message, 80)),
                frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 24.0),
                16.0 * s,
                if good {
                    color::GOLD_BRIGHT
                } else {
                    color::EMBER
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let buttons_y = y + 36.0;
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, buttons_y, 220.0, 46.0],
            "Remove emoji",
            false,
            self.writable && !self.emoji.is_empty() && self.emoji_saving.is_none(),
            self.focus == Focus::RemoveEmoji,
            REMOVE_EMOJI_TOKEN,
        );
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + MIDDLE_WIDTH - 130.0, buttons_y, 130.0, 46.0],
            "Done",
            false,
            true,
            self.focus == Focus::BioBack,
            BIO_BACK_TOKEN,
        );
    }

    /// The medals (on its own only: the Collection has them), the achievements and the
    /// collection.
    fn right_column(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let y = if self.mode == Mode::Hub {
            TOP + 20.0
        } else {
            self.medals_block(frame, inputs).max(TOP + 300.0) + 20.0
        };
        self.achievements_block(frame, inputs, y);
        self.unlockables(frame, inputs, y + 176.0);
        self.holocrons(frame, inputs, y + 322.0);
    }

    /// The medals given, up to [`MEDALS_SHOWN`], from the column's top; returns the y
    /// under them.
    fn medals_block(&mut self, frame: &Frame, inputs: &Inputs<'_>) -> f32 {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            TOP + 20.0,
            RIGHT_WIDTH,
            &crate::terms::say("Medals"),
        );
        let mut y = TOP + 48.0;
        let medals = inputs
            .snapshot
            .and_then(|snapshot| snapshot.me.as_ref())
            .map(|me| crate::medals::awards(&me.medals))
            .unwrap_or_default();
        if medals.is_empty() {
            for part in wrap(&crate::terms::say(NO_MEDALS), 48) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(RIGHT_X, y, RIGHT_WIDTH, 24.0),
                    16.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 24.0;
            }
            y += 16.0;
        }
        let seconds = self.epoch.elapsed().as_secs_f32();
        for award in medals.iter().take(MEDALS_SHOWN) {
            crate::medals::rank::draw_art(
                self.ui.draw_list_mut(),
                crate::medals::rank::Art::new(
                    award.medal,
                    award.rank,
                    frame.rect(RIGHT_X, y, 84.0, 84.0),
                    seconds,
                ),
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", award.label()),
                frame.rect(RIGHT_X + 98.0, y + 8.0, RIGHT_WIDTH - 98.0, 30.0),
                23.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", award.rank_and_given()),
                frame.rect(RIGHT_X + 98.0, y + 40.0, RIGHT_WIDTH - 98.0, 24.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 96.0;
        }
        if medals.len() > MEDALS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("and {} more", medals.len() - MEDALS_SHOWN),
                frame.rect(RIGHT_X, y, RIGHT_WIDTH, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 30.0;
        }
        y
    }

    /// How many achievements are unlocked, and the way to the board, from `y`.
    fn achievements_block(&mut self, frame: &Frame, inputs: &Inputs<'_>, y: f32) {
        let s = frame.s;
        kit::heading(&mut self.ui, frame, RIGHT_X, y, RIGHT_WIDTH, "Achievements");
        let (unlocked, total) = crate::achievements::tally(inputs.standings);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{unlocked} of {total} unlocked"),
            frame.rect(RIGHT_X, y + 26.0, RIGHT_WIDTH, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        bar(
            &mut self.ui,
            frame,
            [RIGHT_X, y + 76.0, RIGHT_WIDTH, 8.0],
            if total == 0 {
                0.0
            } else {
                unlocked as f32 / total as f32
            },
            color::GOLD,
        );
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X, y + 104.0, 240.0, 46.0],
            "See the board",
            false,
            true,
            self.focus == Focus::Board,
            BOARD_TOKEN,
        );
    }

    /// How many unlockables the player owns, and the way to their page, from `y`.
    fn unlockables(&mut self, frame: &Frame, inputs: &Inputs<'_>, y: f32) {
        let s = frame.s;
        let (heading, button) = (crate::profile_hub::COLLECTION, "See the collection");
        kit::heading(&mut self.ui, frame, RIGHT_X, y, RIGHT_WIDTH, heading);
        let holdings = crate::unlockables::Holdings::of(inputs.enabled, inputs.snapshot);
        let (line, size, colour) = if holdings.reason().is_none() {
            let owned = crate::unlockables::ALL
                .iter()
                .filter(|unlockable| holdings.unlock(unlockable.id).is_some())
                .count();
            // Out of the shaders the player sees: an unheld Unique one is not counted.
            (
                format!("{owned} of {} shaders owned", holdings.listed_count()),
                30.0,
                color::TEXT,
            )
        } else {
            (
                crate::terms::say("Medals, shaders, toys and more, kept on the SJK hub")
                    .into_owned(),
                17.0,
                color::MUTED,
            )
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{line}"),
            frame.rect(RIGHT_X, y + 26.0, RIGHT_WIDTH, 40.0),
            size * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X, y + 76.0, 240.0, 46.0],
            button,
            false,
            true,
            self.focus == Focus::Unlockables,
            UNLOCKABLES_TOKEN,
        );
    }

    /// How many holocrons the player holds, and the way to their page, from `y`.
    fn holocrons(&mut self, frame: &Frame, inputs: &Inputs<'_>, y: f32) {
        let s = frame.s;
        let heading = if self.mode == Mode::Hub {
            crate::profile_hub::HOLOCRONS
        } else {
            "Holocrons"
        };
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            y,
            RIGHT_WIDTH,
            &crate::terms::say(heading),
        );
        let known = inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.me.as_ref())
            .map(|me| {
                crate::holocrons::counts_of(&me.holocron_counts)
                    .iter()
                    .sum::<u32>()
            });
        let (line, size, colour) = match known {
            Some(1) => (
                crate::terms::say("1 holocron").into_owned(),
                30.0,
                color::TEXT,
            ),
            Some(total) => (
                crate::terms::say_owned(format!("{total} holocrons")),
                30.0,
                color::TEXT,
            ),
            None => (
                "Found by playing, kept on the SJK hub".to_owned(),
                17.0,
                color::MUTED,
            ),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{line}"),
            frame.rect(RIGHT_X, y + 26.0, RIGHT_WIDTH, 40.0),
            size * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X, y + 76.0, 240.0, 46.0],
            &crate::terms::say("See holocrons"),
            false,
            true,
            self.focus == Focus::Holocrons,
            HOLOCRONS_TOKEN,
        );
    }

    /// The keys of what has the keyboard, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let typing = self.focus == Focus::Bio && self.writable;
        let enter = match self.focus {
            Focus::Staff => "open",
            Focus::Bio | Focus::Save => "save",
            Focus::Revert => "revert",
            Focus::Board | Focus::Unlockables | Focus::Holocrons => "open",
            Focus::Picture => "change picture",
            Focus::Browse => "choose a file",
            Focus::UsePicture => "use this picture",
            Focus::RemovePicture => "remove picture",
            Focus::BioBack => "back to your bio",
            Focus::Name => "change your name",
            Focus::Emoji => "choose an emoji",
            Focus::EmojiGrid => "use this emoji",
            Focus::RemoveEmoji => "remove emoji",
        };
        let mut keys: Vec<(&[&str], &str)> = vec![(&["Tab"], "next"), (&["Enter"], enter)];
        if typing {
            keys.push((&["Shift", "Enter"], "new line"));
        }
        if self.mode == Mode::Hub {
            keys.push((&["Ctrl", "Tab"], HubTab::Profile.next(true).label()));
        }
        keys.push((&["Esc"], "back"));
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

/// A progress bar over `rect` (frame pixels), filled to `fraction` in `fill`.
fn bar(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    fraction: f32,
    fill: Color,
) {
    let [x, y, width, height] = rect;
    let s = frame.s;
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: frame.rect(x, y, width, height),
        radius: height * 0.5 * s,
        color: color::alpha(color::HOLO, 0.12),
    });
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, (width * fraction).max(height), height),
            radius: height * 0.5 * s,
            color: fill,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::snapshot;
    use super::*;
    use crate::achievements;
    use sjk_identity::{Medal, Profile, WornName};

    struct Fonts {
        display: crate::text::FontAtlas,
        body: crate::text::FontAtlas,
    }

    fn fonts() -> Fonts {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a bundled family");
        Fonts {
            display: load(&crate::text::DISPLAY),
            body: load(&crate::text::BODY),
        }
    }

    fn full_profile() -> Profile {
        Profile {
            tag_cards: Vec::new(),
            credits: 0,
            cards: Vec::new(),
            keys: Vec::new(),
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: "^1Ш^7Sol^3Vulpes^7 the Long Named".to_owned(),
            banner: "".into(),
            bio: format!(
                "{}\n\nline three\n{}",
                "Äöüß word ".repeat(30),
                "x".repeat(60)
            ),
            verified: true,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            jkr_creator: false,
            trust: None,
            emoji: String::new(),
            created: 1_759_708_800,
            names: ["^2Fox", "Sol", "^4Another Long Name", "Fourth"]
                .iter()
                .map(|name| WornName {
                    name: (*name).to_owned(),
                    first_seen: 0,
                    last_seen: 0,
                })
                .collect(),
            medals: ["early_tester", "early_contributor", "bug_hunter", "unknown"]
                .iter()
                .map(|id| Medal {
                    id: (*id).to_owned(),
                    count: 3,
                    awarded: 1_759_900_000,
                    note: "a note".to_owned(),
                    rank: 1,
                })
                .collect(),
            achievements: Vec::new(),
            avatar: String::new(),
            unlocks: Vec::new(),
            holocron_counts: Default::default(),
            holocrons: Vec::new(),
            shards: 0,
        }
    }

    fn record() -> Vec<(&'static str, String)> {
        [
            "Players defeated",
            "Saber kills",
            "Best streak",
            "Duels won",
            "Flags captured",
            "Maps played",
            "Servers",
            "Time played",
        ]
        .iter()
        .map(|label| (*label, "123456".to_owned()))
        .collect()
    }

    /// Half unlocked, half part-way.
    fn standings() -> Vec<Standing> {
        achievements::ALL
            .iter()
            .enumerate()
            .map(|(index, kind)| Standing {
                kind,
                progress: if index % 2 == 0 {
                    kind.goal
                } else {
                    kind.goal / 3
                },
                unlocked: (index % 2 == 0).then_some(1_759_900_000),
                waiting: false,
            })
            .collect()
    }

    /// Every tab, focus and state, at its longest, fits the canvas at 1080 lines, 4K,
    /// 4:3 and 21:9, in the families and in Inter.
    #[test]
    fn every_state_fits_the_canvas() {
        let families = fonts();
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let record = record();
        let standings = standings();
        let online = snapshot(Some(full_profile()), Some("saved"));
        let offline = Snapshot {
            status: Status::Failed("the hub took too long to answer, again and again".into()),
            ..snapshot(None, None)
        };
        for (enabled, shot) in [(true, Some(&online)), (true, Some(&offline)), (false, None)] {
            {
                for focus in [
                    Focus::Picture,
                    Focus::Staff,
                    Focus::Browse,
                    Focus::Bio,
                    Focus::Save,
                    Focus::Revert,
                    Focus::Board,
                    Focus::Unlockables,
                    Focus::Holocrons,
                ] {
                    for body in [&families.body.font, &inter.font] {
                        for viewport in [
                            [1920.0, 1080.0],
                            [3840.0, 2160.0],
                            [1440.0, 1080.0],
                            [2560.0, 1080.0],
                        ] {
                            for mode in [Mode::Pages, Mode::Hub] {
                                let mut panel = Panel::new();
                                panel.open_as(true, mode);
                                panel.focus = focus;
                                panel.message = "a bio uses letters, digits, spaces and simple punctuation (no emoji or symbols)".into();
                                let inputs = Inputs {
                                    enabled,
                                    snapshot: shot,
                                    standings: &standings,
                                    record: &record,
                                };
                                panel.build(&inputs, body, body.style(), viewport);
                                assert!(
                                    !panel.ui.overflowed(),
                                    "{focus:?} {mode:?} {enabled} at {viewport:?}"
                                );
                                // The Profile screen's tabs answer where they are drawn.
                                let strip = panel.ui.rect_for(crate::profile_hub::TOKEN);
                                assert_eq!(strip.is_some(), mode == Mode::Hub);
                                if mode == Mode::Hub {
                                    clear_of_the_row(&panel, viewport);
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = &families.display;
    }

    /// On the Profile screen a staff key gets Change name, the Name emoji button and
    /// Staff tools in one row: each label fits its button, the buttons keep apart and
    /// within the left column, and each answers the pointer (hub report #202).
    #[test]
    fn change_name_emoji_and_staff_tools_fit_their_buttons() {
        let families = fonts();
        let record = record();
        let standings = standings();
        for emoji in ["", "\u{1F98A}"] {
            let mut me = full_profile();
            me.staff = true;
            me.emoji = emoji.to_owned();
            let online = snapshot(Some(me), None);
            for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
                let frame = Frame::new(viewport);
                let mut panel = Panel::new();
                panel.open_as(true, Mode::Hub);
                let inputs = Inputs {
                    enabled: true,
                    snapshot: Some(&online),
                    standings: &standings,
                    record: &record,
                };
                panel.build(&inputs, &families.body.font, TextStyle::NEUTRAL, viewport);
                assert!(!panel.ui.overflowed());
                assert!(panel.staff);
                assert_eq!(panel.emoji, emoji);
                // An emoji is drawn about as wide as the text is tall.
                let emoji_label = if emoji.is_empty() {
                    ("Add a name emoji", 0.0)
                } else {
                    ("Name emoji ", 22.0)
                };
                let left = frame.point(LEFT_X, 0.0)[0];
                let right = frame.point(LEFT_X + LEFT_WIDTH, 0.0)[0];
                let mut end = left - 1.0;
                for (token, (label, extra)) in [
                    (NAME_TOKEN, ("Change name", 0.0)),
                    (EMOJI_TOKEN, emoji_label),
                    (STAFF_TOKEN, ("Staff tools", 0.0)),
                ] {
                    let rect = panel.ui.rect_for(token).expect("the button's area");
                    // Room for the label (measured in the heavier weight) and the
                    // pill's rounded ends.
                    let needed = (crate::text::display_width(label, 20.0) + extra + 24.0) * frame.s;
                    assert!(
                        needed <= rect.width,
                        "{label:?} needs {needed}, has {} at {viewport:?}",
                        rect.width
                    );
                    assert!(
                        rect.x > end && rect.right() <= right + 0.5,
                        "{label:?} at {viewport:?}"
                    );
                    end = rect.right();
                }
            }
        }
        // On its own the page has no Change name.
        let online = snapshot(Some(full_profile()), None);
        let mut alone = Panel::new();
        alone.open_as(true, Mode::Pages);
        let inputs = Inputs {
            enabled: true,
            snapshot: Some(&online),
            standings: &standings,
            record: &record,
        };
        alone.build(
            &inputs,
            &families.body.font,
            TextStyle::NEUTRAL,
            [1920.0, 1080.0],
        );
        assert!(alone.ui.rect_for(NAME_TOKEN).is_none());
    }

    /// The picture panel fits the canvas in each of its states, keeps to the middle
    /// column, and its controls (and the picture that opens it) answer the pointer
    /// above the keys, at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_picture_panel_fits_its_column_and_answers_the_pointer() {
        let families = fonts();
        let record = record();
        let standings = standings();
        let online = snapshot(Some(full_profile()), Some("saved"));
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
        ] {
            let frame = Frame::new(viewport);
            for (state, mode) in (0..5).flat_map(|state| [(state, Mode::Pages), (state, Mode::Hub)])
            {
                let mut panel = Panel::new();
                panel.open_as(true, mode);
                panel.middle = Middle::Picture;
                match state {
                    0 => {}
                    1 => {
                        panel.ready = Some((
                            "a picture file whose name goes on and on and on.png".into(),
                            vec![1],
                        ));
                    }
                    2 => {
                        panel.changing = Some(Changing {
                            serial_before: 0,
                            removing: true,
                        });
                    }
                    3 => {
                        panel.picture_message = Some((
                            "That picture is too big: at most 16 MB and 8192 pixels a side. "
                                .repeat(3),
                            false,
                        ));
                    }
                    _ => panel.remove_armed = Some(Instant::now()),
                }
                for focus in [
                    Focus::Picture,
                    Focus::Browse,
                    Focus::UsePicture,
                    Focus::RemovePicture,
                    Focus::BioBack,
                ] {
                    panel.focus = focus;
                    let inputs = Inputs {
                        enabled: state != 4,
                        snapshot: Some(&online),
                        standings: &standings,
                        record: &record,
                    };
                    panel.build(&inputs, &families.body.font, TextStyle::NEUTRAL, viewport);
                    assert!(!panel.ui.overflowed(), "{state} {viewport:?}");
                    let keys = frame.point(0.0, KEYS_Y)[1];
                    for token in [
                        PICTURE_TOKEN,
                        BROWSE_TOKEN,
                        USE_TOKEN,
                        REMOVE_TOKEN,
                        BIO_BACK_TOKEN,
                    ] {
                        let rect = panel.ui.rect_for(token).expect("the control's area");
                        assert!(rect.x >= 0.0 && rect.right() <= viewport[0], "{token}");
                        assert!(rect.y >= 0.0 && rect.bottom() < keys, "{token} {state}");
                    }
                    let (middle, right) =
                        (frame.point(MIDDLE_X, 0.0)[0], frame.point(RIGHT_X, 0.0)[0]);
                    for command in panel.ui.draw_list().commands() {
                        // The keys' line runs under every column.
                        if let DrawCommand::Text { rect, .. } = command
                            && rect.x >= middle - 0.5
                            && rect.x < right
                            && rect.y < keys
                        {
                            assert!(rect.right() <= right - 20.0 * frame.s, "{state} {rect:?}");
                        }
                    }
                }
            }
        }
    }

    /// On the Profile screen nothing of the page stands in the row of tabs: between the
    /// title and the page's top only the tabs' names are drawn.
    fn clear_of_the_row(panel: &Panel, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        let (above, below) = (frame.point(0.0, 120.0)[1], frame.point(0.0, 192.0)[1]);
        let tabs = frame.point(0.0, 146.0)[1];
        for command in panel.ui.draw_list().commands() {
            if let DrawCommand::Text { rect, .. } = command
                && rect.y > above
                && rect.y < below
            {
                // A tab's name is set a little under the row's top.
                assert!(
                    rect.y >= tabs && rect.y - tabs < 10.0 * frame.s,
                    "{rect:?} in the row"
                );
            }
        }
    }

    /// The bio's lines fit the box's width, in any text style; a click on a line puts the
    /// caret where it lands and Up takes it a line up (Kushina's report, 22/10/2026: the
    /// lines could not be clicked).
    #[test]
    fn the_bio_wraps_inside_its_box_and_a_click_places_the_caret() {
        let fonts = fonts();
        let body = &fonts.body.font;
        let record = record();
        let standings = standings();
        let online = snapshot(Some(full_profile()), None);
        let inputs = Inputs {
            enabled: true,
            snapshot: Some(&online),
            standings: &standings,
            record: &record,
        };
        for (viewport, scale) in [([1920.0, 1080.0], 1.0), ([3840.0, 2160.0], 1.3)] {
            let style = TextStyle {
                scale,
                tracking: 0.0,
            };
            let frame = Frame::new(viewport);
            let mut panel = Panel::new();
            panel.open_as(true, Mode::Pages);
            panel.build(&inputs, body, style, viewport);
            let bio = panel.bio.clone();
            panel.bio_layout.show(0, 100, [0.0, 0.0], 1.0);
            let (_, lines) = panel.bio_layout.visible();
            assert!(lines.len() > 4);
            assert_eq!(lines.iter().filter(|line| line.is_empty()).count(), 1);
            let size = BIO_SIZE * frame.s * scale;
            let room = (MIDDLE_WIDTH - BIO_PAD * 2.0) * frame.s;
            for line in lines {
                let width =
                    crate::text::visible_text_width(body, &bio[line.clone()], size / body.height);
                assert!(width <= room, "{:?}", &bio[line.clone()]);
            }
            // A click on the second row shown, some way in.
            panel.build(&inputs, body, style, viewport);
            let (first, shown) = panel.bio_layout.visible();
            let second = shown[1].clone();
            let inside = second.start + bio[second.clone()].char_indices().nth(4).unwrap().0;
            let row = frame.rect(
                MIDDLE_X + BIO_PAD,
                BIO_TOP + BIO_PAD + BIO_LINE,
                0.0,
                BIO_LINE,
            );
            let x = crate::text::visible_text_width(
                body,
                &bio[second.start..inside],
                size / body.height,
            );
            let point = sjk_ui::Vec2::new(row.x + x + 0.5, row.y + row.height * 0.5);
            let button = sjk_ui::PointerButton::Primary;
            panel.handle_pointer(InputEvent::PointerMove(point), None);
            panel.handle_pointer(
                InputEvent::PointerPress {
                    position: point,
                    button,
                },
                None,
            );
            panel.handle_pointer(
                InputEvent::PointerRelease {
                    position: point,
                    button,
                },
                None,
            );
            assert_eq!(panel.focus, Focus::Bio);
            let at = panel.bio_edit.cursor(&panel.bio);
            assert_eq!(panel.bio_layout.locate(&bio, at).0, first + 1);
            assert_eq!(at, inside, "{viewport:?}");
            // Up: the line above, at about the same place.
            panel.build(&inputs, body, style, viewport);
            assert!(panel.bio_key(KeyCode::ArrowUp, None, false, false, false));
            let up = panel.bio_edit.cursor(&panel.bio);
            assert_eq!(panel.bio_layout.locate(&bio, up).0, first);
        }
    }
}
