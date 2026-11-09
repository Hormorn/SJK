//! Local lights of the real-time mode, from the map's emissive surfaces: q3map bakes
//! `q3map_surfacelight` faces as area lights and strips `light` entities from the BSP,
//! so the lamp faces are the map's lamps. Each connected patch of emissive triangles
//! becomes one Lambertian area light (position, normal, colour, power = radiance × area)
//! with a radius where its irradiance falls below a floor, filed in a world grid so a
//! pixel visits only the lamps of its cell.
use glam::{IVec3, Vec3};
#[path = "lamp_emitters.rs"]
mod emitters;
pub(crate) use emitters::collect as collect_patches;
#[path = "lamp_grid.rs"]
mod grid;
#[path = "lamp_grid_refine.rs"]
mod refine;
#[path = "lamp_grid_selection.rs"]
mod selection;
#[path = "lamp_visibility.rs"]
mod visibility;
pub(crate) use visibility::FORMAT as VISIBILITY_FORMAT;

/// One lamp: an area light seen from beyond its own size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Lamp {
    pub(crate) position: Vec3,
    /// Zero denotes a two-sided area, or an omnidirectional glow when axes are zero.
    pub(crate) normal: Vec3,
    pub(crate) color: [f32; 3],
    /// Radiance times area, display units (a sunlit white surface is about one).
    pub(crate) power: f32,
    pub(crate) radius: f32,
    /// Half-axes of the finite emitting rectangle.
    pub(crate) axis_u: Vec3,
    pub(crate) axis_v: Vec3,
    /// The fixture the lamp comes from, so map edits can turn it off or move it.
    pub(crate) fixture: Fixture,
}

/// Which of the map's things a lamp comes from: one BSP surface, one surface of a
/// `misc_model_static` prop's model, or one steady effect. Stable across loads of the
/// same BSP (the numbers are the file's own), so a map edit names it
/// ([`crate::map_edits::document::Kind::Lamp`]).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Fixture {
    /// Not tied to one thing (a vertex-coloured cluster over several surfaces): not
    /// editable.
    #[default]
    None,
    /// While lamps are found: the index-buffer position of the first triangle of the
    /// patch, resolved to its [`Fixture::Surface`] by the world's build.
    Index(u32),
    /// BSP surface `0`.
    Surface(u32),
    /// Surface `surface` of prop `entity`'s model (its number in the entity lump).
    Prop { entity: u32, surface: u32 },
    /// The steady `fx_runner` of entity number `0`.
    Effect(u32),
}

impl Fixture {
    /// Read a fixture as [`std::fmt::Display`] writes it: `surface:1043`, `prop:12:3`,
    /// `effect:40`. `None` for anything else (and for the build's own kinds).
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split(':');
        let kind = parts.next()?;
        let mut number = || parts.next()?.parse::<u32>().ok();
        let fixture = match kind {
            "surface" => Self::Surface(number()?),
            "prop" => Self::Prop {
                entity: number()?,
                surface: number()?,
            },
            "effect" => Self::Effect(number()?),
            _ => return None,
        };
        parts.next().is_none().then_some(fixture)
    }

    /// Whether a map edit may name it.
    pub(crate) fn editable(self) -> bool {
        matches!(self, Self::Surface(_) | Self::Prop { .. } | Self::Effect(_))
    }
}

impl std::fmt::Display for Fixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Index(index) => write!(f, "index:{index}"),
            Self::Surface(surface) => write!(f, "surface:{surface}"),
            Self::Prop { entity, surface } => write!(f, "prop:{entity}:{surface}"),
            Self::Effect(entity) => write!(f, "effect:{entity}"),
        }
    }
}

/// What map edits do to the lamps when a world is built: fixtures turned off, and
/// fixtures moved (by an offset). Empty off a map with no lamp edits.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LampEdits {
    pub(crate) off: std::collections::BTreeSet<Fixture>,
    pub(crate) moved: std::collections::BTreeMap<Fixture, Vec3>,
}

impl LampEdits {
    pub(crate) fn is_empty(&self) -> bool {
        self.off.is_empty() && self.moved.is_empty()
    }

    /// Drop the lamps of fixtures turned off and move those of fixtures moved; the rest
    /// keep their order.
    pub(crate) fn apply(&self, lamps: &mut Vec<Lamp>) {
        if self.is_empty() {
            return;
        }
        lamps.retain(|lamp| !self.off.contains(&lamp.fixture));
        for lamp in lamps.iter_mut() {
            if let Some(offset) = self.moved.get(&lamp.fixture) {
                lamp.position += *offset;
            }
        }
    }
}

