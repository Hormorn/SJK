//! Allocation-free rodio callback and bounded command/decode queues.

use ringbuf::{HeapCons, HeapProd, HeapRb, traits::*};
use rodio::{OutputStream, Source};
use sjk_audio::{
    ChannelId, DecodedSound, Mixer, MixerConfig, PlayRequest, SoundHandle, SourceId,
    decode_encoded, decode_mono,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[path = "audio_device.rs"]
mod device;
#[path = "audio_feed.rs"]
mod feed;
#[path = "audio_pending.rs"]
mod pending;

pub(super) const SAMPLE_RATE: u32 = 44_100;
/// Four times retail's 32 channels: a busy saber fight filled 32, and every
/// voice costs one pass over a 256-sample block.
pub(super) const VOICES: usize = 128;
pub(super) const LOOPS: usize = 32;
const MIX_SAMPLES: usize = 256;
const MIX_BLOCK_BUDGET: Duration =
    Duration::from_micros((MIX_SAMPLES as u64 / 2) * 1_000_000 / SAMPLE_RATE as u64);
const COMMANDS: usize = 8_192;
const DECODE_JOBS: usize = 8_192;
/// Released sounds on their way from the callback back to the decode worker,
/// which frees them; a handful at most change hands at each music change.
const RELEASES: usize = 256;
/// Room responses waiting to be freed (`s_spatial`; a few a second at most).
const RETIRED_RESPONSES: usize = 16;
/// How often an idle decode worker frees what the callback released.
const RELEASE_POLL: Duration = Duration::from_millis(200);

/// Counters the audio thread publishes for the render thread's diagnostics;
/// the audio thread only ever touches atomics.
#[derive(Default)]
pub(super) struct OutputStats {
    /// Mix blocks that took longer than their playback duration to render.
    pub(super) underruns: AtomicU64,
    /// Bits of the loudest sample rendered since the last `take_peak`.
    peak_bits: AtomicU32,
    pub(super) rendered_blocks: AtomicU64,
    pub(super) music_starts: AtomicU64,
    /// Music starts the mixer refused (a handle without decoded samples).
    pub(super) music_rejections: AtomicU64,
    /// Sounds registered as silence because their bytes did not decode.
    pub(super) decode_failures: AtomicU64,
    /// Registrations whose bank index differed from the handle handed out.
    pub(super) handle_mismatches: AtomicU64,
    /// Mix blocks rendered while the render thread was still sending a
    /// frame's loops; each one used to cut the unsent loops for a block.
    pub(super) split_loop_frames: AtomicU64,
    /// Bytes of decoded PCM the mixer holds, as of its last registration or release.
    pub(super) pcm_bytes: AtomicU64,
    /// Bytes of released sounds the decode worker has freed.
    pub(super) freed_bytes: AtomicU64,
    /// Releases the callback could not hand back (no room): those sounds stay decoded.
    pub(super) releases_kept: AtomicU64,
}

impl OutputStats {
    /// Loudest sample since the previous call.
    pub(super) fn take_peak(&self) -> f32 {
        f32::from_bits(self.peak_bits.swap(0, Ordering::Relaxed))
    }

    fn observe(&self, block: &[f32]) {
        let peak = block
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        // Non-negative floats order like their bit patterns.
        self.peak_bits.fetch_max(peak.to_bits(), Ordering::Relaxed);
        self.rendered_blocks.fetch_add(1, Ordering::Relaxed);
    }
}

struct MixerSource {
    mixer: Mixer,
    commands: HeapCons<AudioCommand>,
    registrations: HeapCons<DecodedAction>,
    /// Released PCM back to the decode worker: the callback never frees memory.
    released: HeapProd<DecodedSound>,
    /// Room responses the 3D sound no longer uses, back to the decode worker too,
    /// and the one waiting for room in that queue.
    retired: HeapProd<Box<sjk_audio::RoomResponse>>,
    held: Option<Box<sjk_audio::RoomResponse>>,
    scratch: [f32; MIX_SAMPLES],
    cursor: usize,
    running: bool,
    stats: Arc<OutputStats>,
    pending: pending::PendingStarts,
}

impl MixerSource {
    /// Queue the room responses the mixer let go of for the decode worker to free;
    /// one that finds the queue full waits for the next block.
    fn hand_back_responses(&mut self) {
        loop {
            let Some(response) = self
                .held
                .take()
                .or_else(|| self.mixer.take_retired_response())
            else {
                return;
            };
            if let Err(response) = self.retired.try_push(response) {
                self.held = Some(response);
                return;
            }
        }
    }
}

impl Iterator for MixerSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.running {
            return None;
        }
        if self.cursor == self.scratch.len() {
            let mut bank_changed = false;
            while let Some(action) = self.registrations.try_pop() {
                bank_changed |= matches!(
                    action,
                    DecodedAction::Register(..) | DecodedAction::Release(_)
                );
                match action {
                    DecodedAction::Register(expected, decoded) => {
                        // Registration must run in release too: the bank
                        // index is the handle the render thread already
                        // handed out, so it has to be assigned in order.
                        let registered = self.mixer.sound_bank_mut().register_decoded(decoded);
                        if registered != expected {
                            self.stats.handle_mismatches.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    DecodedAction::Release(handle) => {
                        // Without room to hand it back the sound stays decoded:
                        // dropping it here would free memory on the audio thread.
                        if self.released.vacant_len() == 0 {
                            self.stats.releases_kept.fetch_add(1, Ordering::Relaxed);
                        } else if let Some(sound) = self.mixer.release_sound(handle) {
                            // Cannot fail: room was checked, and this is the only producer.
                            let _ = self.released.try_push(sound);
                        }
                    }
                    DecodedAction::StopMusic => self.mixer.stop_background_track(),
                    DecodedAction::StartMusic(intro, repeating, offset) => {
                        let elapsed = offset.map_or(0, |offset| offset.position(Instant::now()));
                        let counter = if self
                            .mixer
                            .start_background_track_at(intro, repeating, 1.0, elapsed)
                        {
                            &self.stats.music_starts
                        } else {
                            &self.stats.music_rejections
                        };
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            if bank_changed {
                let bytes = self.mixer.sound_bank().sample_bytes() as u64;
                self.stats.pcm_bytes.store(bytes, Ordering::Relaxed);
            }
            while let Some(command) = self.commands.try_pop() {
                if let AudioCommand::Play(handle, request) = command {
                    self.pending.play(&mut self.mixer, handle, request);
                    continue;
                }
                if let AudioCommand::PlayTrack(handle, request, elapsed_ms, queued_at) = command {
                    let offset = MusicOffset {
                        elapsed_ms,
                        queued_at,
                    };
                    self.pending
                        .play_track(&mut self.mixer, handle, request, offset);
                    continue;
                }
                if let AudioCommand::StopChannel(source, channel)
                | AudioCommand::PlayFrom(
                    _,
                    PlayRequest {
                        source, channel, ..
                    },
                    ..,
                ) = &command
                {
                    self.pending.cancel(*source, *channel);
                }
                if matches!(command, AudioCommand::StopAll | AudioCommand::StopEffects) {
                    self.pending.clear();
                }
                self.running = apply_command(&mut self.mixer, command);
                if !self.running {
                    return None;
                }
            }
            self.pending.flush(&mut self.mixer);
            if self.mixer.loop_frame_open() {
                self.stats.split_loop_frames.fetch_add(1, Ordering::Relaxed);
            }
            let started = Instant::now();
            self.mixer.render(&mut self.scratch);
            self.hand_back_responses();
            if started.elapsed() > MIX_BLOCK_BUDGET {
                self.stats.underruns.fetch_add(1, Ordering::Relaxed);
            }
            self.stats.observe(&self.scratch);
            self.cursor = 0;
        }
        let sample = self.scratch[self.cursor];
        self.cursor += 1;
        Some(sample)
    }
}

impl Source for MixerSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub(super) enum AudioCommand {
    Play(SoundHandle, PlayRequest),
    /// Play a loaded sound from this many mono samples in, or at the sample the
    /// given source's voice plays it near there ([`sjk_audio::Mixer::play_aligned`]);
    /// one still decoding is not started.
    PlayFrom(SoundHandle, PlayRequest, usize, Option<SourceId>),
    /// Play a long sound (a boombox track) this many milliseconds in, as of the
    /// instant given; one still decoding starts once it is, further in by the time it
    /// waited.
    PlayTrack(SoundHandle, PlayRequest, u64, Instant),
    Listener {
        origin: [f32; 3],
        forward: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        velocity: [f32; 3],
    },
    ListenerSource(Option<SourceId>),
    SourcePosition(SourceId, [f32; 3]),
    StopChannel(SourceId, ChannelId),
    StopLoops(SourceId),
    /// Start a frame's loop set; the previous set plays on until `CommitLoops`.
    ClearLoops,
    /// End the loop set begun by `ClearLoops`.
    CommitLoops,
    StopAll,
    StopEffects,
    SetLoop(SoundHandle, PlayRequest, [f32; 3]),
    /// Effects (`s_volume`), music, voice (`s_volumeVoice`) and emote-channel gains.
    Gains(f32, f32, f32, f32),
    Doppler(bool),
    Separation(f32),
    /// 3D sound on or off (`s_spatial`, `audio_spatial.rs`).
    SpatialEffects(bool),
    /// Through the head (headphones) or panned (speakers).
    SpatialHeadphones(bool),
    /// What reaches the listener from a source.
    SourceAcoustics(SourceId, sjk_audio::SourceAcoustics),
    /// The room round the listener; `None` is dry.
    RoomResponse(Option<Box<sjk_audio::RoomResponse>>),
    /// How loud the room rings (`s_spatialReverb`).
    ReverbGain(f32),
    Shutdown,
}

/// Clock anchor travels with one synced start through decoding and callback queues.
#[derive(Clone, Copy)]
pub(super) struct MusicOffset {
    elapsed_ms: u64,
    queued_at: Instant,
}
impl MusicOffset {
    fn position(self, now: Instant) -> u64 {
        self.elapsed_ms.saturating_add(
            now.saturating_duration_since(self.queued_at)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        )
    }
}

enum DecodedAction {
    Register(SoundHandle, DecodedSound),
    /// Stop what plays the sound and hand its PCM back to be freed.
    Release(SoundHandle),
    StartMusic(SoundHandle, SoundHandle, Option<MusicOffset>),
    StopMusic,
}

pub(super) struct AudioOutput {
    _stream: Option<OutputStream>,
    pub(super) description: String,
    commands: HeapProd<AudioCommand>,
    /// Whether the callback still drains `commands`; a stopped stream is not waited on.
    feed: feed::Feed,
    decode_jobs: Option<SyncSender<DecodeJob>>,
    decoder: Option<JoinHandle<()>>,
    /// Set as the output is dropped: the decoder stops waiting for queue room.
    closing: Arc<AtomicBool>,
    pub(super) stats: Arc<OutputStats>,
    pub(super) show: bool,
    pub(super) started: std::collections::VecDeque<SoundHandle>,
    pub(super) cache: std::cell::RefCell<Vec<(SoundHandle, usize, String)>>,
    /// Allocated at registration; the decoder publishes duration without taking a lock.
    lengths: std::cell::RefCell<Vec<Arc<AtomicU32>>>,
    /// The world sounds sent lately, for the sound sonar (`sound_sonar.rs`).
    pub(super) sonar: crate::sound_sonar::Sonar,
    /// The world sounds sent lately and the room, for 3D sound (`audio_spatial.rs`).
    pub(super) spatial: crate::audio::spatial::Tracker,
    /// The device the stream plays on and where its mixer comes back (`audio_device.rs`).
    follow: Option<device::Follow>,
    /// Sounds muted by name (`s_muteSounds`), never started.
    pub(super) muted: crate::audio::muted_sounds::MutedSounds,
}

enum DecodeJob {
    Sound {
        handle: SoundHandle,
        bytes: Box<[u8]>,
        extension: Box<str>,
        length_ms: Arc<AtomicU32>,
    },
    /// A sound made from a decoded one: `make` turns its mixer-rate samples into the
    /// new sound's (`sjk_chat_sound.rs`).
    Variant {
        handle: SoundHandle,
        bytes: Box<[u8]>,
        extension: Box<str>,
        make: fn(&[f32], u32) -> Vec<f32>,
    },
    /// Free a registered sound ([`AudioOutput::release`]).
    Release(SoundHandle),
    StartMusic(SoundHandle, SoundHandle, Option<MusicOffset>),
    StopMusic,
}

impl AudioOutput {
    /// Open the default output device; a failure is logged, not swallowed,
    /// so a silent game names its cause.
    pub(super) fn start(config: MixerConfig) -> Option<Self> {
        let device = device::default_device();
        let opened = device
            .as_ref()
            .and_then(|device| OutputStream::try_from_device(device).ok());
        // rodio's own choice (the default, else the first device that opens) otherwise.
        let (device, (stream, output)) = match opened {
            Some(opened) => (device, opened),
            None => match OutputStream::try_default() {
                Ok(opened) => (None, opened),
                Err(error) => {
                    crate::log::progress(format_args!("audio output unavailable: {error}"));
                    return None;
                }
            },
        };
        let description = device::describe(device.as_ref().or(device::default_device().as_ref()));
        crate::log::progress(format_args!("audio output: {description}"));
        let (mut audio, source) = Self::assemble(config, description)?;
        let home = device::Home::default();
        if let Err(error) = output.play_raw(device::Handback::new(source, &home)) {
            crate::log::progress(format_args!("audio mixer could not start: {error}"));
            return None;
        }
        audio._stream = Some(stream);
        audio.follow = Some(device::Follow::new(device.as_ref(), home));
        Some(audio)
    }

    /// Every [`device::CHECK`]: move the mixer to the system's default output device
    /// when it changed (headphones plugged in) or the stream stopped rendering, as it
    /// is, without the sound reset of `snd_restart` (#210). Cheap between checks.
    pub(super) fn follow_default_device(&mut self, now: Instant) {
        let Some(follow) = self.follow.as_mut() else {
            return;
        };
        if !follow.due(now) {
            return;
        }
        let stalled = follow.stalled(self.stats.rendered_blocks.load(Ordering::Relaxed));
        // No device at all (the last one unplugged): keep what plays.
        let Some(default) = device::default_device() else {
            return;
        };
        // Devices compare by name (cpal's `Device` has no equality on every platform):
        // two identical headsets swapped for each other are not told apart.
        let name = device::name(&default);
        let moved = name.is_some() && follow.name != name;
        if !moved && !stalled {
            return;
        }
        // Dropping the stream ends its thread, which gives the mixer back.
        self._stream = None;
        let Some(source) = device::take_back(&follow.home) else {
            crate::log::progress(format_args!(
                "audio output: the mixer did not come back from the old device; \
                 snd_restart opens the output again"
            ));
            self.follow = None;
            return;
        };
        let played = match OutputStream::try_from_device(&default) {
            // A failed play drops the handback, which puts the mixer home again.
            Ok((stream, output)) => output
                .play_raw(device::Handback::new(source, &follow.home))
                .map(|()| stream)
                .map_err(|e| e.to_string()),
            Err(error) => {
                if let Ok(mut home) = follow.home.lock() {
                    *home = Some(source);
                }
                Err(error.to_string())
            }
        };
        match played {
            Ok(stream) => {
                self._stream = Some(stream);
                self.description = device::describe(Some(&default));
                crate::log::progress(format_args!(
                    "audio output {}: {}",
                    if moved { "moved" } else { "reopened" },
                    self.description
                ));
                follow.name = name;
            }
            // The mixer waits at home: tried again at the next check.
            Err(error) => crate::log::progress(format_args!(
                "audio output: could not open {}: {error}",
                device::describe(Some(&default))
            )),
        }
    }

    /// The queues, the decode worker and the source the device's callback pulls.
    fn assemble(config: MixerConfig, description: String) -> Option<(Self, MixerSource)> {
        let (command_producer, command_consumer) = HeapRb::new(COMMANDS).split();
        let (registration_producer, registration_consumer) = HeapRb::new(COMMANDS).split();
        let (released_producer, released_consumer) = HeapRb::new(RELEASES).split();
        let (retired_producer, retired_consumer) = HeapRb::new(RETIRED_RESPONSES).split();
        let (decode_sender, decode_receiver) = sync_channel::<DecodeJob>(DECODE_JOBS);
        let stats = Arc::new(OutputStats::default());
        let closing = Arc::new(AtomicBool::new(false));
        let decoder = {
            let stats = Arc::clone(&stats);
            let closing = Arc::clone(&closing);
            thread::Builder::new()
                .name("sjk-audio-decode".into())
                .spawn(move || {
                    decode(
                        &decode_receiver,
                        registration_producer,
                        released_consumer,
                        retired_consumer,
                        &stats,
                        &closing,
                    );
                })
                .ok()?
        };
        let source = MixerSource {
            mixer: Mixer::new(config),
            commands: command_consumer,
            registrations: registration_consumer,
            released: released_producer,
            retired: retired_producer,
            held: None,
            scratch: [0.0; MIX_SAMPLES],
            cursor: MIX_SAMPLES,
            running: true,
            stats: Arc::clone(&stats),
            pending: pending::PendingStarts::new(),
        };
        let audio = Self {
            _stream: None,
            description,
            commands: command_producer,
            feed: feed::Feed::default(),
            decode_jobs: Some(decode_sender),
            decoder: Some(decoder),
            closing,
            stats,
            show: false,
            started: std::collections::VecDeque::with_capacity(64),
            sonar: crate::sound_sonar::Sonar::default(),
            spatial: crate::audio::spatial::Tracker::default(),
            cache: std::cell::RefCell::new(Vec::new()),
            lengths: std::cell::RefCell::new(Vec::new()),
            follow: None,
            muted: Default::default(),
        };
        Some((audio, source))
    }

    /// Reuse the normal queues/decoder/mixer without opening a device in focused tests.
    #[cfg(test)]
    pub(super) fn test_output() -> (Self, impl Iterator<Item = f32>) {
        Self::test_source()
    }

    #[cfg(test)]
    fn test_source() -> (Self, MixerSource) {
        Self::assemble(
            MixerConfig {
                voices: VOICES,
                loops: LOOPS,
                sample_rate: SAMPLE_RATE,
                loop_attenuation: sjk_client::legacy_sound_attenuation(0),
                doppler: sjk_client::legacy_doppler_config(true),
                protected_channels: crate::audio::PROTECTED_CHANNELS,
            },
            "detached music test".into(),
        )
        .unwrap()
    }

    pub(super) fn send(&mut self, command: AudioCommand) {
        if let AudioCommand::Play(handle, ..)
        | AudioCommand::PlayFrom(handle, ..)
        | AudioCommand::PlayTrack(handle, ..)
        | AudioCommand::SetLoop(handle, ..) = &command
            && self.muted.mutes(handle.0)
        {
            return;
        }
        if self.show
            && let AudioCommand::Play(handle, _) = &command
        {
            if self.started.len() == 64 {
                self.started.pop_front();
            }
            self.started.push_back(*handle);
        }
        match &command {
            AudioCommand::Play(_, request) | AudioCommand::PlayFrom(_, request, ..) => {
                self.sonar.hear(request, Instant::now());
            }
            AudioCommand::ListenerSource(source) => self.sonar.listener_source = *source,
            AudioCommand::StopEffects | AudioCommand::StopAll => self.sonar.clear(),
            _ => {}
        }
        self.spatial.observe(&command, Instant::now());
        // Never wait for good on a callback that stopped (its device went away): the
        // game would stop answering. Every frame sends its state again.
        let pushed = self.feed.push(&mut self.commands, command, feed::PATIENCE);
        if pushed == (feed::Pushed::Dropped { stopped: true }) {
            crate::log::progress(format_args!(
                "audio output stopped taking sound (its device went away?); sounds are \
                 dropped until it resumes or snd_restart opens the output again"
            ));
        }
    }

    /// Cached decoded duration, or zero while unavailable/invalid. No file access or lock.
    pub(super) fn sample_length_ms(&self, handle: SoundHandle) -> u32 {
        self.lengths
            .borrow()
            .get(handle.0 as usize)
            .map_or(0, |length| length.load(Ordering::Relaxed))
    }

    fn reserve_length(&self, handle: SoundHandle) -> Arc<AtomicU32> {
        let mut lengths = self.lengths.borrow_mut();
        let needed = (handle.0 as usize + 1).max(lengths.len());
        lengths.resize_with(needed, || Arc::new(AtomicU32::new(0)));
        Arc::clone(&lengths[handle.0 as usize])
    }

    pub(super) fn decode(&self, handle: SoundHandle, bytes: &[u8], extension: &str) {
        self.cache
            .borrow_mut()
            .push((handle, bytes.len(), extension.to_owned()));
        let Some(sender) = &self.decode_jobs else {
            return;
        };
        let job = DecodeJob::Sound {
            handle,
            bytes: bytes.into(),
            extension: extension.into(),
            length_ms: self.reserve_length(handle),
        };
        if sender.try_send(job).is_err() {
            crate::log::progress(format_args!(
                "audio decode queue is full; dropping {handle:?}"
            ));
        }
    }

    /// Decode `bytes` on the worker, as [`Self::decode`] does, and register under `handle`
    /// what `make` makes of the samples (at [`SAMPLE_RATE`]) instead of the samples.
    pub(super) fn decode_variant(
        &self,
        handle: SoundHandle,
        bytes: &[u8],
        extension: &str,
        make: fn(&[f32], u32) -> Vec<f32>,
    ) {
        self.cache
            .borrow_mut()
            .push((handle, bytes.len(), extension.to_owned()));
        let Some(sender) = &self.decode_jobs else {
            return;
        };
        let job = DecodeJob::Variant {
            handle,
            bytes: bytes.into(),
            extension: extension.into(),
            make,
        };
        if sender.try_send(job).is_err() {
            crate::log::progress(format_args!(
                "audio decode queue is full; dropping {handle:?}"
            ));
        }
    }

    /// Free a decoded sound nothing will play again, such as a previous map's music.
    /// It travels the decode queue, so a sound still decoding is registered first; the
    /// callback stops what plays it and hands its PCM back to the decode worker, which
    /// frees it. A full queue keeps it decoded.
    pub(super) fn release(&self, handle: SoundHandle) {
        self.cache
            .borrow_mut()
            .retain(|(cached, ..)| *cached != handle);
        let Some(sender) = &self.decode_jobs else {
            return;
        };
        if sender.try_send(DecodeJob::Release(handle)).is_err() {
            crate::log::progress(format_args!(
                "audio decode queue is full; {handle:?} stays decoded"
            ));
        }
    }

    pub(super) fn stop_music(&self) {
        if let Some(sender) = &self.decode_jobs {
            let _ = sender.try_send(DecodeJob::StopMusic);
        }
    }

    pub(super) fn start_music(&self, intro: SoundHandle, repeating: SoundHandle) {
        self.queue_music(intro, repeating, None);
    }

    pub(super) fn start_music_at(
        &self,
        intro: SoundHandle,
        repeating: SoundHandle,
        elapsed_ms: u64,
    ) {
        self.queue_music(
            intro,
            repeating,
            Some(MusicOffset {
                elapsed_ms,
                queued_at: Instant::now(),
            }),
        );
    }

    fn queue_music(&self, intro: SoundHandle, repeating: SoundHandle, offset: Option<MusicOffset>) {
        let Some(sender) = &self.decode_jobs else {
            return;
        };
        if sender
            .try_send(DecodeJob::StartMusic(intro, repeating, offset))
            .is_err()
        {
            crate::log::progress(format_args!("audio decode queue is full; music deferred"));
        }
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.decode_jobs.take();
        self.closing.store(true, Ordering::Release);
        if let Some(decoder) = self.decoder.take() {
            let _ = decoder.join();
        }
        let _ = self.commands.try_push(AudioCommand::Shutdown);
    }
}

/// The decode worker: decode each job and queue its registration for the callback, in
/// order, until the jobs end or the output closes. It also frees the sounds the
/// callback released, between jobs and every [`RELEASE_POLL`] when idle.
fn decode(
    jobs: &Receiver<DecodeJob>,
    mut registrations: HeapProd<DecodedAction>,
    mut released: HeapCons<DecodedSound>,
    mut retired: HeapCons<Box<sjk_audio::RoomResponse>>,
    stats: &OutputStats,
    closing: &AtomicBool,
) {
    loop {
        let job = jobs.recv_timeout(RELEASE_POLL);
        free_released(&mut released, stats);
        while let Some(response) = retired.try_pop() {
            drop(response);
        }
        if closing.load(Ordering::Acquire) {
            return;
        }
        let job = match job {
            Ok(job) => job,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let action = match job {
            DecodeJob::Sound {
                handle,
                bytes,
                extension,
                length_ms,
            } => {
                let decoded = match decode_encoded(&bytes, &extension, SAMPLE_RATE) {
                    Ok(decoded) => {
                        let milliseconds = (decoded.samples().len() as u64 * 1000)
                            .div_ceil(u64::from(SAMPLE_RATE))
                            .min(u64::from(u32::MAX))
                            as u32;
                        length_ms.store(milliseconds, Ordering::Relaxed);
                        decoded
                    }
                    Err(_) => {
                        length_ms.store(0, Ordering::Relaxed);
                        stats.decode_failures.fetch_add(1, Ordering::Relaxed);
                        DecodedSound::silence()
                    }
                };
                DecodedAction::Register(handle, decoded)
            }
            DecodeJob::Variant {
                handle,
                bytes,
                extension,
                make,
            } => DecodedAction::Register(
                handle,
                match decode_mono(&bytes, &extension, SAMPLE_RATE) {
                    Ok(samples) => DecodedSound::from_samples(make(&samples, SAMPLE_RATE)),
                    Err(_) => {
                        stats.decode_failures.fetch_add(1, Ordering::Relaxed);
                        DecodedSound::silence()
                    }
                },
            ),
            DecodeJob::Release(handle) => DecodedAction::Release(handle),
            DecodeJob::StopMusic => DecodedAction::StopMusic,
            DecodeJob::StartMusic(intro, repeating, offset) => {
                DecodedAction::StartMusic(intro, repeating, offset)
            }
        };
        if !feed::push_until_closed(&mut registrations, action, closing, feed::PATIENCE) {
            return;
        }
    }
}

/// Drop the PCM of the sounds the callback released: their memory is freed on the
/// decode worker, never on the audio thread.
fn free_released(released: &mut HeapCons<DecodedSound>, stats: &OutputStats) {
    while let Some(sound) = released.try_pop() {
        stats
            .freed_bytes
            .fetch_add(sound.byte_len() as u64, Ordering::Relaxed);
        drop(sound);
    }
}

fn apply_command(mixer: &mut Mixer, command: AudioCommand) -> bool {
    match command {
        AudioCommand::Play(handle, request) => {
            mixer.play(handle, request);
        }
        AudioCommand::PlayFrom(handle, request, offset, align) => match align {
            // Half a second either way: a leader's voice further off is another
            // round of its loop, about to end or just begun.
            Some(align) => {
                mixer.play_aligned(handle, request, offset, align, SAMPLE_RATE as usize / 2);
            }
            None => {
                mixer.play_from(handle, request, offset);
            }
        },
        AudioCommand::PlayTrack(handle, request, elapsed_ms, queued_at) => {
            pending::start_track(
                mixer,
                handle,
                request,
                MusicOffset {
                    elapsed_ms,
                    queued_at,
                },
            );
        }
        AudioCommand::Listener {
            origin,
            forward,
            right,
            up,
            velocity,
        } => {
            mixer.listener(origin, forward, right, up);
            mixer.set_listener_velocity(velocity);
        }
        AudioCommand::ListenerSource(source) => mixer.set_listener_source(source),
        AudioCommand::SourcePosition(source, origin) => {
            mixer.update_source_position(source, origin);
        }
        AudioCommand::StopChannel(source, channel) => {
            mixer.stop_source_channel(source, channel);
        }
        AudioCommand::StopLoops(source) => {
            mixer.stop_source_loops(source);
        }
        AudioCommand::ClearLoops => {
            mixer.clear_loops();
        }
        AudioCommand::CommitLoops => mixer.commit_loops(),
        AudioCommand::StopAll => mixer.stop_all(),
        AudioCommand::StopEffects => mixer.stop_effects(),
        AudioCommand::SetLoop(handle, request, velocity) => {
            mixer.set_loop_request(handle, request, velocity);
        }
        AudioCommand::Gains(effects, music, voice, emotes) => {
            mixer.set_master_gain(effects);
            mixer.set_music_gain(music);
            mixer.set_channel_gain(crate::audio::VOICE_CHANNEL, Some(voice));
            mixer.set_channel_gain(crate::audio::EMOTE_CHANNEL, Some(emotes));
        }
        AudioCommand::Doppler(enabled) => {
            mixer.set_doppler_enabled(enabled);
        }
        AudioCommand::Separation(value) => mixer.set_separation(value),
        AudioCommand::SpatialEffects(enabled) => mixer.set_spatial_effects(enabled),
        AudioCommand::SpatialHeadphones(headphones) => mixer.set_spatial_headphones(headphones),
        AudioCommand::SourceAcoustics(source, acoustics) => {
            mixer.set_source_acoustics(source, acoustics);
        }
        AudioCommand::RoomResponse(response) => mixer.set_room_response(response),
        AudioCommand::ReverbGain(gain) => mixer.set_reverb_gain(gain),
        AudioCommand::Shutdown => return false,
    }
    true
}

#[cfg(test)]
mod stopped_stream_tests {
    use super::*;
    use std::sync::mpsc::channel;

    /// Long enough for the work below; the old endless waits never finished it.
    const LIMIT: Duration = Duration::from_secs(20);

    /// An output whose source no device pulls: the callback of a stream that ended.
    fn detached() -> (AudioOutput, MixerSource) {
        AudioOutput::assemble(
            MixerConfig {
                voices: VOICES,
                loops: LOOPS,
                sample_rate: SAMPLE_RATE,
                loop_attenuation: sjk_client::legacy_sound_attenuation(0),
                doppler: sjk_client::legacy_doppler_config(true),
                protected_channels: crate::audio::PROTECTED_CHANNELS,
            },
            "detached".into(),
        )
        .expect("decode worker starts")
    }

    #[test]
    fn decoded_sample_duration_is_published_without_an_audio_device() {
        let (output, _source) = detached();
        assert_eq!(output.sample_length_ms(SoundHandle(0)), 0);
        // Synthetic 10 ms, 44.1 kHz mono PCM WAV; no game assets.
        let samples = 441_u32;
        let size = samples * 2;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + size).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&size.to_le_bytes());
        wav.resize(44 + size as usize, 0);
        output.decode(SoundHandle(0), &wav, "wav");
        output.decode(SoundHandle(1), b"invalid", "wav");
        let start = Instant::now();
        while output.stats.decode_failures.load(Ordering::Relaxed) == 0 {
            assert!(start.elapsed() < LIMIT);
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(output.sample_length_ms(SoundHandle(0)), 10);
        assert_eq!(output.sample_length_ms(SoundHandle(1)), 0);
    }

    #[test]
    fn a_stopped_callback_does_not_hold_the_render_thread() {
        let (done, finished) = channel();
        std::thread::spawn(move || {
            let (mut output, mut source) = detached();
            // Several seconds of a live game's commands (a listener and gains each
            // frame, a position for each entity of each snapshot), far past the queue.
            for _ in 0..COMMANDS * 4 {
                output.send(AudioCommand::Gains(0.0, 0.0, 0.0, 0.0));
            }
            let stalled = output.feed.stalled();
            // A callback that drains the queue again (one mix block) ends the stall.
            let _ = source.next();
            output.send(AudioCommand::Gains(0.5, 0.25, 1.0, 1.0));
            let _ = done.send((stalled, output.feed.stalled()));
        });
        assert_eq!(finished.recv_timeout(LIMIT), Ok((true, false)));
    }

    #[test]
    fn dropping_the_output_does_not_wait_on_a_stopped_callback() {
        let (done, finished) = channel();
        std::thread::spawn(move || {
            let (output, source) = detached();
            let decoded = |count: u64| {
                let started = Instant::now();
                while output.stats.decode_failures.load(Ordering::Relaxed) < count {
                    assert!(started.elapsed() < LIMIT, "decoder stalled at {count}");
                    thread::sleep(Duration::from_millis(1));
                }
            };
            // Fill the registration queue nothing drains, then give the decoder one
            // more sound, which it holds waiting for room (`snd_restart`, quitting).
            for index in 0..COMMANDS {
                output.decode(SoundHandle(index as u32), b"none", "wav");
            }
            decoded(COMMANDS as u64);
            output.decode(SoundHandle(COMMANDS as u32), b"none", "wav");
            decoded(COMMANDS as u64 + 1);
            drop(output);
            drop(source);
            let _ = done.send(());
        });
        assert_eq!(finished.recv_timeout(LIMIT), Ok(()));
    }
}

#[cfg(test)]
mod music_clock_tests {
    use super::*;
    #[test]
    fn music_start_clock_compensates_queue_delay_without_leaking_to_other_starts() {
        let start = Instant::now();
        let offset = MusicOffset {
            elapsed_ms: 1234,
            queued_at: start,
        };
        assert_eq!(offset.position(start + Duration::from_millis(76)), 1310);
        assert_eq!(offset.position(start - Duration::from_millis(1)), 1234);
        assert_eq!(
            MusicOffset {
                elapsed_ms: u64::MAX,
                queued_at: start
            }
            .position(start + Duration::from_millis(1)),
            u64::MAX
        );
    }
}

#[cfg(test)]
mod sonar_tests {
    use super::*;

    fn play(source: u32) -> AudioCommand {
        AudioCommand::Play(
            SoundHandle(0),
            PlayRequest {
                origin: Some([300.0, 0.0, 0.0]),
                source: SourceId(source),
                channel: ChannelId(2),
                volume: 1.0,
                attenuation: sjk_client::legacy_sound_attenuation(2),
            },
        )
    }

    #[test]
    fn sent_world_sounds_reach_the_sonar_and_a_map_change_clears_it() {
        let (mut output, _source) = AudioOutput::test_output();
        output.sonar.enabled = true;
        output.send(AudioCommand::ListenerSource(Some(SourceId(1))));
        output.send(play(1));
        output.send(play(5));
        let count = || output.sonar.markers(Instant::now(), [0.0; 3], 0.0).count;
        assert_eq!(count(), 1, "the listener's own sound is left out");
        output.send(AudioCommand::StopEffects);
        assert_eq!(output.sonar.markers(Instant::now(), [0.0; 3], 0.0).count, 0);
    }
}
