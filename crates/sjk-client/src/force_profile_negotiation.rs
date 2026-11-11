//! Keep the player's Force profile legal for the current server.
//!
//! The user's preferred `forcepowers` is a client setting; what goes on the
//! wire is a per-server legalized copy. `WP_InitForcePowers`
//! (`codemp/game/w_force.c:240-251`) strips disabled or over-budget powers,
//! and (`w_force.c:368-392`) parks any client whose profile needed stripping
//! — or whose session has not yet confirmed its powers (`!sess.setForce`) —
//! in spectator mode with `spc` + `nfr <rank> 1 <team>`: the stock "Force menu
//! on first join". [`enter_play`] sends the legalized value and then the
//! stock Force menu's own accept command, `forcechanged "<TEAM>"`, while the
//! client is still the spectator it connects as; `Cmd_ForceChanged_f`
//! (`codemp/game/g_cmds.c:1276-1313`) runs `WP_InitForcePowers` immediately,
//! marks the session and joins the team, so play starts without a bounce
//! (verified against JA+ 2.4 on the EFF server,
//! `target/parity-reports/playtest-japlus5`).
//!
//! When a server still sends `nfr <rank> 1 <team>`, stock JKA opens the Force
//! menu; closing it flushes the legalized userinfo, sends `forcechanged`, and
//! the player clicks Join again (`assets1.pk3:ui/jamp/ingame_playerforce.menu:
//! 1556-1562`). [`ForceProfileNegotiator`] performs those steps itself. Because
//! `Cmd_Team_f` refuses a second team change within five seconds of the first
//! (`codemp/game/g_cmds.c:998-1001`, `1031`), the join is retried a bounded
//! number of times, except in duel and power duel, where waiting as a
//! spectator is the queue: there `SetTeam` keeps a queued player spectating
//! but announces and respawns them on every request (`g_cmds.c:829-841`,
//! `906`, `922`), and `Cmd_Team_f` refuses any change in power duel
//! (`g_cmds.c:1016-1022`).
//!
//! The value sent is always the player's own `forcepowers` fitted to the
//! server's current rules ([`ForceLegalizeRules::for_sent_profile`]), worked
//! out each time the userinfo is built, so a profile applied in play is the
//! one the server receives. A server reads a changed profile only when told
//! to: `Cmd_ForceChanged_f` re-reads it at once for a spectator and at the
//! next respawn in play (`forceDoInit`, `g_cmds.c:1276-1296`,
//! `g_client.c:3174-3178`), so an applied profile is followed by
//! `forcechanged` ([`ForceProfileNegotiator::profile_applied`]), as stock
//! `UI_UpdateClientForcePowers` does (`codemp/ui/ui_force.c`).

use crate::force_profile::{ForceLegalizeRules, legalize_force_powers};
use crate::force_rank_reply::{
    ForceRankReply, force_rank_reply, force_rules_from_serverinfo, server_force_rules,
    serverinfo_shows_free_saber,
};
use crate::team_commands::{LegacyTeamChoice, legacy_team_command};
use crate::{ClientError, ClientSession, UserinfoUpdateStatus};
use sjk_network::LegacyUserInfo;
use sjk_protocol::{GameState, InfoString};
use std::time::{Duration, Instant};

/// `switchTeamTime` window (`g_cmds.c:1031`) plus transport slack.
const REJOIN_INTERVAL: Duration = Duration::from_millis(5_500);
/// Bounded so a server that keeps refusing never produces a join/spectate loop.
const REJOIN_ATTEMPTS: u8 = 3;

/// Fit `preferred` to the rules the server advertises in `CS_SERVERINFO`,
/// using its `g_maxForceRank` as the rank ceiling, as it is sent
/// ([`ForceLegalizeRules::for_sent_profile`]: disabled and team powers are kept).
///
/// When `CS_SERVERINFO` lacks the weapon mask `HasSetSaberOnly` reads, the
/// profile is not cut by a guess ([`fit_sent`]): stock sends
/// `forcepowers` as the player last committed it (a `CVAR_USERINFO` cvar,
/// `codemp/client/cl_main.cpp:3894`, written only by the Force menu's
/// `UI_UpdateClientForcePowers`, `ui_force.c:192-216`) and the server fits it
/// itself (`w_force.c:240-247`).
pub fn server_legal_forcepowers(game_state: &GameState, preferred: &str) -> String {
    let rules = server_force_rules(game_state).for_sent_profile();
    fit_sent(preferred, rules, serverinfo_shows_free_saber(game_state))
}

