//! JA+'s grappling hook on this server ([`sjk_game_jka::japlus_hook`]): the hook fired on
//! `+button12` after a player's move, flying as a missile, holding where it struck, the
//! fuel it draws, and letting go: on the key, on use, when the fuel is out, at death, a
//! spawn, a teleport or a disconnect, and when a blade or a shot cuts its rope.
//!
//! Only a mod with a hook ([`sjk_server_mod::ServerMod::hook`], JA+'s `jp_allowHook`) has
//! any of this; on the base game no player ever has a hook out, and every function here
//! returns at once. The pull and the hang are the shared movement's
//! (`ModMovement::grapple`), which the clients predict; this sets what it reads.

use super::*;
use sjk_game_jka::japlus_hook::{self as hook, Shooter};

/// `G_SetAnim`'s flags for the throw's pose.
const POSE_FLAGS: u8 =
    sjk_game_jka::pmove_anim::SETANIM_FLAG_OVERRIDE | sjk_game_jka::pmove_anim::SETANIM_FLAG_HOLD;
/// The bolt the rope leaves the hand from (`renderInfo.handRBolt`).
const HAND_BOLT: &str = "*r_hand";
/// The first entity number that is not a client's.
const MAX_CLIENTS: u16 = 32;

/// What a hook missile is to the player who fired it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HookMissile {
    /// The owner's place.
    pub(super) client: usize,
    /// The twin (`hook2`) rather than the hook.
    pub(super) twin: bool,
}

impl NativeGame {
    /// Whether any player has a hook or its twin out: the missiles' run looks for hooks
    /// and cutters only then.
    pub(super) fn any_hook_out(&self) -> bool {
        self.mod_hook.enabled()
            && (0..self.players.places()).any(|client| {
                self.peer(client)
                    .is_some_and(|peer| peer.hook.out() || peer.hook.twin.is_some())
            })
    }

    /// The player whose hook (or twin) entity `number` is.
    pub(super) fn hook_missile(&self, number: EntityId) -> Option<HookMissile> {
        (0..self.players.places()).find_map(|client| {
            let peer = self.peer(client)?;
            if peer.hook.entity == Some(number) {
                Some(HookMissile {
                    client,
                    twin: false,
                })
            } else if peer.hook.twin == Some(number) {
                Some(HookMissile { client, twin: true })
            } else {
                None
            }
        })
    }

