//! `CheckSaberDamage` ([`check`]) on one trace through a stand-in world: the base game's
//! SP-style damage, unchanged by default and under JA+'s `jp_alterDMG 0`, and each JA+
//! rule the running mod switches on (VisionJK `w_saber.c:4211-4446, 4626, 4726, 4818`).

use super::*;
use crate::player_death::Rng;
use crate::saber_clash::{BladeHistory, SaberSet};
use crate::saber_damage_japlus::JaPlusSaberDamage;

/// The victim's number, and the world's.
const VICTIM: u16 = 1;
const WORLD: u16 = ENTITY_NUMBER_WORLD;
/// `LS_A_TL2BR`, `LS_READY`; `BOTH_A3_T__B_`; `WEAPON_FIRING`; `SS_STRONG`.
const ATTACK_MOVE: u32 = 4;
const STRONG_ATTACK: u16 = 280;
const FIRING: u8 = 3;
const STRONG: i32 = 3;

/// A world where every trace meets `hit` half-way, which is a client or the world.
struct World {
    rng: Rng,
    hit: u16,
    victim: SaberVictim,
    owner: Option<Fighter>,
    owners_asked: std::cell::Cell<usize>,
}

impl World {
    fn new(hit: u16) -> Self {
        Self {
            rng: Rng(7),
            hit,
            victim: SaberVictim {
                client: true,
                takes_damage: true,
                health: 100,
                ..SaberVictim::default()
            },
            owner: None,
            owners_asked: std::cell::Cell::new(0),
        }
    }
}

impl crate::saber_clash::ClashWorld for World {
    fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }
    fn check_lock(&mut self, _: u16, _: u16) -> bool {
        false
    }
    fn knock_out(&mut self, _: u16, _: [f32; 3]) -> bool {
        false
    }
    fn smash(&mut self, _: u16, _: u16, _: i32) -> bool {
        false
    }
}

impl SaberTargets for World {
    fn trace(
        &mut self,
        start: [f32; 3],
        _: [f32; 3],
        _: [f32; 3],
        end: [f32; 3],
        _: u32,
    ) -> MovementTrace {
        MovementTrace {
            fraction: 0.5,
            end_position: std::array::from_fn(|axis| (start[axis] + end[axis]) / 2.0),
            plane_normal: [0.0, 0.0, 1.0],
            surface_flags: 0,
            start_solid: false,
            all_solid: false,
            entity_number: self.hit,
        }
    }
    fn collide(&mut self, _: u16, _: [f32; 3], _: [f32; 3], _: f32) -> Ghoul2Answer {
        Ghoul2Answer::NoModel
    }
    fn victim(&self, number: u16) -> Option<SaberVictim> {
        (number == VICTIM).then_some(self.victim)
    }
    fn saber_owner(&self, _: u16) -> Option<Fighter> {
        self.owners_asked.set(self.owners_asked.get() + 1);
        self.owner
    }
    fn set_saber_owner(&mut self, _: &Fighter) {}
    fn wall_bounce(&mut self, _: &WallBounce) {}
}

/// A strong attack half-way through its 1000 ms (JA+'s ramp at its peak), in FFA.
fn swinger(ja_plus: JaPlusSaberDamage) -> Swinger {
    let blade = BladeHistory {
        length: 40.0,
        ..BladeHistory::default()
    };
    Swinger {
        fighter: Fighter {
            number: 0,
            saber_move: ATTACK_MOVE,
            weapon_state: FIRING,
            torso: STRONG_ATTACK,
            torso_timer: 500,
            torso_length: 1000,
            style: STRONG,
            blade,
            sabers: SaberSet::single(blade),
            has_saber: true,
            ..Fighter::default()
        },
        level_time: 1000,
        weapon_time: 100,
        weapon: 3,
        hilt_speed_scales: [1.0; 2],
        ja_plus,
        ..Swinger::default()
    }
}

/// One trace 40 units long; the damage the victim takes and the world after it.
fn cut(swinger: &mut Swinger, world: &mut World) -> Option<i32> {
    let mut hits = SaberHits::default();
    check(
        &mut hits,
        swinger,
        &mut SweepState::default(),
        [0.0; 3],
        [0.0, 0.0, 40.0],
        false,
        world,
    );
    hits.blows(&|number| number == VICTIM)
        .find(|blow| blow.victim == VICTIM)
        .map(|blow| blow.damage)
}

fn ja_plus() -> JaPlusSaberDamage {
    JaPlusSaberDamage {
        enabled: true,
        ..JaPlusSaberDamage::default()
    }
}

fn move_table() -> JaPlusSaberDamage {
    JaPlusSaberDamage {
        move_damage: true,
        ..ja_plus()
    }
}

