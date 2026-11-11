//! The unlock pop-up: a small card that slides in at the top centre of the screen when
//! something comes to the player ([`Unlock`]), and plays the single-player game's
//! secret-area sound (`audio/ui_cues.rs`, off with `cg_achievementSound 0`):
//!
//! - an achievement unlocked (`achievements_frame.rs`), with the board's medallion
//!   (`achievements/medallion.rs`), "Achievement unlocked", its name, category and what
//!   it asked;
//! - a new medal or holocron that arrives during a match (`medal_popup.rs`,
//!   `holocron_popup.rs`), with the medallion's picture or the tier's icon (its gem
//!   without one), "New medal" or "New holocron", its name, what it is for or where it
//!   came from, and where to see it: the large pop-up still waits for the game menu, and
//!   takes the cards of what it shows away as it opens ([`UnlockToast::withdraw`]);
//! - a newer SJK release the update check found (`update.rs`), once a session, with
//!   SJK's emblem, "Update available", the version and "Click to install", in the SJK
//!   UI's holo blue and without the sound; the Update page opening takes it away;
//! - a content collection's automatic update that changed files
//!   (`console_jof_content.rs`), with SJK's emblem, "Content updated", "Sith cloud
//!   updated" and what came, in holo blue and without the sound.
//!
//! Unlike the large pop-ups it is not modal: it pauses nothing, over play as over the
//! menus, and takes no keys. The one input it takes is a click on an update's card while
//! the pointer is free ([`crate::GpuState::update_card_pointer`]), which starts the
//! download and opens the Update page. It enters in [`ENTER`] seconds, sliding down and growing to
//! its size while a ring sweeps round the medallion, light bursts from it (a glow, a
//! ring of light, sparks) and a glint crosses the card; it holds [`HOLD`] seconds and
//! fades out in [`LEAVE`]. The light is gold, a holocron's in its tier's colour. Several
//! unlocks queue and show one after another, each with its sound.
//!
//! The top centre is free in play: the HUDs' gauges sit in the bottom corners,
//! timers at the top right, notify lines at the top left, centre prints and the
//! crosshair lower (`version_overlay.rs`), so the card covers neither the aim nor the
//! chat. It waits while the console is open, a large pop-up shows or the window is away
//! (alt-tabbed out or minimised), and its clock runs only while it is drawn, so a card
//! is never spent unseen: a card hidden half way resumes where it was, and a long frame
//! (a hitch, a map loading, a minimised window drawing nothing) moves it on by at most
//! [`MAX_STEP`]. It draws nothing, nor allocates, while no unlock waits.

use crate::achievements::Kind;
use crate::achievements::medallion::{self, Medallion, tint};
use crate::audio::ui_cues::{self, Cue};
use crate::holocrons::{Tier, gem, icons};
use crate::medals::Medal;
use crate::menu::sjk::{DISPLAY_CENTRE, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::text::{TextStyle, TextVertex, UiFont};
use crate::update::Version;
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, InputEvent, Rect, TextAlign, Vec2};
use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::fmt;
use std::time::{Duration, Instant};

/// The cvar that plays the sound with each pop-up (1, the default) or not (0).
pub(crate) const SOUND_CVAR: &str = "cg_achievementSound";

/// The most cards one arrival of medals or holocrons brings (a reinstalled PC may find
/// twenty holocrons at once); the game menu's pop-up shows them all.
pub(crate) const AT_ONCE: usize = 3;
/// Where a medal's or a holocron's card says it is shown.
pub(crate) const MENU_HINT: &str = "Open the game menu to see it";
/// What a click on the update card does ([`UnlockToast::update_card_at`]).
pub(crate) const UPDATE_HINT: &str = "Click to install";
/// The same for a folder that cannot update itself: the click opens the release page.
pub(crate) const UPDATE_MANUAL_HINT: &str = "Click for its page";
/// The console command that shows the update card for a made-up release.
pub(crate) const UPDATE_COMMAND: &str = "debug_update";
/// Its line in the command list and browser.
pub(crate) const UPDATE_HELP: &str =
    "Rehearse the update card, nothing checked or installed: debug_update [version] [manual]";
/// The release `debug_update` names when it is given none.
const REHEARSED_VERSION: &str = "2026.1231.1";

/// Seconds the card takes to come in.
pub(crate) const ENTER: f32 = 0.45;
/// Seconds it then stays.
pub(crate) const HOLD: f32 = 5.0;
/// Seconds it takes to fade out.
pub(crate) const LEAVE: f32 = 0.65;
/// Seconds one pop-up lasts.
pub(crate) const LIFETIME: f32 = ENTER + HOLD + LEAVE;
/// Pause between two pop-ups.
const GAP: Duration = Duration::from_millis(300);
/// The most one frame moves a card's clock, in seconds.
pub(crate) const MAX_STEP: f32 = 0.1;

/// The card, in 1080-line pixels: its size, its top's distance from the screen's
/// top and its corners' radius.
const WIDTH: f32 = 620.0;
const HEIGHT: f32 = 116.0;
const TOP: f32 = 104.0;
const RADIUS: f32 = 18.0;
/// The medallion's middle from the card's left, and its radius.
const MEDAL_X: f32 = 64.0;
const MEDAL_RADIUS: f32 = 38.0;
/// Where the text column starts and the room it has.
const TEXT_X: f32 = 126.0;
pub(crate) const TEXT_WIDTH: f32 = WIDTH - TEXT_X - 24.0;
/// The name's and the description's type sizes.
pub(crate) const NAME_SIZE: f32 = 32.0;
pub(crate) const DESCRIPTION_SIZE: f32 = 16.0;
/// The kicker's type size and letter spacing, and the tag's type size.
const KICKER_SIZE: f32 = 15.0;
const KICKER_SPACING: f32 = 2.6;
const TAG_SIZE: f32 = 15.0;

/// The sparks the burst throws: direction (degrees, 0 to the right, clockwise), how
/// far each flies (pixels at 1080 lines), when it leaves (seconds) and its size.
const SPARKS: [(f32, f32, f32, f32); 14] = [
    (-90.0, 74.0, 0.20, 3.4),
    (-62.0, 58.0, 0.26, 2.6),
    (-35.0, 88.0, 0.22, 3.0),
    (-8.0, 66.0, 0.30, 2.4),
    (18.0, 92.0, 0.21, 3.2),
    (44.0, 60.0, 0.27, 2.6),
    (70.0, 80.0, 0.24, 3.0),
    (96.0, 56.0, 0.31, 2.4),
    (124.0, 86.0, 0.20, 3.4),
    (150.0, 64.0, 0.28, 2.6),
    (178.0, 94.0, 0.23, 3.0),
    (205.0, 58.0, 0.29, 2.4),
    (232.0, 82.0, 0.22, 3.2),
    (258.0, 62.0, 0.27, 2.6),
];

/// Where a pop-up is in its life.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Entering,
    Shown,
    Leaving,
    Gone,
}

impl Phase {
    /// The phase `t` seconds after the pop-up began.
    pub(crate) fn at(t: f32) -> Self {
        if t < ENTER {
            Self::Entering
        } else if t < ENTER + HOLD {
            Self::Shown
        } else if t < LIFETIME {
            Self::Leaving
        } else {
            Self::Gone
        }
    }
}

/// How the card stands `t` seconds after it began: its opacity, how far it is above
/// its place (1080-line pixels) and its size against its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Moment {
    pub(crate) alpha: f32,
    pub(crate) lift: f32,
    pub(crate) scale: f32,
}

impl Moment {
    pub(crate) fn at(t: f32) -> Self {
        let enter = ease_out_cubic(span(t, 0.0, ENTER));
        let fade_in = ease_out_cubic(span(t, 0.0, ENTER * 0.7));
        let leave = span(t, ENTER + HOLD, LIFETIME);
        let leave_eased = leave * leave;
        Self {
            alpha: fade_in * (1.0 - leave_eased),
            lift: 30.0 * (1.0 - enter) + 18.0 * leave_eased,
            scale: (0.86 + 0.14 * ease_out_back(span(t, 0.0, ENTER))) * (1.0 - 0.04 * leave),
        }
    }
}

