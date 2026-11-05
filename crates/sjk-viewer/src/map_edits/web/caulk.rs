//! What the browser editor's brush view, linked caulk and nearby texture ask of the map
//! (round 3, Sol's asks of 03/11/2026), answered on the game thread from the loaded
//! BSP and kept per map ([`Cache`]): the invisible brush faces as `r_showBrushes` sorts
//! them ([`crate::brush_view::each_face`], so the page shows the game's own kinds and
//! colours) plus the liquid volumes, the faces joined to a caulk face
//! ([`crate::map_edits::linked`]), and the shaders round a set of faces
//! ([`crate::map_edits::nearby`]).

use crate::brush_view::kinds::{Kind, is_liquid};
use crate::map_edits::linked::{Grow, LINK_LIMIT, Sides};
use crate::map_edits::nearby::{Asking, DrawnSurfaces, is_effect, skipped_by_flags};
use glam::Vec3;
use serde_json::{Value, json};
use sjk_bsp::Bsp;

/// The page's own kind after `r_showBrushes`' kinds: water, lava and slime volumes.
pub(crate) const LIQUID: usize = Kind::ALL.len();
/// Its colour (display sRGB).
const LIQUID_COLOUR: [f32; 3] = [0.25, 0.55, 0.95];
/// The page's second own kind: the world's sky sides (04/11/2026), which a click selects
/// to cover with a texture or to swap the sky ([`crate::map_edits::faces`]).
pub(crate) const SKY: usize = LIQUID + 1;
const SKY_COLOUR: [f32; 3] = [0.35, 0.80, 0.98];
/// A side smaller than this (square units) is a bevel or a sliver, as the game's view.
const MIN_AREA: f32 = 0.5;

/// What the answers are built from, made once per map on first use.
#[derive(Default)]
pub(crate) struct Cache {
    map: String,
    brushes: Option<Value>,
    sides: Option<Sides>,
    drawn: Option<DrawnSurfaces>,
}

impl Cache {
    /// Forget what was made for another map than `map`.
    fn on(&mut self, map: &str) -> &mut Self {
        if self.map != map {
            *self = Self {
                map: map.to_owned(),
                ..Self::default()
            };
        }
        self
    }

    pub(crate) fn brushes(&mut self, map: &str, bsp: &Bsp) -> &Value {
        self.on(map)
            .brushes
            .get_or_insert_with(|| brushes_json(bsp))
    }

    pub(crate) fn sides(&mut self, map: &str, bsp: &Bsp) -> &Sides {
        self.on(map)
            .sides
            .get_or_insert_with(|| Sides::from_bsp(bsp))
    }

    /// The sides and the drawn surfaces that vote, made with `catalog`'s shaders.
    pub(crate) fn sides_and_drawn(
        &mut self,
        map: &str,
        bsp: &Bsp,
        catalog: Option<&sjk_shader::ShaderCatalog>,
    ) -> (&Sides, &DrawnSurfaces) {
        let cache = self.on(map);
        let sides = cache.sides.get_or_insert_with(|| Sides::from_bsp(bsp));
        let drawn = cache
            .drawn
            .get_or_insert_with(|| drawn_surfaces(bsp, catalog));
        (sides, drawn)
    }
}

fn rounded(point: &[f32; 3]) -> [f64; 3] {
    point.map(|v| (f64::from(v) * 100.0).round() / 100.0)
}