/// Fit `preferred` to the sent-profile `rules`. While whether saber skills
/// come free is not `known`, a profile that only fits with free saber skills
/// is fitted with them, so it is not cut by a guess: the server, which knows,
/// cuts it if it must, as it would stock's unfitted profile.
fn fit_sent(preferred: &str, rules: ForceLegalizeRules, known: bool) -> String {
    let fitted = legalize_force_powers(preferred, rules);
    if known || rules.free_saber || fitted.was_legal {
        return fitted.allocation.encode();
    }
    let free = ForceLegalizeRules {
        free_saber: true,
        ..rules
    };
    legalize_force_powers(preferred, free).allocation.encode()
}

/// `gametype_t` values (`bg_public.h:235-244`) for which `Cmd_ForceChanged_f`
/// ignores its team argument (`g_cmds.c:1297-1300`).
const GT_DUEL: i32 = 3;
const GT_POWERDUEL: i32 = 4;
const GT_SIEGE: i32 = 7;
/// Ordinary `team` → first snapshot in play on a loaded server.
/// How long to wait for the server to confirm the Force profile and spawn
/// us. Reaching it is no longer a failure -- the caller enters the game
/// anyway -- so this only decides how long a join sits still on a server
/// that never answers `nfr`, which a modified game module need not do. Ten
/// seconds of that was the "takes longer to join" the owner reported.
const JOIN_TIMEOUT: Duration = Duration::from_secs(3);
/// Upper bound on the userinfo coalescing wait before joining regardless.
const USERINFO_TIMEOUT: Duration = Duration::from_secs(2);

/// What [`ForceProfileNegotiator::profile_applied`] did about a new profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileApplied {
    /// `forcechanged` is queued behind the userinfo.
    Queued,
    /// A reply that makes the server read the profile is already waiting for
    /// the userinfo, which carries this one.
    AlreadyQueued,
    /// No server's rules are known (not connected, or before its game state),
    /// so nothing was queued; the join sends the profile.
    NotOnServer,
}

/// How the client entered play.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnterPlayOutcome {
    /// The `forcepowers` value now on the wire (also written to `userinfo`).
    pub forcepowers: String,
    /// Whether the server confirmed the profile and a snapshot in play
    /// arrived before the timeout.
    ///
    /// This is not a verdict on the connection. Plenty of servers hold a
    /// fresh client in spectator until it picks a team, and a mod's game
    /// module need not answer with `nfr` at all; a stock client is simply
    /// connected, watching, with the join menu up. Treat a false here as
    /// "still spectating", never as a failed join.
    pub active: bool,
    /// The server answered the force profile (`nfr`), or the gametype never
    /// sends one.
    pub confirmed: bool,
    /// The newest snapshot still shows this client spectating.
    pub spectating: bool,
}

