//! Private duels (`Cmd_EngageDuel_f`, `g_cmds.c:2996-3143`, and `ClientThink_real`'s duel,
//! `g_active.c:2405-2528`): outside the team games and the duel game types, a player with
//! its saber in hand challenges the one it aims at within 256 units; the other, aiming
//! back while the challenge stands (five seconds), accepts. Both are holstered and held
//! still two seconds, then their sabers light and they fight each other alone — no one
//! else can hurt them or be hurt by them (`G_Damage`, `g_combat.c:4540-4556`; a missile at
//! a duellist is spent on it harmlessly, `G_MissileImpact`'s `killProj`), and between them
//! only the saber hurts. The duel ends when one dies (the other healed to its maximum and
//! protected as at a spawn, everyone told), when either leaves, or when they are 1024
//! units apart.
//!
//! The wire fields are the player state's own (`duelInProgress`, `duelIndex`,
//! `duelTime`); the movement freezes and arms a duellist from them.
//!
//! A server mod may run other rules, given as plain values ([`DuelRules`], the base
//! game's by default): JA+'s are those of VisionJK (Blackwolf's GPL-2.0 rebuild of the JA+
//! 2.4 B7 server module), `codemp/game/g_cmds.c:3262-3452` (the challenge),
//! `g_active.c:2439-2572` (the duel's think), `g_combat.c:4580-4601` (the full-force
//! duel's damage) and `japlus/g_japlus_dim.c` (duellists and the others do not touch).

use sjk_protocol::PlayerState;

/// `EV_PRIVATE_DUEL`: 1 accepted, 2 begun, 0 over.
pub const EV_PRIVATE_DUEL: u32 = 15;
const PS_WEAPON_TIME: usize = 10;
const PS_EFLAGS: usize = 17;
const PS_SPEED: usize = 12;
const PS_DUEL_INDEX: usize = 44;
const PS_WEAPON: usize = 47;
const PS_FORCE_HAND_EXTEND: usize = 80;
const PS_SABER_HOLSTERED: usize = 81;
const PS_SABER_IN_FLIGHT: usize = 88;
const PS_DUEL_TIME: usize = 118;
const PS_DUEL_IN_PROGRESS: usize = 119;
const PS_FORCE_POWER: usize = 18;
const PS_FORCE_DODGE_ANIM: usize = 89;
const STAT_HEALTH: usize = 0;
const STAT_HOLDABLE_ITEMS: usize = 2;
const STAT_ARMOR: usize = 5;
const STAT_MAX_HEALTH: usize = 8;
const WP_SABER: u32 = 3;
const HANDEXTEND_DUELCHALLENGE: u32 = 9;
const HANDEXTEND_TAUNT: u32 = 10;
const EF_INVULNERABLE: u32 = 1 << 27;
/// `PW_CLOAKED`.
const PW_CLOAKED: usize = 11;
/// `TEAM_SPECTATOR`.
const TEAM_SPECTATOR: i32 = 3;
const GT_FFA: i32 = 0;
const GT_DUEL: i32 = 3;
const GT_POWERDUEL: i32 = 4;
const GT_TEAM: i32 = 6;
const MOD_SABER: u32 = 3;
const MAX_CLIENTS: u16 = 32;
/// `g_spawnInvulnerability`: the winner's protection.
const SPAWN_INVULNERABILITY: i32 = 3_000;

/// `HI_JETPACK`, a bit of `STAT_HOLDABLE_ITEMS`.
const HI_JETPACK: u32 = 7;
/// What a JA+ winner's armour is set to (VisionJK `g_active.c:2541`).
const JA_PLUS_WINNER_ARMOR: u32 = 25;
/// The health both JA+ duellists start with (VisionJK `g_cmds.c:3390-3391`).
const JA_PLUS_START_HEALTH: i32 = 100;
/// `privateDuelTime` while a JA+ duel goes on (VisionJK `g_active.c:2446`): no challenge
/// for two seconds after it. The base game never reads its own ten seconds.
const JA_PLUS_REST: i32 = 2_000;

/// Who is told how much health and armour a JA+ duel's winner kept
/// (`jp_endDuelBroadcastMSG`, VisionJK `g_active.c:2504-2524`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WinnerReport {
    /// The base game's: everyone, centred, `@@@PLDUELWINNER`.
    #[default]
    Base,
    /// 0: nobody.
    Silent,
    /// 1: the two centred, and everyone in the console.
    Everyone,
    /// 2: the two only, centred and in the console.
    Duellists,
}

impl WinnerReport {
    /// The report `jp_endDuelBroadcastMSG` names: 0 or less nobody, 1 everyone, more the
    /// two duellists (`> 0`, then `== 1`).
    pub fn from_setting(value: i32) -> Self {
        match value {
            ..=0 => Self::Silent,
            1 => Self::Everyone,
            _ => Self::Duellists,
        }
    }
}

