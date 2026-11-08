//! The play signals every active claim carries (`docs/holocrons.md`, "Earning"; Rollembones'
//! idea, 03/11/2026): what the player really did since the hub last accepted a claim, so
//! the hub counts time toward holocrons by play (moving, attacking, hitting, being hit,
//! killing, dying) and not by presence alone.
//!
//! - **moved**: the horizontal distance the player's own origin travelled, summed per
//!   snapshot while alive in their own view; a step over [`TELEPORT`] units in one
//!   snapshot (a teleporter, a respawn) is left out.
//! - **attacks**: presses of attack or alt attack in the commands the client sends.
//! - **hits**: rises of the game's own hit counter (`PERS_HITS`).
//! - **hurt**: each snapshot whose health or armour fell while the last attacker
//!   (`PERS_ATTACKER`) is another client: falls and the wear above the maximum health
//!   are not counted.
//! - **kills** and **deaths**: obituaries of the player killing another client, and of
//!   another client killing the player.
//! - **bites**: bites of the fishing toy (`crate::fishing`) hooked in time (04/11/2026),
//!   so a player standing at the water earns while they answer the bites.
//!
//! Only counted while the player is actively playing ([`super::activity`]): the viewer
//! sets [`set_counting`] twice a second with the activity it tells the hub. The counts
//! are atomics; the per-snapshot watch lives on the thread that presents snapshots. No
//! allocation, a few comparisons a snapshot.

use sjk_client::ObituaryEvent;
use sjk_identity::PlaySignals;
use sjk_protocol::PlayerState;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// A step longer than this in one snapshot is a teleport or a respawn, not a move.
pub(crate) const TELEPORT: f32 = 512.0;
/// `BUTTON_ATTACK | BUTTON_ALT_ATTACK` (`codemp/qcommon/q_shared.h`).
const ATTACK_BUTTONS: u16 = 1 | 128;
/// `PERS_HITS`, `PERS_SPAWN_COUNT` and `PERS_ATTACKER` (`persEnum_t`, `bg_public.h`).
const PERS_HITS: usize = 1;
const PERS_SPAWN_COUNT: usize = 4;
const PERS_ATTACKER: usize = 6;
/// Client numbers below this are players; above are the world's.
const PLAYERS: u16 = 32;
/// `PMF_FOLLOW`.
const PMF_FOLLOW: u16 = 0x1000;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ATTACK_HELD: AtomicBool = AtomicBool::new(false);
static MOVED: AtomicU64 = AtomicU64::new(0);
static ATTACKS: AtomicU64 = AtomicU64::new(0);
static HITS: AtomicU64 = AtomicU64::new(0);
static HURT: AtomicU64 = AtomicU64::new(0);
static KILLS: AtomicU64 = AtomicU64::new(0);
static DEATHS: AtomicU64 = AtomicU64::new(0);
static BITES: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static WATCH: RefCell<Watch> = const { RefCell::new(Watch::new()) };
}

/// Whether what the player does counts now: the activity the claims say.
pub(crate) fn set_counting(on: bool) {
    COUNTING.store(on, Ordering::Relaxed);
}

/// A bite of the fishing toy hooked in time.
pub(crate) fn bite() {
    if COUNTING.load(Ordering::Relaxed) {
        BITES.fetch_add(1, Ordering::Relaxed);
    }
}

/// A command the client sends: a press of attack or alt attack is an attack.
pub(crate) fn command(buttons: u16) {
    let held = buttons & ATTACK_BUTTONS != 0;
    let was = ATTACK_HELD.swap(held, Ordering::Relaxed);
    if pressed(buttons, was) && COUNTING.load(Ordering::Relaxed) {
        ATTACKS.fetch_add(1, Ordering::Relaxed);
    }
}

/// Whether `buttons` press attack or alt attack anew, after a command that held one
/// (`was`) or not.
fn pressed(buttons: u16, was: bool) -> bool {
    buttons & ATTACK_BUTTONS != 0 && !was
}

