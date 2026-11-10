//! Block-compressed textures (`r_textureCompression`) measured against RGBA8: what a map
//! looks like with and without them (off-screen frames from the same cameras, two runs
//! with it off giving the noise between runs), how far each of the map's textures moves
//! when encoded (PSNR of level zero, worst ones named, BC7 beside the formats used), and
//! how long a world takes to build each way. Ignored like the other world shots: they need
//! a GPU adapter and `JKA_GAME_DATA`. Video memory is `low_end_vram_audit`'s
//! (`SJK_AUDIT_COMPRESSION=0,1`, `world_shot_vram.rs`).
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer texture_compression_
//! -- --ignored --test-threads=1 --nocapture`. `SJK_TC_MAPS` (comma separated) overrides
//! the maps; the load test takes `SJK_TC` (0 or 1) and `SJK_TC_PICMIP`, one process each,
//! so its first build is cold.

use super::*;
use crate::graphics_quality::{Level, preset_cvars};
use crate::world_materials::filtering::mips::{self, compress};
use image::RgbaImage;
use std::time::Instant;

fn maps() -> Vec<String> {
    std::env::var("SJK_TC_MAPS")
        .unwrap_or_else(|_| "maps/mp/ffa3.bsp,maps/mp/siege_desert.bsp".to_owned())
        .split(',')
        .map(str::to_owned)
        .collect()
}

/// The SJK (recommended) preset, whose frames repeat from run to run, with `extra`.
fn cvars<'a>(extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut cvars = preset_cvars(Level::VanillaPlus);
    cvars.extend_from_slice(extra);
    cvars
}

fn stem(map: &str) -> String {
    map.trim_start_matches("maps/")
        .trim_end_matches(".bsp")
        .replace(['/', ':'], "-")
}

/// Mean squared error over `channels` of `a` and `b` (same size).
fn squared_error(a: &RgbaImage, b: &RgbaImage, channels: usize) -> (f64, u64) {
    let mut sum = 0.0;
    let mut count = 0;
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        for channel in 0..channels {
            let delta = f64::from(pa[channel]) - f64::from(pb[channel]);
            sum += delta * delta;
            count += 1;
        }
    }
    (sum, count)
}

fn psnr(sum: f64, count: u64) -> f64 {
    if sum == 0.0 {
        99.0
    } else {
        10.0 * (255.0_f64 * 255.0 / (sum / count as f64)).log10()
    }
}

/// PSNR, pixels that differ and the largest channel difference between two frames.
fn compare(a: &RgbaImage, b: &RgbaImage) -> (f64, usize, u8) {
    let (sum, count) = squared_error(a, b, 3);
    let differing = a.pixels().zip(b.pixels()).filter(|(x, y)| x != y).count();
    let largest = a
        .pixels()
        .zip(b.pixels())
        .flat_map(|(x, y)| (0..3).map(move |c| x[c].abs_diff(y[c])))
        .max()
        .unwrap_or(0);
    (psnr(sum, count), differing, largest)
}

/// `|a - b|` times eight, to see where frames differ.
fn difference(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
    RgbaImage::from_fn(a.width(), a.height(), |x, y| {
        let (p, q) = (a.get_pixel(x, y), b.get_pixel(x, y));
        let channel = |c: usize| p[c].abs_diff(q[c]).saturating_mul(8);
        image::Rgba([channel(0), channel(1), channel(2), 255])
    })
}

/// The middle `width` x `height` of `a` and `b`, side by side and doubled.
fn crops(a: &RgbaImage, b: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let (x, y) = ((a.width() - width) / 2, (a.height() - height) / 2);
    let mut out = RgbaImage::new(width * 4 + 8, height * 2);
    for (index, frame) in [a, b].into_iter().enumerate() {
        let crop = image::imageops::crop_imm(frame, x, y, width, height).to_image();
        let big = image::imageops::resize(
            &crop,
            width * 2,
            height * 2,
            image::imageops::FilterType::Nearest,
        );
        image::imageops::overlay(&mut out, &big, i64::from(index as u32 * (width * 2 + 8)), 0);
    }
    out
}

/// Frames of `map` from its start camera and the same camera looking down, with `extra`
/// cvars; with `capture`, also the textures the build prepared.
fn frames(
    map: &str,
    extra: &[(&str, &str)],
    capture: bool,
) -> Option<(Vec<RgbaImage>, Vec<(String, Vec<Arc<RgbaImage>>)>)> {
    if capture {
        *crate::world_materials::texture_prepare_capture()
            .lock()
            .expect("capture lock") = Some(Vec::new());
    }
    let (mut gpu, _profile) = open(map, [1920, 1080], None, &cvars(extra))?;
    let captured = crate::world_materials::texture_prepare_capture()
        .lock()
        .expect("capture lock")
        .take()
        .unwrap_or_default();
    let (origin, yaw) = (gpu.camera_position, gpu.camera_yaw.to_degrees());
    let mut shots = vec![frame(&mut gpu, 8)];
    aim(&mut gpu, origin.to_array(), yaw, -30.0);
    shots.push(frame(&mut gpu, 8));
    aim(&mut gpu, origin.to_array(), yaw + 90.0, -10.0);
    shots.push(frame(&mut gpu, 8));
    Some((shots, captured))
}