/// The private-duel rules a server runs, as plain values. [`Default`] is the base game's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuelRules {
    /// JA+'s challenge and duel (VisionJK `g_cmds.c:3262-3452`, `g_active.c:2439-2572`):
    /// a challenge with any weapon, the saber only needed to accept; none with a jetpack
    /// or the cloak (said), in a melee grapple or from a spectator; the game types it is
    /// refused in said (`@@@NODUEL_GAMETYPE`), as is a challenge less than two seconds
    /// after a duel (`@@@CANTDUEL_JUSTDID`); both duellists start with 100 health, a full
    /// Force pool and [`Self::start_armor`]; the sabers stay off when it begins; the
    /// winner keeps 25 armour and is reported as [`Self::winner_report`] says; and the
    /// full-force duel ([`Duellist::full_force`]).
    pub ja_plus: bool,
    /// Duellists and everybody else do not block, hit or push each other
    /// (`jp_DuelAlpha` 0 or more, VisionJK `japlus/g_japlus_dim.c`), in the game types
    /// [`Self::isolates`] names: see [`isolated`].
    pub isolation: bool,
    /// Private duels in team deathmatch, against the other team (`jp_allowTeamDuel`).
    pub team_duels: bool,
    /// The duel ends when its two are 1024 units apart (`jp_removeDistanceDuelLimit` 0).
    pub distance_limit: bool,
    /// The two bow when the duel is accepted (`jp_autoDuelBOW` 1, VisionJK
    /// `g_cmds.c:3404-3412`), instead of the acceptor's challenge gesture.
    pub bow: bool,
    /// The armour both duellists start with under [`Self::ja_plus`]
    /// (`jp_duelStartArmor`).
    pub start_armor: i32,
    /// How the winner is announced.
    pub winner_report: WinnerReport,
    /// Milliseconds a point of Force takes to come back in a private duel
    /// (`jp_privateDuelForceRegen`, VisionJK `w_force.c:6256-6257`); `None` leaves the
    /// server's own setting.
    pub force_regen_time: Option<i32>,
}

impl Default for DuelRules {
    fn default() -> Self {
        Self {
            ja_plus: false,
            isolation: false,
            team_duels: false,
            distance_limit: true,
            bow: false,
            start_armor: 0,
            winner_report: WinnerReport::Base,
            force_regen_time: None,
        }
    }
}

impl DuelRules {
    /// Whether duellists and the others do not touch in `gametype`: only in a
    /// free-for-all and team deathmatch (`JP_NonInterferenceRules`, VisionJK
    /// `g_japlus_dim.c:130-133`, and `G_canInterferWith`, `:66-68`).
    pub fn isolates(self, gametype: i32) -> bool {
        self.isolation && (gametype == GT_FFA || gametype == GT_TEAM)
    }
}

/// `G_TempUnlink`'s rule (VisionJK `g_japlus_dim.c:144-164`), under
/// [`DuelRules::isolates`]: whether a player duelling `my_duel`'s opponent (`None` for
/// one not duelling) passes through player `other`: its traces and moves leave it out,
/// and they cannot hit, push or Force each other (`G_canInterferWith`, `:56-82`). One not
/// duelling passes through every duellist; a duellist through everyone but its opponent,
/// duelling it back. The JA+ client plugin predicts the same (`CG_JPNonInterference` in
/// VisionJK's `cgame/cg_predict.c:465-497`).
pub fn isolated(
    my_duel: Option<u16>,
    other: u16,
    other_duelling: bool,
    other_duels_me: bool,
) -> bool {
    match my_duel {
        None => other_duelling,
        Some(opponent) => !(other_duelling && other_duels_me && opponent == other),
    }
}

/// The players whose private duel is a JA+ full-force duel, a bit each: none on the base
/// game. The Force powers between its two are let through and hurt (VisionJK
/// `bg_misc.c:1756-1760`, `w_force.c:589-601`, `g_combat.c:4586,4597`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FullForce(pub u64);

impl FullForce {
    /// Whether player `client` duels with full Force.
    pub fn has(self, client: u16) -> bool {
        client < 64 && self.0 >> client & 1 != 0
    }
}

/// `ForcePowerUsableOn`'s duel rule (`w_force.c:589-601`): a duellist uses no power on
/// anybody, nor anybody on a duellist, but for the two of a full-force duel on each other
/// (VisionJK `w_force.c:589-601`).
/// `attacker_duel` and `other_duel` are their [`duel_of`].
pub fn force_refused(
    attacker: u16,
    attacker_duel: Option<u16>,
    other: u16,
    other_duel: Option<u16>,
    full_force: FullForce,
) -> bool {
    let pair = attacker_duel == Some(other) && other_duel == Some(attacker);
    (attacker_duel.is_some() && !(pair && full_force.has(attacker)))
        || (other_duel.is_some() && !(pair && full_force.has(other)))
}

/// The opponent `state`'s player duels, `None` outside a duel.
pub fn duel_of(state: &PlayerState) -> Option<u16> {
    state.duel_in_progress().then(|| state.duel_index())
}

/// Whether `state`'s player duels somebody other than player `other`.
pub fn elsewhere(state: &PlayerState, other: u16) -> bool {
    state.duel_in_progress() && state.duel_index() != other
}

