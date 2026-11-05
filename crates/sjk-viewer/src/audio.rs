//! Platform audio boundary: one software mixer feeding one rodio stream.
//!
//! Asset reads and decoding happen on workers. The rodio source owns the mixer
//! and receives bounded SPSC commands, so rendering cannot starve audio and
//! the platform callback never takes a mutex. The only remaining drop-out
//! mode is the callback overrunning its real-time budget; `underruns` counts
//! mix blocks that took longer than the block duration to render.

pub(crate) mod sjk_chat_sound;
pub(crate) mod ui_cues;

use crate::audio_output::{AudioCommand, AudioOutput, LOOPS, SAMPLE_RATE, VOICES};
use crate::console::ViewerConsole;
use sjk_audio::{ChannelId, MixerConfig, PlayRequest, SoundHandle, SourceId};
use sjk_bsp::Bsp;
use sjk_client::{
    LegacyMaintainedActionKind, LegacyMusicAction, LegacySaberView, LegacySoundAdapter,
    legacy_evaluate_trajectory,
};
use sjk_protocol::{EntityState, GameState, Snapshot};
use sjk_runtime::{EntityId, World};
use sjk_vfs::VirtualFileSystem;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, atomic::Ordering};
use std::time::Instant;

#[path = "audio_assets.rs"]
mod assets;
#[path = "audio_boombox.rs"]
mod boombox;
#[path = "audio_commands.rs"]
mod commands;
#[path = "audio_controls.rs"]
mod controls;
#[cfg(feature = "mod-jof")]
#[path = "jof_dialogue_audio.rs"]
mod dialogue;
pub(crate) use controls::{
    BOOMBOX_CHANNEL, EMOTE_CHANNEL, PROTECTED_CHANNELS, SKIN_SOUNDS_CVAR, TOY_CHANNEL,
    TOY_SHOW_CHANNEL, VOICE_CHANNEL, VOICE_VOLUME_CVAR, interface_channel,
};
#[path = "audio_dynamic.rs"]
mod dynamic;
#[path = "audio_feedback.rs"]
mod feedback;
/// `cg_hitsounds`' sounds made in memory when no pack brings them.
mod hit_sounds;
#[path = "audio_holstered_staff.rs"]
mod holstered_staff;
#[cfg(feature = "mod-jof")]
#[path = "jof_force_audio.rs"]
mod jof_force;
#[path = "audio_music.rs"]
pub(crate) mod music;
pub(crate) mod muted_sounds;
#[path = "audio_spatial.rs"]
pub(crate) mod spatial;
#[path = "audio_transitions.rs"]
mod transitions;
#[path = "audio_voice_throttle.rs"]
mod voice_throttle;
pub(crate) use assets::SoundPrefetch;
#[path = "flame_audio.rs"]
mod flame;
#[path = "audio_legacy.rs"]
mod legacy;

use legacy::LegacyLoadTask;
#[path = "audio_config_strings.rs"]
mod config_strings;
use config_strings::{DeferredSoundChanges, SoundTableRefresh};

const SOLID_BMODEL: u32 = 0x00ff_ffff; // codemp/qcommon/q_shared.h:1540

