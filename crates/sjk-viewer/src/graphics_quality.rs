//! Graphics presets in two families (Sol, 10/11/2026, in place of Vanilla style, SJK
//! (recommended) and Experimental of 16/10/2026):
//!
//! - **Vanilla**: the maps' own baked lighting. *Vanilla* is the original game's look at
//!   the least cost, every SJK extra off: EternalJK's defaults on its own renderer
//!   (rd-eternaljk, rd-vanilla's code: no dynamic glow, no light flares; Vanilla style
//!   before). *Vanilla Plus* is SJK's image (HDR, bloom, glow, FXAA, ambient occlusion,
//!   lit models, emission maps) over the same lighting: SJK (recommended) before, still a
//!   new profile's preset ([`apply_new_profile`]).
//! - **Solar Renderer** (SJK's own renderer, its real-time lighting on: a live sun, sky
//!   and lamps in place of the baked light; Experimental before) at *Low* (everything it
//!   can leave out left out), *Medium*, *High* (the renderer cvars' own defaults, the
//!   look they were tuned for, Experimental's values) and *Ultra* (finer shadows, the
//!   richest weather).
//!
//! Each preset is rated for this PC by its video memory ([`Rating`], the coloured marks
//! in the preset list), and [`recommended`] picks the richest one rated smooth. The
//! preset is not stored: it is whichever preset the settings match, and Custom once one
//! of them is changed on its own. Only settings with a row in Settings are touched, so
//! every change a preset makes can be seen and undone there; the FPS cap, vertical sync,
//! resolution, supersampling and anything that changes gameplay or a map's content (its
//! fog) are left alone.

use crate::console::ViewerConsole;

/// Console command that names or sets the preset (its name is from when the
/// presets were quality levels).
pub(crate) const COMMAND: &str = "graphicsquality";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show or set the graphics preset: vanilla, vanillaplus, low, medium, \
     high, ultra (the Solar Renderer's) or recommended";
/// The Graphics preset row's stand-in for a cvar name: the command that sets it.
pub(crate) const ROW_NAME: &str = COMMAND;
/// Console command that sets the preset recommended for this PC ([`recommended`]); the
/// Settings row under the preset runs it.
pub(crate) const RECOMMEND_COMMAND: &str = "graphicsrecommended";
/// Help text of [`RECOMMEND_COMMAND`].
pub(crate) const RECOMMEND_HELP: &str =
    "Set the graphics preset recommended for this PC's graphics card";
/// Console command that names the renderer or switches Vanilla on and off, keeping the
/// player's settings ([`toggle_vanilla`]).
pub(crate) const RENDERER_COMMAND: &str = "renderer";
/// Help text of [`RENDERER_COMMAND`] for completion and `cmdlist`.
pub(crate) const RENDERER_HELP: &str = "Show or set the renderer: vanilla (the original look, \
     your settings kept) or sjk (SJK's renderer, your settings back)";
/// What turning Vanilla on put aside, `cvar=value` pairs joined by `;`, so
/// turning the SJK renderer on again restores the player's own settings. Its
/// name is from when Vanilla was Ultra low.
pub(crate) const RESTORE_CVAR: &str = "r_ultraLowRestore";
/// The renderer command's two values.
pub(crate) const VANILLA: &str = "vanilla";
pub(crate) const SJK: &str = "sjk";

/// A graphics preset: the Vanilla family, then the Solar Renderer's, each cheapest first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Level {
    /// The original game's look at the least cost (EJK, Ultra low, Vanilla style before):
    /// every SJK extra off, maps show their baked lightmaps.
    Vanilla,
    /// SJK's image over the maps' own lighting: the default (SJK (recommended) before).
    VanillaPlus,
    /// The Solar Renderer with everything it can leave out left out.
    SolarLow,
    /// The Solar Renderer with its lighting, material maps and character shadows.
    SolarMedium,
    /// Everything the Solar Renderer draws at the quality it was tuned for (Experimental
    /// before; the renderer cvars' defaults).
    SolarHigh,
    /// High with the finest shadows and the richest weather.
    SolarUltra,
}

/// Which family a preset is in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Family {
    /// The maps' baked lighting.
    Vanilla,
    /// SJK's real-time lighting.
    Solar,
}

/// How a preset is expected to run on this PC, from its graphics card's video memory
/// ([`Level::rating`]): the colour of its mark in the preset list.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Rating {
    /// Plenty of room: green.
    Smooth,
    /// Enough: yellow.
    Fine,
    /// Tight, it may stutter on large maps: orange.
    Heavy,
    /// More than the card holds: red.
    TooHeavy,
}

impl Rating {
    /// The rating's colour, green to red.
    pub(crate) fn color(self) -> sjk_ui::Color {
        match self {
            Self::Smooth => sjk_ui::Color::new(0.36, 0.84, 0.42, 1.0),
            Self::Fine => sjk_ui::Color::new(0.96, 0.85, 0.3, 1.0),
            Self::Heavy => sjk_ui::Color::new(1.0, 0.58, 0.2, 1.0),
            Self::TooHeavy => sjk_ui::Color::new(0.93, 0.3, 0.27, 1.0),
        }
    }

