//! World shots of map edits ([`crate::map_edits`]): duel6's prop nearest its first spawn
//! point seen without edits, then with that prop hidden, the world surface ahead of the
//! camera hidden, a copy of the prop placed beside it and the floor under it remapped;
//! then the edits taken back. Ignored like the other world shots.

use super::*;
use crate::map_edits::document::{Document, Kind};
use crate::map_edits::panel::Tab;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_prop_hidden_copied_and_floor_remapped() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[]) else {
            return;
        };
        let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let spawn = Vec3::from_array(spawn);
        let (entity, prop) = crate::static_models::spawns(&gpu.bsp)
            .into_iter()
            .min_by(|(_, a), (_, b)| {
                let distance =
                    |s: &crate::static_models::Spawn| Vec3::from_array(s.origin).distance(spawn);
                distance(a).total_cmp(&distance(b))
            })
            .expect("duel6 places props");
        let origin = Vec3::from_array(prop.origin);
        let camera = origin + Vec3::new(-220.0, -160.0, 120.0);
        let (yaw, pitch) = look(camera.to_array(), (origin + Vec3::Z * 24.0).to_array());
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        let before = frame(&mut gpu, 40);

        // The surface under the prop and the one the camera looks at.
        let down = gpu
            .decal_surfaces
            .ray_hit(origin + Vec3::Z * 8.0, -Vec3::Z, 512.0)
            .expect("a floor under the prop");
        let forward = (origin - camera).normalize();
        let ahead = gpu
            .decal_surfaces
            .ray_hit(camera, forward, 4096.0)
            .expect("a surface ahead");
        let shader_of = |gpu: &GpuState, surface: usize| {
            let render = gpu.bsp.render();
            let index = render.surfaces()[surface].shader;
            gpu.bsp.shaders()[index].name_lossy().into_owned()
        };
        let floor = shader_of(&gpu, down.surface);
        let ahead_shader = shader_of(&gpu, ahead.surface);
        let mut document = Document::new("mp/duel6", 1);
        document.add(
            Kind::HideProp {
                entity,
                model: prop.model.clone(),
            },
            None,
        );
        if ahead.surface != down.surface {
            document.add(
                Kind::HideSurface {
                    surface: ahead.surface as u32,
                    shader: ahead_shader.clone(),
                },
                None,
            );
        }
        document.add(
            Kind::Place {
                model: prop.model.clone(),
                origin: (origin + Vec3::new(0.0, 96.0, 0.0)).to_array(),
                angles: [0.0, prop.angles[1] + 90.0, 0.0],
                scale: prop.scale,
            },
            None,
        );
        // Another of the map's own shaders, so it is surely in the game data.
        let target = gpu
            .bsp
            .shaders()
            .iter()
            .map(|shader| shader.name_lossy().into_owned())
            .find(|name| {
                name != &floor
                    && name.starts_with("textures/")
                    && !["sky", "nodraw", "caulk", "clip", "trigger", "system"]
                        .iter()
                        .any(|word| name.contains(word))
            })
            .expect("another shader");
        document.add(
            Kind::Remap {
                from: floor.clone(),
                to: target.clone(),
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        assert!(
            gpu.decal_surfaces
                .ray_hit(camera, forward, 4096.0)
                .is_none_or(|hit| hit.surface != ahead.surface || ahead.surface == down.surface),
            "a hidden surface is not picked"
        );
        let edited = frame(&mut gpu, 40);
        gpu.shot_map_edits(None);
        let after = frame(&mut gpu, 40);
        let changed = |a: &image::RgbaImage, b: &image::RgbaImage| {
            a.pixels()
                .zip(b.pixels())
                .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 24))
                .count()
        };
        println!(
            "prop {entity} {} at {origin:?}; hid surface {} ({ahead_shader}); floor {floor} -> {target}",
            prop.model, ahead.surface
        );
        println!(
            "pixels changed: edited {}, restored {}",
            changed(&before, &edited),
            changed(&before, &after)
        );
        println!(
            "{}",
            sheet(&[before, edited, after], 3, 960, "map-edits-duel6").display()
        );
    });
}

/// The map editor's panel over duel6: a prop selected under the crosshair, the Place page
/// searching for trees, the Edits page with a few edits, a surface clicked on the scene,
/// and a 4:3 window.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_map_editor_panel() {
    on_big_stack(|| {
        for (size, prefix) in [
            ([1920, 1080], "map-editor"),
            ([1440, 1080], "map-editor-4x3"),
        ] {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &[]) else {
                return;
            };
            let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
            let spawn = Vec3::from_array(spawn);
            let (entity, prop) = crate::static_models::spawns(&gpu.bsp)
                .into_iter()
                .min_by(|(_, a), (_, b)| {
                    let distance = |s: &crate::static_models::Spawn| {
                        Vec3::from_array(s.origin).distance(spawn)
                    };
                    distance(a).total_cmp(&distance(b))
                })
                .expect("duel6 places props");
            let origin = Vec3::from_array(prop.origin);
            let camera = origin + Vec3::new(-220.0, -160.0, 120.0);
            let (yaw, pitch) = look(camera.to_array(), (origin + Vec3::Z * 24.0).to_array());
            aim(&mut gpu, camera.to_array(), yaw, pitch);
            if let Some(console) = gpu.console.as_mut() {
                console.close_for_connection();
            }
            let _ = frame(&mut gpu, 40);
            let mut document = Document::new("mp/duel6", 1);
            document.add(
                Kind::Place {
                    model: prop.model.clone(),
                    origin: (origin + Vec3::new(0.0, 96.0, 0.0)).to_array(),
                    angles: [0.0, 90.0, 0.0],
                    scale: [1.0; 3],
                },
                None,
            );
            document.add(
                Kind::HideSurface {
                    surface: 12,
                    shader: "textures/yavin/floor".to_owned(),
                },
                Some("192.0.2.1:29070".to_owned()),
            );
            document.add(
                Kind::Remap {
                    from: "textures/yavin/wall".to_owned(),
                    to: "textures/yavin/wall2".to_owned(),
                },
                None,
            );
            document.unpublished = true;
            gpu.shot_map_edits(Some(document));
            let _ = frame(&mut gpu, 10);
            gpu.shot_map_edit_panel(Tab::Selected, None);
            println!("prop {entity} {}", prop.model);
            println!(
                "{}",
                shoot(&mut gpu, 8, &format!("{prefix}-selected")).display()
            );
            if prefix != "map-editor" {
                continue;
            }
            gpu.shot_map_edit_panel(Tab::Place, None);
            gpu.shot_map_edit_search("yavin");
            println!("{}", shoot(&mut gpu, 4, "map-editor-place").display());
            gpu.shot_map_edit_panel(Tab::Edits, None);
            println!("{}", shoot(&mut gpu, 4, "map-editor-edits").display());
            // A click low on the left of the scene: the floor.
            gpu.shot_map_edit_panel(Tab::Selected, Some([300.0, 900.0]));
            println!("{}", shoot(&mut gpu, 4, "map-editor-surface").display());
        }
    });
}

