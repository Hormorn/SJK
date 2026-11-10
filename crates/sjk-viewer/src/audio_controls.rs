//! Listener-facing playback controls and output diagnostics.
use super::*;

impl crate::GpuState {
    /// Update the process-lifetime listener and gains before completing this render frame.
    pub(crate) fn finish_audio_frame(
        &mut self,
        audio: &mut Option<GameAudio>,
        forward: glam::Vec3,
        right: glam::Vec3,
    ) {
        if let Some(audio) = audio {
            // `s_spatial`: heard from the player's head, turned as the camera is.
            let hearing = self.audio_hearing_point(audio.spatial_enabled());
            let up = right.cross(forward).normalize_or(glam::Vec3::Z);
            audio.listener(hearing, forward, right, up, glam::Vec3::ZERO);
            audio.sync_gains(self.console.as_ref());
            self.spatial_audio_frame(audio, hearing, [forward, right, up]);
            #[cfg(feature = "mod-jof")]
            {
                let dialogue = self
                    .live_session
                    .as_ref()
                    .filter(|_| self.live_presentation_ready())
                    .and(self.console.as_ref())
                    .filter(|console| console.server_dialogue_active())
                    .map(|console| console.dialogue_voice());
                audio.sync_dialogue(dialogue, self.vfs.as_deref());
            }
            #[cfg(feature = "mod-jof")]
            audio.sync_force_feedback(
                self.live_session.is_some()
                    && crate::mods::force_feedback::enabled(self.console.as_ref()),
            );
            if self.live_session.is_none() && self.demo_session.is_none() {
                audio.idle_frame();
            }
            audio.end_frame();
        }
    }
}

/// The command starting an emote's sound `offset` in. Without a voice to line up with it
/// waits for its decoder as a boombox track does, and starts as far in as the song is by
/// then: as a `Play` it was given up after a game event's 300 ms, so music still decoding
/// (a map's sounds ahead of it) left that whole dance silent (hub report #270).
fn emote_start(
    handle: SoundHandle,
    request: PlayRequest,
    offset: std::time::Duration,
    align: Option<u8>,
    now: Instant,
) -> AudioCommand {
    match align {
        Some(align) => AudioCommand::PlayFrom(
            handle,
            request,
            (offset.as_secs_f64() * f64::from(crate::audio_output::SAMPLE_RATE)) as usize,
            Some(SourceId(u32::from(align))),
        ),
        None => AudioCommand::PlayTrack(
            handle,
            request,
            u64::try_from(offset.as_millis()).unwrap_or(u64::MAX),
            now,
        ),
    }
}

/// The channel emote sounds play on: JKA's `CHAN_MUSIC`, which no entity sound of the
/// game uses, so an emote's music never cuts a voice or a swing. The mixer protects
/// it: a full voice table gives up every other sound before an emote's.
pub(crate) const EMOTE_CHANNEL: ChannelId = ChannelId(13);

/// The channel SJK's toys' one-shot sounds play on (the cardboard box's): past the
/// game's channels, so a toy's sound never cuts a voice, a swing or an emote.
pub(crate) const TOY_CHANNEL: ChannelId = ChannelId(14);

/// The channel boombox tracks play on: apart from [`EMOTE_CHANNEL`], whose gain is the
/// emotes' own (#163), so the boombox, a toy, keeps the effects volume.
pub(crate) const BOOMBOX_CHANNEL: ChannelId = ChannelId(15);

/// The channels of long music the mixer cuts last ([`sjk_audio::MixerConfig`]).
pub(crate) const PROTECTED_CHANNELS: &[ChannelId] =
    &[EMOTE_CHANNEL, BOOMBOX_CHANNEL, TOY_SHOW_CHANNEL];

/// The channel a Toy Master toy's show sound plays on: a song of minutes, kept like the
/// boombox's so a busy fight never steals it from a player who walks up later.
pub(crate) const TOY_SHOW_CHANNEL: ChannelId = ChannelId(16);

/// The channel every voice sound plays on: `CHAN_VOICE`, which `CHAN_VOICE_ATTEN` and
/// `CHAN_VOICE_GLOBAL` join (`sjk_client` `normalize_voice_channel`, as the game's
/// channel stomp groups them). Its sounds take `s_volumeVoice` in place of `s_volume`,
/// as retail's mixer gives them (`snd_mix.cpp` `S_PaintChannels`, `voice_vol`).
pub(crate) const VOICE_CHANNEL: ChannelId = ChannelId(3);

