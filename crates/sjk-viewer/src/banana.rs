//! The Banana peel, SJK's meme toy (`toy_banana`; Kushina's idea, approved by Sol on
//! 29/10/2026): a player with the toy on drops banana peels on the floor
//! (`banana_drop`), and a player who has it on too and walks onto one slips: a cartoon
//! slip sound, and their body goes limp as a ragdoll, its feet swept along the way they
//! walked and up.
//!
//! - **Opt-in:** the switch (`toy_banana`, the Collection's Toys tab, the quick wheel)
//!   is both the right to drop peels and the consent to slip on them; it stays as last
//!   switched ([`crate::toy_saved::BANANA_CVAR`]). A player without it walks over peels.
//!   Nobody drops or slips while dead, spectating, in a duel, with the flag, limp,
//!   hacked, held by the server or in a vehicle (the Ragdoll's rules,
//!   [`crate::ragdoll::Ragdoll::strikable`]).
//! - **The drop:** at the player's feet, a little ahead ([`DROP_REACH`]), or tossed
//!   [`TOSS_REACH`] ahead the way they move when they move; it flies from the hand for
//!   [`FLIGHT_MS`], tumbling, and lands on the floor below, lying flat on it. Not within
//!   [`SPAWN_CLEARANCE`] of a spawn point ([`SPAWN_CLASSES`]), nor where no floor is near.
//! - **The slip** is decided by the slipping player's client alone, as the Monitor's
//!   hit is: on the ground, moving at least [`MIN_SLIP_SPEED`], their feet within
//!   [`SLIP_RADIUS`] of a peel they came at (one that landed under them waits until
//!   they have stepped off it, [`ARM_RADIUS`]; their own waits [`OWN_SAFE_MS`] more).
//!   The body falls and takes a blow at a foot ([`slip_push`]); the peel is used up for
//!   everyone. They get up as from any fall the Ragdoll did not start by itself: a jump,
//!   an attack or a taunt, or by themselves once the body has lain still.
//! - **Limits:** [`PER_PLAYER`] peels down a player (a new one takes the oldest's place),
//!   each for [`LIFETIME_MS`], one every [`COOLDOWN`]; all go with a new map, another
//!   server, or when their dropper turns the toy off or leaves.
//! - **Unseen:** in a duel (the player's own or the one they follow, and on Duel and
//!   Power Duel servers) no peel is drawn or heard; [`SHOW_CVAR`] at 0 hides every peel
//!   and its sounds in the world, everywhere. Neither changes who may slip: the switch
//!   does, as for the Monitor.
//!
//! No game server knows of it. While it is on, the player's hub look lists it
//! ([`TOY`] in [`crate::looks::TOYS`]), as the other toys' do; the peels go otherwise:
//! every SJK client on the server learns of a peel from its dropper's client through the
//! toy relay
//! (`PROTOCOL.md`, "Toy relay"), which tells each SJK player in turn, the nearest first,
//! and the ones who come later: [`Event::Drop`], `drop_<number>_<moment>_<x>_<y>_<z>_
//! <dx>_<dy>_<dz>_<yaw>`. The slipping player's client tells every one that the peel is
//! gone (`slip_<dropper>_<number>`), and the dropper's that all theirs are, when the toy
//! goes off (`clear`). The fall itself travels as the Ragdoll's, its blow naming the
//! dropper and no monitor ([`crate::ragdoll::UNTHROWN`]). Players on stock clients see
//! nothing.

use glam::{Quat, Vec3};
use std::time::{Duration, Instant};

/// The toy's console command: the switch (drop peels, and slip on them).
pub(crate) const COMMAND: &str = "toy_banana";
/// Drop a peel.
pub(crate) const DROP_COMMAND: &str = "banana_drop";
/// `cg_toyBananaShow` (archived, on by default): off, no peel is drawn, and none of
/// their sounds in the world is heard, on this PC.
pub(crate) const SHOW_CVAR: &str = "cg_toyBananaShow";
/// The toy's id in relayed toy events.
pub(crate) const TOY: &str = "banana";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_banana.png";
/// The peel lying on the floor: its origin where it rests, +Z up (made by
/// `scripts/banana_assets.py`).
pub(crate) const MODEL: &str = "models/sjk/banana.md3";
/// Heard as a player slips (theirs in both ears, at the peel for the others), and as a
/// peel lands.
pub(crate) const SLIP_SOUND: &str = "sound/sjk/banana/slip.wav";
pub(crate) const DROP_SOUND: &str = "sound/sjk/banana/drop.wav";

/// The most peels a player has down: a new one takes the oldest's place.
pub(crate) const PER_PLAYER: usize = 3;
/// How long a peel lies (game server milliseconds), shrinking away over its last
/// [`FADE_MS`].
pub(crate) const LIFETIME_MS: i64 = 120_000;
const FADE_MS: i64 = 500;
/// The shortest time between two drops of a player.
pub(crate) const COOLDOWN: Duration = Duration::from_millis(2_000);
/// No peel lands closer than this to a spawn point (units, from its origin).
pub(crate) const SPAWN_CLEARANCE: f32 = 96.0;
/// The map entities players spawn at (`codemp/game/g_spawn.c`'s spawn table, the
/// `info_player_*` and CTF team spawns): no peel is dropped by one. The game's own
/// classes, so a map needs nothing of its own.
pub(crate) const SPAWN_CLASSES: [&str; 11] = [
    "info_player_deathmatch",
    "info_player_start",
    "info_player_duel",
    "info_player_duel1",
    "info_player_duel2",
    "info_player_siegeteam1",
    "info_player_siegeteam2",
    "team_CTF_redplayer",
    "team_CTF_blueplayer",
    "team_CTF_redspawn",
    "team_CTF_bluespawn",
];
/// The most peels shown at once, every player's (the oldest goes).
const MAX_SHOWN: usize = 64;