/// One fixture as the map editor shows it: where its lamps are (their power-weighted
/// middle), their colour, total power, reach and count, as found before any edit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FixtureSummary {
    pub(crate) fixture: Fixture,
    pub(crate) position: Vec3,
    pub(crate) color: [f32; 3],
    pub(crate) power: f32,
    pub(crate) radius: f32,
    pub(crate) lamps: u32,
}

/// The editable fixtures of `lamps`, one each, in fixture order.
pub(crate) fn summarise(lamps: &[Lamp]) -> Vec<FixtureSummary> {
    let mut sums: std::collections::BTreeMap<Fixture, (Vec3, Vec3, f32, f32, f32, u32)> =
        Default::default();
    for lamp in lamps.iter().filter(|lamp| lamp.fixture.editable()) {
        let weight = lamp.power.max(1e-6);
        let sum = sums.entry(lamp.fixture).or_default();
        sum.0 += lamp.position * weight;
        sum.1 += Vec3::from_array(lamp.color) * weight;
        sum.2 += weight;
        sum.3 += lamp.power;
        sum.4 = sum.4.max(lamp.radius);
        sum.5 += 1;
    }
    sums.into_iter()
        .map(
            |(fixture, (position, color, weight, power, radius, count))| FixtureSummary {
                fixture,
                position: position / weight,
                color: (color / weight).to_array(),
                power,
                radius,
                lamps: count,
            },
        )
        .collect()
}

/// Every lamp of a map and the grid that finds them.
#[derive(Default)]
pub(crate) struct LampSet {
    pub(crate) lamps: Vec<Lamp>,
    pub(crate) cell: f32,
    pub(crate) origin: IVec3,
    pub(crate) counts: [u32; 3],
    /// Root cells, followed by optional children: list offset/count, or child-table
    /// offset and `refine::BRANCH | side` for a crowded root.
    pub(crate) cells: Vec<[u32; 2]>,
    pub(crate) cell_lamps: Vec<u32>,
    /// Shared node thresholds, sampled with smooth trilinear weights.
    pub(crate) thresholds: Vec<f32>,
}

/// Irradiance below which a lamp's reach ends.
pub(crate) const FLOOR: f32 = 0.004;
/// Scale from surface radiance × area to lamp power (calibrated against the bake).
pub(crate) const POWER_SCALE: f32 = 1.5;

/// One emissive material's contribution: its radiance (display units) and its triangles.
pub(crate) struct Emitter<'a> {
    /// A compact fixture shares one influence range across its luminous pieces.
    pub(crate) shared_reach: bool,
    pub(crate) omnidirectional: bool,
    pub(crate) texture: &'a crate::world_materials::emission::Texture,
    pub(crate) radiance: [f32; 3],
    pub(crate) ranges: Vec<std::ops::Range<u32>>,
    /// The fixture its lamps come from; `None`: each patch's own first triangle
    /// ([`Fixture::Index`]), for the world's build to resolve.
    pub(crate) fixture: Option<Fixture>,
}

impl LampSet {
    /// The set of the map's lamps, found ([`collect_patches`] and the other sources,
    /// then map edits'), with their shared lookup, built exactly once.
    pub(crate) fn from_lamps(lamps: Vec<Lamp>) -> Self {
        grid::build(lamps)
    }
}

/// GPU form: a grid uniform and one storage buffer of vec4<u32> holding the lamps (five
/// vec4 of floats each, bit-cast), the cell table (two cells per vec4: offset, count) and
/// the cell lamp lists (eight 16-bit indices per vec4 while every index fits, else four
/// 32-bit ones; [`NARROW_LISTS`]). One buffer keeps the compute stage of the probe update
/// within the eight storage buffers a baseline device allows.
pub(crate) struct Gpu {
    pub(crate) grid: wgpu::Buffer,
    pub(crate) data: wgpu::Buffer,
    visibility: wgpu::TextureView,
    /// Directional samples per edge of the traced atlas; `None` before tracing.
    resolution: Option<u32>,
    lamp_count: u32,
    /// Mover door tiles after the lamps' own in the atlas (`mover_occlusion.rs`).
    door_tiles: u32,
    doors: Option<crate::world_materials::mover_occlusion::Doors>,
}

