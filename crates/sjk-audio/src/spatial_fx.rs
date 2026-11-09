//! The mixer's side of the 3D sound (`s_spatial`): each sound placed in the world
//! heard through the head ([`crate::hrtf`], [`crate::binaural`]), filtered by what is
//! in its way ([`SourceAcoustics`]), and the room ringing from all of them
//! ([`crate::convolver`]).
//!
//! What is in the way and how the room rings are found by `sjk-acoustics` on its own
//! threads and handed over as plain numbers ([`crate::Mixer::set_source_acoustics`])
//! and as a ready response ([`crate::Mixer::set_room_response`]). Everything here is
//! sized when the mixer is built: nothing allocates, frees or locks while mixing.
//! With the effects off ([`crate::Mixer::set_spatial_effects`]) the mixer takes its
//! plain path, sample for sample the output it had before this module existed.

use crate::SourceId;
use crate::binaural::{BLOCK, BandEq, Binaural, BinauralScratch};
use crate::convolver::{Convolver, RoomResponse};
use crate::hrtf::{BAND_EDGES_HZ, BANDS, HrirPair, HrtfSet};

/// Frames the reverb's send holds: a longer render is mixed in parts of this.
pub(crate) const SEND_FRAMES: usize = 4_096;

/// How many sources' acoustics the mixer remembers, for voices that start after
/// their source was measured.
pub(crate) const ACOUSTIC_SOURCES: usize = 128;

/// Time constant of a source's glide to new acoustics: a door shutting muffles in
/// about this long, without a click.
const ACOUSTICS_GLIDE_SECONDS: f32 = 0.05;

/// A sound this close to the head has no direction: both ears alike.
pub(crate) const NEAR_HEAD: f32 = 8.0;

/// What reaches the listener of one source: a gain per band (low, mid, high; 1
/// open, less through walls or round a corner), and where it seems to come from
/// when that is not where it is (the opening it comes round).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceAcoustics {
    pub bands: [f32; BANDS],
    pub via: Option<[f32; 3]>,
}

impl SourceAcoustics {
    pub const OPEN: Self = Self {
        bands: [1.0; BANDS],
        via: None,
    };

