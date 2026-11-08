//! World shots of a rig model ([`crate::rig_model`]) on a recorded game: the demo
//! `SJK_TEST_DEMO` names is played without a server, one player is given the
//! appearance `SJK_TEST_RIG` names (a model folder holding `sjk_rig.cfg`) through the
//! mid-match model change path, and the camera orbits them. Ignored like the other
//! world shots; skipped when either variable is not set.

use super::*;

#[test]
#[ignore = "renders with the GPU, the installed game data and the demo SJK_TEST_DEMO names"]
fn rig_model_on_a_demo() {
    on_big_stack(|| {
        let (Some(demo), Some(rig)) = (
            std::env::var_os("SJK_TEST_DEMO").map(PathBuf::from),
            std::env::var("SJK_TEST_RIG").ok(),
        ) else {
            println!("SJK_TEST_DEMO or SJK_TEST_RIG is not set: skipped");
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
        let Some(mut gpu) = card_outline_shots::open_demo(
            &demo,
            demo_playback::Camera::Orbit(target),
            [2560, 1920],
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        let appearance = Appearance {
            model: format!("models/players/{rig}"),
            // SJK_TEST_VARIANT: one of the model's variants (sjk_rig_<variant>.cfg).
            variant: std::env::var("SJK_TEST_VARIANT").unwrap_or_else(|_| "default".to_owned()),
        };
        let entity = EntityId::new(u64::from(target) + 1);
        let names = [Some("single_1".to_owned()), None];
        let mesh = gpu
            .build_live_actor(&appearance, entity, names)
            .expect("the rig model loads and uploads");
        assert!(mesh.preview.rig.is_some(), "{rig} is not a rig model");
        gpu.place_client_mesh(target, mesh, &appearance);
        let _ = frame(&mut gpu, 5);
        for mesh in gpu
            .actor_meshes
            .iter()
            .filter(|mesh| mesh.entity_id.is_some())
        {
            let id = mesh.entity_id.map(|id| id.get() - 1);
            let at = mesh
                .entity_id
                .and_then(|id| gpu.demo_session.as_ref()?.world().entity(id))
                .map(|entity| entity.sample(0).translation);
            println!(
                "actor {id:?} {} rig={} draws={} palette={} requested={} at {at:?}",
                mesh.appearance.model,
                mesh.preview.rig.is_some(),
                mesh.draws.len(),
                mesh.gpu_palette.is_some(),
                mesh.animator.requested.is_some(),
            );
        }
        println!("camera {:?} target {target}", gpu.camera_position);
        // SJK_TEST_EMOTE: the player plays that emote of the installed catalogue (a
        // rig clip), shots taken while it runs.
        let emote = std::env::var("SJK_TEST_EMOTE").ok();
        if let Some(id) = &emote {
            let mut state = crate::emotes::lock();
            let catalogue = crate::emotes::load(gpu.vfs.as_deref().expect("the game's files"));
            let def = catalogue.iter().find(|def| &def.id == id).cloned();
            println!("emote {id}: {def:?}");
            let slot = u8::try_from(target).expect("a player slot");
            let at = gpu
                .demo_session
                .as_ref()
                .and_then(|session| session.world().entity(entity))
                .map(|entity| entity.current().transform.translation)
                .unwrap_or_default();
            state.catalogue = Some(catalogue);
            state
                .active
                .start(slot, id, def.as_ref(), Some(at), std::time::Instant::now());
        }
        let between = if emote.is_some() { 6 } else { 50 };
        // Close-ups about a second apart: the orbit keeps the player near the centre.
        let mut shots = Vec::new();
        for _ in 0..12 {
            let image = frame(&mut gpu, between);
            let (width, height) = image.dimensions();
            let side = height / 2;
            shots.push(
                image::imageops::crop_imm(&image, (width - side) / 2, height / 4, side, side)
                    .to_image(),
            );
        }
        let name = if emote.is_some() {
            "rig_model_emote"
        } else {
            "rig_model_orbit"
        };
        println!("{}", sheet(&shots, 4, 480, name).display());
    });
}

/// `rig_reload` on a recorded game: every player wears the rig model `SJK_TEST_RIG`
/// names (`cg_forcemodel`), its pack is replaced by a copy at half its scale in a
/// GameData of hard links to the installed one, and after `rig_reload` the players
/// wear the new copy without a map load. Before/after in
/// `target/world-shots/rig_reload.png`.
#[test]
#[ignore = "renders with the GPU, the installed game data and the demo SJK_TEST_DEMO names"]
fn rig_reload_reads_a_replaced_pack_without_a_map_load() {
    on_big_stack(|| {
        let (Some(demo), Some(rig), Some(game_data)) = (
            std::env::var_os("SJK_TEST_DEMO").map(PathBuf::from),
            std::env::var("SJK_TEST_RIG").ok(),
            std::env::var_os("JKA_GAME_DATA").map(PathBuf::from),
        ) else {
            println!("SJK_TEST_DEMO, SJK_TEST_RIG or JKA_GAME_DATA is not set: skipped");
            return;
        };
        let pack_name = format!("rig_{rig}.pk3");
        let pack = game_data.join("base").join(&pack_name);
        let manifest_path = format!("models/players/{rig}/sjk_rig.cfg");
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
        let Some(mut gpu) =
            card_outline_shots::open_demo(&demo, demo_playback::Camera::Orbit(target), [1280, 960])
        else {
            return;
        };
        let model = format!("models/players/{rig}");
        let entity = EntityId::new(u64::from(target) + 1);
        let worn = |gpu: &GpuState| {
            gpu.actor_meshes
                .iter()
                .find(|mesh| !mesh.corpse_pool && mesh.entity_id == Some(entity))
                .filter(|mesh| mesh.appearance.model == model)
                .and_then(|mesh| mesh.preview.rig.clone())
        };
        let run = |gpu: &mut GpuState, frames: u32| {
            for _ in 0..frames {
                gpu.run_client_commands(&mut None);
                let _ = frame(gpu, 1);
            }
        };
        gpu.console
            .as_mut()
            .expect("a console")
            .set_cvars([("model", rig.as_str()), ("cg_forcemodel", "1")]);
        let mut before = None;
        for _ in 0..600 {
            run(&mut gpu, 1);
            before = worn(&gpu);
            if before.is_some() {
                break;
            }
        }
        let before = before.expect("the player wears the rig model");
        let _ = frame(&mut gpu, 30);
        let first = frame(&mut gpu, 1);

        // The replaced pack: the same model at half its scale.
        let archive = std::fs::read(&pack).expect("the installed rig pack");
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).expect("a zip");
        let install = tempfile::tempdir().expect("a temporary GameData");
        let base = install.path().join("base");
        std::fs::create_dir_all(&base).unwrap();
        let mut out = zip::ZipWriter::new(std::fs::File::create(base.join(&pack_name)).unwrap());
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).unwrap();
            let name = entry.name().to_owned();
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
            if name.eq_ignore_ascii_case(&manifest_path) {
                let text = String::from_utf8(bytes).unwrap();
                let halved: Vec<String> = text
                    .lines()
                    .map(|line| match line.trim().strip_prefix("scale ") {
                        Some(value) => {
                            let scale: f32 = value.trim().parse().expect("a number scale");
                            format!("scale {}", scale / 2.0)
                        }
                        None => line.to_owned(),
                    })
                    .collect();
                bytes = (halved.join("\n") + "\n").into_bytes();
            }
            out.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut out, &bytes).unwrap();
        }
        out.finish().unwrap();
        for file in std::fs::read_dir(game_data.join("base")).unwrap().flatten() {
            let name = file.file_name();
            if name.to_string_lossy().eq_ignore_ascii_case(&pack_name)
                || !file.file_type().is_ok_and(|kind| kind.is_file())
            {
                continue;
            }
            if let Err(error) = std::fs::hard_link(file.path(), base.join(&name)) {
                println!("cannot link {name:?} into a temporary GameData ({error}): skipped");
                return;
            }
        }
        gpu.game_data = install.path().to_path_buf();
        gpu.console
            .as_mut()
            .expect("a console")
            .queue_command(crate::rig_reload::COMMAND)
            .expect("the command queues");
        // A debug build reads a model for seconds, after the players' first loads.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
        let mut after = None;
        while std::time::Instant::now() < deadline {
            run(&mut gpu, 1);
            after = worn(&gpu).filter(|rig| !Arc::ptr_eq(rig, &before));
            if after.is_some() {
                break;
            }
        }
        let after = after.expect("rig_reload loads the replaced pack");
        let ratio = after.top() / before.top();
        println!("top {} -> {} (ratio {ratio})", before.top(), after.top());
        assert!((ratio - 0.5).abs() < 0.05, "ratio {ratio}");
        let _ = frame(&mut gpu, 30);
        let second = frame(&mut gpu, 1);
        println!(
            "{}",
            sheet(&[first, second], 2, 640, "rig_reload").display()
        );
    });
}
