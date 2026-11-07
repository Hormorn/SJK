//! The E-Web (OpenJK `codemp/game/g_items.c:1453-2051`, "Portable E-Web -rww"): the
//! holdable heavy repeater a player sets down in front of itself with the use key
//! ([`place`], `EWeb_Create`) and works from behind until it puts it away, backs off, dies
//! or the gun is shot to pieces (`EWebThink`, `EWebDie`). While worked the player's weapon
//! is the emplaced gun's (`ps.emplacedIndex` names the E-Web, which the clients tell from
//! an emplaced gun by its `s.weapon` of `WP_NONE`), the gun's top turns after its view at
//! four degrees a frame ([`turn`], `EWebUpdateBoneAngles`), the player is kept behind it
//! (`EWebPositionUser`) and every 100 ms the fire button sends a turret bolt from the
//! muzzle ([`bolt`], `EWebFire`).
//!
//! Single-player maps (`hoth2`) also place E-Webs of their own (`emplaced_eweb`,
//! `code/game/g_emplaced.cpp:153-499`), which the multiplayer game has no spawn for: here
//! they stand where the map puts them ([`from_map`]), anyone facing their way takes them
//! with the use key (`eweb_can_be_used`) and they are worked as the holdable is, staying
//! where they are when their gunner lets go.
//!
//! The gun's top is turned on the clients by the angles of its two bones sent in its
//! state (`EWeb_SetBoneAngles`: `boneIndex1..4`, `boneAngles1..4`, `boneOrient`) and its
//! muzzle and gunner's place are read on the server's Ghoul2 instance of the model, which
//! the caller keeps; its short animations are played by the clients from `torsoAnim`
//! and `legsAnim` (`EF_G2ANIMATING`, [`set_animation`]).

use crate::npc_droid::write_bone_angles;
use crate::player_angle_math::angle_subtract;
use crate::pmove::MovementTrace;
use crate::weapon_fire::Missile;
use sjk_entity::Entity;
use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS};

