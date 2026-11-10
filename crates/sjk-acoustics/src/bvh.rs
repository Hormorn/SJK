//! A bounding volume hierarchy over a mesh's triangles, built once per level on the
//! acoustics' thread: binned surface-area splits, up to [`LEAF`] triangles a leaf,
//! rays walked with a fixed stack (no allocation per ray).

use glam::Vec3;

/// Most triangles in a leaf.
const LEAF: usize = 4;
/// Bins the splits are chosen from.
const BINS: usize = 12;
/// The deepest the walk goes (a tree of a few million triangles stays below it).
const STACK: usize = 64;

/// One triangle, ready for intersection.
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub v0: Vec3,
    pub e1: Vec3,
    pub e2: Vec3,
    /// Out of the solid it bounds (its brush side's plane), unit length.
    pub normal: Vec3,
    /// An index into [`crate::material::MATERIALS`].
    pub material: u8,
    /// The brush it bounds, [`NO_BRUSH`] for a curve.
    pub brush: u32,
}

/// [`Triangle::brush`] of a curve's triangle.
pub const NO_BRUSH: u32 = u32::MAX;

impl Triangle {
    pub fn new(points: [Vec3; 3], normal: Vec3, material: u8, brush: u32) -> Self {
        Self {
            v0: points[0],
            e1: points[1] - points[0],
            e2: points[2] - points[0],
            normal,
            material,
            brush,
        }
    }

    fn bounds(&self) -> (Vec3, Vec3) {
        let (a, b, c) = (self.v0, self.v0 + self.e1, self.v0 + self.e2);
        (a.min(b).min(c), a.max(b).max(c))
    }

    fn centroid(&self) -> Vec3 {
        self.v0 + (self.e1 + self.e2) / 3.0
    }

    /// Where along `direction` (any length, its multiple) the ray from `origin`
    /// meets the triangle, from either side.
    #[inline]
    fn intersect(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let p = direction.cross(self.e2);
        let determinant = self.e1.dot(p);
        if determinant.abs() < 1.0e-9 {
            return None;
        }
        let inverse = 1.0 / determinant;
        let s = origin - self.v0;
        let u = s.dot(p) * inverse;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(self.e1);
        let v = direction.dot(q) * inverse;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        Some(self.e2.dot(q) * inverse)
    }
}

#[derive(Clone, Copy, Debug)]
struct Node {
    minimum: Vec3,
    maximum: Vec3,
    /// A leaf's first triangle, or an inner node's first child (the second follows
    /// it).
    first: u32,
    /// Triangles of a leaf, 0 for an inner node.
    count: u32,
}

/// What a ray met.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    /// Distance along the ray (the direction is unit length).
    pub distance: f32,
    pub triangle: u32,
}

/// Triangles and their tree.
#[derive(Clone, Debug, Default)]
pub struct Bvh {
    triangles: Vec<Triangle>,
    nodes: Vec<Node>,
}

