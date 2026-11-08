//! Force push and pull (`ForceThrow`, `w_force.c:2914-3677`): the thrower's hand, sound,
//! cost and powerups; who is reached — at level 1 the one aimed at, at 2 and 3 anyone
//! within the arc (60 or 180 degrees) — in front, within 1024 units, in the thrower's
//! PVS and in its sight; and what it does to each: a player is shoved along the line
//! between them (less the further it is, at least 16), knocked down at level 3 up close,
//! may push back if it has the power and stands (`CanCounterThrow`, a level less if it
//! moves), may lose the weapon in its hand to a pull, and absorption takes some of it
//! (`WP_AbsorbConversion`); a flying missile is pushed back at whoever fired it
//! (`G_ReflectMissile`).
//!
//! The update's Force button calls for the throw ([`crate::force_powers::Begun::throw`]);
//! the caller runs [`throw`] with a [`ThrowWorld`] that reaches every player and entity,
//! then finishes the update.
//!
//! A push or pull on the one gripping the thrower breaks its grip at the grip's level or
//! above.
//!
//! The server's mod may change some of it ([`ForceRules`], [`Throw::rules`]): JA+ throws
//! players on the ground and, with friendly fire, teammates, and flings items
//! ([`crate::item_push`]).
//!
//! NPCs are clients too: they throw and are thrown alike (an NPC's world is
//! [`crate::npc_force_throw`]'s), but a vehicle, and any NPC in siege, is out of reach.
//!
//! A door the map made Force-usable (`MOVER_FORCE_ACTIVATE`) is pushed away or pulled near
//! when it is aimed at, and a Force-pushable button is pressed (`w_force.c:3584-3663`).
//!
//! Not yet: `func_static`s pushed or pulled, thrown sabers and limbs, a push breaking a
//! saber lock, and riders thrown off their vehicles.

use crate::event_entity::EventEntity;
use crate::force_powers::{FORCE_POWER_NEEDED, FP_ABSORB, FP_GRIP, FP_PULL, FP_PUSH, ForcePowers};
use crate::force_rules::ForceRules;
use crate::knockdown::Knockdown;
use crate::player_death::Rng;
use crate::pmove::MovementTrace;
use sjk_protocol::{PlayerState, UserCommand};

/// A client (a player or an NPC) the throw reads and changes: its wire state, its Force,
/// the game's health, its knockdown memory, who last shoved it, its class if it is an NPC,
/// the last command, and the push effect's end.
pub struct ThrowPlayer<'a> {
    pub state: &'a mut PlayerState,
    pub force: &'a mut ForcePowers,
    pub health: i32,
    pub knockdown: &'a mut Knockdown,
    /// Who last pushed or held it (`ps.otherKiller`), which a shove sets.
    pub other_killer: &'a mut crate::damage::OtherKiller,
    /// An NPC's class (`NPC_class`); `None` for a player.
    pub npc_class: Option<i32>,
    pub invulnerable_until: &'a mut i32,
    /// `pers.cmd`.
    pub command: UserCommand,
    /// `sess.sessionTeam`.
    pub team: i32,
    /// `pushEffectTime`: until when `EF_BODYPUSH` shows.
    pub push_effect_until: &'a mut i32,
    /// `ps.saberLockHits`, which a push in a saber lock adds to.
    pub lock_hits: &'a mut i32,
}

/// An entity `EntitiesInBox` found, as the throw reads it.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub number: u16,
    pub kind: CandidateKind,
    /// `r.absmin`, `r.absmax`.
    pub absmin: [f32; 3],
    pub absmax: [f32; 3],
    /// A player's `ps.origin`; anything else's `s.pos.trBase`.
    pub origin: [f32; 3],
}

/// What a candidate is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateKind {
    Player,
    /// A missile: its trajectory type, `EF_MISSILE_STICK`, its weapon.
    Missile {
        trajectory: u32,
        stuck: bool,
        weapon: u32,
    },
    /// A `func_door` the Force opens (`MOVER_FORCE_ACTIVATE`): reached only at rest.
    ForceDoor {
        at_rest: bool,
    },
    /// A `func_button`: a push presses it where the map made it Force-pushable
    /// (`SPF_BUTTON_FPUSHABLE`); a pull never does.
    Button {
        pushable: bool,
    },
    /// An item (`ET_ITEM`), which a server that pushes items flings
    /// ([`ForceRules::push_items`], [`crate::item_push`]).
    Item,
    /// Anything else the throw leaves alone (the world, other movers).
    Other,
}

/// A mover a throw reached, as the throw reads it (`moverState`, `pos1`, `pos2`,
/// `r.currentOrigin`, and whether `s.origin` is the world's origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoverView {
    pub state: crate::movers::MoverState,
    pub pos1: [f32; 3],
    pub pos2: [f32; 3],
    /// `spawnflags & 1`: it started open (`MOVER_START_OPEN`).
    pub start_open: bool,
    /// Made with an origin brush: `pos1` and `pos2` are where it stands, not offsets.
    pub origin_brush: bool,
}

/// Everything beyond the thrower that a throw reaches.
pub trait ThrowWorld {
    /// `trap->EntitiesInBox`, in entity number order.
    fn entities_in_box(&mut self, mins: [f32; 3], maxs: [f32; 3], out: &mut Vec<Candidate>);
    /// A point trace (`trap->Trace` with no box) past `pass`.
    fn trace(&mut self, start: [f32; 3], end: [f32; 3], pass: u16, mask: u32) -> MovementTrace;
    /// `trap->InPVS`.
    fn in_pvs(&mut self, from: [f32; 3], to: [f32; 3]) -> bool;
    /// A player, one at a time.
    fn player(&mut self, number: u16) -> Option<ThrowPlayer<'_>>;
    /// `G_ReflectMissile(thrower, missile, forward)`.
    fn reflect_missile(&mut self, missile: u16, thrower: u16, forward: [f32; 3]);
    /// `TossClientWeapon(victim, direction, speed)`.
    fn toss_weapon(&mut self, victim: u16, direction: [f32; 3], speed: f32);
    /// A temp entity raised now.
    fn raise(&mut self, event: EventEntity);
    /// `G_SoundIndex`.
    fn sound_index(&mut self, name: &[u8]) -> u16;
    /// The game's generator (`Q_irand`).
    fn rng(&mut self) -> &mut Rng;
    /// The mover numbered `number`, for a door the throw reached.
    fn mover(&mut self, _number: u16) -> Option<MoverView> {
        None
    }
    /// `GEntity_UseFunc(mover, thrower, thrower)`: the mover is used (`Use_BinaryMover`).
    fn use_mover(&mut self, _number: u16, _thrower: u16) {}
    /// The item numbered `number`, for a server that pushes items.
    fn item(&mut self, _number: u16) -> Option<&mut crate::items::Pickup> {
        None
    }
}

