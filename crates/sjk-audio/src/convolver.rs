//! The room's ring in the 3D sound: the mono mix of the sounds placed in the world
//! convolved with the room's response for each ear ([`RoomResponse`], traced by
//! `sjk-acoustics` off the audio thread). Uniformly partitioned convolution in the
//! frequency domain, [`PARTITION`] frames a block (2.9 ms at 44.1 kHz of delay); a
//! new response crosses over from the old in [`CROSSFADE_BLOCKS`] blocks. The audio
//! thread only multiplies and adds what it was handed: a response is built, and
//! freed, elsewhere ([`crate::Mixer::take_retired_response`]).

use crate::fft::{Complex, Fft};

/// Frames of a block and of each part of a response.
pub const PARTITION: usize = 128;
const FFT_SIZE: usize = PARTITION * 2;
const BINS: usize = PARTITION + 1;

/// The longest response, in seconds at 44.1 kHz.
pub const MAX_RESPONSE_SECONDS: f32 = 3.0;

/// Parts of the longest response.
pub const MAX_PARTITIONS: usize = (MAX_RESPONSE_SECONDS * 44_100.0) as usize / PARTITION + 1;

/// Blocks a new response takes to replace the old (about 46 ms).
const CROSSFADE_BLOCKS: usize = 16;

/// Responses waiting to be handed back for freeing.
const RETIRED: usize = 4;

/// A room's response for both ears, cut into [`PARTITION`]-frame parts and
/// transformed: what [`Convolver`] multiplies the sounds with.
#[derive(Clone, Debug)]
pub struct RoomResponse {
    partitions: usize,
    /// Each ear's parts' spectra, [`PARTITION`] + 1 bins a part, real and imaginary
    /// parts apart (so the products vectorize).
    re: [Box<[f32]>; 2],
    im: [Box<[f32]>; 2],
}

impl RoomResponse {
    /// Transform each ear's impulse response (samples at the mixer's rate, at most
    /// [`MAX_RESPONSE_SECONDS`] long; longer is cut). Allocates: build it off the
    /// audio thread.
    pub fn from_impulses(left: &[f32], right: &[f32]) -> Self {
        let length = left.len().max(right.len()).min(MAX_PARTITIONS * PARTITION);
        let partitions = length.div_ceil(PARTITION).max(1);
        let fft = Fft::new(FFT_SIZE);
        let mut buffer = vec![Complex::ZERO; FFT_SIZE];
        let spectra = [left, right].map(|ear| {
            let mut re = vec![0.0; partitions * BINS].into_boxed_slice();
            let mut im = vec![0.0; partitions * BINS].into_boxed_slice();
            for part in 0..partitions {
                buffer.fill(Complex::ZERO);
                let start = part * PARTITION;
                let end = (start + PARTITION).min(ear.len()).min(length);
                for (slot, sample) in buffer.iter_mut().zip(ear.get(start..end).unwrap_or(&[])) {
                    slot.re = *sample;
                }
                fft.forward(&mut buffer);
                for (bin, value) in buffer[..BINS].iter().enumerate() {
                    re[part * BINS + bin] = value.re;
                    im[part * BINS + bin] = value.im;
                }
            }
            (re, im)
        });
        let [(left_re, left_im), (right_re, right_im)] = spectra;
        Self {
            partitions,
            re: [left_re, right_re],
            im: [left_im, right_im],
        }
    }

    /// Parts of the response.
    pub fn partitions(&self) -> usize {
        self.partitions
    }
}

