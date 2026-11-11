//! Viewer reaction to typed client-session lifecycle events.
//!
//! Protocol parsing remains in `sjk-client`; this module owns the expensive,
//! one-shot world teardown/reload and has no steady-state frame cost.

#[path = "resident_world.rs"]
pub(crate) mod resident;

use super::{GpuState, GpuWorldInput, assets};
use sjk_client::{ServerClock, SessionTransition, SessionTransitionKind};
use sjk_protocol::InfoString;
use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

/// Result of applying a client lifecycle event to the viewer shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Reaction {
    ReloadWorld,
    ReturnToMenu,
}

pub(crate) fn reaction(transition: &SessionTransition) -> Reaction {
    match transition.kind {
        SessionTransitionKind::NewMap
        | SessionTransitionKind::SameMapGamestate
        | SessionTransitionKind::MapRestart => Reaction::ReloadWorld,
        SessionTransitionKind::Disconnected => Reaction::ReturnToMenu,
    }
}

/// Aggregate the normally-empty transition queue without allocating.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Reactions {
    /// A map transition needs the existing world reload path.
    pub(crate) reload_world: bool,
    pub(crate) gamestate_sequence: Option<i32>,
    /// A disconnection needs the connection-failed screen with this reason.
    pub(crate) disconnect_reason: Option<String>,
}

/// Preserve lifecycle reasons while leaving map transition handling unchanged.
pub(crate) fn consume<'a>(transitions: impl Iterator<Item = SessionTransition> + 'a) -> Reactions {
    let mut result = Reactions::default();
    for transition in transitions {
        crate::log::progress(format_args!(
            concat!(
                "session transition: {:?} {:?} -> {:?}, serverId={}, ",
                "gamestate={:?}, firstSnapshot={:?}, reason={:?}"
            ),
            transition.kind,
            transition.old_map,
            transition.new_map,
            transition.server_id,
            transition.gamestate_message_sequence,
            transition.first_snapshot_message_sequence,
            transition.reason,
        ));
        match reaction(&transition) {
            Reaction::ReloadWorld => {
                result.reload_world = true;
                result.gamestate_sequence = transition.gamestate_message_sequence;
            }
            Reaction::ReturnToMenu => {
                result.disconnect_reason = Some(disconnect_reason(transition.reason));
            }
        }
    }
    result
}

fn disconnect_reason(reason: Option<String>) -> String {
    reason.unwrap_or_else(|| "Server closed the connection".to_owned())
}

/// Surface a server or netchan failure in stderr, scrollback, and the menus.
pub(crate) fn show_disconnect(
    reason: String,
    console: Option<&mut crate::console::ViewerConsole>,
    menu: Option<&mut crate::menu::ClientMenu>,
) {
    crate::log::progress(format_args!("Disconnected: {reason}"));
    if let Some(console) = console {
        console.push_log(format!("^1Disconnected: {reason}"));
    }
    if let Some(menu) = menu {
        menu.join_failed(reason);
    }
}

#[path = "world_load.rs"]
mod load;
pub(crate) use load::{WorldLoadPoll, WorldLoadTask};

/// One complete replacement world built against the retained GPU context.
pub(crate) struct WorldInstallTask {
    // Boxed: a world is ~600 KiB, and every by-value move of one (through the
    // channel, the poll result, the adoption) is a stack copy on the main thread,
    // whose nested frames overflowed its 8 MiB joining JoF (02/11/2026).
    receiver: Receiver<Result<Box<GpuState>, String>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    /// The worker's answer once it has arrived, until `poll` hands it out.
    result: Option<Result<Box<GpuState>, String>>,
    pub(crate) map_path: String,
    pub(crate) started: Instant,
    /// It builds the empty world that stands in while the next map builds.
    unload: bool,
}

pub(crate) enum WorldInstallPoll {
    Pending,
    Ready(Box<GpuState>),
    Failed(String),
}

