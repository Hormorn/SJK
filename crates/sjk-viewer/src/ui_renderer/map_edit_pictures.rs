//! The map editor's shader pictures (`docs/map-edits.md`, "The panel"): a small grid of
//! [`CELL`] thumbnails for the panel's shader rows and tiles, and one large preview of
//! the shader chosen or under the pointer at its own resolution with its mips.
//!
//! Both textures exist only while the panel shows pictures: they are made on the first
//! upload and dropped by [`MapEditPictures::release`] when the panel closes, so a player
//! who never opens the editor carries none of it (`docs/rendering.md`, small graphics
//! cards: the card faces' VRAM was a lesson).

use super::levelshot::LevelshotTexture;
use crate::menu::levelshot::LevelshotImage;
use sjk_ui::TextureId;

/// Edge of one thumbnail, as the icon atlas's cells.
pub(crate) const CELL: u32 = super::ICON_SIZE;
/// Thumbnails across the grid texture, and how many it holds (8 x 8 of 128: 4 MB).
const COLUMNS: u32 = 8;
pub(crate) const CELLS: u32 = COLUMNS * COLUMNS;
const SIZE: u32 = COLUMNS * CELL;
/// The texture ids: thumbnails `TEXTURE_BASE..TEXTURE_BASE + CELLS`, then the preview.
const TEXTURE_BASE: u32 = 0x6200_0000;
/// `TexturedQuad` texture naming the large preview.
pub(crate) const PREVIEW: TextureId = TextureId(TEXTURE_BASE + 0x100);

/// `TexturedQuad` texture naming thumbnail `cell`.
pub(crate) fn cell_texture(cell: u32) -> TextureId {
    TextureId(TEXTURE_BASE + cell.min(CELLS - 1))
}

/// The thumbnail `texture` names.
pub(crate) fn cell_of(texture: TextureId) -> Option<u32> {
    (TEXTURE_BASE..TEXTURE_BASE + CELLS)
        .contains(&texture.0)
        .then(|| texture.0 - TEXTURE_BASE)
}

/// The texture coordinates of thumbnail `cell` (top-left and bottom-right).
pub(super) fn uv_range(cell: u32) -> ([f32; 2], [f32; 2]) {
    let cell = cell.min(CELLS - 1);
    let (x, y) = ((cell % COLUMNS) * CELL, (cell / COLUMNS) * CELL);
    let uv = |x: u32, y: u32| [x as f32 / SIZE as f32, y as f32 / SIZE as f32];
    (uv(x, y), uv(x + CELL, y + CELL))
}

/// The grid and the preview, each made when first needed.
#[derive(Default)]
pub(super) struct MapEditPictures {
    grid: Option<(wgpu::Texture, wgpu::BindGroup)>,
    preview: Option<LevelshotTexture>,
}

impl MapEditPictures {
    pub(super) fn grid_group(&self) -> Option<&wgpu::BindGroup> {
        self.grid.as_ref().map(|(_, group)| group)
    }

    pub(super) fn preview_group(&self) -> Option<&wgpu::BindGroup> {
        self.preview.as_ref().map(LevelshotTexture::bind_group)
    }

    /// Put `rgba` ([`CELL`] square) in thumbnail `cell`, making the grid first.
    pub(super) fn upload_cell(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        layout: &wgpu::BindGroupLayout,
        cell: u32,
        rgba: &[u8],
    ) {
        if rgba.len() != (CELL * CELL * 4) as usize || cell >= CELLS {
            return;
        }
        let (texture, _) = self.grid.get_or_insert_with(|| grid(device, layout));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: (cell % COLUMNS) * CELL,
                    y: (cell / COLUMNS) * CELL,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(CELL * 4),
                rows_per_image: Some(CELL),
            },
            wgpu::Extent3d {
                width: CELL,
                height: CELL,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Show `image` as the large preview.
    pub(super) fn upload_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        layout: &wgpu::BindGroupLayout,
        image: &LevelshotImage,
    ) {
        self.preview
            .get_or_insert_with(|| LevelshotTexture::new(device, layout))
            .upload(device, queue, layout, image);
    }

    /// Drop both textures (the panel closed).
    pub(super) fn release(&mut self) {
        self.grid = None;
        self.preview = None;
    }

    /// Whether either texture is held.
    pub(super) fn held(&self) -> bool {
        self.grid.is_some() || self.preview.is_some()
    }
}

fn grid(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SJK map editor shader pictures"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::ui_target::TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("SJK map editor shader pictures sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("SJK map editor shader pictures bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    (texture, group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_name_their_cells_and_stay_apart_from_other_textures() {
        assert_eq!(cell_of(cell_texture(5)), Some(5));
        assert_eq!(cell_of(cell_texture(CELLS + 3)), Some(CELLS - 1));
        assert_eq!(cell_of(PREVIEW), None);
        assert_eq!(cell_of(TextureId(TEXTURE_BASE - 1)), None);
        assert!(crate::cards::faces::index_of(cell_texture(0)).is_none());
        assert!(crate::chat_gifs::slot_of(cell_texture(0)).is_none());
        let (a, b) = uv_range(9);
        assert_eq!(a, [0.125, 0.125]);
        assert_eq!(b, [0.25, 0.25]);
    }
}
