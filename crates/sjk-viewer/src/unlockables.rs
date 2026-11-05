//! The unlockables this client knows (`docs/unlockables.md`, "Catalogue"): the one
//! catalogue the looks, the blade skins, the `saberskin` command, the Unlockables page
//! and the Staff page read. The hub keeps its own copy (`src/unlocks.rs`); a new
//! unlockable needs both, and an id never changes meaning. An id this client does not
//! know is ignored wherever it comes from (a profile, a look, `cg_saberSkin`).
//!
//! The catalogue is public metadata only: what an unlockable looks and sounds like is
//! art in the SJK hub's packs (`sjk_packs.rs`), a blade skin's in its blade-skin file.

/// Archived: the blade-skin unlock id the player wears, empty for the stock blade. It
/// shows (and is sent to the hub) only while the player's own hub profile lists it.
pub(crate) const SABER_SKIN_CVAR: &str = "cg_saberSkin";

/// What an unlockable is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// A look a player's blades wear instead of their colour, drawn from its blade-skin
    /// file (`skins/blades/<id>.bladeskin` in a pack).
    BladeSkin,
}

/// How rare an unlockable is, the commonest first: Common (14/10/2026, white: filler
/// that holocrons of any tier may drop; there is no Common holocron), the holocrons'
/// four tiers ([`crate::holocrons::TIERS`]) with their names and colours, and Unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
    Mythical,
    /// 25/10/2026, rose: Sol's personal thank-you gifts, given by hand at the hub. Never
    /// found, dropped, sold, traded or crafted (no holocron, crystal, shop or Forge has the
    /// tier), and seen only by the players who hold one
    /// ([`Unlockable::hidden_unless_held`]). After Mythical, so it sorts as the rarest.
    Unique,
}

impl Rarity {
    /// The holocron tier of the same name; `None` for Common and Unique, which no
    /// holocron is.
    fn holocron_tier(self) -> Option<&'static crate::holocrons::Tier> {
        match self {
            Self::Common | Self::Unique => None,
            tier => Some(&crate::holocrons::TIERS[tier as usize - 1]),
        }
    }

    /// The tier's name ("Rare").
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Unique => "Unique",
            tier => tier.holocron_tier().map_or("Common", |tier| tier.label),
        }
    }

    /// The tier's colour: Common's is white (a touch cool, to sit on the menus), Unique's
    /// rose.
    pub(crate) fn colour(self) -> sjk_ui::Color {
        match self {
            Self::Unique => crate::holocrons::rgb(0xFF_6B9A),
            tier => tier
                .holocron_tier()
                .map_or(crate::holocrons::rgb(0xE6_EAF2), |tier| tier.colour),
        }
    }

    /// The tier's mark in SJK Symbols, Common's the empty octagon.
    pub(crate) fn symbol(self) -> char {
        crate::symbols::RARITY[self as usize]
    }

    /// Its place among the effects' ranks ([`crate::rarity_fx`]): Common and Uncommon
    /// are 0 and draw none; Unique has Mythical's (Sol: "mythical level of effects"),
    /// drawn in its own colour.
    pub(crate) fn effect_rank(self) -> usize {
        (self.min(Self::Mythical) as usize).saturating_sub(1)
    }
}

/// The saber smiths: who gave a shader's idea ([`Unlockable::smith`]).
pub(crate) const SOL: &str = "Sol";
pub(crate) const CREYON: &str = "Creyon";

/// How to get a shader that holocrons drop ([`Unlockable::how_to_get`]).
pub(crate) const FROM_HOLOCRONS: &str = "Found in holocrons.";
/// How to get a Unique one ([`Rarity::Unique`]): Sol gives it by hand.
pub(crate) const PERSONAL_THANKS: &str = "A personal thank-you from Sol.";
/// How to get the mystery boxes' own blade (`mystery_box`, 29/10/2026).
pub(crate) const FROM_MYSTERY_BOXES: &str =
    "Found only in mystery boxes: never in holocrons, the shop or a trade.";

/// The blades a card brings, `(blade id, card id)`: a key holding a copy of the card holds
/// the blade with it, and loses that copy of it with its last one, as a medal's blade goes with
/// the medal ([`crate::medals::Medal::blade`]). Unlike a medal's, a card's blade may also
/// drop: the card is a second way to get it. The hub decides it from the card file's
/// `grants` (`docs/cards.md`, "Faces and the first set") and says so in the unlock's `card`;
/// this mirrors those files so the catalogue names the card without the packs. A new card
/// that brings a blade adds an entry here and to its file.
pub(crate) const CARD_BLADES: [(&str, &str); 1] = [
    // Sol's decision, 20/10/2026: the Sun is Sol's own blade, so Sol's card (SJK Alpha 000)
    // brings it as well as holocrons.
    ("saber_sun", "sjk_alpha_000"),
];

/// The card that brings blade `id` ([`CARD_BLADES`]), if one does.
pub(crate) fn card_bringing(id: &str) -> Option<&'static str> {
    CARD_BLADES
        .iter()
        .find(|(blade, _)| *blade == id)
        .map(|(_, card)| *card)
}

/// One unlockable of the catalogue.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Unlockable {
    /// The id the hub, a look and `cg_saberSkin` name it by.
    pub(crate) id: &'static str,
    /// Its number, as a card's in its set: given once, in the order blades came (the Sun
    /// is 1), never changed or reused, so notes and reviews can name a blade by it
    /// ("Blade 042"). A new blade takes the next free number.
    pub(crate) number: u16,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    /// How rare it is ([`Rarity`]): Common, the holocrons' four tiers or Unique.
    pub(crate) tier: Rarity,
    /// A chroma: a blade skin that takes the wearer's saber colour (`color1` for the
    /// first saber, `color2` for the second), its colours turned round to that hue
    /// ([`crate::saber_skins::Chroma`]). The others keep the colours their file gives.
    pub(crate) chroma: bool,
    /// Whether a holocron may drop it (the hub's `droppable`, `docs/unlockables.md`,
    /// "Holocron shader drops"): since 17/10/2026 every shader but those a medal brings
    /// ([`crate::medals::Medal::blade`]) and the Unique ones, given by hand; a card's blade
    /// drops too ([`CARD_BLADES`]).
    pub(crate) droppable: bool,
    /// The edition it belongs to ([`crate::editions`]): the stamp of when it came.
    pub(crate) edition: &'static str,
    /// Its saber smith: who gave the idea, credited on its cards ("Saber smith: Sol"),
    /// even when it was made together.
    pub(crate) smith: &'static str,
    /// What it is, in a line.
    pub(crate) description: &'static str,
    /// How a player gets it, in a line.
    pub(crate) how_to_get: &'static str,
}

impl Unlockable {
    /// Whether it is a blade skin.
    pub(crate) const fn is_blade_skin(&self) -> bool {
        matches!(self.kind, Kind::BladeSkin)
    }

    /// Its [`number`](Self::number) as players and notes read it: three digits, "042".
    pub(crate) fn number_text(&self) -> String {
        format!("{:03}", self.number)
    }

    /// Whether only the players who hold it may see it (Sol's rule, 25/10/2026): a Unique
    /// one, which no one can find, buy or trade, is never listed, counted or found by a
    /// search for anyone else ([`Holdings::shows`]). The Staff page, where it is granted,
    /// and `debug_collectibles` list it all the same.
    pub(crate) const fn hidden_unless_held(&self) -> bool {
        matches!(self.tier, Rarity::Unique)
    }
}

/// The blade skin numbered `number` ([`Unlockable::number`]).
pub(crate) fn blade_skin_numbered(number: u16) -> Option<&'static Unlockable> {
    blade_skins().find(|unlockable| unlockable.number == number)
}

/// The blade skin a word names: its id, or its number ("42", "042" or "#42").
pub(crate) fn blade_skin_named(word: &str) -> Option<&'static Unlockable> {
    let digits = word.strip_prefix('#').unwrap_or(word);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return digits.parse().ok().and_then(blade_skin_numbered);
    }
    blade_skin(word)
}

