//! When the client stops predicting its own movement and shows the server's instead
//! (`CG_PredictPlayerState`, JoF EternalJK `codemp/cgame/cg_predict.c:1160-1230`).
//!
//! Three situations take the snapshot's state: `cg_noPredict`, a synchronous server and,
//! on a JA+ server, the added side/back kick (the server runs knockdown and get-up rules
//! the client's pmove does not replicate, so every input mispredicts and the correction
//! shakes the camera). Whether the view's angles follow the server too (`grabAngles`)
//! is part of the answer.

use crate::knockdown::in_knockdown;
use crate::legacy_animation_index;
use crate::pmove_japlus::JaPlusRules;
use sjk_protocol::PlayerState;

/// `HANDEXTEND_KNOCKDOWN`.
const HANDEXTEND_KNOCKDOWN: u8 = 8;
/// `PM_FREEZE`: a JA+ emote holds the body.
const PM_FREEZE: u8 = 6;
/// `ps.forceDodgeAnim` (player-state field 89) in a JA+ added kick's victim.
const KICK_DODGE_ANIMS: [u32; 2] = [4, 5];

/// `GHOST_KNOWN_FLAG` (`q_shared.h`): the JA+ server sets this top bit of
/// `fd.forcePowersKnown` every think while it lets the player through other players.
pub const GHOST_KNOWN_FLAG: u32 = 1 << 31;

/// How the local view is presented instead of predicted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interpolation {
    /// The view takes the server's origin; the mouse keeps control of the angles.
    KeepAngles,
    /// The view takes the server's origin and angles.
    ServerAngles,
}

/// `cg_noPredict`'s values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NoPredict(pub i64);

fn index(name: &str) -> u16 {
    legacy_animation_index(name).map_or(u16::MAX, |index| index as u16)
}

fn is_one_of(animation: u16, names: &[&str]) -> bool {
    names.iter().any(|name| index(name) == animation)
}

/// `CG_InKnockDownState`: from the moment of the knockdown until the get-up has played
/// out, on either channel, including JA+'s kick falls.
fn in_knockdown_state(player: &PlayerState) -> bool {
    const FALLS: [&str; 3] = [
        "BOTH_BACK_FALLING",
        "BOTH_BACK_FALLING_GETUP",
        "BOTH_BACK_FALLING_GETUP_SLOW",
    ];
    let (legs, torso) = (player.leg_animation(), player.torso_animation());
    player.force_hand_extend() == HANDEXTEND_KNOCKDOWN
        || in_knockdown(legs)
        || (in_knockdown(torso) && player.torso_timer() > 0)
        || is_one_of(legs, &FALLS)
        || is_one_of(torso, &FALLS)
}

/// `CG_JAPlusViewLockedState`: JA+ rewrites `delta_angles` every server frame while the
/// player is kicked down, getting up, kissing or hanging from a ledge.
pub fn japlus_view_locked(player: &PlayerState) -> bool {
    japlus_view_held(player) || japlus_knockdown(player)
}

/// [`japlus_view_locked`]'s knockdowns and their get-ups, which some JA+ servers leave
/// free ([`JaPlusRules::free_knockdown_view`]).
fn japlus_knockdown(player: &PlayerState) -> bool {
    player.torso_animation() == index("BOTH_GETUP1") || in_knockdown_state(player)
}

/// `HANDEXTEND_PRETHROWN` and `HANDEXTEND_POSTTHROWN`: the victim of a melee throw, whose
/// view the server turns to the thrower every think (`g_active.c` `ClientThink_real`,
/// `SetClientViewAngle( ent, otherAngles )`).
const THROWN_HAND_EXTENDS: [u8; 2] = [13, 14];

/// `BG_InGrappleMove` (`bg_panimate.c`): grabbing, beating or beaten in a melee grapple,
/// which `PmoveSingle` holds the view through (`PM_SetPMViewAngle`).
const GRAPPLE_ANIMS: [&str; 9] = [
    "BOTH_KYLE_GRAB",
    "BOTH_KYLE_MISS",
    "BOTH_KYLE_PA_1",
    "BOTH_KYLE_PA_2",
    "BOTH_KYLE_PA_3",
    "BOTH_PLAYER_PA_1",
    "BOTH_PLAYER_PA_2",
    "BOTH_PLAYER_PA_FLY",
    "BOTH_PLAYER_PA_3",
];