const RADIUS: f32 = 1_024.0;
const MASK_PLAYERSOLID: u32 = 0x1 | 0x10 | 0x100 | 0x1000;
const MASK_SHOT: u32 = 0x1 | 0x100 | 0x200 | 0x1000;
const ENTITY_NUMBER_NONE: u16 = 1_023;
const HANDEXTEND_NONE: u32 = 0;
const HANDEXTEND_FORCEPUSH: u32 = 1;
const HANDEXTEND_FORCEPULL: u32 = 2;
const HANDEXTEND_KNOCKDOWN: u32 = 8;
const PW_PULL: usize = 3;
const PW_DISINT_4: usize = 9;
const CHAN_VOICE: u32 = 3;
const CHAN_BODY: u32 = 6;
const TR_STATIONARY: u32 = 0;
const TR_INTERPOLATE: u32 = 1;
const WP_THERMAL: u32 = 12;
const WEAPON_CHARGING: u32 = 4;
const WEAPON_CHARGING_ALT: u32 = 5;
const EF_INVULNERABLE: u32 = 1 << 27;
const PDSOUND_ABSORBHIT: u32 = 3;
const GT_SIEGE: i32 = 7;
const PS_VELOCITY: [usize; 3] = [6, 7, 8];
const PS_WEAPON_TIME: usize = 10;
const PS_LEGS_ANIM: usize = 13;
const PS_TORSO_ANIM: usize = 15;
const PS_GROUND_ENTITY: usize = 16;
const PS_EFLAGS: usize = 17;
const PS_FORCE_POWER: usize = 18;
const PS_VIEW_HEIGHT: usize = 22;
const PS_ROCKET_LOCK_INDEX: usize = 24;
const PS_WEAPON_STATE: usize = 33;
const PS_ACTIVE: usize = 82;
const PS_ROCKET_TARGET_TIME: usize = 71;
const PS_ROCKET_LOCK_TIME: usize = 79;
const PS_FORCE_HAND_EXTEND: usize = 80;
const PS_FORCE_DODGE_ANIM: usize = 89;
const ES_SABER_ENTITY: usize = 37;
const ES_TRICKED: usize = 58;
const EV_GENERAL_SOUND: u32 = 76;

/// The throw's context: the time, the game type, the thrower's number, the server's
/// Force rules.
#[derive(Clone, Copy, Debug)]
pub struct Throw {
    pub level_time: i32,
    pub gametype: i32,
    pub thrower: u16,
    pub pull: bool,
    pub rules: ForceRules,
    /// The players in a JA+ full-force duel ([`crate::duel::FullForce`]): none on the
    /// base game.
    pub full_force: crate::duel::FullForce,
}

/// What the thrower brings to every target, read once.
struct Thrower {
    origin: [f32; 3],
    eye: [f32; 3],
    view: [f32; 3],
    forward: [f32; 3],
    level: usize,
    cost: i32,
    team: i32,
}

/// `ForceThrow(self, pull)`, called by the thrower's Force button. The thrower's own
/// state changes first (the cost, the hand, the sound, the powerups); then each target
/// in turn; then the thrower's invulnerability ends.
pub fn throw(context: Throw, world: &mut dyn ThrowWorld) {
    let Some(thrower) = begin(context, world) else {
        return;
    };
    let mut candidates = Vec::new();
    // At level 1 an aim that strikes nothing, or a player the power cannot reach, ends
    // the throw there: the thrower's protection and danger time are left as they were.
    let Some(listed) = targets(context, &thrower, world, &mut candidates) else {
        return;
    };
    let mut reached = Vec::new();
    for candidate in listed {
        if let Some(candidate) = in_reach(context, &thrower, candidate, world) {
            reached.push(candidate);
        }
    }
    for candidate in reached {
        match candidate.kind {
            CandidateKind::Player => shove(context, &thrower, candidate, world),
            CandidateKind::Missile {
                trajectory, weapon, ..
            } => {
                let rolling_thermal = trajectory == TR_INTERPOLATE && weapon == WP_THERMAL;
                if trajectory != TR_STATIONARY && !rolling_thermal && !context.pull {
                    world.reflect_missile(candidate.number, context.thrower, thrower.forward);
                }
            }
            CandidateKind::ForceDoor { .. } => force_door(context, &thrower, candidate, world),
            CandidateKind::Button { .. } => {
                // `Touch_Button`: only a button at rest goes in.
                if world
                    .mover(candidate.number)
                    .is_some_and(|button| button.state == crate::movers::MoverState::Pos1)
                {
                    world.use_mover(candidate.number, context.thrower);
                }
            }
            CandidateKind::Item => {
                let in_duel = world
                    .player(context.thrower)
                    .is_some_and(|player| player.state.duel_in_progress());
                if let Some(item) = world.item(candidate.number)
                    && crate::item_push::pushable(item, context.rules.push_items, in_duel)
                {
                    crate::item_push::push(item, thrower.forward, context.pull, context.level_time);
                }
            }
            CandidateKind::Other => {}
        }
    }
    // The pusher's spawn protection is over; the grip on it ends.
    if let Some(player) = world.player(context.thrower) {
        let flags = player.state.raw_field(PS_EFLAGS).unwrap_or(0);
        player
            .state
            .set_raw_field(PS_EFLAGS, flags & !EF_INVULNERABLE);
        *player.invulnerable_until = 0;
        player.force.danger_time = context.level_time;
        if player.force.grip_being_gripped > context.level_time as f32 {
            player.force.grip_being_gripped = 0.0;
        }
    }
}

