//! Local-player movement prediction state of a live session.
//!
//! Owns the `Predictor`, the window of not-yet-acknowledged user commands
//! that is replayed on every snapshot (`CG_PredictPlayerState`,
//! `cg_predict.c:1020-1114`), the predicted view height and the prediction
//! error smoother (`cg_predict.c:1188-1237`, `cg_view.c:1597-1608`). The
//! renderer reads the predicted eye position from here; the model root uses
//! the un-offset origin exactly like `cent->lerpOrigin`.

use std::collections::VecDeque;
use std::sync::Arc;

use glam::Vec3;
use sjk_bsp::{Bsp, TraceScratch};
use sjk_client::pmove::{MovementConfig, MovementState, Predictor, predict_legacy};
use sjk_client::{
    AnimationLengthTable, AnimationLengths, EF_TELEPORT_BIT, PredictionErrorDecay,
    legacy_saber_combat, legacy_saber_hands, legacy_saber_movement,
};
use sjk_model::AnimationConfig;
use sjk_protocol::{GameState, Snapshot, UserCommand};
use sjk_vfs::VirtualFileSystem;

use crate::movement_collision::BspMovementCollision;

#[path = "prediction_entities.rs"]
mod entity_context;
#[path = "prediction_history_gap.rs"]
mod history_gap;
#[path = "prediction_movers.rs"]
pub(crate) mod movers;
#[path = "local_prediction_reconcile.rs"]
mod reconcile;
#[path = "saber_lock_view.rs"]
mod saber_lock_view;
#[path = "prediction_miss.rs"]
pub(crate) mod telemetry;
#[path = "wall_grab_view.rs"]
mod wall_grab_view;

/// Commands kept for re-prediction: about 1.5 s at a gameplay FPS of 333, 4 s at
/// 125 (`command_rate`). Stock's `CMD_BACKUP` 64 holds half a second at 125 FPS;
/// JoF EJK raises it with `cl_commandsize`, up to this 512.
const COMMAND_BACKUP: usize = 512;

/// How the server's `ps.commandTime` relates to the commands this client sent
/// (`ClientThink_real` clamps a stamp to `[level.time - 1000, level.time + 200]`
/// and drops one that does not advance `ps.commandTime`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CommandStatus {
    /// `ps.commandTime` is a stamp still in the pending window.
    #[default]
    Acknowledged,
    /// `ps.commandTime` advanced to a stamp this client never sent.
    Rewritten,
    /// `ps.commandTime` did not advance: no command ran for this snapshot.
    Stalled,
}

impl CommandStatus {
    /// Classify a snapshot's `ps.commandTime` against the previous one and
    /// the stamps of the commands still pending.
    pub(crate) fn classify(
        acknowledged: i32,
        previous: i32,
        mut pending_stamps: impl Iterator<Item = i32>,
    ) -> Self {
        if acknowledged == previous {
            Self::Stalled
        } else if pending_stamps.any(|stamp| stamp == acknowledged) {
            Self::Acknowledged
        } else {
            Self::Rewritten
        }
    }
}

/// What one snapshot told us about the local prediction, for the
/// `cl_showtimedelta` window (`net_timing`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PredictionSample {
    pub(crate) status: CommandStatus,
    /// Commands still unacknowledged after this snapshot.
    pub(crate) pending: usize,
    /// Distance between the previous prediction and the re-prediction at the
    /// same command time (`cg_predict.c:1188-1237`), in units.
    pub(crate) miss_units: f32,
    /// Replay work and endpoint-matched large-miss correlations for this snapshot.
    pub(crate) counts: telemetry::Counts,
}

pub(crate) struct LocalPrediction {
    predict_items: bool,
    pending_events: sjk_client::predicted_events::PredictedEvents,
    predictor: Option<Predictor>,
    /// The committed predictor advanced through this frame's not-yet-sent
    /// input; what the frame presents (see [`Self::preview_command`]).
    preview: Option<Predictor>,
    presented: Option<MovementState>,
    physics_movers: movers::Movers,
    entities: entity_context::Entities,
    saber_lock_view: saber_lock_view::LockView,
    render_movers: movers::Movers,
    presentation_time: i32,
    pending: VecDeque<UserCommand>,
    history_gap: history_gap::HistoryGap,
    latest_input: Option<UserCommand>,
    view_height: f32,
    error: PredictionErrorDecay,
    /// `(eFlags, clientNum, vehicleNum)` for teleport and mount-boundary resets.
    previous_player: Option<(u32, u16, u16)>,
    frame_millis: i32,
    previous_frame_millis: i32,
    animation_lengths: Option<Arc<dyn AnimationLengths>>,
    movement_config: MovementConfig,
    /// Force a local wall-grab lock even when the profile permits free look.
    wall_grab_lock: bool,
    last_acknowledged: i32,
    /// Presentation-only support for the unadvanced snapshot seed. Commands
    /// still start with the untouched server ground flag and run normal Pmove.
    seed_ground: Option<u16>,
    last_sample: PredictionSample,
    miss_log: telemetry::MissLog,
    /// The vehicle the local player pilots, predicted with it.
    rides: ride::Rides,
    /// `cg_fakeNoclip`: the predictor flies on its own while the server is sent a still player.
    fake_noclip: bool,
    /// Reset without smoothing on the next usable snapshot (returning from fake noclip,
    /// or a discontinuity received while incomplete history prevents reseeding).
    snap_back: bool,
    /// The view angles the server is sent while flying: the ones held when it began.
    frozen_angles: [i32; 3],
    free_camera: bool,
    frozen_weapon: u8,
}

