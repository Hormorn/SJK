//! Where the player has moved the HUD's elements (Lex's idea, #ideas 29/10/2026: move
//! the HUD with the mouse). The HUD edit mode ([`crate::hud_edit`]) drags them; the
//! HUD applies the offsets as it draws ([`super::HudOverlay::layout`]).
//!
//! An element that already has position cvars of its own (the lagometer's
//! `cg_lagometerX`/`Y`, the movement keys', the speedometer's, the kill feed's, the
//! personal score's, the chat's `cg_chatBoxX` and `cg_chatBoxHeight`) is moved by
//! changing those, in their own units ([`Binding::Cvars`]), so the two never disagree.
//! The others keep an offset in `cg_hudOffsets` ([`OFFSETS_CVAR`]): `id:x,y` pairs
//! separated by spaces, in units of the 480-line virtual screen (a unit is the
//! screen's height over 480 on both axes, as the cvars above count), right and down
//! positive. An element left out is where its HUD puts it.

use crate::console::ViewerConsole;
use sjk_ui::Rect;
use std::fmt::Write;

/// The offsets of the elements without position cvars of their own.
pub(crate) const OFFSETS_CVAR: &str = "cg_hudOffsets";
/// The farthest an offset goes, in 480-line units: past it the element would leave
/// even a 32:9 screen.
pub(crate) const OFFSET_LIMIT: f32 = 1_000.0;

/// One part of the HUD the player can move.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Element {
    /// Health and armour: the left meters (the radial look's left pill and arcs, the
    /// game HUD's left block).
    Vitals,
    /// Force and the saber style (the radial look's right pill, its ammo with them; the
    /// game HUD's right block, with its ammo and score line).
    Force,
    /// The weapon's name and ammo (the radial look's weapon name).
    Weapon,
    /// The match clock under the top's middle.
    Timer,
    /// The vote panel, while a vote runs.
    Vote,
    /// The jetpack's fuel bar.
    Jetpack,
    /// The vehicle's HP: SJK's vector strip, or the layouts' `vehicle_*` widgets.
    Vehicle,
    /// The weapon, Force and inventory selectors.
    Selection,
    /// The CTF flags and team scores.
    Flags,
    /// The name of the player followed.
    Follow,
    Lagometer,
    MovementKeys,
    Speedometer,
    KillFeed,
    /// The personal score line (`cg_drawScore`).
    Score,
    Chat,
    /// The local time (`cg_drawClock`, 05/11/2026).
    Clock,
    /// The frame timings panel (`r_drawTimings`, [`crate::frame_timings`]): from the
    /// top right corner.
    Timings,
}

/// How many [`Element`]s there are.
pub(crate) const COUNT: usize = Element::ALL.len();

/// How many pixels one unit of a position cvar is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Unit {
    /// The screen's height over 480.
    Height480,
    /// The screen's width over 640 (a 640-wide virtual screen stretched).
    Width640,
    /// EternalJK's aspect-corrected 640-wide unit: the smaller of the two above.
    Aspect640,
}

impl Unit {
    pub(crate) fn pixels(self, viewport: [f32; 2]) -> f32 {
        let [width, height] = viewport;
        match self {
            Self::Height480 => height / 480.0,
            Self::Width640 => width / 640.0,
            Self::Aspect640 => (height / 480.0).min(width / 640.0),
        }
    }
}

/// One axis of an element's own position cvars: its name, whether it grows with the
/// screen axis (1) or against it (-1, a distance from the right or bottom), its unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Axis {
    pub(crate) cvar: &'static str,
    pub(crate) sign: f32,
    pub(crate) unit: Unit,
}

impl Axis {
    const fn new(cvar: &'static str, sign: f32, unit: Unit) -> Self {
        Self { cvar, sign, unit }
    }

    /// The cvar's value that moves the element `pixels` from where `start` put it.
    pub(crate) fn moved(self, start: f32, pixels: f32, viewport: [f32; 2]) -> f32 {
        let unit = self.unit.pixels(viewport).max(1e-3);
        start + self.sign * pixels / unit
    }
}

