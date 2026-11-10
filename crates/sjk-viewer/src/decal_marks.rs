//! Projects decal marks onto the static world.
//!
//! Mirrors `RE_AddDecalToScene` (rd-vanilla `tr_decals.cpp:160-251`): build
//! the texture axes from the impact normal and the random orientation, clip
//! the radius-sized square into the world with the engine-generic
//! `R_MarkFragments` port in `sjk-bsp`, and map every fragment point back to
//! texture space with `0.5 + dot(delta, axis) * 0.5 / radius`. The surface
//! walk (`R_MarkFragments`, `tr_marks.cpp:256-453`) feeds planar faces whole,
//! curved patches triangle by triangle behind a facing test and skips
//! triangle soups (`r_marksOnTriangleMeshes` defaults to 0).
//!
//! The world triangles come from the same tessellated batches the renderer
//! draws, so marks follow the rendered patch geometry exactly. Surfaces the
//! game forbids marks on are filtered with the caller-supplied predicate the
//! clipper exposes; the JKA flag values live in `particle_physics`.

use super::*;
use crate::particle_physics::{CONTENTS_FOG, SURF_NOIMPACT, SURF_NOMARKS};
use sjk_bsp::{MAX_VERTICES_ON_POLY, MarkFragments, MarkProjection, SurfaceKind};

/// `MAX_DECAL_POINTS` / `MAX_DECAL_FRAGMENTS` (`tr_decals.cpp:157-158`).
const MAX_DECAL_POINTS: usize = 384;
const MAX_DECAL_FRAGMENTS: usize = 128;
/// Surface list capacity handed to `R_BoxSurfaces_r` (`tr_marks.cpp:313`).
const MAX_MARK_SURFACES: usize = 64;
/// A tag's budget: it lasts and is wider than a mark, so a curved patch or a face cut
/// into many triangles must not run it out and leave the picture cut short.
const MAX_TAG_POINTS: usize = 4_096;
const MAX_TAG_FRAGMENTS: usize = 512;
const MAX_TAG_SURFACES: usize = 128;
/// `MAX_VERTS_ON_DECAL_POLY` (`tr_decals.cpp:26`).
pub(crate) const MAX_VERTICES_ON_DECAL: usize = 10;
/// Decals project 20 units into the surface (`tr_decals.cpp:198`).
const PROJECTION_DEPTH: f32 = 20.0;
/// Curved-patch triangles must face the projection (`tr_marks.cpp:364`).
/// The reference alternates -0.1 / -0.05 per cell half; this applies the
/// stricter limit to every triangle.
const GRID_FACING_LIMIT: f32 = -0.1;

/// One mark to project, as handed over by `FxScheduler` (`FxScheduler.cpp:1621`).
#[derive(Clone, Debug)]
pub(crate) struct DecalRequest {
    pub(crate) origin: Vec3,
    /// Impact normal; the mark faces along it.
    pub(crate) direction: Vec3,
    /// Texture rotation about the normal in degrees.
    pub(crate) orientation: f32,
    pub(crate) color: [f32; 4],
    pub(crate) radius: f32,
    pub(crate) shader: Arc<str>,
}

/// World-space point of a clipped decal with its texture coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DecalVertex {
    pub(crate) position: [f32; 3],
    pub(crate) st: [f32; 2],
}

/// World triangles grouped by BSP surface, captured once at load.
pub(crate) struct DecalSurfaces {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    /// Index range per BSP surface; empty for surfaces the renderer skipped.
    ranges: Vec<Range<u32>>,
    /// Surfaces a map edit hides ([`crate::map_edits`]): no marks, no picks.
    hidden: Vec<bool>,
    /// Surfaces a tag is not put on ([`tag_unfit`]): moving, see-through, cut out or
    /// laid over another surface.
    no_tags: Vec<bool>,
    /// Each surface's bounds (low, high): a ray tests the triangles of the surfaces whose
    /// box it meets nearer than its nearest hit only (the map editor's hover and box
    /// cast many rays).
    bounds: Vec<(Vec3, Vec3)>,
}

