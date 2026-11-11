//! The Drone, SJK's toy (`toy_drone`; Sol's idea, 24/10/2026, from Slash's Peek work in
//! #199): the free camera and Peek fly a little training remote, the game's own seeker
//! ball ([`MODEL`]), which every other SJK player on the server sees with the pilot's
//! name over it.
//!
//! - `freecam` (`free_camera.rs`) flies the drone: it is where the camera is.
//! - `peek` (`peek.rs`) sends the drone to a player: it hovers behind them, where the
//!   peeking camera stands ([`crate::peek::behind`]). A player may be peeked only while
//!   their own Drone switch is on (the consent: their look shows [`TOY`]); turning it off
//!   sends any drone away at once (`away`, relayed to the pilot, as is an answer to a
//!   drone that comes while it is off). The watched player does not see the drones
//!   behind them; a line at the top of their screen names who watches.
//! - A player in a private duel does not see the drones of pilots outside it
//!   ([`duel_hidden_pilots`]), unless `cg_duelHideOthers` is 0.
//!
//! The switch (`toy_drone`, the Collection's Toys tab, the quick wheel) is only that
//! consent; `freecam` and `peek` keep their commands and binds, and fly the drone
//! whether it is on or not. It stays as last switched ([`crate::toy_saved::DRONE_CVAR`]).
//!
//! No game server knows of it. The pilot's client tells every other SJK player on the
//! server where the drone is through the SJK hub's toy relay (`PROTOCOL.md`, "Toy
//! relay"): `fly_<x>_<y>_<z>_<yaw>` while the camera flies, `peek_<slot>` while it
//! watches a player, `land` when it is put away. One event goes every [`SEND_GAP`], to
//! the players in turn, each one the drone's latest place or a repeat every
//! [`KEEPALIVE`]; a drone not heard of for [`LOST`] goes. Each client glides the drones
//! it shows between the places it hears of. Players on stock clients see nothing.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec3};
use sjk_runtime::EntityId;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// The toy's console command: the consent switch.
pub(crate) const COMMAND: &str = "toy_drone";
/// The toy's id in a look's `toys` (the consent) and in relayed toy events.
pub(crate) const TOY: &str = "drone";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_drone.png";
/// The drone: Jedi Academy's own seeker remote, a ball about six units across.
pub(crate) const MODEL: &str = "models/items/remote.md3";
/// Heard where a drone comes out: the game's remote hiss.
pub(crate) const LAUNCH_SOUND: &str = "sound/chars/remote/misc/hiss.wav";
/// The drone's size against the stock ball: the game's own.
const SCALE: f32 = 1.0;
/// Shortest time between two events sent (the relay takes 40 a second from a player,
/// which the Hax and the Ragdoll share).
const SEND_GAP: Duration = Duration::from_millis(50);
/// A player told of the drone hears it again this often while it does not move.
const KEEPALIVE: Duration = Duration::from_millis(1_000);
/// A drone not heard of this long goes.
const LOST: Duration = Duration::from_millis(3_000);
/// A flying drone moved this far, or turned this much (degrees), is told again.
const MOVED: i32 = 2;
const TURNED: i32 = 3;
/// How fast a shown drone glides to its latest place (per second), and the jump past
/// which it is placed at once.
const FOLLOW: f32 = 9.0;
const SNAP: f32 = 768.0;
/// Seconds to appear or go.
const FADE: f32 = 0.35;
/// Its hover: up and down, and the period in seconds.
const BOB: f32 = 1.5;
const BOB_PERIOD: f32 = 1.9;
/// Names show over drones this close at most.
const NAME_RANGE: f32 = 2_400.0;
/// The name's place above the drone's centre.
const NAME_RISE: f32 = 10.0;
/// The watched player's line, by its top, against the screen's height.
const WATCHED_TOP: f32 = 0.14;
/// The watched player's line's text id (the names over drones are their slots).
const WATCHED_TEXT: u32 = SLOTS as u32;
const SLOTS: usize = crate::looks::SLOTS;

/// A bundled file's bytes, by its name in `assets/drone`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/drone/", $name))
    };
}

/// The bundled files at their game paths (made by `scripts/drone_assets.py`).
const FILES: [(&str, &[u8]); 1] = [(ICON, bundled!("toy_drone.png"))];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Drone", FILES.iter().copied())?;
    Ok(())
}

