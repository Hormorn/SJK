//! The Hax, SJK's meme toy (`toy_hax`; Sol's idea, 21/10/2026): a player who has it on
//! holds `+use` on another player who has it on, the siege hack bar fills with its
//! sound, and once the hack is done the hacker steers the hacked player for a few
//! seconds (running, walking, jumping, crouching, swinging), watching them from behind.
//!
//! No game server knows of it, and no client can move another player: the hacked
//! player's own client plays the hacker's controls as its own user commands. The hacker's
//! client sends them, many times a second, through the SJK hub's toy relay
//! (`player_identity::relay_toy_event`, `PROTOCOL.md` "Toy relay"), which hands them to
//! the hacked player's client at once. The toy itself travels as a toy in the player's
//! look (`looks.rs`, [`TOY`]), so the hacker's client knows who may be hacked.
//!
//! The exchange, every event from one player to the other (the relay's `target`):
//!
//! - hacker: `hack_<moment>` (a hack began, done at the game server's `moment`), `stop`
//!   (it ended early), `seize` (it is done), then `in_<frame>` ([`Frame`]) while in control;
//! - hacked player: `taken_<until>` (in control until the game server's `until`),
//!   `refused_<why>` ([`Refusal`]), or `stop` (broke free: the toy turned off, died).
//!
//! Only the hacked player's client decides: it takes a seize only from a player the game
//! shows under the name they claimed, not blocked here, who has the toy out, while its own
//! toy is on and its player plays (not in a duel, not carrying a flag, not away, not
//! hacked in the last [`IMMUNE_MS`]). A hack does not begin on a player who is typing in
//! the chat (their talk balloon, `EF_TALK`) or away as the scoreboard marks them. Turning the toy off frees the player at once. Controls are
//! limited to [`CONTROL_BUTTONS`] and the movement keys: no Force power, weapon change,
//! chat or command goes through. The hacker's own player stands still meanwhile, open to
//! attack. Players on stock clients see a player who stands still and one who moves.
//!
//! A trap (Sol's idea, 23/10/2026): a player with [`TRAP_CVAR`] on answers a seize with
//! `trap_<moment>` instead. Nobody is steered: both players read "IT'S A TRAP!" over
//! Order 66's beeps, and at the moment, two seconds on, both clients send `kill` and
//! both players blow up as Order 66's do (`order66::order66_blow_up`). The hacked player
//! tells everyone else through a feed toy event (`trap_<moment>`, to the hacker), so
//! their clients blow the two up too; the two seconds give it time to arrive. With
//! [`AWAY_CVAR`] on as well, the trap holds while the player is away: their look shows
//! [`AWAY_TOY`], so hackers' clients let a hack begin on them though they are away.
//!
//! A hacked player is wreathed in a green digital rain ([`GLYPHS`]) while steered: on
//! their own screen and the hacker's at once, and on every other SJK client of the server
//! through a feed toy event the hacked player sends (`hacked_<until>`, then `freed` when
//! they break free sooner), which may come a second or two late.
//!
//! The hack bar is the siege one (`hud/hack_bar.rs`, the stock `gfx/mp/c_icon_tech`),
//! shown to both, and the loop is the stock hacking sound ([`HACK_SOUND`]); the seize and
//! the release play the stock mind trick's ([`SEIZE_SOUND`], [`RELEASE_SOUND`]).

use crate::GpuState;
use crate::ghost_replay::Drive;
use glam::Vec3;
use sjk_protocol::UserCommand;
use sjk_runtime::EntityId;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// The toy's console command (a toy is a `toy_<name>` command): turn it on or off.
pub(crate) const COMMAND: &str = "toy_hax";
/// The toy's id in a look's `toys` and in relayed toy events.
pub(crate) const TOY: &str = "hax";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_hax.png";
/// The stock siege hacking loop (`G_SetClientSound`, `level.snd_hack`).
pub(crate) const HACK_SOUND: &str = "sound/player/hacking.wav";
/// Played to both when the hacker takes control, and when they let go.
pub(crate) const SEIZE_SOUND: &str = "sound/weapons/force/distract.wav";
pub(crate) const RELEASE_SOUND: &str = "sound/weapons/force/distractstop.wav";
/// How long `+use` is held to hack, in game server milliseconds.
pub(crate) const HACK_MS: i32 = 3_000;
/// How long the hacker steers.
pub(crate) const CONTROL_MS: i32 = 5_000;
/// After steering, the hacker waits this long before hacking again.
const COOLDOWN_MS: i32 = 15_000;
/// After being steered, a player cannot be hacked for this long.
pub(crate) const IMMUNE_MS: i32 = 15_000;
/// A hack holds while the target stays this close (the start needs the cardboard box's
/// reach, `cardboard_box::REACH`).
const HOLD_REACH: f32 = 160.0;
/// A hack holds while the hacker faces the target this much (cosine of the angle).
const HOLD_FACING: f32 = 0.3;
/// The hacker gives up on an answer to a seize after this long.
const ANSWER_WAIT: Duration = Duration::from_secs(2);
/// A warning whose hack came neither to an end nor to a seize goes this long after its
/// moment.
const WARNING_GRACE_MS: i32 = 2_500;
/// Without a frame this long, the hacked player stands still; this long, they are free.
const FRAME_STALE: Duration = Duration::from_millis(400);
const FRAME_LOST: Duration = Duration::from_millis(1_500);
/// Shortest time between two frames sent; one with other keys or buttons goes after
/// [`FRAME_GAP_CHANGED`], else after [`FRAME_GAP`]. The relay takes 40 a second.
const FRAME_GAP_CHANGED: Duration = Duration::from_millis(30);
const FRAME_GAP: Duration = Duration::from_millis(50);
/// `BUTTON_ATTACK`, `BUTTON_WALKING`, `BUTTON_USE` and `BUTTON_ALT_ATTACK`.
const BUTTON_ATTACK: u16 = 1;
const BUTTON_WALKING: u16 = 16;
const BUTTON_USE: u16 = 32;
const BUTTON_ALT_ATTACK: u16 = 128;
/// The buttons a hacker may press for the hacked player: swings and walking.
pub(crate) const CONTROL_BUTTONS: u16 = BUTTON_ATTACK | BUTTON_ALT_ATTACK | BUTTON_WALKING;
/// The hacker's camera: above the hacked player's origin, and how far behind.
const CAMERA_HEIGHT: f32 = 26.0;
const CAMERA_RANGE: f32 = 100.0;
const CAMERA_RISE: f32 = 12.0;
const SLOTS: usize = crate::looks::SLOTS;
/// `PMF_FOLLOW`.
const PMF_FOLLOW: u16 = 4096;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
/// `EF_TALK`: the player's chat is open (the talk balloon).
const EF_TALK: u32 = sjk_game_jka::pmove_talk::EF_TALK;
/// `EF_NODRAW` (`bg_public.h`); `1 << 3` is `EF_TELEPORT_BIT`, which the server flips at
/// every teleport and respawn, so a player with it set is drawn as any other.
const EF_NODRAW: u32 = 1 << 8;
/// The game server's clock going back more than this is a new map: what was timed on
/// the old clock (a hack, the cooldown, the immunity) is forgotten.
const REWIND_MS: i32 = 1_000;

/// A bundled file's bytes, by its name in `assets/hax`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/hax/", $name))
    };
}

