//! Tags on the world (`tags.rs`), drawn in a pass of their own (31/10/2026, Sol: tags
//! were blurry in the game). They went through the effects' texture atlas, which squeezes
//! every picture into a 128-pixel tile: a 256-pixel tag kept a quarter of its pixels,
//! stretched over a tag's 56 units of wall. Here each picture on show has a texture of
//! its own at full size, mipmapped with alpha-weighted levels and sampled anisotropically;
//! a tag smaller than [`EDGE_TAG`] is enlarged to it once (Catmull-Rom), so it does not
//! blur further as the GPU magnifies it. Its finish is shaded per pixel
//! (`tag_render.wgsl`), as the cards' are (`sjk_ui::shine`): Holo's rainbow foil, its
//! light following the viewer's angle, and Solar's corona burning round the picture's own
//! outline.
//!
//! - **Geometry:** the decal store projects each tag onto the world
//!   ([`crate::decal_store::DecalStore::tag_fragments`]); this pass rebuilds its vertices
//!   only when they change ([`TagRender::prepare`]).
//! - **Pictures:** a tag's `gfx/tags/<id>.png` from the hub's `sjk_tags` pack; a card's
//!   picture from its card pack, made a sticker ([`sticker`]: rounded corners and a rim of
//!   the card's colour). Built on a worker thread the first time one is on show, kept
//!   while it is drawn and let go [`UNUSED`] after; packs mounted anew drop them all.
//!   A picture not there yet (or missing) draws nothing.
//! - **GIF tags** (10/11/2026): the chat's decoded GIF (`chat_gifs::wall`), each in a
//!   square texture of its own (the GIF centred on transparency, so it keeps its shape),
//!   one level, written again only when the animation reaches a new frame. No finish, so
//!   no outline: a blank one stands in.
//! - **The outline:** a signed distance to the picture's edge ([`outline`]), in a second
//!   texture, for Solar's corona.
//! - **Drawn** in the effect layer right after the world's marks (`particle_draw.rs`),
//!   depth-tested with the marks' polygon offset, never writing depth.

use glam::Vec3;
use image::{GrayImage, RgbaImage};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// A tag's picture edge in texels: a smaller one is enlarged to it.
pub(crate) const EDGE_TAG: u32 = 512;
/// A card sticker's canvas edge.
pub(crate) const EDGE_CARD: u32 = 1_024;
/// The largest edge a picture keeps; a bigger one is reduced.
const EDGE_MAX: u32 = 1_024;
/// A picture not drawn for this long is let go.
const UNUSED: Duration = Duration::from_secs(60);
/// The outline's reach either way, a share of the picture's edge: the signed distance is
/// kept from this far outside the picture (0) to this far inside (1). `tag_render.wgsl`
/// reads it with the same figure.
pub(crate) const OUTLINE_REACH: f32 = 0.25;
/// A card sticker: the picture's width against the canvas, its corners' radius and its
/// rim's width against the picture's width.
const STICKER_WIDTH: f32 = 0.92;
const STICKER_CORNER: f32 = 0.055;
const STICKER_RIM: f32 = 0.016;
/// `qglPolygonOffset(-1, -2)`, as the marks' (`effect_geometry_gpu.rs`).
const BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: -2,
    slope_scale: -1.0,
    clamp: 0.0,
};

/// One corner of a tag's fragment.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    /// The picture's left-to-right and top-to-bottom on the world.
    right: [f32; 3],
    down: [f32; 3],
    /// The finish (0 Normal, 1 Holo, 2 Solar), a seed of the tag's own, and two spare.
    params: [f32; 4],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 => Float32x3, 4 => Float32x4];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// What a picture is drawn from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Source {
    /// A tag of the catalogue: its picture in the `sjk_tags` pack.
    Tag(String),
    /// A card: its picture in the card packs, made a sticker.
    Card(String),
    /// A GIF tag: its GIPHY GIF, animated.
    Gif(crate::chat_gifs::GifId),
}

impl Source {
    /// The picture a placed tag shows.
    pub(crate) fn of(id: &str) -> Self {
        if let Some(gif) = crate::tags::gif_id(id) {
            Self::Gif(gif)
        } else if crate::tags::get(id).is_some() {
            Self::Tag(id.to_owned())
        } else {
            Self::Card(id.to_owned())
        }
    }
}

