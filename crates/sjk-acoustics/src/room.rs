//! How the room round the listener rings: rays from the head bounce through the
//! level, each wall taking its material's share per band, and what comes back
//! becomes an impulse response for each ear ([`RoomTracer::trace`]).
//!
//! - **The rays.** `rays` directions round the head (a Fibonacci sphere, turned anew
//!   each pass so the passes add up to more), each followed for up to `bounces`
//!   walls. At each wall the share its material scatters reaches the head straight
//!   from there when nothing is between (diffuse rain); the rest mirrors on, and a
//!   mirrored ray that passes the head is heard too (a sphere of
//!   [`LISTENER_RADIUS`] catches it). The sky swallows a ray.
//! - **The table.** What arrives is kept by time ([`BIN_SECONDS`] bins), band and ear:
//!   each ear gets what the head's filters ([`sjk_audio::hrtf`]) give that
//!   direction in that band, so a wall on the left rings in the left ear first.
//!   Passes from about the same place are averaged, less grainy over time.
//! - **The tail.** Rays stop after their bounces, so a long hall loses its end:
//!   where many rays have stopped, the decay found before is carried on.
//! - **The response.** Fixed band-limited noise per ear, shaped by the table, is
//!   the impulse response, cut where it has fallen 60 dB, at most
//!   [`sjk_audio::MAX_RESPONSE_SECONDS`]. Loudness is relative to a sound
//!   [`REFERENCE_DISTANCE`] away in the open.

use crate::material::{MATERIALS, SKY};
use crate::scene::Scene;
use glam::{Quat, Vec3};
use sjk_audio::fft::{Complex, Fft};
use sjk_audio::hrtf::{BAND_EDGES_HZ, BANDS, HrtfSet};
use sjk_audio::{MAX_RESPONSE_SECONDS, RoomResponse};

/// Time bins of the energy table.
pub const BIN_SECONDS: f32 = 0.005;
const BINS: usize = (MAX_RESPONSE_SECONDS / BIN_SECONDS) as usize;
const RATE: f32 = 44_100.0;
const SAMPLES_PER_BIN: f32 = RATE * BIN_SECONDS;
/// Sound's speed in units a second (a unit about an inch).
pub const SPEED_OF_SOUND: f32 = 13_500.0;
/// Radius round the head that catches mirrored rays.
pub const LISTENER_RADIUS: f32 = 48.0;
/// The distance the reverb's loudness is measured against: a sound this far away
/// in the open is as loud as the response's 1.
pub const REFERENCE_DISTANCE: f32 = 128.0;
/// A wall nearer the head than this counts as this near (no spike from a wall the
/// player leans on).
const NEAREST_WALL: f32 = 48.0;
/// The longest segment a ray follows.
const RAY_REACH: f32 = 16_384.0;
/// What the air takes per unit travelled, per band (of the energy).
const AIR: [f32; 3] = [0.0, 0.000_02, 0.000_08];
/// The quietest the response is kept to, against its loudest bin (-60 dB).
const FLOOR: f32 = 1.0e-6;
/// Moved less than this since the last pass: the passes are averaged.
const SAME_PLACE: f32 = 48.0;
/// The weight of the newest pass in that average.
const NEWEST: f32 = 0.35;

/// How hard to trace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomSettings {
    pub rays: usize,
    pub bounces: usize,
    pub threads: usize,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            rays: 2_048,
            bounces: 24,
            threads: 2,
        }
    }
}

/// What a pass found, for the console.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoomStats {
    /// Share of the first rays that met a wall (not the sky or nothing).
    pub enclosed: f32,
    /// Seconds the mid band takes to fall 60 dB.
    pub decay_seconds: f32,
    /// Seconds of the response.
    pub length_seconds: f32,
    /// Microseconds the pass took.
    pub micros: u64,
}

/// Energy by ear, band and time bin.
#[derive(Clone)]
struct Table {
    bins: Vec<[[f32; BINS]; BANDS]>,
}

impl Table {
    fn new() -> Self {
        Self {
            bins: vec![[[0.0; BINS]; BANDS]; 2],
        }
    }

    fn add(&mut self, other: &Self) {
        for (mine, theirs) in self.bins.iter_mut().zip(&other.bins) {
            for (band, other_band) in mine.iter_mut().zip(theirs) {
                for (bin, value) in band.iter_mut().zip(other_band) {
                    *bin += value;
                }
            }
        }
    }

