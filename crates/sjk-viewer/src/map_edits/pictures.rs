//! The in-game panel's shader pictures (Sol, 09/11/2026: "be able to see textures better
//! too"): a thumbnail beside each shader row and on each tile, and a large preview of the
//! shader chosen or under the pointer beside the selection's own.
//!
//! Each picture is the image the game draws the shader with: the image its script selects
//! for the world renderer (`ShaderCatalog::resolve_image`, as the HUD icons are found), a
//! sky's box front (`<box>_ft`) as the browser editor's sky list shows it, decoded through
//! the world's own decoded-image cache. Only the rows and tiles drawn ask for theirs
//! ([`Pictures::thumbnail`] while the panel is built); a few are decoded a frame
//! ([`PER_FRAME`]), the least lately drawn giving up its cell when the grid is full. The
//! textures ([`crate::ui_renderer::map_edit_pictures`]) are made on the first picture and
//! dropped when the panel closes.

use crate::GpuState;
use crate::ui_renderer::map_edit_pictures::{CELL, CELLS, PREVIEW, cell_texture};
use sjk_ui::TextureId;

/// Pictures decoded in one frame at most (the preview counts as one).
const PER_FRAME: usize = 2;
/// The large preview's longest edge.
pub(crate) const PREVIEW_EDGE: u32 = 512;

/// What the panel's pictures hold and want.
#[derive(Default)]
pub(crate) struct Pictures {
    /// Each thumbnail cell's shader (lower case) and the build it was last drawn in.
    cells: Vec<(String, u64)>,
    /// Shaders found to have no picture (lower case).
    missing: Vec<String>,
    /// Thumbnails asked for and not held: each shader and the build that asked.
    queue: Vec<(String, u64)>,
    /// The panel's builds so far.
    build: u64,
    /// The preview wanted (lower case) and the build that asked; the one shown, with its
    /// picture's size (none: it has no picture).
    preview_wanted: Option<(String, u64)>,
    preview_shown: Option<(String, Option<[u32; 2]>)>,
    /// The textures exist.
    held: bool,
}

impl Pictures {
    /// A new build of the panel: what it draws asks for its pictures again.
    pub(crate) fn begin(&mut self) {
        self.build += 1;
    }

    /// The thumbnail of `shader`, if held; asked for otherwise (the next frames decode
    /// it). `Some(None)`: it has no picture.
    pub(crate) fn thumbnail(&mut self, shader: &str) -> Option<Option<TextureId>> {
        let build = self.build;
        if let Some(cell) = self
            .cells
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(shader))
        {
            self.cells[cell].1 = build;
            return Some(Some(cell_texture(cell as u32)));
        }
        if self
            .missing
            .iter()
            .any(|name| name.eq_ignore_ascii_case(shader))
        {
            return Some(None);
        }
        match self
            .queue
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case(shader))
        {
            Some(asked) => asked.1 = build,
            None => self.queue.push((shader.to_ascii_lowercase(), build)),
        }
        None
    }

    /// The large preview of `shader` and its picture's size, if shown; asked for
    /// otherwise. `Some(None)`: it has no picture.
    pub(crate) fn preview(&mut self, shader: &str) -> Option<Option<(TextureId, [u32; 2])>> {
        let build = self.build;
        if let Some((shown, size)) = &self.preview_shown
            && shown.eq_ignore_ascii_case(shader)
        {
            return Some(size.map(|size| (PREVIEW, size)));
        }
        match &mut self.preview_wanted {
            Some((wanted, asked)) if wanted.eq_ignore_ascii_case(shader) => *asked = build,
            slot => *slot = Some((shader.to_ascii_lowercase(), build)),
        }
        None
    }

    /// The next thumbnail to decode: asked for by the last build (older asks, rows
    /// scrolled away, are dropped).
    fn next_thumbnail(&mut self) -> Option<String> {
        let build = self.build;
        self.queue.retain(|(_, asked)| *asked == build);
        (!self.queue.is_empty()).then(|| self.queue.remove(0).0)
    }

    /// The preview to decode, when the last build asked for one not shown.
    fn next_preview(&mut self) -> Option<String> {
        let build = self.build;
        match self.preview_wanted.take() {
            Some((shader, asked)) if asked == build => Some(shader),
            _ => None,
        }
    }

    /// The cell `shader`'s thumbnail goes in: a free one, else the least lately drawn of
    /// those the last build did not draw. None when every cell is on show.
    fn cell_for(&mut self, shader: String) -> Option<u32> {
        if self.cells.len() < CELLS as usize {
            self.cells.push((shader, self.build));
            return Some(self.cells.len() as u32 - 1);
        }
        let build = self.build;
        let (cell, _) = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, (_, drawn))| *drawn < build)
            .min_by_key(|(_, (_, drawn))| *drawn)?;
        self.cells[cell] = (shader, build);
        Some(cell as u32)
    }

    fn mark_missing(&mut self, shader: String) {
        if self.missing.len() >= 4096 {
            self.missing.clear();
        }
        self.missing.push(shader);
    }

    /// Forget every picture (the textures are gone, or the game data changed).
    fn forget(&mut self) {
        self.cells.clear();
        self.missing.clear();
        self.queue.clear();
        self.preview_wanted = None;
        self.preview_shown = None;
        self.held = false;
    }
}

