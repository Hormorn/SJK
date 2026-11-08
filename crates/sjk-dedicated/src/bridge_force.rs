//! `WP_ForcePowersUpdate` for a playing client (`g_main.c:3316`), every frame: the
//! knockdown's lying and get-up ([`sjk_game_jka::knockdown::force_update`]), the first
//! half of the powers' update through the Force button ([`force_powers::begin`]), the
//! push or pull the button called for ([`force_throw::throw`], over [`ServerThrow`]:
//! the other players, the missiles, the Force-usable doors and buttons, the map), then the
//! rest ([`force_powers::finish`]).
//! Grip, lightning and drain reach the others through the NPCs' host
//! (`bridge_npc_force_frame.rs`): a blow on an NPC is dealt as it lands, a blow on another
//! player once the update is over. The others are the players and the begun NPCs
//! of the roster alike (`w_force.c` treats every client the same): an NPC is gripped,
//! struck, drained, pushed and pulled as a player is, and its pain and death run through
//! the roster (`G_Damage`'s NPC branches).

use super::{NativeGame, Peer, Told};
use crate::collision::{Void, WithPlayers, WorldCollision};
use crate::visibility::Eye;
use sjk_game_jka::crt_rand::CrtRand;
use sjk_game_jka::entity_clip::BoxObstacle;
use sjk_game_jka::entity_id::EntityId;
use sjk_game_jka::entity_pool::EntityPool;
use sjk_game_jka::event_entity::EventEntity;
use sjk_game_jka::force_powers;
use sjk_game_jka::force_throw::{
    self, Candidate, CandidateKind, MoverView, ThrowPlayer, ThrowWorld,
};
use sjk_game_jka::items::Pickup;
use sjk_game_jka::movers::{Door, MoverKind, MoverState};
use sjk_game_jka::npc_spawn::NpcActor;
use sjk_game_jka::player_death::Rng;
use sjk_game_jka::pmove::{MovementCollision, MovementTrace};
use sjk_game_jka::registries::SoundTable;
use sjk_game_jka::weapon_fire::Missile;
use sjk_protocol::PlayerState;
use sjk_server::ServerWorld;

/// `CONTENTS_BODY`; a missile's `EF_MISSILE_STICK`; the missile wire fields a throw reads.
const CONTENTS_BODY: u32 = 0x100;
const EF_MISSILE_STICK: u32 = 1 << 22;
const ES_EFLAGS: usize = 19;
const ES_WEAPON: usize = 14;
const ES_POS_TYPE: usize = 23;
const ES_POS_BASE: [usize; 3] = [2, 1, 4];
/// `externalEvent`, which a weapon pulled from the hand raises (`EV_NOAMMO`).
const PS_EXTERNAL_EVENT: usize = 56;

