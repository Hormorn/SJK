//! Status balloons (`cg_statusBalloons`, Sol's idea, 08/11/2026): the talk balloon uses
//! a clan theme and, when the SJK reason is known, shows why it is up: the three dots lit
//! in turn while they type in the chat, a pause sign while a menu or the console is open,
//! a moon with rising z's while their window is alt-tabbed or minimised.
//!
//! The four [`Mode`]s choose stock's picture, Sol's original SJK reason animation, or a
//! clan's static/animated theme. `EF_TALK`, which the player's `BUTTON_TALK` sets
//! ([`sjk_game_jka::pmove_talk`]), still decides whether a balloon shows. Reasons travel
//! as one toy relay event (`docs/hub-chat.md`, "Toy relay") to each SJK player on the
//! server when they change ([`TOY`], [`Reason::word`]); a reason is kept until its player
//! is seen without the balloon, so it needs no event when it ends.

use std::time::{Duration, Instant};

/// Artwork family for a clan, using the tags already recognised by `jof_tag`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ClanTheme {
    #[default]
    Default,
    S,
    Jof,
}

/// Clan tags with dedicated status balloon artwork. Every other recognised clan uses
/// the standard theme.
const CLAN_THEMES: &[(&str, ClanTheme)] = &[
    // Sol's request, 12/11/2026: dedicated artwork for the {{S}} clan.
    ("{{S}}", ClanTheme::S),
    // Sol's request, 12/11/2026: dedicated artwork for Jedi of Freedom.
    ("JoF", ClanTheme::Jof),
];

impl ClanTheme {
    fn for_clan(clan: &str) -> Self {
        CLAN_THEMES
            .iter()
            .find(|(tag, _)| *tag == clan)
            .map_or(Self::Default, |(_, theme)| *theme)
    }

    fn for_name(name: &str) -> Self {
        crate::jof_tag::clan(name)
            .map(|clan| Self::for_clan(clan.name))
            .unwrap_or_default()
    }

    const fn default_shader(self) -> &'static str {
        match self {
            Self::Default => "gfx/sjk/status/default/default",
            Self::S => "gfx/sjk/status/s/default",
            Self::Jof => "gfx/sjk/status/jof/default",
        }
    }

    const fn shader(self, reason: Reason) -> &'static str {
        match (self, reason) {
            (Self::Default, Reason::Typing) => "gfx/sjk/status/default/typing",
            (Self::Default, Reason::Menu | Reason::Console) => "gfx/sjk/status/default/busy",
            (Self::Default, Reason::Away) => "gfx/sjk/status/default/away",
            (Self::S, Reason::Typing) => "gfx/sjk/status/s/typing",
            (Self::S, Reason::Menu | Reason::Console) => "gfx/sjk/status/s/busy",
            (Self::S, Reason::Away) => "gfx/sjk/status/s/away",
            (Self::Jof, Reason::Typing) => "gfx/sjk/status/jof/typing",
            (Self::Jof, Reason::Menu | Reason::Console) => "gfx/sjk/status/jof/busy",
            (Self::Jof, Reason::Away) => "gfx/sjk/status/jof/away",
        }
    }
}

/// How the talk sprite is drawn. These numeric values are saved in `cg_statusBalloons`
/// and named in the Settings choice list below.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i64)]
pub(crate) enum Mode {
    /// Stock game's talk balloon; no SJK image or animation.
    Default = 0,
    /// Sol's original SJK status animations, using stock icons for unknown reasons.
    #[default]
    Sjk = 1,
    /// Clan-specific static `default.png` pictures.
    SjkClanDefault = 2,
    /// Clan-specific reason animations, with static fallback pictures.
    SjkClanAnimated = 3,
}

/// Fresh profiles keep Sol's requested animated SJK behavior as the selection.
pub(crate) const DEFAULT_MODE: i64 = Mode::Sjk as i64;

impl Mode {
    pub(crate) const CHOICES: &[&str] = &["0", "1", "2", "3"];

