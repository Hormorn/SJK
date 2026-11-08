//! A signature blade's effects in the air, drawn at half resolution.
//!
//! A signature's `sig_vol` traces every floating piece along the view ray of every pixel its
//! quad covers. Close to the camera that quad covers much of the screen, and at 3440x1440 one
//! blade cost more than the frame had (Creyon, 4/11/2026: 125 FPS falling to 25 near Dromund
//! Kaas). So in a view the effects go into a half-size layer first, a quarter of the pixels,
//! tested against the view's depth, and are added back after the blades' main pass. The blade's
//! body, core and glow stay at full size in the main pass (`SIG_SPLIT` in `saber.wgsl`).
use std::sync::Arc;

/// Floats, so faint mist does not band when it is scaled up.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The view's depth for `fragment_volume` (group 3, binding 2).
pub(crate) fn depth_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SJK saber volume depth layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    })
}

/// The half-size layer as `composite` reads it.
pub(crate) fn read_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SJK saber volume read layout"),
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
    })
}

/// A signature's pipeline into the half-size layer: no depth attachment, it reads the depth.
pub(crate) fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    label: &str,
    module: &wgpu::ShaderModule,
    instance: wgpu::VertexBufferLayout<'_>,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vertex_volume"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(instance)],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fragment_volume"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: FORMAT,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache,
    })
}

const COMPOSITE: &str = r"
@group(0) @binding(0) var layer: texture_2d<f32>;
@group(0) @binding(1) var layer_sampler: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex
fn vertex_all(@builtin(vertex_index) index: u32) -> Out {
    let uv = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Out;
    out.position = vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}
@fragment
fn fragment_all(input: Out) -> @location(0) vec4<f32> {
    // Catmull-Rom from nine taps: it keeps a small piece as sharp as the half-size layer allows
    // (Creyon: the blades were blurry from afar; the smoother cubic this replaces blurred it).
    let size = vec2<f32>(textureDimensions(layer));
    let p = input.uv * size - 0.5;
    let i = floor(p);
    let f = p - i;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let a = (i - 1.0 + 0.5) / size;
    let m = (i + w2 / w12 + 0.5) / size;
    let b = (i + 2.0 + 0.5) / size;
    let sum = w0.x * w0.y * textureSampleLevel(layer, layer_sampler, a, 0.0)
        + w12.x * w0.y * textureSampleLevel(layer, layer_sampler, vec2(m.x, a.y), 0.0)
        + w3.x * w0.y * textureSampleLevel(layer, layer_sampler, vec2(b.x, a.y), 0.0)
        + w0.x * w12.y * textureSampleLevel(layer, layer_sampler, vec2(a.x, m.y), 0.0)
        + w12.x * w12.y * textureSampleLevel(layer, layer_sampler, m, 0.0)
        + w3.x * w12.y * textureSampleLevel(layer, layer_sampler, vec2(b.x, m.y), 0.0)
        + w0.x * w3.y * textureSampleLevel(layer, layer_sampler, vec2(a.x, b.y), 0.0)
        + w12.x * w3.y * textureSampleLevel(layer, layer_sampler, vec2(m.x, b.y), 0.0)
        + w3.x * w3.y * textureSampleLevel(layer, layer_sampler, b, 0.0);
    // Its small undershoot must not darken what lies under it.
    return max(sum, vec4(0.0));
}
";

/// The pipeline adding the half-size layer to `format`'s target, over everything: the layer was
/// depth-tested when it was drawn. `depth` when the pass it draws in has a depth attachment.
pub(crate) fn composite_pipeline(
    device: &wgpu::Device,
    read: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    depth: bool,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SJK saber volume composite"),
        source: wgpu::ShaderSource::Wgsl(COMPOSITE.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SJK saber volume composite layout"),
        bind_group_layouts: &[Some(read)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("SJK saber volume composite"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex_all"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fragment_all"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                // Light adds; a cloud (`uAirCover`) also hides what is behind it.
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                }),
                write_mask: wgpu::ColorWrites::COLOR,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: depth.then_some(wgpu::DepthStencilState {
            format: crate::DepthTarget::FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// The half-size layer of a view of `size`.
pub(crate) struct Target {
    pub(crate) size: [u32; 2],
    pub(crate) view: wgpu::TextureView,
    pub(crate) read: wgpu::BindGroup,
}

impl Target {
    pub(crate) fn new(
        device: &wgpu::Device,
        read: &wgpu::BindGroupLayout,
        size: [u32; 2],
    ) -> Arc<Self> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SJK saber volume layer"),
            size: wgpu::Extent3d {
                width: size[0].div_ceil(2).max(1),
                height: size[1].div_ceil(2).max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SJK saber volume sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK saber volume layer"),
            layout: read,
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
        Arc::new(Self {
            size,
            view,
            read: bind,
        })
    }
}
