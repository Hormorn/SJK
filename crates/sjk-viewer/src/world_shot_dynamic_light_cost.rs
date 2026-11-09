//! GPU cost shots for the graphics settings work, off screen under the renderer's defaults
//! (Experimental) on mp/ffa3 from its start. Ignored like the other world shots: they need
//! a GPU adapter and the installed game data named by `JKA_GAME_DATA`.
//!
//! - `dynamic_light_shadow_cost`: the point lights' wall shadows
//!   (`dynamic_light_shadows.rs`, the `dlight-shadows` section) with a saber fight's worth
//!   of lights around the camera (`SJK_COST_LIGHTS` pairs, default 12), and a still at a
//!   fixed scene time to compare two builds' pictures.
//! - `frame_section_cost`: every section's median, to compare two builds or settings
//!   (`SJK_SHOT_CVARS`).
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer --bin sjk <name> --
//! --ignored --test-threads=1 --nocapture`. `SJK_COST_SIZE` sets the window (`WxH`,
//! default 3840x2160).

use super::*;
use crate::dynamic_lights::PointLight;

const MAP: &str = "maps/mp/ffa3.bsp";
const FRAMES: usize = 240;
const WARM_UP: u32 = 600;

fn size() -> [u32; 2] {
    std::env::var("SJK_COST_SIZE")
        .ok()
        .and_then(|size| {
            let (w, h) = size.split_once('x')?;
            Some([w.parse().ok()?, h.parse().ok()?])
        })
        .unwrap_or([3840, 2160])
}

/// Render `frames` frames back to back and return each section's median milliseconds in
/// frame order, then the whole frame's (`total`), over the sampled frames.
fn section_medians(gpu: &mut GpuState, frames: usize) -> Vec<(&'static str, f32)> {
    let mut sections: Vec<(&'static str, Vec<f32>)> = Vec::new();
    let mut total = Vec::new();
    for _ in 0..frames {
        // The frame timings readout turns the sections off each frame while
        // `r_drawTimings` is off: on again before the frame starts and before its read.
        if let Some(phases) = &gpu.gpu_phases {
            phases.show(true);
        }
        let _ = gpu.render(&mut None);
        // Frames are submitted on their own thread: wait for this one first.
        gpu.frame_pacer.split.wait_previous();
        let Some(phases) = &gpu.gpu_phases else {
            break;
        };
        // Map the sampled frame's readback now and read it once it lands, rather than at
        // the next frame's acquire.
        phases.after_submit();
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        phases.show(true);
        phases.report();
        let mut frame: Vec<(&'static str, f32)> = Vec::new();
        let Some(all) = phases.take_sections(|name, ms| {
            match frame.iter_mut().find(|(seen, _)| *seen == name) {
                Some((_, sum)) => *sum += ms,
                None => frame.push((name, ms)),
            }
        }) else {
            continue;
        };
        total.push(all);
        for (name, ms) in frame {
            match sections.iter_mut().find(|(seen, _)| *seen == name) {
                Some((_, values)) => values.push(ms),
                None => sections.push((name, vec![ms])),
            }
        }
    }
    if let Some(phases) = &gpu.gpu_phases {
        phases.show(false);
    }
    let samples = total.len();
    let median = |mut values: Vec<f32>| {
        // A section missing from a sampled frame counts as 0 there.
        values.resize(samples.max(values.len()), 0.0);
        values.sort_by(f32::total_cmp);
        values.get(values.len() / 2).copied().unwrap_or(f32::NAN)
    };
    let mut medians: Vec<_> = sections
        .into_iter()
        .map(|(name, values)| (name, median(values)))
        .collect();
    medians.push(("total", median(total)));
    medians
}

/// One frame at the same scene time each call (the clock is `ui_epoch`'s age), so two
/// stills differ only by what the renderer was told.
fn still(gpu: &mut GpuState) -> image::RgbaImage {
    gpu.ui_epoch = std::time::Instant::now() - std::time::Duration::from_secs(60);
    let _ = gpu.render(&mut None);
    let texture = gpu.headless_frame.as_ref().expect("a world shot target");
    read_back(&gpu.device, &gpu.queue, texture)
}

fn print_medians(label: &str, medians: &[(&'static str, f32)]) {
    let line: Vec<String> = medians
        .iter()
        .filter(|(_, ms)| *ms >= 0.005)
        .map(|(name, ms)| format!("{name}={ms:.3}"))
        .collect();
    println!("{label}: {}", line.join(" "));
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn dynamic_light_shadow_cost() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open(MAP, size(), None, &[]) else {
            return;
        };
        // Past the map's first reflection probe captures.
        let _ = frame(&mut gpu, WARM_UP);
        let eye = gpu.camera_position;
        let forward = Vec3::new(gpu.camera_yaw.cos(), gpu.camera_yaw.sin(), 0.0);
        let side = Vec3::new(-forward.y, forward.x, 0.0);
        // codemp `CG_AddSaberBlade`'s light: twice the blade's length, here 200 units.
        let saber = |at: Vec3, color: [f32; 3]| PointLight {
            origin: at.to_array(),
            radius: 200.0,
            color,
        };
        let lights = &mut gpu.effect_aux.held_lights;
        lights.clear();
        let pairs: usize = std::env::var("SJK_COST_LIGHTS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(12);
        for n in 0..pairs {
            let across = (n % 4) as f32 - 1.5;
            let ahead = 120.0 + 90.0 * (n / 4) as f32;
            lights.push(saber(
                eye + forward * ahead + side * across * 70.0,
                [0.2, 0.4, 1.0],
            ));
            let behind = 260.0 + 90.0 * (n / 4) as f32;
            lights.push(saber(
                eye - forward * behind + side * across * 70.0,
                [1.0, 0.2, 0.2],
            ));
        }
        std::fs::create_dir_all(directory()).expect("the shot directory");
        let _ = frame(&mut gpu, 20);
        print_medians("lights", &section_medians(&mut gpu, FRAMES));
        let path = directory().join("dlight-cost.png");
        still(&mut gpu).save(&path).expect("write the shot");
        println!("{}", path.display());
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn frame_section_cost() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open(MAP, size(), None, &[]) else {
            return;
        };
        let _ = frame(&mut gpu, WARM_UP);
        std::fs::create_dir_all(directory()).expect("the shot directory");
        let tag = std::env::var("SJK_SHOT_TAG").unwrap_or_else(|_| "shot".to_owned());
        for run in 0..3 {
            print_medians(
                &format!("{tag} run {run}"),
                &section_medians(&mut gpu, FRAMES),
            );
        }
        let path = directory().join(format!("frame-cost-{tag}.png"));
        still(&mut gpu).save(&path).expect("write the shot");
        println!("{}", path.display());
    });
}
