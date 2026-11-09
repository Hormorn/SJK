//! jaPRO's movement styles, and which of them this client's movement prediction models.
//!
//! A jaPRO (TaystJK lineage) server lets each player pick a movement style (`/move`):
//! CPM, QW, Q3, WSW and others change friction, acceleration, air control and jumps
//! (JoF EternalJK `codemp/game/bg_pmove.c`, every `moveStyle` branch). The server says
//! which one in `ps.stats[STAT_MOVEMENTSTYLE]`, and a jaPRO-aware client reads it
//! (`PM_GetMovePhysics`, `bg_pmove.c:470-486`): `SWOOP` while riding a vehicle, the
//! co-op styles folded back onto their base style.
//!
//! SJK's pmove is stock JKA's, so predicting a style it does not model disagrees with
//! the server every frame and the view snaps back (hub reports #97/#98, 19/10/2026).
//! Until a style is ported it is shown as the server has it, as `cg_noPredict 1`
//! does; [`PREDICT_ANYWAY_CVAR`] predicts every style as JKA, the earlier behaviour.
//!
//! Porting a style: its physics go in this crate, beside the table (one module per
//! style family, CPM first: Q3, Slick and BotCPM build on it), called by the viewer's
//! prediction through the same bridge as [`unpredicted`]; once a recorded jaPRO demo
//! predicts it without misses, its entry turns `predicted: true`.

use sjk_protocol::PlayerState;

/// jaPRO's `STAT_MOVEMENTSTYLE` (`bg_public.h:724`).
pub const STAT_MOVEMENTSTYLE: usize = 13;

/// 1 predicts every style with this client's JKA movement, modelled or not.
pub const PREDICT_ANYWAY_CVAR: &str = "japro.predictMoveStyles";

/// One movement style (`movementStyle_e`, `bg_public.h:551-575`).
#[derive(Debug, Eq, PartialEq)]
pub struct MoveStyle {
    /// The style's number on the wire.
    pub id: u32,
    /// Its name, as jaPRO's `/move` lists it.
    pub name: &'static str,
    /// Whether this client's pmove models it, so predicting it matches the server.
    pub predicted: bool,
}

const fn style(id: u32, name: &'static str, predicted: bool) -> MoveStyle {
    MoveStyle {
        id,
        name,
        predicted,
    }
}

/// Every style, by number. `predicted` is true only where SJK's pmove is the style's:
/// stock JKA, Siege (stock's Siege acceleration), the vehicle style (vehicles move
/// with their own pmove), and Speed (no pmove branch: the server's `ps.speed`,
/// `g_active.c:4262`). The rest have their own physics, not ported yet.
pub const MOVE_STYLES: &[MoveStyle] = &[
    style(0, "Siege", true),
    style(1, "JKA", true),
    style(2, "QW", false),
    style(3, "CPM", false),
    style(4, "Q3", false),
    style(5, "PJK", false),
    style(6, "WSW", false),
    style(7, "RJQ3", false),
    style(8, "RJCPM", false),
    style(9, "Swoop", true),
    style(10, "Jetpack", false),
    style(11, "Speed", true),
    style(12, "SP", false),
    style(13, "Slick", false),
    style(14, "BotCPM", false),
];

/// `MV_SWOOP`, the style of a player riding a vehicle.
const SWOOP: u32 = 9;
/// `MV_COOP_JKA`: it and the numbers above it are co-op variants of a base style.
const COOP_JKA: u32 = 15;

/// The style a jaPRO server moves `player` with (`PM_GetMovePhysics`), when it is one
/// this client knows.
pub fn server_style(player: &PlayerState) -> Option<&'static MoveStyle> {
    let id = if player.vehicle_entity_num() != 0 {
        SWOOP
    } else {
        let stat = player.stats[STAT_MOVEMENTSTYLE];
        if stat >= COOP_JKA {
            stat - (COOP_JKA - 1)
        } else {
            stat
        }
    };
    MOVE_STYLES.iter().find(|style| style.id == id)
}

