//! The puddles' mirror (`r_weatherQuality` 3, ultra): puddles reflect the world round
//! them, not only the sky. Right after the opaque world, before the wet pass, the scene
//! is copied at half size ([`Mirror::encode_copy`]); the wet pass marches each puddle
//! pixel's mirrored ray through the depth and reads that copy where it meets the world
//! (`mirrored` in weather.wgsl, after VisionJK's `Mirror()` in `wet.frag` by Blackwolf:
//! 32 steps from 2 units growing by 1.18, then five halvings).
//!
//! Since 09/11/2026 every wet surface can mirror the world (`r_weatherReflections`): the
//! copy then carries a chain of smaller, softer levels ([`Mirror::encode_blur`], each half
//! the last), and a rough wet street reads a soft level where a puddle reads the sharp one.
//!
//! The copy needs a scene target it can sample: the render-scale or post-AA scene (every
//! preset draws to one). Drawn straight to the window, the puddles keep the sky alone.
//! Players are drawn after the wet pass, so puddles mirror the world, not the players.

/// The copy's levels at most: half the scene's size down to a sixteenth.
const LEVELS: u32 = 4;

/// The copy at half the scene's size with its softer levels, and the wet pass's bind
/// group of them all.
struct Target {
    size: [u32; 2],
    bind_group: wgpu::BindGroup,
    /// Each level alone: what its pass draws into.
    levels: Vec<wgpu::TextureView>,
    /// Each level but the last, for the pass that makes the next from it.
    sources: Vec<wgpu::BindGroup>,
}

/// The mirror's pipeline, its copy and the bind groups the passes read.
pub(crate) struct Mirror {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    copy: wgpu::RenderPipeline,
    format: wgpu::TextureFormat,
    target: Option<Target>,
    /// The scene the copy pass reads, with its bind group: made again only when the
    /// scene target is (a resize, a graphics reload).
    source: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    /// What the wet pass binds while there is no copy: one black texel.
    empty: wgpu::BindGroup,
}

impl Mirror {
    /// The layout (group 3 of the weather shader), the copy's pipeline from `shader`
    /// into `format` (the scene's), and the stand-in.
    pub(crate) fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK puddle mirror layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SJK puddle mirror sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK puddle mirror copy layout"),
            bind_group_layouts: &[None, None, None, Some(&layout)],
            immediate_size: 0,
        });
        let copy = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fragment_mirror_copy"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vertex_volume"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fragment_mirror_copy"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let black = texture(device, format, [1, 1], 1).create_view(&Default::default());
        let empty = bind_group(device, &layout, &black, &sampler);
        Self {
            layout,
            sampler,
            copy,
            format,
            target: None,
            source: None,
            empty,
        }
    }

    /// A bind group of `view` with the linear sampler, in the layout of group 3 (the
    /// fog's half-size march uses it too).
    pub(crate) fn bind_group_of(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        bind_group(device, &self.layout, view, &self.sampler)
    }

    /// The layout of group 3, for the wet pass's pipeline.
    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Get ready to copy `scene` this frame: the copy at its half size, and the copy
    /// pass's bind group of it, each made again only when they change.
    pub(crate) fn prepare(&mut self, device: &wgpu::Device, scene: &wgpu::TextureView) {
        let full = scene.texture().size();
        let size = [full.width.div_ceil(2), full.height.div_ceil(2)];
        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size != size)
        {
            let count = LEVELS.min(size[0].max(size[1]).max(1).ilog2() + 1);
            let texture = texture(device, self.format, size, count);
            let levels: Vec<_> = (0..count)
                .map(|level| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        base_mip_level: level,
                        mip_level_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect();
            let sources = levels[..levels.len() - 1]
                .iter()
                .map(|view| bind_group(device, &self.layout, view, &self.sampler))
                .collect();
            let whole = texture.create_view(&Default::default());
            self.target = Some(Target {
                size,
                bind_group: bind_group(device, &self.layout, &whole, &self.sampler),
                levels,
                sources,
            });
        }
        if self.source.as_ref().is_none_or(|(view, _)| view != scene) {
            let group = bind_group(device, &self.layout, scene, &self.sampler);
            self.source = Some((scene.clone(), group));
        }
    }

    /// Drop the copy (puddles are off): its memory goes back.
    pub(crate) fn release(&mut self) {
        self.target = None;
        self.source = None;
    }

    /// Copy the scene at half size, when [`Self::prepare`] got it ready; answers whether
    /// it did.
    pub(crate) fn encode_copy(&self, encoder: &mut wgpu::CommandEncoder) -> bool {
        let (Some(target), Some((_, source))) = (&self.target, &self.source) else {
            return false;
        };
        self.halve(encoder, source, &target.levels[0]);
        true
    }

    /// Make the copy's softer levels, each from the one before, after
    /// [`Self::encode_copy`].
    pub(crate) fn encode_blur(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(target) = &self.target else {
            return;
        };
        for (source, level) in target.sources.iter().zip(&target.levels[1..]) {
            self.halve(encoder, source, level);
        }
    }

    /// Draw `source` at half its size into `view`.
    fn halve(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::BindGroup,
        view: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SJK puddle mirror copy"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.copy);
        pass.set_bind_group(3, source, &[]);
        pass.draw(0..3, 0..1);
    }

    /// What the wet pass binds as group 3: the copy, or the stand-in.
    pub(crate) fn bind_group(&self, copied: bool) -> &wgpu::BindGroup {
        match &self.target {
            Some(target) if copied => &target.bind_group,
            _ => &self.empty,
        }
    }
}

fn texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    levels: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SJK puddle mirror"),
        size: wgpu::Extent3d {
            width: size[0].max(1),
            height: size[1].max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("SJK puddle mirror bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
