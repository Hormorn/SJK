//! The WHEEL OF DOOM (Sol's design of 07/11/2026; `docs/wheel-of-doom.md`): a toy any SJK
//! player spins for credits with `toy_wheelofdoom` (or Collection > Toys). The hub takes
//! the price, keeps the cooldown and rolls the outcome (`sjk_identity::wheel_of_doom`);
//! this module asks for the spin after a second press (the first shows the price),
//! stands a big dark wheel on a stand before the spinner, spins it for [`SPIN`], slowing,
//! onto the hub's segment under the pointer, and plays what it landed on: one of the
//! [`effects`] on the spinner's own input and view, the game's `kill`, a curse, nothing,
//! or the Mythical jackpot. The other SJK players on the server see and hear the wheel
//! through the hub's toy event (`wheel_of_doom`, `spin_<outcome>`). No game server knows
//! of the wheel: it is drawn by SJK clients only and has no collision.

mod draw;
mod effects;
mod mirror;

pub(crate) use effects::{Effect, Effects, Saber};

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec3};
use sjk_identity::wheel_of_doom as wire;
use std::time::{Duration, Instant};

/// The toy's command: the first press shows the price, a second within [`CONFIRM`]
/// spins.
pub(crate) const COMMAND: &str = "toy_wheelofdoom";
/// Its line in `cmdlist`.
pub(crate) const HELP: &str = "Toy: spin the WHEEL OF DOOM for credits (run it once to see the price, again to spin); it may do something terrible to you";

/// The toy's icon (key bindings; Collection > Toys draws the quick wheel atlas's copy).
pub(crate) const ICON: &str = "gfx/sjk/toy_wheelofdoom.png";
/// The stand (with the pointer) and the wheel (bundled, `scripts/wheel_of_doom_model.py`;
/// the contract Sol's model keeps is in `docs/wheel-of-doom.md`).
pub(crate) const STAND_MODEL: &str = "models/sjk/wheel_of_doom_stand.md3";
pub(crate) const WHEEL_MODEL: &str = "models/sjk/wheel_of_doom_wheel.md3";
/// Both, loaded with every map.
pub(crate) const MODELS: [&str; 2] = [STAND_MODEL, WHEEL_MODEL];
/// The wheel's radius, and how high its axle stands over the stand's origin (the floor).
const RADIUS: f32 = 40.0;
const AXLE: f32 = 64.0;
/// The segments round the wheel, and the angle each takes.
pub(crate) const SEGMENTS: usize = 16;
const SEGMENT_DEGREES: f32 = 360.0 / SEGMENTS as f32;
/// How long the wheel spins, how many whole turns it makes before the last, how long it
/// stays once stopped and how long it takes to shrink away.
const SPIN: Duration = Duration::from_secs(6);
const TURNS: f32 = 4.0;
const HOLD: Duration = Duration::from_secs(4);
const SHRINK: Duration = Duration::from_millis(600);
/// How long after the stop a wheel landing on `explode` blows its spinner up.
const BLAST_DELAY: Duration = Duration::from_millis(1_500);
/// How long the price shown by a first press waits for the second.
const CONFIRM: Duration = Duration::from_secs(5);
/// How long a spin waits for the hub before the player may ask again.
const ASK_TIMEOUT: Duration = Duration::from_secs(10);
/// The cooldown when the hub has not said it yet, seconds.
const DEFAULT_COOLDOWN: u64 = 300;
/// How far before the spinner the wheel stands, and the nearest it comes when a wall
/// is closer.
const PLACE: f32 = 110.0;
const PLACE_MIN: f32 = 48.0;
/// The most wheels at once (the spinner's and others').
const MOST: usize = 8;

/// A peg passing the pointer, the stop, the jackpot's fanfare and the Quack effect's
/// quack (bundled, `scripts/wheel_of_doom_assets.py`).
const TICK: &str = "sound/sjk/wheel_of_doom/tick.wav";
const STOP: &str = "sound/sjk/wheel_of_doom/stop.wav";
const FANFARE: &str = "sound/sjk/wheel_of_doom/fanfare.wav";
const QUACK: &str = "sound/sjk/wheel_of_doom/quack.wav";
/// The laugh on a bad outcome, the mystery box's: Sol's laugh in the optional SJK extras
/// pack, else the retail gloat.
const LAUGH: &str = "sound/sjk/mystery_box/laugh.mp3";
const LAUGH_FALLBACK: &str = "sound/chars/alora/misc/gloat1.mp3";
/// The explosion where a spinner landing on `explode` stands, the mystery box's.
const BLAST_SOUND: &str = "sound/weapons/detpack/explode.mp3";
const BLAST_EFFECT: &str = "detpack/explosion";
/// The wheels' sound sources, past the other toys' (`0x5000_0000` to `0x5500_0000`),
/// one a slot.
const SOURCE_BASE: u32 = 0x5600_0000;

/// A bundled file's bytes, by its name in `assets/wheel_of_doom`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/wheel_of_doom/", $name))
    };
}

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 10] = [
    (STAND_MODEL, bundled!("wheel_of_doom_stand.md3")),
    (WHEEL_MODEL, bundled!("wheel_of_doom_wheel.md3")),
    (
        "models/sjk/wheel_of_doom.jpg",
        bundled!("wheel_of_doom.jpg"),
    ),
    (
        "models/sjk/wheel_of_doom_glow.jpg",
        bundled!("wheel_of_doom_glow.jpg"),
    ),
    (
        "shaders/sjk_wheel_of_doom.shader",
        bundled!("wheel_of_doom.shader"),
    ),
    (
        "gfx/sjk/toy_wheelofdoom.png",
        bundled!("toy_wheelofdoom.png"),
    ),
    (TICK, bundled!("tick.wav")),
    (STOP, bundled!("stop.wav")),
    (FANFARE, bundled!("fanfare.wav")),
    (QUACK, bundled!("quack.wav")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK wheel of doom", FILES.iter().copied())?;
    Ok(())
}

/// What a segment does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A Mythical holocron.
    Jackpot,
    Nothing,
    Effect(Effect),
    /// The game's `kill`, as a mystery box that blows up.
    Explode,
    /// Another bad effect, later.
    Curse,
}

