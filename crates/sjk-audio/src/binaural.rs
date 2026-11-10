//! One voice of the 3D sound heard through the head: its sound filtered by what is
//! in the way ([`BandEq`]: a gain per band of [`crate::hrtf::BAND_EDGES_HZ`]), then
//! through each ear's filter and delay for its direction ([`Binaural`]). Every
//! buffer is fixed: a block allocates nothing.

use crate::hrtf::{BANDS, HRIR_TAPS, HrirPair};

/// Frames a voice is filtered in at a time; a longer block goes in parts.
pub(crate) const BLOCK: usize = 128;

/// The longest ear delay taken, in samples (the head's is about 30 at 44.1 kHz).
const MAX_DELAY: usize = 48;

/// Input kept from block to block: the filter's length and the longest delay.
const HISTORY: usize = HRIR_TAPS + MAX_DELAY + 2;

/// Scratch the voices share while one of them is filtered: an ear's delayed input.
pub(crate) struct BinauralScratch {
    delayed: [f32; BLOCK + HRIR_TAPS],
    previous: [f32; BLOCK + HRIR_TAPS],
}

impl BinauralScratch {
    pub(crate) fn new() -> Self {
        Self {
            delayed: [0.0; BLOCK + HRIR_TAPS],
            previous: [0.0; BLOCK + HRIR_TAPS],
        }
    }
}

/// A voice's ears: its input lately and the filters it was last heard through.
#[derive(Clone)]
pub(crate) struct Binaural {
    line: [f32; HISTORY + BLOCK],
    filters: HrirPair,
    /// [`Self::filters`]' taps last first, for a straight dot product.
    reversed: [[f32; HRIR_TAPS]; 2],
    primed: bool,
}

impl Binaural {
    pub(crate) fn new() -> Self {
        Self {
            line: [0.0; HISTORY + BLOCK],
            filters: HrirPair::SILENT,
            reversed: [[0.0; HRIR_TAPS]; 2],
            primed: false,
        }
    }

    /// Forget the input and filters: a new sound starts on the voice.
    pub(crate) fn reset(&mut self) {
        self.line.fill(0.0);
        self.primed = false;
    }

    /// Add `input` (at most [`BLOCK`] frames) heard through `filters` to `out`
    /// (interleaved stereo, as many frames), each ear times `gain`. A change of
    /// filters crosses over the block, so a moving sound never clicks.
    pub(crate) fn process(
        &mut self,
        input: &[f32],
        filters: &HrirPair,
        gain: f32,
        out: &mut [f32],
        scratch: &mut BinauralScratch,
    ) {
        let frames = input.len().min(BLOCK).min(out.len() / 2);
        if frames == 0 {
            return;
        }
        self.line[HISTORY..HISTORY + frames].copy_from_slice(&input[..frames]);
        let mut reversed = filters.taps;
        for ear in &mut reversed {
            ear.reverse();
        }
        if !self.primed {
            self.filters = *filters;
            self.reversed = reversed;
            self.primed = true;
        }
        let changed = self.filters != *filters;
        for ear in 0..2 {
            delay_into(
                &self.line,
                filters.delays[ear],
                frames,
                &mut scratch.delayed,
            );
            let taps = &reversed[ear];
            if changed {
                delay_into(
                    &self.line,
                    self.filters.delays[ear],
                    frames,
                    &mut scratch.previous,
                );
                let old = &self.reversed[ear];
                let step = 1.0 / frames as f32;
                for frame in 0..frames {
                    let new = convolve(&scratch.delayed, taps, frame);
                    let before = convolve(&scratch.previous, old, frame);
                    let t = (frame + 1) as f32 * step;
                    out[frame * 2 + ear] += (before + (new - before) * t) * gain;
                }
            } else {
                for frame in 0..frames {
                    out[frame * 2 + ear] += convolve(&scratch.delayed, taps, frame) * gain;
                }
            }
        }
        self.filters = *filters;
        self.reversed = reversed;
        self.line.copy_within(frames..frames + HISTORY, 0);
    }
}

/// Write into `delayed` the input `delay` samples late, from `HRIR_TAPS - 1`
/// samples before the block to its end, read between samples where it falls.
fn delay_into(line: &[f32], delay: f32, frames: usize, delayed: &mut [f32]) {
    let delay = delay.clamp(0.0, (MAX_DELAY - 1) as f32);
    let whole = delay as usize;
    let fraction = delay - whole as f32;
    let first = HISTORY - (HRIR_TAPS - 1) - whole;
    for (index, value) in delayed[..frames + HRIR_TAPS - 1].iter_mut().enumerate() {
        let at = first + index;
        let (now, before) = (line[at], line[at - 1]);
        *value = now + (before - now) * fraction;
    }
}

/// The filter's output at `frame` of the block from the delayed input; `reversed`
/// is the filter last tap first. Eight running sums, so it vectorizes.
#[inline]
fn convolve(delayed: &[f32], reversed: &[f32; HRIR_TAPS], frame: usize) -> f32 {
    const LANES: usize = 8;
    let window = &delayed[frame..frame + HRIR_TAPS];
    let mut sums = [0.0_f32; LANES];
    for (taps, values) in reversed.chunks_exact(LANES).zip(window.chunks_exact(LANES)) {
        for lane in 0..LANES {
            sums[lane] += taps[lane] * values[lane];
        }
    }
    sums.iter().sum()
}