/// The map editor's panel's pictures and new pages over duel6 (09/11/2026): the floor
/// clicked, its shader list with a picture per row and the large preview card, the same
/// list as tiles, the Lights page with two added lights (their bulbs over the scene, the
/// chosen one ringed with its reach), and Place's things.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_map_editor_pictures_and_lights() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], None, &[]) else {
            return;
        };
        let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let spawn = Vec3::from_array(spawn);
        let (_, prop) = crate::static_models::spawns(&gpu.bsp)
            .into_iter()
            .min_by(|(_, a), (_, b)| {
                let distance =
                    |s: &crate::static_models::Spawn| Vec3::from_array(s.origin).distance(spawn);
                distance(a).total_cmp(&distance(b))
            })
            .expect("duel6 places props");
        let origin = Vec3::from_array(prop.origin);
        let camera = origin + Vec3::new(-220.0, -160.0, 120.0);
        let (yaw, pitch) = look(camera.to_array(), (origin + Vec3::Z * 24.0).to_array());
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let _ = frame(&mut gpu, 40);
        let mut document = Document::new("mp/duel6", 1);
        for (offset, color) in [
            (Vec3::new(0.0, 0.0, 64.0), [1.0, 0.3, 0.2]),
            (Vec3::new(0.0, 160.0, 48.0), [0.3, 0.55, 1.0]),
        ] {
            document.add(
                Kind::Light {
                    origin: (origin + offset).to_array(),
                    color,
                    radius: 200.0,
                    intensity: 1.0,
                },
                None,
            );
        }
        document.unpublished = true;
        gpu.shot_map_edits(Some(document));
        let _ = frame(&mut gpu, 10);
        // The path at the bottom: its shader list, pictured.
        gpu.shot_map_edit_panel(Tab::Selected, Some([960.0, 1010.0]));
        let _ = frame(&mut gpu, 2);
        // The map's own textures, the third chosen.
        gpu.shot_map_edit_remap_search("textures/yavin");
        println!("{}", shoot(&mut gpu, 16, "map-editor-pictures").display());
        gpu.map_edits.panel.toggle_tiles();
        println!("{}", shoot(&mut gpu, 20, "map-editor-tiles").display());
        let pictures = gpu.map_edits.panel.shader_shown.len();
        gpu.map_edits.panel.toggle_tiles();
        gpu.shot_map_edit_panel(Tab::Lights, None);
        let _ = frame(&mut gpu, 2);
        gpu.map_edits.panel.light_row = 0;
        println!("{}", shoot(&mut gpu, 6, "map-editor-lights").display());
        let lights = gpu.map_edits.panel.facts.lights.len();
        assert_eq!(lights, 2, "both lights listed");
        let target = gpu.shot_light_target();
        assert!(target.is_some(), "the gizmo stands on the chosen light");
        gpu.shot_map_edit_panel(Tab::Place, None);
        gpu.map_edits.panel.things = true;
        gpu.map_edits.panel.stale();
        println!("{}", shoot(&mut gpu, 6, "map-editor-things").display());
        println!("{pictures} shaders listed; {lights} lights; gizmo at {target:?}");
    });
}

