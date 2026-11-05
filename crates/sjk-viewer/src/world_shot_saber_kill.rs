//! World shots of the Disintegrate finisher burning saber kills away (`crate::finishers`,
//! [`crate::disintegration::Forced`]): on a local game of duel6 with a bot (its server
//! named by `JKA_DEDICATED`), the player wears the finisher with a Saber Lab blade and cuts the
//! bot down, standing before it and swinging; the bot burns away as a disruptor's kill
//! does, and once it respawns no body is left where it fell. Off-screen renders, ignored
//! like the other world shots: they need a GPU adapter, the installed game data named by
//! `JKA_GAME_DATA` and `sjk-server`.
//!
//! Run: `JKA_GAME_DATA=<GameData> JKA_DEDICATED=<sjk-server> cargo test -p sjk-viewer
//! saber_kill_disintegrates -- --ignored --nocapture`; the pictures are in
//! `target/world-shots`.

use super::graphics_reload::{step, until};
use super::*;
use std::time::{Duration, Instant};

/// How far from the bot the player is stepped when it is farther than `FARTHEST`
/// (units): well off its path, for it runs at them and a step onto it telefrags it; it
/// comes on to fight. How often (frames) that is looked at.
const BEFORE: f32 = 96.0;
const FARTHEST: f32 = 160.0;
const CATCH_UP: u32 = 30;
/// Nearer than this (units) the player stops walking in and only swings.
const SWING: f32 = 52.0;