/// `api/brushes`: the kinds (name, label, colour, the `r_showBrushes` mode that shows
/// them, whether a click selects their faces) and every face as `[kind, brush, side,
/// [x, y, z, ...], normal]` (brush and side -1 for a mover's travel box).
pub(crate) fn brushes_json(bsp: &Bsp) -> Value {
    let mut faces: Vec<Value> = Vec::new();
    let mut counts = [0usize; SKY + 1];
    let mut push =
        |kind: usize, side: Option<(usize, usize)>, points: &[[f32; 3]], normal: [f32; 3]| {
            counts[kind] += 1;
            let (brush, side) = side.map_or((-1, -1), |(b, s)| (b as i64, s as i64));
            let flat: Vec<f64> = points.iter().flat_map(rounded).collect();
            faces.push(json!([kind, brush, side, flat, rounded(&normal)]));
        };
    crate::brush_view::each_face(bsp, |kind, side, points, normal| {
        push(kind.index(), side, points, normal);
    });
    if let Some(world) = bsp.render().models().first() {
        for brush_index in world.brushes.clone() {
            let Some(brush) = bsp.brushes().get(brush_index) else {
                continue;
            };
            let liquid = is_liquid(brush.content_flags);
            for side in brush.sides.clone() {
                let sky = bsp
                    .brush_sides()
                    .get(side)
                    .and_then(|side| bsp.shaders().get(side.shader))
                    .is_some_and(|shader| {
                        crate::map_edits::faces::is_sky_side(
                            brush.content_flags,
                            shader.surface_flags,
                        )
                    });
                if !liquid && !sky {
                    continue;
                }
                let points = bsp.brush_side_polygon(brush_index, side);
                let corners: Vec<Vec3> = points.iter().copied().map(Vec3::from_array).collect();
                let normal = bsp.brush_sides()[side].plane;
                if crate::map_edits::linked::area(&corners) >= MIN_AREA {
                    push(
                        if liquid { LIQUID } else { SKY },
                        Some((brush_index, side)),
                        &points,
                        bsp.planes()[normal].normal,
                    );
                }
            }
        }
    }
    let mut kinds: Vec<Value> = Kind::ALL
        .into_iter()
        .map(|kind| {
            json!({
                "name": kind.name(),
                "label": kind.label().join(" "),
                "colour": kind.colour(),
                "mode": if Kind::GAMEPLAY.contains(kind) { 1 } else { 2 },
                "selects": matches!(kind, Kind::Caulk | Kind::Solid | Kind::Nodraw),
                "count": counts[kind.index()],
            })
        })
        .collect();
    kinds.push(json!({
        "name": "water",
        "label": "WATER LAVA SLIME",
        "colour": LIQUID_COLOUR,
        "mode": 2,
        "selects": false,
        "count": counts[LIQUID],
    }));
    kinds.push(json!({
        "name": "sky",
        "label": "SKY",
        "colour": SKY_COLOUR,
        "mode": 2,
        "selects": true,
        "count": counts[SKY],
    }));
    json!({ "kinds": kinds, "faces": faces })
}

/// Whether `point` lies in an open leaf of `bsp` (one in a visibility cluster), not in
/// the solid void round the map or inside a structural brush.
pub(crate) fn open_at(bsp: &Bsp, point: Vec3) -> bool {
    if bsp.nodes().is_empty() {
        return true;
    }
    bsp.leaves()
        .get(bsp.leaf_at(point.to_array()))
        .is_some_and(|leaf| leaf.cluster >= 0)
}

/// `api/link`: the faces joined to side `side` of brush `brush` by `grow`, each with its
/// shader and corners, how many joined faces were left out as no player sees them, and
/// whether [`LINK_LIMIT`] stopped it.
pub(crate) fn link_json(
    bsp: &Bsp,
    sides: &Sides,
    brush: u32,
    side: u32,
    grow: Grow,
) -> Result<Value, String> {
    let start = sides
        .find(brush, side)
        .filter(|&index| sides.side(index).showable)
        .ok_or_else(|| {
            format!("Brush {brush} side {side} is not a caulk, nodraw or sky face of this map")
        })?;
    let open = |point: Vec3| open_at(bsp, point);
    let linked = sides.link(start, grow, LINK_LIMIT, &open);
    let faces: Vec<Value> = linked
        .faces
        .iter()
        .map(|&index| {
            let face = sides.side(index);
            let points: Vec<[f64; 3]> = face
                .polygon
                .iter()
                .map(|p| rounded(&p.to_array()))
                .collect();
            json!({ "brush": face.brush, "side": face.side, "shader": face.shader, "points": points, "sky": face.sky })
        })
        .collect();
    Ok(json!({
        "faces": faces,
        "buried": linked.buried,
        "capped": linked.capped,
        "limit": LINK_LIMIT,
    }))
}

/// `api/suggest`: the shaders the drawn surfaces round faces `faces` (brush, side) use,
/// the weightiest first.
pub(crate) fn suggest_json(sides: &Sides, drawn: &DrawnSurfaces, faces: &[[u32; 2]]) -> Value {
    let asking: Vec<Asking<'_>> = faces
        .iter()
        .filter_map(|&[brush, side]| sides.find(brush, side))
        .map(|index| {
            let face = sides.side(index);
            Asking {
                polygon: &face.polygon,
                normal: face.normal,
                distance: face.distance,
            }
        })
        .collect();
    json!({ "faces": asking.len(), "candidates": drawn.vote(&asking) })
}