/// Where an element's position is kept.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Binding {
    /// In [`OFFSETS_CVAR`].
    Free,
    /// In its own cvars, x then y.
    Cvars([Axis; 2]),
}

impl Element {
    pub(crate) const ALL: [Self; 18] = [
        Self::Vitals,
        Self::Force,
        Self::Weapon,
        Self::Timer,
        Self::Vote,
        Self::Jetpack,
        Self::Vehicle,
        Self::Selection,
        Self::Flags,
        Self::Follow,
        Self::Lagometer,
        Self::MovementKeys,
        Self::Speedometer,
        Self::KillFeed,
        Self::Score,
        Self::Chat,
        Self::Clock,
        Self::Timings,
    ];

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    /// The name in [`OFFSETS_CVAR`] and the `hud_edit reset` command.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Vitals => "vitals",
            Self::Force => "force",
            Self::Weapon => "weapon",
            Self::Timer => "timer",
            Self::Vote => "vote",
            Self::Jetpack => "jetpack",
            Self::Vehicle => "vehicle",
            Self::Selection => "selection",
            Self::Flags => "flags",
            Self::Follow => "follow",
            Self::Lagometer => "lagometer",
            Self::MovementKeys => "movementkeys",
            Self::Speedometer => "speedometer",
            Self::KillFeed => "killfeed",
            Self::Score => "score",
            Self::Chat => "chat",
            Self::Clock => "clock",
            Self::Timings => "timings",
        }
    }

    /// What the edit mode's label says.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Vitals => "Health and armour",
            Self::Force => "Force",
            Self::Weapon => "Weapon",
            Self::Timer => "Match timer",
            Self::Vote => "Vote",
            Self::Jetpack => "Jetpack fuel",
            Self::Vehicle => "Vehicle",
            Self::Selection => "Selection",
            Self::Flags => "Flags",
            Self::Follow => "Following",
            Self::Lagometer => "Lagometer",
            Self::MovementKeys => "Movement keys",
            Self::Speedometer => "Speedometer",
            Self::KillFeed => "Kill feed",
            Self::Score => "Score",
            Self::Chat => "Chat",
            Self::Clock => "Local time",
            Self::Timings => "Frame timings",
        }
    }

    pub(crate) fn from_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|element| element.id().eq_ignore_ascii_case(id))
    }

    pub(crate) const fn binding(self) -> Binding {
        use Unit::*;
        match self {
            // TaystJK's distances from the right and the bottom (`targeting.rs`).
            Self::Lagometer => Binding::Cvars([
                Axis::new("cg_lagometerX", -1.0, Height480),
                Axis::new("cg_lagometerY", -1.0, Height480),
            ]),
            // JoF EJK's top left corner, its x aspect-corrected (`movement_keys.rs`).
            Self::MovementKeys => Binding::Cvars([
                Axis::new("cg_movementKeysX", 1.0, Aspect640),
                Axis::new("cg_movementKeysY", 1.0, Height480),
            ]),
            // The text's right edge and baseline (`options.rs`).
            Self::Speedometer => Binding::Cvars([
                Axis::new("cg_speedometerX", 1.0, Width640),
                Axis::new("cg_speedometerY", 1.0, Height480),
            ]),
            // Leftward and downward (`kill_feed.rs`).
            Self::KillFeed => Binding::Cvars([
                Axis::new("cg_killfeedX", -1.0, Width640),
                Axis::new("cg_killfeedY", 1.0, Height480),
            ]),
            // Rightward and upward (`family.rs`).
            Self::Score => Binding::Cvars([
                Axis::new("cg_drawScoreX", 1.0, Width640),
                Axis::new("cg_drawScoreY", -1.0, Height480),
            ]),
            // The chat's left and its bottom (`chat/options.rs`).
            Self::Chat => Binding::Cvars([
                Axis::new("cg_chatBoxX", 1.0, Width640),
                Axis::new("cg_chatBoxHeight", 1.0, Height480),
            ]),
            // The text's centre and bottom (`clock.rs`).
            Self::Clock => Binding::Cvars([
                Axis::new(super::clock::X_CVAR, 1.0, Width640),
                Axis::new(super::clock::Y_CVAR, 1.0, Height480),
            ]),
            _ => Binding::Free,
        }
    }
}

