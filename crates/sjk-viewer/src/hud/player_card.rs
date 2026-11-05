//! The player card: look at a player for a moment without moving, or press the
//! `inspect` key while aiming at them, and a small card appears beside their hips with
//! what the game publishes about them (name, model and its icon, saber, hat and cape,
//! bot skill), the saber shader they wear (a live swatch and its name, only when it
//! draws on their blade here) and, when the SJK hub knows them, their profile picture
//! (the model's icon then sits on its corner), SJK's emblem, their hub name, whether
//! they are verified, an SJK TEAM mark for staff, a gem of the rarest holocron tier
//! they hold and the medals the SJK team gave them (their medallions). `inspect` pins
//! the card: it shows at once, follows the player wherever they go and stays until
//! `inspect` is pressed again or Escape is. A pinned card adds what takes reading:
//! the medals' names in small print, the achievements unlocked out of the catalogue
//! with the three rarest as medallions, the first line of their bio and since when
//! they are in SJK. The card needs the player in sight and near: within
//! [`SIGHT_RANGE`] of the camera with no wall between it and their head or hips; a
//! pinned card hides while they are out of sight and unpins after [`LOST_MS`]. While the card is up, a ring in the card's accent colour runs
//! around the outside of the player's body (`cg_playerCardOutline`,
//! [`crate::card_outline`]).
//!
//! Everything shown is already public to every client (the player's `CS_PLAYERS`
//! string, the hub's public profile and the looks every SJK player sees), so the card
//! gives no advantage a scoreboard glance would not. The profile is asked of the hub
//! when the card is built for a player, at most every ten minutes a player
//! (`player_identity::hub_info`), never per frame; a hub that does not send a field
//! leaves its line out. The
//! target comes from the crosshair scan (`crosshair_scan.rs`), the hip position
//! from the same presented world and camera as the overhead names
//! ([`super::identification`]).

use super::identification::Camera;
use crate::{TextVertex, UiFont};
use glam::Vec3;
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use sjk_client::{LegacyClientInfo, decode_legacy};
use sjk_protocol::GameState;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::TextureId;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};

/// `CS_PLAYERS`: the first player's configstring.
const CS_PLAYERS: usize = 1131;
/// A target may leave the crosshair this long without the dwell starting over, so
/// a jittering aim does not flicker the card.
const GAP_MS: i32 = 300;
/// The view may turn this far from where the dwell began.
const STEADY_DEGREES: f32 = 6.0;
const FADE_IN_MS: f32 = 180.0;
const FADE_OUT_MS: f32 = 120.0;
/// Height of the hips above the origin, standing: the humanoid skeleton's pelvis is
/// 35.6 units above the soles in its base pose, the origin 24, and a stance bends the
/// knees a little.
const HIP_HEIGHT: f32 = 10.0;
/// Share of a crouch's drop of the box top (below standing) the hips drop too.
const HIP_DROP: f32 = 0.5;
/// World units from the hips to the side of the body the card keeps clear of.
const BODY_SIDE: f32 = 22.0;
/// How far a body may reach from its origin, sideways (arms, a flip) and below and
/// above its box, for the screen area its outline may cover.
const REACH_SIDE: f32 = 40.0;
const REACH_BELOW: f32 = -30.0;
const REACH_ABOVE: f32 = 24.0;
/// How far off screen (in half screens) a pinned player may be and still be followed.
const PINNED_LIMIT: f32 = 40.0;
/// World units from the camera beyond which no card shows, dwelled or pinned.
const SIGHT_RANGE: f32 = 1_200.0;
/// How long a pinned player may stay out of sight (behind a wall or beyond
/// [`SIGHT_RANGE`]) before the pin drops; the card is hidden meanwhile.
const LOST_MS: i32 = 3_000;
/// Colour index of a saber's blade in `c1`/`c2`: only the six retail colours have
/// a swatch.
const RETAIL_COLOURS: i32 = 6;

/// The setting saying what opens the card ([`Opens`]).
pub(crate) const CVAR: &str = "cg_playerCard";

/// What opens the card, `cg_playerCard`: 0 nothing, 1 a steady look or `inspect` (the
/// default, and what the old on/off setting's 1 was), 2 only `inspect` (Key only, Blep's
/// idea, 29/10/2026: looking at a player never opens it).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Opens {
    Off,
    LookAndKey,
    KeyOnly,
}

impl Opens {
    /// The setting's value: 0 and below off, 1 (or unset) look and key, 2 and above key
    /// only.
    pub(crate) fn of(value: Option<i64>) -> Self {
        match value {
            Some(..=0) => Self::Off,
            Some(2..) => Self::KeyOnly,
            _ => Self::LookAndKey,
        }
    }

    /// Looking at a player, steady, for `cg_playerCardDelay` shows their card.
    pub(crate) fn by_look(self) -> bool {
        self == Self::LookAndKey
    }

    /// `inspect` pins the card (and Escape unpins it).
    pub(crate) fn by_key(self) -> bool {
        self != Self::Off
    }
}

/// Register the card's settings.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        CVAR,
        1_i64,
        CvarFlags::ARCHIVE,
        "Player card: 0 off, 1 when you look at a player without moving or press inspect, 2 only from the inspect key",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_playerCardDelay",
        1.5_f64,
        CvarFlags::ARCHIVE,
        "Seconds you must look at a player, steady, before their card shows (cg_playerCard 1)",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_playerCardOutline",
        true,
        CvarFlags::ARCHIVE,
        "Outline the player whose card is up",
    ))
}

/// Which player is being looked at, and since when.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Dwell {
    target: Option<u16>,
    since: i32,
    last_seen: i32,
    anchor: Vec3,
}

impl Dwell {
    /// Feed one frame: the player under the crosshair now (`seen`), the unit
    /// view direction and the clock in milliseconds.
    pub(crate) fn observe(&mut self, seen: Option<u16>, forward: Vec3, now: i32) {
        if self.target.is_some() && now < self.last_seen {
            // The clock restarted (a map change): start over.
            *self = Self::default();
        }
        match (seen, self.target) {
            (Some(client), Some(target)) if client == target => self.last_seen = now,
            (Some(client), _) => {
                *self = Self {
                    target: Some(client),
                    since: now,
                    last_seen: now,
                    anchor: forward,
                };
            }
            (None, Some(_)) if now - self.last_seen > GAP_MS => *self = Self::default(),
            (None, _) => {}
        }
        if self.target.is_some() && angle_degrees(forward, self.anchor) > STEADY_DEGREES {
            self.since = now;
            self.anchor = forward;
        }
    }

    /// The player looked at long enough, steadily, to show their card.
    pub(crate) fn ready(&self, now: i32, delay_ms: i32) -> Option<u16> {
        self.target.filter(|_| now - self.since >= delay_ms)
    }
}

/// The player an `inspect` press pinned the card to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Pin {
    target: Option<u16>,
    request: bool,
}

impl Pin {
    /// `inspect` was pressed: the next [`Self::resolve`] toggles the pin.
    pub(crate) fn request(&mut self) {
        self.request = true;
    }

    /// Escape: unpin at the next [`Self::resolve`], as a press would. False when nothing
    /// is pinned or an unpin is already pending, so the key can go on to the game menu.
    pub(crate) fn cancel(&mut self) -> bool {
        let cancels = self.target.is_some() && !self.request;
        self.request |= cancels;
        cancels
    }

    /// Apply a pending press and report who is pinned. A press with a pin set
    /// unpins; one without pins the player under the crosshair (`seen`), if any. The
    /// pin also drops when the player is gone (`alive` false) or the clock restarted.
    pub(crate) fn resolve(&mut self, seen: Option<u16>, alive: bool, restarted: bool) -> PinStep {
        let mut step = PinStep::default();
        if restarted || (self.target.is_some() && !alive) {
            step.released = self.target.take();
        }
        if std::mem::take(&mut self.request) {
            if let Some(client) = self.target.take() {
                step.released = Some(client);
            } else if let Some(client) = seen {
                self.target = Some(client);
            }
        }
        step.target = self.target;
        step
    }

    pub(crate) fn target(self) -> Option<u16> {
        self.target
    }
}

/// How long the pinned player has been out of sight.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Sight {
    lost_since: Option<i32>,
}

impl Sight {
    /// Feed whether the pinned player is in sight this frame (`None`: nothing pinned).
    /// True once they have been out of sight longer than [`LOST_MS`].
    pub(crate) fn observe(&mut self, in_sight: Option<bool>, now: i32) -> bool {
        if in_sight == Some(false) {
            now - *self.lost_since.get_or_insert(now) > LOST_MS
        } else {
            self.lost_since = None;
            false
        }
    }

    /// The pinned player is out of sight now: their card is hidden.
    pub(crate) fn hidden(self) -> bool {
        self.lost_since.is_some()
    }
}

/// Whether `client` is within [`SIGHT_RANGE`] of the camera with no wall between it and
/// their head or their hips.
fn in_sight(input: &mut Input<'_>, client: u16) -> bool {
    let Some(presented) = input
        .world
        .entity(sjk_runtime::EntityId::new(u64::from(client) + 1))
    else {
        return false;
    };
    let origin = Vec3::from_array(presented.sample(i64::from(input.now)).translation);
    let eye = input.camera.eye;
    if eye.distance(origin) > SIGHT_RANGE {
        return false;
    }
    let top = input
        .entities
        .iter()
        .find(|entity| entity.number() == client)
        .map_or(super::nameplate_math::head_height(0), |entity| {
            super::nameplate_math::head_height(entity.solid())
        });
    [origin + Vec3::Z * top, origin + Vec3::Z * HIP_HEIGHT]
        .into_iter()
        .any(|point| {
            let trace = input.bsp.trace_box_with(
                input.scratch,
                eye.to_array(),
                point.to_array(),
                Aabb::new([0.0; 3], [0.0; 3]).expect("point bounds are valid"),
                1 | 0x0200_0000,
            );
            super::identification::unoccluded(trace.fraction, trace.start_solid, trace.all_solid)
        })
}

/// What [`Pin::resolve`] decided.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PinStep {
    /// The pinned player after the step.
    pub(crate) target: Option<u16>,
    /// The player just unpinned.
    pub(crate) released: Option<u16>,
}

fn angle_degrees(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos().to_degrees()
}

/// The word after the hub name: VERIFIED, else the player's trust; empty for none.
fn trust_word(hub: &HubInfo) -> &'static str {
    use sjk_identity::Trust;
    match (hub.verified, hub.trust) {
        (true, _) => "VERIFIED",
        (false, Some(Trust::New)) => "NEW",
        (false, Some(Trust::Trusted)) => "TRUSTED",
        (false, Some(Trust::Banned)) => "CHAT BANNED",
        _ => "",
    }
}

/// The trust word's colour at alpha `a` (the trust marks' grey, green and red, and
/// VERIFIED the tick's colour); `None` for none.
fn trust_colour(hub: &HubInfo, a: f32) -> Option<Color> {
    use sjk_identity::Trust;
    if hub.verified {
        return crate::ui_renderer::tick_accent(hub.tick.shown(true), a);
    }
    match (hub.verified, hub.trust?) {
        (true, _) | (_, Trust::Unknown) => None,
        (false, Trust::New) => Some(Color::new(0.8, 0.82, 0.85, a)),
        (false, Trust::Trusted) => Some(Color::new(0.42, 0.86, 0.45, a)),
        (false, Trust::Banned) => Some(Color::new(0.97, 0.38, 0.35, a)),
    }
}

/// What the hub knows about the player.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct HubInfo {
    /// Hub display name, empty if they have none yet.
    pub(crate) name: String,
    pub(crate) verified: bool,
    /// Their tick, as the hub sent it: small on the emblem's foot, and VERIFIED's colour.
    pub(crate) tick: sjk_identity::Tick,
    /// Their trust (`PROTOCOL.md` of the hub, "Trust"): NEW, TRUSTED or CHAT BANNED
    /// where VERIFIED would be.
    pub(crate) trust: Option<sjk_identity::Trust>,
    /// The medals the SJK team gave them.
    pub(crate) medals: crate::medals::Medals,
    /// Their key id, and the version of their picture (empty for none).
    pub(crate) key_id: String,
    pub(crate) avatar: String,
    /// What their public profile says, once the hub has sent it.
    pub(crate) profile: Option<ProfileFacts>,
    /// What they share on their nameplate (`PROTOCOL.md`, "Vitals").
    pub(crate) vitals: Vitals,
    /// Whether they see the model the local player wears (`PROTOCOL.md`, "Models").
    pub(crate) model: ModelSight,
    /// The client they claimed from, when it has its own icon in place of SJK's emblem
    /// (`chat::client_icons`).
    pub(crate) client: Option<&'static crate::chat::client_icons::ClientIcon>,
}

