//! Live hour/rate/clarity snapshots. Resource-enabling controls remain startup policy.
use sjk_shell::{CvarError, CvarRegistry, CvarValue, local_time::LocalTime};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU32, AtomicU64, Ordering},
};
use std::time::Instant;

/// `r_dayClock`: the hour of the sun follows this PC's clock.
pub(crate) const PC_CLOCK_CVAR: &str = "r_dayClock";
/// `r_daySky`: without the live sun, the sky still shows the hour (`r_dayHour`, turning
/// with `r_dayMinutes`): dawn, dusk and night tint it, and the night darkens the view.
pub(crate) const SKY_CVAR: &str = "r_daySky";
/// How often the PC's clock is read while [`PC_CLOCK_CVAR`] is on, in milliseconds: the
/// sun moves a quarter of a degree a minute, so once a second is plenty, and the frame
/// never asks the time zone rules itself.
const PC_CLOCK_EVERY_MS: u64 = 1_000;

/// The hour of the sun for the PC's local time `time`: hours, minutes and seconds as a
/// fraction of the hour, 0 up to 24.
pub(crate) fn pc_hour(time: &LocalTime) -> f32 {
    (f32::from(time.hour) + f32::from(time.minute) / 60. + f32::from(time.second) / 3600.)
        .rem_euclid(24.)
}

/// How dark the PC clock's night is without the live sun: the exposure's factor from
/// [`NIGHT_DARK_FROM`] to [`NIGHT_DARK_UNTIL`]: night in the SJK and Vanilla presets too, which
/// keep real-time lighting off.
pub(crate) const NIGHT_FACTOR: f32 = 0.4;
/// Dusk: full day until 18:00, full night from 20:30.
const DUSK: (f32, f32) = (18.0, 20.5);
/// Dawn: full night until 05:00, full day from 07:00.
const DAWN: (f32, f32) = (5.0, 7.0);
#[cfg(test)]
const NIGHT_DARK_FROM: f32 = DUSK.1;
#[cfg(test)]
const NIGHT_DARK_UNTIL: f32 = DAWN.0;

/// The exposure factor for the sun's `hour` without the live sun: 1 by day,
/// [`NIGHT_FACTOR`] at night, eased (cosine) through dusk and dawn.
pub(crate) fn night_tint(hour: f32) -> f32 {
    let hour = if hour.is_finite() {
        hour.rem_euclid(24.)
    } else {
        12.
    };
    let ease = |from: f32, to: f32| {
        let t = ((hour - from) / (to - from)).clamp(0., 1.);
        0.5 - 0.5 * (t * std::f32::consts::PI).cos()
    };
    let night = if hour >= DUSK.0 {
        ease(DUSK.0, DUSK.1)
    } else if hour < DAWN.1 {
        1. - ease(DAWN.0, DAWN.1)
    } else {
        0.
    };
    1. - night * (1. - NIGHT_FACTOR)
}

/// The PC clock's last hour read and when the next read is due (milliseconds since the
/// first read), shared by every copy of a [`Clock`].
#[derive(Default)]
struct PcClock {
    hour: AtomicU32,
    next_ms: AtomicU64,
}

impl PcClock {
    /// The PC clock's hour, read again at most every [`PC_CLOCK_EVERY_MS`].
    fn hour(&self) -> f32 {
        static START: OnceLock<Instant> = OnceLock::new();
        let now_ms = START.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1;
        if now_ms >= self.next_ms.load(Ordering::Relaxed) {
            self.next_ms
                .store(now_ms + PC_CLOCK_EVERY_MS, Ordering::Relaxed);
            self.hour
                .store(pc_hour(&LocalTime::now()).to_bits(), Ordering::Relaxed);
        }
        f32::from_bits(self.hour.load(Ordering::Relaxed))
    }
}

/// Allocation-free presentation controls shared by console callbacks and render upload:
/// solar hour, minutes per day, volumetric clarity, real-time light scale, the
/// diagnostics bitmask, contact shadows, gap closure, ambient readability fill and
/// indirect-light controls and whether the hour follows the PC's clock.
#[derive(Clone)]
pub(crate) struct Clock(Arc<[AtomicU32; 12]>, Arc<PcClock>);