    /// The rating in words, for the detail column.
    pub(crate) fn words(self) -> &'static str {
        match self {
            Self::Smooth => "runs smoothly on this PC",
            Self::Fine => "should run well on this PC",
            Self::Heavy => "is heavy for this PC",
            Self::TooHeavy => "is too heavy for this PC",
        }
    }
}

/// Each setting a preset sets, with its value at Vanilla, Vanilla Plus, then the Solar
/// Renderer's Low, Medium, High and Ultra. High is each setting's default (pinned by a
/// test); Vanilla Plus is Sol's config of 16/10/2026. Within each family no preset costs
/// less than the one before it (a test). Settings that only the Solar Renderer reads keep
/// their default in the Vanilla family (where they do nothing), so profiles saved on
/// Vanilla style and SJK (recommended) read as Vanilla and Vanilla Plus. Float rows that
/// are dropdowns take their choice's text ("2.0").
const VALUES: &[(&str, [&str; 6])] = &[
    // Image.
    ("r_sceneHdr", ["0", "1", "1", "1", "1", "1"]),
    ("r_sceneBloom", ["0", "1", "1", "1", "1", "1"]),
    ("r_DynamicGlow", ["0", "1", "1", "1", "1", "1"]),
    ("r_fxaa", ["0", "1", "1", "1", "1", "1"]),
    ("r_softParticles", ["0", "0", "0", "1", "1", "1"]),
    // EternalJK's rd-vanilla registers `r_flares` 0; SJK draws them by default.
    ("r_flares", ["0", "1", "1", "1", "1", "1"]),
    ("r_ssao", ["0", "1", "0", "1", "1", "1"]),
    // Dust shows only in light shafts.
    (
        crate::dust_motes::CVAR,
        ["0.0", "0.0", "0.0", "0.0", "1.0", "1.0"],
    ),
    ("r_modelPixelLight", ["0", "1", "1", "1", "1", "1"]),
    ("r_cubeMapping", ["0", "0", "0", "0", "1", "1"]),
    ("r_floorReflections", ["0", "0", "0", "0", "1", "1"]),
    ("r_normalMapping", ["0", "0", "0", "1", "1", "1"]),
    ("r_specularMapping", ["0", "0", "0", "1", "1", "1"]),
    ("r_parallaxMapping", ["0", "0", "0", "0", "1", "1"]),
    ("r_emissiveMaps", ["0", "1", "1", "1", "1", "1"]),
    ("r_emissiveGlow", ["0", "1", "1", "1", "1", "1"]),
    // Lighting.
    ("r_dayNight", ["0", "0", "1", "1", "1", "1"]),
    (
        crate::world_materials::shadows::settings::BOUNCE_CVAR,
        ["1", "1", "0", "1", "1", "1"],
    ),
    (
        crate::world_materials::shadows::settings::LAMP_SHADOWS_CVAR,
        ["1", "1", "0", "0", "1", "1"],
    ),
    ("r_emissiveLights", ["0", "0", "0", "1", "1", "1"]),
    ("r_volumetrics", ["0", "0", "0", "1", "3", "3"]),
    // Shadows.
    ("r_worldSunShadows", ["0", "0", "1", "1", "1", "1"]),
    ("r_actorSunShadows", ["0", "0", "0", "1", "1", "1"]),
    (
        "r_sunShadowResolution",
        ["1024", "1024", "512", "1024", "2048", "4096"],
    ),
    ("r_sunShadowTaps", ["8", "8", "4", "8", "16", "24"]),
    (
        "r_sunShadowDistance",
        ["1024", "1024", "512", "1024", "1024", "2048"],
    ),
    // Weather.
    (
        crate::weather::DENSITY_CVAR,
        ["1.0", "1.0", "1.0", "2.0", "2.0", "3.0"],
    ),
    (crate::weather::QUALITY_CVAR, ["0", "0", "0", "1", "2", "3"]),
    // The volumetric clouds (`r_clouds`) are no preset's: off by default, the player's own
    // choice under every preset (Sol, 05/11/2026).
];

/// The video memory (MiB) each preset wants to run smoothly; [`Level::rating`] compares
/// the card's with it. From the VRAM measurements in `docs/rendering.md` ("GPU memory"):
/// the Vanilla family runs in 1-2 GB, the Solar Renderer's lamp cache, GI and shadow maps
/// add several hundred MiB on large maps, more at each level.
const WANTS_MIB: [u64; 6] = [1024, 2048, 3072, 4096, 6144, 8192];

/// The share of a preset's [`WANTS_MIB`] a card must hold for each rating.
const SMOOTH_SHARE: f64 = 1.25;
const FINE_SHARE: f64 = 1.0;
const HEAVY_SHARE: f64 = 0.7;
/// The card assumed where its video memory cannot be read (outside Windows, or a driver
/// that does not say): a mid-range card, so the marks still order the presets by cost.
const UNKNOWN_CARD_MIB: u64 = 4096;

