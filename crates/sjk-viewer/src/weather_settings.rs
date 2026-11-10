//! The weather and cloud settings, shared by the console and every installed world.

use sjk_shell::{CvarDefinition, CvarError, CvarFlags, CvarRegistry, CvarValue};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Weather on (1) or off (0).
pub(crate) const CVAR: &str = "r_weather";
/// Particle count scale.
pub(crate) const DENSITY_CVAR: &str = "r_weatherDensity";
/// How much is drawn: 0 low to 3 ultra ([`Quality`]).
pub(crate) const QUALITY_CVAR: &str = "r_weatherQuality";
/// Weather chosen by the player instead of the map's ([`forced_commands`]).
pub(crate) const FORCE_CVAR: &str = "r_weatherForce";
/// Ground fog: 0 none, 1 the map's, 2 on every map with sky.
pub(crate) const FOG_CVAR: &str = "r_weatherFog";
/// Volumetric clouds over open sky.
pub(crate) const CLOUDS_CVAR: &str = "r_clouds";
/// Lightning in storms (`weather_lightning.rs`).
pub(crate) const LIGHTNING_CVAR: &str = super::lightning::CVAR;

/// What wet surfaces mirror ([`Reflections`]): 0 by the quality, 1 the sky alone, 2 the
/// world in puddles, 3 the world in every wet surface.
pub(crate) const REFLECTIONS_CVAR: &str = "r_weatherReflections";
/// How much standing water gathers on flat ground, 0 none to 2 flooded (1 as designed).
pub(crate) const PUDDLES_CVAR: &str = "r_weatherPuddles";
/// The ground fog's thickness, 1 as designed.
pub(crate) const FOG_DENSITY_CVAR: &str = "r_weatherFogDensity";
/// The haze falling weather leaves in the air and the spray it raises, 1 as designed.
pub(crate) const HAZE_CVAR: &str = "r_weatherHaze";
/// How visible the rain's streaks are, 1 as designed.
pub(crate) const RAIN_OPACITY_CVAR: &str = "r_weatherRainOpacity";
/// Snow settles on what it falls on.
pub(crate) const SNOW_COVER_CVAR: &str = "r_weatherSnowCover";

/// SJK's default density: twice the reference's particles.
const DEFAULT_DENSITY: f32 = 2.0;
const DEFAULT_QUALITY: u32 = 2;

const ENABLED: usize = 0;
const DENSITY: usize = 1;
const QUALITY: usize = 2;
const FORCE: usize = 3;
const FOG: usize = 4;
const CLOUDS: usize = 5;
const LIGHTNING: usize = 6;
const REFLECTIONS: usize = 7;
const SNOW_COVER: usize = 8;
const PUDDLES: usize = 9;
const FOG_DENSITY: usize = 10;
const HAZE: usize = 11;
const RAIN_OPACITY: usize = 12;
const SLOTS: usize = 13;

/// The sliders: cvar, slot, default, lowest, highest, what it says.
const SLIDERS: [(&str, usize, f32, f32, f32, &str); 5] = [
    (
        DENSITY_CVAR,
        DENSITY,
        DEFAULT_DENSITY,
        0.25,
        4.0,
        "Weather particle count, 1 as the original game, 0.25 to 4",
    ),
    (
        PUDDLES_CVAR,
        PUDDLES,
        1.0,
        0.0,
        2.0,
        "Standing water on flat ground in rain (ultra): 0 none, 1 some, 2 flooded",
    ),
    (
        FOG_DENSITY_CVAR,
        FOG_DENSITY,
        1.0,
        0.25,
        3.0,
        "Ground fog thickness, 0.25 to 3 (1 as designed)",
    ),
    (
        HAZE_CVAR,
        HAZE,
        1.0,
        0.0,
        2.0,
        "Haze in falling rain and snow and the spray over the ground, 0 to 2 (1 as designed)",
    ),
    (
        RAIN_OPACITY_CVAR,
        RAIN_OPACITY,
        1.0,
        0.25,
        2.0,
        "How visible the rain's streaks are, 0.25 to 2 (1 as designed)",
    ),
];

