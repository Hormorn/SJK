//! The Ragdoll, SJK's meme toy (`toy_ragdoll`; Sol's idea, 23/10/2026): the player's
//! body goes limp and falls where physics takes it, as in the sandbox games, though the
//! player is not dead; `toy_ragdoll` again gets them up.
//!
//! No game server knows of it. The body is the client's: a simulated humanoid
//! ([`body`]) drawn with the player's own model ([`skin`]), the camera following it
//! from behind (the mouse turns it). Every SJK client on the server shows the same fall
//! without any pose being sent: the faller's client relays one start event to every SJK
//! player there (`fall_<moment>_<origin>_<speed>_<yaw>`, the toy relay, `PROTOCOL.md`
//! "Toy relay"), and each runs the same simulation from it in fixed steps of the game
//! server's clock, fast-forwarding a fall it hears late. Getting up relays `up_<moment>`.
//! A player who comes onto the server (or whose SJK client starts) during a fall is told
//! of it too.
//!
//! The player's real body stays on the game server: while limp, the client walks it to
//! where the ragdoll lies (its own user commands, as the Hax's steering), so that the
//! player gets up there. Every SJK client draws the ragdoll in its place; players on
//! stock clients see a player walking about. A limp player sends no button, so cannot
//! attack, and gets up at once when hurt, dead, spectating, in a duel or carrying a flag
//! ([`crate::order66::spared`]), or when the server holds their view (a kiss, a grapple,
//! a kick: [`crate::held_view`]); nobody falls in one of those either.
//!
//! The fall is heard: an original flatline, the toy's own ([`FLATLINE`], made by
//! `scripts/ragdoll_assets.py`; a PK3 with the same path replaces it), at the body, and
//! every part's knocks and scrapes on the world, flesh and the surface's material
//! ([`sounds`]). Lifts, doors and trains carry and push the body ([`scene`]).
//!
//! A body can be struck (the Monitor toy, [`crate::monitor`], 26/10/2026): the struck
//! player's client gives its body a [`body::Strike`] (falling first if standing) and
//! relays it after the fall, `hit_<fall moment>_<step>_<joint>_<push>_<by>`, `by` naming
//! the monitor (its thrower's slot, its number and its step, so that every client bounces
//! it off the body alike). A client that hears of a blow after its step ran the body on
//! runs it again from its start with it ([`body::Body::rerun`]). A fall a monitor began
//! ends by itself once the body has lain still a while ([`REST_UP`]). A slip on a banana
//! peel ([`crate::banana`], 29/10/2026) is such a blow too, at a foot, naming the peel's
//! dropper and no monitor ([`UNTHROWN`]).

mod body;
mod scene;
mod skin;
mod sounds;

use crate::GpuState;
use crate::ghost_replay::Drive;
use body::PELVIS;
pub(crate) use body::{
    Body, JOINTS, L_FOOT, MAX_PUSH, R_FOOT, STEP_MS, STOCK_SCALE, Start, Strike, World, friction,
    scale_percent, unbounced, unit,
};
use glam::Vec3;
pub(crate) use scene::{Movers, Scene};
use sjk_protocol::UserCommand;
use sjk_runtime::EntityId;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

pub(crate) use skin::Scratch;

/// The toy's console command: fall, or get up.
pub(crate) const COMMAND: &str = "toy_ragdoll";
/// The toy's id in relayed toy events.
pub(crate) const TOY: &str = "ragdoll";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_ragdoll.png";
/// Heard as the body goes limp.
pub(crate) const FLATLINE: &str = "sound/sjk/ragdoll/flatline.wav";
/// A player's flatline is heard at most once in this long; the fall itself is not held.
const FLATLINE_GAP: Duration = Duration::from_secs(15);
/// The start event: this, then the start ([`body::encode`]); the end: this, then the
/// fall's moment.
const FALL_PREFIX: &str = "fall_";
const UP_PREFIX: &str = "up_";
/// A blow to a fallen body: this, then [`Hit::text`].
const HIT_PREFIX: &str = "hit_";
/// A fall takes this many blows at most; later ones pass it by.
const MAX_HITS: usize = 24;
/// A fall a monitor began ends this long after the body came to rest, once the real
/// body has walked to it (or [`REST_WAIT`] later whatever it did).
const REST_UP: Duration = Duration::from_millis(2_500);
const REST_WAIT: Duration = Duration::from_secs(6);
/// A limp player gets up on a new jump, attack or taunt this long after falling: the
/// fall shows a moment first, and a button held as they fell does not end it.
const WAKE_AFTER: Duration = Duration::from_millis(500);
/// `BUTTON_ATTACK`, `BUTTON_GESTURE` (`+button3`) and `BUTTON_ALT_ATTACK`.
const WAKE_BUTTONS: u16 = 1 | 8 | 128;
/// The `generic_cmd` taunts: `taunt`, `bow`, `meditate`, `flourish`, `gloat`.
const WAKE_TAUNTS: std::ops::RangeInclusive<u8> = 27..=31;
/// Between two start events sent (the relay takes 40 a second from a player).
const TELL_GAP: Duration = Duration::from_millis(35);
/// A blow this client foresaw on another player ([`GpuState::ragdoll_foresee`]) waits
/// this long for their own word of it, then is undone.
pub(crate) const GUESS_WAIT: Duration = Duration::from_millis(1_200);
/// A fall heard this far after its moment, or before it, is dropped (ms).
const LATE_MS: i64 = 600_000;
const EARLY_MS: i64 = 5_000;
/// A fallen player whose real body is further than this from the ragdoll for
/// [`ASTRAY`] without coming closer (by [`ASTRAY_NEARER`]: it walks to a body thrown
/// far) got up without the news reaching here: their ragdoll goes.
const ASTRAY_REACH: f32 = 320.0;
const ASTRAY: Duration = Duration::from_secs(3);
const ASTRAY_NEARER: f32 = 24.0;
/// The limp player's real body walks to the ragdoll: it stops this close, and walks
/// (rather than runs) this close.
const WALK_STOP: f32 = 12.0;
const WALK_NEAR: f32 = 72.0;
/// `BUTTON_WALKING`.
const BUTTON_WALKING: u16 = 16;
/// The camera: above the pelvis, and how far behind.
const CAMERA_HEIGHT: f32 = 12.0;
const CAMERA_RANGE: f32 = 110.0;
const CAMERA_RISE: f32 = 10.0;
const SLOTS: usize = crate::looks::SLOTS;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
/// `EF_TELEPORT_BIT`, flipped at every teleport and respawn.
const EF_TELEPORT_BIT: u32 = 1 << 3;
/// `EF_NODRAW`.
const EF_NODRAW: u32 = 1 << 8;
/// `PMF_FOLLOW`.
const PMF_FOLLOW: u16 = 4096;

/// A bundled file's bytes, by its name in `assets/ragdoll`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/ragdoll/", $name))
    };
}

/// The bundled files at their game paths (made by `scripts/ragdoll_assets.py`).
const FILES: [(&str, &[u8]); 2] = [
    (ICON, bundled!("toy_ragdoll.png")),
    (FLATLINE, bundled!("flatline.wav")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Ragdoll", FILES.iter().copied())?;
    Ok(())
}

/// What struck a body: a monitor, by its thrower's slot, its number among theirs and
/// the step of its flight it struck at; or a banana peel the body slipped on
/// ([`crate::banana`]), by its dropper's slot, its number [`UNTHROWN`] and step 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct By {
    pub(crate) thrower: u8,
    pub(crate) number: u8,
    pub(crate) step: u16,
}

/// [`By::number`] of a blow no monitor dealt: a slip on a banana peel. Past every
/// monitor's number, so no client bounces a monitor off the body for it.
pub(crate) const UNTHROWN: u8 = u8::MAX;

/// A blow to the fall that began at `fall` (its moment), and what dealt it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Hit {
    pub(crate) fall: i32,
    pub(crate) strike: Strike,
    pub(crate) by: By,
}