impl Level {
    pub(crate) const ALL: [Self; 6] = [
        Self::Vanilla,
        Self::VanillaPlus,
        Self::SolarLow,
        Self::SolarMedium,
        Self::SolarHigh,
        Self::SolarUltra,
    ];
    /// A new profile's preset, which the row's Backspace returns to.
    pub(crate) const DEFAULT: Self = Self::VanillaPlus;

    /// The preset's name in Settings.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::VanillaPlus => "Vanilla Plus",
            Self::SolarLow => "Solar Renderer: Low",
            Self::SolarMedium => "Solar Renderer: Medium",
            Self::SolarHigh => "Solar Renderer: High",
            Self::SolarUltra => "Solar Renderer: Ultra",
        }
    }

    /// [`Self::label`] marked as the one [`recommended`] for this PC.
    pub(crate) fn label_recommended(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla (recommended)",
            Self::VanillaPlus => "Vanilla Plus (recommended)",
            Self::SolarLow => "Solar Renderer: Low (recommended)",
            Self::SolarMedium => "Solar Renderer: Medium (recommended)",
            Self::SolarHigh => "Solar Renderer: High (recommended)",
            Self::SolarUltra => "Solar Renderer: Ultra (recommended)",
        }
    }

    /// The preset's family.
    pub(crate) fn family(self) -> Family {
        match self {
            Self::Vanilla | Self::VanillaPlus => Family::Vanilla,
            _ => Family::Solar,
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// The preset's name in the command.
    fn command_name(self) -> &'static str {
        match self {
            Self::Vanilla => VANILLA,
            Self::VanillaPlus => "vanillaplus",
            Self::SolarLow => "low",
            Self::SolarMedium => "medium",
            Self::SolarHigh => "high",
            Self::SolarUltra => "ultra",
        }
    }

    /// The preset `name` names, in any case: its command name or label, with or without
    /// spaces and the colon, or with `solar` before it; the old names too (EJK, Ultra low
    /// and Vanilla style for Vanilla, SJK for Vanilla Plus, Experimental for High).
    fn parse(name: &str) -> Option<Self> {
        let name: String = name.split_whitespace().collect::<String>().replace(':', "");
        let old = [
            ("ultralow", Self::Vanilla),
            ("ejk", Self::Vanilla),
            ("vanillastyle", Self::Vanilla),
            (SJK, Self::VanillaPlus),
            ("sjk(recommended)", Self::VanillaPlus),
            ("experimental", Self::SolarHigh),
        ];
        if let Some((_, level)) = old.iter().find(|(old, _)| name.eq_ignore_ascii_case(old)) {
            return Some(*level);
        }
        Self::ALL.into_iter().find(|level| {
            let command = level.command_name();
            name.eq_ignore_ascii_case(command)
                || name.eq_ignore_ascii_case(&format!("solar{command}"))
                || name.eq_ignore_ascii_case(&level.label().replace([' ', ':'], ""))
        })
    }

    /// The preset a step of `direction` from this one reaches, stopping at the
    /// ends rather than wrapping.
    pub(crate) fn step(self, direction: i32) -> Self {
        let last = Self::ALL.len() as i32 - 1;
        Self::ALL[(self.index() as i32 + direction.signum()).clamp(0, last) as usize]
    }

    /// The preset whose values `console` holds, if every one matches.
    pub(crate) fn current(console: &ViewerConsole) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.matches(console) == VALUES.len())
    }

    /// The current preset, or for custom settings the preset they are nearest,
    /// sharing the most values (the higher on a tie). Steps and lists start here.
    pub(crate) fn nearest(console: &ViewerConsole) -> Self {
        Self::ALL
            .into_iter()
            .max_by_key(|level| level.matches(console))
            .unwrap_or(Self::DEFAULT)
    }

    /// How many of the preset's values `console` holds.
    fn matches(self, console: &ViewerConsole) -> usize {
        VALUES
            .iter()
            .filter(|(cvar, values)| holds(console, cvar, values[self.index()]))
            .count()
    }

    /// Set the preset's values, saving the profile once.
    pub(crate) fn apply(self, console: &mut ViewerConsole) {
        console.set_cvars(
            VALUES
                .iter()
                .map(|(cvar, values)| (*cvar, values[self.index()])),
        );
    }

    /// How the preset should run on a card of `mib` MiB of video memory (`None`: not
    /// known, a mid-range card assumed). Vanilla, the original game's renderer, is never
    /// rated worse than fine.
    pub(crate) fn rating(self, mib: Option<u64>) -> Rating {
        let share = mib.unwrap_or(UNKNOWN_CARD_MIB) as f64 / WANTS_MIB[self.index()] as f64;
        let rating = if share >= SMOOTH_SHARE {
            Rating::Smooth
        } else if share >= FINE_SHARE {
            Rating::Fine
        } else if share >= HEAVY_SHARE {
            Rating::Heavy
        } else {
            Rating::TooHeavy
        };
        if self == Self::Vanilla {
            rating.min(Rating::Fine)
        } else {
            rating
        }
    }
}