    fn blend(&mut self, newest: &Self, weight: f32) {
        for (mine, theirs) in self.bins.iter_mut().zip(&newest.bins) {
            for (band, other_band) in mine.iter_mut().zip(theirs) {
                for (bin, value) in band.iter_mut().zip(other_band) {
                    *bin += (value - *bin) * weight;
                }
            }
        }
    }
}

/// One thread's share of a pass.
struct Share {
    table: Table,
    /// When each ray that ran out of bounces stopped (seconds).
    stopped: Vec<f32>,
    enclosed: u32,
}

/// Traces rooms; holds the noise the responses are made of and the last table.
pub struct RoomTracer {
    hrtf: HrtfSet,
    /// Band-limited unit noise per ear and band.
    noise: [[Box<[f32]>; BANDS]; 2],
    last: Option<(Vec3, Table)>,
    pass: u32,
}

impl Default for RoomTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomTracer {
    pub fn new() -> Self {
        Self {
            hrtf: HrtfSet::kemar(),
            noise: band_noise(),
            last: None,
            pass: 0,
        }
    }

    /// Forget the last room (a new level).
    pub fn forget(&mut self) {
        self.last = None;
    }

    /// The room round `listener`, whose head looks along `axes` (forward, right,
    /// up).
    pub fn trace(
        &mut self,
        scene: &Scene,
        listener: Vec3,
        axes: [Vec3; 3],
        settings: RoomSettings,
    ) -> (RoomResponse, RoomStats) {
        let started = std::time::Instant::now();
        self.pass = self.pass.wrapping_add(1);
        let rays = settings.rays.clamp(64, 16_384);
        let bounces = settings.bounces.clamp(1, 64);
        let threads = settings.threads.clamp(1, 16);
        let turn = pass_turn(self.pass);
        let hrtf = &self.hrtf;
        let pass = self.pass;
        let shares: Vec<Share> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|thread| {
                    scope.spawn(move || {
                        let range = thread * rays / threads..(thread + 1) * rays / threads;
                        trace_rays(
                            scene, hrtf, listener, axes, turn, rays, range, bounces, pass,
                        )
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|handle| handle.join().ok())
                .collect()
        });
        let mut table = Table::new();
        let mut stopped = Vec::new();
        let mut enclosed = 0;
        for share in &shares {
            table.add(&share.table);
            stopped.extend_from_slice(&share.stopped);
            enclosed += share.enclosed;
        }
        let tail_from = tail_start(&mut stopped, rays);
        extend_tail(&mut table, tail_from);
        if let Some((place, last)) = &mut self.last
            && place.distance(listener) < SAME_PLACE
        {
            last.blend(&table, NEWEST);
            table = last.clone();
        }
        self.last = Some((listener, table.clone()));
        let (left, right, length) = self.synthesize(&table);
        let decay_seconds = decay_seconds(&table, 1);
        let stats = RoomStats {
            enclosed: enclosed as f32 / rays as f32,
            decay_seconds,
            length_seconds: length as f32 / RATE,
            micros: started.elapsed().as_micros() as u64,
        };
        (RoomResponse::from_impulses(&left, &right), stats)
    }

    /// Each ear's response from the table, and its length in samples.
    fn synthesize(&self, table: &Table) -> (Vec<f32>, Vec<f32>, usize) {
        let peak = table
            .bins
            .iter()
            .flatten()
            .flatten()
            .fold(0.0_f32, |most, value| most.max(*value));
        if peak <= 0.0 {
            return (Vec::new(), Vec::new(), 0);
        }
        let last_bin = (0..BINS)
            .rev()
            .find(|&bin| {
                table
                    .bins
                    .iter()
                    .any(|ear| ear.iter().any(|band| band[bin] > peak * FLOOR))
            })
            .unwrap_or(0);
        let length =
            (((last_bin + 1) as f32 * SAMPLES_PER_BIN) as usize).min(self.noise[0][0].len());
        let ears: Vec<Vec<f32>> = (0..2)
            .map(|ear| {
                let mut out = vec![0.0; length];
                for band in 0..BANDS {
                    let energy = &table.bins[ear][band];
                    let noise = &self.noise[ear][band];
                    let amplitude =
                        |bin: usize| (energy[bin.min(BINS - 1)] / SAMPLES_PER_BIN).sqrt();
                    for (index, sample) in out.iter_mut().enumerate() {
                        // Between bin centres, the amplitude goes in a line.
                        let at = (index as f32 / SAMPLES_PER_BIN - 0.5).max(0.0);
                        let bin = at as usize;
                        let t = at - bin as f32;
                        let level = amplitude(bin) * (1.0 - t) + amplitude(bin + 1) * t;
                        *sample += noise[index] * level;
                    }
                }
                out
            })
            .collect();
        let mut ears = ears.into_iter();
        let left = ears.next().unwrap_or_default();
        let right = ears.next().unwrap_or_default();
        (left, right, length)
    }
}

