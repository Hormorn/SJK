//! Viewer-side GPU/text adapter for renderer-neutral UI draw commands.

use crate::text::{self, TextVertex, UiFont};
use bytemuck::{Pod, Zeroable};
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId};

pub(crate) mod art;
mod card_art;
pub(crate) mod duel_badge;
mod emblem;
mod gif_textures;
mod icons;
mod levelshot;
pub(crate) mod map_edit_pictures;
mod medal_art;
mod tick_badge;
mod trust_badge;
mod verified_badge;
use art::{ArtTextures, Run, Source};
use emblem::EmblemTextures;
use gif_textures::GifTextures;
use icons::IconAtlas;
pub(crate) use icons::{
    ATLAS_CELLS, BIND_ICON_CELLS, BIND_ICON_FIRST, CROSSHAIR_ICON_CELLS, CROSSHAIR_ICON_FIRST,
    EMOJI_ICON_CELLS, EMOJI_ICON_FIRST, FORCE_ICON_CELLS, FORCE_ICON_FIRST, FORCE_WHEEL_ICON_CELLS,
    FORCE_WHEEL_ICON_FIRST, HOLOCRON_ICON_FIRST, ICON_CELLS, ICON_SIZE, LOGO_ICON,
    MOVEMENT_KEY_ICON_CELLS, MOVEMENT_KEY_ICON_FIRST, PART_ICON_CELLS, PART_ICON_FIRST,
    ROCKET_LOCK_ICON_FIRST, SCOREBOARD_ICON_CELLS, VEHICLE_CROSSHAIR_ICON,
};
pub(crate) use levelshot::LEVELSHOT_TEXTURE;
use map_edit_pictures::MapEditPictures;
use medal_art::MedalTextures;

/// `TexturedQuad` texture naming the classic profile's model preview
/// (`menu_stage::preview`), bound by [`ShapeRenderer::set_preview`].
pub(crate) const PREVIEW_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(u32::MAX - 2);
/// `TexturedQuad` texture naming the settings' HUD picker preview, uploaded by
/// [`ShapeRenderer::upload_hud_preview`].
pub(crate) const HUD_PREVIEW_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(u32::MAX - 3);
/// `TexturedQuad` texture naming the Collection's live blade pictures
/// ([`crate::blade_pictures`]), bound by [`ShapeRenderer::set_blade_pictures`] and added
/// over what is below (the blades' light, in display values).
pub(crate) const BLADE_PICTURES_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(u32::MAX - 4);
/// Pictures the News panel shows at once, each its own texture ([`news_image_texture`]).
pub(crate) const NEWS_IMAGE_SLOTS: usize = 3;
/// The first News picture's texture id; the others follow it.
const NEWS_IMAGE_FIRST: u32 = u32::MAX - 4 - NEWS_IMAGE_SLOTS as u32;

/// `TexturedQuad` texture naming News picture slot `slot` ([`crate::news_images`]),
/// uploaded by [`ShapeRenderer::upload_news_image`].
pub(crate) fn news_image_texture(slot: usize) -> sjk_ui::TextureId {
    sjk_ui::TextureId(NEWS_IMAGE_FIRST + slot.min(NEWS_IMAGE_SLOTS - 1) as u32)
}

/// The News picture slot `texture` names.
fn news_slot_of(texture: sjk_ui::TextureId) -> Option<u8> {
    (NEWS_IMAGE_FIRST..NEWS_IMAGE_FIRST + NEWS_IMAGE_SLOTS as u32)
        .contains(&texture.0)
        .then(|| (texture.0 - NEWS_IMAGE_FIRST) as u8)
}
/// `TexturedQuad` texture naming SJK's emblem, uploaded once at start.
pub(crate) const LOGO_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(LOGO_ICON);
/// `TexturedQuad` texture naming JoF EternalJK's icon, which SJK chat messages sent from
/// it start with in place of SJK's emblem, uploaded once at start.
pub(crate) const EJK_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(icons::EJK_ICON);
/// `TexturedQuad` texture naming the SJK Launcher's icon, which SJK chat messages sent from
/// its chat window start with in place of SJK's emblem, uploaded once at start.
pub(crate) const LAUNCHER_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(icons::LAUNCHER_ICON);
/// `TexturedQuad` texture naming the gold verified badge, drawn once at start.
pub(crate) const VERIFIED_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(icons::VERIFIED_ICON);
/// `TexturedQuad` texture naming the trust mark for `trust` (`trust_badge`), drawn once
/// at start; `None` for a level this build does not know.
pub(crate) fn trust_texture(trust: sjk_identity::Trust) -> Option<sjk_ui::TextureId> {
    use sjk_identity::Trust;
    let cell = match trust {
        Trust::New => 0,
        Trust::Trusted => 1,
        Trust::Banned => 2,
        Trust::Unknown => return None,
    };
    Some(sjk_ui::TextureId(icons::TRUST_ICON_FIRST + cell))
}

/// `TexturedQuad` texture naming tick `tick` (`tick_badge`, the gold seal for `Gold`),
/// drawn once at start; `None` for no tick. The creator's sun names its first frame,
/// which the renderer steps through as time goes ([`animated`]).
pub(crate) fn tick_texture(tick: sjk_identity::Tick) -> Option<sjk_ui::TextureId> {
    use sjk_identity::Tick;
    let cell = match tick {
        Tick::None => return None,
        Tick::Gold | Tick::Other => return Some(VERIFIED_TEXTURE),
        Tick::Jof => 0,
        Tick::Sith => 1,
        Tick::Jawa => 2,
        Tick::Jkr => 3,
        Tick::Staff => 4,
        Tick::Creator => return Some(sjk_ui::TextureId(icons::CREATOR_TICK_FIRST)),
    };
    Some(sjk_ui::TextureId(icons::TICK_ICON_FIRST + cell))
}

/// The colour of a word about tick `tick` (`tick_badge::accent`), at alpha `a`.
pub(crate) fn tick_accent(tick: sjk_identity::Tick, a: f32) -> Option<sjk_ui::Color> {
    tick_badge::accent(tick).map(|[r, g, b]| sjk_ui::Color::new(r, g, b, a))
}

/// The cell to draw for `texture` at `now` seconds: the creator's sun's frame then, any
/// other texture itself.
fn animated(texture: sjk_ui::TextureId, now: f64) -> sjk_ui::TextureId {
    if texture.0 != icons::CREATOR_TICK_FIRST {
        return texture;
    }
    let frame =
        (now.max(0.0) * tick_badge::CREATOR_FPS) as u64 % u64::from(tick_badge::CREATOR_FRAMES);
    sjk_ui::TextureId(icons::CREATOR_TICK_FIRST + frame as u32)
}

/// The mark after an SJK player's name: their tick (`tick`, the hub's, as
/// [`sjk_identity::Tick::shown`] reads it with `verified`), else their trust mark
/// (`trust`, the hub's; none from hubs before it or for staff).
pub(crate) fn mark_texture(
    verified: bool,
    tick: sjk_identity::Tick,
    trust: Option<sjk_identity::Trust>,
) -> Option<sjk_ui::TextureId> {
    tick_texture(tick.shown(verified)).or_else(|| trust.and_then(trust_texture))
}

/// `TexturedQuad` texture naming the duel badge (crossed sabers) in duel colour
/// `index` (`duel_badge::COLOURS`, wrapping), drawn once at start.
pub(crate) fn duel_texture(index: usize) -> sjk_ui::TextureId {
    sjk_ui::TextureId(icons::DUEL_ICON_FIRST + (index % icons::DUEL_ICON_CELLS as usize) as u32)
}
/// Atlas cells reserved for the clan emblems (`jof_tag::KNOWN`).
pub(crate) const CLAN_ICON_CELLS: usize = icons::CLAN_ICON_CELLS as usize;

/// `TexturedQuad` texture naming the emblem of `jof_tag::KNOWN[index]`, uploaded at start.
pub(crate) fn clan_texture(index: usize) -> sjk_ui::TextureId {
    sjk_ui::TextureId(icons::CLAN_ICON_FIRST + index.min(CLAN_ICON_CELLS - 1) as u32)
}

/// Atlas cells reserved for the quick wheels' icons.
pub(crate) const WHEEL_ICON_CELLS: usize = icons::WHEEL_ICON_CELLS as usize;

/// `TexturedQuad` texture naming quick wheel icon `index` (`quick_wheel::ICONS`).
pub(crate) fn wheel_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < WHEEL_ICON_CELLS);
    sjk_ui::TextureId(icons::WHEEL_ICON_FIRST + index as u32)
}

/// Atlas cells reserved for the emotes' pictures (`emote_icons`).
pub(crate) const EMOTE_ICON_CELLS: usize = icons::EMOTE_ICON_CELLS as usize;

/// `TexturedQuad` texture naming emote picture cell `index` (`emote_icons`).
pub(crate) fn emote_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < EMOTE_ICON_CELLS);
    sjk_ui::TextureId(icons::EMOTE_ICON_FIRST + index as u32)
}

