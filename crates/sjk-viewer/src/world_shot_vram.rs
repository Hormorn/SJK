//! GPU memory across in-game map changes: builds the next map on the running
//! world's context, the way the world-install worker does (`session_transition.rs`),
//! adopts it and drops the old world, again and again. After each change it prints
//! the allocator's live bytes and the labels that grew, so a world that outlives its
//! map shows up by name, and the worst peak (both worlds alive) with the textures
//! they shared ([`crate::shared_textures`]). Players' 2026.1013-2026.1018 builds ran out of video memory
//! building a map after an hour or more of map changes. Off-screen, ignored like the
//! other world shots: it needs a GPU adapter and `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer map_changes_vram --
//! --ignored --test-threads=1 --nocapture`. `SJK_TEST_PICMIP=<n>` measures it at that
//! texture detail (`r_picmip`).

use super::*;
use std::collections::BTreeMap;

const MAPS: [&str; 4] = [
    "maps/mp/ffa3.bsp",
    "maps/mp/ffa5.bsp",
    "maps/mp/siege_desert.bsp",
    "maps/mp/duel6.bsp",
];
const CHANGES: usize = 12;
const MIB: f64 = 1024.0 * 1024.0;

/// Live bytes per allocation label, and their total.
fn live(device: &wgpu::Device) -> (u64, BTreeMap<String, u64>) {
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let Some(report) = device.generate_allocator_report() else {
        return (0, BTreeMap::new());
    };
    eprintln!(
        "    reserved {:.0} MiB in {} blocks, DXGI process usage {:.0} MiB",
        report.total_reserved_bytes as f64 / MIB,
        report.blocks.len(),
        sjk_gpu_memory::high_performance()
            .and_then(|memory| memory.usage)
            .unwrap_or(0) as f64
            / MIB
    );
    let mut by_label = BTreeMap::new();
    for allocation in report.allocations {
        *by_label.entry(allocation.name).or_insert(0) += allocation.size;
    }
    (report.total_allocated_bytes, by_label)
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn map_changes_vram() {
    on_big_stack(|| {
        let size = [1920, 1080];
        let picmip = std::env::var("SJK_TEST_PICMIP").unwrap_or_else(|_| "0".to_owned());
        let cvars = [(
            crate::world_materials::filtering::mips::PICMIP_CVAR,
            picmip.as_str(),
        )];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &cvars) else {
            return;
        };
        let _ = frame(&mut gpu, 4);
        let (start, mut previous) = live(&gpu.device);
        eprintln!("vram: start {:.0} MiB", start as f64 / MIB);
        let mut after = Vec::new();
        let mut worst = 0;
        for change in 0..CHANGES {
            let map = MAPS[change % MAPS.len()];
            let (mut input, _next) = world_input(map, None, &[]);
            input.console = None;
            let built = pollster::block_on(GpuState::new_with_context(
                Arc::clone(&gpu.context),
                size,
                input,
                None,
            ))
            .expect("the next map is built");
            let (peak, _) = live(&gpu.device);
            worst = worst.max(peak);
            let mut adopted = gpu.adopt_world(Box::new(built));
            attach_target(&mut adopted, size);
            gpu = *adopted;
            let _ = frame(&mut gpu, 4);
            let (total, by_label) = live(&gpu.device);
            eprintln!(
                "vram: change {change:2} {map:28} peak {:6.0} MiB, after {:6.0} MiB, decoded images {:5.0} MiB",
                peak as f64 / MIB,
                total as f64 / MIB,
                crate::decoded_image_cache::held_bytes() as f64 / MIB
            );
            for (label, bytes) in &by_label {
                let before = previous.get(label).copied().unwrap_or(0);
                if *bytes > before + (1 << 20) && change >= MAPS.len() {
                    eprintln!(
                        "    grew: {label} {:.1} -> {:.1} MiB",
                        before as f64 / MIB,
                        *bytes as f64 / MIB
                    );
                }
            }
            if change < MAPS.len() {
                let mut largest: Vec<_> = by_label.iter().collect();
                largest.sort_by(|a, b| b.1.cmp(a.1));
                for (label, bytes) in largest.into_iter().take(8) {
                    eprintln!("    largest: {label} {:.1} MiB", *bytes as f64 / MIB);
                }
            }
            previous = by_label;
            after.push(total);
        }
        // The same map, a whole cycle apart, must not hold more than it did then.
        let first = after[MAPS.len() - 1];
        let last = after[after.len() - 1];
        eprintln!(
            "vram: duel6 after cycle 1 {:.0} MiB, after the last {:.0} MiB",
            first as f64 / MIB,
            last as f64 / MIB
        );
        eprintln!(
            "vram: worst peak {:.0} MiB; {} shared textures",
            worst as f64 / MIB,
            gpu.context.shared_textures.len()
        );
        assert!(
            last <= first + first / 20,
            "GPU memory grew across map changes: {first} -> {last} bytes"
        );
        let cache = Arc::clone(&gpu.context.shared_textures);
        drop(gpu);
        assert_eq!(cache.len(), 0, "shared textures outlived the last world");
    });
}

