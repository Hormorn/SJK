//! The player commands a server mod adds ([`sjk_server_mod::ModCommand`]), carried out
//! with the shared code, gated and answered by the mod's rules
//! ([`sjk_server_mod::PlayerCommandRules`]): JA+'s emotes, `amhug`, `amflip`,
//! `amknockmedown`, `amdropsaber`, `jetpack`, `amcolor`, `sabercolor`, `saber`, its
//! lists and `refuseTele` (VisionJK `codemp/game/japlus/g_japlus_cmds.c`,
//! `g_japlus_info.c`, `g_japlus_tele.c`). The base game has none of them.

use super::{
    GAMETYPE_CTF, GAMETYPE_DUEL, GAMETYPE_FFA, GAMETYPE_POWERDUEL, GAMETYPE_SIEGE, GAMETYPE_TEAM,
    NativeGame, Told,
};
use crate::collision::{Void, WithPlayers, WorldCollision};
use sjk_game_jka::emote_commands;
use sjk_game_jka::generic_commands::SaberSound;
use sjk_game_jka::pmove::MovementCollision;
use sjk_server_mod::{EmoteKind, ModCommand, ModEmote, PlayerCommandRules};

/// `MASK_PLAYERSOLID`: what the hug's line stops at (`ent->clipmask`).
const MASK_PLAYERSOLID: u32 = 0x1 | 0x10 | 0x100 | 0x1000;
/// `STAT_HOLDABLE_ITEMS`, `STAT_WEAPONS`; `HI_JETPACK`; `WP_SABER`.
const STAT_HOLDABLE_ITEMS: usize = 2;
const STAT_WEAPONS: usize = 4;
const WP_SABER: u8 = 3;
/// The wire fields `saberMove`, `saberBlocked`, `forceHandExtend`.
const PS_SABER_MOVE: usize = 34;
const PS_SABER_BLOCKED: usize = 77;
const PS_FORCE_HAND_EXTEND: usize = 80;
/// `PM_FREEZE`, which a mod's held emote puts a player in for one move; `PM_NORMAL`.
const PM_FREEZE: u8 = 6;
const PM_NORMAL: u8 = 0;
/// `TAUNT_GLOAT`: `amvictory`'s taunt event.
const TAUNT_GLOAT: u32 = 4;
/// `CHAN_AUTO`.
const CHAN_AUTO: u32 = 0;
/// `amhug`'s `jp_emotDisallow` bit and reach (`g_japlus_cmds.c:172,301`).
const HUG_BIT: u8 = 13;
const HUG_RANGE: f32 = 40.0;
const MAX_CLIENTS: usize = 32;

/// `vectoyaw`: the yaw of `direction`, 0 to 360.
fn yaw_of(direction: [f32; 3]) -> f32 {
    if direction[0] == 0.0 && direction[1] == 0.0 {
        return 0.0;
    }
    let yaw = direction[1].atan2(direction[0]).to_degrees();
    if yaw < 0.0 { yaw + 360.0 } else { yaw }
}

/// `Q_atoi` of an argument.
fn atoi(word: &[u8]) -> i32 {
    sjk_game_jka::userinfo::atoi(word)
}

impl NativeGame {
    /// The running mod's player-command rules, the base game's (none) without one.
    fn command_rules(&self) -> PlayerCommandRules {
        self.server_mod
            .map_or_else(PlayerCommandRules::default, |server_mod| {
                server_mod.player_commands(&self.cvars)
            })
    }

    /// One of the mod's texts to `client`; nothing for an empty one.
    fn tell_text(&mut self, client: usize, text: &str) {
        if !text.is_empty() {
            self.told.push(Told::One(client, text.as_bytes().to_vec()));
        }
    }

    /// A command the running server mod adds, `command` as the player typed it (the whole
    /// line `text`), with its `arguments`, at the server's clock `server_time`. None at the intermission (VisionJK `g_japlus_admin.c:1264-1272`).
    pub(super) fn mod_command(
        &mut self,
        client: usize,
        command: &[u8],
        text: &[u8],
        arguments: &[&[u8]],
        added: ModCommand,
        server_time: i32,
    ) {
        let rules = self.command_rules();
        if self.match_end.ending() {
            let (before, after) = rules.texts.intermission;
            let refusal = [before.as_bytes(), command, after.as_bytes()].concat();
            self.told.push(Told::One(client, refusal));
            return;
        }
        let level_time = self.last_frame_time;
        match added {
            ModCommand::FullForceDuel => self.duel_key(client, level_time, true),
            ModCommand::Admin(admin) => {
                self.admin_player_command(client, text, admin, server_time);
            }
            ModCommand::Emote(emote) => self.emote_command(client, emote, &rules, level_time),
            ModCommand::Hug => self.hug(client, &rules, level_time),
            ModCommand::Kiss => {
                // `Cmd_amkiss_f` (`g_japlus_cmds.c:307-319`) with `jp_newGLAAnims` off:
                // SJK plays no JA+ animation.
                if self.peer(client).is_some_and(|peer| {
                    peer.playing() && peer.health > 0 && peer.state.stats[0] as i32 > 0
                }) {
                    self.tell_text(client, rules.texts.kiss_refused);
                }
            }
            ModCommand::Flip => self.flip(client, level_time),
            ModCommand::KnockMeDown => self.knock_me_down(client, &rules, level_time),
            ModCommand::DropSaber => self.drop_saber(client, &rules, level_time),
            ModCommand::Jetpack => self.jetpack_command(client, &rules),
            ModCommand::SkinColor => self.skin_color(client, arguments, &rules),
            ModCommand::SaberColor => self.saber_color(client, arguments, &rules),
            ModCommand::SaberSwitch => self.saber_switch(client, arguments, &rules, level_time),
            ModCommand::MoveList | ModCommand::DamageList => {
                let prints = self
                    .server_mod
                    .map(|server_mod| server_mod.command_prints(added, &self.cvars))
                    .unwrap_or_default();
                for text in prints {
                    self.told.push(Told::One(client, text.into_bytes()));
                }
            }
            ModCommand::DuelList => self.duel_list(client, &rules),
            ModCommand::RefuseTeleport => {
                // `Cmd_refuseTele_f` (`g_japlus_tele.c:297-302`).
                let Some(peer) = self.peer_mut(client) else {
                    return;
                };
                peer.refuse_teleport = !peer.refuse_teleport;
                let text = if peer.refuse_teleport {
                    rules.texts.refuse_teleport_on
                } else {
                    rules.texts.refuse_teleport_off
                };
                self.tell_text(client, text);
            }
        }
    }