impl Hit {
    /// The event's text after its prefix, whole numbers parted by `_`.
    fn text(&self) -> String {
        let Strike { step, joint, push } = self.strike;
        let [x, y, z] = push;
        let By {
            thrower,
            number,
            step: flight,
        } = self.by;
        format!(
            "{}_{step}_{joint}_{x}_{y}_{z}_{thrower}_{number}_{flight}",
            self.fall
        )
    }

    /// The hit [`Hit::text`] wrote; `None` for anything else or out of bounds.
    fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('_');
        let mut number = || parts.next()?.parse::<i32>().ok();
        let fall = number()?;
        let step = u32::try_from(number()?)
            .ok()
            .filter(|step| *step < 1 << 22)?;
        let joint = u8::try_from(number()?)
            .ok()
            .filter(|joint| usize::from(*joint) < JOINTS)?;
        let push = [number()?, number()?, number()?];
        let thrower = u8::try_from(number()?)
            .ok()
            .filter(|slot| usize::from(*slot) < SLOTS)?;
        let flight_number = u8::try_from(number()?).ok()?;
        let flight = u16::try_from(number()?).ok()?;
        if parts.next().is_some() || push.iter().any(|c| c.abs() > MAX_PUSH) {
            return None;
        }
        Some(Self {
            fall,
            strike: Strike { step, joint, push },
            by: By {
                thrower,
                number: flight_number,
                step: flight,
            },
        })
    }
}

/// A relayed event of the toy, as read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Fall(Start),
    Up(i32),
    Hit(Hit),
}

impl Event {
    fn parse(event: &str) -> Option<Self> {
        if let Some(start) = event.strip_prefix(FALL_PREFIX) {
            return body::decode(start).map(Self::Fall);
        }
        if let Some(hit) = event.strip_prefix(HIT_PREFIX) {
            return Hit::parse(hit).map(Self::Hit);
        }
        event.strip_prefix(UP_PREFIX)?.parse().ok().map(Self::Up)
    }

    fn text(&self) -> String {
        match self {
            Self::Fall(start) => format!("{FALL_PREFIX}{}", body::encode(start)),
            Self::Up(moment) => format!("{UP_PREFIX}{moment}"),
            Self::Hit(hit) => format!("{HIT_PREFIX}{}", hit.text()),
        }
    }
}

/// One body falling or lying, the player's own or another's.
struct Fall {
    start: Start,
    body: Body,
    /// When its parts' knocks were last heard.
    heard: sounds::Heard,
    /// The `EF_TELEPORT_BIT` the faller had (another's): a flip is a respawn.
    teleport: Option<u32>,
    /// Since when their real body has been far from the ragdoll without coming closer,
    /// and the nearest it came in that time (another's).
    astray: Option<(Instant, f32)>,
}

impl Fall {
    fn new(start: Start) -> Self {
        Self {
            body: Body::new(&start),
            start,
            heard: sounds::Heard::default(),
            teleport: None,
            astray: None,
        }
    }

    /// The body takes `strike`, run again from its start when its step went by.
    fn strike(&mut self, strike: Strike) {
        if self.body.has_strike(&strike) || self.body.strike_count() >= MAX_HITS {
            return;
        }
        if !self.body.strike(strike) {
            self.body = self.body.rerun(&self.start, strike);
        }
    }
}

/// A blow this client foresaw on another player's body, waiting for their own word:
/// the fall it struck (its moment), the blow, when it landed (game server
/// milliseconds), whether the fall itself was foreseen (they stood), and until when it
/// waits.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Guess {
    moment: i32,
    strike: Strike,
    at: i64,
    new_fall: bool,
    until: Instant,
}

/// The step of a fall begun at `moment` that `time` falls in (game server
/// milliseconds), 0 before it.
fn step_of(moment: i32, time: i64) -> u32 {
    u32::try_from((time - i64::from(moment)).max(0) / i64::from(STEP_MS)).unwrap_or(u32::MAX)
}

/// A monitor struck the body of the player in slot `struck`: every client bounces it
/// off at `by`'s step, `push` being the body's (`crate::monitor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounce {
    pub(crate) by: By,
    pub(crate) push: [i32; 3],
    pub(crate) struck: u8,
}

/// The player's own fall: the moment, how many of its events each slot has been told
/// (the fall, then its hits in turn), when the next may be, the damage feedback it began
/// with, and the camera's angles to give back.
struct Mine {
    moment: i32,
    told: [u8; SLOTS],
    tell_at: Instant,
    /// The blows the body took, told after the fall.
    hits: Vec<Hit>,
    /// A monitor began it: it ends by itself once the body has lain still a while
    /// ([`REST_UP`]); since when it has.
    auto: bool,
    rest: Option<Instant>,
    /// The player state's `damageEvent` at the fall (`P_DamageFeedback` changes it at
    /// every hurt): a new one wakes the player. Health was compared before, and woke a
    /// player who fell again after a hit without being hit (Sanji's report #135).
    damage_event: u8,
    camera: (f32, f32),
    /// When it began: a press to get up counts only [`WAKE_AFTER`] on.
    fell: Instant,
}

/// The toy's state.
#[derive(Default)]
pub(crate) struct Ragdoll {
    server: Option<SocketAddr>,
    /// `toy_ragdoll` was pressed since the last frame.
    toggled: bool,
    /// A jump, an attack or a taunt was pressed since the last frame, while limp
    /// ([`Ragdoll::wake`]); and whether one was held at the last command.
    woken: bool,
    wake_held: bool,
    falls: Vec<Option<Fall>>,
    mine: Option<Mine>,
    /// The player's own slot, and where its real body is to walk to.
    own: Option<u8>,
    walk_to: Option<Vec3>,
    /// The sounds to play.
    cues: Vec<Cue>,
    /// When each slot's flatline was last heard ([`FLATLINE_GAP`]).
    flatline_heard: std::collections::HashMap<u8, Instant>,
    /// The player may not fall now (dead, in a duel, with the flag, held, hacked), as the
    /// last frame found.
    spared: bool,
    /// The movers in the bodies' way.
    movers: scene::Movers,
    /// The monitors that struck a body since the Monitor toy last took them.
    bounces: Vec<Bounce>,
    /// The blows this client foresaw on other players, by slot, until their word comes.
    guesses: std::collections::HashMap<u8, Guess>,
    sounds_registered: Option<u64>,
    /// The game server time last stepped to.
    time: i64,
    /// For the world shots: what is shown is left as a shot set it.
    #[cfg(test)]
    shot_frozen: bool,
}

/// Each slot's ragdoll as shown this frame (world space), for the actors' posing and
/// placing, which run without the viewer's state.
static SHOWN: LazyLock<Mutex<[Option<[Vec3; JOINTS]>; SLOTS]>> =
    LazyLock::new(|| Mutex::new([None; SLOTS]));

fn shown_lock() -> std::sync::MutexGuard<'static, [Option<[Vec3; JOINTS]>; SLOTS]> {
    SHOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The ragdoll of the player in `slot` this frame, world space, if they are limp, or
/// floating in a Toy Master's show ([`set_floating`]).
pub(crate) fn shown(slot: u8) -> Option<[Vec3; JOINTS]> {
    shown_lock()
        .get(usize::from(slot))
        .copied()
        .flatten()
        .or_else(|| floating_lock().get(usize::from(slot)).copied().flatten())
}

/// The players floating in a Toy Master's show this frame (`toy_master::float`): drawn
/// as their ragdolls are, a fall of their own coming first.
static FLOATING: LazyLock<Mutex<[Option<[Vec3; JOINTS]>; SLOTS]>> =
    LazyLock::new(|| Mutex::new([None; SLOTS]));

fn floating_lock() -> std::sync::MutexGuard<'static, [Option<[Vec3; JOINTS]>; SLOTS]> {
    FLOATING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Set the floating players of this frame, by slot.
pub(crate) fn set_floating(bodies: impl IntoIterator<Item = (u8, [Vec3; JOINTS])>) {
    let mut floating = floating_lock();
    *floating = [None; SLOTS];
    for (slot, points) in bodies {
        if let Some(entry) = floating.get_mut(usize::from(slot)) {
            *entry = Some(points);
        }
    }
}