/// The image file shader `name` is pictured by: a sky's box front, else the image the
/// world renderer selects for it.
fn picture_path(
    vfs: &sjk_vfs::VirtualFileSystem,
    catalog: &sjk_shader::ShaderCatalog,
    name: &str,
) -> Option<String> {
    let sky = catalog
        .get(name)
        .is_some_and(|definition| definition.sky.is_some());
    if sky && let Some(path) = super::web::sky_picture(vfs, catalog, name) {
        return Some(path);
    }
    catalog
        .resolve_image(vfs, name)
        .ok()
        .flatten()
        .map(|path| path.as_str().to_owned())
}

/// Shader `name`'s picture, decoded.
fn decode(
    vfs: &sjk_vfs::VirtualFileSystem,
    catalog: &sjk_shader::ShaderCatalog,
    name: &str,
) -> Option<std::sync::Arc<image::RgbaImage>> {
    let path = picture_path(vfs, catalog, name)?;
    crate::decoded_image_cache::cached_decoded_image(vfs, &path)
        .ok()
        .flatten()
}

/// `image` as a thumbnail: square, its middle kept when it is not.
fn thumbnail_of(image: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = image.dimensions();
    let side = width.min(height).max(1);
    let view = image::imageops::crop_imm(
        image,
        (width - side.min(width)) / 2,
        (height - side.min(height)) / 2,
        side,
        side,
    );
    image::imageops::resize(&*view, CELL, CELL, image::imageops::FilterType::Triangle)
}

/// `image` reduced to [`PREVIEW_EDGE`] at most along its longest edge.
fn preview_of(image: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = image.dimensions();
    let longest = width.max(height).max(1);
    if longest <= PREVIEW_EDGE {
        return image.clone();
    }
    let scale = PREVIEW_EDGE as f32 / longest as f32;
    image::imageops::resize(
        image,
        ((width as f32 * scale).round() as u32).max(1),
        ((height as f32 * scale).round() as u32).max(1),
        image::imageops::FilterType::Triangle,
    )
}

impl GpuState {
    /// After the panel's build: decode and upload a few of the pictures it asked for.
    pub(crate) fn map_edit_pictures_frame(&mut self) {
        let Some(vfs) = self.vfs.clone() else {
            return;
        };
        let mut budget = PER_FRAME;
        if let Some(shader) = self.map_edits.panel.pictures.next_preview() {
            budget -= 1;
            let image = decode(&vfs, &self.shaders, &shader);
            let size = image.as_ref().map(|image| {
                let reduced = preview_of(image);
                let size = [reduced.width(), reduced.height()];
                let levels = crate::menu::levelshot::LevelshotImage::from_rgba(reduced);
                self.ui_shapes
                    .upload_map_edit_preview(&self.device, &self.queue, &levels);
                size
            });
            let pictures = &mut self.map_edits.panel.pictures;
            pictures.held |= size.is_some();
            pictures.preview_shown = Some((shader, size));
        }
        for _ in 0..budget {
            let Some(shader) = self.map_edits.panel.pictures.next_thumbnail() else {
                break;
            };
            let Some(image) = decode(&vfs, &self.shaders, &shader) else {
                self.map_edits.panel.pictures.mark_missing(shader);
                continue;
            };
            let Some(cell) = self.map_edits.panel.pictures.cell_for(shader) else {
                break;
            };
            let thumbnail = thumbnail_of(&image);
            self.ui_shapes.upload_map_edit_cell(
                &self.device,
                &self.queue,
                cell,
                thumbnail.as_raw(),
            );
            self.map_edits.panel.pictures.held = true;
        }
    }

