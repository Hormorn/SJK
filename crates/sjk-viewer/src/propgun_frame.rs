//! The frame glue of the Prop gun (`propgun.rs`): the switch and the gun chosen, the
//! relay's events heard and told, the player's shots, every bolt flown against the
//! world and the player's own body, and the guns and bolts drawn and heard.

use super::*;
use crate::actor_instance::ActorInstance;
use crate::propgun::{
    Bolt, Cue, End, Event, GUN_CVAR, Gun, HOLD_EXPIRES, IMMUNE_MS, LATE_MS, NUMBERS, RETELL,
    SEND_GAP, SHOW_CVAR, Shot, Splash, TOY, Target,
};
use glam::{Quat, Vec3};

/// `CONTENTS_SOLID`: what stops a bolt.
const BOLT_MASK: u32 = 1;
/// The toy's sounds, on a source of their own.
const SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(0x5400_0000);
const SLOTS: usize = crate::looks::SLOTS;
/// How far ahead the player's aim is traced, to aim the bolt at what the crosshair meets.
const AIM_REACH: f32 = 8_192.0;
/// Milliseconds between two of a bolt's trail effects.
const TRAIL_GAP: i64 = 25;
/// Flying bolts heard at once.
const MAX_LOOPS: usize = 8;

/// A gun's world model has the view model's axes (X forward, Y left, Z up); the hand's
/// bolt (`*r_hand`) holds the game's own world guns barrel along -Y, Z up
/// (`models/weapons2/*_w.glm`), so this turns the one into the other.
pub(crate) const HAND_TURN: Quat = Quat::from_xyzw(
    0.0,
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
    std::f32::consts::FRAC_1_SQRT_2,
);

/// How a held gun is drawn on one player in third person: its mesh and its placing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HeldDraw {
    pub(crate) mesh: usize,
    pub(crate) offset: Vec3,
    pub(crate) rotation: Quat,
    pub(crate) scale: f32,
    /// The muzzle on the model's own axes (first person only).
    pub(crate) muzzle: Option<Vec3>,
}

/// The models of every gun in the game's files, for the map-load appearance set.
pub(crate) fn extend_appearances(
    vfs: &sjk_vfs::VirtualFileSystem,
    appearances: &mut BTreeSet<Appearance>,
) {
    let (guns, _) = crate::propgun::catalogue(vfs);
    for gun in &guns {
        for model in gun.models() {
            appearances.insert(Appearance {
                model: model.to_owned(),
                variant: String::new(),
            });
        }
    }
}