/// The element a layout document's widget belongs to, by its id. The radial look's
/// ammo sits in its right pill with the Force, so it moves with them; the crosshair,
/// its name, the warm-up and the interruption notice stay in the middle; the
/// lagometer has its own cvars, which its widget ignores.
pub(crate) fn widget_element(id: &str, radial: bool) -> Option<Element> {
    Some(match id {
        "health_value" | "health_meter" | "health_label" | "health_arc" | "armor_value"
        | "armor_meter" | "armor_label" | "armor_arc" | "left_pill" => Element::Vitals,
        "ammo_value" | "ammo_arc" if radial => Element::Force,
        "force_value" | "force_meter" | "force_label" | "force_arc" | "style_value"
        | "style_arc" | "right_pill" => Element::Force,
        "weapon_value" | "weapon_label" | "ammo_value" | "ammo_label" | "ammo_arc" => {
            Element::Weapon
        }
        "vehicle_arc"
        | "vehicle_value"
        | "vehicle_label"
        | "vehicle_meter"
        | "vehicle_speed_arc"
        | "vehicle_speed_value"
        | "vehicle_speed_label"
        | "vehicle_speed_meter"
        | "vehicle_ammo_arc"
        | "vehicle_ammo_value"
        | "vehicle_ammo_label"
        | "vehicle_ammo_meter"
        | "vehicle_turbo_arc_charging"
        | "vehicle_turbo_arc_ready"
        | "vehicle_turbo_arc_burning"
        | "vehicle_turbo_value"
        | "vehicle_turbo_label"
        | "vehicle_turbo_meter_charging"
        | "vehicle_turbo_meter_ready"
        | "vehicle_turbo_meter_burning" => Element::Vehicle,
        "match_timer" => Element::Timer,
        "vote_panel" => Element::Vote,
        // The classic layout's groups: health and armour, Force, the held weapon and
        // the vehicle, each moving as one.
        id if id.starts_with("vitals_") => Element::Vitals,
        id if id.starts_with("forcebox_") => Element::Force,
        id if id.starts_with("weaponbox_") => Element::Weapon,
        id if id.starts_with("vehicle_") => Element::Vehicle,
        _ => return None,
    })
}

/// The [`Binding::Free`] elements' offsets, in 480-line units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Offsets([[f32; 2]; COUNT]);

impl Offsets {
    /// Read `cg_hudOffsets`'s text; what it cannot read, or names no free element,
    /// is left out. Allocates nothing.
    pub(crate) fn parse(text: &str) -> Self {
        let mut offsets = Self::default();
        for entry in text.split_whitespace() {
            let Some((id, value)) = entry.split_once(':') else {
                continue;
            };
            let Some((x, y)) = value.split_once(',') else {
                continue;
            };
            let (Some(element), Ok(x), Ok(y)) =
                (Element::from_id(id), x.parse::<f32>(), y.parse::<f32>())
            else {
                continue;
            };
            if element.binding() == Binding::Free && x.is_finite() && y.is_finite() {
                offsets.set(element, [x, y]);
            }
        }
        offsets
    }

    /// The offsets as `cg_hudOffsets` keeps them: the moved elements only, in
    /// [`Element::ALL`]'s order, to a tenth of a unit.
    pub(crate) fn format(&self) -> String {
        let mut text = String::new();
        for element in Element::ALL {
            let [x, y] = self.get(element);
            if x == 0.0 && y == 0.0 {
                continue;
            }
            if !text.is_empty() {
                text.push(' ');
            }
            let _ = write!(text, "{}:{},{}", element.id(), tenth(x), tenth(y));
        }
        text
    }

