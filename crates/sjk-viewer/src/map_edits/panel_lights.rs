//! The panel's Lights page (`docs/map-edits.md`, "The panel"; the browser editor's
//! [Lights](../../../../docs/map-edits.md#lights-in-the-page) in game, Sol 09/11/2026):
//! the added lights and the map's lamp fixtures listed, markers for them over the scene
//! (a click on one chooses it), and the gizmo moving the chosen one.
//!
//! Every button runs the editor's own command, as the page's requests do:
//! `mapedit_light` (a light where the next click lands, or at the view's centre),
//! `mapedit_light_set` (its colour, reach and strength, and the gizmo's move),
//! `mapedit_delete`, and `mapedit_lamp` (off, on, reset, and the gizmo's move as an offset
//! from where the map has the lamp).

use super::document::{Document, LIGHT_INTENSITY, LIGHT_RADIUS};
use super::gizmo::{Placement, View};
use super::panel::{Action, Tab};
use crate::GpuState;
use glam::{Vec2, Vec3};

/// The colours a light is given from the page: name and colour.
pub(crate) const SWATCHES: [(&str, [f32; 3]); 8] = [
    ("Warm", super::lights::DEFAULT_COLOR),
    ("White", [1.0, 1.0, 1.0]),
    ("Red", [1.0, 0.25, 0.2]),
    ("Orange", [1.0, 0.55, 0.15]),
    ("Yellow", [1.0, 0.95, 0.4]),
    ("Green", [0.35, 1.0, 0.4]),
    ("Blue", [0.3, 0.55, 1.0]),
    ("Violet", [0.7, 0.35, 1.0]),
];
/// What Smaller and Bigger, Dimmer and Brighter change a light by.
pub(crate) const STEP: f32 = 1.25;
/// How many of the nearest lamps get a marker over the scene.
const LAMP_MARKERS: usize = 64;
/// How near (window pixels) a click must come to a marker to choose it.
const MARKER_REACH: f32 = 14.0;

/// An added light as the page lists it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LightRow {
    pub(crate) id: u32,
    pub(crate) origin: [f32; 3],
    pub(crate) color: [f32; 3],
    pub(crate) radius: f32,
    pub(crate) intensity: f32,
}

impl LightRow {
    /// Its line in the list.
    pub(crate) fn label(&self) -> String {
        format!(
            "{}: #{} r{:.0} x{:.2}",
            self.id,
            super::document::hex(self.color),
            self.radius,
            self.intensity
        )
    }

    /// `mapedit_light_set`'s words for this light at `origin` with `color`, `radius` and
    /// `intensity`.
    pub(crate) fn set_args(
        &self,
        origin: [f32; 3],
        color: [f32; 3],
        radius: f32,
        intensity: f32,
    ) -> Vec<String> {
        std::iter::once(self.id.to_string())
            .chain(
                origin
                    .iter()
                    .chain(color.iter())
                    .chain([radius, intensity].iter())
                    .map(|value| value.to_string()),
            )
            .collect()
    }
}

/// One of the map's lamp fixtures as the page lists it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LampRow {
    /// Its fixture's name (`surface:1043`).
    pub(crate) fixture: String,
    /// Its shader, model or effect.
    pub(crate) name: String,
    /// Where the world found it (before edits), and its colour.
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
    /// What the map's edits do to it here.
    pub(crate) off: bool,
    pub(crate) offset: [f32; 3],
    /// The world was built without this edit: it applies on the next load.
    pub(crate) pending: bool,
}

impl LampRow {
    /// Where its marker stands: where the edit puts it.
    pub(crate) fn at(&self) -> Vec3 {
        Vec3::from_array(self.position) + Vec3::from_array(self.offset)
    }

    /// Its line in the list.
    pub(crate) fn label(&self) -> String {
        let short = self.name.rsplit('/').next().unwrap_or(&self.name);
        let state = if self.off {
            " off"
        } else if self.offset.iter().any(|axis| *axis != 0.0) {
            " moved"
        } else {
            ""
        };
        let pending = if self.pending { " (on reload)" } else { "" };
        format!("{short} ({}){state}{pending}", self.fixture)
    }
}

/// Which list's row the page's buttons and the gizmo act on.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum LightChoice {
    #[default]
    Light,
    Lamp,
}

/// A light's reach or strength stepped by `factor`, kept within the document's limits.
pub(crate) fn stepped(value: f32, factor: f32, limits: &std::ops::RangeInclusive<f32>) -> f32 {
    (value * factor).clamp(*limits.start(), *limits.end())
}

