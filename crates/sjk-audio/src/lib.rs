//! Engine-generic decoded-sound storage, spatialization, and stereo mixing.
//!
//! Asset registration is deliberately separated from playback: decoding and
//! resampling may allocate, while [`Mixer::play`], [`Mixer::set_loop`], and
//! [`Mixer::render`] operate entirely inside fixed-capacity storage.
//!
//! Decoded sounds are kept as 16-bit mono PCM at the mixer's rate, half the
//! memory of floats; the mixer converts each sample as it reads it. A sound
//! can be released ([`Mixer::release_sound`]): its PCM is handed back to the
//! caller, who frees it off the audio thread.

use std::error::Error;
use std::fmt;
use std::io::Cursor;

mod binaural;
mod convolver;
pub mod fft;
pub mod hrtf;
mod id3;
mod spatial_fx;

pub use convolver::{MAX_RESPONSE_SECONDS, PARTITION, RoomResponse};
pub use spatial_fx::SourceAcoustics;

/// Opaque index of decoded PCM in a [`SoundBank`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SoundHandle(pub u32);

/// Stable identity of the entity or subsystem producing a voice.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceId(pub u32);

/// Logical channel used to replace an earlier voice from the same source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChannelId(pub u32);

/// Distance behavior supplied by the compatibility or game adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Attenuation {
    /// Listener-relative or global sound with equal full-volume channels.
    None,
    /// Full volume through `full_volume_distance`, then linear falloff.
    Linear {
        full_volume_distance: f32,
        falloff_per_unit: f32,
    },
}

/// One allocation-free voice start request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayRequest {
    pub origin: Option<[f32; 3]>,
    pub source: SourceId,
    pub channel: ChannelId,
    pub volume: f32,
    pub attenuation: Attenuation,
}

/// Mixer capacity and output format.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerConfig {
    pub voices: usize,
    /// Independent per-frame loop capacity.
    pub loops: usize,
    pub sample_rate: u32,
    /// Attenuation applied to per-frame looping sources; game adapters supply
    /// their own falloff parameters (the engine ships no game-specific preset).
    pub loop_attenuation: Attenuation,
    /// Caller-supplied loop pitch policy. Compatibility adapters own all
    /// game-specific values; the mixer only evaluates this generic ratio.
    pub doppler: DopplerConfig,
    /// Channels whose voices are stolen last when the table is full: long
    /// sounds (an emote's music, a boombox track) are always the oldest, so the
    /// plain oldest-first steal would cut them first. Their voices playing the same
    /// sound at the same sample on one channel (synced emotes,
    /// [`Mixer::play_aligned`]) are mixed once, each ear as loud as the loudest of
    /// them, not their sum.
    pub protected_channels: &'static [ChannelId],
}

/// Whether `other` plays the very sample `voice` plays, on its channel: a synced
/// group mixed as one.
fn same_sample(other: &Voice, voice: &Voice) -> bool {
    other.active
        && other.handle == voice.handle
        && other.cursor == voice.cursor
        && other.request.channel == voice.request.channel
}

/// Allocation-free distance-ratio Doppler policy for looping voices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DopplerConfig {
    /// Runtime gate (the legacy adapter maps this from its archived cvar).
    pub enabled: bool,
    /// Divisor applied to the squared-distance ratio.
    pub distance_ratio_divisor: f32,
    /// Upper pitch/resampling clamp.
    pub maximum_scale: f32,
}

impl DopplerConfig {
    /// A neutral engine-default policy; games opt in with explicit values.
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            distance_ratio_divisor: 1.0,
            maximum_scale: 1.0,
        }
    }
}

/// What one stored 16-bit step is worth in the mixer's [-1, 1] float range.
const SAMPLE_STEP: f32 = 1.0 / 32_768.0;

/// Store a [-1, 1] float sample in 16 bits: rounded to the nearest step and
/// clamped, so a 16-bit source read as `value / 32768` comes back exactly.
pub fn quantize_sample(sample: f32) -> i16 {
    (sample * 32_768.0).round().clamp(-32_768.0, 32_767.0) as i16
}

/// The [-1, 1] float the mixer hears for a stored 16-bit sample.
pub fn sample_value(sample: i16) -> f32 {
    f32::from(sample) * SAMPLE_STEP
}

#[derive(Clone, Debug)]
struct Sound {
    samples: Box<[i16]>,
}

/// Decoded 16-bit mono PCM ready to install into a mixer-rate [`SoundBank`].
///
/// Decoding and resampling may allocate. Moving this value into a bank does
/// not, provided the bank was created with sufficient reserved capacity.
#[derive(Clone, Debug)]
pub struct DecodedSound {
    samples: Box<[i16]>,
}

impl DecodedSound {
    /// One silent sample used when an asynchronous asset decoder rejects data.
    pub fn silence() -> Self {
        Self {
            samples: Box::new([0]),
        }
    }

    /// Float mono PCM already at the mixer's rate, such as a sound made from
    /// another one ([`decode_mono`]); stored at 16 bits ([`quantize_sample`]).
    pub fn from_samples(samples: Vec<f32>) -> Self {
        Self {
            samples: samples.into_iter().map(quantize_sample).collect(),
        }
    }

    /// 16-bit mono PCM already at the mixer's rate.
    pub fn from_pcm16(samples: Vec<i16>) -> Self {
        Self {
            samples: samples.into_boxed_slice(),
        }
    }

    /// The decoded 16-bit mono samples ([`sample_value`] gives each one's float).
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Bytes of PCM this sound keeps in memory.
    pub fn byte_len(&self) -> usize {
        self.samples.len() * std::mem::size_of::<i16>()
    }
}

/// Decode WAV or MP3 bytes and resample them before the audio thread sees them.
pub fn decode_encoded(
    bytes: &[u8],
    extension: &str,
    target_sample_rate: u32,
) -> Result<DecodedSound, DecodeError> {
    let (samples, source_rate) = decode_pcm16(bytes, extension)?;
    let samples = if source_rate == target_sample_rate {
        samples
    } else {
        resample(
            &samples,
            source_rate,
            target_sample_rate,
            sample_value,
            quantize_sample,
        )
    };
    Ok(DecodedSound::from_pcm16(samples))
}