/// The next [`GameAudio::generation`].
static NEXT_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Process-lifetime audio state. Only the rodio callback owns the mixer.
pub(crate) struct GameAudio {
    flame: flame::State,
    #[cfg(feature = "mod-jof")]
    dialogue: dialogue::Voice,
    #[cfg(feature = "mod-jof")]
    force_feedback: jof_force::State,
    output: AudioOutput,
    /// This output's number among those the process started ([`Self::generation`]).
    generation: u64,
    reported_underruns: u64,
    handles: HashMap<String, SoundHandle>,
    /// Effect paths no installed file answered ([`Self::register_effect`]), until the
    /// next gamestate brings new content.
    missing_effects: std::collections::HashSet<String>,
    next_handle: u32,
    legacy: Option<LegacySoundAdapter>,
    legacy_load: Option<LegacyLoadTask>,
    sound_table_refresh: Option<SoundTableRefresh>,
    legacy_vfs: Option<Arc<VirtualFileSystem>>,
    map_music: Option<(SoundHandle, SoundHandle)>,
    /// Music decoded for the current selection, outside `handles`, so it can be freed
    /// when the music changes (`audio_assets.rs`).
    music_tracks: Vec<assets::MusicTrack>,
    duel_music: Option<SoundHandle>,
    deferred_snapshots: VecDeque<(Instant, Snapshot, DeferredSoundChanges)>,
    music_value: Option<Vec<u8>>,
    dynamic: Option<dynamic::Level>,
    allow_dynamic: bool,
    duel_enabled: bool,
    /// The viewed player is in a duel ([`crate::duel_blades::in_duel`]), as of the last
    /// snapshot: the emotes fall silent with `s_muteEmotesInDuel` ([`Self::sync_gains`]).
    in_duel: bool,
    music_sync: bool,
    music_sync_pending: bool,
    /// `cgs.musicStartTime`: server time the current `CS_MUSIC` track began.
    music_start_time: i32,
    /// The last observed snapshot's server time, for a `CS_MUSIC` change.
    last_server_time: Option<i32>,
    /// The chat sound settings (`chat_sounds.rs`), read once a frame.
    pub(crate) chat_sounds: crate::chat_sounds::Options,
    pm_only_chat: bool,
    /// Quiet mode (`chat/quiet.rs`): the sound adapter keeps its chat beeps to its rule.
    quiet_chat: bool,
    deferred_starts: u64,
    maximum_deferred_start_ms: f64,
    /// `cg_footsteps`: whether the footstep event family is voiced.
    footsteps: bool,
    /// `s_saberSkinSounds` 2: a blade skin's sounds replace the stock saber sounds.
    pub(crate) skin_sounds_only: bool,
    /// `s_saberSkinSounds` 0: a blade skin's sounds are not played, the stock ones are.
    pub(crate) skin_sounds_off: bool,
    voice_policy: feedback::VoicePolicy,
    voice_throttle: voice_throttle::VoiceThrottle,
    kill_sounds: i64,
    transitions: transitions::Transitions,
    jetpack_sounds: sjk_client::LegacyJetpackSounds,
    holster_sync: holstered_staff::Sync,
    /// `SJK_TRACE_AUDIO=1`: when the next level line is due.
    trace_due: Option<Instant>,
    /// Players muted on this PC, whose sounds are not played (`audio_mute.rs`).
    mute: crate::audio_mute::AudioMute,
    /// The blade skins whose sounds the gamestate's tables get (`saber_skins.rs`).
    blade_skins: Arc<crate::saber_skins::LoadedSkins>,
    /// The generation of the blade skins registered in `legacy`, once it is ready.
    saber_sets_generation: Option<u64>,
    /// The cardboard box's music on each boxed player this frame (`cardboard_box.rs`):
    /// their slot, and where they stand (`None`: the player's own, in both ears).
    box_music: Vec<(SourceId, Option<[f32; 3]>)>,
    /// The Hax toy's hacking sound this frame (`hax.rs`): the hacker's slot, and where
    /// they stand (`None`: the player's own, in both ears).
    hax_loop: Option<(SourceId, Option<[f32; 3]>)>,
    /// The Prop gun's flying bolts' loops this frame (source, sound, where).
    propgun_loops: Vec<(SourceId, String, [f32; 3])>,
    /// The toys' volume and per-toy switches (`toy_sounds.rs`), read each frame.
    toy_sounds: crate::toy_sounds::Options,
    /// `cg_fleshSparks` ([`crate::impacts::flesh_sparks`]): below 2 the special saber
    /// hit is silent, as in JoF EJK.
    flesh_sparks: u8,
    /// The loops asked for this frame with no match looping any ([`Self::menu_loop`]):
    /// the Saber Lab's hum and buzz.
    menu_loops: [Option<(SoundHandle, PlayRequest)>; MENU_LOOPS],
    /// The blade skins' crackle buzzes heard last frame (`hear_sabers`), looped in this
    /// frame's set, and the dice for which snap sounds.
    crackle_buzzes: Vec<crate::saber_smoke::Sound>,
    crackle_roll: u32,
}

