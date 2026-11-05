//! `EF_DISINTEGRATION`: a body burning away after a disruptor kill, or a corpse shot or
//! cut until its health runs out (`body_die`, `g_combat.c`).
//!
//! cgame freezes the pose (`cg_players.c`, the `EF_DISINTEGRATION` block of `CG_Player`;
//! `cg_ents.c` for `ET_BODY`) and draws the entity twice in `CG_Disintegration`
//! (`cg_ents.c`): the model with `RF_DISINTEGRATE1`, eaten away from the hit point, and
//! again with `gfx/effects/burn` and `RF_DISINTEGRATE2`, the glowing edge. rd-vanilla
//! does the work per vertex (`RB_CalcDisintegrateColors`, `RB_CalcDisintegrateVertDeform`,
//! `tr_shade_calc.cpp`); the stage program does it here from the instance this module
//! marks. A player stops drawing 1.5 s in; a body draws until the server frees it.

use crate::ActorInstance;
use sjk_protocol::Snapshot;
use std::cell::Cell;

/// `EF_DISINTEGRATION` (`bg_public.h`).
pub(crate) const EF_DISINTEGRATION: u32 = 1 << 26;
/// Instance `view_flags` bit for `RF_DISINTEGRATE1`, read by `stage_runtime.wgsl`.
pub(crate) const RF_DISINTEGRATE1: u32 = 1 << 24;
/// Instance `view_flags` bit for `RF_DISINTEGRATE2`, read by `stage_runtime.wgsl`.
pub(crate) const RF_DISINTEGRATE2: u32 = 1 << 25;
/// A disintegrating player is no longer drawn after this (`cg_players.c`).
pub(crate) const PLAYER_MILLIS: i64 = 1_500;
/// `cgs.media.disruptorShader`.
pub(crate) const BURN_SHADER: &str = "gfx/effects/burn";
/// `cgs.effects.mDisruptorDeathSmoke`.
pub(crate) const SMOKE_EFFECT: &str = "disruptor/death_smoke";

/// One actor's disintegration (`centity_t::dustTrailTime`).
#[derive(Debug)]
pub(crate) struct State {
    /// Presentation time the flag was first seen.
    pub(crate) started: i64,
    /// World-space hit location: `origin2`, the player state's `lastHitLoc`.
    pub(crate) hit: [f32; 3],
    /// Started by this client, not the server ([`Forced`]): a body goes once burnt, as a
    /// player does, for the server keeps it.
    pub(crate) forced: bool,
    smoke_tick: Cell<i64>,
}

impl State {
    pub(crate) fn new(started: i64, hit: [f32; 3]) -> Self {
        Self {
            started,
            hit,
            forced: false,
            smoke_tick: Cell::new(i64::MIN),
        }
    }

    /// The burn radius `(time - endTime) * 0.045` (`RB_CalcDisintegrateColors`).
    pub(crate) fn threshold(&self, now: i64) -> f32 {
        (now - self.started).max(0) as f32 * 0.045
    }

    /// `CG_Disintegration` puffs smoke once per 50 ms frame for the first second.
    pub(crate) fn smoke_due(&self, now: i64) -> bool {
        let tick = now.div_euclid(50);
        if now - self.started >= 1_000 || self.smoke_tick.get() == tick {
            return false;
        }
        self.smoke_tick.set(tick);
        true
    }

    /// Mark `instance` for one of the two passes, with the hit point and radius the
    /// stage program reads in place of the (unused) entity light.
    pub(crate) fn mark(&self, instance: &mut ActorInstance, pass: u32, now: i64) {
        instance.view_flags |= pass;
        instance.light_direction = self.hit;
        instance.light_directed = [self.threshold(now), 0.0, 0.0];
    }
}

/// `EF_DEAD` (`bg_public.h`).
const EF_DEAD: u32 = 1 << 1;

/// TRACE-DISINTEGRATE (temporary): whether `SJK_TRACE_DISINTEGRATE` asks for the forced
/// disintegrations' steps in the log.
pub(crate) fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SJK_TRACE_DISINTEGRATE").is_some())
}

/// TRACE-DISINTEGRATE (temporary): one step, when tracing.
pub(crate) fn trace(arguments: std::fmt::Arguments<'_>) {
    if tracing() {
        crate::log::progress(format_args!("disintegrate: {arguments}"));
    }
}
/// The most forced disintegrations followed at once, and how long one is kept at most.
const MOST_FORCED: usize = 64;
const FORGET_MILLIS: i64 = 120_000;
/// Where the burn starts: this far above a victim's origin, about the chest.
const HIT_ABOVE: f32 = 8.0;

