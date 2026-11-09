//! JKA strafe guides (JoF EJK's lines, CGAZ markers, SJK's strafe aura) and snap zones;
//! no simulation or wire changes.
use super::*;
use sjk_ui::{Color, DrawCommand, Rect};

#[path = "movement_math.rs"]
pub(crate) mod math;
#[path = "movement_settings.rs"]
mod settings;
pub(crate) use settings::{
    AURA_CVAR, AURA_GROUND_CVAR, LINES_ROW, lines_on, register, toggle_lines,
};

/// Degrees of turn the strafe aura shows each side of the view.
const AURA_SPAN: f32 = 60.0;
/// The strafe aura's glow thickness, as a share of its radius.
const AURA_WIDTH: f32 = 0.45;
/// Draw commands the strafe aura pushes in a frame: the aura and the view tick.
pub(crate) const AURA_COMMANDS: usize = 2;
/// The wish direction of each movement direction (numbered as in [`math::direction`]),
/// in degrees left of the view: W, WA, A, SA, S, SD, D, WD.
const WISH: [f32; 8] = [0.0, 45.0, 90.0, 135.0, 180.0, -135.0, -90.0, -45.0];

/// The movement physics the guides are worked out with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StrafePhysics {
    /// Ground acceleration.
    pub(crate) ground_acceleration: f32,
    /// Ground friction.
    pub(crate) friction: f32,
    /// Air acceleration.
    pub(crate) air_acceleration: f32,
}

impl StrafePhysics {
    /// Stock JKA's (`bg_pmove.c`).
    pub(crate) const STOCK: Self = Self {
        ground_acceleration: 10.0,
        friction: 6.0,
        air_acceleration: 1.0,
    };
}

/// Retained guide input and settings, populated from the existing prediction/snapshot projection.
pub(super) struct Guides {
    settings: settings::Settings,
    physics: StrafePhysics,
    velocity: [f32; 3],
    angles: [f32; 3],
    speed: f32,
    direction: usize,
    allowed: bool,
    airborne: bool,
    zones: [f32; 128],
    zone_key: [f32; 2],
    zone_count: usize,
    reported_helper: Option<i64>,
    projection: Option<([f32; 3], [f32; 3], f32)>,
    full_input: bool,
}

/// Whether the guides and the strafe aura follow a player in this `pm_type`:
/// `PM_NORMAL` (0) and `PM_JETPACK` (1, `bg_public.h`), which the server sets for
/// the whole of a jetpack flight (`g_active.c` `ClientThink_real`) while the air
/// movement stays `PM_AirMove`'s, its push scaled 0.8 unless thrusting
/// (`bg_pmove.c`); the guides were blank there (hub #102, Rollembones).
fn guided_movement_type(movement_type: u8) -> bool {
    const PM_NORMAL: u8 = 0;
    const PM_JETPACK: u8 = 1;
    matches!(movement_type, PM_NORMAL | PM_JETPACK)
}

impl Default for Guides {
    fn default() -> Self {
        Self {
            settings: settings::Settings::read(None),
            physics: StrafePhysics::STOCK,
            velocity: [0.0; 3],
            angles: [0.0; 3],
            speed: 250.0,
            direction: 0,
            allowed: false,
            airborne: false,
            zones: [0.0; 128],
            zone_key: [0.0; 2],
            zone_count: 0,
            reported_helper: None,
            projection: None,
            full_input: true,
        }
    }
}

impl Guides {
    /// Refresh borrowed player data. Unsupported physics are suppressed, not approximated.
    pub(super) fn update(
        &mut self,
        player: &PlayerState,
        predicted: Option<&MovementState>,
        console: Option<&ViewerConsole>,
    ) {
        self.settings = settings::Settings::read(console);
        if self.reported_helper != Some(self.settings.helper) {
            self.reported_helper = Some(self.settings.helper);
            if self.settings.helper != 0 {
                eprintln!(
                    "strafe helper: JKA lines (option 1) and airborne CGAZ markers (option 2); \
                    original, Warsow and Weze styles, sounds, accel bar and centre line unavailable"
                );
            }
        }
        self.velocity = predicted.map_or(player.velocity(), |p| p.velocity);
        self.angles = predicted.map_or(player.view_angles(), |p| p.view_angles);
        self.speed = predicted.map_or(player.speed(), |p| p.speed);
        self.direction = predicted
            .map_or(player.movement_direction(), |p| p.movement_direction)
            .clamp(0, 7) as usize;
        if predicted.is_none() && self.velocity[0].hypot(self.velocity[1]) < 9.0 {
            self.direction = 8;
        }
        self.airborne =
            predicted.map_or(player.ground_entity_num(), |p| p.ground_entity_number) == 1023;
        self.allowed = guided_movement_type(player.movement_type())
            && player.vehicle_entity_num() == 0
            && player.zoom_mode() == 0;
    }

