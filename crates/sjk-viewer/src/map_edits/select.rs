//! Better selections in the in-game map editor (Sol, 10/11/2026: "better selections"):
//! the hover (what is under the pointer outlined faintly, with a one-line tip), a box
//! drawn with the left button over the scene (Shift adds, Ctrl takes out), Ctrl+click
//! taking one thing out, `mapedit_select shader` (every surface and seen face drawn with
//! the selection's shader) and `mapedit_select placed` (every placed model), and
//! `mapedit_duplicate` (Ctrl+D: copies of the selected placed models, offset, selected).
//!
//! Nothing here runs a full scan of the map each frame: the hover picks once the pointer
//! has moved and [`HOVER_EVERY`] has passed, the box and the commands once when asked.

use super::document::Kind;
use crate::world_notes::{Combine, Member, Scene, Selection, Toggled};
use glam::Vec3;
use std::time::{Duration, Instant};

/// How far the pointer moves with the left button held before the press is a box.
pub(crate) const MARQUEE_MIN: f32 = 6.0;
/// How often the hover picks again at most while the pointer moves.
const HOVER_EVERY: Duration = Duration::from_millis(40);
/// How far a pick reaches (as `world_notes`').
const REACH: f32 = 16_384.0;
/// About how many rays a box casts over the drawn surfaces it covers.
const BOX_RAYS: f32 = 2_000.0;
/// How many caulk faces in a box are tried at most (each is tested for being seen).
const BOX_FACES: usize = 4_096;
/// How far a duplicate stands from its original by default (units).
pub(crate) const DUPLICATE_STEP: f32 = 16.0;

/// The pointer over the scene while the panel is open: where it is, a press that may
/// become a box, the box, the hover's last pick and its tip, and placed edits to select
/// once they are drawn (a duplicate's copies).
#[derive(Default)]
pub(crate) struct ScenePointer {
    /// The pointer over the scene (physical pixels), `None` over the column.
    pub(crate) at: Option<[f32; 2]>,
    press: Option<([f32; 2], Combine)>,
    /// The box being drawn: its corners (physical pixels, low then high).
    pub(crate) marquee: Option<[f32; 4]>,
    hovered: Option<([f32; 2], Instant)>,
    /// The hover's line: kind and shader or model.
    pub(crate) tip: Option<String>,
    select_ids: Vec<u32>,
}

/// The box between `a` and `b` (physical pixels): low corner then high.
pub(crate) fn rect_of(a: [f32; 2], b: [f32; 2]) -> [f32; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[0].max(b[0]),
        a[1].max(b[1]),
    ]
}

/// Whether `point` lies in `rect` ([`rect_of`]'s).
pub(crate) fn inside(rect: [f32; 4], point: [f32; 2]) -> bool {
    (rect[0]..=rect[2]).contains(&point[0]) && (rect[1]..=rect[3]).contains(&point[1])
}

/// The points a box `rect` casts its rays through: a grid about [`BOX_RAYS`] strong,
/// never finer than 4 pixels, the corners' middles included.
pub(crate) fn samples(rect: [f32; 4]) -> Vec<[f32; 2]> {
    let (width, height) = (rect[2] - rect[0], rect[3] - rect[1]);
    let step = (width * height / BOX_RAYS).sqrt().max(4.0);
    let (columns, rows) = (
        (width / step).ceil().max(1.0) as usize,
        (height / step).ceil().max(1.0) as usize,
    );
    let mut points = Vec::with_capacity((columns + 1) * (rows + 1));
    for row in 0..=rows {
        for column in 0..=columns {
            points.push([
                rect[0] + width * column as f32 / columns as f32,
                rect[1] + height * row as f32 / rows as f32,
            ]);
        }
    }
    points
}

/// Whether the hover picks again: the pointer moved two pixels or more since the last
/// pick and [`HOVER_EVERY`] passed.
fn hover_due(last: Option<([f32; 2], Instant)>, at: [f32; 2], now: Instant) -> bool {
    last.is_none_or(|(from, when)| {
        ((from[0] - at[0]).abs() >= 2.0 || (from[1] - at[1]).abs() >= 2.0)
            && now.duration_since(when) >= HOVER_EVERY
    })
}