/// Every unlockable, in the hub's catalogue order (the order profiles list them in).
pub(crate) static ALL: [Unlockable; 169] = [
    Unlockable {
        id: "saber_sun",
        number: 1,
        name: "Sun blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A star in the hand: a white-gold core breathing under an orange corona, sunspots drifting along it, prominences, coronal rain and a solar wind, and the strongest light of any blade. It lights as a sunrise, goes out as a sunset, and a clash is an eclipse's diamond ring.",
        how_to_get: "Found in holocrons, and comes with Sol's card (SJK Alpha 000) for as long as you hold it.",
    },
    Unlockable {
        id: "saber_storm",
        number: 2,
        name: "Storm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A white-blue core crackling with lightning that leaps off it, sparks at its tip.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_void",
        number: 3,
        name: "Void blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A dark core in a violet rim, star specks drifting into it, and a deep hum.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_frost",
        number: 4,
        name: "Frost blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A pale ice core with a pointed tip, frost shards trailing it, and a chiming hum.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_prism",
        number: 5,
        name: "Prism blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A white core in a corona whose colours run through the rainbow, slowly turning.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_unstable",
        number: 6,
        name: "Unstable blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A raw red core that crackles and sputters, its edge ragged and its length faltering.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_molten",
        number: 7,
        name: "Molten blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A dark rock core split by glowing lava cracks, embers dripping off it and falling.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_spectral",
        number: 8,
        name: "Spectral blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A pale, see-through blade: wisps rise off it, souls drift round it, an echo sways beside it and afterimages trail its swings.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_glitch",
        number: 9,
        name: "Glitch blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A blade whose colours split apart, blocks of it jumping sideways now and then.",
        how_to_get: "Comes with the Bug Hunter medal, for as long as you hold it.",
    },
    Unlockable {
        id: "saber_hologram",
        number: 10,
        name: "Hologram blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A wireframe projection with scan lines running along it, jittering like a training hologram.",
        how_to_get: "Comes with the Tester medal, for as long as you hold it.",
    },
    Unlockable {
        id: "saber_runic",
        number: 11,
        name: "Runic blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A gold core with glyphs scrolling along it that spell its wearer's name.",
        how_to_get: "Comes with the Contributor medal, for as long as you hold it.",
    },
    Unlockable {
        id: "saber_chameleon",
        number: 12,
        name: "Chameleon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A blade that takes the colour of the light where it is, changing from map to map.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_banner",
        number: 13,
        name: "Banner blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "Your team's colour in team games, rippling like a flag; silver outside them.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_heartbeat",
        number: 14,
        name: "Heartbeat blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A warm, deep blade that pulses with a slow double heartbeat, its light beating with it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_tidal",
        number: 15,
        name: "Kamino blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A sea-blue blade from Kamino's endless ocean, a wave of brighter light surging from hilt to tip on a slow loop.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_circuit",
        number: 16,
        name: "Droid Logic blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A thin blade in its wearer's colour with sharp pulses of droid data racing along it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_aurora",
        number: 17,
        name: "Hoth Aurora blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Slow ribbons of green, teal and violet drifting along the blade, like the sky over Hoth.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_echo",
        number: 18,
        name: "Echo Base blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade in its wearer's colour with a bright ring rippling along it and fading away, like a signal from Echo Base.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_nebula",
        number: 19,
        name: "Nebula blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Deep indigo with tiny stars twinkling inside, a slice of the galaxy.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_cinder",
        number: 20,
        name: "Mustafar Ember blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A dull coal-orange blade from Mustafar that flickers and sheds glowing embers.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_venom",
        number: 21,
        name: "Geonosis Acid blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Acid green with droplets that bead up and drip off the blade, like the acid pits of Geonosis.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sakura",
        number: 22,
        name: "Naboo Blossom blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A soft pink blade shedding blossom petals that drift away, like the gardens of Naboo.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_firefly",
        number: 23,
        name: "Endor Lights blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A soft warm-green blade with the glowing lights of Endor's forest moon blinking around it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_slate",
        number: 24,
        name: "Durasteel blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A cool steel-grey blade with a faint, slow shimmer, plain and sturdy as durasteel.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_amber",
        number: 25,
        name: "Tatooine Gold blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A warm honey-gold blade with a soft glow that swells and settles, like Tatooine's sand at noon.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_lilac",
        number: 26,
        name: "Lothal Twilight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A gentle lilac blade whose colour drifts a little as it moves, like the grass plains of Lothal at dusk.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_mint",
        number: 27,
        name: "Yavin Jungle blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pale, fresh green blade with a cool wave of light running along it, like the jungle moon of Yavin.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_coral",
        number: 28,
        name: "Mon Cala Coral blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A salmon-pink blade with a warm pulse travelling from hilt to tip, like the coral cities of Mon Cala.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ivory",
        number: 29,
        name: "Jedi Temple blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A creamy, near-white blade with a soft golden fringe, like the halls of the Jedi Temple.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_glacier",
        number: 30,
        name: "Ilum blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Ice-blue with tiny crystals of frost glittering inside, from the frozen kyber world of Ilum.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_dusk",
        number: 31,
        name: "Binary Sunset blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Orange sinking into violet, the colour of Tatooine's twin suns setting, slowly turning.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ruby",
        number: 32,
        name: "Sith Ruby blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A deep red blade shedding bright sparks as it burns.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_amethyst",
        number: 33,
        name: "Kyber Amethyst blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber crystal in its wearer's colour, with tiny flecks of light twinkling in it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_jade",
        number: 34,
        name: "Dagobah Jade blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A calm sea-green blade whose hue slowly shifts, with specks of light inside, like the swamps of Dagobah.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_moonlight",
        number: 35,
        name: "Ahch-To Moonlight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Silver-blue with pale motes drifting along it like dust in the moonlight over Ahch-To.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_neon",
        number: 36,
        name: "Coruscant Neon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A glass neon tube from the lower levels of Coruscant, gas glowing inside, one of its six sections stuttering out now and then.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_peppermint",
        number: 37,
        name: "Kessel Spice blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pale gold blade glittering with refined spice from Kessel: crystals sparkling inside it and glittering dust drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_bubble",
        number: 38,
        name: "Bacta blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade of glowing bacta: blue-green healing fluid with currents rising in it and bubbles of every size streaming up round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_lavalamp",
        number: 39,
        name: "Coaxium blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Raw coaxium hyperfuel: glowing violet-red globules churning inside an unstable blade that spits sparks and short arcs.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_reactor",
        number: 40,
        name: "Reactor blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A hypermatter reactor core: containment rings marching up the blade, magnetic loops arching between them, reactor dust; it heats to critical red, vents and flashes, then cools.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_bleeding",
        number: 41,
        name: "Bleeding Kyber blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pure kyber blade, in its wearer's colour, fighting a red corruption that bleeds up from the hilt: Sith lightning crackling over it, red drops falling, while beams of pure light push it back and the two clash in sparks.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sketch",
        number: 42,
        name: "Death Star Plans blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The stolen battle station plans: a blueprint-blue blade with a scan line racing up it inside a turning holographic schematic.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_hourglass",
        number: 43,
        name: "Beskar Forge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Mandalorian steel fresh from the forge: a silver blade glowing orange-hot from the hilt with hammer marks glowing in it, forge flames licking up it, the hammer striking showers of sparks and quench steam rising.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_fiber",
        number: 44,
        name: "Kyber Cluster blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A cluster of raw kyber crystals: a blade in its wearer's colour with a ring of kyber shards in their own colours floating round it, each linked to it by a pulsing beam, their song rising round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_serpent",
        number: 45,
        name: "Sith Alchemy blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The dark science of the Sith: an emerald blade veined with gold, tall tongues of green fire licking off it, green and gold lightning crackling and alchemical vapour coiling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_monarch",
        number: 46,
        name: "Felucia Spores blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The fungal jungle of Felucia: glowing puffball spores rest on an amber-green blade while you stand still, swarm round it and follow it when you run or swing, and settle back once you stop.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_phoenix",
        number: 47,
        name: "Phoenix Squadron blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Rebellion's Phoenix: a blade of living flame with a plume of fire rising up it, embers and burning sparks streaming up, reborn in a flare of fire every few seconds.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_quantum",
        number: 48,
        name: "Mortis blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The realm of Mortis: the Daughter's and the Son's ghost blades swing out and fold back into it, and every second time they meet the realm turns from day to night or back, rising light by day and red lightning by night.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_comic",
        number: 49,
        name: "Clone Wars blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade drawn like the animated Clone Wars: flat colour, a thick black outline, its glow in halftone dots and comic words when lit, switched off, swung or clashed, a comic panel, impact lines and speed lines on swings, clone armour stripes, now and then a holo flicker, and cartoon smoke and spark stars. It takes your saber colour.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_quicksilver",
        number: 50,
        name: "Liquid Beskar blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade of liquid beskar: ripples run up its mirror steel, beads roll to the tip, rings of liquid metal ripple up round it, droplets orbit and fly off it and silver glints fly.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_obsidian",
        number: 51,
        name: "Obsidian Edge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A flat blade of black volcanic glass with white-hot edges, obsidian shards orbiting it, magma lightning leaping between them and the air shimmering with heat.",
        how_to_get: FROM_HOLOCRONS,
    },
    // Creyon's Blade Lab signature shaders (blade_signatures.rs), mythicals first: not in
    // the holocrons yet, given by the team while they are tried in game.
    Unlockable {
        id: "saber_abyssal_eye",
        number: 52,
        name: "Abyssal Eye blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A slit cut into an endless dark, its lips burning in its wearer's colour; it peels open into a great amber eye that turns to watch whoever looks at it and flinches shut on a clash.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_aurora_wyrm",
        number: 53,
        name: "Ilum Aurora Storm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Ilum's polar sky in a storm: green and magenta aurora curtains hanging round a pale-green blade, pale lightning crackling all round it, ice crystals glittering in the cold.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_balance",
        number: 54,
        name: "Balance blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Force in balance: a grey blade whose white-gold and violet faces turn round it, bands of light climbing through bands of shadow, a wave of each meeting at its middle in a flash, and a sun of light and an orb of shadow circling it opposite each other.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_between_worlds",
        number: 55,
        name: "Between Worlds blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A path of light from the World Between Worlds, in its wearer's colour: a translucent blade with sparkling edges, paths of light crossing the void beside it, nebula clouds drifting, and two bright lights, white and red, duelling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_big_bang",
        number: 56,
        name: "Big Bang blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A young universe: every ignition bursts from one blinding point into a fireball that lights the blade, then nebula clouds swirl round it, stars drift outward and small bangs throw shells of light off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_carbonite",
        number: 57,
        name: "Carbonite blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Frozen in carbonite: a dark bronze slab holds the blade while cracks of light spread, bursts into shards that float round it while the blade blazes, then fuses solid again.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_dyson_sphere",
        number: 58,
        name: "Dyson Sphere blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A star caged in a megastructure: five tilted hoops of light turning round a boiling white-gold blade, collector beams drinking from it, flame tongues and solar flares leaping up to the hoops.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_eclipse",
        number: 59,
        name: "Eclipse blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade in its wearer's colour with a total eclipse travelling along its upper half: the moon's band darkens it, diamond-ring flashes spark on its edge, the stars of totality come out round it and eclipsed suns circle it, glowing in its colour.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_event_horizon",
        number: 60,
        name: "Event Horizon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A black hole stretched into a blade: a rod of pure darkness with a razor photon ring, two accretion disks of glowing matter orbiting it, streams spiralling in and a gravity wave on a clash.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_exogorth",
        number: 61,
        name: "Exogorth blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The space slug's living gullet: a ribbed, glowing throat with a maw of pale fangs at the emitter, swallowing waves, asteroid rocks and grit sucked in and crushed in sparks, hot breath and a roar on a clash.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_hyperspace",
        number: 62,
        name: "Hyperspace blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A ship's jump as a blade in its wearer's colour: the stars stretched into streaks rushing up it inside a tunnel of light, coils of light turning slowly round it, all in its colour; it jumps when lit and drops out of hyperspace on a clash.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_kyber_heart",
        number: 63,
        name: "Kyber Heart blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber blade that beats like a heart, rose or in its wearer's colour: every beat pumps a pulse of light up it and throws sparks off it, veins of light winding round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_nebula_song",
        number: 64,
        name: "Nebula Song blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A deep-space nebula that sings: a deep-blue blade (or one in its wearer's colour) breathing with slow waves of light, clouds of rose, violet and teal glowing up beside it with each note, ribbons of glowing nebula gas winding round it and a hyperspace shimmer now and then.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_quasar",
        number: 65,
        name: "Quasar blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A galaxy's engine: a white-violet jet (or one in its wearer's colour) with bright knots racing up it, a tilted accretion disk spiralling round it and matter flung off on every swing.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_singing_kyber",
        number: 66,
        name: "Singing Kyber blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber crystal that sings: a clear faceted crystal over a teal blade, its standing wave's nodes blazing and trembling, sound rings rippling off it and a sonic boom on a clash.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sith_holocron",
        number: 67,
        name: "Sith Holocron blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A holocron's crimson crystal stretched into a blade: red lightning crackling all round it, a red heart beating inside and Sith knowledge streaming up the glass.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_star_dragon",
        number: 68,
        name: "Ashla Radiance blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Ashla, the light side of the Force, as a blade in its wearer's colour, calm, radiant light rising off it like a quiet flame, two great wings of light, soft rays of light and orbs of the Ashla's light circling it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_starforge",
        number: 69,
        name: "Starforge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber blade forged inside a star: tongues of white-gold star plasma licking off a boiling blade, hammer blows of sparks, the star drawn down into the forge in spiralling streams, a forge nova every fourth blow, embers falling and the air shimmering with its heat.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_superlaser",
        number: 70,
        name: "Superlaser blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A battle station's main weapon as a blade: eight beams fan in to a blazing, crackling focus above the hilt, plasma strands twist up the beam and surges race up it, each swelling into a ball of green plasma.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_uneti_tree",
        number: 71,
        name: "Uneti Tree blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Force-sensitive tree of Ahch-To as living wood-light: glowing fibres braided round a bright heartwood, sap of the Force pulsing up them, Force pollen and sparks of sap drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_witchfire",
        number: 72,
        name: "Witchfire blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Nightsister magick of Dathomir: a sickly green ichor blade with pulses of magick climbing it, green ghost-fire licking off it, energy tendrils coiling round it, ichor mist and dark smoke.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_world_shift",
        number: 73,
        name: "World Shift blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The galaxy's worlds in turn: Mustafar fire, Hoth ice, a Kamino storm, Tatooine sand, Crait salt and Mon Cala water, each change an explosion that blasts the old climate off and turns the blade into the next.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_wraithlight",
        number: 74,
        name: "Wraithlight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pale blade of the Force's spirits: a glass rod of moonlight brighter at its edges, cold spirit fire licking up it, spirit mist sinking to the hilt, faint echoes of the blade hanging in the air and wraiths darting round and through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_lightspeed_ram",
        number: 75,
        name: "Holdo Maneuver blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Holdo Maneuver: a cruiser jumping to lightspeed straight through a fleet. Starlines stream past a blade in its wearer's colour split red and cyan at its edges, its engines glowing at the hilt, until it falls silent, draws light in from all round it and cuts the air in a blinding white slice, wreckage flung off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_hoth_avalanche",
        number: 76,
        name: "Hoth Avalanche blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An ice-blue blade from Hoth: soft snowflakes tumble round it, a cold mist rolls off and sinks, and avalanche surges thunder down it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_blood_moon",
        number: 77,
        name: "Dathomir Blood Moon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The blood-red night of Dathomir: a dark crimson blade with a shadowed heart, red light dripping off it, a red mist curling up, two dark red streams winding round it and a blood moon circling it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_force_echo",
        number: 78,
        name: "Force Echo blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade that echoes through time: copies of it linger where it has been, an echo sways beside it, rings of time tick up it and a scale of time marks lights up beside it, tick by tick.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_hypermatter",
        number: 79,
        name: "Hypermatter blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Hypermatter racing up a starship's reactor stream: pulses chase up the blade through flickering containment rings, jets of plasma shooting up beside it and sparks spitting off.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_kessel_run",
        number: 80,
        name: "Kessel Run blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Kessel Run in under twelve parsecs: an icy blade with speed bands racing up it, two tails of light streaming back, a blue exhaust rolling off and sparks peeling away.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_unknown_regions",
        number: 81,
        name: "Unknown Regions blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The uncharted dark of the Unknown Regions: a deep violet blade in its own swirling nebula, faint stars drifting through it and a hyperspace storm of violet and blue gas whirling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_krayt_pearl",
        number: 82,
        name: "Krayt Pearl blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The pearl of a krayt dragon: a blade of mother-of-pearl whose nacre swirls pink, gold and blue, lustrous krayt pearls circling it, desert sand streaming past and a veil of sand dust curling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_starkiller_frost",
        number: 83,
        name: "Starkiller Frost blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The frozen planet that became a weapon: a frost-white blade with a frozen bolt inside, still forks of frozen lightning hanging off it, ice shards breaking off and frost mist sinking.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_royal_naboo",
        number: 84,
        name: "Royal Naboo blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The grace of Naboo's royal court: a white-gold blade traced with interlaced gold filigree round a thread of royal crimson, a veil of golden silk fluttering round it and petals of gold light drifting down.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_bespin_storm",
        number: 85,
        name: "Bespin Storm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A storm round Cloud City: the gas giant's banded sunset clouds swirl round the blade, gale streaks race past it and tibanna gas rises up it in shimmering rings.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_scarif_tide",
        number: 86,
        name: "Scarif Tide blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The turquoise seas of Scarif: clear tropical water lit by a moving web of sunlit caustics, a waterspout spiralling up round it, bubbles rising beside it and sea spray glittering off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_assassin_droid",
        number: 87,
        name: "Assassin Droid blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An assassin droid's weapon: a blade of shifting plated bands, a scanner ring sweeping up and down it, red targeting lasers locking on, two armoured gear rings turning round it, sparks spitting off its joints and a thin grey smoke rising.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_aurodium",
        number: 88,
        name: "Aurodium blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The rarest gold in the galaxy: molten gold flowing down the blade, drops of gold dripping off it, gold dust glittering round it and a gold shimmer winding up it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_kashyyyk_monsoon",
        number: 89,
        name: "Kashyyyk Monsoon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The monsoon over Kashyyyk: rain lashing past and streaming down the blade, green jungle mist steaming off it and torn wroshyr leaves tumbling past in the storm.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_malachor_forge",
        number: 90,
        name: "Malachor Forge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Beaten out in the old Sith forges of Malachor: a hot-steel blade struck by hammer blows that flare white with sprays of sparks, embers falling and forge smoke rising.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_yavin_vines",
        number: 91,
        name: "Yavin Vines blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The jungle of Yavin 4: a blade veined with green life, vines winding up round it with light pulsing along them, golden pollen drifting up and leaves of light falling.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_exegol_rift",
        number: 92,
        name: "Exegol Rift blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Sith world of Exegol torn open: a dark rift down the blade's heart between red-violet edges, tearing wide and spilling violet light, a vortex of the dark side whirling out of it, Sith lightning leaping, a dark storm churning and dust pulled up.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_shattered_kyber",
        number: 93,
        name: "Shattered Kyber blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A shattered kyber crystal: cracks flickering with light, clear shards always floating and turning round it (further out on a swing), splinters of light snapping off and kyber dust glittering.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_twin_suns",
        number: 94,
        name: "Twin Suns blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Tatooine's twin suns at full blaze: a seething gold and orange blade, loops of plasma arching off it, golden sunset beams slanting softly across it, a solar wind streaming off and the air shimmering.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_jedi_spirits",
        number: 95,
        name: "Jedi Spirits blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The light of the Jedi who became one with the Force: soft warm lights lift off the blade and float away, a pale spirit mist rises round it, three spirits climb round it trailing veils of light, a Force ghost's pale glow breathes round it and an echo of it sways beside it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ord_mantell_storm",
        number: 96,
        name: "Ord Mantell Thunderhead blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A storm cloud over the seas of Ord Mantell: a thin white-hot blade in its own churning grey cloud, lightning crawling through it and lighting it up, rain falling.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_mustafar_eruption",
        number: 97,
        name: "Mustafar Eruption blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An eruption on Mustafar: molten rock pouring down the blade, lava runnels running down both sides, flames licking up it, lava fountains, embers raining, ash smoke and shimmering heat.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_hyperlane",
        number: 98,
        name: "Hyperlane blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A hyperspace lane folded into a blade: rings of light rushing down a deep blue tunnel (or one in its wearer's colour) round it, starlines streaming along it and lane beacons blinking as they wind round it in a chase.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ahch_to_glow",
        number: 99,
        name: "Ahch-To Glow blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The glowing seas round Ahch-To: soft pulses climb a sea-green blade and specks of plankton light drift round it, sparkling brighter when it swings.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_imperial_schematics",
        number: 100,
        name: "Imperial Schematics blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Imperial schematics as a blade: schematic lines draw themselves up beside it, measure marks ticking out, then fade and start again.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_jedi_texts",
        number: 101,
        name: "Jedi Texts blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The sacred Jedi texts as a blade: two flowing strokes of golden ink curl up beside it, thick and thin like a brush, never still.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_star_map",
        number: 102,
        name: "Star Map blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A Jedi star map as a blade: stars light up beside it one by one and link into a figure (a route, a ring, a triangle, a spiral, a diamond, a crown), then fade while a new chart forms.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ilum_frost",
        number: 103,
        name: "Ilum Frost blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The ice caves of Ilum: frost creeping up the blade in cracked crystal, a cold mist rolling off and sinking, ice crystals glinting as they drift down and shards of ice shaken off by a swing.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_christophsis_crystal",
        number: 104,
        name: "Christophsis Crystal blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The crystal fields of Christophsis: facets glinting inside a violet blade (or one in its wearer's colour) and crystals growing out of it all round, glinting and sinking back in.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_deflector_shield",
        number: 105,
        name: "Deflector Shield blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A starship's deflector shield, cyan or in its wearer's colour, wrapped round the blade from hilt to tip: a shell of hexagonal cells shimmering round it, hits flashing on it and sending ripples racing across the field.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_healed_kyber",
        number: 106,
        name: "Healed Kyber blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A broken kyber crystal mended with gold: a blade in its wearer's colour crossed by glowing gold seams, a soft healing light breathing round it, rings of gold light rising up it and gold dust drifting off.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_temple_mosaic",
        number: 107,
        name: "Temple Mosaic blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An old Jedi Temple floor as a blade: small tiles of gold, teal and red, each catching the light on its own.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_dagobah_wisps",
        number: 108,
        name: "Dagobah Wisps blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The swamps of Dagobah: warm wisps of light circling the blade and scattering on a swing, a green swamp mist rolling round it and fireflies blinking in the dark.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_nar_shaddaa_rain",
        number: 109,
        name: "Nar Shaddaa Rain blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A rainy night on Nar Shaddaa, the smugglers' moon: pink and blue neon slides along a wet blade while rain slants past it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_navigation_beacon",
        number: 110,
        name: "Navigation Beacon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A hyperspace navigation beacon: a pulse of light climbs the blade every few seconds and rings out from its middle, guiding ships home.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_puzzle_holocron",
        number: 111,
        name: "Puzzle Holocron blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A Jedi holocron's puzzle as a blade: a crystal core in a lattice of panels whose glowing seams lock with a flash as one row after another turns round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_dac_reef",
        number: 112,
        name: "Dac Reef blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The coral reefs of Dac, the Mon Calamari world: the blade is a living branch of rose and sea-green coral, its openings glowing with the light inside and polyps breathing in their rims, in a soft sea haze.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_jakku_sandstorm",
        number: 113,
        name: "Jakku Sandstorm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A sandstorm over Jakku: grit scouring the blade's light, streaks of sand blowing across it, a dust devil whirling round it and a haze of dust rolling off.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_moisture_farm",
        number: 114,
        name: "Moisture Farm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A moisture farm's dew lines: four fine strands running from the hilt to the tip beside the blade, drops of dew sliding down them and glinting, a faint mist condensing round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_tipoca_stormglass",
        number: 115,
        name: "Tipoca Stormglass blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A weather glass from stormy Kamino: feathery crystals form and dissolve inside a sea-green blade, and thin flickers of lightning cross it now and then.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_temple_sunbeam",
        number: 116,
        name: "Temple Sunbeam blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Sunlight through a Jedi Temple window: a warm blade in a shaft of light, soft slanted sunbeams drifting across it, dust floating through the light and motes glinting in the sun.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ion_coil",
        number: 117,
        name: "Ion Coil blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An ion cannon's coil as a blade: rings of the coil pulse up it in turn and ion arcs jump off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sith_thorn",
        number: 118,
        name: "Sith Thorn blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A dark Sith thorn: a deep crimson blade veined with dark power, sharp thorns of light along its sides flaring as a pulse climbs it, a thorned vine coiling round it and red petals drifting down.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_holo_recording",
        number: 119,
        name: "Holo-Recording blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A worn hologram recording: the blade flickers in scan lines with its colours split, a tracking band rolls up it and static crackles beside it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_data_disc",
        number: 120,
        name: "Data Disc blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Spinning data discs: three rings of fine grooves turn round the blade, each with a bright read-light sweeping round it, and packets of data stream up between them.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_cargo_stack",
        number: 121,
        name: "Cargo Stack blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A freighter's hold as a blade: cargo blocks of light stack up it from the hilt, flash when the stack is full and clear, while more drop past, hold lights twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_otoh_gunga",
        number: 122,
        name: "Otoh Gunga blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Gungan bubble city under Naboo's lakes: lake light shimmers through the blade and wobbling bubbles drift up round it, lake haze drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_victory_fireworks",
        number: 123,
        name: "Victory Fireworks blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The night the Empire fell, as a blade: fireworks burst in every colour beside a warm white blade (or one in its wearer's colour) and gold glitter drifts down, embers falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_endor_ivy",
        number: 124,
        name: "Endor Ivy blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The forest moon of Endor: green vines of light climb through the blade and leaves of light drift down from it, pollen twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_naboo_core",
        number: 125,
        name: "Naboo Core blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The glowing waters of Naboo's core: soft pulses of light swell up the blade and ribbons of light trail beside it, an echo of it swaying beside it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_gungan_waters",
        number: 126,
        name: "Gungan Waters blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The clear lakes of the Gungans: light shimmers through the blade as through water and a soft current flows up round it with tiny bubbles, lake mist drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_cantina_lights",
        number: 127,
        name: "Cantina Lights blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A night in a spaceport cantina: the blade cycles through the band's coloured lights and soft spots of light sweep through the smoky air beside it, cantina smoke drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_imperial_searchlight",
        number: 128,
        name: "Imperial Searchlight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An Imperial base's searchlight at night: a bright band of light sweeps up and down the blade through a cold night haze, dust in the beam twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_flare_stick",
        number: 129,
        name: "Flare Stick blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A hand flare from a crashed starfighter's kit: a red-orange blade that sputters and flickers, spitting sparks, with smoke rising off it, embers falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_flimsiplast",
        number: 130,
        name: "Flimsiplast blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Folded flimsiplast as a blade: its light is creased into facets that slowly refold, and small sheets of light drift down from it, flecks of light twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_canto_bight",
        number: 131,
        name: "Canto Bight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The casino city of Canto Bight: lights flash along the blade like a gaming table and a bright ball of light bounces up and down inside it, gold glitter round it, sparkles twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sensor_sweep",
        number: 132,
        name: "Sensor Sweep blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A ship's sensor screen as a blade: a bright sweep runs up it again and again and small contacts light up beside it and fade, sensor grid running through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_takodana_rain",
        number: 133,
        name: "Takodana Rain blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Soft rain on the lake at Takodana: drops land on the blade and send ripples of light running along it and rings rippling out round it, a light rain falling past, lake mist drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_starfighter_canopy",
        number: 134,
        name: "Starfighter Canopy blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Rain on a starfighter's canopy: drops run down the blade, joining and racing each other, while rain is swept past in long slanting streaks, landing lights blink up its sides and droplets twinkle round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_distress_beacon",
        number: 135,
        name: "Distress Beacon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A crashed ship's distress beacon: the blade blinks out its call for help in short and long flashes and orange smoke rises off it, sparks falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_orto_plutonia",
        number: 136,
        name: "Orto Plutonia blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The frozen world of Orto Plutonia: snow swirls round the blade in slow circles, whirled up harder when it swings, and frost tints its light, blowing snow drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_rhen_var_frost",
        number: 137,
        name: "Rhen Var Frost blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The icy ruins of Rhen Var: patches of frost crystals grow on the blade, glitter and melt away, then grow somewhere else, cold mist drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_seismic_charge",
        number: 138,
        name: "Seismic Charge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A seismic charge: the blade holds still and dim, then a silent pulse races up it with a burst of sparks, and again, a shock haze shimmering round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_thermal_fuse",
        number: 139,
        name: "Thermal Fuse blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A thermal detonator's fuse: a white-hot point burns slowly down the blade from the tip to the hilt, spraying sparks, then starts again, embers falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_senate_glass",
        number: 140,
        name: "Senate Glass blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The tall coloured windows of the Galactic Senate: panes of deep red, blue and gold set in dark leading, light slowly moving through them, dust in the light twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_holonet_feed",
        number: 141,
        name: "Holonet Feed blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The HoloNet's endless news feed: dashes of data scroll up through the blade and small flecks of data drift up beside it, data flecks twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_holo_map",
        number: 142,
        name: "Holo-Map blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A tactical hologram map: the blade is drawn in fine wireframe lines that slowly turn, flickering like a projection, with a faint holo glow round it, a projection echo swaying beside it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_cockpit_console",
        number: 143,
        name: "Cockpit Console blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A starship cockpit's consoles: rows of small red, green and amber lights blink along the blade as the systems run, circuits running through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_code_cylinder",
        number: 144,
        name: "Code Cylinder blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An Imperial officer's code cylinder: data stripes of different widths scroll slowly up the blade, data traces running through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_cortosis_weave",
        number: 145,
        name: "Cortosis Weave blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade woven from cortosis threads: a fine weave runs through its light and a glint slides along it now and then, weave threads running through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_tatooine_dust",
        number: 146,
        name: "Tatooine Dust blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Desert dust of Tatooine: a soft, smudged blade with a little dust drifting off it and falling, dust haze drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_trench_run",
        number: 147,
        name: "Trench Run blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The battle station trench run: grey metal panels race up the blade, small lights flashing past on their edges, turbolaser sparks falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ryloth_dawn",
        number: 148,
        name: "Ryloth Dawn blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Dawn over the mountains of Ryloth: warm orange bands of light glow up the blade, soft as sunrise, a warm haze shimmering round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_power_coupling",
        number: 149,
        name: "Power Coupling blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A starship's power coupling: a glowing copper filament winds through the blade and now and then a spark jumps off it, sparks falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_kyber_filament",
        number: 150,
        name: "Kyber Filament blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber filament in glass: a fine zigzag of light glows inside a blade in its wearer's colour and flickers now and then, filament glow running through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_frosted_viewport",
        number: 151,
        name: "Frosted Viewport blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A frosted viewport on an ice world: the blade's light is soft and milky, and a little cold mist sinks off it, frost glints twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_holo_stylus",
        number: 152,
        name: "Holo-Stylus blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A bright holo-stylus stroke: the blade is drawn in streaky, uneven strokes, a brighter stroke running up it again and again, stylus sparks twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sabacc_card",
        number: 153,
        name: "Sabacc Card blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A shimmering sabacc card: rainbow colours shift slowly along the blade as on a card's face, glints twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_geonosis_hive",
        number: 154,
        name: "Geonosis Hive blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The hives of Geonosis: the blade is made of amber cells that slowly brighten and dim, hive dust twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_mygeeto_frost",
        number: 155,
        name: "Mygeeto Frost blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The frozen crystal world of Mygeeto: a frosty blade, pale blue at the hilt and white above, with a drop of melt falling off it now and then, ice glints twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_bogano_moss",
        number: 156,
        name: "Bogano Moss blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The green moss of Bogano: a soft green blade with a living texture and tiny spores of light drifting off it, moss haze drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_kef_bir_sea_glass",
        number: 157,
        name: "Kef Bir Sea Glass blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Sea glass from the stormy oceans of Kef Bir: a translucent sea-green blade with slow waves rolling up it, sea spray drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_jedha_lantern",
        number: 158,
        name: "Jedha Lantern blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pilgrim's lantern in the holy city of Jedha: a warm glow behind thin cloth and ribs, flickering softly, a few embers rising, lantern embers falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_corellian_pearl",
        number: 159,
        name: "Corellian Pearl blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pearl from Corellia's seas: a smooth pale blade with a soft pink and blue sheen moving over it, glints twinkling round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_targeting_display",
        number: 160,
        name: "Targeting Display blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "An old starfighter's targeting display: a blocky blade of chunky lit squares, a few blinking as the computer reads, target grid running through it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_raxus_scrap",
        number: 161,
        name: "Raxus Scrap blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Scrap metal from the junk world of Raxus: dark rusted iron with the blade's colour glowing hot through its cracks, a flake falling now and then, embers falling off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_pasaana_stone",
        number: 162,
        name: "Pasaana Stone blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The layered desert rock of Pasaana: warm bands of stone with light glowing between the layers and a little sand trickling off, dust drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_smoked_transparisteel",
        number: 163,
        name: "Smoked Transparisteel blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Smoked transparisteel: a dark tinted blade with its core light softly showing through, thin dark smoke curling off it, smoke drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_batuu_dusk",
        number: 164,
        name: "Batuu Dusk blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Dusk over Batuu's spires: warm orange at the hilt fading into deep violet at the top, the first stars appearing round it, dusk haze drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_alderaan_velvet",
        number: 165,
        name: "Alderaan Velvet blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The deep crimson velvet of Alderaan's royal court: a soft dark-red blade with a sheen that drifts along it, an echo of it swaying beside it.",
        how_to_get: FROM_HOLOCRONS,
    },
    // Kushina's idea (15/10/2026): the Naboo Blossom's Mythical sibling.
    Unlockable {
        id: "saber_varykino_blossom",
        number: 166,
        name: "Varykino Blossom blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "Spring in the gardens of Varykino: a soft blossom-pink blade with a pearly sheen, wreathed in lavender and white mist, blossoms drifting down round it, a branch of cherry wood in flower at its hilt and a pearl of nacre at its tip. Swing it and it sheds blossoms that flutter to the ground behind it.",
        how_to_get: FROM_HOLOCRONS,
    },
    // 25/10/2026: Sol's Thousand Cranes, the first Unique blade, a hand-given thank-you.
    Unlockable {
        id: "saber_thousand_cranes",
        number: 167,
        name: "Thousand Cranes blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Unique,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A thousand paper cranes, the old gift of thanks: a silk-white blade with a red thread wound round it, origami cranes in coloured paper circling it and a golden crane at its tip. Swing it and cranes take flight.",
        how_to_get: PERSONAL_THANKS,
    },
    // 25/10/2026: Sol's Hearth, the second Unique blade.
    Unlockable {
        id: "saber_hearth",
        number: 168,
        name: "Hearth blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Unique,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A home fire: a warm amber blade of glowing coals with embers rising off it and flames licking up its edges, the air shimmering over it. A ring of hearthstones stands round its emitter, and ash drifts off its swings.",
        how_to_get: PERSONAL_THANKS,
    },
    // 29/10/2026: the mystery boxes' own blade, which only a box gives.
    Unlockable {
        id: "saber_mystery",
        number: 169,
        name: "Mystery blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A blade that never settles on a colour, rolling through them all like a mystery box: gold confetti drifts round it, it flickers as if still deciding, and afterimages trail its swings.",
        how_to_get: FROM_MYSTERY_BOXES,
    },
];

