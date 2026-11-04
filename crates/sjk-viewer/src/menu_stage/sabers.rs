//! Sabers in the stage model's hands. Each hilt is its own detached mesh
//! (the shared object buffers only exist inside a session) parked on the
//! hand bolt of the pose the stage was skinned with, and its blades go
//! through the same glow/core renderer as in-game sabers, so the menu shows
//! exactly the hilt, sockets and colours the player will carry. On
//! lightsaber creation's preview the hilts leave the hands for the
//! [`showcase`](super::showcase). On the stage the blades leave a match's
//! motion trail behind them ([`crate::saber_trail`]) as the model swings.

use super::hilt_guard;
use super::lone;
use super::showcase::{self, Line, View};
use super::throw::{Pose, Throw};
use super::*;
use crate::menu_backdrop::Focus;
use crate::player_menu::StageSabers;
use crate::saber::{self, Attachment, HAND_BOLT, LEFT_HAND_BOLT};
use crate::saber_hilts::{HiltBlade, hilt_blades, hilt_sockets};
use crate::saber_rgb::BladeColor;

/// The last request the hands were synced to, so a request that has not
/// changed costs a comparison and nothing else — including one whose hilt
/// failed to load (it is retried only when the request changes).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Request {
    hilts: [Option<String>; 2],
    colors: [[u8; 3]; 2],
    black: [bool; 2],
}

impl Request {
    pub(super) fn matches(&self, wanted: &StageSabers<'_>) -> bool {
        self.colors == wanted.colors
            && self.black == wanted.black
            && self
                .hilts
                .iter()
                .zip(wanted.hilts)
                .all(|(current, wanted)| current.as_deref() == wanted)
    }

    /// The hilt in `hand`, by its `.sab` name.
    pub(super) fn hilt(&self, hand: usize) -> Option<&str> {
        self.hilts.get(hand)?.as_deref()
    }

    pub(super) fn from(wanted: &StageSabers<'_>) -> Self {
        Self {
            hilts: wanted.hilts.map(|name| name.map(str::to_owned)),
            colors: wanted.colors,
            black: wanted.black,
        }
    }
}

/// One loaded hilt in a hand.
pub(super) struct StageSaber {
    color: BladeColor,
    blades: [Option<HiltBlade>; 8],
    num_blades: u8,
    /// Where the hilt lies on the showcase; `None` without a blade.
    line: Option<Line>,
    draws: Vec<DetachedDraw>,
    materials: DetachedMaterials,
    vertex_buffer: wgpu::Buffer,
    geometry_binding: wgpu::BindGroup,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    /// Where the Saber Lab's floating copy of the hilt lies, beside the one in the hand.
    float_instance_buffer: wgpu::Buffer,
    /// Each blade's length as the Saber Lab lights it and puts it out, growing and
    /// shrinking as a match's do ([`saber::Extension`]).
    extension: [saber::Extension; 8],
    /// Each blade's motion trail, as a match keeps one ([`crate::saber_trail::Trail`]).
    trails: [crate::saber_trail::Trail; 8],
    /// Each blade's afterimages' poses in the hand, for a skin leaving them, as a match
    /// keeps them ([`crate::saber_ghosts`]).
    ghosts: [crate::saber_ghosts::GhostTrail; 8],
    /// When each blade in the hand last lit, for its emitter stars' surge.
    ignition: [saber::Ignition; 8],
    /// Each blade's motion in the hand (`saber_motion.rs`): its swing, for a skin whose
    /// unrest follows it and the Blade Lab designs that answer it, and the floating copy's
    /// (which clash with the Saber Lab's button too).
    motion: [crate::saber_motion::BladeMotion; 8],
    float_motion: [crate::saber_motion::BladeMotion; 8],
    /// The Saber Lab's clashes this hilt last took (`LabStage::clash`).
    clash_seen: Option<u32>,
    /// Each blade's emitter's smoke, in the hand and on the Saber Lab's floating copy
    /// ([`crate::saber_smoke`]).
    smoke: [crate::saber_smoke::Emitter; 8],
    float_smoke: [crate::saber_smoke::Emitter; 8],
}

impl StageSaber {
    /// Draw the hilt's opaque or blended surfaces.
    pub(super) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        world_materials: &'pass world_materials::Runtime,
        camera: &'pass wgpu::BindGroup,
        blended: bool,
    ) {
        world_materials.draw_detached(
            pass,
            camera,
            &self.geometry_binding,
            &self.vertex_buffer,
            &self.index_buffer,
            &self.instance_buffer,
            &self.materials,
            &self.draws,
            blended,
        );
    }

    /// Draw the Saber Lab's floating copy of the hilt.
    pub(super) fn draw_floating<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        world_materials: &'pass world_materials::Runtime,
        camera: &'pass wgpu::BindGroup,
        blended: bool,
    ) {
        world_materials.draw_detached(
            pass,
            camera,
            &self.geometry_binding,
            &self.vertex_buffer,
            &self.index_buffer,
            &self.float_instance_buffer,
            &self.materials,
            &self.draws,
            blended,
        );
    }
}