/// A live snapshot's own player state, for client `me`, and the obituaries it brought.
pub(crate) fn snapshot(player: &PlayerState, me: i32, obituaries: &[ObituaryEvent]) {
    let counting = COUNTING.load(Ordering::Relaxed);
    let seen = WATCH.with(|watch| watch.borrow_mut().step(player, me, obituaries));
    if counting {
        add(&seen);
    }
}

fn add(seen: &PlaySignals) {
    for (counter, value) in [
        (&MOVED, seen.moved),
        (&ATTACKS, seen.attacks),
        (&HITS, seen.hits),
        (&HURT, seen.hurt),
        (&KILLS, seen.kills),
        (&DEATHS, seen.deaths),
    ] {
        if value > 0 {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }
}

/// Everything counted since the last call, the counters cleared.
pub(crate) fn take() -> PlaySignals {
    PlaySignals {
        moved: MOVED.swap(0, Ordering::Relaxed),
        attacks: ATTACKS.swap(0, Ordering::Relaxed),
        hits: HITS.swap(0, Ordering::Relaxed),
        hurt: HURT.swap(0, Ordering::Relaxed),
        kills: KILLS.swap(0, Ordering::Relaxed),
        deaths: DEATHS.swap(0, Ordering::Relaxed),
        bites: BITES.swap(0, Ordering::Relaxed),
    }
}

/// The last snapshot's numbers, to tell what changed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Watch {
    /// The player whose view it was, `None` before one.
    me: Option<i32>,
    spawn: u32,
    origin: [f32; 2],
    hits: u32,
    health: i32,
    armor: i32,
    /// Movement is fractional; whole units are sent.
    carry: f32,
}

impl Watch {
    pub(crate) const fn new() -> Self {
        Self {
            me: None,
            spawn: 0,
            origin: [0.0; 2],
            hits: 0,
            health: 0,
            armor: 0,
            carry: 0.0,
        }
    }

