//! The Collection's Shaders tab shows the player's saber on its own (`docs/unlockables.md`,
//! "The Shaders tab"): the hilts the stage model would hold float before the backdrop
//! camera over the page's [`SABER_AREA`], lying across the window, turned as the page
//! says ([`SaberTurn`]), and the model is not drawn. Like the Holocrons tab's holocron
//! ([`crate::holocrons::stage`]), it is placed from the view the frame renders, so it
//! sits where the page leaves room in every window shape; it stands as far off as makes
//! the longest saber [`LENGTH`] frame pixels long, so a staff and a single saber fill the
//! same room.
//!
//! [`SABER_AREA`]: crate::console::collection_panel::SABER_AREA

use super::showcase::Line;
use crate::console::collection_panel::SaberTurn;
use crate::hud::identification::Camera;
use glam::{Quat, Vec3};

/// How long the longest saber lies across the window, in frame pixels (the turn shortens
/// it as its tip comes round), at most four fifths of the area it lies over.
pub(super) const LENGTH: f32 = 600.0;
/// The nearest it may float, in world units, whatever the field (clear of the near
/// plane).
const NEAREST: f32 = 12.0;

/// Where the sabers float before `view`: the centre of their row and the camera's right
/// and up there, and how far each world unit of saber is from the next on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Place {
    pub(super) centre: Vec3,
    pub(super) right: Vec3,
    pub(super) up: Vec3,
}

impl Place {
    /// The place for sabers whose longest span is `longest` world units, before `view`,
    /// over `area` (frame pixels: the Shaders tab's [`SABER_AREA`], or the Saber Lab's
    /// `LAB_SABER_AREA`).
    pub(super) fn new(view: &Camera, longest: f32, area: [f32; 4]) -> Self {
        let viewport = view.viewport;
        let frame = crate::menu::sjk::Frame::new(viewport);
        let [x, y, width, height] = area;
        let [px, py] = frame.point(x + width * 0.5, y + height * 0.5);
        let (ndc_x, ndc_y) = (px / viewport[0] * 2.0 - 1.0, 1.0 - py / viewport[1] * 2.0);
        let half = (view.fov.to_radians() * 0.5).tan().max(1e-3);
        let aspect = viewport[0] / viewport[1].max(1.0);
        // The basis `Camera::project` reads the view with.
        let forward = (view.target - view.eye).normalize_or(Vec3::X);
        let right = forward.cross(view.up).normalize_or(Vec3::Y);
        let up = right.cross(forward);
        // At distance `d` a window pixel is `2 half d / height` world units: far enough
        // that the longest saber is LENGTH frame pixels across, or four fifths of a
        // narrower area (the Saber Lab's), its turned tip coming nearer within it.
        let pixels = LENGTH.min(width * 0.8) * frame.s;
        let distance = (longest.max(1.0) * viewport[1] / (2.0 * half * pixels)).max(NEAREST);
        let centre = view.eye
            + forward * distance
            + right * (ndc_x * half * aspect * distance)
            + up * (ndc_y * half * distance);
        Self { centre, right, up }
    }

    /// The grip and orientation that lay the saber on `line` across the window, its
    /// length centred `height` world units above the row's centre, turned by `turn`.
    pub(super) fn pose(&self, line: &Line, height: f32, turn: SaberTurn) -> (Vec3, Quat) {
        let rotation = Quat::from_axis_angle(self.up, turn.yaw)
            * Quat::from_axis_angle(self.right, turn.roll)
            * Quat::from_rotation_arc(line.axis, self.right);
        // The middle of its length, hilt-local: on the blade line, halfway along.
        let lateral = line.through - line.axis * line.through.dot(line.axis);
        let middle = lateral + line.axis * line.span.middle();
        let centre = self.centre + self.up * height;
        (centre - rotation * middle, rotation)
    }
}

#[cfg(test)]
mod tests {
    use super::super::showcase::Span;
    use super::*;
    use crate::console::collection_panel::SABER_AREA;

    /// A single saber: hilt from 0 to 10 along +Z, blade 40 from the top.
    fn single() -> Line {
        let socket = Vec3::new(0.5, 0.0, 10.0);
        Line {
            axis: Vec3::Z,
            through: socket,
            span: Span {
                back: 0.0,
                front: 50.0,
            },
        }
    }

