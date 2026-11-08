//! DPI-aware text layout and cached glyph atlases for the client UI.
//!
//! Inter is rasterized once when the graphics device is created.  The render
//! loop only performs glyph lookup and appends vertices into reused buffers;
//! it never rasterizes a glyph or grows the atlas.  The console font,
//! JetBrains Mono ([`console_font`]), and the vector replacements for the retail
//! game fonts ([`retail_font`]), which the classic HUD font and the optional game
//! fonts ([`crate::game_font`]) use, are rasterized the same way. The retail fonts'
//! `¬` logo, bundled as a vector glyph, replaces Inter's ([`logo_glyph`]).
//! Characters outside those atlases' 256 code-page slots (Cyrillic, Japanese) come
//! from one shared atlas filled as they first appear ([`wide`]).

mod bounded;
mod cell;
pub(crate) mod charset;
pub(crate) mod console_font;
pub(crate) mod emoji;
pub(crate) mod logo_glyph;
pub(crate) mod retail_font;
pub(crate) mod sdf;
pub(crate) mod style;
pub(crate) mod system_font;
pub(crate) mod vertex_buffer;
pub(crate) mod wide;
pub(crate) mod wide_texture;
pub(crate) use bounded::append_bounded;
pub(crate) use cell::append_cell;
pub(crate) use logo_glyph::LogoGlyph;
pub(crate) use style::TextStyle;

use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings, Metrics};
use image::{GrayImage, Luma};
use std::error::Error;

/// Vertices one text batch (a font's layer, the Inter HUD text) holds in a frame. Its
/// GPU buffer starts smaller and grows to it ([`vertex_buffer`]): the chat outline
/// draws nine quads a letter, so the old 32 768 cap (606 outlined letters) dropped
/// whole chat lines (hub #211).
pub(crate) const MAX_TEXT_VERTICES: usize = 196_608;
const GLYPH_COUNT: usize = 256;
const ATLAS_WIDTH: u32 = 4_096;
/// Glyph gutter; must stay >= 2 px at the deepest mip level so minified
/// sampling never bleeds a neighbour's coverage into a glyph edge.
const ATLAS_PADDING: u32 = 2 << (ATLAS_MIP_LEVELS - 1);
/// Modern glyphs are rasterized at 3x and minified for small UI text.
const MODERN_RASTER_SCALE: f32 = 3.0;
/// Mip levels uploaded for the modern atlas (96 px raster down to 12 px).
pub(crate) const ATLAS_MIP_LEVELS: u32 = 4;
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const RAJDHANI_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Rajdhani-SemiBold.ttf");
pub(crate) const RAJDHANI_BOLD: &[u8] = include_bytes!("../assets/fonts/Rajdhani-Bold.ttf");
const EXO2_REGULAR: &[u8] = include_bytes!("../assets/fonts/Exo2-Regular.ttf");
const EXO2_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Exo2-SemiBold.ttf");
const FIRA_CONDENSED_SEMIBOLD: &[u8] =
    include_bytes!("../assets/fonts/FiraSansCondensed-SemiBold.ttf");
const FIRA_CONDENSED_BOLD: &[u8] = include_bytes!("../assets/fonts/FiraSansCondensed-Bold.ttf");

/// One family of the vector UI atlas: its [`TextFace::Regular`] and
/// [`TextFace::Semibold`] fonts, and fonts of the same weights lending the
/// glyphs those lack.
pub(crate) struct Family {
    pub(crate) name: &'static str,
    faces: [&'static [u8]; 2],
    fallback: Option<[&'static [u8]; 2]>,
    /// Fonts lending each face the characters beyond its code-page slots.
    wide: [&'static [wide::Source]; 2],
}

/// Inter, the HUD's, chat's and SJK's own screens' font.
pub(crate) const INTER: Family = Family {
    name: "Inter",
    faces: [INTER_REGULAR, INTER_SEMIBOLD],
    fallback: None,
    wide: wide::INTER,
};

/// The SJK UI's display family, as on SJK's site: Rajdhani SemiBold and Bold
/// for navigation and titles. It has no superscripts, fractions or ordinals,
/// so those come from Exo 2 SemiBold.
pub(crate) const DISPLAY: Family = Family {
    name: "Rajdhani",
    faces: [RAJDHANI_SEMIBOLD, RAJDHANI_BOLD],
    fallback: Some([EXO2_SEMIBOLD, EXO2_SEMIBOLD]),
    wide: [wide::EXO2[1], wide::EXO2[1]],
};

/// The SJK UI's body family, as on SJK's site: Exo 2 Regular and SemiBold
/// (static instances of the variable font), covering all of Windows-1252.
pub(crate) const BODY: Family = Family {
    name: "Exo 2",
    faces: [EXO2_REGULAR, EXO2_SEMIBOLD],
    fallback: None,
    wide: wide::EXO2,
};

/// The headline family: Fira Sans Condensed SemiBold and Bold, close to the
/// retail `ergoec` but a real typeface, for centre prints and the words in the
/// world ([`crate::game_font::RetailFont::Headline`]). Subset to Windows-1252;
/// what lies beyond comes from Exo 2 SemiBold.
pub(crate) const HEADLINE: Family = Family {
    name: "Fira Sans Condensed",
    faces: [FIRA_CONDENSED_SEMIBOLD, FIRA_CONDENSED_BOLD],
    fallback: None,
    wide: [wide::EXO2[1], wide::EXO2[1]],
};

/// Font weight available in the modern UI atlas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextFace {
    Regular,
    Semibold,
}

impl TextFace {
    fn index(self) -> usize {
        match self {
            Self::Regular => 0,
            Self::Semibold => 1,
        }
    }
}

/// A single cached glyph: layout metrics in font units (physical pixels at
/// scale 1) and its rectangle in normalized atlas coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FontGlyph {
    /// Ink width.
    pub(crate) width: f32,
    /// Ink height.
    pub(crate) height: f32,
    /// Pen advance.
    pub(crate) advance: f32,
    /// Ink left edge from the pen.
    pub(crate) offset_x: f32,
    /// Ink top edge below the line top.
    pub(crate) offset_y: f32,
    /// Atlas rectangle `[u0, v0, u1, v1]`, `v0` at the ink top.
    pub(crate) uv: [f32; 4],
}

/// Immutable glyph metrics used by all menu, console, HUD, and chat layout.
#[derive(Clone)]
pub(crate) struct UiFont {
    glyphs: [[FontGlyph; GLYPH_COUNT]; 2],
    /// Fonts lending each face the characters beyond its slots ([`wide`]).
    wide: [&'static [wide::Source]; 2],
    /// Baseline-to-baseline line height in physical framebuffer pixels.
    pub(crate) height: f32,
    modern: bool,
    /// Player size and spacing preference for menu text, see [`style`].
    style: TextStyle,
}

impl UiFont {
    /// Metrics and atlas rectangle of `byte` (Latin-1) in `face`, for layouts
    /// outside the 2D text path such as the ground HUD's world-space numbers.
    pub(crate) fn glyph(&self, face: TextFace, byte: u8) -> FontGlyph {
        self.glyphs[self.face_index(face)][usize::from(byte)]
    }

