//! World shots of a storm on `T2_Rogue` at ultra (`r_weatherForce 3`, `r_weatherQuality
//! 3`, `r_weatherFog 2`, as Sol set it there on 05/11/2026): the streets from the start,
//! their puddles mirroring the world ([`crate::weather::mirror`]) and, for comparison,
//! the same views with the puddles' mirror off (the sky alone, as before it). Ignored
//! like the other world shots. With `SJK_GPU_PHASES=1` the `gpu-phases` lines print the
//! wet pass's time (`rain-wet`, the mirror's copy included) for each half.

use super::*;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn t2_rogue_storm_puddles() {
    on_big_stack(|| {
        let cvars = [
            ("r_weatherForce", "3"),
            ("r_weatherQuality", "3"),
            ("r_weatherFog", "2"),
        ];
        let Some((mut gpu, _profile)) = open("maps/t2_rogue.bsp", [1600, 900], None, &cvars) else {
            return;
        };
        gpu.is_menu_world = false;
        // The map's start (info_player_start at 1872 -1984 -552, facing +Y), at eye height.
        let eye = [1872.0, -1984.0, -510.0];
        let views: [(f32, f32); 4] = [(90.0, -12.0), (90.0, -30.0), (0.0, -20.0), (200.0, -20.0)];
        std::fs::create_dir_all(directory()).expect("the shot directory");
        let mut images = Vec::new();
        for mirror in [true, false] {
            gpu.weather.set_mirror_allowed(mirror);
            eprintln!("perf: puddle mirror {}", if mirror { "on" } else { "off" });
            for (index, (yaw, pitch)) in views.iter().enumerate() {
                aim(&mut gpu, eye, *yaw, *pitch);
                // The cover and the far cover survey on workers: give them time.
                let frames = if images.is_empty() { 240 } else { 96 };
                let image = frame(&mut gpu, frames);
                let name = format!(
                    "t2_rogue_storm_{index}_{}",
                    if mirror { "mirror" } else { "sky" }
                );
                image
                    .save(directory().join(format!("{name}.png")))
                    .expect("write the shot");
                images.push(image);
            }
        }
        let sheet = sheet(&images, 4, 640, "t2_rogue_storm_puddles");
        println!("{}", sheet.display());
    });
}

/// The storm's GPU cost on `T2_Rogue`, each weather setting switched in turn, for a
/// release run with `SJK_GPU_PHASES=1`: a `perf: <setting>` line, then the `gpu-phases`
/// lines (every 32nd frame) of 192 frames under it, at 2560x1440, from two views (the
/// street and the start looking down it). Sum the `total_ms` per setting: weather is
/// drawn inside the effect pass, so the frame's total is what it costs.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn t2_rogue_storm_timing() {
    on_big_stack(|| {
        let cvars = [
            ("r_weatherForce", "3"),
            ("r_weatherQuality", "3"),
            ("r_weatherFog", "2"),
        ];
        let Some((mut gpu, _profile)) = open("maps/t2_rogue.bsp", [2560, 1440], None, &cvars)
        else {
            return;
        };
        gpu.is_menu_world = false;
        let eye = [1872.0, -1984.0, -510.0];
        // The surveys land first.
        aim(&mut gpu, eye, 90.0, -12.0);
        let _ = frame(&mut gpu, 240);
        let settings: [(&str, &[(&str, &str)]); 8] = [
            ("nothing", &[("r_weather", "0"), ("r_clouds", "0")]),
            ("clouds only", &[("r_clouds", "1")]),
            (
                "storm high, no fog",
                &[
                    ("r_weather", "1"),
                    ("r_weatherQuality", "2"),
                    ("r_weatherFog", "0"),
                ],
            ),
            ("storm high", &[("r_weatherFog", "2")]),
            ("storm high, no clouds", &[("r_clouds", "0")]),
            (
                "storm ultra",
                &[("r_clouds", "1"), ("r_weatherQuality", "3")],
            ),
            ("storm medium", &[("r_weatherQuality", "1")]),
            ("storm low", &[("r_weatherQuality", "0")]),
        ];
        // A first round compiles every pipeline the settings use (a frame of 260 ms the
        // first time); the two after it are timed.
        for round in 0..3 {
            for (name, values) in settings {
                if let Some(console) = gpu.console.as_mut() {
                    for (cvar, value) in values {
                        console.set_cvar(cvar, value);
                    }
                }
                for (view, (yaw, pitch)) in
                    [(90.0_f32, -12.0_f32), (90.0, -30.0)].iter().enumerate()
                {
                    aim(&mut gpu, eye, *yaw, *pitch);
                    // Let the setting and the view settle (and the last samples land).
                    let _ = frame(&mut gpu, 64);
                    if round == 0 {
                        eprintln!("perf: warm-up");
                    } else {
                        eprintln!("perf: {name} view {view}");
                    }
                    let _ = frame(&mut gpu, if round == 0 { 32 } else { 160 });
                }
            }
        }
    });
}