/// `mapedit_lamp`'s words to stand `row` at `at`: moved by the offset from where the map
/// has it, or reset when that is nothing.
pub(crate) fn lamp_move_args(row: &LampRow, at: Vec3) -> Vec<String> {
    let offset = at - Vec3::from_array(row.position);
    if offset.length() < 0.05 {
        return vec![row.fixture.clone(), "reset".into()];
    }
    let mut args = vec![row.fixture.clone(), "move".into()];
    args.extend(offset.to_array().iter().map(|axis| format!("{axis:.2}")));
    args
}

/// The added lights of `document` here, as listed.
pub(crate) fn light_rows(document: Option<&Document>, server: Option<&str>) -> Vec<LightRow> {
    document.map_or_else(Vec::new, |document| {
        super::lights::lights(document, server)
            .into_iter()
            .map(|light| LightRow {
                id: light.id,
                origin: light.origin,
                color: light.color,
                radius: light.radius,
                intensity: light.intensity,
            })
            .collect()
    })
}

impl GpuState {
    /// The map's lamp fixtures for the page, nearest `view` first, with what the edits
    /// here do to them and whether the world was built with that.
    pub(crate) fn map_edit_lamp_rows(&self, view: Option<Vec3>) -> Vec<LampRow> {
        let server = self.map_edits_server();
        let edits = self
            .map_edits
            .document
            .as_ref()
            .map(|document| super::lights::lamp_edits(document, server.as_deref()))
            .unwrap_or_default();
        let built = &self.world_materials.lamp_edits;
        let mut rows: Vec<LampRow> = self
            .world_materials
            .lamp_fixtures
            .iter()
            .map(|summary| {
                let fixture = summary.fixture;
                let off = edits.off.contains(&fixture);
                let offset = edits.moved.get(&fixture).copied().unwrap_or(Vec3::ZERO);
                let built_off = built.off.contains(&fixture);
                let built_offset = built.moved.get(&fixture).copied().unwrap_or(Vec3::ZERO);
                LampRow {
                    fixture: fixture.to_string(),
                    name: super::lights::fixture_name(&self.bsp, fixture),
                    position: summary.position.to_array(),
                    color: summary.color,
                    off,
                    offset: offset.to_array(),
                    pending: off != built_off || offset.distance(built_offset) > 0.01,
                }
            })
            .collect();
        if let Some(view) = view {
            rows.sort_by(|a, b| {
                Vec3::from_array(a.position)
                    .distance_squared(view)
                    .total_cmp(&Vec3::from_array(b.position).distance_squared(view))
            });
        }
        rows
    }

    /// The light or lamp the gizmo stands on while the Lights page shows: the chosen one,
    /// as a placement (a lamp's id is its row).
    pub(crate) fn map_edit_light_target(&self) -> Option<(LightChoice, Placement)> {
        let panel = &self.map_edits.panel;
        if panel.tab != Tab::Lights {
            return None;
        }
        let at = |id: u32, origin: Vec3| Placement {
            id,
            origin,
            angles: [0.0; 3],
            scale: [1.0; 3],
        };
        match panel.light_choice {
            LightChoice::Light => {
                let row = panel.facts.lights.get(panel.light_row)?;
                // As drawn now: a drag's preview moves the drawn light.
                let origin = self
                    .map_edits
                    .lights
                    .iter()
                    .find(|light| light.id == row.id)
                    .map_or(row.origin, |light| light.origin);
                Some((LightChoice::Light, at(row.id, Vec3::from_array(origin))))
            }
            LightChoice::Lamp => {
                let row = panel.facts.lamps.get(panel.lamp_row)?;
                Some((LightChoice::Lamp, at(panel.lamp_row as u32, row.at())))
            }
        }
    }

    /// Show the gizmo's light or lamp at `placement` without saving.
    pub(crate) fn map_edit_light_preview(&mut self, choice: LightChoice, placement: &Placement) {
        match choice {
            LightChoice::Light => {
                if let Some(light) = self
                    .map_edits
                    .lights
                    .iter_mut()
                    .find(|light| light.id == placement.id)
                {
                    light.origin = placement.origin.to_array();
                }
            }
            LightChoice::Lamp => {
                if let Some(row) = self
                    .map_edits
                    .panel
                    .facts
                    .lamps
                    .get_mut(placement.id as usize)
                {
                    row.offset = (placement.origin - Vec3::from_array(row.position)).to_array();
                }
            }
        }
    }

