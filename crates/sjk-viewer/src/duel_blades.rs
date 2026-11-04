//! Stock blades in duels (Sol's request, 21/10/2026): with `cg_duelStockBlades 1`, while
//! the local (or followed) player is in a private duel, or the server plays Duel or Power
//! Duel, every player's blade is drawn as the stock one in the colour they set (a stock
//! colour or RGB), with the stock sounds, instead of their blade skin, so a duellist reads
//! the swings and nothing round a blade hides where it is
//! ([`crate::saber_skins::SaberSkins::set_plain`]).
//!
//! Off by default (Creyon: the skins stay on in duels unless a player asks). It changes
//! only what this client draws: the other duellist sees what their own setting says.

/// Archived, default 0: 1 draws every blade stock in duels.
pub(crate) const CVAR: &str = "cg_duelStockBlades";
const LOOKUP: &str = "cg_duelstockblades";

/// `g_gametype` values that are duels (`bg_public.h` `GT_DUEL`, `GT_POWERDUEL`).
const DUEL_GAMETYPES: [i32; 2] = [3, 4];

/// Whether blades are drawn stock this frame: the setting is on, and the viewed player is
/// in a private duel or the server's gametype is a duel.
pub(crate) fn plain(
    console: Option<&crate::console::ViewerConsole>,
    game: Option<&sjk_protocol::GameState>,
    player: Option<&sjk_protocol::PlayerState>,
) -> bool {
    let stock = console
        .and_then(|console| console.bool_cvar(LOOKUP))
        .unwrap_or(false);
    stock && in_duel(game, player)
}

/// The viewed player is in a private duel, or the server plays a duel gametype (also
/// what silences the emotes with `s_muteEmotesInDuel`, `crate::emotes::DUEL_MUTE_CVAR`).
pub(crate) fn in_duel(
    game: Option<&sjk_protocol::GameState>,
    player: Option<&sjk_protocol::PlayerState>,
) -> bool {
    let private = player.is_some_and(sjk_protocol::PlayerState::duel_in_progress);
    let gametype = game
        .and_then(|game| game.config_string(0))
        .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"));
    private || gametype.is_some_and(|gametype| DUEL_GAMETYPES.contains(&gametype))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(info: &[u8]) -> sjk_protocol::GameState {
        let mut game = sjk_protocol::GameState::empty_local(0);
        game.replace_config_string(0, info.to_vec()).unwrap();
        game
    }

    #[test]
    fn duels_are_the_duel_gametypes_and_private_duels() {
        let ffa = game(br"\g_gametype\0");
        let mut player = sjk_protocol::PlayerState::default();
        assert!(!in_duel(Some(&ffa), Some(&player)));
        assert!(in_duel(Some(&game(br"\g_gametype\3")), Some(&player)));
        assert!(in_duel(Some(&game(br"\g_gametype\4")), None));
        assert!(!in_duel(Some(&game(br"\g_gametype\6")), None));
        // A private duel: `duelInProgress` (netfield 119).
        player.set_raw_field(119, 1);
        assert!(in_duel(Some(&ffa), Some(&player)));
        assert!(!in_duel(None, None));
        assert_eq!(LOOKUP, CVAR.to_ascii_lowercase());
    }
}