/// The unlockable `id` names (spaces round it and case ignored), if this client knows it.
pub(crate) fn find(id: &str) -> Option<&'static Unlockable> {
    let id = id.trim();
    ALL.iter()
        .find(|unlockable| unlockable.id.eq_ignore_ascii_case(id))
        // The Saber Lab's own blade, while the lab is on.
        .or_else(|| crate::saber_lab::blade(id))
}

/// The blade skin `id` names, if this client knows one by it.
pub(crate) fn blade_skin(id: &str) -> Option<&'static Unlockable> {
    find(id).filter(|unlockable| unlockable.is_blade_skin())
}

/// Whether the blade skin `id` names is a chroma ([`Unlockable::chroma`]).
pub(crate) fn is_chroma(id: &str) -> bool {
    blade_skin(id).is_some_and(|unlockable| unlockable.chroma)
}

/// The blade skins, in catalogue order.
pub(crate) fn blade_skins() -> impl Iterator<Item = &'static Unlockable> {
    ALL.iter().filter(|unlockable| unlockable.is_blade_skin())
}

/// Every blade skin in the order players see them listed: the rarest tier first, then in
/// catalogue order within a tier. A player's own lists leave out what they may not see
/// ([`Holdings::listed`]); the Unique ones come first for those who hold them.
pub(crate) fn blade_skins_by_tier() -> &'static [&'static Unlockable] {
    static ORDER: std::sync::OnceLock<Vec<&'static Unlockable>> = std::sync::OnceLock::new();
    ORDER.get_or_init(|| {
        let mut skins: Vec<_> = blade_skins().collect();
        // A stable sort keeps the catalogue's order within a tier.
        skins.sort_by_key(|skin| std::cmp::Reverse(skin.tier));
        skins
    })
}

