//! The graphics reload (`docs/rendering.md`, "Graphics reload"): the settings the
//! renderer reads only when its graphics context is made (scene HDR, FXAA,
//! supersampling and render scale, sun and sky, sun shadows, light shafts, material
//! maps, reflection probes and texture filtering, so the renderer choice and the
//! graphics quality levels too) apply without restarting SJK.
//!
//! The device, window and surface stay; a new [`Context`] on them reads those settings
//! again ([`Context::resampled`]) and the world on show is built once more on it, the
//! way a map change builds the next map's world:
//!
//! - On a server (or a local game), it is a map change to the same map: the session
//!   waits in the resident world (`resident_world.rs`) under the loading screen while
//!   the map loads, then the new world takes the shell over. The connection stays up.
//! - On the menu world, the map is loaded again in the background while the menu keeps
//!   running over the old world, and the new one takes over when it is ready
//!   ([`Rebuild`]). The menu world, dropped during a match, is built again the same way
//!   when the menus come back to it ([`crate::menu_world`]), on the context of the day.
//!
//! `vid_restart` reloads the graphics when one of those settings changed, as it reloads
//! the renderer in the original game, and the reload card ([`card`]) offers it once the
//! player leaves Settings for a menu. `vid_restart force` reloads them with nothing
//! changed too ([`wanted`]).

use crate::frame_target::scale::NATIVE;
use crate::gpu_context::Context;
use crate::session_transition::{WorldInstallPoll, WorldInstallTask, WorldLoadPoll, WorldLoadTask};
use crate::world_materials::{filtering, material_maps};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "graphics_reload_card.rs"]
pub(crate) mod card;

/// How a notice ends: what applies the change.
pub(crate) const APPLY: &str = "reload the graphics to apply (vid_restart)";

/// One of the settings changed since the last frame looked.
static NOTICE: AtomicBool = AtomicBool::new(false);

/// A setting read only when the graphics context is made changed: the next frame
/// checks whether the graphics still run with the settings ([`crate::GpuState`]'s card).
pub(crate) fn notice() {
    NOTICE.store(true, Ordering::Relaxed);
}

/// Whether a notice came since the last call.
fn take_notice() -> bool {
    NOTICE.swap(false, Ordering::Relaxed)
}

/// The settings a context reads only when it is made, in comparable form.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Startup {
    hdr: u32,
    fxaa: bool,
    scale: u32,
    high_dpi: bool,
    day: bool,
    /// Sun shadows on, with world casters, then the sharp and close cascades' depths
    /// (bits), the map's side, the filter's taps and whether static casters are held.
    shadows: (bool, bool, u32, u32, u32, u32, bool),
    /// The sky shows the hour without the live sun (`r_daySky`, `r_dayClock`).
    sky: bool,
    lamp_shadows: bool,
    bounce: bool,
    shafts: u32,
    maps: material_maps::Settings,
    filtering: filtering::Policy,
}

impl Startup {
    /// What `context` runs with.
    fn of(context: &Context) -> Self {
        let sun = &context.sun_shadows;
        Self {
            hdr: context.hdr.mode,
            fxaa: context.fxaa,
            scale: context.render_scale,
            high_dpi: context.high_dpi,
            day: sun.day.enabled,
            shadows: (
                sun.enabled,
                sun.world,
                sun.distance.to_bits(),
                sun.near.to_bits(),
                sun.resolution,
                sun.taps,
                sun.held,
            ),
            sky: sun.day.sky().enabled,
            lamp_shadows: sun.lamp_shadows,
            bounce: sun.bounce,
            shafts: sun.volumetrics,
            maps: context.material_maps,
            filtering: context.filtering,
        }
    }

