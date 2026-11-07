use super::*;
use glam::{Quat, Vec3A};
use sjk_model::GlaBone;
use sjk_model::gltf::{GltfAnimation, GltfChannel, GltfJoint, GltfPath, GltfPrimitive};

fn identity_rows() -> [[f32; 4]; 3] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]
}

fn translated(x: f32, y: f32, z: f32) -> [[f32; 4]; 3] {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

/// A driver with a pelvis 40 units up and a cranium above it.
fn driver() -> Gla {
    let bone = |name: &str, parent, base| GlaBone {
        name: name.to_owned(),
        flags: 0,
        parent,
        base_pose: base,
        inverse_base_pose: identity_rows(),
        children: Vec::new(),
    };
    Gla {
        name: "_humanoid".into(),
        scale: 1.0,
        bones: vec![
            bone("model_root", None, identity_rows()),
            bone("pelvis", Some(0), translated(0.0, 0.0, 40.0)),
            bone("cranium", Some(1), translated(0.0, 0.0, 60.0)),
        ],
        frames: Vec::new(),
        compressed_bones: Vec::new(),
    }
}

/// A rig with a pelvis one metre up (glTF +Y) and a head half a metre above it,
/// one vertex on the head, and a clip "Nod" turning the head a quarter turn about
/// glTF +Y (up) over one second.
fn rig() -> GltfModel {
    let joint = |name: &str, parent, y: f32, rest: f32| GltfJoint {
        name: name.to_owned(),
        parent,
        bind: translated(0.0, y, 0.0),
        inverse_bind: translated(0.0, -y, 0.0),
        rest: translated(0.0, rest, 0.0),
    };
    let half = std::f32::consts::FRAC_1_SQRT_2;
    GltfModel {
        animations: vec![GltfAnimation {
            name: "Nod".into(),
            channels: vec![GltfChannel {
                joint: 1,
                path: GltfPath::Rotation,
                times: vec![0.0, 1.0],
                values: vec![[0.0, 0.0, 0.0, 1.0], [0.0, half, 0.0, half]],
                step: false,
            }],
        }],
        joints: vec![
            joint("pelvis", None, 1.0, 1.0),
            joint("head", Some(0), 1.5, 0.5),
        ],
        primitives: vec![GltfPrimitive {
            base_color_image: Some("textures/skin.png".into()),
            positions: vec![[0.0, 1.6, 0.1]; 3],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            texture_coordinates: vec![[0.0; 2]; 3],
            joints: vec![[1, 0, 0, 0]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
            indices: vec![0, 1, 2],
            ..Default::default()
        }],
    }
}

fn manifest(text: &str) -> Manifest {
    Manifest::parse(text).expect("the manifest")
}

#[test]
fn a_manifest_reads_every_key_and_rejects_nonsense() {
    let parsed = manifest(
        "// wolf\nmodel wolf.gltf\ndriver models/players/jedi_hm/\npreset Mannequin\nscale 40\nmap tail pelvis\nmap ear_l cranium as head\n",
    );
    assert_eq!(parsed.model.as_deref(), Some("wolf.gltf"));
    assert_eq!(parsed.driver, "models/players/jedi_hm");
    assert_eq!(parsed.scale, Some(40.0));
    assert_eq!(parsed.map[1].align, Align::As("head".into()));
    assert!(Manifest::parse("scale -1").is_err());
    assert!(Manifest::parse("colour red").is_err());
    assert_eq!(manifest("").driver, DEFAULT_DRIVER);
}

#[test]
fn manifest_map_lines_replace_preset_entries() {
    let map = manifest("map head cervical").bone_map().expect("the map");
    let head: Vec<_> = map.iter().filter(|e| e.joint == "head").collect();
    assert_eq!(head.len(), 1);
    assert_eq!(head[0].bone, "cervical");
    assert!(manifest("preset nothing").bone_map().is_err());
}

#[test]
fn auto_scale_fits_the_pelvis_and_the_axes_turn_to_z_up() {
    let rig = RigModel::fit(rig(), "models/players/wolf", &manifest(""), &driver()).unwrap();
    // 1 m pelvis onto a 40-unit pelvis: 40 units a metre; glTF +Y becomes +Z,
    // glTF +Z (front) becomes -Y.
    let vertex = rig.surfaces[0].vertices[0].position;
    assert!((vertex[2] - 64.0).abs() < 1e-4, "{vertex:?}");
    assert!((vertex[1] + 4.0).abs() < 1e-4, "{vertex:?}");
    assert_eq!(
        rig.surfaces[0].shader,
        "models/players/wolf/textures/skin.png"
    );
}

#[test]
fn the_base_pose_gives_the_bind_pose() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    rig.pose(&bones, None, &mut scratch);
    let palette = &scratch.palette;
    for matrix in palette {
        for (row, expected) in matrix.iter().zip(identity_rows()) {
            for (a, b) in row.iter().zip(expected) {
                assert!((a - b).abs() < 1e-4, "{palette:?}");
            }
        }
    }
}

#[test]
fn a_turned_driver_bone_turns_its_joint_and_children() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    // The cranium turned 90 degrees about Z around its base position (0, 0, 60).
    let turn = [
        [0.0, -1.0, 0.0, 60.0],
        [1.0, 0.0, 0.0, 60.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let bones = vec![identity_rows(), identity_rows(), turn];
    let mut scratch = Scratch::default();
    rig.pose(&bones, None, &mut scratch);
    let vertex = Vec3A::from_array(rig.surfaces[0].vertices[0].position);
    let moved = affine(&scratch.palette[1]).transform_point3a(vertex);
    // The CPU path skins the same way.
    let (cpu, _) = skin(&rig.surfaces[0].vertices[0], &scratch.palette);
    assert!((Vec3A::from_array(cpu) - moved).length() < 1e-4);
    // Before: 4 units in front (-Y) of the head joint; after a quarter turn about +Z
    // the offset points along +X, and the head joint itself stays where it was.
    let head = rig.joints[1].bind.translation;
    let offset = moved - head;
    assert!(
        (offset.x - 4.0).abs() < 1e-3 && offset.y.abs() < 1e-3,
        "{offset:?}"
    );
}

#[test]
fn triangles_turn_to_the_game_winding() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    // glTF fronts are counter-clockwise; the game's `cull front` keeps clockwise ones.
    assert_eq!(rig.surfaces[0].indices, vec![0, 2, 1]);
}

#[test]
fn a_bolt_moves_from_the_driver_bone_onto_the_rig_joint() {
    let mut driver = driver();
    driver.bones.push(GlaBone {
        name: "ceyebrow".into(),
        flags: 0,
        parent: Some(2),
        base_pose: translated(0.0, -3.0, 62.0),
        inverse_base_pose: identity_rows(),
        children: Vec::new(),
    });
    // 50 units a metre: the rig's head joint sits at 75, the driver's cranium at 60.
    let rig = RigModel::fit(rig(), "", &manifest("scale 50"), &driver).unwrap();
    // The cranium shifted 10 units along +X; its bolt 5 in front and 5 above it.
    let bones = vec![
        identity_rows(),
        identity_rows(),
        translated(10.0, 0.0, 0.0),
        translated(10.0, 0.0, 0.0),
    ];
    let bolt = translated(10.0, -5.0, 65.0);
    let moved = rig.retarget_bolt(2, &bones, None, bolt);
    let origin = Vec3A::new(moved[0][3], moved[1][3], moved[2][3]);
    assert!(
        (origin - Vec3A::new(0.0, -5.0, 80.0)).length() < 1e-4,
        "{origin:?}"
    );
    // The orientation is the driver's.
    assert_eq!(moved[0][..3], bolt[0][..3]);
    // An unmapped bone (the eyebrow) follows its mapped parent's joint.
    let point = rig.retarget_point(3, &bones, None, Vec3A::new(10.0, -5.0, 65.0));
    assert!((point - origin).length() < 1e-4, "{point:?}");
}

/// The head vertex skinned with `scratch`'s palette, minus the head joint.
fn head_offset(rig: &RigModel, scratch: &Scratch) -> Vec3A {
    let vertex = Vec3A::from_array(rig.surfaces[0].vertices[0].position);
    affine(&scratch.palette[1]).transform_point3a(vertex) - rig.joints[1].bind.translation
}

#[test]
fn a_clip_plays_on_the_rigs_own_skeleton() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    assert_eq!(rig.clips.len(), 1);
    assert_eq!(rig.clips[0].duration, 1.0);
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    // At its first key the clip is the rest pose: the vertex stays 4 units in front
    // (-Y) and 4 above the head joint.
    let play = rig.play("Nod", 0.0, None, false).map(|play| ClipPlay {
        weight: 1.0,
        ..play
    });
    rig.pose(&bones, play, &mut scratch);
    let offset = head_offset(&rig, &scratch);
    assert!(
        (offset - Vec3A::new(0.0, -4.0, 4.0)).length() < 1e-4,
        "{offset:?}"
    );
    // At its end, a quarter turn about glTF +Y is one about the game's +Z.
    let play = rig.play("Nod", 1.0, None, false);
    assert_eq!(play.map(|play| play.weight), Some(1.0));
    rig.pose(&bones, play, &mut scratch);
    let offset = head_offset(&rig, &scratch);
    assert!(
        (offset - Vec3A::new(4.0, 0.0, 4.0)).length() < 1e-3,
        "{offset:?}"
    );
    // Half faded in, half way round.
    let half = ClipPlay {
        weight: 0.5,
        ..play.unwrap()
    };
    rig.pose(&bones, Some(half), &mut scratch);
    let offset = head_offset(&rig, &scratch);
    let side = 4.0 * std::f32::consts::FRAC_1_SQRT_2;
    assert!(
        (offset - Vec3A::new(side, -side, 4.0)).length() < 1e-3,
        "{offset:?}"
    );
    assert!(rig.play("Wave", 0.0, None, false).is_none());
}

#[test]
fn a_clip_fades_in_and_out_and_a_loop_wraps() {
    let play = ClipPlay::new(0, 2.0, 0.1, Some(5.0), false);
    assert!((play.weight - 0.4).abs() < 1e-6 && play.time == 0.1);
    let play = ClipPlay::new(0, 2.0, 3.0, Some(0.05), false);
    assert!((play.weight - 0.2).abs() < 1e-6 && play.time == 2.0);
    let play = ClipPlay::new(0, 2.0, 5.5, None, true);
    assert!(play.weight == 1.0 && (play.time - 1.5).abs() < 1e-6);
}

