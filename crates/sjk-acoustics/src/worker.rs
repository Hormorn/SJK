//! The acoustics' own threads: the game hands over a level and, each frame, where
//! the listener and the sounds are ([`Acoustics::update`]); the threads hand back
//! what reaches the listener from each sound and how the room rings
//! ([`Acoustics::poll`]). The game never waits for them.
//!
//! - `sjk-acoustics-direct` builds a new level's walls, then about
//!   [`DIRECT_HZ`] times a second traces every sound's straight way and corner.
//! - `sjk-acoustics-room` traces the room round the listener after each pass
//!   rests as long as the pass took, at least [`ROOM_INTERVAL`], so its threads
//!   never run flat out.

use crate::direct::DirectTracer;
use crate::material::ShaderMaterials;
use crate::room::{RoomSettings, RoomStats, RoomTracer};
use crate::scene::{LevelGeometry, Mover, PlacedMover, Scene};
use glam::Vec3;
use sjk_audio::{RoomResponse, SourceAcoustics, SourceId};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Passes of the straight ways a second.
pub const DIRECT_HZ: u32 = 20;
/// The least time between two passes of the room.
pub const ROOM_INTERVAL: Duration = Duration::from_millis(120);
/// A change of a source's acoustics smaller than this is not sent.
const SEND_EPSILON: f32 = 0.01;
/// How far out of a mover's box a sound inside it is moved (a door's sounds sit in
/// its middle, inside its own walls).
const OUT_OF_MOVER: f32 = 8.0;

/// Where everything is this frame, filled in place by [`Acoustics::update`].
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// The hearing point.
    pub listener: Vec3,
    /// The head's forward, right and up.
    pub axes: [Vec3; 3],
    pub sources: Vec<(SourceId, Vec3)>,
    pub movers: Vec<Mover>,
    pub room: RoomSettings,
    /// Whether the room is traced at all.
    pub reverb: bool,
}

/// What the threads found.
pub enum Output {
    Source(SourceId, SourceAcoustics),
    /// A new room response, `None` for a dry one (no level, or the reverb off).
    Room(Option<Box<RoomResponse>>, RoomStats),
    /// A level's walls are ready: triangles, of them with a named material, and
    /// the milliseconds it took.
    Level {
        triangles: usize,
        named: usize,
        millis: u64,
    },
}