/// Atlas cells reserved for the quick wheel's own pictures (`quick_wheel::pictures`).
pub(crate) const CUSTOM_WHEEL_ICON_CELLS: usize = icons::CUSTOM_WHEEL_ICON_CELLS as usize;

/// `TexturedQuad` texture naming own wheel picture cell `index` (`quick_wheel::pictures`).
pub(crate) fn custom_wheel_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < CUSTOM_WHEEL_ICON_CELLS);
    sjk_ui::TextureId(icons::CUSTOM_WHEEL_ICON_FIRST + index as u32)
}

/// Atlas cells reserved for the tags' pictures (`tag_icons`).
pub(crate) const TAG_ICON_CELLS: usize = icons::TAG_ICON_CELLS as usize;

/// `TexturedQuad` texture naming tag picture cell `index` (`tag_icons`).
pub(crate) fn tag_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < TAG_ICON_CELLS);
    sjk_ui::TextureId(icons::TAG_ICON_FIRST + index as u32)
}

/// Atlas cells reserved for the settings menu's icons.
pub(crate) const SETTINGS_ICON_CELLS: usize = icons::SETTINGS_ICON_CELLS as usize;

/// `TexturedQuad` texture naming settings icon `index` (`settings_icons::ICONS`).
pub(crate) fn settings_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < SETTINGS_ICON_CELLS);
    sjk_ui::TextureId(icons::SETTINGS_ICON_FIRST + index as u32)
}

/// Atlas cells reserved for the medals' small medallions.
pub(crate) const MEDAL_ICON_CELLS: usize = icons::MEDAL_ICON_CELLS as usize;

/// `TexturedQuad` texture naming the small medallion of medal `index`
/// (`medals::Medal::icon`).
pub(crate) fn medal_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < MEDAL_ICON_CELLS);
    sjk_ui::TextureId(icons::MEDAL_ICON_FIRST + index as u32)
}

/// Atlas cells reserved for players' pictures (`avatars`).
pub(crate) const AVATAR_ICON_CELLS: usize = icons::AVATAR_ICON_CELLS as usize;

/// `TexturedQuad` texture naming picture cell `index` (`avatars`).
pub(crate) fn avatar_icon(index: usize) -> sjk_ui::TextureId {
    debug_assert!(index < AVATAR_ICON_CELLS);
    sjk_ui::TextureId(icons::AVATAR_ICON_FIRST + index as u32)
}

/// The verified badge's pixels in one cell, as the renderer uploads them, for the
/// off-screen snapshots.
#[cfg(test)]
pub(crate) fn verified_badge_pixels() -> Vec<u8> {
    verified_badge::pixels(icons::ICON_SIZE)
}
use levelshot::LevelshotTexture;

/// SJK's emblem, scaled into one icon cell at start.
const SJK_EMBLEM: &[u8] = include_bytes!("../../../assets/branding/sjk-logo-512.png");
/// JoF EternalJK's icon, scaled into one icon cell at start.
const EJK_ICON: &[u8] = include_bytes!("../../../assets/branding/jof-ejk-icon.png");
/// The SJK Launcher's icon, scaled into one icon cell at start.
const LAUNCHER_ICON: &[u8] = include_bytes!("../../../assets/branding/sjk-launcher-icon.png");

/// Shape vertices one frame may draw (six per quad), across every layer: the Collection alone
/// may fill its [`crate::console::collection_panel::DRAW_COMMANDS`] (#117: its Forge reached
/// about 11 600 vertices with its swatches drawn, under the 16 384 of before by little).
const MAX_SHAPE_VERTICES: usize = 32_768;

/// `parameters.y` value that selects the arc shader path (`ui_shapes.wgsl`).
const ARC_MODE: f32 = 3.0;

/// `parameters.y` value that selects the line shader path (`ui_shapes.wgsl`).
const LINE_MODE: f32 = 4.0;

/// `parameters.y` value that selects the strafe aura shader path (`ui_shapes.wgsl`).
const AURA_MODE: f32 = 5.0;

/// `parameters.y` value that selects the card finish shader path (`ui_shapes.wgsl`).
const SHINE_MODE: f32 = 6.0;

/// `parameters.y` value that selects the convex polygon shader path (`ui_shapes.wgsl`).
const POLYGON_MODE: f32 = 7.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct ShapeVertex {
    position: [f32; 2],
    local: [f32; 2],
    size: [f32; 2],
    start_color: [f32; 4],
    end_color: [f32; 4],
    parameters: [f32; 2],
    uv: [f32; 2],
}

impl ShapeVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x2,
        6 => Float32x2
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Colour adds as light (`src * alpha + dst`) and the target's alpha is
/// kept: the menu emblem's glow layers, whose black adds nothing.
const ADDITIVE_BLENDING: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Texture switches (bind-group runs) the renderer makes for `lists` with every
/// texture loaded, as [`ShapeRenderer::prepare_layers`] counts them against
/// [`art::MAX_RUNS`]; for screens' tests.
#[cfg(test)]
pub(crate) fn texture_switches(lists: &[&DrawList]) -> usize {
    let mut runs = vec![Run {
        start: 0,
        source: Source::Atlas,
    }];
    let mut vertices = 0;
    let commands = lists
        .iter()
        .flat_map(|list| std::iter::once(None).chain(list.commands().iter().map(Some)));
    for command in commands {
        let Some(command) = command else {
            art::begin_layer(&mut runs, vertices);
            continue;
        };
        let texture = match command {
            DrawCommand::TexturedQuad { texture, .. }
            | DrawCommand::TexturedQuadUv { texture, .. }
            | DrawCommand::TexturedQuadTurned { texture, .. } => *texture,
            // A shape after the emblem's light goes back to alpha blending.
            _ => {
                art::untextured(&mut runs, vertices);
                vertices += 6;
                continue;
            }
        };
        let source = match crate::menu::art::ArtPiece::from_texture(texture) {
            Some(piece) => Source::Art(piece),
            None if texture == LEVELSHOT_TEXTURE => Source::Levelshot,
            None if texture == HUD_PREVIEW_TEXTURE => Source::HudPreview,
            None if news_slot_of(texture).is_some() => {
                Source::News(news_slot_of(texture).unwrap_or_default())
            }
            None if texture == PREVIEW_TEXTURE => Source::Preview,
            None if texture == BLADE_PICTURES_TEXTURE => Source::BladePictures,
            None if map_edit_pictures::cell_of(texture).is_some() => Source::MapEditCells,
            None if texture == map_edit_pictures::PREVIEW => Source::MapEditPreview,
            None if crate::chat_gifs::slot_of(texture).is_some() => {
                Source::Gif(crate::chat_gifs::slot_of(texture).unwrap_or_default() as u8)
            }
            None if crate::cards::faces::index_of(texture).is_some() => {
                Source::Card(crate::cards::faces::index_of(texture).unwrap_or_default() as u16)
            }
            None => match crate::medals::Medal::from_art(texture) {
                Some(medal) => Source::Medal(medal),
                None => crate::menu::emblem::EmblemLayer::from_texture(texture)
                    .map_or(Source::Atlas, Source::Emblem),
            },
        };
        if runs.len() < art::MAX_RUNS {
            art::switch(&mut runs, vertices, source);
        } else if runs.last().is_some_and(|run| run.source != source) {
            runs.push(Run {
                start: vertices as u32,
                source,
            });
        }
        vertices += 6;
    }
    runs.len()
}

/// Fixed-capacity WGPU shape renderer. GPU ownership never leaks into `sjk-ui`.
pub(crate) struct ShapeRenderer {
    pipeline: wgpu::RenderPipeline,
    /// The same shapes blended additively ([`ADDITIVE_BLENDING`]), for the
    /// runs of the menu emblem's glow layers.
    additive_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertices: Vec<ShapeVertex>,
    icons: IconAtlas,
    /// Bind-group runs of `vertices`, in draw order.
    runs: Vec<Run>,
    /// Layout the icon atlas and the classic menu art are bound with.
    texture_layout: wgpu::BindGroupLayout,
    /// The player's retail menu artwork, one texture per piece.
    art: ArtTextures,
    /// SJK's menu emblem, one texture per layer.
    emblem: EmblemTextures,
    /// The medals' whole pictures, one texture each, once a screen drew one.
    medals: MedalTextures,
    /// The card faces' pictures, one texture each, as they decode.
    cards: card_art::CardTextures,
    /// The SJK chat's GIFs on screen, one texture each.
    gifs: GifTextures,
    /// The current map preview at its own resolution.
    levelshot: LevelshotTexture,
    /// The HUD picker's preview of the highlighted HUD.
    hud_preview: LevelshotTexture,
    /// The News panel's pictures on show.
    news: Vec<LevelshotTexture>,
    /// The model preview's display texture, once one exists.
    preview: Option<wgpu::BindGroup>,
    blade_pictures: Option<wgpu::BindGroup>,
    /// The map editor's shader pictures, while its panel shows them.
    map_edit: MapEditPictures,
    preview_sampler: wgpu::Sampler,
    /// Whether a frame past the vertex or run storage has been logged: shapes
    /// past it are dropped, so the log names the cause of a missing picture.
    overflow_logged: bool,
    /// See [`Self::id`].
    id: u64,
}