#[test]
fn a_bolt_follows_the_joint_a_clip_moves() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    let bolt = translated(0.0, -5.0, 65.0);
    rig.pose(&bones, None, &mut scratch);
    let still = rig.retarget_bolt(2, &bones, Some(&scratch), bolt);
    assert_eq!(still, rig.retarget_bolt(2, &bones, None, bolt));
    rig.pose(&bones, rig.play("Nod", 1.0, None, false), &mut scratch);
    let turned = rig.retarget_bolt(2, &bones, Some(&scratch), bolt);
    let origin = Vec3A::new(turned[0][3], turned[1][3], turned[2][3]);
    assert!(
        (origin - Vec3A::new(5.0, 0.0, 65.0)).length() < 1e-3,
        "{origin:?}"
    );
    // Its axes turn with the head: X now points along +Y.
    assert!((turned[1][0] - 1.0).abs() < 1e-3, "{turned:?}");
}

/// The lowest point of the driver (Kyle) and of a rig model posed from the same
/// frames, to see where the rig's feet land against the driver's.
#[test]
#[ignore = "reads the retail Kyle and an installed rig model (JKA_GAME_DATA, SJK_TEST_RIG)"]
fn feet_against_the_driver() {
    let (Some(data), Ok(name)) = (
        std::env::var_os("JKA_GAME_DATA").map(std::path::PathBuf::from),
        std::env::var("SJK_TEST_RIG"),
    ) else {
        println!("JKA_GAME_DATA or SJK_TEST_RIG is not set: skipped");
        return;
    };
    let mut vfs = VirtualFileSystem::new();
    let base = data.join("base");
    for entry in std::fs::read_dir(&base).unwrap().flatten() {
        let path = entry.path();
        let file = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        if file.starts_with("assets") && file.ends_with(".pk3") || file == format!("rig_{name}.pk3")
        {
            vfs.mount_pk3(&path).unwrap();
        }
    }
    vfs.mount_directory(&base).unwrap();
    let read = |path: &str| vfs.read(path).unwrap().unwrap().bytes;
    let gla = Gla::parse(&read("models/players/_humanoid/_humanoid.gla")).unwrap();
    let glm = sjk_model::Glm::parse(&read("models/players/kyle/model.glm")).unwrap();
    let kyle_skin = sjk_model::Skin::parse(&read("models/players/kyle/model_default.skin"));
    let config =
        sjk_model::AnimationConfig::parse(&read("models/players/_humanoid/animation.cfg")).unwrap();
    let directory = format!("models/players/{name}");
    let variant = std::env::var("SJK_TEST_VARIANT").unwrap_or_else(|_| "default".to_owned());
    let manifest = super::manifest(&vfs, &directory, &variant)
        .unwrap()
        .expect("a rig model");
    let rig = RigModel::load(&vfs, &directory, &manifest, &gla, &glm).unwrap();
    let mut scratch = Scratch::default();
    let identity = vec![
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0]
        ];
        gla.bones.len()
    ];
    let kyle_base = glm
        .skin_pose_matrices(&kyle_skin, 0, &identity)
        .unwrap()
        .into_iter()
        .filter(|s| s.shader.is_some() && !s.name.contains("_cap_") && !s.name.starts_with('*'))
        .flat_map(|s| s.vertices)
        .map(|v| v.position[2])
        .fold(f32::MAX, f32::min);
    // Each pose stands alone (`None`: planted at once, not eased from the last).
    rig.pose_after(&identity, None, &mut scratch, None);
    let rig_base = rig
        .surfaces
        .iter()
        .flat_map(|s| &s.vertices)
        .map(|v| skin(v, &scratch.palette).0[2])
        .fold(f32::MAX, f32::min);
    println!(
        "base pose: Kyle lowest {kyle_base:.2} (raw {:?}), rig lowest {rig_base:.2} ({:+.2})",
        super::sole(&glm),
        rig_base - kyle_base
    );
    for (animation, frame) in [
        ("BOTH_STAND1", 0),
        ("BOTH_STAND2", 0),
        ("BOTH_RUN1", 4),
        ("BOTH_CROUCH1IDLE", 0),
        ("BOTH_A2_T__B_", 0),
        ("BOTH_WALK1", 3),
    ] {
        let sequence = config.get(animation).unwrap();
        let frame = sequence.first_frame + frame;
        let kyle = glm
            .skin(&gla, &kyle_skin, frame, 0)
            .unwrap()
            .into_iter()
            .filter(|s| s.shader.is_some() && !s.name.contains("_cap_") && !s.name.starts_with('*'))
            .flat_map(|s| s.vertices)
            .map(|v| v.position[2])
            .fold(f32::MAX, f32::min);
        let matrices = gla.frame_matrices(frame).unwrap();
        rig.pose_after(&matrices, None, &mut scratch, None);
        let lowest = rig
            .surfaces
            .iter()
            .flat_map(|s| &s.vertices)
            .map(|v| skin(v, &scratch.palette).0[2])
            .fold(f32::MAX, f32::min);
        let talus = gla.bones.iter().position(|b| b.name == "ltalus").unwrap();
        let kyle_ankle = affine(&matrices[talus])
            .transform_point3a(affine(&gla.bones[talus].base_pose).translation)
            .z;
        // The feet: the vertices whose strongest joint is a foot's or under it (a tail
        // or a cape may hang lower, printed apart).
        let feet_joints: Vec<usize> = rig.feet.as_ref().map_or(Vec::new(), |f| f.joints.clone());
        let on_foot = |mut joint: usize| loop {
            if feet_joints.contains(&joint) {
                return true;
            }
            match rig.joints[joint].parent {
                Some(parent) => joint = parent,
                None => return false,
            }
        };
        let paws = rig
            .surfaces
            .iter()
            .flat_map(|s| &s.vertices)
            .filter(|v| on_foot(usize::from(v.joints[0])))
            .map(|v| skin(v, &scratch.palette).0[2])
            .fold(f32::MAX, f32::min);
        let paws = if paws < f32::MAX { paws } else { lowest };
        let foot = rig.joints.len();
        println!(
            "{animation}: Kyle lowest {kyle:.2}, ankle {kyle_ankle:.2}; rig feet {paws:.2} ({:+.2}), lowest {lowest:.2} ({:+.2}), {foot} joints",
            paws - kyle,
            lowest - kyle
        );
        assert!(
            (paws - kyle).abs() < 1.5,
            "{animation}: feet off by {}",
            paws - kyle
        );
        // A clip of the rig's own over this frame stands on the same ground.
        if !rig.clips.is_empty() {
            let play = ClipPlay {
                clip: 0,
                time: 0.0,
                weight: 1.0,
            };
            rig.pose_after(&matrices, Some(play), &mut scratch, None);
            let dancing = rig
                .surfaces
                .iter()
                .flat_map(|s| &s.vertices)
                .map(|v| skin(v, &scratch.palette).0[2])
                .fold(f32::MAX, f32::min);
            println!(
                "  clip {}: lowest {dancing:.2} ({:+.2})",
                rig.clips[0].name,
                dancing - kyle
            );
        }
    }
}

#[test]
fn the_feet_land_on_the_drivers_ground_and_lift_raises_them() {
    // A driver whose soles are 2 units up in its base pose, its ankles at 10, and a
    // rig whose ankles stand 0.1 m (4 units at 40 a metre) above its soles at 0.
    let mut driver = driver();
    for (name, x) in [("ltalus", 5.0), ("rtalus", -5.0)] {
        driver.bones.push(GlaBone {
            name: name.into(),
            flags: 0,
            parent: Some(1),
            base_pose: translated(x, 0.0, 10.0),
            inverse_base_pose: identity_rows(),
            children: Vec::new(),
        });
    }
    let mut model = rig();
    let joint = |name: &str, x: f32| GltfJoint {
        name: name.to_owned(),
        parent: Some(0),
        bind: translated(x, 0.1, 0.0),
        inverse_bind: translated(-x, -0.1, 0.0),
        rest: translated(x, -0.9, 0.0),
    };
    model.joints.push(joint("foot_l", 0.1));
    model.joints.push(joint("foot_r", -0.1));
    // A sole vertex at 0 under the left foot.
    let primitive = &mut model.primitives[0];
    primitive.positions.push([0.1, 0.0, 0.0]);
    primitive.normals.push([0.0, 1.0, 0.0]);
    primitive.texture_coordinates.push([0.0; 2]);
    primitive.joints.push([2, 0, 0, 0]);
    primitive.weights.push([1.0, 0.0, 0.0, 0.0]);
    let fit = |text: &str| {
        RigModel::fit(model.clone(), "", &manifest(text), &driver)
            .unwrap()
            .with_feet(&driver, Some(2.0))
    };
    let rig = fit("map foot_l ltalus\nmap foot_r rtalus");
    let mut bones = vec![identity_rows(); 5];
    // The driver sinks 27 units, as a GLA frame does, and lifts its right foot.
    for bone in &mut bones[1..] {
        bone[2][3] = -27.0;
    }
    bones[4][2][3] = -20.0;
    let mut scratch = Scratch::default();
    rig.pose(&bones, None, &mut scratch);
    let lowest = |scratch: &Scratch| {
        rig.surfaces[0]
            .vertices
            .iter()
            .map(|v| skin(v, &scratch.palette).0[2])
            .fold(f32::MAX, f32::min)
    };
    // The driver's soles are at 2 - 27 = -25; so are the rig's, which the pelvis
    // alone puts at -27.
    assert!(
        (lowest(&scratch) + 25.0).abs() < 1e-3,
        "{}",
        lowest(&scratch)
    );
    let unplanted = RigModel::fit(model.clone(), "", &manifest(""), &driver).unwrap();
    let mut other = Scratch::default();
    unplanted.pose(&bones, None, &mut other);
    let low = unplanted.surfaces[0]
        .vertices
        .iter()
        .map(|v| skin(v, &other.palette).0[2])
        .fold(f32::MAX, f32::min);
    assert!((low + 27.0).abs() < 1e-3, "{low}");
    let lifted = fit("map foot_l ltalus\nmap foot_r rtalus\nlift 2");
    lifted.pose(&bones, None, &mut scratch);
    let low = lifted.surfaces[0]
        .vertices
        .iter()
        .map(|v| skin(v, &scratch.palette).0[2])
        .fold(f32::MAX, f32::min);
    assert!((low + 23.0).abs() < 1e-3, "{low}");
    assert!(Manifest::parse("lift up").is_err());
}

