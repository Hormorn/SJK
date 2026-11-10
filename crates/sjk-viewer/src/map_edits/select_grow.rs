//! Grow, shrink and invert the selection (Sol, 10/11/2026: "continue, do the good
//! stuff"), as Blender's Select More, Less and Invert: `mapedit_select grow` adds the
//! drawn surfaces sharing a corner with a selected one and the seen caulk, nodraw or sky
//! faces joined to a selected face (one ring); `shrink` takes out those on the
//! selection's edge, touching one that is not selected; `invert` selects every placed
//! model, or every drawn surface, that is not selected, within the selection's limit.
//! Each runs once when asked, never per frame.

use super::linked::Sides;
use crate::world_notes::{Combine, Member, Scene, Selection};
use glam::Vec3;
use std::collections::HashSet;

/// What `mapedit_select grow|shrink|invert` does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ring {
    Grow,
    Shrink,
    Invert,
}

/// The members of `members` on the selection's edge: those with a neighbour in
/// `neighbours` (one list a member) that is not a member.
pub(crate) fn edge<T: Copy + Eq + std::hash::Hash>(members: &[T], neighbours: &[Vec<T>]) -> Vec<T> {
    let set: HashSet<T> = members.iter().copied().collect();
    members
        .iter()
        .zip(neighbours)
        .filter(|(_, around)| around.iter().any(|other| !set.contains(other)))
        .map(|(member, _)| *member)
        .collect()
}

/// The neighbours of `neighbours` (one list a member of `members`) that are not members,
/// each once, in order: one ring out.
pub(crate) fn ring<T: Copy + Eq + std::hash::Hash>(members: &[T], neighbours: &[Vec<T>]) -> Vec<T> {
    let mut seen: HashSet<T> = members.iter().copied().collect();
    neighbours
        .iter()
        .flatten()
        .copied()
        .filter(|other| seen.insert(*other))
        .collect()
}

impl crate::GpuState {
    /// `mapedit_select grow|shrink|invert` ([`Ring`]): the answer line.
    pub(super) fn map_edit_ring(&mut self, ring: Ring) -> Result<String, String> {
        let members: Vec<Member> = self
            .world_notes
            .edit_selection()
            .filter_map(Selection::member)
            .collect();
        if members.is_empty() {
            return Err("Select surfaces, faces or placed models first".into());
        }
        let split = super::editor::split_members(&members);
        let surfaces: Vec<usize> = split.surfaces.iter().map(|&s| s as usize).collect();
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let world = self
            .bsp
            .render()
            .models()
            .first()
            .map_or(0..0, |world| world.surfaces.clone());
        let drawable = |index: usize| world.contains(&index) && !self.map_edit_sky_surface(index);
        match ring {
            Ring::Grow | Ring::Shrink => {
                let around = self.decal_surfaces.touching(&surfaces, drawable);
                let (faces, face_rings) = self.map_edit_face_rings(&split.faces);
                if ring == Ring::Grow {
                    let new_surfaces = super::select_grow::ring(&surfaces, &around);
                    let new_faces = super::select_grow::ring(&faces, &face_rings);
                    if new_surfaces.is_empty() && new_faces.is_empty() {
                        return Ok("Grow: nothing more joins the selection".into());
                    }
                    let added = new_surfaces.len() + new_faces.len();
                    // The selection as it is (its newest kept the newest), then the ring.
                    let mut found: Vec<Selection> =
                        self.world_notes.edit_selection().cloned().collect();
                    found.extend(self.map_edit_surface_selections(&map, &new_surfaces));
                    found.extend(self.map_edit_face_selections(&map, &new_faces));
                    let (count, capped) =
                        self.world_notes.combine_for_edit(found, Combine::Replace);
                    return Ok(format!(
                        "Grow: {added} joined, {count} selected{}",
                        capped_words(capped)
                    ));
                }
                let mut gone: Vec<Member> = super::select_grow::edge(&surfaces, &around)
                    .into_iter()
                    .map(Member::Surface)
                    .collect();
                let sides = self
                    .map_edits
                    .web_cache
                    .sides(&self.world_load_map, &self.bsp);
                gone.extend(
                    super::select_grow::edge(&faces, &face_rings)
                        .into_iter()
                        .map(|index| {
                            let side = sides.side(index);
                            Member::Face {
                                brush: side.brush,
                                side: side.side,
                            }
                        }),
                );
                let shrinking = surfaces.len() + faces.len();
                if gone.is_empty() {
                    return Ok(
                        "Shrink: no selected surface or face is on the selection's edge".into(),
                    );
                }
                if gone.len() == shrinking && split.placed.is_empty() {
                    return Err(
                        "Shrink: every one is on the selection's edge, nothing would be left"
                            .into(),
                    );
                }
                let removed: Vec<Selection> = self
                    .world_notes
                    .edit_selection()
                    .filter(|selection| selection.member().is_some_and(|m| gone.contains(&m)))
                    .cloned()
                    .collect();
                let (count, _) = self.world_notes.combine_for_edit(removed, Combine::Remove);
                Ok(format!(
                    "Shrink: {} off the edge, {count} selected",
                    gone.len()
                ))
            }
            Ring::Invert => {
                if !split.faces.is_empty() {
                    return Err(
                        "Invert takes placed models and drawn surfaces; Same plane and Connected select faces"
                            .into(),
                    );
                }
                let limit = crate::world_notes::SET_LIMIT;
                let other_surfaces: Vec<usize> = if surfaces.is_empty() {
                    Vec::new()
                } else {
                    let selected: HashSet<usize> = surfaces.iter().copied().collect();
                    world
                        .clone()
                        .filter(|&index| {
                            !selected.contains(&index)
                                && drawable(index)
                                && !self.decal_surfaces.is_hidden(index)
                                && self.decal_surfaces.surface_triangles(index, 1).len() == 1
                        })
                        .collect()
                };
                if other_surfaces.len() > limit {
                    return Err(format!(
                        "Invert would select {} surfaces: more than the {limit} a selection holds",
                        other_surfaces.len()
                    ));
                }
                let mut found: Vec<Selection> = Vec::new();
                if !split.placed.is_empty() {
                    let others: Vec<u32> = self
                        .map_edits
                        .placed
                        .iter()
                        .map(|placed| placed.id)
                        .filter(|id| !split.placed.contains(id))
                        .collect();
                    if others.len() > crate::world_notes::PLACED_LIMIT {
                        return Err(format!(
                            "Invert would select {} placed models: more than the {} a selection holds",
                            others.len(),
                            crate::world_notes::PLACED_LIMIT
                        ));
                    }
                    found.extend(self.map_edit_placed_selections(&map, &others));
                }
                if found.len() + other_surfaces.len() > limit {
                    return Err(format!(
                        "Invert would select {}: more than the {limit} a selection holds",
                        found.len() + other_surfaces.len()
                    ));
                }
                found.extend(self.map_edit_surface_selections(&map, &other_surfaces));
                if found.is_empty() {
                    return Ok("Invert: everything of the kind is selected already".into());
                }
                let (count, capped) = self.world_notes.combine_for_edit(found, Combine::Replace);
                Ok(format!("Invert: {count} selected{}", capped_words(capped)))
            }
        }
    }

