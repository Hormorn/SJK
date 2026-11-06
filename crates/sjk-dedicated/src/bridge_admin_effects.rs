//! The admin system's commands on players and on the server (`bridge_admin` holds the
//! ranks and the login): kicks aside, the teleports, slap, sleep, protection, silence,
//! the admins' chat and centre prints, the lists, the map, renames, the team locks and
//! moves, the votes and the message of the day, with what they leave on a player
//! ([`AdminEffects`]) and the once-a-second upkeep that keeps them going.
//!
//! The behaviour is VisionJK's port of the JA+ 2.4 B7 module
//! (`codemp/game/japlus/g_japlus_fx.c`, `g_japlus_tele.c`, `g_japlus_admin.c`,
//! `g_japlus_admin2.c`), the code SJK's; every text is the mod's ([`AdminSystem`]).

use super::bridge_admin::Caller;
use super::{NativeGame, Told};
use crate::peer::EFLAGS;
use sjk_server_mod::admin::{
    self as rules, AdminCommand, AdminEntry, AdminSystem, EffectTexts, Rank, TeamAsked, TeamLock,
};

/// `forceHandExtend`, `forceDodgeAnim`, `HANDEXTEND_KNOCKDOWN`, `EF_INVULNERABLE`,
/// `STAT_HEALTH`.
const PS_FORCE_HAND_EXTEND: usize = 80;
const PS_FORCE_DODGE_ANIM: usize = 89;
const HANDEXTEND_KNOCKDOWN: u32 = sjk_game_jka::pmove_hand_extend::HANDEXTEND_KNOCKDOWN as u32;
const EF_INVULNERABLE: u32 = 1 << 27;
const STAT_HEALTH: usize = 0;
/// `GT_FFA`, and the game types `ammap` refuses: holocron, Jedi Master, single player.
const GT_FFA: i32 = 0;
const AMMAP_REFUSED: [i32; 3] = [1, 2, 5];
/// The seconds a cloaked player waits before a teleport beams it (`JP_STARTREK_DELAY`).
const STAR_TREK_SECONDS: i32 = 3;
/// `MAX_NETNAME` and the chat's `MAX_SAY_TEXT`, ends included.
const NAME_BYTES: usize = 35;
const SAY_BYTES: usize = 149;
/// `CONTENTS_SOLID | LAVA | WATER | TERRAIN | SLIME`, the lava check's trace mask, and
/// what it looks for; `CONTENTS_SOLID | TERRAIN` for the trigger check's.
const LAVA_TRACE: u32 = 0x21007;
const LAVA_OR_SLIME: u32 = 0x2 | 0x20000;
const GROUND_TRACE: u32 = 0x1001;
/// The chat's escape character, around a name (`EC`).
const ESCAPE: u8 = 0x19;
/// The names of `jp_teamLock`'s bits 1 to 4, which `amlockteam` takes by first letter.
const TEAM_NAMES: [&str; 4] = ["spectator", "free", "blue", "red"];
/// `JP_PLUGIN_VERSION`: the JA+ plugin a bot is listed with.
const BOT_PLUGIN: &[u8] = b"1.4B4";

/// What the admin commands left on a player (`pers.jp`): ended with the level.
#[derive(Clone, Debug, Default)]
pub(crate) struct AdminEffects {
    /// `amsilence`: no `say`, `say_team`, `tell`, `amsay`, and no rename.
    pub(crate) silenced: bool,
    /// `amsleep`: lying, and hurt by nothing that respects god mode.
    pub(crate) sleeping: bool,
    /// `amprotect`: no other may hurt it, until it attacks.
    pub(crate) protected: bool,
    /// `amdenyvote`: no `callvote`.
    pub(crate) deny_vote: bool,
    /// `amtelemark`'s place and yaw.
    tele_mark: Option<([f32; 3], f32)>,
    /// No `amslap` on it before this level time (`slapTime`).
    slap_until: i32,
    /// The message of the day shown, and the seconds it has left.
    motd: Option<(Vec<u8>, i32)>,
    /// A teleport waiting under the cloak (`jp_starTrekTeleport`).
    star_trek: Option<StarTrek>,
    /// When the once-a-second upkeep runs next.
    next_second: i32,
}

/// `stCounter`, `stDone` and where the beam goes.
#[derive(Clone, Copy, Debug)]
struct StarTrek {
    counter: i32,
    done: bool,
    origin: [f32; 3],
    angles: [f32; 3],
}

/// One effect command's call (`jpFxCall_t`): whom it is for.
#[derive(Clone, Copy, Debug)]
struct FxCall {
    /// A client number, -1 for everybody, -2 for nobody.
    target: i32,
    single: bool,
    /// A player of the rcon rank ran it: nobody else is told.
    silent: bool,
}

/// What one of the commands on every player does to one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fx {
    Sleep,
    Wake,
    Silence,
    Unsilence,
}

/// `vtos`: `(x y z)` in whole units.
fn vtos(point: [f32; 3]) -> String {
    format!(
        "({} {} {})",
        point[0] as i32, point[1] as i32, point[2] as i32
    )
}

/// `ConcatArgs(start)`: the words from `start` on, a space between them, cut to `limit`.
fn concat(words: &[&[u8]], start: usize, limit: usize) -> Vec<u8> {
    let mut text = words.get(start..).unwrap_or_default().join(&b' ');
    text.truncate(limit);
    text
}

/// A `cp "..."` command.
fn centre_print(text: &[u8]) -> Vec<u8> {
    [&b"cp \""[..], text, b"\""].concat()
}

/// `text` (a fixed text, one `%s`) with `word` in it.
fn with_word(text: &str, word: &[u8]) -> Vec<u8> {
    rules::format_name(text.as_bytes(), word)
}

/// `text` with its `%i`s filled in, in order.
fn with_numbers(text: &str, numbers: &[i32]) -> Vec<u8> {
    let mut out = text.to_owned();
    for number in numbers {
        if let Some(at) = out.find("%i") {
            out.replace_range(at..at + 2, &number.to_string());
        }
    }
    out.into_bytes()
}

impl NativeGame {
    /// The player's name, as the game keeps it.
    fn admin_name(&self, client: usize) -> Vec<u8> {
        self.peer(client)
            .map(|peer| peer.name.clone())
            .unwrap_or_default()
    }

    /// The text a setting holds.
    fn admin_text(&self, name: &str) -> Vec<u8> {
        if name.is_empty() {
            return Vec::new();
        }
        self.cvars.string(name.as_bytes()).to_vec()
    }