    /// The emote table's commands (`JP_PlayerCommand`, `g_japlus_cmds.c:871-893`).
    fn emote_command(
        &mut self,
        client: usize,
        emote: ModEmote,
        rules: &PlayerCommandRules,
        level_time: i32,
    ) {
        let gametype = self.gametype;
        let Some(peer) = self.peer(client) else {
            return;
        };
        let state = &peer.state;
        let base = peer.movement.state().saber_anim_level_base;
        let saber_in_hand =
            state.weapon() == WP_SABER && state.saber_entity_num() != 0 && !state.saber_in_flight();
        match emote.kind {
            EmoteKind::Plain => {}
            EmoteKind::FreeForAllOnly => {
                if gametype != GAMETYPE_FFA || state.duel_in_progress() {
                    return;
                }
            }
            EmoteKind::LightsSaber => {
                // `Cmd_amfinishinghim_f` (`g_japlus_cmds.c:155-165`).
                let dead = state.movement_type() == 5
                    || peer.health < 1
                    || (state.stats[0] as i32) < 1
                    || state.entity_flags() & 1 != 0;
                if dead {
                    return;
                }
                if saber_in_hand && state.saber_holstered() != 0 {
                    self.toggle_saber_command(client, level_time);
                }
            }
            EmoteKind::Victory => {
                if state.weapon() != WP_SABER {
                    return;
                }
                if emote_commands::sabers_off(state, base) {
                    self.toggle_saber_command(client, level_time);
                }
            }
        }
        self.pose(client, emote, rules, level_time);
        if emote.kind == EmoteKind::Victory
            && let Some(peer) = self.peer_mut(client)
        {
            // `G_AddEvent(ent, EV_TAUNT, TAUNT_GLOAT)`.
            sjk_game_jka::generic_commands::taunt_event(&mut peer.state, TAUNT_GLOAT);
            peer.entity.event_raised(level_time);
            peer.movement = peer.movement.reseeded(&peer.state);
        }
    }

    /// `JP_Emote` (`g_japlus_cmds.c:87-112`): the mask's and the game type's refusals said
    /// first, then the pose for a player free to play it.
    fn pose(
        &mut self,
        client: usize,
        emote: ModEmote,
        rules: &PlayerCommandRules,
        level_time: i32,
    ) {
        if rules.emotes_refused & (1 << emote.bit) != 0 {
            self.tell_text(client, rules.texts.emote_refused);
            return;
        }
        if self.gametype >= GAMETYPE_TEAM && emote.free_for_all_pose {
            self.tell_text(client, rules.texts.emote_gametype);
            return;
        }
        let Some(animation) = sjk_game_jka::legacy_animation_index(emote.animation) else {
            return;
        };
        self.play_pose(client, animation as u16, true, level_time);
    }

    /// `JP_EmoteExec` for a player who may act (`JP_CanAct`) and, where `check`, has its
    /// hands free and stands or sits (`JP_Emote`'s last checks).
    fn play_pose(&mut self, client: usize, animation: u16, check: bool, level_time: i32) {
        let lengths = self.map.as_ref().and_then(|map| map.animations.clone());
        let empty = sjk_game_jka::AnimationLengthTable::new([]);
        let lengths: &dyn sjk_game_jka::AnimationLengths =
            lengths.as_deref().map_or(&empty, |lengths| lengths);
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if check
            && (!peer.playing()
                || !emote_commands::can_act(&peer.state, peer.health)
                || !emote_commands::may_pose(&peer.state)
                || peer.gear.packs.jetpack_on)
        {
            return;
        }
        emote_commands::play_pose(
            &mut peer.state,
            &mut peer.knockdown.hand_extend_time,
            animation,
            lengths,
            level_time,
        );
        peer.movement = peer.movement.reseeded(&peer.state);
    }