/// The source the blade skins' snaps play from: no entity's, so they stay where they
/// went (an entity's source follows it), and are muted with the player's own.
const CRACKLE_SOURCE: SourceId = SourceId(0x6200_0000);

/// The most snaps heard a frame.
const SNAPS_A_FRAME: usize = 4;

/// The most loops the menus ask for at once.
const MENU_LOOPS: usize = 4;

impl GameAudio {
    /// Set `cg_fleshSparks` for the snapshots observed next.
    pub(crate) fn set_flesh_sparks(&mut self, level: u8) {
        self.flesh_sparks = level;
    }

    /// Feed immediate local movement through the snapshot sound resolver and mixer.
    pub(crate) fn observe_predicted_event(
        &mut self,
        event: sjk_client::predicted_events::PredictedEvent,
        snapshot: &Snapshot,
    ) {
        let Some(adapter) = &mut self.legacy else {
            return;
        };
        adapter.observe_predicted_event(event, snapshot);
        self.voice_throttle.observe_time(snapshot.server_time);
        for decision in adapter.decisions() {
            if let Some(handle) = decision.handle
                && let Some(handle) = controls::event_handle(
                    decision.event,
                    handle,
                    self.footsteps,
                    &self.chat_sounds,
                    &self.handles,
                )
                && self
                    .voice_policy
                    .allows(*decision, snapshot.player.client_num())
                && self.voice_throttle.allows(
                    decision.request.source.0,
                    decision.voice_line,
                    snapshot.server_time,
                    self.output.sample_length_ms(handle),
                )
            {
                self.output
                    .send(AudioCommand::Play(handle, decision.request));
            }
        }
    }
    /// Which blade skin's sound set each client slot wears (`saber_skins.rs`); cheap,
    /// called every frame. Until the gamestate's tables are ready there is nothing to set.
    pub(crate) fn set_saber_sound_overrides(
        &mut self,
        clients: &[Option<u8>; sjk_client::SABER_SOUND_CLIENTS],
    ) {
        if let Some(adapter) = &mut self.legacy {
            // With the skins' sounds off, nobody wears a sound set.
            const NONE: [Option<u8>; sjk_client::SABER_SOUND_CLIENTS] =
                [None; sjk_client::SABER_SOUND_CLIENTS];
            adapter.set_saber_sound_overrides(if self.skin_sounds_off { &NONE } else { clients });
            adapter.set_saber_skin_sounds_only(self.skin_sounds_only);
        }
    }

    /// Play `paths` once each, together, at full stereo on the interface's own source
    /// (a channel each, so one does not cut another): the Saber tab's preview of a sound
    /// set ([`crate::saber_sound_choice`]). A path not in `vfs` is left out.
    pub(crate) fn play_preview(&mut self, vfs: &VirtualFileSystem, paths: &[&str]) {
        for (channel, path) in paths.iter().enumerate() {
            if let Some(handle) = self.register_vfs_async(vfs, path) {
                self.output.send(AudioCommand::Play(
                    handle,
                    PlayRequest {
                        origin: None,
                        source: SourceId(u32::MAX - 2),
                        channel: interface_channel(channel),
                        volume: 0.9,
                        attenuation: sjk_audio::Attenuation::None,
                    },
                ));
            }
        }
    }

    /// Which output this is: a new number at every start (`snd_restart` makes a new
    /// output with an empty sample cache), so sounds registered once per output
    /// (emotes, toys) know to register again.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Create the one process-lifetime output stream and its decode worker.
    pub(crate) fn start() -> Option<Self> {
        let output = AudioOutput::start(MixerConfig {
            voices: VOICES,
            loops: LOOPS,
            sample_rate: SAMPLE_RATE,
            loop_attenuation: sjk_client::legacy_sound_attenuation(0),
            doppler: sjk_client::legacy_doppler_config(true),
            protected_channels: crate::audio::PROTECTED_CHANNELS,
        })?;
        Some(Self::with_output(output))
    }