    fn camera(viewport: [f32; 2], fov: f32) -> Camera {
        let eye = Vec3::new(30.0, -40.0, 90.0);
        let (yaw, pitch) = (2.1_f32, -0.15_f32);
        let forward = Vec3::new(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        Camera {
            eye,
            target: eye + forward * 10.0,
            up: Vec3::Z,
            fov,
            viewport,
        }
    }

    /// Resting (no turn), the saber lies across the page's area, tip to the right,
    /// LENGTH frame pixels long, centred on the area, in every window shape and field.
    #[test]
    fn it_lies_across_the_area_in_every_window() {
        let line = single();
        for fov in [90.0, 64.3] {
            for viewport in [
                [1920.0, 1080.0],
                [3840.0, 2160.0],
                [1440.0, 1080.0],
                [2560.0, 1080.0],
            ] {
                let view = camera(viewport, fov);
                let place = Place::new(&view, line.span.front - line.span.back, SABER_AREA);
                let (grip, rotation) = place.pose(&line, 0.0, SaberTurn::default());
                // The ends of its blade line: the hilt's end and the blade's tip.
                let end = line.through + line.axis * (line.span.back - line.through.dot(line.axis));
                let base = view.project(grip + rotation * end).expect("in view");
                let tip = view
                    .project(grip + rotation * (line.through + line.axis * 40.0))
                    .expect("in view");
                let frame = crate::menu::sjk::Frame::new(viewport);
                assert!(tip[0] > base[0], "tip to the right: {base:?} {tip:?}");
                assert!((tip[1] - base[1]).abs() < 1.0, "level: {base:?} {tip:?}");
                let long = tip[0] - base[0];
                assert!(
                    (long - LENGTH * frame.s).abs() < 2.0,
                    "{fov} {viewport:?}: {long}"
                );
                let [x, y, width, height] = SABER_AREA;
                let want = frame.point(x + width * 0.5, y + height * 0.5);
                let middle = [(base[0] + tip[0]) * 0.5, (base[1] + tip[1]) * 0.5];
                assert!(
                    (middle[0] - want[0]).abs() < 1.0 && (middle[1] - want[1]).abs() < 1.0,
                    "{viewport:?}: {middle:?} against {want:?}"
                );
                // It stays within the area, whatever the window.
                let left = frame.point(x, y);
                let right = frame.point(x + width, y + height);
                assert!(base[0] >= left[0] && tip[0] <= right[0], "{viewport:?}");
            }
        }
    }

    #[test]
    fn its_turn_keeps_its_middle_in_place() {
        let line = single();
        let view = camera([1920.0, 1080.0], 90.0);
        let place = Place::new(&view, 50.0, SABER_AREA);
        let middle = line.through + line.axis * (line.span.middle() - line.through.dot(line.axis));
        for (yaw, roll) in [(0.0, 0.0), (-0.32, 1.0), (2.5, -4.0)] {
            let (grip, rotation) = place.pose(&line, 0.0, SaberTurn { yaw, roll });
            assert!((grip + rotation * middle - place.centre).length() < 1e-3);
        }
        // Rolled, the blade still lies along the window's horizontal; yawed, its tip
        // comes towards the camera.
        let (_, rolled) = place.pose(
            &line,
            0.0,
            SaberTurn {
                yaw: 0.0,
                roll: 1.3,
            },
        );
        assert!((rolled * line.axis - place.right).length() < 1e-4);
        let (_, yawed) = place.pose(
            &line,
            0.0,
            SaberTurn {
                yaw: -0.5,
                roll: 0.0,
            },
        );
        let forward = (view.target - view.eye).normalize();
        assert!((yawed * line.axis).dot(forward) < -0.1);
    }

    #[test]
    fn a_longer_saber_stands_further_off() {
        let view = camera([1920.0, 1080.0], 90.0);
        let near = Place::new(&view, 50.0, SABER_AREA)
            .centre
            .distance(view.eye);
        let far = Place::new(&view, 90.0, SABER_AREA)
            .centre
            .distance(view.eye);
        assert!(far > near * 1.5);
        // Never through the near plane.
        assert!(Place::new(&view, 0.0, SABER_AREA).centre.distance(view.eye) >= NEAREST);
    }
}
