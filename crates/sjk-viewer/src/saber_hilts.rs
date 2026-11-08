//! Load-time saber hilt meshes and their numbered blade sockets.

use crate::{Appearance, FlattenedScene, StaticModelMesh, append_static_glm_mesh};
use sjk_model::Glm;
use sjk_vfs::VirtualFileSystem;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use crate::saber::{BladeSocket, blade_socket_from_bolt};

const DEFAULT_HILT_MODEL: &str = "models/weapons2/saber_1/saber_1.glm";
const DEFAULT_BLADE_LENGTH: f32 = 40.0;
const DEFAULT_BLADE_RADIUS: f32 = 3.0;

/// Saber carriers one hilt mesh is sized for: every client slot plus as many
/// saber NPCs (`LEGACY_MAX_CLIENTS` each way), not every entity number.
const HILT_ACTORS: usize = 2 * sjk_protocol::LEGACY_MAX_CLIENTS;

/// Two hilts per saber carrier, reserved at load rather than on a frame (about
/// 16 KB per hilt mesh). Held and thrown hilts may still grow it; only stowed
/// hilts past this many are skipped rather than growing it in the frame loop.
pub(crate) fn instance_group() -> Vec<crate::actor_instance::ActorInstance> {
    Vec::with_capacity(2 * HILT_ACTORS)
}

/// One numbered blade socket and its `.sab` presentation parameters.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HiltBlade {
    pub(crate) socket: BladeSocket,
    pub(crate) length: f32,
    pub(crate) radius: f32,
    pub(crate) trail_style: u8,
}

/// Loaded hilt mesh and all valid `*blade1` through `*blade8` sockets.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Hilt {
    pub(crate) mesh_index: usize,
    blades: [Option<HiltBlade>; 8],
    pub(crate) num_blades: u8,
    pub(crate) is_staff: bool,
    /// Suppress scene lights for this hilt without suppressing its visible blade.
    pub(crate) no_dlight: bool,
    /// Authored contact-effect suppression.
    pub(crate) no_wall_marks: bool,
    /// The `.sab` `saberColor` an NPC's blade wears without an override.
    pub(crate) default_color: sjk_client::LegacySaberColor,
}

impl Hilt {
    pub(crate) fn blade(self, index: usize) -> Option<HiltBlade> {
        (index < usize::from(self.num_blades))
            .then(|| self.blades[index])
            .flatten()
    }

    /// Whether blade `index` is this hilt's other half ([`other_half`]).
    pub(crate) fn is_other_half(self, index: usize) -> bool {
        other_half(&self.blades, self.num_blades, index)
    }

    /// Whether this hilt has another half ([`has_other_half`]): a staff's second blade.
    pub(crate) fn has_other_half(self) -> bool {
        has_other_half(&self.blades, self.num_blades)
    }
}

/// Whether blade `index` of the first `used` of `blades` is a staff's other half: it points
/// away from the first blade (more than 120 degrees from it), as a staff's second blade
/// does. A crossguard's side blades stand across the first and belong to its half, so a
/// one-handed saber with three blades is no staff (review of 22/10/2026: counting blades
/// took it for one).
pub(crate) fn other_half(blades: &[Option<HiltBlade>; 8], used: u8, index: usize) -> bool {
    let used = usize::from(used).min(blades.len());
    let Some(first) = blades[..used].iter().flatten().next() else {
        return false;
    };
    let Some(blade) = blades[..used].get(index).copied().flatten() else {
        return false;
    };
    let along =
        |blade: &HiltBlade| glam::Vec3::from_array(blade.socket.direction).normalize_or_zero();
    along(first).dot(along(&blade)) < -0.5
}

/// Whether any of the first `used` of `blades` is a staff's other half ([`other_half`]).
pub(crate) fn has_other_half(blades: &[Option<HiltBlade>; 8], used: u8) -> bool {
    (0..usize::from(used)).any(|index| other_half(blades, used, index))
}

/// Hilts required by current clientinfo, with `single_1` as a safe fallback.
pub(crate) struct HiltCatalog {
    hilts: BTreeMap<String, Hilt>,
    fallback: Hilt,
    definitions: BTreeMap<String, crate::saber_defs::Definition>,
    /// Hilt models already in the object buffers, by model path.
    model_cache: BTreeMap<String, CachedModel>,
    /// Names whose load failed; not retried.
    failed: BTreeSet<String>,
}

