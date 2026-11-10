//! Measuring (Sol, 10/11/2026: "continue, do the good stuff"): M (or `mapedit_measure`)
//! starts the measure tool; two clicks on the scene put its ends on what they meet, and
//! the panel draws the line between them with its length in units and the axis deltas
//! (`128.0 u  dx 128 dy 0 dz 0`). A third click starts a new one; M or Escape stops.
//! For a sense of scale: a player stands 64 units tall, a jump rises about 48.

use crate::GpuState;
use glam::Vec3;

/// The measure tool's state, on the panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Measure {
    /// The tool is on: a click on the scene puts an end.
    pub(crate) on: bool,
    /// Its ends so far (none, one, or both).
    pub(crate) ends: Vec<Vec3>,
    /// Where its label shows on the screen this frame, and what it says.
    pub(crate) label: Option<([f32; 2], String)>,
}

impl Measure {
    /// Put an end at `point`: the first, the second, or a new first after two.
    pub(crate) fn put(&mut self, point: Vec3) {
        if self.ends.len() >= 2 {
            self.ends.clear();
        }
        self.ends.push(point);
    }

    /// The distance between the ends and the deltas along each axis, once both are put.
    pub(crate) fn reading(&self) -> Option<(f32, Vec3)> {
        let [from, to] = self.ends.as_slice() else {
            return None;
        };
        let delta = *to - *from;
        Some((delta.length(), delta))
    }
}

/// How a measure reads: `128.0 u  dx 128 dy 0 dz 0`.
pub(crate) fn words(distance: f32, delta: Vec3) -> String {
    format!(
        "{distance:.1} u  dx {:.0} dy {:.0} dz {:.0}",
        delta.x, delta.y, delta.z
    )
}

impl GpuState {
    /// `mapedit_measure [on|off|clear|<x y z>]`: start or stop the tool (alone: the other
    /// way), forget its ends, or put an end at a point (a click's).
    pub(super) fn map_edit_measure(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let measure = &mut self.map_edits.panel.measure;
        let switched = match args {
            [] => Some(!measure.on),
            [word] if word == "on" || word == "off" => Some(word == "on"),
            _ => None,
        };
        let line = match args {
            _ if switched.is_some() => {
                measure.on = switched.unwrap_or(false);
                if !measure.on {
                    measure.ends.clear();
                    measure.label = None;
                    "Measure off".to_owned()
                } else {
                    "Measure: click two points on the scene (M or Esc stops)".to_owned()
                }
            }
            [word] if word == "clear" => {
                measure.ends.clear();
                measure.label = None;
                "Measure cleared".to_owned()
            }
            [x, y, z] => {
                let number = |text: &String| {
                    text.parse::<f32>()
                        .ok()
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| format!("not a number: {text}"))
                };
                let point = Vec3::new(number(x)?, number(y)?, number(z)?);
                measure.on = true;
                measure.put(point);
                match measure.reading() {
                    Some((distance, delta)) => format!("Measure: {}", words(distance, delta)),
                    None => format!(
                        "Measure from {:.0} {:.0} {:.0}: click the other end",
                        point.x, point.y, point.z
                    ),
                }
            }
            _ => return Err("usage: mapedit_measure [on|off|clear|<x y z>]".into()),
        };
        Ok(vec![line])
    }

    /// A click on the scene while measuring: an end where its ray meets the world (a
    /// drawn surface, else a caulk or sky face).
    pub(super) fn map_edit_measure_at(&mut self, point: [f32; 2]) {
        let Some((eye, direction)) = self.world_notes.ray_at(point) else {
            return;
        };
        let reach = 16_384.0;
        let surface = self
            .decal_surfaces
            .ray_hit(eye, direction, reach)
            .map(|hit| hit.distance);
        let face = super::faces::pick(&self.bsp, eye, direction, reach).map(|face| face.distance);
        let distance = match (surface, face) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let result = match distance {
            Some(distance) => {
                let at = eye + direction * distance;
                let words: Vec<String> = [at.x, at.y, at.z]
                    .iter()
                    .map(|value| format!("{value:.2}"))
                    .collect();
                self.map_edit_command("mapedit_measure", &words)
            }
            None => Err("Nothing there to measure to".into()),
        };
        self.map_edits.panel.answer(result);
    }

    /// Draw the measure into `list` (its line and ends) and keep where its label shows.
    pub(super) fn map_edit_measure_draw(
        &mut self,
        list: &mut sjk_ui::DrawList,
        view: &super::gizmo::View,
    ) {
        let measure = &mut self.map_edits.panel.measure;
        measure.label = None;
        if !measure.on || measure.ends.is_empty() {
            return;
        }
        let gold = sjk_ui::Color::new(1.0, 0.82, 0.3, 1.0);
        let dark = sjk_ui::Color::new(0.02, 0.03, 0.06, 0.85);
        let points: Vec<Option<glam::Vec2>> =
            measure.ends.iter().map(|end| view.project(*end)).collect();
        if let [Some(a), Some(b)] = points.as_slice() {
            for (width, colour) in [(4.0, dark), (2.0, gold)] {
                let _ = list.push(sjk_ui::DrawCommand::Line {
                    from: a.to_array(),
                    to: b.to_array(),
                    width,
                    color: colour,
                });
            }
        }
        for at in points.iter().flatten() {
            for (radius, colour) in [(6.0, dark), (4.0, gold)] {
                let _ = list.push(sjk_ui::DrawCommand::RoundedRect {
                    rect: sjk_ui::Rect::new(
                        at.x - radius,
                        at.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    ),
                    radius,
                    color: colour,
                });
            }
        }
        if let (Some((distance, delta)), Some(at)) = (
            measure.reading(),
            view.project((measure.ends[0] + measure.ends[1]) * 0.5),
        ) {
            measure.label = Some((at.to_array(), words(distance, delta)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two ends read as their distance and deltas; a third click starts a new measure.
    #[test]
    fn two_ends_read_their_distance_and_a_third_starts_again() {
        let mut measure = Measure::default();
        measure.put(Vec3::new(0.0, 0.0, 0.0));
        assert!(measure.reading().is_none());
        measure.put(Vec3::new(30.0, 40.0, 0.0));
        let (distance, delta) = measure.reading().unwrap();
        assert!((distance - 50.0).abs() < 1e-4);
        assert_eq!(words(distance, delta), "50.0 u  dx 30 dy 40 dz 0");
        measure.put(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(measure.ends, vec![Vec3::new(1.0, 2.0, 3.0)]);
        assert!(measure.reading().is_none());
        assert_eq!(
            words(64.0, Vec3::new(0.0, 0.0, -64.0)),
            "64.0 u  dx 0 dy 0 dz -64"
        );
    }
}
