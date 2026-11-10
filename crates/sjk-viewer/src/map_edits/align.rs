//! Texture alignment of shown faces (Sol, 10/11/2026: "continue, do the good stuff"):
//! `mapedit_align` shifts, scales, turns, fits or resets the texture of the selected
//! caulk, nodraw or sky faces an edit draws, as Radiant's surface inspector does a brush
//! face's ([`super::document::Align`], [`super::faces::texture_coordinates`]). Every
//! selected drawn face changes in one step to undo; the faces' meshes are written again
//! in place, no new mesh is made.
//!
//! The alignment is optional fields of the `show_face` edit: a client from before it
//! ignores them and draws the face at Radiant's default.

use super::document::{ALIGN_ROTATE, ALIGN_SCALE, ALIGN_SHIFT, Align, Kind};
use crate::GpuState;

/// What the panel's steps change by: a shift in texels, a turn in degrees, a growth;
/// Shift held makes them fine (a texel, a degree, a tenth of the growth).
pub(crate) const SHIFT_STEP: f32 = 8.0;
pub(crate) const TURN_STEP: f32 = 15.0;
pub(crate) const SCALE_STEP: f32 = 1.25;

/// One change of `mapedit_align`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Step {
    /// Shift by texels along the texture's width and height.
    Shift([f32; 2]),
    /// Multiply each scale (a negative factor flips).
    Scale([f32; 2]),
    /// Turn by degrees.
    Rotate(f32),
    /// Lay the image once across the face ([`super::faces::fit`]).
    Fit,
    /// Back to Radiant's default.
    Reset,
    /// Exactly this.
    Set(Align),
}

const USAGE: &str = "usage: mapedit_align shift <s t> | scale <s t> | rotate <degrees> | fit | \
                     reset | set <shift s t> <scale s t> <degrees>";

/// `mapedit_align`'s words as a step.
pub(crate) fn parse(args: &[String]) -> Result<Step, String> {
    let numbers = |words: &[String]| -> Result<Vec<f32>, String> {
        words
            .iter()
            .map(|word| {
                word.parse::<f32>()
                    .ok()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| format!("not a number: {word}"))
            })
            .collect()
    };
    let Some((what, rest)) = args.split_first() else {
        return Err(USAGE.into());
    };
    let values = numbers(rest)?;
    match (what.to_ascii_lowercase().as_str(), values.as_slice()) {
        ("shift", &[s, t]) => Ok(Step::Shift([s, t])),
        ("scale", &[s, t]) if s != 0.0 && t != 0.0 => Ok(Step::Scale([s, t])),
        ("scale", &[both]) if both != 0.0 => Ok(Step::Scale([both; 2])),
        ("scale", _) => Err("the scale's factors are not 0 (a negative one flips)".into()),
        ("rotate", &[degrees]) => Ok(Step::Rotate(degrees)),
        ("fit", []) => Ok(Step::Fit),
        ("reset", []) => Ok(Step::Reset),
        ("set", &[shift_s, shift_t, scale_s, scale_t, rotate]) => {
            let align = Align {
                shift: [shift_s, shift_t],
                scale: [scale_s, scale_t],
                rotate,
            };
            if align.valid() {
                Ok(Step::Set(align))
            } else {
                Err(format!(
                    "out of bounds: the scale's size {} to {}, the shift within {ALIGN_SHIFT}",
                    ALIGN_SCALE.start(),
                    ALIGN_SCALE.end()
                ))
            }
        }
        _ => Err(USAGE.into()),
    }
}

/// `align` after `step`; `fitted` is what Fit gives this face.
pub(crate) fn stepped(align: &Align, step: Step, fitted: impl FnOnce() -> Align) -> Align {
    let mut next = *align;
    match step {
        Step::Shift(by) => {
            for (shift, by) in next.shift.iter_mut().zip(by) {
                *shift = (*shift + by).clamp(-ALIGN_SHIFT, ALIGN_SHIFT);
            }
        }
        Step::Scale(by) => {
            for (scale, by) in next.scale.iter_mut().zip(by) {
                let scaled = *scale * by;
                *scale =
                    scaled.signum() * scaled.abs().clamp(*ALIGN_SCALE.start(), *ALIGN_SCALE.end());
            }
        }
        Step::Rotate(by) => {
            // Kept within a turn either way.
            next.rotate = (next.rotate + by) % 360.0;
            next.rotate = next.rotate.clamp(-ALIGN_ROTATE, ALIGN_ROTATE);
        }
        Step::Fit => next = fitted(),
        Step::Reset => next = Align::default(),
        Step::Set(align) => next = align,
    }
    next
}