    /// Metrics and atlas rectangle of `key` in `face`: a code-page slot from the
    /// font's own atlas, a wide character from the wide atlas ([`wide`]), scaled so its
    /// capitals match this font's (`H`) and set on the same baseline. A character no
    /// font of the face has draws `?`.
    pub(crate) fn glyph_of(&self, face: TextFace, key: GlyphKey) -> FontGlyph {
        let character = match key {
            GlyphKey::Slot(byte) => return self.glyph(face, byte),
            GlyphKey::Wide(character) => character,
            GlyphKey::Emoji(id) => return self.emoji_glyph(face, id),
        };
        if invisible(character) {
            return FontGlyph::default();
        }
        let index = self.face_index(face);
        let Some(glyph) = wide::lookup(self.wide[index], character) else {
            return self.glyph(face, b'?');
        };
        let capital = self.glyphs[index][usize::from(b'H')];
        let ratio = glyph.source.map_or(1.0, wide::Source::cap_ratio);
        let unit = capital.height / (ratio * wide::RASTER_EM);
        FontGlyph {
            width: glyph.width * unit,
            height: glyph.height * unit,
            advance: glyph.advance * unit,
            offset_x: glyph.left * unit,
            offset_y: capital.offset_y + capital.height - glyph.top * unit,
            uv: glyph.uv,
        }
    }

    /// Emoji `id` ([`emoji`]) in `face`: a square an em of this font wide (the em
    /// taken from its capitals, at Inter's 0.73 of an em), sitting a tenth of itself
    /// below the baseline as emoji fonts set it, with a twentieth of itself either side.
    /// It has no picture (`uv` all 0) until the picture is decoded.
    fn emoji_glyph(&self, face: TextFace, id: u16) -> FontGlyph {
        let capital = self.glyphs[self.face_index(face)][usize::from(b'H')];
        let side = capital.height / 0.73;
        let baseline = capital.offset_y + capital.height;
        let uv = emoji::picture(id);
        let size = if uv.is_some() { side } else { 0.0 };
        FontGlyph {
            width: size,
            height: size,
            advance: side * 1.1,
            offset_x: side * 0.05,
            offset_y: baseline + side * 0.1 - side,
            uv: uv.unwrap_or_default(),
        }
    }

    fn face_index(&self, face: TextFace) -> usize {
        if self.modern { face.index() } else { 0 }
    }

    /// How far below the top of a line drawn at `scale` the middle of the
    /// capitals sits: where an icon set in the line centres to sit level with it.
    pub(crate) fn capital_middle(&self, scale: f32) -> f32 {
        let capital = self.glyph(TextFace::Regular, b'H');
        (capital.offset_y + capital.height * 0.5) * scale
    }

    /// How tall the capitals of a line drawn at `scale` are: what an icon set in the
    /// line is sized from to match its words.
    pub(crate) fn capital_height(&self, scale: f32) -> f32 {
        self.glyph(TextFace::Regular, b'H').height * scale
    }

    /// Whether this is the bundled vector font rather than the retail HUD font.
    pub(crate) const fn is_modern(&self) -> bool {
        self.modern
    }

    /// The player's menu text style (neutral until the cvars are synced).
    pub(crate) const fn style(&self) -> TextStyle {
        self.style
    }

    /// Apply the player's `ui_textScale` / `ui_letterSpacing` preference.
    pub(crate) fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }
}

/// CPU assets uploaded once to the GPU text texture.
pub(crate) struct FontAtlas {
    pub(crate) font: UiFont,
    /// Glyph coverage, one channel (uploaded as `R8Unorm`, read from `.r`).
    pub(crate) image: GrayImage,
    /// `image` is a signed distance field for the distance-field text pipeline.
    pub(crate) distance_field: bool,
}

/// Vertex consumed by `text.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct TextVertex {
    position: [f32; 2],
    texture_coordinates: [f32; 2],
    color: [f32; 4],
}

impl TextVertex {
    /// The vertex's colour, used by drawing assertions.
    #[cfg(test)]
    pub(crate) fn colour(&self) -> [f32; 4] {
        self.color
    }

    /// The vertex's position in window pixels of `viewport`.
    pub(crate) fn window_position(&self, viewport: [f32; 2]) -> [f32; 2] {
        [
            (self.position[0] + 1.0) * 0.5 * viewport[0],
            (1.0 - self.position[1]) * 0.5 * viewport[1],
        ]
    }

    /// Make one already-built text vertex transparent.
    pub(crate) fn hide(&mut self) {
        self.color[3] = 0.0;
    }
}

impl TextVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    /// WGPU layout shared by all text draw calls.
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct RasterizedGlyph {
    face: usize,
    byte: usize,
    metrics: Metrics,
    pixels: Vec<u8>,
}

/// Rasterize Inter Regular and SemiBold at the current monitor DPI.
///
/// Latin-1 is deliberately pre-cached: protocol-26 UI strings are byte based,
/// so this covers the complete wire character range without atlas mutation in
/// the frame loop. With the retail `logo` from the game data, byte 0xAC draws
/// it in both faces instead of Inter's `¬`, as Jedi Academy's fonts do.
pub(crate) fn load_modern(
    dpi_scale: f64,
    logo: Option<&LogoGlyph>,
) -> Result<FontAtlas, Box<dyn Error>> {
    load_family(&INTER, dpi_scale, logo)
}

/// How wide `text` is in the display family's regular face (Rajdhani SemiBold) at
/// `size`, the line height the SJK UI sets its text at, from the font's own
/// advances: for a layout made before the glyph atlas is at hand (the Profile
/// screen's tabs). The font is read once; 0 if it cannot be.
pub(crate) fn display_width(text: &str, size: f32) -> f32 {
    /// The size the advances are read at; they scale with it.
    const READ_AT: f32 = 100.0;
    static FONT: std::sync::OnceLock<Option<(Font, f32)>> = std::sync::OnceLock::new();
    let Some((font, line)) = FONT.get_or_init(|| {
        let font = Font::from_bytes(RAJDHANI_SEMIBOLD, FontSettings::default()).ok()?;
        let line = font.horizontal_line_metrics(READ_AT)?.new_line_size;
        Some((font, line))
    }) else {
        return 0.0;
    };
    let advances: f32 = text
        .chars()
        .map(|character| font.metrics(character, READ_AT).advance_width)
        .sum();
    advances / line * size
}

