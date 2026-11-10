//! The live preview of a remap (10/11/2026): while the pointer rests on a row or tile of
//! Draw another shader (or the keyboard walks the list), the selected surfaces are drawn
//! with that shader in the scene, before anything is kept; leaving the list puts them
//! back. The preview is a remap the world draws over the document's own
//! (`World::set_edit_remaps`, the path remap edits take), never written to the document,
//! never sent to the hub, never a step to undo: Remap then keeps it as one step, as before.
//!
//! What it previews: drawn surfaces (their shader, as a remap edit would, on the whole map)
//! and a sky swapped for a sky. Caulk and nodraw faces have no surface to remap (a shown
//! face is a mesh of its own, built once and kept), so they are not previewed. A preview
//! waits [`SETTLE`] on a row before it is drawn, so walking past rows costs nothing; each
//! one drawn recompiles the materials of one shader, as a remap edit does.

use std::time::{Duration, Instant};

/// How long the pointer (or the keyboard) rests on a row before its shader is drawn.
pub(crate) const SETTLE: Duration = Duration::from_millis(250);

/// The preview's state, on the panel.
#[derive(Default)]
pub(crate) struct Preview {
    /// The shader drawn in preview now, and the shaders it stands in for.
    pub(crate) shown: Option<(String, Vec<String>)>,
    /// The shader wanted and since when.
    wanted: Option<(String, Instant)>,
}

/// What the preview should do this frame, from the shader `wanted` now (none: off the
/// list) and the state: draw a shader, put the map's back, or nothing.
#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    Keep,
    Draw(String),
    Revert,
}

impl Preview {
    /// The step for `wanted` at `now`: a new shader waits [`SETTLE`]; off the list, the
    /// map's is put back at once.
    pub(crate) fn step(&mut self, wanted: Option<&str>, now: Instant) -> Step {
        let Some(wanted) = wanted else {
            self.wanted = None;
            return if self.shown.is_some() {
                Step::Revert
            } else {
                Step::Keep
            };
        };
        if self
            .shown
            .as_ref()
            .is_some_and(|(shown, _)| shown == wanted)
        {
            self.wanted = None;
            return Step::Keep;
        }
        match &self.wanted {
            Some((known, since)) if known == wanted => {
                if now.duration_since(*since) >= SETTLE {
                    self.wanted = None;
                    Step::Draw(wanted.to_owned())
                } else {
                    Step::Keep
                }
            }
            _ => {
                self.wanted = Some((wanted.to_owned(), now));
                Step::Keep
            }
        }
    }

    /// The document's remaps were applied again (an edit, an undo): no preview is drawn.
    pub(crate) fn dropped(&mut self) {
        self.shown = None;
    }
}

/// The remaps the world draws: the document's `remaps` (from, to) with `preview`'s
/// shader standing for each of its shaders.
pub(crate) fn with_preview(
    remaps: &[(String, String)],
    preview: Option<&(String, Vec<String>)>,
) -> Vec<(String, String)> {
    let mut all = remaps.to_vec();
    if let Some((to, froms)) = preview {
        for from in froms {
            all.retain(|(known, _)| !known.eq_ignore_ascii_case(from));
            all.push((from.clone(), to.clone()));
        }
    }
    all
}

impl crate::GpuState {
    /// Once a frame while the panel is open: draw the shader under the pointer (or the
    /// keyboard's chosen row) on the selected surfaces once it settles, or put the map's
    /// back off the list.
    pub(super) fn map_edit_preview_tick(&mut self) {
        let wanted = self.map_edit_preview_wanted();
        let now = Instant::now();
        let step = self.map_edits.panel.preview.step(wanted.as_deref(), now);
        let shown = match step {
            Step::Keep => return,
            Step::Revert => {
                self.map_edit_face_preview_off();
                None
            }
            Step::Draw(shader) => {
                let mut froms = self.map_edit_preview_froms();
                // Caulk and nodraw faces: the preview mesh's own shader stands for them.
                let on_faces = !self.map_edits.panel.is_sky(&shader);
                match on_faces
                    .then(|| self.map_edit_face_preview(&shader))
                    .flatten()
                {
                    Some(face_shader) => {
                        if !froms.iter().any(|f| f.eq_ignore_ascii_case(&face_shader)) {
                            froms.push(face_shader);
                        }
                    }
                    None => self.map_edit_face_preview_off(),
                }
                (!froms.is_empty()).then_some((shader, froms))
            }
        };
        self.map_edits.panel.preview.shown = shown;
        let remaps = with_preview(
            &self.map_edits.remaps,
            self.map_edits.panel.preview.shown.as_ref(),
        );
        if let Some(vfs) = self.vfs.clone() {
            let _ = self.world_materials.set_edit_remaps(
                &vfs,
                &self.shaders,
                remaps.iter().map(|(a, b)| (a.as_str(), b.as_str())),
            );
        }
    }