#[test]
fn soles_plant_the_lowest_sole_point_eased_over_time() {
    // The driver and rig of the test above, planted by their soles: the driver's 8
    // units under its ankles, the rig's sole vertex.
    let mut driver = driver();
    for (name, x) in [("ltalus", 5.0), ("rtalus", -5.0)] {
        driver.bones.push(GlaBone {
            name: name.into(),
            flags: 0,
            parent: Some(1),
            base_pose: translated(x, 0.0, 10.0),
            inverse_base_pose: identity_rows(),
            children: Vec::new(),
        });
    }
    let mut model = rig();
    for (name, x) in [("foot_l", 0.1), ("foot_r", -0.1)] {
        model.joints.push(GltfJoint {
            name: name.to_owned(),
            parent: Some(0),
            bind: translated(x, 0.1, 0.0),
            inverse_bind: translated(-x, -0.1, 0.0),
            rest: translated(x, -0.9, 0.0),
        });
    }
    let primitive = &mut model.primitives[0];
    primitive.positions.push([0.1, 0.0, 0.0]);
    primitive.normals.push([0.0, 1.0, 0.0]);
    primitive.texture_coordinates.push([0.0; 2]);
    primitive.joints.push([2, 0, 0, 0]);
    primitive.weights.push([1.0, 0.0, 0.0, 0.0]);
    let mut rig = RigModel::fit(
        model,
        "",
        &manifest(
            "map foot_l ltalus
map foot_r rtalus",
        ),
        &driver,
    )
    .unwrap()
    .with_feet(&driver, Some(2.0));
    let sole = Vec3A::from_array(rig.surfaces[0].vertices.last().unwrap().position);
    rig.feet.as_mut().expect("feet").soles = Some(Soles {
        driver: vec![
            vec![(3, Vec3A::new(5.0, 0.0, 2.0))],
            vec![(4, Vec3A::new(-5.0, 0.0, 2.0))],
        ],
        rig: vec![vec![(2, sole)], vec![(3, sole)]],
    });
    let lowest = |scratch: &Scratch| {
        let vertex = rig.surfaces[0].vertices.last().unwrap();
        skin(vertex, &scratch.palette).0[2]
    };
    // The driver sunk 27 units (a GLA frame), its ankles `depth`.
    let sunk = |depth: f32| {
        let mut bones = vec![identity_rows(); 5];
        for bone in &mut bones[1..3] {
            bone[2][3] = -27.0;
        }
        for bone in &mut bones[3..] {
            bone[2][3] = depth;
        }
        bones
    };
    let mut scratch = Scratch::default();
    // First pose: the rig's sole exactly on the driver's (2 - 27).
    rig.pose_after(&sunk(-27.0), None, &mut scratch, None);
    assert!(
        (lowest(&scratch) + 25.0).abs() < 1e-3,
        "{}",
        lowest(&scratch)
    );
    // The driver's feet drop 2 more: a pose PLANT_EASE later goes 1 - 1/e of the way.
    rig.pose_after(&sunk(-29.0), None, &mut scratch, Some(PLANT_EASE));
    let expected = -25.0 - 2.0 * (1.0 - (-1.0f32).exp());
    assert!(
        (lowest(&scratch) - expected).abs() < 1e-3,
        "{}",
        lowest(&scratch)
    );
    // A long gap (a teleport) plants at once.
    rig.pose_after(&sunk(-29.0), None, &mut scratch, Some(0.5));
    assert!(
        (lowest(&scratch) + 27.0).abs() < 1e-3,
        "{}",
        lowest(&scratch)
    );
}

/// [`rig`]'s skeleton with a longer neck (the head 0.8 metres above the pelvis), a
/// tail the clip knows nothing of, and no clip of its own.
fn longer_rig() -> GltfModel {
    let mut model = rig();
    model.animations.clear();
    model.joints[1].bind = translated(0.0, 1.8, 0.0);
    model.joints[1].inverse_bind = translated(0.0, -1.8, 0.0);
    model.joints[1].rest = translated(0.0, 0.8, 0.0);
    model.joints.push(GltfJoint {
        name: "tail".into(),
        parent: Some(0),
        bind: translated(0.0, 0.9, 0.2),
        inverse_bind: translated(0.0, -0.9, -0.2),
        rest: translated(0.0, -0.1, 0.2),
    });
    model
}

#[test]
fn a_rig_with_the_same_joint_names_borrows_a_clip_and_keeps_its_lengths() {
    let source = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    let mut target = RigModel::fit(longer_rig(), "", &manifest(""), &driver()).unwrap();
    assert!(target.clips.is_empty());
    let clip = target
        .borrowed(&source, &source.clips[0])
        .expect("the same names");
    assert_eq!(clip.name, "Nod");
    assert_eq!(clip.duration, 1.0);
    target.clips.push(clip);
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    target.pose(&bones, target.play("Nod", 1.0, None, false), &mut scratch);
    // The head turns a quarter about the game's +Z, as the source's does, and stays
    // on its own longer neck (0.8 m above the pelvis at 40 units a metre).
    let head = scratch.pose[1];
    let turned = Mat3A::from_rotation_z(std::f32::consts::FRAC_PI_2);
    assert!(head.matrix3.abs_diff_eq(turned, 1e-4), "{head:?}");
    let expected = target.joints[1].bind.translation;
    assert!((head.translation - expected).length() < 1e-3, "{head:?}");
    assert!((expected.z - 72.0).abs() < 1e-3, "{expected:?}");
    // The tail, unknown to the clip, stays where the pelvis holds it.
    let tail = scratch.pose[2];
    assert!(tail.abs_diff_eq(target.joints[2].bind, 1e-4), "{tail:?}");
}

#[test]
fn a_rig_without_the_clips_joints_does_not_borrow_it() {
    let source = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    let mut other = longer_rig();
    other.joints[1].name = "skull".into();
    let mut map = manifest("map skull cranium");
    map.preset = "mannequin".into();
    let target = RigModel::fit(other, "", &map, &driver()).unwrap();
    assert!(target.borrowed(&source, &source.clips[0]).is_none());
}

#[test]
fn an_emote_names_the_rig_its_clip_comes_from() {
    let def = crate::emotes::parse(
        "kane_dance",
        r#"{"name": "Dance", "clip": "kane_dance", "length_ms": 6800, "rig": "kane"}"#,
    )
    .unwrap();
    assert_eq!(def.rig, "kane");
    for bad in ["../kane", "a/b", r"a\b"] {
        let text = format!(r#"{{"name": "Dance", "length_ms": 6800, "rig": "{bad}"}}"#);
        assert!(crate::emotes::parse("kane_dance", &text).is_err(), "{bad}");
    }
}

/// [`rig`] with a three-joint tail behind the pelvis (glTF +Z is the front, so the
/// tail runs along -Z), moving on its own.
fn tailed() -> RigModel {
    tailed_with("dynamic tail_root")
}

/// [`tailed`] with the manifest `text`.
fn tailed_with(text: &str) -> RigModel {
    let mut model = rig();
    let joint = |name: &str, parent, z: f32, rest: f32| GltfJoint {
        name: name.to_owned(),
        parent: Some(parent),
        bind: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, z],
        ],
        inverse_bind: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, -1.0],
            [0.0, 0.0, 1.0, -z],
        ],
        rest: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, rest],
        ],
    };
    model.joints.push(joint("tail_root", 0, -0.1, -0.1));
    model.joints.push(joint("tail_1", 2, -0.4, -0.3));
    model.joints.push(joint("tail_2", 3, -0.7, -0.3));
    RigModel::fit(model, "", &manifest(text), &driver()).unwrap()
}

#[test]
fn a_dynamic_tail_lags_behind_the_body_then_catches_up() {
    let rig = tailed();
    let mut scratch = Scratch::default();
    let still = vec![identity_rows(); 3];
    rig.pose_after(&still, None, &mut scratch, None);
    let length = |pose: &[Affine3A]| (pose[4].translation - pose[3].translation).length();
    let bone = length(&scratch.pose);
    // The body steps 10 units sideways: on the next frame the tip has not followed.
    let mut moved = still.clone();
    moved[1] = translated(10.0, 0.0, 0.0);
    rig.pose_after(&moved, None, &mut scratch, Some(1.0 / 60.0));
    let root = scratch.pose[2].translation.x;
    let tip = scratch.pose[4].translation.x;
    assert!((root - 10.0).abs() < 1e-3, "{root}");
    assert!(tip < 5.0, "the tip lags: {tip}");
    assert!((length(&scratch.pose) - bone).abs() < 1e-3);
    // Three seconds later it is back behind the body, give or take the wag.
    for _ in 0..180 {
        rig.pose_after(&moved, None, &mut scratch, Some(1.0 / 60.0));
    }
    let tip = scratch.pose[4].translation.x;
    assert!((tip - 10.0).abs() < 2.0 * dynamics::WAG_UNITS, "{tip}");
    assert!((length(&scratch.pose) - bone).abs() < 1e-3);
}

#[test]
fn a_dynamic_tail_wags_while_the_body_stands_still() {
    let rig = tailed();
    let mut scratch = Scratch::default();
    let still = vec![identity_rows(); 3];
    rig.pose_after(&still, None, &mut scratch, None);
    let (mut left, mut right) = (0.0f32, 0.0f32);
    for _ in 0..300 {
        rig.pose_after(&still, None, &mut scratch, Some(1.0 / 60.0));
        let tip = scratch.pose[4].translation.x;
        left = left.min(tip);
        right = right.max(tip);
    }
    assert!(left < -0.5 && right > 0.5, "{left} {right}");
    assert!(left > -2.0 * dynamics::WAG_UNITS && right < 2.0 * dynamics::WAG_UNITS);
    // The body itself does not move.
    assert!(scratch.pose[0].abs_diff_eq(rig.joints[0].bind, 1e-4));
    // A long hitch starts the tail again where the animation puts it.
    rig.pose_after(&still, None, &mut scratch, Some(2.0));
    assert!(scratch.pose[3].translation.x.abs() < dynamics::WAG_UNITS);
}

#[test]
fn a_chains_limit_may_be_anything_from_none_to_no_limit() {
    let limit = |line: &str| manifest(line).dynamic[0].1.limit;
    assert_eq!(limit("dynamic tail limit 0"), 0.0);
    assert_eq!(limit("dynamic tail limit 180"), dynamics::NO_LIMIT);
}

