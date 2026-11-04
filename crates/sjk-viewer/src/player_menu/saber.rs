//! Draft state and catalogue filtering for the saber page.

use super::controller::wrap;
use super::rows::SaberRow;
use super::*;
use crate::saber::Color;
use sjk_client::{LegacySaberDefinition, LegacySaberType, pack_saber_rgb, unpack_saber_rgb};

/// `color1`/`color2` value selecting a custom RGB tint (JA+/TaystJK
/// `SABER_RGB`).
pub(super) const RGB_COLOR_INDEX: u8 = 6;
/// Keyboard step of one RGB channel.
const CHANNEL_STEP: isize = 5;
/// `color1`/`color2` value of JA+'s black blade (`SABER_BLACK`), drawn black on
/// JA+, jaPRO and TaystJK servers with `cg_blackSabers` on (`docs/client.md`, "JA+
/// black sabers"); other servers show it as before (hub report #130).
pub(super) const BLACK_COLOR_INDEX: u8 = 11;
/// The blade colour palette: the six stock colours, the custom chip, then JA+'s
/// black, last. A colour's chip is its position here, not its number.
pub(super) const PALETTE: [u8; 8] = [0, 1, 2, 3, 4, 5, RGB_COLOR_INDEX, BLACK_COLOR_INDEX];
/// Near black, so the blade's preview and the chip still show on dark menus.
const BLACK_RGB: [u8; 3] = [16, 16, 18];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SaberStyle {
    #[default]
    Single,
    Staff,
    Dual,
}

impl SaberStyle {
    const ALL: [Self; 3] = [Self::Single, Self::Staff, Self::Dual];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::Staff => "Staff",
            Self::Dual => "Dual",
        }
    }

    /// Ready stance the game idles in with this style's sabers drawn
    /// (`PM_GetSaberStance`, `codemp/game/bg_pmove.c:276-333`: dual and
    /// staff have their own stances, every single-saber style but fast and
    /// strong stands in `BOTH_STAND2`).
    pub(super) fn stance(self) -> &'static str {
        match self {
            Self::Single => "BOTH_STAND2",
            Self::Staff => "BOTH_SABERSTAFF_STANCE",
            Self::Dual => "BOTH_SABERDUAL_STANCE",
        }
    }
}