impl LocalPrediction {
    /// Take `previous`'s animation length table, when it has one: reusing the loaded map
    /// keeps the model the client started with instead of falling back to the humanoid set.
    pub(crate) fn keep_animation_lengths(&mut self, previous: &Self) {
        let Some(lengths) = &previous.animation_lengths else {
            return;
        };
        if let Some(predictor) = &mut self.predictor {
            predictor.set_animation_lengths(Arc::clone(lengths));
        }
        self.animation_lengths = Some(Arc::clone(lengths));
    }

    /// Prediction state primed from an optional first snapshot.
    pub(crate) fn new(
        snapshot: Option<&Snapshot>,
        animation_config: Option<&AnimationConfig>,
        game_state: Option<&GameState>,
        vfs: &VirtualFileSystem,
    ) -> Self {
        // EternalJK `cg_predict.c:1311` (a40e793) hands Pmove the local player's
        // animation set every frame. Without one the kicks and saber attacks wait a
        // round trip for the server; a joined game has no preview model, so it gets
        // the humanoid set, the skeleton player models use unless they bring their own.
        let humanoid = animation_config
            .is_none()
            .then(|| game_state.and_then(|_| humanoid_animation_config(vfs)))
            .flatten();
        let animation_lengths = animation_config.or(humanoid.as_ref()).map(|config| {
            Arc::new(AnimationLengthTable::from_animation_config(config))
                as Arc<dyn AnimationLengths>
        });
        let view_height = snapshot.map_or(36.0, |snapshot| snapshot.player.view_height() as f32);
        let mut movement_config =
            game_state.map_or_else(MovementConfig::default, MovementConfig::from_game_state);
        if let Some(game_state) = game_state {
            let client = snapshot.map_or(game_state.client_num as u16, |s| s.player.client_num());
            let (no_rolls, scales, anim_scales) = legacy_saber_movement(vfs, game_state, client);
            movement_config.roll_rules.saber_forbids_rolls = no_rolls;
            movement_config.saber_speed_scales = scales;
            movement_config.saber_anim_speed_scales = anim_scales;
            movement_config.saber_hands = Some(legacy_saber_hands(vfs, game_state, client));
            movement_config.saber_definitions = Some(legacy_saber_combat(vfs, game_state, client));
        }
        let predictor = snapshot.and_then(|snapshot| {
            (predicts_local_view(snapshot.player.movement_flags())
                && !game_state
                    .is_some_and(crate::prediction_preview::interpolated::server_synchronous))
            .then(|| {
                let mut predictor = Predictor::from_player_state(&snapshot.player, movement_config);
                if let Some(lengths) = &animation_lengths {
                    predictor.set_animation_lengths(Arc::clone(lengths));
                }
                predictor
            })
        });
        let mut physics_movers = movers::Movers::new();
        let mut render_movers = movers::Movers::new();
        let mut rides = ride::Rides::new(vfs);
        if let (Some(snapshot), Some(game)) = (snapshot, game_state) {
            rides.resolve(snapshot, game);
        }
        let (revision, bounds) = rides.collision_bounds();
        physics_movers.set_vehicle_bounds(revision, bounds);
        render_movers.set_vehicle_bounds(revision, bounds);
        physics_movers.set_permanents(game_state);
        render_movers.set_permanents(game_state);
        if let Some(snapshot) = snapshot {
            physics_movers.update(snapshot, snapshot.server_time);
            render_movers.update(snapshot, snapshot.server_time);
        }
        Self {
            predict_items: true,
            pending_events: Default::default(),
            predictor,
            preview: None,
            presented: None,
            physics_movers,
            entities: entity_context::Entities::new(snapshot),
            saber_lock_view: saber_lock_view::LockView::new(snapshot),
            render_movers,
            presentation_time: snapshot.map_or(0, |s| s.server_time),
            pending: VecDeque::with_capacity(COMMAND_BACKUP),
            history_gap: Default::default(),
            latest_input: None,
            view_height,
            error: PredictionErrorDecay::default(),
            previous_player: None,
            frame_millis: 0,
            previous_frame_millis: 0,
            animation_lengths,
            movement_config,
            wall_grab_lock: false,
            last_acknowledged: snapshot.map_or(0, |snapshot| snapshot.player.command_time()),
            seed_ground: None,
            last_sample: PredictionSample::default(),
            miss_log: telemetry::MissLog::default(),
            rides,
            fake_noclip: false,
            snap_back: false,
            frozen_angles: [0; 3],
            free_camera: false,
            frozen_weapon: 0,
        }
    }

