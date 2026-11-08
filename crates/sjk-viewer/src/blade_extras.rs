//! Blade extras (Sol, 26/10/2026; `docs/unlockables.md`, "Extras"): pieces of a signature
//! skin that stay off until their wearer switches them on, and that only a holder of a
//! given card in a given finish may switch on. The Sun's are its other swing trails, a
//! Solar copy of SJK Alpha 000 (Sol's card) unlocking them; other blades may get theirs
//! the same way.
//!
//! A signature declares them in its header ([`crate::blade_signatures::Signature`]):
//!
//! ```text
//! // @trail plasma
//! // @extras {"card":"sjk_alpha_000","card_name":"SJK Alpha 000 (Sol)","finish":"solar"}
//! // @extra {"id":"flare_trail","name":"Flare trail","trail":"flare"}
//! ```
//!
//! `@trail` is the trail every wearer gets ([`TrailKind`]); `@extras` names the card and
//! finish that unlock the skin's extras ([`Gate`]); each `@extra` line is one of them
//! ([`Extra`]): another trail, something its swing sheds (`saber_petals.rs`), or a heat wake
//! bending what is behind the trail. The choice is kept with the skin's options
//! (`saber_skin_options.rs`): a word there names an extra switched on, where it names a
//! part switched off. The look carries them as `saber_on` and the hub relays only those
//! its wearer may wear, so every SJK player draws the wearer the same way.

use crate::saber_petals::PetalSpec;
use crate::saber_skin_options::OptionsOff;
use serde::Deserialize;
use std::sync::OnceLock;