impl HiltCatalog {
    /// Recognize shared hilt meshes by catalog ownership, never by asset path.
    pub(crate) fn uses_mesh(&self, index: usize) -> bool {
        self.model_cache.values().any(|(mesh, _)| *mesh == index)
    }
    /// Animation-event overrides from the same catalog as the visible hilt.
    pub(crate) fn animation_sound(&self, name: &str, path: &str, variant: usize) -> Option<&str> {
        let definition = self.definitions.get(name)?;
        if path.starts_with("sound/weapons/saber/saberspin") {
            definition.sound_spin.as_deref()
        } else if path.starts_with("sound/weapons/saber/saberhup")
            && definition.sound_swing[0].is_some()
        {
            definition.sound_swing[variant % 3].as_deref()
        } else {
            None
        }
    }

    pub(crate) fn get(&self, name: &str) -> Hilt {
        self.hilts.get(name).copied().unwrap_or(self.fallback)
    }

    /// [`Self::get`] for a name as the server wrote it (an NPC's `npcSaber1`
    /// keeps its `.sab` case), lower-cased on the stack without allocating.
    pub(crate) fn get_any_case(&self, name: &str) -> Hilt {
        let mut buffer = [0u8; 64];
        if !name.bytes().any(|byte| byte.is_ascii_uppercase()) || name.len() > buffer.len() {
            return self.get(name);
        }
        let lower = &mut buffer[..name.len()];
        lower.copy_from_slice(name.as_bytes());
        lower.make_ascii_lowercase();
        std::str::from_utf8(lower).map_or(self.fallback, |lower| self.get(lower))
    }

    /// Whether `name` is loaded, unknown to the `.sab` files, or failed —
    /// in every case [`Self::get`] has an answer without loading anything.
    /// Whether the saber `name` is a staff (or any one hilt with more blades), as its
    /// definition says; an unknown saber is not.
    pub(crate) fn is_staff(&self, name: &str) -> bool {
        self.definitions
            .get(name)
            .or_else(|| self.definitions.get(&name.to_ascii_lowercase()))
            .is_some_and(|definition| definition.is_staff)
    }

    pub(crate) fn is_resolved(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("none")
            || self.hilts.contains_key(name)
            || self.failed.contains(name)
            || !self.definitions.contains_key(name)
    }

    /// Load `name` (a `.sab` saber a player picked up mid-match) into `scene`
    /// and `object_meshes`. Returns whether a mesh was appended; the caller
    /// then uploads `scene` and registers the new object mesh.
    pub(crate) fn load(
        &mut self,
        vfs: &VirtualFileSystem,
        name: &str,
        scene: &mut FlattenedScene,
        object_meshes: &mut Vec<StaticModelMesh>,
    ) -> bool {
        if self.is_resolved(name) {
            return false;
        }
        let Some(definition) = self.definitions.get(name).cloned() else {
            return false;
        };
        let before = object_meshes.len();
        match append_hilt(
            vfs,
            &definition,
            scene,
            object_meshes,
            &mut self.model_cache,
        ) {
            Ok(hilt) => {
                self.hilts.insert(name.to_owned(), hilt);
            }
            Err(error) => {
                eprintln!("could not load saber definition {name}: {error}");
                self.failed.insert(name.to_owned());
            }
        }
        object_meshes.len() > before
    }
}

/// Append all client-advertised saber models to the shared object buffers.
pub(crate) fn load_hilts<'a>(
    vfs: &VirtualFileSystem,
    names: impl Iterator<Item = &'a str>,
    scene: &mut FlattenedScene,
    object_meshes: &mut Vec<StaticModelMesh>,
) -> Result<HiltCatalog, Box<dyn Error>> {
    let definitions = crate::saber_defs::load(vfs)?;
    let default = definitions
        .get("single_1")
        .cloned()
        .unwrap_or_else(default_definition);
    let mut required = names
        .filter(|name| !name.eq_ignore_ascii_case("none"))
        .map(str::to_ascii_lowercase)
        .collect::<BTreeSet<_>>();
    required.insert("single_1".to_owned());
    let mut model_cache = BTreeMap::new();
    let fallback = append_hilt(vfs, &default, scene, object_meshes, &mut model_cache)?;
    let mut hilts = BTreeMap::new();
    hilts.insert("single_1".to_owned(), fallback);
    for name in required {
        let Some(definition) = definitions.get(&name) else {
            continue;
        };
        match append_hilt(vfs, definition, scene, object_meshes, &mut model_cache) {
            Ok(hilt) => {
                hilts.insert(name, hilt);
            }
            Err(error) => eprintln!("could not load saber definition {name}: {error}"),
        }
    }
    Ok(HiltCatalog {
        hilts,
        fallback,
        definitions,
        model_cache,
        failed: BTreeSet::new(),
    })
}

