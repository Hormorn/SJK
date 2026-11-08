//! Graphics presets: Vanilla style, SJK (recommended) and Experimental, each one
//! choice that sets the renderer's costly settings together (Sol's three,
//! 16/10/2026, in place of the Renderer row and the SJK renderer's four levels).
//!
//! Vanilla style is the original game's look at the least cost, every SJK extra
//! off: EternalJK's defaults on its own renderer (rd-eternaljk, rd-vanilla's
//! code: no dynamic glow, no light flares; the EJK level until 12/10/2026, then
//! Vanilla). SJK (recommended) is
//! Sol's own setup: SJK's image (HDR, bloom, glow, FXAA, ambient occlusion, lit
//! models, emission maps) over the maps' baked lighting, without the sun,
//! shadows, reflections, material maps or weather extras still in alpha. A new
//! profile starts on it ([`apply_new_profile`]). Experimental turns everything
//! the SJK renderer draws on; it is the renderer cvars' own defaults, the look
//! they were tuned for.
//! The preset is not stored: it is whichever preset the settings match, and
//! Custom once one of them is changed on its own. Only settings with a row in
//! Settings are touched, so every change a preset makes can be seen and undone
//! there; the FPS cap, vertical sync, resolution, supersampling and anything
//! that changes gameplay or a map's content (its fog) are left alone.

use crate::console::ViewerConsole;

/// Console command that names or sets the preset (its name is from when the
/// presets were quality levels).
pub(crate) const COMMAND: &str = "graphicsquality";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Show or set the graphics preset: vanilla, sjk (recommended) or experimental";
/// The Graphics preset row's stand-in for a cvar name: the command that sets it.
pub(crate) const ROW_NAME: &str = COMMAND;
/// Console command that names the renderer or switches Vanilla style on and
/// off, keeping the player's settings ([`toggle_vanilla`]).
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

/// A graphics preset, cheapest first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Level {
    /// The original game's look at the least cost (EJK, Ultra low and Vanilla,
    /// before): every SJK extra off, maps show their baked lightmaps.
    Vanilla,
    /// SJK's image over the maps' own lighting: Sol's setup, the default.
    Sjk,
    /// Everything the SJK renderer draws: sun and sky, shadows, reflections,
    /// material maps, light shafts and the richest weather; still in alpha.
    Experimental,
}

/// Each setting a preset sets, with its value at Vanilla style, SJK
/// (recommended) and Experimental. Experimental is each setting's default
/// (pinned by a test); SJK (recommended) is Sol's config of 16/10/2026.
const VALUES: &[(&str, [&str; 3])] = &[
    // Image.
    ("r_sceneHdr", ["0", "1", "1"]),
    ("r_sceneBloom", ["0", "1", "1"]),
    ("r_DynamicGlow", ["0", "1", "1"]),
    ("r_fxaa", ["0", "1", "1"]),
    ("r_softParticles", ["0", "0", "1"]),
    // EternalJK's rd-vanilla registers `r_flares` 0; SJK draws them by default.
    ("r_flares", ["0", "1", "1"]),
    ("r_ssao", ["0", "1", "1"]),
    // Dust shows only in light shafts.
    (crate::dust_motes::CVAR, ["0", "0", "1"]),
    ("r_modelPixelLight", ["0", "1", "1"]),
    ("r_cubeMapping", ["0", "0", "1"]),
    ("r_floorReflections", ["0", "0", "1"]),
    ("r_normalMapping", ["0", "0", "1"]),
    ("r_specularMapping", ["0", "0", "1"]),
    ("r_parallaxMapping", ["0", "0", "1"]),
    ("r_emissiveMaps", ["0", "1", "1"]),
    ("r_emissiveGlow", ["0", "1", "1"]),
    // Lighting.
    ("r_dayNight", ["0", "0", "1"]),
    ("r_emissiveLights", ["0", "0", "1"]),
    ("r_volumetrics", ["0", "0", "3"]),
    // Shadows.
    ("r_worldSunShadows", ["0", "0", "1"]),
    ("r_actorSunShadows", ["0", "0", "1"]),
    ("r_sunShadowResolution", ["1024", "1024", "2048"]),
    ("r_sunShadowTaps", ["8", "8", "16"]),
    // Weather.
    (crate::weather::DENSITY_CVAR, ["1", "1", "2"]),
    (crate::weather::QUALITY_CVAR, ["0", "0", "2"]),
    // The volumetric clouds (`r_clouds`) are no preset's: off by default, the player's own
    // choice under every preset (Sol, 05/11/2026).
];