    /// What a context made now would run with, read without latching anything.
    fn read(console: &crate::console::ViewerConsole, anisotropy_limit: u16) -> Self {
        let console = Some(console);
        let sun = crate::world_materials::shadows::settings::Settings::sample(console);
        Self {
            hdr: crate::frame_target::aa::hdr::Settings::sample(console).mode,
            fxaa: crate::frame_target::aa::enabled(console),
            scale: crate::frame_target::scale::requested(console),
            high_dpi: console.map_or(crate::settings::HIGH_DPI_DEFAULT, crate::settings::high_dpi),
            day: sun.day.enabled,
            shadows: (
                sun.enabled,
                sun.world,
                sun.distance.to_bits(),
                sun.near.to_bits(),
                sun.resolution,
                sun.taps,
                sun.held,
            ),
            sky: sun.day.sky().enabled,
            lamp_shadows: sun.lamp_shadows,
            bounce: sun.bounce,
            shafts: sun.volumetrics,
            maps: material_maps::Settings::read(console),
            filtering: filtering::Policy::read(console, anisotropy_limit),
        }
    }

    /// The names of what `wanted` changes, in Settings' words and order.
    fn changes(self, wanted: Self) -> Vec<&'static str> {
        let (maps, other) = (self.maps, wanted.maps);
        [
            ("HDR", self.hdr != wanted.hdr),
            ("FXAA", self.fxaa != wanted.fxaa),
            // One number, the scene's percent of the window: named by the side it moved.
            (
                "supersampling",
                self.scale.max(NATIVE) != wanted.scale.max(NATIVE),
            ),
            (
                "render scale",
                self.scale.min(NATIVE) != wanted.scale.min(NATIVE),
            ),
            ("high DPI", self.high_dpi != wanted.high_dpi),
            ("normal maps", maps.normal != other.normal),
            ("specular maps", maps.specular != other.specular),
            ("parallax", maps.parallax != other.parallax),
            ("emission maps", maps.emission != other.emission),
            ("reflection probes", maps.reflections != other.reflections),
            ("real-time lighting", self.day != wanted.day),
            ("time-of-day sky", self.sky != wanted.sky),
            ("lamp shadows", self.lamp_shadows != wanted.lamp_shadows),
            ("bounced light", self.bounce != wanted.bounce),
            ("light shafts", self.shafts != wanted.shafts),
            ("sun shadows", self.shadows != wanted.shadows),
            ("texture filtering", self.filtering != wanted.filtering),
        ]
        .into_iter()
        .filter_map(|(name, changed)| changed.then_some(name))
        .collect()
    }
}

/// Whether `vid_restart` reloads the graphics: when one of the settings changed
/// (`changes`, [`crate::GpuState::graphics_changes`]), or always when `force`d.
pub(crate) fn wanted(force: bool, changes: &[&str]) -> bool {
    force || !changes.is_empty()
}

/// What a reload changes, in the console's and the log's words: the
/// [`summary`] of `changes`, or that it was forced with none.
fn reason(changes: &[&str]) -> String {
    if changes.is_empty() {
        "forced, no setting changed".into()
    } else {
        summary(changes)
    }
}

/// The changes in words: "Sun and sky, sun shadows and light shafts", "HDR, FXAA and 3
/// more".
pub(crate) fn summary(changes: &[&str]) -> String {
    let named = changes.len().min(if changes.len() > 3 { 2 } else { 3 });
    let mut words = changes[..named].join(", ");
    match changes.len() - named {
        0 if named > 1 => {
            let last = words.rfind(", ").unwrap_or(0);
            words.replace_range(last..last + 2, " and ");
        }
        0 => {}
        more => words.push_str(&format!(" and {more} more")),
    }
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// A world without a session being built again on a newer context: the map is read in
/// the background, then installed, while the old world stays on show.
pub(crate) struct Rebuild {
    context: Arc<Context>,
    map: String,
    load: Option<WorldLoadTask>,
    install: Option<WorldInstallTask>,
    /// The menu world coming back after a match ([`crate::menu_world`]), not a reload.
    menu: bool,
}

/// The reload's state, handed from world to world with the shell.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) card: card::Card,
    /// The map a world without a session shows, to load it again.
    pub(crate) map: String,
    rebuild: Option<Rebuild>,
    /// The menu world could not be built again here: not tried again on this world.
    pub(crate) menu_failed: bool,
}

impl State {
    /// Whether a world is being built again for the reload.
    pub(crate) fn rebuilding(&self) -> bool {
        self.rebuild.is_some()
    }
}

