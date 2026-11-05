//! The local time on the HUD (Nib's idea, #ideas 05/11/2026: "local time like drawn in
//! the console, but place-able in the ingame hud", with time, date and 24/12 h).
//!
//! [`CVAR`] (`cg_drawClock`, off by default) shows the PC's local time, and with 2 the
//! date after it (`dd/mm/yyyy`, as SJK writes dates); [`HOURS_CVAR`] (`cg_clock12h`)
//! counts the hours from 1 to 12 with am/pm instead of 0 to 23. The text is centred on
//! [`X_CVAR`] (640-wide units) with its bottom at [`Y_CVAR`] (480-high units), scaled by
//! [`SIZE_CVAR`]; the HUD edit mode drags it like the speedometer
//! ([`super::placement::Element::Clock`]). The time is read at most once a second and
//! written into retained storage, so a frame allocates nothing.
use super::*;
use sjk_shell::local_time::LocalTime;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{DrawCommand, FontWeight, Rect, TextAlign, TextOverflow};
use std::fmt::Write;
use std::time::{Duration, Instant};

/// 0 off (the default), 1 the time, 2 the time and the date.
pub(crate) const CVAR: &str = "cg_drawClock";
/// Whether the hours run 1 to 12 with am/pm (off: 0 to 23).
pub(crate) const HOURS_CVAR: &str = "cg_clock12h";
/// The text's centre, 640-wide screen.
pub(crate) const X_CVAR: &str = "cg_clockX";
/// The text's bottom, 480-high screen.
pub(crate) const Y_CVAR: &str = "cg_clockY";
/// The text's scale.
pub(crate) const SIZE_CVAR: &str = "cg_clockSize";
pub(crate) const DEFAULT_X: f64 = 320.0;
pub(crate) const DEFAULT_Y: f64 = 476.0;
pub(crate) const DEFAULT_SIZE: f64 = 0.6;
/// The text's id in [`HudOverlay::resolve_text`].
pub(super) const TEXT: u32 = 349;

/// How often the PC's clock is read.
const EVERY: Duration = Duration::from_secs(1);

/// What the clock shows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Show {
    #[default]
    Off,
    Time,
    TimeAndDate,
}

impl Show {
    fn of(value: i64) -> Self {
        match value {
            1 => Self::Time,
            2.. => Self::TimeAndDate,
            _ => Self::Off,
        }
    }
}

/// Register the clock's options.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        CVAR,
        0_i64,
        CvarFlags::ARCHIVE,
        "Local time on the HUD: 0 off, 1 time, 2 time and date",
    ))?;
    cvars.register(CvarDefinition::new(
        HOURS_CVAR,
        false,
        CvarFlags::ARCHIVE,
        "HUD clock counts hours 1 to 12 with am/pm",
    ))?;
    for (name, value, help) in [
        (X_CVAR, DEFAULT_X, "HUD clock's centre, 640-wide screen"),
        (Y_CVAR, DEFAULT_Y, "HUD clock's bottom, 480-high screen"),
        (SIZE_CVAR, DEFAULT_SIZE, "HUD clock's scale"),
    ] {
        cvars.register(CvarDefinition::new(name, value, CvarFlags::ARCHIVE, help))?;
    }
    Ok(())
}

/// The retained clock text and where it goes.
pub(super) struct Clock {
    /// Text storage reserved once when the HUD is constructed.
    pub(super) text: String,
    show: Show,
    twelve: bool,
    /// x, y and scale.
    position: [f32; 3],
    /// When the PC's clock was last read, and what it said.
    read: Option<(Instant, LocalTime)>,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            text: String::with_capacity(32),
            show: Show::Off,
            twelve: false,
            position: [DEFAULT_X as f32, DEFAULT_Y as f32, DEFAULT_SIZE as f32],
            read: None,
        }
    }
}

impl Clock {
    /// Follow the options and, once a second, the PC's clock.
    pub(super) fn update(&mut self, console: Option<&ViewerConsole>) {
        let show = Show::of(console.and_then(|c| c.integer_cvar(CVAR)).unwrap_or(0));
        if show == Show::Off {
            self.show = show;
            self.text.clear();
            self.read = None;
            return;
        }
        self.position = [
            (X_CVAR, DEFAULT_X),
            (Y_CVAR, DEFAULT_Y),
            (SIZE_CVAR, DEFAULT_SIZE),
        ]
        .map(|(name, value)| crate::cgame_options::scalar(console, name, value));
        let twelve = console
            .and_then(|c| c.integer_cvar(HOURS_CVAR))
            .is_some_and(|value| value != 0);
        let now = Instant::now();
        let time = match self.read {
            Some((at, time)) if now.saturating_duration_since(at) < EVERY => time,
            _ => {
                let time = LocalTime::now();
                self.read = Some((now, time));
                time
            }
        };
        self.set(show, twelve, time);
    }

