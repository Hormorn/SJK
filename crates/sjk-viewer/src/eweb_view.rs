//! An E-Web set down (`ET_GENERAL` with `models/map_objects/hoth/eweb_model.glm`, the
//! holdable or a map's `emplaced_eweb`), drawn with its top turned as the server turns it
//! (`CG_General`'s `CG_G2ServerBoneAngles`, OpenJK `codemp/cgame/cg_players.c:4385-4440`):
//! the entity's bone slots name `cannon_Yrot` and `cannon_Xrot` by their `CS_G2BONES`
//! strings, with the angles and axes (`boneOrient`) `EWeb_SetBoneAngles` gave them.
//!
//! Rigid models are drawn as instanced bind-pose meshes ([`crate::object_meshes`]), so the
//! gun is cut into three parts by the bone each triangle's first vertex leans on most:
//! the stand (the base and the tripod's legs), the top (`cannon_Yrot`) and the barrel
//! (`cannon_Xrot`), each moved by its bone's matrix in the model's pose at frame 0 — the
//! gun standing unfolded on its legs, as it does after its unfolding — with the turns
//! applied (`BONE_ANGLES_POSTMULT`). The short unfolding and firing animations
//! (`EF_G2ANIMATING`) are not played. The parts load on demand the first time a gun is
//! seen ([`wanted`]); until they have, the gun is drawn whole.

use super::*;
use sjk_model::{BoneAngleCommand, BoneAngleMode, BoneAxis, BoneOverridePose, Gla, GlmSurface};
use sjk_protocol::EntityState;
use sjk_runtime::Transform;
use std::sync::Arc;

/// The gun's model.
pub(crate) const MODEL: &str = "models/map_objects/hoth/eweb_model.glm";
/// `CS_G2BONES` (`bg_public.h:141`).
const CS_G2BONES: usize = 1_163;
/// `MAX_CLIENTS`: player slots are never guns.
const MAX_CLIENTS: u16 = 32;
/// `entityState_t::eType` on the wire, and `ET_GENERAL`.
const ES_ETYPE: usize = 8;
const ET_GENERAL: u32 = 0;
/// The bones the server turns, and the stand's.
const YAW_BONE: &str = "cannon_Yrot";
const PITCH_BONE: &str = "cannon_Xrot";
const STAND_BONE: &str = "base";
/// The appearance variants that name the three parts.
const STAND: &str = "*eweb stand";
const TOP: &str = "*eweb top";
const BARREL: &str = "*eweb barrel";

/// One of the three parts the gun is drawn as.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Part {
    Stand,
    Top,
    Barrel,
}

impl Part {
    const ALL: [Self; 3] = [Self::Stand, Self::Top, Self::Barrel];

    /// The part `appearance` names, if it names one.
    pub(crate) fn of(appearance: &Appearance) -> Option<Self> {
        match appearance.variant.as_str() {
            STAND => Some(Self::Stand),
            TOP => Some(Self::Top),
            BARREL => Some(Self::Barrel),
            _ => None,
        }
    }

    fn variant(self) -> &'static str {
        match self {
            Self::Stand => STAND,
            Self::Top => TOP,
            Self::Barrel => BARREL,
        }
    }

    fn bone(self) -> &'static str {
        match self {
            Self::Stand => STAND_BONE,
            Self::Top => YAW_BONE,
            Self::Barrel => PITCH_BONE,
        }
    }
}

/// What a loaded part keeps to be posed: the skeleton, the bone that carries it, and the
/// two the server turns.
#[derive(Debug)]
pub(crate) struct PartPose {
    skeleton: Arc<Gla>,
    bone: usize,
    yaw: usize,
    pitch: usize,
}

/// Whether `state` is a gun set down whose model is `appearance`'s.
fn is_eweb(state: &EntityState, appearance: &Appearance) -> bool {
    state.number() >= MAX_CLIENTS
        && state.raw_field(ES_ETYPE) == Some(ET_GENERAL)
        && appearance.model.eq_ignore_ascii_case(MODEL)
        && appearance.variant.is_empty()
}

/// The parts a gun of `snapshot` needs and `attempted` lacks, one at a time, for the model
/// loader ([`crate::model_demand`]).
pub(crate) fn wanted(
    snapshot: &Snapshot,
    world: &World,
    attempted: impl Fn(&Appearance) -> bool,
) -> Option<Appearance> {
    snapshot
        .entities
        .iter()
        .filter_map(|state| {
            let appearance = world
                .entity(EntityId::new(u64::from(state.number()) + 1))?
                .appearance()?;
            is_eweb(state, appearance).then(|| appearance.model.clone())
        })
        .flat_map(|model| {
            Part::ALL.map(|part| Appearance {
                model: model.clone(),
                variant: part.variant().to_owned(),
            })
        })
        .find(|appearance| !attempted(appearance))
}