/// A picture built by a worker, sent back with what it was built from (`None`: missing).
type Delivery = (Source, Option<Built>);

/// A picture built for upload: its mip chain and its outline's.
pub(crate) struct Built {
    picture: Vec<RgbaImage>,
    outline: Vec<GrayImage>,
}

/// A picture's place in the cache.
enum State {
    /// Asked of a worker.
    Loading,
    /// Uploaded.
    Ready(wgpu::BindGroup),
    /// Its file is missing or would not decode: asked again once packs are mounted anew.
    Missing,
    /// Let go after [`UNUSED`].
    Gone,
}

struct Slot {
    source: Source,
    state: State,
    used: Instant,
    /// A GIF tag's texture, once its GIF is decoded.
    gif: Option<GifTexture>,
}

/// A GIF tag's texture: square, its side the GIF's longer one.
struct GifTexture {
    texture: wgpu::Texture,
    /// The GIF it holds (its frames' address) and the frame written last.
    gif: usize,
    frame: usize,
}

/// One tag's draw: its picture and its vertices.
struct Draw {
    slot: usize,
    vertices: std::ops::Range<u32>,
    /// Its centre and how far it reaches, for the effect layer's rectangle.
    centre: Vec3,
}

/// The tags' pass.
pub(crate) struct TagRender {
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_group: wgpu::BindGroup,
    picture_layout: wgpu::BindGroupLayout,
    vertex_buffer: Option<wgpu::Buffer>,
    vertices: Vec<Vertex>,
    draws: Vec<Draw>,
    slots: Vec<Slot>,
    /// The decal store's tags version the draws were built from.
    version: Option<u64>,
    /// The packs' generation the pictures came from.
    packs: u64,
    /// Where workers send the pictures they built, and where this pass takes them.
    built: (Sender<Delivery>, Receiver<Delivery>),
    /// The last time slots were swept for pictures not drawn lately.
    swept: Instant,
    /// The outline GIF tags are drawn with: one texel, outside every picture.
    blank_outline: wgpu::TextureView,
}

