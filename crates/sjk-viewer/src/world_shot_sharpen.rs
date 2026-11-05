//! World shots of the scene sharpening (`r_sharpen`, [`crate::frame_target::aa::color`]):
//! mp/ffa3 from its start at each strength, then 21 sampled frames' GPU times
//! ([`crate::gpu_phases`], printed with `SJK_GPU_PHASES=1`). Off-screen renders (no game
//! window), ignored like the other world shots: they need a GPU adapter and the installed
//! game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer sharpen_shots --
//! --ignored --test-threads=1 --nocapture`; the pictures are in `target/world-shots`,
//! named `sharpen-<strength>`. `SJK_SHARPEN_SHOTS` lists the strengths (default
//! `0,50,100`) and `SJK_SHARPEN_FXAA` sets `r_fxaa` (default the renderer's, on).

use super::*;

const MAP: &str = "maps/mp/ffa3.bsp";

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn sharpen_shots() {
    on_big_stack(|| {
        let strengths =
            std::env::var("SJK_SHARPEN_SHOTS").unwrap_or_else(|_| "0,50,100".to_owned());
        let fxaa = std::env::var("SJK_SHARPEN_FXAA").ok();
        std::fs::create_dir_all(directory()).expect("the shot directory");
        for strength in strengths.split(',').map(str::trim) {
            let mut cvars = vec![(crate::frame_target::aa::color::SHARPEN_CVAR, strength)];
            if let Some(fxaa) = fxaa.as_deref() {
                cvars.push(("r_fxaa", fxaa));
            }
            let Some((mut gpu, _profile)) = open(MAP, [1920, 1080], None, &cvars) else {
                return;
            };
            let image = frame(&mut gpu, 40);
            let path = directory().join(format!("sharpen-{strength}.png"));
            image.save(&path).expect("write the shot");
            eprintln!("r_sharpen {strength}: {}", path.display());
            for _ in 0..32 * 21 {
                let _ = gpu.render(&mut None);
                let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
            }
        }
    });
}