/// The world's drawn surfaces that may vote: not sky, nodraw, caulk, triggers, liquids
/// or see-through by their flags, and not an effect by their script in `catalog`.
pub(crate) fn drawn_surfaces(
    bsp: &Bsp,
    catalog: Option<&sjk_shader::ShaderCatalog>,
) -> DrawnSurfaces {
    DrawnSurfaces::from_bsp(bsp, |shader| {
        let name = shader.name_lossy();
        skipped_by_flags(&name, shader.surface_flags, shader.content_flags)
            || catalog
                .and_then(|catalog| catalog.get(&name))
                .is_some_and(is_effect)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The brush view lists the sky sides as a kind a click selects, and a link from one
    /// takes the sky only.
    #[test]
    fn the_brush_view_offers_the_sky() {
        let bsp = crate::map_edits::faces::tests::sky_room();
        let view = brushes_json(&bsp);
        let sky = view["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|kind| kind["name"] == "sky")
            .expect("a sky kind");
        assert_eq!(sky["selects"], true);
        // the lid's six sides; the sky-flagged water is a liquid
        assert_eq!(sky["count"], 6);
        let faces: Vec<&Value> = view["faces"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|face| face[0] == SKY)
            .collect();
        assert_eq!(faces.len(), 6);
        let sides = Sides::from_bsp(&bsp);
        let [_, brush, side, ..] = faces[0].as_array().unwrap().as_slice() else {
            panic!("a face");
        };
        let (brush, side) = (
            brush.as_u64().unwrap() as u32,
            side.as_u64().unwrap() as u32,
        );
        let linked = link_json(&bsp, &sides, brush, side, Grow::Surface).unwrap();
        let linked = linked["faces"].as_array().unwrap();
        assert!(!linked.is_empty());
        assert!(linked.iter().all(|face| face["sky"] == true));
    }

    /// On retail maps (the GameData folder in `SJK_MAPEDIT_WEB_GAMEDATA`, maps in
    /// `SJK_CAULK_MAPS`, mp/duel6 and mp/ffa3 by default): how long the caches take, and
    /// for a few seen caulk faces, what links and what the vote suggests.
    #[test]
    #[ignore = "reads the retail game data"]
    fn linked_caulk_and_nearby_textures_on_retail_maps() {
        let base = std::env::var("SJK_MAPEDIT_WEB_GAMEDATA")
            .expect("SJK_MAPEDIT_WEB_GAMEDATA: the GameData folder");
        let maps = std::env::var("SJK_CAULK_MAPS").unwrap_or_else(|_| "mp/duel6 mp/ffa3".into());
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_pk3_directory(std::path::Path::new(&base).join("base"))
            .unwrap();
        let catalog = sjk_shader::ShaderCatalog::load_with_warnings(&vfs, |_, _| {});
        for map in maps.split_whitespace() {
            let bytes = vfs.read(&format!("maps/{map}.bsp")).unwrap().unwrap().bytes;
            let bsp = Bsp::parse(&bytes).unwrap();
            let started = std::time::Instant::now();
            let brushes = brushes_json(&bsp);
            let brushes_time = started.elapsed();
            let size = serde_json::to_vec(&brushes).unwrap().len();
            let started = std::time::Instant::now();
            let sides = Sides::from_bsp(&bsp);
            let sides_time = started.elapsed();
            let started = std::time::Instant::now();
            let drawn = drawn_surfaces(&bsp, Some(&catalog));
            let drawn_time = started.elapsed();
            println!(
                "{map}: brush view {} faces, {} KB, {brushes_time:?}; sides {sides_time:?}; \
                 drawn surfaces {drawn_time:?}",
                brushes["faces"].as_array().unwrap().len(),
                size / 1024
            );
            let open = |point: Vec3| open_at(&bsp, point);
            // The cover test against q3map2's own: a drawn side whose surface it kept (its
            // draw surface number) should be seen, one it dropped not.
            // (kept: a planar surface of its shader lies on its plane over it)
            let render = bsp.render();
            // (planar surfaces and the triangle soups -meta makes of brush faces, by triangle)
            let planar: Vec<(usize, Vec3, f32, Vec3, Vec3)> = render.models()[0]
                .surfaces
                .clone()
                .flat_map(|index| {
                    let surface = &render.surfaces()[index];
                    let at = |i: u32| {
                        Vec3::from_array(
                            render.vertices()[surface.vertices.start + i as usize].position,
                        )
                    };
                    render.indices()[surface.indices.clone()]
                        .chunks_exact(3)
                        .filter_map(|t| {
                            let points = [at(t[0]), at(t[1]), at(t[2])];
                            let normal = -(points[1] - points[0])
                                .cross(points[2] - points[0])
                                .try_normalize()?;
                            let (low, high) = crate::map_edits::linked::bounds(&points);
                            Some((surface.shader, normal, normal.dot(points[0]), low, high))
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            let mut agree = [[0usize; 2]; 2];
            for i in 0..sides.len() {
                let face = sides.side(i);
                let side = &bsp.brush_sides()[face.side as usize];
                let flags = bsp.shaders()[side.shader].surface_flags;
                let contents = bsp.shaders()[side.shader].content_flags;
                // (models/ shaders: a misc_model's clip brushes, which q3map2 never draws)
                if face.showable
                    || flags & 0x0020_2000 != 0
                    || contents & 0x2000_0000 != 0
                    || face.shader.starts_with("models/")
                {
                    continue;
                }
                let (low, high) = crate::map_edits::linked::bounds(&face.polygon);
                let kept = planar.iter().any(|(shader, normal, distance, a, b)| {
                    bsp.shaders()[*shader].name() == bsp.shaders()[side.shader].name()
                        && normal.dot(face.normal).abs() > 0.99
                        && (distance * normal.dot(face.normal).signum() - face.distance).abs() < 0.5
                        && crate::map_edits::linked::overlaps(
                            (*a + *b) * 0.5,
                            (*a + *b) * 0.5,
                            low - Vec3::splat(0.5),
                            high + Vec3::splat(0.5),
                        )
                });
                let seen = sides.visible(i, &open);
                agree[usize::from(kept)][usize::from(seen)] += 1;
            }
            println!(
                "{map}: drawn sides kept by q3map2 and seen {}, kept not seen {}, dropped but \
                 seen {}, dropped not seen {}",
                agree[1][1], agree[1][0], agree[0][1], agree[0][0]
            );
            // the caulk faces players see, biggest first
            let mut seen: Vec<(usize, f32)> = (0..sides.len())
                .filter(|&i| {
                    let face = sides.side(i);
                    face.showable && face.shader.contains("caulk")
                })
                .filter(|&i| sides.visible(i, &open))
                .map(|i| (i, crate::map_edits::linked::area(&sides.side(i).polygon)))
                .collect();
            println!("{map}: {} caulk faces seen", seen.len());
            // the seen caulk faces nearest the first spawn points: where players look
            let spawns: Vec<Vec3> = sjk_entity::parse_entity_lump(bsp.entities())
                .unwrap()
                .iter()
                .filter(|e| {
                    e.classname()
                        .is_some_and(|c| c.starts_with("info_player_deathmatch"))
                })
                .filter_map(|e| e.vector("origin").ok().flatten())
                .map(Vec3::from_array)
                .take(3)
                .collect();
            let mut picks: Vec<(usize, f32)> = Vec::new();
            for spawn in &spawns {
                seen.sort_by(|a, b| {
                    let d = |i: usize| {
                        crate::map_edits::linked::centre(&sides.side(i).polygon).distance(*spawn)
                    };
                    d(a.0).total_cmp(&d(b.0))
                });
                if let Some(&pick) = seen.iter().find(|p| !picks.contains(p)) {
                    picks.push(pick);
                }
            }
            seen.sort_by(|a, b| b.1.total_cmp(&a.1));
            picks.extend(seen.first().copied());
            for &(index, area) in &picks {
                let face = sides.side(index);
                for grow in [Grow::Plane, Grow::Surface] {
                    let started = std::time::Instant::now();
                    let linked = sides.link(index, grow, LINK_LIMIT, &open);
                    let time = started.elapsed();
                    let faces: Vec<[u32; 2]> = linked
                        .faces
                        .iter()
                        .map(|&i| [sides.side(i).brush, sides.side(i).side])
                        .collect();
                    let votes = suggest_json(&sides, &drawn, &faces);
                    let top: Vec<String> = votes["candidates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .take(3)
                        .map(|c| {
                            format!(
                                "{} {:.0}%",
                                c["shader"].as_str().unwrap(),
                                c["share"].as_f64().unwrap() * 100.0
                            )
                        })
                        .collect();
                    let middle = crate::map_edits::linked::centre(&face.polygon);
                    println!(
                        "  brush {} side {} ({area:.0} u2, at {:.0} {:.0} {:.0}, normal {:.1} \
                         {:.1} {:.1}) {grow:?}: {} faces, {} buried left out{} in {time:?}; \
                         nearby: {}",
                        face.brush,
                        face.side,
                        middle.x,
                        middle.y,
                        middle.z,
                        face.normal.x,
                        face.normal.y,
                        face.normal.z,
                        linked.faces.len(),
                        linked.buried,
                        if linked.capped { ", capped" } else { "" },
                        top.join(", ")
                    );
                }
            }
        }
    }
}