#[test]
fn layer_lines_that_do_nothing_are_named() {
    let names = ["a", "b", "c", "d"];
    let problems = |layers: &[(usize, usize)], anchors: &[usize]| {
        layer_problems(layers, anchors, |joint| names[joint])
    };
    assert!(problems(&[(0, 1), (0, 2), (1, 2)], &[0, 1, 2]).is_empty());
    assert_eq!(
        problems(&[(0, 0)], &[0]),
        ["layer a over a: a chain cannot lie over itself"]
    );
    assert_eq!(
        problems(&[(0, 3)], &[0]),
        ["layer a over d: d has no dynamic line"]
    );
    assert_eq!(
        problems(&[(3, 0)], &[0]),
        ["layer d over a: d has no dynamic line"]
    );
    assert_eq!(
        problems(&[(0, 1), (1, 0)], &[0, 1]),
        [
            "layer a over b: the layer lines go round in a circle",
            "layer b over a: the layer lines go round in a circle"
        ]
    );
    // Round three chains, each line named.
    assert_eq!(problems(&[(0, 1), (1, 2), (2, 0)], &[0, 1, 2]).len(), 3);
    // A line leading into a circle is not in it.
    assert_eq!(problems(&[(2, 0), (0, 1), (1, 0)], &[0, 1, 2]).len(), 2);
}

#[test]
fn a_manifest_names_its_dynamic_joints() {
    let parsed = manifest(
        "dynamic tail_root\ndynamic hair 0\n\
         dynamic cape_l wag 0 stiffness 120 damping 9 nocollide\ndynamic cape_r 800 wag 0\n\
         dynamic strand limit 4 clearance 1.5\nlayer strand over cape_l",
    );
    assert_eq!(parsed.layers, [("strand".to_owned(), "cape_l".to_owned())]);
    let default = dynamics::Settings::default();
    assert_eq!(
        parsed.dynamic,
        [
            ("tail_root".to_owned(), default),
            (
                "hair".to_owned(),
                dynamics::Settings {
                    gravity: 0.0,
                    ..default
                }
            ),
            (
                "cape_l".to_owned(),
                dynamics::Settings {
                    wag: 0.0,
                    stiffness: 120.0,
                    damping: 9.0,
                    collide: false,
                    ..default
                }
            ),
            (
                "cape_r".to_owned(),
                dynamics::Settings {
                    gravity: 800.0,
                    wag: 0.0,
                    ..default
                }
            ),
            (
                "strand".to_owned(),
                dynamics::Settings {
                    limit: 4.0,
                    clearance: 1.5,
                    ..default
                }
            ),
        ]
    );
    for bad in [
        "dynamic tail limit -1",
        "dynamic tail limit 181",
        "dynamic tail heavy",
        "dynamic tail wag",
        "dynamic tail stiffness 0",
        "dynamic tail damping -1",
        "dynamic tail 0 1",
    ] {
        assert!(Manifest::parse(bad).is_err(), "{bad}");
    }
    let rig = RigModel::fit(rig(), "", &manifest("dynamic nothing"), &driver()).unwrap();
    assert!(rig.chains.is_empty());
}

#[test]
fn gravity_hangs_a_tail_and_a_dynamic_line_can_turn_it_off() {
    let still = vec![identity_rows(); 3];
    let tip_drop = |text: &str| {
        let rig = tailed_with(text);
        let mut scratch = Scratch::default();
        rig.pose_after(&still, None, &mut scratch, None);
        for _ in 0..240 {
            rig.pose_after(&still, None, &mut scratch, Some(1.0 / 60.0));
        }
        rig.joints[4].bind.translation.z - scratch.pose[4].translation.z
    };
    // The game's gravity against the spring: the tip hangs several units lower.
    let hanging = tip_drop("dynamic tail_root");
    assert!(hanging > 5.0 && hanging < 30.0, "{hanging}");
    // `dynamic <joint> 0`: no gravity, the tip stays level with its rest.
    let level = tip_drop("dynamic tail_root 0");
    assert!(level.abs() < 1.0, "{level}");
}

/// A change of animation moves the body far at once: a limited chain's bones take that
/// up over several frames, so its tip swings instead of jumping.
#[test]
fn a_limited_chain_takes_up_a_jump_of_the_body_gradually() {
    let rig = tailed_with(
        "dynamic tail_root gravity 0 wag 0 stiffness 200 damping 20 limit 20 nocollide",
    );
    let step = 1.0 / 142.0;
    let direction = |scratch: &Scratch| {
        (scratch.pose[4].translation - scratch.pose[3].translation).normalize_or_zero()
    };
    let mut bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    rig.pose_after(&bones, None, &mut scratch, None);
    rig.pose_after(&bones, None, &mut scratch, Some(step));
    let mut last = direction(&scratch);
    let mut turns = Vec::new();
    // Ten units in the game's tenth of a second blend.
    for tick in 0..30 {
        if tick < 14 {
            for row in &mut bones {
                row[0][3] += 10.0 / 14.0;
            }
        }
        rig.pose_after(&bones, None, &mut scratch, Some(step));
        let now = direction(&scratch);
        turns.push(now.dot(last).clamp(-1.0, 1.0).acos().to_degrees());
        last = now;
    }
    let largest = turns.iter().copied().fold(0.0, f32::max);
    assert!(largest > 0.2, "the chain did not answer: {turns:?}");
    assert!(
        turns[0] < largest * 0.5,
        "the first frame took the jump at once: {turns:?}"
    );
}

/// An installed rig model's dynamic chains settled in its bind pose: how far each
/// chain's tip stands above (or below) its anchor, and at what angle from level, with
/// the manifest's gravity and with none. `SJK_TEST_PACK` names the pack holding it
/// (default `rig_<name>.pk3`).
#[test]
#[ignore = "reads the retail skeleton and an installed rig model (JKA_GAME_DATA, SJK_TEST_RIG)"]
fn dynamic_chains_of_an_installed_rig() {
    let (Some(data), Ok(name)) = (
        std::env::var_os("JKA_GAME_DATA").map(std::path::PathBuf::from),
        std::env::var("SJK_TEST_RIG"),
    ) else {
        println!("JKA_GAME_DATA or SJK_TEST_RIG is not set: skipped");
        return;
    };
    let pack = std::env::var("SJK_TEST_PACK").unwrap_or_else(|_| format!("rig_{name}.pk3"));
    let mut vfs = VirtualFileSystem::new();
    let base = data.join("base");
    for entry in std::fs::read_dir(&base).unwrap().flatten() {
        let path = entry.path();
        let file = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        if file.starts_with("assets") && file.ends_with(".pk3") || file == pack.to_ascii_lowercase()
        {
            vfs.mount_pk3(&path).unwrap();
        }
    }
    let read = |path: &str| vfs.read(path).unwrap().unwrap().bytes;
    let gla = Gla::parse(&read("models/players/_humanoid/_humanoid.gla")).unwrap();
    let glm = sjk_model::Glm::parse(&read("models/players/kyle/model.glm")).unwrap();
    let directory = format!("models/players/{name}");
    let variant = std::env::var("SJK_TEST_VARIANT").unwrap_or_else(|_| "default".to_owned());
    let manifest = super::manifest(&vfs, &directory, &variant)
        .unwrap()
        .expect("a rig model");
    let mut weightless = manifest.clone();
    for line in &mut weightless.dynamic {
        line.1.gravity = 0.0;
    }
    let bones = vec![identity_rows(); gla.bones.len()];
    for (label, manifest) in [("manifest", &manifest), ("no gravity", &weightless)] {
        let rig = RigModel::load(&vfs, &directory, manifest, &gla, &glm).unwrap();
        for capsule in &rig.chains.capsules {
            println!(
                "capsule {} to {}: radius {:.1}",
                rig.joints[capsule.from].name, rig.joints[capsule.to].name, capsule.radius
            );
        }
        let mut scratch = Scratch::default();
        rig.pose_after(&bones, None, &mut scratch, None);
        for _ in 0..600 {
            rig.pose_after(&bones, None, &mut scratch, Some(1.0 / 60.0));
        }
        let Some(&tip) = rig.chains.joints.last() else {
            println!("{name}: no dynamic chain");
            return;
        };
        let mut anchor = tip;
        while let Some(parent) = rig.joints[anchor].parent {
            anchor = parent;
            if !rig.chains.joints.contains(&anchor) {
                break;
            }
        }
        let offset = scratch.pose[tip].translation - scratch.pose[anchor].translation;
        let level = offset.truncate().length();
        println!(
            "{name} ({label}): tip {:.1} units above its anchor, {:.0} degrees from level, \
             {:.1} long",
            offset.z,
            offset.z.atan2(level).to_degrees(),
            offset.length()
        );
    }
}

/// The driver's bones at `time` seconds of a hard sway and bob, and the sway's angle.
fn violent_sway(time: f32) -> ([[f32; 4]; 3], f32) {
    let angle = (0.4 * (std::f32::consts::TAU * 1.2 * time).sin()).clamp(-1.0, 1.0);
    let (sin, cos) = angle.sin_cos();
    let z = 3.0 * (std::f32::consts::TAU * 2.0 * time).sin();
    (
        [
            [cos, -sin, 0.0, 0.0],
            [sin, cos, 0.0, 0.0],
            [0.0, 0.0, 1.0, z],
        ],
        angle,
    )
}

/// Under [`violent_sway`], each joint that swings on a lever: its name, how far the middle
/// of what it carries gets from rest in units, and its largest step in a frame.
fn lever_motion(rig: &RigModel, bone_count: usize, frames: usize) -> Vec<(String, f32, f32)> {
    let step = 1.0 / 142.0;
    let swings = rig.chains.swings();
    let mut bones = vec![identity_rows(); bone_count];
    let mut scratch = Scratch::default();
    let mut last: Vec<Option<Vec3A>> = vec![None; swings.len()];
    let mut worst = vec![(0.0f32, 0.0f32); swings.len()];
    rig.pose_after(&bones, None, &mut scratch, None);
    for frame in 0..frames {
        let (rows, angle) = violent_sway(frame as f32 * step);
        bones.fill(rows);
        rig.pose_after(&bones, None, &mut scratch, Some(step));
        let undo = Quat::from_rotation_z(-angle);
        for (slot, &(joint, lever)) in swings.iter().enumerate() {
            let arm = undo * (scratch.pose[joint].matrix3 * lever);
            let rest = rig.joints[joint].bind.matrix3 * lever;
            worst[slot].0 = worst[slot].0.max((arm - rest).length());
            if let Some(before) = last[slot] {
                worst[slot].1 = worst[slot].1.max((arm - before).length());
            }
            last[slot] = Some(arm);
        }
    }
    swings
        .iter()
        .zip(worst)
        .map(|(&(joint, _), (off, turn))| (rig.joints[joint].name.clone(), off, turn))
        .collect()
}

