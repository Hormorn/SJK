//! The nearby texture: for caulk or nodraw faces about to be shown ([`super::faces`]),
//! the shader the drawn surfaces around them use, so the browser map editor can fill
//! them with it in one go (Sol, 03/11/2026: "replace visible caulks by nearby
//! textures").
//!
//! The map's own drawn surfaces vote, each shader by how much of it meets the faces:
//! the length of edge a drawn triangle shares with a face (three times over when it lies
//! on the face's plane, the face's wall going on), and, for a triangle on the face's
//! plane within [`NEAR`] units of it, the square root of its area (a wall of the same
//! plane round a hole). Sky, nodraw, caulk, triggers, liquids, see-through and effect
//! shaders do not vote ([`DrawnSurfaces::from_bsp`]'s `skip`).

use super::linked::{Grid, area, bounds, centre, overlaps, shared_length};
use glam::Vec3;
use sjk_bsp::{Bsp, SurfaceKind};
use std::collections::HashMap;

/// How far from a face a drawn surface on its plane still votes (units).
const NEAR: f32 = 24.0;
/// How far apart an edge of a triangle and an edge of a face may lie and be one.
const EDGE_EPSILON: f32 = 1.0;
/// How far a triangle may lie off a face's plane and be on it.
const PLANE_EPSILON: f32 = 1.0;
/// How many candidates a vote names.
pub(crate) const CANDIDATES: usize = 6;

/// One drawn surface of the world: its shader and its triangles.
pub(crate) struct DrawnSurface {
    /// Its BSP surface number (`usize::MAX` when made up): a surface asking does not
    /// vote for itself.
    pub(crate) index: usize,
    pub(crate) shader: String,
    pub(crate) triangles: Vec<[Vec3; 3]>,
    low: Vec3,
    high: Vec3,
}

impl DrawnSurface {
    pub(crate) fn new(shader: &str, triangles: Vec<[Vec3; 3]>) -> Self {
        let corners: Vec<Vec3> = triangles.iter().flatten().copied().collect();
        let (low, high) = bounds(&corners);
        Self {
            index: usize::MAX,
            shader: shader.to_owned(),
            triangles,
            low,
            high,
        }
    }
}

/// The world's drawn surfaces that may vote, found by place.
pub(crate) struct DrawnSurfaces {
    surfaces: Vec<DrawnSurface>,
    grid: Grid,
    /// Each shader with how many surfaces draw it, the most used first: the textures the
    /// map itself uses ([`super::suggest`]).
    usage: Vec<(String, u32)>,
}

/// A shader the vote names, with its weight and its share of all the weight.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct Candidate {
    pub(crate) shader: String,
    pub(crate) weight: f32,
    pub(crate) share: f32,
}

/// A face that asks: its corners and its plane.
pub(crate) struct Asking<'a> {
    pub(crate) polygon: &'a [Vec3],
    pub(crate) normal: Vec3,
    pub(crate) distance: f32,
}

impl DrawnSurfaces {
    /// The world model's planar surfaces, triangle soups (merged models) and patches (as
    /// their control grids) of `bsp`, without those whose shader `skip` refuses.
    pub(crate) fn from_bsp(bsp: &Bsp, skip: impl Fn(&sjk_bsp::Shader) -> bool) -> Self {
        let render = bsp.render();
        let Some(world) = render.models().first() else {
            return Self::new(Vec::new());
        };
        let mut surfaces = Vec::new();
        for index in world.surfaces.clone() {
            let Some(surface) = render.surfaces().get(index) else {
                continue;
            };
            let Some(shader) = bsp.shaders().get(surface.shader) else {
                continue;
            };
            if skip(shader) {
                continue;
            }
            let triangles = surface_triangles(bsp, index);
            if !triangles.is_empty() {
                let mut drawn = DrawnSurface::new(&shader.name_lossy(), triangles);
                drawn.index = index;
                surfaces.push(drawn);
            }
        }
        Self::new(surfaces)
    }