    /// One of the commands this module carries out; `false` for another.
    pub(super) fn run_admin_effect(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        command: AdminCommand,
        words: &[&[u8]],
    ) -> bool {
        let Some(system) = self.admin_system() else {
            return false;
        };
        match command {
            AdminCommand::Protect => self.amprotect(caller, entry, words, system),
            AdminCommand::Sleep => self.all_players_fx(caller, entry, words, Fx::Sleep, system),
            AdminCommand::Wake => self.all_players_fx(caller, entry, words, Fx::Wake, system),
            AdminCommand::Silence => self.all_players_fx(caller, entry, words, Fx::Silence, system),
            AdminCommand::Unsilence => {
                self.all_players_fx(caller, entry, words, Fx::Unsilence, system)
            }
            AdminCommand::DenyVote => self.amdenyvote(caller, entry, words, system),
            AdminCommand::Slap => self.amslap(caller, entry, words, system),
            AdminCommand::Tele => self.amtele(caller, entry, words, system),
            AdminCommand::TeleMark => self.amtelemark(caller, entry, system),
            AdminCommand::Origin => self.amorigin(caller, entry, words),
            AdminCommand::RefuseTele => self.refuse_tele(caller, system),
            AdminCommand::Say => self.amsay(caller, words, system),
            AdminCommand::PSay => self.ampsay(caller, entry, words),
            AdminCommand::Status => self.amstatus(caller, entry, system),
            AdminCommand::Whois => self.amwhois(caller, entry, system),
            AdminCommand::Map => self.ammap(caller, entry, words, system),
            AdminCommand::Rename => self.amrename(caller, entry, words),
            AdminCommand::LockTeam => self.amlockteam(caller, entry, words, system),
            AdminCommand::ForceTeam => self.amforceteam(caller, entry, words, system),
            AdminCommand::KillVote => self.amkillvote(caller, entry, system),
            AdminCommand::Motd => {
                if let Caller::Player(client) = caller {
                    self.admin_set_motd(client, system);
                }
            }
            AdminCommand::ShowMotd => self.amshowmotd(caller, entry, words, system),
            _ => return false,
        }
        true
    }

    /// `G_AdminCSDisplay`: a centre print for everyone in the game, `target_text` for the
    /// target and `broadcast` (its `%s` the target's name) for the others.
    fn admin_display(&mut self, target: usize, target_text: &[u8], broadcast: &[u8]) {
        let name = self.admin_name(target);
        for client in 0..self.players.places() {
            if self.admin_valid(client as i32).is_none() {
                continue;
            }
            let text = if client == target {
                target_text.to_vec()
            } else {
                rules::format_name(broadcast, &name)
            };
            let text = rules::new_lines(&text);
            self.told.push(Told::One(
                client,
                centre_print(&[&b"^7"[..], &text, b"\n"].concat()),
            ));
        }
    }

    /// `JP_FxAnnounce`: an effect's texts, `off` for its undoing.
    fn admin_announce(&mut self, call: FxCall, texts: Option<&EffectTexts>, off: bool) {
        let (Some(texts), Ok(target)) = (texts, usize::try_from(call.target)) else {
            return;
        };
        let (target_text, broadcast) = if off {
            (texts.target_off, texts.broadcast_off)
        } else {
            (texts.target, texts.broadcast)
        };
        let target_text = self.admin_text(target_text);
        let broadcast = if call.silent {
            Vec::new()
        } else {
            self.admin_text(broadcast)
        };
        self.admin_display(target, &target_text, &broadcast);
    }

    /// `JP_FxDisallowedInGametype`: the effects work in a free-for-all without Jedi
    /// against mercenaries only; refused (and told) elsewhere, unless the command ends
    /// for want of a player first.
    fn admin_fx_refused_here(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        self_by_default: bool,
        system: &AdminSystem,
    ) -> bool {
        if words.len() < 2 && (!self_by_default || caller == Caller::Console) {
            return false;
        }
        if self.gametype == GT_FFA && self.cvars.integer(b"g_jediVmerc") == 0 {
            return false;
        }
        self.admin_reply(
            caller,
            &with_word(system.texts.disallowed_gametype, entry.name.as_bytes()),
        );
        true
    }

    /// `JP_FxBegin`: whom the command is for; without a player, the caller itself where
    /// `self_by_default`.
    fn admin_fx_begin(
        &self,
        caller: Caller,
        words: &[&[u8]],
        self_by_default: bool,
    ) -> Option<FxCall> {
        let target = match (words.get(1), caller) {
            (Some(word), _) => self.admin_client_for(word),
            (None, Caller::Player(client)) if self_by_default => client as i32,
            (None, _) => return None,
        };
        let silent =
            matches!(caller, Caller::Player(client) if self.admin_rank(client) == Some(Rank::Rcon));
        Some(FxCall {
            target,
            single: target != -1,
            silent,
        })
    }

    /// `JP_FxSingleTarget`: the one player the command is for, once the rank allows the
    /// command and allows it on that player.
    fn admin_fx_single(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        call: FxCall,
        abuse: bool,
    ) -> Option<usize> {
        if !self.admin_allowed(caller, entry, true) {
            return None;
        }
        let target = self.admin_valid(call.target)?;
        self.admin_allowed_on(caller, target, true, abuse)
            .then_some(target)
    }

