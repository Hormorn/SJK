//! Linked caulk: from one caulk or nodraw face ([`super::faces`]), every face of the
//! world's brushes joined to it that a player can see, for the browser map editor to
//! select in one go (Sol, 03/11/2026: "detect all linked caulks, that's what I would
//! replace the most").
//!
//! Two faces are joined when they share a stretch of edge (within [`EDGE_EPSILON`]):
//! on the same plane and facing the same way ([`Grow::Plane`]), or at any angle but
//! back to back ([`Grow::Surface`]). A face is offered only when a player can see some
//! of it, as q3map2 keeps a side: its polygon, with the parts inside or flush against
//! the other solid brushes that touch it cut away, keeps some area, and that area lies
//! in an open leaf of the map (not the solid void around it).
//!
//! Sky sides link too (04/11/2026), with sky sides only: a sky face selects the sky round
//! it, a caulk face the caulk.

use glam::Vec3;
use sjk_bsp::Bsp;
use std::collections::{HashMap, HashSet, VecDeque};

/// How far two edges may lie apart and still be one (units).
const EDGE_EPSILON: f32 = 0.5;
/// How far apart two planes may lie and still be one (units).
const PLANE_EPSILON: f32 = 0.5;
/// How much two normals of one plane may differ (1 - their dot product).
const NORMAL_EPSILON: f32 = 1e-3;
/// The shortest shared edge that joins two faces (units).
const MIN_SHARED: f32 = 1.0;
/// Faces back to back (a thin wall's two sides over its top) are not one surface.
const BACK_TO_BACK: f32 = -0.95;
/// A brush face flush against another solid (within this) counts as covered by it.
const COVER_EPSILON: f32 = 0.25;
/// The area a face must keep in the open to be seen (square units).
const MIN_VISIBLE_AREA: f32 = 1.0;
/// How far in front of a face its open side is looked for.
const OPEN_OFFSET: f32 = 1.0;
/// A side smaller than this (square units) is a bevel or a sliver (as [`super::faces`]).
const MIN_AREA: f32 = 0.5;
/// How many faces a link takes at most.
pub(crate) const LINK_LIMIT: usize = 512;

/// `CONTENTS_SOLID` and `CONTENTS_TRANSLUCENT` (`codemp/game/surfaceflags.h`).
const CONTENTS_SOLID: u32 = 0x0000_0001;
const CONTENTS_TRANSLUCENT: u32 = super::faces::CONTENTS_TRANSLUCENT;

/// How a link grows from its first face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Grow {
    /// Faces on the first one's plane, facing the same way.
    Plane,
    /// Faces joined at any angle: the whole surface they make.
    Surface,
}

/// One side of a world brush.
pub(crate) struct Side {
    pub(crate) brush: u32,
    pub(crate) side: u32,
    pub(crate) shader: String,
    /// A side edits may show (caulk, nodraw, sky: [`super::faces`]'s rule).
    pub(crate) showable: bool,
    /// A sky side: it links with sky sides only, the others with the others.
    pub(crate) sky: bool,
    pub(crate) polygon: Vec<Vec3>,
    pub(crate) normal: Vec3,
    pub(crate) distance: f32,
    low: Vec3,
    high: Vec3,
}

/// A world brush as the cover test sees it.
struct Solid {
    brush: u32,
    /// Its planes, normals out.
    planes: Vec<(Vec3, f32)>,
    corners: Vec<Vec3>,
    low: Vec3,
    high: Vec3,
}

/// A brush to build [`Sides`] from: its number, whether it hides what is inside or
/// against it (solid and not see-through), and its sides.
pub(crate) struct BrushIn {
    pub(crate) brush: u32,
    pub(crate) occludes: bool,
    pub(crate) sides: Vec<SideIn>,
}

/// A brush side to build [`Sides`] from.
pub(crate) struct SideIn {
    pub(crate) side: u32,
    pub(crate) normal: Vec3,
    pub(crate) distance: f32,
    pub(crate) polygon: Vec<Vec3>,
    pub(crate) shader: String,
    pub(crate) showable: bool,
    pub(crate) sky: bool,
}

/// The world's brush sides, found by place.
pub(crate) struct Sides {
    sides: Vec<Side>,
    solids: Vec<Solid>,
    side_grid: Grid,
    solid_grid: Grid,
    by_number: HashMap<(u32, u32), usize>,
}