/// How a box or a Select command's answer reads.
fn combined_line(
    what: &str,
    found: usize,
    (count, capped): (usize, bool),
    combine: Combine,
) -> String {
    let mut line = match combine {
        Combine::Replace => format!("{what}: {count} selected"),
        Combine::Add => format!("{what}: {found} found, {count} selected"),
        Combine::Remove => format!("{what}: taken out, {count} selected"),
    };
    if capped {
        line.push_str(&format!(
            " (the selection holds {} at most, {} placed models)",
            crate::world_notes::SET_LIMIT,
            crate::world_notes::PLACED_LIMIT
        ));
    }
    line
}

/// The scene the editor's picks look at (fields of `$gpu`, borrowed apart).
macro_rules! scene {
    ($gpu:ident, $map:expr, $entities:expr) => {
        Scene {
            bsp: &$gpu.bsp,
            surfaces: &$gpu.decal_surfaces,
            scratch: &mut $gpu.trace_scratch,
            vfs: $gpu.vfs.as_deref(),
            map: $map,
            remaps: Vec::new(),
            markers: Vec::new(),
            entities: $entities,
        }
    };
}

impl crate::GpuState {
    /// The left button pressed on the scene: a click, or the start of a box, decided at
    /// the release ([`MARQUEE_MIN`]); Shift adds, Ctrl takes out.
    pub(super) fn map_edit_scene_press(&mut self, point: [f32; 2], combine: Combine) {
        let scene = &mut self.map_edits.panel.scene;
        scene.press = Some((point, combine));
        scene.marquee = None;
    }

    /// The pointer moved over the scene (or anywhere while a press is held): the hover
    /// follows it, a press held far enough draws a box. Whether a press is held.
    pub(super) fn map_edit_scene_move(&mut self, point: [f32; 2]) -> bool {
        let scene_right = self.map_edits.panel.scene_right;
        // The preview card stands over the scene's edge: no hover under it.
        let card = self
            .map_edits
            .panel
            .preview_rect
            .is_some_and(|rect| rect.contains(sjk_ui::Vec2::new(point[0], point[1])));
        let scene = &mut self.map_edits.panel.scene;
        scene.at = (point[0] < scene_right && !card).then_some(point);
        let Some((start, _)) = scene.press else {
            return false;
        };
        let far = (point[0] - start[0]).abs().max((point[1] - start[1]).abs());
        if scene.marquee.is_some() || far >= MARQUEE_MIN {
            // Kept on the scene: the column is no part of it.
            let point = [point[0].min(scene_right - 1.0), point[1]];
            scene.marquee = Some(rect_of(start, point));
            scene.tip = None;
        }
        true
    }

    /// The left button released after a press on the scene: the box selects what it
    /// covers, else the click selects what is under the pointer. Whether a press was held.
    pub(super) fn map_edit_scene_release(&mut self) -> bool {
        let scene = &mut self.map_edits.panel.scene;
        let Some((start, combine)) = scene.press.take() else {
            return false;
        };
        let handled = self.map_edit_scene_select(start, combine);
        self.world_notes.recheck_hover();
        handled
    }

    /// What a released press selects: the box drawn, else the thing at `start`.
    fn map_edit_scene_select(&mut self, start: [f32; 2], combine: Combine) -> bool {
        let scene = &mut self.map_edits.panel.scene;
        if let Some(rect) = scene.marquee.take() {
            let line = self.map_edit_box(rect, combine);
            let panel = &mut self.map_edits.panel;
            panel.answer(line);
            if panel.tab != super::panel::Tab::Place {
                panel.show(super::panel::Tab::Selected);
            }
            panel.stale();
            return true;
        }
        match combine {
            Combine::Replace => {
                let picked = self.map_edit_pick_at(start);
                self.map_edit_picked(picked, "Nothing there to edit");
            }
            Combine::Add => {
                let toggled = self.map_edit_toggle_pick(Some(start));
                self.map_edit_toggled(toggled);
            }
            Combine::Remove => {
                let toggled = self.map_edit_remove_at(start);
                self.map_edit_toggled(toggled);
            }
        }
        true
    }