/// Join `team` with a server-legal Force profile, without the stock
/// spectator bounce.
///
/// Legalizes `userinfo.forcepowers` against `CS_SERVERINFO`, pushes it when
/// it changed, then sends `forcechanged "<TEAM>"`: as a spectator that runs
/// `WP_InitForcePowers` immediately (marking the session so the join does not
/// bounce) and hands the team to `Cmd_Team_f` (`g_cmds.c:1276-1313`). Duel
/// gametypes ignore that argument, so a plain `team` command follows there.
/// The `nfr` notices the exchange provokes are consumed. Must be called
/// while the client is still the spectator it connects as.
pub fn enter_play(
    session: &mut ClientSession,
    userinfo: &mut LegacyUserInfo,
    team: LegacyTeamChoice,
) -> Result<EnterPlayOutcome, ClientError> {
    let forcepowers = server_legal_forcepowers(session.game_state(), &userinfo.forcepowers);
    if forcepowers != userinfo.forcepowers {
        userinfo.forcepowers = forcepowers.clone();
        // The tracker coalesces updates to one per second from the connect
        // userinfo on; wait that out so the legal value precedes the
        // `forcechanged` instead of being skipped.
        let deadline = Instant::now() + USERINFO_TIMEOUT;
        while session.update_userinfo(userinfo, Instant::now())?
            == UserinfoUpdateStatus::RateLimited
            && Instant::now() < deadline
        {
            receive_quietly(session)?;
        }
    }
    let rules = force_rules_from_serverinfo(session.game_state(), 0, 0);
    let reply = force_rank_reply(&forcepowers, rules, Some(team));
    session.send_reliable_command(&reply.command)?;
    if matches!(
        serverinfo_i32(session.game_state(), "g_gametype"),
        GT_DUEL | GT_POWERDUEL
    ) {
        session.send_reliable_command(legacy_team_command(team))?;
    }
    // The first snapshots can still show the pre-park in-game state, so wait
    // for the server's own confirmation: `WP_InitForcePowers` answers a
    // legal, confirmed profile with `nfr <rank> 0 <team>`
    // (`w_force.c:394`). Siege never sends it (`scl` instead). JA+ never sends
    // it either, and waiting the whole timeout for it held every join there 3 s
    // behind the loading screen with the player already in the game: once the
    // server has acknowledged the reply (it ran it) and a later snapshot still
    // shows this client in play, no bounce is coming.
    let siege = serverinfo_i32(session.game_state(), "g_gametype") == GT_SIEGE;
    let deadline = Instant::now() + JOIN_TIMEOUT;
    let mut confirmed = siege;
    let mut active = false;
    let mut ran_by = None;
    while Instant::now() < deadline {
        receive_quietly(session)?;
        while let Some(update) = session.take_force_rank_event() {
            confirmed |= !update.open_profile;
        }
        let snapshot_time = session.latest_snapshot().server_time;
        if ran_by.is_none() && session.reliable_commands_acknowledged() {
            ran_by = Some(snapshot_time);
        }
        let settled = ran_by.is_some_and(|time| snapshot_time > time);
        active = (confirmed || settled) && !session.latest_snapshot().player.is_spectator();
        if active {
            break;
        }
    }
    let spectating = session.latest_snapshot().player.is_spectator();
    Ok(EnterPlayOutcome {
        forcepowers,
        active,
        confirmed,
        spectating,
    })
}

/// One receive step that also releases paced commands; timeouts are normal.
fn receive_quietly(session: &mut ClientSession) -> Result<(), ClientError> {
    match session.receive_snapshot(Duration::from_millis(100)) {
        Ok(_) => Ok(()),
        Err(error) if error.is_timeout() => Ok(()),
        Err(error) => Err(error),
    }
}

fn serverinfo_i32(game_state: &GameState, key: &str) -> i32 {
    game_state
        .config_string(0)
        .and_then(|raw| std::str::from_utf8(raw).ok())
        .and_then(|text| InfoString::parse(text).ok())
        .and_then(|info| info.get_i32(key))
        .unwrap_or(0)
}

/// Reliable commands the shell must send this frame, in order.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RejoinOutput {
    /// Byte-exact reliable commands, in send order.
    pub commands: Vec<Vec<u8>>,
    /// Console notice once the bounded retries are exhausted.
    pub notice: Option<String>,
}

#[derive(Debug)]
struct Retry {
    due: Instant,
    attempts_left: u8,
}

/// `forcechanged` without a team: the server re-reads the profile and the
/// player stays where they are (`setForce "none"`, `ui_main.c:6335-6345`).
const FORCE_CHANGED: &[u8] = b"forcechanged";

/// Per-connection Force-profile state: the server's rules the sent profile is
/// fitted to and the `nfr` reply / rejoin machine.
#[derive(Debug)]
pub struct ForceProfileNegotiator {
    /// The server's Force rules, `g_forcePowerDisable` included; `None` off a
    /// server, where the profile goes out as the player wrote it.
    rules: Option<ForceLegalizeRules>,
    /// The server's `EV_SET_FREE_SABER` (its own `HasSetSaberOnly()`), which
    /// wins over the `CS_SERVERINFO` guess once it has arrived. Kept across
    /// the same server's map changes ([`Self::begin_server`]).
    free_saber: Option<bool>,
    /// The address `free_saber` came from.
    address: Option<String>,
    /// `CS_SERVERINFO` carries the weapon mask `HasSetSaberOnly` reads.
    free_saber_shown: bool,
    desired_team: LegacyTeamChoice,
    pending: Option<Vec<u8>>,
    retry: Option<Retry>,
}

impl Default for ForceProfileNegotiator {
    fn default() -> Self {
        Self {
            rules: None,
            free_saber: None,
            address: None,
            free_saber_shown: true,
            desired_team: LegacyTeamChoice::Free,
            pending: None,
            retry: None,
        }
    }
}

impl ForceProfileNegotiator {
    /// The Force rules of the server this connection plays on, its
    /// `g_forcePowerDisable` included.
    pub fn server_rules(&self) -> Option<ForceLegalizeRules> {
        self.rules
    }

