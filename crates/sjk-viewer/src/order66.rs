//! Order 66, SJK's meme toy (`toy_order66`; Sol's idea, 17/10/2026): every player who
//! has it on dies at once, blown apart, when one of them executes the order (`order66`).
//! No game server knows of it. The toy travels through the SJK hub as a toy in the
//! player's look (`looks.rs`, [`TOY`]), and the order as a toy event
//! ([`EVENT_PREFIX`] and the moment it falls due, in the game server's time).
//!
//! The order falls due [`COUNTDOWN_MS`] after it is given, on the game server's clock,
//! which every client on the server shares through its snapshots: each client counts
//! down on the centre of the screen and, at that moment, sends the stock `kill` command
//! for its own player, so the deaths land together whatever each client's way to the
//! hub took. A client sends it only while its own toy is on and its player plays: not
//! spectating, not in a duel, not carrying a flag ([`spared`]). One order counts per
//! server every [`COOLDOWN_MS`]: every client drops one that comes sooner, so a sender
//! cannot keep a server in a kill loop.
//!
//! A player who has the toy on and dies in the order's window ([`BLAST_EARLY_MS`] before
//! to [`BLAST_WINDOW_MS`] after) blows up on every SJK client that sees it: the thermal
//! detonator's explosion ([`EXPLOSION`]), and the body flies apart in six pieces (head,
//! arms, legs and trunk, `dismember.rs`'s limb meshes, [`GpuState::blow_apart`]) that
//! bounce off the world, smoke and sink away. Where dismemberment is off
//! (`cg_dismember 0`) the body bursts into confetti instead ([`CONFETTI`]). Players on
//! stock clients see an ordinary suicide.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec3};
use sjk_bsp::Aabb;
use sjk_runtime::EntityId;
use std::net::SocketAddr;
use std::time::Instant;

/// The toy's console command (a toy is a `toy_<name>` command): turn it on or off.
pub(crate) const COMMAND: &str = "toy_order66";
/// Execute the order.
pub(crate) const EXECUTE_COMMAND: &str = "order66";
/// The toy's id in a look's `toys` and in toy events (`PROTOCOL.md`, "Looks").
pub(crate) const TOY: &str = "order66";
/// The order's toy event: this, then the game server time (ms) it falls due at.
pub(crate) const EVENT_PREFIX: &str = "execute_";
/// From the order to the deaths, in game server milliseconds.
pub(crate) const COUNTDOWN_MS: i32 = 3_000;
/// The shortest time between two orders on one server.
pub(crate) const COOLDOWN_MS: i32 = 30_000;
/// The furthest ahead an order may fall due; one further is dropped.
const AHEAD_MS: i32 = 10_000;
/// An order heard this long after it fell due is dropped.
const LATE_MS: i32 = 500;
/// A holder of the toy who dies from this long before the order's moment...
pub(crate) const BLAST_EARLY_MS: i32 = 100;
/// ...to this long after it blows up.
pub(crate) const BLAST_WINDOW_MS: i32 = 2_000;
/// The explosion where a player blows up (the thermal detonator's, sound included).
pub(crate) const EXPLOSION: &str = "thermal/explosion";
/// The confetti that takes the place of the pieces without dismemberment.
pub(crate) const CONFETTI: &str = "sjk/order66_confetti";
/// The smoke a flying piece trails.
const SMOKE: &str = crate::dismember::SMOKE;
/// Played to the players who have the toy on when an order is given: a game path SJK
/// does not ship (nothing plays where it is missing).
pub(crate) const ORDER_SOUND: &str = "sound/sjk/order66/order.wav";
/// Each second of the countdown: the stock thermal detonator's warning beep.
pub(crate) const TICK_SOUND: &str = "sound/weapons/thermal/warning.wav";
/// The source of the order's sound, apart from the players' own (`ENTITYNUM_WORLD`).
const ORDER_SOURCE: u32 = 1022;
/// The pieces' entity ids start here, far past the game's entity numbers, so nothing
/// that reads a slot or an entity number from an id takes one for a player.
const PIECE_ID_BASE: u64 = 1 << 40;
/// The most pieces flying at once; past it a player blows up without pieces.
const PIECES_MAX: usize = 48;
/// The pieces of one player.
const PIECES_PER_PLAYER: usize = 6;
/// Pieces are made only for a player this close to the camera.
const PIECE_REACH: f32 = 3_000.0;
/// How many limb meshes a frame may upload for the blasts (a pooled one is free).
const UPLOADS_PER_FRAME: usize = PIECES_PER_PLAYER;
/// Where the ground pulls a piece, in units per second per second (`g_gravity`).
const GRAVITY: f32 = 800.0;
/// How much of its speed a piece keeps off a bounce, and of its spin.
const BOUNCE: f32 = 0.45;
const SPIN_KEPT: f32 = 0.6;
/// A piece slower than this on a floor (a surface facing up this much) lies still.
const REST_SPEED: f32 = 60.0;
const FLOOR: f32 = 0.7;
/// How long a piece lies about, then how long it takes to sink away and how far.
const LIFE: f32 = 6.0;
const SINK: f32 = 1.0;
const SINK_DEPTH: f32 = 14.0;
/// Seconds between the puffs of smoke of a flying piece.
const SMOKE_EVERY: f32 = 0.18;
/// Half a piece's size in the world's collision.
const PIECE_HALF: f32 = 3.0;
/// `CONTENTS_SOLID`.
const CONTENTS_SOLID: u32 = 1;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
/// `PMF_FOLLOW`: the player state is that of the player being followed.
const PMF_FOLLOW: u16 = 4096;
/// `GT_DUEL` and `GT_POWERDUEL`.
const GT_DUEL: i32 = 3;
const GT_POWERDUEL: i32 = 4;
/// `PW_REDFLAG`, `PW_BLUEFLAG`, `PW_NEUTRALFLAG`.
const FLAGS: [usize; 3] = [4, 5, 6];
const SLOTS: usize = crate::looks::SLOTS;

