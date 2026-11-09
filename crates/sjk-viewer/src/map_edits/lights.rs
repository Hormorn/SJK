//! Lights in map edits (Sol, 08/11/2026: "for mapedit_web I want to see lights so I can
//! move them, delete them or add new ones"):
//!
//! - **Added lights** ([`Kind::Light`]): point lights staff place, drawn every frame as
//!   dynamic lights, the frame's point-light list `r_dynamiclight` lights go through, so
//!   every SJK player sees them on every preset. At most [`BUDGET`] of them a frame, the
//!   nearest to the view: their slots are kept apart from the effects' (reserved at the
//!   frame's start, [`crate::dynamic_lights::PointLightList::reserve`]), so neither evicts
//!   the other.
//! - **The map's own lamps** ([`Kind::Lamp`]): the real-time lighting's lamps (only the
//!   Experimental preset draws them) are found when the world is built, grouped by the
//!   fixture they come from ([`crate::lamp_lights::Fixture`]). A lamp edit turns a
//!   fixture off or moves it; the world's build reads the map's document itself
//!   ([`lamp_edits_at_build`]), so a lamp edit applies from the next build of the map's
//!   world: its next load, or a graphics reload (`vid_restart force`).
//!
//! The baked lightmaps are never changed.

use super::document::{Document, Kind};
use crate::GpuState;
use crate::dynamic_lights::{PointLight, PointLightList};
use crate::lamp_lights::{Fixture, LampEdits};
use glam::Vec3;
use serde_json::{Value, json};
use std::sync::Mutex;

/// Most added lights drawn in one frame (of the 32 dynamic lights).
pub(crate) const BUDGET: usize = 8;
/// A new light's colour, reach and strength when none is given: a warm lamp.
pub(crate) const DEFAULT_COLOR: [f32; 3] = [1.0, 0.85, 0.65];
pub(crate) const DEFAULT_RADIUS: f32 = 320.0;
/// How far in front of the surface aimed at a light is placed.
const OFF_SURFACE: f32 = 12.0;

/// An added light as drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EditLight {
    pub(crate) id: u32,
    pub(crate) origin: [f32; 3],
    pub(crate) color: [f32; 3],
    pub(crate) radius: f32,
    pub(crate) intensity: f32,
}

impl EditLight {
    fn point(&self) -> PointLight {
        PointLight {
            origin: self.origin,
            radius: self.radius,
            color: self.color.map(|channel| channel * self.intensity),
        }
    }
}

/// How many slots this frame keeps for the added lights.
pub(crate) fn reserve(state: &super::State) -> usize {
    state.lights.len().min(BUDGET)
}

/// Submit the added lights nearest `view` (by how far their reach is from it) into the
/// slots [`reserve`] kept.
pub(crate) fn append(state: &super::State, view: Vec3, out: &mut PointLightList) {
    for index in nearest(&state.lights, view, BUDGET) {
        out.push_reserved(state.lights[index].point());
    }
}

/// The indices of the `count` lights whose reach comes nearest `view`, nearest first.
pub(crate) fn nearest(lights: &[EditLight], view: Vec3, count: usize) -> Vec<usize> {
    let gap = |light: &EditLight| Vec3::from_array(light.origin).distance(view) - light.radius;
    let mut order: Vec<usize> = (0..lights.len()).collect();
    order.sort_by(|&a, &b| gap(&lights[a]).total_cmp(&gap(&lights[b])));
    order.truncate(count);
    order
}

/// The added lights of `document` that apply on `server`.
pub(crate) fn lights(document: &Document, server: Option<&str>) -> Vec<EditLight> {
    document
        .active(server)
        .filter_map(|edit| match edit.kind {
            Kind::Light {
                origin,
                color,
                radius,
                intensity,
            } => Some(EditLight {
                id: edit.id,
                origin,
                color,
                radius,
                intensity,
            }),
            _ => None,
        })
        .collect()
}

