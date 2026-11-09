//! The box hunt's cheat watch (Sol, 01/11/2026): a player in a chase, the mark or a
//! hunter, who teleports themselves (JA+'s `amtele` and the `japlus.*` teleports that send
//! it, `setviewpos`) or flies in noclip or fake noclip is out of it at once. The hub sees
//! neither, so the player's own client watches its own player state, as it tells the hub
//! who killed the mark.
//!
//! Only the player's own doing counts (hub reports #235, #237, 03/11/2026): the map's
//! teleporters, a spawn and an admin moving the player (JA+'s `amtele <player> ...`, a
//! bring) flip the same `EF_TELEPORT_BIT`, and a mark brought by an admin blew up. So a
//! teleport is a cheat only when the bit flips within [`OWN_MOVE_WINDOW`] of a command of
//! [`OWN_MOVES`] that this client sent ([`sjk_client::sent_commands`]), and noclip only
//! after the player's own `noclip` on this map. Fake noclip (`fakenoclip`, the free
//! camera) is this client's own flight, never the server's, so it always counts.

use sjk_protocol::PlayerState;
use std::time::{Duration, Instant};

/// `EF_DEAD`, `EF_TELEPORT_BIT` (`bg_public.h`).
const EF_DEAD: u32 = 1 << 1;
const EF_TELEPORT_BIT: u32 = 1 << 3;
/// `PMF_FOLLOW`: the view is someone else's.
const PMF_FOLLOW: u16 = 0x1000;
/// `pm_type`: `PM_NOCLIP`, `PM_SPECTATOR`, `PM_DEAD`, `PM_INTERMISSION`,
/// `PM_SPINTERMISSION`.
const PM_NOCLIP: u8 = 3;
const PM_SPECTATOR: u8 = 4;
const PM_DEAD: u8 = 5;
const PM_INTERMISSION: u8 = 7;
const PM_SPINTERMISSION: u8 = 8;
/// How long after the player's own teleport command the teleport may show: JA+'s
/// `jp_starTrekTeleport 1` cloaks the player and beams them about two seconds later
/// (`JP_STARTREK_DELAY`, `g_japlus_tele.c`), and the snapshot comes a ping after.
const OWN_MOVE_WINDOW: Duration = Duration::from_secs(5);

/// What a command of the player's own does to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoveKind {
    /// Turns noclip on or off.
    Noclip,
    /// Teleports the player.
    Teleport,
    /// JA+'s `amtele`: teleports the player unless its first argument names another
    /// player in a form that moves them.
    Amtele,
    /// JA+'s `amghost`: walks through players, unseen, unless its argument names another
    /// player.
    Ghost,
}

/// A server command that moves the player who sends it.
struct OwnMove {
    /// The command's name, whatever the case.
    name: &'static str,
    kind: MoveKind,
}

/// The commands of the player's own that move them, one entry each, with its evidence (a
/// new mod's teleport is one more entry).
const OWN_MOVES: &[OwnMove] = &[
    // Base JA cheat: `Cmd_Noclip_f` (codemp/game/g_cmds.c) toggles the sender's noclip.
    OwnMove {
        name: "noclip",
        kind: MoveKind::Noclip,
    },
    // Base JA cheat: `Cmd_SetViewpos_f` (g_cmds.c) teleports the sender.
    OwnMove {
        name: "setviewpos",
        kind: MoveKind::Teleport,
    },
    // JA+ `Cmd_amtele_f` (VisionJK g_japlus_tele.c); `japlus.goto`, `japlus.guntele`,
    // `japlus.guntelewall` and the rest of crates/sjk-mod-japlus/src/teleport.rs send it.
    OwnMove {
        name: "amtele",
        kind: MoveKind::Amtele,
    },
    // JA+ `Cmd_amghost_f` (VisionJK g_japlus_fx.c): no argument is the sender (hub report
    // #240). A ghost shows in nothing the client receives, so the command is the evidence.
    OwnMove {
        name: "amghost",
        kind: MoveKind::Ghost,
    },
];

/// What `command` (a released reliable command) does to the player on slot `own`.
fn own_move(command: &[u8], own: u8) -> Option<MoveKind> {
    let text = String::from_utf8_lossy(command);
    let words: Vec<&str> = text.split_whitespace().collect();
    let name = words.first()?;
    let kind = OWN_MOVES
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))?
        .kind;
    if kind == MoveKind::Ghost {
        // A number names a slot; a name may be the player's own, so it counts.
        let other = words
            .get(1)
            .is_some_and(|target| target.parse::<u8>().is_ok_and(|slot| slot != own));
        return (!other).then_some(kind);
    }
    if kind != MoveKind::Amtele {
        return Some(kind);
    }
    // `amtele <player> <to player>` and `amtele <player> <x> <y> <z> <yaw>` move
    // `<player>`; the other forms move the sender. A number names a slot; a name may be
    // the player's own, so it counts.
    let moves_other =
        matches!(words.len(), 3 | 6) && words[1].parse::<u8>().is_ok_and(|slot| slot != own);
    (!moves_other).then_some(MoveKind::Teleport)
}

