//! Blender-style transform gizmos for placed models (Sol, 04/11/2026: "add gizmos on the
//! web/ingame to move on an axis the selection or rotate it, scale it, the classic stuff
//! like in Blender"; `docs/map-edits.md`, "Gizmos").
//!
//! The maths here are the in-game editor's; the browser editor's page has the same in
//! `web/page/sjk/gizmo.js`. Three modes: Move (an arrow per axis, a square per plane, the
//! centre in the view's plane), Rotate (a ring per axis and one round the view) and Scale
//! (an arm with a box end per axis of the model, the centre for all three). World axes,
//! or the model's own ("local"). A drag on a handle, or G, R or T and then the mouse
//! (X, Y, Z constrain, twice for local, Shift with them for a plane; a number typed sets
//! the amount; Enter or a click confirms, Esc or a right click cancels), previews the
//! change on the model only; the end runs [`COMMAND`] once, so a drag is one step to undo
//! and one write of the document. Ctrl snaps (8 units, 15 degrees, 0.1 of scale), Shift
//! makes it fine (a tenth).
//!
//! In game the gizmo shows while the panel frees the pointer, on the selected placed model
//! or else the current one, drawn over the scene in screen space (on top, the same size
//! whatever the distance); the panel's keys stay the panel's while a search field types.

use super::document::{Document, Kind};
use crate::GpuState;
use glam::{Mat3, Mat4, Vec2, Vec3};

/// The command that sets placed models' origin, angles and scale exactly: one step to undo
/// whatever the count.
pub(crate) const COMMAND: &str = "mapedit_set";
pub(crate) const HELP: &str = "Map editor: set placed models exactly (id x y z pitch yaw roll [sx sy sz], repeated for several), one step to undo";
/// Most placed models one [`COMMAND`] sets.
pub(crate) const MAX_SET: usize = 256;
/// A placed model's scale on each axis, as documents keep it.
const SCALE_MIN: f32 = 0.01;
const SCALE_MAX: f32 = 64.0;
/// What Ctrl snaps to.
pub(crate) const SNAP: Snap = Snap {
    grid: 8.0,
    angle: 15.0,
    scale: 0.1,
};
/// Shift: a tenth of the move.
const FINE: f32 = 0.1;
/// An arm of the gizmo on screen, a share of the window's height.
const ARM_SHARE: f32 = 0.12;
/// The points a ring is drawn and picked with.
const RING_POINTS: usize = 48;
/// Most draw commands a frame of the gizmo takes.
const LIST_CAPACITY: usize = 512;

/// What the gizmo does.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Mode {
    #[default]
    Move,
    Rotate,
    Scale,
}

/// What Ctrl snaps to: units, degrees and a step of scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Snap {
    pub(crate) grid: f32,
    pub(crate) angle: f32,
    pub(crate) scale: f32,
}

/// A placed model where it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) id: u32,
    pub(crate) origin: Vec3,
    /// Pitch, yaw and roll in degrees, as JA's angles.
    pub(crate) angles: [f32; 3],
    /// Along the model's own forward, left and up.
    pub(crate) scale: [f32; 3],
}

/// One part of the gizmo a press can take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Handle {
    /// Move along axis 0 (X, red), 1 (Y, green) or 2 (Z, blue).
    Axis(usize),
    /// Move in the plane whose normal is that axis.
    Plane(usize),
    /// Move in the view's plane (the centre).
    Free,
    /// Turn about that axis.
    Ring(usize),
    /// Turn about the view's axis (the outer ring).
    ViewRing,
    /// Scale along the model's own axis.
    ScaleAxis(usize),
    /// Scale on all three (the centre).
    Uniform,
}

/// A G, R or T transform's constraint: X, Y or Z once for the world's axis, twice for the
/// model's own, a third time for none; with Shift, the plane without that axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Constraint {
    #[default]
    None,
    Axis {
        axis: usize,
        local: bool,
    },
    Plane {
        normal: usize,
        local: bool,
    },
}

impl Constraint {
    /// The constraint after X, Y or Z (`axis`), with Shift for a plane.
    pub(crate) fn pressed(self, axis: usize, plane: bool) -> Self {
        match (self, plane) {
            (
                Self::Axis {
                    axis: a,
                    local: false,
                },
                false,
            ) if a == axis => Self::Axis { axis, local: true },
            (
                Self::Plane {
                    normal,
                    local: false,
                },
                true,
            ) if normal == axis => Self::Plane {
                normal: axis,
                local: true,
            },
            (
                Self::Axis {
                    axis: a,
                    local: true,
                },
                false,
            ) if a == axis => Self::None,
            (
                Self::Plane {
                    normal,
                    local: true,
                },
                true,
            ) if normal == axis => Self::None,
            (_, false) => Self::Axis { axis, local: false },
            (_, true) => Self::Plane {
                normal: axis,
                local: false,
            },
        }
    }
}

/// JA's AnglesToAxis: the model's forward, left and up for `angles` (pitch, yaw, roll in
/// degrees; pitch is positive looking down).
pub(crate) fn axes_of(angles: [f32; 3]) -> [Vec3; 3] {
    let [pitch, yaw, roll] = angles.map(f32::to_radians);
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();
    let (sr, cr) = roll.sin_cos();
    let forward = Vec3::new(cp * cy, cp * sy, -sp);
    let right = Vec3::new(-sr * sp * cy + cr * sy, -sr * sp * sy - cr * cy, -sr * cp);
    let up = Vec3::new(cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp);
    [forward, -right, up]
}

/// The angles whose axes are `axes` (forward, left, up), each in 0..360: the inverse of
/// [`axes_of`]. Straight up or down, the turn is all yaw and the roll 0.
pub(crate) fn angles_of([forward, left, up]: [Vec3; 3]) -> [f32; 3] {
    let pitch = (-forward.z).clamp(-1.0, 1.0).asin();
    let (yaw, roll) = if pitch.cos() > 1e-4 {
        (forward.y.atan2(forward.x), left.z.atan2(up.z))
    } else {
        ((-left.x).atan2(left.y), 0.0)
    };
    [pitch, yaw, roll].map(|angle| {
        let degrees = angle.to_degrees().rem_euclid(360.0);
        if 360.0 - degrees < 1e-3 { 0.0 } else { degrees }
    })
}

/// The world's axes.
const WORLD: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];

/// What a drag or a G, R or T transform does to the placed models.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Change {
    Move(Vec3),
    /// About `axis` (a unit vector) through the pivot, right-handed.
    Turn {
        axis: Vec3,
        degrees: f32,
    },
    /// Along `along` (each model's own axis nearest it), or on all three.
    Scale {
        along: Option<Vec3>,
        factor: f32,
    },
}

/// `start` changed by `change` about `pivot`: several models move, turn and scale
/// together about their common centre.
pub(crate) fn apply(start: &[Placement], pivot: Vec3, change: Change) -> Vec<Placement> {
    start
        .iter()
        .map(|placement| {
            let mut out = *placement;
            match change {
                Change::Move(offset) => out.origin += offset,
                Change::Turn { axis, degrees } => {
                    let turn =
                        Mat3::from_axis_angle(axis.normalize_or_zero(), degrees.to_radians());
                    out.origin = pivot + turn * (placement.origin - pivot);
                    out.angles = angles_of(axes_of(placement.angles).map(|a| turn * a));
                }
                Change::Scale { along, factor } => {
                    let factor = factor.max(1e-3);
                    let offset = placement.origin - pivot;
                    match along {
                        None => {
                            out.origin = pivot + offset * factor;
                            out.scale = placement.scale.map(|s| clamp_scale(s * factor));
                        }
                        Some(along) => {
                            let along = along.normalize_or_zero();
                            out.origin =
                                placement.origin + along * (offset.dot(along) * (factor - 1.0));
                            // The model's own axis nearest the handle's.
                            let axes = axes_of(placement.angles);
                            let nearest = (0..3)
                                .max_by(|&a, &b| {
                                    axes[a]
                                        .dot(along)
                                        .abs()
                                        .total_cmp(&axes[b].dot(along).abs())
                                })
                                .unwrap_or(0);
                            out.scale[nearest] = clamp_scale(placement.scale[nearest] * factor);
                        }
                    }
                }
            }
            out
        })
        .collect()
}

fn clamp_scale(scale: f32) -> f32 {
    scale.clamp(SCALE_MIN, SCALE_MAX)
}

/// The middle of the models' origins: what they turn and scale about.
pub(crate) fn pivot_of(placements: &[Placement]) -> Vec3 {
    if placements.is_empty() {
        return Vec3::ZERO;
    }
    placements.iter().map(|p| p.origin).sum::<Vec3>() / placements.len() as f32
}

/// A ray from the eye.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ray {
    pub(crate) origin: Vec3,
    pub(crate) dir: Vec3,
}