impl Kind {
    /// The wheel laughs as it stops on it.
    fn bad(self) -> bool {
        matches!(self, Self::Effect(_) | Self::Explode | Self::Curse)
    }
}

/// One segment: its hub id, what it does, its name over the wheel and what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) id: &'static str,
    pub(crate) kind: Kind,
    pub(crate) title: &'static str,
    pub(crate) line: &'static str,
}

const fn outcome(id: &'static str, kind: Kind, title: &'static str, line: &'static str) -> Outcome {
    Outcome {
        id,
        kind,
        title,
        line,
    }
}

/// The segments in the wheel's order, as the hub's contract of 07/11/2026 lists them
/// (segment 0, the gold jackpot, under the pointer at angle 0, the others clockwise).
pub(crate) const OUTCOMES: [Outcome; SEGMENTS] = [
    outcome(
        "mythical",
        Kind::Jackpot,
        "MYTHICAL HOLOCRON!",
        "The jackpot",
    ),
    outcome("nothing", Kind::Nothing, "NOTHING", "Nothing happens..."),
    outcome(
        "reverse_keys",
        Kind::Effect(Effect::ReverseKeys),
        "REVERSE KEYS",
        "Forward is back for 30 s",
    ),
    outcome(
        "invert_mouse",
        Kind::Effect(Effect::InvertMouse),
        "INVERTED MOUSE",
        "The mouse turns the wrong way for 30 s",
    ),
    outcome(
        "mirror",
        Kind::Effect(Effect::Mirror),
        "MIRROR",
        "The world is mirrored for 30 s",
    ),
    outcome(
        "upside_down",
        Kind::Effect(Effect::UpsideDown),
        "UPSIDE DOWN",
        "The world turns over for 20 s",
    ),
    outcome(
        "blind",
        Kind::Effect(Effect::Blind),
        "BLIND",
        "Darkness for 15 s",
    ),
    outcome(
        "drunk",
        Kind::Effect(Effect::Drunk),
        "DRUNK",
        "The room sways for 30 s",
    ),
    outcome(
        "tiny_fov",
        Kind::Effect(Effect::TinyFov),
        "TUNNEL VISION",
        "A narrow view for 20 s",
    ),
    outcome(
        "huge_fov",
        Kind::Effect(Effect::HugeFov),
        "FISHEYE",
        "A huge view for 20 s",
    ),
    outcome(
        "no_saber",
        Kind::Effect(Effect::NoSaber),
        "NO SABER",
        "Your saber stays off for 20 s",
    ),
    outcome(
        "quack",
        Kind::Effect(Effect::Quack),
        "QUACK",
        "Every swing quacks for 30 s",
    ),
    outcome(
        "walk",
        Kind::Effect(Effect::Walk),
        "WALK",
        "No running for 30 s",
    ),
    outcome(
        "bunny",
        Kind::Effect(Effect::Bunny),
        "BUNNY",
        "You cannot stop jumping for 20 s",
    ),
    outcome("explode", Kind::Explode, "BOOM!", "It blows up"),
    outcome(
        "curse",
        Kind::Curse,
        "CURSED...",
        "Something bad comes within a minute",
    ),
];

/// The segment `id` names, by the built-in order.
pub(crate) fn known(id: &str) -> Option<(usize, &'static Outcome)> {
    OUTCOMES
        .iter()
        .enumerate()
        .find(|(_, outcome)| outcome.id == id)
}

/// The clockwise angle (degrees, 0 to 360) the wheel stands at when `segment` is under
/// the pointer: segment `k` is painted `k` segments clockwise of the top, so the wheel
/// turns back by as much.
pub(crate) fn segment_angle(segment: usize) -> f32 {
    (360.0 - (segment % SEGMENTS) as f32 * SEGMENT_DEGREES).rem_euclid(360.0)
}

/// How far the wheel has turned clockwise (degrees) `elapsed` into a spin landing on
/// `segment`: [`TURNS`] whole turns and the last to its angle, fast at first, slowing
/// to a stop at [`SPIN`] (a cubic ease out, as the box's roll slows).
pub(crate) fn wheel_angle(elapsed: Duration, segment: usize) -> f32 {
    let total = TURNS * 360.0 + segment_angle(segment);
    let t = (elapsed.as_secs_f32() / SPIN.as_secs_f32()).min(1.0);
    total * (1.0 - (1.0 - t).powi(3))
}

/// How many pegs (the dividers, half a segment either side of each segment's middle)
/// have passed the pointer once the wheel turned `angle` degrees.
fn pegs_passed(angle: f32) -> u32 {
    ((angle + SEGMENT_DEGREES / 2.0) / SEGMENT_DEGREES) as u32
}

/// The wheel's own turn about its axle at `angle` degrees clockwise (as seen from the
/// front, the model's +X): a turn about -X.
fn wheel_turn(angle: f32) -> Quat {
    Quat::from_rotation_x(-angle.to_radians())
}

