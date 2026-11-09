//! JA+'s grappling hook on the server: the hook missile fired on `+button12`, taking hold
//! of the world or a player, the rope's pull and hang, the fuel it shares with the
//! jetpack, letting go, and the blades and shots that cut the rope.
//!
//! JA+ is closed source. The reference is VisionJK's rebuild of the JA+ 2.4 B7 server
//! module (`codemp/game/japlus/g_japlus_hook.c`, with its calls in `g_active.c:3403-3405`,
//! `g_main.c:3341-3344`, `g_missile.c:776-778,897-900`, `w_saber.c:4013-4016,6299-6300`);
//! its behaviour is ported here, not its code.
//!
//! The movement itself is the shared `Pmove`'s ([`crate::pmove::grapple`]), which every
//! JA+ client predicts: the server only sets what it reads, `PMF_GRAPPLE_PULL`
//! ([`PMF_GRAPPLE`]) in `pm_flags` while the rope pulls, `EF_GRAPPLE_SUSPEND`
//! ([`EF_ROPE_HANG`]) in `eFlags` while the player hangs on it, the anchor in
//! `ps.lastHitLoc` and the fuel in `ps.jetpackFuel`, and [`EF2_HOOK`] in `eFlags2` while
//! the hook is out. The game's part runs after the move (`JP_HookClientThink` follows
//! `Pmove` in `ClientThink_real`), so a command with use held is still pulled and the hook
//! lets go after it.
//!
//! The pieces are plain functions over the player's state and its [`Hook`] memory; the
//! server (`sjk-dedicated`'s `bridge_hook`) spawns, moves and frees the entities.

use crate::entity_id::EntityId;
use crate::pmove::grapple::{EF_ROPE_HANG, PMF_GRAPPLE};
use crate::weapon_fire::{Missile, create_missile_by, muzzle_point};
use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS, PlayerState};

/// `BUTTON_HOOK`, `+button12` (VisionJK `q_shared.h:1373`).
pub const BUTTON_HOOK: u16 = 4096;
/// `BUTTON_USE`: lets go of the hook.
pub const BUTTON_USE: u16 = 32;
/// `BUTTON_WALKING`: a plugin user who lets go of the hook key while walking neither hangs
/// nor is pulled.
const BUTTON_WALKING: u16 = 16;
/// `EF2_HOOK` (VisionJK `bg_public.h:739`): the hook is out.
pub const EF2_HOOK: u32 = 1 << 9;
/// `EF2_FLYING`: no hook while flying.
const EF2_FLYING: u32 = 1 << 4;
/// `EF_DEAD`, `EF_JETPACK_ACTIVE`, `EF_JETPACK_FLAMING`.
const EF_DEAD: u32 = 1 << 1;
const EF_JETPACK_ACTIVE: u32 = 1 << 11;
const EF_JETPACK_FLAMING: u32 = 1 << 30;
/// `PM_DEAD`.
const PM_DEAD: u8 = 5;
/// The fuel the hook needs to be fired (`JP_HOOK_MIN_FUEL`), and its most.
pub const MIN_FUEL: i32 = 10;
pub const FUEL_MAX: i32 = 100;
/// `JP_HOOK_CLIPMASK`: what the hook flies into (solid, player clip, bodies, the
/// terrain and the shot clip).
pub const CLIP_MASK: u32 = 0x41301;
/// `DF_NO_HOOK_FUEL`: the `dmflags` bit with which the hook costs no fuel, and no rope is
/// cut.
pub const DF_NO_HOOK_FUEL: i32 = 0x200;
/// `CreateMissile`'s life for the hook and its twin (`fire_grapple`'s 10000).
const HOOK_LIFE: i32 = 10_000;
/// The twin's `otherEntityNum` (`MAX_CLIENTS`, "32 in the original, not the owner").
const TWIN_OTHER_ENTITY: u32 = 32;
/// A hook that took hold thinks 100 ms on (`JP_HookMissileImpact`), then every five
/// minutes (`Weapon_HookThink`).
pub const FIRST_THINK: i32 = 100;
pub const HOLD_THINK: i32 = 300_000;
/// `+use` lets go at most every 200 ms.
const USE_REPEAT: i32 = 200;
/// No hook in the 1.5 seconds after a spawn.
const SPAWN_GRACE: i32 = 1_500;
/// A cut rope: no new hook for 10 seconds, the jetpack switch locked for 3.
pub const CUT_HOOK_DELAY: i32 = 10_000;
pub const CUT_TOGGLE_DELAY: i32 = 3_000;
/// The cutter is credited for a fall for 10 seconds, with a 5-second debounce.
pub const CUT_KILLER_TIME: i32 = 10_000;
pub const CUT_KILLER_DEBOUNCE: i32 = 5_000;
/// The swing, the shot and the thrown saber's reach on a rope (`G_WeapCutGrappleRope`'s
/// radius at its three calls).
pub const SABER_CUT_RADIUS: f32 = 5.0;
pub const MISSILE_CUT_RADIUS: f32 = 10.0;
pub const THROWN_CUT_RADIUS: f32 = 25.0;
pub const RETURNING_CUT_RADIUS: f32 = 10.0;
/// A rope's own owner cuts it only beyond this far from the hand (times the model scale).
const OWN_CUT_REACH: f32 = 60.0;
/// What firing plays (`G_SoundOnEnt(..., CHAN_ITEM, ...)`) and the cut's flash.
pub const FIRE_SOUND: &[u8] = b"sound/weapons/blaster/reflect1.mp3";
pub const CUT_EFFECT: &[u8] = b"blaster/deflect";
/// `CHAN_ITEM`.
pub const CHAN_ITEM: u32 = 4;
/// The pose of the throw, both halves held (`BOTH_RESISTPUSH`).
pub const BOTH_RESISTPUSH: u16 = 1_330;
/// `WP_STUN_BATON` (the hook of a plugin user, whose plugin draws the rope; also the miss
/// effect's weapon), `WP_BRYAR_PISTOL` (the hook of anyone else, and the twin).
const WP_STUN_BATON: u32 = 1;
const WP_BRYAR_PISTOL: u32 = 4;
/// `MOD_STUN_BATON`, `DAMAGE_NO_KNOCKBACK`.
const MOD_STUN_BATON: u32 = 1;
const DAMAGE_NO_KNOCKBACK: u32 = 0x4;
/// `GT_FFA`, `GT_TEAM`.
const GT_FFA: i32 = 0;
const GT_TEAM: i32 = 6;