/// How a skin's swing trail is drawn (`saber_trail.wgsl`'s vertex styles).
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TrailKind {
    /// The stock blur in the skin's trail colour (style 3).
    #[default]
    Blur,
    /// Comic speed lines (style 4, the Clone Wars blade).
    Comic,
    /// Plasma: white-hot where the blade just was, cooling through gold and orange to deep
    /// red as it ages, its trailing edge torn into flames (style 5, the Sun's).
    Plasma,
    /// The plasma with flames and magnetic loops licking off past the tip's edge, as
    /// prominences off the Sun's limb (style 6).
    Flare,
    /// Scythe (style 10, the Trayus Scythe blade's): a ribbon of pink light, a white seam where the tip
    /// passed, full of glitter, its edge torn into wisps and fading to violet as it ages.
    Scythe,
    /// Wolf spirit (style 11, the Vulptex Crystalfire blade's): a ribbon of red-orange spirit flame, gold
    /// where the blade just passed, its trailing edge boiling into bubbles that pop and fade.
    Foxfire,
    /// Storm (style 12, the Zeffo Thunderchorus blade's): a white-hot seam with jagged cyan lightning
    /// forking back from it across the swept band, flickering out as it ages.
    Storm,
    /// Erasure (style 13, the Moraband Erasure blade's): a deep violet ribbon crumbling into motes
    /// of light from its trailing edge as it ages, as if erased.
    Erasure,
    /// Ragnos (style 14, the Korriban Ragnos blade's): a ribbon of black with a crimson edge
    /// where the blade just passed, red cracks of light flickering through it as it fades.
    Ragnos,
    /// Infinite (style 15, the Rakata Infinite blade's): a ribbon of blue flame, white where
    /// the blade just passed, round glints of light twinkling in it as it fades.
    Infinite,
    /// Forge (style 16, the Tatooine Krayt Spine blade's): a ribbon of molten metal, white-hot where
    /// the blade just passed, cooling through orange and red to black with glowing cracks.
    Forge,
    /// Sand (style 17, the Lah'mu Sandveil blade's): a band of golden sand streaming back from
    /// the swing, grains glinting in it, thinning and falling away as it fades.
    Sand,
    /// Vitiate (style 18, the Dromund Kaas Vitiate blade's): a magenta and violet ribbon with a
    /// dark heart, its trailing edge torn into rising wisps.
    Vitiate,
    /// Equilibrium (style 19, the Bendu Equilibrium blade's): a thin silver ribbon with a white seam,
    /// a cool shimmer running through it like heat haze.
    Equilibrium,
    /// Vortex (style 20, the Lehon Maelstrom blade's): a deep blue band whose light swirls
    /// round in spirals as it fades, as if drawn into a vortex.
    Vortex,
    /// Starlight (style 21, the Zonama Starlight blade's): a white-gold ribbon full of
    /// twinkling stars, fading to a soft blue.
    Starlight,
    /// Convergence (style 22, the Peridea Convergence blade's): two bands, red and blue, laid
    /// side by side across the swing and running together into a deep violet as it fades.
    Convergence,
    /// Shadowflame (style 23, the Byss Force Storm blade's): a dark band in the storm's colour with
    /// blue-white Force lightning forking through it.
    Shadowflame,
    /// Sorcery (style 24, the Tund Sorcerers blade's): a band of gold light with dark beads
    /// strung along it, fading to white.
    Sorcery,
    /// Wildheart (style 25, the Haruun Kal Wildheart blade's): a green band with jagged green-white
    /// lightning crawling across it.
    Wildheart,
    /// Darksaber (style 26, the Kalevala Darksaber blade's): a dark band with a crackling white
    /// edge where the tip passed.
    Darksaber,
    /// Thoughtbomb (style 27, the Ruusan Thoughtbomb blade's): a silver-white band of pale flame with
    /// a gold seam.
    Thoughtbomb,
    /// Bone (style 28, the Tatooine Krayt Spine blade's): a sheet of blown sand with bone notches along
    /// its cutting edge.
    Bone,
    /// Hunger (style 29, the Nihilus Hunger blade's): a thin burning edge and pale motes drawn along it.
    Hunger,
    /// Hex (style 30, the Nightsister Ichor blade's): thick sap hanging in the air, drops sagging out of it.
    Hex,
    /// Gilded (style 31, the Ossus Scroll blade's): one stroke of a brush full of gold, drying.
    Gilded,
    /// Gale (style 32, the Utapau Whirlwind blade's): streaks of air combed along the swing, with dust.
    Gale,
    /// Salt (style 33, the Crait Salt Flats blade's): red scored open behind a skim of white salt.
    Salt,
    /// Kathol (style 34, the Kathol Rift blade's): a twisted ribbon, night and gold changing places along it.
    Kathol,
    /// Temple (style 35, Exar Kun Temple): a violet band with gold seams through it.
    Temple,
    /// Ash (style 36, Ziost Silence): a pale band crumbling into flakes.
    Ash,
    /// Rust (style 37, Nathema Deadworld): a dull band with dying embers in it.
    Rust,
    /// Neon (style 38, Taris Undercity): two thin tubes of light, one stuttering.
    Neon,
    /// Hush (style 39, Ysalamir Hush): a very soft breath of light.
    Hush,
    /// Blood (style 40, Naga Sadow Alchemy): a thick band sagging into drops.
    Blood,
    /// Carapace (style 41, Orbalisk Carapace): burning cells that crack apart.
    Carapace,
    /// Serpent (style 42, Amphistaff Serpent): a band wriggling as it fades.
    Serpent,
    /// Nebula (style 43, Vergence Scatter): a cloud of blue and gold with stars in it.
    Nebula,
    /// Sonic (style 44, Voxyn Scream): bands of sound running along the swing.
    Sonic,
    /// Plague (style 45, Rakghoul Plague): a sick band with spots of pus.
    Plague,
    /// Growth (style 46, Telos Restoration): green that climbs up a white band.
    Growth,
    /// Ion (style 47, Malevolence Ion Beam): thin crackling filaments.
    Ion,
    /// Cinder (style 48, Massassi Altar): smoke with embers in it.
    Cinder,
    /// Tremor (style 49, Sith Tremor Sword): a band shaken into jagged bars.
    Tremor,
    /// Press (style 50, Wayland Gravity Press): a heavy band sagging down as gold grains sink from it.
    Press,
    /// Cloud (style 51, the Angel blade): a soft white mist of cloud left in the air, slowly thinning.
    Cloud,
    /// Moonlight, the plasma's night sister: a soft silver ribbon with light rippling across it
    /// as on water, cooling to lilac and violet, its edge dissolving into stardust that drifts
    /// down (style 7, the Moon blade's).
    Moonlight,
    /// The moonlight hollowed into a ring of light along its edges, as the corona round an
    /// eclipsed moon, with a diamond-ring flash where the tip just passed (style 8).
    Eclipse,
    /// A faint moonlight ribbon with shooting stars streaking out of it and falling away
    /// (style 9).
    Starfall,
}