/// A player moving faster than this (units a second, along the floor) tosses the peel
/// [`TOSS_REACH`] ahead the way they move; slower, it drops [`DROP_REACH`] ahead of the
/// feet the way they look.
pub(crate) const TOSS_SPEED: f32 = 80.0;
pub(crate) const TOSS_REACH: f32 = 72.0;
pub(crate) const DROP_REACH: f32 = 20.0;
/// It leaves the hand this far above the player's origin (32 over the floor).
const HAND_HEIGHT: f32 = 8.0;
/// The floor is looked for this far under the spot; a floor steeper than this (its
/// normal's height) takes no peel.
const FLOOR_SEARCH: f32 = 256.0;
const FLAT_ENOUGH: f32 = 0.7;
/// The flight from the hand to the floor (game server milliseconds), and how high it
/// arcs over the straight way.
pub(crate) const FLIGHT_MS: i64 = 400;
const ARC: f32 = 12.0;
/// The hand's offset from where the peel lands, at most this far on each axis.
const MAX_OFFSET: i32 = 128;

/// A player slips on a peel their feet come within this of (along the floor), standing
/// no more than [`STEP_ABOVE`] over it nor [`STEP_BELOW`] under it.
pub(crate) const SLIP_RADIUS: f32 = 14.0;
const STEP_ABOVE: f32 = 12.0;
const STEP_BELOW: f32 = 8.0;
/// A peel slips a player only once they have been farther than this from it since it
/// landed: one landing under someone waits until they step off it.
pub(crate) const ARM_RADIUS: f32 = 26.0;
/// Slower than this along the floor (units a second), a player does not slip.
pub(crate) const MIN_SLIP_SPEED: f32 = 40.0;
/// The dropper's own peel cannot trip them for this long after it lands.
pub(crate) const OWN_SAFE_MS: i64 = 1_500;
/// The blow at a foot as the player slips (what the body's centre gains, units a
/// second): along the way they walk, and up; off the centre, it sweeps the legs from
/// under the body and lays it on its back.
const SLIP_PUSH: f32 = 40.0;
const SLIP_LIFT: f32 = 110.0;
/// The player's origin stands this high over their feet (`playerMins`).
pub(crate) const FEET: f32 = 24.0;

/// Between two events sent (the relay takes 40 a second from a player, which the other
/// toys share).
pub(crate) const SEND_GAP: Duration = Duration::from_millis(50);
/// A drop heard this long before its moment is dropped (ms); one older than a peel lives,
/// too.
const EARLY_MS: i64 = 3_000;
/// A peel used up is remembered this long, so its drop heard late does not put it back.
const GONE_KEPT: Duration = Duration::from_secs(15);
/// A peel's landing is heard when it lands no longer ago than this (ms).
const LANDING_LATE_MS: i64 = 1_000;
/// The numbers of a player's peels go round this many.
const NUMBERS: u8 = 100;
const SLOTS: usize = crate::looks::SLOTS;

/// The event prefixes.
const DROP_PREFIX: &str = "drop_";
const SLIP_PREFIX: &str = "slip_";
const CLEAR: &str = "clear";

/// A bundled file's bytes, by its name in `assets/banana`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/banana/", $name))
    };
}

/// The bundled files at their game paths (made by `scripts/banana_assets.py`).
const FILES: [(&str, &[u8]); 6] = [
    (MODEL, bundled!("banana.md3")),
    ("models/sjk/banana.jpg", bundled!("banana.jpg")),
    ("shaders/sjk_banana.shader", bundled!("banana.shader")),
    (ICON, bundled!("toy_banana.png")),
    (SLIP_SOUND, bundled!("slip.wav")),
    (DROP_SOUND, bundled!("drop.wav")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Banana peel", FILES.iter().copied())?;
    Ok(())
}

/// A peel's drop, as relayed: its number among its dropper's, the game server time it
/// left the hand, where it lands, the hand's offset from there (whole units) and how it
/// lies turned (degrees).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Drop {
    pub(crate) number: u8,
    pub(crate) moment: i32,
    pub(crate) at: [i32; 3],
    pub(crate) from: [i32; 3],
    pub(crate) yaw: u16,
}

impl Drop {
    /// The game server time it lands.
    fn lands(&self) -> i64 {
        i64::from(self.moment) + FLIGHT_MS
    }
}

/// A relayed event of the toy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    /// The sender dropped a peel.
    Drop(Drop),
    /// The sender slipped on peel `number` of the player in slot `dropper`: it is gone.
    Slip { dropper: u8, number: u8 },
    /// The sender turned the toy off: all their peels are gone.
    Clear,
}

