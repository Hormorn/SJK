//! World shots of the frame timings panel ([`crate::frame_timings`]) over duel6: the
//! graphics card's sections with the processor's phases under them (`r_drawTimings 2`),
//! then the graphics card's alone (1) with the log's line. Off-screen renders (no game
//! window), ignored like the other world shots: they need a GPU adapter and the installed
//! game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer frame_timings_panel
//! -- --ignored --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.

use super::*;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn frame_timings_panel() {
    on_big_stack(|| {
        let size = [1920, 1080];
        let cvars = [(crate::frame_timings::DRAW_CVAR, "2")];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &cvars) else {
            return;
        };
        // A second of frames: the panel shows its first half second's means.
        let _ = frame(&mut gpu, 60);
        assert!(gpu.frame_timings.readout_ready(), "the panel has numbers");
        shoot(&mut gpu, 2, "frame-timings-gpu-cpu");

        if let Some(console) = gpu.console.as_mut() {
            console.set_cvar(crate::frame_timings::DRAW_CVAR, "1");
            console.set_cvar(crate::frame_timings::LOG_CVAR, "1");
        }
        let _ = frame(&mut gpu, 40);
        shoot(&mut gpu, 2, "frame-timings-gpu");
        if let Some(console) = gpu.console.as_mut() {
            console.set_cvar(crate::frame_timings::LOG_CVAR, "0");
            // Moved as the HUD edit mode keeps it: 100 units left, 40 down.
            console.set_cvar(crate::hud::placement::OFFSETS_CVAR, "timings:-100,40");
        }
        let _ = frame(&mut gpu, 4);
        shoot(&mut gpu, 2, "frame-timings-moved");
    });
}