/// For each (parent, child) of `joints`, while the driver `bones` named by `moved` sway
/// and bob for `frames` frames at 142 FPS: how far the bone strays from its bind
/// direction and the most it turns in one frame, in degrees, the sway's own turn taken
/// out.
fn largest_turn(
    rig: &RigModel,
    bones: &mut [[[f32; 4]; 3]],
    moved: &[usize],
    joints: &[(usize, usize)],
    frames: usize,
) -> Vec<(f32, f32)> {
    let step = 1.0 / 142.0;
    let mut scratch = Scratch::default();
    let mut last: Vec<Option<Vec3A>> = vec![None; joints.len()];
    let mut worst = vec![(0.0f32, 0.0f32); joints.len()];
    rig.pose_after(bones, None, &mut scratch, None);
    for frame in 0..frames {
        let (rows, angle) = violent_sway(frame as f32 * step);
        for &bone in moved {
            bones[bone] = rows;
        }
        rig.pose_after(bones, None, &mut scratch, Some(step));
        let undo = Quat::from_rotation_z(-angle);
        for (slot, &(parent, child)) in joints.iter().enumerate() {
            let bone = scratch.pose[child].translation - scratch.pose[parent].translation;
            let Some(direction) = bone.try_normalize() else {
                continue;
            };
            let direction = undo * direction;
            let bind = (rig.joints[child].bind.translation - rig.joints[parent].bind.translation)
                .try_normalize()
                .unwrap_or(direction);
            let off = direction.dot(bind).clamp(-1.0, 1.0).acos().to_degrees();
            worst[slot].0 = worst[slot].0.max(off);
            if let Some(before) = last[slot] {
                let turn = direction.dot(before).clamp(-1.0, 1.0).acos().to_degrees();
                worst[slot].1 = worst[slot].1.max(turn);
            }
            last[slot] = Some(direction);
        }
    }
    worst
}

/// [`rig`] with a chain that forks: a stem under the pelvis, and two branches of two
/// joints each under the stem, short ones like hair strands.
fn forked(text: &str) -> RigModel {
    let mut model = rig();
    let joint = |name: &str, parent, x: f32, z: f32, rest: [f32; 2]| GltfJoint {
        name: name.to_owned(),
        parent: Some(parent),
        bind: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, z]],
        inverse_bind: [
            [1.0, 0.0, 0.0, -x],
            [0.0, 1.0, 0.0, -1.0],
            [0.0, 0.0, 1.0, -z],
        ],
        rest: [
            [1.0, 0.0, 0.0, rest[0]],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, rest[1]],
        ],
    };
    model.joints.push(joint("root", 0, 0.0, -0.1, [0.0, -0.1]));
    model.joints.push(joint("stem", 2, 0.0, -0.2, [0.0, -0.1]));
    // On the stem's spot but for rounding, as some rigs stack joints.
    model.joints.push(joint("pin", 3, 1e-6, -0.2, [1e-6, 0.0]));
    for (name, parent, x, z, rest) in [
        ("left", 4, -0.03, -0.25, [-0.03, -0.05]),
        ("left_tip", 5, -0.06, -0.3, [-0.03, -0.05]),
        ("right", 4, 0.03, -0.25, [0.03, -0.05]),
        ("right_tip", 7, 0.06, -0.3, [0.03, -0.05]),
    ] {
        model.joints.push(joint(name, parent, x, z, rest));
    }
    RigModel::fit(model, "", &manifest(text), &driver()).unwrap()
}

const FORKED_BONES: [(usize, usize); 3] = [(2, 3), (5, 6), (7, 8)];

#[test]
fn a_joint_stacked_on_its_parent_and_a_fork_do_not_throw_a_chain_about() {
    let rig = forked("dynamic root gravity 0 wag 0 stiffness 400 damping 24 nocollide");
    let mut bones = vec![identity_rows(); 3];
    let worst = largest_turn(&rig, &mut bones, &[0, 1, 2], &FORKED_BONES, 142 * 4);
    for (bone, (_, turn)) in worst.into_iter().enumerate() {
        assert!(turn < 6.0, "bone {bone} jumps {turn} degrees in a frame");
    }
}

#[test]
fn a_chains_limit_holds_each_bone_near_its_animated_direction() {
    let rig = forked("dynamic root gravity 0 wag 0 stiffness 400 damping 24 limit 4 nocollide");
    let mut bones = vec![identity_rows(); 3];
    let worst = largest_turn(&rig, &mut bones, &[0, 1, 2], &FORKED_BONES, 142 * 4);
    // Each bone answers to its parent, so the limits add up down the chain: the stem
    // has one above it, the branches' tips three.
    for (bone, (off, turn)) in worst.into_iter().enumerate() {
        assert!(off < 4.0 * 3.0, "bone {bone} strays {off} degrees");
        // Turns add up the same way. A hard clamp would throw a single bone across its
        // limit, twice 4 degrees, as the lag turns round.
        assert!(turn < 2.0, "bone {bone} jumps {turn} degrees in a frame");
    }
}

/// How far each dynamic joint of an installed rig strays and how fast it turns while
/// the body sways, worst first (`JKA_GAME_DATA`, `SJK_TEST_RIG`, `SJK_TEST_PACK`).
#[test]
#[ignore = "needs an installed rig model and the game's files"]
fn dynamic_chain_motion_of_an_installed_rig() {
    let (Some(data), Ok(name)) = (
        std::env::var_os("JKA_GAME_DATA").map(std::path::PathBuf::from),
        std::env::var("SJK_TEST_RIG"),
    ) else {
        println!("JKA_GAME_DATA or SJK_TEST_RIG is not set: skipped");
        return;
    };
    let pack = std::env::var("SJK_TEST_PACK").unwrap_or_else(|_| format!("rig_{name}.pk3"));
    let mut vfs = VirtualFileSystem::new();
    for entry in std::fs::read_dir(data.join("base")).unwrap().flatten() {
        let path = entry.path();
        let file = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        if file.starts_with("assets") && file.ends_with(".pk3") || file == pack.to_ascii_lowercase()
        {
            vfs.mount_pk3(&path).unwrap();
        }
    }
    let read = |path: &str| vfs.read(path).unwrap().unwrap().bytes;
    let gla = Gla::parse(&read("models/players/_humanoid/_humanoid.gla")).unwrap();
    let glm = sjk_model::Glm::parse(&read("models/players/kyle/model.glm")).unwrap();
    let directory = format!("models/players/{name}");
    let manifest = super::manifest(&vfs, &directory, "default")
        .unwrap()
        .expect("a rig model");
    let rig = RigModel::load(&vfs, &directory, &manifest, &gla, &glm).unwrap();
    let pairs: Vec<(usize, usize)> = rig
        .chains
        .joints
        .iter()
        .filter_map(|&joint| Some((rig.joints[joint].parent?, joint)))
        // A joint stacked on its parent's spot has no direction to measure.
        .filter(|&(parent, joint)| {
            (rig.joints[joint].bind.translation - rig.joints[parent].bind.translation).length()
                > 0.05
        })
        .collect();
    let mut bones = vec![identity_rows(); gla.bones.len()];
    let every: Vec<usize> = (0..gla.bones.len()).collect();
    let worst = largest_turn(&rig, &mut bones, &every, &pairs, 142 * 6);
    let mut lines: Vec<(f32, f32, &str)> = worst
        .iter()
        .zip(&pairs)
        .map(|(&(off, turn), &(_, joint))| (off, turn, rig.joints[joint].name.as_str()))
        .collect();
    lines.sort_by(|a, b| b.1.total_cmp(&a.1));
    let count = lines.len().max(1) as f32;
    println!(
        "{name}: {} joints, worst of each averaged: off {:.1} degrees, turn a frame {:.2} degrees",
        lines.len(),
        lines.iter().map(|line| line.0).sum::<f32>() / count,
        lines.iter().map(|line| line.1).sum::<f32>() / count,
    );
    for (off, turn, joint) in lines.iter().take(12) {
        println!("  {joint}: off {off:.1} degrees, turn a frame {turn:.2} degrees");
    }
    for (joint, off, turn) in lever_motion(&rig, gla.bones.len(), 142 * 6) {
        println!("  lever {joint}: off {off:.2} units, step a frame {turn:.3} units");
    }
    legs_through_chains(&rig, &gla);
    let config =
        sjk_model::AnimationConfig::parse(&read("models/players/_humanoid/animation.cfg")).unwrap();
    let sequence = |name: &str| config.get(name).unwrap();
    for name in ["BOTH_WALK1", "BOTH_RUN1"] {
        levers_in_a_cycle(&rig, &gla, name, sequence(name));
    }
    chains_on_a_change(&rig, &gla, sequence("BOTH_RUN1"), sequence("BOTH_STAND1"));
}

/// `a` and `b`'s bones `share` of the way across.
fn mixed(a: &[[[f32; 4]; 3]], b: &[[[f32; 4]; 3]], share: f32) -> Vec<[[f32; 4]; 3]> {
    a.iter()
        .zip(b)
        .map(|(a, b)| {
            std::array::from_fn(|row| {
                std::array::from_fn(|col| a[row][col] + (b[row][col] - a[row][col]) * share)
            })
        })
        .collect()
}

/// `sequence`'s bones `time` seconds in, between its frames, looping.
fn played(gla: &Gla, sequence: &sjk_model::AnimationSequence, time: f32) -> Vec<[[f32; 4]; 3]> {
    let spot = time * sequence.frames_per_second.abs().max(1.0);
    let index = spot.floor() as usize % sequence.frame_count;
    let frame = |index: usize| {
        gla.frame_matrices(sequence.first_frame + index % sequence.frame_count)
            .unwrap()
    };
    mixed(&frame(index), &frame(index + 1), spot.fract())
}

/// Play `sequence` and print how far each lever joint's vertices get from where the
/// animation alone puts them, after a cycle to settle.
fn levers_in_a_cycle(
    rig: &RigModel,
    gla: &Gla,
    name: &str,
    sequence: &sjk_model::AnimationSequence,
) {
    let step = 1.0 / 142.0;
    let cycle = sequence.frame_count as f32 / sequence.frames_per_second.abs().max(1.0);
    let ticks = (cycle * 3.0 / step) as usize;
    let swings = rig.chains.swings();
    let carried: Vec<Vec<&Vertex>> = swings
        .iter()
        .map(|&(joint, _)| {
            rig.surfaces
                .iter()
                .flat_map(|surface| &surface.vertices)
                .filter(|vertex| {
                    vertex
                        .joints
                        .iter()
                        .zip(&vertex.weights)
                        .any(|(j, weight)| usize::from(*j) == joint && *weight > 0.05)
                })
                .collect()
        })
        .collect();
    let mut live = Scratch::default();
    let mut still = Scratch::default();
    let mut moved = vec![0.0f32; swings.len()];
    rig.pose_after(&played(gla, sequence, 0.0), None, &mut live, None);
    for tick in 0..ticks {
        let bones = played(gla, sequence, tick as f32 * step);
        rig.pose_after(&bones, None, &mut live, Some(step));
        rig.pose_after(&bones, None, &mut still, None);
        if tick > ticks / 3 {
            for (worst, vertices) in moved.iter_mut().zip(&carried) {
                for vertex in vertices {
                    let now = Vec3A::from_array(skin(vertex, &live.palette).0);
                    let rest = Vec3A::from_array(skin(vertex, &still.palette).0);
                    *worst = worst.max((now - rest).length());
                }
            }
        }
    }
    for (&(joint, _), worst) in swings.iter().zip(&moved) {
        println!(
            "  {name}: skin of {} up to {worst:.2} units",
            rig.joints[joint].name
        );
    }
}