impl TrailKind {
    /// Its vertex style in `saber_trail.wgsl`.
    pub(crate) const fn style(self) -> u32 {
        match self {
            Self::Blur => 3,
            Self::Comic => 4,
            Self::Plasma => 5,
            Self::Flare => 6,
            Self::Scythe => 10,
            Self::Foxfire => 11,
            Self::Storm => 12,
            Self::Erasure => 13,
            Self::Ragnos => 14,
            Self::Infinite => 15,
            Self::Forge => 16,
            Self::Sand => 17,
            Self::Vitiate => 18,
            Self::Equilibrium => 19,
            Self::Vortex => 20,
            Self::Starlight => 21,
            Self::Convergence => 22,
            Self::Shadowflame => 23,
            Self::Sorcery => 24,
            Self::Wildheart => 25,
            Self::Darksaber => 26,
            Self::Thoughtbomb => 27,
            Self::Bone => 28,
            Self::Hunger => 29,
            Self::Hex => 30,
            Self::Gilded => 31,
            Self::Gale => 32,
            Self::Salt => 33,
            Self::Kathol => 34,
            Self::Temple => 35,
            Self::Ash => 36,
            Self::Rust => 37,
            Self::Neon => 38,
            Self::Hush => 39,
            Self::Blood => 40,
            Self::Carapace => 41,
            Self::Serpent => 42,
            Self::Nebula => 43,
            Self::Sonic => 44,
            Self::Plague => 45,
            Self::Growth => 46,
            Self::Ion => 47,
            Self::Cinder => 48,
            Self::Tremor => 49,
            Self::Press => 50,
            Self::Cloud => 51,
            Self::Moonlight => 7,
            Self::Eclipse => 8,
            Self::Starfall => 9,
        }
    }
}

/// The card, in a finish, whose holder may switch a skin's extras on.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Gate {
    /// The card's id (`sjk_alpha_000`).
    pub(crate) card: String,
    /// What players read for it ("SJK Alpha 000 (Sol)").
    pub(crate) card_name: String,
    /// The finish a copy of it must have (`solar`, `crate::cards::FINISHES`).
    pub(crate) finish: String,
}

impl Gate {
    /// What unlocks the extras, as players read it: "a Solar copy of SJK Alpha 000 (Sol)
    /// unlocks them".
    pub(crate) fn unlocks(&self) -> String {
        let finish = crate::cards::Finish::from_id(&self.finish).map_or("", |finish| finish.name);
        format!("a {finish} copy of {} unlocks them", self.card_name)
    }
}

/// One extra of a skin.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Extra {
    /// What the looks carry (`saber_on`): 1 to 24 of `a` to `z`, `0` to `9` and `_`.
    pub(crate) id: String,
    /// What the Collection shows: 1 to 24 printable ASCII characters.
    pub(crate) name: String,
    /// The trail it draws instead of the skin's own.
    #[serde(default)]
    pub(crate) trail: Option<TrailKind>,
    /// What its swing sheds (`saber_petals.rs`; its `part` is not read), for a skin that
    /// sheds nothing of its own.
    #[serde(default)]
    pub(crate) sheds: Option<PetalSpec>,
    /// Whether the trail bends what is behind it, as heat does (`saber_trail.wgsl`
    /// `fragment_wake`).
    #[serde(default)]
    pub(crate) wake: bool,
}