    /// Work the guides out with `physics`, the server's; disable them without any.
    pub(super) fn restrict(&mut self, physics: Option<StrafePhysics>) {
        match physics {
            Some(physics) => self.physics = physics,
            None => self.allowed = false,
        }
    }

    /// Emit fixed-capacity rects; CGAZ markers and snap zones are informational, not backplates.
    pub(super) fn emit(&mut self, draw: &mut DrawList, viewport: [f32; 2], time_ms: u64) {
        if !self.allowed {
            return;
        }
        let s = &self.settings;
        let fov = if let Some((_, _, fov)) = self.projection {
            fov
        } else if s.aspect_adjust {
            crate::scope::aspect_adjusted_fov(s.fov, viewport[0] / viewport[1])
        } else {
            s.fov
        };
        if s.helper & 2 != 0 {
            self.emit_lines(draw, viewport, fov);
        }
        if s.aura {
            self.emit_aura(draw, viewport, time_ms);
        }
        if s.helper & 4 != 0 && self.airborne {
            let speed = self.velocity[0].hypot(self.velocity[1]);
            let acceleration = self.physics.air_acceleration * self.speed / s.helper_fps;
            let delta = math::optimal(speed, acceleration, self.speed) + s.offset;
            let velocity_yaw = self.velocity[1].atan2(self.velocity[0]).to_degrees();
            const BITS: [i64; 8] = [32, 64, 256, 65536, 32768, 131072, 512, 128];
            for (key, bit) in BITS.into_iter().enumerate() {
                if s.helper & bit == 0 {
                    continue;
                }
                let angle = velocity_yaw + math::direction(delta, key) - self.angles[1];
                let projected = if let Some((forward, left, _)) = self.projection {
                    math::project_basis(
                        velocity_yaw + math::direction(delta, key),
                        s.precision,
                        forward,
                        left,
                        fov,
                    )
                } else {
                    math::project(angle, -self.angles[0], s.precision, fov)
                };
                let Some(x) = projected else {
                    continue;
                };
                let half = if s.cutoff > 240.0 {
                    5.0
                } else {
                    20.0 - s.cutoff / 16.0
                };
                let color = if key == self.direction && self.full_input {
                    s.active
                } else {
                    let mut color = match key {
                        1 | 7 => Color::new(1.0, 1.0, 1.0, 1.0),
                        2 | 6 => Color::new(0.5, 1.0, 1.0, 1.0),
                        3 | 5 => Color::new(0.75, 0.0, 1.0, 1.0),
                        _ => Color::new(1.0, 0.75, 0.0, 1.0),
                    };
                    color.a = s.inactive;
                    color
                };
                rect(
                    draw,
                    viewport,
                    [x - s.width * 0.5, 240.0 - half, s.width, half * 2.0],
                    color,
                );
            }
        }
        if s.snap {
            let speed = if s.snap_speed == 0.0 {
                self.speed
            } else {
                s.snap_speed
            };
            if self.zone_key != [speed, s.snap_fps] {
                self.zone_key = [speed, s.snap_fps];
                self.zone_count = math::zones(speed, s.snap_fps, &mut self.zones);
            }
            let count = self.zone_count;
            let offset = match s.snap_auto {
                0 => s.snap_def,
                1 | 2 if self.direction == 8 => s.snap_def,
                1 | 2 if self.direction & 1 != 0 => 45.0,
                1 | 2 => 0.0,
                _ => return,
            };
            for i in 0..count {
                for add in [0.0, 90.0] {
                    let span = math::snap_span(
                        self.zones[i] + add,
                        self.zones[i + 1] + add,
                        self.angles[1] + offset,
                        fov,
                    );
                    rect(
                        draw,
                        viewport,
                        [span[0], s.snap_y, span[1], s.snap_height],
                        s.snap_colors[i & 1],
                    );
                }
            }
        }
    }
}