/// Protocol-26 player state fields: `eFlags`, `eFlags2`, `jetpackFuel`,
/// `lastHitLoc[0..2]`.
const PS_EFLAGS: usize = 17;
const PS_EFLAGS2: usize = 103;
pub const PS_JETPACK_FUEL: usize = 39;
const PS_LAST_HIT_LOC: [usize; 3] = [102, 105, 100];

/// Protocol-26 entity state fields.
const ES_POS_TIME: usize = 0;
const ES_POS_BASE: [usize; 3] = [2, 1, 4];
const ES_APOS_BASE: [usize; 3] = [5, 3, 33];
const ES_POS_DELTA: [usize; 3] = [6, 7, 10];
const ES_TYPE: usize = 8;
const ES_WEAPON: usize = 14;
const ES_POS_DURATION: usize = 20;
const ES_POS_TYPE: usize = 23;
const ES_EVENT: usize = 28;
const ES_EVENT_PARM: usize = 42;
const ES_OTHER_ENTITY: usize = 59;
const ET_GENERAL: u32 = 0;
const TR_STATIONARY: u32 = 0;
const EV_MISSILE_HIT: u32 = 85;
const EV_MISSILE_MISS: u32 = 86;
const EVENT_BIT1: u32 = 0x100;

/// The hook's settings, as the running mod gives them. [`Default`] is the base game's:
/// no hook.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HookRules {
    /// `jp_allowHook`: 0 none, 1 in free-for-all (and in team deathmatch with
    /// `g_jediVmerc`), 2 only for the mercenaries of `g_jediVmerc`.
    pub allow: i32,
    /// `jp_hookSpeed`: the missile's speed.
    pub speed: i32,
    /// `jp_hookFloodProtect`: milliseconds between two hooks.
    pub flood_protect: i32,
    /// `dmflags` [`DF_NO_HOOK_FUEL`]: the hook costs no fuel and its rope is never cut.
    pub fuel_free: bool,
}

impl HookRules {
    /// Whether the server has a hook at all.
    pub fn enabled(self) -> bool {
        self.allow != 0
    }

    /// `JP_HookAllowedHere` (`g_japlus_hook.c:274-287`): the game type and setting let
    /// this player hook. (The alternate dimension, which also forbids it, is not built.)
    pub fn allowed_here(self, gametype: i32, jedi_v_merc: bool, true_non_jedi: bool) -> bool {
        if gametype != GT_FFA && gametype != GT_TEAM {
            return false;
        }
        match self.allow {
            1 => gametype == GT_FFA || jedi_v_merc,
            2 => jedi_v_merc && true_non_jedi,
            _ => false,
        }
    }

    /// `G_WeapCutGrappleRope`'s guard: ropes are cut only with the hook on and costing fuel.
    pub fn ropes_cut(self) -> bool {
        self.enabled() && !self.fuel_free
    }
}

/// What a player keeps of its hook (`client->hook` and `pers.jp`'s hook fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hook {
    /// `client->hook`: the hook's entity, flying or holding.
    pub entity: Option<EntityId>,
    /// Its twin (`hook2`), fired beside it for a plugin user, and when the twin's own life
    /// ends.
    pub twin: Option<EntityId>,
    pub twin_free_at: i32,
    /// Where both were fired from (`s.pos.trBase`), which the point they hold is snapped
    /// towards.
    pub base: [f32; 3],
    /// The hook has taken hold: it is no flying missile any more, and thinks at
    /// [`Self::think_at`].
    pub holding: bool,
    pub think_at: i32,
    /// Where the hook holds (`r.currentOrigin`), and the player or NPC it holds, if any
    /// (`hook->enemy`).
    pub origin: [f32; 3],
    pub enemy: Option<u16>,
    /// `pers.jp.hooked`: the hook took hold and pulled once.
    pub hooked: bool,
    /// `pers.jp.hookTime`, `pers.jp.hookUseTime`.
    pub hook_time: i32,
    pub use_time: i32,
    /// `pers.jp.pluginVersion >= 1.0`: the client plugin's user, which hangs on the rope
    /// when it lets go of the key; once known, kept for the connection.
    pub plugin: bool,
}

