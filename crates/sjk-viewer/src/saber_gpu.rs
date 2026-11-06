//! Production WGPU resources for saber blades and motion trails.

use crate::saber::{self, Instance};
use crate::saber_skins::{LoadedSkins, MAX_SKINS, SkinUniform};
use crate::saber_trail::SegmentPool;
use crate::{DepthTarget, texture_layout_entry};
use sjk_shader::ShaderCatalog;
use sjk_vfs::VirtualFileSystem;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Range;
use std::sync::{Arc, Mutex, OnceLock};

#[path = "saber_glow_integral.rs"]
pub(crate) mod glow_integral;
#[path = "saber_volume.rs"]
pub(crate) mod volume;

pub(crate) struct Runtime {
    pipeline: wgpu::RenderPipeline,
    black_pipeline: wgpu::RenderPipeline,
    /// The glow capsules alone, for the dynamic glow image: the retail `*_glow` blade
    /// shaders carry `glow`, the `*_line` cores do not (`sabers.shader`).
    glow_pipeline: wgpu::RenderPipeline,
    materials: Vec<wgpu::BindGroup>,
    /// What a skin slot holds until a blade draws its skin.
    neutral: wgpu::BindGroup,
    /// The loaded skins, whose pairs are uploaded only once a blade draws them
    /// ([`Self::upload_drawn`]), and which slots have theirs.
    loaded: Arc<LoadedSkins>,
    uploaded: [bool; MAX_SKINS],
    device: wgpu::Device,
    /// What a blade-skin slot's pair is made with when a skin loads.
    texture_layout: wgpu::BindGroupLayout,
    samplers: saber::Samplers,
    /// The loaded blade skins' parameters (`saber.wgsl` `skins`), written only when
    /// skins load.
    skins_buffer: wgpu::Buffer,
    skins: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    /// The half-size layer of the signatures' effects (`volume`).
    volume_depth: wgpu::BindGroupLayout,
    volume_read: wgpu::BindGroupLayout,
    volume_composite: wgpu::RenderPipeline,
    volume_targets: std::cell::RefCell<Vec<std::sync::Arc<volume::Target>>>,
    volume_now: std::cell::RefCell<Option<std::sync::Arc<volume::Target>>>,
    ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    trails: crate::saber_trail_gpu::Runtime,
    /// Blade skins' smoke ribbons (`saber_ribbons.rs`), drawn after the blades.
    ribbons: crate::saber_ribbons::Gpu,
    /// The classic model preview's blades, kept apart from the world's.
    preview_buffer: wgpu::Buffer,
    preview_ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    /// The Collection's blade pictures (`blade_pictures.rs`), kept apart from the others.
    pictures_buffer: wgpu::Buffer,
    pictures_ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    /// Skins' heat haze (`saber.wgsl` `fragment_haze`): drawn first into the effect layer,
    /// replacing its pixels with the scene before effects, bent.
    haze_pipeline: wgpu::RenderPipeline,
    haze_layout: wgpu::BindGroupLayout,
    haze_sampler: wgpu::Sampler,
    /// The effect layer's scene image bound for the haze, made again only when the layer
    /// is rebuilt (a resize).
    haze_source: std::cell::RefCell<Option<(wgpu::TextureView, wgpu::BindGroup)>>,
    /// Whether a loaded skin has haze at all.
    any_haze: bool,
    /// Skins' black holes: their lens (`saber.wgsl` `fragment_lens`), drawn with the haze
    /// and blended over it, and their horizons (`fragment_shade`), blended black over
    /// the blades' glow after it.
    lens_pipeline: wgpu::RenderPipeline,
    shade_pipeline: wgpu::RenderPipeline,
    /// Whether a loaded skin has black holes at all.
    any_holes: bool,
    /// Skins' solid parts (`saber.wgsl` `fragment_solid`: ink and chrome), blended over the
    /// blades just drawn, and whether a loaded skin has any.
    solid_pipeline: wgpu::RenderPipeline,
    any_solids: bool,
    /// The signature builds waiting and their builders (`BuildQueue`), the context's.
    builds: Arc<BuildQueue>,
    /// The context's signature pipelines, which every world built on it shares.
    signatures: Arc<Signatures>,
    /// The loaded skins' signatures by skin number, and their pipelines once built.
    signed: Vec<Option<Signed>>,
}

/// A loaded skin's signature shader and its pipelines: built from its own module off the
/// render thread, so the frame never waits. A signature is built once a blade draws it (worn,
/// in the preview or in the Collection's pictures), its pipelines side by side; until then
/// its blades are drawn by the shared pipelines, from the skin's file. Until 21/10/2026 every
/// loaded signature was also built in the background, wanted or not.
#[derive(Clone)]
struct Signed {
    signature: &'static crate::blade_signatures::Signature,
    built: Arc<OnceLock<Option<SignaturePipelines>>>,
    started: Arc<std::sync::atomic::AtomicBool>,
}

impl Signed {
    /// Claim the build: true the first time only.
    fn claim(&self) -> bool {
        !self.started.swap(true, std::sync::atomic::Ordering::AcqRel)
    }
}