/// The `index`th of the interface's own channels (menu cues, previews): past the
/// game's channels, so none of them is [`VOICE_CHANNEL`] and takes the voice volume.
pub(crate) fn interface_channel(index: usize) -> ChannelId {
    ChannelId(64 + index as u32)
}

/// `s_volumeVoice`: the loudness of voices (pain, death, taunts, jump grunts, voice
/// commands), retail's name and default 1 (`snd_dma.cpp` `S_Init`).
pub(crate) const VOICE_VOLUME_CVAR: &str = "s_volumeVoice";

/// Which saber sounds a blade skin's wearer is heard with (its ignition, switching off,
/// swings and hum): 0 the stock saber sounds alone (the skin's are not played), 1 the
/// skin's over the stock ones (the default), 2 the skin's alone.
pub(crate) const SKIN_SOUNDS_CVAR: &str = "s_saberSkinSounds";

/// The sound `event` plays, `handle` being the adapter's choice: footsteps under
/// `cg_footsteps`, and a chat beep after the chat sound settings (`chat_sounds.rs`),
/// which mute its kind or play its chosen sound instead; `None` plays nothing. Combat
/// is never muted here. A chosen sound that is not loaded (missing from the game data)
/// leaves the adapter's.
pub(super) fn event_handle(
    event: sjk_client::LegacySoundEvent,
    handle: SoundHandle,
    footsteps: bool,
    chat: &crate::chat_sounds::Options,
    handles: &HashMap<String, SoundHandle>,
) -> Option<SoundHandle> {
    use crate::chat_sounds::{Kind, Route};
    if let Some(kind) = Kind::of_event(event) {
        return match chat.route(kind) {
            Route::Muted => None,
            Route::Default => Some(handle),
            Route::Sound(key) => Some(handles.get(key).copied().unwrap_or(handle)),
        };
    }
    (footsteps || !event.is_footstep()).then_some(handle)
}

impl GameAudio {
    /// Apply the live archived `s_volume`, `s_musicvolume`, `s_volumeVoice` and
    /// `s_emoteVolume` values (the emote channel's own gain, apart from `s_volume`,
    /// #163, so a change reaches the emotes already playing, #188; 0 in a duel with
    /// `s_muteEmotesInDuel`, #234); all 0 while the window is muted.
    pub(crate) fn set_gains(&mut self, effects: f32, music: f32, voice: f32, emotes: f32) {
        self.output
            .send(AudioCommand::Gains(effects, music, voice, emotes));
    }