impl Hook {
    /// Whether the hook is out (`client->hook`).
    pub fn out(&self) -> bool {
        self.entity.is_some()
    }
}

/// `pers.jp.pluginVersion >= 1.0` from a `cjp_client` value: a JA+ server takes a client
/// for a plugin user only when the value starts with a digit and reads as a version of
/// 1.0 or more (`sscanf("%f")`, VisionJK `g_japlus_misc.c:766-773`): `1.4B4`, `1.4 SJK`,
/// `1.4JAPRO`, not `SJK`.
pub fn plugin_user(cjp_client: &[u8]) -> bool {
    if !cjp_client.first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    let end = cjp_client
        .iter()
        .enumerate()
        .take_while(|(index, byte)| {
            byte.is_ascii_digit() || (**byte == b'.' && !cjp_client[..*index].contains(&b'.'))
        })
        .count();
    std::str::from_utf8(&cjp_client[..end])
        .ok()
        .and_then(|text| text.trim_end_matches('.').parse::<f32>().ok())
        .is_some_and(|version| version >= 1.0)
}

/// `ps.eFlags2`.
pub fn entity_flags2(state: &PlayerState) -> u32 {
    state.raw_field(PS_EFLAGS2).unwrap_or(0)
}

/// `ps.jetpackFuel`.
pub fn fuel(state: &PlayerState) -> i32 {
    state.raw_field(PS_JETPACK_FUEL).unwrap_or(0) as i32
}

/// `ps.lastHitLoc`, the point the rope pulls to.
pub fn set_anchor(state: &mut PlayerState, point: [f32; 3]) {
    for (index, value) in PS_LAST_HIT_LOC.into_iter().zip(point) {
        state.set_raw_field(index, value.to_bits());
    }
}

/// `Weapon_HookFree`'s part on the owner (`g_japlus_hook.c:57-63`): not hooked, not
/// pulled, not hanging, no hook out. The entities ([`Hook::entity`], [`Hook::twin`]) are
/// the caller's to free and forget.
pub fn release(state: &mut PlayerState, hook: &mut Hook) {
    hook.hooked = false;
    hook.holding = false;
    hook.enemy = None;
    state.set_movement_flags(state.movement_flags() & !PMF_GRAPPLE);
    let flags = state.raw_field(PS_EFLAGS).unwrap_or(0);
    state.set_raw_field(PS_EFLAGS, flags & !EF_ROPE_HANG);
    state.set_raw_field(PS_EFLAGS2, entity_flags2(state) & !EF2_HOOK);
}

/// What the server knows of a player beside its state for `JP_HookCanFire`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shooter {
    /// `ent->health`.
    pub health: i32,
    /// `client->respawnTime`.
    pub respawn_time: i32,
    /// `fd.forceGripBeingGripped >= level.time`.
    pub gripped: bool,
}

/// `JP_HookCanFire` (`g_japlus_hook.c:245-272`) with the command's `buttons`. JA+'s
/// sleeping and knocked-down admin states are not built, so they never forbid it.
pub fn can_fire(
    state: &PlayerState,
    hook: &Hook,
    rules: HookRules,
    shooter: Shooter,
    buttons: u16,
    level_time: i32,
) -> bool {
    let flags = state.entity_flags();
    let flags2 = entity_flags2(state);
    buttons & BUTTON_HOOK != 0
        && !state.duel_in_progress()
        && state.movement_type() != PM_DEAD
        && shooter.health > 0
        && state.stats[0] as i32 > 0
        && flags & EF_DEAD == 0
        && !crate::saber_rules::in_attack(state.saber_move())
        && state.vehicle_entity_num() == 0
        && shooter.respawn_time + SPAWN_GRACE < level_time
        && !crate::prediction_policy::in_grapple_move(state.leg_animation())
        && !crate::prediction_policy::in_grapple_move(state.torso_animation())
        && hook.hook_time.wrapping_add(rules.flood_protect) < level_time
        && !shooter.gripped
        && flags2 & EF2_HOOK == 0
        && !hook.hooked
        && !hook.out()
        && state.movement_flags() & PMF_GRAPPLE == 0
        && flags & EF_ROPE_HANG == 0
        && flags & (EF_JETPACK_FLAMING | EF_JETPACK_ACTIVE) == 0
        && flags2 & EF2_FLYING == 0
        && (fuel(state) >= MIN_FUEL || rules.fuel_free)
}

/// What a think asks of the server, in this order: fire the hook ([`fire`]), then free
/// it (its entities, which the caller forgets; [`release`] has already been applied).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Think {
    pub fire: bool,
    pub free: bool,
}

