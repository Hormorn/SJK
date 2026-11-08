//! Input motion: OpenJK cl_input.cpp:480-510,822-917,1038-1182.
use super::{GameButton, GameplayInput};
use crate::{GpuState, pointer_input::MouseLook};
use sjk_client::pmove::MovementState;

pub(crate) struct Motion {
    pub now: u64,
    /// How long before `now` the command being built was made: a frame slower than
    /// 8 ms makes several commands, each at its own time ([`GameplayInput::set_command_age`]).
    pub command_age: u64,
    previous_time: u64,
    command_millis: u64,
    pub freelook: bool,
    pub raw: [f32; 2],
    previous_mouse: [f32; 2],
    pub forward: f32,
    pub side: f32,
    pub yaw_speed: f32,
    pub pitch_speed: f32,
    pub angle_speed_key: f32,
    pub filter: bool,
    pub accel: f32,
    pub accel_style: i64,
    pub accel_offset: f32,
    pub m_forward: f32,
    pub m_side: f32,
    /// Cgame FOV multiplier applied after acceleration (`cl_input.cpp:1155-1156`).
    pub zoom_sensitivity: f32,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            now: 0,
            command_age: 0,
            previous_time: 0,
            command_millis: 0,
            freelook: true,
            raw: [0.0; 2],
            previous_mouse: [0.0; 2],
            forward: 0.0,
            side: 0.0,
            yaw_speed: 140.0,
            pitch_speed: 140.0,
            angle_speed_key: 1.5,
            filter: false,
            accel: 0.0,
            accel_style: 0,
            accel_offset: 5.0,
            m_forward: 0.25,
            m_side: 0.25,
            zoom_sensitivity: 1.0,
        }
    }
}

impl Motion {
    pub fn clear(&mut self) {
        self.command_millis = 0;
        self.raw = [0.0; 2];
        self.previous_mouse = [0.0; 2];
        self.forward = 0.0;
        self.side = 0.0;
    }

    /// Filtering precedes acceleration; style 1 raises each axis independently.
    pub fn mouse(&mut self, settings: MouseLook, millis: u64) -> [f32; 2] {
        let raw = std::mem::take(&mut self.raw);
        let mut value = raw;
        if self.filter {
            for i in 0..2 {
                value[i] = (raw[i] + self.previous_mouse[i]) * 0.5;
            }
        }
        self.previous_mouse = raw;
        if value == [0.0; 2] {
            return value;
        }
        let millis = millis.max(1) as f32;
        if self.accel == 0.0 {
            for axis in &mut value {
                *axis *= settings.sensitivity;
            }
        } else if self.accel_style == 0 {
            // SQRTFAST uses one Q_rsqrt iteration, not the platform square root.
            // shared/qcommon/q_math.h:91, q_math.c:360-378 (used by codemp).
            let squared = value[0] * value[0] + value[1] * value[1];
            let reciprocal = f32::from_bits(0x5f3759df - (squared.to_bits() >> 1));
            let reciprocal = reciprocal * (1.5 - squared * 0.5 * reciprocal * reciprocal);
            let rate = squared * reciprocal / millis;
            for axis in &mut value {
                *axis *= settings.sensitivity + rate * self.accel;
            }
        } else {
            for axis in &mut value {
                let power = (axis.abs() / millis / self.accel_offset).powf(self.accel);
                *axis = settings.sensitivity
                    * (*axis + if *axis < 0.0 { -power } else { power } * self.accel_offset);
            }
        }
        value.map(|axis| axis * self.zoom_sensitivity)
    }
}

impl GameButton {
    pub(super) fn slot(self) -> usize {
        match self {
            Self::Button(n) => usize::from(n),
            Self::Forward => 16,
            Self::Back => 17,
            Self::MoveLeft => 18,
            Self::MoveRight => 19,
            Self::Up => 20,
            Self::Down => 21,
            Self::Speed => 22,
            Self::Scores => 23,
            Self::Left => 24,
            Self::Right => 25,
            Self::Lookup => 26,
            Self::Lookdown => 27,
            Self::Strafe => 28,
            Self::Mlook => 29,
            Self::ForceStasis => 30,
            Self::Duck => 31,
        }
    }
}

impl GameplayInput {
    /// Consume frame input while a wall move owns the local view. Movement and
    /// button accounting still run; look input must not build up until release.
    pub(crate) fn sample_view_motion(
        &mut self,
        now: u64,
        settings: MouseLook,
        view: Option<&MovementState>,
    ) -> [f32; 2] {
        let delta = self.sample_motion(now, settings);
        if view.is_some_and(|view| view.wall_view_locked) {
            // Filtering must not carry the final held sample into the release frame.
            self.motion.previous_mouse = [0.0; 2];
            [0.0; 2]
        } else {
            delta
        }
    }