/// What `document`'s lamp edits do on `server`: a server's own edit of a fixture wins
/// over the every-server one.
pub(crate) fn lamp_edits(document: &Document, server: Option<&str>) -> LampEdits {
    let mut active: Vec<_> = document.active(server).collect();
    active.sort_by_key(|edit| edit.server.is_some());
    let mut edits = LampEdits::default();
    for edit in active {
        let Kind::Lamp {
            fixture,
            off,
            offset,
            ..
        } = &edit.kind
        else {
            continue;
        };
        let Some(fixture) = Fixture::parse(fixture) else {
            continue;
        };
        edits.off.remove(&fixture);
        edits.moved.remove(&fixture);
        if *off {
            edits.off.insert(fixture);
        } else if offset.iter().any(|axis| *axis != 0.0) {
            edits.moved.insert(fixture, Vec3::from_array(*offset));
        }
    }
    edits
}

/// BSPs loaded lately: their fingerprint ([`fingerprint`]), map name and checksum, so the
/// world's build knows which document its map has without hashing the file again.
static LOADED: Mutex<Vec<(u64, String, i32)>> = Mutex::new(Vec::new());

/// What tells one parsed BSP from another without its bytes: its entity lump and its
/// counts of surfaces and shaders.
fn fingerprint(bsp: &sjk_bsp::Bsp) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    bsp.entities().hash(&mut hasher);
    bsp.render().surfaces().len().hash(&mut hasher);
    bsp.shaders().len().hash(&mut hasher);
    hasher.finish()
}

/// A map file `path` (`maps/mp/ffa3.bsp`) was read with `checksum` and parsed as `bsp`.
pub(crate) fn note_loaded(path: &str, checksum: i32, bsp: &sjk_bsp::Bsp) {
    let map = crate::remap_blocked_maps::map_name(path).to_ascii_lowercase();
    let print = fingerprint(bsp);
    let Ok(mut loaded) = LOADED.lock() else {
        return;
    };
    loaded.retain(|(known, ..)| *known != print);
    loaded.push((print, map, checksum));
    if loaded.len() > 4 {
        loaded.remove(0);
    }
}

/// The map name and checksum `bsp` was loaded as, when [`note_loaded`] saw it.
fn loaded(bsp: &sjk_bsp::Bsp) -> Option<(String, i32)> {
    let print = fingerprint(bsp);
    let loaded = LOADED.lock().ok()?;
    loaded
        .iter()
        .rev()
        .find(|(known, ..)| *known == print)
        .map(|(_, map, checksum)| (map.clone(), *checksum))
}

/// The lamp edits a world being built for `bsp` applies: those of the map's document in
/// the SJK config folder of `console` for `server` (`None` off a server), none with SJK
/// map edits off or without a document.
pub(crate) fn lamp_edits_at_build(
    bsp: &sjk_bsp::Bsp,
    console: Option<&crate::console::ViewerConsole>,
    server: Option<&str>,
) -> LampEdits {
    let Some(console) = console.filter(|console| console.map_edits_on()) else {
        return LampEdits::default();
    };
    let Some((map, checksum)) = loaded(bsp) else {
        return LampEdits::default();
    };
    let directory = super::folder(console.config_directory());
    if !super::has_documents(&directory, &map) {
        return LampEdits::default();
    }
    super::load(&directory, &map, checksum)
        .map(|document| lamp_edits(&document, server))
        .unwrap_or_default()
}

/// What fixture `fixture` of `bsp` is: its surface's shader, its prop's model or its
/// effect's file; empty when the map has no such thing.
pub(crate) fn fixture_name(bsp: &sjk_bsp::Bsp, fixture: Fixture) -> String {
    match fixture {
        Fixture::Surface(surface) => bsp
            .render()
            .surfaces()
            .get(surface as usize)
            .and_then(|surface| bsp.shaders().get(surface.shader))
            .map(|shader| shader.name_lossy().into_owned())
            .unwrap_or_default(),
        Fixture::Prop { entity, .. } => crate::static_models::spawns(bsp)
            .into_iter()
            .find(|(number, _)| *number == entity)
            .map(|(_, spawn)| spawn.model)
            .unwrap_or_default(),
        Fixture::Effect(entity) => sjk_entity::parse_entity_lump(bsp.entities())
            .ok()
            .and_then(|entities| {
                entities
                    .get(entity as usize)
                    .and_then(|entity| entity.get("fxFile").map(str::to_owned))
            })
            .unwrap_or_default(),
        Fixture::None | Fixture::Index(_) => String::new(),
    }
}

