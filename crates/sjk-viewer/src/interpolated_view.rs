//! Stock `CG_InterpolatePlayerState`: smooth authoritative views between snapshots.
//! Follow views interpolate angles; synchronous local views keep immediate mouse-look.

use super::GpuState;
use glam::Vec3;
use sjk_game_jka::prediction_policy::{self, Interpolation, NoPredict};
use sjk_protocol::{GameState, PlayerState, Snapshot};

/// Whether the local view is the server's rather than predicted, and how
/// (`CG_PredictPlayerState`): following, a synchronous server, `cg_noPredict`, a
/// JA+ kick victim, on an E-Web (`eweb`, [`using_eweb`]), or, while jaPRO tools are
/// loaded, a movement style SJK's pmove does not model ([`japro_unpredicted`]).
pub(crate) fn mode(
    console: Option<&crate::console::ViewerConsole>,
    game: &GameState,
    player: &PlayerState,
    eweb: bool,
) -> Option<Interpolation> {
    let no_predict = console
        .and_then(|console| console.integer_cvar("cg_noPredict"))
        .unwrap_or(0);
    prediction_policy::interpolation(
        player,
        NoPredict(no_predict),
        !crate::local_prediction::predicts_local_view(player.movement_flags()),
        server_synchronous(game),
        || sjk_game_jka::pmove_japlus::JaPlusRules::from_game_state(game).enabled,
    )
    // `CG_UsingEWeb`: the server moves the gunner behind the turning gun every frame, so
    // its place is taken from the snapshots, the mouse kept (`CG_InterpolatePlayerState(
    // qtrue)`, `cg_predict.c:1080`).
    .or_else(|| eweb.then_some(Interpolation::KeepAngles))
    .or_else(|| {
        sjk_game_jka::pmove_lugormod::LugormodRules::from_game_state(game)
            .needs_snapshot(player)
            .then_some(Interpolation::KeepAngles)
    })
    .or_else(|| {
        let style = japro_unpredicted(console, player);
        announce_style(style);
        style.map(|_| Interpolation::KeepAngles)
    })
}

/// The jaPRO tools mod's bridge (`sjk_mod_japro::movement_styles`): the style the
/// player moves with that SJK's pmove does not model, while the mod is loaded (a
/// jaPRO server, `mod_japro` on) and `japro.predictMoveStyles` is off.
#[cfg(feature = "mod-japro")]
fn japro_unpredicted(
    console: Option<&crate::console::ViewerConsole>,
    player: &PlayerState,
) -> Option<&'static str> {
    use sjk_mod_japro::movement_styles;
    let console = console.filter(|console| console.mods.loaded(sjk_mod_japro::ID))?;
    let predict_anyway = console
        .bool_cvar(movement_styles::PREDICT_ANYWAY_CVAR)
        .unwrap_or(false);
    movement_styles::unpredicted(player, predict_anyway)
}

/// Without jaPRO tools every style is predicted with the stock JKA movement.
#[cfg(not(feature = "mod-japro"))]
fn japro_unpredicted(
    _console: Option<&crate::console::ViewerConsole>,
    _player: &PlayerState,
) -> Option<&'static str> {
    None
}

/// Say in the log, once per change, that a movement style is shown unpredicted.
fn announce_style(style: Option<&'static str>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    // The last style's name by address (0 for none): the names are statics.
    static LAST: AtomicUsize = AtomicUsize::new(0);
    let address = style.map_or(0, |name| name.as_ptr() as usize);
    if LAST.swap(address, Ordering::Relaxed) == address {
        return;
    }
    if let Some(name) = style {
        crate::log::progress(format_args!(
            "movement style {name}: SJK does not predict it yet, so your movement shows as \
             the server has it (japro.predictMoveStyles 1 predicts it as JKA)",
        ));
    }
}

/// `CG_UsingEWeb` (`cg_predict.c:978-988`): the local player works an E-Web — on the
/// emplaced gun's weapon, its `emplacedIndex` naming an entity whose weapon is
/// `WP_NONE` (an emplaced gun's is `WP_EMPLACED_GUN`).
pub(crate) fn using_eweb(snapshot: &Snapshot) -> bool {
    const WP_EMPLACED_GUN: u8 = 17;
    let player = &snapshot.player;
    let gun = player.emplaced_index();
    player.weapon() == WP_EMPLACED_GUN
        && gun != 0
        && snapshot
            .entities
            .binary_search_by_key(&gun, |state| state.number())
            .is_ok_and(|index| snapshot.entities[index].weapon() == 0)
}

pub(crate) fn server_synchronous(game: &GameState) -> bool {
    game.config_string(1)
        .and_then(|info| sjk_protocol::info_value(info, b"g_synchronousClients"))
        .is_some_and(|value| sjk_game_jka::userinfo::atoi(value) != 0)
}

/// Do not bridge a teleport, a followed-player change, or a change of view mode.
fn fraction(previous: &Snapshot, next: &Snapshot, time: i32) -> f32 {
    if next.server_time <= previous.server_time
        || (previous.player.entity_flags() ^ next.player.entity_flags()) & 8 != 0
        || previous.player.client_num() != next.player.client_num()
        || (previous.player.movement_flags() ^ next.player.movement_flags()) & 4096 != 0
    {
        return 0.0;
    }
    ((time - previous.server_time) as f32 / (next.server_time - previous.server_time) as f32)
        .clamp(0.0, 1.0)
}

