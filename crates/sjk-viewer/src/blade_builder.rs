//! The Saber Lab's builder (`docs/client.md`, "Saber Lab"): every effect a blade-skin file
//! can draw, named, sorted by kind and said in a sentence ([`EFFECTS`]), and how a skin's
//! effects sit in its groups ([`Shape`]): the blade itself (the sections every skin has),
//! the effects always on, and the parts its wearer may switch off (the file's `options`,
//! at most [`MAX_GROUPS`] of [`MAX_IN_GROUP`] each). Moving an effect between them only
//! changes the `options` ([`moved`]); the files are the editor's
//! ([`crate::blade_editor::Editor`]).

use crate::blade_skin_file::extras::{MAX_OPTIONS, Section, SkinOption, option_bit};

/// Most groups a skin has, and most effects in one.
pub(crate) const MAX_GROUPS: usize = MAX_OPTIONS;
pub(crate) const MAX_IN_GROUP: usize = 4;
/// The longest group name.
const NAME_MAX: usize = 24;

/// What an effect is, the library's tabs in their order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// Round the blade: arcs, motes, smoke rising, a glint.
    Around,
    /// On the blade itself: its colours, lines and patterns.
    Along,
    /// Out of the emitter.
    Emitter,
    /// A figure drawn in place of the plain blade.
    Figure,
    /// The blade's own make: every skin has these, and they are never grouped.
    Core,
}

impl Kind {
    pub(crate) const ALL: [Self; 5] = [
        Self::Around,
        Self::Along,
        Self::Emitter,
        Self::Figure,
        Self::Core,
    ];

    /// Its tab's name in the library.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Around => "Around",
            Self::Along => "On the blade",
            Self::Emitter => "Emitter",
            Self::Figure => "Figures",
            Self::Core => "Core",
        }
    }
}

/// One effect: its section in the file, its name, its kind, a few words and a sentence or
/// two on what it draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Effect {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    pub(crate) short: &'static str,
    pub(crate) about: &'static str,
}

const fn effect(
    key: &'static str,
    name: &'static str,
    kind: Kind,
    short: &'static str,
    about: &'static str,
) -> Effect {
    Effect {
        key,
        name,
        kind,
        short,
        about,
    }
}