/// One texture's numbers.
struct Measured {
    key: String,
    size: (u32, u32),
    layers: usize,
    megapixels: f64,
    format: compress::Format,
    psnr: f64,
    milliseconds: f64,
    /// BC7 (alpha-capable "ultra fast" settings) on the same levels.
    bc7_psnr: f64,
    bc7_milliseconds: f64,
    /// A BC1 texel decoded translucent (never expected).
    lost_opacity: bool,
}

/// Level zero of every layer of `blocks` against `chains`.
fn level_psnr(chains: &[Vec<RgbaImage>], blocks: &compress::Blocks) -> f64 {
    let channels = if blocks.format == compress::Format::Bc1 {
        3
    } else {
        4
    };
    let (mut sum, mut count) = (0.0, 0);
    for (layer, chain) in chains.iter().enumerate() {
        let (s, c) = squared_error(&chain[0], &compress::decode(blocks, layer, 0), channels);
        sum += s;
        count += c;
    }
    psnr(sum, count)
}

fn measure(key: &str, images: &[Arc<RgbaImage>]) -> Option<Measured> {
    let (width, height) = mips::extent(images);
    if !compress::fits(width, height) {
        return None;
    }
    let chains: Vec<_> = images
        .iter()
        .map(|image| mips::chain(image, width, height))
        .collect();
    let started = Instant::now();
    let blocks = compress::encode(&chains, 0);
    let milliseconds = started.elapsed().as_secs_f64() * 1000.0;
    let lost_opacity = blocks.format == compress::Format::Bc1
        && (0..chains.len()).any(|layer| {
            (0..blocks.layers[layer].len()).any(|level| {
                compress::decode(&blocks, layer, level)
                    .pixels()
                    .any(|pixel| pixel[3] != 255)
            })
        });
    let started = Instant::now();
    let bc7 = compress::encode_as(&chains, 0, compress::Format::Bc7);
    let bc7_milliseconds = started.elapsed().as_secs_f64() * 1000.0;
    let megapixels = chains
        .iter()
        .flatten()
        .map(|level| f64::from(level.width() * level.height()))
        .sum::<f64>()
        / 1e6;
    Some(Measured {
        key: key.to_owned(),
        size: (width, height),
        layers: images.len(),
        megapixels,
        format: blocks.format,
        psnr: level_psnr(&chains, &blocks),
        milliseconds,
        bc7_psnr: level_psnr(&chains, &bc7),
        bc7_milliseconds,
        lost_opacity,
    })
}

/// Every texture of `captured`, measured on eight threads.
fn measure_all(captured: &[(String, Vec<Arc<RgbaImage>>)]) -> Vec<Measured> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                while let Some((key, images)) =
                    captured.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                {
                    if let Some(measured) = measure(key, images) {
                        results.lock().expect("results").push(measured);
                    }
                }
            });
        }
    });
    results.into_inner().expect("results")
}

/// The worst textures' level zero (its middle 192 x 192 at most) as RGBA8 and decoded from
/// its blocks, doubled side by side: `tc-<map>-worst-<n>.png`.
fn save_worst(stem: &str, captured: &[(String, Vec<Arc<RgbaImage>>)], worst: &[&Measured]) {
    for (index, measured) in worst.iter().enumerate() {
        let Some((_, images)) = captured.iter().find(|(key, _)| *key == measured.key) else {
            continue;
        };
        let (width, height) = mips::extent(images);
        let chains: Vec<_> = images
            .iter()
            .map(|image| mips::chain(image, width, height))
            .collect();
        let blocks = compress::encode(&chains, 0);
        let side = width.min(height).min(192);
        let _ = crops(&chains[0][0], &compress::decode(&blocks, 0, 0), side, side)
            .save(directory().join(format!("tc-{stem}-worst-{index}.png")));
    }
}

