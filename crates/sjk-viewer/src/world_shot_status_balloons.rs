//! World shots of the status balloons ([`crate::status_balloons`]) on a recorded game:
//! the demo `SJK_TEST_DEMO` names is played without a server, the camera orbits another
//! player and each reason's balloon is drawn over everyone, at two moments of its
//! animation. Ignored like the other world shots; skipped when `SJK_TEST_DEMO` is not set.

use super::*;
use crate::status_balloons::{Reason, Shown};

#[test]
#[ignore = "renders with the GPU, the installed game data and the demo SJK_TEST_DEMO names"]
fn status_balloons_on_a_demo() {
    on_big_stack(|| {
        let Some(demo) = std::env::var_os("SJK_TEST_DEMO").map(PathBuf::from) else {
            println!("SJK_TEST_DEMO is not set: skipped");
            return;
        };
        let probe = demo_playback::Session::open(&demo, demo_playback::Camera::FirstPerson, 60)
            .expect("the demo");
        let own = probe.latest_snapshot().player.client_num();
        let Some(target) = probe
            .latest_snapshot()
            .entities
            .iter()
            .map(|entity| entity.number())
            .find(|number| *number < 32 && *number != own)
        else {
            println!("no other player in the demo's first snapshot: skipped");
            return;
        };
        let Some(mut gpu) = super::card_outline_shots::open_demo(
            &demo,
            demo_playback::Camera::Orbit(target),
            [1600, 900],
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 20);
        let mut images = Vec::new();
        for reason in [Reason::Typing, Reason::Menu, Reason::Away] {
            gpu.status_balloons.shot = Some(Shown::for_shot(reason));
            images.push(frame(&mut gpu, 6));
            images.push(frame(&mut gpu, 12));
        }
        println!("{}", sheet(&images, 2, 800, "status-balloons").display());
    });
}