    /// Set the player's optional wall-grab camera lock; movement rules stay authoritative.
    pub(crate) fn set_wall_grab_lock(&mut self, on: bool) {
        self.wall_grab_lock = on;
    }

    /// Start or end `cg_fakeNoclip`. Ending it returns to the server's position on the next
    /// snapshot, with no error smoothing across the jump.
    pub(crate) fn set_fake_noclip(&mut self, on: bool) {
        if self.fake_noclip == on {
            return;
        }
        if on && let Some(latest) = self.latest_input {
            self.frozen_angles = latest.angles;
        }
        self.fake_noclip = on;
        self.snap_back = !on;
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_fake_noclip(on);
        }
    }

    /// The map editor's own flight speed while a fake noclip flies (`None`: stock), handed
    /// to every predictor each frame (`Predictor::set_fake_noclip_boost`).
    pub(crate) fn set_fake_noclip_boost(&mut self, boost: Option<f32>) {
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_fake_noclip_boost(boost);
        }
    }

    /// Put a fake noclip's flight with its eye at `eye`, still: the map editor's fly-to
    /// glides the free camera there (`map_edits::fly`). Only the client's own predicted
    /// flight moves; the server is still sent the still body ([`Self::command_for_server`]).
    /// Whether there was a flight to move.
    pub(crate) fn place_flight(&mut self, eye: Vec3) -> bool {
        if !self.fake_noclip {
            return false;
        }
        let Some(predictor) = self.predictor.as_mut() else {
            return false;
        };
        let state = predictor.state_mut();
        state.origin = (eye - Vec3::Z * self.view_height).to_array();
        state.velocity = [0.0; 3];
        self.preview = None;
        self.presented = None;
        true
    }

    /// Free camera additionally suppresses gameplay actions and weapon changes.
    pub(crate) fn set_free_camera(&mut self, on: bool, weapon: u8) {
        if on && !self.free_camera {
            self.frozen_weapon = weapon;
        }
        self.free_camera = on;
    }

    pub(crate) fn free_camera(&self) -> bool {
        self.free_camera
    }

    pub(crate) fn fake_noclip(&self) -> bool {
        self.fake_noclip
    }

    /// What the server is sent for `command`: while flying, a player with the talk balloon
    /// up, standing still with a frozen view (`CL_WritePacket`, `cl_input.cpp:2175-2196`).
    /// The time stays, so command timing keeps in step.
    pub(crate) fn command_for_server(&self, command: UserCommand) -> UserCommand {
        if !self.fake_noclip {
            return command;
        }
        UserCommand {
            angles: self.frozen_angles,
            generic_command: if self.free_camera {
                0
            } else {
                command.generic_command
            },
            weapon: if self.free_camera {
                self.frozen_weapon
            } else {
                command.weapon
            },
            buttons: sjk_game_jka::pmove_talk::BUTTON_TALK,
            forward_move: 0,
            right_move: 0,
            up_move: 0,
            ..command
        }
    }

    /// The bookkeeping of the most recent [`Self::apply_snapshot`].
    pub(crate) fn last_sample(&self) -> PredictionSample {
        self.last_sample
    }

    /// Refresh server policy while preserving saber restrictions and prediction history.
    pub(crate) fn refresh_roll_rules(&mut self, game_state: &GameState) {
        let free_knockdown_view = self.movement_config.ja_plus.free_knockdown_view;
        self.movement_config.refresh_game_state(game_state);
        self.movement_config.ja_plus.free_knockdown_view = free_knockdown_view;
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_config(self.movement_config);
        }
        self.physics_movers.set_duel_isolation(Some(game_state));
        self.render_movers.set_duel_isolation(Some(game_state));
    }

    /// Reload local equipment policy when CS_PLAYERS changes, never per frame.
    pub(crate) fn refresh_sabers(
        &mut self,
        game: &GameState,
        vfs: &VirtualFileSystem,
        client: u16,
    ) {
        let local = self
            .predictor
            .as_ref()
            .map(|predictor| predictor.state().client_num)
            .unwrap_or(game.client_num as u16);
        if client != local {
            return;
        }
        let (no_rolls, scales, anim_scales) = legacy_saber_movement(vfs, game, client);
        self.movement_config.roll_rules.saber_forbids_rolls = no_rolls;
        self.movement_config.saber_speed_scales = scales;
        self.movement_config.saber_anim_speed_scales = anim_scales;
        self.movement_config.saber_hands = Some(legacy_saber_hands(vfs, game, client));
        self.movement_config.saber_definitions = Some(legacy_saber_combat(vfs, game, client));
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_config(self.movement_config);
        }
    }

    /// The solids where the frame presents them (doors, lifts, other players' boxes),
    /// for traces that are not movement (the room acoustics' occlusion, `s_spatial`).
    pub(crate) fn presented_solids(&self) -> &[movers::Collider] {
        &self.render_movers.colliders
    }

    /// Predicted movement state, when prediction is active
    /// (`cg.predictedPlayerState`).
    pub(crate) fn predicted_state(&self) -> Option<&MovementState> {
        self.presented.as_ref().or_else(|| {
            self.preview
                .as_ref()
                .or(self.predictor.as_ref())
                .map(Predictor::state)
        })
    }

    /// The state after the last command sent (not the frame's preview), when
    /// prediction is active: what the next command starts from.
    pub(crate) fn committed_state(&self) -> Option<&MovementState> {
        self.predictor.as_ref().map(Predictor::state)
    }

    /// Vehicle strafing already computed by prediction; read-only camera input.
    pub(crate) fn vehicle_camera_strafe(&self) -> Option<i32> {
        self.rides
            .preview
            .as_ref()
            .or(self.rides.committed.as_ref())
            .map(|ride| ride.state().hacking_time)
    }

    /// The predicted vehicle's speed and turbo time for the vehicle HUD
    /// (`cg.predictedVehicleState.speed` and `m_pVehicle->m_iTurboTime`), when the ride
    /// predicted is vehicle `number`.
    pub(crate) fn vehicle_readout(&self, number: u16) -> Option<(f32, Option<i32>)> {
        let ride = self
            .rides
            .preview
            .as_ref()
            .or(self.rides.committed.as_ref())?;
        (ride.number() == number).then(|| (ride.state().speed, ride.turbo_time()))
    }

    /// Render the piloted vehicle from the same command as the predicted camera,
    /// as CG_AddPacketEntities does with cg.predictedVehicleState.
    pub(crate) fn vehicle_pose(&self) -> Option<crate::vehicle_pose::Predicted> {
        let pilot = self.predicted_state()?;
        let ride = self
            .rides
            .preview
            .as_ref()
            .or(self.rides.committed.as_ref())?;
        if pilot.vehicle_entity_num != ride.number() || pilot.movement_type == 7 {
            return None;
        }
        Some(crate::vehicle_pose::Predicted {
            number: ride.number(),
            origin: ride.state().origin,
            angles: ride.state().view_angles,
        })
    }

    /// Predicted view height above the origin.
    pub(crate) fn view_height(&self) -> f32 {
        self.view_height
    }

    /// Override the view height (demo playback follows the snapshot).
    pub(crate) fn set_view_height(&mut self, view_height: f32) {
        self.view_height = view_height;
    }

    /// Refresh the existing player's jaPRO only-bhop preference without enabling
    /// jaPRO on a foreign server. Serverinfo refresh preserves this preference.
    pub(crate) fn set_ja_pro_plugin_disable(&mut self, bits: u32) {
        let player_bhop = bits & sjk_game_jka::pmove_japro::PLUGIN_BHOP != 0;
        if self.movement_config.ja_pro.player_bhop == player_bhop {
            return;
        }
        self.movement_config.ja_pro.player_bhop = player_bhop;
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_config(self.movement_config);
        }
    }

    /// Update `cg_predictLegacy`: the prediction changes since 2026.1028.1 this player
    /// turned back off ([`predict_legacy`]).
    pub(crate) fn set_predict_legacy(&mut self, bits: u32) {
        let bits = bits & predict_legacy::ALL;
        if self.movement_config.predict_legacy == bits {
            return;
        }
        self.movement_config.predict_legacy = bits;
        self.saber_lock_view.off = bits & predict_legacy::SABER_LOCK_FACING != 0;
        self.entities
            .set_off(bits & predict_legacy::ACTOR_CONTEXT != 0);
        if bits & predict_legacy::HISTORY_GAP != 0 {
            self.history_gap = Default::default();
        }
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_config(self.movement_config);
        }
    }

    /// Update whether the JA+ server leaves the view free through knockdowns and get-ups
    /// (`held_view::free_knockdown_view`), which no serverinfo says.
    pub(crate) fn set_free_knockdown_view(&mut self, free: bool) {
        if self.movement_config.ja_plus.free_knockdown_view == free {
            return;
        }
        self.movement_config.ja_plus.free_knockdown_view = free;
        for predictor in self.predictor.iter_mut().chain(self.preview.iter_mut()) {
            predictor.set_config(self.movement_config);
        }
    }

    /// Update `cg_showMiss`: whether large misses are written to the log.
    pub(crate) fn set_show_miss(&mut self, show: bool) {
        self.miss_log.show = show;
    }

    /// Update `cg_errorDecay`.
    pub(crate) fn set_error_decay_millis(&mut self, decay_millis: f32) {
        self.error.set_decay_millis(decay_millis);
    }

    /// Advance the frame clock (`cg.time` / `cg.oldTime`).
    pub(crate) fn begin_frame(&mut self, frame_millis: i32) {
        self.previous_frame_millis = self.frame_millis;
        self.frame_millis = frame_millis;
    }

    /// View-origin offset for the current frame (`cg_view.c:1597-1608`).
    pub(crate) fn view_offset(&mut self) -> Vec3 {
        Vec3::from_array(self.error.view_offset(self.frame_millis))
    }

    /// Forget every pending command (map change, intermission, spectating).
    /// Commands not yet acknowledged by a snapshot, for diagnostics.
    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn clear_pending(&mut self) {
        self.latest_input = None;
        self.pending_events.clear();
        self.pending.clear();
        self.history_gap = Default::default();
        self.presented = None;
    }

    /// Drop prediction entirely; the view follows the snapshot.
    pub(crate) fn stop(&mut self, view_height: f32) {
        self.error = PredictionErrorDecay::new(self.error.decay_millis());
        self.latest_input = None;
        self.pending_events.clear();
        self.pending.clear();
        self.history_gap = Default::default();
        self.predictor = None;
        self.preview = None;
        self.presented = None;
        self.rides.committed = None;
        self.rides.preview = None;
        self.view_height = view_height;
    }

    /// Which vehicle the local player pilots in `snapshot`, for the next
    /// [`Self::apply_snapshot`] to predict with it.
    pub(crate) fn resolve_ride(&mut self, snapshot: &Snapshot, game_state: &GameState) {
        self.rides.resolve(snapshot, game_state);
        let (revision, bounds) = self.rides.collision_bounds();
        self.physics_movers.set_vehicle_bounds(revision, bounds);
        self.render_movers.set_vehicle_bounds(revision, bounds);
    }

    /// Queue the command `sent` to the server and predict `command`, the same command unless
    /// a fake noclip sends the server a still player; returns the eye position when
    /// prediction is active.
    pub(crate) fn apply_command(
        &mut self,
        command: UserCommand,
        sent: UserCommand,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
    ) -> Option<Vec3> {
        self.latest_input = Some(command);
        self.remember_command(sent);
        if !self.fake_noclip && self.history_gap.incomplete() {
            return self.frozen_eye();
        }
        let predictor = self.predictor.as_mut()?;
        if !self.fake_noclip {
            self.saber_lock_view.face(
                predictor.state_mut(),
                self.presentation_time,
                self.physics_movers
                    .entity_origin(self.saber_lock_view.enemy()),
            );
        }
        let collision = BspMovementCollision::for_actor(
            bsp,
            scratch,
            &self.physics_movers.colliders,
            self.physics_movers.local_actor,
        );
        self.entities.predict_with_collisions(
            predictor,
            self.rides.committed.as_mut(),
            command,
            &collision,
            &collision.chassis(self.physics_movers.vehicle_actor),
        );
        self.rides.preview = None;
        touch_triggers(
            predictor,
            &self.physics_movers.triggers,
            bsp,
            self.presentation_time,
            self.movement_config.roll_rules.gametype,
            self.predict_items,
        );
        predictor.emit_events(|event| self.pending_events.push(event));
        self.preview = None;
        self.presented = None;
        self.view_height = predictor.state().view_height as f32;
        Some(Vec3::from_array(predictor.state().origin) + Vec3::Z * self.view_height)
    }

    /// Advance the committed prediction through the input of a frame that
    /// sends no packet, without queueing it. The stock client builds a
    /// usercmd every frame and predicts through all of them
    /// (`cl_input.cpp` `CL_CreateNewCommands`, `cg_predict.c:1124-1237`)
    /// while packets leave at `cl_maxpackets`; SJK makes a command every
    /// gameplay FPS step (`command_rate`), so at higher frame rates the presented state
    /// would otherwise stand still between commands and then step. Returns
    /// the eye position.
    pub(crate) fn preview_command(
        &mut self,
        command: UserCommand,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
    ) -> Option<Vec3> {
        self.latest_input = Some(command);
        if !self.fake_noclip && self.history_gap.incomplete() {
            return self.frozen_eye();
        }
        let committed = self.predictor.as_ref()?;
        if command.server_time <= committed.state().command_time {
            return None;
        }
        // `clone_from` reuses the preview's storage: no per-frame allocation.
        let preview = match &mut self.preview {
            Some(preview) => {
                preview.clone_from(committed);
                preview
            }
            None => self.preview.insert(committed.clone()),
        };
        if !self.fake_noclip {
            self.saber_lock_view.face(
                preview.state_mut(),
                self.presentation_time,
                self.physics_movers
                    .entity_origin(self.saber_lock_view.enemy()),
            );
        }
        let collision = BspMovementCollision::for_actor(
            bsp,
            scratch,
            &self.physics_movers.colliders,
            self.physics_movers.local_actor,
        );
        self.entities.predict_with_collisions(
            preview,
            self.rides.begin_preview(),
            command,
            &collision,
            &collision.chassis(self.physics_movers.vehicle_actor),
        );
        touch_triggers(
            preview,
            &self.physics_movers.triggers,
            bsp,
            self.presentation_time,
            self.movement_config.roll_rules.gametype,
            self.predict_items,
        );
        preview.emit_events(|event| self.pending_events.push(event));
        self.view_height = preview.state().view_height as f32;
        Some(Vec3::from_array(preview.state().origin) + Vec3::Z * self.view_height)
    }

    /// Retain the exact wire command, tracking any missing simulation interval.
    fn remember_command(&mut self, sent: UserCommand) {
        if self.pending.len() == COMMAND_BACKUP
            && let Some(dropped) = self.pending.pop_front()
            && self.movement_config.predict_legacy & predict_legacy::HISTORY_GAP == 0
        {
            self.history_gap.discarded(
                self.movement_config.command_time(dropped.server_time),
                self.last_acknowledged,
            );
        }
        // What the server was sent is what a re-prediction replays; a fake noclip's own
        // flight is only the predictor's (`cg_predict.c:1355`).
        self.pending.push_back(sent);
    }

    /// Hold the last display copy, including its mover offset, while history is missing.
    fn frozen_eye(&self) -> Option<Vec3> {
        let state = self.predicted_state()?;
        Some(Vec3::from_array(state.origin) + Vec3::Z * state.view_height as f32)
    }

    /// Latest fully constructed input, including mouse movement and strafe modifiers.
    pub(crate) fn guide_input(&self) -> Option<[i8; 2]> {
        self.latest_input.map(|c| [c.forward_move, c.right_move])
    }

    /// The movement of the command made last (`forwardmove`, `rightmove`, `upmove`),
    /// for the movement keys (`hud::movement_keys`).
    pub(crate) fn movement_keys_input(&self) -> Option<[i8; 3]> {
        self.latest_input
            .map(|c| [c.forward_move, c.right_move, c.up_move])
    }

    /// Deliver this frame's committed, replayed and preview events without allocating.
    pub(crate) fn drain_events(
        &mut self,
        mut sink: impl FnMut(sjk_client::predicted_events::PredictedEvent),
    ) {
        for event in self.pending_events.iter() {
            sink(event);
        }
        self.pending_events.clear();
    }
}