/// The digital rain round a hacked player: one puff of glyphs, emitted every
/// [`GLYPHS_EVERY`] where they stand so it follows them.
pub(crate) const GLYPHS: &str = "sjk/hax_glyphs";
const GLYPHS_EVERY: Duration = Duration::from_millis(70);
/// The feed toy events that tell every SJK player of the server who is hacked: this,
/// then the game server time it lasts until; and the end, when it comes sooner.
const HACKED_PREFIX: &str = "hacked_";
/// The trap: this, then the game server time both players blow up at.
const TRAP_PREFIX: &str = "trap_";
/// From the trap sprung to the blast.
pub(crate) const TRAP_MS: i32 = 2_000;
/// Archived cvar: a hack on the player springs the trap instead (off by default).
pub(crate) const TRAP_CVAR: &str = "cl_haxTrap";
/// Archived cvar: with the trap on, the player may be hacked while away.
pub(crate) const AWAY_CVAR: &str = "cl_haxAway";
/// The look's toy that says the player may be hacked while away (the trap is set).
pub(crate) const AWAY_TOY: &str = "hax_away";
const FREED: &str = "freed";

/// The bundled files at their game paths (the pictures made by `scripts/hax_assets.py`,
/// the effect and the shader written by hand).
const FILES: [(&str, &[u8]); 11] = [
    (ICON, bundled!("toy_hax.png")),
    ("gfx/sjk/hax/glyph_0.png", bundled!("glyph_0.png")),
    ("gfx/sjk/hax/glyph_1.png", bundled!("glyph_1.png")),
    ("gfx/sjk/hax/glyph_2.png", bundled!("glyph_2.png")),
    ("gfx/sjk/hax/glyph_3.png", bundled!("glyph_3.png")),
    ("gfx/sjk/hax/glyph_4.png", bundled!("glyph_4.png")),
    ("gfx/sjk/hax/glyph_5.png", bundled!("glyph_5.png")),
    ("gfx/sjk/hax/glyph_6.png", bundled!("glyph_6.png")),
    ("gfx/sjk/hax/glyph_7.png", bundled!("glyph_7.png")),
    ("effects/sjk/hax_glyphs.efx", bundled!("hax_glyphs.efx")),
    ("shaders/sjk_hax.shader", bundled!("hax.shader")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Hax", FILES.iter().copied())?;
    Ok(())
}

/// The hacker's controls at one moment: the movement keys, the buttons
/// ([`CONTROL_BUTTONS`]) and the view the hacked player turns to (the game server's
/// angles, 16-bit), numbered so the newest wins.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Frame {
    seq: u32,
    forward: i8,
    right: i8,
    up: i8,
    buttons: u16,
    pitch: u16,
    yaw: u16,
}

impl Frame {
    /// `in_<seq>_<forward>_<right>_<up>_<buttons>_<pitch>_<yaw>`.
    fn event(&self) -> String {
        format!(
            "in_{}_{}_{}_{}_{}_{}_{}",
            self.seq, self.forward, self.right, self.up, self.buttons, self.pitch, self.yaw
        )
    }

    fn parse(rest: &str) -> Option<Self> {
        let mut parts = rest.split('_');
        let mut next = || parts.next();
        let frame = Self {
            seq: next()?.parse().ok()?,
            forward: next()?.parse().ok()?,
            right: next()?.parse().ok()?,
            up: next()?.parse().ok()?,
            buttons: next()?.parse::<u16>().ok()? & CONTROL_BUTTONS,
            pitch: next()?.parse().ok()?,
            yaw: next()?.parse().ok()?,
        };
        next().is_none().then_some(frame)
    }

    /// Whether the keys or buttons differ from `other`'s (not the view).
    fn presses_differ(&self, other: &Self) -> bool {
        (self.forward, self.right, self.up, self.buttons)
            != (other.forward, other.right, other.up, other.buttons)
    }
}

/// Why a hacked player's client turned a seize down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    /// Its toy is off, or it is busy with another hack.
    Busy,
    /// Not playing: spectating, dead, at the intermission.
    Away,
    Duel,
    Flag,
    /// Hacked a moment ago ([`IMMUNE_MS`]).
    Immune,
    /// Away from the keyboard.
    Afk,
}

impl Refusal {
    fn word(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Away => "away",
            Self::Duel => "duel",
            Self::Flag => "flag",
            Self::Immune => "immune",
            Self::Afk => "afk",
        }
    }

    fn from_word(word: &str) -> Self {
        match word {
            "away" => Self::Away,
            "duel" => Self::Duel,
            "flag" => Self::Flag,
            "immune" => Self::Immune,
            "afk" => Self::Afk,
            _ => Self::Busy,
        }
    }

    /// Why, as the hacker reads it after the player's name.
    fn words(self) -> &'static str {
        match self {
            Self::Busy => "is busy",
            Self::Away => "is not playing",
            Self::Duel => "is in a duel",
            Self::Flag => "carries the flag",
            Self::Immune => "was hacked a moment ago",
            Self::Afk => "is away",
        }
    }
}

/// A relayed event of the toy, as read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Event {
    Hack(i32),
    Stop,
    Seize,
    Frame(Frame),
    Taken(i32),
    Refused(Refusal),
    Trap(i32),
}

impl Event {
    pub(crate) fn parse(event: &str) -> Option<Self> {
        if let Some(rest) = event.strip_prefix("in_") {
            return Frame::parse(rest).map(Self::Frame);
        }
        if let Some(moment) = event.strip_prefix("hack_") {
            return moment.parse().ok().map(Self::Hack);
        }
        if let Some(until) = event.strip_prefix("taken_") {
            return until.parse().ok().map(Self::Taken);
        }
        if let Some(why) = event.strip_prefix("refused_") {
            return Some(Self::Refused(Refusal::from_word(why)));
        }
        if let Some(moment) = event.strip_prefix(TRAP_PREFIX) {
            return moment.parse().ok().map(Self::Trap);
        }
        match event {
            "stop" => Some(Self::Stop),
            "seize" => Some(Self::Seize),
            _ => None,
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Hack(moment) => format!("hack_{moment}"),
            Self::Stop => "stop".to_owned(),
            Self::Seize => "seize".to_owned(),
            Self::Frame(frame) => frame.event(),
            Self::Taken(until) => format!("taken_{until}"),
            Self::Refused(why) => format!("refused_{}", why.word()),
            Self::Trap(moment) => format!("{TRAP_PREFIX}{moment}"),
        }
    }
}

/// Where the player stands in a hack.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Idle,
    /// Holding `+use` on `target`; the hack is done at `done_at`.
    Hacking {
        target: u8,
        done_at: i32,
    },
    /// The hack is done; waiting for `target`'s answer since `since`.
    Seizing {
        target: u8,
        since: Instant,
    },
    /// Steering `target` until `until`. `frozen` is the own view the commands keep (the
    /// game server's angles) and `camera` the camera's own angles to give back (pitch,
    /// yaw in radians), both taken at the first command.
    Controlling {
        target: u8,
        until: i32,
        frozen: Option<[i32; 3]>,
        camera: Option<(f32, f32)>,
    },
    /// `hacker` hacks the player; done at `done_at`.
    Targeted {
        hacker: u8,
        done_at: i32,
    },
    /// `hacker` steers the player until `until`, with their newest frame (when it came)
    /// and the weapon held when it began.
    Controlled {
        hacker: u8,
        until: i32,
        frame: Option<(Frame, Instant)>,
        since: Instant,
        weapon: Option<u8>,
    },
    /// In a trap with `other` (the hacker, or the player who set it): both blow up at
    /// `at`; `shown` is the countdown's second last shown.
    Trapped {
        other: u8,
        at: i32,
        shown: i32,
    },
}