/// The editor's texture guesses and selections over duel6 (10/11/2026): the path
/// clicked, its Suggested rows (nearby, same family, on this map) at the head of its
/// shader list; the hover's faint outline and tip over a wall; a box drawn over the scene
/// and what it selected; and the remap preview of a Suggested row the pointer rests on,
/// gone again once the pointer leaves the list.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_map_editor_suggestions_hover_and_box() {
    use sjk_ui::{InputEvent, PointerButton, Vec2};
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], None, &[]) else {
            return;
        };
        let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let spawn = Vec3::from_array(spawn);
        let (_, prop) = crate::static_models::spawns(&gpu.bsp)
            .into_iter()
            .min_by(|(_, a), (_, b)| {
                let distance =
                    |s: &crate::static_models::Spawn| Vec3::from_array(s.origin).distance(spawn);
                distance(a).total_cmp(&distance(b))
            })
            .expect("duel6 places props");
        let origin = Vec3::from_array(prop.origin);
        let camera = origin + Vec3::new(-220.0, -160.0, 120.0);
        let (yaw, pitch) = look(camera.to_array(), (origin + Vec3::Z * 24.0).to_array());
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let _ = frame(&mut gpu, 40);
        let mut document = Document::new("mp/duel6", 1);
        document.add(
            Kind::Place {
                model: prop.model.clone(),
                origin: (origin + Vec3::new(0.0, 96.0, 0.0)).to_array(),
                angles: [0.0, 90.0, 0.0],
                scale: [1.0; 3],
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        let _ = frame(&mut gpu, 10);
        // The path at the bottom: Suggested rows lead its list.
        gpu.shot_map_edit_panel(Tab::Selected, Some([960.0, 1010.0]));
        let _ = frame(&mut gpu, 4);
        let panel = &gpu.map_edits.panel;
        let suggested: Vec<String> = (0..panel.candidate_rows)
            .map(|index| {
                format!(
                    "{} ({})",
                    panel.shown_shader(index).unwrap_or("?"),
                    panel
                        .suggestion(index)
                        .map(|r| r.mark())
                        .unwrap_or_default()
                )
            })
            .collect();
        println!(
            "selected {:?}",
            panel.facts.selection.as_ref().map(|s| &s.name)
        );
        println!("suggested: {suggested:?}");
        assert!(!suggested.is_empty(), "the path gets suggestions");
        assert!(
            suggested.iter().all(|row| !row.contains("system/")),
            "no tool shader is suggested: {suggested:?}"
        );
        println!("{}", shoot(&mut gpu, 16, "map-editor-suggested").display());
        // The hover over the tower's stones.
        let over = Vec2::new(812.0, 330.0);
        gpu.map_edit_pointer(InputEvent::PointerMove(over));
        let _ = frame(&mut gpu, 3);
        let tip = gpu.map_edits.panel.scene.tip.clone();
        println!("hover: {tip:?}");
        assert!(
            gpu.world_notes.hover().is_some(),
            "something is under the pointer"
        );
        println!("{}", shoot(&mut gpu, 4, "map-editor-hover").display());
        // A box over the floor ahead.
        let (from, to) = (Vec2::new(380.0, 640.0), Vec2::new(1150.0, 1040.0));
        gpu.map_edit_pointer(InputEvent::PointerPress {
            position: from,
            button: PointerButton::Primary,
        });
        for step in 1..=6 {
            let t = step as f32 / 6.0;
            gpu.map_edit_pointer(InputEvent::PointerMove(Vec2::new(
                from.x + (to.x - from.x) * t,
                from.y + (to.y - from.y) * t,
            )));
        }
        assert!(
            gpu.map_edits.panel.scene.marquee.is_some(),
            "a box is drawn"
        );
        println!("{}", shoot(&mut gpu, 3, "map-editor-box-drawing").display());
        let started = std::time::Instant::now();
        gpu.map_edit_pointer(InputEvent::PointerRelease {
            position: to,
            button: PointerButton::Primary,
        });
        let took = started.elapsed();
        let count = gpu.world_notes.edit_selection_len();
        println!(
            "box: {} ({count} selected in {took:?})",
            gpu.map_edits.panel.status
        );
        assert!(count > 1, "the box selects several things");
        println!("{}", shoot(&mut gpu, 6, "map-editor-box").display());
        // The path again, and the pointer resting on its first suggested row.
        gpu.shot_map_edit_panel(Tab::Selected, Some([960.0, 1010.0]));
        let _ = frame(&mut gpu, 4);
        let row = gpu
            .map_edits
            .panel
            .shader_list
            .expect("the shader list is drawn");
        let at = Vec2::new(row.x + row.width * 0.5, row.y + 8.0 + 17.0);
        gpu.map_edit_pointer(InputEvent::PointerMove(at));
        let _ = frame(&mut gpu, 2);
        std::thread::sleep(crate::map_edits::preview::SETTLE * 2);
        let _ = frame(&mut gpu, 4);
        let shown = gpu.map_edits.panel.preview.shown.clone();
        println!("preview: {shown:?}");
        assert!(shown.is_some(), "the row under the pointer is previewed");
        assert!(
            gpu.map_edits.panel.facts.edit_count == 1,
            "a preview writes no edit"
        );
        println!("{}", shoot(&mut gpu, 6, "map-editor-preview").display());
        gpu.map_edit_pointer(InputEvent::PointerMove(Vec2::new(520.0, 360.0)));
        let _ = frame(&mut gpu, 3);
        assert!(
            gpu.map_edits.panel.preview.shown.is_none(),
            "leaving the list puts the map's back"
        );
        println!(
            "{}",
            shoot(&mut gpu, 6, "map-editor-preview-gone").display()
        );
    });
}

/// The map editor's gizmo over duel6 ([`crate::map_edits::gizmo`]): a placed model
/// current on Place with its move, rotate and scale handles, then a drag on its X arrow
/// half-way (the model previewed 40 units along X), and the release (refused here: a shot
/// is not staff, so the model goes back).
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_map_editor_gizmo() {
    use crate::map_edits::gizmo::Mode;
    use sjk_ui::{InputEvent, PointerButton};
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], None, &[]) else {
            return;
        };
        let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let spawn = Vec3::from_array(spawn);
        let (_, prop) = crate::static_models::spawns(&gpu.bsp)
            .into_iter()
            .min_by(|(_, a), (_, b)| {
                let distance =
                    |s: &crate::static_models::Spawn| Vec3::from_array(s.origin).distance(spawn);
                distance(a).total_cmp(&distance(b))
            })
            .expect("duel6 places props");
        let placed_at = Vec3::from_array(prop.origin) + Vec3::new(0.0, 96.0, 0.0);
        let camera = placed_at + Vec3::new(-260.0, -200.0, 150.0);
        let (yaw, pitch) = look(camera.to_array(), (placed_at + Vec3::Z * 24.0).to_array());
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let _ = frame(&mut gpu, 40);
        let mut document = Document::new("mp/duel6", 1);
        let id = document.add(
            Kind::Place {
                model: prop.model.clone(),
                origin: placed_at.to_array(),
                angles: [0.0, 30.0, 0.0],
                scale: [1.0; 3],
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        let _ = frame(&mut gpu, 10);
        gpu.map_edits.editor.set_current(id);
        gpu.shot_map_edit_panel(Tab::Place, None);
        // The gizmo on the current placed model rather than on what the crosshair met.
        gpu.world_notes.cancel_selection();
        let mut shots = Vec::new();
        for (mode, name) in [
            (Mode::Move, "map-editor-gizmo-move"),
            (Mode::Rotate, "map-editor-gizmo-rotate"),
            (Mode::Scale, "map-editor-gizmo-scale"),
        ] {
            gpu.map_edits.gizmo.mode = mode;
            shots.push(shoot(&mut gpu, 6, name));
            assert!(
                gpu.map_edits.gizmo.list().is_some(),
                "{name}: the gizmo is drawn"
            );
        }
        gpu.map_edits.gizmo.mode = Mode::Move;
        let _ = frame(&mut gpu, 2);
        let [from, to] = gpu
            .shot_gizmo_arrow(40.0)
            .expect("the X arrow on the screen");
        gpu.map_edit_pointer(InputEvent::PointerMove(from));
        gpu.map_edit_pointer(InputEvent::PointerPress {
            position: from,
            button: PointerButton::Primary,
        });
        for step in 1..=8 {
            let t = step as f32 / 8.0;
            gpu.map_edit_pointer(InputEvent::PointerMove(sjk_ui::Vec2::new(
                from.x + (to.x - from.x) * t,
                from.y + (to.y - from.y) * t,
            )));
        }
        let moved = gpu.shot_placed_origin(id).expect("the placed model") - placed_at;
        println!("dragged by {moved:?}: {}", gpu.map_edits.panel.status);
        assert!(
            (moved.x - 40.0).abs() < 1.0 && moved.y.abs() < 0.5 && moved.z.abs() < 0.5,
            "the preview follows the X arrow: {moved:?}"
        );
        shots.push(shoot(&mut gpu, 4, "map-editor-gizmo-drag"));
        gpu.map_edit_pointer(InputEvent::PointerRelease {
            position: to,
            button: PointerButton::Primary,
        });
        println!("released: {}", gpu.map_edits.panel.status);
        for shot in shots {
            println!("{}", shot.display());
        }
    });
}