/// A server map change with and without the old map unloaded first
/// (`session_transition.rs`, [`GpuState::empty_world_input`]): the peak with ffa3 and
/// siege_desert alive at once, against an empty world and siege_desert. Run like
/// `map_changes_vram`.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn map_change_unloaded_vram() {
    on_big_stack(|| {
        let size = [1920, 1080];
        let Some((mut gpu, _profile)) = open("maps/mp/ffa3.bsp", size, None, &[]) else {
            return;
        };
        let _ = frame(&mut gpu, 4);
        let (alone, labels) = live(&gpu.device);
        let mut labels: Vec<_> = labels.into_iter().collect();
        labels.sort_by_key(|(_, bytes)| std::cmp::Reverse(*bytes));
        for (label, bytes) in labels.iter().take(30) {
            eprintln!("    ffa3 {:>8.1} MiB  {label}", *bytes as f64 / MIB);
        }
        let next = |gpu: &GpuState| {
            let (mut input, _next) = world_input("maps/mp/siege_desert.bsp", None, &[]);
            input.console = None;
            pollster::block_on(GpuState::new_with_context(
                Arc::clone(&gpu.context),
                size,
                input,
                None,
            ))
            .expect("the next map is built")
        };
        let built = next(&gpu);
        let (both, _) = live(&gpu.device);
        drop(built);
        let empty = pollster::block_on(GpuState::new_with_context(
            Arc::clone(&gpu.context),
            size,
            gpu.empty_world_input().expect("an empty world"),
            None,
        ))
        .expect("the empty world is built");
        let mut empty = gpu.adopt_world(Box::new(empty));
        attach_target(&mut empty, size);
        gpu = *empty;
        let _ = frame(&mut gpu, 4);
        let (unloaded, labels) = live(&gpu.device);
        let mut labels: Vec<_> = labels.into_iter().collect();
        labels.sort_by_key(|(_, bytes)| std::cmp::Reverse(*bytes));
        for (label, bytes) in labels.iter().take(30) {
            eprintln!("    empty {:>8.1} MiB  {label}", *bytes as f64 / MIB);
        }
        let built = next(&gpu);
        let (peak, _) = live(&gpu.device);
        drop(built);
        eprintln!(
            "vram: ffa3 {:.0} MiB; ffa3 + siege_desert {:.0} MiB; empty world {:.0} MiB; empty + siege_desert {:.0} MiB",
            alone as f64 / MIB,
            both as f64 / MIB,
            unloaded as f64 / MIB,
            peak as f64 / MIB
        );
        assert!(peak < both, "unloading first lowers the peak");
    });
}

/// The main menu on pictures, as a 3 GB card starts it (Vin's, 21/10/2026: out of
/// memory in the menu 18 s in): labels allocated more than three times, then the
/// biggest labels. Measured 21/10/2026 at 1920x1080, r_picmip 2: 1082 MiB, 801 MiB once
/// the saber glows followed r_picmip (they were 300 MiB). Run like `map_changes_vram`.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn menu_pictures_vram() {
    on_big_stack(|| {
        for style in ["classic", "sjk"] {
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", style),
                (crate::menu_pictures::CVAR, "1"),
                (crate::world_materials::filtering::mips::PICMIP_CVAR, "2"),
            ];
            let map = crate::menu_pictures::menu_map("maps/mp/duel6.bsp", true);
            let Some((mut gpu, _profile)) = open(&map, [1920, 1080], Some(menu), &cvars) else {
                return;
            };
            gpu.graphics_reload.map = map;
            crate::menu_pictures::fit_backdrop(&mut gpu);
            let _ = frame(&mut gpu, 120);
            if let Some(report) = gpu.device.generate_allocator_report() {
                let mut counts = BTreeMap::<String, (u64, u64)>::new();
                for allocation in &report.allocations {
                    let entry = counts.entry(allocation.name.clone()).or_default();
                    entry.0 += 1;
                    entry.1 = entry.1.max(allocation.size);
                }
                for (label, (count, biggest)) in counts.iter().filter(|(_, (c, _))| *c > 3) {
                    eprintln!(
                        "    count {count:>5} biggest {:>7.2} MiB  {label}",
                        *biggest as f64 / MIB
                    );
                }
            }
            let (total, by_label) = live(&gpu.device);
            eprintln!("vram {style}: {:.0} MiB", total as f64 / MIB);
            let mut labels: Vec<_> = by_label.into_iter().collect();
            labels.sort_by_key(|(_, bytes)| std::cmp::Reverse(*bytes));
            for (label, bytes) in labels.iter().take(25) {
                eprintln!("    {:>8.1} MiB  {label}", *bytes as f64 / MIB);
            }
        }
    });
}

