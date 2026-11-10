//! Small graphics cards found at start: SJK reads the card's video memory
//! ([`sjk_gpu_memory`]) and, on a card of 4 GB or less, lowers the texture detail
//! (`r_picmip`) and stands the menus on pictures ([`crate::menu_pictures`]) before the
//! first map is built, so a 3 GB card does not have to run out first (Vin's, 21/10/2026).
//!
//! It chooses once a card: [`CVAR`] keeps the size it chose for, so a player who sets
//! the texture detail back is not overridden at the next start, and a new card is
//! looked at again. It only lowers a setting still at its default. The card's size and
//! budget also go into the log and into crash reports ([`summary`]), and its size sets
//! the real-time lighting's lamp cache budget ([`dedicated`]).
//!
//! Since 25/10/2026 (Sol's choice) a card under 3.5 GiB, integrated GPUs included, also
//! gets lighter graphics once ([`PRESET_CVAR`], [`lighten`]): Vanilla instead of
//! the untouched Vanilla Plus preset (its full-resolution ambient occlusion, HDR
//! and glow cost a weak GPU most), other players' blades stock (`cg_stockBlades 1`) and
//! a 60 FPS menu cap (`com_maxfpsMenu 60`), each only from its default. A card in the
//! menus then says what changed and offers to keep it or go back
//! (`small_card_notice.rs`).

use crate::console::ViewerConsole;
use std::sync::OnceLock;

/// The video memory (MiB) the small-card settings were last chosen for; 0 never.
pub(crate) const CVAR: &str = "r_videoMemoryChecked";
/// The video memory (MiB) the lighter graphics were last considered for; 0 never. Its
/// own setting, so profiles [`CVAR`] already chose for are considered once too.
pub(crate) const PRESET_CVAR: &str = "r_smallCardPreset";
/// The menus' frame cap the lighter graphics set.
const MENU_FPS: &str = "60";
const MIB: u64 = 1 << 20;

static SUMMARY: OnceLock<String> = OnceLock::new();
/// What [`lighten`] changed at this start, for the card.
static LIGHTENED: OnceLock<Lightened> = OnceLock::new();

/// What the lighter graphics changed for a small card.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Lightened {
    /// The card's video memory, MiB.
    pub(crate) mib: i64,
    /// Vanilla Plus became Vanilla.
    pub(crate) vanilla: bool,
    /// Other players' blades became stock.
    pub(crate) stock_blades: bool,
    /// The menus got their 60 FPS cap.
    pub(crate) menu_cap: bool,
    /// The texture detail is lowered (by [`apply`] or the player), for the card's words;
    /// not a change of its own.
    pub(crate) smaller_textures: bool,
}

impl Lightened {
    /// Whether anything changed.
    pub(crate) fn any(self) -> bool {
        self.vanilla || self.stock_blades || self.menu_cap
    }
}

static DEDICATED: OnceLock<Option<u64>> = OnceLock::new();

/// The card's own video memory in bytes, read once (at start, or by the first world
/// built without it); `None` off Windows or when the driver does not say. In tests,
/// `SJK_VIDEO_MEMORY_MIB` stands in for it, so a world shot can be built as on a smaller
/// card (`world_shot_vram.rs`).
pub(crate) fn dedicated() -> Option<u64> {
    #[cfg(test)]
    if let Some(mib) = std::env::var("SJK_VIDEO_MEMORY_MIB")
        .ok()
        .and_then(|mib| mib.parse::<u64>().ok())
    {
        return Some(mib * MIB);
    }
    *DEDICATED.get_or_init(|| {
        sjk_gpu_memory::high_performance()
            .map(|memory| memory.dedicated)
            .filter(|&bytes| bytes > 0)
    })
}

/// Register [`CVAR`] (archived).
pub(crate) fn register(cvars: &mut sjk_shell::CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(sjk_shell::CvarDefinition::new(
        CVAR,
        0_i64,
        sjk_shell::CvarFlags::ARCHIVE,
        "The graphics card's video memory (MiB) SJK last chose the small-card settings for; \
         0 to choose them again at the next start",
    ))?;
    cvars.register(sjk_shell::CvarDefinition::new(
        PRESET_CVAR,
        0_i64,
        sjk_shell::CvarFlags::ARCHIVE,
        "The graphics card's video memory (MiB) SJK last considered lighter graphics for \
         (Vanilla graphics, stock blades for others, a menu cap); 0 to consider them again",
    ))
}

/// The texture detail a card with `dedicated` bytes of its own starts on: `None` when it
/// has room. An integrated GPU's few hundred MB count as a small card.
pub(crate) fn picmip_for(dedicated: u64) -> Option<i64> {
    match dedicated / MIB {
        0 => None,
        mib if mib < 3584 => Some(2),
        mib if mib < 4608 => Some(1),
        _ => None,
    }
}