/// The model (`G_ModelIndex`), its two turned bones and its two bolts.
pub const MODEL: &[u8] = b"models/map_objects/hoth/eweb_model.glm";
pub const YAW_BONE: &[u8] = b"cannon_Yrot";
pub const PITCH_BONE: &[u8] = b"cannon_Xrot";
pub const MUZZLE_BOLT: &str = "*cannonflash";
/// The bolt the gunner is placed from: the holdable's `cannon_Yrot`, the map gun's
/// `cannon_Xrot` (`SP_emplaced_eweb`'s `headBolt`).
pub const PORTABLE_PLACE_BOLT: &str = "cannon_Yrot";
pub const MAP_PLACE_BOLT: &str = "cannon_Xrot";
/// The muzzle flash (`G_EffectIndex("turret/muzzle_flash.efx")`).
pub const MUZZLE_FLASH: &[u8] = b"turret/muzzle_flash.efx";
/// The map gun's death (`G_PlayEffect("emplaced/explode")`).
pub const MAP_EXPLOSION: &[u8] = b"emplaced/explode";
/// The holdable's box (`EWeb_Create`) and the map gun's (`SP_emplaced_eweb`).
pub const BOUNDS: ([f32; 3], [f32; 3]) = ([-32.0, -32.0, -24.0], [32.0, 32.0, 24.0]);
pub const MAP_BOUNDS: ([f32; 3], [f32; 3]) = ([-12.0, -12.0, -24.0], [12.0, 12.0, 24.0]);
/// `MASK_PLAYERSOLID`: the holdable's clip mask and contents; `CONTENTS_BODY`, the map
/// gun's.
pub const MASK_PLAYERSOLID: u32 = 0x1 | 0x10 | 0x100 | 0x1000;
pub const CONTENTS_BODY: u32 = 0x100;
/// `EWEB_HEALTH`.
pub const HEALTH: i32 = 200;
/// `EWEB_DEATH_RADIUS`, `EWEB_DEATH_DMG`.
pub const DEATH_RADIUS: i32 = 128;
pub const DEATH_DAMAGE: i32 = 90;
/// `EWEB_USE_DEBOUNCE`: between two uses of the holdable.
pub const USE_DEBOUNCE: i32 = 1_000;
/// Between two shots (`genericValue5 = level.time + 100`).
pub const FIRE_DEBOUNCE: i32 = 100;
/// "don't allow use until the anim is done playing (rough time estimate)".
pub const UNFOLD_TIME: i32 = 500;
/// `EWEB_MISSILE_DAMAGE`; `CreateMissile(bPoint, d, 1200.0f, 10000, ...)`.
const MISSILE_DAMAGE: i32 = 20;
const MISSILE_SPEED: f32 = 1_200.0;
const MISSILE_LIFE: i32 = 10_000;
/// "max degrees we can turn per update".
const TURN_CAP: f32 = 4.0;
/// `(NEGATIVE_X) | (NEGATIVE_Z << 3) | (POSITIVE_Y << 6)`: `EWeb_SetBoneAngles`' axes
/// (forward, right, up).
pub const BONE_ORIENT: u32 = 4 | (5 << 3) | (3 << 6);
/// `WP_NONE`, `WP_EMPLACED_GUN`, `WP_TURRET`; `MOD_TURBLAST`; `DAMAGE_DEATH_KNOCKBACK`;
/// `MASK_SHOT | CONTENTS_LIGHTSABER`; `EF_G2ANIMATING`; `ENTITYNUM_WORLD`.
pub const WP_EMPLACED_GUN: u32 = 17;
const WP_TURRET: u32 = 18;
use crate::means_of_death::MOD_TURBLAST;
const DAMAGE_DEATH_KNOCKBACK: u32 = 0x0080;
const SHOT_CLIP_MASK: u32 = 0x0004_1301;
const EF_G2ANIMATING: u32 = 1;
/// `EV_PLAY_EFFECT_ID`.
const EV_PLAY_EFFECT_ID: u32 = 69;
const ENTITYNUM_WORLD: u16 = 1_022;
/// `emplaced_eweb`'s spawnflags: `EMPLACED_INACTIVE`, `EMPLACED_FACING`,
/// `EWEB_INVULNERABLE`.
const EMPLACED_INACTIVE: i32 = 1;
const EMPLACED_FACING: i32 = 2;
const EWEB_INVULNERABLE: i32 = 4;

/// Wire fields (`entityState_t`, protocol 26).
mod es {
    pub const POS_BASE: [usize; 3] = [2, 1, 4];
    pub const APOS_BASE: [usize; 3] = [5, 3, 33];
    pub const ANGLES: [usize; 3] = [25, 9, 24];
    pub const ORIGIN: [usize; 3] = [11, 12, 13];
    pub const WEAPON: usize = 14;
    pub const LEGS_ANIM: usize = 16;
    pub const TORSO_ANIM: usize = 17;
    pub const EFLAGS: usize = 19;
    pub const TEAM_OWNER: usize = 21;
    pub const G2_RADIUS: usize = 38;
    pub const OWNER: usize = 40;
    pub const MODEL_INDEX: usize = 46;
    pub const TORSO_FLIP: usize = 50;
    pub const MODEL_GHOUL2: usize = 54;
    pub const ORIGIN2_X: usize = 56;
}

/// Who an E-Web is: the holdable a player set down, or a map's gun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `EWeb_Create`'s: `r.ownerNum`, the player that set it down and works it.
    Portable { owner: u16 },
    /// `SP_emplaced_eweb`'s, with its blast (`splashDamage`, `splashRadius`).
    Map {
        splash_damage: i32,
        splash_radius: i32,
    },
}

