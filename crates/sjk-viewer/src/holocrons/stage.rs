//! The Holocrons tab's 3D holocron ([`crate::console::holocrons_panel`]): one holocron floating in
//! the world behind the tab, over the left of the screen where the page leaves it clear.
//! It turns and bobs as Illuminate's does ([`crate::illuminate`]) and is the holocron of
//! the tier chosen in the list, each tier an object of its own (`docs/holocrons.md`, "The
//! four holocrons"): the Jedi cube, the Great Holocron with its turning halo, the Sith
//! pyramid with its floating cap and eye, the gold engram with its glowing seams and
//! sparks, and a point light in the tier's colour. It also draws the four kyber
//! crystals (`kyber_<tier>`: a crystal in its sheath, tumbling rings, motes), asked by the
//! hub's tier id ([`Request::of`]). For a tier the player holds none of
//! it is the locked look: the same shape drained and dark, its parts at rest, with a faint
//! light (padlock-like dimming; it is never hidden, so every tier can be seen).
//!
//! It is an ordinary object of the world pass: an instance of each part's model in
//! `object_groups`, loaded with the map's other rigid models ([`models`]), and a point
//! light, so the menu map's stage (the Character tab's) is not needed: it floats before
//! the backdrop camera ([`place`]), which the tab asks to park on its own shot
//! ([`crate::menu_backdrop::Shot::Holocrons`]). Over a match (the game menu's Collection)
//! it floats before the game's camera, the third-person one too: it is placed from the
//! view the frame renders (`GpuState::frame_view`), with that view's own field.
//!
//! A form is a model per part ([`FORMS`], written with the models by
//! `scripts/holocron_assets.py`): its body, and each part that moves against it, posed
//! from the time ([`Part::pose`]). The locked look has models of its own because a
//! surface's shader is named by its model file.
//!
//! Changing the tier shrinks the old look away and grows the new one ([`Stage`]); the
//! frame allocates nothing.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use crate::dynamic_lights::PointLight;
use crate::hud::identification::Camera;
use glam::{Quat, Vec3};
use std::time::Instant;

/// One part of a tier's holocron: its model in the tier's look and in the locked one, and
/// how it moves against the holocron (`scripts/holocron_forms.py`'s `Motion`). A moving
/// part is modelled about its own centre, which it turns about; the body about the
/// holocron's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Part {
    pub(crate) model: &'static str,
    pub(crate) locked: &'static str,
    /// Where its centre rests, in model units from the holocron's (z up).
    at: [f32; 3],
    /// Its bob along z: units, and radians per second.
    bob: [f32; 2],
    /// Its orbit about z: radius in units, radians per second, start angle.
    orbit: [f32; 3],
    /// Its turns, each a start angle and radians per second: about its own z (spin),
    /// then about x (tumble), then about the holocron's z (precess).
    spin: [f32; 2],
    tumble: [f32; 2],
    precess: [f32; 2],
}

/// One thing the stage can draw: a holocron tier or a kyber crystal, by the hub's tier id
/// (`mythical`, `kyber_rare`), the colour of its point light (linear RGB on the scale of
/// Illuminate's `[1.5, 1.3, 1.0]`, as `scripts/holocron_assets.py`'s `Tier.light`) and its
/// parts, the body first. [`FORMS`] is keyed by slot: the holocrons first (0-3, in
/// [`super::TIERS`] order, so a tier's index there is its slot here), then the kyber crystals
/// (4-7: uncommon, rare, legendary, mythical).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Form {
    pub(crate) id: &'static str,
    pub(crate) light: [f32; 3],
    pub(crate) parts: &'static [Part],
}

include!("../../assets/holocron/holocron_parts.rs");

/// The index in [`FORMS`] of the form the hub's tier `id` names (`rare`, `kyber_rare`), if
/// the stage can draw it.
pub(crate) fn form_index(id: &str) -> Option<usize> {
    FORMS.iter().position(|form| form.id == id)
}

impl Part {
    /// Where it is and how it is turned `seconds` in, against the holocron.
    pub(crate) fn pose(&self, seconds: f32) -> (Vec3, Quat) {
        let turn = |[start, rate]: [f32; 2]| start + rate * seconds;
        let rotation = Quat::from_rotation_z(turn(self.precess))
            * Quat::from_rotation_x(turn(self.tumble))
            * Quat::from_rotation_z(turn(self.spin));
        let [radius, rate, start] = self.orbit;
        let angle = start + rate * seconds;
        let [lift, pace] = self.bob;
        let position = Vec3::from_array(self.at)
            + Vec3::new(
                radius * angle.cos(),
                radius * angle.sin(),
                lift * (pace * seconds).sin(),
            );
        (position, rotation)
    }