impl TagRender {
    /// The pass, drawing into the effect layer's `format` with the cameras of
    /// `camera_layout`, sampling with up to `anisotropy` (1 to 16).
    pub(crate) fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        anisotropy: u16,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK tag shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("tag_render.wgsl").into()),
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK tag globals layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let picture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK tag picture layout"),
            entries: &[
                crate::render_helpers::texture_layout_entry(0),
                crate::render_helpers::texture_layout_entry(1),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK tag pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(&globals_layout),
                Some(&picture_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK tag pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DepthTarget::FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: BIAS,
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK tag globals"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SJK tag sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: anisotropy.clamp(1, 16),
            ..Default::default()
        });
        let globals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK tag globals"),
            layout: &globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: globals.as_entire_binding(),
                },
            ],
        });
        Self {
            pipeline,
            globals,
            globals_group,
            picture_layout,
            vertex_buffer: None,
            vertices: Vec::new(),
            draws: Vec::new(),
            slots: Vec::new(),
            version: None,
            packs: crate::sjk_packs::generation(),
            built: channel(),
            swept: Instant::now(),
            // wgpu clears a new texture: its one texel is 0, outside every picture.
            blank_outline: device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("SJK GIF tag outline"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }

    /// Ready this frame's tags: take the pictures the workers built, rebuild the vertices
    /// when `decals`' tags changed, ask for the pictures on show and set the finishes'
    /// clock to `seconds`.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        decals: &crate::decal_store::DecalStore,
        now: Instant,
        seconds: f32,
    ) {
        let packs = crate::sjk_packs::generation();
        if packs != self.packs {
            // New packs may hold new pictures: everything is asked again.
            self.packs = packs;
            self.slots.clear();
            self.version = None;
        }
        while let Ok((source, built)) = self.built.1.try_recv() {
            let Some(index) = self
                .slots
                .iter()
                .position(|slot| slot.source == source && matches!(slot.state, State::Loading))
            else {
                continue;
            };
            let state = match built {
                Some(built) => State::Ready(self.upload(device, queue, &built)),
                None => State::Missing,
            };
            self.slots[index].state = state;
        }
        if self.version != Some(decals.tags_version()) {
            self.rebuild(device, queue, decals, now);
        }
        for index in 0..self.draws.len() {
            let slot = self.draws[index].slot;
            if let Some(held) = self.slots.get_mut(slot) {
                held.used = now;
            }
            self.animate(device, queue, slot);
        }
        if !self.draws.is_empty() {
            queue.write_buffer(
                &self.globals,
                0,
                bytemuck::cast_slice(&[seconds, 0.0, 0.0, 0.0]),
            );
        }
        if now.saturating_duration_since(self.swept) > Duration::from_secs(5) {
            self.swept = now;
            for slot in &mut self.slots {
                if now.saturating_duration_since(slot.used) > UNUSED
                    && matches!(slot.state, State::Ready(_) | State::Missing)
                {
                    slot.state = State::Gone;
                    slot.gif = None;
                }
            }
        }
    }

    /// Bring GIF tag slot `index` to its animation's frame of the moment: its texture made
    /// once the GIF is decoded, the frame written when it changed. A GIF that cannot be
    /// had (or is hidden, or GIF tags went off) draws nothing.
    fn animate(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        index: usize,
    ) {
        let Some(Slot {
            source: Source::Gif(id),
            ..
        }) = self.slots.get(index)
        else {
            return;
        };
        let (gif, frame) = match crate::chat_gifs::wall(id) {
            crate::chat_gifs::Wall::Ready { gif, frame } => (gif, frame),
            crate::chat_gifs::Wall::Loading => {
                let slot = &mut self.slots[index];
                if slot.gif.is_none() {
                    slot.state = State::Loading;
                }
                return;
            }
            crate::chat_gifs::Wall::Unavailable => {
                let slot = &mut self.slots[index];
                slot.state = State::Missing;
                slot.gif = None;
                return;
            }
        };
        let address = Arc::as_ptr(&gif) as usize;
        let [width, height] = gif.size;
        let edge = width.max(height).max(1);
        let fresh = self.slots[index]
            .gif
            .as_ref()
            .is_none_or(|held| held.gif != address);
        if fresh {
            // wgpu clears a new texture: round the GIF it stays transparent.
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("SJK GIF tag"),
                size: wgpu::Extent3d {
                    width: edge,
                    height: edge,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let group = self.bind(device, &view, &self.blank_outline);
            let slot = &mut self.slots[index];
            slot.state = State::Ready(group);
            slot.gif = Some(GifTexture {
                texture,
                gif: address,
                frame: usize::MAX,
            });
        }
        let Some(held) = self.slots[index].gif.as_mut() else {
            return;
        };
        let Some(pixels) = gif.frames.get(frame) else {
            return;
        };
        if held.frame == frame || pixels.rgba.len() != (width * height * 4) as usize {
            return;
        }
        held.frame = frame;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &held.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: (edge - width) / 2,
                    y: (edge - height) / 2,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &pixels.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// The draws and vertices of `decals`' tags, and their pictures asked for.
    fn rebuild(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        decals: &crate::decal_store::DecalStore,
        now: Instant,
    ) {
        self.version = Some(decals.tags_version());
        // Slots let go are forgotten, which renumbers the others: the draws are made anew
        // right after.
        if self.slots.len() > 48 {
            self.slots.retain(|slot| !matches!(slot.state, State::Gone));
        }
        self.vertices.clear();
        self.draws.clear();
        for (tag, fragments) in decals.tag_fragments() {
            let source = Source::of(&tag.id);
            let slot = self.slot(source, now);
            let normal = Vec3::from_array(tag.normal);
            let axes = crate::decal_marks::texture_axes(normal, tag.spin);
            let finish = crate::tags::finish_of(tag).index as f32;
            let at = Vec3::from_array(tag.at);
            let seed = (at.x * 0.013 + at.y * 0.029 + at.z * 0.071).rem_euclid(1.0);
            let start = self.vertices.len() as u32;
            for fragment in fragments {
                let corner = |point: &crate::decal_marks::DecalVertex| Vertex {
                    position: point.position,
                    uv: point.st,
                    right: axes[1].to_array(),
                    down: axes[2].to_array(),
                    params: [finish, seed, 0.0, 0.0],
                };
                // A convex fragment as a fan of triangles.
                for index in 1..fragment.len().saturating_sub(1) {
                    self.vertices.extend([
                        corner(&fragment[0]),
                        corner(&fragment[index]),
                        corner(&fragment[index + 1]),
                    ]);
                }
            }
            let end = self.vertices.len() as u32;
            if end > start {
                self.draws.push(Draw {
                    slot,
                    vertices: start..end,
                    centre: at,
                });
            }
        }
        if self.vertices.is_empty() {
            return;
        }
        let bytes = std::mem::size_of_val(self.vertices.as_slice()) as u64;
        let fits = self
            .vertex_buffer
            .as_ref()
            .is_some_and(|buffer| buffer.size() >= bytes);
        if !fits {
            self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("SJK tag vertices"),
                size: bytes.next_power_of_two().max(4_096),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(buffer) = &self.vertex_buffer {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    /// The slot of `source`'s picture, asked for when it is new (or was let go).
    fn slot(&mut self, source: Source, now: Instant) -> usize {
        let index = match self.slots.iter().position(|slot| slot.source == source) {
            Some(index) => index,
            None => {
                self.slots.push(Slot {
                    source: source.clone(),
                    state: State::Gone,
                    used: now,
                    gif: None,
                });
                self.slots.len() - 1
            }
        };
        if matches!(source, Source::Gif(_)) {
            // A GIF comes from the chat's cache, frame by frame ([`Self::animate`]).
            if matches!(self.slots[index].state, State::Gone) {
                self.slots[index].state = State::Loading;
                self.slots[index].used = now;
            }
            return index;
        }
        if matches!(self.slots[index].state, State::Gone) {
            self.slots[index].state = State::Loading;
            self.slots[index].used = now;
            ask(source, self.built.0.clone());
        }
        index
    }

    /// Upload `built` and bind it.
    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        built: &Built,
    ) -> wgpu::BindGroup {
        let picture =
            crate::gpu_texture::upload_levels(device, queue, "SJK tag picture", &built.picture);
        let outline = upload_gray_levels(device, queue, "SJK tag outline", &built.outline);
        self.bind(device, &picture, &outline)
    }

    /// Bind a picture and its outline.
    fn bind(
        &self,
        device: &wgpu::Device,
        picture: &wgpu::TextureView,
        outline: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK tag picture"),
            layout: &self.picture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(picture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(outline),
                },
            ],
        })
    }

    /// Whether some tag has its picture to draw.
    pub(crate) fn has_draws(&self) -> bool {
        self.draws.iter().any(|draw| self.ready(draw).is_some())
    }

    fn ready(&self, draw: &Draw) -> Option<&wgpu::BindGroup> {
        match &self.slots.get(draw.slot)?.state {
            State::Ready(group) => Some(group),
            _ => None,
        }
    }

    /// Add the tags drawn to the effect layer's screen bounds.
    pub(crate) fn bound(&self, bounds: &mut crate::frame_target::aa::effects::bounds::Bounds) {
        for draw in &self.draws {
            if self.ready(draw).is_some() {
                bounds.sphere(draw.centre, crate::tags::RADIUS * std::f32::consts::SQRT_2);
            }
        }
    }

    /// Draw the tags whose pictures are ready.
    pub(crate) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera: &'a wgpu::BindGroup) {
        let Some(buffer) = &self.vertex_buffer else {
            return;
        };
        if !self.has_draws() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.globals_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for draw in &self.draws {
            if let Some(group) = self.ready(draw) {
                pass.set_bind_group(2, group, &[]);
                pass.draw(draw.vertices.clone(), 0..1);
            }
        }
    }
}