/// A bundled file's bytes, by its name in `assets/order66`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/order66/", $name))
    };
}

/// The toy's icon and the order's (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_order66.png";
pub(crate) const EXECUTE_ICON: &str = "gfx/sjk/order66_execute.png";

/// The bundled files at their game paths (made by `scripts/order66_assets.py`, but the
/// effect and the shader).
const FILES: [(&str, &[u8]); 10] = [
    (ICON, bundled!("toy_order66.png")),
    (EXECUTE_ICON, bundled!("order66_execute.png")),
    (
        "effects/sjk/order66_confetti.efx",
        bundled!("order66_confetti.efx"),
    ),
    ("shaders/sjk_order66.shader", bundled!("order66.shader")),
    (
        "gfx/sjk/order66/confetti_red.png",
        bundled!("confetti_red.png"),
    ),
    (
        "gfx/sjk/order66/confetti_yellow.png",
        bundled!("confetti_yellow.png"),
    ),
    (
        "gfx/sjk/order66/confetti_green.png",
        bundled!("confetti_green.png"),
    ),
    (
        "gfx/sjk/order66/confetti_cyan.png",
        bundled!("confetti_cyan.png"),
    ),
    (
        "gfx/sjk/order66/confetti_blue.png",
        bundled!("confetti_blue.png"),
    ),
    (
        "gfx/sjk/order66/confetti_pink.png",
        bundled!("confetti_pink.png"),
    ),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Order 66", FILES.iter().copied())?;
    Ok(())
}

/// Whether `id` is one of Order 66's pieces.
pub(crate) fn is_piece(id: EntityId) -> bool {
    id.get() >= PIECE_ID_BASE
}

/// The moment an order's toy event names, if it is one.
pub(crate) fn order_moment(event: &str) -> Option<i32> {
    event.strip_prefix(EVENT_PREFIX)?.parse().ok()
}

/// The order: when it falls due (game server ms) and who gave it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Order {
    at: i32,
    sender: u8,
    /// The countdown's second last shown (`i32::MAX`: none yet).
    shown: i32,
    /// Its moment came and this client acted on it.
    done: bool,
}

/// Why the player's own `kill` is not sent at the order's moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spared {
    /// Spectating, or at the intermission: nothing to say.
    Away,
    /// Already dead: nothing to say.
    Dead,
    Duel,
    Flag,
    /// The game server holds the player's view (a kiss, a grapple, a kick: `held_view`).
    /// Never from [`spared`]: the toys that steer the player add it, as their commands
    /// would fight the server's view.
    Held,
}

impl Spared {
    /// What the centre of the screen says, if anything.
    fn words(self) -> Option<&'static str> {
        match self {
            Self::Away | Self::Dead | Self::Held => None,
            Self::Duel => Some("you are in a duel"),
            Self::Flag => Some("you carry the flag"),
        }
    }
}

/// Whether `player` (the player's own state, or the followed player's) is in a duel: a
/// private one, or any game of a Duel or Power Duel server (`gametype`, `g_gametype`).
pub(crate) fn duelling(player: &sjk_protocol::PlayerState, gametype: i32) -> bool {
    matches!(gametype, GT_DUEL | GT_POWERDUEL) || player.duel_in_progress()
}

/// Why the player's own `kill` is not sent, if it is not: `player` is the player's
/// state, `gametype` the server's `g_gametype`.
pub(crate) fn spared(player: &sjk_protocol::PlayerState, gametype: i32) -> Option<Spared> {
    // `pmtype_t`: PM_SPECTATOR 4, PM_DEAD 5, PM_INTERMISSION 7, PM_SPINTERMISSION 8.
    if player.movement_flags() & PMF_FOLLOW != 0
        || player.is_spectator()
        || matches!(player.movement_type(), 7 | 8)
    {
        return Some(Spared::Away);
    }
    if player.health() <= 0 || player.movement_type() == 5 {
        return Some(Spared::Dead);
    }
    if duelling(player, gametype) {
        return Some(Spared::Duel);
    }
    let time = player.command_time();
    if FLAGS.iter().any(|&flag| player.powerup_active(flag, time)) {
        return Some(Spared::Flag);
    }
    None
}

/// A one-shot sound to play: the order's or a tick of the countdown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cue {
    path: &'static str,
    /// Its source: the player's own slot, or [`ORDER_SOURCE`].
    source: u32,
}

/// An explosion to set off where a player blew up, with confetti or not.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Blast {
    origin: Vec3,
    confetti: bool,
    seed: u32,
}

/// One piece flying, then lying, then sinking away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Flight {
    id: EntityId,
    /// Where its pivot bone is, and how fast it goes.
    position: Vec3,
    velocity: Vec3,
    /// Its owner's turn at the blast, then the spin about `axis` on top.
    base: Quat,
    axis: Vec3,
    spin: f32,
    angle: f32,
    /// Its owner's model scale.
    scale: [f32; 3],
    /// Seconds since the blast.
    age: f32,
    resting: bool,
    /// Seconds to the next puff of smoke.
    smoke: f32,
}

impl Flight {
    /// Move `dt` seconds on: gravity, then a bounce off what `trace` finds between two
    /// points (where it stopped and the surface's normal; `None`: nothing in the way),
    /// slower each time, until it lies slow on a floor.
    fn step(&mut self, dt: f32, mut trace: impl FnMut(Vec3, Vec3) -> Option<(Vec3, Vec3)>) {
        self.age += dt;
        if self.resting {
            return;
        }
        self.velocity.z -= GRAVITY * dt;
        self.angle += self.spin * dt;
        let to = self.position + self.velocity * dt;
        match trace(self.position, to) {
            None => self.position = to,
            Some((at, normal)) => {
                self.position = at + normal * 0.25;
                self.velocity = (self.velocity - 2.0 * self.velocity.dot(normal) * normal) * BOUNCE;
                self.spin *= SPIN_KEPT;
                if normal.z > FLOOR && self.velocity.length() < REST_SPEED {
                    self.resting = true;
                    self.velocity = Vec3::ZERO;
                }
            }
        }
    }

