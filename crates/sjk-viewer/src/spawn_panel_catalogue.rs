//! What the spawn list offers: every NPC of the mounted `ext_data/npcs/*.npc` files and
//! every vehicle of `ext_data/vehicles/*.veh`, by the names `npc spawn <name>` and
//! `npc spawn vehicle <name>` take, plus a short table of cheat commands.
//!
//! The names are read as the server's game reads them: the NPC blocks with
//! [`sjk_game_jka::npc_names::npc_names`] over the joined NPC text, the vehicle blocks
//! with [`VehicleFiles::vehicle_names`]. A name that is also a vehicle's stays on the
//! Vehicles tab only (the server spawns the vehicle for it). The retail game's own come
//! first (Sol's rule, told by the archive: [`sjk_client::retail_view`]), each part in
//! name order without regard to case, and a name listed twice is listed once.
//!
//! The files are read when the panel opens, never per frame.

use sjk_game_jka::npc_names::npc_names;
use sjk_game_jka::vehicle_parms::VehicleFiles;
use sjk_vfs::VirtualFileSystem;
use std::collections::HashSet;

/// One kind of name (NPCs or vehicles): the retail ones first, then the community's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Group {
    pub(super) names: Vec<String>,
    /// How many of `names`, from the start, the retail archives hold.
    pub(super) retail: usize,
}

/// The NPCs and vehicles a spawn can name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Catalogue {
    pub(super) npcs: Group,
    pub(super) vehicles: Group,
}

/// A cheat command of the Cheats tab: what it is called, the line it runs and what it
/// does. Each is a stock JKA command the server takes with cheats on (`devmap`).
#[derive(Clone, Copy, Debug)]
pub(super) struct Cheat {
    pub(super) title: &'static str,
    pub(super) command: &'static str,
    pub(super) about: &'static str,
}

/// The Cheats tab, in its order.
pub(super) const CHEATS: [Cheat; 7] = [
    Cheat {
        title: "Noclip",
        command: "noclip",
        about: "Fly through walls and floors. Run it again to walk.",
    },
    Cheat {
        title: "God mode",
        command: "god",
        about: "Take no damage. Run it again to turn it off.",
    },
    Cheat {
        title: "No target",
        command: "notarget",
        about: "NPCs do not see you. Run it again to turn it off.",
    },
    Cheat {
        title: "Give all",
        command: "give all",
        about: "Every weapon, full ammo, health, armor and items.",
    },
    Cheat {
        title: "Remove all NPCs",
        command: "npc kill all",
        about: "Kill every NPC on the map, vehicles too.",
    },
    Cheat {
        title: "Show NPC bounds",
        command: "npc showbounds",
        about: "Draw the bounding box of every NPC. Run it again to hide them.",
    },
    Cheat {
        title: "Respawn",
        command: "kill",
        about: "Kill yourself to respawn.",
    },
];

impl Catalogue {
    /// Read the names from the mounted files.
    pub(crate) fn scan(vfs: &VirtualFileSystem) -> Self {
        let retail = sjk_client::retail_view(vfs);
        let (npcs, vehicles) = names_in(vfs);
        let (retail_npcs, retail_vehicles) = retail.as_ref().map(names_in).unwrap_or_default();
        Self::from_names(npcs, vehicles, &retail_npcs, &retail_vehicles)
    }

    /// The catalogue of the names read, `retail_*` being those the retail archives hold.
    fn from_names(
        npcs: Vec<String>,
        vehicles: Vec<String>,
        retail_npcs: &[String],
        retail_vehicles: &[String],
    ) -> Self {
        let vehicle_keys: HashSet<String> = vehicles
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect();
        let npcs = npcs
            .into_iter()
            .filter(|name| !vehicle_keys.contains(&name.to_ascii_lowercase()))
            .collect();
        Self {
            npcs: group(npcs, retail_npcs),
            vehicles: group(vehicles, retail_vehicles),
        }
    }
}

/// The NPC and vehicle names of `vfs`'s files, in their order, unchecked.
fn names_in(vfs: &VirtualFileSystem) -> (Vec<String>, Vec<String>) {
    let mut text = Vec::new();
    for name in vfs.list_files("ext_data/npcs", ".npc") {
        if let Ok(Some(asset)) = vfs.read(&format!("ext_data/npcs/{name}")) {
            text.extend_from_slice(&asset.bytes);
            text.push(b'\n');
        }
    }
    let npcs = npc_names(&text);
    let vehicles = VehicleFiles::from_listing(
        |directory, extension| vfs.list_files(directory, extension),
        |path| vfs.read(path).ok().flatten().map(|asset| asset.bytes),
    )
    .vehicle_names();
    (texts(npcs), texts(vehicles))
}

/// The names a command line can carry: printable ASCII with no quote, `;` or space,
/// so a spawn never runs more than the one command.
fn texts(names: Vec<Vec<u8>>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| {
            !name.is_empty()
                && name
                    .iter()
                    .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'"' | b';'))
        })
        .filter_map(|name| String::from_utf8(name).ok())
        .collect()
}

/// `names` once each (any case), the ones `retail` holds first, each part in name order.
fn group(names: Vec<String>, retail: &[String]) -> Group {
    let retail: HashSet<String> = retail
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    let mut seen = HashSet::new();
    let mut names: Vec<String> = names
        .into_iter()
        .filter(|name| seen.insert(name.to_ascii_lowercase()))
        .collect();
    names.sort_by_cached_key(|name| name.to_ascii_lowercase());
    let (mut first, community): (Vec<String>, Vec<String>) = names
        .into_iter()
        .partition(|name| retail.contains(&name.to_ascii_lowercase()));
    let count = first.len();
    first.extend(community);
    Group {
        names: first,
        retail: count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn retail_names_come_first_each_part_sorted_once_each() {
        let catalogue = Catalogue::from_names(
            strings(&["stormtrooper", "Zealot_mod", "alora", "Alora", "jedi_hm1"]),
            strings(&["Swoop", "tie-fighter", "Swoop", "my_speeder"]),
            &strings(&["alora", "stormtrooper", "jedi_hm1"]),
            &strings(&["Swoop", "tie-fighter"]),
        );
        assert_eq!(
            catalogue.npcs.names,
            ["alora", "jedi_hm1", "stormtrooper", "Zealot_mod"]
        );
        assert_eq!(catalogue.npcs.retail, 3);
        assert_eq!(
            catalogue.vehicles.names,
            ["Swoop", "tie-fighter", "my_speeder"]
        );
        assert_eq!(catalogue.vehicles.retail, 2);
    }

    #[test]
    fn a_vehicles_name_stays_off_the_npcs() {
        let catalogue = Catalogue::from_names(
            strings(&["atst", "ATST_vehicle", "Tauntaun"]),
            strings(&["atst_vehicle", "tauntaun"]),
            &[],
            &[],
        );
        assert_eq!(catalogue.npcs.names, ["atst"]);
        assert_eq!(catalogue.npcs.retail, 0);
    }

    #[test]
    fn names_that_would_run_another_command_are_left_out() {
        let names = texts(vec![
            b"reborn".to_vec(),
            b"bad;quit".to_vec(),
            b"two words".to_vec(),
            b"quo\"te".to_vec(),
            Vec::new(),
        ]);
        assert_eq!(names, ["reborn"]);
    }

    #[test]
    fn the_npc_blocks_are_named_as_the_game_parses_them() {
        let text = b"// comment\nstormtrooper\n{\n playerModel stormtrooper\n}\n\
                     reborn { weapon WP_SABER }\n";
        assert_eq!(texts(npc_names(text)), ["stormtrooper", "reborn"]);
    }
}
