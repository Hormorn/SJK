//! An emplaced gun's top turning with its gunner's view (`CG_General`, OpenJK
//! `codemp/cgame/cg_ents.c:1392-1416`): while a gun is worked (`activeForcePass` is
//! `NUM_FORCE_POWERS + 1`, set by `emplaced_gun_use`), its model's `Bone02` takes the
//! gunner's view angles, the pitch held to 40 degrees down and the yaw less the gun's own,
//! with `BONE_ANGLES_REPLACE` about `NEGATIVE_Y`, `NEGATIVE_X`, `POSITIVE_Z`. The turn stays
//! on the gun's Ghoul2 instance after the gunner leaves, so the top keeps pointing where
//! it was last aimed. A gun no one has worked keeps its bind pose, the barrels to its
//! right, as it does in the retail game.
//!
//! Rigid models are drawn as instanced bind-pose meshes ([`crate::object_meshes`]), so a
//! turned gun is drawn as two parts cut from its model: the surfaces bound to `Bone02` and
//! its descendants (the top and both barrels in `models/map_objects/mp/turret_chair.glm`),
//! moved by the bone's matrix ([`sjk_model::replace_bone_matrix`]), and the rest, the
//! stand. Unlike Ghoul2, which leaves the stand on `Bone01` where it is, SJK turns the
//! stand with the top's yaw about the bone's vertical: its brace carries the handles the
//! gunner holds, which belong behind the top (they sit there at the bind pose), and a
//! stand left still shows them off to the gun's side. The parts load on demand the first
//! time a gun is worked ([`wanted`]).

use super::*;
use sjk_model::{BoneAxis, GlaBone, GlmSurface};
use sjk_protocol::EntityState;
use sjk_runtime::Transform;

/// The bone `CG_General` turns.
const BONE: &str = "Bone02";
/// `activeForcePass` of a worked gun: `NUM_FORCE_POWERS + 1`.
const WORKED: u32 = 19;
/// `entityState_t::activeForcePass`, protocol-26 netfield 68 of an entity.
const ES_ACTIVE_FORCE_PASS: usize = 68;
/// `MAX_CLIENTS`: player slots never take the rule.
const MAX_CLIENTS: u16 = 32;
/// `CLASS_VEHICLE`: vehicles never take the rule.
const CLASS_VEHICLE: u8 = 53;
/// The lowest the top is pitched, in degrees down.
const PITCH_LIMIT: f32 = 40.0;
/// The appearance variants that name the two parts of a gun's model.
const STAND: &str = "*emplaced stand";
const TURNING: &str = "*emplaced turning";

/// One of the two parts a turned gun is drawn as.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Part {
    /// The surfaces the bone does not carry: the stand, its brace and the handles,
    /// turned with the top's yaw only.
    Stand,
    /// The surfaces bound to the bone or its descendants.
    Turning,
}

impl Part {
    /// The part `appearance` names, if it names one.
    pub(crate) fn of(appearance: &Appearance) -> Option<Self> {
        match appearance.variant.as_str() {
            STAND => Some(Self::Stand),
            TURNING => Some(Self::Turning),
            _ => None,
        }
    }

    fn appearance(self, model: &str) -> Appearance {
        let variant = match self {
            Self::Stand => STAND,
            Self::Turning => TURNING,
        };
        Appearance {
            model: model.to_owned(),
            variant: variant.to_owned(),
        }
    }
}

/// The last angles each gun's top was turned to, by entity number: Ghoul2 keeps a
/// replaced bone's angles until they are set again.
#[derive(Default)]
pub(crate) struct Turned {
    angles: HashMap<u16, [f32; 3]>,
}

/// Whether `state` is a gun being worked, the case `CG_General` turns the bone in.
fn worked(state: &EntityState) -> bool {
    state.number() >= MAX_CLIENTS
        && state.raw_field(ES_ACTIVE_FORCE_PASS) == Some(WORKED)
        && state.npc_class() != CLASS_VEHICLE
}