/// How far `t` is from `from` to `to`, from 0 to 1.
fn span(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

fn ease_in_out_cubic(x: f32) -> f32 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) * 0.5
    }
}

/// Past 1 a little before settling, for a card that pops into place.
fn ease_out_back(x: f32) -> f32 {
    const C1: f32 = 1.4;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (x - 1.0).powi(3) + C1 * (x - 1.0).powi(2)
}

/// What a card announces.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Unlock {
    /// An achievement unlocked; `crystal` when the hub unlocked it, so its kyber crystal
    /// comes (without a hub it unlocks on this client and brings none).
    Achievement { kind: &'static Kind, crystal: bool },
    /// The SJK team gave a medal, `count` times in all for a repeatable one; `promoted`
    /// when it was held already and its rank rose to this one.
    Medal {
        medal: Medal,
        count: u32,
        promoted: Option<crate::medals::Rank>,
    },
    /// A holocron dropped, numbered `id` at the hub; `gift` when staff gave it. A kyber
    /// crystal ([`crate::kyber`]) arrives the same way, `earned` saying from what.
    Holocron {
        tier: &'static Tier,
        id: u64,
        gift: bool,
        earned: Option<crate::kyber::Earned>,
    },
    /// A newer SJK release is out; `manual` when this folder cannot install it itself
    /// (the Update page then offers the release page).
    Update { version: Version, manual: bool },
    /// A contributor reward (`official.rs`) waits for the player, numbered `id` at the
    /// hub: the claim window waits for the game menu.
    Reward { id: u64 },
    /// A content collection (`title`: JoF content, the Sith cloud) updated itself:
    /// `files` downloaded, `bytes` in all, and `removed` the server no longer lists.
    Content {
        title: &'static str,
        files: u32,
        bytes: u64,
        removed: u32,
    },
    /// A saber shader on the player's wishlist is among the shop's offers today
    /// (`wishlist.rs`, 10/11/2026), told once a shop day.
    Wished {
        skin: &'static crate::unlockables::Unlockable,
    },
}

impl Unlock {
    /// The same arrival: the same achievement, medal and count, or holocron.
    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Achievement { kind: a, .. }, Self::Achievement { kind: b, .. }) => a.id == b.id,
            (
                Self::Medal {
                    medal,
                    count,
                    promoted,
                },
                Self::Medal {
                    medal: other,
                    count: other_count,
                    promoted: other_promoted,
                },
            ) => medal == other && count == other_count && promoted == other_promoted,
            (Self::Holocron { id, .. }, Self::Holocron { id: other, .. }) => id == other,
            (Self::Update { version, .. }, Self::Update { version: other, .. }) => version == other,
            (Self::Reward { id }, Self::Reward { id: other }) => id == other,
            (Self::Wished { skin }, Self::Wished { skin: other }) => skin.id == other.id,
            (
                Self::Content {
                    title,
                    files,
                    bytes,
                    removed,
                },
                Self::Content {
                    title: other_title,
                    files: other_files,
                    bytes: other_bytes,
                    removed: other_removed,
                },
            ) => {
                (title, files, bytes, removed)
                    == (other_title, other_files, other_bytes, other_removed)
            }
            _ => false,
        }
    }

    /// The capitals over the name.
    pub(crate) const fn kicker(self) -> &'static str {
        match self {
            Self::Achievement { .. } => "ACHIEVEMENT UNLOCKED",
            Self::Medal {
                promoted: Some(_), ..
            } => "MEDAL PROMOTED",
            Self::Medal { .. } => "NEW MEDAL",
            Self::Holocron { tier, .. } if tier.is_kyber() => "NEW KYBER CRYSTAL",
            Self::Holocron { .. } => "NEW HOLOCRON",
            Self::Update { .. } => "UPDATE AVAILABLE",
            Self::Reward { .. } => "CONTRIBUTOR REWARD",
            Self::Content { .. } => "CONTENT UPDATED",
            Self::Wished { .. } => "ON YOUR WISHLIST",
        }
    }

    /// Whether the card plays the chime: everything but an update or a content
    /// update, which are news, not rewards.
    pub(crate) const fn chimes(self) -> bool {
        !matches!(
            self,
            Self::Update { .. } | Self::Content { .. } | Self::Wished { .. }
        )
    }

    /// The words at the kicker's line's end: the kyber crystal an achievement brings when
    /// the hub unlocked it, else its category; else where the large pop-up or the Update
    /// page shows it.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Achievement {
                kind,
                crystal: true,
            } => crate::kyber::reward(kind.id)
                .map_or(kind.category.name(), crate::kyber::reward_line),
            Self::Achievement { kind, .. } => kind.category.name(),
            Self::Medal { .. } | Self::Holocron { .. } => MENU_HINT,
            Self::Update { manual: false, .. } => UPDATE_HINT,
            Self::Update { manual: true, .. } => UPDATE_MANUAL_HINT,
            Self::Reward { .. } => "From the SJK team",
            Self::Content { .. } => "Automatic update",
            Self::Wished { .. } => "Collection, Forge, Shop",
        }
    }

    /// The colour the card is washed with from the left.
    fn hue(self) -> Color {
        match self {
            Self::Achievement { kind, .. } => tint(kind.category),
            Self::Medal { .. } | Self::Reward { .. } => color::GOLD,
            Self::Holocron { tier, .. } => tier.colour,
            Self::Wished { skin } => skin.tier.colour(),
            Self::Update { .. } | Self::Content { .. } => color::HOLO,
        }
    }

    /// The light: the burst, the rings, the sparks, the name; a holocron's is its tier's
    /// colour, lightened as its pop-up's.
    fn light(self) -> Color {
        match self {
            Self::Holocron { tier, .. } => Color::new(
                tier.colour.r * 0.5 + 0.5,
                tier.colour.g * 0.5 + 0.5,
                tier.colour.b * 0.5 + 0.5,
                1.0,
            ),
            Self::Update { .. } | Self::Content { .. } => color::HOLO,
            _ => color::GOLD_BRIGHT,
        }
    }

    /// The card's edge.
    fn edge(self) -> Color {
        match self {
            Self::Holocron { tier, .. } => tier.colour,
            Self::Update { .. } | Self::Content { .. } => color::HOLO,
            _ => color::GOLD,
        }
    }

    /// The name, with a medal's count when it was given more than once.
    pub(crate) const fn name(self) -> Name {
        Name(self)
    }

    /// The line under the name.
    pub(crate) const fn description(self) -> Description {
        Description(self)
    }
}

/// An unlock's name, written without allocating.
pub(crate) struct Name(Unlock);

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Unlock::Achievement { kind, .. } => formatter.write_str(&crate::terms::say(kind.name)),
            Unlock::Medal { medal, count, .. } if count > 1 => {
                write!(formatter, "{} x{count}", medal.name())
            }
            Unlock::Medal { medal, .. } => formatter.write_str(medal.name()),
            Unlock::Holocron { tier, .. } => formatter.write_str(&tier.title()),
            Unlock::Update { version, .. } => write!(formatter, "SJK {}", version.as_str()),
            Unlock::Reward { .. } => formatter.write_str("Contributor reward waiting"),
            Unlock::Content { title, .. } => write!(formatter, "{title} updated"),
            Unlock::Wished { skin } => formatter.write_str(skin.name),
        }
    }
}

/// The line under an unlock's name, written without allocating: what an achievement
/// asked, what a medal is for, where a holocron came from and how rare it is.
pub(crate) struct Description(Unlock);

