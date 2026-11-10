//! The live preview on caulk and nodraw faces (Sol, 10/11/2026: "continue, do the good
//! stuff"; skipped by the round before, [`super::preview`]): a shown face is a mesh of
//! its own, so previewing each row's shader on faces would have kept a mesh for every
//! row seen. Instead one preview mesh per world, of [`CORNERS`] corners, is made the
//! first time a face is previewed and kept for the world: its corners are written over
//! in place with the selected faces (their polygons, their texture laid for the shader
//! under the pointer) and the rest left as empty triangles. It is drawn with the newest
//! face's own shader (caulk, say, which the map draws nowhere), and the preview is a remap
//! of that shader to the row's, through the path every remap preview takes: no new
//! material per row, nothing kept once the pointer leaves the list but the mesh itself.
//! Faces past [`CORNERS`] corners are not previewed; a face an edit draws already is
//! hidden while the preview stands on it.

use super::document::{Align, Kind};
use crate::world_notes::EditTarget;
use crate::{ActorInstance, GpuState};

/// How many corners the preview mesh holds: the faces it previews at once.
pub(crate) const CORNERS: usize = 4_096;
/// Its triangles' indices: a face of `n` corners takes `3 (n - 2)`.
const INDICES: usize = CORNERS * 3;
/// The preview mesh's variant of the shown faces' model.
const VARIANT: &str = "*preview";

/// A face to lay on the preview mesh: its polygon, its normal and its alignment.
pub(crate) type FaceLay = (Vec<[f32; 3]>, [f32; 3], Align);

/// This world's preview mesh.
pub(crate) struct FacePreview {
    mesh: usize,
    vertex_base: u32,
    index_base: u32,
    /// The shader it is drawn with: what the preview remaps.
    pub(crate) shader: String,
    /// The faces written on it now (brush, side).
    pub(crate) faces: Vec<[u32; 2]>,
    /// Drawn this frame, lit from the newest face's middle.
    pub(crate) instance: Option<ActorInstance>,
}

/// The selected faces a preview draws (brush, side, own shader): caulk and nodraw sides,
/// not the sky (a sky row swaps the sky instead).
fn faces_of(targets: impl Iterator<Item = EditTarget>) -> Vec<(u32, u32, String)> {
    targets
        .filter_map(|target| match target {
            EditTarget::Face {
                brush,
                side,
                shader,
                sky: false,
            } => Some((brush, side, shader)),
            _ => None,
        })
        .collect()
}

/// The corners and the triangles of `faces` (polygon, normal, alignment) laid for a
/// texture of `size`, relative to `origin`, within the mesh's room: what is written over
/// the preview mesh, its indices from 0, the rest empty (all on corner 0). How many faces
/// fit.
pub(crate) fn layout(
    faces: &[FaceLay],
    origin: [f32; 3],
    size: Option<[u32; 2]>,
) -> (Vec<crate::GpuVertex>, Vec<u32>, usize) {
    let mut corners = Vec::new();
    let mut indices = Vec::new();
    let mut fitted = 0;
    for (polygon, normal, align) in faces {
        if polygon.len() < 3 || corners.len() + polygon.len() > CORNERS {
            continue;
        }
        let base = corners.len() as u32;
        indices.extend(super::faces::fan_indices(polygon, *normal, base));
        corners.extend(super::faces::vertices(
            polygon, *normal, origin, size, align,
        ));
        fitted += 1;
    }
    corners.resize(CORNERS, bytemuck::Zeroable::zeroed());
    indices.resize(INDICES, 0);
    (corners, indices, fitted)
}

impl GpuState {
    /// Draw the selected caulk and nodraw faces with `shader` in preview: write them over
    /// the preview mesh (made the first time) and answer the shader to remap to `shader`,
    /// or `None` when no such face is selected (or the preview cannot be made).
    pub(super) fn map_edit_face_preview(&mut self, shader: &str) -> Option<String> {
        let faces = faces_of(
            self.world_notes
                .edit_selection()
                .map(crate::world_notes::Selection::edit_target),
        );
        let Some((_, _, own)) = faces.first() else {
            self.map_edit_face_preview_off();
            return None;
        };
        self.map_edit_face_preview_mesh(own)?;
        let server = self.map_edits_server();
        let aligns: Vec<Align> = faces
            .iter()
            .map(|(brush, side, _)| {
                self.map_edits
                    .document
                    .as_ref()
                    .and_then(|document| {
                        document
                            .active(server.as_deref())
                            .find_map(|edit| match &edit.kind {
                                Kind::ShowFace {
                                    brush: b,
                                    side: s,
                                    align,
                                    ..
                                } if b == brush && s == side => Some(*align),
                                _ => None,
                            })
                    })
                    .unwrap_or_default()
            })
            .collect();
        let polygons: Vec<FaceLay> = faces
            .iter()
            .zip(aligns)
            .filter_map(|((brush, side, own), align)| {
                super::faces::polygon(&self.bsp, *brush, *side, own)
                    .map(|(polygon, normal)| (polygon, normal, align))
            })
            .collect();
        let origin = polygons
            .first()
            .map_or([0.0; 3], |(polygon, _, _)| super::faces::centre(polygon));
        let size = self
            .vfs
            .as_ref()
            .and_then(|vfs| super::texture_size(vfs, &self.shaders, shader));
        let (corners, local, _) = layout(&polygons, origin, size);
        let preview = self.map_edits.face_preview.as_mut()?;
        let indices: Vec<u32> = local
            .iter()
            .map(|index| preview.vertex_base + index)
            .collect();
        let written = self
            .geometry
            .write_vertices(&self.queue, preview.vertex_base, &corners)
            && self
                .geometry
                .write_indices(&self.queue, preview.index_base, &indices);
        if !written {
            preview.instance = None;
            return None;
        }
        preview.faces = faces.iter().map(|(b, s, _)| [*b, *s]).collect();
        preview.instance = Some(ActorInstance::new(origin, [0.0, 0.0, 0.0, 1.0], [1.0; 3]));
        Some(preview.shader.clone())
    }