impl Guides {
    /// JoF EJK's "updated style" (`cg_strafeHelper` option 1, `DrawStrafeLine` and
    /// `CG_StrafeHelper` in JoF's `cg_draw.c`): one line per enabled key from the bottom
    /// centre of the screen up to just above the crosshair, where the best heading for
    /// that key projects. On the ground too, with JKA's ground acceleration and friction.
    fn emit_lines(&self, draw: &mut DrawList, viewport: [f32; 2], fov: f32) {
        let s = &self.settings;
        let speed = self.velocity[0].hypot(self.velocity[1]);
        let base = self.speed;
        if speed < base - 1.0 {
            return;
        }
        let frame = 1.0 / s.helper_fps;
        let physics = self.physics;
        let optimal = if self.airborne {
            math::optimal(speed, physics.air_acceleration * base * frame, base)
        } else {
            math::optimal(
                speed * (1.0 - physics.friction * frame),
                physics.ground_acceleration * base * frame,
                base,
            )
        };
        let delta = optimal + s.offset;
        let velocity_yaw = self.velocity[1].atan2(self.velocity[0]).to_degrees();
        // JoF's line top, a little above the crosshair so it stays on the line.
        const TOP: f32 = 230.0;
        let cutoff = (480.0 - s.cutoff.trunc()).clamp(TOP + 20.0, 480.0);
        let width = s.width * viewport[0] / 640.0;
        let scale = |p: [f32; 2]| [p[0] * viewport[0] / 640.0, p[1] * viewport[1] / 480.0];
        for (bit, offset, key, tint) in math::jof_lines(delta) {
            if s.helper & bit == 0 {
                continue;
            }
            let projected = if let Some((forward, left, _)) = self.projection {
                math::project_basis(velocity_yaw + offset, s.precision, forward, left, fov)
            } else {
                math::project(
                    velocity_yaw + offset - self.angles[1],
                    -self.angles[0],
                    s.precision,
                    fov,
                )
            };
            let Some(x) = projected else {
                continue;
            };
            let x = x.clamp(-20_000.0, 20_000.0);
            let color = if key == self.direction {
                s.active
            } else {
                Color::new(tint[0], tint[1], tint[2], s.inactive)
            };
            // JoF draws only the part above the cutoff height.
            let start = 320.0 + (x - 320.0) * (480.0 - cutoff) / (480.0 - TOP);
            let _ = draw.push(DrawCommand::Line {
                from: scale([start, cutoff]),
                to: scale([x, TOP]),
                width,
                color,
            });
        }
    }
}

impl Guides {
    /// SJK's strafe aura: an arc over the crosshair, its top the current view and each
    /// point a turn of that many degrees (left is left), glowing with the speed the keys
    /// held would gain there, shaded per pixel by the renderer
    /// ([`sjk_ui::strafe_aura_shade`]). The best heading is a white-hot seam: short of
    /// it nothing is gained, past it the glow thins and fades, and red marks headings
    /// that slow you down. The view tick turns blue once you are on it. Airborne only
    /// unless `cg_strafeAuraGround`; otherwise the lines' physics and gates.
    fn emit_aura(&self, draw: &mut DrawList, viewport: [f32; 2], time_ms: u64) {
        let s = &self.settings;
        let Some(&wish) = WISH.get(self.direction) else {
            return;
        };
        let speed = self.velocity[0].hypot(self.velocity[1]);
        let base = self.speed;
        if speed < base - 1.0 || s.aura_alpha <= 0.0 || !(self.airborne || s.aura_ground) {
            return;
        }
        let frame = 1.0 / s.helper_fps;
        let physics = self.physics;
        let (vf, acceleration) = if self.airborne {
            (speed, physics.air_acceleration * base * frame)
        } else {
            (
                speed * (1.0 - physics.friction * frame),
                physics.ground_acceleration * base * frame,
            )
        };
        let best = math::peak(vf, acceleration, base);
        let top = math::gain(vf, acceleration, base, best);
        if top <= 0.0 {
            return;
        }
        let velocity_yaw = self.velocity[1].atan2(self.velocity[0]).to_degrees();
        // The wish angle off the velocity at the top, the view.
        let base_angle = (self.angles[1] + wish - velocity_yaw + 180.0).rem_euclid(360.0) - 180.0;
        let unit = viewport[1] / 480.0;
        let center = [viewport[0] * 0.5, viewport[1] * 0.5];
        let radius = s.aura_radius * unit;
        let alpha = s.aura_alpha;
        let _ = draw.push(DrawCommand::StrafeAura {
            center,
            aura: sjk_ui::StrafeAura {
                radius,
                width: radius * AURA_WIDTH,
                span: AURA_SPAN.to_radians(),
                speed: vf,
                acceleration,
                wishspeed: base,
                base: base_angle.to_radians(),
                time: (time_ms % 1_000_000) as f32 * 0.001,
                alpha,
            },
        });
        // The view tick: where the crosshair sits on the arc.
        let current = math::gain(vf, acceleration, base, base_angle) / top;
        let tick = if current > 0.9 {
            Color::new(0.55, 0.85, 1.0, alpha)
        } else if current < -0.02 {
            Color::new(1.0, 0.35, 0.3, 0.8 * alpha)
        } else {
            Color::new(1.0, 1.0, 1.0, 0.5 * alpha)
        };
        let _ = draw.push(DrawCommand::Line {
            from: [center[0], center[1] - radius - 6.0 * unit],
            to: [center[0], center[1] - radius + 6.0 * unit],
            width: 1.5 * unit,
            color: tick,
        });
    }
}

