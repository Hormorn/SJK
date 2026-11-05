//! Rain and wind heard (`s_weatherSounds`, off by default as every change to how stock
//! Jedi Academy sounds): the game's own outdoor loops, `rainy_lp` and `windy1_lp` of
//! `sound/ambience/outdoor/`, in both ears, as loud as the weather is strong and as open
//! the sky is round the listener.
//!
//! - **Rain** follows the rain clouds' haze ([`rain`]): a downpour (`heavyrain`) is a full
//!   loop, plain rain about half of it, drizzle a third; snow and fog are silent.
//! - **Wind** follows the wind's speed across the ground ([`wind`]): a gust of the
//!   gusting wind (3000 units a second) is a full loop, so it rises and falls with it.
//! - **Shelter** ([`exposure`]) reads the weather cover's columns round the listener (the
//!   same survey that keeps rain off roofs): under open sky the loops play fully, under a
//!   roof or indoors a fifth of it reaches the ear ([`SHELTERED`]), the rain on the roof.
//!
//! Frozen weather (`r_we freeze`) is still, `r_weather 0` silent. Levels ease over
//! [`EASE`] seconds, so stepping under a roof or a gust coming does not jump.

use super::cover::Column;
use super::effects::Cloud;

/// The setting: rain and wind heard (off by default).
pub(crate) const CVAR: &str = "s_weatherSounds";
/// The game's own loops (`assets0.pk3`).
pub(crate) const RAIN_LOOP: &str = "sound/ambience/outdoor/rainy_lp.wav";
pub(crate) const WIND_LOOP: &str = "sound/ambience/outdoor/windy1_lp.wav";
/// Each loop's loudness at a full downpour or gale in the open.
pub(crate) const RAIN_VOLUME: f32 = 0.6;
pub(crate) const WIND_VOLUME: f32 = 0.5;
/// What reaches the listener under a roof or indoors, of what the open sky gives.
pub(crate) const SHELTERED: f32 = 0.2;
/// The rain haze of a downpour (`heavyrain`, `weather_effects.rs`): a full rain loop.
const DOWNPOUR_HAZE: f32 = 2.8e-4;
/// Wind speed across the ground heard as a full gale (units a second): the gusting
/// wind's strongest.
const GALE: f32 = 3000.0;
/// How far round the listener the sky is looked at, and the step between columns.
const REACH: f32 = 192.0;
const STEP: f32 = 64.0;
/// Weather reaches the listener in a column whose floor is at most this far above them.
const HEAD_ROOM: f32 = 64.0;
/// Seconds over which a loop eases most of the way to a new loudness.
pub(crate) const EASE: f32 = 0.6;

/// How loud each loop plays, 0..1 before its volume.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Levels {
    pub(crate) rain: f32,
    pub(crate) wind: f32,
}

impl Levels {
    /// Both at `share` of themselves.
    fn scaled(self, share: f32) -> Self {
        Self {
            rain: self.rain * share,
            wind: self.wind * share,
        }
    }

    /// Ease towards `target` over `seconds` (none: at once).
    pub(crate) fn ease(&mut self, target: Self, seconds: f32) {
        let step = if seconds <= 0.0 {
            1.0
        } else {
            1.0 - (-seconds / EASE).exp()
        };
        self.rain += (target.rain - self.rain) * step;
        self.wind += (target.wind - self.wind) * step;
        // Below hearing: silent, so the loops stop.
        if self.rain < 1e-3 {
            self.rain = 0.0;
        }
        if self.wind < 1e-3 {
            self.wind = 0.0;
        }
    }
}

/// Rain falling, 0..1: the rain clouds' haze against a downpour's.
pub(crate) fn rain(clouds: &[Cloud]) -> f32 {
    let haze: f32 = clouds
        .iter()
        .filter(|cloud| cloud.is_rain())
        .map(|cloud| cloud.haze)
        .sum();
    (haze / DOWNPOUR_HAZE).clamp(0.0, 1.0)
}

/// Wind blowing, 0..1: its speed across the ground against a gale's.
pub(crate) fn wind(velocity: [f32; 3]) -> f32 {
    (velocity[0].hypot(velocity[1]) / GALE).clamp(0.0, 1.0)
}