/// Audit: every label an in-game map holds at each preset and texture detail, for the
/// 3 GB cards. `SJK_AUDIT_MAPS` (comma separated) and `SJK_AUDIT_SIZE` (`WxH`) override
/// the maps and the screen size; `SJK_AUDIT_GENERATED` mounts the player's generated
/// packs (`JKA_GAME_DATA/SJK/generated`, the material maps), which the other shots
/// leave out; `SJK_AUDIT_COMPRESSION` (comma separated, default `0`) measures each
/// `r_textureCompression` value. `SJK_VIDEO_MEMORY_MIB` builds the worlds as for a card of
/// that size (what follows it: the lamp cache budget, `video_memory::dedicated`). Each
/// build's last frame is saved as `vram-audit-<map>-<preset>-<picmip>` (`-tc1` with
/// compression).
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn low_end_vram_audit() {
    on_big_stack(|| {
        use crate::graphics_quality::{Level, preset_cvars};
        if std::env::var_os("SJK_AUDIT_GENERATED").is_some()
            && let Some(game_data) = std::env::var_os("JKA_GAME_DATA")
        {
            crate::assets::generated::locate_at_start(&PathBuf::from(game_data).join("SJK"));
        }
        let maps = std::env::var("SJK_AUDIT_MAPS")
            .unwrap_or_else(|_| "maps/mp/ffa3.bsp,maps/mp/siege_desert.bsp".to_owned());
        let size = std::env::var("SJK_AUDIT_SIZE")
            .ok()
            .and_then(|size| {
                let (w, h) = size.split_once('x')?;
                Some([w.parse().ok()?, h.parse().ok()?])
            })
            .unwrap_or([1920, 1080]);
        let empty = crate::menu_pictures::menu_map("maps/mp/duel6.bsp", true);
        let mut summary = Vec::new();
        eprintln!(
            "audit: DXGI process usage before any device {:.0} MiB",
            sjk_gpu_memory::high_performance()
                .and_then(|memory| memory.usage)
                .unwrap_or(0) as f64
                / MIB
        );
        let picmips: Vec<String> = std::env::var("SJK_AUDIT_PICMIPS")
            .unwrap_or_else(|_| "0,2".to_owned())
            .split(',')
            .map(str::to_owned)
            .collect();
        let compressions: Vec<String> = std::env::var("SJK_AUDIT_COMPRESSION")
            .unwrap_or_else(|_| "0".to_owned())
            .split(',')
            .map(str::to_owned)
            .collect();
        for map in maps.split(',').chain([empty.as_str()]) {
            for (level, name) in [
                (Level::Vanilla, "vanilla"),
                (Level::VanillaPlus, "sjk"),
                (Level::SolarHigh, "experimental"),
            ] {
                for (picmip, compression) in picmips.iter().flat_map(|picmip| {
                    compressions
                        .iter()
                        .map(move |compression| (picmip.as_str(), compression.as_str()))
                }) {
                    let mut cvars = preset_cvars(level);
                    cvars.push((crate::world_materials::filtering::mips::PICMIP_CVAR, picmip));
                    cvars.push((
                        crate::world_materials::filtering::mips::compress::CVAR,
                        compression,
                    ));
                    let tc = if compression == "0" {
                        String::new()
                    } else {
                        format!("-tc{compression}")
                    };
                    if crate::menu_pictures::pictured(map).is_some() {
                        cvars.push((crate::menu_pictures::CVAR, "1"));
                    }
                    let Some((mut gpu, _profile)) = open(map, size, None, &cvars) else {
                        return;
                    };
                    let shot = frame(&mut gpu, 8);
                    let stem = map
                        .trim_start_matches("maps/")
                        .trim_end_matches(".bsp")
                        .replace(['/', ':'], "-");
                    let _ = std::fs::create_dir_all(directory());
                    let _ = shot.save(
                        directory()
                            .join(format!("vram-audit-{stem}-{name}-picmip{picmip}{tc}.png")),
                    );
                    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                    let Some(report) = gpu.device.generate_allocator_report() else {
                        return;
                    };
                    let mut by_label = BTreeMap::<String, (u64, u64)>::new();
                    for allocation in &report.allocations {
                        let entry = by_label.entry(allocation.name.clone()).or_default();
                        entry.0 += 1;
                        entry.1 += allocation.size;
                    }
                    let total = report.total_allocated_bytes;
                    let dxgi = sjk_gpu_memory::high_performance()
                        .and_then(|memory| memory.usage)
                        .unwrap_or(0);
                    eprintln!(
                        "audit {map} {name} picmip {picmip}{tc}: {:.0} MiB in use, {:.0} MiB reserved, {} labels, DXGI process usage {:.0} MiB ({:?})",
                        total as f64 / MIB,
                        report.total_reserved_bytes as f64 / MIB,
                        by_label.len(),
                        dxgi as f64 / MIB,
                        gpu.context.adapter_backend
                    );
                    let mut labels: Vec<_> = by_label.into_iter().collect();
                    labels.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));
                    for (label, (count, bytes)) in labels.iter().take(45) {
                        eprintln!("    {:>8.1} MiB {count:>5}x  {label}", *bytes as f64 / MIB);
                    }
                    let rest: u64 = labels.iter().skip(45).map(|(_, (_, b))| b).sum();
                    eprintln!("    {:>8.1} MiB in the other labels", rest as f64 / MIB);
                    drop(gpu);
                    let after = sjk_gpu_memory::high_performance()
                        .and_then(|memory| memory.usage)
                        .unwrap_or(0);
                    summary.push(format!(
                        "{map:40} {name:12} picmip {picmip}{tc:4}: {:6.0} MiB allocator, {:6.0} MiB DXGI, {:6.0} MiB DXGI after drop",
                        total as f64 / MIB,
                        dxgi as f64 / MIB,
                        after as f64 / MIB
                    ));
                }
            }
        }
        for line in summary {
            eprintln!("summary: {line}");
        }
    });
}