impl Event {
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Drop(drop) => {
                let [x, y, z] = drop.at;
                let [dx, dy, dz] = drop.from;
                format!(
                    "{DROP_PREFIX}{}_{}_{x}_{y}_{z}_{dx}_{dy}_{dz}_{}",
                    drop.number, drop.moment, drop.yaw
                )
            }
            Self::Slip { dropper, number } => format!("{SLIP_PREFIX}{dropper}_{number}"),
            Self::Clear => CLEAR.to_owned(),
        }
    }

    /// The event [`Event::text`] wrote; `None` for anything else or out of bounds.
    pub(crate) fn parse(event: &str) -> Option<Self> {
        if event == CLEAR {
            return Some(Self::Clear);
        }
        if let Some(slip) = event.strip_prefix(SLIP_PREFIX) {
            let (dropper, number) = slip.split_once('_')?;
            let dropper = dropper
                .parse::<u8>()
                .ok()
                .filter(|slot| usize::from(*slot) < SLOTS)?;
            let number = number.parse::<u8>().ok().filter(|n| *n < NUMBERS)?;
            return Some(Self::Slip { dropper, number });
        }
        let mut parts = event.strip_prefix(DROP_PREFIX)?.split('_');
        let mut number = || parts.next()?.parse::<i32>().ok();
        let peel = u8::try_from(number()?).ok().filter(|n| *n < NUMBERS)?;
        let moment = number()?;
        let at = [number()?, number()?, number()?];
        let from = [number()?, number()?, number()?];
        let yaw = u16::try_from(number()?).ok().filter(|yaw| *yaw < 360)?;
        if parts.next().is_some()
            || at.iter().any(|c| c.abs() > 65_536)
            || from.iter().any(|c| c.abs() > MAX_OFFSET)
        {
            return None;
        }
        Some(Self::Drop(Drop {
            number: peel,
            moment,
            at,
            from,
            yaw,
        }))
    }
}

/// A peel out: whose (their slot), its drop, where it rests and the floor's normal there
/// (found by each client under the relayed spot), and this client's own marks.
pub(crate) struct Peel {
    pub(crate) dropper: u8,
    pub(crate) drop: Drop,
    pub(crate) rest: Vec3,
    pub(crate) normal: Vec3,
    /// The SJK players told of it (the player's own peels), bits by slot.
    pub(crate) told: u64,
    /// The player has been farther than [`ARM_RADIUS`] from it since it landed: walking
    /// onto it now slips them.
    pub(crate) armed: bool,
    /// Its dropper was seen among the SJK players: their leaving takes it away.
    pub(crate) seen: bool,
    /// Its landing was heard (or is too old to be).
    pub(crate) landed: bool,
}

impl Peel {
    pub(crate) fn new(dropper: u8, drop: Drop, rest: Vec3, normal: Vec3) -> Self {
        Self {
            dropper,
            drop,
            rest,
            normal,
            told: 0,
            armed: false,
            seen: false,
            landed: false,
        }
    }

    /// How much of it shows at `time` (it shrinks away over its last [`FADE_MS`]); 0 or
    /// less: gone.
    pub(crate) fn level(&self, time: i64) -> f32 {
        let left = i64::from(self.drop.moment) + LIFETIME_MS - time;
        (left as f32 / FADE_MS as f32).min(1.0)
    }

    /// Where it is and how it is turned at `time`: on its way from the hand, tumbling,
    /// then lying on the floor.
    pub(crate) fn pose(&self, time: i64) -> (Vec3, Quat) {
        let s = ((time - i64::from(self.drop.moment)) as f32 / FLIGHT_MS as f32).clamp(0.0, 1.0);
        let yaw = f32::from(self.drop.yaw).to_radians();
        if s >= 1.0 {
            let flat = Quat::from_rotation_arc(Vec3::Z, self.normal.normalize_or(Vec3::Z));
            return (self.rest, flat * Quat::from_rotation_z(yaw));
        }
        let hand = self.rest + Vec3::from_array(self.drop.from.map(|c| c as f32));
        let at = hand.lerp(self.rest, s) + Vec3::Z * (ARC * 4.0 * s * (1.0 - s));
        let left = 1.0 - s;
        let turn = Quat::from_rotation_z(yaw + std::f32::consts::PI * left)
            * Quat::from_rotation_y(std::f32::consts::TAU * left);
        (at, turn)
    }
}

/// Every peel out, and the rules of how many.
#[derive(Default)]
pub(crate) struct Peels {
    pub(crate) list: Vec<Peel>,
}

impl Peels {
    /// Peel `number` of the player in `dropper`, if out.
    pub(crate) fn find(&self, dropper: u8, number: u8) -> Option<usize> {
        self.list
            .iter()
            .position(|peel| peel.dropper == dropper && peel.drop.number == number)
    }

    /// Put out a peel, in place of one of the same dropper and number; the dropper's
    /// oldest goes past [`PER_PLAYER`], the oldest of all past [`MAX_SHOWN`].
    pub(crate) fn put(&mut self, peel: Peel) {
        if let Some(same) = self.find(peel.dropper, peel.drop.number) {
            self.list.remove(same);
        }
        let theirs = self
            .list
            .iter()
            .filter(|other| other.dropper == peel.dropper)
            .count();
        if theirs >= PER_PLAYER
            && let Some(oldest) = self
                .list
                .iter()
                .position(|other| other.dropper == peel.dropper)
        {
            self.list.remove(oldest);
        }
        if self.list.len() >= MAX_SHOWN {
            self.list.remove(0);
        }
        self.list.push(peel);
    }

    /// Take every peel of the player in `dropper` away.
    pub(crate) fn clear_of(&mut self, dropper: u8) {
        self.list.retain(|peel| peel.dropper != dropper);
    }

    /// Let the peels go that lay their time out at `time`, and those of droppers once
    /// seen among the SJK players (`sjk`, bits by slot) who are no longer.
    pub(crate) fn prune(&mut self, time: i64, sjk: u64) {
        self.list.retain_mut(|peel| {
            let here = sjk & (1 << peel.dropper) != 0;
            peel.seen |= here;
            peel.level(time) > 0.0 && (here || !peel.seen)
        });
    }
}