/// Every effect, by kind, in the order the library shows them (`docs/unlockables.md`,
/// "Blade-skin files", says each field).
pub(crate) const EFFECTS: [Effect; 48] = [
    effect(
        "arcs",
        "Lightning arcs",
        Kind::Around,
        "Lightning along the blade",
        "Lightning strikes crawling along the blade; some leap off the tip into the air.",
    ),
    effect(
        "motes",
        "Motes",
        Kind::Around,
        "Specks drifting off it",
        "Sparks, specks or shards drifting along the blade and out from it, twinkling.",
    ),
    effect(
        "embers",
        "Falling embers",
        Kind::Around,
        "Embers dripping off",
        "Embers drip off the blade and fall, faster as they go.",
    ),
    effect(
        "wisps",
        "Wisps",
        Kind::Around,
        "Smoke rising off it",
        "Thin smoke curling off the blade and rising, whichever way it is held.",
    ),
    effect(
        "haze",
        "Heat haze",
        Kind::Around,
        "The air shimmers",
        "What is behind the blade shimmers, the air bent as by rising heat.",
    ),
    effect(
        "echo",
        "Echo",
        Kind::Around,
        "A copy swaying beside it",
        "One faint copy of the blade's glow beside it, swaying slowly, there even while the blade is held still.",
    ),
    effect(
        "ghosts",
        "Afterimages",
        Kind::Around,
        "Copies left by a swing",
        "Copies of the blade's glow left behind as it moves, each fainter than the last.",
    ),
    effect(
        "star",
        "Tip glint",
        Kind::Around,
        "A star at the tip",
        "A star of rays at the tip, turning slowly and twinkling. Not with black holes.",
    ),
    effect(
        "orbit",
        "Orbit",
        Kind::Around,
        "Glints circling it",
        "The tip glint or the black holes circling the blade below the tip, one to four of them. Goes where the glint or the holes are.",
    ),
    effect(
        "holes",
        "Black holes",
        Kind::Around,
        "Holes bending the scene",
        "Small black holes in the glints' places, bending the scene behind them round a bright ring. Not with a tip glint.",
    ),
    effect(
        "hue",
        "Colour turn",
        Kind::Along,
        "Colours turning round",
        "The blade's colours turn round the colour wheel with time, along the blade and outward.",
    ),
    effect(
        "sputter",
        "Frayed edge",
        Kind::Along,
        "A ragged, sputtering edge",
        "Noise frays the glow's edge, and now and then the blade sputters shorter.",
    ),
    effect(
        "glitch",
        "Glitch",
        Kind::Along,
        "Blocks jumping aside",
        "Blocks of the blade jump sideways and flash, its red and blue splitting from its green.",
    ),
    effect(
        "scan",
        "Hologram",
        Kind::Along,
        "Scan lines and a wireframe",
        "Scan lines running along the blade over a hologram's wireframe rings and side lines, jittering now and then.",
    ),
    effect(
        "pulse",
        "Heartbeat",
        Kind::Along,
        "A beating pulse",
        "The blade brightens and swells in a heartbeat's double beat.",
    ),
    effect(
        "veins",
        "Veins",
        Kind::Along,
        "Glowing veins inside",
        "Glowing veins running through the glow's inside, and through the core when it says so.",
    ),
    effect(
        "glyphs",
        "Glyphs",
        Kind::Along,
        "The wearer's name scrolling",
        "The wearer's name scrolling up the blade, a glyph a letter.",
    ),
    effect(
        "bands",
        "Bands",
        Kind::Along,
        "Rings running up it",
        "Bands round the blade running toward the tip: rings squeezing it, or slanted, a barber's pole.",
    ),
    effect(
        "segments",
        "Segments",
        Kind::Along,
        "A tube in sections",
        "The blade as a tube in sections with dark gaps between, a section dimming now and then.",
    ),
    effect(
        "film",
        "Soap film",
        Kind::Along,
        "Swirling film colours",
        "A soap film's swirling colours over the glow, its inside clearing as its rim brightens.",
    ),
    effect(
        "blobs",
        "Wax blobs",
        Kind::Along,
        "Blobs rising as in a lamp",
        "Wax blobs rising up the blade as in a lava lamp, merging where they touch and pooling at its ends.",
    ),
    effect(
        "front",
        "Climbing colour",
        Kind::Along,
        "A colour rising and falling",
        "A colour climbing the blade from the hilt and falling back, over and over.",
    ),
    effect(
        "team",
        "Team colour",
        Kind::Along,
        "Tinted by the team",
        "The blade takes some of its colour from its wearer's team: red, blue, or neither. Not with the place's light.",
    ),
    effect(
        "ambient",
        "Place's light",
        Kind::Along,
        "Tinted by where it is",
        "The blade takes some of its colour from the light where it is. Not with the team colour.",
    ),
    effect(
        "smoke",
        "Smoke",
        Kind::Emitter,
        "Puffs from the emitter",
        "Puffs of smoke from the emitter, strung out behind a swing; it smokes on a while after the blade goes out.",
    ),
    effect(
        "ribbon",
        "Smoke ribbons",
        Kind::Emitter,
        "Soft trailing strands",
        "Smoke as long soft strands trailing from the emitter, swaying and curling as they rise.",
    ),
    effect(
        "sparks",
        "Sparks",
        Kind::Emitter,
        "Bursts of falling sparks",
        "Bursts of sparks crackling out of the emitter and falling, more as it lights; up to three sprays.",
    ),
    effect(
        "bolts",
        "Bolts",
        Kind::Emitter,
        "Jagged electric bolts",
        "Jagged electric bolts struck from the emitter, crackling into a new shape every frame.",
    ),
    effect(
        "unrest",
        "Unrest",
        Kind::Emitter,
        "Wild at the emitter",
        "The blade's grain, flares, tongues and arcs are wild at the emitter and calm toward the tip; a swing or a clash stirs them further.",
    ),
    effect(
        "flash",
        "Stars",
        Kind::Emitter,
        "Crackling stars",
        "Crackling stars at the emitter that shuffle many times a second, the light flickering with them; they surge as it lights and sputter on after it goes out.",
    ),
    effect(
        "crackle",
        "Crackle",
        Kind::Emitter,
        "A buzz and snaps",
        "The emitter heard: a buzz under the hum while it is lit, and a snap each time its bolts strike.",
    ),
    effect(
        "sketch",
        "Pencil sketch",
        Kind::Figure,
        "Drawn in pencil",
        "The blade drawn as a pencil sketch, hatched and redrawn many times a second.",
    ),
    effect(
        "hourglass",
        "Hourglass",
        Kind::Figure,
        "Sand pouring down",
        "Sand pouring through an hourglass the blade becomes, its glass rim glowing.",
    ),
    effect(
        "serpent",
        "Serpent",
        Kind::Figure,
        "Coiled round the blade",
        "A serpent coiled round the blade from hilt to tip, slithering; in front of the blade it hides it.",
    ),
    effect(
        "strands",
        "Glass strands",
        Kind::Figure,
        "Fibres fanning out",
        "Glass fibres fanning out from the hilt, swaying, their tips in slowly turning colours.",
    ),
    effect(
        "butterflies",
        "Butterflies",
        Kind::Figure,
        "Resting and flying off",
        "Butterflies resting on the blade, flying off now and then and coming back.",
    ),
    effect(
        "wings",
        "Wings of fire",
        Kind::Figure,
        "Fire unfolding",
        "Wings of fire unfolding from near the hilt, white-hot where they overlap.",
    ),
    effect(
        "copies",
        "Ghost copies",
        Kind::Figure,
        "Copies fanning out",
        "Ghost copies of the blade fanning out beside it, collapsing back with a flash.",
    ),
    effect(
        "ink",
        "Ink",
        Kind::Figure,
        "A comic's inked body",
        "An inked body over the blade: a comic's black outline, a glossy streak and halftone dots. Drawn over the glow, so it can be dark.",
    ),
    effect(
        "chrome",
        "Liquid chrome",
        Kind::Figure,
        "A rippling mirror",
        "A liquid mirror over the blade, rippling, beads rolling up it. Drawn over the glow, so it can be dark.",
    ),
    effect(
        "core",
        "Core",
        Kind::Core,
        "The hot line inside",
        "The hot white line down the middle of the blade and its coloured fringe.",
    ),
    effect(
        "corona",
        "Glow",
        Kind::Core,
        "The coloured glow",
        "The coloured glow round the core: its rim, its inside, how bright and how wide.",
    ),
    effect(
        "granulation",
        "Grain",
        Kind::Core,
        "Noise along it",
        "Noise drifting along the blade, heating and widening the glow in patches.",
    ),
    effect(
        "flares",
        "Flares",
        Kind::Core,
        "Bright patches running",
        "Bright patches running along the blade from the hilt past the tip.",
    ),
    effect(
        "shimmer",
        "Shimmer",
        Kind::Core,
        "The glow breathing",
        "The glow breathing wider and narrower, in up to two waves.",
    ),
    effect(
        "tongues",
        "Flame tongues",
        Kind::Core,
        "Tongues off its edge",
        "Flickering tongues licking off the glow's edge and round the tip.",
    ),
    effect(
        "glow_profile",
        "Glow shape",
        Kind::Core,
        "How the glow fades out",
        "How wide the glow is and how it fades out, with an optional wider tail.",
    ),
    effect(
        "core_profile",
        "Core shape",
        Kind::Core,
        "How wide the core is",
        "How wide the hot centre and its coloured fringe are.",
    ),
];