#[derive(Default)]
struct State {
    snapshot: Snapshot,
    /// Bumped by every [`Acoustics::update`].
    serial: u64,
    /// A level to build, its key.
    pending: Option<(usize, Arc<sjk_bsp::Bsp>)>,
    level: Option<(usize, Arc<LevelGeometry>)>,
    /// Bumped when the level changes: the threads drop what they kept.
    level_serial: u64,
    stop: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

/// The running acoustics; dropping it stops the threads.
pub struct Acoustics {
    shared: Arc<Shared>,
    results: Receiver<Output>,
    threads: Vec<JoinHandle<()>>,
}

impl Acoustics {
    /// Start the threads.
    pub fn start() -> Option<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
        });
        let (sender, results) = channel();
        let direct = {
            let (shared, sender) = (Arc::clone(&shared), sender.clone());
            std::thread::Builder::new()
                .name("sjk-acoustics-direct".into())
                .spawn(move || direct_thread(&shared, &sender))
                .ok()?
        };
        let room = {
            let shared = Arc::clone(&shared);
            std::thread::Builder::new()
                .name("sjk-acoustics-room".into())
                .spawn(move || room_thread(&shared, &sender))
                .ok()?
        };
        Some(Self {
            shared,
            results,
            threads: vec![direct, room],
        })
    }

    /// Hear in `bsp`'s level from now on (`None`: out of a level). The same
    /// level again changes nothing.
    pub fn set_level(&self, bsp: Option<&Arc<sjk_bsp::Bsp>>) {
        let key = bsp.map_or(0, |bsp| Arc::as_ptr(bsp) as usize);
        let mut state = self.lock();
        let current = state
            .pending
            .as_ref()
            .map(|(key, _)| *key)
            .or(state.level.as_ref().map(|(key, _)| *key))
            .unwrap_or(0);
        if current == key {
            return;
        }
        state.level = None;
        state.pending = bsp.map(|bsp| (key, Arc::clone(bsp)));
        state.level_serial += 1;
        drop(state);
        self.shared.wake.notify_all();
    }

    /// Change this frame's snapshot in place (its vectors keep their room, so a
    /// frame allocates nothing once they have grown).
    pub fn update(&self, change: impl FnOnce(&mut Snapshot)) {
        let mut state = self.lock();
        change(&mut state.snapshot);
        state.serial += 1;
    }

    /// The next result, if any.
    pub fn poll(&self) -> Option<Output> {
        self.results.try_recv().ok()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Drop for Acoustics {
    fn drop(&mut self) {
        self.lock().stop = true;
        self.shared.wake.notify_all();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

fn lock(shared: &Shared) -> std::sync::MutexGuard<'_, State> {
    shared
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Wait up to `timeout` or until woken; whether to stop.
fn rest(shared: &Shared, timeout: Duration) -> bool {
    let state = lock(shared);
    if state.stop {
        return true;
    }
    let (state, _) = shared
        .wake
        .wait_timeout(state, timeout)
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.stop
}

/// The level and a copy of the snapshot, when there is a level.
fn take(shared: &Shared, into: &mut Snapshot) -> Option<(Arc<LevelGeometry>, u64)> {
    let state = lock(shared);
    let level = state.level.as_ref().map(|(_, level)| Arc::clone(level))?;
    let snapshot = &state.snapshot;
    into.listener = snapshot.listener;
    into.axes = snapshot.axes;
    into.room = snapshot.room;
    into.reverb = snapshot.reverb;
    into.sources.clear();
    into.sources.extend_from_slice(&snapshot.sources);
    into.movers.clear();
    into.movers.extend_from_slice(&snapshot.movers);
    Some((level, state.level_serial))
}

fn place_movers(level: &LevelGeometry, movers: &[Mover], out: &mut Vec<PlacedMover>) {
    out.clear();
    out.extend(
        movers
            .iter()
            .filter_map(|mover| PlacedMover::new(level, *mover)),
    );
}

fn direct_thread(shared: &Shared, sender: &Sender<Output>) {
    let materials = ShaderMaterials::bundled();
    let tracer = DirectTracer::default();
    let mut snapshot = Snapshot::default();
    let mut placed = Vec::new();
    let mut sent: HashMap<SourceId, SourceAcoustics> = HashMap::new();
    let mut corners: HashMap<SourceId, Vec3> = HashMap::new();
    let mut level_serial = 0;
    let period = Duration::from_secs(1) / DIRECT_HZ;
    loop {
        let started = Instant::now();
        let pending = lock(shared).pending.take();
        if let Some((key, bsp)) = pending {
            let geometry = LevelGeometry::from_bsp(&bsp, &materials);
            let _ = sender.send(Output::Level {
                triangles: geometry.triangle_count(),
                named: geometry.named_triangles,
                millis: started.elapsed().as_millis() as u64,
            });
            let mut state = lock(shared);
            // Unless another level was asked for meanwhile.
            if state.pending.is_none() {
                state.level = Some((key, Arc::new(geometry)));
            }
            drop(state);
            shared.wake.notify_all();
        }
        if let Some((level, serial)) = take(shared, &mut snapshot) {
            if serial != level_serial {
                level_serial = serial;
                sent.clear();
                corners.clear();
            }
            place_movers(&level, &snapshot.movers, &mut placed);
            let scene = Scene {
                geometry: &level,
                movers: &placed,
            };
            for &(source, position) in &snapshot.sources {
                let position = out_of_movers(&placed, snapshot.listener, position);
                let direct = tracer.trace(
                    &scene,
                    snapshot.listener,
                    position,
                    corners.get(&source).copied(),
                );
                match direct.acoustics.via {
                    Some(via) => {
                        corners.insert(source, Vec3::from_array(via));
                    }
                    None => {
                        corners.remove(&source);
                    }
                }
                let changed = sent
                    .get(&source)
                    .is_none_or(|last| differs(last, &direct.acoustics));
                if changed {
                    sent.insert(source, direct.acoustics);
                    let _ = sender.send(Output::Source(source, direct.acoustics));
                }
            }
        }
        let left = period.saturating_sub(started.elapsed());
        if rest(shared, left.max(Duration::from_millis(5))) {
            return;
        }
    }
}

fn differs(a: &SourceAcoustics, b: &SourceAcoustics) -> bool {
    let bands = a
        .bands
        .iter()
        .zip(&b.bands)
        .any(|(x, y)| (x - y).abs() >= SEND_EPSILON);
    let via = match (a.via, b.via) {
        (None, None) => false,
        (Some(x), Some(y)) => Vec3::from_array(x).distance(Vec3::from_array(y)) > 4.0,
        _ => true,
    };
    bands || via
}

/// A sound inside a mover's box, moved out along the line to the listener (unless
/// the listener is inside too), no further than halfway to them.
fn out_of_movers(movers: &[PlacedMover], listener: Vec3, position: Vec3) -> Vec3 {
    for mover in movers {
        let (low, high) = mover.world_box();
        let inside = |p: Vec3| p.cmpge(low).all() && p.cmple(high).all();
        if !inside(position) || inside(listener) {
            continue;
        }
        let delta = listener - position;
        let length = delta.length();
        if length < 1.0 {
            continue;
        }
        let direction = delta / length;
        let inverse = crate::bvh::safe_inverse(direction);
        let t1 = (low - position) * inverse;
        let t2 = (high - position) * inverse;
        let leave = t1.max(t2).min_element();
        return position + direction * (leave + OUT_OF_MOVER).min(length * 0.5);
    }
    position
}

fn room_thread(shared: &Shared, sender: &Sender<Output>) {
    let mut tracer = RoomTracer::new();
    let mut snapshot = Snapshot::default();
    let mut placed = Vec::new();
    let mut level_serial = 0;
    let mut dry = false;
    loop {
        let started = Instant::now();
        match take(shared, &mut snapshot) {
            Some((level, serial)) if snapshot.reverb => {
                if serial != level_serial {
                    level_serial = serial;
                    tracer.forget();
                }
                place_movers(&level, &snapshot.movers, &mut placed);
                let scene = Scene {
                    geometry: &level,
                    movers: &placed,
                };
                let (response, stats) =
                    tracer.trace(&scene, snapshot.listener, snapshot.axes, snapshot.room);
                let _ = sender.send(Output::Room(Some(Box::new(response)), stats));
                dry = false;
            }
            _ => {
                if !dry {
                    let _ = sender.send(Output::Room(None, RoomStats::default()));
                    dry = true;
                }
            }
        }
        // Rest at least as long as the pass took.
        let took = started.elapsed();
        if rest(shared, ROOM_INTERVAL.max(took)) {
            return;
        }
    }
}