/// Source of [`ShapeRenderer::id`].
static NEXT_RENDERER_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl ShapeRenderer {
    /// Create the one process-lifetime pipeline and fixed vertex storage.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK retained UI shape shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_shapes.wgsl").into()),
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK retained UI texture layout"),
            entries: &[
                crate::render_helpers::texture_layout_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK retained UI shape pipeline layout"),
            bind_group_layouts: &[Some(&texture_layout)],
            immediate_size: 0,
        });
        let create_pipeline = |label, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(ShapeVertex::layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = create_pipeline(
            "SJK retained UI shape pipeline",
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let additive_pipeline =
            create_pipeline("SJK retained UI additive shape pipeline", ADDITIVE_BLENDING);
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK retained UI shape vertices"),
            size: (MAX_SHAPE_VERTICES * std::mem::size_of::<ShapeVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let icons = IconAtlas::new(device, &texture_layout);
        let levelshot = LevelshotTexture::new(device, &texture_layout);
        let hud_preview = LevelshotTexture::new(device, &texture_layout);
        let news = (0..NEWS_IMAGE_SLOTS)
            .map(|_| LevelshotTexture::new(device, &texture_layout))
            .collect();
        for (name, bytes, texture) in [
            ("SJK emblem", SJK_EMBLEM, LOGO_TEXTURE),
            ("JoF EternalJK icon", EJK_ICON, EJK_TEXTURE),
            ("SJK Launcher icon", LAUNCHER_ICON, LAUNCHER_TEXTURE),
        ] {
            match image::load_from_memory(bytes) {
                Ok(emblem) => {
                    let cell = image::imageops::resize(
                        &emblem.into_rgba8(),
                        icons::ICON_SIZE,
                        icons::ICON_SIZE,
                        image::imageops::FilterType::Lanczos3,
                    );
                    icons.upload(queue, texture, cell.as_raw());
                }
                Err(error) => eprintln!("{name}: {error}"),
            }
        }
        icons.upload(
            queue,
            VERIFIED_TEXTURE,
            &verified_badge::pixels(icons::ICON_SIZE),
        );
        for (cell, tick) in [
            sjk_identity::Tick::Jof,
            sjk_identity::Tick::Sith,
            sjk_identity::Tick::Jawa,
            sjk_identity::Tick::Jkr,
            sjk_identity::Tick::Staff,
        ]
        .into_iter()
        .enumerate()
        {
            icons.upload(
                queue,
                sjk_ui::TextureId(icons::TICK_ICON_FIRST + cell as u32),
                &tick_badge::pixels(icons::ICON_SIZE, tick, 0),
            );
        }
        for frame in 0..tick_badge::CREATOR_FRAMES {
            icons.upload(
                queue,
                sjk_ui::TextureId(icons::CREATOR_TICK_FIRST + frame),
                &tick_badge::pixels(icons::ICON_SIZE, sjk_identity::Tick::Creator, frame),
            );
        }
        for trust in [
            sjk_identity::Trust::New,
            sjk_identity::Trust::Trusted,
            sjk_identity::Trust::Banned,
        ] {
            if let Some(texture) = trust_texture(trust) {
                icons.upload(
                    queue,
                    texture,
                    &trust_badge::pixels(icons::ICON_SIZE, trust),
                );
            }
        }
        for (index, colour) in duel_badge::COLOURS.into_iter().enumerate() {
            icons.upload(
                queue,
                duel_texture(index),
                &duel_badge::pixels(icons::ICON_SIZE, colour),
            );
        }
        for (index, clan) in crate::jof_tag::KNOWN
            .iter()
            .take(CLAN_ICON_CELLS)
            .enumerate()
        {
            match image::load_from_memory(clan.emblem) {
                Ok(emblem) => {
                    icons.upload(queue, clan_texture(index), emblem.into_rgba8().as_raw())
                }
                Err(error) => eprintln!("{} emblem: {error}", clan.name),
            }
        }
        let wheel = crate::quick_wheel::ICONS.iter().enumerate();
        let wheel = wheel.map(|(index, icon)| ("quick wheel", wheel_icon(index), icon));
        let settings = crate::settings_icons::ICONS.iter().enumerate();
        let settings = settings.map(|(index, icon)| ("settings", settings_icon(index), icon));
        let medals = crate::medals::Medal::ALL
            .into_iter()
            .map(|medal| ("medal", medal.icon(), (medal.id(), medal.small_png())));
        let icons_at_start = wheel
            .chain(settings)
            .map(|(set, texture, &(name, bytes))| (set, texture, (name, bytes)))
            .chain(medals);
        for (set, texture, (name, bytes)) in icons_at_start {
            match image::load_from_memory(bytes) {
                Ok(icon) => {
                    let icon = icon.into_rgba8();
                    if icon.dimensions() == (icons::ICON_SIZE, icons::ICON_SIZE) {
                        icons.upload(queue, texture, icon.as_raw());
                    }
                }
                Err(error) => eprintln!("{set} icon {name}: {error}"),
            }
        }
        let mut renderer = Self {
            pipeline,
            additive_pipeline,
            vertex_buffer,
            vertices: Vec::with_capacity(MAX_SHAPE_VERTICES),
            icons,
            runs: Vec::with_capacity(art::MAX_RUNS),
            texture_layout,
            art: ArtTextures::new(),
            emblem: EmblemTextures::new(),
            medals: MedalTextures::new(),
            cards: card_art::CardTextures::new(),
            gifs: GifTextures::new(),
            levelshot,
            hud_preview,
            news,
            preview: None,
            blade_pictures: None,
            map_edit: MapEditPictures::default(),
            preview_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("SJK model preview sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            overflow_logged: false,
            id: NEXT_RENDERER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        // Artwork decoded for an earlier world is uploaded with this one, on
        // the install worker rather than the frame thread.
        renderer.install_menu_art(device, queue);
        renderer
    }

    /// Upload the classic menu artwork once its decode has finished; does
    /// nothing before that or after it has been installed.
    pub(crate) fn install_menu_art(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        if self.art.installed() {
            return;
        }
        if let Some(decoded) = crate::menu::art::decoded() {
            self.art
                .install(device, queue, &self.texture_layout, decoded);
        }
    }

    /// Upload SJK's menu emblem once its decode has finished; does nothing
    /// before that or after it has been installed.
    pub(crate) fn install_emblem(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        if self.emblem.installed() {
            return;
        }
        if let Some(decoded) = crate::menu::emblem::decoded() {
            self.emblem
                .install(device, queue, &self.texture_layout, decoded);
        }
    }

    /// Upload the medals' whole pictures once a screen asked for them and their decode
    /// has finished; does nothing before that or after they have been installed.
    pub(crate) fn install_medals(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        if self.medals.installed() || !crate::medals::art::requested() {
            return;
        }
        if let Some(decoded) = crate::medals::art::decoded() {
            self.medals
                .install(device, queue, &self.texture_layout, decoded);
        }
    }

    /// Upload the card faces' pictures decoded since the last frame.
    pub(crate) fn install_cards(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        self.cards.install(device, queue, &self.texture_layout);
    }

    /// Classic menu art pieces this renderer can draw.
    pub(crate) fn menu_art(&self) -> crate::menu::art::ArtSet {
        self.art.ready()
    }

    /// Translate every active retained layer and upload one shared vertex batch.
    pub(crate) fn prepare_layers<'a>(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        draw_lists: impl IntoIterator<Item = &'a DrawList>,
        viewport: [f32; 2],
    ) {
        self.vertices.clear();
        self.runs.clear();
        self.runs.push(Run {
            start: 0,
            source: Source::Atlas,
        });
        self.art.begin_frame();
        let now = crate::menu::art::motion::seconds();
        let mut opacity = [1.0_f32; 8];
        let mut opacity_depth = 0_usize;
        let mut clips = [Rect::new(0.0, 0.0, viewport[0], viewport[1]); 8];
        // The rounded clip in force at each depth, if any: its rectangle and radius.
        let mut rounds: [Option<(Rect, f32)>; 8] = [None; 8];
        let mut clip_depth = 0_usize;
        // The transform in force at each depth ([`DrawCommand::PushTransform`]), and the
        // first vertex not yet mapped by it.
        let mut transforms: [Option<Transform>; 4] = [None; 4];
        let mut transform_depth = 0_usize;
        let mut mapped = 0_usize;
        let commands = draw_lists.into_iter().flat_map(|list| {
            // Marks where a layer begins; `None` is never a command.
            std::iter::once(None).chain(list.commands().iter().map(Some))
        });
        for command in commands {
            let Some(command) = command else {
                // A new layer starts untransformed.
                map_shapes(
                    &mut self.vertices[mapped..],
                    transforms[transform_depth],
                    viewport,
                );
                mapped = self.vertices.len();
                transform_depth = 0;
                art::begin_layer(&mut self.runs, self.vertices.len());
                continue;
            };
            if matches!(
                command,
                DrawCommand::PushTransform(_) | DrawCommand::PopTransform
            ) {
                // What came before is mapped by the transform it was drawn under.
                map_shapes(
                    &mut self.vertices[mapped..],
                    transforms[transform_depth],
                    viewport,
                );
                mapped = self.vertices.len();
                match *command {
                    DrawCommand::PushTransform(matrix)
                        if transform_depth + 1 < transforms.len() =>
                    {
                        transform_depth += 1;
                        transforms[transform_depth] =
                            Some(Transform::compose(transforms[transform_depth - 1], matrix));
                    }
                    DrawCommand::PopTransform => {
                        transform_depth = transform_depth.saturating_sub(1)
                    }
                    _ => {}
                }
                continue;
            }
            match *command {
                DrawCommand::SolidRect { rect, color }
                | DrawCommand::RoundedRect {
                    rect,
                    radius: 0.0,
                    color,
                } => {
                    self.push_rect(
                        clipped(rect, clips[clip_depth]),
                        color,
                        color,
                        0.0,
                        false,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color,
                } => {
                    self.push_rect(
                        clipped(rect, clips[clip_depth]),
                        color,
                        color,
                        radius,
                        false,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::GradientRect {
                    rect,
                    radius,
                    gradient,
                } => {
                    self.push_rect(
                        clipped(rect, clips[clip_depth]),
                        gradient.start,
                        gradient.end,
                        radius,
                        gradient.vertical,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Border {
                    rect,
                    width,
                    color,
                    radius,
                } if radius > 0.0 => {
                    self.push_ring(
                        clipped(rect, clips[clip_depth]),
                        color,
                        radius,
                        width,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Border {
                    rect, width, color, ..
                } => {
                    for edge in border_rects(rect, width) {
                        self.push_rect(
                            clipped(edge, clips[clip_depth]),
                            color,
                            color,
                            0.0,
                            false,
                            opacity[opacity_depth],
                            viewport,
                        );
                    }
                }
                DrawCommand::PushClip(rect) if clip_depth + 1 < clips.len() => {
                    clip_depth += 1;
                    clips[clip_depth] = clipped(clips[clip_depth - 1], rect);
                    rounds[clip_depth] = rounds[clip_depth - 1];
                }
                DrawCommand::PushRoundedClip { rect, radius } if clip_depth + 1 < clips.len() => {
                    clip_depth += 1;
                    clips[clip_depth] = clipped(clips[clip_depth - 1], rect);
                    rounds[clip_depth] = Some((rect, radius));
                }
                DrawCommand::PopClip => clip_depth = clip_depth.saturating_sub(1),
                DrawCommand::PushOpacity(value) if opacity_depth + 1 < opacity.len() => {
                    opacity_depth += 1;
                    opacity[opacity_depth] = opacity[opacity_depth - 1] * value.clamp(0.0, 1.0);
                }
                DrawCommand::PopOpacity => opacity_depth = opacity_depth.saturating_sub(1),
                DrawCommand::TexturedQuad {
                    rect,
                    texture,
                    color,
                } => {
                    let Some(source) = self.texture_source(queue, texture, now) else {
                        continue;
                    };
                    let uv = match source {
                        Source::Art(_)
                        | Source::Emblem(_)
                        | Source::Medal(_)
                        | Source::Card(_)
                        | Source::Gif(_)
                        | Source::Levelshot
                        | Source::HudPreview
                        | Source::News(_)
                        | Source::BladePictures
                        | Source::MapEditPreview
                        | Source::Preview => ([0.0, 0.0], [1.0, 1.0]),
                        Source::MapEditCells => map_edit_pictures::uv_range(
                            map_edit_pictures::cell_of(texture).unwrap_or_default(),
                        ),
                        Source::Atlas => icons::uv_range(animated(texture, now)),
                    };
                    icons::push_quad(
                        &mut self.vertices,
                        clipped(rect, clips[clip_depth]),
                        uv,
                        color,
                        opacity[opacity_depth],
                        viewport,
                        MAX_SHAPE_VERTICES,
                    );
                }
                DrawCommand::TexturedQuadTurned {
                    rect,
                    texture,
                    color,
                    pivot,
                    degrees,
                } => {
                    let Some(source) = self.texture_source(queue, texture, now) else {
                        continue;
                    };
                    let uv = match source {
                        Source::Atlas => icons::uv_range(animated(texture, now)),
                        Source::MapEditCells => map_edit_pictures::uv_range(
                            map_edit_pictures::cell_of(texture).unwrap_or_default(),
                        ),
                        _ => ([0.0, 0.0], [1.0, 1.0]),
                    };
                    // Like explicit coordinates, a turned quad is not clipped.
                    icons::push_quad_at(
                        &mut self.vertices,
                        icons::turned_corners(rect, pivot, degrees),
                        uv,
                        color,
                        opacity[opacity_depth],
                        viewport,
                        MAX_SHAPE_VERTICES,
                    );
                }
                DrawCommand::TexturedQuadUv {
                    rect,
                    texture,
                    color,
                    uv,
                } => {
                    let Some(source) = self.texture_source(queue, texture, now) else {
                        continue;
                    };
                    // Explicit coordinates are mapped into an atlas cell;
                    // the quad is not clipped, as clipping would need its
                    // coordinates cut to match.
                    let uv = match source {
                        Source::Art(_)
                        | Source::Emblem(_)
                        | Source::Medal(_)
                        | Source::Card(_)
                        | Source::Gif(_)
                        | Source::Levelshot
                        | Source::HudPreview
                        | Source::News(_)
                        | Source::MapEditPreview
                        | Source::Preview => uv,
                        Source::MapEditCells => {
                            let (low, high) = map_edit_pictures::uv_range(
                                map_edit_pictures::cell_of(texture).unwrap_or_default(),
                            );
                            uv.map(|[s, t]| {
                                [
                                    low[0] + s * (high[0] - low[0]),
                                    low[1] + t * (high[1] - low[1]),
                                ]
                            })
                        }
                        // A blade picture is clipped as other shapes are (it scrolls in
                        // the Collection's list): its coordinates are an upright
                        // rectangle, cut with it.
                        Source::BladePictures => {
                            let clip = clipped(rect, clips[clip_depth]);
                            if clip.width <= 0.0 || clip.height <= 0.0 {
                                continue;
                            }
                            let [s0, t0] = uv[0];
                            let [s1, t1] = uv[2];
                            let at = |x: f32, y: f32| {
                                [
                                    s0 + (s1 - s0) * (x - rect.x) / rect.width.max(1e-6),
                                    t0 + (t1 - t0) * (y - rect.y) / rect.height.max(1e-6),
                                ]
                            };
                            let corners = [
                                at(clip.x, clip.y),
                                at(clip.right(), clip.y),
                                at(clip.right(), clip.bottom()),
                                at(clip.x, clip.bottom()),
                            ];
                            icons::push_quad_corners(
                                &mut self.vertices,
                                clip,
                                corners,
                                color,
                                opacity[opacity_depth],
                                viewport,
                                MAX_SHAPE_VERTICES,
                            );
                            continue;
                        }
                        Source::Atlas => {
                            let (low, high) = icons::uv_range(texture);
                            uv.map(|[s, t]| {
                                [
                                    low[0] + s * (high[0] - low[0]),
                                    low[1] + t * (high[1] - low[1]),
                                ]
                            })
                        }
                    };
                    icons::push_quad_corners(
                        &mut self.vertices,
                        rect,
                        uv,
                        color,
                        opacity[opacity_depth],
                        viewport,
                        MAX_SHAPE_VERTICES,
                    );
                }
                DrawCommand::Arc {
                    center,
                    radius,
                    width,
                    start,
                    sweep,
                    color,
                    knockout,
                } => {
                    self.push_arc(
                        center,
                        radius,
                        width,
                        start,
                        sweep,
                        color,
                        knockout,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::StrafeAura { center, aura } => {
                    self.push_aura(center, aura, opacity[opacity_depth], viewport);
                }
                DrawCommand::CardShine { rect, shine } => {
                    self.push_shine(
                        rect,
                        clipped(rect, clips[clip_depth]),
                        shine,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } => self.push_line(
                    from,
                    to,
                    width,
                    color,
                    opacity[opacity_depth],
                    (clips[clip_depth], rounds[clip_depth]),
                    viewport,
                ),
                DrawCommand::Polygon {
                    points,
                    count,
                    color,
                    texture,
                } => {
                    let uv = match texture {
                        None => {
                            art::untextured(&mut self.runs, self.vertices.len());
                            None
                        }
                        Some((texture, [low, high])) => {
                            let Some(source) = self.texture_source(queue, texture, now) else {
                                continue;
                            };
                            Some(match source {
                                Source::Atlas => {
                                    let (cell_low, cell_high) =
                                        icons::uv_range(animated(texture, now));
                                    let at = |[s, t]: [f32; 2]| {
                                        [
                                            cell_low[0] + s * (cell_high[0] - cell_low[0]),
                                            cell_low[1] + t * (cell_high[1] - cell_low[1]),
                                        ]
                                    };
                                    (at(low), at(high))
                                }
                                _ => (low, high),
                            })
                        }
                    };
                    self.push_polygon(
                        &points[..usize::from(count).min(points.len())],
                        color,
                        uv,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Text { .. }
                | DrawCommand::PushClip(_)
                | DrawCommand::PushRoundedClip { .. }
                | DrawCommand::PushOpacity(_)
                | DrawCommand::PushTransform(_)
                | DrawCommand::PopTransform => {}
            }
        }
        map_shapes(
            &mut self.vertices[mapped..],
            transforms[transform_depth],
            viewport,
        );
        if self.vertices.len() + 6 > MAX_SHAPE_VERTICES {
            self.log_overflow("shape vertices");
        }
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    /// Log, once, that a frame ran out of `storage` and dropped shapes.
    fn log_overflow(&mut self, storage: &str) {
        if !std::mem::replace(&mut self.overflow_logged, true) {
            crate::log::progress(format_args!(
                "UI shapes dropped: the frame ran out of {storage}                  (limits {MAX_SHAPE_VERTICES} vertices, {} texture switches)",
                art::MAX_RUNS
            ));
        }
    }

    /// The source a textured quad naming `texture` samples, with its run
    /// begun and an animated art piece stepped to `now`; `None` when it
    /// draws nothing (art that is not loaded, whose screen falls back to
    /// vector shapes, or a full run list).
    fn texture_source(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        texture: sjk_ui::TextureId,
        now: f64,
    ) -> Option<Source> {
        let gif = crate::chat_gifs::slot_of(texture);
        let source = match crate::menu::art::ArtPiece::from_texture(texture) {
            Some(piece) if self.art.ready().has(piece) => {
                self.art.animate(queue, piece, now);
                Source::Art(piece)
            }
            Some(_) => return None,
            None if texture == LEVELSHOT_TEXTURE => Source::Levelshot,
            None if texture == HUD_PREVIEW_TEXTURE => Source::HudPreview,
            None if news_slot_of(texture).is_some() => {
                Source::News(news_slot_of(texture).unwrap_or_default())
            }
            None if texture == PREVIEW_TEXTURE => match self.preview {
                Some(_) => Source::Preview,
                None => return None,
            },
            None if texture == BLADE_PICTURES_TEXTURE => match self.blade_pictures {
                Some(_) => Source::BladePictures,
                None => return None,
            },
            None if map_edit_pictures::cell_of(texture).is_some() => {
                self.map_edit.grid_group()?;
                Source::MapEditCells
            }
            None if texture == map_edit_pictures::PREVIEW => {
                self.map_edit.preview_group()?;
                Source::MapEditPreview
            }
            None if gif.is_some() => {
                let slot = gif.unwrap_or_default();
                self.gifs.group(slot)?;
                Source::Gif(slot as u8)
            }
            None if crate::cards::faces::index_of(texture).is_some() => {
                let index = crate::cards::faces::index_of(texture).unwrap_or_default();
                // Not uploaded yet: asked for, and the window stays dark until it is.
                if !self.cards.draw(index) {
                    return None;
                }
                Source::Card(index as u16)
            }
            None => match crate::menu::emblem::EmblemLayer::from_texture(texture) {
                Some(layer) if self.emblem.group(layer).is_some() => Source::Emblem(layer),
                Some(_) => return None,
                None => match crate::medals::Medal::from_art(texture) {
                    Some(medal) if self.medals.group(medal).is_some() => Source::Medal(medal),
                    // Asked for the first time: decoded on a worker, drawn once uploaded.
                    Some(_) => {
                        crate::medals::art::request();
                        return None;
                    }
                    None => Source::Atlas,
                },
            },
        };
        if art::switch(&mut self.runs, self.vertices.len(), source) {
            Some(source)
        } else {
            self.log_overflow("texture switches");
            None
        }
    }

    /// Draw every retained shape in one pipeline/buffer submission.
    pub(crate) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertices.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        let mut additive = false;
        let total = self.vertices.len() as u32;
        for (index, run) in self.runs.iter().enumerate() {
            let end = self.runs.get(index + 1).map_or(total, |next| next.start);
            if end <= run.start {
                continue;
            }
            if run.source.additive() != additive {
                additive = run.source.additive();
                pass.set_pipeline(if additive {
                    &self.additive_pipeline
                } else {
                    &self.pipeline
                });
            }
            let group = match run.source {
                Source::Art(piece) => self.art.group(piece),
                Source::Emblem(layer) => self.emblem.group(layer),
                Source::Medal(medal) => self.medals.group(medal),
                Source::Card(index) => self.cards.group(usize::from(index)),
                Source::Gif(slot) => self.gifs.group(usize::from(slot)),
                Source::Levelshot => Some(self.levelshot.bind_group()),
                Source::HudPreview => Some(self.hud_preview.bind_group()),
                Source::News(slot) => self
                    .news
                    .get(usize::from(slot))
                    .map(LevelshotTexture::bind_group),
                Source::Preview => self.preview.as_ref(),
                Source::BladePictures => self.blade_pictures.as_ref(),
                Source::MapEditCells => self.map_edit.grid_group(),
                Source::MapEditPreview => self.map_edit.preview_group(),
                Source::Atlas => None,
            };
            pass.set_bind_group(0, group.unwrap_or(self.icons.bind_group()), &[]);
            pass.draw(run.start..end, 0..1);
        }
    }

    fn push_rect(
        &mut self,
        rect: Rect,
        start: Color,
        end: Color,
        radius: f32,
        vertical: bool,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        self.push_shape(
            rect,
            start,
            end,
            radius,
            f32::from(vertical),
            opacity,
            viewport,
        );
    }

    /// Rounded outline `width` pixels thick, drawn as one quad the shader
    /// hollows out (negative gradient parameter = ring width).
    fn push_ring(
        &mut self,
        rect: Rect,
        color: Color,
        radius: f32,
        width: f32,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        self.push_shape(
            rect,
            color,
            color,
            radius,
            -width.max(0.5),
            opacity,
            viewport,
        );
    }

    /// A round-capped arc stroke as one quad around its circle; the shader cuts out
    /// the stroke from a signed distance (mode 3, geometry in `end_color`).
    #[allow(clippy::too_many_arguments)]
    fn push_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        width: f32,
        start: f32,
        sweep: f32,
        color: Color,
        knockout: Option<Rect>,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        // The knockout stripe, relative to the arc's centre: left edge, right edge and half
        // height (the stripe is centred vertically on the arc's centre). It rides in the
        // parameters the arc shader leaves unused; a zero height is no stripe.
        let [stripe_left, stripe_right, stripe_half] = knockout.map_or([0.0; 3], |band| {
            [
                band.x - center[0],
                band.right() - center[0],
                band.height * 0.5,
            ]
        });
        // One pixel of margin leaves room for the anti-aliased edge.
        let extent = radius + width * 0.5 + 1.0;
        let rect = Rect::new(
            center[0] - extent,
            center[1] - extent,
            extent * 2.0,
            extent * 2.0,
        );
        if radius <= 0.0 || width <= 0.0 || self.vertices.len() + 6 > MAX_SHAPE_VERTICES {
            return;
        }
        art::untextured(&mut self.runs, self.vertices.len());
        let position = |x: f32, y: f32| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let tint = [color.r, color.g, color.b, color.a * opacity];
        let points = [
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.y], [1.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.bottom()], [0.0, 1.0]),
        ];
        self.vertices
            .extend(points.map(|(pixel, local)| ShapeVertex {
                position: position(pixel[0], pixel[1]),
                local,
                size: [rect.width, rect.height],
                start_color: tint,
                end_color: [radius, width, start, sweep],
                parameters: [stripe_left, ARC_MODE],
                uv: [stripe_right, stripe_half],
            }));
    }

    /// A square-ended stroke as one quad along it, one pixel wider on each side for
    /// the anti-aliased edges the shader fades (mode 4, stroke width in `end_color.x`).
    /// A straight stroke's quad, cut to `clip` (window pixels) as every shape is: the
    /// part outside is left out, a stroke wholly outside draws nothing.
    #[allow(clippy::too_many_arguments)]
    fn push_line(
        &mut self,
        from: [f32; 2],
        to: [f32; 2],
        width: f32,
        color: Color,
        opacity: f32,
        (clip, round): (Rect, Option<(Rect, f32)>),
        viewport: [f32; 2],
    ) {
        let along = [to[0] - from[0], to[1] - from[1]];
        let length = along[0].hypot(along[1]);
        if !length.is_finite()
            || length < 0.01
            || width.is_nan()
            || width <= 0.0
            || self.vertices.len() + 3 * (CLIPPED_MAX - 2) > MAX_SHAPE_VERTICES
        {
            return;
        }
        let half = width * 0.5 + 1.0;
        let side = [-along[1] / length * half, along[0] / length * half];
        let corner =
            |point: [f32; 2], sign: f32| [point[0] + side[0] * sign, point[1] + side[1] * sign];
        let quad = [
            (corner(from, -1.0), [0.0, 0.0]),
            (corner(to, -1.0), [1.0, 0.0]),
            (corner(to, 1.0), [1.0, 1.0]),
            (corner(from, 1.0), [0.0, 1.0]),
        ];
        let (points, count) = clip_polygon(quad, clip, round);
        if count < 3 {
            return;
        }
        art::untextured(&mut self.runs, self.vertices.len());
        let position =
            |[x, y]: [f32; 2]| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let tint = [color.r, color.g, color.b, color.a * opacity];
        let vertex = |(point, local): ([f32; 2], [f32; 2])| ShapeVertex {
            position: position(point),
            local,
            size: [length, half * 2.0],
            start_color: tint,
            end_color: [width, 0.0, 0.0, 0.0],
            parameters: [0.0, LINE_MODE],
            uv: local,
        };
        // A fan from the first corner.
        for index in 1..count - 1 {
            self.vertices.push(vertex(points[0]));
            self.vertices.push(vertex(points[index]));
            self.vertices.push(vertex(points[index + 1]));
        }
    }

    /// A convex polygon as a fan of triangles from its middle, each carrying its
    /// distance to its outer edge for the shader to fade the last pixel (mode 7);
    /// textured when `uv` gives the texture coordinates of its bounding box's top-left
    /// and bottom-right corners.
    fn push_polygon(
        &mut self,
        points: &[[f32; 2]],
        color: Color,
        uv: Option<([f32; 2], [f32; 2])>,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        let count = points.len();
        if count < 3
            || self.vertices.len() + 3 * count > MAX_SHAPE_VERTICES
            || points.iter().flatten().any(|value| !value.is_finite())
        {
            return;
        }
        let middle = points
            .iter()
            .fold([0.0, 0.0], |sum, [x, y]| [sum[0] + x, sum[1] + y])
            .map(|sum| sum / count as f32);
        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
        for [x, y] in points {
            low = [low[0].min(*x), low[1].min(*y)];
            high = [high[0].max(*x), high[1].max(*y)];
        }
        let size = [(high[0] - low[0]).max(1e-6), (high[1] - low[1]).max(1e-6)];
        let position =
            |[x, y]: [f32; 2]| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let tint = [color.r, color.g, color.b, color.a * opacity];
        let (textured, (uv_low, uv_high)) = match uv {
            Some(uv) => (1.0, uv),
            None => (0.0, ([0.0; 2], [0.0; 2])),
        };
        let vertex = |point: [f32; 2], edge: f32| ShapeVertex {
            position: position(point),
            local: [edge, 0.0],
            size,
            start_color: tint,
            end_color: tint,
            parameters: [textured, POLYGON_MODE],
            uv: [
                uv_low[0] + (uv_high[0] - uv_low[0]) * (point[0] - low[0]) / size[0],
                uv_low[1] + (uv_high[1] - uv_low[1]) * (point[1] - low[1]) / size[1],
            ],
        };
        for index in 0..count {
            let (from, to) = (points[index], points[(index + 1) % count]);
            let along = [to[0] - from[0], to[1] - from[1]];
            let length = along[0].hypot(along[1]);
            if length < 1e-4 {
                continue;
            }
            // The middle's distance from this edge: the fan is wound either way.
            let reach = ((middle[0] - from[0]) * along[1] - (middle[1] - from[1]) * along[0]).abs()
                / length;
            self.vertices.push(vertex(middle, reach));
            self.vertices.push(vertex(from, 0.0));
            self.vertices.push(vertex(to, 0.0));
        }
    }

    /// One quad over the aura's shown part; `ui_shapes.wgsl` shades it per pixel as
    /// [`sjk_ui::strafe_aura_shade`] does, from the aura packed in the colours.
    fn push_aura(
        &mut self,
        center: [f32; 2],
        aura: sjk_ui::StrafeAura,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        // The shader draws up to 1.6 half widths off the centre line, within the span.
        let outer = aura.radius + aura.width * 0.8 + 1.0;
        let inner = (aura.radius - aura.width * 0.8).max(0.0);
        let span = aura.span.clamp(0.0, std::f32::consts::PI);
        let bottom = if span < std::f32::consts::FRAC_PI_2 {
            -inner * span.cos()
        } else {
            -outer * span.cos()
        };
        let side = if span < std::f32::consts::FRAC_PI_2 {
            outer * span.sin()
        } else {
            outer
        };
        let rect = Rect::new(
            center[0] - side,
            center[1] - outer,
            side * 2.0,
            outer + bottom.min(outer),
        );
        if aura.radius <= 0.0
            || aura.width <= 0.0
            || rect.height <= 0.0
            || self.vertices.len() + 6 > MAX_SHAPE_VERTICES
        {
            return;
        }
        art::untextured(&mut self.runs, self.vertices.len());
        let position = |x: f32, y: f32| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let points = [
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.y], [1.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.bottom()], [0.0, 1.0]),
        ];
        self.vertices
            .extend(points.map(|(pixel, local)| ShapeVertex {
                position: position(pixel[0], pixel[1]),
                local,
                size: [rect.width, rect.height],
                start_color: [aura.speed, aura.acceleration, aura.wishspeed, aura.base],
                end_color: [aura.radius, aura.width, aura.span, aura.time],
                parameters: [aura.alpha * opacity, AURA_MODE],
                // The quad's top-left corner relative to the aura's centre.
                uv: [rect.x - center[0], rect.y - center[1]],
            }));
    }

    /// One quad over `shown` (the part of the card at `rect` left by the clip);
    /// `ui_shapes.wgsl` shades it per pixel as [`sjk_ui::card_shine_shade`] does, from the
    /// finish packed in the colours, its pattern held to the whole card.
    fn push_shine(
        &mut self,
        rect: Rect,
        shown: Rect,
        shine: sjk_ui::CardShine,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        if shown.width <= 0.0
            || shown.height <= 0.0
            || shine.alpha * opacity <= 0.0
            || self.vertices.len() + 6 > MAX_SHAPE_VERTICES
        {
            return;
        }
        art::untextured(&mut self.runs, self.vertices.len());
        let position = |x: f32, y: f32| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let kind = match shine.kind {
            sjk_ui::ShineKind::Holo => 0.0,
            sjk_ui::ShineKind::Solar => 1.0,
            sjk_ui::ShineKind::Glare => 2.0,
        };
        let points = [
            ([shown.x, shown.y], [0.0, 0.0]),
            ([shown.right(), shown.y], [1.0, 0.0]),
            ([shown.right(), shown.bottom()], [1.0, 1.0]),
            ([shown.x, shown.y], [0.0, 0.0]),
            ([shown.right(), shown.bottom()], [1.0, 1.0]),
            ([shown.x, shown.bottom()], [0.0, 1.0]),
        ];
        self.vertices
            .extend(points.map(|(pixel, local)| ShapeVertex {
                position: position(pixel[0], pixel[1]),
                local,
                size: [shown.width, shown.height],
                start_color: [kind, shine.time, shine.light[0], shine.light[1]],
                end_color: [shine.size[0], shine.size[1], shine.radius, shine.hold],
                parameters: [shine.alpha * opacity, SHINE_MODE],
                uv: [shown.x - rect.x, shown.y - rect.y],
            }));
    }

    fn push_shape(
        &mut self,
        rect: Rect,
        start: Color,
        end: Color,
        radius: f32,
        mode: f32,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        if rect.width <= 0.0 || rect.height <= 0.0 || self.vertices.len() + 6 > MAX_SHAPE_VERTICES {
            return;
        }
        art::untextured(&mut self.runs, self.vertices.len());
        let position = |x: f32, y: f32| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let color = |value: Color| [value.r, value.g, value.b, value.a * opacity];
        let points = [
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.y], [1.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.bottom()], [0.0, 1.0]),
        ];
        self.vertices
            .extend(points.map(|(pixel, local)| ShapeVertex {
                position: position(pixel[0], pixel[1]),
                local,
                size: [rect.width, rect.height],
                start_color: color(start),
                end_color: color(end),
                parameters: [radius.min(rect.width.min(rect.height) * 0.5), mode],
                uv: local,
            }));
    }

    /// This renderer's identity, unique for the process. Every world has a
    /// renderer of its own, with an empty icon atlas, map preview and HUD
    /// preview, while the menu moves from world to world: an image uploaded
    /// into one renderer is not in another.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Upload one decoded [`ICON_SIZE`]-square RGBA icon into a stable atlas
    /// cell, sampled by `TexturedQuad` commands naming `texture`.
    pub(crate) fn upload_icon(
        &self,
        queue: &crate::frame_queue::FrameQueue,
        texture: sjk_ui::TextureId,
        rgba: &[u8],
    ) {
        self.icons.upload(queue, texture, rgba);
    }

    /// Give GIF slot `slot` ([`crate::chat_gifs::slot_texture`]) the frame `rgba` of a
    /// GIF of `size`.
    pub(crate) fn upload_gif(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        slot: usize,
        size: [u32; 2],
        rgba: &[u8],
    ) {
        self.gifs
            .upload(device, queue, &self.texture_layout, slot, size, rgba);
    }

    /// Sample `view` for `TexturedQuad` commands naming [`PREVIEW_TEXTURE`]
    /// (the model preview's display texture, after it is made or resized).
    pub(crate) fn set_preview(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        self.preview = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK model preview UI binding"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.preview_sampler),
                },
            ],
        }));
    }

    /// Sample `view` for `TexturedQuad` commands naming [`BLADE_PICTURES_TEXTURE`]
    /// (the Collection's blade pictures, once made).
    /// Let go of the blade pictures' binding, so their target can be freed.
    pub(crate) fn clear_blade_pictures(&mut self) {
        self.blade_pictures = None;
    }

    pub(crate) fn set_blade_pictures(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        self.blade_pictures = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade pictures UI binding"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.preview_sampler),
                },
            ],
        }));
    }

    /// Replace the map preview sampled by `TexturedQuad` commands naming
    /// [`LEVELSHOT_TEXTURE`] with `image`, at its own resolution.
    pub(crate) fn upload_levelshot(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        image: &crate::menu::levelshot::LevelshotImage,
    ) {
        self.levelshot
            .upload(device, queue, &self.texture_layout, image);
    }

    /// Replace News picture slot `slot` ([`news_image_texture`]) with `image`.
    pub(crate) fn upload_news_image(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        slot: usize,
        image: &crate::menu::levelshot::LevelshotImage,
    ) {
        if let Some(texture) = self.news.get_mut(slot) {
            texture.upload(device, queue, &self.texture_layout, image);
        }
    }

    /// Put the map editor's thumbnail `cell` ([`map_edit_pictures::cell_texture`]).
    pub(crate) fn upload_map_edit_cell(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        cell: u32,
        rgba: &[u8],
    ) {
        self.map_edit
            .upload_cell(device, queue, &self.texture_layout, cell, rgba);
    }

    /// Show `image` as the map editor's large preview ([`map_edit_pictures::PREVIEW`]).
    pub(crate) fn upload_map_edit_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        image: &crate::menu::levelshot::LevelshotImage,
    ) {
        self.map_edit
            .upload_preview(device, queue, &self.texture_layout, image);
    }

    /// Drop the map editor's pictures (its panel closed); whether any were held.
    pub(crate) fn release_map_edit_pictures(&mut self) -> bool {
        let held = self.map_edit.held();
        self.map_edit.release();
        held
    }

    /// Replace the HUD picker's preview sampled by `TexturedQuad` commands
    /// naming [`HUD_PREVIEW_TEXTURE`] with `image`.
    pub(crate) fn upload_hud_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        image: &crate::menu::levelshot::LevelshotImage,
    ) {
        self.hud_preview
            .upload(device, queue, &self.texture_layout, image);
    }
}

