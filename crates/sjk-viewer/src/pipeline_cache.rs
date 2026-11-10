//! The pipeline cache kept on disk (`docs/rendering.md`, "Pipeline cache on disk"): what the
//! driver compiled for the blade and world pipelines, saved between runs, so a pipeline built
//! once is not compiled from nothing the next time (Sol, 10/11/2026: "store the shaders
//! locally in a cache once they loaded them so they load faster next time").
//!
//! Only Vulkan has one (wgpu's `PIPELINE_CACHE`); DX12, GL and Metal rely on the driver's own
//! cache, as before. One file per graphics card ([`file_name`]) in the `cache/` folder beside
//! the settings, loaded at start with `fallback`: a file from another driver or another wgpu,
//! or a damaged one, is dropped by wgpu and the cache starts empty. It is saved, written to a
//! temporary file then renamed over the old one, 10 s after the last new pipeline and at a
//! clean exit, by a thread of its own.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The folder beside `config.cfg` the cache files go in.
pub(crate) const FOLDER: &str = "cache";
/// A cache file larger than this is neither read nor written (a few MiB is usual).
pub(crate) const MAX_BYTES: usize = 64 << 20;
/// How long after the last new pipeline the cache is saved: a map load or a round of blade
/// builds makes many in a row, saved once.
const SETTLE: Duration = Duration::from_secs(10);

/// The device feature the cache needs, where `adapter` offers it (Vulkan).
pub(crate) fn feature(adapter: &wgpu::Adapter) -> wgpu::Features {
    adapter.features() & wgpu::Features::PIPELINE_CACHE
}

/// The cache file's name for the card `info` describes: wgpu's key (backend, vendor and
/// device; the driver version is checked inside the file), `None` where wgpu has no cache.
pub(crate) fn file_name(info: &wgpu::AdapterInfo) -> Option<String> {
    wgpu::util::pipeline_cache_key(info).map(|key| format!("{key}.bin"))
}

/// Why a cache starts empty, or what it was loaded from.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Read {
    Loaded(Vec<u8>),
    Missing,
    TooLarge(u64),
    Unreadable(String),
}

/// The bytes of the cache file `path`, unless it is missing, too large or unreadable.
pub(crate) fn read(path: &Path) -> Read {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Read::Missing,
        Err(error) => return Read::Unreadable(error.to_string()),
        Ok(meta) if meta.len() > MAX_BYTES as u64 => return Read::TooLarge(meta.len()),
        Ok(_) => {}
    }
    match std::fs::read(path) {
        Ok(bytes) => Read::Loaded(bytes),
        Err(error) => Read::Unreadable(error.to_string()),
    }
}

/// Write `data` to `path` whole or not at all: into a temporary file beside it, then renamed
/// over it, so a crash or a second client mid-save never leaves half a file.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let temporary = path.with_extension(format!("tmp{}", std::process::id()));
    let written = (|| {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()
    })();
    let renamed = written.and_then(|()| std::fs::rename(&temporary, path));
    if renamed.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    renamed
}

/// One device's pipeline cache and the file it is saved to.
pub(crate) struct Disk {
    device: wgpu::Device,
    cache: wgpu::PipelineCache,
    path: PathBuf,
    opened: Instant,
    /// When the last pipeline was made with the cache since it was saved: milliseconds after
    /// `opened`, plus one; 0 when nothing is waiting to be saved.
    touched: AtomicU64,
    /// The hash of the data saved last, held while saving so two saves never overlap.
    saved: Mutex<Option<u64>>,
}

impl Disk {
    /// `device`'s cache from the file `path` (empty when it is missing or wgpu drops it), and
    /// the log line saying so; `None` when the device was opened without the feature.
    pub(crate) fn open(device: &wgpu::Device, path: PathBuf) -> Option<(Self, String)> {
        if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
            return None;
        }
        let read = read(&path);
        let data = match &read {
            Read::Loaded(bytes) => Some(bytes.as_slice()),
            _ => None,
        };
        // Only ever what `get_data` wrote under this card's key; wgpu checks its header and
        // drops what does not match (`sjk_pipeline_cache::create`).
        let cache = sjk_pipeline_cache::create(device, "SJK pipeline cache", data);
        let kept = cache.get_data().map_or(0, |data| data.len());
        let line = match read {
            Read::Loaded(bytes) => format!(
                "pipeline cache: loaded {} bytes from {} ({kept} bytes kept by the driver)",
                bytes.len(),
                path.display()
            ),
            Read::Missing => format!("pipeline cache: created empty ({} is new)", path.display()),
            Read::TooLarge(size) => format!(
                "pipeline cache: created empty ({} is {size} bytes, over {MAX_BYTES})",
                path.display()
            ),
            Read::Unreadable(why) => format!(
                "pipeline cache: created empty ({} unreadable: {why})",
                path.display()
            ),
        };
        let disk = Self {
            device: device.clone(),
            cache,
            path,
            opened: Instant::now(),
            touched: AtomicU64::new(0),
            saved: Mutex::new(None),
        };
        Some((disk, line))
    }

    /// Note a pipeline made with the cache, for the next save.
    fn touch(&self) {
        let now = u64::try_from(self.opened.elapsed().as_millis()).unwrap_or(u64::MAX - 1);
        self.touched.store(now + 1, Ordering::Release);
    }

    /// Whether a pipeline was made since the last save, longer ago than `settle`.
    fn settled(&self, settle: Duration) -> Option<u64> {
        let touched = self.touched.load(Ordering::Acquire);
        let since = self.opened.elapsed().as_millis() as u64;
        (touched != 0 && since + 1 >= touched + settle.as_millis() as u64).then_some(touched)
    }

    /// Save the cache's data now, unless it is what was saved last or too large; the log
    /// line saying what was done, if anything.
    pub(crate) fn save(&self) -> Option<String> {
        let mut saved = self
            .saved
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.touched.store(0, Ordering::Release);
        let data = self.cache.get_data()?;
        if data.len() > MAX_BYTES {
            return Some(format!(
                "pipeline cache: not saved, {} bytes is over {MAX_BYTES}",
                data.len()
            ));
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        if *saved == Some(hash) {
            return None;
        }
        Some(match write_atomic(&self.path, &data) {
            Ok(()) => {
                *saved = Some(hash);
                format!(
                    "pipeline cache: saved {} bytes to {}",
                    data.len(),
                    self.path.display()
                )
            }
            Err(error) => format!(
                "pipeline cache: could not save {}: {error}",
                self.path.display()
            ),
        })
    }
}