// The poll result stays pointer-sized (see `WorldInstallTask::receiver`).
const _: () = assert!(std::mem::size_of::<WorldInstallPoll>() <= 64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LoadStage {
    Idle,
    Preparing(u64),
    Installing(u64),
    Failed(u64),
}

/// Ordered, cancellable state for CPU preparation followed by background GPU install.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoadStateMachine {
    generation: u64,
    stage: LoadStage,
}

impl LoadStateMachine {
    pub(crate) const fn new() -> Self {
        Self {
            generation: 0,
            stage: LoadStage::Idle,
        }
    }

    fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.stage = LoadStage::Preparing(self.generation);
        self.generation
    }

    fn prepared(&mut self) {
        debug_assert_eq!(self.stage, LoadStage::Preparing(self.generation));
        self.stage = LoadStage::Installing(self.generation);
    }

    fn installed(&mut self) {
        debug_assert_eq!(self.stage, LoadStage::Installing(self.generation));
        self.stage = LoadStage::Idle;
    }

    fn failed(&mut self) -> Reaction {
        self.stage = LoadStage::Failed(self.generation);
        Reaction::ReturnToMenu
    }
}

/// Stack reserved for the world-install worker. See the comment at its spawn.
pub(crate) const WORLD_INSTALL_STACK_BYTES: usize = 64 * 1024 * 1024;

impl WorldInstallTask {
    pub(crate) fn start(
        context: Arc<super::gpu_context::Context>,
        size: [u32; 2],
        input: GpuWorldInput,
        map_path: String,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancelled);
        let started = Instant::now();
        thread::Builder::new()
            .name("sjk-world-install".into())
            // `new_with_context` is async, so `block_on` materialises the whole
            // world-construction future — every intermediate it holds across an
            // await — as a single frame on this thread. That outgrew the 2 MiB
            // a spawned thread gets by default and aborted the process on join
            // ("thread 'sjk-world-install' has overflowed its stack"). The main
            // thread has its own, set in build.rs. Reserve room
            // explicitly rather than depending on how much the future happens
            // to need this month; untouched pages cost nothing.
            .stack_size(WORLD_INSTALL_STACK_BYTES)
            .spawn(move || {
                // wgpu panics on an error nothing captures (the card out of memory, say):
                // the panic hook reports it as ever, and the player is told what it was
                // instead of "stopped unexpectedly".
                let result = caught(|| {
                    pollster::block_on(GpuState::new_with_context(
                        context,
                        size,
                        input,
                        Some(&worker_cancel),
                    ))
                    .map(Box::new)
                    .map_err(|error| error.to_string())
                });
                let _ = sender.send(result);
            })
            .expect("world install thread creation failed");
        Self {
            receiver,
            cancelled,
            result: None,
            map_path,
            started,
            unload: false,
        }
    }

    /// Whether the worker has answered (with a world or an error), without
    /// taking the answer.
    pub(crate) fn settled(&mut self) -> bool {
        if self.result.is_none() {
            self.result = match self.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err("world installer stopped unexpectedly".into()))
                }
            };
        }
        self.result.is_some()
    }

    pub(crate) fn poll(&mut self) -> WorldInstallPoll {
        if !self.settled() {
            return WorldInstallPoll::Pending;
        }
        match self.result.take() {
            Some(Ok(world)) => WorldInstallPoll::Ready(world),
            Some(Err(error)) => WorldInstallPoll::Failed(error),
            None => WorldInstallPoll::Pending,
        }
    }
}

/// `build`'s result, or what it said if it panicked.
fn caught<T>(build: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(build))
        .unwrap_or_else(|payload| Err(panic_message(payload.as_ref())))
}

/// What a caught panic said, for the player.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the world installer failed".to_owned())
}