impl GpuState {
    /// The skins the stage's blades wear, the first saber's and the second's (or a staff's
    /// other half's): the stage model is the local player, so the player's. The shader the
    /// Collection shows goes on the side being dressed; the other side shows what it wears
    /// now, so the pair is seen as Equip will leave it. With the target switch on the
    /// second, the shader goes there and the first keeps what it wears. The Saber Lab's tab
    /// dresses both.
    pub(super) fn stage_skins(&self) -> [Option<crate::saber_skins::SkinColor>; 2] {
        let shown = match self.menu_stage.skin_override {
            Some(crate::console::collection_panel::PreviewSkin::Stock) => Some(None),
            Some(crate::console::collection_panel::PreviewSkin::Skin(id)) => Some(
                self.blade_skins
                    .color_with(id, self.looks.own_options_off(id))
                    .map(|skin| crate::saber_skins::SkinColor {
                        persona: self.saber_skins.local_persona(),
                        ..skin
                    }),
            ),
            _ => None,
        };
        let dressed = if self.menu_stage.lab.is_some() {
            [true; 2]
        } else {
            self.menu_stage.override_dressed
        };
        crate::saber_skins::previewed(self.saber_skins.local_both(), shown, dressed)
    }

    /// Reload the hands for a changed request. Sockets and blade parameters
    /// come from the `.sab` definitions; a hilt that fails to load is logged and
    /// the hand holds the default hilt instead (empty only when that fails too).
    pub(super) fn sync_stage_sabers(&mut self, request: Request) {
        for hand in 0..2 {
            let color = if request.black[hand] {
                BladeColor::Black
            } else {
                BladeColor::from_rgb(request.colors[hand])
            };
            self.menu_stage.sabers[hand] = request.hilts[hand]
                .as_deref()
                .and_then(|name| self.stage_saber_or_default(name, color));
        }
        self.menu_stage.request = request;
        self.menu_stage.sabers_dirty = true;
    }

    /// `name`'s hilt, or [`hilt_guard::DEFAULT_HILT`]'s when it cannot be shown.
    fn stage_saber_or_default(&mut self, name: &str, color: BladeColor) -> Option<StageSaber> {
        let error = match self.guarded_stage_saber(name, color) {
            Ok(saber) => return Some(saber),
            Err(error) => error,
        };
        if name.eq_ignore_ascii_case(hilt_guard::DEFAULT_HILT) {
            crate::log::progress(format_args!(
                "player stage could not load saber {name}: {error}"
            ));
            return None;
        }
        crate::log::progress(format_args!(
            "player stage could not load saber {name}: {error}; showing {} instead",
            hilt_guard::DEFAULT_HILT
        ));
        self.guarded_stage_saber(hilt_guard::DEFAULT_HILT, color)
            .inspect_err(|error| {
                crate::log::progress(format_args!(
                    "player stage could not load saber {}: {error}",
                    hilt_guard::DEFAULT_HILT
                ));
            })
            .ok()
    }

    /// [`Self::build_stage_saber`] with wgpu's errors captured (an uncaptured one
    /// ends the game), and not tried again for a hilt the game closed while loading
    /// the last time ([`hilt_guard`]): a hilt saved in the profile must not take
    /// every later start down with it (#214).
    fn guarded_stage_saber(
        &mut self,
        name: &str,
        color: BladeColor,
    ) -> Result<StageSaber, Box<dyn Error>> {
        let directory = crate::crash_report::directory();
        if hilt_guard::crashed_on(directory.as_deref(), name) {
            return Err("the game closed while loading it last time".into());
        }
        hilt_guard::begin(directory.as_deref(), name);
        let kept = self.world_materials.texture_keys();
        let device = self.device.clone();
        let built = crate::gpu_guard::capture(&device, || self.build_stage_saber(name, color))
            .unwrap_or_else(|error| {
                self.world_materials.forget_textures_since(&kept);
                Err(format!("the graphics card: {error}").into())
            });
        hilt_guard::end(directory.as_deref());
        built
    }

