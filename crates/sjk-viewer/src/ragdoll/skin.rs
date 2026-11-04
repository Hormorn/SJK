//! The Ragdoll toy's body on a player model: the stock `_humanoid` skeleton's bones
//! posed from the simulated points ([`super::body`]) in place of the animation.
//!
//! The skeleton is cut into eleven rigid parts (pelvis, chest, head, upper arms,
//! forearms, thighs, shins), each led by one bone ([`PARTS`]); every other bone goes with
//! the part of its nearest leading ancestor (a hand with its forearm, a foot with its
//! shin, the face with the head). A part turns as the points it spans do: its own line
//! (the thigh from hip to knee) and a second direction that sets its twist (the trunk's
//! left, or its front, carried down the limb). Each part hangs from its parent at the
//! model's own joint, so a model of any proportions stays in one piece; only the
//! pelvis stands where the simulation puts it.
//!
//! The matrices are skinning matrices in the GLA's model space (+X left, -Y front, +Z
//! up), for an actor placed at the body's pelvis without a turn ([`model_space`]).

use super::body::{
    CHEST, HEAD, JOINTS, L_ELBOW, L_FOOT, L_HAND, L_HIP, L_KNEE, L_SHOULDER, PELVIS, R_ELBOW,
    R_FOOT, R_HAND, R_HIP, R_KNEE, R_SHOULDER,
};
use glam::{Affine3A, Mat3A, Quat, Vec3, Vec3A};
use sjk_model::Gla;
use std::sync::Arc;

/// How a part sets its twist: by the trunk's left (from the right point to the left
/// one, `side`), by the front of a parent part, or by its parent's twist carried down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Twist {
    /// (right point, left point) of the simulation, and the bones of the model.
    Side(usize, usize, &'static str, &'static str),
    /// The front of a part, as the trunk faces.
    Front(usize),
    /// The parent's front, carried.
    Carried,
}

/// One rigid part: the bone that leads it, its parent part, the simulated points its
/// line runs between and the bones of the model they stand for (the end bone given as a
/// pair to take the middle of), and its twist.
struct Part {
    bone: &'static str,
    parent: Option<usize>,
    from: (usize, Option<usize>),
    to: (usize, Option<usize>),
    from_bones: (&'static str, Option<&'static str>),
    to_bones: (&'static str, Option<&'static str>),
    twist: Twist,
}

const PELVIS_PART: usize = 0;
const CHEST_PART: usize = 1;
const L_UPPER: usize = 3;
const R_UPPER: usize = 5;
const L_THIGH: usize = 7;
const R_THIGH: usize = 9;