fn summarize(map: &str, captured: usize, mut measured: Vec<Measured>) {
    measured.sort_by(|a, b| a.psnr.total_cmp(&b.psnr));
    for (name, format) in [("BC1", compress::Format::Bc1), ("alpha", compress::ALPHA)] {
        let group: Vec<_> = measured.iter().filter(|m| m.format == format).collect();
        if group.is_empty() {
            continue;
        }
        let megapixels: f64 = group.iter().map(|m| m.megapixels).sum();
        let mean = group.iter().map(|m| m.psnr).sum::<f64>() / group.len() as f64;
        let bc7 = group.iter().map(|m| m.bc7_psnr).sum::<f64>() / group.len() as f64;
        let median = group[group.len() / 2].psnr;
        let low = group[group.len() / 20].psnr;
        eprintln!(
            "quality {map} {name:5} ({format:?}): {} textures, {megapixels:.1} MP with mips, \
             PSNR mean {mean:.2} dB, median {median:.2}, 5th percentile {low:.2}; \
             encode {:.0} ms ({:.1} ms/MP, one thread); BC7 instead: mean {bc7:.2} dB, \
             {:.0} ms ({:.1} ms/MP)",
            group.len(),
            group.iter().map(|m| m.milliseconds).sum::<f64>(),
            group.iter().map(|m| m.milliseconds).sum::<f64>() / megapixels,
            group.iter().map(|m| m.bc7_milliseconds).sum::<f64>(),
            group.iter().map(|m| m.bc7_milliseconds).sum::<f64>() / megapixels,
        );
    }
    eprintln!(
        "quality {map}: {captured} arrays prepared, {} compressible, {} with a BC1 texel \
         decoded translucent",
        measured.len(),
        measured.iter().filter(|m| m.lost_opacity).count()
    );
    for m in measured.iter().take(12) {
        eprintln!(
            "    worst: {:6.2} dB (BC7 {:6.2}) {:?} {}x{}x{} {}",
            m.psnr, m.bc7_psnr, m.format, m.size.0, m.size.1, m.layers, m.key
        );
    }
}

fn measured_worst(stem: &str, captured: &[(String, Vec<Arc<RgbaImage>>)], measured: &[Measured]) {
    let mut worst: Vec<&Measured> = measured.iter().collect();
    worst.sort_by(|a, b| a.psnr.total_cmp(&b.psnr));
    worst.truncate(4);
    save_worst(stem, captured, &worst);
}

/// Frames with compression off twice and on once, from the same cameras, compared and
/// saved (`tc-<map>-<view>-off|off2|on.png`, the difference times eight, and the middle of
/// the frame doubled, off left and on right); then every texture the map prepared, encoded
/// and measured.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn texture_compression_quality() {
    on_big_stack(|| {
        let _ = std::fs::create_dir_all(directory());
        for map in maps() {
            let off = [(compress::CVAR, "0")];
            let on = [(compress::CVAR, "1")];
            let Some((first, captured)) = frames(&map, &off, true) else {
                return;
            };
            let Some((second, _)) = frames(&map, &off, false) else {
                return;
            };
            let Some((compressed, _)) = frames(&map, &on, false) else {
                return;
            };
            let stem = stem(&map);
            for (view, ((a, b), c)) in first.iter().zip(&second).zip(&compressed).enumerate() {
                let noise = compare(a, b);
                let change = compare(a, c);
                eprintln!(
                    "frame {map} view {view}: off vs off {:.2} dB ({} pixels differ, largest {}), \
                     off vs on {:.2} dB ({} pixels differ, largest {})",
                    noise.0, noise.1, noise.2, change.0, change.1, change.2
                );
                let name =
                    |suffix: &str| directory().join(format!("tc-{stem}-{view}-{suffix}.png"));
                let _ = a.save(name("off"));
                let _ = b.save(name("off2"));
                let _ = c.save(name("on"));
                let _ = difference(a, c).save(name("diff"));
                let _ = crops(a, c, 480, 270).save(name("crop"));
            }
            let started = Instant::now();
            let measured = measure_all(&captured);
            eprintln!(
                "quality {map}: measured in {:.1} s",
                started.elapsed().as_secs_f64()
            );
            measured_worst(&stem, &captured, &measured);
            summarize(&map, captured.len(), measured);
        }
    });
}

/// World build times: each map built twice in this process (the first cold, the second
/// with the decoded images and the CPU mip cache warm) with `r_textureCompression` from
/// `SJK_TC` and `r_picmip` from `SJK_TC_PICMIP`; the texture preparation's own time is in
/// its log line.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn texture_compression_load() {
    on_big_stack(|| {
        let setting = std::env::var("SJK_TC").unwrap_or_else(|_| "0".to_owned());
        let picmip = std::env::var("SJK_TC_PICMIP").unwrap_or_else(|_| "0".to_owned());
        let extra = [
            (compress::CVAR, setting.as_str()),
            (mips::PICMIP_CVAR, picmip.as_str()),
        ];
        for map in maps() {
            for build in ["cold", "warm"] {
                let started = Instant::now();
                let Some((gpu, _profile)) = open(&map, [1920, 1080], None, &cvars(&extra)) else {
                    return;
                };
                let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                drop(gpu);
                eprintln!(
                    "load {map} r_textureCompression {setting} r_picmip {picmip} {build}: \
                     {elapsed:.0} ms"
                );
            }
        }
    });
}
