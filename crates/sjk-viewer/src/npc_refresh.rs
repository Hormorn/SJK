//! In-match NPC bodies and unarmed player display copies — the viewer half of `CG_G2AnimEntModelLoad`
//! (`codemp/cgame/cg_players.c:7046-7308`, run from `CG_G2Animated` when an
//! `ET_NPC` entity has no Ghoul2 instance yet or its `modelindex` changed,
//! `:7502-7589`).
//!
//! The client adapter resolves an NPC's appearance from `CS_MODELS`
//! (`sjk_client::legacy_npc_appearance`) and stores it on the scene entity;
//! this module notices an actor whose appearance has no mesh keyed by its
//! entity id — a fresh spawn, or an entity number reused for a different
//! NPC — and builds one through the same path the clientinfo refresh uses
//! for a player who changed model. Saber hilts follow `npcSaber1/2`
//! (`:7184-7203`), so an existing mesh only gets its hilt names refreshed.
//!
//! Bounded per frame: one entity scan with no allocation, and at most one
//! NPC model loading at a time, read on a worker ([`npc_loads`]); a wave of
//! spawns loads one NPC after another.

use super::*;
use sjk_client::legacy_npc_saber_names_borrowed;

#[path = "npc_loads.rs"]
pub(crate) mod npc_loads;

/// What one frame's scan decided to do.
enum Work {
    Build(EntityId, Appearance, [Option<String>; 2]),
    Rename(usize, [Option<String>; 2]),
}

/// `true` when the hilt catalog names on a mesh differ from `wanted`
/// (catalog names are lower-cased; `.sab` lookup is case-insensitive).
fn names_differ(current: &[Option<String>; 2], wanted: [Option<&str>; 2]) -> bool {
    current
        .iter()
        .zip(wanted)
        .any(|(current, wanted)| match (current, wanted) {
            (Some(current), Some(wanted)) => !current.eq_ignore_ascii_case(wanted),
            (None, None) => false,
            _ => true,
        })
}

fn owned_lowercase(names: [Option<&str>; 2]) -> [Option<String>; 2] {
    names.map(|name| name.map(str::to_ascii_lowercase))
}

impl GpuState {
    /// Give every NPC actor in the world a mesh of its own appearance.
    pub(crate) fn refresh_npc_actors(&mut self) {
        self.poll_npc_load();
        let Some(work) = self.scan_npc_actors() else {
            return;
        };
        match work {
            Work::Rename(index, names) => {
                self.actor_meshes[index].saber_names = names;
                self.actor_meshes[index].holsters.invalidate();
            }
            Work::Build(entity_id, appearance, names) => {
                if let Err(error) = self.start_npc_load(entity_id, appearance.clone(), names) {
                    self.clientinfo_watch.failed.insert(appearance.clone());
                    log::progress(format_args!(
                        "npc {}: could not load {}/{}: {error}",
                        entity_id.get() - 1,
                        appearance.model,
                        appearance.variant
                    ));
                }
            }
        }
    }