/// Wear blade skin `id`, `""` for the stock blade: what the Collection's Equip and
/// Unequip and the Saber tab's blade choice do.
pub(crate) fn wear(console: &mut crate::console::ViewerConsole, id: &str) {
    console.set_cvar(SABER_SKIN_CVAR, id);
}

/// What this client knows of the unlocks the player's own hub profile lists, by the
/// same rule as the gate on what they wear (`player_identity::owns_unlock`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Holdings<'a> {
    /// The identity is off: unlockables need it.
    IdentityOff,
    /// No hub is set (`cl_hubUrl`).
    NoHub,
    /// The hub has not answered yet, or cannot be reached.
    Waiting,
    /// The own profile's unlocks.
    Known(&'a [sjk_identity::Unlock]),
}

impl<'a> Holdings<'a> {
    /// From `cl_identity` (`enabled`) and the identity service's snapshot.
    pub(crate) fn of(enabled: bool, snapshot: Option<&'a sjk_identity::Snapshot>) -> Self {
        use sjk_identity::Status;
        let Some(snapshot) = snapshot.filter(|_| enabled) else {
            return Self::IdentityOff;
        };
        match (&snapshot.status, &snapshot.me) {
            (Status::Disabled, _) => Self::IdentityOff,
            (Status::NoHub, _) => Self::NoHub,
            (_, Some(me)) => Self::Known(&me.unlocks),
            (_, None) => Self::Waiting,
        }
    }