/// Rasterize `family`'s two faces at the current monitor DPI into one atlas,
/// as [`load_modern`] does Inter's.
pub(crate) fn load_family(
    family: &Family,
    dpi_scale: f64,
    logo: Option<&LogoGlyph>,
) -> Result<FontAtlas, Box<dyn Error>> {
    let font = |bytes: &[u8]| Font::from_bytes(bytes, FontSettings::default());
    let fonts = [font(family.faces[0])?, font(family.faces[1])?];
    let fallbacks = match family.fallback {
        Some([regular, strong]) => Some([font(regular)?, font(strong)?]),
        None => None,
    };
    // The largest menu face is 2.7x body size.  A 3x source atlas means every
    // production text size is sampled at native resolution or downsampled,
    // never magnified from a small bitmap as the retail font was.
    let pixel_size = (32.0 * dpi_scale.clamp(1.0, 3.0) as f32 * MODERN_RASTER_SCALE).round();
    let line_metrics = fonts[0]
        .horizontal_line_metrics(pixel_size)
        .ok_or_else(|| format!("{} has no horizontal line metrics", family.name))?;
    let mut rasterized = Vec::with_capacity(GLYPH_COUNT * fonts.len());
    for (face, font) in fonts.iter().enumerate() {
        let fallback = fallbacks.as_ref().map(|fallbacks| &fallbacks[face]);
        for byte in 0..GLYPH_COUNT {
            // Slot `byte` holds the Windows-1252 character of that byte, as JKA's
            // own fonts do, so 0x80 is `€` rather than an invisible C1 control.
            let (font, character) = slot_glyph(font, fallback, byte as u8);
            let (metrics, pixels) = font.rasterize(character, pixel_size);
            rasterized.push(RasterizedGlyph {
                face,
                byte,
                metrics,
                pixels,
            });
        }
    }
    if let Some(logo) = logo {
        for face in 0..fonts.len() {
            let cap_height = rasterized[face * GLYPH_COUNT + usize::from(b'H')]
                .metrics
                .height as f32;
            let glyph = &mut rasterized[face * GLYPH_COUNT + usize::from(logo_glyph::BYTE)];
            let scaled = logo.rasterize(cap_height);
            glyph.metrics = Metrics {
                xmin: scaled.xmin,
                ymin: scaled.ymin,
                width: scaled.width,
                height: scaled.height,
                advance_width: scaled.advance,
                ..glyph.metrics
            };
            glyph.pixels = scaled.pixels;
        }
    }

    let (image, rectangles) = paint_atlas(&rasterized);
    let mut glyphs = [[FontGlyph::default(); GLYPH_COUNT]; 2];
    for (glyph, uv) in rasterized.iter().zip(rectangles) {
        let top = (line_metrics.ascent - (glyph.metrics.ymin as f32 + glyph.metrics.height as f32))
            / MODERN_RASTER_SCALE;
        glyphs[glyph.face][glyph.byte] = FontGlyph {
            width: glyph.metrics.width as f32 / MODERN_RASTER_SCALE,
            height: glyph.metrics.height as f32 / MODERN_RASTER_SCALE,
            advance: glyph.metrics.advance_width / MODERN_RASTER_SCALE,
            offset_x: glyph.metrics.xmin as f32 / MODERN_RASTER_SCALE,
            offset_y: top,
            uv,
        };
    }
    Ok(FontAtlas {
        font: UiFont {
            glyphs,
            wide: family.wide,
            height: line_metrics.new_line_size / MODERN_RASTER_SCALE,
            modern: true,
            style: TextStyle::NEUTRAL,
        },
        image,
        distance_field: false,
    })
}

/// Pack `rasterized` into a coverage atlas [`ATLAS_WIDTH`] wide with mipmappable
/// padding, and return it with each glyph's rectangle `[u0, v0, u1, v1]`.
fn paint_atlas(rasterized: &[RasterizedGlyph]) -> (GrayImage, Vec<[f32; 4]>) {
    paint_atlas_within(rasterized, u32::MAX).expect("no height limit")
}

/// The size a glyph takes in the atlas: its own, or nothing for one wider than the
/// atlas or whose pixels do not match its size (it then draws no ink).
fn drawn_size(glyph: &RasterizedGlyph) -> (u32, u32) {
    let metrics = glyph.metrics;
    let fits = metrics.width as u64 + 2 * u64::from(ATLAS_PADDING) <= u64::from(ATLAS_WIDTH)
        && metrics.width.checked_mul(metrics.height) == Some(glyph.pixels.len());
    if fits {
        (metrics.width as u32, metrics.height as u32)
    } else {
        (0, 0)
    }
}

/// [`paint_atlas`], or `None` when the atlas would be taller than `max_height`
/// (decided before it is allocated).
fn paint_atlas_within(
    rasterized: &[RasterizedGlyph],
    max_height: u32,
) -> Option<(GrayImage, Vec<[f32; 4]>)> {
    let placements = pack_glyphs(rasterized);
    let height = placements
        .iter()
        .zip(rasterized)
        .map(|([_, y], glyph)| {
            u64::from(*y) + u64::from(drawn_size(glyph).1) + u64::from(ATLAS_PADDING)
        })
        .max()
        .unwrap_or(1)
        .next_power_of_two();
    if height > u64::from(max_height) {
        return None;
    }
    let height = height as u32;
    // Coverage only: the text colour comes from the vertices, so the atlas needs
    // no RGB (the shaders read coverage from the red channel).
    let mut image = GrayImage::new(ATLAS_WIDTH, height);
    let rectangles = rasterized
        .iter()
        .zip(placements)
        .map(|(glyph, [x, y])| {
            let (width, rows) = drawn_size(glyph);
            for row in 0..rows {
                for column in 0..width {
                    let coverage = glyph.pixels[(row * width + column) as usize];
                    image.put_pixel(x + column, y + row, Luma([coverage]));
                }
            }
            [
                x as f32 / ATLAS_WIDTH as f32,
                y as f32 / height as f32,
                (x + width) as f32 / ATLAS_WIDTH as f32,
                (y + rows) as f32 / height as f32,
            ]
        })
        .collect();
    Some((image, rectangles))
}

fn pack_glyphs(glyphs: &[RasterizedGlyph]) -> Vec<[u32; 2]> {
    let mut positions = Vec::with_capacity(glyphs.len());
    let mut x = ATLAS_PADDING;
    let mut y = ATLAS_PADDING;
    let mut row_height = 0;
    for glyph in glyphs {
        let (width, height) = drawn_size(glyph);
        if x + width + ATLAS_PADDING > ATLAS_WIDTH {
            x = ATLAS_PADDING;
            y = y.saturating_add(row_height + ATLAS_PADDING);
            row_height = 0;
        }
        positions.push([x, y]);
        x += width + ATLAS_PADDING;
        row_height = row_height.max(height);
    }
    positions
}

/// The classic status HUD's font: SJK HUD, the bundled vector replacement for
/// retail's `arialnb` ([`retail_font::HUD`]).
pub(crate) fn load_classic() -> Result<FontAtlas, Box<dyn Error>> {
    retail_font::load(&retail_font::HUD)
}

