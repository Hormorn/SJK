//! Optional load-time mip chains. Level zero uses the existing resize policy unchanged.
//!
//! `r_picmip` (as in Jedi Academy) leaves a texture's largest levels off the card:
//! each step halves its sides and takes a quarter of its video memory, for cards
//! that run out (a 3 GB card ran out on most JoF maps, 21/10/2026). Textures of
//! [`PICMIP_FLOOR`] pixels or less keep their size, and so do the stages of a shader
//! that says `nopicmip` or `nomipmaps` ([`NO_PICMIP_KEY`]). The chains are still built
//! whole on the processor; only the upload skips levels. Textures uploaded without
//! mips (sky boxes, the non-mipmapped filtering) and material maps take the same
//! size ([`skipped`]). A change applies to the textures uploaded after it, so from the
//! next map. With `r_textureCompression` the kept levels go up block-compressed
//! ([`compress`], [`Levels`]).
//!
//! `r_maxTextureSize` (SJK, 2048 by default, 0 for none) also leaves off the levels
//! larger than its size, whatever `r_picmip`: a community model's 4096x4096 textures take
//! four times the video memory of 2048 ones and a frame 90 MB to upload with their mips,
//! for a player a few hundred pixels tall (08/11/2026).
use image::{RgbaImage, imageops};
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

#[path = "texture_mip_cache.rs"]
pub(crate) mod cache;
#[path = "texture_compression.rs"]
pub(crate) mod compress;

/// The texture detail setting: the levels left off.
pub(crate) const PICMIP_CVAR: &str = "r_picmip";
/// The largest `r_picmip` taken.
pub(crate) const PICMIP_MAX: u32 = 3;
/// Textures whose longest side is this or less are never shrunk.
pub(crate) const PICMIP_FLOOR: u32 = 128;
/// `r_picmip` as last set ([`set_picmip`]).
static PICMIP: AtomicU32 = AtomicU32::new(0);
/// The largest texture side setting.
pub(crate) const MAX_SIZE_CVAR: &str = "r_maxTextureSize";
/// `r_maxTextureSize`'s default.
pub(crate) const MAX_SIZE_DEFAULT: u32 = 2048;
/// `r_maxTextureSize` as last set ([`set_max_size`]), 0 for no cap.
static MAX_SIZE: AtomicU32 = AtomicU32::new(MAX_SIZE_DEFAULT);

/// Read `r_picmip` when a graphics context is made, with the filtering
/// ([`super::Policy::sample`]).
pub(crate) fn sample_picmip(console: Option<&crate::console::ViewerConsole>) {
    set_picmip(
        console
            .and_then(|console| console.integer_cvar(PICMIP_CVAR))
            .unwrap_or(0),
    );
    set_max_size(
        console
            .and_then(|console| console.integer_cvar(MAX_SIZE_CVAR))
            .unwrap_or(i64::from(MAX_SIZE_DEFAULT)),
    );
}

/// Cap the textures uploaded from now on at `size` pixels a side (0 for none; less than
/// [`PICMIP_FLOOR`] is the floor), from the next map as [`set_picmip`].
pub(crate) fn set_max_size(size: i64) {
    let size = if size <= 0 {
        0
    } else {
        size.clamp(i64::from(PICMIP_FLOOR), i64::from(u32::MAX)) as u32
    };
    if MAX_SIZE.swap(size, Ordering::Relaxed) != size {
        crate::log::progress(format_args!(
            "texture size cap: {MAX_SIZE_CVAR} {size}, from the next map"
        ));
    }
}

/// Take `levels` of `r_picmip` for the textures uploaded from now on: a world being
/// built when it changes may mix the two sizes, the next map has the new one.
pub(crate) fn set_picmip(levels: i64) {
    let levels = levels.clamp(0, i64::from(PICMIP_MAX)) as u32;
    if PICMIP.swap(levels, Ordering::Relaxed) != levels {
        crate::log::progress(format_args!(
            "texture detail: r_picmip {levels}, from the next map"
        ));
    }
}

/// `r_picmip` as last set.
pub(crate) fn picmip() -> u32 {
    PICMIP.load(Ordering::Relaxed)
}

/// Ends the texture key of a stage whose shader says `nopicmip` or `nomipmaps`
/// (`ShaderStage::no_picmip`): its images keep their size ([`picmip_for`]), and never
/// share an upload with the same images at the texture detail.
pub(crate) const NO_PICMIP_KEY: &str = "$nopicmip;";

/// The `r_picmip` the texture under `key` takes: none when the key ends with
/// [`NO_PICMIP_KEY`], else the setting.
pub(crate) fn picmip_for(key: &str) -> u32 {
    if key.ends_with(NO_PICMIP_KEY) {
        0
    } else {
        picmip()
    }
}