#[test]
#[ignore = "renders with the GPU, the installed game data named by JKA_GAME_DATA and a local sjk-server named by JKA_DEDICATED"]
fn saber_kill_disintegrates() {
    on_big_stack(|| {
        let size = [1280, 720];
        // The lab blade (the test sample), worn with the Disintegrate finisher.
        let folder = tempfile::tempdir().unwrap();
        let blades = folder.path().join(crate::blade_skin_file::FOLDER);
        std::fs::create_dir_all(&blades).unwrap();
        std::fs::write(
            blades.join("saber_lab.bladeskin"),
            crate::blade_skin_file::tests::SAMPLE,
        )
        .unwrap();
        crate::saber_lab::start(folder.path()).expect("the lab's folder");
        super::saber_skins::mount_test_packs();
        let menu = menu::ClientMenu::new(true, String::new());
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, crate::saber_lab::ID),
            (crate::finishers::CVAR, "finisher_disintegrate"),
        ];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, Some(menu), &cvars) else {
            return;
        };
        // The shots have no hub: they own the unlocks, as a profile listing them would.
        gpu.looks.shot_owns_unlocks = true;
        let mut parked = crate::menu_world::Parked::new();
        gpu.start_local_game(crate::connection::local_server::HostSettings {
            map: "mp/duel6".into(),
            gametype: "ffa",
            hostname: "SJK saber kill".into(),
            bots: vec!["Jedi".into()],
            bot_skill: 1,
            fraglimit: 0,
            timelimit: 0,
            capturelimit: 0,
            allow_lan: false,
            cheats: true,
        });
        until(&mut gpu, &mut parked, size, 120, "joined duel6", |gpu| {
            gpu.live_map_installed && gpu.live_session.is_some()
        });
        until(&mut gpu, &mut parked, size, 60, "the bot in", |gpu| {
            bot(gpu).is_some()
        });
        let (number, _, _) = bot(&gpu).expect("the bot");
        let queue = |gpu: &mut GpuState, command: &str| {
            gpu.console
                .as_mut()
                .expect("a console")
                .queue_command(command)
                .expect("the command queues");
        };
        // Cut it down: before it, facing it, swinging, until the kill burns it away.
        queue(&mut gpu, "give armor");
        queue(&mut gpu, "+attack");
        let started = Instant::now();
        let mut frames = 0;
        let mut walking = false;
        loop {
            let now = gpu.presentation_clock.sample(Instant::now());
            if gpu.forced_disintegrations.active(number, now).is_some() {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(150),
                "the bot cut down within 150 s"
            );
            if let Some((_, at, dead)) = bot(&gpu)
                && !dead
            {
                let me = gpu
                    .live_session
                    .as_ref()
                    .map(|session| session.latest_snapshot().player.origin())
                    .unwrap_or(at);
                let mut way = [at[0] - me[0], at[1] - me[1]];
                let length = (way[0] * way[0] + way[1] * way[1]).sqrt();
                way = if length > 1.0 {
                    [way[0] / length, way[1] / length]
                } else {
                    [1.0, 0.0]
                };
                // Facing it, level, every frame (the command's angles are the camera's),
                // walking in until within a swing of it.
                gpu.camera_yaw = way[1].atan2(way[0]);
                gpu.camera_pitch = 0.0;
                let walk = length > SWING;
                if walk != walking {
                    queue(&mut gpu, if walk { "+forward" } else { "-forward" });
                    walking = walk;
                }
                if frames % CATCH_UP == 0 && length > FARTHEST {
                    let step_in = format!(
                        "setviewpos {} {} {} {}",
                        at[0] - way[0] * BEFORE,
                        at[1] - way[1] * BEFORE,
                        at[2] + 1.0,
                        gpu.camera_yaw.to_degrees()
                    );
                    queue(&mut gpu, "give armor");
                    queue(&mut gpu, &step_in);
                }
            }
            step(&mut gpu, &mut parked, size);
            frames += 1;
        }
        queue(&mut gpu, "-attack");
        queue(&mut gpu, "-forward");
        eprintln!("cut down: {:.1} s", started.elapsed().as_secs_f32());
        // It burns away, and where it fell nothing comes back as it respawns: frames read
        // back as they come (saving one takes longer than the burn), kept for a contact
        // sheet, until a second after it is back.
        let (_, start) = gpu
            .forced_disintegrations
            .active(number, gpu.presentation_clock.sample(Instant::now()))
            .expect("burning");
        // Stepped back from where it fell, facing it, to see it whole (it is dead: no
        // telefrag).
        if let Some((_, at, _)) = bot(&gpu) {
            let me = gpu
                .live_session
                .as_ref()
                .map(|session| session.latest_snapshot().player.origin())
                .unwrap_or(at);
            let way = [at[0] - me[0], at[1] - me[1]];
            let length = (way[0] * way[0] + way[1] * way[1]).sqrt().max(1.0);
            let way = [way[0] / length, way[1] / length];
            let yaw = way[1].atan2(way[0]);
            gpu.camera_yaw = yaw;
            gpu.camera_pitch = -0.25;
            let back = format!(
                "setviewpos {} {} {} {}",
                at[0] - way[0] * 140.0,
                at[1] - way[1] * 140.0,
                at[2] + 1.0,
                yaw.to_degrees()
            );
            queue(&mut gpu, &back);
        }
        let mut burning = Vec::new();
        let mut back_at = None;
        loop {
            let image = frame(&mut gpu, 2);
            let into = gpu.presentation_clock.sample(Instant::now()) - start;
            if back_at.is_none() && bot(&gpu).is_some_and(|(_, _, dead)| !dead) {
                back_at = Some(into);
            }
            // Its body copy, once the snapshot holds it: hidden from the first frame.
            let body = gpu.live_session.as_ref().and_then(|session| {
                session
                    .latest_snapshot()
                    .entities
                    .iter()
                    .find(|state| state.entity_type() == 15 && state.client_num() == number)
                    .map(|state| state.number())
            });
            let now = gpu.presentation_clock.sample(Instant::now());
            // Hidden: burnt out before it shows, never drawn.
            let hidden = body.map(|body| {
                gpu.forced_disintegrations
                    .active(body, now)
                    .is_some_and(|(_, started)| {
                        now - started > crate::disintegration::PLAYER_MILLIS
                    })
            });
            eprintln!(
                "burning: {into} ms{}, body {body:?} hidden {hidden:?}",
                if back_at.is_some() { ", back" } else { "" }
            );
            assert_ne!(
                hidden,
                Some(false),
                "its body copy hidden from its first frame"
            );
            burning.push(image);
            if back_at.is_some_and(|back| into > back + 1_000) || burning.len() >= 30 {
                break;
            }
        }
        sheet(&burning, 6, 480, "saber-kill-burning");
        // It respawns, and no body is left where it fell.
        until(&mut gpu, &mut parked, size, 60, "the bot back", |gpu| {
            bot(gpu).is_some_and(|(_, _, dead)| !dead)
        });
        let _ = frame(&mut gpu, 20);
        shoot(&mut gpu, 4, "saber-kill-respawned");
        // The body the server copied of it as it respawned is the one hidden.
        assert_eq!(
            gpu.forced_disintegrations.hidden_bodies(),
            1,
            "its body hidden"
        );
        crate::saber_lab::stop();
    });
}

/// The bot in the latest snapshot: its number, where it is and whether it is dead.
fn bot(gpu: &GpuState) -> Option<(u16, [f32; 3], bool)> {
    let snapshot = gpu.live_session.as_ref()?.latest_snapshot();
    let me = snapshot.player.client_num();
    snapshot
        .entities
        .iter()
        .find(|state| {
            usize::from(state.number()) < crate::saber_skins::MAX_CLIENTS
                && state.number() != me
                && state.entity_type() == 1
        })
        .map(|state| {
            (
                state.number(),
                state.trajectory_base(),
                state.e_flags() & (1 << 1) != 0,
            )
        })
}