    pub(crate) fn from_value(value: i64) -> Self {
        match value {
            0 => Self::Default,
            1 => Self::Sjk,
            2 => Self::SjkClanDefault,
            3 => Self::SjkClanAnimated,
            // An unknown cvar value should always fall back to stock presentation.
            _ => Self::Default,
        }
    }

    pub(crate) const fn uses_clan_themes(self) -> bool {
        matches!(self, Self::SjkClanDefault | Self::SjkClanAnimated)
    }

    pub(crate) const fn uses_animations(self) -> bool {
        matches!(self, Self::Sjk | Self::SjkClanAnimated)
    }
}

/// Clan themes for the current server. Player configstrings are only reparsed when the
/// server changes a name or clan emblems are toggled, so this adds no per-frame allocations.
#[derive(Debug)]
struct ClanThemes {
    player_info: [Option<Vec<u8>>; SLOTS],
    themes: [ClanTheme; SLOTS],
    emblems_shown: Option<bool>,
}

impl Default for ClanThemes {
    fn default() -> Self {
        Self {
            player_info: std::array::from_fn(|_| None),
            themes: [ClanTheme::Default; SLOTS],
            emblems_shown: None,
        }
    }
}

impl ClanThemes {
    fn follow(&mut self, game_state: &sjk_protocol::GameState, emblems_shown: bool) {
        let visibility_changed = self.emblems_shown != Some(emblems_shown);
        self.emblems_shown = Some(emblems_shown);
        for slot in 0..SLOTS {
            let info = game_state.config_string(crate::player_identity::CS_PLAYERS + slot);
            if !visibility_changed && self.player_info[slot].as_deref() == info {
                continue;
            }
            self.player_info[slot] = info.map(<[u8]>::to_vec);
            self.themes[slot] = crate::player_identity::shown_name(game_state, slot)
                .as_deref()
                .map(ClanTheme::for_name)
                .unwrap_or_default();
        }
    }
}

/// The relay's toy id. The hub knows no toys: any id of `a` to `z` and `_` goes.
pub(crate) const TOY: &str = "status_balloon";
/// The setting for stock, original SJK, static clan, or animated clan balloons.
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
}

/// The themed status balloon shaders and their static fallbacks. The reason names and
/// animation timing stay the same across all three artwork families.
pub(crate) const SHADERS: [&str; 12] = [
    "gfx/sjk/status/default/typing",
    "gfx/sjk/status/default/busy",
    "gfx/sjk/status/default/away",
    "gfx/sjk/status/s/typing",
    "gfx/sjk/status/s/busy",
    "gfx/sjk/status/s/away",
    "gfx/sjk/status/jof/typing",
    "gfx/sjk/status/jof/busy",
    "gfx/sjk/status/jof/away",
    "gfx/sjk/status/default/default",
    "gfx/sjk/status/s/default",
    "gfx/sjk/status/jof/default",
];

macro_rules! theme_files {
    ($folder:literal, $theme:literal) => {
        [
            (
                concat!("gfx/sjk/status/", $theme, "/typing_1.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/typing_1.png"
                )),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/typing_2.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/typing_2.png"
                )),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/typing_3.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/typing_3.png"
                )),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/busy.png"),
                include_bytes!(concat!("../assets/status_balloons/", $folder, "/busy.png")),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/away_1.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/away_1.png"
                )),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/away_2.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/away_2.png"
                )),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/away_3.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/away_3.png"
                )),
            ),
            (
                concat!("gfx/sjk/status/", $theme, "/default.png"),
                include_bytes!(concat!(
                    "../assets/status_balloons/",
                    $folder,
                    "/default.png"
                )),
            ),
        ]
    };
}

const SHADER_FILE: (&str, &[u8]) = (
    "shaders/sjk_status_balloons.shader",
    include_bytes!("../assets/status_balloons/status_balloons.shader"),
);
const DEFAULT_FILES: [(&str, &[u8]); 8] = theme_files!("Default", "default");
const S_FILES: [(&str, &[u8]); 8] = theme_files!("S", "s");
const JOF_FILES: [(&str, &[u8]); 8] = theme_files!("Jof", "jof");