/// The in-game gizmo on a set of placed models (ffa3, `SJK_GIZMO_MAP`): three retail
/// crates placed in a row before the first spawn, each Shift+clicked into the
/// selection, the rotate gizmo drawn at their middle, then R Z 90 previewed and put down
/// as one `mapedit_set` (one step to undo): every origin orbits the pivot, every model
/// turns by 90 degrees, and the selection's outline follows them.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn ingame_gizmo_on_a_set_of_placed_models() {
    use crate::map_edits::gizmo::{ModalKey, Mode};
    on_big_stack(|| {
        let map = std::env::var("SJK_GIZMO_MAP").unwrap_or_else(|_| "ffa3".to_owned());
        let Some((mut gpu, _profile)) =
            open(&format!("maps/mp/{map}.bsp"), [1920, 1080], None, &[])
        else {
            return;
        };
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let (spawn, facing) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let spawn = Vec3::from_array(spawn);
        let ahead = Vec3::new(facing.cos(), facing.sin(), 0.0);
        let side = Vec3::new(-ahead.y, ahead.x, 0.0);
        // A retail crate (ffa3 places no MD3 props of its own).
        let model = "models/map_objects/factory/s_crate_01.md3";
        // Three in a row on the floor ahead of the spawn, turned 0, 30 and 60 degrees.
        let floor = |at: Vec3| {
            gpu.decal_surfaces
                .ray_hit(at + Vec3::Z * 16.0, -Vec3::Z, 512.0)
                .map_or(at, |hit| at + Vec3::Z * 16.0 - Vec3::Z * hit.distance)
        };
        let spots: Vec<Vec3> = (0..3)
            .map(|i| floor(spawn + ahead * 170.0 + side * ((i as f32 - 1.0) * 72.0)))
            .collect();
        let mut document = Document::new(&format!("mp/{map}"), 1);
        let ids: Vec<u32> = spots
            .iter()
            .enumerate()
            .map(|(i, spot)| {
                document.add(
                    Kind::Place {
                        model: model.to_owned(),
                        origin: spot.to_array(),
                        angles: [0.0, i as f32 * 30.0, 0.0],
                        scale: [1.0; 3],
                    },
                    None,
                )
            })
            .collect();
        gpu.shot_map_edits(Some(document));
        // The client's frame work the shots skip (`refresh_map_edits` waits for a map
        // name): the crate's mesh loaded on demand, the edits applied again after a
        // change, the selection's outline following its models.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !gpu
            .object_meshes
            .iter()
            .any(|mesh| mesh.appearance.model.eq_ignore_ascii_case(model))
        {
            assert!(std::time::Instant::now() < deadline, "the crate's mesh");
            gpu.refresh_demand_models();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let document = gpu.map_edits.document.clone();
        gpu.shot_map_edits(document);
        assert!(
            gpu.map_edits.wanted_models().next().is_none(),
            "crates drawn"
        );
        let settle = |gpu: &mut GpuState| gpu.shot_follow_placed();
        let middle = spots.iter().copied().sum::<Vec3>() / 3.0;
        let camera = spawn - ahead * 40.0 + side * 40.0 + Vec3::Z * 90.0;
        let (yaw, pitch) = look(camera.to_array(), (middle + Vec3::Z * 16.0).to_array());
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        println!(
            "spawn {spawn}, spots {spots:?}, {}",
            shoot(&mut gpu, 30, "mapedit-multi-gizmo-scene").display()
        );
        gpu.map_edits.editor.active = true;
        gpu.world_notes.hold_selection(true);
        gpu.open_map_edit_panel();
        // Shift+click each model: up its middle until the click lands on it.
        for &id in &ids {
            let origin = gpu.shot_placed_origin(id).expect("placed");
            let mut added = false;
            for lift in [8.0, 16.0, 24.0, 4.0, 32.0, 40.0, 2.0] {
                let Some(point) = gpu.shot_project(origin + Vec3::Z * lift) else {
                    continue;
                };
                let toggled = gpu.shot_map_edit_toggle(point);
                let newest = gpu
                    .world_notes
                    .waiting_selection()
                    .and_then(|s| s.placed_id());
                println!(
                    "{id} lift {lift} at {point:?}: {toggled:?}, newest {newest:?} {:?}",
                    gpu.world_notes.waiting_selection().map(|s| s.summary())
                );
                if matches!(toggled, crate::world_notes::Toggled::Added(_)) {
                    if newest == Some(id) {
                        added = true;
                        break;
                    }
                    // Something else (the floor) was under the pointer: out again.
                    gpu.shot_map_edit_toggle(point);
                }
            }
            assert!(added, "placed model {id} Shift+clicked into the set");
        }
        assert_eq!(
            gpu.map_edit_set_ids(),
            [ids[2], ids[1], ids[0]],
            "newest first"
        );
        gpu.map_edits.panel.stale();
        gpu.map_edits.gizmo.mode = Mode::Rotate;
        let (targets, pivot) = gpu.shot_gizmo_targets();
        assert_eq!(targets.len(), 3);
        assert!(
            pivot.distance(middle) < 0.5,
            "the handles at the set's middle"
        );
        settle(&mut gpu);
        let before = shoot(&mut gpu, 8, "mapedit-multi-gizmo-before");
        assert!(gpu.map_edits.gizmo.list().is_some(), "the gizmo is drawn");
        assert_eq!(gpu.map_edits.panel.facts.models, 3);
        // R Z 9 0: the preview, then put down.
        gpu.shot_gizmo_keys(
            Mode::Rotate,
            glam::Vec2::new(120.0, -40.0),
            &[
                ModalKey::Axis(2, false),
                ModalKey::Typed('9'),
                ModalKey::Typed('0'),
            ],
        );
        println!("previewed: {}", gpu.map_edits.panel.status);
        settle(&mut gpu);
        let during = shoot(&mut gpu, 4, "mapedit-multi-gizmo-turning");
        let committed = gpu.shot_gizmo_commit();
        println!("put down: {committed:?}");
        assert!(committed.is_ok(), "{committed:?}");
        assert_eq!(
            gpu.map_edits.history.can(),
            (true, false),
            "one step to undo"
        );
        for (id, spot) in ids.iter().zip(&spots) {
            let now = gpu.shot_placed_origin(*id).expect("placed");
            let offset = *spot - pivot;
            let expected = pivot + Vec3::new(-offset.y, offset.x, offset.z);
            assert!(now.distance(expected) < 0.05, "{id}: {now} != {expected}");
        }
        gpu.map_edits.panel.answer(committed);
        gpu.map_edits.panel.stale();
        settle(&mut gpu);
        let after = shoot(&mut gpu, 8, "mapedit-multi-gizmo-after");
        assert_eq!(gpu.world_notes.edit_selection_len(), 3, "the set stays");
        // Undo: all three back where they were.
        let undone = gpu.shot_map_edit_run("mapedit_undo", &[]);
        println!("undo: {undone:?}");
        // Drawn again from the document, the outline back with them.
        let document = gpu.map_edits.document.clone();
        gpu.shot_map_edits(document);
        settle(&mut gpu);
        let undone = shoot(&mut gpu, 8, "mapedit-multi-gizmo-undone");
        for (id, spot) in ids.iter().zip(&spots) {
            let now = gpu.shot_placed_origin(*id).expect("placed");
            assert!(now.distance(*spot) < 0.05, "{id} back: {now} != {spot}");
        }
        for shot in [&before, &during, &after, &undone] {
            println!("{}", shot.display());
        }
    });
}