/// At start, before the first map: read the card and choose for it once
/// ([`apply`]); the menus' blade pictures follow the menu pictures either way.
pub(crate) fn at_start(console: &mut ViewerConsole) {
    let memory = sjk_gpu_memory::high_performance();
    let _ = DEDICATED.set(
        memory
            .as_ref()
            .map(|memory| memory.dedicated)
            .filter(|&bytes| bytes > 0),
    );
    if let Some(memory) = memory {
        let budget = memory.budget.map_or_else(String::new, |budget| {
            format!(", budget {} MiB", budget / MIB)
        });
        let summary = format!("{} MiB video memory{budget}", memory.dedicated / MIB);
        crate::log::progress(format_args!("graphics card: {}, {summary}", memory.name));
        let _ = SUMMARY.set(summary);
        for line in apply(console, memory.dedicated) {
            crate::log::progress(format_args!("{line}"));
            console.push_log_quiet(&line);
        }
        let lightened = lighten(console, memory.dedicated);
        if lightened.any() {
            crate::log::progress(format_args!(
                "small graphics card: {lightened:?} (r_smallCardPreset {})",
                lightened.mib
            ));
            let _ = LIGHTENED.set(lightened);
        }
    }
    crate::blade_pictures::set_live(!crate::menu_pictures::enabled(Some(console)));
}

/// Choose the small-card settings for a card of `dedicated` bytes, unless they were
/// chosen for this size already: the lines saying what changed.
fn apply(console: &mut ViewerConsole, dedicated: u64) -> Vec<String> {
    let mib = (dedicated / MIB) as i64;
    if mib == 0 || console.integer_cvar(CVAR) == Some(mib) {
        return Vec::new();
    }
    console.set_cvar(CVAR, &mib.to_string());
    let Some(picmip) = picmip_for(dedicated) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let detail = crate::world_materials::filtering::mips::PICMIP_CVAR;
    if console.integer_cvar(detail).unwrap_or(0) == 0
        && console.set_cvar(detail, &picmip.to_string())
    {
        lines.push(format!(
            "Your graphics card has {mib} MiB of video memory: Texture detail set to {picmip} \
             (Settings, Renderer)."
        ));
    }
    if !crate::menu_pictures::enabled(Some(console))
        && console.set_cvar(crate::menu_pictures::CVAR, "1")
    {
        lines.push("The menus use pictures to save video memory (Settings, Menu pictures).".into());
    }
    lines
}

/// Give a card of `dedicated` bytes under 3.5 GiB lighter graphics, once a card: Vanilla
/// when the preset is the untouched Vanilla Plus one (kept so Back to SJK
/// restores it, [`crate::graphics_quality::toggle_vanilla`]), stock blades for other
/// players and the menus' 60 FPS cap when those are off. What changed.
fn lighten(console: &mut ViewerConsole, dedicated: u64) -> Lightened {
    let mib = (dedicated / MIB) as i64;
    let mut lightened = Lightened {
        mib,
        ..Lightened::default()
    };
    if picmip_for(dedicated) != Some(2) || console.integer_cvar(PRESET_CVAR) == Some(mib) {
        return lightened;
    }
    console.set_cvar(PRESET_CVAR, &mib.to_string());
    let detail = crate::world_materials::filtering::mips::PICMIP_CVAR;
    lightened.smaller_textures = console.integer_cvar(detail).unwrap_or(0) > 0;
    use crate::graphics_quality::Level;
    if Level::current(console) == Some(Level::VanillaPlus) {
        crate::graphics_quality::toggle_vanilla(console);
        lightened.vanilla = Level::current(console) == Some(Level::Vanilla);
    }
    let stock = crate::stock_blades::CVAR;
    if console.integer_cvar(stock).unwrap_or(0) == 0 {
        lightened.stock_blades = console.set_cvar(stock, "1");
    }
    let cap = crate::runtime_settings::MENU_CAP_CVAR;
    if console.integer_cvar(cap).unwrap_or(0) == 0 {
        lightened.menu_cap = console.set_cvar(cap, MENU_FPS);
    }
    lightened
}

/// What the lighter graphics changed at this start, if anything; the card shows it.
pub(crate) fn lightened() -> Option<Lightened> {
    LIGHTENED.get().copied()
}

/// Say `lightened` changed at this start, for world shots of the card.
#[cfg(test)]
pub(crate) fn lighten_for_shot(lightened: Lightened) {
    let _ = LIGHTENED.set(lightened);
}