#[path = "prediction_ride.rs"]
mod ride;

#[path = "prediction_triggers.rs"]
mod triggers;
#[cfg(test)]
#[path = "prediction_vehicle_bounds_tests.rs"]
mod vehicle_bounds_tests;

use triggers::touch_triggers;

/// `models/players/_humanoid/animation.cfg`, the skeleton player models use unless they
/// bring their own.
fn humanoid_animation_config(vfs: &VirtualFileSystem) -> Option<AnimationConfig> {
    let asset = vfs
        .read("models/players/_humanoid/animation.cfg")
        .ok()
        .flatten()?;
    AnimationConfig::parse(&asset.bytes).ok()
}

/// cg_predict.c:952: only following another player bypasses live prediction.
pub(crate) fn predicts_local_view(movement_flags: u16) -> bool {
    movement_flags & 4096 == 0 // PMF_FOLLOW, not the spectator team/type.
}

#[cfg(test)]
#[path = "prediction_japro_preferences_tests.rs"]
mod ja_pro_preferences_tests;

#[cfg(test)]
#[path = "prediction_legacy_tests.rs"]
mod predict_legacy_tests;

#[cfg(test)]
mod fake_noclip_tests {
    use super::*;

    fn prediction() -> LocalPrediction {
        LocalPrediction::new(None, None, None, &VirtualFileSystem::new())
    }

