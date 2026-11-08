//! World shots of toon models (`r_toonModels`): a recorded game played without a
//! server, the camera orbiting and looking at a player, stock shading beside toon
//! shading, every player taken to have chosen it (`Looks::shot_toon`). Ignored like the other world shots; skipped when `SJK_TEST_DEMO` is not set.
//! `SJK_SHOT_CVARS` runs it under other lighting (e.g. `r_dayNight=1`).

use super::*;

#[test]
#[ignore = "renders with the GPU, the installed game data and the demo SJK_TEST_DEMO names"]
fn toon_models_on_a_demo() {
    on_big_stack(|| {
        let Some(demo) = std::env::var_os("SJK_TEST_DEMO").map(PathBuf::from) else {
            println!("SJK_TEST_DEMO is not set: skipped");
            return;
        };
        let probe = demo_playback::Session::open(&demo, demo_playback::Camera::FirstPerson, 60)
            .expect("the demo");
        let own = probe.latest_snapshot().player.client_num();
        let players: Vec<u16> = probe
            .latest_snapshot()
            .entities
            .iter()
            .map(|entity| entity.number())
            .filter(|number| *number < 32 && *number != own)
            .collect();
        let Some(&target) = players.first() else {
            println!("no other player in the demo's first snapshot: skipped");
            return;
        };
        let suffix = std::env::var("SJK_SHOT_SUFFIX").unwrap_or_default();
        let mut images = Vec::new();
        for (camera, view) in [
            (demo_playback::Camera::Orbit(target), "orbit"),
            (demo_playback::Camera::LookAt(target), "look"),
        ] {
            let Some(mut gpu) = card_outline_shots::open_demo(&demo, camera, [1600, 900]) else {
                return;
            };
            let _ = frame(&mut gpu, 30);
            for toon in ["0", "1"] {
                if let Some(console) = gpu.console.as_mut() {
                    assert!(console.set_cvar(crate::dynamic_lights::TOON_MODELS_CVAR, toon));
                }
                gpu.looks.shot_toon = toon == "1";
                let image = frame(&mut gpu, 10);
                let name = format!("toon_models_{view}_{toon}{suffix}");
                std::fs::create_dir_all(directory()).expect("the shot directory");
                let path = directory().join(format!("{name}.png"));
                image.save(&path).expect("write the shot");
                println!("{}", path.display());
                images.push(image);
            }
        }
        println!(
            "{}",
            sheet(&images, 2, 1200, &format!("toon_models_sheet{suffix}")).display()
        );
    });
}
