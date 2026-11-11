//! Tactical mode (Sol, 11/11/2026): one switch, `cg_tactical`, that strips SJK's extras
//! for a player who wants to fight, not look. Each level holds the ones below it:
//!
//! - **Minimal** (1): nothing social over the game. Other players' emotes and toys are
//!   not seen nor heard ([`hides_fun`], read where their relayed events are taken), the
//!   unlock cards wait until Tactical is off again (`unlock_toast.rs`), and the toys'
//!   own switches, status balloons, tags and holocron cards go off.
//! - **Medium** (2): the Vanilla graphics preset (`graphics_quality.rs`), no weather,
//!   clouds, weather sounds or map fog, every blade stock (`cg_stockBlades 2`), no
//!   finishers, Force Speed afterimages or toon shading.
//! - **Max** (3): the world flat (`r_flatWorld`, `flat_world.rs`: each surface in its
//!   texture's average colour under the map's light, only its first stage drawn), no
//!   server or shared remaps, and every other player drawn plain ([`plain_players`]):
//!   in [`MODEL_CVAR`] (Kyle by default, a team game's red or blue kept), with the
//!   default saber in the default blue, without hat, cape or blade skin, as a muted
//!   player is (`muted_players.rs`), their chat and sounds untouched.
//!
//! The settings a level changes are put aside first, `cvar=value` pairs in
//! [`RESTORE_CVAR`], and given back when the level drops, as `renderer vanilla` does
//! (`graphics_quality::toggle_vanilla`): Tactical off leaves the player's settings as they
//! were. A setting changed by hand while Tactical holds it goes back when Tactical lets
//! go. The level is followed each frame ([`crate::GpuState::update_tactical`]), so
//! the console, the Settings row and a config all switch it.

use crate::console::ViewerConsole;
use std::sync::atomic::{AtomicU8, Ordering};

/// Archived, default 0: 0 off, 1 minimal, 2 medium, 3 max.
pub(crate) const CVAR: &str = "cg_tactical";
/// Archived: the model other players are drawn in at Max, `model` or `model/skin`.
pub(crate) const MODEL_CVAR: &str = "cg_tacticalModel";
/// [`MODEL_CVAR`]'s default: Kyle, the game's own stand-in.
pub(crate) const MODEL_DEFAULT: &str = "kyle";
/// Archived: what the current level put aside, `cvar=value` pairs joined by `;`.
pub(crate) const RESTORE_CVAR: &str = "cg_tacticalRestore";
/// Console command that names or sets the level.
pub(crate) const COMMAND: &str = "tactical";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show or set Tactical mode: off, minimal (no emotes, toys or \
     pop-ups), medium (also lowest graphics, no weather, fog or blade skins) or max (also a \
     flat world, everyone in one model with a blue stock saber)";

/// A Tactical level; each holds the ones below it.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Level {
    /// Everything as the player set it (0, the default).
    #[default]
    Off,
    /// No emotes, toys or pop-ups from others (1).
    Minimal,
    /// Also the lowest graphics, no weather or fog, stock blades (2).
    Medium,
    /// Also a flat world and every other player plain (3).
    Max,
}

impl Level {
    /// Every level, lowest first.
    pub(crate) const ALL: [Self; 4] = [Self::Off, Self::Minimal, Self::Medium, Self::Max];

    /// The level `value` of [`CVAR`] names; out of range is held to Off..Max.
    pub(crate) fn of(value: Option<i64>) -> Self {
        match value.unwrap_or(0) {
            ..=0 => Self::Off,
            1 => Self::Minimal,
            2 => Self::Medium,
            _ => Self::Max,
        }
    }

    /// The value [`CVAR`] holds for it.
    pub(crate) fn value(self) -> i64 {
        self as i64
    }