/// Run, stop and start again over the game's usual blend, and print how each bone of the
/// chains turns in the half second after each change, under the first joint up that no
/// chain moves: its largest turn in a tick and how often it turned back.
fn chains_on_a_change(
    rig: &RigModel,
    gla: &Gla,
    run: &sjk_model::AnimationSequence,
    stand: &sjk_model::AnimationSequence,
) {
    const BLEND: f32 = 0.1;
    let (stop_at, start_at, end) = (2.0, 4.0, 6.0);
    let step = 1.0 / 142.0;
    let bones_at = |time: f32| {
        let standing = if time < stop_at {
            0.0
        } else if time < start_at {
            ((time - stop_at) / BLEND).min(1.0)
        } else {
            1.0 - ((time - start_at) / BLEND).min(1.0)
        };
        mixed(&played(gla, run, time), &played(gla, stand, time), standing)
    };
    let pairs: Vec<(usize, usize, usize)> = rig
        .chains
        .joints
        .iter()
        .filter_map(|&joint| {
            let parent = rig.joints[joint].parent?;
            let mut anchor = parent;
            while rig.chains.joints.contains(&anchor) {
                anchor = rig.joints[anchor].parent?;
            }
            let bone = rig.joints[joint].bind.translation - rig.joints[parent].bind.translation;
            (bone.length() > 0.05).then_some((anchor, parent, joint))
        })
        .collect();
    let mut scratch = Scratch::default();
    rig.pose_after(&bones_at(0.0), None, &mut scratch, None);
    let mut last: Vec<Vec3A> = Vec::new();
    let mut swing = vec![Vec3A::ZERO; pairs.len()];
    let mut largest = vec![0.0f32; pairs.len()];
    let mut backs = vec![0u32; pairs.len()];
    for tick in 0..(end / step) as usize {
        let time = tick as f32 * step;
        rig.pose_after(&bones_at(time), None, &mut scratch, Some(step));
        let now: Vec<Vec3A> = pairs
            .iter()
            .map(|&(anchor, parent, joint)| {
                let bone = scratch.pose[joint].translation - scratch.pose[parent].translation;
                (scratch.pose[anchor].matrix3.inverse() * bone).normalize_or_zero()
            })
            .collect();
        if last.len() == now.len() {
            let watched = (stop_at..stop_at + 0.5).contains(&time)
                || (start_at..start_at + 0.5).contains(&time);
            for slot in 0..now.len() {
                let change = now[slot] - last[slot];
                if watched {
                    largest[slot] = largest[slot].max(change.length().to_degrees());
                    if change.dot(swing[slot]) < 0.0 && change.length() > 0.002 {
                        backs[slot] += 1;
                    }
                }
                swing[slot] = change;
            }
        }
        last = now;
    }
    let mut order: Vec<usize> = (0..pairs.len()).collect();
    order.sort_by(|a, b| largest[*b].total_cmp(&largest[*a]));
    for &slot in order.iter().take(5) {
        println!(
            "  stop and start: {}: {:.1} degrees a tick, turned back {} times",
            rig.joints[pairs[slot].2].name, largest[slot], backs[slot]
        );
    }
}

/// Stride the driver's legs 35 degrees each way and print how deep each chain's joints and
/// bones' middles get into the capsules that do not exempt them, in game units.
fn legs_through_chains(rig: &RigModel, gla: &Gla) {
    let mut bones = vec![identity_rows(); gla.bones.len()];
    let leg = |bone: &sjk_model::GlaBone, side: &str| {
        ["femur", "tibia", "talus", "foot"]
            .iter()
            .any(|part| bone.name.starts_with(&format!("{side}{part}")))
    };
    let hip = |side: &str| {
        let bone = gla
            .bones
            .iter()
            .find(|bone| bone.name.starts_with(&format!("{side}femur")))
            .unwrap();
        Vec3A::new(
            bone.base_pose[0][3],
            bone.base_pose[1][3],
            bone.base_pose[2][3],
        )
    };
    let step = 1.0 / 142.0;
    let mut scratch = Scratch::default();
    rig.pose_after(&bones, None, &mut scratch, None);
    let chains = &rig.chains;
    let mut lowest = f32::MAX;
    let mut layered = 0;
    let exempt: usize = chains
        .ignored
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .sum();
    println!(
        "{} capsules, {} chain joints, {exempt} exemptions; radii {:?}",
        chains.capsules.len(),
        chains.joints.len(),
        chains
            .capsules
            .iter()
            .map(|capsule| (capsule.radius * 10.0).round() / 10.0)
            .collect::<Vec<_>>(),
    );
    let mut deepest = vec![0.0f32; chains.joints.len()];
    let mut moved = 0.0f32;
    let mut first: Vec<Vec3A> = Vec::new();
    for frame in 0..142 * 4 {
        let phase = std::f32::consts::TAU * 1.2 * frame as f32 * step;
        for (side, sign) in [("l", 1.0), ("r", -1.0)] {
            let angle = sign * 35f32.to_radians() * phase.sin();
            let (sin, cos) = angle.sin_cos();
            let at = hip(side);
            // About the side axis through the hip.
            let rows = [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, cos, -sin, at.y - cos * at.y + sin * at.z],
                [0.0, sin, cos, at.z - sin * at.y - cos * at.z],
            ];
            for (index, bone) in gla.bones.iter().enumerate() {
                if leg(bone, side) {
                    bones[index] = rows;
                }
            }
        }
        // The trunk sways too, so a cape swings against what lies on it.
        let sway = (0.3 * (std::f32::consts::TAU * 0.9 * frame as f32 * step).sin()).sin_cos();
        let bob = 3.0 * (std::f32::consts::TAU * 2.0 * frame as f32 * step).sin();
        for (index, bone) in gla.bones.iter().enumerate() {
            if !leg(bone, "l") && !leg(bone, "r") {
                bones[index] = [
                    [sway.1, 0.0, sway.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [-sway.0, 0.0, sway.1, bob],
                ];
            }
        }
        rig.pose_after(&bones, None, &mut scratch, Some(step));
        let margins = chains.layer_margins(&scratch.pose);
        layered = margins.len();
        lowest = margins.iter().fold(lowest, |low, margin| low.min(*margin));
        if first.is_empty() {
            first = chains
                .capsules
                .iter()
                .map(|capsule| scratch.pose[capsule.to].translation)
                .collect();
        }
        for (capsule, start) in chains.capsules.iter().zip(&first) {
            moved = moved.max((scratch.pose[capsule.to].translation - *start).length());
        }
        for (slot, &joint) in chains.joints.iter().enumerate() {
            let Some(parent) = rig.joints[joint].parent else {
                continue;
            };
            let tip = scratch.pose[joint].translation;
            let middle = (scratch.pose[parent].translation + tip) / 2.0;
            for (bit, capsule) in chains.capsules.iter().enumerate() {
                if chains.ignored[slot] & (1 << bit) != 0 {
                    continue;
                }
                let from = scratch.pose[capsule.from].translation;
                let to = scratch.pose[capsule.to].translation;
                for sample in [tip, middle] {
                    let near = dynamics::closest(from, to, sample);
                    let depth = capsule.radius - (sample - near).length();
                    deepest[slot] = deepest[slot].max(depth);
                }
            }
        }
    }
    println!("layers: {layered} joints layered; lowest margin {lowest:.2} units");
    let mut lines: Vec<(f32, &str)> = deepest
        .iter()
        .zip(&chains.joints)
        .map(|(depth, joint)| (*depth, rig.joints[*joint].name.as_str()))
        .collect();
    lines.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!(
        "stride: knees travel up to {moved:.1} units; {} joints get in; deepest {:?}",
        lines.iter().filter(|line| line.0 > 0.0).count(),
        lines
            .iter()
            .take(6)
            .map(|(depth, name)| format!("{name} {depth:.2}"))
            .collect::<Vec<_>>(),
    );
}

/// The test rig fitted to the test driver, as a stock skeleton's emote source.
fn stock_emotes(model: GltfModel) -> StockEmotes {
    let source = Arc::new(RigModel::fit(model, "", &manifest(""), &driver()).unwrap());
    StockEmotes::of_source(&driver(), source).expect("the driver follows the rig")
}

fn full(play: Option<ClipPlay>) -> ClipPlay {
    ClipPlay {
        weight: 1.0,
        ..play.expect("the clip")
    }
}

#[test]
fn a_stock_model_plays_a_rig_clip_on_its_own_bones() {
    let stock = stock_emotes(rig());
    let driver_bones = vec![identity_rows(); 3];
    let mut scratch = StockScratch::default();
    // At its first key the clip is the rig's bind pose, which is the driver's base
    // pose: every bone matrix stays the identity.
    assert!(stock.pose(
        &driver_bones,
        full(stock.play("Nod", 0.0, None, false)),
        &mut scratch
    ));
    for matrix in &scratch.matrices {
        for (row, expected) in matrix.iter().zip(identity_rows()) {
            for (a, b) in row.iter().zip(expected) {
                assert!((a - b).abs() < 1e-4, "{:?}", scratch.matrices);
            }
        }
    }
    // At its end the head's quarter turn about the game's +Z turns the cranium
    // about its own base position (0, 0, 60); the pelvis stays.
    let play = stock.play("Nod", 1.0, None, false);
    assert_eq!(play.map(|play| play.weight), Some(1.0));
    assert!(stock.pose(&driver_bones, play.unwrap(), &mut scratch));
    let cranium = affine(&scratch.matrices[2]);
    let front = cranium.transform_point3a(Vec3A::new(0.0, -4.0, 64.0));
    assert!(
        (front - Vec3A::new(4.0, 0.0, 64.0)).length() < 1e-3,
        "{front:?}"
    );
    let pivot = cranium.transform_point3a(Vec3A::new(0.0, 0.0, 60.0));
    assert!(
        (pivot - Vec3A::new(0.0, 0.0, 60.0)).length() < 1e-3,
        "{pivot:?}"
    );
    let pelvis = affine(&scratch.matrices[1]).transform_point3a(Vec3A::new(1.0, 0.0, 40.0));
    assert!(
        (pelvis - Vec3A::new(1.0, 0.0, 40.0)).length() < 1e-3,
        "{pelvis:?}"
    );
    // Half faded in, half way round.
    let half = ClipPlay {
        weight: 0.5,
        ..play.unwrap()
    };
    assert!(stock.pose(&driver_bones, half, &mut scratch));
    let front = affine(&scratch.matrices[2]).transform_point3a(Vec3A::new(0.0, -4.0, 64.0));
    let side = 4.0 * std::f32::consts::FRAC_1_SQRT_2;
    assert!(
        (front - Vec3A::new(side, -side, 64.0)).length() < 1e-3,
        "{front:?}"
    );
    // Faded out it leaves the driver's matrices alone; an unknown clip plays nothing.
    let none = ClipPlay {
        weight: 0.0,
        ..play.unwrap()
    };
    assert!(!stock.pose(&driver_bones, none, &mut scratch));
    assert!(stock.play("Wave", 0.0, None, false).is_none());
    // Matrices of another skeleton are not posed.
    assert!(!stock.pose(&driver_bones[..2], play.unwrap(), &mut scratch));
}