/// The signature pipelines of one graphics context, shared by every world built on it: a map
/// change builds the next world's [`Runtime`] while the last one runs, and each built every
/// loaded signature again on threads of its own that never ended (21/10/2026: ~130 builds a map
/// on two or three more threads each time, while the next map loaded and the old one played).
/// Pipelines made for one world serve the next: wgpu pools identical bind group layouts, and the
/// target format is fixed.
#[derive(Default)]
pub(crate) struct Signatures {
    builds: OnceLock<Arc<BuildQueue>>,
    signed: Mutex<HashMap<&'static str, Signed>>,
}

impl Signatures {
    /// `signature`'s entry: the one made before when there is one, so its build is not done twice.
    fn signed(&self, signature: &'static crate::blade_signatures::Signature) -> Signed {
        self.signed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(signature.id)
            .or_insert_with(|| Signed {
                signature,
                built: Arc::default(),
                started: Arc::default(),
            })
            .clone()
    }
}

/// How soon a signature's pipelines are wanted (Creyon's round 20: with every build waiting its
/// turn alike, the blade in the preview waited behind the Collection's pictures and most blades
/// seemed not to load).
const WANTED_NOW: u8 = 2;
const WANTED_PICTURE: u8 = 1;
const WANTED_LATER: u8 = 0;

/// The signature builds waiting, by how soon they are wanted then in the order asked, and the
/// threads building them (`BuildQueue::start`): a blade worn or in the preview
/// first, then the Collection's pictures.
struct BuildQueue {
    jobs: std::sync::Mutex<Vec<(u8, u64, Signed)>>,
    ready: std::sync::Condvar,
    asked: std::sync::atomic::AtomicU64,
}

impl BuildQueue {
    /// The queue and its builders; none when no thread can be had (blades then keep the
    /// shared pipelines).
    fn start(builder: &SignatureBuilder) -> Arc<Self> {
        let queue = Arc::new(Self {
            jobs: std::sync::Mutex::new(Vec::new()),
            ready: std::sync::Condvar::new(),
            asked: std::sync::atomic::AtomicU64::new(0),
        });
        // Two builders, three on a machine of nine cores or more.
        let workers = std::thread::available_parallelism()
            .map_or(2, |cores| cores.get() / 3)
            .clamp(2, 3);
        for worker in 0..workers {
            let queue = queue.clone();
            let builder = builder.clone();
            let spawned = std::thread::Builder::new()
                .name("sjk-blade-signatures".to_owned())
                .spawn(move || {
                    loop {
                        // The first builder waits for the blade in hand or in the preview alone, so it
                        // never queues behind the Collection's pictures (update 22).
                        let (wanted, signed) = queue.next(worker == 0);
                        if signed.claim() {
                            let _ = signed
                                .built
                                .set(builder.build(signed.signature, wanted == WANTED_NOW));
                            if let Some(Some(built)) = signed.built.get() {
                                builder.build_menu(signed.signature, built);
                            }
                            if wanted == WANTED_LATER {
                                // A short rest, so the game keeps its frames and its connection
                                // (round 20: building several at once at launch stuttered it).
                                std::thread::sleep(std::time::Duration::from_millis(40));
                            }
                        }
                    }
                });
            if let Err(why) = spawned {
                crate::log::progress(format_args!(
                    "blade skins: signature shaders: no thread: {why}"
                ));
            }
        }
        queue
    }

    /// Ask for `signed` to be built, `wanted` this soon; asking again sooner moves it up.
    fn ask(&self, signed: &Signed, wanted: u8) {
        if signed.started.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        let Ok(mut jobs) = self.jobs.lock() else {
            return;
        };
        if let Some(job) = jobs
            .iter_mut()
            .find(|job| Arc::ptr_eq(&job.2.built, &signed.built))
        {
            job.0 = job.0.max(wanted);
            return;
        }
        let order = self
            .asked
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        jobs.push((wanted, order, signed.clone()));
        drop(jobs);
        self.ready.notify_all();
    }

