//! Toys that stay on until the player turns them off (Sol's request, 22/10/2026): Order 66,
//! the Hax, the Drone, the Monitor, Box hunt, the Banana peel, the Prop gun and Quests keep
//! their switch in an archived cvar ([`ORDER66_CVAR`], [`HAX_CVAR`], [`DRONE_CVAR`],
//! [`MONITOR_CVAR`], [`BOX_HUNT_CVAR`], [`BANANA_CVAR`], [`PROPGUN_CVAR`],
//! [`QUESTS_CVAR`]), so
//! a toy turned on is still on after a map change, another server or a restart of the
//! game. The toy's own state stays the viewer's; [`Saved::sync`] keeps the two in step
//! once a frame, the cvar winning when it was set from the console or a config.

use crate::console::ViewerConsole;

/// Order 66's switch, saved.
pub(crate) const ORDER66_CVAR: &str = "cl_toyOrder66";
/// The Hax's switch, saved.
pub(crate) const HAX_CVAR: &str = "cl_toyHax";
/// The Drone's switch (the consent to be peeked), saved.
pub(crate) const DRONE_CVAR: &str = "cl_toyDrone";
/// The Monitor's switch (throw them, and be hit), saved.
pub(crate) const MONITOR_CVAR: &str = "cl_toyMonitor";
/// Box hunt's switch (hunt, and maybe be the mark), saved.
pub(crate) const BOX_HUNT_CVAR: &str = "cl_toyBoxHunt";
/// The Banana peel's switch (drop peels, and slip on them), saved.
pub(crate) const BANANA_CVAR: &str = "cl_toyBanana";
/// The Prop gun's switch (hold and fire it, and be hit), saved.
pub(crate) const PROPGUN_CVAR: &str = "cl_toyPropGun";
/// Quests' switch (be given quests, and named in them), saved.
pub(crate) const QUESTS_CVAR: &str = "cl_toyQuests";

/// The value a toy's cvar had when it was last in step with the toy.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Saved {
    last: Option<bool>,
}

impl Saved {
    /// The toy's state after this frame, `on` being what the toy says: at the first call
    /// the saved value; after, the cvar's when it changed since the last call, else
    /// `on`, written to the cvar when it differs.
    pub(crate) fn sync(&mut self, console: &mut ViewerConsole, cvar: &str, on: bool) -> bool {
        let saved = console.bool_cvar(cvar).unwrap_or(false);
        let state = match self.last {
            Some(last) if last == saved => on,
            _ => saved,
        };
        if state != saved {
            console.set_cvar(cvar, if state { "1" } else { "0" });
        }
        self.last = Some(state);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_saved_switch_comes_back_and_follows_the_toy_and_the_console() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut saved = Saved::default();
        // Off by default; the toy turned on is written.
        assert!(!saved.sync(&mut console, HAX_CVAR, false));
        assert!(saved.sync(&mut console, HAX_CVAR, true));
        assert_eq!(console.bool_cvar(HAX_CVAR), Some(true));
        // A new run starts from the saved value, whatever the toy says.
        let mut next_run = Saved::default();
        assert!(next_run.sync(&mut console, HAX_CVAR, false));
        // Set from the console, the cvar wins; then the toy's toggle wins again.
        console.set_cvar(HAX_CVAR, "0");
        assert!(!next_run.sync(&mut console, HAX_CVAR, true));
        assert!(next_run.sync(&mut console, HAX_CVAR, true));
        assert_eq!(console.bool_cvar(HAX_CVAR), Some(true));
    }
}