/// Where a drone is, as its pilot tells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flight {
    /// Flown by the free camera: its place in whole units and its yaw in degrees.
    Fly { at: [i32; 3], yaw: i32 },
    /// Watching the player in this slot (Peek).
    Peek(u8),
}

impl Flight {
    /// Whether a player told of `self` must hear of `now`.
    fn differs(self, now: Self) -> bool {
        match (self, now) {
            (
                Self::Fly { at, yaw },
                Self::Fly {
                    at: to,
                    yaw: turned,
                },
            ) => {
                at.iter().zip(to).any(|(a, b)| (a - b).abs() >= MOVED)
                    || (yaw - turned)
                        .rem_euclid(360)
                        .min((turned - yaw).rem_euclid(360))
                        >= TURNED
            }
            (old, new) => old != new,
        }
    }
}

/// A relayed event of the toy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    /// From the pilot: where the drone is.
    Flight(Flight),
    /// From the pilot: the drone is put away.
    Land,
    /// From a watched player to the pilot: they do not let drones follow them (their
    /// switch is off, or was just turned off), so the peek ends.
    Away,
}

const FLY_PREFIX: &str = "fly_";
const PEEK_PREFIX: &str = "peek_";
const LAND: &str = "land";
const AWAY: &str = "away";

impl Event {
    fn text(self) -> String {
        match self {
            Self::Flight(Flight::Fly { at: [x, y, z], yaw }) => {
                format!("{FLY_PREFIX}{x}_{y}_{z}_{yaw}")
            }
            Self::Flight(Flight::Peek(slot)) => format!("{PEEK_PREFIX}{slot}"),
            Self::Land => LAND.to_owned(),
            Self::Away => AWAY.to_owned(),
        }
    }

    fn parse(event: &str) -> Option<Self> {
        if event == LAND {
            return Some(Self::Land);
        }
        if event == AWAY {
            return Some(Self::Away);
        }
        if let Some(slot) = event.strip_prefix(PEEK_PREFIX) {
            return slot
                .parse()
                .ok()
                .filter(|&slot| usize::from(slot) < SLOTS)
                .map(|slot| Self::Flight(Flight::Peek(slot)));
        }
        let mut numbers = event
            .strip_prefix(FLY_PREFIX)?
            .split('_')
            .map(|word| word.parse::<i32>().ok().filter(|n| n.abs() <= 1 << 17));
        let at = [numbers.next()??, numbers.next()??, numbers.next()??];
        let yaw = numbers.next()??.rem_euclid(360);
        numbers
            .next()
            .is_none()
            .then_some(Self::Flight(Flight::Fly { at, yaw }))
    }
}

/// Another player's drone as this client shows it.
#[derive(Clone, Copy, Debug, Default)]
struct Shown {
    /// Its latest place as heard; `None` once landed or lost (it then fades out).
    flight: Option<Flight>,
    heard: Option<Instant>,
    /// Where it is drawn, without the hover, and which way it faces (radians).
    position: Option<Vec3>,
    yaw: f32,
    /// 0 gone, 1 fully there.
    level: f32,
}

/// The toy's state: the switch, what the players were told of the own drone, and the
/// other players' drones.
pub(crate) struct Drone {
    on: bool,
    server: Option<SocketAddr>,
    /// What each slot was last told and when; `None` told nothing (or `land`).
    told: [Option<(Flight, Instant)>; SLOTS],
    /// The next slot to tell, and when.
    next: usize,
    send_at: Option<Instant>,
    others: [Shown; SLOTS],
    /// Who watches the player now (bits by slot).
    watchers: u64,
    /// Where each shown drone is this frame, for its name; and the names.
    placed: [Option<Vec3>; SLOTS],
    names: [String; SLOTS],
    /// The watched player's line ("Name is spectating you with the Drone").
    watched_line: String,
    /// Where a drone came out this frame, to be heard, by its pilot's slot.
    launches: Vec<(u8, Vec3)>,
    /// The audio generation the launch sound was registered with.
    sounds_registered: Option<u64>,
    /// The names over the drones, for the HUD pass.
    pub(crate) list: DrawList,
    last: Option<Instant>,
    /// The player the own drone watches (Peek) at the last frame, and the last one it
    /// watched with when it landed (the peek ended), for Eye in the Sky and Special
    /// Delivery.
    peeking: Option<u16>,
    landed: Option<(u16, Instant)>,
    /// A world shot placed the drones: the frame leaves them be without a server.
    #[cfg(test)]
    shot: bool,
}