    /// The preview on faces stops drawing (the mesh stays for the world).
    pub(super) fn map_edit_face_preview_off(&mut self) {
        if let Some(preview) = &mut self.map_edits.face_preview {
            preview.instance = None;
            preview.faces.clear();
        }
    }

    /// The preview mesh, made with `shader` the first time (or again for a new world).
    fn map_edit_face_preview_mesh(&mut self, shader: &str) -> Option<()> {
        let appearance = sjk_runtime::Appearance {
            model: super::FACE_MODEL.to_owned(),
            variant: VARIANT.to_owned(),
        };
        let held = self.map_edits.face_preview.as_ref().is_some_and(|preview| {
            self.object_meshes
                .get(preview.mesh)
                .is_some_and(|mesh| mesh.appearance == appearance)
        });
        if held {
            return Some(());
        }
        self.map_edits.face_preview = None;
        // Made in place of any other world's: still corners, empty triangles.
        let scene = crate::scene_flatten::FlattenedScene {
            vertices: vec![bytemuck::Zeroable::zeroed(); CORNERS],
            indices: vec![0; INDICES],
            materials: vec![crate::scene_flatten::ViewerMaterial {
                shader: shader.to_owned(),
                lightmap: -1,
            }],
            ..Default::default()
        };
        let base = self.geometry.vertex_count();
        let installed = self.install_config_model(
            crate::object_meshes::StaticModelMesh {
                appearance: appearance.clone(),
                draws: vec![crate::scene_flatten::ActorDraw {
                    indices: 0..INDICES as u32,
                    material: 0,
                }],
                center: [0.0; 3],
                flash_bolt: None,
                turning_bone: None,
                eweb_part: None,
            },
            scene,
        );
        if let Err(error) = installed {
            crate::log::progress(format_args!("map edit face preview: {error}"));
            return None;
        }
        let mesh = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance == appearance)?;
        if self.geometry.vertex_count() != base + CORNERS as u32 {
            return None;
        }
        let index_base = self.object_meshes[mesh].draws.first()?.indices.start;
        self.map_edits.face_preview = Some(FacePreview {
            mesh,
            vertex_base: base,
            index_base,
            shader: shader.to_owned(),
            faces: Vec::new(),
            instance: None,
        });
        Some(())
    }

    /// Submit the preview mesh this frame, while it previews; whether face `[brush,
    /// side]` is under it (its own shown face is then not drawn).
    pub(super) fn map_edit_face_preview_instance(&mut self) -> Option<(usize, ActorInstance)> {
        let preview = self.map_edits.face_preview.as_ref()?;
        Some((preview.mesh, preview.instance?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f32) -> Vec<[f32; 3]> {
        vec![
            [x, 0.0, 0.0],
            [x + 64.0, 0.0, 0.0],
            [x + 64.0, 64.0, 0.0],
            [x, 64.0, 0.0],
        ]
    }

    /// Faces are laid one after the other with their own triangles, the rest of the mesh
    /// left empty, and faces past its room are left out.
    #[test]
    fn faces_are_laid_on_the_mesh_and_the_rest_is_empty() {
        let up = [0.0, 0.0, 1.0];
        let faces = vec![
            (square(0.0), up, Align::default()),
            (square(64.0), up, Align::default()),
        ];
        let (corners, indices, fitted) = layout(&faces, [32.0, 32.0, 0.0], Some([128, 128]));
        assert_eq!(fitted, 2);
        assert_eq!((corners.len(), indices.len()), (CORNERS, INDICES));
        assert_eq!(&indices[..6], &[0, 2, 1, 0, 3, 2]);
        assert_eq!(&indices[6..12], &[4, 6, 5, 4, 7, 6]);
        assert!(
            indices[12..].iter().all(|index| *index == 0),
            "empty triangles"
        );
        assert_eq!(corners[0].position, [-32.0, -32.0, 0.0]);
        assert_eq!(corners[4].position, [32.0, -32.0, 0.0]);
        // laid as Radiant would lay the shader under the pointer
        assert_eq!(corners[1].texture_coordinates, [1.0, 0.0]);
        // too many corners: the faces past the room are left out
        let many: Vec<_> = (0..CORNERS / 4 + 3)
            .map(|i| (square(i as f32 * 64.0), up, Align::default()))
            .collect();
        let (_, _, fitted) = layout(&many, [0.0; 3], None);
        assert_eq!(fitted, CORNERS / 4);
        // only caulk and nodraw faces are previewed this way, not the sky nor surfaces
        let picked = faces_of(
            [
                EditTarget::Face {
                    brush: 1,
                    side: 2,
                    shader: "textures/system/caulk".into(),
                    sky: false,
                },
                EditTarget::Face {
                    brush: 3,
                    side: 4,
                    shader: "textures/skies/night".into(),
                    sky: true,
                },
                EditTarget::Surface {
                    index: 9,
                    shader: "textures/a/b".into(),
                    kind: "planar",
                },
            ]
            .into_iter(),
        );
        assert_eq!(picked, vec![(1, 2, "textures/system/caulk".to_owned())]);
    }
}