/// `G_Damage`'s duel rule (`g_combat.c:4540-4556`): a duellist is hurt by nobody but its
/// opponent, and by it only with the saber; a duellist hurts nobody but its opponent, and
/// it only with the saber. In a JA+ full-force duel the opponent hurts with anything
/// (VisionJK `g_combat.c:4580-4601`). `attacker` is a client's number and state.
pub fn damage_refused(
    target: u16,
    target_state: &PlayerState,
    attacker: Option<(u16, &PlayerState)>,
    means: u32,
    full_force: FullForce,
) -> bool {
    let Some((attacker, attacker_state)) = attacker else {
        return false;
    };
    if target_state.duel_in_progress()
        && (attacker != target_state.duel_index()
            || (means != MOD_SABER && !full_force.has(attacker)))
    {
        return true;
    }
    attacker_state.duel_in_progress()
        && (target != attacker_state.duel_index()
            || (means != MOD_SABER && !full_force.has(target)))
}

/// A player as a duel reads and changes it.
pub struct Duellist<'a> {
    pub number: u16,
    pub state: &'a mut PlayerState,
    /// `gentity_t::health`.
    pub health: &'a mut i32,
    /// `pers.netname`, as messages print it.
    pub name: &'a [u8],
    /// `forceHandExtendTime` (the knockdown's memory).
    pub hand_extend_time: &'a mut i32,
    /// `invulnerableTimer`.
    pub invulnerable_until: &'a mut i32,
    /// `sess.sessionTeam`, `inuse`.
    pub team: i32,
    /// `fd.forcePowerMax`, which a JA+ duel fills the pool to.
    pub force_max: i32,
    /// `pers.jp.fullForceDuel` (JA+): the challenge, or the duel, is a full-force one.
    pub full_force: &'a mut bool,
    /// `fd.privateDuelTime` (JA+): no challenge before this time.
    pub rest_until: &'a mut i32,
}

/// What a challenge or a duel's think asks the caller to do besides the states.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Told {
    /// Server commands: to a client, or to everyone (`None`).
    pub commands: Vec<(Option<u16>, Vec<u8>)>,
    /// Saber sounds (`G_Sound` on `CHAN_AUTO`) on a player: `true` its on-sound, `false`
    /// its off-sound — for each of its two sabers, the second counted as the game counts
    /// it.
    pub sounds: Vec<(u16, crate::generic_commands::SaberSound)>,
    /// The players an event was added to (`G_AddEvent`: `eventTime` is the caller's).
    pub raised: Vec<u16>,
}

/// `G_AddEvent(player, EV_PRIVATE_DUEL, parameter)`.
fn add_event(state: &mut PlayerState, parameter: u32) {
    crate::player_entity::add_event(state, EV_PRIVATE_DUEL, parameter);
}

/// The line `Cmd_EngageDuel_f` traces (`MASK_PLAYERSOLID`, from the origin): 256 units
/// along the view, to eye height.
pub fn challenge_line(state: &PlayerState) -> ([f32; 3], [f32; 3]) {
    let origin = state.origin();
    let forward = crate::pmove::flight::flight_axes(state.view_angles())
        .0
        .to_array();
    let view_height = state.raw_field(22).unwrap_or(0) as i32 as f32;
    let end = [
        origin[0] + forward[0] * 256.0,
        origin[1] + forward[1] * 256.0,
        (origin[2] + view_height) + forward[2] * 256.0,
    ];
    (origin, end)
}

/// Whether `Cmd_EngageDuel_f` goes on to its trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Challenge {
    /// It traces.
    Traces,
    /// It stops, silently.
    Refused,
    /// It stops, and tells the player this (`print`).
    Told(&'static [u8]),
}

/// Whether `Cmd_EngageDuel_f` goes as far as its trace for this player, of team `team`:
/// private duels on, a free-for-all, not holding off a challenge of its own, the saber in
/// hand and not thrown, no duel already. JA+'s rules ([`DuelRules::ja_plus`], VisionJK
/// `g_cmds.c:3262-3311`) are its own checks, in its order: no jetpack, no cloak, no
/// grapple, no spectator, private duels on, not a duel game type, team deathmatch only
/// with [`DuelRules::team_duels`], not holding off a challenge, no duel already and
/// `rest_until` passed; any weapon will do.
pub fn may_challenge(
    state: &PlayerState,
    team: i32,
    gametype: i32,
    private_duels: bool,
    level_time: i32,
    rest_until: i32,
    rules: &DuelRules,
) -> Challenge {
    let field = |index: usize| state.raw_field(index).unwrap_or(0);
    if !rules.ja_plus {
        let traces = private_duels
            && gametype != GT_DUEL
            && gametype != GT_POWERDUEL
            && gametype < GT_TEAM
            && state.duel_time() < level_time
            && field(PS_WEAPON) == WP_SABER
            && field(PS_SABER_IN_FLIGHT) == 0
            && !state.duel_in_progress();
        return if traces {
            Challenge::Traces
        } else {
            Challenge::Refused
        };
    }
    if state.stats[STAT_HOLDABLE_ITEMS] & (1 << HI_JETPACK) != 0 {
        return Challenge::Told(b"print \"You have to disable your jetpack in order to duel.\n\"");
    }
    if state.powerups[PW_CLOAKED] != 0 {
        return Challenge::Told(
            b"print \"You have to disable your Cloak item in order to duel.\n\"",
        );
    }
    if crate::prediction_policy::in_grapple_move(state.leg_animation())
        || crate::prediction_policy::in_grapple_move(state.torso_animation())
        || team == TEAM_SPECTATOR
        || !private_duels
    {
        return Challenge::Refused;
    }
    if gametype == GT_DUEL
        || gametype == GT_POWERDUEL
        || gametype > GT_TEAM
        || (gametype == GT_TEAM && !rules.team_duels)
    {
        return Challenge::Told(b"print \"@@@NODUEL_GAMETYPE\n\"");
    }
    if state.duel_time() >= level_time || state.duel_in_progress() {
        return Challenge::Refused;
    }
    if level_time < rest_until {
        return Challenge::Told(b"print \"@@@CANTDUEL_JUSTDID\n\"");
    }
    Challenge::Traces
}