/// An E-Web as its spawn leaves it and its think keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct Eweb {
    /// Its entity number, which its gunner's `emplacedIndex` names.
    pub number: u16,
    /// Its wire state.
    pub state: EntityState,
    /// `r.currentOrigin`.
    pub origin: [f32; 3],
    /// `s.angles`: "angle 0" of its turn, the way it was set down.
    pub facing: [f32; 3],
    /// `angle`: the turn of its yaw bone, from [`Self::facing`].
    pub angle: f32,
    pub health: i32,
    pub max_health: i32,
    /// `FL_GODMODE` (a map gun's `INVULNERABLE`).
    pub invulnerable: bool,
    pub kind: Kind,
    /// Its gunner: always its owner for the holdable, whoever took it for a map gun.
    pub gunner: Option<u16>,
    /// `genericValue11`: the gunner's weapons, given back when it lets go.
    pub saved_weapons: u32,
    /// `genericValue8`: no turning or firing before this (the unfolding).
    pub ready_at: i32,
    /// `genericValue5`: no shot before this.
    pub next_fire: i32,
    /// `genericValue3`: the firing animation is on.
    pub firing: bool,
    /// A map gun's `delay`: when it was last taken (half a second before it may be again).
    pub taken_at: i32,
    /// `svFlags & SVF_INACTIVE` (a map gun's `INACTIVE`, never activated here).
    pub inactive: bool,
    /// A map gun's `FACING`: who takes it must face its way.
    pub needs_facing: bool,
    /// Gone (dead and blown up): freed by the caller.
    pub dead: bool,
}

impl Eweb {
    /// Its box and contents.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        match self.kind {
            Kind::Portable { .. } => BOUNDS,
            Kind::Map { .. } => MAP_BOUNDS,
        }
    }

    pub fn contents(&self) -> u32 {
        match self.kind {
            Kind::Portable { .. } => MASK_PLAYERSOLID,
            Kind::Map { .. } => CONTENTS_BODY,
        }
    }

    /// The holdable's owner (`r.ownerNum`), whose traces pass it.
    pub fn owner(&self) -> Option<u16> {
        match self.kind {
            Kind::Portable { owner } => Some(owner),
            Kind::Map { .. } => None,
        }
    }

    /// The bolt its gunner is placed from.
    pub fn place_bolt(&self) -> &'static str {
        match self.kind {
            Kind::Portable { .. } => PORTABLE_PLACE_BOLT,
            Kind::Map { .. } => MAP_PLACE_BOLT,
        }
    }

    /// `s.apos.trBase`: the angles its model is posed at.
    pub fn model_angles(&self) -> [f32; 3] {
        self.facing
    }

    /// `G_ScaleNetHealth`: the health bar the clients draw.
    pub fn publish_health(&mut self) {
        crate::map_turret_world::publish_net_health(&mut self.state, self.health, self.max_health);
    }
}

/// `EWeb_Create`'s two traces of the gun's box (`JP_Trace`, `MASK_PLAYERSOLID`, passing the
/// player) from the player at `origin` looking along `yaw`: 48 units ahead of it, 12 up,
/// clear all the way, then down onto the world within 18 units. Where it is set down, or
/// `None` ("can't spawn here").
pub fn place(
    origin: [f32; 3],
    yaw: f32,
    mut trace: impl FnMut([f32; 3], [f32; 3], [f32; 3], [f32; 3]) -> MovementTrace,
) -> Option<[f32; 3]> {
    let (forward, ..) = crate::npc_sniper::angle_vectors([0.0, yaw, 0.0]);
    let mut start = origin;
    start[2] += 12.0;
    let ahead: [f32; 3] = std::array::from_fn(|axis| start[axis] + 48.0 * forward[axis]);
    let (mins, maxs) = BOUNDS;
    let clear = trace(start, mins, maxs, ahead);
    if clear.all_solid || clear.start_solid || clear.fraction != 1.0 {
        return None;
    }
    let mut down = ahead;
    down[2] -= 18.0;
    let ground = trace(ahead, mins, maxs, down);
    if ground.start_solid
        || ground.all_solid
        || ground.fraction == 1.0
        || ground.entity_number < ENTITYNUM_WORLD
    {
        return None;
    }
    Some(ground.end_position)
}