/// Audit: the menus on pictures at r_picmip 2 (a 3 GB card's start) with the SJK packs
/// the hub serves mounted from `JKA_GAME_DATA/SJK/assets`, before and after enough frames
/// for every card face to upload (they uploaded whether drawn or not until 25/10/2026:
/// 539 -> 2316 MiB), then the next map built beside it (a map change's peak).
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn card_faces_vram_audit() {
    on_big_stack(|| {
        use crate::graphics_quality::{Level, preset_cvars};
        let game_data = std::env::var("JKA_GAME_DATA").expect("JKA_GAME_DATA");
        let directory = std::path::Path::new(&game_data).join("SJK").join("assets");
        let mut cvars = preset_cvars(Level::VanillaPlus);
        cvars.push((crate::world_materials::filtering::mips::PICMIP_CVAR, "2"));
        cvars.push((crate::menu_pictures::CVAR, "1"));
        let map = crate::menu_pictures::menu_map("maps/mp/duel6.bsp", true);
        let size = [1920, 1080];
        let Some((mut gpu, _profile)) = open(&map, size, None, &cvars) else {
            return;
        };
        let _ = frame(&mut gpu, 8);
        let (before, _) = live(&gpu.device);
        let _skins = crate::sjk_packs::mount(&directory);
        let count = crate::cards::faces::current().list.len();
        eprintln!("audit: {count} card faces mounted");
        let _ = frame(&mut gpu, (count as u32) / 2 + 20);
        let (after, by_label) = live(&gpu.device);
        let emblem = by_label.get("SJK card art").copied().unwrap_or(0);
        eprintln!(
            "audit: picture menu {:.0} MiB before the packs, {:.0} MiB after {count} faces (SJK card art {:.0} MiB)",
            before as f64 / MIB,
            after as f64 / MIB,
            emblem as f64 / MIB
        );
        let (mut input, _next) = world_input("maps/mp/siege_desert.bsp", None, &cvars);
        input.console = None;
        let built = pollster::block_on(GpuState::new_with_context(
            Arc::clone(&gpu.context),
            size,
            input,
            None,
        ))
        .expect("the next map is built");
        let (peak, _) = live(&gpu.device);
        eprintln!(
            "audit: map change peak (picture menu with faces + siege_desert) {:.0} MiB",
            peak as f64 / MIB
        );
        let mut adopted = gpu.adopt_world(Box::new(built));
        attach_target(&mut adopted, size);
        gpu = *adopted;
        let _ = frame(&mut gpu, (count as u32) / 2 + 20);
        let (in_map, by_label) = live(&gpu.device);
        eprintln!(
            "audit: siege_desert after its faces upload {:.0} MiB (SJK card art {:.0} MiB)",
            in_map as f64 / MIB,
            by_label.get("SJK card art").copied().unwrap_or(0) as f64 / MIB
        );
    });
}