#[test]
fn a_stock_pelvis_follows_the_clips_pelvis_and_carries_its_children() {
    // The rig crouches: its pelvis drops from 1 m to 0.5 m over a second.
    let mut model = rig();
    model.animations = vec![GltfAnimation {
        name: "Crouch".into(),
        channels: vec![GltfChannel {
            joint: 0,
            path: GltfPath::Translation,
            times: vec![0.0, 1.0],
            values: vec![[0.0, 1.0, 0.0, 0.0], [0.0, 0.5, 0.0, 0.0]],
            step: false,
        }],
    }];
    let stock = stock_emotes(model);
    let mut scratch = StockScratch::default();
    // The driver itself stands 3 units further front (-Y) than its base pose.
    let driver_bones = vec![translated(0.0, -3.0, 0.0); 3];
    assert!(stock.pose(
        &driver_bones,
        full(stock.play("Crouch", 1.0, None, false)),
        &mut scratch
    ));
    // Half a metre at 40 units a metre: 20 units down, from where the driver stands.
    let pelvis = affine(&scratch.matrices[1]).transform_point3a(Vec3A::new(0.0, 0.0, 40.0));
    assert!(
        (pelvis - Vec3A::new(0.0, -3.0, 20.0)).length() < 1e-3,
        "{pelvis:?}"
    );
    let cranium = affine(&scratch.matrices[2]).transform_point3a(Vec3A::new(0.0, 0.0, 60.0));
    assert!(
        (cranium - Vec3A::new(0.0, -3.0, 40.0)).length() < 1e-3,
        "{cranium:?}"
    );
    // The root bone, which the map leaves out, keeps the driver's matrix.
    assert_eq!(scratch.matrices[0], translated(0.0, -3.0, 0.0));
}

#[test]
fn stock_bones_are_posed_parents_first() {
    assert_eq!(stock::bone_order(&[Some(2), None, Some(1)]), vec![1, 2, 0]);
    // A cycle or a parent past the end still places every bone once.
    let order = stock::bone_order(&[Some(1), Some(0), Some(9)]);
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![0, 1, 2]);
}

/// The test rig with a `hip` joint half a metre under the pelvis.
fn rig_with_hip() -> GltfModel {
    let mut model = rig();
    model.joints.push(GltfJoint {
        name: "hip".into(),
        parent: Some(0),
        bind: translated(0.0, 0.5, 0.0),
        inverse_bind: translated(0.0, -0.5, 0.0),
        rest: translated(0.0, -0.5, 0.0),
    });
    model
}

#[test]
fn the_pelvis_vertices_make_a_capsule_for_the_dynamic_chains() {
    let mut model = rig_with_hip();
    // A two-joint tail behind the pelvis, moving on its own.
    let tail = |name: &str, parent, z: f32| GltfJoint {
        name: name.to_owned(),
        parent: Some(parent),
        bind: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, z],
        ],
        inverse_bind: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, -1.0],
            [0.0, 0.0, 1.0, -z],
        ],
        rest: translated(0.0, 0.0, -0.3),
    };
    model.joints.push(tail("tail_root", 0, -0.3));
    model.joints.push(tail("tail_1", 3, -0.6));
    // A ring of vertices 0.2 m round the pelvis-to-hip segment, on the pelvis.
    let primitive = &mut model.primitives[0];
    for step in 0..12 {
        let angle = step as f32 * std::f32::consts::TAU / 12.0;
        for height in [0.6, 0.8] {
            primitive
                .positions
                .push([0.2 * angle.cos(), height, 0.2 * angle.sin()]);
            primitive.normals.push([0.0, 0.0, 1.0]);
            primitive.texture_coordinates.push([0.0; 2]);
            primitive.joints.push([0, 0, 0, 0]);
            primitive.weights.push([1.0, 0.0, 0.0, 0.0]);
        }
    }
    let rig = RigModel::fit(
        model.clone(),
        "",
        &manifest("map pelvis pelvis hip cranium\ndynamic tail_root"),
        &driver(),
    )
    .unwrap();
    // 0.2 m at 40 units a metre.
    assert_eq!(rig.chains.capsules.len(), 1, "{:?}", rig.chains.capsules);
    let capsule = rig.chains.capsules[0];
    assert_eq!((capsule.from, capsule.to), (0, 2));
    assert!((capsule.radius - 8.0).abs() < 0.1, "{capsule:?}");
    // Without a dynamic chain no capsule is measured.
    let still = RigModel::fit(
        model,
        "",
        &manifest("map pelvis pelvis hip cranium"),
        &driver(),
    )
    .unwrap();
    assert!(still.chains.capsules.is_empty());
}

/// A rig model beside a stock GLM model in the same `_humanoid` frames, as the game
/// poses them: each pose written as Wavefront OBJ files (`<pose>_stock.obj`,
/// `<pose>_rig.obj`, model space in game units, one object per surface named after
/// its shader) into `SJK_TEST_OUT` (default `target/rig-lineup`), with both models'
/// extents and the joints' bind places against the JKA bones they follow printed, to
/// compare proportions. `SJK_TEST_STOCK` names the stock folder (default `kyle`);
/// `GameData/SJK/assets` packs are mounted too (hub models).
#[test]
#[ignore = "reads the installed game data (JKA_GAME_DATA, SJK_TEST_RIG, SJK_TEST_STOCK)"]
fn lineup_beside_a_stock_model() {
    let (Some(data), Ok(name)) = (
        std::env::var_os("JKA_GAME_DATA").map(std::path::PathBuf::from),
        std::env::var("SJK_TEST_RIG"),
    ) else {
        println!("JKA_GAME_DATA or SJK_TEST_RIG is not set: skipped");
        return;
    };
    let stock = std::env::var("SJK_TEST_STOCK").unwrap_or_else(|_| "kyle".to_owned());
    let out = std::env::var_os("SJK_TEST_OUT")
        .map_or_else(|| std::path::PathBuf::from("target/rig-lineup"), Into::into);
    std::fs::create_dir_all(&out).unwrap();
    let mut vfs = VirtualFileSystem::new();
    let base = data.join("base");
    let mut packs: Vec<std::path::PathBuf> = std::fs::read_dir(&base)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let file = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase();
            file.starts_with("assets") && file.ends_with(".pk3")
                || file == format!("rig_{name}.pk3")
        })
        .collect();
    if let Ok(entries) = std::fs::read_dir(data.join("SJK").join("assets")) {
        packs.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("pk3"))
        }));
    }
    packs.sort();
    for pack in &packs {
        vfs.mount_pk3(pack).unwrap();
    }
    vfs.mount_directory(&base).unwrap();
    let read = |path: &str| vfs.read(path).unwrap().unwrap().bytes;
    let gla = Gla::parse(&read("models/players/_humanoid/_humanoid.gla")).unwrap();
    let kyle = sjk_model::Glm::parse(&read("models/players/kyle/model.glm")).unwrap();
    let stock_glm =
        sjk_model::Glm::parse(&read(&format!("models/players/{stock}/model.glm"))).unwrap();
    let stock_skin =
        sjk_model::Skin::parse(&read(&format!("models/players/{stock}/model_default.skin")));
    let config =
        sjk_model::AnimationConfig::parse(&read("models/players/_humanoid/animation.cfg")).unwrap();
    let directory = format!("models/players/{name}");
    let variant = std::env::var("SJK_TEST_VARIANT").unwrap_or_else(|_| "default".to_owned());
    let manifest = super::manifest(&vfs, &directory, &variant)
        .unwrap()
        .expect("a rig model");
    let rig = RigModel::load(&vfs, &directory, &manifest, &gla, &kyle).unwrap();

    // Proportions: each mapped joint's bind place against its JKA bone's base place.
    let bone_at = |bone: usize| affine(&gla.bones[bone].base_pose).translation;
    println!("joint -> bone: rig bind place | JKA base place (game units)");
    for (index, driven) in rig.driven.iter().enumerate() {
        let Some(driven) = driven else { continue };
        let joint = &rig.joints[index];
        let rig_at = joint.bind.translation;
        let jka_at = bone_at(driven.bone);
        println!(
            "{:>22} -> {:<14} {:>7.2} {:>7.2} {:>7.2} | {:>7.2} {:>7.2} {:>7.2}",
            joint.name,
            gla.bones[driven.bone].name,
            rig_at.x,
            rig_at.y,
            rig_at.z,
            jka_at.x,
            jka_at.y,
            jka_at.z
        );
    }

    type Mesh = Vec<(String, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[u32; 3]>)>;
    let write = |file: &str, mesh: &Mesh| {
        let mut text = String::new();
        let mut first = 1;
        for (shader, positions, uvs, triangles) in mesh {
            text += &format!("o {}\nusemtl {shader}\n", shader.replace(['/', ' '], "_"));
            for p in positions {
                text += &format!("v {} {} {}\n", p[0], p[1], p[2]);
            }
            for uv in uvs {
                text += &format!("vt {} {}\n", uv[0], 1.0 - uv[1]);
            }
            // Game fronts wind clockwise, OBJ's counter-clockwise.
            for t in triangles {
                let [a, b, c] = t.map(|i| i + first);
                text += &format!("f {a}/{a} {c}/{c} {b}/{b}\n");
            }
            first += u32::try_from(positions.len()).unwrap();
        }
        std::fs::write(out.join(file), text).unwrap();
    };
    let extent = |mesh: &Mesh| {
        let mut low = [f32::MAX; 3];
        let mut high = [f32::MIN; 3];
        for p in mesh.iter().flat_map(|(_, positions, _, _)| positions) {
            for axis in 0..3 {
                low[axis] = low[axis].min(p[axis]);
                high[axis] = high[axis].max(p[axis]);
            }
        }
        format!(
            "x {:.1}..{:.1} y {:.1}..{:.1} z {:.1}..{:.1} (height {:.1})",
            low[0],
            high[0],
            low[1],
            high[1],
            low[2],
            high[2],
            high[2] - low[2]
        )
    };
    let identity = vec![identity_rows(); gla.bones.len()];
    let mut poses = vec![("base".to_owned(), identity)];
    for (animation, frame) in [
        ("BOTH_STAND1", 0),
        ("BOTH_STAND2", 0),
        ("BOTH_WALK1", 3),
        ("BOTH_RUN1", 4),
        ("BOTH_CROUCH1IDLE", 0),
        ("BOTH_JUMP1", 6),
        ("BOTH_A2_T__B_", 0),
    ] {
        let sequence = config.get(animation).unwrap();
        poses.push((
            animation.to_ascii_lowercase(),
            gla.frame_matrices(sequence.first_frame + frame).unwrap(),
        ));
    }
    let mut scratch = Scratch::default();
    for (pose, matrices) in &poses {
        let stock_mesh: Mesh = stock_glm
            .skin_pose_matrices(&stock_skin, 0, matrices)
            .unwrap()
            .into_iter()
            .filter(|s| s.shader.is_some() && !s.name.contains("_cap_") && !s.name.starts_with('*'))
            .map(|s| {
                (
                    s.shader.unwrap_or_default(),
                    s.vertices.iter().map(|v| v.position).collect(),
                    s.vertices.iter().map(|v| v.texture_coordinates).collect(),
                    s.triangles,
                )
            })
            .collect();
        rig.pose_after(matrices, None, &mut scratch, None);
        let rig_mesh: Mesh = rig
            .surfaces
            .iter()
            .map(|s| {
                (
                    s.shader.clone(),
                    s.vertices
                        .iter()
                        .map(|v| skin(v, &scratch.palette).0)
                        .collect(),
                    s.vertices.iter().map(|v| v.texture_coordinates).collect(),
                    s.indices
                        .chunks_exact(3)
                        .map(|t| [t[0], t[1], t[2]])
                        .collect(),
                )
            })
            .collect();
        println!("{pose}: {stock} {}", extent(&stock_mesh));
        println!("{pose}: {name} {}", extent(&rig_mesh));
        write(&format!("{pose}_stock.obj"), &stock_mesh);
        write(&format!("{pose}_rig.obj"), &rig_mesh);
    }
    println!("written to {}", out.display());
}

