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
use glam::Vec3;
use std::time::{Duration, Instant};

/// How long a fly-to glides from the view to where it frames its target.
pub(crate) const GLIDE: Duration = Duration::from_millis(450);
/// The nearest and farthest a framed view stands from its target's middle (units).
const FRAME_NEAREST: f32 = 64.0;
const FRAME_FARTHEST: f32 = 6_000.0;
/// A framed target's radius fills this much of the half view: the distance is its
/// radius over `tan(FRAME_HALF_ANGLE)`, about a 70 degree view with a margin.
const FRAME_HALF_ANGLE: f32 = 0.52;

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
    /// A fly-to under way ([`GpuState::map_edit_fly_to`]).
    pub(crate) glide: Option<Glide>,
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
            glide: None,
        }
    }
}

/// A view: where the eye is and where it looks, as the camera keeps them (pitch up
/// positive and yaw, radians: `camera_pitch`, `camera_yaw`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewPose {
    pub(crate) eye: Vec3,
    pub(crate) pitch: f32,
    pub(crate) yaw: f32,
}

/// A fly-to: from one view to another over [`GLIDE`], eased in and out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Glide {
    from: ViewPose,
    to: ViewPose,
    started: Instant,
}

impl Glide {
    pub(crate) fn new(from: ViewPose, to: ViewPose, now: Instant) -> Self {
        Self {
            from,
            to,
            started: now,
        }
    }

    /// The view at `now`, and whether the glide is over.
    pub(crate) fn at(&self, now: Instant) -> (ViewPose, bool) {
        let t = (now.duration_since(self.started).as_secs_f32() / GLIDE.as_secs_f32()).min(1.0);
        (between(self.from, self.to, t), t >= 1.0)
    }
}

/// The view `t` (0 to 1) of the way from `from` to `to`, eased (smoothstep), the yaw the
/// short way round.
pub(crate) fn between(from: ViewPose, to: ViewPose, t: f32) -> ViewPose {
    let t = t.clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    let turn = crate::input::view_authority::wrap_yaw(to.yaw - from.yaw);
    ViewPose {
        eye: from.eye.lerp(to.eye, eased),
        pitch: from.pitch + (to.pitch - from.pitch) * eased,
        yaw: crate::input::view_authority::wrap_yaw(from.yaw + turn * eased),
    }
}

/// The camera's angles looking along `forward` (any length), the pitch within the
/// view's limit.
pub(crate) fn angles_of(forward: Vec3) -> (f32, f32) {
    let forward = forward.normalize_or(Vec3::X);
    let limit = crate::input::view_authority::PITCH_LIMIT;
    (
        forward.z.clamp(-1.0, 1.0).asin().clamp(-limit, limit),
        forward.y.atan2(forward.x),
    )
}

/// The view that frames a target of `radius` about `centre`, as Blender's View Selected
/// does: looking along `forward` (the view's own direction, kept), stepped back until
/// the target fills the view with a margin, between [`FRAME_NEAREST`] and
/// [`FRAME_FARTHEST`] from its middle.
pub(crate) fn frame(centre: Vec3, radius: f32, forward: Vec3) -> ViewPose {
    let forward = forward.normalize_or(Vec3::X);
    let distance = (radius.max(0.0) / FRAME_HALF_ANGLE.tan()).clamp(FRAME_NEAREST, FRAME_FARTHEST);
    let (pitch, yaw) = angles_of(forward);
    // The kept direction, as the clamped angles give it.
    let along = Vec3::new(
        pitch.cos() * yaw.cos(),
        pitch.cos() * yaw.sin(),
        pitch.sin(),
    );
    ViewPose {
        eye: centre - along * distance,
        pitch,
        yaw,
    }
}

