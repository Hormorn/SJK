//! Placing on a surface (Sol, 10/11/2026: "continue, do the good stuff"): with
//! `mapedit_surface on` (Place's Surface switch) a model placed or grabbed stands on the
//! surface under the pointer with its up axis along the surface's normal, as a decal or a
//! prop dropped in an editor's "align to surface" mode: on a slope it leans with it, on a
//! wall it sticks out of it, on a ceiling it hangs. Its origin is the point met, so it
//! sits on the surface. Its heading is the view's (placing) or its own (grabbing), laid
//! along the surface.

use super::gizmo::{angles_of, axes_of};
use glam::Vec3;

/// JA angles (pitch, yaw, roll in degrees) that stand a model on a surface of `normal`:
/// its up axis along the normal, its forward the heading `yaw` (degrees about the world's
/// Z) laid along the surface. A normal straight along the heading takes the world's X or
/// Y instead; a zero normal stands it upright.
pub(crate) fn surface_angles(normal: Vec3, yaw: f32) -> [f32; 3] {
    let up = normal.normalize_or(Vec3::Z);
    let (sin, cos) = yaw.to_radians().sin_cos();
    let heading = Vec3::new(cos, sin, 0.0);
    let along = |direction: Vec3| (direction - up * direction.dot(up)).try_normalize();
    let forward = along(heading)
        .or_else(|| along(Vec3::X))
        .or_else(|| along(Vec3::Y))
        .unwrap_or(Vec3::X);
    let left = up.cross(forward);
    angles_of([forward, left, up])
}

/// The heading (degrees about the world's Z) of a model turned by `angles`: where its
/// forward points on the floor, its up's when it faces straight up or down.
pub(crate) fn heading_of(angles: [f32; 3]) -> f32 {
    let [forward, _, up] = axes_of(angles);
    let flat = if forward.truncate().length() > 1e-3 {
        forward
    } else {
        -up
    };
    flat.y.atan2(flat.x).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 1e-4
    }

    /// On the floor a model stands as placed before; on a slope, a wall or a ceiling its
    /// up follows the normal and its forward keeps the heading laid along the surface.
    #[test]
    fn a_model_stands_along_the_surfaces_normal() {
        assert_eq!(surface_angles(Vec3::Z, 90.0), [0.0, 90.0, 0.0]);
        for (normal, yaw) in [
            (Vec3::new(0.3, 0.0, 1.0), 0.0),
            (Vec3::new(-1.0, 0.0, 0.0), 180.0),
            (Vec3::new(0.0, 1.0, 0.0), 30.0),
            (Vec3::NEG_Z, 45.0),
            (Vec3::new(0.0, 0.0, 0.0), 10.0),
            // the heading straight into a wall: the world's X or Y lay it along
            (Vec3::X, 0.0),
        ] {
            let angles = surface_angles(normal, yaw);
            let [forward, left, up] = axes_of(angles);
            let wanted_up = normal.normalize_or(Vec3::Z);
            assert!(close(up, wanted_up), "{normal} {yaw}: up {up}");
            assert!(forward.dot(up).abs() < 1e-4 && left.dot(up).abs() < 1e-4);
            assert!((forward.length() - 1.0).abs() < 1e-4);
            // the heading kept where the surface allows it
            let heading = Vec3::new(yaw.to_radians().cos(), yaw.to_radians().sin(), 0.0);
            let laid = (heading - wanted_up * heading.dot(wanted_up)).try_normalize();
            if let Some(laid) = laid {
                assert!(close(forward, laid), "{normal} {yaw}: forward {forward}");
            }
        }
    }

    #[test]
    fn a_heading_is_read_back_from_the_angles() {
        assert!((heading_of([0.0, 135.0, 0.0]) - 135.0).abs() < 1e-3);
        assert!((heading_of(surface_angles(Vec3::new(0.2, 0.1, 1.0), 60.0)) - 60.0).abs() < 2.0);
        // facing straight up: the heading comes from its up
        let up = heading_of([-90.0, 0.0, 0.0]);
        assert!(up.is_finite());
    }
}
