//! Private-duel pass-through in movement prediction on JA+ and jaPRO/TaystJK.
//!
//! Stock codemp keeps duellers solid to everyone. JA+ and jaPRO let the two
//! duellers and everyone else pass through each other, and leave part of that to
//! the client: a JA+ 2.4 server sends a dueller to a client-plugin bystander as a
//! solid player box (with `bolt1`, the duel flag), so the plugin client must skip
//! it in prediction, while a client without the plugin gets the dueller with
//! `solid 0` and bystanders are not sent to duellers at all. jaPRO's server does
//! the same through `BeginHack` (`codemp/game/g_syscalls.c` in EternalJK's tree),
//! and EternalJK's prediction mirrors both mods in `CG_ClipMoveToEntities`
//! (`cgame/cg_predict.c:267-280`).
//!
//! A JA+ server isolates its duels only while `jp_DuelAlpha` is 0 or more, only in a
//! free-for-all or team deathmatch, and only between players: VisionJK's rebuild of its
//! server module (`japlus/g_japlus_dim.c:130-164`, `G_TempUnlink`), whose port of the
//! client plugin predicts from `jp_DuelAlpha` too (`cgame/cg_predict.c:465-497`). That
//! is what SJK's own server runs with `g_mod japlus`
//! ([`sjk_game_jka::duel::isolated`], the rule both use).

use crate::CompatProfile;
use sjk_protocol::{EntityState, GameState, InfoString, PlayerState};

const ET_PLAYER: u8 = 1;
const ET_NPC: u8 = 13;
/// `MAX_CLIENTS`: only players are left out on a JA+ server.
const MAX_CLIENTS: u16 = 32;

/// Which server's duel isolation the movement prediction follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuelIsolation {
    /// JA+ (`jp_DuelAlpha` 0 or more, a free-for-all or team deathmatch): duellers and
    /// the other players pass through each other.
    JaPlus,
    /// jaPRO/TaystJK: a dueller passes every player and NPC but its opponent.
    JaPro,
}

impl DuelIsolation {
    /// The isolation `CS_SERVERINFO` announces, if any.
    pub fn from_game_state(game_state: &GameState) -> Option<Self> {
        let info = game_state
            .config_string(0)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|text| InfoString::parse(text).ok())?;
        Self::from_server_info(&info)
    }

    /// [`Self::from_game_state`] from a parsed server info.
    pub fn from_server_info(info: &InfoString) -> Option<Self> {
        let number = |key: &str| {
            info.get(key)
                .map_or(0, |value| crate::team_info::legacy_atoi(value.as_bytes()))
        };
        match CompatProfile::from_server_info(info) {
            CompatProfile::JaPlus { .. } => {
                // `cgs.jpDuelAlpha`, `atoi` of the key (0 without it), and the server's
                // `JP_NonInterferenceRules`: `GT_FFA` or `GT_TEAM`.
                let gametype = number("g_gametype");
                (number("jp_DuelAlpha") >= 0 && (gametype == 0 || gametype == 6))
                    .then_some(Self::JaPlus)
            }
            CompatProfile::TaystJk => Some(Self::JaPro),
            _ => None,
        }
    }
}