/// What a link found.
#[derive(Debug, Default)]
pub(crate) struct Linked {
    /// The faces, the first one first: indices into [`Sides::side`].
    pub(crate) faces: Vec<usize>,
    /// Joined faces left out because no player sees them.
    pub(crate) buried: usize,
    /// The link stopped at its limit.
    pub(crate) capped: bool,
}

impl Sides {
    /// The world model's brush sides of `bsp`.
    pub(crate) fn from_bsp(bsp: &Bsp) -> Self {
        let Some(world) = bsp.render().models().first() else {
            return Self::new(Vec::new());
        };
        let brushes = world
            .brushes
            .clone()
            .filter_map(|index| {
                let brush = bsp.brushes().get(index)?;
                let sides = brush
                    .sides
                    .clone()
                    .filter_map(|side_index| {
                        let side = bsp.brush_sides().get(side_index)?;
                        let plane = bsp.planes().get(side.plane)?;
                        let shader = bsp.shaders().get(side.shader)?;
                        let sky =
                            super::faces::is_sky_side(brush.content_flags, shader.surface_flags);
                        let offered = super::faces::offered(
                            brush.content_flags,
                            shader.content_flags,
                            shader.surface_flags,
                            shader.name(),
                        );
                        let showable = offered.is_some();
                        Some(SideIn {
                            side: side_index as u32,
                            normal: Vec3::from_array(plane.normal),
                            distance: plane.distance,
                            polygon: bsp
                                .brush_side_polygon(index, side_index)
                                .into_iter()
                                .map(Vec3::from_array)
                                .collect(),
                            shader: shader.name_lossy().into_owned(),
                            showable,
                            sky,
                        })
                    })
                    .collect();
                Some(BrushIn {
                    brush: index as u32,
                    occludes: brush.content_flags & CONTENTS_SOLID != 0
                        && brush.content_flags & CONTENTS_TRANSLUCENT == 0,
                    sides,
                })
            })
            .collect();
        Self::new(brushes)
    }

    pub(crate) fn new(brushes: Vec<BrushIn>) -> Self {
        let mut sides = Vec::new();
        let mut solids = Vec::new();
        for brush in brushes {
            let corners: Vec<Vec3> = brush
                .sides
                .iter()
                .flat_map(|side| side.polygon.iter().copied())
                .collect();
            if brush.occludes && !corners.is_empty() {
                let (low, high) = bounds(&corners);
                solids.push(Solid {
                    brush: brush.brush,
                    planes: brush
                        .sides
                        .iter()
                        .map(|side| (side.normal, side.distance))
                        .collect(),
                    corners,
                    low,
                    high,
                });
            }
            for side in brush.sides {
                if area(&side.polygon) < MIN_AREA {
                    continue;
                }
                let (low, high) = bounds(&side.polygon);
                sides.push(Side {
                    brush: brush.brush,
                    side: side.side,
                    shader: side.shader,
                    showable: side.showable,
                    sky: side.sky,
                    polygon: side.polygon,
                    normal: side.normal,
                    distance: side.distance,
                    low,
                    high,
                });
            }
        }
        let mut side_grid = Grid::default();
        for (index, side) in sides.iter().enumerate() {
            side_grid.insert(side.low, side.high, index);
        }
        let mut solid_grid = Grid::default();
        for (index, solid) in solids.iter().enumerate() {
            solid_grid.insert(solid.low, solid.high, index);
        }
        let by_number = sides
            .iter()
            .enumerate()
            .map(|(index, side)| ((side.brush, side.side), index))
            .collect();
        Self {
            sides,
            solids,
            side_grid,
            solid_grid,
            by_number,
        }
    }

    /// How many sides there are.
    pub(crate) fn len(&self) -> usize {
        self.sides.len()
    }

    pub(crate) fn side(&self, index: usize) -> &Side {
        &self.sides[index]
    }

    /// The index of side `side` of brush `brush`, when it is a world brush's.
    pub(crate) fn find(&self, brush: u32, side: u32) -> Option<usize> {
        self.by_number.get(&(brush, side)).copied()
    }