    /// Assemble audio state around a real or test-owned null output.
    pub(super) fn with_output(output: AudioOutput) -> Self {
        let mut audio = Self {
            flame: Default::default(),
            #[cfg(feature = "mod-jof")]
            dialogue: Default::default(),
            #[cfg(feature = "mod-jof")]
            force_feedback: Default::default(),
            output,
            generation: NEXT_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            reported_underruns: 0,
            handles: HashMap::with_capacity(2_048),
            missing_effects: std::collections::HashSet::new(),
            next_handle: 0,
            legacy: None,
            legacy_load: None,
            sound_table_refresh: None,
            legacy_vfs: None,
            blade_skins: Arc::default(),
            saber_sets_generation: None,
            box_music: Vec::with_capacity(8),
            hax_loop: None,
            propgun_loops: Vec::with_capacity(8),
            toy_sounds: Default::default(),
            flesh_sparks: crate::impacts::FLESH_SPARKS_ALL,
            menu_loops: [None; MENU_LOOPS],
            crackle_buzzes: Vec::with_capacity(16),
            crackle_roll: 0,
            map_music: None,
            music_tracks: Vec::with_capacity(8),
            duel_music: None,
            deferred_snapshots: VecDeque::with_capacity(16),
            music_value: None,
            dynamic: None,
            allow_dynamic: true,
            duel_enabled: true,
            in_duel: false,
            music_sync: true,
            music_sync_pending: false,
            music_start_time: 0,
            last_server_time: None,
            chat_sounds: crate::chat_sounds::Options::default(),
            pm_only_chat: false,
            quiet_chat: false,
            deferred_starts: 0,
            maximum_deferred_start_ms: 0.0,
            footsteps: true,
            skin_sounds_only: false,
            skin_sounds_off: false,
            voice_policy: feedback::VoicePolicy::default(),
            voice_throttle: voice_throttle::VoiceThrottle::default(),
            kill_sounds: 2,
            transitions: transitions::Transitions::default(),
            jetpack_sounds: sjk_client::LegacyJetpackSounds::default(),
            holster_sync: holstered_staff::Sync::default(),
            trace_due: std::env::var_os("SJK_TRACE_AUDIO").map(|_| Instant::now()),
            mute: crate::audio_mute::AudioMute::default(),
        };
        // `s_volume`/`s_musicvolume`/`s_volumeVoice` defaults from `S_Init` in
        // codemp/client/snd_dma.cpp:462-466. The console overwrites these on
        // the first frame, but the initial ring fill must use them too.
        audio.set_gains(0.5, 0.25, 1.0, 1.0);
        audio
    }

    /// Update listener axes (`S_Respatialize`, snd_dma.cpp:2503) once per frame.
    pub(crate) fn listener(
        &mut self,
        origin: Vec3,
        forward: Vec3,
        right: Vec3,
        up: Vec3,
        _velocity: Vec3,
    ) {
        self.output.send(AudioCommand::Listener {
            origin: origin.to_array(),
            forward: forward.to_array(),
            right: right.to_array(),
            up: up.to_array(),
            // S_Respatialize supplies no listener velocity. Feeding predicted
            // motion into the generic mixer's relative Doppler pitch shifts
            // stationary hum/ambient loops (up to the legacy 50x clamp).
            velocity: [0.0; 3],
        });
    }