/// The thrower's part before anyone is reached (`w_force.c:2941-3068`): whether it may
/// throw at all, and then its cost, its hand, its sound and its powerups.
fn begin(context: Throw, world: &mut dyn ThrowWorld) -> Option<Thrower> {
    let level_time = context.level_time;
    let power = if context.pull { FP_PULL } else { FP_PUSH };
    let player = world.player(context.thrower)?;
    let state = player.state;
    let field = |state: &PlayerState, index: usize| state.raw_field(index).unwrap_or(0);
    let hand = field(state, PS_FORCE_HAND_EXTEND);
    if hand != HANDEXTEND_NONE && (hand != HANDEXTEND_KNOCKDOWN || !in_get_up(state)) {
        return None;
    }
    // `g_useWhileThrowing` is 1: a thrown saber does not keep its thrower from throwing.
    if field(state, PS_WEAPON_TIME) as i32 > 0
        || player.health <= 0
        || state.powerups[PW_DISINT_4] as i32 > level_time
    {
        return None;
    }
    if !crate::force_powers::usable(
        state,
        player.force,
        player.health,
        power,
        level_time,
        context.gametype,
        context.full_force,
    ) {
        return None;
    }
    // `G_SoundIndex` once the thrower may throw (`w_force.c:2984`, `3056`).
    let sound = world.sound_index(if context.pull {
        b"sound/weapons/force/pull.wav"
    } else {
        b"sound/weapons/force/push.wav"
    });
    let player = world.player(context.thrower)?;
    let state = player.state;
    // `BG_ClearRocketLock`.
    state.set_raw_field(PS_ROCKET_LOCK_INDEX, u32::from(ENTITY_NUMBER_NONE));
    state.set_raw_field(PS_ROCKET_LOCK_TIME, (-1.0f32).to_bits());
    state.set_raw_field(PS_ROCKET_TARGET_TIME, 0);
    // A push in a saber lock (`w_force.c:2982-2991`) pushes the lock: its level twice
    // over in hits, and nothing else is thrown.
    const PS_SABER_LOCK_TIME: usize = 107;
    const PS_SABER_LOCK_FRAME: usize = 108;
    if !context.pull
        && field(state, PS_SABER_LOCK_TIME) as i32 > level_time
        && field(state, PS_SABER_LOCK_FRAME) != 0
    {
        let mut sound_event = EventEntity {
            event: EV_GENERAL_SOUND,
            parameter: u32::from(sound),
            origin: state.origin(),
            client: None,
            broadcast: false,
            extra: [(0, 0); 12],
        };
        sound_event.extra[0] = (ES_SABER_ENTITY, CHAN_BODY);
        state.powerups[PW_DISINT_4] = (level_time + 1_500) as u32;
        *player.lock_hits += i32::from(player.force.levels[FP_PUSH]) * 2;
        start(state, player.force, power, level_time);
        world.raise(sound_event);
        return None;
    }
    start(state, player.force, power, level_time);
    let origin = state.origin();
    let mut sound_event = EventEntity {
        event: EV_GENERAL_SOUND,
        parameter: u32::from(sound),
        origin,
        client: None,
        broadcast: false,
        extra: [(0, 0); 12],
    };
    sound_event.extra[0] = (ES_SABER_ENTITY, CHAN_BODY);
    if context.pull {
        if hand == HANDEXTEND_NONE {
            state.set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_FORCEPULL);
            let hold = if context.gametype == GT_SIEGE && field(state, 47) == 3 {
                200
            } else {
                400
            };
            player.knockdown.hand_extend_time = level_time + hold;
        }
        state.powerups[PW_DISINT_4] = (player.knockdown.hand_extend_time + 200) as u32;
        state.powerups[PW_PULL] = state.powerups[PW_DISINT_4];
    } else {
        if hand == HANDEXTEND_NONE {
            state.set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_FORCEPUSH);
            player.knockdown.hand_extend_time = level_time + 1_000;
        } else if hand == HANDEXTEND_KNOCKDOWN && in_get_up(state) {
            // Pushed from the ground: the push on the upper body, the get-up on the legs.
            let mut dodge = field(state, PS_FORCE_DODGE_ANIM);
            if dodge > 4 {
                dodge -= 8;
            }
            state.set_raw_field(PS_FORCE_DODGE_ANIM, dodge + 8);
        }
        state.powerups[PW_DISINT_4] = (level_time + 1_100) as u32;
        state.powerups[PW_PULL] = 0;
    }
    let view = state.view_angles();
    let (forward, _) = crate::pmove::flight::flight_axes(view);
    let forward = forward.to_array();
    let level = usize::from(player.force.levels[power]);
    let eye = [
        origin[0],
        origin[1],
        origin[2] + field(state, PS_VIEW_HEIGHT) as i32 as f32,
    ];
    let thrower = Thrower {
        origin,
        eye,
        view,
        forward,
        level,
        cost: FORCE_POWER_NEEDED[level][power],
        team: player.team,
    };
    world.raise(sound_event);
    (level != 0).then_some(thrower)
}

/// `WP_ForcePowerStart` for a throw: a full-body taunt cut short, no duration, the noise
/// the bots hear, the cost.
fn start(state: &mut PlayerState, force: &mut ForcePowers, power: usize, level_time: i32) {
    crate::force_powers::heard(force, power, level_time);
    for (animation, timer) in [(PS_LEGS_ANIM, 21), (PS_TORSO_ANIM, 20)] {
        if crate::pmove_input_freeze::full_body_taunt(state.raw_field(animation).unwrap_or(0) as u16)
        {
            state.set_raw_field(timer, 0);
        }
    }
    force.duration[power] = 0;
    force.debounce[power] = 0;
    crate::force_powers::drain(state, force, power, 0);
}

/// Who may be reached (`w_force.c:3108-3224`): at level 1 whatever the aim strikes within
/// 512 units — `None`, the throw given up, where it strikes nothing or a player out of the
/// power's reach; above, everything in the box of 1024 around the thrower but the players
/// outside the arc or out of the power's reach (`ForcePowerUsableOn`).
fn targets(
    context: Throw,
    thrower: &Thrower,
    world: &mut dyn ThrowWorld,
    candidates: &mut Vec<Candidate>,
) -> Option<Vec<Candidate>> {
    let power = if context.pull { FP_PULL } else { FP_PUSH };
    if thrower.level == 1 {
        let end: [f32; 3] =
            std::array::from_fn(|axis| thrower.eye[axis] + thrower.forward[axis] * RADIUS / 2.0);
        let hit = world.trace(thrower.eye, end, context.thrower, MASK_PLAYERSOLID);
        if hit.fraction == 1.0 || hit.entity_number == ENTITY_NUMBER_NONE {
            return None;
        }
        let mins = thrower.origin.map(|axis| axis - RADIUS);
        let maxs = thrower.origin.map(|axis| axis + RADIUS);
        world.entities_in_box(mins, maxs, candidates);
        let target = candidates
            .iter()
            .find(|candidate| candidate.number == hit.entity_number)
            .copied();
        let Some(target) = target else {
            return Some(Vec::new());
        };
        if target.kind == CandidateKind::Player && !usable_on(context, target.number, power, world)
        {
            return None;
        }
        return Some(vec![target]);
    }
    let arc = if thrower.level == 2 { 60.0 } else { 180.0 };
    let mins = thrower.origin.map(|axis| axis - RADIUS);
    let maxs = thrower.origin.map(|axis| axis + RADIUS);
    world.entities_in_box(mins, maxs, candidates);
    let mut listed = Vec::new();
    for candidate in candidates.iter().copied() {
        if candidate.kind == CandidateKind::Player {
            let towards: [f32; 3] =
                std::array::from_fn(|axis| candidate.origin[axis] - thrower.eye[axis]);
            let (pitch, yaw) = crate::damage::vector_to_angles(towards);
            if !in_field_of_vision(thrower.view, arc, [pitch, yaw])
                || !usable_on(context, candidate.number, power, world)
            {
                continue;
            }
        }
        listed.push(candidate);
    }
    Some(listed)
}

