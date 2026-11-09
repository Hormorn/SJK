//! Head-related impulse responses of the 3D sound (`s_spatial`): how a sound from
//! one direction reaches each ear, measured on a KEMAR dummy head by Bill Gardner
//! and Keith Martin at the MIT Media Lab (1994, diffuse-field equalized set; free to
//! use, with credit). `scripts/hrtf_kemar.py` turns the 368 measured directions of
//! the right half into `data/hrtf_kemar.bin`: per ear a minimum-phase filter of
//! [`HRIR_TAPS`] taps and the delay after the near ear. The left half is the right
//! one with the ears swapped (the dummy head is symmetric).
//!
//! A direction between measured ones blends its four nearest: two on each of the
//! two nearest rings of elevation. Minimum-phase filters and separate delays blend
//! without the comb filtering raw impulse responses give.

use crate::fft::{Complex, Fft};

/// Taps of each ear's filter.
pub const HRIR_TAPS: usize = 64;

/// The sample rate the filters were measured at, the mixer's.
pub const HRTF_SAMPLE_RATE: u32 = 44_100;

/// Edges of the three frequency bands the room acoustics work in (low, mid, high).
pub const BAND_EDGES_HZ: [f32; 2] = [500.0, 4_000.0];

/// Bands of [`BAND_EDGES_HZ`].
pub const BANDS: usize = 3;

/// The energy both ears get together from a sound, averaged over every direction:
/// near the old mixer's (0.5 straight ahead, 1 at one side).
const TARGET_ENERGY: f32 = 0.6;

static DATA: &[u8] = include_bytes!("../data/hrtf_kemar.bin");

/// Both ears' filters and delays for one direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HrirPair {
    /// Left and right ear filters.
    pub taps: [[f32; HRIR_TAPS]; 2],
    /// Left and right ear delays in samples (the near ear's is about 0).
    pub delays: [f32; 2],
}

impl HrirPair {
    pub const SILENT: Self = Self {
        taps: [[0.0; HRIR_TAPS]; 2],
        delays: [0.0; 2],
    };
}

#[derive(Clone, Debug)]
struct Entry {
    pair: HrirPair,
    /// Each ear's power in each band, 1 on average over all directions.
    bands: [[f32; BANDS]; 2],
}

#[derive(Clone, Debug)]
struct Ring {
    elevation: f32,
    /// Azimuths 0 to 360 (0 ahead, 90 right), ascending, with their entry.
    azimuths: Vec<(f32, usize)>,
}

/// Every direction's filters, built once.
#[derive(Clone, Debug)]
pub struct HrtfSet {
    entries: Vec<Entry>,
    rings: Vec<Ring>,
}

/// The four entries a direction blends and their weights.
type Blend = [(usize, f32); 4];

impl HrtfSet {
    /// The built-in KEMAR set.
    pub fn kemar() -> Self {
        Self::parse(DATA).expect("the bundled HRTF table is valid")
    }