    /// While the panel is not drawn: drop its pictures' textures, once.
    pub(crate) fn map_edit_pictures_release(&mut self) {
        if !self.map_edits.panel.pictures.held {
            return;
        }
        self.ui_shapes.release_map_edit_pictures();
        self.map_edits.panel.pictures.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows drawn ask for their pictures; those asked by an older build (scrolled
    /// away) are dropped; a full grid gives up the least lately drawn cell, never one on
    /// show.
    #[test]
    fn pictures_are_asked_by_the_rows_drawn_and_cells_are_reused() {
        let mut pictures = Pictures::default();
        pictures.begin();
        assert_eq!(pictures.thumbnail("textures/a/One"), None);
        assert_eq!(pictures.thumbnail("textures/a/two"), None);
        assert_eq!(pictures.thumbnail("textures/a/one"), None, "asked once");
        assert_eq!(pictures.queue.len(), 2);
        pictures.begin();
        assert_eq!(pictures.thumbnail("textures/a/two"), None);
        assert_eq!(pictures.next_thumbnail().as_deref(), Some("textures/a/two"));
        assert_eq!(pictures.next_thumbnail(), None, "one was scrolled away");
        let cell = pictures.cell_for("textures/a/two".into()).unwrap();
        assert_eq!(
            pictures.thumbnail("TEXTURES/A/TWO"),
            Some(Some(cell_texture(cell)))
        );
        pictures.mark_missing("textures/a/none".into());
        assert_eq!(pictures.thumbnail("textures/a/none"), Some(None));
        // Fill the grid, all drawn in this build: no cell is free.
        for index in 1..CELLS {
            pictures.cell_for(format!("textures/b/{index}")).unwrap();
        }
        assert_eq!(pictures.cell_for("textures/c/new".into()), None);
        // The next build draws only one of them: the others give up their cells.
        pictures.begin();
        let _ = pictures.thumbnail("textures/b/1");
        let cell = pictures.cell_for("textures/c/new".into()).unwrap();
        assert_ne!(pictures.cells[cell as usize].0, "textures/b/1");
        assert_eq!(pictures.cells[cell as usize].0, "textures/c/new");
    }

    #[test]
    fn the_preview_is_asked_for_and_then_shown() {
        let mut pictures = Pictures::default();
        pictures.begin();
        assert_eq!(pictures.preview("textures/a/wall"), None);
        assert_eq!(pictures.next_preview().as_deref(), Some("textures/a/wall"));
        pictures.preview_shown = Some(("textures/a/wall".into(), Some([256, 128])));
        assert_eq!(
            pictures.preview("Textures/A/Wall"),
            Some(Some((PREVIEW, [256, 128])))
        );
        // Asked by an older build only: not decoded.
        assert_eq!(pictures.preview("textures/a/floor"), None);
        pictures.begin();
        assert_eq!(pictures.next_preview(), None);
    }

    #[test]
    fn a_thumbnail_keeps_the_middle_of_a_wide_picture() {
        let mut image = image::RgbaImage::new(256, 128);
        for (x, _, pixel) in image.enumerate_pixels_mut() {
            *pixel = if (64..192).contains(&x) {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            };
        }
        let thumbnail = thumbnail_of(&image);
        assert_eq!(thumbnail.dimensions(), (CELL, CELL));
        assert_eq!(thumbnail.get_pixel(2, 64).0, [255, 0, 0, 255]);
        assert_eq!(preview_of(&image).dimensions(), (256, 128));
        let big = image::RgbaImage::new(2048, 1024);
        assert_eq!(preview_of(&big).dimensions(), (PREVIEW_EDGE, 256));
    }
}
