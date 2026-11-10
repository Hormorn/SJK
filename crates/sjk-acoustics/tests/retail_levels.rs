//! What the acoustics cost on retail levels: building the walls, a pass of the
//! room and of the straight ways, from a few places. Reads the installed game named
//! by `JKA_GAME_DATA` (its `GameData` folder); nothing is written or kept. Run in
//! release:
//! `cargo test --release -p sjk-acoustics --test retail_levels -- --ignored --nocapture`

use glam::Vec3;
use sjk_acoustics::direct::DirectTracer;
use sjk_acoustics::material::ShaderMaterials;
use sjk_acoustics::room::{RoomSettings, RoomTracer};
use sjk_acoustics::scene::{LevelGeometry, Scene};
use sjk_vfs::VirtualFileSystem;
use std::time::Instant;

/// Levels and a few places in each (player spawn heights).
const LEVELS: &[(&str, &[[f32; 3]])] = &[
    (
        "maps/mp/ffa3.bsp",
        &[
            [0.0, 0.0, 100.0],
            [600.0, -400.0, 40.0],
            [-900.0, 300.0, 150.0],
        ],
    ),
    (
        "maps/mp/ffa5.bsp",
        &[[0.0, 0.0, 100.0], [-500.0, 500.0, 60.0]],
    ),
    ("maps/mp/duel1.bsp", &[[0.0, 0.0, 60.0]]),
];

#[test]
#[ignore = "reads the installed game data named by JKA_GAME_DATA"]
fn retail_levels_build_and_trace() {
    let Ok(data) = std::env::var("JKA_GAME_DATA") else {
        return;
    };
    let mut vfs = VirtualFileSystem::new();
    for index in 0..4 {
        let archive = std::path::Path::new(&data).join(format!("base/assets{index}.pk3"));
        vfs.mount_pk3(&archive).unwrap();
    }
    let materials = ShaderMaterials::bundled();
    for (path, places) in LEVELS {
        let bytes = vfs.read(path).unwrap().unwrap().bytes;
        let bsp = sjk_bsp::Bsp::parse(&bytes).unwrap();
        let started = Instant::now();
        let geometry = LevelGeometry::from_bsp(&bsp, &materials);
        let built = started.elapsed();
        println!(
            "{path}: {} triangles ({} named), built in {:.1} ms",
            geometry.triangle_count(),
            geometry.named_triangles,
            built.as_secs_f64() * 1e3
        );
        let scene = Scene {
            geometry: &geometry,
            movers: &[],
        };
        let mut tracer = RoomTracer::new();
        let direct = DirectTracer::default();
        let axes = [Vec3::X, -Vec3::Y, Vec3::Z];
        for place in *places {
            let listener = Vec3::from_array(*place);
            for settings in [
                RoomSettings {
                    rays: 1_024,
                    bounces: 16,
                    threads: 2,
                },
                RoomSettings::default(),
                RoomSettings {
                    rays: 4_096,
                    bounces: 32,
                    threads: 4,
                },
            ] {
                tracer.forget();
                let (response, stats) = tracer.trace(&scene, listener, axes, settings);
                println!(
                    "  {place:?} room {} rays x {} bounces, {} threads: {:.1} ms, enclosed {:.2}, \
                     decay {:.2} s, {:.2} s long ({} parts), energy {:.2}",
                    settings.rays,
                    settings.bounces,
                    settings.threads,
                    stats.micros as f64 / 1e3,
                    stats.enclosed,
                    stats.decay_seconds,
                    stats.length_seconds,
                    response.partitions(),
                    stats.energy
                );
            }
            let started = Instant::now();
            let mut hidden = 0;
            for index in 0..48 {
                let angle = index as f32 * 0.4;
                let source = listener
                    + Vec3::new(angle.cos(), angle.sin(), 0.0) * (200.0 + index as f32 * 30.0);
                if direct.trace(&scene, listener, source, None).visible < 0.5 {
                    hidden += 1;
                }
            }
            println!(
                "  {place:?} 48 sounds' straight ways: {:.2} ms ({hidden} hidden)",
                started.elapsed().as_secs_f64() * 1e3
            );
        }
    }
}