    fn parse(data: &[u8]) -> Option<Self> {
        let read_u16 = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
        let read_i16 = |at: usize| Some(i16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
        let read_u32 = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
        let read_f32 = |at: usize| Some(f32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
        if data.get(..4)? != b"SJKH" || read_u16(4)? != 1 || read_u32(6)? != HRTF_SAMPLE_RATE {
            return None;
        }
        if usize::from(read_u16(10)?) != HRIR_TAPS {
            return None;
        }
        let count = usize::from(read_u16(12)?);
        let scale = read_f32(14)? / 32_767.0;
        let mut at = 18;
        let mut measured = Vec::with_capacity(count);
        for _ in 0..count {
            let elevation = f32::from(read_i16(at)?);
            let azimuth = f32::from(read_i16(at + 2)?);
            let delays = [read_f32(at + 4)?, read_f32(at + 8)?];
            at += 12;
            let mut taps = [[0.0; HRIR_TAPS]; 2];
            for ear in &mut taps {
                for tap in ear.iter_mut() {
                    *tap = f32::from(read_i16(at)?) * scale;
                    at += 2;
                }
            }
            measured.push((elevation, azimuth, HrirPair { taps, delays }));
        }
        // Loudness: the mean energy of both ears together is TARGET_ENERGY.
        let mean: f32 = measured
            .iter()
            .map(|(.., pair)| energy(&pair.taps[0]) + energy(&pair.taps[1]))
            .sum::<f32>()
            / count.max(1) as f32;
        let gain = (TARGET_ENERGY / mean.max(f32::EPSILON)).sqrt();
        let fft = Fft::new(256);
        let mut entries = Vec::with_capacity(count * 2);
        let mut rings: Vec<Ring> = Vec::new();
        for (elevation, azimuth, mut pair) in measured {
            for ear in &mut pair.taps {
                for tap in ear.iter_mut() {
                    *tap *= gain;
                }
            }
            let bands = [
                band_powers(&fft, &pair.taps[0]),
                band_powers(&fft, &pair.taps[1]),
            ];
            let ring = match rings.iter().position(|ring| ring.elevation == elevation) {
                Some(index) => index,
                None => {
                    rings.push(Ring {
                        elevation,
                        azimuths: Vec::new(),
                    });
                    rings.len() - 1
                }
            };
            entries.push(Entry { pair, bands });
            rings[ring].azimuths.push((azimuth, entries.len() - 1));
            if azimuth > 0.0 && azimuth < 180.0 {
                // The left half: the same head, the ears swapped.
                let mirrored = Entry {
                    pair: HrirPair {
                        taps: [pair.taps[1], pair.taps[0]],
                        delays: [pair.delays[1], pair.delays[0]],
                    },
                    bands: [bands[1], bands[0]],
                };
                entries.push(mirrored);
                rings[ring]
                    .azimuths
                    .push((360.0 - azimuth, entries.len() - 1));
            }
        }
        // Each band 1 on average over every direction, both ears.
        let mut means = [0.0_f32; BANDS];
        for entry in &entries {
            for (mean, (left, right)) in means
                .iter_mut()
                .zip(entry.bands[0].iter().zip(&entry.bands[1]))
            {
                *mean += (left + right) * 0.5;
            }
        }
        for mean in &mut means {
            *mean /= entries.len().max(1) as f32;
        }
        for entry in &mut entries {
            for ear in &mut entry.bands {
                for band in 0..BANDS {
                    ear[band] /= means[band].max(f32::EPSILON);
                }
            }
        }
        rings.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        for ring in &mut rings {
            ring.azimuths.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        Some(Self { entries, rings })
    }

    /// Directions the set has, both halves.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Write the filters of a sound from `direction` into `out`. `direction` is in
    /// the listener's frame: `[forward, right, up]`, any length but zero.
    pub fn filters(&self, direction: [f32; 3], out: &mut HrirPair) {
        let blend = self.blend(direction);
        *out = HrirPair::SILENT;
        for (index, weight) in blend {
            if weight <= 0.0 {
                continue;
            }
            let pair = &self.entries[index].pair;
            for ear in 0..2 {
                for (tap, value) in out.taps[ear].iter_mut().zip(&pair.taps[ear]) {
                    *tap += value * weight;
                }
                out.delays[ear] += pair.delays[ear] * weight;
            }
        }
    }

    /// Each ear's power in the three bands for a sound from `direction` (as for
    /// [`Self::filters`]), 1 on average over every direction.
    pub fn band_powers(&self, direction: [f32; 3]) -> [[f32; BANDS]; 2] {
        let mut out = [[0.0; BANDS]; 2];
        for (index, weight) in self.blend(direction) {
            for (out, bands) in out.iter_mut().zip(&self.entries[index].bands) {
                for (power, band) in out.iter_mut().zip(bands) {
                    *power += band * weight;
                }
            }
        }
        out
    }

    fn blend(&self, direction: [f32; 3]) -> Blend {
        let [forward, right, up] = direction;
        let length = (forward * forward + right * right + up * up).sqrt();
        let (elevation, azimuth) = if length > f32::EPSILON {
            let elevation = (up / length).clamp(-1.0, 1.0).asin().to_degrees();
            let azimuth = right.atan2(forward).to_degrees().rem_euclid(360.0);
            (elevation, azimuth)
        } else {
            (0.0, 0.0)
        };
        let lowest = self.rings.first().map_or(0.0, |ring| ring.elevation);
        let elevation = elevation.max(lowest);
        let upper = self
            .rings
            .iter()
            .position(|ring| ring.elevation >= elevation)
            .unwrap_or(self.rings.len() - 1);
        let lower = upper.saturating_sub(1);
        let (low, high) = (&self.rings[lower], &self.rings[upper]);
        let t = if upper == lower || high.elevation <= low.elevation {
            1.0
        } else {
            (elevation - low.elevation) / (high.elevation - low.elevation)
        };
        let [a, b] = ring_blend(low, azimuth);
        let [c, d] = ring_blend(high, azimuth);
        [
            (a.0, a.1 * (1.0 - t)),
            (b.0, b.1 * (1.0 - t)),
            (c.0, c.1 * t),
            (d.0, d.1 * t),
        ]
    }
}

/// The two entries of `ring` round `azimuth` and their weights.
fn ring_blend(ring: &Ring, azimuth: f32) -> [(usize, f32); 2] {
    let azimuths = &ring.azimuths;
    if azimuths.len() == 1 {
        return [(azimuths[0].1, 1.0), (azimuths[0].1, 0.0)];
    }
    let next = azimuths.iter().position(|(a, _)| *a > azimuth).unwrap_or(0);
    let previous = (next + azimuths.len() - 1) % azimuths.len();
    let (from, to) = (azimuths[previous].0, azimuths[next].0);
    let span = (to - from).rem_euclid(360.0).max(f32::EPSILON);
    let along = ((azimuth - from).rem_euclid(360.0) / span).clamp(0.0, 1.0);
    [
        (azimuths[previous].1, 1.0 - along),
        (azimuths[next].1, along),
    ]
}

fn energy(taps: &[f32]) -> f32 {
    taps.iter().map(|tap| tap * tap).sum()
}

/// A filter's mean power in each band.
fn band_powers(fft: &Fft, taps: &[f32; HRIR_TAPS]) -> [f32; BANDS] {
    let mut spectrum = [Complex::ZERO; 256];
    for (bin, tap) in spectrum.iter_mut().zip(taps) {
        bin.re = *tap;
    }
    fft.forward(&mut spectrum);
    let bin_hz = HRTF_SAMPLE_RATE as f32 / 256.0;
    let mut sums = [0.0_f32; BANDS];
    let mut counts = [0_u32; BANDS];
    for (index, bin) in spectrum[1..=128].iter().enumerate() {
        let hz = (index + 1) as f32 * bin_hz;
        let band = band_of(hz);
        sums[band] += bin.re * bin.re + bin.im * bin.im;
        counts[band] += 1;
    }
    std::array::from_fn(|band| sums[band] / counts[band].max(1) as f32)
}

/// The band (0 low, 1 mid, 2 high) a frequency falls in.
pub fn band_of(hz: f32) -> usize {
    BAND_EDGES_HZ.iter().filter(|edge| hz >= **edge).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_set_covers_both_halves() {
        let set = HrtfSet::kemar();
        // 368 measured, all but the 0 and 180 degree ones mirrored.
        assert!(set.len() > 650, "{}", set.len());
    }

    #[test]
    fn a_sound_on_the_right_is_louder_and_sooner_in_the_right_ear() {
        let set = HrtfSet::kemar();
        let mut pair = HrirPair::SILENT;
        set.filters([0.0, 1.0, 0.0], &mut pair);
        assert!(energy(&pair.taps[1]) > 4.0 * energy(&pair.taps[0]));
        assert!(
            pair.delays[0] > 20.0 && pair.delays[1] < 2.0,
            "{:?}",
            pair.delays
        );
        set.filters([0.0, -1.0, 0.0], &mut pair);
        assert!(energy(&pair.taps[0]) > 4.0 * energy(&pair.taps[1]));
    }

    #[test]
    fn straight_ahead_both_ears_are_alike() {
        let set = HrtfSet::kemar();
        let mut pair = HrirPair::SILENT;
        set.filters([1.0, 0.0, 0.0], &mut pair);
        let (left, right) = (energy(&pair.taps[0]), energy(&pair.taps[1]));
        assert!((left - right).abs() < 0.02 * left);
        assert!(pair.delays[0] < 1.0 && pair.delays[1] < 1.0);
    }

    #[test]
    fn blends_move_smoothly_between_measured_directions() {
        let set = HrtfSet::kemar();
        let mut last = None;
        for step in 0..=360 {
            let angle = (step as f32).to_radians();
            let mut pair = HrirPair::SILENT;
            set.filters([angle.cos(), angle.sin(), 0.1], &mut pair);
            let delay = pair.delays[0] - pair.delays[1];
            if let Some(previous) = last {
                let jump: f32 = delay - previous;
                assert!(jump.abs() < 1.5, "step {step}: {previous} -> {delay}");
            }
            last = Some(delay);
        }
    }

    #[test]
    fn band_powers_average_one_and_follow_the_side() {
        let set = HrtfSet::kemar();
        let right = set.band_powers([0.0, 1.0, 0.0]);
        assert!(right[1][2] > right[0][2] * 4.0, "{right:?}");
        let below = set.band_powers([0.0, 0.0, -1.0]);
        assert!(below.iter().flatten().all(|power| power.is_finite()));
    }
}