/// `step` made fine (Shift held on a button): a texel, a degree, a tenth of a growth; a
/// flip, Fit and Reset stay.
pub(crate) fn fine(step: Step) -> Step {
    match step {
        Step::Shift(by) => Step::Shift(by.map(|v| if v == 0.0 { 0.0 } else { v.signum() })),
        Step::Rotate(by) => Step::Rotate(by.signum()),
        Step::Scale(by) if by.iter().all(|factor| *factor > 0.0) => {
            Step::Scale(by.map(|factor| factor.powf(0.1)))
        }
        step => step,
    }
}

/// The command's words for `step`.
pub(crate) fn words(step: Step) -> Vec<String> {
    // The shortest form that reads back as the same number.
    let text = |value: f32| format!("{value}");
    match step {
        Step::Shift([s, t]) => vec!["shift".into(), text(s), text(t)],
        Step::Scale([s, t]) => vec!["scale".into(), text(s), text(t)],
        Step::Rotate(degrees) => vec!["rotate".into(), text(degrees)],
        Step::Fit => vec!["fit".into()],
        Step::Reset => vec!["reset".into()],
        Step::Set(align) => vec![
            "set".into(),
            text(align.shift[0]),
            text(align.shift[1]),
            text(align.scale[0]),
            text(align.scale[1]),
            text(align.rotate),
        ],
    }
}

impl GpuState {
    /// `mapedit_align`: the step on every selected face an edit of this scope draws, one
    /// step to undo.
    pub(super) fn map_edit_align(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let step = parse(args)?;
        let server = self.map_edit_server()?;
        let faces = self.map_edit_selected_faces();
        if faces.is_empty() {
            return Err("Select a face an edit draws (Draw a shader on a caulk face first)".into());
        }
        let vfs = self.vfs.clone();
        let shaders = self.shaders.clone();
        let bsp = &self.bsp;
        let document = self
            .map_edits
            .document
            .as_mut()
            .ok_or("No face of the selection is drawn yet: Draw a shader on it first")?;
        let mut changed = 0;
        let mut last = None;
        for edit in &mut document.edits {
            if edit.server != server {
                continue;
            }
            let Kind::ShowFace {
                brush,
                side,
                shader,
                to,
                align,
            } = &mut edit.kind
            else {
                continue;
            };
            if !faces.contains(&[*brush, *side]) {
                continue;
            }
            let fitted = || {
                let size = vfs
                    .as_deref()
                    .and_then(|vfs| super::texture_size(vfs, &shaders, to));
                super::faces::polygon(bsp, *brush, *side, shader)
                    .map_or(*align, |(polygon, normal)| {
                        super::faces::fit(&polygon, normal, size, align)
                    })
            };
            let next = stepped(align, step, fitted);
            if next.valid() && next != *align {
                *align = next;
                changed += 1;
            }
            last = Some(*align);
        }
        let Some(last) = last else {
            return Err(
                "No face of the selection is drawn for these servers: Draw a shader on it first"
                    .into(),
            );
        };
        if changed == 0 {
            return Ok(vec![format!("The texture stays: {}", last.describe())]);
        }
        document.unpublished = true;
        let line = if changed == 1 {
            format!("Texture: {}", last.describe())
        } else {
            format!("{changed} faces' textures: {}", last.describe())
        };
        self.map_edit_saved(line)
    }

