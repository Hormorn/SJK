//! JA+'s emotes in the movement (VisionJK `codemp/game/bg/bg_pmove.c:10789-10823`,
//! `11125-11196`; `bg_panimate.c:1630-1667`): the poses JA+ adds to the full-body taunts,
//! the ones it holds until a button is pressed, and the animation that ends each.
//!
//! An emote (`amsit`, `sleep`...) plays its pose as a taunt (`HANDEXTEND_TAUNT`). On a JA+
//! server a held pose keeps the player still, the view free, its timers topped up to a
//! second, until an attack, a Force button, the hook's button or a jump ends it with its
//! own end animation. The JA+ plugin predicts the hold; a JA+ server freezes
//! (`PM_FREEZE`) the players who do not ([`EmoteFreeze`]). Every pose here is in the
//! retail `_humanoid` animations; JA+'s own (`japlus_gla_anims.pk3`) are not played.

/// Which held emotes the server freezes (`PM_FREEZE`), so that a client that does not
/// predict the hold is not pulled back: only a server sets it. A client predicts the
/// hold itself (`Off`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmoteFreeze {
    /// A client's own movement: no freeze, and `amatease`'s pose is not held.
    #[default]
    Off,
    /// Only `amatease` (`BOTH_STAND4`), which no client holds: every player's client
    /// predicts the others (`jp_fixEmot 1` with every player on the JA+ plugin).
    AtEase,
    /// Every held pose but meditation.
    Held,
}

/// The poses JA+ adds to `BG_FullBodyTauntAnim` (VisionJK `bg_panimate.c:1649-1662`),
/// which are also the emotes it refuses in team games (`JP_EmoteNotForTeams`,
/// `g_japlus_cmds.c:72-80`).
pub fn full_body_pose(name: &str) -> bool {
    matches!(
        name,
        "BOTH_DEATH15"
            | "BOTH_STAND4"
            | "BOTH_STAND5TOSIT3"
            | "BOTH_STAND5TOSTAND8"
            | "BOTH_CROUCH3"
            | "BOTH_KNEES2"
            | "BOTH_HIT1"
            | "BOTH_SLEEP1"
            | "BOTH_SLEEP6START"
            | "BOTH_STAND10"
            | "BOTH_SIT6"
            | "BOTH_SIT7"
            | "TORSO_SURRENDER_START"
    )
}

/// `PM_JPEmoteHeld` (VisionJK `bg_pmove.c:10793-10805`): the poses held until a button
/// ends them, meditation's too. `amatease`'s only on a server, which freezes it.
pub(crate) fn held(name: &str, freeze: EmoteFreeze) -> bool {
    match name {
        "BOTH_STAND4" => freeze != EmoteFreeze::Off,
        "BOTH_MEDITATE" => true,
        _ => full_body_pose(name),
    }
}

/// `PM_JPEmoteEndAnim` (VisionJK `bg_pmove.c:10807-10823`): the animation that ends a held
/// pose, none for `amatease`'s.
pub(crate) fn end_animation(name: &str) -> Option<&'static str> {
    Some(match name {
        "BOTH_MEDITATE" => "BOTH_MEDITATE_END",
        "BOTH_SLEEP6START" => "BOTH_SLEEP6STOP",
        "BOTH_SLEEP1" => "BOTH_SLEEP1GETUP2",
        "TORSO_SURRENDER_START" => "TORSO_SURRENDER_STOP",
        "BOTH_CROUCH3" => "BOTH_UNCROUCH3",
        "BOTH_STAND5TOSIT3" => "BOTH_SIT3TOSTAND5",
        "BOTH_SIT6" => "BOTH_SIT2TOSTAND5",
        "BOTH_SIT7" => "BOTH_STANDUP2",
        "BOTH_STAND10" => "BOTH_STAND10TOSTAND1",
        "BOTH_HIT1" => "BOTH_FORCE_GETUP_F2",
        "BOTH_DEATH15" => "BOTH_BACK_FLIP_UP",
        "BOTH_STAND5TOSTAND8" => "BOTH_STAND8TOSTAND5",
        "BOTH_KNEES2" => "BOTH_CHOKE1START",
        _ => return None,
    })
}

/// Whether the server freezes a player held in `torso`'s pose (VisionJK
/// `bg_pmove.c:11186-11193`): never meditation, always `amatease`, the rest unless every
/// client predicts them.
pub(crate) fn freezes(torso: &str, freeze: EmoteFreeze) -> bool {
    match freeze {
        EmoteFreeze::Off => false,
        _ if torso == "BOTH_MEDITATE" => false,
        EmoteFreeze::AtEase => torso == "BOTH_STAND4",
        EmoteFreeze::Held => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pose_and_its_end_are_retail_animations() {
        for name in [
            "BOTH_DEATH15",
            "BOTH_STAND4",
            "BOTH_STAND5TOSIT3",
            "BOTH_STAND5TOSTAND8",
            "BOTH_CROUCH3",
            "BOTH_KNEES2",
            "BOTH_HIT1",
            "BOTH_SLEEP1",
            "BOTH_SLEEP6START",
            "BOTH_STAND10",
            "BOTH_SIT6",
            "BOTH_SIT7",
            "TORSO_SURRENDER_START",
        ] {
            assert!(full_body_pose(name), "{name}");
            assert!(crate::legacy_animation_index(name).is_some(), "{name}");
            if let Some(end) = end_animation(name) {
                assert!(crate::legacy_animation_index(end).is_some(), "{end}");
            }
        }
        assert_eq!(end_animation("BOTH_STAND4"), None);
        assert!(!full_body_pose("BOTH_BOW"));
    }

    #[test]
    fn only_a_server_holds_at_ease_and_freezes() {
        assert!(!held("BOTH_STAND4", EmoteFreeze::Off));
        assert!(held("BOTH_STAND4", EmoteFreeze::AtEase));
        assert!(held("BOTH_SIT6", EmoteFreeze::Off) && held("BOTH_MEDITATE", EmoteFreeze::Off));
        assert!(!freezes("BOTH_SIT6", EmoteFreeze::Off));
        assert!(!freezes("BOTH_SIT6", EmoteFreeze::AtEase));
        assert!(freezes("BOTH_STAND4", EmoteFreeze::AtEase));
        assert!(freezes("BOTH_SIT6", EmoteFreeze::Held));
        assert!(!freezes("BOTH_MEDITATE", EmoteFreeze::Held));
    }
}
