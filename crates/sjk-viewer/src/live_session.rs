//! Per-frame live network polling and command submission.

use super::{GameAudio, GpuState, server_commands, session_transition};
use std::time::{Duration, Instant};

#[path = "emplaced_view.rs"]
mod emplaced_view;

/// Most snapshots one frame reads; the rest wait for the next frames.
pub(crate) const SNAPSHOTS_PER_FRAME: usize = 8;

/// Consume queued snapshots in order, stopping on timeout/failure or after
/// [`SNAPSHOTS_PER_FRAME`]. True when it stopped before that (the queue ran dry
/// or a snapshot ended the loop), false when packets may still be waiting.
pub(crate) fn drain_snapshots(mut receive_and_present: impl FnMut() -> bool) -> bool {
    for _ in 0..SNAPSHOTS_PER_FRAME {
        if !receive_and_present() {
            return true;
        }
    }
    false
}

impl GpuState {
    /// Only the installed live world shares the session's snapshot time domain.
    pub(crate) fn live_presentation_ready(&self) -> bool {
        if self
            .live_session
            .as_ref()
            .is_some_and(sjk_client::ClientSession::is_local)
        {
            return self.live_map_installed && !self.is_menu_world;
        }
        self.live_session.is_some()
            && self.live_map_installed
            && !self.is_menu_world
            && !self.pending_map_reload
            && self.world_load_task.is_none()
            && self.world_install_task.is_none()
    }