    /// The alignment of the newest selected face an edit of this scope draws, for the
    /// panel; `None` when none is drawn.
    pub(crate) fn map_edit_selected_align(&self) -> Option<Align> {
        let server = self.map_edit_server().ok()?;
        let [brush, side] = *self.map_edit_selected_faces().first()?;
        let document = self.map_edits.document.as_ref()?;
        document.edits.iter().find_map(|edit| match &edit.kind {
            Kind::ShowFace {
                brush: b,
                side: s,
                align,
                ..
            } if edit.server == server && *b == brush && *s == side => Some(*align),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_command_reads_its_steps() {
        assert_eq!(parse(&words("shift 8 -8")), Ok(Step::Shift([8.0, -8.0])));
        assert_eq!(parse(&words("scale 2")), Ok(Step::Scale([2.0, 2.0])));
        assert_eq!(parse(&words("scale -1 1")), Ok(Step::Scale([-1.0, 1.0])));
        assert_eq!(parse(&words("rotate 15")), Ok(Step::Rotate(15.0)));
        assert_eq!(parse(&words("FIT")), Ok(Step::Fit));
        assert_eq!(parse(&words("reset")), Ok(Step::Reset));
        assert_eq!(
            parse(&words("set 1 2 0.25 0.5 90")),
            Ok(Step::Set(Align {
                shift: [1.0, 2.0],
                scale: [0.25, 0.5],
                rotate: 90.0
            }))
        );
        for bad in [
            "",
            "shift 8",
            "scale 0",
            "rotate x",
            "fit 2",
            "set 0 0 0 0.5 0",
            "spin 3",
        ] {
            assert!(parse(&words(bad)).is_err(), "{bad}");
        }
    }

    /// Every button's step reads back from its own words, and Shift makes it fine.
    #[test]
    fn the_buttons_steps_are_words_the_command_reads() {
        for (label, step) in crate::map_edits::panel::ALIGN_STEPS {
            assert_eq!(parse(&super::words(step)), Ok(step), "{label}");
            let fine = super::fine(step);
            assert_eq!(parse(&super::words(fine)), Ok(fine), "{label} fine");
        }
        assert_eq!(
            fine(Step::Shift([SHIFT_STEP, 0.0])),
            Step::Shift([1.0, 0.0])
        );
        assert_eq!(
            fine(Step::Shift([0.0, -SHIFT_STEP])),
            Step::Shift([0.0, -1.0])
        );
        assert_eq!(fine(Step::Rotate(-TURN_STEP)), Step::Rotate(-1.0));
        assert_eq!(fine(Step::Scale([-1.0, 1.0])), Step::Scale([-1.0, 1.0]));
        let Step::Scale([grow, _]) = fine(Step::Scale([SCALE_STEP; 2])) else {
            panic!("a scale");
        };
        assert!((grow - SCALE_STEP.powf(0.1)).abs() < 1e-6);
    }

    #[test]
    fn steps_add_up_within_the_limits() {
        let start = Align::default();
        let fit = || Align {
            scale: [2.0, 2.0],
            ..Align::default()
        };
        let shifted = stepped(&start, Step::Shift([8.0, 0.0]), fit);
        let shifted = stepped(&shifted, Step::Shift([8.0, -4.0]), fit);
        assert_eq!(shifted.shift, [16.0, -4.0]);
        let bigger = stepped(&start, Step::Scale([SCALE_STEP; 2]), fit);
        assert!((bigger.scale[0] - 0.625).abs() < 1e-6);
        let flipped = stepped(&start, Step::Scale([-1.0, 1.0]), fit);
        assert_eq!(flipped.scale, [-0.5, 0.5]);
        let tiny = stepped(&start, Step::Scale([1e-6, 1.0]), fit);
        assert_eq!(tiny.scale[0], *ALIGN_SCALE.start());
        let mut turned = start;
        for _ in 0..30 {
            turned = stepped(&turned, Step::Rotate(TURN_STEP), fit);
        }
        assert_eq!(turned.rotate, 90.0, "450 degrees is a quarter turn");
        assert_eq!(stepped(&start, Step::Fit, fit), fit());
        assert_eq!(stepped(&shifted, Step::Reset, fit), Align::default());
        assert!(turned.valid() && tiny.valid() && flipped.valid());
    }
}