/// The turn of this pass's rays: a new one each pass, spread evenly.
fn pass_turn(pass: u32) -> Quat {
    let golden = 2.399_963_f32;
    let axis = Vec3::new(
        (pass as f32 * 0.618_034).fract() * 2.0 - 1.0,
        (pass as f32 * 0.414_214).fract() * 2.0 - 1.0,
        (pass as f32 * 0.732_051).fract() * 2.0 - 1.0,
    )
    .normalize_or(Vec3::Z);
    Quat::from_axis_angle(axis, pass as f32 * golden)
}

/// A small fast random number generator (xorshift*).
struct Random(u64);

impl Random {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32) / (1_u64 << 24) as f32
    }
}

/// A direction round `normal`, more of them near it (Lambert's).
fn scattered(normal: Vec3, random: &mut Random) -> Vec3 {
    let (a, b) = (random.next(), random.next());
    let radius = a.sqrt();
    let angle = std::f32::consts::TAU * b;
    let (tangent, bitangent) = normal.any_orthonormal_pair();
    (tangent * radius * angle.cos()
        + bitangent * radius * angle.sin()
        + normal * (1.0 - a).max(0.0).sqrt())
    .normalize_or(normal)
}

#[allow(clippy::too_many_arguments)]
fn trace_rays(
    scene: &Scene,
    hrtf: &HrtfSet,
    listener: Vec3,
    axes: [Vec3; 3],
    turn: Quat,
    rays: usize,
    range: std::ops::Range<usize>,
    bounces: usize,
    pass: u32,
) -> Share {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let mut share = Share {
        table: Table::new(),
        stopped: Vec::new(),
        enclosed: 0,
    };
    let mut random = Random(0x9e37_79b9_7f4a_7c15 ^ (u64::from(pass) << 32) ^ range.start as u64);
    let scale = 4.0 * REFERENCE_DISTANCE * REFERENCE_DISTANCE / rays as f32;
    let catch = LISTENER_RADIUS * LISTENER_RADIUS;
    let deposit = |table: &mut Table, seconds: f32, from: Vec3, energy: [f32; BANDS]| {
        let bin = (seconds / BIN_SECONDS) as usize;
        if bin >= BINS {
            return;
        }
        let local = [from.dot(axes[0]), from.dot(axes[1]), from.dot(axes[2])];
        let powers = hrtf.band_powers(local);
        for (ear, powers) in table.bins.iter_mut().zip(&powers) {
            for ((band, power), energy) in ear.iter_mut().zip(powers).zip(&energy) {
                band[bin] += energy * power;
            }
        }
    };
    for index in range {
        let z = 1.0 - (index as f32 + 0.5) * 2.0 / rays as f32;
        let radius = (1.0 - z * z).max(0.0).sqrt();
        let angle = golden * index as f32;
        let mut direction = turn * Vec3::new(radius * angle.cos(), radius * angle.sin(), z);
        let mut origin = listener;
        let mut travelled = 0.0_f32;
        let mut energy = [1.0_f32; BANDS];
        let mut mirrored = false;
        let mut ran_out = true;
        for bounce in 0..bounces {
            let Some(hit) = scene.closest(origin, direction, 0.01, RAY_REACH) else {
                ran_out = false;
                break;
            };
            if hit.material == SKY {
                ran_out = false;
                break;
            }
            if bounce == 0 {
                share.enclosed += 1;
            }
            let point = origin + direction * hit.distance;
            // A mirrored ray passing the head.
            if mirrored {
                let along = (listener - origin).dot(direction).clamp(0.0, hit.distance);
                let nearest = origin + direction * along;
                if nearest.distance_squared(listener) < catch {
                    let seconds = (travelled + along) / SPEED_OF_SOUND;
                    // Each ray through the sphere carries its share over its cross-section.
                    let caught = energy.map(|e| e * scale / catch);
                    deposit(&mut share.table, seconds, -direction, caught);
                }
            }
            travelled += hit.distance;
            let material = &MATERIALS[usize::from(hit.material).min(MATERIALS.len() - 1)];
            for band in 0..BANDS {
                energy[band] *=
                    (1.0 - material.absorption[band]) * (-AIR[band] * hit.distance).exp();
            }
            let normal = if hit.normal.dot(direction) > 0.0 {
                -hit.normal
            } else {
                hit.normal
            };
            // Diffuse rain: the scattered share straight to the head.
            let to_head = listener - point;
            let distance = to_head.length();
            let lifted = point + normal * 0.5;
            if distance > 1.0 {
                let cosine = normal.dot(to_head) / distance;
                if cosine > 0.0 && scene.sees(lifted, listener, 0.0) {
                    let near = distance.max(NEAREST_WALL);
                    let factor = scale * material.scattering.max(0.05) * cosine / (near * near);
                    let seconds = (travelled + distance) / SPEED_OF_SOUND;
                    deposit(
                        &mut share.table,
                        seconds,
                        -to_head / distance,
                        energy.map(|e| e * factor),
                    );
                }
            }
            if energy.iter().all(|e| *e < 1.0e-5) {
                ran_out = false;
                break;
            }
            if random.next() < material.scattering {
                direction = scattered(normal, &mut random);
                mirrored = false;
            } else {
                direction = direction - 2.0 * direction.dot(normal) * normal;
                mirrored = true;
            }
            origin = lifted;
        }
        if ran_out {
            share.stopped.push(travelled / SPEED_OF_SOUND);
        }
    }
    share
}

