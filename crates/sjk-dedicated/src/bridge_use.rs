//! The use key on this server (`ClientThink_real`, `codemp/game/g_active.c:3368-3372`, and
//! `TryUse`, `g_utils.c:1594-1760`): while the key is held, every 100 ms (`ps.useDelay`),
//! a player not busy reaches for what is in front of its eyes — or, on a vehicle, gets off
//! it. A vehicle it reaches it boards; a button is pressed, a `func_usable` fires, a siege
//! objective is taken. Reaching nothing, it switches a held jetpack on in the air
//! (`tryJetPack`), and while the jetpack burns it only switches it off.

use super::*;
use sjk_game_jka::npc_spawn::NpcHost;
use sjk_game_jka::vehicle_drive::VehicleUse;

/// `BUTTON_USE`.
const BUTTON_USE: u16 = sjk_game_jka::use_key::BUTTON_USE;
/// `BOTH_BUTTON_HOLD`, `BOTH_CONSOLE1`: the poses a weapon's time does not stop using in.
const BOTH_BUTTON_HOLD: u16 = 1_328;
const BOTH_CONSOLE1: u16 = 954;
/// `HANDEXTEND_NONE`, `HANDEXTEND_DRAGGING`.
const HANDEXTEND_NONE: u8 = 0;
const HANDEXTEND_DRAGGING: u8 = 15;
/// `PMF_FOLLOW`.
const PMF_FOLLOW: u16 = 4_096;
/// `s.eFlags`' entity field (`msg.cpp`).
const USABLE_EFLAGS: usize = 19;
/// `ENTITYNUM_NONE`: the ground entity of a player in the air.
const ENTITYNUM_NONE: u16 = 1_023;

impl NativeGame {
    /// The `func_usable` brushes a map places, as the entities the use key reaches for,
    /// each at its origin with its `model2` registered (`G_ModelIndex`) for every client
    /// to draw. One that starts off is spawned all the same but carries nothing a client
    /// draws or a trace meets, and is not sent: `SVF_NOCLIENT` and `EF_NODRAW`.
    pub(super) fn spawn_usables(&mut self) {
        let Some(map) = self.map.take() else { return };
        for usable in &map.usables {
            let (models, told) = (&mut self.models, &mut self.told);
            let model2 = if usable.model2.is_empty() {
                0
            } else {
                models.index(usable.model2.as_bytes(), &mut |index, value| {
                    told.push(Told::ConfigString {
                        index,
                        previous: Vec::new(),
                        value: value.to_vec(),
                    })
                })
            };
            let mut state = EntityState::zero(0, &sjk_protocol::LEGACY_ENTITY_FIELDS);
            usable.project(&mut state, model2);
            if let Some(number) = self.pool.spawn_entity(state, 0) {
                self.pool.set_bounds(number, usable.bounds);
                self.pool.show(number, usable.present);
                self.usable_entities.push((number, usable.clone()));
            }
        }
        self.map = Some(map);
    }

    /// `ClientThink_real`'s use key: held, `TryUse` every 100 ms.
    pub(super) fn try_use(&mut self, client: usize, level_time: i32) {
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if peer.last_command.buttons & BUTTON_USE == 0 || peer.riding.use_delay >= level_time {
            return;
        }
        self.use_now(client, level_time);
        if let Some(peer) = self.peer_mut(client) {
            peer.riding.use_delay = level_time + 100;
        }
    }