/// Whether a tag stays off a surface drawn with `shader`: the tag is a flat picture fixed
/// on the world, so on a surface that moves (`deformVertexes`), is seen through (glass,
/// water, force fields, a blended first stage), has holes cut out (`alphaFunc` grates,
/// leaves), is a decal on another surface (`polygonOffset`, whose wall under it takes
/// the tag) or is sky or fog it hung in the air, fought the surface's depth or showed
/// twice.
pub(crate) fn tag_unfit(shader: &sjk_shader::ShaderDefinition) -> bool {
    use sjk_shader::Deform;
    let moves = shader
        .deforms
        .iter()
        .any(|deform| !matches!(deform, Deform::Normals { .. } | Deform::Text(_)));
    let first = shader
        .stages
        .iter()
        .find(|stage| stage.surface_sprites.is_none());
    let see_through = first.is_none_or(|stage| {
        stage.blend != sjk_shader::StageBlend::Replace || stage.alpha_function.is_some()
    });
    // Q3's `SS_OPAQUE` is 3: anything sorted after is blended.
    let sorted_late = shader.sort.is_some_and(|sort| sort > 3.0);
    moves
        || see_through
        || sorted_late
        || shader.polygon_offset
        || shader.sky.is_some()
        || shader.fog.is_some()
}

impl DecalSurfaces {
    /// Capture the world batches. Only draws that belong to the world model
    /// take part (`R_BoxSurfaces_r` walks `tr.world->nodes`, so marks never
    /// land on inline models).
    pub(crate) fn from_flattened(
        scene: &FlattenedScene,
        bsp: &Bsp,
        shaders: &sjk_shader::ShaderCatalog,
    ) -> Self {
        let mut ranges = vec![0..0; bsp.render().surfaces().len()];
        let mut no_tags = vec![false; ranges.len()];
        for draw in &scene.draws {
            let Some(surface_index) = draw.surface_index else {
                continue;
            };
            if !draw.world_surface || surface_index >= ranges.len() {
                continue;
            }
            if ranges[surface_index].is_empty() {
                ranges[surface_index] = draw.indices.clone();
                no_tags[surface_index] = scene
                    .materials
                    .get(draw.material)
                    .and_then(|material| shaders.get(&material.shader))
                    .is_some_and(tag_unfit);
            }
        }
        let bounds = ranges
            .iter()
            .map(|range| {
                scene.indices[range.start as usize..range.end as usize]
                    .iter()
                    .filter_map(|&index| scene.vertices.get(index as usize))
                    .map(|vertex| Vec3::from_array(vertex.position))
                    .fold((Vec3::MAX, Vec3::MIN), |(low, high), p| {
                        (low.min(p), high.max(p))
                    })
            })
            .collect();
        Self {
            positions: scene
                .vertices
                .iter()
                .map(|vertex| vertex.position)
                .collect(),
            normals: scene.vertices.iter().map(|vertex| vertex.normal).collect(),
            indices: scene.indices.clone(),
            hidden: vec![false; ranges.len()],
            no_tags,
            ranges,
            bounds,
        }
    }

    /// Whether a map edit hides `surface` (or the renderer skipped it): it draws nothing.
    pub(crate) fn is_hidden(&self, surface: usize) -> bool {
        self.hidden.get(surface).copied().unwrap_or(true)
            || self
                .ranges
                .get(surface)
                .is_none_or(|range| range.is_empty())
    }

    /// Hide `surface` from marks and picks, or show it again; true when that changed.
    pub(crate) fn set_hidden(&mut self, surface: usize, hidden: bool) -> bool {
        match self.hidden.get_mut(surface) {
            Some(state) if *state != hidden => {
                *state = hidden;
                true
            }
            _ => false,
        }
    }