impl Level {
    pub(crate) const ALL: [Self; 3] = [Self::Vanilla, Self::Sjk, Self::Experimental];
    /// A new profile's preset, which the row's Backspace returns to.
    pub(crate) const DEFAULT: Self = Self::Sjk;

    /// The preset's name in Settings.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla style",
            Self::Sjk => "SJK (recommended)",
            Self::Experimental => "Experimental",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// The preset's name in the command.
    fn command_name(self) -> &'static str {
        match self {
            Self::Vanilla => VANILLA,
            Self::Sjk => SJK,
            Self::Experimental => "experimental",
        }
    }

    /// The preset `name` names, in any case: its command name or label, with
    /// or without spaces; EJK and Ultra low are Vanilla style's old names.
    fn parse(name: &str) -> Option<Self> {
        let name: String = name.split_whitespace().collect();
        if ["ultralow", "ejk"]
            .iter()
            .any(|old| name.eq_ignore_ascii_case(old))
        {
            return Some(Self::Vanilla);
        }
        Self::ALL.into_iter().find(|level| {
            name.eq_ignore_ascii_case(level.command_name())
                || name.eq_ignore_ascii_case(&level.label().replace(' ', ""))
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
}

/// Whether the renderer is Vanilla: the settings are Vanilla style's (the first preset
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

/// Switch between Vanilla style and the SJK renderer. To Vanilla, it keeps the
/// player's value of every setting a preset sets, then applies Vanilla style;
/// back, it puts the kept values back (SJK (recommended) when none were kept,
/// as when Vanilla style was picked as a preset).
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

/// Start a new profile on SJK (recommended).
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
                "Renderer: SJK, {} graphics; it can still show glitches, which the SJK menu's \
                 Report a bug sends to the team",
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
/// while a map plays).
fn holds(console: &ViewerConsole, cvar: &str, value: &str) -> bool {
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
            let level =
                Level::parse(name).ok_or_else(|| format!("usage: {COMMAND} [{}]", names()))?;
            level.apply(console);
            Ok(vec![format!(
                "Graphics preset: {}; shadows, light shafts, HDR, FXAA, sun and sky, \
                 reflection probes and material maps apply after a graphics reload \
                 (vid_restart), emission-map lights on the next map",
                level.label()
            )])
        }
        _ => Err(format!("usage: {COMMAND} [{}]", names())),
    }
}