/// Whether movement for `local` passes through `entity` because of a private
/// duel, on a server with `isolation`.
///
/// On jaPRO a dueller passes through every player and NPC but its opponent, as jaPRO's
/// server makes them non-solid for it (EternalJK's client skips every non-mover
/// instead; only players and NPCs are made non-solid by the server). On JA+ only the
/// players are left out (`G_TempUnlink` walks the clients). Anyone else passes through
/// duelling players.
pub fn duel_passes_through(
    isolation: DuelIsolation,
    local: &PlayerState,
    entity: &EntityState,
) -> bool {
    let kind = entity.entity_type();
    if local.duel_in_progress() {
        match isolation {
            DuelIsolation::JaPro => {
                matches!(kind, ET_PLAYER | ET_NPC) && entity.number() != local.duel_index()
            }
            // A player entity's bolt1 is its duelInProgress (BG_PlayerStateToEntityState);
            // whom it duels the client is not told, so its own opponent stands for it, as
            // the plugin has it (`ps->duelIndex != i`).
            DuelIsolation::JaPlus => {
                kind == ET_PLAYER
                    && entity.number() < MAX_CLIENTS
                    && sjk_game_jka::duel::isolated(
                        Some(local.duel_index()),
                        entity.number(),
                        entity.bolt1(),
                        true,
                    )
            }
        }
    } else {
        // A player entity's bolt1 is its duelInProgress (BG_PlayerStateToEntityState).
        kind == ET_PLAYER && entity.bolt1()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::LEGACY_ENTITY_FIELDS;

    fn entity(number: u16, kind: u8, bolt1: bool) -> EntityState {
        let mut state = EntityState::zero(number, &LEGACY_ENTITY_FIELDS);
        state.set_raw_field(8, u32::from(kind));
        state.set_raw_field(91, u32::from(bolt1));
        state
    }

    fn player(client: u16, duel: Option<u16>) -> PlayerState {
        let mut state = PlayerState::default();
        state.set_client_num(client);
        if let Some(opponent) = duel {
            state.set_raw_field(119, 1);
            state.set_raw_field(44, u32::from(opponent));
        }
        state
    }

    fn isolation(info: &str) -> Option<DuelIsolation> {
        DuelIsolation::from_server_info(&InfoString::parse(info).unwrap())
    }

    #[test]
    fn a_japro_dueller_passes_through_everyone_but_the_opponent() {
        let local = player(0, Some(1));
        let passes =
            |entity: &EntityState| duel_passes_through(DuelIsolation::JaPro, &local, entity);
        assert!(!passes(&entity(1, ET_PLAYER, true)));
        assert!(passes(&entity(2, ET_PLAYER, false)));
        assert!(passes(&entity(40, ET_NPC, false)));
        // Movers and other solids keep blocking.
        assert!(!passes(&entity(60, 6, false)));
        assert!(!passes(&entity(61, 0, false)));
    }

    #[test]
    fn a_ja_plus_dueller_passes_through_the_other_players_only() {
        let local = player(0, Some(1));
        let passes =
            |entity: &EntityState| duel_passes_through(DuelIsolation::JaPlus, &local, entity);
        assert!(!passes(&entity(1, ET_PLAYER, true)));
        // The opponent, no longer duelling: the server leaves it out too.
        assert!(passes(&entity(1, ET_PLAYER, false)));
        assert!(passes(&entity(2, ET_PLAYER, false)));
        assert!(passes(&entity(3, ET_PLAYER, true)));
        // NPCs and movers keep blocking: `G_TempUnlink` walks the clients.
        assert!(!passes(&entity(40, ET_NPC, false)));
        assert!(!passes(&entity(60, 6, false)));
    }

    #[test]
    fn a_bystander_passes_through_duellers_only() {
        for rule in [DuelIsolation::JaPlus, DuelIsolation::JaPro] {
            let local = player(0, None);
            assert!(duel_passes_through(
                rule,
                &local,
                &entity(1, ET_PLAYER, true)
            ));
            assert!(!duel_passes_through(
                rule,
                &local,
                &entity(2, ET_PLAYER, false)
            ));
            // bolt1 means something else on other entity types.
            assert!(!duel_passes_through(rule, &local, &entity(60, 6, true)));
        }
    }

    #[test]
    fn ja_plus_isolates_with_jp_duel_alpha_in_ffa_and_team_games() {
        let ja_plus = r"\gamename\JA+ Mod v2.4 B7\jp_cinfo\19";
        assert_eq!(
            isolation(&format!(r"{ja_plus}\g_gametype\0\jp_DuelAlpha\100")),
            Some(DuelIsolation::JaPlus)
        );
        assert_eq!(
            isolation(&format!(r"{ja_plus}\g_gametype\6\jp_DuelAlpha\0")),
            Some(DuelIsolation::JaPlus)
        );
        // `atoi` of a key the server does not send is 0.
        assert_eq!(isolation(ja_plus), Some(DuelIsolation::JaPlus));
        assert_eq!(isolation(&format!(r"{ja_plus}\jp_DuelAlpha\-1")), None);
        // Holocron, Jedi Master, CTF: the server does not isolate there.
        for gametype in [1, 2, 7, 8] {
            assert_eq!(
                isolation(&format!(r"{ja_plus}\g_gametype\{gametype}")),
                None
            );
        }
    }

    #[test]
    fn the_base_game_and_unknown_servers_do_not_isolate() {
        assert_eq!(isolation(r"\gamename\basejka\g_gametype\0"), None);
        assert_eq!(isolation(r"\gamename\something\g_gametype\0"), None);
    }
}
