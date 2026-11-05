//! HUD data bindings and allocation-free formatting helpers.

use super::*;

/// Borrowed values and visibility predicates for the retained HUD document.
pub(super) struct WidgetData<'a> {
    /// visibility binding supplied by the owning HUD.
    pub(super) visibility: HudVisibility,
    /// ratios binding supplied by the owning HUD.
    pub(super) ratios: [f32; 3],
    /// How full the current weapon's ammunition is, `0..=1`; zero without ammunition.
    pub(super) ammo_ratio: f32,
    /// health binding supplied by the owning HUD.
    pub(super) health: &'a str,
    /// armor binding supplied by the owning HUD.
    pub(super) armor: &'a str,
    /// force binding supplied by the owning HUD.
    pub(super) force: &'a str,
    /// weapon binding supplied by the owning HUD.
    pub(super) weapon: &'a str,
    /// ammo binding supplied by the owning HUD.
    pub(super) ammo: &'a str,
    /// health value binding supplied by the owning HUD.
    pub(super) health_value: &'a str,
    /// armor value binding supplied by the owning HUD.
    pub(super) armor_value: &'a str,
    /// force value binding supplied by the owning HUD.
    pub(super) force_value: &'a str,
    /// weapon value binding supplied by the owning HUD.
    pub(super) weapon_value: &'a str,
    /// ammo value binding supplied by the owning HUD.
    pub(super) ammo_value: &'a str,
    /// The saber style shown beside the Force meter, while a saber is held.
    pub(super) saber_style: Option<u8>,
    /// Current opacity of the weapon-name transient.
    pub(super) weapon_alpha: f32,
    /// Retail's weapon selection row shows; it draws the weapon name itself, so the
    /// layouts' own weapon-name widgets step aside. Ammunition never does.
    pub(super) weapon_row: bool,
    /// vote active binding supplied by the owning HUD.
    pub(super) vote_active: bool,
    /// team vote active binding supplied by the owning HUD.
    pub(super) team_vote_active: bool,
    /// crosshair name binding supplied by the owning HUD.
    pub(super) crosshair_name: bool,
    /// timer binding supplied by the owning HUD.
    pub(super) timer: bool,
    /// warmup binding supplied by the owning HUD.
    pub(super) warmup: bool,
    /// interrupted binding supplied by the owning HUD.
    pub(super) interrupted: bool,
    /// The vehicle's hull, speed and ammunition meters while SJK's own look draws the
    /// vehicle HUD; `None` on foot, and when the game HUD or the vector strip draws it.
    pub(super) vehicle: Option<vehicle::Meters>,
    /// The held weapon counts ammunition (`ClientHudData::ammo` is set).
    pub(super) has_ammo: bool,
}

impl WidgetData<'_> {
    /// Weapon `slot`'s ammunition ratio while the vehicle HUD shows and it has that weapon.
    fn vehicle_ammo_slot(&self, slot: usize) -> Option<f32> {
        self.vehicle
            .filter(|_| self.visibility.vehicle)
            .and_then(|meters| meters.ammo_slots[slot])
    }

    /// The vehicle HUD shows (SJK's own look, a piloted vehicle).
    fn vehicle_shown(&self) -> bool {
        self.visibility.vehicle && self.vehicle.is_some()
    }

    /// The classic layout's centre weapon panel: on foot, holding a weapon that counts
    /// ammunition or a saber with a style. It gives way to the weapon selection row,
    /// which retail draws in the same place (`y = 410`) and which names the weapon.
    fn weapon_panel(&self) -> bool {
        self.visibility.weapon
            && !self.visibility.menu_hud
            && !self.weapon_row
            && self.vehicle.is_none()
            && (self.has_ammo || self.saber_style.is_some())
    }

    /// The turbo's phase while the vehicle HUD shows and the vehicle has one.
    fn vehicle_turbo(&self) -> Option<vehicle::TurboPhase> {
        self.vehicle
            .filter(|_| self.visibility.vehicle)
            .and_then(|meters| meters.turbo)
            .map(|turbo| turbo.phase)
    }
}