/// The client's cache, once [`open_for_client`] made it.
static DISK: OnceLock<Disk> = OnceLock::new();

/// Open the client's pipeline cache for `device` on the card `info` describes, in `folder`
/// (the settings' `cache/`), log what was found, and start the thread that saves it. Once per
/// process; off-screen renders (tests, world shots) pass no folder and get none.
pub(crate) fn open_for_client(device: &wgpu::Device, info: &wgpu::AdapterInfo, folder: &Path) {
    let Some(name) = file_name(info) else {
        crate::log::progress(format_args!(
            "pipeline cache: unsupported on {:?}, the driver's own cache only",
            info.backend
        ));
        return;
    };
    let Some((disk, line)) = Disk::open(device, folder.join(name)) else {
        crate::log::progress(format_args!(
            "pipeline cache: unsupported, {} offers none on {:?}",
            info.name, info.backend
        ));
        return;
    };
    crate::log::progress(format_args!("{line}"));
    if DISK.set(disk).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("sjk-pipeline-cache".to_owned())
        .spawn(|| {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                if let Some(disk) = DISK.get()
                    && disk.settled(SETTLE).is_some()
                    && let Some(line) = disk.save()
                {
                    crate::log::progress(format_args!("{line}"));
                }
            }
        });
    if let Err(why) = spawned {
        crate::log::progress(format_args!("pipeline cache: no saving thread: {why}"));
    }
}

/// The pipeline cache to make a pipeline on `device` with: the client's, when it has one on
/// that device (noted for the next save), else none.
pub(crate) fn of(device: &wgpu::Device) -> Option<wgpu::PipelineCache> {
    let disk = DISK.get().filter(|disk| disk.device == *device)?;
    disk.touch();
    Some(disk.cache.clone())
}

/// Save the client's cache if pipelines were made since it was last saved (a clean exit).
pub(crate) fn save_at_exit() {
    if let Some(disk) = DISK.get()
        && disk.touched.load(Ordering::Acquire) != 0
        && let Some(line) = disk.save()
    {
        crate::log::progress(format_args!("{line}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("sjk-pipeline-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn the_file_is_named_by_card_on_vulkan_only() {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Vulkan);
        info.vendor = 0x10de;
        info.device = 0x2c02;
        assert_eq!(
            file_name(&info).as_deref(),
            Some("wgpu_pipeline_cache_vulkan_4318_11266.bin")
        );
        for backend in [wgpu::Backend::Dx12, wgpu::Backend::Gl, wgpu::Backend::Metal] {
            info.backend = backend;
            assert_eq!(file_name(&info), None, "{backend:?}");
        }
    }

    #[test]
    fn a_missing_or_oversized_file_starts_empty() {
        let folder = folder("read");
        let path = folder.join("cache.bin");
        assert_eq!(read(&path), Read::Missing);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&path, b"not a cache").unwrap();
        // Damaged bytes are handed over: wgpu's fallback drops them (the GPU test below).
        assert_eq!(read(&path), Read::Loaded(b"not a cache".to_vec()));
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_BYTES as u64 + 1).unwrap();
        drop(file);
        assert_eq!(read(&path), Read::TooLarge(MAX_BYTES as u64 + 1));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_save_replaces_the_file_whole_and_leaves_no_temporary() {
        let folder = folder("save");
        let path = folder.join("deep").join("cache.bin");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second, longer").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second, longer");
        let left: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, [std::ffi::OsString::from("cache.bin")]);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    #[ignore = "requires an available GPU; uses no installed game assets"]
    fn a_damaged_file_gives_an_empty_cache_that_still_builds_and_saves() {
        let gpu = pollster::block_on(crate::gpu_context::Context::new(None, None)).unwrap();
        let folder = folder("gpu");
        let path = folder.join("cache.bin");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&path, vec![0x5a; 4096]).unwrap();
        let Some((disk, line)) = Disk::open(&gpu.device, path.clone()) else {
            eprintln!("{}: no pipeline cache on this backend", gpu.adapter_backend);
            return;
        };
        assert!(
            line.starts_with("pipeline cache: loaded 4096 bytes"),
            "{line}"
        );
        let module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(
                    "@vertex fn v() -> @builtin(position) vec4f { return vec4f(0.0); }
                     @fragment fn f() -> @location(0) vec4f { return vec4f(1.0); }"
                        .into(),
                ),
            });
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("v"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("f"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: Some(&disk.cache),
            });
        assert!(pollster::block_on(scope.pop()).is_none());
        disk.touch();
        let saved = disk.save().unwrap();
        assert!(saved.starts_with("pipeline cache: saved"), "{saved}");
        assert_ne!(std::fs::read(&path).unwrap(), vec![0x5a; 4096]);
        // Nothing new: the same data is not written again.
        assert_eq!(disk.save(), None);
        let (_, again) = Disk::open(&gpu.device, path).unwrap();
        assert!(again.starts_with("pipeline cache: loaded"), "{again}");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
