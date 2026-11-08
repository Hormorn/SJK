//! Status balloons (`cg_statusBalloons`, Sol's idea, 08/11/2026): the talk balloon over
//! an SJK player shows why it is up: the three dots lit in turn while they type in the
//! chat, a pause sign while a menu or the console is open, a moon with rising z's while
//! their window is alt-tabbed or minimised.
//!
//! The balloon itself stays the server's: `EF_TALK`, which the player's `BUTTON_TALK`
//! sets ([`sjk_game_jka::pmove_talk`]), decides whether one shows, as in stock, so a
//! balloon here is never up when stock's would not be (a JA+ blade spares a player under
//! it). Only the reason travels, as one toy relay event (`docs/hub-chat.md`, "Toy
//! relay") to each SJK player on the server when it changes ([`TOY`], [`Reason::word`]);
//! the others, and any player whose reason has not come, keep the stock balloon. A
//! reason is kept until its player is seen without the balloon, so it needs no event when
//! it ends.

use std::time::{Duration, Instant};

/// The relay's toy id. The hub knows no toys: any id of `a` to `z` and `_` goes.
pub(crate) const TOY: &str = "status_balloon";
/// The setting: draw the reasons (off by default: stock's balloon).
pub(crate) const CVAR: &str = "cg_statusBalloons";
/// Client slots.
pub(crate) const SLOTS: usize = crate::looks::SLOTS;
/// One relay event at most this often, as the other toys' ([`crate::banana::SEND_GAP`]),
/// well under the hub's 40 a second per key.
const SEND_GAP: Duration = Duration::from_millis(50);
/// A reason is told once it has held this long, so a menu flicked open and shut, or
/// the console passed through, sends nothing.
const HOLD: Duration = Duration::from_millis(200);
/// A reason heard this recently is kept even though its player is seen without the
/// balloon: the relay may be quicker than the game's snapshots.
const GRACE: Duration = Duration::from_millis(1500);

/// Why a player's talk balloon is up.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Reason {
    /// The chat field is open.
    Typing,
    /// A menu (the game menu, a pop-up, the HUD editor) is open.
    Menu,
    /// The console is open.
    Console,
    /// The window is alt-tabbed or minimised (`cl_unfocusedChatbox`).
    Away,
}

impl Reason {
    const ALL: [Self; 4] = [Self::Typing, Self::Menu, Self::Console, Self::Away];

    /// The relay event's word.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Typing => "typing",
            Self::Menu => "menu",
            Self::Console => "console",
            Self::Away => "away",
        }
    }

    /// The reason a relay event names, if it names one.
    pub(crate) fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.word() == word)
    }

    /// The balloon drawn for it. A menu and the console share the pause sign: to the
    /// others both mean "not playing for a moment", and three pictures read better from
    /// afar than four.
    pub(crate) const fn shader(self) -> &'static str {
        match self {
            Self::Typing => SHADERS[0],
            Self::Menu | Self::Console => SHADERS[1],
            Self::Away => SHADERS[2],
        }
    }
}

/// The balloons' shaders (`assets/status_balloons/status_balloons.shader`), which the
/// frame's effect materials load ([`crate::effect_assets::CODE_SHADERS`]).
pub(crate) const SHADERS: [&str; 3] = [
    "gfx/sjk/status/typing",
    "gfx/sjk/status/busy",
    "gfx/sjk/status/away",
];

/// The pictures, at their game paths, and their shader (made by
/// `scripts/status_balloon_assets.py`, but the shader).
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/status_balloons/", $name))
    };
}
const FILES: [(&str, &[u8]); 8] = [
    (
        "shaders/sjk_status_balloons.shader",
        bundled!("status_balloons.shader"),
    ),
    ("gfx/sjk/status/typing_1.png", bundled!("typing_1.png")),
    ("gfx/sjk/status/typing_2.png", bundled!("typing_2.png")),
    ("gfx/sjk/status/typing_3.png", bundled!("typing_3.png")),
    ("gfx/sjk/status/busy.png", bundled!("busy.png")),
    ("gfx/sjk/status/away_1.png", bundled!("away_1.png")),
    ("gfx/sjk/status/away_2.png", bundled!("away_2.png")),
    ("gfx/sjk/status/away_3.png", bundled!("away_3.png")),
];