/// The bin from which the table is short of rays that ran out of bounces: when a
/// tenth of the rays have stopped. `None` when few ever did.
fn tail_start(stopped: &mut [f32], rays: usize) -> Option<usize> {
    if stopped.len() * 5 < rays {
        return None;
    }
    stopped.sort_by(f32::total_cmp);
    let seconds = stopped[rays / 10].max(0.05);
    Some((seconds / BIN_SECONDS) as usize)
}

/// Carry each band's decay on past `from`: a line fitted to its log energy over
/// the bins before, both ears in their share.
fn extend_tail(table: &mut Table, from: Option<usize>) {
    let Some(from) = from.filter(|from| *from < BINS) else {
        return;
    };
    let start = from * 2 / 5;
    for band in 0..BANDS {
        let points: Vec<(f32, f32)> = (start..from)
            .filter_map(|bin| {
                let energy = table.bins[0][band][bin] + table.bins[1][band][bin];
                (energy > 0.0).then(|| (bin as f32, energy.ln()))
            })
            .collect();
        if points.len() < 6 {
            continue;
        }
        let count = points.len() as f32;
        let mean_x = points.iter().map(|p| p.0).sum::<f32>() / count;
        let mean_y = points.iter().map(|p| p.1).sum::<f32>() / count;
        let covariance: f32 = points.iter().map(|p| (p.0 - mean_x) * (p.1 - mean_y)).sum();
        let variance: f32 = points.iter().map(|p| (p.0 - mean_x).powi(2)).sum();
        let slope = covariance / variance.max(f32::EPSILON);
        if slope >= -1.0e-4 {
            continue;
        }
        let (left, right): (f32, f32) = (start..from).fold((0.0, 0.0), |(l, r), bin| {
            (l + table.bins[0][band][bin], r + table.bins[1][band][bin])
        });
        let left_share = left / (left + right).max(f32::MIN_POSITIVE);
        for bin in from..BINS {
            let energy = (mean_y + slope * (bin as f32 - mean_x)).exp();
            table.bins[0][band][bin] = energy * left_share;
            table.bins[1][band][bin] = energy * (1.0 - left_share);
        }
    }
}