/// The parts, parents first.
const PARTS: [Part; 11] = [
    Part {
        bone: "pelvis",
        parent: None,
        from: (PELVIS, None),
        to: (CHEST, None),
        from_bones: ("pelvis", None),
        to_bones: ("thoracic", None),
        twist: Twist::Side(R_HIP, L_HIP, "rfemurYZ", "lfemurYZ"),
    },
    Part {
        bone: "lower_lumbar",
        parent: Some(PELVIS_PART),
        from: (PELVIS, None),
        to: (L_SHOULDER, Some(R_SHOULDER)),
        from_bones: ("pelvis", None),
        to_bones: ("lhumerus", Some("rhumerus")),
        twist: Twist::Side(R_SHOULDER, L_SHOULDER, "rhumerus", "lhumerus"),
    },
    Part {
        bone: "cervical",
        parent: Some(CHEST_PART),
        from: (CHEST, None),
        to: (HEAD, None),
        from_bones: ("thoracic", None),
        to_bones: ("cranium", None),
        twist: Twist::Side(R_SHOULDER, L_SHOULDER, "rhumerus", "lhumerus"),
    },
    Part {
        bone: "lhumerus",
        parent: Some(CHEST_PART),
        from: (L_SHOULDER, None),
        to: (L_ELBOW, None),
        from_bones: ("lhumerus", None),
        to_bones: ("lradius", None),
        twist: Twist::Front(CHEST_PART),
    },
    Part {
        bone: "lradius",
        parent: Some(L_UPPER),
        from: (L_ELBOW, None),
        to: (L_HAND, None),
        from_bones: ("lradius", None),
        to_bones: ("lhand", None),
        twist: Twist::Carried,
    },
    Part {
        bone: "rhumerus",
        parent: Some(CHEST_PART),
        from: (R_SHOULDER, None),
        to: (R_ELBOW, None),
        from_bones: ("rhumerus", None),
        to_bones: ("rradius", None),
        twist: Twist::Front(CHEST_PART),
    },
    Part {
        bone: "rradius",
        parent: Some(R_UPPER),
        from: (R_ELBOW, None),
        to: (R_HAND, None),
        from_bones: ("rradius", None),
        to_bones: ("rhand", None),
        twist: Twist::Carried,
    },
    Part {
        bone: "lfemurYZ",
        parent: Some(PELVIS_PART),
        from: (L_HIP, None),
        to: (L_KNEE, None),
        from_bones: ("lfemurYZ", None),
        to_bones: ("ltibia", None),
        twist: Twist::Front(PELVIS_PART),
    },
    Part {
        bone: "ltibia",
        parent: Some(L_THIGH),
        from: (L_KNEE, None),
        to: (L_FOOT, None),
        from_bones: ("ltibia", None),
        to_bones: ("ltalus", None),
        twist: Twist::Carried,
    },
    Part {
        bone: "rfemurYZ",
        parent: Some(PELVIS_PART),
        from: (R_HIP, None),
        to: (R_KNEE, None),
        from_bones: ("rfemurYZ", None),
        to_bones: ("rtibia", None),
        twist: Twist::Front(PELVIS_PART),
    },
    Part {
        bone: "rtibia",
        parent: Some(R_THIGH),
        from: (R_KNEE, None),
        to: (R_FOOT, None),
        from_bones: ("rtibia", None),
        to_bones: ("rtalus", None),
        twist: Twist::Carried,
    },
];

/// The model's front in its own space: `-Y`.
const MODEL_FRONT: Vec3A = Vec3A::NEG_Y;

/// What a skeleton needs to be posed, worked out once per skeleton.
#[derive(Debug)]
struct Rig {
    /// The part each bone goes with.
    part_of: Vec<u8>,
    /// Each part's joint in the bind pose (its leading bone), and its frame there.
    anchors: [Vec3A; PARTS.len()],
    frames: [Mat3A; PARTS.len()],
}

/// A model's posing, kept with its actor: the skeleton's rig (`None` when it lacks the
/// stock bones) and the matrices of the last pose.
#[derive(Debug, Default)]
pub(crate) struct Scratch {
    rig: Option<(usize, Option<Rig>)>,
    pub(crate) matrices: Vec<[[f32; 4]; 3]>,
    /// The actor was posed so this frame: it is to be placed at the ragdoll too.
    pub(crate) posed: bool,
}

fn translation(matrix: &[[f32; 4]; 3]) -> Vec3A {
    Vec3A::new(matrix[0][3], matrix[1][3], matrix[2][3])
}

/// `a` with `b`'s direction taken out, normalised; `None` when nothing is left.
fn across(a: Vec3A, b: Vec3A) -> Option<Vec3A> {
    let left = a - b * a.dot(b);
    (left.length_squared() > 1e-6).then(|| left.normalize())
}

/// The frame of a line (`dir`, its first axis) twisted by `twist` (its third, made
/// square to the line); `None` for a line of no length.
fn frame(dir: Vec3A, twist: Vec3A) -> Option<Mat3A> {
    if dir.length_squared() < 1e-6 {
        return None;
    }
    let x = dir.normalize();
    let z = across(twist, x)
        .or_else(|| across(Vec3A::Z, x))
        .or_else(|| across(Vec3A::X, x))?;
    Some(Mat3A::from_cols(x, z.cross(x), z))
}