impl NativeGame {
    /// The Force update for `client`, if it is playing; every player whose state changed
    /// has its movement reseeded.
    pub(super) fn force_update(&mut self, client: usize, server_time: i32) {
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if !peer.body_active() {
            return;
        }
        // Taken before the knockdown's reseed, which would forget it.
        let jump_sound = peer.movement.take_force_jump_sound();
        self.knockdown_update(client, server_time);
        self.seeker_drone(client, server_time);
        let command = peer_command(self, client);
        let begun = self.with_force_frame(client, server_time, |state, force, frame| {
            force_powers::begin(state, force, &command, jump_sound, frame)
        });
        let Some(begun) = begun else { return };
        // Gripped with its saber in flight: `Cmd_ToggleSaber_f` knocks it down.
        if begun.saber_knocked_down {
            self.knock_down_in_flight(client, server_time);
        }
        // Gripped with little Force where the mod says so (JA+'s `jp_gripOpenSaber` 2):
        // `saberKnockOutOfHand(saber, self, ps.velocity)`.
        if begun.saber_knocked_out {
            let velocity = self
                .peer(client)
                .map_or([0.0; 3], |peer| peer.state.velocity());
            self.knock_out(client, velocity, server_time);
        }
        if let Some(pull) = begun.throw {
            self.force_throw(client, pull, server_time);
        }
        let changed = self.with_force_frame(client, server_time, |state, force, frame| {
            force_powers::finish(state, force, begun, frame)
        });
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if changed == Some(true) {
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        if let Some(base) = peer.force.saber_base_reset.take() {
            peer.movement.set_saber_anim_level_base(base);
        }
    }

    /// `WP_ForcePowersUpdate`'s knockdown part: the lying and the get-up, on the wire
    /// state the movement restarts from; the roll-up's animation and sound.
    fn knockdown_update(&mut self, client: usize, server_time: i32) {
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let updated = sjk_game_jka::knockdown::force_update(
            &mut peer.state,
            &mut peer.knockdown,
            peer.health,
            &peer.last_command,
            server_time,
        );
        if updated.changed {
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        if updated.blocking_cleared {
            peer.movement.set_saber_blocking(0);
        }
        if let Some(animation) = updated.animation {
            use sjk_game_jka::pmove_anim::{
                SETANIM_BOTH, SETANIM_FLAG_HOLD, SETANIM_FLAG_OVERRIDE,
            };
            peer.movement.set_animation_parts(
                SETANIM_BOTH,
                animation,
                SETANIM_FLAG_OVERRIDE | SETANIM_FLAG_HOLD,
            );
            peer.movement.write_player_state(&mut peer.state);
        }
        if updated.events.is_empty() {
            return;
        }
        let (sounds, told) = (&mut self.sounds, &mut self.told);
        let jump = updated.jump_sound.then(|| {
            sounds.index(b"*jump1.wav", &mut |index, value| {
                told.push(Told::ConfigString {
                    index,
                    previous: Vec::new(),
                    value: value.to_vec(),
                })
            })
        });
        for mut event in updated.events {
            if event.event == sjk_game_jka::knockdown::EV_ENTITY_SOUND {
                event.parameter = u32::from(jump.unwrap_or(0));
            }
            let _ = self.pool.spawn_temporary(event.state(), server_time, None);
        }
    }

    /// `ForceThrow` by `client`, over everyone — the players, the begun NPCs, the
    /// missiles; the players it changed restart their movement from their state (an NPC's
    /// restarts from its own at its next move).
    pub(super) fn force_throw(&mut self, client: usize, pull: bool, server_time: i32) {
        let context = force_throw::Throw {
            level_time: server_time,
            gametype: self.gametype,
            thrower: client as u16,
            pull,
            rules: self.mod_force,
            full_force: self.full_force_duels(),
        };
        // What every player was, to tell afterwards whom the throw changed (a throw is a
        // button press, not a per-frame cost).
        let before: Vec<Option<sjk_protocol::PlayerState>> = (0..self.players.places())
            .map(|number| self.peer(number).map(|peer| peer.state.clone()))
            .collect();
        // The brushes a throw's traces meet (the func_usable walls too, as movement's
        // `bridge_collision_gather` has them), and the movers it may reach.
        let brushes: Vec<BoxObstacle> =
            crate::collision::brush_obstacles(&self.breakables, &self.doors, server_time)
                .chain(self.stock.obstacles())
                .chain(self.scripts.brushes())
                .chain(crate::collision::usable_obstacles(&self.usable_entities))
                .collect();
        let movers: Vec<ThrowMover> = self
            .doors
            .iter()
            .filter_map(|(number, door)| ThrowMover::of(number.legacy_number(), door, server_time))
            .collect();
        let Self {
            server,
            world,
            players,
            pool,
            missiles,
            items,
            sounds,
            told,
            deaths,
            rand,
            map,
            npcs,
            gametype,
            ..
        } = self;
        let Some(world) = server.world_mut(*world) else {
            return;
        };
        let npcs = &mut npcs.roster.actors;
        let mut throw_world = ServerThrow {
            world,
            peers: players,
            npcs,
            invulnerable: 0,
            pool,
            missiles,
            items,
            sounds,
            told,
            rng: &mut deaths.rng,
            crt: rand,
            map: map.as_ref(),
            level_time: server_time,
            gametype: *gametype,
            brushes,
            movers,
            used: Vec::new(),
            push_items: context.rules.push_items > 0,
        };
        force_throw::throw(context, &mut throw_world);
        let used = std::mem::take(&mut throw_world.used);
        // `GEntity_UseFunc` on each door or button the throw moved, as the use key does.
        for number in used {
            let Some(index) = self
                .doors
                .iter()
                .position(|(id, _)| id.legacy_number() == number)
            else {
                continue;
            };
            self.use_door_at(index, client, server_time);
        }
        for (number, before) in before.into_iter().enumerate() {
            if let Some(peer) = self.peer_mut(number)
                && before.as_ref() != Some(&peer.state)
            {
                peer.movement = peer.movement.reseeded(&peer.state);
            }
        }
    }
}

/// The last command `client` sent, for the Force button.
fn peer_command(game: &mut NativeGame, client: usize) -> sjk_protocol::UserCommand {
    game.peer_mut(client)
        .map_or_else(Default::default, |peer| peer.last_command)
}

pub(super) use super::bridge_npcs::force_frame::ForceScratch;

/// A door the Force opens (`MOVER_FORCE_ACTIVATE`) or a button, as a throw reaches it:
/// its number, its linked box where it stands now, its kind and what the throw reads.
struct ThrowMover {
    number: u16,
    absmin: [f32; 3],
    absmax: [f32; 3],
    /// `s.pos.trBase`: where it stands now.
    origin: [f32; 3],
    kind: CandidateKind,
    view: MoverView,
}

impl ThrowMover {
    /// `func_door` spawnflag `MOVER_START_OPEN`, `MOVER_FORCE_ACTIVATE`; `func_button`
    /// spawnflag `SPF_BUTTON_FPUSHABLE`.
    const START_OPEN: u32 = 1;
    const FORCE_ACTIVATE: u32 = 2;
    const BUTTON_FPUSHABLE: u32 = 2;

    /// The mover a throw may reach, or `None` for one it never moves (a plat, a door
    /// without `MOVER_FORCE_ACTIVATE`).
    fn of(number: u16, door: &Door, level_time: i32) -> Option<Self> {
        let at_rest = matches!(door.state, MoverState::Pos1 | MoverState::Pos2);
        let kind = match door.kind {
            MoverKind::Door if door.spawnflags & Self::FORCE_ACTIVATE != 0 => {
                CandidateKind::ForceDoor { at_rest }
            }
            MoverKind::Button => CandidateKind::Button {
                pushable: door.spawnflags & Self::BUTTON_FPUSHABLE != 0,
            },
            _ => return None,
        };
        let start_open = door.kind == MoverKind::Door && door.spawnflags & Self::START_OPEN != 0;
        // `s.origin`: where the map put it, which START_OPEN made `pos2`.
        let spawned_at = if start_open { door.pos2 } else { door.pos1 };
        let origin = sjk_game_jka::movers::origin_at(door, level_time);
        // `SV_LinkEntity` grows a brush's box by a unit as it does any other.
        Some(Self {
            number,
            absmin: std::array::from_fn(|axis| origin[axis] + door.bounds.0[axis] - 1.0),
            absmax: std::array::from_fn(|axis| origin[axis] + door.bounds.1[axis] + 1.0),
            origin,
            kind,
            view: MoverView {
                state: door.state,
                pos1: door.pos1,
                pos2: door.pos2,
                start_open,
                origin_brush: spawned_at != [0.0; 3],
            },
        })
    }
}

/// A Force push or pull's reach on the server: the players by their places, the begun
/// NPCs, the missiles in flight, the Force-usable doors and buttons, the pool, the items a
/// pull drops, the map and its brushes.
struct ServerThrow<'a> {
    world: &'a mut ServerWorld<(), Peer>,
    peers: &'a crate::players::PlayerRoster,
    npcs: &'a mut [NpcActor],
    /// An NPC's `invulnerableTimer`, which nothing gives it: a throw's write lands here.
    invulnerable: i32,
    pool: &'a mut EntityPool,
    missiles: &'a mut Vec<(EntityId, Missile)>,
    items: &'a mut Vec<(EntityId, Pickup)>,
    sounds: &'a mut SoundTable,
    told: &'a mut Vec<Told>,
    rng: &'a mut Rng,
    crt: &'a mut CrtRand,
    map: Option<&'a super::LoadedMap>,
    level_time: i32,
    gametype: i32,
    /// The doors, lifts, breakables and scripted brushes, which the throw's traces meet.
    brushes: Vec<BoxObstacle>,
    movers: Vec<ThrowMover>,
    /// The movers the throw used, in order, for the game to use once it is over.
    used: Vec<u16>,
    /// The items are within reach (JA+'s `jp_pushAll`); the base game's throw lists none.
    push_items: bool,
}

impl ServerThrow<'_> {
    /// A playing peer by its place.
    fn peer(&mut self, number: u16) -> Option<&mut Peer> {
        let handle = self.peers.at(usize::from(number))?;
        self.world
            .entity_mut(handle)
            .filter(|peer| peer.begun && peer.body_active())
    }
}