/// The disintegrations this client starts itself: the Disintegrate finisher
/// (`crate::finishers`), on a saber kill by its wearer
/// ([`crate::GpuState::observe_saber_kills`]). The server knows
/// nothing of them: the victim burns away as a disruptor's does and stays hidden until it
/// lives again, and the body the server copies of a player as they respawn
/// (`corpse_actors.rs`) is hidden from the start.
#[derive(Debug, Default)]
pub(crate) struct Forced {
    entities: Vec<Forcing>,
    /// The clients burnt so, until the server copies their body.
    awaiting_body: Vec<Awaiting>,
}

/// One entity burning away: from when, where the burn starts (once its place is known),
/// whether it has been seen dead, and, for a body copy, whose.
#[derive(Clone, Copy, Debug)]
struct Forcing {
    number: u16,
    start: i64,
    hit: Option<[f32; 3]>,
    seen_dead: bool,
    body: Option<u16>,
}

/// A client burnt away whose body the server has not copied yet (its `ircg` command):
/// from when.
#[derive(Clone, Copy, Debug)]
struct Awaiting {
    client: u16,
    start: i64,
}

impl Forced {
    /// Entity `number` was killed with a blade that burns its kills away, from `start`
    /// (presentation ms).
    pub(crate) fn kill(&mut self, number: u16, start: i64) {
        trace(format_args!("kill of {number} burns from {start}"));
        self.entities.retain(|forcing| forcing.number != number);
        if self.entities.len() < MOST_FORCED {
            self.entities.push(Forcing {
                number,
                start,
                hit: None,
                seen_dead: false,
                body: None,
            });
        }
        if usize::from(number) < crate::saber_skins::MAX_CLIENTS {
            self.awaiting_body
                .retain(|awaiting| awaiting.client != number);
            self.awaiting_body.push(Awaiting {
                client: number,
                start,
            });
        }
    }

    /// The server copied client `client`'s body to entity `body` (its `ircg` command):
    /// hidden from the start when that client burnt away; another's body there before is
    /// not.
    pub(crate) fn body_copied(&mut self, client: u16, body: u16) {
        trace(format_args!(
            "ircg: body {body} of client {client}, awaited {}",
            self.awaiting_body
                .iter()
                .any(|awaiting| awaiting.client == client)
        ));
        self.entities
            .retain(|forcing| forcing.number != body || forcing.body == Some(client));
        if let Some(index) = self
            .awaiting_body
            .iter()
            .position(|awaiting| awaiting.client == client)
        {
            let awaiting = self.awaiting_body.swap_remove(index);
            self.hide_body(body, awaiting);
        }
    }

    /// Hide body `number`, the copy of `awaiting`'s client, from the start.
    fn hide_body(&mut self, number: u16, awaiting: Awaiting) {
        trace(format_args!(
            "body {number} of client {} hidden",
            awaiting.client
        ));
        if self.entities.len() < MOST_FORCED
            && !self.entities.iter().any(|forcing| forcing.number == number)
        {
            self.entities.push(Forcing {
                number,
                start: awaiting.start,
                hit: Some([0.0; 3]),
                seen_dead: true,
                body: Some(awaiting.client),
            });
        }
    }

    /// The server freed body `number`.
    pub(crate) fn freed(&mut self, number: u16) {
        self.entities
            .retain(|forcing| !(forcing.body.is_some() && forcing.number == number));
    }

    /// Follow `snapshot` at `now`: the burn starts where each victim is; a player alive
    /// again after being seen dead is drawn again; an NPC gone from the snapshot is
    /// forgotten (a player out of sight is not); and none is kept past
    /// [`FORGET_MILLIS`]. `local` is the viewer's own entity number.
    pub(crate) fn update(&mut self, snapshot: &Snapshot, local: Option<u16>, now: i64) {
        self.awaiting_body
            .retain(|awaiting| now - awaiting.start <= FORGET_MILLIS);
        self.entities.retain_mut(|forcing| {
            if now - forcing.start > FORGET_MILLIS {
                return false;
            }
            if forcing.body.is_some() {
                return true;
            }
            let seen = if Some(forcing.number) == local {
                Some((snapshot.player.health() <= 0, snapshot.player.origin()))
            } else {
                snapshot
                    .entities
                    .binary_search_by_key(&forcing.number, |state| state.number())
                    .ok()
                    .map(|index| {
                        let state = &snapshot.entities[index];
                        (state.e_flags() & EF_DEAD != 0, state.trajectory_base())
                    })
            };
            let Some((dead, origin)) = seen else {
                return usize::from(forcing.number) < crate::saber_skins::MAX_CLIENTS;
            };
            forcing
                .hit
                .get_or_insert([origin[0], origin[1], origin[2] + HIT_ABOVE]);
            if dead && !forcing.seen_dead {
                trace(format_args!(
                    "{} seen dead at {now}, at {origin:?}",
                    forcing.number
                ));
            }
            if !dead && forcing.seen_dead {
                trace(format_args!(
                    "{} alive again at {now}, at {origin:?}",
                    forcing.number
                ));
            }
            forcing.seen_dead |= dead;
            dead || !forcing.seen_dead
        });
    }