impl crate::GpuState {
    /// The context the graphics run with, or the one the world loading now is built on.
    pub(crate) fn graphics_context(&self) -> &Arc<Context> {
        self.next_context.as_ref().unwrap_or(&self.context)
    }

    /// What the graphics would change on a reload, in Settings' words; empty when they
    /// run with the settings.
    pub(crate) fn graphics_changes(&self) -> Vec<&'static str> {
        let Some(console) = &self.console else {
            return Vec::new();
        };
        Startup::of(self.graphics_context())
            .changes(Startup::read(console, self.context.anisotropy_limit))
    }

    /// Build the world on show again on a context with the settings read anew. Returns
    /// the console's answer. Without `force` it does nothing while the graphics run with
    /// the settings; with it the world is built again all the same, on a context that
    /// reads the same settings.
    pub(crate) fn reload_graphics(
        &mut self,
        audio: &mut Option<crate::GameAudio>,
        force: bool,
    ) -> Result<String, String> {
        let changes = self.graphics_changes();
        if !wanted(force, &changes) {
            return Ok("The graphics already run with these settings".into());
        }
        if self.world_load_task.is_some()
            || self.world_install_task.is_some()
            || self.pending_map_reload
            || self.join_task.is_some()
            || self.resident.session.is_some()
            || self.graphics_reload.rebuilding()
        {
            return Err("A map is loading: reload the graphics once it is in".into());
        }
        if self.demo_session.is_some() || self.resident.exploring() {
            return Err(
                "The graphics reload with the next map; leave the demo or map to reload now".into(),
            );
        }
        let on_server = self.live_session.is_some();
        if (on_server && (!self.live_map_installed || self.is_menu_world))
            || (!on_server && self.graphics_reload.map.is_empty())
        {
            return Err("Nothing to reload here: the graphics reload with the next map".into());
        }
        let summary = reason(&changes);
        let context = self.context.resampled(self.console.as_ref());
        if on_server {
            // As a map change to the same map (`session_transition`): the session waits
            // in the resident world while the map loads.
            if let Some(audio) = audio {
                audio.begin_map_change();
            }
            self.next_context = Some(context);
            self.retain_world_for_reload();
            crate::log::progress(format_args!("graphics reload: {summary}; map reloads"));
            return Ok(format!("Reloading the graphics: {summary}"));
        }
        self.rebuild_world(context, true);
        crate::log::progress(format_args!("graphics reload: {summary}"));
        Ok(format!("Reloading the graphics: {summary}"))
    }

    /// Load this world's map again and build it on `context`; the old world stays on show
    /// until [`Self::poll_rebuild`] hands the new one over. `shown`: the card says so
    /// meanwhile (the player asked for it), rather than the menus going on over the old
    /// world (a parked menu world catching up with a reload made on a server).
    pub(crate) fn rebuild_world(&mut self, context: Arc<Context>, shown: bool) {
        let map = self.graphics_reload.map.clone();
        let Some(vfs) = self.vfs.clone() else {
            return;
        };
        crate::log::progress(format_args!("graphics reload: loading {map} again"));
        self.graphics_reload.rebuild = Some(Rebuild {
            load: Some(WorldLoadTask::start(vfs, map.clone())),
            install: None,
            context,
            map,
            menu: false,
        });
        if shown {
            self.graphics_reload.card.reloading(true);
        }
    }

    /// Build the menu world on `map` again on this world's context, from the installed
    /// game data as at start; this world stays on show under the menus until
    /// [`Self::poll_rebuild`] hands the menu world over.
    pub(crate) fn rebuild_menu_world(&mut self, map: String) {
        crate::log::progress(format_args!("menu world: loading {map} again"));
        self.graphics_reload.rebuild = Some(Rebuild {
            load: Some(WorldLoadTask::start_installed(
                self.game_data.clone(),
                map.clone(),
            )),
            install: None,
            context: Arc::clone(&self.context),
            map,
            menu: true,
        });
    }

    /// The rebuilt world once it is ready, the shell handed to it; the caller makes it the
    /// current world. A failed rebuild keeps the old world and says why.
    pub(crate) fn poll_rebuild(&mut self) -> Option<Box<crate::GpuState>> {
        let rebuild = self.graphics_reload.rebuild.as_mut()?;
        // Not over a join: the menu world it leaves is the one parked, and a menu world
        // coming back waits until the menus are back again.
        if self.join_task.is_some() || self.resident.session.is_some() {
            let menu = rebuild.menu;
            self.graphics_reload.rebuild = None;
            if !menu {
                self.graphics_reload.card.reloading(false);
                // The world it joins is built on this one's context: offer the reload again.
                notice();
            }
            return None;
        }
        if let Some(load) = &rebuild.load {
            let loaded = match load.poll() {
                WorldLoadPoll::Pending => return None,
                WorldLoadPoll::Failed(error) => return self.fail_rebuild(&error),
                WorldLoadPoll::Ready(loaded) => loaded,
            };
            let bounds = loaded.bsp.render().models()[0].clone();
            // The menu world starts where it did at start; its camera routes take over.
            let (camera_origin, camera_yaw) = if rebuild.menu {
                match crate::assets::initial_camera(&loaded.bsp) {
                    Ok(camera) => camera,
                    Err(error) => return self.fail_rebuild(&error.to_string()),
                }
            } else {
                (self.camera_position.to_array(), self.camera_yaw)
            };
            let input = crate::GpuWorldInput {
                scene: loaded.scene,
                bsp: loaded.bsp,
                vfs: loaded.vfs,
                shaders: loaded.shaders,
                world_minimums: bounds.minimums,
                world_maximums: bounds.maximums,
                camera_origin,
                camera_yaw,
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
            };
            rebuild.load = None;
            rebuild.install = Some(WorldInstallTask::start(
                Arc::clone(&rebuild.context),
                [self.size.width, self.size.height],
                input,
                loaded.map_path,
            ));
            return None;
        }
        let install = rebuild.install.as_mut()?;
        let mut world = match install.poll() {
            WorldInstallPoll::Pending => return None,
            WorldInstallPoll::Failed(error) => return self.fail_rebuild(&error),
            WorldInstallPoll::Ready(world) => world,
        };
        let map = std::mem::take(&mut rebuild.map);
        let menu = rebuild.menu;
        let elapsed = install.started.elapsed();
        self.graphics_reload.rebuild = None;
        self.hand_shell_to(&mut world);
        world.is_menu_world = self.is_menu_world || menu;
        world.graphics_reload.map = map;
        if menu {
            crate::menu_pictures::fit_backdrop(&mut world);
        }
        if !menu {
            world.graphics_reload.card.reloaded();
            world.camera_position = self.camera_position;
            world.camera_pitch = self.camera_pitch;
            world.camera_yaw = self.camera_yaw;
        }
        world.resize(self.size);
        crate::log::progress(format_args!(
            "{}: {} built again in {:.1} ms",
            if menu {
                "menu world"
            } else {
                "graphics reload"
            },
            world.graphics_reload.map,
            elapsed.as_secs_f64() * 1_000.0
        ));
        Some(world)
    }

    fn fail_rebuild(&mut self, error: &str) -> Option<Box<crate::GpuState>> {
        if self
            .graphics_reload
            .rebuild
            .as_ref()
            .is_some_and(|rebuild| rebuild.menu)
        {
            crate::log::progress(format_args!("menu world failed to load again: {error}"));
            let map = self
                .graphics_reload
                .rebuild
                .take()
                .map(|rebuild| rebuild.map)
                .unwrap_or_default();
            // Out of video memory: it comes back as pictures instead, which need little.
            let retry = crate::menu_pictures::is_out_of_memory(error)
                && crate::menu_pictures::pictured(&map).is_none();
            if retry {
                let notice = crate::menu_pictures::after_out_of_memory(self.console.as_mut(), &map);
                if let Some(console) = &mut self.console {
                    console.push_log(format!("^3{notice}"));
                }
            }
            self.graphics_reload.menu_failed = !retry;
            return None;
        }
        crate::log::progress(format_args!("graphics reload failed: {error}"));
        if let Some(console) = &mut self.console {
            console.push_log(format!("^1Graphics reload failed: {error}"));
        }
        self.graphics_reload.rebuild = None;
        self.graphics_reload.card.reloading(false);
        None
    }

    /// Check the settings when one of them changed: the card offers a reload while the
    /// graphics differ from them, and stops offering it once they match again.
    pub(crate) fn sync_graphics_reload(&mut self) {
        if take_notice() {
            let changes = self.graphics_changes();
            let level = self
                .console
                .as_ref()
                .and_then(crate::graphics_quality::Level::current)
                .map(crate::graphics_quality::Level::label);
            self.graphics_reload.card.offer(&changes, level);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, crate::console::ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn the_same_settings_change_nothing() {
        let (_directory, mut console) = console();
        let before = Startup::read(&console, 16);
        assert!(before.changes(Startup::read(&console, 16)).is_empty());
        // Live settings are not the reload's.
        console.set_cvar("r_hdrExposure", "2");
        console.set_cvar("r_dayHour", "18");
        assert!(before.changes(Startup::read(&console, 16)).is_empty());
    }

    #[test]
    fn ejk_names_what_it_turns_off() {
        let (_directory, mut console) = console();
        let before = Startup::read(&console, 16);
        crate::graphics_quality::Level::Vanilla.apply(&mut console);
        let changes = before.changes(Startup::read(&console, 16));
        for name in [
            "HDR",
            "FXAA",
            "normal maps",
            "real-time lighting",
            "sun shadows",
            "light shafts",
        ] {
            assert!(changes.contains(&name), "{name}: {changes:?}");
        }
        assert!(!changes.contains(&"texture filtering"), "{changes:?}");
    }

    #[test]
    fn the_scene_scale_is_named_by_the_side_it_moved() {
        let (_directory, mut console) = console();
        let before = Startup::read(&console, 16);
        console.set_cvar(crate::frame_target::scale::CVAR, "50");
        assert_eq!(
            before.changes(Startup::read(&console, 16)),
            ["render scale"]
        );
        // Supersampling wins: the render scale under it changes nothing.
        console.set_cvar("r_superSample", "2");
        let supersampled = Startup::read(&console, 16);
        assert_eq!(before.changes(supersampled), ["supersampling"]);
        // From 50 % to 200 %, both sides moved.
        let half = Startup {
            scale: 50,
            ..supersampled
        };
        assert_eq!(
            half.changes(supersampled),
            ["supersampling", "render scale"]
        );
        console.set_cvar(crate::frame_target::scale::CVAR, "75");
        assert!(supersampled.changes(Startup::read(&console, 16)).is_empty());
    }

    #[test]
    fn reading_latches_nothing() {
        let (_directory, mut console) = console();
        // The startup sample, as the context takes it.
        let running = material_maps::Settings::sample(Some(&console));
        console.set_cvar("r_normalMapping", "0");
        let _ = Startup::read(&console, 16);
        // The running value is still the one a later change compares with.
        assert!(running.normal);
        assert!(!material_maps::Settings::read(Some(&console)).normal);
    }

    #[test]
    fn only_a_change_or_force_reloads() {
        assert!(!wanted(false, &[]));
        assert!(wanted(true, &[]));
        assert!(wanted(false, &["HDR"]));
        assert!(wanted(true, &["HDR"]));
    }

    #[test]
    fn a_forced_reload_says_nothing_changed() {
        assert_eq!(reason(&[]), "forced, no setting changed");
        assert_eq!(reason(&["HDR", "FXAA"]), "HDR and FXAA");
    }

    #[test]
    fn the_summary_names_three_then_counts() {
        assert_eq!(summary(&["sun and sky"]), "Sun and sky");
        assert_eq!(summary(&["HDR", "FXAA"]), "HDR and FXAA");
        assert_eq!(
            summary(&["sun and sky", "light shafts", "sun shadows"]),
            "Sun and sky, light shafts and sun shadows"
        );
        assert_eq!(
            summary(&["HDR", "FXAA", "normal maps", "sun and sky", "sun shadows"]),
            "HDR, FXAA and 3 more"
        );
    }
}