    pub(crate) fn new(surfaces: Vec<DrawnSurface>) -> Self {
        let mut grid = Grid::default();
        let mut counts: HashMap<String, u32> = HashMap::new();
        for (index, surface) in surfaces.iter().enumerate() {
            grid.insert(surface.low, surface.high, index);
            *counts
                .entry(surface.shader.to_ascii_lowercase())
                .or_default() += 1;
        }
        let mut usage: Vec<(String, u32)> = counts.into_iter().collect();
        usage.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Self {
            surfaces,
            grid,
            usage,
        }
    }

    /// Each shader (lower case) with how many of the voting surfaces draw it, the most
    /// used first.
    pub(crate) fn usage(&self) -> &[(String, u32)] {
        &self.usage
    }

    /// The shaders round `faces`, the most weighty first, [`CANDIDATES`] at most.
    pub(crate) fn vote(&self, faces: &[Asking<'_>]) -> Vec<Candidate> {
        self.vote_except(faces, &[])
    }

    /// [`Self::vote`] without the votes of surfaces `skip` (BSP numbers): the surfaces that
    /// ask, so a surface does not vote for itself.
    pub(crate) fn vote_except(&self, faces: &[Asking<'_>], skip: &[usize]) -> Vec<Candidate> {
        let mut weights: HashMap<&str, f32> = HashMap::new();
        for face in faces {
            if face.polygon.len() < 3 {
                continue;
            }
            let (low, high) = bounds(face.polygon);
            let reach = Vec3::splat(NEAR);
            let middle = centre(face.polygon);
            for index in self.grid.near(low - reach, high + reach) {
                let surface = &self.surfaces[index];
                if skip.contains(&surface.index) {
                    continue;
                }
                if !overlaps(surface.low, surface.high, low - reach, high + reach) {
                    continue;
                }
                let mut weight = 0.0;
                for triangle in &surface.triangles {
                    let normal = (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]);
                    let Some(normal) = normal.try_normalize() else {
                        continue;
                    };
                    // q3map2 winds drawn triangles clockwise seen from the front.
                    let normal = -normal;
                    let facing = normal.dot(face.normal);
                    if facing < -0.3 {
                        // a back side
                        continue;
                    }
                    let on_plane = facing > 0.99
                        && triangle
                            .iter()
                            .all(|p| (face.normal.dot(*p) - face.distance).abs() <= PLANE_EPSILON);
                    let edge = shared_length(face.polygon, triangle, EDGE_EPSILON);
                    weight += edge * if on_plane { 3.0 } else { 1.0 };
                    if on_plane {
                        let gap = gap_to(face.polygon, middle, triangle);
                        if gap <= NEAR {
                            weight += area(triangle).sqrt() * (1.0 - gap / NEAR);
                        }
                    }
                }
                if weight > 0.0 {
                    *weights.entry(surface.shader.as_str()).or_default() += weight;
                }
            }
        }
        let total: f32 = weights.values().sum();
        let mut candidates: Vec<Candidate> = weights
            .into_iter()
            .map(|(shader, weight)| Candidate {
                shader: shader.to_owned(),
                weight,
                share: if total > 0.0 { weight / total } else { 0.0 },
            })
            .collect();
        candidates.sort_by(|a, b| {
            b.weight
                .total_cmp(&a.weight)
                .then_with(|| a.shader.cmp(&b.shader))
        });
        candidates.truncate(CANDIDATES);
        candidates
    }
}

/// The triangles of surface `index` of `bsp` as drawn (planar surfaces and triangle soups
/// by their indices, patches by their control grid, flares none), wound as q3map2 winds
/// them (clockwise seen from the front).
pub(crate) fn surface_triangles(bsp: &Bsp, index: usize) -> Vec<[Vec3; 3]> {
    let render = bsp.render();
    let Some(surface) = render.surfaces().get(index) else {
        return Vec::new();
    };
    let vertices = render.vertices();
    let at = |i: usize| {
        vertices
            .get(surface.vertices.start + i)
            .map(|v| Vec3::from_array(v.position))
    };
    let mut triangles = Vec::new();
    match surface.kind {
        SurfaceKind::Planar | SurfaceKind::TriangleSoup => {
            for corner in render.indices()[surface.indices.clone()].chunks_exact(3) {
                if let (Some(a), Some(b), Some(c)) = (
                    at(corner[0] as usize),
                    at(corner[1] as usize),
                    at(corner[2] as usize),
                ) {
                    triangles.push([a, b, c]);
                }
            }
        }
        SurfaceKind::Patch => {
            let Some([width, height]) = surface.patch_dimensions else {
                return Vec::new();
            };
            for row in 0..height - 1 {
                for column in 0..width - 1 {
                    let i = row * width + column;
                    if let (Some(a), Some(b), Some(c), Some(d)) =
                        (at(i), at(i + 1), at(i + width), at(i + width + 1))
                    {
                        triangles.push([a, c, b]);
                        triangles.push([b, c, d]);
                    }
                }
            }
        }
        SurfaceKind::Flare => {}
    }
    triangles
}

/// The planes a drawn surface's `triangles` ask a vote with ([`Asking`]): each triangle
/// (its corners, its front's normal and its plane's distance), at most `most`, taken
/// evenly from the list.
pub(crate) fn surface_asking(triangles: &[[Vec3; 3]], most: usize) -> Vec<([Vec3; 3], Vec3, f32)> {
    let step = triangles.len().div_ceil(most.max(1)).max(1);
    triangles
        .iter()
        .step_by(step)
        .filter_map(|triangle| {
            let normal = -(triangle[1] - triangle[0])
                .cross(triangle[2] - triangle[0])
                .try_normalize()?;
            Some((*triangle, normal, normal.dot(triangle[0])))
        })
        .collect()
}

/// Roughly how far triangle `triangle` lies from polygon `polygon` (whose middle is
/// `middle`): 0 when a corner of one is within the other's reach.
fn gap_to(polygon: &[Vec3], middle: Vec3, triangle: &[Vec3; 3]) -> f32 {
    let radius = polygon
        .iter()
        .map(|p| p.distance(middle))
        .fold(0.0f32, f32::max);
    let nearest = triangle
        .iter()
        .map(|p| p.distance(middle))
        .fold(f32::MAX, f32::min);
    (nearest - radius).max(0.0)
}

/// Whether a shader of the map does not vote: sky, nodraw (caulk among them), triggers,
/// liquids, fog, see-through brushes, by its compiled flags.
pub(crate) fn skipped_by_flags(name: &str, surface_flags: u32, content_flags: u32) -> bool {
    const SURF_SKY: u32 = 0x0000_2000;
    const SURF_NODRAW: u32 = 0x0020_0000;
    // lava, water, fog, trigger, slime, translucent
    const CONTENTS_SKIPPED: u32 = 0x2 | 0x4 | 0x8 | 0x400 | 0x0002_0000 | 0x8000_0000;
    surface_flags & (SURF_SKY | SURF_NODRAW) != 0
        || content_flags & CONTENTS_SKIPPED != 0
        || name.to_ascii_lowercase().contains("caulk")
}

/// Whether a shader script makes an effect rather than a wall: no colour pass, a sky, a
/// first stage blended over what is behind or cut by its alpha (grates, glass, glows,
/// decals), or its corners moved (autosprites, flags).
pub(crate) fn is_effect(definition: &sjk_shader::ShaderDefinition) -> bool {
    if !definition.has_color_pass() || definition.sky.is_some() {
        return true;
    }
    if !definition.deforms.is_empty() || definition.polygon_offset {
        return true;
    }
    let Some(first) = definition.stages.first() else {
        return true;
    };
    let opaque = match &first.blend {
        sjk_shader::StageBlend::Replace => true,
        sjk_shader::StageBlend::Custom {
            source,
            destination,
        } => source.eq_ignore_ascii_case("gl_one") && destination.eq_ignore_ascii_case("gl_zero"),
        _ => false,
    };
    !opaque || first.alpha_function.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The square from `low` to `high` on the floor z = 0 as two triangles wound as
    /// q3map2 winds them (clockwise from above).
    fn floor(low: [f32; 2], high: [f32; 2]) -> Vec<[Vec3; 3]> {
        let p = |x: f32, y: f32| Vec3::new(x, y, 0.0);
        let (a, b, c, d) = (
            p(low[0], low[1]),
            p(high[0], low[1]),
            p(high[0], high[1]),
            p(low[0], high[1]),
        );
        vec![[a, c, b], [a, d, c]]
    }

    #[test]
    fn the_floor_round_a_hole_outvotes_a_wall_beside_it() {
        // A caulk hole x 64-128, y 0-64 in a floor of stone, a strip of metal along one
        // side further off, and a wall rising from the hole's far edge.
        let hole = [
            Vec3::new(64.0, 0.0, 0.0),
            Vec3::new(128.0, 0.0, 0.0),
            Vec3::new(128.0, 64.0, 0.0),
            Vec3::new(64.0, 64.0, 0.0),
        ];
        let wall = {
            let p = |y: f32, z: f32| Vec3::new(128.0, y, z);
            // facing -x, clockwise seen from -x
            vec![
                [p(0.0, 0.0), p(64.0, 0.0), p(64.0, 64.0)],
                [p(0.0, 0.0), p(64.0, 64.0), p(0.0, 64.0)],
            ]
        };
        let drawn = DrawnSurfaces::new(vec![
            DrawnSurface::new("textures/a/stone", floor([0.0, 0.0], [64.0, 64.0])),
            DrawnSurface::new("textures/a/stone", floor([64.0, 64.0], [128.0, 128.0])),
            DrawnSurface::new("textures/a/metal", floor([64.0, -40.0], [128.0, -20.0])),
            DrawnSurface::new("textures/a/wall", wall),
            // far away: no vote
            DrawnSurface::new("textures/a/far", floor([1000.0, 0.0], [1064.0, 64.0])),
        ]);
        let votes = drawn.vote(&[Asking {
            polygon: &hole,
            normal: Vec3::Z,
            distance: 0.0,
        }]);
        let names: Vec<&str> = votes.iter().map(|c| c.shader.as_str()).collect();
        assert_eq!(names[0], "textures/a/stone");
        assert!(names.contains(&"textures/a/wall"));
        assert!(names.contains(&"textures/a/metal"));
        assert!(!names.contains(&"textures/a/far"));
        // stone: two edges of 64 on the plane (three times) and two near squares
        assert!(votes[0].weight > 2.0 * 64.0 * 3.0);
        let wall_vote = votes
            .iter()
            .find(|c| c.shader == "textures/a/wall")
            .unwrap();
        assert!((wall_vote.weight - 64.0).abs() < 1.0, "{wall_vote:?}");
        let shares: f32 = votes.iter().map(|c| c.share).sum();
        assert!((shares - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_back_of_a_floor_does_not_vote() {
        let hole = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(64.0, 0.0, 0.0),
            Vec3::new(64.0, 64.0, 0.0),
            Vec3::new(0.0, 64.0, 0.0),
        ];
        // a ceiling below, facing down, flush with the hole
        let ceiling: Vec<[Vec3; 3]> = floor([64.0, 0.0], [128.0, 64.0])
            .into_iter()
            .map(|[a, b, c]| [a, c, b])
            .collect();
        let drawn = DrawnSurfaces::new(vec![DrawnSurface::new("textures/a/under", ceiling)]);
        let votes = drawn.vote(&[Asking {
            polygon: &hole,
            normal: Vec3::Z,
            distance: 0.0,
        }]);
        assert!(votes.is_empty(), "{votes:?}");
    }

    #[test]
    fn skies_caulk_and_triggers_do_not_vote() {
        assert!(skipped_by_flags("textures/skies/a", 0x2000, 0));
        assert!(skipped_by_flags("textures/system/caulk", 0x20_0000, 1));
        assert!(skipped_by_flags("textures/x/caulk_wall", 0, 1));
        assert!(skipped_by_flags("textures/system/trigger", 0, 0x400));
        assert!(skipped_by_flags("textures/x/water", 0, 0x4));
        assert!(skipped_by_flags("textures/x/glass", 0, 0x8000_0001));
        // retail physics_clip, a trigger and duel6's ground plants are `trans`
        assert!(skipped_by_flags(
            "textures/yavin/ground_plants",
            0,
            0x8000_8001
        ));
        assert!(!skipped_by_flags("textures/x/stone", 0, 1));
    }
}