impl Default for Drone {
    fn default() -> Self {
        Self {
            on: false,
            server: None,
            told: [None; SLOTS],
            next: 0,
            send_at: None,
            others: [Shown::default(); SLOTS],
            watchers: 0,
            placed: [None; SLOTS],
            names: std::array::from_fn(|_| String::new()),
            watched_line: String::new(),
            launches: Vec::new(),
            sounds_registered: None,
            list: DrawList::new(SLOTS * 2 + 2),
            last: None,
            peeking: None,
            landed: None,
            #[cfg(test)]
            shot: false,
        }
    }
}

impl Drone {
    /// The player the own drone last watched, and when it landed.
    pub(crate) fn landed(&self) -> Option<(u16, Instant)> {
        self.landed
    }

    /// The own drone watches `watching` at `now`: a watch that ended (or moved to another
    /// player) is a landing.
    fn follow_peek(&mut self, watching: Option<u16>, now: Instant) {
        if let Some(watched) = self.peeking
            && watching != Some(watched)
        {
            self.landed = Some((watched, now));
        }
        self.peeking = watching;
    }

    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the player lets drones follow them.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Forget everything of the server: no game, or another one.
    fn leave(&mut self) {
        let on = self.on;
        *self = Self {
            on,
            ..Self::default()
        };
    }
}

/// Send `event` to the player in `slot`; a newer place replaces one still waiting.
fn relay(event: Event, slot: u8) {
    crate::player_identity::relay_toy_event(TOY, event.text(), slot, true);
}