fn files() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    std::iter::once(SHADER_FILE)
        .chain(DEFAULT_FILES.iter().copied())
        .chain(S_FILES.iter().copied())
        .chain(JOF_FILES.iter().copied())
}

/// Mount the bundled balloon files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK status balloons", files())?;
    Ok(())
}

/// What this client tells: its reason, since when, and the slots told it.
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

/// The themed picture shown for each talking player this frame (`player_sprites.rs`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shown {
    mode: Mode,
    reasons: [Option<Reason>; SLOTS],
    themes: [ClanTheme; SLOTS],
    /// World shots: the balloon drawn over these players whatever their `EF_TALK`.
    #[cfg(test)]
    pub(crate) force: bool,
}

impl Default for Shown {
    fn default() -> Self {
        Self {
            mode: Mode::Default,
            reasons: [None; SLOTS],
            themes: [ClanTheme::Default; SLOTS],
            #[cfg(test)]
            force: false,
        }
    }
}

impl Shown {
    /// World shots use the original SJK animations over every player.
    #[cfg(test)]
    pub(crate) fn for_shot(reason: Reason) -> Self {
        Self {
            mode: Mode::Sjk,
            reasons: [Some(reason); SLOTS],
            themes: [ClanTheme::Default; SLOTS],
            force: true,
        }
    }

    /// The replacement shader for a talking player, if this mode selects one. `None`
    /// leaves the stock sprite; clan animated mode falls back to its static clan image
    /// until a reason arrives.
    pub(crate) fn shader(&self, slot: u16) -> Option<&'static str> {
        let index = usize::from(slot);
        let theme = self.themes.get(index).copied().unwrap_or_default();
        let reason = self.reasons.get(index).copied().flatten();
        match self.mode {
            Mode::Default => None,
            Mode::Sjk => reason.map(|reason| ClanTheme::Default.shader(reason)),
            Mode::SjkClanDefault => Some(theme.default_shader()),
            Mode::SjkClanAnimated => {
                Some(reason.map_or_else(|| theme.default_shader(), |reason| theme.shader(reason)))
            }
        }
    }
}

/// The status balloons' state, kept on the client.
#[derive(Debug, Default)]
pub(crate) struct StatusBalloons {
    /// The game server the state is for.
    pub(crate) server: Option<std::net::SocketAddr>,
    outbox: Outbox,
    heard: Heard,
    clans: ClanThemes,
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

    /// Cache themes by the game's published names, using `jof_tag::clan` and its setting.
    pub(crate) fn follow_clans(
        &mut self,
        game_state: &sjk_protocol::GameState,
        clan_emblems_shown: bool,
    ) {
        self.clans.follow(game_state, clan_emblems_shown);
    }