/// The player as a peel may trip them: where their feet are, how fast they go, and
/// whether they stand on the ground.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Walker {
    pub(crate) feet: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) on_ground: bool,
}

/// Whether the player may drop a peel or slip on one now: the toy on (`on`), not limp,
/// and the Ragdoll's rules let them fall ([`crate::ragdoll::Ragdoll::strikable`]: on a
/// game server, alive and playing, not following anyone, in no duel, without the flag,
/// not hacked, not held by the server, not in a vehicle).
pub(crate) fn may_play(on: bool, ragdoll: &crate::ragdoll::Ragdoll) -> bool {
    on && !ragdoll.on() && ragdoll.strikable()
}

/// Whether feet at `feet` are on a peel resting at `rest`.
fn underfoot(feet: Vec3, rest: Vec3) -> bool {
    (feet - rest).truncate().length() <= SLIP_RADIUS
        && (-STEP_BELOW..=STEP_ABOVE).contains(&(feet.z - rest.z))
}

/// Arm the landed peels the player (feet at `feet`) is off at `time`: from then on,
/// walking onto them slips them.
pub(crate) fn arm(peels: &mut Peels, feet: Vec3, time: i64) {
    for peel in &mut peels.list {
        if !peel.armed && time >= peel.drop.lands() {
            let off = (feet - peel.rest).truncate().length() > ARM_RADIUS
                || !(-STEP_BELOW..=STEP_ABOVE).contains(&(feet.z - peel.rest.z));
            peel.armed = off;
        }
    }
}

/// The peel the player in slot `own` slips on at `time`, walking as `walker`, if one:
/// on the ground and moving, their feet on a peel that landed, that they came at (armed),
/// and not their own in its first [`OWN_SAFE_MS`].
pub(crate) fn slips_on(walker: &Walker, own: u8, peels: &Peels, time: i64) -> Option<usize> {
    if !walker.on_ground || walker.velocity.truncate().length() < MIN_SLIP_SPEED {
        return None;
    }
    peels.list.iter().position(|peel| {
        peel.armed
            && time >= peel.drop.lands()
            && (peel.dropper != own || time - peel.drop.lands() >= OWN_SAFE_MS)
            && underfoot(walker.feet, peel.rest)
    })
}

/// The blow at a foot of a player slipping while moving at `velocity`: along the way
/// they walk and up ([`SLIP_PUSH`], [`SLIP_LIFT`]).
pub(crate) fn slip_push(velocity: Vec3) -> [i32; 3] {
    let way = velocity.truncate().normalize_or_zero();
    [way.x * SLIP_PUSH, way.y * SLIP_PUSH, SLIP_LIFT].map(|c| c.round() as i32)
}

/// What a trace met: where it stopped, the surface's normal, and whether it was the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ground {
    pub(crate) stop: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) sky: bool,
}

/// Where a peel dropped now goes: from the hand (over a player at `origin` moving at
/// `velocity`, looking `yaw` degrees), tossed [`TOSS_REACH`] ahead the way they move when
/// they move, else [`DROP_REACH`] ahead the way they look, short of a wall, down onto the
/// floor under it. `trace` says what a small box meets between two points. The hand, and
/// where it rests with the floor's normal; `None` with no floor flat enough near under it.
pub(crate) fn placement(
    origin: Vec3,
    velocity: Vec3,
    yaw: f32,
    mut trace: impl FnMut(Vec3, Vec3) -> Option<Ground>,
) -> Option<(Vec3, Ground)> {
    let hand = origin + Vec3::Z * HAND_HEIGHT;
    let moving = velocity.truncate();
    let (way, reach) = if moving.length() >= TOSS_SPEED {
        (moving.normalize().extend(0.0), TOSS_REACH)
    } else {
        let yaw = yaw.to_radians();
        (Vec3::new(yaw.cos(), yaw.sin(), 0.0), DROP_REACH)
    };
    let wanted = hand + way * reach;
    let ahead = match trace(hand, wanted) {
        // Short of the wall in the way.
        Some(wall) => wall.stop - way * 4.0,
        None => wanted,
    };
    let floor = trace(ahead, ahead - Vec3::Z * FLOOR_SEARCH)?;
    (floor.normal.z >= FLAT_ENOUGH && !floor.sky).then_some((hand, floor))
}

/// Whether `at` is within [`SPAWN_CLEARANCE`] of one of `spawns`.
pub(crate) fn near_spawn(at: Vec3, spawns: &[Vec3]) -> bool {
    spawns
        .iter()
        .any(|spawn| spawn.distance(at) < SPAWN_CLEARANCE)
}

/// The spawn points among a map's `entities` ([`SPAWN_CLASSES`], whatever the case).
pub(crate) fn spawn_points(entities: &[sjk_entity::Entity]) -> Vec<Vec3> {
    entities
        .iter()
        .filter(|entity| {
            entity.classname().is_some_and(|class| {
                SPAWN_CLASSES
                    .iter()
                    .any(|spawn| spawn.eq_ignore_ascii_case(class))
            })
        })
        .filter_map(|entity| entity.vector("origin").ok().flatten())
        .map(Vec3::from_array)
        .collect()
}

/// The drop of peel `number` leaving `hand` at `moment` to rest at `rest`, turned `yaw`
/// degrees.
pub(crate) fn drop_of(number: u8, moment: i32, hand: Vec3, rest: Vec3, yaw: f32) -> Drop {
    let at = rest.to_array().map(|c| c.round() as i32);
    let from = std::array::from_fn(|axis| {
        ((hand[axis] - at[axis] as f32).round() as i32).clamp(-MAX_OFFSET, MAX_OFFSET)
    });
    Drop {
        number,
        moment,
        at,
        from,
        yaw: (yaw.rem_euclid(360.0).round() as u16) % 360,
    }
}

