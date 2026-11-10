//! What reaches the listener straight from one sound: how much of it is in sight,
//! what the walls in between let through, and, when it is hidden, the way round a
//! corner it takes instead.
//!
//! - **In sight.** Rays from the listener to [`SPHERE`] points round the sound (a
//!   sound has a size: a muzzle, a door) count the share that is free. Points the
//!   sound itself cannot see (under the floor it stands on) do not count.
//! - **Through walls.** Along the straight line, every wall entered takes its
//!   material's [`crate::material::Acoustic::transmission`]; the share hidden is
//!   heard through them.
//! - **Round a corner.** For a hidden sound, rays from the sound and from the
//!   listener look for a point both see: the doorway, the end of the corridor. The
//!   sound comes from there, quieter by the longer way and duller the more it
//!   bends (the highs bend least), and it is heard as coming from there. One
//!   corner: a sound two rooms away is heard through the walls only. Nothing is
//!   baked, so a door shut or open counts as it is.

use crate::material::MATERIALS;
use crate::scene::Scene;
use glam::Vec3;
use sjk_audio::SourceAcoustics;

/// Points round a sound tested for sight.
pub const SPHERE: usize = 12;
/// How large a sound is (units): its points are this far from it.
const SOURCE_RADIUS: f32 = 16.0;
/// A wall this close before the sound is the one it is on (an impact): not counted.
const NEAR_SOURCE: f32 = 12.0;
/// Walls counted along the straight line at most.
const MAX_WALLS: usize = 6;
/// Rays cast from each end looking for a corner.
const CORNER_RAYS: usize = 20;
/// How far those rays look.
const CORNER_REACH: f32 = 1_500.0;
/// Where along each ray (of the way to its wall) a corner is looked for.
const CORNER_STEPS: [f32; 3] = [0.35, 0.65, 0.92];
/// What bending round a corner takes per band, in dB at a right angle (half at
/// 45 degrees, double turning back).
const BEND_LOSS_DB: [f32; 3] = [3.0, 7.0, 12.0];
/// A sound further than this is past hearing: not traced.
pub const REACH: f32 = 2_500.0;

/// The straight way's result and the corner's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direct {
    /// Share of the sound in sight, 0 to 1.
    pub visible: f32,
    pub acoustics: SourceAcoustics,
}

/// Unit directions spread over the sphere (a Fibonacci spiral).
pub fn sphere<const N: usize>() -> [Vec3; N] {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    std::array::from_fn(|index| {
        let z = 1.0 - (index as f32 + 0.5) * 2.0 / N as f32;
        let radius = (1.0 - z * z).max(0.0).sqrt();
        let angle = golden * index as f32;
        Vec3::new(radius * angle.cos(), radius * angle.sin(), z)
    })
}

/// Works out [`Direct`] for sounds; holds its fixed ray tables.
pub struct DirectTracer {
    points: [Vec3; SPHERE],
    corner: [Vec3; CORNER_RAYS],
}

impl Default for DirectTracer {
    fn default() -> Self {
        Self {
            points: sphere::<SPHERE>(),
            corner: sphere::<CORNER_RAYS>(),
        }
    }
}

impl DirectTracer {
    /// What reaches `listener` from the sound at `source`; `previous_via` is the
    /// corner it came round last time, kept while it is nearly as good (so the
    /// sound does not hop between equal ways).
    pub fn trace(
        &self,
        scene: &Scene,
        listener: Vec3,
        source: Vec3,
        previous_via: Option<Vec3>,
    ) -> Direct {
        let distance = listener.distance(source);
        if !(1.0..=REACH).contains(&distance) {
            return Direct {
                visible: 1.0,
                acoustics: SourceAcoustics::OPEN,
            };
        }
        let (mut counted, mut seen) = (0_u32, 0_u32);
        for offset in &self.points {
            let point = source + *offset * SOURCE_RADIUS;
            if !scene.sees(source, point, 0.0) {
                continue;
            }
            counted += 1;
            if scene.sees(listener, point, NEAR_SOURCE) {
                seen += 1;
            }
        }
        let visible = if counted == 0 {
            f32::from(u8::from(scene.sees(listener, source, NEAR_SOURCE)))
        } else {
            seen as f32 / counted as f32
        };
        if visible >= 0.999 {
            return Direct {
                visible,
                acoustics: SourceAcoustics::OPEN,
            };
        }
        let through = through_walls(scene, listener, source);
        let mut bands: [f32; 3] =
            std::array::from_fn(|band| visible + (1.0 - visible) * through[band]);
        let mut via = None;
        if let Some((point, corner)) = self.corner(scene, listener, source, distance, previous_via)
            && corner[1] > bands[1]
        {
            bands = std::array::from_fn(|band| bands[band].max(corner[band]));
            via = Some(point.to_array());
        }
        Direct {
            visible,
            acoustics: SourceAcoustics { bands, via },
        }
    }