/// Whether a listed candidate is really reached (`w_force.c:3226-3322`): not the thrower
/// nor a teammate, a kind the throw moves, in front (within 53 degrees of the aim),
/// within reach of the box's nearest point, in the PVS, and in sight from the thrower's
/// feet or eyes.
fn in_reach(
    context: Throw,
    thrower: &Thrower,
    candidate: Candidate,
    world: &mut dyn ThrowWorld,
) -> Option<Candidate> {
    if candidate.number == context.thrower {
        return None;
    }
    match candidate.kind {
        CandidateKind::Player => {
            let team = world.player(candidate.number).map(|player| player.team)?;
            if on_same_team(context.gametype, thrower.team, team) && !context.rules.throw_teammates
            {
                return None;
            }
        }
        CandidateKind::Missile {
            trajectory,
            stuck,
            weapon,
        } => {
            if trajectory == TR_STATIONARY && (stuck || weapon != WP_THERMAL) {
                return None;
            }
        }
        CandidateKind::ForceDoor { at_rest } => {
            if !at_rest {
                return None;
            }
        }
        CandidateKind::Button { pushable } => {
            if context.pull || !pushable {
                return None;
            }
        }
        CandidateKind::Item => {
            if context.rules.push_items < 1 {
                return None;
            }
        }
        CandidateKind::Other => return None,
    }
    let centre = thrower.origin;
    let mut v = [0.0f32; 3];
    for axis in 0..3 {
        v[axis] = if centre[axis] < candidate.absmin[axis] {
            candidate.absmin[axis] - centre[axis]
        } else if centre[axis] > candidate.absmax[axis] {
            centre[axis] - candidate.absmax[axis]
        } else {
            0.0
        };
    }
    let size: [f32; 3] =
        std::array::from_fn(|axis| candidate.absmax[axis] - candidate.absmin[axis]);
    let middle: [f32; 3] = std::array::from_fn(|axis| candidate.absmin[axis] + 0.5 * size[axis]);
    let mut dir: [f32; 3] = std::array::from_fn(|axis| middle[axis] - centre[axis]);
    normalize(&mut dir);
    if dot(dir, thrower.forward) < 0.6 {
        return None;
    }
    if length(v) >= RADIUS {
        return None;
    }
    // A brush model is never ruled out by the PVS (`!ent->r.bmodel && !InPVS`).
    let brush = matches!(
        candidate.kind,
        CandidateKind::ForceDoor { .. } | CandidateKind::Button { .. }
    );
    if !brush && !world.in_pvs(middle, thrower.origin) {
        return None;
    }
    let sighted = |world: &mut dyn ThrowWorld, from: [f32; 3]| {
        let hit = world.trace(from, middle, context.thrower, MASK_SHOT);
        !(hit.fraction < 1.0 && hit.entity_number != candidate.number)
    };
    if !sighted(world, thrower.origin) && !sighted(world, thrower.eye) {
        return None;
    }
    Some(candidate)
}

/// A Force-usable door the throw reached (`w_force.c:3584-3658`): it must be aimed at
/// straight from the eyes, then a push moves it to whichever of its two places is further
/// from the thrower and a pull to the nearer; one already there is left alone.
fn force_door(context: Throw, thrower: &Thrower, candidate: Candidate, world: &mut dyn ThrowWorld) {
    let end: [f32; 3] =
        std::array::from_fn(|axis| thrower.eye[axis] + RADIUS * thrower.forward[axis]);
    let hit = world.trace(thrower.eye, end, context.thrower, MASK_SHOT);
    if hit.entity_number != candidate.number
        || hit.fraction == 1.0
        || hit.all_solid
        || hit.start_solid
    {
        return;
    }
    let Some(door) = world.mover(candidate.number) else {
        return;
    };
    let (pos1, pos2) = if door.origin_brush {
        (door.pos1, door.pos2)
    } else {
        // No origin brush: `pos1` and `pos2` are offsets, taken from where the middle of
        // the door started.
        use crate::movers::MoverState;
        let mut centre: [f32; 3] = std::array::from_fn(|axis| {
            candidate.absmin[axis] + 0.5 * (candidate.absmax[axis] - candidate.absmin[axis])
        });
        if door.start_open && door.state == MoverState::Pos1 {
            centre = std::array::from_fn(|axis| centre[axis] - door.pos1[axis]);
        } else if !door.start_open && door.state == MoverState::Pos2 {
            centre = std::array::from_fn(|axis| centre[axis] - door.pos2[axis]);
        }
        (
            std::array::from_fn(|axis| centre[axis] + door.pos1[axis]),
            std::array::from_fn(|axis| centre[axis] + door.pos2[axis]),
        )
    };
    let distance =
        |point: [f32; 3]| length(std::array::from_fn(|axis| point[axis] - thrower.eye[axis]));
    let pos1_closer = distance(pos1) < distance(pos2);
    let at = match door.state {
        crate::movers::MoverState::Pos1 => Some(pos1_closer),
        crate::movers::MoverState::Pos2 => Some(!pos1_closer),
        _ => None,
    };
    // `at`: whether it rests at its place nearer the thrower.
    match at {
        Some(true) if context.pull => return,
        Some(false) if !context.pull => return,
        _ => {}
    }
    world.use_mover(candidate.number, context.thrower);
}