    /// `amprotect [player]` (`JP_ToggleFxCmd` with `jpFxProtect`): protected from the
    /// others until it attacks, or no longer.
    fn amprotect(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        if self.admin_fx_refused_here(caller, entry, words, true, system) {
            return;
        }
        let Some(call) = self.admin_fx_begin(caller, words, true) else {
            return;
        };
        let Some(target) = self.admin_fx_single(caller, entry, call, false) else {
            return;
        };
        if let Caller::Player(client) = caller
            && self.admin_abused(client, true)
        {
            return;
        }
        let Some(peer) = self.peer_mut(target) else {
            return;
        };
        let on = !peer.admin.effects.protected;
        peer.admin.effects.protected = on;
        let flags = peer.state.raw_field(EFLAGS).unwrap_or(0);
        peer.state.set_raw_field(
            EFLAGS,
            if on {
                flags | EF_INVULNERABLE
            } else {
                flags & !EF_INVULNERABLE
            },
        );
        self.admin_announce(call, system.effect(AdminCommand::Protect), !on);
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amsleep`, `amwake`, `amsilence`, `amunsilence` `<player|-1>` (`JP_AllPlayersFx`).
    fn all_players_fx(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        fx: Fx,
        system: &AdminSystem,
    ) {
        if fx == Fx::Sleep && self.admin_fx_refused_here(caller, entry, words, false, system) {
            return;
        }
        let Some(call) = self.admin_fx_begin(caller, words, false) else {
            return;
        };
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let places = self.players.places() as i32;
        if call.target < -1 || call.target >= places {
            return;
        }
        if fx == Fx::Sleep
            && let Caller::Player(client) = caller
            && self.admin_abused(client, true)
        {
            return;
        }
        let command = match fx {
            Fx::Sleep => AdminCommand::Sleep,
            Fx::Wake => AdminCommand::Wake,
            Fx::Silence => AdminCommand::Silence,
            Fx::Unsilence => AdminCommand::Unsilence,
        };
        let texts = system.effect(command);
        let (first, last) = if call.single {
            (call.target, call.target)
        } else {
            (0, places - 1)
        };
        let now = self.last_frame_time;
        for client in first..=last {
            let Some(target) = self.admin_valid(client) else {
                continue;
            };
            if caller != Caller::Console && !self.admin_allowed_on(caller, target, false, false) {
                continue;
            }
            let Some(peer) = self.peer_mut(target) else {
                continue;
            };
            let effects = &mut peer.admin.effects;
            let applied = match fx {
                Fx::Sleep => {
                    // `G_GoInSleep`: down, and kept down every second.
                    peer.state
                        .set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_KNOCKDOWN);
                    peer.state.set_raw_field(PS_FORCE_DODGE_ANIM, 0);
                    peer.knockdown.hand_extend_time = now + 1_200;
                    peer.movement = peer.movement.reseeded(&peer.state);
                    effects.sleeping = true;
                    true
                }
                Fx::Wake if effects.sleeping => {
                    effects.sleeping = false;
                    peer.state.set_raw_field(PS_FORCE_DODGE_ANIM, 1);
                    peer.movement = peer.movement.reseeded(&peer.state);
                    true
                }
                Fx::Wake => false,
                Fx::Silence => {
                    effects.silenced = true;
                    true
                }
                Fx::Unsilence => {
                    effects.silenced = false;
                    true
                }
            };
            if applied && call.single {
                self.admin_announce(call, texts, false);
            }
        }
        if !call.single
            && let Some(texts) = texts
        {
            let text = rules::new_lines(&self.admin_text(texts.all));
            self.told.push(Told::Everyone(centre_print(
                &[&b"^7"[..], &text, b"\n"].concat(),
            )));
        }
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amdenyvote <player>`: its right to call votes taken, or given back.
    fn amdenyvote(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        let Some(call) = self.admin_fx_begin(caller, words, false) else {
            return;
        };
        let Some(target) = self.admin_fx_single(caller, entry, call, false) else {
            return;
        };
        let Some(peer) = self.peer_mut(target) else {
            return;
        };
        let denied = !peer.admin.effects.deny_vote;
        peer.admin.effects.deny_vote = denied;
        self.admin_announce(call, system.effect(AdminCommand::DenyVote), !denied);
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amslap <player>` (`Cmd_amslap_f`): knocked down and thrown in a random
    /// direction, at most once in 15 seconds a player (but by the rcon rank).
    fn amslap(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        if self.admin_fx_refused_here(caller, entry, words, false, system) {
            return;
        }
        let Some(call) = self.admin_fx_begin(caller, words, false) else {
            return;
        };
        let Some(target) = self.admin_fx_single(caller, entry, call, true) else {
            return;
        };
        let now = self.last_frame_time;
        let flooding = match caller {
            Caller::Console => true,
            Caller::Player(client) => self
                .admin_rank(client)
                .is_some_and(|rank| rank != Rank::Rcon),
        };
        let until = self
            .peer(target)
            .map_or(0, |peer| peer.admin.effects.slap_until);
        if flooding && now < until {
            self.admin_reply(
                caller,
                &with_numbers(system.texts.slap_flood, &[(until - now) / 1000]),
            );
            return;
        }
        // `flrand(0, 2 pi)`, then the direction away from it. The original cuts the
        // decimals off the length (a hair below or above 10): the slap is 175 or 200.
        let angle = self.deaths.rng.flrand(0.0, 2.0 * std::f32::consts::PI);
        let sound = self.deaths.rng.irand(1, 4);
        let flat = [angle.cos() * 10.0, angle.sin() * 10.0];
        let length = (f64::from(flat[0]) * f64::from(flat[0])
            + f64::from(flat[1]) * f64::from(flat[1]))
        .sqrt();
        let strength = ((length as i32) as f64 * 0.8) as i32 * 25;
        let norm = (flat[0] * flat[0] + flat[1] * flat[1])
            .sqrt()
            .max(f32::EPSILON);
        let direction = [-flat[0] / norm, -flat[1] / norm];
        let Some(peer) = self.peer_mut(target) else {
            return;
        };
        peer.admin.effects.slap_until = now + 15_000;
        if peer.health > 0 && peer.state.stats[STAT_HEALTH] as i32 > 0 {
            peer.state
                .set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_KNOCKDOWN);
            peer.state.set_raw_field(PS_FORCE_DODGE_ANIM, 0);
            peer.knockdown.hand_extend_time = now + 1_000;
            peer.state.set_velocity([
                direction[0] * strength as f32,
                direction[1] * strength as f32,
                170.0,
            ]);
            peer.movement = peer.movement.reseeded(&peer.state);
            let origin = peer.state.origin();
            let name = format!("sound/weapons/melee/punch{sound}");
            self.holdable_sound(name.as_bytes(), origin, 0, now);
            self.admin_announce(call, system.effect(AdminCommand::Slap), false);
        }
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amtele` (`Cmd_amtele_f`): `<player> <x> <y> <z> <yaw>`, `<x> <y> <z> <yaw>`,
    /// `<player> <to player>`, `<to player>`, or nothing for the tele mark. Free-for-all
    /// only; never onto lava, slime or a hurting trigger.
    fn amtele(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        if self.gametype != GT_FFA || self.cvars.integer(b"g_jediVmerc") != 0 {
            self.admin_reply(
                caller,
                &with_word(system.texts.disallowed_gametype, entry.name.as_bytes()),
            );
            return;
        }
        let me = caller.player();
        let coordinates = |at: usize| -> Option<([f32; 3], f32)> {
            let number = |index: usize| {
                words
                    .get(index)
                    .map(|word| sjk_game_jka::userinfo::atoi(word) as f32)
            };
            Some((
                [number(at)?, number(at + 1)?, number(at + 2)?],
                number(at + 3)?,
            ))
        };
        // (whom, to whom, where): one of the three gives the destination.
        let (target, to, at) = match (words.len(), me) {
            (6, _) => (self.admin_client_for(words[1]), None, coordinates(2)),
            (5, Some(me)) => (me as i32, None, coordinates(1)),
            (3, _) => (
                self.admin_client_for(words[1]),
                Some(self.admin_client_for(words[2])),
                None,
            ),
            (_, None) => return,
            (2, Some(me)) => (me as i32, Some(self.admin_client_for(words[1])), None),
            (1, Some(me))
                if self
                    .peer(me)
                    .is_some_and(|peer| peer.admin.effects.tele_mark.is_some()) =>
            {
                (me as i32, None, None)
            }
            _ => return,
        };
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let Some(target) = self.admin_valid(target) else {
            return;
        };
        if !self.admin_allowed_on(caller, target, true, true) {
            return;
        }
        if self.peer(target).is_some_and(|peer| peer.refuse_teleport) {
            self.admin_reply(caller, system.texts.refuses_tele.as_bytes());
            return;
        }
        if me != Some(target) {
            self.told.push(Told::One(
                target,
                [
                    &b"print \""[..],
                    system.texts.refuse_tele_hint.as_bytes(),
                    b"\"",
                ]
                .concat(),
            ));
        }
        let (destination, yaw) = match (to, at) {
            (Some(to), _) => {
                let Some(to) = self.admin_valid(to) else {
                    return;
                };
                let origin = self.peer(to).map_or([0.0; 3], |peer| peer.state.origin());
                let destination = [origin[0] + 60.0, origin[1], origin[2] + 50.0];
                // `vectoyaw` of the way back, its y left at 0, as JA+ has it.
                let back = [origin[0] - destination[0], 0.0_f32];
                let yaw = if back == [0.0, 0.0] {
                    0.0
                } else {
                    let yaw = back[1].atan2(back[0]).to_degrees();
                    if yaw < 0.0 { yaw + 360.0 } else { yaw }
                };
                (destination, yaw)
            }
            (None, Some(at)) => at,
            (None, None) => match self
                .peer(target)
                .and_then(|peer| peer.admin.effects.tele_mark)
            {
                Some(mark) => mark,
                None => return,
            },
        };
        if let Some(refusal) = self.admin_unsafe_destination(destination, caller, system) {
            self.admin_reply(caller, refusal.as_bytes());
            return;
        }
        self.admin_teleport(target, destination, [0.0, yaw, 0.0]);
        let call = FxCall {
            target: target as i32,
            single: true,
            silent: me.is_some_and(|me| self.admin_rank(me) == Some(Rank::Rcon)),
        };
        self.admin_announce(call, system.effect(AdminCommand::Tele), false);
        self.admin_log_command(caller, entry.name, b"");
    }

    /// The refusal of a teleport over lava or slime, or over a hurting trigger, within
    /// 5000 units below; `None` where it is safe, or with no map to look at.
    fn admin_unsafe_destination(
        &self,
        destination: [f32; 3],
        caller: Caller,
        system: &AdminSystem,
    ) -> Option<&'static str> {
        let map = self.map.as_ref()?;
        let point = sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).ok()?;
        let below = [destination[0], destination[1], destination[2] - 5_000.0];
        let mut scratch = map.scratch.borrow_mut();
        let lava = map
            .bsp
            .trace_box_with(&mut scratch, destination, below, point, LAVA_TRACE);
        if lava.fraction < 1.0 && lava.content_flags & LAVA_OR_SLIME != 0 {
            return Some(match caller {
                Caller::Console => system.texts.lava_console,
                Caller::Player(_) => system.texts.lava,
            });
        }
        let ground = map
            .bsp
            .trace_box_with(&mut scratch, destination, below, point, GROUND_TRACE);
        let end = ground.end_position;
        let near = |(low, high): &([f32; 3], [f32; 3])| {
            let reach = [40.0, 40.0, 52.0];
            (0..3).all(|axis| {
                low[axis] <= end[axis] + reach[axis] && high[axis] >= end[axis] - reach[axis]
            })
        };
        self.triggers
            .iter()
            .any(|(_, hurt)| hurt.linked && near(&hurt.bounds))
            .then_some(system.texts.hurt_below)
    }