    /// Pull the two codemp volume cvars through the existing shell registry.
    pub(crate) fn sync_gains(&mut self, console: Option<&ViewerConsole>) {
        self.jetpack_sounds = crate::jetpack::sound_options(console);
        self.flame.override_value = console
            .and_then(|c| c.integer_cvar(crate::actor_world_submission::force_powers::flame::CVAR))
            .unwrap_or(-1);
        self.holster_sync.enabled = console
            .and_then(|c| c.bool_cvar("cg_holsteredstaffsound"))
            .unwrap_or(true);
        let muted_words = console
            .and_then(|c| c.text_value(crate::audio::muted_sounds::CVAR))
            .unwrap_or("");
        self.output.muted.sync(
            muted_words,
            self.handles
                .iter()
                .map(|(path, handle)| (path.as_str(), handle.0)),
        );
        self.transitions.hits = console
            .and_then(|c| c.integer_cvar("cg_hitsounds"))
            .unwrap_or(0);
        self.transitions.duel = console
            .and_then(|c| c.integer_cvar("cg_duelsounds"))
            .unwrap_or(1);
        self.transitions.old_pain = console
            .and_then(|c| c.bool_cvar("cg_oldpainsounds"))
            .unwrap_or(false);
        self.voice_policy.sample(console);
        self.voice_throttle.enabled = console
            .and_then(|c| c.bool_cvar("cg_tauntantispam"))
            .unwrap_or(true);
        self.music_sync = console
            .and_then(|c| c.bool_cvar("cg_musicsync"))
            .unwrap_or(true);
        self.kill_sounds = console
            .and_then(|c| c.integer_cvar("cg_killsounds"))
            .unwrap_or(2);
        self.quiet_chat = console
            .and_then(|c| c.bool_cvar(crate::chat::quiet::CVAR))
            .unwrap_or(false);
        self.pm_only_chat = console
            .and_then(|c| c.bool_cvar("cg_pmonlychat"))
            .unwrap_or(false);
        if let Some(adapter) = &mut self.legacy {
            adapter.set_pm_only_chat(self.pm_only_chat);
            adapter.set_quiet_chat(self.quiet_chat);
            adapter.set_chat_sound_mode(
                console
                    .and_then(|c| c.integer_cvar("cg_chatsounds"))
                    .unwrap_or(1),
                console
                    .and_then(|c| c.integer_cvar("cg_cleanchatbox"))
                    .unwrap_or(0)
                    != 0,
            );
        }
        self.chat_sounds = crate::chat_sounds::Options::read(console);
        self.toy_sounds = crate::toy_sounds::Options::read(console);
        self.duel_enabled = console
            .and_then(|c| c.bool_cvar("cg_duelMusic"))
            .unwrap_or(true);
        self.output.send(AudioCommand::Separation(
            console
                .and_then(|c| c.float_cvar("s_separation"))
                .unwrap_or(0.5) as f32,
        ));
        self.output.show = console.and_then(|c| c.bool_cvar("s_show")).unwrap_or(false);
        if let Some(language) = console.and_then(|c| c.text_cvar(assets::LANGUAGE_CVAR).ok()) {
            assets::set_voice_language(language);
        }
        self.allow_dynamic = console
            .and_then(|c| c.bool_cvar("s_allowDynamicMusic"))
            .unwrap_or(true);
        let effects = console
            .and_then(|console| console.float_cvar("s_volume"))
            .unwrap_or(0.5) as f32;
        let music = console
            .and_then(|console| console.float_cvar("s_musicVolume"))
            .unwrap_or(0.25) as f32;
        let voice = console
            .and_then(|console| console.float_cvar(VOICE_VOLUME_CVAR))
            .unwrap_or(1.0) as f32;
        let emotes = crate::emotes::heard_volume(
            console
                .and_then(|console| console.float_cvar(crate::emotes::VOLUME_CVAR))
                .unwrap_or(crate::emotes::DEFAULT_VOLUME),
            console
                .and_then(|console| console.bool_cvar(crate::emotes::DUEL_MUTE_CVAR))
                .unwrap_or(crate::emotes::DEFAULT_DUEL_MUTE),
            self.in_duel,
        );
        if console.is_some_and(ViewerConsole::window_muted) {
            self.set_gains(0.0, 0.0, 0.0, 0.0);
        } else {
            self.set_gains(effects, music, voice, emotes);
        }
        self.footsteps = console
            .and_then(|console| console.bool_cvar("cg_footsteps"))
            .unwrap_or(true);
        let sonar = console
            .and_then(|console| console.bool_cvar(crate::sound_sonar::CVAR))
            .unwrap_or(false);
        if self.output.sonar.enabled && !sonar {
            self.output.sonar.clear();
        }
        self.output.sonar.enabled = sonar;
        self.output.spatial.enabled = console
            .and_then(|console| console.bool_cvar(spatial::CVAR))
            .unwrap_or(false);
        self.output.spatial.headphones = console
            .and_then(|console| console.bool_cvar(spatial::HEADPHONES_CVAR))
            .unwrap_or(true);
        self.output.spatial.reverb = console
            .and_then(|console| console.float_cvar(spatial::REVERB_CVAR))
            .unwrap_or(spatial::DEFAULT_REVERB)
            .clamp(0.0, 2.0) as f32;
        self.weather_sounds = console
            .and_then(|console| console.bool_cvar(crate::weather::sound::CVAR))
            .unwrap_or(false);
        let mode = console
            .and_then(|console| console.integer_cvar(SKIN_SOUNDS_CVAR))
            .unwrap_or(1);
        self.skin_sounds_off = mode <= 0;
        self.skin_sounds_only = mode >= 2;
        self.output.send(AudioCommand::Doppler(
            console
                .and_then(|console| console.bool_cvar("s_doppler"))
                .unwrap_or(true),
        ));
    }

    /// Whether 3D sound (`s_spatial`) is on, as of the last [`Self::sync_gains`].
    pub(crate) fn spatial_enabled(&self) -> bool {
        self.output.spatial.enabled
    }

    /// The world sounds sent lately, for the sound sonar (`sound_sonar.rs`).
    pub(crate) fn sonar(&self) -> &crate::sound_sonar::Sonar {
        &self.output.sonar
    }