/// `JP_HookClientThink` (`g_japlus_hook.c:296-359`) after the move, with the buttons of
/// the command the move worked on: fire, hold, hang, pull again and let go. `allowed` is
/// [`HookRules::allowed_here`] for a connected player, `fires` [`can_fire`]. The state's
/// flags and the memory change here; the entities are the caller's.
pub fn client_think(
    state: &mut PlayerState,
    hook: &mut Hook,
    buttons: u16,
    allowed: bool,
    fires: bool,
    level_time: i32,
) -> Think {
    let mut think = Think::default();
    let mut out = hook.out();
    let free = |state: &mut PlayerState, hook: &mut Hook, out: &mut bool, think: &mut Think| {
        release(state, hook);
        *out = false;
        think.free = true;
    };
    if allowed {
        if fires {
            // `Weapon_GrapplingHook_Fire`: the hook is out, not yet hooked.
            hook.hook_time = level_time;
            think.fire = true;
            out = true;
            state.set_raw_field(PS_EFLAGS2, entity_flags2(state) | EF2_HOOK);
            hook.hooked = false;
        }
        if out && buttons & BUTTON_HOOK == 0 {
            if !hook.plugin || !hook.hooked {
                // Without the plugin, or before it took hold: letting go drops the hook.
                free(state, hook, &mut out, &mut think);
            } else if buttons & BUTTON_WALKING == 0 {
                // Hang on the rope.
                state.set_movement_flags(state.movement_flags() & !PMF_GRAPPLE);
                let flags = state.raw_field(PS_EFLAGS).unwrap_or(0);
                state.set_raw_field(PS_EFLAGS, flags | EF_ROPE_HANG);
            } else {
                state.set_movement_flags(state.movement_flags() & !PMF_GRAPPLE);
                let flags = state.raw_field(PS_EFLAGS).unwrap_or(0);
                state.set_raw_field(PS_EFLAGS, flags & !EF_ROPE_HANG);
            }
        }
        // Pressing the key again while hanging pulls again.
        let flags = state.raw_field(PS_EFLAGS).unwrap_or(0);
        if out && buttons & BUTTON_HOOK != 0 && hook.hooked && flags & EF_ROPE_HANG != 0 {
            state.set_raw_field(PS_EFLAGS, flags & !EF_ROPE_HANG);
            state.set_movement_flags(state.movement_flags() | PMF_GRAPPLE);
        }
    }
    if out
        && (entity_flags2(state) & EF2_HOOK == 0
            || state.movement_type() == PM_DEAD
            || state.duel_in_progress())
    {
        free(state, hook, &mut out, &mut think);
    }
    // `+use` lets go of the hook.
    if out && buttons & BUTTON_USE != 0 && hook.use_time.wrapping_add(USE_REPEAT) < level_time {
        hook.use_time = level_time;
        free(state, hook, &mut out, &mut think);
    }
    let flags = state.raw_field(PS_EFLAGS).unwrap_or(0);
    if !out
        && (hook.hooked
            || state.movement_flags() & PMF_GRAPPLE != 0
            || flags & EF_ROPE_HANG != 0
            || entity_flags2(state) & EF2_HOOK != 0)
    {
        // No hook, so none of its flags.
        release(state, hook);
    }
    think
}

/// The hook missile and, for a plugin user, its twin (`fire_grapple`,
/// `g_japlus_hook.c:115-155`), from the muzzle along the view (`CalcMuzzlePoint`) of
/// the player whose entity `entity` this think converted. The twin is the rope's other
/// end for the JA+ plugin; it is a pistol bolt that does no damage.
pub fn fire(
    state: &PlayerState,
    entity: &EntityState,
    rules: HookRules,
    plugin: bool,
    level_time: i32,
) -> (Missile, Option<Missile>) {
    let (muzzle, forward) = muzzle_point(state, entity);
    let length =
        (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2]).sqrt();
    let direction = if length > 0.0 {
        forward.map(|axis| axis / length)
    } else {
        forward
    };
    let owner = state.client_num();
    let speed = rules.speed as f32;
    let launch = || {
        let mut missile = create_missile_by(owner, muzzle, direction, speed, level_time, false);
        missile.free_at = level_time + HOOK_LIFE;
        missile.method_of_death = MOD_STUN_BATON;
        missile.damage_flags = DAMAGE_NO_KNOCKBACK;
        missile.clip_mask = CLIP_MASK;
        missile
    };
    let mut hook = launch();
    // `vectoangles` and `G_SetAngles`.
    let (pitch, yaw) = crate::damage::vector_to_angles(direction);
    for (index, value) in ES_APOS_BASE.into_iter().zip([pitch, yaw, 0.0]) {
        hook.state.set_raw_field(index, value.to_bits());
    }
    let twin = plugin.then(|| {
        let mut twin = launch();
        twin.state.set_raw_field(ES_WEAPON, WP_BRYAR_PISTOL);
        twin.state.set_raw_field(ES_OTHER_ENTITY, TWIN_OTHER_ENTITY);
        twin.damage = 0;
        twin
    });
    hook.state.set_raw_field(
        ES_WEAPON,
        if plugin {
            WP_STUN_BATON
        } else {
            WP_BRYAR_PISTOL
        },
    );
    hook.damage = 1;
    hook.state
        .set_raw_field(ES_OTHER_ENTITY, u32::from(state.client_num()));
    (hook, twin)
}