    /// The command that puts the gizmo's light or lamp down at `placement`.
    pub(crate) fn map_edit_light_commit(
        &self,
        choice: LightChoice,
        placement: &Placement,
    ) -> Option<(&'static str, Vec<String>)> {
        let facts = &self.map_edits.panel.facts;
        match choice {
            LightChoice::Light => {
                let row = facts.lights.iter().find(|row| row.id == placement.id)?;
                Some((
                    "mapedit_light_set",
                    row.set_args(
                        placement.origin.to_array(),
                        row.color,
                        row.radius,
                        row.intensity,
                    ),
                ))
            }
            LightChoice::Lamp => {
                let row = facts.lamps.get(placement.id as usize)?;
                Some(("mapedit_lamp", lamp_move_args(row, placement.origin)))
            }
        }
    }

    /// A light-page button: the command it runs, or `None` when it did its work itself
    /// (or refused, saying why on the answer line).
    pub(crate) fn map_edit_light_action(
        &mut self,
        action: Action,
    ) -> Option<(&'static str, Vec<String>)> {
        let panel = &mut self.map_edits.panel;
        let light = panel.facts.lights.get(panel.light_row).cloned();
        let lamp = panel.facts.lamps.get(panel.lamp_row).cloned();
        let no_light = || Err("Choose a light in the list (or click its bulb) first".to_owned());
        let no_lamp = || Err("Choose a lamp in the list (or click its cube) first".to_owned());
        match action {
            Action::LightAdd => {
                panel.adding_light = !panel.adding_light;
                panel.answer(Ok(vec![if panel.adding_light {
                    "Click where the light goes: 12 units in front of what it meets (Esc stops)"
                        .to_owned()
                } else {
                    "Stopped adding a light".to_owned()
                }]));
                None
            }
            Action::LightAtView => {
                panel.adding_light = false;
                panel.choose_newest_light = true;
                // What the view's centre meets, as the page's Add at view centre.
                let _ = self.map_edit_pick();
                Some(("mapedit_light", Vec::new()))
            }
            Action::LightDelete => match (panel.light_choice, light, lamp) {
                (LightChoice::Light, Some(light), _) => {
                    Some(("mapedit_delete", vec![light.id.to_string()]))
                }
                (LightChoice::Lamp, _, Some(lamp)) => {
                    Some(("mapedit_lamp", vec![lamp.fixture, "reset".into()]))
                }
                (LightChoice::Light, None, _) => {
                    panel.answer(no_light());
                    None
                }
                (LightChoice::Lamp, _, None) => {
                    panel.answer(no_lamp());
                    None
                }
            },
            Action::LightRadius(_) | Action::LightIntensity(_) | Action::LightColour(_) => {
                let Some(light) = light else {
                    panel.answer(no_light());
                    return None;
                };
                let (mut color, mut radius, mut intensity) =
                    (light.color, light.radius, light.intensity);
                match action {
                    Action::LightRadius(factor) => radius = stepped(radius, factor, &LIGHT_RADIUS),
                    Action::LightIntensity(factor) => {
                        intensity = stepped(intensity, factor, &LIGHT_INTENSITY);
                    }
                    Action::LightColour(swatch) => {
                        color = SWATCHES.get(swatch).map_or(color, |(_, color)| *color);
                    }
                    _ => {}
                }
                Some((
                    "mapedit_light_set",
                    light.set_args(light.origin, color, radius, intensity),
                ))
            }
            Action::LampToggle | Action::LampReset => {
                let Some(lamp) = lamp else {
                    panel.answer(no_lamp());
                    return None;
                };
                let word = match action {
                    Action::LampReset => "reset",
                    _ if lamp.off => "on",
                    _ => "off",
                };
                Some(("mapedit_lamp", vec![lamp.fixture, word.into()]))
            }
            _ => None,
        }
    }

    /// A click on the scene while adding a light: one where it lands.
    pub(crate) fn map_edit_light_add_at(&mut self, point: [f32; 2]) {
        let panel = &mut self.map_edits.panel;
        panel.adding_light = false;
        panel.choose_newest_light = true;
        let picked = self.map_edit_pick_at(point);
        let result = if picked {
            self.map_edit_command("mapedit_light", &[])
        } else {
            Err("Nothing there: click a wall or the floor".to_owned())
        };
        let panel = &mut self.map_edits.panel;
        panel.answer(result);
        panel.stale();
    }

