//! The map editor's camera (Sol, 04/11/2026: "fly freely while editing"): while the
//! editor is on in a live game the view is the free camera ([`crate::free_camera`],
//! `cg_freeCamera`: the body stays at its server position), turned on with the editor
//! and off with it. As in Blender's and Unreal's editors the pointer stays free for the
//! panel and the scene; holding the right mouse button on the scene looks around with
//! the mouse and flies with the movement keys (W, A, S, D; Space and C up and down, as
//! bound), Shift faster, the wheel setting the speed. Releasing it puts the pointer back
//! where it was.
//!
//! While the button is held the in-game menu steps aside (`game_menu` off, its page kept)
//! so the movement keys and the mouse reach the game as in play, with the talk balloon
//! down (it would stop the flight); the panel is still drawn. Only the keys bound to a
//! movement command (and the world-note key, which selects at the crosshair) act; the
//! others wait, so no chat, weapon or scoreboard key fires mid-flight. Where the free
//! camera is refused (dead, in a duel, riding), the right button looks and moves with
//! the player's own view, and the panel says why in one line. Exploring a map walks
//! with its own movement.

use crate::GpuState;
use crate::ingame_menu::Page;

/// The wheel's speed factor steps by this, between [`SLOWEST`] and [`FASTEST`].
const SPEED_STEP: f32 = 1.25;
const SLOWEST: f32 = 0.25;
const FASTEST: f32 = 8.0;
/// Shift held flies this many times faster.
const SHIFT_BOOST: f32 = 4.0;

/// The bound commands that act while flying: the movement and the world-note key.
const FLY_COMMANDS: &[&str] = &[
    "+forward",
    "+back",
    "+moveleft",
    "+moveright",
    "+moveup",
    "+movedown",
    "+speed",
    "+left",
    "+right",
    "+lookup",
    "+lookdown",
    "+strafe",
    "worldnote",
];

/// The editor's camera state.
pub(crate) struct Fly {
    /// The right mouse button is held on the scene: flying.
    pub(crate) held: bool,
    /// Where the pointer was when the button went down (physical pixels).
    from: [f32; 2],
    /// The wheel's speed factor.
    pub(crate) speed: f32,
    /// The editor turned the free camera on, so turns it off.
    started: bool,
    /// Why there is no free camera while the editor is on, when there is none.
    pub(crate) refused: Option<String>,
    /// The boost last handed to the predictor.
    boost: Option<f32>,
}

impl Default for Fly {
    fn default() -> Self {
        Self {
            held: false,
            from: [0.0; 2],
            speed: 1.0,
            started: false,
            refused: None,
            boost: None,
        }
    }
}

/// The speed factor after a notch of the wheel (`up`: faster).
pub(crate) fn wheel_speed(speed: f32, up: bool) -> f32 {
    let speed = if up {
        speed * SPEED_STEP
    } else {
        speed / SPEED_STEP
    };
    speed.clamp(SLOWEST, FASTEST)
}

/// The flight's boost over the body's speed: the wheel's factor, times [`SHIFT_BOOST`]
/// with Shift held.
pub(crate) fn boost(speed: f32, shift: bool) -> f32 {
    if shift { speed * SHIFT_BOOST } else { speed }
}

/// Whether a key bound to `script` acts while flying.
pub(crate) fn flies(script: &str) -> bool {
    let script = script.trim();
    FLY_COMMANDS
        .iter()
        .any(|command| command.eq_ignore_ascii_case(script))
}

/// The short line the panel shows when the free camera was refused (`error`, the
/// `freecam` command's).
fn refusal(error: &str) -> String {
    if error.contains("alive") {
        // `fake_noclip_allowed`: alive, on foot, out of a duel, not following anyone.
        return "Own view: free camera needs you alive, on foot, no duel".to_owned();
    }
    format!(
        "Own view: {}",
        error.strip_prefix("freecam: ").unwrap_or(error)
    )
}

impl GpuState {
    /// Whether the editor flies now (the right mouse button held on the scene).
    pub(crate) fn map_edit_flying(&self) -> bool {
        self.map_edits.fly.held
    }

    /// The editor turned on in a world: the view goes to the free camera when allowed.
    pub(super) fn map_edit_camera_on(&mut self) {
        let fly = &mut self.map_edits.fly;
        fly.refused = None;
        if self.live_session.is_none() || self.free_camera_active() {
            // Exploring walks with its own movement; a free camera already on stays.
            return;
        }
        match self.free_camera_command(&["on".to_owned()]) {
            Ok(_) => self.map_edits.fly.started = true,
            Err(error) => self.map_edits.fly.refused = Some(refusal(&error)),
        }
    }