/// Lamp counts up to which the cell lists hold 16-bit indices (`lamp_list_lamp`), half the
/// memory of 32-bit ones: the refined lists of mp/ffa5 (10,023 lamps, 34.6 million
/// entries) took 132 of the buffer's 150 MiB until 25/10/2026.
const NARROW_LISTS: usize = 1 << 16;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Grid {
    origin: [i32; 4],
    counts: [u32; 4],
    cell: [f32; 4],
    offsets: [u32; 4],
    selection: [u32; 4],
}

impl Gpu {
    /// Upload a set; an empty set is the neutral stand-in (zero counts, one dummy lamp).
    /// Lamps that `occluders` (the map's movers) can reach get door tiles, as many as the
    /// atlas holds without lowering its resolution.
    pub(crate) fn new(
        device: &wgpu::Device,
        set: &LampSet,
        occluders: &[crate::world_materials::mover_occlusion::Occluder],
    ) -> Self {
        use crate::world_materials::mover_occlusion::Doors;
        use wgpu::util::DeviceExt;
        let make = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let lamps: &[Lamp] = if set.lamps.is_empty() {
            &[Lamp {
                position: Vec3::ZERO,
                normal: Vec3::Z,
                color: [0.; 3],
                power: 0.,
                radius: 0.,
                axis_u: Vec3::ZERO,
                axis_v: Vec3::ZERO,
                fixture: Fixture::None,
            }]
        } else {
            &set.lamps
        };
        let capacity = visibility::door_capacity(
            set.lamps.len() as u32,
            device.limits().max_texture_dimension_2d,
        );
        let doors = Doors::assign(&set.lamps, occluders, capacity as usize);
        // Per lamp: its door tile + 1, zero without one, in the first axis's spare word.
        let mut door_of = vec![0u32; lamps.len()];
        for (tile, door) in doors.tiles.iter().enumerate() {
            door_of[door.lamp as usize] = tile as u32 + 1;
        }
        let mut data: Vec<[u32; 4]> = Vec::new();
        for (l, door) in lamps.iter().zip(&door_of) {
            data.push([l.position.x, l.position.y, l.position.z, l.radius].map(f32::to_bits));
            data.push([l.normal.x, l.normal.y, l.normal.z, l.power].map(f32::to_bits));
            let area = 4. * l.axis_u.cross(l.axis_v).length();
            data.push(
                [
                    l.color[0],
                    l.color[1],
                    l.color[2],
                    if area > 1e-6 { 1. / area } else { 0. },
                ]
                .map(f32::to_bits),
            );
            let [x, y, z] = l.axis_u.to_array().map(f32::to_bits);
            data.push([x, y, z, *door]);
            data.push(l.axis_v.extend(0.).to_array().map(f32::to_bits));
        }
        let cells_offset = data.len() as u32;
        let cells: &[[u32; 2]] = if set.cells.is_empty() {
            &[[0, 0]]
        } else {
            &set.cells
        };
        for pair in cells.chunks(2) {
            let second = pair.get(1).copied().unwrap_or([0, 0]);
            data.push([pair[0][0], pair[0][1], second[0], second[1]]);
        }
        let lists_offset = data.len() as u32;
        let lists: &[u32] = if set.cell_lamps.is_empty() {
            &[0]
        } else {
            &set.cell_lamps
        };
        let narrow = lamps.len() <= NARROW_LISTS;
        push_lists(&mut data, lists, narrow);
        let threshold_offset = data.len() as u32;
        for values in set.thresholds.chunks(4) {
            let mut entry = [0u32; 4];
            for (to, value) in entry.iter_mut().zip(values) {
                *to = value.to_bits();
            }
            data.push(entry);
        }
        let grid = Grid {
            origin: [set.origin.x, set.origin.y, set.origin.z, 0],
            counts: [
                set.counts[0],
                set.counts[1],
                set.counts[2],
                set.lamps.len() as u32,
            ],
            cell: [set.cell.max(1.), 0., 0., 0.],
            offsets: [cells_offset, lists_offset, 0, 0],
            selection: [
                threshold_offset,
                u32::from(!set.thresholds.is_empty()),
                u32::from(narrow),
                doors.tiles.len() as u32,
            ],
        };
        Self {
            visibility: visibility::texture(device, [1, 1]),
            resolution: None,
            lamp_count: set.lamps.len() as u32,
            door_tiles: doors.tiles.len() as u32,
            doors: (!doors.tiles.is_empty()).then_some(doors),

            grid: make(
                "SJK lamp grid",
                bytemuck::bytes_of(&grid),
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
            data: make(
                "SJK lamps",
                bytemuck::cast_slice(&data),
                wgpu::BufferUsages::STORAGE,
            ),
        }
    }

    /// The door tiles, for the mover occlusion runtime; `None` after the first call or
    /// without any.
    pub(crate) fn take_doors(&mut self) -> Option<crate::world_materials::mover_occlusion::Doors> {
        self.doors.take()
    }

    /// The visibility atlas, lamp tiles then door tiles.
    pub(crate) fn visibility_view(&self) -> &wgpu::TextureView {
        &self.visibility
    }

    /// Directional samples per tile edge, once the atlas is traced.
    pub(crate) fn visibility_resolution(&self) -> Option<u32> {
        self.resolution
    }

    /// Layout entries for the two lamp bindings at `base`, fragment and compute visible.
    pub(crate) fn layout_entries(base: u32) -> [wgpu::BindGroupLayoutEntry; 2] {
        let stages = wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT;
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding: base + binding,
            visibility: stages,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        [
            buffer(0, wgpu::BufferBindingType::Uniform),
            buffer(1, wgpu::BufferBindingType::Storage { read_only: true }),
        ]
    }

    /// Bind group entries at `base`, matching `layout_entries`.
    pub(crate) fn entries(&self, base: u32) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: base,
                resource: self.grid.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: base + 1,
                resource: self.data.as_entire_binding(),
            },
        ]
    }
}