/// Whether a player sees the model the local player wears: their game has it, or
/// draws its default in its place.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ModelSight {
    /// Not known: the hub has no list of theirs, the local player wears a retail model
    /// (every player has those) or `cl_shareModels` is off on their side.
    #[default]
    Unknown,
    Sees,
    Missing,
}

impl ModelSight {
    /// The card's line, empty for none.
    fn line(self) -> &'static str {
        match self {
            Self::Unknown => "",
            Self::Sees => "Sees your model",
            Self::Missing => "Can't see your model",
        }
    }
}

/// What a player shares on their nameplate, as the card says it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Vitals {
    /// Nothing, or the hub did not say.
    #[default]
    Private,
    /// Their health, shield and Force (`cg_nameplate`, See and share nameplates; before
    /// 17/10/2026 `cg_nameplateShare`).
    Shared,
    /// They see everyone's estimated bars (`cg_nameplateEstimate` of clients before
    /// 17/10/2026), which shares theirs.
    SeesAll,
}

impl Vitals {
    /// The card's line, empty for none.
    fn line(self) -> &'static str {
        match self {
            Self::Private => "",
            Self::Shared => "Shares their health, shield and Force",
            Self::SeesAll => "Sees your health, shield and Force",
        }
    }
}

/// Achievements shown as medallions on a pinned card.
const RAREST: usize = 3;

/// What the card takes from a player's public profile.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProfileFacts {
    /// The SJK team's own (`staff`).
    pub(crate) staff: bool,
    /// SJK's creator (`creator`): SJK CREATOR in place of SJK TEAM.
    pub(crate) creator: bool,
    /// JKR's creator (`jkr_creator`): a JKR CREATOR mark of its own, no staff rights.
    pub(crate) jkr_creator: bool,
    /// The bio's first line, empty for none.
    pub(crate) bio: String,
    /// Achievements unlocked, of those the client knows.
    pub(crate) unlocked: usize,
    /// Of how many: every obtainable one and the retired ones they hold
    /// ([`crate::achievements::total_for`]).
    pub(crate) total: usize,
    /// The rarest unlocked ones: the highest step of their ladder first (Legend of the
    /// Arena before Centurion), the latest first among equals.
    pub(crate) rarest: Vec<&'static crate::achievements::Kind>,
    /// The index of the rarest holocron tier they hold (`holocrons::TIERS`).
    pub(crate) holocron: Option<usize>,
    /// Registration, unix seconds; 0 when the hub did not say.
    pub(crate) created: i64,
}

impl ProfileFacts {
    pub(crate) fn of(profile: &sjk_identity::Profile) -> Self {
        use crate::achievements::{ALL, Category, Kind, held_kind};
        // A kind's step: how many kinds of the same counter have a lower goal.
        let step = |kind: &Kind| {
            ALL.iter()
                .filter(|other| other.source == kind.source && other.goal < kind.goal)
                .count()
        };
        let mut done: Vec<(&'static Kind, i64)> = profile
            .achievements
            .iter()
            .filter(|got| got.unlocked > 0 || (got.goal > 0 && got.progress >= got.goal))
            .filter_map(|got| held_kind(got).map(|kind| (kind, got.unlocked)))
            .collect();
        done.sort_by(|a, b| step(b.0).cmp(&step(a.0)).then(b.1.cmp(&a.1)));
        // The rarest holocron ever found: the counts are the unopened ones only, so the
        // listed ones (opened too) count as well.
        let counts = &profile.holocron_counts;
        let listed = profile
            .holocrons
            .iter()
            .filter_map(|holocron| crate::holocrons::Tier::from_id(&holocron.tier))
            .map(|tier| tier.index)
            .max();
        let held = [
            counts.uncommon,
            counts.rare,
            counts.legendary,
            counts.mythical,
        ]
        .iter()
        .rposition(|count| *count > 0);
        let holocron = held.max(listed);
        Self {
            staff: profile.staff,
            creator: profile.creator,
            jkr_creator: profile.jkr_creator,
            bio: profile
                .bio
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or_default()
                .to_owned(),
            unlocked: done.len(),
            // The Toy Master's found count in the total too: they are listed only then.
            total: crate::achievements::total_for(|kind| {
                done.iter().any(|(held, _)| held.id == kind.id)
            }) + done
                .iter()
                .filter(|(kind, _)| kind.category == Category::ToyMaster)
                .count(),
            rarest: done.iter().take(RAREST).map(|(kind, _)| *kind).collect(),
            holocron,
            created: profile.created,
        }
    }
}

/// Text slots of a card: ids are indices into [`Card::texts`].
const T_NAME: usize = 0;
const T_MODEL: usize = 1;
const T_SABER: usize = 2;
const T_EXTRA: usize = 3;
const T_HUB: usize = 4;
const T_VERIFIED: usize = 5;
const T_WORN: usize = 6;
const T_MEDALS: usize = 7;
const T_MEDALS_MORE: usize = 8;
const T_SHADER: usize = 9;
const T_STAFF: usize = 10;
const T_HOLOCRON: usize = 11;
const T_ACHIEVEMENTS: usize = 12;
const T_BIO: usize = 13;
const T_SINCE: usize = 14;
/// The rarest achievements' goals, one slot each.
const T_GOAL: usize = 15;
/// What they share on their nameplate, or that they see everyone's bars.
const T_VITALS: usize = T_GOAL + RAREST;
/// Whether they see the model the local player wears.
const T_SIGHT: usize = T_VITALS + 1;
/// The JKR CREATOR mark, beside or in place of SJK TEAM.
const T_JKR: usize = T_SIGHT + 1;
/// The second saber's (or a staff's other half's) shader, when it is not the first's.
const T_SHADER2: usize = T_JKR + 1;
const TEXTS: usize = T_SHADER2 + 1;
/// Draw commands a card holds: its own shapes and two shaders' swatches.
const LIST_CAPACITY: usize = 640;
/// The saber shader's swatch on the card, in card units.
const SWATCH_WIDTH: f32 = 74.0;
const SWATCH_HEIGHT: f32 = 18.0;
/// A chroma's colour wheel beside its swatch.
const CHROMA_MARK: f32 = 14.0;
/// The SJK TEAM mark's colour: a teal of its own, apart from the verified gold.
const STAFF_COLOUR: [f32; 3] = [0.22, 0.86, 0.78];
/// The SJK CREATOR mark's colour: a sun orange, apart from staff's teal and the gold.
const CREATOR_COLOUR: [f32; 3] = [1.0, 0.6, 0.22];
/// The JKR CREATOR mark's colour: rust (JKR is written in it), darker and redder than
/// the creator's orange.
const JKR_CREATOR_COLOUR: [f32; 3] = [0.84, 0.38, 0.2];
/// Characters a line of medal names holds on the card (Inter at 11).
const MEDAL_LINE_CHARS: usize = 52;
/// Height of a line of medal names: small print under the medallions.
const MEDAL_LINE: f32 = 15.0;

/// A player's card: the texts and values drawn, built when the target or the
/// hub's roster changes and drawn every frame without allocating.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Card {
    pub(crate) texts: [String; TEXTS],
    /// The saber shaders they wear, catalogue ids, when they draw on their blades: the
    /// first saber's, then the second saber's (or a staff's other half's) only when it
    /// is not the first's.
    shaders: [Option<&'static str>; 2],
    /// 0 free, 1 red, 2 blue, 3 spectator.
    team: u8,
    /// Blade colours of the one or two sabers, if retail colours.
    swatches: [Option<[u8; 3]>; 2],
    hub: Option<HubInfo>,
    /// The model's icon, once resolved.
    icon: Option<TextureId>,
    /// Their profile picture, once in the atlas.
    avatar: Option<TextureId>,
}

/// The saber shaders' loaded looks (as [`Card::shaders`]) and the time they move at, for
/// one frame's card.
#[derive(Clone, Copy, Default)]
pub(crate) struct Look<'a> {
    pub(crate) skins: [Option<&'a crate::saber_skins::LoadedSkin>; 2],
    pub(crate) seconds: f32,
}

impl Card {
    fn has_extra(&self) -> bool {
        !self.texts[T_EXTRA].is_empty()
    }

    fn has_worn(&self) -> bool {
        !self.texts[T_WORN].is_empty()
    }

    /// The SJK TEAM mark or a holocron gem: their row under the hub name.
    fn has_marks(&self) -> bool {
        [T_STAFF, T_JKR, T_HOLOCRON]
            .iter()
            .any(|id| !self.texts[*id].is_empty())
    }

    fn facts(&self) -> Option<&ProfileFacts> {
        self.hub.as_ref()?.profile.as_ref()
    }

    /// The pinned card's profile lines (achievements, bio, since when).
    fn profile_lines(&self) -> usize {
        [T_ACHIEVEMENTS, T_BIO, T_SINCE]
            .iter()
            .filter(|id| !self.texts[**id].is_empty())
            .count()
    }