    /// Put the map's remaps back when a preview is drawn (the panel closing).
    pub(crate) fn map_edit_preview_off(&mut self) {
        self.map_edit_face_preview_off();
        if self.map_edits.panel.preview.step(None, Instant::now()) == Step::Revert {
            self.map_edits.panel.preview.shown = None;
            if let Some(vfs) = self.vfs.clone() {
                let _ = self.world_materials.set_edit_remaps(
                    &vfs,
                    &self.shaders,
                    self.map_edits
                        .remaps
                        .iter()
                        .map(|(a, b)| (a.as_str(), b.as_str())),
                );
            }
        }
    }

    /// The shader the preview wants now: on Selected, the row or tile under the pointer,
    /// else the chosen one while the keyboard is in the list or its search; none on faces,
    /// on a sky unless it is a sky, or while the editor flies, grabs or transforms.
    fn map_edit_preview_wanted(&self) -> Option<String> {
        use super::panel::{Picked, REMAP_FIELD, SHADER_ROW, Tab};
        let panel = &self.map_edits.panel;
        if panel.tab != Tab::Selected
            || !self.map_edit_panel_open()
            || self.map_edits.gizmo.busy()
            || panel.facts.count == 0
        {
            return None;
        }
        let picked = panel.facts.selection.as_ref()?.picked;
        let in_list = panel.focus == REMAP_FIELD
            || (SHADER_ROW..SHADER_ROW + panel.shader_slots().0 as u16).contains(&panel.focus);
        let index = panel
            .hover_shader
            .or_else(|| in_list.then_some(panel.shader_row))?;
        let shader = panel.shown_shader(index)?;
        match picked {
            Picked::Surface { .. } => Some(shader.to_owned()),
            Picked::Sky if panel.is_sky(shader) => Some(shader.to_owned()),
            // On caulk and nodraw faces any texture (a face takes no sky).
            Picked::Face if !panel.is_sky(shader) => Some(shader.to_owned()),
            _ => None,
        }
    }

    /// The shaders the preview stands in for: the selected surfaces' (each once), or the
    /// selected sky's.
    fn map_edit_preview_froms(&self) -> Vec<String> {
        use crate::world_notes::EditTarget;
        let mut froms: Vec<String> = Vec::new();
        for selection in self.world_notes.edit_selection() {
            let shader = match selection.edit_target() {
                EditTarget::Surface { shader, .. } => shader,
                EditTarget::Face {
                    shader, sky: true, ..
                } => shader,
                _ => continue,
            };
            if !froms
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&shader))
            {
                froms.push(shader);
            }
        }
        froms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preview_waits_on_a_row_and_goes_back_off_the_list() {
        let mut preview = Preview::default();
        let start = Instant::now();
        // passing over a row draws nothing
        assert_eq!(preview.step(Some("textures/a/b"), start), Step::Keep);
        assert_eq!(
            preview.step(Some("textures/a/c"), start + SETTLE / 2),
            Step::Keep
        );
        // resting on one draws it once
        let later = start + SETTLE / 2 + SETTLE;
        assert_eq!(
            preview.step(Some("textures/a/c"), later),
            Step::Draw("textures/a/c".into())
        );
        preview.shown = Some(("textures/a/c".into(), vec!["textures/a/own".into()]));
        assert_eq!(
            preview.step(Some("textures/a/c"), later + SETTLE),
            Step::Keep
        );
        // off the list: the map's back at once, then nothing more
        assert_eq!(preview.step(None, later + SETTLE), Step::Revert);
        preview.shown = None;
        assert_eq!(preview.step(None, later + SETTLE), Step::Keep);
        // an edit applied drops it
        preview.shown = Some(("x".into(), Vec::new()));
        preview.dropped();
        assert_eq!(preview.step(None, later), Step::Keep);
    }

    #[test]
    fn the_preview_stands_over_the_documents_remaps_and_reverts_to_them() {
        let document = vec![
            ("textures/a/own".to_owned(), "textures/a/old".to_owned()),
            ("textures/a/other".to_owned(), "textures/a/kept".to_owned()),
        ];
        let preview = (
            "textures/a/new".to_owned(),
            vec!["TEXTURES/A/OWN".to_owned()],
        );
        let drawn = with_preview(&document, Some(&preview));
        assert_eq!(
            drawn,
            [
                ("textures/a/other".to_owned(), "textures/a/kept".to_owned()),
                ("TEXTURES/A/OWN".to_owned(), "textures/a/new".to_owned()),
            ]
        );
        assert_eq!(with_preview(&document, None), document);
    }
}