impl HudDataSource for WidgetData<'_> {
    fn number(&self, binding: &str) -> Option<f32> {
        match binding {
            "health_ratio" => Some(self.ratios[0]),
            "armor_ratio" => Some(self.ratios[1]),
            "force_ratio" => Some(self.ratios[2]),
            "ammo_ratio" => Some(self.ammo_ratio),
            // A saber style has no amount: its meter is one full line.
            "style_ratio" => Some(if self.saber_style.is_some() { 1.0 } else { 0.0 }),
            "vehicle_ratio" => Some(self.vehicle.map_or(0.0, |meters| meters.hull)),
            "vehicle_speed_ratio" => Some(self.vehicle.map_or(0.0, |meters| meters.speed)),
            "vehicle_turbo_ratio" => Some(
                self.vehicle
                    .and_then(|meters| meters.turbo)
                    .map_or(0.0, |turbo| turbo.charge),
            ),
            "vehicle_ammo1_ratio" => Some(self.vehicle_ammo_slot(0).unwrap_or(0.0)),
            "vehicle_ammo2_ratio" => Some(self.vehicle_ammo_slot(1).unwrap_or(0.0)),
            "vehicle_ammo_ratio" => {
                Some(self.vehicle.and_then(|meters| meters.ammo).unwrap_or(0.0))
            }
            _ => None,
        }
    }

    fn text(&self, binding: &str) -> Option<&str> {
        match binding {
            "health_text" => Some(self.health),
            "armor_text" => Some(self.armor),
            "force_text" => Some(self.force),
            "weapon_text" => Some(self.weapon),
            "ammo_text" => Some(self.ammo),
            "health_value" => Some(self.health_value),
            "armor_value" => Some(self.armor_value),
            "force_value" => Some(self.force_value),
            "weapon_value" => Some(self.weapon_value),
            "ammo_value" => Some(self.ammo_value),
            _ => None,
        }
    }

    fn flag(&self, binding: &str) -> Option<bool> {
        match binding {
            // The ground HUD replaces the status block (and its scrim) and the stance.
            // So does the game-data menu HUD, which also shows the ammo.
            "draw_status" => Some(
                self.visibility.status && !self.visibility.ground_hud && !self.visibility.menu_hud,
            ),
            "draw_weapon" => Some(
                self.visibility.weapon
                    && !self.visibility.menu_hud
                    && !self.weapon_row
                    && self.weapon_alpha > 0.0,
            ),
            // The ammunition count and arc stay while the selection row shows, as the
            // retail HUD menus keep theirs (EternalJK's row never hides the ammo).
            "draw_ammo" => Some(
                self.visibility.weapon && !self.visibility.menu_hud && !self.ammo_value.is_empty(),
            ),
            // The classic layout's ammunition (or saber style) line, which has no
            // numeric value to test.
            "draw_ammo_text" => Some(self.visibility.weapon && !self.visibility.menu_hud),
            "draw_style" => Some(
                self.visibility.status
                    && !self.visibility.ground_hud
                    && !self.visibility.menu_hud
                    && self.saber_style.is_some(),
            ),
            "draw_crosshair" => Some(self.visibility.crosshair),
            "draw_vote" => {
                Some(self.visibility.elements && (self.vote_active || self.team_vote_active))
            }
            "draw_crosshair_name" => Some(self.visibility.crosshair_names && self.crosshair_name),
            "draw_timer" => Some(self.visibility.timer && self.timer),
            "draw_warmup" => Some(self.visibility.elements && self.warmup),
            "draw_lagometer" => Some(self.visibility.lagometer),
            "draw_interrupted" => Some(self.visibility.elements && self.interrupted),
            "draw_vehicle" => Some(self.visibility.vehicle && self.vehicle.is_some()),
            // The turbo's widgets, one per colour: a vehicle without a turbo shows none.
            "draw_vehicle_turbo" => Some(self.vehicle_turbo().is_some()),
            "draw_vehicle_turbo_charging" => {
                Some(self.vehicle_turbo() == Some(vehicle::TurboPhase::Charging))
            }
            "draw_vehicle_turbo_ready" => {
                Some(self.vehicle_turbo() == Some(vehicle::TurboPhase::Ready))
            }
            "draw_vehicle_turbo_burning" => {
                Some(self.vehicle_turbo() == Some(vehicle::TurboPhase::Burning))
            }
            "draw_vehicle_turbo_none" => {
                Some(self.vehicle_shown() && self.vehicle_turbo().is_none())
            }
            // Each weapon's row, or its greyed place without that weapon.
            "draw_vehicle_ammo1" => Some(self.vehicle_ammo_slot(0).is_some()),
            "draw_vehicle_ammo2" => Some(self.vehicle_ammo_slot(1).is_some()),
            "draw_vehicle_ammo1_none" => {
                Some(self.vehicle_shown() && self.vehicle_ammo_slot(0).is_none())
            }
            "draw_vehicle_ammo2_none" => {
                Some(self.vehicle_shown() && self.vehicle_ammo_slot(1).is_none())
            }
            "draw_weapon_panel" => Some(self.weapon_panel()),
            "draw_weapon_ammo" => Some(self.weapon_panel() && self.saber_style.is_none()),
            "draw_weapon_style" => Some(self.weapon_panel() && self.saber_style.is_some()),
            // A vehicle without a weapon has no ammunition to show.
            "draw_vehicle_ammo" => Some(
                self.visibility.vehicle && self.vehicle.is_some_and(|meters| meters.ammo.is_some()),
            ),
            _ => None,
        }
    }
}

/// Fixed words a layout's text widget names as `label_<name>`, from text id
/// [`LABEL_FIRST`] on.
pub(super) const LABELS: [(&str, &str); 10] = [
    ("label_health", "HEALTH"),
    ("label_armor", "ARMOR"),
    ("label_force", "FORCE"),
    ("label_hp", "HP"),
    ("label_speed", "SPD"),
    ("label_turbo", "TURBO"),
    ("label_ammo", "AMMO"),
    ("label_ammo1", "AMMO 1"),
    ("label_ammo2", "AMMO 2"),
    ("label_none", "--"),
];
pub(super) const LABEL_FIRST: u32 = 360;
pub(super) const LABEL_LAST: u32 = LABEL_FIRST + LABELS.len() as u32 - 1;