/// Names in slot order; every slot is a live cvar registered by `day::register`.
pub(crate) const LIVE_CVARS: [&str; 12] = [
    "r_dayHour",
    "r_dayMinutes",
    "r_volumetricClarity",
    "r_dayBrightness",
    "r_dayDebug",
    "r_contactShadows",
    "r_sunShadowGapClose",
    "r_ambientFill",
    "r_indirectBoost",
    "r_ambientFillOcclusion",
    PC_CLOCK_CVAR,
    SKY_CVAR,
];

impl Default for Clock {
    fn default() -> Self {
        Self(
            Arc::new([
                AtomicU32::new(12_f32.to_bits()),
                AtomicU32::new(0),
                AtomicU32::new(1_f32.to_bits()),
                AtomicU32::new(1_f32.to_bits()),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0.025_f32.to_bits()),
                AtomicU32::new(1_f32.to_bits()),
                AtomicU32::new(1_f32.to_bits()),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ]),
            Arc::default(),
        )
    }
}

impl Clock {
    /// Seed archived values before subscribing; no callback is needed for the initial state.
    pub(crate) fn bind(cvars: &mut CvarRegistry) -> Result<Self, CvarError> {
        if cvars.get("r_dayNight").is_none() {
            super::register(cvars)?;
        }
        let clock = Self::default();
        for (index, name) in LIVE_CVARS.into_iter().enumerate() {
            if let Some(cvar) = cvars.get(name) {
                clock.store(index, &cvar.value);
            }
            let changed = clock.clone();
            cvars.on_change(name, move |change| changed.store(index, &change.current))?;
        }
        Ok(clock)
    }

    fn store(&self, index: usize, value: &CvarValue) {
        let value = match value {
            CvarValue::Float(v) => *v as f32,
            CvarValue::Integer(v) => *v as f32,
            _ => return,
        };
        if !value.is_finite() {
            return;
        }
        let value = match index {
            0 => value.rem_euclid(24.),
            1 => value.clamp(0., 1440.),
            2 => value.clamp(0., 1.),
            3 => value.clamp(0.1, 10.),
            6 => value.clamp(0., 64.),
            7 => value.clamp(0., 0.2),
            8 => value.clamp(0., 4.),
            9 => value.clamp(0., 1.),
            10 | 11 => f32::from(u8::from(value != 0.)),
            _ => value.clamp(0., 4095.),
        };
        self.0[index].store(value.to_bits(), Ordering::Relaxed);
    }

    /// Read the bounded scalar controls without a registry lookup, allocation or lock.
    /// With [`PC_CLOCK_CVAR`] on, the hour is the PC's local time and the day holds
    /// (no minutes per day): the clock itself moves it.
    pub(crate) fn values(&self) -> [f32; 4] {
        let mut values = [0, 1, 2, 3].map(|i| f32::from_bits(self.0[i].load(Ordering::Relaxed)));
        if f32::from_bits(self.0[10].load(Ordering::Relaxed)) != 0. {
            values[0] = self.1.hour();
            values[1] = 0.;
        }
        values
    }

    /// What the sky alone follows without the live sun (hour, day length): the PC's hour
    /// while [`PC_CLOCK_CVAR`] is on, `r_dayHour` and `r_dayMinutes` with [`SKY_CVAR`],
    /// else noon, where the sky's tint leaves the authored sky as it is; so switching both
    /// off gives the map its own sky back at once, whatever `r_dayHour` says.
    pub(crate) fn sky_values(&self) -> [f32; 2] {
        if self.on(10) {
            [self.1.hour(), 0.]
        } else if self.on(11) {
            [0, 1].map(|i| f32::from_bits(self.0[i].load(Ordering::Relaxed)))
        } else {
            [12., 0.]
        }
    }

    fn on(&self, slot: usize) -> bool {
        f32::from_bits(self.0[slot].load(Ordering::Relaxed)) != 0.
    }

    /// `r_sunShadowGapClose`: holes in the shadow maps narrower than this many world units
    /// are closed.
    pub(crate) fn gap_close(&self) -> f32 {
        f32::from_bits(self.0[6].load(Ordering::Relaxed))
    }

    /// Live readability fill; zero preserves fully unlit rooms.
    pub(crate) fn ambient_fill(&self) -> f32 {
        f32::from_bits(self.0[7].load(Ordering::Relaxed))
    }

    /// Indirect-light gain and the fraction of AO applied to readability fill.
    pub(crate) fn indirect_readability(&self) -> [f32; 2] {
        [8, 9].map(|i| f32::from_bits(self.0[i].load(Ordering::Relaxed)))
    }