impl Rig {
    fn of(gla: &Gla) -> Option<Self> {
        let bone = |name: &str| {
            gla.bones
                .iter()
                .position(|bone| bone.name.eq_ignore_ascii_case(name))
        };
        let at = |name: &str| Some(translation(&gla.bones[bone(name)?].base_pose));
        let between = |(a, b): (&str, Option<&str>)| match b {
            None => at(a),
            Some(b) => Some((at(a)? + at(b)?) * 0.5),
        };
        let leaders: Vec<usize> = PARTS
            .iter()
            .map(|part| bone(part.bone))
            .collect::<Option<_>>()?;
        let mut anchors = [Vec3A::ZERO; PARTS.len()];
        let mut frames = [Mat3A::IDENTITY; PARTS.len()];
        for (index, part) in PARTS.iter().enumerate() {
            anchors[index] = translation(&gla.bones[leaders[index]].base_pose);
            let dir = between(part.to_bones)? - between(part.from_bones)?;
            let twist = match part.twist {
                Twist::Side(_, _, right, left) => at(left)? - at(right)?,
                Twist::Front(_) | Twist::Carried => MODEL_FRONT,
            };
            frames[index] = frame(dir, twist)?;
        }
        // Each bone goes with its nearest leading ancestor (itself included); a bone
        // above them all (the root) with the pelvis.
        let part_of = (0..gla.bones.len())
            .map(|mut index| {
                loop {
                    if let Some(part) = leaders.iter().position(|&leader| leader == index) {
                        break part as u8;
                    }
                    match gla.bones[index].parent {
                        Some(parent) => index = parent,
                        None => break PELVIS_PART as u8,
                    }
                }
            })
            .collect();
        Some(Self {
            part_of,
            anchors,
            frames,
        })
    }
}

/// `points` (world space) in the model space of an actor drawn at the pelvis without a
/// turn: the game draws a model turned a quarter (`weapon_view::actor_world_rotation`).
pub(crate) fn model_space(points: &[Vec3; JOINTS]) -> [Vec3A; JOINTS] {
    let back = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    let pelvis = points[PELVIS];
    points.map(|point| Vec3A::from(back * (point - pelvis)))
}