pub(super) fn text_id(binding: Option<&str>) -> Option<TextId> {
    match binding {
        Some("health_text") => Some(TextId(0)),
        Some("armor_text") => Some(TextId(1)),
        Some("force_text") => Some(TextId(2)),
        Some("weapon_text") => Some(TextId(3)),
        Some("ammo_text") => Some(TextId(4)),
        Some("health_value") => Some(TextId(6)),
        Some("armor_value") => Some(TextId(8)),
        Some("force_value") => Some(TextId(10)),
        Some("weapon_value") => Some(TextId(12)),
        Some("ammo_value") => Some(TextId(14)),
        Some("style_value") => Some(TextId(15)),
        Some("crosshair_name") => Some(TextId(310)),
        Some("match_timer") => Some(TextId(311)),
        Some("warmup_text") => Some(TextId(312)),
        Some("connection_interrupted") => Some(TextId(313)),
        Some("vehicle_value") => Some(TextId(vehicle::VALUE_TEXT)),
        Some("vehicle_text") => Some(TextId(vehicle::LABEL_TEXT)),
        Some("vehicle_speed_value") => Some(TextId(vehicle::SPEED_VALUE_TEXT)),
        Some("vehicle_speed_text") => Some(TextId(vehicle::SPEED_LABEL_TEXT)),
        Some("vehicle_ammo_value") => Some(TextId(vehicle::AMMO_VALUE_TEXT)),
        Some("vehicle_ammo_text") => Some(TextId(vehicle::AMMO_LABEL_TEXT)),
        Some("vehicle_turbo_value") => Some(TextId(vehicle::TURBO_VALUE_TEXT)),
        Some("vehicle_turbo_text") => Some(TextId(vehicle::TURBO_LABEL_TEXT)),
        Some("vehicle_ammo1_value") => Some(TextId(vehicle::AMMO1_VALUE_TEXT)),
        Some("vehicle_ammo2_value") => Some(TextId(vehicle::AMMO2_VALUE_TEXT)),
        // The weapon's name without the fade `weapon_value` takes after a switch.
        Some("weapon_name") => Some(TextId(12)),
        Some(label) if label.starts_with("label_") => LABELS
            .iter()
            .position(|(name, _)| *name == label)
            .map(|index| TextId(LABEL_FIRST + index as u32)),
        _ => None,
    }
}

pub(super) fn load_override(path: &Path) -> Option<HudLayoutDocument> {
    let source = std::fs::read_to_string(path).ok()?;
    let document = HudLayoutDocument::from_json(&source).ok()?;
    (document.version == 1 && document.widgets.len() <= WIDGET_LIMIT).then_some(document)
}

pub(super) fn client_name_and_team(
    game_state: &GameState,
    client_num: u16,
) -> (std::borrow::Cow<'_, str>, u8) {
    let (name, team) = game_state
        .config_string(1_131 + usize::from(client_num))
        .and_then(parse_client_info)
        .unwrap_or((std::borrow::Cow::Borrowed("unknown"), 3));
    // A blocked player's name reads BLOCKED here (`muted_players.rs`).
    if crate::muted_players::shown_muted(client_num) {
        return (
            std::borrow::Cow::Borrowed(crate::muted_players::LABEL),
            team,
        );
    }
    (name, team)
}

/// Split one `configstring` client-info entry into a display name and team.
///
/// JKA client info is a byte string, not UTF-8: names routinely carry high-bit bytes, both
/// from Latin-1 and from the decorated names players use. Validating the whole entry as UTF-8
/// and bailing turned every such player into "unknown" in the crosshair, while chat rendered
/// them correctly because `sjk-client/src/chat.rs` already converts lossily. Parsing the raw
/// bytes means a bad byte in any other field cannot cost us the name, and only a name that is
/// genuinely not UTF-8 allocates.
pub(super) fn parse_client_info(bytes: &[u8]) -> Option<(std::borrow::Cow<'_, str>, u8)> {
    let body = bytes.strip_prefix(b"\\").unwrap_or(bytes);
    let mut name: Option<&[u8]> = None;
    let mut team = 3;
    let mut fields = body.split(|byte| *byte == b'\\');
    while let (Some(key), Some(value)) = (fields.next(), fields.next()) {
        match key {
            b"n" | b"name" if name.is_none() && !value.is_empty() => name = Some(value),
            b"t" => {
                team = std::str::from_utf8(value)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3);
            }
            _ => {}
        }
    }
    // Latin-1, not UTF-8 with replacement marks: a name byte above 0x7F is the character of
    // that value in JKA's font, so lossy conversion rendered `Mara\xf1` as `Marai¿½`.
    Some((sjk_client::decode_legacy(name?), team))
}