    /// Ctrl+click: take what is under `point` out of the selection, when it is in it.
    fn map_edit_remove_at(&mut self, point: [f32; 2]) -> Toggled {
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let entities = self.map_edits.pick_entities(&self.bsp);
        let candidate = self
            .world_notes
            .candidate_at(scene!(self, &map, entities), point);
        let Some(member) = candidate.as_ref().and_then(Selection::member) else {
            return Toggled::Nothing;
        };
        if !self.world_notes.holds_member(member) {
            return Toggled::Removed(self.world_notes.edit_selection_len());
        }
        let entities = self.map_edits.pick_entities(&self.bsp);
        self.world_notes
            .toggle_for_edit_at(scene!(self, &map, entities), point)
    }

    /// Select what box `rect` (physical pixels) covers, as `combine` says: the drawn
    /// surfaces its rays meet, the caulk and nodraw faces seen in it (no drawn surface in
    /// front, facing the view) and the placed models standing in it.
    fn map_edit_box(&mut self, rect: [f32; 4], combine: Combine) -> Result<Vec<String>, String> {
        let (eye, view_projection, viewport) = self
            .world_notes
            .view_for_box()
            .ok_or("No view to select in")?;
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let project = |point: Vec3| {
            let clip = view_projection * point.extend(1.0);
            (clip.w > 0.5).then(|| {
                [
                    (clip.x / clip.w + 1.0) * 0.5 * viewport[0],
                    (1.0 - clip.y / clip.w) * 0.5 * viewport[1],
                ]
            })
        };
        let world_surfaces = self
            .bsp
            .render()
            .models()
            .first()
            .map_or(0..0, |world| world.surfaces.clone());
        // Drawn surfaces: what the rays through the box meet first.
        let mut surfaces: Vec<(usize, Vec3, Vec3)> = Vec::new();
        for point in samples(rect) {
            let Some((from, direction)) = self.world_notes.ray_at(point) else {
                continue;
            };
            let Some(hit) = self.decal_surfaces.ray_hit(from, direction, REACH) else {
                continue;
            };
            if !world_surfaces.contains(&hit.surface)
                || surfaces.iter().any(|(index, ..)| *index == hit.surface)
                || self.map_edit_sky_surface(hit.surface)
            {
                continue;
            }
            surfaces.push((hit.surface, from + direction * hit.distance, hit.normal));
        }
        // Caulk and nodraw faces seen in the box: facing the view, nothing drawn before
        // their middle, in an open part of the map.
        let mut faces: Vec<super::faces::Face> = Vec::new();
        {
            let catalog = self.shaders.clone();
            let (sides, _) = self.map_edits.web_cache.sides_and_drawn(
                &self.world_load_map,
                &self.bsp,
                Some(&catalog),
            );
            let open = |point: Vec3| super::web::caulk::open_at(&self.bsp, point);
            let mut tried = 0;
            for index in 0..sides.len() {
                let side = sides.side(index);
                if !side.showable || side.sky || side.polygon.len() < 3 {
                    continue;
                }
                let middle = super::linked::centre(&side.polygon);
                if side.normal.dot(eye - middle) <= 0.0
                    || !project(middle).is_some_and(|at| inside(rect, at))
                {
                    continue;
                }
                tried += 1;
                if tried > BOX_FACES {
                    break;
                }
                let distance = middle.distance(eye);
                let direction = (middle - eye) / distance.max(1e-3);
                if self
                    .decal_surfaces
                    .ray_hit(eye, direction, distance - 1.0)
                    .is_some()
                    || !sides.visible(index, &open)
                {
                    continue;
                }
                faces.push(super::faces::Face {
                    brush: side.brush,
                    side: side.side,
                    shader: side.shader.clone(),
                    sky: false,
                    distance,
                    normal: side.normal,
                    polygon: side.polygon.iter().map(|p| p.to_array()).collect(),
                });
            }
        }
        // Placed models standing in the box with nothing drawn before them.
        let placed: Vec<u32> = self
            .map_edits
            .placed
            .iter()
            .filter(|placed| {
                let origin = Vec3::from_array(placed.origin) + Vec3::Z * 8.0;
                let distance = origin.distance(eye);
                project(origin).is_some_and(|at| inside(rect, at))
                    && self
                        .decal_surfaces
                        .ray_hit(eye, (origin - eye) / distance.max(1e-3), distance - 8.0)
                        .is_none()
            })
            .map(|placed| placed.id)
            .collect();
        let mut found = self.map_edit_placed_selections(&map, &placed);
        let entities = self.map_edits.pick_entities(&self.bsp);
        let scene = scene!(self, &map, entities);
        found.extend(surfaces.into_iter().filter_map(|(index, point, normal)| {
            self.world_notes
                .surface_for_edit(&scene, index, point, normal)
        }));
        found.extend(
            faces
                .into_iter()
                .filter_map(|face| self.world_notes.face_for_edit(&map, face)),
        );
        let count = found.len();
        if count == 0 && combine != Combine::Remove {
            return Err("Nothing to edit in that box".into());
        }
        let result = self.world_notes.combine_for_edit(found, combine);
        Ok(vec![combined_line("Box", count, result, combine)])
    }