    fn command() -> UserCommand {
        UserCommand {
            server_time: 100,
            angles: [10, 20, 30],
            buttons: 1,
            forward_move: 127,
            right_move: -127,
            up_move: 127,
            ..UserCommand::default()
        }
    }

    #[test]
    fn free_camera_suppresses_actions_at_each_command_step_and_keeps_time() {
        for step in [8, 7, 4, 3] {
            let mut prediction = prediction();
            prediction.latest_input = Some(command());
            prediction.set_fake_noclip(true);
            prediction.set_free_camera(true, 3);
            prediction.set_free_camera(true, 7); // Later snapshots cannot change the held weapon.
            let input = UserCommand {
                server_time: 100 + step,
                weapon: 4,
                generic_command: 10,
                ..command()
            };
            let sent = prediction.command_for_server(input);
            assert_eq!(sent.server_time, input.server_time);
            assert_eq!(sent.weapon, 3);
            assert_eq!(sent.generic_command, 0);
            assert_eq!(sent.angles, command().angles);
            assert_eq!(
                (sent.forward_move, sent.right_move, sent.up_move),
                (0, 0, 0)
            );
            assert_eq!(sent.buttons, sjk_game_jka::pmove_talk::BUTTON_TALK);
            prediction.set_free_camera(false, 0);
            assert_eq!(prediction.command_for_server(input).weapon, 4);
            assert_eq!(prediction.command_for_server(input).generic_command, 10);
            prediction.set_fake_noclip(false);
            assert_eq!(prediction.command_for_server(input), input);
        }
    }