/// The part of the skeleton a bone belongs to: the barrel's bone and what hangs from it,
/// else the top's, else the stand.
fn part_of_bone(skeleton: &Gla, mut index: usize, yaw: usize, pitch: usize) -> Part {
    let mut top = false;
    loop {
        if index == pitch {
            return Part::Barrel;
        }
        if index == yaw {
            top = true;
        }
        match skeleton.bones.get(index).and_then(|bone| bone.parent) {
            Some(parent) => index = parent,
            None => break,
        }
    }
    if top { Part::Top } else { Part::Stand }
}

/// The bone a vertex leans on most.
fn heaviest_bone(surface: &GlmSurface, vertex: u32) -> Option<usize> {
    let vertex = surface.vertices.get(vertex as usize)?;
    let weight = vertex
        .weights
        .iter()
        .max_by(|left, right| left.weight.total_cmp(&right.weight))?;
    surface.bone_references.get(weight.bone_reference).copied()
}

/// Parse one part of the gun's model and append its bind mesh to `flattened`; an error when
/// the skeleton lacks the bones.
pub(crate) fn load_part(
    vfs: &VirtualFileSystem,
    appearance: &Appearance,
    part: Part,
    flattened: &mut FlattenedScene,
) -> Result<StaticModelMesh, Box<dyn Error>> {
    let model = Glm::parse(
        &vfs.read(&appearance.model)?
            .ok_or("E-Web model was not found")?
            .bytes,
    )?;
    let skeleton = Gla::parse(
        &vfs.read(&format!("{}.gla", model.animation_name))?
            .ok_or("E-Web skeleton was not found")?
            .bytes,
    )?;
    let find = |name: &str| {
        skeleton
            .bones
            .iter()
            .position(|bone| bone.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("E-Web skeleton has no {name}"))
    };
    let (yaw, pitch, bone) = (find(YAW_BONE)?, find(PITCH_BONE)?, find(part.bone())?);
    let vertex_start = flattened.vertices.len();
    let draws = append_static_glm_triangles(flattened, &model, |surface, triangle| {
        heaviest_bone(surface, triangle[0])
            .is_some_and(|bone| part_of_bone(&skeleton, bone, yaw, pitch) == part)
    })?;
    Ok(StaticModelMesh {
        appearance: appearance.clone(),
        draws,
        center: render_helpers::mesh_center(&flattened.vertices[vertex_start..]),
        flash_bolt: None,
        turning_bone: None,
        eweb_part: Some(Arc::new(PartPose {
            skeleton: Arc::new(skeleton),
            bone,
            yaw,
            pitch,
        })),
    })
}

/// Ghoul2's root turn (`tr_ghoul2.cpp`'s "90 degree offset"): a quarter turn about the
/// vertical that the model's skeleton is posed in before the entity's placement, the one
/// the server reads the gun's muzzle through (`server_skeleton::GHOUL2_ROOT`). Without it
/// the barrel points to the gun's right.
const GHOUL2_ROOT: [[f32; 4]; 3] = [
    [0.0, -1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

/// `left` after `right`, as 3x4 affine matrices.
fn multiply(left: [[f32; 4]; 3], right: [[f32; 4]; 3]) -> [[f32; 4]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let turned = (0..3)
                .map(|inner| left[row][inner] * right[inner][column])
                .sum::<f32>();
            if column == 3 {
                turned + left[row][3]
            } else {
                turned
            }
        })
    })
}

/// The axis three bits of `boneOrient` name (`Eorientations`).
fn axis(bits: u32) -> Option<BoneAxis> {
    match bits & 7 {
        1 => Some(BoneAxis::PositiveX),
        2 => Some(BoneAxis::PositiveZ),
        3 => Some(BoneAxis::PositiveY),
        4 => Some(BoneAxis::NegativeX),
        5 => Some(BoneAxis::NegativeZ),
        6 => Some(BoneAxis::NegativeY),
        _ => None,
    }
}

/// The turns `state` gives the gun's two bones, by their `CS_G2BONES` names: the yaw
/// bone's and the pitch bone's angles, with the axes they are about.
fn turns(state: &EntityState, game_state: Option<&GameState>) -> [Option<BoneAngleCommand>; 2] {
    let orient = state.bone_orient();
    let (Some(up), Some(left), Some(forward)) =
        (axis(orient >> 6), axis(orient >> 3), axis(orient))
    else {
        return [None, None];
    };
    let mut turned = [None, None];
    for slot in 0..4 {
        let index = usize::from(state.bone_index(slot));
        if index == 0 {
            continue;
        }
        let Some(name) = game_state.and_then(|game| game.config_string(CS_G2BONES + index)) else {
            continue;
        };
        let which = if name.eq_ignore_ascii_case(YAW_BONE.as_bytes()) {
            0
        } else if name.eq_ignore_ascii_case(PITCH_BONE.as_bytes()) {
            1
        } else {
            continue;
        };
        turned[which] = Some(BoneAngleCommand {
            angles_degrees: state.bone_angles(slot),
            mode: BoneAngleMode::PostMultiply,
            up,
            left,
            forward,
        });
    }
    turned
}