/// The head joint's centre lies this far below the top of the head.
const HEAD_TOP: f32 = 6.0;

/// Where names over the player in `slot` hang while they are limp (their nameplate,
/// their overhead name): the ragdoll as shown this frame, its pelvis (the point names
/// are traced to) and how far above it the body's highest point is, the top of the head
/// standing, a little above the body lying.
pub(crate) fn shown_top(slot: u8) -> Option<(Vec3, f32)> {
    let points = shown(slot)?;
    Some(top_of(&points))
}

/// [`shown_top`] for `points`.
fn top_of(points: &[Vec3; JOINTS]) -> (Vec3, f32) {
    let pelvis = points[PELVIS];
    let top = points.iter().map(|point| point.z).fold(f32::MIN, f32::max) + HEAD_TOP;
    (pelvis, top - pelvis.z)
}

/// The ragdoll of the player whose actor is `entity` (`slot + 1`), if any.
pub(crate) fn shown_entity(entity: EntityId) -> Option<[Vec3; JOINTS]> {
    let slot = u8::try_from(entity.get().checked_sub(1)?).ok()?;
    shown(slot)
}

/// Pose an actor's bones for the ragdoll `points` (world space) into `scratch`: false
/// when its skeleton is not the stock humanoid's (it is then drawn as animated).
pub(crate) fn pose(
    scratch: &mut Scratch,
    gla: &std::sync::Arc<sjk_model::Gla>,
    points: &[Vec3; JOINTS],
) -> bool {
    scratch.pose(gla, &skin::model_space(points))
}

/// Where an actor posed by [`pose`] is placed: at the pelvis, unturned, keeping the
/// scale its `modelScale` gave it (the body was simulated at that scale,
/// [`Start::scale`]): a youngling or a jawa the server made small grew to full size as
/// it fell (hub report #223). The posed bones hang from the pelvis at the model's own
/// joints, so the scale shrinks the model about the pelvis the body lies on.
pub(crate) fn place(transform: &mut sjk_runtime::Transform, points: &[Vec3; JOINTS]) {
    transform.translation = points[PELVIS].to_array();
    transform.rotation = glam::Quat::IDENTITY.to_array();
}

/// The points of a body standing at `origin` (a player's origin), facing `yaw`
/// (degrees), at `scale` ([`Start::scale`]): where a blow to a standing player lands
/// ([`crate::monitor`]).
pub(crate) fn standing(origin: Vec3, yaw: f32, scale: u16) -> [Vec3; JOINTS] {
    let start = Start {
        moment: 0,
        origin: origin.to_array().map(|c| c.round() as i32),
        velocity: [0; 3],
        yaw: yaw.rem_euclid(360.0).round() as u16 % 360,
        scale,
    };
    *Body::new(&start).points()
}

/// The game server's 16-bit angle for `degrees` (`ANGLE2SHORT`).
fn short(degrees: f32) -> i32 {
    (degrees * 65_536.0 / 360.0) as i32 & 0xFFFF
}

impl Ragdoll {
    /// `toy_ragdoll`: fall, or get up (at the next frame).
    pub(crate) fn toggle(&mut self) {
        self.toggled = true;
    }

    /// Go limp (`true`) or get up, at the next frame, unless already so.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.toggled = on != self.on();
    }

    /// The player's command this frame, before [`Ragdoll::drive`] takes its buttons: a
    /// new jump, attack or taunt gets a limp player up (at the next frame), once
    /// [`WAKE_AFTER`] has passed since the fall.
    pub(crate) fn wake(&mut self, input: &UserCommand) {
        let wish = input.up_move > 0
            || input.buttons & WAKE_BUTTONS != 0
            || WAKE_TAUNTS.contains(&input.generic_command);
        let pressed = wish && !self.wake_held;
        self.wake_held = wish;
        if pressed
            && self
                .mine
                .as_ref()
                .is_some_and(|mine| mine.fell.elapsed() >= WAKE_AFTER)
        {
            self.woken = true;
        }
    }

    /// Whether the player's own body is limp.
    pub(crate) fn on(&self) -> bool {
        self.mine.is_some()
    }

    /// Whether the player can be struck down now: on a game server, and neither dead nor
    /// spectating, in a duel, with the flag, held by the server or hacked (as for going
    /// limp).
    pub(crate) fn strikable(&self) -> bool {
        self.server.is_some() && self.own.is_some() && !self.spared
    }

    /// The command to send in place of `input` while the player is limp: no button, the
    /// real body walking to the ragdoll; `delta` the player's `delta_angles` and `from`
    /// where the real body is.
    pub(crate) fn drive(
        &self,
        input: &UserCommand,
        delta: [i32; 3],
        from: [f32; 3],
    ) -> Option<Drive> {
        self.mine.as_ref()?;
        let mut command = UserCommand {
            forward_move: 0,
            right_move: 0,
            up_move: 0,
            buttons: 0,
            generic_command: 0,
            ..*input
        };
        let way = self
            .walk_to
            .map(|to| (to - Vec3::from_array(from)).truncate())
            .unwrap_or_default();
        let distance = way.length();
        let yaw = if distance > 0.5 {
            way.y.atan2(way.x).to_degrees()
        } else {
            0.0
        };
        let view = [0, short(yaw), input.angles[2].wrapping_add(delta[2])];
        command.angles = std::array::from_fn(|i| view[i].wrapping_sub(delta[i]) & 0xFFFF);
        if distance > WALK_STOP {
            command.forward_move = 127;
            if distance < WALK_NEAR {
                command.buttons = BUTTON_WALKING;
            }
        }
        Some(Drive {
            command,
            view: None,
        })
    }

    /// Shape a command that is only shown (the prediction's preview), as
    /// [`Ragdoll::drive`] would.
    pub(crate) fn shape_preview(&self, command: &mut UserCommand, delta: [i32; 3], from: [f32; 3]) {
        if let Some(drive) = self.drive(command, delta, from) {
            *command = drive.command;
        }
    }

    /// Forget every fall: no game, or another server.
    fn leave(&mut self) {
        self.server = None;
        self.falls.clear();
        self.guesses.clear();
        self.mine = None;
        self.walk_to = None;
        *shown_lock() = [None; SLOTS];
    }

    fn fall_mut(&mut self, slot: u8) -> Option<&mut Fall> {
        self.falls.get_mut(usize::from(slot))?.as_mut()
    }

    /// The monitors that struck a body since the last call (`crate::monitor`).
    pub(crate) fn take_bounces(&mut self) -> Vec<Bounce> {
        std::mem::take(&mut self.bounces)
    }

    /// The player's own body's points as they are now (its last step), if limp.
    pub(crate) fn own_points(&self) -> Option<[Vec3; JOINTS]> {
        self.mine.as_ref()?;
        let fall = self.falls.get(usize::from(self.own?))?.as_ref()?;
        Some(*fall.body.points())
    }

    /// The body of the player in `slot` as it is now (its last step), if limp.
    pub(crate) fn points_of(&self, slot: u8) -> Option<[Vec3; JOINTS]> {
        Some(*self.falls.get(usize::from(slot))?.as_ref()?.body.points())
    }

    /// Whether a blow this client foresaw on the player in `slot` still waits for their
    /// word.
    pub(crate) fn guessing(&self, slot: u8) -> bool {
        self.guesses.contains_key(&slot)
    }

    /// The player in `slot` (another) is foreseen struck at `time` at point `joint`,
    /// pushed by `push`: their body falls from `standing` if they stand, and takes the
    /// blow at once. False when it did not land (their own, one already foreseen, or a
    /// body that takes no more).
    fn foresee(
        &mut self,
        slot: u8,
        time: i64,
        joint: u8,
        push: [i32; 3],
        standing: Start,
        now: Instant,
    ) -> bool {
        if Some(slot) == self.own || usize::from(slot) >= SLOTS || self.guesses.contains_key(&slot)
        {
            return false;
        }
        let push = push.map(|c| c.clamp(-MAX_PUSH, MAX_PUSH));
        let new_fall = self.fall_mut(slot).is_none();
        if new_fall {
            self.set_fall(slot, Some(Fall::new(standing)));
            self.cues
                .push(Cue::flatline(Some(start_point(&standing)), slot));
        }
        let Some(fall) = self.fall_mut(slot) else {
            return false;
        };
        if fall.body.strike_count() >= MAX_HITS {
            return false;
        }
        let moment = fall.start.moment;
        let strike = Strike {
            step: step_of(moment, time).max(fall.body.steps()),
            joint,
            push,
        };
        fall.strike(strike);
        self.guesses.insert(
            slot,
            Guess {
                moment,
                strike,
                at: time,
                new_fall,
                until: now + GUESS_WAIT,
            },
        );
        true
    }

    /// One relayed event of the player in `slot`, heard at game server time `now`.
    fn hear(&mut self, slot: u8, event: Event, now: i64) {
        match event {
            Event::Fall(start) => {
                let moment = i64::from(start.moment);
                let known = self.fall_mut(slot).is_some_and(|fall| fall.start == start);
                if known || moment > now + EARLY_MS || moment < now - LATE_MS {
                    return;
                }
                let mut fall = Fall::new(start);
                // A fall this client foresaw: the blow it foresaw stays, on the real
                // fall, until the real blow comes.
                let guess = self.guesses.get(&slot).copied();
                if let Some(guess) = guess {
                    let strike = Strike {
                        step: step_of(start.moment, guess.at),
                        ..guess.strike
                    };
                    fall.strike(strike);
                    self.guesses.insert(
                        slot,
                        Guess {
                            moment: start.moment,
                            strike,
                            new_fall: false,
                            ..guess
                        },
                    );
                }
                self.set_fall(slot, Some(fall));
                if guess.is_none() && moment >= now - 1_000 {
                    self.cues
                        .push(Cue::flatline(Some(start_point(&start)), slot));
                }
            }
            Event::Up(moment) => {
                if self
                    .fall_mut(slot)
                    .is_some_and(|fall| fall.start.moment == moment)
                {
                    self.set_fall(slot, None);
                    self.guesses.remove(&slot);
                }
            }
            Event::Hit(hit) => {
                let guess = self
                    .guesses
                    .get(&slot)
                    .copied()
                    .filter(|guess| guess.moment == hit.fall);
                let Some(fall) = self
                    .fall_mut(slot)
                    .filter(|fall| fall.start.moment == hit.fall)
                else {
                    return;
                };
                // The real blow takes the place of the one foreseen.
                if let Some(guess) = guess.filter(|guess| guess.strike != hit.strike) {
                    fall.body = fall.body.without(&fall.start, &guess.strike);
                }
                let fresh = !fall.body.has_strike(&hit.strike);
                fall.strike(hit.strike);
                if guess.is_some() {
                    self.guesses.remove(&slot);
                }
                // The monitor bounces off it (and, foreseen, its foreseen bounce gives way).
                if fresh || guess.is_some() {
                    self.bounces.push(Bounce {
                        by: hit.by,
                        push: hit.strike.push,
                        struck: slot,
                    });
                }
            }
        }
    }

    /// Undo the blows foreseen whose word did not come by `now`: a foreseen fall ends
    /// (the player stands again), a foreseen blow on a fall leaves it.
    fn expire_guesses(&mut self, now: Instant) {
        let due: Vec<(u8, Guess)> = self
            .guesses
            .iter()
            .filter(|(_, guess)| guess.until <= now)
            .map(|(slot, guess)| (*slot, *guess))
            .collect();
        for (slot, guess) in due {
            self.guesses.remove(&slot);
            let Some(fall) = self
                .fall_mut(slot)
                .filter(|fall| fall.start.moment == guess.moment)
            else {
                continue;
            };
            if guess.new_fall {
                self.set_fall(slot, None);
            } else {
                fall.body = fall.body.without(&fall.start, &guess.strike);
            }
        }
    }

    /// The player's own fall's moment and the steps its body has taken, if limp.
    fn own_steps(&self) -> Option<(i32, u32)> {
        let mine = self.mine.as_ref()?;
        let fall = self.falls.get(usize::from(self.own?))?.as_ref()?;
        Some((mine.moment, fall.body.steps()))
    }

    fn set_fall(&mut self, slot: u8, fall: Option<Fall>) {
        let index = usize::from(slot);
        if index >= SLOTS {
            return;
        }
        if self.falls.len() <= index {
            self.falls.resize_with(index + 1, || None);
        }
        self.falls[index] = fall;
    }
}

