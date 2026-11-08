//! The Mirror effect's flip of the whole picture left to right (`docs/wheel-of-doom.md`):
//! once the frame is drawn, the window's image is copied into a texture of the same
//! size and drawn back mirrored over itself. A mirrored projection would turn every
//! triangle's winding and the culling inside out, so the image is flipped instead, as
//! the floor mirrors flip theirs. The texture is made the first frame it is needed (and
//! again on a resize), never every frame; without a surface that can be copied from
//! (its `COPY_SRC`), the picture stays as it is and the controls alone are mirrored.

use crate::GpuState;

/// The flip's pipeline, its texture and the size and format they were made for.
#[derive(Default)]
pub(crate) struct Mirror {
    pass: Option<Pass>,
}

struct Pass {
    pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    bind: wgpu::BindGroup,
    size: [u32; 2],
    format: wgpu::TextureFormat,
}

const SHADER: &str = r"
@group(0) @binding(0) var source: texture_2d<f32>;

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(positions[index], 0.0, 1.0);
}

@fragment fn fragment(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(source));
    let x = size.x - 1 - i32(at.x);
    return textureLoad(source, vec2(clamp(x, 0, size.x - 1), i32(at.y)), 0);
}
";

impl Pass {
    fn new(device: &wgpu::Device, size: [u32; 2], format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SJK WHEEL OF DOOM mirror"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK WHEEL OF DOOM mirror"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK WHEEL OF DOOM mirror"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK WHEEL OF DOOM mirror"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        Self {
            pipeline,
            texture,
            bind,
            size,
            format,
        }
    }
}

impl GpuState {
    /// Flip this frame's finished picture left to right while the Mirror effect runs:
    /// `frame` the window's image, `view` its default view.
    pub(crate) fn encode_wheel_of_doom_mirror(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: Option<&wgpu::SurfaceTexture>,
        view: &wgpu::TextureView,
    ) {
        if !self
            .wheel_of_doom
            .effects
            .mirrored(std::time::Instant::now())
        {
            return;
        }
        let Some(frame) = frame else {
            return;
        };
        if !self
            .configuration
            .usage
            .contains(wgpu::TextureUsages::COPY_SRC)
        {
            return;
        }
        let texture = &frame.texture;
        let size = [texture.width(), texture.height()];
        let format = texture.format();
        let mirror = &mut self.wheel_of_doom.mirror;
        if mirror
            .pass
            .as_ref()
            .is_none_or(|pass| pass.size != size || pass.format != format)
        {
            mirror.pass = Some(Pass::new(&self.device, size, format));
        }
        let Some(pass) = mirror.pass.as_ref() else {
            return;
        };
        encoder.copy_texture_to_texture(
            texture.as_image_copy(),
            pass.texture.as_image_copy(),
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
        let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SJK WHEEL OF DOOM mirror"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        render.set_pipeline(&pass.pipeline);
        render.set_bind_group(0, &pass.bind, &[]);
        render.draw(0..3, 0..1);
    }

    /// Let the mirror's texture go once the effect is over.
    pub(crate) fn release_wheel_of_doom_mirror(&mut self) {
        if !self
            .wheel_of_doom
            .effects
            .mirrored(std::time::Instant::now())
        {
            self.wheel_of_doom.mirror.pass = None;
        }
    }
}