/// Upload an `R8Unorm` texture of the caller's mip levels, `chain[0]` the largest.
fn upload_gray_levels(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    label: &str,
    chain: &[GrayImage],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: chain[0].width(),
            height: chain[0].height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: chain.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, pixels) in chain.iter().enumerate() {
        let (width, height) = pixels.dimensions();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// What a worker builds a picture from, read on the asking thread.
enum Job {
    Tag(Vec<u8>, String),
    Card(Arc<[u8]>, [f32; 3]),
}

/// Ask for `source`'s picture: its file is read here, built on a worker thread (here in
/// tests) and sent to `done`, `None` when it is missing or would not decode.
fn ask(source: Source, done: Sender<Delivery>) {
    let job = match &source {
        Source::Tag(id) => {
            let path = crate::tags::get(id)
                .map_or_else(|| format!("gfx/tags/{id}.png"), |tag| tag.picture());
            let skins = crate::sjk_packs::skins();
            skins
                .packs()
                .read(&path)
                .ok()
                .flatten()
                .map(|asset| Job::Tag(asset.bytes, path))
        }
        // A GIF tag's frames come from the chat's cache, never from here.
        Source::Gif(_) => None,
        Source::Card(id) => {
            let faces = crate::cards::faces::current();
            faces.find(id).and_then(|(index, face)| {
                let colour = face.tone().colour;
                faces
                    .picture(index)
                    .map(|bytes| Job::Card(bytes, [colour.r, colour.g, colour.b]))
            })
        }
    };
    let Some(job) = job else {
        let _ = done.send((source, None));
        return;
    };
    let shrink = crate::world_materials::filtering::mips::picmip().min(1);
    let work = move || {
        let built = match job {
            Job::Tag(bytes, path) => tag_picture(&bytes, &path, shrink),
            Job::Card(bytes, colour) => sticker(&bytes, colour, EDGE_CARD >> shrink),
        };
        if let Err(why) = &built {
            crate::log::progress(format_args!("tags: {source:?}: {why}"));
        }
        let _ = done.send((source, built.ok()));
    };
    if cfg!(test) {
        work();
        return;
    }
    if let Err(why) = std::thread::Builder::new()
        .name("SJK tag picture".to_owned())
        .spawn(work)
    {
        crate::log::progress(format_args!("tags: no worker for a picture: {why}"));
    }
}

/// A tag's picture: decoded, made square on transparency, sized to a power of two from
/// [`EDGE_TAG`] to [`EDGE_MAX`] (halved `shrink` times, for `r_picmip`) and its outline
/// traced.
fn tag_picture(bytes: &[u8], path: &str, shrink: u32) -> Result<Built, String> {
    let image = crate::gpu_texture::decode_image(bytes, path)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err("the picture is empty".to_owned());
    }
    let side = width.max(height);
    let mut square = RgbaImage::new(side, side);
    image::imageops::overlay(
        &mut square,
        &image,
        i64::from((side - width) / 2),
        i64::from((side - height) / 2),
    );
    let edge = (side.next_power_of_two().clamp(EDGE_TAG, EDGE_MAX) >> shrink).max(64);
    let sized = if edge == side {
        square
    } else {
        resize_straight(&square, edge)
    };
    let field = outline(&sized);
    Ok(Built {
        picture: picture_levels(sized),
        outline: gray_levels(field),
    })
}