/// What wet surfaces mirror.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Reflections {
    /// The overcast sky's colour alone: no copy of the scene.
    Sky,
    /// Puddles mirror the world round them.
    Puddles,
    /// Wet ground, wet walls and running water mirror it too.
    Everything,
}

/// The highest `r_weatherForce`.
pub(crate) const FORCE_MAX: i64 = 4;

/// What a quality level draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quality {
    /// Rain splashes on the ground and water.
    pub(crate) splashes: bool,
    /// Splashes per near rain streak.
    pub(crate) splash_share: f32,
    /// Far rain streaks per near one; 0 draws no far layer.
    pub(crate) far_share: f32,
    /// Samples along each pixel's ray through the rain haze and ground fog; 0 keeps the
    /// original's drifting fog sprites instead.
    pub(crate) fog_steps: u32,
    /// Samples through the cloud layer.
    pub(crate) cloud_steps: u32,
    /// What rain does to the surfaces it falls on: 0 nothing, 1 they are wet, 2 water
    /// also runs down slopes and walls, 3 flat ground also gathers puddles.
    pub(crate) wet: u32,
}

impl Quality {
    /// Level 0 (low) to 3 (ultra).
    pub(crate) fn level(level: u32) -> Self {
        match level {
            0 => Self {
                splashes: false,
                splash_share: 0.0,
                far_share: 0.0,
                fog_steps: 0,
                cloud_steps: 10,
                wet: 0,
            },
            1 => Self {
                splashes: true,
                splash_share: 0.4,
                far_share: 0.0,
                fog_steps: 10,
                cloud_steps: 16,
                wet: 1,
            },
            2 => Self {
                splashes: true,
                splash_share: 0.6,
                far_share: 1.5,
                fog_steps: 16,
                cloud_steps: 24,
                wet: 2,
            },
            _ => Self {
                splashes: true,
                splash_share: 0.9,
                far_share: 2.5,
                fog_steps: 24,
                cloud_steps: 40,
                wet: 3,
            },
        }
    }
}

/// The world effect commands `r_weatherForce` stands for; empty for the map's own.
pub(crate) fn forced_commands(force: u32) -> &'static [&'static str] {
    match force {
        1 => &["lightrain"],
        2 => &["rain", "wind"],
        3 => &["heavyrain", "gustingwind"],
        4 => &["snow", "wind"],
        _ => &[],
    }
}

/// Live values; every read is one relaxed load.
#[derive(Clone)]
pub(crate) struct Settings(Arc<[AtomicU32; SLOTS]>);

impl Default for Settings {
    fn default() -> Self {
        let one = 1.0_f32.to_bits();
        Self(Arc::new([
            AtomicU32::new(1),
            AtomicU32::new(DEFAULT_DENSITY.to_bits()),
            AtomicU32::new(DEFAULT_QUALITY),
            AtomicU32::new(0),
            AtomicU32::new(1),
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(one),
            AtomicU32::new(one),
            AtomicU32::new(one),
            AtomicU32::new(one),
        ]))
    }
}