/// `SnapVectorTowards` as the JA+ module's 1.01 game code has it: the decimals cut off,
/// one more towards `to` (VisionJK `docs/japlus/differences.md`: its own rounds, so the
/// anchor can be a unit or two away from the original's).
pub fn snap_towards(point: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let whole = point[axis] as i32;
        if to[axis] <= point[axis] {
            whole as f32
        } else {
            (whole + 1) as f32
        }
    })
}

/// `JP_HookImpactPoint` on a player or NPC: the middle of its box where it stands.
pub fn box_middle(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| (mins[axis] + maxs[axis]) * 0.5 + origin[axis])
}

/// `G_SetOrigin` on the hook's entity: still at `point`. The rest of `state` (its type,
/// weapon and owner) is kept, so a plugin user's hook stays the stun baton's missile the
/// rope is drawn to.
pub fn stand(state: &mut EntityState, point: [f32; 3]) {
    state.set_raw_field(ES_POS_TYPE, TR_STATIONARY);
    state.set_raw_field(ES_POS_TIME, 0);
    state.set_raw_field(ES_POS_DURATION, 0);
    for axis in 0..3 {
        state.set_raw_field(ES_POS_BASE[axis], point[axis].to_bits());
        state.set_raw_field(ES_POS_DELTA[axis], 0);
    }
}

/// The twin taking hold (`g_japlus_hook.c:456-462`): a general entity still at `point`.
pub fn stand_twin(state: &mut EntityState, point: [f32; 3]) {
    state.set_raw_field(ES_TYPE, ET_GENERAL);
    state.set_raw_field(ES_EVENT, 0);
    state.set_raw_field(ES_EVENT_PARM, 0);
    stand(state, point);
}

/// The entity `JP_HookMissileImpact` spawns for the hook's impact (`g_japlus_hook.c:
/// 419-438`): a general entity at `point` freed after its event, the miss (as the stun
/// baton's) on the world, the hit naming the player or NPC struck. `normal` is
/// `DirToByte` of the surface's normal.
pub fn impact_effect(point: [f32; 3], normal: u32, struck: Option<u16>) -> EntityState {
    let mut state = EntityState::zero(0, &LEGACY_ENTITY_FIELDS);
    state.set_raw_field(ES_TYPE, ET_GENERAL);
    match struck {
        Some(other) => {
            state.set_raw_field(ES_EVENT, EV_MISSILE_HIT | EVENT_BIT1);
            state.set_raw_field(ES_OTHER_ENTITY, u32::from(other));
        }
        None => {
            state.set_raw_field(ES_WEAPON, WP_STUN_BATON);
            state.set_raw_field(ES_EVENT, EV_MISSILE_MISS | EVENT_BIT1);
        }
    }
    state.set_raw_field(ES_EVENT_PARM, normal);
    stand(&mut state, point);
    state
}

/// Whether the impact on a missile is the hit of a body (`EV_MISSILE_HIT`), not a miss.
pub fn struck_body(state: &EntityState) -> bool {
    state.raw_field(ES_EVENT).unwrap_or(0) & !0x300 == EV_MISSILE_HIT
}

/// `G_PlayEffectID(effect, point, direction)`: the flash where a rope was cut.
pub fn cut_effect(
    effect: u16,
    point: [f32; 3],
    direction: [f32; 3],
) -> crate::event_entity::EventEntity {
    crate::map_turret_world::effect_event(
        crate::map_turret_world::EV_PLAY_EFFECT_ID,
        u32::from(effect),
        point,
        direction,
    )
}

/// The surface normal's byte an impact put on a missile (`G_AddEvent`'s parameter).
pub fn impact_normal(state: &EntityState) -> u32 {
    state.raw_field(ES_EVENT_PARM).unwrap_or(0)
}

/// What the hook out draws from the fuel this frame ([`crate::holdables::batteries`] with
/// the jetpack off), and whether it ran dry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HookFuel {
    /// Points every 200 ms: one while pulled, `jp_hookSpeed / 800 + 1` while the hook
    /// still flies out, none while it holds without pulling.
    pub cost: i32,
    /// The fuel ran out: the hook lets go.
    pub ran_dry: bool,
}

/// `JP_HookFuelThink`'s draw (`g_japlus_hook.c:373-386`): a hook out, not costing nothing
/// and not hanging draws from the fuel; `None` recharges it as the base game does.
pub fn fuel_draw(state: &PlayerState, hook: &Hook, rules: HookRules) -> Option<HookFuel> {
    let hanging = state.entity_flags() & EF_ROPE_HANG != 0;
    if rules.fuel_free || !hook.out() || entity_flags2(state) & EF2_HOOK == 0 || hanging {
        return None;
    }
    let cost = if state.movement_flags() & PMF_GRAPPLE != 0 {
        1
    } else if !hook.hooked {
        rules.speed / 800 + 1
    } else {
        0
    };
    Some(HookFuel {
        cost,
        ran_dry: false,
    })
}