    /// The share of the command `button` moves for; a press let go in no time (a
    /// wheel notch, [`super::state::KeyState::tapped`]) moves all of it.
    pub(super) fn movement_fraction(&self, button: GameButton) -> f32 {
        if self.held[button.slot()].tapped() {
            return 1.0;
        }
        if self.motion.command_millis == 0 {
            return self.fraction(button);
        }
        (self.held[button.slot()].command_elapsed as f32 / self.motion.command_millis as f32)
            .clamp(0.0, 1.0)
    }
    pub(super) fn fraction(&self, button: GameButton) -> f32 {
        self.held[button.slot()].fraction
    }

    /// A pair's fractions after `cl_idrive` ([`super::idrive`]).
    pub(super) fn idrive_pair(
        &self,
        positive: GameButton,
        negative: GameButton,
        positive_fraction: f32,
        negative_fraction: f32,
    ) -> (f32, f32) {
        let key = |button: GameButton, fraction| {
            let state = &self.held[button.slot()];
            super::idrive::Key {
                fraction,
                pressed_at: state.pressed_at,
                released_at: state.released_at,
                active: state.active,
            }
        };
        self.idrive.resolve(
            positive == GameButton::Up,
            key(positive, positive_fraction),
            key(negative, negative_fraction),
            self.motion.now.saturating_sub(self.motion.command_age),
        )
    }

    pub(super) fn axis(
        &self,
        positive: GameButton,
        negative: GameButton,
        speed: i8,
        mouse: f32,
    ) -> i8 {
        let (positive, negative) = self.idrive_pair(
            positive,
            negative,
            self.movement_fraction(positive),
            self.movement_fraction(negative),
        );
        // Stock truncates each contribution as it adds to the integer movement axis.
        let positive = (speed as f32 * positive) as i32;
        let negative = (positive as f32 - speed as f32 * negative) as i32;
        (negative as f32 + mouse).clamp(-127.0, 127.0) as i8
    }

    /// Reset button impulses only after a real command, not a prediction preview.
    pub(crate) fn finish_command(&mut self) {
        for state in &mut self.held {
            state.pressed = false;
            state.command_elapsed = 0;
        }
        self.motion.command_millis = 0;
        self.motion.side = 0.0;
        self.motion.forward = 0.0;
    }

    /// Consume one frame's key durations and mouse counts, without heap allocation.
    pub(crate) fn sample_motion(&mut self, now: u64, settings: MouseLook) -> [f32; 2] {
        let millis = now.saturating_sub(self.motion.previous_time).clamp(1, 200);
        self.motion.now = now;
        self.motion.previous_time = now;
        self.motion.command_millis += millis;
        for state in &mut self.held {
            state.sample(now, millis);
        }
        let speed = millis as f32
            * 0.001
            * if self.held(GameButton::Speed) {
                self.motion.angle_speed_key
            } else {
                1.0
            };
        let strafe = self.held(GameButton::Strafe);
        let yaw = if strafe {
            0.0
        } else {
            speed
                * self.motion.yaw_speed
                * (self.fraction(GameButton::Left) - self.fraction(GameButton::Right))
        };
        let pitch = speed
            * self.motion.pitch_speed
            * (self.fraction(GameButton::Lookup) - self.fraction(GameButton::Lookdown));
        let mouse = self.motion.mouse(settings, millis);
        self.motion.side += if strafe {
            mouse[0] * self.motion.m_side
        } else {
            0.0
        };
        let looking = (self.motion.freelook || self.held(GameButton::Mlook)) && !strafe;
        self.motion.forward += if looking {
            0.0
        } else {
            -mouse[1] * self.motion.m_forward
        };
        let yaw = yaw
            - if strafe {
                0.0
            } else {
                settings.yaw_scale * mouse[0]
            };
        let pitch = pitch
            - if looking {
                settings.pitch_scale * mouse[1] * if settings.invert { -1.0 } else { 1.0 }
            } else {
                0.0
            };
        [yaw.to_radians(), pitch.to_radians()]
    }
}