/// The wire state both kinds share: a Ghoul2 model at `origin` facing `angles`.
fn base_state(number: u16, origin: [f32; 3], angles: [f32; 3], model_index: u16) -> EntityState {
    let mut state = EntityState::zero(0, &LEGACY_ENTITY_FIELDS);
    let _ = state.set_number(number);
    for axis in 0..3 {
        state.set_raw_field(es::POS_BASE[axis], origin[axis].to_bits());
        state.set_raw_field(es::ORIGIN[axis], origin[axis].to_bits());
        state.set_raw_field(es::ANGLES[axis], angles[axis].to_bits());
        state.set_raw_field(es::APOS_BASE[axis], angles[axis].to_bits());
    }
    state.set_raw_field(es::MODEL_INDEX, u32::from(model_index));
    state.set_raw_field(es::MODEL_GHOUL2, 1);
    // "360 degrees in either direction" (`BG_EmplacedView`'s constraint).
    state.set_raw_field(es::ORIGIN2_X, 360.0_f32.to_bits());
    state
}

/// The bones [`set_bone_angles`] turns, by their `CS_G2BONES` indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bones {
    pub yaw: u16,
    pub pitch: u16,
}

/// `EWeb_Create` after its traces: the gun `owner` sets down at `origin` along `yaw`,
/// owned by it (and its team), with the health its last one was left with (`health`, a
/// fresh one's when that is gone), its weapons kept, its bones unturned and its
/// unfolding begun.
#[allow(clippy::too_many_arguments)]
pub fn portable(
    number: u16,
    owner: u16,
    team: i32,
    origin: [f32; 3],
    yaw: f32,
    health: i32,
    weapons: u32,
    model_index: u16,
    bones: Bones,
    level_time: i32,
) -> Eweb {
    let facing = [0.0, yaw, 0.0];
    let mut state = base_state(number, origin, facing, model_index);
    // "for the sake of being able to differentiate client-side between this and an
    // emplaced gun".
    state.set_raw_field(es::WEAPON, 0);
    state.set_raw_field(es::OWNER, u32::from(owner));
    state.set_raw_field(es::TEAM_OWNER, team as u32);
    state.set_raw_field(es::G2_RADIUS, 128);
    let mut eweb = Eweb {
        number,
        state,
        origin,
        facing,
        angle: 0.0,
        health: if health <= 0 { HEALTH } else { health },
        max_health: HEALTH,
        invulnerable: false,
        kind: Kind::Portable { owner },
        gunner: Some(owner),
        saved_weapons: weapons,
        ready_at: level_time + UNFOLD_TIME,
        next_fire: 0,
        firing: false,
        taken_at: 0,
        inactive: false,
        needs_facing: false,
        dead: false,
    };
    eweb.publish_health();
    set_bone_angles(&mut eweb.state, bones.yaw, [0.0; 3]);
    set_bone_angles(&mut eweb.state, bones.pitch, [0.0; 3]);
    // "start the 'unfolding' anim".
    set_animation(&mut eweb.state, 4, 20);
    eweb
}

/// `SP_emplaced_eweb` for the map's `entity`: where the map puts it, 250 of health unless
/// the map says otherwise, a blast of 40 within 100, `INVULNERABLE` making it god-moded.
/// It keeps the weapon an emplaced gun has (`s.weapon`), as the single-player game does.
pub fn from_map(entity: &Entity, number: u16, model_index: u16, bones: Bones) -> Eweb {
    let int = |key: &str, default: i32| {
        entity
            .get(key)
            .map_or(default, |text| crate::userinfo::atoi(text.as_bytes()))
    };
    let origin = crate::fx_runner::vector(entity, "origin");
    let facing = crate::fx_runner::spawn_angles(entity);
    let spawnflags = int("spawnflags", 0);
    let mut state = base_state(number, origin, facing, model_index);
    // Told apart from the holdable by the clients only for its own aim view: the
    // multiplayer clients take a `WP_NONE` one for an E-Web, which this is.
    state.set_raw_field(es::WEAPON, 0);
    state.set_raw_field(es::OWNER, u32::from(sjk_protocol::ENTITY_NUMBER_NONE));
    state.set_raw_field(es::G2_RADIUS, 80);
    let health = int("health", 250);
    let mut eweb = Eweb {
        number,
        state,
        origin,
        facing,
        angle: 0.0,
        health,
        max_health: health,
        invulnerable: spawnflags & EWEB_INVULNERABLE != 0,
        kind: Kind::Map {
            splash_damage: last_int(entity, &["splashDamage"], 40),
            splash_radius: last_int(entity, &["splashRadius"], 100),
        },
        gunner: None,
        saved_weapons: 0,
        ready_at: 0,
        next_fire: 0,
        firing: false,
        taken_at: -UNFOLD_TIME,
        inactive: spawnflags & EMPLACED_INACTIVE != 0,
        needs_facing: spawnflags & EMPLACED_FACING != 0,
        dead: false,
    };
    eweb.publish_health();
    set_bone_angles(&mut eweb.state, bones.yaw, [0.0; 3]);
    set_bone_angles(&mut eweb.state, bones.pitch, [0.0; 3]);
    eweb
}