/// `G_WeapCutGrappleRope`'s test on one rope from `rope_start` (the owner's right hand)
/// to the hook at `hook`, for a cutter moving from `from` to `to` that reaches `radius`:
/// where on the rope and on the cutter's path it is cut, if it is. The owner's own cut
/// (`own`) must fall farther than 60 units (times the model `scale`) from the hand.
pub fn cut(
    rope_start: [f32; 3],
    hook: [f32; 3],
    from: [f32; 3],
    to: [f32; 3],
    radius: f32,
    own: bool,
    scale: f32,
) -> Option<([f32; 3], [f32; 3])> {
    let distance = |a: [f32; 3], b: [f32; 3]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    if distance(from, to) < 1.0 || distance(hook, rope_start) < 1.0 {
        return None;
    }
    let (length, on_rope, on_cutter) =
        crate::npc_jedi_block::shortest_between_segments(rope_start, hook, from, to);
    (length < radius && (!own || distance(on_rope, rope_start) > scale * OWN_CUT_REACH))
        .then_some((on_rope, on_cutter))
}

#[cfg(test)]
mod tests {
    use super::*;

    const JA_PLUS: HookRules = HookRules {
        allow: 1,
        speed: 800,
        flood_protect: 750,
        fuel_free: false,
    };

    fn player() -> PlayerState {
        let mut state = PlayerState::default();
        state.stats[0] = 100;
        state.set_raw_field(PS_JETPACK_FUEL, 100);
        state
    }

    fn shooter() -> Shooter {
        Shooter {
            health: 100,
            respawn_time: 0,
            gripped: false,
        }
    }

    #[test]
    fn the_base_game_has_no_hook() {
        let rules = HookRules::default();
        assert!(!rules.enabled() && !rules.ropes_cut());
        assert!(!rules.allowed_here(GT_FFA, false, false));
    }

    #[test]
    fn the_setting_and_game_type_say_who_may_hook() {
        assert!(JA_PLUS.allowed_here(GT_FFA, false, false));
        assert!(!JA_PLUS.allowed_here(GT_TEAM, false, false));
        assert!(JA_PLUS.allowed_here(GT_TEAM, true, false));
        // Duel, power duel, siege, CTF: never.
        for gametype in [3, 4, 7, 8] {
            assert!(!JA_PLUS.allowed_here(gametype, true, true));
        }
        let mercs = HookRules {
            allow: 2,
            ..JA_PLUS
        };
        assert!(!mercs.allowed_here(GT_FFA, false, true));
        assert!(!mercs.allowed_here(GT_FFA, true, false));
        assert!(mercs.allowed_here(GT_FFA, true, true));
        let free = HookRules {
            fuel_free: true,
            ..JA_PLUS
        };
        assert!(free.enabled() && !free.ropes_cut());
    }

    #[test]
    fn a_plugin_user_names_a_version() {
        assert!(plugin_user(b"1.4B4"));
        assert!(plugin_user(b"1.4 SJK"));
        assert!(plugin_user(b"1.4JAPRO"));
        assert!(plugin_user(b"1.6.4 JoF"));
        assert!(plugin_user(b"2"));
        assert!(!plugin_user(b"SJK"));
        assert!(!plugin_user(b"0.9"));
        assert!(!plugin_user(b""));
    }

    #[test]
    fn firing_needs_the_key_fuel_and_a_free_moment() {
        let state = player();
        let hook = Hook::default();
        assert!(can_fire(
            &state,
            &hook,
            JA_PLUS,
            shooter(),
            BUTTON_HOOK,
            2_000
        ));
        assert!(!can_fire(&state, &hook, JA_PLUS, shooter(), 0, 2_000));
        // 1.5 seconds after a spawn.
        assert!(!can_fire(
            &state,
            &hook,
            JA_PLUS,
            shooter(),
            BUTTON_HOOK,
            1_500
        ));
        // The flood protection after the last hook.
        let recent = Hook {
            hook_time: 1_500,
            ..Hook::default()
        };
        assert!(!can_fire(
            &state,
            &recent,
            JA_PLUS,
            shooter(),
            BUTTON_HOOK,
            2_250
        ));
        assert!(can_fire(
            &state,
            &recent,
            JA_PLUS,
            shooter(),
            BUTTON_HOOK,
            2_251
        ));
        // Too little fuel, unless the hook is free.
        let mut low = player();
        low.set_raw_field(PS_JETPACK_FUEL, 9);
        assert!(!can_fire(
            &low,
            &hook,
            JA_PLUS,
            shooter(),
            BUTTON_HOOK,
            2_000
        ));
        let free = HookRules {
            fuel_free: true,
            ..JA_PLUS
        };
        assert!(can_fire(&low, &hook, free, shooter(), BUTTON_HOOK, 2_000));
        // Gripped, dead, flying the jetpack.
        let gripped = Shooter {
            gripped: true,
            ..shooter()
        };
        assert!(!can_fire(
            &state,
            &hook,
            JA_PLUS,
            gripped,
            BUTTON_HOOK,
            2_000
        ));
        let dead = Shooter {
            health: 0,
            ..shooter()
        };
        assert!(!can_fire(&state, &hook, JA_PLUS, dead, BUTTON_HOOK, 2_000));
        let mut jetting = player();
        jetting.set_raw_field(PS_EFLAGS, EF_JETPACK_ACTIVE);
        assert!(!can_fire(
            &jetting,
            &hook,
            JA_PLUS,
            shooter(),
            BUTTON_HOOK,
            2_000
        ));
    }