/// The kind of a fixture as the page names it.
fn fixture_kind(fixture: Fixture) -> &'static str {
    match fixture {
        Fixture::Surface(_) => "surface",
        Fixture::Prop { .. } => "prop",
        Fixture::Effect(_) => "effect",
        Fixture::None | Fixture::Index(_) => "other",
    }
}

fn number(text: &str) -> Result<f32, String> {
    text.parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("not a number: {text}"))
}

fn numbers<const N: usize>(args: &[String]) -> Result<[f32; N], String> {
    let mut out = [0.0; N];
    for (slot, arg) in out.iter_mut().zip(args) {
        *slot = number(arg)?;
    }
    Ok(out)
}

impl GpuState {
    /// `mapedit_light [<x y z>] [<r g b> <radius> <intensity>]`: add a light at the point
    /// given, or in front of what is aimed at (the selection, else the crosshair's).
    pub(super) fn map_edit_light(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let usage = "usage: mapedit_light [<x y z>] [<r g b> <radius> <intensity>]";
        let (origin, look) = match args.len() {
            0 | 5 => (None, args),
            3 | 8 => (Some(numbers::<3>(&args[..3])?), &args[3..]),
            _ => return Err(usage.into()),
        };
        let (color, radius, intensity) = if look.is_empty() {
            (DEFAULT_COLOR, DEFAULT_RADIUS, 1.0)
        } else {
            let [r, g, b, radius, intensity] = numbers::<5>(look)?;
            ([r, g, b], radius, intensity)
        };
        let origin = match origin {
            Some(origin) => origin,
            None => {
                let aim = self.map_edit_aim()?;
                (aim.hit - aim.forward.normalize_or_zero() * OFF_SURFACE).to_array()
            }
        };
        super::document::check_light(origin, color, radius, intensity)?;
        let server = self.map_edit_server()?;
        let id = self.map_edit_document()?.add(
            Kind::Light {
                origin,
                color,
                radius,
                intensity,
            },
            server,
        );
        self.map_edit_saved(format!(
            "{id}: light at {:.0} {:.0} {:.0}, radius {radius:.0}",
            origin[0], origin[1], origin[2]
        ))
    }

    /// `mapedit_light_set <id> <x y z> <r g b> <radius> <intensity>`: set added light
    /// `id` exactly (the page's gizmo and its light's fields).
    pub(super) fn map_edit_light_set(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let [id, rest @ ..] = args else {
            return Err(
                "usage: mapedit_light_set <id> <x y z> <r g b> <radius> <intensity>".into(),
            );
        };
        if rest.len() != 8 {
            return Err(
                "usage: mapedit_light_set <id> <x y z> <r g b> <radius> <intensity>".into(),
            );
        }
        let id: u32 = id.parse().map_err(|_| format!("not an id: {id}"))?;
        let [x, y, z, r, g, b, radius, intensity] = numbers::<8>(rest)?;
        let (origin, color) = ([x, y, z], [r, g, b]);
        super::document::check_light(origin, color, radius, intensity)?;
        let edit = self
            .map_edit_document()?
            .get_mut(id)
            .filter(|edit| matches!(edit.kind, Kind::Light { .. }))
            .ok_or_else(|| format!("No light {id}"))?;
        edit.kind = Kind::Light {
            origin,
            color,
            radius,
            intensity,
        };
        if let Some(document) = self.map_edits.document.as_mut() {
            document.unpublished = true;
        }
        self.map_edit_saved(format!(
            "{id}: light at {x:.0} {y:.0} {z:.0}, #{} radius {radius:.0} x{intensity:.2}",
            super::document::hex(color)
        ))
    }