/// Two players a trap blows up at a moment (bits by slot), and those already blown up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrapBlast {
    slots: u64,
    at: i32,
    blasted: u64,
    /// Heard this frame: a player of it already dead blows up too (the news came late).
    fresh: bool,
}

/// The toy's state.
pub(crate) struct Hax {
    on: bool,
    server: Option<SocketAddr>,
    role: Role,
    /// `+use` in the last command, and a press since the last frame.
    use_held: bool,
    use_pressed: bool,
    /// The game server time before which the player may not hack, nor be hacked.
    ready_at: i32,
    immune_until: i32,
    /// The game server time last seen, to notice its clock going back ([`REWIND_MS`]).
    seen: Option<i32>,
    /// Until when each slot's player is hacked (game server time), for the rain.
    hacked: [i32; SLOTS],
    /// When the rain was last emitted.
    glyphs_at: Option<Instant>,
    /// The traps sprung on this server, to blow their two players up.
    traps: Vec<TrapBlast>,
    /// The slots seen dead in the last snapshot, for the traps.
    dead: u64,
    /// The last frame sent, and when.
    sent: Option<(Frame, Instant)>,
    seq: u32,
    /// The one-shot sounds to play.
    cues: Vec<&'static str>,
    sounds_registered: Option<u64>,
}

impl Default for Hax {
    fn default() -> Self {
        Self {
            on: false,
            server: None,
            role: Role::Idle,
            use_held: false,
            use_pressed: false,
            ready_at: i32::MIN,
            immune_until: i32::MIN,
            seen: None,
            hacked: [i32::MIN; SLOTS],
            glyphs_at: None,
            traps: Vec::with_capacity(4),
            dead: 0,
            sent: None,
            seq: 0,
            cues: Vec::with_capacity(4),
            sounds_registered: None,
        }
    }
}

/// The game server's 16-bit angle for `degrees` (`ANGLE2SHORT`).
fn short(degrees: f32) -> u16 {
    ((degrees * 65_536.0 / 360.0) as i32 & 0xFFFF) as u16
}