/// The parts of `model` a worked gun needs and `attempted` lacks, one at a time, for the
/// model loader ([`crate::model_demand`]).
pub(crate) fn wanted(
    snapshot: &Snapshot,
    world: &World,
    attempted: impl Fn(&Appearance) -> bool,
) -> Option<Appearance> {
    snapshot
        .entities
        .iter()
        .filter(|state| worked(state))
        .filter_map(|state| {
            world
                .entity(EntityId::new(u64::from(state.number()) + 1))?
                .appearance()
        })
        .filter(|appearance| appearance.model.to_ascii_lowercase().ends_with(".glm"))
        .flat_map(|appearance| {
            [Part::Stand, Part::Turning].map(|part| part.appearance(&appearance.model))
        })
        .find(|appearance| !attempted(appearance))
}

/// Parse one part of a gun's model and append its bind mesh to `flattened`; an error
/// when the model's skeleton has no `Bone02`.
pub(crate) fn load_part(
    vfs: &VirtualFileSystem,
    appearance: &Appearance,
    part: Part,
    flattened: &mut FlattenedScene,
) -> Result<StaticModelMesh, Box<dyn Error>> {
    let model = Glm::parse(
        &vfs.read(&appearance.model)?
            .ok_or("emplaced gun model was not found")?
            .bytes,
    )?;
    let skeleton = Gla::parse(
        &vfs.read(&format!("{}.gla", model.animation_name))?
            .ok_or("emplaced gun skeleton was not found")?
            .bytes,
    )?;
    let bone = skeleton
        .bones
        .iter()
        .position(|bone| bone.name.eq_ignore_ascii_case(BONE))
        .ok_or("emplaced gun skeleton has no Bone02")?;
    let carried = |mut index: usize| loop {
        if index == bone {
            return true;
        }
        match skeleton.bones.get(index).and_then(|bone| bone.parent) {
            Some(parent) => index = parent,
            None => return false,
        }
    };
    let turns = |surface: &GlmSurface| {
        surface.vertices.iter().any(|vertex| {
            vertex.weights.iter().any(|weight| {
                surface
                    .bone_references
                    .get(weight.bone_reference)
                    .is_some_and(|&index| carried(index))
            })
        })
    };
    let vertex_start = flattened.vertices.len();
    let draws = append_static_glm_surfaces(flattened, &model, |surface| {
        turns(surface) == (part == Part::Turning)
    })?;
    Ok(StaticModelMesh {
        appearance: appearance.clone(),
        draws,
        center: render_helpers::mesh_center(&flattened.vertices[vertex_start..]),
        flash_bolt: None,
        turning_bone: (part == Part::Turning).then(|| skeleton.bones[bone].clone()),
        eweb_part: None,
    })
}

/// The angles `CG_General` gives the bone from the gunner's view: the pitch held to
/// [`PITCH_LIMIT`] down, the yaw made the gun's own.
fn bone_angles(view: [f32; 3], gun_yaw: f32) -> [f32; 3] {
    [view[0].min(PITCH_LIMIT), view[1] - gun_yaw, view[2]]
}

/// Draw the gun `state` describes in its two parts when its top has been turned, the
/// turning part carried by `Bone02`'s matrix and the stand by its yaw; `false` (nothing
/// drawn) for any other entity, a gun no one has worked, or one whose parts are not
/// loaded yet. `gunner_view` gives a gunner's view angles by client number.
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit(
    turned: &mut Turned,
    meshes: &[StaticModelMesh],
    groups: &mut [Vec<ActorInstance>],
    state: &EntityState,
    appearance: &Appearance,
    transform: &Transform,
    view_flags: u32,
    gunner_view: impl FnOnce(u16) -> Option<[f32; 3]>,
) -> bool {
    let number = state.number();
    if worked(state)
        && let Ok(gunner) = u16::try_from(state.emplaced_owner())
        && let Some(view) = gunner_view(gunner)
    {
        turned
            .angles
            .insert(number, bone_angles(view, state.angles()[1]));
    }
    let Some(&angles) = turned.angles.get(&number) else {
        return false;
    };
    let part = |part: Part| {
        meshes.iter().position(|mesh| {
            mesh.appearance.model == appearance.model && Part::of(&mesh.appearance) == Some(part)
        })
    };
    let (Some(stand), Some(turning)) = (part(Part::Stand), part(Part::Turning)) else {
        return false;
    };
    let Some(bone) = meshes[turning].turning_bone.as_ref() else {
        return false;
    };
    for (mesh, matrix) in [
        (stand, stand_matrix(bone, angles)),
        (turning, turned_matrix(bone, angles)),
    ] {
        let (translation, rotation) = carried(transform, matrix);
        let mut instance = ActorInstance::new(translation, rotation, transform.scale);
        instance.view_flags = view_flags;
        groups[mesh].push(instance);
    }
    true
}

