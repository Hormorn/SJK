//! The frame glue of the Banana peel (`banana.rs`): the switch said on the screen, the
//! relay's events heard, the player's own drop placed and told, the slip decided against
//! the player's own feet, and the peels drawn and heard.

use super::*;
use crate::actor_instance::ActorInstance;
use crate::banana::{
    Cue, DROP_SOUND, Event, FEET, MODEL, Peel, SEND_GAP, SHOW_CVAR, SLIP_SOUND, TOY, Walker,
};
use glam::Vec3;
use sjk_runtime::EntityId;

/// `CONTENTS_SOLID`: what a peel lies on (not clips, which nobody sees).
const PEEL_MASK: u32 = 1;
/// The box a peel is traced as: about its width, its bottom at the point traced.
const PEEL_HALF: f32 = 2.0;
/// The numbers of a player's peels go round this many.
const NUMBERS: u8 = 100;
/// `ENTITYNUM_NONE` as the player state's ground entity: in the air.
const NO_GROUND: u16 = 1023;
/// The toy's sounds, on a source of their own.
const SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(0x5200_0000);
const SLOTS: usize = crate::looks::SLOTS;

impl GpuState {
    /// `toy_banana`: on or off, said on the centre of the screen.
    pub(crate) fn toggle_banana(&mut self) {
        self.banana.toggle();
        self.banana_centre(&crate::toy_switch_words::switched(
            "Banana peel",
            self.banana.on(),
        ));
    }

    /// `banana_drop`: drop a peel at the next frame.
    pub(crate) fn drop_banana(&mut self) {
        self.banana.drop_peel();
    }