/// Mount the bundled balloon files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK status balloons", FILES.iter().copied())?;
    Ok(())
}

/// What the own client tells: its reason, since when, and the slots told it.
#[derive(Debug, Default)]
struct Outbox {
    reason: Option<Reason>,
    since: Option<Instant>,
    /// Bits by slot: the players told the current reason.
    told: u64,
    next_send: Option<Instant>,
}

impl Outbox {
    /// Follow the own reason at `now`; a new one is told to everyone afresh.
    fn follow(&mut self, reason: Option<Reason>, now: Instant) {
        if reason != self.reason {
            self.reason = reason;
            self.since = Some(now);
            self.told = 0;
        }
    }

    /// The next slot to tell, and what, at `now`: among `others` (the other SJK players,
    /// bits by slot) one not told yet, those `in_view` (bits) first. Marks it told.
    fn next(&mut self, others: u64, in_view: u64, now: Instant) -> Option<(u8, Reason)> {
        // One who left is told again should they come back.
        self.told &= others;
        let reason = self.reason?;
        if self.since.is_none_or(|since| now < since + HOLD)
            || self.next_send.is_some_and(|at| now < at)
        {
            return None;
        }
        let untold = others & !self.told;
        let pick = if untold & in_view != 0 {
            untold & in_view
        } else {
            untold
        };
        if pick == 0 {
            return None;
        }
        let slot = pick.trailing_zeros() as u8;
        self.told |= 1 << slot;
        self.next_send = Some(now + SEND_GAP);
        Some((slot, reason))
    }
}

/// The reasons heard from the other SJK players, by slot, with when each came.
#[derive(Debug)]
struct Heard([Option<(Reason, Instant)>; SLOTS]);

impl Default for Heard {
    fn default() -> Self {
        Self([None; SLOTS])
    }
}

impl Heard {
    fn hear(&mut self, slot: u8, reason: Reason, now: Instant) {
        if let Some(entry) = self.0.get_mut(usize::from(slot)) {
            *entry = Some((reason, now));
        }
    }

    /// Forget the reasons of the players seen without the balloon (`seen`: bits by slot
    /// in this snapshot, `talking`: those of them under it), unless they came within
    /// [`GRACE`].
    fn follow(&mut self, seen: u64, talking: u64, now: Instant) {
        for (slot, entry) in self.0.iter_mut().enumerate() {
            let quiet = (seen & !talking) & (1 << slot) != 0;
            if quiet && entry.is_some_and(|(_, at)| now.duration_since(at) > GRACE) {
                *entry = None;
            }
        }
    }
}

/// The reason drawn over each slot this frame, for the balloon's sprite
/// (`player_sprites.rs`); none draws stock's.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shown(
    [Option<Reason>; SLOTS],
    /// World shots: the balloon drawn over these players whatever their `EF_TALK`.
    #[cfg(test)]
    pub(crate) bool,
);

impl Default for Shown {
    fn default() -> Self {
        Self(
            [None; SLOTS],
            #[cfg(test)]
            false,
        )
    }
}

impl Shown {
    /// World shots: `reason` over every player, with or without the balloon.
    #[cfg(test)]
    pub(crate) fn for_shot(reason: Reason) -> Self {
        Self([Some(reason); SLOTS], true)
    }

    /// The balloon's shader for the player in `slot`, if their reason is known.
    pub(crate) fn shader(&self, slot: u16) -> Option<&'static str> {
        self.0
            .get(usize::from(slot))
            .copied()
            .flatten()
            .map(Reason::shader)
    }
}

