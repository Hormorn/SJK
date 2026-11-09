//! Lamps from additive surfaces coloured by their vertices: flares such as
//! `flares/solid_blue` (`t2_rancor`) and `flares/solid_red` (`mp/ffa5`), drawn
//! `blendFunc add` with `rgbGen exactVertex` and no lightmap. Their colour lives in the
//! vertices, so the material alone cannot say how much they emit
//! (`world_emission::vertex_radiance` gives the light per unit vertex colour); here
//! each cluster of a material's static triangles becomes one small omnidirectional
//! light of its own colour, at map load. Sol's world notes asked for both flares to be
//! lights (08/10 and 12/10/2026).
use super::*;
use crate::lamp_lights::{FLOOR, Lamp, POWER_SCALE};
use glam::{IVec3, Vec3};

/// Triangles within this distance of a cluster's first one join it, as texture
/// patches merge in `lamp_emitters.rs`.
const MERGE: f32 = 96.;
/// The light sits this far in front of its surface, so its shadow trace does not
/// start inside the face it is drawn on.
const LIFT: f32 = 4.;

/// One cluster's sums while its triangles are added.
#[derive(Default)]
struct Cluster {
    anchor: Vec3,
    /// Radiance × area, per channel.
    flux: Vec3,
    /// Centroids weighted by each triangle's light, and that weight.
    centre: Vec3,
    weight: f32,
    /// Area-weighted normal.
    normal: Vec3,
}

pub(super) fn extract(
    materials: &[PendingMaterial],
    vertices: &[crate::GpuVertex],
    indices: &[u32],
) -> Vec<Lamp> {
    let mut lamps = Vec::new();
    for material in materials {
        let radiance = Vec3::from_array(material.vertex_emission);
        if !radiance.is_finite() || radiance.max_element() <= 0. {
            continue;
        }
        let triangles = material.static_draws.iter().flat_map(|draw| {
            indices[draw.indices.start as usize..draw.indices.end as usize].chunks_exact(3)
        });
        lamps.extend(clusters(
            radiance,
            triangles.map(|t| [t[0], t[1], t[2]].map(|i| &vertices[i as usize])),
        ));
    }
    crate::log::progress(format_args!(
        "Vertex-coloured flare lamps: {}, total power {:.0}",
        lamps.len(),
        lamps.iter().map(|lamp| lamp.power).sum::<f32>()
    ));
    lamps
}

/// The lamps of one material: `radiance` per unit (linear) vertex colour, over its
/// triangles.
fn clusters<'a>(
    radiance: Vec3,
    triangles: impl Iterator<Item = [&'a crate::GpuVertex; 3]>,
) -> Vec<Lamp> {
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut cells: std::collections::HashMap<IVec3, Vec<usize>> = Default::default();
    for corners in triangles {
        let p = corners.map(|v| Vec3::from_array(v.position));
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        let area = cross.length() * 0.5;
        if !area.is_finite() || area <= 1e-3 {
            continue;
        }
        // Vertex bytes are gamma-encoded like the texture they tint.
        let colour = corners
            .iter()
            .map(|v| Vec3::new(v.color[0], v.color[1], v.color[2]).powf(2.2))
            .sum::<Vec3>()
            / 3.;
        let flux = radiance * colour * area;
        let weight = luminance(flux);
        if weight <= 0. {
            continue;
        }
        let centroid = (p[0] + p[1] + p[2]) / 3.;
        let cell = (centroid / MERGE).floor().as_ivec3();
        let found = (-1..=1)
            .flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| IVec3::new(x, y, z))))
            .filter_map(|offset| cells.get(&(cell + offset)))
            .flatten()
            .copied()
            .find(|&id| clusters[id].anchor.distance_squared(centroid) < MERGE * MERGE);
        let id = found.unwrap_or_else(|| {
            cells.entry(cell).or_default().push(clusters.len());
            clusters.push(Cluster {
                anchor: centroid,
                ..Cluster::default()
            });
            clusters.len() - 1
        });
        let cluster = &mut clusters[id];
        cluster.flux += flux;
        cluster.centre += centroid * weight;
        cluster.weight += weight;
        cluster.normal += cross * 0.5;
    }
    clusters
        .into_iter()
        .filter_map(|cluster| {
            let luminance = luminance(cluster.flux);
            if !luminance.is_finite() || luminance <= 1e-6 {
                return None;
            }
            let power = luminance * POWER_SCALE;
            let position =
                cluster.centre / cluster.weight + cluster.normal.normalize_or_zero() * LIFT;
            Some(Lamp {
                position,
                normal: Vec3::ZERO,
                color: (cluster.flux / luminance).to_array(),
                power,
                radius: (power / FLOOR).sqrt().clamp(32., 1024.),
                axis_u: Vec3::ZERO,
                axis_v: Vec3::ZERO,
                // A cluster may span surfaces: not one fixture.
                fixture: crate::lamp_lights::Fixture::None,
            })
        })
        .collect()
}