#[test]
fn the_base_game_cuts_as_before() {
    // 2.5 a level of the strong style's 3, doubled outside the duels: 15 over a 40 unit
    // trace that struck half-way, ceil(15 * 40 * 0.5 * 0.1 * 0.33) = 10, and 1.5 times
    // that on a client.
    let mut world = World::new(VICTIM);
    assert_eq!(
        cut(&mut swinger(JaPlusSaberDamage::default()), &mut world),
        Some(15)
    );
    // A saber in the victim's hand changes nothing, nor the chat bubble.
    world.victim.saber_in_hand = true;
    let mut talking = swinger(JaPlusSaberDamage::default());
    talking.talking = true;
    assert_eq!(cut(&mut talking, &mut world), Some(15));
    assert_eq!(world.rng.0, 7);
}

#[test]
fn jp_alter_dmg_0_keeps_the_sp_style_damage() {
    let mut world = World::new(VICTIM);
    world.victim.saber_in_hand = true;
    assert_eq!(cut(&mut swinger(ja_plus()), &mut world), Some(15));
}

#[test]
fn jp_alter_dmg_takes_the_moves_damage_from_the_table() {
    // The strong style's plain attack at its peak: 110, and 1.5 times on a client.
    let mut world = World::new(VICTIM);
    assert_eq!(cut(&mut swinger(move_table()), &mut world), Some(165));
    // With `jp_alterDMG 1`, half of it on a player holding a saber.
    world.victim.saber_in_hand = true;
    let halved = JaPlusSaberDamage {
        halved_on_sabers: true,
        ..move_table()
    };
    assert_eq!(cut(&mut swinger(halved), &mut world), Some(82));
}

#[test]
fn a_thrown_saber_keeps_the_sp_damage_scaled() {
    let thrown = |rules| {
        let mut swinger = swinger(rules);
        swinger.thrown = ThrownSwing {
            in_flight: true,
            first_saber: true,
            going_out: true,
            level: 3,
        };
        swinger
    };
    // 2.5 a throw level, doubled: 15, as the base game's.
    let mut world = World::new(VICTIM);
    world.victim.saber_in_hand = true;
    assert_eq!(
        cut(&mut thrown(JaPlusSaberDamage::default()), &mut world),
        Some(15)
    );
    // `jp_SaberThrowScaleDMG 0.5`: 10 is 5 before the client's 1.5; not halved on a saber.
    let scaled = JaPlusSaberDamage {
        throw_scale: Some(0.5),
        halved_on_sabers: true,
        ..move_table()
    };
    assert_eq!(cut(&mut thrown(scaled), &mut world), Some(7));
}

#[test]
fn a_ja_plus_swinger_under_the_chat_bubble_cuts_nobody() {
    let mut talking = swinger(ja_plus());
    talking.talking = true;
    assert_eq!(cut(&mut talking, &mut World::new(VICTIM)), None);
    // Nor marks its next touch's delay.
    assert_eq!(talking.idle_wound, 0);
}

#[test]
fn jp_allow_saber_touch_dmg_hurts_with_any_touch() {
    let idle = |rules| Swinger {
        fighter: Fighter {
            saber_move: 1,
            weapon_state: 0,
            ..swinger(rules).fighter
        },
        ..swinger(rules)
    };
    assert_eq!(
        cut(
            &mut idle(JaPlusSaberDamage::default()),
            &mut World::new(VICTIM)
        ),
        None
    );
    assert_eq!(cut(&mut idle(ja_plus()), &mut World::new(VICTIM)), None);
    let touch = JaPlusSaberDamage {
        touch_damage: true,
        ..ja_plus()
    };
    assert_eq!(cut(&mut idle(touch), &mut World::new(VICTIM)), Some(1));
}

#[test]
fn jp_reduce_saber_block_draws_and_lets_blades_pass() {
    // Whatever else the blade meets draws the chance on a JA+ server, before it is known
    // to be a saber; the base game draws nothing.
    let mut base = World::new(WORLD);
    let _ = cut(&mut swinger(JaPlusSaberDamage::default()), &mut base);
    assert_eq!((base.rng.0, base.owners_asked.get()), (7, 1));
    let mut world = World::new(WORLD);
    let _ = cut(&mut swinger(ja_plus()), &mut world);
    let mut drawn = Rng(7);
    let _ = drawn.irand(1, 100);
    assert_eq!(world.rng.0, drawn.0);
    // At 0 % the saber is always looked for, at 100 % never: the blades pass.
    assert_eq!(world.owners_asked.get(), 1);
    let always = JaPlusSaberDamage {
        reduce_block: 100,
        ..ja_plus()
    };
    let mut world = World::new(WORLD);
    let _ = cut(&mut swinger(always), &mut world);
    assert_eq!(world.owners_asked.get(), 0);
}