#[test]
fn a_variant_wears_its_own_manifest_else_the_default() {
    let mut vfs = VirtualFileSystem::new();
    vfs.mount_memory(
        "pack",
        [
            (
                "models/players/fox/sjk_rig.cfg",
                b"model model.gltf
scale 60
"
                .to_vec(),
            ),
            (
                "models/players/fox/sjk_rig_cloak.cfg",
                b"model model_cloak.gltf
scale 61
dynamic cape_0
"
                .to_vec(),
            ),
        ],
    )
    .unwrap();
    let wear = |variant: &str| {
        super::manifest(&vfs, "models/players/fox", variant)
            .unwrap()
            .expect("a rig model")
    };
    assert_eq!(wear("default").model.as_deref(), Some("model.gltf"));
    assert_eq!(wear("").scale, Some(60.0));
    let cloak = wear("Cloak");
    assert_eq!(cloak.model.as_deref(), Some("model_cloak.gltf"));
    assert_eq!(cloak.scale, Some(61.0));
    assert_eq!(cloak.dynamic.len(), 1);
    // A variant without a manifest of its own looks like the default.
    assert_eq!(wear("red").model.as_deref(), Some("model.gltf"));
    assert!(
        super::manifest(&vfs, "models/players/kyle", "default")
            .unwrap()
            .is_none()
    );
}

/// Per frame of a walk and a run: how far the rig's pelvis stands from the driver's
/// (constant when the rig rides the animation as the driver does) and its lowest point
/// against the driver's (the feet on the ground).
#[test]
#[ignore = "reads the retail Kyle and an installed rig model (JKA_GAME_DATA, SJK_TEST_RIG)"]
fn walk_against_the_driver() {
    let (Some(data), Ok(name)) = (
        std::env::var_os("JKA_GAME_DATA").map(std::path::PathBuf::from),
        std::env::var("SJK_TEST_RIG"),
    ) else {
        println!("JKA_GAME_DATA or SJK_TEST_RIG is not set: skipped");
        return;
    };
    let mut vfs = VirtualFileSystem::new();
    let base = data.join("base");
    for entry in std::fs::read_dir(&base).unwrap().flatten() {
        let path = entry.path();
        let file = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        if file.starts_with("assets") && file.ends_with(".pk3") || file == format!("rig_{name}.pk3")
        {
            vfs.mount_pk3(&path).unwrap();
        }
    }
    let read = |path: &str| vfs.read(path).unwrap().unwrap().bytes;
    let gla = Gla::parse(&read("models/players/_humanoid/_humanoid.gla")).unwrap();
    let glm = sjk_model::Glm::parse(&read("models/players/kyle/model.glm")).unwrap();
    let kyle_skin = sjk_model::Skin::parse(&read("models/players/kyle/model_default.skin"));
    let config =
        sjk_model::AnimationConfig::parse(&read("models/players/_humanoid/animation.cfg")).unwrap();
    let directory = format!("models/players/{name}");
    let variant = std::env::var("SJK_TEST_VARIANT").unwrap_or_else(|_| "default".to_owned());
    let manifest = super::manifest(&vfs, &directory, &variant)
        .unwrap()
        .expect("a rig model");
    let rig = RigModel::load(&vfs, &directory, &manifest, &gla, &glm).unwrap();
    let pelvis_joint = rig
        .driven
        .iter()
        .position(|d| d.is_some_and(|d| d.translate.is_some()))
        .unwrap();
    let (pelvis_bone, pelvis_base) = rig.pelvis.unwrap();
    let mut scratch = Scratch::default();
    for animation in ["BOTH_WALK1", "BOTH_RUN1"] {
        let sequence = config.get(animation).unwrap();
        let mut bobs = Vec::new();
        let mut feet = Vec::new();
        // Two cycles, the second measured, each frame in four steps of bones blended
        // between frames as the game draws them (the planting eases over time).
        let substeps = 4;
        let step = 1.0 / (sequence.frames_per_second.max(1.0) * substeps as f32);
        let mut first = true;
        for frame in 0..sequence.frame_count * 2 {
            let measured = frame >= sequence.frame_count;
            let frame = sequence.first_frame + frame % sequence.frame_count;
            let next =
                sequence.first_frame + (frame - sequence.first_frame + 1) % sequence.frame_count;
            let (from, to) = (
                gla.frame_matrices(frame).unwrap(),
                gla.frame_matrices(next).unwrap(),
            );
            for sub in 0..substeps {
                let t = sub as f32 / substeps as f32;
                let blended: Vec<[[f32; 4]; 3]> = from
                    .iter()
                    .zip(&to)
                    .map(|(a, b)| {
                        let mut m = *a;
                        for (row, (ra, rb)) in m.iter_mut().zip(a.iter().zip(b)) {
                            for (value, (va, vb)) in row.iter_mut().zip(ra.iter().zip(rb)) {
                                *value = va + (vb - va) * t;
                            }
                        }
                        m
                    })
                    .collect();
                rig.pose_after(&blended, None, &mut scratch, (!first).then_some(step));
                first = false;
            }
            if !measured {
                continue;
            }
            let matrices = from;
            let kyle = glm
                .skin(&gla, &kyle_skin, frame, 0)
                .unwrap()
                .into_iter()
                .filter(|s| {
                    s.shader.is_some() && !s.name.contains("_cap_") && !s.name.starts_with('*')
                })
                .flat_map(|s| s.vertices)
                .map(|v| v.position[2])
                .fold(f32::MAX, f32::min);
            rig.pose_after(&matrices, None, &mut scratch, Some(step));
            let lowest = rig
                .surfaces
                .iter()
                .flat_map(|s| &s.vertices)
                .map(|v| skin(v, &scratch.palette).0[2])
                .fold(f32::MAX, f32::min);
            let kyle_pelvis = affine(&matrices[pelvis_bone])
                .transform_point3a(pelvis_base)
                .z;
            let bob = scratch.pose[pelvis_joint].translation.z - kyle_pelvis;
            // Each foot's lowest sole point, the driver's and the rig's.
            let soles = rig.feet.as_ref().and_then(|feet| feet.soles.as_ref());
            let per_foot: Vec<String> = soles
                .map(|soles| {
                    soles
                        .driver
                        .iter()
                        .zip(&soles.rig)
                        .map(|(driver, own)| {
                            let k = driver
                                .iter()
                                .map(|(bone, at)| affine(&matrices[*bone]).transform_point3a(*at).z)
                                .fold(f32::MAX, f32::min);
                            let r = own
                                .iter()
                                .map(|(joint, at)| {
                                    (scratch.pose[*joint] * rig.joints[*joint].inverse_bind)
                                        .transform_point3a(*at)
                                        .z
                                })
                                .fold(f32::MAX, f32::min);
                            format!("{k:7.2}/{r:7.2}")
                        })
                        .collect()
                })
                .unwrap_or_default();
            println!(
                "{animation} {frame}: pelvis {:+.2} vs Kyle's, lowest {:+.2} vs Kyle's, feet (Kyle/rig) {}",
                bob,
                lowest - kyle,
                per_foot.join(" ")
            );
            bobs.push(bob);
            feet.push(lowest - kyle);
        }
        let span = |v: &[f32]| {
            v.iter().copied().fold(f32::MIN, f32::max) - v.iter().copied().fold(f32::MAX, f32::min)
        };
        let jump = bobs
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0, f32::max);
        println!(
            "{animation}: pelvis offset spans {:.2} (most in a frame {jump:.2}), lowest point vs Kyle's {:+.2}..{:+.2}",
            span(&bobs),
            feet.iter().copied().fold(f32::MAX, f32::min),
            feet.iter().copied().fold(f32::MIN, f32::max)
        );
    }
}