/// Decode WAV or MP3 bytes to float mono PCM at `target_sample_rate`, for a
/// caller that reworks the samples before registering them
/// ([`DecodedSound::from_samples`]).
pub fn decode_mono(
    bytes: &[u8],
    extension: &str,
    target_sample_rate: u32,
) -> Result<Vec<f32>, DecodeError> {
    let (samples, source_rate) = decode_pcm16(bytes, extension)?;
    Ok(resample(
        &samples,
        source_rate,
        target_sample_rate,
        sample_value,
        std::convert::identity,
    ))
}

/// Decoded mono PCM, resampled to one mixer rate at registration time.
///
/// Handles are indices assigned in registration order and never reused; a
/// released sound leaves its slot empty, so later handles keep their meaning.
#[derive(Default)]
pub struct SoundBank {
    sample_rate: u32,
    sounds: Vec<Option<Sound>>,
    /// Bytes of PCM held by the registered, unreleased sounds.
    sample_bytes: usize,
}

impl SoundBank {
    /// Construct an empty bank targeting `sample_rate`.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            sounds: Vec::with_capacity(2_048),
            sample_bytes: 0,
        }
    }

    /// Register mono PCM. Resampling occurs now, never from `play`/`render`.
    pub fn register_pcm(&mut self, samples: &[f32], sample_rate: u32) -> SoundHandle {
        let samples = resample(
            samples,
            sample_rate,
            self.sample_rate,
            std::convert::identity,
            quantize_sample,
        );
        self.register_decoded(DecodedSound::from_pcm16(samples))
    }

    /// Decode PCM WAV or MP3 bytes once and register their mono mixdown.
    pub fn register_encoded(
        &mut self,
        bytes: &[u8],
        extension: &str,
    ) -> Result<SoundHandle, DecodeError> {
        let decoded = decode_encoded(bytes, extension, self.sample_rate)?;
        Ok(self.register_decoded(decoded))
    }

    /// Install already-decoded mixer-rate PCM without decoding in the mixer.
    pub fn register_decoded(&mut self, decoded: DecodedSound) -> SoundHandle {
        let handle = SoundHandle(u32::try_from(self.sounds.len()).unwrap_or(u32::MAX));
        self.sample_bytes += decoded.byte_len();
        self.sounds.push(Some(Sound {
            samples: decoded.samples,
        }));
        handle
    }

    /// Take a sound's PCM out of the bank without freeing it: the caller drops
    /// it where freeing is allowed. The handle then names no sound. Prefer
    /// [`Mixer::release_sound`], which also stops what plays it.
    pub fn release(&mut self, handle: SoundHandle) -> Option<DecodedSound> {
        let sound = self.sounds.get_mut(handle.0 as usize)?.take()?;
        let decoded = DecodedSound {
            samples: sound.samples,
        };
        self.sample_bytes -= decoded.byte_len();
        Some(decoded)
    }

    fn get(&self, handle: SoundHandle) -> Option<&Sound> {
        self.sounds.get(handle.0 as usize)?.as_ref()
    }

    /// Number of handles registered, released ones included.
    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    /// Whether no sounds have been registered.
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// Mixer-rate mono sample count for a registered, unreleased sound.
    pub fn sample_count(&self, handle: SoundHandle) -> Option<usize> {
        self.get(handle).map(|sound| sound.samples.len())
    }

    /// Bytes of PCM the bank holds now (released sounds excluded).
    pub const fn sample_bytes(&self) -> usize {
        self.sample_bytes
    }

    /// Target sample rate shared by every registered sound.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Decode or format error produced only during sound registration.
#[derive(Debug)]
pub struct DecodeError(String);

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DecodeError {}

/// Listener data used by spatial models.
#[derive(Clone, Copy, Debug)]
pub struct Listener {
    pub origin: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    /// World-space velocity supplied independently from the view axes.
    pub velocity: [f32; 3],
}

/// Pluggable conversion from a source position to stereo gains.
pub trait SpatialModel: Send {
    /// Update separation for spatial models that expose that parameter.
    fn set_separation(&mut self, _separation: f32) {}
    fn gains(&self, listener: &Listener, origin: [f32; 3], attenuation: Attenuation) -> [f32; 2];
}

/// Quake/JKA's linear attenuation and stereo-separation calculation.
///
/// The default parameters reproduce `snd_dma.cpp:1345-1422`: full-volume
/// distance 256, `SOUND_ATTENUATE` 0.0008, and the requested symmetric 0.5
/// separation form. Voice-channel distance offsets remain adapter-selected by
/// supplying a different [`Attenuation::Linear`] full-volume distance.
#[derive(Clone, Copy, Debug)]
pub struct LegacyQuakeSpatial {
    separation: f32,
}

impl LegacyQuakeSpatial {
    pub fn new(separation: f32) -> Self {
        Self { separation }
    }
}