/// The rest of `Cmd_EngageDuel_f` for `me`, whose trace struck `other` (a client
/// number): accepted if `other` challenged `me` and its challenge stands, else a
/// challenge — either way `me` gestures and names `other` for five seconds. Nothing for
/// one dead, without its saber in hand, duelling or with its saber thrown.
///
/// Under JA+'s rules (VisionJK `g_cmds.c:3325-3452`) the challenged may hold any weapon:
/// the acceptance needs both sabers in hand and not thrown, and no jetpack on the
/// challenger. `full_force_key` is the command `engage_FullForceDuel` (which sets
/// [`Duellist::full_force`] first): any other challenge is not a full-force one. The
/// acceptance gives both duellists 100 health, the full Force pool and
/// [`DuelRules::start_armor`], the acceptor the challenger's kind of duel, and with
/// [`DuelRules::bow`] both the bow `bow` names (its animation and `BG_AnimLength`). A
/// challenge lets the challenged out of its rest.
pub fn engage(
    me: &mut Duellist,
    other: &mut Duellist,
    gametype: i32,
    level_time: i32,
    rules: &DuelRules,
    full_force_key: bool,
    bow: Option<(u16, i32)>,
) -> Told {
    let mut told = Told::default();
    let field = |state: &PlayerState, index: usize| state.raw_field(index).unwrap_or(0);
    if other.number >= MAX_CLIENTS
        || *other.health < 1
        || (other.state.stats[STAT_HEALTH] as i32) < 1
        || (!rules.ja_plus && field(other.state, PS_WEAPON) != WP_SABER)
        || other.state.duel_in_progress()
        || (!rules.ja_plus && field(other.state, PS_SABER_IN_FLIGHT) != 0)
    {
        return told;
    }
    if gametype >= GT_TEAM && other.team == me.team {
        return told;
    }
    if rules.ja_plus && !full_force_key {
        *me.full_force = false;
    }
    let mut gesture = true;
    if other.state.duel_index() == me.number && other.state.duel_time() >= level_time {
        if rules.ja_plus
            && (field(me.state, PS_WEAPON) != WP_SABER
                || field(me.state, PS_SABER_IN_FLIGHT) != 0
                || field(other.state, PS_WEAPON) != WP_SABER
                || field(other.state, PS_SABER_IN_FLIGHT) != 0
                || other.state.stats[STAT_HOLDABLE_ITEMS] & (1 << HI_JETPACK) != 0)
        {
            return told;
        }
        let text = [
            b"print \"".as_slice(),
            other.name,
            b" @@@PLDUELACCEPT ",
            me.name,
            b"!\n\"",
        ]
        .concat();
        told.commands.push((None, text));
        for state in [&mut *me.state, &mut *other.state] {
            state.set_raw_field(PS_DUEL_IN_PROGRESS, 1);
            state.set_raw_field(PS_DUEL_TIME, (level_time + 2_000) as u32);
        }
        for (number, state) in [
            (me.number, &mut *me.state),
            (other.number, &mut *other.state),
        ] {
            add_event(state, 1);
            told.raised.push(number);
        }
        if rules.ja_plus {
            *me.full_force = *other.full_force;
            let refill = |duellist: &mut Duellist| {
                *duellist.health = JA_PLUS_START_HEALTH;
                duellist.state.stats[STAT_HEALTH] = JA_PLUS_START_HEALTH as u32;
                duellist.state.stats[STAT_ARMOR] = rules.start_armor as u32;
                duellist
                    .state
                    .set_raw_field(PS_FORCE_POWER, duellist.force_max.max(0) as u32);
            };
            refill(me);
            refill(other);
        }
        // Holstered until the duel begins, to light them then.
        for (number, state) in [
            (me.number, &mut *me.state),
            (other.number, &mut *other.state),
        ] {
            if field(state, PS_SABER_HOLSTERED) == 0 {
                // `saber[0].soundOff`; the second saber's only with a model.
                told.sounds.extend(
                    crate::generic_commands::SaberSound::both(false).map(|sound| (number, sound)),
                );
                state.set_raw_field(PS_WEAPON_TIME, 400);
                state.set_raw_field(PS_SABER_HOLSTERED, 2);
            }
        }
        if rules.ja_plus
            && rules.bow
            && let Some((animation, length)) = bow
        {
            let bow = |duellist: &mut Duellist| {
                duellist
                    .state
                    .set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_TAUNT);
                duellist
                    .state
                    .set_raw_field(PS_FORCE_DODGE_ANIM, u32::from(animation));
                *duellist.hand_extend_time = level_time + length;
            };
            bow(me);
            bow(other);
            gesture = false;
        }
    } else {
        let full = rules.ja_plus && *me.full_force;
        told.commands.push((
            Some(other.number),
            if full {
                [
                    b"cp \"".as_slice(),
                    me.name,
                    b" \n^7 is challenging you in ^5Full Force\n\"",
                ]
                .concat()
            } else {
                [b"cp \"".as_slice(), me.name, b" @@@PLDUELCHALLENGE\n\""].concat()
            },
        ));
        told.commands.push((
            Some(me.number),
            if full {
                [
                    b"cp \"^7Asking ^5Full Force ^7Duel to\n ".as_slice(),
                    other.name,
                    b"\n\"",
                ]
                .concat()
            } else {
                [b"cp \"@@@PLDUELCHALLENGED ".as_slice(), other.name, b"\n\""].concat()
            },
        ));
    }
    if gesture {
        me.state
            .set_raw_field(PS_FORCE_HAND_EXTEND, HANDEXTEND_DUELCHALLENGE);
        *me.hand_extend_time = level_time + 1_000;
    }
    if rules.ja_plus {
        // A player just out of a duel may still accept (`g_cmds.c:3446`).
        *other.rest_until = 0;
    }
    me.state
        .set_raw_field(PS_DUEL_INDEX, u32::from(other.number));
    me.state
        .set_raw_field(PS_DUEL_TIME, (level_time + 5_000) as u32);
    told
}