/// `text` without the `^<digit>` colour codes the renderer reads, written
/// without allocating: `Plain("^1J^7oF")` shows `JoF`.
pub(crate) struct Plain<'a>(pub(crate) &'a str);

impl std::fmt::Display for Plain<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut rest = self.0;
        while let Some(at) = rest.find('^') {
            formatter.write_str(&rest[..at])?;
            let after = &rest[at + 1..];
            rest = match after.as_bytes().first() {
                Some(code) if code.is_ascii_digit() => &after[1..],
                _ => {
                    formatter.write_str("^")?;
                    after
                }
            };
        }
        formatter.write_str(rest)
    }
}

/// How text draws its `^<digit>` colour codes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CodePalette {
    /// The game's own colours ([`quake_color`]).
    #[default]
    Game,
    /// The same hues lifted to read on the SJK UI's navy ground: black,
    /// red, green, blue, magenta and grey lighter, the rest as the game's.
    Legible,
    /// The game's own colours, a dark glyph edged in a soft light halo instead of
    /// its shadow, so a black name reads on a dark nameplate (Sol's pick, 28/10/2026).
    Haloed,
    /// As [`Self::Haloed`], the halo stronger, and every other glyph edged in a dark
    /// outline instead of its shadow (`cg_nameplateOutline 2`; Lex's idea, 29/10/2026:
    /// names hard to read on some backgrounds).
    Outlined,
    /// The game's own colours, every glyph edged in a dark outline instead of its
    /// shadow but a black one, which an outline as dark as it would hide: it keeps the
    /// faint light halo of [`Self::Haloed`] (`cg_chatBoxOutline`; Creo's idea,
    /// 29/10/2026: chat hard to read over a bright map).
    DarkOutlined,
    /// As [`Self::Haloed`], a dark glyph edged in a crisp white outline instead of the
    /// soft halo ([`DARK_OUTLINE_CVAR`]; Sol's request, 03/11/2026: the halo reads as a
    /// glow), the rest in its shadow.
    WhiteEdged,
    /// Every glyph outlined instead of its shadow: a dark one in white, the rest in the
    /// dark outline ([`Self::Outlined`] or [`Self::DarkOutlined`] with
    /// [`DARK_OUTLINE_CVAR`] on).
    TwoTone,
}

/// `cg_darkTextOutline`: dark letters of names and chat edged in a white outline
/// instead of the soft light halo; off by default.
pub(crate) const DARK_OUTLINE_CVAR: &str = "cg_darkTextOutline";

/// Darkest glyph colour drawn without a halo in [`CodePalette::Haloed`], by its
/// luminance.
const HALO_BELOW: f32 = 0.3;

/// How a glyph is set off from what is behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Edge {
    /// The dark drop shadow, a step down and right.
    Shadow,
    /// Eight copies a step out all round, each in this colour (its alpha the copy's).
    Ring([f32; 4]),
}

/// A dark glyph's pale halo, and how opaque each of its copies is: faint (Sol,
/// 29/10/2026: 0.38 drew too strong an outline), or strong in [`CodePalette::Outlined`].
const HALO_TINT: [f32; 3] = [0.86, 0.89, 0.95];
const HALO_ALPHA: f32 = 0.18;
const HALO_ALPHA_STRONG: f32 = 0.38;
/// A light glyph's dark outline in [`CodePalette::Outlined`].
const OUTLINE_TINT: [f32; 3] = [0.04, 0.04, 0.06];
const OUTLINE_ALPHA: f32 = 0.45;
/// A dark glyph's white outline in [`CodePalette::WhiteEdged`] and
/// [`CodePalette::TwoTone`]: opaque enough per copy that the ring reads as a line,
/// not a glow.
const WHITE_TINT: [f32; 3] = [1.0, 1.0, 1.0];
const WHITE_ALPHA: f32 = 0.85;
/// Darkest glyph colour [`CodePalette::DarkOutlined`] edges in a dark outline, by its
/// luminance: below it (black, `^0`) the glyph keeps the faint light halo.
const OUTLINE_BLACK_BELOW: f32 = 0.05;

/// How light `color` looks (Rec. 709 luminance of its display value).
fn luminance(color: [f32; 4]) -> f32 {
    0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]
}

impl CodePalette {
    /// Whether a glyph of `color` gets a light halo instead of its dark shadow.
    pub(crate) fn halo(self, color: [f32; 4]) -> bool {
        match self {
            Self::Haloed | Self::Outlined | Self::WhiteEdged | Self::TwoTone => {
                luminance(color) < HALO_BELOW
            }
            Self::DarkOutlined => luminance(color) < OUTLINE_BLACK_BELOW,
            Self::Game | Self::Legible => false,
        }
    }

    /// How a glyph of `color` is set off: a ring of copies or its shadow.
    pub(crate) fn edge(self, color: [f32; 4]) -> Edge {
        let ring =
            |tint: [f32; 3], alpha: f32| Edge::Ring([tint[0], tint[1], tint[2], alpha * color[3]]);
        match (self, self.halo(color)) {
            (Self::Haloed | Self::DarkOutlined, true) => ring(HALO_TINT, HALO_ALPHA),
            (Self::Outlined, true) => ring(HALO_TINT, HALO_ALPHA_STRONG),
            (Self::WhiteEdged | Self::TwoTone, true) => ring(WHITE_TINT, WHITE_ALPHA),
            (Self::Outlined | Self::DarkOutlined | Self::TwoTone, false) => {
                ring(OUTLINE_TINT, OUTLINE_ALPHA)
            }
            _ => Edge::Shadow,
        }
    }

    /// The nameplates' palette for `cg_nameplateOutline`: 0 the shadow only, 1 (the
    /// default) a faint halo on dark letters, 2 a strong one and a dark outline on the
    /// rest.
    pub(crate) fn nameplate(outline: i64) -> Self {
        match outline {
            ..=0 => Self::Game,
            1 => Self::Haloed,
            _ => Self::Outlined,
        }
    }

    /// This palette with [`DARK_OUTLINE_CVAR`] applied: `white` swaps the light halo
    /// round dark glyphs for a white outline; a palette with no halo is unchanged.
    pub(crate) fn white_outlined(self, white: bool) -> Self {
        match (self, white) {
            (Self::Haloed, true) => Self::WhiteEdged,
            (Self::Outlined | Self::DarkOutlined, true) => Self::TwoTone,
            _ => self,
        }
    }

    /// Whether [`DARK_OUTLINE_CVAR`] is on.
    pub(crate) fn white_outline_on(console: Option<&crate::console::ViewerConsole>) -> bool {
        console
            .and_then(|c| c.integer_cvar(DARK_OUTLINE_CVAR))
            .unwrap_or(0)
            != 0
    }

