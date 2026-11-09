//! Load-time polygons of the convex brush boundaries retained in a BSP.
use super::{
    Bsp, Plane,
    patch_geometry::{base_winding, chop, flipped},
};

impl Bsp {
    /// Reconstruct a brush side by clipping its plane against the remaining sides.
    /// `side` is an absolute index in [`Self::brush_sides`]. Bevel/redundant planes
    /// may produce an empty polygon. This does not change collision geometry.
    pub fn brush_side_polygon(&self, brush: usize, side: usize) -> Vec<[f32; 3]> {
        let Some(brush) = self.brushes.get(brush) else {
            return Vec::new();
        };
        if !brush.sides.contains(&side) {
            return Vec::new();
        }
        polygon(
            self.planes[self.brush_sides[side].plane],
            brush
                .sides
                .clone()
                .filter(|&i| i != side)
                .map(|i| self.planes[self.brush_sides[i].plane]),
        )
    }
}

fn polygon(face: Plane, boundaries: impl Iterator<Item = Plane>) -> Vec<[f32; 3]> {
    let mut points = base_winding(face);
    for plane in boundaries {
        // Redundant coplanar sides describe the same boundary, not an empty
        // brush. The collision winding helper otherwise discards all-on points.
        if (0..3).all(|i| (face.normal[i] - plane.normal[i]).abs() < 1e-6)
            && (face.distance - plane.distance).abs() < 1e-4
        {
            continue;
        }
        chop(&mut points, flipped(plane));
        if points.len() < 3 {
            return Vec::new();
        }
    }
    points
}

/// A patch's collision surface as a grid of points (the subdivision collision
/// uses, not the renderer's tessellation): `point(x, y)` for `x < width`,
/// `y < height`, neighbours forming quads.
#[derive(Clone, Debug, PartialEq)]
pub struct PatchCollisionGrid {
    pub width: usize,
    pub height: usize,
    points: Vec<[f32; 3]>,
}

impl PatchCollisionGrid {
    pub fn point(&self, x: usize, y: usize) -> [f32; 3] {
        self.points[x * self.height + y]
    }
}

impl Bsp {
    /// The collision grid of render surface `surface` when it is a patch whose
    /// control points subdivide (for other systems that need the solid shape as
    /// triangles, such as the 3D sound). Collision itself does not use it.
    pub fn patch_collision_grid(&self, surface: usize) -> Option<PatchCollisionGrid> {
        let surface = self.render.surfaces().get(surface)?;
        let [width, height] = surface.patch_dimensions?;
        let points: Vec<_> = self.render.vertices()[surface.vertices.clone()]
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        let grid = super::patch_grid::Grid::generate(width, height, &points).ok()?;
        let (width, height) = (grid.width(), grid.height());
        let points = (0..width)
            .flat_map(|x| (0..height).map(move |y| (x, y)))
            .map(|(x, y)| grid.point(x, y))
            .collect();
        Some(PatchCollisionGrid {
            width,
            height,
            points,
        })
    }
}