/// How open the sky is round a listener at `at`, 0 (sheltered) to 1, from the columns
/// `column` gives (`None`: not surveyed yet): each column within [`REACH`] counts by its
/// nearness, open when weather comes down to within [`HEAD_ROOM`] of the listener's
/// height and its sky is above them. `None` when no column is surveyed.
pub(crate) fn exposure(at: [f32; 3], column: impl Fn(f32, f32) -> Option<Column>) -> Option<f32> {
    let steps = (REACH / STEP) as i32;
    let (mut open, mut total) = (0.0_f32, 0.0_f32);
    for dy in -steps..=steps {
        for dx in -steps..=steps {
            let (x, y) = (dx as f32 * STEP, dy as f32 * STEP);
            let distance = x.hypot(y);
            if distance > REACH {
                continue;
            }
            let Some(found) = column(at[0] + x, at[1] + y) else {
                continue;
            };
            let weight = 1.0 - distance / (REACH + STEP);
            total += weight;
            if found.is_open() && found.bottom <= at[2] + HEAD_ROOM && found.top >= at[2] {
                open += weight;
            }
        }
    }
    (total > 0.0).then(|| (open / total).clamp(0.0, 1.0))
}

/// The loops' loudness for `clouds` and `wind`, at `exposure` (none: unknown, heard as
/// open).
pub(crate) fn target(clouds: &[Cloud], wind_velocity: [f32; 3], exposure: Option<f32>) -> Levels {
    let open = exposure.unwrap_or(1.0);
    Levels {
        rain: rain(clouds),
        wind: wind(wind_velocity),
    }
    .scaled(SHELTERED + (1.0 - SHELTERED) * open)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weather::effects::Effects;

    fn clouds(commands: &[&str]) -> Vec<Cloud> {
        Effects::from_commands(commands.iter().copied()).clouds
    }

    #[test]
    fn rain_is_as_loud_as_it_falls_and_snow_is_silent() {
        assert_eq!(rain(&clouds(&["heavyrain"])), 1.0);
        let plain = rain(&clouds(&["rain"]));
        let drizzle = rain(&clouds(&["lightrain"]));
        assert!(0.5 < plain && plain < 0.6, "{plain}");
        assert!(0.3 < drizzle && drizzle < plain, "{drizzle}");
        assert_eq!(rain(&clouds(&["snow", "fog", "mist"])), 0.0);
        assert_eq!(
            rain(&clouds(&["heavyrain", "rain"])),
            1.0,
            "never above a loop"
        );
    }

    #[test]
    fn wind_follows_its_speed_across_the_ground() {
        assert_eq!(wind([0.0, 0.0, 900.0]), 0.0, "up and down is not heard");
        assert_eq!(wind([-5000.0, 0.0, 0.0]), 1.0);
        assert!((wind([1800.0, 2400.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!((wind([1500.0, 0.0, 0.0]) - 0.5).abs() < 1e-6);
    }

    fn open(bottom: f32, top: f32) -> Option<Column> {
        Some(Column {
            bottom,
            top,
            flags: Column::SPLASH,
        })
    }

    #[test]
    fn a_roof_shelters_and_open_ground_does_not() {
        // Out in the open: rain comes down to the ground under the sky.
        assert_eq!(
            exposure([0.0, 0.0, 40.0], |_, _| open(0.0, 900.0)),
            Some(1.0)
        );
        // Under a roof at 200: the columns open above it only.
        assert_eq!(
            exposure([0.0, 0.0, 40.0], |_, _| open(200.0, 900.0)),
            Some(0.0)
        );
        // In a doorway: the open half of the street nearer counts more than its share.
        let doorway = exposure([0.0, 0.0, 40.0], |x, _| {
            if x >= 0.0 {
                open(0.0, 900.0)
            } else {
                Some(Column::COVERED)
            }
        })
        .unwrap();
        assert!(0.5 < doorway && doorway < 0.7, "{doorway}");
        assert_eq!(exposure([0.0; 3], |_, _| None), None, "not surveyed yet");
        // Above the sky's end (a marked outside below the listener): not in the weather.
        assert_eq!(
            exposure([0.0, 0.0, 950.0], |_, _| open(0.0, 900.0)),
            Some(0.0)
        );
    }

    #[test]
    fn sheltered_weather_is_a_fifth_and_levels_ease() {
        let storm = clouds(&["heavyrain"]);
        let gale = [3000.0, 0.0, 0.0];
        assert_eq!(
            target(&storm, gale, Some(1.0)),
            Levels {
                rain: 1.0,
                wind: 1.0
            }
        );
        let indoors = target(&storm, gale, Some(0.0));
        assert!((indoors.rain - SHELTERED).abs() < 1e-6);
        assert_eq!(
            target(&storm, gale, None).rain,
            1.0,
            "unknown is heard as open"
        );
        let mut heard = Levels::default();
        heard.ease(target(&storm, gale, Some(1.0)), EASE);
        assert!(0.6 < heard.rain && heard.rain < 0.65, "{}", heard.rain);
        heard.ease(Levels::default(), 0.0);
        assert_eq!(heard, Levels::default(), "no time: at once");
        heard.ease(
            Levels {
                rain: 1e-4,
                wind: 1e-4,
            },
            0.0,
        );
        assert_eq!(heard, Levels::default(), "below hearing is silent");
    }
}