/// The status balloons' state, kept on the client.
#[derive(Debug, Default)]
pub(crate) struct StatusBalloons {
    /// The game server the state is for.
    pub(crate) server: Option<std::net::SocketAddr>,
    outbox: Outbox,
    heard: Heard,
    /// This frame's balloons ([`Shown`]).
    pub(crate) shown: Shown,
    /// World shots: drawn instead of `shown`, in a demo too.
    #[cfg(test)]
    pub(crate) shot: Option<Shown>,
}

impl StatusBalloons {
    /// Forget everything: no game, or another server.
    pub(crate) fn leave(&mut self, server: Option<std::net::SocketAddr>) {
        *self = Self {
            server,
            ..Self::default()
        };
    }

    /// One frame at `now`: the own `reason`, the reasons `heard` (slot and reason, already
    /// checked), the players `seen` in the snapshot and those of them `talking` (bits by
    /// slot). Returns the event to relay, if one is due, for the SJK players `others`
    /// (bits), those `seen` first. `draw` is the setting; `own` the own slot, whose
    /// balloon shows its own reason.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn frame(
        &mut self,
        now: Instant,
        reason: Option<Reason>,
        heard: impl IntoIterator<Item = (u8, Reason)>,
        seen: u64,
        talking: u64,
        others: u64,
        own: Option<u8>,
        draw: bool,
    ) -> Option<(u8, Reason)> {
        for (slot, reason) in heard {
            self.heard.hear(slot, reason, now);
        }
        self.heard.follow(seen, talking, now);
        self.outbox.follow(reason, now);
        self.shown = Shown::default();
        if draw {
            for (shown, heard) in self.shown.0.iter_mut().zip(self.heard.0.iter()) {
                *shown = heard.map(|(reason, _)| reason);
            }
            if let Some(slot) = own.filter(|slot| usize::from(*slot) < SLOTS) {
                self.shown.0[usize::from(slot)] = reason;
            }
        }
        let others = own.map_or(others, |own| others & !(1 << own));
        self.outbox.next(others, seen, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn their_shaders_are_loaded_with_the_effects_and_bundled() {
        // Missing from the list, the sprite would draw the stand-in picture.
        for shader in SHADERS {
            assert!(
                crate::effect_assets::CODE_SHADERS.contains(&shader),
                "{shader}"
            );
        }
        let source = std::str::from_utf8(FILES[0].1).unwrap();
        for shader in SHADERS {
            assert!(source.contains(shader), "{shader}");
        }
        for (path, bytes) in &FILES[1..] {
            assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']), "{path}");
            let picture = path.trim_end_matches(".png");
            assert!(source.contains(picture), "{picture} unused");
        }
    }

    #[test]
    fn words_read_back_and_fit_the_relay() {
        for reason in Reason::ALL {
            let word = reason.word();
            assert_eq!(Reason::parse(word), Some(reason));
            assert!(
                word.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            );
        }
        assert!(TOY.len() <= 32 && TOY.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'));
        assert_eq!(Reason::parse("dancing"), None);
        assert_eq!(Reason::Menu.shader(), Reason::Console.shader());
    }

    #[test]
    fn a_reason_is_told_once_to_each_player_after_it_holds_those_in_view_first() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        let others = 1 << 2 | 1 << 5 | 1 << 9;
        let seen = 1 << 9;
        let step = |balloons: &mut StatusBalloons, at: Duration| {
            balloons.frame(
                start + at,
                Some(Reason::Typing),
                [],
                seen,
                0,
                others,
                Some(1),
                true,
            )
        };
        // Not yet held long enough.
        assert_eq!(step(&mut balloons, Duration::ZERO), None);
        assert_eq!(step(&mut balloons, Duration::from_millis(100)), None);
        // The one in view first, then by slot, one a gap.
        let at = Duration::from_millis(250);
        assert_eq!(step(&mut balloons, at), Some((9, Reason::Typing)));
        assert_eq!(step(&mut balloons, at), None);
        assert_eq!(
            step(&mut balloons, at + SEND_GAP),
            Some((2, Reason::Typing))
        );
        assert_eq!(
            step(&mut balloons, at + SEND_GAP * 2),
            Some((5, Reason::Typing))
        );
        assert_eq!(step(&mut balloons, at + SEND_GAP * 3), None);
    }

    #[test]
    fn a_new_reason_is_told_again_and_a_newcomer_is_told_the_current_one() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        let mut at = start;
        let mut tick = |balloons: &mut StatusBalloons, reason, others| {
            at += Duration::from_millis(300);
            balloons.frame(at, reason, [], 0, 0, others, Some(0), true)
        };
        tick(&mut balloons, Some(Reason::Menu), 1 << 3);
        assert_eq!(
            tick(&mut balloons, Some(Reason::Menu), 1 << 3),
            Some((3, Reason::Menu))
        );
        assert_eq!(tick(&mut balloons, Some(Reason::Menu), 1 << 3), None);
        // Someone joins: told the reason still up.
        assert_eq!(
            tick(&mut balloons, Some(Reason::Menu), 1 << 3 | 1 << 4),
            Some((4, Reason::Menu))
        );
        // The reason ends: nothing to tell.
        assert_eq!(tick(&mut balloons, None, 1 << 3 | 1 << 4), None);
        assert_eq!(tick(&mut balloons, None, 1 << 3 | 1 << 4), None);
        // Another starts: everyone again.
        tick(&mut balloons, Some(Reason::Away), 1 << 3 | 1 << 4);
        assert_eq!(
            tick(&mut balloons, Some(Reason::Away), 1 << 3 | 1 << 4),
            Some((3, Reason::Away))
        );
        // The own slot is never told.
        let mut own = StatusBalloons::default();
        own.frame(start, Some(Reason::Typing), [], 0, 0, 1 << 6, Some(6), true);
        let later = start + Duration::from_secs(1);
        assert_eq!(
            own.frame(later, Some(Reason::Typing), [], 0, 0, 1 << 6, Some(6), true),
            None
        );
    }

    #[test]
    fn a_heard_reason_shows_until_its_player_is_seen_without_the_balloon() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        balloons.frame(start, None, [(4, Reason::Away)], 0, 0, 0, Some(0), true);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        assert_eq!(balloons.shown.shader(5), None);
        // Seen without the balloon before it showed: kept for the grace.
        let soon = start + Duration::from_millis(500);
        balloons.frame(soon, None, [], 1 << 4, 0, 0, Some(0), true);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        // Out of view, kept whatever the time.
        let late = start + Duration::from_secs(60);
        balloons.frame(late, None, [], 0, 0, 0, Some(0), true);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        // Seen under it, kept.
        balloons.frame(late, None, [], 1 << 4, 1 << 4, 0, Some(0), true);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        // Seen without it after the grace: gone.
        balloons.frame(late, None, [], 1 << 4, 0, 0, Some(0), true);
        assert_eq!(balloons.shown.shader(4), None);
    }

    #[test]
    fn the_setting_off_draws_stock_but_still_tells_and_hears() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        balloons.frame(
            start,
            Some(Reason::Console),
            [(2, Reason::Typing)],
            0,
            0,
            1 << 2,
            Some(1),
            false,
        );
        assert_eq!(balloons.shown.shader(1), None);
        assert_eq!(balloons.shown.shader(2), None);
        let later = start + Duration::from_secs(1);
        assert_eq!(
            balloons.frame(
                later,
                Some(Reason::Console),
                [],
                0,
                0,
                1 << 2,
                Some(1),
                true
            ),
            Some((2, Reason::Console))
        );
        assert_eq!(balloons.shown.shader(1), Some(SHADERS[1]));
        assert_eq!(balloons.shown.shader(2), Some(SHADERS[0]));
    }
}