/// Where along the line through `point` along `axis` (a unit vector) the ray passes
/// nearest; `None` when they run parallel.
pub(crate) fn along_axis(ray: Ray, point: Vec3, axis: Vec3) -> Option<f32> {
    let (u, v, w) = (ray.dir, axis, ray.origin - point);
    let (a, b, c, d, e) = (u.dot(u), u.dot(v), v.dot(v), u.dot(w), v.dot(w));
    let denominator = a * c - b * b;
    (denominator.abs() > 1e-6 * a * c).then(|| (a * e - b * d) / denominator)
}

/// Where the ray meets the plane through `point` with `normal`, ahead of the eye.
pub(crate) fn on_plane(ray: Ray, point: Vec3, normal: Vec3) -> Option<Vec3> {
    let facing = ray.dir.dot(normal);
    if facing.abs() < 1e-6 {
        return None;
    }
    let distance = (point - ray.origin).dot(normal) / facing;
    (distance > 0.0).then(|| ray.origin + ray.dir * distance)
}

/// The angle in degrees from `from` to `to` round `centre` on the screen (y down),
/// counter-clockwise as seen positive, in -180..180.
pub(crate) fn screen_turn(centre: Vec2, from: Vec2, to: Vec2) -> f32 {
    let angle = |p: Vec2| (-(p.y - centre.y)).atan2(p.x - centre.x);
    let mut degrees = (angle(to) - angle(from)).to_degrees();
    if degrees > 180.0 {
        degrees -= 360.0;
    } else if degrees < -180.0 {
        degrees += 360.0;
    }
    degrees
}

/// `value` to the nearest multiple of `step`.
fn snapped(value: f32, step: f32) -> f32 {
    if step > 0.0 {
        (value / step).round() * step
    } else {
        value
    }
}

/// A view of the scene: where it is from, and its screen.
#[derive(Clone, Copy, Debug)]
pub(crate) struct View {
    pub(crate) eye: Vec3,
    pub(crate) forward: Vec3,
    pub(crate) view_projection: Mat4,
    /// Physical pixels.
    pub(crate) viewport: [f32; 2],
}

impl View {
    /// `point` on the screen, if in front of the eye.
    pub(crate) fn project(&self, point: Vec3) -> Option<Vec2> {
        let clip = self.view_projection * point.extend(1.0);
        (clip.w > 1e-3).then(|| {
            Vec2::new(
                (clip.x / clip.w + 1.0) * 0.5 * self.viewport[0],
                (1.0 - clip.y / clip.w) * 0.5 * self.viewport[1],
            )
        })
    }

    /// The ray from the eye through `at` on the screen.
    pub(crate) fn ray(&self, at: Vec2) -> Ray {
        let x = at.x / self.viewport[0].max(1.0) * 2.0 - 1.0;
        let y = 1.0 - at.y / self.viewport[1].max(1.0) * 2.0;
        let far = self
            .view_projection
            .inverse()
            .project_point3(Vec3::new(x, y, 0.5));
        let mut dir = (far - self.eye).normalize_or_zero();
        if dir.dot(self.forward) < 0.0 {
            dir = -dir;
        }
        Ray {
            origin: self.eye,
            dir,
        }
    }

    /// An arm of the gizmo at `point` in world units: the same size on screen whatever
    /// the distance.
    fn arm(&self, point: Vec3) -> Option<f32> {
        let distance = (point - self.eye).length().max(1.0);
        let side = self
            .forward
            .cross(Vec3::Z)
            .try_normalize()
            .unwrap_or(Vec3::Y);
        let step = distance * 0.05;
        let pixels = self
            .project(point)?
            .distance(self.project(point + side * step)?);
        (pixels > 1e-3).then(|| step / pixels * self.viewport[1] * ARM_SHARE)
    }

    /// How near the pointer a handle takes it, in pixels.
    fn reach(&self) -> f32 {
        (self.viewport[1] / 110.0).max(7.0)
    }
}

/// The gizmo laid out for a frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub(crate) pivot: Vec3,
    /// The axes move and rotate along (the world's, or the model's own).
    pub(crate) basis: [Vec3; 3],
    /// The model's own axes, which scale is along.
    pub(crate) own: [Vec3; 3],
    /// An arm in world units.
    pub(crate) arm: f32,
    pub(crate) centre: Vec2,
}

impl Frame {
    pub(crate) fn new(view: &View, placements: &[Placement], local: bool) -> Option<Self> {
        let first = placements.first()?;
        let pivot = pivot_of(placements);
        let own = axes_of(first.angles);
        Some(Self {
            pivot,
            basis: if local { own } else { WORLD },
            own,
            arm: view.arm(pivot)?,
            centre: view.project(pivot)?,
        })
    }

    fn towards_eye(&self, view: &View) -> Vec3 {
        (view.eye - self.pivot).normalize_or_zero()
    }

    /// Whether axis `axis` of `axes` is worth a handle: not pointing at the eye.
    fn axis_shown(&self, view: &View, axis: Vec3) -> bool {
        axis.dot(self.towards_eye(view)).abs() < 0.97
    }

    /// Whether the plane with `normal` is worth a handle: not edge-on.
    fn plane_shown(&self, view: &View, normal: Vec3) -> bool {
        normal.dot(self.towards_eye(view)).abs() > 0.2
    }

    /// The corners of the square of the plane with normal `axis` of [`Self::basis`].
    fn plane_corners(&self, axis: usize) -> [Vec3; 4] {
        let (u, v) = (self.basis[(axis + 1) % 3], self.basis[(axis + 2) % 3]);
        let at = |a: f32, b: f32| self.pivot + (u * a + v * b) * self.arm;
        [at(0.3, 0.3), at(0.55, 0.3), at(0.55, 0.55), at(0.3, 0.55)]
    }

    /// The ring about axis `axis` of [`Self::basis`].
    fn ring(&self, axis: usize) -> impl Iterator<Item = Vec3> + '_ {
        let (u, v) = (self.basis[(axis + 1) % 3], self.basis[(axis + 2) % 3]);
        (0..=RING_POINTS).map(move |i| {
            let angle = i as f32 / RING_POINTS as f32 * std::f32::consts::TAU;
            self.pivot + (u * angle.cos() + v * angle.sin()) * self.arm
        })
    }

    /// The radius of the centre, and of the view's ring, in pixels.
    fn centre_radius(view: &View) -> f32 {
        view.viewport[1] * ARM_SHARE * 0.13
    }
    fn view_ring_radius(view: &View) -> f32 {
        view.viewport[1] * ARM_SHARE * 1.2
    }

    /// The handle under `mouse`, if any: the centre first, then planes, arms and rings.
    pub(crate) fn pick(&self, view: &View, mode: Mode, mouse: Vec2) -> Option<Handle> {
        let reach = view.reach();
        let near_centre = mouse.distance(self.centre) <= Self::centre_radius(view).max(reach);
        let segment = |a: Vec3, b: Vec3| -> f32 {
            match (view.project(a), view.project(b)) {
                (Some(a), Some(b)) => distance_to_segment(mouse, a, b),
                _ => f32::MAX,
            }
        };
        let mut best: Option<(f32, Handle)> = None;
        let mut offer = |distance: f32, handle: Handle| {
            if distance <= reach && best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, handle));
            }
        };
        match mode {
            Mode::Move => {
                if near_centre {
                    return Some(Handle::Free);
                }
                // Of the squares under the pointer, the one facing the eye most.
                let towards = self.towards_eye(view);
                let plane = (0..3)
                    .filter(|&axis| {
                        self.plane_shown(view, self.basis[axis]) && {
                            let corners: Vec<Vec2> = self
                                .plane_corners(axis)
                                .iter()
                                .filter_map(|c| view.project(*c))
                                .collect();
                            corners.len() == 4 && inside_quad(mouse, &corners)
                        }
                    })
                    .max_by(|&a, &b| {
                        let facing = |axis: usize| self.basis[axis].dot(towards).abs();
                        facing(a).total_cmp(&facing(b))
                    });
                if let Some(axis) = plane {
                    return Some(Handle::Plane(axis));
                }
                for axis in 0..3 {
                    let a = self.basis[axis];
                    if self.axis_shown(view, a) {
                        offer(
                            segment(
                                self.pivot + a * self.arm * 0.2,
                                self.pivot + a * self.arm * 1.05,
                            ),
                            Handle::Axis(axis),
                        );
                    }
                }
            }
            Mode::Scale => {
                if near_centre {
                    return Some(Handle::Uniform);
                }
                for axis in 0..3 {
                    let a = self.own[axis];
                    if self.axis_shown(view, a) {
                        offer(
                            segment(
                                self.pivot + a * self.arm * 0.2,
                                self.pivot + a * self.arm * 1.05,
                            ),
                            Handle::ScaleAxis(axis),
                        );
                    }
                }
            }
            Mode::Rotate => {
                for axis in 0..3 {
                    let points: Vec<Option<Vec2>> =
                        self.ring(axis).map(|p| view.project(p)).collect();
                    let distance = points
                        .windows(2)
                        .filter_map(|pair| match pair {
                            [Some(a), Some(b)] => Some(distance_to_segment(mouse, *a, *b)),
                            _ => None,
                        })
                        .fold(f32::MAX, f32::min);
                    offer(distance, Handle::Ring(axis));
                }
                let outer = (mouse.distance(self.centre) - Self::view_ring_radius(view)).abs();
                offer(outer, Handle::ViewRing);
            }
        }
        best.map(|(_, handle)| handle)
    }
}

fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

fn inside_quad(p: Vec2, corners: &[Vec2]) -> bool {
    let cross = |a: Vec2, b: Vec2| (b - a).perp_dot(p - a);
    let signs: Vec<f32> = (0..corners.len())
        .map(|i| cross(corners[i], corners[(i + 1) % corners.len()]))
        .collect();
    signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
}

/// How a transform under way was started.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum How {
    /// A press on a handle, held.
    Handle(Handle),
    /// G, R or T and the mouse (and keys).
    Keys(Mode, Constraint),
}

/// A transform under way: the models as they were, and what the pointer and keys did.
#[derive(Clone, Debug)]
pub(crate) struct Active {
    pub(crate) how: How,
    pub(crate) start: Vec<Placement>,
    pub(crate) pivot: Vec3,
    /// The world's axes, the model's own, and the gizmo's (one of the two).
    own: [Vec3; 3],
    basis: [Vec3; 3],
    /// Where the pointer began, and the turn added up since (a ring may go round).
    begin: Vec2,
    last: Vec2,
    turned: f32,
    /// An arm on screen in pixels, for the centre's scale.
    arm_pixels: f32,
    /// A number typed (G, R, T): the amount, instead of the pointer's.
    pub(crate) typed: String,
}

impl Active {
    pub(crate) fn new(
        how: How,
        start: Vec<Placement>,
        frame: &Frame,
        view: &View,
        at: Vec2,
    ) -> Self {
        Self {
            how,
            pivot: frame.pivot,
            own: frame.own,
            basis: frame.basis,
            start,
            begin: at,
            last: at,
            turned: 0.0,
            arm_pixels: view.viewport[1] * ARM_SHARE,
            typed: String::new(),
        }
    }

    /// What the change is made of, by its handle or keys.
    fn control(&self) -> Control {
        let axis_of = |axis: usize, local: bool| if local { self.own[axis] } else { WORLD[axis] };
        match self.how {
            How::Handle(Handle::Axis(axis)) => Control::MoveAxis(self.basis[axis]),
            How::Handle(Handle::Plane(axis)) => Control::MovePlane(
                self.basis[axis],
                [self.basis[(axis + 1) % 3], self.basis[(axis + 2) % 3]],
            ),
            How::Handle(Handle::Free) | How::Keys(Mode::Move, Constraint::None) => {
                Control::MoveView
            }
            How::Handle(Handle::Ring(axis)) => Control::Turn(Some(self.basis[axis])),
            How::Handle(Handle::ViewRing) | How::Keys(Mode::Rotate, Constraint::None) => {
                Control::Turn(None)
            }
            How::Handle(Handle::ScaleAxis(axis)) => Control::ScaleAxis(self.own[axis], false),
            How::Handle(Handle::Uniform) => Control::ScaleUniform(false),
            How::Keys(Mode::Scale, Constraint::None) => Control::ScaleUniform(true),
            How::Keys(Mode::Move, Constraint::Axis { axis, local }) => {
                Control::MoveAxis(axis_of(axis, local))
            }
            How::Keys(Mode::Move, Constraint::Plane { normal, local }) => Control::MovePlane(
                axis_of(normal, local),
                [
                    axis_of((normal + 1) % 3, local),
                    axis_of((normal + 2) % 3, local),
                ],
            ),
            How::Keys(Mode::Rotate, Constraint::Axis { axis, local })
            | How::Keys(
                Mode::Rotate,
                Constraint::Plane {
                    normal: axis,
                    local,
                },
            ) => Control::Turn(Some(axis_of(axis, local))),
            How::Keys(Mode::Scale, Constraint::Axis { axis, local })
            | How::Keys(
                Mode::Scale,
                Constraint::Plane {
                    normal: axis,
                    local,
                },
            ) => Control::ScaleAxis(axis_of(axis, local), true),
        }
    }

    /// The typed amount, when a number was typed.
    fn typed_amount(&self) -> Option<f32> {
        let text = self.typed.trim();
        let negative = text.starts_with('-');
        let digits = text.trim_start_matches('-');
        if digits.is_empty() {
            return None;
        }
        let value = digits.parse::<f32>().ok().filter(|v| v.is_finite())?;
        Some(if negative { -value } else { value })
    }

    /// The change for the pointer at `mouse`, Ctrl (`snap`) and Shift (`fine`) held.
    pub(crate) fn change(
        &mut self,
        view: &View,
        mouse: Vec2,
        snap: Option<Snap>,
        fine: bool,
    ) -> Change {
        let centre = view.project(self.pivot).unwrap_or(self.begin);
        let ray = |at: Vec2| view.ray(at);
        let typed = self.typed_amount();
        let control = self.control();
        // A ring adds its turn up move by move, so it can go round more than once.
        if matches!(control, Control::Turn(_)) {
            self.turned += screen_turn(centre, self.last, mouse);
            self.last = mouse;
        }
        let fine_by = if fine { FINE } else { 1.0 };
        match control {
            Control::MoveAxis(axis) => {
                let amount = typed.unwrap_or_else(|| {
                    let t = |at| along_axis(ray(at), self.pivot, axis).unwrap_or(0.0);
                    (t(mouse) - t(self.begin)) * fine_by
                });
                let amount = match snap {
                    Some(snap) if typed.is_none() => snapped(amount, snap.grid),
                    _ => amount,
                };
                Change::Move(axis * amount)
            }
            Control::MovePlane(normal, [u, v]) => {
                if let Some(typed) = typed {
                    return Change::Move(u * typed);
                }
                let hit = |at| on_plane(ray(at), self.pivot, normal);
                let offset = match (hit(mouse), hit(self.begin)) {
                    (Some(now), Some(then)) => (now - then) * fine_by,
                    _ => Vec3::ZERO,
                };
                let step = snap.map_or(0.0, |snap| snap.grid);
                Change::Move(u * snapped(offset.dot(u), step) + v * snapped(offset.dot(v), step))
            }
            Control::MoveView => {
                if let Some(typed) = typed {
                    return Change::Move(WORLD[0] * typed);
                }
                let hit = |at| on_plane(ray(at), self.pivot, view.forward);
                let offset = match (hit(mouse), hit(self.begin)) {
                    (Some(now), Some(then)) => (now - then) * fine_by,
                    _ => Vec3::ZERO,
                };
                let step = snap.map_or(0.0, |snap| snap.grid);
                Change::Move(
                    WORLD
                        .iter()
                        .map(|a| *a * snapped(offset.dot(*a), step))
                        .sum(),
                )
            }
            Control::Turn(axis) => {
                let towards = view.eye - self.pivot;
                let (axis, sign) = match axis {
                    // Seen from the eye, the view's axis turns as the pointer does.
                    None => (towards.normalize_or_zero(), 1.0),
                    Some(axis) => (axis, if axis.dot(towards) >= 0.0 { 1.0 } else { -1.0 }),
                };
                let degrees = typed.unwrap_or(self.turned * sign * fine_by);
                let degrees = match snap {
                    Some(snap) if typed.is_none() => snapped(degrees, snap.angle),
                    _ => degrees,
                };
                Change::Turn { axis, degrees }
            }
            Control::ScaleAxis(_, true) | Control::ScaleUniform(true) => {
                let factor = typed.unwrap_or_else(|| {
                    let from = self.begin.distance(centre).max(16.0);
                    1.0 + (mouse.distance(centre) / from - 1.0) * fine_by
                });
                let along = match control {
                    Control::ScaleAxis(along, _) => Some(along),
                    _ => None,
                };
                Change::Scale {
                    along,
                    factor: snap_factor(factor, snap, typed.is_some()),
                }
            }
            Control::ScaleAxis(along, false) => {
                let factor = typed.unwrap_or_else(|| {
                    let t = |at| along_axis(ray(at), self.pivot, along).unwrap_or(0.0);
                    let from = t(self.begin);
                    if from.abs() < 1e-3 {
                        1.0
                    } else {
                        1.0 + (t(mouse) / from - 1.0) * fine_by
                    }
                });
                Change::Scale {
                    along: Some(along),
                    factor: snap_factor(factor, snap, typed.is_some()),
                }
            }
            Control::ScaleUniform(false) => {
                let factor = typed.unwrap_or_else(|| {
                    let moved = (mouse.x - self.begin.x) - (mouse.y - self.begin.y);
                    1.0 + moved / self.arm_pixels * fine_by
                });
                Change::Scale {
                    along: None,
                    factor: snap_factor(factor, snap, typed.is_some()),
                }
            }
        }
    }