fn default_definition() -> crate::saber_defs::Definition {
    crate::saber_defs::Definition {
        sound_on: None,
        sound_off: None,
        name: "single_1".to_owned(),
        sound_spin: None,
        sound_swing: [None, None, None],
        model: DEFAULT_HILT_MODEL.to_owned(),
        num_blades: 1,
        is_staff: false,
        blade_lengths: [DEFAULT_BLADE_LENGTH; 8],
        blade_radii: [DEFAULT_BLADE_RADIUS; 8],
        blade_style2_start: 0,
        trail_style: 0,
        trail_style2: 0,
        no_dlight: false,
        no_wall_marks: false,
        default_color: sjk_client::LegacySaberColor::Fixed(sjk_client::SaberColor::Red),
    }
}

type CachedModel = (usize, [Option<BladeSocket>; 8]);

fn append_hilt(
    vfs: &VirtualFileSystem,
    definition: &crate::saber_defs::Definition,
    scene: &mut FlattenedScene,
    object_meshes: &mut Vec<StaticModelMesh>,
    model_cache: &mut BTreeMap<String, CachedModel>,
) -> Result<Hilt, Box<dyn Error>> {
    let (mesh_index, sockets) = if let Some(cached) = model_cache.get(&definition.model) {
        *cached
    } else {
        load_hilt_model(vfs, definition, scene, object_meshes, model_cache)?
    };
    Ok(Hilt {
        mesh_index,
        blades: hilt_blades(definition, &sockets),
        num_blades: definition.num_blades,
        is_staff: definition.is_staff,
        no_dlight: definition.no_dlight,
        no_wall_marks: definition.no_wall_marks,
        default_color: definition.default_color,
    })
}

/// Pair every socket the model has with the `.sab` blade parameters.
pub(crate) fn hilt_blades(
    definition: &crate::saber_defs::Definition,
    sockets: &[Option<BladeSocket>; 8],
) -> [Option<HiltBlade>; 8] {
    std::array::from_fn(|index| {
        let socket = sockets[index]?;
        let secondary = definition.blade_style2_start > 0
            && index >= usize::from(definition.blade_style2_start);
        Some(HiltBlade {
            socket,
            length: definition.blade_lengths[index],
            radius: definition.blade_radii[index],
            trail_style: if secondary {
                definition.trail_style2
            } else {
                definition.trail_style
            },
        })
    })
}

/// Read the `*blade1`..`*blade8` sockets of a hilt model in its bind pose.
/// `CG_InitG2SaberData` adds the numbered tags in order and only blade 1
/// falls back to `*flash` (`codemp/cgame/cg_players.c:1212-1237`).
pub(crate) fn hilt_sockets(model: &Glm) -> Result<[Option<BladeSocket>; 8], Box<dyn Error>> {
    let identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let pose = vec![identity; model.bone_count];
    let sockets = std::array::from_fn(|index| {
        let name = format!("*blade{}", index + 1);
        model
            .surface_bolt_matrix(&name, 0, &pose)
            .ok()
            .flatten()
            .or_else(|| {
                (index == 0)
                    .then(|| model.surface_bolt_matrix("*flash", 0, &pose).ok().flatten())
                    .flatten()
            })
            .and_then(blade_socket_from_bolt)
    });
    if sockets[0].is_none() {
        return Err("saber hilt has neither *blade1 nor *flash".into());
    }
    Ok(sockets)
}