/// A projective transform of window points ([`DrawCommand::PushTransform`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Transform([f32; 9]);

impl Transform {
    /// `inner` applied first, then `outer` (none: `inner` alone).
    fn compose(outer: Option<Self>, inner: [f32; 9]) -> Self {
        let Some(Self(a)) = outer else {
            return Self(inner);
        };
        let b = inner;
        let mut out = [0.0; 9];
        for row in 0..3 {
            for column in 0..3 {
                out[row * 3 + column] = (0..3).map(|k| a[row * 3 + k] * b[k * 3 + column]).sum();
            }
        }
        Self(out)
    }

    /// Window point `[x, y]` mapped.
    pub(crate) fn map(self, [x, y]: [f32; 2]) -> [f32; 2] {
        let m = self.0;
        let w = m[6] * x + m[7] * y + m[8];
        let w = if w.abs() < 1e-6 { 1e-6 } else { w };
        [
            (m[0] * x + m[1] * y + m[2]) / w,
            (m[3] * x + m[4] * y + m[5]) / w,
        ]
    }

    /// A point in normalized device coordinates of a `viewport` mapped.
    pub(crate) fn map_ndc(self, [x, y]: [f32; 2], viewport: [f32; 2]) -> [f32; 2] {
        let window = [(x + 1.0) * 0.5 * viewport[0], (1.0 - y) * 0.5 * viewport[1]];
        let [x, y] = self.map(window);
        [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0]
    }
}