    /// The models as `change` leaves them.
    pub(crate) fn placements(&self, change: Change) -> Vec<Placement> {
        apply(&self.start, self.pivot, change)
    }

    /// A key while G, R or T runs: what it does.
    pub(crate) fn key(&mut self, key: ModalKey) -> Option<bool> {
        let How::Keys(mode, constraint) = self.how else {
            return match key {
                ModalKey::Cancel => Some(false),
                _ => None,
            };
        };
        match key {
            ModalKey::Axis(axis, plane) => {
                self.how = How::Keys(mode, constraint.pressed(axis, plane));
            }
            ModalKey::Typed(c) => match c {
                '-' => {
                    if let Some(rest) = self.typed.strip_prefix('-') {
                        self.typed = rest.to_owned();
                    } else {
                        self.typed.insert(0, '-');
                    }
                }
                '0'..='9' | '.' if self.typed.len() < 16 => self.typed.push(c),
                _ => {}
            },
            ModalKey::Backspace => {
                self.typed.pop();
            }
            ModalKey::Confirm => return Some(true),
            ModalKey::Cancel => return Some(false),
        }
        None
    }

    /// What the transform says it does, for the panel's line.
    pub(crate) fn describe(&self, change: Change) -> String {
        let axis_name = |axis: Vec3| {
            let names = ["X", "Y", "Z"];
            let (index, local) = (0..3)
                .flat_map(|i| [(i, false), (i, true)])
                .max_by(|a, b| {
                    let v = |(i, local): (usize, bool)| {
                        let base = if local { self.own[i] } else { WORLD[i] };
                        base.dot(axis).abs() + if local { 0.0 } else { 1e-4 }
                    };
                    v(*a).total_cmp(&v(*b))
                })
                .unwrap_or((0, false));
            format!("{}{}", names[index], if local { " local" } else { "" })
        };
        let typed = if self.typed.is_empty() {
            String::new()
        } else {
            format!(" (typed {})", self.typed)
        };
        match change {
            Change::Move(offset) => format!(
                "Move {:.1} {:.1} {:.1}{typed}",
                offset.x, offset.y, offset.z
            ),
            Change::Turn { axis, degrees } => {
                format!(
                    "Rotate {degrees:.1} degrees about {}{typed}",
                    axis_name(axis)
                )
            }
            Change::Scale { along, factor } => match along {
                Some(axis) => format!("Scale {factor:.2} along {}{typed}", axis_name(axis)),
                None => format!("Scale {factor:.2}{typed}"),
            },
        }
    }
}

fn snap_factor(factor: f32, snap: Option<Snap>, typed: bool) -> f32 {
    match snap {
        Some(snap) if !typed => {
            let stepped = 1.0 + snapped(factor - 1.0, snap.scale);
            if stepped.abs() < 1e-3 {
                snap.scale
            } else {
                stepped
            }
        }
        _ => factor,
    }
}

/// What a change is made of.
#[derive(Clone, Copy, Debug)]
enum Control {
    MoveAxis(Vec3),
    MovePlane(Vec3, [Vec3; 2]),
    MoveView,
    /// About an axis, or the view's.
    Turn(Option<Vec3>),
    /// Along an axis; `true` by the pointer's distance from the pivot (G, R, T).
    ScaleAxis(Vec3, bool),
    ScaleUniform(bool),
}

/// A key of a G, R or T transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ModalKey {
    /// X, Y or Z, with Shift for the plane without it.
    Axis(usize, bool),
    Typed(char),
    Backspace,
    Confirm,
    Cancel,
}

/// One placed model [`COMMAND`] sets: its id and where it goes (its scale kept without
/// one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Set {
    pub(crate) id: u32,
    pub(crate) origin: [f32; 3],
    pub(crate) angles: [f32; 3],
    pub(crate) scale: Option<[f32; 3]>,
}

/// [`COMMAND`]'s arguments: `id x y z pitch yaw roll` for one model, or groups of
/// `id x y z pitch yaw roll sx sy sz`.
pub(crate) fn parse_set(args: &[String]) -> Result<Vec<Set>, String> {
    const USAGE: &str = "usage: mapedit_set <id> <x y z> <pitch yaw roll> [<sx sy sz>], repeated with the scale for several";
    let groups: Vec<&[String]> = match args.len() {
        7 => vec![args],
        n if n >= 10 && n % 10 == 0 => args.chunks(10).collect(),
        _ => return Err(USAGE.into()),
    };
    if groups.len() > MAX_SET {
        return Err(format!("{MAX_SET} placed models at most"));
    }
    groups
        .into_iter()
        .map(|group| {
            let id = group[0]
                .parse::<u32>()
                .map_err(|_| format!("not an id: {}", group[0]))?;
            let numbers = group[1..]
                .iter()
                .map(|text| {
                    text.parse::<f32>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or_else(|| format!("not a number: {text}"))
                })
                .collect::<Result<Vec<f32>, String>>()?;
            let scale = numbers
                .get(6..9)
                .map(|s| [s[0], s[1], s[2]])
                .filter(|s| s.iter().all(|v| *v > 0.0));
            if numbers.len() == 9 && scale.is_none() {
                return Err("a scale must be above 0".into());
            }
            Ok(Set {
                id,
                origin: [numbers[0], numbers[1], numbers[2]],
                angles: [numbers[3], numbers[4], numbers[5]],
                scale,
            })
        })
        .collect()
}

/// Set each of `sets` in `document`, all or none: every id must be a placed model.
pub(crate) fn set_placed(document: &mut Document, sets: &[Set]) -> Result<String, String> {
    for set in sets {
        let placed = document
            .edits
            .iter()
            .any(|edit| edit.id == set.id && matches!(edit.kind, Kind::Place { .. }));
        if !placed {
            return Err(format!("No placed model {}", set.id));
        }
    }
    let mut line = String::new();
    for set in sets {
        if let Some(edit) = document.get_mut(set.id)
            && let Kind::Place {
                origin,
                angles,
                scale,
                ..
            } = &mut edit.kind
        {
            *origin = set.origin;
            *angles = set.angles.map(|a| a.rem_euclid(360.0));
            if let Some(set_scale) = set.scale {
                *scale = set_scale.map(clamp_scale);
            }
            line = edit.describe();
        }
    }
    document.unpublished = true;
    Ok(match sets.len() {
        1 => line,
        n => format!("{n} placed models set"),
    })
}

/// The arguments of [`COMMAND`] for `placements`, with their scale.
pub(crate) fn set_args(placements: &[Placement]) -> Vec<String> {
    placements
        .iter()
        .flat_map(|p| {
            std::iter::once(p.id.to_string()).chain(
                p.origin
                    .to_array()
                    .into_iter()
                    .chain(p.angles)
                    .chain(p.scale)
                    .map(|v| format!("{v}")),
            )
        })
        .collect()
}

/// The in-game gizmo's state, with the map's edits.
pub(crate) struct Gizmo {
    pub(crate) mode: Mode,
    pub(crate) local: bool,
    hover: Option<Handle>,
    active: Option<Active>,
    /// The last change previewed, to put down.
    change: Option<Change>,
    /// The map's entity count, by the entity lump's address and length (the placed
    /// models' pick numbers follow the map's entities).
    lump: Option<(usize, usize, usize)>,
    list: sjk_ui::DrawList,
}

impl Default for Gizmo {
    fn default() -> Self {
        Self {
            mode: Mode::Move,
            local: false,
            hover: None,
            active: None,
            change: None,
            lump: None,
            list: sjk_ui::DrawList::new(LIST_CAPACITY),
        }
    }
}

impl Gizmo {
    /// Whether a drag or a G, R or T transform is under way.
    pub(crate) fn busy(&self) -> bool {
        self.active.is_some()
    }

    /// This frame's gizmo, if one is drawn.
    pub(crate) fn list(&self) -> Option<&sjk_ui::DrawList> {
        (!self.list.is_empty()).then_some(&self.list)
    }
}

impl super::State {
    /// Draw placed edit `id` as `placement`, without saving (a gizmo's preview).
    fn preview(&mut self, placement: &Placement) {
        if let Some(placed) = self.placed.iter_mut().find(|p| p.id == placement.id) {
            placed.origin = placement.origin.to_array();
            placed.angles = placement.angles;
            placed.scale = placement.scale;
            placed.instance = super::instance(placed.origin, placed.angles, placed.scale);
        }
    }
}

