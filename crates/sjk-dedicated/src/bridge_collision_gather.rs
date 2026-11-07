//! Reused collision and splash scratch for the level's entities.
use super::*;

/// Whether `peer`, player `other`, is left out of player `client`'s traces, `mine` being
/// [`NativeGame::duel_isolation`] for `client`. `G_TempUnlink` leaves out the living only
/// (VisionJK `g_japlus_dim.c:150-152`).
pub(super) fn isolated_from(
    mine: Option<Option<u16>>,
    client: usize,
    other: usize,
    peer: &Peer,
) -> bool {
    mine.is_some_and(|mine| {
        peer.health >= 1
            && peer.state.stats[0] as i32 >= 1
            && sjk_game_jka::duel::isolated(
                mine,
                other as u16,
                peer.state.duel_in_progress(),
                usize::from(peer.state.duel_index()) == client,
            )
    })
}

impl NativeGame {
    /// The duel isolation for player `client`'s traces: `None` when there is none (the
    /// base game, the setting off, another game type, an NPC or no one), else the
    /// opponent it duels, if any ([`isolated_from`]).
    pub(super) fn duel_isolation(&self, client: usize) -> Option<Option<u16>> {
        if !self.duel_rules.isolates(self.gametype) {
            return None;
        }
        self.peer(client)
            .map(|peer| sjk_game_jka::duel::duel_of(&peer.state))
    }

    /// Everyone `client` can run into: the other playing peers, each a body in the box of
    /// its last move, where it last thought (`r.currentOrigin`). In the players' order; the
    /// reference's order is its sector tree's, which only decides between stops at the
    /// very same fraction.
    ///
    /// With a server mod that isolates private duels (JA+'s `jp_DuelAlpha` 0 or more,
    /// `sjk_game_jka::duel::DuelRules::isolates`), the players `client` may not touch are
    /// left out: every trace and move of a player passes through them, as JA+'s
    /// `JP_Trace` with `G_TempUnlink`'s list (VisionJK `g_japlus_dim.c:137-188`,
    /// `g_active.c:3100-3102`), and its missiles fly through them (`g_missile.c:908-924`).
    pub(super) fn gather_obstacles(&mut self, client: usize) {
        self.gather_obstacles_isolated(client, true);
    }

