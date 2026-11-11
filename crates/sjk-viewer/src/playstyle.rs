//! Playstyles (Sol's request, 11/11/2026): each player says how they like to play
//! (saber duels, Force duels, FFA chaos, goofing around, teaching), a switch each
//! ([`Playstyle::cvar`], all off by default), and SJK players see it as small badges
//! over their nameplate (`hud::nameplate`, `cg_nameplatePlaystyle`). Purely social: it
//! blocks nothing and changes no rule, it only tells people what someone is up for.
//!
//! The picks travel in the player's hub look as toys ([`Playstyle::toy`], as
//! `toon_choice.rs` does), so the hub needs no change and older SJK clients ignore them.

use crate::looks::Looks;
use crate::saber_skins::MAX_CLIENTS;

/// Shows the badges on nameplates (`hud::nameplate`).
pub(crate) const SHOW_CVAR: &str = "cg_nameplatePlaystyle";
/// The own switches, one per playstyle ([`Playstyle::cvar`]).
pub(crate) const SABER_DUELS_CVAR: &str = "cg_playSaberDuels";
pub(crate) const FORCE_DUELS_CVAR: &str = "cg_playForceDuels";
pub(crate) const FFA_CVAR: &str = "cg_playFfa";
pub(crate) const GOOFING_CVAR: &str = "cg_playGoofing";
pub(crate) const TEACHING_CVAR: &str = "cg_playTeaching";

/// One way to play a player can say they enjoy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Playstyle {
    SaberDuels,
    ForceDuels,
    Ffa,
    Goofing,
    Teaching,
}

impl Playstyle {
    /// Every playstyle, in the order the badges stand: a playstyle's bit in [`Picks`]
    /// is its place here.
    pub(crate) const ALL: [Self; 5] = [
        Self::SaberDuels,
        Self::ForceDuels,
        Self::Ffa,
        Self::Goofing,
        Self::Teaching,
    ];

    /// The own switch.
    pub(crate) const fn cvar(self) -> &'static str {
        match self {
            Self::SaberDuels => SABER_DUELS_CVAR,
            Self::ForceDuels => FORCE_DUELS_CVAR,
            Self::Ffa => FFA_CVAR,
            Self::Goofing => GOOFING_CVAR,
            Self::Teaching => TEACHING_CVAR,
        }
    }

    /// The look's toy id (the hub knows no toys: any id of `a` to `z` and `_` goes).
    pub(crate) const fn toy(self) -> &'static str {
        match self {
            Self::SaberDuels => "play_saber",
            Self::ForceDuels => "play_force",
            Self::Ffa => "play_ffa",
            Self::Goofing => "play_goof",
            Self::Teaching => "play_teach",
        }
    }

    /// The words a focused player's badge shows.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::SaberDuels => "Saber duels",
            Self::ForceDuels => "Force duels",
            Self::Ffa => "FFA chaos",
            Self::Goofing => "Goofing around",
            Self::Teaching => "Teaching",
        }
    }

    /// The settings' explanation of the switch.
    pub(crate) const fn help(self) -> &'static str {
        match self {
            Self::SaberDuels => {
                "Tells SJK players you enjoy saber duels: a lit saber badge over your nameplate."
            }
            Self::ForceDuels => {
                "Tells SJK players you enjoy full Force duels: a lightning badge over your nameplate."
            }
            Self::Ffa => {
                "Tells SJK players you are up for FFA chaos: a burst badge over your nameplate."
            }
            Self::Goofing => {
                "Tells SJK players you are here to goof around, not to fight: a smile badge over your nameplate."
            }
            Self::Teaching => {
                "Tells SJK players you are happy to teach (or to learn): a book badge over your nameplate."
            }
        }
    }

    /// The badge's colour.
    pub(crate) const fn colour(self) -> [f32; 3] {
        match self {
            Self::SaberDuels => [0.35, 0.72, 1.0],
            Self::ForceDuels => [0.72, 0.5, 1.0],
            Self::Ffa => [1.0, 0.42, 0.3],
            Self::Goofing => [1.0, 0.84, 0.3],
            Self::Teaching => [0.4, 0.9, 0.55],
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A player's playstyles, a bit each in [`Playstyle::ALL`]'s order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Picks(pub(crate) u8);

impl Picks {
    pub(crate) fn with(self, style: Playstyle, on: bool) -> Self {
        if on {
            Self(self.0 | style.bit())
        } else {
            Self(self.0 & !style.bit())
        }
    }

    pub(crate) fn has(self, style: Playstyle) -> bool {
        self.0 & style.bit() != 0
    }

    pub(crate) fn any(self) -> bool {
        self.0 != 0
    }

    /// The playstyles picked, in badge order.
    pub(crate) fn iter(self) -> impl Iterator<Item = Playstyle> {
        Playstyle::ALL
            .into_iter()
            .filter(move |style| self.has(*style))
    }

    pub(crate) fn count(self) -> usize {
        self.0.count_ones() as usize
    }
}

/// Each client slot's picks as the looks carry them (the local player's own included).
pub(crate) fn slots(looks: &Looks) -> [Picks; MAX_CLIENTS] {
    let mut picks = [Picks::default(); MAX_CLIENTS];
    for (slot, pick) in picks.iter_mut().enumerate() {
        *pick = Playstyle::ALL
            .into_iter()
            .fold(Picks::default(), |pick, style| {
                pick.with(style, looks.toy(slot, style.toy()))
            });
    }
    picks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_carries_every_playstyle() {
        for style in Playstyle::ALL {
            assert!(crate::looks::TOYS.contains(&style.toy()), "{style:?}");
            assert!(
                style
                    .toy()
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_'),
                "the hub takes a to z and _"
            );
        }
    }

    #[test]
    fn picks_keep_badge_order() {
        let picks = Picks::default()
            .with(Playstyle::Teaching, true)
            .with(Playstyle::SaberDuels, true)
            .with(Playstyle::Ffa, true)
            .with(Playstyle::Ffa, false);
        assert_eq!(
            picks.iter().collect::<Vec<_>>(),
            [Playstyle::SaberDuels, Playstyle::Teaching]
        );
        assert_eq!(picks.count(), 2);
        assert!(!Picks::default().any());
    }
}