    /// `mapedit_lamp <fixture> off|on|move <dx dy dz>|reset`: turn one of the map's
    /// lamp fixtures off or on, move it by an offset from where the map has it, or take
    /// its edit away. Applies from the next build of the map's world.
    pub(super) fn map_edit_lamp(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let usage = "usage: mapedit_lamp <fixture> off|on|move <dx dy dz>|reset";
        let [fixture, action, rest @ ..] = args else {
            return Err(usage.into());
        };
        let parsed = Fixture::parse(fixture).ok_or_else(|| {
            format!("{fixture}: not a lamp fixture (surface:<n>, prop:<entity>:<n>, effect:<n>)")
        })?;
        let server = self.map_edit_server()?;
        let name = fixture_name(&self.bsp, parsed);
        let known = self
            .world_materials
            .lamp_fixtures
            .iter()
            .any(|summary| summary.fixture == parsed);
        let document = self.map_edit_document()?;
        let existing = document
            .edits
            .iter()
            .find(|edit| {
                edit.server == server
                    && matches!(&edit.kind, Kind::Lamp { fixture: f, .. } if f == fixture)
            })
            .map(|edit| (edit.id, edit.kind.clone()));
        let (mut off, mut offset) = match &existing {
            Some((_, Kind::Lamp { off, offset, .. })) => (*off, *offset),
            _ => (false, [0.0; 3]),
        };
        match (action.as_str(), rest) {
            ("off", []) => off = true,
            ("on", []) => off = false,
            ("move", [_, _, _]) => offset = numbers::<3>(rest)?,
            ("reset", []) => {
                off = false;
                offset = [0.0; 3];
            }
            _ => return Err(usage.into()),
        }
        if !known && existing.is_none() {
            return Err(format!(
                "{fixture}: no lamp of this map's world comes from it (lamps are found on the Experimental preset)"
            ));
        }
        let pending =
            " (applies when the map's world is built again: its next load, or vid_restart force)";
        if !off && offset.iter().all(|axis| *axis == 0.0) {
            let Some((id, _)) = existing else {
                return Ok(vec![format!("{fixture}: lamp as the map has it already")]);
            };
            document.remove(id);
            return self.map_edit_saved(format!("{fixture}: lamp as the map has it{pending}"));
        }
        if offset
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > super::document::LAMP_OFFSET)
        {
            return Err(format!(
                "a lamp moves at most {} units along each axis",
                super::document::LAMP_OFFSET
            ));
        }
        let id = document.add(
            Kind::Lamp {
                fixture: fixture.clone(),
                name,
                off,
                offset,
            },
            server,
        );
        let what = if off {
            "off".to_owned()
        } else {
            format!("moved {:.0} {:.0} {:.0}", offset[0], offset[1], offset[2])
        };
        self.map_edit_saved(format!("{id}: lamp {fixture} {what}{pending}"))
    }

    /// `api/lamps`: the map's lamp fixtures as this world found them (before edits), and
    /// the lamp edits it was built with.
    pub(crate) fn map_edit_web_lamps(&self) -> Value {
        let runtime = &self.world_materials;
        let fixtures: Vec<Value> = runtime
            .lamp_fixtures
            .iter()
            .map(|summary| {
                json!({
                    "id": summary.fixture.to_string(),
                    "kind": fixture_kind(summary.fixture),
                    "name": fixture_name(&self.bsp, summary.fixture),
                    "position": summary.position.to_array(),
                    "color": summary.color,
                    "power": summary.power,
                    "radius": summary.radius,
                    "lamps": summary.lamps,
                })
            })
            .collect();
        let applied = &runtime.lamp_edits;
        json!({
            "map": crate::remap_blocked_maps::map_name(&self.world_load_map),
            "realtime": self.context.sun_shadows.day.enabled,
            "fixtures": fixtures,
            "applied": {
                "off": applied.off.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "moved": applied
                    .moved
                    .iter()
                    .map(|(fixture, offset)| json!([fixture.to_string(), offset.to_array()]))
                    .collect::<Vec<_>>(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(id: u32, origin: [f32; 3], radius: f32) -> EditLight {
        EditLight {
            id,
            origin,
            color: [1.0; 3],
            radius,
            intensity: 1.0,
        }
    }

    /// The lights drawn are those whose reach comes nearest the view, at most the budget.
    #[test]
    fn the_nearest_lights_are_drawn_within_the_budget() {
        let lights: Vec<EditLight> = (0..12)
            .map(|k| light(k, [k as f32 * 1000.0, 0.0, 0.0], 100.0))
            .collect();
        let drawn = nearest(&lights, Vec3::new(5400.0, 0.0, 0.0), BUDGET);
        assert_eq!(drawn.len(), BUDGET);
        assert_eq!(drawn[0], 5);
        assert!(
            drawn.contains(&2) && drawn.contains(&9) && !drawn.contains(&1) && !drawn.contains(&10)
        );
        // A far light with a long reach comes before a near one with a short one.
        let lights = [
            light(1, [300.0, 0.0, 0.0], 50.0),
            light(2, [900.0, 0.0, 0.0], 800.0),
        ];
        assert_eq!(nearest(&lights, Vec3::ZERO, 1), [1]);
        let mut list = PointLightList::default();
        let state = super::super::State {
            lights: lights.to_vec(),
            ..Default::default()
        };
        list.reserve(reserve(&state));
        append(&state, Vec3::ZERO, &mut list);
        assert_eq!(list.as_slice().len(), 2);
        assert_eq!(list.as_slice()[0].radius, 800.0);
    }

    /// An added light is coloured by its colour times its intensity.
    #[test]
    fn a_light_is_its_colour_times_its_intensity() {
        let light = EditLight {
            id: 1,
            origin: [1.0, 2.0, 3.0],
            color: [1.0, 0.5, 0.25],
            radius: 200.0,
            intensity: 2.0,
        };
        assert_eq!(
            light.point(),
            PointLight {
                origin: [1.0, 2.0, 3.0],
                radius: 200.0,
                color: [2.0, 1.0, 0.5]
            }
        );
    }

    /// Lights and lamp edits apply by scope; a server's own lamp edit wins.
    #[test]
    fn lights_and_lamp_edits_follow_their_scope() {
        let mut document = Document::new("mp/ffa5", 1);
        document.add(
            Kind::Light {
                origin: [0.0; 3],
                color: [1.0; 3],
                radius: 200.0,
                intensity: 1.0,
            },
            None,
        );
        document.add(
            Kind::Light {
                origin: [64.0; 3],
                color: [1.0; 3],
                radius: 200.0,
                intensity: 1.0,
            },
            Some("1.2.3.4:29070".into()),
        );
        let lamp = |fixture: &str, off: bool, offset: [f32; 3]| Kind::Lamp {
            fixture: fixture.into(),
            name: String::new(),
            off,
            offset,
        };
        document.add(lamp("surface:5", true, [0.0; 3]), None);
        document.add(
            lamp("surface:5", false, [0.0, 0.0, 32.0]),
            Some("1.2.3.4:29070".into()),
        );
        document.add(lamp("effect:9", false, [0.0; 3]), None);
        assert_eq!(lights(&document, None).len(), 1);
        assert_eq!(lights(&document, Some("1.2.3.4:29070")).len(), 2);
        let everywhere = lamp_edits(&document, None);
        assert!(everywhere.off.contains(&Fixture::Surface(5)));
        assert!(
            everywhere.moved.is_empty(),
            "an edit with no change moves nothing"
        );
        let here = lamp_edits(&document, Some("1.2.3.4:29070"));
        assert!(here.off.is_empty(), "the server's own edit wins");
        assert_eq!(
            here.moved.get(&Fixture::Surface(5)),
            Some(&Vec3::new(0.0, 0.0, 32.0))
        );
    }

    /// A BSP noted as loaded is known again by its parse alone, and another map is not.
    #[test]
    fn a_loaded_map_is_known_by_its_fingerprint() {
        let one = sjk_bsp::Bsp::empty([0.0; 3], [64.0; 3]);
        note_loaded("maps/mp/Lights_Test.bsp", -77, &one);
        assert_eq!(loaded(&one), Some(("mp/lights_test".to_owned(), -77)));
    }
}