    fn hooked_plugin() -> Hook {
        Hook {
            entity: Some(EntityId::new(3, 1)),
            hooked: true,
            plugin: true,
            ..Hook::default()
        }
    }

    fn pulled() -> PlayerState {
        let mut state = player();
        state.set_movement_flags(PMF_GRAPPLE);
        state.set_raw_field(PS_EFLAGS2, EF2_HOOK);
        state
    }

    #[test]
    fn a_plugin_user_hangs_when_the_key_is_let_go_and_pulls_again_on_it() {
        let mut state = pulled();
        let mut hook = hooked_plugin();
        let think = client_think(&mut state, &mut hook, 0, true, false, 5_000);
        assert_eq!(think, Think::default());
        assert_eq!(state.movement_flags() & PMF_GRAPPLE, 0);
        assert_ne!(state.entity_flags() & EF_ROPE_HANG, 0);
        // Walking: neither hanging nor pulled, still holding.
        let mut walking = pulled();
        let think = client_think(&mut walking, &mut hook, BUTTON_WALKING, true, false, 5_000);
        assert!(!think.free);
        assert_eq!(walking.movement_flags() & PMF_GRAPPLE, 0);
        assert_eq!(walking.entity_flags() & EF_ROPE_HANG, 0);
        // The key again: pulled again.
        let think = client_think(&mut state, &mut hook, BUTTON_HOOK, true, false, 5_008);
        assert!(!think.free && hook.out());
        assert_ne!(state.movement_flags() & PMF_GRAPPLE, 0);
        assert_eq!(state.entity_flags() & EF_ROPE_HANG, 0);
    }

    #[test]
    fn without_the_plugin_letting_go_of_the_key_drops_the_hook() {
        let mut state = pulled();
        let mut hook = Hook {
            plugin: false,
            ..hooked_plugin()
        };
        let think = client_think(&mut state, &mut hook, 0, true, false, 5_000);
        assert!(think.free && !hook.hooked);
        assert_eq!(state.movement_flags() & PMF_GRAPPLE, 0);
        assert_eq!(entity_flags2(&state) & EF2_HOOK, 0);
        // A plugin user's hook still flying is dropped too.
        let mut state = pulled();
        let mut hook = Hook {
            hooked: false,
            ..hooked_plugin()
        };
        assert!(client_think(&mut state, &mut hook, 0, true, false, 5_000).free);
    }

    #[test]
    fn use_lets_go_after_the_move_at_most_every_200_ms() {
        let mut state = pulled();
        let mut hook = Hook {
            use_time: 4_900,
            ..hooked_plugin()
        };
        let buttons = BUTTON_HOOK | BUTTON_USE;
        assert!(!client_think(&mut state, &mut hook, buttons, true, false, 5_000).free);
        let think = client_think(&mut state, &mut hook, buttons, true, false, 5_101);
        assert!(think.free);
        assert_eq!(hook.use_time, 5_101);
        assert_eq!(state.movement_flags() & PMF_GRAPPLE, 0);
    }

    #[test]
    fn firing_puts_the_hook_out() {
        let mut state = player();
        let mut hook = Hook::default();
        let think = client_think(&mut state, &mut hook, BUTTON_HOOK, true, true, 3_000);
        assert_eq!(
            think,
            Think {
                fire: true,
                free: false
            }
        );
        assert_eq!(hook.hook_time, 3_000);
        assert_ne!(entity_flags2(&state) & EF2_HOOK, 0);
        // A dead player, or a duellist, lets go; no hook clears stale flags.
        let mut dead = pulled();
        dead.set_movement_type(PM_DEAD);
        let mut hook = hooked_plugin();
        assert!(client_think(&mut dead, &mut hook, BUTTON_HOOK, true, false, 3_000).free);
        let mut stale = pulled();
        let mut none = Hook::default();
        assert_eq!(
            client_think(&mut stale, &mut none, 0, false, false, 3_000),
            Think::default()
        );
        assert_eq!(stale.movement_flags() & PMF_GRAPPLE, 0);
        assert_eq!(entity_flags2(&stale) & EF2_HOOK, 0);
    }

    #[test]
    fn the_hook_and_its_twin_fly_as_ja_plus_fires_them() {
        let mut state = player();
        state.set_client_num(3);
        state.set_view_angles([0.0, 90.0, 0.0]);
        let entity = EntityState::zero(3, &LEGACY_ENTITY_FIELDS);
        let (hook, twin) = fire(&state, &entity, JA_PLUS, true, 1_000);
        assert_eq!(hook.state.raw_field(ES_WEAPON), Some(WP_STUN_BATON));
        assert_eq!(hook.state.raw_field(ES_OTHER_ENTITY), Some(3));
        assert_eq!((hook.damage, hook.method_of_death), (1, MOD_STUN_BATON));
        assert_eq!(
            (hook.clip_mask, hook.damage_flags, hook.free_at),
            (CLIP_MASK, DAMAGE_NO_KNOCKBACK, 11_000)
        );
        let delta = ES_POS_DELTA.map(|index| f32::from_bits(hook.state.raw_field(index).unwrap()));
        assert_eq!(delta, [0.0, 800.0, 0.0]);
        let twin = twin.unwrap();
        assert_eq!(twin.state.raw_field(ES_WEAPON), Some(WP_BRYAR_PISTOL));
        assert_eq!(twin.state.raw_field(ES_OTHER_ENTITY), Some(32));
        assert_eq!(twin.damage, 0);
        // Without the plugin: a pistol bolt, no twin.
        let (hook, twin) = fire(&state, &entity, JA_PLUS, false, 1_000);
        assert_eq!(hook.state.raw_field(ES_WEAPON), Some(WP_BRYAR_PISTOL));
        assert!(twin.is_none());
    }