    #[test]
    fn the_server_is_sent_the_command_unchanged_when_not_flying() {
        assert_eq!(prediction().command_for_server(command()), command());
    }

    #[test]
    fn a_flying_player_is_sent_still_with_the_talk_balloon_and_the_starting_view() {
        let mut prediction = prediction();
        prediction.latest_input = Some(command());
        prediction.set_fake_noclip(true);
        // The view moves on; the server keeps seeing the one held when the flight began.
        let moved = UserCommand {
            angles: [90, 91, 92],
            server_time: 150,
            ..command()
        };
        let sent = prediction.command_for_server(moved);
        assert_eq!(sent.angles, [10, 20, 30]);
        assert_eq!(sent.buttons, sjk_game_jka::pmove_talk::BUTTON_TALK);
        assert_eq!(
            (sent.forward_move, sent.right_move, sent.up_move),
            (0, 0, 0)
        );
        assert_eq!(sent.server_time, 150, "command timing stays in step");
    }

    #[test]
    fn ending_the_flight_snaps_back_without_smoothing() {
        let mut prediction = prediction();
        prediction.set_fake_noclip(true);
        assert!(!prediction.snap_back);
        prediction.set_fake_noclip(false);
        assert!(prediction.snap_back);
        assert_eq!(prediction.command_for_server(command()), command());
    }