    fn clamped(self) -> Self {
        Self {
            bands: self.bands.map(|gain| finite_or(gain, 1.0).clamp(0.0, 1.0)),
            via: self
                .via
                .filter(|point| point.iter().all(|value| value.is_finite())),
        }
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// A voice's or loop's ears and filter, kept apart from the voice tables so voices
/// stay small to copy.
pub(crate) struct Ear {
    binaural: Binaural,
    eq: BandEq,
    /// What it was set up for (the voice's start serial, the loop's sound): another
    /// means a new sound, whose ears start fresh.
    owner: u64,
}

impl Ear {
    fn new() -> Self {
        Self {
            binaural: Binaural::new(),
            eq: BandEq::OPEN,
            owner: u64::MAX,
        }
    }
}

/// How one voice is placed this block.
pub(crate) struct Placement {
    /// The game's loudness at its distance, times its volume, bus and the 16-bit
    /// step.
    pub(crate) gain: f32,
    /// Left and right gains of the old mixer's panning (speakers), the same loudness.
    pub(crate) pan: [f32; 2],
    /// The direction it is heard from, in the listener's frame (forward, right, up);
    /// `None` when it is at the head.
    pub(crate) direction: Option<[f32; 3]>,
    pub(crate) bands: [f32; BANDS],
    /// Whether the room rings from it.
    pub(crate) send: bool,
}

/// The mixer's 3D sound state, built once with the mixer.
pub(crate) struct SpatialEffects {
    pub(crate) enabled: bool,
    /// Heard through the head (headphones) or panned (speakers).
    pub(crate) headphones: bool,
    sources: Box<[(SourceId, SourceAcoustics)]>,
    sources_used: usize,
    next_replaced: usize,
    hrtf: HrtfSet,
    coefficients: [f32; 2],
    sample_rate: f32,
    /// The mono mix of the world-placed voices and loops of the block, the room's
    /// input; the mixer fills it, so menu, announcer and listener-relative sounds
    /// stay dry.
    pub(crate) send: Box<[f32]>,
    convolver: Convolver,
    reverb_gain: f32,
    voice_ears: Box<[Ear]>,
    loop_ears: Box<[Ear]>,
    mono: Box<[f32]>,
    scratch: Box<BinauralScratch>,
    filters: HrirPair,
}

impl SpatialEffects {
    pub(crate) fn new(sample_rate: u32, voices: usize, loops: usize) -> Self {
        let rate = sample_rate.max(1) as f32;
        Self {
            enabled: false,
            headphones: true,
            sources: vec![(SourceId(0), SourceAcoustics::OPEN); ACOUSTIC_SOURCES]
                .into_boxed_slice(),
            sources_used: 0,
            next_replaced: 0,
            hrtf: HrtfSet::kemar(),
            coefficients: BAND_EDGES_HZ.map(|hz| one_pole_coefficient(hz, rate)),
            sample_rate: rate,
            send: vec![0.0; SEND_FRAMES].into_boxed_slice(),
            convolver: Convolver::new(),
            reverb_gain: 1.0,
            voice_ears: (0..voices).map(|_| Ear::new()).collect(),
            loop_ears: (0..loops).map(|_| Ear::new()).collect(),
            mono: vec![0.0; BLOCK].into_boxed_slice(),
            scratch: Box::new(BinauralScratch::new()),
            filters: HrirPair::SILENT,
        }
    }

    /// The acoustics last measured for `source`, open when it never was.
    pub(crate) fn acoustics_of(&self, source: SourceId) -> SourceAcoustics {
        self.sources[..self.sources_used]
            .iter()
            .find(|(known, _)| *known == source)
            .map_or(SourceAcoustics::OPEN, |(_, acoustics)| *acoustics)
    }

    /// Remember `source`'s acoustics; a full table gives up its oldest entry.
    pub(crate) fn remember(&mut self, source: SourceId, acoustics: SourceAcoustics) {
        let acoustics = acoustics.clamped();
        if let Some(entry) = self.sources[..self.sources_used]
            .iter_mut()
            .find(|(known, _)| *known == source)
        {
            entry.1 = acoustics;
            return;
        }
        if self.sources_used < self.sources.len() {
            self.sources[self.sources_used] = (source, acoustics);
            self.sources_used += 1;
            return;
        }
        self.sources[self.next_replaced] = (source, acoustics);
        self.next_replaced = (self.next_replaced + 1) % self.sources.len();
    }

    /// Forget every source's acoustics and silence the room's ring.
    pub(crate) fn clear(&mut self) {
        self.sources_used = 0;
        self.next_replaced = 0;
        self.convolver.silence();
    }

    pub(crate) fn set_room_response(&mut self, response: Option<Box<RoomResponse>>) {
        self.convolver.set_response(response);
    }

    pub(crate) fn take_retired_response(&mut self) -> Option<Box<RoomResponse>> {
        self.convolver.take_retired()
    }

    pub(crate) fn set_reverb_gain(&mut self, gain: f32) {
        self.reverb_gain = finite_or(gain, 1.0).clamp(0.0, 4.0);
    }

    /// Mix one voice: `next` gives its samples (to be scaled by `placement`'s gains),
    /// `None` once it ended. Adds it to `out` (interleaved stereo) and to the room's
    /// send; whether it still plays. `owner` names the sound on that voice or loop
    /// (another than last time starts its ears fresh).
    pub(crate) fn mix(
        &mut self,
        ear: EarRef,
        owner: u64,
        placement: &Placement,
        out: &mut [f32],
        mut next: impl FnMut() -> Option<f32>,
    ) -> bool {
        let through_head = self.headphones && placement.direction.is_some();
        if let (true, Some(direction)) = (through_head, placement.direction) {
            self.hrtf.filters(direction, &mut self.filters);
        }
        let ear = match ear {
            EarRef::Voice(index) => &mut self.voice_ears[index],
            EarRef::Loop(index) => &mut self.loop_ears[index],
        };
        if ear.owner != owner {
            ear.owner = owner;
            ear.binaural.reset();
            ear.eq = BandEq::OPEN;
            ear.eq.gains = placement.bands;
        }
        let frames = out.len() / 2;
        let mut active = true;
        let mut offset = 0;
        while offset < frames && active {
            let count = (frames - offset).min(BLOCK);
            let mono = &mut self.mono[..count];
            for sample in mono.iter_mut() {
                *sample = if active {
                    next().unwrap_or_else(|| {
                        active = false;
                        0.0
                    })
                } else {
                    0.0
                };
            }
            let step = glide_fraction(count, self.sample_rate, ACOUSTICS_GLIDE_SECONDS);
            let from = ear.eq.gains;
            let to: [f32; BANDS] = std::array::from_fn(|band| {
                from[band] + (placement.bands[band] - from[band]) * step
            });
            ear.eq.process(mono, to, self.coefficients);
            if placement.send {
                for (send, sample) in self.send[offset..offset + count]
                    .iter_mut()
                    .zip(mono.iter())
                {
                    *send += sample * placement.gain;
                }
            }
            let part = &mut out[offset * 2..(offset + count) * 2];
            if through_head {
                ear.binaural
                    .process(mono, &self.filters, placement.gain, part, &mut self.scratch);
            } else {
                for (frame, sample) in part.chunks_exact_mut(2).zip(mono.iter()) {
                    frame[0] += sample * placement.pan[0];
                    frame[1] += sample * placement.pan[1];
                }
            }
            offset += count;
        }
        active
    }

    /// Add the room's ring of the block's send ([`Self::send`]) to `out`
    /// (interleaved stereo, at most [`SEND_FRAMES`] frames).
    pub(crate) fn apply_reverb(&mut self, out: &mut [f32]) {
        let frames = (out.len() / 2).min(SEND_FRAMES);
        self.convolver.process(
            &self.send[..frames],
            &mut out[..frames * 2],
            self.reverb_gain,
        );
    }
}

/// Which ear state a mix uses.
#[derive(Clone, Copy, Debug)]
pub(crate) enum EarRef {
    Voice(usize),
    Loop(usize),
}

/// The fraction of the distance a first-order glide with time constant `seconds`
/// covers in `frames` at `rate`.
fn glide_fraction(frames: usize, rate: f32, seconds: f32) -> f32 {
    1.0 - (-(frames as f32) / (rate * seconds)).exp()
}

/// Coefficient `a` of the one-pole lowpass `y += a * (x - y)` with cutoff `hz`.
pub(crate) fn one_pole_coefficient(hz: f32, rate: f32) -> f32 {
    1.0 - (-std::f32::consts::TAU * hz / rate).exp()
}