    /// `TryUse` (`g_utils.c:1594-1760`): in siege nothing is used before the round
    /// begins.
    fn use_now(&mut self, client: usize, level_time: i32) {
        if self.gametype == GAMETYPE_SIEGE
            && !self.siege.as_ref().is_some_and(|siege| siege.round.begun)
        {
            return;
        }
        // A map E-Web's gunner lets go of it with the key (`bridge_eweb`).
        if self.leave_map_eweb_by_use(client, level_time) {
            return;
        }
        let Some(peer) = self.peer(client) else {
            return;
        };
        let state = &peer.state;
        let torso = state.torso_animation();
        let busy = state.weapon_time() > 0 && torso != BOTH_BUTTON_HOLD && torso != BOTH_CONSOLE1;
        let hands = state.force_hand_extend();
        if busy
            || peer.health < 1
            || !peer.playing()
            || state.movement_flags() & PMF_FOLLOW != 0
            || !matches!(hands, HANDEXTEND_NONE | HANDEXTEND_DRAGGING)
            || state.emplaced_index() != 0
        {
            return;
        }
        if state.vehicle_entity_num() != 0 {
            // "on an emplaced gun or using a vehicle": off it, unless still boarding.
            let _ = self.with_rider(client, |roster, rider, host| {
                let bodies = Self::bodies_but_owned(roster, rider.number);
                let owner = rider.number;
                let mut trace = |start, mins, maxs, end, mask| {
                    host.trace(start, mins, maxs, end, owner, mask, &bodies)
                };
                roster.use_while_riding(rider, level_time, &mut trace)
            });
            return;
        }
        // A switched-on jetpack takes the key before anything in front of the eyes.
        if self
            .peer(client)
            .is_some_and(|peer| peer.gear.packs.jetpack_on)
        {
            self.try_jetpack(client, level_time);
            return;
        }
        let Some(found) = self.use_target(client) else {
            self.try_jetpack(client, level_time);
            return;
        };
        if self.use_vehicle(client, found, level_time)
            || self.use_gun(client, found, level_time)
            || self.use_map_eweb(client, found, level_time)
        {
            return;
        }
        // A button the use key reached is pressed, and a `PLAYER_USE` door used
        // (`ValidUseTarget`, `GlobalUse` to `Use_BinaryMover`) — in siege not by the team
        // it lets through (`TryUse`'s `alliedTeam` rule, unless `g_ff_objectives`).
        let team = self.peer(client).map_or(0, |peer| peer.session.team);
        let objectives = self.integer_cvar(b"g_ff_objectives", 0) != 0;
        let siege = self.gametype == GAMETYPE_SIEGE;
        if let Some(index) = self.doors.iter().position(|(number, door)| {
            number.legacy_number() == found
                && match door.kind {
                    sjk_game_jka::movers::MoverKind::Button => true,
                    sjk_game_jka::movers::MoverKind::Door => {
                        sjk_game_jka::movers::usable_by(door, siege, team, objectives)
                    }
                    sjk_game_jka::movers::MoverKind::Plat => false,
                }
        }) {
            self.use_door_at(index, client, level_time);
            return;
        }
        // A `func_usable` the map made `PLAYER_USE` is used (`ValidUseTarget`,
        // `func_usable_use`).
        if let Some(index) = self
            .usable_entities
            .iter()
            .position(|(number, usable)| number.legacy_number() == found && usable.answers_use())
        {
            self.use_usable(index, client, level_time);
            return;
        }
        self.try_jetpack(client, level_time);
    }

    /// `tryJetPack` (`g_utils.c:2040-2048`): the use key, having used nothing else,
    /// switches a held jetpack on in the air, or off while it burns. Stock flies only
    /// this way: `G_ItemUsable` refuses the jetpack's own `use_jetpack`.
    fn try_jetpack(&mut self, client: usize, level_time: i32) {
        let Some(peer) = self.peer(client) else {
            return;
        };
        let held =
            peer.state.stats[STAT_HOLDABLE_ITEMS] & (1 << sjk_game_jka::holdables::HI_JETPACK) != 0;
        let airborne = peer.state.ground_entity_num() == ENTITYNUM_NONE;
        if sjk_game_jka::holdables::try_jetpack(held, peer.gear.packs.jetpack_on, airborne) {
            self.use_jetpack(client, level_time);
        }
    }

    /// `func_usable_use` on the usable at `index` of the map's list: it toggles (or, when
    /// `ALWAYS_ON`, rests), clients are sent what it now is, and it fires what it fires.
    fn use_usable(&mut self, index: usize, client: usize, level_time: i32) {
        let (number, usable) = &mut self.usable_entities[index];
        let number = *number;
        let fired = usable.used(level_time);
        let (present, eflags) = (usable.present, usable.eflags);
        if let Some(state) = self.pool.state_mut(number) {
            state.set_raw_field(USABLE_EFLAGS, eflags);
        }
        self.pool.show(number, present);
        self.fire_targets(&fired, client, level_time);
    }