    /// Where its mesh goes for a pivot bone at `pivot` in model space: the pivot at the
    /// flight's position, the piece turned, sunk once it has lain long enough.
    fn placement(&self, pivot: [f32; 3]) -> (Vec3, Quat) {
        let rotation = Quat::from_axis_angle(self.axis, self.angle) * self.base;
        let sink = ((self.age - LIFE) / SINK).clamp(0.0, 1.0) * SINK_DEPTH;
        let pivot = Vec3::from_array(pivot) * Vec3::from_array(self.scale);
        (self.position - Vec3::Z * sink - rotation * pivot, rotation)
    }

    fn gone(&self) -> bool {
        self.age >= LIFE + SINK
    }

    /// A puff of smoke is due (and the next one set), while it flies.
    fn smoke_due(&mut self, dt: f32) -> bool {
        if self.resting {
            return false;
        }
        self.smoke -= dt;
        if self.smoke > 0.0 {
            return false;
        }
        self.smoke = SMOKE_EVERY;
        true
    }
}

/// A number in 0..1 from `seed` and `salt`, the same every time.
fn unit(seed: u32, salt: u32) -> f32 {
    let mut mixed = seed ^ salt.wrapping_mul(0x9e37_79b9);
    mixed ^= mixed >> 16;
    mixed = mixed.wrapping_mul(0x85eb_ca6b);
    mixed ^= mixed >> 13;
    mixed = mixed.wrapping_mul(0xc2b2_ae35);
    mixed ^= mixed >> 16;
    (mixed >> 8) as f32 / (1 << 24) as f32
}

/// A number between `low` and `high` from `seed` and `salt`.
fn between(seed: u32, salt: u32, low: f32, high: f32) -> f32 {
    low + (high - low) * unit(seed, salt)
}

/// How a piece starts off: its pivot at `start` on a body standing at `centre`, thrown
/// out from the middle and up (the head highest), spinning; `seed` and `salt` choose.
fn throw(start: Vec3, centre: Vec3, head: bool, seed: u32, salt: u32) -> (Vec3, Vec3, f32) {
    let random = |n: u32| unit(seed, salt * 16 + n) * 2.0 - 1.0;
    let out = (start - centre).truncate();
    let out = if out.length_squared() > 1.0 {
        out.normalize()
    } else {
        glam::Vec2::new(random(0), random(1)).normalize_or(glam::Vec2::X)
    };
    let jitter = glam::Vec2::new(random(2), random(3)) * 0.4;
    let level = (out + jitter).normalize_or(out) * between(seed, salt * 16 + 4, 180.0, 340.0);
    let up = between(seed, salt * 16 + 5, 260.0, 420.0) + if head { 180.0 } else { 0.0 };
    let axis = Vec3::new(random(6), random(7), random(8)).normalize_or(Vec3::X);
    let spin = between(seed, salt * 16 + 9, 6.0, 14.0) * if random(10) < 0.0 { -1.0 } else { 1.0 };
    (level.extend(up), axis, spin)
}

/// The toy's state: whether it is on, the order heard or given, who was seen dead and
/// blown up, the pieces flying, and the sounds and explosions to come.
pub(crate) struct Order66 {
    on: bool,
    /// The server the order is for.
    server: Option<SocketAddr>,
    order: Option<Order>,
    /// When the last order fell due, for [`COOLDOWN_MS`].
    last: Option<i32>,
    /// The slots seen dead in the last snapshot, and those blown up for this order.
    dead: u64,
    blasted: u64,
    flights: Vec<Flight>,
    next_piece: u64,
    cues: Vec<Cue>,
    blasts: Vec<Blast>,
    /// The audio output ([`crate::GameAudio::generation`]) its sounds were registered
    /// with: a new output (`snd_restart`) registers them again.
    sounds_registered: Option<u64>,
    /// When the pieces last moved.
    moved: Option<Instant>,
    /// The puffs of smoke of this frame's pieces (kept for its room).
    smoke: Vec<(Vec3, u32)>,
    /// The pieces hold still, for the world shots.
    #[cfg(test)]
    pub(crate) shot_frozen: bool,
    /// The player a world shot blew up, kept hidden though the demo has them alive.
    #[cfg(test)]
    pub(crate) shot_hidden: Option<EntityId>,
}

impl Default for Order66 {
    fn default() -> Self {
        Self {
            on: false,
            server: None,
            order: None,
            last: None,
            dead: 0,
            blasted: 0,
            flights: Vec::with_capacity(PIECES_MAX),
            next_piece: PIECE_ID_BASE,
            cues: Vec::with_capacity(4),
            blasts: Vec::with_capacity(8),
            sounds_registered: None,
            moved: None,
            smoke: Vec::with_capacity(PIECES_MAX),
            #[cfg(test)]
            shot_frozen: false,
            #[cfg(test)]
            shot_hidden: None,
        }
    }
}

impl Order66 {
    /// `toy_order66`: turn the toy on or off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// Turn it on (`true`) or off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the player's own toy is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Forget the order and who died: no game, or another server.
    fn leave(&mut self) {
        self.server = None;
        self.order = None;
        self.last = None;
        self.dead = 0;
        self.blasted = 0;
    }

    /// The game server's clock went back (a new map): forget what it counted.
    fn rewind(&mut self, now: i32) {
        if self.last.is_some_and(|last| now < last - AHEAD_MS) {
            self.order = None;
            self.last = None;
            self.blasted = 0;
        }
    }

    /// An order from `sender` falling due at `at`, heard at `now` (game server ms):
    /// taken unless it is too late, too far ahead, or sooner than [`COOLDOWN_MS`] after
    /// the last one (a repeat included). Whether it was taken.
    fn hear(&mut self, at: i32, sender: u8, now: i32) -> bool {
        if at < now - LATE_MS || at > now + AHEAD_MS {
            return false;
        }
        if self.last.is_some_and(|last| at < last + COOLDOWN_MS) {
            return false;
        }
        self.order = Some(Order {
            at,
            sender,
            shown: i32::MAX,
            done: false,
        });
        self.last = Some(at);
        self.blasted = 0;
        true
    }