/// A skin's extras: the card that unlocks them and each one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Extras {
    pub(crate) skin: &'static str,
    pub(crate) gate: Gate,
    pub(crate) list: Vec<Extra>,
}

/// What a wearer's extras switched on add to their blade.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Worn {
    pub(crate) trail: Option<TrailKind>,
    pub(crate) sheds: Option<PetalSpec>,
    pub(crate) wake: bool,
}

/// Most extras a skin may have.
pub(crate) const MAX_EXTRAS: usize = 6;

impl Extras {
    /// Whether `id` is one of them.
    pub(crate) fn has(&self, id: &str) -> bool {
        self.list.iter().any(|extra| extra.id == id)
    }

    /// Their ids' bits ([`crate::blade_skin_file::extras::option_bit`]).
    #[cfg(test)]
    pub(crate) fn bits(&self) -> OptionsOff {
        crate::saber_skin_options::bits(self.list.iter().map(|extra| extra.id.as_str()))
    }

    /// What those of them on in `choice` (the wearer's choice's bits) add, the later
    /// ones winning where two draw a trail or shed.
    pub(crate) fn worn(&self, choice: OptionsOff) -> Worn {
        let mut worn = Worn::default();
        for extra in &self.list {
            if choice & crate::blade_skin_file::extras::option_bit(&extra.id) == 0 {
                continue;
            }
            worn.trail = extra.trail.or(worn.trail);
            worn.sheds = extra.sheds.or(worn.sheds);
            worn.wake |= extra.wake;
        }
        worn
    }

    /// Whether everything in them is in reason: 1 to [`MAX_EXTRAS`] extras with distinct
    /// ids and names of the options' form, each doing something, whose bits take none of
    /// another's or of the skin's options (`options`' bits), and a card and finish named.
    #[cfg(test)]
    pub(crate) fn check(&self, options: OptionsOff) -> Result<(), String> {
        let id_ok = |id: &str| {
            (1..=24).contains(&id.len())
                && id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        };
        if self.gate.card.is_empty()
            || self.gate.card_name.is_empty()
            || crate::cards::Finish::from_id(&self.gate.finish).is_none()
        {
            return Err(format!("{}: @extras names no card or no finish", self.skin));
        }
        if self.list.is_empty() || self.list.len() > MAX_EXTRAS {
            return Err(format!("{}: 1 to {MAX_EXTRAS} extras", self.skin));
        }
        let mut taken = options;
        for extra in &self.list {
            let bit = crate::blade_skin_file::extras::option_bit(&extra.id);
            if !id_ok(&extra.id)
                || !(1..=24).contains(&extra.name.len())
                || !extra.name.bytes().all(|b| (b' '..=b'~').contains(&b))
            {
                return Err(format!(
                    "{}: extra {:?} is badly named",
                    self.skin, extra.id
                ));
            }
            if taken & bit != 0 {
                return Err(format!(
                    "{}: extra {:?} takes another option's or extra's bit",
                    self.skin, extra.id
                ));
            }
            if extra.trail.is_none() && extra.sheds.is_none() && !extra.wake {
                return Err(format!("{}: extra {:?} does nothing", self.skin, extra.id));
            }
            if extra.sheds.is_some_and(|sheds| !sheds.valid()) {
                return Err(format!(
                    "{}: extra {:?} sheds out of reason",
                    self.skin, extra.id
                ));
            }
            taken |= bit;
        }
        Ok(())
    }
}

/// Every signature skin's extras, read once from their headers.
fn table() -> &'static [Extras] {
    static TABLE: OnceLock<Vec<Extras>> = OnceLock::new();
    TABLE.get_or_init(|| {
        crate::blade_signatures::SIGNATURES
            .iter()
            .filter_map(crate::blade_signatures::Signature::extras)
            .collect()
    })
}