impl Drop for WorldInstallTask {
    fn drop(&mut self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl GpuState {
    pub(crate) fn poll_world_install(&mut self) -> Result<Option<Box<GpuState>>, Box<dyn Error>> {
        if self.is_menu_world {
            return self.poll_destination();
        }
        if self.join_task.is_some()
            || self
                .resident
                .session
                .as_ref()
                .is_some_and(sjk_client::ClientSession::needs_download)
        {
            return Ok(None);
        }
        if self.pending_map_reload {
            self.world_load_state.begin();
            self.load_event_gap.start(Instant::now());
            self.world_load_task = None;
            self.world_install_task = None;
            let map_path = self.pending_map_path()?;
            self.world_load_started = Some(Instant::now());
            self.world_load_map.clear();
            self.world_load_map.push_str(&map_path);
            crate::log::progress(format_args!(
                "session transition: loading {map_path} on worker"
            ));
            // The old world is a backdrop beneath the loading notice.
            let game = self
                .resident
                .session
                .as_ref()
                .or(self.live_session.as_ref())
                .map(|session| session.game_state().clone());
            if let Some(menu) = &mut self.client_menu {
                menu.state_loading(
                    map_path
                        .trim_start_matches("maps/")
                        .trim_end_matches(".bsp"),
                );
                if let Some(game) = &game {
                    let local = menu.hosting_local();
                    menu.loading_mut()
                        .set_game(game, local, &self.localization.strings);
                }
            }
            let game = self
                .resident
                .session
                .as_ref()
                .map(sjk_client::ClientSession::game_state)
                .or_else(|| self.demo_session.as_ref().map(|s| s.game_state()))
                .ok_or("map transition lost its session")?;
            let selection = assets::session_content::Selection::from_game(game)?;
            self.world_load_task = Some(WorldLoadTask::start_session(
                self.game_data.clone(),
                map_path,
                selection,
            ));
            self.pending_map_reload = false;
            self.world_install_task = self.unload_task();
        }
        if !self.resident.snapshot_ready() {
            return Ok(None);
        }
        if let Some(task) = self.world_install_task.as_mut() {
            return match task.poll() {
                WorldInstallPoll::Pending => Ok(None),
                WorldInstallPoll::Failed(error) => Err(error.into()),
                WorldInstallPoll::Ready(mut world) => {
                    if task.unload {
                        world.map_unloaded = true;
                        crate::log::progress(format_args!(
                            "session transition: old map unloaded in {:.1} ms",
                            task.started.elapsed().as_secs_f64() * 1_000.0
                        ));
                    } else {
                        self.world_load_state.installed();
                        crate::log::progress(format_args!(
                            "session transition: installed {} in {:.1} ms",
                            task.map_path,
                            task.started.elapsed().as_secs_f64() * 1_000.0
                        ));
                    }
                    self.world_install_task = None;
                    Ok(Some(world))
                }
            };
        }
        let Some(task) = &self.world_load_task else {
            return Ok(None);
        };
        let loaded = match task.poll() {
            WorldLoadPoll::Pending => return Ok(None),
            WorldLoadPoll::Failed(error) => return Err(error.into()),
            WorldLoadPoll::Ready(loaded) => loaded,
        };
        self.world_load_task = None;
        self.world_load_state.prepared();
        if let Some(timeline) = &mut self.connect_timeline {
            timeline.mark(crate::log::TimelinePhase::MapLoaded);
        }
        crate::log::progress(format_args!(
            "session transition: parsed {} in {:.1} ms",
            loaded.map_path,
            loaded.elapsed.as_secs_f64() * 1_000.0
        ));
        let game_state = self
            .resident
            .session
            .as_ref()
            .map(sjk_client::ClientSession::game_state)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(|session| session.game_state())
            })
            .ok_or("map transition lost its session")?;
        let snapshot = self
            .resident
            .session
            .as_ref()
            .map(sjk_client::ClientSession::latest_snapshot)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(|session| session.latest_snapshot())
            })
            .ok_or("map transition lost its snapshot")?;
        let player = &snapshot.player;
        let intermission = sjk_client::IntermissionView::from_player_state(player);
        let mut camera_origin = intermission.map_or_else(|| player.origin(), |view| view.origin);
        if intermission.is_none() {
            camera_origin[2] += player.view_height() as f32;
        }
        let camera_yaw = player.view_angles()[1].to_radians();
        let world_bounds = loaded.bsp.render().models()[0].clone();
        let input = GpuWorldInput {
            scene: loaded.scene,
            bsp: loaded.bsp,

            vfs: loaded.vfs,
            shaders: loaded.shaders,
            world_minimums: world_bounds.minimums,
            world_maximums: world_bounds.maximums,
            camera_origin,
            camera_yaw,
            player_preview: None,
            build_game_state: Some(game_state.clone()),
            build_snapshot: Some(snapshot.clone()),
            live_session: None,
            demo_session: None,
            console: None,
            client_menu: None,
            game_data: self.game_data.clone(),
            connect_timeline: self.connect_timeline.clone(),
            game_fonts: crate::game_font::enabled(self.console.as_ref()),

            completed_map_changes: self.completed_map_changes,
            remap_mode: Some(self.remap_mode_for(&loaded.map_path)),
        };
        self.world_install_task = Some(WorldInstallTask::start(
            Arc::clone(self.graphics_context()),
            [self.size.width, self.size.height],
            input,
            loaded.map_path,
        ));
        Ok(None)
    }

    /// Build an empty world to take the shell over from this server world before the
    /// next map is built, as the next map's file is read meanwhile. Building the next
    /// map beside this one held both maps on the graphics card at once, which ran
    /// cards short of memory on a map change or a switch to another server; the
    /// loading screen covers the world either way.
    fn unload_task(&self) -> Option<WorldInstallTask> {
        if self.map_unloaded || self.resident.session.is_none() || self.demo_session.is_some() {
            return None;
        }
        let mut task = WorldInstallTask::start(
            Arc::clone(self.graphics_context()),
            [self.size.width, self.size.height],
            self.empty_world_input()?,
            "empty world".to_owned(),
        );
        task.unload = true;
        Some(task)
    }

    /// The input of the empty world [`Self::unload_task`] builds.
    pub(crate) fn empty_world_input(&self) -> Option<GpuWorldInput> {
        let bsp = crate::menu_pictures::empty_world();
        let scene = sjk_scene::StaticWorld::build(
            &bsp,
            sjk_scene::MeshBuildOptions::default().with_sky_surfaces(),
        )
        .ok()?;
        let bounds = bsp.render().models()[0].clone();
        Some(GpuWorldInput {
            scene,
            bsp,

            vfs: Arc::clone(self.vfs.as_ref()?),
            shaders: (*self.shaders).clone(),
            world_minimums: bounds.minimums,
            world_maximums: bounds.maximums,
            camera_origin: self.camera_position.to_array(),
            camera_yaw: self.camera_yaw,
            player_preview: None,
            build_game_state: None,
            build_snapshot: None,
            live_session: None,
            demo_session: None,
            console: None,
            client_menu: None,
            game_data: self.game_data.clone(),
            connect_timeline: None,
            game_fonts: crate::game_font::enabled(self.console.as_ref()),

            completed_map_changes: self.completed_map_changes,
            remap_mode: None,
        })
    }

    /// Move everything that belongs to the player's session with the client
    /// — sessions, console, menu, pointer state — from this world into `to`.
    /// `to` is built on this world's context, or on a newer one after a graphics
    /// reload ([`crate::graphics_reload`]).
    pub(crate) fn hand_shell_to(&mut self, to: &mut GpuState) {
        debug_assert!(to.context.id >= self.context.id || to.is_menu_world || self.is_menu_world);
        // Both worlds draw to the one window surface: this world's last frame must be
        // presented on its submit thread before `to` reconfigures the surface or takes
        // its next image. Otherwise wgpu refuses the configure ("The `SurfaceOutput`
        // ... must be dropped before re-configuring"), which closed the game as it
        // left a server for the menu world.
        self.frame_pacer.split.wait_previous();
        let now = Instant::now();
        let was_local = self
            .live_session
            .as_ref()
            .is_some_and(sjk_client::ClientSession::is_local);
        to.live_session = self.live_session.take();
        to.resident.session = self.resident.session.take();
        to.resident.map_change_pending = std::mem::take(&mut self.resident.map_change_pending);
        to.demo_session = self.demo_session.take();
        to.console = self.console.take();
        to.net_timing = std::mem::take(&mut self.net_timing);
        to.server_view = std::mem::take(&mut self.server_view);
        to.client_menu = self.client_menu.take();
        to.join_task = self.join_task.take();
        to.connect_timeline = self.connect_timeline.take().or(to.connect_timeline.take());
        to.last_connect_address = self.last_connect_address.take();
        // With the address goes the password its join sent: a world that started
        // with none took a saved `password` for a new one and joined again at every
        // map load, graphics reload and failed join, until the server refused the
        // address ("Too Many players with the same IP", Sol 08/11/2026).
        to.join_password = std::mem::take(&mut self.join_password);
        to.pointer_captured = self.pointer_captured;
        to.cursor_policy = std::mem::replace(
            &mut self.cursor_policy,
            super::pointer_input::CursorPolicy::new(),
        );
        to.cursor_position = self.cursor_position;
        to.applied_display = self.applied_display;
        to.display_suspended = self.display_suspended;
        to.display_changed_at = self.display_changed_at;
        to.applied_resolution = self.applied_resolution;
        to.applied_high_dpi = self.applied_high_dpi;
        // Preserve a remote connection's command floor across world installs.
        // A local authority has an independent timeline that must never stamp
        // the destination server's commands.
        to.server_clock = std::mem::replace(&mut self.server_clock, ServerClock::unanchored(now));
        if was_local {
            to.server_clock.reset_connection(now);
            to.command_schedule.reset();
            to.packet_pacer.reset();
        }
        if to.resident.bound {
            to.server_clock.activate();
        }
        to.graphics_reload.card = std::mem::take(&mut self.graphics_reload.card);
        to.log_request_card = std::mem::take(&mut self.log_request_card);
        to.e2e_whispers = std::mem::take(&mut self.e2e_whispers);
        to.mod_notice = std::mem::take(&mut self.mod_notice);
        to.news = std::mem::take(&mut self.news);
        to.load_event_gap = self.load_event_gap.clone();
        to.hitches = std::mem::take(&mut self.hitches);
        // The trades go on across the map (the toast and what it already said): a world
        // that started without them took the last refusal the hub gave for a new one and
        // showed it again at every map change. The game menu does not go on, so neither
        // does the window's claim to have opened it.
        to.trades = std::mem::take(&mut self.trades).handed_over();
        to.hitches.settle(now);
        to.completed_map_changes = self.completed_map_changes;
        to.gameplay_input.clear();
        to.gameplay_input.inherit_focus(&self.gameplay_input);
        // A chat field open over a map change or a graphics reload stays open (and the
        // talk balloon with it); leaving for the menu closes it.
        if to.live_session.is_some() || to.resident.session.is_some() {
            self.chat.hand_composer_to(&mut to.chat);
        }
    }

    pub(crate) fn adopt_world(&mut self, mut loaded: Box<GpuState>) -> Box<GpuState> {
        self.hand_shell_to(&mut loaded);
        if let Some(console) = &mut loaded.console {
            crate::chat::pm_only::reset(console);
            crate::chat::quiet::reset(console);
        }
        // The world was built at the window's size when its install began; the window
        // may have been resized in the seconds since. Its depth and scene targets would
        // then no longer match the swapchain, and wgpu refuses that frame.
        if loaded.size != self.size {
            loaded.resize(self.size);
        }
        loaded.transition_report_pending = loaded.connect_timeline.is_none();
        loaded.completed_map_changes = self.completed_map_changes.saturating_add(u32::from(
            self.live_map_installed || std::mem::take(&mut loaded.resident.map_change_pending),
        ));
        loaded.live_map_installed = loaded.resident.bound
            && (loaded.resident.session.is_some() || loaded.demo_session.is_some());
        if loaded.live_map_installed {
            loaded.live_session = loaded
                .resident
                .session
                .take()
                .or(loaded.live_session.take());
        } else {
            loaded.pending_map_reload = loaded.resident.session.is_some();
            // The menus keep their loading screen over this world instead
            // of letting the player walk it while the game loads.
            if loaded.client_menu.is_none() {
                loaded.start_exploring();
            }
        }
        if let Some(session) = &mut loaded.live_session {
            session.take_retired_world();
        }
        if loaded.live_map_installed && loaded.live_session.is_some() {
            loaded.reset_live_presentation();
        }
        loaded.world_load_state = self.world_load_state;
        loaded.world_load_started = self.world_load_started;
        loaded.world_load_map.clear();
        loaded.world_load_map.push_str(&self.world_load_map);
        loaded.resident.map.clone_from(&loaded.world_load_map);
        if loaded.map_unloaded {
            // The next map's file goes on loading for the empty world to build it.
            loaded.world_load_task = self.world_load_task.take();
            loaded.pending_map_reload &= loaded.world_load_task.is_none();
            loaded.resident.after_sequence = self.resident.after_sequence;
            // The map transition is reported when the next map is in, not now.
            loaded.transition_report_pending = false;
        }
        crate::vfs_lifetime::log("world attach", loaded.vfs.as_ref());
        loaded
    }

    pub(crate) fn fail_world_install(&mut self, error: String) {
        // Out of video memory: the menus save theirs from now on, and say why.
        let error = if crate::menu_pictures::is_out_of_memory(&error) {
            crate::menu_pictures::after_out_of_memory(self.console.as_mut(), &self.world_load_map)
        } else {
            error
        };
        let reaction = self.world_load_state.failed();
        debug_assert_eq!(reaction, Reaction::ReturnToMenu);
        self.world_load_task = None;
        self.world_install_task = None;
        self.leave_session();
        self.gameplay_input.clear();
        if let Some(menu) = &mut self.client_menu {
            menu.join_failed(error);
        }
    }

    pub(crate) fn pending_map_path(&self) -> Result<String, Box<dyn Error>> {
        let game_state = self
            .resident
            .session
            .as_ref()
            .map(sjk_client::ClientSession::game_state)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(|session| session.game_state())
            })
            .ok_or("map transition lost its session")?;
        let server_info = game_state
            .config_string(0)
            .ok_or("new gamestate has no CS_SERVERINFO")?;
        let server_info = InfoString::parse(std::str::from_utf8(server_info)?)?;
        let map_name = server_info
            .get("mapname")
            .ok_or("new CS_SERVERINFO has no mapname")?;
        Ok(format!("maps/{map_name}.bsp"))
    }
}

#[cfg(test)]
mod install_tests {
    use super::*;

    #[test]
    fn a_cancelled_install_worker_releases_its_vfs() {
        let vfs = Arc::new(sjk_vfs::VirtualFileSystem::new());
        let weak = Arc::downgrade(&vfs);
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            while !worker_cancelled.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::yield_now();
            }
            drop(vfs);
        });
        let task = WorldInstallTask {
            receiver,
            cancelled,
            result: None,
            map_path: String::new(),
            started: Instant::now(),
            unload: false,
        };

        drop(task);
        drop(sender);
        worker.join().unwrap();

        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn a_panicking_install_reports_what_it_said() {
        let result: Result<(), String> = caught(|| panic!("wgpu error: Out of Memory"));
        let error = result.expect_err("the panic is an error");
        assert_eq!(error, "wgpu error: Out of Memory");
        assert!(crate::menu_pictures::is_out_of_memory(&error));
        let formatted: Result<(), String> = caught(|| panic!("buffer {} is invalid", 7));
        assert_eq!(formatted.expect_err("an error"), "buffer 7 is invalid");
        assert_eq!(caught(|| Ok::<_, String>(3)), Ok(3));
    }
}