/// `image` resized to `edge` square with Catmull-Rom, its colour weighted by its alpha so
/// the transparent surroundings' black does not bleed into the picture's edge.
fn resize_straight(image: &RgbaImage, edge: u32) -> RgbaImage {
    let mut weighted = image.clone();
    for pixel in weighted.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] = ((u16::from(pixel[channel]) * alpha + 127) / 255) as u8;
        }
    }
    let mut sized = image::imageops::resize(
        &weighted,
        edge,
        edge,
        image::imageops::FilterType::CatmullRom,
    );
    for pixel in sized.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 {
            continue;
        }
        for channel in 0..3 {
            pixel[channel] = ((u16::from(pixel[channel]) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
    sized
}

/// A card's picture as a sticker on an `edge`-square canvas: cut to the art window's 6:5
/// about its middle, [`STICKER_WIDTH`] of the canvas wide in its middle, its corners
/// rounded and a rim of the card's `colour` (its rank's or rarity's) round it. Its
/// outline is the rounded rectangle's own.
fn sticker(bytes: &[u8], colour: [f32; 3], edge: u32) -> Result<Built, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err("the picture is empty".to_owned());
    }
    let aspect = crate::cards::faces::ART_ASPECT;
    let (cut_w, cut_h) = if width as f32 / height as f32 > aspect {
        (((height as f32 * aspect).round() as u32).min(width), height)
    } else {
        (width, ((width as f32 / aspect).round() as u32).min(height))
    };
    let cut = image::imageops::crop_imm(
        &image,
        (width - cut_w) / 2,
        (height - cut_h) / 2,
        cut_w,
        cut_h,
    )
    .to_image();
    let box_w = (edge as f32 * STICKER_WIDTH).round() as u32;
    let box_h = (box_w as f32 / aspect).round() as u32;
    let art = image::imageops::resize(&cut, box_w, box_h, image::imageops::FilterType::CatmullRom);
    let (left, top) = ((edge - box_w) as f32 * 0.5, (edge - box_h) as f32 * 0.5);
    let half = [box_w as f32 * 0.5, box_h as f32 * 0.5];
    let radius = box_w as f32 * STICKER_CORNER;
    let rim = (box_w as f32 * STICKER_RIM).max(1.5);
    // The rim a touch brighter than the colour, as the cards' edges are.
    let rim_colour = colour.map(|c| (c * 0.85 + 0.15).clamp(0.0, 1.0) * 255.0);
    let mut canvas = RgbaImage::new(edge, edge);
    let mut field = GrayImage::new(edge, edge);
    for y in 0..edge {
        for x in 0..edge {
            let px = x as f32 + 0.5 - left - half[0];
            let py = y as f32 + 0.5 - top - half[1];
            let distance = rounded_box(px, py, half, radius);
            let inside = (-distance / edge as f32 / OUTLINE_REACH * 0.5 + 0.5).clamp(0.0, 1.0);
            field.put_pixel(x, y, image::Luma([(inside * 255.0).round() as u8]));
            let coverage = (0.5 - distance).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let ax = (x as f32 - left).clamp(0.0, (box_w - 1) as f32) as u32;
            let ay = (y as f32 - top).clamp(0.0, (box_h - 1) as f32) as u32;
            let picture = art.get_pixel(ax, ay);
            // The rim: from the edge to `rim` in, blended over a pixel or so; a thin
            // shadow where it meets the picture.
            let in_rim = (distance + rim + 0.5).clamp(0.0, 1.0);
            let seam = (-((distance + rim) / 1.2).powi(2)).exp() * 0.45;
            let mut pixel = [0_u8; 4];
            for channel in 0..3 {
                let art = f32::from(picture[channel]) * (1.0 - seam);
                let value = art + (rim_colour[channel] - art) * in_rim;
                pixel[channel] = value.round().clamp(0.0, 255.0) as u8;
            }
            pixel[3] = (coverage * 255.0).round() as u8;
            canvas.put_pixel(x, y, image::Rgba(pixel));
        }
    }
    Ok(Built {
        picture: picture_levels(canvas),
        outline: gray_levels(field),
    })
}