/// A caulk face drawn by an edit (`Kind::ShowFace`): the first caulk side of `SJK_FACE_MAP`
/// (default duel6) with open space in front and no drawn surface over it, seen from 96
/// units out without edits, selected (the panel's Hidden face page), drawn with a wall
/// shader of the map, then the edit taken back.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn caulk_face_shown() {
    on_big_stack(|| {
        let map = std::env::var("SJK_FACE_MAP").unwrap_or_else(|_| "duel6".to_owned());
        let Some((mut gpu, _profile)) = open(&format!("maps/mp/{map}.bsp"), [1280, 720], None, &[])
        else {
            return;
        };
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let Some((brush, side, shader, centre, eye)) = seen_caulk_wall(&gpu) else {
            panic!("{map}: no caulk wall seen from open space");
        };
        let (yaw, pitch) = look(eye.to_array(), centre.to_array());
        aim(&mut gpu, eye.to_array(), yaw, pitch);
        let before = frame(&mut gpu, 40);
        let mut document = Document::new(&format!("mp/{map}"), 1);
        document.unpublished = true;
        gpu.shot_map_edits(Some(document.clone()));
        gpu.shot_map_edit_panel(Tab::Selected, None);
        let selected = gpu.world_notes.waiting_selection().map(|s| s.summary());
        println!("selected: {selected:?}");
        assert!(
            matches!(
                gpu.world_notes.waiting_selection().map(|s| s.edit_target()),
                Some(crate::world_notes::EditTarget::Face { brush: b, side: s, .. })
                    if b as usize == brush && s as usize == side
            ),
            "the crosshair selects the caulk face"
        );
        let panel = shoot(&mut gpu, 8, "caulk-face-selected");
        let target = gpu
            .bsp
            .shaders()
            .iter()
            .map(|shader| shader.name_lossy().into_owned())
            .find(|name| {
                name.starts_with("textures/")
                    && name.contains("wall")
                    && !["sky", "nodraw", "caulk", "clip", "trigger", "system"]
                        .iter()
                        .any(|word| name.contains(word))
            })
            .expect("a wall shader");
        gpu.world_notes.cancel_selection();
        gpu.close_map_edit_panel();
        document.add(
            Kind::ShowFace {
                brush: brush as u32,
                side: side as u32,
                shader: shader.clone(),
                to: target.clone(),
                align: crate::map_edits::document::Align::default(),
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        let shown = frame(&mut gpu, 40);
        gpu.shot_map_edits(None);
        let after = frame(&mut gpu, 40);
        let changed = |a: &image::RgbaImage, b: &image::RgbaImage| {
            a.pixels()
                .zip(b.pixels())
                .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 24))
                .count()
        };
        println!("{map}: brush {brush} side {side} ({shader}) at {centre:?} drawn as {target}");
        println!(
            "pixels changed: shown {}, restored {}",
            changed(&before, &shown),
            changed(&before, &after)
        );
        println!("{}", panel.display());
        println!(
            "{}",
            sheet(
                &[before, shown, after],
                3,
                960,
                &format!("caulk-face-{map}")
            )
            .display()
        );
    });
}

/// The first caulk wall of the loaded map seen from open space 96 units in front of it,
/// with no drawn surface over it: its brush, side, shader, middle and that eye.
fn seen_caulk_wall(gpu: &GpuState) -> Option<(usize, usize, String, Vec3, Vec3)> {
    seen_caulk_walls(gpu, 1).into_iter().next()
}