    /// The grant of `id` the profile lists, if it lists it.
    /// Whether `debug_collectibles` dressed these: one of them is a grant the mode added.
    pub(crate) fn debug(&self) -> bool {
        matches!(self, Self::Known(held) if held.iter().any(crate::debug_collectibles::added))
    }

    pub(crate) fn unlock(&self, id: &str) -> Option<&'a sjk_identity::Unlock> {
        match self {
            Self::Known(unlocks) => unlocks.iter().find(|unlock| unlock.id == id),
            _ => None,
        }
    }

    /// Whether the player may wear `unlockable`: a blade skin their profile lists. The
    /// Collection's Equip and the Saber tab's blade choice offer these.
    pub(crate) fn can_wear(&self, unlockable: &Unlockable) -> bool {
        unlockable.is_blade_skin() && self.unlock(unlockable.id).is_some()
    }

    /// The blade skin the player wears: the one `setting` (`cg_saberSkin`) names, while
    /// their profile lists it; `None` for the stock blade.
    pub(crate) fn worn_blade_skin(&self, setting: &str) -> Option<&'static Unlockable> {
        blade_skin(setting).filter(|skin| self.can_wear(skin))
    }

    /// Whether `unlockable` shows to this player wherever the catalogue is listed, counted
    /// or searched (the Collection, the Profile, the Saber tab, `saberskin`): every one,
    /// but one hidden unless held ([`Unlockable::hidden_unless_held`]) only while their
    /// profile lists it. Another player wearing it is still drawn and named as worn.
    pub(crate) fn shows(&self, unlockable: &Unlockable) -> bool {
        !unlockable.hidden_unless_held() || self.unlock(unlockable.id).is_some()
    }

    /// The blade skins this player sees listed, the rarest tier first
    /// ([`blade_skins_by_tier`], kept to [`Self::shows`]): read with each frame's
    /// holdings, so a Unique one appears as soon as it is given and goes when taken back.
    pub(crate) fn listed(self) -> impl Iterator<Item = &'static Unlockable> + 'a {
        blade_skins_by_tier()
            .iter()
            .copied()
            .filter(move |skin| self.shows(skin))
    }

    /// How many blade skins this player sees ([`Self::shows`]): the M of "N of M owned".
    pub(crate) fn listed_count(&self) -> usize {
        blade_skins().filter(|skin| self.shows(skin)).count()
    }

    /// Why what the player holds is not known, in a sentence; `None` once it is.
    pub(crate) fn reason(&self) -> Option<&'static str> {
        match self {
            Self::IdentityOff => {
                Some("Unlockables need the SJK identity: switch it on in Identity settings.")
            }
            Self::NoHub => Some("No hub is set: unlockables are kept on the SJK hub (cl_hubUrl)."),
            Self::Waiting => Some("Waiting for the SJK hub to answer."),
            Self::Known(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_chromas_are_the_skins_drawn_in_any_colour() {
        let chromas: Vec<_> = super::blade_skins()
            .filter(|unlockable| unlockable.chroma)
            .map(|unlockable| unlockable.id)
            .collect();
        assert_eq!(
            chromas,
            [
                "saber_storm",
                "saber_unstable",
                "saber_spectral",
                "saber_glitch",
                "saber_hologram",
                "saber_runic",
                "saber_heartbeat",
                // Creyon's lower tiers that fit a colour (round 14): droids, kyber, holograms,
                // data and light; their effects keep their own colours.
                "saber_circuit",
                "saber_echo",
                "saber_amethyst",
                "saber_neon",
                // Creyon's blades that take a colour only where that makes sense (his round
                // 13: kyber, holograms, the Force, cartoon blades); every other keeps its own.
                // Its pure part takes the colour, its corruption stays red (round 12).
                "saber_bleeding",
                // Its blade takes the colour, its crystals keep theirs (round 13).
                "saber_fiber",
                "saber_comic",
                // Its cut stays black and its eye amber; its lips take the colour (round 14).
                "saber_abyssal_eye",
                // Their blades take the colour, their effects keep theirs (round 14); Between
                // Worlds' duel stays white and red.
                "saber_between_worlds",
                "saber_eclipse",
                "saber_hyperspace",
                "saber_kyber_heart",
                "saber_nebula_song",
                "saber_quasar",
                "saber_singing_kyber",
                "saber_star_dragon",
                "saber_lightspeed_ram",
                "saber_shattered_kyber",
                "saber_hyperlane",
                // Creyon's rares, uncommons and commons that fit a colour (round 14).
                "saber_star_map",
                "saber_christophsis_crystal",
                "saber_deflector_shield",
                "saber_healed_kyber",
                "saber_navigation_beacon",
                "saber_puzzle_holocron",
                "saber_holo_recording",
                "saber_data_disc",
                "saber_cargo_stack",
                "saber_victory_fireworks",
                "saber_flimsiplast",
                "saber_sensor_sweep",
                "saber_holonet_feed",
                "saber_holo_map",
                "saber_code_cylinder",
                "saber_cortosis_weave",
                "saber_kyber_filament",
                "saber_holo_stylus",
                "saber_targeting_display",
            ]
        );
        assert!(super::is_chroma("SABER_GLITCH"));
        assert!(!super::is_chroma("saber_sun"));
        assert!(!super::is_chroma("saber_nothing"));
    }

    use super::*;

    /// Creyon's "Saber ideas" page gave the 21 plain ones (14/10/2026) and his Blade Lab
    /// page the ones after them (15/10/2026); Sol the others.
    #[test]
    fn every_shader_names_its_saber_smith() {
        let creyon: Vec<_> = blade_skins()
            .filter(|skin| skin.smith == CREYON)
            .map(|skin| skin.id)
            .collect();
        assert_eq!(
            creyon,
            [
                "saber_tidal",
                "saber_circuit",
                "saber_aurora",
                "saber_echo",
                "saber_nebula",
                "saber_cinder",
                "saber_venom",
                "saber_sakura",
                "saber_firefly",
                "saber_slate",
                "saber_amber",
                "saber_lilac",
                "saber_mint",
                "saber_coral",
                "saber_ivory",
                "saber_glacier",
                "saber_dusk",
                "saber_ruby",
                "saber_amethyst",
                "saber_jade",
                "saber_moonlight",
                "saber_neon",
                "saber_peppermint",
                "saber_bubble",
                "saber_lavalamp",
                "saber_reactor",
                "saber_bleeding",
                "saber_sketch",
                "saber_hourglass",
                "saber_fiber",
                "saber_serpent",
                "saber_monarch",
                "saber_phoenix",
                "saber_quantum",
                "saber_comic",
                "saber_quicksilver",
                "saber_obsidian",
                // His Blade Lab's signature shaders (19/10/2026).
                "saber_abyssal_eye",
                "saber_aurora_wyrm",
                "saber_balance",
                "saber_between_worlds",
                "saber_big_bang",
                "saber_carbonite",
                "saber_dyson_sphere",
                "saber_eclipse",
                "saber_event_horizon",
                "saber_exogorth",
                "saber_hyperspace",
                "saber_kyber_heart",
                "saber_nebula_song",
                "saber_quasar",
                "saber_singing_kyber",
                "saber_sith_holocron",
                "saber_star_dragon",
                "saber_starforge",
                "saber_superlaser",
                "saber_uneti_tree",
                "saber_witchfire",
                "saber_world_shift",
                "saber_wraithlight",
                "saber_lightspeed_ram",
                "saber_hoth_avalanche",
                "saber_blood_moon",
                "saber_force_echo",
                "saber_hypermatter",
                "saber_kessel_run",
                "saber_unknown_regions",
                "saber_krayt_pearl",
                "saber_starkiller_frost",
                "saber_royal_naboo",
                "saber_bespin_storm",
                "saber_scarif_tide",
                "saber_assassin_droid",
                "saber_aurodium",
                "saber_kashyyyk_monsoon",
                "saber_malachor_forge",
                "saber_yavin_vines",
                "saber_exegol_rift",
                "saber_shattered_kyber",
                "saber_twin_suns",
                "saber_jedi_spirits",
                "saber_ord_mantell_storm",
                "saber_mustafar_eruption",
                "saber_hyperlane",
                "saber_ahch_to_glow",
                "saber_imperial_schematics",
                "saber_jedi_texts",
                "saber_star_map",
                "saber_ilum_frost",
                "saber_christophsis_crystal",
                "saber_deflector_shield",
                "saber_healed_kyber",
                "saber_temple_mosaic",
                "saber_dagobah_wisps",
                "saber_nar_shaddaa_rain",
                "saber_navigation_beacon",
                "saber_puzzle_holocron",
                "saber_dac_reef",
                "saber_jakku_sandstorm",
                "saber_moisture_farm",
                "saber_tipoca_stormglass",
                "saber_temple_sunbeam",
                "saber_ion_coil",
                "saber_sith_thorn",
                "saber_holo_recording",
                "saber_data_disc",
                "saber_cargo_stack",
                "saber_otoh_gunga",
                "saber_victory_fireworks",
                "saber_endor_ivy",
                "saber_naboo_core",
                "saber_gungan_waters",
                "saber_cantina_lights",
                "saber_imperial_searchlight",
                "saber_flare_stick",
                "saber_flimsiplast",
                "saber_canto_bight",
                "saber_sensor_sweep",
                "saber_takodana_rain",
                "saber_starfighter_canopy",
                "saber_distress_beacon",
                "saber_orto_plutonia",
                "saber_rhen_var_frost",
                "saber_seismic_charge",
                "saber_thermal_fuse",
                "saber_senate_glass",
                "saber_holonet_feed",
                "saber_holo_map",
                "saber_cockpit_console",
                "saber_code_cylinder",
                "saber_cortosis_weave",
                "saber_tatooine_dust",
                "saber_trench_run",
                "saber_ryloth_dawn",
                "saber_power_coupling",
                "saber_kyber_filament",
                "saber_frosted_viewport",
                "saber_holo_stylus",
                "saber_sabacc_card",
                "saber_geonosis_hive",
                "saber_mygeeto_frost",
                "saber_bogano_moss",
                "saber_kef_bir_sea_glass",
                "saber_jedha_lantern",
                "saber_corellian_pearl",
                "saber_targeting_display",
                "saber_raxus_scrap",
                "saber_pasaana_stone",
                "saber_smoked_transparisteel",
                "saber_batuu_dusk",
                "saber_alderaan_velvet",
            ]
        );
        assert!(blade_skins().all(|skin| [SOL, CREYON].contains(&skin.smith)));
    }

    /// Sol's tiers (10/10/2026; 26 blades moved 20/10/2026 after Sol's review, 38 more 21/10/2026).
    #[test]
    fn every_shader_has_its_tier_and_the_rarest_list_first() {
        let of = |rarity: Rarity| -> Vec<&str> {
            blade_skins()
                .filter(|skin| skin.tier == rarity)
                .map(|skin| skin.id)
                .collect()
        };
        assert_eq!(
            of(Rarity::Mythical),
            [
                "saber_sun",
                "saber_void",
                "saber_comic",
                "saber_obsidian",
                "saber_abyssal_eye",
                "saber_carbonite",
                "saber_eclipse",
                "saber_event_horizon",
                "saber_exogorth",
                "saber_world_shift",
                "saber_exegol_rift",
                "saber_varykino_blossom",
            ]
        );
        assert_eq!(
            of(Rarity::Legendary),
            [
                "saber_molten",
                "saber_spectral",
                "saber_reactor",
                "saber_bleeding",
                "saber_fiber",
                "saber_serpent",
                "saber_monarch",
                "saber_phoenix",
                "saber_quantum",
                "saber_quicksilver",
                "saber_aurora_wyrm",
                "saber_balance",
                "saber_between_worlds",
                "saber_big_bang",
                "saber_dyson_sphere",
                "saber_hyperspace",
                "saber_nebula_song",
                "saber_quasar",
                "saber_singing_kyber",
                "saber_sith_holocron",
                "saber_starforge",
                "saber_uneti_tree",
                "saber_lightspeed_ram",
                "saber_blood_moon",
                "saber_unknown_regions",
                "saber_starkiller_frost",
                "saber_royal_naboo",
                "saber_bespin_storm",
                "saber_scarif_tide",
                "saber_yavin_vines",
                "saber_shattered_kyber",
                "saber_twin_suns",
                "saber_jedi_spirits",
                "saber_ord_mantell_storm",
                "saber_hyperlane",
                "saber_mystery",
            ]
        );
        assert_eq!(
            of(Rarity::Rare),
            [
                "saber_storm",
                "saber_glitch",
                "saber_hologram",
                "saber_runic",
                "saber_bubble",
                "saber_lavalamp",
                "saber_sketch",
                "saber_hourglass",
                "saber_kyber_heart",
                "saber_star_dragon",
                "saber_superlaser",
                "saber_witchfire",
                "saber_wraithlight",
                "saber_hoth_avalanche",
                "saber_force_echo",
                "saber_hypermatter",
                "saber_kessel_run",
                "saber_krayt_pearl",
                "saber_assassin_droid",
                "saber_aurodium",
                "saber_kashyyyk_monsoon",
                "saber_malachor_forge",
                "saber_mustafar_eruption",
                "saber_ahch_to_glow",
                "saber_imperial_schematics",
                "saber_jedi_texts",
                "saber_star_map",
                "saber_christophsis_crystal",
                "saber_deflector_shield",
                "saber_healed_kyber",
                "saber_nar_shaddaa_rain",
                "saber_navigation_beacon",
                "saber_puzzle_holocron",
                "saber_dac_reef",
                "saber_jakku_sandstorm",
                "saber_moisture_farm",
                "saber_tipoca_stormglass",
                "saber_temple_sunbeam",
                "saber_ion_coil",
                "saber_sith_thorn",
                "saber_holo_recording",
                "saber_data_disc",
                "saber_thermal_fuse",
            ]
        );
        assert_eq!(
            of(Rarity::Uncommon),
            [
                "saber_prism",
                "saber_unstable",
                "saber_heartbeat",
                "saber_cinder",
                "saber_venom",
                "saber_sakura",
                "saber_ruby",
                "saber_amethyst",
                "saber_jade",
                "saber_moonlight",
                "saber_neon",
                "saber_peppermint",
                "saber_cargo_stack",
                "saber_otoh_gunga",
                "saber_victory_fireworks",
                "saber_endor_ivy",
                "saber_naboo_core",
                "saber_gungan_waters",
                "saber_cantina_lights",
                "saber_imperial_searchlight",
                "saber_flare_stick",
                "saber_flimsiplast",
                "saber_canto_bight",
                "saber_sensor_sweep",
                "saber_starfighter_canopy",
                "saber_distress_beacon",
                "saber_rhen_var_frost",
                "saber_seismic_charge",
                "saber_senate_glass",
                "saber_holonet_feed",
                "saber_holo_map",
                "saber_cockpit_console",
                "saber_targeting_display",
            ]
        );
        assert_eq!(
            of(Rarity::Common),
            [
                "saber_frost",
                "saber_chameleon",
                "saber_banner",
                "saber_tidal",
                "saber_circuit",
                "saber_aurora",
                "saber_echo",
                "saber_nebula",
                "saber_firefly",
                "saber_slate",
                "saber_amber",
                "saber_lilac",
                "saber_mint",
                "saber_coral",
                "saber_ivory",
                "saber_glacier",
                "saber_dusk",
                "saber_ilum_frost",
                "saber_temple_mosaic",
                "saber_dagobah_wisps",
                "saber_takodana_rain",
                "saber_orto_plutonia",
                "saber_code_cylinder",
                "saber_cortosis_weave",
                "saber_tatooine_dust",
                "saber_trench_run",
                "saber_ryloth_dawn",
                "saber_power_coupling",
                "saber_kyber_filament",
                "saber_frosted_viewport",
                "saber_holo_stylus",
                "saber_sabacc_card",
                "saber_geonosis_hive",
                "saber_mygeeto_frost",
                "saber_bogano_moss",
                "saber_kef_bir_sea_glass",
                "saber_jedha_lantern",
                "saber_corellian_pearl",
                "saber_raxus_scrap",
                "saber_pasaana_stone",
                "saber_smoked_transparisteel",
                "saber_batuu_dusk",
                "saber_alderaan_velvet",
            ]
        );
        // Sol's thank-you gifts (25/10/2026).
        assert_eq!(
            of(Rarity::Unique),
            ["saber_thousand_cranes", "saber_hearth"]
        );
        let order = blade_skins_by_tier();
        assert_eq!(order.len(), blade_skins().count());
        assert!(order.windows(2).all(|pair| pair[0].tier >= pair[1].tier));
        assert_eq!(order[0].id, "saber_thousand_cranes");
        let unique = of(Rarity::Unique).len();
        assert_eq!(order[unique].id, "saber_sun");
        assert_eq!(
            order[unique + of(Rarity::Mythical).len()].tier,
            Rarity::Legendary
        );
        // The holocrons' names and colours.
        assert_eq!(Rarity::Mythical.label(), "Mythical");
        assert_eq!(Rarity::Uncommon.label(), "Uncommon");
        assert_eq!(Rarity::Common.label(), "Common");
        assert_eq!(Rarity::Rare.colour(), crate::holocrons::TIERS[1].colour);
        assert_eq!(Rarity::Common.symbol(), crate::symbols::RARITY[0]);
        assert_eq!(Rarity::Mythical.symbol(), crate::symbols::RARITY[4]);
    }

    /// Unique is no holocron's tier: its own name, rose and mark, and Mythical's effects.
    #[test]
    fn unique_has_its_own_name_colour_and_mark_and_mythical_effects() {
        assert_eq!(Rarity::Unique.holocron_tier(), None);
        assert_eq!(Rarity::Common.holocron_tier(), None);
        assert_eq!(
            Rarity::Mythical.holocron_tier().map(|tier| tier.id),
            Some("mythical")
        );
        assert_eq!(Rarity::Unique.label(), "Unique");
        assert_eq!(Rarity::Unique.colour(), crate::holocrons::rgb(0xFF_6B9A));
        assert!(
            crate::holocrons::TIERS
                .iter()
                .all(|tier| tier.colour != Rarity::Unique.colour())
        );
        assert_eq!(Rarity::Unique.symbol(), crate::symbols::RARITY[5]);
        assert_eq!(Rarity::Unique.effect_rank(), Rarity::Mythical.effect_rank());
        assert_eq!(Rarity::Mythical.effect_rank(), 3);
        assert!(Rarity::Unique > Rarity::Mythical, "the rarest");
    }

    /// Sol's rule (17/10/2026, and 20/10/2026 for the Blade Lab signatures): every shader
    /// drops from holocrons but those a medal brings (no achievement brings one yet) and
    /// the Unique ones, Sol's thank-you gifts given by hand (25/10/2026), and its "How to
    /// get it" says which. No other shader is handed out by the team instead.
    #[test]
    fn every_shader_but_the_medals_and_the_unique_ones_drops() {
        for skin in blade_skins() {
            let medal = crate::medals::Medal::ALL
                .iter()
                .any(|medal| medal.blade() == Some(skin.id));
            let card = card_bringing(skin.id).is_some();
            let unique = skin.tier == Rarity::Unique;
            // The mystery boxes' own blade (29/10/2026) comes from boxes alone.
            let boxed = skin.id == sjk_identity::mystery_box::BLADE_ID;
            assert!(!(medal && card), "{}: brought twice", skin.id);
            assert!(!(unique && (medal || card)), "{}: given by hand", skin.id);
            assert_eq!(
                skin.droppable,
                !medal && !unique && !boxed,
                "{}: a medal's, a Unique or the boxes' shader never drops, every other one does (a card's too)",
                skin.id
            );
            assert_eq!(skin.hidden_unless_held(), unique, "{}", skin.id);
            if unique {
                assert_eq!(skin.how_to_get, PERSONAL_THANKS, "{}", skin.id);
                assert!(!crate::economy::in_trade(skin), "{}: never traded", skin.id);
            }
            if card {
                assert!(skin.how_to_get.contains("card"), "{}", skin.id);
            }
            if boxed {
                assert_eq!(skin.how_to_get, FROM_MYSTERY_BOXES);
                assert!(!crate::economy::in_trade(skin), "{}: never traded", skin.id);
            }
            assert_eq!(
                skin.how_to_get
                    .starts_with(FROM_HOLOCRONS.trim_end_matches('.')),
                skin.droppable,
                "{}",
                skin.id
            );
        }
    }

    #[test]
    fn every_card_blade_is_a_blade_skin() {
        for (blade, card) in CARD_BLADES {
            assert!(blade_skin(blade).is_some(), "{blade}");
            assert!(!card.is_empty());
        }
        assert_eq!(card_bringing("saber_sun"), Some("sjk_alpha_000"));
        assert_eq!(card_bringing("saber_glitch"), None);
    }

    #[test]
    fn ids_are_unique_and_every_blade_skin_has_one_entry() {
        for (index, unlockable) in ALL.iter().enumerate() {
            assert!(
                ALL[index + 1..]
                    .iter()
                    .all(|other| !other.id.eq_ignore_ascii_case(unlockable.id)),
                "{} is listed twice",
                unlockable.id
            );
            assert!(
                unlockable
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{} is a hub id",
                unlockable.id
            );
            assert!(!unlockable.name.is_empty() && !unlockable.description.is_empty());
            assert!(unlockable.how_to_get.ends_with('.'));
        }
        assert_eq!(
            blade_skins().count(),
            ALL.iter().filter(|u| u.kind == Kind::BladeSkin).count()
        );
    }

    #[test]
    fn holdings_follow_the_identity_and_the_own_profile() {
        use sjk_identity::{Profile, Snapshot, Status, Unlock};
        let unlock = Unlock {
            holo: 0,
            solar: 0,
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "Thanks".into(),
            count: 1,
            medal: None,
            card: None,
        };
        let me = Profile {
            tag_cards: Vec::new(),
            credits: 0,
            cards: Vec::new(),
            keys: Vec::new(),
            key_id: "0123456789abcdef".into(),
            key: String::new(),
            name: "Sol".into(),
            banner: "".into(),
            bio: String::new(),
            verified: false,
            tick: sjk_identity::Tick::None,
            staff: false,
            creator: false,
            jkr_creator: false,
            trust: None,
            emoji: String::new(),
            created: 0,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            unlocks: vec![unlock.clone()],
            avatar: String::new(),
            holocron_counts: Default::default(),
            holocrons: Vec::new(),
            shards: 0,
        };
        let snapshot = |status, me| Snapshot {
            loot: Vec::new(),
            min_client: None,
            opening: None,
            reveal: None,
            local_key_id: "0123456789abcdef".into(),
            status,
            key_id: "0123456789abcdef".into(),
            me,
            server: None,
            map_blocks: Vec::new(),
            players: Vec::new(),
            profiles: std::collections::HashMap::new(),
            model_checks: std::collections::HashMap::new(),
            profiles_revision: 0,
            me_revision: 0,
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            avatar: None,
            look_outcome: None,
            packs_revision: 0,
            map_edits_revision: 0,
            map_edits_note: None,
            map_edits_published: None,
            mystery_boxes: None,
            mystery_boxes_revision: 0,
            mystery_box: None,
            junk: None,
            junk_sale: None,
            box_hunt: None,
            quests: None,
            secrets: None,
            secrets_revision: 0,
            secret_shows: None,
            secret_shows_revision: 0,
            secret: None,
            assets_note: None,
            crashes_sent: 0,
            crash_note: None,
            holocrons: None,
            log_requests: Vec::new(),
            log_answer: None,
            economy: None,
            kyber: None,
            shop: None,
            trade: None,
            tag_trade: None,
            marks: None,
            marks_revision: 0,
            mark_refusal: None,
            trades: None,
            history: None,
            announcements: Vec::new(),
            announcements_revision: 0,
            rewards: None,
            rewards_revision: 0,
            claim: None,
            announce: None,
            news_image: None,
            public_key: String::new(),
        };
        let online = snapshot(Status::Online, Some(me));
        assert_eq!(Holdings::of(false, Some(&online)), Holdings::IdentityOff);
        assert_eq!(Holdings::of(true, None), Holdings::IdentityOff);
        let known = Holdings::of(true, Some(&online));
        assert_eq!(known.unlock("saber_sun"), Some(&unlock));
        assert_eq!(known.unlock("saber_moon"), None);
        assert_eq!(known.reason(), None);
        let waiting = snapshot(Status::Registering, None);
        assert_eq!(Holdings::of(true, Some(&waiting)), Holdings::Waiting);
        let no_hub = snapshot(Status::NoHub, None);
        assert_eq!(Holdings::of(true, Some(&no_hub)), Holdings::NoHub);
        assert!(
            Holdings::IdentityOff
                .reason()
                .unwrap()
                .contains("SJK identity")
        );
        assert_eq!(Holdings::Waiting.unlock("saber_sun"), None);
    }

    /// Sol's rule (25/10/2026): a Unique shader shows only to its holders. Not held, it is
    /// neither listed nor counted; held, it is listed first (the rarest) and counted.
    #[test]
    fn a_unique_shader_shows_only_to_its_holders() {
        let cranes = find("saber_thousand_cranes").unwrap();
        let grant = |id: &str| sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: id.into(),
            granted: 1_793_000_000,
            note: String::new(),
            count: 1,
            medal: None,
            card: None,
        };
        let total = blade_skins().count();
        let unique = blade_skins()
            .filter(|skin| skin.hidden_unless_held())
            .count();
        let sun = [grant("saber_sun")];
        for holdings in [
            Holdings::Known(&[]),
            Holdings::Known(&sun),
            Holdings::IdentityOff,
            Holdings::NoHub,
            Holdings::Waiting,
        ] {
            assert!(!holdings.shows(cranes), "{holdings:?}");
            assert!(holdings.listed().all(|skin| skin.id != cranes.id));
            assert_eq!(holdings.listed().count(), total - unique);
            assert_eq!(holdings.listed_count(), total - unique, "not counted");
            assert_eq!(
                holdings.listed().next().map(|skin| skin.id),
                Some("saber_sun")
            );
        }
        let held = [grant("saber_sun"), grant("saber_thousand_cranes")];
        let holder = Holdings::Known(&held);
        assert!(holder.shows(cranes));
        assert_eq!(
            holder.listed().next().map(|skin| skin.id),
            Some("saber_thousand_cranes"),
            "the rarest first"
        );
        assert_eq!(holder.listed_count(), total - unique + 1);
        // Every other shader shows to everyone, held or not.
        assert!(
            blade_skins()
                .filter(|skin| !skin.hidden_unless_held())
                .all(|skin| Holdings::Known(&[]).shows(skin))
        );
    }

    /// The hub's catalogue (`src/unlocks.rs` in Sol-Vulpes/SJK-hub) lists exactly these, in
    /// this order and with these names; its test pins the same list, so a change to one
    /// side fails until the other follows.
    /// Numbers are the blades' names in notes: each its own, from 1, and given in the
    /// order blades came, so a new one appended takes the next.
    #[test]
    fn every_blade_has_its_own_number_in_catalogue_order() {
        let numbers: Vec<u16> = blade_skins().map(|skin| skin.number).collect();
        let expected: Vec<u16> = (1..=numbers.len() as u16).collect();
        assert_eq!(numbers, expected);
        assert_eq!(
            blade_skin_numbered(1).map(|skin| skin.id),
            Some("saber_sun")
        );
        assert_eq!(blade_skin_numbered(0), None);
        assert_eq!(ALL[41].number_text(), "042");
    }

    /// `docs/unlockables.md` lists every blade by number, the list Sol reviews them from.
    #[test]
    fn the_docs_number_every_blade() {
        let docs = include_str!("../../../docs/unlockables.md");
        for skin in blade_skins() {
            let row = format!("| {} | `{}` | {} |", skin.number_text(), skin.id, skin.name);
            assert!(docs.contains(&row), "docs/unlockables.md lacks {row}");
        }
    }

    #[test]
    fn a_blade_is_named_by_its_id_or_its_number() {
        let sun = Some("saber_sun");
        assert_eq!(blade_skin_named("saber_sun").map(|skin| skin.id), sun);
        assert_eq!(blade_skin_named("1").map(|skin| skin.id), sun);
        assert_eq!(blade_skin_named("001").map(|skin| skin.id), sun);
        assert_eq!(blade_skin_named("#1").map(|skin| skin.id), sun);
        assert_eq!(blade_skin_named("9999"), None);
        assert_eq!(blade_skin_named("#"), None);
    }

    #[test]
    fn the_catalogue_matches_the_hubs() {
        let listed: Vec<(&str, &str)> = ALL.iter().map(|u| (u.id, u.name)).collect();
        assert_eq!(
            listed,
            [
                ("saber_sun", "Sun blade"),
                ("saber_storm", "Storm blade"),
                ("saber_void", "Void blade"),
                ("saber_frost", "Frost blade"),
                ("saber_prism", "Prism blade"),
                ("saber_unstable", "Unstable blade"),
                ("saber_molten", "Molten blade"),
                ("saber_spectral", "Spectral blade"),
                ("saber_glitch", "Glitch blade"),
                ("saber_hologram", "Hologram blade"),
                ("saber_runic", "Runic blade"),
                ("saber_chameleon", "Chameleon blade"),
                ("saber_banner", "Banner blade"),
                ("saber_heartbeat", "Heartbeat blade"),
                ("saber_tidal", "Kamino blade"),
                ("saber_circuit", "Droid Logic blade"),
                ("saber_aurora", "Hoth Aurora blade"),
                ("saber_echo", "Echo Base blade"),
                ("saber_nebula", "Nebula blade"),
                ("saber_cinder", "Mustafar Ember blade"),
                ("saber_venom", "Geonosis Acid blade"),
                ("saber_sakura", "Naboo Blossom blade"),
                ("saber_firefly", "Endor Lights blade"),
                ("saber_slate", "Durasteel blade"),
                ("saber_amber", "Tatooine Gold blade"),
                ("saber_lilac", "Lothal Twilight blade"),
                ("saber_mint", "Yavin Jungle blade"),
                ("saber_coral", "Mon Cala Coral blade"),
                ("saber_ivory", "Jedi Temple blade"),
                ("saber_glacier", "Ilum blade"),
                ("saber_dusk", "Binary Sunset blade"),
                ("saber_ruby", "Sith Ruby blade"),
                ("saber_amethyst", "Kyber Amethyst blade"),
                ("saber_jade", "Dagobah Jade blade"),
                ("saber_moonlight", "Ahch-To Moonlight blade"),
                ("saber_neon", "Coruscant Neon blade"),
                ("saber_peppermint", "Kessel Spice blade"),
                ("saber_bubble", "Bacta blade"),
                ("saber_lavalamp", "Coaxium blade"),
                ("saber_reactor", "Reactor blade"),
                ("saber_bleeding", "Bleeding Kyber blade"),
                ("saber_sketch", "Death Star Plans blade"),
                ("saber_hourglass", "Beskar Forge blade"),
                ("saber_fiber", "Kyber Cluster blade"),
                ("saber_serpent", "Sith Alchemy blade"),
                ("saber_monarch", "Felucia Spores blade"),
                ("saber_phoenix", "Phoenix Squadron blade"),
                ("saber_quantum", "Mortis blade"),
                ("saber_comic", "Clone Wars blade"),
                ("saber_quicksilver", "Liquid Beskar blade"),
                ("saber_obsidian", "Obsidian Edge blade"),
                // Not in the hub yet: its catalogue follows when the team adds them.
                ("saber_abyssal_eye", "Abyssal Eye blade"),
                ("saber_aurora_wyrm", "Ilum Aurora Storm blade"),
                ("saber_balance", "Balance blade"),
                ("saber_between_worlds", "Between Worlds blade"),
                ("saber_big_bang", "Big Bang blade"),
                ("saber_carbonite", "Carbonite blade"),
                ("saber_dyson_sphere", "Dyson Sphere blade"),
                ("saber_eclipse", "Eclipse blade"),
                ("saber_event_horizon", "Event Horizon blade"),
                ("saber_exogorth", "Exogorth blade"),
                ("saber_hyperspace", "Hyperspace blade"),
                ("saber_kyber_heart", "Kyber Heart blade"),
                ("saber_nebula_song", "Nebula Song blade"),
                ("saber_quasar", "Quasar blade"),
                ("saber_singing_kyber", "Singing Kyber blade"),
                ("saber_sith_holocron", "Sith Holocron blade"),
                ("saber_star_dragon", "Ashla Radiance blade"),
                ("saber_starforge", "Starforge blade"),
                ("saber_superlaser", "Superlaser blade"),
                ("saber_uneti_tree", "Uneti Tree blade"),
                ("saber_witchfire", "Witchfire blade"),
                ("saber_world_shift", "World Shift blade"),
                ("saber_wraithlight", "Wraithlight blade"),
                ("saber_lightspeed_ram", "Holdo Maneuver blade"),
                ("saber_hoth_avalanche", "Hoth Avalanche blade"),
                ("saber_blood_moon", "Dathomir Blood Moon blade"),
                ("saber_force_echo", "Force Echo blade"),
                ("saber_hypermatter", "Hypermatter blade"),
                ("saber_kessel_run", "Kessel Run blade"),
                ("saber_unknown_regions", "Unknown Regions blade"),
                ("saber_krayt_pearl", "Krayt Pearl blade"),
                ("saber_starkiller_frost", "Starkiller Frost blade"),
                ("saber_royal_naboo", "Royal Naboo blade"),
                ("saber_bespin_storm", "Bespin Storm blade"),
                ("saber_scarif_tide", "Scarif Tide blade"),
                ("saber_assassin_droid", "Assassin Droid blade"),
                ("saber_aurodium", "Aurodium blade"),
                ("saber_kashyyyk_monsoon", "Kashyyyk Monsoon blade"),
                ("saber_malachor_forge", "Malachor Forge blade"),
                ("saber_yavin_vines", "Yavin Vines blade"),
                ("saber_exegol_rift", "Exegol Rift blade"),
                ("saber_shattered_kyber", "Shattered Kyber blade"),
                ("saber_twin_suns", "Twin Suns blade"),
                ("saber_jedi_spirits", "Jedi Spirits blade"),
                ("saber_ord_mantell_storm", "Ord Mantell Thunderhead blade"),
                ("saber_mustafar_eruption", "Mustafar Eruption blade"),
                ("saber_hyperlane", "Hyperlane blade"),
                ("saber_ahch_to_glow", "Ahch-To Glow blade"),
                ("saber_imperial_schematics", "Imperial Schematics blade"),
                ("saber_jedi_texts", "Jedi Texts blade"),
                ("saber_star_map", "Star Map blade"),
                ("saber_ilum_frost", "Ilum Frost blade"),
                ("saber_christophsis_crystal", "Christophsis Crystal blade"),
                ("saber_deflector_shield", "Deflector Shield blade"),
                ("saber_healed_kyber", "Healed Kyber blade"),
                ("saber_temple_mosaic", "Temple Mosaic blade"),
                ("saber_dagobah_wisps", "Dagobah Wisps blade"),
                ("saber_nar_shaddaa_rain", "Nar Shaddaa Rain blade"),
                ("saber_navigation_beacon", "Navigation Beacon blade"),
                ("saber_puzzle_holocron", "Puzzle Holocron blade"),
                ("saber_dac_reef", "Dac Reef blade"),
                ("saber_jakku_sandstorm", "Jakku Sandstorm blade"),
                ("saber_moisture_farm", "Moisture Farm blade"),
                ("saber_tipoca_stormglass", "Tipoca Stormglass blade"),
                ("saber_temple_sunbeam", "Temple Sunbeam blade"),
                ("saber_ion_coil", "Ion Coil blade"),
                ("saber_sith_thorn", "Sith Thorn blade"),
                ("saber_holo_recording", "Holo-Recording blade"),
                ("saber_data_disc", "Data Disc blade"),
                ("saber_cargo_stack", "Cargo Stack blade"),
                ("saber_otoh_gunga", "Otoh Gunga blade"),
                ("saber_victory_fireworks", "Victory Fireworks blade"),
                ("saber_endor_ivy", "Endor Ivy blade"),
                ("saber_naboo_core", "Naboo Core blade"),
                ("saber_gungan_waters", "Gungan Waters blade"),
                ("saber_cantina_lights", "Cantina Lights blade"),
                ("saber_imperial_searchlight", "Imperial Searchlight blade"),
                ("saber_flare_stick", "Flare Stick blade"),
                ("saber_flimsiplast", "Flimsiplast blade"),
                ("saber_canto_bight", "Canto Bight blade"),
                ("saber_sensor_sweep", "Sensor Sweep blade"),
                ("saber_takodana_rain", "Takodana Rain blade"),
                ("saber_starfighter_canopy", "Starfighter Canopy blade"),
                ("saber_distress_beacon", "Distress Beacon blade"),
                ("saber_orto_plutonia", "Orto Plutonia blade"),
                ("saber_rhen_var_frost", "Rhen Var Frost blade"),
                ("saber_seismic_charge", "Seismic Charge blade"),
                ("saber_thermal_fuse", "Thermal Fuse blade"),
                ("saber_senate_glass", "Senate Glass blade"),
                ("saber_holonet_feed", "Holonet Feed blade"),
                ("saber_holo_map", "Holo-Map blade"),
                ("saber_cockpit_console", "Cockpit Console blade"),
                ("saber_code_cylinder", "Code Cylinder blade"),
                ("saber_cortosis_weave", "Cortosis Weave blade"),
                ("saber_tatooine_dust", "Tatooine Dust blade"),
                ("saber_trench_run", "Trench Run blade"),
                ("saber_ryloth_dawn", "Ryloth Dawn blade"),
                ("saber_power_coupling", "Power Coupling blade"),
                ("saber_kyber_filament", "Kyber Filament blade"),
                ("saber_frosted_viewport", "Frosted Viewport blade"),
                ("saber_holo_stylus", "Holo-Stylus blade"),
                ("saber_sabacc_card", "Sabacc Card blade"),
                ("saber_geonosis_hive", "Geonosis Hive blade"),
                ("saber_mygeeto_frost", "Mygeeto Frost blade"),
                ("saber_bogano_moss", "Bogano Moss blade"),
                ("saber_kef_bir_sea_glass", "Kef Bir Sea Glass blade"),
                ("saber_jedha_lantern", "Jedha Lantern blade"),
                ("saber_corellian_pearl", "Corellian Pearl blade"),
                ("saber_targeting_display", "Targeting Display blade"),
                ("saber_raxus_scrap", "Raxus Scrap blade"),
                ("saber_pasaana_stone", "Pasaana Stone blade"),
                ("saber_smoked_transparisteel", "Smoked Transparisteel blade"),
                ("saber_batuu_dusk", "Batuu Dusk blade"),
                ("saber_alderaan_velvet", "Alderaan Velvet blade"),
                ("saber_varykino_blossom", "Varykino Blossom blade"),
                ("saber_thousand_cranes", "Thousand Cranes blade"),
                ("saber_hearth", "Hearth blade"),
                ("saber_mystery", "Mystery blade"),
            ]
        );
        assert!(ALL.iter().all(Unlockable::is_blade_skin));
        // The blade skins fit the renderer's slots, every one loaded at once.
        assert!(blade_skins().count() <= crate::saber_skins::MAX_SKINS);
    }

    #[test]
    fn ids_are_found_whatever_the_case_and_spaces() {
        assert_eq!(find("saber_sun").map(|u| u.name), Some("Sun blade"));
        assert_eq!(blade_skin(" SABER_Sun ").map(|u| u.id), Some("saber_sun"));
        assert!(blade_skin("saber_sun").is_some_and(Unlockable::is_blade_skin));
        assert!(find("saber_moon").is_none());
        assert!(find("").is_none());
    }
}