/// This PC's graphics card's video memory in MiB, when it can be read.
pub(crate) fn card_mib() -> Option<u64> {
    crate::video_memory::dedicated().map(|bytes| bytes >> 20)
}

/// The preset for a card of `mib` MiB: the richest one rated smooth, Vanilla when none
/// is. Where the card is not known, Vanilla Plus (a new profile's).
pub(crate) fn recommended_for(mib: Option<u64>) -> Level {
    if mib.is_none() {
        return Level::DEFAULT;
    }
    Level::ALL
        .into_iter()
        .rev()
        .find(|level| level.rating(mib) == Rating::Smooth)
        .unwrap_or(Level::Vanilla)
}

/// The preset recommended for this PC ([`recommended_for`] its card).
pub(crate) fn recommended() -> Level {
    recommended_for(card_mib())
}

/// The name the preset list and the Recommended row give `level` on this PC.
pub(crate) fn list_label(level: Level) -> &'static str {
    if level == recommended() {
        level.label_recommended()
    } else {
        level.label()
    }
}

/// What the detail column says of the presets on this PC: the card and the
/// recommendation.
pub(crate) fn card_words() -> String {
    let pick = recommended();
    match card_mib() {
        Some(mib) => format!(
            "This PC's graphics card has {:.1} GB: {} {}. Green runs smoothly, yellow well, \
             orange is heavy, red too heavy.",
            mib as f64 / 1024.0,
            pick.label(),
            pick.rating(Some(mib)).words()
        ),
        None => format!(
            "This PC's video memory is not known: {} is a safe start. The colours assume a \
             4 GB card.",
            pick.label()
        ),
    }
}

/// Whether the renderer is Vanilla: the settings are Vanilla's (the first preset
/// [`Level::current`] tries), read up to the first value that differs.
pub(crate) fn vanilla(console: &ViewerConsole) -> bool {
    let vanilla = Level::Vanilla.index();
    VALUES
        .iter()
        .all(|(cvar, values)| holds(console, cvar, values[vanilla]))
}

/// Set the renderer to `value` ([`VANILLA`] or [`SJK`], any case), as the
/// renderer command does; another value changes nothing.
pub(crate) fn set_renderer(console: &mut ViewerConsole, value: &str) {
    let value = value.trim();
    let to_vanilla = value.eq_ignore_ascii_case(VANILLA);
    if (to_vanilla || value.eq_ignore_ascii_case(SJK)) && to_vanilla != vanilla(console) {
        toggle_vanilla(console);
    }
}

/// Switch between Vanilla and the SJK renderer. To Vanilla, it keeps the
/// player's value of every setting a preset sets, then applies Vanilla style;
/// back, it puts the kept values back (Vanilla Plus when none were kept,
/// as when Vanilla was picked as a preset).
pub(crate) fn toggle_vanilla(console: &mut ViewerConsole) {
    if vanilla(console) {
        let kept = console
            .cvar(RESTORE_CVAR)
            .map(sjk_shell::CvarValue::as_text)
            .unwrap_or_default();
        let restored: Vec<(&str, &str)> = kept
            .split(';')
            .filter_map(|pair| pair.split_once('='))
            .filter(|(cvar, _)| VALUES.iter().any(|(known, _)| known == cvar))
            .chain([(RESTORE_CVAR, "")])
            .collect();
        if restored.len() == 1 {
            Level::DEFAULT.apply(console);
        }
        console.set_cvars(restored);
    } else {
        let kept = VALUES
            .iter()
            .filter_map(|(cvar, _)| Some(format!("{cvar}={}", console.cvar(cvar)?.as_text())))
            .collect::<Vec<_>>()
            .join(";");
        console.set_cvar(RESTORE_CVAR, &kept);
        Level::Vanilla.apply(console);
    }
}

/// Start a new profile on Vanilla Plus.
pub(crate) fn apply_new_profile(console: &mut ViewerConsole) {
    Level::DEFAULT.apply(console);
}

/// What the Graphics preset row shows: the preset, or Custom.
pub(crate) fn shown(console: &ViewerConsole) -> &'static str {
    Level::current(console).map_or("Custom", Level::label)
}

/// The default a renderer setting's row shows and returns to: its value in the
/// preset in use, or for custom settings in the preset they are nearest, so a
/// preset's rows do not read as changed and Backspace leads back to it; `None`
/// for a setting no preset sets, whose default is its cvar's.
pub(crate) fn row_default(console: &ViewerConsole, cvar: &str) -> Option<&'static str> {
    let (_, values) = VALUES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))?;
    Some(values[Level::nearest(console).index()])
}