    /// The exposure factor of the PC clock's night ([`night_tint`]) while
    /// [`PC_CLOCK_CVAR`] is on, else 1: what stands for the sun's hour when the live sun
    /// is off.
    pub(crate) fn night_tint(&self) -> f32 {
        if self.on(10) {
            night_tint(self.1.hour())
        } else if self.on(11) {
            // A turning day's hour is the sky shader's; the set hour stands for it here.
            night_tint(f32::from_bits(self.0[0].load(Ordering::Relaxed)))
        } else {
            1.
        }
    }

    /// `r_dayDebug` plus the contact shadow toggle folded into bit 2: which real-time
    /// terms to leave out (see `day::register`).
    pub(crate) fn debug(&self) -> u32 {
        let bits = f32::from_bits(self.0[4].load(Ordering::Relaxed)) as u32;
        let contact = f32::from_bits(self.0[5].load(Ordering::Relaxed)) != 0.;
        bits | if contact { 0 } else { 2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u8, minute: u8, second: u8) -> LocalTime {
        LocalTime {
            year: 2026,
            month: 10,
            day: 29,
            weekday: 4,
            hour,
            minute,
            second,
        }
    }

    /// The PC's local time as the sun's hour: minutes and seconds as fractions.
    #[test]
    fn the_pc_clock_gives_the_hour_of_the_sun() {
        assert_eq!(pc_hour(&at(0, 0, 0)), 0.);
        assert_eq!(pc_hour(&at(12, 0, 0)), 12.);
        assert_eq!(pc_hour(&at(19, 30, 0)), 19.5);
        assert!((pc_hour(&at(6, 45, 36)) - 6.76).abs() < 1e-4);
        assert!(pc_hour(&at(23, 59, 59)) < 24.);
    }

    /// Without the live sun: day untouched, night darker, dusk and dawn in between and
    /// never brighter than day.
    #[test]
    fn the_night_tint_darkens_the_night_only() {
        for hour in [8., 12., 17.5] {
            assert_eq!(night_tint(hour), 1., "{hour}");
        }
        for hour in [NIGHT_DARK_FROM, 22., 0., 3., NIGHT_DARK_UNTIL] {
            assert!((night_tint(hour) - NIGHT_FACTOR).abs() < 1e-5, "{hour}");
        }
        let dusk = night_tint(19.25);
        let dawn = night_tint(6.);
        assert!(NIGHT_FACTOR < dusk && dusk < 1., "{dusk}");
        assert!(NIGHT_FACTOR < dawn && dawn < 1., "{dawn}");
        assert!(night_tint(18.5) > night_tint(20.));
        assert_eq!(night_tint(f32::NAN), 1.);
        // The switch off leaves the exposure alone.
        let clock = Clock::default();
        assert_eq!(clock.night_tint(), 1.);
    }

    /// Off, `r_dayHour` and `r_dayMinutes` rule; on, the hour is the PC's and the day
    /// holds; off again, the set hour is back.
    #[test]
    fn the_pc_clock_switch_takes_the_hour_and_gives_it_back() {
        let clock = Clock::default();
        clock.store(0, &CvarValue::Float(7.5));
        clock.store(1, &CvarValue::Float(30.));
        assert_eq!(clock.values()[..2], [7.5, 30.]);
        clock.store(10, &CvarValue::Integer(1));
        let [hour, minutes, ..] = clock.values();
        assert!((0. ..24.).contains(&hour));
        assert_eq!(minutes, 0.);
        clock.store(10, &CvarValue::Integer(0));
        assert_eq!(clock.values()[..2], [7.5, 30.]);
    }

    #[test]
    fn without_the_pc_clock_the_sky_keeps_its_own_look() {
        let clock = Clock::default();
        clock.store(0, &CvarValue::Float(21.));
        assert_eq!(clock.sky_values(), [12., 0.], "noon: the authored sky");
        assert_eq!(clock.night_tint(), 1.);
        // The time-of-day sky shows the set hour, and its night darkens the view.
        clock.store(11, &CvarValue::Integer(1));
        clock.store(1, &CvarValue::Float(20.));
        assert_eq!(clock.sky_values(), [21., 20.]);
        assert!(clock.night_tint() < 1.);
        clock.store(11, &CvarValue::Integer(0));
        clock.store(10, &CvarValue::Integer(1));
        let [hour, minutes] = clock.sky_values();
        assert!((0. ..24.).contains(&hour));
        assert_eq!(minutes, 0.);
    }
}