    /// Lines of medal names a pinned card shows.
    fn medal_lines(&self) -> usize {
        [T_MEDALS, T_MEDALS_MORE]
            .iter()
            .filter(|id| !self.texts[**id].is_empty())
            .count()
    }
}

/// `atoi`: the number at the start of `text` (JoF EJK appends a cosmetic's name
/// after the colour digit).
fn atoi(text: &str) -> i32 {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

fn blade(info: LegacyClientInfo<'_>, key: &str) -> Option<[u8; 3]> {
    let index = atoi(info.text(key)?);
    (0..RETAIL_COLOURS)
        .contains(&index)
        .then(|| crate::saber::Color::ALL[index as usize].blade_rgb())
}

/// The swatches of a player whose blades take their team's `forced` colours over
/// their own `swatches`: one per saber held, including a custom colour's, which has
/// no stock swatch of its own.
fn team_swatches(
    info: &[u8],
    swatches: [Option<[u8; 3]>; 2],
    forced: [Option<[u8; 3]>; 2],
) -> [Option<[u8; 3]>; 2] {
    let dual = LegacyClientInfo::new(info)
        .text("st2")
        .is_some_and(|second| !second.is_empty() && !second.eq_ignore_ascii_case("none"));
    [
        forced[0].or(swatches[0]),
        dual.then(|| forced[1].or(swatches[1])).flatten(),
    ]
}

/// The cosmetic named in a `c1`/`c2` value (JoF EJK appends it after the colour digits).
fn cosmetic(value: Option<&str>) -> Option<String> {
    let (_, name) = sjk_client::split_color_value(value?);
    name.map(crate::cosmetics::display_name)
}

/// "Hat Santa hat  /  Cape Jedi" for what the player wears, empty when nothing.
fn worn_line(info: LegacyClientInfo<'_>) -> String {
    let hat = cosmetic(info.text("c1"));
    let cape = cosmetic(info.text("c2"));
    match (hat, cape) {
        (Some(hat), Some(cape)) => format!("Hat {hat}  /  Cape {cape}"),
        (Some(hat), None) => format!("Hat {hat}"),
        (None, Some(cape)) => format!("Cape {cape}"),
        (None, None) => String::new(),
    }
}

/// The card for the player whose configstring is `info`, wearing saber shader `shader`
/// (a catalogue id, `None` for the stock blade) on every blade.
#[cfg(test)]
pub(crate) fn card_from(info: &[u8], hub: Option<HubInfo>, shader: Option<&'static str>) -> Card {
    card_with(info, hub, [shader, shader])
}

/// [`card_from`] for a player wearing `shaders`: the first saber's, then the second
/// saber's (or a staff's other half's; `None` the stock blade, and when they hold no
/// second). The second gets a row of its own only when it is not the first's.
pub(crate) fn card_with(
    info: &[u8],
    hub: Option<HubInfo>,
    shaders: [Option<&'static str>; 2],
) -> Card {
    let info = LegacyClientInfo::new(info);
    let text = |key: &str| {
        info.bytes(key)
            .map(|bytes| decode_legacy(bytes).into_owned())
            .unwrap_or_default()
    };
    let mut card = Card {
        team: u8::try_from(atoi(&text("t"))).unwrap_or(0),
        ..Card::default()
    };
    card.texts[T_NAME] = {
        let name = text("n");
        if name.is_empty() { text("name") } else { name }
    };
    card.texts[T_MODEL] = text("model").replace('/', " / ");
    let (first, second) = (text("st"), text("st2"));
    let dual = !second.is_empty() && !second.eq_ignore_ascii_case("none");
    card.texts[T_SABER] = match (first.is_empty(), dual) {
        (true, _) => String::new(),
        (false, false) => first,
        (false, true) => format!("{first} + {second}"),
    };
    card.swatches = [
        blade(info, "c1"),
        if dual { blade(info, "c2") } else { None },
    ];
    card.texts[T_WORN] = worn_line(info);
    let [first, second] = shaders.map(|id| id.and_then(crate::unlockables::blade_skin));
    if let Some(skin) = first {
        card.shaders[0] = Some(skin.id);
        card.texts[T_SHADER] = skin.name.to_owned();
    }
    if let Some(skin) = second.filter(|second| Some(second.id) != card.shaders[0]) {
        card.shaders[1] = Some(skin.id);
        // Named as the Collection names the side it goes on.
        let side = if dual { "2nd blade" } else { "other half" };
        card.texts[T_SHADER2] = format!("{}  ({side})", skin.name);
    }
    card.texts[T_EXTRA] = info
        .text("skill")
        .filter(|skill| !skill.is_empty())
        .map(|skill| format!("Bot, skill {skill}"))
        .unwrap_or_default();
    if let Some(hub) = &hub {
        card.texts[T_HUB] = if hub.name.is_empty() {
            "SJK player".to_owned()
        } else {
            hub.name.clone()
        };
        card.texts[T_VERIFIED] = trust_word(hub).to_owned();
        // The names on one line, or two when they are long.
        for (medal, count) in hub.medals.iter() {
            let label = medal.label(count);
            let line = if card.texts[T_MEDALS_MORE].is_empty()
                && card.texts[T_MEDALS].len() + label.len() + 2 <= MEDAL_LINE_CHARS
            {
                T_MEDALS
            } else {
                T_MEDALS_MORE
            };
            if !card.texts[line].is_empty() {
                card.texts[line].push_str(", ");
            }
            card.texts[line].push_str(&label);
        }
        if let Some(facts) = &hub.profile {
            if facts.creator {
                card.texts[T_STAFF] = "SJK CREATOR".to_owned();
            } else if facts.staff {
                card.texts[T_STAFF] = "SJK TEAM".to_owned();
            }
            if facts.jkr_creator {
                card.texts[T_JKR] = "JKR CREATOR".to_owned();
            }
            if let Some(tier) = facts
                .holocron
                .and_then(|index| crate::holocrons::TIERS.get(index))
            {
                card.texts[T_HOLOCRON] = tier.title().into_owned();
            }
            if facts.unlocked > 0 {
                card.texts[T_ACHIEVEMENTS] =
                    format!("Achievements  {} / {}", facts.unlocked, facts.total);
            }
            card.texts[T_BIO] = facts.bio.clone();
            let date = crate::medals::date_text(facts.created);
            if !date.is_empty() {
                card.texts[T_SINCE] = format!("In SJK since {date}");
            }
            for (index, kind) in facts.rarest.iter().enumerate() {
                card.texts[T_GOAL + index] =
                    crate::achievements::medallion::goal_label(kind).to_string();
            }
        }
        hub.vitals.line().clone_into(&mut card.texts[T_VITALS]);
        hub.model.line().clone_into(&mut card.texts[T_SIGHT]);
    }
    card.hub = hub;
    card
}

/// Where the card goes and how big it is, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) card: Rect,
    /// Whether the card is to the right of the anchor point.
    pub(crate) right: bool,
}

/// The card's accent for a player of `team`: red, blue, or SJK's light blue.
fn accent_rgb(team: u8) -> [f32; 3] {
    match team {
        1 => [0.95, 0.28, 0.30],
        2 => [0.30, 0.60, 1.0],
        _ => [0.55, 0.78, 0.95],
    }
}

/// Put a card of `size` beside `anchor`, to its right unless that leaves the
/// screen, kept inside the viewport.
pub(crate) fn place(
    anchor: [f32; 2],
    size: [f32; 2],
    offset: f32,
    viewport: [f32; 2],
) -> Placement {
    let right = anchor[0] + offset + size[0] <= viewport[0] - offset * 0.25;
    let x = if right {
        anchor[0] + offset
    } else {
        anchor[0] - offset - size[0]
    };
    let margin = offset * 0.25;
    let x = x.clamp(margin, (viewport[0] - size[0] - margin).max(margin));
    let y = (anchor[1] - size[1] * 0.5).clamp(margin, (viewport[1] - size[1] - margin).max(margin));
    Placement {
        card: Rect::new(x, y, size[0], size[1]),
        right,
    }
}

/// Screen point of `anchor` and the pixels a body half-width (`BODY_SIDE` world units,
/// toward the camera's right) spans there, so the card clears the body at any range.
fn anchor_on_screen(camera: Camera, anchor: Vec3, limit: f32) -> Option<([f32; 2], f32)> {
    let point = camera.project_within(anchor, limit)?;
    let forward = (camera.target - camera.eye).normalize_or_zero();
    let right = forward.cross(camera.up).normalize_or_zero();
    let side = camera
        .project_within(anchor + right * BODY_SIDE, limit)
        .map_or(0.0, |side| (side[0] - point[0]).abs());
    Some((point, side))
}

/// The screen area (`[x, y, width, height]`) a body at `origin` whose box top is `top`
/// may cover: its reach projected, or the whole screen when part of it is behind the
/// eye or far off screen. The team outlines use it too.
pub(super) fn body_area(camera: Camera, origin: Vec3, top: f32) -> [f32; 4] {
    let [width, height] = camera.viewport;
    let mut low = [f32::MAX; 2];
    let mut high = [f32::MIN; 2];
    for corner in 0..8 {
        let pick = |bit: usize, a: f32, b: f32| if corner & bit == 0 { a } else { b };
        let point = origin
            + Vec3::new(
                pick(1, -REACH_SIDE, REACH_SIDE),
                pick(2, -REACH_SIDE, REACH_SIDE),
                pick(4, REACH_BELOW, top + REACH_ABOVE),
            );
        let Some(at) = camera.project_within(point, PINNED_LIMIT) else {
            return [0.0, 0.0, width, height];
        };
        for axis in 0..2 {
            low[axis] = low[axis].min(at[axis]);
            high[axis] = high[axis].max(at[axis]);
        }
    }
    [low[0], low[1], high[0] - low[0], high[1] - low[1]]
}

/// Push a text command for the card's text slot `id`.
fn put_text(
    list: &mut DrawList,
    id: usize,
    rect: Rect,
    size: f32,
    color: Color,
    weight: FontWeight,
    align: TextAlign,
) {
    let _ = list.push(DrawCommand::Text {
        rect,
        text: TextId(id as u32),
        size,
        color,
        align,
        overflow: TextOverflow::Ellipsis,
        weight,
        letter_spacing: 0.0,
    });
}

/// Inputs of one frame.
pub(crate) struct Input<'a> {
    /// The player under the crosshair this frame.
    pub(crate) seen: Option<u16>,
    /// The map and a trace scratch, for the walls between the camera and a player.
    pub(crate) bsp: &'a Bsp,
    pub(crate) scratch: &'a mut TraceScratch,
    pub(crate) game: &'a GameState,
    /// `cg_saberTeamColors` and `cg_ctfTeamSabers` (`team_sabers.rs`).
    pub(crate) team_sabers: crate::team_sabers::Settings,
    /// Siege's class-forced blade colours (`siege_sabers.rs`).
    pub(crate) class_sabers: &'a crate::siege_sabers::Forced,
    pub(crate) world: &'a sjk_runtime::World,
    pub(crate) now: i32,
    pub(crate) camera: Camera,
    /// The scoreboard, a menu, the console or intermission is up.
    pub(crate) hidden: bool,
    /// What the hub knows about the slot, shown as the game's name.
    pub(crate) hub: &'a dyn Fn(u8, &str) -> Option<HubInfo>,
    /// Counts changes to the hub's roster, so the card is rebuilt.
    pub(crate) hub_revision: u64,
    /// The snapshot's entities, for the height of each player's box (a crouch).
    pub(crate) entities: &'a [sjk_protocol::EntityState],
    /// The resolved model icon of a client slot.
    pub(crate) icon: &'a dyn Fn(u8) -> Option<TextureId>,
    /// The saber shaders the player in a client slot wears, catalogue ids, each only when
    /// it draws on their blade here (theirs by the looks, its pack loaded): the first
    /// saber's, then the second saber's or a staff's other half's (`None` when they hold
    /// neither).
    pub(crate) shader: &'a dyn Fn(u8) -> [Option<&'static str>; 2],
    /// The loaded saber shaders, for the swatch.
    pub(crate) skins: &'a crate::saber_skins::LoadedSkins,
    /// Counts changes to the looks and the loaded shaders, so the card is rebuilt.
    pub(crate) looks_revision: u64,
    /// Counts the profiles the hub sent, so the card is rebuilt.
    pub(crate) profiles_revision: u64,
}

/// The card's state: settings, dwell, fade and the draw list.
pub(crate) struct State {
    /// `cg_playerCard`: what opens the card.
    opens: Opens,
    outline: bool,
    delay_ms: i32,
    dwell: Dwell,
    pin: Pin,
    sight: Sight,
    /// The player whose card an `inspect` press just hid; their dwell is ignored until
    /// the crosshair leaves them, so the card does not come straight back.
    suppressed: Option<u16>,
    shown: Option<u16>,
    /// The slot and the revisions the card was built at.
    key: (u16, u64, u64, u64),
    card: Card,
    alpha: f32,
    last_frame: i32,
    /// The screen area the shown player's body may cover (`[x, y, width, height]`).
    area: [f32; 4],
    /// Shapes and text ids; texts resolve through [`Card::texts`].
    pub(crate) list: DrawList,
}

impl Default for State {
    fn default() -> Self {
        Self {
            opens: Opens::LookAndKey,
            outline: true,
            delay_ms: 1_500,
            dwell: Dwell::default(),
            pin: Pin::default(),
            sight: Sight::default(),
            suppressed: None,
            shown: None,
            key: (0, 0, 0, 0),
            card: Card::default(),
            alpha: 0.0,
            last_frame: 0,
            area: [0.0; 4],
            list: DrawList::new(LIST_CAPACITY),
        }
    }
}

impl State {
    /// Sample the settings once a frame, outside text emission.
    pub(crate) fn sample(&mut self, console: Option<&crate::console::ViewerConsole>) {
        self.opens = Opens::of(console.and_then(|c| c.integer_cvar(CVAR)));
        self.outline = console
            .and_then(|c| c.bool_cvar("cg_playercardoutline"))
            .unwrap_or(true);
        let seconds = crate::cgame_options::scalar(console, "cg_playercarddelay", 1.5);
        self.delay_ms = (seconds.clamp(0.3, 10.0) * 1_000.0) as i32;
    }

    /// Escape: unpin a pinned card, as `inspect` would. False when no card is pinned
    /// (or it is hidden with `cg_playerCard` off), so Escape opens the game menu instead.
    pub(crate) fn cancel(&mut self) -> bool {
        self.opens.by_key() && self.pin.cancel()
    }

    /// `inspect`: pin the card to the player under the crosshair, or unpin it.
    pub(crate) fn inspect(&mut self) {
        self.pin.request();
    }

    /// The player whose card is up, so their model icon can be resolved.
    pub(crate) fn shown_client(&self) -> Option<u16> {
        self.shown.filter(|_| self.alpha > 0.0)
    }

    /// The player to outline, the outline's colour (the card's accent, at the card's
    /// fade) and the screen area their body may cover, while the card is up and
    /// `cg_playerCardOutline` is on.
    pub(crate) fn outline(&self) -> Option<(u16, crate::card_outline::Frame)> {
        let client = self.shown_client().filter(|_| self.outline)?;
        let [r, g, b] = accent_rgb(self.card.team);
        let byte = |value: f32| (value * 255.0).round() as u8;
        Some((
            client,
            crate::card_outline::Frame {
                color: [byte(r), byte(g), byte(b), byte(self.alpha)],
                area: self.area,
            },
        ))
    }

    /// Pin the card to `client` without a press, for world shots.
    #[cfg(test)]
    pub(crate) fn pin_for_shot(&mut self, client: u16) {
        self.pin = Pin {
            target: Some(client),
            request: false,
        };
    }