    /// `JP_ManualTeleport`: with `jp_starTrekTeleport 1` a playing player is cloaked first
    /// and beamed two seconds later ([`Self::admin_second_actions`]); otherwise at once,
    /// with the flashes.
    fn admin_teleport(&mut self, target: usize, destination: [f32; 3], angles: [f32; 3]) {
        let star_trek = self
            .admin_system()
            .is_some_and(|system| self.cvars.integer(system.settings.star_trek.as_bytes()) == 1);
        let Some(peer) = self.peer_mut(target) else {
            return;
        };
        if star_trek && peer.playing() {
            if peer.admin.effects.star_trek.is_none() {
                peer.admin.effects.star_trek = Some(StarTrek {
                    counter: STAR_TREK_SECONDS,
                    done: false,
                    origin: destination,
                    angles,
                });
                let now = self.last_frame_time;
                self.cloak_toggled(target, sjk_game_jka::holdables::Toggled::Cloaked, now);
            }
            return;
        }
        self.admin_teleport_now(target, destination, angles, true);
    }

    /// `JP_TeleportNow`: `TeleportPlayer` with the flashes where `effects`, the speed
    /// cleared, as JA+'s own teleport does.
    fn admin_teleport_now(
        &mut self,
        target: usize,
        destination: [f32; 3],
        angles: [f32; 3],
        effects: bool,
    ) {
        let now = self.last_frame_time;
        let Some(peer) = self.peer(target) else {
            return;
        };
        let (from, spectating) = (peer.state.origin(), !peer.playing());
        self.teleport_client_with(target, from, destination, angles, spectating, now, effects);
        if let Some(peer) = self.peer_mut(target) {
            peer.state.set_velocity([0.0; 3]);
            peer.movement = peer.movement.reseeded(&peer.state);
        }
    }

    /// `amtelemark`: where `amtele` alone goes, told as `vtos` writes it.
    fn amtelemark(&mut self, caller: Caller, entry: &AdminEntry, system: &AdminSystem) {
        let Caller::Player(client) = caller else {
            return;
        };
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let origin = peer.state.origin();
        let yaw = peer.state.view_angles()[1] as i32 as f32;
        peer.admin.effects.tele_mark = Some((origin, yaw));
        self.admin_reply(
            caller,
            &with_word(system.texts.tele_mark, vtos(origin).as_bytes()),
        );
    }

    /// `amorigin [player]`: where a player is.
    fn amorigin(&mut self, caller: Caller, entry: &AdminEntry, words: &[&[u8]]) {
        let Caller::Player(client) = caller else {
            return;
        };
        let target = words
            .get(1)
            .map_or(client as i32, |word| self.admin_client_for(word));
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let Some(target) = self.admin_valid(target) else {
            return;
        };
        if !self.admin_allowed_on(caller, target, true, false) {
            return;
        }
        let origin = self
            .peer(target)
            .map_or([0.0; 3], |peer| peer.state.origin());
        let text = [
            &self.admin_name(target),
            &b" : "[..],
            vtos(origin).as_bytes(),
            b"\n",
        ]
        .concat();
        self.admin_reply(caller, &text);
    }

    /// `refuseTele`: the admins' teleports refused, or accepted again.
    fn refuse_tele(&mut self, caller: Caller, system: &AdminSystem) {
        let Caller::Player(client) = caller else {
            return;
        };
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        // The one flag the player command `refuseTele` toggles too (`bridge_mod_commands`).
        let refused = !peer.refuse_teleport;
        peer.refuse_teleport = refused;
        let text = if refused {
            system.texts.refuse_on
        } else {
            system.texts.refuse_off
        };
        self.admin_reply(caller, text.as_bytes());
    }

    /// `amsay <text>` (`Cmd_amsay_f`): chat only the admins and its sender read.
    fn amsay(&mut self, caller: Caller, words: &[&[u8]], system: &AdminSystem) {
        let Caller::Player(client) = caller else {
            return;
        };
        if words.len() < 2
            || self
                .peer(client)
                .is_none_or(|peer| peer.admin.effects.silenced)
        {
            return;
        }
        let text = concat(words, 1, SAY_BYTES);
        let name = self.admin_name(client);
        self.log(&format!(
            "say_admin: {}: {}\n",
            String::from_utf8_lossy(&name),
            String::from_utf8_lossy(&text)
        ));
        let line = [
            &b"chat \""[..],
            system.texts.say_prefix.as_bytes(),
            &[ESCAPE, b'('],
            &name,
            b"^7",
            &[ESCAPE, b')', ESCAPE],
            b": ^3",
            &text,
            b"\"",
        ]
        .concat();
        for other in 0..self.players.places() {
            if self.admin_valid(other as i32).is_none()
                || (other != client && self.admin_rank(other).is_none())
            {
                continue;
            }
            self.told.push(Told::One(other, line.clone()));
        }
    }

