//! The frame glue of the status balloons (`status_balloons.rs`): the own reason read
//! from what is open, the clan themes from the existing tag display, the relay's reasons
//! heard, the next one told, and this frame's sprites (`player_sprites.rs`).

use super::*;
use crate::status_balloons::{CVAR, Mode, Reason, SLOTS, TOY};

impl GpuState {
    /// Why the own talk balloon is up, if it is ([`Self::talk_button`]): the window away
    /// first, then the console, the chat, and anything else open is a menu.
    pub(crate) fn status_reason(&self) -> Option<Reason> {
        if !self.talk_button() {
            return None;
        }
        let console = self.console.as_ref();
        Some(if console.is_some_and(|console| console.window_talk()) {
            Reason::Away
        } else if console.is_some_and(|console| console.is_open()) && !self.console_keeps_moving() {
            Reason::Console
        } else if self.chat.is_typing() {
            Reason::Typing
        } else {
            Reason::Menu
        })
    }

    /// The frame's status balloons, from the relay's events.
    pub(crate) fn update_status_balloons(&mut self, relay_events: &[sjk_identity::ToyEvent]) {
        let reason = self.status_reason();
        // The relay is independent of the selected artwork mode; even a stock-mode
        // client contributes reasons that animated SJK clients can display.
        let mode = self
            .console
            .as_ref()
            .and_then(|console| console.integer_cvar(CVAR))
            .map(Mode::from_value)
            .unwrap_or(Mode::Sjk);
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            if self.status_balloons.server.is_some() {
                self.status_balloons.leave(None);
            }
            return;
        };
        let server = session.server();
        if self.status_balloons.server != Some(server) {
            self.status_balloons.leave(Some(server));
        }
        let game_state = session.game_state();
        // Clan mode follows the same visibility preference as the existing clan tags.
        let clan_emblems_shown = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(crate::jof_tag::CVAR))
            != Some(false);
        if mode.uses_clan_themes() {
            self.status_balloons
                .follow_clans(game_state, clan_emblems_shown);
        }
        let own = u8::try_from(game_state.client_num).ok();
        let muted = &self.muted_players;
        let heard = relay_events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && usize::from(event.slot) < SLOTS
                    && !muted.contains(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Reason::parse(&event.event)?)));
        let (mut seen, mut talking) = (0u64, 0u64);
        for entity in &session.latest_snapshot().entities {
            let number = usize::from(entity.number());
            if number < SLOTS {
                seen |= 1 << number;
                if entity.e_flags() & sjk_game_jka::pmove_talk::EF_TALK != 0 {
                    talking |= 1 << number;
                }
            }
        }
        let tell = self.status_balloons.frame(
            Instant::now(),
            reason,
            heard,
            seen,
            talking,
            self.looks.sjk_slots(),
            own,
            mode,
        );
        if let Some((slot, reason)) = tell {
            crate::player_identity::relay_toy_event(TOY, reason.word().to_owned(), slot, true);
        }
    }
}