/// Running water on `T2_Rogue`'s slanted walls and ramps in a storm at high quality, close
/// up from the start's street (both sides, a step further on), as a contact sheet.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn t2_rogue_running_water() {
    on_big_stack(|| {
        let cvars = [("r_weatherForce", "3"), ("r_weatherQuality", "2")];
        let Some((mut gpu, _profile)) = open("maps/t2_rogue.bsp", [1600, 900], None, &cvars) else {
            return;
        };
        gpu.is_menu_world = false;
        std::fs::create_dir_all(directory()).expect("the shot directory");
        let views: [([f32; 3], f32, f32); 6] = [
            ([1872.0, -1634.0, -510.0], 180.0, -5.0),
            ([1872.0, -1634.0, -510.0], 0.0, -5.0),
            ([1872.0, -1284.0, -510.0], 160.0, 0.0),
            ([1872.0, -1284.0, -510.0], 20.0, 0.0),
            ([1872.0, -1634.0, -510.0], 135.0, 10.0),
            ([1872.0, -1634.0, -510.0], 45.0, 10.0),
        ];
        let mut images = Vec::new();
        for (index, (eye, yaw, pitch)) in views.iter().enumerate() {
            aim(&mut gpu, *eye, *yaw, *pitch);
            let image = frame(&mut gpu, if images.is_empty() { 240 } else { 48 });
            image
                .save(directory().join(format!("t2_rogue_running_water_{index}.png")))
                .expect("write the shot");
            images.push(image);
        }
        println!(
            "{}",
            sheet(&images, 3, 800, "t2_rogue_running_water").display()
        );
    });
}

/// A lightning strike in the storm on `T2_Rogue` (`r_weatherLightning 1`), down the street
/// from the start: the frame at its brightest beside the same view between strikes.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn t2_rogue_lightning() {
    on_big_stack(|| {
        let cvars = [
            ("r_weatherForce", "3"),
            ("r_weatherFog", "2"),
            ("r_weatherLightning", "1"),
        ];
        let Some((mut gpu, _profile)) = open("maps/t2_rogue.bsp", [1600, 900], None, &cvars) else {
            return;
        };
        gpu.is_menu_world = false;
        std::fs::create_dir_all(directory()).expect("the shot directory");
        aim(&mut gpu, [1872.0, -1984.0, -510.0], 90.0, -8.0);
        // No strike of the storm's own while the surveys land, then one now.
        gpu.weather.strike_in(1.0e6);
        let calm = frame(&mut gpu, 240);
        gpu.weather.strike_in(0.0);
        let lit = frame(&mut gpu, 1);
        println!(
            "lightning flash in the shot: {:.2}; setting {} force {}; {}",
            gpu.weather.flash(),
            gpu.context.weather.lightning(),
            gpu.context.weather.force(),
            gpu.weather.lightning_state()
        );
        calm.save(directory().join("t2_rogue_lightning_calm.png"))
            .expect("write the shot");
        lit.save(directory().join("t2_rogue_lightning_flash.png"))
            .expect("write the shot");
        println!(
            "{}",
            sheet(&[calm, lit], 2, 800, "t2_rogue_lightning").display()
        );
    });
}