    /// Adopt the server's rules (`None` once off a server).
    pub fn set_server_rules(&mut self, rules: Option<ForceLegalizeRules>) {
        self.rules = rules.map(|rules| self.with_free_saber(rules));
    }

    /// Adopt the server's `EV_SET_FREE_SABER` ([`crate::free_saber_event`]),
    /// as stock cgame sets `ui_freeSaber` from it; returns whether that was
    /// news ([`Self::free_saber_news`]). Kept for the same server
    /// ([`Self::begin_server`]), ignored off a server.
    pub fn note_free_saber(&mut self, free_saber: bool) -> bool {
        let Some(rules) = self.rules else {
            return false;
        };
        let news = self.free_saber_news(free_saber);
        self.free_saber = Some(free_saber);
        self.rules = Some(ForceLegalizeRules {
            free_saber,
            ..rules
        });
        news
    }

    fn with_free_saber(&self, rules: ForceLegalizeRules) -> ForceLegalizeRules {
        ForceLegalizeRules {
            free_saber: self.free_saber.unwrap_or(rules.free_saber),
            ..rules
        }
    }

    /// Adopt the rules `CS_SERVERINFO` advertises now, as it changes during a
    /// connection. The side an `nfr` notice held the player to under
    /// `g_forceBasedTeams` is kept: `CS_SERVERINFO` does not carry it.
    pub fn refresh_server_rules(&mut self, game_state: &GameState) {
        let mut rules = server_force_rules(game_state);
        if rules.team_side.is_none() && serverinfo_i32(game_state, "g_forceBasedTeams") != 0 {
            rules.team_side = self.rules.and_then(|held| held.team_side);
        }
        self.free_saber_shown = serverinfo_shows_free_saber(game_state);
        self.rules = Some(self.with_free_saber(rules));
    }

    /// The rules the sent profile is fitted to
    /// ([`ForceLegalizeRules::for_sent_profile`]); `None` off a server.
    pub fn sent_rules(&self) -> Option<ForceLegalizeRules> {
        self.rules.map(ForceLegalizeRules::for_sent_profile)
    }

    /// Whether the server's event or `CS_SERVERINFO` tells if saber skills
    /// come free.
    fn free_saber_known(&self) -> bool {
        self.free_saber.is_some() || self.free_saber_shown
    }

    /// Whether [`Self::note_free_saber`] with `free_saber` would change what
    /// the sent profile is fitted to.
    pub fn free_saber_news(&self, free_saber: bool) -> bool {
        self.rules
            .is_some_and(|rules| rules.free_saber != free_saber || !self.free_saber_known())
    }

    /// The `forcepowers` value sent for the player's `preferred` one: fitted
    /// to the server's rules while on one, keeping disabled and team powers
    /// for the server to grant or remove.
    pub fn sent_forcepowers(&self, preferred: &str) -> String {
        match self.sent_rules() {
            Some(rules) => fit_sent(preferred, rules, self.free_saber_known()),
            None => preferred.to_owned(),
        }
    }

    /// Replace the preferred profile in `userinfo` with the value sent for it.
    pub fn apply_to_userinfo(&self, userinfo: &mut LegacyUserInfo) {
        if self.rules.is_some() {
            userinfo.forcepowers = self.sent_forcepowers(&userinfo.forcepowers);
        }
    }

    /// The player applied a new profile: once its userinfo has gone out, send
    /// `forcechanged` so the server reads it, at once for a spectator and at
    /// the next respawn in play. Nothing to do off a server; a queued `nfr`
    /// reply already does it.
    pub fn profile_applied(&mut self) -> ProfileApplied {
        if self.rules.is_none() {
            ProfileApplied::NotOnServer
        } else if self.pending.is_some() {
            ProfileApplied::AlreadyQueued
        } else {
            self.pending = Some(FORCE_CHANGED.to_vec());
            ProfileApplied::Queued
        }
    }

    /// Remember the team the player last asked for; spectator disables rejoin.
    pub fn note_team_choice(&mut self, choice: LegacyTeamChoice) {
        self.desired_team = choice;
        if choice == LegacyTeamChoice::Spectator {
            self.pending = None;
            self.retry = None;
        }
    }

    /// The team a `forcechanged` reply should return the player to.
    pub fn rejoin_team(&self) -> Option<LegacyTeamChoice> {
        (self.desired_team != LegacyTeamChoice::Spectator).then_some(self.desired_team)
    }

