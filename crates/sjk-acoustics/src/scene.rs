//! The level as the sound meets it: every side of a solid brush and every solid
//! curve as triangles with their material ([`LevelGeometry`], built once per level
//! off the game's threads), and the doors, lifts and other brush models placed
//! where the game shows them ([`Scene`]).

use crate::bvh::{Bvh, NO_BRUSH, RayHit, Triangle};
use crate::material::{SURF_NODRAW, SURF_SKY, ShaderMaterials};
use glam::{Mat3, Vec3};
use sjk_bsp::Bsp;

/// `CONTENTS_SOLID`: walls, not water, fog, clips or players.
const CONTENTS_SOLID: u32 = 1;

/// The level's walls: the world's and each brush model's.
#[derive(Debug, Default)]
pub struct LevelGeometry {
    world: Bvh,
    /// By inline model number (`*1` is 1); `None` when it has no solid.
    models: Vec<Option<Bvh>>,
    /// Triangles whose material came from the shader table or the level (not the
    /// generic wall).
    pub named_triangles: usize,
}

impl LevelGeometry {
    /// Gather a level's solid brush sides and curves.
    pub fn from_bsp(bsp: &Bsp, materials: &ShaderMaterials) -> Self {
        let mut named_triangles = 0;
        let mut models = Vec::new();
        let mut world = Bvh::default();
        for (index, model) in bsp.render().models().iter().enumerate() {
            let mut triangles = Vec::new();
            named_triangles += model_triangles(bsp, model, materials, &mut triangles);
            if index == 0 {
                world = Bvh::build(triangles);
                models.push(None);
            } else {
                models.push((!triangles.is_empty()).then(|| Bvh::build(triangles)));
            }
        }
        Self {
            world,
            models,
            named_triangles,
        }
    }

    /// From triangles alone (tests, tools): the world and no brush models.
    pub fn from_triangles(triangles: Vec<Triangle>) -> Self {
        Self {
            world: Bvh::build(triangles),
            models: Vec::new(),
            named_triangles: 0,
        }
    }

    /// Triangles of the world and of every brush model.
    pub fn triangle_count(&self) -> usize {
        self.world.triangles().len()
            + self
                .models
                .iter()
                .flatten()
                .map(|model| model.triangles().len())
                .sum::<usize>()
    }

    /// The world's box.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.world.bounds()
    }

    /// Whether brush model `model` has walls.
    pub fn has_model(&self, model: usize) -> bool {
        self.models.get(model).is_some_and(Option::is_some)
    }
}

/// Collect `model`'s solid triangles; how many have a named material.
fn model_triangles(
    bsp: &Bsp,
    model: &sjk_bsp::Model,
    materials: &ShaderMaterials,
    out: &mut Vec<Triangle>,
) -> usize {
    let shaders = bsp.shaders();
    let mut named = 0;
    for brush_index in model.brushes.clone() {
        let Some(brush) = bsp.brushes().get(brush_index) else {
            continue;
        };
        if brush.content_flags & CONTENTS_SOLID == 0 {
            continue;
        }
        let sides = &bsp.brush_sides()[brush.sides.clone()];
        let material_of = |side: &sjk_bsp::BrushSide| {
            let shader = &shaders[side.shader];
            materials.material(&shader.name_lossy(), shader.surface_flags)
        };
        // A side that is not drawn (caulk, the back of a wall) is what the drawn
        // sides of its brush are made of: a wall is one thing to go through.
        let brush_material = sides
            .iter()
            .filter(|side| shaders[side.shader].surface_flags & (SURF_NODRAW | SURF_SKY) == 0)
            .find_map(material_of)
            .unwrap_or(0);
        for (offset, side) in sides.iter().enumerate() {
            let points = bsp.brush_side_polygon(brush_index, brush.sides.start + offset);
            if points.len() < 3 {
                continue;
            }
            let shader = &shaders[side.shader];
            let drawn = shader.surface_flags & SURF_NODRAW == 0;
            let own = material_of(side);
            let material = match own {
                Some(material) if drawn || shader.surface_flags & SURF_SKY != 0 => material,
                _ => brush_material,
            };
            let normal = Vec3::from_array(bsp.planes()[side.plane].normal);
            let points: Vec<Vec3> = points.into_iter().map(Vec3::from_array).collect();
            for i in 2..points.len() {
                out.push(Triangle::new(
                    [points[0], points[i - 1], points[i]],
                    normal,
                    material,
                    brush_index as u32,
                ));
                if material != 0 {
                    named += 1;
                }
            }
        }
    }
    for surface_index in model.surfaces.clone() {
        let Some(surface) = bsp.render().surfaces().get(surface_index) else {
            continue;
        };
        let shader = &shaders[surface.shader];
        if shader.content_flags & CONTENTS_SOLID == 0 {
            continue;
        }
        let Some(grid) = bsp.patch_collision_grid(surface_index) else {
            continue;
        };
        let material = materials
            .material(&shader.name_lossy(), shader.surface_flags)
            .unwrap_or(0);
        for x in 1..grid.width {
            for y in 1..grid.height {
                let corners = [
                    grid.point(x - 1, y - 1),
                    grid.point(x, y - 1),
                    grid.point(x, y),
                    grid.point(x - 1, y),
                ]
                .map(Vec3::from_array);
                for triangle in [
                    [corners[0], corners[1], corners[2]],
                    [corners[0], corners[2], corners[3]],
                ] {
                    let normal = (triangle[1] - triangle[0])
                        .cross(triangle[2] - triangle[0])
                        .normalize_or_zero();
                    if normal == Vec3::ZERO {
                        continue;
                    }
                    out.push(Triangle::new(triangle, normal, material, NO_BRUSH));
                    if material != 0 {
                        named += 1;
                    }
                }
            }
        }
    }
    named
}