/// Degrees for a 16-bit angle, from -180 to 180.
fn degrees(short: u16) -> f32 {
    let degrees = f32::from(short) * (360.0 / 65_536.0);
    if degrees > 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

/// A command that keeps `view` (the game server's angles) with `delta`, and nothing
/// else of `input` but its weapon and Force power: no key, no button.
fn still(input: &UserCommand, view: [i32; 3], delta: [i32; 3]) -> UserCommand {
    UserCommand {
        angles: std::array::from_fn(|i| view[i].wrapping_sub(delta[i]) & 0xFFFF),
        forward_move: 0,
        right_move: 0,
        up_move: 0,
        buttons: 0,
        generic_command: 0,
        ..*input
    }
}

/// The command `frame` makes from `input` with `delta`: its keys, buttons and view
/// (the roll stays the player's), `weapon` held; `None` when it is out of date, which
/// stands still at its view.
fn steered(
    input: &UserCommand,
    frame: &Frame,
    fresh: bool,
    delta: [i32; 3],
    weapon: Option<u8>,
) -> UserCommand {
    let view = [
        i32::from(frame.pitch),
        i32::from(frame.yaw),
        input.angles[2].wrapping_add(delta[2]),
    ];
    let mut command = still(input, view, delta);
    if fresh {
        command.forward_move = frame.forward;
        command.right_move = frame.right;
        command.up_move = frame.up;
        command.buttons = frame.buttons & CONTROL_BUTTONS;
    }
    if let Some(weapon) = weapon {
        command.weapon = weapon;
    }
    command
}

impl Hax {
    /// `toy_hax`: turn the toy on or off.
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

    /// Note `+use` in the command about to be sent (the start of a hack is a press).
    pub(crate) fn observe_use(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
    }

    /// The command to send in place of `input` while the player steers or is steered,
    /// `delta` the player's `delta_angles` and `camera` the camera's angles (pitch, yaw
    /// in radians, pitch up). Steering, it also hands the frame to the relay when due.
    pub(crate) fn drive(
        &mut self,
        input: &UserCommand,
        delta: [i32; 3],
        camera: (f32, f32),
    ) -> Option<Drive> {
        let now = Instant::now();
        match &mut self.role {
            Role::Controlling {
                target,
                frozen,
                camera: kept,
                ..
            } => {
                let view = *frozen.get_or_insert_with(|| {
                    std::array::from_fn(|i| input.angles[i].wrapping_add(delta[i]) & 0xFFFF)
                });
                kept.get_or_insert(camera);
                let target = *target;
                let frame = Frame {
                    seq: self.seq,
                    forward: input.forward_move,
                    right: input.right_move,
                    up: input.up_move,
                    buttons: input.buttons & CONTROL_BUTTONS,
                    pitch: short(-camera.0.to_degrees()),
                    yaw: short(camera.1.to_degrees()),
                };
                let due = match self.sent {
                    None => true,
                    Some((last, at)) => {
                        let since = now.duration_since(at);
                        since >= FRAME_GAP
                            || (since >= FRAME_GAP_CHANGED && frame.presses_differ(&last))
                    }
                };
                if due {
                    self.seq = self.seq.wrapping_add(1);
                    let frame = Frame {
                        seq: self.seq,
                        ..frame
                    };
                    crate::player_identity::relay_toy_event(TOY, frame.event(), target, true);
                    self.sent = Some((frame, now));
                }
                Some(Drive {
                    command: still(input, view, delta),
                    view: None,
                })
            }
            Role::Controlled { frame, weapon, .. } => {
                let held = *weapon.get_or_insert(input.weapon);
                let Some((frame, at)) = frame else {
                    let view =
                        std::array::from_fn(|i| input.angles[i].wrapping_add(delta[i]) & 0xFFFF);
                    return Some(Drive {
                        command: still(input, view, delta),
                        view: None,
                    });
                };
                let fresh = now.duration_since(*at) < FRAME_STALE;
                Some(Drive {
                    command: steered(input, frame, fresh, delta, Some(held)),
                    view: Some([degrees(frame.pitch), degrees(frame.yaw)]),
                })
            }
            _ => None,
        }
    }

    /// Shape a command that is only shown, never sent (the prediction's preview between
    /// commands), as [`Hax::drive`] would, without sending anything.
    pub(crate) fn shape_preview(&self, command: &mut UserCommand, delta: [i32; 3]) {
        match &self.role {
            Role::Controlling {
                frozen: Some(view), ..
            } => *command = still(command, *view, delta),
            Role::Controlled {
                frame: Some((frame, at)),
                weapon,
                ..
            } => {
                let fresh = at.elapsed() < FRAME_STALE;
                *command = steered(command, frame, fresh, delta, *weapon);
            }
            _ => {}
        }
    }

    /// Whether the player is in a hack, as hacker or hacked (the Ragdoll waits).
    pub(crate) fn busy(&self) -> bool {
        !matches!(self.role, Role::Idle)
    }

    /// The slot the player steers, while steering.
    pub(crate) fn steering(&self) -> Option<u8> {
        match self.role {
            Role::Controlling { target, .. } => Some(target),
            _ => None,
        }
    }

    /// Forget any hack: no game, or another server.
    fn leave(&mut self) {
        self.server = None;
        self.role = Role::Idle;
        self.ready_at = i32::MIN;
        self.immune_until = i32::MIN;
        self.seen = None;
        self.hacked = [i32::MIN; SLOTS];
        self.traps.clear();
        self.sent = None;
    }

    /// Blow `slots` up at `at`, `fresh` when the news may come after their deaths.
    fn trap(&mut self, slots: u64, at: i32, fresh: bool) {
        if self
            .traps
            .iter()
            .any(|trap| trap.slots == slots && trap.at == at)
        {
            return;
        }
        if self.traps.len() >= 8 {
            self.traps.remove(0);
        }
        self.traps.push(TrapBlast {
            slots,
            at,
            blasted: 0,
            fresh,
        });
    }

    /// The trapped players newly dead at `now` in their trap's window blow up, each
    /// once: `seen` and `dead` are the snapshot's players and dead ones (bits). Their
    /// bits.
    fn trap_deaths(&mut self, seen: u64, dead: u64, now: i32) -> u64 {
        let newly = dead & !self.dead & seen;
        self.dead = (self.dead & !seen) | (dead & seen);
        let early = crate::order66::BLAST_EARLY_MS;
        let window = crate::order66::BLAST_WINDOW_MS;
        let mut blown = 0;
        for trap in &mut self.traps {
            if now >= trap.at - early && now <= trap.at + window {
                let candidates = if trap.fresh { dead & seen } else { newly };
                let hit = candidates & trap.slots & !trap.blasted;
                trap.blasted |= hit;
                blown |= hit;
            }
            trap.fresh = false;
        }
        self.traps
            .retain(|trap| now <= trap.at + window && now >= trap.at - 10 * TRAP_MS);
        blown
    }

    /// Note the game server time `now`; when its clock went back (a new map on the same
    /// server), forget the cooldown and the immunity, timed on the old clock. Whether it
    /// went back: the hack under way, timed on it too, is to be dropped.
    fn rewind(&mut self, now: i32) -> bool {
        let back = self.seen.is_some_and(|seen| now < seen - REWIND_MS);
        self.seen = Some(now);
        if back {
            self.ready_at = i32::MIN;
            self.immune_until = i32::MIN;
            self.hacked = [i32::MIN; SLOTS];
            self.traps.clear();
        }
        back
    }
}

/// What a feed toy event of the toy says of its sender: hacked until a moment (`Some`),
/// or free again (`None`); not one of these, nothing.
fn hacked_event(event: &str) -> Option<Option<i32>> {
    if event == FREED {
        return Some(None);
    }
    event.strip_prefix(HACKED_PREFIX)?.parse().ok().map(Some)
}

/// Mark `slot` hacked until `until`, or free (`None`).
fn mark(hacked: &mut [i32; SLOTS], slot: u8, until: Option<i32>) {
    if let Some(mark) = hacked.get_mut(usize::from(slot)) {
        *mark = until.unwrap_or(i32::MIN);
    }
}

/// The trap's moment a feed toy event of the toy names, if it is a trap.
fn trap_event(event: &str) -> Option<i32> {
    event.strip_prefix(TRAP_PREFIX)?.parse().ok()
}

/// Whether the player's trap is set (`cl_haxTrap`).
fn trap_set(console: Option<&crate::console::ViewerConsole>) -> bool {
    console.and_then(|console| console.bool_cvar(TRAP_CVAR)) == Some(true)
}

/// Whether the player may be hacked while away: the trap set and `cl_haxAway` on.
pub(crate) fn away_hackable(console: Option<&crate::console::ViewerConsole>) -> bool {
    trap_set(console) && console.and_then(|console| console.bool_cvar(AWAY_CVAR)) == Some(true)
}

/// Whether a player entity with `e_flags` is drawn and alive, so it may be hacked.
fn hackable_flags(e_flags: u32) -> bool {
    e_flags & (EF_DEAD | EF_NODRAW) == 0
}

/// Send `event` to the player in `slot`.
fn relay(event: Event, slot: u8) {
    crate::player_identity::relay_toy_event(TOY, event.text(), slot, false);
}

impl GpuState {
    /// `toy_hax`: turn the toy on or off.
    pub(crate) fn toggle_hax(&mut self) {
        self.hax.toggle();
        self.hax_centre(crate::toy_switch_words::switched("Hax", self.hax.on()));
    }

    /// Say `text` on the centre of the screen.
    fn hax_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// The name the game shows in `slot`, or "Someone".
    fn hax_name(&self, slot: u8) -> String {
        self.live_session
            .as_ref()
            .and_then(|session| {
                crate::player_identity::shown_name(session.game_state(), usize::from(slot))
            })
            .unwrap_or_else(|| "Someone".to_owned())
    }

    /// The frame's toy logic: the relay's events, the hack under way, the steering's end,
    /// the hack bar and the hacking sound.
    pub(crate) fn update_hax(
        &mut self,
        toy_events: &[sjk_identity::ToyEvent],
        events: &[sjk_identity::ToyEvent],
    ) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.end_hax_role();
            self.hax.leave();
            self.hud.set_toy_hack(None);
            return;
        };
        let server = session.server();
        if self.hax.server != Some(server) {
            self.end_hax_role();
            self.hax.leave();
            self.hax.server = Some(server);
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let game_state = session.game_state();
        let now = snapshot.server_time;
        let own = u8::try_from(game_state.client_num).ok();
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        // A view the server holds (a kiss, a grapple, a kick) ends a hack and refuses one:
        // the steering's angles fought it and the camera spun (#139, `held_view`).
        let spared = crate::order66::spared(&snapshot.player, gametype).or_else(|| {
            self.server_holds_view()
                .then_some(crate::order66::Spared::Held)
        });
        // A new map restarts the server's clock: a hack timed on the old one ends here,
        // the camera given back, without a word to the other player (their clock went
        // back too).
        if self.hax.rewind(now) {
            if let Role::Controlling {
                camera: Some((pitch, yaw)),
                ..
            } = self.hax.role
            {
                self.camera_pitch = pitch;
                self.camera_yaw = yaw;
            }
            self.hax.role = Role::Idle;
            self.hax.sent = None;
        }
        // The relay's events: a sender counts only when the game shows the name they
        // claimed, and never one blocked on this PC.
        let heard: Vec<(u8, Event)> = events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && !self.muted_players.hides_fun(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        for (slot, event) in heard {
            self.hax_event(slot, event, now, spared);
        }
        // Who is hacked, from the feed: for the rain round them. Taken from anyone the
        // game shows under the name they claimed, not blocked here, for at most a
        // steering's length.
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let game_state = session.game_state();
        for event in toy_events {
            if event.toy != TOY
                || Some(event.slot) == own
                || self.muted_players.hides_fun(u16::from(event.slot))
                || !crate::player_identity::shown_name(game_state, usize::from(event.slot))
                    .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            {
                continue;
            }
            if let Some(at) = trap_event(&event.event)
                && at > now - crate::order66::BLAST_WINDOW_MS
                && at <= now + TRAP_MS + 1_000
                && usize::from(event.target) < SLOTS
            {
                self.hax
                    .trap((1 << event.slot) | (1 << event.target), at, true);
                continue;
            }
            match hacked_event(&event.event) {
                Some(Some(until)) if until > now && until <= now + CONTROL_MS + 1_000 => {
                    mark(&mut self.hax.hacked, event.slot, Some(until));
                }
                Some(None) => mark(&mut self.hax.hacked, event.slot, None),
                _ => {}
            }
        }
        self.hax_hack(now, spared, own);
        self.hax_progress(now, spared);
        self.hax_trap_blasts();
        let bar = match self.hax.role {
            Role::Hacking { done_at, .. } | Role::Targeted { done_at, .. } => {
                Some((done_at - now) as f32 / HACK_MS as f32)
            }
            _ => None,
        };
        self.hud.set_toy_hack(bar);
    }

    /// One relayed event from the player in `slot`.
    fn hax_event(
        &mut self,
        slot: u8,
        event: Event,
        now: i32,
        spared: Option<crate::order66::Spared>,
    ) {
        let holds = self.looks.toy(usize::from(slot), TOY);
        match (event, self.hax.role) {
            // A hack begins on the player.
            (Event::Hack(done_at), Role::Idle)
                if self.hax.on
                    && holds
                    && now >= self.hax.immune_until
                    && (!self.hax_own_away() || away_hackable(self.console.as_ref())) =>
            {
                self.hax.role = Role::Targeted {
                    hacker: slot,
                    done_at,
                };
                let name = self.hax_name(slot);
                self.hax_centre(format!("^2{name}^7 is hacking you!\n^3Get away"));
            }
            (Event::Stop, Role::Targeted { hacker, .. }) if hacker == slot => {
                self.hax.role = Role::Idle;
            }
            (Event::Seize, role) => {
                let busy = !matches!(role, Role::Idle)
                    && !matches!(role, Role::Targeted { hacker, .. } if hacker == slot);
                let why = if !self.hax.on || !holds || busy {
                    Some(Refusal::Busy)
                } else if now < self.hax.immune_until {
                    Some(Refusal::Immune)
                } else if self.hax_own_away() && !away_hackable(self.console.as_ref()) {
                    Some(Refusal::Afk)
                } else {
                    spared.map(|spared| match spared {
                        crate::order66::Spared::Away | crate::order66::Spared::Dead => {
                            Refusal::Away
                        }
                        crate::order66::Spared::Duel => Refusal::Duel,
                        crate::order66::Spared::Flag => Refusal::Flag,
                        crate::order66::Spared::Held => Refusal::Busy,
                    })
                };
                if let Some(why) = why {
                    relay(Event::Refused(why), slot);
                    if matches!(role, Role::Targeted { hacker, .. } if hacker == slot) {
                        self.hax.role = Role::Idle;
                    }
                    return;
                }
                if trap_set(self.console.as_ref())
                    && let Some(own) = self.hax_own()
                {
                    let at = now + TRAP_MS;
                    relay(Event::Trap(at), slot);
                    // It's a Trap!: a hacker caught.
                    if self.toys_count() {
                        crate::achievements::note(crate::achievements::Counter::HaxTraps);
                    }
                    crate::player_identity::toy_event(TOY, &format!("{TRAP_PREFIX}{at}"), slot);
                    self.hax.trap((1 << own) | (1 << slot), at, false);
                    self.hax.role = Role::Trapped {
                        other: slot,
                        at,
                        shown: i32::MAX,
                    };
                    return;
                }
                let until = now + CONTROL_MS;
                relay(Event::Taken(until), slot);
                // Everyone else learns it through the feed, for the rain; the hub wants a
                // target other than the sender: the hacker.
                crate::player_identity::toy_event(TOY, &format!("{HACKED_PREFIX}{until}"), slot);
                if let Some(own) = self.hax_own() {
                    mark(&mut self.hax.hacked, own, Some(until));
                }
                self.hax.role = Role::Controlled {
                    hacker: slot,
                    until,
                    frame: None,
                    since: Instant::now(),
                    weapon: None,
                };
                self.hax.cues.push(SEIZE_SOUND);
                let name = self.hax_name(slot);
                self.hax_centre(format!(
                    "^1HACKED\n^2{name}^7 has your controls\n^3toy_hax^7 off breaks free"
                ));
            }
            (Event::Frame(frame), Role::Controlled { hacker, .. }) if hacker == slot => {
                if let Role::Controlled { frame: kept, .. } = &mut self.hax.role
                    && kept.is_none_or(|(old, _)| {
                        frame.seq > old.seq || old.seq - frame.seq > u32::MAX / 2
                    })
                {
                    *kept = Some((frame, Instant::now()));
                }
            }
            (Event::Stop, Role::Controlled { hacker, .. }) if hacker == slot => {
                self.hax_freed(now, false);
            }
            // The answers to the player's own seize.
            (Event::Taken(until), Role::Seizing { target, .. }) if target == slot => {
                self.hax.role = Role::Controlling {
                    target,
                    until,
                    frozen: None,
                    camera: None,
                };
                mark(&mut self.hax.hacked, target, Some(until));
                self.hax.sent = None;
                self.hax.cues.push(SEIZE_SOUND);
                let name = self.hax_name(slot);
                self.hax_centre(format!("^2HACKED^7\nYou steer {name}"));
            }
            (Event::Trap(at), Role::Seizing { target, .. }) if target == slot => {
                if let Some(own) = self.hax_own() {
                    self.hax.trap((1 << own) | (1 << target), at, false);
                }
                self.hax.role = Role::Trapped {
                    other: target,
                    at,
                    shown: i32::MAX,
                };
            }
            (Event::Refused(why), Role::Seizing { target, .. }) if target == slot => {
                self.hax.role = Role::Idle;
                let name = self.hax_name(slot);
                self.hax_centre(format!("^1Hack failed^7\n{name}^7 {}", why.words()));
            }
            (Event::Stop, Role::Controlling { target, .. } | Role::Seizing { target, .. })
                if target == slot =>
            {
                let name = self.hax_name(slot);
                self.end_hax_role();
                self.hax_centre(format!("{name}^7 broke free"));
            }
            _ => {}
        }
    }

    /// The player's own hack: one begins on a `+use` press facing a holder of the toy
    /// within reach, holds while `+use` does and the target stays near and in front, and
    /// is seized when its moment comes.
    fn hax_hack(&mut self, now: i32, spared: Option<crate::order66::Spared>, own: Option<u8>) {
        let pressed = std::mem::take(&mut self.hax.use_pressed);
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let from = Vec3::from_array(
            self.local_prediction
                .predicted_state()
                .map_or_else(|| snapshot.player.origin(), |state| state.origin),
        );
        let world = &self.live_world;
        let origin_of = |slot: u8| {
            world
                .entity(EntityId::new(u64::from(slot) + 1))
                .map(|entity| Vec3::from_array(entity.sample(i64::from(now)).translation))
        };
        match self.hax.role {
            Role::Idle if pressed && self.hax.on && spared.is_none() => {
                let looks = &self.looks;
                let candidates = snapshot.entities.iter().filter_map(|entity| {
                    let slot = u8::try_from(entity.number()).ok()?;
                    (usize::from(slot) < SLOTS
                        && Some(slot) != own
                        && entity.entity_type() == ET_PLAYER
                        && hackable_flags(entity.e_flags())
                        && looks.toy(usize::from(slot), TOY))
                    .then(|| Some((slot, origin_of(slot)?)))
                    .flatten()
                });
                let Some((target, _)) =
                    crate::cardboard_box::pick_target(from, self.camera_yaw, candidates)
                else {
                    return;
                };
                let typing = snapshot.entities.iter().any(|entity| {
                    u8::try_from(entity.number()) == Ok(target) && entity.e_flags() & EF_TALK != 0
                });
                let name = self.hax_name(target);
                let away_ok = self.looks.toy(usize::from(target), AWAY_TOY);
                if typing || (!away_ok && self.scoreboard.seen_away(target, &name)) {
                    let why = if typing { "is typing" } else { "is away" };
                    self.hax_centre(format!("{name}^7 {why}: no hacking them now"));
                    return;
                }
                if now < self.hax.ready_at {
                    let wait = (self.hax.ready_at - now + 999) / 1000;
                    self.hax_centre(format!("Hax again in {wait} s"));
                    return;
                }
                let done_at = now + HACK_MS;
                self.hax.role = Role::Hacking { target, done_at };
                relay(Event::Hack(done_at), target);
            }
            Role::Hacking { target, done_at } => {
                let facing = glam::Vec2::from_angle(self.camera_yaw);
                let holds = self.hax.on
                    && self.hax.use_held
                    && spared.is_none()
                    && self.looks.toy(usize::from(target), TOY)
                    && origin_of(target).is_some_and(|at| {
                        let way = (at - from).truncate();
                        at.distance(from) <= HOLD_REACH
                            && (way.length_squared() < 1.0
                                || way.normalize().dot(facing) >= HOLD_FACING)
                    });
                if !holds {
                    self.hax.role = Role::Idle;
                    relay(Event::Stop, target);
                } else if now >= done_at {
                    self.hax.role = Role::Seizing {
                        target,
                        since: Instant::now(),
                    };
                    relay(Event::Seize, target);
                }
            }
            _ => {}
        }
    }

    /// The ends that come with time: an unanswered seize, the steering's time, a warning
    /// that led nowhere, a hacked player whose hacker went quiet; and the player's own
    /// toy turned off or their player no longer playing.
    fn hax_progress(&mut self, now: i32, spared: Option<crate::order66::Spared>) {
        if let Role::Trapped { other, at, shown } = self.hax.role {
            if now >= at {
                // Both send `kill` now; a player already dead, or away from the match,
                // has nothing to say.
                self.hax.role = Role::Idle;
                self.hax.ready_at = now + COOLDOWN_MS;
                self.hax.immune_until = now + IMMUNE_MS;
                if !matches!(
                    spared,
                    Some(crate::order66::Spared::Away | crate::order66::Spared::Dead)
                ) && let Some(session) = self.live_session.as_mut()
                    && let Err(error) = session.send_reliable_command(b"kill")
                {
                    crate::log::progress(format_args!("hax: kill not sent: {error}"));
                }
                return;
            }
            let seconds = (at - now + 999) / 1000;
            if seconds != shown {
                self.hax.role = Role::Trapped {
                    other,
                    at,
                    shown: seconds,
                };
                self.hax.cues.push(crate::order66::TICK_SOUND);
                let name = self.hax_name(other);
                self.hax_centre(format!("^1IT'S A TRAP!\n^7{name}^7 and you\n^3{seconds}"));
            }
            return;
        }
        match self.hax.role {
            Role::Seizing { target, since } if since.elapsed() >= ANSWER_WAIT => {
                self.hax.role = Role::Idle;
                let name = self.hax_name(target);
                self.hax_centre(format!("^1Hack failed^7\n{name}^7 did not answer"));
            }
            Role::Controlling { until, .. } if now >= until => self.end_hax_role(),
            Role::Controlling { .. } | Role::Seizing { .. } | Role::Hacking { .. }
                if !self.hax.on || spared.is_some() =>
            {
                self.end_hax_role();
            }
            Role::Targeted { done_at, .. } if !self.hax.on || now > done_at + WARNING_GRACE_MS => {
                self.hax.role = Role::Idle;
            }
            Role::Controlled {
                until,
                frame,
                since,
                ..
            } => {
                let heard = frame.map_or(since, |(_, at)| at);
                if now >= until {
                    self.hax_freed(now, true);
                } else if !self.hax.on || spared.is_some() || heard.elapsed() >= FRAME_LOST {
                    self.hax_freed(now, false);
                }
            }
            _ => {}
        }
    }

    /// The player is free again: the hacker is told unless the time was up (`timely`),
    /// and the player cannot be hacked for [`IMMUNE_MS`].
    fn hax_freed(&mut self, now: i32, timely: bool) {
        if let Role::Controlled { hacker, .. } = self.hax.role {
            if !timely {
                relay(Event::Stop, hacker);
                crate::player_identity::toy_event(TOY, FREED, hacker);
            }
            if let Some(own) = self.hax_own() {
                mark(&mut self.hax.hacked, own, None);
            }
            self.hax.cues.push(RELEASE_SOUND);
            self.hax_centre("^2Free again".to_owned());
        }
        self.hax.role = Role::Idle;
        self.hax.immune_until = now + IMMUNE_MS;
    }

    /// End the player's own hack, steering included: the target is told, the camera
    /// turns back to where it was, and the cooldown starts after a steering.
    fn end_hax_role(&mut self) {
        let now = self
            .live_session
            .as_ref()
            .map_or(0, |session| session.latest_snapshot().server_time);
        match self.hax.role {
            Role::Hacking { target, .. } | Role::Seizing { target, .. } => {
                relay(Event::Stop, target);
            }
            Role::Controlling {
                target,
                until,
                camera,
                ..
            } => {
                if now < until {
                    relay(Event::Stop, target);
                }
                mark(&mut self.hax.hacked, target, None);
                if let Some((pitch, yaw)) = camera {
                    self.camera_pitch = pitch;
                    self.camera_yaw = yaw;
                }
                self.hax.cues.push(RELEASE_SOUND);
                self.hax.ready_at = now + COOLDOWN_MS;
            }
            Role::Controlled { .. } => {
                self.hax_freed(now, false);
                return;
            }
            _ => {}
        }
        self.hax.role = Role::Idle;
        self.hax.sent = None;
    }

    /// Whether the player is away from the keyboard, as their own client knows it: idle or
    /// alt-tabbed (`away_emote`), or wearing an away tag in their name (`afk`).
    pub(crate) fn hax_own_away(&self) -> bool {
        crate::away_emote::own_away()
            || self.hax_own().is_some_and(|own| {
                let name = self.hax_name(own);
                self.scoreboard.seen_away(own, &name)
            })
    }

    /// The traps' players: their limb meshes made ready while the trap counts down, and
    /// those who die in its window blown up.
    fn hax_trap_blasts(&mut self) {
        let Some((seen, dead, now)) = self.order66_seen_dead() else {
            return;
        };
        let ready = self
            .hax
            .traps
            .iter()
            .filter(|trap| now < trap.at)
            .fold(0, |ready, trap| ready | trap.slots)
            & seen;
        self.order66_ready(ready);
        let blown = self.hax.trap_deaths(seen, dead, now);
        self.order66_blow_up(blown);
    }

    /// The player's own slot, in a game.
    fn hax_own(&self) -> Option<u8> {
        u8::try_from(self.live_session.as_ref()?.game_state().client_num).ok()
    }

    /// Emit the digital rain round every hacked player the game shows (the own player
    /// at their predicted place), every [`GLYPHS_EVERY`]; a player blocked here gets none.
    pub(crate) fn spawn_hax_glyphs(
        &mut self,
        audio: &mut Option<crate::GameAudio>,
        presentation_time: i64,
    ) {
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let now = snapshot.server_time;
        if self.hax.hacked.iter().all(|&until| until <= now) {
            return;
        }
        let wall = Instant::now();
        if self
            .hax
            .glyphs_at
            .is_some_and(|at| wall.duration_since(at) < GLYPHS_EVERY)
        {
            return;
        }
        self.hax.glyphs_at = Some(wall);
        let Some(vfs) = self.vfs.clone() else {
            return;
        };
        let own = usize::try_from(session.game_state().client_num).ok();
        let mut origins = [None; SLOTS];
        for (slot, origin) in origins.iter_mut().enumerate() {
            if self.hax.hacked[slot] <= now || self.muted_players.hides_fun(slot as u16) {
                continue;
            }
            *origin = if Some(slot) == own {
                Some(self.local_prediction.predicted_state().map_or_else(
                    || Vec3::from_array(snapshot.player.origin()),
                    |state| Vec3::from_array(state.origin),
                ))
            } else if snapshot.entities.iter().any(|entity| {
                usize::from(entity.number()) == slot
                    && entity.entity_type() == ET_PLAYER
                    && hackable_flags(entity.e_flags())
            }) {
                self.live_world
                    .entity(EntityId::new(slot as u64 + 1))
                    .map(|entity| Vec3::from_array(entity.sample(presentation_time).translation))
            } else {
                None
            };
        }
        let up = crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]);
        for (slot, origin) in origins.into_iter().enumerate() {
            let Some(origin) = origin else {
                continue;
            };
            crate::effect_runtime::spawn_effect(
                &mut self.particles,
                &mut self.effect_aux,
                &mut self.effects,
                &vfs,
                GLYPHS,
                origin,
                wall,
                (presentation_time as u32) ^ (slot as u32).wrapping_mul(0x9e37_79b9),
                0,
                audio,
                up,
            );
        }
    }

    /// The hacking sound's loop this frame: on the hacker (in both ears for the own
    /// player), for the hacker and the player they hack.
    pub(crate) fn hax_loop(&self, presentation_time: i64) -> Option<(u8, Option<[f32; 3]>)> {
        match self.hax.role {
            Role::Hacking { .. } => {
                let own = self.live_session.as_ref()?.game_state().client_num;
                Some((u8::try_from(own).ok()?, None))
            }
            Role::Targeted { hacker, .. } => {
                let origin = self
                    .live_world
                    .entity(EntityId::new(u64::from(hacker) + 1))?
                    .sample(presentation_time)
                    .translation;
                Some((hacker, Some(origin)))
            }
            _ => None,
        }
    }

    /// Play the toy's one-shot sounds; they are handed to the audio the first time there
    /// is one.
    pub(crate) fn play_hax_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.hax.cues.clear();
            return;
        };
        if self.hax.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(
                vfs,
                [
                    HACK_SOUND,
                    SEIZE_SOUND,
                    RELEASE_SOUND,
                    crate::order66::TICK_SOUND,
                ],
            );
            self.hax.sounds_registered = Some(audio.generation());
        }
        let own = self
            .live_session
            .as_ref()
            .map_or(0, |session| session.game_state().client_num.max(0) as u32);
        for path in self.hax.cues.drain(..) {
            audio.play_toy(
                crate::toy_sounds::Toy::Hax,
                path,
                1.0,
                None,
                sjk_audio::SourceId(own),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: one puff of the rain round the player in `slot` of the demo
    /// playing.
    pub(crate) fn shot_hax_glyphs(&mut self, slot: u16) {
        let session = self.demo_session.as_ref().expect("a demo playing");
        let time = i64::from(session.latest_snapshot().server_time);
        let origin = session
            .world()
            .entity(EntityId::new(u64::from(slot) + 1))
            .expect("the player")
            .sample(time)
            .translation;
        let vfs = self.vfs.clone().expect("game data");
        let up = crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]);
        let seed = self.particles.len() as u32;
        crate::effect_runtime::spawn_effect(
            &mut self.particles,
            &mut self.effect_aux,
            &mut self.effects,
            &vfs,
            GLYPHS,
            Vec3::from_array(origin),
            Instant::now(),
            seed,
            0,
            &mut None,
            up,
        );
    }
}

