//! What a player sends: `ClientCommand` (`g_cmds.c`) and `ClientThink` (`g_active.c`), for
//! the player at a place in the profile's order. The host boundary (`bridge_host.rs`) calls
//! them for legacy clients; the bots' frame calls them for bots.

use super::*;
use sjk_network::LegacyTokens;

/// `text` split as `Cmd_Argv` splits it, the command and then its arguments: the
/// client sends quoted ones (`forcechanged "RED"`), and an empty one (`""`) is no
/// argument.
fn command_words(text: &[u8]) -> Option<(&[u8], Vec<&[u8]>)> {
    let mut tokens = LegacyTokens::new(text);
    let command = tokens.next()?;
    Some((command, tokens.filter(|word| !word.is_empty()).collect()))
}

impl NativeGame {
    /// `ClientCommand`: a reliable command from the player at `client`'s place, at the
    /// server's clock `server_time`.
    pub(super) fn player_command(&mut self, client: usize, text: &[u8], server_time: i32) {
        let Some((command, words)) = command_words(text) else {
            return;
        };
        // The running server mod's own commands come first (VisionJK `g_cmds.c:3862`,
        // `JP_ClientCommand`), so a name the game's table also has (`jetpack`) is the
        // mod's (`sjk_server_mod::ModCommand`).
        if let Some(added) = self
            .server_mod
            .and_then(|server_mod| server_mod.command(command))
        {
            self.mod_command(client, command, text, &words, added, server_time);
            return;
        }
        let mut words = words.into_iter();
        if [
            b"kill".as_slice(),
            b"team",
            b"follow",
            b"follownext",
            b"followprev",
            b"setviewpos",
        ]
        .iter()
        .any(|name| command.eq_ignore_ascii_case(name))
        {}
        if command.eq_ignore_ascii_case(b"score") {
            // `Cmd_Score_f`: the scoreboard, as this client asked for it.
            let message = self.scoreboard(server_time);
            self.told.push(Told::One(client, message));
            return;
        }
        if command.eq_ignore_ascii_case(b"kill") {
            if !self.kill_refused(client) {
                self.kill(client, server_time);
            }
            return;
        }
        if command.eq_ignore_ascii_case(b"duelteam") {
            self.duel_team_command(client, words.next(), server_time);
            return;
        }
        if [&b"follow"[..], b"follownext", b"followprev"]
            .iter()
            .any(|name| command.eq_ignore_ascii_case(name))
        {
            // `CMD_NOINTERMISSION` (`g_cmds.c:3398-3400`).
            if !self.refused_at_intermission(client, command) {
                let words: Vec<&[u8]> = std::iter::once(command).chain(words).collect();
                self.follow_command(client, &words, server_time);
            }
            return;
        }
        if command.eq_ignore_ascii_case(b"setviewpos") {
            let arguments: Vec<&[u8]> = words.collect();
            self.set_view_position(client, &arguments, server_time);
            return;
        }
        if command.eq_ignore_ascii_case(b"give") {
            // `ClientCommand`'s gates (`g_cmds.c:3466-3480`): not at the intermission
            // (`CMD_NOINTERMISSION`), a cheat needs `sv_cheats`, and this one a living
            // player; then `Cmd_Give_f`.
            if self.refused_at_intermission(client, command) {
                return;
            }
            if !self.settings.cheats {
                self.told
                    .push(Told::One(client, b"print \"@@@NOCHEATS\n\"".to_vec()));
                return;
            }
            let arguments: Vec<&[u8]> = words.collect();
            if self
                .peer(client)
                .is_none_or(|peer| peer.health <= 0 || !peer.playing())
            {
                self.told
                    .push(Told::One(client, b"print \"@@@MUSTBEALIVE\n\"".to_vec()));
                return;
            }
            let (name, rest) = arguments
                .split_first()
                .map_or((&b""[..], &[][..]), |(name, rest)| (*name, rest));
            self.give_to(client, name, rest);
            return;
        }
        if command.eq_ignore_ascii_case(b"siegeclass") {
            // `Cmd_SiegeClass_f`: only a siege game has classes.
            self.siege_class_command(client, text, server_time);
            return;
        }
        if self.chat_command(client, text, server_time)
            || self.vote_command(client, text, server_time)
            || self.npc_client_command(client, text)
            || self.cheat_command(client, text)
        {
            return;
        }
        let arguments: Vec<&[u8]> = words.collect();
        if self.table_command(client, command, &arguments, server_time) {
            return;
        }
        // `Cmd_ForceChanged_f` goes on as `Cmd_Team_f` when the menu sent a team with it.
        let team = command.eq_ignore_ascii_case(b"team")
            || (command.eq_ignore_ascii_case(b"forcechanged")
                && self.force_changed(client, !arguments.is_empty()));
        if !team {
            use sjk_game_jka::client_commands::{ADDBOT_REFUSED, is_command, unknown};
            // A name `commands[]` does not have is unknown; `addbot` is the server's.
            if command.eq_ignore_ascii_case(b"addbot") {
                self.told.push(Told::One(client, ADDBOT_REFUSED.to_vec()));
            } else if !is_command(command) {
                self.told.push(Told::One(client, unknown(command)));
            }
            return;
        }
        // The running mod's team locks (JA+'s `jp_teamLock`), which `SetTeam` asks first.
        let mut arguments = arguments;
        if let Some(&word) = arguments.first() {
            match self.admin_team_word(client, word) {
                Some(word) => arguments[0] = word,
                None => return,
            }
        }
        let gametype = self.gametype;
        // `SetTeam` on a siege server, once `Cmd_Team_f`'s own checks have passed.
        if self.siege.is_some()
            && let [word] = arguments.as_slice()
            && self
                .peer(client)
                .is_some_and(|peer| peer.session.switch_team_time <= server_time)
        {
            self.siege_team_command(client, word, server_time);
            return;
        }
        let sides = self.sides(client);
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let (before, name) = (peer.session.team, peer.name.clone());
        // The game this server is running, not a hard-coded free-for-all: `SetTeam`
        // only knows red and blue from `GT_TEAM` up, which is what a siege class's own
        // side change depends on.
        match team_command(
            &mut peer.session,
            &name,
            &arguments,
            gametype,
            sides,
            server_time,
        ) {
            TeamCommand::Told(text) => self.told.push(Told::One(client, text)),
            TeamCommand::Unchanged | TeamCommand::NotSupported => {}
            TeamCommand::Changed {
                announcement,
                queued,
            } => self.team_changed(client, before, announcement, queued, server_time),
        }
        if let Some(peer) = self.peer_mut(client) {
            peer.session.team_command_done(before, server_time);
        }
    }