    /// What a player can see of side `index`: its polygon with what the other solid
    /// brushes touching it cover cut away, as convex pieces, each of
    /// [`MIN_AREA`] at least and in front of an open place (`open`: a point is in an
    /// open leaf of the map). Empty when the pieces keep less than
    /// [`MIN_VISIBLE_AREA`] in all.
    pub(crate) fn seen_pieces(&self, index: usize, open: &dyn Fn(Vec3) -> bool) -> Vec<Vec<Vec3>> {
        let side = &self.sides[index];
        let mut pieces = vec![side.polygon.clone()];
        let reach = Vec3::splat(COVER_EPSILON + 0.01);
        for solid in self.solid_grid.near(side.low - reach, side.high + reach) {
            let solid = &self.solids[solid];
            if solid.brush == side.brush
                || !overlaps(solid.low, solid.high, side.low - reach, side.high + reach)
            {
                continue;
            }
            // A brush wholly behind the face's plane cannot cover its front.
            if solid
                .corners
                .iter()
                .all(|corner| side.normal.dot(*corner) - side.distance <= COVER_EPSILON)
            {
                continue;
            }
            pieces = pieces
                .into_iter()
                .flat_map(|piece| outside(piece, &solid.planes))
                .collect();
            if pieces.is_empty() {
                return pieces;
            }
        }
        pieces.retain(|piece| {
            area(piece) >= MIN_AREA && open(centre(piece) + side.normal * OPEN_OFFSET)
        });
        let total: f32 = pieces.iter().map(|piece| area(piece)).sum();
        if total < MIN_VISIBLE_AREA {
            pieces.clear();
        }
        pieces
    }

    /// Whether a player can see some of side `index` ([`Self::seen_pieces`]).
    pub(crate) fn visible(&self, index: usize, open: &dyn Fn(Vec3) -> bool) -> bool {
        !self.seen_pieces(index, open).is_empty()
    }

    /// The faces joined to face `start` (an index of [`Self::find`]) that players see,
    /// growing by `grow`, at most `limit` of them with `start` first. Faces join where
    /// what players see of them meets: a side half covered by a lower floor joins that
    /// floor along the cut.
    pub(crate) fn link(
        &self,
        start: usize,
        grow: Grow,
        limit: usize,
        open: &dyn Fn(Vec3) -> bool,
    ) -> Linked {
        let mut linked = Linked {
            faces: vec![start],
            ..Linked::default()
        };
        let mut seen: HashSet<usize> = HashSet::from([start]);
        let mut pieces: HashMap<usize, Vec<Vec<Vec3>>> = HashMap::new();
        let mut first = self.seen_pieces(start, open);
        if first.is_empty() {
            // Clicked, so seen somewhere: grow from the whole of it.
            first = vec![self.sides[start].polygon.clone()];
        }
        pieces.insert(start, first);
        let mut queue = VecDeque::from([start]);
        let reach = Vec3::splat(EDGE_EPSILON);
        let sky = self.sides[start].sky;
        'grow: while let Some(current) = queue.pop_front() {
            let face = &self.sides[current];
            for other in self.side_grid.near(face.low - reach, face.high + reach) {
                let candidate = &self.sides[other];
                if seen.contains(&other)
                    || !candidate.showable
                    || candidate.sky != sky
                    || !overlaps(
                        candidate.low,
                        candidate.high,
                        face.low - reach,
                        face.high + reach,
                    )
                    || !facing(face, candidate, grow)
                {
                    continue;
                }
                let current_pieces = &pieces[&current];
                // Touching at all (its whole polygon), before the cost of what is seen.
                if shared(current_pieces, std::slice::from_ref(&candidate.polygon)) < MIN_SHARED {
                    continue;
                }
                let candidate_pieces = self.seen_pieces(other, open);
                if candidate_pieces.is_empty() {
                    seen.insert(other);
                    linked.buried += 1;
                    continue;
                }
                if shared(current_pieces, &candidate_pieces) < MIN_SHARED {
                    // They touch where one of them is covered: another face may join it.
                    continue;
                }
                seen.insert(other);
                if linked.faces.len() >= limit {
                    linked.capped = true;
                    break 'grow;
                }
                pieces.insert(other, candidate_pieces);
                linked.faces.push(other);
                queue.push_back(other);
            }
        }
        linked
    }
}

/// Whether faces `a` and `b` lie as `grow` joins them.
fn facing(a: &Side, b: &Side, grow: Grow) -> bool {
    let facing = a.normal.dot(b.normal);
    match grow {
        Grow::Plane => {
            facing >= 1.0 - NORMAL_EPSILON && (a.distance - b.distance).abs() <= PLANE_EPSILON
        }
        Grow::Surface => facing > BACK_TO_BACK,
    }
}

