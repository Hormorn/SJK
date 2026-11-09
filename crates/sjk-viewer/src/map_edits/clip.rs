//! Copy and paste in the in-game editor (the browser editor's, `docs/map-edits.md`,
//! "Copy and paste"): Ctrl+C copies the selection, a placed model with its angles and
//! scale, a prop's model, or a surface's or a face's shader (the remap's when one draws
//! it), as plain text (the model's or the shader's path) on the system clipboard. Ctrl+V
//! pastes a model as a placed copy where the pointer is (`mapedit_place <model> <pitch
//! yaw roll> <sx sy sz>`) or a shader as a remap, or a shown face, of the selection
//! (`mapedit_remap`). A path copied elsewhere, the page included, pastes the same way;
//! the angles and scale come along when the clipboard still holds this editor's copy.

use super::document::Kind;
use crate::GpuState;
use crate::world_notes::EditTarget;

/// Longest text taken as a path.
const LONGEST: usize = 160;

/// What a copy holds.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Clip {
    Model {
        model: String,
        angles: Option<[f32; 3]>,
        scale: Option<[f32; 3]>,
    },
    Shader(String),
}

impl Clip {
    /// Its plain text: the path.
    pub(crate) fn text(&self) -> &str {
        match self {
            Self::Model { model, .. } => model,
            Self::Shader(shader) => shader,
        }
    }
}

/// What `text` pasted is: this editor's own copy when it is its text, else a model path
/// (`.md3`) or a shader path (with a slash), as the page reads a text from elsewhere.
pub(crate) fn clip_of_text(text: &str, own: Option<&Clip>) -> Option<Clip> {
    let text = text.trim().replace('\\', "/");
    if text.is_empty() || text.len() > LONGEST || text.chars().any(char::is_whitespace) {
        return None;
    }
    if let Some(own) = own.filter(|own| own.text() == text) {
        return Some(own.clone());
    }
    if text.to_ascii_lowercase().ends_with(".md3") {
        return Some(Clip::Model {
            model: text,
            angles: None,
            scale: None,
        });
    }
    text.contains('/').then_some(Clip::Shader(text))
}

/// The command a pasted model runs: `mapedit_place` with its angles and scale when known.
pub(crate) fn place_args(
    model: &str,
    angles: Option<[f32; 3]>,
    scale: Option<[f32; 3]>,
) -> Vec<String> {
    let mut args = vec![model.to_owned()];
    if let Some(angles) = angles {
        args.extend(angles.iter().map(f32::to_string));
        if let Some(scale) = scale {
            args.extend(scale.iter().map(f32::to_string));
        }
    }
    args
}

impl GpuState {
    /// What Ctrl+C copies of the selection.
    fn map_edit_clip_of_selection(&self) -> Option<Clip> {
        let selection = self.world_notes.waiting_selection()?;
        let server = self.map_edits_server();
        let document = self.map_edits.document.as_ref();
        match selection.edit_target() {
            EditTarget::Model { number, model, .. } => {
                let lump = sjk_entity::parse_entity_lump(self.bsp.entities())
                    .map_or(0, |entities| entities.len());
                if let Some(id) = self.map_edits.placed_edit(number, lump)
                    && let Some(placed) = self.map_edits.placements(&[id]).first()
                {
                    return Some(Clip::Model {
                        model,
                        angles: Some(placed.angles),
                        scale: Some(placed.scale),
                    });
                }
                model
                    .to_ascii_lowercase()
                    .ends_with(".md3")
                    .then_some(Clip::Model {
                        model,
                        angles: None,
                        scale: None,
                    })
            }
            EditTarget::Surface { shader, .. } => {
                // The shader drawn there: a remap edit's, else its own.
                let remapped = document.and_then(|document| {
                    document
                        .active(server.as_deref())
                        .find_map(|edit| match &edit.kind {
                            Kind::Remap { from, to } if from.eq_ignore_ascii_case(&shader) => {
                                Some(to.clone())
                            }
                            _ => None,
                        })
                });
                Some(Clip::Shader(remapped.unwrap_or(shader)))
            }
            EditTarget::Face {
                brush,
                side,
                shader,
                ..
            } => {
                let drawn = document.and_then(|document| {
                    document
                        .active(server.as_deref())
                        .find_map(|edit| match &edit.kind {
                            Kind::ShowFace {
                                brush: b,
                                side: s,
                                to,
                                ..
                            } if *b == brush && *s == side => Some(to.clone()),
                            _ => None,
                        })
                });
                Some(Clip::Shader(drawn.unwrap_or(shader)))
            }
            EditTarget::Other => None,
        }
    }