/// The colours: X red, Y green, Z blue as Blender's, the view's white, a hovered or held
/// handle gold.
fn colour(axis: Option<usize>, lit: bool) -> sjk_ui::Color {
    if lit {
        return sjk_ui::Color::new(1.0, 0.82, 0.3, 1.0);
    }
    match axis {
        Some(0) => sjk_ui::Color::new(0.95, 0.27, 0.33, 0.95),
        Some(1) => sjk_ui::Color::new(0.55, 0.82, 0.2, 0.95),
        Some(_) => sjk_ui::Color::new(0.25, 0.55, 0.98, 0.95),
        None => sjk_ui::Color::new(0.92, 0.92, 0.92, 0.85),
    }
}

impl GpuState {
    /// The view the gizmo is drawn and picked in: the last rendered main view.
    fn map_edit_gizmo_view(&self) -> Option<View> {
        let (view_projection, eye, forward, viewport) = self.world_notes.camera_for_gizmo()?;
        (viewport[0] > 0.0 && viewport[1] > 0.0).then_some(View {
            eye,
            forward,
            view_projection,
            viewport,
        })
    }

    /// The placed model the gizmo is on: the selected one, else the current one, as drawn.
    fn map_edit_gizmo_targets(&mut self) -> Vec<Placement> {
        let entities = self.bsp.entities();
        let key = (entities.as_ptr() as usize, entities.len());
        let lump = match self.map_edits.gizmo.lump {
            Some((at, len, count)) if (at, len) == key => count,
            _ => {
                let count = sjk_entity::parse_entity_lump(entities).map_or(0, |e| e.len());
                self.map_edits.gizmo.lump = Some((key.0, key.1, count));
                count
            }
        };
        let selected = self.world_notes.waiting_selection().and_then(|selection| {
            match selection.edit_target() {
                crate::world_notes::EditTarget::Model { number, .. } => {
                    self.map_edits.placed_edit(number, lump)
                }
                _ => None,
            }
        });
        let id = selected.or(self.map_edits.editor.current());
        self.map_edits
            .placed
            .iter()
            .filter(|placed| Some(placed.id) == id)
            .map(|placed| Placement {
                id: placed.id,
                origin: Vec3::from_array(placed.origin),
                angles: placed.angles,
                scale: placed.scale,
            })
            .collect()
    }

    fn map_edit_gizmo_modifiers(&self) -> (bool, bool) {
        self.console.as_ref().map_or((false, false), |console| {
            (console.control_held(), console.shift_held())
        })
    }

    /// Preview the transform under way for the pointer at `mouse`.
    fn map_edit_gizmo_follow(&mut self, mouse: Vec2) {
        let Some(view) = self.map_edit_gizmo_view() else {
            return;
        };
        let (control, shift) = self.map_edit_gizmo_modifiers();
        let Some(active) = self.map_edits.gizmo.active.as_mut() else {
            return;
        };
        let change = active.change(&view, mouse, control.then_some(SNAP), shift);
        let placements = active.placements(change);
        let line = active.describe(change);
        self.map_edits.gizmo.change = Some(change);
        for placement in &placements {
            self.map_edits.preview(placement);
        }
        let panel = &mut self.map_edits.panel;
        if panel.status != line {
            panel.status = line;
            panel.status_error = false;
        }
    }

    /// Begin a transform: the selection gives way to the current placed model, which the
    /// gizmo then follows.
    fn map_edit_gizmo_begin(&mut self, how: How, at: Vec2) -> bool {
        let (Some(view), targets) = (self.map_edit_gizmo_view(), self.map_edit_gizmo_targets())
        else {
            return false;
        };
        let gizmo = &self.map_edits.gizmo;
        let Some(frame) = Frame::new(&view, &targets, gizmo.local) else {
            return false;
        };
        if let Some(first) = targets.first() {
            self.map_edits.editor.set_current(first.id);
        }
        self.world_notes.cancel_selection();
        self.map_edits.gizmo.active = Some(Active::new(how, targets, &frame, &view, at));
        self.map_edits.gizmo.change = None;
        self.map_edits.panel.transforming = true;
        self.map_edit_gizmo_follow(at);
        true
    }

    /// End the transform under way: put it down with one [`COMMAND`] (`keep`), or back.
    fn map_edit_gizmo_end(&mut self, keep: bool) {
        let gizmo = &mut self.map_edits.gizmo;
        let (Some(active), change) = (gizmo.active.take(), gizmo.change.take()) else {
            return;
        };
        let result = match change.filter(|_| keep) {
            Some(change) => {
                let placements = active.placements(change);
                self.map_edit_command(COMMAND, &set_args(&placements))
            }
            None => Ok(vec!["Transform cancelled".into()]),
        };
        if result.is_err() || !keep {
            for placement in &active.start {
                self.map_edits.preview(placement);
            }
        }
        let panel = &mut self.map_edits.panel;
        panel.transforming = false;
        panel.answer(result);
        panel.stale();
    }

    /// A pointer event on the scene while the panel is open: the gizmo's when it takes it.
    pub(crate) fn map_edit_gizmo_pointer(&mut self, event: sjk_ui::InputEvent) -> bool {
        use sjk_ui::{InputEvent, PointerButton};
        let scene_right = self.map_edits.panel.scene_right;
        match event {
            InputEvent::PointerMove(at) => {
                let at = Vec2::new(at.x, at.y);
                if self.map_edits.gizmo.busy() {
                    self.map_edit_gizmo_follow(at);
                    return true;
                }
                let hover = (at.x < scene_right)
                    .then(|| self.map_edit_gizmo_pick(at))
                    .flatten();
                self.map_edits.gizmo.hover = hover;
                false
            }
            InputEvent::PointerPress { position, button } => {
                if self.map_edits.gizmo.busy() {
                    // A G, R or T transform: a click puts it down, a right click takes it back.
                    let keys = matches!(
                        self.map_edits.gizmo.active.as_ref().map(|a| a.how),
                        Some(How::Keys(..))
                    );
                    match button {
                        PointerButton::Primary if keys => self.map_edit_gizmo_end(true),
                        PointerButton::Secondary => self.map_edit_gizmo_end(false),
                        _ => {}
                    }
                    return true;
                }
                if button != PointerButton::Primary || position.x >= scene_right {
                    return false;
                }
                let position = Vec2::new(position.x, position.y);
                match self.map_edit_gizmo_pick(position) {
                    Some(handle) => self.map_edit_gizmo_begin(How::Handle(handle), position),
                    None => false,
                }
            }
            InputEvent::PointerRelease {
                button: PointerButton::Primary,
                ..
            } => {
                let held = matches!(
                    self.map_edits.gizmo.active.as_ref().map(|a| a.how),
                    Some(How::Handle(_))
                );
                if held {
                    self.map_edit_gizmo_end(true);
                }
                held
            }
            _ => self.map_edits.gizmo.busy(),
        }
    }

    fn map_edit_gizmo_pick(&mut self, at: Vec2) -> Option<Handle> {
        let view = self.map_edit_gizmo_view()?;
        let targets = self.map_edit_gizmo_targets();
        let frame = Frame::new(&view, &targets, self.map_edits.gizmo.local)?;
        frame.pick(&view, self.map_edits.gizmo.mode, at)
    }