/// A wheel spinning, or stopped, before someone.
#[derive(Clone, Debug, PartialEq)]
struct Spinning {
    /// The spinner's slot.
    slot: Option<u8>,
    /// The player's own spin.
    mine: bool,
    outcome: String,
    /// What it does, `None` for an outcome a newer hub knows and this client does not.
    kind: Option<Kind>,
    segment: usize,
    /// Where it stands (the floor under it) and the way its face looks, degrees.
    at: Vec3,
    yaw: f32,
    started: Instant,
    /// Pegs that passed the pointer, for the ticks.
    pegs: u32,
    stopped: bool,
    blasted: bool,
    /// Its words once stopped.
    title: String,
    line: String,
}

impl Spinning {
    fn angle(&self, now: Instant) -> f32 {
        wheel_angle(now.saturating_duration_since(self.started), self.segment)
    }

    /// How big it is at `now`: whole, shrinking away after its hold, `None` once gone.
    fn scale(&self, now: Instant) -> Option<f32> {
        let since = now.saturating_duration_since(self.started);
        let left = (SPIN + HOLD + SHRINK)
            .checked_sub(since)
            .filter(|left| !left.is_zero())?;
        Some((left.as_secs_f32() / SHRINK.as_secs_f32()).min(1.0))
    }
}

/// A sound to play once, at a wheel (`None`: in the player's ears).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cue {
    path: &'static str,
    origin: Option<[f32; 3]>,
    source: sjk_audio::SourceId,
}

fn source(slot: Option<u8>) -> sjk_audio::SourceId {
    sjk_audio::SourceId(SOURCE_BASE | slot.map_or(0x100, u32::from))
}

/// What the player is told as a wheel stops or blows up.
#[derive(Clone, Debug, PartialEq)]
enum Moment {
    /// The player's own wheel stopped on `kind` (`None`: unknown), `title` and `line`.
    Stopped {
        kind: Option<Kind>,
        title: String,
        line: String,
    },
    /// A wheel on `explode` blew its spinner on `slot` up.
    Blast { slot: Option<u8>, at: Vec3 },
}

/// A number to vary a curse by.
fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos())
}

/// What the player knows before asking for a spin.
#[derive(Clone, Copy, Debug, Default)]
struct Check {
    /// On a game server of the internet (not a local game).
    online: bool,
    /// The Ragdoll could strike them: alive, playing, not in a duel or with the flag.
    strikable: bool,
    /// Seconds left of the cooldown, known to the client.
    cooldown: Option<u64>,
    /// A spin was asked and the hub has not answered.
    asked: bool,
    price: Option<u64>,
    credits: Option<u64>,
}

/// Why a spin is refused before the hub is asked, in the player's words; `None` when it
/// may go.
fn refusal(check: Check) -> Option<String> {
    if !check.online {
        return Some("Join a game server to spin the WHEEL OF DOOM".to_owned());
    }
    if check.asked {
        return Some("The WHEEL OF DOOM is asking the hub...".to_owned());
    }
    if !check.strikable {
        return Some(
            "Not now: the WHEEL OF DOOM spins out of duels, alive, without the flag".to_owned(),
        );
    }
    if let Some(left) = check.cooldown {
        return Some(format!("Your next spin in {left} s"));
    }
    match (check.price, check.credits) {
        (Some(price), Some(credits)) if credits < price => Some(format!(
            "The WHEEL OF DOOM costs {price} credits: not enough"
        )),
        _ => None,
    }
}

/// The toy's state: the wheels, the spinner's effects, the spin asked and the sounds.
pub(crate) struct WheelOfDoom {
    spinning: Vec<Spinning>,
    /// The spinner's own effect and curse.
    pub(crate) effects: Effects,
    /// When a first press showed the price.
    armed: Option<Instant>,
    asked: Option<Instant>,
    /// When the player may spin again, by the client's count or the hub's refusal.
    cooldown_until: Option<Instant>,
    answer_seen: u64,
    /// When the price was last asked of the hub.
    read_at: Option<Instant>,
    /// The server and map the effects began on: another ends them.
    place: Option<(std::net::SocketAddr, String)>,
    cues: Vec<Cue>,
    blasts: Vec<Vec3>,
    sounds_registered: Option<u64>,
    meshes: [Option<usize>; 2],
    meshes_seen: usize,
    pub(crate) draw: draw::Draw,
    mirror: mirror::Mirror,
    /// A world shot's wheels, kept without a session.
    #[cfg(test)]
    shot: bool,
    /// How far into their spin a world shot's wheels stay, frame after frame.
    #[cfg(test)]
    shot_since: Duration,
}

impl Default for WheelOfDoom {
    fn default() -> Self {
        Self {
            spinning: Vec::with_capacity(MOST),
            effects: Effects::default(),
            armed: None,
            asked: None,
            cooldown_until: None,
            answer_seen: 0,
            read_at: None,
            place: None,
            cues: Vec::with_capacity(16),
            blasts: Vec::with_capacity(2),
            sounds_registered: None,
            meshes: [None; 2],
            meshes_seen: 0,
            draw: draw::Draw::default(),
            mirror: mirror::Mirror::default(),
            #[cfg(test)]
            shot: false,
            #[cfg(test)]
            shot_since: Duration::ZERO,
        }
    }
}

impl WheelOfDoom {
    /// The player's own wheel stands, a press waits for its second, or an effect or a
    /// curse runs: the Toys tab's switch shows on.
    pub(crate) fn running(&self) -> bool {
        self.armed.is_some_and(|armed| armed.elapsed() < CONFIRM)
            || self.spinning.iter().any(|wheel| wheel.mine)
            || self.effects.any()
    }