    /// Forget everything shown (no session).
    pub(crate) fn clear(&mut self) {
        self.list.clear();
        self.dwell = Dwell::default();
        self.pin = Pin::default();
        self.sight = Sight::default();
        self.suppressed = None;
        self.shown = None;
        self.alpha = 0.0;
    }

    /// Advance the dwell and the fade and rebuild the draw list.
    pub(crate) fn update(&mut self, mut input: Input<'_>) {
        self.list.clear();
        let [width, height] = input.camera.viewport;
        self.area = [0.0, 0.0, width, height];
        let now = input.now;
        let forward = (input.camera.target - input.camera.eye).normalize_or_zero();
        // The clock going back is a map change: nothing pinned or shown carries over.
        let restarted = now < self.last_frame;
        if restarted {
            self.shown = None;
            self.alpha = 0.0;
            self.suppressed = None;
        }
        // No card through a wall or from afar: the crosshair's player counts only in
        // sight, and a pinned one hides out of sight and drops after a while.
        let aimed = input.seen;
        let aimed = aimed.filter(|&client| in_sight(&mut input, client));
        let before = self.pin.target();
        let pinned_in_sight = before.map(|client| in_sight(&mut input, client));
        let lost = self.sight.observe(pinned_in_sight, now);
        let alive = before.is_none_or(|client| {
            !lost
                && input
                    .world
                    .entity(sjk_runtime::EntityId::new(u64::from(client) + 1))
                    .is_some()
                && input
                    .game
                    .config_string(CS_PLAYERS + usize::from(client))
                    .is_some_and(|info| !info.is_empty())
        });
        let step = self.pin.resolve(aimed, alive, restarted);
        if step.target != before {
            self.sight = Sight::default();
        }
        if let Some(released) = step.released {
            self.suppressed = Some(released);
            self.dwell = Dwell::default();
        }
        if self.suppressed.is_some() && self.suppressed != aimed {
            self.suppressed = None;
        }
        let (pinned, want) = self.choose(aimed, step.target, input.hidden, forward, now);
        let elapsed = (now - self.last_frame).clamp(0, 100) as f32;
        self.last_frame = now;
        match want {
            Some(client) => {
                self.shown = Some(client);
                self.alpha = (self.alpha + elapsed / FADE_IN_MS).min(1.0);
            }
            None => {
                self.alpha = (self.alpha - elapsed / FADE_OUT_MS).max(0.0);
                if self.alpha <= 0.0 {
                    self.shown = None;
                }
            }
        }
        let Some(client) = self.shown else { return };
        if self.alpha <= 0.0 {
            return;
        }
        if self.key != Self::key_of(client, &input) || self.card == Card::default() {
            self.rebuild(client, &input);
        }
        let Some(presented) = input
            .world
            .entity(sjk_runtime::EntityId::new(u64::from(client) + 1))
        else {
            return;
        };
        let origin = Vec3::from_array(presented.sample(i64::from(now)).translation);
        let standing = super::nameplate_math::head_height(0);
        let top = input
            .entities
            .iter()
            .find(|entity| entity.number() == client)
            .map_or(standing, |entity| {
                super::nameplate_math::head_height(entity.solid())
            });
        self.area = body_area(input.camera, origin, top);
        let limit = if pinned.is_some() { PINNED_LIMIT } else { 1.2 };
        let anchor = origin + Vec3::Z * (HIP_HEIGHT + (top - standing).min(0.0) * HIP_DROP);
        let Some((hips, side)) = anchor_on_screen(input.camera, anchor, limit) else {
            return;
        };
        self.card.icon = u8::try_from(client)
            .ok()
            .and_then(|slot| (input.icon)(slot));
        self.card.avatar = self
            .card
            .hub
            .as_ref()
            .filter(|hub| !hub.avatar.is_empty())
            .and_then(|hub| crate::avatars::texture(&hub.key_id, &hub.avatar));
        let look = Look {
            skins: self
                .card
                .shaders
                .map(|id| id.and_then(|id| input.skins.get(id))),
            seconds: now as f32 / 1_000.0,
        };
        self.emit(hips, side, input.camera.viewport, pinned.is_some(), look);
    }

    /// Who is pinned and whose card shows this frame, from the player aimed at in sight
    /// (`aimed`), the pin's player (`pin`) and whether a menu or the like hides the card
    /// (`hidden`), as `cg_playerCard` lets them open it: a steady look only with 1 (Key
    /// only, 2, leaves the look out), the pin with 1 and 2.
    fn choose(
        &mut self,
        aimed: Option<u16>,
        pin: Option<u16>,
        hidden: bool,
        forward: Vec3,
        now: i32,
    ) -> (Option<u16>, Option<u16>) {
        let look = self.opens.by_look() && !hidden && self.suppressed.is_none();
        self.dwell.observe(aimed.filter(|_| look), forward, now);
        let pinned = pin.filter(|_| self.opens.by_key());
        let want = if hidden {
            None
        } else if pinned.is_some() {
            pinned.filter(|_| !self.sight.hidden())
        } else if self.opens.by_look() {
            self.dwell.ready(now, self.delay_ms)
        } else {
            None
        };
        (pinned, want)
    }

    fn key_of(client: u16, input: &Input<'_>) -> (u16, u64, u64, u64) {
        (
            client,
            input.hub_revision,
            input.profiles_revision,
            input.looks_revision,
        )
    }

    fn rebuild(&mut self, client: u16, input: &Input<'_>) {
        self.key = Self::key_of(client, input);
        let info = input
            .game
            .config_string(CS_PLAYERS + usize::from(client))
            .unwrap_or_default();
        let name = LegacyClientInfo::new(info)
            .bytes("n")
            .map(|bytes| decode_legacy(bytes).into_owned())
            .unwrap_or_default();
        let hub = u8::try_from(client)
            .ok()
            .and_then(|slot| (input.hub)(slot, &name));
        // In team games the card shows the blades as the world draws them: the team's
        // colour, with no shader over it under CTF team sabers (`team_sabers.rs`), and
        // in Siege the class's forced colour.
        let team_blade = crate::team_sabers::Rules::new(input.team_sabers, input.game)
            .of_client(input.game, client)
            .over_class(
                input
                    .class_sabers
                    .get(usize::from(client))
                    .copied()
                    .unwrap_or_default(),
            );
        let shaders = u8::try_from(client)
            .ok()
            .map_or([None; 2], |slot| (input.shader)(slot))
            .map(|shader| shader.filter(|_| !team_blade.plain));
        self.card = card_with(info, hub, shaders);
        if team_blade.any() {
            self.card.swatches = team_swatches(info, self.card.swatches, team_blade.colors);
        }
    }

    /// The card's height in pixels at `unit`; a `pinned` card adds the medals' names
    /// and the profile's lines.
    fn height(card: &Card, unit: f32, pinned: bool) -> f32 {
        let row = |height: f32| height * unit;
        let mut height = 14.0 * unit * 2.0 + row(30.0) + row(22.0);
        for (shown, rows) in [
            (!card.texts[T_SABER].is_empty(), 22.0),
            (card.shaders[0].is_some(), 24.0),
            (card.shaders[1].is_some(), 24.0),
            (card.has_worn(), 22.0),
            (card.has_extra(), 22.0),
        ] {
            if shown {
                height += row(rows);
            }
        }
        // The icon (or the picture) fills the top right: the rows beside it never end
        // above it.
        height = height.max(14.0 * unit * 2.0 + row(56.0));
        let Some(hub) = &card.hub else {
            return height;
        };
        height += row(10.0) + row(26.0);
        if card.has_marks() {
            height += row(26.0);
        }
        for line in [T_VITALS, T_SIGHT] {
            if !card.texts[line].is_empty() {
                height += row(22.0);
            }
        }
        if !hub.medals.is_empty() {
            height += row(30.0);
            if pinned {
                height += row(MEDAL_LINE) * card.medal_lines() as f32;
            }
        }
        if pinned && card.profile_lines() > 0 {
            height += row(6.0);
            if !card.texts[T_ACHIEVEMENTS].is_empty() {
                height += row(30.0);
            }
            if !card.texts[T_BIO].is_empty() {
                height += row(20.0);
            }
            if !card.texts[T_SINCE].is_empty() {
                height += row(18.0);
            }
        }
        height
    }