    /// `ampsay <player|-1> <text>`: a centre print (and the same in the console).
    fn ampsay(&mut self, caller: Caller, entry: &AdminEntry, words: &[&[u8]]) {
        // `say_team` rerouted to it (`say_team_mod`) is not built: it needs a player.
        if words.len() < 3 {
            return;
        }
        let target = sjk_game_jka::userinfo::atoi(words[1]);
        let text = concat(words, 2, SAY_BYTES);
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        if target < -1 || target >= self.players.places() as i32 {
            return;
        }
        if target != -1 && self.admin_valid(target).is_none() {
            return;
        }
        let text = [&b"^7"[..], &rules::new_lines(&text), b"\n"].concat();
        let commands = [
            centre_print(&text),
            [&b"print \""[..], &text, b"\""].concat(),
        ];
        for command in commands {
            self.told.push(match usize::try_from(target) {
                Ok(target) => Told::One(target, command),
                Err(_) => Told::Everyone(command),
            });
        }
    }

    /// `amstatus`: every player in the game with its JA+ plugin.
    fn amstatus(&mut self, caller: Caller, entry: &AdminEntry, system: &AdminSystem) {
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let gap = if caller == Caller::Console {
            "   "
        } else {
            "  "
        };
        let mut out = Vec::new();
        for client in 0..self.players.places() {
            let Some(peer) = self
                .admin_valid(client as i32)
                .and_then(|client| self.peer(client))
            else {
                continue;
            };
            let plugin = if peer.bot {
                BOT_PLUGIN.to_vec()
            } else {
                sjk_protocol::info_value(&peer.userinfo, b"cjp_client")
                    .filter(|value| !value.is_empty())
                    .map_or_else(
                        || system.texts.no_plugin.as_bytes().to_vec(),
                        <[u8]>::to_vec,
                    )
            };
            out.extend_from_slice(
                [
                    format!("^5{client} ^7---> ").as_bytes(),
                    &peer.name,
                    gap.as_bytes(),
                    b"^7Plugin:^2",
                    &plugin,
                    b"\n",
                ]
                .concat()
                .as_slice(),
            );
        }
        self.admin_reply(caller, &out);
    }

    /// `amwhois`: the admins of the three ranks, told to everyone (only to the asker of
    /// the rcon rank).
    fn amwhois(&mut self, caller: Caller, entry: &AdminEntry, system: &AdminSystem) {
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let to_asker =
            matches!(caller, Caller::Player(client) if self.admin_rank(client) == Some(Rank::Rcon));
        for client in 0..self.players.places() {
            if self.admin_valid(client as i32).is_none() {
                continue;
            }
            let Some(index) = self.admin_rank(client).and_then(Rank::named_index) else {
                continue;
            };
            let line = [
                &b"print \""[..],
                system.texts.whois_titles[index].as_bytes(),
                b" ^7: ",
                &self.admin_name(client),
                b"  \n\"",
            ]
            .concat();
            self.told.push(match caller {
                Caller::Player(asker) if to_asker => Told::One(asker, line),
                _ => Told::Everyone(line),
            });
        }
    }