/// Map `vertices` (shape vertices just written) by `transform`, if any.
fn map_shapes(vertices: &mut [ShapeVertex], transform: Option<Transform>, viewport: [f32; 2]) {
    let Some(transform) = transform else {
        return;
    };
    for vertex in vertices {
        vertex.position = transform.map_ndc(vertex.position, viewport);
    }
}

/// Chords along each corner of a rounded clip ([`DrawCommand::PushRoundedClip`]).
const CORNER_CHORDS: usize = 4;
/// The most corners a quad cut by a rectangle, and by a rounded clip's corners, keeps:
/// one more for each edge it is cut by.
const CLIPPED_MAX: usize = 4 + 4 + 4 * CORNER_CHORDS;

/// A corner of a stroke's quad: its place (window pixels) and its local coordinates.
type Corner = ([f32; 2], [f32; 2]);

/// A convex quad (window pixels, each corner with its local coordinates) cut to `clip`
/// and, under a rounded clip (`round`: its rectangle and radius), to that rectangle's
/// rounded corners, each drawn as [`CORNER_CHORDS`] chords of its arc (Sutherland and
/// Hodgman, one edge at a time): the corners left, in order, and how many. The local
/// coordinates are carried along the cut edges, which is exact for a parallelogram's
/// (they are affine in the position).
fn clip_polygon(
    quad: [Corner; 4],
    clip: Rect,
    round: Option<(Rect, f32)>,
) -> ([Corner; CLIPPED_MAX], usize) {
    let mut points: [Corner; CLIPPED_MAX] = [([0.0; 2], [0.0; 2]); CLIPPED_MAX];
    points[..4].copy_from_slice(&quad);
    let mut count = 4;
    // Each edge as a half plane: inside where `normal . point >= bound`.
    let mut edges = [([0.0_f32; 2], 0.0_f32); 4 + 4 * CORNER_CHORDS];
    edges[..4].copy_from_slice(&[
        ([1.0, 0.0], clip.x),
        ([-1.0, 0.0], -clip.right()),
        ([0.0, 1.0], clip.y),
        ([0.0, -1.0], -clip.bottom()),
    ]);
    let mut edge_count = 4;
    if let Some((rect, radius)) = round {
        let r = radius.min(rect.width * 0.5).min(rect.height * 0.5);
        if r > 0.5 {
            // Each corner's centre and the angle its arc starts at (y grows downwards).
            let corners = [
                ([rect.x + r, rect.y + r], std::f32::consts::PI),
                ([rect.right() - r, rect.y + r], std::f32::consts::PI * 1.5),
                ([rect.right() - r, rect.bottom() - r], 0.0),
                ([rect.x + r, rect.bottom() - r], std::f32::consts::FRAC_PI_2),
            ];
            for (centre, start) in corners {
                for chord in 0..CORNER_CHORDS {
                    let step = std::f32::consts::FRAC_PI_2 / CORNER_CHORDS as f32;
                    let at =
                        |angle: f32| [centre[0] + r * angle.cos(), centre[1] + r * angle.sin()];
                    let (a, b) = (
                        at(start + step * chord as f32),
                        at(start + step * (chord + 1) as f32),
                    );
                    // The chord's inward normal points at the corner's centre.
                    let middle = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
                    let normal = [centre[0] - middle[0], centre[1] - middle[1]];
                    edges[edge_count] = (normal, normal[0] * a[0] + normal[1] * a[1]);
                    edge_count += 1;
                }
            }
        }
    }
    for &(normal, bound) in &edges[..edge_count] {
        let side = |point: &Corner| normal[0] * point.0[0] + normal[1] * point.0[1] - bound;
        let mut out: [Corner; CLIPPED_MAX] = [([0.0; 2], [0.0; 2]); CLIPPED_MAX];
        let mut kept = 0;
        for index in 0..count {
            let current = points[index];
            let previous = points[(index + count - 1) % count];
            let (now, was) = (side(&current), side(&previous));
            let (now_in, was_in) = (now >= 0.0, was >= 0.0);
            if now_in != was_in && kept < CLIPPED_MAX {
                let t = if (was - now).abs() > f32::EPSILON {
                    was / (was - now)
                } else {
                    0.0
                };
                let mix =
                    |a: [f32; 2], b: [f32; 2]| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                let mut point = mix(previous.0, current.0);
                // On a rectangle's edge exactly, whatever the rounding.
                if normal[1] == 0.0 {
                    point[0] = bound / normal[0];
                } else if normal[0] == 0.0 {
                    point[1] = bound / normal[1];
                }
                out[kept] = (point, mix(previous.1, current.1));
                kept += 1;
            }
            if now_in && kept < CLIPPED_MAX {
                out[kept] = current;
                kept += 1;
            }
        }
        points = out;
        count = kept;
        if count == 0 {
            break;
        }
    }
    (points, count)
}