/// Undo what [`lighten`] changed: Vanilla Plus back with the player's kept values,
/// and the stock blades and menu cap off again where it set them.
pub(crate) fn undo(console: &mut ViewerConsole, lightened: Lightened) {
    if lightened.vanilla && crate::graphics_quality::vanilla(console) {
        crate::graphics_quality::set_renderer(console, crate::graphics_quality::SJK);
    }
    if lightened.stock_blades {
        console.set_cvar(crate::stock_blades::CVAR, "0");
    }
    if lightened.menu_cap {
        console.set_cvar(crate::runtime_settings::MENU_CAP_CVAR, "0");
    }
}

/// The card's video memory for crash reports, once read.
pub(crate) fn summary() -> Option<&'static str> {
    SUMMARY.get().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn fresh() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn small_cards_start_on_smaller_textures() {
        assert_eq!(picmip_for(512 * MIB), Some(2), "an integrated GPU");
        assert_eq!(picmip_for(2 * GIB), Some(2));
        assert_eq!(picmip_for(3 * GIB), Some(2));
        assert_eq!(picmip_for(4 * GIB), Some(1));
        assert_eq!(picmip_for(6 * GIB), None);
        assert_eq!(picmip_for(0), None, "a driver that does not say");
    }

    #[test]
    fn it_chooses_once_a_card_and_keeps_what_the_player_set() {
        let (_directory, mut console) = fresh();
        let detail = crate::world_materials::filtering::mips::PICMIP_CVAR;
        assert_eq!(apply(&mut console, 3 * GIB).len(), 2);
        assert_eq!(console.integer_cvar(detail), Some(2));
        assert!(crate::menu_pictures::enabled(Some(&console)));
        // The player sets it back: the same card does not change it again.
        console.set_cvar(detail, "0");
        assert!(apply(&mut console, 3 * GIB).is_empty());
        assert_eq!(console.integer_cvar(detail), Some(0));
        // A large card changes nothing.
        let (_other, mut console) = fresh();
        assert!(apply(&mut console, 8 * GIB).is_empty());
        assert_eq!(console.integer_cvar(detail), Some(0));
        assert_eq!(console.integer_cvar(CVAR), Some(8192));
    }

    #[test]
    fn a_small_card_gets_lighter_graphics_once_and_can_go_back() {
        use crate::graphics_quality::Level;
        let (_directory, mut console) = fresh();
        crate::graphics_quality::apply_new_profile(&mut console);
        // A card the texture detail was already chosen for is still considered.
        console.set_cvar(CVAR, "3072");
        let lightened = lighten(&mut console, 3 * GIB);
        assert_eq!(
            lightened,
            Lightened {
                mib: 3072,
                vanilla: true,
                stock_blades: true,
                menu_cap: true,
                smaller_textures: false
            }
        );
        assert_eq!(Level::current(&console), Some(Level::Vanilla));
        assert_eq!(console.integer_cvar(crate::stock_blades::CVAR), Some(1));
        assert_eq!(
            console.integer_cvar(crate::runtime_settings::MENU_CAP_CVAR),
            Some(60)
        );
        // Once a card: the next start changes nothing.
        assert!(!lighten(&mut console, 3 * GIB).any());
        // Back to SJK: the preset and the two settings as they were.
        undo(&mut console, lightened);
        assert_eq!(Level::current(&console), Some(Level::VanillaPlus));
        assert_eq!(console.integer_cvar(crate::stock_blades::CVAR), Some(0));
        assert_eq!(
            console.integer_cvar(crate::runtime_settings::MENU_CAP_CVAR),
            Some(0)
        );
    }

    #[test]
    fn only_the_untouched_preset_and_small_cards_are_lightened() {
        use crate::graphics_quality::Level;
        // A player's own settings are left alone; only what is still off changes.
        let (_directory, mut console) = fresh();
        Level::SolarHigh.apply(&mut console);
        console.set_cvar(crate::runtime_settings::MENU_CAP_CVAR, "90");
        let lightened = lighten(&mut console, 2 * GIB);
        assert!(!lightened.vanilla && lightened.stock_blades && !lightened.menu_cap);
        assert_eq!(Level::current(&console), Some(Level::SolarHigh));
        assert_eq!(
            console.integer_cvar(crate::runtime_settings::MENU_CAP_CVAR),
            Some(90)
        );
        // A 4 GB card and a large one keep everything.
        for size in [4 * GIB, 8 * GIB] {
            let (_directory, mut console) = fresh();
            crate::graphics_quality::apply_new_profile(&mut console);
            assert!(!lighten(&mut console, size).any());
            assert_eq!(Level::current(&console), Some(Level::VanillaPlus));
        }
    }
}
