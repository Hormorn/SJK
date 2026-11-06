//! The player state side of a server mod's emote commands (JA+'s, VisionJK
//! `codemp/game/japlus/g_japlus_cmds.c`): who may act, a pose played as a taunt, the
//! knock-me-down, the show-off `amflip` picks. The server (`sjk-dedicated`'s
//! `bridge_mod_commands.rs`) gates and answers them; the movement holds the poses
//! ([`crate::pmove_japlus_emotes`]).

use crate::event_entity::EventEntity;
use crate::pmove_anim::AnimationLengths;
use sjk_protocol::PlayerState;

const PS_EFLAGS: usize = 17;
const PS_SABER_ANIM_LEVEL: usize = 23;
const PS_FORCE_HAND_EXTEND: usize = 80;
const PS_FORCE_DODGE_ANIM: usize = 89;
const EF_DEAD: u32 = 1;
const PM_DEAD: u8 = 5;
const HANDEXTEND_NONE: u32 = 0;
const HANDEXTEND_KNOCKDOWN: u32 = 8;
const HANDEXTEND_TAUNT: u32 = 10;
const ENTITY_NUMBER_NONE: u16 = 1_023;
/// `PDSOUND_FORCEJUMP`.
const PDSOUND_FORCEJUMP: u32 = 5;
/// `SS_DUAL`, `SS_STAFF`.
const SS_DUAL: u8 = 6;
const SS_STAFF: u8 = 7;

/// `JP_CanAct` (`g_japlus_cmds.c:39-51`) past its connection and team checks: not in a
/// melee grapple, a saber attack or transition, nor dead. (JA+'s sleeping and knocked
/// down admin states are not built.)
pub fn can_act(state: &PlayerState, health: i32) -> bool {
    !grappling(state)
        && !crate::saber_rules::in_attack(state.saber_move())
        && !crate::saber_rules::in_transition_any(state.saber_move())
        && state.movement_type() != PM_DEAD
        && health > 0
        && state.stats[0] as i32 > 0
        && state.raw_field(PS_EFLAGS).unwrap_or(0) & EF_DEAD == 0
}

/// `BG_InGrappleMove` of the legs or the torso: in a melee grapple.
pub fn grappling(state: &PlayerState) -> bool {
    crate::prediction_policy::in_grapple_move(state.leg_animation())
        || crate::prediction_policy::in_grapple_move(state.torso_animation())
}

/// `JP_Emote`'s last check (`g_japlus_cmds.c:106-109`) but the jetpack's: nothing else
/// in the hands, not getting up nor lying.
pub fn may_pose(state: &PlayerState) -> bool {
    let (legs, torso) = (state.leg_animation(), state.torso_animation());
    state.raw_field(PS_FORCE_HAND_EXTEND).unwrap_or(0) == HANDEXTEND_NONE
        && !crate::knockdown::in_get_up(legs)
        && !crate::knockdown::in_get_up(torso)
        && !crate::pmove_roll_anim::on_ground(legs)
        && !crate::pmove_roll_anim::on_ground(torso)
}

/// `JP_EmoteExec` (`g_japlus_cmds.c:65-69`): `animation` played as a taunt for its
/// length (`forceHandExtendTime`, which the server keeps in `hand_extend_time`).
pub fn play_pose(
    state: &mut PlayerState,
    hand_extend_time: &mut i32,
    animation: u16,
    lengths: &dyn AnimationLengths,
    level_time: i32,
) {
    state.set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_TAUNT);
    state.set_raw_field(PS_FORCE_DODGE_ANIM, u32::from(animation));
    *hand_extend_time = level_time + lengths.length_ms(animation).unwrap_or(0);
}

/// `amknockmedown`'s fall (`g_japlus_cmds.c:446-457`), once its gates have passed: down
/// for 700 ms with a hop, unless a hand extend's time still runs (pushed to 200 ms from
/// now). The Force jump's sound to raise when it falls.
pub fn knock_me_down(
    state: &mut PlayerState,
    memory: &mut crate::knockdown::Knockdown,
    level_time: i32,
) -> Option<EventEntity> {
    if level_time < memory.hand_extend_time {
        memory.hand_extend_time = level_time + 200;
    }
    let hand = state.raw_field(PS_FORCE_HAND_EXTEND).unwrap_or(0);
    if !crate::knockdown::knockdownable(state)
        || memory.hand_extend_time >= level_time
        || (hand == HANDEXTEND_KNOCKDOWN && level_time >= memory.hand_extend_time + 200)
    {
        return None;
    }
    state.set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_KNOCKDOWN);
    state.set_raw_field(PS_FORCE_DODGE_ANIM, 0);
    let mut velocity = state.velocity();
    velocity[2] = 400.0;
    state.set_velocity(velocity);
    memory.hand_extend_time = level_time + 700;
    memory.quicker_getup = false;
    Some(crate::knockdown::predef_sound(
        state.origin(),
        PDSOUND_FORCEJUMP,
    ))
}