    /// Whether it moves against the holocron at all.
    #[cfg(test)]
    pub(crate) fn moves(&self) -> bool {
        self.pose(0.0) != self.pose(1.0)
    }
}

/// Every model the stage may draw, for the world to load with its other rigid models
/// (a model the table names twice, the motes', once).
pub(crate) fn models() -> impl Iterator<Item = &'static str> {
    let every = || {
        FORMS
            .iter()
            .flat_map(|form| form.parts)
            .flat_map(|part| [part.model, part.locked])
    };
    every()
        .enumerate()
        .filter(move |(index, model)| every().position(|other| other == *model) == Some(*index))
        .map(|(_, model)| model)
}

/// What the tab asks the stage to show.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Request {
    /// The form to draw, its slot: an index into [`FORMS`], 0-3 the holocron tiers in
    /// [`super::TIERS`] order, 4-7 the kyber crystals (uncommon, rare, legendary,
    /// mythical); [`Request::of`] and [`form_index`] find it by the hub's id.
    pub(crate) tier: usize,
    /// The player holds at least one of it: its own look, else the locked one.
    pub(crate) owned: bool,
}

impl Request {
    /// The form the hub's tier `id` names (`mythical`, `kyber_rare`), in its own look when
    /// `owned`, else the locked one; `None` for an id the stage cannot draw.
    #[allow(dead_code)] // for the kyber crystals' screens, which know a form by the hub's id
    pub(crate) fn of(id: &str, owned: bool) -> Option<Self> {
        form_index(id).map(|tier| Self { tier, owned })
    }

    /// Its form; an index out of range is the last one, never a panic.
    pub(crate) fn form(self) -> &'static Form {
        &FORMS[self.tier.min(FORMS.len() - 1)]
    }

    /// Its parts, the body first.
    pub(crate) fn parts(self) -> &'static [Part] {
        self.form().parts
    }

    /// The model that draws `part` in its look.
    pub(crate) fn model_of(self, part: &Part) -> &'static str {
        if self.owned { part.model } else { part.locked }
    }

    /// The model of its body.
    #[cfg(test)]
    pub(crate) fn model(self) -> &'static str {
        self.model_of(&self.parts()[0])
    }

    /// The time its parts are posed at, `seconds` in: a locked holocron's rest still.
    pub(crate) fn part_seconds(self, seconds: f32) -> f32 {
        if self.owned { seconds } else { 0.0 }
    }

    /// The light it casts: the form's colour, a fifth as strong while locked.
    pub(crate) fn light(self) -> [f32; 3] {
        let light = self.form().light;
        let strength = if self.owned { 1.0 } else { LOCKED_LIGHT };
        light.map(|channel| channel * strength)
    }
}

/// How the opening moves the holocron (`docs/cards.md`, "The ceremony"): it glides to the
/// middle of the window, spins up, swells and brightens, shakes, then is gone in the
/// flash. The tab's opening works it out each frame
/// ([`crate::console::holocrons_panel::Panel::stage_motion`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Motion {
    /// Where it floats, a point of the 1920 by 1080 frame (in place of the tab's
    /// [`crate::console::holocrons_panel::STAGE_AT`]).
    pub(crate) at: [f32; 2],
    /// Its turn about the vertical on top of the idle one, in radians.
    pub(crate) turn: f32,
    /// Its size against the tab's, and its light's strength against the tier's.
    pub(crate) swell: f32,
    pub(crate) glow: f32,
    /// False once the flash has taken it: nothing is drawn, and the next showing grows
    /// it from nothing.
    pub(crate) shown: bool,
}

/// How strong a locked holocron's light is against the tier's.
const LOCKED_LIGHT: f32 = 0.2;
/// Seconds to shrink away when the look changes, and to grow into the new one.
const OUT: f32 = 0.12;
const IN: f32 = 0.32;
/// The holocron's edge on screen, in the 1080p frame's pixels, and how far before the
/// camera it floats in world units (clear of the near plane and of the nearest walls).
pub(crate) const SIZE: f32 = 185.0;
const DISTANCE: f32 = 64.0;
/// The light's reach, in world units, at full level.
const RADIUS: f32 = 190.0;
/// The model units [`SIZE`] spans: the Jedi cube's edge (`CUBE_HALF` in
/// `scripts/holocron_forms.py`); the other forms are made to fill about the same room.
const EDGE: f32 = 6.0;

