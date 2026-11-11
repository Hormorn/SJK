//! The cardboard box, SJK's own toy (`toy_box`; Sol's idea, 16/10/2026): a shipping
//! carton the player hides in, Metal Gear style. While it is on, the player crouches
//! in it (the user commands hold crouch, [`CardboardBox::shape_command`]) and its
//! music plays on them. No game server knows of it: it travels through the SJK hub as
//! a toy in the player's look (`looks.rs`, [`TOY`]), so other SJK players on the server
//! see the box over that player and hear the music; players on stock clients see a
//! crouching player.
//!
//! Another SJK player who presses `+use` facing a boxed player within reach lifts the
//! box ([`pick_target`]): the "spotted" sound plays, the box goes at once on the
//! lifter's screen, and the hub relays a toy event ([`EVENT_SPOTTED`]) on which the
//! boxed player's client takes it off (and tells its look) and every other SJK client
//! hides it and plays the sound. A player who dies in the box plays its death sound
//! where they fell, on every SJK client that sees it; the box is lost with the life
//! (off at the respawn). It is out at every start, like Illuminate.
//!
//! A box put on plays its sound ([`EQUIP`]). A box taken off or lifted, its player
//! still standing, tips over on its side ([`Fallen`]: away from the lifter, or the way
//! its player faced), lands with a thud ([`DROP`]), lies there a few seconds and goes.
//!
//! The carton's model, picture, shader and icon are bundled ([`FILES`], made by
//! `scripts/cardboard_box_assets.py`) and mounted below all game data, so a PK3 with
//! the same paths replaces them. The sounds ([`MUSIC`], [`EQUIP`], [`SPOTTED`],
//! [`DROP`], [`DEATH`]) are game paths that SJK does not ship; where they are missing
//! nothing plays.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec2, Vec3};
use sjk_protocol::{EntityState, PlayerState, UserCommand};
use sjk_vfs::{VfsError, VirtualFileSystem};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The toy's console command (a toy is a `toy_<name>` command).
pub(crate) const COMMAND: &str = "toy_box";
/// The toy's id in a look's `toys` and in toy events (`PROTOCOL.md`, "Looks").
pub(crate) const TOY: &str = "cardboard_box";
/// The toy event of a box lifted off a player.
pub(crate) const EVENT_SPOTTED: &str = "spotted";
pub(crate) const MODEL: &str = "models/sjk/cardboard_box.md3";
/// The music that plays on a boxed player, looping.
pub(crate) const MUSIC: &str = "sound/sjk/cardboard_box/music.wav";
/// The sound of a box lifted off a player.
pub(crate) const SPOTTED: &str = "sound/sjk/cardboard_box/spotted.wav";
/// The sound of a player who dies in the box.
pub(crate) const DEATH: &str = "sound/sjk/cardboard_box/death.wav";
/// The sound of a box put on.
pub(crate) const EQUIP: &str = "sound/sjk/cardboard_box/equip.wav";
/// The sound of a fallen box landing on its side.
pub(crate) const DROP: &str = "sound/sjk/cardboard_box/drop.wav";

/// A bundled file's bytes, by its name in `assets/cardboard_box`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/cardboard_box/", $name))
    };
}

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 4] = [
    (MODEL, bundled!("cardboard_box.md3")),
    (
        "models/sjk/cardboard_box.jpg",
        bundled!("cardboard_box.jpg"),
    ),
    (
        "shaders/sjk_cardboard_box.shader",
        bundled!("cardboard_box.shader"),
    ),
    ("gfx/sjk/toy_box.png", bundled!("toy_box.png")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    vfs.mount_memory("SJK cardboard box", FILES.iter().copied())?;
    Ok(())
}

const SLOTS: usize = crate::looks::SLOTS;
/// `BUTTON_USE`, `+use`.
const BUTTON_USE: u16 = 32;
/// The crouch a held `+movedown` sends.
const CROUCH: i8 = -127;
/// How far from the lifter's origin a box can be lifted: the stock `+use` reach
/// (`USE_DISTANCE` 64, `g_active.c`) and a little, as the box is wider than the player.
pub(crate) const REACH: f32 = 96.0;
/// How far off the lifter's facing a box can be lifted: the cosine of 60 degrees.
const FACING: f32 = 0.5;
/// How long a box lifted here stays hidden while the boxed player's look still says it
/// is on (their client takes it off on the hub's event and tells its look): past this
/// it shows again, as their look says.
const LIFT_HOLD: Duration = Duration::from_secs(15);
/// From the player's origin down to their feet (`DEFAULT_MINS_2`).
const FEET: f32 = -24.0;
/// Half the carton's width: from its middle to the edge it tips over.
const HALF_WIDTH: f32 = 23.0;
/// How long a box takes to tip over onto its side.
const FALL: Duration = Duration::from_millis(450);
/// How long a fallen box lies there.
const REST: Duration = Duration::from_secs(5);
/// How long a fallen box takes to shrink away.
const SHRINK: Duration = Duration::from_millis(400);
/// The most fallen boxes at once; a new one replaces the oldest.
const FALLEN_MAX: usize = 8;
/// While a fallen box is the player's own in first person, it shows in mirrors only
/// until it has tipped this far (radians), out of the view inside it.
const OWN_CLEAR: f32 = 0.5;
/// After joining a server, the boxes in the looks that arrive are not put on: no sound.
const SETTLE: Duration = Duration::from_secs(5);
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 8;
const PW_CLOAKED: u32 = 1 << 11;
/// `PMF_FOLLOW`: the player state is that of the player being followed.
const PMF_FOLLOW: u16 = 4096;

/// A one-shot sound to play: on `slot`, at `origin` (`None`: the player's own, in both
/// ears).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cue {
    pub(crate) path: &'static str,
    pub(crate) slot: u8,
    pub(crate) origin: Option<[f32; 3]>,
}

/// The music on a boxed player this frame: `slot`, at `origin` (`None`: the player's
/// own, in both ears).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Music {
    pub(crate) slot: u8,
    pub(crate) origin: Option<[f32; 3]>,
}