fn clipped(left: Rect, right: Rect) -> Rect {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    Rect::new(
        x,
        y,
        left.right().min(right.right()) - x,
        left.bottom().min(right.bottom()) - y,
    )
}

fn border_rects(rect: Rect, width: f32) -> [Rect; 4] {
    [
        Rect::new(rect.x, rect.y, rect.width, width),
        Rect::new(rect.x, rect.bottom() - width, rect.width, width),
        Rect::new(rect.x, rect.y, width, rect.height),
        Rect::new(rect.right() - width, rect.y, width, rect.height),
    ]
}

/// Append all text commands without allocating; HUD rectangles are consumed by
/// the existing fullscreen quad pass. `style` scales and tracks every command
/// ([`text::TextStyle::NEUTRAL`] draws the layout as authored).
pub(crate) fn append_text_commands<'a>(
    draw_list: &DrawList,
    resolve: impl Fn(TextId) -> &'a str,
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    viewport: [f32; 2],
    style: text::TextStyle,
) {
    append_text_commands_where(
        draw_list,
        resolve,
        |_, _| true,
        vertices,
        font,
        viewport,
        style,
        text::CodePalette::Game,
    );
}

/// Append the text commands `keep` accepts (given each command's id and text),
/// so one draw list can be split between fonts. Opacity and clip scopes apply
/// to every pass alike, and `style` as in [`append_text_commands`]; colour
/// codes draw in `palette`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_text_commands_where<'a>(
    draw_list: &DrawList,
    resolve: impl Fn(TextId) -> &'a str,
    keep: impl Fn(TextId, &str) -> bool,
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    viewport: [f32; 2],
    style: text::TextStyle,
    palette: text::CodePalette,
) {
    append_text_commands_paletted(
        draw_list,
        resolve,
        keep,
        vertices,
        font,
        viewport,
        style,
        |_| palette,
    );
}