impl HudOverlay {
    /// Apply the camera/physics safety gate and real keyboard direction after projection.
    pub(crate) fn restrict_guides(
        &mut self,
        physics: Option<StrafePhysics>,
        keys: Option<[i8; 2]>,
    ) {
        self.guides.restrict(physics);
        if let Some([f, r]) = keys {
            self.guides.full_input = [f, r].into_iter().all(|v| v == 0 || v == 127 || v == -127);
            self.guides.direction = match (f.signum(), r.signum()) {
                (1, 0) => 0,
                (1, -1) => 1,
                (0, -1) => 2,
                (-1, -1) => 3,
                (-1, 0) => 4,
                (-1, 1) => 5,
                (0, 1) => 6,
                (1, 1) => 7,
                _ => 8,
            };
        }
    }

    /// Use the rendered camera axes/FOV, including bob and roll, rather than a second camera.
    pub(crate) fn guide_camera(
        &mut self,
        forward: glam::Vec3,
        up: glam::Vec3,
        vertical_fov: f32,
        aspect: f32,
    ) {
        let forward = forward.normalize_or_zero();
        let left = up.cross(forward).normalize_or_zero();
        let fov = ((vertical_fov * 0.5).to_radians().tan() * aspect)
            .atan()
            .to_degrees()
            * 2.0;
        self.guides.angles[1] = forward.y.atan2(forward.x).to_degrees();
        self.guides.projection = Some((forward.to_array(), left.to_array(), fov));
    }
}