impl ThrowWorld for ServerThrow<'_> {
    fn entities_in_box(&mut self, mins: [f32; 3], maxs: [f32; 3], out: &mut Vec<Candidate>) {
        out.clear();
        player_candidates(self.world, self.peers, mins, maxs, out);
        missile_candidates(self.missiles, mins, maxs, out);
        out.extend(
            self.movers
                .iter()
                .filter(|mover| overlaps(mover.absmin, mover.absmax, mins, maxs))
                .map(|mover| Candidate {
                    number: mover.number,
                    kind: mover.kind,
                    absmin: mover.absmin,
                    absmax: mover.absmax,
                    origin: mover.origin,
                }),
        );
        out.extend(
            self.npcs
                .iter()
                .filter_map(|npc| sjk_game_jka::npc_force_throw::npc_candidate(npc, mins, maxs)),
        );
        if self.push_items {
            item_candidates(self.items, mins, maxs, out);
        }
        out.sort_by_key(|candidate| candidate.number);
    }

    fn item(&mut self, number: u16) -> Option<&mut Pickup> {
        self.items
            .iter_mut()
            .find(|(id, _)| id.legacy_number() == number)
            .map(|(_, pickup)| pickup)
    }

    fn trace(&mut self, start: [f32; 3], end: [f32; 3], pass: u16, mask: u32) -> MovementTrace {
        // A push is a button press, not a per-frame cost: the bodies are gathered afresh.
        let mut players = Vec::new();
        for number in 0..self.peers.places() as u16 {
            if number == pass {
                continue;
            }
            let Some(peer) = self.peer(number) else {
                continue;
            };
            let (mut bounds, mut contents) = (peer.movement.box_bounds(), CONTENTS_BODY);
            if let Some((corpse_contents, top)) = peer.corpse {
                (bounds.1[2], contents) = (top, corpse_contents);
            }
            players.push(BoxObstacle {
                entity: number,
                origin: peer.state.origin(),
                bounds,
                contents,
                model: None,
            });
        }
        players.extend(
            self.npcs
                .iter()
                .filter(|npc| npc.begun() && npc.contents != 0 && npc.number != pass)
                .map(NpcActor::body),
        );
        players.extend(self.brushes.iter().copied());
        match self.map {
            Some(map) => WithPlayers {
                world: WorldCollision {
                    bsp: &map.bsp,
                    scratch: &map.scratch,
                },
                players: &players,
            }
            .trace(start, [0.0; 3], [0.0; 3], end, mask),
            None => WithPlayers {
                world: Void,
                players: &players,
            }
            .trace(start, [0.0; 3], [0.0; 3], end, mask),
        }
    }

    fn in_pvs(&mut self, from: [f32; 3], to: [f32; 3]) -> bool {
        self.map
            .is_none_or(|map| Eye::new(&map.bsp, &map.areas, from).sees_point(&map.bsp, to))
    }

    fn player(&mut self, number: u16) -> Option<ThrowPlayer<'_>> {
        if let Some(at) = self.npcs.iter().position(|npc| npc.number == number) {
            let npc = &mut self.npcs[at];
            return npc.begun().then(|| {
                sjk_game_jka::npc_force_throw::npc_as_thrown(npc, &mut self.invulnerable)
            });
        }
        let peer = self.peer(number)?;
        Some(ThrowPlayer {
            state: &mut peer.state,
            force: &mut peer.force,
            health: peer.health,
            knockdown: &mut peer.knockdown,
            other_killer: &mut peer.wounds.other_killer,
            npc_class: None,
            invulnerable_until: &mut peer.invulnerable_until,
            command: peer.last_command,
            team: peer.session.team,
            push_effect_until: &mut peer.push_effect_until,
            lock_hits: &mut peer.lock.hits,
        })
    }

    fn reflect_missile(&mut self, missile: u16, thrower: u16, forward: [f32; 3]) {
        let Some(thrower_origin) = self.peer(thrower).map(|peer| peer.state.origin()) else {
            return;
        };
        let Some(owner) = self
            .missiles
            .iter()
            .find(|(number, _)| number.legacy_number() == missile)
            .map(|(_, missile)| missile.owner)
        else {
            return;
        };
        let shooter_origin = self
            .peer(owner)
            .map(|peer| peer.state.origin())
            .or_else(|| {
                self.npcs
                    .iter()
                    .find(|npc| npc.number == owner)
                    .map(|npc| npc.current_origin)
            });
        reflect_from(
            self.missiles,
            self.crt,
            missile,
            (thrower, thrower_origin),
            forward,
            shooter_origin,
            self.level_time,
        );
    }

    /// `TossClientWeapon` on a player a pull reached; an NPC's own weapon is not tossed
    /// (its weapon's pickup is not ported).
    fn toss_weapon(&mut self, victim: u16, direction: [f32; 3], speed: f32) {
        let (level_time, gametype) = (self.level_time, self.gametype);
        let Some(peer) = self.peer(victim) else {
            return;
        };
        if let Some(dropped) = toss_weapon_of(peer, direction, speed, gametype, level_time)
            && let Some(number) = self.pool.spawn_entity(dropped.state.clone(), level_time)
        {
            self.pool.set_bounds(number, dropped.bounds);
            self.items.push((number, dropped));
        }
    }

    fn raise(&mut self, event: EventEntity) {
        let _ = self
            .pool
            .spawn_temporary(event.state(), self.level_time, None);
    }

    fn sound_index(&mut self, name: &[u8]) -> u16 {
        let told = &mut *self.told;
        self.sounds.index(name, &mut |index, value| {
            told.push(Told::ConfigString {
                index,
                previous: Vec::new(),
                value: value.to_vec(),
            })
        })
    }

    fn rng(&mut self) -> &mut Rng {
        self.rng
    }

    fn mover(&mut self, number: u16) -> Option<MoverView> {
        self.movers
            .iter()
            .find(|mover| mover.number == number)
            .map(|mover| mover.view)
    }

    fn use_mover(&mut self, number: u16, _thrower: u16) {
        self.used.push(number);
    }
}