/// Draw the gun `state` describes in its three parts, each by its bone's matrix in the
/// turned pose; `false` (nothing drawn) for any other entity or a gun whose parts are not
/// loaded yet.
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit(
    meshes: &[StaticModelMesh],
    groups: &mut [Vec<ActorInstance>],
    state: &EntityState,
    appearance: &Appearance,
    transform: &Transform,
    view_flags: u32,
    game_state: Option<&GameState>,
) -> bool {
    if !is_eweb(state, appearance) {
        return false;
    }
    let part = |part: Part| {
        meshes.iter().position(|mesh| {
            mesh.appearance.model == appearance.model && Part::of(&mesh.appearance) == Some(part)
        })
    };
    let (Some(stand), Some(top), Some(barrel)) =
        (part(Part::Stand), part(Part::Top), part(Part::Barrel))
    else {
        return false;
    };
    let Some(posing) = meshes[stand].eweb_part.as_deref() else {
        return false;
    };
    let skeleton = &posing.skeleton;
    let mut pose = BoneOverridePose::new(skeleton.bones.len());
    let [yaw, pitch] = turns(state, game_state);
    for (bone, command) in [(posing.yaw, yaw), (posing.pitch, pitch)] {
        if let Some(command) = command {
            let _ = pose.set_bone_angles(skeleton, bone, command);
        }
    }
    if pose.evaluate(skeleton, 0).is_err() {
        return false;
    }
    let matrices = pose.matrices();
    for mesh in [stand, top, barrel] {
        let Some(bone) = meshes[mesh].eweb_part.as_deref().map(|part| part.bone) else {
            continue;
        };
        let Some(&matrix) = matrices.get(bone) else {
            continue;
        };
        let (translation, rotation) =
            crate::emplaced_swivel::carried(transform, multiply(GHOUL2_ROOT, matrix));
        let mut instance = ActorInstance::new(translation, rotation, transform.scale);
        instance.view_flags = view_flags;
        groups[mesh].push(instance);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(name: &str, parent: Option<usize>) -> sjk_model::GlaBone {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        sjk_model::GlaBone {
            name: name.to_owned(),
            flags: 0,
            parent,
            base_pose: identity,
            inverse_base_pose: identity,
            children: Vec::new(),
        }
    }

    /// `model.gla`'s hierarchy: the root, the base, the top on it, the barrel on the top,
    /// a leg on the base.
    #[test]
    fn the_barrel_and_the_top_take_their_bones_and_what_hangs_from_them() {
        let skeleton = Gla {
            name: "model".into(),
            scale: 1.0,
            bones: vec![
                bone("model_root", None),
                bone("base", Some(0)),
                bone(YAW_BONE, Some(1)),
                bone(PITCH_BONE, Some(2)),
                bone("tripod_front", Some(1)),
            ],
            frames: Vec::new(),
            compressed_bones: Vec::new(),
        };
        assert_eq!(part_of_bone(&skeleton, 3, 2, 3), Part::Barrel);
        assert_eq!(part_of_bone(&skeleton, 2, 2, 3), Part::Top);
        assert_eq!(part_of_bone(&skeleton, 1, 2, 3), Part::Stand);
        assert_eq!(part_of_bone(&skeleton, 4, 2, 3), Part::Stand);
    }

    /// The root turn takes the model's +X (where the barrel lies in `eweb_model.glm`'s
    /// skeleton space) to the entity's +Y, as the server's bolts read it.
    #[test]
    fn the_root_turn_is_a_quarter_turn_counterclockwise() {
        let shift = [
            [1.0, 0.0, 0.0, 5.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, -24.0],
        ];
        let turned = multiply(GHOUL2_ROOT, shift);
        assert_eq!([turned[0][0], turned[1][0], turned[2][0]], [0.0, 1.0, 0.0]);
        assert_eq!(
            [turned[0][3], turned[1][3], turned[2][3]],
            [0.0, 5.0, -24.0]
        );
    }

    #[test]
    fn the_server_axes_are_read_from_bone_orient() {
        // `EWeb_SetBoneAngles`: forward -X, right -Z, up +Y.
        let orient = 4 | (5 << 3) | (3 << 6);
        assert_eq!(axis(orient), Some(BoneAxis::NegativeX));
        assert_eq!(axis(orient >> 3), Some(BoneAxis::NegativeZ));
        assert_eq!(axis(orient >> 6), Some(BoneAxis::PositiveY));
        assert_eq!(axis(0), None);
    }

    #[test]
    fn only_a_gun_set_down_is_an_eweb() {
        let appearance = |model: &str| Appearance {
            model: model.to_owned(),
            variant: String::new(),
        };
        let mut state = EntityState::zero(80, &sjk_protocol::LEGACY_ENTITY_FIELDS);
        assert!(is_eweb(&state, &appearance(MODEL)));
        assert!(!is_eweb(
            &state,
            &appearance("models/map_objects/mp/turret_chair.glm")
        ));
        // The holdable's pickup is an item (`ET_ITEM`).
        state.set_raw_field(ES_ETYPE, 2);
        assert!(!is_eweb(&state, &appearance(MODEL)));
        let player = EntityState::zero(3, &sjk_protocol::LEGACY_ENTITY_FIELDS);
        assert!(!is_eweb(&player, &appearance(MODEL)));
    }
}