/// `renderer [vanilla|sjk]`: name the renderer, or set it.
pub(crate) fn renderer_command(
    console: &mut ViewerConsole,
    args: &[String],
) -> Result<Vec<String>, String> {
    let named = |console: &ViewerConsole| {
        if vanilla(console) {
            "Renderer: Vanilla (the original look)".to_owned()
        } else {
            format!(
                "Renderer: SJK, {} graphics; the Solar Renderer can still show glitches, \
                 which the SJK menu's Report a bug sends to the team",
                shown(console)
            )
        }
    };
    match args {
        [] => Ok(vec![named(console)]),
        [value]
            if [VANILLA, SJK, "ejk"]
                .iter()
                .any(|name| value.eq_ignore_ascii_case(name)) =>
        {
            let value = if value.eq_ignore_ascii_case(SJK) {
                SJK
            } else {
                VANILLA
            };
            set_renderer(console, value);
            Ok(vec![
                named(console),
                "It applies after a graphics reload (vid_restart)".to_owned(),
            ])
        }
        _ => Err(format!("usage: {RENDERER_COMMAND} [{VANILLA}|{SJK}]")),
    }
}

/// Whether `cvar` holds `value`: numbers compare as numbers ("1.0" holds "1"), a typed
/// cvar by its own value, without writing it out as text (the low-FPS help asks
/// while a map plays; Tactical mode's start, `tactical.rs`).
pub(crate) fn holds(console: &ViewerConsole, cvar: &str, value: &str) -> bool {
    use sjk_shell::CvarValue;
    let wanted = value.parse::<f64>().ok();
    let near = |current: f64| wanted.is_some_and(|value| (current - value).abs() < 1e-6);
    match console.cvar(cvar) {
        None => false,
        Some(CvarValue::Bool(on)) => near(if *on { 1.0 } else { 0.0 }),
        Some(CvarValue::Integer(current)) => near(*current as f64),
        Some(CvarValue::Float(current)) => near(*current),
        Some(CvarValue::Text(current)) => {
            let current = current.trim();
            match current.parse::<f64>() {
                Ok(current) if wanted.is_some() => near(current),
                _ => current.eq_ignore_ascii_case(value),
            }
        }
    }
}

/// `graphicsquality [preset]`: name the preset, or set it.
pub(crate) fn command(console: &mut ViewerConsole, args: &[String]) -> Result<Vec<String>, String> {
    let names = || Level::ALL.map(Level::command_name).join(", ");
    match args {
        [] => Ok(vec![format!(
            "Graphics preset: {} ({})",
            shown(console),
            names()
        )]),
        [name] => {
            let level = if name.eq_ignore_ascii_case("recommended") {
                recommended()
            } else {
                Level::parse(name).ok_or_else(|| format!("usage: {COMMAND} [{}]", names()))?
            };
            level.apply(console);
            Ok(vec![format!(
                "Graphics preset: {}; shadows, light shafts, HDR, FXAA, real-time lighting, \
                 reflection probes and material maps apply after a graphics reload \
                 (vid_restart), emission-map lights on the next map",
                level.label()
            )])
        }
        _ => Err(format!("usage: {COMMAND} [{}]", names())),
    }
}

/// `graphicsrecommended`: set the preset recommended for this PC ([`recommended`]).
pub(crate) fn recommend_command(console: &mut ViewerConsole) -> Result<Vec<String>, String> {
    command(console, &["recommended".to_owned()])
}