/// Up to `most` such caulk walls ([`seen_caulk_wall`]), in the map's order.
fn seen_caulk_walls(gpu: &GpuState, most: usize) -> Vec<(usize, usize, String, Vec3, Vec3)> {
    let bsp = &gpu.bsp;
    let world = bsp.render().models()[0].brushes.clone();
    let mut found = Vec::new();
    for brush in world {
        for side in bsp.brushes()[brush].sides.clone() {
            let shader = &bsp.shaders()[bsp.brush_sides()[side].shader];
            if !shader.name_lossy().contains("caulk") {
                continue;
            }
            let polygon = bsp.brush_side_polygon(brush, side);
            if polygon.len() < 3 {
                continue;
            }
            let normal = Vec3::from_array(bsp.planes()[bsp.brush_sides()[side].plane].normal);
            // A wall, so the camera stands beside it rather than over or under it.
            if normal.z.abs() > 0.3 {
                continue;
            }
            let centre = Vec3::from_array(crate::map_edits::faces::centre(&polygon));
            let eye = centre + normal * 96.0;
            if bsp.point_contents(eye.to_array(), 1) != 0 {
                continue;
            }
            let drawn_over = gpu.decal_surfaces.ray_hit(eye, -normal, 97.0).is_some();
            let picked = crate::map_edits::faces::pick(bsp, eye, -normal, 200.0);
            if !drawn_over
                && picked
                    .is_some_and(|face| face.brush as usize == brush && face.side as usize == side)
            {
                found.push((brush, side, shader.name_lossy().into_owned(), centre, eye));
                if found.len() == most {
                    return found;
                }
            }
        }
    }
    found
}

/// The in-game editor's multiple selection and Fill (04/11/2026, the browser editor's
/// tools in game): the first caulk wall of `SJK_FACE_MAP` (default ffa3) seen from open
/// space whose Connected surface holds several faces with drawn surfaces round them,
/// selected under the crosshair, its Connected surface selected (every face drawn), the
/// face under the crosshair taken out and added again as a Shift+click does, then Fill
/// with the nearby vote's shader (one step to undo), and the fill undone.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn ingame_link_select_and_fill() {
    on_big_stack(|| {
        let map = std::env::var("SJK_FACE_MAP").unwrap_or_else(|_| "ffa3".to_owned());
        let Some((mut gpu, _profile)) =
            open(&format!("maps/mp/{map}.bsp"), [1920, 1080], None, &[])
        else {
            return;
        };
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let mut document = Document::new(&format!("mp/{map}"), 1);
        document.unpublished = true;
        gpu.shot_map_edits(Some(document));
        let mut chosen = None;
        for (brush, side, shader, centre, eye) in seen_caulk_walls(&gpu, 200) {
            let (yaw, pitch) = look(eye.to_array(), centre.to_array());
            aim(&mut gpu, eye.to_array(), yaw, pitch);
            let _ = frame(&mut gpu, 1);
            gpu.shot_map_edit_panel(Tab::Selected, None);
            let linked = gpu.shot_map_edit_run("mapedit_link", &["surface"]);
            let count = gpu.world_notes.edit_selection_len();
            if linked.is_ok() && count > 1 && gpu.shot_map_edit_fill_suggestion().is_some() {
                chosen = Some((brush, side, shader, eye, centre, linked, count));
                break;
            }
            gpu.world_notes.cancel_selection();
        }
        let Some((brush, side, shader, eye, centre, linked, count)) = chosen else {
            panic!("{map}: no caulk wall with joined faces and drawn surfaces round them");
        };
        gpu.world_notes.cancel_selection();
        gpu.close_map_edit_panel();
        let (yaw, pitch) = look(eye.to_array(), centre.to_array());
        aim(&mut gpu, eye.to_array(), yaw, pitch);
        let before = frame(&mut gpu, 40);
        gpu.shot_map_edit_panel(Tab::Selected, None);
        let linked_again = gpu.shot_map_edit_run("mapedit_link", &["surface"]);
        assert_eq!(linked_again, linked);
        let selected = gpu.world_notes.waiting_selection().map(|s| s.summary());
        println!("{map}: brush {brush} side {side} ({shader}) at {centre:?}; {selected:?}");
        println!("link: {linked:?}");
        gpu.map_edits.panel.answer(linked);
        gpu.map_edits.panel.stale();
        let linked_shot = shoot(&mut gpu, 8, "mapedit-ingame-linked");
        let highlighted = frame(&mut gpu, 1);
        // The middle of the screen is the face linked from: a Shift+click there takes it
        // out, a second puts it back.
        let middle = [960.0, 540.0];
        let out = gpu.shot_map_edit_toggle(middle);
        let back = gpu.shot_map_edit_toggle(middle);
        println!("toggled: {out:?} then {back:?}");
        if count > 1 {
            assert_eq!(out, crate::world_notes::Toggled::Removed(count - 1));
            assert_eq!(back, crate::world_notes::Toggled::Added(count));
        }
        gpu.map_edits.panel.stale();
        let _ = frame(&mut gpu, 2);
        let fill = gpu.map_edits.panel.facts.fill.clone();
        println!("fill suggestion: {fill:?}");
        let filled = gpu.shot_map_edit_run("mapedit_fill", &[]);
        println!("fill: {filled:?}");
        assert!(filled.is_ok(), "{filled:?}");
        let edits = gpu.map_edits.document.as_ref().map_or(0, |d| d.edits.len());
        assert_eq!(edits, count, "a shown face for each selected face");
        assert_eq!(
            gpu.map_edits.history.can(),
            (true, false),
            "one step to undo"
        );
        gpu.map_edits.panel.answer(filled);
        gpu.map_edits.panel.stale();
        let filled_panel = shoot(&mut gpu, 8, "mapedit-ingame-filled-panel");
        gpu.world_notes.cancel_selection();
        gpu.close_map_edit_panel();
        let document = gpu.map_edits.document.clone();
        gpu.shot_map_edits(document);
        let filled_view = frame(&mut gpu, 40);
        let undone = gpu.shot_map_edit_run("mapedit_undo", &[]);
        println!("undo: {undone:?}");
        let document = gpu.map_edits.document.clone();
        assert_eq!(document.as_ref().map_or(0, |d| d.edits.len()), 0);
        gpu.shot_map_edits(document);
        let after = frame(&mut gpu, 40);
        let changed = |a: &image::RgbaImage, b: &image::RgbaImage| {
            a.pixels()
                .zip(b.pixels())
                .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 24))
                .count()
        };
        println!(
            "pixels changed: highlighted {}, filled {}, undone {}",
            changed(&before, &highlighted),
            changed(&before, &filled_view),
            changed(&before, &after)
        );
        println!("{}", linked_shot.display());
        println!("{}", filled_panel.display());
        println!(
            "{}",
            sheet(
                &[before, highlighted, filled_view, after],
                2,
                960,
                &format!("mapedit-ingame-{map}")
            )
            .display()
        );
    });
}