    /// Keep background music fed while no live snapshot is producing loops; the loops
    /// the menus asked for this frame ([`Self::menu_loop`]) are the set.
    pub(crate) fn idle_frame(&mut self) {
        self.in_duel = false;
        self.stop_flame();
        self.poll_legacy_load();
        self.output.send(AudioCommand::ClearLoops);
        for (handle, request) in self.menu_loops.iter_mut().filter_map(Option::take) {
            self.output
                .send(AudioCommand::SetLoop(handle, request, [0.0; 3]));
        }
        self.output.send(AudioCommand::CommitLoops);
    }

    /// Loop `path` this frame, in both ears at `volume`, for `source`, while no match
    /// loops any sound (the Saber Lab's hum and buzz): asked again each frame it plays,
    /// it stops the first frame it is not. A sound not registered is not heard.
    pub(crate) fn menu_loop(&mut self, path: &str, volume: f32, source: SourceId) {
        let Some(handle) = self.find_handle(path) else {
            return;
        };
        if let Some(slot) = self.menu_loops.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some((
                handle,
                PlayRequest {
                    origin: None,
                    source,
                    channel: ChannelId(0),
                    volume,
                    attenuation: sjk_audio::Attenuation::None,
                },
            ));
        }
    }

    /// Lookup-only jetpack switch using the original choice and corrected attribution.
    pub(crate) fn play_jetpack_switch(&mut self, ignition: bool, origin: [f32; 3], wearer: u16) {
        // EJK #87: the actual wearer on CHAN_AUTO, never hardcoded player zero.
        let (path, request) = self.jetpack_sounds.switch_request(ignition, wearer, origin);
        if let Some(handle) = self.find_handle(path) {
            self.output.send(AudioCommand::Play(handle, request));
        }
    }

    /// Lookup-only EFX sound playback; `vfs` remains in the signature so old
    /// callers cannot accidentally bypass map asset ownership.
    pub(crate) fn play(
        &mut self,
        _vfs: &VirtualFileSystem,
        path: &str,
        volume: f32,
        origin: [f32; 3],
        source: SourceId,
    ) {
        self.play_on_channel(path, volume, origin, source, ChannelId(0));
    }

    /// Positional playback on a legacy channel. A non-zero channel replaces
    /// the sound `source` is still playing on it, like `S_PickChannel`.
    pub(crate) fn play_on_channel(
        &mut self,
        path: &str,
        volume: f32,
        origin: [f32; 3],
        source: SourceId,
        channel: ChannelId,
    ) {
        let Some(handle) = self.find_handle(path) else {
            return;
        };
        self.output.send(AudioCommand::Play(
            handle,
            PlayRequest {
                origin: Some(origin),
                source,
                channel,
                volume,
                attenuation: sjk_client::legacy_sound_attenuation(channel.0),
            },
        ));
    }

    /// Play a preloaded model animation cue with cgame channel/spatial semantics.
    pub(crate) fn play_animation(
        &mut self,
        path: &str,
        channel: u8,
        footstep: bool,
        origin: [f32; 3],
        entity: u64,
        local: bool,
    ) {
        if footstep && !self.footsteps {
            return;
        }
        // A muted player's footsteps, swings and other animation cues.
        let source = SourceId(entity.saturating_sub(1) as u32);
        if self.mute.silences(source, None) {
            return;
        }
        let Some(handle) = self.find_handle(path) else {
            return;
        };
        let relative = local || channel == 12;
        self.output.send(AudioCommand::Play(
            handle,
            PlayRequest {
                origin: (!relative).then_some(origin),
                source,
                channel: ChannelId(u32::from(if channel == 4 || channel == 12 {
                    3
                } else {
                    channel
                })),
                volume: 1.0,
                attenuation: if relative {
                    sjk_audio::Attenuation::None
                } else {
                    sjk_client::legacy_sound_attenuation(u32::from(channel))
                },
            },
        ));
    }

    /// Play an emote's sound on player `slot` from `offset` in (a synced emote's
    /// music joining), at the very sample player `align`'s voice plays it when they
    /// lead a synced emote, on the channel [`EMOTE_CHANNEL`], whose gain is
    /// `s_emoteVolume` ([`Self::sync_gains`]), so [`Self::stop_emote`] can cut it and a
    /// volume change reaches it while it plays (#188); a muted player's stays
    /// silent. Another player's is placed at `origin` and follows them (the snapshot
    /// moves their source); the player's own (`origin` `None`: their entity is not in
    /// the snapshot to move it) plays in both ears.
    pub(crate) fn play_emote(
        &mut self,
        path: &str,
        slot: u8,
        origin: Option<[f32; 3]>,
        offset: std::time::Duration,
        align: Option<u8>,
    ) {
        const VOLUME: f32 = 1.0;
        let source = SourceId(u32::from(slot));
        if self.mute.silences(source, None) {
            return;
        }
        let Some(handle) = self.find_handle(path) else {
            return;
        };
        let request = PlayRequest {
            origin,
            source,
            channel: EMOTE_CHANNEL,
            volume: VOLUME,
            attenuation: match origin {
                Some(_) => sjk_client::legacy_sound_attenuation(EMOTE_CHANNEL.0),
                None => sjk_audio::Attenuation::None,
            },
        };
        self.output
            .send(emote_start(handle, request, offset, align, Instant::now()));
    }

    /// Stop the emote sound player `slot` plays.
    pub(crate) fn stop_emote(&mut self, slot: u8) {
        self.output.send(AudioCommand::StopChannel(
            SourceId(u32::from(slot)),
            EMOTE_CHANNEL,
        ));
    }

    /// Play a sound of `toy` at `volume` times its toy volume, placed at `origin`
    /// (`None`: in both ears) on `channel`; nothing when the settings silence the toy.
    pub(crate) fn play_toy(
        &mut self,
        toy: crate::toy_sounds::Toy,
        path: &str,
        volume: f32,
        origin: Option<[f32; 3]>,
        source: SourceId,
        channel: ChannelId,
    ) {
        let Some(volume) = self.toy_sounds.gain(toy, volume) else {
            return;
        };
        match origin {
            Some(origin) => self.play_on_channel(path, volume, origin, source, channel),
            None => self.play_local(path, volume, source, channel),
        }
    }

    /// [`Self::play_toy`] at `origin` with its own `attenuation`, for a toy sound that
    /// must carry farther than the game's own (the Monitor's knocks on the world).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn play_toy_reaching(
        &mut self,
        toy: crate::toy_sounds::Toy,
        path: &str,
        volume: f32,
        origin: [f32; 3],
        source: SourceId,
        channel: ChannelId,
        attenuation: sjk_audio::Attenuation,
        elapsed_ms: u64,
    ) {
        let Some(volume) = self.toy_sounds.gain(toy, volume) else {
            return;
        };
        let Some(handle) = self.find_handle(path) else {
            return;
        };
        let request = PlayRequest {
            origin: Some(origin),
            source,
            channel,
            volume,
            attenuation,
        };
        // A sound joined after it started plays from as far in (the boombox's way).
        self.output.send(if elapsed_ms == 0 {
            AudioCommand::Play(handle, request)
        } else {
            AudioCommand::PlayTrack(handle, request, elapsed_ms, std::time::Instant::now())
        });
    }

    /// Stop the toy sound playing on `source`'s `channel`.
    pub(crate) fn stop_toy(&mut self, source: SourceId, channel: ChannelId) {
        self.output.send(AudioCommand::StopChannel(source, channel));
    }

    /// Play a listener-relative sound (interface cues) at full stereo.
    pub(crate) fn play_local(
        &mut self,
        path: &str,
        volume: f32,
        source: SourceId,
        channel: ChannelId,
    ) {
        let Some(handle) = self.find_handle(path) else {
            return;
        };
        self.output.send(AudioCommand::Play(
            handle,
            PlayRequest {
                origin: None,
                source,
                channel,
                volume,
                attenuation: sjk_audio::Attenuation::None,
            },
        ));
    }

    /// Per-frame tail: play the interface cues the menus posted, then
    /// report output health.
    pub(crate) fn end_frame(&mut self) {
        ui_cues::play_pending(self);
        self.report_underruns();
        self.output.follow_default_device(Instant::now());
    }

    pub(crate) fn report_underruns(&mut self) {
        let count = self.underrun_count();
        if count != self.reported_underruns {
            crate::log::progress(format_args!("audio output underruns: {count}"));
            self.reported_underruns = count;
        }
        self.trace_level();
    }

    pub(crate) fn underrun_count(&self) -> u64 {
        self.output.stats.underruns.load(Ordering::Relaxed)
    }

    /// Every few seconds, what the mixer actually rendered: a peak of zero
    /// with music started means the mixer is silent; a peak above zero with
    /// nothing audible means the sound leaves the process and is lost after.
    fn trace_level(&mut self) {
        let Some(due) = self.trace_due else { return };
        let now = Instant::now();
        if now < due {
            return;
        }
        self.trace_due = Some(now + TRACE_INTERVAL);
        let stats = &self.output.stats;
        crate::log::progress(format_args!(
            "audio trace: peak={:.3} blocks={} music starts={} rejected={} \
             decode failures={} handle mismatches={} split loop frames={} sounds={} \
             pcm={:.1}MiB freed={:.1}MiB kept={}",
            stats.take_peak(),
            stats.rendered_blocks.load(Ordering::Relaxed),
            stats.music_starts.load(Ordering::Relaxed),
            stats.music_rejections.load(Ordering::Relaxed),
            stats.decode_failures.load(Ordering::Relaxed),
            stats.handle_mismatches.load(Ordering::Relaxed),
            stats.split_loop_frames.load(Ordering::Relaxed),
            self.handles.len(),
            stats.pcm_bytes.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0),
            stats.freed_bytes.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0),
            stats.releases_kept.load(Ordering::Relaxed),
        ));
    }
}