/// A key the lump gives, without regard to its case (`G_SpawnInt`), else `default`.
fn last_int(entity: &Entity, keys: &[&str], default: i32) -> i32 {
    entity
        .fields()
        .iter()
        .rev()
        .find(|(key, _)| keys.iter().any(|wanted| key.eq_ignore_ascii_case(wanted)))
        .map_or(default, |(_, value)| {
            crate::userinfo::atoi(value.as_bytes())
        })
}

/// What a player asking to take a map gun is, as `eweb_can_be_used` asks.
#[derive(Clone, Copy, Debug)]
pub struct Taker {
    pub view: [f32; 3],
    pub alive: bool,
    pub ducked: bool,
    pub riding: bool,
}

/// `eweb_can_be_used` (`g_emplaced.cpp:270-342`): a map gun alive, active and free, taken
/// by a living player standing up and on foot — facing its way (the `FACING` flag; the
/// single-player game asks it of the player whatever the flag) — half a second after it
/// was last taken.
pub fn can_take(eweb: &Eweb, taker: &Taker, level_time: i32) -> bool {
    if eweb.health <= 0 || eweb.dead || eweb.inactive || eweb.gunner.is_some() {
        return false;
    }
    if !matches!(eweb.kind, Kind::Map { .. }) || taker.riding || taker.ducked || !taker.alive {
        return false;
    }
    let (mut mine, ..) = crate::npc_sniper::angle_vectors(taker.view);
    let (mut its, ..) = crate::npc_sniper::angle_vectors(eweb.facing);
    mine[2] = 0.0;
    its[2] = 0.0;
    let dot = mine[0] * its[0] + mine[1] * its[1];
    // "Must be reasonably facing the way the gun points ( 90 degrees or so )".
    if dot < 0.75 {
        return false;
    }
    eweb.taken_at + UNFOLD_TIME < level_time
}

/// `EWeb_SetBoneAngles`' wire half: the bone's slot among the four, its angles and the
/// axes they turn it by. Nothing ("no free bone indexes") once four other bones hold them.
pub fn set_bone_angles(state: &mut EntityState, bone: u16, angles: [f32; 3]) {
    let _ = write_bone_angles(state, u32::from(bone), angles, BONE_ORIENT);
}

/// `EWeb_SetBoneAnim`'s wire half: the clients play frames `start` to `end` on the
/// model's root (`EF_G2ANIMATING`); asked again for the same frames, it starts over.
pub fn set_animation(state: &mut EntityState, start: u32, end: u32) {
    let flags = state.raw_field(es::EFLAGS).unwrap_or(0);
    state.set_raw_field(es::EFLAGS, flags | EF_G2ANIMATING);
    if state.raw_field(es::TORSO_ANIM) == Some(start) && state.raw_field(es::LEGS_ANIM) == Some(end)
    {
        let flip = state.raw_field(es::TORSO_FLIP).unwrap_or(0);
        state.set_raw_field(es::TORSO_FLIP, u32::from(flip == 0));
    } else {
        state.set_raw_field(es::TORSO_ANIM, start);
        state.set_raw_field(es::LEGS_ANIM, end);
    }
}