/// What the watch reads of one snapshot of the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Sample {
    /// The snapshot's server time.
    pub(super) time: i32,
    /// The player state is the player's own (not someone followed).
    pub(super) own: bool,
    /// Alive and in the game (not dead, spectating or at the intermission).
    pub(super) alive: bool,
    pub(super) noclip: bool,
    /// This client flies on its own (`fakenoclip`, the free camera).
    pub(super) fake_noclip: bool,
    /// `EF_TELEPORT_BIT`.
    pub(super) teleport: bool,
}

impl Sample {
    /// The sample of `player` (the snapshot at `time`), the player on slot `own`.
    pub(super) fn of(player: &PlayerState, time: i32, own: u8, fake_noclip: bool) -> Self {
        let kind = player.movement_type();
        let flags = player.entity_flags();
        Self {
            time,
            own: player.client_num() == u16::from(own) && player.movement_flags() & PMF_FOLLOW == 0,
            alive: player.health() > 0
                && flags & EF_DEAD == 0
                && !player.is_spectator()
                && ![PM_SPECTATOR, PM_DEAD, PM_INTERMISSION, PM_SPINTERMISSION].contains(&kind),
            noclip: kind == PM_NOCLIP,
            fake_noclip,
            teleport: flags & EF_TELEPORT_BIT != 0,
        }
    }
}

/// The player's last sample and own commands, to judge the next sample by.
#[derive(Default)]
pub(super) struct Watch {
    last: Option<Sample>,
    /// The number of the newest sent command already read.
    read: u64,
    /// When the player last sent a command that teleports them.
    teleported: Option<Instant>,
    /// The player sent `noclip` on this map.
    noclip: bool,
    /// When the player last ghosted themselves, not yet judged.
    ghosted: Option<Instant>,
}

impl Watch {
    /// Read the commands this client sent (`sent`, oldest first, numbered), the player on
    /// `own`. A new connection numbers its commands from 1 again.
    pub(super) fn read_commands<'a>(
        &mut self,
        sent: impl Iterator<Item = (u64, Instant, &'a [u8])> + Clone,
        own: u8,
    ) {
        if sent
            .clone()
            .last()
            .is_some_and(|(newest, ..)| newest < self.read)
        {
            self.read = 0;
        }
        for (number, at, command) in sent {
            if number <= self.read {
                continue;
            }
            self.read = number;
            match own_move(command, own) {
                Some(MoveKind::Noclip) => self.noclip = true,
                Some(MoveKind::Ghost) => self.ghosted = Some(at),
                Some(_) => self.teleported = Some(at),
                None => {}
            }
        }
    }

    /// Take `sample`, read at `now`: whether it shows the player cheating since the last.
    pub(super) fn step(&mut self, sample: Sample, now: Instant) -> bool {
        let last = self.last.replace(sample);
        if !sample.own || !sample.alive {
            self.last = None;
            return false;
        }
        if sample.fake_noclip || (sample.noclip && self.noclip) {
            return true;
        }
        if self
            .ghosted
            .take()
            .is_some_and(|at| now.saturating_duration_since(at) <= OWN_MOVE_WINDOW)
        {
            return true;
        }
        let Some(last) = last.filter(|last| last.time < sample.time) else {
            // The first sample, or a new map (its clock starts again).
            return false;
        };
        last.alive
            && last.teleport != sample.teleport
            && self
                .teleported
                .is_some_and(|at| now.saturating_duration_since(at) <= OWN_MOVE_WINDOW)
    }