/// Seconds `band` takes to fall 60 dB (Schroeder's backward sum, fitted from -5 to
/// -25 dB).
fn decay_seconds(table: &Table, band: usize) -> f32 {
    let mut remaining = [0.0_f32; BINS];
    let mut sum = 0.0;
    for bin in (0..BINS).rev() {
        sum += table.bins[0][band][bin] + table.bins[1][band][bin];
        remaining[bin] = sum;
    }
    let total = remaining[0];
    if total <= 0.0 {
        return 0.0;
    }
    let level = |bin: usize| 10.0 * (remaining[bin] / total).max(1.0e-12).log10();
    let first = (0..BINS).find(|&bin| level(bin) <= -5.0);
    let last = (0..BINS).find(|&bin| level(bin) <= -25.0);
    match (first, last) {
        (Some(first), Some(last)) if last > first => (last - first) as f32 * BIN_SECONDS * 3.0,
        _ => 0.0,
    }
}

/// Unit-power white noise per ear, split into the three bands, as long as the
/// longest response.
fn band_noise() -> [[Box<[f32]>; BANDS]; 2] {
    let length = (MAX_RESPONSE_SECONDS * RATE) as usize;
    let size = length.next_power_of_two();
    let fft = Fft::new(size);
    let bin_hz = RATE / size as f32;
    let mut random = Random(0x5eed_a11c_e500_0001);
    std::array::from_fn(|_| {
        let mut white: Vec<Complex> = (0..size)
            .map(|_| Complex::new(random.next() * 2.0 - 1.0, 0.0))
            .collect();
        fft.forward(&mut white);
        std::array::from_fn(|band| {
            let mut spectrum = white.clone();
            for (index, value) in spectrum.iter_mut().enumerate() {
                let bin = index.min(size - index);
                let gain = band_gain(band, bin as f32 * bin_hz);
                value.re *= gain;
                value.im *= gain;
            }
            fft.inverse(&mut spectrum);
            let mut samples: Vec<f32> = spectrum[..length].iter().map(|v| v.re).collect();
            let power = samples.iter().map(|s| s * s).sum::<f32>() / length as f32;
            let scale = 1.0 / power.max(f32::MIN_POSITIVE).sqrt();
            for sample in &mut samples {
                *sample *= scale;
            }
            samples.into_boxed_slice()
        })
    })
}