/// Ease in and out.
/// The pilots (bits by slot) whose drones `player` does not see: everyone outside
/// their private duel while they (or the player they follow) duel, unless `mode` is
/// Off (Sanji's report #145: drones flying about distract a duel). Drones are SJK's
/// own, so this holds on every server, not only on those whose bystanders
/// `cg_duelHideOthers`'s Auto hides.
fn duel_hidden_pilots(player: &sjk_protocol::PlayerState, mode: sjk_client::DuelHideMode) -> u64 {
    if mode == sjk_client::DuelHideMode::Off || !player.duel_in_progress() {
        return 0;
    }
    let mut duellists = 0u64;
    for slot in [player.client_num(), player.duel_index()] {
        if usize::from(slot) < SLOTS {
            duellists |= 1 << slot;
        }
    }
    !duellists
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl GpuState {
    /// `toy_drone`: let drones follow the player, or not.
    pub(crate) fn toggle_drone(&mut self) {
        self.drone.toggle();
        if !self.drone.on() {
            self.drone_send_away(self.drone.watchers);
        }
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            crate::toy_switch_words::switched("Drone", self.drone.on()),
            None,
            Instant::now(),
        );
    }

    /// Where the player's own drone is now: flown by the free camera, watching a player,
    /// or put away.
    fn own_flight(&self) -> Option<Flight> {
        if self.free_camera_active() {
            let at = self.camera_position.to_array().map(|c| c.round() as i32);
            let yaw = (self.camera_yaw.to_degrees().round() as i32).rem_euclid(360);
            return Some(Flight::Fly { at, yaw });
        }
        self.peek
            .watching()
            .and_then(|slot| u8::try_from(slot).ok())
            .map(Flight::Peek)
    }

    /// The frame's toy logic: tell the other SJK players where the own drone is, and
    /// take in where theirs are.
    pub(crate) fn update_drone(&mut self, relay_events: &[sjk_identity::ToyEvent]) {
        #[cfg(test)]
        if self.drone.shot {
            return;
        }
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.drone.leave();
            return;
        };
        let server = session.server();
        if self.drone.server != Some(server) {
            self.drone.leave();
            self.drone.server = Some(server);
        }
        let game_state = session.game_state();
        let own = u8::try_from(game_state.client_num).ok();
        let now = Instant::now();
        self.drone.follow_peek(self.peek.watching(), now);
        // The relay's events: a sender counts only when the game shows the name they
        // claimed, and never one blocked on this PC.
        let heard: Vec<(u8, Event)> = relay_events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && usize::from(event.slot) < SLOTS
                    && !self.muted_players.hides_fun(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        for (slot, event) in heard {
            let shown = &mut self.drone.others[usize::from(slot)];
            match event {
                Event::Flight(flight) => {
                    shown.flight = Some(flight);
                    shown.heard = Some(now);
                    // Watching the player, who does not let it: told so at each
                    // repeat (one a second) until the peek ends.
                    if Some(flight) == own.map(Flight::Peek) && !self.drone.on() {
                        relay(Event::Away, slot);
                    }
                }
                Event::Land => shown.flight = None,
                Event::Away => {
                    if self.peek.watching() == Some(u16::from(slot)) {
                        self.peek.stop();
                        let name = self
                            .live_session
                            .as_ref()
                            .and_then(|session| {
                                crate::player_identity::shown_name(
                                    session.game_state(),
                                    usize::from(slot),
                                )
                            })
                            .unwrap_or_else(|| "Someone".to_owned());
                        self.chat.receive(
                            sjk_client::ServerEventKind::CenterPrint,
                            format!("{name}^7 sent your drone away"),
                            None,
                            now,
                        );
                    }
                }
            }
        }
        for shown in &mut self.drone.others {
            if shown
                .heard
                .is_none_or(|heard| now.saturating_duration_since(heard) > LOST)
            {
                shown.flight = None;
            }
        }
        // Who watches the player (named by the line at the top of the screen).
        self.drone.watchers = own.map_or(0, |own| {
            self.drone
                .others
                .iter()
                .enumerate()
                .filter(|(_, shown)| shown.flight == Some(Flight::Peek(own)))
                .fold(0_u64, |bits, (slot, _)| bits | 1 << slot)
        });
        self.drone_tell(own, now);
    }

    /// Tell the pilots of the drones in `slots` (bits) that the player lets none follow.
    fn drone_send_away(&self, slots: u64) {
        for slot in 0..SLOTS {
            if slots & 1 << slot != 0 {
                relay(Event::Away, slot as u8);
            }
        }
    }

    /// Tell the next SJK player on the server who must hear of the own drone.
    fn drone_tell(&mut self, own: Option<u8>, now: Instant) {
        let Some(own) = own else {
            return;
        };
        if self.drone.send_at.is_some_and(|at| now < at) {
            return;
        }
        let flight = self.own_flight();
        let sjk = self.looks.sjk_slots() & !(1 << own);
        // One who left is told again should they come back.
        for (slot, told) in self.drone.told.iter_mut().enumerate() {
            if sjk & 1 << slot == 0 {
                *told = None;
            }
        }
        let due = |slot: usize| -> Option<Event> {
            if sjk & 1 << slot == 0 {
                return None;
            }
            match (self.drone.told[slot], flight) {
                (None, None) => None,
                (Some(_), None) => Some(Event::Land),
                (None, Some(flight)) => Some(Event::Flight(flight)),
                (Some((told, at)), Some(flight)) => (told.differs(flight)
                    || now.saturating_duration_since(at) >= KEEPALIVE)
                    .then_some(Event::Flight(flight)),
            }
        };
        let start = self.drone.next;
        let Some((slot, event)) = (0..SLOTS)
            .map(|step| (start + step) % SLOTS)
            .find_map(|slot| due(slot).map(|event| (slot, event)))
        else {
            return;
        };
        self.drone.told[slot] = match event {
            Event::Flight(flight) => Some((flight, now)),
            Event::Land | Event::Away => None,
        };
        self.drone.next = (slot + 1) % SLOTS;
        self.drone.send_at = Some(now + SEND_GAP);
        relay(event, slot as u8);
    }

    /// Where a shown drone should be this frame and which way it faces (radians), or
    /// `None` while the place it names is not to be seen (a watched player out of view).
    fn drone_target(&mut self, flight: Flight, time: i64) -> Option<(Vec3, f32)> {
        match flight {
            Flight::Fly { at, yaw } => Some((
                Vec3::from_array(at.map(|c| c as f32)),
                (yaw as f32).to_radians(),
            )),
            Flight::Peek(slot) => {
                let own = u8::try_from(self.drone_game()?.client_num).ok();
                let (origin, yaw) = if Some(slot) == own {
                    let session = self.live_session.as_ref()?;
                    let origin = self
                        .local_prediction
                        .committed_state()
                        .map_or(session.latest_snapshot().player.origin(), |state| {
                            state.origin
                        });
                    (Vec3::from_array(origin), self.camera_yaw)
                } else {
                    let entity = self.live_world.entity(EntityId::new(u64::from(slot) + 1))?;
                    let yaw = entity
                        .sample_pose(time)
                        .map_or(0., |pose| pose.view_angles_degrees[1])
                        .to_radians();
                    (Vec3::from_array(entity.sample(time).translation), yaw)
                };
                let focus = origin + Vec3::Z * crate::peek::FOCUS_HEIGHT;
                Some((crate::peek::behind(self, time, focus, yaw), yaw))
            }
        }
    }

    /// Add this frame's drones: every other player's that shows, gliding to its place.
    pub(crate) fn submit_drones(&mut self, presentation_time: i64, now: Instant) {
        let dt = self
            .drone
            .last
            .map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f32()
            })
            .min(0.1);
        self.drone.last = Some(now);
        self.drone.placed = [None; SLOTS];
        let mesh = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == MODEL);
        let seconds = presentation_time as f32 * 0.001;
        // A drone watching the player is not seen by them (it stands in their back, where
        // a peeking camera would): the line at the top of the screen says who.
        let own = self
            .drone_game()
            .and_then(|game| u8::try_from(game.client_num).ok())
            .map(Flight::Peek);
        // Nor are the drones of pilots outside the player's private duel.
        let duel_hidden = match (&self.live_session, &self.console) {
            (Some(session), Some(console)) => {
                duel_hidden_pilots(&session.latest_snapshot().player, console.duel_hide().mode)
            }
            _ => 0,
        };
        // Every other pilot's, when the player hides other players' drones
        // (Collection > Toys); who peeks at them is still told.
        let duel_hidden = if crate::toy_sight::seen(self.console.as_ref(), crate::toy_sight::DRONE)
        {
            duel_hidden
        } else {
            u64::MAX
        };
        for slot in 0..SLOTS {
            let shown = self.drone.others[slot];
            if (own.is_some() && shown.flight == own) || duel_hidden & 1 << slot != 0 {
                let shown = &mut self.drone.others[slot];
                shown.level = 0.0;
                shown.position = None;
                continue;
            }
            if shown.flight.is_none() && shown.level <= 0.0 {
                continue;
            }
            let target = shown
                .flight
                .and_then(|flight| self.drone_target(flight, presentation_time));
            let shown = &mut self.drone.others[slot];
            let was_out = shown.level <= 0.0;
            let wanted = if target.is_some() { 1.0 } else { 0.0 };
            shown.level = if shown.level < wanted {
                (shown.level + dt / FADE).min(wanted)
            } else {
                (shown.level - dt / FADE).max(wanted)
            };
            if let Some((point, yaw)) = target {
                let position = match shown.position {
                    Some(position) if !was_out && position.distance(point) < SNAP => {
                        position + (point - position) * (1.0 - (-dt * FOLLOW).exp())
                    }
                    _ => {
                        self.drone.launches.push((slot as u8, point));
                        point
                    }
                };
                shown.position = Some(position);
                let turn = (yaw - shown.yaw + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                shown.yaw += turn * (1.0 - (-dt * FOLLOW).exp());
            }
            if shown.level <= 0.0 {
                shown.position = None;
                continue;
            }
            let Some(position) = shown.position else {
                continue;
            };
            let phase = slot as f32 * 0.41;
            let bob = ((seconds + phase) * std::f32::consts::TAU / BOB_PERIOD).sin() * BOB;
            let centre = position + Vec3::Z * bob;
            self.drone.placed[slot] = Some(centre);
            let Some(mesh) = mesh else {
                continue;
            };
            let level = smooth(shown.level);
            let wobble = ((seconds + phase) * 1.3).sin() * 0.12;
            let rotation = Quat::from_rotation_z(shown.yaw) * Quat::from_rotation_x(wobble);
            self.object_groups[mesh].push(ActorInstance::new(
                centre.to_array(),
                rotation.to_array(),
                [SCALE * level; 3],
            ));
        }
    }

    /// Hear the drones that came out this frame.
    pub(crate) fn play_drone_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.drone.launches.clear();
            return;
        };
        if self.drone.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [LAUNCH_SOUND]);
            self.drone.sounds_registered = Some(audio.generation());
        }
        for (slot, point) in self.drone.launches.drain(..) {
            audio.play_toy(
                crate::toy_sounds::Toy::Drone,
                LAUNCH_SOUND,
                0.8,
                Some(point.to_array()),
                sjk_audio::SourceId(u32::from(slot)),
                sjk_audio::ChannelId(0),
            );
        }
    }

    /// Lay the names over the shown drones out over this frame's view and add their
    /// words to the HUD's text; nothing while `shown` is false.
    pub(crate) fn append_drone_names(&mut self, viewport: [f32; 2], shown: bool) {
        self.drone.list.clear();
        let Some(camera) = self.frame_view.filter(|_| shown) else {
            return;
        };
        let Some(game_state) = self.drone_game() else {
            return;
        };
        // The pilots' names, without their colours.
        let names: Vec<(usize, String)> = (0..SLOTS)
            .filter(|&slot| self.drone.placed[slot].is_some())
            .map(|slot| {
                let name = crate::player_identity::shown_name(game_state, slot).unwrap_or_default();
                let name = sjk_game_jka::client_view::strip_colours(name.as_bytes());
                (slot, String::from_utf8_lossy(&name).into_owned())
            })
            .collect();
        for (slot, name) in names {
            self.drone.names[slot] = name;
        }
        let unit = crate::ui_scale::height_scale(viewport[1]);
        self.append_watched_line(viewport, unit);
        let size = 16.0 * unit;
        let width = (260.0 * unit).min(viewport[0]);
        for slot in 0..SLOTS {
            let Some(centre) = self.drone.placed[slot] else {
                continue;
            };
            let point = centre + Vec3::Z * NAME_RISE;
            if point.distance(camera.eye) > NAME_RANGE {
                continue;
            }
            let Some(at) = camera.project(point) else {
                continue;
            };
            let trace = self.bsp.trace_box_with(
                &mut self.trace_scratch,
                camera.eye.to_array(),
                centre.to_array(),
                sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                1,
            );
            if trace.fraction < 1.0 || trace.start_solid || trace.all_solid {
                continue;
            }
            let alpha = smooth(self.drone.others[slot].level);
            let x = (at[0] - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
            let y = at[1] - size * 1.4;
            for (offset, color) in [
                (1.5 * unit, Color::new(0.0, 0.0, 0.0, 0.7 * alpha)),
                (0.0, Color::new(1.0, 1.0, 1.0, alpha)),
            ] {
                let _ = self.drone.list.push(DrawCommand::Text {
                    rect: Rect::new(x + offset, y + offset, width, size * 1.4),
                    text: TextId(slot as u32),
                    size,
                    color,
                    align: TextAlign::Center,
                    overflow: TextOverflow::Ellipsis,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
        }
        if self.drone.list.commands().is_empty() {
            return;
        }
        let drone = &self.drone;
        crate::ui_renderer::append_text_commands(
            &drone.list,
            |id| match id.0 {
                WATCHED_TEXT => drone.watched_line.as_str(),
                slot => drone.names.get(slot as usize).map_or("", String::as_str),
            },
            &mut self.text_vertices,
            &self.ui_font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }

    /// The watched player's line, while drones watch them: "Name is spectating you with
    /// the Drone" (names joined when several do).
    fn append_watched_line(&mut self, viewport: [f32; 2], unit: f32) {
        let Some(game_state) = self.drone_game() else {
            return;
        };
        let names: Vec<String> = (0..SLOTS)
            .filter(|&slot| self.drone.watchers & 1 << slot != 0)
            .map(|slot| {
                let name = crate::player_identity::shown_name(game_state, slot)
                    .unwrap_or_else(|| "Someone".to_owned());
                let name = sjk_game_jka::client_view::strip_colours(name.as_bytes());
                String::from_utf8_lossy(&name).into_owned()
            })
            .collect();
        if names.is_empty() {
            return;
        }
        let verb = if names.len() == 1 { "is" } else { "are" };
        self.drone.watched_line =
            format!("{} {verb} spectating you with the Drone", names.join(", "));
        let size = 16.0 * unit;
        let y = viewport[1] * WATCHED_TOP;
        for (offset, color) in [
            (1.5 * unit, Color::new(0.0, 0.0, 0.0, 0.7)),
            (0.0, Color::new(1.0, 1.0, 1.0, 0.9)),
        ] {
            let _ = self.drone.list.push(DrawCommand::Text {
                rect: Rect::new(offset, y + offset, viewport[0], size * 1.4),
                text: TextId(WATCHED_TEXT),
                size,
                color,
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight: FontWeight::Semibold,
                letter_spacing: 0.0,
            });
        }
    }

    /// The game the drones fly in: the server's (a world shot's demo too).
    fn drone_game(&self) -> Option<&sjk_protocol::GameState> {
        let live = self
            .live_session
            .as_ref()
            .map(|session| session.game_state());
        #[cfg(test)]
        let live = live.or_else(|| {
            self.demo_session
                .as_ref()
                .map(crate::demo_playback::Session::game_state)
        });
        live
    }

    /// Whether the player in `slot` lets drones follow them (always, in a local game,
    /// where no look is shared).
    pub(crate) fn drone_allowed(&self, slot: u16) -> bool {
        self.live_session
            .as_ref()
            .is_some_and(|session| session.is_local())
            || self.looks.toy(usize::from(slot), TOY)
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: the player in `slot` flies a drone at `at`, facing `yaw`
    /// (degrees), or (`at` `None`) peeks at the player in `watched`.
    pub(crate) fn shot_drone(&mut self, slot: u8, at: Option<[f32; 3]>, yaw: i32, watched: u8) {
        self.drone.shot = true;
        let shown = &mut self.drone.others[usize::from(slot)];
        shown.flight = Some(at.map_or(Flight::Peek(watched), |at| Flight::Fly {
            at: at.map(|c| c.round() as i32),
            yaw,
        }));
        shown.heard = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_read_back_as_written_and_fit_the_relay() {
        for event in [
            Event::Flight(Flight::Fly {
                at: [-131_072, 4_096, -12],
                yaw: 359,
            }),
            Event::Flight(Flight::Peek(31)),
            Event::Land,
            Event::Away,
        ] {
            let text = event.text();
            assert!(text.len() <= 64, "{text}");
            assert_eq!(Event::parse(&text), Some(event), "{text}");
        }
        for wrong in [
            "fly_1_2",
            "fly_1_2_3_4_5",
            "fly_a_2_3_4",
            "peek_64",
            "peek_",
            "lands",
        ] {
            assert_eq!(Event::parse(wrong), None, "{wrong}");
        }
        // A yaw out of range comes back in it.
        assert_eq!(
            Event::parse("fly_0_0_0_-90"),
            Some(Event::Flight(Flight::Fly {
                at: [0; 3],
                yaw: 270
            }))
        );
    }

    #[test]
    fn a_drone_is_told_again_once_it_moved_or_turned_enough() {
        let at = Flight::Fly {
            at: [0, 0, 0],
            yaw: 1,
        };
        assert!(!at.differs(Flight::Fly {
            at: [1, -1, 1],
            yaw: 359
        }));
        assert!(at.differs(Flight::Fly {
            at: [2, 0, 0],
            yaw: 1
        }));
        assert!(at.differs(Flight::Fly {
            at: [0, 0, 0],
            yaw: 358
        }));
        assert!(at.differs(Flight::Peek(1)));
        assert!(!Flight::Peek(1).differs(Flight::Peek(1)));
    }

    #[test]
    fn a_duel_hides_the_drones_of_pilots_outside_it() {
        use sjk_client::DuelHideMode;
        let mut player = sjk_protocol::PlayerState::default();
        player.set_client_num(3);
        assert_eq!(duel_hidden_pilots(&player, DuelHideMode::Auto), 0);
        player.set_raw_field(119, 1);
        player.set_raw_field(44, 5);
        let hidden = duel_hidden_pilots(&player, DuelHideMode::Auto);
        assert_eq!(hidden & (1 << 3 | 1 << 5), 0);
        assert_ne!(hidden & 1 << 4, 0);
        assert_eq!(hidden.count_ones() as usize, 64 - 2);
        assert_eq!(duel_hidden_pilots(&player, DuelHideMode::Always), hidden);
        assert_eq!(duel_hidden_pilots(&player, DuelHideMode::Off), 0);
    }

    #[test]
    fn the_bundled_icon_mounts() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        mount(&mut vfs).unwrap();
        let icon = vfs.read(ICON).unwrap().unwrap();
        let decoded = image::load_from_memory(&icon.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (128, 128));
    }
}