    /// The selected faces (brush, side) as indices of the map's sides, and each one's ring.
    fn map_edit_face_rings(&mut self, faces: &[[u32; 2]]) -> (Vec<usize>, Vec<Vec<usize>>) {
        if faces.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let bsp = &self.bsp;
        let sides: &Sides = self.map_edits.web_cache.sides(&self.world_load_map, bsp);
        let indices: Vec<usize> = faces
            .iter()
            .filter_map(|[brush, side]| sides.find(*brush, *side))
            .collect();
        let open = |point: Vec3| super::web::caulk::open_at(bsp, point);
        let rings = sides.joined(&indices, &open);
        (indices, rings)
    }

    /// Drawn surfaces `indices` as selections from the last rendered view, met at their
    /// first triangle's middle.
    fn map_edit_surface_selections(&mut self, map: &str, indices: &[usize]) -> Vec<Selection> {
        let met: Vec<(usize, Vec3, Vec3)> = indices
            .iter()
            .filter_map(|&index| {
                let &[a, b, c] = self.decal_surfaces.surface_triangles(index, 1).first()?;
                Some((
                    index,
                    (a + b + c) / 3.0,
                    (b - a).cross(c - a).normalize_or_zero(),
                ))
            })
            .collect();
        let entities = self.map_edits.pick_entities(&self.bsp);
        let scene = Scene {
            bsp: &self.bsp,
            surfaces: &self.decal_surfaces,
            scratch: &mut self.trace_scratch,
            vfs: self.vfs.as_deref(),
            map,
            remaps: Vec::new(),
            markers: Vec::new(),
            entities,
        };
        met.into_iter()
            .filter_map(|(index, point, normal)| {
                self.world_notes
                    .surface_for_edit(&scene, index, point, normal)
            })
            .collect()
    }

    /// Faces `indices` of the map's sides as selections from the last rendered view.
    fn map_edit_face_selections(&mut self, map: &str, indices: &[usize]) -> Vec<Selection> {
        if indices.is_empty() {
            return Vec::new();
        }
        let eye = self.world_notes.view().map_or(Vec3::ZERO, |(eye, _)| eye);
        let sides = self
            .map_edits
            .web_cache
            .sides(&self.world_load_map, &self.bsp);
        let faces: Vec<super::faces::Face> = indices
            .iter()
            .map(|&index| {
                let side = sides.side(index);
                let middle = super::linked::centre(&side.polygon);
                super::faces::Face {
                    brush: side.brush,
                    side: side.side,
                    shader: side.shader.clone(),
                    sky: side.sky,
                    distance: middle.distance(eye),
                    normal: side.normal,
                    polygon: side.polygon.iter().map(|p| p.to_array()).collect(),
                }
            })
            .collect();
        faces
            .into_iter()
            .filter_map(|face| self.world_notes.face_for_edit(map, face))
            .collect()
    }
}

fn capped_words(capped: bool) -> String {
    if capped {
        format!(
            " (the selection holds {} at most)",
            crate::world_notes::SET_LIMIT
        )
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row of five: growing from the middle takes its two neighbours, shrinking three in
    /// a row takes the two ends, and a lone member is all edge.
    #[test]
    fn a_ring_grows_out_and_an_edge_shrinks_in() {
        let around = |index: usize| -> Vec<usize> {
            [index.wrapping_sub(1), index + 1]
                .into_iter()
                .filter(|other| *other < 5)
                .collect()
        };
        let rings: Vec<Vec<usize>> = [2].iter().map(|&i| around(i)).collect();
        assert_eq!(ring(&[2], &rings), vec![1, 3]);
        let three = [1, 2, 3];
        let rings: Vec<Vec<usize>> = three.iter().map(|&i| around(i)).collect();
        assert_eq!(ring(&three, &rings), vec![0, 4]);
        assert_eq!(edge(&three, &rings), vec![1, 3]);
        let all = [0, 1, 2, 3, 4];
        let rings: Vec<Vec<usize>> = all.iter().map(|&i| around(i)).collect();
        assert!(ring(&all, &rings).is_empty() && edge(&all, &rings).is_empty());
        assert_eq!(edge(&[2], &[around(2)]), vec![2]);
    }
}
