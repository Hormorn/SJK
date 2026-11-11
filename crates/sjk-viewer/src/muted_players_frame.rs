//! The per-frame hook of muting (`muted_players.rs`): read the mute list once, carry
//! out a mute the chat's profile card asked for, and keep the muted slots of the
//! server being played in step with the list, the hub's claims and the server's
//! players. Each frame costs a few comparisons; the slots are worked out again only
//! when one of those changed. Their looks (blade skin, holocron) go at once
//! (`looks.rs`); the models, sabers and sounds follow in
//! [`GpuState::refresh_muted_models`].

use super::*;

impl GpuState {
    /// Follow the mute list into the chat and the muted slots.
    pub(crate) fn update_muted_players(&mut self) {
        if let Some(console) = &self.console {
            player_mutes::load(console.config_directory());
        }
        if let Some(request) = self.chat.take_mute_request() {
            player_mutes::set_muted(request.key_id.as_deref(), &request.name, request.muted);
        }
        if let Some(request) = self.chat.take_map_block_request() {
            let slot = usize::from(request.slot);
            let done = match (request.everyone, request.blocked) {
                (true, blocked) => self.staff_map_block(request.slot, &request.name, blocked),
                (false, true) => {
                    self.map_blocks.block(slot, &request.name);
                    Ok(())
                }
                (false, false) => {
                    self.map_blocks.unblock(slot);
                    Ok(())
                }
            };
            if let Err(error) = done {
                log::progress(format_args!("block: {error}"));
            }
        }
        let follow = self
            .console
            .as_ref()
            .is_none_or(|console| console.bool_cvar(map_blocks::CVAR) != Some(false));
        let Some(session) = self.live_session.as_ref() else {
            self.map_blocks.enter(None, follow);
            self.sound_mutes.players.enter(None, false);
            if self.muted_players.clear() {
                player_mutes::set_roster(std::iter::empty(), None);
            }
            self.chat.set_muted_slots(0);
            self.looks.set_muted(0);
            muted_players::set_shown(0);
            return;
        };
        // A new map load (`sv_serverid`) ends the blocks for the map, and the muted sounds.
        self.map_blocks.enter(Some(session.server_id()), follow);
        self.sound_mutes
            .players
            .enter(Some(session.server_id()), false);
        let own = session.game_state().client_num;
        let roster = self.chat.roster_targets();
        let chat = &self.chat;
        let map_blocks = &mut self.map_blocks;
        self.muted_players.refresh(
            player_mutes::revision(),
            player_identity::revision(),
            map_blocks.revision(),
            roster,
            own,
            || {
                let own = usize::try_from(own).ok();
                player_mutes::set_roster(
                    (0..muted_players::SLOTS as u16).map(|slot| chat.player_label(slot)),
                    own,
                );
                let shown: [&str; muted_players::SLOTS] =
                    std::array::from_fn(|slot| chat.player_label(slot as u16));
                let mut slots = player_mutes::slots();
                slots.muted |= map_blocks.resolve(&shown, own, &player_identity::map_blocks());
                slots
            },
        );
        let muted = self.muted_players.slots().muted;
        muted_players::set_shown(muted);
        // Players whose sounds alone are muted (`mutesounds`, `ignore`): worked out only
        // while there are some, as the players in their slots may leave.
        self.follow_ignores();
        if self.sound_mutes.players.pending() {
            let chat = &self.chat;
            let shown: [&str; muted_players::SLOTS] =
                std::array::from_fn(|slot| chat.player_label(slot as u16));
            let own = self
                .live_session
                .as_ref()
                .and_then(|session| usize::try_from(session.game_state().client_num).ok());
            self.sound_mutes.players.resolve(&shown, own, &[]);
        }
        self.chat.set_map_blocks(
            self.map_blocks.own_slots(),
            self.map_blocks.shared_slots(),
            player_identity::is_staff(),
        );
        self.chat.set_muted_slots(muted);
        // Their look too: the stock blade and its sounds, no holocron (`looks.rs`); at
        // Tactical Max every other player's (`tactical.rs`).
        self.looks.set_muted(self.muted_players.plain());
        // The toys hidden on other players here (Collection > Toys).
        let console = self.console.as_ref();
        let seen = |cvar| crate::toy_sight::seen(console, cvar);
        let mut unseen = 0;
        for (cvar, bit) in [
            (
                crate::toy_sight::ILLUMINATE,
                crate::looks::Unseen::ILLUMINATE,
            ),
            (
                crate::toy_sight::CARDBOARD_BOX,
                crate::looks::Unseen::CARDBOARD_BOX,
            ),
            (crate::toy_sight::BOOMBOX, crate::looks::Unseen::BOOMBOX),
        ] {
            if !seen(cvar) {
                unseen |= bit;
            }
        }
        self.looks.set_unseen(unseen);
    }

    /// Draw the muted players as the stand-in, or as themselves again, when the muted
    /// slots changed, and tell the sound which players to silence.
    pub(crate) fn refresh_muted_models(&mut self, audio: &mut Option<GameAudio>) {
        let muted = self.muted_players.slots().muted;
        if let Some(audio) = audio {
            audio.set_muted_clients(muted | self.sound_mutes.players.own_slots());
        }
        // Drawn plain: the muted, and at Tactical Max the others in its model.
        let plain = self.muted_players.plain();
        let mut changed = plain ^ self.muted_players.applied;
        let model =
            crate::tactical::plain_players().then(|| crate::tactical::model(self.console.as_ref()));
        if model.is_some() && model != self.muted_players.applied_model {
            changed |= plain & !muted;
        }
        if changed == 0 || self.live_session.is_none() {
            return;
        }
        self.muted_players.applied = plain;
        self.muted_players.applied_model = model;
        for client in 0..muted_players::SLOTS as u16 {
            if changed & (1 << client) != 0
                && let Err(error) = self.apply_clientinfo(client)
            {
                log::progress(format_args!("mute client {client}: {error}"));
            }
        }
    }
}