    /// The build wanted soonest (the first asked among those), waiting for one.
    fn next(&self, only_now: bool) -> (u8, Signed) {
        let mut jobs = self
            .jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(best) =
                (0..jobs.len()).max_by_key(|&i| (jobs[i].0, std::cmp::Reverse(jobs[i].1)))
                && (!only_now || jobs[best].0 == WANTED_NOW)
            {
                let (wanted, _, signed) = jobs.swap_remove(best);
                return (wanted, signed);
            }
            jobs = self
                .ready
                .wait(jobs)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

/// A signature skin's glow (`fragment_main`) pipeline and, when it has a solid layer, its solid
/// (`fragment_solid`) one. Its bloom is the shared pipeline's (round 20: its blade's glow only, so
/// the bloom no longer hides its effects, and a third less to build: Creyon saw most blades not
/// load while they built).
struct SignaturePipelines {
    /// The whole blade in one pass, for the menus; a new blade's comes after it is worn (`build_menu`).
    main: OnceLock<wgpu::RenderPipeline>,
    /// The main pass in a view, without its effects in the air, and those into the half-size layer.
    /// Only the new blades' (`@raycache fast`): the blades already in the client draw as they always did.
    view: Option<(wgpu::RenderPipeline, wgpu::RenderPipeline)>,
    solid: Option<wgpu::RenderPipeline>,
}

/// The passes a signature skin has pipelines of its own for.
#[derive(Clone, Copy)]
enum Pass {
    Main,
    /// The main pass of a view whose effects are in the half-size layer.
    World,
    Glow,
    Solid,
}

/// The device, layout and target format the blade pipelines are made with, kept to build
/// the signature skins' own.
#[derive(Clone)]
struct SignatureBuilder {
    device: wgpu::Device,
    layout: wgpu::PipelineLayout,
    volume_layout: wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
}

impl SignatureBuilder {
    /// A blade pipeline: `vertex` and `fragment` of `module`, depth-tested, blended so.
    fn pipeline(
        &self,
        label: &str,
        module: &wgpu::ShaderModule,
        vertex: &str,
        fragment: &str,
        blend: Option<wgpu::BlendState>,
        cache: Option<&wgpu::PipelineCache>,
    ) -> wgpu::RenderPipeline {
        blade_pipeline(
            &self.device,
            &self.layout,
            self.format,
            label,
            module,
            vertex,
            fragment,
            blend,
            cache,
        )
    }

    /// The whole-blade pipeline the menus draw `signature` with, after its view's are in use.
    fn build_menu(
        &self,
        signature: &crate::blade_signatures::Signature,
        built: &SignaturePipelines,
    ) {
        if built.main.get().is_some() {
            return;
        }
        let source = signature.module_source();
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(signature.id),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self.pipeline(
            signature.id,
            &module,
            "vertex_main",
            "fragment_main_deep",
            Some(wgpu::BlendState::ADDITIVE),
            None,
        );
        if pollster::block_on(scope.pop()).is_none() {
            let _ = built.main.set(pipeline);
        }
    }

    /// `signature`'s pipelines; `None` (logged) when its module does not build. With
    /// `side_by_side` (a blade worn or shown now) its two pipelines are built at once, the solid
    /// one on a thread of its own (round 19: a blade's effects took seconds to come in), each
    /// thread catching its own errors (error scopes are the thread's own).
    fn build(
        &self,
        signature: &crate::blade_signatures::Signature,
        side_by_side: bool,
    ) -> Option<SignaturePipelines> {
        let started = std::time::Instant::now();
        let source = signature.module_source();
        let cache = None;
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(signature.id),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let module_ms = started.elapsed().as_millis();
        let mut failed = pollster::block_on(scope.pop()).map(|error| error.to_string());
        let checked = |vertex: &str, fragment: &str, blend: wgpu::BlendState| {
            let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let pipeline =
                self.pipeline(signature.id, &module, vertex, fragment, Some(blend), cache);
            (
                pipeline,
                pollster::block_on(scope.pop()).map(|error| error.to_string()),
            )
        };
        // A new blade draws in a view with `view_of`'s two: its whole-blade pipeline waits (`build_menu`).
        let lazy = signature.meta("raycache") == Some("fast") && !side_by_side;
        let main_of = || {
            if lazy {
                (None, None)
            } else {
                let (pipeline, error) = checked(
                    "vertex_main",
                    "fragment_main_deep",
                    wgpu::BlendState::ADDITIVE,
                );
                (Some(pipeline), error)
            }
        };
        let solid_of = || {
            signature.solid().then(|| {
                checked(
                    "vertex_solid",
                    if signature.meta("solidnear").is_some() {
                        "fragment_solid_near"
                    } else {
                        "fragment_solid"
                    },
                    wgpu::BlendState::ALPHA_BLENDING,
                )
            })
        };
        // The view's two: the main pass without its effects in the air, and those at half size.
        let view_of = || {
            if signature.meta("raycache") != Some("fast") {
                return (None, None);
            }
            let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let split = blade_pipeline_with(
                &self.device,
                &self.layout,
                self.format,
                signature.id,
                &module,
                "vertex_main",
                "fragment_main_deep",
                Some(wgpu::BlendState::ADDITIVE),
                cache,
                &[("SIG_SPLIT", 1.0)],
            );
            let volume = volume::pipeline(
                &self.device,
                &self.volume_layout,
                signature.id,
                &module,
                Instance::layout(),
                cache,
            );
            (
                Some((split, volume)),
                pollster::block_on(scope.pop()).map(|error| error.to_string()),
            )
        };
        let (view, (main, solid)) = std::thread::scope(|threads| {
            let view = threads.spawn(view_of);
            let pair = if side_by_side {
                std::thread::scope(|threads| {
                    let solid = threads.spawn(solid_of);
                    let main = main_of();
                    (main, solid.join())
                })
            } else {
                (main_of(), Ok(solid_of()))
            };
            (view.join(), pair)
        });
        let Ok(view) = view else {
            crate::log::progress(format_args!(
                "blade skins: {}'s signature shader: a build thread failed",
                signature.id
            ));
            return None;
        };
        if failed.is_none() {
            failed.clone_from(&view.1);
        }
        let Ok(solid) = solid else {
            crate::log::progress(format_args!(
                "blade skins: {}'s signature shader: a build thread failed",
                signature.id
            ));
            return None;
        };
        if failed.is_none() {
            failed.clone_from(&main.1);
        }
        if failed.is_none()
            && let Some((_, error)) = &solid
        {
            failed.clone_from(error);
        }
        if let Some(error) = failed {
            crate::log::progress(format_args!(
                "blade skins: {}'s signature shader: {error}",
                signature.id
            ));
            return None;
        }
        let pipelines = SignaturePipelines {
            main: {
                let cell = OnceLock::new();
                if let Some(pipeline) = main.0 {
                    let _ = cell.set(pipeline);
                }
                cell
            },
            view: view.0,
            solid: solid.map(|(pipeline, _)| pipeline),
        };
        crate::log::progress(format_args!(
            "blade skins: {}'s signature shader built in {} ms (module {module_ms} ms)",
            signature.id,
            started.elapsed().as_millis(),
        ));
        Some(pipelines)
    }
}

/// A blade pipeline over the instances: `vertex` and `fragment` of `module`, depth-tested
/// against the scene without writing depth, blended with `blend`.
#[allow(clippy::too_many_arguments)]
fn blade_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    label: &str,
    module: &wgpu::ShaderModule,
    vertex: &str,
    fragment: &str,
    blend: Option<wgpu::BlendState>,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    blade_pipeline_with(
        device,
        layout,
        format,
        label,
        module,
        vertex,
        fragment,
        blend,
        cache,
        &[],
    )
}

/// [`blade_pipeline`] with the fragment stage's `constants` (`override`s) set.
#[allow(clippy::too_many_arguments)]
fn blade_pipeline_with(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    label: &str,
    module: &wgpu::ShaderModule,
    vertex: &str,
    fragment: &str,
    blend: Option<wgpu::BlendState>,
    cache: Option<&wgpu::PipelineCache>,
    constants: &[(&str, f64)],
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some(vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(Instance::layout())],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fragment),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants,
                ..Default::default()
            },
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                // Only what covers (solid parts, black holes) writes alpha, so a target
                // cleared transparent keeps their coverage apart from the added light (the
                // model preview lays them over its backdrop by it, `preview.wgsl`).
                write_mask: if blend == Some(wgpu::BlendState::ALPHA_BLENDING) {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::COLOR
                },
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DepthTarget::FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache,
    })
}