    #[test]
    fn the_fuel_drains_while_flying_out_and_pulling() {
        let rules = HookRules {
            speed: 1_600,
            ..JA_PLUS
        };
        let mut state = player();
        state.set_raw_field(PS_EFLAGS2, EF2_HOOK);
        let flying = Hook {
            entity: Some(EntityId::new(1, 1)),
            ..Hook::default()
        };
        assert_eq!(
            fuel_draw(&state, &flying, rules).map(|fuel| fuel.cost),
            Some(3)
        );
        let mut pulled = pulled();
        let holding = Hook {
            hooked: true,
            ..flying
        };
        assert_eq!(
            fuel_draw(&pulled, &holding, rules).map(|fuel| fuel.cost),
            Some(1)
        );
        // Hanging on the rope, or no hook: it recharges.
        pulled.set_movement_flags(0);
        pulled.set_raw_field(PS_EFLAGS, EF_ROPE_HANG);
        assert_eq!(fuel_draw(&pulled, &holding, rules), None);
        assert_eq!(fuel_draw(&state, &Hook::default(), rules), None);
        let free = HookRules {
            fuel_free: true,
            ..rules
        };
        assert_eq!(fuel_draw(&state, &flying, free), None);
        // Through the batteries: drawn every 200 ms, dry at zero.
        let mut packs = crate::holdables::Packs::default();
        let (mut jet, mut cloak) = (2, 100);
        let mut draw = fuel_draw(&state, &flying, rules).unwrap();
        crate::holdables::batteries(
            &mut packs,
            &mut jet,
            &mut cloak,
            false,
            false,
            1_000,
            true,
            Some(&mut draw),
        );
        assert_eq!(jet, 0);
        assert!(draw.ran_dry);
    }

    #[test]
    fn the_anchor_is_snapped_as_the_original_snaps() {
        assert_eq!(
            snap_towards([10.6, -10.6, 5.0], [0.0, 0.0, 9.0]),
            [10.0, -9.0, 6.0]
        );
        assert_eq!(
            box_middle(
                [100.0, 0.0, 24.0],
                [-15.0, -15.0, -24.0],
                [15.0, 15.0, 40.0]
            ),
            [100.0, 0.0, 32.0]
        );
    }

    #[test]
    fn a_rope_is_cut_where_a_cutter_crosses_it() {
        let hand = [0.0, 0.0, 0.0];
        let hook = [0.0, 0.0, 400.0];
        // A shot across the rope halfway up.
        let cut_at = cut(
            hand,
            hook,
            [-50.0, 3.0, 200.0],
            [50.0, 3.0, 200.0],
            10.0,
            false,
            1.0,
        );
        let (on_rope, _) = cut_at.unwrap();
        assert!((on_rope[2] - 200.0).abs() < 1e-3);
        // Too far off, or not moving.
        assert!(
            cut(
                hand,
                hook,
                [-50.0, 20.0, 200.0],
                [50.0, 20.0, 200.0],
                10.0,
                false,
                1.0
            )
            .is_none()
        );
        assert!(
            cut(
                hand,
                hook,
                [0.0, 3.0, 200.0],
                [0.5, 3.0, 200.0],
                10.0,
                false,
                1.0
            )
            .is_none()
        );
        // The owner's own shot cuts only beyond 60 units of the hand.
        assert!(
            cut(
                hand,
                hook,
                [-50.0, 3.0, 30.0],
                [50.0, 3.0, 30.0],
                10.0,
                true,
                1.0
            )
            .is_none()
        );
        assert!(
            cut(
                hand,
                hook,
                [-50.0, 3.0, 90.0],
                [50.0, 3.0, 90.0],
                10.0,
                true,
                1.0
            )
            .is_some()
        );
    }

    #[test]
    fn the_impact_effect_is_a_miss_or_a_named_hit() {
        let miss = impact_effect([1.0, 2.0, 3.0], 7, None);
        assert_eq!(miss.raw_field(ES_EVENT), Some(EV_MISSILE_MISS | EVENT_BIT1));
        assert_eq!(miss.raw_field(ES_WEAPON), Some(WP_STUN_BATON));
        assert_eq!(miss.raw_field(ES_EVENT_PARM), Some(7));
        let hit = impact_effect([1.0, 2.0, 3.0], 7, Some(5));
        assert_eq!(hit.raw_field(ES_EVENT), Some(EV_MISSILE_HIT | EVENT_BIT1));
        assert_eq!(hit.raw_field(ES_OTHER_ENTITY), Some(5));
    }
}