    /// Ctrl+C: copy the selection.
    pub(super) fn map_edit_copy(&mut self) -> Result<Vec<String>, String> {
        let clip = self
            .map_edit_clip_of_selection()
            .ok_or("Nothing selected to copy: select a model, a surface or a face")?;
        let copied = crate::console::clipboard::copy(clip.text());
        let line = match &clip {
            Clip::Model { model, angles, .. } if angles.is_some() => {
                format!("Copied {model} with its turn and scale: Ctrl+V places a copy")
            }
            Clip::Model { model, .. } => format!("Copied {model}: Ctrl+V places a copy"),
            Clip::Shader(shader) => format!("Copied {shader}: Ctrl+V draws it on a selection"),
        };
        self.map_edits.panel.clip = Some(clip);
        if copied {
            Ok(vec![line])
        } else {
            Ok(vec![format!(
                "{line} (this game only: the clipboard refused)"
            )])
        }
    }

    /// Ctrl+V: paste the clipboard's model where the pointer is, or its shader on the
    /// selection.
    pub(super) fn map_edit_paste(&mut self) -> Result<Vec<String>, String> {
        let own = self.map_edits.panel.clip.clone();
        let clip = crate::console::clipboard::paste()
            .and_then(|text| clip_of_text(&text, own.as_ref()))
            .or(own)
            .ok_or("Nothing to paste: copy a model or a shader path, or a selection here")?;
        match clip {
            Clip::Model {
                model,
                angles,
                scale,
            } => {
                // Where the pointer is, as the page's paste; off the scene, the view's centre.
                let point = self.cursor_position;
                let on_scene = point[0] < self.map_edits.panel.scene_right;
                let picked = if on_scene {
                    self.map_edit_pick_at(point)
                } else {
                    self.map_edit_pick().is_some()
                };
                if !picked {
                    return Err("Point at the floor or a wall where the copy goes".into());
                }
                self.map_edit_command("mapedit_place", &place_args(&model, angles, scale))
            }
            Clip::Shader(shader) => {
                if self.world_notes.waiting_selection().is_none() {
                    return Err(format!("Select a surface or a face to paste {shader} on"));
                }
                self.map_edit_command("mapedit_remap", &[shader])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A text from elsewhere is a model or a shader path; this editor's own copy keeps
    /// its angles and scale; anything else pastes nothing.
    #[test]
    fn a_pasted_text_is_read_as_the_page_reads_it() {
        assert_eq!(
            clip_of_text(" models\\map_objects\\crate.MD3\n", None),
            Some(Clip::Model {
                model: "models/map_objects/crate.MD3".into(),
                angles: None,
                scale: None,
            })
        );
        assert_eq!(
            clip_of_text("textures/yavin/floor", None),
            Some(Clip::Shader("textures/yavin/floor".into()))
        );
        for nothing in ["", "hello", "two words/x", &"a/".repeat(100)] {
            assert_eq!(clip_of_text(nothing, None), None, "{nothing}");
        }
        let own = Clip::Model {
            model: "models/a.md3".into(),
            angles: Some([0.0, 90.0, 0.0]),
            scale: Some([2.0, 2.0, 1.0]),
        };
        assert_eq!(clip_of_text("models/a.md3", Some(&own)), Some(own.clone()));
        assert_eq!(
            place_args(
                "models/a.md3",
                Some([0.0, 90.0, 0.0]),
                Some([2.0, 2.0, 1.0])
            ),
            ["models/a.md3", "0", "90", "0", "2", "2", "1"]
        );
        assert_eq!(place_args("models/a.md3", None, None), ["models/a.md3"]);
    }
}