/// Blade instances the model preview can draw: two sabers of eight blades, each its glow,
/// core and echo and a skin's glints.
pub(crate) const PREVIEW_BLADES: usize =
    2 * 8 * (3 + crate::blade_skin_file::extras::MAX_GLINTS as usize);

impl Runtime {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        vfs: &VirtualFileSystem,
        shaders: &ShaderCatalog,
        camera_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        signatures: &Arc<Signatures>,
    ) -> Result<Self, Box<dyn Error>> {
        let source = include_str!("saber.wgsl");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK saber shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK saber texture layout"),
            entries: &[
                texture_layout_entry(0),
                texture_layout_entry(1),
                texture_layout_entry(3),
                sampler_entry(2),
                sampler_entry(4),
                texture_layout_entry(5),
                sampler_entry(6),
            ],
        });
        let samplers = saber::Samplers::new(device);
        let materials =
            saber::create_materials(device, queue, vfs, shaders, &texture_layout, &samplers)?;
        let skins_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK saber skins layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let skins_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK saber skins"),
            size: std::mem::size_of::<[SkinUniform; MAX_SKINS]>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skins = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK saber skins"),
            layout: &skins_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: skins_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK saber pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(&texture_layout),
                Some(&skins_layout),
            ],
            immediate_size: 0,
        });
        let volume_depth = volume::depth_layout(device);
        let volume_read = volume::read_layout(device);
        let volume_composite = volume::composite_pipeline(
            device,
            &volume_read,
            crate::frame_target::aa::effects::FORMAT,
            true,
        );
        let volume_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK saber volume pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(&texture_layout),
                Some(&skins_layout),
                Some(&volume_depth),
            ],
            immediate_size: 0,
        });
        let haze_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK saber haze scene layout"),
            entries: &[texture_layout_entry(0), sampler_entry(1)],
        });
        let haze_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK saber haze pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(&texture_layout),
                Some(&skins_layout),
                Some(&haze_layout),
            ],
            immediate_size: 0,
        });
        let create = |fragment| {
            blade_pipeline(
                device,
                &layout,
                format,
                "SJK saber pipeline",
                &shader,
                "vertex_main",
                fragment,
                Some(wgpu::BlendState::ADDITIVE),
                None,
            )
        };
        let pipeline = create("fragment_main");
        let glow_pipeline = create("fragment_glow");
        // The haze replaces what it covers (the layer still holds the scene there, as no
        // effect has drawn yet), depth-tested so a wall before the blade stays sharp; a
        // black hole's lens and horizon blend over what is there.
        let special = |label, layout, vertex, fragment, blend| {
            blade_pipeline(
                device, layout, format, label, &shader, vertex, fragment, blend, None,
            )
        };
        let haze_pipeline = special(
            "SJK saber haze pipeline",
            &haze_pipeline_layout,
            "vertex_haze",
            "fragment_haze",
            None,
        );
        let lens_pipeline = special(
            "SJK saber black hole lens pipeline",
            &haze_pipeline_layout,
            "vertex_hole",
            "fragment_lens",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let shade_pipeline = special(
            "SJK saber black hole horizon pipeline",
            &layout,
            "vertex_hole",
            "fragment_shade",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let solid_pipeline = special(
            "SJK saber solid pipeline",
            &layout,
            "vertex_solid",
            "fragment_solid",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let black_pipeline = special(
            "SJK JA+ black saber pipeline",
            &layout,
            "vertex_main",
            "fragment_black",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let haze_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SJK saber haze sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK saber instances"),
            size: (saber::MAX_BLADE_INSTANCES * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let preview_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK model preview saber instances"),
            size: (PREVIEW_BLADES * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pictures_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade picture saber instances"),
            size: (crate::blade_pictures::BLADES * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let neutral = materials[crate::saber_rgb::SKIN_MATERIAL as usize].clone();
        // Its heat wakes bend the scene the haze's way, through the haze's bind group.
        let trails = crate::saber_trail_gpu::Runtime::new(
            device,
            queue,
            vfs,
            shaders,
            camera_layout,
            &haze_layout,
            format,
        )?;
        Ok(Self {
            pipeline,
            black_pipeline,
            glow_pipeline,
            materials,
            neutral,
            loaded: Arc::default(),
            uploaded: [false; MAX_SKINS],
            device: device.clone(),
            texture_layout,
            samplers,
            skins_buffer,
            skins,
            instance_buffer,
            volume_depth,
            volume_read,
            volume_composite,
            volume_targets: Default::default(),
            volume_now: Default::default(),
            preview_buffer,
            preview_ranges: std::array::from_fn(|_| 0..0),
            pictures_buffer,
            pictures_ranges: std::array::from_fn(|_| 0..0),
            ranges: std::array::from_fn(|_| 0..0),
            haze_pipeline,
            haze_layout,
            haze_sampler,
            haze_source: std::cell::RefCell::new(None),
            any_haze: false,
            lens_pipeline,
            shade_pipeline,
            any_holes: false,
            solid_pipeline,
            any_solids: false,
            builds: signatures
                .builds
                .get_or_init(|| {
                    BuildQueue::start(&SignatureBuilder {
                        device: device.clone(),
                        layout,
                        volume_layout,
                        format,
                    })
                })
                .clone(),
            signatures: signatures.clone(),
            signed: Vec::new(),
            trails,
            ribbons: crate::saber_ribbons::Gpu::new(device, camera_layout, format),
        })
    }

    /// Take the loaded blade skins: their parameters into the uniform array. Their glow/core
    /// pairs and signature pipelines wait until a blade draws them ([`Self::upload_drawn`]):
    /// every skin of the hub's pack up front was about 45 MiB of video memory on every
    /// world (165 skins, 21/10/2026), most of them worn by nobody on the server. Only when
    /// skins load, never per frame; an edit of skin `only` puts its pair alone up again.
    pub(crate) fn upload_skins(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        skins: &Arc<LoadedSkins>,
        only: Option<crate::saber_skins::SkinIndex>,
    ) {
        queue.write_buffer(
            &self.skins_buffer,
            0,
            bytemuck::cast_slice(&skins.uniforms()),
        );
        // A flash's heat burst is drawn with the haze.
        self.any_haze = skins.iter().any(|(_, skin)| {
            let shown = skin.shown();
            shown.haze.is_some() || shown.flash.is_some_and(|f| f.burst.is_some())
        });
        self.any_holes = skins.iter().any(|(_, skin)| skin.shown().holes.is_some());
        self.any_solids = skins.iter().any(|(_, skin)| {
            skin.shown().solids().any()
                || skin.design_shown().is_some_and(|design| design.solid)
                || skin
                    .signature_shown()
                    .is_some_and(|signature| signature.solid())
        });
        // Pipelines already built for a signature are kept, whatever number it loads at now
        // (Creyon's round 21: entering a server reloaded the skins in another order and every
        // signature was built again, 233 builds for 128 blades, its effects gone meanwhile), and
        // whatever world of the context built them ([`Signatures`]).
        let mut signed: Vec<Option<Signed>> = (0..MAX_SKINS).map(|_| None).collect();
        for (index, skin) in skins.iter() {
            if let Some(signature) = skin.signature_shown() {
                signed[usize::from(index)] = Some(self.signatures.signed(signature));
            }
        }
        self.signed = signed;
        // The skins' slots only: the black pair sits after them (2026.1021.2 lost it here,
        // black blades drawn with the neutral pair's opaque squares).
        let first = crate::saber_rgb::SKIN_MATERIAL as usize;
        let black = crate::saber_rgb::BLACK_MATERIAL as usize;
        // An edit of one skin (the Saber Lab's editor) puts its pair alone up again.
        for slot in 0..black - first {
            if only.is_none_or(|only| usize::from(only) == slot) {
                self.materials[first + slot] = self.neutral.clone();
                self.uploaded[slot] = false;
            }
        }
        self.loaded = Arc::clone(skins);
    }

    /// Upload the glow/core pair of every skin `ranges` draws that has none yet, and ask for
    /// its signature's pipelines `wanted` this soon.
    fn upload_drawn(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        ranges: &[Range<u32>],
        wanted: u8,
    ) {
        let first = crate::saber_rgb::SKIN_MATERIAL as usize;
        let loaded = Arc::clone(&self.loaded);
        for (index, skin) in loaded.iter() {
            let slot = usize::from(index);
            if self.uploaded[slot] || ranges[first + slot].is_empty() {
                continue;
            }
            self.uploaded[slot] = true;
            self.materials[first + slot] = saber::material(
                &self.device,
                queue,
                &self.texture_layout,
                &self.samplers,
                &skin.glow,
                &skin.core,
            );
        }
        self.build_signatures(ranges, wanted);
    }

    /// Ask for every loaded signature's pipelines, as a blade worn later would (the world
    /// shots that wait for them all).
    #[cfg(test)]
    pub(crate) fn build_every_signature(&self) {
        for signed in self.signed.iter().flatten() {
            self.builds.ask(signed, WANTED_LATER);
        }
    }

    /// The loaded signature skins whose pipelines are built, and how many are loaded (the
    /// world shots wait for every one).
    #[cfg(test)]
    pub(crate) fn signatures_built(&self) -> (usize, usize) {
        let loaded = self.signed.iter().flatten();
        let built = loaded
            .clone()
            .filter(|signed| signed.built.get().is_some())
            .count();
        (built, loaded.count())
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
        trails: &mut SegmentPool,
        ribbons: &crate::saber_ribbons::Ribbons,
        now: i64,
    ) {
        self.ranges = saber::material_ranges(blades);
        let ranges = self.ranges.clone();
        self.upload_drawn(queue, &ranges, WANTED_NOW);
        if !blades.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(blades));
        }
        self.trails.prepare(queue, trails, now);
        self.ribbons.prepare(queue, ribbons, now);
    }

    /// Ask for the pipelines of every signature skin `ranges` draws whose build has not begun,
    /// `wanted` this soon.
    fn build_signatures(&self, ranges: &[Range<u32>], wanted: u8) {
        let skins = crate::saber_rgb::SKIN_MATERIAL as usize;
        for (signed, range) in self.signed.iter().zip(&ranges[skins..]) {
            if let Some(signed) = signed
                && !range.is_empty()
            {
                self.builds.ask(signed, wanted);
            }
        }
    }

    /// The pipeline drawing material `material` in `pass`: a signature skin's own once
    /// built, else `shared`.
    fn pick<'a>(
        &'a self,
        material: usize,
        pass: Pass,
        shared: &'a wgpu::RenderPipeline,
    ) -> &'a wgpu::RenderPipeline {
        let own = material
            .checked_sub(crate::saber_rgb::SKIN_MATERIAL as usize)
            .and_then(|index| self.signed.get(index)?.as_ref()?.built.get()?.as_ref());
        match (own, pass) {
            (Some(own), Pass::Main) => own.main.get().unwrap_or(shared),
            (Some(own), Pass::World) => own
                .view
                .as_ref()
                .map_or(own.main.get().unwrap_or(shared), |view| &view.0),
            (Some(own), Pass::Solid) => own.solid.as_ref().unwrap_or(shared),
            (Some(_), Pass::Glow) => shared,
            (None, _) => shared,
        }
    }

    pub(crate) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        self.trails.draw(pass, camera, true);
        let layer = self.volume_now.borrow_mut().take();
        self.draw_blades(
            pass,
            camera,
            if layer.is_some() {
                Pass::World
            } else {
                Pass::Main
            },
        );
        if let Some(layer) = layer {
            pass.set_pipeline(&self.volume_composite);
            pass.set_bind_group(0, &layer.read, &[]);
            pass.draw(0..3, 0..1);
        }
        self.draw_horizons(pass, camera, &self.instance_buffer, &self.ranges, true);
        self.ribbons.draw(pass, camera);
    }

    /// This view's signature blades' effects in the air, into a half-size layer that
    /// [`Self::draw`] adds after their main pass (`saber_volume.rs`): a quarter of the pixels.
    /// Runs before the effect layer's pass, with the depth that pass will read.
    pub(crate) fn draw_volumes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera: &wgpu::BindGroup,
        depth: &wgpu::TextureView,
        size: [u32; 2],
        region: Option<[u32; 4]>,
    ) {
        *self.volume_now.borrow_mut() = None;
        let first = crate::saber_rgb::SKIN_MATERIAL as usize;
        let drawn: Vec<(usize, &wgpu::RenderPipeline, Range<u32>)> = self
            .ranges
            .iter()
            .enumerate()
            .skip(first)
            .filter(|(_, instances)| !instances.is_empty())
            .filter_map(|(material, instances)| {
                let own = self
                    .signed
                    .get(material - first)?
                    .as_ref()?
                    .built
                    .get()?
                    .as_ref()?;
                Some((material, &own.view.as_ref()?.1, instances.clone()))
            })
            .collect();
        if drawn.is_empty() {
            return;
        }
        let target = {
            let mut targets = self.volume_targets.borrow_mut();
            match targets.iter().find(|target| target.size == size) {
                Some(target) => target.clone(),
                None => {
                    // A resize leaves the old size behind; views are few.
                    if targets.len() >= 4 {
                        targets.remove(0);
                    }
                    let target = volume::Target::new(&self.device, &self.volume_read, size);
                    targets.push(target.clone());
                    target
                }
            }
        };
        let depth_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK saber volume depth"),
            layout: &self.volume_depth,
            entries: &[wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(depth),
            }],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK saber volume layer"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some([x, y, w, h]) = region {
                let (half_w, half_h) = (size[0].div_ceil(2), size[1].div_ceil(2));
                let (x, y) = ((x / 2).min(half_w), (y / 2).min(half_h));
                pass.set_scissor_rect(
                    x,
                    y,
                    (w / 2 + 2).min(half_w - x),
                    (h / 2 + 2).min(half_h - y),
                );
            }
            pass.set_bind_group(0, camera, &[]);
            pass.set_bind_group(2, &self.skins, &[]);
            pass.set_bind_group(3, &depth_bind, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            for (material, pipeline, instances) in drawn {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(1, &self.materials[material], &[]);
                pass.draw(0..6, instances);
            }
        }
        *self.volume_now.borrow_mut() = Some(target);
    }

    /// Whether this frame's blades may bend the scene with heat haze or a black hole's
    /// lens: a loaded skin has one and a skinned blade is drawn (the vertex stage folds
    /// away every other one), or a trail with a heat wake is drawn.
    pub(crate) fn has_haze(&self) -> bool {
        (self.any_haze || self.any_holes) && skinned(&self.ranges) || self.trails.has_wake()
    }

    /// How far this frame's haze or lenses may bend the picture they sample, as a share of
    /// the target's longer side (`effect_bounds`): the lens's reach when a drawn blade's skin
    /// has black holes, the haze's when one has heat haze, else none. At most one look per
    /// loaded skin, nothing allocated.
    pub(crate) fn bent_reach(&self) -> f32 {
        use crate::frame_target::aa::effects::bounds::{HAZE_REACH, LENS_REACH};
        if !self.has_haze() {
            return 0.0;
        }
        let first = crate::saber_rgb::SKIN_MATERIAL as usize;
        // A heat wake bends as far as the haze does.
        let mut reach = if self.trails.has_wake() {
            HAZE_REACH
        } else {
            0.0_f32
        };
        for (index, skin) in self.loaded.iter() {
            if self.ranges[first + usize::from(index)].is_empty() {
                continue;
            }
            let shown = skin.shown();
            if shown.holes.is_some() {
                return LENS_REACH;
            }
            if shown.haze.is_some() {
                reach = HAZE_REACH;
            }
        }
        reach
    }

    /// The black holes' horizons and the skins' solid parts (ink, chrome) over the blades
    /// just drawn from `buffer`.
    fn draw_horizons<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        buffer: &'pass wgpu::Buffer,
        ranges: &[Range<u32>],
        black_blades: bool,
    ) {
        let black = crate::saber_rgb::BLACK_MATERIAL as usize;
        if black_blades && !ranges[black].is_empty() {
            draw_instances(
                pass,
                camera,
                &self.skins,
                |_| &self.black_pipeline,
                &self.materials[black..],
                buffer,
                &ranges[black..],
            );
        }
        if self.any_holes && skinned(ranges) {
            draw_instances(
                pass,
                camera,
                &self.skins,
                |_| &self.shade_pipeline,
                &self.materials,
                buffer,
                ranges,
            );
        }
        if self.any_solids && skinned(ranges) {
            draw_instances(
                pass,
                camera,
                &self.skins,
                |material| self.pick(material, Pass::Solid, &self.solid_pipeline),
                &self.materials,
                buffer,
                ranges,
            );
        }
    }

    /// Draw the skins' heat haze into the effect layer whose scene image is `scene`
    /// (`effect_layer.rs` `original`), before any other effect: each pixel it covers is
    /// the scene there, bent. The trails' heat wakes after it, then the black holes'
    /// lenses.
    pub(crate) fn draw_haze(
        &self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'_>,
        camera: &wgpu::BindGroup,
        scene: &wgpu::TextureView,
    ) {
        if !self.has_haze() {
            return;
        }
        let bind = {
            let mut source = self.haze_source.borrow_mut();
            match source.as_ref() {
                Some((view, bind)) if view == scene => bind.clone(),
                _ => {
                    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("SJK saber haze scene"),
                        layout: &self.haze_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(scene),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.haze_sampler),
                            },
                        ],
                    });
                    *source = Some((scene.clone(), bind.clone()));
                    bind
                }
            }
        };
        // The haze first, then the trails' wakes, then the lenses over them: each in its
        // own run, as the instances of a material are in no set order.
        if self.any_haze {
            self.draw_bending(pass, camera, &bind, &self.haze_pipeline);
        }
        self.trails.draw_wake(pass, camera, &bind);
        if self.any_holes {
            self.draw_bending(pass, camera, &bind, &self.lens_pipeline);
        }
    }

    /// Draw every skinned blade with `pipeline`, one of those that bend the scene bound in
    /// `bind` (the haze's or the lenses').
    fn draw_bending(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        camera: &wgpu::BindGroup,
        bind: &wgpu::BindGroup,
        pipeline: &wgpu::RenderPipeline,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(2, &self.skins, &[]);
        pass.set_bind_group(3, bind, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        let skins = crate::saber_rgb::SKIN_MATERIAL as usize;
        for (material, instances) in self
            .materials
            .iter()
            .zip(self.ranges.iter().cloned())
            .skip(skins)
        {
            if instances.is_empty() {
                continue;
            }
            pass.set_bind_group(1, material, &[]);
            pass.draw(0..6, instances);
        }
    }

    /// Upload the model preview's blades (at most [`PREVIEW_BLADES`]).
    pub(crate) fn prepare_preview(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
    ) {
        let count = blades.len().min(PREVIEW_BLADES);
        let blades = &mut blades[..count];
        self.preview_ranges = saber::material_ranges(blades);
        let ranges = self.preview_ranges.clone();
        self.upload_drawn(queue, &ranges, WANTED_NOW);
        if !blades.is_empty() {
            queue.write_buffer(&self.preview_buffer, 0, bytemuck::cast_slice(blades));
        }
    }

    /// Draw the model preview's blades with the preview's `camera`.
    pub(crate) fn draw_preview<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        draw_instances(
            pass,
            camera,
            &self.skins,
            |material| self.pick(material, Pass::Main, &self.pipeline),
            &self.materials,
            &self.preview_buffer,
            &self.preview_ranges,
        );
        self.draw_horizons(
            pass,
            camera,
            &self.preview_buffer,
            &self.preview_ranges,
            true,
        );
    }

    /// Upload the Collection's blade pictures' blades (at most
    /// [`crate::blade_pictures::BLADES`]), sorted by material.
    pub(crate) fn prepare_pictures(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
    ) {
        let count = blades.len().min(crate::blade_pictures::BLADES);
        let blades = &mut blades[..count];
        self.pictures_ranges = saber::material_ranges(blades);
        let ranges = self.pictures_ranges.clone();
        self.upload_drawn(queue, &ranges, WANTED_PICTURE);
        if !blades.is_empty() {
            queue.write_buffer(&self.pictures_buffer, 0, bytemuck::cast_slice(blades));
        }
    }

    /// Draw the blade pictures' blades with the pictures' `camera`.
    pub(crate) fn draw_pictures<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        draw_instances(
            pass,
            camera,
            &self.skins,
            |material| self.pick(material, Pass::Main, &self.pipeline),
            &self.materials,
            &self.pictures_buffer,
            &self.pictures_ranges,
        );
        self.draw_horizons(
            pass,
            camera,
            &self.pictures_buffer,
            &self.pictures_ranges,
            true,
        );
    }

    /// Draw into the dynamic glow image: the trails (`saberBlur`/`swordTrail` glow)
    /// with `trails` (not for `r_DynamicGlow 2`, stock's `RT_SABER_GLOW` test), then the
    /// blades' glow capsules without their cores.
    pub(crate) fn draw_glow<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        trails: bool,
    ) {
        if trails {
            self.trails.draw(pass, camera, false);
        }
        self.draw_blades(pass, camera, Pass::Glow);
        // A hole in front of the blade hides its glow there too.
        self.draw_horizons(pass, camera, &self.instance_buffer, &self.ranges, false);
    }

    /// Whether the glow image gets any saber this frame, its trails with `trails`.
    pub(crate) fn has_glow(&self, trails: bool) -> bool {
        (trails && self.trails.has_draws()) || self.ranges.iter().any(|r| !r.is_empty())
    }

    fn draw_blades<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        kind: Pass,
    ) {
        let shared = match kind {
            Pass::Glow => &self.glow_pipeline,
            _ => &self.pipeline,
        };
        draw_instances(
            pass,
            camera,
            &self.skins,
            |material| self.pick(material, kind, shared),
            &self.materials,
            &self.instance_buffer,
            &self.ranges,
        );
    }

    /// Add this frame's trail quads to the effect layer's screen bounds.
    pub(crate) fn bound_trails(
        &self,
        bounds: &mut crate::frame_target::aa::effects::bounds::Bounds,
    ) {
        for position in self.trails.positions() {
            bounds.sphere(glam::Vec3::from_array(position), 0.0);
        }
        // The smoke ribbons too, as wide as they are drawn: without them the effect layer's
        // rectangle held the blades alone, and a blade put out left its smoke undrawn.
        for (position, half_width) in self.ribbons.extents() {
            bounds.sphere(glam::Vec3::from_array(position), half_width);
        }
    }

    /// Whether this frame has any blade or trail to draw.
    pub(crate) fn has_draws(&self) -> bool {
        self.trails.has_draws()
            || self.ribbons.has_draws()
            || self.ranges.iter().any(|range| !range.is_empty())
    }
}