/// The effect drawn by section `key`, if the builder shows it.
pub(crate) fn effect_of(key: &str) -> Option<&'static Effect> {
    EFFECTS.iter().find(|effect| effect.key == key)
}

/// The effects of `kind`, in the library's order.
pub(crate) fn of_kind(kind: Kind) -> impl Iterator<Item = &'static Effect> {
    EFFECTS.iter().filter(move |effect| effect.kind == kind)
}

/// Whether `key` may be in a group: a section an option may switch.
pub(crate) fn groupable(key: &str) -> bool {
    Section::named(key).is_some()
}

/// The first file of the skin's (an index of [`crate::blade_skin_file::FILES`]) a client
/// knowing section `key` reads: its set's (`docs/unlockables.md`, "Blade-skin files").
pub(crate) fn set_file(key: &str) -> usize {
    match key {
        "star" | "wisps" | "haze" | "echo" | "options" => 1,
        "orbit" | "holes" => 2,
        "bands" | "segments" | "film" | "blobs" | "front" => 3,
        "sketch" | "hourglass" | "serpent" | "strands" | "butterflies" | "wings" | "copies" => 4,
        "ink" | "chrome" => 5,
        "smoke" | "ribbon" | "sparks" | "bolts" | "unrest" | "flash" | "crackle" | "chroma" => 6,
        _ => 0,
    }
}

/// One group: a part its wearer may switch off.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Group {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) effects: Vec<String>,
}

/// How a skin's effects sit: the blade's own make, the effects always on, the groups, in
/// the file's order (a group's effects in its own).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Shape {
    pub(crate) core: Vec<String>,
    pub(crate) always: Vec<String>,
    pub(crate) groups: Vec<Group>,
}

impl Shape {
    /// The shape of a skin holding sections `present` (its files' members, laid over each
    /// other) with `options`. An orbit goes with its glint or holes, so it is not listed
    /// alone; the settings (trail, light, sounds, chroma) are not effects.
    pub(crate) fn of(present: &[String], options: &[SkinOption]) -> Self {
        let grouped = |key: &str| {
            options
                .iter()
                .any(|option| option.sections.iter().any(|name| name == key))
        };
        let mut shape = Self::default();
        for effect in of_kind(Kind::Core) {
            if present.iter().any(|key| key == effect.key) {
                shape.core.push(effect.key.to_owned());
            }
        }
        shape.always = present
            .iter()
            .filter(|key| groupable(key) && !grouped(key))
            .cloned()
            .collect();
        shape.groups = options
            .iter()
            .map(|option| Group {
                id: option.id.clone(),
                name: option.name.clone(),
                effects: option.sections.clone(),
            })
            .collect();
        shape
    }