    /// The marker under `at` (window pixels) on the Lights page: an added light before a
    /// lamp at the same spot.
    pub(crate) fn map_edit_marker_at(&self, at: Vec2) -> Option<(LightChoice, usize)> {
        let view = self.map_edit_view()?;
        let facts = &self.map_edits.panel.facts;
        let near = |point: Vec3| {
            view.project(point)
                .map(|screen| screen.distance(at))
                .filter(|distance| *distance <= MARKER_REACH)
        };
        let light = facts
            .lights
            .iter()
            .enumerate()
            .filter_map(|(index, row)| Some((near(Vec3::from_array(row.origin))?, index)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, index)) = light {
            return Some((LightChoice::Light, index));
        }
        facts
            .lamps
            .iter()
            .take(LAMP_MARKERS)
            .enumerate()
            .filter_map(|(index, row)| Some((near(row.at())?, index)))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, index)| (LightChoice::Lamp, index))
    }

    /// Choose row `index` of the lights or the lamps.
    pub(crate) fn map_edit_choose_light(&mut self, choice: LightChoice, index: usize) {
        let panel = &mut self.map_edits.panel;
        panel.light_choice = choice;
        let line = match choice {
            LightChoice::Light => {
                panel.light_row = index;
                panel
                    .facts
                    .lights
                    .get(index)
                    .map(|row| format!("Light {}: G or its arrows move it", row.id))
            }
            LightChoice::Lamp => {
                panel.lamp_row = index;
                panel
                    .facts
                    .lamps
                    .get(index)
                    .map(|row| format!("Lamp {}: G or its arrows move it", row.fixture))
            }
        };
        panel.settle_light_lists();
        if let Some(line) = line {
            panel.answer(Ok(vec![line]));
        }
    }

    /// Draw the Lights page's markers into `list`: each added light a bulb in its colour,
    /// the nearest lamps small squares in theirs (grey when off), the chosen one ringed
    /// in gold.
    pub(crate) fn map_edit_markers_draw(&self, list: &mut sjk_ui::DrawList) {
        let panel = &self.map_edits.panel;
        if panel.tab != Tab::Lights {
            return;
        }
        let Some(view) = self.map_edit_view() else {
            return;
        };
        let colour = |rgb: [f32; 3], alpha: f32| sjk_ui::Color::new(rgb[0], rgb[1], rgb[2], alpha);
        let gold = sjk_ui::Color::new(1.0, 0.82, 0.3, 1.0);
        let dark = sjk_ui::Color::new(0.02, 0.03, 0.06, 0.85);
        let chosen_lamp = (panel.light_choice == LightChoice::Lamp).then_some(panel.lamp_row);
        for (index, row) in panel.facts.lamps.iter().take(LAMP_MARKERS).enumerate() {
            let Some(at) = view.project(row.at()) else {
                continue;
            };
            let fill = if row.off {
                sjk_ui::Color::new(0.45, 0.45, 0.45, 0.9)
            } else {
                colour(row.color.map(|c| c.clamp(0.15, 1.0)), 0.95)
            };
            let side = 9.0;
            let _ = list.push(sjk_ui::DrawCommand::RoundedRect {
                rect: sjk_ui::Rect::new(
                    at.x - side * 0.5 - 1.5,
                    at.y - side * 0.5 - 1.5,
                    side + 3.0,
                    side + 3.0,
                ),
                radius: 2.0,
                color: dark,
            });
            let _ = list.push(sjk_ui::DrawCommand::RoundedRect {
                rect: sjk_ui::Rect::new(at.x - side * 0.5, at.y - side * 0.5, side, side),
                radius: 1.5,
                color: fill,
            });
            if chosen_lamp == Some(index) {
                ring(list, at, 10.0, gold);
            }
        }
        let chosen_light = (panel.light_choice == LightChoice::Light).then_some(panel.light_row);
        for (index, row) in panel.facts.lights.iter().enumerate() {
            let origin = self
                .map_edits
                .lights
                .iter()
                .find(|light| light.id == row.id)
                .map_or(row.origin, |light| light.origin);
            let Some(at) = view.project(Vec3::from_array(origin)) else {
                continue;
            };
            dot(list, at, 8.5, dark);
            dot(list, at, 6.5, colour(row.color, 1.0));
            if chosen_light == Some(index) {
                ring(list, at, 12.0, gold);
                // Its reach, as a ring the size its radius makes at its distance.
                if let Some(edge) =
                    view.project(Vec3::from_array(origin) + view_right(&view) * row.radius)
                {
                    ring(
                        list,
                        at,
                        edge.distance(at).min(4000.0),
                        sjk_ui::Color::new(1.0, 0.82, 0.3, 0.45),
                    );
                }
            }
        }
    }

    /// The view the markers and the gizmo are drawn in.
    pub(crate) fn map_edit_view(&self) -> Option<View> {
        let (view_projection, eye, forward, viewport) = self.world_notes.camera_for_gizmo()?;
        (viewport[0] > 0.0 && viewport[1] > 0.0).then_some(View {
            eye,
            forward,
            view_projection,
            viewport,
        })
    }
}