    /// `Cmd_amhug_f` (`g_japlus_cmds.c:210-305`): the player right in front, 40 units
    /// along the view, both sabers put away; the two turn to each other and hug.
    fn hug(&mut self, client: usize, rules: &PlayerCommandRules, level_time: i32) {
        let gametype = self.gametype;
        let Some(peer) = self.peer(client) else {
            return;
        };
        // `JP_EmoteAlive`.
        if !peer.playing() || peer.health <= 0 || (peer.state.stats[0] as i32) <= 0 {
            return;
        }
        if rules.emotes_refused & (1 << HUG_BIT) != 0 {
            self.tell_text(client, rules.texts.emote_refused);
            return;
        }
        if gametype >= GAMETYPE_TEAM {
            self.tell_text(client, rules.texts.emote_gametype);
            return;
        }
        let state = &peer.state;
        if !emote_commands::can_act(state, peer.health)
            || state.raw_field(PS_FORCE_HAND_EXTEND).unwrap_or(0) != 0
            || peer.gear.packs.jetpack_on
        {
            return;
        }
        let start = state.origin();
        let forward = sjk_game_jka::pmove::flight::flight_axes(state.view_angles()).0;
        let end: [f32; 3] = std::array::from_fn(|axis| start[axis] + HUG_RANGE * forward[axis]);
        let duelling = state.duel_in_progress();
        // `trap->Trace`, the duellists in it too.
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
        if other >= MAX_CLIENTS || other == client {
            return;
        }
        let Some(them) = self.peer(other) else {
            return;
        };
        if emote_commands::grappling(&them.state) || them.state.duel_in_progress() || duelling {
            return;
        }
        // `JP_HolsterPair`.
        let saber_out = |game: &Self, number: usize| {
            game.peer(number).is_some_and(|peer| {
                peer.state.weapon() == WP_SABER && peer.state.saber_holstered() == 0
            })
        };
        for number in [client, other] {
            if saber_out(self, number) {
                self.toggle_saber_command(number, level_time);
            }
        }
        if saber_out(self, client) || saber_out(self, other) {
            return;
        }
        // `JP_Hug` (`g_japlus_cmds.c:281-298`).
        let origins = [client, other].map(|number| {
            self.peer(number)
                .map_or([0.0; 3], |peer| peer.state.origin())
        });
        for (index, (number, name)) in [(client, "BOTH_HUGGER1"), (other, "BOTH_HUGGEE1")]
            .into_iter()
            .enumerate()
        {
            let (me, partner) = (origins[index], origins[1 - index]);
            let Some(peer) = self.peer_mut(number) else {
                continue;
            };
            let direction: [f32; 3] = std::array::from_fn(|axis| partner[axis] - me[axis]);
            let mut angles = peer.state.view_angles();
            angles[1] = yaw_of(direction);
            let command_angles = peer.last_command.angles;
            sjk_game_jka::triggers::face(&mut peer.state, angles, command_angles);
            peer.state.set_raw_field(PS_SABER_MOVE, 0);
            peer.state.set_raw_field(PS_SABER_BLOCKED, 0);
            peer.movement = peer.movement.reseeded(&peer.state);
            peer.movement.set_saber_blocking(0);
            if let Some(animation) = sjk_game_jka::legacy_animation_index(name) {
                use sjk_game_jka::pmove_anim::{
                    SETANIM_BOTH, SETANIM_FLAG_HOLD, SETANIM_FLAG_HOLDLESS, SETANIM_FLAG_OVERRIDE,
                };
                peer.movement.set_animation_parts(
                    SETANIM_BOTH,
                    animation as u16,
                    SETANIM_FLAG_OVERRIDE | SETANIM_FLAG_HOLD | SETANIM_FLAG_HOLDLESS,
                );
            }
            peer.movement.write_player_state(&mut peer.state);
        }
    }

    /// `Cmd_amflip_f` (`g_japlus_cmds.c:356-420`): on the ground with the saber in hand,
    /// it turns on or off (the on-sounds both ways, as JA+ plays them) with its style's
    /// show-off.
    fn flip(&mut self, client: usize, level_time: i32) {
        let Some(peer) = self.peer(client) else {
            return;
        };
        let state = &peer.state;
        if !peer.playing()
            || !emote_commands::can_act(state, peer.health)
            || state.raw_field(PS_FORCE_HAND_EXTEND).unwrap_or(0) != 0
            || peer.gear.packs.jetpack_on
            || state.weapon() != WP_SABER
            || state.saber_in_flight()
        {
            return;
        }
        let base = peer.movement.state().saber_anim_level_base;
        // `Q_irand(0, 10) >= 4`, drawn only for a holstered staff, as JA+ draws it.
        let staff_draw = state.raw_field(23) == Some(7) && state.saber_holstered() != 0;
        let staff_draw = staff_draw && self.deaths.rng.irand(0, 10) >= 4;
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let Some(animation) = emote_commands::flip_animation(&peer.state, base, staff_draw) else {
            return;
        };
        if !emote_commands::grounded(&peer.state) {
            return;
        }
        let state = &mut peer.state;
        let mut sounds = Vec::new();
        if state.stats[STAT_WEAPONS] & (1 << WP_SABER) != 0
            && state.weapon() == WP_SABER
            && state.saber_entity_num() != 0
            && !state.saber_in_flight()
        {
            // `JP_FlipToggleSaber` (`g_japlus_cmds.c:344-354`).
            let holstered = if state.saber_holstered() == 0 { 2 } else { 0 };
            state.set_raw_field(81, holstered);
            sounds.extend(SaberSound::both(true));
        }
        let origin = state.origin();
        self.play_pose(client, animation, false, level_time);
        self.saber_sounds(client, sounds, origin, CHAN_AUTO, level_time);
    }