/// Both duellists out of their duel, and told so (`EV_PRIVATE_DUEL` 0).
fn end_both(me: &mut Duellist, other: &mut Duellist, told: &mut Told) {
    for state in [&mut *me.state, &mut *other.state] {
        state.set_raw_field(PS_DUEL_IN_PROGRESS, 0);
    }
    *me.full_force = false;
    *other.full_force = false;
    for (number, state) in [
        (me.number, &mut *me.state),
        (other.number, &mut *other.state),
    ] {
        add_event(state, 0);
        told.raised.push(number);
    }
}

/// What the duel's think leaves of the command: held still while the duel has not begun.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Thought {
    /// `speed`, `basespeed` and the command's moves zeroed.
    pub frozen: bool,
}

/// `ClientThink_real`'s duel for `me`, duelling `against` (`None` for one gone): the
/// sabers lit once the wait is over; the duel ended when the other is gone or no longer
/// duels `me`, when it is dead (the winner healed and protected), or when they are 1024
/// units apart.
///
/// Under JA+'s rules (VisionJK `g_active.c:2439-2572`) a duellist rests two seconds
/// after the duel ([`Duellist::rest_until`]); the sabers are not lit at the start, the
/// duellists light them themselves; the winner's health and armour are reported as
/// [`DuelRules::winner_report`] says before it is healed, and it keeps 25 armour; the
/// distance ends a duel only with [`DuelRules::distance_limit`].
pub fn think(
    me: &mut Duellist,
    against: Option<&mut Duellist>,
    level_time: i32,
    rules: &DuelRules,
    told: &mut Told,
) -> Thought {
    let mut thought = Thought::default();
    if !me.state.duel_in_progress() {
        return thought;
    }
    if rules.ja_plus {
        *me.rest_until = level_time + JA_PLUS_REST;
    }
    let field = |state: &PlayerState, index: usize| state.raw_field(index).unwrap_or(0);
    let mut against = against;
    if me.state.duel_time() < level_time {
        // Bring out the sabers: this player's, then the opponent's.
        let bring_out = |number: u16, state: &mut PlayerState, told: &mut Told| {
            if field(state, PS_WEAPON) == WP_SABER
                && field(state, PS_SABER_HOLSTERED) != 0
                && state.duel_time() != 0
            {
                if !rules.ja_plus {
                    state.set_raw_field(PS_SABER_HOLSTERED, 0);
                    // Both sabers' on-sounds, the empty second's default one too.
                    told.sounds.extend(
                        crate::generic_commands::SaberSound::both(true)
                            .map(|sound| (number, sound)),
                    );
                }
                add_event(state, 2);
                told.raised.push(number);
                state.set_raw_field(PS_DUEL_TIME, 0);
            }
        };
        bring_out(me.number, me.state, told);
        if let Some(other) = against.as_deref_mut() {
            bring_out(other.number, other.state, told);
        }
    } else {
        me.state.set_raw_field(PS_SPEED, 0f32.to_bits());
        thought.frozen = true;
    }
    let Some(other) = against.filter(|other| other.state.duel_index() == me.number) else {
        me.state.set_raw_field(PS_DUEL_IN_PROGRESS, 0);
        *me.full_force = false;
        add_event(me.state, 0);
        told.raised.push(me.number);
        return thought;
    };
    if *other.health < 1 || (other.state.stats[STAT_HEALTH] as i32) < 1 {
        end_both(me, other, told);
        let alive = *me.health > 0 && me.state.stats[STAT_HEALTH] as i32 > 0;
        if alive {
            report_winner(me, other, rules.winner_report, told);
            let max = me.state.stats[STAT_MAX_HEALTH] as i32;
            if *me.health < max {
                *me.health = max;
                me.state.stats[STAT_HEALTH] = max as u32;
            }
            let flags = field(me.state, PS_EFLAGS);
            me.state.set_raw_field(PS_EFLAGS, flags | EF_INVULNERABLE);
            *me.invulnerable_until = level_time + SPAWN_INVULNERABILITY;
            if rules.ja_plus {
                me.state.stats[STAT_ARMOR] = JA_PLUS_WINNER_ARMOR;
            }
        } else {
            // Both died in the same frame.
            told.commands
                .push((None, b"cp \"@@@PLDUELTIE\n\"".to_vec()));
        }
        return thought;
    }
    let (mine, theirs) = (me.state.origin(), other.state.origin());
    let apart = (0..3)
        .map(|axis| (mine[axis] - theirs[axis]) * (mine[axis] - theirs[axis]))
        .sum::<f32>()
        .sqrt();
    if apart >= 1_024.0 && rules.distance_limit {
        end_both(me, other, told);
        told.commands
            .push((None, b"print \"@@@PLDUELSTOP\n\"".to_vec()));
    }
    thought
}