/// `image` with the levels `r_picmip` leaves off ([`skipped`]) taken away, for a
/// texture uploaded whole rather than as a chain; `None` when it keeps its size.
pub(crate) fn shrunk(image: &RgbaImage) -> Option<RgbaImage> {
    let (width, height) = image.dimensions();
    let skip = skipped(width, height, picmip());
    (skip > 0).then(|| {
        imageops::resize(
            image,
            (width >> skip).max(1),
            (height >> skip).max(1),
            imageops::FilterType::Triangle,
        )
    })
}

/// The levels of a `width` x `height` texture left off the card at `picmip`.
pub(crate) fn skipped(width: u32, height: u32, picmip: u32) -> u32 {
    skipped_capped(width, height, picmip, MAX_SIZE.load(Ordering::Relaxed))
}

/// [`skipped`] with `max_size` for `r_maxTextureSize`: at least the levels that bring
/// the longest side to `max_size` or less (none for 0).
fn skipped_capped(width: u32, height: u32, picmip: u32, max_size: u32) -> u32 {
    let longest = width.max(height);
    let detail = (0..picmip.min(PICMIP_MAX))
        .take_while(|skip| longest >> (skip + 1) >= PICMIP_FLOOR)
        .count() as u32;
    let cap = if max_size == 0 {
        0
    } else {
        (0..31)
            .take_while(|&skip| longest >> skip > max_size)
            .count() as u32
    };
    detail.max(cap)
}

/// Number of complete mip levels through 1x1, including rectangular/NPOT images.
pub(crate) fn levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// One layer's mip chain, level zero first: `image` fitted to `width` x `height`, then
/// halved down to one texel. Pure processor work, so a loader may build many at once.
pub(crate) fn chain(image: &RgbaImage, width: u32, height: u32) -> Vec<RgbaImage> {
    let mut chain = Vec::with_capacity(levels(width, height) as usize);
    chain.push(if image.width() == width && image.height() == height {
        image.clone()
    } else {
        imageops::resize(image, width, height, imageops::FilterType::Lanczos3)
    });
    for _ in 1..levels(width, height) {
        let above = chain.last().expect("level zero is there");
        chain.push(imageops::resize(
            above,
            (above.width() / 2).max(1),
            (above.height() / 2).max(1),
            imageops::FilterType::Triangle,
        ));
    }
    chain
}

/// The extent of the array `images` become layers of.
pub(crate) fn extent(images: &[Arc<RgbaImage>]) -> (u32, u32) {
    (
        images.iter().map(|v| v.width()).max().unwrap_or(1),
        images.iter().map(|v| v.height()).max().unwrap_or(1),
    )
}

/// One texture's prepared levels, one array layer per image.
#[derive(Clone)]
pub(crate) enum Levels {
    /// Whole RGBA8 [`chain`]s; the upload leaves off the levels `r_picmip` skips.
    Rgba(Arc<Vec<Vec<RgbaImage>>>),
    /// Block-compressed levels, those `r_picmip` skips already gone.
    Blocks(Arc<compress::Blocks>),
}

/// What the worlds of one context share a map texture as ([`crate::shared_textures`]):
/// the view, and the images it was made from, held so their addresses (the key) cannot
/// name other pixels while it lives.
type Shared = (wgpu::TextureView, Arc<[Arc<RgbaImage>]>);
const SHARED: &str = "map texture";

/// The key of the `width` x `height` array `images` become, `skip` levels off. Decoded
/// map images are one `Arc` per mounted file and content for the process
/// ([`crate::decoded_image_cache`]), so two maps using the same file give one key; a
/// world built after `r_picmip` changed (or a `nopicmip` stage) gets its own size, and
/// one built after `r_textureCompression` changed its own format.
fn shared_key(images: &[Arc<RgbaImage>], width: u32, height: u32, skip: u32) -> u64 {
    let mut parts = vec![
        width.to_le_bytes().to_vec(),
        height.to_le_bytes().to_vec(),
        skip.to_le_bytes().to_vec(),
    ];
    parts.extend(
        images
            .iter()
            .map(|image| (Arc::as_ptr(image) as usize as u64).to_le_bytes().to_vec()),
    );
    if compress::applies(width >> skip, height >> skip) {
        parts.push(b"bc".to_vec());
    }
    crate::shared_textures::key(&parts.iter().map(Vec::as_slice).collect::<Vec<_>>())
}

/// The mipmapped array of `images` at `picmip` another world of this context holds,
/// if any.
pub(crate) fn find_shared(images: &[Arc<RgbaImage>], picmip: u32) -> Option<wgpu::TextureView> {
    let (width, height) = extent(images);
    let key = shared_key(images, width, height, skipped(width, height, picmip));
    crate::shared_textures::find::<Shared>(SHARED, key).map(|(view, _)| view)
}