    /// The editor turned off: the flight ends and the camera returns to the player, when
    /// the editor turned it away.
    pub(super) fn map_edit_camera_off(&mut self) {
        self.map_edit_fly_stop();
        let fly = &mut self.map_edits.fly;
        let started = std::mem::take(&mut fly.started);
        fly.refused = None;
        if started && self.free_camera_active() {
            self.end_free_camera();
        }
        self.map_edit_fly_frame();
    }

    /// The right mouse button went down on the scene at `at`: fly until it comes up.
    pub(super) fn map_edit_fly_start(&mut self, at: [f32; 2]) {
        if !self.map_edits.editor.active || self.map_edits.fly.held {
            return;
        }
        // Refused at the start (dead, say), or ended since (a reconnection): try again.
        if self.live_session.is_some() && !self.free_camera_active() {
            self.map_edit_camera_on();
            self.map_edits.panel.stale();
        }
        let fly = &mut self.map_edits.fly;
        fly.held = true;
        fly.from = at;
        // The menu steps aside, its page kept: the keys and the mouse reach the game.
        self.game_menu = false;
        self.gameplay_input.release_keys();
        self.capture_pointer();
    }

    /// The right mouse button came up (or the window lost the focus): the panel takes
    /// the pointer again, put back where the flight began.
    pub(crate) fn map_edit_fly_stop(&mut self) {
        if !std::mem::take(&mut self.map_edits.fly.held) {
            return;
        }
        if self.map_edits.editor.active
            && (self.live_session.is_some() || self.resident.exploring())
        {
            self.game_menu = true;
            self.game_menu_page = Page::MapEdit;
        }
        self.release_pointer();
        let [x, y] = self.map_edits.fly.from;
        if let Some(window) = &self.window {
            let _ = window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                f64::from(x),
                f64::from(y),
            ));
        }
        self.cursor_position = [x, y];
    }

    /// The wheel while flying: faster (`up`) or slower.
    pub(crate) fn map_edit_fly_wheel(&mut self, up: bool) {
        let fly = &mut self.map_edits.fly;
        fly.speed = wheel_speed(fly.speed, up);
        let line = format!("Flight speed x{:.2}", fly.speed);
        self.map_edits.panel.answer(Ok(vec![line]));
    }

    /// A key while flying: whether the editor keeps it from the game. The movement keys
    /// and the world-note key go to the game; the others wait for the button's release.
    pub(super) fn map_edit_fly_holds_key(&self, event: &winit::event::KeyEvent) -> bool {
        let Some(name) = crate::input::keys::key_name(event) else {
            return true;
        };
        !self
            .console
            .as_ref()
            .and_then(|console| console.key_script(name.as_str()))
            .is_some_and(flies)
    }

    /// Once a frame: the flight's speed to the predictor while the editor is on (Shift
    /// read as it is held).
    pub(super) fn map_edit_fly_frame(&mut self) {
        let shift = self
            .console
            .as_ref()
            .is_some_and(|console| console.shift_held());
        let wanted = (self.map_edits.editor.active && self.free_camera_active())
            .then(|| boost(self.map_edits.fly.speed, shift));
        if self.map_edits.fly.boost != wanted {
            self.map_edits.fly.boost = wanted;
            self.local_prediction.set_fake_noclip_boost(wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_steps_the_speed_within_its_bounds_and_shift_boosts_it() {
        let mut speed = 1.0;
        speed = wheel_speed(speed, true);
        assert!((speed - 1.25).abs() < 1e-6);
        speed = wheel_speed(speed, false);
        assert!((speed - 1.0).abs() < 1e-6);
        for _ in 0..40 {
            speed = wheel_speed(speed, true);
        }
        assert_eq!(speed, FASTEST);
        for _ in 0..40 {
            speed = wheel_speed(speed, false);
        }
        assert_eq!(speed, SLOWEST);
        assert_eq!(boost(2.0, false), 2.0);
        assert_eq!(boost(2.0, true), 8.0);
    }

    #[test]
    fn only_movement_and_the_world_note_key_act_while_flying() {
        for script in ["+forward", " +MoveUp ", "+movedown", "+speed", "worldnote"] {
            assert!(flies(script), "{script}");
        }
        for script in [
            "+attack",
            "messagemode",
            "weapon 1",
            "+scores",
            "mapedit",
            "",
        ] {
            assert!(!flies(script), "{script}");
        }
    }

    #[test]
    fn a_refusal_reads_as_one_short_line() {
        let line = refusal("freecam: only available while alive, on foot and not in a duel");
        assert_eq!(
            line,
            "Own view: free camera needs you alive, on foot, no duel"
        );
        assert!(line.len() <= 56);
        assert_eq!(refusal("freecam: not in a game"), "Own view: not in a game");
    }
}