/// What the stage model holds: the `.sab` name in each hand (`None` for an
/// empty hand), the blade tint of each saber, the stance to idle in, and
/// whether the sabers are thrown out to the backdrop's saber shot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StageSabers<'a> {
    pub(crate) hilts: [Option<&'a str>; 2],
    pub(crate) colors: [[u8; 3]; 2],
    /// Whether each hand's blade is JA+'s black (colour 11), drawn black rather than
    /// as its near-black tint, which the additive blade pass shows white (#130).
    pub(crate) black: [bool; 2],
    pub(crate) stance: &'static str,
    /// The style the hilts are held in, which picks the Saber Lab's swings.
    pub(crate) style: SaberStyle,
    pub(crate) thrown: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SaberDraft {
    style: SaberStyle,
    primary: String,
    secondary: String,
    color1: u8,
    color2: u8,
    /// Custom tints behind `cp_sbRGB1`/`cp_sbRGB2`; `None` while the cvar
    /// is still 0 (never chosen), so switching to RGB can start from the
    /// colour that was selected before.
    rgb: [Option<[u8; 3]>; 2],
}

impl Default for SaberDraft {
    fn default() -> Self {
        Self {
            style: SaberStyle::Single,
            primary: "single_1".to_owned(),
            secondary: "none".to_owned(),
            color1: 4,
            color2: 4,
            rgb: [None; 2],
        }
    }
}

/// Blade tint of a `color1`/`color2` value with its custom RGB.
fn blade_rgb(index: u8, rgb: Option<[u8; 3]>) -> [u8; 3] {
    if index == BLACK_COLOR_INDEX {
        return BLACK_RGB;
    }
    match Color::ALL.get(usize::from(index)) {
        Some(color) => color.blade_rgb(),
        None => rgb.unwrap_or_else(|| unpack_saber_rgb(0)),
    }
}

/// A `color1`/`color2` number as the menu keeps it: a stock colour, black, or the
/// custom chip for anything else (JA+'s flame and electric blades among them).
fn menu_colour(index: u8) -> u8 {
    if index == BLACK_COLOR_INDEX {
        index
    } else {
        index.min(RGB_COLOR_INDEX)
    }
}

/// The chip of a menu colour in [`PALETTE`].
pub(super) fn palette_position(index: u8) -> usize {
    PALETTE
        .iter()
        .position(|colour| *colour == index)
        .unwrap_or(usize::from(RGB_COLOR_INDEX))
}

pub(super) struct SaberMenu {
    draft: SaberDraft,
    /// Words a listed hilt's name must contain (the SJK UI's hilt search; empty
    /// lists every hilt the style allows).
    search: String,
}

impl SaberMenu {
    pub(super) fn new() -> Self {
        Self {
            draft: SaberDraft::default(),
            search: String::with_capacity(32),
        }
    }

    /// The hilt search as typed.
    pub(super) fn search(&self) -> &str {
        &self.search
    }

    /// Change the hilt search to `text`.
    pub(super) fn set_search(&mut self, text: &str) {
        if self.search != text {
            self.search.clear();
            self.search.push_str(text);
        }
    }

    /// The hilts the first saber's list (`second` false) or the second's offers,
    /// in the catalogue's order (the game's load order), with each one's place
    /// among the hilts the style allows (what [`Self::select`] takes): the
    /// style's hilts whose name or file name holds every word of the search.
    pub(super) fn listed<'a>(
        &'a self,
        catalog: &'a LegacyAssetCatalog,
        second: bool,
    ) -> impl Iterator<Item = (usize, &'a LegacySaberDefinition)> + 'a {
        let style = self.hand_style(second);
        catalog
            .saber_hilts
            .iter()
            .filter(move |hilt| allowed(hilt, style))
            .enumerate()
            .filter(|(_, hilt)| {
                let search = self.search.trim();
                search.is_empty()
                    || super::team_filter::search_matches(search, &hilt.display_name)
                    || super::team_filter::search_matches(search, &hilt.name)
            })
    }

    /// The style a hand's hilts must suit: the second saber is a single one.
    fn hand_style(&self, second: bool) -> SaberStyle {
        if second {
            SaberStyle::Single
        } else {
            self.draft.style
        }
    }

    /// Read the saber cvars again when `saber1` or `saber2` no longer say what the draft
    /// holds (the `saber` command, or another screen, changed them), so the stage holds what
    /// the player does.
    pub(super) fn follow(&mut self, console: &ViewerConsole) {
        let same = |cvar: &str, held: &str| {
            console
                .text_value(cvar)
                .is_none_or(|value| value.eq_ignore_ascii_case(held))
        };
        if !same("saber1", &self.draft.primary)
            || !same("saber2", &self.draft.secondary)
            || self.colours_moved(console)
        {
            self.open(console);
        }
    }

    /// Whether `color1`/`color2` or the custom tints no longer say what the draft holds:
    /// the `saberColor` command, a config or another screen changed the colour while the
    /// hilts stayed (Neffie: an RGB blade stayed red on the stage until a slider moved).
    fn colours_moved(&self, console: &ViewerConsole) -> bool {
        let colours = [
            menu_colour(colour_cvar(console, "color1")),
            menu_colour(colour_cvar(console, "color2")),
        ];
        let tints = ["cp_sbRGB1", "cp_sbRGB2"].map(|cvar| {
            console
                .integer_cvar(cvar)
                .and_then(|value| u32::try_from(value).ok())
                .filter(|packed| *packed != 0)
                .map(unpack_saber_rgb)
        });
        colours != [self.draft.color1, self.draft.color2] || tints != self.draft.rgb
    }

    pub(super) fn open(&mut self, console: &ViewerConsole) {
        self.draft.primary = console
            .text_value("saber1")
            .unwrap_or("single_1")
            .to_owned();
        self.draft.secondary = console.text_value("saber2").unwrap_or("none").to_owned();
        self.draft.color1 = menu_colour(colour_cvar(console, "color1"));
        self.draft.color2 = menu_colour(colour_cvar(console, "color2"));
        for (slot, cvar) in self.draft.rgb.iter_mut().zip(["cp_sbRGB1", "cp_sbRGB2"]) {
            let packed = console
                .integer_cvar(cvar)
                .and_then(|value| u32::try_from(value).ok());
            *slot = packed.filter(|packed| *packed != 0).map(unpack_saber_rgb);
        }
        self.draft.style = if self.draft.secondary.eq_ignore_ascii_case("none") {
            SaberStyle::Single
        } else {
            SaberStyle::Dual
        };
    }

    /// A `saber1` that names a staff hilt is a Staff profile, which the
    /// cvars alone cannot tell from Single.
    pub(super) fn reconcile_style(&mut self, catalog: Option<&LegacyAssetCatalog>) {
        if self.draft.style == SaberStyle::Single
            && catalog
                .and_then(|catalog| find_hilt(catalog, &self.draft.primary))
                .is_some_and(|hilt| hilt.saber_type == LegacySaberType::Staff)
        {
            self.draft.style = SaberStyle::Staff;
        }
    }

    pub(super) fn style(&self) -> SaberStyle {
        self.draft.style
    }

    pub(super) fn hilt(&self, second: bool) -> &str {
        if second {
            &self.draft.secondary
        } else {
            &self.draft.primary
        }
    }

    pub(super) fn color(&self, second: bool) -> u8 {
        if second {
            self.draft.color2
        } else {
            self.draft.color1
        }
    }

    /// The tint the blade is drawn with: the stock triplet, or the custom
    /// RGB when the colour is RGB.
    pub(super) fn rgb(&self, second: bool) -> [u8; 3] {
        blade_rgb(self.color(second), self.stored_rgb(second))
    }

    /// The stored custom tint, unless it is black's own near-black tint ([`BLACK_RGB`]),
    /// which an earlier test build stored as the custom tint for black: that drew a
    /// second black chip that gave a white blade (Dave's test of #130, 28/10/2026).
    /// Black is its own chip; that exact tint counts as none. Any other tint is the
    /// player's, however dark (#230's review: a filter on darkness also dropped a real
    /// dark red stepped down from the sliders).
    fn stored_rgb(&self, second: bool) -> Option<[u8; 3]> {
        self.draft.rgb[usize::from(second)].filter(|rgb| *rgb != BLACK_RGB)
    }

    /// The custom tint the RGB chip shows: the stored one, else what the
    /// blade is drawn with now. Black is not a tint to start from (it drew a
    /// second black chip and a custom blade too dark to see): the stock blue then.
    pub(super) fn custom_rgb(&self, second: bool) -> [u8; 3] {
        self.stored_rgb(second)
            .unwrap_or_else(|| self.tint_to_nudge(second))
    }

    /// What the sliders show and start from: the blade's tint, or with black the
    /// custom one ([`Self::custom_rgb`]).
    fn tint_to_nudge(&self, second: bool) -> [u8; 3] {
        if self.color(second) == BLACK_COLOR_INDEX {
            self.stored_rgb(second)
                .unwrap_or_else(|| Color::Blue.blade_rgb())
        } else {
            self.rgb(second)
        }
    }

    /// One channel of the custom RGB.
    pub(super) fn channel(&self, second: bool, channel: usize) -> u8 {
        self.tint_to_nudge(second)[channel]
    }

    /// Set one channel; the colour becomes custom (RGB), starting from the
    /// tint that was showing, so a stock colour can be nudged directly.
    pub(super) fn set_channel(&mut self, second: bool, channel: usize, value: u8) {
        let mut rgb = self.tint_to_nudge(second);
        rgb[channel] = value;
        self.draft.rgb[usize::from(second)] = Some(rgb);
        self.select_color(second, RGB_COLOR_INDEX);
    }

    /// The draft as the stage model should hold it.
    pub(super) fn stage_sabers(&self, thrown: bool) -> StageSabers<'_> {
        fn hand(name: &str) -> Option<&str> {
            (!name.eq_ignore_ascii_case("none")).then_some(name)
        }
        StageSabers {
            hilts: [hand(&self.draft.primary), hand(&self.draft.secondary)],
            colors: [self.rgb(false), self.rgb(true)],
            black: [false, true].map(|second| self.color(second) == BLACK_COLOR_INDEX),
            stance: self.draft.style.stance(),
            style: self.draft.style,
            thrown,
        }
    }

    /// Display name of the hilt in `saber1`/`saber2`, falling back to the
    /// cvar value while the catalogue is unavailable.
    pub(super) fn hilt_label<'a>(
        &'a self,
        catalog: Option<&'a LegacyAssetCatalog>,
        second: bool,
    ) -> &'a str {
        let name = self.hilt(second);
        catalog
            .and_then(|catalog| find_hilt(catalog, name))
            .map_or(name, |hilt| hilt.display_name.as_str())
    }

    pub(super) fn set_style(&mut self, style: SaberStyle, catalog: Option<&LegacyAssetCatalog>) {
        self.draft.style = style;
        if let Some(catalog) = catalog {
            if !matches_style(find_hilt(catalog, &self.draft.primary), style) {
                if let Some(hilt) = catalog.saber_hilts.iter().find(|hilt| allowed(hilt, style)) {
                    self.draft.primary.clone_from(&hilt.name);
                }
            }
            if style == SaberStyle::Dual
                && !matches_style(
                    find_hilt(catalog, &self.draft.secondary),
                    SaberStyle::Single,
                )
            {
                if let Some(hilt) = catalog
                    .saber_hilts
                    .iter()
                    .find(|hilt| allowed(hilt, SaberStyle::Single))
                {
                    self.draft.secondary.clone_from(&hilt.name);
                }
            }
        }
        if style != SaberStyle::Dual {
            self.draft.secondary.clear();
            self.draft.secondary.push_str("none");
        }
    }

    /// Step `row` by `direction`, wrapping within the hilts the style allows.
    pub(super) fn adjust(
        &mut self,
        row: SaberRow,
        direction: isize,
        catalog: Option<&LegacyAssetCatalog>,
    ) {
        match row {
            SaberRow::Style => {
                let index = SaberStyle::ALL
                    .iter()
                    .position(|style| *style == self.draft.style)
                    .unwrap_or(0);
                let style = SaberStyle::ALL[wrap(index, direction, SaberStyle::ALL.len())];
                self.set_style(style, catalog);
            }
            SaberRow::Hilt | SaberRow::SecondHilt => {
                if let Some(catalog) = catalog {
                    self.cycle_hilt(catalog, row.second(), direction);
                }
            }
            SaberRow::Blade | SaberRow::SecondBlade => {
                let current = palette_position(self.color(row.second()));
                self.select_color(
                    row.second(),
                    PALETTE[wrap(current, direction, PALETTE.len())],
                );
            }
            _ => {
                let Some(channel) = row.channel() else {
                    return;
                };
                let current = isize::from(self.channel(row.second(), channel));
                let value = (current + direction * CHANNEL_STEP).clamp(0, 255) as u8;
                self.set_channel(row.second(), channel, value);
            }
        }
    }

    /// Step the hand's hilt `direction` through the hilts listed (the search's
    /// matches); a hilt the search leaves out steps in from the list's edge.
    fn cycle_hilt(&mut self, catalog: &LegacyAssetCatalog, second: bool, direction: isize) {
        let current = self.hilt(second);
        let (mut count, mut at, mut first, mut last) = (0, None, None, None);
        for (index, hilt) in self.listed(catalog, second) {
            if hilt.name.eq_ignore_ascii_case(current) {
                at = Some(count);
            }
            first = first.or(Some(index));
            last = Some(index);
            count += 1;
        }
        let target = match at {
            Some(at) => self
                .listed(catalog, second)
                .nth(wrap(at, direction, count))
                .map(|(index, _)| index),
            None if direction > 0 => first,
            None => last,
        };
        if let Some(index) = target {
            self.select(catalog, index, second);
        }
    }

    pub(super) fn select(&mut self, catalog: &LegacyAssetCatalog, index: usize, second: bool) {
        let style = self.hand_style(second);
        let Some(hilt) = catalog
            .saber_hilts
            .iter()
            .filter(|hilt| allowed(hilt, style))
            .nth(index)
        else {
            return;
        };
        if second {
            self.draft.secondary.clone_from(&hilt.name);
        } else {
            self.draft.primary.clone_from(&hilt.name);
        }
    }

    /// Choose a colour; picking RGB for the first time starts from the tint
    /// of the colour that was selected until then.
    pub(super) fn select_color(&mut self, second: bool, index: u8) {
        let index = menu_colour(index);
        let previous = self.tint_to_nudge(second);
        let slot = usize::from(second);
        if index == RGB_COLOR_INDEX && self.stored_rgb(second).is_none() {
            self.draft.rgb[slot] = Some(previous);
        }
        if second {
            self.draft.color2 = index;
        } else {
            self.draft.color1 = index;
        }
    }

    /// Write the saber cvars to the console. Called after every change.
    pub(super) fn apply(&mut self, console: &mut ViewerConsole) {
        console.set_cvar("saber1", &self.draft.primary);
        console.set_cvar("saber2", &self.draft.secondary);
        // The colour keys also carry the worn hat and cape (JoF EJK's
        // `CG_SetSaberColorCvar` keeps them the same way).
        for (cvar, colour) in [("color1", self.draft.color1), ("color2", self.draft.color2)] {
            let worn = console
                .text_value(cvar)
                .and_then(|value| sjk_client::split_color_value(value).1)
                .map(str::to_owned);
            let value = sjk_client::join_color_value(i64::from(colour), worn.as_deref());
            console.set_cvar(cvar, &value);
        }
        for (slot, cvar) in self.draft.rgb.iter().zip(["cp_sbRGB1", "cp_sbRGB2"]) {
            let packed = slot.map_or(0, pack_saber_rgb);
            console.set_cvar(cvar, &packed.to_string());
        }
    }
}