/// [`upload_levels`] of `images`, or another world's copy of the same array.
pub(crate) fn upload_shared(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    images: &[Arc<RgbaImage>],
    width: u32,
    height: u32,
    picmip: u32,
    levels: impl FnOnce() -> Levels,
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    let key = shared_key(images, width, height, skipped(width, height, picmip));
    if let Some((view, _)) = crate::shared_textures::find::<Shared>(SHARED, key) {
        return Ok(view);
    }
    let view = upload_levels(device, queue, width, height, &levels(), picmip)?;
    let shared: Shared = (view.clone(), images.into());
    crate::shared_textures::share(SHARED, key, || shared);
    Ok(view)
}

/// Upload mipmapped stage frames once, less the levels `picmip` leaves off; no
/// frame-path image generation or readback. Outside a world's build (a model's material
/// on the render thread that no worker prepared) the texture stays RGBA8: blocks are
/// encoded only off the render thread.
pub(crate) fn upload(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    images: &[Arc<RgbaImage>],
    picmip: u32,
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    let (width, height) = extent(images);
    upload_shared(device, queue, images, width, height, picmip, || {
        if crate::shared_textures::in_build() {
            cache::prepare_levels(images, width, height, picmip)
        } else {
            Levels::Rgba(cache::prepare(images, width, height))
        }
    })
}

/// Upload prepared `levels` of a `width` x `height` array: RGBA8 chains less the levels
/// `picmip` leaves off ([`upload_chains`]), or blocks as they are ([`compress::upload`]).
pub(crate) fn upload_levels(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    width: u32,
    height: u32,
    levels: &Levels,
    picmip: u32,
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    match levels {
        Levels::Rgba(chains) => upload_chains(device, queue, width, height, chains, picmip),
        Levels::Blocks(blocks) => compress::upload(device, queue, blocks),
    }
}

/// Upload one [`chain`] per layer of a `width` x `height` array, less the levels
/// `picmip` leaves off ([`skipped`]).
pub(crate) fn upload_chains(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    width: u32,
    height: u32,
    chains: &[Vec<RgbaImage>],
    picmip: u32,
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    let shortest = chains.iter().map(Vec::len).min().unwrap_or(0);
    let skip = skipped(width, height, picmip).min(u32::try_from(shortest.saturating_sub(1))?);
    let (width, height) = ((width >> skip).max(1), (height >> skip).max(1));
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SJK optional mipmapped material"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: chains.len().max(1).try_into()?,
        },
        mip_level_count: levels(width, height),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, chain) in chains.iter().enumerate() {
        for (level, pixels) in chain.iter().skip(skip as usize).enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level.try_into()?,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer.try_into()?,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                pixels.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pixels.width() * 4),
                    rows_per_image: Some(pixels.height()),
                },
                wgpu::Extent3d {
                    width: pixels.width(),
                    height: pixels.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    Ok(texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picmip_halves_large_textures_down_to_the_floor_only() {
        assert_eq!(skipped(1024, 512, 0), 0);
        assert_eq!(skipped(1024, 512, 1), 1);
        assert_eq!(skipped(1024, 512, 3), 3);
        // 256 halves once to the floor, 128 and less keep their size.
        assert_eq!(skipped(256, 256, 3), 1);
        assert_eq!(skipped(128, 64, 3), 0);
        assert_eq!(skipped(4096, 4096, 9), PICMIP_MAX);
    }

    #[test]
    fn the_size_cap_halves_textures_past_it_whatever_the_detail() {
        assert_eq!(skipped_capped(4096, 4096, 0, 2048), 1);
        assert_eq!(skipped_capped(4096, 1024, 0, 1024), 2);
        assert_eq!(skipped_capped(8192, 8192, 0, 2048), 2);
        // At the cap or under it, only r_picmip counts.
        assert_eq!(skipped_capped(2048, 2048, 0, 2048), 0);
        assert_eq!(skipped_capped(2048, 2048, 2, 2048), 2);
        assert_eq!(skipped_capped(4096, 4096, 2, 2048), 2);
        // A side not a power of two still ends at the cap or less.
        assert_eq!(skipped_capped(3000, 1500, 0, 2048), 1);
        // 0 is no cap.
        assert_eq!(skipped_capped(16384, 16384, 0, 0), 0);
    }

    #[test]
    fn nopicmip_keys_take_no_texture_detail() {
        let key = format!("textures/a/b.tga;{NO_PICMIP_KEY}");
        assert_eq!(picmip_for(&key), 0);
        assert_eq!(picmip_for("textures/a/b.tga;"), picmip());
    }

    #[test]
    fn a_shrunk_chain_keeps_its_smaller_levels_whole() {
        // A 512 x 256 texture at r_picmip 2 is its 128 x 64 level and the levels under.
        let (width, height) = (512_u32, 256_u32);
        let skip = skipped(width, height, 2);
        assert_eq!(skip, 2);
        let image = RgbaImage::new(width, height);
        let chain = chain(&image, width, height);
        let kept = &chain[skip as usize..];
        assert_eq!((kept[0].width(), kept[0].height()), (128, 64));
        assert_eq!(kept.len() as u32, levels(width >> skip, height >> skip));
    }
}