    /// Queue the reply; it is sent once the userinfo has reached the server.
    /// The team is asked for again later while the player stays parked,
    /// except where spectating is the duel queue.
    pub fn queue(&mut self, reply: &ForceRankReply, now: Instant) {
        self.pending = Some(reply.command.clone());
        let duel = self
            .rules
            .is_some_and(|rules| matches!(rules.gametype, GT_DUEL | GT_POWERDUEL));
        self.retry = self.rejoin_team().filter(|_| !duel).map(|_| Retry {
            due: now + REJOIN_INTERVAL,
            attempts_left: REJOIN_ATTEMPTS,
        });
    }

    /// Drop all per-connection state, e.g. on disconnect or a new server.
    pub fn reset(&mut self) {
        self.begin_server("");
        self.free_saber = None;
        self.address = None;
    }

    /// A new game state from `address` (a join or the same server's map
    /// change): per-connection state goes, but the server's free-saber word
    /// stays for the same address. Its next `EV_SET_FREE_SABER`, at every
    /// player's `WP_InitForcePowers`, corrects it if the rules changed.
    pub fn begin_server(&mut self, address: &str) {
        if self.address.as_deref() != Some(address) {
            self.free_saber = None;
            self.address = Some(address.to_owned());
        }
        self.rules = None;
        self.free_saber_shown = true;
        self.pending = None;
        self.retry = None;
    }