/// The saber colour of a `color1`/`color2` value (`atoi`, so a worn
/// cosmetic after the digits does not change it); blue when unset.
fn colour_cvar(console: &ViewerConsole, cvar: &str) -> u8 {
    console
        .text_value(cvar)
        .map(|value| sjk_client::split_color_value(value).0)
        .and_then(|colour| u8::try_from(colour).ok())
        .unwrap_or(4)
}

fn find_hilt<'a>(catalog: &'a LegacyAssetCatalog, name: &str) -> Option<&'a LegacySaberDefinition> {
    catalog
        .saber_hilts
        .iter()
        .find(|hilt| hilt.name.eq_ignore_ascii_case(name))
}

fn matches_style(hilt: Option<&LegacySaberDefinition>, style: SaberStyle) -> bool {
    hilt.is_some_and(|hilt| allowed(hilt, style))
}

pub(super) fn allowed(hilt: &LegacySaberDefinition, style: SaberStyle) -> bool {
    match style {
        SaberStyle::Staff => hilt.saber_type == LegacySaberType::Staff,
        SaberStyle::Single | SaberStyle::Dual => hilt.saber_type == LegacySaberType::Single,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tint the menu's draft shows, after its cvars were written and read back
    /// by a fresh menu, and as the userinfo sent to a server carries it.
    fn round_trip(start: u8, channel: usize, value: u8) -> ([u8; 3], [u8; 3], Option<u32>) {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", &start.to_string());
        let mut menu = SaberMenu::new();
        menu.open(&console);
        menu.set_channel(false, channel, value);
        let shown = menu.rgb(false);
        menu.apply(&mut console);
        let mut reopened = SaberMenu::new();
        reopened.open(&console);
        let userinfo = console.userinfo().unwrap();
        (shown, reopened.rgb(false), userinfo.saber_rgb[0])
    }

    /// Hub report #130: JA+'s black blade (11) is a chip of its own. A saved black
    /// opens as black (it used to read as the custom chip), is kept when written, and
    /// the arrows step through it; other numbers past 6 still read as custom.
    #[test]
    fn black_is_a_colour_of_its_own() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", "11");
        console.set_cvar("color2", "8");
        let mut menu = SaberMenu::new();
        menu.open(&console);
        assert_eq!(menu.color(false), BLACK_COLOR_INDEX);
        assert_eq!(
            menu.color(true),
            RGB_COLOR_INDEX,
            "a flame blade reads as custom"
        );
        assert_eq!(menu.rgb(false), BLACK_RGB);
        menu.apply(&mut console);
        assert_eq!(colour_cvar(&console, "color1"), BLACK_COLOR_INDEX);
        // Purple, then the custom chip, then black, last.
        assert_eq!(PALETTE[palette_position(5) + 1], RGB_COLOR_INDEX);
        assert_eq!(
            PALETTE[palette_position(RGB_COLOR_INDEX) + 1],
            BLACK_COLOR_INDEX
        );
        assert_eq!(palette_position(BLACK_COLOR_INDEX), PALETTE.len() - 1);
        menu.select_color(false, PALETTE[PALETTE.len() - 1]);
        assert_eq!(menu.color(false), BLACK_COLOR_INDEX);
    }

    /// Dave's test of #130: with black chosen the custom chip showed black too, and
    /// picking it gave a near-black custom blade. The custom chip and the sliders now
    /// keep a real tint (the stored one, else stock blue), and the custom blade
    /// starts from it.
    #[test]
    fn black_does_not_become_the_custom_tint() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", "11");
        let mut menu = SaberMenu::new();
        menu.open(&console);
        let blue = Color::Blue.blade_rgb();
        assert_eq!(menu.custom_rgb(false), blue, "the custom chip is not black");
        assert_eq!(menu.channel(false, 2), blue[2], "nor are the sliders");
        menu.select_color(false, RGB_COLOR_INDEX);
        assert_eq!(menu.rgb(false), blue, "the custom blade starts from blue");
        // A stored custom tint survives black and comes back with the custom chip.
        menu.set_channel(false, 0, 200);
        let tint = menu.rgb(false);
        menu.select_color(false, BLACK_COLOR_INDEX);
        assert_eq!(menu.custom_rgb(false), tint);
        menu.select_color(false, RGB_COLOR_INDEX);
        assert_eq!(menu.rgb(false), tint);
    }

    /// Dave's test of #130: the menu's model held white blades with black chosen, as
    /// the near-black tint went through the additive pass. The stage is told black.
    #[test]
    fn the_menu_model_holds_black_blades() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", "11");
        console.set_cvar("color2", "4");
        let mut menu = SaberMenu::new();
        menu.open(&console);
        assert_eq!(menu.stage_sabers(false).black, [true, false]);
        menu.select_color(true, BLACK_COLOR_INDEX);
        assert_eq!(menu.stage_sabers(false).black, [true, true]);
        menu.select_color(false, RGB_COLOR_INDEX);
        assert_eq!(menu.stage_sabers(false).black, [false, true]);
    }

    /// Dave's test of #130: black's own tint, stored as the custom tint by an earlier
    /// test build, drew a second black chip, which gave a white blade. It counts as no
    /// tint: the custom chip shows, and picking it starts from, the stock blue.
    #[test]
    fn a_black_custom_tint_is_no_custom_tint() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let packed = sjk_client::pack_saber_rgb([16, 16, 18]);
        console.set_cvar("cp_sbRGB1", &packed.to_string());
        console.set_cvar("color1", "4");
        let mut menu = SaberMenu::new();
        menu.open(&console);
        let blue = Color::Blue.blade_rgb();
        assert_eq!(menu.custom_rgb(false), blue, "the custom chip is not black");
        menu.select_color(false, RGB_COLOR_INDEX);
        assert_eq!(menu.rgb(false), blue, "the custom blade starts from blue");
        menu.apply(&mut console);
        assert_eq!(
            console.integer_cvar("cp_sbRGB1"),
            Some(i64::from(sjk_client::pack_saber_rgb(blue))),
            "and the black tint is replaced"
        );
    }

    /// #230's review: a dark custom tint the player chose stays theirs; stepping the
    /// red channel of [45, 0, 0] down keeps [35, 0, 0] in the menu, not white.
    #[test]
    fn a_dark_custom_tint_stays_the_players() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let packed = sjk_client::pack_saber_rgb([45, 0, 0]);
        console.set_cvar("cp_sbRGB1", &packed.to_string());
        console.set_cvar("color1", "6");
        let mut menu = SaberMenu::new();
        menu.open(&console);
        assert_eq!(menu.rgb(false), [45, 0, 0]);
        menu.set_channel(false, 0, 35);
        assert_eq!(menu.rgb(false), [35, 0, 0], "the blade");
        assert_eq!(menu.custom_rgb(false), [35, 0, 0], "the chip");
        assert_eq!(menu.channel(false, 0), 35, "the slider");
    }

    /// Neffie: an RGB colour set while the hilts stayed (`saberColor`, a config) showed on
    /// the stage only once a slider moved. The stage follows the colour cvars too.
    #[test]
    fn the_stage_follows_a_colour_set_outside_the_menu() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut menu = SaberMenu::new();
        menu.open(&console);
        menu.follow(&console);
        assert_eq!(menu.rgb(false), Color::Blue.blade_rgb());
        crate::saber_colour_command::run(&mut console, &["40", "200", "90"].map(str::to_owned))
            .unwrap();
        menu.follow(&console);
        assert_eq!(menu.stage_sabers(false).colors, [[40, 200, 90]; 2]);
        // Followed once, it holds still while nothing changes.
        let draft = menu.draft.clone();
        menu.follow(&console);
        assert_eq!(menu.draft, draft);
    }

    /// Kushina's report #126: sabers chosen with the `saber` command (a quick-wheel
    /// custom choice) are what Profile > Saber shows, not the menu's last pick.
    #[test]
    fn the_saber_command_is_what_the_menu_shows() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut menu = SaberMenu::new();
        menu.open(&console);
        menu.set_style(SaberStyle::Single, None);
        menu.apply(&mut console);
        let words = |line: &str| {
            line.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        crate::saber_hilt_command::run(&mut console, &words("single_2 single_3"), |_| true)
            .unwrap();
        let mut reopened = SaberMenu::new();
        reopened.open(&console);
        assert_eq!(reopened.style(), SaberStyle::Dual);
        assert_eq!(reopened.hilt(false), "single_2");
        assert_eq!(reopened.hilt(true), "single_3");
        crate::saber_hilt_command::run(&mut console, &words("single_4"), |_| true).unwrap();
        reopened.open(&console);
        assert_eq!(reopened.style(), SaberStyle::Single);
        assert_eq!(reopened.hilt(false), "single_4");
        assert_eq!(reopened.hilt(true), "none");
    }

    #[test]
    fn a_nudged_channel_keeps_the_other_two_and_reaches_the_server() {
        for start in 0..=5u8 {
            let stock = Color::ALL[usize::from(start)].blade_rgb();
            for channel in 0..3 {
                for value in (0..=255u8).step_by(5).chain([255]) {
                    let mut expected = stock;
                    expected[channel] = value;
                    let (shown, reopened, sent) = round_trip(start, channel, value);
                    assert_eq!(shown, expected, "colour {start} channel {channel}={value}");
                    let packed = pack_saber_rgb(expected);
                    // An all-zero tint is the packed 0 that reads as red, as in JA+.
                    if packed != 0 {
                        assert_eq!(reopened, expected, "reopened {start}/{channel}/{value}");
                        assert_eq!(sent, Some(packed), "sent {start}/{channel}/{value}");
                    }
                }
            }
        }
    }
}