    /// [`Self::gather_obstacles`], leaving out the players `client` may not touch only
    /// with `isolate` (the duel challenge's line, a plain `trap->Trace`, does not).
    pub(super) fn gather_obstacles_isolated(&mut self, client: usize, isolate: bool) {
        let mine = if isolate {
            self.duel_isolation(client)
        } else {
            None
        };
        let Self {
            server,
            world,
            players,
            obstacles,
            body_legs,
            missiles,
            husks,
            pool,
            charges,
            breakables,
            doors,
            last_frame_time,
            npcs,
            map_turrets,
            stock,
            mounted,
            scripts,
            usable_entities,
            ..
        } = self;
        obstacles.clear();
        body_legs.clear();
        let Some(world) = server.world(*world) else {
            return;
        };
        body_legs.extend(players.holders().map(|handle| {
            handle
                .and_then(|handle| world.entity(handle))
                .filter(|peer| peer.begun && peer.body_active())
                .map(|peer| peer.state.leg_animation())
        }));
        for (other, handle) in players.holders().enumerate() {
            // A client that has not begun has no entity in the world yet; what `client`
            // owns (the gun it works) its traces pass.
            let peer = handle
                .and_then(|handle| world.entity(handle))
                .filter(|peer| {
                    other != client
                        && peer.begun
                        && peer.body_active()
                        && usize::from(peer.riding.owner) != client
                        && !isolated_from(mine, client, other, peer)
                });
            if let Some(peer) = peer {
                let (mut bounds, mut contents) = (peer.movement.box_bounds(), peer.riding.contents);
                if let Some((corpse_contents, top)) = peer.corpse {
                    (bounds.1[2], contents) = (top, corpse_contents);
                }
                obstacles.push(BoxObstacle {
                    entity: other as u16,
                    origin: peer.state.origin(),
                    bounds,
                    contents,
                    model: None,
                });
            }
        }
        // The solid missiles in flight (a rocket) and their husks, linked until their
        // event's time is up — but the ones `client` owns, which its traces pass
        // (`SV_ClipMoveToEntities`).
        for (number, missile) in missiles.iter() {
            if missile.contents != 0 && missile.linked && usize::from(missile.owner) != client {
                obstacles.push(BoxObstacle {
                    entity: number.legacy_number(),
                    origin: missile.current,
                    bounds: missile.bounds,
                    contents: missile.contents,
                    model: None,
                });
            }
        }
        // The brushes, each its own model: breakables and doors and lifts.
        obstacles.extend(
            crate::collision::brush_obstacles(breakables, doors, *last_frame_time)
                .chain(stock.obstacles())
                .chain(scripts.brushes())
                .chain(crate::collision::usable_obstacles(usable_entities)),
        );
        map_turrets.add_obstacles(obstacles, client);
        husks.retain(|(_, _, id)| pool.is_linked(*id));
        obstacles.extend(
            husks
                .iter()
                .filter(|(_, owner, _)| usize::from(*owner) != client)
                .map(|(husk, ..)| *husk),
        );
        // A charge is passed by its owner while it flies; stuck (`SVF_OWNERNOTSHARED`), it
        // is solid to its owner too.
        obstacles.extend(
            charges
                .iter()
                .filter(|(_, charge)| {
                    usize::from(charge.missile.owner) != client || charge.owner_not_shared
                })
                .map(|(number, charge)| BoxObstacle {
                    entity: number.legacy_number(),
                    origin: charge.missile.current,
                    bounds: charge.missile.bounds,
                    contents: charge.missile.contents,
                    model: None,
                }),
        );
        Self::add_npc_bodies(npcs, obstacles, body_legs, client);
        obstacles.extend(
            mounted.obstacles(
                client as u16,
                players
                    .at(client)
                    .and_then(|handle| world.entity(handle))
                    .map_or(sjk_protocol::ENTITY_NUMBER_NONE, |peer| peer.riding.owner),
            ),
        );
        // The E-Webs, but the one `client` owns or works (`bridge_eweb`).
        let ewebs: Vec<_> = self.ewebs.obstacles(client as u16).collect();
        self.obstacles.extend(ewebs);
        // The solid siege items (`bridge_siege_items`).
        let items = self.siege_item_obstacles();
        self.obstacles.extend(items);
    }

    /// The players and NPCs a blast may reach, with their linked boxes.
    pub(super) fn gather_splash_targets(&mut self) {
        let Self {
            server,
            world,
            players,
            splash_targets,
            npcs,
            map_turrets,
            mounted,
            ..
        } = self;
        splash_targets.clear();
        if let Some(world) = server.world(*world) {
            for (number, handle) in players.holders().enumerate() {
                let peer = handle
                    .and_then(|handle| world.entity(handle))
                    .filter(|peer| peer.begun && peer.body_active());
                if let Some(peer) = peer {
                    let (bottom, mut top) = peer.movement.box_bounds();
                    if let Some((_, corpse_top)) = peer.corpse {
                        top[2] = corpse_top;
                    }
                    let origin = peer.state.origin();
                    let bounds = (
                        std::array::from_fn(|axis| origin[axis] + bottom[axis] - 1.0),
                        std::array::from_fn(|axis| origin[axis] + top[axis] + 1.0),
                    );
                    splash_targets.push(SplashTarget {
                        number: number as u16,
                        bounds,
                        origin,
                        takes_damage: true,
                    });
                }
            }
        }
        Self::add_npc_splash_targets(npcs, splash_targets);
        map_turrets.add_splash_targets(splash_targets);
        splash_targets.extend(mounted.splash_targets());
        splash_targets.extend(self.ewebs.splash_targets());
        splash_targets.extend(self.stock.shields.splash_targets());
    }
}