    /// Where `surface` lies in the world's index buffer, and its indices as loaded.
    /// `None` for a surface the renderer did not draw.
    pub(crate) fn surface_indices(&self, surface: usize) -> Option<(Range<u32>, &[u32])> {
        let range = self.ranges.get(surface).filter(|range| !range.is_empty())?;
        Some((
            range.clone(),
            &self.indices[range.start as usize..range.end as usize],
        ))
    }

    /// `surface` and the surfaces touching it, through shared corners, among those
    /// `joins` accepts, nearest first and at most `limit`: the pieces a compiled model
    /// (a `misc_model` the map compiler merged into the world) is left as.
    pub(crate) fn connected(
        &self,
        surface: usize,
        joins: impl Fn(usize) -> bool,
        limit: usize,
    ) -> Vec<usize> {
        // Corners an eighth of a unit apart are the same corner.
        let key =
            |index: u32| self.positions[index as usize].map(|axis| (axis * 8.0).round() as i32);
        let mut by_corner: std::collections::HashMap<[i32; 3], Vec<usize>> = Default::default();
        for (other, range) in self.ranges.iter().enumerate() {
            if other != surface && (!joins(other) || self.hidden[other]) {
                continue;
            }
            for &index in &self.indices[range.start as usize..range.end as usize] {
                let list = by_corner.entry(key(index)).or_default();
                if list.last() != Some(&other) {
                    list.push(other);
                }
            }
        }
        let mut found = vec![surface];
        let mut seen = std::collections::HashSet::from([surface]);
        let mut next = 0;
        while next < found.len() && found.len() < limit {
            let range = self.ranges[found[next]].clone();
            next += 1;
            for &index in &self.indices[range.start as usize..range.end as usize] {
                for &other in by_corner.get(&key(index)).into_iter().flatten() {
                    if found.len() < limit && seen.insert(other) {
                        found.push(other);
                    }
                }
            }
        }
        found
    }

    fn triangles(&self, surface_index: usize) -> impl Iterator<Item = [usize; 3]> + '_ {
        let range = if self.hidden.get(surface_index).copied().unwrap_or(false) {
            0..0
        } else {
            self.ranges.get(surface_index).cloned().unwrap_or(0..0)
        };
        self.indices[range.start as usize..range.end as usize]
            .chunks_exact(3)
            .map(|triangle| {
                [
                    triangle[0] as usize,
                    triangle[1] as usize,
                    triangle[2] as usize,
                ]
            })
    }

    /// The nearest world triangle the ray from `origin` along unit `direction` meets
    /// within `reach`, either side (`world_notes`).
    pub(crate) fn ray_hit(&self, origin: Vec3, direction: Vec3, reach: f32) -> Option<RayHit> {
        let mut nearest: Option<RayHit> = None;
        let inverse = direction.recip();
        for surface in 0..self.ranges.len() {
            // The surface's box first: missed, or entered beyond the nearest hit, skip it.
            if let Some(&(low, high)) = self.bounds.get(surface) {
                let pad = Vec3::splat(0.01);
                let a = (low - pad - origin) * inverse;
                let b = (high + pad - origin) * inverse;
                let near = a.min(b).max_element().max(0.0);
                let far = a.max(b).min_element();
                if far < near
                    || near > reach
                    || nearest.as_ref().is_some_and(|hit| hit.distance < near)
                {
                    continue;
                }
            }
            for triangle in self.triangles(surface) {
                let [a, b, c] = triangle.map(|index| Vec3::from_array(self.positions[index]));
                // Moeller-Trumbore.
                let (edge1, edge2) = (b - a, c - a);
                let p = direction.cross(edge2);
                let determinant = edge1.dot(p);
                if determinant.abs() < 1e-8 {
                    continue;
                }
                let inverse = 1.0 / determinant;
                let s = origin - a;
                let u = s.dot(p) * inverse;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = s.cross(edge1);
                let v = direction.dot(q) * inverse;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let distance = edge2.dot(q) * inverse;
                if distance <= 0.0
                    || distance > reach
                    || nearest.as_ref().is_some_and(|hit| hit.distance <= distance)
                {
                    continue;
                }
                nearest = Some(RayHit {
                    surface,
                    distance,
                    normal: self.triangle_normal(triangle),
                });
            }
        }
        nearest
    }

    /// The triangles of one BSP surface, at most `limit` (`world_notes`'s highlight).
    pub(crate) fn surface_triangles(&self, surface: usize, limit: usize) -> Vec<[Vec3; 3]> {
        self.triangles(surface)
            .take(limit)
            .map(|triangle| triangle.map(|index| Vec3::from_array(self.positions[index])))
            .collect()
    }

    /// The distinct triangle edges of one BSP surface, at most `limit` (`world_notes`'s
    /// highlight).
    pub(crate) fn surface_edges(&self, surface: usize, limit: usize) -> Vec<[Vec3; 2]> {
        let mut seen = std::collections::HashSet::new();
        let mut edges = Vec::new();
        for [a, b, c] in self.triangles(surface) {
            for (from, to) in [(a, b), (b, c), (c, a)] {
                if edges.len() == limit {
                    return edges;
                }
                if seen.insert((from.min(to), from.max(to))) {
                    edges.push([from, to].map(|index| Vec3::from_array(self.positions[index])));
                }
            }
        }
        edges
    }

    /// Outward triangle normal, oriented by the stored vertex normal so the
    /// facing test does not depend on the tessellator's winding.
    fn triangle_normal(&self, triangle: [usize; 3]) -> Vec3 {
        let [a, b, c] = triangle.map(|index| Vec3::from_array(self.positions[index]));
        let normal = (b - a).cross(c - a).normalize_or_zero();
        if normal.dot(Vec3::from_array(self.normals[triangle[0]])) < 0.0 {
            -normal
        } else {
            normal
        }
    }
}