/// [`append_text_commands_where`], each command's colour codes and edge drawn in the
/// palette `palette_of` gives for its id (the chat's outline, `cg_chatBoxOutline`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_text_commands_paletted<'a>(
    draw_list: &DrawList,
    resolve: impl Fn(TextId) -> &'a str,
    keep: impl Fn(TextId, &str) -> bool,
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    viewport: [f32; 2],
    style: text::TextStyle,
    palette_of: impl Fn(TextId) -> text::CodePalette,
) {
    let mut opacity = [1.0_f32; 8];
    let mut opacity_depth = 0_usize;
    let mut clips = [Rect::new(0.0, 0.0, viewport[0], viewport[1]); 8];
    let mut clip_depth = 0_usize;
    let mut transforms: [Option<Transform>; 4] = [None; 4];
    let mut transform_depth = 0_usize;
    for command in draw_list.commands() {
        let DrawCommand::Text {
            rect,
            text: id,
            size,
            color,
            align,
            weight,
            letter_spacing,
            overflow,
        } = command
        else {
            match command {
                DrawCommand::PushOpacity(value) if opacity_depth + 1 < opacity.len() => {
                    opacity_depth += 1;
                    opacity[opacity_depth] = opacity[opacity_depth - 1] * value.clamp(0.0, 1.0);
                }
                DrawCommand::PopOpacity => opacity_depth = opacity_depth.saturating_sub(1),
                DrawCommand::PushTransform(matrix) if transform_depth + 1 < transforms.len() => {
                    transform_depth += 1;
                    transforms[transform_depth] =
                        Some(Transform::compose(transforms[transform_depth - 1], *matrix));
                }
                DrawCommand::PopTransform => transform_depth = transform_depth.saturating_sub(1),
                DrawCommand::PushClip(rect) | DrawCommand::PushRoundedClip { rect, .. }
                    if clip_depth + 1 < clips.len() =>
                {
                    clip_depth += 1;
                    clips[clip_depth] = clipped(clips[clip_depth - 1], *rect);
                }
                DrawCommand::PopClip => clip_depth = clip_depth.saturating_sub(1),
                _ => {}
            }
            continue;
        };
        let value = resolve(*id);
        if !keep(*id, value) {
            continue;
        }
        let placement = style.place(*rect, *size, *letter_spacing);
        let rect = placement.bounds;
        let scale = placement.size / font.height.max(1.0);
        let face = match weight {
            FontWeight::Regular => text::TextFace::Regular,
            FontWeight::Semibold => text::TextFace::Semibold,
        };
        let measured =
            text::visible_text_width_style(font, value, scale, face, placement.letter_spacing);
        let width = if *overflow == sjk_ui::TextOverflow::Ellipsis {
            measured.min(rect.width)
        } else {
            measured
        };
        let x = match align {
            TextAlign::Start => rect.x,
            TextAlign::Center => rect.x + (rect.width - width) * 0.5,
            TextAlign::End => rect.right() - width,
        };
        let start = vertices.len();
        text::append_bounded(
            vertices,
            font,
            value,
            [x, placement.y],
            rect,
            clipped(rect, clips[clip_depth]),
            scale,
            viewport,
            face,
            [color.r, color.g, color.b, color.a * opacity[opacity_depth]],
            placement.letter_spacing,
            *overflow,
            palette_of(*id),
        );
        if let Some(transform) = transforms[transform_depth] {
            text::map_vertices(&mut vertices[start..], |point| {
                transform.map_ndc(point, viewport)
            });
        }
    }
}