    /// Whether drawn surface `index` is drawn as the sky (no edit's: its sky side is).
    pub(super) fn map_edit_sky_surface(&self, index: usize) -> bool {
        self.bsp
            .render()
            .surfaces()
            .get(index)
            .and_then(|surface| self.bsp.shaders().get(surface.shader))
            .is_some_and(|shader| shader.surface_flags & super::faces::SURF_SKY != 0)
    }

    /// Placed edits `ids` drawn now, as selections.
    pub(super) fn map_edit_placed_selections(&mut self, map: &str, ids: &[u32]) -> Vec<Selection> {
        if ids.is_empty() {
            return Vec::new();
        }
        let lump =
            sjk_entity::parse_entity_lump(self.bsp.entities()).map_or(0, |entities| entities.len());
        let numbers: Vec<usize> = ids
            .iter()
            .filter_map(|id| {
                self.map_edits
                    .placed
                    .iter()
                    .position(|placed| placed.id == *id)
                    .map(|at| lump + at)
            })
            .collect();
        let entities = self.map_edits.pick_entities(&self.bsp);
        let scene = scene!(self, map, entities);
        numbers
            .into_iter()
            .filter_map(|number| self.world_notes.model_for_edit(&scene, number))
            .collect()
    }

    /// `mapedit_select shader|placed [add|remove]`: every drawn surface of the map and
    /// every seen caulk or nodraw face drawn with the selection's shader, or every placed
    /// model, become the selection (join it, leave it), within its limits.
    pub(super) fn map_edit_select_command(
        &mut self,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        let usage = "usage: mapedit_select shader|placed [add|remove] | grow|shrink|invert";
        let ring = match args {
            [what] if what == "grow" => Some(super::select_grow::Ring::Grow),
            [what] if what == "shrink" => Some(super::select_grow::Ring::Shrink),
            [what] if what == "invert" => Some(super::select_grow::Ring::Invert),
            _ => None,
        };
        if let Some(ring) = ring {
            return self.map_edit_ring(ring).map(|line| vec![line]);
        }
        let (what, combine) = match args {
            [what] => (what.as_str(), Combine::Replace),
            [what, how] => (
                what.as_str(),
                match how.as_str() {
                    "add" => Combine::Add,
                    "remove" => Combine::Remove,
                    _ => return Err(usage.into()),
                },
            ),
            _ => return Err(usage.into()),
        };
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let (label, found) = match what {
            "placed" => {
                let ids: Vec<u32> = self
                    .map_edits
                    .placed
                    .iter()
                    .map(|placed| placed.id)
                    .take(crate::world_notes::PLACED_LIMIT)
                    .collect();
                if ids.is_empty() {
                    return Err("No placed model on this map".into());
                }
                (
                    "Placed models".to_owned(),
                    self.map_edit_placed_selections(&map, &ids),
                )
            }
            "shader" => {
                let shader = self
                    .world_notes
                    .waiting_selection()
                    .and_then(|selection| match selection.edit_target() {
                        crate::world_notes::EditTarget::Surface { shader, .. } => Some(shader),
                        crate::world_notes::EditTarget::Face {
                            shader, sky: false, ..
                        } => Some(shader),
                        _ => None,
                    })
                    .ok_or(
                        "Select a surface or a caulk face first: Same shader takes its shader",
                    )?;
                let found = self.map_edit_same_shader(&map, &shader);
                (format!("Same shader ({})", short_shader(&shader)), found)
            }
            _ => return Err(usage.into()),
        };
        let count = found.len();
        let result = self.world_notes.combine_for_edit(found, combine);
        Ok(vec![combined_line(&label, count, result, combine)])
    }