    /// `JP_HookClientThink` (VisionJK `japlus/g_japlus_hook.c:296-359`) for player `client`
    /// after its move (`g_active.c:3403-3405`), with the buttons the move left: fire,
    /// hold, hang, pull again, let go.
    pub(super) fn hook_think(&mut self, client: usize, level_time: i32) {
        let rules = self.mod_hook;
        let (gametype, jedi_v_merc) = (self.gametype, self.cvars.integer(b"g_jediVmerc") != 0);
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        // A spectator thinks as one (`SpectatorThink` returns before the hook's think).
        if !(rules.enabled() || peer.hook.out()) || !peer.playing() || !peer.begun {
            return;
        }
        let buttons = peer.movement.moved_buttons();
        if !peer.hook.plugin && (peer.hook.out() || buttons & hook::BUTTON_HOOK != 0) {
            // Bots count as the newest plugin's users (`g_japlus_misc.c:688-689`).
            peer.hook.plugin = peer.bot
                || sjk_protocol::info_value(&peer.userinfo, b"cjp_client")
                    .is_some_and(hook::plugin_user);
        }
        let allowed = rules.allowed_here(gametype, jedi_v_merc, peer.state.true_non_jedi());
        let shooter = Shooter {
            health: peer.health,
            respawn_time: peer.mortality.respawn_time,
            gripped: peer.force.grip_being_gripped >= level_time as f32,
        };
        let fires =
            allowed && hook::can_fire(&peer.state, &peer.hook, rules, shooter, buttons, level_time);
        let before = watched(&peer.state);
        let think = hook::client_think(
            &mut peer.state,
            &mut peer.hook,
            buttons,
            allowed,
            fires,
            level_time,
        );
        if watched(&peer.state) != before {
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        if think.fire {
            self.fire_hook(client, level_time);
        }
        if think.free {
            self.free_hook_entities(client, level_time);
        }
    }

    /// `Weapon_GrapplingHook_Fire` (`g_japlus_hook.c:162-176`): the hook (and a plugin
    /// user's twin) from the muzzle, the throw's pose and its sound.
    fn fire_hook(&mut self, client: usize, level_time: i32) {
        let rules = self.mod_hook;
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let (missile, twin) = hook::fire(
            &peer.state,
            peer.entity.state(),
            rules,
            peer.hook.plugin,
            level_time,
        );
        peer.hook.base = missile.current;
        peer.hook.twin_free_at = 0;
        peer.movement.set_animation_parts(
            sjk_game_jka::pmove_anim::SETANIM_BOTH,
            hook::BOTH_RESISTPUSH,
            POSE_FLAGS,
        );
        peer.movement.write_player_state(&mut peer.state);
        let origin = peer.state.origin();
        for (missile, is_twin) in std::iter::once((missile, false)).chain(twin.map(|t| (t, true))) {
            let Some(id) = self.pool.spawn_entity(missile.state.clone(), level_time) else {
                continue;
            };
            self.pool.set_bounds(id, missile.bounds);
            self.missiles.push((id, missile));
            if let Some(peer) = self.peer_mut(client) {
                if is_twin {
                    peer.hook.twin = Some(id);
                } else {
                    peer.hook.entity = Some(id);
                }
            }
        }
        // `G_SoundOnEnt(ent, CHAN_ITEM, ...)`.
        let (sounds, told) = (&mut self.sounds, &mut self.told);
        let index = sounds.index(hook::FIRE_SOUND, &mut |index, value| {
            told.push(Told::ConfigString {
                index,
                previous: Vec::new(),
                value: value.to_vec(),
            })
        });
        let mut sound =
            sjk_game_jka::knockdown::entity_sound(origin, client as u16, hook::CHAN_ITEM);
        sound.parameter = u32::from(index);
        let _ = self.pool.spawn_temporary(sound.state(), level_time, None);
    }

    /// `Weapon_HookFree`'s entities (`g_japlus_hook.c:65-69`): the hook and its twin freed
    /// and forgotten, out of the missiles' flight.
    fn free_hook_entities(&mut self, client: usize, level_time: i32) {
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let ids = [peer.hook.entity.take(), peer.hook.twin.take()];
        peer.hook.holding = false;
        for id in ids.into_iter().flatten() {
            self.pool.free(id, level_time);
            self.hooks_freed.push(id);
        }
        self.missiles
            .retain(|(number, _)| !ids.contains(&Some(*number)));
    }

    /// `Weapon_HookFree` / `JP_HookFree` for player `client`: it lets go (no pull, no hang,
    /// no hook flag) and its hook is gone. At its death, its spawn, a teleport and its
    /// disconnect (VisionJK `g_combat.c:2111`, `g_client.c:3439,4118`, `g_misc.c:228`),
    /// when its fuel runs out and when its rope is cut.
    pub(super) fn free_hook(&mut self, client: usize, level_time: i32) {
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if !peer.hook.out() && peer.hook.twin.is_none() {
            return;
        }
        hook::release(&mut peer.state, &mut peer.hook);
        peer.movement = peer.movement.reseeded(&peer.state);
        self.free_hook_entities(client, level_time);
    }

    /// The hook's draw on player `client`'s fuel for this frame's batteries
    /// (`JP_HookFuelThink`, called by `G_RunFrame` for a player whose jetpack is off).
    pub(super) fn hook_fuel(&self, client: usize) -> Option<hook::HookFuel> {
        let peer = self.peer(client)?;
        if !peer.hook.out() {
            return None;
        }
        hook::fuel_draw(&peer.state, &peer.hook, self.mod_hook)
    }

    /// What a frame's run did to hook or twin `which` at entity `number`, run from
    /// `before`: its life over (`Weapon_HookFree` as the hook's think, `G_FreeEntity` as
    /// the twin's) or a surface nothing impacts on lets go; an impact is
    /// `G_MissileImpact`'s damage (the hook's one point, the twin's none) and then the
    /// hold ([`Self::hook_struck`]) instead of the explosion; a saber that kills it holds
    /// it where it met. `None` for a run the missiles' own handling takes (still flying,
    /// bounced off a blade). Whether it stays among the missiles.
    pub(super) fn hook_run(
        &mut self,
        number: EntityId,
        missile: &Missile,
        which: HookMissile,
        run: &MissileRun,
        before: EntityState,
        level_time: i32,
    ) -> Option<bool> {
        let target = match *run {
            MissileRun::Freed => {
                if which.twin {
                    self.pool.free(number, level_time);
                    if let Some(peer) = self.peer_mut(which.client) {
                        peer.hook.twin = None;
                    }
                } else {
                    self.free_hook(which.client, level_time);
                }
                return Some(false);
            }
            MissileRun::Missed => None,
            MissileRun::Hit(other) => {
                if !which.twin {
                    let _ = self.missile_hit(usize::from(other), missile, level_time);
                }
                Some(other)
            }
            MissileRun::HitEntity(other) => {
                if !which.twin {
                    self.hurt_thing(other, missile, level_time);
                }
                None
            }
            MissileRun::Blocked { struck, ref block } if block.outcome != Blocked::Bounced => {
                let _ = self
                    .pool
                    .spawn_temporary(block.flash.state(), level_time, None);
                if let Some(defender) = self.peer_mut(usize::from(struck)) {
                    let raised = defender.state.saber_blocked();
                    defender.movement.set_saber_blocked(raised);
                }
                hook::struck_body(&missile.state).then_some(struck)
            }
            _ => return None,
        };
        // The hook's owner may have died of its own hook's point.
        if self.hooks_freed.contains(&number) {
            return Some(false);
        }
        self.hook_struck(number, missile, which, target, before, level_time);
        Some(false)
    }

    /// `JP_HookMissileImpact` (`g_japlus_hook.c:411-466`) for the hook or twin `which` at
    /// entity `number`, which struck `target` (a player or NPC; `None` for the world or a
    /// thing) and was run from `before`: it holds where it struck, the middle of a body
    /// it struck, and the hook pulls its owner there.
    pub(super) fn hook_struck(
        &mut self,
        number: EntityId,
        missile: &Missile,
        which: HookMissile,
        target: Option<u16>,
        before: EntityState,
        level_time: i32,
    ) {
        let normal = hook::impact_normal(&missile.state);
        let point = target
            .and_then(|other| self.body_middle(other))
            .unwrap_or(missile.impact_point);
        let Some(peer) = self.peer_mut(which.client) else {
            self.pool.free(number, level_time);
            return;
        };
        let point = hook::snap_towards(point, peer.hook.base);
        let mut state = before;
        if which.twin {
            hook::stand_twin(&mut state, point);
            peer.hook.twin_free_at = missile.free_at;
            self.pool.set_state(number, &state);
            return;
        }
        hook::stand(&mut state, point);
        peer.hook.holding = true;
        peer.hook.origin = point;
        peer.hook.enemy = target;
        peer.hook.think_at = level_time + hook::FIRST_THINK;
        if peer.begun {
            hook::set_anchor(&mut peer.state, point);
            if !peer.hook.hooked {
                let flags = peer.state.movement_flags();
                peer.state
                    .set_movement_flags(flags | sjk_game_jka::pmove::grapple::PMF_GRAPPLE);
                peer.hook.hooked = true;
            }
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        self.pool.set_state(number, &state);
        let effect = hook::impact_effect(point, normal, target);
        let _ = self.pool.spawn_temporary(effect, level_time, None);
    }

    /// The middle of player or NPC `number`'s box where it stands (`JP_HookImpactPoint`).
    fn body_middle(&mut self, number: u16) -> Option<[f32; 3]> {
        if number < MAX_CLIENTS {
            let peer = self.peer(usize::from(number))?;
            let (mins, maxs) = peer.movement.box_bounds();
            return Some(hook::box_middle(peer.state.origin(), mins, maxs));
        }
        let npc = self
            .npcs
            .roster
            .actors
            .iter()
            .find(|npc| npc.number == number)?;
        let (mins, maxs) = npc.movement.box_bounds();
        Some(hook::box_middle(npc.player.origin(), mins, maxs))
    }

    /// `Weapon_HookThink` (`g_japlus_hook.c:84-108`) for every hook holding whose time has
    /// come, and the end of a holding twin's life: a hook on a body moves to its middle,
    /// the anchor follows, and it thinks again five minutes on; a spectator's lets go.
    pub(super) fn run_hooks(&mut self, level_time: i32) {
        if !self.mod_hook.enabled() {
            return;
        }
        for client in 0..self.players.places() {
            let Some(peer) = self.peer_mut(client) else {
                continue;
            };
            if let Some(twin) = peer.hook.twin
                && peer.hook.twin_free_at != 0
                && peer.hook.twin_free_at <= level_time
            {
                peer.hook.twin = None;
                self.pool.free(twin, level_time);
            }
            let Some(peer) = self.peer(client) else {
                continue;
            };
            let Some(id) = peer.hook.entity.filter(|_| peer.hook.holding) else {
                continue;
            };
            if peer.hook.think_at > level_time {
                continue;
            }
            if !(peer.begun && peer.playing()) {
                self.free_hook(client, level_time);
                continue;
            }
            let (old, enemy) = (peer.hook.origin, peer.hook.enemy);
            let alive = enemy.filter(|&other| {
                if other < MAX_CLIENTS {
                    self.peer(usize::from(other))
                        .is_some_and(|them| them.begun && them.playing())
                } else {
                    self.npcs.roster.is_npc(other)
                }
            });
            let moved = alive.and_then(|other| self.body_middle(other));
            let Some(peer) = self.peer_mut(client) else {
                continue;
            };
            if let Some(middle) = moved {
                let point = hook::snap_towards(middle, old);
                peer.hook.origin = point;
                if let Some(state) = self.pool.state_mut(id) {
                    hook::stand(state, point);
                }
            }
            let Some(peer) = self.peer_mut(client) else {
                continue;
            };
            let point = peer.hook.origin;
            hook::set_anchor(&mut peer.state, point);
            peer.hook.think_at = level_time + hook::HOLD_THINK;
            peer.movement = peer.movement.reseeded(&peer.state);
        }
    }

    /// `G_WeapCutGrappleRope` (`g_japlus_hook.c:186-238`): something of `cutter`'s moving
    /// from `from` to `to`, reaching `radius`, cuts the ropes it crosses, its owner's own
    /// too with `cut_own`. A cut rope's owner lets go, its fuel is spent, it waits 10
    /// seconds for a new hook and 3 for its jetpack, and a fall in the next 10 seconds is
    /// the cutter's.
    pub(super) fn cut_ropes(
        &mut self,
        cutter: u16,
        from: [f32; 3],
        to: [f32; 3],
        radius: f32,
        cut_own: bool,
        level_time: i32,
    ) {
        if !self.mod_hook.ropes_cut() {
            return;
        }
        let isolation = (cutter < MAX_CLIENTS)
            .then(|| self.duel_isolation(usize::from(cutter)))
            .flatten();
        for owner in 0..self.players.places() {
            let Some(peer) = self.peer(owner) else {
                continue;
            };
            let Some(id) = peer.hook.entity.filter(|_| peer.begun) else {
                continue;
            };
            let own = owner == usize::from(cutter);
            if own && !cut_own {
                continue;
            }
            // `G_canInterferWith`: the duel's isolation keeps others' blades and shots off.
            if !own
                && super::bridge_collision_gather::isolated_from(
                    isolation,
                    usize::from(cutter),
                    owner,
                    peer,
                )
            {
                continue;
            }
            let hook_point = if peer.hook.holding {
                peer.hook.origin
            } else {
                match self.missiles.iter().find(|(number, _)| *number == id) {
                    Some((_, missile)) => missile.current,
                    None => peer.hook.origin,
                }
            };
            let start = self.rope_start(owner, level_time);
            // JA+'s model scale is not built: every player is at 100 %.
            let Some((on_rope, on_cutter)) =
                hook::cut(start, hook_point, from, to, radius, own, 1.0)
            else {
                continue;
            };
            let flood = self.mod_hook.flood_protect;
            let Some(peer) = self.peer_mut(owner) else {
                continue;
            };
            peer.hook.hook_time = level_time - flood + hook::CUT_HOOK_DELAY;
            peer.state.set_raw_field(hook::PS_JETPACK_FUEL, 0);
            peer.gear.packs.jetpack_toggle_time = level_time + hook::CUT_TOGGLE_DELAY;
            peer.wounds.other_killer = sjk_game_jka::damage::OtherKiller {
                number: cutter,
                time: level_time + hook::CUT_KILLER_TIME,
                debounce_time: level_time + hook::CUT_KILLER_DEBOUNCE,
            };
            self.free_hook(owner, level_time);
            // `G_PlayEffectID(G_EffectIndex("blaster/deflect"), ...)` where the rope parted.
            let (effects, told) = (&mut self.map_effects.effects, &mut self.told);
            let effect = effects.index(hook::CUT_EFFECT, &mut |index, value| {
                told.push(Told::ConfigString {
                    index,
                    previous: Vec::new(),
                    value: value.to_vec(),
                })
            });
            let direction = std::array::from_fn(|axis| on_cutter[axis] - on_rope[axis]);
            let event = hook::cut_effect(effect, on_rope, direction);
            let _ = self.pool.spawn_temporary(event.state(), level_time, None);
        }
    }

    /// Where player `client`'s rope leaves its hand: its right hand's bolt on its posed
    /// model, turned by the whole view (`G2API_GetBoltMatrix(..., ps.viewangles, ...)`),
    /// else where it stands.
    fn rope_start(&mut self, client: usize, level_time: i32) -> [f32; 3] {
        let Some(peer) = self.peer_mut(client) else {
            return [0.0; 3];
        };
        let (angles, origin) = (peer.state.view_angles(), peer.state.origin());
        peer.skeleton
            .as_mut()
            .and_then(|skeleton| {
                skeleton
                    .skeleton
                    .bolt_matrix_turned(&skeleton.models, HAND_BOLT, angles, origin, level_time)
                    .ok()
                    .flatten()
            })
            .map_or(origin, |matrix| [matrix[0][3], matrix[1][3], matrix[2][3]])
    }

    /// The ropes this frame's shots crossed (`G_RunMissile` cuts with every missile that is
    /// no hook, `g_missile.c:897-900`), once the missiles have run.
    pub(super) fn cut_ropes_with_shots(&mut self, level_time: i32) {
        if self.rope_cutters.is_empty() {
            return;
        }
        let mut cutters = std::mem::take(&mut self.rope_cutters);
        for &(owner, from, to) in &cutters {
            self.cut_ropes(owner, from, to, hook::MISSILE_CUT_RADIUS, true, level_time);
        }
        cutters.clear();
        self.rope_cutters = cutters;
    }
}

/// The player state fields the hook's think may change, which the movement reads.
fn watched(state: &sjk_protocol::PlayerState) -> (u16, u32, u32) {
    (
        state.movement_flags(),
        state.entity_flags(),
        hook::entity_flags2(state),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::Identity;
    use crate::cvars::Cvars;
    use sjk_game_jka::pmove::grapple::{EF_ROPE_HANG, PMF_GRAPPLE};

    fn server(settings: &[(&str, &str)]) -> NativeGame {
        let mut cvars = Cvars::new();
        for (name, value) in settings {
            cvars.set(name.as_bytes(), value.as_bytes());
        }
        cvars.set(b"g_userinfoValidate", b"0");
        cvars.register_server();
        let identity = Identity {
            hostname: b"test".to_vec(),
            mapname: b"mp/ffa3".to_vec(),
        };
        NativeGame::with_cvars(identity, None, 4, 4, cvars).unwrap()
    }

    /// A mapless free-for-all whose start had `g_mod` set to `server_mod`: player 0 at
    /// x 500 facing player 1 at x 600, both with full fuel, past the spawn's grace.
    /// Player 0 uses the client plugin when `plugin`.
    fn game(server_mod: &str, plugin: bool) -> NativeGame {
        let mut game = server(&[("g_mod", server_mod)]);
        for (client, origin, yaw) in [(0, [500.0, 0.0, 0.0], 0.0), (1, [600.0, 0.0, 0.0], 180.0)] {
            let userinfo = if plugin && client == 0 {
                format!("\\name\\P{client}\\cjp_client\\1.4 SJK")
            } else {
                format!("\\name\\P{client}")
            };
            game.connect_client(client, userinfo.as_bytes(), false)
                .unwrap();
            game.begin(client, 0, None);
            // Playing (`TEAM_FREE`), not watching: begun again on the team.
            game.peer_mut(client).unwrap().session.team = 0;
            game.begin(client, 0, None);
            let peer = game.peer_mut(client).unwrap();
            peer.state.set_origin(origin);
            peer.state.set_view_angles([0.0, yaw, 0.0]);
            peer.state.set_raw_field(hook::PS_JETPACK_FUEL, 100);
            peer.mortality.respawn_time = 0;
            // Past the spawn's protection.
            peer.invulnerable_until = 0;
            let flags = peer.state.raw_field(crate::peer::EFLAGS).unwrap_or(0);
            peer.state
                .set_raw_field(crate::peer::EFLAGS, flags & !EF_INVULNERABLE);
            peer.health = 50;
            peer.state.stats[0] = 50;
            peer.state.set_command_time(2_000);
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        // Each moves once, so that the others meet it where it stands.
        think_as(&mut game, 1, 2_008, 0);
        game
    }

    fn think_as(game: &mut NativeGame, client: usize, time: i32, buttons: u16) {
        game.last_frame_time = time - 8;
        let command = UserCommand {
            server_time: time,
            buttons,
            ..UserCommand::default()
        };
        game.player_think(client, &command, time);
    }

    fn think(game: &mut NativeGame, time: i32, buttons: u16) {
        think_as(game, 0, time, buttons);
    }

    fn flags(game: &NativeGame) -> (bool, bool, bool) {
        let state = &game.peer(0).unwrap().state;
        (
            state.movement_flags() & PMF_GRAPPLE != 0,
            state.entity_flags() & EF_ROPE_HANG != 0,
            hook::entity_flags2(state) & hook::EF2_HOOK != 0,
        )
    }

    #[test]
    fn the_base_game_has_no_hook() {
        let mut game = game("base", true);
        assert!(!game.mod_hook.enabled());
        think(&mut game, 2_008, hook::BUTTON_HOOK);
        assert!(!game.peer(0).unwrap().hook.out());
        assert_eq!(flags(&game), (false, false, false));
        assert!(game.missiles.is_empty());
        assert_eq!(game.movement_config().grapple, None);
    }

    #[test]
    fn jp_allow_hook_0_has_no_hook() {
        let game = server(&[("g_mod", "japlus"), ("jp_allowHook", "0")]);
        assert!(!game.mod_hook.enabled());
    }

    #[test]
    fn the_hook_flies_holds_a_player_pulls_hangs_and_lets_go() {
        let mut game = game("japlus", true);
        assert!(game.mod_hook.enabled());
        assert!(game.movement_config().grapple.is_some());
        think(&mut game, 2_008, hook::BUTTON_HOOK);
        let peer = game.peer(0).unwrap();
        assert!(peer.hook.out() && peer.hook.twin.is_some() && peer.hook.plugin);
        assert_eq!(flags(&game), (false, false, true));
        // The hook and its twin fly.
        assert_eq!(game.missiles.len(), 2);
        let fuel = hook::fuel(&game.peer(0).unwrap().state);
        // It flies the 100 units to player 1 and holds the middle of its box.
        for time in [2_050, 2_100, 2_150, 2_200] {
            game.run_frame(time);
        }
        let peer = game.peer(0).unwrap();
        assert!(peer.hook.holding, "{:?}", peer.hook);
        assert_eq!(peer.hook.enemy, Some(1));
        assert!(game.missiles.is_empty());
        assert_eq!(flags(&game), (true, false, true));
        let anchor = [102, 105, 100]
            .map(|index| f32::from_bits(game.peer(0).unwrap().state.raw_field(index).unwrap()));
        assert_eq!(anchor[0], 600.0, "{anchor:?}");
        // Its one point of damage, and the fuel it drew flying out.
        assert_eq!(game.peer(1).unwrap().health, 49);
        assert!(hook::fuel(&game.peer(0).unwrap().state) < fuel);
        // Held, the key pulls towards the anchor, 16 units short of it along the view.
        think(&mut game, 2_208, hook::BUTTON_HOOK);
        let origin = game.peer(0).unwrap().state.origin();
        assert!((550.0..584.0).contains(&origin[0]), "{origin:?}");
        // Let go: a plugin user hangs on the rope; the key again pulls again.
        think(&mut game, 2_216, 0);
        assert_eq!(flags(&game), (false, true, true));
        think(&mut game, 2_224, hook::BUTTON_HOOK);
        assert_eq!(flags(&game), (true, false, true));
        // Use lets go after the move, which was still pulled.
        think(&mut game, 2_232, hook::BUTTON_HOOK | hook::BUTTON_USE);
        assert!(!game.peer(0).unwrap().hook.out());
        assert_eq!(flags(&game), (false, false, false));
        assert!(game.peer(0).unwrap().hook.twin.is_none());
    }

    #[test]
    fn without_the_plugin_the_key_let_go_drops_the_hook() {
        let mut game = game("japlus", false);
        think(&mut game, 2_008, hook::BUTTON_HOOK);
        let peer = game.peer(0).unwrap();
        assert!(peer.hook.out() && peer.hook.twin.is_none() && !peer.hook.plugin);
        assert_eq!(game.missiles.len(), 1);
        think(&mut game, 2_016, 0);
        assert!(!game.peer(0).unwrap().hook.out());
        assert!(game.missiles.is_empty());
        assert_eq!(flags(&game), (false, false, false));
        // The flood protection: no new hook for 750 ms after the last.
        think(&mut game, 2_024, hook::BUTTON_HOOK);
        assert!(!game.peer(0).unwrap().hook.out());
        think(&mut game, 2_800, hook::BUTTON_HOOK);
        assert!(game.peer(0).unwrap().hook.out());
    }

    #[test]
    fn a_shot_cuts_the_rope() {
        let mut game = game("japlus", false);
        think(&mut game, 2_008, hook::BUTTON_HOOK);
        for time in [2_050, 2_100, 2_150, 2_200] {
            game.run_frame(time);
            // The key held, so the hook stays out.
            think(&mut game, time + 8, hook::BUTTON_HOOK);
        }
        assert!(game.peer(0).unwrap().hook.holding);
        // Player 1's shot across the rope, halfway along it.
        let start = game.rope_start(0, 2_208);
        let hold = game.peer(0).unwrap().hook.origin;
        let middle: [f32; 3] = std::array::from_fn(|axis| (start[axis] + hold[axis]) * 0.5);
        let from = [middle[0], middle[1] - 50.0, middle[2]];
        let to = [middle[0], middle[1] + 50.0, middle[2]];
        game.cut_ropes(1, from, to, hook::MISSILE_CUT_RADIUS, true, 2_210);
        let peer = game.peer(0).unwrap();
        assert!(!peer.hook.out());
        assert_eq!(hook::fuel(&peer.state), 0);
        assert_eq!(peer.wounds.other_killer.number, 1);
        assert_eq!(flags(&game), (false, false, false));
    }

    #[test]
    fn the_watched_fields_see_the_hooks_flags() {
        let mut state = sjk_protocol::PlayerState::default();
        let before = watched(&state);
        state.set_raw_field(103, hook::EF2_HOOK);
        assert_ne!(watched(&state), before);
    }
}