/// A box tipping over, or lying on its side, where a player took it off or was spotted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fallen {
    /// The middle of its bottom when it started to fall, and its yaw (radians).
    feet: Vec3,
    yaw: f32,
    /// The way it falls, level and of unit length.
    toward: Vec2,
    /// Whose it was, for the sound.
    slot: u8,
    since: Instant,
    /// The player's own seen in first person: in mirrors only until [`OWN_CLEAR`].
    own_view: bool,
    /// Its thud was played.
    landed: bool,
}

impl Fallen {
    /// Where it is at `now`: its model's place and the angle tipped, or `None` once gone.
    fn pose(&self, now: Instant) -> Option<(ActorInstance, f32)> {
        let age = now.saturating_duration_since(self.since).as_secs_f32();
        let fall = FALL.as_secs_f32();
        let shrink = (age - fall - REST.as_secs_f32()) / SHRINK.as_secs_f32();
        if shrink >= 1.0 {
            return None;
        }
        // Falling as under gravity: slow at first, fast at the end.
        let t = (age / fall).min(1.0);
        let angle = std::f32::consts::FRAC_PI_2 * t * t;
        let toward = self.toward.extend(0.0);
        let tilt = Quat::from_axis_angle(Vec3::Z.cross(toward), angle);
        let pivot = self.feet + toward * HALF_WIDTH;
        let position = pivot + tilt * (self.feet - pivot);
        let mut instance = ActorInstance::new(
            position.to_array(),
            (tilt * Quat::from_rotation_z(self.yaw)).to_array(),
            [1.0 - shrink.max(0.0); 3],
        );
        if self.own_view && angle < OWN_CLEAR {
            instance.view_flags |= 1;
        }
        Some((instance, angle))
    }
}

/// A slot this frame, as [`CardboardBox::frame`] reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Seen {
    /// The carton drawn over the player: its origin (the player's) and yaw (radians).
    carton: Option<([f32; 3], f32)>,
    /// Their look (or, for the player's own, the toy) has the box on.
    worn: bool,
    /// Where the game draws the player alive, if it does.
    drawn: Option<[f32; 3]>,
}

/// The toy's state: the player's own box, the boxes lifted here, who was seen dead,
/// what was drawn the last frame, the fallen boxes and the sounds to play. Fixed-size
/// but for the short lists, which keep their room: a frame allocates nothing.
pub(crate) struct CardboardBox {
    /// The player's own box is on.
    on: bool,
    /// The user commands hold crouch: the box is on and the player plays.
    crouching: bool,
    /// The player flies the free camera or `fakenoclip` (hub report #152): the
    /// commands' `upmove` steers the flight, so the box leaves it alone, and the
    /// player's own carton stays with the body the server holds.
    flying: bool,
    /// Whether the player was alive at the last frame (`None` out of a game).
    own_alive: Option<bool>,
    /// `+use` went down in a user command since the last frame.
    use_pressed: bool,
    /// `+use` was down in the last user command.
    use_held: bool,
    /// Other players' boxes lifted here, and when.
    lifted: [Option<Instant>; SLOTS],
    /// Who lifted the player's own box last, and when that was heard (Surprise!).
    lifted_by: Option<(u16, Instant)>,
    /// The slots seen dead in the last snapshot (bits).
    dead: u64,
    /// The way a lifted box is pushed, by slot, until it falls.
    pushed: [Option<Vec2>; SLOTS],
    /// The cartons drawn the last frame, by slot: origin and yaw.
    shown: [Option<([f32; 3], f32)>; SLOTS],
    /// Who had the box on the last frame (bits).
    worn: u64,
    /// When the game began on this server (the looks' boxes settle first, [`SETTLE`]).
    joined: Option<Instant>,
    /// The boxes tipping over or lying on their side.
    fallen: Vec<Fallen>,
    /// Sounds to play, taken by the audio each frame.
    cues: Vec<Cue>,
    /// The music this frame.
    music: Vec<Music>,
    /// The audio output ([`crate::GameAudio::generation`]) its sounds were registered
    /// with: a new output (`snd_restart`) registers them again.
    sounds_registered: Option<u64>,
    /// Cartons to draw without a session, for the world shots: origin and yaw.
    #[cfg(test)]
    pub(crate) shot_cartons: Vec<([f32; 3], f32)>,
    /// Fallen boxes to draw without a session, for the world shots: origin, yaw, the
    /// way they fall (radians) and how long ago they started.
    #[cfg(test)]
    pub(crate) shot_fallen: Vec<([f32; 3], f32, f32, Duration)>,
}

impl Default for CardboardBox {
    fn default() -> Self {
        Self {
            on: false,
            crouching: false,
            flying: false,
            own_alive: None,
            use_pressed: false,
            use_held: false,
            lifted: [None; SLOTS],
            lifted_by: None,
            dead: 0,
            pushed: [None; SLOTS],
            shown: [None; SLOTS],
            worn: 0,
            joined: None,
            fallen: Vec::with_capacity(FALLEN_MAX),
            cues: Vec::with_capacity(8),
            music: Vec::with_capacity(8),
            sounds_registered: None,
            #[cfg(test)]
            shot_cartons: Vec::new(),
            #[cfg(test)]
            shot_fallen: Vec::new(),
        }
    }
}

impl CardboardBox {
    /// `toy_box`: put the box on, or take it off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// Put it on (`true`) or take it off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the player's own box is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Whether the player hides in their box: on, alive and playing (Tactical
    /// Espionage counts this time, `achievements/tracker.rs`).
    pub(crate) fn hiding(&self) -> bool {
        self.crouching && !self.flying
    }

    /// Hold crouch in `command` while the box is on and the player plays, as a held
    /// `+movedown` would (over a jump), and note a `+use` press for [`Self::update`].
    pub(crate) fn shape_command(&mut self, command: &mut UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
        if self.crouching && !self.flying {
            command.up_move = CROUCH;
        }
    }

    /// Whether the player flies the free camera or `fakenoclip` (hub report #152).
    /// While flying, [`Self::shape_command`] holds no crouch: a held crouch was the
    /// flight's `upmove` and sank the camera. The server stands the body up while
    /// flying (the free camera's talk balloon); SJK clients draw it crouched in the
    /// box all the same ([`crouched_in_box`]).
    pub(crate) fn set_flying(&mut self, flying: bool) {
        self.flying = flying;
    }

    /// Hold crouch in a command that is only shown, never sent (the prediction's
    /// preview between commands).
    pub(crate) fn shape_preview(&self, command: &mut UserCommand) {
        if self.crouching && !self.flying {
            command.up_move = CROUCH;
        }
    }