/// The linked box of a peer, grown by the unit `SV_LinkEntity` adds.
pub(super) fn linked_box(peer: &Peer) -> ([f32; 3], [f32; 3]) {
    let (mut bottom, top) = peer.movement.box_bounds();
    let top = match peer.corpse {
        Some((_, corpse_top)) => [top[0], top[1], corpse_top],
        None => top,
    };
    bottom = bottom.map(|axis| axis - 1.0);
    let origin = peer.state.origin();
    (
        std::array::from_fn(|axis| origin[axis] + bottom[axis]),
        std::array::from_fn(|axis| origin[axis] + top[axis] + 1.0),
    )
}

/// Player `number`'s state remembered as a Force power first reaches it in an update, into
/// scratch kept between updates, to tell afterwards whether the power changed it.
pub(super) fn remember_touched(
    touched: &mut Vec<u16>,
    before: &mut Vec<PlayerState>,
    number: u16,
    state: &PlayerState,
) {
    if touched.contains(&number) {
        return;
    }
    let at = touched.len();
    touched.push(number);
    match before.get_mut(at) {
        Some(before) => before.copy_from(state),
        None => before.push(state.clone()),
    }
}

/// `EntitiesInBox`' players for a push or pull (`w_force.c:3214-3260`): the begun, playing
/// peers whose linked box meets `mins`..`maxs`, appended to `out` in place order.
pub(super) fn player_candidates(
    world: &ServerWorld<(), Peer>,
    peers: &crate::players::PlayerRoster,
    mins: [f32; 3],
    maxs: [f32; 3],
    out: &mut Vec<Candidate>,
) {
    for number in 0..peers.places() as u16 {
        let Some(peer) = peers
            .at(usize::from(number))
            .and_then(|handle| world.entity(handle))
            .filter(|peer| peer.begun && peer.body_active())
        else {
            continue;
        };
        let (absmin, absmax) = linked_box(peer);
        if overlaps(absmin, absmax, mins, maxs) {
            out.push(Candidate {
                number,
                kind: CandidateKind::Player,
                absmin,
                absmax,
                origin: peer.state.origin(),
            });
        }
    }
}