    /// The whole seconds left before the player may spin again.
    fn cooldown_left(&self, now: Instant) -> Option<u64> {
        let wait = self.cooldown_until?.checked_duration_since(now)?;
        (!wait.is_zero()).then(|| wait.as_secs_f32().ceil() as u64)
    }

    /// A wheel starts spinning at `now` for the player on `slot` (`mine`: the player's
    /// own), landing on `segment` for `outcome`; it stands at `at` facing `yaw` degrees.
    #[allow(clippy::too_many_arguments)]
    fn start(
        &mut self,
        slot: Option<u8>,
        mine: bool,
        outcome: &str,
        segment: usize,
        at: Vec3,
        yaw: f32,
        now: Instant,
    ) {
        // One wheel a spinner; the oldest goes when there are too many.
        self.spinning
            .retain(|wheel| wheel.slot != slot || slot.is_none());
        if self.spinning.len() >= MOST {
            self.spinning.remove(0);
        }
        let found = known(outcome).map(|(_, known)| known);
        let (title, line) = match found {
            Some(known) => (known.title.to_owned(), known.line.to_owned()),
            None => (
                outcome.replace('_', " ").to_uppercase(),
                "Nothing happens...".to_owned(),
            ),
        };
        self.spinning.push(Spinning {
            slot,
            mine,
            outcome: outcome.to_owned(),
            kind: found.map(|known| known.kind),
            segment: segment % SEGMENTS,
            at,
            yaw,
            started: now,
            pegs: 0,
            stopped: false,
            blasted: false,
            title,
            line,
        });
    }

    /// Move the wheels on to `now`: a tick for each peg passing, the stop and what it
    /// says, a blast after an `explode`, and wheels gone after their hold.
    fn tick(&mut self, now: Instant) -> Vec<Moment> {
        let mut moments = Vec::new();
        for wheel in &mut self.spinning {
            let elapsed = now.saturating_duration_since(wheel.started);
            let origin = Some((wheel.at + Vec3::Z * AXLE).to_array());
            let source = source(wheel.slot);
            if !wheel.stopped {
                let pegs = pegs_passed(wheel.angle(now));
                if pegs > wheel.pegs {
                    wheel.pegs = pegs;
                    self.cues.push(Cue {
                        path: TICK,
                        origin,
                        source,
                    });
                }
            }
            if !wheel.stopped && elapsed >= SPIN {
                wheel.stopped = true;
                self.cues.push(Cue {
                    path: STOP,
                    origin,
                    source,
                });
                let follow = match wheel.kind {
                    Some(Kind::Jackpot) => Some(FANFARE),
                    Some(kind) if kind.bad() => Some(LAUGH),
                    _ => None,
                };
                if let Some(path) = follow {
                    self.cues.push(Cue {
                        path,
                        origin,
                        source,
                    });
                }
                if wheel.mine {
                    moments.push(Moment::Stopped {
                        kind: wheel.kind,
                        title: wheel.title.clone(),
                        line: wheel.line.clone(),
                    });
                }
            }
            if wheel.kind == Some(Kind::Explode) && !wheel.blasted && elapsed >= SPIN + BLAST_DELAY
            {
                wheel.blasted = true;
                moments.push(Moment::Blast {
                    slot: wheel.slot,
                    at: wheel.at,
                });
                self.cues.push(Cue {
                    path: BLAST_SOUND,
                    origin,
                    source,
                });
            }
        }
        self.spinning.retain(|wheel| wheel.scale(now).is_some());
        moments
    }

    /// Everything of this server and map goes: the wheels and the effects.
    fn clear(&mut self) {
        self.spinning.clear();
        self.effects.clear();
        self.armed = None;
        self.asked = None;
    }

    /// The player's own view from `eye` at `target` with `up`, turned at `now` by the
    /// effect running (Upside down's roll, Drunk's sway): the new target and up. The aim
    /// is not turned, only what is seen.
    pub(crate) fn turn_view(
        &self,
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        now: Instant,
    ) -> (Vec3, Vec3) {
        let Some([roll, pitch, yaw]) = self.effects.view_turn(now) else {
            return (target, up);
        };
        let forward = (target - eye).normalize_or_zero();
        let right = forward.cross(up).normalize_or_zero();
        if forward == Vec3::ZERO || right == Vec3::ZERO {
            return (target, up);
        }
        let sway = Quat::from_axis_angle(up.normalize_or_zero(), yaw.to_radians())
            * Quat::from_axis_angle(right, pitch.to_radians());
        let forward = sway * forward;
        let up = Quat::from_axis_angle(forward, roll.to_radians()) * (sway * up);
        (eye + forward * target.distance(eye), up)
    }

    /// The models to find among the loaded ones.
    fn mesh_of(&self, model: &str) -> Option<usize> {
        let index = MODELS.iter().position(|known| *known == model)?;
        self.meshes[index]
    }
}