/// What the throw does to a player it reached (`w_force.c:3326-3571`).
fn shove(context: Throw, thrower: &Thrower, candidate: Candidate, world: &mut dyn ThrowWorld) {
    let level_time = context.level_time;
    let power = if context.pull { FP_PULL } else { FP_PUSH };
    let mut events = Vec::new();
    let mod_level = {
        let Some(victim) = world.player(candidate.number) else {
            return;
        };
        match absorb_conversion(
            victim.state,
            victim.force,
            victim.state.client_num(),
            power,
            thrower.level as i32,
            thrower.cost,
            level_time,
            context.rules,
        ) {
            Some((level, event)) => {
                events.extend(event);
                level
            }
            None => thrower.level as i32,
        }
    };
    let push_sound = world.sound_index(if context.pull {
        b"sound/weapons/force/pull.wav"
    } else {
        b"sound/weapons/force/push.wav"
    });
    let push_power = 256 * mod_level;
    let mut can_pull_weapon = true;
    let mut dir_len = 0.0f32;
    let origin;
    let other_push;
    let mut push_mod = push_power;
    let mut toss = None;
    {
        let Some(victim) = world.player(candidate.number) else {
            return;
        };
        origin = victim.state.origin();
        let mut other = i32::from(victim.force.levels[power]);
        if victim.command.forward_move != 0 || victim.command.right_move != 0 {
            other = (other - 1).max(0);
        }
        other_push = other;
        if other != 0 && can_counter_throw(&victim, thrower, context, power) {
            let hand = if context.pull {
                HANDEXTEND_FORCEPULL
            } else {
                HANDEXTEND_FORCEPUSH
            };
            victim.state.set_raw_field(PS_FORCE_HAND_EXTEND, hand);
            victim.knockdown.hand_extend_time = level_time + if context.pull { 400 } else { 1_000 };
            victim.state.powerups[PW_DISINT_4] = (victim.knockdown.hand_extend_time + 200) as u32;
            victim.state.powerups[PW_PULL] = if context.pull {
                victim.state.powerups[PW_DISINT_4]
            } else {
                0
            };
            let mut sound = EventEntity {
                event: EV_GENERAL_SOUND,
                parameter: u32::from(push_sound),
                origin,
                client: None,
                broadcast: false,
                extra: [(0, 0); 12],
            };
            sound.extra[0] = (ES_SABER_ENTITY, CHAN_BODY);
            events.push(sound);
            if other >= mod_level {
                push_mod = 0;
                can_pull_weapon = false;
            } else {
                let difference = mod_level - other;
                let cut = match difference {
                    3.. => 0.2,
                    2 => 0.4,
                    _ => 0.8,
                };
                push_mod = (push_mod as f64 - f64::from(push_mod) * cut) as i32;
                push_mod = push_mod.max(0);
            }
        }
    }
    for event in events {
        world.raise(event);
    }
    let push_dir: [f32; 3] = if context.pull {
        let towards: [f32; 3] = std::array::from_fn(|axis| thrower.origin[axis] - origin[axis]);
        if length(towards) <= 256.0 {
            let chance = match mod_level {
                1 => 3,
                2 => 7,
                3 => 10,
                _ => 0,
            };
            let foe = world
                .player(candidate.number)
                .is_some_and(|victim| !on_same_team(context.gametype, thrower.team, victim.team));
            if foe && world.rng().irand(1, 10) <= chance && can_pull_weapon {
                let mut above = thrower.origin;
                above[2] += 64.0;
                let mut to_thrower: [f32; 3] =
                    std::array::from_fn(|axis| above[axis] - origin[axis]);
                normalize(&mut to_thrower);
                toss = Some(to_thrower);
            }
        }
        towards
    } else {
        std::array::from_fn(|axis| origin[axis] - thrower.origin[axis])
    };
    if let Some(direction) = toss {
        world.toss_weapon(candidate.number, direction, 500.0);
    }
    let Some(victim) = world.player(candidate.number) else {
        return;
    };
    let mut push_dir = push_dir;
    let vehicle = victim.state.raw_field(84).unwrap_or(0) != 0;
    if (mod_level > other_push || vehicle)
        && mod_level == 3
        && victim.state.raw_field(PS_FORCE_HAND_EXTEND) != Some(HANDEXTEND_KNOCKDOWN)
    {
        dir_len = length(push_dir);
        if crate::knockdown::knockdownable(victim.state)
            && dir_len <= (64 * ((mod_level - other_push) - 1)) as f32
        {
            victim
                .state
                .set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_KNOCKDOWN);
            victim.knockdown.hand_extend_time = level_time + 700;
            victim.state.set_raw_field(PS_FORCE_DODGE_ANIM, 0);
            victim.knockdown.quicker_getup = true;
        }
    }
    if dir_len == 0.0 {
        dir_len = length(push_dir);
    }
    normalize(&mut push_dir);
    break_grip(context, candidate.number, mod_level, world);
    let Some(victim) = world.player(candidate.number) else {
        return;
    };
    *victim.other_killer = crate::damage::OtherKiller::credit(context.thrower, level_time);
    // `pushPowerMod -= dirLen*0.7`: an int less a double product, truncated.
    push_mod = (f64::from(push_mod) - f64::from(dir_len) * 0.7) as i32;
    push_mod = push_mod.max(16);
    *victim.push_effect_until = level_time + 600;
    let mut velocity = victim.state.velocity();
    velocity[0] = push_dir[0] * push_mod as f32;
    velocity[1] = push_dir[1] * push_mod as f32;
    if velocity[2] as i32 == 0 {
        velocity[2] = (push_dir[2] * push_mod as f32).max(128.0);
    } else {
        velocity[2] = push_dir[2] * push_mod as f32;
    }
    for (axis, index) in PS_VELOCITY.into_iter().enumerate() {
        victim.state.set_raw_field(index, velocity[axis].to_bits());
    }
}

/// A thrower held in a grip breaks it by throwing the one who grips it, at the grip's
/// level or above (`w_force.c:3517-3529`): the gripper lets go (`WP_ForcePowerStop`) and
/// may grip again in a second, and the thrower is free.
fn break_grip(context: Throw, number: u16, level: i32, world: &mut dyn ThrowWorld) {
    let level_time = context.level_time;
    if !world
        .player(context.thrower)
        .is_some_and(|thrower| thrower.force.grip_being_gripped > level_time as f32)
    {
        return;
    }
    let Some(gripper) = world.player(number) else {
        return;
    };
    if gripper.force.grip_entity != context.thrower
        || level < i32::from(gripper.force.levels[FP_GRIP])
    {
        return;
    }
    let active = gripper.state.raw_field(PS_ACTIVE).unwrap_or(0);
    gripper
        .state
        .set_raw_field(PS_ACTIVE, active & !(1 << FP_GRIP));
    let (held, gasp) = crate::force_dark::let_go(
        gripper.state,
        gripper.force,
        &mut gripper.knockdown.hand_extend_time,
        active & (1 << FP_GRIP) != 0,
        level_time,
    );
    gripper.force.grip_use_time = level_time + 1_000;
    let Some(thrower) = world.player(held) else {
        return;
    };
    let gasps = crate::force_dark::released(thrower.force, thrower.health, gasp, level_time);
    thrower.force.grip_being_gripped = 0.0;
    let origin = thrower.state.origin();
    if gasps {
        let mut sound = crate::knockdown::entity_sound(origin, held, CHAN_VOICE);
        sound.parameter = u32::from(world.sound_index(b"*gasp.wav"));
        world.raise(sound);
    }
}

