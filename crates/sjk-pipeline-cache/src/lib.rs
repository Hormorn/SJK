//! A wgpu pipeline cache made from the bytes an earlier run saved, behind a safe API
//! (`docs/rendering.md`, "Pipeline cache on disk"; the client's side is
//! `crates/sjk-viewer/src/pipeline_cache.rs`).
//!
//! This crate is allowed `unsafe` (as sjk-mediafoundation and sjk-gpu-memory are) for one
//! call: wgpu's `Device::create_pipeline_cache` is unsafe because the bytes handed to it must
//! come from `PipelineCache::get_data` of a compatible adapter. [`create`] always asks wgpu to
//! check them and fall back to an empty cache.

/// A pipeline cache on `device` (which must have `Features::PIPELINE_CACHE`), started from
/// `data` when wgpu accepts it, else empty. `data` must be what `PipelineCache::get_data`
/// returned in an earlier run, read back unchanged as far as the caller knows (a file the
/// client wrote, under a name from `wgpu::util::pipeline_cache_key`).
pub fn create(device: &wgpu::Device, label: &str, data: Option<&[u8]>) -> wgpu::PipelineCache {
    // SAFETY: wgpu requires `data` to come from `get_data` of a compatible adapter; the
    // caller passes only what an earlier run's `get_data` wrote to a file named by the card's
    // `pipeline_cache_key`, written whole (temporary file, then a rename). With `fallback`,
    // wgpu checks the header it wrote (magic, header version and ABI, backend, vendor and
    // device, the driver's pipeline cache UUID, the body's exact length) and starts empty when
    // anything differs, so data from another card, driver or wgpu, or a cut or grown file,
    // never reaches the driver; the body itself is the driver's to check (Vulkan's own cache
    // header). wgpu's docs accept that the data's origin on disk cannot be proven further.
    unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some(label),
            data,
            fallback: true,
        })
    }
}