    /// `G_UseTargets` reaching the `func_usable`s called `name`: each is used in turn, but
    /// one the map made `INACTIVE` (`GlobalUse`).
    pub(super) fn use_usables(&mut self, name: &str, client: usize, level_time: i32) {
        for index in 0..self.usable_entities.len() {
            let usable = &self.usable_entities[index].1;
            if usable.targetname.eq_ignore_ascii_case(name) && !usable.inactive() {
                self.use_usable(index, client, level_time);
            }
        }
    }

    /// `func_usable_think`: the `ALWAYS_ON` usables whose `wait` is over answer again.
    pub(super) fn run_usables(&mut self, level_time: i32) {
        for (_, usable) in &mut self.usable_entities {
            usable.wake(level_time);
        }
    }

    /// `TryUse`'s vehicle (`g_utils.c:1699-1723`): the one the player rides it gets off, any
    /// other it boards (its allied team's alone in a team game); either way the press is
    /// spent. Whether `found` is a vehicle.
    fn use_vehicle(&mut self, client: usize, found: u16, level_time: i32) -> bool {
        let gametype = self.gametype;
        // `ent->client->sess.sessionTeam`, which a vehicle with an allied team checks.
        let team = self.peer(client).map_or(0, |peer| peer.session.team);
        let result = self.with_rider(client, |roster, rider, host| {
            let zoomed = rider.player.zoom_mode() != 0;
            let wanted = roster.vehicle_use(found, rider, zoomed, gametype, team);
            match wanted {
                VehicleUse::NotOne => return false,
                VehicleUse::GetOff => {
                    let bodies = Self::bodies_but_owned(roster, rider.number);
                    let owner = rider.number;
                    let mut trace = |start, mins, maxs, end, mask| {
                        host.trace(start, mins, maxs, end, owner, mask, &bodies)
                    };
                    roster.eject(found, rider, false, level_time, &mut trace);
                }
                VehicleUse::GetOn => {
                    roster.board(found, rider, level_time, host);
                }
                VehicleUse::Refused => {}
            }
            rider.command.buttons &= !BUTTON_USE;
            true
        });
        result.unwrap_or(false)
    }

    /// The NPCs' bodies a rider's traces meet: all but the vehicle it owns.
    fn bodies_but_owned(
        roster: &sjk_game_jka::npc_roster::NpcRoster,
        rider: u16,
    ) -> Vec<BoxObstacle> {
        roster
            .actors
            .iter()
            .filter(|npc| npc.contents != 0 && sjk_game_jka::vehicle_board::owner_of(npc) != rider)
            .map(|npc| npc.body())
            .collect()
    }

    /// `TryUse`'s trace (`g_utils.c:1661-1675`): what the player is looking at within reach,
    /// if anything.
    fn use_target(&mut self, client: usize) -> Option<u16> {
        let (eyes, angles) = {
            let peer = self.peer_mut(client)?;
            let origin = peer.state.origin();
            let height = peer.state.view_height() as f32;
            (
                [origin[0], origin[1], origin[2] + height],
                peer.state.view_angles(),
            )
        };
        let (from, to) = sjk_game_jka::use_key::reach(eyes, angles);
        self.gather_obstacles(client);
        let Self { map, obstacles, .. } = self;
        let map = map.as_ref()?;
        let world = WorldCollision {
            bsp: &map.bsp,
            scratch: &map.scratch,
        };
        let trace = WithPlayers {
            world,
            players: obstacles,
        }
        .trace(
            from,
            [0.0; 3],
            [0.0; 3],
            to,
            sjk_game_jka::use_key::USE_MASK,
        );
        // Nothing within reach, or the world itself.
        if trace.fraction == 1.0 || trace.entity_number >= 1_022 {
            return None;
        }
        Some(trace.entity_number)
    }
}

#[path = "bridge_emplaced.rs"]
pub(super) mod bridge_emplaced;