/// `WP_AbsorbConversion` (`w_force.c:791-848`) on player `number` for a push, pull,
/// grip, lightning or drain (`power`) at `level` that cost `spent`: with absorption
/// running, the power loses the victim's absorb levels and the victim's pool gains a
/// third of the cost for each (one at least; the server's [`ForceRules::absorb_gain`]),
/// with the absorbing hit's sound at most every 400 ms. Returns the power's level now and
/// that sound's event, or `None` where nothing is absorbed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn absorb_conversion(
    state: &mut PlayerState,
    force: &mut ForcePowers,
    number: u16,
    power: usize,
    level: i32,
    spent: i32,
    level_time: i32,
    rules: ForceRules,
) -> Option<(i32, Option<EventEntity>)> {
    let absorb = i32::from(force.levels[FP_ABSORB]);
    if absorb == 0 || state.raw_field(PS_ACTIVE).unwrap_or(0) & (1 << FP_ABSORB) == 0 {
        return None;
    }
    let continuous = !matches!(power, FP_PUSH | FP_PULL);
    let gain = rules.absorb_gain(spent, absorb, continuous);
    let pool = (state.raw_field(PS_FORCE_POWER).unwrap_or(0) as i32 + gain).min(force.max);
    state.set_raw_field(PS_FORCE_POWER, pool as u32);
    let event = (force.sound_debounce < level_time).then(|| {
        force.sound_debounce = level_time + 400;
        let mut event = crate::knockdown::predef_sound(state.origin(), PDSOUND_ABSORBHIT);
        event.extra[3] = (ES_TRICKED, u32::from(number));
        event
    });
    Some(((level - absorb).max(0), event))
}

/// `CanCounterThrow` (`w_force.c:2761-2829`): a victim with its hands and weapon free,
/// alive, not charging, on the ground, who could use the power itself, pushes back.
fn can_counter_throw(
    victim: &ThrowPlayer<'_>,
    thrower: &Thrower,
    context: Throw,
    power: usize,
) -> bool {
    let state = &*victim.state;
    let field = |index: usize| state.raw_field(index).unwrap_or(0);
    if field(PS_FORCE_HAND_EXTEND) != HANDEXTEND_NONE
        || field(PS_WEAPON_TIME) as i32 > 0
        || victim.health <= 0
    {
        return false;
    }
    if state.powerups[PW_DISINT_4] as i32 > context.level_time
        || matches!(
            field(PS_WEAPON_STATE),
            WEAPON_CHARGING | WEAPON_CHARGING_ALT
        )
    {
        return false;
    }
    if context.gametype == GT_SIEGE && context.pull {
        // A pulled player facing more than 60 degrees away cannot resist.
        let towards: [f32; 3] =
            std::array::from_fn(|axis| thrower.origin[axis] - state.origin()[axis]);
        let (_, yaw) = crate::damage::vector_to_angles(towards);
        let difference = angle_subtract(yaw, state.view_angles()[1]);
        if !(-60.0..=60.0).contains(&difference) {
            return false;
        }
    }
    if !crate::force_powers::usable(
        state,
        victim.force,
        victim.health,
        power,
        context.level_time,
        context.gametype,
        context.full_force,
    ) {
        return false;
    }
    field(PS_GROUND_ENTITY) != u32::from(ENTITY_NUMBER_NONE)
}

/// `ForcePowerUsableOn` for a player (`w_force.c:543-618`): not one with ysalamiri, nor
/// in a duel, nor — for a push or pull — knocked down.
fn usable_on(context: Throw, number: u16, _power: usize, world: &mut dyn ThrowWorld) -> bool {
    let Some(thrower) = world.player(context.thrower) else {
        return false;
    };
    if !crate::force_powers::can_use_now(
        thrower.state,
        if context.pull { FP_PULL } else { FP_PUSH },
        context.level_time,
        context.gametype,
        context.full_force,
    ) {
        return false;
    }
    let thrower_duel = crate::duel::duel_of(thrower.state);
    if thrower_duel.is_some() && !context.full_force.has(context.thrower) {
        return false;
    }
    let Some(other) = world.player(number) else {
        return false;
    };
    if crate::force_powers::has_ysalamiri(other.state, context.gametype)
        || crate::duel::force_refused(
            context.thrower,
            thrower_duel,
            number,
            crate::duel::duel_of(other.state),
            context.full_force,
        )
    {
        return false;
    }
    if !context.rules.throw_knocked_down
        && crate::knockdown::in_knockdown(other.state.raw_field(PS_LEGS_ANIM).unwrap_or(0) as u16)
    {
        return false;
    }
    // No push or pull on a vehicle, nor on any NPC in siege (`w_force.c:598-615`).
    !npc_immune(other.npc_class, context.gametype, false)
}

/// `ForcePowerUsableOn`'s NPC rules (`w_force.c:598-615`): the Force does not reach a
/// vehicle (but its `lightning`), nor any NPC in siege.
pub(crate) fn npc_immune(npc_class: Option<i32>, gametype: i32, lightning: bool) -> bool {
    const CLASS_VEHICLE: i32 = 53;
    match npc_class {
        Some(CLASS_VEHICLE) if !lightning => true,
        Some(_) => gametype == GT_SIEGE,
        None => false,
    }
}

/// `OnSameTeam` for players: teammates in the team games; nobody in the others.
fn on_same_team(gametype: i32, one: i32, other: i32) -> bool {
    gametype >= 6 && one == other
}

/// `G_InGetUpAnim`: the legs or the torso in a get-up.
fn in_get_up(state: &PlayerState) -> bool {
    [PS_LEGS_ANIM, PS_TORSO_ANIM]
        .into_iter()
        .any(|index| crate::knockdown::in_get_up(state.raw_field(index).unwrap_or(0) as u16))
}

/// `InFieldOfVision` (`ai_main.c:2058-2095`): `angles` within half of `fov` of the view,
/// in pitch and in yaw.
pub(crate) fn in_field_of_vision(view: [f32; 3], fov: f32, angles: [f32; 2]) -> bool {
    for axis in 0..2 {
        let angle = angle_mod(view[axis]);
        let target = angle_mod(angles[axis]);
        let mut difference = target - angle;
        if target > angle {
            if difference > 180.0 {
                difference -= 360.0;
            }
        } else if difference < -180.0 {
            difference += 360.0;
        }
        if difference > 0.0 {
            if difference > fov * 0.5 {
                return false;
            }
        } else if difference < -fov * 0.5 {
            return false;
        }
    }
    true
}

/// `AngleMod`: an angle's short, as the game rounds it (double constants, a float
/// result).
fn angle_mod(angle: f32) -> f32 {
    ((360.0 / 65_536.0) * f64::from((f64::from(angle) * (65_536.0 / 360.0)) as i32 & 65_535)) as f32
}