#[cfg(test)]
mod line_clip_tests {
    use super::*;

    fn quad(x: f32, y: f32, width: f32, height: f32) -> [([f32; 2], [f32; 2]); 4] {
        [
            ([x, y], [0.0, 0.0]),
            ([x + width, y], [1.0, 0.0]),
            ([x + width, y + height], [1.0, 1.0]),
            ([x, y + height], [0.0, 1.0]),
        ]
    }

    /// A stroke inside the clip keeps its four corners, one across an edge is cut there
    /// with its local coordinates carried along, one outside keeps nothing, and a slanted
    /// one cut at a corner keeps at most eight.
    #[test]
    fn a_strokes_quad_is_cut_to_the_clip() {
        let clip = Rect::new(0.0, 0.0, 100.0, 100.0);
        let (points, count) = clip_polygon(quad(10.0, 10.0, 20.0, 20.0), clip, None);
        assert_eq!(count, 4);
        assert_eq!(points[..4], quad(10.0, 10.0, 20.0, 20.0));
        let (points, count) = clip_polygon(quad(80.0, 10.0, 40.0, 20.0), clip, None);
        assert_eq!(count, 4);
        let right = points[..count]
            .iter()
            .map(|(point, _)| point[0])
            .fold(f32::MIN, f32::max);
        assert_eq!(right, 100.0, "cut at the clip's edge");
        let cut = points[..count]
            .iter()
            .find(|(point, _)| point[0] == 100.0)
            .unwrap();
        assert!((cut.1[0] - 0.5).abs() < 1e-5, "half way along");
        assert_eq!(clip_polygon(quad(200.0, 10.0, 20.0, 20.0), clip, None).1, 0);
        let diamond = [
            ([50.0, -40.0], [0.0, 0.0]),
            ([140.0, 50.0], [1.0, 0.0]),
            ([50.0, 140.0], [1.0, 1.0]),
            ([-40.0, 50.0], [0.0, 1.0]),
        ];
        let (points, count) = clip_polygon(diamond, clip, None);
        assert_eq!(count, 8);
        for (point, _) in &points[..count] {
            assert!((0.0..=100.0).contains(&point[0]) && (0.0..=100.0).contains(&point[1]));
        }
    }

    /// Under a rounded clip a stroke keeps off the corners: nothing is left outside the
    /// arc, while the middle of an edge is cut as a plain rectangle would cut it.
    #[test]
    fn a_rounded_clip_keeps_strokes_off_the_corners() {
        let clip = Rect::new(0.0, 0.0, 100.0, 100.0);
        let round = Some((clip, 20.0));
        // A square over the top-left corner.
        let (points, count) = clip_polygon(quad(-10.0, -10.0, 30.0, 30.0), clip, round);
        assert!(count >= 3);
        for (point, _) in &points[..count] {
            let from_centre = (point[0] - 20.0).hypot(point[1] - 20.0);
            let in_corner = point[0] < 20.0 && point[1] < 20.0;
            assert!(
                !in_corner || from_centre <= 20.01,
                "{point:?} is past the arc"
            );
        }
        // Along the middle of the top edge, the cut is the rectangle's.
        let (points, count) = clip_polygon(quad(40.0, -10.0, 20.0, 30.0), clip, round);
        assert_eq!(count, 4);
        let top = points[..count]
            .iter()
            .map(|(point, _)| point[1])
            .fold(f32::MAX, f32::min);
        assert_eq!(top, 0.0);
        // Wholly in the corner, outside the arc: nothing.
        assert_eq!(clip_polygon(quad(0.0, 0.0, 3.0, 3.0), clip, round).1, 0);
    }
}

#[cfg(test)]
mod shape_shader_tests {
    #[test]
    fn the_shape_shader_validates() {
        use wgpu::naga;
        let source = include_str!("ui_shapes.wgsl");
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
    }
}