/// The look on show and how far it has grown (0 gone, 1 full size).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Stage {
    shown: Option<Request>,
    level: f32,
    last: Option<Instant>,
}

impl Stage {
    /// Step to `now` toward `wanted` (`None`: the tab is not on show) and return what
    /// to draw: the look and its level, `None` while nothing shows.
    pub(crate) fn advance(
        &mut self,
        wanted: Option<Request>,
        now: Instant,
    ) -> Option<(Request, f32)> {
        let dt = self
            .last
            .map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f32()
            })
            .min(0.1);
        self.last = Some(now);
        let Some(wanted) = wanted else {
            // Gone with the tab: the next opening grows it from nothing.
            *self = Self::default();
            return None;
        };
        if self.shown != Some(wanted) {
            if self.shown.is_some() && self.level > 0.0 {
                self.level = (self.level - dt / OUT).max(0.0);
            }
            if self.shown.is_none() || self.level <= 0.0 {
                self.shown = Some(wanted);
                self.level = 0.0;
            }
        } else {
            self.level = (self.level + dt / IN).min(1.0);
        }
        let shown = self.shown?;
        (self.level > 0.0).then_some((shown, crate::illuminate::smooth(self.level)))
    }
}

/// Where the holocron floats and how large to draw it before `view` (the camera the frame
/// renders, its vertical field and window): on the ray through the frame point `at` (1920
/// by 1080 frame pixels), [`DISTANCE`] ahead, its edge as wide as [`SIZE`] frame pixels
/// at `at`. Returns the centre and the model's scale.
pub(crate) fn place(view: &Camera, at: [f32; 2]) -> (Vec3, f32) {
    let viewport = view.viewport;
    let frame = crate::menu::sjk::Frame::new(viewport);
    let [x, y] = frame.point(at[0], at[1]);
    let (ndc_x, ndc_y) = (x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0);
    let half = (view.fov.to_radians() * 0.5).tan();
    let aspect = viewport[0] / viewport[1];
    // The basis `Camera::project` reads the view with.
    let forward = (view.target - view.eye).normalize_or_zero();
    let right = forward.cross(view.up).normalize_or_zero();
    let up = right.cross(forward);
    let centre = view.eye
        + forward * DISTANCE
        + right * (ndc_x * half * aspect * DISTANCE)
        + up * (ndc_y * half * DISTANCE);
    // World units across the window's height at that distance, over its pixels.
    let per_pixel = 2.0 * half * DISTANCE / viewport[1];
    (centre, SIZE * frame.s * per_pixel / EDGE)
}