    /// `Cmd_amknockmedown_f` (`g_japlus_cmds.c:427-458`).
    fn knock_me_down(&mut self, client: usize, rules: &PlayerCommandRules, level_time: i32) {
        let gametype = self.gametype;
        let Some(peer) = self.peer(client) else {
            return;
        };
        if !peer.playing() {
            return;
        }
        if !rules.knock_me_down || peer.state.duel_in_progress() {
            self.tell_text(client, rules.texts.knock_me_down_refused);
            return;
        }
        if gametype != GAMETYPE_FFA {
            self.tell_text(client, rules.texts.gametype_refused);
            return;
        }
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if !emote_commands::can_act(&peer.state, peer.health)
            || peer.state.raw_field(PS_FORCE_HAND_EXTEND).unwrap_or(0) != 0
            || peer.gear.packs.jetpack_on
        {
            return;
        }
        let sound = emote_commands::knock_me_down(&mut peer.state, &mut peer.knockdown, level_time);
        peer.movement = peer.movement.reseeded(&peer.state);
        if let Some(sound) = sound {
            let _ = self.pool.spawn_temporary(sound.state(), level_time, None);
        }
    }

    /// `Cmd_amdropsaber_f` (`g_japlus_cmds.c:465-482`): the saber knocked out of the hand
    /// where it stands.
    fn drop_saber(&mut self, client: usize, rules: &PlayerCommandRules, level_time: i32) {
        let gametype = self.gametype;
        let Some(peer) = self.peer(client) else {
            return;
        };
        if !peer.playing() {
            return;
        }
        if !rules.drop_saber {
            self.tell_text(client, rules.texts.drop_saber_refused);
            return;
        }
        if gametype != GAMETYPE_FFA {
            self.tell_text(client, rules.texts.gametype_refused);
            return;
        }
        let state = &peer.state;
        if emote_commands::can_act(state, peer.health)
            && state.weapon() == WP_SABER
            && state.saber_entity_num() != 0
            && !state.saber_in_flight()
        {
            self.knock_out(client, [0.0; 3], level_time);
        }
    }

    /// `Cmd_jetpack_f` (`g_japlus_cmds.c:489-523`): the jetpack item taken (when it is
    /// off), or given on the ground where the settings and the game type allow it.
    fn jetpack_command(&mut self, client: usize, rules: &PlayerCommandRules) {
        let gametype = self.gametype;
        let jedi_vs_merc = self.cvars.integer(b"g_jediVmerc") != 0;
        let texts = rules.texts;
        if matches!(
            gametype,
            GAMETYPE_SIEGE | GAMETYPE_DUEL | GAMETYPE_POWERDUEL | GAMETYPE_CTF
        ) {
            self.tell_text(client, texts.jetpack_refused);
            return;
        }
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let jetpack = 1 << sjk_game_jka::holdables::HI_JETPACK;
        if peer.state.stats[STAT_HOLDABLE_ITEMS] & jetpack != 0 {
            if !peer.gear.packs.jetpack_on {
                peer.state.stats[STAT_HOLDABLE_ITEMS] &= !jetpack;
                peer.movement = peer.movement.reseeded(&peer.state);
            }
            return;
        }
        let state = &peer.state;
        if !emote_commands::grounded(state) {
            return;
        }
        let refusal = if !rules.jetpack || state.duel_in_progress() {
            Some(texts.jetpack_refused)
        } else if jedi_vs_merc && !state.true_non_jedi() && state.is_true_jedi() {
            Some(texts.jetpack_jedi)
        } else if gametype == GAMETYPE_TEAM && !jedi_vs_merc {
            Some(texts.jetpack_team)
        } else {
            None
        };
        if let Some(text) = refusal {
            self.tell_text(client, text);
            return;
        }
        peer.state.stats[STAT_HOLDABLE_ITEMS] |= jetpack;
        peer.movement = peer.movement.reseeded(&peer.state);
    }

    /// `ClientUserinfoChanged` for `client`'s userinfo with `pairs` set in it (a mod
    /// command's own `trap->SetUserinfo`).
    fn set_userinfo_keys(&mut self, client: usize, pairs: &[(&[u8], &[u8])]) {
        let Some(peer) = self.peer(client) else {
            return;
        };
        let mut userinfo = peer.userinfo.clone();
        for (key, value) in pairs {
            userinfo = sjk_protocol::info_set_value(&userinfo, key, value);
        }
        self.userinfo_changed(client, &userinfo);
    }

    /// `Cmd_amcolor_f` (`g_japlus_cmds.c:530-555`): the model's tint, 0 to 255 each (768
    /// keeps the model's own).
    fn skin_color(&mut self, client: usize, arguments: &[&[u8]], rules: &PlayerCommandRules) {
        let valid = |word: &[u8]| {
            let value = atoi(word);
            value == 0x300 || (0..=255).contains(&value)
        };
        let [red, green, blue, ..] = arguments else {
            self.tell_text(client, rules.texts.skin_color_usage);
            return;
        };
        if ![red, green, blue].into_iter().all(|word| valid(word)) {
            self.tell_text(client, rules.texts.skin_color_usage);
            return;
        }
        self.set_userinfo_keys(
            client,
            &[
                (b"char_color_red", red),
                (b"char_color_green", green),
                (b"char_color_blue", blue),
            ],
        );
    }

