//! The toys' sound settings (Settings > Sound > Toys): one volume over every toy's
//! sounds, times the effects volume, and a switch per toy that silences it on this PC
//! (its sounds alone: the toy still runs and is still seen).

use crate::console::ViewerConsole;

/// How loud the toys' sounds play, times the effects volume.
pub(crate) const VOLUME_CVAR: &str = "s_toyVolume";
/// [`VOLUME_CVAR`]'s default: as the toys were heard before 25/10/2026.
pub(crate) const DEFAULT_VOLUME: f64 = 1.0;
/// [`VOLUME_CVAR`]'s top: twice as loud.
pub(crate) const MAX_VOLUME: f64 = 2.0;

/// The toys that make sounds (the Illuminate holocron is silent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Toy {
    Box,
    Order66,
    Hax,
    Ragdoll,
    Drone,
    Boombox,
    Monitor,
    MysteryBox,
    Banana,
    /// The Toy Master's hidden toys (`toy_master.rs`).
    Master,
    PropGun,
    Tag,
    Fishing,
    /// The WHEEL OF DOOM (`wheel_of_doom.rs`).
    WheelOfDoom,
    Mining,
}

impl Toy {
    pub(crate) const ALL: [Self; 15] = [
        Self::Box,
        Self::Order66,
        Self::Hax,
        Self::Ragdoll,
        Self::Drone,
        Self::Boombox,
        Self::Monitor,
        Self::MysteryBox,
        Self::Banana,
        Self::Master,
        Self::PropGun,
        Self::Tag,
        Self::Fishing,
        Self::WheelOfDoom,
        Self::Mining,
    ];

    /// The switch that lets this toy be heard (on by default).
    pub(crate) const fn cvar(self) -> &'static str {
        match self {
            Self::Box => "s_toyBoxSound",
            Self::Order66 => "s_toyOrder66Sound",
            Self::Hax => "s_toyHaxSound",
            Self::Ragdoll => "s_toyRagdollSound",
            Self::Drone => "s_toyDroneSound",
            Self::Boombox => "s_toyBoomboxSound",
            Self::Monitor => "s_toyMonitorSound",
            Self::MysteryBox => "s_toyMysteryBoxSound",
            Self::Banana => "s_toyBananaSound",
            Self::Master => "s_toyMasterSound",
            Self::PropGun => "s_toyPropGunSound",
            Self::Tag => "s_toyTagSound",
            Self::Fishing => "s_toyFishingSound",
            Self::WheelOfDoom => "s_toyWheelOfDoomSound",
            Self::Mining => "s_toyMiningSound",
        }
    }
}

/// The settings as read this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Options {
    volume: f32,
    heard: [bool; Toy::ALL.len()],
}

impl Default for Options {
    fn default() -> Self {
        Self {
            volume: DEFAULT_VOLUME as f32,
            heard: [true; Toy::ALL.len()],
        }
    }
}

impl Options {
    pub(crate) fn read(console: Option<&ViewerConsole>) -> Self {
        let mut options = Self::default();
        let Some(console) = console else {
            return options;
        };
        if let Some(volume) = console.float_cvar(VOLUME_CVAR) {
            options.volume = volume.clamp(0.0, MAX_VOLUME) as f32;
        }
        for (heard, toy) in options.heard.iter_mut().zip(Toy::ALL) {
            *heard = console.bool_cvar(toy.cvar()).unwrap_or(true);
        }
        options
    }

    /// The gain a sound of `toy` recorded at `volume` plays with; `None` when it is
    /// not to be heard (switched off, or the volume at 0).
    pub(crate) fn gain(&self, toy: Toy, volume: f32) -> Option<f32> {
        let gain = volume * self.volume;
        (self.heard[toy as usize] && gain > 0.0).then_some(gain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switched_off_toy_or_zero_volume_is_silent() {
        let mut options = Options::default();
        assert_eq!(options.gain(Toy::Hax, 0.8), Some(0.8));
        options.heard[Toy::Hax as usize] = false;
        assert_eq!(options.gain(Toy::Hax, 1.0), None);
        assert_eq!(options.gain(Toy::Box, 1.0), Some(1.0));
        options.volume = 0.5;
        assert_eq!(options.gain(Toy::Box, 0.8), Some(0.4));
        options.volume = 0.0;
        assert_eq!(options.gain(Toy::Box, 1.0), None);
    }

    #[test]
    fn cvars_are_distinct() {
        for (index, toy) in Toy::ALL.iter().enumerate() {
            assert_eq!(*toy as usize, index);
            assert!(
                Toy::ALL[..index]
                    .iter()
                    .all(|other| other.cvar() != toy.cvar())
            );
        }
    }
}