    /// The map editor's fly-to moves the client's own flight only, never a walking body,
    /// and leaves it still with its eye where it was put.
    #[test]
    fn a_flight_is_put_where_the_editor_says_and_a_walk_is_not() {
        let mut prediction = prediction();
        let state = MovementState {
            velocity: [300.0, 0.0, 0.0],
            ..MovementState::default()
        };
        prediction.predictor = Some(Predictor::from_state(state, MovementConfig::default()));
        prediction.view_height = 36.0;
        let eye = Vec3::new(100.0, -50.0, 200.0);
        assert!(!prediction.place_flight(eye), "walking: not moved");
        prediction.set_fake_noclip(true);
        assert!(prediction.place_flight(eye));
        let state = prediction.committed_state().unwrap();
        assert_eq!(state.origin, [100.0, -50.0, 164.0]);
        assert_eq!(state.velocity, [0.0; 3]);
        assert!(prediction.preview.is_none());
        // The server still sees the still body.
        assert_eq!(
            prediction.command_for_server(command()).forward_move,
            0,
            "the server is sent no move"
        );
    }
}

#[cfg(test)]
mod animation_length_tests {
    use super::*;

    fn vfs_with_humanoid() -> VirtualFileSystem {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "humanoid",
            [(
                "models/players/_humanoid/animation.cfg",
                b"BOTH_A7_KICK_L\t100\t30\t-1\t20\n".to_vec(),
            )],
        )
        .expect("mount");
        vfs
    }

    #[test]
    fn a_joined_game_without_a_preview_model_predicts_with_the_humanoid_set() {
        let game = GameState::empty_local(0);
        let prediction = LocalPrediction::new(None, None, Some(&game), &vfs_with_humanoid());
        assert!(
            prediction.animation_lengths.is_some(),
            "kicks and saber attacks need the lengths to be predicted"
        );
    }

    #[test]
    fn reusing_the_map_keeps_the_previous_table() {
        let game = GameState::empty_local(0);
        let previous = LocalPrediction::new(None, None, Some(&game), &vfs_with_humanoid());
        let mut restarted = LocalPrediction::new(None, None, None, &VirtualFileSystem::new());
        assert!(restarted.animation_lengths.is_none());
        restarted.keep_animation_lengths(&previous);
        assert!(restarted.animation_lengths.is_some());
    }

    #[test]
    fn no_game_or_no_humanoid_file_leaves_the_lengths_out() {
        let game = GameState::empty_local(0);
        let without_game = LocalPrediction::new(None, None, None, &vfs_with_humanoid());
        assert!(without_game.animation_lengths.is_none());
        let without_file = LocalPrediction::new(None, None, Some(&game), &VirtualFileSystem::new());
        assert!(without_file.animation_lengths.is_none());
    }
}

#[cfg(test)]
mod history_overflow_tests {
    use super::*;

    #[test]
    fn overflowing_the_real_queue_marks_a_gap_and_reset_clears_it() {
        for step in [8, 7, 4, 3] {
            let mut prediction = LocalPrediction::new(None, None, None, &VirtualFileSystem::new());
            for n in 1..=COMMAND_BACKUP {
                prediction.remember_command(UserCommand {
                    server_time: n as i32 * step,
                    ..Default::default()
                });
            }
            assert!(!prediction.history_gap.incomplete());
            prediction.remember_command(UserCommand {
                server_time: (COMMAND_BACKUP as i32 + 1) * step,
                ..Default::default()
            });
            assert_eq!(prediction.pending.len(), COMMAND_BACKUP);
            assert!(prediction.history_gap.incomplete());
            prediction.history_gap.acknowledge(step - 1);
            assert!(prediction.history_gap.incomplete());
            prediction.history_gap.acknowledge(step);
            assert!(!prediction.history_gap.incomplete());
            prediction.remember_command(UserCommand {
                server_time: (COMMAND_BACKUP as i32 + 2) * step,
                ..Default::default()
            });
            assert!(prediction.history_gap.incomplete());
            prediction.clear_pending();
            assert!(!prediction.history_gap.incomplete());
            assert!(prediction.pending.is_empty());
        }
    }