    /// Its name in the console and the Settings row.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Medium => "medium",
            Self::Max => "max",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        if let Ok(value) = name.parse::<i64>() {
            return (0..=3).contains(&value).then(|| Self::of(Some(value)));
        }
        Self::ALL
            .into_iter()
            .find(|level| level.name().eq_ignore_ascii_case(name))
    }

    /// What the level takes away, for the console.
    fn words(self) -> &'static str {
        match self {
            Self::Off => "everything as you set it",
            Self::Minimal => "no emotes, toys, tags, status balloons or unlock cards",
            Self::Medium => {
                "minimal, plus Vanilla graphics, no weather or fog, stock blades, no finishers"
            }
            Self::Max => "medium, plus a flat world, no remaps, everyone plain with a blue saber",
        }
    }
}

/// The level in force, for code far from the console.
static LEVEL: AtomicU8 = AtomicU8::new(0);

/// The level in force (the last one [`apply`] set).
pub(crate) fn level() -> Level {
    Level::of(Some(i64::from(LEVEL.load(Ordering::Relaxed))))
}

/// Whether other players' emotes, toys and the unlock cards are hidden (Minimal and up).
pub(crate) fn hides_fun() -> bool {
    level() >= Level::Minimal
}

/// Whether other players are drawn plain (Max).
pub(crate) fn plain_players() -> bool {
    level() >= Level::Max
}

/// The settings Minimal sets: the toys' and pop-ups' own switches.
const MINIMAL: &[(&str, &str)] = &[
    (crate::toy_sight::ILLUMINATE, "0"),
    (crate::toy_sight::CARDBOARD_BOX, "0"),
    (crate::toy_sight::RAGDOLL, "0"),
    (crate::toy_sight::DRONE, "0"),
    (crate::toy_sight::BOOMBOX, "0"),
    (crate::monitor::SHOW_CVAR, "0"),
    (crate::banana::SHOW_CVAR, "0"),
    (crate::propgun::SHOW_CVAR, "0"),
    (crate::tags::SHOW_CVAR, "0"),
    (crate::tags::GIF_SHOW_CVAR, "0"),
    (crate::status_balloons::CVAR, "0"),
    (crate::holocron_popup::NOTIFICATIONS_CVAR, "0"),
];

/// The settings Medium adds to the Vanilla preset's.
const MEDIUM: &[(&str, &str)] = &[
    (crate::weather::settings::CVAR, "0"),
    (crate::weather::settings::FOG_CVAR, "0"),
    (crate::weather::settings::CLOUDS_CVAR, "0"),
    (crate::weather::settings::LIGHTNING_CVAR, "0"),
    (crate::weather::sound::CVAR, "0"),
    ("r_drawfog", "0"),
    (crate::stock_blades::CVAR, "2"),
    (crate::finishers::CVAR, "0"),
    ("cg_speedTrail", "0"),
    (crate::dynamic_lights::TOON_MODELS_CVAR, "0"),
];

/// The settings Max adds.
const MAX: &[(&str, &str)] = &[
    (crate::flat_world::CVAR, "1"),
    (crate::remap_blocked_maps::SERVER_CVAR, "0"),
    (crate::remap_blocked_maps::SHARED_CVAR, "0"),
];

/// Every setting `level` holds, and the value it holds it at; a setting two lists name
/// takes the later (the higher level's) value.
pub(crate) fn values(level: Level) -> Vec<(&'static str, &'static str)> {
    let mut values: Vec<(&'static str, &'static str)> = Vec::new();
    let mut add = |list: &[(&'static str, &'static str)]| {
        for &(cvar, value) in list {
            match values
                .iter_mut()
                .find(|(known, _)| known.eq_ignore_ascii_case(cvar))
            {
                Some(entry) => entry.1 = value,
                None => values.push((cvar, value)),
            }
        }
    };
    if level >= Level::Minimal {
        add(MINIMAL);
    }
    if level >= Level::Medium {
        add(&crate::graphics_quality::preset_cvars(
            crate::graphics_quality::Level::Vanilla,
        ));
        add(MEDIUM);
    }
    if level >= Level::Max {
        add(MAX);
    }
    values
}