/// What a voice loses on its way: a gain per band (1 open, less through walls),
/// split at the band edges by two second-order lowpasses (two one-poles each) and
/// gliding between blocks. The bands always add back up to the sound.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BandEq {
    low: [f32; 2],
    high: [f32; 2],
    pub(crate) gains: [f32; BANDS],
}

impl BandEq {
    pub(crate) const OPEN: Self = Self {
        low: [0.0; 2],
        high: [0.0; 2],
        gains: [1.0; BANDS],
    };

    /// Filter `samples` in place, the gains gliding from [`Self::gains`] to `to`
    /// across them. `coefficients` are the lowpasses' at the two band edges.
    pub(crate) fn process(
        &mut self,
        samples: &mut [f32],
        to: [f32; BANDS],
        coefficients: [f32; 2],
    ) {
        let from = self.gains;
        self.gains = to;
        if from == [1.0; BANDS] && to == [1.0; BANDS] {
            // Open both ends: the sound as it is, the filters following it.
            if let Some(last) = samples.last() {
                self.low = [*last; 2];
                self.high = [*last; 2];
            }
            return;
        }
        let count = samples.len().max(1) as f32;
        let steps: [f32; BANDS] = std::array::from_fn(|band| (to[band] - from[band]) / count);
        let mut gains = from;
        for sample in samples {
            self.low[0] += coefficients[0] * (*sample - self.low[0]);
            self.low[1] += coefficients[0] * (self.low[0] - self.low[1]);
            self.high[0] += coefficients[1] * (*sample - self.high[0]);
            self.high[1] += coefficients[1] * (self.high[0] - self.high[1]);
            for band in 0..BANDS {
                gains[band] += steps[band];
            }
            let (below_low, below_high) = (self.low[1], self.high[1]);
            let (low, mid, high) = (below_low, below_high - below_low, *sample - below_high);
            *sample = low * gains[0] + mid * gains[1] + high * gains[2];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hrtf::HrtfSet;

    fn impulse_through(direction: [f32; 3]) -> Vec<f32> {
        let set = HrtfSet::kemar();
        let mut filters = HrirPair::SILENT;
        set.filters(direction, &mut filters);
        let mut voice = Binaural::new();
        let mut scratch = BinauralScratch::new();
        let mut input = [0.0; BLOCK];
        input[0] = 1.0;
        let mut out = vec![0.0; BLOCK * 2];
        voice.process(&input, &filters, 1.0, &mut out, &mut scratch);
        out
    }

    #[test]
    fn an_impulse_from_the_left_reaches_the_left_ear_first() {
        let out = impulse_through([0.0, -1.0, 0.0]);
        let first = |ear: usize| {
            (0..BLOCK)
                .position(|frame| out[frame * 2 + ear].abs() > 0.01)
                .unwrap()
        };
        assert!(first(1) > first(0) + 15, "{} {}", first(0), first(1));
    }

    #[test]
    fn a_held_filter_gives_the_same_as_one_long_block() {
        let set = HrtfSet::kemar();
        let mut filters = HrirPair::SILENT;
        set.filters([0.6, 0.5, 0.2], &mut filters);
        let signal: Vec<f32> = (0..BLOCK * 2)
            .map(|i| ((i * 7919) % 13) as f32 - 6.0)
            .collect();
        let mut scratch = BinauralScratch::new();
        let mut whole = Binaural::new();
        let mut a = vec![0.0; BLOCK * 4];
        whole.process(
            &signal[..BLOCK],
            &filters,
            1.0,
            &mut a[..BLOCK * 2],
            &mut scratch,
        );
        whole.process(
            &signal[BLOCK..],
            &filters,
            1.0,
            &mut a[BLOCK * 2..],
            &mut scratch,
        );
        let mut halves = Binaural::new();
        let mut b = vec![0.0; BLOCK * 4];
        for part in 0..4 {
            let range = part * BLOCK / 2..(part + 1) * BLOCK / 2;
            halves.process(
                &signal[range.clone()],
                &filters,
                1.0,
                &mut b[range.start * 2..range.end * 2],
                &mut scratch,
            );
        }
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-4);
        }
    }

    #[test]
    fn open_bands_leave_the_sound_as_it_is() {
        let mut eq = BandEq::OPEN;
        let mut samples = [0.3, -0.2, 0.9, 0.1];
        eq.process(&mut samples, [1.0; BANDS], [0.07, 0.43]);
        assert_eq!(samples, [0.3, -0.2, 0.9, 0.1]);
    }

    #[test]
    fn a_wall_takes_the_highs_more_than_the_lows() {
        let coefficients = [
            crate::spatial_fx::one_pole_coefficient(500.0, 44_100.0),
            crate::spatial_fx::one_pole_coefficient(4_000.0, 44_100.0),
        ];
        let power = |hz: f32| {
            let mut eq = BandEq::OPEN;
            eq.gains = [0.8, 0.3, 0.05];
            let mut samples: Vec<f32> = (0..8_820)
                .map(|i| (std::f32::consts::TAU * hz * i as f32 / 44_100.0).sin())
                .collect();
            eq.process(&mut samples, [0.8, 0.3, 0.05], coefficients);
            samples[4_410..].iter().map(|s| s * s).sum::<f32>()
        };
        assert!(power(100.0) > 5.0 * power(8_000.0));
    }
}
