//! The Staff page's drawing, in the SJK UI's look: the search pill in the top bar,
//! the players found down the left, and on the right the chosen player's picture and
//! name beside the note sent with the next gift, over a row of tabs:
//!
//! Key ids never show (nor anywhere in the game): players go by name, a key id typed or
//! pasted into the search or the merge field shows as dots, linked keys are numbered.
//!
//! - Overview: their account (flags, trust, when last seen, what they hold, the names
//!   they wore)
//!   with Verify, and moderation: their picture (Take down, Block pictures, Allow
//!   pictures) and the SJK chat (Mute at the hub, Unmute, Lift chat bans);
//! - Medals and unlocks: the medals with Give and Take back, the unlockables with
//!   Unlock and Relock;
//! - Holocrons and credits: a holocron or kyber crystal tier to choose, Give, and their
//!   holocrons and crystals with Remove;
//!   their credits and kyber shards with an amount, Give and Take;
//! - Achievements with Clear (and Clear all), in two columns;
//! - Keys: their keys with Unlink and merging another player into them;
//! - Logs: asking the player for their game's logs, the requests with Open;
//! - Crashes and Chat bans, of every player: the crash reports by kind, then one
//!   kind's with Open; the SJK chat bans in force, each with Lift.

use super::*;
use crate::achievements;
use crate::medals::Medal;
use crate::menu::sjk::{
    Frame, SearchPill, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar, wrap,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The columns (frame pixels).
const LEFT_X: f32 = 96.0;
const LEFT_WIDTH: f32 = 560.0;
const MIDDLE_X: f32 = 720.0;
const MIDDLE_WIDTH: f32 = 520.0;
const RIGHT_X: f32 = 1_300.0;
const RIGHT_WIDTH: f32 = 524.0;
const TOP: f32 = 170.0;
/// The tabs' row under the chosen player's name.
const TABS_Y: f32 = 250.0;
/// The width the one-column views (Keys, Logs, Crashes, Chat bans) take from
/// [`MIDDLE_X`].
const PANE_WIDTH: f32 = 760.0;
/// The players' rows.
const PLAYERS_TOP: f32 = 236.0;
const PLAYER_ROW: f32 = 46.0;
/// The medals' rows and the achievements' rows.
const SECTION_TOP: f32 = 342.0;
const MEDAL_ROW: f32 = 64.0;
/// An unlockable's cell, two to a row: compact, so the whole catalogue (fourteen blade
/// skins since 10/10/2026) fits above the keys.
const UNLOCK_ROW: f32 = 36.0;
const UNLOCK_GAP: f32 = 14.0;
const ACHIEVEMENT_ROW: f32 = 34.0;
/// Two columns of achievements.
const ACHIEVEMENTS_PER_COLUMN: usize = 16;
const ACHIEVEMENTS_SHOWN: usize = ACHIEVEMENTS_PER_COLUMN * 2;
/// The Logs and Crashes views' rows.
const LOG_ROW: f32 = 36.0;
const CRASH_GROUP_ROW: f32 = 52.0;
/// The holocrons' section under the players: its heading, the tier chips, and its rows.
const HOLOCRONS_TOP: f32 = SECTION_TOP + 16.0;
const TIER_CHIP: [f32; 2] = [104.0, 42.0];
const HOLOCRON_ROW: f32 = 34.0;
/// The Keys and merge view: the linked keys' rows, then the merge section.
const LINKED_ROW: f32 = 36.0;
const MERGE_TOP: f32 = 640.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;

/// `text` as a field shows it: a key id (8 hex digits or more, nothing else) as dots,
/// since key ids never show in the game; anything else as it is.
fn hide_key(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() >= 8 && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        format!("{} (key id)", "\u{2022}".repeat(trimmed.len().min(16)))
    } else {
        text.to_owned()
    }
}

/// When the hub last saw `profile` wear a name in a game, unix seconds; 0 when never.
fn last_seen(profile: &Profile) -> i64 {
    profile
        .names
        .iter()
        .map(|worn| worn.last_seen)
        .max()
        .unwrap_or(0)
}

/// How many kyber crystals of each tier `profile` holds unopened, as its list shows
/// them (in `crate::kyber::TIERS` order).
fn crystal_counts(profile: &Profile) -> [u32; crate::kyber::COUNT] {
    let mut counts = [0; crate::kyber::COUNT];
    for holocron in profile.holocrons.iter().filter(|h| h.opened.is_none()) {
        if let Some(tier) = crate::kyber::from_id(&holocron.tier) {
            counts[tier.index] += 1;
        }
    }
    counts
}