impl Settings {
    /// Register the archived cvars and follow their changes.
    pub(crate) fn bind(cvars: &mut CvarRegistry) -> Result<Self, CvarError> {
        let settings = Self::default();
        let integers: [(&str, i64, i64, i64, usize, &str); 8] = [
            (
                CVAR,
                1,
                0,
                1,
                ENABLED,
                "Rain, snow and mist on maps that have them",
            ),
            (
                QUALITY_CVAR,
                i64::from(DEFAULT_QUALITY),
                0,
                3,
                QUALITY,
                "Weather quality: 0 low, 1 medium, 2 high, 3 ultra",
            ),
            (
                FORCE_CVAR,
                0,
                0,
                FORCE_MAX,
                FORCE,
                "Weather on every map: 0 the map's, 1 drizzle, 2 rain, 3 storm, 4 snow",
            ),
            (
                FOG_CVAR,
                1,
                0,
                2,
                FOG,
                "Ground fog: 0 none, 1 the map's, 2 on every map with sky",
            ),
            (
                CLOUDS_CVAR,
                0,
                0,
                1,
                CLOUDS,
                "Volumetric clouds over open sky (off by default, under every preset)",
            ),
            (
                LIGHTNING_CVAR,
                0,
                0,
                1,
                LIGHTNING,
                "Lightning in storms: flashes through the rain and fog, thunder with s_weatherSounds",
            ),
            (
                REFLECTIONS_CVAR,
                0,
                0,
                3,
                REFLECTIONS,
                "What wet surfaces mirror: 0 by the weather quality, 1 the sky, 2 the world in puddles, 3 in every wet surface",
            ),
            (
                SNOW_COVER_CVAR,
                0,
                0,
                1,
                SNOW_COVER,
                "Snow settles on the ground and ledges it falls on",
            ),
        ];
        for (name, default, low, high, slot, help) in integers {
            cvars.register(CvarDefinition::new(name, default, CvarFlags::ARCHIVE, help))?;
            if let Some(cvar) = cvars.get(name) {
                settings.set_integer(slot, &cvar.value, low, high);
            }
            let changed = settings.clone();
            cvars.on_change(name, move |change| {
                changed.set_integer(slot, &change.current, low, high)
            })?;
        }
        for (name, slot, default, low, high, help) in SLIDERS {
            cvars.register(CvarDefinition::new(
                name,
                f64::from(default),
                CvarFlags::ARCHIVE,
                help,
            ))?;
            if let Some(cvar) = cvars.get(name) {
                settings.set_float(slot, &cvar.value, low, high);
            }
            let changed = settings.clone();
            cvars.on_change(name, move |change| {
                changed.set_float(slot, &change.current, low, high)
            })?;
        }
        Ok(settings)
    }

    fn set_integer(&self, slot: usize, value: &CvarValue, low: i64, high: i64) {
        let value = match value {
            CvarValue::Bool(value) => i64::from(*value),
            CvarValue::Integer(value) => *value,
            CvarValue::Float(value) if value.is_finite() => *value as i64,
            _ => return,
        };
        self.0[slot].store(value.clamp(low, high) as u32, Ordering::Relaxed);
    }

    fn set_float(&self, slot: usize, value: &CvarValue, low: f32, high: f32) {
        let value = match value {
            CvarValue::Float(value) => *value,
            CvarValue::Integer(value) => *value as f64,
            _ => return,
        };
        if value.is_finite() {
            let value = (value as f32).clamp(low, high);
            self.0[slot].store(value.to_bits(), Ordering::Relaxed);
        }
    }

    fn float(&self, slot: usize) -> f32 {
        f32::from_bits(self.0[slot].load(Ordering::Relaxed))
    }

    pub(crate) fn enabled(&self) -> bool {
        self.0[ENABLED].load(Ordering::Relaxed) != 0
    }

    pub(crate) fn density(&self) -> f32 {
        self.float(DENSITY)
    }

    /// `r_weatherPuddles`: 0 none, 1 as designed, 2 flooded.
    pub(crate) fn puddles(&self) -> f32 {
        self.float(PUDDLES)
    }

    /// `r_weatherFogDensity`: the ground fog's thickness, times its own.
    pub(crate) fn fog_density(&self) -> f32 {
        self.float(FOG_DENSITY)
    }

    /// `r_weatherHaze`: the weather's haze and spray, times their own.
    pub(crate) fn haze(&self) -> f32 {
        self.float(HAZE)
    }

    /// `r_weatherRainOpacity`: the rain streaks' opacity, times their own.
    pub(crate) fn rain_opacity(&self) -> f32 {
        self.float(RAIN_OPACITY)
    }

    /// `r_weatherSnowCover`.
    pub(crate) fn snow_cover(&self) -> bool {
        self.0[SNOW_COVER].load(Ordering::Relaxed) != 0
    }

    /// What wet surfaces mirror: `r_weatherReflections`, or by the quality's `wet` level
    /// (ultra: everything) when it is 0.
    pub(crate) fn reflections(&self) -> Reflections {
        match self.0[REFLECTIONS].load(Ordering::Relaxed) {
            0 if self.quality().wet >= 3 => Reflections::Everything,
            0 | 1 => Reflections::Sky,
            2 => Reflections::Puddles,
            _ => Reflections::Everything,
        }
    }