    /// Say `text` on the centre of the screen.
    fn banana_centre(&mut self, text: &str) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text.to_owned(),
            None,
            Instant::now(),
        );
    }

    /// What a small box (a peel's width) meets between two points of the map's
    /// brushes, if anything.
    pub(crate) fn banana_trace(&mut self, from: Vec3, to: Vec3) -> Option<crate::banana::Ground> {
        let bounds = sjk_bsp::Aabb::new([-PEEL_HALF, -PEEL_HALF, 0.0], [PEEL_HALF, PEEL_HALF, 1.0])
            .expect("the peel's bounds are valid");
        let trace = self.bsp.trace_box_with(
            &mut self.trace_scratch,
            from.to_array(),
            to.to_array(),
            bounds,
            PEEL_MASK,
        );
        if trace.start_solid || trace.fraction >= 1.0 {
            return None;
        }
        Some(crate::banana::Ground {
            stop: Vec3::from_array(trace.end_position),
            normal: trace
                .plane
                .as_ref()
                .map_or(Vec3::Z, |plane| Vec3::from_array(plane.normal)),
            sky: trace.surface_flags & weather::cover::SURF_SKY != 0,
        })
    }

    /// Where a peel relayed to land at `at` rests here, and the floor's normal there:
    /// the floor found just under it (whole units carry it half a unit off), else as told.
    pub(crate) fn banana_rest(&mut self, at: [i32; 3]) -> (Vec3, Vec3) {
        let at = Vec3::from_array(at.map(|c| c as f32));
        self.banana_trace(at + Vec3::Z * 8.0, at - Vec3::Z * 16.0)
            .filter(|floor| !floor.sky)
            .map_or((at, Vec3::Z), |floor| {
                (floor.stop, floor.normal.normalize_or(Vec3::Z))
            })
    }

    /// The map's spawn points, read once a map ([`crate::banana::spawn_points`]).
    fn banana_spawns(&mut self) -> &[Vec3] {
        let map = Arc::as_ptr(&self.bsp) as usize;
        if self.banana.spawns_for != map {
            self.banana.spawns_for = map;
            self.banana.spawns = sjk_entity::parse_entity_lump(self.bsp.entities())
                .map(|entities| crate::banana::spawn_points(&entities))
                .unwrap_or_default();
        }
        &self.banana.spawns
    }

    /// The frame's toy logic, from the relay's events: peels heard of, used up or taken
    /// away, the player's own drop, and those to tell. After the Ragdoll's, whose rules
    /// say who may drop.
    pub(crate) fn update_banana(&mut self, relay_events: &[sjk_identity::ToyEvent]) {
        let turned_off = self.banana.was_on && !self.banana.on;
        self.banana.was_on = self.banana.on;
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            if self.banana.server.is_some() {
                self.banana.leave();
                self.banana.server = None;
                self.banana.server_id = None;
            }
            self.banana.drop_asked = false;
            return;
        };
        let server = session.server();
        let server_id = session.server_id();
        if self.banana.server != Some(server) || self.banana.server_id != Some(server_id) {
            // Another server, or a new map on it: every peel goes.
            self.banana.leave();
            self.banana.server = Some(server);
            self.banana.server_id = Some(server_id);
        }
        let game_state = session.game_state();
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        self.banana.duel = crate::order66::duelling(&session.latest_snapshot().player, gametype);
        self.banana.unseen = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(SHOW_CVAR))
            == Some(false);
        let own = u8::try_from(game_state.client_num).ok();
        let now = self.banana.time;
        let heard: Vec<(u8, Event)> = relay_events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && usize::from(event.slot) < SLOTS
                    && !self.muted_players.hides_fun(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        for (slot, event) in heard {
            self.banana_hear(slot, event, now);
        }
        let sjk = self.looks.sjk_slots();
        if let Some(own) = own {
            if turned_off {
                // The own peels are picked up, for everyone told of them.
                let told = self
                    .banana
                    .peels
                    .list
                    .iter()
                    .filter(|peel| peel.dropper == own)
                    .fold(0_u64, |told, peel| told | peel.told);
                self.banana.peels.clear_of(own);
                if told != 0 {
                    self.banana.outbox.push((Event::Clear, told));
                }
            }
            // One who left is told again should they come back.
            for peel in &mut self.banana.peels.list {
                peel.told &= sjk;
            }
        }
        self.banana.peels.prune(now, sjk);
        if std::mem::take(&mut self.banana.drop_asked) {
            self.banana_drop_now(own);
        }
        self.banana_tell(own, sjk);
    }

    /// One relayed event of the player in `slot`, heard at game server time `now`.
    fn banana_hear(&mut self, slot: u8, event: Event, now: i64) {
        match event {
            Event::Drop(drop) => {
                let known = self
                    .banana
                    .peels
                    .find(slot, drop.number)
                    .is_some_and(|index| self.banana.peels.list[index].drop == drop);
                if known
                    || self.banana.used_up(slot, drop.number)
                    || !crate::banana::Banana::in_time(drop.moment, now)
                {
                    return;
                }
                let (rest, normal) = self.banana_rest(drop.at);
                if crate::banana::near_spawn(rest, self.banana_spawns()) {
                    return;
                }
                self.banana.peels.put(Peel::new(slot, drop, rest, normal));
            }
            Event::Slip { dropper, number } => {
                self.banana.note_gone(dropper, number);
                if let Some(index) = self.banana.peels.find(dropper, number) {
                    let peel = self.banana.peels.list.remove(index);
                    self.banana.cues.push(Cue {
                        path: SLIP_SOUND,
                        at: Some(peel.rest),
                        volume: 1.0,
                    });
                }
            }
            Event::Clear => self.banana.peels.clear_of(slot),
        }
    }

    /// Why the player may not drop a peel now, if they may not.
    fn banana_refusal(&self) -> Option<&'static str> {
        if !self.banana.on {
            Some("Turn the Banana peel toy on to drop one")
        } else if !crate::banana::may_play(self.banana.on, &self.ragdoll) {
            Some("No banana peel right now")
        } else {
            None
        }
    }

    /// The player drops a peel now, if they may.
    fn banana_drop_now(&mut self, own: Option<u8>) {
        let Some(own) = own else {
            return;
        };
        if let Some(why) = self.banana_refusal() {
            self.banana_centre(why);
            return;
        }
        if self
            .banana
            .last_drop
            .is_some_and(|last| last.elapsed() < crate::banana::COOLDOWN)
        {
            return;
        }
        let Some(state) = self.local_prediction.predicted_state() else {
            return;
        };
        let origin = Vec3::from_array(state.origin);
        let velocity = Vec3::from_array(state.velocity);
        let yaw = state.view_angles[1];
        let placed = crate::banana::placement(origin, velocity, yaw, |from, to| {
            self.banana_trace(from, to)
        });
        let Some((hand, floor)) = placed else {
            self.banana_centre("Nowhere to drop a peel here");
            return;
        };
        if crate::banana::near_spawn(floor.stop, self.banana_spawns()) {
            self.banana_centre("Not so close to a spawn point");
            return;
        }
        let number = self.banana.next_number;
        self.banana.next_number = (number + 1) % NUMBERS;
        self.banana.last_drop = Some(Instant::now());
        let drop = crate::banana::drop_of(number, self.banana.time as i32, hand, floor.stop, yaw);
        let normal = floor.normal.normalize_or(Vec3::Z);
        self.banana
            .peels
            .put(Peel::new(own, drop, floor.stop, normal));
    }

    /// Tell the next SJK player what they are to hear: a slip or a clear first, then the
    /// player's newest peel not yet told of, the nearest first.
    fn banana_tell(&mut self, own: Option<u8>, sjk: u64) {
        let Some(own) = own else {
            return;
        };
        let now = Instant::now();
        if self.banana.tell_at.is_some_and(|at| now < at) {
            return;
        }
        let others = sjk & !(1 << own);
        // A slip or a clear, to each slot still to tell that is still here.
        while let Some((event, slots)) = self.banana.outbox.first_mut() {
            *slots &= others;
            if *slots == 0 {
                self.banana.outbox.remove(0);
                continue;
            }
            let slot = slots.trailing_zeros() as u8;
            *slots &= !(1 << slot);
            let event = *event;
            self.banana.tell_at = Some(now + SEND_GAP);
            crate::player_identity::relay_toy_event(TOY, event.text(), slot, false);
            return;
        }
        let time = self.banana.time;
        let world = &self.live_world;
        let Some(peel) = self
            .banana
            .peels
            .list
            .iter_mut()
            .rev()
            .find(|peel| peel.dropper == own && others & !peel.told != 0)
        else {
            return;
        };
        let Some(slot) = (0..SLOTS as u8)
            .filter(|slot| (others & !peel.told) & (1 << slot) != 0)
            .min_by(|a, b| {
                let distance = |slot: &u8| {
                    world
                        .entity(EntityId::new(u64::from(*slot) + 1))
                        .map_or(f32::MAX, |entity| {
                            Vec3::from_array(entity.sample(time).translation).distance(peel.rest)
                        })
                };
                distance(a).total_cmp(&distance(b))
            })
        else {
            return;
        };
        peel.told |= 1 << slot;
        self.banana.tell_at = Some(now + SEND_GAP);
        crate::player_identity::relay_toy_event(TOY, Event::Drop(peel.drop).text(), slot, false);
    }

    /// Step the peels on to `time` (game server milliseconds, the presentation's): their
    /// landings heard, those the player has stepped off armed, and the player slipping on
    /// one they walk onto. Before the Ragdoll's bodies are stepped, so the fall shows
    /// this frame.
    pub(crate) fn step_bananas(&mut self, time: i64) {
        self.banana.time = time;
        self.banana.landings(time);
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            return;
        };
        let Some(own) = u8::try_from(session.game_state().client_num).ok() else {
            return;
        };
        let Some(state) = self.local_prediction.predicted_state() else {
            return;
        };
        let walker = Walker {
            feet: Vec3::from_array(state.origin) - Vec3::Z * FEET,
            velocity: Vec3::from_array(state.velocity),
            on_ground: state.ground_entity_number != NO_GROUND,
        };
        crate::banana::arm(&mut self.banana.peels, walker.feet, time);
        if !crate::banana::may_play(self.banana.on, &self.ragdoll) {
            return;
        }
        let Some(index) = crate::banana::slips_on(&walker, own, &self.banana.peels, time) else {
            return;
        };
        let (dropper, number) = {
            let peel = &self.banana.peels.list[index];
            (peel.dropper, peel.drop.number)
        };
        // The fall, its feet swept from under it: told by the Ragdoll, naming the dropper.
        let foot = if number % 2 == 0 {
            crate::ragdoll::L_FOOT
        } else {
            crate::ragdoll::R_FOOT
        } as u8;
        let by = crate::ragdoll::By {
            thrower: dropper,
            number: crate::ragdoll::UNTHROWN,
            step: 0,
        };
        if !self.ragdoll_struck(time, foot, crate::banana::slip_push(walker.velocity), by) {
            return;
        }
        self.banana.peels.list.remove(index);
        self.banana.note_gone(dropper, number);
        let others = self.looks.sjk_slots() & !(1 << own);
        if others != 0 {
            self.banana
                .outbox
                .push((Event::Slip { dropper, number }, others));
        }
        self.banana.cues.push(Cue {
            path: SLIP_SOUND,
            at: None,
            volume: 1.0,
        });
    }

    /// Add this frame's peels.
    pub(crate) fn submit_bananas(&mut self, presentation_time: i64) {
        #[cfg(test)]
        let presentation_time = self.banana.shot_time.unwrap_or(presentation_time);
        if self.banana.duel || self.banana.unseen || self.banana.peels.list.is_empty() {
            return;
        }
        let Some(mesh) = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == MODEL)
        else {
            return;
        };
        for peel in &self.banana.peels.list {
            let level = peel.level(presentation_time).clamp(0.0, 1.0);
            let (at, turn) = peel.pose(presentation_time);
            self.object_groups[mesh].push(ActorInstance::new(
                at.to_array(),
                turn.to_array(),
                [level; 3],
            ));
        }
    }

    /// Play the toy's sounds of the frame.
    pub(crate) fn play_banana_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.banana.cues.clear();
            return;
        };
        if self.banana.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [SLIP_SOUND, DROP_SOUND]);
            self.banana.sounds_registered = Some(audio.generation());
        }
        for Cue { path, at, volume } in self.banana.cues.drain(..) {
            // In a duel nothing is heard; unseen, no peel in the world is.
            if self.banana.duel || (self.banana.unseen && at.is_some()) {
                continue;
            }
            audio.play_toy(
                crate::toy_sounds::Toy::Banana,
                path,
                volume,
                at.map(|at| at.to_array()),
                SOURCE,
                sjk_audio::ChannelId(0),
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: peel `number` of the player in `slot` resting on the floor
    /// at `rest` (found under it), turned `yaw` degrees, dropped `age` milliseconds before
    /// the shots' time ([`Self::shot_banana_time`]; less than the flight: still on its way
    /// from a hand 30 units back and 40 up).
    pub(crate) fn shot_banana(&mut self, slot: u8, number: u8, rest: Vec3, yaw: f32, age: i64) {
        let (rest, normal) = self.banana_rest(rest.to_array().map(|c| c.round() as i32));
        let hand = rest + Vec3::new(-30.0, 0.0, 40.0);
        let moment = (self.banana.shot_time.unwrap_or(0) - age) as i32;
        let drop = crate::banana::drop_of(number, moment, hand, rest, yaw);
        self.banana.peels.put(Peel::new(slot, drop, rest, normal));
    }

    /// For the world shots: the peels are drawn at `time` (game server ms) whatever the
    /// clock says.
    pub(crate) fn shot_banana_time(&mut self, time: i64) {
        self.banana.shot_time = Some(time);
    }
}