#[cfg(test)]
mod listener_tests {
    use super::*;
    use sjk_protocol::PlayerState;

    /// A constant mono WAV, so any difference between the ears is the pan.
    fn constant_wav() -> Vec<u8> {
        let samples = 44100_u32;
        let size = samples * 2;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + size).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&44100_u32.to_le_bytes());
        wav.extend_from_slice(&88200_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&size.to_le_bytes());
        for _ in 0..samples {
            wav.extend_from_slice(&8192_i16.to_le_bytes());
        }
        wav
    }

    /// Hub report #99: a restarted output is a new generation, so sounds registered
    /// once per output (emotes, toys) are registered with it again.
    #[test]
    fn every_output_is_a_new_generation() {
        let first = GameAudio::with_output(AudioOutput::test_output().0);
        let second = GameAudio::with_output(AudioOutput::test_output().0);
        assert_ne!(first.generation(), second.generation());
    }

    #[test]
    fn a_snapshot_leaves_the_listener_where_the_frame_put_it() {
        let (output, mut source) = AudioOutput::test_output();
        let mut audio = GameAudio::with_output(output);
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory("synthetic", [("sound/hum.wav", constant_wav())])
            .unwrap();
        let hum = audio.register_vfs_async(&vfs, "sound/hum.wav").unwrap();
        // The frame's listener, at the view, and the local saber's hum on it.
        audio.listener(Vec3::ZERO, Vec3::X, Vec3::NEG_Y, Vec3::Z, Vec3::ZERO);
        // Sent again until the hum is decoded and sounds, as frames send their loops.
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        loop {
            audio.output.send(AudioCommand::SetLoop(
                hum,
                PlayRequest {
                    origin: Some([0.0; 3]),
                    source: SourceId(0),
                    channel: ChannelId(0),
                    volume: 1.0,
                    attenuation: sjk_client::legacy_sound_attenuation(0),
                },
                [0.0; 3],
            ));
            if (0..2048).any(|_| source.next().unwrap().abs() > 0.001) {
                break;
            }
            assert!(Instant::now() < deadline, "the hum did not start");
            std::thread::yield_now();
        }
        // A snapshot whose player stands 200 units to the left of the view: the hum
        // would sound in the right ear alone from there.
        let mut player = PlayerState::zero();
        player.set_origin([0.0, 200.0, 0.0]);
        let snapshot = Snapshot {
            message_sequence: 1,
            reliable_acknowledge: 0,
            server_commands: vec![],
            server_time: 1000,
            delta_from: None,
            flags: 0,
            area_mask: vec![],
            player,
            vehicle_player: None,
            entities: vec![],
            consumed_bits: 0,
        };
        audio.observe_snapshot(&snapshot, &GameState::empty_local(0), &vfs);
        for _ in 0..8192 {
            let (left, right) = (source.next().unwrap(), source.next().unwrap());
            assert!(
                left > 0.001 && (left - right).abs() < 1e-4,
                "{left} {right}"
            );
        }
    }
    /// Hub report #141 (Rollembones): `s_volumeVoice` did nothing. Voices (the voice
    /// channel) now take it in place of `s_volume`, as retail's mixer gives them; the
    /// interface's own sounds and every other channel keep `s_volume`.
    #[test]
    fn voices_take_the_voice_volume_and_other_sounds_the_effects_volume() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(console.set_cvar("s_volume", "0.5"));
        assert!(console.set_cvar(VOICE_VOLUME_CVAR, "0.25"));
        // The sample's level, 8192 of 32768, at each gain, in both ears.
        for (channel, gain) in [
            (VOICE_CHANNEL, 0.25),
            // Channel 0 never replaces its sound, so the sends below would add up.
            (ChannelId(2), 0.5),
            (interface_channel(3), 0.5),
        ] {
            let (output, mut source) = AudioOutput::test_output();
            let mut audio = GameAudio::with_output(output);
            audio.sync_gains(Some(&console));
            let mut vfs = VirtualFileSystem::new();
            vfs.mount_memory("synthetic", [("sound/voice.wav", constant_wav())])
                .unwrap();
            let sound = audio.register_vfs_async(&vfs, "sound/voice.wav").unwrap();
            let request = PlayRequest {
                origin: None,
                source: SourceId(1),
                channel,
                volume: 1.0,
                attenuation: sjk_audio::Attenuation::None,
            };
            // Sent again until the sound is decoded and sounds.
            let deadline = Instant::now() + std::time::Duration::from_secs(10);
            let level = loop {
                audio.output.send(AudioCommand::Play(sound, request));
                if let Some(level) = (0..2048)
                    .map(|_| source.next().unwrap())
                    .find(|sample| sample.abs() > 0.001)
                {
                    break level;
                }
                assert!(Instant::now() < deadline, "the sound did not start");
                std::thread::yield_now();
            };
            let want = 0.25 * gain;
            assert!(
                (level - want).abs() < 1e-3,
                "{channel:?}: {level} against {want}"
            );
        }
    }

    /// Hub report #188 (KãTsumøTø): lowering `s_emoteVolume` left the emotes already
    /// playing as loud until their player was far away. The emote channel's gain is
    /// `s_emoteVolume` itself, so a change reaches a sound while it plays, louder than
    /// recorded above 1.
    #[test]
    fn an_emote_playing_follows_the_emote_volume() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(console.set_cvar("s_volume", "0.5"));
        assert!(console.set_cvar(crate::emotes::VOLUME_CVAR, "2"));
        let (output, mut source) = AudioOutput::test_output();
        let mut audio = GameAudio::with_output(output);
        audio.sync_gains(Some(&console));
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory("synthetic", [("sound/emote.wav", constant_wav())])
            .unwrap();
        audio.register_vfs_async(&vfs, "sound/emote.wav").unwrap();
        // Started again until the sound is decoded and sounds.
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        let level = loop {
            audio.play_emote("sound/emote.wav", 1, None, std::time::Duration::ZERO, None);
            if let Some(level) = (0..2048)
                .map(|_| source.next().unwrap())
                .find(|sample| sample.abs() > 0.001)
            {
                break level;
            }
            assert!(Instant::now() < deadline, "the sound did not start");
            std::thread::yield_now();
        };
        assert!((level - 0.5).abs() < 1e-3, "{level} against 0.5");
        // Turned down while it plays: silent from the next mix block, not restarted.
        assert!(console.set_cvar(crate::emotes::VOLUME_CVAR, "0"));
        audio.sync_gains(Some(&console));
        let _ = (0..4096).map(|_| source.next().unwrap()).count();
        let loudest = (0..4096)
            .map(|_| source.next().unwrap().abs())
            .fold(0.0_f32, f32::max);
        assert!(loudest < 1e-4, "still heard at {loudest}");
    }

    /// Hub report #270: an emote's music waits for its decoder.
    #[test]
    fn an_emotes_music_waits_for_its_decoder() {
        let request = PlayRequest {
            origin: None,
            source: SourceId(1),
            channel: EMOTE_CHANNEL,
            volume: 1.0,
            attenuation: sjk_audio::Attenuation::None,
        };
        let now = Instant::now();
        let second = std::time::Duration::from_secs(1);
        assert!(matches!(
            emote_start(
                SoundHandle(1),
                request,
                std::time::Duration::ZERO,
                None,
                now
            ),
            AudioCommand::PlayTrack(_, _, 0, _)
        ));
        assert!(matches!(
            emote_start(SoundHandle(1), request, second, None, now),
            AudioCommand::PlayTrack(_, _, 1000, _)
        ));
        // A follower lines up with its leader's voice.
        assert!(matches!(
            emote_start(SoundHandle(1), request, second, Some(3), now),
            AudioCommand::PlayFrom(_, _, _, Some(SourceId(3)))
        ));
    }

    /// Hub report #234 (Groark): with `s_muteEmotesInDuel` on, an emote playing falls
    /// silent when the viewed player's private duel starts (the snapshot's
    /// `duelInProgress`) and is heard again once it ends.
    #[test]
    fn an_emote_playing_is_silent_in_a_duel_with_the_setting() {
        fn loudest(source: &mut impl Iterator<Item = f32>) -> f32 {
            let _ = (0..4096).map(|_| source.next().unwrap()).count();
            (0..4096)
                .map(|_| source.next().unwrap().abs())
                .fold(0.0_f32, f32::max)
        }
        let snapshot = |duelling: bool| {
            let mut player = PlayerState::zero();
            // `duelInProgress` (netfield 119).
            player.set_raw_field(119, u32::from(duelling));
            Snapshot {
                message_sequence: 1,
                reliable_acknowledge: 0,
                server_commands: vec![],
                server_time: 1000,
                delta_from: None,
                flags: 0,
                area_mask: vec![],
                player,
                vehicle_player: None,
                entities: vec![],
                consumed_bits: 0,
            }
        };
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(console.set_cvar(crate::emotes::VOLUME_CVAR, "1"));
        let (output, mut source) = AudioOutput::test_output();
        let mut audio = GameAudio::with_output(output);
        audio.sync_gains(Some(&console));
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory("synthetic", [("sound/emote.wav", constant_wav())])
            .unwrap();
        audio.register_vfs_async(&vfs, "sound/emote.wav").unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        loop {
            audio.play_emote("sound/emote.wav", 1, None, std::time::Duration::ZERO, None);
            if (0..2048).any(|_| source.next().unwrap().abs() > 0.001) {
                break;
            }
            assert!(Instant::now() < deadline, "the sound did not start");
            std::thread::yield_now();
        }
        let game = GameState::empty_local(0);
        // In a duel with the setting off (its default): still heard.
        audio.observe_snapshot(&snapshot(true), &game, &vfs);
        audio.sync_gains(Some(&console));
        assert!(loudest(&mut source) > 0.1);
        // On: silent while the duel lasts.
        assert!(console.set_cvar(crate::emotes::DUEL_MUTE_CVAR, "1"));
        audio.sync_gains(Some(&console));
        let heard = loudest(&mut source);
        assert!(heard < 1e-4, "heard in the duel at {heard}");
        // The duel over: the same sound is heard again, not restarted.
        audio.observe_snapshot(&snapshot(false), &game, &vfs);
        audio.sync_gains(Some(&console));
        assert!(loudest(&mut source) > 0.1);
    }
}