/// Where a ray met the world ([`DecalSurfaces::ray_hit`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RayHit {
    /// BSP draw-surface index.
    pub(crate) surface: usize,
    pub(crate) distance: f32,
    /// The triangle's outward normal.
    pub(crate) normal: Vec3,
}

/// Fixed-capacity buffers reused by every projection.
pub(crate) struct DecalScratch {
    surfaces: Vec<usize>,
    max_surfaces: usize,
    fragments: MarkFragments,
    /// Tags' buffers: their surfaces are filtered further ([`tag_unfit`]).
    tags: bool,
}

impl Default for DecalScratch {
    fn default() -> Self {
        Self {
            surfaces: Vec::with_capacity(MAX_MARK_SURFACES),
            max_surfaces: MAX_MARK_SURFACES,
            fragments: MarkFragments::with_capacity(MAX_DECAL_POINTS, MAX_DECAL_FRAGMENTS),
            tags: false,
        }
    }
}

impl DecalScratch {
    /// The buffers tags project with: larger, and keeping off surfaces a tag does not
    /// sit well on ([`tag_unfit`]).
    pub(crate) fn for_tags() -> Self {
        Self {
            surfaces: Vec::with_capacity(MAX_TAG_SURFACES),
            max_surfaces: MAX_TAG_SURFACES,
            fragments: MarkFragments::with_capacity(MAX_TAG_POINTS, MAX_TAG_FRAGMENTS),
            tags: true,
        }
    }
}