    /// The sounds to play since the last call.
    pub(crate) fn take_cues(&mut self) -> std::vec::Drain<'_, Cue> {
        self.cues.drain(..)
    }

    /// The music on boxed players this frame.
    pub(crate) fn music(&self) -> &[Music] {
        &self.music
    }

    /// Who lifted the player's own box last, and when that was heard.
    pub(crate) fn lifted_by(&self) -> Option<(u16, Instant)> {
        self.lifted_by
    }

    /// Whether the box of `slot` (not the player's own) was lifted here and is hidden
    /// while their look catches up.
    pub(crate) fn lifted(&self, slot: usize) -> bool {
        self.lifted.get(slot).is_some_and(Option::is_some)
    }

    /// The player's own life: alive (`Some(true)`), dead, or out of a game (`None`).
    /// Dying in the box plays its death sound; the box is lost with the life, off when
    /// the player is alive again.
    fn own_life(&mut self, alive: Option<bool>, slot: Option<u8>) {
        match (self.own_alive, alive) {
            (Some(true), Some(false)) if self.on => self.cues.push(Cue {
                path: DEATH,
                slot: slot.unwrap_or_default(),
                origin: None,
            }),
            (Some(false), Some(true)) if self.on => self.on = false,
            _ => {}
        }
        self.own_alive = alive;
        self.crouching = self.on && alive == Some(true);
    }

    /// A toy event relayed for this server, from a sender (whose claim matches what the
    /// game shows) about `target`; `own` is the player's slot, `origin` where the target
    /// stands if the game draws them, `boxed` whether this client shows their box,
    /// `toward` the way from the sender to the target (the box falls that way).
    fn spotted(
        &mut self,
        target: u8,
        own: Option<u8>,
        origin: Option<[f32; 3]>,
        boxed: bool,
        toward: Option<Vec2>,
        now: Instant,
    ) {
        if let Some(pushed) = self.pushed.get_mut(usize::from(target)) {
            *pushed = toward;
        }
        if Some(target) == own {
            if self.on {
                self.on = false;
                self.crouching = false;
                self.cues.push(Cue {
                    path: SPOTTED,
                    slot: target,
                    origin: None,
                });
            }
            return;
        }
        let Some(lifted) = self.lifted.get_mut(usize::from(target)) else {
            return;
        };
        if boxed && lifted.is_none() {
            self.cues.push(Cue {
                path: SPOTTED,
                slot: target,
                origin: origin.or(Some([0.0; 3])),
            });
        }
        *lifted = Some(now);
    }

    /// Lift the box of `target` here, standing at `origin`, from `from`: the sound, and
    /// the box falls away from the lifter.
    fn lift(&mut self, target: u8, origin: [f32; 3], from: [f32; 3], now: Instant) {
        if let Some(lifted) = self.lifted.get_mut(usize::from(target)) {
            *lifted = Some(now);
            self.pushed[usize::from(target)] = level_way(from, origin);
            self.cues.push(Cue {
                path: SPOTTED,
                slot: target,
                origin: Some(origin),
            });
        }
    }

    /// Forget a lift once the slot's look no longer has the box (`worn` says who has it
    /// on), or after [`LIFT_HOLD`].
    fn expire_lifts(&mut self, worn: impl Fn(usize) -> bool, now: Instant) {
        for (slot, lifted) in self.lifted.iter_mut().enumerate() {
            if lifted.is_some_and(|at| !worn(slot) || now.saturating_duration_since(at) > LIFT_HOLD)
            {
                *lifted = None;
            }
        }
    }

    /// Note who is dead in this snapshot (`dead` bits over the slots present in
    /// `seen`): a boxed player (`boxed`) newly dead plays the death sound at `origin`.
    fn deaths(
        &mut self,
        seen: u64,
        dead: u64,
        boxed: impl Fn(usize) -> bool,
        origin: impl Fn(usize) -> [f32; 3],
    ) {
        let newly = dead & !self.dead & seen;
        for slot in 0..SLOTS {
            if newly & (1 << slot) != 0 && boxed(slot) {
                self.cues.push(Cue {
                    path: DEATH,
                    slot: slot as u8,
                    origin: Some(origin(slot)),
                });
            }
        }
        self.dead = (self.dead & !seen) | (dead & seen);
    }

    /// Every other player's box is forgotten: no game, or another server.
    fn clear_others(&mut self) {
        self.lifted = [None; SLOTS];
        self.dead = 0;
        self.pushed = [None; SLOTS];
        self.shown = [None; SLOTS];
        self.worn = 0;
        self.joined = None;
        self.fallen.clear();
    }

    /// This frame's boxes, by slot (`own` the player's, seen in first person or not):
    /// a box put on plays its sound where its player is drawn (the player's own, in
    /// both ears; others' only once the looks settled after joining); a carton drawn
    /// the last frame and gone while its player is still drawn, taken off or lifted,
    /// tips over on its side.
    fn frame(
        &mut self,
        seen: &[Seen; SLOTS],
        own: Option<usize>,
        first_person: bool,
        now: Instant,
    ) {
        let joined = *self.joined.get_or_insert(now);
        let settled = now.saturating_duration_since(joined) >= SETTLE;
        for (slot, seen) in seen.iter().enumerate() {
            let bit = 1_u64 << slot;
            let mine = Some(slot) == own;
            if seen.worn && self.worn & bit == 0 {
                if mine {
                    self.cues.push(Cue {
                        path: EQUIP,
                        slot: slot as u8,
                        origin: None,
                    });
                } else if let Some(origin) = seen.drawn.filter(|_| settled) {
                    self.cues.push(Cue {
                        path: EQUIP,
                        slot: slot as u8,
                        origin: Some(origin),
                    });
                }
            }
            let pushed = self.pushed[slot];
            if let Some((origin, yaw)) = self.shown[slot]
                && seen.carton.is_none()
                && seen.drawn.is_some()
                && (pushed.is_some() || !seen.worn)
            {
                if self.fallen.len() == FALLEN_MAX {
                    self.fallen.remove(0);
                }
                self.fallen.push(Fallen {
                    feet: Vec3::from_array(origin) + Vec3::Z * FEET,
                    yaw,
                    toward: pushed.unwrap_or_else(|| Vec2::from_angle(yaw)),
                    slot: slot as u8,
                    since: now,
                    own_view: mine && first_person,
                    landed: false,
                });
                self.pushed[slot] = None;
            }
            if seen.carton.is_none() && !seen.worn {
                self.pushed[slot] = None;
            }
            self.shown[slot] = seen.carton;
            self.worn = if seen.worn {
                self.worn | bit
            } else {
                self.worn & !bit
            };
        }
    }

    /// The fallen boxes at `now`: their thuds as they land, the gone ones forgotten, and
    /// the rest handed to `draw`.
    fn fallen_frame(&mut self, now: Instant, mut draw: impl FnMut(ActorInstance)) {
        let cues = &mut self.cues;
        self.fallen.retain_mut(|fallen| {
            let Some((instance, angle)) = fallen.pose(now) else {
                return false;
            };
            if !fallen.landed && angle >= std::f32::consts::FRAC_PI_2 {
                fallen.landed = true;
                cues.push(Cue {
                    path: DROP,
                    slot: fallen.slot,
                    origin: Some(instance.position),
                });
            }
            draw(instance);
            true
        });
    }
}