    /// Play newly latched `CG_EntityEvent` sounds.
    pub(crate) fn observe_snapshot(
        &mut self,
        snapshot: &Snapshot,
        game_state: &GameState,
        vfs: &VirtualFileSystem,
    ) {
        // The listener stays where the frame put it (`finish_audio_frame`, the view's
        // origin and axes): cgame respatializes once a frame, from `refdef.vieworg`
        // (`cg_view.c:3245`). Moving it here to the snapshot's origin and angles, until
        // the frame put it back, panned the sounds near the player for the mix blocks
        // in between whenever they moved: the saber's hum crackled.
        self.transitions.race = crate::chat::combat::race(game_state, &snapshot.player);
        self.in_duel = crate::duel_blades::in_duel(Some(game_state), Some(&snapshot.player));
        self.last_server_time = Some(snapshot.server_time);
        if self.music_sync_pending {
            self.music_sync_pending = false;
            self.restore_map_music(snapshot, game_state);
        }
        self.poll_legacy_load();
        self.output
            .send(AudioCommand::ListenerSource(Some(SourceId(u32::from(
                snapshot.player.client_num(),
            )))));
        for entity in &snapshot.entities {
            // `CG_SetEntitySoundPosition`: a door's sounds follow its middle.
            let origin = self.legacy.as_ref().map_or_else(
                || entity.trajectory_base(),
                |adapter| adapter.sound_origin(entity),
            );
            self.output.send(AudioCommand::SourcePosition(
                SourceId(u32::from(entity.number())),
                origin,
            ));
        }
        if self.legacy.is_none() {
            if self.deferred_snapshots.len() == 16 {
                self.deferred_snapshots.pop_front();
            }
            let changes = self
                .sound_table_refresh
                .as_ref()
                .and_then(SoundTableRefresh::pending_snapshot);
            self.deferred_snapshots
                .push_back((Instant::now(), snapshot.clone(), changes));
            return;
        }
        let mut music_action = None;
        if let Some(adapter) = &mut self.legacy {
            let output = &mut self.output;
            let handles = &mut self.handles;
            let next_handle = &mut self.next_handle;
            adapter.refresh_clients(game_state, vfs, |path, bytes| {
                let key = path.to_ascii_lowercase();
                if let Some(handle) = handles.get(&key) {
                    return Some(*handle);
                }
                let extension = path.rsplit('.').next().unwrap_or("wav");
                let handle = SoundHandle(*next_handle);
                *next_handle = next_handle.wrapping_add(1);
                output.decode(handle, bytes, extension);
                handles.insert(key, handle);
                Some(handle)
            });
            adapter.set_local_health_pain(self.transitions.old_pain);
            adapter.set_flesh_sparks(self.flesh_sparks);
            adapter.observe_snapshot(snapshot);
            self.voice_throttle.observe_time(snapshot.server_time);
            self.voice_throttle.observe_clients(game_state);
            self.holster_sync.observe(snapshot);
            self.mute.observe(&snapshot.entities);
            let footsteps = self.footsteps;
            for decision in adapter.decisions() {
                if let Some(handle) = self.transitions.handle(decision, snapshot.server_time)
                    && let Some(handle) = controls::event_handle(
                        decision.event,
                        handle,
                        footsteps,
                        &self.chat_sounds,
                        &self.handles,
                    )
                    && self
                        .voice_policy
                        .allows(*decision, snapshot.player.client_num())
                    && self.voice_throttle.allows(
                        decision.request.source.0,
                        decision.voice_line,
                        snapshot.server_time,
                        self.output.sample_length_ms(handle),
                    )
                    && !self.mute.silences(decision.request.source, decision.cause)
                    && !self.holster_sync.intercept(
                        handle,
                        decision.request,
                        i64::from(snapshot.server_time),
                    )
                {
                    self.output
                        .send(AudioCommand::Play(handle, decision.request));
                }
            }
            self.holster_sync.pay_back(adapter);
            for action in adapter.maintained_actions() {
                match action.kind {
                    LegacyMaintainedActionKind::Start => {}
                    LegacyMaintainedActionKind::Stop => {
                        self.output.send(AudioCommand::StopLoops(action.source));
                    }
                    LegacyMaintainedActionKind::Mute => {
                        if let Some(channel) = action.channel {
                            self.output
                                .send(AudioCommand::StopChannel(action.source, channel));
                        }
                        self.output.send(AudioCommand::StopLoops(action.source));
                    }
                }
            }
            music_action = adapter.music_actions().last().copied();
        }
        self.transition_cues(snapshot, true);
        self.apply_music_action(music_action, snapshot, game_state);
    }