/// `EntitiesInBox`' missiles for a push or pull: the linked ones whose box, grown by the
/// unit `SV_LinkEntity` adds, meets `mins`..`maxs`, appended to `out`.
pub(super) fn missile_candidates(
    missiles: &[(EntityId, Missile)],
    mins: [f32; 3],
    maxs: [f32; 3],
    out: &mut Vec<Candidate>,
) {
    for (number, missile) in missiles.iter().filter(|(_, missile)| missile.linked) {
        let absmin: [f32; 3] =
            std::array::from_fn(|axis| missile.current[axis] + missile.bounds.0[axis] - 1.0);
        let absmax: [f32; 3] =
            std::array::from_fn(|axis| missile.current[axis] + missile.bounds.1[axis] + 1.0);
        if overlaps(absmin, absmax, mins, maxs) {
            let read = |index: usize| missile.state.raw_field(index).unwrap_or(0);
            let kind = CandidateKind::Missile {
                trajectory: read(ES_POS_TYPE),
                stuck: read(ES_EFLAGS) & EF_MISSILE_STICK != 0,
                weapon: read(ES_WEAPON),
            };
            out.push(Candidate {
                number: number.legacy_number(),
                kind,
                absmin,
                absmax,
                origin: ES_POS_BASE.map(|index| f32::from_bits(read(index))),
            });
        }
    }
}