    /// The best point both ends see, and what reaches the listener through it.
    fn corner(
        &self,
        scene: &Scene,
        listener: Vec3,
        source: Vec3,
        distance: f32,
        previous: Option<Vec3>,
    ) -> Option<(Vec3, [f32; 3])> {
        let mut best: Option<(f32, Vec3)> = None;
        let mut consider = |point: Vec3| {
            let length = source.distance(point) + point.distance(listener);
            if best.is_none_or(|(shortest, _)| length < shortest) {
                best = Some((length, point));
            }
        };
        // From each end, points the other end sees.
        for (from, to) in [(source, listener), (listener, source)] {
            for direction in &self.corner {
                let reach = scene
                    .closest(from, *direction, 0.5, CORNER_REACH)
                    .map_or(CORNER_REACH, |hit| hit.distance);
                for step in CORNER_STEPS {
                    let point = from + *direction * (reach * step).max(SOURCE_RADIUS);
                    if scene.sees(to, point, 0.0) {
                        consider(point);
                    }
                }
            }
        }
        let (mut length, mut point) = best?;
        if let Some(previous) = previous {
            let kept = source.distance(previous) + previous.distance(listener);
            if kept < length * 1.08
                && scene.sees(source, previous, 0.0)
                && scene.sees(listener, previous, 0.0)
            {
                (length, point) = (kept, previous);
            }
        }
        let into = (point - source).normalize_or_zero();
        let out = (listener - point).normalize_or_zero();
        let bend = into.dot(out).clamp(-1.0, 1.0).acos() / std::f32::consts::FRAC_PI_2;
        // The longer way spreads the sound more (1/r).
        let spread = (distance / length).clamp(0.2, 1.0);
        let bands = BEND_LOSS_DB.map(|loss| spread * 10.0_f32.powf(-loss * bend / 20.0));
        Some((point, bands))
    }
}

/// What the walls on the straight line let through, per band.
fn through_walls(scene: &Scene, listener: Vec3, source: Vec3) -> [f32; 3] {
    let delta = source - listener;
    let length = delta.length();
    let direction = delta / length;
    let mut gains = [1.0_f32; 3];
    let mut at = 0.5;
    let mut walls = 0;
    let end = length - NEAR_SOURCE;
    let mut last_wall = None;
    while walls < MAX_WALLS * 2 {
        let Some(hit) = scene.closest(listener, direction, at, end) else {
            break;
        };
        at = hit.distance + 0.5;
        walls += 1;
        // A brush is entered through a side facing the ray; a curve at every hit.
        let entering = hit.normal.dot(direction) < 0.0 || hit.wall.is_none();
        if entering && (hit.wall.is_none() || hit.wall != last_wall) {
            let material = &MATERIALS[usize::from(hit.material).min(MATERIALS.len() - 1)];
            for (gain, through) in gains.iter_mut().zip(&material.transmission) {
                *gain *= through;
            }
        }
        last_wall = hit.wall;
    }
    gains
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bvh::Triangle;
    use crate::scene::LevelGeometry;

    fn slab(low: Vec3, high: Vec3, brush: u32) -> Vec<Triangle> {
        crate::room::tests::cube(low, high, brush)
    }

    #[test]
    fn a_sound_in_the_open_is_open() {
        let geometry = LevelGeometry::from_triangles(slab(
            Vec3::new(-1000.0, -1000.0, -20.0),
            Vec3::new(1000.0, 1000.0, 0.0),
            0,
        ));
        let scene = Scene {
            geometry: &geometry,
            movers: &[],
        };
        let direct = DirectTracer::default().trace(
            &scene,
            Vec3::new(0.0, 0.0, 40.0),
            Vec3::new(300.0, 0.0, 1.0),
            None,
        );
        // Standing on the floor: the points under it do not count.
        assert_eq!(direct.acoustics, SourceAcoustics::OPEN);
    }

    #[test]
    fn a_wall_takes_the_highs_and_a_doorway_brings_them_back_from_its_side() {
        let tracer = DirectTracer::default();
        let (listener, source) = (Vec3::new(0.0, 0.0, 50.0), Vec3::new(400.0, 0.0, 50.0));
        // Floor and ceiling, and a wall across x = 200 with a doorway at y 400..520.
        let mut shell = slab(
            Vec3::new(-4000.0, -4000.0, -20.0),
            Vec3::new(4000.0, 4000.0, 0.0),
            10,
        );
        shell.extend(slab(
            Vec3::new(-4000.0, -4000.0, 300.0),
            Vec3::new(4000.0, 4000.0, 320.0),
            11,
        ));
        let mut doorway = shell.clone();
        doorway.extend(slab(
            Vec3::new(190.0, -4000.0, 0.0),
            Vec3::new(210.0, 400.0, 300.0),
            1,
        ));
        doorway.extend(slab(
            Vec3::new(190.0, 520.0, 0.0),
            Vec3::new(210.0, 4000.0, 300.0),
            2,
        ));
        let doorway = LevelGeometry::from_triangles(doorway);
        let scene = Scene {
            geometry: &doorway,
            movers: &[],
        };
        let heard = tracer.trace(&scene, listener, source, None);
        assert!(heard.visible < 0.01);
        let via = Vec3::from_array(heard.acoustics.via.expect("round the doorway"));
        assert!(via.y > 300.0, "{via}");
        let bands = heard.acoustics.bands;
        assert!(bands[2] < bands[0] && bands[1] < 0.9, "{bands:?}");
        // Sealed, it comes through the wall only: less, and concrete's 0.06 of the mids.
        let mut sealed = shell;
        sealed.extend(slab(
            Vec3::new(190.0, -4000.0, 0.0),
            Vec3::new(210.0, 4000.0, 300.0),
            4,
        ));
        let sealed = LevelGeometry::from_triangles(sealed);
        let scene = Scene {
            geometry: &sealed,
            movers: &[],
        };
        let shut = tracer.trace(&scene, listener, source, None);
        assert!(shut.acoustics.via.is_none());
        assert!(shut.acoustics.bands[1] < heard.acoustics.bands[1]);
        assert!(
            (shut.acoustics.bands[1] - 0.06).abs() < 0.01,
            "{:?}",
            shut.acoustics.bands
        );
    }
}