    /// Drain queued snapshots, surface lifecycle failures, and submit due input.
    pub(crate) fn update_live_session(
        &mut self,
        game_audio: &mut Option<GameAudio>,
        visual_now: Instant,
        timing: &mut crate::frame_pacing::budget::Timer,
    ) {
        use crate::frame_pacing::budget::Phase;
        timing.mark(Phase::Snapshot);
        let caught_up = drain_snapshots(|| {
            let Some(session) = &mut self.live_session else {
                return false;
            };
            timing.mark(Phase::Receive);
            let received = session.receive_snapshot(Duration::ZERO).cloned();
            timing.mark(Phase::Snapshot);
            // svc_gamestate may have been installed even if the following
            // snapshot times out. Consume its clock reset before sending input.
            let reactions = session_transition::consume(session.drain_transitions());
            if received.is_ok()
                && !reactions.reload_world
                && !self.pending_map_reload
                && self.world_load_task.is_none()
                && self.world_install_task.is_none()
                && let (Some(audio), Some(vfs)) = (game_audio.as_mut(), &self.vfs)
            {
                audio.prepare_config_strings(
                    session.config_string_changes(),
                    session.game_state(),
                    vfs,
                );
            }
            if reactions.reload_world {
                if let Some(console) = &mut self.console {
                    console.reset_dialogue();
                }
                if let Some(audio) = game_audio.as_mut() {
                    audio.begin_map_change();
                }
                // Before presenting: the server reports inflated snapshot
                // times until it sees this client acknowledge the new
                // `serverId` (`sv_snapshot.cpp:190-198`), and a command
                // stamped from that era outranks every later one once
                // `SV_ClientEnterWorld` latches it (`sv_client.cpp:576`).
                // Stock stamps the whole map load 0 (`CL_ClearState`); the
                // world built for the new timeline anchors the clock again.
                self.server_clock.restart();
                self.pending_map_reload = true;
                self.ghost_leave();
                self.retain_world_for_connection();
                self.resident.after_sequence = reactions.gamestate_sequence.or_else(|| {
                    self.resident
                        .session
                        .as_ref()
                        .map(|s| s.latest_snapshot().message_sequence)
                });
            }
            if let Some(reason) = reactions.disconnect_reason {
                self.session_disconnected(reason);
                return false;
            }
            if reactions.reload_world {
                return false;
            }
            let snapshot = match received {
                Ok(snapshot) => snapshot,
                Err(error) if error.is_timeout() => return false,
                Err(error) => {
                    self.session_disconnected(error.to_string());
                    return false;
                }
            };
            if self.live_session.is_none() {
                return false;
            }
            // CG_ProcessSnapshots does not install SNAPFLAG_NOT_ACTIVE frames.
            // These may contain cleared player state while a new map is primed.
            if !active_snapshot(&snapshot) {
                return true;
            }
            self.net_timing.snapshot_received(snapshot.server_time);
            self.present_live_snapshot(&snapshot, true, game_audio, visual_now);
            true
        });
        // Stock adjusts the command clock once a frame, on the newest snapshot,
        // after reading every queued packet (`CL_SetCGameTime`). After a hitch the
        // queue holds a second of snapshots that this frame's eight do not empty:
        // their times are as old as the hitch, and adjusting on them put the
        // commands a second behind the server, read as a huge `cl_timeNudge`
        // (JoF's Heimdall kicks for it). The clock keeps its delta until the queue
        // is read ([`sjk_client::ServerClock::adjust`]).
        if caught_up {
            self.server_clock.adjust(Instant::now());
        }
        timing.mark(Phase::Commands);
        self.ghost_frame();
        let talking = self.talk_button();
        // The FPS cap the gameplay FPS's Auto follows (`command_rate`).
        let frame_cap = self.frame_cap();
        // A held push or pull key fills every command no press filled
        // (`input::force_repeat`), while the keys are the game's.
        let force_repeat = if self.text_has_keyboard() {
            None
        } else {
            self.console
                .as_ref()
                .and_then(crate::console::ViewerConsole::force_repeat_command)
        };
        self.refresh_force_support();
        self.apply_anti_emote();
        let Some(session) = &mut self.live_session else {
            return;
        };
        if let Some(silence) = self.net_timing.silence_to_report(Instant::now()) {
            // A session with no snapshots reports nothing else, so say what
            // the socket sees: a quiet server, packets arriving and being
            // discarded, or packets going out to nobody.
            let traffic = session.connection_traffic();
            let expected = session.server();
            let rejected = session
                .last_rejected_packet()
                .map(|(source, bytes)| format!("{source} {:?}", String::from_utf8_lossy(bytes)));
            crate::log::progress(format_args!(
                "no snapshots for {:.1} s: sent={} received={} stale={} wrong-source={} \
                 out-of-band={} expected={expected} last-rejected={}",
                silence.as_secs_f64(),
                traffic.sent,
                traffic.received,
                traffic.stale,
                traffic.wrong_source,
                traffic.out_of_band,
                rejected.as_deref().unwrap_or("none"),
            ));
        }
        let dialogue_was_active = self
            .console
            .as_ref()
            .is_some_and(|c| c.server_dialogue_active());
        server_commands::consume(
            session,
            &self.localization,
            self.resident.session.is_none().then_some(&mut self.chat),
            self.console.as_mut(),
            self.legacy_world_adapter.as_mut(),
            &mut self.clientinfo_watch,
            &mut self.hud.duel_pairs,
            game_audio.as_mut(),
        );

        let dialogue_active = self
            .console
            .as_ref()
            .is_some_and(|c| c.server_dialogue_active());
        if dialogue_active && !dialogue_was_active {
            self.gameplay_input.release_keys();
            self.pending_generic_command = 0;
            self.generic_command_sent = 0;
        }
        let talking = talking || dialogue_active;
        let force_repeat = if dialogue_active { None } else { force_repeat };

        // User commands are made on the gameplay FPS's grid and wait for the next
        // `cl_maxpackets` packet (`command_rate`).
        let now = Instant::now();
        crate::emote_end_debug::observe_frame(now);
        let server_time = self.server_clock.server_time(now);
        let (step, pacing) = self.console.as_mut().map_or(
            (sjk_client::command_rate::COMMAND_MILLIS, Default::default()),
            |console| {
                console.set_frame_cap(frame_cap);
                (console.command_step(), console.command_pacing())
            },
        );
        let due =
            self.command_schedule
                .due(server_time, self.server_clock.anchored(), now, step, pacing);
        if let Some(console) = &self.console {
            session.set_packet_dup(console.packet_dup());
        }
        if due.as_slice().is_empty() {
            self.send_due_packet(now);
            return;
        }
        let intermission =
            session.latest_snapshot().player.movement_type() == sjk_client::PM_INTERMISSION;
        if intermission
            && self
                .scoreboard
                .score_request_due(session.latest_snapshot().server_time)
            && let Err(error) = session.send_reliable_command(b"score")
        {
            eprintln!("failed to request intermission scoreboard: {error}");
        }
        let snapshot = session.latest_snapshot();
        let snapshot_origin = snapshot.player.origin();
        let delta_angles = self
            .gameplay_input
            .view_authority
            .command_delta()
            .unwrap_or(snapshot.player.delta_angles());
        self.gameplay_input
            .selection
            .sync(&snapshot.player, server_time);
        // At an emplaced gun, a view past its arc is turned back (`CG_EmplacedView`).
        // The camera's yaw is in radians, the game's in degrees.
        if let Some(yaw) = emplaced_view::forced_yaw(snapshot, self.camera_yaw.to_degrees()) {
            self.camera_yaw = yaw.to_radians();
        }
        let held_weapon = snapshot.player.weapon();
        let weapon = self.selected_weapon.unwrap_or(held_weapon);
        let force_power = snapshot.player.selected_force_power();
        let known = self.gameplay_input.selection.known(&snapshot.player);
        let own_slot = u8::try_from(snapshot.player.client_num()).ok();
        let attack_stops_emote = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(crate::emotes::ATTACK_STOP_CVAR))
            .unwrap_or(crate::emotes::DEFAULT_ATTACK_STOP);
        let flip_kick = self
            .console
            .as_ref()
            .map(crate::console::ViewerConsole::flip_kick_timing)
            .unwrap_or_default();
        // A frame slower than 8 ms makes a command for each slot it passed; the
        // first carries the frame's presses, the rest the keys still held.
        let last_stamp = due.as_slice().last().copied().unwrap_or_default();
        for &stamp in due.as_slice() {
            self.gameplay_input
                .set_command_age(u64::try_from(last_stamp.saturating_sub(stamp)).unwrap_or(0));
            let mut command = self.gameplay_input.user_command(
                stamp,
                self.camera_pitch,
                self.camera_yaw,
                delta_angles,
                weapon,
                force_power,
                match self.pending_generic_command {
                    0 => force_repeat.unwrap_or(0),
                    pressed => pressed,
                },
            );
            self.gameplay_input.apply_flip_kick(&mut command, flip_kick);
            // In the cardboard box the player crouches (`cardboard_box`), but not
            // while flying the free camera, whose `upmove` it would steer down.
            self.cardboard_box
                .set_flying(self.local_prediction.fake_noclip());
            self.cardboard_box.shape_command(&mut command);
            // Moving, jumping or attacking puts the rod away; `+use` hooks a bite
            // (`fishing`).
            self.fishing.shape_command(&command);
            // The attack button (or `+use`) swings the pickaxe and is kept from the weapon;
            // an alt attack puts it away (`mining`).
            self.mining.shape_command(&mut command);
            // Attacks count toward the play the holocron claims report.
            crate::holocrons::signals::command(command.buttons);
            // A Hax hack starts and holds on `+use` (`hax`).
            self.hax.observe_use(&command);
            // `+use` at a mystery box opens it (`mystery_box`).
            self.mystery_boxes.observe_use(&command);
            // `+use` on a toy the Toy Master hid (`toy_master`).
            self.toy_master.observe_use(&command);
            // Getting up from a JA+ away emote crouches (`away_emote`).
            crate::away_emote::shape_command(&mut command);
            command.buttons = sjk_game_jka::pmove_talk::command_buttons(command.buttons, talking);
            // JoF EJK's Force wheel: a selected Stasis, Repulse or Dash takes
            // `+useforce`.
            let (buttons, wheel_command) = self
                .gameplay_input
                .force_wheel_buttons(command.buttons, known);
            command.buttons = buttons;
            // A ghost replay sends the recorded command instead (`ghost_replay`).
            if self.local_prediction.fake_noclip() {
                self.ghost.interrupt("you are flying");
                self.mimic.stop_with("^3mimic: stopped, you are flying.");
            }
            let pose = self
                .local_prediction
                .committed_state()
                .map(crate::ghost_replay::Pose::of);
            // The Hax toy steers, or stands still while steering another (`hax`); a
            // limp player's real body walks to their ragdoll (`ragdoll`), though not
            // while they fly the free camera: the flight is theirs, and the server's body
            // is held still (`command_for_server`).
            let flying = self.local_prediction.fake_noclip();
            let origin = self
                .local_prediction
                .committed_state()
                .map_or(snapshot_origin, |state| state.origin);
            // A jump, attack or taunt gets a limp player up, though not while flying the
            // free camera, whose rise is `upmove` and whose speed key a wake button
            // (#232's review: the first rise ended the ragdoll).
            if !flying {
                self.ragdoll.wake(&command);
            }
            // A held prop gun fires instead of the real weapon (`propgun`).
            self.propgun.shape_command(&mut command);
            let replay = self
                .hax
                .drive(&command, delta_angles, (self.camera_pitch, self.camera_yaw))
                .or_else(|| {
                    (!flying)
                        .then(|| self.ragdoll.drive(&command, delta_angles, origin))
                        .flatten()
                })
                .or_else(|| pose.and_then(|pose| self.ghost.drive(&command, &pose)));
            // Or the mimic copies another player (`mimic`).
            let replay = replay.or_else(|| {
                let state = self.local_prediction.committed_state()?;
                self.mimic.drive(&command, state)
            });
            if let Some(drive) = &replay {
                command = drive.command;
                if let Some([pitch, yaw]) = drive.view {
                    self.camera_pitch = (-pitch).to_radians();
                    self.camera_yaw = yaw.to_radians();
                }
            }
            if !self.local_prediction.free_camera()
                && replay.is_none()
                && let Some(wheel_command) = wheel_command
                && let Err(error) = session.send_reliable_command(wheel_command.as_bytes())
            {
                eprintln!("failed to send {wheel_command}: {error}");
            }
            let sent = self.local_prediction.command_for_server(command);
            session.queue_command(&sent);
            crate::emote_end_debug::observe_command(&sent, held_weapon, now);
            // With `cg_emoteAttackStop`, an attack or alt attack ends the player's
            // own emote, here and, as `sjkemote stop` does, for the other SJK
            // players: the server attacks whatever the emote shows.
            if let Some(own) = own_slot
                && crate::emotes::lock()
                    .active
                    .end_on_attack(own, sent.buttons, attack_stops_emote)
            {
                let _ = crate::player_identity::emote(crate::emotes::STOP.to_owned(), None);
            }
            self.gameplay_input.finish_command();
            if !intermission
                && let Some(position) = self.local_prediction.apply_command(
                    command,
                    sent,
                    &self.bsp,
                    &mut self.trace_scratch,
                )
            {
                self.camera_position = position;
            }
            if !intermission && let Some(pose) = pose {
                self.ghost
                    .record(&command, &pose, self.local_prediction.committed_state());
            }
            // A pressed `generic_cmd` goes on every command until a packet takes
            // them (`CL_FinishMove`, cleared in `CL_WritePacket`): a single command
            // the server skips (`ClientThink_real` drops one whose `serverTime` it
            // clamped onto the last) no longer loses the press (hub report 172).
            // The server's 300 ms debounce takes it once. A local game takes each
            // command at once.
            if session.queued_commands() == 0 {
                self.pending_generic_command = 0;
                self.generic_command_sent = 0;
            } else {
                self.generic_command_sent = self.pending_generic_command;
            }
        }
        self.gameplay_input.set_command_age(0);
        self.report_emote_end(now);
        self.send_due_packet(now);
        self.sync_cursor_policy();
    }

    /// `cg_debugEmoteEnd`: say in the console what came before a JA+ emote let go of the
    /// player ([`crate::emote_end_debug`]).
    fn report_emote_end(&mut self, now: Instant) {
        let (Some(session), Some(console)) = (&self.live_session, &mut self.console) else {
            return;
        };
        let enabled = console.bool_cvar(crate::emote_end_debug::CVAR) == Some(true);
        crate::emote_end_debug::set_enabled(enabled);
        if !enabled {
            return;
        }
        let movement_type = session.latest_snapshot().player.movement_type();
        let context = crate::emote_end_debug::Context {
            unfocused: console.bool_cvar("com_unfocused") == Some(true)
                || console.bool_cvar("com_minimized") == Some(true),
            console_open: console.is_open(),
            collection_open: console.collection_stage_wish().is_some(),
        };
        if let Some(line) = crate::emote_end_debug::report(movement_type, context, now) {
            console.push_log(line);
        }
    }

    /// Send the queued user commands once `cl_maxpackets` allows a packet.
    fn send_due_packet(&mut self, now: Instant) {
        let Some(session) = &mut self.live_session else {
            return;
        };
        let max_packets = self.console.as_ref().map_or(
            crate::console::DEFAULT_MAX_PACKETS,
            crate::console::ViewerConsole::max_packets,
        );
        if session.queued_commands() == 0 || !self.packet_pacer.ready(now, max_packets) {
            return;
        }
        if let Err(error) = session.send_queued_commands() {
            self.session_disconnected(error.to_string());
            return;
        }
        self.packet_pacer.sent(now);
        // A press made after the packet's commands waits for the next one.
        if std::mem::take(&mut self.generic_command_sent) == self.pending_generic_command {
            self.pending_generic_command = 0;
        }
    }
}

pub(crate) fn active_snapshot(snapshot: &sjk_protocol::Snapshot) -> bool {
    snapshot.flags & 2 == 0
}