    /// Forget the last sample and the player's noclip (the map changed); commands already
    /// read stay read.
    pub(super) fn reset(&mut self) {
        self.last = None;
        self.noclip = false;
        self.teleported = None;
        self.ghosted = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(time: i32, teleport: bool) -> Sample {
        Sample {
            time,
            own: true,
            alive: true,
            noclip: false,
            fake_noclip: false,
            teleport,
        }
    }

    #[test]
    fn a_teleport_of_the_map_or_an_admin_is_no_cheat() {
        let now = Instant::now();
        let mut watch = Watch::default();
        assert!(!watch.step(sample(100, false), now));
        // A map teleporter, or an admin's bring: the bit flips, no command of the own.
        assert!(!watch.step(sample(200, true), now));
        // The player's own chat, and their `amtele <player> ...` moving someone else.
        watch.read_commands(
            [
                (1, now, b"say hi".as_slice()),
                (2, now, b"amtele 3 4".as_slice()),
                (3, now, b"amtele 5 1 2 3 90".as_slice()),
            ]
            .into_iter(),
            4,
        );
        assert!(!watch.step(sample(300, false), now + Duration::from_secs(1)));
    }

    #[test]
    fn the_players_own_teleport_is_a_cheat() {
        let now = Instant::now();
        let mut watch = Watch::default();
        assert!(!watch.step(sample(100, false), now));
        watch.read_commands([(1, now, b"amtele 10 20 30 90".as_slice())].into_iter(), 4);
        // Sent but not shown yet: nothing.
        assert!(!watch.step(sample(150, false), now));
        // Beamed two seconds later (JA+'s Star Trek teleport): a cheat.
        assert!(watch.step(sample(200, true), now + Duration::from_secs(2)));
        // A map teleporter long after: no cheat.
        assert!(!watch.step(sample(300, false), now + Duration::from_secs(30)));
        // Read again, the same command does not count twice.
        let later = now + Duration::from_secs(31);
        watch.read_commands(
            [(1, later, b"amtele 10 20 30 90".as_slice())].into_iter(),
            4,
        );
        assert!(!watch.step(sample(400, true), later));
        // A new connection numbers its commands from 1 again: read.
        watch.read_commands(
            [(1, later, b"amtele 10 20 30 90".as_slice())].into_iter(),
            4,
        );
        assert!(!watch.step(sample(500, false), later), "nothing new yet");
        let mut watch = Watch {
            read: 7,
            ..Watch::default()
        };
        assert!(!watch.step(sample(100, false), later));
        watch.read_commands([(1, later, b"amtele".as_slice())].into_iter(), 4);
        assert!(watch.step(sample(200, true), later));
    }

    #[test]
    fn amtele_forms_that_move_the_sender() {
        assert_eq!(own_move(b"amtele", 4), Some(MoveKind::Teleport));
        assert_eq!(own_move(b"AMTELE 7", 4), Some(MoveKind::Teleport));
        assert_eq!(own_move(b"amtele 1 2 3 90", 4), Some(MoveKind::Teleport));
        assert_eq!(own_move(b"amtele 4 1 2 3 90", 4), Some(MoveKind::Teleport));
        assert_eq!(
            own_move(b"amtele Kyle 1 2 3 90", 4),
            Some(MoveKind::Teleport)
        );
        assert_eq!(own_move(b"amtele 5 1 2 3 90", 4), None);
        assert_eq!(own_move(b"amtele 5 4", 4), None);
        assert_eq!(own_move(b"setviewpos 1 2 3 0", 4), Some(MoveKind::Teleport));
        assert_eq!(own_move(b"noclip", 4), Some(MoveKind::Noclip));
        assert_eq!(own_move(b"say amtele", 4), None);
    }

    #[test]
    fn ghosting_oneself_is_a_cheat_and_ghosting_another_is_not() {
        assert_eq!(own_move(b"amghost", 4), Some(MoveKind::Ghost));
        assert_eq!(own_move(b"AMGHOST 4", 4), Some(MoveKind::Ghost));
        assert_eq!(own_move(b"amghost Kyle", 4), Some(MoveKind::Ghost));
        assert_eq!(own_move(b"amghost 5", 4), None);

        let now = Instant::now();
        let mut watch = Watch::default();
        assert!(!watch.step(sample(100, false), now));
        watch.read_commands([(1, now, b"amghost 5".as_slice())].into_iter(), 4);
        assert!(!watch.step(sample(150, false), now));
        watch.read_commands([(2, now, b"amghost".as_slice())].into_iter(), 4);
        assert!(watch.step(sample(200, false), now));
        // Judged once.
        assert!(!watch.step(sample(250, false), now));
        // Sent long before the chase began: not this chase's cheat.
        watch.read_commands([(3, now, b"amghost".as_slice())].into_iter(), 4);
        assert!(!watch.step(sample(300, false), now + Duration::from_secs(30)));
    }

    #[test]
    fn noclip_counts_when_the_player_asked_for_it() {
        let now = Instant::now();
        let flying = Sample {
            noclip: true,
            ..sample(100, false)
        };
        // An admin put the player in noclip: no cheat.
        let mut watch = Watch::default();
        assert!(!watch.step(flying, now));
        watch.read_commands([(1, now, b"noclip".as_slice())].into_iter(), 4);
        assert!(watch.step(
            Sample {
                time: 150,
                ..flying
            },
            now
        ));
        // A new map forgets it.
        watch.reset();
        assert!(!watch.step(Sample { time: 50, ..flying }, now));
        // Fake noclip is always the player's own.
        let fake = Sample {
            fake_noclip: true,
            ..sample(60, false)
        };
        assert!(watch.step(fake, now));
    }

    #[test]
    fn a_spawn_after_death_or_spectating_is_no_cheat() {
        let now = Instant::now();
        let mut watch = Watch::default();
        watch.read_commands([(1, now, b"amtele".as_slice())].into_iter(), 4);
        assert!(!watch.step(sample(100, false), now));
        let dead = Sample {
            alive: false,
            ..sample(150, false)
        };
        assert!(!watch.step(dead, now));
        assert!(!watch.step(sample(200, true), now));
        let followed = Sample {
            own: false,
            ..sample(250, false)
        };
        assert!(!watch.step(followed, now));
        assert!(!watch.step(sample(300, true), now));
    }
}