/// The cvars `level` sets: for measurements that build a client at a preset, and
/// Tactical mode's Medium (`tactical.rs`).
pub(crate) fn preset_cvars(level: Level) -> Vec<(&'static str, &'static str)> {
    VALUES
        .iter()
        .map(|(cvar, values)| (*cvar, values[level.index()]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn the_cvars_defaults_are_solar_high() {
        let (_directory, console) = console();
        for (cvar, values) in VALUES {
            let default = console
                .cvar_default(cvar)
                .unwrap_or_else(|| panic!("{cvar} is not registered"))
                .as_text();
            assert_eq!(
                default.parse::<f64>().ok(),
                values[Level::SolarHigh.index()].parse::<f64>().ok(),
                "{cvar}: Solar High is not its default"
            );
        }
        assert_eq!(Level::current(&console), Some(Level::SolarHigh));
        assert_eq!(shown(&console), "Solar Renderer: High");
    }

    #[test]
    fn each_preset_sets_its_values_and_reads_back() {
        let (directory, mut console) = console();
        for level in Level::ALL.into_iter().rev() {
            level.apply(&mut console);
            assert_eq!(Level::current(&console), Some(level));
            assert_eq!(shown(&console), level.label());
        }
        // The profile was saved: a new start reads the last preset.
        drop(console);
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Vanilla));
    }

    /// Within each family, no preset costs less than the one before it; the Solar
    /// Renderer's are all real-time lit, the Vanilla family's never.
    #[test]
    fn no_preset_costs_less_than_the_one_below_it_in_its_family() {
        for (cvar, values) in VALUES {
            let numbers: Vec<f64> = values.iter().map(|v| v.parse().unwrap()).collect();
            for family in [&numbers[0..2], &numbers[2..6]] {
                assert!(
                    family.windows(2).all(|pair| pair[0] <= pair[1]),
                    "{cvar}: {values:?}"
                );
            }
            assert!(
                numbers.iter().any(|number| *number != numbers[0]),
                "{cvar} is the same in every preset"
            );
        }
        for level in Level::ALL {
            let lit = VALUES
                .iter()
                .find(|(cvar, _)| *cvar == "r_dayNight")
                .is_some_and(|(_, values)| values[level.index()] == "1");
            assert_eq!(lit, level.family() == Family::Solar, "{level:?}");
        }
    }

    #[test]
    fn every_value_is_one_its_settings_row_can_show() {
        let (_directory, mut console) = console();
        for level in Level::ALL {
            for (cvar, values) in VALUES {
                let value = values[level.index()];
                assert!(
                    crate::settings::row_takes(cvar, value),
                    "{cvar} {value}: no row in Settings shows it"
                );
                assert!(console.set_cvar(cvar, value), "{cvar} {value}");
                assert!(holds(&console, cvar, value), "{cvar} refused {value}");
            }
        }
    }

    /// A profile saved on the presets of 16/10/2026 reads as their new names: Vanilla
    /// style as Vanilla, SJK (recommended) as Vanilla Plus, Experimental as Solar High.
    #[test]
    fn the_old_presets_read_as_their_new_names() {
        let old: [(&[(&str, &str)], Level); 3] = [
            (
                &[
                    ("r_sceneHdr", "0"),
                    ("r_sceneBloom", "0"),
                    ("r_DynamicGlow", "0"),
                    ("r_fxaa", "0"),
                    ("r_softParticles", "0"),
                    ("r_flares", "0"),
                    ("r_ssao", "0"),
                    (crate::dust_motes::CVAR, "0"),
                    ("r_modelPixelLight", "0"),
                    ("r_cubeMapping", "0"),
                    ("r_floorReflections", "0"),
                    ("r_normalMapping", "0"),
                    ("r_specularMapping", "0"),
                    ("r_parallaxMapping", "0"),
                    ("r_emissiveMaps", "0"),
                    ("r_emissiveGlow", "0"),
                    ("r_dayNight", "0"),
                    ("r_emissiveLights", "0"),
                    ("r_volumetrics", "0"),
                    ("r_worldSunShadows", "0"),
                    ("r_actorSunShadows", "0"),
                    ("r_sunShadowResolution", "1024"),
                    ("r_sunShadowTaps", "8"),
                    (crate::weather::DENSITY_CVAR, "1"),
                    (crate::weather::QUALITY_CVAR, "0"),
                ],
                Level::Vanilla,
            ),
            (
                &[
                    ("r_sceneHdr", "1"),
                    ("r_sceneBloom", "1"),
                    ("r_DynamicGlow", "1"),
                    ("r_fxaa", "1"),
                    ("r_softParticles", "0"),
                    ("r_flares", "1"),
                    ("r_ssao", "1"),
                    (crate::dust_motes::CVAR, "0"),
                    ("r_modelPixelLight", "1"),
                    ("r_cubeMapping", "0"),
                    ("r_floorReflections", "0"),
                    ("r_normalMapping", "0"),
                    ("r_specularMapping", "0"),
                    ("r_parallaxMapping", "0"),
                    ("r_emissiveMaps", "1"),
                    ("r_emissiveGlow", "1"),
                    ("r_dayNight", "0"),
                    ("r_emissiveLights", "0"),
                    ("r_volumetrics", "0"),
                    ("r_worldSunShadows", "0"),
                    ("r_actorSunShadows", "0"),
                    ("r_sunShadowResolution", "1024"),
                    ("r_sunShadowTaps", "8"),
                    (crate::weather::DENSITY_CVAR, "1"),
                    (crate::weather::QUALITY_CVAR, "0"),
                ],
                Level::VanillaPlus,
            ),
            (&[], Level::SolarHigh),
        ];
        for (values, level) in old {
            let (_directory, mut console) = console();
            console.set_cvars(values.iter().copied());
            assert_eq!(Level::current(&console), Some(level), "{level:?}");
        }
    }

    #[test]
    fn a_setting_changed_on_its_own_is_custom_near_its_preset() {
        let (_directory, mut console) = console();
        Level::SolarHigh.apply(&mut console);
        console.set_cvar("r_sunShadowTaps", "12");
        assert_eq!(Level::current(&console), None);
        assert_eq!(shown(&console), "Custom");
        assert_eq!(Level::nearest(&console), Level::SolarHigh);
        Level::VanillaPlus.apply(&mut console);
        console.set_cvar("r_normalMapping", "1");
        assert_eq!(Level::nearest(&console), Level::VanillaPlus);
        // Its rows' defaults are the nearest preset's: the changed one reads
        // as changed, the others do not.
        assert_eq!(row_default(&console, "r_normalMapping"), Some("0"));
        assert_eq!(row_default(&console, "r_sceneHdr"), Some("1"));
        assert_eq!(row_default(&console, "cg_fov"), None);
    }

    #[test]
    fn steps_go_through_both_families_and_stop_at_the_ends() {
        assert_eq!(Level::Vanilla.step(-1), Level::Vanilla);
        assert_eq!(Level::Vanilla.step(1), Level::VanillaPlus);
        assert_eq!(Level::VanillaPlus.step(1), Level::SolarLow);
        assert_eq!(Level::SolarLow.step(-1), Level::VanillaPlus);
        assert_eq!(Level::SolarHigh.step(1), Level::SolarUltra);
        assert_eq!(Level::SolarUltra.step(1), Level::SolarUltra);
    }

    #[test]
    fn the_command_names_and_sets_the_preset() {
        let (_directory, mut console) = console();
        let lines = command(&mut console, &[]).unwrap();
        assert!(
            lines[0].starts_with("Graphics preset: Solar Renderer: High"),
            "{lines:?}"
        );
        let cases = [
            ("SJK", Level::VanillaPlus),
            ("ultralow", Level::Vanilla),
            ("Experimental", Level::SolarHigh),
            ("ejk", Level::Vanilla),
            ("SJK (recommended)", Level::VanillaPlus),
            ("Vanilla style", Level::Vanilla),
            ("Vanilla Plus", Level::VanillaPlus),
            ("vanillaplus", Level::VanillaPlus),
            ("low", Level::SolarLow),
            ("Solar Medium", Level::SolarMedium),
            ("Solar Renderer: Ultra", Level::SolarUltra),
            ("high", Level::SolarHigh),
        ];
        for (name, level) in cases {
            command(&mut console, &[name.to_owned()]).unwrap();
            assert_eq!(Level::current(&console), Some(level), "{name}");
        }
        assert!(command(&mut console, &["rtx".to_owned()]).is_err());
        assert_eq!(Level::current(&console), Some(Level::SolarHigh));
        // Recommended is this PC's pick, whatever it is.
        recommend_command(&mut console).unwrap();
        assert_eq!(Level::current(&console), Some(recommended()));
    }

    /// The marks follow the card: a big card runs every preset smoothly, a small one only
    /// Vanilla; an unknown card is a mid-range one; Vanilla is never worse than fine.
    #[test]
    fn ratings_follow_the_cards_video_memory() {
        let gib = |gib: u64| Some(gib * 1024);
        for level in Level::ALL {
            assert_eq!(level.rating(gib(16)), Rating::Smooth, "{level:?}");
            assert!(level.rating(Some(256)) >= Rating::Fine, "{level:?}");
        }
        assert_eq!(Level::Vanilla.rating(Some(256)), Rating::Fine);
        assert_eq!(Level::SolarUltra.rating(Some(256)), Rating::TooHeavy);
        // Each card's marks get no better from one preset to the next.
        for mib in [None, gib(2), gib(4), gib(6), gib(8), gib(12)] {
            let ratings = Level::ALL.map(|level| level.rating(mib));
            assert!(ratings.windows(2).all(|pair| pair[0] <= pair[1]), "{mib:?}");
        }
        assert_eq!(Level::SolarHigh.rating(gib(8)), Rating::Smooth);
        assert_eq!(Level::SolarUltra.rating(gib(8)), Rating::Fine);
        assert_eq!(Level::SolarHigh.rating(gib(4)), Rating::TooHeavy);
    }

    #[test]
    fn the_recommendation_is_the_richest_smooth_preset() {
        let gib = |gib: u64| Some(gib * 1024);
        assert_eq!(recommended_for(None), Level::VanillaPlus);
        assert_eq!(recommended_for(Some(512)), Level::Vanilla);
        assert_eq!(recommended_for(gib(2)), Level::Vanilla);
        assert_eq!(recommended_for(gib(4)), Level::SolarLow);
        assert_eq!(recommended_for(gib(6)), Level::SolarMedium);
        assert_eq!(recommended_for(gib(8)), Level::SolarHigh);
        assert_eq!(recommended_for(gib(16)), Level::SolarUltra);
        for level in Level::ALL {
            assert!(level.label_recommended().starts_with(level.label()));
            assert!(level.label_recommended().ends_with("(recommended)"));
        }
    }

    #[test]
    fn vanilla_turns_sjk_lighting_and_material_maps_off() {
        let (_directory, mut console) = console();
        Level::Vanilla.apply(&mut console);
        for cvar in [
            "r_dayNight",
            "r_worldSunShadows",
            "r_volumetrics",
            "r_cubeMapping",
            "r_floorReflections",
            "r_normalMapping",
            "r_specularMapping",
            "r_parallaxMapping",
            "r_emissiveMaps",
            "r_ssao",
            "r_sceneHdr",
        ] {
            assert!(holds(&console, cvar, "0"), "{cvar}");
        }
        // EternalJK's own renderer defaults (rd-vanilla `tr_init.cpp`).
        for cvar in ["r_DynamicGlow", "r_flares"] {
            assert!(holds(&console, cvar, "0"), "{cvar}");
        }
    }

    #[test]
    fn vanilla_and_back_restores_the_players_settings() {
        let (directory, mut console) = console();
        Level::SolarHigh.apply(&mut console);
        console.set_cvar("r_sunShadowTaps", "12");
        assert!(!vanilla(&console));
        set_renderer(&mut console, VANILLA);
        assert!(vanilla(&console));
        // Kept across a restart.
        drop(console);
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(vanilla(&console));
        set_renderer(&mut console, VANILLA);
        assert!(vanilla(&console), "Vanilla again changes nothing");
        set_renderer(&mut console, SJK);
        assert!(!vanilla(&console));
        assert!(holds(&console, "r_sunShadowTaps", "12"));
        console.set_cvar("r_sunShadowTaps", "16");
        assert_eq!(Level::current(&console), Some(Level::SolarHigh));
        assert!(holds(&console, RESTORE_CVAR, ""));
        // Vanilla picked as a preset has nothing kept: back is Vanilla Plus.
        Level::Vanilla.apply(&mut console);
        toggle_vanilla(&mut console);
        assert_eq!(Level::current(&console), Some(Level::VanillaPlus));
    }

    /// `holds` as it was, through the value's text.
    fn holds_by_text(console: &ViewerConsole, cvar: &str, value: &str) -> bool {
        let Some(current) = console.cvar(cvar).map(sjk_shell::CvarValue::as_text) else {
            return false;
        };
        match (current.trim().parse::<f64>(), value.parse::<f64>()) {
            (Ok(current), Ok(value)) => (current - value).abs() < 1e-6,
            _ => current.trim().eq_ignore_ascii_case(value),
        }
    }

    /// Typed values compare as their text did, every preset value of every preset and a
    /// text cvar holding numbers, words and spaces; and Vanilla is still the preset
    /// `Level::current` finds first.
    #[test]
    fn typed_values_hold_what_their_text_held() {
        let (_directory, mut console) = console();
        let values = [
            "0", "1", "1.0", "0.5", "2", "3", "8", "12", "16", "1024", "2048", "", "abc", " 1",
        ];
        let check = |console: &ViewerConsole| {
            for (cvar, _) in VALUES.iter().chain([&(RESTORE_CVAR, [""; 6])]) {
                for value in values {
                    assert_eq!(
                        holds(console, cvar, value),
                        holds_by_text(console, cvar, value),
                        "{cvar} = {:?} holds {value:?}",
                        console.cvar(cvar)
                    );
                }
            }
            assert_eq!(
                vanilla(console),
                Level::current(console) == Some(Level::Vanilla)
            );
        };
        for level in Level::ALL {
            level.apply(&mut console);
            check(&console);
        }
        console.set_cvar("r_sunShadowTaps", "12");
        check(&console);
        Level::Vanilla.apply(&mut console);
        console.set_cvar("r_sceneHdr", "1");
        check(&console);
        for text in [" 1.0 ", "ABC", "abc", "", "2048", "r_sceneHdr=0"] {
            console.set_cvar(RESTORE_CVAR, text);
            check(&console);
        }
        console.set_cvar(RESTORE_CVAR, " 1.0 ");
        assert!(holds(&console, RESTORE_CVAR, "1"));
        assert!(!holds(&console, "no_such_cvar", "0"));
    }

    /// Reading the preset takes no allocation, all of Vanilla's values read included.
    #[cfg(debug_assertions)]
    #[test]
    fn reading_vanilla_allocates_nothing() {
        let (_directory, mut console) = console();
        for level in Level::ALL {
            level.apply(&mut console);
            let before = assert_no_alloc::violation_count();
            let vanilla = assert_no_alloc::assert_no_alloc(|| vanilla(&console));
            assert_eq!(assert_no_alloc::violation_count(), before, "{level:?}");
            assert_eq!(vanilla, level == Level::Vanilla);
        }
    }

    /// Every preset value is an archived cvar: a change of one changes the stamp the
    /// low-FPS help keeps its Vanilla reading by (`fps_help.rs`).
    #[test]
    fn every_preset_value_changes_the_archive_stamp() {
        let (_directory, mut console) = console();
        for (cvar, values) in VALUES {
            let vanilla = values[Level::Vanilla.index()];
            // Another preset's value first, so setting Vanilla's is a change.
            let other = values.iter().find(|value| *value != &vanilla).unwrap();
            console.set_cvar(cvar, other);
            let before = console.archive_revision();
            console.set_cvar(cvar, vanilla);
            assert_ne!(console.archive_revision(), before, "{cvar}");
        }
        assert!(vanilla(&console));
    }

    #[test]
    fn a_new_profile_is_vanilla_plus_and_its_rows_read_unchanged() {
        let (_directory, mut console) = console();
        apply_new_profile(&mut console);
        assert_eq!(Level::current(&console), Some(Level::VanillaPlus));
        assert_eq!(shown(&console), "Vanilla Plus");
        assert_eq!(row_default(&console, "r_sceneHdr"), Some("1"));
        assert_eq!(row_default(&console, "r_dayNight"), Some("0"));
        assert_eq!(row_default(&console, "cg_fov"), None);
        let lines = renderer_command(&mut console, &[]).unwrap();
        assert!(lines[0].contains("Vanilla Plus"), "{lines:?}");
        renderer_command(&mut console, &["vanilla".to_owned()]).unwrap();
        assert!(vanilla(&console));
        assert_eq!(row_default(&console, "r_sceneHdr"), Some("0"));
        renderer_command(&mut console, &["SJK".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::VanillaPlus));
        assert!(renderer_command(&mut console, &["rtx".to_owned()]).is_err());
    }
}