/// Signed distance from a point (`px`, `py` from the box's middle) to a box of half sizes
/// `half` with corners of `radius`; negative inside.
fn rounded_box(px: f32, py: f32, half: [f32; 2], radius: f32) -> f32 {
    let qx = px.abs() - (half[0] - radius);
    let qy = py.abs() - (half[1] - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

/// The picture's mip chain, each level averaged by alpha (`MipChain`), so the
/// transparent surroundings do not darken its edge as it shrinks.
fn picture_levels(image: RgbaImage) -> Vec<RgbaImage> {
    let chain = crate::menu::emblem::MipChain::new(image);
    let mut edge = chain.size;
    chain
        .levels
        .into_iter()
        .map(|pixels| {
            let level = RgbaImage::from_raw(edge, edge, pixels).expect("a square level");
            edge = (edge / 2).max(1);
            level
        })
        .collect()
}

/// A one-channel image's mip chain down to one texel, each level the box average of the
/// one above.
fn gray_levels(image: GrayImage) -> Vec<GrayImage> {
    let mut levels = vec![image];
    while let Some(last) = levels.last() {
        let (width, height) = last.dimensions();
        if width <= 1 && height <= 1 {
            break;
        }
        let (next_w, next_h) = ((width / 2).max(1), (height / 2).max(1));
        let next = GrayImage::from_fn(next_w, next_h, |x, y| {
            let mut sum = 0_u32;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let sx = (x * 2 + dx).min(width - 1);
                let sy = (y * 2 + dy).min(height - 1);
                sum += u32::from(last.get_pixel(sx, sy)[0]);
            }
            image::Luma([((sum + 2) / 4) as u8])
        });
        levels.push(next);
    }
    levels
}

/// The signed distance to `image`'s outline (where its alpha crosses a half), as a share
/// of its edge from [`OUTLINE_REACH`] outside (0) to as far inside (1): exact Euclidean
/// distances (Felzenszwalb and Huttenlocher's transform), for Solar's corona.
fn outline(image: &RgbaImage) -> GrayImage {
    let (width, height) = image.dimensions();
    let solid: Vec<bool> = image.pixels().map(|pixel| pixel[3] >= 128).collect();
    let to_solid = squared_distances(&solid, width, height, true);
    let to_clear = squared_distances(&solid, width, height, false);
    let reach = OUTLINE_REACH * width.max(height) as f32;
    GrayImage::from_fn(width, height, |x, y| {
        let at = (y * width + x) as usize;
        // Half a texel either way, so the outline lies between the two kinds of pixel.
        let signed = if solid[at] {
            to_clear[at].sqrt() as f32 - 0.5
        } else {
            0.5 - to_solid[at].sqrt() as f32
        };
        let value = (signed / reach * 0.5 + 0.5).clamp(0.0, 1.0);
        image::Luma([(value * 255.0).round() as u8])
    })
}

/// Each pixel's squared distance, in texels, to the nearest pixel whose `solid` is
/// `target` (a large number when there is none).
fn squared_distances(solid: &[bool], width: u32, height: u32, target: bool) -> Vec<f64> {
    // Far beyond any distance in a picture, and small enough that adding a squared
    // coordinate to it stays exact.
    const FAR: f64 = 1.0e10;
    let (w, h) = (width as usize, height as usize);
    let mut grid: Vec<f64> = solid
        .iter()
        .map(|&is| if is == target { 0.0 } else { FAR })
        .collect();
    let longest = w.max(h);
    let mut f = vec![0.0; longest];
    let mut d = vec![0.0; longest];
    let mut v = vec![0_usize; longest];
    let mut z = vec![0.0; longest + 1];
    for x in 0..w {
        for y in 0..h {
            f[y] = grid[y * w + x];
        }
        transform_1d(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        f[..w].copy_from_slice(&grid[y * w..(y + 1) * w]);
        transform_1d(&f[..w], &mut d[..w], &mut v, &mut z);
        grid[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
    grid
}

/// The squared-distance transform of one line, `f` in, `d` out (Felzenszwalb and
/// Huttenlocher, "Distance Transforms of Sampled Functions").
fn transform_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    let mut k = 0;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let at = q as f64;
        let mut s;
        loop {
            let p = v[k] as f64;
            s = ((f[q] + at * at) - (f[v[k]] + p * p)) / (2.0 * (at - p));
            // `z[0]` is minus infinity: the first parabola always ends somewhere.
            if s > z[k] || k == 0 {
                break;
            }
            k -= 1;
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate().take(n) {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let delta = q as f64 - v[k] as f64;
        *out = delta * delta + f[v[k]];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A disc of `radius` texels in the middle of an `edge` square, opaque white.
    fn disc(edge: u32, radius: f32) -> RgbaImage {
        RgbaImage::from_fn(edge, edge, |x, y| {
            let dx = x as f32 + 0.5 - edge as f32 * 0.5;
            let dy = y as f32 + 0.5 - edge as f32 * 0.5;
            if dx.hypot(dy) <= radius {
                image::Rgba([255, 255, 255, 255])
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        })
    }

    /// The outline's value at a point as a signed distance in texels (positive inside).
    fn signed_at(field: &GrayImage, x: u32, y: u32) -> f32 {
        let edge = field.width() as f32;
        (f32::from(field.get_pixel(x, y)[0]) / 255.0 - 0.5) * 2.0 * OUTLINE_REACH * edge
    }

    /// The outline is the distance to the picture's edge: about 0 on it, the disc's radius
    /// less the distance from its middle inside, and growing outside.
    #[test]
    fn the_outline_measures_the_distance_to_the_edge() {
        let edge = 256;
        let field = outline(&disc(edge, 60.0));
        let middle = edge / 2;
        let precision = 2.0 * OUTLINE_REACH * edge as f32 / 255.0;
        for (x, want) in [
            (middle + 20, 40.0),
            (middle + 60, 0.0),
            (middle + 80, -20.0),
        ] {
            let got = signed_at(&field, x, middle);
            assert!(
                (got - want).abs() <= 1.5 + precision,
                "{x}: {got} for {want}"
            );
        }
        // Far inside and far outside, it holds at its reach.
        assert_eq!(field.get_pixel(0, 0)[0], 0);
        let solid = outline(&RgbaImage::from_pixel(64, 64, image::Rgba([9, 9, 9, 255])));
        assert_eq!(solid.get_pixel(32, 32)[0], 255);
    }

    #[test]
    fn the_one_dimensional_transform_finds_the_nearest_zero() {
        let far = 1.0e10;
        let f = [far, 0.0, far, far, far, 0.0, far];
        let mut d = [0.0; 7];
        let mut v = [0; 7];
        let mut z = [0.0; 8];
        transform_1d(&f, &mut d, &mut v, &mut z);
        assert_eq!(d, [1.0, 0.0, 1.0, 4.0, 1.0, 0.0, 1.0]);
    }

    /// A small tag is enlarged to the tag edge, a square one kept square, and a picture
    /// bigger than the largest edge reduced; every chain goes down to one texel.
    #[test]
    fn a_tag_picture_is_enlarged_to_its_edge_and_squared() {
        let png = |image: &RgbaImage| {
            let mut bytes = Vec::new();
            image
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            bytes
        };
        let small = tag_picture(&png(&disc(256, 100.0)), "gfx/tags/a.png", 0).unwrap();
        assert_eq!(small.picture[0].dimensions(), (EDGE_TAG, EDGE_TAG));
        assert_eq!(small.outline[0].dimensions(), (EDGE_TAG, EDGE_TAG));
        assert_eq!(small.picture.last().unwrap().dimensions(), (1, 1));
        assert_eq!(small.outline.last().unwrap().dimensions(), (1, 1));
        // The disc's edge stays clean: no dark fringe where it meets transparency.
        let middle = EDGE_TAG / 2;
        let rim = small.picture[0].get_pixel(middle + 199, middle);
        assert!(rim[3] > 0 && rim[0] > 240, "{rim:?}");
        let halved = tag_picture(&png(&disc(256, 100.0)), "gfx/tags/a.png", 1).unwrap();
        assert_eq!(halved.picture[0].width(), EDGE_TAG / 2);
        let wide = RgbaImage::from_pixel(300, 100, image::Rgba([200, 10, 10, 255]));
        let squared = tag_picture(&png(&wide), "gfx/tags/b.png", 0).unwrap();
        assert_eq!(squared.picture[0].dimensions(), (EDGE_TAG, EDGE_TAG));
        assert_eq!(squared.picture[0].get_pixel(4, 4)[3], 0, "padded above");
        assert_eq!(squared.picture[0].get_pixel(middle, middle)[3], 255);
        let big = RgbaImage::from_pixel(2_000, 2_000, image::Rgba([0, 0, 0, 255]));
        let reduced = tag_picture(&png(&big), "gfx/tags/c.png", 0).unwrap();
        assert_eq!(reduced.picture[0].width(), EDGE_MAX);
        assert!(tag_picture(b"not a picture", "gfx/tags/d.png", 0).is_err());
    }

    /// A card sticker: the picture in the middle, transparent round it, a rim of the
    /// card's colour at its edge, and an outline that is the rounded box's.
    #[test]
    fn a_card_becomes_a_sticker_with_a_rim_of_its_colour() {
        let mut jpeg = Vec::new();
        image::RgbImage::from_pixel(1_020, 850, image::Rgb([20, 200, 40]))
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        let edge = 512;
        let built = sticker(&jpeg, [1.0, 0.2, 0.6], edge).unwrap();
        let canvas = &built.picture[0];
        assert_eq!(canvas.dimensions(), (edge, edge));
        let middle = edge / 2;
        let centre = canvas.get_pixel(middle, middle);
        assert!(
            centre[1] > 150 && centre[0] < 80 && centre[3] == 255,
            "{centre:?}"
        );
        assert_eq!(
            canvas.get_pixel(middle, 4)[3],
            0,
            "clear above the 6:5 picture"
        );
        assert_eq!(canvas.get_pixel(2, 2)[3], 0, "clear in the corner");
        // At the picture's left edge, the rim.
        let box_w = (edge as f32 * STICKER_WIDTH).round() as u32;
        let left = (edge - box_w) / 2;
        let rim = canvas.get_pixel(left + 2, middle);
        assert!(rim[0] > 200 && rim[1] < 120 && rim[3] == 255, "{rim:?}");
        // The outline: inside the box above a half, outside below.
        let field = &built.outline[0];
        assert!(field.get_pixel(middle, middle)[0] > 128);
        assert!(field.get_pixel(middle, 2)[0] < 128);
        assert!(sticker(b"no", [1.0; 3], edge).is_err());
    }

    #[test]
    fn a_placed_tag_draws_its_tag_or_its_card() {
        assert_eq!(Source::of("tag_gg"), Source::Tag("tag_gg".to_owned()));
        assert_eq!(
            Source::of("sjk_alpha_001"),
            Source::Card("sjk_alpha_001".to_owned())
        );
    }
}
