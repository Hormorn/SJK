//! World shots of the graphics presets of 10/11/2026 (`graphics_quality.rs`): `ffa3`
//! from its start camera under Vanilla, Vanilla Plus and the Solar Renderer's Low,
//! Medium, High and Ultra, each with its rating for this card; and the time-of-day sky
//! (`r_daySky`) on Vanilla Plus at noon, dusk and night. Ignored like the other world
//! shots; the sheets land in `target/world-shots`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer solar_ -- --ignored
//! --test-threads=1 --nocapture`.

use super::*;
use crate::graphics_quality::{Level, preset_cvars};

const MAP: &str = "maps/mp/ffa3.bsp";
const SIZE: [u32; 2] = [1280, 720];

/// Every preset on one sheet, Vanilla to Solar Ultra.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn solar_presets_sheet() {
    on_big_stack(|| {
        let mut shots = Vec::new();
        for level in Level::ALL {
            let Some((mut gpu, _profile)) = open(MAP, SIZE, None, &preset_cvars(level)) else {
                return;
            };
            // Let the lamp cache and the probes settle.
            let shot = frame(&mut gpu, 120);
            eprintln!(
                "solar preset {}: rated {:?} for this card",
                level.label(),
                level.rating(crate::graphics_quality::card_mib()),
            );
            shots.push(shot);
        }
        eprintln!("{}", sheet(&shots, 3, 640, "solar-presets").display());
    });
}

/// The time-of-day sky without real-time lighting: Vanilla Plus with `r_daySky` at
/// noon, dusk and night, beside the map's own sky.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn solar_day_sky_without_live_light() {
    on_big_stack(|| {
        let sky = crate::world_materials::shadows::day::live::SKY_CVAR;
        let mut shots = Vec::new();
        for (on, hour) in [("0", "12"), ("1", "12"), ("1", "19.5"), ("1", "23")] {
            let mut cvars = preset_cvars(Level::VanillaPlus);
            cvars.extend([(sky, on), ("r_dayHour", hour)]);
            let Some((mut gpu, _profile)) = open(MAP, SIZE, None, &cvars) else {
                return;
            };
            let shot = frame(&mut gpu, 30);
            eprintln!(
                "day sky {on} at {hour}: night factor {:.2}",
                gpu.context.dynamic_light_settings.day_clock().night_tint()
            );
            shots.push(shot);
        }
        eprintln!("{}", sheet(&shots, 2, 640, "solar-day-sky").display());
    });
}