    /// The group holding `key`, if one does.
    pub(crate) fn group_of(&self, key: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.effects.iter().any(|effect| effect == key))
    }
}

/// Where an effect may be put: back in the library (taken off the blade), the blade's own
/// make, always on, a group, or a new group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Place {
    Library,
    Core,
    Always,
    Group(usize),
    NewGroup,
}

/// Whether effect `key` may go to `place` on a skin of `shape`, and why not.
pub(crate) fn may_go(shape: &Shape, key: &str, place: Place) -> Result<(), &'static str> {
    let core = effect_of(key).is_some_and(|effect| effect.kind == Kind::Core);
    match place {
        Place::Library if core => Err("Every blade has its own make: it cannot be taken out."),
        Place::Library => Ok(()),
        Place::Core if core => Ok(()),
        Place::Core => Err("Only the blade's own make goes there."),
        _ if core => Err("The blade's own make is always there: it cannot be grouped."),
        _ if !groupable(key) && place != Place::Always => {
            Err("It goes with its glint or holes, wherever they are.")
        }
        Place::Always => Ok(()),
        Place::Group(index) => {
            let group = shape.groups.get(index).ok_or("No such group.")?;
            if group.effects.iter().any(|effect| effect == key) {
                Ok(())
            } else if group.effects.len() >= MAX_IN_GROUP {
                Err("A group holds at most four effects.")
            } else {
                Ok(())
            }
        }
        Place::NewGroup if shape.groups.len() >= MAX_GROUPS => {
            Err("A blade has at most six groups.")
        }
        Place::NewGroup => Ok(()),
    }
}

/// `options` with effect `key` moved to `place` (always on, a group or a new one named
/// after it); taken out of every other group, a group left empty going with it.
pub(crate) fn moved(options: &[SkinOption], key: &str, place: Place) -> Vec<SkinOption> {
    let target = match place {
        Place::Group(index) => options.get(index).map(|option| option.id.clone()),
        _ => None,
    };
    let mut options: Vec<SkinOption> = options
        .iter()
        .cloned()
        .map(|mut option| {
            option.sections.retain(|name| name != key);
            option
        })
        .filter(|option| !option.sections.is_empty() || target.as_ref() == Some(&option.id))
        .collect();
    match place {
        Place::Group(_) => {
            if let Some(option) = options
                .iter_mut()
                .find(|option| Some(&option.id) == target.as_ref())
            {
                option.sections.push(key.to_owned());
            }
        }
        Place::NewGroup => {
            let name = effect_of(key).map_or(key, |effect| effect.name);
            let id = new_id(&options, name);
            options.push(SkinOption {
                id,
                name: name.to_owned(),
                sections: vec![key.to_owned()],
            });
        }
        Place::Library | Place::Core | Place::Always => {}
    }
    options
}

/// `options` without group `index`: its effects always on.
pub(crate) fn ungrouped(options: &[SkinOption], index: usize) -> Vec<SkinOption> {
    let mut options = options.to_vec();
    if index < options.len() {
        options.remove(index);
    }
    options
}

/// `options` with group `index` named `name` (trimmed, at most 24 printable characters;
/// its id, which the wearers' choices keep, stays); `None` when the name is no name.
pub(crate) fn renamed(options: &[SkinOption], index: usize, name: &str) -> Option<Vec<SkinOption>> {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| (' '..='~').contains(c))
        .take(NAME_MAX)
        .collect();
    let name = name.trim();
    if name.is_empty() || index >= options.len() {
        return None;
    }
    let mut options = options.to_vec();
    options[index].name = name.to_owned();
    Some(options)
}

/// A new group's id, from `name`: lower case letters, digits and `_`, not one of
/// `options`' nor on a bit one of theirs has (a number added until it is free).
fn new_id(options: &[SkinOption], name: &str) -> String {
    let mut base: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    base.truncate(NAME_MAX - 3);
    if base.is_empty() {
        base.push_str("group");
    }
    let taken = |id: &str| {
        options
            .iter()
            .any(|option| option.id == id || option_bit(&option.id) == option_bit(id))
    };
    let mut id = base.clone();
    let mut n = 2;
    while taken(&id) {
        id = format!("{base}_{n}");
        n += 1;
    }
    id
}