impl GpuState {
    pub(crate) fn update_input_motion(&mut self) {
        if self.resident.map_change_pending {
            self.gameplay_input.clear();
            return;
        }
        let Some(console) = &self.console else { return };
        let now = console.input_millis();
        self.gameplay_input.motion.zoom_sensitivity = self.scope.sensitivity;
        let view = if self.local_prediction.fake_noclip() {
            None
        } else {
            self.local_prediction.predicted_state()
        };
        let delta = self
            .gameplay_input
            .sample_view_motion(now, self.mouse_look, view);
        // A view the server holds is not turned (`held_view`): the motion is spent.
        let delta = if self.mouse_look_held() {
            [0.0; 2]
        } else {
            delta
        };
        // The WHEEL OF DOOM's inverted, mirrored or drunk mouse (`wheel_of_doom`).
        let delta = self
            .wheel_of_doom
            .effects
            .shape_look(delta, std::time::Instant::now());
        self.camera_yaw = crate::input::view_authority::wrap_yaw(self.camera_yaw + delta[0]);
        self.camera_pitch = (self.camera_pitch + delta[1]).clamp(
            -crate::input::view_authority::PITCH_LIMIT,
            crate::input::view_authority::PITCH_LIMIT,
        );
        if self.live_session.is_none() {
            self.gameplay_input.finish_command();
        }
    }
}

#[cfg(test)]
mod wall_view_tests {
    use super::*;
    use sjk_client::pmove::{MovementConfig, PMF_STUCK_TO_WALL, Predictor};
    use sjk_game_jka::pmove_debug_melee::DebugMelee;
    use sjk_protocol::InfoString;

    #[test]
    fn wall_grab_look_follows_the_profile_and_player_option() {
        for step in [8, 7, 4, 3] {
            for server in [
                r"\gamename\basejka\g_debugMelee\2",
                r"\gamename\JA+ Mod v2.4 Build 7\g_debugMelee\2",
                r"\gamename\JA+ Mod v2.4 Build 7\sv_hostname\JoF\g_debugMelee\2",
            ] {
                for force_lock in [false, true] {
                    let info = InfoString::parse(server).unwrap();
                    let rules = DebugMelee::from_server_info(&info);
                    let config = MovementConfig {
                        debug_melee: rules,
                        ..Default::default()
                    };
                    let held = MovementState {
                        health: 100,
                        movement_flags: PMF_STUCK_TO_WALL,
                        view_angles: [12.0, 90.0, 0.0],
                        ..Default::default()
                    };
                    let mut snapshot = sjk_protocol::PlayerState::default();
                    held.write_player_state(&mut snapshot);
                    let predictor = Predictor::from_player_state(&snapshot, config);
                    let mut view = predictor.state().clone();
                    // Presentation may force the profile's hold on, never off.
                    view.wall_view_locked |= force_lock;
                    let locked = !rules.free_wall_look || force_lock;
                    assert_eq!(view.wall_view_locked, locked);
                    let mut input = GameplayInput::default();
                    input.motion.filter = true;
                    for button in [
                        GameButton::Forward,
                        GameButton::Up,
                        GameButton::Left,
                        GameButton::Lookup,
                    ] {
                        input.held[button.slot()].event(true, None, 0);
                    }
                    let (mut pitch, mut yaw) = (-12.0_f32.to_radians(), 90.0_f32.to_radians());
                    let baseline = input.user_command(0, pitch, yaw, [0; 3], 3, 0, 0).angles;
                    for frame in 1..=3 {
                        input.motion.raw = [80.0, -40.0];
                        let delta = input.sample_view_motion(
                            frame * step,
                            MouseLook::default(),
                            Some(&view),
                        );
                        yaw += delta[0];
                        pitch += delta[1];
                        let command =
                            input.user_command((frame * step) as i32, pitch, yaw, [0; 3], 3, 0, 0);
                        if locked {
                            assert_eq!(
                                command.angles, baseline,
                                "{server}, {force_lock}, {step} ms"
                            );
                        } else {
                            assert_ne!(command.angles[0], baseline[0]);
                            assert_ne!(command.angles[1], baseline[1]);
                        }
                        assert_eq!((command.forward_move, command.up_move), (127, 127));
                        input.finish_command();
                    }
                    let released = MovementState {
                        movement_flags: 0,
                        wall_view_locked: false,
                        ..held
                    };
                    for button in [GameButton::Left, GameButton::Lookup] {
                        input.held[button.slot()].event(false, None, 3 * step);
                    }
                    let delta =
                        input.sample_view_motion(4 * step, MouseLook::default(), Some(&released));
                    if locked {
                        // Held mouse samples cannot leak out through the filter.
                        assert_eq!(delta, [0.0; 2]);
                    }
                    input.motion.raw = [10.0, -5.0];
                    let delta =
                        input.sample_view_motion(5 * step, MouseLook::default(), Some(&released));
                    assert_ne!(delta[0], 0.0);
                    assert_ne!(delta[1], 0.0);
                }
            }
        }
    }
}