/// `EWebUpdateBoneAngles`' turns for a gunner looking along `view`: the yaw bone at most
/// four degrees nearer the view's yaw (from the way the gun was set down), and the pitch
/// bone at four fifths of the view's pitch. Its angles, the yaw's and the pitch's.
pub fn turn(eweb: &mut Eweb, view: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let ideal = angle_subtract(view[1], eweb.facing[1]);
    let increment = angle_subtract(ideal, eweb.angle).clamp(-TURN_CAP, TURN_CAP);
    eweb.angle += increment;
    let yaw = [eweb.angle, 0.0, 0.0];
    let pitch = [0.0, 0.0, angle_subtract(view[0], eweb.facing[0]) * 0.8];
    (yaw, pitch)
}

/// `EWebPositionUser`'s place for the gunner from the place bolt's world matrix: 32 units
/// back along the bolt's `-X`, at the gun's height and four units over it.
pub fn gunner_place(eweb: &Eweb, bolt: [[f32; 4]; 3]) -> [f32; 3] {
    let origin: [f32; 3] = std::array::from_fn(|row| bolt[row][3]);
    let back: [f32; 3] = std::array::from_fn(|row| -bolt[row][0]);
    let mut place: [f32; 3] = std::array::from_fn(|axis| origin[axis] + 32.0 * back[axis]);
    place[2] = eweb.origin[2] + 4.0;
    place
}

/// `EWebFire` from the muzzle bolt's world matrix: a turret bolt of the gunner's along the
/// bolt's `-Y`, from 16 units back into the gun's box "so it can't start inside other
/// solid things", flying through the gun itself, 20 of damage as `MOD_TURBLAST`, eight
/// bounces. The bolt and the muzzle (where the flash plays, and along what).
pub fn bolt(
    eweb: &Eweb,
    gunner: u16,
    muzzle: [[f32; 4]; 3],
    level_time: i32,
) -> (Missile, [f32; 3], [f32; 3]) {
    let point: [f32; 3] = std::array::from_fn(|row| muzzle[row][3]);
    let direction: [f32; 3] = std::array::from_fn(|row| -muzzle[row][1]);
    let start: [f32; 3] = std::array::from_fn(|axis| point[axis] - 16.0 * direction[axis]);
    let mut missile = crate::npc_machine_parts::create_missile(
        gunner,
        start,
        direction,
        MISSILE_SPEED,
        MISSILE_LIFE,
        level_time,
    );
    missile.state.set_raw_field(es::WEAPON, WP_TURRET);
    missile.state.set_raw_field(es::OWNER, u32::from(gunner));
    missile.damage = MISSILE_DAMAGE;
    missile.damage_flags = DAMAGE_DEATH_KNOCKBACK;
    missile.method_of_death = MOD_TURBLAST;
    missile.splash_method_of_death = MOD_TURBLAST;
    missile.clip_mask = SHOT_CLIP_MASK;
    missile.pass_through = Some(eweb.number);
    missile.bounce_count = 8;
    (missile, point, direction)
}

/// The blast a gun's end deals: the holdable's 90 within 128, a map gun's own.
pub fn blast(eweb: &Eweb) -> (i32, i32) {
    match eweb.kind {
        Kind::Portable { .. } => (DEATH_DAMAGE, DEATH_RADIUS),
        Kind::Map {
            splash_damage,
            splash_radius,
        } => (splash_damage, splash_radius),
    }
}

/// `G_Damage` on the gun: none for a god-moded one, the health down, the bar redrawn.
/// Whether it died of it.
pub fn damage(eweb: &mut Eweb, amount: i32) -> bool {
    if eweb.invulnerable || eweb.dead || eweb.health <= 0 || amount <= 0 {
        return false;
    }
    eweb.health -= amount;
    eweb.publish_health();
    eweb.health <= 0
}

