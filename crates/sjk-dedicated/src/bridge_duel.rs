//! Private duels on the server: the duel key (`Cmd_EngageDuel_f`), the duel's part of a
//! think (`ClientThink_real`, `g_active.c:2405-2530`) and what a duel shields
//! (`G_Damage`'s duel rule, `G_MissileImpact`'s `killProj`). The rules themselves are
//! [`sjk_game_jka::duel`]'s, with the values the running server mod gives
//! ([`sjk_game_jka::duel::DuelRules`]: JA+'s isolation, challenge rules and full-force
//! duel).

use super::{NativeGame, PLAYER_SPEED, Told};
use crate::collision::{Void, WithPlayers, WorldCollision};
use crate::peer::Peer;
use sjk_game_jka::damage::{DamageRequest, Target};
use sjk_game_jka::duel::{self, Challenge, Duellist, FullForce};
use sjk_game_jka::event_entity::EventEntity;
use sjk_game_jka::pmove::MovementCollision;
use sjk_protocol::UserCommand;

/// `MASK_PLAYERSOLID`: what the challenge's trace stops at.
const MASK_PLAYERSOLID: u32 = 0x1 | 0x10 | 0x100 | 0x1000;
/// `EV_GENERAL_SOUND`, and the wire field its channel goes in (`saberEntityNum`).
const EV_GENERAL_SOUND: u32 = 76;
const ES_SABER_ENTITY: usize = 37;
/// `CHAN_AUTO`: the duel's saber sounds.
const CHAN_AUTO: u32 = 0;
/// `g_privateDuel`, on by default; there is no cvar system yet.
const PRIVATE_DUELS: bool = true;

/// `peer`, player `number`, as a duel reads and changes it.
fn duellist(number: usize, peer: &mut Peer) -> Duellist<'_> {
    Duellist {
        number: number as u16,
        state: &mut peer.state,
        health: &mut peer.health,
        name: &peer.name,
        hand_extend_time: &mut peer.knockdown.hand_extend_time,
        invulnerable_until: &mut peer.invulnerable_until,
        team: peer.session.team,
        force_max: peer.force.max,
        full_force: &mut peer.full_force_duel,
        rest_until: &mut peer.duel_rest_until,
    }
}

impl NativeGame {
    /// `run` with `client` as a duellist and `other` as the other, when it is somebody
    /// else still connected. `client`'s state and name are taken out of its peer while it
    /// runs (into a slot kept for it, so nothing allocates), so the other can be reached
    /// through the world.
    fn with_duellists<T>(
        &mut self,
        client: usize,
        other: usize,
        run: impl FnOnce(&mut Duellist, Option<&mut Duellist>) -> T,
    ) -> Option<T> {
        let Self {
            server,
            world,
            players,
            duel_slot,
            ..
        } = self;
        let world = server.world_mut(*world)?;
        let handle = players.at(client)?;
        let peer = world.entity_mut(handle)?;
        std::mem::swap(&mut peer.state, duel_slot);
        let name = std::mem::take(&mut peer.name);
        let (mut health, mut hand, mut invulnerable, team) = (
            peer.health,
            peer.knockdown.hand_extend_time,
            peer.invulnerable_until,
            peer.session.team,
        );
        let (force_max, mut full_force, mut rest_until) =
            (peer.force.max, peer.full_force_duel, peer.duel_rest_until);
        let result = {
            let mut me = Duellist {
                number: client as u16,
                state: duel_slot,
                health: &mut health,
                name: &name,
                hand_extend_time: &mut hand,
                invulnerable_until: &mut invulnerable,
                team,
                force_max,
                full_force: &mut full_force,
                rest_until: &mut rest_until,
            };
            let them = players
                .at(other)
                .filter(|_| other != client)
                .and_then(|handle| world.entity_mut(handle));
            let mut them = them.map(|peer| duellist(other, peer));
            run(&mut me, them.as_mut())
        };
        let peer = world.entity_mut(handle)?;
        std::mem::swap(&mut peer.state, duel_slot);
        peer.name = name;
        (
            peer.health,
            peer.knockdown.hand_extend_time,
            peer.invulnerable_until,
        ) = (health, hand, invulnerable);
        (peer.full_force_duel, peer.duel_rest_until) = (full_force, rest_until);
        Some(result)
    }