/// The convolution's state, built once with the mixer.
pub(crate) struct Convolver {
    fft: Fft,
    /// The last two blocks of input, the transform's window.
    input: Box<[f32]>,
    /// Interleaved stereo output of the last block, read out during the next.
    output: Box<[f32]>,
    fill: usize,
    /// Spectra of the last [`MAX_PARTITIONS`] input blocks, a ring (real and
    /// imaginary parts apart).
    history_re: Box<[f32]>,
    history_im: Box<[f32]>,
    head: usize,
    current: Option<Box<RoomResponse>>,
    fading: Option<Box<RoomResponse>>,
    fade_block: usize,
    retired: [Option<Box<RoomResponse>>; RETIRED],
    /// Blocks of silent input in a row: once the response has rung out, a block
    /// costs nothing.
    quiet_blocks: usize,
    buffer: Box<[Complex]>,
    sums: Sums,
    previous_sums: Sums,
}

/// Each ear's sum of products of a block, real and imaginary parts apart.
struct Sums {
    re: [[f32; BINS]; 2],
    im: [[f32; BINS]; 2],
}

impl Sums {
    fn new() -> Self {
        Self {
            re: [[0.0; BINS]; 2],
            im: [[0.0; BINS]; 2],
        }
    }
}

impl Convolver {
    pub(crate) fn new() -> Self {
        Self {
            fft: Fft::new(FFT_SIZE),
            input: vec![0.0; FFT_SIZE].into_boxed_slice(),
            output: vec![0.0; PARTITION * 2].into_boxed_slice(),
            fill: 0,
            history_re: vec![0.0; MAX_PARTITIONS * BINS].into_boxed_slice(),
            history_im: vec![0.0; MAX_PARTITIONS * BINS].into_boxed_slice(),
            head: 0,
            current: None,
            fading: None,
            fade_block: 0,
            retired: [None, None, None, None],
            quiet_blocks: usize::MAX / 2,
            buffer: vec![Complex::ZERO; FFT_SIZE].into_boxed_slice(),
            sums: Sums::new(),
            previous_sums: Sums::new(),
        }
    }

    /// Ring with `response` from now on (crossing over from the one before), or
    /// with none. A response that was still fading out is retired at once.
    pub(crate) fn set_response(&mut self, response: Option<Box<RoomResponse>>) {
        if let Some(old) = self.fading.take() {
            self.retire(old);
        }
        self.fading = self.current.take();
        self.fade_block = 0;
        self.current = response;
    }

    /// A response the convolver no longer uses, for the caller to free.
    pub(crate) fn take_retired(&mut self) -> Option<Box<RoomResponse>> {
        self.retired.iter_mut().find_map(Option::take)
    }

