//! An offscreen render of every signature skin through its own pipelines (`saber.wgsl`
//! with its signature) and its real glow and core images, on a dark ground: still, mid
//! swing, just after a clash, while it ignites and, for a chroma, worn green. Each must
//! light its blade and leave the picture's corners dark (nothing reaching the quad's
//! edge). With `SJK_SIGNATURE_PREVIEW` set to a folder, each sheet is saved there as
//! `<id>.png`; `SJK_SIGNATURES` (ids, comma-separated) renders only those, and
//! `SJK_SIGNATURE_ORBIT` set renders five views from a camera moving round it instead (to check
//! that its effects stay with the blade as the view changes; `tip` looks down the blade's length
//! instead, at its tip); `SJK_SIGNATURE_MORE` set adds
//! three moments: just after it is out, half a second after a clash and while it goes out;
//! `SJK_SIGNATURE_MOTION=<frames>,<step>` renders frames a fixed step apart.
use super::*;
use wgpu::util::DeviceExt;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 300;

#[test]
#[ignore = "requires an available GPU; uses no installed game assets"]
fn every_signature_draws_its_blade_and_nothing_at_the_edge() {
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
    let volume_depth = crate::saber_gpu::volume::depth_layout(device);
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
    // The blade seen side on, its 40 units filling most of the picture's height, with
    // room for 8 units of effects round it.
    let matrix = glam::camera::rh::proj::directx::orthographic(-24., 24., -28., 32., 0.1, 200.)
        * glam::camera::rh::view::look_at_mat4(
            glam::Vec3::new(0., -100., 0.),
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        );
    let mut camera = matrix.to_cols_array().to_vec();
    // Round 20: with `SJK_SIGNATURE_PICTURE` set, the rays come from far off a little toward
    // the tip, as the Collection's pictures are drawn (`blade_pictures.rs`).
    let picture = std::env::var_os("SJK_SIGNATURE_PICTURE").is_some();
    if picture {
        camera.extend_from_slice(&[0., -20_000., 4_000., 0.]);
    } else {
        camera.extend_from_slice(&[0., -100., 0., 0.]);
    }
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
    let samplers = crate::saber::Samplers::new(device);
    let save = std::env::var_os("SJK_SIGNATURE_PREVIEW").map(std::path::PathBuf::from);
    let only: Option<Vec<String>> = std::env::var("SJK_SIGNATURES").ok().map(|ids| {
        ids.split(',')
            .map(|id| format!("saber_{}", id.trim()))
            .collect()
    });
    // Every signature skin, from its built-in file.
    let skins = crate::saber_skins::LoadedSkins::of(
        SIGNATURES
            .iter()
            .map(|signature| {
                let def =
                    crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
                crate::saber_skins::LoadedSkin::new(
                    signature.id,
                    def,
                    &sjk_vfs::VirtualFileSystem::new(),
                )
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
    for (_, skin) in skins.iter() {
        let signature = skin.signature.unwrap();
        if only
            .as_ref()
            .is_some_and(|only| !only.iter().any(|id| id == signature.id))
        {
            continue;
        }
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(signature.id),
            source: wgpu::ShaderSource::Wgsl(signature.module_source().into()),
        });
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let glow = pipeline(&module, "vertex_main", "fragment_main", additive);
        // Round 19: with `SJK_SIGNATURE_EDGES` set, each moment is drawn again without the
        // quad's edge fade (`sig_edge`), and where the two differ much an effect reached the
        // quad's edge, where it would be faded or cut.
        let edges = std::env::var_os("SJK_SIGNATURE_EDGES").is_some();
        let unfaded = edges.then(|| {
            let source = signature.module_source();
            let head = "fn sig_edge(input: VertexOutput) -> f32 {";
            let at = source.find(head).unwrap() + head.len();
            let source = format!(
                "{}\n    return 1.0;\n}}\nfn sig_edge_unused(input: VertexOutput) -> f32 {{{}",
                &source[..at],
                &source[at..]
            );
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(signature.id),
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
        });
        let solid = pipeline(
            &module,
            "vertex_solid",
            "fragment_solid",
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let material = crate::saber::material(
            device,
            queue,
            &material_layout,
            &samplers,
            &skin.glow,
            &skin.core,
        );
        let color = skins.color_of(signature.id).unwrap();
        let mut moments = vec![
            // Still, mid swing, just after a clash, while it ignites.
            (3.0, crate::saber_motion::STILL, color),
            (5.4, [0.9, 1_000.0, 1_000.0, 0.25], color),
            (24.0, [0.0, 0.1, 1_000.0, 1_000.0], color),
            (11.0, [0.0, 1_000.0, 0.2, 1_000.0], color),
        ];
        if std::env::var_os("SJK_SIGNATURE_MORE").is_some() {
            moments.push((12.0, [0.0, 1_000.0, 0.7, 1_000.0], color));
            moments.push((24.0, [0.0, 0.25, 1_000.0, 1_000.0], color));
            let going_out = crate::saber_motion::GOING_OUT + 0.2;
            moments.push((30.0, [0.0, 1_000.0, going_out, 1_000.0], color));
        }
        if crate::unlockables::is_chroma(signature.id) {
            let green = color.worn_with(crate::saber_rgb::BladeColor::from_rgb([40, 255, 60]));
            moments.push((3.0, crate::saber_motion::STILL, green));
        }
        if edges {
            // Many moments over a whole cycle, still, swinging and just after a clash.
            let swing = [0.9, 1_000.0, 1_000.0, 0.25];
            let clash = [0.0, 0.1, 1_000.0, 1_000.0];
            moments = [2.5f32, 12.3, 21.7, 26.6, 27.2, 41.5]
                .iter()
                .enumerate()
                .map(|(i, &t)| {
                    let motion = [crate::saber_motion::STILL, swing, clash][i % 3];
                    (f64::from(t), motion, color)
                })
                .collect();
        }
        if picture {
            moments.truncate(1);
        }
        // `SJK_SIGNATURE_MOTION=<frames>,<step>`: frames a fixed step apart, to judge its motion.
        if let Ok(spec) = std::env::var("SJK_SIGNATURE_MOTION") {
            let (frames, step) = spec.split_once(',').unwrap();
            let step: f64 = step.parse().unwrap();
            moments = (0..frames.parse::<u32>().unwrap())
                .map(|i| {
                    let seconds = 6.0 + f64::from(i) * step;
                    (seconds, crate::saber_motion::STILL, color)
                })
                .collect();
        }
        let orbit = std::env::var("SJK_SIGNATURE_ORBIT").ok();
        if orbit.is_some() {
            let at = std::env::var("SJK_SIGNATURE_AT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3.0);
            moments = vec![(at, crate::saber_motion::STILL, color); 5];
        }
        let mut frames = Vec::new();
        for (index, (seconds, motion, color)) in moments.into_iter().enumerate() {
            let camera_bind = if orbit.is_some() {
                // Round 19: from further away, the last two from high above (Creyon looked
                // down on the blades and saw flat effects following the camera).
                let yaw = [0.0f32, 0.6, 1.2, 1.8, 0.3][index];
                let lift = [0.0f32, 12.0, 0.0, 45.0, 85.0][index];
                let reach = [52.0f32, 52.0, 52.0, 52.0, 28.0][index];
                let mut eye = glam::Vec3::new(reach * yaw.sin(), -reach * yaw.cos(), lift);
                let mut at = glam::Vec3::ZERO;
                // Round 23 review (`SJK_SIGNATURE_ORBIT=tip`): down the blade's length, where
                // players saw a box round the tip (Obsidian Edge): the first three from just
                // beyond the tip looking back at it, the last two from behind the hilt.
                if orbit.as_deref() == Some("close") {
                    // Update 20: from where a player holds it, as in Creyon's videos.
                    let yaw = [0.0f32, 0.5, 1.0, 1.6, 2.2][index];
                    eye = glam::Vec3::new(9.0 * yaw.sin(), -9.0 * yaw.cos(), -6.0);
                    at = glam::Vec3::new(0.0, 0.0, 2.0);
                }
                // Update 28: the point from the side, near, to check every blade's end.
                if orbit.as_deref() == Some("point") {
                    let yaw = [0.0f32, 0.9, 1.8, 2.7, 3.6][index];
                    eye = glam::Vec3::new(14.0 * yaw.sin(), -14.0 * yaw.cos(), 14.0);
                    at = glam::Vec3::new(0.0, 0.0, 14.0);
                }
                if orbit.as_deref() == Some("tip") {
                    let aside = [3.0f32, 1.5, 0.6, 4.0, 1.0][index];
                    let (height, look) = if index < 3 {
                        (18.0 + [26.0, 10.0, 5.0][index], 18.0)
                    } else {
                        (-22.0 - 14.0, 18.0)
                    };
                    eye = glam::Vec3::new(aside, -aside * 0.5, height);
                    at = glam::Vec3::new(0.0, 0.0, look);
                }
                let mut fov = 0.75f32;
                if orbit.as_deref() == Some("fp") {
                    let aside = [10.0f32, 16.0, 22.0, 8.0, 5.0][index];
                    eye = glam::Vec3::new(
                        aside,
                        -aside * 0.6,
                        -40.0 - [0.0f32, 4.0, 8.0, 0.0, 0.0][index],
                    );
                    at = glam::Vec3::new(0.0, 0.0, 8.0);
                    fov = 0.62;
                }
                let m = glam::camera::rh::proj::directx::perspective(
                    fov,
                    WIDTH as f32 / HEIGHT as f32,
                    1.0,
                    400.0,
                ) * glam::camera::rh::view::look_at_mat4(eye, at, glam::Vec3::Z);
                let mut cam = m.to_cols_array().to_vec();
                cam.extend_from_slice(&[eye.x, eye.y, eye.z, 0.0]);
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&cam),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &camera_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            } else {
                camera_bind.clone()
            };
            let pair = crate::saber::Instance::pair(
                crate::saber::Blade {
                    base: [0.0, 0.0, -22.0],
                    direction: [0.0, 0.0, 1.0],
                    length: 40.0,
                    radius: 3.0,
                },
                crate::saber_rgb::BladeColor::Skin(color),
            )
            .map(|i| {
                let i = i.with_animation(seconds, 7).with_motion(motion);
                if picture {
                    i.with_frame([0.0, 1.0, 0.0], 40.0)
                } else {
                    i
                }
            });
            let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pair),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.02,
                                g: 0.025,
                                b: 0.035,
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
                pass.set_bind_group(1, &material, &[]);
                pass.set_bind_group(2, &skins_bind, &[]);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_pipeline(&glow);
                pass.draw(0..6, 0..2);
                pass.set_pipeline(&solid);
                pass.draw(0..6, 0..1);
            }
            queue.submit([encoder.finish()]);
            let faded = crate::world_shot::read_back(device, queue, &target);
            if let Some((glow2, solid2)) = &unfaded {
                let mut encoder = device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.02,
                                    g: 0.025,
                                    b: 0.035,
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
                    pass.set_bind_group(1, &material, &[]);
                    pass.set_bind_group(2, &skins_bind, &[]);
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    pass.set_pipeline(glow2);
                    pass.draw(0..6, 0..2);
                    pass.set_pipeline(solid2);
                    pass.draw(0..6, 0..1);
                }
                queue.submit([encoder.finish()]);
                let full = crate::world_shot::read_back(device, queue, &target);
                let mut cut = 0u32;
                let mut worst = 0i32;
                let mut marked = faded.clone();
                for (x, y, p) in full.enumerate_pixels() {
                    let q = faded.get_pixel(x, y);
                    let d = (0..3)
                        .map(|c| i32::from(p.0[c]) - i32::from(q.0[c]))
                        .max()
                        .unwrap();
                    if d > 40 {
                        cut += 1;
                        marked.put_pixel(x, y, image::Rgba([255, 0, 255, 255]));
                    }
                    worst = worst.max(d);
                }
                println!(
                    "EDGE {} moment {index}: {cut} pixels cut, worst {worst}",
                    signature.id
                );
                frames.push(marked);
                continue;
            }
            frames.push(faded);
        }
        for frame in frames.iter().filter(|_| orbit.is_none() && !edges) {
            let lit = (HEIGHT / 4..HEIGHT * 3 / 4)
                .flat_map(|y| (WIDTH / 2 - 12..WIDTH / 2 + 12).map(move |x| (x, y)))
                .map(|(x, y)| {
                    frame.get_pixel(x, y).0[..3]
                        .iter()
                        .map(|&c| u32::from(c))
                        .sum::<u32>()
                })
                .max()
                .unwrap_or(0);
            assert!(lit > 120, "{}: the blade is not drawn", signature.id);
            for corner in [
                (1, 1),
                (WIDTH - 2, 1),
                (1, HEIGHT - 2),
                (WIDTH - 2, HEIGHT - 2),
            ] {
                let pixel = frame.get_pixel(corner.0, corner.1).0;
                assert!(
                    pixel[..3].iter().all(|&c| c < 24),
                    "{}: light at the picture's corner ({pixel:?})",
                    signature.id
                );
            }
        }
        if let Some(folder) = &save {
            let mut sheet = image::RgbaImage::new(WIDTH * frames.len() as u32, HEIGHT);
            for (index, frame) in frames.iter().enumerate() {
                image::imageops::overlay(&mut sheet, frame, i64::from(WIDTH) * index as i64, 0);
            }
            sheet
                .save(folder.join(format!("{}.png", signature.id)))
                .unwrap();
        }
    }
}