    fn play_deferred_snapshots(&mut self) {
        while let Some((queued_at, snapshot, changes)) = self.deferred_snapshots.pop_front() {
            if let Some((mut changes, game)) = changes {
                if let Some(vfs) = self.legacy_vfs.clone() {
                    changes.drain(|index| self.apply_sound_slot(index, &game, &vfs));
                }
            }
            let Some(adapter) = &mut self.legacy else {
                return;
            };
            adapter.set_local_health_pain(self.transitions.old_pain);
            adapter.set_flesh_sparks(self.flesh_sparks);
            adapter.observe_snapshot(&snapshot);
            self.last_server_time = Some(snapshot.server_time);
            self.voice_throttle.observe_time(snapshot.server_time);
            self.mute.observe(&snapshot.entities);
            let latency = queued_at.elapsed().as_secs_f64() * 1_000.0;
            // Advance all latches, but never compress loading history into one audio block.
            if !self.deferred_snapshots.is_empty() || latency > 300.0 {
                self.transitions.observe(&snapshot);
                continue;
            }
            for decision in adapter.decisions() {
                if let Some(handle) = self.transitions.handle(decision, snapshot.server_time)
                    && let Some(handle) = controls::event_handle(
                        decision.event,
                        handle,
                        self.footsteps,
                        &self.chat_sounds,
                        &self.handles,
                    )
                    && self
                        .voice_policy
                        .allows(*decision, snapshot.player.client_num())
                    && self.voice_throttle.allows(
                        decision.request.source.0,
                        decision.voice_line,
                        snapshot.server_time,
                        self.output.sample_length_ms(handle),
                    )
                    && !self.mute.silences(decision.request.source, decision.cause)
                {
                    self.output
                        .send(AudioCommand::Play(handle, decision.request));
                    self.deferred_starts = self.deferred_starts.saturating_add(1);
                    self.maximum_deferred_start_ms = self.maximum_deferred_start_ms.max(latency);
                }
            }
            self.transition_cues(&snapshot, true);
        }
        if self.deferred_starts != 0 {
            crate::log::progress(format_args!(
                "deferred audio starts: count={} max-latency={:.1}ms",
                self.deferred_starts, self.maximum_deferred_start_ms
            ));
        }
    }

    pub(crate) fn deferred_start_stats(&self) -> (u64, f64) {
        (self.deferred_starts, self.maximum_deferred_start_ms)
    }

    /// Play cgame's saber ignition/retraction for weapon switches, once per rendered
    /// frame as `CG_AddPacketEntities` runs: `predicted` is the live predicted state
    /// (`cg.predictedPlayerState`); without it the snapshot's playerstate is the view.
    pub(crate) fn observe_saber_switches(
        &mut self,
        snapshot: &Snapshot,
        predicted: Option<&sjk_client::pmove::MovementState>,
    ) {
        let Some(adapter) = &mut self.legacy else {
            return;
        };
        let view = predicted.map_or_else(
            || LegacySaberView::from_player_state(&snapshot.player),
            LegacySaberView::from_movement_state,
        );
        adapter.observe_saber_switches(snapshot, view);
        self.holster_sync.observe(snapshot);
        for decision in adapter.decisions() {
            if let Some(handle) = decision.handle
                && let Some(handle) = controls::event_handle(
                    decision.event,
                    handle,
                    self.footsteps,
                    &self.chat_sounds,
                    &self.handles,
                )
                && !self.holster_sync.intercept(
                    handle,
                    decision.request,
                    i64::from(snapshot.server_time),
                )
            {
                self.output
                    .send(AudioCommand::Play(handle, decision.request));
            }
        }
        self.holster_sync.pay_back(adapter);
    }

    /// The client slots muted on this PC (bits), whose sounds are no longer played;
    /// called every frame, it costs a comparison while they stay the same.
    pub(crate) fn set_muted_clients(&mut self, clients: u32) {
        self.mute.set_clients(clients);
    }

    /// Whether a sound from `source`, caused by `cause`, belongs to a muted player.
    pub(crate) fn silenced(&self, source: SourceId, cause: Option<u16>) -> bool {
        self.mute.silences(source, cause)
    }

    /// The cardboard box's music this frame, played with the frame's loops.
    pub(crate) fn set_box_music(&mut self, music: &[crate::cardboard_box::Music]) {
        self.box_music.clear();
        self.box_music.extend(
            music
                .iter()
                .map(|music| (SourceId(u32::from(music.slot)), music.origin)),
        );
    }