    /// `ClientThink`: one movement command from the player at `client`'s place.
    pub(super) fn player_think(&mut self, client: usize, command: &UserCommand, server_time: i32) {
        // `ClientThink_real` (g_active.c:2006-2012) keeps a command's time within a
        // second behind and a fifth of a second ahead of the server's.
        let mut command = *command;

        command.server_time = command.server_time.clamp(
            server_time.wrapping_sub(1000),
            server_time.wrapping_add(200),
        );

        // A pilot's command goes to its vehicle first (`g_active.c:1889-1910`).
        self.riding_opening(client, &mut command, self.last_frame_time);
        // `ClientThink_real`'s stance upkeep (`g_active.c:1914-1993`): the style the
        // sabers held allow.
        if let Some(peer) = self.peer_mut(client) {
            sjk_game_jka::saber_stance::upkeep_player(
                &mut peer.state,
                &mut peer.movement,
                &peer.sabers.hands,
            );
            // The powers the sabers forbid (`WP_ForcePowerUsable`), as they stand now.
            peer.force.saber_restrictions = peer.sabers.force_restrictions();
        }
        // `G_HeldByMonster` (`g_active.c:2000-2003`): a monster's victim goes where it is held.
        self.held_by_monster(client, &mut command);
        // `ClientThink_real` (`g_active.c:2356-2363`) sets the speed before EVERY move,
        // not once at the spawn: `g_speed`, times a siege class's own multiplier. Doing
        // it only at the spawn leaves a client that picked a class predicting one speed
        // while the server moves it at another, which shows up as prediction misses a
        // snapshot's worth of movement wide.
        if let Some(siege) = self.siege.as_ref() {
            let registry = std::sync::Arc::clone(&siege.registry);
            if let Some(peer) = self.peer_mut(client)
                && peer.playing()
            {
                let class_speed = peer
                    .siege_class_index
                    .map_or(1.0, |index| registry.classes[index].speed);
                peer.state.set_speed(PLAYER_SPEED * class_speed);
                peer.state
                    .set_base_speed((PLAYER_SPEED * class_speed) as i32);
                peer.movement = peer.movement.reseeded(&peer.state);
            }
        }
        // `ClientThink_real` (`g_active.c:2046-2054`): at an intermission a player does
        // nothing at all but press the button that says it is ready to go.
        if self.match_end.intermission_time != 0 {
            self.intermission_think(client, &command, server_time);
            return;
        }
        // `SpectatorThink` (`g_active.c:697-760`): a follower does not move itself; the
        // buttons pick whom to follow.
        if let Some(following) = self
            .peer(client)
            .filter(|peer| !peer.playing())
            .map(|peer| {
                peer.session.spectator_state == sjk_game_jka::client_begin::SPECTATOR_FOLLOW
            })
        {
            if following {
                if let Some(peer) = self.peer_mut(client) {
                    peer.last_command_time = server_time;
                    peer.last_command = command;
                }
            } else {
                self.think_in_world(client, command, server_time);
            }
            self.spectator_buttons(client, &command, server_time);
            return;
        }
        self.think_in_world(client, command, server_time);
    }
}

#[cfg(test)]
mod tests {
    use super::command_words;
    use sjk_game_jka::client_begin::{Sides, TEAM_RED, team_for_word};

    #[test]
    fn quoted_arguments_are_split_as_cmd_argv_splits_them() {
        let (command, words) = command_words(b"forcechanged \"RED\"").unwrap();
        assert_eq!(command, b"forcechanged");
        assert_eq!(words, [b"RED".as_slice()]);
        // The menu's team is the team asked for, not a random one.
        assert_eq!(team_for_word(words[0], Sides::default()), TEAM_RED);
        // An empty argument is none: `forcechanged ""` does not go on as `team`.
        assert_eq!(command_words(b"forcechanged \"\"").unwrap().1.len(), 0);
        assert_eq!(
            command_words(b"team  blue ").unwrap().1,
            [b"blue".as_slice()]
        );
        assert!(command_words(b"   ").is_none());
    }
}
