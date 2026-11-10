//! Fly-to and frame (Sol, 10/11/2026: "continue, do the good stuff"): `mapedit_frame`
//! glides the editor's camera ([`super::fly`]) to frame the selection (F in the panel,
//! Blender's View Selected), or a point and a radius the panel names: a double click on
//! an edit of the Edits list, a thing of Place's things or a light or a lamp of Lights.
//! The view keeps its direction and steps back until the target fills it
//! ([`super::fly::frame`]). Only the client's camera moves: the body stays at its server
//! position.

use super::document::Kind;
use super::fly::sphere_of;
use crate::GpuState;
use glam::Vec3;

/// How many triangles of a hidden surface are read to frame it.
const SURFACE_TRIANGLES: usize = 512;
/// The radius a point-like target is framed with: a light, a prop, a thing.
pub(crate) const POINT_RADIUS: f32 = 48.0;

impl GpuState {
    /// `mapedit_frame [<x y z> [radius]]`: frame the selection, or the point given.
    pub(super) fn map_edit_frame_command(
        &mut self,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        let number = |text: &String| {
            text.parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("not a number: {text}"))
        };
        let (centre, radius) = match args {
            [] => self
                .map_edit_selection_sphere()
                .ok_or("Select something to frame first (F frames the selection)")?,
            [x, y, z] => (Vec3::new(number(x)?, number(y)?, number(z)?), POINT_RADIUS),
            [x, y, z, radius] => (
                Vec3::new(number(x)?, number(y)?, number(z)?),
                number(radius)?.clamp(0.0, 16_384.0),
            ),
            _ => return Err("usage: mapedit_frame [<x y z> [radius]]".into()),
        };
        self.map_edit_fly_to(centre, radius).map(|line| vec![line])
    }

    /// The middle and the radius of everything selected.
    pub(crate) fn map_edit_selection_sphere(&self) -> Option<(Vec3, f32)> {
        sphere_of(
            self.world_notes
                .edit_selection()
                .flat_map(crate::world_notes::Selection::extent_points),
        )
    }

    /// Where edit `id` of the map's document stands, and how big it is, when it has one
    /// place: a placed model, an added light, a lamp, a shown face, a hidden surface or
    /// prop. A remap or the weather has none.
    pub(crate) fn map_edit_edit_sphere(&self, id: u32) -> Result<(Vec3, f32), String> {
        let edit = self
            .map_edits
            .document
            .as_ref()
            .and_then(|document| document.edits.iter().find(|edit| edit.id == id))
            .ok_or("That edit is gone")?;
        let found = match &edit.kind {
            Kind::Place { origin, scale, .. } => Some((
                Vec3::from_array(*origin),
                POINT_RADIUS * scale.iter().copied().fold(1.0, f32::max),
            )),
            Kind::Light { origin, .. } => Some((Vec3::from_array(*origin), POINT_RADIUS)),
            Kind::ShowFace {
                brush,
                side,
                shader,
                ..
            } => super::faces::polygon(&self.bsp, *brush, *side, shader)
                .and_then(|(polygon, _)| sphere_of(polygon.into_iter().map(Vec3::from_array))),
            Kind::HideSurface { surface, .. } => sphere_of(
                self.decal_surfaces
                    .surface_triangles(*surface as usize, SURFACE_TRIANGLES)
                    .into_iter()
                    .flatten(),
            ),
            Kind::HideProp { entity, .. } => self.map_entity_origin(*entity as usize),
            Kind::Lamp {
                fixture, offset, ..
            } => self
                .map_edits
                .panel
                .facts
                .lamps
                .iter()
                .find(|lamp| lamp.fixture == *fixture)
                .map(|lamp| {
                    (
                        Vec3::from_array(lamp.position) + Vec3::from_array(*offset),
                        POINT_RADIUS,
                    )
                }),
            Kind::Remap { .. } | Kind::Weather { .. } => {
                return Err(format!("{}: it has no one place to fly to", edit.label()));
            }
        };
        found.ok_or_else(|| format!("{}: not found on this map", edit.label()))
    }

    /// The origin of entity `number` of the map's entity lump, framed as a point.
    fn map_entity_origin(&self, number: usize) -> Option<(Vec3, f32)> {
        let entities = sjk_entity::parse_entity_lump(self.bsp.entities()).ok()?;
        let origin = entities.get(number)?.get("origin")?;
        let mut words = origin.split_whitespace().map(str::parse::<f32>);
        let (Some(Ok(x)), Some(Ok(y)), Some(Ok(z))) = (words.next(), words.next(), words.next())
        else {
            return None;
        };
        Some((Vec3::new(x, y, z), POINT_RADIUS))
    }
}

/// The words of `mapedit_frame` for a target at `centre` of `radius`.
pub(crate) fn frame_words(centre: Vec3, radius: f32) -> Vec<String> {
    [centre.x, centre.y, centre.z, radius]
        .iter()
        .map(|value| format!("{value:.1}"))
        .collect()
}