impl Bvh {
    /// Build the tree; the triangles are kept, in the tree's order.
    pub fn build(mut triangles: Vec<Triangle>) -> Self {
        if triangles.is_empty() {
            return Self::default();
        }
        let centroids: Vec<Vec3> = triangles.iter().map(Triangle::centroid).collect();
        let bounds: Vec<(Vec3, Vec3)> = triangles.iter().map(Triangle::bounds).collect();
        let mut order: Vec<u32> = (0..triangles.len() as u32).collect();
        let mut nodes = vec![Node {
            minimum: Vec3::ZERO,
            maximum: Vec3::ZERO,
            first: 0,
            count: triangles.len() as u32,
        }];
        let mut pending = vec![0_usize];
        while let Some(index) = pending.pop() {
            let (first, count) = (nodes[index].first as usize, nodes[index].count as usize);
            let slice = &mut order[first..first + count];
            let (minimum, maximum) = slice.iter().fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(low, high), &t| {
                    (
                        low.min(bounds[t as usize].0),
                        high.max(bounds[t as usize].1),
                    )
                },
            );
            nodes[index].minimum = minimum;
            nodes[index].maximum = maximum;
            if count <= LEAF {
                continue;
            }
            let Some(split) = best_split(slice, &centroids, &bounds) else {
                continue;
            };
            let (axis, at) = split;
            let mut left = 0;
            for i in 0..slice.len() {
                if centroids[slice[i] as usize][axis] < at {
                    slice.swap(i, left);
                    left += 1;
                }
            }
            if left == 0 || left == count {
                continue;
            }
            let child = nodes.len();
            nodes.push(Node {
                minimum: Vec3::ZERO,
                maximum: Vec3::ZERO,
                first: first as u32,
                count: left as u32,
            });
            nodes.push(Node {
                minimum: Vec3::ZERO,
                maximum: Vec3::ZERO,
                first: (first + left) as u32,
                count: (count - left) as u32,
            });
            nodes[index].first = child as u32;
            nodes[index].count = 0;
            pending.push(child);
            pending.push(child + 1);
        }
        let ordered = order.iter().map(|&t| triangles[t as usize]).collect();
        triangles.clear();
        Self {
            triangles: ordered,
            nodes,
        }
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// The box round every triangle.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.nodes.first().map(|node| (node.minimum, node.maximum))
    }

    /// The nearest triangle the ray from `origin` along unit `direction` meets
    /// between `near` and `far`.
    pub fn closest(&self, origin: Vec3, direction: Vec3, near: f32, far: f32) -> Option<RayHit> {
        let mut best: Option<RayHit> = None;
        let mut far = far;
        self.walk(origin, direction, near, far, |triangle, index| {
            if let Some(distance) = triangle.intersect(origin, direction)
                && distance > near
                && distance < far
            {
                far = distance;
                best = Some(RayHit {
                    distance,
                    triangle: index,
                });
            }
            far
        });
        best
    }

    /// Whether anything is between `near` and `far` along the ray.
    pub fn blocked(&self, origin: Vec3, direction: Vec3, near: f32, far: f32) -> bool {
        let mut hit = false;
        self.walk(origin, direction, near, far, |triangle, _| {
            if !hit
                && let Some(distance) = triangle.intersect(origin, direction)
                && distance > near
                && distance < far
            {
                hit = true;
            }
            if hit { f32::MIN } else { far }
        });
        hit
    }

    /// Visit the triangles of the leaves the ray's box test lets through, nearest
    /// first; `visit` returns how far the walk still needs to look (`f32::MIN`
    /// stops it).
    fn walk(
        &self,
        origin: Vec3,
        direction: Vec3,
        near: f32,
        far: f32,
        mut visit: impl FnMut(&Triangle, u32) -> f32,
    ) {
        if self.nodes.is_empty() {
            return;
        }
        let inverse = safe_inverse(direction);
        let mut stack = [0_u32; STACK];
        if slab(origin, inverse, self.nodes[0], near, far).is_none() {
            return;
        }
        let mut depth = 1;
        let mut far = far;
        while depth > 0 {
            depth -= 1;
            let node = self.nodes[stack[depth] as usize];
            if node.count > 0 {
                for index in node.first..node.first + node.count {
                    let reach = visit(&self.triangles[index as usize], index);
                    if reach == f32::MIN {
                        return;
                    }
                    far = reach;
                }
                continue;
            }
            let (a, b) = (node.first as usize, node.first as usize + 1);
            let hit_a = slab(origin, inverse, self.nodes[a], near, far);
            let hit_b = slab(origin, inverse, self.nodes[b], near, far);
            // Push the farther first so the nearer is walked first.
            match (hit_a, hit_b) {
                (Some(ta), Some(tb)) => {
                    let (first, second) = if ta <= tb { (a, b) } else { (b, a) };
                    if depth + 2 <= STACK {
                        stack[depth] = second as u32;
                        stack[depth + 1] = first as u32;
                        depth += 2;
                    }
                }
                (Some(_), None) if depth < STACK => {
                    stack[depth] = a as u32;
                    depth += 1;
                }
                (None, Some(_)) if depth < STACK => {
                    stack[depth] = b as u32;
                    depth += 1;
                }
                _ => {}
            }
        }
    }
}

/// `1 / direction` per axis, an axis of 0 taken as a tiny positive one (so a ray in
/// a box's face plane gives no 0 * infinity).
pub(crate) fn safe_inverse(direction: Vec3) -> Vec3 {
    let nudge = |value: f32| {
        if value.abs() < 1.0e-12 {
            1.0e-12
        } else {
            value
        }
    };
    Vec3::new(nudge(direction.x), nudge(direction.y), nudge(direction.z)).recip()
}

/// Where the ray enters the node's box, if it does before `far`.
#[inline]
fn slab(origin: Vec3, inverse: Vec3, node: Node, near: f32, far: f32) -> Option<f32> {
    let t1 = (node.minimum - origin) * inverse;
    let t2 = (node.maximum - origin) * inverse;
    let enter = t1.min(t2).max_element().max(near);
    let leave = t1.max(t2).min_element().min(far);
    (enter <= leave).then_some(enter)
}