/// Clip one request into the world and hand each fragment (at most
/// `MAX_VERTICES_ON_DECAL` points, `tr_decals.cpp:212-214`) to `emit`.
/// Returns the number of fragments emitted. Allocation-free.
pub(crate) fn project(
    world: &DecalSurfaces,
    bsp: &Bsp,
    request: &DecalRequest,
    scratch: &mut DecalScratch,
    mut emit: impl FnMut(&[DecalVertex]),
) -> usize {
    if request.radius <= 0.0 {
        return 0;
    }
    let axis = texture_axes(request.direction, request.orientation);
    let radius = request.radius;
    let corners = [
        request.origin - radius * axis[1] - radius * axis[2],
        request.origin + radius * axis[1] - radius * axis[2],
        request.origin + radius * axis[1] + radius * axis[2],
        request.origin - radius * axis[1] + radius * axis[2],
    ]
    .map(|corner| corner.to_array());
    let projection = MarkProjection::new(&corners, (axis[0] * -PROJECTION_DEPTH).to_array());
    clip(world, bsp, &projection, scratch);
    let scale = 0.5 / radius;
    let mut vertices = [DecalVertex::default(); MAX_VERTICES_ON_POLY];
    let mut emitted = 0;
    for fragment in scratch.fragments.fragments() {
        let points = scratch.fragments.fragment_points(*fragment);
        let count = points.len().min(MAX_VERTICES_ON_DECAL);
        if count < 3 {
            continue;
        }
        for (vertex, point) in vertices.iter_mut().zip(points) {
            let delta = Vec3::from_array(*point) - request.origin;
            *vertex = DecalVertex {
                position: *point,
                // Mark shaders sample with `clampmap`; the atlas wraps, so the
                // clip epsilon must not push a coordinate past the edge.
                st: [
                    (0.5 + delta.dot(axis[1]) * scale).clamp(0.0, 1.0),
                    (0.5 + delta.dot(axis[2]) * scale).clamp(0.0, 1.0),
                ],
            };
        }
        emit(&vertices[..count]);
        emitted += 1;
    }
    emitted
}

/// Project a narrow continuous cut, following CG_CreateSaberMarks' four corners.
/// Uses the same surface filters and reusable clipping buffers as impact decals.
pub(crate) fn project_strip(
    world: &DecalSurfaces,
    bsp: &Bsp,
    start: Vec3,
    end: Vec3,
    normal: Vec3,
    width: f32,
    scratch: &mut DecalScratch,
    mut emit: impl FnMut(&[DecalVertex]),
) {
    let along = (end - start).normalize_or(Vec3::X);
    let across = along.cross(normal).normalize_or(Vec3::Y);
    let corners = [
        start - width * along - width * across,
        end + width * along - width * across,
        end + width * along + width * across,
        start - width * along + width * across,
    ]
    .map(|p| p.to_array());
    let projection = MarkProjection::new(&corners, (-normal).to_array());
    clip(world, bsp, &projection, scratch);
    let center = (start + end) * 0.5;
    let mut vertices = [DecalVertex::default(); MAX_VERTICES_ON_DECAL];
    for fragment in scratch.fragments.fragments() {
        let points = scratch.fragments.fragment_points(*fragment);
        let count = points.len().min(MAX_VERTICES_ON_DECAL);
        if count < 3 {
            continue;
        }
        for (vertex, point) in vertices.iter_mut().zip(points) {
            let delta = Vec3::from_array(*point) - center;
            *vertex = DecalVertex {
                position: *point,
                st: [
                    (0.5 + delta.dot(along) * 0.065).clamp(0., 1.),
                    (0.5 + delta.dot(across) * 0.175).clamp(0., 1.),
                ],
            };
        }
        emit(&vertices[..count]);
    }
}