    /// The player in slot `own` gives the order at `now`: when it falls due, or the
    /// seconds to wait for the cooldown.
    fn give(&mut self, own: u8, now: i32) -> Result<i32, i32> {
        let at = now + COUNTDOWN_MS;
        if let Some(last) = self.last
            && at < last + COOLDOWN_MS
        {
            return Err((last + COOLDOWN_MS - at + 999) / 1000);
        }
        self.hear(at, own, now);
        Ok(at)
    }

    /// The countdown's second to show at `now` when it changed: 3, 2, 1.
    fn countdown(&mut self, now: i32) -> Option<i32> {
        let order = self.order.as_mut().filter(|order| now < order.at)?;
        let seconds = (order.at - now + 999) / 1000;
        (seconds != order.shown).then(|| {
            order.shown = seconds;
            seconds
        })
    }

    /// The order's moment came at `now`: its sender, once.
    fn due(&mut self, now: i32) -> Option<u8> {
        let order = self
            .order
            .as_mut()
            .filter(|order| now >= order.at && !order.done)?;
        order.done = true;
        Some(order.sender)
    }

    /// Whether the order heard or given was given by the player in `slot`.
    fn given_by(&self, slot: usize) -> bool {
        self.order
            .is_some_and(|order| usize::from(order.sender) == slot)
    }

    /// Whether `now` lies in the order's window, where holders who die blow up.
    fn in_window(&self, now: i32) -> bool {
        self.order.is_some_and(|order| {
            now >= order.at - BLAST_EARLY_MS && now <= order.at + BLAST_WINDOW_MS
        })
    }

    /// Note who is dead now (`dead` bits over the slots present in `seen`): the holders
    /// of the toy (`holds`) newly dead in the order's window blow up, each once. Their
    /// bits.
    fn deaths(&mut self, seen: u64, dead: u64, now: i32, holds: impl Fn(usize) -> bool) -> u64 {
        let newly = dead & !self.dead & seen;
        self.dead = (self.dead & !seen) | (dead & seen);
        if !self.in_window(now) {
            return 0;
        }
        let mut blown = 0;
        for slot in 0..SLOTS {
            let bit = 1_u64 << slot;
            if newly & bit != 0 && self.blasted & bit == 0 && holds(slot) {
                blown |= bit;
            }
        }
        self.blasted |= blown;
        blown
    }

    /// The first of `count` piece ids, set aside.
    fn reserve(&mut self, count: usize) -> u64 {
        let first = self.next_piece;
        self.next_piece = self.next_piece.wrapping_add(count as u64);
        if self.next_piece < PIECE_ID_BASE {
            self.next_piece = PIECE_ID_BASE;
        }
        first
    }
}

impl GpuState {
    /// `toy_order66`: turn the toy on or off.
    pub(crate) fn toggle_order66(&mut self) {
        self.order66.toggle();
        self.order66_centre(crate::toy_switch_words::switched(
            "Order 66",
            self.order66.on(),
        ));
    }