impl fmt::Display for Description {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Unlock::Achievement { kind, .. } => {
                formatter.write_str(&crate::terms::say(kind.description))
            }
            Unlock::Medal {
                promoted: Some(rank),
                ..
            } => write!(formatter, "Promoted to {}", rank.title()),
            Unlock::Medal { medal, .. } => formatter.write_str(medal.description()),
            Unlock::Holocron { gift: true, .. } => formatter.write_str("A gift from the SJK team"),
            Unlock::Holocron {
                earned: Some(earned),
                ..
            } => write!(formatter, "{earned}"),
            Unlock::Holocron { tier, .. } if tier.is_kyber() => {
                formatter.write_str(crate::kyber::ABOUT)
            }
            Unlock::Holocron { tier, .. } => write!(
                formatter,
                "Found while playing: {} of drops are {}",
                tier.odds(),
                tier.label
            ),
            Unlock::Update { manual: false, .. } => {
                formatter.write_str("Downloads now; it starts when SJK restarts")
            }
            Unlock::Update { manual: true, .. } => {
                formatter.write_str("This folder cannot update itself: see its page")
            }
            Unlock::Reward { .. } => formatter.write_str("Open the game menu to claim it"),
            Unlock::Wished { .. } => {
                formatter.write_str("A shader on your wishlist is in the shop today")
            }
            Unlock::Content {
                files,
                bytes,
                removed,
                ..
            } => {
                if files > 0 {
                    let plural = if files == 1 { "" } else { "s" };
                    write!(formatter, "{files} file{plural}, ")?;
                    write_size(formatter, bytes)?;
                    if removed > 0 {
                        write!(formatter, ", {removed} removed")?;
                    }
                } else {
                    let plural = if removed == 1 { "" } else { "s" };
                    write!(formatter, "{removed} old file{plural} removed")?;
                }
                formatter.write_str("  ·  from the next map")
            }
        }
    }
}

/// "1.4 GB", "75 MB", "820 KB" (decimal units, as the content card counts),
/// written without allocating.
fn write_size(formatter: &mut fmt::Formatter<'_>, bytes: u64) -> fmt::Result {
    const KB: f64 = 1000.0;
    let value = bytes as f64;
    if value >= KB * KB * KB {
        write!(formatter, "{:.1} GB", value / (KB * KB * KB))
    } else if value >= KB * KB {
        write!(formatter, "{:.0} MB", value / (KB * KB))
    } else if value >= KB {
        write!(formatter, "{:.0} KB", value / KB)
    } else {
        write!(formatter, "{bytes} B")
    }
}

/// The unlock showing and how long it has been seen.
#[derive(Clone, Copy)]
struct Showing {
    unlock: Unlock,
    /// Seconds it has been drawn.
    shown: f32,
    /// The frame that last moved it on.
    last: Instant,
}

/// The pop-up's state: the unlocks waiting, the one showing and its canvas.
pub(crate) struct UnlockToast {
    canvas: MenuCanvas,
    queue: VecDeque<Unlock>,
    current: Option<Showing>,
    /// When the last pop-up ended, for the pause before the next.
    ended: Option<Instant>,
    /// The update state's generation last read (`update::generation`), and the release
    /// whose card was given this session.
    update_seen: u32,
    update_given: Option<Version>,
    /// Where the card showing was laid out this frame, for a click on an update's card;
    /// `None` while none draws.
    card: Option<Rect>,
    /// A moment held still (seconds after the start), for the off-screen shots.
    #[cfg(test)]
    held: Option<f32>,
}

impl Default for UnlockToast {
    fn default() -> Self {
        Self {
            // Four text runs and about seventy shapes a frame, a hundred with a gem.
            canvas: MenuCanvas::with_capacities(8, 96, 160),
            queue: VecDeque::with_capacity(4),
            current: None,
            ended: None,
            update_seen: 0,
            update_given: None,
            card: None,
            #[cfg(test)]
            held: None,
        }
    }
}

impl UnlockToast {
    /// `unlock` came: its pop-up waits for those before it.
    pub(crate) fn push(&mut self, unlock: Unlock) {
        let showing = self
            .current
            .is_some_and(|current| current.unlock.same(unlock));
        if !showing && !self.queue.iter().any(|waiting| waiting.same(unlock)) {
            self.queue.push_back(unlock);
        }
    }

    /// Take back the cards `which` picks, waiting or showing, at `now`: a large pop-up
    /// shows what they announce.
    pub(crate) fn withdraw(&mut self, now: Instant, which: impl Fn(Unlock) -> bool) {
        self.queue.retain(|waiting| !which(*waiting));
        if self.current.is_some_and(|showing| which(showing.unlock)) {
            self.current = None;
            self.ended = Some(now);
        }
    }

    /// The update state moved to `generation`: when `available` (read only then) names a
    /// newer release, give its card, once a session for each release, unless the Update
    /// page shows it (`page_open`), which also takes a card still waiting or showing back.
    pub(crate) fn offer_update(
        &mut self,
        now: Instant,
        generation: u32,
        page_open: bool,
        available: impl FnOnce() -> Option<(String, bool)>,
    ) {
        if page_open && self.pending() > 0 {
            self.withdraw(now, |unlock| matches!(unlock, Unlock::Update { .. }));
        }
        if generation == self.update_seen {
            return;
        }
        self.update_seen = generation;
        let Some((text, manual)) = available() else {
            return;
        };
        let Some(version) = Version::new(&text) else {
            return;
        };
        if self.update_given == Some(version) {
            return;
        }
        self.update_given = Some(version);
        if !page_open {
            self.push(Unlock::Update { version, manual });
        }
    }

    /// Unlocks waiting, the one showing included.
    pub(crate) fn pending(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    /// Seconds the pop-up showing has been seen.
    fn elapsed(&self, showing: Showing) -> f32 {
        #[cfg(test)]
        if let Some(held) = self.held {
            return held;
        }
        showing.shown
    }

    /// [`Self::update_holding`] with no card held, for the tests and off-screen shots.
    #[cfg(test)]
    pub(crate) fn update(&mut self, now: Instant, may_show: bool, sound: bool) -> bool {
        self.update_holding(now, may_show, |_| false, sound)
    }

    /// Move the pop-ups on to `now`: the one showing by the frame's time while it
    /// `may_show` (not under the console or a large pop-up, the window not away), at most
    /// [`MAX_STEP`]; end it once its time is up, and start the next one waiting when it
    /// may show, with its `sound` where it chimes. Returns whether a pop-up draws this
    /// frame. The cards `held` picks also wait (a private duel's, [`held_in_duel`]): one
    /// showing stands still unseen, and the first waiting card not held starts before
    /// those held ahead of it, which keep their place.
    pub(crate) fn update_holding(
        &mut self,
        now: Instant,
        may_show: bool,
        held: impl Fn(Unlock) -> bool,
        sound: bool,
    ) -> bool {
        if let Some(showing) = &mut self.current {
            let step = now
                .saturating_duration_since(showing.last)
                .as_secs_f32()
                .min(MAX_STEP);
            showing.last = now;
            if may_show && !held(showing.unlock) {
                showing.shown += step;
            }
        }
        if let Some(showing) = self.current
            && Phase::at(self.elapsed(showing)) == Phase::Gone
        {
            self.current = None;
            self.ended = Some(now);
        }
        if self.current.is_none()
            && may_show
            && let Some(next) = self.queue.iter().position(|waiting| !held(*waiting))
        {
            let rested = self
                .ended
                .is_none_or(|ended| now.saturating_duration_since(ended) >= GAP);
            if rested && let Some(unlock) = self.queue.remove(next) {
                self.current = Some(Showing {
                    unlock,
                    shown: 0.0,
                    last: now,
                });
                if sound && unlock.chimes() {
                    ui_cues::post(Cue::Achievement);
                }
            }
        }
        let draws = may_show && self.current.is_some_and(|showing| !held(showing.unlock));
        if !draws {
            self.card = None;
        }
        draws
    }

    /// The update card under `position` (window pixels) as last laid out: whether this
    /// folder must take the release from its page. `None` off the card, while it fades
    /// out, or while no update card draws.
    pub(crate) fn update_card_at(&self, position: Vec2) -> Option<bool> {
        let showing = self.current?;
        let Unlock::Update { manual, .. } = showing.unlock else {
            return None;
        };
        let leaving = Phase::at(self.elapsed(showing)) == Phase::Leaving;
        (!leaving && self.card?.contains(position)).then_some(manual)
    }

    /// Take the update's card away at `now`: it was clicked.
    pub(crate) fn dismiss_update(&mut self, now: Instant) {
        self.withdraw(now, |unlock| matches!(unlock, Unlock::Update { .. }));
        self.card = None;
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.canvas.draw_list()
    }

    /// Lay the pop-up showing out for `viewport` as it stands.
    pub(crate) fn build(&mut self, viewport: [f32; 2]) {
        let Some(showing) = self.current else {
            return;
        };
        let t = self.elapsed(showing);
        self.card = Some(card_rect(viewport, t));
        draw(&mut self.canvas, showing.unlock, t, viewport);
    }

    /// Append the pop-up's text: in the SJK UI's families when `fonts` has them, else
    /// in Inter (`font`).
    pub(crate) fn append_text(
        &self,
        fonts: Option<crate::game_font::SjkFonts<'_>>,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        match fonts {
            Some(fonts) => self
                .canvas
                .append_text_families(fonts, viewport, TextStyle::NEUTRAL),
            None => self
                .canvas
                .append_text_styled(vertices, font, viewport, TextStyle::NEUTRAL),
        }
    }
}