/// The SJK player (`sjk`, bits by slot) to tell next of a fall of `events` events
/// (the fall, then its blows), each slot told `told` of them so far: a thrower of one of
/// its blows (`throwers`) first, so the player who threw sees the hit soonest; then one
/// partly told, so each hears the fall and its blows back to back; then the nearest
/// (`distances` from the body). One told of all of them before others hear a new blow
/// waits: every partly told slot comes first.
fn next_told(
    told: &[u8; SLOTS],
    events: usize,
    sjk: u64,
    throwers: u64,
    distances: &[f32; SLOTS],
) -> Option<usize> {
    (0..SLOTS)
        .filter(|slot| sjk & (1 << slot) != 0 && usize::from(told[*slot]) < events)
        .min_by(|a, b| {
            let rank = |slot: usize| (throwers & (1 << slot) == 0, told[slot] == 0);
            rank(*a)
                .cmp(&rank(*b))
                .then(distances[*a].total_cmp(&distances[*b]))
        })
}

/// Send `event` to the player in `slot`.
fn relay(event: Event, slot: u8) {
    crate::player_identity::relay_toy_event(TOY, event.text(), slot, false);
}

/// A sound of the toy to play: its path, where (`None`: the player's own, not placed),
/// whose body, and how loud.
struct Cue {
    path: &'static str,
    at: Option<Vec3>,
    slot: u8,
    volume: f32,
}

impl Cue {
    fn flatline(at: Option<Vec3>, slot: u8) -> Self {
        Self {
            path: FLATLINE,
            at,
            slot,
            volume: 1.0,
        }
    }
}

impl GpuState {
    /// `toy_ragdoll`: fall, or get up.
    pub(crate) fn toggle_ragdoll(&mut self) {
        self.ragdoll.toggle();
    }