fn lerp_angle(from: f32, mut to: f32, fraction: f32) -> f32 {
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + fraction * (to - from)
}

// Lugormod cg_view.c:270 carries the upper eye-height bit in userInt2.
fn snapshot_eye_height(player: &PlayerState, lugormod: bool) -> i32 {
    if lugormod && player.movement_type() != 5 {
        (player.view_height() as u8) as i32
            + (((player.raw_field(126).unwrap_or(0) & 1) << 8) as i32)
    } else {
        player.view_height()
    }
}

fn sample(previous: &Snapshot, next: &Snapshot, time: i32, lugormod: bool) -> ([f32; 3], [f32; 3]) {
    let f = fraction(previous, next, time);
    let from = previous.player.origin();
    let to = next.player.origin();
    let mut eye = std::array::from_fn(|axis| from[axis] + f * (to[axis] - from[axis]));
    eye[2] += snapshot_eye_height(&previous.player, lugormod) as f32;
    let from = previous.player.view_angles();
    let to = next.player.view_angles();
    (
        eye,
        std::array::from_fn(|axis| lerp_angle(from[axis], to[axis], f)),
    )
}

pub(crate) fn present(gpu: &mut GpuState, time: i32) {
    let Some(session) = gpu.live_session.as_ref() else {
        return;
    };
    let latest = session.latest_snapshot();
    // Intermission owns its camera. A missing predictor alone is not enough.
    if matches!(latest.player.movement_type(), 7 | 8) {
        return;
    }
    let eweb = using_eweb(latest);
    let Some(mode) = mode(
        gpu.console.as_ref(),
        session.game_state(),
        &latest.player,
        eweb,
    ) else {
        return;
    };
    let previous = session.snapshot_at_or_before(time);
    let next = session
        .snapshot_after(previous.server_time)
        .unwrap_or(previous);
    let lugormod =
        sjk_game_jka::pmove_lugormod::LugormodRules::from_game_state(session.game_state()).enabled;
    let (eye, angles) = sample(previous, next, time, lugormod);
    gpu.camera_position = Vec3::from_array(eye);
    if mode == Interpolation::ServerAngles {
        gpu.camera_pitch = -angles[0].to_radians();
        gpu.camera_yaw = angles[1].to_radians();
    }
}

#[cfg(test)]
mod lugormod_tests {
    use super::*;

    #[test]
    fn scaled_snapshot_eye_uses_upper_bit_only_for_live_lugormod_players() {
        let mut player = PlayerState::default();
        player.set_view_height(-112);
        player.set_raw_field(126, 1);
        assert_eq!(snapshot_eye_height(&player, true), 400);
        assert_eq!(snapshot_eye_height(&player, false), -112);
        player.set_movement_type(5);
        assert_eq!(snapshot_eye_height(&player, true), -112);
    }

    #[test]
    fn missing_rope_state_keeps_mouse_look_and_clears_on_server_switch() {
        let mut game = GameState::empty_local(0);
        let mut player = PlayerState::default();
        player.stats[0] = 100;
        player.set_movement_type(9);
        for name in ["Lugormod v3", "basejka", "JA+ Mod", "japro", "Lugormod v3"] {
            game.replace_config_string(0, format!("\\gamename\\{name}").into_bytes())
                .unwrap();
            assert_eq!(
                mode(None, &game, &player, false),
                if name == "Lugormod v3" {
                    Some(Interpolation::KeepAngles)
                } else {
                    None
                },
                "{name}"
            );
        }
        player.set_movement_type(0);
        assert_eq!(mode(None, &game, &player, false), None);
    }

    /// On the emplaced gun's weapon at an E-Web (`WP_NONE`): unpredicted; at an emplaced
    /// gun, or with another weapon, predicted.
    #[test]
    fn a_gunner_on_an_eweb_is_not_predicted() {
        let gun = |weapon: u32| {
            let mut state =
                sjk_protocol::EntityState::zero(70, &sjk_protocol::LEGACY_ENTITY_FIELDS);
            state.set_raw_field(14, weapon);
            state
        };
        let mut snapshot = Snapshot {
            message_sequence: 0,
            reliable_acknowledge: 0,
            server_commands: Vec::new(),
            server_time: 0,
            delta_from: None,
            flags: 0,
            area_mask: Vec::new(),
            player: PlayerState::default(),
            vehicle_player: None,
            entities: vec![gun(0)],
            consumed_bits: 0,
        };
        // `ps.weapon`, `ps.emplacedIndex`.
        snapshot.player.set_raw_field(47, 17);
        snapshot.player.set_raw_field(112, 70);
        assert!(using_eweb(&snapshot));
        snapshot.entities = vec![gun(17)];
        assert!(!using_eweb(&snapshot));
        snapshot.entities = vec![gun(0)];
        snapshot.player.set_raw_field(47, 3);
        assert!(!using_eweb(&snapshot));
        let game = GameState::empty_local(0);
        assert_eq!(
            mode(None, &game, &PlayerState::default(), true),
            Some(Interpolation::KeepAngles)
        );
    }
}