    /// The colour of code `index` (0-9), as a display value.
    pub(crate) fn colour(self, index: u8) -> [f32; 4] {
        match (self, index) {
            (Self::Legible, 0) => [0.55, 0.57, 0.62, 1.0],
            (Self::Legible, 1) => [1.0, 0.36, 0.36, 1.0],
            (Self::Legible, 2) => [0.4, 1.0, 0.45, 1.0],
            (Self::Legible, 4) => [0.45, 0.6, 1.0, 1.0],
            (Self::Legible, 6) => [1.0, 0.45, 1.0, 1.0],
            (Self::Legible, 9) => [0.68, 0.7, 0.74, 1.0],
            _ => quake_color(index),
        }
    }
}

/// Colour of the `^<digit>` code `index` (0-9), as a display value.
///
/// OpenJK's `g_color_table` (`shared/qcommon/q_color.c`) at full strength: the
/// 2D layer draws display values ([`crate::ui_target`]), so `^1` is the same
/// pure red retail showed. The table has ten entries and `ColorIndex` masks with
/// `Q_COLOR_BITS` (0xF), so `^8` is orange and `^9` mid grey rather than
/// retail's `& 7` wrap to black and red.
pub(crate) fn quake_color(index: u8) -> [f32; 4] {
    match index {
        0 => [0.0, 0.0, 0.0, 1.0],
        1 => [1.0, 0.0, 0.0, 1.0],
        2 => [0.0, 1.0, 0.0, 1.0],
        3 => [1.0, 1.0, 0.0, 1.0],
        4 => [0.0, 0.0, 1.0, 1.0],
        5 => [0.0, 1.0, 1.0, 1.0],
        6 => [1.0, 0.0, 1.0, 1.0],
        8 => [1.0, 0.5, 0.0, 1.0],
        9 => [0.5, 0.5, 0.5, 1.0],
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}

/// Leave out the glyphs from `vertices[from..]` that touch `rect` (window pixels in a
/// `viewport`): the page's text under a card laid over it
/// ([`crate::menu_widgets::MenuCanvas::cover`]). Every glyph is one quad of six
/// vertices ([`push_quad`]).
pub(crate) fn drop_quads_under(
    vertices: &mut Vec<TextVertex>,
    from: usize,
    rect: sjk_ui::Rect,
    viewport: [f32; 2],
) {
    let left = rect.x / viewport[0] * 2.0 - 1.0;
    let right = rect.right() / viewport[0] * 2.0 - 1.0;
    let top = 1.0 - rect.y / viewport[1] * 2.0;
    let bottom = 1.0 - (rect.y + rect.height) / viewport[1] * 2.0;
    let mut kept = from;
    let mut quad = from;
    while quad + 6 <= vertices.len() {
        let corners = &vertices[quad..quad + 6];
        let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
        for vertex in corners {
            for axis in 0..2 {
                min[axis] = min[axis].min(vertex.position[axis]);
                max[axis] = max[axis].max(vertex.position[axis]);
            }
        }
        let under = max[0] > left && min[0] < right && max[1] > bottom && min[1] < top;
        if !under {
            vertices.copy_within(quad..quad + 6, kept);
            kept += 6;
        }
        quad += 6;
    }
    vertices.truncate(kept);
}

/// Move `vertices` so the bounds of their glyphs centre on `centre` (pixels from the top
/// left of `viewport`): a word in a box, centred by its ink rather than its line box
/// (a key cap, `use_prompt`).
pub(crate) fn centre_vertices(vertices: &mut [TextVertex], centre: [f32; 2], viewport: [f32; 2]) {
    let Some(first) = vertices.first() else {
        return;
    };
    let (mut low, mut high) = (first.position, first.position);
    for vertex in vertices.iter() {
        for axis in 0..2 {
            low[axis] = low[axis].min(vertex.position[axis]);
            high[axis] = high[axis].max(vertex.position[axis]);
        }
    }
    let target = [
        centre[0] / viewport[0] * 2.0 - 1.0,
        1.0 - centre[1] / viewport[1] * 2.0,
    ];
    let shift = [
        target[0] - (low[0] + high[0]) * 0.5,
        target[1] - (low[1] + high[1]) * 0.5,
    ];
    map_vertices(vertices, |point| [point[0] + shift[0], point[1] + shift[1]]);
}

/// Move every vertex of `vertices` by `map` (normalized device coordinates in and out):
/// text drawn under a transform ([`sjk_ui::DrawCommand::PushTransform`]).
pub(crate) fn map_vertices(vertices: &mut [TextVertex], map: impl Fn([f32; 2]) -> [f32; 2]) {
    for vertex in vertices {
        vertex.position = map(vertex.position);
    }
}

fn push_quad(
    vertices: &mut Vec<TextVertex>,
    rectangle: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    viewport: [f32; 2],
) {
    push_quad_within(vertices, rectangle, uv, color, viewport, MAX_TEXT_VERTICES);
}

/// [`push_quad`] into a batch of at most `limit` vertices.
fn push_quad_within(
    vertices: &mut Vec<TextVertex>,
    rectangle: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    viewport: [f32; 2],
    limit: usize,
) {
    if vertices.len() + 6 > limit {
        return;
    }
    let [x, y, width, height] = rectangle;
    let ndc = |position: [f32; 2]| {
        [
            position[0] / viewport[0] * 2.0 - 1.0,
            1.0 - position[1] / viewport[1] * 2.0,
        ]
    };
    let points = [
        ([x, y], [uv[0], uv[1]]),
        ([x + width, y], [uv[2], uv[1]]),
        ([x + width, y + height], [uv[2], uv[3]]),
        ([x, y], [uv[0], uv[1]]),
        ([x + width, y + height], [uv[2], uv[3]]),
        ([x, y + height], [uv[0], uv[3]]),
    ];
    vertices.extend(points.map(|(position, texture_coordinates)| TextVertex {
        position: ndc(position),
        texture_coordinates,
        color,
    }));
}

/// Append anti-aliased glyph quads without allocating.
pub(crate) fn append_text(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
) -> usize {
    append_text_face(
        vertices,
        font,
        text,
        origin,
        scale,
        viewport,
        TextFace::Regular,
    )
}

/// Append text using an explicit Inter weight.
pub(crate) fn append_text_face(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
) -> usize {
    append_text_style(
        vertices,
        font,
        text,
        origin,
        scale,
        viewport,
        face,
        [0.964, 0.978, 0.991, 1.0],
        0.0,
    )
}

/// Append text with an explicit weight, base colour, and tracking. A glyph that
/// would come within one line height of the viewport's right edge starts a new
/// line, as does `\n`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_text_style(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
    base_color: [f32; 4],
    letter_spacing: f32,
) -> usize {
    append_text_wrapping(
        vertices,
        font,
        text,
        origin,
        scale,
        viewport,
        face,
        base_color,
        letter_spacing,
        true,
    )
}