    /// `Cmd_sabercolor_f` (`g_japlus_cmds.c:563-595`): a saber's RGB colour, for a
    /// player with the plugin, its saber in hand and not swinging.
    fn saber_color(&mut self, client: usize, arguments: &[&[u8]], rules: &PlayerCommandRules) {
        let Some(peer) = self.peer(client) else {
            return;
        };
        if !peer.playing() {
            return;
        }
        let plugin = self
            .server_mod
            .is_some_and(|server_mod| server_mod.client_has_plugin(&peer.userinfo));
        if !plugin {
            self.tell_text(client, rules.texts.saber_color_plugin);
            return;
        }
        let state = &peer.state;
        let in_hand = state.stats[STAT_WEAPONS] & (1 << WP_SABER) != 0
            && state.weapon() == WP_SABER
            && state.saber_entity_num() != 0
            && !state.saber_in_flight();
        if !in_hand || sjk_game_jka::saber_rules::in_attack(state.saber_move()) {
            return;
        }
        let &[number, red, green, blue] = arguments else {
            self.tell_text(client, rules.texts.saber_color_usage);
            return;
        };
        let [number, red, green, blue] = [number, red, green, blue].map(atoi);
        let channel = 0..=255;
        if !(1..=2).contains(&number)
            || ![red, green, blue]
                .iter()
                .all(|value| channel.contains(value))
        {
            self.tell_text(client, rules.texts.saber_color_usage);
            return;
        }
        let rgb = (red | (green << 8) | (blue << 16)).to_string();
        let (rgb_key, color_key) = (format!("cp_sbRGB{number}"), format!("color{number}"));
        // `SABER_RGB`.
        self.set_userinfo_keys(
            client,
            &[
                (rgb_key.as_bytes(), rgb.as_bytes()),
                (color_key.as_bytes(), b"6"),
            ],
        );
    }

    /// `Cmd_saber_f` (`g_japlus_cmds.c:598-635`, not in a duel: `:903-906`): the sabers of
    /// `saber1` and `saber2` while playing (the second "none" when not named), no more
    /// often than the mod's interval, then `G_UpdateSaberFromUserinfo` and
    /// `G_ClampSaberLevel` (VisionJK `g_client.c:3221-3317`).
    fn saber_switch(
        &mut self,
        client: usize,
        arguments: &[&[u8]],
        rules: &PlayerCommandRules,
        level_time: i32,
    ) {
        let gametype = self.gametype;
        let Some(peer) = self.peer(client) else {
            return;
        };
        if peer.state.duel_in_progress() || !peer.playing() {
            return;
        }
        if rules.saber_switch_interval == 0 {
            self.tell_text(client, rules.texts.saber_switch_refused);
            return;
        }
        if matches!(
            gametype,
            GAMETYPE_SIEGE | GAMETYPE_POWERDUEL | GAMETYPE_DUEL
        ) {
            self.tell_text(client, rules.texts.gametype_refused);
            return;
        }
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let state = &peer.state;
        let in_hand = state.stats[STAT_WEAPONS] & (1 << WP_SABER) != 0
            && state.weapon() == WP_SABER
            && state.saber_entity_num() != 0
            && !state.saber_in_flight();
        if !in_hand || peer.saber_switch_time >= level_time {
            return;
        }
        peer.saber_switch_time = level_time + rules.saber_switch_interval;
        let saber_move = peer.state.saber_move();
        if sjk_game_jka::saber_rules::in_attack(saber_move)
            || sjk_game_jka::saber_rules::in_transition_any(saber_move)
        {
            return;
        }
        if let Some(first) = arguments.first() {
            let second = arguments.get(1).copied().unwrap_or(b"none");
            self.set_userinfo_keys(client, &[(b"saber1", first), (b"saber2", second)]);
        }
        if !self.spawn_sabers(client) {
            return;
        }
        // The new sabers' names to everyone (`ClientUserinfoChanged`).
        if let Some(userinfo) = self.peer(client).map(|peer| peer.userinfo.clone()) {
            self.userinfo_changed(client, &userinfo);
        }
        self.restyle_sabers(client);
    }

    /// `G_UpdateSaberFromUserinfo`'s stance for the sabers just set, then
    /// `G_ClampSaberLevel`.
    fn restyle_sabers(&mut self, client: usize) {
        use sjk_game_jka::client_spawn::SaberKit;
        const FP_SABER_OFFENSE: usize = 15;
        let siege = self.gametype == GAMETYPE_SIEGE;
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let attack = i32::from(peer.force.levels[FP_SABER_OFFENSE]);
        let mut base = peer.movement.state().saber_anim_level_base;
        let style = &mut peer.session.saber_style;
        let set = |state: &mut sjk_protocol::PlayerState, level: i32| {
            // `fd.saberAnimLevel`, `fd.saberDrawAnimLevel`.
            state.set_raw_field(23, level as u32);
            state.set_raw_field(25, level as u32);
        };
        match peer.sabers.kit() {
            SaberKit::Dual => {
                base = 6;
                set(&mut peer.state, 6);
            }
            SaberKit::Staff => set(&mut peer.state, 7),
            SaberKit::Single => {
                style.session_level = style.session_level.clamp(1, 3);
                if !siege && style.session_level > attack {
                    style.session_level = attack;
                }
                base = style.session_level as u8;
                set(&mut peer.state, style.session_level);
            }
        }
        if !siege {
            sjk_game_jka::saber_stance::settle_after_set(
                &mut peer.state,
                &peer.sabers.hands,
                &mut peer.saber.cycle_queue,
                &mut base,
            );
        }
        // `G_ClampSaberLevel`.
        let level = peer.state.raw_field(23).unwrap_or(0) as i32;
        let draw = peer.state.raw_field(25).unwrap_or(0) as i32;
        let style = &mut peer.session.saber_style;
        if !matches!(level, 6 | 7) && level == draw && level == style.session_level {
            style.session_level = style.session_level.clamp(1, 3);
            if !siege && style.session_level > attack {
                style.session_level = attack;
            }
            set(&mut peer.state, style.session_level);
        }
        // Set for this life: the next spawn does not set them again.
        peer.session.sabers_set = true;
        peer.movement = peer.movement.reseeded(&peer.state);
        peer.movement.set_saber_anim_level_base(base);
    }

