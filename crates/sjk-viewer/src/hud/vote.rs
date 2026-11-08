//! Vote panel backed by BaseJKA vote configstrings: a centred card with the
//! proposal, the time left and the YES / NO counts beside their keys.

use super::{HudOverlay, widgets::EmitContext};
use crate::console::ViewerConsole;
use crate::menu::sjk::color;
use crate::menu_widgets::VoteLayout;
use sjk_protocol::GameState;
use sjk_ui::{
    Color, DrawCommand, DrawList, FontWeight, HudWidget, Rect, TextAlign, TextId, TextOverflow,
    Theme,
};
use std::fmt::Write as _;

/// A BaseJKA vote runs 30 seconds (`VOTE_TIME`, `g_main.c`).
const VOTE_MILLIS: f32 = 30_000.0;
/// First text id of the global vote's card; the team vote's follows it.
const CARD_TEXT_FIRST: u32 = 200;
/// Text ids per card: label, proposal, seconds, YES, NO.
const CARD_TEXTS: u32 = 5;
/// The keys bound to `vote yes` and `vote no`.
pub(super) const YES_KEYS_TEXT: u32 = 210;
pub(super) const NO_KEYS_TEXT: u32 = 211;
/// Last vote text id.
pub(super) const TEXT_LAST: u32 = NO_KEYS_TEXT;

// SJK's own palette ([`crate::menu::sjk::color`]), as its menus' cards draw it.
const YES: Color = color::TRUSTED;
const NO: Color = color::RED_TEAM;
const PLATE: Color = Color::new(0.04, 0.06, 0.12, 0.9);
const SHADOW: Color = color::alpha(color::SPACE, 0.5);
const EDGE: Color = color::alpha(color::HOLO, 0.45);
const TRACK: Color = color::alpha(color::HOLO, 0.16);
/// Room above the card, so it stands under the match timer at the top centre.
const BELOW_TIMER: f32 = 44.0;

/// One vote's texts and numbers for its card.
#[derive(Debug, Default)]
pub(super) struct Card {
    label: String,
    text: String,
    seconds: String,
    yes: String,
    no: String,
    /// The vote shows.
    pub(super) active: bool,
    /// Time left as a fraction of the vote's 30 seconds.
    pub(super) time: f32,
    /// Votes cast for and against.
    pub(super) yes_count: i32,
    pub(super) no_count: i32,
}

impl Card {
    fn set(&mut self, vote: Option<sjk_client::VoteState<'_>>, label: &str) {
        self.label.clear();
        self.text.clear();
        self.seconds.clear();
        self.yes.clear();
        self.no.clear();
        let Some(vote) = vote.filter(|vote| vote.active) else {
            self.active = false;
            self.time = 0.0;
            self.yes_count = 0;
            self.no_count = 0;
            return;
        };
        self.active = true;
        self.label.push_str(label);
        self.text.push_str(vote.text);
        let _ = write!(self.seconds, "{}s", vote.remaining_seconds().max(0));
        let _ = write!(self.yes, "YES  {}", vote.yes);
        let _ = write!(self.no, "{}  NO", vote.no);
        self.time = (vote.remaining_ms as f32 / VOTE_MILLIS).clamp(0.0, 1.0);
        self.yes_count = vote.yes.max(0);
        self.no_count = vote.no.max(0);
    }

    /// Card text `index` (0 label, 1 proposal, 2 seconds, 3 YES, 4 NO).
    fn text(&self, index: u32) -> &str {
        match index {
            0 => &self.label,
            1 => &self.text,
            2 => &self.seconds,
            3 => &self.yes,
            _ => &self.no,
        }
    }
}

/// The `testvote` console command: a made-up vote on the HUD, to see the panel
/// where no vote can run (alone on a devmap a vote passes at once).
pub(crate) const PREVIEW_COMMAND: &str = "testvote";
/// Its help line.
pub(crate) const PREVIEW_HELP: &str =
    "Show a made-up vote on the HUD for 30 seconds: testvote [team], testvote off";

/// A `testvote` preview's state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum Preview {
    #[default]
    Off,
    /// Asked for; starts at the next HUD update, with its server time.
    Asked { team: bool },
    /// Running since this server time.
    Since { start: i32, team: bool },
}

/// The preview's vote `elapsed` ms in: counts that grow, and the time left.
fn preview_vote(
    elapsed: i32,
    scope: sjk_client::LegacyVoteScope,
) -> sjk_client::VoteState<'static> {
    let team = scope != sjk_client::LegacyVoteScope::Global;
    sjk_client::VoteState {
        active: true,
        scope,
        text: if team {
            "Leader: Rain"
        } else {
            "poll all vs all?"
        },
        yes: 1 + elapsed / 2_500,
        no: elapsed / 6_000,
        remaining_ms: VOTE_MILLIS as i32 - elapsed,
    }
}

/// What the vote widget draws: the global and the team vote.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Meters {
    pub(super) cards: [Meter; 2],
}

/// One card's numbers.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Meter {
    pub(super) active: bool,
    pub(super) time: f32,
    pub(super) yes: i32,
    pub(super) no: i32,
}