/// The level way from `from` to `to`, of unit length, if they are apart on the level.
fn level_way(from: [f32; 3], to: [f32; 3]) -> Option<Vec2> {
    let way = (Vec3::from_array(to) - Vec3::from_array(from)).truncate();
    (way.length_squared() > 1.0).then(|| way.normalize())
}

/// The player `+use` lifts the box of, if any: the nearest of `boxed` (slot and origin)
/// within [`REACH`] of `origin` and in front of `yaw` (radians), on the level plane.
pub(crate) fn pick_target(
    origin: Vec3,
    yaw: f32,
    boxed: impl IntoIterator<Item = (u8, Vec3)>,
) -> Option<(u8, Vec3)> {
    let facing = Vec2::from_angle(yaw);
    boxed
        .into_iter()
        .filter_map(|(slot, at)| {
            let distance = at.distance(origin);
            let flat = (at - origin).truncate().normalize_or_zero();
            (distance <= REACH && (flat == Vec2::ZERO || flat.dot(facing) >= FACING))
                .then_some((distance, slot, at))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, slot, at)| (slot, at))
}

/// Whether the game draws a player entity: alive, not hidden and not cloaked.
fn drawn(entity: &EntityState) -> bool {
    entity.entity_type() == ET_PLAYER
        && entity.e_flags() & (EF_DEAD | EF_NODRAW) == 0
        && entity.powerups() & PW_CLOAKED == 0
}

/// Whether the player whose state is `player` is alive and in the match (not
/// spectating, nor at the intermission).
fn alive(player: &PlayerState) -> bool {
    // `pmtype_t`: PM_SPECTATOR 4, PM_DEAD 5, PM_INTERMISSION 7, PM_SPINTERMISSION 8.
    player.health() > 0 && !matches!(player.movement_type(), 4 | 5 | 7 | 8)
}

/// Whether `camera` is in a carton on a player standing at `origin`: within its
/// width either way of any yaw, from its open bottom to a little over its top.
fn inside_carton(camera: [f32; 3], origin: [f32; 3]) -> bool {
    let reach = HALF_WIDTH * std::f32::consts::SQRT_2 + 4.0;
    let [dx, dy, dz] = [0, 1, 2].map(|axis| camera[axis] - origin[axis]);
    dx.hypot(dy) <= reach && (FEET..=FEET + 48.0).contains(&dz)
}

/// The slots whose player is in a drawn carton, set once a frame by
/// [`GpuState::note_boxed_players`] before the actor poses read it.
static BOXED: AtomicU64 = AtomicU64::new(0);

/// Whether the player in `slot` is in a drawn carton, so standing there is drawn
/// crouched. The box asks the server to crouch, but `PmoveSingle` zeroes a talking
/// player's moves, `upmove` included (EternalJK `codemp/game/bg_pmove.c:12441-12448` at
/// a40e793; JoF EJK `bg_pmove.c:13108-13115` at 390f756), so the server stands up a
/// boxed player who opens the chat or flies the free camera; the balloon stays (it keeps
/// them from being attacked), and every SJK client poses a boxed player who stands as
/// crouching (`BOTH_CROUCH1IDLE`, `actor_pose_steps.rs`; Dave's test of #152,
/// 28/10/2026). Any other move (a lunge, a kick, a roll, a knockdown) plays as sent. A
/// crouch keeps the player's origin, so the pose sits where a real crouch would.
pub(crate) fn crouched_in_box(slot: usize) -> bool {
    slot < SLOTS && BOXED.load(Ordering::Relaxed) & (1 << slot) != 0
}

/// `CROUCH_VIEWHEIGHT` (`bg_public.h`): a crouching player's eye above the origin.
const CROUCH_VIEWHEIGHT: f32 = 12.0;

/// How far a boxed player's eye at `view_height` drops to crouch height, if it is
/// above it ([`GpuState::keep_boxed_eye`]).
fn eye_drop(view_height: f32) -> Option<f32> {
    (view_height > CROUCH_VIEWHEIGHT).then_some(view_height - CROUCH_VIEWHEIGHT)
}

/// The carton over a player standing at `origin` (their entity's), facing `yaw`
/// (radians).
fn carton(origin: [f32; 3], yaw: f32) -> ActorInstance {
    ActorInstance::new(
        [origin[0], origin[1], origin[2] + FEET],
        Quat::from_rotation_z(yaw).to_array(),
        [1.0; 3],
    )
}