    /// Write `time` as `show` and `twelve` say.
    fn set(&mut self, show: Show, twelve: bool, time: LocalTime) {
        self.show = show;
        self.twelve = twelve;
        self.text.clear();
        if show == Show::Off {
            return;
        }
        if twelve {
            let hour = match time.hour % 12 {
                0 => 12,
                hour => hour,
            };
            let half = if time.hour < 12 { "am" } else { "pm" };
            let _ = write!(self.text, "{hour}:{:02} {half}", time.minute);
        } else {
            let _ = write!(self.text, "{:02}:{:02}", time.hour, time.minute);
        }
        if show == Show::TimeAndDate {
            let _ = write!(
                self.text,
                "  {:02}/{:02}/{}",
                time.day, time.month, time.year
            );
        }
    }

    /// Show `text` as the clock, for snapshots.
    #[cfg(test)]
    pub(super) fn preview(&mut self, time: LocalTime, show: Show, twelve: bool) {
        self.set(show, twelve, time);
    }

    /// Append the clock's text command, referencing retained storage.
    pub(super) fn emit(&self, draw: &mut DrawList, theme: Theme, viewport: [f32; 2]) {
        if self.show == Show::Off || self.text.is_empty() {
            return;
        }
        let s = crate::ui_scale::height_scale(viewport[1]);
        let size = 32.0 * s * self.position[2].clamp(0.1, 4.0);
        let centre = self.position[0] * viewport[0] / 640.0;
        let bottom = self.position[1] * viewport[1] / 480.0;
        let width = 16.0 * size;
        let _ = draw.push(DrawCommand::Text {
            rect: Rect::new(
                centre - width / 2.0,
                bottom - size * 1.25,
                width,
                size * 1.25,
            ),
            text: TextId(TEXT),
            size,
            color: theme.foreground,
            align: TextAlign::Center,
            overflow: TextOverflow::Clip,
            weight: FontWeight::Semibold,
            letter_spacing: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u8, minute: u8) -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 5,
            weekday: 4,
            hour,
            minute,
            second: 0,
        }
    }

    #[test]
    fn the_clock_reads_24_or_12_hours_and_the_date_day_first() {
        let mut clock = Clock::default();
        clock.preview(at(9, 5), Show::Time, false);
        assert_eq!(clock.text, "09:05");
        clock.preview(at(21, 30), Show::TimeAndDate, false);
        assert_eq!(clock.text, "21:30  05/11/2026");
        for (hour, shown) in [
            (0, "12:07 am"),
            (9, "9:07 am"),
            (12, "12:07 pm"),
            (23, "11:07 pm"),
        ] {
            clock.preview(at(hour, 7), Show::Time, true);
            assert_eq!(clock.text, shown, "{hour}");
        }
        clock.preview(at(9, 5), Show::Off, false);
        assert!(clock.text.is_empty());
    }

    #[test]
    fn the_clock_is_off_by_default_and_draws_nothing_then() {
        let mut clock = Clock::default();
        clock.update(None);
        assert_eq!(clock.show, Show::Off);
        let mut draw = DrawList::new(1);
        clock.emit(&mut draw, Theme::default(), [1_920.0, 1_080.0]);
        assert!(draw.commands().is_empty());
        assert_eq!(Show::of(0), Show::Off);
        assert_eq!(Show::of(1), Show::Time);
        assert_eq!(Show::of(2), Show::TimeAndDate);
        assert_eq!(Show::of(-1), Show::Off);
    }

    /// The text is centred on `cg_clockX` with its bottom on `cg_clockY`.
    #[test]
    fn the_clock_stands_where_its_cvars_say() {
        let mut clock = Clock::default();
        clock.preview(at(9, 5), Show::Time, false);
        let mut draw = DrawList::new(1);
        clock.emit(&mut draw, Theme::default(), [1_920.0, 1_080.0]);
        let [DrawCommand::Text { rect, align, .. }] = draw.commands() else {
            panic!("expected one clock text command");
        };
        assert_eq!(*align, TextAlign::Center);
        assert!((rect.x + rect.width / 2.0 - 960.0).abs() < 0.01);
        assert!((rect.y + rect.height - DEFAULT_Y as f32 * 1_080.0 / 480.0).abs() < 0.01);
    }
}