    /// A key while the panel is open: the gizmo's (1, 2, 3, L, G, R, T, and every key
    /// while a transform runs) unless a search field types.
    pub(crate) fn map_edit_gizmo_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        use winit::event::ElementState;
        use winit::keyboard::{KeyCode, PhysicalKey};
        let PhysicalKey::Code(key) = event.physical_key else {
            return self.map_edits.gizmo.busy();
        };
        let (_, shift) = self.map_edit_gizmo_modifiers();
        if self.map_edits.gizmo.busy() {
            if event.state != ElementState::Pressed {
                return true;
            }
            let modal = match key {
                KeyCode::KeyX => Some(ModalKey::Axis(0, shift)),
                KeyCode::KeyY => Some(ModalKey::Axis(1, shift)),
                KeyCode::KeyZ => Some(ModalKey::Axis(2, shift)),
                KeyCode::Backspace => Some(ModalKey::Backspace),
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => Some(ModalKey::Confirm),
                KeyCode::Escape => Some(ModalKey::Cancel),
                KeyCode::Minus | KeyCode::NumpadSubtract => Some(ModalKey::Typed('-')),
                KeyCode::Period | KeyCode::NumpadDecimal | KeyCode::Comma => {
                    Some(ModalKey::Typed('.'))
                }
                _ => digit_of(key).map(ModalKey::Typed),
            };
            let Some(modal) = modal.filter(|_| !event.repeat || modal == Some(ModalKey::Backspace))
            else {
                return true;
            };
            let outcome = self
                .map_edits
                .gizmo
                .active
                .as_mut()
                .and_then(|active| active.key(modal));
            match outcome {
                Some(keep) => self.map_edit_gizmo_end(keep),
                None => {
                    let at = Vec2::from_array(self.cursor_position);
                    self.map_edit_gizmo_follow(at);
                }
            }
            return true;
        }
        if event.state != ElementState::Pressed || event.repeat {
            return false;
        }
        let typing = matches!(
            self.map_edits.panel.focus,
            super::panel::SEARCH | super::panel::REMAP_FIELD
        );
        let (control, _) = self.map_edit_gizmo_modifiers();
        if typing || control {
            return false;
        }
        let gizmo = &mut self.map_edits.gizmo;
        let (mode, says) = match key {
            KeyCode::Digit1 => (Some(Mode::Move), None),
            KeyCode::Digit2 => (Some(Mode::Rotate), None),
            KeyCode::Digit3 => (Some(Mode::Scale), None),
            KeyCode::KeyL => {
                gizmo.local = !gizmo.local;
                (
                    None,
                    Some(if gizmo.local {
                        "Gizmo: the model's own axes"
                    } else {
                        "Gizmo: the world's axes"
                    }),
                )
            }
            KeyCode::KeyG | KeyCode::KeyR | KeyCode::KeyT => {
                let mode = match key {
                    KeyCode::KeyG => Mode::Move,
                    KeyCode::KeyR => Mode::Rotate,
                    _ => Mode::Scale,
                };
                let at = Vec2::from_array(self.cursor_position);
                if !self.map_edit_gizmo_begin(How::Keys(mode, Constraint::None), at) {
                    self.map_edits
                        .panel
                        .answer(Err("Select a placed model first".into()));
                }
                return true;
            }
            _ => return false,
        };
        if let Some(mode) = mode {
            gizmo.mode = mode;
        }
        let says = says.map_or_else(
            || {
                format!(
                    "Gizmo: {}",
                    match gizmo.mode {
                        Mode::Move => "move",
                        Mode::Rotate => "rotate",
                        Mode::Scale => "scale",
                    }
                )
            },
            str::to_owned,
        );
        self.map_edits.panel.answer(Ok(vec![says]));
        true
    }

    /// Build this frame's gizmo while the panel is open, on screen over the scene.
    pub(crate) fn map_edit_gizmo_draw(&mut self) {
        self.map_edits.gizmo.list.clear();
        if !self.map_edit_panel_open() {
            if self.map_edits.gizmo.busy() {
                self.map_edit_gizmo_end(false);
            }
            return;
        }
        let Some(view) = self.map_edit_gizmo_view() else {
            return;
        };
        let targets = match &self.map_edits.gizmo.active {
            Some(active) => {
                let change = self.map_edits.gizmo.change;
                change.map_or_else(|| active.start.clone(), |c| active.placements(c))
            }
            None => self.map_edit_gizmo_targets(),
        };
        let gizmo = &mut self.map_edits.gizmo;
        let Some(frame) = Frame::new(&view, &targets, gizmo.local) else {
            return;
        };
        let held = match gizmo.active.as_ref().map(|a| a.how) {
            Some(How::Handle(handle)) => Some(handle),
            Some(How::Keys(..)) => {
                // G, R, T: a line from the pivot to the pointer, as Blender's.
                let to = Vec2::from_array(self.cursor_position);
                let _ = gizmo.list.push(sjk_ui::DrawCommand::Line {
                    from: frame.centre.to_array(),
                    to: to.to_array(),
                    width: 1.5,
                    color: sjk_ui::Color::new(1.0, 1.0, 1.0, 0.6),
                });
                draw_dot(&mut gizmo.list, frame.centre, 4.0, colour(None, true));
                return;
            }
            None => gizmo.hover,
        };
        draw(&mut gizmo.list, &view, &frame, gizmo.mode, held);
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shot: where the X arrow's middle shows, and a point `units` further
    /// along it, on the screen.
    pub(crate) fn shot_gizmo_arrow(&mut self, units: f32) -> Option<[sjk_ui::Vec2; 2]> {
        let view = self.map_edit_gizmo_view()?;
        let targets = self.map_edit_gizmo_targets();
        let frame = Frame::new(&view, &targets, false)?;
        let on = frame.pivot + Vec3::X * frame.arm * 0.6;
        let [a, b] = [view.project(on)?, view.project(on + Vec3::X * units)?];
        Some([sjk_ui::Vec2::new(a.x, a.y), sjk_ui::Vec2::new(b.x, b.y)])
    }

    /// For the world shot: where placed edit `id` is drawn now.
    pub(crate) fn shot_placed_origin(&self, id: u32) -> Option<Vec3> {
        self.map_edits
            .placed
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| Vec3::from_array(placed.origin))
    }
}

fn digit_of(key: winit::keyboard::KeyCode) -> Option<char> {
    use winit::keyboard::KeyCode;
    let digits = [
        (KeyCode::Digit0, KeyCode::Numpad0),
        (KeyCode::Digit1, KeyCode::Numpad1),
        (KeyCode::Digit2, KeyCode::Numpad2),
        (KeyCode::Digit3, KeyCode::Numpad3),
        (KeyCode::Digit4, KeyCode::Numpad4),
        (KeyCode::Digit5, KeyCode::Numpad5),
        (KeyCode::Digit6, KeyCode::Numpad6),
        (KeyCode::Digit7, KeyCode::Numpad7),
        (KeyCode::Digit8, KeyCode::Numpad8),
        (KeyCode::Digit9, KeyCode::Numpad9),
    ];
    digits
        .iter()
        .position(|(a, b)| *a == key || *b == key)
        .and_then(|digit| char::from_digit(digit as u32, 10))
}

fn draw_dot(list: &mut sjk_ui::DrawList, at: Vec2, radius: f32, color: sjk_ui::Color) {
    let _ = list.push(sjk_ui::DrawCommand::Arc {
        center: at.to_array(),
        radius: radius * 0.5,
        width: radius,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color,
        knockout: None,
    });
}

fn polygon(list: &mut sjk_ui::DrawList, points: &[Vec2], color: sjk_ui::Color) {
    let mut all = [[0.0; 2]; sjk_ui::POLYGON_POINTS];
    let count = points.len().min(sjk_ui::POLYGON_POINTS);
    for (slot, point) in all.iter_mut().zip(points) {
        *slot = point.to_array();
    }
    let _ = list.push(sjk_ui::DrawCommand::Polygon {
        points: all,
        count: count as u8,
        color,
        texture: None,
    });
}