    /// The duel's messages, saber sounds (`G_Sound` on `CHAN_AUTO` where each player
    /// stands) and events (`eventTime`), and both players' movement restarted from their
    /// states.
    fn apply_duel(&mut self, told: duel::Told, players: [usize; 2], level_time: i32) {
        for (to, text) in told.commands {
            self.told.push(match to {
                Some(client) => Told::One(usize::from(client), text),
                None => Told::Everyone(text),
            });
        }
        for (number, played) in told.sounds {
            // The duellist's sabers' own sounds (`saber[n].soundOn`, `soundOff`).
            let Some(peer) = self.peer(usize::from(number)) else {
                continue;
            };
            if !played.plays(peer.sabers.hands[1].is_held()) {
                continue;
            }
            let (origin, saber) = (
                peer.state.origin(),
                &peer.sabers.hands[usize::from(played.hand)],
            );
            let index = if played.on {
                saber.sound_on
            } else {
                saber.sound_off
            };
            if index == 0 {
                continue;
            }
            let mut sound = EventEntity {
                event: EV_GENERAL_SOUND,
                parameter: u32::from(index),
                origin,
                client: None,
                broadcast: false,
                extra: [(0, 0); 12],
            };
            sound.extra[0] = (ES_SABER_ENTITY, CHAN_AUTO);
            let _ = self.pool.spawn_temporary(sound.state(), level_time, None);
        }
        for number in told.raised {
            if let Some(peer) = self.peer_mut(usize::from(number)) {
                peer.entity.event_raised(level_time);
            }
        }
        for number in players {
            if let Some(peer) = self.peer_mut(number) {
                peer.movement = peer.movement.reseeded(&peer.state);
            }
        }
    }

    /// `Cmd_EngageDuel_f` for `client` (its duel key; with `full_force`, JA+'s
    /// `engage_FullForceDuel`, `Cmd_EngageFullForceDuel_f`): the challenge line through
    /// the map and the others, and a challenge or an acceptance of whom it strikes.
    pub(super) fn duel_key(&mut self, client: usize, level_time: i32, full_force: bool) {
        let rules = self.duel_rules;
        let gametype = self.gametype;
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if full_force {
            // Not for one already duelling (VisionJK `g_cmds.c:3456-3462`).
            if peer.state.duel_in_progress() {
                return;
            }
            peer.full_force_duel = true;
        }
        let challenge = duel::may_challenge(
            &peer.state,
            peer.session.team,
            gametype,
            PRIVATE_DUELS,
            level_time,
            peer.duel_rest_until,
            &rules,
        );
        let (start, end) = duel::challenge_line(&peer.state);
        match challenge {
            Challenge::Traces => {}
            Challenge::Refused => return,
            Challenge::Told(text) => {
                self.told.push(Told::One(client, text.to_vec()));
                return;
            }
        }
        // `trap->Trace`, not JA+'s `JP_Trace`: the line stops at a duellist too.
        self.gather_obstacles_isolated(client, false);
        let Self { map, obstacles, .. } = &*self;
        let hit = match map {
            Some(map) => WithPlayers {
                world: WorldCollision {
                    bsp: &map.bsp,
                    scratch: &map.scratch,
                },
                players: obstacles,
            }
            .trace(start, [0.0; 3], [0.0; 3], end, MASK_PLAYERSOLID),
            None => WithPlayers {
                world: Void,
                players: obstacles,
            }
            .trace(start, [0.0; 3], [0.0; 3], end, MASK_PLAYERSOLID),
        };
        let other = usize::from(hit.entity_number);
        if hit.fraction == 1.0 || other >= self.players.places() || other == client {
            return;
        }
        // `BOTH_BOW` and its length (`BG_AnimLength`), for JA+'s bow.
        let bow = sjk_game_jka::legacy_animation_index("BOTH_BOW").map(|index| {
            let length = self
                .map
                .as_ref()
                .and_then(|map| map.animations.as_ref())
                .and_then(|lengths| lengths.length_ms(index as u16))
                .unwrap_or(0);
            (index as u16, length)
        });
        let told = self
            .with_duellists(client, other, |me, them| {
                them.map(|them| {
                    duel::engage(me, them, gametype, level_time, &rules, full_force, bow)
                })
            })
            .flatten();
        if let Some(told) = told {
            self.apply_duel(told, [client, other], level_time);
        }
    }