impl GpuState {
    /// Say `text` on the centre of the screen.
    fn propgun_centre(&mut self, text: &str) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text.to_owned(),
            None,
            Instant::now(),
        );
    }

    /// Read the guns from the files again when the files changed, and the chosen gun
    /// from its cvar once.
    fn propgun_load(&mut self) {
        let Some(vfs) = self.vfs.clone() else {
            return;
        };
        let generation = Arc::as_ptr(&vfs) as usize;
        if self.propgun.guns_for != Some(generation) {
            let (guns, problems) = crate::propgun::catalogue(&vfs);
            for problem in problems {
                eprintln!("prop gun: {problem}");
            }
            self.propgun.guns = guns;
            self.propgun.guns_for = Some(generation);
        }
        if !self.propgun.gun_cvar_read {
            self.propgun.gun_cvar_read = true;
            let saved = self
                .console
                .as_ref()
                .and_then(|console| console.text_cvar(GUN_CVAR).ok())
                .map(|id| id.trim().to_ascii_lowercase());
            if let Some(id) = saved.filter(|id| crate::propgun::valid_id(id)) {
                self.propgun.chosen = Some(id);
            }
        }
        if self.propgun.gun().is_none() {
            self.propgun.chosen = self.propgun.guns.first().map(|gun| gun.id.clone());
        }
    }

    /// Choose gun `id`, saved in its cvar, its raise sound played when held.
    fn propgun_choose(&mut self, id: String) {
        if let Some(console) = self.console.as_mut() {
            console.set_cvar(GUN_CVAR, &id);
        }
        self.propgun.chosen = Some(id);
        self.propgun_raised();
    }

    /// The raise sound of the gun now held, and its name on the centre of the screen.
    fn propgun_raised(&mut self) {
        let Some((_, gun)) = self.propgun.holding() else {
            return;
        };
        let name = gun.name.clone();
        if let Some(raise) = gun.raise.clone() {
            self.propgun.cues.push(Cue {
                path: raise,
                at: None,
                mine: true,
            });
        }
        self.propgun_centre(&format!("{name} ^2ON"));
    }

    /// `toy_propgun`: on or off, said on the centre of the screen.
    pub(crate) fn toggle_propgun(&mut self) {
        self.propgun_load();
        self.propgun.toggle();
        if !self.propgun.on() {
            self.propgun_centre(&crate::toy_switch_words::switched("Prop gun", false));
        } else if self.propgun.gun().is_none() {
            self.propgun_centre(&format!(
                "Prop gun ^2ON^7: no gun found ({}/<name>{} in a pk3)",
                crate::propgun::FOLDER,
                crate::propgun::EXTENSION
            ));
        } else {
            self.propgun_raised();
        }
    }

    /// `propgun [<id>|next|prev]`: the guns listed, or one chosen.
    pub(crate) fn propgun_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        self.propgun_load();
        let ids: Vec<String> = self.propgun.guns.iter().map(|gun| gun.id.clone()).collect();
        let Some(word) = args.first().map(|word| word.to_ascii_lowercase()) else {
            if ids.is_empty() {
                return Ok(vec![format!(
                    "No prop guns. Put <name>{} in {}/ of a pk3: <name> is 1 to {} of a to z, 0 to 9 and _",
                    crate::propgun::EXTENSION,
                    crate::propgun::FOLDER,
                    crate::propgun::MAX_ID
                )]);
            }
            let chosen = self.propgun.chosen.clone();
            let mut lines = vec![format!("{} prop guns:", ids.len())];
            lines.extend(self.propgun.guns.iter().map(|gun| {
                let mark = if Some(&gun.id) == chosen.as_ref() {
                    " (chosen)"
                } else {
                    ""
                };
                format!("  {} ^7({}){mark}", gun.id, gun.name)
            }));
            return Ok(lines);
        };
        if ids.is_empty() {
            return Err("No prop guns to choose from".to_owned());
        }
        let at = self
            .propgun
            .chosen
            .as_ref()
            .and_then(|id| ids.iter().position(|other| other == id))
            .unwrap_or(0);
        let id = match word.as_str() {
            "next" => ids[(at + 1) % ids.len()].clone(),
            "prev" => ids[(at + ids.len() - 1) % ids.len()].clone(),
            id if ids.iter().any(|other| other == id) => id.to_owned(),
            id => return Err(format!("No prop gun {id}")),
        };
        self.propgun_choose(id.clone());
        Ok(vec![format!("Prop gun: {id}")])
    }

    /// The frame's toy logic, from the relay's events: what others hold and fire, and
    /// what to tell them.
    pub(crate) fn update_propgun(&mut self, relay_events: &[sjk_identity::ToyEvent]) {
        self.propgun_load();
        let turned = self.propgun.was_on != self.propgun.on;
        self.propgun.was_on = self.propgun.on;
        let Some(session) = self.live_session.as_ref() else {
            if self.propgun.server.is_some() {
                self.propgun.leave();
                self.propgun.server = None;
                self.propgun.server_id = None;
            }
            return;
        };
        let local = session.is_local();
        let server = session.server();
        let server_id = session.server_id();
        if self.propgun.server != Some(server) || self.propgun.server_id != Some(server_id) {
            self.propgun.leave();
            self.propgun.server = Some(server);
            self.propgun.server_id = Some(server_id);
        }
        let game_state = session.game_state();
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        self.propgun.duel = crate::order66::duelling(&session.latest_snapshot().player, gametype);
        self.propgun.unseen = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(SHOW_CVAR))
            == Some(false);
        let own = u8::try_from(game_state.client_num).ok();
        if turned && self.propgun.on {
            // Others' bolts fired before the switch do not count against the player.
            self.propgun.struck_by.clear();
        }
        if local {
            return;
        }
        let heard: Vec<(u8, Event)> = relay_events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && usize::from(event.slot) < SLOTS
                    && !self.muted_players.contains(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        let now = Instant::now();
        let time = self.propgun.time;
        if self.propgun.holders.len() < SLOTS {
            self.propgun.holders.resize(SLOTS, None);
        }
        for (slot, event) in heard {
            let holder = &mut self.propgun.holders[usize::from(slot)];
            match event {
                Event::Hold(id) => *holder = Some((id, now)),
                Event::Put => *holder = None,
                Event::Fire(shot, id) => {
                    *holder = Some((id.clone(), now));
                    self.propgun_heard_shot(slot, shot, &id, time);
                }
            }
        }
        let sjk = self.looks.sjk_slots();
        for (slot, holder) in self.propgun.holders.iter_mut().enumerate() {
            if sjk & (1 << slot) == 0
                || holder
                    .as_ref()
                    .is_some_and(|(_, told)| told.elapsed() > HOLD_EXPIRES)
            {
                *holder = None;
            }
        }
        // What the player holds: a put told to whoever heard the hold.
        let holding = self.propgun.holding().map(|(_, gun)| gun.id.clone());
        if holding != self.propgun.told_hold {
            if self.propgun.told_hold.is_some() && self.propgun.hold_told != 0 {
                let told = self.propgun.hold_told;
                self.propgun.outbox.push((Event::Put, told));
            }
            self.propgun.told_hold = holding;
            self.propgun.hold_told = 0;
            self.propgun.retell_at = Some(now + RETELL);
        }
        if self.propgun.retell_at.is_some_and(|at| now >= at) {
            self.propgun.hold_told = 0;
            self.propgun.retell_at = Some(now + RETELL);
        }
        self.propgun_tell(own, sjk);
    }

    /// A shot another player fired, heard at game server time `time`.
    fn propgun_heard_shot(&mut self, slot: u8, shot: Shot, id: &str, time: i64) {
        let Some(gun) = self.propgun.gun_index(id) else {
            return;
        };
        if self.propgun.knows(slot, &shot) || time - i64::from(shot.moment) > LATE_MS {
            return;
        }
        let (speed, life, sound) = {
            let gun = &self.propgun.guns[gun];
            (
                gun.bolt_speed,
                gun.bolt_life_ms,
                crate::propgun::one_of(&gun.fire, shot.number).map(str::to_owned),
            )
        };
        if let Some(path) = sound {
            self.propgun.cues.push(Cue {
                path,
                at: Some(shot.origin()),
                mine: false,
            });
        }
        self.propgun_muzzle(gun, &shot);
        self.propgun
            .add_bolt(Bolt::new(slot, gun, shot, speed, life));
    }

    /// Tell the next SJK player what they are to hear: a put first, then the player's
    /// newest shots, then what they hold.
    fn propgun_tell(&mut self, own: Option<u8>, sjk: u64) {
        let Some(own) = own else {
            return;
        };
        let now = Instant::now();
        if self.propgun.tell_at.is_some_and(|at| now < at) {
            return;
        }
        let others = sjk & !(1 << own);
        let send = |toy: &mut crate::propgun::PropGun, event: &Event, slot: u8| {
            toy.tell_at = Some(now + SEND_GAP);
            crate::player_identity::relay_toy_event(TOY, event.text(), slot, false);
        };
        while let Some((event, slots)) = self.propgun.outbox.first_mut() {
            *slots &= others;
            if *slots == 0 {
                self.propgun.outbox.remove(0);
                continue;
            }
            let slot = slots.trailing_zeros() as u8;
            *slots &= !(1 << slot);
            let event = event.clone();
            send(&mut self.propgun, &event, slot);
            return;
        }
        let time = self.propgun.time;
        let gun_ids: Vec<String> = self.propgun.guns.iter().map(|gun| gun.id.clone()).collect();
        if let Some(bolt) = self.propgun.bolts.iter_mut().rev().find(|bolt| {
            bolt.shooter == own
                && others & !bolt.told != 0
                && time - i64::from(bolt.shot.moment) <= LATE_MS
        }) {
            let slot = (others & !bolt.told).trailing_zeros() as u8;
            bolt.told |= 1 << slot;
            let event = Event::Fire(bolt.shot, gun_ids[bolt.gun].clone());
            send(&mut self.propgun, &event, slot);
            return;
        }
        if let Some(id) = self.propgun.told_hold.clone() {
            let untold = others & !self.propgun.hold_told;
            if untold != 0 {
                let slot = untold.trailing_zeros() as u8;
                self.propgun.hold_told |= 1 << slot;
                send(&mut self.propgun, &Event::Hold(id), slot);
            }
        }
    }

    /// What a small box meets between two points of the map's brushes: where, its
    /// normal, and how far along (0 to 1).
    fn propgun_trace(&mut self, from: Vec3, to: Vec3) -> Option<(Vec3, Vec3, f32)> {
        let bounds = sjk_bsp::Aabb::new([-1.0; 3], [1.0; 3]).expect("the bolt's bounds are valid");
        let trace = self.bsp.trace_box_with(
            &mut self.trace_scratch,
            from.to_array(),
            to.to_array(),
            bounds,
            BOLT_MASK,
        );
        if trace.start_solid || trace.fraction >= 1.0 {
            return None;
        }
        let normal = trace
            .plane
            .as_ref()
            .map_or(Vec3::Z, |plane| Vec3::from_array(plane.normal));
        Some((Vec3::from_array(trace.end_position), normal, trace.fraction))
    }

    /// Whether the player may fire now (a gun held and the Ragdoll's rules).
    fn propgun_may_fire(&self, local: bool) -> bool {
        self.propgun.holding().is_some()
            && !self.propgun.duel
            && !self.ragdoll.on()
            && (local || self.ragdoll.strikable())
    }

    /// The player's own body as a bolt may strike it, if it may: the toy on, not
    /// typing, away, dead, in a duel or held.
    fn propgun_target(&self) -> Option<Target> {
        if !self.propgun.on
            || !self.ragdoll.strikable()
            || self.talk_button()
            || self.hax_own_away()
        {
            return None;
        }
        if let Some(points) = self.ragdoll.own_points() {
            return Some(Target::Limp(points));
        }
        let state = self.local_prediction.predicted_state()?;
        let origin = Vec3::from_array(state.origin);
        let top = (state.view_height as f32 + 4.0).max(16.0);
        Some(Target::Standing {
            low: origin + Vec3::new(-15.0, -15.0, -24.0),
            high: origin + Vec3::new(15.0, 15.0, top),
            points: crate::ragdoll::standing(
                origin,
                state.view_angles[1],
                self.ragdoll_own_scale(),
            ),
        })
    }

    /// The player fires now: a bolt from by the eyes to what the crosshair meets.
    fn propgun_fire(&mut self, own: u8, time: i64) {
        let Some((gun_index, gun)) = self.propgun.holding() else {
            return;
        };
        let (speed, life, refire) = (gun.bolt_speed, gun.bolt_life_ms, gun.refire_ms);
        if self
            .propgun
            .last_shot
            .is_some_and(|last| time - last < refire && time >= last)
        {
            return;
        }
        let Some(state) = self.local_prediction.predicted_state() else {
            return;
        };
        let eye = Vec3::from_array(state.origin) + Vec3::Z * state.view_height as f32;
        let [pitch, yaw, _] = state.view_angles;
        let forward = crate::propgun::looking(pitch, yaw);
        let right = Vec3::new(yaw.to_radians().sin(), -yaw.to_radians().cos(), 0.0);
        let aim = self
            .propgun_trace(eye, eye + forward * AIM_REACH)
            .map_or(eye + forward * AIM_REACH, |(at, _, _)| at);
        let mut from = self
            .propgun
            .view_muzzle
            .unwrap_or(eye + forward * 16.0 + right * 5.0 - Vec3::Z * 5.0);
        // Not from inside a wall in front of the player.
        if self.propgun_trace(eye, from).is_some() {
            from = eye;
        }
        let way = (aim - from).normalize_or(forward);
        let number = self.propgun.next_number;
        self.propgun.next_number = (number + 1) % NUMBERS;
        self.propgun.last_shot = Some(time);
        let shot = Shot::new(number, time as i32, from, way);
        let gun = &self.propgun.guns[gun_index];
        let own_sound = crate::propgun::one_of(&gun.fire_own, number)
            .or_else(|| crate::propgun::one_of(&gun.fire, number))
            .map(str::to_owned);
        if let Some(path) = own_sound {
            self.propgun.cues.push(Cue {
                path,
                at: None,
                mine: true,
            });
        }
        self.propgun_muzzle(gun_index, &shot);
        self.propgun
            .add_bolt(Bolt::new(own, gun_index, shot, speed, life));
    }

    /// The gun's muzzle effect where shot `shot` leaves, along its way.
    fn propgun_muzzle(&mut self, gun: usize, shot: &Shot) {
        if let Some(effect) = self.propgun.guns[gun].fire_effect.clone() {
            // Where the shot left: the drawn muzzle, for the player's own in first person.
            self.propgun.splashes.push(Splash {
                effect,
                at: shot.origin(),
                normal: shot.direction(),
            });
        }
    }

    /// Fire if attack asks, then step every bolt on to `time` (game server
    /// milliseconds, the presentation's): against the world, and against the player's
    /// own body. Before the Ragdoll's bodies are stepped, so a blow lands this frame.
    pub(crate) fn step_propgun(&mut self, time: i64) {
        self.propgun.time = time;
        let Some(session) = self.live_session.as_ref() else {
            // A demo's world shots keep the bolts they placed.
            if self.demo_session.is_none() {
                self.propgun.bolts.clear();
            }
            return;
        };
        let local = session.is_local();
        let Some(own) = u8::try_from(session.game_state().client_num).ok() else {
            return;
        };
        let pressed = std::mem::take(&mut self.propgun.pressed);
        if (pressed || self.propgun.held) && self.propgun_may_fire(local) {
            self.propgun_fire(own, time);
        }
        let target = if local { None } else { self.propgun_target() };
        let immune = self
            .propgun
            .last_struck
            .is_some_and(|when| (0..IMMUNE_MS).contains(&(time - when)));
        let mut index = 0;
        while index < self.propgun.bolts.len() {
            let bolt = self.propgun.bolts[index].clone();
            if bolt.end.is_some() || time <= bolt.stepped {
                index += 1;
                continue;
            }
            let until = time.min(i64::from(bolt.shot.moment) + bolt.life_ms);
            let (a, b) = (bolt.at(bolt.stepped), bolt.at(until));
            let wall = self.propgun_trace(a, b);
            let reach = wall.map_or(b, |(at, _, _)| at);
            let span = (until - bolt.stepped) as f32;
            let mut end = None;
            if let Some(target) = target.as_ref().filter(|_| {
                bolt.shooter != own
                    && !immune
                    && !self
                        .propgun
                        .struck_by
                        .contains(&(bolt.shooter, bolt.shot.number))
            }) && let Some((at, joint)) = target.struck_along(a, reach)
            {
                let along = (at - a).length() / (b - a).length().max(0.001);
                let hit_time = bolt.stepped + (span * along) as i64;
                let push =
                    crate::propgun::blow(bolt.shot.direction(), self.propgun.guns[bolt.gun].push);
                let by = crate::ragdoll::By {
                    thrower: bolt.shooter,
                    number: crate::ragdoll::UNTHROWN,
                    step: 0,
                };
                if self.ragdoll_struck(hit_time, joint, push, by) {
                    self.propgun.last_struck = Some(hit_time);
                    self.propgun
                        .struck_by
                        .push((bolt.shooter, bolt.shot.number));
                    if self.propgun.struck_by.len() > 32 {
                        self.propgun.struck_by.remove(0);
                    }
                    end = Some(End::Body { time: hit_time, at });
                }
            }
            if end.is_none()
                && let Some((at, normal, fraction)) = wall
            {
                end = Some(End::Wall {
                    time: bolt.stepped + (span * fraction) as i64,
                    at,
                    normal,
                });
            }
            if end.is_none() && until >= i64::from(bolt.shot.moment) + bolt.life_ms {
                end = Some(End::Spent);
            }
            let impact = match end {
                Some(End::Wall { at, normal, .. }) => Some((at, normal)),
                Some(End::Body { at, .. }) => Some((at, -bolt.shot.direction())),
                _ => None,
            };
            if let Some((at, normal)) = impact {
                let gun = &self.propgun.guns[bolt.gun];
                if let Some(path) = crate::propgun::one_of(&gun.impact, bolt.shot.number) {
                    self.propgun.cues.push(Cue {
                        path: path.to_owned(),
                        at: Some(at),
                        mine: bolt.shooter == own,
                    });
                }
                if let Some(effect) = gun.impact_effect.clone() {
                    self.propgun.splashes.push(Splash { effect, at, normal });
                }
            }
            let bolt = &mut self.propgun.bolts[index];
            bolt.stepped = until;
            bolt.end = end;
            index += 1;
        }
        // Bolts done with go once told (the shooter's own) or a moment after.
        self.propgun
            .bolts
            .retain(|bolt| time - bolt.last_time() < LATE_MS);
    }

    /// The gun each player holds in third person this frame, by slot: the player's own,
    /// and others' unless hidden here or in a duel.
    pub(crate) fn propgun_held_draws(&self) -> [Option<HeldDraw>; SLOTS] {
        let mut draws = [None; SLOTS];
        let own = self
            .live_session
            .as_ref()
            .and_then(|session| u8::try_from(session.game_state().client_num).ok());
        let mesh_of = |gun: &Gun| {
            let mesh = self.object_meshes.iter().position(|mesh| {
                mesh.appearance.variant.is_empty() && mesh.appearance.model == gun.world.model
            })?;
            Some(HeldDraw {
                mesh,
                offset: gun.world.offset,
                rotation: gun.world.rotation(),
                scale: gun.world.scale,
                muzzle: None,
            })
        };
        if let (Some(own), Some((_, gun))) = (own, self.propgun.holding()) {
            draws[usize::from(own)] = mesh_of(gun);
        }
        #[cfg(test)]
        if self.propgun.shot_all
            && let Some((_, gun)) = self.propgun.gun()
        {
            return [mesh_of(gun); SLOTS];
        }
        if self.propgun.duel {
            return draws;
        }
        if !self.propgun.unseen {
            for (slot, draw) in draws.iter_mut().enumerate() {
                if Some(slot as u8) == own {
                    continue;
                }
                if let Some(gun) = self.propgun.held_by(slot) {
                    *draw = mesh_of(&self.propgun.guns[gun]);
                }
            }
        }
        draws
    }

    /// The player's own gun in first person: its mesh and placing, when held.
    pub(crate) fn propgun_view(&self) -> Option<HeldDraw> {
        let (_, gun) = self.propgun.holding()?;
        let mesh = self.object_meshes.iter().position(|mesh| {
            mesh.appearance.variant.is_empty() && mesh.appearance.model == gun.view.model
        })?;
        Some(HeldDraw {
            mesh,
            offset: gun.view.offset,
            rotation: gun.view.rotation(),
            scale: gun.view.scale,
            muzzle: gun.view_muzzle,
        })
    }

    /// Add this frame's bolts and their lights.
    pub(crate) fn submit_propgun(&mut self, presentation_time: i64) {
        self.propgun.loops.clear();
        if self.propgun.bolts.is_empty() || self.propgun.duel {
            return;
        }
        let own = self
            .live_session
            .as_ref()
            .and_then(|session| u8::try_from(session.game_state().client_num).ok());
        for index in 0..self.propgun.bolts.len() {
            let bolt = &self.propgun.bolts[index];
            if !bolt.flying(presentation_time) || (self.propgun.unseen && Some(bolt.shooter) != own)
            {
                continue;
            }
            let gun = &self.propgun.guns[bolt.gun];
            let at = bolt.at(presentation_time);
            if let Some(sound) = gun.bolt_sound.clone()
                && self.propgun.loops.len() < MAX_LOOPS
            {
                let source = sjk_audio::SourceId(
                    SOURCE.0 + 1 + u32::from(bolt.shooter) * 128 + u32::from(bolt.shot.number),
                );
                self.propgun.loops.push((source, sound, at.to_array()));
            }
            if let Some(effect) = gun.bolt_effect.clone()
                && bolt.trailed.saturating_add(TRAIL_GAP) <= presentation_time
            {
                self.propgun.splashes.push(Splash {
                    effect,
                    at,
                    normal: -bolt.shot.direction(),
                });
                self.propgun.bolts[index].trailed = presentation_time;
            }
            let bolt = &self.propgun.bolts[index];
            let gun = &self.propgun.guns[bolt.gun];
            let light = crate::dynamic_lights::PointLight {
                origin: at.to_array(),
                radius: gun.bolt_light,
                color: gun.bolt_color,
            };
            let model = gun.bolt_model.clone();
            let scale = gun.bolt_scale;
            let turn = Quat::from_rotation_arc(Vec3::X, bolt.shot.direction());
            if light.radius > 0.0 {
                self.dynamic_lights.push_radiant(light);
            }
            if let Some(model) = model
                && let Some(mesh) = self.object_meshes.iter().position(|mesh| {
                    mesh.appearance.variant.is_empty() && mesh.appearance.model == model
                })
            {
                self.object_groups[mesh].push(ActorInstance::new(
                    at.to_array(),
                    turn.to_array(),
                    [scale; 3],
                ));
            }
        }
    }

    /// Play the toy's sounds and start its impact effects of the frame.
    pub(crate) fn play_propgun_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let splashes: Vec<Splash> = self.propgun.splashes.drain(..).collect();
        if !self.propgun.duel
            && let Some(vfs) = self.vfs.clone()
        {
            for (index, Splash { effect, at, normal }) in splashes.into_iter().enumerate() {
                crate::effect_runtime::spawn_effect(
                    &mut self.particles,
                    &mut self.effect_aux,
                    &mut self.effects,
                    &vfs,
                    &effect,
                    at,
                    Instant::now(),
                    (self.propgun.time as u32) ^ (index as u32).wrapping_mul(0x9e37_79b9),
                    0,
                    audio,
                    crate::combat_effects::rotation_from_direction(normal.to_array()),
                );
            }
        }
        let Some(audio) = audio.as_mut() else {
            self.propgun.cues.clear();
            return;
        };
        audio.set_propgun_loops(&self.propgun.loops);
        if self.propgun.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            let paths: Vec<String> = self
                .propgun
                .guns
                .iter()
                .flat_map(|gun| gun.sounds().map(str::to_owned).collect::<Vec<_>>())
                .collect();
            audio.register_effect_paths(vfs, paths.iter().map(String::as_str));
            self.propgun.sounds_registered = Some(audio.generation());
        }
        for Cue { path, at, mine } in self.propgun.cues.drain(..) {
            if self.propgun.duel || (self.propgun.unseen && !mine) {
                continue;
            }
            audio.play_toy(
                crate::toy_sounds::Toy::PropGun,
                &path,
                1.0,
                at.map(|at| at.to_array()),
                SOURCE,
                sjk_audio::ChannelId(0),
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: the player holds gun `id` (read from the game data), and
    /// with `everyone` every player does.
    pub(crate) fn shot_propgun(&mut self, id: &str, everyone: bool) {
        self.propgun_load();
        self.propgun.chosen = Some(id.to_owned());
        self.propgun.on = true;
        self.propgun.shot_all = everyone;
        if let Some(console) = self.console.as_mut() {
            console.set_cvar(crate::toy_saved::PROPGUN_CVAR, "1");
        }
        assert!(self.propgun.gun().is_some(), "gun {id} in the game data");
    }

    /// For the world shots: the chosen gun's muzzle effect where a shot leaves the
    /// rendered view, and its impact effect where the crosshair meets the world.
    pub(crate) fn shot_propgun_effects(&mut self) {
        let Some((gun, def)) = self.propgun.gun() else {
            return;
        };
        let (yaw, pitch) = (self.camera_yaw, self.camera_pitch);
        let way = Vec3::new(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        let right = Vec3::new(yaw.sin(), -yaw.cos(), 0.0);
        let from = self
            .propgun
            .view_muzzle
            .unwrap_or(self.view_origin + way * 16.0 + right * 5.0 - Vec3::Z * 5.0);
        let impact = def.impact_effect.clone();
        self.propgun_muzzle(gun, &Shot::new(0, 0, from, way));
        if let (Some(effect), Some((at, normal, _))) =
            (impact, self.propgun_trace(from, from + way * AIM_REACH))
        {
            self.propgun.splashes.push(Splash {
                effect: effect.clone(),
                at,
                normal,
            });
            // And one on the floor close ahead, to see it.
            let ahead = self.view_origin + Vec3::new(way.x, way.y, 0.0).normalize_or_zero() * 160.0;
            if let Some((at, normal, _)) = self.propgun_trace(ahead, ahead - Vec3::Z * 400.0) {
                self.propgun.splashes.push(Splash { effect, at, normal });
            }
        }
    }

    /// For the world shots: a bolt of the chosen gun flying from `from` along `way`,
    /// `age` milliseconds out at game server time `time`.
    pub(crate) fn shot_propgun_bolt(&mut self, from: Vec3, way: Vec3, time: i64, age: i64) {
        let Some((gun, def)) = self.propgun.gun() else {
            return;
        };
        let (speed, life) = (def.bolt_speed, def.bolt_life_ms);
        let shot = Shot::new(0, (time - age) as i32, from, way);
        self.propgun.add_bolt(Bolt::new(0, gun, shot, speed, life));
    }
}