fn clip(world: &DecalSurfaces, bsp: &Bsp, projection: &MarkProjection, scratch: &mut DecalScratch) {
    let direction = Vec3::from_array(projection.direction());

    scratch.surfaces.clear();
    scratch.fragments.clear();
    let shaders = bsp.shaders();
    let tags = scratch.tags;
    bsp.mark_surfaces(
        projection,
        scratch.max_surfaces,
        &mut scratch.surfaces,
        |index, surface| {
            let shader = &shaders[surface.shader];
            shader.surface_flags & (SURF_NOIMPACT | SURF_NOMARKS) == 0
                && shader.content_flags & CONTENTS_FOG == 0
                && !(tags && world.no_tags.get(index).copied().unwrap_or(false))
        },
    );
    let surfaces = bsp.render().surfaces();
    'surfaces: for &surface_index in &scratch.surfaces {
        // Tags also go on models the compiler merged into the world (a panel flush with
        // a brush wall left the tag cut at the seam); marks keep the reference's
        // `r_marksOnTriangleMeshes 0`.
        let curved = match surfaces[surface_index].kind {
            SurfaceKind::Planar => false,
            SurfaceKind::Patch => true,
            SurfaceKind::TriangleSoup if tags => true,
            _ => continue,
        };
        for triangle in world.triangles(surface_index) {
            if curved && world.triangle_normal(triangle).dot(direction) >= GRID_FACING_LIMIT {
                continue;
            }
            let polygon = triangle.map(|index| world.positions[index]);
            scratch.fragments.push_polygon(&polygon, projection);
            if scratch.fragments.is_full() {
                break 'surfaces;
            }
        }
    }
}

/// `tr_decals.cpp:180-184`: axis 0 along the normal, axis 1 any
/// perpendicular rotated by `orientation` about the normal, axis 2 closing
/// the frame.
pub(crate) fn texture_axes(direction: Vec3, orientation: f32) -> [Vec3; 3] {
    let forward = direction.normalize_or(Vec3::Z);
    let seed = perpendicular_vector(forward);
    let rotated = Quat::from_axis_angle(forward, orientation.to_radians()) * seed;
    [forward, forward.cross(rotated), rotated]
}

/// `PerpendicularVector` (`q_math.c`): project the unit axis of the smallest
/// component onto the plane and normalise.
fn perpendicular_vector(source: Vec3) -> Vec3 {
    let components = source.to_array();
    let mut smallest = 0;
    for axis in 1..3 {
        if components[axis].abs() < components[smallest].abs() {
            smallest = axis;
        }
    }
    let mut seed = Vec3::ZERO;
    seed[smallest] = 1.0;
    (seed - source * seed.dot(source)).normalize_or(Vec3::Y)
}

#[cfg(test)]
mod tag_surface_tests {
    use super::tag_unfit;

    fn unfit(script: &str) -> bool {
        let shaders = sjk_shader::parse_shader_script(script.as_bytes(), "shaders/test.shader")
            .expect("parse");
        tag_unfit(&shaders[0])
    }

    #[test]
    fn tags_keep_off_moving_see_through_cut_out_and_laid_over_surfaces() {
        let wall = "textures/a/wall\n{\n{\nmap $lightmap\n}\n{\nmap textures/a/wall\nblendFunc GL_DST_COLOR GL_ZERO\n}\n}\n";
        assert!(!unfit(wall), "a plain lightmapped wall takes tags");
        let shiny = "textures/a/shiny\n{\n{\nmap textures/a/shiny\n}\n{\nmap textures/env\ntcGen environment\nblendFunc GL_ONE GL_ONE\n}\n}\n";
        assert!(!unfit(shiny), "an opaque wall with a shine on top too");
        let glass = "textures/a/glass\n{\nsurfaceparm trans\n{\nmap textures/a/glass\nblendFunc GL_ONE GL_ONE\n}\n}\n";
        assert!(unfit(glass), "glass");
        let grate =
            "textures/a/grate\n{\n{\nmap textures/a/grate\nalphaFunc GE128\ndepthWrite\n}\n}\n";
        assert!(unfit(grate), "a grate");
        let flag = "textures/a/flag\n{\ndeformVertexes wave 30 sin 0 3 0 0.2\n{\nmap textures/a/flag\n}\n}\n";
        assert!(unfit(flag), "a waving flag");
        let poster = "textures/a/poster\n{\npolygonOffset\n{\nmap textures/a/poster\n}\n}\n";
        assert!(unfit(poster), "a decal laid on a wall");
    }
}
