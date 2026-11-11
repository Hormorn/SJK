//! The per-frame hook of emotes (`emotes.rs`): the emotes the SJK hub relayed for
//! this server start on the players they name, when the game shows the name their
//! claim was made under in that slot; emotes end when they are over, their player
//! moves, or mounts a vehicle (hub report #247). Costs two uncontended locks a
//! frame when nothing comes.

use super::*;
use sjk_runtime::EntityId;

/// The players typing in `snapshot` ([`emotes::typing_slots`]): the own player by
/// `own_typing` (their open chat, console or menu, known here at once), else by their
/// talk balloon as the others.
fn typing(enabled: bool, snapshot: &sjk_protocol::Snapshot, own_typing: Option<bool>) -> u64 {
    let player = &snapshot.player;
    let own = u8::try_from(player.client_num()).ok().map(|slot| {
        let balloon = player.entity_flags() & sjk_game_jka::pmove_talk::EF_TALK != 0;
        (slot, own_typing.unwrap_or(balloon))
    });
    emotes::typing_slots(
        enabled,
        own,
        snapshot
            .entities
            .iter()
            .map(|entity| (entity.number(), entity.e_flags())),
    )
}

/// The players riding a vehicle in `snapshot` ([`emotes::riding_slots`]).
fn riding(snapshot: &sjk_protocol::Snapshot) -> u64 {
    let player = &snapshot.player;
    let own = u8::try_from(player.client_num())
        .ok()
        .map(|slot| (slot, player.vehicle_entity_num()));
    emotes::riding_slots(
        own,
        snapshot
            .entities
            .iter()
            .map(|entity| (entity.number(), entity.vehicle_entity_num())),
    )
}