    /// Say `text` on the centre of the screen.
    fn ragdoll_centre(&mut self, text: &str) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text.to_owned(),
            None,
            Instant::now(),
        );
    }

    /// The frame's toy logic, from the relay's events: falls heard and ended, the
    /// player's own fall begun or ended, and those to tell of it.
    pub(crate) fn update_ragdoll(&mut self, relay_events: &[sjk_identity::ToyEvent]) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            if self.ragdoll.server.is_some() {
                self.ragdoll_get_up(false);
                self.ragdoll.leave();
            }
            self.ragdoll.toggled = false;
            self.ragdoll.woken = false;
            return;
        };
        let server = session.server();
        if self.ragdoll.server != Some(server) {
            self.ragdoll.leave();
            self.ragdoll.server = Some(server);
        }
        let snapshot = session.latest_snapshot();
        let game_state = session.game_state();
        let own = u8::try_from(game_state.client_num).ok();
        self.ragdoll.own = own;
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        // A view the server holds (a kiss, a grapple, a kick) gets the player up and keeps
        // them from falling: the walk's angles fought it and the camera spun (#139, #140,
        // `held_view`). A pilot or passenger is held by the vehicle the same way: a limp
        // body could neither drive nor get out (a Monitor's hit, #168).
        let riding = snapshot.player.vehicle_entity_num() != 0;
        let spared = crate::order66::spared(&snapshot.player, gametype).or_else(|| {
            (riding || self.server_holds_view()).then_some(crate::order66::Spared::Held)
        });
        let damage_event = snapshot.player.damage_event();
        self.ragdoll.spared = spared.is_some() || self.hax.busy();
        let now = self.ragdoll.time;
        // The relay's events: a sender counts only when the game shows the name they
        // claimed, and never one blocked on this PC, nor anyone's while the player hides
        // other players' falls (Collection > Toys).
        let seen = crate::toy_sight::seen(self.console.as_ref(), crate::toy_sight::RAGDOLL);
        let heard: Vec<(u8, Event)> = relay_events
            .iter()
            .filter(|event| {
                seen && event.toy == TOY
                    && Some(event.slot) != own
                    && usize::from(event.slot) < SLOTS
                    && !self.muted_players.contains(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        for (slot, event) in heard {
            self.ragdoll.hear(slot, event, now);
        }
        self.ragdoll.expire_guesses(Instant::now());
        // The player's own fall.
        let toggled = std::mem::take(&mut self.ragdoll.toggled);
        // A press only ever gets up: it never begins a fall.
        let woken = std::mem::take(&mut self.ragdoll.woken);
        let rested = self.ragdoll_rested();
        if let Some(mine) = &self.ragdoll.mine {
            let why = if toggled || woken {
                None
            } else if spared.is_some() || self.hax.busy() || rested {
                // A hack gets a limp player up too: the hacker steers a standing body,
                // not one left on the floor (#167).
                Some("")
            } else if hurt(mine.damage_event, damage_event) {
                Some("^3Ouch!^7 You got up")
            } else {
                None
            };
            if toggled || woken || why.is_some() {
                self.ragdoll_get_up(true);
                if let Some(why) = why.filter(|why| !why.is_empty()) {
                    self.ragdoll_centre(why);
                }
            }
        } else if toggled {
            match spared {
                Some(crate::order66::Spared::Duel) => self.ragdoll_centre("No ragdoll in a duel"),
                Some(crate::order66::Spared::Flag) => {
                    self.ragdoll_centre("No ragdoll while carrying the flag");
                }
                Some(crate::order66::Spared::Held) if riding => {
                    self.ragdoll_centre("No ragdoll in a vehicle");
                }
                Some(crate::order66::Spared::Held) => self.ragdoll_centre("No ragdoll right now"),
                Some(_) => {}
                None if self.hax.busy() => self.ragdoll_centre("No ragdoll in a hack"),
                None => self.ragdoll_fall(damage_event),
            }
        }
        self.ragdoll_tell();
    }

    /// The player falls, now.
    fn ragdoll_fall(&mut self, damage_event: u8) {
        self.ragdoll_fall_at(self.ragdoll.time as i32, damage_event, false);
    }

    /// The player falls at `moment` (game server milliseconds); `auto`: a monitor
    /// knocked them down, and they get up by themselves. False when the player is not
    /// in a game.
    fn ragdoll_fall_at(&mut self, moment: i32, damage_event: u8, auto: bool) -> bool {
        self.ragdoll_fall_kicked(moment, damage_event, auto, [0.0; 3])
    }

    /// [`Self::ragdoll_fall_at`], the body starting with `kick` added to the player's speed.
    fn ragdoll_fall_kicked(
        &mut self,
        moment: i32,
        damage_event: u8,
        auto: bool,
        kick: [f32; 3],
    ) -> bool {
        let Some(own) = self.ragdoll.own else {
            return false;
        };
        let Some(state) = self.local_prediction.predicted_state() else {
            return false;
        };
        let round = |v: f32| v.round() as i32;
        let yaw = self.camera_yaw.to_degrees().rem_euclid(360.0).round() as u16 % 360;
        let scale = self.ragdoll_own_scale();
        let start = Start {
            moment,
            origin: state.origin.map(round),
            velocity: [0, 1, 2]
                .map(|axis| round((state.velocity[axis] + kick[axis]).clamp(-3_999.0, 3_999.0))),
            yaw,
            scale,
        };
        self.ragdoll.set_fall(own, Some(Fall::new(start)));
        self.ragdoll.mine = Some(Mine {
            moment: start.moment,
            told: [0; SLOTS],
            tell_at: Instant::now(),
            hits: Vec::new(),
            auto,
            rest: None,
            damage_event,
            camera: (self.camera_pitch, self.camera_yaw),
            fell: Instant::now(),
        });
        self.ragdoll.cues.push(Cue::flatline(None, own));
        true
    }

    /// How big the player's own body falls ([`Start::scale`]): their `iModelScale`, as
    /// their model is drawn (`actor_world_submission::model_scale::source`).
    pub(crate) fn ragdoll_own_scale(&self) -> u16 {
        self.live_session.as_ref().map_or(STOCK_SCALE, |session| {
            scale_percent(session.latest_snapshot().player.model_scale_percent())
        })
    }

    /// The player is struck by monitor `by` (or slips on a banana peel, `by`'s number
    /// [`UNTHROWN`]) at `time` (game server milliseconds) at point `joint` of their body,
    /// pushed by `push` (units a second): they fall if they stand (and may:
    /// [`Ragdoll::strikable`]), the body takes the blow, and everyone is told. False when
    /// it did not land.
    pub(crate) fn ragdoll_struck(&mut self, time: i64, joint: u8, push: [i32; 3], by: By) -> bool {
        if self.ragdoll.mine.is_none() {
            let Some(session) = self.live_session.as_ref() else {
                return false;
            };
            let damage_event = session.latest_snapshot().player.damage_event();
            if !self.ragdoll_fall_at(time as i32, damage_event, true) {
                return false;
            }
        }
        let Some((moment, steps)) = self.ragdoll.own_steps() else {
            return false;
        };
        let step = step_of(moment, time).max(steps);
        let push = push.map(|c| c.clamp(-MAX_PUSH, MAX_PUSH));
        let hit = Hit {
            fall: moment,
            strike: Strike { step, joint, push },
            by,
        };
        let Some(own) = self.ragdoll.own else {
            return false;
        };
        let Some(fall) = self.ragdoll.fall_mut(own) else {
            return false;
        };
        if fall.body.strike_count() >= MAX_HITS {
            return false;
        }
        fall.strike(hit.strike);
        if let Some(mine) = self.ragdoll.mine.as_mut() {
            mine.hits.push(hit);
            mine.rest = None;
        }
        self.ragdoll.bounces.push(Bounce {
            by,
            push,
            struck: own,
        });
        true
    }

    /// This client foresees the player in `slot` (another, with the Monitor on) struck
    /// by a monitor at `time` at point `joint`, pushed by `push`: their body falls from
    /// `standing` if they stand and takes the blow at once, rather than a ping or more
    /// later when their own word of it comes; that word then takes its place, and
    /// without it the blow is undone after [`GUESS_WAIT`]. False when it did not land.
    pub(crate) fn ragdoll_foresee(
        &mut self,
        slot: u8,
        time: i64,
        joint: u8,
        push: [i32; 3],
        standing: Start,
    ) -> bool {
        self.ragdoll
            .foresee(slot, time, joint, push, standing, Instant::now())
    }

    /// Whether the player's own fall, begun by a monitor, has lain still long enough to
    /// end by itself: [`REST_UP`] since the body came to rest, the real body by it (or
    /// [`REST_WAIT`] more).
    fn ragdoll_rested(&mut self) -> bool {
        let asleep = self
            .ragdoll
            .own
            .and_then(|own| self.ragdoll.falls.get(usize::from(own))?.as_ref())
            .is_some_and(|fall| fall.body.asleep());
        let near = match (
            self.ragdoll.walk_to,
            self.local_prediction.predicted_state(),
        ) {
            (Some(to), Some(state)) => {
                (to - Vec3::from_array(state.origin)).truncate().length() <= WALK_NEAR
            }
            _ => true,
        };
        let Some(mine) = self.ragdoll.mine.as_mut().filter(|mine| mine.auto) else {
            return false;
        };
        if !asleep {
            mine.rest = None;
            return false;
        }
        let rest = mine.rest.get_or_insert_with(Instant::now).elapsed();
        rest >= REST_UP && (near || rest >= REST_UP + REST_WAIT)
    }

    /// The player gets up: everyone told hears of it (`tell`), the camera's angles are
    /// given back.
    fn ragdoll_get_up(&mut self, tell: bool) {
        let Some(mine) = self.ragdoll.mine.take() else {
            return;
        };
        if tell {
            for slot in 0..SLOTS as u8 {
                if mine.told[usize::from(slot)] != 0 {
                    relay(Event::Up(mine.moment), slot);
                }
            }
        }
        if let Some(own) = self.ragdoll.own {
            self.ragdoll.set_fall(own, None);
        }
        self.ragdoll.walk_to = None;
        (self.camera_pitch, self.camera_yaw) = mine.camera;
    }

    /// Tell the next SJK player on the server not yet told all of the player's fall: the
    /// fall first, then each blow it took.
    fn ragdoll_tell(&mut self) {
        let Some(own) = self.ragdoll.own else {
            return;
        };
        let start = match self.ragdoll.fall_mut(own) {
            Some(fall) => fall.start,
            None => return,
        };
        let sjk = self.looks.sjk_slots() & !(1 << own);
        let from = self
            .ragdoll
            .falls
            .get(usize::from(own))
            .and_then(Option::as_ref)
            .map_or(Vec3::ZERO, |fall| fall.body.centre());
        let time = self.ragdoll.time;
        let distances: [f32; SLOTS] = std::array::from_fn(|slot| {
            self.live_world
                .entity(EntityId::new(slot as u64 + 1))
                .map_or(f32::MAX, |entity| {
                    Vec3::from_array(entity.sample(time).translation).distance(from)
                })
        });
        let Some(mine) = self.ragdoll.mine.as_mut() else {
            return;
        };
        // One who left is told again should they come back.
        for (slot, told) in mine.told.iter_mut().enumerate() {
            if sjk & (1 << slot) == 0 {
                *told = 0;
            }
        }
        let now = Instant::now();
        if now < mine.tell_at {
            return;
        }
        let throwers = mine
            .hits
            .iter()
            .filter(|hit| usize::from(hit.by.thrower) < SLOTS)
            .fold(0_u64, |throwers, hit| throwers | 1 << hit.by.thrower);
        let Some(slot) = next_told(&mine.told, 1 + mine.hits.len(), sjk, throwers, &distances)
        else {
            return;
        };
        let told = usize::from(mine.told[slot]);
        mine.told[slot] += 1;
        mine.tell_at = now + TELL_GAP;
        let event = match told {
            0 => Event::Fall(start),
            hit => Event::Hit(mine.hits[hit - 1]),
        };
        relay(event, slot as u8);
    }

    /// Step every fall on to `time` (game server milliseconds, the presentation's), end
    /// the falls of players who are gone, and set what is shown this frame.
    pub(crate) fn step_ragdolls(&mut self, time: i64) {
        #[cfg(test)]
        if self.ragdoll.shot_frozen {
            return;
        }
        self.ragdoll.time = time;
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            *shown_lock() = [None; SLOTS];
            return;
        };
        let snapshot = session.snapshot_at_or_before(time as i32);
        let own = self.ragdoll.own;
        let mut shown = [None; SLOTS];
        let mut movers = std::mem::take(&mut self.ragdoll.movers);
        let mut world = scene::Scene::new(
            &self.bsp,
            &mut self.trace_scratch,
            Some(session),
            &mut movers,
        );
        for (slot, entry) in self.ragdoll.falls.iter_mut().enumerate() {
            let Some(fall) = entry else {
                continue;
            };
            let slot = slot as u8;
            // Another's: gone, dead, hidden or respawned ends it; so does their real
            // body staying far from the ragdoll (the end did not reach here).
            if Some(slot) != own {
                let entity = snapshot.entities.iter().find(|entity| {
                    entity.number() == u16::from(slot) && entity.entity_type() == ET_PLAYER
                });
                let ended = entity.is_none_or(|entity| {
                    let flags = entity.e_flags();
                    let respawned = fall
                        .teleport
                        .replace(flags & EF_TELEPORT_BIT)
                        .is_some_and(|bit| bit != flags & EF_TELEPORT_BIT);
                    flags & (EF_DEAD | EF_NODRAW) != 0 || respawned
                });
                let distance = self
                    .live_world
                    .entity(EntityId::new(u64::from(slot) + 1))
                    .map(|entity| Vec3::from_array(entity.sample(time).translation))
                    .map_or(0.0, |at| at.distance(fall.body.centre()));
                // Far, and not walking nearer (the faller's client walks the real body
                // to a ragdoll thrown far): the clock runs.
                let astray = match fall.astray {
                    _ if distance <= ASTRAY_REACH => {
                        fall.astray = None;
                        Instant::now()
                    }
                    Some((since, nearest)) if distance > nearest - ASTRAY_NEARER => since,
                    _ => {
                        fall.astray = Some((Instant::now(), distance));
                        Instant::now()
                    }
                };
                if ended || astray.elapsed() > ASTRAY {
                    *entry = None;
                    continue;
                }
            }
            let since = (time - i64::from(fall.start.moment)).max(0);
            let steps = (since / i64::from(STEP_MS)) as u32;
            let fraction = (since % i64::from(STEP_MS)) as f32 / STEP_MS as f32;
            let before = fall.body.steps();
            fall.body.run_to(steps, &mut world);
            // The knocks and scrapes of this frame's steps (not while fast-forwarding a
            // fall heard late).
            if fall.body.steps() > before && fall.body.steps() - before < 8 {
                sounds::hear(
                    &mut fall.heard,
                    &fall.body,
                    time,
                    slot,
                    &mut self.ragdoll.cues,
                );
            }
            shown[usize::from(slot)] = Some(fall.body.between(fraction));
        }
        self.ragdoll.movers = movers;
        // Where the player's own real body walks to: the ragdoll's pelvis.
        self.ragdoll.walk_to = own
            .and_then(|own| shown[usize::from(own)])
            .filter(|_| self.ragdoll.mine.is_some())
            .map(|points| points[PELVIS]);
        *shown_lock() = shown;
    }

    /// Play the toy's sounds of the frame.
    pub(crate) fn play_ragdoll_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.ragdoll.cues.clear();
            return;
        };
        if self.ragdoll.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [FLATLINE].into_iter().chain(sounds::paths()));
            self.ragdoll.sounds_registered = Some(audio.generation());
        }
        for Cue {
            path,
            at,
            slot,
            volume,
        } in self.ragdoll.cues.drain(..)
        {
            if path == FLATLINE {
                let now = Instant::now();
                let heard = self.ragdoll.flatline_heard.get(&slot);
                if heard.is_some_and(|heard| now.duration_since(*heard) < FLATLINE_GAP) {
                    continue;
                }
                self.ragdoll.flatline_heard.insert(slot, now);
            }
            audio.play_toy(
                crate::toy_sounds::Toy::Ragdoll,
                path,
                volume,
                at.map(|at| at.to_array()),
                sjk_audio::SourceId(u32::from(slot)),
                if at.is_none() || path == FLATLINE {
                    crate::audio::TOY_CHANNEL
                } else {
                    sjk_audio::ChannelId(0)
                },
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: the player in `slot` of the demo playing falls where they
    /// stand, with `velocity` and facing `yaw`, and is shown `steps` steps on.
    pub(crate) fn shot_ragdoll(&mut self, slot: u16, velocity: [i32; 3], yaw: u16, steps: u32) {
        self.shot_ragdoll_struck(slot, velocity, yaw, None, steps);
    }

    /// For the world shots: as [`GpuState::shot_ragdoll`], the body struck at its first
    /// step at point `joint` with `push` (the Monitor's blow), when given.
    pub(crate) fn shot_ragdoll_struck(
        &mut self,
        slot: u16,
        velocity: [i32; 3],
        yaw: u16,
        strike: Option<(u8, [i32; 3])>,
        steps: u32,
    ) {
        let session = self.demo_session.as_ref().expect("a demo playing");
        let snapshot = session.latest_snapshot();
        let time = snapshot.server_time;
        let origin = session
            .world()
            .entity(EntityId::new(u64::from(slot) + 1))
            .map(|entity| entity.sample(i64::from(time)).translation)
            .expect("the player");
        let scale = snapshot
            .entities
            .iter()
            .find(|entity| entity.number() == slot)
            .map_or(STOCK_SCALE, |entity| {
                scale_percent(entity.model_scale_percent())
            });
        let start = Start {
            moment: time,
            origin: origin.map(|v| v.round() as i32),
            velocity,
            yaw,
            scale,
        };
        let mut body = Body::new(&start);
        if let Some((joint, push)) = strike {
            body.strike(Strike {
                step: 0,
                joint,
                push,
            });
        }
        let mut movers = scene::Movers::default();
        body.run_to(
            steps,
            &mut scene::Scene::new(&self.bsp, &mut self.trace_scratch, None, &mut movers),
        );
        self.ragdoll.shot_frozen = true;
        shown_lock()[usize::from(slot)] = Some(*body.points());
    }
}

#[cfg(test)]
impl Ragdoll {
    /// For tests: on a game server as slot 0 (`in_game`), spared from falls (dead,
    /// spectating, in a duel, with the flag, hacked, held: `spared`), limp (`limp`).
    pub(crate) fn for_tests(in_game: bool, spared: bool, limp: bool) -> Self {
        let mut ragdoll = Self::default();
        if in_game {
            ragdoll.server = Some(std::net::SocketAddr::from(([127, 0, 0, 1], 29_070)));
            ragdoll.own = Some(0);
        }
        ragdoll.spared = spared;
        if limp {
            ragdoll.mine = Some(Mine {
                moment: 0,
                told: [0; SLOTS],
                tell_at: Instant::now(),
                hits: Vec::new(),
                auto: true,
                rest: None,
                damage_event: 0,
                camera: (0.0, 0.0),
                fell: Instant::now(),
            });
        }
        ragdoll
    }
}

/// For tests: a body falling from `start` onto a floor at height 0, struck at its first
/// step by `strike` if given; its points after each step up to `steps`.
#[cfg(test)]
pub(crate) fn fall_on_a_floor(
    start: &Start,
    strike: Option<Strike>,
    steps: u32,
) -> Vec<[Vec3; JOINTS]> {
    let mut floor = |from: Vec3, to: Vec3, half: f32| {
        (to.z < half).then(|| {
            let t = if from.z > half {
                ((from.z - half) / (from.z - to.z)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut at = from + (to - from) * t;
            at.z = at.z.max(half);
            (at, Vec3::Z)
        })
    };
    let mut body = Body::new(start);
    if let Some(strike) = strike {
        body.strike(strike);
    }
    (1..=steps)
        .map(|step| {
            body.run_to(step, &mut floor);
            *body.points()
        })
        .collect()
}

/// Whether the player was hurt since their fall began: the server's damage feedback
/// sequence (`damage_event`, now) moved on from the one at the fall (`at_fall`).
fn hurt(at_fall: u8, now: u8) -> bool {
    at_fall != now
}

/// Where a fall starts, for its flatline: the player's origin.
fn start_point(start: &Start) -> Vec3 {
    Vec3::new(
        start.origin[0] as f32,
        start.origin[1] as f32,
        start.origin[2] as f32,
    )
}

/// The limp player's camera: behind the ragdoll's pelvis, turned as the player's own
/// camera is; `None` otherwise, and while the player flies the free camera, which keeps
/// its own view (Dave's test, 28/10/2026). Plain `fakenoclip` keeps this camera: it has
/// no detached view of its own (#232's review).
pub(crate) fn camera(state: &mut GpuState, time: i64) -> Option<(Vec3, Vec3)> {
    state.ragdoll.mine.as_ref()?;
    if state.local_prediction.free_camera() {
        return None;
    }
    let session = state.live_session.as_ref()?;
    let game = session.game_state();
    let snapshot = session.snapshot_at_or_before(time as i32);
    if snapshot.player.movement_flags() & PMF_FOLLOW != 0 {
        return None;
    }
    let points = shown(state.ragdoll.own?)?;
    let (pitch, yaw) = (state.camera_pitch, state.camera_yaw);
    let forward = Vec3::new(
        yaw.cos() * pitch.cos(),
        yaw.sin() * pitch.cos(),
        pitch.sin(),
    );
    let focus = points[PELVIS] + Vec3::Z * CAMERA_HEIGHT;
    let wanted = focus - forward * CAMERA_RANGE + Vec3::Z * CAMERA_RISE;
    let position = crate::movement_collision::camera_trace(
        &state.bsp,
        &mut state.trace_scratch,
        Some((game, snapshot)),
        time as i32,
        focus,
        wanted,
    );
    Some((position, position + forward * 256.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guess_start(moment: i32) -> Start {
        Start {
            moment,
            origin: [0, 0, 24],
            velocity: [0; 3],
            yaw: 0,
            scale: STOCK_SCALE,
        }
    }

    #[test]
    fn the_thrower_hears_first_then_each_player_the_fall_and_its_blows_in_a_row() {
        let mut distances = [f32::MAX; SLOTS];
        distances[1] = 500.0;
        distances[2] = 100.0;
        distances[3] = 300.0;
        let sjk = 0b1110;
        let mut told = [0_u8; SLOTS];
        let mut order = Vec::new();
        // A monitor of player 3 (not the nearest) knocked the player down: fall and blow.
        while let Some(slot) = next_told(&told, 2, sjk, 1 << 3, &distances) {
            told[slot] += 1;
            order.push(slot);
        }
        assert_eq!(order, [3, 3, 2, 2, 1, 1]);
        // A new blow by player 1, everyone told of the fall: 1 first, then the nearest.
        let mut order = Vec::new();
        while let Some(slot) = next_told(&told, 3, sjk, 1 << 1, &distances) {
            told[slot] += 1;
            order.push(slot);
        }
        assert_eq!(order, [1, 2, 3]);
    }

    #[test]
    fn a_foreseen_fall_gives_way_to_the_real_one_and_its_blow() {
        let mut ragdoll = Ragdoll::default();
        let now = Instant::now();
        let push = [2_000, 0, 260];
        assert!(ragdoll.foresee(5, 10_000, 2, push, guess_start(10_000), now));
        assert!(ragdoll.guessing(5));
        // A second monitor waits for the first's word.
        assert!(!ragdoll.foresee(5, 10_100, 2, push, guess_start(10_100), now));
        // The real fall (begun a step later where they really stood) keeps the blow.
        let real = Start {
            origin: [8, 0, 24],
            ..guess_start(10_016)
        };
        ragdoll.hear(5, Event::Fall(real), 10_100);
        let fall = ragdoll.fall_mut(5).unwrap();
        assert_eq!(fall.start, real);
        assert_eq!(fall.body.strike_count(), 1);
        assert!(ragdoll.guessing(5));
        // The real blow takes the foreseen one's place; the monitor bounces off it.
        let strike = Strike {
            step: 0,
            joint: 1,
            push: [1_900, 100, 260],
        };
        let by = By {
            thrower: 3,
            number: 7,
            step: 40,
        };
        ragdoll.hear(
            5,
            Event::Hit(Hit {
                fall: real.moment,
                strike,
                by,
            }),
            10_120,
        );
        let fall = ragdoll.fall_mut(5).unwrap();
        assert_eq!(fall.body.strike_count(), 1);
        assert!(fall.body.has_strike(&strike));
        assert!(!ragdoll.guessing(5));
        assert_eq!(
            ragdoll.take_bounces(),
            [Bounce {
                by,
                push: strike.push,
                struck: 5
            }]
        );
    }

    #[test]
    fn a_foreseen_blow_no_word_confirms_is_undone() {
        let mut ragdoll = Ragdoll::default();
        let now = Instant::now();
        // Standing: the foreseen fall ends and they stand again.
        assert!(ragdoll.foresee(4, 5_000, 2, [1_500, 0, 260], guess_start(5_000), now));
        ragdoll.expire_guesses(now + GUESS_WAIT / 2);
        assert!(ragdoll.fall_mut(4).is_some());
        ragdoll.expire_guesses(now + GUESS_WAIT);
        assert!(ragdoll.fall_mut(4).is_none() && !ragdoll.guessing(4));
        // Limp by themselves: the fall stays, without the blow.
        ragdoll.hear(4, Event::Fall(guess_start(6_000)), 6_000);
        assert!(ragdoll.foresee(4, 6_200, 1, [1_500, 0, 260], guess_start(6_200), now));
        assert_eq!(ragdoll.fall_mut(4).unwrap().body.strike_count(), 1);
        ragdoll.expire_guesses(now + GUESS_WAIT);
        let fall = ragdoll.fall_mut(4).unwrap();
        assert_eq!(fall.start.moment, 6_000);
        assert_eq!(fall.body.strike_count(), 0);
        // Never the player's own body.
        ragdoll.own = Some(2);
        assert!(!ragdoll.foresee(2, 7_000, 1, [1_500, 0, 260], guess_start(7_000), now));
    }

    #[test]
    fn a_name_hangs_over_the_head_of_a_standing_body_and_low_over_a_lying_one() {
        let origin = Vec3::new(100.0, 0.0, 24.0);
        let standing = standing(origin, 0.0, STOCK_SCALE);
        let (pelvis, head) = top_of(&standing);
        assert_eq!(pelvis, standing[PELVIS]);
        // About where a standing player's box top is (40 over the origin).
        let top = pelvis.z + head;
        assert!((origin.z + 30.0..origin.z + 52.0).contains(&top), "{top}");
        // The same body laid flat along +X: the top barely over the pelvis.
        let lying = standing.map(|point| {
            let up = point - standing[PELVIS];
            standing[PELVIS] + Vec3::new(up.z, up.y, -up.x * 0.1)
        });
        let (_, low) = top_of(&lying);
        assert!(low < 15.0, "{low}");
    }

    fn start() -> Start {
        Start {
            moment: 123_456,
            origin: [-1_024, 512, 88],
            velocity: [320, -12, 270],
            yaw: 271,
            scale: STOCK_SCALE,
        }
    }

    #[test]
    fn events_read_back_as_written_and_fit_the_relay() {
        let hit = Hit {
            fall: 2_147_000_000,
            strike: Strike {
                step: 4_000_000,
                joint: 14,
                push: [-3_000, 2_999, -3_000],
            },
            by: By {
                thrower: 63,
                number: 255,
                step: 65_535,
            },
        };
        for event in [Event::Fall(start()), Event::Up(123_456), Event::Hit(hit)] {
            let text = event.text();
            assert!(text.len() <= 64, "{text}");
            assert!(
                text.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'_'
                    || b == b'-'),
                "{text}"
            );
            assert_eq!(Event::parse(&text), Some(event));
        }
        assert_eq!(Event::parse("fall_1_2"), None);
        assert_eq!(Event::parse("up_x"), None);
        assert_eq!(Event::parse("in_1"), None);
        assert_eq!(
            Event::parse("hit_1_2_15_0_0_0_0_0_0"),
            None,
            "no 16th joint"
        );
        assert_eq!(Event::parse("hit_1_2_3_0_0_3001_0_0_0"), None, "too hard");
        assert_eq!(Event::parse("hit_1_2_3_0_0_0_64_0_0"), None, "no slot 64");
    }

    #[test]
    fn a_limp_player_walks_to_the_ragdoll_without_a_button() {
        let mut ragdoll = Ragdoll::default();
        let input = UserCommand {
            forward_move: -127,
            buttons: 1,
            ..UserCommand::default()
        };
        assert!(ragdoll.drive(&input, [0; 3], [0.0; 3]).is_none());
        ragdoll.mine = Some(Mine {
            moment: 0,
            told: [0; SLOTS],
            tell_at: Instant::now(),
            hits: Vec::new(),
            auto: false,
            rest: None,
            damage_event: 0,
            camera: (0.0, 0.0),
            fell: Instant::now(),
        });
        // Far off along +Y: runs there, facing it.
        ragdoll.walk_to = Some(Vec3::new(0.0, 300.0, 0.0));
        let drive = ragdoll.drive(&input, [0; 3], [0.0; 3]).unwrap();
        assert_eq!(drive.command.forward_move, 127);
        assert_eq!(drive.command.buttons, 0);
        assert_eq!(drive.command.angles[1], 16_384);
        // Near: walks; there: stands.
        ragdoll.walk_to = Some(Vec3::new(40.0, 0.0, 0.0));
        let drive = ragdoll.drive(&input, [0; 3], [0.0; 3]).unwrap();
        assert_eq!(drive.command.buttons, BUTTON_WALKING);
        ragdoll.walk_to = Some(Vec3::new(5.0, 0.0, 30.0));
        let drive = ragdoll.drive(&input, [0; 3], [0.0; 3]).unwrap();
        assert_eq!(drive.command.forward_move, 0);
        // `delta_angles` are taken off, as the game adds them back.
        ragdoll.walk_to = Some(Vec3::new(0.0, 300.0, 0.0));
        let drive = ragdoll.drive(&input, [0, 100, 0], [0.0; 3]).unwrap();
        assert_eq!(drive.command.angles[1], 16_284);
    }

    #[test]
    fn a_new_jump_attack_or_taunt_gets_a_limp_player_up() {
        let mut ragdoll = Ragdoll::default();
        let idle = UserCommand::default();
        let attack = UserCommand {
            buttons: 1,
            ..UserCommand::default()
        };
        // Standing: nothing.
        ragdoll.wake(&attack);
        assert!(!ragdoll.woken);
        let fall = |fell| Mine {
            moment: 0,
            told: [0; SLOTS],
            tell_at: Instant::now(),
            hits: Vec::new(),
            auto: true,
            rest: None,
            damage_event: 0,
            camera: (0.0, 0.0),
            fell,
        };
        // Attack held as they fell: not until let go and pressed again.
        ragdoll.mine = Some(fall(Instant::now() - WAKE_AFTER));
        ragdoll.wake(&attack);
        assert!(!ragdoll.woken);
        ragdoll.wake(&idle);
        ragdoll.wake(&attack);
        assert!(ragdoll.woken);
        // Too soon after the fall: the press is lost.
        ragdoll.woken = false;
        ragdoll.mine = Some(fall(Instant::now()));
        ragdoll.wake(&idle);
        ragdoll.wake(&attack);
        assert!(!ragdoll.woken);
        ragdoll.mine = Some(fall(Instant::now() - WAKE_AFTER));
        for press in [
            UserCommand {
                up_move: 127,
                ..UserCommand::default()
            },
            UserCommand {
                buttons: 128,
                ..UserCommand::default()
            },
            UserCommand {
                generic_command: 27,
                ..UserCommand::default()
            },
            UserCommand {
                buttons: 8,
                ..UserCommand::default()
            },
        ] {
            ragdoll.woken = false;
            ragdoll.wake(&idle);
            ragdoll.wake(&press);
            assert!(ragdoll.woken);
        }
        // Moving or crouching does not.
        ragdoll.woken = false;
        ragdoll.wake(&idle);
        ragdoll.wake(&UserCommand {
            forward_move: 127,
            up_move: -127,
            ..UserCommand::default()
        });
        assert!(!ragdoll.woken);
    }

    #[test]
    fn only_a_new_hurt_wakes_the_player() {
        // #135: a fall begun after a hit holds that hit's feedback, so it stays down
        // until the next one.
        assert!(!hurt(7, 7));
        assert!(hurt(7, 8));
        assert!(hurt(255, 0));
    }

    #[test]
    fn a_ragdoll_places_its_actor_at_the_pelvis_unturned_at_its_model_scale() {
        // Hub report #223: a player drawn at 60 % (`iModelScale`) stays that size limp,
        // its body simulated at the same scale.
        let body = Body::new(&Start {
            scale: 60,
            ..start()
        });
        let mut transform = sjk_runtime::Transform::IDENTITY;
        crate::actor_world_submission::model_scale::apply(&mut transform, 60, 0);
        transform.rotation = glam::Quat::from_rotation_z(1.0).to_array();
        place(&mut transform, body.points());
        assert_eq!(transform.translation, body.centre().to_array());
        assert_eq!(transform.scale, [0.6; 3]);
        assert_eq!(transform.rotation, glam::Quat::IDENTITY.to_array());
    }
}