/// A sound of the toy to play: its path, where (`None`: in both ears), and how loud.
pub(crate) struct Cue {
    pub(crate) path: &'static str,
    pub(crate) at: Option<Vec3>,
    pub(crate) volume: f32,
}

/// The toy's state.
#[derive(Default)]
pub(crate) struct Banana {
    pub(crate) on: bool,
    /// The switch as the last frame found it: turned off since, the own peels go.
    pub(crate) was_on: bool,
    /// A drop was asked since the last frame.
    pub(crate) drop_asked: bool,
    pub(crate) server: Option<std::net::SocketAddr>,
    /// The gamestate the peels are on (`sv_serverid`: a new map changes it).
    pub(crate) server_id: Option<i32>,
    pub(crate) peels: Peels,
    pub(crate) next_number: u8,
    pub(crate) last_drop: Option<Instant>,
    pub(crate) tell_at: Option<Instant>,
    /// Slips and clears still to tell: the event, and the slots still to tell (bits).
    pub(crate) outbox: Vec<(Event, u64)>,
    /// The peels used up lately: their dropper, number and when.
    pub(crate) gone: Vec<(u8, u8, Instant)>,
    /// The map's spawn points, and the map's BSP they were read from (its address).
    pub(crate) spawns: Vec<Vec3>,
    pub(crate) spawns_for: usize,
    /// This frame: the player is in a duel (no peel seen or heard), and [`SHOW_CVAR`]
    /// is off (no peel seen, nor heard in the world).
    pub(crate) duel: bool,
    pub(crate) unseen: bool,
    pub(crate) cues: Vec<Cue>,
    pub(crate) sounds_registered: Option<u64>,
    /// The game server time last stepped to (the presentation's).
    pub(crate) time: i64,
    /// For the world shots: the time the peels are drawn at.
    #[cfg(test)]
    pub(crate) shot_time: Option<i64>,
}

impl Banana {
    /// `toy_banana`: on or off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// On (`true`) or off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the toy is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// `banana_drop`: drop one at the next frame.
    pub(crate) fn drop_peel(&mut self) {
        self.drop_asked = true;
    }

    /// Forget every peel: no game, another server or another map.
    pub(crate) fn leave(&mut self) {
        self.peels.list.clear();
        self.outbox.clear();
        self.gone.clear();
        self.tell_at = None;
    }

    /// Whether peel `number` of `dropper` was used up lately.
    pub(crate) fn used_up(&self, dropper: u8, number: u8) -> bool {
        self.gone
            .iter()
            .any(|(slot, peel, _)| (*slot, *peel) == (dropper, number))
    }

    /// Note peel `number` of `dropper` used up, and forget the old notes.
    pub(crate) fn note_gone(&mut self, dropper: u8, number: u8) {
        self.gone.retain(|(_, _, when)| when.elapsed() < GONE_KEPT);
        self.gone.push((dropper, number, Instant::now()));
    }

    /// Whether a drop of moment `moment` heard at `now` (game server ms) is in time.
    pub(crate) fn in_time(moment: i32, now: i64) -> bool {
        let moment = i64::from(moment);
        moment <= now + EARLY_MS && moment > now - LIFETIME_MS
    }