/// Give back what the last level put aside, put aside what `level` changes and set it;
/// [`level`] says `level` from then on. Settings `level` names that this client does not
/// have (a build without a toy) are skipped.
pub(crate) fn apply(console: &mut ViewerConsole, level: Level) {
    set(console, level);
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// [`apply`]'s settings, without telling the rest of the client.
fn set(console: &mut ViewerConsole, level: Level) {
    let kept = console
        .cvar(RESTORE_CVAR)
        .map(sjk_shell::CvarValue::as_text)
        .unwrap_or_default();
    let restored: Vec<(&str, &str)> = kept
        .split(';')
        .filter_map(|pair| pair.split_once('='))
        .filter(|(cvar, _)| console.cvar(cvar).is_some())
        .collect();
    console.set_cvars(restored);
    let values: Vec<_> = values(level)
        .into_iter()
        .filter(|(cvar, _)| console.cvar(cvar).is_some())
        .collect();
    let kept = values
        .iter()
        .filter_map(|(cvar, _)| Some(format!("{cvar}={}", console.cvar(cvar)?.as_text())))
        .collect::<Vec<_>>()
        .join(";");
    console.set_cvars(values.into_iter().chain([(RESTORE_CVAR, kept.as_str())]));
    let value = level.value().to_string();
    if console.integer_cvar(CVAR) != Some(level.value()) {
        console.set_cvar(CVAR, &value);
    }
}

/// The level [`CVAR`] asks for.
pub(crate) fn asked(console: &ViewerConsole) -> Level {
    Level::of(console.integer_cvar(CVAR))
}

/// The model other players are drawn in at Max, as `model/skin` (`default` when the
/// cvar names no skin; Kyle when it is empty).
pub(crate) fn model(console: Option<&ViewerConsole>) -> String {
    let value = console
        .and_then(|console| console.text_cvar(MODEL_CVAR).ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains(['\\', '"', ';']))
        .unwrap_or(MODEL_DEFAULT);
    if value.contains('/') {
        value.to_owned()
    } else {
        format!("{value}/default")
    }
}

/// `tactical [off|minimal|medium|max|0-3]`: name the level, or set it.
pub(crate) fn command(console: &mut ViewerConsole, args: &[String]) -> Result<Vec<String>, String> {
    let names = || Level::ALL.map(Level::name).join(", ");
    let shown = |level: Level| format!("Tactical: {} ({})", level.name(), level.words());
    match args {
        [] => Ok(vec![
            shown(asked(console)),
            format!("usage: {COMMAND} [{}]", names()),
        ]),
        [name] => {
            let level =
                Level::parse(name).ok_or_else(|| format!("usage: {COMMAND} [{}]", names()))?;
            let graphics = (level >= Level::Medium) != (self::level() >= Level::Medium);
            apply(console, level);
            let mut lines = vec![shown(level)];
            if graphics {
                lines.push(
                    "Some graphics settings apply after a graphics reload (vid_restart), the \
                     flat world and remaps on the next map"
                        .to_owned(),
                );
            }
            Ok(lines)
        }
        _ => Err(format!("usage: {COMMAND} [{}]", names())),
    }
}

impl crate::GpuState {
    /// Follow [`CVAR`]: when it asks for another level than the one in force, apply it.
    /// A profile saved at a level comes back at it (its settings already set, the
    /// originals still put aside), so the first turn only gives the level to [`level`].
    pub(crate) fn update_tactical(&mut self) {
        let Some(console) = self.console.as_mut() else {
            return;
        };
        let asked = asked(console);
        if asked != level() {
            apply(console, asked);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn text(console: &ViewerConsole, cvar: &str) -> String {
        console
            .cvar(cvar)
            .map(sjk_shell::CvarValue::as_text)
            .unwrap_or_default()
    }

    #[test]
    fn each_level_holds_the_ones_below_it() {
        let minimal = values(Level::Minimal);
        let medium = values(Level::Medium);
        let max = values(Level::Max);
        assert!(values(Level::Off).is_empty());
        assert!(
            minimal
                .iter()
                .all(|entry| medium.iter().any(|(cvar, _)| *cvar == entry.0))
        );
        assert!(
            medium
                .iter()
                .all(|entry| max.iter().any(|(cvar, _)| *cvar == entry.0))
        );
        assert!(max.contains(&(crate::flat_world::CVAR, "1")));
        assert!(medium.contains(&(crate::stock_blades::CVAR, "2")));
        assert!(
            !minimal
                .iter()
                .any(|(cvar, _)| *cvar == crate::stock_blades::CVAR)
        );
    }

    #[test]
    fn every_setting_a_level_names_exists_and_takes_its_value() {
        let (_directory, mut console) = console();
        for (cvar, value) in values(Level::Max) {
            assert!(console.cvar(cvar).is_some(), "{cvar} is registered");
            assert!(
                console.try_set_cvar(cvar, value).is_ok(),
                "{cvar} takes {value}"
            );
        }
    }

    #[test]
    fn off_gives_back_the_players_own_settings() {
        let (_directory, mut console) = console();
        console.set_cvar(crate::stock_blades::CVAR, "1");
        console.set_cvar(crate::weather::settings::FOG_CVAR, "2");
        let before: Vec<(&str, String)> = values(Level::Max)
            .into_iter()
            .map(|(cvar, _)| (cvar, text(&console, cvar)))
            .collect();
        set(&mut console, Level::Max);
        assert_eq!(text(&console, crate::stock_blades::CVAR), "2");
        assert_eq!(text(&console, crate::flat_world::CVAR), "1");
        assert_eq!(console.integer_cvar(CVAR), Some(3));
        // Down a level: Max's own settings come back, Medium's stay held.
        set(&mut console, Level::Medium);
        assert_eq!(text(&console, crate::flat_world::CVAR), "0");
        assert_eq!(text(&console, crate::stock_blades::CVAR), "2");
        set(&mut console, Level::Off);
        for (cvar, value) in before {
            assert_eq!(text(&console, cvar), value, "{cvar} is given back");
        }
        assert_eq!(text(&console, RESTORE_CVAR), "");
    }

    #[test]
    fn a_saved_level_comes_back_without_losing_the_originals() {
        let (_directory, mut console) = console();
        console.set_cvar(crate::stock_blades::CVAR, "1");
        set(&mut console, Level::Medium);
        // A restart: the profile holds Medium's values and the originals, and
        // following the cvar applies Medium again.
        let asked = asked(&console);
        set(&mut console, asked);
        assert_eq!(text(&console, crate::stock_blades::CVAR), "2");
        set(&mut console, Level::Off);
        assert_eq!(text(&console, crate::stock_blades::CVAR), "1");
    }

    #[test]
    fn levels_parse_by_name_and_number() {
        assert_eq!(Level::parse("MAX"), Some(Level::Max));
        assert_eq!(Level::parse("1"), Some(Level::Minimal));
        assert_eq!(Level::parse("4"), None);
        assert_eq!(Level::parse("loud"), None);
        assert_eq!(Level::of(Some(9)), Level::Max);
        assert_eq!(Level::of(Some(-1)), Level::Off);
    }

    #[test]
    fn the_model_names_a_skin() {
        let (_directory, mut console) = console();
        assert_eq!(model(Some(&console)), "kyle/default");
        console.set_cvar(MODEL_CVAR, "jedi_hm/blue");
        assert_eq!(model(Some(&console)), "jedi_hm/blue");
        console.set_cvar(MODEL_CVAR, "reborn");
        assert_eq!(model(Some(&console)), "reborn/default");
        console.set_cvar(MODEL_CVAR, " ");
        assert_eq!(model(Some(&console)), "kyle/default");
    }
}