impl GpuState {
    /// `toy_box`: put the box on, or take it off.
    pub(crate) fn toggle_cardboard_box(&mut self) {
        self.cardboard_box.toggle();
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            crate::toy_switch_words::switched("Cardboard box", self.cardboard_box.on()),
            None,
            Instant::now(),
        );
    }

    /// The frame's toy logic, after the looks: the own life, the hub's events (`events`,
    /// this server's toy events), a `+use` press and the deaths in the box.
    pub(crate) fn update_cardboard_box(&mut self, now: Instant, events: &[sjk_identity::ToyEvent]) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.cardboard_box.own_life(None, None);
            self.cardboard_box.use_pressed = false;
            self.cardboard_box.clear_others();
            return;
        };
        let snapshot = session.latest_snapshot();
        let game_state = session.game_state();
        let own = u8::try_from(game_state.client_num).ok();
        let player = &snapshot.player;
        let own_playing = alive(player) && player.movement_flags() & PMF_FOLLOW == 0;
        let spectating = player.movement_flags() & PMF_FOLLOW != 0
            || matches!(player.movement_type(), 4 | 7 | 8);
        self.cardboard_box
            .own_life((!spectating).then_some(own_playing), own);
        let world = &self.live_world;
        let presentation = i64::from(snapshot.server_time);
        let origin_of = |slot: usize| {
            world
                .entity(sjk_runtime::EntityId::new(slot as u64 + 1))
                .map(|entity| entity.sample(presentation).translation)
        };
        let looks = &self.looks;
        let boxed = |slot: usize| Some(slot) != own.map(usize::from) && looks.toy(slot, TOY);
        // The hub's events: a sender counts only when the game shows the name they claimed,
        // and never one blocked on this PC.
        for event in events {
            if event.toy != TOY
                || event.event != EVENT_SPOTTED
                || Some(event.slot) == own
                || self.muted_players.hides_fun(u16::from(event.slot))
            {
                continue;
            }
            let shown = crate::player_identity::shown_name(game_state, usize::from(event.slot));
            if !shown.is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown)) {
                continue;
            }
            let target = usize::from(event.target);
            let shows = boxed(target) && !self.cardboard_box.lifted(target);
            let at = if Some(event.target) == own {
                Some(player.origin())
            } else {
                origin_of(target)
            };
            let toward = origin_of(usize::from(event.slot))
                .zip(at)
                .and_then(|(from, at)| level_way(from, at));
            if Some(event.target) == own && self.cardboard_box.on {
                self.cardboard_box.lifted_by = Some((u16::from(event.slot), now));
            }
            self.cardboard_box
                .spotted(event.target, own, origin_of(target), shows, toward, now);
        }
        // `+use` facing a boxed player within reach lifts the box.
        if std::mem::take(&mut self.cardboard_box.use_pressed) && own_playing {
            let from = self
                .local_prediction
                .predicted_state()
                .map_or_else(|| player.origin(), |state| state.origin);
            let candidates = snapshot.entities.iter().filter_map(|entity| {
                let slot = usize::from(entity.number());
                (slot < SLOTS && boxed(slot) && !self.cardboard_box.lifted(slot) && drawn(entity))
                    .then(|| Some((slot as u8, Vec3::from_array(origin_of(slot)?))))
                    .flatten()
            });
            if let Some((target, at)) =
                pick_target(Vec3::from_array(from), self.camera_yaw, candidates)
            {
                self.cardboard_box.lift(target, at.to_array(), from, now);
                crate::player_identity::toy_event(TOY, EVENT_SPOTTED, target);
                // Alert!: the boxes of different players lifted, by the name shown.
                if self.toys_count()
                    && let Some(name) =
                        crate::player_identity::shown_name(game_state, usize::from(target))
                {
                    crate::achievements::note_name(crate::achievements::Counter::BoxSpotted, &name);
                }
            }
        }
        // Deaths in the box, of the players in this snapshot.
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
        let lifted = self.cardboard_box.lifted;
        self.cardboard_box.deaths(
            seen,
            dead,
            |slot| boxed(slot) && lifted[slot].is_none(),
            |slot| origin_of(slot).unwrap_or_default(),
        );
        self.cardboard_box.expire_lifts(boxed, now);
    }

    /// Keep one's own eye at crouch height while hidden in the box. The server stands a
    /// talking player up (the menu, the chat or the console open; [`crouched_in_box`]),
    /// and the view rose with them, a jump Dave saw on Escape (28/10/2026). The eye and
    /// the view height drop together, so the body's root (the eye less the view height)
    /// stays put. Run each frame after the prediction presented the eye.
    pub(crate) fn keep_boxed_eye(&mut self) {
        if self.live_session.is_none() || !self.cardboard_box.hiding() {
            return;
        }
        if let Some(drop) = eye_drop(self.local_prediction.view_height()) {
            self.camera_position.z -= drop;
            self.local_prediction.set_view_height(CROUCH_VIEWHEIGHT);
        }
    }

    /// Note which players are in a carton this frame ([`crouched_in_box`]), before the
    /// actors are posed, by the rules [`Self::submit_cardboard_boxes`] draws the cartons
    /// by: noted while drawing them, the pose read the previous frame's set (one
    /// crouched frame after leaving the box, one standing frame on entering it).
    pub(crate) fn note_boxed_players(&self) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            BOXED.store(0, Ordering::Relaxed);
            return;
        };
        let snapshot = session.latest_snapshot();
        let slots = crate::looks::ViewSlots::of(session.game_state(), &snapshot.player);
        let followed = slots.followed().filter(|&slot| slot < SLOTS);
        let own = slots.own.filter(|&slot| slot < SLOTS);
        let shows = |slot: usize| {
            Some(slot) != slots.own
                && self.looks.toy(slot, TOY)
                && self.cardboard_box.lifted[slot].is_none()
        };
        let mut boxed = 0_u64;
        if let Some(own) = own
            && self.cardboard_box.crouching
            && self.cardboard_box.own_alive == Some(true)
        {
            boxed |= 1 << own;
        }
        if let Some(slot) = followed
            && shows(slot)
            && alive(&snapshot.player)
        {
            boxed |= 1 << slot;
        }
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot < SLOTS
                && Some(slot) != followed
                && Some(slot) != own
                && drawn(entity)
                && shows(slot)
                && self
                    .live_world
                    .entity(sjk_runtime::EntityId::new(slot as u64 + 1))
                    .is_some()
            {
                boxed |= 1 << slot;
            }
        }
        BOXED.store(boxed, Ordering::Relaxed);
    }

    /// Add this frame's cartons, and note the music: the player's own (drawn in mirrors
    /// only in first person), then every other boxed player the game draws; then the
    /// boxes put on, taken off and fallen ([`CardboardBox::frame`]).
    pub(crate) fn submit_cardboard_boxes(&mut self, presentation_time: i64, now: Instant) {
        self.cardboard_box.music.clear();
        let mesh = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == MODEL);
        #[cfg(test)]
        if let Some(mesh) = mesh {
            for &(origin, yaw) in &self.cardboard_box.shot_cartons {
                self.object_groups[mesh].push(carton(origin, yaw));
            }
            for &(origin, yaw, way, age) in &self.cardboard_box.shot_fallen {
                let fallen = Fallen {
                    feet: Vec3::from_array(origin) + Vec3::Z * FEET,
                    yaw,
                    toward: Vec2::from_angle(way),
                    slot: 0,
                    since: now - age,
                    own_view: false,
                    landed: true,
                };
                if let Some((instance, _)) = fallen.pose(now) {
                    self.object_groups[mesh].push(instance);
                }
            }
        }
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let slots = crate::looks::ViewSlots::of(session.game_state(), &snapshot.player);
        let followed = slots.followed().filter(|&slot| slot < SLOTS);
        let first_person = !self.third_person;
        let mut cartons = [None::<(usize, ActorInstance)>; SLOTS];
        let mut seen = [Seen::default(); SLOTS];
        let own = slots.own.filter(|&slot| slot < SLOTS);
        if let Some(own) = own {
            // Flying the free camera, the predicted state is the camera's: the
            // carton stays on the body the server holds (hub report #152).
            let origin = if self.local_prediction.fake_noclip() {
                snapshot.player.origin()
            } else {
                self.local_prediction
                    .predicted_state()
                    .map_or_else(|| snapshot.player.origin(), |state| state.origin)
            };
            seen[own].worn = self.cardboard_box.on;
            seen[own].drawn = (self.cardboard_box.own_alive == Some(true)).then_some(origin);
        }
        if self.cardboard_box.crouching
            && let Some(own) = own
            && let Some(origin) = seen[own].drawn
        {
            // Flying, the camera turns on its own: the carton keeps the body's facing,
            // which the server holds (Dave's test of #152, 28/10/2026).
            let yaw = if self.local_prediction.fake_noclip() {
                snapshot.player.view_angles()[1].to_radians()
            } else {
                self.camera_yaw
            };
            seen[own].carton = Some((origin, yaw));
            let mut instance = carton(origin, yaw);
            // A free camera starting in the body looks out of the carton, not at its
            // walls from inside (as the body is hidden while the camera is in its head).
            let camera_inside = self.local_prediction.fake_noclip()
                && inside_carton(self.camera_position.to_array(), origin);
            if first_person || camera_inside {
                instance.view_flags |= 1;
            }
            cartons[own] = Some((own, instance));
            self.cardboard_box.music.push(Music {
                slot: own as u8,
                origin: None,
            });
        }
        let lifted = self.cardboard_box.lifted;
        let looks = &self.looks;
        for (slot, seen) in seen.iter_mut().enumerate() {
            if Some(slot) != own {
                seen.worn = looks.toy(slot, TOY);
            }
        }
        let shows =
            |slot: usize| Some(slot) != slots.own && looks.toy(slot, TOY) && lifted[slot].is_none();
        // The player the view follows: their entity is not in the snapshot.
        if let Some(slot) = followed
            && alive(&snapshot.player)
        {
            seen[slot].drawn = Some(snapshot.player.origin());
        }
        if let Some(slot) = followed
            && shows(slot)
            && alive(&snapshot.player)
        {
            let origin = snapshot.player.origin();
            let yaw = snapshot.player.view_angles()[1].to_radians();
            seen[slot].carton = Some((origin, yaw));
            let mut instance = carton(origin, yaw);
            if first_person {
                instance.view_flags |= 1;
            }
            cartons[slot] = Some((slot, instance));
            self.cardboard_box.music.push(Music {
                slot: slot as u8,
                origin: Some(origin),
            });
        }
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot >= SLOTS || Some(slot) == followed || Some(slot) == own || !drawn(entity) {
                continue;
            }
            let Some(presented) = self
                .live_world
                .entity(sjk_runtime::EntityId::new(slot as u64 + 1))
            else {
                continue;
            };
            let origin = presented.sample(presentation_time).translation;
            seen[slot].drawn = Some(origin);
            if !shows(slot) {
                continue;
            }
            let yaw = presented
                .sample_pose(presentation_time)
                .map_or(0.0, |pose| pose.view_angles_degrees[1])
                .to_radians();
            seen[slot].carton = Some((origin, yaw));
            cartons[slot] = Some((slot, carton(origin, yaw)));
            self.cardboard_box.music.push(Music {
                slot: slot as u8,
                origin: Some(origin),
            });
        }
        self.cardboard_box.frame(&seen, own, first_person, now);
        if let Some(mesh) = mesh {
            for (_, instance) in cartons.into_iter().flatten() {
                self.object_groups[mesh].push(instance);
            }
            let group = &mut self.object_groups[mesh];
            self.cardboard_box
                .fallen_frame(now, |instance| group.push(instance));
        }
    }

    /// Play the box's one-shot sounds; the sounds are handed to the audio the first time
    /// there is one.
    pub(crate) fn play_cardboard_box_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.cardboard_box.cues.clear();
            return;
        };
        if self.cardboard_box.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [MUSIC, EQUIP, SPOTTED, DROP, DEATH]);
            self.cardboard_box.sounds_registered = Some(audio.generation());
        }
        for cue in self.cardboard_box.take_cues() {
            audio.play_toy(
                crate::toy_sounds::Toy::Box,
                cue.path,
                1.0,
                cue.origin,
                sjk_audio::SourceId(u32::from(cue.slot)),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_files_mount_and_the_model_is_an_open_carton_on_the_floor() {
        let mut vfs = VirtualFileSystem::new();
        mount(&mut vfs).unwrap();
        for (path, _) in FILES {
            assert!(vfs.read(path).unwrap().is_some(), "{path}");
        }
        let model = sjk_model::Md3::parse(&vfs.read(MODEL).unwrap().unwrap().bytes).unwrap();
        let surface = &model.surfaces[0];
        assert_eq!(surface.shaders, ["models/sjk/cardboard_box"]);
        // Four sides and the top, no bottom.
        assert_eq!((surface.frames[0].len(), surface.triangles.len()), (20, 10));
        let (mut low, mut high) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for vertex in &surface.frames[0] {
            let position = Vec3::from_array(vertex.position);
            low = low.min(position);
            high = high.max(position);
        }
        // Wider and taller than a crouched player (30 x 30 x 40), from the floor up.
        assert!((low.z).abs() < 1e-3 && high.z > 40.0, "{low} {high}");
        assert!(low.x < -15.0 && high.x > 15.0 && low.y < -15.0 && high.y > 15.0);
        // Clockwise seen from outside, stock's front side.
        let centre = Vec3::new(0.0, 0.0, high.z / 2.0);
        for [a, b, c] in &surface.triangles {
            let [a, b, c] =
                [a, b, c].map(|&i| Vec3::from_array(surface.frames[0][i as usize].position));
            let facing = (b - a).cross(c - a);
            assert!(facing.dot((a + b + c) / 3.0 - centre) < 0.0);
        }
        let icon = image::load_from_memory(FILES[3].1).unwrap();
        assert_eq!((icon.width(), icon.height()), (128, 128));
    }

    #[test]
    fn the_box_holds_crouch_only_while_on_and_playing_and_notes_use_presses() {
        let mut toy = CardboardBox::default();
        let mut command = UserCommand {
            up_move: 127,
            ..UserCommand::default()
        };
        toy.own_life(Some(true), Some(2));
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, 127, "off: the jump goes");
        toy.toggle();
        toy.own_life(Some(true), Some(2));
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, CROUCH, "on: crouch over the jump");
        // Dead, or out of a game, nothing is held.
        toy.own_life(None, None);
        command.up_move = 0;
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, 0);
        // A held `+use` is one press.
        command.buttons = BUTTON_USE;
        toy.shape_command(&mut command);
        assert!(std::mem::take(&mut toy.use_pressed));
        toy.shape_command(&mut command);
        assert!(!toy.use_pressed);
        command.buttons = 0;
        toy.shape_command(&mut command);
        command.buttons = BUTTON_USE;
        toy.shape_command(&mut command);
        assert!(toy.use_pressed);
    }

    #[test]
    fn flying_the_free_camera_the_box_holds_no_crouch_and_hides_nobody() {
        // Hub report #152: the box's crouch steered the free camera down.
        let mut toy = CardboardBox::default();
        toy.set_on(true);
        toy.own_life(Some(true), Some(2));
        toy.set_flying(true);
        let mut command = UserCommand {
            up_move: 127,
            ..UserCommand::default()
        };
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, 127, "flying: up goes up");
        command.up_move = 0;
        toy.shape_preview(&mut command);
        assert_eq!(command.up_move, 0, "nor in the preview");
        assert!(!toy.hiding(), "flying is not hiding");
        assert!(toy.on(), "the box stays on");
        toy.set_flying(false);
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, CROUCH, "back on foot: crouch again");
        assert!(toy.hiding());
    }

    /// Dave's test: opening the menu in the box stood the view up (36) from the crouch
    /// (12); the eye drops back, once, and a crouching eye stays.
    #[test]
    fn a_boxed_eye_stays_at_crouch_height() {
        assert_eq!(eye_drop(36.0), Some(24.0));
        assert_eq!(eye_drop(CROUCH_VIEWHEIGHT), None);
        assert_eq!(eye_drop(8.0), None);
    }

    /// Dave's test of #152: a boxed player is drawn crouching until they leave the
    /// box, also while the server stands them up (talking).
    #[test]
    fn a_boxed_player_is_drawn_crouching() {
        BOXED.store(1 << 3, Ordering::Relaxed);
        assert!(crouched_in_box(3) && !crouched_in_box(4) && !crouched_in_box(SLOTS));
        BOXED.store(0, Ordering::Relaxed);
    }

    /// A free camera that starts in the body is inside the carton; once it has flown
    /// out of it, it is not.
    #[test]
    fn a_camera_in_the_carton_is_inside_it() {
        let origin = [100.0, 200.0, 50.0];
        let eye = [100.0, 200.0, 50.0 + 26.0 - 24.0];
        assert!(inside_carton(eye, origin), "at the crouched eye");
        assert!(inside_carton([120.0, 220.0, 40.0], origin), "a corner");
        assert!(!inside_carton([100.0, 260.0, 50.0], origin), "beside it");
        assert!(!inside_carton([100.0, 200.0, 120.0], origin), "above it");
    }

    #[test]
    fn dying_in_the_box_plays_its_sound_and_the_box_is_lost_at_the_respawn() {
        let mut toy = CardboardBox::default();
        toy.set_on(true);
        toy.own_life(Some(true), Some(4));
        toy.own_life(Some(false), Some(4));
        let cues: Vec<_> = toy.take_cues().collect();
        assert_eq!(
            cues,
            [Cue {
                path: DEATH,
                slot: 4,
                origin: None
            }]
        );
        assert!(
            toy.on(),
            "kept while dead, so others still see whose death it was"
        );
        assert!(!toy.crouching);
        toy.own_life(Some(true), Some(4));
        assert!(!toy.on() && !toy.crouching);
        // Without the box, a death is silent.
        toy.own_life(Some(false), Some(4));
        assert_eq!(toy.take_cues().count(), 0);
    }

    #[test]
    fn a_spotted_event_takes_the_own_box_off_or_hides_anothers() {
        let now = Instant::now();
        let mut toy = CardboardBox::default();
        toy.set_on(true);
        toy.own_life(Some(true), Some(1));
        toy.spotted(1, Some(1), None, false, None, now);
        assert!(!toy.on() && !toy.crouching);
        assert_eq!(toy.take_cues().next().unwrap().path, SPOTTED);
        // Again (a repeat): the box is already off, nothing plays.
        toy.spotted(1, Some(1), None, false, None, now);
        assert_eq!(toy.take_cues().count(), 0);
        // Another player's: hidden, the sound where they stand, once.
        toy.spotted(6, Some(1), Some([1.0, 2.0, 3.0]), true, None, now);
        assert!(toy.lifted(6));
        let cue = toy.take_cues().next().unwrap();
        assert_eq!((cue.slot, cue.origin), (6, Some([1.0, 2.0, 3.0])));
        toy.spotted(6, Some(1), Some([1.0, 2.0, 3.0]), true, None, now);
        assert_eq!(toy.take_cues().count(), 0, "lifted here already");
        // The lift ends when their look drops the box, or after the hold.
        toy.expire_lifts(|_| true, now + Duration::from_secs(1));
        assert!(toy.lifted(6));
        toy.expire_lifts(|_| true, now + LIFT_HOLD + Duration::from_secs(1));
        assert!(!toy.lifted(6));
        toy.lift(7, [0.0; 3], [-50.0, 0.0, 0.0], now);
        toy.expire_lifts(|slot| slot != 7, now);
        assert!(!toy.lifted(7));
    }

    #[test]
    fn a_boxed_player_newly_dead_plays_the_death_sound_once() {
        let mut toy = CardboardBox::default();
        let boxed = |slot: usize| slot == 3;
        let at = |_| [5.0, 0.0, 0.0];
        toy.deaths(0b1111, 0, boxed, at);
        toy.deaths(0b1111, 0b1100, boxed, at);
        let cues: Vec<_> = toy.take_cues().collect();
        assert_eq!(cues.len(), 1, "slot 2 died without a box");
        assert_eq!((cues[0].path, cues[0].slot), (DEATH, 3));
        toy.deaths(0b1111, 0b1100, boxed, at);
        assert_eq!(toy.take_cues().count(), 0, "still dead");
        // Out of the snapshot, the slot keeps what was seen.
        toy.deaths(0b0111, 0, boxed, at);
        toy.deaths(0b1111, 0b1000, boxed, at);
        assert_eq!(toy.take_cues().count(), 0);
    }

    /// A slot drawn at `x` on the X axis, worn or not, with a carton or not.
    fn at(x: f32, worn: bool, carton: bool) -> Seen {
        Seen {
            carton: carton.then_some(([x, 0.0, 24.0], 0.0)),
            worn,
            drawn: Some([x, 0.0, 24.0]),
        }
    }

    #[test]
    fn a_box_put_on_plays_its_sound_once_the_looks_settled() {
        let start = Instant::now();
        let mut toy = CardboardBox::default();
        let mut seen = [Seen::default(); SLOTS];
        seen[3] = at(0.0, true, true);
        seen[5] = at(100.0, true, true);
        // Joining: the boxes already in the looks make no sound, but the own one does.
        toy.frame(&seen, Some(5), true, start);
        let cues: Vec<_> = toy.take_cues().collect();
        assert_eq!(
            cues,
            [Cue {
                path: EQUIP,
                slot: 5,
                origin: None
            }]
        );
        seen[3] = at(0.0, false, false);
        toy.frame(&seen, Some(5), true, start + Duration::from_secs(1));
        toy.fallen.clear();
        let _ = toy.take_cues().count();
        seen[3] = at(0.0, true, true);
        toy.frame(&seen, Some(5), true, start + SETTLE);
        let cue = toy.take_cues().next().unwrap();
        assert_eq!(
            (cue.path, cue.slot, cue.origin),
            (EQUIP, 3, Some([0.0, 0.0, 24.0]))
        );
        toy.frame(&seen, Some(5), true, start + SETTLE);
        assert_eq!(toy.take_cues().count(), 0, "once");
    }

    #[test]
    fn a_box_taken_off_or_lifted_tips_over_lands_with_a_thud_and_goes() {
        let start = Instant::now();
        let mut toy = CardboardBox::default();
        let mut seen = [Seen::default(); SLOTS];
        seen[2] = at(0.0, true, true);
        seen[4] = at(200.0, true, true);
        toy.frame(&seen, None, false, start);
        // Slot 2 takes it off; slot 4 is lifted by someone standing east of them.
        seen[2] = at(0.0, false, false);
        seen[4] = at(200.0, true, false);
        toy.lift(4, [200.0, 0.0, 24.0], [260.0, 0.0, 24.0], start);
        toy.frame(&seen, None, false, start);
        assert_eq!(toy.fallen.len(), 2);
        let ways: Vec<_> = toy.fallen.iter().map(|fallen| fallen.toward).collect();
        assert_eq!(
            ways,
            [Vec2::X, -Vec2::X],
            "the way it faced; away from the lifter"
        );
        let _ = toy.take_cues().count();
        // Halfway, it leans; then it lies on its side, the thud once.
        let mut drawn = Vec::new();
        toy.fallen_frame(start + FALL / 2, |instance| drawn.push(instance));
        assert_eq!((drawn.len(), toy.take_cues().count()), (2, 0));
        let mut drawn = Vec::new();
        toy.fallen_frame(start + FALL, |instance| drawn.push(instance));
        let thuds: Vec<_> = toy.take_cues().collect();
        assert_eq!(thuds.len(), 2);
        assert!(thuds.iter().all(|cue| cue.path == DROP));
        // On its side, the top towards where it fell: the box's up is level now.
        let up = Quat::from_array(drawn[0].rotation) * Vec3::Z;
        assert!(up.abs_diff_eq(Vec3::X, 1e-4), "{up}");
        toy.fallen_frame(start + FALL + REST, |_| {});
        assert_eq!(toy.take_cues().count(), 0);
        let mut left = 0;
        toy.fallen_frame(start + FALL + REST + SHRINK, |_| left += 1);
        assert_eq!((left, toy.fallen.len()), (0, 0), "gone");
        // A box that goes with its player (dead, cloaked, out of sight) does not fall.
        seen[2] = at(0.0, true, true);
        toy.frame(&seen, None, false, start);
        seen[2] = Seen {
            worn: true,
            ..Seen::default()
        };
        toy.frame(&seen, None, false, start);
        assert!(toy.fallen.is_empty());
    }

    #[test]
    fn use_lifts_the_nearest_box_in_reach_and_in_front() {
        let here = Vec3::ZERO;
        let east = 0.0;
        let boxed = [
            (1, Vec3::new(80.0, 0.0, 0.0)),
            (2, Vec3::new(40.0, 10.0, 0.0)),
            (3, Vec3::new(-30.0, 0.0, 0.0)),
            (4, Vec3::new(200.0, 0.0, 0.0)),
        ];
        assert_eq!(
            pick_target(here, east, boxed).map(|(slot, _)| slot),
            Some(2)
        );
        // Behind only: none.
        assert_eq!(pick_target(here, east, [boxed[2]]), None);
        // Too far: none.
        assert_eq!(pick_target(here, east, [boxed[3]]), None);
        // Facing west, the one behind is in front.
        assert_eq!(
            pick_target(here, std::f32::consts::PI, boxed).map(|(slot, _)| slot),
            Some(3)
        );
    }
}