    /// Draw the card beside `anchor` (the hips); a `pinned` card names the player's
    /// medals and adds the profile's lines. `look` is the worn shader's, for its swatch.
    fn emit(
        &mut self,
        anchor: [f32; 2],
        side: f32,
        viewport: [f32; 2],
        pinned: bool,
        look: Look<'_>,
    ) {
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let a = self.alpha;
        let card = &self.card;
        let pad = 14.0 * unit;
        let width = 290.0 * unit;
        let row = |height: f32| height * unit;
        let height = Self::height(card, unit, pinned);
        let medals = card.hub.as_ref().map(|hub| hub.medals).unwrap_or_default();
        // Keep clear of the body: the world gap projected, never less than a fixed one.
        let offset = side.max(14.0 * unit) + 12.0 * unit;
        let placed = place(anchor, [width, height], offset, viewport);
        let rect = placed.card;
        let white = |alpha: f32| Color::new(1.0, 1.0, 1.0, alpha * a);
        let muted = Color::new(0.74, 0.78, 0.86, a);
        let [r, g, b] = accent_rgb(card.team);
        let accent = Color::new(r, g, b, a);
        // The leader from the hips to the card.
        let (from, to) = if placed.right {
            (anchor[0], rect.x)
        } else {
            (rect.x + rect.width, anchor[0])
        };
        if anchor[1] >= rect.y && anchor[1] <= rect.y + rect.height {
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: Rect::new(
                    from,
                    anchor[1] - 0.75 * unit,
                    (to - from).max(0.0),
                    1.5 * unit,
                ),
                color: white(0.5),
            });
        }
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect: Rect::new(
                anchor[0] - 3.5 * unit,
                anchor[1] - 3.5 * unit,
                7.0 * unit,
                7.0 * unit,
            ),
            radius: 3.5 * unit,
            color: white(0.9),
        });
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * unit,
            color: Color::new(0.03, 0.04, 0.07, 0.82 * a),
        });
        let _ = self.list.push(DrawCommand::Border {
            rect,
            radius: 8.0 * unit,
            width: unit.max(1.0),
            color: white(0.16),
        });
        let _ = self.list.push(DrawCommand::SolidRect {
            rect: Rect::new(
                rect.x + 1.0 * unit,
                rect.y + 8.0 * unit,
                3.0 * unit,
                rect.height - 16.0 * unit,
            ),
            color: accent,
        });
        let inner = rect.width - pad * 2.0;
        let mut y = rect.y + pad;
        let left = rect.x + pad;
        // The profile picture fills the top right, beside the name and model, with the
        // model's icon on its lower left corner; without a picture the model's icon
        // fills it.
        let icon_side = row(50.0);
        let icon_x = rect.x + rect.width - pad - icon_side;
        let head_width = match (card.avatar, card.icon) {
            (Some(picture), icon) => {
                let _ = self.list.push(DrawCommand::TexturedQuad {
                    rect: Rect::new(icon_x, y, icon_side, icon_side),
                    texture: picture,
                    color: Color::new(1.0, 1.0, 1.0, a),
                });
                if let Some(texture) = icon {
                    let small = row(22.0);
                    let corner = Rect::new(
                        icon_x - row(4.0),
                        y + icon_side - small + row(4.0),
                        small,
                        small,
                    );
                    let _ = self.list.push(DrawCommand::RoundedRect {
                        rect: corner,
                        radius: row(4.0),
                        color: Color::new(0.03, 0.04, 0.07, 0.9 * a),
                    });
                    let _ = self.list.push(DrawCommand::TexturedQuad {
                        rect: corner,
                        texture,
                        color: Color::new(1.0, 1.0, 1.0, a),
                    });
                }
                inner - icon_side - 12.0 * unit
            }
            (None, Some(texture)) => {
                let _ = self.list.push(DrawCommand::TexturedQuad {
                    rect: Rect::new(icon_x, y, icon_side, icon_side),
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, a),
                });
                inner - icon_side - 8.0 * unit
            }
            (None, None) => inner,
        };
        // The JoF emblem on the name's left ([`crate::jof_tag`]).
        let jof = if let Some(clan) = crate::jof_tag::clan(&card.texts[T_NAME]) {
            let side = crate::jof_tag::side(23.0 * unit);
            crate::jof_tag::draw(clan, left, y + row(30.0) * 0.5, side, a, |command| {
                let _ = self.list.push(command);
            });
            crate::jof_tag::room(clan, side)
        } else {
            0.0
        };
        put_text(
            &mut self.list,
            T_NAME,
            Rect::new(left + jof, y, head_width - jof, row(30.0)),
            23.0 * unit,
            white(1.0),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += row(30.0);
        put_text(
            &mut self.list,
            T_MODEL,
            Rect::new(left, y, head_width, row(22.0)),
            15.0 * unit,
            muted,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += row(22.0);
        // Rows under the icon run the card's width once past it.
        let beside = |y: f32| {
            if y < rect.y + pad + icon_side + row(4.0) {
                head_width
            } else {
                inner
            }
        };
        if !card.texts[T_SABER].is_empty() {
            let swatches = card.swatches.iter().flatten().count() as f32;
            let room = beside(y);
            put_text(
                &mut self.list,
                T_SABER,
                Rect::new(left, y, room - swatches * 18.0 * unit, row(22.0)),
                15.0 * unit,
                muted,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let mut x = left + room - 14.0 * unit;
            for rgb in card.swatches.iter().flatten().rev() {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    // Centred on the hilt name's letters, which sit above the row's middle.
                    rect: Rect::new(x, y + 2.0 * unit, 14.0 * unit, 14.0 * unit),
                    radius: 7.0 * unit,
                    color: Color::new(
                        f32::from(rgb[0]) / 255.0,
                        f32::from(rgb[1]) / 255.0,
                        f32::from(rgb[2]) / 255.0,
                        a,
                    ),
                });
                x -= 18.0 * unit;
            }
            y += row(22.0);
        }
        // The saber shaders they wear: each its live swatch, as the Collection draws it,
        // and its name; the second's only when it is not the first's.
        for side in 0..2 {
            let Some(shader) = card.shaders[side] else {
                continue;
            };
            let skin = look.skins[side];
            let swatch = [
                left,
                y + row(3.0),
                SWATCH_WIDTH * unit,
                SWATCH_HEIGHT * unit,
            ];
            let _ = self.list.push(DrawCommand::PushOpacity(a));
            let _ = crate::console::collection_panel::swatch::small_blade(
                &mut self.list,
                &crate::menu::sjk::Frame {
                    s: 1.0,
                    origin: [0.0, 0.0],
                },
                swatch,
                skin,
                look.seconds,
                // A chroma in its saber's colour, as the world draws it (a staff's other
                // half takes the first saber's).
                card.swatches[side].or(card.swatches[0]).map_or(0.0, |rgb| {
                    crate::console::collection_panel::swatch::chroma_turn(skin, rgb)
                }),
            );
            // A thin edge in the shader's tier colour, which its name takes too.
            let blade_skin = crate::unlockables::blade_skin(shader);
            let tier = blade_skin.map(|skin| skin.tier.colour());
            if let Some(tier) = tier {
                let _ = self.list.push(DrawCommand::Border {
                    rect: Rect::new(swatch[0], swatch[1], swatch[2], swatch[3]),
                    radius: 3.0 * unit,
                    width: 1.2 * unit,
                    color: Color::new(tier.r, tier.g, tier.b, 0.9),
                });
            }
            // Its tier's breath, quiet: the HUD shows only that much of a tier's show.
            if let Some(skin) = blade_skin {
                crate::rarity_fx::draw(
                    &mut self.list,
                    crate::rarity_fx::Item::of(skin.tier, true, crate::rarity_fx::Size::Tiny),
                    Rect::new(swatch[0], swatch[1], swatch[2], swatch[3]),
                    3.0 * unit,
                    unit,
                    look.seconds,
                );
            }
            // A chroma's colour wheel between the swatch and its name.
            let mut text_x = left + swatch[2] + 8.0 * unit;
            if crate::unlockables::is_chroma(shader) {
                crate::console::collection_panel::swatch::chroma_mark(
                    &mut self.list,
                    &crate::menu::sjk::Frame {
                        s: 1.0,
                        origin: [0.0, 0.0],
                    },
                    text_x,
                    y + row(3.0) + (SWATCH_HEIGHT - CHROMA_MARK) * 0.5 * unit,
                    CHROMA_MARK * unit,
                    // Inside the card's opacity group, which fades it with the card.
                    1.0,
                );
                text_x += (CHROMA_MARK + 6.0) * unit;
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
            put_text(
                &mut self.list,
                [T_SHADER, T_SHADER2][side],
                Rect::new(text_x, y + row(1.0), left + beside(y) - text_x, row(22.0)),
                15.0 * unit,
                // The tier's colour lifted toward white, to read on the dark card.
                tier.map_or(white(0.92), |tier| {
                    let lift = |channel: f32| channel + (1.0 - channel) * 0.35;
                    Color::new(lift(tier.r), lift(tier.g), lift(tier.b), 0.95 * a)
                }),
                FontWeight::Semibold,
                TextAlign::Start,
            );
            y += row(24.0);
        }
        if card.has_worn() {
            put_text(
                &mut self.list,
                T_WORN,
                Rect::new(left, y, beside(y), row(22.0)),
                15.0 * unit,
                muted,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += row(22.0);
        }
        if card.has_extra() {
            put_text(
                &mut self.list,
                T_EXTRA,
                Rect::new(left, y, beside(y), row(22.0)),
                15.0 * unit,
                muted,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += row(22.0);
        }
        y = y.max(rect.y + pad + row(56.0));
        if let Some(hub) = &card.hub {
            y += row(10.0);
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: Rect::new(left, y - row(6.0), inner, unit.max(1.0)),
                color: white(0.12),
            });
            let texture = crate::chat::client_icons::texture(hub.client);
            // Another client's icon keeps its own colours; SJK's emblem is white, its
            // tick (or trust mark) small on its foot, as on the scoreboard.
            let tint = if texture != crate::ui_renderer::LOGO_TEXTURE {
                white(1.0)
            } else {
                white(0.95)
            };
            let side = row(22.0);
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect: Rect::new(left, y, side, side),
                texture,
                color: tint,
            });
            if let Some(mark) = crate::ui_renderer::mark_texture(hub.verified, hub.tick, hub.trust)
            {
                let small = side * 0.55;
                let _ = self.list.push(DrawCommand::TexturedQuad {
                    rect: Rect::new(
                        left + side - small * 0.8,
                        y + side - small * 0.9,
                        small,
                        small,
                    ),
                    texture: mark,
                    color: white(1.0),
                });
            }
            let hub_x = left + side + 8.0 * unit;
            let verified_width = if trust_word(hub).is_empty() {
                0.0
            } else {
                92.0 * unit
            };
            put_text(
                &mut self.list,
                T_HUB,
                Rect::new(
                    hub_x,
                    y + row(2.0),
                    inner - side - 8.0 * unit - verified_width,
                    row(26.0),
                ),
                17.0 * unit,
                tint,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if !trust_word(hub).is_empty() {
                put_text(
                    &mut self.list,
                    T_VERIFIED,
                    // Lowered to share the hub name's centre line, as the emblem does.
                    Rect::new(left, y + row(5.0), inner, row(26.0)),
                    12.0 * unit,
                    trust_colour(hub, a).unwrap_or(tint),
                    FontWeight::Semibold,
                    TextAlign::End,
                );
            }
            y += row(26.0);
            // The SJK TEAM mark, a teal pill drawn as VERIFIED is written (SJK CREATOR,
            // wider and orange, for the creator), JKR CREATOR's rust pill, and a gem of the rarest holocron tier
            // they hold, with its name.
            if card.has_marks() {
                let mut x = left;
                let creator = card.facts().is_some_and(|facts| facts.creator);
                let marks = [
                    if creator {
                        (T_STAFF, CREATOR_COLOUR, 106.0)
                    } else {
                        (T_STAFF, STAFF_COLOUR, 78.0)
                    },
                    (T_JKR, JKR_CREATOR_COLOUR, 106.0),
                ];
                for (id, [r, g, b], width) in marks {
                    if card.texts[id].is_empty() {
                        continue;
                    }
                    let pill = Rect::new(x, y + row(3.0), width * unit, row(18.0));
                    let _ = self.list.push(DrawCommand::RoundedRect {
                        rect: pill,
                        radius: pill.height * 0.5,
                        color: Color::new(r, g, b, 0.18 * a),
                    });
                    let _ = self.list.push(DrawCommand::Border {
                        rect: pill,
                        radius: pill.height * 0.5,
                        width: unit.max(1.0),
                        color: Color::new(r, g, b, 0.85 * a),
                    });
                    put_text(
                        &mut self.list,
                        id,
                        Rect::new(pill.x, pill.y + row(3.5), pill.width, pill.height),
                        11.0 * unit,
                        Color::new(r, g, b, a),
                        FontWeight::Semibold,
                        TextAlign::Center,
                    );
                    x += pill.width + 10.0 * unit;
                }
                if let Some(tier) = card
                    .facts()
                    .and_then(|facts| facts.holocron)
                    .and_then(|index| crate::holocrons::TIERS.get(index))
                {
                    let half = row(8.0);
                    crate::holocrons::gem::draw(
                        &mut self.list,
                        [x + half, y + row(12.0)],
                        half,
                        tier.colour,
                        a,
                        crate::holocrons::gem::MARK_ROWS,
                    );
                    let text_x = x + half * 2.0 + 6.0 * unit;
                    put_text(
                        &mut self.list,
                        T_HOLOCRON,
                        Rect::new(text_x, y + row(5.0), left + inner - text_x, row(22.0)),
                        13.0 * unit,
                        tier.colour_alpha(a),
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
                y += row(26.0);
            }
            // What they share on their nameplate; seeing everyone's bars in amber, as
            // it concerns the reader.
            if !card.texts[T_VITALS].is_empty() {
                let color = if hub.vitals == Vitals::SeesAll {
                    Color::new(1.0, 0.72, 0.3, 0.95 * a)
                } else {
                    muted
                };
                put_text(
                    &mut self.list,
                    T_VITALS,
                    Rect::new(left, y, inner, row(22.0)),
                    13.0 * unit,
                    color,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += row(22.0);
            }
            // Whether they see the model the local player wears; not seeing it in
            // amber, as the vitals line.
            if !card.texts[T_SIGHT].is_empty() {
                let color = if hub.model == ModelSight::Missing {
                    Color::new(1.0, 0.72, 0.3, 0.95 * a)
                } else {
                    muted
                };
                put_text(
                    &mut self.list,
                    T_SIGHT,
                    Rect::new(left, y, inner, row(22.0)),
                    13.0 * unit,
                    color,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += row(22.0);
            }
            // The medals the SJK team gave them, small medallions in a row; a pinned
            // card names them under it in small print, with how often a repeatable one
            // was given.
            if !medals.is_empty() {
                let icon = row(26.0);
                let mut x = left;
                for (medal, _) in medals.iter() {
                    let rect = Rect::new(x, y + row(2.0), icon, icon);
                    let _ = self.list.push(DrawCommand::TexturedQuad {
                        rect,
                        texture: medal.icon(),
                        color: Color::new(1.0, 1.0, 1.0, a),
                    });
                    // Its rank's device, a badge (`medals/rank.rs`).
                    crate::medals::rank::icon_device(&mut self.list, medals.rank(medal), rect);
                    x += icon + 6.0 * unit;
                }
                y += row(30.0);
                if pinned {
                    for (index, id) in [T_MEDALS, T_MEDALS_MORE]
                        .into_iter()
                        .take(card.medal_lines())
                        .enumerate()
                    {
                        put_text(
                            &mut self.list,
                            id,
                            Rect::new(
                                left,
                                y + index as f32 * row(MEDAL_LINE),
                                inner,
                                row(MEDAL_LINE),
                            ),
                            11.0 * unit,
                            Color {
                                a: 0.7 * a,
                                ..muted
                            },
                            FontWeight::Regular,
                            TextAlign::Start,
                        );
                    }
                    y += row(MEDAL_LINE) * card.medal_lines() as f32;
                }
            }
            if pinned && card.profile_lines() > 0 {
                y += row(6.0);
                if !card.texts[T_ACHIEVEMENTS].is_empty() {
                    put_text(
                        &mut self.list,
                        T_ACHIEVEMENTS,
                        Rect::new(left, y + row(8.5), inner, row(24.0)),
                        14.0 * unit,
                        white(0.9),
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                    // The rarest ones as small medallions, right-aligned: the disc in
                    // the category's colour, a gold ring and the goal.
                    let radius = row(12.0);
                    let rarest = card.facts().map_or(&[][..], |facts| &facts.rarest[..]);
                    let mut x = left + inner - radius;
                    for (index, kind) in rarest.iter().enumerate().rev() {
                        let centre = [x, y + row(15.0)];
                        let hue = crate::achievements::medallion::tint(kind.category);
                        let _ = self.list.push(DrawCommand::RoundedRect {
                            rect: Rect::new(
                                centre[0] - radius,
                                centre[1] - radius,
                                radius * 2.0,
                                radius * 2.0,
                            ),
                            radius,
                            color: Color { a: 0.3 * a, ..hue },
                        });
                        let _ = self.list.push(DrawCommand::Arc {
                            center: centre,
                            radius,
                            width: 1.8 * unit,
                            start: 0.0,
                            sweep: std::f32::consts::TAU,
                            color: Color::new(1.0, 0.82, 0.25, a),
                            knockout: None,
                        });
                        put_text(
                            &mut self.list,
                            T_GOAL + index,
                            Rect::new(
                                centre[0] - radius,
                                centre[1] - row(5.0),
                                radius * 2.0,
                                row(14.0),
                            ),
                            9.0 * unit,
                            Color::new(1.0, 0.86, 0.4, a),
                            FontWeight::Semibold,
                            TextAlign::Center,
                        );
                        x -= radius * 2.0 + 5.0 * unit;
                    }
                    y += row(30.0);
                }
                if !card.texts[T_BIO].is_empty() {
                    put_text(
                        &mut self.list,
                        T_BIO,
                        Rect::new(left, y, inner, row(20.0)),
                        13.0 * unit,
                        Color {
                            a: 0.85 * a,
                            ..muted
                        },
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                    y += row(20.0);
                }
                if !card.texts[T_SINCE].is_empty() {
                    put_text(
                        &mut self.list,
                        T_SINCE,
                        Rect::new(left, y, inner, row(18.0)),
                        11.0 * unit,
                        Color {
                            a: 0.7 * a,
                            ..muted
                        },
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
            }
        }
    }

    /// Resolve the card's text ids at submission.
    pub(crate) fn append(&self, vertices: &mut Vec<TextVertex>, font: &UiFont, viewport: [f32; 2]) {
        crate::ui_renderer::append_text_commands(
            &self.list,
            |id| {
                self.card
                    .texts
                    .get(id.0 as usize)
                    .map_or("", String::as_str)
            },
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }
}

#[cfg(test)]
impl State {
    /// A card drawn as if its player had been looked at (`pinned`: with `inspect`), for
    /// the off-screen snapshots (`menu_snapshot.rs`).
    pub(crate) fn preview(card: Card, anchor: [f32; 2], viewport: [f32; 2], pinned: bool) -> Self {
        let mut state = Self {
            card,
            alpha: 1.0,
            ..Self::default()
        };
        state.emit(anchor, 24.0, viewport, pinned, Look::default());
        state
    }

    /// As [`Self::preview`], the worn shaders drawn from `skins` (as [`Card::shaders`]).
    pub(crate) fn preview_with(
        card: Card,
        anchor: [f32; 2],
        viewport: [f32; 2],
        pinned: bool,
        skins: [Option<&crate::saber_skins::LoadedSkin>; 2],
    ) -> Self {
        let mut state = Self {
            card,
            alpha: 1.0,
            ..Self::default()
        };
        state.emit(
            anchor,
            24.0,
            viewport,
            pinned,
            Look {
                skins,
                seconds: 1.3,
            },
        );
        state
    }

    /// The text a draw command's id names.
    pub(crate) fn resolve_text(&self, id: TextId) -> &str {
        self.card
            .texts
            .get(id.0 as usize)
            .map_or("", String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORWARD: Vec3 = Vec3::X;

    fn turned(degrees: f32) -> Vec3 {
        let radians = degrees.to_radians();
        Vec3::new(radians.cos(), radians.sin(), 0.0)
    }

    #[test]
    fn a_steady_look_becomes_ready_after_the_delay() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 1_000);
        assert_eq!(dwell.ready(1_000, 1_500), None);
        dwell.observe(Some(4), FORWARD, 2_000);
        assert_eq!(dwell.ready(2_000, 1_500), None);
        dwell.observe(Some(4), FORWARD, 2_500);
        assert_eq!(dwell.ready(2_500, 1_500), Some(4));
    }

    #[test]
    fn turning_the_view_starts_the_wait_again() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 0);
        dwell.observe(Some(4), FORWARD, 2_000);
        assert_eq!(dwell.ready(2_000, 1_500), Some(4));
        dwell.observe(Some(4), turned(10.0), 2_050);
        assert_eq!(dwell.ready(2_050, 1_500), None);
        // A small drift is not a turn.
        let mut steady = Dwell::default();
        steady.observe(Some(4), FORWARD, 0);
        steady.observe(Some(4), turned(3.0), 2_000);
        assert_eq!(steady.ready(2_000, 1_500), Some(4));
    }

    #[test]
    fn another_player_or_a_long_gap_starts_over_but_a_short_gap_does_not() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 0);
        dwell.observe(None, FORWARD, 200);
        dwell.observe(Some(4), FORWARD, 300);
        dwell.observe(Some(4), FORWARD, 1_600);
        assert_eq!(
            dwell.ready(1_600, 1_500),
            Some(4),
            "a 200 ms gap is forgiven"
        );
        dwell.observe(Some(7), FORWARD, 1_700);
        assert_eq!(dwell.ready(1_700, 0), Some(7));
        assert_eq!(dwell.ready(1_700, 1), None, "a new target waits again");
        dwell.observe(None, FORWARD, 1_800);
        dwell.observe(None, FORWARD, 2_300);
        assert_eq!(dwell.ready(2_300, 0), None, "a long gap forgets the target");
    }

    #[test]
    fn a_clock_that_goes_back_forgets_the_target() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 9_000);
        dwell.observe(None, FORWARD, 100);
        assert_eq!(dwell.ready(100, 0), None);
    }

    fn info(text: &str) -> Vec<u8> {
        text.replace('|', "\\").into_bytes()
    }

    #[test]
    fn a_card_reads_what_the_server_publishes() {
        let card = card_from(
            &info("n|^1Sol|t|1|model|kyle/default|ds|m|st|single_1|st2|none|c1|4|c2|0|hc|100|"),
            None,
            None,
        );
        assert_eq!(card.texts[T_NAME], "^1Sol");
        assert_eq!(card.texts[T_MODEL], "kyle / default");
        assert_eq!(card.texts[T_SABER], "single_1", "none is no second saber");
        assert_eq!(card.team, 1);
        assert_eq!(card.swatches, [Some([51, 102, 255]), None]);
        assert!(card.texts[T_EXTRA].is_empty() && card.hub.is_none());
    }

    #[test]
    fn dual_sabers_and_bots_get_their_lines_but_duel_records_do_not() {
        let dual = card_from(
            &info("n|Fox|t|3|model|jedi_hf/blue|st|dual_1|st2|dual_2|c1|0|c2|3|w|5|l|2|"),
            None,
            None,
        );
        assert_eq!(dual.texts[T_SABER], "dual_1 + dual_2");
        assert_eq!(dual.swatches, [Some([255, 51, 51]), Some([51, 255, 51])]);
        assert!(!dual.has_extra(), "no wins and losses on the card");
        let bot = card_from(&info("n|Kyle|t|0|model|kyle|skill|3|"), None, None);
        assert_eq!(bot.texts[T_EXTRA], "Bot, skill 3");
    }

    #[test]
    fn a_team_blade_fills_one_swatch_per_saber_held() {
        const RED: [u8; 3] = [255, 51, 51];
        const GREEN: [u8; 3] = [51, 255, 51];
        let both = [Some(RED); 2];
        let custom = info("n|Sol|t|1|st|single_1|c1|9|");
        assert_eq!(team_swatches(&custom, [None; 2], both), [Some(RED), None]);
        let dual = info("n|Fox|t|1|st|dual_1|st2|dual_2|c1|2|c2|3|");
        let own = [Some([255, 255, 51]), Some(GREEN)];
        assert_eq!(team_swatches(&dual, own, both), [Some(RED), Some(RED)]);
        // `cg_saberTeamColors 0` replaces only one of them.
        assert_eq!(
            team_swatches(&dual, own, [Some(RED), None]),
            [Some(RED), Some(GREEN)]
        );
        let staff_none = info("n|Kyle|t|1|st|staff_1|st2|none|c1|2|");
        assert_eq!(team_swatches(&staff_none, own, both), [Some(RED), None]);
    }

    #[test]
    fn a_cosmetic_after_the_colour_digit_is_still_a_colour() {
        let card = card_from(&info("n|Sol|st|single_1|c1|4santahat|"), None, None);
        assert_eq!(card.swatches[0], Some([51, 102, 255]));
        let custom = card_from(&info("n|Sol|st|single_1|c1|9|"), None, None);
        assert_eq!(
            custom.swatches[0], None,
            "only retail colours have a swatch"
        );
    }

    #[test]
    fn the_hub_adds_the_emblem_name_and_verification() {
        let known = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: "Sol the Fox".to_owned(),
                verified: true,
                medals: crate::medals::Medals::default(),
                ..HubInfo::default()
            }),
            None,
        );
        assert_eq!(known.texts[T_HUB], "Sol the Fox");
        assert_eq!(known.texts[T_VERIFIED], "VERIFIED");
        assert!(known.texts[T_MEDALS].is_empty());
        let unnamed = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: String::new(),
                verified: false,
                medals: crate::medals::Medals::default(),
                ..HubInfo::default()
            }),
            None,
        );
        assert_eq!(unnamed.texts[T_HUB], "SJK player");
        assert!(unnamed.texts[T_VERIFIED].is_empty());
    }

    /// Every medal there is fits one line; a second one waits for more medals.
    #[test]
    fn the_whole_catalogue_fits_one_line_of_names() {
        let medal = |id: &str| sjk_identity::Medal {
            id: id.to_owned(),
            count: 1,
            awarded: 0,
            note: String::new(),
            rank: 1,
        };
        let card = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: "Sol".to_owned(),
                verified: false,
                medals: crate::medals::Medals::from_wire(&[
                    medal("early_tester"),
                    medal("early_contributor"),
                    medal("bug_hunter"),
                ]),
                ..HubInfo::default()
            }),
            None,
        );
        assert_eq!(card.texts[T_MEDALS], "Tester, Contributor, Bug Hunter");
        assert!(card.texts[T_MEDALS_MORE].is_empty());
        assert_eq!(card.medal_lines(), 1);
    }

    #[test]
    fn medals_show_as_medallions_and_a_pinned_card_names_them() {
        let medal = |id: &str, count| sjk_identity::Medal {
            id: id.to_owned(),
            count,
            awarded: 0,
            note: String::new(),
            rank: 1,
        };
        let card = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: "Sol".to_owned(),
                verified: true,
                medals: crate::medals::Medals::from_wire(&[
                    medal("bug_hunter", 2),
                    medal("early_tester", 1),
                    medal("unknown", 1),
                ]),
                ..HubInfo::default()
            }),
            None,
        );
        // Bug Hunter is held once since 21/10/2026: an old count of 2 shows no count.
        assert_eq!(card.texts[T_MEDALS], "Tester, Bug Hunter");
        assert!(card.texts[T_MEDALS_MORE].is_empty());
        let icons = |state: &State| {
            state
                .list
                .commands()
                .iter()
                .filter(|command| {
                    matches!(command, DrawCommand::TexturedQuad { texture, .. }
                        if *texture == crate::medals::Medal::Tester.icon()
                            || *texture == crate::medals::Medal::BugHunter.icon())
                })
                .count()
        };
        let named = |state: &State| {
            state.list.commands().iter().any(|command| {
                matches!(command, DrawCommand::Text { text, .. } if text.0 as usize == T_MEDALS)
            })
        };
        let viewport = [1_920.0, 1_080.0];
        let glance = State::preview(card.clone(), [900.0, 500.0], viewport, false);
        assert_eq!(icons(&glance), 2);
        assert!(!named(&glance));
        let pinned = State::preview(card, [900.0, 500.0], viewport, true);
        assert_eq!(icons(&pinned), 2);
        assert!(named(&pinned));
    }

    /// A profile with every field the card reads.
    fn full_profile() -> sjk_identity::Profile {
        let achievement = |id: &str, progress, goal, unlocked| sjk_identity::Achievement {
            id: id.to_owned(),
            progress,
            goal,
            unlocked,
            reward: None,
            name: None,
            description: None,
            group: None,
        };
        sjk_identity::Profile {
            tag_cards: Vec::new(),
            credits: 0,
            cards: Vec::new(),
            keys: Vec::new(),
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: "Sol".to_owned(),
            banner: "".into(),
            bio: "\n  Fox of the JoF, maker of SJK.  \nSecond line".to_owned(),
            verified: true,
            tick: sjk_identity::Tick::None,
            staff: true,
            creator: false,
            jkr_creator: false,
            trust: None,
            emoji: String::new(),
            created: 1_791_000_000,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: vec![
                achievement("first_blood", 1, 1, 1_791_100_000),
                achievement("kills_100", 100, 100, 1_791_200_000),
                achievement("kills_1000", 1000, 1000, 1_791_300_000),
                achievement("duel_wins_10", 10, 10, 1_791_400_000),
                achievement("maps_10", 4, 10, 0),
                achievement("unknown_one", 5, 5, 1_791_500_000),
            ],
            unlocks: Vec::new(),
            avatar: "v1".to_owned(),
            holocron_counts: sjk_identity::HolocronCounts {
                uncommon: 4,
                rare: 1,
                legendary: 2,
                mythical: 0,
            },
            holocrons: Vec::new(),
            shards: 0,
        }
    }

    fn full_card(shader: Option<&'static str>) -> Card {
        card_from(
            &info("n|^1Sol^7 the Fox|t|1|model|kyle/default|st|single_1|c1|4|c2|0|"),
            Some(HubInfo {
                name: "Sol".to_owned(),
                verified: true,
                tick: sjk_identity::Tick::None,
                trust: None,
                medals: crate::medals::Medals::from_wire(&[sjk_identity::Medal {
                    id: "bug_hunter".to_owned(),
                    count: 2,
                    awarded: 0,
                    note: String::new(),
                    rank: 1,
                }]),
                key_id: "0123456789abcdef".to_owned(),
                avatar: "v1".to_owned(),
                profile: Some(ProfileFacts::of(&full_profile())),
                vitals: Vitals::SeesAll,
                model: ModelSight::Missing,
                client: None,
            }),
            shader,
        )
    }

    #[test]
    fn the_creator_wears_sjk_creator_in_place_of_sjk_team() {
        let facts = ProfileFacts::of(&sjk_identity::Profile {
            creator: true,
            ..full_profile()
        });
        assert!(facts.creator && facts.staff);
        let card = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                profile: Some(facts),
                ..HubInfo::default()
            }),
            None,
        );
        assert_eq!(card.texts[T_STAFF], "SJK CREATOR");
    }

    #[test]
    fn the_jkr_creator_wears_jkr_creator_without_staff() {
        let facts = ProfileFacts::of(&sjk_identity::Profile {
            staff: false,
            jkr_creator: true,
            ..full_profile()
        });
        assert!(facts.jkr_creator && !facts.staff);
        let card = card_from(
            &info("n|Bishop|"),
            Some(HubInfo {
                profile: Some(facts),
                ..HubInfo::default()
            }),
            None,
        );
        assert_eq!(card.texts[T_JKR], "JKR CREATOR");
        assert!(card.texts[T_STAFF].is_empty());
        assert!(card.has_marks());
    }

    /// A retired achievement counts in a player's total only when they hold it, and a
    /// secret one held shows by its goal like any other.
    #[test]
    fn a_retired_achievement_counts_only_for_its_holders() {
        let mut profile = full_profile();
        let total = crate::achievements::ALL.len();
        assert_eq!(ProfileFacts::of(&profile).total, total - 1);
        for id in ["surveyor", "mythical_find"] {
            profile.achievements.push(sjk_identity::Achievement {
                id: id.to_owned(),
                progress: 5,
                goal: 5,
                unlocked: 1_791_336_225,
                reward: None,
                name: None,
                description: None,
                group: None,
            });
        }
        let facts = ProfileFacts::of(&profile);
        assert_eq!((facts.unlocked, facts.total), (6, total));
        // Surveyor's unfinished progress, not unlocked, counts nothing.
        let mut unfinished = full_profile();
        unfinished.achievements.push(sjk_identity::Achievement {
            id: "surveyor".to_owned(),
            progress: 3,
            goal: 5,
            unlocked: 0,
            reward: None,
            name: None,
            description: None,
            group: None,
        });
        let facts = ProfileFacts::of(&unfinished);
        assert_eq!((facts.unlocked, facts.total), (4, total - 1));
    }

    #[test]
    fn the_profile_gives_the_marks_and_the_pinned_lines() {
        let facts = ProfileFacts::of(&full_profile());
        assert!(facts.staff);
        assert_eq!(facts.bio, "Fox of the JoF, maker of SJK.");
        // Four unlocked the client knows; the unknown one and the unfinished are out.
        assert_eq!(facts.unlocked, 4);
        let rarest: Vec<_> = facts.rarest.iter().map(|kind| kind.id).collect();
        assert_eq!(rarest, ["kills_1000", "kills_100", "duel_wins_10"]);
        assert_eq!(facts.holocron, Some(2), "legendary is the rarest held");
        let card = full_card(None);
        assert_eq!(card.texts[T_STAFF], "SJK TEAM");
        assert_eq!(card.texts[T_HOLOCRON], "Legendary Holocron");
        // Out of the obtainable ones: they do not hold the retired Surveyor.
        assert_eq!(card.texts[T_ACHIEVEMENTS], "Achievements  4 / 77");
        assert_eq!(card.texts[T_BIO], "Fox of the JoF, maker of SJK.");
        assert!(card.texts[T_SINCE].starts_with("In SJK since "));
        assert_eq!(card.texts[T_GOAL], "1K");
        assert_eq!(card.texts[T_VITALS], "Sees your health, shield and Force");
        // An older hub's profile, or none yet: no marks, no lines.
        let bare = ProfileFacts::of(&sjk_identity::Profile {
            staff: false,
            banner: "".into(),
            bio: String::new(),
            created: 0,
            achievements: Vec::new(),
            holocron_counts: sjk_identity::HolocronCounts::default(),
            ..full_profile()
        });
        assert_eq!(
            bare,
            ProfileFacts {
                total: crate::achievements::OBTAINABLE,
                ..ProfileFacts::default()
            }
        );
        let plain = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                profile: Some(bare),
                ..HubInfo::default()
            }),
            None,
        );
        assert!(!plain.has_marks() && plain.profile_lines() == 0);
        assert!(
            plain.texts[T_VITALS].is_empty(),
            "sharing nothing says nothing"
        );
    }

    #[test]
    fn the_card_names_the_worn_shader_with_a_swatch() {
        let card = card_from(&info("n|Sol|st|single_1|c1|4|"), None, Some("saber_glitch"));
        assert_eq!(card.texts[T_SHADER], "Glitch blade");
        assert!(card_from(&info("n|Sol|"), None, Some("saber_nothing")).shaders == [None; 2]);
        assert!(card_from(&info("n|Sol|"), None, None).texts[T_SHADER].is_empty());
        let loaded = crate::saber_skins::tests::loaded_sample(1);
        let sun = card_from(&info("n|Sol|st|single_1|c1|4|"), None, Some("saber_sun"));
        let state = State::preview_with(
            sun,
            [900.0, 500.0],
            [1_920.0, 1_080.0],
            false,
            [loaded.get("saber_sun"), None],
        );
        let commands = state.list.commands();
        let named = commands.iter().any(
            |command| matches!(command, DrawCommand::Text { text, .. } if text.0 as usize == T_SHADER),
        );
        assert!(named);
        // The swatch draws inside an opacity group, many shapes of the skin's look.
        let opened = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::PushOpacity(_)))
            .unwrap();
        let closed = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::PopOpacity))
            .unwrap();
        assert!(closed > opened + 10);
        assert!(commands.len() < LIST_CAPACITY, "the list never fills");
    }

    #[test]
    fn a_second_sabers_own_shader_gets_a_row_of_its_own() {
        let two = info("n|Sol|st|single_1|st2|single_2|c1|4|c2|1|");
        let staff = info("n|Sol|st|dual_1|c1|4|");
        let height = |card: &Card| State::height(card, 1.0, false);
        let one = card_with(&two, None, [Some("saber_sun"), None]);
        assert_eq!(one.shaders, [Some("saber_sun"), None]);
        // The same shader on both: one row, as with one saber.
        let same = card_with(&two, None, [Some("saber_sun"), Some("saber_sun")]);
        assert_eq!(same, one);
        assert_eq!(same, card_from(&two, None, Some("saber_sun")));
        // Their own: a row more, named for the side it goes on.
        let both = card_with(&two, None, [Some("saber_sun"), Some("saber_glitch")]);
        assert_eq!(both.shaders, [Some("saber_sun"), Some("saber_glitch")]);
        assert_eq!(both.texts[T_SHADER2], "Glitch blade  (2nd blade)");
        assert_eq!(height(&both), height(&one) + 24.0);
        let half = card_with(&staff, None, [Some("saber_sun"), Some("saber_glitch")]);
        assert_eq!(half.texts[T_SHADER2], "Glitch blade  (other half)");
        // The first stock: only the second's row.
        let second = card_with(&two, None, [None, Some("saber_glitch")]);
        assert_eq!(second.shaders, [None, Some("saber_glitch")]);
        assert!(second.texts[T_SHADER].is_empty());
        assert_eq!(height(&second), height(&one));
        // An unknown second, or none: no row.
        let unknown = card_with(&two, None, [None, Some("saber_nothing")]);
        assert_eq!(unknown.shaders, [None; 2]);
        assert_eq!(height(&unknown) + 24.0, height(&one));
        // Both drawn, each a live swatch and its name, the list still not full on a
        // pinned card with everything (two chromas, their wheels too).
        let loaded = crate::saber_skins::tests::loaded_pair(1);
        let state = State::preview_with(
            card_with(&two, None, [Some("saber_tidal"), Some("saber_frost")]),
            [900.0, 500.0],
            [1_920.0, 1_080.0],
            false,
            [loaded.get("saber_tidal"), loaded.get("saber_frost")],
        );
        let commands = state.list.commands();
        for slot in [T_SHADER, T_SHADER2] {
            assert!(commands.iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text.0 as usize == slot)
            ));
        }
        let groups = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::PushOpacity(_)))
            .count();
        assert_eq!(groups, 2);
        let mut full = full_card(None);
        full.shaders = [Some("saber_storm"), Some("saber_glitch")];
        full.texts[T_SHADER2] = "Glitch blade  (2nd blade)".to_owned();
        let state = State::preview_with(
            full,
            [900.0, 500.0],
            [1_920.0, 1_080.0],
            true,
            [loaded.get("saber_tidal"), loaded.get("saber_frost")],
        );
        assert!(
            state.list.commands().len() < LIST_CAPACITY,
            "{} commands",
            state.list.commands().len()
        );
    }

    #[test]
    fn a_full_card_stays_inside_the_screen_at_every_size() {
        let loaded = crate::saber_skins::tests::loaded_sample(1);
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_024.0, 768.0],
            [2_560.0, 1_080.0],
            [1_280.0, 720.0],
        ] {
            // The Sun, and the Storm (a chroma, its colour wheel by the swatch).
            for (pinned, shader) in [false, true]
                .into_iter()
                .flat_map(|pinned| [(pinned, "saber_sun"), (pinned, "saber_storm")])
            {
                for anchor in [
                    [viewport[0] * 0.5, viewport[1] * 0.5],
                    [viewport[0] * 0.95, viewport[1] * 0.9],
                    [viewport[0] * 0.05, viewport[1] * 0.1],
                ] {
                    let state = State::preview_with(
                        full_card(Some(shader)),
                        anchor,
                        viewport,
                        pinned,
                        [loaded.get(shader), None],
                    );
                    let card = match state.list.commands()[2] {
                        DrawCommand::RoundedRect { rect, .. } => rect,
                        _ => unreachable!("the card's panel is the third shape"),
                    };
                    assert!(card.x >= 0.0 && card.y >= 0.0, "{viewport:?} {pinned}");
                    assert!(card.x + card.width <= viewport[0] + 0.5, "{viewport:?}");
                    assert!(card.y + card.height <= viewport[1] + 0.5, "{viewport:?}");
                    // Every shape and text of the card's own lies on the panel (the
                    // leader and its dot reach out to the hips).
                    // The colour wheel: the run of arcs from its pure red segment.
                    let commands = &state.list.commands()[2..];
                    let red = commands.iter().position(|command| {
                        matches!(command, DrawCommand::Arc { color, .. }
                            if color.r > 0.99 && color.g < 0.01 && color.b < 0.01)
                    });
                    let mut wheel = None::<Rect>;
                    for command in red.map_or(&[][..], |red| &commands[red..red + 12]) {
                        if let DrawCommand::Arc {
                            center,
                            radius,
                            width,
                            ..
                        } = *command
                        {
                            let reach = radius + width * 0.5;
                            let arc = Rect::new(
                                center[0] - reach,
                                center[1] - reach,
                                reach * 2.0,
                                reach * 2.0,
                            );
                            wheel = Some(wheel.map_or(arc, |w| {
                                let (x, y) = (w.x.min(arc.x), w.y.min(arc.y));
                                Rect::new(
                                    x,
                                    y,
                                    (w.x + w.width).max(arc.x + arc.width) - x,
                                    (w.y + w.height).max(arc.y + arc.height) - y,
                                )
                            }));
                        }
                    }
                    assert_eq!(wheel.is_some(), shader == "saber_storm", "{viewport:?}");
                    for command in &state.list.commands()[2..] {
                        // The shader's name starts past the colour wheel.
                        if let (Some(wheel), DrawCommand::Text { rect, .. }) = (wheel, command)
                            && rect.y < wheel.y + wheel.height
                            && wheel.y < rect.y + rect.height
                            && rect.x < wheel.x + wheel.width
                        {
                            assert!(rect.x + rect.width <= wheel.x, "{command:?} over the wheel");
                        }
                        let rect = match command {
                            DrawCommand::Text { rect, .. }
                            | DrawCommand::SolidRect { rect, .. }
                            | DrawCommand::TexturedQuad { rect, .. } => *rect,
                            _ => continue,
                        };
                        assert!(
                            rect.x >= card.x - 6.0 * viewport[1] / 1_080.0
                                && rect.y >= card.y - 0.5
                                && rect.x + rect.width <= card.x + card.width + 0.5
                                && rect.y + rect.height <= card.y + card.height + 0.5,
                            "{command:?} outside {card:?} at {viewport:?}, pinned {pinned}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_card_stays_beside_the_anchor_and_inside_the_screen() {
        let viewport = [1_920.0, 1_080.0];
        let size = [290.0, 140.0];
        let beside = place([900.0, 500.0], size, 34.0, viewport);
        assert!(beside.right && beside.card.x == 934.0);
        let edge = place([1_800.0, 500.0], size, 34.0, viewport);
        assert!(!edge.right, "no room on the right: it flips to the left");
        assert!(edge.card.x + edge.card.width < 1_800.0);
        let corner = place([10.0, 5.0], size, 34.0, viewport);
        assert!(corner.card.y >= 0.0 && corner.card.x >= 0.0);
        let low = place([900.0, 1_075.0], size, 34.0, viewport);
        assert!(low.card.y + low.card.height <= viewport[1]);
    }

    /// `cg_playerCard` is three-way (Blep's idea, 29/10/2026): 0 off, 1 a steady look or
    /// `inspect` (the default), 2 `inspect` only. The old on/off setting saved 0 and 1,
    /// which read as before.
    #[test]
    fn the_setting_reads_three_ways_and_old_configs_keep_their_meaning() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut state = State::default();
        state.sample(Some(&console));
        assert_eq!(state.opens, Opens::LookAndKey, "on by default, as before");
        for (value, opens) in [
            // `seta cg_playerCard "0"` and `"1"`, as the old on/off setting saved them.
            ("0", Opens::Off),
            ("1", Opens::LookAndKey),
            ("2", Opens::KeyOnly),
            ("5", Opens::KeyOnly),
            ("-1", Opens::Off),
        ] {
            assert!(console.set_cvar(CVAR, value), "{value}");
            state.sample(Some(&console));
            assert_eq!(state.opens, opens, "{value}");
        }
        assert_eq!(Opens::of(None), Opens::LookAndKey);
        assert!(Opens::LookAndKey.by_look() && Opens::LookAndKey.by_key());
        assert!(!Opens::KeyOnly.by_look() && Opens::KeyOnly.by_key());
        assert!(!Opens::Off.by_look() && !Opens::Off.by_key());
    }

    /// Frames 50 ms apart from `start` to `end` with `aimed` under the crosshair and `pin`
    /// pinned: whose card the last one wants.
    fn looking(
        state: &mut State,
        aimed: Option<u16>,
        pin: Option<u16>,
        start: i32,
        end: i32,
    ) -> (Option<u16>, Option<u16>) {
        let mut chosen = (None, None);
        for now in (start..=end).step_by(50) {
            chosen = state.choose(aimed, pin, false, Vec3::X, now);
        }
        chosen
    }

    /// 1 (and the default) shows a card after a steady look and on `inspect`; Key only (2)
    /// shows it on `inspect` alone, however long the look; 0 shows none, pinned or not.
    #[test]
    fn each_mode_opens_the_card_its_own_way() {
        let delay = State::default().delay_ms;
        let mut look = State::default();
        assert_eq!(
            looking(&mut look, Some(4), None, 0, delay - 100),
            (None, None)
        );
        assert_eq!(
            looking(&mut look, Some(4), None, delay, delay + 100).1,
            Some(4)
        );
        assert_eq!(
            looking(&mut look, None, Some(7), 5_000, 5_000),
            (Some(7), Some(7)),
            "pinned"
        );
        let mut key = State {
            opens: Opens::KeyOnly,
            ..State::default()
        };
        assert_eq!(
            looking(&mut key, Some(4), None, 0, 10 * delay),
            (None, None),
            "a long steady look opens nothing"
        );
        assert_eq!(
            looking(&mut key, Some(4), Some(4), 10 * delay, 10 * delay),
            (Some(4), Some(4)),
            "inspect does"
        );
        // Escape unpins it in Key only too, and goes to the menu once nothing is pinned.
        key.pin.request();
        key.pin.resolve(Some(4), true, false);
        assert!(key.cancel());
        let mut off = State {
            opens: Opens::Off,
            ..State::default()
        };
        assert_eq!(
            looking(&mut off, Some(4), Some(4), 0, 10 * delay),
            (None, None)
        );
        off.pin.request();
        off.pin.resolve(Some(4), true, false);
        assert!(!off.cancel(), "off: Escape opens the game menu");
        // A hidden card (a menu, the scoreboard) shows in no mode.
        for opens in [Opens::LookAndKey, Opens::KeyOnly] {
            let mut state = State {
                opens,
                ..State::default()
            };
            assert_eq!(state.choose(Some(4), Some(4), true, Vec3::X, 0).1, None);
        }
    }

    #[test]
    fn inspect_pins_the_player_under_the_crosshair_and_a_second_press_unpins() {
        let mut pin = Pin::default();
        // Nothing under the crosshair: the press pins nothing.
        pin.request();
        assert_eq!(pin.resolve(None, true, false), PinStep::default());
        pin.request();
        let step = pin.resolve(Some(4), true, false);
        assert_eq!(step.target, Some(4));
        // Looking away or at someone else does not move it.
        assert_eq!(pin.resolve(None, true, false).target, Some(4));
        assert_eq!(pin.resolve(Some(9), true, false).target, Some(4));
        pin.request();
        let step = pin.resolve(Some(9), true, false);
        assert_eq!((step.target, step.released), (None, Some(4)));
    }

    #[test]
    fn a_pinned_player_out_of_sight_hides_then_drops() {
        let mut sight = Sight::default();
        assert!(!sight.observe(None, 0) && !sight.hidden(), "nothing pinned");
        assert!(!sight.observe(Some(true), 100) && !sight.hidden());
        // Behind a wall: hidden at once, still pinned until LOST_MS passes.
        assert!(!sight.observe(Some(false), 1_000) && sight.hidden());
        assert!(!sight.observe(Some(false), 1_000 + LOST_MS));
        // Back in sight before then: shown again, and the wait starts over.
        assert!(!sight.observe(Some(true), 2_000) && !sight.hidden());
        assert!(!sight.observe(Some(false), 3_000));
        assert!(
            sight.observe(Some(false), 3_001 + LOST_MS),
            "out of sight too long"
        );
    }

    #[test]
    fn escape_unpins_once_and_only_a_pinned_card() {
        let mut pin = Pin::default();
        assert!(!pin.cancel(), "nothing pinned: Escape goes on to the menu");
        pin.request();
        pin.resolve(Some(4), true, false);
        assert!(pin.cancel());
        assert!(
            !pin.cancel(),
            "a second Escape before the frame is not swallowed"
        );
        let step = pin.resolve(Some(4), true, false);
        assert_eq!((step.target, step.released), (None, Some(4)));
        assert!(!pin.cancel());
    }

    #[test]
    fn the_pin_drops_when_the_player_leaves_or_the_map_changes() {
        let mut pin = Pin::default();
        pin.request();
        pin.resolve(Some(4), true, false);
        let step = pin.resolve(None, false, false);
        assert_eq!((step.target, step.released), (None, Some(4)));
        pin.request();
        pin.resolve(Some(5), true, false);
        let step = pin.resolve(Some(5), true, true);
        assert_eq!((step.target, step.released), (None, Some(5)));
    }

    #[test]
    fn the_card_reads_the_hat_and_cape_after_the_colour_digits() {
        let both = card_from(&info("n|Sol|c1|4santa_hat|c2|0red-cape|"), None, None);
        assert_eq!(both.texts[T_WORN], "Hat Santa hat  /  Cape Red cape");
        let hat = card_from(&info("n|Sol|c1|4santa_hat|c2|3|"), None, None);
        assert_eq!(hat.texts[T_WORN], "Hat Santa hat");
        let cape = card_from(&info("n|Sol|c1|4|c2|2jedi|"), None, None);
        assert_eq!(cape.texts[T_WORN], "Cape Jedi");
        let none = card_from(&info("n|Sol|c1|4|c2|0|"), None, None);
        assert!(none.texts[T_WORN].is_empty() && !none.has_worn());
    }

    fn camera() -> Camera {
        Camera {
            eye: Vec3::ZERO,
            target: Vec3::X,
            up: Vec3::Z,
            fov: 90.0,
            viewport: [1_920.0, 1_080.0],
        }
    }

    #[test]
    fn the_outline_area_holds_the_body_and_shrinks_with_range() {
        let near = body_area(camera(), Vec3::new(150.0, 0.0, 0.0), 40.0);
        let far = body_area(camera(), Vec3::new(600.0, 0.0, 0.0), 40.0);
        for [x, y, width, height] in [near, far] {
            // Around the screen centre, where the player stands.
            assert!(x < 960.0 && x + width > 960.0 && y < 540.0 && y + height > 540.0);
        }
        assert!(far[2] < near[2] && far[3] < near[3]);
        // Standing over the eye: part of the reach is behind it, so the whole screen.
        assert_eq!(
            body_area(camera(), Vec3::new(10.0, 0.0, 0.0), 40.0),
            [0.0, 0.0, 1_920.0, 1_080.0]
        );
    }

    #[test]
    fn the_anchor_follows_the_player_and_clears_the_body_at_any_range() {
        let near = anchor_on_screen(camera(), Vec3::new(100.0, 0.0, 0.0), 1.2).unwrap();
        let far = anchor_on_screen(camera(), Vec3::new(400.0, 0.0, 0.0), 1.2).unwrap();
        assert!((near.0[0] - 960.0).abs() < 1.0 && (near.0[1] - 540.0).abs() < 1.0);
        assert!(near.1 > far.1 * 3.5, "a nearer body is wider on screen");
        // Higher in the world is higher on screen.
        let above = anchor_on_screen(camera(), Vec3::new(100.0, 0.0, 30.0), 1.2).unwrap();
        assert!(above.0[1] < near.0[1]);
        // Behind the camera there is no anchor, however far the limit.
        assert!(anchor_on_screen(camera(), Vec3::new(-100.0, 0.0, 0.0), 40.0).is_none());
    }

    #[test]
    fn a_pinned_player_far_off_screen_still_gets_a_card_kept_on_screen() {
        let viewport = [1_920.0, 1_080.0];
        let off = Vec3::new(100.0, 900.0, 0.0);
        assert!(anchor_on_screen(camera(), off, 1.2).is_none());
        let (point, side) = anchor_on_screen(camera(), off, PINNED_LIMIT).unwrap();
        let placed = place(point, [290.0, 140.0], side + 20.0, viewport);
        let card = placed.card;
        assert!(card.x >= 0.0 && card.x + card.width <= viewport[0]);
        assert!(card.y >= 0.0 && card.y + card.height <= viewport[1]);
    }
}