fn rect(draw: &mut DrawList, viewport: [f32; 2], r: [f32; 4], color: Color) {
    if !r.iter().all(|x| x.is_finite()) {
        return;
    }
    let x = r[0].max(0.0);
    let end = (r[0] + r[2]).min(640.0);
    let y = r[1].max(0.0);
    let bottom = (r[1] + r[3]).min(480.0);
    if x >= end || y >= bottom {
        return;
    }
    let _ = draw.push(DrawCommand::SolidRect {
        rect: Rect::new(
            x * viewport[0] / 640.0,
            y * viewport[1] / 480.0,
            (end - x) * viewport[0] / 640.0,
            (bottom - y) * viewport[1] / 480.0,
        ),
        color,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jof_lines_rise_from_the_bottom_to_each_side_of_the_crosshair() {
        // Sol's JoF EJK value: lines style, WA, WD and the centre (not drawn for JKA).
        let mut guides = Guides {
            velocity: [400.0, 0.0, 0.0],
            speed: 250.0,
            airborne: true,
            allowed: true,
            direction: 1,
            ..Guides::default()
        };
        guides.settings.helper = 2242;
        let mut draw = DrawList::new(16);
        guides.emit(&mut draw, [640.0, 480.0], 0);
        let lines: Vec<_> = draw
            .commands()
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from, to, color, ..
                } => Some((from, to, color)),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 2);
        let (wa_from, wa_to, wa_color) = lines[0];
        let (_, wd_to, wd_color) = lines[1];
        assert_eq!(wa_from, [320.0, 480.0]);
        assert_eq!(wa_to[1], 230.0);
        // WA's best heading is left of the velocity, WD's right, mirrored.
        assert!(wa_to[0] < 320.0 && wd_to[0] > 320.0);
        assert!((wa_to[0] + wd_to[0] - 640.0).abs() < 0.01);
        // Holding WA lights its line.
        assert_eq!(wa_color, guides.settings.active);
        assert_eq!(wd_color.a, guides.settings.inactive);
    }

    #[test]
    fn the_gain_peaks_at_the_best_angle() {
        let (v, a, w) = (400.0, 2.0, 250.0);
        let best = math::peak(v, a, w);
        assert!((best - (248.0f32 / 400.0).acos().to_degrees()).abs() < 1e-3);
        let top = math::gain(v, a, w, best);
        assert!(top > 0.0);
        assert!(math::gain(v, a, w, best - 1.0) < top);
        assert!(math::gain(v, a, w, best + 1.0) < top);
        // Short of the edge the push adds nothing; far past it, it slows you down.
        assert_eq!(math::gain(v, a, w, best - 2.0), 0.0);
        assert!(math::gain(v, a, w, 120.0) < 0.0);
    }

    #[test]
    fn the_aura_puts_the_best_heading_where_to_turn() {
        // Holding W+A, looking along the velocity: WA wishes 45 degrees left, the best
        // angle at 400 ups is about 51.7, so the white seam is about 6.7 degrees left.
        let mut guides = Guides {
            velocity: [400.0, 0.0, 0.0],
            speed: 250.0,
            airborne: true,
            allowed: true,
            direction: 1,
            ..Guides::default()
        };
        guides.settings.helper = 0;
        guides.settings.helper_fps = 125.0;
        let mut draw = DrawList::new(super::AURA_COMMANDS);
        guides.emit(&mut draw, [640.0, 480.0], 0);
        assert!(draw.is_empty(), "drawn while cg_strafeAura is off");
        guides.settings.aura = true;
        guides.emit(&mut draw, [640.0, 480.0], 0);
        let Some(&DrawCommand::StrafeAura { center, aura }) = draw.commands().first() else {
            panic!("no aura: {:?}", draw.commands());
        };
        assert_eq!(center, [320.0, 240.0]);
        assert!((aura.base.to_degrees() - 45.0).abs() < 1e-3);
        let best = (math::peak(400.0, 2.0, 250.0) - 45.0).to_radians();
        let point = |turn: f32| {
            let angle = -std::f32::consts::FRAC_PI_2 - turn;
            [aura.radius * angle.cos(), aura.radius * angle.sin()]
        };
        // White-hot left of the top, dark at the top (short of the best heading).
        assert!(sjk_ui::strafe_aura_shade(point(best), &aura)[3] > 0.9);
        assert!(sjk_ui::strafe_aura_shade(point(0.0), &aura)[3] < 0.01);
        assert!(sjk_ui::strafe_aura_shade(point(-best), &aura)[3] < 0.01);
        assert_eq!(draw.len(), super::AURA_COMMANDS);
    }

    #[test]
    fn the_aura_waits_for_a_jump_unless_asked_on_the_ground() {
        let mut guides = Guides {
            velocity: [400.0, 0.0, 0.0],
            speed: 250.0,
            airborne: false,
            allowed: true,
            direction: 1,
            ..Guides::default()
        };
        guides.settings.helper = 0;
        guides.settings.aura = true;
        let mut draw = DrawList::new(super::AURA_COMMANDS);
        guides.emit(&mut draw, [640.0, 480.0], 0);
        assert!(draw.is_empty(), "drawn while walking");
        guides.settings.aura_ground = true;
        guides.emit(&mut draw, [640.0, 480.0], 0);
        assert_eq!(draw.len(), super::AURA_COMMANDS);
    }

    #[test]
    fn the_guides_follow_a_jetpack_flight() {
        let mut player = PlayerState::default();
        let mut guides = Guides::default();
        player.set_movement_type(1);
        guides.update(&player, None, None);
        assert!(guides.allowed, "PM_JETPACK");
        // Spectating (PM_SPECTATOR, 2) and the like stay without guides.
        player.set_movement_type(2);
        guides.update(&player, None, None);
        assert!(!guides.allowed);
    }

    #[test]
    fn jof_lines_wait_for_running_speed() {
        let mut guides = Guides {
            velocity: [200.0, 0.0, 0.0],
            speed: 250.0,
            allowed: true,
            ..Guides::default()
        };
        guides.settings.helper = 2242;
        let mut draw = DrawList::new(16);
        guides.emit(&mut draw, [640.0, 480.0], 0);
        assert!(draw.is_empty());
    }
}
