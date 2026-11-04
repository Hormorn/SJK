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
        let bsp = &gpu.bsp;
        let world = bsp.render().models()[0].brushes.clone();
        let mut found = None;
        'brushes: for brush in world {
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
                    && picked.is_some_and(|face| {
                        face.brush as usize == brush && face.side as usize == side
                    })
                {
                    found = Some((brush, side, shader.name_lossy().into_owned(), centre, eye));
                    break 'brushes;
                }
            }
        }
        let Some((brush, side, shader, centre, eye)) = found else {
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