#[cfg(test)]
mod chat_handle_tests {
    use super::*;
    use crate::chat_sounds::Options;
    use sjk_client::LegacySoundEvent;

    #[test]
    fn a_chat_beep_plays_its_kind_s_chosen_sound_or_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut handles = HashMap::new();
        handles.insert("sound/interface/update.mp3".to_owned(), SoundHandle(7));
        let beep = SoundHandle(1);
        let handle = |event, options: &Options, handles: &HashMap<String, SoundHandle>| {
            event_handle(event, beep, true, options, handles)
        };
        // Defaults: the adapter's beep for every kind.
        let options = Options::read(Some(&console));
        for event in [
            LegacySoundEvent::ChatBeep,
            LegacySoundEvent::TeamChatBeep,
            LegacySoundEvent::PrivateChatBeep,
        ] {
            assert_eq!(handle(event, &options, &handles), Some(beep));
        }
        // Public chat off, private messages on a chosen sound (Turin's case).
        assert!(console.set_cvar("cg_chatBeep", "0"));
        assert!(console.set_cvar("cl_chatSoundPrivate", "blip"));
        let options = Options::read(Some(&console));
        assert_eq!(handle(LegacySoundEvent::ChatBeep, &options, &handles), None);
        assert_eq!(
            handle(LegacySoundEvent::PrivateChatBeep, &options, &handles),
            Some(SoundHandle(7))
        );
        // A chosen sound not loaded leaves the adapter's beep.
        assert_eq!(
            handle(LegacySoundEvent::PrivateChatBeep, &options, &HashMap::new()),
            Some(beep)
        );
        // Combat and footsteps are not chat.
        assert_eq!(
            handle(LegacySoundEvent::Taunt, &options, &handles),
            Some(beep)
        );
        let footstep = LegacySoundEvent::Footstep;
        assert_eq!(
            event_handle(footstep, beep, false, &options, &handles),
            None
        );
    }
}