    /// The landings of the frame (`time`) to hear: each peel's once, when it lands, if
    /// not too long ago.
    pub(crate) fn landings(&mut self, time: i64) {
        for peel in &mut self.peels.list {
            if peel.landed || time < peel.drop.lands() {
                continue;
            }
            peel.landed = true;
            if time - peel.drop.lands() <= LANDING_LATE_MS {
                self.cues.push(Cue {
                    path: DROP_SOUND,
                    at: Some(peel.rest),
                    volume: 0.8,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop(number: u8, moment: i32) -> Drop {
        Drop {
            number,
            moment,
            at: [0, 0, 0],
            from: [0, 0, 32],
            yaw: 0,
        }
    }

    fn peel(dropper: u8, number: u8, moment: i32, rest: Vec3) -> Peel {
        Peel::new(dropper, drop(number, moment), rest, Vec3::Z)
    }

    /// A player walking at `speed` along +X, feet at `feet`.
    fn walker(feet: Vec3, speed: f32) -> Walker {
        Walker {
            feet,
            velocity: Vec3::new(speed, 0.0, 0.0),
            on_ground: true,
        }
    }

    #[test]
    fn events_read_back_as_written_and_fit_the_relay() {
        let widest = Event::Drop(Drop {
            number: 99,
            moment: -2_147_000_000,
            at: [-65_536, 65_536, -65_536],
            from: [-128, 128, -128],
            yaw: 359,
        });
        for event in [
            widest,
            Event::Drop(drop(0, 1_234)),
            Event::Slip {
                dropper: 63,
                number: 99,
            },
            Event::Clear,
        ] {
            let text = event.text();
            assert!(text.len() <= 64, "{text}");
            assert!(
                text.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'_'
                    || b == b'-'),
                "{text}"
            );
            assert_eq!(Event::parse(&text), Some(event), "{text}");
        }
        for bad in [
            "drop_100_0_0_0_0_0_0_0_0",
            "drop_1_0_0_0_0_0_0_129_0",
            "drop_1_0_0_0_0_0_0_0_360",
            "drop_1_0_0_0_0_0_0_0",
            "drop_1_0_0_0_0_0_0_0_0_0",
            "slip_64_1",
            "slip_1_100",
            "slip_1",
            "clear_1",
            "throw_1_0_0_0_0_0_0_0",
        ] {
            assert_eq!(Event::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_player_keeps_three_peels_down_and_a_number_replaces_its_own() {
        let mut peels = Peels::default();
        for number in 0..5 {
            peels.put(peel(3, number, i32::from(number), Vec3::ZERO));
        }
        peels.put(peel(4, 0, 0, Vec3::ZERO));
        let mine: Vec<u8> = peels
            .list
            .iter()
            .filter(|peel| peel.dropper == 3)
            .map(|peel| peel.drop.number)
            .collect();
        assert_eq!(mine, [2, 3, 4]);
        peels.put(peel(3, 3, 99, Vec3::ZERO));
        assert_eq!(peels.list[peels.find(3, 3).unwrap()].drop.moment, 99);
        assert_eq!(peels.list.len(), PER_PLAYER + 1);
        peels.clear_of(3);
        assert_eq!(peels.list.len(), 1);
    }

    #[test]
    fn peels_lie_two_minutes_and_go_with_their_dropper() {
        let mut peels = Peels::default();
        peels.put(peel(3, 0, 1_000, Vec3::ZERO));
        peels.put(peel(5, 0, 1_000, Vec3::ZERO));
        // Neither dropper is known to the hub yet: both stay.
        peels.prune(2_000, 0);
        assert_eq!(peels.list.len(), 2);
        // Both seen, then 5 leaves: theirs goes.
        peels.prune(3_000, 1 << 3 | 1 << 5);
        peels.prune(4_000, 1 << 3);
        assert_eq!(peels.list.len(), 1);
        assert_eq!(peels.list[0].dropper, 3);
        // Shrinking away at the end of its time, then gone.
        assert_eq!(peels.list[0].level(1_000 + LIFETIME_MS - FADE_MS), 1.0);
        assert!(peels.list[0].level(1_000 + LIFETIME_MS - FADE_MS / 2) < 1.0);
        peels.prune(1_000 + LIFETIME_MS - 1, 1 << 3);
        assert_eq!(peels.list.len(), 1);
        peels.prune(1_000 + LIFETIME_MS, 1 << 3);
        assert!(peels.list.is_empty());
        // A drop heard late, or too early, is not put out.
        assert!(Banana::in_time(1_000, 1_000 + LIFETIME_MS - 1));
        assert!(!Banana::in_time(1_000, 1_000 + LIFETIME_MS));
        assert!(!Banana::in_time(10_000, 6_000));
    }

    #[test]
    fn only_a_player_with_the_toy_on_and_free_to_fall_drops_or_slips() {
        use crate::ragdoll::Ragdoll;
        let free = Ragdoll::for_tests(true, false, false);
        assert!(may_play(true, &free));
        assert!(!may_play(false, &free), "the toy off");
        // Not on a game server (a local game, the menus), limp already, or spared: dead,
        // spectating, in a duel, with the flag, hacked, held by the server, riding.
        assert!(!may_play(true, &Ragdoll::default()));
        assert!(
            !may_play(true, &Ragdoll::for_tests(true, false, true)),
            "limp"
        );
        assert!(
            !may_play(true, &Ragdoll::for_tests(true, true, false)),
            "spared"
        );
        assert!(
            !may_play(true, &Ragdoll::for_tests(false, false, false)),
            "no game"
        );
    }

    #[test]
    fn a_used_up_peel_heard_of_late_stays_gone() {
        let mut banana = Banana::default();
        assert!(!banana.used_up(3, 7));
        banana.note_gone(3, 7);
        assert!(banana.used_up(3, 7));
        assert!(!banana.used_up(3, 8) && !banana.used_up(4, 7));
    }

    #[test]
    fn a_player_walking_onto_a_peel_slips_but_not_standing_off_ground_or_slow() {
        let mut peels = Peels::default();
        peels.put(peel(3, 0, 0, Vec3::new(100.0, 0.0, 0.0)));
        let time = FLIGHT_MS + 10;
        // Coming at it from afar arms it.
        arm(&mut peels, Vec3::ZERO, time);
        assert!(peels.list[0].armed);
        let on_it = Vec3::new(100.0 - SLIP_RADIUS + 1.0, 0.0, 0.0);
        assert_eq!(slips_on(&walker(on_it, 250.0), 5, &peels, time), Some(0));
        // Walking slowly (crouched and walking still slips), standing, in the air.
        assert_eq!(slips_on(&walker(on_it, 60.0), 5, &peels, time), Some(0));
        assert_eq!(
            slips_on(&walker(on_it, MIN_SLIP_SPEED - 1.0), 5, &peels, time),
            None
        );
        let airborne = Walker {
            on_ground: false,
            ..walker(on_it, 250.0)
        };
        assert_eq!(slips_on(&airborne, 5, &peels, time), None);
        // Beside it, or on a floor well above or below it.
        let beside = Vec3::new(100.0, SLIP_RADIUS + 1.0, 0.0);
        assert_eq!(slips_on(&walker(beside, 250.0), 5, &peels, time), None);
        let above = on_it + Vec3::Z * (STEP_ABOVE + 1.0);
        assert_eq!(slips_on(&walker(above, 250.0), 5, &peels, time), None);
        let step_up = on_it + Vec3::Z * STEP_ABOVE;
        assert_eq!(slips_on(&walker(step_up, 250.0), 5, &peels, time), Some(0));
        let below = on_it - Vec3::Z * (STEP_BELOW + 1.0);
        assert_eq!(slips_on(&walker(below, 250.0), 5, &peels, time), None);
        // Not before it has landed.
        let mut flying = Peels::default();
        flying.put(peel(3, 0, 0, Vec3::new(100.0, 0.0, 0.0)));
        arm(&mut flying, Vec3::ZERO, FLIGHT_MS - 1);
        assert!(!flying.list[0].armed);
        assert_eq!(
            slips_on(&walker(on_it, 250.0), 5, &flying, FLIGHT_MS - 1),
            None
        );
    }

    #[test]
    fn a_peel_landing_underfoot_waits_until_the_player_steps_off_it() {
        let mut peels = Peels::default();
        peels.put(peel(3, 0, 0, Vec3::ZERO));
        let time = FLIGHT_MS + 10;
        let feet = Vec3::new(5.0, 0.0, 0.0);
        arm(&mut peels, feet, time);
        assert_eq!(slips_on(&walker(feet, 250.0), 5, &peels, time), None);
        // Still close: still waiting.
        arm(&mut peels, Vec3::new(ARM_RADIUS - 1.0, 0.0, 0.0), time);
        assert!(!peels.list[0].armed);
        // Off it, then back onto it.
        arm(&mut peels, Vec3::new(ARM_RADIUS + 1.0, 0.0, 0.0), time);
        assert_eq!(slips_on(&walker(feet, 250.0), 5, &peels, time), Some(0));
    }

    #[test]
    fn the_dropper_is_safe_from_a_fresh_peel_of_their_own_only() {
        let mut peels = Peels::default();
        peels.put(peel(3, 0, 0, Vec3::ZERO));
        let landed = FLIGHT_MS;
        arm(&mut peels, Vec3::new(50.0, 0.0, 0.0), landed);
        let feet = Vec3::new(2.0, 0.0, 0.0);
        assert_eq!(
            slips_on(&walker(feet, 250.0), 3, &peels, landed + 100),
            None
        );
        assert_eq!(
            slips_on(&walker(feet, 250.0), 5, &peels, landed + 100),
            Some(0)
        );
        assert_eq!(
            slips_on(&walker(feet, 250.0), 3, &peels, landed + OWN_SAFE_MS),
            Some(0)
        );
    }

    #[test]
    fn a_slip_sweeps_the_feet_the_way_the_player_walks_and_up() {
        let push = slip_push(Vec3::new(0.0, -300.0, 0.0));
        assert_eq!(push, [0, -SLIP_PUSH as i32, SLIP_LIFT as i32]);
        let diagonal = slip_push(Vec3::new(200.0, 200.0, -50.0));
        assert_eq!(diagonal[0], diagonal[1]);
        assert!(diagonal.iter().all(|c| c.abs() <= crate::ragdoll::MAX_PUSH));
    }

    /// The Ragdoll's body, slipping while walking along +X at a few speeds and moments:
    /// its feet fly up over its head, and it lands on its back, its head behind its feet,
    /// within a few body lengths.
    #[test]
    fn a_slip_throws_the_feet_up_and_lays_the_body_on_its_back() {
        use crate::ragdoll::{L_FOOT, R_FOOT};
        const HEAD: usize = 2;
        for speed in [60, 120, 250, 300] {
            for moment in [1_000, 4_321, 77_777] {
                let start = crate::ragdoll::Start {
                    moment,
                    origin: [0, 0, 24],
                    velocity: [speed, 0, 0],
                    yaw: 0,
                    scale: crate::ragdoll::STOCK_SCALE,
                };
                let strike = crate::ragdoll::Strike {
                    step: 0,
                    joint: L_FOOT as u8,
                    push: slip_push(Vec3::new(speed as f32, 0.0, 0.0)),
                };
                let steps = crate::ragdoll::fall_on_a_floor(&start, Some(strike), 400);
                let feet_up = steps
                    .iter()
                    .filter(|points| points[L_FOOT].z.max(points[R_FOOT].z) > points[HEAD].z)
                    .count();
                let top = steps
                    .iter()
                    .map(|points| points[L_FOOT].z.max(points[R_FOOT].z))
                    .fold(0.0, f32::max);
                let last = steps.last().expect("steps");
                let feet = (last[L_FOOT] + last[R_FOOT]) * 0.5;
                let case = format!("{speed} u/s, moment {moment}");
                assert!(feet_up >= 15, "{case}: feet over the head {feet_up} steps");
                assert!(top >= 45.0, "{case}: the feet rose to {top}");
                assert!(last[HEAD].z < 15.0, "{case}: lying");
                assert!(last[HEAD].x < feet.x, "{case}: on its back, head behind");
                assert!(feet.x > 0.0 && feet.x < 250.0, "{case}: slid {}", feet.x);
            }
        }
    }

    /// Floor at 0 under the whole world, a wall at x = 50 if `wall`; the sky at x < -500.
    fn world(wall: bool) -> impl FnMut(Vec3, Vec3) -> Option<Ground> {
        move |from: Vec3, to: Vec3| {
            if wall && to.x > 50.0 && from.x <= 50.0 {
                let t = (50.0 - from.x) / (to.x - from.x);
                return Some(Ground {
                    stop: from.lerp(to, t),
                    normal: -Vec3::X,
                    sky: false,
                });
            }
            (to.z < 0.0 && from.z >= 0.0).then(|| {
                let t = from.z / (from.z - to.z);
                Ground {
                    stop: from.lerp(to, t),
                    normal: Vec3::Z,
                    sky: from.x < -500.0,
                }
            })
        }
    }

    #[test]
    fn a_peel_drops_ahead_of_the_feet_or_is_tossed_the_way_the_player_moves() {
        let origin = Vec3::new(0.0, 0.0, 24.0);
        // Standing, looking +Y: just ahead of the feet, on the floor.
        let (hand, rest) = placement(origin, Vec3::ZERO, 90.0, world(false)).unwrap();
        assert_eq!(hand, origin + Vec3::Z * HAND_HEIGHT);
        assert!((rest.stop - Vec3::new(0.0, DROP_REACH, 0.0)).length() < 0.01);
        assert_eq!(rest.normal, Vec3::Z);
        // Running along -X while looking +Y: tossed ahead along -X.
        let (_, tossed) =
            placement(origin, Vec3::new(-250.0, 0.0, 0.0), 90.0, world(false)).unwrap();
        assert!((tossed.stop - Vec3::new(-TOSS_REACH, 0.0, 0.0)).length() < 0.01);
        // A wall in the way: short of it.
        let (_, walled) = placement(origin, Vec3::new(250.0, 0.0, 0.0), 0.0, world(true)).unwrap();
        assert!(walled.stop.x < 50.0 && walled.stop.x > 40.0, "{walled:?}");
        // Over the sky, or no floor under it: none.
        let over_sky = Vec3::new(-600.0, 0.0, 24.0);
        assert!(placement(over_sky, Vec3::ZERO, 0.0, world(false)).is_none());
        let in_the_air = origin + Vec3::Z * (FLOOR_SEARCH + 40.0);
        assert!(placement(in_the_air, Vec3::ZERO, 0.0, world(false)).is_none());
        // A steep slope takes none.
        let steep = |_: Vec3, to: Vec3| {
            Some(Ground {
                stop: to,
                normal: Vec3::new(0.8, 0.0, 0.6),
                sky: false,
            })
        };
        assert!(placement(origin, Vec3::ZERO, 0.0, steep).is_none());
    }

    #[test]
    fn no_peel_by_a_spawn_point() {
        let lump = r#"{
"classname" "worldspawn"
}
{
"classname" "info_player_deathmatch"
"origin" "100 0 24"
}
{
"classname" "team_CTF_redplayer"
"origin" "-500 0 24"
}
{
"classname" "info_player_intermission"
"origin" "0 900 24"
}
"#;
        let entities = sjk_entity::parse_entity_lump(lump.as_bytes()).unwrap();
        let spawns = spawn_points(&entities);
        assert_eq!(spawns.len(), 2, "the intermission spot is no spawn");
        assert!(near_spawn(Vec3::new(100.0, 40.0, 0.0), &spawns));
        assert!(near_spawn(Vec3::new(-500.0, 60.0, 0.0), &spawns));
        assert!(!near_spawn(
            Vec3::new(100.0, SPAWN_CLEARANCE + 1.0, 24.0),
            &spawns
        ));
        assert!(!near_spawn(Vec3::new(0.0, 900.0, 0.0), &spawns));
    }

    #[test]
    fn a_drop_carries_the_hand_and_the_rest_and_lands_flat() {
        let hand = Vec3::new(10.4, -3.6, 56.0);
        let rest = Vec3::new(30.2, -3.0, 24.0);
        let drop = drop_of(7, 5_000, hand, rest, -90.0);
        assert_eq!(drop.at, [30, -3, 24]);
        assert_eq!(drop.from, [-20, -1, 32]);
        assert_eq!(drop.yaw, 270);
        // Far over the floor, the hand's offset is held to the event's bounds.
        let far = drop_of(7, 5_000, rest + Vec3::Z * 900.0, rest, 0.0);
        assert_eq!(far.from[2], MAX_OFFSET);
        let peel = Peel::new(3, drop, rest, Vec3::Z);
        // It leaves from the hand, arcs over the way, and lies on the floor once landed.
        let (start, _) = peel.pose(5_000);
        assert!((start - (rest + Vec3::new(-20.0, -1.0, 32.0))).length() < 0.01);
        let (middle, _) = peel.pose(5_000 + FLIGHT_MS / 2);
        let straight = (start + rest) * 0.5;
        assert!((middle.z - straight.z - ARC).abs() < 0.01, "{middle}");
        let (end, turn) = peel.pose(5_000 + FLIGHT_MS);
        assert_eq!(end, rest);
        assert!((turn * Vec3::Z - Vec3::Z).length() < 1e-4);
        // On a slope, it lies along the floor.
        let sloped = Peel::new(3, drop, rest, Vec3::new(0.0, 0.6, 0.8));
        let (_, turn) = sloped.pose(9_000);
        assert!((turn * Vec3::Z - Vec3::new(0.0, 0.6, 0.8)).length() < 1e-4);
    }

    #[test]
    fn each_landing_is_heard_once_and_not_when_long_past() {
        let mut banana = Banana::default();
        banana.peels.put(peel(3, 0, 0, Vec3::ZERO));
        banana.peels.put(peel(4, 0, -10_000, Vec3::ZERO));
        banana.landings(FLIGHT_MS - 1);
        assert!(banana.cues.is_empty());
        banana.landings(FLIGHT_MS);
        banana.landings(FLIGHT_MS + 16);
        assert_eq!(banana.cues.len(), 1);
        assert_eq!(banana.cues[0].path, DROP_SOUND);
        assert!(banana.peels.list.iter().all(|peel| peel.landed));
    }
}