fn load_hilt_model(
    vfs: &VirtualFileSystem,
    definition: &crate::saber_defs::Definition,
    scene: &mut FlattenedScene,
    object_meshes: &mut Vec<StaticModelMesh>,
    model_cache: &mut BTreeMap<String, CachedModel>,
) -> Result<CachedModel, Box<dyn Error>> {
    let asset = vfs
        .read(&definition.model)?
        .ok_or_else(|| format!("hilt model {} is missing", definition.model))?;
    let model = Glm::parse(&asset.bytes)?;
    let sockets = hilt_sockets(&model)?;
    let mesh_index = object_meshes.len();
    let draws = append_static_glm_mesh(scene, &model)?;
    object_meshes.push(StaticModelMesh {
        appearance: Appearance {
            model: definition.model.clone(),
            variant: String::new(),
        },
        draws,
        center: [0.0; 3],
        flash_bolt: None,
        turning_bone: None,
        eweb_part: None,
    });
    let cached = (mesh_index, sockets);
    model_cache.insert(definition.model.clone(), cached);
    Ok(cached)
}

#[cfg(test)]
mod capacity_tests {
    use super::*;

    #[test]
    fn dual_holstered_hilts_fit_every_carrier_without_growing_the_group() {
        let mut group = instance_group();
        let storage = group.as_ptr();
        for _ in 0..HILT_ACTORS {
            for _ in 0..2 {
                assert!(group.len() < group.capacity());
                group.push(crate::actor_instance::ActorInstance::new(
                    [0.; 3],
                    [0., 0., 0., 1.],
                    [1.; 3],
                ));
            }
        }
        assert_eq!(group.as_ptr(), storage);
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    fn pointing(direction: [f32; 3]) -> Option<HiltBlade> {
        Some(HiltBlade {
            socket: BladeSocket {
                origin: [0.0; 3],
                direction,
            },
            length: 40.0,
            radius: 3.0,
            trail_style: 0,
        })
    }

    #[test]
    fn a_staff_has_another_half_and_a_crossguard_has_none() {
        let mut staff = [None; 8];
        staff[0] = pointing([0.0, 0.0, 1.0]);
        staff[1] = pointing([0.0, 0.0, -1.0]);
        assert!(!other_half(&staff, 2, 0));
        assert!(other_half(&staff, 2, 1));
        assert!(has_other_half(&staff, 2));
        // A main blade and two short ones across it at the guard.
        let mut crossguard = [None; 8];
        crossguard[0] = pointing([0.0, 0.0, 1.0]);
        crossguard[1] = pointing([1.0, 0.0, 0.0]);
        crossguard[2] = pointing([-1.0, 0.0, 0.1]);
        assert!(!has_other_half(&crossguard, 3));
        // One blade, or a blade past the ones in use.
        assert!(!has_other_half(&staff, 1));
        assert!(!other_half(&staff, 1, 1));
        // The hilt says so too, for the player card.
        let held = |blades, num_blades| Hilt {
            blades,
            num_blades,
            ..hilt(0, sjk_client::LegacySaberColor::Random)
        };
        assert!(held(staff, 2).has_other_half());
        assert!(!held(crossguard, 3).has_other_half());
    }

    fn hilt(mesh_index: usize, default_color: sjk_client::LegacySaberColor) -> Hilt {
        Hilt {
            mesh_index,
            blades: [None; 8],
            num_blades: 1,
            is_staff: false,
            no_dlight: false,
            no_wall_marks: false,
            default_color,
        }
    }

    #[test]
    fn an_npc_saber_name_finds_its_hilt_in_any_case() {
        let fallback = hilt(
            0,
            sjk_client::LegacySaberColor::Fixed(sjk_client::SaberColor::Red),
        );
        let desann = hilt(
            7,
            sjk_client::LegacySaberColor::Fixed(sjk_client::SaberColor::Purple),
        );
        let catalog = HiltCatalog {
            hilts: BTreeMap::from([("desann".to_owned(), desann)]),
            fallback,
            definitions: BTreeMap::new(),
            model_cache: BTreeMap::new(),
            failed: BTreeSet::new(),
        };
        for name in ["desann", "Desann", "DESANN"] {
            let found = catalog.get_any_case(name);
            assert_eq!(found.mesh_index, 7, "{name}");
            assert_eq!(found.default_color, desann.default_color);
        }
        assert_eq!(catalog.get_any_case("Kyle").mesh_index, 0);
        assert_eq!(catalog.get_any_case(&"X".repeat(80)).mesh_index, 0);
    }
}