/// `BG_InGrappleMove` for one animation: by name (one lookup), as the view asks it every
/// frame.
pub(crate) fn in_grapple_move(animation: u16) -> bool {
    crate::legacy_animation_name(usize::from(animation))
        .is_some_and(|name| GRAPPLE_ANIMS.contains(&name))
}

/// Whether the server holds the player's view this snapshot: a melee grapple or throw, a
/// saber lock (`PM_SetPMViewAngle` while `saberLockTime` runs) and, on a JA+ server
/// (`japlus`, the session's rules, asked only when it matters), [`japlus_view_locked`]'s
/// kisses, kicks, knockdowns, get-ups and ledges. The server rewrites `delta_angles`
/// every frame then, so a view turned by the mouse, or commands whose angles a toy sets,
/// fight it: the camera shakes or spins (hub reports #139, #140, #146).
///
/// A server known to leave the view free through knockdowns and get-ups
/// ([`JaPlusRules::free_knockdown_view`]) holds only the rest: counted as held there,
/// `cg_heldViewLook 0` locked the camera through every knockdown (Kushina's hub report
/// #169).
///
/// A JA+ emote (`PM_FREEZE`) holds the body, not the view: `PmoveSingle` turns the view
/// before it returns for a frozen player (`bg_pmove.c:13270,13328`), and some emotes play
/// a knockdown's animation (JA++'s `amneo` is `BOTH_FORCE_GETUP_B4`, `g_cmds.cpp`), which
/// [`japlus_view_locked`] would take for a knockdown: with `cg_heldViewLook 0` the mouse
/// stopped turning the camera through the emote (vini's hub report #166).
pub fn server_holds_view(player: &PlayerState, japlus: impl FnOnce() -> JaPlusRules) -> bool {
    if player.movement_type() == PM_FREEZE {
        return false;
    }
    let grapple = in_grapple_move;
    if THROWN_HAND_EXTENDS.contains(&player.force_hand_extend())
        || grapple(player.leg_animation())
        || grapple(player.torso_animation())
        || player.saber_lock_time() >= player.command_time() && player.saber_lock_time() > 0
    {
        return true;
    }
    if !japlus_view_locked(player) {
        return false;
    }
    let rules = japlus();
    rules.enabled && (rules.holds_knockdown_view() || japlus_view_held(player))
}

/// [`japlus_view_locked`] less the knockdowns and their get-ups: the kisses, ledges, the
/// stab and the backflip kick's victim, held on every JA+ server.
fn japlus_view_held(player: &PlayerState) -> bool {
    let (legs, torso) = (player.leg_animation(), player.torso_animation());
    let backflip_victim = index("BOTH_JUMP_BACKFLIP_ATCKEE");
    legs == backflip_victim
        || torso == backflip_victim
        || torso == index("BOTH_NEW_STABEE")
        || (legs >= index("BOTH_KISSEE") && legs <= index("BOTH_LEDGE_MERCPULL"))
}

/// `CG_InJAPlusSpecialKickState`: the victim of JA+'s added side and back kicks.
pub fn japlus_special_kick(player: &PlayerState) -> bool {
    const KICK_ANIMS: [&str; 5] = [
        "BOTH_BACK_FALLING",
        "BOTH_BACK_FALLING_GETUP",
        "BOTH_BACK_FALLING_GETUP_SLOW",
        "BOTH_JUMP_BACKFLIP_ATCKEE",
        "BOTH_JUMP_BACKFLIP_ATCKEE_FALL",
    ];
    KICK_DODGE_ANIMS.contains(&player.raw_field(89).unwrap_or(0))
        || is_one_of(player.leg_animation(), &KICK_ANIMS)
        || is_one_of(player.torso_animation(), &KICK_ANIMS)
}

/// The mask for a player the server walks through others: `CONTENTS_BODY` and
/// `CONTENTS_PLAYERCLIP` leave the pmove's trace mask (`cg_predict.c:1299-1313`).
pub fn passes_through_players(player_known_powers: u32) -> bool {
    player_known_powers & GHOST_KNOWN_FLAG != 0
}