    /// Every drawn surface of the world drawn with `shader` (not hidden) and every seen
    /// caulk or nodraw face of it, the selection's own first, within the selection's limit.
    fn map_edit_same_shader(&mut self, map: &str, shader: &str) -> Vec<Selection> {
        let limit = crate::world_notes::SET_LIMIT;
        let own: Option<Member> = self
            .world_notes
            .waiting_selection()
            .and_then(Selection::member);
        let render = self.bsp.render();
        let mut surfaces: Vec<(usize, Vec3, Vec3)> = Vec::new();
        if let Some(world) = render.models().first() {
            for index in world.surfaces.clone() {
                if surfaces.len() == limit {
                    break;
                }
                let same = render
                    .surfaces()
                    .get(index)
                    .and_then(|surface| self.bsp.shaders().get(surface.shader))
                    .is_some_and(|s| s.name_lossy().eq_ignore_ascii_case(shader));
                if !same || self.decal_surfaces.is_hidden(index) {
                    continue;
                }
                let Some(&[a, b, c]) = self.decal_surfaces.surface_triangles(index, 1).first()
                else {
                    continue;
                };
                let normal = (b - a).cross(c - a).normalize_or_zero();
                surfaces.push((index, (a + b + c) / 3.0, normal));
            }
        }
        let mut faces: Vec<super::faces::Face> = Vec::new();
        {
            let catalog = self.shaders.clone();
            let (sides, _) = self.map_edits.web_cache.sides_and_drawn(
                &self.world_load_map,
                &self.bsp,
                Some(&catalog),
            );
            let open = |point: Vec3| super::web::caulk::open_at(&self.bsp, point);
            let eye = self.world_notes.view().map_or(Vec3::ZERO, |(eye, _)| eye);
            for index in 0..sides.len() {
                if surfaces.len() + faces.len() >= limit {
                    break;
                }
                let side = sides.side(index);
                if !side.showable || side.sky || !side.shader.eq_ignore_ascii_case(shader) {
                    continue;
                }
                if !sides.visible(index, &open) {
                    continue;
                }
                let middle = super::linked::centre(&side.polygon);
                faces.push(super::faces::Face {
                    brush: side.brush,
                    side: side.side,
                    shader: side.shader.clone(),
                    sky: false,
                    distance: middle.distance(eye),
                    normal: side.normal,
                    polygon: side.polygon.iter().map(|p| p.to_array()).collect(),
                });
            }
        }
        let entities = self.map_edits.pick_entities(&self.bsp);
        let scene = scene!(self, map, entities);
        let mut found: Vec<Selection> = surfaces
            .into_iter()
            .filter_map(|(index, point, normal)| {
                self.world_notes
                    .surface_for_edit(&scene, index, point, normal)
            })
            .collect();
        found.extend(
            faces
                .into_iter()
                .filter_map(|face| self.world_notes.face_for_edit(map, face)),
        );
        // The selection's own stays the newest.
        if let Some(at) = found
            .iter()
            .position(|s| own.is_some() && s.member() == own)
        {
            found[..=at].rotate_right(1);
        }
        found
    }