impl GpuState {
    /// Add the Holocrons tab's holocron and its light to this frame's world, while the
    /// tab shows. Called with the other frame lights (`submit_illuminate`).
    pub(crate) fn submit_holocron_stage(&mut self, now: Instant) {
        let console = self.console.as_ref();
        let motion = console.and_then(|console| console.holocrons_motion(now));
        // Gone in the opening's flash: it grows again from nothing when shown.
        let wanted = console
            .and_then(crate::console::ViewerConsole::holocrons_stage)
            .filter(|_| motion.is_none_or(|motion| motion.shown));
        #[cfg(test)]
        let wanted = wanted.and(self.holocron_stage_request).or(wanted);
        let Some((request, level)) = self.holocron_stage.advance(wanted, now) else {
            return;
        };
        // The view this frame renders (set before the world is submitted), else the
        // camera's eye and angles with the renderer's vertical field.
        let view = self.frame_view.unwrap_or_else(|| {
            let (yaw, pitch) = (self.camera_yaw, self.camera_pitch);
            let forward = Vec3::new(
                yaw.cos() * pitch.cos(),
                yaw.sin() * pitch.cos(),
                pitch.sin(),
            );
            Camera {
                eye: self.camera_position,
                target: self.camera_position + forward,
                up: Vec3::Z,
                fov: self.field_of_view,
                viewport: [
                    self.configuration.width as f32,
                    self.configuration.height as f32,
                ],
            }
        });
        let (at, turn, swell, glow) = motion.map_or(
            (crate::console::holocrons_panel::STAGE_AT, 0.0, 1.0, 1.0),
            |motion| (motion.at, motion.turn, motion.swell, motion.glow),
        );
        let (centre, scale) = place(&view, at);
        let seconds = now.saturating_duration_since(self.ui_epoch).as_secs_f32();
        #[cfg(test)]
        let seconds = self.holocron_stage_seconds.unwrap_or(seconds);
        let position = centre + Vec3::Z * crate::illuminate::idle_bob(seconds) * scale;
        let size = scale * level * swell;
        self.dynamic_lights.push_radiant(PointLight {
            origin: position.to_array(),
            radius: RADIUS * level * glow.sqrt(),
            color: request.light().map(|channel| channel * glow),
        });
        let rotation = Quat::from_rotation_z(turn) * crate::illuminate::idle_rotation(seconds);
        let part_seconds = request.part_seconds(seconds);
        for part in request.parts() {
            let model = request.model_of(part);
            let Some(mesh) = self
                .object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == model)
            else {
                continue;
            };
            let (at, turned) = part.pose(part_seconds);
            self.object_groups[mesh].push(ActorInstance::new(
                (position + rotation * (at * size)).to_array(),
                (rotation * turned).to_array(),
                [size; 3],
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn every_form_has_parts_of_its_own_named_by_its_id() {
        // The holocrons first, in the tiers' order and with their lights, then a kyber
        // crystal per tier in the same order and light.
        let tiers = &super::super::TIERS;
        assert_eq!(FORMS.len(), 2 * tiers.len());
        for (index, tier) in tiers.iter().enumerate() {
            assert_eq!(FORMS[index].id, tier.id);
            assert_eq!(FORMS[index].light, tier.light);
            let kyber = &FORMS[tiers.len() + index];
            assert_eq!(kyber.id, format!("kyber_{}", tier.id));
            assert_eq!(kyber.light, tier.light);
            assert_eq!(form_index(tier.id), Some(index));
            assert_eq!(form_index(kyber.id), Some(tiers.len() + index));
        }
        assert_eq!(form_index("kyber"), None);
        for form in &FORMS {
            let parts = form.parts;
            // The body first, standing still at the centre.
            let body = parts[0];
            assert_eq!(body.model, format!("models/sjk/holocron_{}.md3", form.id));
            assert!(!body.moves(), "{}", form.id);
            assert_eq!(body.pose(3.0), (Vec3::ZERO, Quat::IDENTITY));
            for part in parts {
                let prefix = format!("models/sjk/holocron_{}", form.id);
                assert!(part.model.starts_with(&prefix), "{}", part.model);
                assert_eq!(
                    part.locked,
                    part.model.replace(".md3", "_locked.md3"),
                    "{}",
                    part.model
                );
            }
            // Every other part moves.
            assert!(parts[1..].iter().all(Part::moves), "{}", form.id);
        }
        let all: Vec<_> = models().collect();
        let unique: std::collections::BTreeSet<_> = all.iter().collect();
        assert_eq!(all.len(), unique.len(), "no model twice");
        assert!(
            !all.contains(&crate::illuminate::MODEL),
            "Illuminate's stays its own"
        );
        // The four holocrons: a still cube, a halo, a cap and an eye, the engram with
        // eight sparks round it; then the kyber crystals, plainer the commoner: one ring
        // and four motes, six, both rings and eight, ten.
        let lengths: Vec<_> = FORMS.iter().map(|form| form.parts.len()).collect();
        assert_eq!(lengths, [1, 2, 3, 9, 7, 9, 12, 14]);
        assert_eq!(all.len(), 2 * (1 + 2 + 3 + 2 + 4 + 4 + 5 + 5));
    }

    /// A part's pose follows its table: the Sith pyramid's cap floats over the body and
    /// bobs, a mote circles at its height, and a locked holocron's parts keep still.
    #[test]
    fn parts_move_by_their_table_and_rest_while_locked() {
        let cap = FORMS[2].parts[1];
        let (rest, turned) = cap.pose(0.0);
        assert_eq!(turned, Quat::IDENTITY);
        assert!(rest.z > 2.0, "above the body: {rest}");
        let (later, turned) = cap.pose(1.0);
        assert!((later.z - rest.z).abs() > 0.1, "it bobs");
        assert!(turned.angle_between(Quat::IDENTITY) > 0.1, "it turns");
        // An engram's spark, then a kyber crystal's mote.
        for mote in [FORMS[3].parts[1], FORMS[7].parts[4]] {
            let heights = [0.0, 1.0, 2.0].map(|t| mote.pose(t).0.z);
            assert!(heights.iter().all(|z| (z - heights[0]).abs() < 1e-5));
            let radii = [0.0, 1.0, 2.0].map(|t| mote.pose(t).0.truncate().length());
            assert!(
                radii
                    .iter()
                    .all(|r| (r - radii[0]).abs() < 1e-4 && *r > 2.0)
            );
            assert_ne!(mote.pose(0.0).0, mote.pose(1.0).0, "it circles");
        }
        let locked = Request {
            tier: 3,
            owned: false,
        };
        assert_eq!(locked.part_seconds(12.5), 0.0);
        let owned = Request {
            tier: 3,
            owned: true,
        };
        assert_eq!(owned.part_seconds(12.5), 12.5);
    }

    /// Every part's model is bundled and loads, a moving one is modelled about its own
    /// centre (which it turns about), and the forms fill about the room of the cube.
    #[test]
    fn every_part_is_bundled_and_moves_about_its_centre() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        crate::illuminate::mount(&mut vfs).unwrap();
        for form in &FORMS {
            let mut reach = 0.0_f32;
            for part in form.parts {
                for model in [part.model, part.locked] {
                    let bytes = vfs
                        .read(model)
                        .unwrap()
                        .unwrap_or_else(|| panic!("{model} is not mounted"))
                        .bytes;
                    let md3 = sjk_model::Md3::parse(&bytes).unwrap();
                    let points: Vec<Vec3> = md3
                        .surfaces
                        .iter()
                        .flat_map(|surface| &surface.frames[0])
                        .map(|vertex| Vec3::from_array(vertex.position))
                        .collect();
                    let (low, high) = points.iter().fold(
                        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                        |(low, high), p| (low.min(*p), high.max(*p)),
                    );
                    if part.moves() {
                        // The middle of its box, or (for a lopsided shape, whose
                        // box is off its centre) of its corners.
                        let middle = (low + high) * 0.5;
                        let mean = points.iter().sum::<Vec3>() / points.len() as f32;
                        let centre = if middle.length() < mean.length() {
                            middle
                        } else {
                            mean
                        };
                        assert!(centre.length() < 0.2, "{model}: centred at {centre}");
                    }
                    let (at, _) = part.pose(0.0);
                    let far = points.iter().map(|p| p.length()).fold(0.0, f32::max);
                    reach = reach.max(at.length() + far);
                }
            }
            assert!(
                (4.0..8.0).contains(&reach),
                "{}: reaches {reach} units",
                form.id
            );
        }
    }

    #[test]
    fn a_request_picks_the_tiers_model_and_light_or_the_locked_ones() {
        let owned = Request {
            tier: 3,
            owned: true,
        };
        assert_eq!(owned.model(), "models/sjk/holocron_mythical.md3");
        assert_eq!(owned.light(), super::super::TIERS[3].light);
        let locked = Request {
            tier: 3,
            owned: false,
        };
        assert_eq!(locked.model(), "models/sjk/holocron_mythical_locked.md3");
        let dim = locked.light();
        for (dim, full) in dim.into_iter().zip(owned.light()) {
            assert!((dim - full * LOCKED_LIGHT).abs() < 1e-6);
        }
        // The tier shows in the dimmed light: the colours stay apart.
        assert!(
            Request {
                tier: 0,
                owned: false
            }
            .light()[1]
                > Request {
                    tier: 0,
                    owned: false
                }
                .light()[0]
        );
        // An index out of range is the last form, never a panic.
        assert_eq!(
            Request {
                tier: 99,
                owned: true
            }
            .model(),
            FORMS[FORMS.len() - 1].parts[0].model
        );
        // A kyber crystal by the hub's id: its own parts and its tier's light.
        let crystal = Request::of("kyber_rare", true).unwrap();
        assert_eq!(crystal.model(), "models/sjk/holocron_kyber_rare.md3");
        assert_eq!(crystal.light(), super::super::TIERS[1].light);
        assert_eq!(
            Request::of("kyber_mythical", false).unwrap().model(),
            "models/sjk/holocron_kyber_mythical_locked.md3"
        );
        assert_eq!(Request::of("mythical", true), Some(owned));
        assert_eq!(Request::of("kyber_common", true), None);
    }

    /// Step 60 frames a second for `seconds`; the last answer.
    fn run(
        stage: &mut Stage,
        wanted: Option<Request>,
        start: Instant,
        from: f32,
        seconds: f32,
    ) -> Option<(Request, f32)> {
        let mut last = None;
        let mut t = from;
        while t <= from + seconds + 1e-4 {
            last = stage.advance(wanted, start + Duration::from_secs_f32(t));
            t += 1.0 / 60.0;
        }
        last
    }

    #[test]
    fn it_grows_in_swaps_through_nothing_and_goes_with_the_tab() {
        let start = Instant::now();
        let mut stage = Stage::default();
        let rare = Request {
            tier: 1,
            owned: true,
        };
        let mythical = Request {
            tier: 3,
            owned: true,
        };
        // Nothing shows until a tab asks, then it grows from nothing.
        assert_eq!(run(&mut stage, None, start, 0.0, 0.2), None);
        let (_, level) = run(&mut stage, Some(rare), start, 0.2, 0.1).unwrap();
        assert!(level > 0.0 && level < 1.0, "growing: {level}");
        let (look, level) = run(&mut stage, Some(rare), start, 0.3, 0.6).unwrap();
        assert_eq!((look, level), (rare, 1.0));
        // A new tier shrinks the old one away (never both at once), then grows.
        let (look, level) = run(&mut stage, Some(mythical), start, 0.9, 0.05).unwrap();
        assert_eq!(look, rare);
        assert!(level < 1.0, "shrinking: {level}");
        let mut grown = None;
        let mut t = 0.95;
        while t < 2.0 {
            if let Some((look, level)) =
                stage.advance(Some(mythical), start + Duration::from_secs_f32(t))
            {
                if look == mythical {
                    grown = Some(level);
                    break;
                }
                assert_eq!(look, rare, "the old look shrinks until it is gone");
            }
            t += 1.0 / 60.0;
        }
        assert!(
            grown.is_some_and(|level| level < 1.0),
            "the new look starts small"
        );
        let (look, level) = run(&mut stage, Some(mythical), start, 1.5, 0.6).unwrap();
        assert_eq!((look, level), (mythical, 1.0));
        // The same tier going from locked to owned (the hub answered) swaps too.
        let locked = Request {
            tier: 3,
            owned: false,
        };
        let (look, _) = run(&mut stage, Some(locked), start, 2.12, 0.04).unwrap();
        assert_eq!(look, mythical);
        // The tab closes: gone at once, and a new opening starts from nothing again.
        assert_eq!(run(&mut stage, None, start, 2.6, 0.1), None);
        let (_, level) =
            run(&mut stage, Some(mythical), start, 2.8, 0.0).unwrap_or((mythical, 0.0));
        assert!(level < 0.2);
    }

    /// The holocron sits on the ray through the frame point asked, [`DISTANCE`] ahead of
    /// the camera, in every window shape, with the menu's field and a match's narrower
    /// one (a match over 16:9 at `cg_fov 80` renders about 64 degrees up the window: the
    /// view's own field, never converted again), and as wide on screen as [`SIZE`] frame
    /// pixels.
    #[test]
    fn it_floats_where_the_page_leaves_room_in_every_window() {
        let eye = Vec3::new(10.0, -20.0, 300.0);
        let (yaw, pitch) = (0.7_f32, -0.1_f32);
        let forward = Vec3::new(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        for fov in [90.0, 64.3] {
            for viewport in [
                [1920.0, 1080.0],
                [3840.0, 2160.0],
                [1440.0, 1080.0],
                [2560.0, 1080.0],
            ] {
                let view = Camera {
                    eye,
                    target: eye + forward * 10.0,
                    up: Vec3::Z,
                    fov,
                    viewport,
                };
                let at = crate::console::holocrons_panel::STAGE_AT;
                let (centre, scale) = place(&view, at);
                assert!(((centre - eye).dot(forward) - DISTANCE).abs() < 1e-3);
                // Projected back as the HUD projects the world, it lands on the point.
                let pixel = view.project(centre).expect("in view");
                let frame = crate::menu::sjk::Frame::new(viewport);
                let want = frame.point(at[0], at[1]);
                assert!(
                    (pixel[0] - want[0]).abs() < 0.5 && (pixel[1] - want[1]).abs() < 0.5,
                    "{fov} {viewport:?}: {pixel:?} against {want:?}"
                );
                // Its edge across the window is SIZE frame pixels.
                let half = (fov.to_radians() * 0.5).tan();
                let edge_pixels = scale * EDGE / (2.0 * half * DISTANCE) * viewport[1];
                assert!((edge_pixels - SIZE * frame.s).abs() < 0.5, "{viewport:?}");
                // Left of the middle of the window, where the list is not.
                assert!(pixel[0] < viewport[0] * 0.5, "{viewport:?}");
            }
        }
    }
}