    fn retire(&mut self, response: Box<RoomResponse>) {
        if let Some(slot) = self.retired.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(response);
        } else {
            // Nobody collects them (a test, or no output thread): the oldest goes.
            self.retired[0] = Some(response);
        }
    }

    /// Stop ringing: the input history and the output are silenced.
    pub(crate) fn silence(&mut self) {
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.history_re.fill(0.0);
        self.history_im.fill(0.0);
        self.fill = 0;
        self.quiet_blocks = usize::MAX / 2;
    }

    /// Add the room's ring of `send` (mono, one sample a frame) to `out`
    /// (interleaved stereo), each ear times `gain`.
    pub(crate) fn process(&mut self, send: &[f32], out: &mut [f32], gain: f32) {
        for (frame, sample) in out.chunks_exact_mut(2).zip(send) {
            frame[0] += self.output[self.fill * 2] * gain;
            frame[1] += self.output[self.fill * 2 + 1] * gain;
            self.input[PARTITION + self.fill] = *sample;
            self.fill += 1;
            if self.fill == PARTITION {
                self.block();
                self.fill = 0;
            }
        }
    }

    fn block(&mut self) {
        let silent = self.input[PARTITION..].iter().all(|sample| *sample == 0.0);
        self.quiet_blocks = if silent {
            self.quiet_blocks.saturating_add(1)
        } else {
            0
        };
        self.head = (self.head + 1) % MAX_PARTITIONS;
        let slot = self.head * BINS;
        if silent {
            self.history_re[slot..slot + BINS].fill(0.0);
            self.history_im[slot..slot + BINS].fill(0.0);
        } else {
            for (value, sample) in self.buffer.iter_mut().zip(self.input.iter()) {
                *value = Complex::new(*sample, 0.0);
            }
            self.fft.forward(&mut self.buffer);
            for (bin, value) in self.buffer[..BINS].iter().enumerate() {
                self.history_re[slot + bin] = value.re;
                self.history_im[slot + bin] = value.im;
            }
        }
        self.input.copy_within(PARTITION.., 0);
        let longest = self
            .current
            .iter()
            .chain(self.fading.iter())
            .map(|response| response.partitions)
            .max()
            .unwrap_or(0);
        if self.quiet_blocks > longest + 1 {
            self.output.fill(0.0);
            self.finish_fade();
            return;
        }
        let history = (&self.history_re[..], &self.history_im[..]);
        let has_current = accumulate(history, self.head, self.current.as_deref(), &mut self.sums);
        let has_fading = accumulate(
            history,
            self.head,
            self.fading.as_deref(),
            &mut self.previous_sums,
        );
        self.inverse(has_current, has_fading);
        self.finish_fade();
    }

    /// Both ears' sums back to samples (one transform: left real, right imaginary),
    /// the old response's crossing over to the new one's.
    fn inverse(&mut self, has_current: bool, has_fading: bool) {
        let fade = has_fading.then(|| {
            let start = self.fade_block as f32 / CROSSFADE_BLOCKS as f32;
            let end = (self.fade_block + 1) as f32 / CROSSFADE_BLOCKS as f32;
            (start, end)
        });
        let ears = |sums: &Sums, buffer: &mut [Complex], fft: &Fft| {
            pack(sums, buffer);
            fft.inverse(buffer);
        };
        if has_current {
            ears(&self.sums, &mut self.buffer, &self.fft);
        } else {
            self.buffer.fill(Complex::ZERO);
        }
        match fade {
            None => {
                for (frame, value) in self
                    .output
                    .chunks_exact_mut(2)
                    .zip(&self.buffer[PARTITION..])
                {
                    frame[0] = value.re;
                    frame[1] = value.im;
                }
            }
            Some((start, end)) => {
                // The new response's block into the output first, then the old one's
                // weighed in: the buffer is reused for it.
                for (frame, value) in self
                    .output
                    .chunks_exact_mut(2)
                    .zip(&self.buffer[PARTITION..])
                {
                    frame[0] = value.re;
                    frame[1] = value.im;
                }
                ears(&self.previous_sums, &mut self.buffer, &self.fft);
                let step = (end - start) / PARTITION as f32;
                for (index, (frame, old)) in self
                    .output
                    .chunks_exact_mut(2)
                    .zip(&self.buffer[PARTITION..])
                    .enumerate()
                {
                    let t = start + step * (index + 1) as f32;
                    frame[0] = old.re + (frame[0] - old.re) * t;
                    frame[1] = old.im + (frame[1] - old.im) * t;
                }
            }
        }
    }

    fn finish_fade(&mut self) {
        if self.fading.is_none() {
            return;
        }
        self.fade_block += 1;
        if self.fade_block >= CROSSFADE_BLOCKS
            && let Some(old) = self.fading.take()
        {
            self.retire(old);
        }
    }
}

/// Sum the input history times `response`'s parts into each ear's `sums`; whether
/// there was a response.
fn accumulate(
    (history_re, history_im): (&[f32], &[f32]),
    head: usize,
    response: Option<&RoomResponse>,
    sums: &mut Sums,
) -> bool {
    let Some(response) = response else {
        return false;
    };
    *sums = Sums::new();
    for part in 0..response.partitions {
        let block = (head + MAX_PARTITIONS - part) % MAX_PARTITIONS;
        let xr = &history_re[block * BINS..][..BINS];
        let xi = &history_im[block * BINS..][..BINS];
        for ear in 0..2 {
            let hr = &response.re[ear][part * BINS..][..BINS];
            let hi = &response.im[ear][part * BINS..][..BINS];
            let (sr, si) = (&mut sums.re[ear], &mut sums.im[ear]);
            for bin in 0..BINS {
                let (a, b) = (xr[bin], xi[bin]);
                sr[bin] += a * hr[bin] - b * hi[bin];
                si[bin] += a * hi[bin] + b * hr[bin];
            }
        }
    }
    true
}

