//! World shots of the WHEEL OF DOOM ([`crate::wheel_of_doom`]) on duel6: three wheels
//! mid-spin, and three just stopped (on the Mythical jackpot, nothing and reverse keys,
//! their words over them). Ignored like the other world shots.

use super::*;

fn wheels(name: &'static str, since: std::time::Duration) {
    on_big_stack(move || {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1600, 900], None, &[]) else {
            return;
        };
        let shots = crate::menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (from, at) = (
            Vec3::from_array(shots[0].from),
            Vec3::from_array(shots[0].at),
        );
        // Three wheels in a row across the view, 260 units ahead, facing the camera.
        let ahead = (at - from).with_z(0.0).normalize_or(Vec3::X);
        let across = Vec3::new(-ahead.y, ahead.x, 0.0);
        let floor = from + ahead * 260.0 - Vec3::Z * 60.0;
        let yaw = ahead.y.atan2(ahead.x).to_degrees() + 180.0;
        let placed: Vec<([f32; 3], f32)> = (0..3)
            .map(|index| {
                (
                    (floor + across * (index as f32 - 1.0) * 110.0).to_array(),
                    yaw,
                )
            })
            .collect();
        gpu.shot_wheels_of_doom(&placed, since);
        let (yaw, pitch) = look(from.to_array(), (floor + Vec3::Z * 64.0).to_array());
        aim(&mut gpu, from.to_array(), yaw, pitch);
        gpu.is_menu_world = false;
        println!("{}", shoot(&mut gpu, 90, name).display());
        assert_eq!(gpu.shot_wheel_of_doom_state().1, 3, "the wheels kept");
        assert!(
            gpu.shot_wheel_of_doom_state().0.iter().all(Option::is_some),
            "the models"
        );
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_wheel_of_doom_spinning() {
    wheels(
        "duel6-wheel-of-doom-spinning",
        std::time::Duration::from_millis(2_500),
    );
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_wheel_of_doom_stopped() {
    wheels(
        "duel6-wheel-of-doom-stopped",
        crate::wheel_of_doom::SHOT_SPIN + std::time::Duration::from_millis(500),
    );
}