impl HudOverlay {
    pub(super) fn update_votes(
        &mut self,
        game_state: &GameState,
        team: u8,
        server_time: i32,
        console: Option<&ViewerConsole>,
    ) {
        let mut global = Some(sjk_client::legacy_global_vote(game_state, server_time));
        let mut team_vote = sjk_client::legacy_team_vote(game_state, team, server_time);
        if let Preview::Asked { team } = self.vote_preview {
            self.vote_preview = Preview::Since {
                start: server_time,
                team,
            };
        }
        if let Preview::Since { start, team } = self.vote_preview {
            let elapsed = server_time.saturating_sub(start);
            if !(0..VOTE_MILLIS as i32).contains(&elapsed) {
                self.vote_preview = Preview::Off;
            } else {
                // A real vote keeps its card.
                if !global.is_some_and(|vote| vote.active) {
                    global = Some(preview_vote(elapsed, sjk_client::LegacyVoteScope::Global));
                }
                if team && !team_vote.is_some_and(|vote| vote.active) {
                    team_vote = Some(preview_vote(elapsed, sjk_client::LegacyVoteScope::RedTeam));
                }
            }
        }
        self.vote_cards[0].set(global, "VOTE");
        self.vote_cards[1].set(team_vote, "TEAM VOTE");
        self.vote_active = self.vote_cards[0].active;
        self.team_vote_active = self.vote_cards[1].active;
        if let Some(console) = console {
            console.write_keys_for_command("vote yes", &mut self.yes_keys);
            console.write_keys_for_command("vote no", &mut self.no_keys);
            // Without a key the console command is the way to vote.
            if self.yes_keys == "unbound" {
                self.yes_keys.clear();
                self.yes_keys.push_str("/vote yes");
            }
            if self.no_keys == "unbound" {
                self.no_keys.clear();
                self.no_keys.push_str("/vote no");
            }
        } else {
            self.yes_keys.clear();
            self.yes_keys.push_str("F1");
            self.no_keys.clear();
            self.no_keys.push_str("F2");
        }
    }

    /// `testvote [team|off]`: start (or stop) the made-up vote; the reply line.
    pub(crate) fn preview_vote_command(&mut self, args: &[String]) -> Result<String, String> {
        match args {
            [] => self.vote_preview = Preview::Asked { team: false },
            [word] if word.eq_ignore_ascii_case("team") => {
                self.vote_preview = Preview::Asked { team: true };
            }
            [word] if word.eq_ignore_ascii_case("off") => {
                self.vote_preview = Preview::Off;
                return Ok("Test vote off".into());
            }
            _ => return Err(format!("usage: {PREVIEW_COMMAND} [team|off]")),
        }
        Ok("Test vote on the HUD for 30 seconds (only you see it)".into())
    }

    /// The numbers the vote widget draws.
    pub(super) fn vote_meters(&self) -> Meters {
        Meters {
            cards: self.vote_cards.each_ref().map(|card| Meter {
                active: card.active,
                time: card.time,
                yes: card.yes_count,
                no: card.no_count,
            }),
        }
    }

    /// Vote text `id`, from [`CARD_TEXT_FIRST`] to [`TEXT_LAST`].
    pub(super) fn vote_text(&self, id: u32) -> &str {
        match id {
            YES_KEYS_TEXT => &self.yes_keys,
            NO_KEYS_TEXT => &self.no_keys,
            _ => {
                let offset = id.saturating_sub(CARD_TEXT_FIRST);
                let card = (offset / CARD_TEXTS) as usize;
                self.vote_cards
                    .get(card)
                    .map_or("", |card| card.text(offset % CARD_TEXTS))
            }
        }
    }
}

pub(super) fn emit(
    draw: &mut DrawList,
    _theme: Theme,
    _widget: &HudWidget,
    rect: Rect,
    context: &EmitContext<'_>,
) {
    let meters = context.data.votes;
    if !meters.cards.iter().any(|card| card.active) {
        return;
    }
    let scale = context.hero_scale;
    let ratio = scale / context.dpi_scale;
    // The card keeps its design width and is centred on the widget's anchor, so a
    // wide widget no longer leaves it at the left.
    let width = VoteLayout::WIDTH * scale;
    let center = rect.x + rect.width * 0.5;
    let mut y = rect.y * ratio + BELOW_TIMER * scale;
    for (index, meter) in meters.cards.iter().enumerate() {
        if !meter.active {
            continue;
        }
        let layout = VoteLayout::new(Rect::new(center - width * 0.5, y, width, 0.0), scale);
        emit_card(draw, &layout, *meter, index as u32, scale);
        y += layout.plate.height + 10.0 * scale;
    }
}