/// `EntitiesInBox`' items for a push or pull that flings them (JA+'s `jp_pushAll`): each
/// one's box where it is, grown by the unit `SV_LinkEntity` adds, that meets
/// `mins`..`maxs`, appended to `out`.
fn item_candidates(
    items: &[(EntityId, Pickup)],
    mins: [f32; 3],
    maxs: [f32; 3],
    out: &mut Vec<Candidate>,
) {
    for (number, pickup) in items {
        let (low, high) = pickup.bounds;
        let absmin: [f32; 3] = std::array::from_fn(|axis| pickup.origin[axis] + low[axis] - 1.0);
        let absmax: [f32; 3] = std::array::from_fn(|axis| pickup.origin[axis] + high[axis] + 1.0);
        if overlaps(absmin, absmax, mins, maxs) {
            out.push(Candidate {
                number: number.legacy_number(),
                kind: CandidateKind::Item,
                absmin,
                absmax,
                origin: pickup.origin,
            });
        }
    }
}

/// Whether the box `absmin`..`absmax` meets `mins`..`maxs`.
fn overlaps(absmin: [f32; 3], absmax: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> bool {
    (0..3).all(|axis| absmin[axis] <= maxs[axis] && absmax[axis] >= mins[axis])
}

/// `G_ReflectMissile(thrower, missile, forward)` ([`sjk_game_jka::saber_block::reflect_missile`]):
/// the missile numbered `missile` sent back by `thrower` (its number and origin), towards
/// its shooter where it stands (`shooter_origin`; the missile's own place where unknown).
pub(super) fn reflect_from(
    missiles: &mut [(EntityId, Missile)],
    crt: &mut CrtRand,
    missile: u16,
    thrower: (u16, [f32; 3]),
    forward: [f32; 3],
    shooter_origin: Option<[f32; 3]>,
    level_time: i32,
) {
    let Some((_, missile)) = missiles
        .iter_mut()
        .find(|(number, _)| number.legacy_number() == missile)
    else {
        return;
    };
    let shooter_origin = shooter_origin.unwrap_or(missile.current);
    sjk_game_jka::saber_block::reflect_missile(
        thrower.0,
        thrower.1,
        missile,
        forward,
        shooter_origin,
        level_time,
        crt,
    );
}

/// `TossClientWeapon` on `peer` (a pull's, `w_force.c:3466-3476`): its weapon out of its
/// hand — the `EV_NOAMMO` it raises flagged on its entity — and the pickup it becomes, for
/// the caller to spawn.
pub(super) fn toss_weapon_of(
    peer: &mut Peer,
    direction: [f32; 3],
    speed: f32,
    gametype: i32,
    level_time: i32,
) -> Option<Pickup> {
    let event_before = peer.state.raw_field(PS_EXTERNAL_EVENT);
    let dropped = sjk_game_jka::dropped_items::toss_client_weapon(
        &mut peer.state,
        peer.entity.state_mut(),
        direction,
        speed,
        gametype,
        level_time,
    );
    if peer.state.raw_field(PS_EXTERNAL_EVENT) != event_before {
        peer.entity.event_raised(level_time);
    }
    dropped
}