/// The cvars `level` sets, for measurements that build a client at a preset.
#[cfg(test)]
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
    fn the_cvars_defaults_are_experimental() {
        let (_directory, console) = console();
        for (cvar, values) in VALUES {
            let default = console
                .cvar_default(cvar)
                .unwrap_or_else(|| panic!("{cvar} is not registered"))
                .as_text();
            assert_eq!(
                default.parse::<f64>().ok(),
                values[Level::Experimental.index()].parse::<f64>().ok(),
                "{cvar}: Experimental is not its default"
            );
        }
        assert_eq!(Level::current(&console), Some(Level::Experimental));
        assert_eq!(shown(&console), "Experimental");
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

    #[test]
    fn no_preset_costs_less_than_the_one_below_it() {
        for (cvar, values) in VALUES {
            let numbers: Vec<f64> = values.iter().map(|v| v.parse().unwrap()).collect();
            assert!(
                numbers.windows(2).all(|pair| pair[0] <= pair[1]),
                "{cvar}: {values:?}"
            );
            assert!(
                numbers[0] < numbers[2],
                "{cvar} is the same in every preset"
            );
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

    #[test]
    fn a_setting_changed_on_its_own_is_custom_near_its_preset() {
        let (_directory, mut console) = console();
        Level::Experimental.apply(&mut console);
        console.set_cvar("r_sunShadowTaps", "12");
        assert_eq!(Level::current(&console), None);
        assert_eq!(shown(&console), "Custom");
        assert_eq!(Level::nearest(&console), Level::Experimental);
        Level::Sjk.apply(&mut console);
        console.set_cvar("r_normalMapping", "1");
        assert_eq!(Level::nearest(&console), Level::Sjk);
        // Its rows' defaults are the nearest preset's: the changed one reads
        // as changed, the others do not.
        assert_eq!(row_default(&console, "r_normalMapping"), Some("0"));
        assert_eq!(row_default(&console, "r_sceneHdr"), Some("1"));
        assert_eq!(row_default(&console, "cg_fov"), None);
    }

    #[test]
    fn steps_stop_at_the_ends() {
        assert_eq!(Level::Vanilla.step(-1), Level::Vanilla);
        assert_eq!(Level::Vanilla.step(1), Level::Sjk);
        assert_eq!(Level::Sjk.step(-1), Level::Vanilla);
        assert_eq!(Level::Sjk.step(1), Level::Experimental);
        assert_eq!(Level::Experimental.step(1), Level::Experimental);
    }

    #[test]
    fn the_command_names_and_sets_the_preset() {
        let (_directory, mut console) = console();
        let lines = command(&mut console, &[]).unwrap();
        assert!(
            lines[0].starts_with("Graphics preset: Experimental"),
            "{lines:?}"
        );
        command(&mut console, &["SJK".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Sjk));
        assert!(command(&mut console, &["ultra".to_owned()]).is_err());
        assert_eq!(Level::current(&console), Some(Level::Sjk));
        command(&mut console, &["ultralow".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Vanilla));
        command(&mut console, &["Experimental".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Experimental));
        command(&mut console, &["ejk".to_owned()]).unwrap();
        assert!(vanilla(&console));
        command(&mut console, &["SJK (recommended)".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Sjk));
        command(&mut console, &["Vanilla style".to_owned()]).unwrap();
        assert!(vanilla(&console));
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
        Level::Experimental.apply(&mut console);
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
        assert_eq!(Level::current(&console), Some(Level::Experimental));
        assert!(holds(&console, RESTORE_CVAR, ""));
        // Vanilla picked as a preset has nothing kept: back is SJK (recommended).
        Level::Vanilla.apply(&mut console);
        toggle_vanilla(&mut console);
        assert_eq!(Level::current(&console), Some(Level::Sjk));
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
            for (cvar, _) in VALUES.iter().chain([&(RESTORE_CVAR, ["", "", ""])]) {
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
        Level::Experimental.apply(&mut console);
        for (cvar, values) in VALUES {
            let before = console.archive_revision();
            console.set_cvar(cvar, values[Level::Vanilla.index()]);
            assert_ne!(console.archive_revision(), before, "{cvar}");
        }
        assert!(vanilla(&console));
    }

    #[test]
    fn a_new_profile_is_sjk_recommended_and_its_rows_read_unchanged() {
        let (_directory, mut console) = console();
        apply_new_profile(&mut console);
        assert_eq!(Level::current(&console), Some(Level::Sjk));
        assert_eq!(shown(&console), "SJK (recommended)");
        assert_eq!(row_default(&console, "r_sceneHdr"), Some("1"));
        assert_eq!(row_default(&console, "r_dayNight"), Some("0"));
        assert_eq!(row_default(&console, "cg_fov"), None);
        let lines = renderer_command(&mut console, &[]).unwrap();
        assert!(lines[0].contains("SJK (recommended)"), "{lines:?}");
        renderer_command(&mut console, &["vanilla".to_owned()]).unwrap();
        assert!(vanilla(&console));
        assert_eq!(row_default(&console, "r_sceneHdr"), Some("0"));
        renderer_command(&mut console, &["SJK".to_owned()]).unwrap();
        assert_eq!(Level::current(&console), Some(Level::Sjk));
        assert!(renderer_command(&mut console, &["rtx".to_owned()]).is_err());
    }
}