/// `text` cut to `chars` characters with an ellipsis.
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
        let body = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        self.build(inputs, body, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out; `_body` is the body family, kept for measured layouts.
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, _body: &UiFont, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.order.clear();
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        let caret = (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let query = if self.focus == SEARCH_TOKEN && caret {
            format!("{}|", hide_key(&self.query))
        } else {
            hide_key(&self.query)
        };
        top_bar(
            &mut self.ui,
            &frame,
            "Back",
            BACK_TOKEN,
            "Staff",
            Some(SearchPill {
                query: &query,
                active: self.focus == SEARCH_TOKEN,
                prompt: "Find a player by name or key id",
                found: (!inputs.staff.players.is_empty()).then_some(inputs.staff.players.len()),
                token: SEARCH_TOKEN,
            }),
        );
        self.order.push(SEARCH_TOKEN);
        self.shown.me = inputs.me.map(|me| me.key_id.clone());
        self.players(&frame, inputs);
        let target = self.target(inputs).cloned();
        self.shown.target = target.as_ref().map(|profile| profile.key_id.clone());
        self.shown.medals = [0; Medal::COUNT];
        self.shown.ranks = [crate::medals::Rank::Issued; Medal::COUNT];
        self.shown.picture = target
            .as_ref()
            .is_some_and(|profile| !profile.avatar.is_empty());
        self.shown.unlocks = vec![false; crate::unlockables::ALL.len()];
        self.shown.holocron_rows = 0;
        self.shown.verified = target.as_ref().is_some_and(|profile| profile.verified);
        self.shown.tick = target.as_ref().map_or(sjk_identity::Tick::None, |profile| {
            profile.tick.shown(profile.verified)
        });
        self.shown.staff = target.as_ref().is_some_and(|profile| profile.staff);
        self.shown.creator = inputs.me.is_some_and(|me| me.creator);
        self.shown.linked.clear();
        self.shown.log_rows.clear();
        self.shown.crash_groups.clear();
        self.shown.crash_rows.clear();
        self.shown.ban_rows.clear();
        if let Some(profile) = &target {
            self.shown
                .linked
                .extend(profile.linked_keys().take(LINKED_SHOWN).map(str::to_owned));
        }
        self.shown.merge_problem = Self::merge_problem(
            &merge_key(&self.merge_from),
            target.as_ref(),
            &inputs.staff.players,
        );
        // A merge confirmed for another player than the one now chosen is let go.
        if self
            .pending_merge
            .as_ref()
            .is_some_and(|merge| self.shown.target.as_ref() != Some(&merge.kept))
        {
            self.pending_merge = None;
        }
        if let Some(profile) = &target {
            for medal in &profile.medals {
                if let Some(known) = Medal::from_id(&medal.id) {
                    self.shown.medals[known.index()] = medal.count.max(1);
                    self.shown.ranks[known.index()] = crate::medals::Rank::from_wire(medal.rank);
                }
            }
            for (held, unlockable) in self.shown.unlocks.iter_mut().zip(&crate::unlockables::ALL) {
                *held = profile
                    .unlocks
                    .iter()
                    .any(|unlock| unlock.id == unlockable.id);
            }
            let mine = self.shown.me.as_deref() == Some(profile.key_id.as_str());
            self.header(&frame, profile, mine);
            self.tabs(&frame);
            match self.view {
                View::Overview => self.overview(&frame, profile, mine),
                View::Rewards => {
                    self.medals(&frame, profile);
                    self.unlockables(&frame, profile);
                }
                View::Economy => {
                    self.holocrons(&frame, profile);
                    self.wallet(&frame, profile);
                }
                View::Achievements => self.achievements(&frame, profile, mine),
                View::Keys => self.keys_and_merge(&frame, profile, &inputs.staff.players),
                View::Logs => self.logs(&frame, profile, inputs.staff),
                View::Crashes => self.crashes(&frame, inputs.staff),
                View::ChatBans => self.chat_bans(&frame, inputs.staff),
            }
        }
        if !self.shown.creator {
            self.order.retain(|token| !super::owner_only(*token));
        }
        self.status(&frame, inputs.staff);
        self.keys(&frame);
        if !self.order.contains(&self.focus) {
            self.focus = SEARCH_TOKEN;
        }
        self.ui.finish(self.focus);
    }

    /// The players found, with Me and Recent over them.
    fn players(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        for (index, (label, token)) in [("Me", ME_TOKEN), ("Seen lately", RECENT_TOKEN)]
            .into_iter()
            .enumerate()
        {
            let width = if index == 0 { 90.0 } else { 160.0 };
            let x = LEFT_X + index as f32 * 104.0;
            kit::button(
                &mut self.ui,
                frame,
                [x, TOP - 2.0, width, 42.0],
                label,
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        self.shown.players.clear();
        let selected = self.shown_selection(inputs);
        for (index, player) in inputs.staff.players.iter().take(PLAYERS_SHOWN).enumerate() {
            let token = PLAYER_BASE + index as u16;
            let y = PLAYERS_TOP + index as f32 * PLAYER_ROW;
            let row = [LEFT_X, y, LEFT_WIDTH, PLAYER_ROW - 4.0];
            if self.focus == token || selected.as_deref() == Some(player.key_id.as_str()) {
                kit::band(&mut self.ui, frame, row);
            }
            let name = if player.name.is_empty() {
                "(no name yet)".to_owned()
            } else {
                cut(&player.name, 34)
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(LEFT_X + 18.0, y + 2.0, LEFT_WIDTH - 150.0, 26.0),
                21.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            // When the hub last saw them in a game, never their key id.
            let seen = crate::medals::date_text(last_seen(player));
            let you = self.shown.me.as_deref() == Some(player.key_id.as_str());
            let about = match (you, seen.is_empty()) {
                (true, _) => "you".to_owned(),
                (false, true) => "not seen in a game yet".to_owned(),
                (false, false) => format!("seen {seen}"),
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{about}"),
                frame.rect(LEFT_X + 18.0, y + 26.0, 260.0, 18.0),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let tags = match (player.staff, player.verified) {
                _ if player.creator => "Creator",
                (true, true) => "Staff, verified",
                (true, false) => "Staff",
                (false, true) => "Verified",
                (false, false) => "",
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{tags}"),
                frame.rect(LEFT_X + LEFT_WIDTH - 150.0, y + 10.0, 136.0, 24.0),
                16.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::End,
            );
            self.ui
                .hit_region(token, frame.rect(row[0], row[1], row[2], row[3]));
            self.order.push(token);
            self.shown.players.push(player.key_id.clone());
        }
        if inputs.staff.players.len() > PLAYERS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "and {} more: refine the search",
                    inputs.staff.players.len() - PLAYERS_SHOWN
                ),
                frame.rect(
                    LEFT_X + 18.0,
                    PLAYERS_TOP + PLAYERS_SHOWN as f32 * PLAYER_ROW,
                    LEFT_WIDTH,
                    22.0,
                ),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The key of the chosen player, as the list marks it.
    fn shown_selection(&self, inputs: &Inputs<'_>) -> Option<String> {
        self.selected
            .clone()
            .or_else(|| inputs.me.map(|me| me.key_id.clone()))
    }

    /// The chosen player's picture, name and facts over the middle column, and the
    /// note sent with the next gift at the right.
    fn header(&mut self, frame: &Frame, profile: &Profile, mine: bool) {
        let s = frame.s;
        let name = if profile.name.is_empty() {
            "(no name yet)".to_owned()
        } else {
            cut(&profile.name, 24)
        };
        let radius = 32.0;
        crate::profile_card::avatar(
            &mut self.ui,
            frame.point(MIDDLE_X + radius, TOP + 26.0),
            radius * s,
            &crate::profile_card::Avatar {
                key_id: &profile.key_id,
                version: &profile.avatar,
                name: &profile.name,
                verified: profile.verified,
                tick: profile.tick,
                trust: profile.trust,
                preview: false,
                lit: false,
            },
        );
        let name_x = MIDDLE_X + radius * 2.0 + 18.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(name_x, TOP - 6.0, RIGHT_X - 24.0 - name_x, 48.0),
            38.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        self.note_field(frame);
        let mut facts = Vec::new();
        if profile.creator {
            facts.push("creator".to_owned());
        }
        if profile.verified {
            facts.push("verified".to_owned());
        }
        if profile.staff && !profile.creator {
            facts.push("staff".to_owned());
        }
        let keys = profile.keys.len().max(1);
        facts.push(if keys == 1 {
            "1 key".to_owned()
        } else {
            format!("{keys} keys")
        });
        let seen = crate::medals::date_text(last_seen(profile));
        if !seen.is_empty() {
            facts.push(format!("seen {seen}"));
        }
        if mine {
            facts.push("this is you".to_owned());
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", facts.join(", ")),
            frame.rect(name_x, TOP + 46.0, RIGHT_X - 24.0 - name_x, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The note sent with the next medal, unlock, holocron, credits or shards, and with
    /// a log request: at the right of the chosen player's name, whatever the tab.
    fn note_field(&mut self, frame: &Frame) {
        let s = frame.s;
        let y = TOP - 14.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Note with the next gift (optional, everyone reads it)"),
            frame.rect(RIGHT_X, y, RIGHT_WIDTH, 22.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let field = [RIGHT_X, y + 26.0, RIGHT_WIDTH, 44.0];
        let focused = self.focus == NOTE_TOKEN;
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let shown = if self.note.chars().count() > 56 {
            let tail: String = self
                .note
                .chars()
                .skip(self.note.chars().count() - 53)
                .collect();
            format!("...{tail}")
        } else {
            self.note.clone()
        };
        kit::field(
            &mut self.ui,
            frame,
            field,
            format_args!("{shown}{}", if caret { "|" } else { "" }),
            focused,
            false,
        );
        self.ui.hit_region(
            NOTE_TOKEN,
            frame.rect(field[0], field[1], field[2], field[3]),
        );
        self.order.push(NOTE_TOKEN);
    }

    /// Every medal of the catalogue with what the player holds, its rank's button (each
    /// press the next rank, V back to I; on a medal not held, the rank Give gives it at),
    /// Give and Take back.
    fn medals(&mut self, frame: &Frame, profile: &Profile) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            SECTION_TOP - 20.0,
            MIDDLE_WIDTH,
            &crate::terms::say("Medals"),
        );
        for medal in Medal::ALL {
            let y = SECTION_TOP + medal.index() as f32 * MEDAL_ROW;
            let held = self.shown.medals[medal.index()];
            let rank = if held > 0 {
                self.shown.ranks[medal.index()]
            } else {
                self.give_ranks[medal.index()]
            };
            let icon = frame.rect(MIDDLE_X, y + 4.0, 54.0, 54.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: icon,
                texture: medal.icon(),
                color: Color::new(1.0, 1.0, 1.0, if held > 0 { 1.0 } else { 0.35 }),
            });
            if held > 0 {
                crate::medals::rank::icon_device(self.ui.draw_list_mut(), rank, icon);
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", medal.label(held)),
                frame.rect(MIDDLE_X + 68.0, y + 4.0, 136.0, 28.0),
                21.0 * s,
                if held > 0 {
                    color::GOLD_BRIGHT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let given = profile
                .medals
                .iter()
                .find(|entry| entry.id == medal.id())
                .map(|entry| crate::medals::date_text(entry.awarded))
                .filter(|date| !date.is_empty());
            let line = match (held, given) {
                (0, _) => format!("Not held \u{b7} gives at {}", rank.numeral()),
                (_, Some(date)) => format!("{} \u{b7} {date}", rank.name()),
                (_, None) => rank.name().to_owned(),
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(MIDDLE_X + 68.0, y + 32.0, 136.0, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let can_give = held == 0 || medal.repeatable();
            let give = GIVE_BASE + medal.index() as u16;
            let take = TAKE_BASE + medal.index() as u16;
            let step = RANK_BASE + medal.index() as u16;
            let right = MIDDLE_X + MIDDLE_WIDTH;
            if held > 0 || can_give {
                kit::button(
                    &mut self.ui,
                    frame,
                    [right - 312.0, y + 10.0, 92.0, 40.0],
                    &format!("Rank {}", rank.numeral()),
                    false,
                    self.shown.creator,
                    self.focus == step,
                    step,
                );
                self.order.push(step);
            }
            kit::button(
                &mut self.ui,
                frame,
                [right - 212.0, y + 10.0, 84.0, 40.0],
                if held > 0 && medal.repeatable() {
                    "Give +1"
                } else {
                    "Give"
                },
                true,
                can_give && self.shown.creator,
                self.focus == give,
                give,
            );
            kit::button(
                &mut self.ui,
                frame,
                [right - 120.0, y + 10.0, 120.0, 40.0],
                "Take back",
                false,
                held > 0 && self.shown.creator,
                self.focus == take,
                take,
            );
            if can_give {
                self.order.push(give);
            }
            if held > 0 {
                self.order.push(take);
            }
        }
    }

    /// Every unlockable of the catalogue, held or not, with Unlock and Relock, beside the
    /// medals.
    fn unlockables(&mut self, frame: &Frame, profile: &Profile) {
        let s = frame.s;
        let top = SECTION_TOP - 20.0;
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            top,
            RIGHT_WIDTH,
            "Unlockables",
        );
        let cell = (RIGHT_WIDTH - UNLOCK_GAP) * 0.5;
        let rows = ((KEYS_Y - 16.0 - (top + 18.0)) / UNLOCK_ROW).floor() as usize;
        let per_page = (rows * 2).max(2);
        self.shown.unlock_per_page = per_page;
        let pages = crate::unlockables::ALL.len().div_ceil(per_page);
        self.unlock_page = self.unlock_page.min(pages - 1);
        if pages > 1 {
            let page = self.unlock_page;
            let buttons = [
                (
                    UNLOCK_PREV_TOKEN,
                    "<",
                    RIGHT_X + RIGHT_WIDTH - 148.0,
                    page > 0,
                ),
                (
                    UNLOCK_NEXT_TOKEN,
                    ">",
                    RIGHT_X + RIGHT_WIDTH - 40.0,
                    page + 1 < pages,
                ),
            ];
            for (token, label, x, enabled) in buttons {
                kit::button(
                    &mut self.ui,
                    frame,
                    [x, top - 4.0, 40.0, 26.0],
                    label,
                    false,
                    enabled,
                    self.focus == token,
                    token,
                );
                if enabled {
                    self.order.push(token);
                }
            }
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{} / {pages}", page + 1),
                frame.rect(RIGHT_X + RIGHT_WIDTH - 104.0, top, 60.0, 20.0),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        let first = self.unlock_page * per_page;
        let last = (first + per_page).min(crate::unlockables::ALL.len());
        for (index, unlockable) in crate::unlockables::ALL
            .iter()
            .enumerate()
            .take(last)
            .skip(first)
        {
            let slot = index - first;
            let x = RIGHT_X + (slot % 2) as f32 * (cell + UNLOCK_GAP);
            let y = top + 18.0 + (slot / 2) as f32 * UNLOCK_ROW;
            let held = self.shown.unlocks.get(index).copied().unwrap_or(false);
            // Held: gold, with the day it was given; not held: muted.
            let granted = profile
                .unlocks
                .iter()
                .find(|unlock| unlock.id == unlockable.id)
                .map(|unlock| crate::medals::date_text(unlock.granted))
                .filter(|date| !date.is_empty());
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", unlockable.name),
                frame.rect(x, y + 1.0, cell - 96.0, 20.0),
                16.0 * s,
                if held {
                    color::GOLD_BRIGHT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let line = match (held, granted) {
                (false, _) => "Not held".to_owned(),
                (true, Some(date)) => format!("Since {date}"),
                (true, None) => "Held".to_owned(),
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(x, y + 20.0, cell - 96.0, 15.0),
                12.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            // One button: Unlock when not held, Relock when held.
            let token = if held {
                RELOCK_BASE + index as u16
            } else {
                UNLOCK_BASE + index as u16
            };
            kit::button(
                &mut self.ui,
                frame,
                [x + cell - 88.0, y + 3.0, 88.0, 30.0],
                if held { "Relock" } else { "Unlock" },
                !held,
                self.shown.creator,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
    }

    /// What the chosen player holds of holocrons, the tier Give holocron gives and the
    /// recent ones with Remove.
    fn holocrons(&mut self, frame: &Frame, profile: &Profile) {
        use crate::holocrons::{self, SLOTS, TIERS, slot};
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            HOLOCRONS_TOP - 36.0,
            MIDDLE_WIDTH,
            &crate::terms::say("Holocrons and kyber crystals"),
        );
        let counts = holocrons::counts_of(&profile.holocron_counts);
        let crystals = crystal_counts(profile);
        let holocrons_word = crate::terms::say("Holocrons");
        for (line, (what, held)) in [(&*holocrons_word, counts), ("Crystals", crystals)]
            .into_iter()
            .enumerate()
        {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{what}: {} uncommon, {} rare, {} legendary, {} mythical unopened",
                    held[0], held[1], held[2], held[3]
                ),
                frame.rect(
                    MIDDLE_X,
                    HOLOCRONS_TOP - 8.0 + line as f32 * 20.0,
                    MIDDLE_WIDTH,
                    20.0,
                ),
                14.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // The tier chips, holocrons on the first row and kyber crystals on the second:
        // its gem and name, the chosen one ringed in its colour.
        let chips_y = HOLOCRONS_TOP + 40.0;
        let row_step = TIER_CHIP[1] + 8.0;
        for index in 0..SLOTS {
            let tier = slot(index);
            let token = HOLOCRON_TIER_BASE + index as u16;
            let column = index % TIERS.len();
            let x = MIDDLE_X + column as f32 * (TIER_CHIP[0] + 8.0);
            let y = chips_y + (index / TIERS.len()) as f32 * row_step;
            kit::button(
                &mut self.ui,
                frame,
                [x, y, TIER_CHIP[0], TIER_CHIP[1]],
                "",
                false,
                true,
                self.focus == token,
                token,
            );
            let chosen = self.holocron_tier == index;
            if chosen {
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(x, y, TIER_CHIP[0], TIER_CHIP[1]),
                    radius: TIER_CHIP[1] * 0.5 * s,
                    width: 2.0 * s,
                    color: tier.colour,
                });
            }
            let [gem_x, gem_y] = frame.point(x + 24.0, y + TIER_CHIP[1] * 0.5);
            holocrons::gem::draw(
                self.ui.draw_list_mut(),
                [gem_x, gem_y],
                if tier.is_kyber() { 7.0 } else { 9.0 } * s,
                tier.colour,
                if chosen { 1.0 } else { 0.7 },
                holocrons::gem::MARK_ROWS,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", tier.label),
                frame.rect(x + 38.0, y + 6.0, TIER_CHIP[0] - 42.0, 30.0),
                16.0 * s,
                if chosen { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            self.order.push(token);
        }
        // Give, as tall as both rows, gives the chosen one.
        let give_x = MIDDLE_X + TIERS.len() as f32 * (TIER_CHIP[0] + 8.0) + 4.0;
        kit::button(
            &mut self.ui,
            frame,
            [
                give_x,
                chips_y,
                MIDDLE_X + MIDDLE_WIDTH - give_x,
                row_step + TIER_CHIP[1],
            ],
            "Give",
            true,
            self.shown.creator,
            self.focus == HOLOCRON_GIVE_TOKEN,
            HOLOCRON_GIVE_TOKEN,
        );
        self.order.push(HOLOCRON_GIVE_TOKEN);
        let chosen = slot(self.holocron_tier);
        let gives = if chosen.is_kyber() {
            format!(
                "Give: {} {}, opened to a saber shader",
                chosen.article(),
                chosen.name
            )
        } else {
            format!("Give: {} {}", chosen.article(), chosen.name)
        };
        let rows_y = chips_y + row_step + TIER_CHIP[1] + 8.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{gives}"),
            frame.rect(MIDDLE_X, rows_y, MIDDLE_WIDTH, 20.0),
            14.0 * s,
            chosen.colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        // The recent ones, holocrons and crystals, newest first.
        let entries: Vec<(holocrons::Entry, bool)> = profile
            .holocrons
            .iter()
            .filter_map(|wire| {
                holocrons::Entry::from_wire(wire).map(|entry| (entry, wire.opened.is_some()))
            })
            .collect();
        let rows_y = rows_y + 28.0;
        if entries.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", crate::terms::say("No holocrons or crystals yet.")),
                frame.rect(MIDDLE_X, rows_y, MIDDLE_WIDTH, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        for (row, (entry, opened)) in entries.iter().take(HOLOCRONS_SHOWN).enumerate() {
            let y = rows_y + row as f32 * HOLOCRON_ROW;
            let token = HOLOCRON_REMOVE_BASE + row as u16;
            let [gem_x, gem_y] = frame.point(MIDDLE_X + 14.0, y + HOLOCRON_ROW * 0.5 - 2.0);
            holocrons::gem::draw(
                self.ui.draw_list_mut(),
                [gem_x, gem_y],
                if entry.tier.is_kyber() { 8.0 } else { 10.0 } * s,
                entry.tier.colour,
                if *opened { 0.45 } else { 1.0 },
                holocrons::gem::MARK_ROWS,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", entry.tier.title()),
                frame.rect(MIDDLE_X + 34.0, y, 190.0, 28.0),
                17.0 * s,
                entry.tier.colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let kind = match (entry.earned, entry.gift, entry.traded.is_some()) {
                (true, ..) => "earned",
                (_, true, _) => "gift",
                (.., true) => "trade",
                _ => "play",
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "#{} {} {kind}{}",
                    entry.id,
                    holocrons::when_text(entry.dropped),
                    if *opened { ", opened" } else { "" }
                ),
                frame.rect(MIDDLE_X + 196.0, y + 3.0, MIDDLE_WIDTH - 332.0, 24.0),
                12.5 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            let remove = if self.confirming(token) {
                "Press again"
            } else {
                "Remove"
            };
            kit::button(
                &mut self.ui,
                frame,
                [MIDDLE_X + MIDDLE_WIDTH - 124.0, y - 1.0, 124.0, 30.0],
                remove,
                false,
                self.shown.creator,
                self.focus == token,
                token,
            );
            self.order.push(token);
            self.shown.holocrons[row] = entry.id;
            self.shown.holocron_rows = row + 1;
        }
        if entries.len() > HOLOCRONS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("and {} more", entries.len() - HOLOCRONS_SHOWN),
                frame.rect(
                    MIDDLE_X,
                    rows_y + HOLOCRONS_SHOWN as f32 * HOLOCRON_ROW,
                    MIDDLE_WIDTH,
                    20.0,
                ),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The player's achievements with a count, each with Clear, and Clear all.
    fn achievements(&mut self, frame: &Frame, profile: &Profile, mine: bool) {
        let s = frame.s;
        let confirming = self.confirming(CLEAR_ALL_TOKEN);
        kit::button(
            &mut self.ui,
            frame,
            [
                RIGHT_X + RIGHT_WIDTH - 140.0,
                SECTION_TOP - 26.0,
                140.0,
                34.0,
            ],
            if confirming {
                "Press again"
            } else {
                "Clear all"
            },
            false,
            !profile.achievements.is_empty() && self.shown.creator,
            self.focus == CLEAR_ALL_TOKEN,
            CLEAR_ALL_TOKEN,
        );
        if !profile.achievements.is_empty() {
            self.order.push(CLEAR_ALL_TOKEN);
        }
        let top = SECTION_TOP + 24.0;
        let listed: Vec<(usize, &sjk_identity::Achievement)> = achievements::ALL
            .iter()
            .enumerate()
            .filter_map(|(index, kind)| {
                profile
                    .achievements
                    .iter()
                    .find(|held| held.id == kind.id && held.progress > 0)
                    .map(|held| (index, held))
            })
            .collect();
        if listed.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("No achievements at the hub yet."),
                frame.rect(MIDDLE_X, top, MIDDLE_WIDTH, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        for (row, (index, held)) in listed.iter().take(ACHIEVEMENTS_SHOWN).enumerate() {
            let kind = &achievements::ALL[*index];
            let (x, width) = if row < ACHIEVEMENTS_PER_COLUMN {
                (MIDDLE_X, MIDDLE_WIDTH)
            } else {
                (RIGHT_X, RIGHT_WIDTH)
            };
            let y = top + (row % ACHIEVEMENTS_PER_COLUMN) as f32 * ACHIEVEMENT_ROW;
            let token = CLEAR_BASE + *index as u16;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", crate::terms::say(kind.name)),
                frame.rect(x, y, 230.0, 28.0),
                19.0 * s,
                if held.unlocked > 0 {
                    color::GOLD_BRIGHT
                } else {
                    color::TEXT
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            let status = if held.unlocked > 0 {
                crate::medals::date_text(held.unlocked)
            } else {
                format!(
                    "{} / {}",
                    kind.amount(held.progress),
                    kind.amount(kind.goal)
                )
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{status}"),
                frame.rect(x + 236.0, y + 2.0, width - 350.0, 24.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            kit::button(
                &mut self.ui,
                frame,
                [x + width - 96.0, y - 2.0, 96.0, 30.0],
                "Clear",
                false,
                self.shown.creator,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        let mut y = top + listed.len().clamp(1, ACHIEVEMENTS_PER_COLUMN) as f32 * ACHIEVEMENT_ROW;
        if listed.len() > ACHIEVEMENTS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("and {} more", listed.len() - ACHIEVEMENTS_SHOWN),
                frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 26.0;
        }
        let note = if mine {
            "Clearing your own also resets your game's count, so the next kill, duel or map unlocks it again."
        } else {
            "Their game sends its own counts again: only what the hub counts stays cleared."
        };
        for part in wrap(note, 110) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(MIDDLE_X, y + 6.0, PANE_WIDTH, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 22.0;
        }
    }

    /// The Overview tab: the chosen player's account with Verify in the middle column,
    /// and on the right the moderation of their picture and of the SJK chat.
    fn overview(&mut self, frame: &Frame, profile: &Profile, mine: bool) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            SECTION_TOP - 20.0,
            MIDDLE_WIDTH,
            "Account",
        );
        let trust = match profile.trust {
            Some(sjk_identity::Trust::New) => "new",
            Some(sjk_identity::Trust::Trusted) => "trusted",
            Some(sjk_identity::Trust::Banned) => "banned from the chat now",
            _ => "not said",
        };
        let since = crate::medals::date_text(profile.created);
        let counts = crate::holocrons::counts_of(&profile.holocron_counts);
        let unlocked = profile
            .achievements
            .iter()
            .filter(|held| held.unlocked > 0)
            .count();
        let seen = crate::medals::date_text(last_seen(profile));
        let crystals = crystal_counts(profile);
        let mut rows = vec![
            (
                "Flags",
                match (profile.staff, profile.verified) {
                    _ if profile.creator && profile.verified => "creator, verified".to_owned(),
                    _ if profile.creator => "creator".to_owned(),
                    (true, true) => "staff, verified".to_owned(),
                    (true, false) => "staff".to_owned(),
                    (false, true) => "verified".to_owned(),
                    (false, false) => "none".to_owned(),
                },
            ),
            ("Trust", trust.to_owned()),
            (
                "Member since",
                if since.is_empty() {
                    "?".to_owned()
                } else {
                    since
                },
            ),
            (
                "Last seen",
                if seen.is_empty() {
                    "not in a game yet".to_owned()
                } else {
                    seen
                },
            ),
            (
                "Keys",
                format!(
                    "1 main, {} linked (never shown)",
                    profile.linked_keys().count()
                ),
            ),
            (
                "Medals",
                format!(
                    "{} held, {} unlockables",
                    profile.medals.len(),
                    profile.unlocks.len()
                ),
            ),
            (
                "Achievements",
                format!(
                    "{unlocked} unlocked, {} counted",
                    profile.achievements.len()
                ),
            ),
            (
                "Holocrons",
                format!(
                    "{} uncommon, {} rare, {} legendary, {} mythical unopened",
                    counts[0], counts[1], counts[2], counts[3]
                ),
            ),
            (
                "Kyber crystals",
                format!(
                    "{} uncommon, {} rare, {} legendary, {} mythical unopened",
                    crystals[0], crystals[1], crystals[2], crystals[3]
                ),
            ),
            (
                "Wallet",
                format!(
                    "{} credits, {} kyber shards, {} kinds of card",
                    profile.credits,
                    profile.shards,
                    profile.cards.len()
                ),
            ),
        ];
        if mine {
            rows.push(("You", "this is your own player".to_owned()));
        }
        let mut y = SECTION_TOP + 4.0;
        for (label, value) in &rows {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", crate::terms::say(label)),
                frame.rect(MIDDLE_X, y, 140.0, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", cut(value, 52)),
                frame.rect(MIDDLE_X + 150.0, y, MIDDLE_WIDTH - 150.0, 24.0),
                15.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 30.0;
        }
        // Verify shows what pressing it does: Unverify while the player is verified.
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, y + 6.0, 170.0, 40.0],
            if profile.verified {
                "Unverify"
            } else {
                "Verify"
            },
            !profile.verified,
            true,
            self.focus == VERIFY_TOKEN,
            VERIFY_TOKEN,
        );
        self.order.push(VERIFY_TOKEN);
        // The creator's own: make the player staff or take it away, on a second press.
        // Never shown on the creator, who is always staff.
        if self.shown.creator && !profile.creator {
            let confirming = self.confirming(SET_STAFF_TOKEN);
            kit::button(
                &mut self.ui,
                frame,
                [MIDDLE_X + 182.0, y + 6.0, 170.0, 40.0],
                match (confirming, profile.staff) {
                    (true, _) => "Press again",
                    (false, true) => "Remove staff",
                    (false, false) => "Make staff",
                },
                false,
                true,
                self.focus == SET_STAFF_TOKEN,
                SET_STAFF_TOKEN,
            );
            self.order.push(SET_STAFF_TOKEN);
        }
        y += 56.0;
        // The tick buttons: the one drawn now lit, each with its tick.
        for (index, tick) in sjk_identity::Tick::GROUPS.into_iter().enumerate() {
            let token = TICK_BASE + index as u16;
            let x = MIDDLE_X + index as f32 * 120.0;
            kit::button(
                &mut self.ui,
                frame,
                [x, y, 112.0, 32.0],
                tick.label(),
                self.shown.tick == tick,
                true,
                self.focus == token,
                token,
            );
            if let Some(texture) = crate::ui_renderer::tick_texture(tick) {
                let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: frame.rect(x + 8.0, y + 6.0, 20.0, 20.0),
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, 1.0),
                });
            }
            self.order.push(token);
        }
        y += 52.0;
        kit::heading(&mut self.ui, frame, MIDDLE_X, y, MIDDLE_WIDTH, "Names worn");
        y += 20.0;
        if profile.names.is_empty() {
            self.column_line(frame, MIDDLE_X, y, "None the hub kept.");
        }
        for worn in profile.names.iter().take(6) {
            let seen = crate::medals::date_text(worn.last_seen);
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", cut(&worn.name, 30)),
                frame.rect(MIDDLE_X, y, 340.0, 26.0),
                17.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("last {seen}"),
                frame.rect(MIDDLE_X + 340.0, y + 2.0, MIDDLE_WIDTH - 340.0, 24.0),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            y += 28.0;
        }
        self.moderation(frame, profile);
    }

    /// The Overview's right column: the player's picture and their SJK chat.
    fn moderation(&mut self, frame: &Frame, profile: &Profile) {
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            SECTION_TOP - 20.0,
            RIGHT_WIDTH,
            "Picture",
        );
        let y = SECTION_TOP + 4.0;
        self.column_line(
            frame,
            RIGHT_X,
            y,
            if profile.avatar.is_empty() {
                "No picture up now."
            } else {
                "A picture is up."
            },
        );
        let third = (RIGHT_WIDTH - 16.0) / 3.0;
        for (column, (label, enabled, token)) in [
            (
                "Take down",
                self.shown.picture && self.shown.creator,
                PICTURE_DOWN_TOKEN,
            ),
            ("Block pictures", true, AVATAR_BLOCK_TOKEN),
            ("Allow pictures", true, AVATAR_ALLOW_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            kit::button(
                &mut self.ui,
                frame,
                [
                    RIGHT_X + column as f32 * (third + 8.0),
                    y + 32.0,
                    third,
                    40.0,
                ],
                label,
                false,
                enabled,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        self.column_wrapped(
            frame,
            RIGHT_X,
            y + 84.0,
            "Block pictures takes theirs down and stops new ones until Allow pictures; \
             the hub does not say which holds now.",
        );
        let y = y + 170.0;
        kit::heading(&mut self.ui, frame, RIGHT_X, y, RIGHT_WIDTH, "SJK chat");
        for (column, (label, token)) in [
            ("Mute at the hub", HUB_MUTE_TOKEN),
            ("Unmute", HUB_UNMUTE_TOKEN),
            ("Lift chat bans", CHAT_UNBAN_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            kit::button(
                &mut self.ui,
                frame,
                [
                    RIGHT_X + column as f32 * (third + 8.0),
                    y + 24.0,
                    third,
                    40.0,
                ],
                label,
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        self.column_wrapped(
            frame,
            RIGHT_X,
            y + 76.0,
            "Muted at the hub, nobody reads their SJK chat until Unmute. Lift chat bans \
             frees them of every 24-hour ban verified players gave (Chat bans lists them).",
        );
    }

    /// The Holocrons and credits tab's right column: the player's credits and kyber
    /// shards, the amount, and Give and Take for each.
    fn wallet(&mut self, frame: &Frame, profile: &Profile) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            SECTION_TOP - 20.0,
            RIGHT_WIDTH,
            "Credits and kyber shards",
        );
        let mut y = SECTION_TOP + 4.0;
        for (label, value) in [
            ("Credits", profile.credits),
            ("Kyber shards", profile.shards),
        ] {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(RIGHT_X, y, 240.0, 30.0),
                20.0 * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{value}"),
                frame.rect(RIGHT_X + 240.0, y, RIGHT_WIDTH - 240.0, 30.0),
                20.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Semibold,
                TextAlign::End,
            );
            y += 36.0;
        }
        y += 8.0;
        self.column_line(frame, RIGHT_X, y, "Amount");
        let field = [RIGHT_X, y + 26.0, 200.0, 44.0];
        let focused = self.focus == AMOUNT_TOKEN;
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        kit::field(
            &mut self.ui,
            frame,
            field,
            format_args!("{}{}", self.amount, if caret { "|" } else { "" }),
            focused,
            false,
        );
        self.ui.hit_region(
            AMOUNT_TOKEN,
            frame.rect(field[0], field[1], field[2], field[3]),
        );
        self.order.push(AMOUNT_TOKEN);
        let ready = self.amount().is_some();
        let half = (RIGHT_WIDTH - 8.0) * 0.5;
        y += 84.0;
        for (row, (give, take, give_token, take_token)) in [
            (
                "Give credits",
                "Take credits",
                CREDITS_GIVE_TOKEN,
                CREDITS_TAKE_TOKEN,
            ),
            (
                "Give shards",
                "Take shards",
                SHARDS_GIVE_TOKEN,
                SHARDS_TAKE_TOKEN,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let row_y = y + row as f32 * 50.0;
            kit::button(
                &mut self.ui,
                frame,
                [RIGHT_X, row_y, half, 40.0],
                give,
                true,
                ready && self.shown.creator,
                self.focus == give_token,
                give_token,
            );
            let take = if self.confirming(take_token) {
                "Press again"
            } else {
                take
            };
            kit::button(
                &mut self.ui,
                frame,
                [RIGHT_X + half + 8.0, row_y, half, 40.0],
                take,
                false,
                ready && self.shown.creator,
                self.focus == take_token,
                take_token,
            );
            if ready {
                self.order.push(give_token);
                self.order.push(take_token);
            }
        }
        self.column_wrapped(
            frame,
            RIGHT_X,
            y + 110.0,
            "The note goes with it. Taking waits for a second press; the hub refuses \
             taking more than they have.",
        );
    }

    /// The Chat bans tab: every SJK chat ban in force, newest first, each with Lift.
    fn chat_bans(&mut self, frame: &Frame, staff: &StaffState) {
        let s = frame.s;
        let mut y = SECTION_TOP + 6.0;
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + PANE_WIDTH - 140.0, y, 140.0, 38.0],
            "Refresh",
            false,
            true,
            self.focus == CHAT_BANS_REFRESH_TOKEN,
            CHAT_BANS_REFRESH_TOKEN,
        );
        self.order.push(CHAT_BANS_REFRESH_TOKEN);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Chat bans in force, newest first"),
            frame.rect(MIDDLE_X, y + 4.0, PANE_WIDTH - 150.0, 30.0),
            20.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 54.0;
        if staff.chat_bans.is_empty() {
            self.right_line(frame, y, "No chat ban in force.", color::MUTED);
        }
        for (row, ban) in staff.chat_bans.iter().take(BAN_ROWS_SHOWN).enumerate() {
            let token = BAN_LIFT_BASE + row as u16;
            let row_y = y + row as f32 * LOG_ROW;
            self.shown.ban_rows.push(ban.key_id.clone());
            let name = if ban.name.is_empty() {
                "(an address)".to_owned()
            } else {
                cut(&ban.name, 22)
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(MIDDLE_X, row_y + 2.0, 230.0, 26.0),
                18.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let by = if ban.by_name.is_empty() {
                "a player".to_owned()
            } else {
                cut(&crate::medals::plain_note(&ban.by_name), 16)
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "by {by}, until {}{}",
                    sjk_identity::ban::local_day_time(ban.until),
                    if ban.address { ", address too" } else { "" }
                ),
                frame.rect(MIDDLE_X + 236.0, row_y + 4.0, PANE_WIDTH - 350.0, 24.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let liftable = !ban.key_id.is_empty();
            kit::button(
                &mut self.ui,
                frame,
                [MIDDLE_X + PANE_WIDTH - 100.0, row_y - 1.0, 100.0, 30.0],
                "Lift",
                false,
                liftable,
                self.focus == token,
                token,
            );
            if liftable {
                self.order.push(token);
            }
        }
        if staff.chat_bans.len() > BAN_ROWS_SHOWN {
            self.right_line(
                frame,
                y + BAN_ROWS_SHOWN as f32 * LOG_ROW,
                &format!("and {} more", staff.chat_bans.len() - BAN_ROWS_SHOWN),
                color::QUIET,
            );
        }
    }

    /// One line of quiet text in a column at `x`, `y`.
    fn column_line(&mut self, frame: &Frame, x: f32, y: f32, line: &str) {
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(x, y, RIGHT_WIDTH, 22.0),
            15.0 * frame.s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// Quiet text wrapped to a column at `x`, from `y`.
    fn column_wrapped(&mut self, frame: &Frame, x: f32, y: f32, note: &str) {
        for (row, part) in wrap(note, 66).enumerate() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(x, y + row as f32 * 21.0, RIGHT_WIDTH, 21.0),
                14.0 * frame.s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The tabs, the one shown filled gold: the chosen player's, then every player's.
    fn tabs(&mut self, frame: &Frame) {
        let mut x = MIDDLE_X;
        for (label, width, view, token) in [
            ("Overview", 116.0, View::Overview, OVERVIEW_TAB_TOKEN),
            (
                "Medals and unlocks",
                186.0,
                View::Rewards,
                REWARDS_TAB_TOKEN,
            ),
            (
                "Holocrons and credits",
                200.0,
                View::Economy,
                ECONOMY_TAB_TOKEN,
            ),
            (
                "Achievements",
                140.0,
                View::Achievements,
                ACHIEVEMENTS_TAB_TOKEN,
            ),
            ("Keys", 74.0, View::Keys, KEYS_TAB_TOKEN),
            ("Logs", 72.0, View::Logs, LOGS_TAB_TOKEN),
            ("Crashes", 104.0, View::Crashes, CRASHES_TAB_TOKEN),
            ("Chat bans", 112.0, View::ChatBans, CHAT_BANS_TAB_TOKEN),
        ] {
            // Crashes and Chat bans are every player's: a gap sets them apart.
            if token == CRASHES_TAB_TOKEN {
                x += 14.0;
            }
            kit::button(
                &mut self.ui,
                frame,
                [x, TABS_Y, width, 38.0],
                &crate::terms::say(label),
                self.view == view,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
            x += width + 8.0;
        }
    }

    /// One line of quiet text across a one-column view at `y`.
    fn right_line(&mut self, frame: &Frame, y: f32, line: &str, colour: Color) {
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(MIDDLE_X, y, PANE_WIDTH, 22.0),
            14.0 * frame.s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// Request logs for the chosen player (with the note), then the log requests,
    /// newest first, each sent one with Open.
    fn logs(&mut self, frame: &Frame, profile: &Profile, staff: &StaffState) {
        let s = frame.s;
        let name = if profile.name.is_empty() {
            "this player".to_owned()
        } else {
            cut(&crate::medals::plain_note(&profile.name), 24)
        };
        let mut y = SECTION_TOP + 6.0;
        let open = staff
            .log_requests
            .iter()
            .any(|row| row.key_id == profile.key_id && row.state == "open");
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, y, 200.0, 38.0],
            "Request logs",
            true,
            !open,
            self.focus == REQUEST_LOGS_TOKEN,
            REQUEST_LOGS_TOKEN,
        );
        if !open {
            self.order.push(REQUEST_LOGS_TOKEN);
        }
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + PANE_WIDTH - 120.0, y, 120.0, 38.0],
            "Refresh",
            false,
            true,
            self.focus == LOGS_REFRESH_TOKEN,
            LOGS_REFRESH_TOKEN,
        );
        self.order.push(LOGS_REFRESH_TOKEN);
        y += 50.0;
        let explain = if open {
            format!("{name} has a request open; their game asks them when they are in the menu.")
        } else {
            format!(
                "Asks {name} {}",
                crate::terms::say(
                    "for their game's logs, with the note under the medals. Their game asks \
                     them first; nothing comes without their yes."
                )
            )
        };
        for part in wrap(&explain, 72).take(2) {
            self.right_line(frame, y, part, color::QUIET);
            y += 22.0;
        }
        y += 18.0;
        kit::heading(&mut self.ui, frame, MIDDLE_X, y, PANE_WIDTH, "Log requests");
        y += 26.0;
        if staff.log_requests.is_empty() {
            self.right_line(
                frame,
                y,
                "None yet, or not read: Refresh reads them.",
                color::MUTED,
            );
        }
        for (row, request) in staff.log_requests.iter().take(LOG_ROWS_SHOWN).enumerate() {
            let token = LOG_OPEN_BASE + row as u16;
            let row_y = y + row as f32 * LOG_ROW;
            let sent = request.state == "sent";
            self.shown.log_rows.push((request.id, sent));
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", cut(&crate::medals::plain_note(&request.name), 18)),
                frame.rect(MIDDLE_X, row_y + 2.0, 190.0, 26.0),
                18.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let mut state = match request.state.as_str() {
                "sent" => format!(
                    "sent {}, {} KB",
                    crate::medals::date_text(request.answered),
                    request.size.div_ceil(1024)
                ),
                "open" => format!("asked {}", crate::medals::date_text(request.created)),
                other => format!("{other} {}", crate::medals::date_text(request.created)),
            };
            if !request.staff_name.is_empty() {
                state = format!(
                    "{state}, by {}",
                    cut(&crate::medals::plain_note(&request.staff_name), 10)
                );
            }
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{state}"),
                frame.rect(MIDDLE_X + 196.0, row_y + 4.0, 230.0, 24.0),
                14.0 * s,
                if sent {
                    color::GOLD_BRIGHT
                } else {
                    color::QUIET
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            if sent {
                kit::button(
                    &mut self.ui,
                    frame,
                    [MIDDLE_X + PANE_WIDTH - 84.0, row_y - 1.0, 84.0, 30.0],
                    "Open",
                    false,
                    true,
                    self.focus == token,
                    token,
                );
                self.order.push(token);
            }
        }
    }

    /// Every player's crash reports: the kinds (newest first) to choose, or the chosen
    /// kind's reports with Open, which saves one as a text file and opens it.
    fn crashes(&mut self, frame: &Frame, staff: &StaffState) {
        let s = frame.s;
        let mut y = SECTION_TOP + 6.0;
        let group = self
            .crash_group
            .clone()
            .filter(|signature| *signature == staff.crashes_of);
        let (token, label) = if self.crash_group.is_some() {
            (CRASHES_BACK_TOKEN, "All kinds")
        } else {
            (CRASHES_REFRESH_TOKEN, "Refresh")
        };
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + PANE_WIDTH - 140.0, y, 140.0, 38.0],
            label,
            false,
            true,
            self.focus == token,
            token,
        );
        self.order.push(token);
        let title = if self.crash_group.is_some() {
            "Reports of this kind"
        } else {
            "Kinds of crash, newest first"
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(MIDDLE_X, y + 4.0, PANE_WIDTH - 150.0, 30.0),
            20.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 54.0;
        match group {
            Some(_) => {
                if staff.crashes.is_empty() {
                    self.right_line(frame, y, "No report of this kind any more.", color::MUTED);
                }
                for (row, crash) in staff.crashes.iter().take(CRASH_ROWS_SHOWN).enumerate() {
                    let token = CRASH_OPEN_BASE + row as u16;
                    let row_y = y + row as f32 * LOG_ROW;
                    self.shown.crash_rows.push(crash.id);
                    text(
                        &mut self.ui,
                        TextFamily::Display,
                        format_args!("{}", cut(&crate::medals::plain_note(&crash.name), 16)),
                        frame.rect(MIDDLE_X, row_y + 2.0, 170.0, 26.0),
                        18.0 * s,
                        color::TEXT,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!(
                            "{}, {}",
                            crate::medals::date_text(crash.created),
                            cut(&crash.build, 14)
                        ),
                        frame.rect(MIDDLE_X + 176.0, row_y + 4.0, 250.0, 24.0),
                        14.0 * s,
                        color::QUIET,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                    kit::button(
                        &mut self.ui,
                        frame,
                        [MIDDLE_X + PANE_WIDTH - 84.0, row_y - 1.0, 84.0, 30.0],
                        "Open",
                        false,
                        true,
                        self.focus == token,
                        token,
                    );
                    self.order.push(token);
                }
            }
            None if self.crash_group.is_some() => {
                self.right_line(frame, y, "Reading...", color::MUTED);
            }
            None => {
                if staff.crash_groups.is_empty() {
                    self.right_line(
                        frame,
                        y,
                        "No crash reports, or not read: Refresh reads them.",
                        color::MUTED,
                    );
                }
                for (row, kind) in staff
                    .crash_groups
                    .iter()
                    .take(CRASH_GROUPS_SHOWN)
                    .enumerate()
                {
                    let token = CRASH_GROUP_BASE + row as u16;
                    let row_y = y + row as f32 * CRASH_GROUP_ROW;
                    self.shown.crash_groups.push(kind.signature.clone());
                    let place = kind.location.rsplit(['/', '\\']).next().unwrap_or_default();
                    kit::button(
                        &mut self.ui,
                        frame,
                        [MIDDLE_X, row_y - 2.0, PANE_WIDTH, CRASH_GROUP_ROW - 6.0],
                        "",
                        false,
                        true,
                        self.focus == token,
                        token,
                    );
                    self.order.push(token);
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!(
                            "{}{}{}, {} report{} from {} player{}, last {}",
                            kind.kind,
                            if place.is_empty() { "" } else { " in " },
                            cut(place, 24),
                            kind.count,
                            if kind.count == 1 { "" } else { "s" },
                            kind.keys,
                            if kind.keys == 1 { "" } else { "s" },
                            crate::medals::date_text(kind.last)
                        ),
                        frame.rect(MIDDLE_X + 12.0, row_y + 1.0, PANE_WIDTH - 24.0, 22.0),
                        15.0 * s,
                        color::TEXT,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!("{}", cut(&kind.message, 62)),
                        frame.rect(MIDDLE_X + 12.0, row_y + 22.0, PANE_WIDTH - 24.0, 20.0),
                        13.0 * s,
                        color::QUIET,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
            }
        }
    }

    /// The chosen player's keys, each linked one with Unlink, then the merge: the key
    /// of the player to merge away and Merge, or the confirmation saying who is kept
    /// and who goes.
    fn keys_and_merge(&mut self, frame: &Frame, profile: &Profile, players: &[Profile]) {
        let s = frame.s;
        let mut y = SECTION_TOP + 10.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Main key: the one this player started with (key ids never show)"),
            frame.rect(MIDDLE_X, y, PANE_WIDTH, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += 34.0;
        let linked: Vec<&str> = profile.linked_keys().collect();
        if linked.is_empty() {
            for part in wrap(
                "No linked keys. The keys of a player merged into this one are linked here.",
                72,
            ) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(MIDDLE_X, y, PANE_WIDTH, 22.0),
                    14.0 * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 22.0;
            }
        }
        for row in 0..linked.len().min(LINKED_SHOWN) {
            let token = UNLINK_BASE + row as u16;
            let row_y = y + row as f32 * LINKED_ROW;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("Linked key {}", row + 1),
                frame.rect(MIDDLE_X, row_y + 2.0, PANE_WIDTH - 150.0, 26.0),
                18.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let label = if self.confirming(token) {
                "Press again"
            } else {
                "Unlink"
            };
            kit::button(
                &mut self.ui,
                frame,
                [MIDDLE_X + PANE_WIDTH - 140.0, row_y - 1.0, 140.0, 30.0],
                label,
                false,
                self.shown.creator,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        if linked.len() > LINKED_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("and {} more", linked.len() - LINKED_SHOWN),
                frame.rect(
                    MIDDLE_X,
                    y + LINKED_SHOWN as f32 * LINKED_ROW,
                    PANE_WIDTH,
                    20.0,
                ),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        if !linked.is_empty() {
            let rows = linked.len().min(LINKED_SHOWN) as f32 * LINKED_ROW;
            let more = if linked.len() > LINKED_SHOWN {
                22.0
            } else {
                0.0
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("An unlinked key starts afresh; nothing moves back."),
                frame.rect(MIDDLE_X, y + rows + more + 4.0, PANE_WIDTH, 20.0),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            MERGE_TOP,
            PANE_WIDTH,
            "Merge another key into this player",
        );
        match self.pending_merge.clone() {
            Some(merge) => self.merge_confirmation(frame, profile, &merge, players),
            None => self.merge_entry(frame, players),
        }
    }

    /// The merge field with Pick in list, what is wrong with it, and Merge.
    fn merge_entry(&mut self, frame: &Frame, players: &[Profile]) {
        let s = frame.s;
        let mut y = MERGE_TOP + 30.0;
        for part in wrap(
            &crate::terms::say(
                "For a player who reset their key: this player is kept, the other one's medals, unlocks, achievements, holocrons and names move here and their keys become linked keys.",
            ),
            72,
        ) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(MIDDLE_X, y, PANE_WIDTH, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 22.0;
        }
        let y = MERGE_TOP + 104.0;
        let pick_width = 150.0;
        let field = [MIDDLE_X, y, PANE_WIDTH - pick_width - 10.0, 44.0];
        let focused = self.focus == MERGE_FIELD_TOKEN;
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        // The key never shows: dots, and the player it names when the list has them.
        let from = merge_key(&self.merge_from);
        let shown = match players.iter().find(|player| player.has_key(&from)) {
            Some(player) if !from.is_empty() => format!(
                "{} {}",
                hide_key(&from),
                crate::medals::plain_note(&player.name)
            ),
            _ => hide_key(&self.merge_from),
        };
        kit::field(
            &mut self.ui,
            frame,
            field,
            format_args!("{}{}", cut(&shown, 44), if caret { "|" } else { "" }),
            focused,
            false,
        );
        self.ui.hit_region(
            MERGE_FIELD_TOKEN,
            frame.rect(field[0], field[1], field[2], field[3]),
        );
        self.order.push(MERGE_FIELD_TOKEN);
        kit::button(
            &mut self.ui,
            frame,
            [
                MIDDLE_X + PANE_WIDTH - pick_width,
                y + 1.0,
                pick_width,
                42.0,
            ],
            if self.picking {
                "Picking..."
            } else {
                "Pick in list"
            },
            self.picking,
            true,
            self.focus == MERGE_PICK_TOKEN,
            MERGE_PICK_TOKEN,
        );
        self.order.push(MERGE_PICK_TOKEN);
        let (line, colour) = if self.picking {
            (
                "Choose the player to merge away in the list (Esc lets go)",
                color::GOLD_BRIGHT,
            )
        } else {
            match self.shown.merge_problem {
                None => (
                    "Ready: Merge asks once more before anything moves",
                    color::MUTED,
                ),
                Some(MergeProblem::Empty) => (MergeProblem::Empty.words(), color::QUIET),
                Some(problem) => (problem.words(), color::EMBER),
            }
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(MIDDLE_X, y + 54.0, PANE_WIDTH, 22.0),
            14.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let ready = self.shown.merge_problem.is_none();
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, y + 88.0, 180.0, 42.0],
            "Merge...",
            ready,
            ready && self.shown.creator,
            self.focus == MERGE_TOKEN,
            MERGE_TOKEN,
        );
        if ready {
            self.order.push(MERGE_TOKEN);
        }
    }

    /// The merge's confirmation: who is kept, who goes, that it cannot be undone, and
    /// Merge for good or Cancel.
    fn merge_confirmation(
        &mut self,
        frame: &Frame,
        kept: &Profile,
        merge: &PendingMerge,
        players: &[Profile],
    ) {
        let s = frame.s;
        let card = [MIDDLE_X - 12.0, MERGE_TOP + 26.0, PANE_WIDTH + 24.0, 316.0];
        kit::card(&mut self.ui, frame, card);
        let named = |name: &str| {
            if name.is_empty() {
                "(no name yet)".to_owned()
            } else {
                cut(name, 30)
            }
        };
        let gone = players
            .iter()
            .find(|player| player.has_key(&merge.from))
            .map_or_else(|| "a player not in the list".to_owned(), |p| named(&p.name));
        let mut y = MERGE_TOP + 42.0;
        // Who goes and who stays by name and when they were seen, never by key.
        let seen_of = |profile: Option<&Profile>| {
            let seen = profile.map_or(0, last_seen);
            match crate::medals::date_text(seen) {
                date if date.is_empty() => "not seen in a game yet".to_owned(),
                date => format!("last seen {date}"),
            }
        };
        let gone_profile = players.iter().find(|player| player.has_key(&merge.from));
        for (label, who, about, colour) in [
            (
                "Kept",
                named(&kept.name),
                seen_of(Some(kept)),
                color::GOLD_BRIGHT,
            ),
            ("Merged away", gone, seen_of(gone_profile), color::EMBER),
        ] {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}: {who}"),
                frame.rect(MIDDLE_X, y, PANE_WIDTH, 28.0),
                20.0 * s,
                colour,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{about}"),
                frame.rect(MIDDLE_X, y + 28.0, PANE_WIDTH, 20.0),
                14.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 58.0;
        }
        for part in wrap(
            "Everything of the merged-away player moves to the kept one, and their keys become its linked keys. This cannot be undone: Unlink only detaches a key, nothing moves back.",
            68,
        ) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(MIDDLE_X, y, PANE_WIDTH, 22.0),
                14.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 22.0;
        }
        let buttons_y = MERGE_TOP + 26.0 + 316.0 - 58.0;
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X, buttons_y, 200.0, 42.0],
            "Merge for good",
            true,
            true,
            self.focus == MERGE_CONFIRM_TOKEN,
            MERGE_CONFIRM_TOKEN,
        );
        kit::button(
            &mut self.ui,
            frame,
            [MIDDLE_X + 212.0, buttons_y, 130.0, 42.0],
            "Cancel",
            false,
            true,
            self.focus == MERGE_CANCEL_TOKEN,
            MERGE_CANCEL_TOKEN,
        );
        self.order.push(MERGE_CONFIRM_TOKEN);
        self.order.push(MERGE_CANCEL_TOKEN);
    }

    /// What the last request came to, bottom left.
    fn status(&mut self, frame: &Frame, staff: &StaffState) {
        let s = frame.s;
        let (line, colour) = if staff.busy {
            ("Working...".to_owned(), color::MUTED)
        } else if staff.failed {
            (cut(&crate::terms::say(&staff.message), 90), color::EMBER)
        } else if staff.message.is_empty()
            && !self.shown.creator
            && matches!(
                self.view,
                View::Rewards | View::Economy | View::Achievements | View::Keys
            )
        {
            (
                "Only the owner gives, takes back, clears or merges; this tab is to read."
                    .to_owned(),
                color::MUTED,
            )
        } else {
            (
                cut(&crate::terms::say(&staff.message), 90),
                color::GOLD_BRIGHT,
            )
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X, KEYS_Y, 900.0, 24.0),
            16.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The keys of what has the keyboard, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let enter = match self.focus {
            SEARCH_TOKEN => "search",
            NOTE_TOKEN | MERGE_FIELD_TOKEN | AMOUNT_TOKEN => "",
            _ => "do",
        };
        let mut keys: Vec<(&[&str], &str)> = vec![(&["Tab"], "next")];
        if !enter.is_empty() {
            keys.push((&["Enter"], enter));
        }
        let escape = if self.pending_merge.is_some() || self.picking {
            "cancel"
        } else {
            "back"
        };
        keys.push((&["Esc"], escape));
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

#[cfg(test)]
mod tests {
    use super::super::tests::profile;
    use super::*;
    use sjk_identity::{Achievement, Medal as WireMedal};

    fn crowded() -> Profile {
        Profile {
            staff: true,
            creator: true,
            verified: true,
            name: "^1Sol^7Vulpes the Very Long Named One^5!".into(),
            medals: Medal::ALL
                .iter()
                .map(|medal| WireMedal {
                    id: medal.id().into(),
                    count: 3,
                    awarded: 1_791_336_225,
                    note: String::new(),
                    rank: 1,
                })
                .collect(),
            achievements: achievements::ALL
                .iter()
                .map(|kind| Achievement {
                    id: kind.id.into(),
                    progress: kind.goal / 2 + 1,
                    goal: kind.goal,
                    unlocked: 0,
                    reward: None,
                    name: None,
                    description: None,
                    group: None,
                })
                .collect(),
            unlocks: crate::unlockables::ALL
                .iter()
                .map(|unlockable| sjk_identity::Unlock {
                    holo: 0,
                    solar: 0,
                    id: unlockable.id.into(),
                    granted: 1_791_336_225,
                    note: "n".repeat(200),
                    count: 1,
                    medal: None,
                    card: None,
                })
                .collect(),
            holocron_counts: sjk_identity::HolocronCounts {
                uncommon: 999,
                rare: 999,
                legendary: 999,
                mythical: 999,
            },
            holocrons: (0..30)
                .map(|id| sjk_identity::Holocron {
                    opened: None,
                    traded: None,
                    traded_from_key: None,
                    traded_from: None,
                    edition: sjk_identity::wire::EDITION_FIRST.to_owned(),
                    id: 1_000 - id,
                    tier: crate::holocrons::TIERS[id as usize % crate::holocrons::COUNT]
                        .id
                        .into(),
                    dropped: 1_791_641_100,
                    source: if id % 2 == 0 { "staff" } else { "play" }.into(),
                    note: "n".repeat(200),
                })
                .collect(),
            keys: (0..12).map(|index| format!("{index:016x}")).collect(),
            ..profile("aaaaaaaaaaaaaaaa", "x")
        }
    }

    /// Every unlockable of the catalogue has its row, with Relock when held and Unlock
    /// when not.
    #[test]
    fn every_unlockable_has_its_row() {
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let staff = StaffState::default();
        let mut me = crowded();
        let mut panel = Panel::new();
        panel.open(true);
        panel.activate(REWARDS_TAB_TOKEN);
        panel.build(
            &Inputs {
                me: Some(&me),
                staff: &staff,
            },
            &inter.font,
            [1920.0, 1080.0],
        );
        let per_page = panel.shown.unlock_per_page;
        assert!(per_page >= 10, "a page shows several rows");
        let visible = |panel: &Panel, base: u16| -> Vec<u16> {
            (0..crate::unlockables::ALL.len() as u16)
                .filter(|index| panel.order.contains(&(base + index)))
                .collect()
        };
        // The first page holds the first rows; the pager reaches the others.
        assert_eq!(
            visible(&panel, RELOCK_BASE),
            (0..per_page.min(crate::unlockables::ALL.len()) as u16).collect::<Vec<_>>()
        );
        let mut seen = visible(&panel, RELOCK_BASE);
        let pages = crate::unlockables::ALL.len().div_ceil(per_page);
        for _ in 1..pages {
            panel.activate(UNLOCK_NEXT_TOKEN);
            panel.build(
                &Inputs {
                    me: Some(&me),
                    staff: &staff,
                },
                &inter.font,
                [1920.0, 1080.0],
            );
            seen.extend(visible(&panel, RELOCK_BASE));
        }
        assert_eq!(
            seen,
            (0..crate::unlockables::ALL.len() as u16).collect::<Vec<_>>(),
            "every row is on some page"
        );
        panel.activate(UNLOCK_NEXT_TOKEN);
        assert_eq!(panel.unlock_page, pages - 1, "stops at the last page");
        for _ in 0..pages {
            panel.activate(UNLOCK_PREV_TOKEN);
        }
        assert_eq!(panel.unlock_page, 0);
        me.unlocks.clear();
        panel.build(
            &Inputs {
                me: Some(&me),
                staff: &staff,
            },
            &inter.font,
            [1920.0, 1080.0],
        );
        for index in 0..per_page.min(crate::unlockables::ALL.len()) as u16 {
            assert!(panel.order.contains(&(UNLOCK_BASE + index)), "row {index}");
        }
    }

    /// Every focus, a full list and a crowded player fit the canvas at 1080p, 4K, 4:3
    /// and 21:9, in the families and in Inter, on every tab and with a merge waiting
    /// for its confirmation.
    #[test]
    fn every_state_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let me = crowded();
        let players: Vec<Profile> = (0..30)
            .map(|index| Profile {
                staff: index % 3 == 0,
                verified: index % 2 == 0,
                ..profile(
                    &format!("{index:016x}"),
                    "^3Someone^7 with a long name here",
                )
            })
            .collect();
        let long = "^3Someone^7 with a long name here".to_owned();
        let log_requests: Vec<sjk_identity::logs::LogRequestRow> = (0..40)
            .map(|index| sjk_identity::logs::LogRequestRow {
                id: index,
                key_id: format!("{index:016x}"),
                name: long.clone(),
                staff_name: long.clone(),
                state: ["sent", "open", "declined", "expired"][index as usize % 4].to_owned(),
                created: 1_791_720_000,
                answered: 1_791_720_000,
                size: 400_000,
                ..Default::default()
            })
            .collect();
        let crash_groups: Vec<sjk_identity::logs::CrashGroup> = (0..40)
            .map(|index| sjk_identity::logs::CrashGroup {
                signature: format!("{index:016x}"),
                kind: "exception".to_owned(),
                location: "crates/sjk-viewer/src/a_very_long_file_name_for_a_crash.rs:1:2"
                    .to_owned(),
                message: "m".repeat(300),
                count: 12_345,
                keys: 678,
                last: 1_791_720_000,
                ..Default::default()
            })
            .collect();
        let crashes: Vec<sjk_identity::logs::CrashSummary> = (0..60)
            .map(|index| sjk_identity::logs::CrashSummary {
                id: index,
                name: long.clone(),
                created: 1_791_720_000,
                build: "2026.1011.2-with-a-long-tail".to_owned(),
                ..Default::default()
            })
            .collect();
        let full = StaffState {
            players: players.clone(),
            message: "x".repeat(200),
            failed: true,
            log_requests,
            crash_groups,
            crashes,
            crashes_of: "0000000000000001".to_owned(),
            chat_bans: (0..40)
                .map(|index| sjk_identity::ChatBanRow {
                    id: index,
                    key_id: if index % 5 == 0 {
                        String::new()
                    } else {
                        format!("{index:016x}")
                    },
                    name: long.clone(),
                    address: index % 2 == 0,
                    by_key_id: format!("{index:016x}"),
                    by_name: long.clone(),
                    created: 1_791_720_000,
                    until: 1_791_806_400,
                })
                .collect(),
            ..StaffState::default()
        };
        for staff in [full, StaffState::default()].into_iter().flat_map(|staff| {
            [
                (staff.clone(), View::Overview, false),
                (staff.clone(), View::Rewards, false),
                (staff.clone(), View::Economy, false),
                (staff.clone(), View::ChatBans, false),
                (staff.clone(), View::Achievements, false),
                (staff.clone(), View::Keys, false),
                (staff.clone(), View::Keys, true),
                (staff.clone(), View::Logs, false),
                (staff.clone(), View::Crashes, false),
                (staff, View::Crashes, true),
            ]
        }) {
            let (staff, right, pending) = staff;
            let mut panel = Panel::new();
            panel.open(true);
            panel.note = "n".repeat(200);
            panel.query = "q".repeat(64);
            panel.view = right;
            panel.amount = "9".repeat(AMOUNT_MAX);
            panel.merge_from = "m".repeat(MERGE_FIELD_MAX);
            // In the Crashes view, `pending` shows one kind's reports.
            if pending && right == View::Crashes {
                panel.crash_group = Some("0000000000000001".into());
            } else if pending {
                panel.merge_from = "0000000000000003".into();
                panel.pending_merge = Some(PendingMerge {
                    kept: me.key_id.clone(),
                    from: "0000000000000003".into(),
                });
            }
            let inputs = Inputs {
                me: Some(&me),
                staff: &staff,
            };
            let font = &inter.font;
            panel.build(&inputs, font, [1920.0, 1080.0]);
            let order = panel.order.clone();
            for focus in order {
                for font in [&body.font, &inter.font] {
                    for viewport in [
                        [1920.0, 1080.0],
                        [3840.0, 2160.0],
                        [1440.0, 1080.0],
                        [2560.0, 1080.0],
                    ] {
                        panel.focus = focus;
                        panel.build(&inputs, font, viewport);
                        assert!(!panel.ui.overflowed(), "{focus} at {viewport:?}");
                    }
                }
            }
        }
    }
}