/// Two real signals' half spectra into one full spectrum whose inverse has the
/// left in its real part and the right in its imaginary part.
fn pack(sums: &Sums, buffer: &mut [Complex]) {
    for bin in 0..BINS {
        let (l, r) = (
            Complex::new(sums.re[0][bin], sums.im[0][bin]),
            Complex::new(sums.re[1][bin], sums.im[1][bin]),
        );
        buffer[bin] = Complex::new(l.re - r.im, l.im + r.re);
        if bin > 0 && bin < PARTITION {
            buffer[FFT_SIZE - bin] = Complex::new(l.re + r.im, r.re - l.im);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(convolver: &mut Convolver, input: &[f32]) -> Vec<f32> {
        let mut out = vec![0.0; input.len() * 2];
        convolver.process(input, &mut out, 1.0);
        out
    }

    #[test]
    fn convolving_matches_the_direct_sum() {
        let left: Vec<f32> = (0..700)
            .map(|i| ((i * 31) % 17) as f32 / 17.0 - 0.5)
            .collect();
        let right: Vec<f32> = (0..300).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
        let mut convolver = Convolver::new();
        convolver.set_response(Some(Box::new(RoomResponse::from_impulses(&left, &right))));
        // Past the first crossover from silence.
        let warmup = vec![0.0; PARTITION * (CROSSFADE_BLOCKS + 1)];
        run(&mut convolver, &warmup);
        let input: Vec<f32> = (0..1_500)
            .map(|i| ((i * 7) % 11) as f32 / 11.0 - 0.5)
            .collect();
        let out = run(&mut convolver, &input);
        for frame in PARTITION..input.len() {
            // One block late.
            let at = frame - PARTITION;
            let expect = |ir: &[f32]| -> f32 {
                (0..=at)
                    .filter(|k| at - k < ir.len())
                    .map(|k| input[k] * ir[at - k])
                    .sum()
            };
            assert!(
                (out[frame * 2] - expect(&left)).abs() < 2e-3,
                "left {frame}"
            );
            assert!(
                (out[frame * 2 + 1] - expect(&right)).abs() < 2e-3,
                "right {frame}"
            );
        }
    }

    #[test]
    fn a_new_response_crosses_over_without_a_step() {
        let mut convolver = Convolver::new();
        let one = |gain: f32| Box::new(RoomResponse::from_impulses(&[gain], &[gain]));
        convolver.set_response(Some(one(1.0)));
        let steady = vec![0.5; PARTITION * 40];
        run(&mut convolver, &steady);
        convolver.set_response(Some(one(0.0)));
        let out = run(&mut convolver, &steady);
        for pair in out.chunks_exact(2).collect::<Vec<_>>().windows(2) {
            assert!((pair[0][0] - pair[1][0]).abs() < 0.01);
        }
        assert!(out[out.len() - 2].abs() < 1e-4);
        assert!(convolver.take_retired().is_some());
    }

    #[test]
    fn silence_after_the_ring_costs_no_work_and_gives_zeros() {
        let mut convolver = Convolver::new();
        convolver.set_response(Some(Box::new(RoomResponse::from_impulses(
            &[1.0; 300],
            &[1.0; 300],
        ))));
        run(&mut convolver, &[1.0; PARTITION]);
        let out = run(&mut convolver, &vec![0.0; PARTITION * 8]);
        assert!(
            out[PARTITION * 2..PARTITION * 6]
                .iter()
                .any(|s| s.abs() > 0.1)
        );
        assert!(out[PARTITION * 12..].iter().all(|s| *s == 0.0));
    }
}