/// `Bone02`'s model-space matrix at `angles`, with `CG_General`'s axes: up `NEGATIVE_Y`,
/// left `NEGATIVE_X`, forward `POSITIVE_Z`.
fn turned_matrix(bone: &GlaBone, angles: [f32; 3]) -> [[f32; 4]; 3] {
    sjk_model::replace_bone_matrix(
        bone,
        angles,
        BoneAxis::NegativeY,
        BoneAxis::NegativeX,
        BoneAxis::PositiveZ,
    )
}

/// The stand's model-space matrix: the yaw the top takes at `angles`, about the vertical
/// through the bone's origin, without its pitch or roll, so the stand stays upright on
/// the ground and its handles stay behind the top.
fn stand_matrix(bone: &GlaBone, angles: [f32; 3]) -> [[f32; 4]; 3] {
    let level = turned_matrix(bone, [0.0, angles[1], 0.0]);
    let (sine, cosine) = level[1][0].atan2(level[0][0]).sin_cos();
    let pivot = [bone.base_pose[0][3], bone.base_pose[1][3]];
    [
        [
            cosine,
            -sine,
            0.0,
            pivot[0] - (cosine * pivot[0] - sine * pivot[1]),
        ],
        [
            sine,
            cosine,
            0.0,
            pivot[1] - (sine * pivot[0] + cosine * pivot[1]),
        ],
        [0.0, 0.0, 1.0, 0.0],
    ]
}

