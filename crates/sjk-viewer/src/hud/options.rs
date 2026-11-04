//! Retained optional readouts; no per-frame string allocation.
use super::*;
use sjk_ui::{DrawCommand, FontWeight, Rect, TextAlign, TextOverflow};
use std::fmt::Write;

/// JoF EJK `CG_DrawHUD` shifts the common speedometer anchor clear of these
/// game-data HUDs after reading `cg_speedometerX` (JoF `cg_draw.c:2236-2245`).
const JOF_HUD_SPEED_X_OFFSETS: &[(i64, f32)] = &[(0, -8.0), (1, -56.0), (2, -42.0)];
const JOF_TEXT_HUD_WIDE_ARMOR_THRESHOLD: i32 = 999;
const JOF_TEXT_HUD_WIDE_ARMOR_OFFSET: f32 = 4.0;

/// Stock cg_draw.c:5194-5199 sizes the crosshair in virtual screen coordinates.
pub(crate) fn crosshair_size(console: Option<&ViewerConsole>, visible: bool) -> f32 {
    if !visible {
        return 0.0;
    }
    console
        .and_then(|c| c.float_cvar("cg_crosshairsize"))
        .filter(|v| v.is_finite())
        .unwrap_or(24.0)
        .clamp(0.0, 640.0) as f32
}

/// Retained speed label, updated from predicted velocity.
pub(super) struct Speed {
    /// Text storage reserved once when the HUD is constructed.
    pub(super) text: String,
    enabled: bool,

    position: [f32; 3],
}

impl Default for Speed {
    fn default() -> Self {
        Self {
            text: String::with_capacity(48),
            enabled: false,

            position: [132.0, 459.0, 0.75],
        }
    }
}

impl Speed {
    /// Show `text` with its right edge at `x` (640-unit), for snapshots.
    #[cfg(test)]
    pub(super) fn preview(&mut self, text: &str, x: f32) {
        self.enabled = true;
        self.text.clear();
        self.text.push_str(text);
        self.position = [x, 459.0, 0.75];
    }

    /// Apply supported speedometer flags to the current predicted velocity.
    pub(super) fn update(
        &mut self,
        velocity: [f32; 3],
        armor: i32,
        console: Option<&ViewerConsole>,
    ) {
        self.position = [
            ("cg_speedometerx", 132.0),
            ("cg_speedometery", 459.0),
            ("cg_speedometersize", 0.75),
        ]
        .map(|(name, value)| crate::cgame_options::scalar(console, name, value));
        // JoF shifts only for its numbered HUDs (`cgs.newHud`); a menu file path such as
        // the retail `ui/jahud.txt` gets no shift.
        if crate::menu_hud::HudStyle::read(console) == crate::menu_hud::HudStyle::Game {
            if let Some(files) = console
                .and_then(|c| c.text_value(crate::menu_hud::FILES_CVAR))
                .and_then(|value| value.trim().parse::<i64>().ok())
            {
                self.position[0] += speedometer_hud_x_offset(files, armor);
            }
        }
        let bits = console
            .and_then(|c| c.integer_cvar("cg_speedometer"))
            .unwrap_or(0);
        self.set(velocity, bits);
    }

    fn set(&mut self, velocity: [f32; 3], bits: i64) {
        self.enabled = bits & 1 != 0;
        self.text.clear();
        if !self.enabled {
            return;
        }
        let [x, y, z] = velocity;
        let speed = (x * x + y * y + if bits & 32768 != 0 { z * z } else { 0.0 }).sqrt();
        let (factor, unit) = if bits & 256 != 0 {
            (0.1028699967, "km/h")
        } else if bits & 512 != 0 {
            (0.06392043271, "mph")
        } else {
            (1.0, "ups")
        };
        let value = if speed.is_finite() {
            (speed * factor).min(999_999.0)
        } else {
            0.0
        };
        let _ = write!(self.text, "{:.0} {unit}", (value + 0.5).floor());
    }