/// The extras of skin `skin`, if it has any.
pub(crate) fn of(skin: &str) -> Option<&'static Extras> {
    table().iter().find(|extras| extras.skin == skin)
}

/// Whether `option` of `skin` is an extra (off until switched on) rather than a part.
pub(crate) fn is_extra(skin: &str, option: &str) -> bool {
    of(skin).is_some_and(|extras| extras.has(option))
}

/// What `skin`'s extras on in `choice` add to its blade.
pub(crate) fn worn(skin: &str, choice: OptionsOff) -> Worn {
    of(skin).map_or_else(Worn::default, |extras| extras.worn(choice))
}

/// `setting` (`cg_saberSkinOptions`) without its words for the extras of the skins whose
/// card the player does not hold (`holds`): what may be worn and sent.
pub(crate) fn gated(setting: &str, holds: impl Fn(&Gate) -> bool) -> String {
    setting
        .split_whitespace()
        .filter(|word| {
            let Some((skin, option)) = word.split_once('.') else {
                return true;
            };
            match of(skin) {
                Some(extras) if extras.has(option) => holds(&extras.gate),
                _ => true,
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `cards` (the own profile's) hold a revealed copy of `gate`'s card in its finish.
pub(crate) fn holds(cards: &[sjk_identity::CardStack], gate: &Gate) -> bool {
    cards.iter().any(|stack| {
        stack.card.as_deref() == Some(gate.card.as_str()) && stack.finish == gate.finish
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sun() -> &'static Extras {
        of("saber_sun").expect("the Sun has extras")
    }

    #[test]
    fn every_signatures_extras_are_in_reason() {
        for extras in table() {
            let signature = crate::blade_signatures::of(extras.skin).unwrap();
            let def = crate::blade_skin_file::parse(extras.skin, signature.default_file())
                .map(|def| signature.shown(&def))
                .unwrap();
            let options = crate::saber_skin_options::bits(
                def.options.iter().map(|option| option.id.as_str()),
            );
            extras.check(options).unwrap();
        }
        // Every `@extra` line parsed: none was dropped for a typo.
        for signature in &crate::blade_signatures::SIGNATURES {
            let lines = signature.header_lines("extra").count();
            assert_eq!(
                signature.extras().map_or(0, |extras| extras.list.len()),
                lines,
                "{}",
                signature.id
            );
            assert!(
                signature
                    .header_lines("trail")
                    .all(|line| signature.trail().is_some() && !line.is_empty())
            );
        }
    }

    #[test]
    fn the_sun_draws_plasma_and_a_solar_copy_of_sols_card_unlocks_the_rest() {
        let signature = crate::blade_signatures::of("saber_sun").unwrap();
        assert_eq!(signature.trail(), Some(TrailKind::Plasma));
        let sun = sun();
        assert_eq!(
            sun.gate,
            Gate {
                card: "sjk_alpha_000".to_owned(),
                card_name: "SJK Alpha 000 (Sol)".to_owned(),
                finish: "solar".to_owned()
            }
        );
        assert_eq!(
            sun.gate.unlocks(),
            "a Solar copy of SJK Alpha 000 (Sol) unlocks them"
        );
        let ids: Vec<&str> = sun.list.iter().map(|extra| extra.id.as_str()).collect();
        assert_eq!(ids, ["flare_trail", "swing_embers", "heat_wake"]);
        assert_eq!(
            sun.worn(0),
            Worn::default(),
            "every extra off until switched on"
        );
        let all = sun.worn(sun.bits());
        assert_eq!(all.trail, Some(TrailKind::Flare));
        assert_eq!(
            all.sheds.map(|sheds| sheds.kind),
            Some(crate::saber_petals::PetalKind::Ember)
        );
        assert!(all.wake);
        let wake = sun.worn(crate::blade_skin_file::extras::option_bit("heat_wake"));
        assert_eq!((wake.trail, wake.sheds, wake.wake), (None, None, true));
    }

    /// The Moon, the Sun's night sister (Creyon's): moonlight for every wearer, and a Solar
    /// copy of Creyon's card unlocks its eclipse and starfall trails and a moonlit wake.
    #[test]
    fn the_moon_draws_moonlight_and_a_solar_copy_of_creyons_card_unlocks_the_rest() {
        let signature = crate::blade_signatures::of("saber_moon").unwrap();
        assert_eq!(signature.trail(), Some(TrailKind::Moonlight));
        let moon = of("saber_moon").expect("the Moon has extras");
        assert_eq!(moon.gate.card, "sjk_alpha_002");
        assert_eq!(
            moon.gate.unlocks(),
            "a Solar copy of SJK Alpha 002 (Creyon) unlocks them"
        );
        let ids: Vec<&str> = moon.list.iter().map(|extra| extra.id.as_str()).collect();
        assert_eq!(ids, ["moonshadow_trail", "starfall_trail", "moonlit_wake"]);
        assert_eq!(moon.worn(0), Worn::default());
        let bit = crate::blade_skin_file::extras::option_bit;
        assert_eq!(
            moon.worn(bit("starfall_trail")).trail,
            Some(TrailKind::Starfall)
        );
        assert!(moon.worn(bit("moonlit_wake")).wake);
        assert_eq!(
            [
                TrailKind::Moonlight,
                TrailKind::Eclipse,
                TrailKind::Starfall
            ]
            .map(TrailKind::style),
            [7, 8, 9]
        );
    }

    #[test]
    fn only_a_holder_of_the_card_in_its_finish_keeps_the_extras_words() {
        let setting = "saber_sun.flare_trail saber_sun.haze saber_void.warp saber_sun.heat_wake";
        assert_eq!(gated(setting, |_| true), setting);
        assert_eq!(gated(setting, |_| false), "saber_sun.haze saber_void.warp");
        let stack = |card: &str, finish: &str| -> sjk_identity::CardStack {
            serde_json::from_value(serde_json::json!({
                "card": card, "rarity": "mythical", "finish": finish, "count": 1
            }))
            .unwrap()
        };
        let gate = &sun().gate;
        assert!(!holds(&[stack("sjk_alpha_000", "holo")], gate));
        assert!(!holds(&[stack("sjk_alpha_001", "solar")], gate));
        assert!(holds(
            &[
                stack("sjk_alpha_001", "solar"),
                stack("sjk_alpha_000", "solar")
            ],
            gate
        ));
    }

    #[test]
    fn a_bad_extras_line_is_refused() {
        let extra = |id: &str| Extra {
            id: id.to_owned(),
            name: "Flare".to_owned(),
            trail: Some(TrailKind::Flare),
            sheds: None,
            wake: false,
        };
        let extras = |list: Vec<Extra>| Extras {
            skin: "saber_x",
            gate: Gate {
                card: "sjk_alpha_000".to_owned(),
                card_name: "SJK Alpha 000 (Sol)".to_owned(),
                finish: "solar".to_owned(),
            },
            list,
        };
        assert!(extras(vec![extra("flare")]).check(0).is_ok());
        assert!(extras(vec![extra("Flare")]).check(0).is_err());
        assert!(
            extras(vec![extra("flare"), extra("flare")])
                .check(0)
                .is_err()
        );
        let bit = crate::blade_skin_file::extras::option_bit("flare");
        assert!(
            extras(vec![extra("flare")]).check(bit).is_err(),
            "an option's bit"
        );
        assert!(extras(Vec::new()).check(0).is_err());
        let idle = Extra {
            trail: None,
            ..extra("idle")
        };
        assert!(extras(vec![idle]).check(0).is_err());
        let mut no_finish = extras(vec![extra("flare")]);
        no_finish.gate.finish = "gold".to_owned();
        assert!(no_finish.check(0).is_err());
    }
}