impl GpuState {
    /// Start the emotes received, end those that are over.
    pub(crate) fn update_emotes(&mut self) {
        let received = player_identity::take_emotes();
        let keep_typing = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(emotes::TYPING_KEEP_CVAR))
            .unwrap_or(emotes::DEFAULT_TYPING_KEEP);
        let own_typing = self.talk_button();
        if let Some(vfs) = self.vfs.as_deref() {
            {
                let mut state = emotes::lock();
                if state.catalogue.is_none() {
                    state.catalogue = Some(emotes::load(vfs));
                }
            }
            crate::emote_icons::service(vfs, &self.ui_shapes, &self.queue);
        }
        crate::tag_icons::service(&self.ui_shapes, &self.queue);
        crate::quick_wheel::pictures::service(&self.ui_shapes, &self.queue);
        let Some(session) = self.live_session.as_ref() else {
            let mut state = emotes::lock();
            match self.demo_session.as_ref() {
                // A demo keeps the emotes started on it (a local one, a world shot's)
                // until they end or their player moves; none arrive from the hub.
                Some(demo) => {
                    let world = demo.world();
                    state
                        .active
                        .end_riding(riding(demo.latest_snapshot()), None);
                    state
                        .active
                        .hold(typing(keep_typing, demo.latest_snapshot(), None));
                    state.active.update(Instant::now(), |slot: u8| {
                        world
                            .entity(EntityId::new(u64::from(slot) + 1))
                            .map(|entity| entity.current().transform.translation)
                    });
                }
                None => state.active = emotes::ActiveEmotes::default(),
            }
            return;
        };
        let world = &self.live_world;
        let position = |slot: u8| {
            world
                .entity(EntityId::new(u64::from(slot) + 1))
                .map(|entity| entity.current().transform.translation)
        };
        let now = Instant::now();
        let own = u8::try_from(session.latest_snapshot().player.client_num()).ok();
        let riders = riding(session.latest_snapshot());
        let mut state = emotes::lock();
        for emote in received {
            // The hub's id is shown in the console as it came when no catalogue entry
            // names it: only an id that could name an emote may.
            if !emotes::valid_id(&emote.emote) {
                continue;
            }
            // The hub's echo of the player's own emote, already started by `sjkemote`.
            if own == Some(emote.slot)
                && state
                    .active
                    .get(emote.slot)
                    .is_some_and(|active| active.emote == emote.emote)
            {
                continue;
            }
            let shown = player_identity::shown_name(session.game_state(), usize::from(emote.slot));
            if !emotes::trusted(&emote, shown.as_deref()) {
                continue;
            }
            // Hidden here (`sjkemote mute`), or the player is blocked on this PC.
            if state.muted.contains_key(&emote.key_id)
                || self.muted_players.hides_fun(u16::from(emote.slot))
            {
                continue;
            }
            if emote.emote == emotes::STOP {
                state.active.stop(emote.slot);
                continue;
            }
            if emote.emote == emotes::SYNC {
                continue;
            }
            // On a vehicle, which plays its rider's animations (hub report #247).
            if riders
                .checked_shr(u32::from(emote.slot))
                .is_some_and(|bits| bits & 1 != 0)
            {
                continue;
            }
            // Its player went away, or came back (`away_emote.rs`).
            if emote.emote == emotes::QUIET || emote.emote == emotes::LOUD {
                let def = state.active.get(emote.slot).and_then(|active| {
                    state
                        .catalogue
                        .as_ref()?
                        .iter()
                        .find(|def| def.id == active.emote)
                        .cloned()
                });
                state
                    .active
                    .set_quiet(emote.slot, emote.emote == emotes::QUIET, def.as_ref(), now);
                continue;
            }
            let def = state
                .catalogue
                .as_ref()
                .and_then(|catalogue| catalogue.iter().find(|def| def.id == emote.emote))
                .cloned();
            // None while the player is not in the snapshot: the first position seen
            // becomes the origin, rather than the map's centre (hub report #107).
            let origin = position(emote.slot);
            // In step with the leader as this client saw them start; a leader not
            // playing it here (gone, or its own echo still on the way) starts it anew.
            let synced = emote.sync.is_some_and(|leader| {
                state
                    .active
                    .sync(emote.slot, leader, &emote.emote, def.as_ref(), origin, now)
            });
            if !synced {
                state
                    .active
                    .start(emote.slot, &emote.emote, def.as_ref(), origin, now);
            }
            if emote.quiet {
                state.active.set_quiet(emote.slot, true, def.as_ref(), now);
            }
            // Until the animations exist, the console says it.
            if let Some(console) = &mut self.console {
                let name = def
                    .as_ref()
                    .map_or(emote.emote.as_str(), |def| def.name.as_str());
                console.push_log(format!("* {}^7: {name}", shown.unwrap_or_default()));
            }
        }
        // A player blocked while an emote of theirs plays: it ends here; at Tactical
        // Minimal and up every other player's does (`tactical.rs`).
        let mut blocked = self.muted_players.slots().muted;
        if crate::tactical::hides_fun() {
            blocked |= own
                .filter(|own| *own < 32)
                .map_or(u32::MAX, |own| !(1u32 << own));
        }
        while blocked != 0 {
            let slot = blocked.trailing_zeros() as u8;
            blocked &= blocked - 1;
            state.active.stop(slot);
        }
        // A player who mounts a vehicle ends their emote; the own player tells the
        // others (after the lock), for the clients that do not see it end by themselves.
        let own_ended = state.active.end_riding(riders, own);
        // A player typing keeps their emote until they move after (`cg_emoteTypingKeep`).
        state.active.hold(typing(
            keep_typing,
            session.latest_snapshot(),
            Some(own_typing),
        ));
        state.active.update(now, position);
        drop(state);
        if own_ended {
            let _ = player_identity::emote(emotes::STOP.to_owned(), None);
        }
    }

    /// Play and stop the emotes' sounds (`emotes::Cue`) on their players; the
    /// catalogue's sounds are loaded the first time it is seen.
    pub(crate) fn play_emote_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let mut state = emotes::lock();
        let cues = state.active.take_cues();
        let Some(audio) = audio.as_mut() else {
            return;
        };
        if state.sounds_loaded != Some(audio.generation())
            && let (Some(catalogue), Some(vfs)) = (state.catalogue.as_ref(), self.vfs.as_deref())
        {
            audio.register_effect_paths(
                vfs,
                catalogue
                    .iter()
                    .flat_map(|def| [def.sound.as_str(), def.sound_loop.as_str()])
                    .filter(|sound| !sound.is_empty()),
            );
            state.sounds_loaded = Some(audio.generation());
        }
        drop(state);
        let world = self
            .demo_session
            .as_ref()
            .map_or(&self.live_world, crate::demo_playback::Session::world);
        let own = match (&self.live_session, &self.demo_session) {
            (Some(session), _) => Some(session.latest_snapshot().player.client_num()),
            (None, Some(demo)) => Some(demo.latest_snapshot().player.client_num()),
            (None, None) => None,
        }
        .and_then(|own| u8::try_from(own).ok());
        for cue in cues {
            match cue {
                emotes::Cue::Start {
                    slot,
                    sound,
                    offset,
                    align,
                } => {
                    let origin = (Some(slot) != own).then(|| {
                        world
                            .entity(EntityId::new(u64::from(slot) + 1))
                            .map(|entity| entity.current().transform.translation)
                            .unwrap_or_default()
                    });
                    audio.play_emote(&sound, slot, origin, offset, align);
                }
                emotes::Cue::Stop { slot } => audio.stop_emote(slot),
            }
        }
    }
}