    /// `Cmd_duelWhois_f` (`g_japlus_info.c:125-142`): every duel being fought, each pair
    /// once.
    fn duel_list(&mut self, client: usize, rules: &PlayerCommandRules) {
        self.tell_text(client, rules.texts.duel_list);
        let (before, between, after) = rules.texts.duel_pair;
        let places = self.players.places();
        for number in 0..places {
            let Some(peer) = self.peer(number).filter(|peer| peer.begun) else {
                continue;
            };
            if !peer.state.duel_in_progress() {
                continue;
            }
            let other = usize::from(peer.state.duel_index());
            if other >= places || other < number {
                continue;
            }
            let Some(them) = self.peer(other) else {
                continue;
            };
            let line = [
                before.as_bytes(),
                &peer.name,
                between.as_bytes(),
                &them.name,
                after.as_bytes(),
            ]
            .concat();
            self.told.push(Told::One(client, line));
        }
    }

    /// `ClientThink_real`'s movement type (`g_active.c:2133-2153`): a mod's held emote
    /// leaves a player in `PM_FREEZE` for one move only, and its next think starts from
    /// `PM_NORMAL` (the base game never freezes a player).
    pub(super) fn thaw_frozen_emote(&mut self, client: usize) {
        if let Some(peer) = self.peer_mut(client)
            && peer.state.movement_type() == PM_FREEZE
        {
            peer.state.set_movement_type(PM_NORMAL);
            peer.movement = peer.movement.reseeded(&peer.state);
        }
    }