/// The hacker's camera while steering: behind the steered player, turned as the
/// hacker's own camera is (the mouse answers at once, the player follows after the
/// relay's way); `None` otherwise, or while the game does not show them.
pub(crate) fn camera(state: &mut GpuState, time: i64) -> Option<(Vec3, Vec3)> {
    let target = state.hax.steering()?;
    let session = state.live_session.as_ref()?;
    let game = session.game_state();
    let snapshot = session.snapshot_at_or_before(time as i32);
    let shown = snapshot.entities.iter().any(|entity| {
        u8::try_from(entity.number()) == Ok(target) && entity.entity_type() == ET_PLAYER
    }) && snapshot.player.movement_flags() & PMF_FOLLOW == 0;
    if !shown {
        return None;
    }
    let origin = state
        .live_world
        .entity(EntityId::new(u64::from(target) + 1))?
        .sample(time)
        .translation;
    let (pitch, yaw) = (state.camera_pitch, state.camera_yaw);
    let forward = Vec3::new(
        yaw.cos() * pitch.cos(),
        yaw.sin() * pitch.cos(),
        pitch.sin(),
    );
    let focus = Vec3::from_array(origin) + Vec3::Z * CAMERA_HEIGHT;
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

    fn frame() -> Frame {
        Frame {
            seq: 12,
            forward: 127,
            right: -127,
            up: 0,
            buttons: BUTTON_ATTACK | BUTTON_WALKING,
            pitch: 65_000,
            yaw: 16_384,
        }
    }

    #[test]
    fn events_read_back_as_written_and_fit_the_relay() {
        let events = [
            Event::Hack(123_456),
            Event::Stop,
            Event::Seize,
            Event::Frame(frame()),
            Event::Frame(Frame {
                seq: u32::MAX,
                forward: -128,
                right: -128,
                up: -128,
                buttons: CONTROL_BUTTONS,
                pitch: u16::MAX,
                yaw: u16::MAX,
            }),
            Event::Taken(-5),
            Event::Refused(Refusal::Duel),
            Event::Refused(Refusal::Immune),
            Event::Refused(Refusal::Afk),
            Event::Trap(1_234_567),
        ];
        for event in events {
            let text = event.text();
            // The hub's rule: 1 to 64 of a-z, 0-9, _ and -.
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
    }

    #[test]
    fn a_frame_carries_only_the_allowed_buttons_and_nothing_else_parses() {
        let parsed = Event::parse("in_1_0_0_0_65535_0_0");
        let Some(Event::Frame(frame)) = parsed else {
            panic!("{parsed:?}");
        };
        assert_eq!(frame.buttons, CONTROL_BUTTONS);
        for bad in [
            "in_1_0_0_0_0_0",
            "in_1_0_0_0_0_0_0_0",
            "in_x_0_0_0_0_0_0",
            "hack_",
            "go",
        ] {
            assert_eq!(Event::parse(bad), None, "{bad}");
        }
        assert_eq!(
            Event::parse("refused_what"),
            Some(Event::Refused(Refusal::Busy))
        );
    }

    #[test]
    fn a_steered_command_turns_to_the_frames_view_through_the_delta() {
        let input = UserCommand {
            angles: [100, 200, 300],
            forward_move: -50,
            buttons: 512 | BUTTON_USE,
            weapon: 3,
            generic_command: 26,
            ..UserCommand::default()
        };
        let delta = [1_000, -2_000, 50];
        let command = steered(&input, &frame(), true, delta, Some(5));
        // The server adds the delta back: the frame's view, the player's own roll.
        let view: [i32; 3] =
            std::array::from_fn(|i| command.angles[i].wrapping_add(delta[i]) & 0xFFFF);
        assert_eq!(view, [65_000, 16_384, 350]);
        assert_eq!(
            (command.forward_move, command.right_move, command.up_move),
            (127, -127, 0)
        );
        assert_eq!(command.buttons, BUTTON_ATTACK | BUTTON_WALKING);
        assert_eq!((command.weapon, command.generic_command), (5, 0));
        // Out of date: the view holds, nothing is pressed.
        let stale = steered(&input, &frame(), false, delta, None);
        assert_eq!(stale.angles, command.angles);
        assert_eq!(
            (stale.forward_move, stale.right_move, stale.buttons),
            (0, 0, 0)
        );
        assert_eq!(stale.weapon, 3);
    }

    #[test]
    fn the_hacker_stands_still_at_the_view_taken_and_sends_frames_at_the_relays_pace() {
        let mut hax = Hax {
            role: Role::Controlling {
                target: 4,
                until: 10_000,
                frozen: None,
                camera: None,
            },
            ..Hax::default()
        };
        let delta = [10, 20, 0];
        let input = UserCommand {
            angles: [1_000, 2_000, 0],
            forward_move: 127,
            buttons: BUTTON_ATTACK | 512,
            ..UserCommand::default()
        };
        let drive = hax
            .drive(&input, delta, (0.5, 1.0))
            .expect("steering drives");
        assert_eq!(drive.view, None, "the camera stays the hacker's");
        assert_eq!(drive.command.angles, [1_000, 2_000, 0]);
        assert_eq!((drive.command.forward_move, drive.command.buttons), (0, 0));
        let (sent, _) = hax.sent.expect("the first frame goes at once");
        assert_eq!((sent.forward, sent.buttons), (127, BUTTON_ATTACK));
        assert_eq!(sent.yaw, short(1.0_f32.to_degrees()));
        assert_eq!(sent.pitch, short(-0.5_f32.to_degrees()));
        // The view stays the one taken, even when the input turns.
        let turned = UserCommand {
            angles: [5_000, 6_000, 0],
            ..input
        };
        let drive = hax.drive(&turned, delta, (0.0, 0.0)).unwrap();
        assert_eq!(drive.command.angles, [1_000, 2_000, 0]);
        assert_eq!(
            hax.sent.unwrap().0.seq,
            sent.seq,
            "too soon for the next frame"
        );
        assert!(matches!(
            hax.role,
            Role::Controlling {
                camera: Some((0.5, 1.0)),
                ..
            }
        ));
    }

    #[test]
    fn a_teleported_or_respawned_player_may_be_hacked() {
        const EF_TELEPORT_BIT: u32 = 1 << 3;
        assert!(hackable_flags(0));
        assert!(hackable_flags(EF_TELEPORT_BIT));
        assert!(!hackable_flags(EF_NODRAW));
        assert!(!hackable_flags(EF_DEAD | EF_TELEPORT_BIT));
    }

    #[test]
    fn the_feed_says_who_is_hacked_until_when_and_who_is_free() {
        assert_eq!(hacked_event("hacked_123456"), Some(Some(123_456)));
        assert_eq!(hacked_event("freed"), Some(None));
        for other in ["hacked_", "hacked_x", "hack_5", "spotted"] {
            assert_eq!(hacked_event(other), None, "{other}");
        }
        // The hub's rule for a toy event: 1 to 32 of a-z, 0-9 and _.
        let longest = format!("{HACKED_PREFIX}{}", i32::MAX);
        assert!(longest.len() <= 32);
        let mut hacked = [i32::MIN; SLOTS];
        mark(&mut hacked, 5, Some(9_000));
        mark(&mut hacked, 200, Some(9_000));
        assert_eq!(hacked[5], 9_000);
        mark(&mut hacked, 5, None);
        assert_eq!(hacked[5], i32::MIN);
    }

    #[test]
    fn the_bundled_files_mount_and_the_rain_reads() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        mount(&mut vfs).unwrap();
        for (path, _) in FILES {
            assert!(vfs.read(path).unwrap().is_some(), "{path}");
        }
        let effect = sjk_effect::load_effect(&vfs, GLYPHS).unwrap();
        assert_eq!(effect.components.len(), 1);
        let rain = &effect.components[0];
        assert!(
            rain.spawn_flags.origin_on_cylinder,
            "a ring round the player"
        );
        assert_eq!(rain.shaders.len(), 8);
        let script = vfs.read("shaders/sjk_hax.shader").unwrap().unwrap().bytes;
        let shaders = sjk_shader::parse_shader_script(&script, "shaders/sjk_hax.shader").unwrap();
        for name in &rain.shaders {
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
    }

    #[test]
    fn a_trap_blows_its_two_players_up_once_in_its_window() {
        let mut hax = Hax::default();
        let (a, b, c) = (1_u64 << 3, 1_u64 << 5, 1_u64 << 7);
        let seen = a | b | c;
        hax.trap(a | b, 10_000, false);
        hax.trap(a | b, 10_000, false);
        assert_eq!(hax.traps.len(), 1, "heard twice, kept once");
        // Before the window, a death does not count.
        assert_eq!(hax.trap_deaths(seen, a, 9_000), 0);
        assert_eq!(hax.trap_deaths(seen, 0, 9_500), 0);
        // In it, each of the two once; a third player never.
        assert_eq!(hax.trap_deaths(seen, a | c, 10_000), a);
        assert_eq!(hax.trap_deaths(seen, a | b | c, 10_100), b);
        assert_eq!(hax.trap_deaths(seen, 0, 10_200), 0);
        assert_eq!(hax.trap_deaths(seen, a | b, 10_300), 0, "once each");
        // Past the window, forgotten.
        assert_eq!(hax.trap_deaths(seen, 0, 12_100), 0);
        assert!(hax.traps.is_empty());
    }

    #[test]
    fn a_trap_heard_late_blows_up_the_already_dead() {
        let mut hax = Hax::default();
        let (a, b) = (1_u64 << 3, 1_u64 << 5);
        assert_eq!(
            hax.trap_deaths(a | b, a | b, 10_200),
            0,
            "dead before the news"
        );
        hax.trap(a | b, 10_000, true);
        assert_eq!(hax.trap_deaths(a | b, a | b, 10_400), a | b);
        assert_eq!(trap_event("trap_10000"), Some(10_000));
        assert_eq!(trap_event("hacked_10000"), None);
    }

    #[test]
    fn a_new_map_forgets_the_cooldown_and_the_immunity() {
        let mut hax = Hax::default();
        assert!(!hax.rewind(500_000));
        hax.ready_at = 510_000;
        hax.immune_until = 515_000;
        // Time going on, or a late snapshot, changes nothing.
        assert!(!hax.rewind(501_000));
        assert!(!hax.rewind(500_500));
        assert_eq!((hax.ready_at, hax.immune_until), (510_000, 515_000));
        // The map changed: the clock starts again.
        assert!(hax.rewind(2_000));
        assert_eq!((hax.ready_at, hax.immune_until), (i32::MIN, i32::MIN));
        assert!(!hax.rewind(2_100));
    }

    #[test]
    fn the_short_angle_round_trips() {
        for degrees_in in [-179.0_f32, -90.0, -1.0, 0.0, 45.0, 89.0, 179.0] {
            let back = degrees(short(degrees_in));
            assert!((back - degrees_in).abs() < 0.01, "{degrees_in} -> {back}");
        }
    }
}