/// The view's right on the floor, to measure a reach on screen.
fn view_right(view: &View) -> Vec3 {
    view.forward
        .cross(Vec3::Z)
        .try_normalize()
        .unwrap_or(Vec3::X)
}

fn dot(list: &mut sjk_ui::DrawList, at: Vec2, radius: f32, color: sjk_ui::Color) {
    let _ = list.push(sjk_ui::DrawCommand::RoundedRect {
        rect: sjk_ui::Rect::new(at.x - radius, at.y - radius, radius * 2.0, radius * 2.0),
        radius,
        color,
    });
}

fn ring(list: &mut sjk_ui::DrawList, at: Vec2, radius: f32, color: sjk_ui::Color) {
    let _ = list.push(sjk_ui::DrawCommand::Border {
        rect: sjk_ui::Rect::new(at.x - radius, at.y - radius, radius * 2.0, radius * 2.0),
        radius,
        width: 2.0,
        color,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_edits::document::Kind;

    fn lamp(fixture: &str, position: [f32; 3]) -> LampRow {
        LampRow {
            fixture: fixture.to_owned(),
            name: "textures/kejim/light".to_owned(),
            position,
            color: [1.0; 3],
            off: false,
            offset: [0.0; 3],
            pending: false,
        }
    }

    #[test]
    fn a_light_is_set_with_its_own_words_and_steps_within_the_limits() {
        let row = LightRow {
            id: 7,
            origin: [1.0, 2.0, 3.0],
            color: [1.0, 0.5, 0.25],
            radius: 320.0,
            intensity: 1.0,
        };
        assert_eq!(
            row.set_args([4.0, 5.0, 6.0], row.color, 400.0, 2.0),
            ["7", "4", "5", "6", "1", "0.5", "0.25", "400", "2"]
        );
        assert_eq!(row.label(), "7: #ff8040 r320 x1.00");
        assert_eq!(stepped(320.0, STEP, &LIGHT_RADIUS), 400.0);
        assert_eq!(stepped(2000.0, STEP, &LIGHT_RADIUS), *LIGHT_RADIUS.end());
        assert_eq!(
            stepped(0.05, 1.0 / STEP, &LIGHT_INTENSITY),
            *LIGHT_INTENSITY.start()
        );
        for (_, color) in SWATCHES {
            assert!(super::super::document::check_light([0.0; 3], color, 320.0, 1.0).is_ok());
        }
    }

    #[test]
    fn a_lamp_moves_by_its_offset_from_the_map_and_resets_back_there() {
        let row = lamp("surface:12", [100.0, 0.0, 50.0]);
        assert_eq!(
            lamp_move_args(&row, Vec3::new(132.0, 0.0, 50.0)),
            ["surface:12", "move", "32.00", "0.00", "0.00"]
        );
        assert_eq!(
            lamp_move_args(&row, Vec3::new(100.0, 0.0, 50.01)),
            ["surface:12", "reset"]
        );
        let mut moved = row.clone();
        moved.offset = [0.0, 0.0, 16.0];
        assert_eq!(moved.at(), Vec3::new(100.0, 0.0, 66.0));
        moved.pending = true;
        assert_eq!(moved.label(), "light (surface:12) moved (on reload)");
        moved.off = true;
        assert_eq!(moved.label(), "light (surface:12) off (on reload)");
    }

    #[test]
    fn the_page_lists_the_lights_of_this_server() {
        let mut document = Document::new("mp/duel6", 1);
        let light = |origin| Kind::Light {
            origin,
            color: [1.0; 3],
            radius: 300.0,
            intensity: 1.0,
        };
        document.add(light([1.0, 0.0, 0.0]), None);
        document.add(light([2.0, 0.0, 0.0]), Some("192.0.2.1:29070".into()));
        assert_eq!(light_rows(Some(&document), None).len(), 1);
        assert_eq!(
            light_rows(Some(&document), Some("192.0.2.1:29070")).len(),
            2
        );
        assert!(light_rows(None, None).is_empty());
    }
}