    /// The Hax toy's hacking sound this frame, played with the frame's loops.
    pub(crate) fn set_hax_loop(&mut self, hacking: Option<(u8, Option<[f32; 3]>)>) {
        self.hax_loop = hacking.map(|(slot, origin)| (SourceId(u32::from(slot)), origin));
    }

    /// The Prop gun's flying bolts' loops this frame, played with the frame's loops.
    pub(crate) fn set_propgun_loops(&mut self, loops: &[(SourceId, String, [f32; 3])]) {
        self.propgun_loops.clear();
        self.propgun_loops.extend_from_slice(loops);
    }

    /// Rebuild all codemp loops at rendered cadence from interpolated origins.
    ///
    /// The set is bracketed by `ClearLoops`/`CommitLoops` because the audio
    /// thread may drain the queue while it is still being filled; without the
    /// commit, loops not yet re-sent would drop out for a mix block.
    pub(crate) fn update_frame_loops(
        &mut self,
        snapshot: &Snapshot,
        presented_time: i32,
        listener_origin: [f32; 3],
        world: &World,
        bsp: &Bsp,
        predicted: Option<&sjk_client::pmove::MovementState>,
    ) {
        self.poll_legacy_load();
        self.output.send(AudioCommand::ClearLoops);
        self.send_frame_loops(snapshot, presented_time, listener_origin, world, bsp);
        self.send_box_music();
        self.send_crackle_buzzes();
        self.update_flame(snapshot, presented_time, world, predicted);
        self.output.send(AudioCommand::CommitLoops);
    }

    /// What the frame's blades of skins with a crackle were heard doing (`saber_smoke`):
    /// their snaps play now where they went, one of the skin's at random, a muted player's
    /// left out; their buzzes join the next frame's loop set. `heard` is emptied.
    pub(crate) fn hear_sabers(&mut self, heard: &mut crate::saber_smoke::Heard) {
        let skins = Arc::clone(&self.blade_skins);
        for snap in heard.snaps.drain(..).take(SNAPS_A_FRAME) {
            if self.mute.silences(SourceId(snap.entity), None) {
                continue;
            }
            let Some(crackle) = skins.crackle(snap.skin).filter(|c| !c.snaps.is_empty()) else {
                continue;
            };
            self.crackle_roll = self
                .crackle_roll
                .wrapping_mul(0x9e37_79b9)
                .wrapping_add(snap.entity + 1);
            let path = &crackle.snaps[(self.crackle_roll >> 16) as usize % crackle.snaps.len()];
            self.play_on_channel(
                path,
                crackle.snap_volume,
                snap.at,
                CRACKLE_SOURCE,
                ChannelId(0),
            );
        }
        heard.snaps.clear();
        self.crackle_buzzes.clear();
        self.crackle_buzzes.append(&mut heard.buzzes);
    }

    /// The blade skins' crackle buzzes heard last frame join the loop set, at their
    /// emitters, a muted player's left out.
    fn send_crackle_buzzes(&mut self) {
        let skins = Arc::clone(&self.blade_skins);
        for buzz in &self.crackle_buzzes {
            let source = SourceId(buzz.entity);
            if self.mute.silences(source, None) {
                continue;
            }
            let Some(crackle) = skins.crackle(buzz.skin).filter(|c| !c.buzz.is_empty()) else {
                continue;
            };
            let Some(handle) = self.find_handle(&crackle.buzz) else {
                continue;
            };
            self.output.send(AudioCommand::SetLoop(
                handle,
                PlayRequest {
                    origin: Some(buzz.at),
                    source,
                    channel: ChannelId(0),
                    volume: crackle.buzz_volume,
                    attenuation: sjk_client::legacy_sound_attenuation(0),
                },
                [0.0; 3],
            ));
        }
    }