    /// One frame at `now`: the selected `mode`, the own `reason`, the reasons `heard`
    /// (slot and reason, already checked), the players `seen` in the snapshot and those
    /// of them `talking` (bits by slot). Returns the event to relay, if one is due, for
    /// the SJK players `others` (bits), those `seen` first. `own` is the own slot.
    /// Reasons are still relayed in stock mode so other SJK clients can use them.
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
        mode: Mode,
    ) -> Option<(u8, Reason)> {
        for (slot, reason) in heard {
            self.heard.hear(slot, reason, now);
        }
        self.heard.follow(seen, talking, now);
        self.outbox.follow(reason, now);
        self.shown = Shown {
            mode,
            themes: if mode.uses_clan_themes() {
                self.clans.themes
            } else {
                [ClanTheme::Default; SLOTS]
            },
            ..Shown::default()
        };
        if mode.uses_animations() {
            for (shown, heard) in self.shown.reasons.iter_mut().zip(self.heard.0.iter()) {
                *shown = heard.map(|(reason, _)| reason);
            }
            if let Some(slot) = own.filter(|slot| usize::from(*slot) < SLOTS) {
                self.shown.reasons[usize::from(slot)] = reason;
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
        let files: Vec<_> = files().collect();
        let source = std::str::from_utf8(files[0].1).unwrap();
        for shader in SHADERS {
            assert!(source.contains(shader), "{shader}");
        }
        for (path, bytes) in &files[1..] {
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
        assert_eq!(
            ClanTheme::Default.shader(Reason::Menu),
            ClanTheme::Default.shader(Reason::Console)
        );
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
                Mode::Sjk,
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
            balloons.frame(at, reason, [], 0, 0, others, Some(0), Mode::Sjk)
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
        own.frame(
            start,
            Some(Reason::Typing),
            [],
            0,
            0,
            1 << 6,
            Some(6),
            Mode::Sjk,
        );
        let later = start + Duration::from_secs(1);
        assert_eq!(
            own.frame(
                later,
                Some(Reason::Typing),
                [],
                0,
                0,
                1 << 6,
                Some(6),
                Mode::Sjk
            ),
            None
        );
    }

    #[test]
    fn a_heard_reason_shows_until_its_player_is_seen_without_the_balloon() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        balloons.frame(
            start,
            None,
            [(4, Reason::Away)],
            0,
            0,
            0,
            Some(0),
            Mode::Sjk,
        );
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        assert_eq!(balloons.shown.shader(5), None);
        // Seen without the balloon before it showed: kept for the grace.
        let soon = start + Duration::from_millis(500);
        balloons.frame(soon, None, [], 1 << 4, 0, 0, Some(0), Mode::Sjk);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        // Out of view, kept whatever the time.
        let late = start + Duration::from_secs(60);
        balloons.frame(late, None, [], 0, 0, 0, Some(0), Mode::Sjk);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        // Seen under it, kept.
        balloons.frame(late, None, [], 1 << 4, 1 << 4, 0, Some(0), Mode::Sjk);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[2]));
        // Seen without it after the grace: gone.
        balloons.frame(late, None, [], 1 << 4, 0, 0, Some(0), Mode::Sjk);
        assert_eq!(balloons.shown.shader(4), None);
    }

    #[test]
    fn stock_mode_keeps_the_stock_balloon_but_still_tells_and_hears() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        balloons.clans.themes[2] = ClanTheme::Jof;
        balloons.frame(
            start,
            Some(Reason::Console),
            [(2, Reason::Typing)],
            0,
            0,
            1 << 2,
            Some(1),
            Mode::Default,
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
                Mode::Sjk
            ),
            Some((2, Reason::Console))
        );
        assert_eq!(balloons.shown.shader(1), Some(SHADERS[1]));
        assert_eq!(balloons.shown.shader(2), Some(SHADERS[0]));
    }

    #[test]
    fn the_clan_theme_table_maps_only_the_requested_clans() {
        assert_eq!(ClanTheme::for_clan("{{S}}"), ClanTheme::S);
        assert_eq!(ClanTheme::for_clan("JoF"), ClanTheme::Jof);
        assert_eq!(ClanTheme::for_clan("JoFA"), ClanTheme::Default);
        assert_eq!(ClanTheme::for_clan("JAWA"), ClanTheme::Default);
    }

    #[test]
    fn a_talking_player_without_a_reason_gets_the_clan_static_fallback() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        balloons.clans.themes[4] = ClanTheme::S;
        balloons.frame(start, None, [], 0, 0, 0, Some(0), Mode::SjkClanDefault);
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[10]));
        assert_eq!(balloons.shown.shader(5), Some(SHADERS[9]));
    }

    #[test]
    fn clan_animated_mode_uses_the_clan_reason_and_static_fallback() {
        let start = Instant::now();
        let mut balloons = StatusBalloons::default();
        balloons.clans.themes[4] = ClanTheme::Jof;
        balloons.frame(
            start,
            None,
            [(4, Reason::Typing)],
            0,
            0,
            0,
            Some(0),
            Mode::SjkClanAnimated,
        );
        assert_eq!(balloons.shown.shader(4), Some(SHADERS[6]));
        assert_eq!(balloons.shown.shader(5), Some(SHADERS[9]));
    }
}