impl Default for LegacyQuakeSpatial {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl SpatialModel for LegacyQuakeSpatial {
    fn set_separation(&mut self, separation: f32) {
        self.separation = separation;
    }
    fn gains(&self, listener: &Listener, origin: [f32; 3], attenuation: Attenuation) -> [f32; 2] {
        if attenuation == Attenuation::None {
            return [1.0; 2];
        }
        let delta = sub(origin, listener.origin);
        let distance = length(delta);
        let direction = if distance > f32::EPSILON {
            scale(delta, 1.0 / distance)
        } else {
            [0.0; 3]
        };
        let dot = dot(listener.right, direction).clamp(-1.0, 1.0);
        let right = (self.separation + (1.0 - self.separation) * dot).max(0.0);
        let left = (self.separation - (1.0 - self.separation) * dot).max(0.0);
        let distance_gain = linear_falloff(distance, attenuation);
        [left * distance_gain, right * distance_gain]
    }
}

/// The game's loudness of a sound at `distance` (Quake's linear falloff), 1 with no
/// attenuation.
fn linear_falloff(distance: f32, attenuation: Attenuation) -> f32 {
    let Attenuation::Linear {
        full_volume_distance,
        falloff_per_unit,
    } = attenuation
    else {
        return 1.0;
    };
    let normalized = ((distance - full_volume_distance).max(0.0) * falloff_per_unit).max(0.0);
    (1.0 - normalized).max(0.0)
}

/// [`linear_falloff`] of a sound at `origin` from the listener.
fn distance_gain(listener: &Listener, origin: [f32; 3], attenuation: Attenuation) -> f32 {
    linear_falloff(length(sub(origin, listener.origin)), attenuation)
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    active: bool,
    handle: SoundHandle,
    request: PlayRequest,
    cursor: usize,
    started: u64,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            handle: SoundHandle(0),
            request: PlayRequest {
                origin: None,
                source: SourceId(0),
                channel: ChannelId(0),
                volume: 0.0,
                attenuation: Attenuation::None,
            },
            cursor: 0,
            started: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LoopVoice {
    active: bool,
    handle: SoundHandle,
    request: PlayRequest,
    velocity: [f32; 3],
    generation: u64,
    cursor: f64,
}

impl Default for LoopVoice {
    fn default() -> Self {
        Self {
            active: false,
            handle: SoundHandle(0),
            request: PlayRequest {
                origin: None,
                source: SourceId(0),
                channel: ChannelId(0),
                volume: 0.0,
                attenuation: Attenuation::None,
            },
            velocity: [0.0; 3],
            generation: 0,
            cursor: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BackgroundTrack {
    intro: SoundHandle,
    repeating: SoundHandle,
    cursor: usize,
    in_intro: bool,
    volume: f32,
}

/// Read-only position of the engine-generic intro/repeating music sequencer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackgroundTrackState {
    /// True until the intro's final sample has been consumed.
    pub in_intro: bool,
    /// Mono sample cursor within the currently selected track.
    pub cursor: usize,
}

/// Fixed-capacity software voice mixer.
pub struct Mixer {
    bank: SoundBank,
    voices: Box<[Voice]>,
    loops: Box<[LoopVoice]>,
    listener: Listener,
    spatial: Box<dyn SpatialModel>,
    serial: u64,
    loop_generation: u64,
    /// True between [`Mixer::clear_loops`] and [`Mixer::commit_loops`].
    loop_frame_open: bool,
    loop_attenuation: Attenuation,
    listener_source: Option<SourceId>,
    rendered_frames: u64,
    master_gain: f32,
    /// Channels whose voices take their own gain in place of the master gain
    /// ([`Mixer::set_channel_gain`]): a handful at most.
    channel_gains: Vec<(ChannelId, f32)>,
    music_gain: f32,
    background: Option<BackgroundTrack>,
    doppler: DopplerConfig,
    protected_channels: &'static [ChannelId],
    /// Opt-in 3D sound ([`Self::set_spatial_effects`]); off, the mix
    /// is the plain one.
    effects: spatial_fx::SpatialEffects,
}

/// The loudest a channel's bus ([`Mixer::set_channel_gain`]) plays its sounds.
pub const MAX_CHANNEL_GAIN: f32 = 4.0;

impl Mixer {
    /// Whether `channel` is one of [`MixerConfig::protected_channels`].
    fn protected(&self, channel: ChannelId) -> bool {
        self.protected_channels.contains(&channel)
    }

    /// Adjust the existing spatializer without replacing or allocating it.
    pub fn set_separation(&mut self, separation: f32) {
        self.spatial.set_separation(separation);
    }
    /// Construct with the legacy-compatible spatial model.
    pub fn new(config: MixerConfig) -> Self {
        Self::with_spatial_and_loop_attenuation(
            config,
            Box::new(LegacyQuakeSpatial::default()),
            config.loop_attenuation,
        )
    }

    /// Construct with a caller-selected spatializer.
    pub fn with_spatial(config: MixerConfig, spatial: Box<dyn SpatialModel>) -> Self {
        Self::with_spatial_and_loop_attenuation(config, spatial, Attenuation::None)
    }

    /// Construct with caller-owned spatial and loop attenuation policies.
    pub fn with_spatial_and_loop_attenuation(
        config: MixerConfig,
        spatial: Box<dyn SpatialModel>,
        loop_attenuation: Attenuation,
    ) -> Self {
        Self {
            bank: SoundBank::new(config.sample_rate),
            voices: vec![Voice::default(); config.voices].into_boxed_slice(),
            loops: vec![LoopVoice::default(); config.loops].into_boxed_slice(),
            listener: Listener {
                origin: [0.0; 3],
                forward: [1.0, 0.0, 0.0],
                right: [0.0, -1.0, 0.0],
                up: [0.0, 0.0, 1.0],
                velocity: [0.0; 3],
            },
            spatial,
            serial: 0,
            loop_generation: 0,
            loop_frame_open: false,
            loop_attenuation,
            listener_source: None,
            rendered_frames: 0,
            master_gain: 1.0,
            channel_gains: Vec::new(),
            music_gain: 1.0,
            background: None,
            doppler: config.doppler,
            protected_channels: config.protected_channels,
            effects: spatial_fx::SpatialEffects::new(
                config.sample_rate,
                config.voices,
                config.loops,
            ),
        }
    }

    pub fn sound_bank(&self) -> &SoundBank {
        &self.bank
    }

    pub fn sound_bank_mut(&mut self) -> &mut SoundBank {
        &mut self.bank
    }

    /// Update the listener exactly once per rendered frame.
    pub fn listener(&mut self, origin: [f32; 3], forward: [f32; 3], right: [f32; 3], up: [f32; 3]) {
        self.listener = Listener {
            origin,
            forward,
            right,
            up,
            velocity: self.listener.velocity,
        };
    }

    /// Set listener velocity for relative-velocity Doppler evaluation.
    pub fn set_listener_velocity(&mut self, velocity: [f32; 3]) {
        self.listener.velocity = velocity;
    }

    /// Toggle Doppler without rebuilding the fixed voice storage.
    pub fn set_doppler_enabled(&mut self, enabled: bool) {
        self.doppler.enabled = enabled;
    }

    /// Identify the listener entity for `S_PickChannel` steal protection.
    pub fn set_listener_source(&mut self, source: Option<SourceId>) {
        self.listener_source = source;
    }

    /// Move every currently spatial voice owned by `source` without allocation.
    /// This is the engine-generic counterpart to `S_UpdateEntityPosition`.
    pub fn update_source_position(&mut self, source: SourceId, origin: [f32; 3]) {
        for voice in &mut self.voices {
            if voice.active && voice.request.source == source && voice.request.origin.is_some() {
                voice.request.origin = Some(origin);
            }
        }
        for loop_voice in &mut self.loops {
            if loop_voice.active
                && loop_voice.request.source == source
                && loop_voice.request.origin.is_some()
            {
                loop_voice.request.origin = Some(origin);
            }
        }
    }

    /// Start a voice without decoding or heap allocation.
    ///
    /// Channel zero follows `CHAN_AUTO`: it never stomps by source/channel.
    /// Non-zero channels replace the same source+channel first. Otherwise the
    /// first free slot wins, then the oldest non-loop voice, matching
    /// `S_PickChannel` (`snd_dma.cpp:1088-1150`).
    pub fn play(&mut self, handle: SoundHandle, request: PlayRequest) -> bool {
        self.play_from(handle, request, 0)
    }

    /// [`Self::play`] from `offset` mono samples into the sound (at the mixer's
    /// rate), as if it had started that long ago; an offset past its end plays
    /// nothing.
    pub fn play_from(&mut self, handle: SoundHandle, request: PlayRequest, offset: usize) -> bool {
        if self
            .bank
            .get(handle)
            .is_none_or(|sound| offset > 0 && offset >= sound.samples.len())
            || self.voices.is_empty()
        {
            return false;
        }
        let replacement = (request.channel != ChannelId(0))
            .then(|| {
                self.voices.iter().position(|voice| {
                    voice.active
                        && voice.request.source == request.source
                        && voice.request.channel == request.channel
                })
            })
            .flatten();
        let free = self.voices.iter().position(|voice| !voice.active);
        let Some(index) = replacement
            .or(free)
            .or_else(|| self.steal_index(request.source))
        else {
            return false;
        };
        self.serial = self.serial.wrapping_add(1);
        self.voices[index] = Voice {
            active: true,
            handle,
            request,
            cursor: offset,
            started: self.serial,
        };
        true
    }

    /// [`Self::play_from`], but at the very sample `align`'s voice on the same
    /// channel plays the same sound, when it plays it within `window` samples of
    /// `offset`: the two then sound as one (see
    /// [`MixerConfig::protected_channels`]). Otherwise from `offset`.
    pub fn play_aligned(
        &mut self,
        handle: SoundHandle,
        request: PlayRequest,
        offset: usize,
        align: SourceId,
        window: usize,
    ) -> bool {
        let aligned = self
            .voices
            .iter()
            .find(|voice| {
                voice.active
                    && voice.handle == handle
                    && voice.request.source == align
                    && voice.request.channel == request.channel
                    && voice.cursor.abs_diff(offset) <= window
            })
            .map_or(offset, |voice| voice.cursor);
        self.play_from(handle, request, aligned)
    }

    /// Apply the legacy channel-picker mute to a source/channel pair.
    ///
    /// Compatibility adapters may normalize related logical channels before
    /// calling this method. Like `S_MuteSound` → `S_PickChannel`, an existing
    /// source/channel is selected first; otherwise a free slot is harmlessly
    /// cleared, or a full table sacrifices its oldest eligible voice. The
    /// operation only touches the fixed voice array and never allocates.
    pub fn stop_source_channel(&mut self, source: SourceId, channel: ChannelId) -> bool {
        let matching = self.voices.iter().position(|voice| {
            voice.active && voice.request.source == source && voice.request.channel == channel
        });
        let free = self.voices.iter().position(|voice| !voice.active);
        let Some(index) = matching.or(free).or_else(|| self.steal_index(source)) else {
            return false;
        };
        let killed = self.voices[index].active;
        self.voices[index].active = false;
        killed
    }

    /// The voice a full table gives up for `source`: the oldest one, never the
    /// listener's for another source's sound, and one on the protected channel
    /// only when every other candidate is protected too.
    fn steal_index(&self, source: SourceId) -> Option<usize> {
        self.voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| {
                !(Some(voice.request.source) == self.listener_source
                    && Some(source) != self.listener_source)
            })
            .min_by_key(|(_, voice)| (self.protected(voice.request.channel), voice.started))
            .map(|(index, _)| index)
    }

    /// Stop every per-frame loop currently mixed for `source` immediately.
    ///
    /// A later [`Self::set_loop`] may re-add it; game-side persistent lifetime
    /// remains the caller's responsibility, just as Quake cgame owns its
    /// event-maintained loop table independently of the sound backend.
    pub fn stop_source_loops(&mut self, source: SourceId) -> usize {
        let mut stopped = 0;
        for loop_voice in &mut self.loops {
            if loop_voice.active && loop_voice.request.source == source {
                loop_voice.active = false;
                stopped += 1;
            }
        }
        stopped
    }

    /// Stop every voice, loop and background track playing `handle`, then take
    /// its PCM out of the bank and hand it back, so the caller frees it away
    /// from the audio thread. The handle plays nothing afterwards; other
    /// handles keep their sounds. Allocation-free and never frees.
    pub fn release_sound(&mut self, handle: SoundHandle) -> Option<DecodedSound> {
        let released = self.bank.release(handle)?;
        for voice in &mut self.voices {
            if voice.handle == handle {
                voice.active = false;
            }
        }
        for loop_voice in &mut self.loops {
            if loop_voice.handle == handle {
                loop_voice.active = false;
            }
        }
        if self
            .background
            .is_some_and(|track| track.intro == handle || track.repeating == handle)
        {
            self.background = None;
        }
        Some(released)
    }

    /// Stop all voices, loops and background music, retaining decoded assets.
    pub fn stop_all(&mut self) {
        self.stop_effects();
        self.background = None;
    }

    /// Stop transient voices and loops without racing independently queued music changes.
    pub fn stop_effects(&mut self) {
        self.voices.fill(Voice::default());
        self.loops.fill(LoopVoice::default());
        self.effects.clear();
    }

    /// Switch the 3D sound on or off: each sound placed in the world heard through
    /// the head and filtered by what is in its way ([`Self::set_source_acoustics`]),
    /// and the room ringing ([`Self::set_room_response`]). Off (the default), the mix
    /// is exactly the plain one; those settings are kept and apply again when
    /// switched on.
    pub fn set_spatial_effects(&mut self, enabled: bool) {
        self.effects.enabled = enabled;
    }

    /// Hear the 3D sound through the head (headphones, the default) or panned as
    /// the old mixer pans (speakers).
    pub fn set_spatial_headphones(&mut self, headphones: bool) {
        self.effects.headphones = headphones;
    }

    /// What reaches the listener from `source`: its voices and loops glide there
    /// over about 50 ms, and voices it starts later begin there. Only positional,
    /// attenuated voices take it. Allocation-free.
    pub fn set_source_acoustics(&mut self, source: SourceId, acoustics: SourceAcoustics) {
        self.effects.remember(source, acoustics);
    }

    /// Ring with `response` from now on, crossing over from the one before; `None`
    /// is a dry room. The old response comes back by [`Self::take_retired_response`]
    /// to be freed off the audio thread.
    pub fn set_room_response(&mut self, response: Option<Box<RoomResponse>>) {
        self.effects.set_room_response(response);
    }

    /// A response the mixer no longer uses ([`Self::set_room_response`]).
    pub fn take_retired_response(&mut self) -> Option<Box<RoomResponse>> {
        self.effects.take_retired_response()
    }

    /// How loud the room rings, 1 as traced (0 to 4).
    pub fn set_reverb_gain(&mut self, gain: f32) {
        self.effects.set_reverb_gain(gain);
    }

    /// Begin rebuilding the current frame's loop set.
    ///
    /// The previous frame's loops keep sounding until [`Self::commit_loops`]
    /// closes the rebuild, so a render that lands between this call and the
    /// frame's last [`Self::set_loop`] cannot cut them for a block. Loops not
    /// re-added by the commit stop at the next render.
    pub fn clear_loops(&mut self) {
        self.loop_generation = self.loop_generation.wrapping_add(1);
        self.loop_frame_open = true;
    }

    /// Finish the loop set begun by [`Self::clear_loops`].
    pub fn commit_loops(&mut self) {
        self.loop_frame_open = false;
    }

    /// Whether a loop set has been cleared but not yet committed.
    pub const fn loop_frame_open(&self) -> bool {
        self.loop_frame_open
    }

    /// Re-add or start a looping sound for this frame without allocation.
    pub fn set_loop(
        &mut self,
        source: SourceId,
        handle: SoundHandle,
        origin: [f32; 3],
        velocity: [f32; 3],
    ) -> bool {
        self.set_loop_request(
            handle,
            PlayRequest {
                origin: Some(origin),
                source,
                channel: ChannelId(0),
                volume: 1.0,
                attenuation: self.loop_attenuation,
            },
            velocity,
        )
    }

    /// Re-add a loop with an explicit generic voice request and source velocity.
    pub fn set_loop_request(
        &mut self,
        handle: SoundHandle,
        request: PlayRequest,
        velocity: [f32; 3],
    ) -> bool {
        if self.bank.get(handle).is_none() {
            return false;
        }
        let (generation, open) = (self.loop_generation, self.loop_frame_open);
        if let Some(loop_voice) = self.loops.iter_mut().find(|loop_voice| {
            loop_voice.request.source == request.source && loop_voice.handle == handle
        }) {
            let continuously_readded = loop_voice.active
                && (loop_voice.generation == generation
                    || loop_voice.generation.wrapping_add(1) == generation);
            if !continuously_readded {
                loop_voice.cursor = self.rendered_frames as f64;
            }
            loop_voice.active = true;
            loop_voice.request = request;
            loop_voice.velocity = velocity;
            loop_voice.generation = self.loop_generation;
            return true;
        }
        // Prefer a slot nobody is mixing; a previous-frame loop that has not
        // been re-added yet is only taken when the table is otherwise full.
        let Some(index) = self
            .loops
            .iter()
            .position(|loop_voice| !loop_audible(loop_voice, generation, open))
            .or_else(|| {
                self.loops
                    .iter()
                    .position(|loop_voice| loop_voice.generation != generation)
            })
        else {
            return false;
        };
        self.loops[index] = LoopVoice {
            active: true,
            handle,
            request,
            velocity,
            generation: self.loop_generation,
            cursor: self.rendered_frames as f64,
        };
        true
    }

    /// Set the effects/voice/loop master gain without touching source voices.
    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain.clamp(0.0, 1.0);
    }

    /// Give the one-shot voices of `channel` the gain `gain` in place of the master
    /// gain (a separate volume bus, as a game's voice volume); `None` ends it. Each
    /// channel keeps its own bus, from 0 to [`MAX_CHANNEL_GAIN`] (above 1 louder than
    /// recorded, as the emotes' own volume). Loops and music are not affected.
    pub fn set_channel_gain(&mut self, channel: ChannelId, gain: Option<f32>) {
        self.channel_gains.retain(|(bus, _)| *bus != channel);
        if let Some(gain) = gain {
            self.channel_gains
                .push((channel, gain.clamp(0.0, MAX_CHANNEL_GAIN)));
        }
    }

    /// Set the background-music gain independently of the master gain.
    pub fn set_music_gain(&mut self, gain: f32) {
        self.music_gain = gain.clamp(0.0, 1.0);
    }

    /// Start one intro sound followed sample-exactly by a repeating sound.
    ///
    /// Supplying the same handle for both implements a conventional single
    /// looping track. Decoding remains an explicit registration-time action.
    pub fn start_background_track(
        &mut self,
        intro: SoundHandle,
        repeating: SoundHandle,
        volume: f32,
    ) -> bool {
        self.start_background_track_at(intro, repeating, volume, 0)
    }

    /// Start at elapsed milliseconds in the intro/repeating sequence, in constant time.
    /// The intro plays once; elapsed time beyond it wraps within the repeating sound.
    /// Decoded PCM must already be registered. A failed start preserves the old track.
    pub fn start_background_track_at(
        &mut self,
        intro: SoundHandle,
        repeating: SoundHandle,
        volume: f32,
        elapsed_ms: u64,
    ) -> bool {
        let Some(intro_len) = self.bank.sample_count(intro).filter(|count| *count > 0) else {
            return false;
        };
        let Some(loop_len) = self.bank.sample_count(repeating).filter(|count| *count > 0) else {
            return false;
        };
        let frames = u128::from(elapsed_ms) * u128::from(self.bank.sample_rate) / 1000;
        let in_intro = frames < intro_len as u128;
        let cursor = if in_intro {
            frames as usize
        } else {
            ((frames - intro_len as u128) % loop_len as u128) as usize
        };
        self.background = Some(BackgroundTrack {
            intro,
            repeating,
            cursor,
            in_intro,
            volume: volume.clamp(0.0, 1.0),
        });
        true
    }

    /// Stop the current background track without affecting sound voices.
    pub fn stop_background_track(&mut self) {
        self.background = None;
    }

    /// Current intro/repeating cursor, primarily for diagnostics and tests.
    pub fn background_track_state(&self) -> Option<BackgroundTrackState> {
        self.background.map(|track| BackgroundTrackState {
            in_intro: track.in_intro,
            cursor: track.cursor,
        })
    }

    /// Mix interleaved stereo samples without allocating.
    pub fn render(&mut self, out: &mut [f32]) {
        // Room acoustics mix into a fixed send: a longer block is mixed in parts.
        let send_samples = spatial_fx::SEND_FRAMES * 2;
        if self.effects.enabled && out.len() > send_samples {
            for part in out.chunks_mut(send_samples) {
                self.render(part);
            }
            return;
        }
        out.fill(0.0);
        let frames = out.len() / 2;
        let spatial = self.effects.enabled;
        if spatial {
            self.effects.send[..frames].fill(0.0);
        }
        for index in 0..self.voices.len() {
            let voice = self.voices[index];
            if !voice.active {
                continue;
            }
            let Some(sound) = self.bank.get(voice.handle) else {
                self.voices[index].active = false;
                continue;
            };
            let merged = self.protected(voice.request.channel);
            // Mixed already with an earlier voice of its group this block.
            if merged
                && self.voices[..index]
                    .iter()
                    .any(|other| same_sample(other, &voice))
            {
                continue;
            }
            // The 16-bit step joins the gain: one multiply a sample, as before.
            let gain = self
                .channel_gains
                .iter()
                .find(|(channel, _)| *channel == voice.request.channel)
                .map_or(self.master_gain, |(_, gain)| *gain);
            if spatial && !merged && occludable(&voice.request) {
                // 3D sound: through the head, filtered by what is in the way, and
                // into the room's send.
                let placement = self.placement(&voice.request, gain, 1.0);
                let samples = &sound.samples;
                let mut cursor = voice.cursor;
                let active = self.effects.mix(
                    spatial_fx::EarRef::Voice(index),
                    voice.started,
                    &placement,
                    out,
                    || {
                        let sample = samples.get(cursor).copied().map(f32::from);
                        cursor += 1;
                        sample
                    },
                );
                self.voices[index].cursor = cursor.min(samples.len());
                self.voices[index].active = active;
                continue;
            }
            let gains_of = |voice: &Voice| {
                let gains = voice.request.origin.map_or([1.0; 2], |origin| {
                    self.spatial
                        .gains(&self.listener, origin, voice.request.attenuation)
                });
                gains.map(|gain| gain * voice.request.volume)
            };
            let mut gains = gains_of(&voice);
            if merged {
                for other in &self.voices[index + 1..] {
                    if same_sample(other, &voice) {
                        let more = gains_of(other);
                        gains = [gains[0].max(more[0]), gains[1].max(more[1])];
                    }
                }
            }
            let mut cursor = voice.cursor;
            let mut active = true;
            let scale = gain * SAMPLE_STEP;
            for frame in out.chunks_exact_mut(2) {
                if cursor >= sound.samples.len() {
                    active = false;
                    break;
                }
                let sample = f32::from(sound.samples[cursor]) * scale;
                frame[0] += sample * gains[0];
                frame[1] += sample * gains[1];
                cursor += 1;
            }
            for other in index..self.voices.len() {
                if other == index || (merged && same_sample(&self.voices[other], &voice)) {
                    self.voices[other].cursor = cursor;
                    self.voices[other].active = active;
                }
            }
        }
        self.render_loops(out, spatial);
        if spatial {
            self.effects.apply_reverb(out);
        }
        self.render_background(out);
        self.rendered_frames = self.rendered_frames.wrapping_add((out.len() / 2) as u64);
        for sample in out {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    /// How a sound placed in the world is heard with the 3D sound: its loudness at
    /// its distance (times `gain` and `loudness`, the 16-bit step folded in), its
    /// panning for speakers, its direction from the head, what is in its way.
    fn placement(&self, request: &PlayRequest, gain: f32, loudness: f32) -> spatial_fx::Placement {
        let origin = request.origin.unwrap_or(self.listener.origin);
        let acoustics = self.effects.acoustics_of(request.source);
        let scale = gain * loudness * request.volume * SAMPLE_STEP;
        let pan = self
            .spatial
            .gains(&self.listener, origin, request.attenuation)
            .map(|pan| pan * scale);
        let heard = sub(acoustics.via.unwrap_or(origin), self.listener.origin);
        let at_head = length(sub(origin, self.listener.origin)) < spatial_fx::NEAR_HEAD
            || Some(request.source) == self.listener_source;
        let direction = (!at_head).then(|| {
            [
                dot(heard, self.listener.forward),
                dot(heard, self.listener.right),
                dot(heard, self.listener.up),
            ]
        });
        spatial_fx::Placement {
            gain: distance_gain(&self.listener, origin, request.attenuation) * scale,
            pan,
            direction,
            bands: acoustics.bands,
            send: true,
        }
    }

    fn render_loops(&mut self, out: &mut [f32], spatial: bool) {
        let (generation, open) = (self.loop_generation, self.loop_frame_open);
        for index in 0..self.loops.len() {
            if !loop_audible(&self.loops[index], generation, open) {
                self.loops[index].active = false;
                continue;
            }
            let handle = self.loops[index].handle;
            if self.loops[..index]
                .iter()
                .any(|earlier| loop_audible(earlier, generation, open) && earlier.handle == handle)
            {
                continue;
            }
            let mut gains = [0.0; 2];
            // 3D sound: the group is heard from its loudest loop placed in the world,
            // as loud as all of them.
            let mut loudest: Option<(usize, f32)> = None;
            let mut loudness = 0.0_f32;
            for (offset, loop_voice) in self.loops[index..].iter().enumerate() {
                if !loop_audible(loop_voice, generation, open) || loop_voice.handle != handle {
                    continue;
                }
                let spatial_gains = loop_voice.request.origin.map_or([1.0; 2], |origin| {
                    self.spatial
                        .gains(&self.listener, origin, loop_voice.request.attenuation)
                });
                gains[0] += spatial_gains[0] * loop_voice.request.volume;
                gains[1] += spatial_gains[1] * loop_voice.request.volume;
                if spatial && occludable(&loop_voice.request) {
                    let origin = loop_voice.request.origin.unwrap_or(self.listener.origin);
                    let heard =
                        distance_gain(&self.listener, origin, loop_voice.request.attenuation)
                            * loop_voice.request.volume.abs();
                    loudness += heard;
                    if loudest.is_none_or(|(_, most)| heard > most) {
                        loudest = Some((index + offset, heard));
                    }
                }
            }
            gains[0] = gains[0].min(1.0) * self.master_gain * SAMPLE_STEP;
            gains[1] = gains[1].min(1.0) * self.master_gain * SAMPLE_STEP;
            let Some(sound) = self.bank.get(handle) else {
                continue;
            };
            if sound.samples.is_empty() {
                continue;
            }
            let pitch = doppler_scale(
                self.doppler,
                &self.listener,
                self.loops[index].request.origin,
                self.loops[index].velocity,
            );
            let mut cursor = self.loops[index].cursor;
            let samples = &sound.samples;
            let mut read = || {
                let base = cursor.floor() as usize;
                let fraction = (cursor - base as f64) as f32;
                let low = base % samples.len();
                let high = (low + 1) % samples.len();
                // In 16-bit steps; the gains carry the step back to [-1, 1].
                let (low, high) = (f32::from(samples[low]), f32::from(samples[high]));
                cursor += f64::from(pitch);
                low + (high - low) * fraction
            };
            match loudest {
                Some((leader, heard)) if heard > 0.0 => {
                    let request = self.loops[leader].request;
                    // As loud as the group, at most full: the leader's own loudness is
                    // scaled up to it.
                    let group = loudness.min(1.0) / heard;
                    let mut placement = self.placement(&request, self.master_gain, group);
                    // Speakers: the old mixer's panning of the whole group.
                    placement.pan = gains;
                    self.effects.mix(
                        spatial_fx::EarRef::Loop(index),
                        u64::from(handle.0),
                        &placement,
                        out,
                        || Some(read()),
                    );
                }
                Some(_) => {
                    // Out of hearing: only the time passes.
                    for _ in 0..out.len() / 2 {
                        read();
                    }
                }
                None => {
                    for frame in out.chunks_exact_mut(2) {
                        let sample = read();
                        frame[0] += sample * gains[0];
                        frame[1] += sample * gains[1];
                    }
                }
            }
            for loop_voice in &mut self.loops[index..] {
                if loop_audible(loop_voice, generation, open) && loop_voice.handle == handle {
                    loop_voice.cursor = cursor;
                }
            }
        }
    }

    fn render_background(&mut self, out: &mut [f32]) {
        let Some(track) = &mut self.background else {
            return;
        };
        let scale = track.volume * self.music_gain * SAMPLE_STEP;
        for frame in out.chunks_exact_mut(2) {
            let handle = if track.in_intro {
                track.intro
            } else {
                track.repeating
            };
            let Some(sound) = self.bank.get(handle) else {
                return;
            };
            if track.cursor >= sound.samples.len() {
                if track.in_intro {
                    track.in_intro = false;
                    track.cursor = 0;
                } else {
                    track.cursor = 0;
                }
            }
            let handle = if track.in_intro {
                track.intro
            } else {
                track.repeating
            };
            let Some(sound) = self.bank.get(handle) else {
                return;
            };
            if sound.samples.is_empty() {
                return;
            }
            let sample = f32::from(sound.samples[track.cursor]) * scale;
            frame[0] += sample;
            frame[1] += sample;
            track.cursor += 1;
        }
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.active).count()
    }

    /// Number of per-frame loops currently retained by the fixed loop table.
    pub fn active_loop_count(&self) -> usize {
        self.loops
            .iter()
            .filter(|loop_voice| loop_voice.active)
            .count()
    }
}

/// Whether a voice can be occluded: placed in the world and fading with distance
/// (a listener-relative or global sound is never behind a wall).
fn occludable(request: &PlayRequest) -> bool {
    request.origin.is_some() && request.attenuation != Attenuation::None
}

/// Whether a loop belongs to the set being mixed: re-added this frame, or, while
/// a rebuild is still open, carried over from the previous frame.
fn loop_audible(loop_voice: &LoopVoice, generation: u64, open: bool) -> bool {
    loop_voice.active
        && (loop_voice.generation == generation
            || (open && loop_voice.generation.wrapping_add(1) == generation))
}

fn doppler_scale(
    config: DopplerConfig,
    listener: &Listener,
    origin: Option<[f32; 3]>,
    source_velocity: [f32; 3],
) -> f32 {
    if !config.enabled || config.distance_ratio_divisor <= 0.0 {
        return 1.0;
    }
    let Some(origin) = origin else { return 1.0 };
    let relative_velocity = sub(source_velocity, listener.velocity);
    if dot(relative_velocity, relative_velocity) <= f32::EPSILON {
        return 1.0;
    }
    let distance = sub(origin, listener.origin);
    let before = dot(distance, distance);
    if before <= f32::EPSILON {
        return config.maximum_scale.max(1.0);
    }
    let after = sub(add(origin, relative_velocity), listener.origin);
    let scale = dot(after, after) / (before * config.distance_ratio_divisor);
    if scale <= 1.0 {
        1.0
    } else {
        scale.min(config.maximum_scale.max(1.0))
    }
}

/// Decode WAV or MP3 bytes to 16-bit mono PCM at the file's own rate.
fn decode_pcm16(bytes: &[u8], extension: &str) -> Result<(Vec<i16>, u32), DecodeError> {
    if extension.eq_ignore_ascii_case("mp3") {
        decode_mp3(bytes)
    } else {
        decode_wav(bytes)
    }
}

fn decode_wav(bytes: &[u8]) -> Result<(Vec<i16>, u32), DecodeError> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes))
        .map_err(|error| DecodeError(format!("WAV: {error}")))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));
    // The header's frame count, never more than the bytes could hold.
    let frames = (reader.len() as usize / channels).min(bytes.len());
    let mut output = Vec::with_capacity(frames);
    match spec.sample_format {
        hound::SampleFormat::Float => {
            mix_down(
                reader.samples::<f32>(),
                channels,
                &mut output,
                std::convert::identity,
            )?;
        }
        hound::SampleFormat::Int => {
            let scale = (1_u64 << spec.bits_per_sample.saturating_sub(1)) as f32;
            mix_down(reader.samples::<i32>(), channels, &mut output, |value| {
                value as f32 / scale
            })?;
        }
    }
    Ok((output, spec.sample_rate))
}