    /// `ClientThink_real`'s duel for `client`, before its move: the sabers lit once the
    /// wait is over, the duel ended, or the command and the speed held still while it has
    /// not begun.
    pub(super) fn duel_think(&mut self, client: usize, command: &mut UserCommand, level_time: i32) {
        let Some(peer) = self.peer(client) else {
            return;
        };
        if !peer.state.duel_in_progress() {
            return;
        }
        let other = usize::from(peer.state.duel_index());
        let mut told = duel::Told::default();
        let rules = self.duel_rules;
        let Some(thought) = self.with_duellists(client, other, |me, them| {
            duel::think(me, them, level_time, &rules, &mut told)
        }) else {
            return;
        };
        let touched =
            !told.raised.is_empty() || !told.sounds.is_empty() || !told.commands.is_empty();
        if touched {
            self.apply_duel(told, [client, other], level_time);
        }
        // `speed`, `basespeed`: `g_speed` every think, 0 while held still.
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let base_speed = peer.state.raw_field(37).unwrap_or(0);
        if thought.frozen {
            (command.forward_move, command.right_move, command.up_move) = (0, 0, 0);
            if base_speed != 0 {
                peer.state.set_speed(0.0);
                peer.state.set_base_speed(0);
                peer.movement = peer.movement.reseeded(&peer.state);
            }
        } else if base_speed == 0 {
            peer.state.set_speed(PLAYER_SPEED);
            peer.state.set_base_speed(PLAYER_SPEED as i32);
            peer.movement = peer.movement.reseeded(&peer.state);
        }
    }

    /// `G_Damage`'s duel rule for a blow on `target` (`g_combat.c:4540-4556`): whether it
    /// is refused, in which case only a DEMP2's shock lands.
    pub(super) fn duel_refuses(&mut self, target: usize, request: &DamageRequest) -> bool {
        let Some(victim) = self.peer(target) else {
            return false;
        };
        // Only a client attacks as a client: a trigger's or a mover's number is past them.
        let attacker = request
            .attacker
            .filter(|attacker| usize::from(attacker.client) < self.players.places())
            .and_then(|attacker| {
                Some((
                    attacker.client,
                    &self.peer(usize::from(attacker.client))?.state,
                ))
            });
        if !duel::damage_refused(
            target as u16,
            &victim.state,
            attacker,
            request.means,
            self.full_force_duels(),
        ) {
            return false;
        }
        let mut rng = self.deaths.rng;
        if let Some(victim) = self.peer_mut(target) {
            let (bottom, top) = victim.movement.box_bounds();
            let origin = victim.state.origin();
            let bounds = (
                std::array::from_fn(|axis| origin[axis] + bottom[axis] - 1.0),
                std::array::from_fn(|axis| origin[axis] + top[axis] + 1.0),
            );
            let team = victim.session.team;
            let mut view = Target {
                client: target as u16,
                state: &mut victim.state,
                health: &mut victim.health,
                origin,
                bounds,
                invulnerable_until: &mut victim.invulnerable_until,
                team,
                wounds: &mut victim.wounds,
                force: None,
                spared_by_master: false,
                god: victim.god,
            };
            sjk_game_jka::damage::shock(&mut view, request, &mut rng);
            victim.movement = victim.movement.reseeded(&victim.state);
        }
        self.deaths.rng = rng;
        true
    }