fn luminance(colour: Vec3) -> f32 {
    colour.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(position: [f32; 3], color: [f32; 3]) -> crate::GpuVertex {
        crate::GpuVertex {
            position,
            normal: [0., 0., 1.],
            color: [color[0], color[1], color[2], 1.],
            texture_coordinates: [0.; 2],
            lightmap_coordinates: [0.; 2],
        }
    }

    /// Two triangles of an 8×8 square at `origin`, all corners `color`.
    fn square(origin: [f32; 3], color: [f32; 3]) -> Vec<crate::GpuVertex> {
        let [x, y, z] = origin;
        let corner = |dx, dy| vertex([x + dx, y + dy, z], color);
        vec![
            corner(0., 0.),
            corner(8., 0.),
            corner(8., 8.),
            corner(0., 0.),
            corner(8., 8.),
            corner(0., 8.),
        ]
    }

    fn lamps(vertices: &[crate::GpuVertex], radiance: Vec3) -> Vec<Lamp> {
        clusters(
            radiance,
            vertices.chunks_exact(3).map(|t| [&t[0], &t[1], &t[2]]),
        )
    }

    #[test]
    fn a_red_flare_lights_red_in_proportion_to_its_area() {
        let radiance = Vec3::new(14.0, 0.0, 0.0);
        let found = lamps(&square([0., 0., 0.], [1.0, 1.0, 1.0]), radiance);
        assert_eq!(found.len(), 1);
        let lamp = found[0];
        // 64 square units of red radiance 14.
        let expected = 0.2126 * 14.0 * 64.0 * POWER_SCALE;
        assert!(
            (lamp.power - expected).abs() < 1e-3 * expected,
            "{}",
            lamp.power
        );
        assert!(lamp.color[0] > 0. && lamp.color[1] == 0. && lamp.color[2] == 0.);
        // In front of the face, at its centre.
        assert!((lamp.position - Vec3::new(4., 4., LIFT)).length() < 1e-3);
        assert_eq!(lamp.normal, Vec3::ZERO, "a small glow lights every way");
    }

    #[test]
    fn the_vertex_colour_tints_and_dims_the_light() {
        let radiance = Vec3::splat(10.0);
        let white = lamps(&square([0., 0., 0.], [1.0; 3]), radiance)[0];
        let dim_blue = lamps(&square([0., 0., 0.], [0.0, 0.0, 0.5]), radiance)[0];
        assert!(dim_blue.power < white.power * 0.1);
        assert!(dim_blue.color[2] > 0. && dim_blue.color[0] == 0.);
    }

    #[test]
    fn distant_flares_stay_separate_and_near_ones_merge() {
        let radiance = Vec3::splat(10.0);
        let mut vertices = square([0., 0., 0.], [1.0; 3]);
        vertices.extend(square([16., 0., 0.], [1.0; 3]));
        assert_eq!(lamps(&vertices, radiance).len(), 1);
        vertices.extend(square([1000., 0., 0.], [1.0; 3]));
        assert_eq!(lamps(&vertices, radiance).len(), 2);
    }

    #[test]
    fn black_vertices_and_degenerate_triangles_light_nothing() {
        let radiance = Vec3::splat(10.0);
        assert!(lamps(&square([0., 0., 0.], [0.0; 3]), radiance).is_empty());
        let point = vertex([0.; 3], [1.0; 3]);
        assert!(lamps(&[point, point, point], radiance).is_empty());
    }
}