/// The entity's placement followed by the bone's model-space `matrix`: the turning
/// part's position and rotation (`[x, y, z, w]`).
pub(crate) fn carried(transform: &Transform, matrix: [[f32; 4]; 3]) -> ([f32; 3], [f32; 4]) {
    let entity = glam::Quat::from_array(transform.rotation);
    let bone = glam::Quat::from_mat3(&glam::Mat3::from_cols(
        glam::Vec3::new(matrix[0][0], matrix[1][0], matrix[2][0]),
        glam::Vec3::new(matrix[0][1], matrix[1][1], matrix[2][1]),
        glam::Vec3::new(matrix[0][2], matrix[1][2], matrix[2][2]),
    ));
    let offset = glam::Vec3::new(matrix[0][3], matrix[1][3], matrix[2][3])
        * glam::Vec3::from_array(transform.scale);
    let translation = glam::Vec3::from_array(transform.translation) + entity * offset;
    (
        translation.to_array(),
        (entity * bone).normalize().to_array(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `entityState_t::NPC_class` on the wire.
    const NPC_CLASS: usize = 88;

    #[test]
    fn the_top_is_held_to_forty_degrees_down_and_turned_from_the_gun_yaw() {
        assert_eq!(bone_angles([60.0, 100.0, 0.0], 90.0), [40.0, 10.0, 0.0]);
        assert_eq!(bone_angles([-30.0, 45.0, 0.0], 360.0), [-30.0, -315.0, 0.0]);
    }

    #[test]
    fn players_and_vehicles_and_idle_guns_are_not_worked() {
        let gun = |number: u16, pass: u32, class: u32| {
            let mut state = EntityState::zero(number, &sjk_protocol::LEGACY_ENTITY_FIELDS);
            state.set_raw_field(ES_ACTIVE_FORCE_PASS, pass);
            state.set_raw_field(NPC_CLASS, class);
            state
        };
        assert!(worked(&gun(100, WORKED, 0)));
        assert!(!worked(&gun(100, 0, 0)));
        assert!(!worked(&gun(5, WORKED, 0)));
        assert!(!worked(&gun(100, WORKED, u32::from(CLASS_VEHICLE))));
    }

    /// The bone's matrix carries the part about the entity: at yaw 90 a part moved one
    /// unit along the model's +X lands one unit along the world's +Y.
    #[test]
    fn the_turning_part_follows_the_entity_placement() {
        let transform = Transform {
            translation: [10.0, 20.0, 30.0],
            rotation: sjk_client::legacy_angles_to_quaternion([0.0, 90.0, 0.0]),
            scale: [1.0; 3],
        };
        let shift = [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let (translation, rotation) = carried(&transform, shift);
        let close = |left: [f32; 3], right: [f32; 3]| {
            left.iter()
                .zip(right)
                .all(|(left, right)| (left - right).abs() < 1e-4)
        };
        assert!(close(translation, [10.0, 21.0, 30.0]));
        let turned = glam::Quat::from_array(rotation) * glam::Vec3::X;
        assert!(close(turned.to_array(), [0.0, 1.0, 0.0]));
    }

    /// The turret's `Bone02`, a little tilted as in `mp.gla`: its local x up, y along
    /// model +Y, z along -X, above the stand.
    fn swivel_bone() -> GlaBone {
        let (tilt, pivot) = (0.0137_f32, [0.0, 1.2, 46.6]);
        let base_pose = [
            [0.0, 0.0, -1.0, pivot[0]],
            [-tilt, 1.0, 0.0, pivot[1]],
            [1.0, tilt, 0.0, pivot[2]],
        ];
        let mut inverse_base_pose = [[0.0; 4]; 3];
        for row in 0..3 {
            for column in 0..3 {
                inverse_base_pose[row][column] = base_pose[column][row];
            }
            inverse_base_pose[row][3] = -(0..3)
                .map(|inner| base_pose[inner][row] * pivot[inner])
                .sum::<f32>();
        }
        GlaBone {
            name: BONE.to_owned(),
            flags: 0,
            parent: None,
            base_pose,
            inverse_base_pose,
            children: Vec::new(),
        }
    }

    /// The stand turns with the top's yaw, about the vertical through the pivot and
    /// never tipped by its pitch: the handles, behind the top at the bind pose (model
    /// +Y, the barrels at -Y), stay behind it.
    #[test]
    fn the_stand_keeps_its_handles_behind_the_top() {
        let bone = swivel_bone();
        let apply = |matrix: [[f32; 4]; 3], point: [f32; 3]| -> [f32; 3] {
            std::array::from_fn(|row| {
                (0..3)
                    .map(|inner| matrix[row][inner] * point[inner])
                    .sum::<f32>()
                    + matrix[row][3]
            })
        };
        let close = |left: [f32; 3], right: [f32; 3]| {
            left.iter()
                .zip(right)
                .all(|(left, right)| (left - right).abs() < 1e-3)
        };
        for angles in [[0.0, 0.0, 0.0], [40.0, 0.0, 0.0], [-30.0, 70.0, 0.0]] {
            let stand = stand_matrix(&bone, angles);
            let top = turned_matrix(&bone, angles);
            // The barrels' level heading and the handles' are opposite.
            let barrel = apply(top, [0.0, -60.0, 46.6]);
            let handle = apply(stand, [0.0, 21.2, 46.6]);
            let (barrel, handle) = (
                glam::Vec2::new(barrel[0], barrel[1] - 1.2).normalize(),
                glam::Vec2::new(handle[0], handle[1] - 1.2).normalize(),
            );
            assert!(barrel.dot(handle) < -0.999, "{angles:?}");
            // Upright, about the pivot: heights and the pivot do not move.
            assert!(close(apply(stand, [0.0, 1.2, 0.0]), [0.0, 1.2, 0.0]));
            assert!((apply(stand, [5.0, -9.0, 17.0])[2] - 17.0).abs() < 1e-4);
        }
    }
}