    /// The players in a JA+ full-force duel now, a bit each: none on the base game, which
    /// never sets the flag.
    pub(super) fn full_force_duels(&self) -> FullForce {
        let Some(world) = self.server.world(self.world) else {
            return FullForce::default();
        };
        let mut bits = 0_u64;
        for (number, handle) in self.players.holders().enumerate().take(64) {
            let duelling = handle
                .and_then(|handle| world.entity(handle))
                .is_some_and(|peer| peer.full_force_duel && peer.state.duel_in_progress());
            bits |= u64::from(duelling) << number;
        }
        FullForce(bits)
    }

    /// `G_MissileImpact`'s `killProj`: a missile at a player duelling somebody other than
    /// its owner is spent harmlessly.
    pub(super) fn duel_spares(&self, target: usize, owner: u16) -> bool {
        target < self.players.places()
            && self
                .peer(target)
                .is_some_and(|peer| duel::elsewhere(&peer.state, owner))
    }
}

#[cfg(test)]
mod tests {
    use super::NativeGame;
    use crate::bridge::Identity;
    use crate::cvars::Cvars;
    use sjk_game_jka::damage::{Attacker, DamageRequest};

    /// A mapless server whose start had `g_mod` set to `server_mod`, with players 0, 1
    /// and 2 in it, sabers in hand: 0 facing 1, 100 units along x and
    /// facing back, 2 off to the side.
    fn game(server_mod: &str) -> NativeGame {
        let mut cvars = Cvars::new();
        cvars.set(b"g_mod", server_mod.as_bytes());
        // Short userinfo strings: only the name.
        cvars.set(b"g_userinfoValidate", b"0");
        cvars.register_server();
        let identity = Identity {
            hostname: b"test".to_vec(),
            mapname: b"mp/ffa3".to_vec(),
        };
        let mut game = NativeGame::with_cvars(identity, None, 4, 4, cvars).unwrap();
        // Away from the spawn point, which the next to begin would telefrag them at.
        for (client, origin, yaw) in [
            (0, [500.0, 0.0, 0.0], 0.0),
            (1, [600.0, 0.0, 0.0], 180.0),
            (2, [500.0, 200.0, 0.0], 0.0),
        ] {
            let userinfo = format!(r"\name\P{client}");
            game.connect_client(client, userinfo.as_bytes(), true)
                .unwrap();
            game.begin(client, 0, None);
            let peer = game.peer_mut(client).unwrap();
            peer.state.set_origin(origin);
            peer.state.set_view_angles([0.0, yaw, 0.0]);
            // `WP_SABER`, half health and no armour.
            peer.state.set_raw_field(47, 3);
            peer.health = 50;
            peer.state.stats[0] = 50;
            peer.state.stats[5] = 0;
            peer.force.max = 100;
        }
        game
    }

    /// 0 challenges 1, and 1 accepts.
    fn duel(game: &mut NativeGame, time: i32) {
        game.last_frame_time = time;
        game.duel_key(0, time, false);
        game.duel_key(1, time + 50, false);
    }

    fn obstacles(game: &mut NativeGame, client: usize) -> Vec<u16> {
        game.gather_obstacles(client);
        game.obstacles
            .iter()
            .map(|obstacle| obstacle.entity)
            .filter(|&entity| entity < 32)
            .collect()
    }

    fn duelling(game: &NativeGame, client: usize) -> bool {
        game.peer(client).unwrap().state.duel_in_progress()
    }

    #[test]
    fn the_base_game_duels_and_collides_as_before() {
        let mut game = game("base");
        assert_eq!(game.duel_rules, sjk_game_jka::duel::DuelRules::default());
        duel(&mut game, 1_000);
        assert!(duelling(&game, 0) && duelling(&game, 1));
        let peer = game.peer(0).unwrap();
        // Nothing refilled, the challenger's gesture.
        assert_eq!((peer.health, peer.state.stats[5]), (50, 0));
        assert_eq!(peer.state.raw_field(80), Some(9));
        // Everyone still runs into everyone.
        assert_eq!(obstacles(&mut game, 2), [0, 1]);
        assert_eq!(obstacles(&mut game, 0), [1, 2]);
        // No JA+ command, and no full-force duel.
        assert!(game.server_mod.is_none());
        assert_eq!(game.full_force_duels().0, 0);
    }

