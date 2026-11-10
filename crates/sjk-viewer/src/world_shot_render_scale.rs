//! World shots of the render scale (`r_renderScale`, [`crate::frame_target::scale`]):
//! mp/ffa3 from its start at each scale, the world alone and under the SJK UI's main
//! page (whose 2D layer stays at the window's size), with the scene's size, then 21
//! sampled frames' GPU times ([`crate::gpu_phases`], printed with `SJK_GPU_PHASES=1`).
//! Off-screen renders (no game window), ignored like the other world shots: they need a
//! GPU adapter and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer render_scale_shots --
//! --ignored --test-threads=1 --nocapture`; the pictures are in `target/world-shots`,
//! named `render-scale-<tag>-<percent>` and `render-scale-menu-<tag>-<percent>`.
//! `SJK_RENDER_SCALE_SHOTS` lists the percents (default `100,75,50`), `SJK_SHOT_TAG`
//! names the run (default `shot`, so two builds' pictures can be compared),
//! `SJK_RENDER_SCALE_SIZE` the window (`WxH`, default 1920x1080) and
//! `SJK_RENDER_SCALE_PRESET` a graphics preset (`vanilla`, `sjk`; default the
//! renderer's own defaults, Experimental).

use super::*;
use std::time::Duration;

const MAP: &str = "maps/mp/ffa3.bsp";

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn render_scale_shots() {
    on_big_stack(|| {
        let size = std::env::var("SJK_RENDER_SCALE_SIZE")
            .ok()
            .and_then(|size| {
                let (w, h) = size.split_once('x')?;
                Some([w.parse().ok()?, h.parse().ok()?])
            })
            .unwrap_or([1920, 1080]);
        let tag = std::env::var("SJK_SHOT_TAG").unwrap_or_else(|_| "shot".to_owned());
        let percents =
            std::env::var("SJK_RENDER_SCALE_SHOTS").unwrap_or_else(|_| "100,75,50".to_owned());
        let preset = match std::env::var("SJK_RENDER_SCALE_PRESET").as_deref() {
            Ok("vanilla") => {
                crate::graphics_quality::preset_cvars(crate::graphics_quality::Level::Vanilla)
            }
            Ok("sjk") => {
                crate::graphics_quality::preset_cvars(crate::graphics_quality::Level::VanillaPlus)
            }
            _ => Vec::new(),
        };
        std::fs::create_dir_all(directory()).expect("the shot directory");
        for percent in percents.split(',').map(str::trim) {
            let mut cvars: Vec<(&str, &str)> = preset.to_vec();
            cvars.push((crate::frame_target::scale::CVAR, percent));
            let Some((mut gpu, _profile)) = open(MAP, size, None, &cvars) else {
                return;
            };
            let image = frame(&mut gpu, 40);
            let path = directory().join(format!("render-scale-{tag}-{percent}.png"));
            image.save(&path).expect("write the shot");
            eprintln!(
                "render scale {percent}: window {size:?}, scene {:?}: {}",
                gpu.scene_size(),
                path.display()
            );
            timed_frames(&mut gpu, 32 * 21);
            drop(gpu);

            let menu = menu::ClientMenu::new(true, String::new());
            cvars.push(("ui_menuStyle", "sjk"));
            cvars.push((crate::settings::quick::HIDE_CVAR, "1"));
            let Some((mut gpu, _profile)) = open(MAP, size, Some(menu), &cvars) else {
                return;
            };
            let _ = frame(&mut gpu, 20);
            gpu.ui_epoch -= Duration::from_millis(2_000);
            let path = shoot(&mut gpu, 4, &format!("render-scale-menu-{tag}-{percent}"));
            println!("{}", path.display());
        }
    });
}

/// Render `frames` frames back to back, polling the device so each sampled frame's
/// timestamps land at once: with `SJK_GPU_PHASES=1` every 32nd frame's GPU times are
/// printed (`gpu-phases ... total_ms=...`), after the shot's line.
fn timed_frames(gpu: &mut GpuState, frames: u32) {
    for _ in 0..frames {
        let _ = gpu.render(&mut None);
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
    }
}