/// A brush model where the game shows it: as collision places it, the model's
/// points `p` at `origin + axesᵀ · p`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mover {
    pub model: usize,
    pub origin: Vec3,
    /// Rows of the turn (collision's `axes`), `None` unturned.
    pub axes: Option<[Vec3; 3]>,
}

/// A mover ready for rays: its turn both ways and its box in the world.
#[derive(Clone, Copy, Debug)]
pub struct PlacedMover {
    model: usize,
    origin: Vec3,
    /// World to model.
    to_model: Mat3,
    /// Model to world.
    to_world: Mat3,
    low: Vec3,
    high: Vec3,
}

impl PlacedMover {
    /// Place `mover`; `None` when the level has no walls for its model.
    pub fn new(geometry: &LevelGeometry, mover: Mover) -> Option<Self> {
        let model = geometry.models.get(mover.model)?.as_ref()?;
        let (low, high) = model.bounds()?;
        let to_model = mover.axes.map_or(Mat3::IDENTITY, |rows| {
            Mat3::from_cols(rows[0], rows[1], rows[2]).transpose()
        });
        let to_world = to_model.transpose();
        let (mut world_low, mut world_high) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for corner in 0..8 {
            let local = Vec3::new(
                if corner & 1 == 0 { low.x } else { high.x },
                if corner & 2 == 0 { low.y } else { high.y },
                if corner & 4 == 0 { low.z } else { high.z },
            );
            let world = mover.origin + to_world * local;
            world_low = world_low.min(world);
            world_high = world_high.max(world);
        }
        Some(Self {
            model: mover.model,
            origin: mover.origin,
            to_model,
            to_world,
            low: world_low,
            high: world_high,
        })
    }
}

impl PlacedMover {
    /// Its box in the world.
    pub fn world_box(&self) -> (Vec3, Vec3) {
        (self.low, self.high)
    }
}

/// What a ray met in the scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneHit {
    pub distance: f32,
    /// Out of the solid, in the world.
    pub normal: Vec3,
    pub material: u8,
    /// The brush (with its model), for counting the walls a sound goes through;
    /// `None` for a curve.
    pub wall: Option<(usize, u32)>,
}

/// The level with its movers where they are now.
#[derive(Clone, Copy)]
pub struct Scene<'a> {
    pub geometry: &'a LevelGeometry,
    pub movers: &'a [PlacedMover],
}

impl Scene<'_> {
    /// The nearest wall along unit `direction` from `origin` between `near` and
    /// `far`.
    pub fn closest(&self, origin: Vec3, direction: Vec3, near: f32, far: f32) -> Option<SceneHit> {
        let mut best = self
            .geometry
            .world
            .closest(origin, direction, near, far)
            .map(|hit| describe(&self.geometry.world, hit, 0, Mat3::IDENTITY));
        for mover in self.movers {
            let reach = best.map_or(far, |hit| hit.distance);
            if !ray_meets_box(origin, direction, near, reach, mover.low, mover.high) {
                continue;
            }
            let Some(model) = self.geometry.models[mover.model].as_ref() else {
                continue;
            };
            let local_origin = mover.to_model * (origin - mover.origin);
            let local_direction = mover.to_model * direction;
            if let Some(hit) = model.closest(local_origin, local_direction, near, reach) {
                best = Some(describe(model, hit, mover.model, mover.to_world));
            }
        }
        best
    }

    /// Whether anything is between `near` and `far` along the ray.
    pub fn blocked(&self, origin: Vec3, direction: Vec3, near: f32, far: f32) -> bool {
        if self.geometry.world.blocked(origin, direction, near, far) {
            return true;
        }
        self.movers.iter().any(|mover| {
            ray_meets_box(origin, direction, near, far, mover.low, mover.high)
                && self.geometry.models[mover.model]
                    .as_ref()
                    .is_some_and(|model| {
                        model.blocked(
                            mover.to_model * (origin - mover.origin),
                            mover.to_model * direction,
                            near,
                            far,
                        )
                    })
        })
    }

    /// Whether the straight way from `from` to `to` is free, but for its last
    /// `spare` units (the wall a sound is on).
    pub fn sees(&self, from: Vec3, to: Vec3, spare: f32) -> bool {
        let delta = to - from;
        let length = delta.length();
        if length <= spare + 1.0 {
            return true;
        }
        !self.blocked(from, delta / length, 0.5, length - spare)
    }
}

fn describe(bvh: &Bvh, hit: RayHit, model: usize, to_world: Mat3) -> SceneHit {
    let triangle = &bvh.triangles()[hit.triangle as usize];
    SceneHit {
        distance: hit.distance,
        normal: to_world * triangle.normal,
        material: triangle.material,
        wall: (triangle.brush != NO_BRUSH).then_some((model, triangle.brush)),
    }
}

fn ray_meets_box(
    origin: Vec3,
    direction: Vec3,
    near: f32,
    far: f32,
    low: Vec3,
    high: Vec3,
) -> bool {
    let inverse = crate::bvh::safe_inverse(direction);
    let t1 = (low - origin) * inverse;
    let t2 = (high - origin) * inverse;
    t1.min(t2).max_element().max(near) <= t1.max(t2).min_element().min(far)
}