    /// `ammap <gametype|HL> <map>` (`Cmd_ammap_f`): the game type set and the map loaded,
    /// `nextmap` kept.
    fn ammap(&mut self, caller: Caller, entry: &AdminEntry, words: &[&[u8]], system: &AdminSystem) {
        let gametypes = match caller {
            Caller::Console => system.texts.map_gametypes_console,
            Caller::Player(_) => system.texts.map_gametypes,
        };
        let (Some(&kind), Some(&map)) = (words.get(1), words.get(2)) else {
            self.admin_reply(
                caller,
                [system.texts.map_usage, gametypes].concat().as_bytes(),
            );
            return;
        };
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        if words[..3]
            .iter()
            .any(|word| word.iter().any(|byte| matches!(byte, b';' | b'\r' | b'\n')))
        {
            self.admin_reply(caller, system.texts.invalid_string.as_bytes());
            return;
        }
        let (gametype, highlander) = if kind.eq_ignore_ascii_case(b"HL") {
            (GT_FFA, true)
        } else {
            let gametype = sjk_game_jka::userinfo::atoi(kind);
            if !(0..=8).contains(&gametype) || AMMAP_REFUSED.contains(&gametype) {
                self.admin_reply(
                    caller,
                    [system.texts.map_bad_gametype, gametypes]
                        .concat()
                        .as_bytes(),
                );
                return;
            }
            (gametype, false)
        };
        self.admin_log_command(caller, entry.name, b"");
        let nextmap = String::from_utf8_lossy(self.cvars.string(b"nextmap")).into_owned();
        let map = String::from_utf8_lossy(map);
        let mut text = format!(
            "jp_highlander {}\ng_gametype {gametype}\nmap {map}",
            u8::from(highlander)
        );
        if !nextmap.is_empty() {
            text += &format!(" ; set nextmap \"{nextmap}\"");
        }
        text.push('\n');
        self.queue_console(text.as_bytes(), &mut |line| {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), line);
        });
    }

    /// `amrename <player> <new name>`: renamed, and kept from renaming itself for 30
    /// seconds.
    fn amrename(&mut self, caller: Caller, entry: &AdminEntry, words: &[&[u8]]) {
        let Some(&who) = words.get(1).filter(|_| words.len() >= 3) else {
            return;
        };
        let name = concat(words, 2, NAME_BYTES);
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let Some(target) = self.admin_valid(self.admin_client_for(who)) else {
            return;
        };
        if !self.admin_allowed_on(caller, target, true, false) {
            return;
        }
        let now = self.last_frame_time;
        let Some(peer) = self.peer_mut(target) else {
            return;
        };
        let userinfo = sjk_protocol::info_set_value(&peer.userinfo, b"name", &name);
        peer.netname_time = 0;
        self.userinfo_changed(target, &userinfo);
        if let Some(peer) = self.peer_mut(target) {
            peer.netname_time = now + 30_000;
        }
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amlockteam <s|f|b|r>`: a team locked, or unlocked, told to everyone.
    fn amlockteam(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        let Some(&team) = words.get(1) else {
            self.admin_reply(caller, system.texts.team_lock_usage.as_bytes());
            return;
        };
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let Some(index) = TEAM_NAMES
            .iter()
            .position(|name| team.first() == name.as_bytes().first())
        else {
            self.admin_reply(caller, system.texts.team_lock_usage.as_bytes());
            return;
        };
        let setting = system.settings.team_lock;
        let lock = self.cvars.integer(setting.as_bytes());
        let bit = 1 << (index + 1);
        let unlocking = lock & bit != 0;
        self.set_cvar(setting, &(lock ^ bit).to_string());
        let text = if unlocking {
            system.texts.team_unlocked
        } else {
            system.texts.team_locked
        };
        self.told.push(Told::Everyone(
            [
                &b"print \""[..],
                &with_word(text, TEAM_NAMES[index].as_bytes()),
                b"\"",
            ]
            .concat(),
        ));
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amforceteam <player|-1> <team>`: moved to the team, past the locks.
    fn amforceteam(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        let (Some(&who), Some(&team)) = (words.get(1), words.get(2)) else {
            self.admin_reply(caller, system.texts.force_team_usage.as_bytes());
            return;
        };
        let target = self.admin_client_for(who);
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let places = self.players.places() as i32;
        if target < -1 || target >= places {
            self.admin_reply(caller, system.texts.force_team_usage.as_bytes());
            return;
        }
        let (first, last) = if target == -1 {
            (0, places - 1)
        } else {
            (target, target)
        };
        let now = self.last_frame_time;
        for client in first..=last {
            let Some(client) = self.admin_valid(client) else {
                continue;
            };
            if caller == Caller::Console || self.admin_allowed_on(caller, client, false, false) {
                self.set_team_to(client, team, now);
            }
        }
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `amkillvote`: the vote running ends, failed, and nothing of it is carried out.
    fn amkillvote(&mut self, caller: Caller, entry: &AdminEntry, system: &AdminSystem) {
        if !self.admin_allowed(caller, entry, true) {
            return;
        }
        let now = self.last_frame_time;
        if self.vote.time == 0 && self.vote.execute_time < now {
            return;
        }
        self.vote.string.clear();
        self.vote.clean.clear();
        self.vote.time = now - 32_000;
        self.told.push(Told::Everyone(
            [&b"print \""[..], system.texts.vote_killed.as_bytes(), b"\""].concat(),
        ));
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `JP_SetMotd`: the message of the day shown to `client` for `jp_motdtime` seconds.
    fn admin_set_motd(&mut self, client: usize, system: &AdminSystem) {
        let text = [
            system.texts.motd_title.as_bytes(),
            &self.admin_text(system.settings.motd),
            b"\n",
        ]
        .concat();
        let seconds = self.cvars.integer(system.settings.motd_time.as_bytes());
        if let Some(peer) = self.peer_mut(client) {
            peer.admin.effects.motd = Some((rules::new_lines(&text), seconds));
        }
    }

    /// `amShowMotd <player|-1>`.
    fn amshowmotd(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
        system: &AdminSystem,
    ) {
        let Some(target) = self.admin_call_target(caller, entry, words) else {
            return;
        };
        let places = self.players.places() as i32;
        if target < -1 || target >= places {
            return;
        }
        let (first, last) = if target == -1 {
            (0, places - 1)
        } else {
            (target, target)
        };
        for client in first..=last {
            let Some(client) = self.admin_valid(client) else {
                continue;
            };
            if caller == Caller::Console || self.admin_allowed_on(caller, client, true, true) {
                self.admin_set_motd(client, system);
            }
        }
        self.admin_log_command(caller, entry.name, b"");
    }

    /// `JP_ClientSecondActions`' part of the admin effects, once a second for each
    /// player: the beam of a teleport under the cloak, the message of the day's seconds,
    /// a sleeping player kept down, a silenced one kept from renaming itself.
    pub(super) fn admin_second_actions(&mut self, client: usize, now: i32) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let effects = &mut peer.admin.effects;
        if effects.next_second > now {
            return;
        }
        effects.next_second = now + 1_000;
        if effects.sleeping && peer.knockdown.hand_extend_time < now + 2_000 {
            peer.state
                .set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_KNOCKDOWN);
            peer.knockdown.hand_extend_time = now + 5_000;
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        if peer.admin.effects.silenced {
            peer.netname_time = now + 1_500;
        }
        let effects = &mut peer.admin.effects;
        if let Some((text, seconds)) = &mut effects.motd {
            let line = [
                &b"^7"[..],
                text,
                &with_numbers(system.texts.motd_time_left, &[*seconds]),
            ]
            .concat();
            *seconds -= 1;
            if *seconds <= 0 {
                effects.motd = None;
            }
            self.told.push(Told::One(client, centre_print(&line)));
        }
        self.admin_star_trek_second(client);
    }

    /// `JP_StarTrekSecond`: the cloaked player beamed two seconds after the command,
    /// uncloaked two seconds after that.
    fn admin_star_trek_second(&mut self, client: usize) {
        let star_trek = self
            .admin_system()
            .is_some_and(|system| self.cvars.integer(system.settings.star_trek.as_bytes()) == 1);
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let Some(trek) = &mut peer.admin.effects.star_trek else {
            return;
        };
        if !star_trek {
            return;
        }
        trek.counter -= 1;
        if trek.counter >= 2 {
            return;
        }
        let trek = *trek;
        if !trek.done && trek.counter >= 0 {
            if let Some(StarTrek { done, .. }) = &mut peer.admin.effects.star_trek {
                *done = true;
            }
            self.admin_teleport_now(client, trek.origin, trek.angles, false);
        } else if trek.counter < 0 {
            peer.admin.effects.star_trek = None;
            let now = self.last_frame_time;
            self.cloak_toggled(client, sjk_game_jka::holdables::Toggled::Decloaked, now);
        }
    }

    /// `G_Damage`'s refusal for a player an admin protects (`amprotect`): nothing another
    /// player or an NPC deals reaches it.
    pub(super) fn admin_protects(
        &self,
        target: usize,
        request: &sjk_game_jka::damage::DamageRequest,
    ) -> bool {
        request
            .attacker
            .is_some_and(|attacker| attacker.npc || usize::from(attacker.client) != target)
            && self
                .peer(target)
                .is_some_and(|peer| peer.admin.effects.protected)
    }

    /// Whether a silenced player's chat command (`say`, `say_team`, `tell`) is swallowed.
    pub(super) fn admin_silences(&self, client: usize, command: &[u8]) -> bool {
        self.admin_system().is_some()
            && ["say", "say_team", "tell"]
                .iter()
                .any(|name| command.eq_ignore_ascii_case(name.as_bytes()))
            && self
                .peer(client)
                .is_some_and(|peer| peer.admin.effects.silenced)
    }

    /// Whether `client`'s `callvote` is refused because an admin took the right (told).
    pub(super) fn admin_denies_vote(&mut self, client: usize) -> bool {
        let Some(system) = self.admin_system() else {
            return false;
        };
        if !self
            .peer(client)
            .is_some_and(|peer| peer.admin.effects.deny_vote)
        {
            return false;
        }
        self.told.push(Told::One(
            client,
            [&b"print \""[..], system.texts.vote_denied.as_bytes(), b"\""].concat(),
        ));
        true
    }

    /// The team lock on `client`'s `team <word>` (`JP_TeamLockRefused` from `SetTeam`):
    /// `None` when it is refused (told), else the word to go on with (another team for an
    /// automatic choice that would end in a locked one).
    pub(super) fn admin_team_word<'a>(
        &mut self,
        client: usize,
        word: &'a [u8],
    ) -> Option<&'a [u8]> {
        let Some(system) = self.admin_system() else {
            return Some(word);
        };
        let lock = self.cvars.integer(system.settings.team_lock.as_bytes());
        if lock == 0 {
            return Some(word);
        }
        let is = |words: &[&[u8]]| words.iter().any(|known| word.eq_ignore_ascii_case(known));
        let spectating = is(&[
            b"scoreboard",
            b"score",
            b"follow1",
            b"follow2",
            b"spectator",
            b"s",
        ]);
        let asked = if spectating {
            TeamAsked::Spectator
        } else if self.gametype < sjk_game_jka::match_end::GT_TEAM {
            TeamAsked::Free
        } else if is(&[b"red", b"r"]) {
            TeamAsked::Red
        } else if is(&[b"blue", b"b"]) {
            TeamAsked::Blue
        } else {
            TeamAsked::Auto
        };
        let duel = matches!(
            self.gametype,
            super::GAMETYPE_DUEL | super::GAMETYPE_POWERDUEL
        );
        match rules::team_lock(lock, asked, duel, system.texts) {
            TeamLock::Allowed => Some(word),
            TeamLock::Instead(TeamAsked::Red) => Some(b"red"),
            TeamLock::Instead(_) => Some(b"blue"),
            TeamLock::Refused(text) => {
                self.told.push(Told::One(
                    client,
                    [&b"print \""[..], text.as_bytes(), b"\""].concat(),
                ));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bridge_admin::tests::{console, game, told};
    use super::*;
    use sjk_game_jka::damage::{Attacker, DamageRequest};

    fn command(game: &mut NativeGame, client: usize, line: &str) -> String {
        run(game, client, line);
        told(game, client)
    }

    /// A player's command, what it told left for the test to read.
    fn run(game: &mut NativeGame, client: usize, line: &str) {
        game.player_command(client, line.as_bytes(), game.last_frame_time);
    }

    /// What `client` alone was told, its centre prints among it.
    fn told_to(game: &mut NativeGame, client: usize) -> String {
        let mut out = String::new();
        for told in std::mem::take(&mut game.told) {
            if let Told::One(to, text) = told
                && to == client
            {
                out += &String::from_utf8_lossy(&text);
            }
        }
        out
    }

    /// A JA+ server whose player 0 is a council admin with every command (JA+'s default
    /// mask leaves the council without `amslap`, `ammindtrick`, `npc` and `amweather`),
    /// the others plain players.
    fn ruled() -> NativeGame {
        let mut game = game("japlus");
        game.set_cvar("jp_councilAllowedCMD", "-1");
        console(&mut game, "admGrant 0 council");
        game.told.clear();
        game
    }

    fn blow(attacker: Option<u16>) -> DamageRequest {
        DamageRequest {
            level_time: 2_000,
            attacker: attacker.map(|client| Attacker {
                npc: false,
                client,
                max_health: 100,
                team: 0,
                saber_knockback: [1.0; 4],
            }),
            direction: None,
            point: None,
            damage: 10,
            flags: 0,
            means: sjk_game_jka::means_of_death::MOD_FORCE_DARK,
        }
    }

    #[test]
    fn the_base_game_has_none_of_these_commands_or_effects() {
        let mut game = game("base");
        for line in [
            "amslap 1",
            "amtele 1 2",
            "amsilence 1",
            "amsay hi",
            "ammap 0 mp/ffa5",
        ] {
            assert!(
                command(&mut game, 0, line).contains("Unknown command"),
                "{line}"
            );
        }
        game.peer_mut(1).unwrap().admin.effects.silenced = true;
        game.admin_second_actions(1, 5_000);
        assert!(!game.admin_silences(1, b"say"));
        assert!(command(&mut game, 1, "say hello").contains("hello"));
        assert_eq!(game.peer(1).unwrap().netname_time, 0);
    }

    #[test]
    fn a_slap_throws_and_floods() {
        let mut game = ruled();
        run(&mut game, 0, "amslap 1");
        let peer = game.peer(1).unwrap();
        assert_eq!(peer.state.velocity()[2], 170.0);
        let flat = peer.state.velocity()[0].hypot(peer.state.velocity()[1]);
        assert!(flat == 175.0 || (flat - 200.0).abs() < 0.01, "{flat}");
        assert_eq!(
            peer.state.raw_field(PS_FORCE_HAND_EXTEND),
            Some(HANDEXTEND_KNOCKDOWN)
        );
        let said = told_to(&mut game, 1);
        assert!(said.contains("You Fly"), "{said}");
        // Within 15 seconds: refused.
        let said = command(&mut game, 0, "amslap 1");
        assert!(said.contains("Admin Flood Protect"), "{said}");
        // Outside a free-for-all: refused and said.
        game.gametype = super::super::GAMETYPE_TEAM;
        assert!(command(&mut game, 0, "amslap 2").contains("disallowed in this gametype"));
    }

    #[test]
    fn sleep_holds_down_and_shields_and_wake_ends_it() {
        let mut game = ruled();
        command(&mut game, 0, "amsleep 1");
        let peer = game.peer(1).unwrap();
        assert!(peer.admin.effects.sleeping);
        assert_eq!(
            peer.state.raw_field(PS_FORCE_HAND_EXTEND),
            Some(HANDEXTEND_KNOCKDOWN)
        );
        let health = peer.health;
        game.hurt(1, blow(Some(2)));
        assert_eq!(game.peer(1).unwrap().health, health);
        // Kept down while it sleeps.
        game.admin_second_actions(1, 3_000);
        assert_eq!(game.peer(1).unwrap().knockdown.hand_extend_time, 8_000);
        command(&mut game, 0, "amwake 1");
        assert!(!game.peer(1).unwrap().admin.effects.sleeping);
        game.hurt(1, blow(Some(2)));
        assert!(game.peer(1).unwrap().health < health);
        // Everybody: the message for all.
        let said = command(&mut game, 0, "amsleep -1");
        assert!(said.contains("You are all sleeping!"), "{said}");
        assert!((1..4).all(|client| game.peer(client).unwrap().admin.effects.sleeping));
    }

    #[test]
    fn protection_holds_until_the_player_attacks() {
        let mut game = ruled();
        command(&mut game, 0, "amprotect 2");
        let peer = game.peer(2).unwrap();
        assert!(peer.admin.effects.protected);
        assert_ne!(peer.state.raw_field(EFLAGS).unwrap() & EF_INVULNERABLE, 0);
        let health = peer.health;
        game.hurt(2, blow(Some(1)));
        assert_eq!(game.peer(2).unwrap().health, health);
        // The world still hurts, as in JA+.
        game.hurt(2, blow(None));
        assert!(game.peer(2).unwrap().health < health);
        // Again: no longer.
        let said = command(&mut game, 0, "amprotect 2");
        assert!(said.contains("is loosing"), "{said}");
        assert!(!game.peer(2).unwrap().admin.effects.protected);
    }

    #[test]
    fn the_silenced_say_nothing_until_unsilenced() {
        let mut game = ruled();
        command(&mut game, 0, "amsilence 1");
        assert!(command(&mut game, 1, "say hello").is_empty());
        assert!(command(&mut game, 1, "amsay hello").is_empty());
        game.admin_second_actions(1, 5_000);
        assert_eq!(game.peer(1).unwrap().netname_time, 6_500);
        command(&mut game, 0, "amunsilence 1");
        assert!(command(&mut game, 1, "say hello").contains("hello"));
    }

    #[test]
    fn teleports_go_to_players_marks_and_places() {
        let mut game = ruled();
        game.set_cvar("jp_starTrekTeleport", "0");
        run(&mut game, 0, "amtele 1 2");
        let to = game.peer(2).unwrap().state.origin();
        let origin = game.peer(1).unwrap().state.origin();
        assert_eq!(origin, [to[0] + 60.0, to[1], to[2] + 51.0]);
        assert_eq!(game.peer(1).unwrap().state.velocity(), [0.0; 3]);
        assert!(told_to(&mut game, 1).contains("You have been teleported"));
        // Places, and the mark.
        command(&mut game, 0, "amtele 3 100 200 300 90");
        assert_eq!(game.peer(3).unwrap().state.origin(), [100.0, 200.0, 301.0]);
        let said = command(&mut game, 0, "amtelemark");
        assert!(said.contains("TELEPORT MARKER :^3 (500 0 0)"), "{said}");
        command(&mut game, 0, "amtele 2");
        command(&mut game, 0, "amtele");
        assert_eq!(game.peer(0).unwrap().state.origin(), [500.0, 0.0, 1.0]);
        // A player may refuse.
        assert!(command(&mut game, 2, "refuseTele").contains("You refuse"));
        assert!(command(&mut game, 0, "amtele 2 0").contains("refuses to be teleported"));
        let said = command(&mut game, 0, "amorigin 3");
        assert!(said.contains("P3 : (100 200 301)"), "{said}");
    }

    #[test]
    fn a_star_trek_teleport_cloaks_then_beams() {
        let mut game = ruled();
        command(&mut game, 0, "amtele 1 100 0 0 0");
        let start = game.peer(1).unwrap().state.origin();
        assert_ne!(game.peer(1).unwrap().state.powerups[11], 0);
        for (second, beamed) in [(2_000, false), (3_000, true), (4_000, true), (5_000, true)] {
            game.admin_second_actions(1, second);
            let origin = game.peer(1).unwrap().state.origin();
            assert_eq!(origin != start, beamed, "{second}");
        }
        assert_eq!(game.peer(1).unwrap().state.powerups[11], 0);
        assert_eq!(game.peer(1).unwrap().state.origin(), [100.0, 0.0, 1.0]);
    }

    #[test]
    fn admins_chat_among_themselves_and_print_to_players() {
        let mut game = ruled();
        console(&mut game, "admGrant 2 knight");
        game.told.clear();
        game.player_command(1, b"amsay psst", game.last_frame_time);
        let to: Vec<usize> = game
            .told
            .iter()
            .filter_map(|told| match told {
                Told::One(to, text) if text.starts_with(b"chat") => Some(*to),
                _ => None,
            })
            .collect();
        assert_eq!(to, [0, 1, 2]);
        game.told.clear();
        run(&mut game, 0, "ampsay 3 hello\\nthere");
        let said = told_to(&mut game, 3);
        assert!(said.contains("cp \"^7hello\nthere\n\""), "{said}");
    }

    #[test]
    fn status_and_whois_list_the_players_and_the_admins() {
        let mut game = ruled();
        let said = command(&mut game, 0, "amstatus");
        assert!(
            said.contains("^5 1 ^7---> P1") || said.contains("^51 ^7---> P1"),
            "{said}"
        );
        assert!(said.contains("No Plugin"), "{said}");
        let said = command(&mut game, 0, "amwhois");
        assert!(said.contains("^3Admin Council ^7: P0"), "{said}");
        assert!(console(&mut game, "amstatus").contains("P3   ^7Plugin"));
    }

    #[test]
    fn ammap_queues_the_game_type_and_the_map() {
        let mut game = ruled();
        assert!(command(&mut game, 0, "ammap").contains("Usage : ammap"));
        assert!(command(&mut game, 0, "ammap 1 mp/ffa5").contains("Invalid GAMEType"));
        command(&mut game, 0, "ammap 6 mp/ffa5");
        let mut pass = crate::command_buffer::CommandPass::default();
        let mut lines = Vec::new();
        while let Some(line) = game.next_console_line(&mut pass) {
            lines.push(String::from_utf8(line).unwrap());
        }
        assert_eq!(lines, ["jp_highlander 0", "g_gametype 6", "map mp/ffa5"]);
    }

    #[test]
    fn a_rename_sticks_for_thirty_seconds() {
        let mut game = ruled();
        command(&mut game, 0, "amrename 1 New Name");
        assert_eq!(game.peer(1).unwrap().name, b"New Name");
        assert_eq!(game.peer(1).unwrap().netname_time, 31_000);
    }

    #[test]
    fn locked_teams_refuse_and_the_force_moves_past_them() {
        let mut game = ruled();
        let said = command(&mut game, 0, "amlockteam s");
        assert!(said.contains("^5spectator ^7team ^6locked!"), "{said}");
        assert_eq!(game.cvars.integer(b"jp_teamLock"), 2);
        let said = command(&mut game, 1, "team spectator");
        assert!(said.contains("Spectator team^7 is locked"), "{said}");
        assert_ne!(game.peer(1).unwrap().session.team, 3);
        console(&mut game, "amforceteam 1 s");
        assert_eq!(game.peer(1).unwrap().session.team, 3);
        assert!(command(&mut game, 0, "amlockteam s").contains("Unlocked"));
        assert_eq!(game.cvars.integer(b"jp_teamLock"), 0);
    }

    #[test]
    fn votes_are_killed_and_the_right_to_call_taken() {
        let mut game = ruled();
        game.vote.time = 900;
        game.vote.string = b"map mp/ffa5".to_vec();
        let said = command(&mut game, 0, "amkillvote");
        assert!(said.contains("Vote has been killed"), "{said}");
        assert_eq!(game.vote.time, 1_000 - 32_000);
        assert!(game.vote.string.is_empty());
        command(&mut game, 0, "amdenyvote 2");
        let said = command(&mut game, 2, "callvote map mp/ffa5");
        assert!(said.contains("not allowed to callVote"), "{said}");
    }

    #[test]
    fn the_message_of_the_day_counts_down() {
        let mut game = ruled();
        game.set_cvar("jp_motd", "Hello\\nall");
        game.set_cvar("jp_motdtime", "2");
        run(&mut game, 3, "ammotd");
        game.admin_second_actions(3, 2_000);
        let said = told_to(&mut game, 3);
        assert!(
            said.contains("Hello\nall") && said.contains("TimeLeft : ^22 s"),
            "{said}"
        );
        game.admin_second_actions(3, 3_000);
        assert!(told_to(&mut game, 3).contains("TimeLeft : ^21 s"));
        game.admin_second_actions(3, 4_000);
        assert!(told_to(&mut game, 3).is_empty());
    }
}