/// The card's place and size in the window `t` seconds after it began.
pub(crate) fn card_rect(viewport: [f32; 2], t: f32) -> Rect {
    let moment = Moment::at(t);
    let s = layout_scale(viewport);
    let k = s * moment.scale;
    let middle_y = (TOP + HEIGHT * 0.5 - moment.lift) * s;
    Rect::new(
        viewport[0] * 0.5 - WIDTH * k * 0.5,
        middle_y - HEIGHT * k * 0.5,
        WIDTH * k,
        HEIGHT * k,
    )
}

/// Window pixels per 1080-line pixel: the height's scale, smaller when the window is
/// too narrow for the card.
fn layout_scale(viewport: [f32; 2]) -> f32 {
    crate::ui_scale::height_scale(viewport[1]).min((viewport[0] - 32.0).max(1.0) / WIDTH)
}

/// Draw `unlock`'s pop-up on `canvas` as it stands `t` seconds after it began.
fn draw(canvas: &mut MenuCanvas, unlock: Unlock, t: f32, viewport: [f32; 2]) {
    let moment = Moment::at(t);
    let card = card_rect(viewport, t);
    let k = card.width / WIDTH;
    let at = |x: f32, y: f32| [card.x + x * k, card.y + y * k];
    let hue = unlock.hue();
    let light = unlock.light();
    canvas.begin_transparent(viewport);
    canvas.push_opacity(moment.alpha);
    let list = canvas.draw_list_mut();

    // A soft shadow under the card.
    for (grow, alpha) in [(14.0, 0.10), (6.0, 0.16)] {
        let _ = list.push(DrawCommand::RoundedRect {
            rect: grown(card, grow * k, 6.0 * k),
            radius: (RADIUS + grow) * k,
            color: Color::new(0.0, 0.0, 0.0, alpha),
        });
    }
    // An outline of the light that leaves the card's edge as it lands, and fades.
    let pulse = span(t, 0.12, 1.0);
    if pulse > 0.0 && pulse < 1.0 {
        let grow = (4.0 + 26.0 * ease_out_cubic(pulse)) * k;
        let _ = list.push(DrawCommand::Border {
            rect: grown(card, grow, 0.0),
            radius: RADIUS * k + grow,
            width: (2.0 * (1.0 - pulse) + 0.5) * k,
            color: color::alpha(light, 0.6 * (1.0 - pulse).powi(3)),
        });
    }
    // The card: deep navy washed with the unlock's colour from the left.
    let _ = list.push(DrawCommand::RoundedRect {
        rect: card,
        radius: RADIUS * k,
        color: Color::new(0.05, 0.065, 0.125, 0.94),
    });
    let _ = list.push(DrawCommand::GradientRect {
        rect: card,
        radius: RADIUS * k,
        gradient: Gradient {
            start: color::alpha(hue, 0.13),
            end: color::alpha(hue, 0.0),
            vertical: false,
        },
    });
    // The border, lit as the card lands.
    let flash = 1.0 - span(t, 0.2, 1.2);
    let _ = list.push(DrawCommand::Border {
        rect: card,
        radius: RADIUS * k,
        width: 1.5 * k,
        color: color::alpha(unlock.edge(), 0.55 + 0.4 * flash),
    });
    // A rule of the light along the card's top, brightest at its middle.
    let rule = Rect::new(
        card.x + 40.0 * k,
        card.y,
        card.width * 0.5 - 40.0 * k,
        k.max(1.0),
    );
    for (half, start, end) in [
        (rule, color::alpha(light, 0.0), light),
        (
            Rect::new(rule.right(), rule.y, rule.width, rule.height),
            light,
            color::alpha(light, 0.0),
        ),
    ] {
        let _ = list.push(DrawCommand::GradientRect {
            rect: half,
            radius: 0.0,
            gradient: Gradient {
                start,
                end,
                vertical: false,
            },
        });
    }

    let centre = at(MEDAL_X, HEIGHT * 0.5);
    let medal = MEDAL_RADIUS * k;
    // The light behind the medallion: a burst as the card lands, then a steady glow.
    let burst = burst_envelope(t);
    for (factor, alpha) in [(2.1, 0.05), (1.65, 0.09), (1.3, 0.14)] {
        let radius = medal * (factor * (0.85 + 0.25 * burst));
        let _ = list.push(DrawCommand::RoundedRect {
            rect: Rect::new(
                centre[0] - radius,
                centre[1] - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            radius,
            color: color::alpha(light, alpha * (0.45 + 1.4 * burst)),
        });
    }
    // A ring of light running out from the medallion.
    let wave = span(t, 0.18, 1.0);
    if wave > 0.0 && wave < 1.0 {
        let _ = list.push(DrawCommand::Arc {
            center: centre,
            radius: medal * (1.0 + 1.5 * ease_out_cubic(wave)),
            width: (5.0 * (1.0 - wave) + 1.0) * k,
            start: 0.0,
            sweep: TAU,
            color: color::alpha(light, 0.85 * (1.0 - wave).powf(1.5)),
            knockout: None,
        });
    }
    // Sparks thrown out from it, each a bright head and a fading tail.
    for (degrees, travel, delay, size) in SPARKS {
        let life = span(t, delay, delay + 0.8);
        if life <= 0.0 || life >= 1.0 {
            continue;
        }
        let (sin, cos) = degrees.to_radians().sin_cos();
        let distance = medal * 0.9 + travel * k * ease_out_cubic(life);
        let fade = (1.0 - life).powf(1.4);
        for (back, shrink, dim) in [(0.0, 1.0, 1.0), (7.0, 0.7, 0.55), (13.0, 0.45, 0.25)] {
            let reach = (distance - back * k * (1.0 - life)).max(medal * 0.9);
            let radius = size * shrink * k * (1.0 - 0.5 * life);
            let point = [centre[0] + cos * reach, centre[1] + sin * reach];
            let _ = list.push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    point[0] - radius,
                    point[1] - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                radius,
                color: color::alpha(
                    if back == 0.0 {
                        Color::new(1.0, 0.96, 0.85, 1.0)
                    } else {
                        light
                    },
                    fade * dim,
                ),
            });
        }
    }

    // The medallion, its ring sweeping round as the card comes in.
    let sweep = ease_in_out_cubic(span(t, 0.08, 0.7));
    emblem(canvas, unlock, centre, medal, sweep);
    let list = canvas.draw_list_mut();
    if sweep > 0.0 && sweep < 1.0 {
        // The sweep's bright head.
        let angle = -FRAC_PI_2 + TAU * sweep;
        let point = [
            centre[0] + medal * angle.cos(),
            centre[1] + medal * angle.sin(),
        ];
        for (radius, alpha) in [(10.0, 0.25), (4.5, 1.0)] {
            let radius = radius * k;
            let _ = list.push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    point[0] - radius,
                    point[1] - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                radius,
                color: Color::new(1.0, 0.97, 0.88, alpha),
            });
        }
    }
    // A flash over the medallion as the ring closes.
    let closed = span(t, 0.7, 1.05);
    if closed > 0.0 && closed < 1.0 {
        let radius = medal * (1.0 + 0.25 * closed);
        let _ = list.push(DrawCommand::RoundedRect {
            rect: Rect::new(
                centre[0] - radius,
                centre[1] - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            radius,
            color: color::alpha(light, 0.45 * (1.0 - closed).powi(2)),
        });
    }
    // A glint crossing the card once the ring has closed.
    let glint = span(t, 0.55, 1.2);
    if glint > 0.0 && glint < 1.0 {
        let band = 70.0 * k;
        let x = card.x - band * 2.0 + (card.width + band * 2.0) * ease_in_out_cubic(glint);
        let strength = 0.16 * (PI * glint).sin();
        let _ = list.push(DrawCommand::PushClip(card));
        for (rect, start, end) in [
            (
                Rect::new(x, card.y, band, card.height),
                Color::new(1.0, 1.0, 1.0, 0.0),
                Color::new(1.0, 0.95, 0.85, strength),
            ),
            (
                Rect::new(x + band, card.y, band, card.height),
                Color::new(1.0, 0.95, 0.85, strength),
                Color::new(1.0, 1.0, 1.0, 0.0),
            ),
        ] {
            let _ = list.push(DrawCommand::GradientRect {
                rect,
                radius: 0.0,
                gradient: Gradient {
                    start,
                    end,
                    vertical: false,
                },
            });
        }
        let _ = list.push(DrawCommand::PopClip);
    }

    // The words: the kicker and its tag, the name, the line under it.
    let column = |y: f32, height: f32| {
        let [x, y] = at(TEXT_X, y);
        Rect::new(x, y, TEXT_WIDTH * k, height * k)
    };
    spaced(
        canvas,
        format_args!("{}", crate::terms::say(unlock.kicker())),
        column(12.0, 24.0),
        KICKER_SIZE * k,
        color::GOLD,
        KICKER_SPACING * k,
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", unlock.tag()),
        column(12.0, 24.0),
        TAG_SIZE * k,
        match unlock {
            // The crystal it brings, in the crystal's colour.
            Unlock::Achievement { kind, .. } => {
                crate::kyber::reward(kind.id).map_or(color::alpha(hue, 0.9), |tier| tier.colour)
            }
            _ => color::MUTED,
        },
        FontWeight::Regular,
        TextAlign::End,
    );
    // The name settles in a moment after the card.
    canvas.push_opacity(ease_out_cubic(span(t, 0.12, 0.5)));
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", unlock.name()),
        column(36.0, 40.0),
        NAME_SIZE * k,
        light,
        FontWeight::Semibold,
        TextAlign::Start,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{}", unlock.description()),
        column(78.0, 24.0),
        DESCRIPTION_SIZE * k,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    canvas.pop_opacity();
    canvas.pop_opacity();
    canvas.finish(0);
}