/// [`append_text_style`] without the wrap at the viewport's right edge: only `\n`
/// starts a new line. For text placed where it must stay, such as the HUD's,
/// whose callers keep it on screen themselves.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_text_unwrapped(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
    base_color: [f32; 4],
    letter_spacing: f32,
) -> usize {
    append_text_wrapping(
        vertices,
        font,
        text,
        origin,
        scale,
        viewport,
        face,
        base_color,
        letter_spacing,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn append_text_wrapping(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
    base_color: [f32; 4],
    letter_spacing: f32,
    edge_wrap: bool,
) -> usize {
    let mut cursor = origin;
    let mut color = base_color;
    let mut lines = 1;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'^' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            color = quake_color(bytes[index + 1] - b'0');
            index += 2;
            continue;
        }
        let (key, step) = glyph_at(text, index);
        index += step;
        let newline = key == GlyphKey::Slot(b'\n');
        if newline || (edge_wrap && cursor[0] + font.height * scale > viewport[0]) {
            cursor[0] = origin[0];
            cursor[1] += font.height * scale * 1.18;
            lines += 1;
            if newline {
                continue;
            }
        }
        let glyph = font.glyph_of(face, key);
        let rectangle = [
            cursor[0] + glyph.offset_x * scale,
            cursor[1] + glyph.offset_y * scale,
            glyph.width * scale,
            glyph.height * scale,
        ];
        let mut shadow = rectangle;
        shadow[0] += scale.max(1.0);
        shadow[1] += scale.max(1.0);
        // The shadow fades with the glyph so opacity groups hide text fully.
        push_quad(
            vertices,
            shadow,
            glyph.uv,
            [0.0, 0.0, 0.0, 0.55 * color[3]],
            viewport,
        );
        push_quad(vertices, rectangle, glyph.uv, color, viewport);
        cursor[0] += glyph.advance * scale + letter_spacing;
    }
    lines
}

/// The character the modern atlas draws in slot `byte`: the Windows-1252 character of
/// that byte, or `.` where Inter has no glyph for it (a control byte such as the 0x0B
/// some players put in names, or one of Windows-1252's five unassigned bytes).
/// OpenJK's `RE_Font_DrawString` draws `.` for every glyph its font lacks, so retail
/// and EternalJK show those names with dots; line feed, carriage return and space keep
/// their own (empty) slots.
fn slot_character(font: &Font, byte: u8) -> char {
    slot_glyph(font, None, byte).1
}

/// The font and character slot `byte` is drawn from: `font`'s own glyph, else
/// `fallback`'s (a family lending what its font lacks), else `font`'s `.` as in
/// [`slot_character`].
fn slot_glyph<'a>(font: &'a Font, fallback: Option<&'a Font>, byte: u8) -> (&'a Font, char) {
    let character = sjk_protocol::windows_1252_char(byte);
    if matches!(byte, b'\n' | b'\r' | b' ') || font.lookup_glyph_index(character) != 0 {
        return (font, character);
    }
    match fallback.filter(|fallback| fallback.lookup_glyph_index(character) != 0) {
        Some(fallback) => (fallback, character),
        None => (font, '.'),
    }
}

/// Whether `character` prints nothing: the joiners, variation selectors and tags
/// emoji sequences are made of, and the other zero-width marks. Inside an emoji they
/// are part of its glyph; on their own (a stray U+FE0F, a joiner between letters)
/// they take no room instead of drawing `?`.
fn invisible(character: char) -> bool {
    matches!(
        u32::from(character),
        0x200B..=0x200F | 0x2060..=0x2064 | 0xFE00..=0xFE0F | 0xE0020..=0xE007F
    )
}

/// Which glyph draws a character: a slot of the font's own atlas, or a wide one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GlyphKey {
    /// The code-page slot of this Windows-1252 byte.
    Slot(u8),
    /// A character with no Windows-1252 byte, from the wide atlas ([`wide`]).
    Wide(char),
    /// An emoji of the pack, one or more code points drawn as one picture ([`emoji`]).
    Emoji(u16),
}

/// Take the glyph that draws the character at `index`, and its UTF-8 length.
///
/// The atlases hold 256 glyphs indexed by Windows-1252 byte, matching how JKA's own fonts are
/// laid out. Text reaching the renderer is a Rust `str`, so any character above ASCII occupies
/// several UTF-8 bytes, and indexing the atlas with those bytes drew one character as two
/// glyphs: `ñ` became `Ã±`, and U+FFFD became `ï¿½` — the trailing `½` players kept seeing in
/// names. A character is drawn with the glyph of its Windows-1252 byte, so `€` typed here and
/// byte 0x80 from another client (decoded as U+0080) both draw slot 0x80. An emoji, which
/// may be several characters (`👍🏽`, a flag), is one glyph from the emoji atlas
/// ([`emoji::match_at`]); any other character (`ж`, `日`) is wide and comes from the wide
/// atlas.
///
/// `index` must be a character boundary, which holds because every caller advances by whole
/// characters or by an ASCII colour escape.
pub(crate) fn glyph_at(text: &str, index: usize) -> (GlyphKey, usize) {
    let rest = &text[index..];
    if !rest.as_bytes().first().is_some_and(u8::is_ascii)
        || rest.as_bytes().get(1).is_some_and(|byte| *byte >= 0x80)
    {
        if let Some((id, length)) = emoji::match_at(rest) {
            return (GlyphKey::Emoji(id), length);
        }
    }
    match rest.chars().next() {
        Some(character) => (
            sjk_protocol::windows_1252_byte(character)
                .map_or(GlyphKey::Wide(character), GlyphKey::Slot),
            character.len_utf8(),
        ),
        None => (GlyphKey::Slot(b'?'), 1),
    }
}

/// Measure visible text while ignoring Quake color escapes.
pub(crate) fn visible_text_width(font: &UiFont, text: &str, scale: f32) -> f32 {
    visible_text_width_face(font, text, scale, TextFace::Regular)
}

/// Measure one face without producing vertices.
pub(crate) fn visible_text_width_face(
    font: &UiFont,
    text: &str,
    scale: f32,
    face: TextFace,
) -> f32 {
    visible_text_width_style(font, text, scale, face, 0.0)
}

/// Measure one face with additional tracking between glyph advances.
pub(crate) fn visible_text_width_style(
    font: &UiFont,
    text: &str,
    scale: f32,
    face: TextFace,
    letter_spacing: f32,
) -> f32 {
    let bytes = text.as_bytes();
    let mut width = 0.0;
    let mut maximum = 0.0_f32;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'^' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            index += 2;
        } else if bytes[index] == b'\n' {
            maximum = maximum.max(width);
            width = 0.0;
            index += 1;
        } else {
            let (key, step) = glyph_at(text, index);
            width += font.glyph_of(face, key).advance * scale + letter_spacing;
            index += step;
        }
    }
    maximum.max(width)
}