/// The gizmo of `mode` on `frame`, `lit` the handle hovered or held.
fn draw(list: &mut sjk_ui::DrawList, view: &View, frame: &Frame, mode: Mode, lit: Option<Handle>) {
    let width = (view.viewport[1] / 360.0).clamp(2.0, 4.0);
    let line = |list: &mut sjk_ui::DrawList, a: Vec3, b: Vec3, color| {
        if let (Some(a), Some(b)) = (view.project(a), view.project(b)) {
            let _ = list.push(sjk_ui::DrawCommand::Line {
                from: a.to_array(),
                to: b.to_array(),
                width,
                color,
            });
        }
    };
    let arm = frame.arm;
    match mode {
        Mode::Move | Mode::Scale => {
            let axes = if mode == Mode::Move {
                frame.basis
            } else {
                frame.own
            };
            for (axis, a) in axes.iter().enumerate() {
                if !frame.axis_shown(view, *a) {
                    continue;
                }
                let handle = if mode == Mode::Move {
                    Handle::Axis(axis)
                } else {
                    Handle::ScaleAxis(axis)
                };
                let color = colour(Some(axis), lit == Some(handle));
                let (start, end) = (frame.pivot + *a * arm * 0.2, frame.pivot + *a * arm * 0.82);
                line(list, start, end, color);
                let (Some(base), Some(tip)) =
                    (view.project(end), view.project(frame.pivot + *a * arm))
                else {
                    continue;
                };
                let across = (tip - base).perp().normalize_or_zero() * width * 2.5;
                if mode == Mode::Move {
                    polygon(list, &[tip, base + across, base - across], color);
                } else {
                    let half = width * 2.5;
                    let square = [
                        tip + Vec2::new(-half, -half),
                        tip + Vec2::new(half, -half),
                        tip + Vec2::new(half, half),
                        tip + Vec2::new(-half, half),
                    ];
                    line(list, end, frame.pivot + *a * arm, color);
                    polygon(list, &square, color);
                }
            }
            if mode == Mode::Move {
                for axis in 0..3 {
                    if !frame.plane_shown(view, frame.basis[axis]) {
                        continue;
                    }
                    let corners: Vec<Vec2> = frame
                        .plane_corners(axis)
                        .iter()
                        .filter_map(|c| view.project(*c))
                        .collect();
                    if corners.len() == 4 {
                        let mut color = colour(Some(axis), lit == Some(Handle::Plane(axis)));
                        color.a = 0.55;
                        polygon(list, &corners, color);
                    }
                }
            }
            let centre_handle = if mode == Mode::Move {
                Handle::Free
            } else {
                Handle::Uniform
            };
            let _ = list.push(sjk_ui::DrawCommand::Arc {
                center: frame.centre.to_array(),
                radius: Frame::centre_radius(view),
                width,
                start: 0.0,
                sweep: std::f32::consts::TAU,
                color: colour(None, lit == Some(centre_handle)),
                knockout: None,
            });
        }
        Mode::Rotate => {
            for axis in 0..3 {
                let color = colour(Some(axis), lit == Some(Handle::Ring(axis)));
                let points: Vec<Vec3> = frame.ring(axis).collect();
                for pair in points.windows(2) {
                    line(list, pair[0], pair[1], color);
                }
            }
            let _ = list.push(sjk_ui::DrawCommand::Arc {
                center: frame.centre.to_array(),
                radius: Frame::view_ring_radius(view),
                width,
                start: 0.0,
                sweep: std::f32::consts::TAU,
                color: colour(None, lit == Some(Handle::ViewRing)),
                knockout: None,
            });
        }
    }
    draw_dot(list, frame.centre, width * 2.0, colour(None, false));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 0.02
    }

    /// A view from `eye` at `target`, 1600 by 900.
    fn view(eye: Vec3, target: Vec3) -> View {
        let viewport = [1600.0, 900.0];
        let projection = glam::camera::rh::proj::directx::perspective(
            80f32.to_radians(),
            16.0 / 9.0,
            4.0,
            8192.0,
        );
        let look = glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Z);
        View {
            eye,
            forward: (target - eye).normalize(),
            view_projection: projection * look,
            viewport,
        }
    }

    fn placed(id: u32, origin: Vec3, yaw: f32) -> Placement {
        Placement {
            id,
            origin,
            angles: [0.0, yaw, 0.0],
            scale: [1.0; 3],
        }
    }

    #[test]
    fn the_axes_are_the_games_and_angles_come_back_from_them() {
        for angles in [
            [0.0, 0.0, 0.0],
            [0.0, 90.0, 0.0],
            [30.0, 45.0, 0.0],
            [10.0, 200.0, 33.0],
            [300.0, 10.0, 120.0],
        ] {
            let axes = axes_of(angles);
            let rotation = glam::Quat::from_array(sjk_client::legacy_angles_to_quaternion(angles));
            for (axis, world) in axes.iter().zip(WORLD) {
                assert!(
                    close(*axis, rotation * world),
                    "{angles:?}: {axis} against {}",
                    rotation * world
                );
            }
            let back = angles_of(axes);
            assert!(
                axes_of(back).iter().zip(axes).all(|(a, b)| close(*a, b)),
                "{angles:?} came back as {back:?}"
            );
        }
        // Straight down: the turn is all yaw.
        let down = axes_of([90.0, 30.0, 40.0]);
        let back = angles_of(down);
        assert_eq!(back[2], 0.0);
        assert!(axes_of(back).iter().zip(down).all(|(a, b)| close(*a, b)));
    }

    #[test]
    fn rays_meet_axes_and_planes() {
        let ray = Ray {
            origin: Vec3::new(5.0, -100.0, 50.0),
            dir: Vec3::Y,
        };
        let t = along_axis(ray, Vec3::ZERO, Vec3::X).unwrap();
        assert!((t - 5.0).abs() < 1e-4);
        assert!(along_axis(ray, Vec3::ZERO, Vec3::Y).is_none(), "parallel");
        let down = Ray {
            origin: Vec3::new(3.0, 4.0, 100.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert!(close(
            on_plane(down, Vec3::ZERO, Vec3::Z).unwrap(),
            Vec3::new(3.0, 4.0, 0.0)
        ));
        let up = Ray {
            dir: Vec3::Z,
            ..down
        };
        assert!(
            on_plane(up, Vec3::ZERO, Vec3::Z).is_none(),
            "behind the eye"
        );
        // A quarter turn counter-clockwise on the screen (y down).
        let centre = Vec2::new(100.0, 100.0);
        let turn = screen_turn(centre, Vec2::new(150.0, 100.0), Vec2::new(100.0, 50.0));
        assert!((turn - 90.0).abs() < 1e-3);
    }

    #[test]
    fn an_arm_drag_moves_along_its_axis_snapped_and_fine() {
        let pivot = Vec3::new(100.0, 0.0, 0.0);
        let view = view(Vec3::new(100.0, -400.0, 200.0), pivot);
        let start = vec![placed(1, pivot, 0.0)];
        let frame = Frame::new(&view, &start, false).unwrap();
        // The arrow's middle takes the press.
        let on_arm = view.project(pivot + Vec3::X * frame.arm * 0.6).unwrap();
        assert_eq!(frame.pick(&view, Mode::Move, on_arm), Some(Handle::Axis(0)));
        assert_eq!(
            frame.pick(&view, Mode::Move, frame.centre),
            Some(Handle::Free)
        );
        assert_eq!(
            frame.pick(&view, Mode::Scale, on_arm),
            Some(Handle::ScaleAxis(0))
        );
        let mut active = Active::new(How::Handle(Handle::Axis(0)), start, &frame, &view, on_arm);
        let to = view
            .project(pivot + Vec3::X * (frame.arm * 0.6 + 30.0))
            .unwrap();
        let Change::Move(moved) = active.change(&view, to, None, false) else {
            panic!("not a move")
        };
        assert!(close(moved, Vec3::X * 30.0), "{moved}");
        assert_eq!(
            active.change(&view, to, Some(SNAP), false),
            Change::Move(Vec3::X * 32.0)
        );
        let Change::Move(fine) = active.change(&view, to, None, true) else {
            panic!("not a move")
        };
        assert!(close(fine, Vec3::X * 3.0), "{fine}");
        let moved = active.placements(Change::Move(Vec3::X * 32.0));
        assert!(close(moved[0].origin, Vec3::new(132.0, 0.0, 0.0)));
    }

    #[test]
    fn a_plane_drag_stays_on_its_plane() {
        let pivot = Vec3::ZERO;
        let view = view(Vec3::new(-300.0, -300.0, 400.0), pivot);
        let start = vec![placed(1, pivot, 0.0)];
        let frame = Frame::new(&view, &start, false).unwrap();
        let square = frame.plane_corners(2);
        let middle = view.project((square[0] + square[2]) * 0.5).unwrap();
        assert_eq!(
            frame.pick(&view, Mode::Move, middle),
            Some(Handle::Plane(2))
        );
        let mut active = Active::new(How::Handle(Handle::Plane(2)), start, &frame, &view, middle);
        let there = view
            .project((square[0] + square[2]) * 0.5 + Vec3::new(21.0, -13.0, 0.0))
            .unwrap();
        let Change::Move(moved) = active.change(&view, there, None, false) else {
            panic!("not a move")
        };
        assert!(close(moved, Vec3::new(21.0, -13.0, 0.0)), "{moved}");
        assert_eq!(
            active.change(&view, there, Some(SNAP), false),
            Change::Move(Vec3::new(24.0, -16.0, 0.0))
        );
    }

    #[test]
    fn a_ring_turns_by_how_the_pointer_goes_round_and_snaps() {
        let pivot = Vec3::ZERO;
        // From above: the Z ring faces the eye, counter-clockwise on screen is positive.
        let view = view(Vec3::new(0.0, -1.0, 500.0), pivot);
        let start = vec![placed(1, pivot, 0.0)];
        let frame = Frame::new(&view, &start, false).unwrap();
        let radius = 80.0;
        let at = |degrees: f32| {
            let r = degrees.to_radians();
            frame.centre + Vec2::new(r.cos(), -r.sin()) * radius
        };
        let mut active = Active::new(
            How::Handle(Handle::Ring(2)),
            start.clone(),
            &frame,
            &view,
            at(0.0),
        );
        for step in 1..=9 {
            let _ = active.change(&view, at(step as f32 * 10.0), None, false);
        }
        let Change::Turn { axis, degrees } = active.change(&view, at(100.0), None, false) else {
            panic!("not a turn")
        };
        assert!(close(axis, Vec3::Z));
        assert!((degrees - 100.0).abs() < 0.5, "{degrees}");
        let Change::Turn { degrees, .. } = active.change(&view, at(100.0), Some(SNAP), false)
        else {
            panic!("not a turn")
        };
        assert_eq!(degrees, 105.0);
        let turned = active.placements(Change::Turn {
            axis: Vec3::Z,
            degrees: 90.0,
        });
        assert!((turned[0].angles[1] - 90.0).abs() < 1e-3);
        // From below, the same circle on screen turns the other way.
        let under = view_from_below();
        let frame = Frame::new(&under, &start, false).unwrap();
        let at = |degrees: f32| {
            let r = degrees.to_radians();
            frame.centre + Vec2::new(r.cos(), -r.sin()) * radius
        };
        let mut active = Active::new(How::Handle(Handle::Ring(2)), start, &frame, &under, at(0.0));
        let Change::Turn { degrees, .. } = active.change(&under, at(45.0), None, false) else {
            panic!("not a turn")
        };
        assert!((degrees + 45.0).abs() < 0.5, "{degrees}");
    }

    fn view_from_below() -> View {
        view(Vec3::new(0.0, -1.0, -500.0), Vec3::ZERO)
    }

    #[test]
    fn local_axes_follow_the_models_turn() {
        let pivot = Vec3::ZERO;
        let view = view(Vec3::new(-200.0, -300.0, 250.0), pivot);
        let start = vec![placed(1, pivot, 90.0)];
        let world = Frame::new(&view, &start, false).unwrap();
        let local = Frame::new(&view, &start, true).unwrap();
        assert!(close(world.basis[0], Vec3::X));
        assert!(
            close(local.basis[0], Vec3::Y),
            "the model's forward is the world's Y"
        );
        // G, X, X: along the model's own X.
        let mut active = Active::new(
            How::Keys(Mode::Move, Constraint::None),
            start,
            &local,
            &view,
            local.centre,
        );
        assert_eq!(active.key(ModalKey::Axis(0, false)), None);
        assert_eq!(
            active.how,
            How::Keys(
                Mode::Move,
                Constraint::Axis {
                    axis: 0,
                    local: false
                }
            )
        );
        assert_eq!(active.key(ModalKey::Axis(0, false)), None);
        assert_eq!(
            active.how,
            How::Keys(
                Mode::Move,
                Constraint::Axis {
                    axis: 0,
                    local: true
                }
            )
        );
        for c in ['1', '6'] {
            active.key(ModalKey::Typed(c));
        }
        let change = active.change(&view, local.centre, None, false);
        let Change::Move(moved) = change else {
            panic!("not a move")
        };
        assert!(close(moved, Vec3::Y * 16.0), "{moved}");
        active.key(ModalKey::Typed('-'));
        assert_eq!(active.typed, "-16");
        let Change::Move(moved) = active.change(&view, local.centre, None, false) else {
            panic!("not a move")
        };
        assert!(close(moved, Vec3::Y * -16.0));
        // A third X: no constraint.
        active.key(ModalKey::Axis(0, false));
        assert_eq!(active.how, How::Keys(Mode::Move, Constraint::None));
        // Shift+Z: the XY plane.
        active.key(ModalKey::Axis(2, true));
        assert_eq!(
            active.how,
            How::Keys(
                Mode::Move,
                Constraint::Plane {
                    normal: 2,
                    local: false
                }
            )
        );
        assert_eq!(active.key(ModalKey::Confirm), Some(true));
        assert_eq!(active.key(ModalKey::Cancel), Some(false));
    }

    #[test]
    fn several_models_turn_and_scale_about_their_middle() {
        let start = vec![
            placed(1, Vec3::ZERO, 0.0),
            placed(2, Vec3::new(100.0, 0.0, 0.0), 30.0),
        ];
        let pivot = pivot_of(&start);
        assert!(close(pivot, Vec3::new(50.0, 0.0, 0.0)));
        let turned = apply(
            &start,
            pivot,
            Change::Turn {
                axis: Vec3::Z,
                degrees: 90.0,
            },
        );
        assert!(close(turned[0].origin, Vec3::new(50.0, -50.0, 0.0)));
        assert!(close(turned[1].origin, Vec3::new(50.0, 50.0, 0.0)));
        assert!((turned[0].angles[1] - 90.0).abs() < 1e-3);
        assert!((turned[1].angles[1] - 120.0).abs() < 1e-3);
        let bigger = apply(
            &start,
            pivot,
            Change::Scale {
                along: None,
                factor: 2.0,
            },
        );
        assert!(close(bigger[0].origin, Vec3::new(-50.0, 0.0, 0.0)));
        assert!(close(bigger[1].origin, Vec3::new(150.0, 0.0, 0.0)));
        assert_eq!(bigger[1].scale, [2.0; 3]);
        // Along the world's Y: the first model's left, the second's nearest own axis
        // (its left too, at 30 degrees), the origins on the X line stay.
        let wider = apply(
            &start,
            pivot,
            Change::Scale {
                along: Some(Vec3::Y),
                factor: 3.0,
            },
        );
        assert_eq!(wider[0].scale, [1.0, 3.0, 1.0]);
        assert_eq!(wider[1].scale, [1.0, 3.0, 1.0]);
        assert!(close(wider[1].origin, start[1].origin));
        // Never past what documents keep.
        let huge = apply(
            &start,
            pivot,
            Change::Scale {
                along: None,
                factor: 1000.0,
            },
        );
        assert_eq!(huge[0].scale, [64.0; 3]);
    }

    #[test]
    fn g_r_t_scale_by_distance_and_typed_numbers() {
        let pivot = Vec3::ZERO;
        let view = view(Vec3::new(0.0, -400.0, 100.0), pivot);
        let start = vec![placed(1, pivot, 0.0)];
        let frame = Frame::new(&view, &start, false).unwrap();
        let from = frame.centre + Vec2::new(50.0, 0.0);
        let mut active = Active::new(
            How::Keys(Mode::Scale, Constraint::None),
            start.clone(),
            &frame,
            &view,
            from,
        );
        let Change::Scale { along, factor } =
            active.change(&view, frame.centre + Vec2::new(100.0, 0.0), None, false)
        else {
            panic!("not a scale")
        };
        assert_eq!(along, None);
        assert!((factor - 2.0).abs() < 1e-3);
        let Change::Scale { factor, .. } = active.change(
            &view,
            frame.centre + Vec2::new(83.0, 0.0),
            Some(SNAP),
            false,
        ) else {
            panic!("not a scale")
        };
        assert!((factor - 1.7).abs() < 1e-4, "{factor}");
        active.key(ModalKey::Axis(2, false));
        for c in ['1', '.', '5'] {
            active.key(ModalKey::Typed(c));
        }
        assert_eq!(
            active.change(&view, from, None, false),
            Change::Scale {
                along: Some(Vec3::Z),
                factor: 1.5
            }
        );
        active.key(ModalKey::Backspace);
        active.key(ModalKey::Backspace);
        assert_eq!(active.typed, "1");
        // R then a number: degrees about the view's axis.
        let mut turn = Active::new(
            How::Keys(Mode::Rotate, Constraint::None),
            start,
            &frame,
            &view,
            from,
        );
        for c in ['4', '5'] {
            turn.key(ModalKey::Typed(c));
        }
        let Change::Turn { axis, degrees } = turn.change(&view, from, None, false) else {
            panic!("not a turn")
        };
        assert_eq!(degrees, 45.0);
        assert!(close(axis, (view.eye - pivot).normalize()));
    }

    #[test]
    fn the_set_command_reads_groups_and_sets_all_or_none() {
        let args = |text: &str| {
            text.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let one = parse_set(&args("3 1 2 3 0 90 0")).unwrap();
        assert_eq!(
            one,
            vec![Set {
                id: 3,
                origin: [1.0, 2.0, 3.0],
                angles: [0.0, 90.0, 0.0],
                scale: None
            }]
        );
        let two = parse_set(&args("3 1 2 3 0 90 0 1 1 2 4 0 0 0 10 0 0 1.5 1.5 1.5")).unwrap();
        assert_eq!(two.len(), 2);
        assert_eq!(two[0].scale, Some([1.0, 1.0, 2.0]));
        assert!(parse_set(&args("3 1 2 3")).is_err());
        assert!(
            parse_set(&args("3 1 2 3 0 90 0 1 0 1")).is_err(),
            "a scale of 0"
        );
        assert!(parse_set(&args("x 1 2 3 0 90 0")).is_err());
        assert!(parse_set(&args("3 1 2 nan 0 90 0")).is_err());
        let placements = [placed(4, Vec3::new(1.5, -2.0, 8.0), 45.0)];
        let back = parse_set(&set_args(&placements)).unwrap();
        assert_eq!(back[0].origin, [1.5, -2.0, 8.0]);
        assert_eq!(back[0].scale, Some([1.0; 3]));
    }
}