/// The winner `me` of a duel against `other` announced, with the health and armour it
/// has left before it is healed (VisionJK `g_active.c:2504-2524`).
fn report_winner(me: &Duellist, other: &Duellist, report: WinnerReport, told: &mut Told) {
    let health = (me.state.stats[STAT_HEALTH] as i32).to_string();
    let armor = (me.state.stats[STAT_ARMOR] as i32).to_string();
    let centred = || {
        [
            b"cp \"".as_slice(),
            me.name,
            b" @@@PLDUELWINNER ",
            other.name,
            b"!\n^5",
            health.as_bytes(),
            b" ^7Health ^5",
            armor.as_bytes(),
            b" ^7Armor\"",
        ]
        .concat()
    };
    let line = || {
        [
            b"print \"".as_slice(),
            me.name,
            b" @@@PLDUELWINNER ",
            other.name,
            b"! ==>  ^5",
            health.as_bytes(),
            b" ^7Health ^5",
            armor.as_bytes(),
            b" ^7Armor left for winner\n\"",
        ]
        .concat()
    };
    match report {
        WinnerReport::Base => told.commands.push((
            None,
            [
                b"cp \"".as_slice(),
                me.name,
                b" @@@PLDUELWINNER ",
                other.name,
                b"!\n\"",
            ]
            .concat(),
        )),
        WinnerReport::Silent => {}
        WinnerReport::Everyone => {
            told.commands.push((Some(me.number), centred()));
            told.commands.push((Some(other.number), centred()));
            told.commands.push((None, line()));
        }
        WinnerReport::Duellists => {
            told.commands.push((Some(me.number), centred()));
            told.commands.push((Some(other.number), centred()));
            told.commands.push((Some(me.number), line()));
            told.commands.push((Some(other.number), line()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A player's own values, for a [`Duellist`] to borrow.
    struct Held {
        state: PlayerState,
        health: i32,
        hand: i32,
        invulnerable: i32,
        full_force: bool,
        rest: i32,
    }

    impl Held {
        fn new(number: u16, origin: [f32; 3]) -> Self {
            let mut state = PlayerState::default();
            state.set_client_num(number);
            state.set_origin(origin);
            state.set_raw_field(PS_WEAPON, WP_SABER);
            state.stats[STAT_HEALTH] = 60;
            state.stats[STAT_MAX_HEALTH] = 100;
            state.stats[STAT_ARMOR] = 10;
            Self {
                state,
                health: 60,
                hand: 0,
                invulnerable: 0,
                full_force: false,
                rest: 0,
            }
        }

        fn duellist(&mut self, number: u16, name: &'static [u8]) -> Duellist<'_> {
            Duellist {
                number,
                state: &mut self.state,
                health: &mut self.health,
                name,
                hand_extend_time: &mut self.hand,
                invulnerable_until: &mut self.invulnerable,
                team: 0,
                force_max: 100,
                full_force: &mut self.full_force,
                rest_until: &mut self.rest,
            }
        }
    }

    const JA_PLUS: DuelRules = DuelRules {
        ja_plus: true,
        isolation: true,
        team_duels: true,
        distance_limit: false,
        bow: true,
        start_armor: 100,
        winner_report: WinnerReport::Everyone,
        force_regen_time: Some(200),
    };

    /// 0 and 1 duelling, accepted at 1000 under `rules`.
    fn duelling(rules: &DuelRules) -> (Held, Held) {
        let (mut a, mut b) = (Held::new(0, [0.0; 3]), Held::new(1, [100.0, 0.0, 0.0]));
        engage(
            &mut a.duellist(0, b"A"),
            &mut b.duellist(1, b"B"),
            0,
            900,
            rules,
            false,
            Some((7, 1_500)),
        );
        engage(
            &mut b.duellist(1, b"B"),
            &mut a.duellist(0, b"A"),
            0,
            1_000,
            rules,
            false,
            Some((7, 1_500)),
        );
        (a, b)
    }

    #[test]
    fn the_isolation_leaves_out_who_does_not_duel_together() {
        // Not duelling: every duellist, nobody else.
        assert!(isolated(None, 3, true, false));
        assert!(!isolated(None, 3, false, false));
        // Duelling 1: everyone but 1 duelling it back.
        assert!(!isolated(Some(1), 1, true, true));
        assert!(isolated(Some(1), 1, false, false));
        assert!(isolated(Some(1), 2, false, false));
        assert!(isolated(Some(1), 2, true, false));
        assert!(
            DuelRules {
                isolation: true,
                ..DuelRules::default()
            }
            .isolates(0)
        );
        assert!(
            DuelRules {
                isolation: true,
                ..DuelRules::default()
            }
            .isolates(6)
        );
        assert!(
            !DuelRules {
                isolation: true,
                ..DuelRules::default()
            }
            .isolates(1)
        );
        assert!(!DuelRules::default().isolates(0));
    }

    #[test]
    fn the_force_and_damage_rules_are_the_base_games_without_a_full_force_duel() {
        let (a, b) = duelling(&DuelRules::default());
        let outsider = Held::new(2, [0.0; 3]);
        let none = FullForce::default();
        let duel = |held: &Held| duel_of(&held.state);
        assert!(force_refused(0, duel(&a), 1, duel(&b), none));
        assert!(force_refused(2, None, 0, duel(&a), none));
        assert!(!force_refused(2, None, 3, None, none));
        assert!(damage_refused(
            1,
            &b.state,
            Some((0, &a.state)),
            MOD_SABER + 1,
            none
        ));
        assert!(!damage_refused(
            1,
            &b.state,
            Some((0, &a.state)),
            MOD_SABER,
            none
        ));
        assert!(damage_refused(
            0,
            &a.state,
            Some((2, &outsider.state)),
            MOD_SABER,
            none
        ));
        // A full-force duel lets the Force through between its two only.
        let full = FullForce(0b11);
        assert!(!force_refused(0, duel(&a), 1, duel(&b), full));
        assert!(force_refused(2, None, 0, duel(&a), full));
        assert!(!damage_refused(
            1,
            &b.state,
            Some((0, &a.state)),
            MOD_SABER + 1,
            full
        ));
        assert!(damage_refused(
            0,
            &a.state,
            Some((2, &outsider.state)),
            MOD_SABER + 1,
            full
        ));
    }

    #[test]
    fn the_base_game_challenges_as_before() {
        let held = Held::new(0, [0.0; 3]);
        let rules = DuelRules::default();
        let challenge = |state: &PlayerState, gametype| {
            may_challenge(state, 0, gametype, true, 1_000, 0, &rules)
        };
        assert_eq!(challenge(&held.state, 0), Challenge::Traces);
        // Silent in a team game and without the saber.
        assert_eq!(challenge(&held.state, 6), Challenge::Refused);
        let mut pistol = held.state.clone();
        pistol.set_raw_field(PS_WEAPON, 4);
        assert_eq!(challenge(&pistol, 0), Challenge::Refused);
        let (a, b) = duelling(&rules);
        // No refill, the challenge gesture, no full force.
        assert_eq!((a.health, a.state.stats[STAT_ARMOR]), (60, 10));
        assert_eq!(
            b.state.raw_field(PS_FORCE_HAND_EXTEND),
            Some(HANDEXTEND_DUELCHALLENGE)
        );
        assert!(!a.full_force && !b.full_force && a.rest == 0);
    }

    #[test]
    fn ja_plus_challenges_by_its_own_rules() {
        let held = Held::new(0, [0.0; 3]);
        let challenge = |state: &PlayerState, gametype, rest| {
            may_challenge(state, 0, gametype, true, 1_000, rest, &JA_PLUS)
        };
        let mut pistol = held.state.clone();
        pistol.set_raw_field(PS_WEAPON, 4);
        assert_eq!(challenge(&pistol, 0, 0), Challenge::Traces);
        assert_eq!(challenge(&held.state, 6, 0), Challenge::Traces);
        let no_team = DuelRules {
            team_duels: false,
            ..JA_PLUS
        };
        assert_eq!(
            may_challenge(&held.state, 0, 6, true, 1_000, 0, &no_team),
            Challenge::Told(b"print \"@@@NODUEL_GAMETYPE\n\"")
        );
        assert_eq!(
            challenge(&held.state, 7, 0),
            Challenge::Told(b"print \"@@@NODUEL_GAMETYPE\n\"")
        );
        assert_eq!(
            challenge(&held.state, 0, 1_500),
            Challenge::Told(b"print \"@@@CANTDUEL_JUSTDID\n\"")
        );
        assert_eq!(
            may_challenge(&held.state, TEAM_SPECTATOR, 0, true, 1_000, 0, &JA_PLUS),
            Challenge::Refused
        );
        let mut cloaked = held.state.clone();
        cloaked.powerups[PW_CLOAKED] = 5_000;
        assert!(matches!(challenge(&cloaked, 0, 0), Challenge::Told(_)));
    }

    #[test]
    fn a_ja_plus_duel_starts_refilled_bowing_and_unlit() {
        let (mut a, mut b) = duelling(&JA_PLUS);
        for held in [&a, &b] {
            assert!(held.state.duel_in_progress());
            assert_eq!((held.health, held.state.stats[STAT_HEALTH]), (100, 100));
            assert_eq!(held.state.stats[STAT_ARMOR], 100);
            assert_eq!(held.state.raw_field(PS_FORCE_POWER), Some(100));
            assert_eq!(
                held.state.raw_field(PS_FORCE_HAND_EXTEND),
                Some(HANDEXTEND_TAUNT)
            );
            assert_eq!(held.state.raw_field(PS_FORCE_DODGE_ANIM), Some(7));
            assert_eq!(held.hand, 2_500);
        }
        // Begun: event 2, but the sabers stay off.
        let mut told = Told::default();
        think(
            &mut a.duellist(0, b"A"),
            Some(&mut b.duellist(1, b"B")),
            3_100,
            &JA_PLUS,
            &mut told,
        );
        assert_eq!(a.state.raw_field(PS_SABER_HOLSTERED), Some(2));
        assert!(told.sounds.is_empty() && told.raised == [0, 1]);
        assert_eq!(a.rest, 3_100 + JA_PLUS_REST);
        // The base game lights them.
        let (mut a, mut b) = duelling(&DuelRules::default());
        let mut told = Told::default();
        let base = DuelRules::default();
        think(
            &mut a.duellist(0, b"A"),
            Some(&mut b.duellist(1, b"B")),
            3_100,
            &base,
            &mut told,
        );
        assert_eq!(a.state.raw_field(PS_SABER_HOLSTERED), Some(0));
        assert!(!told.sounds.is_empty());
    }

    #[test]
    fn the_distance_ends_a_duel_only_with_its_limit() {
        for (rules, ends) in [(DuelRules::default(), true), (JA_PLUS, false)] {
            let (mut a, mut b) = duelling(&rules);
            b.state.set_origin([2_000.0, 0.0, 0.0]);
            let mut told = Told::default();
            think(
                &mut a.duellist(0, b"A"),
                Some(&mut b.duellist(1, b"B")),
                3_100,
                &rules,
                &mut told,
            );
            assert_eq!(!a.state.duel_in_progress(), ends);
        }
    }

    #[test]
    fn the_winner_is_reported_and_healed() {
        let win = |rules: &DuelRules| {
            let (mut a, mut b) = duelling(rules);
            a.health = 40;
            a.state.stats[STAT_HEALTH] = 40;
            a.state.stats[STAT_ARMOR] = 7;
            b.health = 0;
            b.state.stats[STAT_HEALTH] = 0;
            let mut told = Told::default();
            think(
                &mut a.duellist(0, b"A"),
                Some(&mut b.duellist(1, b"B")),
                3_100,
                rules,
                &mut told,
            );
            assert!(!a.state.duel_in_progress() && !b.state.duel_in_progress());
            assert_eq!(a.health, 100);
            (a, told.commands)
        };
        let (a, base) = win(&DuelRules::default());
        assert_eq!(base, [(None, b"cp \"A @@@PLDUELWINNER B!\n\"".to_vec())]);
        assert_eq!(a.state.stats[STAT_ARMOR], 7);
        let (a, everyone) = win(&JA_PLUS);
        let centred = b"cp \"A @@@PLDUELWINNER B!\n^540 ^7Health ^57 ^7Armor\"".to_vec();
        let line =
            b"print \"A @@@PLDUELWINNER B! ==>  ^540 ^7Health ^57 ^7Armor left for winner\n\""
                .to_vec();
        assert_eq!(
            everyone,
            [
                (Some(0), centred.clone()),
                (Some(1), centred.clone()),
                (None, line.clone())
            ]
        );
        assert_eq!(a.state.stats[STAT_ARMOR], JA_PLUS_WINNER_ARMOR);
        let (_, two) = win(&DuelRules {
            winner_report: WinnerReport::Duellists,
            ..JA_PLUS
        });
        assert_eq!(
            two,
            [
                (Some(0), centred.clone()),
                (Some(1), centred),
                (Some(0), line.clone()),
                (Some(1), line)
            ]
        );
        let (_, silent) = win(&DuelRules {
            winner_report: WinnerReport::Silent,
            ..JA_PLUS
        });
        assert!(silent.is_empty());
    }

    #[test]
    fn a_full_force_challenge_is_said_and_carried_to_the_duel() {
        let (mut a, mut b) = (Held::new(0, [0.0; 3]), Held::new(1, [100.0, 0.0, 0.0]));
        a.full_force = true;
        let told = engage(
            &mut a.duellist(0, b"A"),
            &mut b.duellist(1, b"B"),
            0,
            900,
            &JA_PLUS,
            true,
            None,
        );
        assert_eq!(
            told.commands[0],
            (
                Some(1),
                b"cp \"A \n^7 is challenging you in ^5Full Force\n\"".to_vec()
            )
        );
        engage(
            &mut b.duellist(1, b"B"),
            &mut a.duellist(0, b"A"),
            0,
            1_000,
            &JA_PLUS,
            false,
            None,
        );
        assert!(a.full_force && b.full_force);
        // Its end clears it.
        a.health = 0;
        a.state.stats[STAT_HEALTH] = 0;
        let mut told = Told::default();
        think(
            &mut b.duellist(1, b"B"),
            Some(&mut a.duellist(0, b"A")),
            3_100,
            &JA_PLUS,
            &mut told,
        );
        assert!(!a.full_force && !b.full_force);
    }
}