    /// What the console's `cg_hudOffsets` says.
    pub(crate) fn read(console: Option<&ViewerConsole>) -> Self {
        console
            .and_then(|console| console.text_cvar(OFFSETS_CVAR).ok())
            .map(Self::parse)
            .unwrap_or_default()
    }

    pub(crate) fn get(&self, element: Element) -> [f32; 2] {
        self.0[element.index()]
    }

    /// Set an element's offset, held within [`OFFSET_LIMIT`].
    pub(crate) fn set(&mut self, element: Element, offset: [f32; 2]) {
        self.0[element.index()] = offset.map(|value| value.clamp(-OFFSET_LIMIT, OFFSET_LIMIT));
    }

    /// An element's offset in pixels on a `viewport`-sized screen.
    pub(crate) fn pixels(&self, element: Element, viewport: [f32; 2]) -> [f32; 2] {
        let unit = Unit::Height480.pixels(viewport);
        self.get(element).map(|value| value * unit)
    }
}

/// `value` rounded to a tenth, without a trailing ".0" or a "-0".
fn tenth(value: f32) -> f32 {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

/// The box each element drew in this frame, in pixels, for the edit mode.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Bounds([Option<Rect>; COUNT]);

impl Bounds {
    pub(crate) fn clear(&mut self) {
        self.0 = [None; COUNT];
    }

    /// Grow `element`'s box to hold `rect`.
    pub(crate) fn add(&mut self, element: Element, rect: Option<Rect>) {
        let Some(rect) = rect.filter(|rect| rect.width > 0.0 || rect.height > 0.0) else {
            return;
        };
        let slot = &mut self.0[element.index()];
        *slot = Some(slot.map_or(rect, |old| old.union(rect)));
    }

    pub(crate) fn get(&self, element: Element) -> Option<Rect> {
        self.0[element.index()]
    }

    /// The elements drawn, with their boxes.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Element, Rect)> + '_ {
        Element::ALL
            .into_iter()
            .filter_map(|element| self.get(element).map(|rect| (element, rect)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text the cvar keeps reads back as the same offsets; what it cannot read is
    /// left out, and elements with cvars of their own are not kept in it.
    #[test]
    fn offsets_read_back_what_they_write() {
        let mut offsets = Offsets::default();
        assert_eq!(offsets.format(), "");
        offsets.set(Element::Vitals, [12.34, -5.0]);
        offsets.set(Element::Jetpack, [0.0, 40.0]);
        let text = offsets.format();
        assert_eq!(text, "vitals:12.3,-5 jetpack:0,40");
        let read = Offsets::parse(&text);
        assert_eq!(read.get(Element::Vitals), [12.3, -5.0]);
        assert_eq!(read.get(Element::Jetpack), [0.0, 40.0]);
        assert_eq!(read.get(Element::Force), [0.0, 0.0]);
        let messy = Offsets::parse(
            "  VITALS:1,2 nonsense force:x,3 lagometer:5,5 weapon:inf,1 timer:-7.5,0.04",
        );
        assert_eq!(messy.get(Element::Vitals), [1.0, 2.0]);
        assert_eq!(messy.get(Element::Force), [0.0, 0.0]);
        assert_eq!(messy.get(Element::Lagometer), [0.0, 0.0]);
        assert_eq!(messy.get(Element::Weapon), [0.0, 0.0]);
        assert_eq!(messy.format(), "vitals:1,2 timer:-7.5,0");
    }

    /// An offset is held within the limit, and is the screen's height over 480 pixels
    /// a unit on both axes.
    #[test]
    fn offsets_are_held_and_scale_with_the_height() {
        let mut offsets = Offsets::default();
        offsets.set(Element::Timer, [5_000.0, -10.0]);
        assert_eq!(offsets.get(Element::Timer), [OFFSET_LIMIT, -10.0]);
        offsets.set(Element::Vote, [10.0, 20.0]);
        assert_eq!(
            offsets.pixels(Element::Vote, [1_920.0, 1_080.0]),
            [22.5, 45.0]
        );
        assert_eq!(offsets.pixels(Element::Vote, [640.0, 480.0]), [10.0, 20.0]);
    }

    /// Dragging a cvar-bound element moves its cvar by the pixels over its unit, the
    /// right way round.
    #[test]
    fn cvar_axes_follow_their_units_and_directions() {
        let viewport = [1_920.0, 1_080.0];
        let Binding::Cvars([x, y]) = Element::Lagometer.binding() else {
            panic!("the lagometer has cvars");
        };
        // 45 pixels right is 20 units less from the right edge.
        assert!((x.moved(48.0, 45.0, viewport) - 28.0).abs() < 1e-4);
        assert!((y.moved(144.0, -45.0, viewport) - 164.0).abs() < 1e-4);
        let Binding::Cvars([x, _]) = Element::KillFeed.binding() else {
            panic!("the kill feed has cvars");
        };
        // 30 pixels left is 10 units more leftward on a 1920-wide screen.
        assert!((x.moved(0.0, -30.0, viewport) - 10.0).abs() < 1e-4);
        let Binding::Cvars([x, _]) = Element::MovementKeys.binding() else {
            panic!("the movement keys have cvars");
        };
        assert_eq!(x.unit.pixels(viewport), 2.25);
        assert_eq!(x.unit.pixels([1_280.0, 1_024.0]), 2.0);
        assert_eq!(Element::Vitals.binding(), Binding::Free);
    }

    /// Every element's id names it back, and the layouts' status widgets each belong
    /// to one, the radial ammo with the Force.
    #[test]
    fn ids_and_widgets_name_their_elements() {
        for element in Element::ALL {
            assert_eq!(Element::from_id(element.id()), Some(element));
            assert_eq!(Element::ALL[element.index()], element);
        }
        assert_eq!(widget_element("ammo_arc", true), Some(Element::Force));
        assert_eq!(widget_element("ammo_value", false), Some(Element::Weapon));
        assert_eq!(widget_element("left_pill", true), Some(Element::Vitals));
        assert_eq!(widget_element("vehicle_arc", true), Some(Element::Vehicle));
        assert_eq!(
            widget_element("vitals_health_meter", false),
            Some(Element::Vitals)
        );
        assert_eq!(
            widget_element("forcebox_meter", false),
            Some(Element::Force)
        );
        assert_eq!(
            widget_element("weaponbox_name", false),
            Some(Element::Weapon)
        );
        assert_eq!(
            widget_element("vehicle_ammo2_meter", false),
            Some(Element::Vehicle)
        );
        assert_eq!(widget_element("crosshair", false), None);
        assert_eq!(widget_element("lagometer", false), None);
        for layout in [
            super::super::DEFAULT_LAYOUT,
            super::super::CLASSIC_LAYOUT,
            super::super::RADIAL_LAYOUT,
        ] {
            let document = sjk_ui::HudLayoutDocument::from_json(layout).unwrap();
            for widget in &document.widgets {
                let status = widget.binding.as_deref().is_some_and(|binding| {
                    binding.ends_with("_ratio") || binding.ends_with("_value")
                });
                if status {
                    assert!(widget_element(&widget.id, true).is_some(), "{}", widget.id);
                    assert!(widget_element(&widget.id, false).is_some(), "{}", widget.id);
                }
            }
        }
    }

    /// An element's box grows to hold what it drew, and empty boxes are left out.
    #[test]
    fn bounds_hold_what_each_element_drew() {
        let mut bounds = Bounds::default();
        bounds.add(Element::Chat, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
        bounds.add(Element::Chat, Some(Rect::new(20.0, 5.0, 10.0, 10.0)));
        bounds.add(Element::Timer, Some(Rect::new(1.0, 1.0, 0.0, 0.0)));
        bounds.add(Element::Vote, None);
        assert_eq!(
            bounds.get(Element::Chat),
            Some(Rect::new(0.0, 0.0, 30.0, 15.0))
        );
        assert_eq!(bounds.iter().count(), 1);
        bounds.clear();
        assert_eq!(bounds.iter().count(), 0);
    }
}