    /// Say `text` on the centre of the screen.
    fn order66_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// `order66`: execute the order, for every player who has the toy on (the own
    /// toy must be on, in a match on a server). The others hear it through the hub.
    pub(crate) fn execute_order66(&mut self) {
        if !self.order66.on {
            self.order66_centre(format!(
                "Turn Order 66 on first ({COMMAND}, or the quick wheel's Toys page)"
            ));
            return;
        }
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.order66_centre("Join a server to execute Order 66".to_owned());
            return;
        };
        let game_state = session.game_state();
        let Ok(own) = u8::try_from(game_state.client_num) else {
            return;
        };
        let snapshot = session.latest_snapshot();
        if matches!(
            spared(&snapshot.player, 0),
            Some(Spared::Away | Spared::Dead)
        ) {
            self.order66_centre("Order 66 is given alive, in the match".to_owned());
            return;
        }
        let server = session.server();
        let now = snapshot.server_time;
        // Another player with the toy names the event's target; the hub wants one
        // that is not the sender, and the readers do not look at it.
        let target = (0..SLOTS)
            .find(|&slot| slot != usize::from(own) && self.looks.toy(slot, TOY))
            .unwrap_or((usize::from(own) + 1) % SLOTS) as u8;
        if self.order66.server != Some(server) {
            self.order66.leave();
            self.order66.server = Some(server);
        }
        match self.order66.give(own, now) {
            Err(wait) => self.order66_centre(format!("Order 66 again in {wait} s")),
            Ok(at) => {
                self.order66.cues.push(Cue {
                    path: ORDER_SOUND,
                    source: ORDER_SOURCE,
                });
                let event = format!("{EVENT_PREFIX}{at}");
                if !crate::player_identity::toy_event(TOY, &event, target)
                    && let Some(console) = self.console.as_mut()
                {
                    console.push_log(
                        "^3Order 66: the SJK hub is off (cl_identity 0), so only you got the order",
                    );
                }
            }
        }
    }

    /// The frame's toy logic, after the looks: the hub's orders (`events`, this
    /// server's), the countdown, the own `kill` at the order's moment and the holders
    /// who blow up.
    pub(crate) fn update_order66(&mut self, events: &[sjk_identity::ToyEvent]) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.order66.leave();
            return;
        };
        let server = session.server();
        if self.order66.server != Some(server) {
            self.order66.leave();
            self.order66.server = Some(server);
        }
        let snapshot = session.latest_snapshot();
        let game_state = session.game_state();
        let now = snapshot.server_time;
        let own = u8::try_from(game_state.client_num).ok();
        self.order66.rewind(now);
        // The hub's orders: a sender counts only when the game shows the name they claimed,
        // and never one blocked on this PC.
        for event in events {
            if event.toy != TOY
                || Some(event.slot) == own
                || self.muted_players.hides_fun(u16::from(event.slot))
            {
                continue;
            }
            let Some(at) = order_moment(&event.event) else {
                continue;
            };
            let shown = crate::player_identity::shown_name(game_state, usize::from(event.slot));
            if !shown.is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown)) {
                continue;
            }
            if self.order66.hear(at, event.slot, now) && self.order66.on {
                self.order66.cues.push(Cue {
                    path: ORDER_SOUND,
                    source: ORDER_SOURCE,
                });
            }
        }
        // The countdown, for the players in the order.
        let own_slot = own.map_or(0, u32::from);
        let mut centre = None;
        if let Some(seconds) = self.order66.countdown(now)
            && self.order66.on
            && let Some(order) = self.order66.order
        {
            let name = crate::player_identity::shown_name(game_state, usize::from(order.sender))
                .unwrap_or_else(|| "Someone".to_owned());
            self.order66.cues.push(Cue {
                path: TICK_SOUND,
                source: own_slot,
            });
            centre = Some(format!(
                "^1EXECUTE ORDER 66\n^7{name}^7 gave the order\n^3{seconds}"
            ));
        }
        // The moment: the own `kill`, unless the player is spared or the sender put
        // their toy away.
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        let mut kill = false;
        if let Some(sender) = self.order66.due(now)
            && self.order66.on
        {
            let called_off = Some(sender) != own && !self.looks.toy(usize::from(sender), TOY);
            match spared(&snapshot.player, gametype) {
                _ if called_off => {}
                Some(spared) => {
                    if let Some(words) = spared.words() {
                        centre = Some(format!("^1Order 66^7\nSpared: {words}"));
                    }
                }
                None => kill = true,
            }
        }
        // While it counts down, the holders near the camera, whose pieces get ready.
        let mut ready = 0_u64;
        if self.order66.order.is_some_and(|order| now < order.at) {
            for slot in 0..SLOTS {
                let holds = if Some(slot) == own.map(usize::from) {
                    self.order66.on
                } else {
                    self.looks.toy(slot, TOY)
                };
                let near = self
                    .live_world
                    .entity(EntityId::new(slot as u64 + 1))
                    .is_some_and(|entity| {
                        Vec3::from_array(entity.sample(i64::from(now)).translation)
                            .distance(self.camera_position)
                            <= PIECE_REACH
                    });
                if holds && near {
                    ready |= 1 << slot;
                }
            }
        }
        if kill
            && let Some(session) = self.live_session.as_mut()
            && let Err(error) = session.send_reliable_command(b"kill")
        {
            crate::log::progress(format_args!("order 66: kill not sent: {error}"));
        }
        if let Some(text) = centre {
            self.order66_centre(text);
        }
        self.order66_ready(ready);
        self.order66_deaths();
    }

    /// Put one more limb mesh in the pool this frame for the first of the holders
    /// `ready` (bits) whose model has fewer than a player's pieces there, so the blast
    /// needs no upload; nothing without dismemberment (confetti, no pieces).
    pub(crate) fn order66_ready(&mut self, ready: u64) {
        if ready == 0 || self.order66_confetti() {
            return;
        }
        for slot in (0..SLOTS).filter(|slot| ready & (1 << slot) != 0) {
            let owner = EntityId::new(slot as u64 + 1);
            if self.pooled_limb_meshes(owner) < PIECES_PER_PLAYER
                && self.pool_limb_meshes(owner, 1) > 0
            {
                return;
            }
        }
    }

    /// Whether blasts burst into confetti, not pieces: dismemberment is off.
    fn order66_confetti(&self) -> bool {
        self.console
            .as_ref()
            .and_then(|console| console.integer_cvar(crate::dismember::CVAR))
            .unwrap_or(2)
            == 0
    }

    /// The players the latest snapshot shows (bits by slot) and those of them dead, at
    /// its server time: the entities, and the player the view shows (the own, or the one
    /// followed), who is in the player state instead.
    pub(crate) fn order66_seen_dead(&self) -> Option<(u64, u64, i32)> {
        let session = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())?;
        let snapshot = session.latest_snapshot();
        let (mut seen, mut dead) = (0_u64, 0_u64);
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot < SLOTS && entity.entity_type() == ET_PLAYER {
                seen |= 1 << slot;
                if entity.e_flags() & EF_DEAD != 0 {
                    dead |= 1 << slot;
                }
            }
        }
        let player = &snapshot.player;
        let viewed = usize::from(player.client_num());
        if viewed < SLOTS && !(player.is_spectator() && player.movement_flags() & PMF_FOLLOW == 0) {
            seen |= 1 << viewed;
            if player.entity_flags() & EF_DEAD != 0 || player.health() <= 0 {
                dead |= 1 << viewed;
            }
        }
        Some((seen, dead, snapshot.server_time))
    }

    /// The holders of the toy who died in the order's window blow up.
    fn order66_deaths(&mut self) {
        let Some((seen, dead, now)) = self.order66_seen_dead() else {
            return;
        };
        let own = self
            .live_session
            .as_ref()
            .and_then(|session| usize::try_from(session.game_state().client_num).ok());
        let looks = &self.looks;
        let on = self.order66.on;
        let holds = |slot: usize| {
            if Some(slot) == own {
                on
            } else {
                looks.toy(slot, TOY)
            }
        };
        let blown = self.order66.deaths(seen, dead, now, holds);
        if let Some(own) = own
            && blown & !(1 << own) != 0
            && self.order66.given_by(own)
        {
            self.order66_victims(blown & !(1 << own), own);
        }
        self.order66_blow_up(blown);
    }

    /// The player's own order brought down `victims` (bits by slot), the player being
    /// in slot `own`: Wipe Them Out counts the most of one order, I Am the Senate the
    /// players by name.
    fn order66_victims(&self, victims: u64, own: usize) {
        let Some(session) = self.live_session.as_ref().filter(|_| self.toys_count()) else {
            return;
        };
        let all = self.order66.blasted & !(1 << own);
        crate::achievements::note_best(
            crate::achievements::Counter::Order66Best,
            u64::from(all.count_ones()),
        );
        for slot in (0..SLOTS).filter(|slot| victims & 1 << slot != 0) {
            if let Some(name) = crate::player_identity::shown_name(session.game_state(), slot) {
                crate::achievements::note_name(crate::achievements::Counter::Order66Victims, &name);
            }
        }
    }

    /// Blow the players in `blown` (bits by slot) up as an order does, where the latest
    /// snapshot has them: the explosion, and the pieces or the confetti.
    pub(crate) fn order66_blow_up(&mut self, blown: u64) {
        if blown == 0 {
            return;
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let now = snapshot.server_time;
        let player = &snapshot.player;
        let viewed = usize::from(player.client_num());
        let presentation = i64::from(now);
        // Who blew up, where they stand and their actor's place.
        let mut victims = Vec::with_capacity(blown.count_ones() as usize);
        for slot in (0..SLOTS).filter(|slot| blown & (1 << slot) != 0) {
            let transform = self
                .live_world
                .entity(EntityId::new(slot as u64 + 1))
                .map(|entity| entity.sample(presentation));
            let origin = if slot == viewed {
                Vec3::from_array(player.origin())
            } else if let Some(transform) = transform {
                Vec3::from_array(transform.translation)
            } else {
                continue;
            };
            victims.push((slot, origin, transform));
        }
        let confetti = self.order66_confetti();
        let mut uploads = UPLOADS_PER_FRAME;
        for (slot, origin, transform) in victims {
            let seed = (now as u32) ^ (slot as u32).wrapping_mul(0x2545_f491);
            let owner = EntityId::new(slot as u64 + 1);
            self.order66_blast(owner, origin, transform, seed, confetti, &mut uploads);
        }
    }

    /// Blow the player whose actor is `owner` up where they stand (`origin`; their
    /// actor's place `transform`): the explosion, then the body apart with its pieces
    /// thrown (near the camera and within [`PIECES_MAX`], uploading while `uploads`
    /// allows) or, with `confetti`, hidden in a burst of confetti; `seed` chooses.
    fn order66_blast(
        &mut self,
        owner: EntityId,
        origin: Vec3,
        transform: Option<sjk_runtime::Transform>,
        seed: u32,
        confetti: bool,
        uploads: &mut usize,
    ) {
        self.order66.blasts.push(Blast {
            origin,
            confetti,
            seed,
        });
        let near = origin.distance(self.camera_position) <= PIECE_REACH
            && self.order66.flights.len() + PIECES_PER_PLAYER <= PIECES_MAX;
        let mut next = self.order66.reserve(PIECES_PER_PLAYER);
        let pieces = self.blow_apart(
            owner,
            near && !confetti,
            || {
                let id = EntityId::new(next);
                next += 1;
                id
            },
            uploads,
        );
        let Some(transform) = transform else {
            for piece in &pieces {
                self.free_limb_mesh(piece.id);
            }
            return;
        };
        let base = crate::weapon_view::actor_world_rotation(transform.rotation);
        let scale = Vec3::from_array(transform.scale);
        let centre = Vec3::from_array(transform.translation);
        for (index, piece) in pieces.iter().enumerate() {
            let start = centre + base * (Vec3::from_array(piece.pivot) * scale);
            let (velocity, axis, spin) = throw(start, centre, piece.head, seed, index as u32);
            self.order66.flights.push(Flight {
                id: piece.id,
                position: start,
                velocity,
                base,
                axis,
                spin,
                angle: 0.0,
                scale: transform.scale,
                age: 0.0,
                resting: false,
                smoke: between(seed, 100 + index as u32, 0.0, SMOKE_EVERY),
            });
        }
    }

    /// Move every piece `dt` seconds on through the world's walls and floors; the
    /// origins of the puffs of smoke due, with a salt each.
    fn order66_step(&mut self, dt: f32, smoke: &mut Vec<(Vec3, u32)>) {
        let bounds = Aabb::new([-PIECE_HALF; 3], [PIECE_HALF; 3]).unwrap_or(Aabb::POINT);
        let (bsp, scratch) = (&self.bsp, &mut self.trace_scratch);
        for flight in &mut self.order66.flights {
            flight.step(dt, |from, to| {
                let trace = bsp.trace_box_with(
                    scratch,
                    from.to_array(),
                    to.to_array(),
                    bounds,
                    CONTENTS_SOLID,
                );
                if trace.start_solid {
                    return Some((from, Vec3::Z));
                }
                let plane = trace.plane.filter(|_| trace.fraction < 1.0)?;
                Some((
                    Vec3::from_array(trace.end_position),
                    Vec3::from_array(plane.normal),
                ))
            });
            if flight.smoke_due(dt) {
                smoke.push((flight.position, flight.id.get() as u32));
            }
        }
    }

    /// Move the flying pieces on and add them to this frame's actors; free the ones
    /// gone. Without a game they all go.
    pub(crate) fn submit_order66(&mut self, presentation_time: i64, now: Instant) {
        let dt = self
            .order66
            .moved
            .replace(now)
            .map_or(0.0, |then| {
                now.saturating_duration_since(then).as_secs_f32()
            })
            .min(0.1);
        #[cfg(test)]
        let dt = if self.order66.shot_frozen { 0.0 } else { dt };
        if self.order66.flights.is_empty() {
            return;
        }
        if self.live_session.is_none() && self.demo_session.is_none() {
            for flight in std::mem::take(&mut self.order66.flights) {
                self.free_limb_mesh(flight.id);
            }
            return;
        }
        let mut smoke = std::mem::take(&mut self.order66.smoke);
        smoke.clear();
        self.order66_step(dt, &mut smoke);
        for flight in self.order66.flights.iter().filter(|flight| !flight.gone()) {
            let found = self
                .actor_meshes
                .iter()
                .enumerate()
                .find_map(|(index, mesh)| {
                    mesh.limb
                        .as_ref()
                        .filter(|_| mesh.entity_id == Some(flight.id))
                        .map(|limb| (index, limb.pivot))
                });
            let Some((index, pivot)) = found else {
                continue;
            };
            let (position, rotation) = flight.placement(pivot);
            self.actor_groups[index].push(ActorInstance::new(
                position.to_array(),
                rotation.to_array(),
                flight.scale,
            ));
        }
        let mut index = 0;
        while index < self.order66.flights.len() {
            if self.order66.flights[index].gone() {
                let id = self.order66.flights.swap_remove(index).id;
                self.free_limb_mesh(id);
            } else {
                index += 1;
            }
        }
        if let Some(vfs) = self.vfs.clone() {
            for &(origin, salt) in &smoke {
                crate::effect_runtime::spawn_effect(
                    &mut self.particles,
                    &mut self.effect_aux,
                    &mut self.effects,
                    &vfs,
                    SMOKE,
                    origin,
                    now,
                    (presentation_time as u32).rotate_left(5) ^ salt,
                    0,
                    &mut None,
                    crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]),
                );
            }
        }
        self.order66.smoke = smoke;
    }

    /// Play the order's sounds and set off the explosions (their sound with them); the
    /// sounds are handed to the audio the first time there is one.
    pub(crate) fn play_order66_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        if let Some(vfs) = self.vfs.clone() {
            for blast in std::mem::take(&mut self.order66.blasts) {
                let up = crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]);
                let now = Instant::now();
                crate::effect_runtime::spawn_effect(
                    &mut self.particles,
                    &mut self.effect_aux,
                    &mut self.effects,
                    &vfs,
                    EXPLOSION,
                    blast.origin,
                    now,
                    blast.seed,
                    0,
                    audio,
                    up,
                );
                if blast.confetti {
                    crate::effect_runtime::spawn_effect(
                        &mut self.particles,
                        &mut self.effect_aux,
                        &mut self.effects,
                        &vfs,
                        CONFETTI,
                        blast.origin,
                        now,
                        blast.seed.rotate_left(11),
                        0,
                        audio,
                        up,
                    );
                }
            }
        } else {
            self.order66.blasts.clear();
        }
        let Some(audio) = audio.as_mut() else {
            self.order66.cues.clear();
            return;
        };
        if self.order66.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [ORDER_SOUND, TICK_SOUND]);
            self.order66.sounds_registered = Some(audio.generation());
        }
        for cue in self.order66.cues.drain(..) {
            audio.play_toy(
                crate::toy_sounds::Toy::Order66,
                cue.path,
                1.0,
                None,
                sjk_audio::SourceId(cue.source),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: blow the player in `slot` of the demo playing apart as an
    /// order would, the body kept hidden though the demo has them alive (not given the
    /// limbs back); the explosion left out without `explosion`. How many pieces fly.
    pub(crate) fn shot_order66_blast(
        &mut self,
        slot: u16,
        confetti: bool,
        explosion: bool,
    ) -> usize {
        let owner = EntityId::new(u64::from(slot) + 1);
        let session = self.demo_session.as_ref().expect("a demo playing");
        let time = i64::from(session.latest_snapshot().server_time);
        let transform = session
            .world()
            .entity(owner)
            .map(|entity| entity.sample(time));
        let origin = Vec3::from_array(transform.expect("the player").translation);
        let mut uploads = PIECES_PER_PLAYER;
        self.order66_blast(
            owner,
            origin,
            transform,
            0x5eed_0066,
            confetti,
            &mut uploads,
        );
        self.order66.shot_hidden = Some(owner);
        if !explosion {
            self.order66.blasts.retain(|blast| blast.confetti);
        }
        self.order66.moved = None;
        self.order66.flights.len()
    }

    /// For the world shots: move the pieces `seconds` on, in steps of a 120th.
    pub(crate) fn shot_order66_advance(&mut self, seconds: f32) {
        let mut smoke = Vec::new();
        for _ in 0..(seconds * 120.0).round() as usize {
            self.order66_step(1.0 / 120.0, &mut smoke);
        }
        self.order66.moved = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_files_mount_and_the_confetti_is_a_burst_of_six_colours() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        mount(&mut vfs).unwrap();
        for (path, _) in FILES {
            assert!(vfs.read(path).unwrap().is_some(), "{path}");
        }
        for icon in [ICON, EXECUTE_ICON] {
            let bytes = vfs.read(icon).unwrap().unwrap().bytes;
            let picture = image::load_from_memory(&bytes).unwrap();
            assert_eq!((picture.width(), picture.height()), (128, 128));
        }
        let effect = sjk_effect::load_effect(&vfs, CONFETTI).unwrap();
        assert_eq!(effect.components.len(), 1);
        let confetti = &effect.components[0];
        assert!(confetti.count.minimum >= 100.0, "a burst");
        assert_eq!(confetti.shaders.len(), 6);
        let script = vfs
            .read("shaders/sjk_order66.shader")
            .unwrap()
            .unwrap()
            .bytes;
        let shaders =
            sjk_shader::parse_shader_script(&script, "shaders/sjk_order66.shader").unwrap();
        for name in &confetti.shaders {
            assert!(
                shaders
                    .iter()
                    .any(|shader| shader.name.eq_ignore_ascii_case(name)),
                "{name}"
            );
            assert!(
                vfs.read(&format!("{name}.png")).unwrap().is_some(),
                "{name}"
            );
        }
        assert!(crate::effect_assets::STOCK_EFFECTS.contains(&CONFETTI));
    }

    #[test]
    fn an_order_event_names_its_moment() {
        assert_eq!(order_moment("execute_123456"), Some(123_456));
        assert_eq!(order_moment("execute_"), None);
        assert_eq!(order_moment("spotted"), None);
        let event = format!("{EVENT_PREFIX}{}", i32::MAX);
        assert!(
            event.len() <= 32,
            "the hub takes ids of 32 characters at most"
        );
        assert!(crate::looks::TOYS.contains(&TOY));
    }

    #[test]
    fn orders_too_late_too_far_ahead_or_within_the_cooldown_are_dropped() {
        let mut toy = Order66::default();
        assert!(!toy.hear(1_000, 3, 2_000), "heard after it fell due");
        assert!(!toy.hear(20_000, 3, 2_000), "too far ahead");
        assert!(toy.hear(5_000, 3, 2_000));
        assert!(!toy.hear(5_000, 3, 2_100), "a repeat");
        assert!(!toy.hear(20_000, 4, 17_000), "within the cooldown");
        assert!(toy.hear(5_000 + COOLDOWN_MS, 4, 5_000 + COOLDOWN_MS - 3_000));
        // A new map: the clock went back and the orders are forgotten.
        toy.rewind(100);
        assert!(toy.hear(3_000, 5, 100));
    }

    #[test]
    fn giving_the_order_waits_out_the_cooldown() {
        let mut toy = Order66::default();
        assert_eq!(toy.give(2, 10_000), Ok(13_000));
        assert_eq!(toy.order.map(|order| order.sender), Some(2));
        // 3 s after it fell due another may not come for 27 s.
        assert_eq!(toy.give(2, 16_000), Err(24));
        assert_eq!(toy.give(2, 40_000), Ok(43_000));
    }

    #[test]
    fn the_countdown_shows_each_second_once_then_the_moment_comes_once() {
        let mut toy = Order66::default();
        toy.give(1, 0).unwrap();
        assert_eq!(toy.countdown(0), Some(3));
        assert_eq!(toy.countdown(500), None);
        assert_eq!(toy.countdown(1_000), Some(2));
        assert_eq!(toy.countdown(2_999), Some(1));
        assert_eq!(toy.due(2_999), None);
        assert_eq!(toy.countdown(3_000), None);
        assert_eq!(toy.due(3_016), Some(1));
        assert_eq!(toy.due(3_032), None);
    }

    #[test]
    fn holders_newly_dead_in_the_window_blow_up_once() {
        let mut toy = Order66::default();
        let holds = |slot: usize| slot != 2;
        toy.give(0, 0).unwrap();
        // Before the window, a death is an ordinary one.
        assert_eq!(toy.deaths(0b1111, 0b1000, 1_000, holds), 0);
        // In it: slot 1 blows up; slot 2 has no toy; slot 3 was already dead.
        assert_eq!(toy.deaths(0b1111, 0b1110, 3_050, holds), 0b0010);
        assert_eq!(toy.deaths(0b1111, 0b1110, 3_100, holds), 0, "once");
        // Slot 1 respawns and dies again within the window: still once.
        toy.deaths(0b1111, 0b1100, 3_200, holds);
        assert_eq!(toy.deaths(0b1111, 0b1110, 3_300, holds), 0);
        // After the window, nothing.
        toy.deaths(0b1111, 0, 3_400, holds);
        assert_eq!(
            toy.deaths(0b1111, 0b0001, 3_000 + BLAST_WINDOW_MS + 1, holds),
            0
        );
    }

    #[test]
    fn a_piece_falls_bounces_slower_and_comes_to_rest_then_sinks_away() {
        let mut flight = Flight {
            id: EntityId::new(PIECE_ID_BASE),
            position: Vec3::new(0.0, 0.0, 40.0),
            velocity: Vec3::new(100.0, 0.0, 300.0),
            base: Quat::IDENTITY,
            axis: Vec3::X,
            spin: 10.0,
            angle: 0.0,
            scale: [1.0; 3],
            age: 0.0,
            resting: false,
            smoke: 0.0,
        };
        // The floor at height 0.
        let floor = |from: Vec3, to: Vec3| {
            (to.z < 0.0).then(|| {
                let t = from.z / (from.z - to.z);
                (from + (to - from) * t, Vec3::Z)
            })
        };
        let mut highest = 0.0_f32;
        let mut bounces = 0;
        for _ in 0..600 {
            let falling = flight.velocity.z < 0.0;
            flight.step(1.0 / 120.0, floor);
            if falling && flight.velocity.z > 0.0 {
                bounces += 1;
            }
            highest = highest.max(flight.position.z);
            assert!(flight.position.z >= 0.0);
        }
        assert!(highest > 80.0, "thrown up first: {highest}");
        assert!(bounces >= 1);
        assert!(flight.resting, "at rest after five seconds");
        let (lying, _) = flight.placement([0.0; 3]);
        flight.age = LIFE + SINK * 0.5;
        let (sinking, _) = flight.placement([0.0; 3]);
        assert!(sinking.z < lying.z);
        assert!(!flight.gone());
        flight.age = LIFE + SINK;
        assert!(flight.gone());
    }

    #[test]
    fn pieces_fly_out_from_the_body_and_up_the_head_highest_on_average() {
        let centre = Vec3::ZERO;
        let (velocity, axis, _) = throw(Vec3::new(10.0, 0.0, 0.0), centre, false, 7, 1);
        assert!(velocity.x > 0.0 && velocity.z > 200.0, "{velocity}");
        assert!((axis.length() - 1.0).abs() < 1e-4);
        let up = |head: bool| -> f32 {
            (0..64)
                .map(|salt| throw(Vec3::Z * 30.0, centre, head, 99, salt).0.z)
                .sum::<f32>()
        };
        assert!(up(true) > up(false));
        // A piece right over the middle still flies off somewhere.
        let (velocity, _, _) = throw(Vec3::Z * 30.0, centre, false, 3, 2);
        assert!(velocity.truncate().length() > 100.0);
    }

    #[test]
    fn piece_ids_are_never_entity_numbers() {
        let mut toy = Order66::default();
        let first = toy.reserve(PIECES_PER_PLAYER);
        assert!(is_piece(EntityId::new(first)));
        assert!(!is_piece(EntityId::new(1_024)));
        assert_eq!(toy.reserve(1), first + PIECES_PER_PLAYER as u64);
        toy.next_piece = u64::MAX;
        toy.reserve(2);
        assert!(toy.next_piece >= PIECE_ID_BASE);
    }
}