/// Average each frame of `channels` interleaved WAV samples (read as [-1, 1]
/// floats by `value`) into one 16-bit sample; a short last frame averages what
/// it has.
fn mix_down<S>(
    samples: impl Iterator<Item = Result<S, hound::Error>>,
    channels: usize,
    output: &mut Vec<i16>,
    value: impl Fn(S) -> f32,
) -> Result<(), DecodeError> {
    let (mut sum, mut count) = (0.0_f32, 0_usize);
    for sample in samples {
        sum += value(sample.map_err(|error| DecodeError(format!("WAV samples: {error}")))?);
        count += 1;
        if count == channels {
            output.push(quantize_sample(sum / count as f32));
            (sum, count) = (0.0, 0);
        }
    }
    if count > 0 {
        output.push(quantize_sample(sum / count as f32));
    }
    Ok(())
}

fn decode_mp3(bytes: &[u8]) -> Result<(Vec<i16>, u32), DecodeError> {
    let bytes = id3::without_id3v1(bytes);
    let mut decoder = minimp3_fixed::Decoder::new(Cursor::new(bytes));
    let mut output = Vec::new();
    let mut rate = None;
    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                rate.get_or_insert(frame.sample_rate as u32);
                let channels = frame.channels.max(1);
                output.extend(frame.data.chunks(channels).map(|samples| {
                    let sum = samples.iter().map(|sample| f32::from(*sample)).sum::<f32>();
                    quantize_sample(sum * SAMPLE_STEP / samples.len() as f32)
                }));
            }
            Err(minimp3_fixed::Error::Eof) => break,
            Err(error) => return Err(DecodeError(format!("MP3: {error:?}"))),
        }
    }
    Ok((
        output,
        rate.ok_or_else(|| DecodeError("MP3 contained no frames".into()))?,
    ))
}