    /// The box music joins the loop set, a loop per boxed player (none for a player
    /// muted here); it is the music of the toy, not the map's, so it ignores `s_musicVolume`.
    /// The Hax toy's hacking sound joins it the same way.
    fn send_box_music(&mut self) {
        use crate::toy_sounds::Toy;
        if let Some(handle) = self.find_handle(crate::cardboard_box::MUSIC) {
            for index in 0..self.box_music.len() {
                let (source, origin) = self.box_music[index];
                self.send_toy_loop(Toy::Box, handle, source, origin);
            }
        }
        if let (Some(handle), Some((source, origin))) =
            (self.find_handle(crate::hax::HACK_SOUND), self.hax_loop)
        {
            self.send_toy_loop(Toy::Hax, handle, source, origin);
        }
        for index in 0..self.propgun_loops.len() {
            let (source, origin) = (self.propgun_loops[index].0, self.propgun_loops[index].2);
            if let Some(handle) = self.find_handle(&self.propgun_loops[index].1) {
                self.send_toy_loop(Toy::PropGun, handle, source, Some(origin));
            }
        }
    }

    /// A toy's loop on `source` at `origin` (`None`: in both ears), unless muted here
    /// (the player, or the toy's sounds in the settings).
    fn send_toy_loop(
        &mut self,
        toy: crate::toy_sounds::Toy,
        handle: SoundHandle,
        source: SourceId,
        origin: Option<[f32; 3]>,
    ) {
        if self.mute.silences(source, None) {
            return;
        }
        let Some(volume) = self.toy_sounds.gain(toy, 1.0) else {
            return;
        };
        self.output.send(AudioCommand::SetLoop(
            handle,
            PlayRequest {
                origin,
                source,
                channel: TOY_CHANNEL,
                volume,
                attenuation: match origin {
                    Some(_) => sjk_client::legacy_sound_attenuation(0),
                    None => sjk_audio::Attenuation::None,
                },
            },
            [0.0; 3],
        ));
    }

    fn send_frame_loops(
        &mut self,
        snapshot: &Snapshot,
        presented_time: i32,
        listener_origin: [f32; 3],
        world: &World,
        bsp: &Bsp,
    ) {
        let Some(adapter) = &mut self.legacy else {
            return;
        };
        adapter.set_jetpack_sounds(self.jetpack_sounds);
        adapter.observe_loops(snapshot, presented_time, listener_origin, |state| {
            presented_entity_origin(world, bsp, state, presented_time)
        });
        // A muted player's loops (saber hum, Force, a thrown saber's hum) are left out.
        let mute = &self.mute;
        for decision in adapter.loop_decisions() {
            if let Some(handle) = decision.handle
                && !mute.silences(decision.request.source, None)
            {
                self.output.send(AudioCommand::SetLoop(
                    handle,
                    decision.request,
                    decision.velocity,
                ));
            }
        }
        for shot in adapter.ambient_shots() {
            if let Some(handle) = shot.handle
                && !mute.silences(shot.request.source, None)
            {
                self.output.send(AudioCommand::Play(handle, shot.request));
            }
        }
        adapter.observe_maintained_loops(snapshot, |state| {
            presented_entity_origin(world, bsp, state, presented_time)
        });
        for decision in adapter.maintained_loop_decisions() {
            if let Some(handle) = decision.handle
                && !mute.silences(decision.request.source, None)
            {
                self.output.send(AudioCommand::SetLoop(
                    handle,
                    decision.request,
                    decision.velocity,
                ));
            }
        }
    }
}

const TRACE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

fn presented_entity_origin(
    world: &World,
    bsp: &Bsp,
    state: &EntityState,
    presented_time: i32,
) -> [f32; 3] {
    let id = EntityId::new(u64::from(state.number()) + 1);
    let mut origin = world.entity(id).map_or_else(
        || {
            legacy_evaluate_trajectory(
                state.trajectory_base(),
                state.trajectory_delta(),
                state.trajectory_type(),
                state.trajectory_time(),
                state.trajectory_duration(),
                presented_time,
            )
        },
        |entity| entity.sample(i64::from(presented_time)).translation,
    );
    if state.solid() == SOLID_BMODEL
        && let Ok(index) = usize::try_from(state.model_index())
        && let Some(model) = bsp.render().inline_model(index)
    {
        for axis in 0..3 {
            origin[axis] += (model.minimums[axis] + model.maximums[axis]) * 0.5;
        }
    }
    origin
}

use glam::Vec3;