    fn build_stage_saber(
        &mut self,
        name: &str,
        color: BladeColor,
    ) -> Result<StageSaber, Box<dyn Error>> {
        let vfs = self.vfs.clone().ok_or("player stage has no VFS")?;
        if self.menu_stage.saber_definitions.is_none() {
            self.menu_stage.saber_definitions = Some(crate::saber_defs::load(&vfs)?);
        }
        let definition = self
            .menu_stage
            .saber_definitions
            .as_ref()
            .and_then(|definitions| definitions.get(&name.to_ascii_lowercase()))
            .ok_or_else(|| format!("no saber definition named {name}"))?
            .clone();
        let asset = vfs
            .read(&definition.model)?
            .ok_or_else(|| format!("hilt model {} is missing", definition.model))?;
        let model = Glm::parse(&asset.bytes)?;
        let sockets = hilt_sockets(&model)?;
        let mut flattened = FlattenedScene::default();
        let mut draws = append_static_glm_mesh(&mut flattened, &model)?;
        let blades = hilt_blades(&definition, &sockets);
        let line = showcase_line(&blades, definition.num_blades, &flattened.vertices);
        pad_empty_mesh(&mut flattened, &mut draws);
        let materials = self.world_materials.compile_detached(
            &self.device,
            &self.queue,
            &vfs,
            &self.shaders,
            &flattened.materials,
        )?;
        let draws = draws
            .into_iter()
            .map(|draw| DetachedDraw {
                indices: draw.indices,
                material: draw.material,
            })
            .collect();
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SJK player stage hilt vertices"),
                contents: bytemuck::cast_slice(&flattened.vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SJK player stage hilt indices"),
                contents: bytemuck::cast_slice(&flattened.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        let instance = ActorInstance::new([0.0; 3], Quat::IDENTITY.to_array(), [1.0; 3]);
        let quads = crate::shared_geometry::quads::upload(
            &self.device,
            flattened.vertices.len(),
            &flattened.indices,
            0,
        );
        let geometry_binding =
            crate::shared_geometry::quads::bind(&self.device, &vertex_buffer, &quads);
        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SJK player stage hilt instance"),
                contents: bytemuck::bytes_of(&instance),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let float_instance_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("SJK player stage floating hilt instance"),
                    contents: bytemuck::bytes_of(&instance),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        Ok(StageSaber {
            color,
            blades,
            num_blades: definition.num_blades,
            line,
            draws,
            materials,
            vertex_buffer,
            geometry_binding,
            index_buffer,
            instance_buffer,
            float_instance_buffer,
            extension: Default::default(),
            trails: Default::default(),
            ghosts: Default::default(),
            ignition: Default::default(),
            motion: Default::default(),
            float_motion: Default::default(),
            clash_seen: None,
            smoke: Default::default(),
            float_smoke: Default::default(),
        })
    }

    /// Step every hand's throw for this frame and remember where its hilt
    /// is while out of the hand. Two hilts float side by side across the
    /// focus, one alone on it. Without a hand, a hilt or a focus point a
    /// saber simply stays held.
    /// The Force page's Saber Throw: the first saber flies out of the hand and back
    /// on a loop, `seconds` into it ([`super::force_motion::throw_at`]). Run after
    /// [`Self::advance_stage_throw`], which leaves a preview's sabers in hand.
    pub(super) fn advance_preview_throw(&mut self, seconds: f32) {
        let stage = &mut self.menu_stage;
        let Some(actor) = stage.actor.as_ref() else {
            return;
        };
        let Some(bolt) = actor.hands[0] else {
            return;
        };
        let Some(blade) = stage.sabers[0]
            .as_ref()
            .and_then(|saber| saber.blades[0].as_ref())
            .map(|blade| Vec3::from_array(blade.socket.direction))
        else {
            return;
        };
        let (grip, rotation) = saber::world_attachment(actor.origin, actor.rotation, bolt);
        // Out to the model's side and towards the camera in front of it, a little up,
        // so the whole flight stays in the picture.
        let facing = actor.rotation * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * Vec3::X;
        let side = Vec3::Z.cross(facing).normalize_or(Vec3::Y);
        let direction = (facing * 0.6 - side * 0.75 + Vec3::Z * 0.15).normalize_or(facing);
        let flying =
            super::force_motion::throw_at(Pose { grip, rotation }, blade, direction, seconds);
        if flying.is_some() || stage.flying[0].is_some() {
            stage.sabers_dirty = true;
        }
        stage.flying[0] = flying;
    }

    pub(super) fn advance_stage_throw(&mut self, thrown: bool, focus: Option<Focus>, now: Instant) {
        let stage = &mut self.menu_stage;
        let held = stage.sabers.iter().flatten().count();
        for hand in 0..2 {
            let attachment = stage.actor.as_ref().and_then(|actor| {
                let (grip, rotation) =
                    saber::world_attachment(actor.origin, actor.rotation, actor.hands[hand]?);
                Some(Pose { grip, rotation })
            });
            let blade = stage.sabers[hand]
                .as_ref()
                .and_then(|saber| saber.blades[0].as_ref())
                .map(|blade| {
                    let direction = Vec3::from_array(blade.socket.direction).normalize_or(Vec3::Y);
                    (
                        direction,
                        Vec3::from_array(blade.socket.origin).dot(direction),
                    )
                });
            let flying = match blade {
                Some((blade, root)) => {
                    let spot = focus.map(|focus| {
                        focus.point
                            + focus.right * float_offset(hand, held)
                            + Vec3::Z * (FLOAT_ROOT - root)
                    });
                    stage.throw[hand].advance(thrown, attachment, spot, blade, now)
                }
                None => {
                    stage.throw[hand] = Throw::Held;
                    None
                }
            };
            if flying.is_some() || stage.flying[hand].is_some() {
                stage.sabers_dirty = true;
            }
            stage.flying[hand] = flying;
        }
    }

    /// Start this frame's saber instance list: cleared, then seeded with the
    /// stage model's blades (session sabers are appended by entity
    /// submission afterwards). Hilt instances are rewritten only when the
    /// pose or the hilts changed.
    pub(crate) fn begin_saber_instances(&mut self, presentation_time: i64) {
        self.saber_instances.clear();
        let skins = self.stage_skins();
        let now = std::time::Instant::now();
        // `cg_saberTrail`, on unless set to 0, as in a match.
        let trails = self
            .console
            .as_ref()
            .and_then(|console| console.integer_cvar("cg_saberTrail"))
            .unwrap_or(1)
            != 0;
        // The Saber Lab's floating saber shows a still trail behind it while the tab asks,
        // swept about the camera's forward through its grip.
        let still_trail = self.menu_stage.lab.is_some_and(|lab| lab.trail);
        let toward = Vec3::new(
            self.camera_yaw.cos() * self.camera_pitch.cos(),
            self.camera_yaw.sin() * self.camera_pitch.cos(),
            self.camera_pitch.sin(),
        );
        let frame_view = self.frame_view;
        let stage = &mut self.menu_stage;
        stage.preview.blades.clear();
        // What the local player holds, for the Collection's second-saber button.
        crate::saber_skins::set_local_pair(stage.sabers.each_ref().map(|saber| {
            saber
                .as_ref()
                .map(|saber| crate::saber_hilts::has_other_half(&saber.blades, saber.num_blades))
        }));
        // Found again below while a lone staff is drawn whole.
        crate::saber_skins::set_staff_halves(None);
        let two_sabers = stage.sabers[1].is_some();
        // The classic preview's blades are drawn into it, not into the world.
        let preview = stage.preview_only;
        let Some(actor) = &stage.actor else {
            stage.preview.showcase = None;
            return;
        };
        // Lightsaber creation lays the sabers out on their own, turning, in
        // front of the preview camera (the model faces its yaw, as there).
        let showcase = (preview && (stage.showcase || stage.lone.is_some())).then(|| {
            let facing =
                actor.rotation * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * Vec3::X;
            View::new(
                actor.origin + Vec3::Z * showcase::FOCUS_ABOVE_ORIGIN,
                facing,
                stage
                    .sabers
                    .each_ref()
                    .map(|saber| saber.as_ref().and_then(|saber| saber.line)),
            )
        });
        stage.preview.showcase = showcase;
        let shown = stage
            .sabers
            .iter()
            .flatten()
            .filter(|saber| saber.line.is_some())
            .count();
        // The Collection's Shaders tab lays them before the backdrop camera instead, as
        // long as the longest of them asks; the Saber Lab's lays a copy of them there,
        // over its own area, and the model keeps them in its hands.
        let lab = stage.lab.is_some();
        let area = if lab {
            crate::console::collection_panel::LAB_SABER_AREA
        } else {
            crate::console::collection_panel::SABER_AREA
        };
        let placed = stage
            .lone
            .filter(|_| !preview)
            .zip(frame_view)
            .map(|(turn, view)| {
                let longest = stage
                    .sabers
                    .iter()
                    .flatten()
                    .filter_map(|saber| saber.line)
                    .map(|line| line.span.length())
                    .fold(0.0, f32::max);
                (turn, lone::Place::new(&view, longest, area))
            });
        let (lone, floating) = if lab { (None, placed) } else { (placed, None) };
        let seconds = crate::menu::art::motion::seconds();
        // On the live preview the Shaders tab's turn takes the place of retail's roll.
        let (roll, yaw) = match stage.lone.filter(|_| !lab) {
            Some(turn) => (turn.roll.to_degrees(), turn.yaw),
            None => (showcase::roll_degrees(seconds as f32), 0.0),
        };
        // A jumping preview's blades drawn in (none at all once they are off).
        let out = 1.0 - stage.blades_in.clamp(0.0, 1.0);
        // Where a lone staff's two halves lie on screen, for a click on it to choose one.
        let mut halves = [None; 2];
        // The Saber Lab lights the blades and puts them out as a match does, a frame's
        // growth at a time; elsewhere they are out in full.
        let lit = stage.lab.map(|lab| lab.lit);
        let clash = stage.lab.map(|lab| lab.clash);
        let millis = (seconds * 1_000.0) as i64;
        // On the stage (not in a preview) the blades trail as long as the move the model's
        // torso plays has a match's trail last (a stance none, an attack 200 ms).
        let trail = (trails && !preview).then(|| trail_length(&actor.upper.sequence.name));
        for (hand, saber) in stage.sabers.iter_mut().enumerate() {
            let Some(saber) = saber else {
                continue;
            };
            // A Clash asked for since the last frame strikes every blade of the hilt.
            let clashed = saber.clash_seen.is_some() && saber.clash_seen != clash;
            saber.clash_seen = clash;
            if clashed {
                for index in 0..8 {
                    saber.motion[index].clash(millis);
                    saber.float_motion[index].clash(millis);
                }
                stage.lab_sounds.clash();
            }
            let mut lengths = [0.0; 8];
            let mut since = [None; 8];
            for (index, blade) in saber.blades.iter().enumerate() {
                let full = blade.as_ref().map_or(0.0, |blade| blade.length);
                since[index] = saber.ignition[index].update(lit.unwrap_or(true), millis);
                lengths[index] = match lit {
                    Some(lit) => saber.extension[index].update(lit, full, millis),
                    None => {
                        saber.extension[index] = saber::Extension::default();
                        full
                    }
                };
                saber.ignition[index].retracted(lengths[index], millis);
            }
            let pose = match (&showcase, &lone, actor.hands[hand]) {
                (Some(view), _, _) => saber
                    .line
                    .map(|line| view.pose(&line, View::offset(hand, shown), roll, yaw)),
                (None, Some((turn, place)), _) => saber
                    .line
                    .map(|line| place.pose(&line, View::offset(hand, shown), *turn)),
                (None, None, Some(attachment)) => Some(match stage.flying[hand] {
                    Some(pose) => (pose.grip, pose.rotation),
                    None => saber::world_attachment(actor.origin, actor.rotation, attachment),
                }),
                (None, None, None) => None,
            };
            let Some((grip, rotation)) = pose else {
                continue;
            };
            // The blades lit by each skin this saber wears (a staff's other half may wear
            // the second), and how far each side's stars surge, for its light.
            let mut light_blades = [[None; 8]; 2];
            let mut light_colors = [saber.color; 2];
            let mut surge = [0.0_f32; 2];
            // The showcase and the lone saber turn every frame.
            if stage.sabers_dirty || showcase.is_some() || lone.is_some() {
                // In the hand (and on the showcase) the hilt shares the
                // model's light sample; thrown, the grid is read where the
                // hilt actually is.
                // A preview's are lit as its actor is, the same on every map.
                let light = match stage.flying[hand]
                    .filter(|_| showcase.is_none() && !actor.preview_lit)
                {
                    Some(pose) => self
                        .entity_lighting
                        .sample(&self.bsp, pose.grip.to_array(), &[]),
                    None => actor.light,
                };
                let mut instance =
                    ActorInstance::new(grip.to_array(), rotation.to_array(), [1.0; 3]);
                instance.set_light(light);
                self.queue
                    .write_buffer(&saber.instance_buffer, 0, bytemuck::bytes_of(&instance));
            }
            // A skin with smoke: the emitters smoke while lit and a while after (into the
            // world, not into a preview's picture), each blade's by its side's skin.
            if !preview {
                let mut sink = crate::saber_smoke::Sink {
                    particles: &mut self.particles,
                    effects: &mut self.effects,
                    atlas: Some(&self.particle_atlas),
                    ribbons: &mut self.smoke_ribbons,
                    sparks: &mut self.saber_sparks,
                    clashes: &[],
                    heard: None,
                    now,
                };
                for (index, blade) in saber
                    .blades
                    .iter()
                    .take(usize::from(saber.num_blades))
                    .enumerate()
                    .filter_map(|(index, blade)| Some((index, blade.as_ref()?)))
                {
                    let other_half =
                        crate::saber_hilts::other_half(&saber.blades, saber.num_blades, index);
                    let Some(skin) =
                        crate::saber_skins::skin_of_blade(skins, two_sabers, hand, other_half)
                    else {
                        continue;
                    };
                    let (smoke, ribbon, sparks) = (skin.smoke, skin.ribbon, skin.sparks);
                    let length = lengths[index] * out;
                    let reach =
                        saber::world_blade(grip, rotation, blade.socket, length, blade.radius);
                    let at = |along: f32| {
                        Vec3::from_array(reach.base)
                            + Vec3::from_array(reach.direction) * (length * along)
                    };
                    let blade = (hand * 8 + index) as u32;
                    if let Some(spec) = smoke {
                        saber.smoke[index].emit(
                            &spec,
                            at(spec.along),
                            length >= 0.5,
                            presentation_time,
                            blade,
                            &mut sink,
                        );
                    }
                    if let Some(spec) = ribbon {
                        sink.ribbons.emit(
                            STAGE_RIBBONS + u64::from(blade),
                            &spec,
                            at(spec.along),
                            length >= 0.5,
                            presentation_time,
                        );
                    }
                    for (set, spec) in sparks.iter().enumerate() {
                        let Some(spec) = spec else { continue };
                        let key = (STAGE_RIBBONS + u64::from(blade)).wrapping_mul(8) + set as u64;
                        if clashed && length >= 0.5 {
                            sink.kick(key, spec.clash);
                        }
                        let bursts = sink.spark(
                            key,
                            spec,
                            at(spec.along),
                            Vec3::from_array(reach.direction),
                            length >= 0.5,
                            presentation_time,
                        );
                        // The Saber Lab snaps with the hand's bolts (a skin's crackle).
                        if matches!(spec.shape, crate::saber_sparks::Shape::Bolts { .. }) {
                            stage.lab_sounds.snap(bursts);
                        }
                    }
                }
            }
            for (index, blade) in saber
                .blades
                .iter()
                .take(usize::from(saber.num_blades))
                .enumerate()
                .filter_map(|(index, blade)| Some((index, blade.as_ref()?)))
                .filter(|(index, _)| out > 0.0 && lengths[*index] > 0.0)
            {
                let trail_style = blade.trail_style;
                let blade = saber::world_blade(
                    grip,
                    rotation,
                    blade.socket,
                    lengths[index] * out,
                    blade.radius,
                );
                let other_half =
                    crate::saber_hilts::other_half(&saber.blades, saber.num_blades, index);
                if lone.is_some() && !two_sabers && hand == 0 {
                    let middle = Vec3::from_array(blade.base)
                        + Vec3::from_array(blade.direction) * (blade.length * 0.5);
                    let half = &mut halves[usize::from(other_half)];
                    if half.is_none() {
                        *half = frame_view.and_then(|view| view.project_within(middle, 4.0));
                    }
                }
                let skin = crate::saber_skins::skin_of_blade(skins, two_sabers, hand, other_half);
                let color = skin.map_or(saber.color, |skin| {
                    BladeColor::Skin(skin.worn_with(saber.color))
                });
                let side = usize::from(!two_sabers && other_half && skins[0] != skins[1]);
                light_colors[side] = color;
                let seed = (hand * 8 + index) as u32;
                let ghosts = match color {
                    BladeColor::Skin(skin) => skin.ghosts,
                    _ => None,
                };
                if let Some(spec) = ghosts {
                    saber.ghosts[index].record(blade, presentation_time, spec);
                }
                let (moments, motion) = match color {
                    BladeColor::Skin(skin) => (
                        skin.flash_moments(since[index], saber.ignition[index].gone_for(millis)),
                        saber.motion[index].record_turned(
                            blade,
                            blade.direction,
                            millis,
                            lengths[index] < saber.blades[index].as_ref().map_or(0.0, |b| b.length),
                            None,
                            skin.each_swing,
                        ),
                    ),
                    _ => (saber::Moments::default(), crate::saber_motion::STILL),
                };
                surge[side] = surge[side].max(moments.surge);
                let pair = saber::Instance::pair(blade, color)
                    .map(|i| i.with_animation(seconds, seed).with_motion(motion))
                    .into_iter()
                    .chain(
                        saber::skin_extras(blade, color, seconds, seed)
                            .map(move |i| i.with_moments(moments).with_motion(motion)),
                    )
                    .chain(ghosts.into_iter().flat_map(|spec| {
                        saber.ghosts[index]
                            .instances(blade, color, spec)
                            .map(move |i| i.with_animation(seconds, seed))
                    }));
                if preview {
                    stage.preview.blades.extend(pair);
                } else {
                    self.saber_instances.extend(pair);
                    light_blades[side][index] = Some(blade);
                }
                if let Some(length) = trail
                    && let Some(quad) = saber.trails[index].update(
                        crate::saber_trail::Edges::new(None).edge(blade),
                        presentation_time,
                        length,
                        trail_style,
                        color,
                        length > 0,
                    )
                {
                    self.saber_trail_segments.insert(quad, presentation_time);
                }
            }
            // Put out, a skin's emitter stars sputter on a while (`flash.die`), each blade by
            // its side's skin.
            for (index, blade) in saber
                .blades
                .iter()
                .take(usize::from(saber.num_blades))
                .enumerate()
                .filter_map(|(index, blade)| Some((index, blade.as_ref()?)))
                .filter(|(index, _)| out > 0.0 && lengths[*index] <= 0.0)
            {
                let other_half =
                    crate::saber_hilts::other_half(&saber.blades, saber.num_blades, index);
                let Some(skin) =
                    crate::saber_skins::skin_of_blade(skins, two_sabers, hand, other_half)
                else {
                    continue;
                };
                let skin = skin.worn_with(saber.color);
                let color = BladeColor::Skin(skin);
                let ignition = &mut saber.ignition[index];
                let moments = skin.flash_moments(None, ignition.gone_for(millis));
                // The Saber Lab snaps once as the hand's sputter ends.
                let die = skin.flash.map_or(0.0, |flash| flash.die);
                if ignition.sputtered(die, millis) {
                    stage.lab_sounds.snap(1);
                }
                if moments.dying <= 0.0 {
                    continue;
                }
                // On the whole blade's line, where its stars were while it was lit.
                let blade =
                    saber::world_blade(grip, rotation, blade.socket, blade.length, blade.radius);
                let seed = (hand * 8 + index) as u32;
                let stars = saber::Instance::flashes(blade, color)
                    .map(move |i| i.with_animation(seconds, seed).with_moments(moments));
                if preview {
                    stage.preview.blades.extend(stars);
                } else {
                    self.saber_instances.extend(stars);
                }
            }
            // They light the stage as a match's light the map (a preview is a picture), each
            // side by its own skin.
            for (side, (blades, color)) in light_blades.iter().zip(light_colors).enumerate() {
                if blades.iter().any(Option::is_some) {
                    crate::saber_submission::lights::append(
                        &mut self.dynamic_lights,
                        blades,
                        color,
                        saber.num_blades,
                        false,
                        millis,
                        (hand + side * 8) as u64,
                        (hand * 8) as u32,
                        surge[side],
                    );
                }
            }
            // The Saber Lab's copy floats on its area, turned as the tab says, lit as the
            // model is; its blades animate on seeds of their own.
            let Some((turn, place)) = &floating else {
                continue;
            };
            let Some((grip, rotation)) = saber
                .line
                .map(|line| place.pose(&line, View::offset(hand, shown), *turn))
            else {
                continue;
            };
            let mut instance = ActorInstance::new(grip.to_array(), rotation.to_array(), [1.0; 3]);
            instance.set_light(actor.light);
            self.queue.write_buffer(
                &saber.float_instance_buffer,
                0,
                bytemuck::bytes_of(&instance),
            );
            // The floating copy smokes too, lit or a while after, each blade by its side's skin.
            if !preview {
                let mut sink = crate::saber_smoke::Sink {
                    particles: &mut self.particles,
                    effects: &mut self.effects,
                    atlas: Some(&self.particle_atlas),
                    ribbons: &mut self.smoke_ribbons,
                    sparks: &mut self.saber_sparks,
                    clashes: &[],
                    heard: None,
                    now,
                };
                for (index, blade) in saber
                    .blades
                    .iter()
                    .take(usize::from(saber.num_blades))
                    .enumerate()
                    .filter_map(|(index, blade)| Some((index, blade.as_ref()?)))
                {
                    let other_half =
                        crate::saber_hilts::other_half(&saber.blades, saber.num_blades, index);
                    let Some(skin) =
                        crate::saber_skins::skin_of_blade(skins, two_sabers, hand, other_half)
                    else {
                        continue;
                    };
                    let (smoke, ribbon, sparks) = (skin.smoke, skin.ribbon, skin.sparks);
                    let length = lengths[index];
                    let reach =
                        saber::world_blade(grip, rotation, blade.socket, length, blade.radius);
                    let at = |along: f32| {
                        Vec3::from_array(reach.base)
                            + Vec3::from_array(reach.direction) * (length * along)
                    };
                    let blade = (64 + hand * 8 + index) as u32;
                    if let Some(spec) = smoke {
                        saber.float_smoke[index].emit(
                            &spec,
                            at(spec.along),
                            length >= 0.5,
                            presentation_time,
                            blade,
                            &mut sink,
                        );
                    }
                    if let Some(spec) = ribbon {
                        sink.ribbons.emit(
                            STAGE_RIBBONS + u64::from(blade),
                            &spec,
                            at(spec.along),
                            length >= 0.5,
                            presentation_time,
                        );
                    }
                    for (set, spec) in sparks.iter().enumerate() {
                        let Some(spec) = spec else { continue };
                        let key = (STAGE_RIBBONS + u64::from(blade)).wrapping_mul(8) + set as u64;
                        if clashed && length >= 0.5 {
                            sink.kick(key, spec.clash);
                        }
                        sink.spark(
                            key,
                            spec,
                            at(spec.along),
                            Vec3::from_array(reach.direction),
                            length >= 0.5,
                            presentation_time,
                        );
                    }
                }
            }
            for (index, blade) in saber
                .blades
                .iter()
                .take(usize::from(saber.num_blades))
                .enumerate()
                .filter_map(|(index, blade)| Some((index, blade.as_ref()?)))
                .filter(|(index, _)| lengths[*index] > 0.0)
            {
                // Each blade wears its side's skin, as in the hand.
                let other_half =
                    crate::saber_hilts::other_half(&saber.blades, saber.num_blades, index);
                let skin = crate::saber_skins::skin_of_blade(skins, two_sabers, hand, other_half);
                let color = skin.map_or(saber.color, |skin| {
                    BladeColor::Skin(skin.worn_with(saber.color))
                });
                let socket = blade.socket;
                let (radius, trail_style) = (blade.radius, blade.trail_style);
                let blade = saber::world_blade(grip, rotation, socket, lengths[index], radius);
                if still_trail {
                    // As far back as a swing's trail reaches, an attack's colour and age.
                    let edges: [crate::saber_trail::Edge; STILL_SLICES + 1] =
                        std::array::from_fn(|slice| {
                            let back = glam::Quat::from_axis_angle(
                                toward,
                                -STILL_ARC * slice as f32 / STILL_SLICES as f32,
                            );
                            crate::saber_trail::Edges::new(None).edge(saber::world_blade(
                                grip,
                                back * rotation,
                                socket,
                                lengths[index],
                                radius,
                            ))
                        });
                    self.saber_trail_segments.add_still(
                        crate::saber_trail::still_trail(&edges, STILL_TRAIL, trail_style, color)
                            .collect::<Vec<_>>(),
                    );
                }
                let seed = (64 + hand * 8 + index) as u32;
                let reach = match color {
                    BladeColor::Skin(skin) => saber.float_motion[index].record_turned(
                        blade,
                        blade.direction,
                        millis,
                        false,
                        None,
                        skin.each_swing,
                    ),
                    _ => crate::saber_motion::STILL,
                };
                self.saber_instances.extend(
                    saber::Instance::pair(blade, color)
                        .map(|i| i.with_animation(seconds, seed).with_motion(reach))
                        .into_iter()
                        .chain(
                            saber::skin_extras(blade, color, seconds, seed)
                                .map(move |i| i.with_motion(reach)),
                        ),
                );
            }
        }
        crate::saber_skins::set_staff_halves(match halves {
            [Some(first), Some(other)] => Some([first, other]),
            _ => None,
        });
        stage.sabers_dirty = false;
    }
}

/// Give a hilt with nothing to draw one unused vertex and triangle, and no draws.
/// A community "empty hilt" (a model of only its `*blade1` tag, so the player holds
/// a blade with no hilt in a stance) flattens to no vertices; a zero-sized buffer bound
/// for storage fails wgpu's validation, and the uncaptured error closed the game, at
/// every start once the hilt was saved (#214). Its blades stay.
fn pad_empty_mesh(flattened: &mut FlattenedScene, draws: &mut Vec<ActorDraw>) {
    if !flattened.vertices.is_empty() && !flattened.indices.is_empty() {
        return;
    }
    draws.clear();
    if flattened.vertices.is_empty() {
        flattened
            .vertices
            .push(<GpuVertex as bytemuck::Zeroable>::zeroed());
    }
    flattened.indices = vec![0; 3];
}

/// Where the stage's ribbon emitters' keys start, past every entity's
/// (`saber_ribbons::Ribbons::emit`): the hands', then the floating copy's.
const STAGE_RIBBONS: u64 = 1 << 62;

/// The floating saber's still trail: the slices it is drawn in, how far back round it
/// reaches (radians), and the move whose trail it shows (an attack's, ms).
const STILL_SLICES: usize = 8;
const STILL_ARC: f32 = 0.45;
const STILL_TRAIL: u16 = 200;

/// How long a match's trail of the move playing `animation` lasts (`saberMoveData`'s
/// trail length, ms before `CG_AddSaberBlade` divides it by five): the move whose
/// animation it is, an attack, start, return, transition or parry in any of the seven
/// styles, each a group of animations further on than the fast style's
/// (`SABER_ANIM_GROUP_SIZE`); 0 for anything else (a stance, the draw and the put-away).
fn trail_length(animation: &str) -> u16 {
    use sjk_game_jka::{legacy_animation_index, saber_move_data::SABER_MOVES};
    static GROUP: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    let group = *GROUP.get_or_init(|| {
        legacy_animation_index("BOTH_A2_T__B_")?
            .checked_sub(legacy_animation_index("BOTH_A1_T__B_")?)
    });
    let (Some(index), Some(group)) = (legacy_animation_index(animation), group) else {
        return 0;
    };
    // Only the attacks (`LS_A_TL2BR` to `LS_A_T2B`) and the starts, returns,
    // transitions, bounces and deflections (`LS_S_TL2BR` to `LS_V1_B_`) take the style's
    // group (`PM_SetSaberMove`); the parries, broken parries, knockaways and reflects
    // ("have only 1 level") and the rest play the one animation.
    let styled = |number: usize| (4..=10).contains(&number) || (62..=140).contains(&number);
    SABER_MOVES
        .iter()
        .enumerate()
        .filter(|(_, movement)| movement.trail_len > 0)
        .find(|(number, movement)| {
            let styles = if styled(*number) { 0..7 } else { 0..1 };
            styles
                .into_iter()
                .any(|style| usize::from(movement.animation) + style * group == index)
        })
        .map_or(0, |(_, movement)| movement.trail_len)
}

/// Sideways distance between two floating hilts.
const FLOAT_SPACING: f32 = 17.0;
/// Height of a floating hilt's blade root above the focus point. Hilts
/// keep their model origin at different distances below the blade
/// (`single_1` 4.5 units, `single_9` 6.0, the staffs 11.6), so the blade,
/// not the origin, is what is placed on the focus: cycling hilts swaps the
/// hilt under a blade that stays put.
const FLOAT_ROOT: f32 = 4.5;

/// Where along the focus's right axis `hand`'s hilt floats when `held`
/// hands carry one: centred alone, the right hand's hilt to the viewer's
/// left of a pair.
fn float_offset(hand: usize, held: usize) -> f32 {
    if held < 2 {
        0.0
    } else {
        (hand as f32 - 0.5) * FLOAT_SPACING
    }
}

/// Where a hilt lies on the showcase: along its first used blade, its span
/// taken over the mesh and every used blade's root and tip.
fn showcase_line(blades: &[Option<HiltBlade>; 8], used: u8, mesh: &[GpuVertex]) -> Option<Line> {
    let used = blades.iter().take(usize::from(used)).flatten();
    let first = used.clone().next()?;
    let ends = used.flat_map(|blade| {
        let root = Vec3::from_array(blade.socket.origin);
        let direction = Vec3::from_array(blade.socket.direction).normalize_or_zero();
        [root, root + direction * blade.length]
    });
    let points = mesh
        .iter()
        .map(|vertex| Vec3::from_array(vertex.position))
        .chain(ends);
    Line::new(
        Vec3::from_array(first.socket.origin),
        Vec3::from_array(first.socket.direction),
        points,
    )
}

/// Both hand bolts of the whole-body frame the stage was skinned with.
pub(super) fn hand_attachments(preview: &PlayerPreview, frame: usize) -> [Option<Attachment>; 2] {
    let Ok(matrices) = preview.animation.frame_matrices(frame) else {
        return [None; 2];
    };
    hand_attachments_at(preview, &matrices)
}

/// Both hand bolts of the pose `matrices` the stage was skinned with.
pub(super) fn hand_attachments_at(
    preview: &PlayerPreview,
    matrices: &[[[f32; 4]; 3]],
) -> [Option<Attachment>; 2] {
    [HAND_BOLT, LEFT_HAND_BOLT].map(|bolt| saber::attachment_from_matrices(preview, matrices, bolt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_hilt_uploads_no_empty_buffer_and_draws_nothing() {
        let mut flattened = FlattenedScene::default();
        let mut draws = Vec::new();
        pad_empty_mesh(&mut flattened, &mut draws);
        assert_eq!(flattened.vertices.len(), 1);
        assert_eq!(flattened.indices, [0; 3]);
        assert!(draws.is_empty());

        // A surface with vertices but no triangles draws nothing either.
        let mut flattened = FlattenedScene::default();
        flattened
            .vertices
            .push(<GpuVertex as bytemuck::Zeroable>::zeroed());
        let mut draws = vec![ActorDraw {
            indices: 0..0,
            material: 0,
        }];
        pad_empty_mesh(&mut flattened, &mut draws);
        assert_eq!(flattened.vertices.len(), 1);
        assert_eq!(flattened.indices, [0; 3]);
        assert!(draws.is_empty());
    }

    #[test]
    fn a_hilt_with_a_mesh_is_left_as_it_is() {
        let mut flattened = FlattenedScene::default();
        flattened
            .vertices
            .extend([<GpuVertex as bytemuck::Zeroable>::zeroed(); 3]);
        flattened.indices = vec![0, 1, 2];
        let mut draws = vec![ActorDraw {
            indices: 0..3,
            material: 0,
        }];
        pad_empty_mesh(&mut flattened, &mut draws);
        assert_eq!(flattened.vertices.len(), 3);
        assert_eq!(draws.len(), 1);
    }

    /// The stage's hilt buffers and geometry binding, as `build_stage_saber` makes them.
    fn bind_hilt(device: &wgpu::Device, flattened: &FlattenedScene) -> wgpu::BindGroup {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hilt test vertices"),
            contents: bytemuck::cast_slice(&flattened.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        });
        let quads = crate::shared_geometry::quads::upload(
            device,
            flattened.vertices.len(),
            &flattened.indices,
            0,
        );
        crate::shared_geometry::quads::bind(device, &vertices, &quads)
    }

    #[test]
    #[ignore = "requires an available GPU; uses no installed game assets"]
    fn an_empty_hilt_binds_without_a_wgpu_error_once_padded() {
        let gpu = pollster::block_on(crate::gpu_context::Context::new(None, None)).unwrap();
        let device = &gpu.device;
        let empty = FlattenedScene::default();
        assert!(
            crate::gpu_guard::capture(device, || bind_hilt(device, &empty)).is_err(),
            "an empty hilt's zero-sized storage binding is what closed the game (#214)"
        );
        let mut padded = FlattenedScene::default();
        pad_empty_mesh(&mut padded, &mut Vec::new());
        assert!(crate::gpu_guard::capture(device, || bind_hilt(device, &padded)).is_ok());
    }

    /// The starts, attacks and returns of every style trail as long as a match's, the
    /// transitions and parries less, the stances and the draw and put-away not at all.
    #[test]
    fn the_stage_trails_as_long_as_the_move_in_a_match() {
        for attack in [
            "BOTH_S1_S1_T_",
            "BOTH_A1_T__B_",
            "BOTH_R1_B__S1",
            "BOTH_A3__L__R",
            "BOTH_A6_TL_BR",
            "BOTH_R7__R_S7",
        ] {
            assert_eq!(trail_length(attack), 200, "{attack}");
        }
        assert_eq!(trail_length("BOTH_P1_S1_T_"), 150);
        for still in [
            "BOTH_STAND2",
            "BOTH_SABERSLOW_STANCE",
            "BOTH_STAND1TO2",
            "NOT_AN_ANIM",
        ] {
            assert_eq!(trail_length(still), 0, "{still}");
        }
    }
}