impl GpuState {
    fn wheel_of_doom_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// `toy_wheelofdoom`: the first press shows the price, a second within [`CONFIRM`]
    /// asks the hub to spin; or why it cannot now, in the centre of the screen.
    pub(crate) fn wheel_of_doom_command(&mut self) {
        let now = Instant::now();
        let wheel = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .wheel_of_doom
                .as_ref()
                .map(|wheel| (wheel.price, wheel.cooldown))
        })
        .flatten();
        if wheel.is_none() {
            self.read_wheel_of_doom(now);
        }
        let credits = crate::player_identity::with_snapshot(|snapshot| {
            snapshot.me.as_ref().map(|me| me.credits)
        })
        .flatten();
        let server = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(|session| session.server().to_string());
        let toy = &self.wheel_of_doom;
        let check = Check {
            online: server.is_some(),
            strikable: self.ragdoll.strikable(),
            cooldown: toy.cooldown_left(now),
            asked: toy
                .asked
                .is_some_and(|asked| now.saturating_duration_since(asked) < ASK_TIMEOUT),
            price: wheel.map(|(price, _)| price),
            credits,
        };
        if let Some(why) = refusal(check) {
            self.wheel_of_doom.armed = None;
            self.wheel_of_doom_centre(why);
            return;
        }
        let (Some(server), Some(_)) = (server, wheel) else {
            self.wheel_of_doom_centre(
                "Asking the hub for the WHEEL OF DOOM's price: try again in a moment".to_owned(),
            );
            return;
        };
        let armed = self
            .wheel_of_doom
            .armed
            .is_some_and(|armed| now.saturating_duration_since(armed) < CONFIRM);
        if !armed {
            // The price shows by the crosshair (`draw`); a second press pays it. From
            // the Collection, whose page covers the crosshair, in the centre too.
            self.wheel_of_doom.armed = Some(now);
            if self
                .client_menu
                .as_ref()
                .is_some_and(crate::menu::ClientMenu::is_visible)
                && let Some((price, _)) = wheel
            {
                self.wheel_of_doom_centre(format!(
                    "WHEEL OF DOOM: {price} credits. Press it again within 5 s to spin"
                ));
            }
            return;
        }
        self.wheel_of_doom.armed = None;
        if crate::player_identity::with_service(|service| service.spin_wheel_of_doom(server))
            .is_some()
        {
            self.wheel_of_doom.asked = Some(now);
        } else {
            self.wheel_of_doom_centre("Not connected to the SJK hub".to_owned());
        }
    }

    /// Ask the hub for the price and the segments, at most every 5 seconds.
    fn read_wheel_of_doom(&mut self, now: Instant) {
        if self
            .wheel_of_doom
            .read_at
            .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(5))
        {
            return;
        }
        self.wheel_of_doom.read_at = Some(now);
        let _ = crate::player_identity::with_service(|service| service.read_wheel_of_doom());
    }

    /// Where a wheel stands for a player at `from` (their origin) looking `yaw` degrees:
    /// [`PLACE`] before them (nearer when a wall is closer), on the floor.
    fn wheel_of_doom_spot(&mut self, from: Vec3, yaw: f32) -> Vec3 {
        let ahead = Vec3::new(yaw.to_radians().cos(), yaw.to_radians().sin(), 0.0);
        let eye = from + Vec3::Z * 16.0;
        let bounds = sjk_bsp::Aabb::new([-8.0; 3], [8.0; 3])
            .unwrap_or_else(|_| sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).expect("a point is a box"));
        let reach = self.bsp.trace_box_with(
            &mut self.trace_scratch,
            eye.to_array(),
            (eye + ahead * PLACE).to_array(),
            bounds,
            1,
        );
        let distance = if reach.start_solid {
            PLACE
        } else {
            (PLACE * reach.fraction - 24.0).max(PLACE_MIN)
        };
        let over = eye + ahead * distance;
        let floor = self.bsp.trace_box_with(
            &mut self.trace_scratch,
            over.to_array(),
            (over - Vec3::Z * 160.0).to_array(),
            sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).expect("a point is a box"),
            1,
        );
        if floor.fraction < 1.0 && !floor.start_solid {
            Vec3::from_array(floor.end_position)
        } else {
            over - Vec3::Z * 40.0
        }
    }

    /// The frame's WHEEL OF DOOM logic, after the looks: the hub's answer, other
    /// players' spins (`events`, this server's toy events), the wheels and the effects,
    /// which end on a new map or server, a disconnect or a death.
    pub(crate) fn update_wheel_of_doom(&mut self, events: &[sjk_identity::ToyEvent]) {
        #[cfg(test)]
        if self.wheel_of_doom.shot {
            self.find_wheel_of_doom_meshes();
            let started = Instant::now() - self.wheel_of_doom.shot_since;
            for wheel in &mut self.wheel_of_doom.spinning {
                wheel.started = started;
            }
            return;
        }
        let now = Instant::now();
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.wheel_of_doom.clear();
            self.wheel_of_doom.place = None;
            return;
        };
        let server = session.server();
        let own = u8::try_from(session.game_state().client_num).ok();
        let alive = session.latest_snapshot().player.health() > 0;
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map);
        let moved = self
            .wheel_of_doom
            .place
            .as_ref()
            .is_none_or(|(at_server, at_map)| *at_server != server || at_map != map);
        if moved {
            self.wheel_of_doom.clear();
            self.wheel_of_doom.place = Some((server, map.to_owned()));
        }
        if !alive {
            self.wheel_of_doom.effects.clear();
        }
        self.find_wheel_of_doom_meshes();
        self.read_wheel_of_doom_answer(now, own);
        // Other players' spins: the hub made the event, so it holds the outcome.
        for event in events {
            if event.toy != wire::TOY || Some(event.slot) == own {
                continue;
            }
            let Some(outcome) = wire::parse_event(&event.event) else {
                continue;
            };
            let time = self.presentation_clock.sample(now);
            let Some(presented) = self
                .live_world
                .entity(sjk_runtime::EntityId::new(u64::from(event.slot) + 1))
            else {
                continue;
            };
            let pose = presented.sample(time);
            let facing = Quat::from_array(pose.rotation) * Vec3::X;
            let yaw = facing.y.atan2(facing.x).to_degrees();
            let at = self.wheel_of_doom_spot(Vec3::from_array(pose.translation), yaw);
            let segment = self.wheel_of_doom_segment(outcome);
            self.wheel_of_doom.start(
                Some(event.slot),
                false,
                outcome,
                segment,
                at,
                yaw + 180.0,
                now,
            );
        }
        for moment in self.wheel_of_doom.tick(now) {
            self.wheel_of_doom_moment(moment, own, now);
        }
        if let Some(effect) = self.wheel_of_doom.effects.tick(now) {
            self.wheel_of_doom_centre(format!("The curse strikes: {}!", effect.hud()));
        }
        self.release_wheel_of_doom_mirror();
    }

    /// The segment a spin of `outcome` lands on, for another player's: by the hub's list,
    /// else the built-in order, else `nothing`'s.
    fn wheel_of_doom_segment(&self, outcome: &str) -> usize {
        crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .wheel_of_doom
                .as_ref()
                .and_then(|wheel| wheel.segment_of(outcome))
        })
        .flatten()
        .or_else(|| known(outcome).map(|(segment, _)| segment))
        .unwrap_or(1)
    }

    /// Act on the hub's last answer to a spin, once.
    fn read_wheel_of_doom_answer(&mut self, now: Instant, own: Option<u8>) {
        let seen = self.wheel_of_doom.answer_seen;
        let Some((serial, outcome, cooldown)) = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .wheel_of_doom_spin
                .as_ref()
                .filter(|(serial, _)| *serial != seen)
                .map(|(serial, outcome)| {
                    (
                        *serial,
                        outcome.clone(),
                        snapshot.wheel_of_doom.as_ref().map(|wheel| wheel.cooldown),
                    )
                })
        })
        .flatten() else {
            return;
        };
        self.wheel_of_doom.answer_seen = serial;
        let asked = self.wheel_of_doom.asked.take();
        match outcome {
            Ok(spin) => {
                let cooldown = cooldown
                    .filter(|seconds| *seconds > 0)
                    .unwrap_or(DEFAULT_COOLDOWN);
                self.wheel_of_doom.cooldown_until = Some(now + Duration::from_secs(cooldown));
                let from = self
                    .local_prediction
                    .predicted_state()
                    .map(|state| Vec3::from_array(state.origin))
                    .or_else(|| {
                        self.live_session.as_ref().map(|session| {
                            Vec3::from_array(session.latest_snapshot().player.origin())
                        })
                    })
                    .unwrap_or_default();
                let yaw = self.camera_yaw.to_degrees();
                let at = self.wheel_of_doom_spot(from, yaw);
                self.wheel_of_doom.start(
                    own,
                    true,
                    &spin.outcome,
                    spin.segment as usize,
                    at,
                    yaw + 180.0,
                    now,
                );
            }
            Err(refused) => {
                if let Some(left) = refused.left {
                    self.wheel_of_doom.cooldown_until = Some(now + Duration::from_secs(left));
                }
                if asked.is_some() {
                    self.wheel_of_doom_centre(refused.words);
                }
            }
        }
    }

    /// The player's own wheel stopped, or a wheel blew its spinner up.
    fn wheel_of_doom_moment(&mut self, moment: Moment, own: Option<u8>, now: Instant) {
        match moment {
            Moment::Stopped { kind, title, line } => {
                let text = match kind {
                    Some(Kind::Effect(effect)) => {
                        self.wheel_of_doom.effects.start(effect, now);
                        format!("{title} {line}")
                    }
                    Some(Kind::Curse) => {
                        self.wheel_of_doom.effects.curse(now, seed());
                        format!("{title} {line}")
                    }
                    Some(Kind::Jackpot) => format!("{title} In your Collection"),
                    Some(Kind::Explode) => title,
                    Some(Kind::Nothing) => line,
                    None => format!("{title}: {line}"),
                };
                self.wheel_of_doom_centre(text);
            }
            Moment::Blast { slot, at } => {
                let mine = slot.is_some() && slot == own;
                let stands = if mine {
                    self.local_prediction
                        .predicted_state()
                        .map(|state| Vec3::from_array(state.origin))
                } else {
                    slot.and_then(|slot| {
                        let time = self.presentation_clock.sample(now);
                        let presented = self
                            .live_world
                            .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))?;
                        Some(Vec3::from_array(presented.sample(time).translation))
                    })
                };
                self.wheel_of_doom.blasts.push(stands.unwrap_or(at));
                // The game's `kill`, as the mystery box's: never in a duel, with the flag
                // or wherever the Ragdoll could not strike.
                if mine
                    && self.ragdoll.strikable()
                    && let Some(session) = self.live_session.as_mut()
                    && let Err(error) = session.send_reliable_command(b"kill")
                {
                    crate::log::progress(format_args!("wheel of doom: kill not sent: {error}"));
                }
            }
        }
    }

    /// Find the wheel's models among the loaded ones, when more have loaded.
    fn find_wheel_of_doom_meshes(&mut self) {
        if self.wheel_of_doom.meshes_seen == self.object_meshes.len() {
            return;
        }
        self.wheel_of_doom.meshes_seen = self.object_meshes.len();
        for (slot, model) in MODELS.iter().enumerate() {
            self.wheel_of_doom.meshes[slot] = self.object_meshes.iter().position(|mesh| {
                mesh.appearance.variant.is_empty() && mesh.appearance.model == *model
            });
        }
    }

    /// Add this frame's wheels to the scene: each stand, and its wheel turned on its axle.
    pub(crate) fn append_wheel_of_doom_instances(&mut self) {
        let now = Instant::now();
        let toy = &self.wheel_of_doom;
        let (Some(stand), Some(spun)) = (toy.mesh_of(STAND_MODEL), toy.mesh_of(WHEEL_MODEL)) else {
            return;
        };
        for wheel in &toy.spinning {
            let Some(scale) = wheel.scale(now) else {
                continue;
            };
            let facing = Quat::from_array(sjk_client::legacy_angles_to_quaternion([
                0.0, wheel.yaw, 0.0,
            ]));
            self.object_groups[stand].push(ActorInstance::new(
                wheel.at.to_array(),
                facing.to_array(),
                [scale; 3],
            ));
            self.object_groups[spun].push(ActorInstance::new(
                (wheel.at + Vec3::Z * AXLE * scale).to_array(),
                (facing * wheel_turn(wheel.angle(now))).to_array(),
                [scale; 3],
            ));
        }
    }

    /// Play the wheels' sounds and the quacks; the sounds are handed to the audio the
    /// first time there is one.
    pub(crate) fn play_wheel_of_doom_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let quack = self.wheel_of_doom.effects.take_quack();
        let Some(audio) = audio.as_mut() else {
            self.wheel_of_doom.cues.clear();
            self.wheel_of_doom.blasts.clear();
            return;
        };
        if let Some(vfs) = self.vfs.clone() {
            for stands in self.wheel_of_doom.blasts.drain(..) {
                crate::effect_runtime::spawn_effect(
                    &mut self.particles,
                    &mut self.effect_aux,
                    &mut self.effects,
                    &vfs,
                    BLAST_EFFECT,
                    stands + Vec3::Z * 8.0,
                    Instant::now(),
                    seed(),
                    0,
                    &mut None,
                    crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]),
                );
            }
        }
        if self.wheel_of_doom.cues.is_empty() && !quack {
            return;
        }
        if self.wheel_of_doom.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(
                vfs,
                [
                    TICK,
                    STOP,
                    FANFARE,
                    QUACK,
                    LAUGH,
                    LAUGH_FALLBACK,
                    BLAST_SOUND,
                ],
            );
            self.wheel_of_doom.sounds_registered = Some(audio.generation());
        }
        if quack {
            audio.play_toy(
                crate::toy_sounds::Toy::WheelOfDoom,
                QUACK,
                1.0,
                None,
                source(None),
                sjk_audio::ChannelId(1),
            );
        }
        for cue in self.wheel_of_doom.cues.drain(..) {
            let path = if cue.path == LAUGH && !audio.has_sound(LAUGH) {
                LAUGH_FALLBACK
            } else {
                cue.path
            };
            audio.play_toy(
                crate::toy_sounds::Toy::WheelOfDoom,
                path,
                1.0,
                cue.origin,
                cue.source,
                // `CHAN_AUTO`: the ticks, the stop and the laugh overlap.
                sjk_audio::ChannelId(0),
            );
        }
    }

    /// The saber as the player state says, for No saber.
    pub(crate) fn wheel_of_doom_saber(player: &sjk_protocol::PlayerState) -> Saber {
        Saber {
            weapon: player.weapon(),
            holstered: player.saber_holstered(),
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: a wheel at each of `wheels` (origin and facing yaw), spun
    /// `since` ago onto segment `n` for the `n`th.
    pub(crate) fn shot_wheels_of_doom(&mut self, wheels: &[([f32; 3], f32)], since: Duration) {
        let started = Instant::now() - since;
        self.wheel_of_doom.shot = true;
        self.wheel_of_doom.shot_since = since;
        self.wheel_of_doom.spinning.clear();
        for (index, (origin, yaw)) in wheels.iter().enumerate() {
            let outcome = OUTCOMES[index % SEGMENTS].id;
            self.wheel_of_doom.start(
                Some(index as u8),
                false,
                outcome,
                index,
                Vec3::from_array(*origin),
                *yaw,
                started,
            );
            if let Some(wheel) = self.wheel_of_doom.spinning.last_mut() {
                wheel.stopped = since >= SPIN;
            }
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: the wheel's meshes found, and the wheels kept.
    pub(crate) fn shot_wheel_of_doom_state(&self) -> ([Option<usize>; 2], usize) {
        (self.wheel_of_doom.meshes, self.wheel_of_doom.spinning.len())
    }
}

/// How long a spin takes, for the world shots.
#[cfg(test)]
pub(crate) const SHOT_SPIN: Duration = SPIN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outcomes_are_the_hubs_sixteen_in_order() {
        let ids: Vec<_> = OUTCOMES.iter().map(|outcome| outcome.id).collect();
        assert_eq!(
            ids,
            [
                "mythical",
                "nothing",
                "reverse_keys",
                "invert_mouse",
                "mirror",
                "upside_down",
                "blind",
                "drunk",
                "tiny_fov",
                "huge_fov",
                "no_saber",
                "quack",
                "walk",
                "bunny",
                "explode",
                "curse"
            ]
        );
        // Every effect has its segment.
        for effect in Effect::ALL {
            assert!(
                OUTCOMES
                    .iter()
                    .any(|outcome| outcome.kind == Kind::Effect(effect))
            );
        }
    }

    #[test]
    fn each_outcome_stops_its_segment_under_the_pointer() {
        assert_eq!(segment_angle(0), 0.0);
        assert_eq!(segment_angle(1), 337.5);
        assert_eq!(segment_angle(4), 270.0);
        for segment in 0..SEGMENTS {
            // Segment k's middle is painted k segments clockwise of the top (y right, z
            // up, seen from +X): turned by the wheel's stop, it stands at the top.
            let painted = (segment as f32 * SEGMENT_DEGREES).to_radians();
            let middle = Vec3::new(0.0, painted.sin(), painted.cos());
            let stopped = wheel_turn(wheel_angle(SPIN, segment)) * middle;
            assert!(stopped.abs_diff_eq(Vec3::Z, 1e-3), "{segment}: {stopped}");
        }
    }

    #[test]
    fn the_spin_slows_to_a_stop_on_the_segment() {
        let segment = 7;
        let step = Duration::from_millis(100);
        let mut last = 0.0;
        let mut last_speed = f32::MAX;
        let mut at = Duration::ZERO;
        while at < SPIN {
            at += step;
            let angle = wheel_angle(at, segment);
            let speed = angle - last;
            assert!(speed >= 0.0 && speed <= last_speed + 1e-3, "{at:?}");
            last = angle;
            last_speed = speed;
        }
        assert_eq!(
            wheel_angle(SPIN, segment),
            TURNS * 360.0 + segment_angle(segment)
        );
        assert_eq!(wheel_angle(SPIN * 2, segment), wheel_angle(SPIN, segment));
        assert!(wheel_angle(SPIN, segment) >= TURNS * 360.0);
    }

    #[test]
    fn a_spin_ticks_stops_says_its_outcome_then_goes() {
        let mut toy = WheelOfDoom::default();
        let start = Instant::now();
        toy.start(Some(3), true, "mirror", 4, Vec3::ZERO, 0.0, start);
        assert!(toy.tick(start + Duration::from_millis(500)).is_empty());
        assert!(toy.cues.iter().any(|cue| cue.path == TICK));
        toy.cues.clear();
        let moments = toy.tick(start + SPIN);
        assert_eq!(
            moments,
            [Moment::Stopped {
                kind: Some(Kind::Effect(Effect::Mirror)),
                title: "MIRROR".to_owned(),
                line: "The world is mirrored for 30 s".to_owned()
            }]
        );
        // The last peg may tick on the stop's frame.
        let paths: Vec<_> = toy.cues.iter().map(|cue| cue.path).collect();
        assert!(paths.ends_with(&[STOP, LAUGH]), "{paths:?}");
        assert!(toy.tick(start + SPIN + HOLD).is_empty());
        assert_eq!(toy.spinning.len(), 1);
        toy.tick(start + SPIN + HOLD + SHRINK);
        assert!(toy.spinning.is_empty());
    }

    #[test]
    fn the_jackpot_fanfares_and_explode_blows_up_later() {
        let mut toy = WheelOfDoom::default();
        let start = Instant::now();
        toy.start(Some(1), true, "mythical", 0, Vec3::ZERO, 0.0, start);
        toy.start(Some(2), false, "explode", 14, Vec3::X, 0.0, start);
        let moments = toy.tick(start + SPIN);
        assert_eq!(moments.len(), 1, "only the player's own is told");
        assert!(toy.cues.iter().any(|cue| cue.path == FANFARE));
        let blast = toy.tick(start + SPIN + BLAST_DELAY);
        assert_eq!(
            blast,
            [Moment::Blast {
                slot: Some(2),
                at: Vec3::X
            }]
        );
        assert!(toy.tick(start + SPIN + BLAST_DELAY * 2).is_empty(), "once");
    }

    #[test]
    fn an_unknown_outcome_is_shown_by_name_and_does_nothing() {
        let mut toy = WheelOfDoom::default();
        let start = Instant::now();
        toy.start(Some(1), true, "big_party", 9, Vec3::ZERO, 0.0, start);
        let moments = toy.tick(start + SPIN);
        assert_eq!(
            moments,
            [Moment::Stopped {
                kind: None,
                title: "BIG PARTY".to_owned(),
                line: "Nothing happens...".to_owned()
            }]
        );
        assert!(!toy.cues.iter().any(|cue| cue.path == LAUGH));
        assert!(!toy.effects.any());
    }

    #[test]
    fn a_spin_is_refused_in_a_duel_in_the_cooldown_or_without_credits() {
        let ready = Check {
            online: true,
            strikable: true,
            price: Some(100),
            credits: Some(250),
            ..Check::default()
        };
        assert_eq!(refusal(ready), None);
        let duel = Check {
            strikable: false,
            ..ready
        };
        assert!(refusal(duel).is_some_and(|why| why.contains("duels")));
        let cooling = Check {
            cooldown: Some(42),
            ..ready
        };
        assert_eq!(refusal(cooling).as_deref(), Some("Your next spin in 42 s"));
        let poor = Check {
            credits: Some(99),
            ..ready
        };
        assert!(refusal(poor).is_some_and(|why| why.contains("not enough")));
        assert!(refusal(Check::default()).is_some_and(|why| why.contains("game server")));
        let mut toy = WheelOfDoom::default();
        let now = Instant::now();
        toy.cooldown_until = Some(now + Duration::from_millis(41_200));
        assert_eq!(toy.cooldown_left(now), Some(42));
        assert_eq!(toy.cooldown_left(now + Duration::from_secs(42)), None);
    }

    #[test]
    fn a_new_map_clears_the_wheels_and_the_effects() {
        let mut toy = WheelOfDoom::default();
        let now = Instant::now();
        toy.start(Some(1), true, "walk", 12, Vec3::ZERO, 0.0, now);
        toy.effects.start(Effect::Walk, now);
        toy.clear();
        assert!(toy.spinning.is_empty() && !toy.effects.any());
    }
}