    #[test]
    fn ja_plus_duellists_and_the_others_pass_through_each_other() {
        let mut game = game("japlus");
        duel(&mut game, 1_000);
        assert!(duelling(&game, 0) && duelling(&game, 1));
        assert_eq!(obstacles(&mut game, 2), Vec::<u16>::new());
        assert_eq!(obstacles(&mut game, 0), [1]);
        assert_eq!(obstacles(&mut game, 1), [0]);
        // `jp_DuelAlpha -1` switches it off.
        game.set_cvar("jp_DuelAlpha", "-1");
        assert_eq!(obstacles(&mut game, 2), [0, 1]);
        game.set_cvar("jp_DuelAlpha", "0");
        assert_eq!(obstacles(&mut game, 2), Vec::<u16>::new());
    }

    #[test]
    fn a_ja_plus_duel_starts_refilled_with_a_bow() {
        let mut game = game("japlus");
        duel(&mut game, 1_000);
        for client in [0, 1] {
            let peer = game.peer(client).unwrap();
            // 100 health, `jp_duelStartArmor` 100, the full pool, the bow.
            assert_eq!((peer.health, peer.state.stats[0]), (100, 100));
            assert_eq!(peer.state.stats[5], 100);
            assert_eq!(peer.state.raw_field(18), Some(100));
            assert_eq!(peer.state.raw_field(80), Some(10));
        }
    }

    #[test]
    fn ja_plus_challenges_with_any_weapon_and_says_why_not() {
        let mut game = game("japlus");
        // `WP_BRYAR_PISTOL`: a challenge, but no acceptance.
        game.peer_mut(0).unwrap().state.set_raw_field(47, 4);
        duel(&mut game, 1_000);
        assert!(!duelling(&game, 0));
        assert_eq!(game.peer(0).unwrap().state.duel_index(), 1);
        // The jetpack is refused, and said.
        game.told.clear();
        game.peer_mut(2).unwrap().state.stats[2] |= 1 << 7;
        game.duel_key(2, 9_000, false);
        assert!(game.told.iter().any(|told| matches!(told,
            super::Told::One(2, text) if text.windows(7).any(|word| word == b"jetpack"))));
    }

    #[test]
    fn a_full_force_duel_lets_the_force_hurt_between_its_two() {
        let lightning = |attacker: u16| DamageRequest {
            level_time: 2_000,
            attacker: Some(Attacker {
                npc: false,
                client: attacker,
                max_health: 100,
                team: 0,
                saber_knockback: [1.0; 4],
            }),
            direction: None,
            point: None,
            damage: 10,
            flags: 0,
            means: sjk_game_jka::means_of_death::MOD_FORCE_DARK,
        };
        let mut full = game("japlus");
        full.last_frame_time = 1_000;
        full.player_command(0, b"engage_FullForceDuel", 1_000);
        full.duel_key(1, 1_050, false);
        assert!(duelling(&full, 0) && duelling(&full, 1));
        assert_eq!(full.full_force_duels().0, 0b11);
        assert!(!full.duel_refuses(1, &lightning(0)));
        assert!(full.duel_refuses(2, &lightning(0)));
        // An ordinary duel refuses it.
        let mut ordinary = game("japlus");
        duel(&mut ordinary, 1_000);
        assert_eq!(ordinary.full_force_duels().0, 0);
        assert!(ordinary.duel_refuses(1, &lightning(0)));
        // The base game has no such command.
        let mut base = game("base");
        base.last_frame_time = 1_000;
        base.player_command(0, b"engage_FullForceDuel", 1_000);
        assert!(!base.peer(0).unwrap().full_force_duel);
    }
}