/// Whether `ranges` holds a skinned blade.
fn skinned(ranges: &[Range<u32>]) -> bool {
    ranges[crate::saber_rgb::SKIN_MATERIAL as usize..crate::saber_rgb::BLACK_MATERIAL as usize]
        .iter()
        .any(|range| !range.is_empty())
}

/// Draw the blade instances of `buffer`, a run per material, each with the pipeline
/// `pipeline` picks for its place in `materials` (a signature skin's may be its own).
fn draw_instances<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    camera: &'pass wgpu::BindGroup,
    skins: &'pass wgpu::BindGroup,
    pipeline: impl Fn(usize) -> &'pass wgpu::RenderPipeline,
    materials: &'pass [wgpu::BindGroup],
    buffer: &'pass wgpu::Buffer,
    ranges: &[Range<u32>],
) {
    if ranges.iter().all(Range::is_empty) {
        return;
    }
    // Materials offset from the first (the black blades') pick by their own number.
    let first = crate::saber_rgb::MATERIAL_COUNT - materials.len();
    let mut current: Option<&wgpu::RenderPipeline> = None;
    for (index, (material, instances)) in materials.iter().zip(ranges.iter().cloned()).enumerate() {
        if instances.is_empty() {
            continue;
        }
        let chosen = pipeline(first + index);
        if current.is_none_or(|current| !std::ptr::eq(current, chosen)) {
            pass.set_pipeline(chosen);
            if current.is_none() {
                pass.set_bind_group(0, camera, &[]);
                pass.set_bind_group(2, skins, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
            }
            current = Some(chosen);
        }
        pass.set_bind_group(1, material, &[]);
        pass.draw(0..6, instances);
    }
}

#[cfg(test)]
#[path = "blade_signatures_cost_test.rs"]
mod cost_test;