fn emit_card(draw: &mut DrawList, layout: &VoteLayout, meter: Meter, card: u32, scale: f32) {
    let text = |index: u32| TextId(CARD_TEXT_FIRST + card * CARD_TEXTS + index);
    let radius = 18.0 * scale;
    let plate = layout.plate;
    // The SJK menu card: a soft shadow, the navy plate and a holo edge.
    let _ = draw.push(DrawCommand::RoundedRect {
        rect: Rect::new(
            plate.x + 4.0 * scale,
            plate.y + 8.0 * scale,
            plate.width,
            plate.height,
        ),
        radius,
        color: SHADOW,
    });
    let _ = draw.push(DrawCommand::RoundedRect {
        rect: plate,
        radius,
        color: PLATE,
    });
    let _ = draw.push(DrawCommand::Border {
        rect: plate,
        radius,
        width: 1.5 * scale,
        color: EDGE,
    });
    // The gold mark beside the heading, as SJK's menu bands have.
    let _ = draw.push(DrawCommand::RoundedRect {
        rect: Rect::new(
            layout.label.x - 10.0 * scale,
            layout.label.y + 5.0 * scale,
            4.0 * scale,
            layout.label.height - 10.0 * scale,
        ),
        radius: 2.0 * scale,
        color: color::GOLD_BRIGHT,
    });
    emit_text(
        draw,
        layout.label,
        text(0),
        17.0 * scale,
        color::GOLD_BRIGHT,
        TextAlign::Start,
        FontWeight::Semibold,
        2.0,
    );
    emit_text(
        draw,
        layout.label,
        text(2),
        22.0 * scale,
        color::TEXT,
        TextAlign::End,
        FontWeight::Semibold,
        0.5,
    );
    // Time left, shrinking from the full width.
    bar(draw, layout.time, 1.0, TRACK, scale);
    bar(draw, layout.time, meter.time, color::GOLD, scale);
    emit_text(
        draw,
        layout.proposal,
        text(1),
        30.0 * scale,
        color::TEXT,
        TextAlign::Center,
        FontWeight::Semibold,
        0.3,
    );
    // YES fills from the left, NO from the right, in proportion to the votes cast.
    bar(draw, layout.split, 1.0, TRACK, scale);
    let cast = meter.yes + meter.no;
    if cast > 0 {
        let yes = meter.yes as f32 / cast as f32;
        bar(draw, layout.split, yes, YES, scale);
        let no = layout.split;
        let no = Rect::new(
            no.x + no.width * yes,
            no.y,
            no.width * (1.0 - yes),
            no.height,
        );
        bar(draw, no, 1.0, NO, scale);
    }
    emit_text(
        draw,
        layout.counts,
        text(3),
        24.0 * scale,
        YES,
        TextAlign::Start,
        FontWeight::Semibold,
        1.0,
    );
    emit_text(
        draw,
        layout.counts,
        text(4),
        24.0 * scale,
        NO,
        TextAlign::End,
        FontWeight::Semibold,
        1.0,
    );
    emit_text(
        draw,
        layout.keys,
        TextId(YES_KEYS_TEXT),
        15.0 * scale,
        color::MUTED,
        TextAlign::Start,
        FontWeight::Regular,
        0.5,
    );
    emit_text(
        draw,
        layout.keys,
        TextId(NO_KEYS_TEXT),
        15.0 * scale,
        color::MUTED,
        TextAlign::End,
        FontWeight::Regular,
        0.5,
    );
}

/// The leading `fraction` of `rect`, rounded.
fn bar(draw: &mut DrawList, rect: Rect, fraction: f32, color: Color, scale: f32) {
    let width = rect.width * fraction.clamp(0.0, 1.0);
    if width <= 0.0 {
        return;
    }
    let _ = draw.push(DrawCommand::RoundedRect {
        rect: Rect::new(rect.x, rect.y, width, rect.height),
        radius: (rect.height * 0.5).min(width * 0.5).min(4.0 * scale),
        color,
    });
}

#[allow(clippy::too_many_arguments)]
fn emit_text(
    draw: &mut DrawList,
    rect: Rect,
    text: TextId,
    size: f32,
    color: Color,
    align: TextAlign,
    weight: FontWeight,
    letter_spacing: f32,
) {
    let _ = draw.push(DrawCommand::Text {
        rect,
        text,
        size,
        color,
        align,
        overflow: TextOverflow::Ellipsis,
        weight,
        letter_spacing,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_texts_follow_the_vote() {
        let mut card = Card::default();
        card.set(
            Some(sjk_client::VoteState {
                active: true,
                scope: sjk_client::LegacyVoteScope::Global,
                text: "poll all vs all?",
                yes: 14,
                no: 4,
                remaining_ms: 12_400,
            }),
            "VOTE",
        );
        assert!(card.active);
        assert_eq!(card.text(0), "VOTE");
        assert_eq!(card.text(1), "poll all vs all?");
        assert_eq!(card.text(2), "12s");
        assert_eq!(card.text(3), "YES  14");
        assert_eq!(card.text(4), "4  NO");
        assert!((card.time - 12.4 / 30.0).abs() < 1e-4);
        card.set(None, "VOTE");
        assert!(!card.active);
        assert_eq!(card.text(1), "");
        assert_eq!((card.yes_count, card.no_count), (0, 0));
    }
}