/// How long the edges of pieces `a` and pieces `b` run together.
fn shared(a: &[Vec<Vec3>], b: &[Vec<Vec3>]) -> f32 {
    a.iter()
        .flat_map(|a| b.iter().map(move |b| shared_length(a, b, EDGE_EPSILON)))
        .sum()
}

/// How long the edges of polygons `a` and `b` run together (within `epsilon`).
pub(crate) fn shared_length(a: &[Vec3], b: &[Vec3], epsilon: f32) -> f32 {
    let mut total = 0.0;
    for i in 0..a.len() {
        let (p, q) = (a[i], a[(i + 1) % a.len()]);
        let along = q - p;
        let length = along.length();
        if length < 1e-3 {
            continue;
        }
        let direction = along / length;
        for j in 0..b.len() {
            let (r, s) = (b[j], b[(j + 1) % b.len()]);
            let off = |point: Vec3| {
                let v = point - p;
                (v - direction * v.dot(direction)).length()
            };
            if off(r) > epsilon || off(s) > epsilon {
                continue;
            }
            let (t0, t1) = ((r - p).dot(direction), (s - p).dot(direction));
            let overlap = t0.max(t1).min(length) - t0.min(t1).max(0.0);
            if overlap > 0.0 {
                total += overlap;
            }
        }
    }
    total
}

/// What of convex polygon `piece` lies outside the convex brush of `planes` (normals
/// out), as convex pieces; a part flush against it counts as inside.
fn outside(piece: Vec<Vec3>, planes: &[(Vec3, f32)]) -> Vec<Vec<Vec3>> {
    let mut out = Vec::new();
    let mut rest = piece;
    for &(normal, distance) in planes {
        let (front, back) = split(&rest, normal, distance + COVER_EPSILON);
        if front.len() >= 3 {
            out.push(front);
        }
        if back.len() < 3 {
            return out;
        }
        rest = back;
    }
    // What is left is inside the brush: covered.
    out
}

/// Polygon `points` cut by the plane `normal`·p = `distance`: the part in front, and
/// the part behind (on it: both).
pub(crate) fn split(points: &[Vec3], normal: Vec3, distance: f32) -> (Vec<Vec3>, Vec<Vec3>) {
    let (mut front, mut back) = (Vec::new(), Vec::new());
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let (da, db) = (normal.dot(a) - distance, normal.dot(b) - distance);
        if da >= 0.0 {
            front.push(a);
        }
        if da <= 0.0 {
            back.push(a);
        }
        if (da > 0.0 && db < 0.0) || (da < 0.0 && db > 0.0) {
            let cut = a + (b - a) * (da / (da - db));
            front.push(cut);
            back.push(cut);
        }
    }
    (front, back)
}

pub(crate) fn area(points: &[Vec3]) -> f32 {
    let Some(&first) = points.first() else {
        return 0.0;
    };
    let mut sum = Vec3::ZERO;
    for pair in points[1..].windows(2) {
        sum += (pair[0] - first).cross(pair[1] - first);
    }
    sum.length() * 0.5
}

pub(crate) fn centre(points: &[Vec3]) -> Vec3 {
    points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32
}

pub(crate) fn bounds(points: &[Vec3]) -> (Vec3, Vec3) {
    points.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(low, high), point| (low.min(*point), high.max(*point)),
    )
}

pub(crate) fn overlaps(a_low: Vec3, a_high: Vec3, b_low: Vec3, b_high: Vec3) -> bool {
    a_low.cmple(b_high).all() && b_low.cmple(a_high).all()
}

/// Things by the cells of space their boxes touch; a box over too many cells is kept
/// aside and always offered.
#[derive(Default)]
pub(crate) struct Grid {
    cells: HashMap<[i32; 3], Vec<usize>>,
    big: Vec<usize>,
}

impl Grid {
    const CELL: f32 = 256.0;
    const MOST_CELLS: i64 = 512;

    fn cell_range(low: Vec3, high: Vec3) -> ([i32; 3], [i32; 3]) {
        let cell = |v: f32| (v / Self::CELL).floor().clamp(-1e6, 1e6) as i32;
        (
            [cell(low.x), cell(low.y), cell(low.z)],
            [cell(high.x), cell(high.y), cell(high.z)],
        )
    }

