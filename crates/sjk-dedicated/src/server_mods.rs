//! The server mods this server can run (`sjk_server_mod`), and the one `g_mod` names.
//!
//! `g_mod` is read once, when the server starts, as a JKA server's game module is
//! chosen by its `fs_game` at start: the base game (`base`, the default) or a mod of
//! [`SERVER_MODS`]. A change made later waits for the next start (`CVAR_LATCH`, which
//! says so). The running mod registers its settings, announces itself in the server
//! info and gives the movement rules every player moves by.

use crate::cvars::Cvars;
use sjk_server_mod::{ModCombat, ModMovement, ServerMod, Settings};

/// The setting that names the server mod.
pub const MOD_CVAR: &[u8] = b"g_mod";
/// Its value for the base game.
pub const BASE: &str = "base";

/// Every mod the server offers, one entry each; supporting another is an entry here.
pub const SERVER_MODS: &[&dyn ServerMod] = &[
    // JA+ 2.4 Build 7, from VisionJK's port of its server module (`docs/server.md`).
    &sjk_server_mod_japlus::JaPlus,
];

/// The mod with `id`, any letter case; `None` for the base game or a name no mod has.
pub fn find(id: &[u8]) -> Option<&'static dyn ServerMod> {
    SERVER_MODS
        .iter()
        .copied()
        .find(|server_mod| server_mod.id().as_bytes().eq_ignore_ascii_case(id))
}

/// The mod `g_mod` names now.
pub fn selected(cvars: &Cvars) -> Option<&'static dyn ServerMod> {
    find(cvars.string(MOD_CVAR))
}

/// The movement rules of `server_mod`, the base game's without one.
pub fn movement(server_mod: Option<&dyn ServerMod>, cvars: &Cvars) -> ModMovement {
    server_mod.map_or_else(ModMovement::default, |server_mod| {
        server_mod.movement(cvars)
    })
}

/// The combat rules of `server_mod`, the base game's without one.
pub fn combat(server_mod: Option<&dyn ServerMod>, cvars: &Cvars) -> ModCombat {
    server_mod.map_or_else(ModCombat::default, |server_mod| server_mod.combat(cvars))
}

/// The Force rules of `server_mod`, the base game's without one.
pub fn force(
    server_mod: Option<&dyn ServerMod>,
    cvars: &Cvars,
) -> sjk_game_jka::force_rules::ForceRules {
    server_mod.map_or_else(Default::default, |server_mod| server_mod.force(cvars))
}

/// The private-duel rules of `server_mod`, the base game's without one.
pub fn duels(server_mod: Option<&dyn ServerMod>, cvars: &Cvars) -> sjk_game_jka::duel::DuelRules {
    server_mod.map_or_else(Default::default, |server_mod| server_mod.duels(cvars))
}

/// What `g_mod` may be, for the messages: `base, japlus`.
pub fn choices() -> String {
    std::iter::once(BASE)
        .chain(SERVER_MODS.iter().map(|server_mod| server_mod.id()))
        .collect::<Vec<_>>()
        .join(", ")
}

impl Settings for Cvars {
    fn integer(&self, name: &str) -> i32 {
        Cvars::integer(self, name.as_bytes())
    }
    fn value(&self, name: &str) -> f32 {
        Cvars::value(self, name.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mods_are_found_by_their_id_in_any_case() {
        assert_eq!(find(b"japlus").map(|found| found.name()), Some("JA+"));
        assert_eq!(find(b"JAPlus").map(|found| found.id()), Some("japlus"));
        assert!(find(BASE.as_bytes()).is_none());
        assert!(find(b"").is_none());
        assert!(find(b"lugormod").is_none());
        assert_eq!(choices(), "base, japlus");
    }

    #[test]
    fn every_id_is_unique_and_a_plain_word() {
        for (index, server_mod) in SERVER_MODS.iter().enumerate() {
            let id = server_mod.id();
            assert!(
                !id.is_empty()
                    && id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
                "{id}"
            );
            assert_ne!(id, BASE);
            assert!(SERVER_MODS[..index].iter().all(|other| other.id() != id));
        }
    }
}