    pub(crate) fn quality(&self) -> Quality {
        Quality::level(self.0[QUALITY].load(Ordering::Relaxed))
    }

    /// `r_weatherForce`, 0 for the map's own weather.
    pub(crate) fn force(&self) -> u32 {
        self.0[FORCE].load(Ordering::Relaxed)
    }

    /// `r_weatherFog`: 0 none, 1 the map's, 2 always.
    pub(crate) fn fog(&self) -> u32 {
        self.0[FOG].load(Ordering::Relaxed)
    }

    pub(crate) fn clouds(&self) -> bool {
        self.0[CLOUDS].load(Ordering::Relaxed) != 0
    }

    pub(crate) fn lightning(&self) -> bool {
        self.0[LIGHTNING].load(Ordering::Relaxed) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_clamped_and_follow_the_console() {
        let mut cvars = CvarRegistry::default();
        let settings = Settings::bind(&mut cvars).unwrap();
        assert!(
            settings.enabled() && !settings.clouds(),
            "clouds are off by default"
        );
        assert_eq!(settings.quality(), Quality::level(2));
        assert_eq!((settings.force(), settings.fog()), (0, 1));
        settings.set_integer(QUALITY, &CvarValue::Integer(9), 0, 3);
        assert_eq!(settings.quality(), Quality::level(3));
        settings.set_integer(FORCE, &CvarValue::Integer(-2), 0, FORCE_MAX);
        assert_eq!(settings.force(), 0);
        settings.set_float(DENSITY, &CvarValue::Float(100.0), 0.25, 4.0);
        assert_eq!(settings.density(), 4.0);
    }

    #[test]
    fn the_looks_start_as_designed_and_reflections_follow_the_quality() {
        let mut cvars = CvarRegistry::default();
        let settings = Settings::bind(&mut cvars).unwrap();
        assert_eq!(
            [
                settings.puddles(),
                settings.fog_density(),
                settings.haze(),
                settings.rain_opacity()
            ],
            [1.0; 4]
        );
        assert!(!settings.snow_cover(), "stock ground by default");
        // High keeps the sky alone; ultra mirrors the world unless told otherwise.
        assert_eq!(settings.reflections(), Reflections::Sky);
        settings.set_integer(QUALITY, &CvarValue::Integer(3), 0, 3);
        assert_eq!(settings.reflections(), Reflections::Everything);
        settings.set_integer(REFLECTIONS, &CvarValue::Integer(2), 0, 3);
        assert_eq!(settings.reflections(), Reflections::Puddles);
        settings.set_integer(REFLECTIONS, &CvarValue::Integer(1), 0, 3);
        assert_eq!(settings.reflections(), Reflections::Sky);
        settings.set_integer(QUALITY, &CvarValue::Integer(1), 0, 3);
        settings.set_integer(REFLECTIONS, &CvarValue::Integer(3), 0, 3);
        assert_eq!(settings.reflections(), Reflections::Everything);
        settings.set_float(HAZE, &CvarValue::Float(-3.0), 0.0, 2.0);
        assert_eq!(settings.haze(), 0.0);
    }

    #[test]
    fn quality_adds_features_level_by_level() {
        let levels = [0, 1, 2, 3].map(Quality::level);
        assert!(!levels[0].splashes && levels[0].fog_steps == 0);
        for pair in levels.windows(2) {
            assert!(pair[1].cloud_steps > pair[0].cloud_steps);
            assert!(pair[1].fog_steps >= pair[0].fog_steps);
            assert!(pair[1].far_share >= pair[0].far_share);
            assert!(pair[1].wet > pair[0].wet);
        }
        assert!(forced_commands(0).is_empty());
        for force in 1..=FORCE_MAX as u32 {
            let effects = super::super::effects::Effects::from_commands(
                forced_commands(force).iter().copied(),
            );
            assert_eq!(effects.clouds.len(), 1, "force {force}");
        }
    }
}