    #[test]
    fn a_history_gap_holds_the_presented_mover_offset_and_view_height() {
        let mut prediction = LocalPrediction::new(None, None, None, &VirtualFileSystem::new());
        prediction.predictor = Some(Predictor::from_state(
            MovementState {
                origin: [1.0, 2.0, 3.0],
                view_height: 36,
                ..Default::default()
            },
            Default::default(),
        ));
        prediction.presented = Some(MovementState {
            origin: [4.0, 5.0, 6.0],
            view_height: 12,
            ..Default::default()
        });
        prediction.history_gap.discarded(8, 0);
        assert_eq!(prediction.frozen_eye(), Some(Vec3::new(4.0, 5.0, 18.0)));
        assert_eq!(
            prediction.predicted_state().unwrap().origin,
            [4.0, 5.0, 6.0]
        );
        prediction.presented = None;
        assert_eq!(prediction.frozen_eye(), Some(Vec3::new(1.0, 2.0, 39.0)));
    }

    #[test]
    fn a_teleport_during_a_history_gap_stays_pending_until_recovery() {
        for step in [8, 7, 4, 3] {
            let mut prediction = LocalPrediction::new(None, None, None, &VirtualFileSystem::new());
            prediction.predictor = Some(Predictor::from_state(
                MovementState {
                    command_time: 2 * step,
                    health: 100,
                    gravity: 800.0,
                    origin: [10.0, 0.0, 0.0],
                    view_height: 36,
                    ground_entity_number: sjk_protocol::ENTITY_NUMBER_NONE,
                    ..Default::default()
                },
                Default::default(),
            ));
            prediction.presented = Some(MovementState {
                origin: [11.0, 0.0, 0.0],
                view_height: 36,
                ..Default::default()
            });
            prediction.previous_player = Some((0, 0, 0));
            prediction.history_gap.discarded(step, 0);
            prediction.pending.push_back(UserCommand {
                server_time: 2 * step,
                ..Default::default()
            });
            let mut player = sjk_protocol::PlayerState::default();
            player.stats[0] = 100;
            player.set_gravity(800);
            player.set_ground_entity_num(sjk_protocol::ENTITY_NUMBER_NONE);
            player.set_origin([1000.0, 0.0, 0.0]);
            player.set_raw_field(17, EF_TELEPORT_BIT);
            let mut snapshot = Snapshot {
                message_sequence: 1,
                reliable_acknowledge: 0,
                server_commands: Vec::new(),
                server_time: 100,
                delta_from: None,
                flags: 0,
                area_mask: Vec::new(),
                player,
                vehicle_player: None,
                entities: Vec::new(),
                consumed_bits: 0,
            };
            let bsp = Bsp::empty([-2000.0; 3], [2000.0; 3]);
            let mut scratch = TraceScratch::new();
            assert_eq!(
                prediction.apply_snapshot(&snapshot, &bsp, &mut scratch),
                Vec3::new(11.0, 0.0, 36.0)
            );
            assert!(prediction.snap_back);
            assert!(prediction.history_gap.incomplete());
            snapshot.message_sequence += 1;
            prediction.apply_snapshot(&snapshot, &bsp, &mut scratch);
            assert!(
                prediction.snap_back,
                "a stalled ACK must retain the teleport"
            );
            snapshot.message_sequence += 1;
            snapshot.player.set_command_time(step);
            prediction.apply_snapshot(&snapshot, &bsp, &mut scratch);
            assert!(!prediction.history_gap.incomplete());
            assert!(!prediction.snap_back);
            assert_eq!(prediction.committed_state().unwrap().origin[0], 1000.0);
            assert_eq!(
                prediction.error.error(),
                [0.0; 3],
                "never smooth across the delayed teleport"
            );
        }
    }

    #[test]
    fn a_fixed_step_gap_requires_the_rounded_endpoint() {
        let mut prediction = LocalPrediction::new(None, None, None, &VirtualFileSystem::new());
        prediction.movement_config.fixed_millis = Some(8);
        for n in 1..=COMMAND_BACKUP + 1 {
            prediction.remember_command(UserCommand {
                server_time: n as i32 * 3,
                ..Default::default()
            });
        }
        prediction.history_gap.acknowledge(3);
        assert!(prediction.history_gap.incomplete());
        prediction.history_gap.acknowledge(8);
        assert!(!prediction.history_gap.incomplete());
    }
}