    /// What `player` (client `me`'s snapshot) and `obituaries` show since the last step.
    pub(crate) fn step(
        &mut self,
        player: &PlayerState,
        me: i32,
        obituaries: &[ObituaryEvent],
    ) -> PlaySignals {
        let mut seen = PlaySignals::default();
        if let Ok(me16) = u16::try_from(me) {
            for event in obituaries {
                let other = |client: u16| client != me16 && client < PLAYERS;
                if event.attacker == me16 && other(event.target) {
                    seen.kills += 1;
                }
                if event.target == me16 && other(event.attacker) {
                    seen.deaths += 1;
                }
            }
        }
        let own = i32::from(player.client_num()) == me
            && me >= 0
            && player.movement_flags() & PMF_FOLLOW == 0
            && !player.is_spectator();
        let origin = player.origin();
        let origin = [origin[0], origin[1]];
        let hits = player.persistent[PERS_HITS];
        let spawn = player.persistent[PERS_SPAWN_COUNT];
        let (health, armor) = (player.health(), player.armor());
        if !own {
            self.me = None;
            return seen;
        }
        if self.me == Some(me) && self.spawn == spawn {
            if health > 0 && self.health > 0 {
                let step = ((origin[0] - self.origin[0]).powi(2)
                    + (origin[1] - self.origin[1]).powi(2))
                .sqrt();
                if step.is_finite() && step <= TELEPORT {
                    self.carry += step;
                    let whole = self.carry.floor();
                    seen.moved = whole as u64;
                    self.carry -= whole;
                }
            }
            if hits > self.hits {
                seen.hits = u64::from(hits - self.hits);
            }
            let attacker = player.persistent[PERS_ATTACKER];
            let by_other = attacker < u32::from(PLAYERS) && i64::from(attacker) != i64::from(me);
            let max = player.max_health().max(1);
            let hurt = (health < self.health && self.health <= max) || armor < self.armor;
            if by_other && hurt && self.health > 0 {
                seen.hurt = 1;
            }
        }
        *self = Self {
            me: Some(me),
            spawn,
            origin,
            hits,
            health,
            armor,
            carry: self.carry,
        };
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: i32 = 3;

    fn state(at: [f32; 2], health: i32, configure: impl FnOnce(&mut PlayerState)) -> PlayerState {
        let mut player = PlayerState::default();
        player.set_raw_field(43, ME as u32);
        player.set_origin([at[0], at[1], 0.0]);
        player.stats[0] = health as u32;
        player.stats[8] = 100;
        configure(&mut player);
        player
    }

    fn kill(attacker: u16, target: u16) -> ObituaryEvent {
        ObituaryEvent {
            target,
            attacker,
            means_of_death: 1,
            message: "",
            attacker_message: None,
            local_fragged: false,
            local_was_killed: false,
            server_time: 0,
            attacker_force: 0,
        }
    }

    #[test]
    fn moves_hits_and_hurts_are_read_from_the_snapshots() {
        let mut watch = Watch::new();
        let first = watch.step(&state([0.0, 0.0], 100, |_| {}), ME, &[]);
        assert_eq!(
            first,
            PlaySignals::default(),
            "the first snapshot is the baseline"
        );
        let walked = watch.step(&state([30.0, 40.0], 100, |_| {}), ME, &[]);
        assert_eq!(walked.moved, 50);
        // A teleport is not a move.
        let jumped = watch.step(&state([3_000.0, 40.0], 100, |_| {}), ME, &[]);
        assert_eq!(jumped.moved, 0);
        // A hit counted by the game, and damage from another client.
        let fight = watch.step(
            &state([3_000.0, 40.0], 80, |player| {
                player.persistent[PERS_HITS] = 2;
                player.persistent[PERS_ATTACKER] = 7;
            }),
            ME,
            &[],
        );
        assert_eq!((fight.hits, fight.hurt), (2, 1));
        // A fall: the last attacker is the world.
        let fall = watch.step(
            &state([3_000.0, 40.0], 60, |player| {
                player.persistent[PERS_HITS] = 2;
                player.persistent[PERS_ATTACKER] = 1022;
            }),
            ME,
            &[],
        );
        assert_eq!((fall.hits, fall.hurt), (0, 0));
        // Self damage.
        let own = watch.step(
            &state([3_000.0, 40.0], 50, |player| {
                player.persistent[PERS_HITS] = 2;
                player.persistent[PERS_ATTACKER] = ME as u32;
            }),
            ME,
            &[],
        );
        assert_eq!(own.hurt, 0);
    }

    #[test]
    fn kills_and_deaths_count_other_players_only() {
        let mut watch = Watch::new();
        let player = state([0.0, 0.0], 100, |_| {});
        let seen = watch.step(
            &player,
            ME,
            &[
                kill(3, 5),
                kill(5, 3),
                kill(3, 3),
                kill(1022, 3),
                kill(5, 6),
            ],
        );
        assert_eq!((seen.kills, seen.deaths), (1, 1));
    }

    #[test]
    fn spectating_or_following_counts_nothing_and_a_respawn_starts_again() {
        let mut watch = Watch::new();
        watch.step(&state([0.0, 0.0], 100, |_| {}), ME, &[]);
        let following = state([100.0, 0.0], 100, |player| {
            player.set_raw_field(38, 0x1000);
        });
        assert_eq!(watch.step(&following, ME, &[]), PlaySignals::default());
        // Back in the own view: a new baseline, not a 100 unit move.
        assert_eq!(
            watch.step(&state([100.0, 0.0], 100, |_| {}), ME, &[]).moved,
            0
        );
        let respawned = state([150.0, 0.0], 100, |player| {
            player.persistent[PERS_SPAWN_COUNT] = 1;
        });
        assert_eq!(watch.step(&respawned, ME, &[]).moved, 0);
    }

    #[test]
    fn a_press_is_a_new_attack_or_alt_attack() {
        assert!(pressed(1, false));
        assert!(pressed(128, false));
        assert!(!pressed(1, true), "held");
        assert!(!pressed(0, false));
        assert!(!pressed(2, false), "another button");
    }
}