/// Draw what `unlock` brought in the circle of `radius` round `centre`, its ring `sweep`
/// of the way round: an achievement's medallion, a medal's medallion picture, a holocron's
/// icon (its gem where the icon is missing), SJK's emblem for an update, each in a lit
/// disc.
fn emblem(canvas: &mut MenuCanvas, unlock: Unlock, centre: [f32; 2], radius: f32, sweep: f32) {
    let (picture, gem_colour) = match unlock {
        Unlock::Achievement { kind, .. } => {
            medallion::draw(
                canvas,
                Medallion {
                    kind,
                    centre,
                    radius,
                    fraction: sweep,
                    done: true,
                },
            );
            return;
        }
        Unlock::Medal { medal, .. } => (Some((medal.icon(), 0.94)), None),
        Unlock::Holocron { tier, .. } => {
            if icons::is_loaded(tier.slot) {
                (Some((icons::texture(tier.slot), 1.12)), None)
            } else {
                (None, Some(tier.colour))
            }
        }
        Unlock::Update { .. }
        | Unlock::Reward { .. }
        | Unlock::Content { .. }
        | Unlock::Wished { .. } => (Some((crate::ui_renderer::LOGO_TEXTURE, 0.86)), None),
    };
    // The medallion's line widths at this size.
    let k = radius / MEDAL_RADIUS;
    let disc = |radius: f32| {
        Rect::new(
            centre[0] - radius,
            centre[1] - radius,
            radius * 2.0,
            radius * 2.0,
        )
    };
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::RoundedRect {
        rect: disc(radius),
        radius,
        color: color::alpha(unlock.hue(), 0.24),
    });
    if let Some((texture, scale)) = picture {
        let _ = list.push(DrawCommand::TexturedQuad {
            rect: disc(radius * scale),
            texture,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
    }
    if let Some(colour) = gem_colour {
        gem::draw(list, centre, radius * 0.62, colour, 1.0, gem::ROWS);
    }
    // The ring, as the medallion's: a faint whole one and the light's sweeping over it.
    let _ = list.push(DrawCommand::Arc {
        center: centre,
        radius,
        width: 3.0 * k,
        start: 0.0,
        sweep: TAU,
        color: color::alpha(color::HOLO, 0.16),
        knockout: None,
    });
    if sweep > 0.0 {
        let _ = list.push(DrawCommand::Arc {
            center: centre,
            radius,
            width: 4.0 * k,
            start: -FRAC_PI_2,
            sweep: TAU * sweep.min(1.0),
            color: unlock.light(),
            knockout: None,
        });
    }
}

/// `rect` grown by `by` on every side and moved `down`.
fn grown(rect: Rect, by: f32, down: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by + down,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// The burst's strength `t` seconds in: up quickly as the card lands, then down
/// to nothing over a second.
fn burst_envelope(t: f32) -> f32 {
    let rise = span(t, 0.1, 0.32);
    let fall = span(t, 0.32, 1.5);
    rise * (1.0 - ease_out_cubic(fall))
}

/// Capitals in Rajdhani, spaced out, centred on `rect`'s middle line as
/// [`text`] centres its runs.
fn spaced(
    canvas: &mut MenuCanvas,
    value: std::fmt::Arguments<'_>,
    rect: Rect,
    size: f32,
    color: Color,
    spacing: f32,
) {
    let top = rect.y + rect.height * 0.5 - DISPLAY_CENTRE * size;
    canvas.set_family(TextFamily::Display);
    canvas.text_fmt_aligned(
        value,
        Rect::new(
            rect.x,
            top,
            rect.width,
            (size * 1.3).max(rect.bottom() - top),
        ),
        size,
        color,
        FontWeight::Semibold,
        spacing,
        TextAlign::Start,
    );
    canvas.set_family(TextFamily::Body);
}

impl crate::GpuState {
    /// `debug_update [version] [manual]`: show the update card for a made-up release, as
    /// the update check would on finding it (`manual`: a folder that cannot update itself),
    /// without checking or installing anything. The console closes so it shows at once.
    pub(crate) fn debug_update_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let text = args.first().map_or(REHEARSED_VERSION, String::as_str);
        let version = Version::new(text)
            .ok_or_else(|| format!("{text} is not a release version, such as 2026.1011.1"))?;
        let manual = args
            .get(1)
            .is_some_and(|word| word.eq_ignore_ascii_case("manual"));
        self.unlock_toast.push(Unlock::Update { version, manual });
        if let Some(console) = &mut self.console {
            console.set_open(false);
        }
        self.sync_cursor_policy();
        Ok(vec![format!(
            "The update card for SJK {} is queued; nothing is checked or installed",
            version.as_str()
        )])
    }
}

#[cfg(test)]
impl UnlockToast {
    /// Show `unlocks` one after another, the first held `at` seconds after it began,
    /// for the off-screen shots.
    pub(crate) fn preview(unlocks: &[Unlock], at: f32) -> Self {
        let mut toast = Self::default();
        for unlock in unlocks {
            toast.push(*unlock);
        }
        toast.held = Some(at);
        let _ = toast.update(Instant::now(), true, false);
        toast
    }
}

/// Whether a pop-up may show: not under the console or a large pop-up (`covered`) and
/// the window not away.
fn may_show(covered: bool, console_open: bool, away: bool) -> bool {
    !covered && !console_open && !away
}

/// Whether a private duel (`in_duel`) holds `unlock` back: a reward there is a
/// distraction (hub report #81, Milamber), so the card waits and plays once the duel
/// ends. Holocron cards show anyway when `holocrons_in_duel` (`cl_holocronNotifications`
/// 2, [`crate::holocron_popup::NOTIFICATIONS_CVAR`]).
fn held_in_duel(unlock: Unlock, in_duel: bool, holocrons_in_duel: bool) -> bool {
    in_duel && !(holocrons_in_duel && matches!(unlock, Unlock::Holocron { .. }))
}

impl crate::GpuState {
    /// A click on the update card while the pointer is free (over the menus): it starts
    /// the download, as Install does, and opens the Update page with its progress; for a
    /// folder that cannot update itself it opens the release page. Returns whether the
    /// card took the event.
    pub(crate) fn update_card_pointer(&mut self, event: InputEvent) -> bool {
        let InputEvent::PointerPress {
            position,
            button: sjk_ui::PointerButton::Primary,
        } = event
        else {
            return false;
        };
        if self.pointer_captured {
            return false;
        }
        let Some(manual) = self.unlock_toast.update_card_at(position) else {
            return false;
        };
        self.unlock_toast.dismiss_update(Instant::now());
        let _ = crate::update::click_notice();
        if !manual && let Some(console) = &mut self.console {
            console.open_update_panel();
        }
        self.sync_cursor_policy();
        true
    }

    /// Move the unlock pop-up on and lay it out over the frame unless `covered` (the
    /// console over the frame, a large pop-up) or the window is away; returns whether it
    /// draws this frame. A newer release the update check found gets its card here.
    pub(crate) fn append_unlock_toast(&mut self, viewport: [f32; 2], covered: bool) -> bool {
        let now = Instant::now();
        let page_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::update_page_open);
        self.unlock_toast.offer_update(
            now,
            crate::update::generation(),
            page_open,
            crate::update::available,
        );
        if self.unlock_toast.pending() == 0 {
            return false;
        }
        let console = self.console.as_ref();
        let console_open = console.is_some_and(crate::console::ViewerConsole::is_open);
        let away = console.is_some_and(crate::console::ViewerConsole::window_away);
        let sound = console
            .and_then(|console| console.bool_cvar(SOUND_CVAR))
            .unwrap_or(true);
        let in_duel = self
            .live_session
            .as_ref()
            .is_some_and(|session| session.latest_snapshot().player.duel_in_progress());
        let holocrons_in_duel =
            crate::holocron_popup::notification_mode(console) == crate::holocron_popup::ALWAYS;
        // Tactical mode holds every card in a game until it is off (`tactical.rs`).
        let tactical = self.live_session.is_some() && crate::tactical::hides_fun();
        if !self.unlock_toast.update_holding(
            now,
            may_show(covered, console_open, away),
            |unlock| tactical || held_in_duel(unlock, in_duel, holocrons_in_duel),
            sound,
        ) {
            return false;
        }
        self.unlock_toast.build(viewport);
        self.unlock_toast.append_text(
            self.game_fonts.sjk(),
            &mut self.text_vertices,
            &self.ui_font,
            viewport,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hub report #81: a private duel holds the rewards back, like the console does;
    /// `cl_holocronNotifications` 2 lets the holocron cards through.
    #[test]
    fn a_private_duel_holds_the_rewards_back() {
        assert!(may_show(false, false, false));
        assert!(!may_show(true, false, false));
        assert!(!may_show(false, true, false));
        assert!(!may_show(false, false, true));
        for unlock in [kind("first_blood"), medal(Medal::BugHunter, 1)] {
            assert!(!held_in_duel(unlock, false, false));
            assert!(held_in_duel(unlock, true, false));
            assert!(held_in_duel(unlock, true, true));
        }
        assert!(!held_in_duel(holocron(1, 0), false, false));
        assert!(held_in_duel(holocron(1, 0), true, false));
        assert!(!held_in_duel(holocron(1, 0), true, true));
    }

    /// `cl_holocronNotifications` 1: a holocron card that comes during a private duel
    /// waits unseen, its clock still, and plays in full once the duel ends.
    #[test]
    fn a_holocron_card_withheld_during_a_duel_shows_after_it() {
        let mut toast = UnlockToast::default();
        toast.push(holocron(9, 1));
        let duel = |unlock| held_in_duel(unlock, true, false);
        let mut now = Instant::now();
        for _ in 0..200 {
            now += Duration::from_millis(50);
            assert!(!toast.update_holding(now, true, duel, true));
        }
        assert!(toast.current.is_none());
        assert_eq!(toast.pending(), 1, "it waits through the duel");
        // The duel ends: it shows from its start.
        assert!(toast.update_holding(now, true, |unlock| held_in_duel(unlock, false, false), true));
        assert_eq!(shown(&toast), Some(holocron(9, 1).name().to_string()));
        assert_eq!(toast.current.map(|showing| showing.shown), Some(0.0));
    }

    /// `cl_holocronNotifications` 2: a holocron card shows during a duel ahead of the
    /// rewards held before it, which play once the duel ends.
    #[test]
    fn a_holocron_card_may_pass_the_held_rewards_during_a_duel() {
        let mut toast = UnlockToast::default();
        toast.push(kind("first_blood"));
        toast.push(holocron(9, 1));
        let duel = |unlock| held_in_duel(unlock, true, true);
        let mut now = Instant::now();
        assert!(toast.update_holding(now, true, duel, false));
        assert_eq!(shown(&toast), Some(holocron(9, 1).name().to_string()));
        for _ in 0..((LIFETIME + GAP.as_secs_f32()) / 0.05).ceil() as usize + 2 {
            now += Duration::from_millis(50);
            toast.update_holding(now, true, duel, false);
        }
        assert!(toast.current.is_none(), "the holocron card ended");
        assert_eq!(toast.pending(), 1, "the achievement waits");
        assert!(toast.update_holding(now, true, |unlock| held_in_duel(unlock, false, true), false));
        assert_eq!(shown(&toast), Some(kind("first_blood").name().to_string()));
    }
    use crate::achievements::{self, ALL};
    use crate::holocrons::TIERS;

    fn kind(id: &str) -> Unlock {
        Unlock::Achievement {
            kind: achievements::find(id).expect("an achievement"),
            crystal: true,
        }
    }

    fn medal(medal: Medal, count: u32) -> Unlock {
        Unlock::Medal {
            medal,
            count,
            promoted: None,
        }
    }

    fn holocron(id: u64, tier: usize) -> Unlock {
        Unlock::Holocron {
            tier: &TIERS[tier],
            id,
            gift: false,
            earned: None,
        }
    }

    /// Frames 50 ms apart from `from` for `seconds`, each told whether the card
    /// `may_show`; returns the last frame's time and whether it drew.
    fn frames(
        toast: &mut UnlockToast,
        from: Instant,
        seconds: f32,
        may_show: bool,
    ) -> (Instant, bool) {
        let step = Duration::from_millis(50);
        let mut now = from;
        let mut drew = false;
        for _ in 0..(seconds / 0.05).round() as usize {
            now += step;
            drew = toast.update(now, may_show, true);
        }
        (now, drew)
    }

    /// The name of the unlock showing.
    fn shown(toast: &UnlockToast) -> Option<String> {
        toast
            .current
            .map(|showing| showing.unlock.name().to_string())
    }

    /// Every unlock a card can show: each achievement, each medal once and given many
    /// times, each tier dropped and given.
    fn every_unlock() -> Vec<Unlock> {
        let mut unlocks: Vec<Unlock> = ALL
            .iter()
            .flat_map(|kind| [true, false].map(|crystal| Unlock::Achievement { kind, crystal }))
            .collect();
        for each in Medal::ALL {
            unlocks.push(medal(each, 1));
            if each.repeatable() {
                unlocks.push(medal(each, 128));
            }
        }
        for (index, tier) in TIERS.iter().enumerate() {
            unlocks.push(holocron(index as u64, index));
            unlocks.push(Unlock::Holocron {
                tier,
                id: 100 + index as u64,
                gift: true,
                earned: None,
            });
        }
        // Each kyber crystal, earned from a named achievement and from an unnamed one.
        for (index, tier) in crate::kyber::TIERS.iter().enumerate() {
            for earned in [
                crate::kyber::Earned::of("kills_1000"),
                crate::kyber::Earned::of(""),
            ] {
                unlocks.push(Unlock::Holocron {
                    tier,
                    id: 200 + index as u64,
                    gift: false,
                    earned: Some(earned),
                });
            }
        }
        for (files, removed) in [(312, 4), (0, 1)] {
            unlocks.push(Unlock::Content {
                title: "JoF content",
                files,
                bytes: 4_638_000_000,
                removed,
            });
        }
        for manual in [false, true] {
            unlocks.push(Unlock::Update {
                version: Version::new("2026.1231.12").unwrap(),
                manual,
            });
        }
        // The longest name of a saber shader on the wishlist's card.
        let longest = crate::unlockables::blade_skins()
            .max_by_key(|skin| skin.name.len())
            .unwrap();
        unlocks.push(Unlock::Wished { skin: longest });
        unlocks
    }

    #[test]
    fn unlocks_queue_and_show_one_after_another_each_with_its_sound() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        toast.push(kind("first_blood"));
        toast.push(kind("streak_5"));
        toast.push(kind("first_blood"));
        assert_eq!(toast.pending(), 2, "an unlock waiting is not queued twice");
        let start = Instant::now();
        assert!(toast.update(start, true, true));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        toast.push(kind("first_blood"));
        assert_eq!(toast.pending(), 2, "nor the one showing");
        let first = shown(&toast);
        let (now, drew) = frames(&mut toast, start, LIFETIME - 0.1, true);
        assert!(drew);
        assert_eq!(shown(&toast), first);
        assert!(ui_cues::take_posted().is_empty(), "one sound per pop-up");
        // Gone, and the next waits out the pause.
        let (now, drew) = frames(&mut toast, now, 0.15, true);
        assert!(!drew && toast.current.is_none());
        let (now, drew) = frames(&mut toast, now, 0.1, true);
        assert!(!drew);
        let (now, drew) = frames(&mut toast, now, 0.25, true);
        assert!(drew);
        assert_eq!(shown(&toast), Some(kind("streak_5").name().to_string()));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        let (_, drew) = frames(&mut toast, now, LIFETIME + 0.2, true);
        assert!(!drew);
        assert_eq!(toast.pending(), 0);
    }

    /// The card's clock runs only while it is drawn: hidden (the window away, the
    /// console) it waits where it was, and a long frame counts as one short step.
    #[test]
    fn a_hidden_card_waits_where_it_was() {
        let mut toast = UnlockToast::default();
        toast.push(kind("first_blood"));
        let start = Instant::now();
        assert!(toast.update(start, true, false));
        let (now, _) = frames(&mut toast, start, 1.0, true);
        let at = toast.current.map(|showing| showing.shown).unwrap();
        assert!((at - 1.0).abs() < 1e-3, "{at}");
        // Alt-tabbed out for a minute: not drawn, not moved on.
        let (now, drew) = frames(&mut toast, now, 60.0, false);
        assert!(!drew);
        assert_eq!(toast.current.map(|showing| showing.shown), Some(at));
        // Back: it goes on from there.
        let (now, drew) = frames(&mut toast, now, 0.5, true);
        assert!(drew);
        let at = toast.current.map(|showing| showing.shown).unwrap();
        assert!((at - 1.5).abs() < 1e-3, "{at}");
        // A minimised window drew nothing for ten seconds: one step.
        assert!(toast.update(now + Duration::from_secs(10), true, false));
        let later = toast.current.map(|showing| showing.shown).unwrap();
        assert!((later - at - MAX_STEP).abs() < 1e-4, "{later}");
        // Nothing waiting starts while hidden.
        let mut idle = UnlockToast::default();
        idle.push(kind("maps_10"));
        assert!(!idle.update(now, false, true));
        assert!(idle.current.is_none());
    }

    #[test]
    fn a_covered_screen_holds_the_queue_and_the_sound_can_be_off() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        toast.push(kind("maps_10"));
        let now = Instant::now();
        assert!(!toast.update(now, false, true));
        assert!(toast.current.is_none() && toast.pending() == 1);
        assert!(ui_cues::take_posted().is_empty());
        assert!(toast.update(now, true, false));
        assert!(ui_cues::take_posted().is_empty(), "cg_achievementSound 0");
    }

    /// Medals and holocrons queue with the achievements, each with the chime; a medal
    /// given again is a new arrival, the same holocron is not.
    #[test]
    fn medals_and_holocrons_queue_with_the_achievements() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        toast.push(medal(Medal::BugHunter, 1));
        toast.push(holocron(7, 2));
        toast.push(kind("first_blood"));
        toast.push(medal(Medal::BugHunter, 2));
        toast.push(holocron(7, 2));
        toast.push(medal(Medal::BugHunter, 1));
        assert_eq!(toast.pending(), 4);
        let start = Instant::now();
        assert!(toast.update(start, true, true));
        assert_eq!(shown(&toast).as_deref(), Some("Bug Hunter"));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        let (now, drew) = frames(&mut toast, start, LIFETIME + 0.05, true);
        assert!(!drew);
        let (_, drew) = frames(&mut toast, now, 0.3, true);
        assert!(drew);
        assert_eq!(shown(&toast).as_deref(), Some("Legendary Holocron"));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
    }

    /// An update's card comes once a session for each release the check finds, without
    /// the chime, and never over the Update page, which takes it back.
    #[test]
    fn an_update_found_has_one_quiet_card() {
        ui_cues::take_posted();
        let mut toast = UnlockToast::default();
        let now = Instant::now();
        let found = || Some(("2026.1010.2".to_owned(), false));
        // Nothing changed: the state is not even read.
        toast.offer_update(now, 0, false, || panic!("read without a change"));
        toast.offer_update(now, 1, false, || None);
        assert_eq!(toast.pending(), 0, "checking");
        toast.offer_update(now, 2, false, found);
        assert_eq!(toast.pending(), 1);
        assert!(toast.update(now, true, true));
        assert_eq!(shown(&toast).as_deref(), Some("SJK 2026.1010.2"));
        assert!(ui_cues::take_posted().is_empty(), "no chime for an update");
        // Checked again: the same release gives no second card; a newer one does.
        toast.offer_update(now, 3, false, found);
        assert_eq!(toast.pending(), 1);
        toast.offer_update(now, 4, false, || Some(("2026.1010.3".to_owned(), true)));
        assert_eq!(toast.pending(), 2);
        // The Update page opens: both go, and what it finds then gets no card.
        toast.offer_update(now, 4, true, || None);
        assert_eq!(toast.pending(), 0);
        toast.offer_update(now, 5, true, || Some(("2026.1011.1".to_owned(), false)));
        assert_eq!(toast.pending(), 0);
        toast.offer_update(now, 6, false, || Some(("2026.1011.1".to_owned(), false)));
        assert_eq!(toast.pending(), 0, "the page showed it");
    }

    /// The large pop-up opening takes back its own kind's cards, the one showing too,
    /// and leaves the others.
    #[test]
    fn a_large_pop_up_takes_its_cards_back() {
        let mut toast = UnlockToast::default();
        toast.push(holocron(3, 0));
        toast.push(medal(Medal::Tester, 1));
        toast.push(kind("first_blood"));
        toast.push(holocron(4, 1));
        let now = Instant::now();
        assert!(toast.update(now, true, false));
        toast.withdraw(now, |unlock| matches!(unlock, Unlock::Holocron { .. }));
        assert!(toast.current.is_none(), "the holocron showing went");
        assert_eq!(toast.pending(), 2);
        toast.withdraw(now, |unlock| matches!(unlock, Unlock::Medal { .. }));
        assert_eq!(toast.pending(), 1);
        assert!(toast.update(now + GAP, true, false));
        assert_eq!(shown(&toast), Some(kind("first_blood").name().to_string()));
    }

    /// A click lands on the update's card while it shows, not on another card, before
    /// it is laid out, nor once it fades.
    #[test]
    fn the_update_card_answers_a_click_on_itself() {
        let viewport = [1920.0, 1080.0];
        let version = Version::new("2026.1020.1").unwrap();
        let mut toast = UnlockToast::default();
        toast.push(Unlock::Update {
            version,
            manual: false,
        });
        let now = Instant::now();
        assert!(toast.update(now, true, false));
        let middle = Vec2::new(960.0, (TOP + HEIGHT * 0.5) * viewport[1] / 1080.0);
        assert_eq!(toast.update_card_at(middle), None, "not laid out yet");
        toast.build(viewport);
        assert_eq!(toast.update_card_at(middle), Some(false));
        assert_eq!(toast.update_card_at(Vec2::new(4.0, 4.0)), None);
        toast.held = Some(ENTER + HOLD + 0.1);
        assert_eq!(toast.update_card_at(middle), None, "fading");
        toast.held = None;
        toast.dismiss_update(now);
        assert_eq!(toast.pending(), 0);
        assert_eq!(toast.update_card_at(middle), None);

        let mut medal_card = UnlockToast::preview(&[medal(Medal::BugHunter, 1)], 1.0);
        medal_card.build(viewport);
        assert_eq!(medal_card.update_card_at(middle), None);
    }

    #[test]
    fn the_words_of_medals_and_holocrons() {
        let update = Unlock::Update {
            version: Version::new("2026.1010.2").unwrap(),
            manual: false,
        };
        assert_eq!(update.kicker(), "UPDATE AVAILABLE");
        assert_eq!(update.tag(), UPDATE_HINT);
        assert_eq!(update.name().to_string(), "SJK 2026.1010.2");
        assert!(!update.chimes() && kind("streak_5").chimes());
        let content = Unlock::Content {
            title: "Sith cloud",
            files: 3,
            bytes: 120_400_000,
            removed: 0,
        };
        assert_eq!(content.kicker(), "CONTENT UPDATED");
        assert_eq!(content.name().to_string(), "Sith cloud updated");
        assert_eq!(
            content.description().to_string(),
            "3 files, 120 MB  ·  from the next map"
        );
        let removed = Unlock::Content {
            title: "Sith cloud",
            files: 0,
            bytes: 0,
            removed: 1,
        };
        assert_eq!(
            removed.description().to_string(),
            "1 old file removed  ·  from the next map"
        );
        assert!(!content.chimes() && !content.same(removed) && content.same(content));
        let bug_hunter = medal(Medal::BugHunter, 3);
        assert_eq!(bug_hunter.kicker(), "NEW MEDAL");
        let promoted = Unlock::Medal {
            medal: Medal::Tester,
            count: 1,
            promoted: Some(crate::medals::Rank::Holo),
        };
        assert_eq!(promoted.kicker(), "MEDAL PROMOTED");
        assert_eq!(promoted.name().to_string(), "Tester");
        assert_eq!(
            promoted.description().to_string(),
            "Promoted to Rank IV Holo"
        );
        assert!(!promoted.same(bug_hunter));
        assert_eq!(bug_hunter.name().to_string(), "Bug Hunter x3");
        assert_eq!(bug_hunter.tag(), MENU_HINT);
        assert_eq!(medal(Medal::Tester, 1).name().to_string(), "Tester");
        let mythical = holocron(9, 3);
        assert_eq!(mythical.kicker(), "NEW HOLOCRON");
        assert_eq!(
            mythical.description().to_string(),
            "Found while playing: 1.5% of drops are Mythical"
        );
        let gift = Unlock::Holocron {
            tier: &TIERS[0],
            id: 1,
            gift: true,
            earned: None,
        };
        assert_eq!(gift.description().to_string(), "A gift from the SJK team");
        // An achievement's tag is the kyber crystal it brings (21/10/2026).
        assert_eq!(kind("streak_5").tag(), "Reward: Uncommon Kyber Crystal");
        assert_eq!(kind("kills_1000").tag(), "Reward: Mythical Kyber Crystal");
        // Unlocked without a hub (identity off), it brings no crystal: its category.
        let local = Unlock::Achievement {
            kind: achievements::find("streak_5").expect("Rampage"),
            crystal: false,
        };
        assert_eq!(local.tag(), "Combat");
        let crystal = Unlock::Holocron {
            tier: &crate::kyber::TIERS[1],
            id: 2,
            gift: false,
            earned: Some(crate::kyber::Earned::of("")),
        };
        assert_eq!(crystal.kicker(), "NEW KYBER CRYSTAL");
        assert_eq!(crystal.name().to_string(), "Rare Kyber Crystal");
        assert_eq!(
            crystal.description().to_string(),
            "Earned from an achievement"
        );
        let named = Unlock::Holocron {
            tier: &crate::kyber::TIERS[1],
            id: 3,
            gift: false,
            earned: Some(crate::kyber::Earned::of("kills_100")),
        };
        assert_eq!(named.description().to_string(), "Earned: Centurion");
    }

    #[test]
    fn nothing_is_drawn_while_idle() {
        let mut toast = UnlockToast::default();
        assert!(!toast.update(Instant::now(), true, true));
        toast.build([1920.0, 1080.0]);
        assert!(toast.draw_list().is_empty());
    }

    #[test]
    fn the_card_comes_in_holds_and_leaves() {
        assert_eq!(Phase::at(0.0), Phase::Entering);
        assert_eq!(Phase::at(ENTER + 1.0), Phase::Shown);
        assert_eq!(Phase::at(ENTER + HOLD + 0.1), Phase::Leaving);
        assert_eq!(Phase::at(LIFETIME), Phase::Gone);
        let start = Moment::at(0.0);
        assert!(start.alpha == 0.0 && start.lift > 0.0 && start.scale < 1.0);
        // Past its size for a moment as it lands, then still.
        assert!(Moment::at(ENTER * 0.7).scale > 1.0);
        for t in [ENTER, ENTER + 2.0, ENTER + HOLD] {
            let held = Moment::at(t);
            assert!((held.alpha - 1.0).abs() < 1e-4, "{t}");
            assert!(held.lift.abs() < 1e-4 && (held.scale - 1.0).abs() < 1e-4);
        }
        let leaving = Moment::at(ENTER + HOLD + LEAVE * 0.5);
        assert!(leaving.alpha < 1.0 && leaving.alpha > 0.0);
        assert!(Moment::at(LIFETIME).alpha.abs() < 1e-4);
    }

    /// Every unlock, at every moment, fits the canvas and the screen at 1080 lines, 4K,
    /// 4:3 and 21:9, its words its column in the families and in Inter.
    #[test]
    fn every_unlock_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let display = load(&crate::text::DISPLAY);
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let width = |font: &UiFont, value: &str, size: f32, face, spacing| {
            crate::text::visible_text_width_style(font, value, size / font.height, face, spacing)
        };
        let semibold = crate::text::TextFace::Semibold;
        let regular = crate::text::TextFace::Regular;
        let unlocks = every_unlock();
        for (title, words) in [(&display.font, &body.font), (&inter.font, &inter.font)] {
            for unlock in &unlocks {
                let kicker = width(
                    title,
                    unlock.kicker(),
                    KICKER_SIZE,
                    semibold,
                    KICKER_SPACING,
                );
                let tag = width(title, unlock.tag(), TAG_SIZE, regular, 0.0);
                assert!(kicker + 16.0 + tag <= TEXT_WIDTH, "{unlock:?}");
                let name = unlock.name().to_string();
                let name_width = width(title, &name, NAME_SIZE, semibold, 0.0);
                assert!(name_width <= TEXT_WIDTH, "{name}: {name_width}");
                let description = unlock.description().to_string();
                assert!(description.len() <= 96, "{description}: one text slot");
                let description_width = width(words, &description, DESCRIPTION_SIZE, regular, 0.0);
                assert!(
                    description_width <= TEXT_WIDTH,
                    "{description}: {description_width}"
                );
            }
        }
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
            [800.0, 600.0],
        ] {
            for unlock in &unlocks {
                for t in [0.05, 0.3, 0.6, 1.0, 3.0, ENTER + HOLD + 0.3] {
                    let mut toast = UnlockToast::preview(&[*unlock], t);
                    toast.build(viewport);
                    assert!(!toast.canvas.overflowed(), "{unlock:?} {t} {viewport:?}");
                    let card = card_rect(viewport, t);
                    assert!(
                        card.x >= 0.0 && card.right() <= viewport[0] && card.y >= 0.0,
                        "{viewport:?} {t}"
                    );
                    // Clear of the crosshair, in the screen's top third.
                    assert!(card.bottom() < viewport[1] / 3.0, "{viewport:?}");
                }
            }
        }
    }
}