/// `amflip`'s show-off (`g_japlus_cmds.c:359-386`) for the style shown (`saberAnimLevel`)
/// and the one chosen (`saberAnimLevelBase`): `staff_draw` is `Q_irand(0, 10) >= 4`, drawn
/// by the caller. JA+'s own draw and holster animations (`jp_newGLAAnims`) are not
/// played, as SJK announces them off.
pub fn flip_animation(state: &PlayerState, base_style: u8, staff_draw: bool) -> Option<u16> {
    let style = state.raw_field(PS_SABER_ANIM_LEVEL).unwrap_or(0) as u8;
    let holstered_now = state.saber_holstered();
    let mut name = "BOTH_STAND1TO2";
    let mut holstered = holstered_now;
    if style == SS_STAFF {
        name = "BOTH_SHOWOFF_STAFF";
        holstered = 0;
        if holstered_now != 0 {
            holstered = holstered_now;
            if staff_draw {
                name = "BOTH_S1_S7";
            }
        }
    }
    if style == SS_DUAL {
        name = "BOTH_SHOWOFF_FAST";
    }
    if base_style == SS_STAFF && style != SS_STAFF {
        name = "BOTH_S1_S7";
    }
    if holstered == 0 && style != SS_STAFF && style != SS_DUAL {
        name = "BOTH_STAND2TO1";
    }
    crate::legacy_animation_index(name).map(|index| index as u16)
}

/// Whether `amflip` goes on to its pose (`g_japlus_cmds.c:413-414`): on the ground.
pub fn grounded(state: &PlayerState) -> bool {
    state.ground_entity_num() != ENTITY_NUMBER_NONE
}

/// `BG_SabersOff`: every blade off, the second saber's or staff's too.
pub fn sabers_off(state: &PlayerState, base_style: u8) -> bool {
    let holstered = state.saber_holstered();
    holstered != 0 && (!matches!(base_style, SS_DUAL | SS_STAFF) || holstered >= 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animation(name: &str) -> u16 {
        crate::legacy_animation_index(name).unwrap() as u16
    }

    fn standing() -> PlayerState {
        let mut state = PlayerState::default();
        state.stats[0] = 100;
        state.set_ground_entity_num(1_022);
        state
    }

    #[test]
    fn the_dead_and_the_busy_do_not_pose() {
        let state = standing();
        assert!(can_act(&state, 100) && may_pose(&state));
        assert!(!can_act(&state, 0));
        let mut down = standing();
        down.set_raw_field(13, u32::from(animation("BOTH_KNOCKDOWN1")));
        assert!(!may_pose(&down));
        let mut taunting = standing();
        taunting.set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_TAUNT);
        assert!(!may_pose(&taunting));
    }

    #[test]
    fn knock_me_down_waits_for_a_hand_extend() {
        let mut state = standing();
        let mut memory = crate::knockdown::Knockdown::default();
        assert!(knock_me_down(&mut state, &mut memory, 1_000).is_some());
        assert_eq!(
            state.raw_field(PS_FORCE_HAND_EXTEND),
            Some(HANDEXTEND_KNOCKDOWN)
        );
        assert_eq!(
            (state.velocity()[2], memory.hand_extend_time),
            (400.0, 1_700)
        );
        // Again while it runs: put off to 200 ms from now, and nothing more.
        assert!(knock_me_down(&mut state, &mut memory, 1_100).is_none());
        assert_eq!(memory.hand_extend_time, 1_300);
    }

    #[test]
    fn the_flip_turns_the_saber_by_style() {
        let mut state = standing();
        state.set_raw_field(PS_SABER_ANIM_LEVEL, 2);
        // On: the holster; off: the draw.
        assert_eq!(
            flip_animation(&state, 2, false),
            Some(animation("BOTH_STAND2TO1"))
        );
        state.set_raw_field(81, 2);
        assert_eq!(
            flip_animation(&state, 2, false),
            Some(animation("BOTH_STAND1TO2"))
        );
        // The staff off: its draw on the draw's chance, else its show-off.
        state.set_raw_field(PS_SABER_ANIM_LEVEL, 7);
        assert_eq!(
            flip_animation(&state, 7, true),
            Some(animation("BOTH_S1_S7"))
        );
        assert_eq!(
            flip_animation(&state, 7, false),
            Some(animation("BOTH_SHOWOFF_STAFF"))
        );
        state.set_raw_field(PS_SABER_ANIM_LEVEL, 6);
        assert_eq!(
            flip_animation(&state, 6, false),
            Some(animation("BOTH_SHOWOFF_FAST"))
        );
        assert!(sabers_off(&state, 6) && !sabers_off(&standing(), 2));
    }
}