/// The middle and the radius of the box of `points` (none: `None`).
pub(crate) fn sphere_of(points: impl IntoIterator<Item = Vec3>) -> Option<(Vec3, f32)> {
    let mut points = points.into_iter();
    let first = points.next()?;
    let (low, high) = points.fold((first, first), |(low, high), point| {
        (low.min(point), high.max(point))
    });
    Some(((low + high) * 0.5, (high - low).length() * 0.5))
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

    /// The editor's view now.
    pub(crate) fn map_edit_view_pose(&self) -> ViewPose {
        ViewPose {
            eye: self.camera_position,
            pitch: self.camera_pitch,
            yaw: self.camera_yaw,
        }
    }

    /// Whether the editor's camera can be put somewhere: the free camera's flight in a
    /// live game, or the inspection camera of a map seen without a server; not a walk
    /// (exploring) nor the player's own view, which move by the game's rules.
    fn map_edit_camera_movable(&self) -> Result<(), &'static str> {
        if self.live_session.is_some() {
            return (self.free_camera_active() && self.local_prediction.fake_noclip())
                .then_some(())
                .ok_or("Fly-to moves the free camera only, which is off");
        }
        if self.resident.exploring() {
            return Err("Fly-to moves the free camera only: not while walking a map");
        }
        if self.demo_session.is_some() || self.is_menu_world {
            return Err("Fly-to moves the editor's camera only: not in a demo or the menu");
        }
        Ok(())
    }

    /// Glide the editor's camera to frame a target of `radius` about `centre` ([`frame`]),
    /// over [`GLIDE`]: the client's own view only (the body stays at its server position).
    pub(crate) fn map_edit_fly_to(&mut self, centre: Vec3, radius: f32) -> Result<String, String> {
        if !centre.is_finite() || !radius.is_finite() {
            return Err("Nowhere to fly to".into());
        }
        if let Err(why) = self.map_edit_camera_movable() {
            return Err(self
                .map_edits
                .fly
                .refused
                .clone()
                .map_or_else(|| why.to_owned(), |refused| format!("{refused}: no fly-to")));
        }
        let from = self.map_edit_view_pose();
        let forward = Vec3::new(
            from.pitch.cos() * from.yaw.cos(),
            from.pitch.cos() * from.yaw.sin(),
            from.pitch.sin(),
        );
        let to = frame(centre, radius, forward);
        self.map_edits.fly.glide = Some(Glide::new(from, to, Instant::now()));
        Ok(format!(
            "Flying to {:.0} {:.0} {:.0} ({:.0} units away)",
            centre.x,
            centre.y,
            centre.z,
            to.eye.distance(centre)
        ))
    }

    /// Once a frame while a fly-to glides: the view along it; the flight (the right
    /// button) or the editor leaving ends it where it is.
    fn map_edit_glide_frame(&mut self) {
        let Some(glide) = self.map_edits.fly.glide else {
            return;
        };
        if self.map_edits.fly.held || !self.map_edits.editor.active {
            self.map_edits.fly.glide = None;
            return;
        }
        let (pose, done) = glide.at(Instant::now());
        if self.live_session.is_some() && !self.local_prediction.place_flight(pose.eye) {
            self.map_edits.fly.glide = None;
            return;
        }
        self.camera_position = pose.eye;
        self.camera_pitch = pose.pitch;
        self.camera_yaw = pose.yaw;
        if done {
            self.map_edits.fly.glide = None;
        }
    }

    /// The world shots: one step of a fly-to's glide (their frames run no editor frame).
    #[cfg(test)]
    pub(crate) fn shot_glide(&mut self) {
        self.map_edit_glide_frame();
    }

    /// Once a frame: the flight's speed to the predictor while the editor is on (Shift
    /// read as it is held), and a fly-to's glide.
    pub(super) fn map_edit_fly_frame(&mut self) {
        self.map_edit_glide_frame();
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

    /// Framing keeps the view's direction and steps back as far as the target's size
    /// needs, within the nearest and farthest; the target lies straight ahead.
    #[test]
    fn framing_keeps_the_direction_and_steps_back_by_the_size() {
        let centre = Vec3::new(100.0, 200.0, 50.0);
        let forward = Vec3::new(1.0, 1.0, -0.5);
        let small = frame(centre, 8.0, forward);
        assert!((small.eye.distance(centre) - FRAME_NEAREST).abs() < 1e-3);
        let big = frame(centre, 512.0, forward);
        let distance = big.eye.distance(centre);
        assert!(
            (distance - 512.0 / FRAME_HALF_ANGLE.tan()).abs() < 0.1,
            "{distance}"
        );
        let ahead = (centre - big.eye).normalize();
        assert!(ahead.dot(forward.normalize()) > 0.9999);
        // looking along the kept direction
        let (pitch, yaw) = angles_of(forward);
        assert_eq!((big.pitch, big.yaw), (pitch, yaw));
        assert!(pitch < 0.0, "looking down");
        assert!((yaw - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        assert!(frame(centre, 1e9, forward).eye.distance(centre) <= FRAME_FARTHEST + 0.1);
        // straight down is held to the view's pitch limit
        let down = frame(centre, 64.0, Vec3::NEG_Z);
        assert!(down.pitch >= -crate::input::view_authority::PITCH_LIMIT - 1e-6);
    }

    /// A glide eases from one view to the other, turns the short way, and ends on time.
    #[test]
    fn a_glide_eases_and_turns_the_short_way() {
        let from = ViewPose {
            eye: Vec3::ZERO,
            pitch: 0.0,
            yaw: 3.0,
        };
        let to = ViewPose {
            eye: Vec3::new(100.0, 0.0, 0.0),
            pitch: 0.5,
            yaw: -3.0,
        };
        assert_eq!(between(from, to, 0.0), from);
        let end = between(from, to, 1.0);
        assert!(end.eye.distance(to.eye) < 1e-3 && (end.pitch - 0.5).abs() < 1e-6);
        // from 3 to -3 radians the short way is through pi, not through 0
        let middle = between(from, to, 0.5);
        assert!(middle.yaw.abs() > 3.0, "{}", middle.yaw);
        assert!((middle.eye.x - 50.0).abs() < 1e-3);
        // eased: slow at the start
        assert!(between(from, to, 0.1).eye.x < 10.0);
        let now = Instant::now();
        let glide = Glide::new(from, to, now);
        assert!(!glide.at(now).1);
        let (pose, done) = glide.at(now + GLIDE);
        assert!(done && pose.eye.distance(to.eye) < 1e-3);
    }

    #[test]
    fn a_target_is_the_box_of_its_points() {
        assert!(sphere_of(std::iter::empty()).is_none());
        let (centre, radius) = sphere_of([
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 30.0),
        ])
        .unwrap();
        assert_eq!(centre, Vec3::new(0.0, 0.0, 15.0));
        assert!((radius - (20.0f32 * 20.0 + 30.0 * 30.0).sqrt() * 0.5).abs() < 1e-4);
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