/// A font whose every glyph advances 8 units at height 12, for tests.
#[cfg(test)]
pub(crate) fn test_font() -> UiFont {
    let mut glyphs = [[FontGlyph::default(); GLYPH_COUNT]; 2];
    for face in &mut glyphs {
        for glyph in face.iter_mut() {
            *glyph = FontGlyph {
                width: 8.0,
                height: 10.0,
                advance: 8.0,
                ..FontGlyph::default()
            };
        }
    }
    UiFont {
        glyphs,
        wide: wide::NONE,
        height: 12.0,
        modern: true,
        style: TextStyle::NEUTRAL,
    }
}

/// The last colour code in `text` (`^3`), which colours whatever follows it.
pub(crate) fn last_colour(text: &str) -> Option<&str> {
    text.char_indices()
        .rev()
        .find(|&(at, character)| {
            character == '^' && text.as_bytes().get(at + 1).is_some_and(u8::is_ascii_digit)
        })
        .map(|(at, _)| &text[at..at + 2])
}

/// Each visible character of `text` drawn from its base colour, with the digit of
/// the colour code in force at it (`None`: the base colour), for tests of rows that
/// must go on in the colour the row before them ended in.
#[cfg(test)]
pub(crate) fn code_per_char(text: &str) -> Vec<(char, Option<u8>)> {
    let mut drawn = Vec::new();
    let mut code = None;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '^' && chars.peek().is_some_and(char::is_ascii_digit) {
            code = chars
                .next()
                .and_then(|digit| digit.to_digit(10))
                .map(|digit| digit as u8);
        } else {
            drawn.push((character, code));
        }
    }
    drawn
}

/// The colour code in force where a row of wrapped text starts: what a row drawn
/// on its own would otherwise lose. Text that is cut into rows and drawn row by
/// row restarts in the base colour at every row, so each row after the first is
/// written with its `Carry` in front (it prints as `^<digit>`, or nothing when no
/// code came before), as if the whole text were drawn as one line. A code takes
/// no room, so the prefix changes no width and no wrap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Carry(Option<u8>);

impl Carry {
    /// No colour code in force: the row draws in its base colour.
    pub(crate) const NONE: Self = Self(None);

    /// The colour in force after `text`, which starts in `self`'s: the last code
    /// in `text`, else the one it started in.
    pub(crate) fn after(self, text: &str) -> Self {
        match last_colour(text) {
            Some(code) => Self(Some(code.as_bytes()[1] - b'0')),
            None => self,
        }
    }
}