/// The style `player` moves with that this client cannot predict, unless the player
/// asked to predict it anyway. An unknown number is not predicted either. Only for a
/// jaPRO server (the mod loaded): other servers' `stats[13]` means something else.
pub fn unpredicted(player: &PlayerState, predict_anyway: bool) -> Option<&'static str> {
    if predict_anyway {
        return None;
    }
    match server_style(player) {
        Some(style) if style.predicted => None,
        Some(style) => Some(style.name),
        None => Some("unknown"),
    }
}

/// What the strafe helper takes from a style's physics (JoF EternalJK
/// `CG_StrafeHelper`, `cg_draw.c:12105-12160`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuidePhysics {
    /// `pm_accelerate` on the ground.
    pub ground_acceleration: f32,
    /// `pm_friction` on the ground.
    pub friction: f32,
    /// `pm_airaccelerate` in the air.
    pub air_acceleration: f32,
}

/// The physics the strafe helper works with for the style `player` moves with; `None`
/// where it draws nothing: Siege (`if (moveStyle == 0) return`) and an unknown number.
/// Only for a jaPRO server, as [`unpredicted`].
pub fn guide_physics(player: &PlayerState) -> Option<GuidePhysics> {
    let (ground_acceleration, friction, air_acceleration) = match server_style(player)?.id {
        0 => return None,
        // WSW.
        6 => (12.0, 8.0, 1.0),
        // CPM, RJCPM and BotCPM.
        3 | 8 | 14 => (15.0, 8.0, 1.0),
        // SP.
        12 => (12.0, 6.0, 4.0),
        // Slick.
        13 => (30.0, 0.0, 1.0),
        _ => (10.0, 6.0, 1.0),
    };
    Some(GuidePhysics {
        ground_acceleration,
        friction,
        air_acceleration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strafe_helper_follows_each_styles_physics() {
        let physics = |style| guide_physics(&player(style, 0));
        assert_eq!(physics(0), None);
        assert_eq!(physics(99), None);
        let stock = GuidePhysics {
            ground_acceleration: 10.0,
            friction: 6.0,
            air_acceleration: 1.0,
        };
        assert_eq!(physics(1), Some(stock));
        assert_eq!(physics(2), Some(stock));
        let cpm = physics(3).unwrap();
        assert_eq!((cpm.ground_acceleration, cpm.friction), (15.0, 8.0));
        assert_eq!(physics(12).unwrap().air_acceleration, 4.0);
        assert_eq!(physics(13).unwrap().friction, 0.0);
        // A co-op style is its base style's.
        assert_eq!(physics(17), Some(cpm));
    }

    fn player(style: u32, vehicle: u32) -> PlayerState {
        let mut player = PlayerState::default();
        player.stats[STAT_MOVEMENTSTYLE] = style;
        player.set_raw_field(84, vehicle);
        player
    }

    #[test]
    fn the_table_follows_the_enum_order() {
        for (index, style) in MOVE_STYLES.iter().enumerate() {
            assert_eq!(style.id as usize, index, "{}", style.name);
        }
    }

    #[test]
    fn only_unmodelled_styles_stop_prediction() {
        assert_eq!(unpredicted(&player(3, 0), false), Some("CPM"));
        assert_eq!(unpredicted(&player(1, 0), false), None);
        assert_eq!(unpredicted(&player(11, 0), false), None);
        // Riding a vehicle is Swoop, whatever the stat says.
        assert_eq!(unpredicted(&player(3, 7), false), None);
        assert_eq!(unpredicted(&player(3, 0), true), None);
        assert_eq!(unpredicted(&player(99, 0), false), Some("unknown"));
    }

    #[test]
    fn coop_styles_fold_onto_their_base_style() {
        // `bg_pmove.c:479`: MV_COOP_JKA is JKA, the numbers after it QW, CPM...
        assert_eq!(server_style(&player(15, 0)).unwrap().name, "JKA");
        assert_eq!(server_style(&player(17, 0)).unwrap().name, "CPM");
    }
}