/// `AngleSubtract`: the difference brought into -180..180.
fn angle_subtract(a: f32, b: f32) -> f32 {
    let mut difference = a - b;
    while difference > 180.0 {
        difference -= 360.0;
    }
    while difference < -180.0 {
        difference += 360.0;
    }
    difference
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

/// `VectorNormalize`.
fn normalize(v: &mut [f32; 3]) {
    let length = length(*v);
    if length != 0.0 {
        let inverse = 1.0 / length;
        for axis in v.iter_mut() {
            *axis *= inverse;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movers::MoverState;

    /// A level with one mover, numbered 40, struck by every trace that goes its way.
    struct DoorWorld {
        door: MoverView,
        struck: bool,
        used: Vec<u16>,
        rng: Rng,
    }

    impl ThrowWorld for DoorWorld {
        fn entities_in_box(&mut self, _: [f32; 3], _: [f32; 3], out: &mut Vec<Candidate>) {
            out.clear();
        }
        fn trace(&mut self, _: [f32; 3], end: [f32; 3], _: u16, _: u32) -> MovementTrace {
            let mut hit = MovementTrace::miss(end);
            if self.struck {
                hit.fraction = 0.5;
                hit.entity_number = 40;
            }
            hit
        }
        fn in_pvs(&mut self, _: [f32; 3], _: [f32; 3]) -> bool {
            false
        }
        fn player(&mut self, _: u16) -> Option<ThrowPlayer<'_>> {
            None
        }
        fn reflect_missile(&mut self, _: u16, _: u16, _: [f32; 3]) {}
        fn toss_weapon(&mut self, _: u16, _: [f32; 3], _: f32) {}
        fn raise(&mut self, _: EventEntity) {}
        fn sound_index(&mut self, _: &[u8]) -> u16 {
            0
        }
        fn rng(&mut self) -> &mut Rng {
            &mut self.rng
        }
        fn mover(&mut self, number: u16) -> Option<MoverView> {
            (number == 40).then_some(self.door)
        }
        fn use_mover(&mut self, number: u16, _: u16) {
            self.used.push(number);
        }
    }

    /// A thrower at the origin looking down +x.
    fn thrower() -> Thrower {
        Thrower {
            origin: [0.0; 3],
            eye: [0.0, 0.0, 36.0],
            view: [0.0; 3],
            forward: [1.0, 0.0, 0.0],
            level: 3,
            cost: 0,
            team: 0,
        }
    }

    /// A door 200 units ahead, with no origin brush, that slides `travel` along x.
    fn door(state: MoverState, travel: f32) -> (MoverView, Candidate) {
        let view = MoverView {
            state,
            pos1: [0.0; 3],
            pos2: [travel, 0.0, 0.0],
            start_open: false,
            origin_brush: false,
        };
        let shift = if state == MoverState::Pos2 {
            travel
        } else {
            0.0
        };
        let candidate = Candidate {
            number: 40,
            kind: CandidateKind::ForceDoor { at_rest: true },
            absmin: [200.0 + shift, -64.0, 0.0],
            absmax: [208.0 + shift, 64.0, 128.0],
            origin: [shift, 0.0, 0.0],
        };
        (view, candidate)
    }

    fn throw_at(state: MoverState, travel: f32, pull: bool, struck: bool) -> Vec<u16> {
        let (view, candidate) = door(state, travel);
        let mut world = DoorWorld {
            door: view,
            struck,
            used: Vec::new(),
            rng: Rng(0),
        };
        let context = Throw {
            level_time: 1_000,
            gametype: 0,
            thrower: 0,
            pull,
            rules: ForceRules::default(),
            full_force: crate::duel::FullForce::default(),
        };
        force_door(context, &thrower(), candidate, &mut world);
        world.used
    }

    #[test]
    fn a_push_opens_a_door_that_opens_away() {
        assert_eq!(throw_at(MoverState::Pos1, 96.0, false, true), [40]);
        // Already at its far place: a push leaves it, a pull brings it back.
        assert!(throw_at(MoverState::Pos2, 96.0, false, true).is_empty());
        assert_eq!(throw_at(MoverState::Pos2, 96.0, true, true), [40]);
    }

    #[test]
    fn a_pull_opens_a_door_that_opens_towards_the_thrower() {
        assert_eq!(throw_at(MoverState::Pos1, -96.0, true, true), [40]);
        assert!(throw_at(MoverState::Pos1, -96.0, false, true).is_empty());
    }

    #[test]
    fn a_door_must_be_aimed_at() {
        assert!(throw_at(MoverState::Pos1, 96.0, false, false).is_empty());
    }

    #[test]
    fn a_door_on_its_way_or_a_pulled_button_is_out_of_reach() {
        let mut world = DoorWorld {
            door: door(MoverState::Pos1, 96.0).0,
            struck: true,
            used: Vec::new(),
            rng: Rng(0),
        };
        let context = |pull| Throw {
            level_time: 1_000,
            gametype: 0,
            thrower: 0,
            pull,
            rules: ForceRules::default(),
            full_force: crate::duel::FullForce::default(),
        };
        let (_, mut candidate) = door(MoverState::Pos1, 96.0);
        assert!(in_reach(context(false), &thrower(), candidate, &mut world).is_some());
        candidate.kind = CandidateKind::ForceDoor { at_rest: false };
        assert!(in_reach(context(false), &thrower(), candidate, &mut world).is_none());
        candidate.kind = CandidateKind::Button { pushable: true };
        assert!(in_reach(context(false), &thrower(), candidate, &mut world).is_some());
        assert!(in_reach(context(true), &thrower(), candidate, &mut world).is_none());
        candidate.kind = CandidateKind::Button { pushable: false };
        assert!(in_reach(context(false), &thrower(), candidate, &mut world).is_none());
    }

    /// Two players: the thrower (0) at the origin and the other (1) 100 units ahead, in
    /// teams `teams`, every trace clear.
    struct Pair {
        states: [PlayerState; 2],
        forces: [ForcePowers; 2],
        knockdowns: [Knockdown; 2],
        killers: [crate::damage::OtherKiller; 2],
        effects: [i32; 2],
        locks: [i32; 2],
        invulnerable: [i32; 2],
        teams: [i32; 2],
        rng: Rng,
    }

    impl Pair {
        fn new(teams: [i32; 2]) -> Self {
            let mut states = [PlayerState::zero(), PlayerState::zero()];
            states[1].set_origin([100.0, 0.0, 0.0]);
            Self {
                states,
                forces: [
                    ForcePowers::with_levels([0; 18]),
                    ForcePowers::with_levels([0; 18]),
                ],
                knockdowns: [Knockdown::default(); 2],
                killers: [crate::damage::OtherKiller::default(); 2],
                effects: [0; 2],
                locks: [0; 2],
                invulnerable: [0; 2],
                teams,
                rng: Rng(0),
            }
        }

        fn victim(&self) -> Candidate {
            Candidate {
                number: 1,
                kind: CandidateKind::Player,
                absmin: [85.0, -15.0, -24.0],
                absmax: [115.0, 15.0, 40.0],
                origin: [100.0, 0.0, 0.0],
            }
        }
    }

    impl ThrowWorld for Pair {
        fn entities_in_box(&mut self, _: [f32; 3], _: [f32; 3], out: &mut Vec<Candidate>) {
            out.clear();
        }
        fn trace(&mut self, _: [f32; 3], end: [f32; 3], _: u16, _: u32) -> MovementTrace {
            MovementTrace::miss(end)
        }
        fn in_pvs(&mut self, _: [f32; 3], _: [f32; 3]) -> bool {
            true
        }
        fn player(&mut self, number: u16) -> Option<ThrowPlayer<'_>> {
            let at = usize::from(number);
            if at > 1 {
                return None;
            }
            Some(ThrowPlayer {
                state: &mut self.states[at],
                force: &mut self.forces[at],
                health: 100,
                knockdown: &mut self.knockdowns[at],
                other_killer: &mut self.killers[at],
                npc_class: None,
                invulnerable_until: &mut self.invulnerable[at],
                command: UserCommand::default(),
                team: self.teams[at],
                push_effect_until: &mut self.effects[at],
                lock_hits: &mut self.locks[at],
            })
        }
        fn reflect_missile(&mut self, _: u16, _: u16, _: [f32; 3]) {}
        fn toss_weapon(&mut self, _: u16, _: [f32; 3], _: f32) {}
        fn raise(&mut self, _: EventEntity) {}
        fn sound_index(&mut self, _: &[u8]) -> u16 {
            0
        }
        fn rng(&mut self) -> &mut Rng {
            &mut self.rng
        }
    }

    fn push(gametype: i32, rules: ForceRules) -> Throw {
        Throw {
            level_time: 1_000,
            gametype,
            thrower: 0,
            pull: false,
            rules,
            full_force: crate::duel::FullForce::default(),
        }
    }

    fn ja_plus(throw_teammates: bool) -> ForceRules {
        ForceRules {
            throw_knocked_down: true,
            throw_teammates,
            absorb_gain_scale: 2.0,
            ..ForceRules::default()
        }
    }

    fn thrower_in(team: i32) -> Thrower {
        Thrower { team, ..thrower() }
    }

    #[test]
    fn a_teammate_is_thrown_only_where_the_server_says() {
        const GT_TEAM: i32 = 6;
        let mut pair = Pair::new([1, 1]);
        let victim = pair.victim();
        // The base game passes over a teammate.
        let base = push(GT_TEAM, ForceRules::default());
        assert!(in_reach(base, &thrower_in(1), victim, &mut pair).is_none());
        // JA+ without friendly fire likewise; with it, the teammate is thrown.
        let japlus = push(GT_TEAM, ja_plus(false));
        assert!(in_reach(japlus, &thrower_in(1), victim, &mut pair).is_none());
        let friendly = push(GT_TEAM, ja_plus(true));
        assert!(in_reach(friendly, &thrower_in(1), victim, &mut pair).is_some());
        // A foe is reached under every rule.
        let mut pair = Pair::new([1, 2]);
        assert!(in_reach(base, &thrower_in(1), victim, &mut pair).is_some());
        // Outside the team games nobody is a teammate.
        let mut pair = Pair::new([0, 0]);
        let free_for_all = push(0, ForceRules::default());
        assert!(in_reach(free_for_all, &thrower(), victim, &mut pair).is_some());
    }

    #[test]
    fn a_player_knocked_down_is_thrown_only_where_the_server_says() {
        let mut pair = Pair::new([0, 0]);
        let base = push(0, ForceRules::default());
        let japlus = push(0, ja_plus(false));
        for animation in ["BOTH_KNOCKDOWN1", "BOTH_GETUP1"] {
            let animation = crate::legacy_animation_index(animation).unwrap() as u32;
            pair.states[1].set_raw_field(PS_LEGS_ANIM, animation);
            assert!(!usable_on(base, 1, FP_PUSH, &mut pair));
            assert!(usable_on(japlus, 1, FP_PUSH, &mut pair));
        }
        // Standing, the same under every rule.
        pair.states[1].set_raw_field(PS_LEGS_ANIM, 0);
        assert!(usable_on(base, 1, FP_PUSH, &mut pair));
        assert!(usable_on(japlus, 1, FP_PUSH, &mut pair));
    }

    /// The pool player 1 has after absorbing `power` at level 2 that cost `spent`.
    fn absorbed(power: usize, spent: i32, rules: ForceRules) -> (i32, u32) {
        let mut pair = Pair::new([0, 0]);
        pair.forces[1].levels[FP_ABSORB] = 3;
        pair.forces[1].max = 100;
        pair.states[1].set_raw_field(PS_ACTIVE, 1 << FP_ABSORB);
        let (level, _) = absorb_conversion(
            &mut pair.states[1],
            &mut pair.forces[1],
            1,
            power,
            2,
            spent,
            1_000,
            rules,
        )
        .unwrap();
        (level, pair.states[1].raw_field(PS_FORCE_POWER).unwrap())
    }

    #[test]
    fn absorbing_a_push_gains_the_base_games_third_under_every_rule() {
        for rules in [ForceRules::default(), ja_plus(false)] {
            assert_eq!(absorbed(FP_PUSH, 20, rules), (0, 18));
            assert_eq!(absorbed(FP_PULL, 1, rules), (0, 1));
        }
    }

    #[test]
    fn fix_absorb_gains_more_from_the_continuous_powers() {
        use crate::force_powers::{FP_DRAIN, FP_LIGHTNING};
        assert_eq!(absorbed(FP_LIGHTNING, 1, ForceRules::default()), (0, 1));
        assert_eq!(absorbed(FP_LIGHTNING, 1, ja_plus(false)), (0, 2));
        assert_eq!(absorbed(FP_DRAIN, 4, ForceRules::default()), (0, 3));
        assert_eq!(absorbed(FP_DRAIN, 4, ja_plus(false)), (0, 8));
        assert_eq!(absorbed(FP_GRIP, 30, ja_plus(false)), (0, 60));
    }

    #[test]
    fn an_item_is_reached_only_where_the_server_pushes_items() {
        let mut pair = Pair::new([0, 0]);
        let item = Candidate {
            kind: CandidateKind::Item,
            ..pair.victim()
        };
        assert!(in_reach(push(0, ForceRules::default()), &thrower(), item, &mut pair).is_none());
        let pushing = ForceRules {
            push_items: 1,
            ..ForceRules::default()
        };
        assert!(in_reach(push(0, pushing), &thrower(), item, &mut pair).is_some());
    }
}