    /// The held emotes the server freezes this frame: the mod's choice, all of them while
    /// a player's client lacks the mod's plugin (`JP_AllPlayersHavePlugin`, VisionJK
    /// `g_japlus_misc.c:807-818`: bots count as having it), put into every player's
    /// movement where it changed.
    pub(super) fn update_emote_freeze(&mut self) {
        use sjk_game_jka::pmove_japlus_emotes::EmoteFreeze;
        let Some(server_mod) = self.server_mod else {
            return;
        };
        let mut freeze = self.mod_movement.ja_plus.emote_freeze;
        if freeze == EmoteFreeze::AtEase {
            let everyone = (0..self.players.places()).all(|number| {
                self.peer(number).is_none_or(|peer| {
                    !peer.begun || peer.bot || server_mod.client_has_plugin(&peer.userinfo)
                })
            });
            if !everyone {
                freeze = EmoteFreeze::Held;
            }
        }
        if let Some(world) = self.server.world_mut(self.world) {
            for (_, peer) in world.entities_mut() {
                let mut config = peer.movement.config();
                if config.ja_plus.emote_freeze != freeze {
                    config.ja_plus.emote_freeze = freeze;
                    peer.movement.set_config(config);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeGame, Told, yaw_of};
    use crate::bridge::Identity;
    use crate::cvars::Cvars;

    /// A mapless server started with `g_mod` `server_mod`, players 0 and 1 standing 30
    /// units apart and facing each other, 2 off to the side; 0 says it runs the JA+
    /// plugin.
    fn game(server_mod: &str) -> NativeGame {
        let mut cvars = Cvars::new();
        cvars.set(b"g_mod", server_mod.as_bytes());
        cvars.set(b"g_userinfoValidate", b"0");
        cvars.register_server();
        let identity = Identity {
            hostname: b"test".to_vec(),
            mapname: b"mp/ffa3".to_vec(),
        };
        let mut game = NativeGame::with_cvars(identity, None, 4, 4, cvars).unwrap();
        for (client, origin, yaw) in [
            (0, [500.0, 0.0, 0.0], 0.0),
            (1, [530.0, 0.0, 0.0], 180.0),
            (2, [500.0, 300.0, 0.0], 0.0),
        ] {
            let userinfo = if client == 0 {
                format!(r"\name\P{client}\cjp_client\1.4 SJK")
            } else {
                format!(r"\name\P{client}")
            };
            game.connect_client(client, userinfo.as_bytes(), true)
                .unwrap();
            game.begin(client, 0, None);
            let peer = game.peer_mut(client).unwrap();
            peer.state.set_origin(origin);
            peer.state.set_view_angles([0.0, yaw, 0.0]);
            peer.state.set_ground_entity_num(1_022);
            // A human, `WP_SABER` in hand (`saberEntityNum`), its saber lit.
            peer.bot = false;
            peer.state.stats[4] |= 1 << 3;
            peer.state.set_raw_field(47, 3);
            peer.state.set_raw_field(31, 40 + client as u32);
            peer.state.set_raw_field(81, 0);
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        game.last_frame_time = 5_000;
        game
    }

    /// What `client` was told since the last look, as text.
    fn told(game: &mut NativeGame, client: usize) -> Vec<String> {
        game.told
            .drain(..)
            .filter_map(|told| match told {
                Told::One(to, text) if to == client => {
                    Some(String::from_utf8_lossy(&text).into_owned())
                }
                _ => None,
            })
            .collect()
    }

    fn command(game: &mut NativeGame, client: usize, text: &str) -> Vec<String> {
        game.told.clear();
        game.player_command(client, text.as_bytes(), 5_000);
        told(game, client)
    }

    fn hand_extend(game: &NativeGame, client: usize) -> (u32, u32) {
        let state = &game.peer(client).unwrap().state;
        (state.raw_field(80).unwrap(), state.raw_field(89).unwrap())
    }

    fn animation(name: &str) -> u32 {
        sjk_game_jka::legacy_animation_index(name).unwrap() as u32
    }

    #[test]
    fn the_base_game_has_none_of_the_commands() {
        let mut game = game("base");
        for name in [
            "amsit",
            "sleep",
            "amhug",
            "amkiss",
            "amflip",
            "amknockmedown",
            "amdropsaber",
            "amcolor 1 2 3",
            "sabercolor 1 255 0 0",
            "saber single_1",
            "ammove",
            "amDMGs",
            "duelWhois",
            "refuseTele",
        ] {
            let word = name.split(' ').next().unwrap();
            assert_eq!(
                command(&mut game, 0, name),
                [format!("print \"Unknown command {word}\n\"")],
                "{name}"
            );
        }
        assert_eq!(hand_extend(&game, 0), (0, 0));
        // `jetpack` stays the cheat.
        assert_eq!(
            command(&mut game, 0, "jetpack"),
            ["print \"@@@NOCHEATS\n\""]
        );
        let mut printed = Vec::new();
        let handled = game.game_console_command(&[b"modVersion"], 0, &mut |text: &[u8]| {
            printed.extend_from_slice(text)
        });
        assert!(!handled && printed.is_empty());
    }

    #[test]
    fn a_ja_plus_emote_plays_its_pose_where_allowed() {
        let mut game = game("japlus");
        assert!(command(&mut game, 0, "amsit3").is_empty());
        assert_eq!(hand_extend(&game, 0), (10, animation("BOTH_SIT6")));
        // `jp_emotDisallow`'s bit, told first.
        game.set_cvar("jp_emotDisallow", &(1 << 17).to_string());
        assert_eq!(
            command(&mut game, 1, "AMSIT"),
            ["print \"this Emot is not allowed on this server!.\n\""]
        );
        assert_eq!(hand_extend(&game, 1), (0, 0));
        // A nod is free; `amdie` is for free-for-all only, silent elsewhere.
        assert!(command(&mut game, 1, "amnod").is_empty());
        assert_eq!(hand_extend(&game, 1), (10, animation("BOTH_HEADNOD")));
        game.set_gametype(6);
        assert_eq!(
            command(&mut game, 2, "amsit2"),
            ["print \"this Emot is not compatible with this gametype!.\n\""]
        );
        assert!(command(&mut game, 2, "amdie").is_empty());
        assert_eq!(hand_extend(&game, 2), (0, 0));
    }

    #[test]
    fn ja_plus_kiss_hug_and_flip() {
        let mut game = game("japlus");
        assert_eq!(
            command(&mut game, 0, "amkiss"),
            [
                "print \"the new GLA animations are not loaded on this server or the admin disallowed them!.\n\""
            ]
        );
        // The hug: both sabers away, the two facing each other.
        assert!(command(&mut game, 0, "amhug").is_empty());
        for client in [0, 1] {
            assert_eq!(game.peer(client).unwrap().state.saber_holstered(), 2);
        }
        let yaw =
            |game: &NativeGame, client: usize| game.peer(client).unwrap().state.view_angles()[1];
        assert_eq!(yaw(&game, 0), 0.0);
        assert!((yaw(&game, 1) - 180.0).abs() < 1e-3);
        // Nobody in front of 2.
        assert!(command(&mut game, 2, "amhug").is_empty());
        assert_eq!(game.peer(2).unwrap().state.saber_holstered(), 0);
        // The flip lights a holstered saber with the draw.
        let peer = game.peer_mut(0).unwrap();
        peer.state.set_raw_field(23, 2);
        peer.state.set_raw_field(10, 0);
        assert!(command(&mut game, 0, "amflip").is_empty());
        assert_eq!(game.peer(0).unwrap().state.saber_holstered(), 0);
        assert_eq!(hand_extend(&game, 0), (10, animation("BOTH_STAND1TO2")));
    }

    #[test]
    fn ja_plus_knock_me_down_drop_saber_and_jetpack_follow_their_settings() {
        let mut game = game("japlus");
        assert!(command(&mut game, 0, "amknockmedown").is_empty());
        assert_eq!(hand_extend(&game, 0).0, 8);
        game.set_cvar("jp_allowAmKnockmedown", "0");
        assert_eq!(
            command(&mut game, 1, "amknockmedown"),
            ["amknockmedown is not allowed!\n"]
        );
        // `jp_allowAmDropSaber` is off by default.
        assert_eq!(
            command(&mut game, 1, "amdropsaber"),
            ["amdropsaber is not allowed!\n"]
        );
        // `jp_allowJetpack` 0 by default; given once allowed, taken again.
        let jetpack = |game: &NativeGame| game.peer(1).unwrap().state.stats[2] & (1 << 7) != 0;
        assert_eq!(
            command(&mut game, 1, "jetpack"),
            ["print \"Cmd jetpack not allowed\n\""]
        );
        game.set_cvar("jp_allowJetpack", "1");
        assert!(command(&mut game, 1, "jetpack").is_empty());
        assert!(jetpack(&game));
        assert!(command(&mut game, 1, "jetpack").is_empty());
        assert!(!jetpack(&game));
        game.set_gametype(8);
        assert_eq!(
            command(&mut game, 1, "jetpack"),
            ["print \"Cmd jetpack not allowed\n\""]
        );
    }

    #[test]
    fn ja_plus_colours_lists_and_teleport_refusal() {
        let mut game = game("japlus");
        assert_eq!(
            command(&mut game, 1, "amcolor 300 0 0"),
            [
                "print \"Usage: amcolor <R> <G> <B> where R,G,B are (Red,Green,Blue) integers between 0 and 255\n\""
            ]
        );
        command(&mut game, 1, "amcolor 10 20 30");
        let userinfo = game.peer(1).unwrap().userinfo.clone();
        assert_eq!(
            sjk_protocol::info_value(&userinfo, b"char_color_green"),
            Some(&b"20"[..])
        );
        // `sabercolor` needs the plugin: 0 has it, 1 not.
        assert_eq!(
            command(&mut game, 1, "sabercolor 1 255 0 0"),
            [
                "print \"/sabercolor is only usable with the JA+ client PLugin greater or equal to 1.1.\n\""
            ]
        );
        command(&mut game, 0, "sabercolor 2 255 0 1");
        let userinfo = game.peer(0).unwrap().userinfo.clone();
        assert_eq!(
            sjk_protocol::info_value(&userinfo, b"cp_sbRGB2"),
            Some(&b"65791"[..])
        );
        assert_eq!(
            sjk_protocol::info_value(&userinfo, b"color2"),
            Some(&b"6"[..])
        );
        assert!(command(&mut game, 0, "ammove").len() > 5);
        assert!(command(&mut game, 0, "amdmgs")[0].contains("Damages LIST"));
        assert_eq!(
            command(&mut game, 0, "duelWhois"),
            ["print \"^5All Duelers :\n\""]
        );
        assert_eq!(
            command(&mut game, 0, "refuseTele"),
            ["print \"You refuse to be teleported by admins.\n\""]
        );
        assert!(game.peer(0).unwrap().refuse_teleport);
        assert_eq!(
            command(&mut game, 0, "refusetele"),
            ["print \"You accept to be teleported by admins.\n\""]
        );
        // The server console's own.
        let mut printed = Vec::new();
        let handled = game.game_console_command(&[b"modVersion"], 0, &mut |text: &[u8]| {
            printed.extend_from_slice(text)
        });
        assert!(handled);
        assert_eq!(printed, b"JA+ Version JA+ Mod v2.4 Build 7\n");
    }

    #[test]
    fn the_server_freezes_held_emotes_while_a_player_lacks_the_plugin() {
        use sjk_game_jka::pmove_japlus_emotes::EmoteFreeze;
        let freeze =
            |game: &NativeGame| game.peer(0).unwrap().movement.config().ja_plus.emote_freeze;
        let mut base = game("base");
        base.update_emote_freeze();
        assert_eq!(freeze(&base), EmoteFreeze::Off);
        let mut game = game("japlus");
        // 1 and 2 have no plugin.
        game.update_emote_freeze();
        assert_eq!(freeze(&game), EmoteFreeze::Held);
        for client in [1, 2] {
            let userinfo = format!(r"\name\P{client}\cjp_client\1.4JAPRO");
            game.userinfo_changed(client, userinfo.as_bytes());
        }
        game.update_emote_freeze();
        assert_eq!(freeze(&game), EmoteFreeze::AtEase);
        game.set_cvar("jp_fixEmot", "0");
        game.update_emote_freeze();
        assert_eq!(freeze(&game), EmoteFreeze::Held);
        // A frozen move is thawed at the next think.
        game.peer_mut(0).unwrap().state.set_movement_type(6);
        game.thaw_frozen_emote(0);
        assert_eq!(game.peer(0).unwrap().state.movement_type(), 0);
    }

    #[test]
    fn the_ja_plus_saber_change_is_gated_as_ja_plus_gates_it() {
        let mut game = game("japlus");
        // Allowed every 1.5 s by default: the second try within it does nothing.
        assert!(command(&mut game, 0, "saber").is_empty());
        assert_eq!(game.peer(0).unwrap().saber_switch_time, 6_500);
        game.peer_mut(0).unwrap().saber_switch_time = 5_500;
        assert!(command(&mut game, 0, "saber").is_empty());
        assert_eq!(game.peer(0).unwrap().saber_switch_time, 5_500);
        game.set_cvar("jp_allowSaberSwitch", "0");
        assert_eq!(
            command(&mut game, 1, "saber"),
            ["print \"/saber ^1cmd is not allowed on this server!.
\""]
        );
        game.set_cvar("jp_allowSaberSwitch", "1500");
        game.set_gametype(3);
        assert_eq!(
            command(&mut game, 1, "saber"),
            ["print \"this is not compatible with this gametype!.
\""]
        );
    }

    #[test]
    fn ja_plus_commands_wait_out_the_intermission() {
        let mut game = game("japlus");
        game.match_end.intermission_time = 5_000;
        assert_eq!(
            command(&mut game, 0, "amsit"),
            ["print \"@@@CANNOT_TASK_INTERMISSION (amsit) \n\""]
        );
    }

    #[test]
    fn yaw_of_follows_vectoyaw() {
        assert_eq!(yaw_of([0.0, 0.0, 5.0]), 0.0);
        assert_eq!(yaw_of([1.0, 0.0, 0.0]), 0.0);
        assert!((yaw_of([0.0, 1.0, 0.0]) - 90.0).abs() < 1e-4);
        assert!((yaw_of([0.0, -1.0, 0.0]) - 270.0).abs() < 1e-4);
    }
}