    /// Advance the state machine for one frame; call it every frame, as a
    /// retry falls due seconds after the userinfo before it went out.
    ///
    /// `userinfo_settled` is true when no userinfo change is still waiting to be
    /// sent; `spectator` reflects the latest snapshot.
    pub fn poll(&mut self, now: Instant, userinfo_settled: bool, spectator: bool) -> RejoinOutput {
        let mut output = RejoinOutput::default();
        if userinfo_settled {
            if let Some(command) = self.pending.take() {
                output.commands.push(command);
            }
        }
        let Some(retry) = &mut self.retry else {
            return output;
        };
        if self.pending.is_some() || now < retry.due {
            return output;
        }
        if !spectator {
            self.retry = None;
            return output;
        }
        if retry.attempts_left == 0 {
            self.retry = None;
            output.notice = Some(
                "^3Server kept you in spectator mode after the Force profile update; \
                 use the join menu."
                    .to_owned(),
            );
            return output;
        }
        retry.attempts_left -= 1;
        retry.due = now + REJOIN_INTERVAL;
        output
            .commands
            .push(legacy_team_command(self.desired_team).to_vec());
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::force_profile::ForceAllocation;

    /// A server at Jedi Adept (rank 4, 30 points) with Heal (bit 0) and
    /// Grip (bit 6) disabled, in a free-for-all with every weapon.
    fn adept_server() -> GameState {
        let mut game = GameState::empty_local(0);
        game.replace_config_string(
            0,
            b"\\g_gametype\\0\\g_maxForceRank\\4\\g_forcePowerDisable\\65\\g_weaponDisable\\0"
                .to_vec(),
        )
        .unwrap();
        game
    }

    fn levels(value: &str) -> [u8; 18] {
        ForceAllocation::parse(value).unwrap().levels
    }

    #[test]
    fn the_sent_profile_keeps_disabled_powers_within_the_servers_points() {
        let game = adept_server();
        // Light side: Heal 3 (12 points), Jump 1, Push 2 (4), Saber Attack 2 (6).
        let preferred = "7-1-310200000000000200";
        let sent = server_legal_forcepowers(&game, preferred);
        assert_eq!(sent, "4-1-310200000000000200");
        let rules = server_force_rules(&game);
        assert_eq!(rules.max_rank, 4);
        assert_eq!(rules.disabled_mask, 65);
        // What the server grants drops Heal, as `WP_InitForcePowers` does,
        // without calling the profile illegal.
        let granted = legalize_force_powers(&sent, rules);
        assert!(granted.was_legal);
        assert_eq!(granted.allocation.levels[0], 0);
        // Over the 30 points, the sent profile is cut down to fit, disabled
        // powers counted, so the server never parks the player for it.
        let greedy = server_legal_forcepowers(&game, "7-1-333330000000003333");
        let fitted = ForceAllocation::parse(&greedy).unwrap();
        assert!(fitted.used_points(false) <= 30, "{greedy}");
        assert!(legalize_force_powers(&greedy, rules.for_sent_profile()).was_legal);
    }

    #[test]
    fn sent_ffa_team_powers_reach_the_server_without_changing_stock_grants() {
        use crate::force_profile::{ForcePower, ForceSide};
        for (side, team_power, other_power) in [
            (
                ForceSide::Light,
                ForcePower::TeamHeal,
                ForcePower::TeamForce,
            ),
            (ForceSide::Dark, ForcePower::TeamForce, ForcePower::TeamHeal),
        ] {
            let mut profile = ForceAllocation {
                rank: 7,
                side,
                levels: [0; 18],
            };
            profile.levels[team_power as usize] = 3;
            profile.levels[other_power as usize] = 1; // invalid side must still be removed
            for disabled in [0, 1 << team_power as usize] {
                let mut game = GameState::empty_local(0);
                game.replace_config_string(
                    0,
                    format!("\\g_gametype\\0\\g_maxForceRank\\4\\g_forcePowerDisable\\{disabled}")
                        .into_bytes(),
                )
                .unwrap();
                let sent = server_legal_forcepowers(&game, &profile.encode());
                let fitted = ForceAllocation::parse(&sent).unwrap();
                assert_eq!(fitted.rank, 4);
                assert_eq!(fitted.side, side);
                assert_eq!(fitted.levels[team_power as usize], 3);
                assert_eq!(fitted.levels[other_power as usize], 0);
                assert!(fitted.used_points(false) <= 30);
                // OpenJK removes FFA team powers without declaring the profile illegal.
                let granted = legalize_force_powers(&sent, server_force_rules(&game));
                assert!(granted.was_legal);
                assert_eq!(granted.allocation.levels[team_power as usize], 0);
                // A server permitting team powers still applies its disable mask.
                let permitting = legalize_force_powers(
                    &sent,
                    crate::ForceLegalizeRules {
                        gametype: 6,
                        ..server_force_rules(&game)
                    },
                );
                assert_eq!(
                    permitting.allocation.levels[team_power as usize],
                    if disabled == 0 { 3 } else { 0 }
                );
            }
        }
    }

    #[test]
    fn a_serverinfo_refresh_keeps_the_side_an_nfr_notice_set() {
        use crate::force_profile::ForceSide;
        let mut game = adept_server();
        let mut negotiator = ForceProfileNegotiator::default();
        let held = ForceLegalizeRules {
            team_side: Some(ForceSide::Dark),
            ..server_force_rules(&game)
        };
        negotiator.set_server_rules(Some(held));
        // Force-based teams off: the refresh has no side to keep.
        negotiator.refresh_server_rules(&game);
        assert_eq!(negotiator.server_rules().unwrap().team_side, None);
        // On: the side from the notice survives a serverinfo change.
        game.replace_config_string(
            0,
            b"\\g_gametype\\0\\g_maxForceRank\\4\\g_forceBasedTeams\\1".to_vec(),
        )
        .unwrap();
        negotiator.set_server_rules(Some(held));
        negotiator.refresh_server_rules(&game);
        assert_eq!(
            negotiator.server_rules().unwrap().team_side,
            Some(ForceSide::Dark)
        );
    }

    #[test]
    fn the_applied_profile_is_the_one_sent_fitted_to_the_server() {
        let game = adept_server();
        let mut negotiator = ForceProfileNegotiator::default();
        let first = "7-1-310200000000000200";
        assert_eq!(negotiator.sent_forcepowers(first), first, "off a server");
        negotiator.set_server_rules(Some(server_force_rules(&game)));
        assert_eq!(negotiator.sent_forcepowers(first), "4-1-310200000000000200");
        // A profile applied later goes out as applied, not as the join's.
        let later = negotiator.sent_forcepowers("7-2-011110300000000300");
        assert_eq!(levels(&later)[6], 3, "Grip kept");
        assert_eq!(later, "4-2-011110300000000300");
        negotiator.reset();
        assert_eq!(negotiator.server_rules(), None);
        assert_eq!(negotiator.sent_forcepowers(first), first);
    }

    #[test]
    fn an_applied_profile_is_followed_by_forcechanged_once_its_userinfo_is_out() {
        let now = Instant::now();
        let mut negotiator = ForceProfileNegotiator::default();
        assert_eq!(negotiator.profile_applied(), ProfileApplied::NotOnServer);
        assert!(
            negotiator.poll(now, true, false).commands.is_empty(),
            "off a server"
        );
        negotiator.set_server_rules(Some(ForceLegalizeRules::default()));
        assert_eq!(negotiator.profile_applied(), ProfileApplied::Queued);
        assert_eq!(negotiator.profile_applied(), ProfileApplied::AlreadyQueued);
        assert!(negotiator.poll(now, false, false).commands.is_empty());
        let output = negotiator.poll(now, true, false);
        assert_eq!(output.commands, vec![b"forcechanged".to_vec()]);
        assert!(negotiator.poll(now, true, false).commands.is_empty());
        // A queued `nfr` reply, which rejoins the team, is not replaced.
        let reply = force_rank_reply(
            "7-1-032330000000001333",
            ForceLegalizeRules::default(),
            Some(LegacyTeamChoice::Free),
        );
        negotiator.queue(&reply, now);
        assert_eq!(negotiator.profile_applied(), ProfileApplied::AlreadyQueued);
        let output = negotiator.poll(now, true, false);
        assert_eq!(output.commands, vec![b"forcechanged \"FREE\"".to_vec()]);
    }

    #[test]
    fn a_parked_duel_player_waits_in_the_queue() {
        let now = Instant::now();
        for gametype in [GT_DUEL, GT_POWERDUEL] {
            let mut negotiator = ForceProfileNegotiator::default();
            negotiator.set_server_rules(Some(ForceLegalizeRules {
                gametype,
                ..ForceLegalizeRules::default()
            }));
            let reply = force_rank_reply(
                "7-1-032330000000001333",
                ForceLegalizeRules::default(),
                Some(LegacyTeamChoice::Free),
            );
            negotiator.queue(&reply, now);
            assert_eq!(negotiator.poll(now, true, true).commands.len(), 1);
            for step in 1..=REJOIN_ATTEMPTS + 1 {
                let at = now + REJOIN_INTERVAL * u32::from(step);
                assert_eq!(negotiator.poll(at, true, true), RejoinOutput::default());
            }
        }
    }

    #[test]
    fn a_parked_player_is_sent_back_to_the_team_until_the_retries_run_out() {
        let now = Instant::now();
        let mut negotiator = ForceProfileNegotiator::default();
        let reply = force_rank_reply(
            "7-1-032330000000001333",
            ForceLegalizeRules::default(),
            Some(LegacyTeamChoice::Red),
        );
        negotiator.note_team_choice(LegacyTeamChoice::Red);
        negotiator.queue(&reply, now);
        assert_eq!(negotiator.poll(now, true, true).commands.len(), 1);
        let mut at = now;
        for _ in 0..REJOIN_ATTEMPTS {
            at += REJOIN_INTERVAL;
            let output = negotiator.poll(at, true, true);
            assert_eq!(output.commands, vec![b"team red".to_vec()]);
        }
        at += REJOIN_INTERVAL;
        assert!(negotiator.poll(at, true, true).notice.is_some());
        assert_eq!(
            negotiator.poll(at + REJOIN_INTERVAL, true, true),
            RejoinOutput::default()
        );
    }

    fn snapshot(entities: Vec<sjk_protocol::EntityState>) -> sjk_protocol::Snapshot {
        sjk_protocol::Snapshot {
            message_sequence: 1,
            reliable_acknowledge: 0,
            server_commands: Vec::new(),
            server_time: 1_000,
            delta_from: None,
            flags: 0,
            area_mask: Vec::new(),
            player: sjk_protocol::PlayerState::zero(),
            vehicle_player: None,
            entities,
            consumed_bits: 0,
        }
    }

    #[test]
    fn the_servers_free_saber_event_is_read_from_its_temp_entity() {
        use sjk_game_jka::event_entity::EventEntity;
        assert_eq!(crate::free_saber_event(&snapshot(Vec::new())), None);
        for saber_only in [false, true] {
            let entities = EventEntity::force_rules(saber_only, false)
                .iter()
                .map(EventEntity::state)
                .collect();
            assert_eq!(
                crate::free_saber_event(&snapshot(entities)),
                Some(saber_only)
            );
        }
    }

    const FULLFORCE_FFA: &str = "7-1-331330000330001133";
    const WITHOUT_SPEED: &str = "7-1-330330000330001133";

    fn game_with(info: Vec<u8>) -> GameState {
        let mut game = GameState::empty_local(0);
        game.replace_config_string(0, info).unwrap();
        game
    }

    /// The server's `EV_SET_FREE_SABER` wins over the `CS_SERVERINFO` guess,
    /// as `ui_freeSaber` does in stock, through the same server's map changes;
    /// another server or a disconnect forgets it.
    #[test]
    fn the_free_saber_event_decides_the_sent_profile() {
        // Every weapon allowed, as far as the serverinfo tells.
        let game = game_with(b"\\g_gametype\\0\\g_maxForceRank\\7\\g_weaponDisable\\0".to_vec());
        let mut negotiator = ForceProfileNegotiator::default();
        assert!(!negotiator.note_free_saber(true), "off a server");
        negotiator.begin_server("203.0.113.1:29070");
        negotiator.refresh_server_rules(&game);
        assert!(!negotiator.server_rules().unwrap().free_saber);
        assert_eq!(
            negotiator.sent_forcepowers(FULLFORCE_FFA),
            WITHOUT_SPEED,
            "Speed cut as BG_LegalizedForcePowers cuts it"
        );
        assert!(negotiator.note_free_saber(true));
        assert!(!negotiator.note_free_saber(true));
        assert_eq!(negotiator.sent_forcepowers(FULLFORCE_FFA), FULLFORCE_FFA);
        negotiator.set_server_rules(Some(server_force_rules(&game)));
        assert!(negotiator.server_rules().unwrap().free_saber);
        // A map change: a new game state from the same server.
        negotiator.begin_server("203.0.113.1:29070");
        negotiator.refresh_server_rules(&game);
        assert!(negotiator.server_rules().unwrap().free_saber);
        assert_eq!(negotiator.sent_forcepowers(FULLFORCE_FFA), FULLFORCE_FFA);
        // Another server starts from its serverinfo.
        negotiator.begin_server("203.0.113.2:29070");
        negotiator.refresh_server_rules(&game);
        assert!(!negotiator.server_rules().unwrap().free_saber);
        assert!(negotiator.note_free_saber(true));
        // A disconnect forgets it, even for the same server after.
        negotiator.reset();
        negotiator.begin_server("203.0.113.2:29070");
        negotiator.refresh_server_rules(&game);
        assert!(!negotiator.server_rules().unwrap().free_saber);
    }

    /// Without the weapon mask in `CS_SERVERINFO` and before the server's
    /// event, the profile is not cut by a guess: stock sends it as committed.
    #[test]
    fn an_unknown_free_saber_does_not_cut_the_sent_profile() {
        let hidden = game_with(b"\\g_gametype\\0\\g_maxForceRank\\7".to_vec());
        assert!(!serverinfo_shows_free_saber(&hidden));
        assert_eq!(
            server_legal_forcepowers(&hidden, FULLFORCE_FFA),
            FULLFORCE_FFA
        );
        let mut negotiator = ForceProfileNegotiator::default();
        negotiator.begin_server("203.0.113.1:29070");
        negotiator.refresh_server_rules(&hidden);
        assert_eq!(negotiator.sent_forcepowers(FULLFORCE_FFA), FULLFORCE_FFA);
        // The server's word then decides, both ways.
        assert!(negotiator.note_free_saber(false));
        assert_eq!(negotiator.sent_forcepowers(FULLFORCE_FFA), WITHOUT_SPEED);
        assert!(!negotiator.note_free_saber(false));
        // Shown in serverinfo, the guess is used until then.
        let shown = game_with(b"\\g_gametype\\0\\g_maxForceRank\\7\\g_weaponDisable\\0".to_vec());
        assert!(serverinfo_shows_free_saber(&shown));
        assert_eq!(
            server_legal_forcepowers(&shown, FULLFORCE_FFA),
            WITHOUT_SPEED
        );
        let duel = game_with(b"\\g_gametype\\3\\g_weaponDisable\\0".to_vec());
        assert!(
            !serverinfo_shows_free_saber(&duel),
            "duels read g_duelWeaponDisable"
        );
    }

    /// `HasSetSaberOnly` tests `g_weaponDisable` as C `int` bits: -1 disables
    /// every weapon.
    #[test]
    fn a_negative_weapon_disable_is_every_weapon() {
        let mut game = GameState::empty_local(0);
        game.replace_config_string(0, b"\\g_gametype\\0\\g_weaponDisable\\-1".to_vec())
            .unwrap();
        assert!(server_force_rules(&game).free_saber);
        game.replace_config_string(0, b"\\g_gametype\\0\\g_weaponDisable\\524279".to_vec())
            .unwrap();
        assert!(server_force_rules(&game).free_saber);
        game.replace_config_string(0, b"\\g_gametype\\0\\g_weaponDisable\\1".to_vec())
            .unwrap();
        assert!(!server_force_rules(&game).free_saber);
    }
}