impl Scratch {
    /// Pose `gla`'s bones from `points` (model space, [`model_space`]) into
    /// [`Scratch::matrices`]; false when the skeleton is not the stock humanoid's.
    pub(crate) fn pose(&mut self, gla: &Arc<Gla>, points: &[Vec3A; JOINTS]) -> bool {
        let key = Arc::as_ptr(gla) as usize;
        if self.rig.as_ref().is_none_or(|(kept, _)| *kept != key) {
            self.rig = Some((key, Rig::of(gla)));
        }
        let Some((_, Some(rig))) = &self.rig else {
            return false;
        };
        let point = |(a, b): (usize, Option<usize>)| match b {
            None => points[a],
            Some(b) => (points[a] + points[b]) * 0.5,
        };
        let mut placed = [Affine3A::IDENTITY; PARTS.len()];
        let mut fronts = [MODEL_FRONT; PARTS.len()];
        for (index, part) in PARTS.iter().enumerate() {
            let dir = point(part.to) - point(part.from);
            let twist = match part.twist {
                Twist::Side(right, left, _, _) => points[left] - points[right],
                Twist::Front(of) => fronts[of],
                Twist::Carried => part.parent.map_or(MODEL_FRONT, |parent| fronts[parent]),
            };
            let bind = rig.frames[index];
            let now = frame(dir, twist).unwrap_or(bind);
            let turn = now * bind.transpose();
            // Where the part's joint goes: the pelvis where the body is, every other
            // part's on its parent as posed.
            let joint = match part.parent {
                None => points[PELVIS],
                Some(parent) => placed[parent].transform_point3a(rig.anchors[index]),
            };
            placed[index] = Affine3A {
                matrix3: turn,
                translation: joint - turn * rig.anchors[index],
            };
            // The part's front: the bind pose's front turned as the part is.
            fronts[index] = match part.twist {
                Twist::Side(..) => (turn * MODEL_FRONT).normalize_or_zero(),
                _ => turn * MODEL_FRONT,
            };
        }
        self.matrices.clear();
        self.matrices.extend(rig.part_of.iter().map(|&part| {
            let affine = placed[usize::from(part)];
            let m = affine.matrix3;
            let t = affine.translation;
            [
                [m.x_axis.x, m.y_axis.x, m.z_axis.x, t.x],
                [m.x_axis.y, m.y_axis.y, m.z_axis.y, t.y],
                [m.x_axis.z, m.y_axis.z, m.z_axis.z, t.z],
            ]
        }));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ragdoll::body::{Body, Start};

    /// A skeleton with the stock bones at the stock `_humanoid` bind pose's joints
    /// (`base_pose` translations only; the turns do not matter here), plus a hand and a
    /// foot under their limbs and a root above all.
    fn humanoid() -> Gla {
        let joints: [(&str, Option<&str>, [f32; 3]); 18] = [
            ("model_root", None, [0.0, 0.0, 0.0]),
            ("pelvis", Some("model_root"), [0.0, 0.4, 35.6]),
            ("lfemurYZ", Some("pelvis"), [3.6, -0.2, 35.6]),
            ("ltibia", Some("lfemurYZ"), [7.4, 0.8, 21.4]),
            ("ltalus", Some("ltibia"), [11.2, 1.8, 7.5]),
            ("rfemurYZ", Some("pelvis"), [-3.6, -0.2, 35.6]),
            ("rtibia", Some("rfemurYZ"), [-7.4, 0.8, 21.4]),
            ("rtalus", Some("rtibia"), [-11.2, 1.8, 7.5]),
            ("lower_lumbar", Some("pelvis"), [0.0, 0.4, 41.3]),
            ("thoracic", Some("lower_lumbar"), [0.0, 0.4, 51.6]),
            ("cervical", Some("thoracic"), [0.0, 0.4, 56.7]),
            ("cranium", Some("cervical"), [0.0, 0.4, 59.6]),
            ("lhumerus", Some("thoracic"), [5.9, 1.1, 54.5]),
            ("lradius", Some("lhumerus"), [13.2, 0.9, 46.4]),
            ("lhand", Some("lradius"), [19.0, 0.3, 39.9]),
            ("rhumerus", Some("thoracic"), [-5.9, 1.1, 54.5]),
            ("rradius", Some("rhumerus"), [-13.2, 0.9, 46.4]),
            ("rhand", Some("rradius"), [-19.0, 0.3, 39.9]),
        ];
        let index = |name: &str| joints.iter().position(|(n, _, _)| *n == name);
        let bones = joints
            .iter()
            .map(|&(name, parent, [x, y, z])| sjk_model::GlaBone {
                name: name.to_owned(),
                flags: 0,
                parent: parent.and_then(index),
                base_pose: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]],
                inverse_base_pose: [
                    [1.0, 0.0, 0.0, -x],
                    [0.0, 1.0, 0.0, -y],
                    [0.0, 0.0, 1.0, -z],
                ],
                children: Vec::new(),
            })
            .collect();
        Gla {
            name: "test".to_owned(),
            scale: 1.0,
            bones,
            frames: Vec::new(),
            compressed_bones: Vec::new(),
        }
    }

    fn apply(matrix: &[[f32; 4]; 3], point: Vec3A) -> Vec3A {
        let row = |r: &[f32; 4]| r[0] * point.x + r[1] * point.y + r[2] * point.z + r[3];
        Vec3A::new(row(&matrix[0]), row(&matrix[1]), row(&matrix[2]))
    }

    fn bind(gla: &Gla, name: &str) -> (usize, Vec3A) {
        let index = gla.bones.iter().position(|bone| bone.name == name).unwrap();
        (index, translation(&gla.bones[index].base_pose))
    }

    #[test]
    fn a_standing_body_poses_the_model_close_to_its_bind_pose_but_the_arms() {
        let gla = Arc::new(humanoid());
        let body = Body::new(&Start {
            moment: 0,
            origin: [100, 200, 24],
            velocity: [0, 0, 0],
            yaw: 0,
            scale: crate::ragdoll::STOCK_SCALE,
        });
        let mut scratch = Scratch::default();
        assert!(scratch.pose(&gla, &model_space(body.points())));
        assert_eq!(scratch.matrices.len(), gla.bones.len());
        // The legs and the head stand as tall as the model does (the knees bent a
        // little, the legs closer than its stance), about the body's pelvis, the origin
        // of the actor's model space.
        let (_, pelvis) = bind(&gla, "pelvis");
        for name in ["lfemurYZ", "ltibia", "ltalus", "rtalus", "cranium"] {
            let (index, at) = bind(&gla, name);
            let posed = apply(&scratch.matrices[index], at);
            assert!((posed.z - (at.z - pelvis.z)).abs() < 2.0, "{name} {posed}");
            assert!((posed - (at - pelvis)).length() < 8.0, "{name} {posed}");
        }
        // The hands hang down by the hips, not out in the bind pose's A.
        let (index, at) = bind(&gla, "lhand");
        let posed = apply(&scratch.matrices[index], at);
        assert!(posed.x > 4.0 && posed.x < 12.0 && posed.z < 0.0, "{posed}");
    }

    #[test]
    fn a_body_lying_down_lays_the_model_down_in_one_piece() {
        let gla = Arc::new(humanoid());
        let mut body = Body::new(&Start {
            moment: 9,
            origin: [0, 0, 24],
            velocity: [0, 0, 0],
            yaw: 45,
            scale: crate::ragdoll::STOCK_SCALE,
        });
        let mut floor = |from: Vec3, to: Vec3, half: f32| -> Option<(Vec3, Vec3)> {
            if to.z >= half {
                return None;
            }
            let t = if from.z > half {
                (from.z - half) / (from.z - to.z)
            } else {
                0.0
            };
            let mut at = from + (to - from) * t;
            at.z = at.z.max(half);
            Some((at, Vec3::Z))
        };
        body.run_to(crate::ragdoll::body::MAX_STEPS, &mut floor);
        let mut scratch = Scratch::default();
        let local = model_space(body.points());
        assert!(scratch.pose(&gla, &local));
        // Every limb's far joint is where its part, hung from its parent, takes it: the
        // model holds together (a child's joint on its parent's), and lies low.
        for (child, parent) in [
            ("ltibia", "lfemurYZ"),
            ("lradius", "lhumerus"),
            ("cervical", "lower_lumbar"),
        ] {
            let (c, at) = bind(&gla, child);
            let (p, _) = bind(&gla, parent);
            let by_child = apply(&scratch.matrices[c], at);
            let by_parent = apply(&scratch.matrices[p], at);
            assert!((by_child - by_parent).length() < 1e-3, "{child}");
        }
        let (head, at) = bind(&gla, "cranium");
        let posed = apply(&scratch.matrices[head], at);
        // Model space is the pelvis's: the head is no higher than a lying body's.
        assert!(posed.z < 12.0, "{posed}");
        // The matrices turn without stretching.
        for matrix in &scratch.matrices {
            let column = |c: usize| Vec3A::new(matrix[0][c], matrix[1][c], matrix[2][c]);
            for c in 0..3 {
                assert!((column(c).length() - 1.0).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn a_skeleton_without_the_stock_bones_is_not_posed() {
        let mut gla = humanoid();
        gla.bones.retain(|bone| bone.name != "ltibia");
        let body = Body::new(&Start {
            moment: 0,
            origin: [0, 0, 24],
            velocity: [0, 0, 0],
            yaw: 0,
            scale: crate::ragdoll::STOCK_SCALE,
        });
        let mut scratch = Scratch::default();
        assert!(!scratch.pose(&Arc::new(gla), &model_space(body.points())));
    }

    /// A body at a player's `modelScale` (hub report #223) turns the model's parts as a
    /// stock-sized one does: the pose hangs every part from the pelvis at the model's
    /// own joints, so the actor's scale alone sizes the model to the body.
    #[test]
    fn a_scaled_body_poses_the_model_as_a_stock_one_does() {
        let gla = Arc::new(humanoid());
        let start = Start {
            moment: 3_000,
            origin: [0, 0, 24],
            velocity: [0, 0, 0],
            yaw: 45,
            scale: crate::ragdoll::STOCK_SCALE,
        };
        let mut stock = Scratch::default();
        assert!(stock.pose(&gla, &model_space(Body::new(&start).points())));
        let mut small = Scratch::default();
        let body = Body::new(&Start { scale: 60, ..start });
        assert!(small.pose(&gla, &model_space(body.points())));
        for (a, b) in stock.matrices.iter().zip(&small.matrices) {
            for (row_a, row_b) in a.iter().zip(b) {
                for (x, y) in row_a.iter().zip(row_b) {
                    assert!((x - y).abs() < 1e-3, "{a:?} {b:?}");
                }
            }
        }
    }
}
