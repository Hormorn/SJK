//! World shots of tags ([`crate::tags`]) on `mp/duel6`: from where the client opens, the
//! crosshair's trace finds walls round about and the floor below, a tag goes on each as
//! `toy_tag` sticks one (upright on a wall, facing the player on the floor), and each is
//! seen from in front. The pictures come from the hub's `sjk_tags` and `sjk_cards` packs
//! in the folder `SJK_TEST_TAGS` names (the hub's `assets/dist`), else nothing is drawn.
//! Since 31/10/2026 the tags show their finishes and cards stick too: a Normal tag, a
//! Holo one, a Holo and a Solar credits card, and a Solar tag on the floor. Ignored like
//! the other world shots.

use super::*;
use glam::Vec3;

/// `CONTENTS_SOLID`.
const SOLID: u32 = 1;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_tags_on_walls_and_floor() {
    on_big_stack(|| {
        // The hub's packs, mounted below the game data as a client's cache is.
        let cache = tempfile::tempdir().unwrap();
        if let Some(dist) = std::env::var_os("SJK_TEST_TAGS") {
            let assets = crate::sjk_packs::directory_in(cache.path());
            std::fs::create_dir_all(&assets).unwrap();
            for pack in ["sjk_tags.pk3", "sjk_cards.pk3"] {
                std::fs::copy(std::path::Path::new(&dist).join(pack), assets.join(pack)).unwrap();
            }
            crate::sjk_packs::mount_at_start(cache.path());
        }
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[]) else {
            return;
        };
        gpu.is_menu_world = false;
        let eye = gpu.camera_position;
        let trace = |gpu: &mut GpuState, to: Vec3| {
            let result = gpu.bsp.trace_box_with(
                &mut gpu.trace_scratch,
                eye.to_array(),
                to.to_array(),
                sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                SOLID,
            );
            (result.fraction < 1.0 && !result.start_solid)
                .then(|| {
                    result.plane.map(|plane| {
                        (
                            Vec3::from_array(result.end_position),
                            Vec3::from_array(plane.normal),
                        )
                    })
                })
                .flatten()
        };
        // Walls the eye sees within reach of a few steps, and the floor under it.
        let mut spots = Vec::new();
        for step in 0..8 {
            let yaw = (step as f32 * 45.0).to_radians();
            let forward = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
            if let Some((at, normal)) = trace(&mut gpu, eye + forward * 700.0)
                && normal.z.abs() < 0.7
            {
                spots.push((at, normal, forward));
            }
        }
        spots.truncate(4);
        let down = Vec3::new(1.0, 0.0, -1.5).normalize();
        if let Some((at, normal)) = trace(&mut gpu, eye + down * 600.0) {
            spots.push((at, normal, down));
        }
        assert!(!spots.is_empty(), "the eye sees a wall or the floor");
        // What each spot shows: the walls', then the floor's (the last spot).
        let shows = [
            ("tag_gonk", None),
            ("tag_kyber", Some("holo")),
            ("sjk_alpha_000", Some("holo")),
            ("sjk_alpha_001", Some("solar")),
        ];
        let floor = ("tag_solar_fox", Some("solar"));
        let mut images = Vec::new();
        for (index, (at, normal, forward)) in spots.iter().enumerate() {
            let (id, finish) = if normal.z.abs() >= 0.7 {
                floor
            } else {
                shows[index % shows.len()]
            };
            let tag = sjk_identity::PlacedTag {
                id: id.to_owned(),
                map: "mp/duel6".to_owned(),
                at: at.to_array(),
                normal: normal.to_array(),
                spin: crate::tags::spin_for(*normal, crate::tags::up_for(*normal, *forward)),
                finish: finish.map(str::to_owned),
            };
            gpu.effect_aux.decals.set_tag(index, Some(&tag));
            // Seen from in front, a little back from where it was stuck.
            let back = *at - *forward * 110.0 + Vec3::Z * 10.0;
            let look = (*at - back).normalize();
            let yaw = look.y.atan2(look.x).to_degrees();
            let pitch = look.z.asin().to_degrees();
            aim(&mut gpu, back.to_array(), yaw, pitch);
            images.push(frame(&mut gpu, 6));
            println!(
                "tag {} ({:?}) at {at} facing {normal}: {} fragments, drawn {}",
                tag.id,
                tag.finish,
                gpu.effect_aux.decals.tag_polys(),
                gpu.tag_render.has_draws()
            );
        }
        assert!(
            gpu.effect_aux.decals.tag_polys() > 0,
            "the tags were projected"
        );
        // A GIF tag (10/11/2026) on the first wall, a made-up animation standing in for its
        // GIPHY GIF, two frames apart.
        crate::chat_gifs::set_walls(true);
        let gif = crate::chat_gifs::GifId::new("TestGif1").unwrap();
        let animation = crate::chat_gifs::decode::test_animation([320, 200], 8, 120);
        let (at, normal, forward) = spots[0];
        let tag = sjk_identity::PlacedTag {
            id: "gif:TestGif1".to_owned(),
            map: "mp/duel6".to_owned(),
            at: at.to_array(),
            normal: normal.to_array(),
            spin: crate::tags::spin_for(normal, crate::tags::up_for(normal, forward)),
            finish: None,
        };
        gpu.effect_aux.decals.set_tag(0, Some(&tag));
        let back = at - forward * 110.0 + Vec3::Z * 10.0;
        let look = (at - back).normalize();
        aim(
            &mut gpu,
            back.to_array(),
            look.y.atan2(look.x).to_degrees(),
            look.z.asin().to_degrees(),
        );
        for held in [0, 480] {
            crate::chat_gifs::insert_wall_for_shot(&gif, &animation, held);
            images.push(frame(&mut gpu, 6));
            println!("GIF tag at {held} ms: drawn {}", gpu.tag_render.has_draws());
        }
        println!("{}", sheet(&images, 2, 960, "duel6-tags").display());
    });
}
