//! Lightning in storms (`r_weatherLightning`, off by default as every change to stock
//! Jedi Academy's look): now and then a strike flickers through the storm, two or three
//! quick pulses lighting the rain's haze and ground fog, the clouds, the falling rain and
//! the wet ground and puddles; its thunder follows after the time the sound takes to
//! come (`s_weatherSounds`), louder and sharper when it struck near. With
//! `cg_reduceFlashes` the flash is a third as bright and never sudden.
//!
//! Only a storm flashes: weather whose haze is at least [`STORM`] of a downpour's
//! (`heavyrain`, the Storm preset and `r_weatherForce 3`). The multiplayer game has no
//! lightning of its own (single player's `fx_rain` flag is ignored there), so nothing on a
//! server or in a map asks for it: the player turns it on.
//!
//! Also here: the wind's gusts heard on their own ([`Gusts`]): when a gust rises past
//! [`GUST`] of a gale, one of the game's gust sounds (`windy_a` to `_d`) plays.

/// The setting: lightning in storms (off by default).
pub(crate) const CVAR: &str = "r_weatherLightning";
/// How stormy the weather must be (its haze against a downpour's) for lightning.
pub(crate) const STORM: f32 = 0.75;
/// Seconds between strikes: at least this, and up to this much more.
const INTERVAL: f32 = 7.0;
const INTERVAL_SPREAD: f32 = 15.0;
/// How long a pulse's light takes to fall to a third (seconds).
const PULSE_DECAY: f32 = 0.07;
/// A strike lasts this long, its last pulse included.
const STRIKE_LENGTH: f32 = 0.9;
/// The thunder's delay: this for a strike overhead, and this much more at the farthest.
const THUNDER_NEAR: f32 = 0.25;
const THUNDER_FAR: f32 = 3.8;
/// Under `cg_reduceFlashes`: this share of the light, rising over [`REDUCED_RISE`] seconds.
const REDUCED: f32 = 0.33;
const REDUCED_RISE: f32 = 0.15;

/// The game's thunder (`assets0.pk3`): near strikes' and far ones'.
pub(crate) const THUNDER_CLOSE: [&str; 2] = [
    "sound/ambience/thunder_close1.mp3",
    "sound/ambience/thunder_close2.mp3",
];
pub(crate) const THUNDER_DISTANT: [&str; 4] = [
    "sound/ambience/thunder1.mp3",
    "sound/ambience/thunder2.wav",
    "sound/ambience/thunder3.wav",
    "sound/ambience/thunder4.mp3",
];
/// The game's single gusts.
pub(crate) const GUSTS: [&str; 4] = [
    "sound/ambience/outdoor/windy_a.mp3",
    "sound/ambience/outdoor/windy_b.mp3",
    "sound/ambience/outdoor/windy_c.mp3",
    "sound/ambience/outdoor/windy_d.mp3",
];

/// A sound the weather asks for this frame, in both ears.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cue {
    pub(crate) path: &'static str,
    /// 0..1, before the effects volume and the shelter.
    pub(crate) volume: f32,
}

/// One pulse of a strike: when it starts after the strike and how bright it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Pulse {
    start: f32,
    strength: f32,
}

/// The storm's lightning: the time to the next strike, the strike under way and the
/// thunder on its way.
#[derive(Clone, Debug)]
pub(crate) struct Lightning {
    dice: u32,
    until: f32,
    /// Seconds since the strike under way began, and its pulses.
    age: Option<f32>,
    pulses: [Pulse; 3],
    /// The thunder of the last strike: seconds until it is heard, and its sound.
    thunder: Option<(f32, Cue)>,
    /// The flash this frame, 0..1, and the light it gives so far under reduced flashes.
    flash: f32,
    eased: f32,
}

impl Default for Lightning {
    fn default() -> Self {
        Self {
            dice: 0x9E37_79B9,
            // The first strike comes a little after the storm starts.
            until: 4.0,
            age: None,
            pulses: [Pulse::default(); 3],
            thunder: None,
            flash: 0.0,
            eased: 0.0,
        }
    }
}