/// A sky side of `map` seen from open space under it, as a player sees the sky: the
/// side's brush, side, shader, middle, and an eye looking at it obliquely from 200 units
/// below and beside it, where the drawn sky surface is what the ray meets first.
fn seen_sky_side(gpu: &GpuState, map: &str) -> (u32, u32, String, Vec3, Vec3) {
    let bsp = &gpu.bsp;
    let mut best: Option<(f32, u32, u32, String, Vec3, Vec3)> = None;
    for brush in bsp.render().models()[0].brushes.clone() {
        for side in bsp.brushes()[brush].sides.clone() {
            let Some((shader, true)) =
                crate::map_edits::faces::world_side_shader(bsp, brush as u32, side as u32)
            else {
                continue;
            };
            let polygon = bsp.brush_side_polygon(brush, side);
            let normal = Vec3::from_array(bsp.planes()[bsp.brush_sides()[side].plane].normal);
            // A ceiling of sky, big enough to fill the view's middle.
            let corners: Vec<Vec3> = polygon.iter().copied().map(Vec3::from_array).collect();
            let area = crate::map_edits::linked::area(&corners);
            if normal.z > -0.9 || area < 256.0 * 256.0 {
                continue;
            }
            let centre = Vec3::from_array(crate::map_edits::faces::centre(&polygon));
            let eye = centre + Vec3::new(-120.0, -90.0, -200.0);
            if bsp.point_contents(eye.to_array(), 1) != 0 {
                continue;
            }
            let along = (centre - eye).normalize();
            let reach = centre.distance(eye);
            let sky_first = gpu
                .decal_surfaces
                .ray_hit(eye, along, reach + 4.0)
                .is_some_and(|hit| (hit.distance - reach).abs() < 1.0);
            if sky_first && best.as_ref().is_none_or(|known| area > known.0) {
                best = Some((area, brush as u32, side as u32, shader, centre, eye));
            }
        }
    }
    let (_, brush, side, shader, centre, eye) =
        best.unwrap_or_else(|| panic!("{map}: no sky ceiling seen from open space"));
    (brush, side, shader, centre, eye)
}