/// Linear interpolation from `source_rate` to `target_rate`: `read` gives each
/// source sample's [-1, 1] float and `store` keeps each result.
fn resample<S: Copy, T>(
    samples: &[S],
    source_rate: u32,
    target_rate: u32,
    read: impl Fn(S) -> f32,
    store: impl Fn(f32) -> T,
) -> Vec<T> {
    if source_rate == target_rate || samples.is_empty() {
        return samples.iter().map(|sample| store(read(*sample))).collect();
    }
    let length = (samples.len() as u64 * u64::from(target_rate) / u64::from(source_rate)) as usize;
    (0..length)
        .map(|index| {
            let source = index as f64 * f64::from(source_rate) / f64::from(target_rate);
            let low = source.floor() as usize;
            let high = (low + 1).min(samples.len() - 1);
            let fraction = (source - low as f64) as f32;
            let (low, high) = (read(samples[low]), read(samples[high]));
            store(low + (high - low) * fraction)
        })
        .collect()
}

fn sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] - right[axis])
}
fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] + right[axis])
}
fn scale(value: [f32; 3], amount: f32) -> [f32; 3] {
    value.map(|component| component * amount)
}
fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    (0..3).map(|axis| left[axis] * right[axis]).sum()
}
fn length(value: [f32; 3]) -> f32 {
    dot(value, value).sqrt()
}

#[cfg(test)]
mod loop_frame_tests;
#[cfg(test)]
mod spatial_tests;
#[cfg(test)]
mod sync_mix_tests;
#[cfg(test)]
mod voice_steal_tests;

#[cfg(test)]
mod music_offset_tests;
#[cfg(test)]
mod pcm16_tests;
#[cfg(test)]
mod spatial_fx_tests;

/// Counts allocator calls inside `assert_no_alloc` blocks (`pcm16_tests`).
#[cfg(all(test, debug_assertions))]
#[global_allocator]
static ALLOCATOR: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