/// The axis and place of the cheapest split of `slice` by surface area, if any
/// beats keeping it whole.
fn best_split(slice: &[u32], centroids: &[Vec3], bounds: &[(Vec3, Vec3)]) -> Option<(usize, f32)> {
    let (low, high) = slice.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(low, high), &t| {
            (
                low.min(centroids[t as usize]),
                high.max(centroids[t as usize]),
            )
        },
    );
    let area = |low: Vec3, high: Vec3| {
        let size = (high - low).max(Vec3::ZERO);
        size.x * size.y + size.y * size.z + size.z * size.x
    };
    let mut best: Option<(f32, usize, f32)> = None;
    for axis in 0..3 {
        let extent = high[axis] - low[axis];
        if extent <= 1.0e-4 {
            continue;
        }
        let mut counts = [0_u32; BINS];
        let mut boxes = [(Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)); BINS];
        for &t in slice {
            let bin = (((centroids[t as usize][axis] - low[axis]) / extent * BINS as f32) as usize)
                .min(BINS - 1);
            counts[bin] += 1;
            boxes[bin].0 = boxes[bin].0.min(bounds[t as usize].0);
            boxes[bin].1 = boxes[bin].1.max(bounds[t as usize].1);
        }
        for split in 1..BINS {
            let side = |range: std::ops::Range<usize>| {
                range.fold(
                    (0_u32, Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                    |acc, i| {
                        (
                            acc.0 + counts[i],
                            acc.1.min(boxes[i].0),
                            acc.2.max(boxes[i].1),
                        )
                    },
                )
            };
            let (left, left_low, left_high) = side(0..split);
            let (right, right_low, right_high) = side(split..BINS);
            if left == 0 || right == 0 {
                continue;
            }
            let cost = left as f32 * area(left_low, left_high)
                + right as f32 * area(right_low, right_high);
            if best.is_none_or(|(most, ..)| cost < most) {
                best = Some((cost, axis, low[axis] + extent * split as f32 / BINS as f32));
            }
        }
    }
    best.map(|(_, axis, at)| (axis, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::tests::cube;

    #[test]
    fn a_ray_meets_the_nearest_of_many_boxes() {
        let mut triangles = Vec::new();
        for i in 0..50 {
            let x = i as f32 * 100.0;
            triangles.extend(cube(
                Vec3::new(x, -10.0, -10.0),
                Vec3::new(x + 20.0, 10.0, 10.0),
                i,
            ));
        }
        let bvh = Bvh::build(triangles);
        let hit = bvh
            .closest(Vec3::new(-50.0, 0.0, 0.0), Vec3::X, 0.0, 10_000.0)
            .unwrap();
        assert!((hit.distance - 50.0).abs() < 1.0e-3);
        let hit = bvh
            .closest(Vec3::new(1_050.0, 0.0, 0.0), Vec3::X, 0.0, 10_000.0)
            .unwrap();
        assert!((hit.distance - 50.0).abs() < 1.0e-3);
        assert_eq!(bvh.triangles()[hit.triangle as usize].brush, 11);
        assert!(
            bvh.closest(Vec3::new(-50.0, 50.0, 0.0), Vec3::X, 0.0, 10_000.0)
                .is_none()
        );
        assert!(bvh.blocked(Vec3::new(-50.0, 0.0, 0.0), Vec3::X, 0.0, 60.0));
        assert!(!bvh.blocked(Vec3::new(-50.0, 0.0, 0.0), Vec3::X, 0.0, 40.0));
    }

    #[test]
    fn rays_agree_with_trying_every_triangle() {
        let mut triangles = Vec::new();
        for i in 0..30 {
            let p = Vec3::new(
                (i * 37 % 11) as f32 * 40.0,
                (i * 17 % 7) as f32 * 50.0,
                (i % 5) as f32 * 30.0,
            );
            triangles.extend(cube(p, p + Vec3::new(25.0, 15.0, 35.0), i));
        }
        let bvh = Bvh::build(triangles.clone());
        for k in 0..200 {
            let origin = Vec3::new(-100.0, (k % 13) as f32 * 25.0, (k % 7) as f32 * 20.0);
            let direction = Vec3::new(
                1.0,
                ((k * 7) % 11) as f32 * 0.05 - 0.2,
                ((k * 3) % 5) as f32 * 0.04 - 0.1,
            )
            .normalize();
            let brute = triangles
                .iter()
                .filter_map(|t| t.intersect(origin, direction))
                .filter(|d| *d > 0.0)
                .fold(f32::MAX, f32::min);
            let hit = bvh
                .closest(origin, direction, 0.0, f32::MAX)
                .map_or(f32::MAX, |h| h.distance);
            assert!((brute - hit).abs() < 1.0e-2, "{k}: {brute} {hit}");
        }
    }
}