/// How many pixels of `a` and `b` differ (any channel by more than 24), in all, and the
/// share of those differing in the square of `half` pixels round the middle.
fn changed_pixels(a: &image::RgbaImage, b: &image::RgbaImage, half: u32) -> (usize, f32) {
    let differs = |x: u32, y: u32| {
        a.get_pixel(x, y)
            .0
            .iter()
            .zip(b.get_pixel(x, y).0)
            .any(|(p, q)| p.abs_diff(q) > 24)
    };
    let all = (0..a.height())
        .flat_map(|y| (0..a.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| differs(x, y))
        .count();
    let (cx, cy) = (a.width() / 2, a.height() / 2);
    let square: Vec<bool> = (cy - half..=cy + half)
        .flat_map(|y| (cx - half..=cx + half).map(move |x| (x, y)))
        .map(|(x, y)| differs(x, y))
        .collect();
    let share = square.iter().filter(|differ| **differ).count() as f32 / square.len() as f32;
    (all, share)
}

/// A sky ceiling of a map with open sky (`SJK_SKY_MAP`, default ffa3) selected under the
/// crosshair as a sky side, covered with a wall shader of the map (a `show_face` edit),
/// seen from three eyes a fraction of a unit apart (z-fighting with the sky would show
/// as sky through it at some), then the edit taken back.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn sky_face_covered() {
    on_big_stack(|| {
        let map = std::env::var("SJK_SKY_MAP").unwrap_or_else(|_| "ffa3".to_owned());
        let Some((mut gpu, _profile)) = open(&format!("maps/mp/{map}.bsp"), [1280, 720], None, &[])
        else {
            return;
        };
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let (brush, side, shader, centre, eye) = seen_sky_side(&gpu, &map);
        let (yaw, pitch) = look(eye.to_array(), centre.to_array());
        aim(&mut gpu, eye.to_array(), yaw, pitch);
        // (a frame first: the pick takes the camera the last frame was drawn from)
        let _ = frame(&mut gpu, 4);
        let mut document = Document::new(&format!("mp/{map}"), 1);
        document.unpublished = true;
        gpu.shot_map_edits(Some(document.clone()));
        gpu.shot_map_edit_panel(Tab::Selected, None);
        let selected = gpu.world_notes.waiting_selection().map(|s| s.summary());
        println!("selected: {selected:?}");
        assert!(
            matches!(
                gpu.world_notes.waiting_selection().map(|s| s.edit_target()),
                Some(crate::world_notes::EditTarget::Face { brush: b, side: s, sky: true, .. })
                    if b == brush && s == side
            ),
            "the crosshair selects the sky side"
        );
        let panel = shoot(&mut gpu, 8, "sky-face-selected");
        gpu.world_notes.cancel_selection();
        gpu.close_map_edit_panel();
        let target = gpu
            .bsp
            .shaders()
            .iter()
            .map(|shader| shader.name_lossy().into_owned())
            .find(|name| {
                name.starts_with("textures/")
                    && name.contains("wall")
                    && !["sky", "nodraw", "caulk", "clip", "trigger", "system"]
                        .iter()
                        .any(|word| name.contains(word))
            })
            .expect("a wall shader");
        let eyes = [
            eye,
            eye + Vec3::new(0.37, 0.0, 0.21),
            eye + Vec3::new(-0.19, 0.43, -0.29),
        ];
        let mut befores = Vec::new();
        for at in eyes {
            aim(&mut gpu, at.to_array(), yaw, pitch);
            befores.push(frame(&mut gpu, 30));
        }
        document.add(
            Kind::ShowFace {
                brush,
                side,
                shader: shader.clone(),
                to: target.clone(),
                align: crate::map_edits::document::Align::default(),
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        println!("{map}: brush {brush} side {side} ({shader}) at {centre:?} covered with {target}");
        let mut images = vec![befores[0].clone()];
        for (index, at) in eyes.iter().enumerate() {
            aim(&mut gpu, at.to_array(), yaw, pitch);
            let shown = frame(&mut gpu, 30);
            let (all, share) = changed_pixels(&befores[index], &shown, 40);
            let middle = |image: &image::RgbaImage| image.get_pixel(640, 360).0;
            println!(
                "eye {index}: pixels changed {all}, middle 81x81 square changed {:.1}%, \
                 middle pixel {:?} -> {:?}",
                share * 100.0,
                middle(&befores[index]),
                middle(&shown)
            );
            assert!(share > 0.99, "the face covers the sky in the middle");
            if index == 0 {
                images.push(shown);
            }
        }
        gpu.shot_map_edits(None);
        aim(&mut gpu, eye.to_array(), yaw, pitch);
        let after = frame(&mut gpu, 30);
        let (restored, _) = changed_pixels(&befores[0], &after, 40);
        println!("restored: pixels changed {restored}");
        images.push(after);
        println!("{}", panel.display());
        println!(
            "{}",
            sheet(&images, 3, 960, &format!("sky-face-{map}")).display()
        );
    });
}

/// The sky of a map with open sky (`SJK_SKY_MAP`, default ffa3) swapped for another sky
/// of the game data that has a box (`SJK_SKY_TARGET`, else the first other one), as the
/// editor's `mapedit_sky` makes it (a remap edit), then taken back.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn sky_swapped() {
    on_big_stack(|| {
        let map = std::env::var("SJK_SKY_MAP").unwrap_or_else(|_| "ffa3".to_owned());
        let Some((mut gpu, _profile)) = open(&format!("maps/mp/{map}.bsp"), [1280, 720], None, &[])
        else {
            return;
        };
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
        }
        let skies = crate::map_edits::faces::map_skies(&gpu.bsp);
        println!("{map}'s skies: {skies:?}");
        let (_, _, shader, centre, eye) = seen_sky_side(&gpu, &map);
        let vfs = gpu.vfs.clone().expect("game data");
        let boxed = |name: &str| {
            gpu.shaders
                .get(name)
                .and_then(|d| d.sky.as_ref())
                .and_then(|sky| sky.outer_box.as_deref())
                .is_some_and(|prefix| {
                    gpu.shaders
                        .resolve_stage_image(&vfs, &format!("{prefix}_ft"))
                        .ok()
                        .flatten()
                        .is_some()
                })
        };
        let target = std::env::var("SJK_SKY_TARGET").unwrap_or_else(|_| {
            crate::map_edits::panel::sky_shaders(&gpu.shaders)
                .into_iter()
                .find(|name| !name.eq_ignore_ascii_case(&shader) && boxed(name))
                .expect("another sky with a box")
        });
        let clouds = gpu.shaders.get(&target).map_or(0, |d| d.stages.len());
        println!("{shader} swapped for {target} ({clouds} cloud stages)");
        // Looking at the sky over the side, higher than its middle.
        let (yaw, pitch) = look(eye.to_array(), (centre + Vec3::Z * 400.0).to_array());
        aim(&mut gpu, eye.to_array(), yaw, pitch);
        let before = frame(&mut gpu, 30);
        let mut document = Document::new(&format!("mp/{map}"), 1);
        document.add(
            Kind::Remap {
                from: shader.clone(),
                to: target.clone(),
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        let swapped = frame(&mut gpu, 30);
        gpu.shot_map_edits(None);
        let after = frame(&mut gpu, 30);
        let (changed, share) = changed_pixels(&before, &swapped, 40);
        let (restored, _) = changed_pixels(&before, &after, 40);
        let middle = |image: &image::RgbaImage| image.get_pixel(640, 360).0;
        println!(
            "pixels changed: swapped {changed} (middle square {:.1}%), restored {restored}; \
             middle pixel {:?} -> {:?} -> {:?}",
            share * 100.0,
            middle(&before),
            middle(&swapped),
            middle(&after)
        );
        assert!(changed > 1280 * 720 / 10, "the sky drawn is the new one");
        assert!(
            restored < changed / 20,
            "taking the edit back restores the sky"
        );
        println!(
            "{}",
            sheet(
                &[before, swapped, after],
                3,
                960,
                &format!("sky-swap-{map}")
            )
            .display()
        );
    });
}

/// An added light ([`Kind::Light`], 08/11/2026) drawn as a dynamic light: duel6's floor
/// ahead of its first spawn seen without it, with a strong red light 40 units over it,
/// then without it again. On the shot's preset, as on every one: the frame's point-light
/// list draws it.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn added_light_drawn() {
    on_big_stack(|| {
        let map = std::env::var("SJK_SHOT_MAP").unwrap_or_else(|_| "mp/duel6".into());
        let Some((mut gpu, _profile)) = open(&format!("maps/{map}.bsp"), [1280, 720], None, &[])
        else {
            return;
        };
        let (spawn, yaw) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let camera = Vec3::from_array(spawn) + Vec3::Z * 32.0;
        let pitch = -25.0_f32;
        let forward = Vec3::new(
            yaw.to_radians().cos() * pitch.to_radians().cos(),
            yaw.to_radians().sin() * pitch.to_radians().cos(),
            pitch.to_radians().sin(),
        );
        let hit = gpu
            .decal_surfaces
            .ray_hit(camera, forward, 4096.0)
            .expect("the floor ahead");
        let at = camera + forward * hit.distance;
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        let before = frame(&mut gpu, 40);
        let mut document = Document::new(&map, 1);
        document.add(
            Kind::Light {
                origin: (at + Vec3::Z * 40.0).to_array(),
                color: [1.0, 0.15, 0.05],
                radius: 360.0,
                intensity: 3.0,
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        assert_eq!(gpu.map_edits.lights.len(), 1, "the light is applied");
        let lit = frame(&mut gpu, 40);
        gpu.shot_map_edits(None);
        let after = frame(&mut gpu, 40);
        let (changed, _) = changed_pixels(&before, &lit, 40);
        let (restored, _) = changed_pixels(&before, &after, 40);
        let redder = before
            .pixels()
            .zip(lit.pixels())
            .filter(|(a, b)| i32::from(b.0[0]) - i32::from(a.0[0]) > 24 && b.0[0] > b.0[2])
            .count();
        println!(
            "{map}: light over {at:?}; pixels changed: lit {changed} ({redder} redder), \
             restored {restored}"
        );
        assert!(redder > 5000, "the light reddens the floor round it");
        assert!(restored < changed / 20, "taking the light away restores it");
        println!(
            "{}",
            sheet(
                &[before, lit, after],
                3,
                960,
                &format!("added-light-{}", map.replace('/', "_"))
            )
            .display()
        );
    });
}