impl Lightning {
    /// A number in 0..1 from the dice (xorshift).
    fn roll(&mut self) -> f32 {
        let mut x = self.dice;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.dice = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Strike in `seconds` (0: at the next advance), whatever strike is under way
    /// (world shots).
    #[cfg(test)]
    pub(crate) fn strike_in(&mut self, seconds: f32) {
        self.until = seconds;
        self.age = None;
    }

    /// The flash now, 0..1.
    pub(crate) fn flash(&self) -> f32 {
        self.flash
    }

    /// Move on `seconds` in weather `storm` (0..1) with lightning `enabled`; `reduced`:
    /// `cg_reduceFlashes`. Answers the thunder to play now, if any.
    pub(crate) fn advance(
        &mut self,
        seconds: f32,
        storm: f32,
        enabled: bool,
        reduced: bool,
    ) -> Option<Cue> {
        let mut heard = None;
        if let Some((delay, cue)) = &mut self.thunder {
            *delay -= seconds;
            if *delay <= 0.0 {
                heard = Some(*cue);
                self.thunder = None;
            }
        }
        if !enabled || storm < STORM {
            self.age = None;
            self.flash = 0.0;
            self.eased = 0.0;
            self.until = self.until.max(2.0);
            return heard;
        }
        self.until -= seconds;
        // A strike shows its first pulse whole in the frame it begins, however long the
        // frame was (at a low frame rate a pulse would otherwise fade unseen).
        let begun = self.age.is_none() && self.until <= 0.0;
        if begun {
            self.strike(storm);
        }
        let mut light = 0.0_f32;
        if let Some(age) = &mut self.age {
            if !begun {
                *age += seconds;
            }
            let now = *age;
            for pulse in self.pulses {
                if pulse.strength > 0.0 && now >= pulse.start {
                    light = light.max(pulse.strength * (-(now - pulse.start) / PULSE_DECAY).exp());
                }
            }
            if now > STRIKE_LENGTH {
                self.age = None;
            }
        }
        self.flash = if reduced {
            // Never sudden: the light eases in and out, a third as bright.
            let step = if seconds <= 0.0 {
                1.0
            } else {
                1.0 - (-seconds / REDUCED_RISE).exp()
            };
            self.eased += (light * REDUCED - self.eased) * step;
            self.eased
        } else {
            light
        };
        heard
    }

    /// Start a strike: two or three pulses, brighter when it falls near, its thunder on
    /// its way, and the next strike's time.
    fn strike(&mut self, storm: f32) {
        let far = self.roll();
        let near = 1.0 - 0.6 * far;
        let count = 2 + usize::from(self.roll() < 0.5);
        let mut start = 0.0;
        for index in 0..3 {
            self.pulses[index] = if index < count {
                let pulse = Pulse {
                    start,
                    strength: near * (0.55 + 0.45 * self.roll()),
                };
                start += 0.07 + 0.16 * self.roll();
                pulse
            } else {
                Pulse::default()
            };
        }
        self.age = Some(0.0);
        let close = far < 0.25;
        let path = if close {
            THUNDER_CLOSE[(self.roll() * 2.0) as usize % 2]
        } else {
            THUNDER_DISTANT[(self.roll() * 4.0) as usize % 4]
        };
        self.thunder = Some((
            THUNDER_NEAR + THUNDER_FAR * far,
            Cue {
                path,
                volume: 0.9 - 0.45 * far,
            },
        ));
        // A fiercer storm strikes more often.
        let fierce = ((storm - STORM) / (1.0 - STORM)).clamp(0.0, 1.0);
        self.until = INTERVAL + INTERVAL_SPREAD * self.roll() * (1.0 - 0.4 * fierce);
    }
}

/// The wind's gusts as single sounds: one plays when the wind rises past [`GUST`] of a
/// gale, at most every [`GUST_REST`] seconds, as loud as the gust is strong.
#[derive(Clone, Debug, Default)]
pub(crate) struct Gusts {
    /// The wind level last frame, and whether it was above the gust's level.
    level: f32,
    up: bool,
    rest: f32,
    dice: u32,
}

/// The wind level (0..1 of a gale) a gust is heard at, and the level it must fall back
/// below before the next.
pub(crate) const GUST: f32 = 0.55;
const GUST_CALM: f32 = 0.4;
/// Seconds at least between two gusts.
const GUST_REST: f32 = 4.0;

impl Gusts {
    /// The wind is `level` (0..1 of a gale) after `seconds`: a gust's sound, when one
    /// rises now.
    pub(crate) fn advance(&mut self, level: f32, seconds: f32) -> Option<Cue> {
        self.rest = (self.rest - seconds).max(0.0);
        let rising = level > self.level;
        self.level = level;
        if self.up {
            if level < GUST_CALM {
                self.up = false;
            }
            return None;
        }
        if level < GUST || !rising || self.rest > 0.0 {
            return None;
        }
        self.up = true;
        self.rest = GUST_REST;
        self.dice = self
            .dice
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add(0x7F4A_7C15);
        Some(Cue {
            path: GUSTS[(self.dice >> 30) as usize],
            volume: 0.35 + 0.45 * ((level - GUST) / (1.0 - GUST)).clamp(0.0, 1.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `lightning` for `seconds` in 1/60 s steps; answer the brightest flash and the
    /// thunders heard.
    fn run(lightning: &mut Lightning, seconds: f32, storm: f32, reduced: bool) -> (f32, Vec<Cue>) {
        let mut brightest = 0.0_f32;
        let mut heard = Vec::new();
        for _ in 0..(seconds * 60.0) as usize {
            heard.extend(lightning.advance(1.0 / 60.0, storm, true, reduced));
            brightest = brightest.max(lightning.flash());
        }
        (brightest, heard)
    }

    #[test]
    fn storms_strike_now_and_then_and_their_thunder_follows() {
        let mut lightning = Lightning::default();
        let (brightest, heard) = run(&mut lightning, 120.0, 1.0, false);
        assert!(brightest > 0.3, "{brightest}");
        // Every 7 to 22 seconds, the first after 4: five to sixteen in two minutes.
        assert!((5..=17).contains(&heard.len()), "{}", heard.len());
        assert!(heard.iter().all(|cue| THUNDER_CLOSE.contains(&cue.path)
            || THUNDER_DISTANT.contains(&cue.path)));
        assert!(heard.iter().all(|cue| (0.4..=0.9).contains(&cue.volume)));
    }

    #[test]
    fn rain_short_of_a_storm_and_the_setting_off_stay_dark() {
        let mut lightning = Lightning::default();
        assert_eq!(run(&mut lightning, 60.0, 0.57, false), (0.0, Vec::new()));
        let mut off = Lightning::default();
        for _ in 0..3600 {
            assert!(off.advance(1.0 / 60.0, 1.0, false, false).is_none());
            assert_eq!(off.flash(), 0.0);
        }
    }

    #[test]
    fn a_strike_flickers_in_pulses_and_is_over_within_a_second() {
        let mut lightning = Lightning {
            until: 0.0,
            ..Lightning::default()
        };
        let mut lit = Vec::new();
        for _ in 0..90 {
            lightning.advance(1.0 / 60.0, 1.0, true, false);
            lit.push(lightning.flash());
        }
        // The light falls between pulses and rises again: at least two peaks.
        let peaks = lit
            .windows(3)
            .filter(|w| w[1] > w[0] && w[1] >= w[2] && w[1] > 0.1)
            .count();
        assert!(peaks >= 1, "{lit:?}");
        assert!(lit[0] > 0.3, "the first pulse is whole in its own frame");
        // At a low frame rate too: the frame a strike begins shows it at full strength.
        let mut slow = Lightning {
            until: 0.0,
            ..Lightning::default()
        };
        slow.advance(0.25, 1.0, true, false);
        assert!(slow.flash() > 0.3, "{}", slow.flash());
        assert!(*lit.last().unwrap() < 0.01, "over within 1.5 s");
    }

    #[test]
    fn reduced_flashes_are_dimmer_and_never_sudden() {
        let mut lightning = Lightning {
            until: 0.0,
            ..Lightning::default()
        };
        let mut last = 0.0_f32;
        let mut brightest = 0.0_f32;
        for _ in 0..90 {
            lightning.advance(1.0 / 60.0, 1.0, true, true);
            let flash = lightning.flash();
            assert!((flash - last).abs() < 0.12, "{last} -> {flash}");
            last = flash;
            brightest = brightest.max(flash);
        }
        assert!(brightest > 0.05 && brightest <= REDUCED, "{brightest}");
    }

    #[test]
    fn gusts_are_heard_as_they_rise_and_rest_between() {
        let mut gusts = Gusts::default();
        assert!(gusts.advance(0.3, 0.1).is_none());
        let first = gusts
            .advance(0.7, 0.1)
            .expect("a gust rising past the level");
        assert!(GUSTS.contains(&first.path) && first.volume > 0.35);
        // Staying up, or rising again too soon after falling: no second one.
        assert!(gusts.advance(0.9, 0.1).is_none());
        assert!(gusts.advance(0.3, 0.1).is_none());
        assert!(gusts.advance(0.7, 0.1).is_none(), "resting");
        assert!(gusts.advance(0.3, 4.0).is_none());
        assert!(gusts.advance(0.7, 0.1).is_some());
    }
}