/// `G_PlayEffect(effect, origin, angles)`: the holdable's death (`EFFECT_EXPLOSION_DETPACK`).
pub fn play_effect(
    effect: u32,
    origin: [f32; 3],
    angles: [f32; 3],
) -> crate::event_entity::EventEntity {
    crate::mines::effect(effect, origin, angles)
}

/// `G_PlayEffectID(effect, origin, angles)`: an effect by its `CS_EFFECTS` index.
pub fn play_effect_id(
    effect: u16,
    origin: [f32; 3],
    angles: [f32; 3],
) -> crate::event_entity::EventEntity {
    crate::event_entity::EventEntity {
        event: EV_PLAY_EFFECT_ID,
        ..crate::mines::effect(u32::from(effect), origin, angles)
    }
}

/// `G_Sound(ent, CHAN_AUTO, sound)` from `origin`.
pub fn sound(origin: [f32; 3], sound: u16) -> crate::event_entity::EventEntity {
    crate::weapon_fire::sound_event(origin, 0, sound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(fraction: f32, end: [f32; 3], entity: u16) -> MovementTrace {
        MovementTrace {
            fraction,
            end_position: end,
            plane_normal: [0.0, 0.0, 1.0],
            surface_flags: 0,
            start_solid: false,
            all_solid: false,
            entity_number: entity,
        }
    }

    #[test]
    fn it_is_set_down_48_units_ahead_on_the_world() {
        let mut calls = Vec::new();
        let placed = place([100.0, 0.0, 24.0], 0.0, |start, _, _, end| {
            calls.push((start, end));
            if calls.len() == 1 {
                trace(1.0, end, sjk_protocol::ENTITY_NUMBER_NONE)
            } else {
                trace(0.5, [end[0], end[1], 27.0], ENTITYNUM_WORLD)
            }
        });
        assert_eq!(placed, Some([148.0, 0.0, 27.0]));
        assert_eq!(calls[0].0, [100.0, 0.0, 36.0]);
        assert!((calls[0].1[0] - 148.0).abs() < 1e-4);
        assert_eq!(calls[1].1[2], calls[1].0[2] - 18.0);
    }

    #[test]
    fn it_is_not_set_down_in_a_wall_in_the_air_or_on_someone() {
        let blocked = |_, _, _, end: [f32; 3]| trace(0.3, end, ENTITYNUM_WORLD);
        assert_eq!(place([0.0; 3], 0.0, blocked), None);
        // Clear ahead, and nothing under it either.
        let air = |_, _, _, end| trace(1.0, end, sjk_protocol::ENTITY_NUMBER_NONE);
        assert_eq!(place([0.0; 3], 0.0, air), None);
        let mut first = true;
        let player = |_, _, _, end| {
            let (fraction, entity) = if first {
                (1.0, sjk_protocol::ENTITY_NUMBER_NONE)
            } else {
                (0.5, 3)
            };
            first = false;
            trace(fraction, end, entity)
        };
        assert_eq!(place([0.0; 3], 0.0, player), None);
    }

    #[test]
    fn the_top_turns_at_most_four_degrees_a_frame() {
        let bones = Bones { yaw: 1, pitch: 2 };
        let mut eweb = portable(70, 3, 1, [0.0; 3], 90.0, 0, 0, 5, bones, 0);
        let (yaw, pitch) = turn(&mut eweb, [10.0, 120.0, 0.0]);
        assert_eq!(yaw, [4.0, 0.0, 0.0]);
        assert_eq!(pitch, [0.0, 0.0, 8.0]);
        for _ in 0..20 {
            let _ = turn(&mut eweb, [10.0, 120.0, 0.0]);
        }
        assert!((eweb.angle - 30.0).abs() < 1e-4);
        let (yaw, _) = turn(&mut eweb, [0.0, 80.0, 0.0]);
        assert!((yaw[0] - 26.0).abs() < 1e-4);
    }

    #[test]
    fn a_new_one_has_full_health_unless_the_last_one_was_hurt() {
        let bones = Bones { yaw: 1, pitch: 2 };
        assert_eq!(
            portable(70, 3, 1, [0.0; 3], 0.0, -1, 0, 5, bones, 0).health,
            HEALTH
        );
        assert_eq!(
            portable(70, 3, 1, [0.0; 3], 0.0, 0, 0, 5, bones, 0).health,
            HEALTH
        );
        assert_eq!(
            portable(70, 3, 1, [0.0; 3], 0.0, 120, 0, 5, bones, 0).health,
            120
        );
    }

    #[test]
    fn its_state_turns_both_bones_and_unfolds() {
        let bones = Bones { yaw: 7, pitch: 8 };
        let mut eweb = portable(70, 3, 1, [0.0; 3], 0.0, 0, 0, 5, bones, 1_000);
        assert_eq!(eweb.state.bone_index(0), 7);
        assert_eq!(eweb.state.bone_index(1), 8);
        assert_eq!(eweb.state.bone_orient(), BONE_ORIENT);
        assert_eq!(eweb.state.weapon(), 0);
        assert_eq!(eweb.state.torso_animation(), 4);
        assert_eq!(eweb.state.leg_animation(), 20);
        assert_eq!(eweb.ready_at, 1_500);
        set_bone_angles(&mut eweb.state, 8, [0.0, 0.0, 12.0]);
        assert_eq!(eweb.state.bone_angles(1), [0.0, 0.0, 12.0]);
        set_animation(&mut eweb.state, 2, 4);
        let flip = eweb.state.torso_flip();
        set_animation(&mut eweb.state, 2, 4);
        assert_ne!(eweb.state.torso_flip(), flip);
    }

    #[test]
    fn its_bolt_leaves_the_muzzle_along_minus_y() {
        let bones = Bones { yaw: 1, pitch: 2 };
        let eweb = portable(70, 3, 1, [0.0; 3], 0.0, 0, 0, 5, bones, 0);
        // A muzzle at (10, 0, 40) whose -Y is the world's +X.
        let muzzle = [
            [0.0, -1.0, 0.0, 10.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 40.0],
        ];
        let (missile, point, direction) = bolt(&eweb, 3, muzzle, 0);
        assert_eq!(point, [10.0, 0.0, 40.0]);
        assert_eq!(direction, [1.0, 0.0, 0.0]);
        assert_eq!(missile.owner, 3);
        assert_eq!(missile.pass_through, Some(70));
        assert_eq!(missile.damage, 20);
        assert_eq!(missile.method_of_death, MOD_TURBLAST);
    }

    #[test]
    fn hurting_it_kills_it_unless_it_is_invulnerable() {
        let bones = Bones { yaw: 1, pitch: 2 };
        let mut eweb = portable(70, 3, 1, [0.0; 3], 0.0, 0, 0, 5, bones, 0);
        assert!(!damage(&mut eweb, 150));
        assert_eq!(eweb.state.health(), 50);
        assert!(damage(&mut eweb, 60));
        eweb.invulnerable = true;
        eweb.health = 10;
        assert!(!damage(&mut eweb, 60));
        assert_eq!(eweb.health, 10);
    }

    #[test]
    fn a_map_gun_is_taken_facing_its_way() {
        let entity = Entity::from_fields(vec![
            ("classname".into(), "emplaced_eweb".into()),
            ("origin".into(), "10 20 30".into()),
            ("angle".into(), "180".into()),
            ("health".into(), "500".into()),
            ("spawnflags".into(), "6".into()),
        ]);
        let eweb = from_map(&entity, 80, 4, Bones { yaw: 1, pitch: 2 });
        assert_eq!(eweb.origin, [10.0, 20.0, 30.0]);
        assert_eq!(eweb.health, 500);
        assert!(eweb.invulnerable && eweb.needs_facing);
        let taker = |yaw: f32| Taker {
            view: [0.0, yaw, 0.0],
            alive: true,
            ducked: false,
            riding: false,
        };
        assert!(can_take(&eweb, &taker(180.0), 1_000));
        assert!(!can_take(&eweb, &taker(0.0), 1_000));
        let mut ducked = taker(180.0);
        ducked.ducked = true;
        assert!(!can_take(&eweb, &ducked, 1_000));
    }
}