    pub(crate) fn insert(&mut self, low: Vec3, high: Vec3, item: usize) {
        let (a, b) = Self::cell_range(low, high);
        let count = (0..3)
            .map(|axis| i64::from(b[axis] - a[axis] + 1))
            .product::<i64>();
        if count > Self::MOST_CELLS {
            self.big.push(item);
            return;
        }
        for x in a[0]..=b[0] {
            for y in a[1]..=b[1] {
                for z in a[2]..=b[2] {
                    self.cells.entry([x, y, z]).or_default().push(item);
                }
            }
        }
    }

    /// The things whose cells meet the box (some may lie outside it), each once.
    pub(crate) fn near(&self, low: Vec3, high: Vec3) -> Vec<usize> {
        let (a, b) = Self::cell_range(low, high);
        let mut found: Vec<usize> = self.big.clone();
        let count = (0..3)
            .map(|axis| i64::from(b[axis] - a[axis] + 1))
            .product::<i64>();
        if count > Self::MOST_CELLS {
            found.extend(self.cells.values().flatten().copied());
        } else {
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        if let Some(items) = self.cells.get(&[x, y, z]) {
                            found.extend_from_slice(items);
                        }
                    }
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// An axial box brush: its six sides (−x, +x, −y, +y, −z, +z), each with the
    /// shader `shader` gives its number.
    pub(crate) fn box_brush(
        brush: u32,
        low: [f32; 3],
        high: [f32; 3],
        shader: impl Fn(usize) -> &'static str,
    ) -> BrushIn {
        let (low, high) = (Vec3::from_array(low), Vec3::from_array(high));
        let corner = |i: usize| {
            Vec3::new(
                if i & 1 == 0 { low.x } else { high.x },
                if i & 2 == 0 { low.y } else { high.y },
                if i & 4 == 0 { low.z } else { high.z },
            )
        };
        let faces: [([usize; 4], Vec3, f32); 6] = [
            ([0, 4, 6, 2], Vec3::NEG_X, -low.x),
            ([1, 3, 7, 5], Vec3::X, high.x),
            ([0, 1, 5, 4], Vec3::NEG_Y, -low.y),
            ([2, 6, 7, 3], Vec3::Y, high.y),
            ([0, 2, 3, 1], Vec3::NEG_Z, -low.z),
            ([4, 5, 7, 6], Vec3::Z, high.z),
        ];
        BrushIn {
            brush,
            occludes: true,
            sides: faces
                .into_iter()
                .enumerate()
                .map(|(index, (ring, normal, distance))| {
                    let name = shader(index);
                    SideIn {
                        side: brush * 6 + index as u32,
                        normal,
                        distance,
                        polygon: ring.map(corner).to_vec(),
                        shader: name.to_owned(),
                        showable: name.contains("caulk") || name.contains("skies"),
                        sky: name.contains("skies"),
                    }
                })
                .collect(),
        }
    }

    const CAULK: &str = "textures/system/caulk";
    const TOP: usize = 5;
    const OPEN: &dyn Fn(Vec3) -> bool = &|_| true;

    /// A caulk floor in three brushes side by side (x 0-64, 64-128, 128-192, top at
    /// z 0), a crate standing on the middle one, a fourth floor brush two units lower
    /// beyond the third, and a block whose top is buried under a lid brush.
    fn floor() -> Sides {
        Sides::new(vec![
            box_brush(0, [0.0, 0.0, -16.0], [64.0, 64.0, 0.0], |_| CAULK),
            box_brush(1, [64.0, 0.0, -16.0], [128.0, 64.0, 0.0], |_| CAULK),
            box_brush(2, [128.0, 0.0, -16.0], [192.0, 64.0, 0.0], |_| CAULK),
            box_brush(3, [192.0, 0.0, -18.0], [256.0, 64.0, -2.0], |_| CAULK),
            // a crate on the middle brush, drawn on every side
            box_brush(
                4,
                [80.0, 16.0, 0.0],
                [112.0, 48.0, 32.0],
                |_| "textures/a/crate",
            ),
            // beside the floor, a caulk block whose top a lid covers flush
            box_brush(5, [0.0, -64.0, -16.0], [64.0, 0.0, 0.0], |_| CAULK),
            box_brush(6, [0.0, -64.0, 0.0], [64.0, 0.0, 8.0], |_| "textures/a/lid"),
        ])
    }

    fn top(sides: &Sides, brush: u32) -> usize {
        sides.find(brush, brush * 6 + TOP as u32).unwrap()
    }

    #[test]
    fn a_floor_in_three_brushes_links_on_its_plane_and_leaves_the_rest() {
        let sides = floor();
        let linked = sides.link(top(&sides, 0), Grow::Plane, LINK_LIMIT, OPEN);
        let mut faces = linked.faces.clone();
        faces.sort_unstable();
        let mut expected = vec![top(&sides, 0), top(&sides, 1), top(&sides, 2)];
        expected.sort_unstable();
        // the three tops; not the lower fourth brush, nor the buried top beside them
        assert_eq!(faces, expected);
        assert_eq!(linked.faces[0], top(&sides, 0));
        assert_eq!(linked.buried, 1, "the covered top beside the floor");
        assert!(!linked.capped);
        // the middle top keeps its area round the crate's foot
        assert!(sides.visible(top(&sides, 1), OPEN));
        assert!(!sides.visible(top(&sides, 5), OPEN));
    }

    #[test]
    fn a_connected_surface_turns_corners_but_skips_what_is_covered() {
        let sides = floor();
        let linked = sides.link(top(&sides, 0), Grow::Surface, LINK_LIMIT, OPEN);
        let has = |brush: u32, side: usize| {
            linked
                .faces
                .contains(&sides.find(brush, brush * 6 + side as u32).unwrap())
        };
        // the three tops, and the step down to the fourth brush: the third brush's +x
        // side shows above the lower top, and the lower top joins along its foot
        assert!(has(0, TOP) && has(1, TOP) && has(2, TOP));
        assert!(has(2, 1), "the step's face");
        assert!(has(3, TOP), "the lower floor, through the step");
        // sides flush against the next brush are covered
        assert!(!has(0, 1) && !has(1, 0));
        assert!(!has(5, TOP));
    }

    #[test]
    fn a_link_stops_at_its_limit_and_says_so() {
        let sides = floor();
        let linked = sides.link(top(&sides, 0), Grow::Plane, 2, OPEN);
        assert_eq!(linked.faces.len(), 2);
        assert!(linked.capped);
    }

    /// A sky face links the sky round it, never the caulk beside it, and a caulk face
    /// never the sky.
    #[test]
    fn sky_links_with_sky_only() {
        const SKY: &str = "textures/skies/yavin";
        // two sky ceilings side by side (tops at z 256), a caulk ceiling beside them
        let sides = Sides::new(vec![
            box_brush(0, [0.0, 0.0, 240.0], [64.0, 64.0, 256.0], |_| SKY),
            box_brush(1, [64.0, 0.0, 240.0], [128.0, 64.0, 256.0], |_| SKY),
            box_brush(2, [128.0, 0.0, 240.0], [192.0, 64.0, 256.0], |_| CAULK),
        ]);
        const BOTTOM: usize = 4;
        let bottom = |brush: u32| sides.find(brush, brush * 6 + BOTTOM as u32).unwrap();
        for grow in [Grow::Plane, Grow::Surface] {
            let linked = sides.link(bottom(0), grow, LINK_LIMIT, OPEN);
            assert!(linked.faces.contains(&bottom(1)), "{grow:?}");
            assert!(linked.faces.iter().all(|&i| sides.side(i).sky), "{grow:?}");
            let linked = sides.link(bottom(2), grow, LINK_LIMIT, OPEN);
            assert!(linked.faces.iter().all(|&i| !sides.side(i).sky), "{grow:?}");
        }
    }

    #[test]
    fn a_face_in_the_void_is_not_seen() {
        let sides = floor();
        assert!(!sides.visible(top(&sides, 0), &|_| false));
    }

    #[test]
    fn edges_run_together_over_their_overlap() {
        let a = [
            Vec3::ZERO,
            Vec3::new(64.0, 0.0, 0.0),
            Vec3::new(64.0, 64.0, 0.0),
            Vec3::new(0.0, 64.0, 0.0),
        ];
        // beside it, half as tall: 32 units of the x = 64 edge
        let b = [
            Vec3::new(64.0, 0.0, 0.0),
            Vec3::new(96.0, 0.0, 0.0),
            Vec3::new(96.0, 32.0, 0.0),
            Vec3::new(64.0, 32.0, 0.0),
        ];
        assert!((shared_length(&a, &b, 0.5) - 32.0).abs() < 1e-3);
        let far = b.map(|p| p + Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(shared_length(&a, &far, 0.5), 0.0);
    }
}