    /// `mapedit_duplicate [dx dy dz]`: a copy of each selected placed model (the set's,
    /// else the current one) moved by the offset (by default [`DUPLICATE_STEP`] to the
    /// view's right along the nearest world axis), all in one step to undo; the copies
    /// become the selection once drawn, so G moves them at once.
    pub(super) fn map_edit_duplicate(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let offset = match args {
            [] => self
                .map_edit_nudge(DUPLICATE_STEP, 0.0)
                .unwrap_or([DUPLICATE_STEP, 0.0, 0.0]),
            [x, y, z] => [x, y, z].map(|v| v.parse::<f32>().unwrap_or(f32::NAN)),
            _ => return Err("usage: mapedit_duplicate [dx dy dz]".into()),
        };
        if offset.iter().any(|v| !v.is_finite() || v.abs() > 4096.0) {
            return Err("the offset is three numbers of at most 4096 units".into());
        }
        let mut ids = self.map_edit_set_ids();
        if ids.is_empty() {
            let selected = self
                .world_notes
                .waiting_selection()
                .and_then(Selection::placed_id);
            ids.extend(selected.or(self.map_edits.editor.current()));
        }
        if ids.is_empty() {
            return Err("Select placed models to duplicate (Shift+click adds more)".into());
        }
        let server = self.map_edit_server()?;
        let copies: Vec<Kind> = self
            .map_edits
            .document
            .as_ref()
            .map(|document| {
                ids.iter()
                    .filter_map(|id| document.edits.iter().find(|edit| edit.id == *id))
                    .filter_map(|edit| match &edit.kind {
                        Kind::Place {
                            model,
                            origin,
                            angles,
                            scale,
                        } => Some(Kind::Place {
                            model: model.clone(),
                            origin: [0, 1, 2].map(|axis| origin[axis] + offset[axis]),
                            angles: *angles,
                            scale: *scale,
                        }),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if copies.is_empty() {
            return Err("No placed model selected to duplicate".into());
        }
        let document = self.map_edit_document()?;
        let new: Vec<u32> = copies
            .into_iter()
            .map(|kind| document.add(kind, server.clone()))
            .collect();
        if let Some(&first) = new.first() {
            self.map_edits.editor.set_current(first);
        }
        let count = new.len();
        self.map_edits.panel.scene.select_ids = new;
        self.map_edit_saved(format!(
            "Duplicated {count} placed model{} {:.0} {:.0} {:.0} away (G moves them)",
            if count == 1 { "" } else { "s" },
            offset[0],
            offset[1],
            offset[2]
        ))
    }

    /// Once a frame while the panel is open: select a duplicate's copies once drawn, and
    /// pick what is under the pointer for the hover when it moved ([`hover_due`]), not
    /// while flying, drawing a box, grabbing or transforming.
    pub(crate) fn map_edit_hover_tick(&mut self) {
        if !self.map_edits.panel.scene.select_ids.is_empty() {
            let ids = std::mem::take(&mut self.map_edits.panel.scene.select_ids);
            if ids
                .iter()
                .all(|id| self.map_edits.placed.iter().any(|placed| placed.id == *id))
            {
                let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
                let found = self.map_edit_placed_selections(&map, &ids);
                self.world_notes.combine_for_edit(found, Combine::Replace);
                self.map_edits.panel.stale();
            } else {
                self.map_edits.panel.scene.select_ids = ids;
            }
        }
        let busy = self.map_edit_flying()
            || self.map_edits.editor.grabbing()
            || self.map_edits.gizmo.busy()
            || self.map_edits.panel.scene.marquee.is_some()
            || !self.map_edit_panel_open();
        let at = self.map_edits.panel.scene.at.filter(|_| !busy);
        let Some(at) = at else {
            if self.world_notes.hover().is_some() {
                self.world_notes.set_hover(None);
            }
            self.map_edits.panel.scene.tip = None;
            self.map_edits.panel.scene.hovered = None;
            return;
        };
        let now = Instant::now();
        if !hover_due(self.map_edits.panel.scene.hovered, at, now) {
            return;
        }
        self.map_edits.panel.scene.hovered = Some((at, now));
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let entities = self.map_edits.pick_entities(&self.bsp);
        let hover = self
            .world_notes
            .candidate_at(scene!(self, &map, entities), at);
        let tip = hover.as_ref().map(|hover| self.map_edit_hover_tip(hover));
        self.world_notes.set_hover(hover);
        self.map_edits.panel.scene.tip = tip;
    }

    /// The hover's line: what it is and its shader or model, as Selected names it.
    fn map_edit_hover_tip(&self, hover: &Selection) -> String {
        use crate::world_notes::EditTarget;
        let short = |path: &str| super::editor::short_name(path).to_owned();
        match hover.edit_target() {
            EditTarget::Model {
                number,
                classname,
                model,
                ..
            } => {
                if hover.placed_id().is_some() {
                    format!("Placed model: {}", short(&model))
                } else if self.static_models.is_prop(number as u32) {
                    format!("Map prop: {}", short(&model))
                } else {
                    format!("Game entity: {classname}")
                }
            }
            EditTarget::Surface { shader, .. } => format!("Surface: {shader}"),
            EditTarget::Face {
                shader, sky: true, ..
            } => format!("Sky: {shader}"),
            EditTarget::Face { shader, .. } => format!("Hidden face: {shader}"),
            EditTarget::Other => "Not something edits change".to_owned(),
        }
    }
}

/// A shader name without `textures/` or `models/`.
fn short_shader(shader: &str) -> &str {
    shader
        .strip_prefix("textures/")
        .or_else(|| shader.strip_prefix("models/"))
        .unwrap_or(shader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_its_corners_and_casts_a_bounded_grid() {
        let rect = rect_of([300.0, 200.0], [100.0, 50.0]);
        assert_eq!(rect, [100.0, 50.0, 300.0, 200.0]);
        assert!(inside(rect, [100.0, 200.0]) && inside(rect, [250.0, 60.0]));
        assert!(!inside(rect, [99.0, 60.0]) && !inside(rect, [150.0, 201.0]));
        let points = samples(rect);
        assert!(points.len() <= 2_400, "{}", points.len());
        assert!(points.iter().all(|p| inside(rect, *p)));
        assert!(points.contains(&[100.0, 50.0]) && points.contains(&[300.0, 200.0]));
        // a small box: every 4 pixels
        let small = samples(rect_of([0.0, 0.0], [8.0, 8.0]));
        assert_eq!(small.len(), 9);
        // a whole 4K screen stays near the budget
        assert!(samples([0.0, 0.0, 3840.0, 2160.0]).len() <= 2_200);
    }

    /// A box adds each thing once and keeps the limits; Ctrl takes out only what it holds;
    /// a plain box replaces; a thing selected alone gives way.
    #[test]
    fn a_box_meets_the_selection_as_its_keys_say() {
        let key = |member: &Option<Member>| *member;
        let surface = |index| Some(Member::Surface(index));
        let (mut primary, mut more) = (Some(surface(1)), vec![surface(2)]);
        let found = vec![surface(3), surface(2), surface(4)];
        let result =
            crate::world_notes::combine_in(&mut primary, &mut more, found, Combine::Add, key);
        assert_eq!(result, (4, false));
        assert_eq!(primary, Some(surface(3)), "the first found is the newest");
        let gone = crate::world_notes::combine_in(
            &mut primary,
            &mut more,
            vec![surface(3), surface(9)],
            Combine::Remove,
            key,
        );
        assert_eq!(gone, (3, false));
        assert!(primary != Some(surface(3)) && primary.is_some());
        let replaced = crate::world_notes::combine_in(
            &mut primary,
            &mut more,
            vec![surface(7)],
            Combine::Replace,
            key,
        );
        assert_eq!(replaced, (1, false));
        assert_eq!((primary, more.len()), (Some(surface(7)), 0));
        // a map prop (no member) gives way to what a box adds
        let (mut primary, mut more) = (Some(None), Vec::new());
        let added = crate::world_notes::combine_in(
            &mut primary,
            &mut more,
            vec![surface(5)],
            Combine::Add,
            key,
        );
        assert_eq!(added, (1, false));
        // the placed models' limit
        let limit = crate::world_notes::PLACED_LIMIT;
        let placed: Vec<Option<Member>> = (0..limit as u32 + 3)
            .map(|i| Some(Member::Placed(i)))
            .collect();
        let (mut primary, mut more) = (None, Vec::new());
        let full =
            crate::world_notes::combine_in(&mut primary, &mut more, placed, Combine::Add, key);
        assert_eq!(full, (limit, true));
    }

    #[test]
    fn the_hover_picks_again_only_when_the_pointer_moved_and_time_passed() {
        let now = Instant::now();
        assert!(hover_due(None, [10.0, 10.0], now));
        let earlier = now - Duration::from_millis(100);
        assert!(!hover_due(Some(([10.0, 10.0], earlier)), [11.0, 10.0], now));
        assert!(hover_due(Some(([10.0, 10.0], earlier)), [13.0, 10.0], now));
        assert!(!hover_due(Some(([10.0, 10.0], now)), [40.0, 10.0], now));
    }
}