    /// TRACE-DISINTEGRATE (temporary): whether entity `number` (of client `client`, a
    /// body's) is one this follows: burning, a burnt client's, or a body awaited.
    pub(crate) fn watching(&self, number: u16, client: Option<u16>) -> bool {
        self.entities.iter().any(|forcing| {
            forcing.number == number
                || client.is_some() && (Some(forcing.number) == client || forcing.body == client)
        }) || self
            .awaiting_body
            .iter()
            .any(|awaiting| Some(awaiting.client) == client)
    }

    /// How many body copies are hidden.
    #[cfg(test)]
    pub(crate) fn hidden_bodies(&self) -> usize {
        self.entities
            .iter()
            .filter(|forcing| forcing.body.is_some())
            .count()
    }

    /// Where entity `number`'s forced disintegration started and when, once it has begun
    /// by `now` and its place is known.
    /// A body copy's is over before it shows: it comes as its player respawns, with the
    /// burn short of its 1.5 s, and would be drawn whole (it was copied as it lay) until
    /// then.
    pub(crate) fn active(&self, number: u16, now: i64) -> Option<([f32; 3], i64)> {
        self.entities
            .iter()
            .find(|forcing| forcing.number == number && now >= forcing.start)
            .and_then(|forcing| {
                let started = match forcing.body {
                    Some(_) => forcing.start - PLAYER_MILLIS - 1,
                    None => forcing.start,
                };
                Some((forcing.hit?, started))
            })
    }
}

impl crate::GpuState {
    /// The kill feed's new kills: a saber kill (a thrown one too) by a wearer of the
    /// Disintegrate finisher (`crate::finishers`, the attacker's look) starts the
    /// victim's ([`Forced::kill`]), from the blow's server time and the finisher's delay.
    pub(crate) fn observe_saber_kills(&mut self) {
        let decoded = self.obituaries.decoded();
        let new = decoded.saturating_sub(self.saber_kills_seen).min(8) as usize;
        self.saber_kills_seen = decoded;
        for offset in (0..new).rev() {
            let Some(kill) = self.obituaries.feed().newest(offset) else {
                continue;
            };
            trace(format_args!(
                "obituary: {} killed {} by mod {} at {}",
                kill.attacker, kill.target, kill.means_of_death, kill.server_time
            ));
            let saber = u32::from(kill.means_of_death) == sjk_game_jka::means_of_death::MOD_SABER;
            if !saber
                || kill.target == kill.attacker
                || usize::from(kill.attacker) >= crate::saber_skins::MAX_CLIENTS
            {
                continue;
            }
            let Some(crate::finishers::Effect::Disintegrate { delay_ms }) = self
                .looks
                .finisher(usize::from(kill.attacker))
                .map(|finisher| finisher.effect)
            else {
                continue;
            };
            self.forced_disintegrations
                .kill(kill.target, i64::from(kill.server_time) + delay_ms);
        }
    }
}

/// The hit location of entity `number` when it is disintegrating in `snapshot`.
/// The viewer's own player comes from the player state (`lastHitLoc`), others from
/// their entity state (`origin2`, where `BG_PlayerStateToEntityState` copies it).
pub(crate) fn hit_location(snapshot: &Snapshot, local: bool, number: u16) -> Option<[f32; 3]> {
    if local {
        let player = &snapshot.player;
        if player.entity_flags() & EF_DISINTEGRATION == 0 {
            return None;
        }
        // lastHitLoc[0..3] are protocol fields 102, 105 and 100.
        return Some(
            [102, 105, 100].map(|field| f32::from_bits(player.raw_field(field).unwrap_or(0))),
        );
    }
    // Snapshot entities are ordered by number, so this is one binary search per
    // actor, not a scan of every entity.
    let index = snapshot
        .entities
        .binary_search_by_key(&number, |state| state.number())
        .ok()?;
    let state = &snapshot.entities[index];
    (state.e_flags() & EF_DISINTEGRATION != 0).then(|| state.origin2())
}

#[cfg(test)]
mod tests {
    use super::State;

    #[test]
    fn the_burn_radius_grows_from_the_hit() {
        let state = State::new(1_000, [0.0; 3]);
        assert_eq!(state.threshold(900), 0.0);
        assert!((state.threshold(2_000) - 45.0).abs() < 1e-4);
    }

    #[test]
    fn smoke_puffs_once_per_tick_for_a_second() {
        let state = State::new(1_000, [0.0; 3]);
        assert!(state.smoke_due(1_010));
        assert!(!state.smoke_due(1_020));
        assert!(state.smoke_due(1_060));
        assert!(!state.smoke_due(2_000));
    }
}