/// How much of `hz` a band keeps: a smooth crossover an octave wide at each edge,
/// the three adding up to one in power.
fn band_gain(band: usize, hz: f32) -> f32 {
    let below = |edge: f32| {
        let octaves = (hz.max(1.0) / edge).log2().clamp(-0.5, 0.5);
        // Power share below the edge, 1 to 0 over the octave.
        0.5 - 0.5 * (octaves * std::f32::consts::PI).sin()
    };
    let (low_edge, high_edge) = (BAND_EDGES_HZ[0], BAND_EDGES_HZ[1]);
    let share = match band {
        0 => below(low_edge),
        1 => (1.0 - below(low_edge)) * below(high_edge),
        _ => 1.0 - below(high_edge),
    };
    share.max(0.0).sqrt()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::bvh::Triangle;
    use crate::scene::LevelGeometry;

    /// A closed box of 12 triangles from `low` to `high`, concrete.
    pub(crate) fn cube(low: Vec3, high: Vec3, brush: u32) -> Vec<Triangle> {
        let corner = |x: bool, y: bool, z: bool| {
            Vec3::new(
                if x { high.x } else { low.x },
                if y { high.y } else { low.y },
                if z { high.z } else { low.z },
            )
        };
        let faces = [
            (
                [
                    (false, false, false),
                    (false, true, false),
                    (false, true, true),
                    (false, false, true),
                ],
                -Vec3::X,
            ),
            (
                [
                    (true, false, false),
                    (true, true, false),
                    (true, true, true),
                    (true, false, true),
                ],
                Vec3::X,
            ),
            (
                [
                    (false, false, false),
                    (true, false, false),
                    (true, false, true),
                    (false, false, true),
                ],
                -Vec3::Y,
            ),
            (
                [
                    (false, true, false),
                    (true, true, false),
                    (true, true, true),
                    (false, true, true),
                ],
                Vec3::Y,
            ),
            (
                [
                    (false, false, false),
                    (true, false, false),
                    (true, true, false),
                    (false, true, false),
                ],
                -Vec3::Z,
            ),
            (
                [
                    (false, false, true),
                    (true, false, true),
                    (true, true, true),
                    (false, true, true),
                ],
                Vec3::Z,
            ),
        ];
        faces
            .iter()
            .flat_map(|(q, normal)| {
                let q = q.map(|(x, y, z)| corner(x, y, z));
                [
                    Triangle::new([q[0], q[1], q[2]], *normal, 11, brush),
                    Triangle::new([q[0], q[2], q[3]], *normal, 11, brush),
                ]
            })
            .collect()
    }

    /// A hollow room: six slabs round the inside from `-half` to `half`.
    pub(crate) fn room(half: Vec3) -> Vec<Triangle> {
        let t = 16.0;
        let mut triangles = Vec::new();
        for axis in 0..3 {
            for sign in [-1.0, 1.0] {
                let mut low = -half - Vec3::splat(t);
                let mut high = half + Vec3::splat(t);
                if sign < 0.0 {
                    high[axis] = -half[axis];
                } else {
                    low[axis] = half[axis];
                }
                triangles.extend(cube(low, high, (axis * 2) as u32 + u32::from(sign > 0.0)));
            }
        }
        triangles
    }

    /// The ring of a room hung with cloth (an absorbent room: its decay fits in
    /// the response).
    fn ring(half: Vec3) -> RoomStats {
        let mut triangles = room(half);
        triangles.iter_mut().for_each(|t| t.material = 21);
        let geometry = LevelGeometry::from_triangles(triangles);
        let scene = Scene {
            geometry: &geometry,
            movers: &[],
        };
        let mut tracer = RoomTracer::new();
        let axes = [Vec3::X, -Vec3::Y, Vec3::Z];
        tracer
            .trace(
                &scene,
                Vec3::new(0.0, 0.0, 0.0),
                axes,
                RoomSettings::default(),
            )
            .1
    }

    #[test]
    fn a_hall_rings_longer_than_a_cupboard() {
        let cupboard = ring(Vec3::new(64.0, 64.0, 64.0));
        let hall = ring(Vec3::new(1_024.0, 1_024.0, 512.0));
        assert!(cupboard.enclosed > 0.99 && hall.enclosed > 0.99);
        assert!(
            hall.decay_seconds > cupboard.decay_seconds * 3.0,
            "{cupboard:?} {hall:?}"
        );
        assert!(
            hall.length_seconds > cupboard.length_seconds,
            "{cupboard:?} {hall:?}"
        );
    }

    #[test]
    fn under_the_open_sky_nothing_rings() {
        let geometry = LevelGeometry::from_triangles(Vec::new());
        let scene = Scene {
            geometry: &geometry,
            movers: &[],
        };
        let (response, stats) = RoomTracer::new().trace(
            &scene,
            Vec3::ZERO,
            [Vec3::X, -Vec3::Y, Vec3::Z],
            RoomSettings::default(),
        );
        assert_eq!(stats.enclosed, 0.0);
        assert_eq!(response.partitions(), 1);
    }

    #[test]
    fn a_wall_on_the_left_rings_in_the_left_ear_first() {
        // A single wall 200 units to the left (+Y), a floor far below.
        let mut triangles = cube(
            Vec3::new(-2_000.0, 200.0, -2_000.0),
            Vec3::new(2_000.0, 240.0, 2_000.0),
            0,
        );
        triangles.iter_mut().for_each(|t| t.material = 21);
        let geometry = LevelGeometry::from_triangles(triangles);
        let scene = Scene {
            geometry: &geometry,
            movers: &[],
        };
        let mut tracer = RoomTracer::new();
        let axes = [Vec3::X, -Vec3::Y, Vec3::Z];
        let mut table = Table::new();
        let share = trace_rays(
            &scene,
            &tracer.hrtf,
            Vec3::ZERO,
            axes,
            Quat::IDENTITY,
            2_048,
            0..2_048,
            4,
            1,
        );
        table.add(&share.table);
        let ear = |ear: usize| table.bins[ear][2].iter().sum::<f32>();
        assert!(ear(0) > ear(1) * 2.0, "left {} right {}", ear(0), ear(1));
        tracer.forget();
    }

    #[test]
    fn the_bands_add_up_to_the_whole() {
        for hz in [50.0, 300.0, 500.0, 900.0, 3_000.0, 4_000.0, 9_000.0] {
            let total: f32 = (0..3).map(|band| band_gain(band, hz).powi(2)).sum();
            assert!((total - 1.0).abs() < 1.0e-4, "{hz}: {total}");
        }
    }
}