impl std::fmt::Display for Carry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(digit) => write!(formatter, "^{digit}"),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_wrapping_variant_breaks_at_the_right_edge() {
        let font = test_font();
        let draw = |wrap: bool| {
            let mut vertices = Vec::new();
            let append = if wrap {
                append_text_style
            } else {
                append_text_unwrapped
            };
            let lines = append(
                &mut vertices,
                &font,
                "MEDIUM\nX",
                [600.0, 0.0],
                1.0,
                [640.0, 480.0],
                TextFace::Regular,
                [1.0; 4],
                0.0,
            );
            (lines, vertices.len())
        };
        let (wrapped, wrapped_vertices) = draw(true);
        let (unwrapped, unwrapped_vertices) = draw(false);
        assert!(wrapped > 2, "{wrapped}");
        assert_eq!(unwrapped, 2);
        assert_eq!(wrapped_vertices, unwrapped_vertices);
    }

    #[test]
    fn the_legible_palette_lifts_dark_codes_and_keeps_their_hue() {
        let luminance = |[r, g, b, _]: [f32; 4]| 0.2126 * r + 0.7152 * g + 0.0722 * b;
        for index in 0..=9 {
            let game = CodePalette::Game.colour(index);
            let legible = CodePalette::Legible.colour(index);
            assert_eq!(game, quake_color(index));
            // Every code reads on the navy ground (luminance 0.004).
            assert!(luminance(legible) >= 0.35, "^{index}: {legible:?}");
            assert!(luminance(legible) >= luminance(game) - 1e-6, "^{index}");
            // The strongest channel stays the strongest: red stays red.
            let strongest = |colour: [f32; 4]| {
                (0..3)
                    .max_by(|&a, &b| colour[a].total_cmp(&colour[b]))
                    .unwrap()
            };
            if game[..3].iter().any(|channel| *channel != game[0]) {
                assert_eq!(strongest(legible), strongest(game), "^{index}");
            }
        }
        assert_eq!(Plain("^1J^7o^1F").to_string(), "JoF");
    }

    /// Glyph bytes of `text`, walking it as the renderer does; a wide character is `?`.
    fn glyph_bytes(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut index = 0;
        while index < text.len() {
            let (key, step) = glyph_at(text, index);
            bytes.push(match key {
                GlyphKey::Slot(byte) => byte,
                GlyphKey::Wide(_) | GlyphKey::Emoji(_) => b'?',
            });
            index += step;
        }
        bytes
    }

    #[test]
    fn the_headline_subset_keeps_every_windows_1252_character() {
        for bytes in HEADLINE.faces {
            let font = Font::from_bytes(bytes, FontSettings::default()).unwrap();
            for byte in (0x21..=0x7e).chain(0x80..=0xff) {
                let character = sjk_protocol::windows_1252_char(byte);
                // Bytes Windows-1252 leaves undefined, the soft hyphen and NBSP aside.
                if character.is_control() || matches!(byte, 0xa0 | 0xad) {
                    continue;
                }
                assert_ne!(font.lookup_glyph_index(character), 0, "{character}");
            }
        }
    }

    #[test]
    fn typed_and_received_symbols_draw_their_windows_1252_glyph() {
        let symbols = "×¥’¡²³‘€½¼©ñæ§®™£°µ·«»ÄäÖöÜüßÆØøÑ…";
        let expected: Vec<u8> = symbols
            .chars()
            .map(|character| sjk_protocol::windows_1252_byte(character).unwrap())
            .collect();
        // Typed here: Unicode characters.
        assert_eq!(glyph_bytes(symbols), expected);
        // Received from a legacy client: the same bytes, decoded as Latin-1.
        let received: String = expected.iter().map(|&byte| char::from(byte)).collect();
        assert_eq!(glyph_bytes(&received), expected);
        assert_eq!(glyph_at("α", 0), (GlyphKey::Wide('α'), 2));
        // A pictograph is an emoji even without U+FE0F after it.
        assert!(matches!(glyph_at("♥", 0), (GlyphKey::Emoji(_), 3)));
        assert_eq!(glyph_at("жa", 0), (GlyphKey::Wide('ж'), 2));
        // An emoji sequence is one glyph; ASCII before it stays a slot.
        let (key, step) = glyph_at("a👍🏽", 1);
        assert!(
            matches!(key, GlyphKey::Emoji(_)) && step == 8,
            "{key:?} {step}"
        );
        assert_eq!(glyph_at("a👍🏽", 0), (GlyphKey::Slot(b'a'), 1));
        assert_eq!(glyph_at("©", 0), (GlyphKey::Slot(0xA9), 2));
    }

    #[test]
    fn an_emoji_is_an_em_square_on_the_baseline() {
        let font = load_modern(1.0, None).unwrap().font;
        let (key, _) = glyph_at("😀", 0);
        let capital = font.glyph(TextFace::Regular, b'H');
        // Its advance holds before its picture is decoded.
        let pending = font.glyph_of(TextFace::Regular, key);
        while emoji::rasterize_pending(
            std::time::Instant::now() + std::time::Duration::from_secs(1),
        ) {}
        let face = font.glyph_of(TextFace::Regular, key);
        assert_eq!(pending.advance, face.advance);
        assert!(face.uv[0] >= emoji::U_OFFSET, "{face:?}");
        assert!((face.width - capital.height / 0.73).abs() < 0.01);
        assert!(face.advance > face.width);
        let baseline = capital.offset_y + capital.height;
        assert!(face.offset_y + face.height > baseline && face.offset_y < capital.offset_y);
    }

    #[test]
    fn wide_characters_match_the_capitals_and_sit_on_the_baseline() {
        let font = load_modern(1.0, None).unwrap().font;
        let capital = font.glyph(TextFace::Regular, b'H');
        let baseline = capital.offset_y + capital.height;
        let drawn = |character| {
            // The advance is known before the glyph is rasterized.
            let pending = font.glyph_of(TextFace::Regular, GlyphKey::Wide(character));
            while wide::rasterize_pending(std::time::Duration::from_secs(1)) {}
            let glyph = font.glyph_of(TextFace::Regular, GlyphKey::Wide(character));
            assert_eq!(pending.advance, glyph.advance);
            glyph
        };
        // `Ж` is a capital of Inter's own Cyrillic: as tall as `H`, on its baseline.
        let zhe = drawn('Ж');
        assert!(zhe.advance > capital.advance * 0.8, "{zhe:?}");
        assert!(
            (zhe.height - capital.height).abs() < 0.5,
            "{zhe:?} {capital:?}"
        );
        assert!((zhe.offset_y + zhe.height - baseline).abs() < 0.5);
        assert!(zhe.uv[0] >= wide::U_OFFSET);
        // A kanji is wider and taller than a capital.
        let kanji = drawn('漢');
        assert!(
            kanji.width > capital.width && kanji.height > capital.height,
            "{kanji:?}"
        );
        // A stray joiner or selector takes no room.
        let joiner = font.glyph_of(TextFace::Regular, GlyphKey::Wide('\u{200D}'));
        assert_eq!((joiner.advance, joiner.width), (0.0, 0.0));
        // No font has it: the face's `?`.
        let missing = font.glyph_of(TextFace::Regular, GlyphKey::Wide('😀'));
        assert_eq!(missing.uv, font.glyph(TextFace::Regular, b'?').uv);
    }

    #[test]
    fn slots_inter_lacks_draw_a_dot_as_retail_does() {
        let font = Font::from_bytes(INTER_REGULAR, FontSettings::default()).unwrap();
        assert_eq!(slot_character(&font, 0x0b), '.');
        assert_eq!(slot_character(&font, 0x81), '.');
        assert_eq!(slot_character(&font, b' '), ' ');
        assert_eq!(slot_character(&font, b'\n'), '\n');
        for (byte, character) in [(0x80, '€'), (0x85, '…'), (0x92, '’'), (0xd7, '×')] {
            assert_eq!(slot_character(&font, byte), character);
        }
    }

    #[test]
    fn a_carry_writes_the_code_a_row_starts_in() {
        let none = Carry::NONE;
        assert_eq!(none.to_string(), "");
        // The last code of a row is the next one's start; a row with none keeps it.
        let green = none.after("^1red ^2green");
        assert_eq!(green.to_string(), "^2");
        assert_eq!(green.after("still green").to_string(), "^2");
        assert_eq!(green.after("then ^0black").to_string(), "^0");
        // A caret with no digit after it is no code, at the end of a row or not.
        assert_eq!(green.after("end^"), green);
        assert_eq!(none.after("a ^ b ^x"), none);
        assert_eq!(none.after("a^^5b").to_string(), "^5");
        assert_eq!(
            code_per_char("a^1b^2^3c"),
            [('a', None), ('b', Some(1)), ('c', Some(3))]
        );
    }

    #[test]
    fn colour_codes_eight_and_nine_are_orange_and_grey() {
        assert_eq!(quake_color(8), [1.0, 0.5, 0.0, 1.0]);
        assert_eq!(quake_color(9), [0.5, 0.5, 0.5, 1.0]);
        // No longer the retail `& 7` wrap onto black and red.
        assert_ne!(quake_color(8), quake_color(0));
        assert_ne!(quake_color(9), quake_color(1));
    }

    #[test]
    fn colour_codes_zero_to_seven_match_the_retail_table() {
        // OpenJK `g_color_table`, shared/qcommon/q_color.c.
        let retail = [
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ];
        for (index, colour) in retail.iter().enumerate() {
            assert_eq!(quake_color(index as u8), *colour, "^{index}");
        }
    }

    #[test]
    fn drawn_glyphs_take_the_extended_code_colours() {
        let font = test_font();
        let mut vertices = Vec::new();
        append_text(
            &mut vertices,
            &font,
            "^8a^9b",
            [0.0, 0.0],
            1.0,
            [640.0, 480.0],
        );
        // Each glyph emits a shadow quad then the glyph quad, six vertices each.
        assert_eq!(vertices.len(), 24);
        assert_eq!(vertices[6].color, [1.0, 0.5, 0.0, 1.0]);
        assert_eq!(vertices[18].color, [0.5, 0.5, 0.5, 1.0]);
    }
}

#[cfg(test)]
mod centre_tests {
    use super::*;

    #[test]
    fn glyphs_are_centred_by_their_ink() {
        let viewport = [200.0, 100.0];
        let mut vertices = Vec::new();
        // A glyph high and left in its line box: 10..30 across, 5..15 down.
        push_quad(
            &mut vertices,
            [10.0, 5.0, 20.0, 10.0],
            [0.0; 4],
            [1.0; 4],
            viewport,
        );
        centre_vertices(&mut vertices, [100.0, 50.0], viewport);
        let xs = vertices.iter().map(|v| v.position[0]);
        let ys = vertices.iter().map(|v| v.position[1]);
        let (lx, hx) = xs.fold((f32::MAX, f32::MIN), |(l, h), x| (l.min(x), h.max(x)));
        let (ly, hy) = ys.fold((f32::MAX, f32::MIN), |(l, h), y| (l.min(y), h.max(y)));
        // The screen's middle is the origin of device coordinates.
        assert!(((lx + hx) * 0.5).abs() < 1e-5 && ((ly + hy) * 0.5).abs() < 1e-5);
        // Its size is kept: 20 of 200 pixels across is 0.2 of the device's 2.
        assert!((hx - lx - 0.2).abs() < 1e-5);
        centre_vertices(&mut [], [0.0; 2], viewport);
    }
}