    /// The first NPC actor whose mesh is missing, stale, or misnamed.
    fn scan_npc_actors(&self) -> Option<Work> {
        let (world, game_state, snapshot) = match (&self.live_session, &self.demo_session) {
            (Some(session), _) => (
                &self.live_world,
                session.game_state(),
                session.latest_snapshot(),
            ),
            (None, Some(demo)) => (demo.world(), demo.game_state(), demo.latest_snapshot()),
            (None, None) => return None,
        };
        for entity in world
            .entities()
            .filter(|entity| entity.kind == EntityKind::Actor)
        {
            let Some(appearance) = entity.appearance() else {
                continue;
            };
            let Ok(number) = u16::try_from(entity.id.get().saturating_sub(1)) else {
                continue;
            };
            // Snapshot entities are in wire-number order. Also refresh unarmed
            // replicated player display copies, whose clientNum names the
            // real player but whose entity number is outside the player slots.
            let Ok(at) = snapshot
                .entities
                .binary_search_by_key(&number, sjk_protocol::EntityState::number)
            else {
                continue;
            };
            let state = &snapshot.entities[at];
            if !refreshable(number, state.entity_type(), state.weapon()) {
                continue;
            }
            let wanted = if state.entity_type() == 13 {
                legacy_npc_saber_names_borrowed(game_state, state)
            } else {
                [None, None]
            };
            let index = self
                .actor_meshes
                .iter()
                .position(|mesh| !mesh.corpse_pool && mesh.entity_id == Some(entity.id));
            match index {
                Some(index) if self.actor_meshes[index].appearance == *appearance => {
                    if names_differ(&self.actor_meshes[index].saber_names, wanted) {
                        return Some(Work::Rename(index, owned_lowercase(wanted)));
                    }
                }
                _ if self.clientinfo_watch.failed.contains(appearance) => {}
                // One NPC model loads at a time; the next waits, the scan allocating nothing.
                _ if !self.clientinfo_watch.npc_loads.idle() => {}
                _ => {
                    return Some(Work::Build(
                        entity.id,
                        appearance.clone(),
                        owned_lowercase(wanted),
                    ));
                }
            }
        }
        None
    }
}

/// Ordinary player and corpse refresh keep their own lifetime rules. A static,
/// unarmed display copy needs independent pose storage and live model changes.
fn refreshable(number: u16, kind: u8, weapon: u8) -> bool {
    kind == 13 || (kind == 1 && number >= 32 && weapon == 0)
}

/// `true` while an NPC (or unarmed display copy) waits for its own model to load:
/// it is drawn as nothing then, not as the shared Kyle stand-in. Its model loads on
/// a worker over several frames (`npc_loads.rs`), so each fresh spawn (a tauntaun
/// that respawns, an admin's NPC) used to flash as an unarmed Kyle first. One whose
/// model failed to load keeps the stand-in, as before.
pub(crate) fn awaits_own_mesh(
    state: &sjk_protocol::EntityState,
    appearance: Option<&Appearance>,
    failed: &BTreeSet<Appearance>,
) -> bool {
    refreshable(state.number(), state.entity_type(), state.weapon())
        && appearance.is_some_and(|appearance| !failed.contains(appearance))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS};

    fn state(number: u16, kind: u32, weapon: u32) -> EntityState {
        let mut state = EntityState::zero(number, &LEGACY_ENTITY_FIELDS);
        state.set_raw_field(8, kind);
        state.set_raw_field(14, weapon);
        state
    }

    fn look(model: &str) -> Appearance {
        Appearance {
            model: model.to_owned(),
            variant: String::new(),
        }
    }

    #[test]
    fn a_loading_npc_is_hidden_rather_than_drawn_as_kyle() {
        let none = BTreeSet::new();
        let tauntaun = look("$tauntaun");
        // A fresh tauntaun (Lex, 11/11/2026: an unarmed Kyle flashed before each one).
        assert!(awaits_own_mesh(&state(80, 13, 0), Some(&tauntaun), &none));
        // An unarmed display copy of a player waits the same way.
        assert!(awaits_own_mesh(&state(40, 1, 0), Some(&look("jan")), &none));
        // A model that failed to load keeps the stand-in.
        let failed = BTreeSet::from([tauntaun.clone()]);
        assert!(!awaits_own_mesh(
            &state(80, 13, 0),
            Some(&tauntaun),
            &failed
        ));
        // No appearance known: the stand-in, as before.
        assert!(!awaits_own_mesh(&state(80, 13, 0), None, &none));
        // Players keep their own rules.
        assert!(!awaits_own_mesh(&state(3, 1, 0), Some(&look("jan")), &none));
        assert!(!awaits_own_mesh(
            &state(40, 1, 2),
            Some(&look("jan")),
            &none
        ));
    }
}
