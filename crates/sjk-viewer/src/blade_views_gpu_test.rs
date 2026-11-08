//! Offscreen renders of blades as players see them: side on, from a third-person camera
//! behind and above, and from first person looking along the blade from just behind the
//! hilt (where a blade must start inside the emitter as a stock one does). A stock blade
//! is drawn beside each skin for comparison. With `SJK_VIEWS` set to a folder, each skin's
//! sheet is saved there as `<id>.png` (a row: stock and skin per view); `SJK_VIEW_SKINS`
//! (ids without `saber_`, comma-separated) picks which, else every signature and design.
use wgpu::util::DeviceExt;

const SIZE: u32 = 256;

struct View {
    eye: glam::Vec3,
    target: glam::Vec3,
    /// Vertical field of view (degrees); 0 for the orthographic side view.
    fov: f32,
}

#[test]
#[ignore = "requires an available GPU; uses no installed game assets"]
fn blades_seen_as_players_see_them() {
    let Some(folder) = std::env::var_os("SJK_VIEWS").map(std::path::PathBuf::from) else {
        return;
    };
    let gpu = pollster::block_on(crate::gpu_context::Context::new(None, None)).unwrap();
    let device = &gpu.device;
    let queue = &gpu.queue;
    let visible = wgpu::ShaderStages::VERTEX_FRAGMENT;
    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: visible,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    };
    let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            crate::texture_layout_entry(0),
            crate::texture_layout_entry(1),
            crate::texture_layout_entry(3),
            sampler_entry(2),
            sampler_entry(4),
            crate::texture_layout_entry(5),
            sampler_entry(6),
        ],
    });
    let skins_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: visible,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            Some(&camera_layout),
            Some(&material_layout),
            Some(&skins_layout),
        ],
        immediate_size: 0,
    });
    let pipeline =
        |module: &wgpu::ShaderModule, vertex: &str, fragment: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[Some(crate::saber::Instance::layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
    let additive = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent::OVER,
    };
    let pipelines = |source: String| {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        (
            pipeline(&module, "vertex_main", "fragment_main", additive),
            pipeline(
                &module,
                "vertex_solid",
                "fragment_solid",
                wgpu::BlendState::ALPHA_BLENDING,
            ),
        )
    };
    let shared = pipelines(include_str!("saber.wgsl").to_owned());
    let samplers = crate::saber::Samplers::new(device);
    // The blade from the hilt at the origin, leaning away from the first-person eye.
    let direction = glam::Vec3::new(0.0, 0.62, 0.78).normalize();
    let base = glam::Vec3::ZERO;
    let tip = base + direction * 40.0;
    let views = [
        View {
            eye: base + direction * 20.0 + glam::Vec3::new(-100.0, 0.0, 0.0),
            target: base + direction * 20.0,
            fov: 0.0,
        },
        View {
            eye: glam::Vec3::new(-40.0, -70.0, 30.0),
            target: (base + tip) * 0.5,
            fov: 60.0,
        },
        View {
            eye: base + glam::Vec3::new(3.0, -12.0, 6.0),
            target: base + direction * 14.0,
            fov: 80.0,
        },
        View {
            eye: base + glam::Vec3::new(1.5, -6.0, 2.5),
            target: base + direction * 8.0,
            fov: 80.0,
        },
    ];
    let wanted: Option<Vec<String>> = std::env::var("SJK_VIEW_SKINS").ok().map(|ids| {
        ids.split(',')
            .map(|id| format!("saber_{}", id.trim()))
            .collect()
    });
    let ids: Vec<&'static str> = crate::blade_signatures::SIGNATURES
        .iter()
        .map(|signature| signature.id)
        .chain(
            crate::blade_lab_designs::DESIGNS
                .iter()
                .map(|design| design.id),
        )
        .filter(|id| {
            wanted
                .as_ref()
                .is_none_or(|wanted| wanted.iter().any(|w| w == id))
        })
        .collect();
    // Each skin from its built-in file, or the test sample for the hand-ported designs.
    let skins = crate::saber_skins::LoadedSkins::of(
        ids.iter()
            .map(|&id| {
                let text = crate::blade_signatures::of(id)
                    .map_or(crate::blade_skin_file::tests::SAMPLE, |s| s.default_file());
                let def = crate::blade_skin_file::parse(id, text).unwrap();
                crate::saber_skins::LoadedSkin::new(id, def, &sjk_vfs::VirtualFileSystem::new())
                    .unwrap()
            })
            .collect(),
        1,
    );
    let skins_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&skins.uniforms()),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let skins_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &skins_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: skins_buffer.as_entire_binding(),
        }],
    });
    // The stock blade's materials (the neutral pair, generated).
    let stock_material = crate::saber::material(
        device,
        queue,
        &material_layout,
        &samplers,
        &image::RgbaImage::from_raw(
            crate::saber_rgb::glow_size()[0],
            crate::saber_rgb::glow_size()[1],
            crate::saber_rgb::generated_glow(crate::saber_rgb::GlowProfile::NEUTRAL),
        )
        .unwrap(),
        &image::RgbaImage::from_raw(
            crate::saber_rgb::core_size()[0],
            crate::saber_rgb::core_size()[1],
            crate::saber_rgb::generated_core(crate::saber_rgb::CoreProfile::NEUTRAL),
        )
        .unwrap(),
    );
    let render = |view: &View,
                  pair: [crate::saber::Instance; 2],
                  material: &wgpu::BindGroup,
                  pipes: &(wgpu::RenderPipeline, wgpu::RenderPipeline)| {
        let aspect = 1.0;
        let projection = if view.fov == 0.0 {
            glam::camera::rh::proj::directx::orthographic(-26., 26., -26., 26., 0.1, 300.)
        } else {
            glam::camera::rh::proj::directx::perspective(view.fov.to_radians(), aspect, 0.5, 400.0)
        };
        let matrix =
            projection * glam::camera::rh::view::look_at_mat4(view.eye, view.target, glam::Vec3::Z);
        let mut camera = matrix.to_cols_array().to_vec();
        camera.extend_from_slice(&[view.eye.x, view.eye.y, view.eye.z, 0.0]);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&camera),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&pair),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.06,
                            b: 0.06,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &camera_bind, &[]);
            pass.set_bind_group(1, material, &[]);
            pass.set_bind_group(2, &skins_bind, &[]);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_pipeline(&pipes.0);
            pass.draw(0..6, 0..2);
            pass.set_pipeline(&pipes.1);
            pass.draw(0..6, 0..1);
        }
        queue.submit([encoder.finish()]);
        let mut frame = crate::world_shot::read_back(device, queue, &target);
        // The hilt's place: a grey mark where the blade leaves it.
        let clip = matrix * base.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        let (x, y) = (
            ((ndc.x * 0.5 + 0.5) * SIZE as f32) as i64,
            ((0.5 - ndc.y * 0.5) * SIZE as f32) as i64,
        );
        for (dx, dy) in [(-3, 0), (-2, 0), (2, 0), (3, 0), (0, 2), (0, 3)] {
            let (px, py) = (x + dx, y + dy);
            if (0..SIZE as i64).contains(&px) && (0..SIZE as i64).contains(&py) {
                frame.put_pixel(px as u32, py as u32, image::Rgba([255, 0, 255, 255]));
            }
        }
        frame
    };
    let blade = crate::saber::Blade {
        base: base.to_array(),
        direction: direction.to_array(),
        length: 40.0,
        radius: 3.0,
    };
    for (index, (_, skin)) in skins.iter().enumerate() {
        let id = ids[index];
        let own = skin
            .signature
            .map(|signature| pipelines(signature.module_source()));
        let pipes = own.as_ref().unwrap_or(&shared);
        let material = crate::saber::material(
            device,
            queue,
            &material_layout,
            &samplers,
            &skin.glow,
            &skin.core,
        );
        let color = skins.color_of(id).unwrap();
        let stock = crate::saber::Instance::pair(
            blade,
            crate::saber_rgb::BladeColor::from_rgb([40, 120, 255]),
        )
        .map(|i| i.with_animation(3.0, 7));
        // `SJK_VIEW_AT` ("seconds,swing,since clash,age,since swing") draws the skin at that
        // moment instead of still at 3 seconds (round 19: the Clone Wars blade's swing and
        // clash effects).
        let (seconds, motion) =
            std::env::var("SJK_VIEW_AT")
                .ok()
                .map_or((3.0, crate::saber_motion::STILL), |at| {
                    let v: Vec<f32> = at.split(',').map(|x| x.trim().parse().unwrap()).collect();
                    (f64::from(v[0]), [v[1], v[2], v[3], v[4]])
                });
        let skinned =
            crate::saber::Instance::pair(blade, crate::saber_rgb::BladeColor::Skin(color))
                .map(|i| i.with_animation(seconds, 7).with_motion(motion));
        let mut sheet = image::RgbaImage::new(SIZE * views.len() as u32 * 2, SIZE);
        for (column, view) in views.iter().enumerate() {
            let a = render(view, stock, &stock_material, &shared);
            let b = render(view, skinned, &material, pipes);
            image::imageops::overlay(&mut sheet, &a, i64::from(SIZE) * (column as i64 * 2), 0);
            image::imageops::overlay(&mut sheet, &b, i64::from(SIZE) * (column as i64 * 2 + 1), 0);
        }
        sheet.save(folder.join(format!("{id}.png"))).unwrap();
    }
}