/// Whether the local view is the server's this frame, and how. `None` predicts.
///
/// `following` is `PMF_FOLLOW`; `synchronous` is `g_synchronousClients`; `japlus` tells
/// whether the server is JA+, asked only when the player's state makes it matter.
pub fn interpolation(
    player: &PlayerState,
    no_predict: NoPredict,
    following: bool,
    synchronous: bool,
    japlus: impl FnOnce() -> bool,
) -> Option<Interpolation> {
    if following || no_predict.0 == 2 {
        return Some(Interpolation::ServerAngles);
    }
    if no_predict.0 != 0 || synchronous {
        // While JA+ holds the view locked it rewrites `delta_angles` each server frame, and
        // re-applying the live mouse on top of a stale lock snaps the view back with every
        // snapshot: take the server's angles for that window.
        return Some(
            if no_predict.0 == 1 && japlus_view_locked(player) && japlus() {
                Interpolation::ServerAngles
            } else {
                Interpolation::KeepAngles
            },
        );
    }
    (japlus_special_kick(player) && japlus()).then_some(Interpolation::ServerAngles)
}

/// `CG_FakeNoclip_f`'s and `CG_PredictPlayerState`'s gate: only a live player on foot,
/// and not in a duel.
///
/// The flight sends the server a still player with the talk balloon up, and a server
/// that spares chatting players their damage (JoF's JA+ servers) then spared a duellist
/// all of the opponent's (Nib's hub report #209, 30/10/2026). jaPRO leaves duels out of
/// its own chat protection (`g_godChat`, `!ps.duelInProgress`); SJK keeps the flight out
/// of them instead, so a duel starting ends it.
pub fn fake_noclip_allowed(player: &PlayerState, following: bool) -> bool {
    const PM_DEAD: u8 = 5;
    !(following
        || player.duel_in_progress()
        || player.is_spectator()
        || player.movement_type() == PM_DEAD
        || player.movement_type() == crate::PM_INTERMISSION
        || player.vehicle_entity_num() != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player_with(fields: &[(usize, u32)]) -> PlayerState {
        let mut player = PlayerState::default();
        for &(field, value) in fields {
            player.set_raw_field(field, value);
        }
        player
    }

    fn animation(name: &str) -> u32 {
        legacy_animation_index(name).expect("known animation") as u32
    }

    fn ja_plus() -> JaPlusRules {
        JaPlusRules {
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn ghost_flag_is_the_top_known_power_bit() {
        assert!(passes_through_players(1 << 31));
        assert!(!passes_through_players((1 << 18) - 1));
    }

    #[test]
    fn predicting_is_the_default() {
        let player = PlayerState::default();
        assert_eq!(
            interpolation(&player, NoPredict(0), false, false, || true),
            None
        );
    }

    #[test]
    fn no_predict_two_and_following_take_the_server_angles() {
        let player = PlayerState::default();
        let server = Some(Interpolation::ServerAngles);
        assert_eq!(
            interpolation(&player, NoPredict(2), false, false, || false),
            server
        );
        assert_eq!(
            interpolation(&player, NoPredict(0), true, false, || false),
            server
        );
    }

    #[test]
    fn no_predict_one_and_synchronous_keep_the_mouse() {
        let player = PlayerState::default();
        let keep = Some(Interpolation::KeepAngles);
        assert_eq!(
            interpolation(&player, NoPredict(1), false, false, || true),
            keep
        );
        assert_eq!(
            interpolation(&player, NoPredict(0), false, true, || true),
            keep
        );
    }

    #[test]
    fn japlus_view_lock_takes_the_server_angles_only_for_no_predict_one() {
        let player = player_with(&[(13, animation("BOTH_JUMP_BACKFLIP_ATCKEE"))]);
        assert!(japlus_view_locked(&player));
        assert_eq!(
            interpolation(&player, NoPredict(1), false, false, || true),
            Some(Interpolation::ServerAngles)
        );
        assert_eq!(
            interpolation(&player, NoPredict(1), false, false, || false),
            Some(Interpolation::KeepAngles)
        );
        assert_eq!(
            interpolation(&player, NoPredict(0), false, true, || true),
            Some(Interpolation::KeepAngles)
        );
    }

    #[test]
    fn a_japlus_kick_victim_is_not_predicted() {
        let down = player_with(&[(13, animation("BOTH_BACK_FALLING"))]);
        assert!(japlus_special_kick(&down));
        assert_eq!(
            interpolation(&down, NoPredict(0), false, false, || true),
            Some(Interpolation::ServerAngles)
        );
        assert_eq!(
            interpolation(&down, NoPredict(0), false, false, || false),
            None
        );
        let marked = player_with(&[(89, 4)]);
        assert!(japlus_special_kick(&marked));
        assert!(!japlus_special_kick(&PlayerState::default()));
    }

    #[test]
    fn fake_noclip_needs_a_live_player_on_foot_out_of_a_duel() {
        assert!(fake_noclip_allowed(&PlayerState::default(), false));
        assert!(!fake_noclip_allowed(&PlayerState::default(), true));
        assert!(!fake_noclip_allowed(&player_with(&[(84, 7)]), false));
        // `duelInProgress`: hub report #209.
        assert!(!fake_noclip_allowed(&player_with(&[(119, 1)]), false));
    }

    #[test]
    fn the_server_holds_the_view_in_grapples_throws_locks_and_japlus_locks() {
        let free = PlayerState::default();
        assert!(!server_holds_view(&free, ja_plus));
        // A melee grapple, either side of it, on any server.
        let beaten = player_with(&[(15, animation("BOTH_PLAYER_PA_1"))]);
        assert!(server_holds_view(&beaten, JaPlusRules::default));
        let grabbing = player_with(&[(13, animation("BOTH_KYLE_GRAB"))]);
        assert!(server_holds_view(&grabbing, JaPlusRules::default));
        // Thrown: `HANDEXTEND_PRETHROWN`.
        assert!(server_holds_view(
            &player_with(&[(80, 13)]),
            JaPlusRules::default
        ));
        // A saber lock running at the command's time, not one long over.
        let locked = player_with(&[(0, 5_000), (107, 5_200)]);
        assert!(server_holds_view(&locked, JaPlusRules::default));
        let over = player_with(&[(0, 5_000), (107, 4_000)]);
        assert!(!server_holds_view(&over, JaPlusRules::default));
        // A JA+ kiss only on a JA+ server.
        let kiss = player_with(&[(13, animation("BOTH_KISSEE"))]);
        assert!(server_holds_view(&kiss, ja_plus));
        assert!(!server_holds_view(&kiss, JaPlusRules::default));
    }

    #[test]
    fn japlus_knockdowns_and_get_ups_hold_the_view_unless_the_server_leaves_it_free() {
        let free_knockdowns = || JaPlusRules {
            free_knockdown_view: true,
            ..ja_plus()
        };
        for (legs, torso, hand) in [
            ("BOTH_KNOCKDOWN1", "BOTH_KNOCKDOWN1", 8),
            ("BOTH_GETUP1", "BOTH_GETUP1", 0),
            ("BOTH_GETUP_BROLL_R", "BOTH_GETUP_BROLL_R", 0),
            ("BOTH_FORCE_GETUP_B1", "BOTH_FORCE_GETUP_B1", 0),
        ] {
            let player = player_with(&[(13, animation(legs)), (15, animation(torso)), (80, hand)]);
            assert!(
                japlus_view_locked(&player),
                "{legs}: still a lock for cg_noPredict 1"
            );
            // EternalJK's rule, kept on JA+ servers not known to differ.
            assert!(server_holds_view(&player, ja_plus), "{legs}");
            // As the JoF server's demos show: `delta_angles` unchanged, the view the
            // mouse's.
            assert!(!server_holds_view(&player, free_knockdowns), "{legs}");
        }
        // The kiss stays held there.
        let kiss = player_with(&[(13, animation("BOTH_KISSEE"))]);
        assert!(server_holds_view(&kiss, free_knockdowns));
    }

    #[test]
    fn a_japlus_emote_with_a_knockdowns_animation_leaves_the_view_free() {
        // `amneo`: a get-up animation, the body frozen by the emote.
        let neo = animation("BOTH_FORCE_GETUP_B4");
        let emote = player_with(&[(13, neo), (15, neo), (63, u32::from(PM_FREEZE))]);
        assert!(!server_holds_view(&emote, ja_plus));
    }
}