    /// Append a text command referencing retained storage, without a backplate.
    pub(super) fn emit(&self, draw: &mut DrawList, theme: Theme, viewport: [f32; 2]) {
        if !self.enabled {
            return;
        }
        let s = crate::ui_scale::height_scale(viewport[1]);
        let right = self.position[0] * viewport[0] / 640.0;
        let width = 240.0 * s;
        let _ = draw.push(DrawCommand::Text {
            rect: Rect::new(
                right - width,
                self.position[1] * viewport[1] / 480.0 - 32.0 * s,
                width,
                32.0 * s,
            ),
            text: TextId(314),
            size: 32.0 * s * self.position[2].clamp(0.0, 4.0),
            color: theme.foreground,
            // JoF EJK passes cg_speedometerX to CG_Text_Paint with
            // ITEM_ALIGN_RIGHT: the cvar names the text's right edge.
            align: TextAlign::End,
            overflow: TextOverflow::Clip,
            weight: FontWeight::Semibold,
            letter_spacing: 0.0,
        });
    }
}

fn speedometer_hud_x_offset(files: i64, armor: i32) -> f32 {
    let base = JOF_HUD_SPEED_X_OFFSETS
        .iter()
        .find_map(|&(known, offset)| (files == known).then_some(offset))
        .unwrap_or(0.0);
    base + if files == 1 && armor > JOF_TEXT_HUD_WIDE_ARMOR_THRESHOLD {
        JOF_TEXT_HUD_WIDE_ARMOR_OFFSET
    } else {
        0.0
    }
}

impl HudOverlay {}

#[cfg(test)]
mod tests {
    use super::*;

    /// JoF EJK's `CG_Speedometer` paints at `cg_speedometerX` with
    /// `ITEM_ALIGN_RIGHT`; changing the field width must not move that edge.
    #[test]
    fn speedometer_x_is_the_right_edge() {
        let mut speed = Speed::default();
        speed.set([300.0, 400.0, 0.0], 1);
        let mut draw = DrawList::new(1);
        speed.emit(&mut draw, Theme::default(), [1_920.0, 1_080.0]);

        let [DrawCommand::Text { rect, align, .. }] = draw.commands() else {
            panic!("expected one speedometer text command");
        };
        assert_eq!(*align, TextAlign::End);
        assert!((rect.right() - 396.0).abs() < f32::EPSILON);
    }

    #[test]
    fn game_hud_offsets_match_jof_eternaljk() {
        assert_eq!(speedometer_hud_x_offset(0, 0), -8.0);
        assert_eq!(speedometer_hud_x_offset(1, 0), -56.0);
        assert_eq!(speedometer_hud_x_offset(1, 1_000), -52.0);
        assert_eq!(speedometer_hud_x_offset(2, 0), -42.0);
        assert_eq!(speedometer_hud_x_offset(3, 0), 0.0);
    }

    #[test]
    fn only_the_game_hud_applies_its_jof_offset() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(console.set_cvar("cg_speedometer", "1"));
        assert!(console.set_cvar("cg_speedometerX", "132"));
        assert!(console.set_cvar("cg_hudStyle", "game"));
        assert!(console.set_cvar("cg_hudFiles", "1"));

        let mut speed = Speed::default();
        speed.update([300.0, 400.0, 0.0], 0, Some(&console));
        let mut draw = DrawList::new(1);
        speed.emit(&mut draw, Theme::default(), [1_920.0, 1_080.0]);
        let [DrawCommand::Text { rect, .. }] = draw.commands() else {
            panic!("expected one speedometer text command");
        };
        assert!((rect.right() - 228.0).abs() < f32::EPSILON);

        assert!(console.set_cvar("cg_hudStyle", "classic"));
        speed.update([300.0, 400.0, 0.0], 0, Some(&console));
        draw.clear();
        speed.emit(&mut draw, Theme::default(), [1_920.0, 1_080.0]);
        let [DrawCommand::Text { rect, .. }] = draw.commands() else {
            panic!("expected one speedometer text command");
        };
        assert!((rect.right() - 396.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_menu_file_path_gets_no_jof_offset() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(console.set_cvar("cg_speedometer", "1"));
        assert!(console.set_cvar("cg_speedometerX", "132"));
        assert!(console.set_cvar("cg_hudStyle", "game"));
        assert!(console.set_cvar("cg_hudFiles", "ui/jahud.txt"));

        let mut speed = Speed::default();
        speed.update([300.0, 400.0, 0.0], 0, Some(&console));
        let mut draw = DrawList::new(1);
        speed.emit(&mut draw, Theme::default(), [1_920.0, 1_080.0]);
        let [DrawCommand::Text { rect, .. }] = draw.commands() else {
            panic!("expected one speedometer text command");
        };
        assert!((rect.right() - 396.0).abs() < f32::EPSILON);
    }
}