/// Append the cell lamp lists to `data`: with `narrow`, two 16-bit indices to a word, the
/// even entry in the low half (`lamp_list_lamp` reads them).
fn push_lists(data: &mut Vec<[u32; 4]>, lists: &[u32], narrow: bool) {
    for chunk in lists.chunks(if narrow { 8 } else { 4 }) {
        let mut entry = [0u32; 4];
        if narrow {
            for (k, &lamp) in chunk.iter().enumerate() {
                debug_assert!(lamp < 1 << 16);
                entry[k / 2] |= lamp << (16 * (k % 2));
            }
        } else {
            entry[..chunk.len()].copy_from_slice(chunk);
        }
        data.push(entry);
    }
}

pub(crate) const SHADER: &str = include_str!("lamp_lights.wgsl");

/// The lamp module at `group` from `base`: bindings `base`, `base+1` for the lamps and,
/// when `shadowed`, `base+4..base+6` for the shadow maps and their table; unshadowed
/// (the probe update) visibility comes from the same static fixture cache.
pub(crate) fn source(group: u32, base: u32, shadowed: bool) -> String {
    let mut source = format!("{}{}", include_str!("lamp_area.wgsl"), SHADER);
    if !shadowed {
        let begin = source.find("// LAMP_SHADOWS_BEGIN").expect("marker");
        let end = source.find("// LAMP_SHADOWS_END").expect("marker");
        source.replace_range(begin..end, "// LAMP_SHADOWS_BEGIN\n");
        source = source.replace(
            "let shadow = lamp_shadow(lamp/5u, world, -to, normal);",
            "let shadow = lamp_static_visibility(lamp/5u, world, normal);",
        );
    }
    {
        source = source.replace(
            "// LAMP_SHADOWS_BEGIN",
            &format!(
                "// LAMP_SHADOWS_BEGIN\n{}{}",
                include_str!("lamp_visibility_common.wgsl"),
                include_str!("lamp_visibility_sample.wgsl")
            ),
        );
        source = source.replace(
            "@group(3) @binding(19)",
            &format!(
                "@group({group}) @binding({})",
                if shadowed { base - 5 } else { 6 }
            ),
        );
    }
    for (from, to) in [(24, 0), (25, 1), (28, 4), (29, 5), (30, 6)] {
        source = source.replace(
            &format!("@group(3) @binding({from})"),
            &format!("@group({group}) @binding({})", base + to),
        );
    }
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `lamp_list_lamp` of lamp_lights.wgsl.
    fn list_lamp(data: &[[u32; 4]], narrow: bool, k: u32) -> u32 {
        if !narrow {
            return data[(k / 4) as usize][(k & 3) as usize];
        }
        let word = data[(k / 8) as usize][((k / 2) & 3) as usize];
        (word >> ((k & 1) * 16)) & 0xffff
    }

    fn lamp(position: [f32; 3], power: f32, fixture: Fixture) -> Lamp {
        Lamp {
            position: Vec3::from_array(position),
            normal: Vec3::Z,
            color: [1.0, 0.5, 0.0],
            power,
            radius: 100.0 + power,
            axis_u: Vec3::ZERO,
            axis_v: Vec3::ZERO,
            fixture,
        }
    }

    #[test]
    fn fixtures_read_back_as_written() {
        for fixture in [
            Fixture::Surface(1043),
            Fixture::Prop {
                entity: 12,
                surface: 3,
            },
            Fixture::Effect(40),
        ] {
            assert_eq!(Fixture::parse(&fixture.to_string()), Some(fixture));
            assert!(fixture.editable());
        }
        for text in [
            "",
            "surface",
            "surface:",
            "surface:-1",
            "prop:1",
            "prop:1:2:3",
            "index:4",
            "none",
        ] {
            assert_eq!(Fixture::parse(text), None, "{text}");
        }
        assert!(!Fixture::Index(3).editable());
        assert!(!Fixture::None.editable());
    }

    /// The ten thousand patches of a big map show as one marker per fixture: their
    /// power-weighted middle, colour and count; unnamed lamps are left out.
    #[test]
    fn lamps_group_by_fixture() {
        let lamps = [
            lamp([0.0, 0.0, 0.0], 1.0, Fixture::Surface(7)),
            lamp([30.0, 0.0, 0.0], 2.0, Fixture::Surface(7)),
            lamp(
                [0.0, 90.0, 0.0],
                1.0,
                Fixture::Prop {
                    entity: 4,
                    surface: 0,
                },
            ),
            lamp([5.0, 5.0, 5.0], 9.0, Fixture::None),
        ];
        let fixtures = summarise(&lamps);
        assert_eq!(fixtures.len(), 2);
        let surface = fixtures
            .iter()
            .find(|f| f.fixture == Fixture::Surface(7))
            .unwrap();
        assert_eq!(surface.lamps, 2);
        assert!((surface.position - Vec3::new(20.0, 0.0, 0.0)).length() < 1e-4);
        assert!((surface.power - 3.0).abs() < 1e-6);
        assert_eq!(surface.radius, 102.0);
        assert_eq!(surface.color, [1.0, 0.5, 0.0]);
    }

    /// A fixture turned off loses all its lamps, a moved one moves all of its lamps, the
    /// others stay as they were and in their order.
    #[test]
    fn lamp_edits_turn_fixtures_off_and_move_them() {
        let mut lamps = vec![
            lamp([0.0, 0.0, 0.0], 1.0, Fixture::Surface(1)),
            lamp([10.0, 0.0, 0.0], 1.0, Fixture::Surface(2)),
            lamp([20.0, 0.0, 0.0], 1.0, Fixture::Surface(1)),
            lamp([30.0, 0.0, 0.0], 1.0, Fixture::Effect(9)),
            lamp([40.0, 0.0, 0.0], 1.0, Fixture::None),
        ];
        let mut edits = LampEdits::default();
        edits.off.insert(Fixture::Surface(1));
        edits
            .moved
            .insert(Fixture::Effect(9), Vec3::new(0.0, 0.0, 64.0));
        edits.apply(&mut lamps);
        let positions: Vec<[f32; 3]> = lamps.iter().map(|l| l.position.to_array()).collect();
        assert_eq!(
            positions,
            [[10.0, 0.0, 0.0], [30.0, 0.0, 64.0], [40.0, 0.0, 0.0]]
        );
        let before = lamps.clone();
        LampEdits::default().apply(&mut lamps);
        assert_eq!(lamps, before);
    }

    #[test]
    fn narrow_lists_read_back_the_same_lamps() {
        let lists: Vec<u32> = (0..1001).map(|k| (k * 7919 + 65535) % 65536).collect();
        for narrow in [false, true] {
            let mut data = Vec::new();
            push_lists(&mut data, &lists, narrow);
            assert_eq!(data.len(), lists.len().div_ceil(if narrow { 8 } else { 4 }));
            for (k, &lamp) in lists.iter().enumerate() {
                assert_eq!(list_lamp(&data, narrow, k as u32), lamp, "entry {k}");
            }
        }
    }
}
