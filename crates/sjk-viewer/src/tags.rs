//! Tags, SJK's toy (`toy_tag`; Sol's idea, 27/10/2026): a player sticks a premade
//! picture on the wall or floor they look at, as TF2's sprays, and every SJK player on
//! the same map sees it there (`docs/client.md`, Tag).
//!
//! - **The tags** are unlocks of the hub (`PROTOCOL.md`, "Unlocks"): [`CATALOGUE`] here,
//!   the same list there. Three are free to everyone ([`Tag::free`]); the others drop
//!   from holocrons, each with a tier as shaders have. Sol picks every one: there are
//!   no uploads (his decision, 27/10/2026); new tags come from rounds on Discord.
//! - **Choosing:** [`CVAR`] holds the chosen tag (the Collection's Tags tab sets it);
//!   one the player does not hold is not stuck.
//! - **Sticking:** [`COMMAND`] traces the crosshair [`REACH`] units into the world and
//!   sticks the chosen tag where it lands, upright on a wall and facing the player on a
//!   floor ([`spin_for`]); `toy_tag clear` takes it off. One tag a player: a new one
//!   replaces the last. At most one every [`GAP`].
//! - **Others:** the tag travels in the player's look (`looks.rs`, `PROTOCOL.md`,
//!   "Looks"), so players who come later see it too. It stays until the player sticks
//!   another, clears it, changes map or server, or leaves. [`SHOW_CVAR`] 0 hides other
//!   players' tags on this PC; a muted or blocked player's never show.
//!
//! - **Finishes and cards** (31/10/2026, Sol's decisions): a tag a holocron drops comes
//!   Holo now and then and Solar rarely, and a tag is stuck in a finish the player holds a
//!   copy in ([`FINISH_CVAR`], [`Held`]); the credits cards they hold stick as tags too,
//!   their picture as a sticker ([`is_card_id`]). Both need a hub that lists `tag_cards`
//!   in profiles ([`Held::of`]); an older hub refuses them, so they are not offered there.
//!
//! - **Collections** (08/11/2026, Sol's decision): every tag belongs to one of
//!   [`COLLECTIONS`], Alpha for SJK's own and Community for those players drew (picked
//!   from #tag-proposals). A collection is a label: the Tags tab lists each under its own
//!   heading and the tag's card names it; it changes nothing in drops (unlike a
//!   blade's edition, `editions.rs`), so the hub does not know it.
//!
//! - **GIF tags** (10/11/2026, Sol's decisions): a blank tag ([`BLANK`], dropped as the
//!   others) is spent once on a GIPHY GIF of the player's choosing, a GIF tag for good,
//!   stuck as `gif:<id>` ([`gif_id`]) and played on the wall. Verified players only (the
//!   hub's rule); a GIF GIPHY removes is lost with its tag. Opt-in: [`GIF_SHOW_CVAR`] is
//!   off by default and, off, no GIF tag is fetched, drawn or stuck on this PC; a GIF the
//!   player hid in chat never shows on a wall either (`chat_gifs::wall`).
//!
//! The pictures are art in the hub's `sjk_tags` pack (`gfx/tags/<id>.png`) and, for a
//! card, its picture in the card packs; the client projects a tag onto the world as a
//! lasting decal of its own (`decal_store.rs`, [`crate::decal_store::DecalStore::set_tag`])
//! and draws it at the picture's full size with its finish (`tag_render.rs`).

use crate::unlockables::Rarity;
use glam::Vec3;
use std::time::{Duration, Instant};

/// The toy's console command: stick the chosen tag, or `toy_tag clear` take it off.
pub(crate) const COMMAND: &str = "toy_tag";
/// Archived: the chosen tag's id, or a card's.
pub(crate) const CVAR: &str = "cg_tag";
/// Archived: the finish the chosen tag is stuck in, `normal`, `holo` or `solar`
/// (31/10/2026): one the player holds no copy in sticks Normal.
pub(crate) const FINISH_CVAR: &str = "cg_tagFinish";
/// The tag chosen before the player chose one.
pub(crate) const DEFAULT: &str = "tag_sjk";
/// Archived: other players' tags are drawn (0 hides them on this PC; the own still
/// shows and is still sent).
pub(crate) const SHOW_CVAR: &str = "cg_toyTags";
/// Archived, off by default (10/11/2026, Sol: opt-in): GIF tags are fetched from GIPHY
/// and drawn, the own and other players'; off, none is, and none is stuck.
pub(crate) const GIF_SHOW_CVAR: &str = "cg_toyGifTags";
/// The blank tag: spent on a GIPHY GIF it becomes a GIF tag; never stuck itself.
pub(crate) const BLANK: &str = "tag_blank";
/// What a GIF tag's id begins with in looks and `cg_tag`: `gif:` and the GIPHY id.
pub(crate) const GIF_PREFIX: &str = "gif:";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_tag.png";
/// How far the crosshair reaches for a wall, in units.
pub(crate) const REACH: f32 = 128.0;

/// How far a tag's trace from `start` goes: [`REACH`] past the player's `eye`. In third
/// person without the dynamic crosshair (`cg_dynamicCrosshair 0`) the trace starts at the
/// camera, some 80 units behind the player, and 128 from there seldom reached a wall
/// (Kushina: tags did not work from her bind or the wheel). At most [`CAMERA_SLACK`] is
/// added, so a camera flown away (free camera, drone) does not tag from afar.
pub(crate) fn reach_from(start: glam::Vec3, eye: Option<glam::Vec3>) -> f32 {
    REACH + eye.map_or(0.0, |eye| start.distance(eye).min(CAMERA_SLACK))
}

/// The most a camera behind the player adds to [`REACH`]: past the longest third-person
/// range players use.
const CAMERA_SLACK: f32 = 256.0;
/// Half a tag's width, in units: a tag is a bit less than a player is tall.
pub(crate) const RADIUS: f32 = 28.0;
/// Shortest time between two tags stuck.
pub(crate) const GAP: Duration = Duration::from_secs(10);

/// The collection of SJK's own tags, the first.
pub(crate) const ALPHA: &str = "alpha";
/// The collection of tags players drew, picked from SJK's Discord (#tag-proposals).
pub(crate) const COMMUNITY: &str = "community";

/// One collection of tags: its id and how players read it.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Collection {
    pub(crate) id: &'static str,
    /// Its name: "Alpha".
    pub(crate) name: &'static str,
    /// Its heading on the Tags tab's wall: "Alpha collection".
    pub(crate) heading: &'static str,
    /// What it holds, in a line for the tag's card.
    pub(crate) about: &'static str,
}

/// Every collection, in the order the Tags tab lists them. A new one is a line here and
/// the tags put in it name its id.
pub(crate) const COLLECTIONS: [Collection; 2] = [
    Collection {
        id: ALPHA,
        name: "Alpha",
        heading: "Alpha collection",
        about: "Alpha collection: SJK's own tags.",
    },
    Collection {
        id: COMMUNITY,
        name: "Community",
        heading: "Community collection",
        about: "Community collection: drawn by a player, picked from SJK's Discord.",
    },
];

/// The collection `id` names; one the client does not know is the first.
pub(crate) fn collection(id: &str) -> &'static Collection {
    COLLECTIONS
        .iter()
        .find(|collection| collection.id == id)
        .unwrap_or(&COLLECTIONS[0])
}

/// One tag of the catalogue.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Tag {
    /// Its unlock id at the hub and in looks.
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) tier: Rarity,
    /// Every player holds it, without a grant (the starter set).
    pub(crate) free: bool,
    /// The id of the collection it belongs to ([`COLLECTIONS`]).
    pub(crate) collection: &'static str,
}

impl Tag {
    /// Its picture in the hub's `sjk_tags` pack (`gfx/tags/<id>.png`).
    pub(crate) fn picture(&self) -> String {
        format!("gfx/tags/{}.png", self.id)
    }

    /// Its collection.
    pub(crate) fn collection(&self) -> &'static Collection {
        collection(self.collection)
    }

    /// How a player gets it, in a line.
    pub(crate) fn how_to_get(&self) -> &'static str {
        if self.free {
            "Everyone's, from the start."
        } else {
            crate::unlockables::FROM_HOLOCRONS
        }
    }
}

const fn tag(id: &'static str, name: &'static str, tier: Rarity, free: bool) -> Tag {
    Tag {
        id,
        name,
        tier,
        free,
        collection: ALPHA,
    }
}

/// A tag of the Community collection: a player's art (they are credited in the pack).
const fn community(id: &'static str, name: &'static str, tier: Rarity) -> Tag {
    Tag {
        collection: COMMUNITY,
        ..tag(id, name, tier, false)
    }
}

/// Every tag, in the hub's order (`src/unlocks.rs`). Ids never change meaning; a new
/// tag (one Sol picked from a Discord round) takes a new id here and at the hub, and its
/// picture goes in the pack.
pub(crate) const CATALOGUE: &[Tag] = &[
    tag("tag_sjk", "SJK tag", Rarity::Common, true),
    tag("tag_gg", "GG tag", Rarity::Common, true),
    tag("tag_fox", "Fox tag", Rarity::Common, true),
    tag(
        "tag_crossed_sabers",
        "Crossed Sabers tag",
        Rarity::Common,
        false,
    ),
    tag("tag_was_here", "Was Here tag", Rarity::Common, false),
    tag("tag_this_way", "This Way tag", Rarity::Common, false),
    tag("tag_kyber", "Kyber tag", Rarity::Uncommon, false),
    tag(
        "tag_droid_doodle",
        "Droid Doodle tag",
        Rarity::Uncommon,
        false,
    ),
    tag("tag_rage_quit", "Rage Quit tag", Rarity::Uncommon, false),
    community("tag_gonk", "Gonk tag", Rarity::Uncommon),
    tag("tag_holocron", "Holocron tag", Rarity::Rare, false),
    community("tag_ez", "EZ tag", Rarity::Rare),
    tag("tag_saber_skull", "Saber Skull tag", Rarity::Rare, false),
    community("tag_geyon", "Geyon tag", Rarity::Rare),
    tag(BLANK, "Blank tag", Rarity::Rare, false),
    tag("tag_force_push", "Force Push tag", Rarity::Legendary, false),
    tag("tag_dark_mask", "Dark Mask tag", Rarity::Legendary, false),
    tag("tag_solar_fox", "Solar Fox tag", Rarity::Mythical, false),
];

/// The GIPHY GIF of GIF tag `id` (`gif:<giphy id>`), if it is one.
pub(crate) fn gif_id(id: &str) -> Option<crate::chat_gifs::GifId> {
    crate::chat_gifs::GifId::new(id.strip_prefix(GIF_PREFIX)?)
}

/// GIF tag id of GIPHY GIF `giphy`.
pub(crate) fn gif_tag_id(giphy: &str) -> String {
    format!("{GIF_PREFIX}{giphy}")
}

/// The GIF tags of profile `me`, newest first; `None` on a hub before GIF tags.
pub(crate) fn gif_tags(me: Option<&sjk_identity::Profile>) -> Option<&[sjk_identity::GifTag]> {
    me?.gif_tags.as_deref()
}

/// The tag called `id`, if this client knows it.
pub(crate) fn get(id: &str) -> Option<&'static Tag> {
    CATALOGUE.iter().find(|tag| tag.id == id)
}

/// Whether `id` names a card rather than a tag (the hub's rule, `PROTOCOL.md`, "Looks"):
/// lowercase letters, digits and `_`, ending in `_` and three digits (`sjk_alpha_003`),
/// and no tag of the catalogue.
pub(crate) fn is_card_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (5..=48).contains(&bytes.len())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
        && bytes[bytes.len() - 4] == b'_'
        && bytes[bytes.len() - 3..].iter().all(u8::is_ascii_digit)
        && get(id).is_none()
}

/// The finish a placed tag shows: its own, Normal when it has none or one this client
/// does not know.
pub(crate) fn finish_of(tag: &sjk_identity::PlacedTag) -> &'static crate::cards::Finish {
    tag.finish
        .as_deref()
        .and_then(crate::cards::Finish::from_id)
        .unwrap_or(&crate::cards::FINISHES[0])
}

/// The copies of a tag (or a card) the player holds, by finish: any copy sticks it Normal,
/// a Holo or Solar one in its finish too.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Held {
    pub(crate) copies: u32,
    pub(crate) holo: u32,
    pub(crate) solar: u32,
}

impl Held {
    /// Every finish, in a local game, which has no hub.
    pub(crate) const LOCAL: Self = Self {
        copies: 1,
        holo: 1,
        solar: 1,
    };

    /// What the own hub profile `me` (`None`: not known) holds of tag or card `id`: a free
    /// tag one Normal copy; a dropped tag the profile's copies, counted by finish; a card
    /// its revealed copies, while the hub lets cards of its kind be tags (the profile's
    /// `tag_cards`, [`card_kind_allowed`]). Nothing for an id this client does not know.
    pub(crate) fn of(id: &str, me: Option<&sjk_identity::Profile>) -> Self {
        Self::of_parts(id, me.map(|me| me.unlocks.as_slice()), me)
    }

    /// [`Self::of`] with the profile's unlocks given apart (`unlocks`, as the Collection's
    /// holdings list them).
    pub(crate) fn of_parts(
        id: &str,
        unlocks: Option<&[sjk_identity::Unlock]>,
        me: Option<&sjk_identity::Profile>,
    ) -> Self {
        if let Some(giphy) = id.strip_prefix(GIF_PREFIX) {
            let held = gif_tags(me).is_some_and(|tags| tags.iter().any(|tag| tag.id == giphy));
            return Self {
                copies: u32::from(held),
                ..Self::default()
            };
        }
        if let Some(tag) = get(id) {
            if tag.free {
                return Self {
                    copies: 1,
                    ..Self::default()
                };
            }
            return unlocks
                .and_then(|unlocks| unlocks.iter().find(|unlock| unlock.id == id))
                .map_or_else(Self::default, |unlock| Self {
                    copies: unlock.count.max(1),
                    holo: unlock.holo,
                    solar: unlock.solar,
                });
        }
        let Some(me) = me.filter(|me| card_kind_allowed(id, me)) else {
            return Self::default();
        };
        let mut held = Self::default();
        for stack in me
            .cards
            .iter()
            .filter(|stack| stack.card.as_deref() == Some(id))
        {
            held.copies += stack.count;
            match stack.finish.as_str() {
                "holo" => held.holo += stack.count,
                "solar" => held.solar += stack.count,
                _ => {}
            }
        }
        held
    }

    /// Whether it sticks in `finish`.
    pub(crate) fn has(&self, finish: &crate::cards::Finish) -> bool {
        match finish.id {
            "holo" => self.holo > 0,
            "solar" => self.solar > 0,
            _ => self.copies > 0,
        }
    }

    /// The finish to stick it in: `wanted` when held, else Normal.
    pub(crate) fn finish(&self, wanted: &str) -> &'static crate::cards::Finish {
        crate::cards::Finish::from_id(wanted)
            .filter(|finish| self.has(finish))
            .unwrap_or(&crate::cards::FINISHES[0])
    }
}

/// Whether the hub of profile `me` lets card `id` be a tag: its face's kind is one the
/// profile's `tag_cards` lists (the credits cards, 31/10/2026). False for a card this
/// client has no face of, and on a hub before card tags.
pub(crate) fn card_kind_allowed(id: &str, me: &sjk_identity::Profile) -> bool {
    !me.tag_cards.is_empty()
        && crate::cards::faces::current()
            .find(id)
            .is_some_and(|(_, face)| me.tag_cards.iter().any(|kind| kind == face.kind.id()))
}

/// The cards the hub of profile `me` lets be tags, in the faces' order (their sets and
/// numbers): every face of a kind its `tag_cards` lists, held or not. Empty on a hub
/// before card tags.
pub(crate) fn tag_cards(me: Option<&sjk_identity::Profile>) -> Vec<String> {
    let Some(me) = me.filter(|me| !me.tag_cards.is_empty()) else {
        return Vec::new();
    };
    crate::cards::faces::current()
        .list
        .iter()
        .filter(|face| me.tag_cards.iter().any(|kind| kind == face.kind.id()))
        .map(|face| face.id.clone())
        .collect()
}

/// Whether the player may stick tag or card `id` in `finish`, holding `held` of it.
pub(crate) fn may_stick(held: Held, finish: &crate::cards::Finish) -> bool {
    held.has(finish)
}

/// Whether `id` can be stuck at all: a tag of the catalogue but the blank, a card, or a
/// GIF tag.
pub(crate) fn stickable(id: &str) -> bool {
    id != BLANK && (get(id).is_some() || is_card_id(id) || gif_id(id).is_some())
}

/// The turn about `normal`, in degrees from 0 up to 360, that stands a tag's top along
/// `up` (a direction in its plane): on a wall the world's up, on a floor or a ceiling
/// the way the player looks ([`up_for`]). Every client lays the picture out from the
/// normal and this turn alone (`decal_marks::texture_axes`), so all see it the same.
pub(crate) fn spin_for(normal: Vec3, up: Vec3) -> f32 {
    let normal = normal.normalize_or(Vec3::Z);
    let seed = crate::decal_marks::texture_axes(normal, 0.0)[2];
    // The picture's rows run down its texture's `t`, the third axis: its top is that
    // axis turned round.
    let want = -(up - normal * up.dot(normal)).normalize_or(seed);
    let angle = normal
        .dot(seed.cross(want))
        .atan2(seed.dot(want))
        .to_degrees();
    let spin = angle.rem_euclid(360.0);
    if spin >= 360.0 { 0.0 } else { spin }
}

/// The way a tag's top points on a surface facing `normal`, for a player looking along
/// `forward`: the world's up on a wall, the look's way on a floor or a ceiling.
pub(crate) fn up_for(normal: Vec3, forward: Vec3) -> Vec3 {
    if normal.z.abs() < 0.7 {
        Vec3::Z
    } else {
        forward
    }
}

/// The spray can's sound when a tag goes up (`scripts/tag_sounds.py`).
pub(crate) const SPRAY_SOUND: &str = "sound/sjk/tags/spray.wav";

/// The player's own tag and when the last one went up.
#[derive(Default)]
pub(crate) struct Tags {
    /// Where a tag went up this frame: the spray is heard there.
    pub(crate) sprays: Vec<Vec3>,
    /// Each other player's tag as last seen, `None` before their look was known: a
    /// player whose look arrives with a tag is not heard spraying it.
    pub(crate) seen: Vec<Option<Option<sjk_identity::PlacedTag>>>,
    /// The audio generation the spray was registered with.
    pub(crate) sound_registered: Option<u64>,
    /// Sprays played so far: each takes the next of a few sources, so two at once do not
    /// cut each other.
    pub(crate) sprays_played: u32,
    /// The tag stuck, with the server it is on.
    own: Option<(std::net::SocketAddr, sjk_identity::PlacedTag)>,
    last: Option<Instant>,
    /// The own tag changed since the look last went: send it now.
    changed: bool,
    /// The decal store was handed tags in a game: leaving it clears them once.
    pub(crate) in_game: bool,
}

impl Tags {
    /// Whether one may go up at `now`; `Err` the time still to wait.
    pub(crate) fn may_stick(&self, now: Instant) -> Result<(), Duration> {
        match self.last {
            Some(last) if now.saturating_duration_since(last) < GAP => {
                Err(GAP - now.saturating_duration_since(last))
            }
            _ => Ok(()),
        }
    }

    /// Stick `tag` on `server`, replacing the last.
    pub(crate) fn stick(
        &mut self,
        server: std::net::SocketAddr,
        tag: sjk_identity::PlacedTag,
        now: Instant,
    ) {
        self.own = Some((server, tag));
        self.last = Some(now);
        self.changed = true;
    }

    /// Take the own tag off; true when there was one.
    pub(crate) fn clear(&mut self) -> bool {
        let had = self.own.take().is_some();
        self.changed |= had;
        had
    }

    /// Whether the own tag changed since last asked.
    pub(crate) fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// The own tag while it stands on `server` and `map` and the player still may stick
    /// it in its finish (`held`): any other, it goes (a tag taken back, another server or
    /// map).
    pub(crate) fn own(
        &mut self,
        server: Option<std::net::SocketAddr>,
        map: Option<&str>,
        held: impl Fn(&str) -> Held,
    ) -> Option<&sjk_identity::PlacedTag> {
        let stands = self.own.as_ref().is_some_and(|(on, tag)| {
            Some(*on) == server
                && Some(tag.map.as_str()) == map
                && may_stick(held(&tag.id), finish_of(tag))
        });
        if !stands {
            self.own = None;
        }
        self.own.as_ref().map(|(_, tag)| tag)
    }
}

/// Mount the toy's icon.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory(
        "SJK Tag",
        [
            (
                ICON,
                include_bytes!("../assets/tags/toy_tag.png").as_slice(),
            ),
            (
                SPRAY_SOUND,
                include_bytes!("../assets/tags/spray.wav").as_slice(),
            ),
        ],
    )?;
    Ok(())
}

/// Where a placed tag lies on the world, when this client knows the tag or it names a
/// card (`tag_render.rs` draws the picture; a card whose face is missing draws nothing).
pub(crate) fn decal(tag: &sjk_identity::PlacedTag) -> Option<crate::decal_marks::DecalRequest> {
    stickable(&tag.id).then(|| crate::decal_marks::DecalRequest {
        origin: Vec3::from_array(tag.at),
        direction: Vec3::from_array(tag.normal),
        orientation: tag.spin,
        color: [1.0; 4],
        radius: RADIUS,
        shader: std::sync::Arc::from(""),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every tag names a collection the table has; Community holds the players' tags
    /// (Blep's EZ, Gonk and Geyon, 30/10/2026) and Alpha the rest.
    #[test]
    fn every_tag_is_in_a_known_collection() {
        for tag in CATALOGUE {
            assert!(
                COLLECTIONS.iter().any(|known| known.id == tag.collection),
                "{}",
                tag.id
            );
        }
        let community: Vec<_> = CATALOGUE
            .iter()
            .filter(|tag| tag.collection == COMMUNITY)
            .map(|tag| tag.id)
            .collect();
        assert_eq!(community, ["tag_gonk", "tag_ez", "tag_geyon"]);
        assert_eq!(get("tag_sjk").unwrap().collection().name, "Alpha");
        assert_eq!(collection("nope").id, ALPHA);
    }

    /// From the eye (first person, the dynamic crosshair) the reach is 128; from a
    /// third-person camera 80 behind, 208.
    #[test]
    fn the_reach_counts_from_the_eye() {
        let eye = glam::Vec3::new(0.0, 0.0, 64.0);
        assert_eq!(reach_from(eye, Some(eye)), REACH);
        assert_eq!(
            reach_from(eye - glam::Vec3::X * 80.0, Some(eye)),
            REACH + 80.0
        );
        assert_eq!(reach_from(eye, None), REACH);
        assert_eq!(
            reach_from(eye - glam::Vec3::X * 4_000.0, Some(eye)),
            REACH + CAMERA_SLACK
        );
    }

    #[test]
    fn ids_are_unique_tag_words_and_three_are_free() {
        for (index, tag) in CATALOGUE.iter().enumerate() {
            assert!(tag.id.starts_with("tag_"), "{}", tag.id);
            assert!(tag.id.len() <= 24, "{}", tag.id);
            assert_eq!(get(tag.id), Some(tag));
            assert!(
                CATALOGUE[..index].iter().all(|other| other.id != tag.id),
                "{}",
                tag.id
            );
        }
        let free: Vec<_> = CATALOGUE
            .iter()
            .filter(|tag| tag.free)
            .map(|tag| tag.id)
            .collect();
        assert_eq!(free, ["tag_sjk", "tag_gg", "tag_fox"]);
        assert!(get(DEFAULT).is_some_and(|tag| tag.free));
    }

    /// A profile holding `unlocks` and `cards` (card, finish, count), on a hub that
    /// lists `tag_cards`.
    fn profile(
        unlocks: &[(&str, u32, u32, u32)],
        cards: &[(&str, &str, u32)],
        tag_cards: &[&str],
    ) -> sjk_identity::Profile {
        let mut me: sjk_identity::Profile = serde_json::from_value(serde_json::json!({
            "key_id": "0123456789abcdef", "key": "", "name": "Sol", "bio": "",
            "verified": false, "created": 0, "names": [], "medals": [], "unlocks": []
        }))
        .unwrap();
        me.unlocks = unlocks
            .iter()
            .map(|(id, count, holo, solar)| {
                serde_json::from_value(serde_json::json!({"id": id, "count": count,
                    "holo": holo, "solar": solar}))
                .unwrap()
            })
            .collect();
        me.cards = cards
            .iter()
            .map(|(card, finish, count)| {
                serde_json::from_value(serde_json::json!({"card": card, "rarity": "mythical",
                    "finish": finish, "count": count}))
                .unwrap()
            })
            .collect();
        me.tag_cards = tag_cards.iter().map(|kind| (*kind).to_owned()).collect();
        me
    }

    /// GIF tags (10/11/2026): `gif:` and a GIPHY id, held when the profile lists the GIF,
    /// never on a hub before them; the blank tag is held as a tag but never stuck.
    #[test]
    fn a_gif_tag_is_held_from_the_profile_and_the_blank_never_sticks() {
        let normal = &crate::cards::FINISHES[0];
        assert_eq!(gif_id("gif:TestGif1").unwrap().as_str(), "TestGif1");
        assert_eq!(gif_tag_id("TestGif1"), "gif:TestGif1");
        for not in [
            "TestGif1",
            "gif:",
            "gif:birthday",
            "gif:Te",
            "GIF:TestGif1",
            "tag_gg",
        ] {
            assert!(gif_id(not).is_none(), "{not}");
        }
        assert!(!is_card_id("gif:TestGif1"));
        assert!(stickable("gif:TestGif1"));
        assert!(stickable("tag_gg") && stickable("sjk_alpha_001"));
        assert!(!stickable(BLANK) && !stickable("tag_nope") && !stickable("gif:x"));
        let mut me = profile(&[(BLANK, 2, 0, 0)], &[], &[]);
        assert_eq!(gif_tags(Some(&me)), None, "a hub before GIF tags");
        assert!(!may_stick(Held::of("gif:TestGif1", Some(&me)), normal));
        assert_eq!(Held::of(BLANK, Some(&me)).copies, 2);
        me.gif_tags = Some(vec![sjk_identity::GifTag {
            id: "TestGif1".to_owned(),
            made: 1,
        }]);
        let held = Held::of("gif:TestGif1", Some(&me));
        assert!(may_stick(held, normal));
        assert!(
            !held.has(&crate::cards::FINISHES[1]),
            "a GIF tag has no finish"
        );
        assert!(!may_stick(Held::of("gif:OtherGif2", Some(&me)), normal));
        // The own GIF tag stands while it is held.
        let mut tags = Tags::default();
        let server: std::net::SocketAddr = "127.0.0.1:29070".parse().unwrap();
        let tag = sjk_identity::PlacedTag {
            id: "gif:TestGif1".to_owned(),
            map: "mp/ffa3".to_owned(),
            at: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            spin: 0.0,
            finish: None,
        };
        assert!(decal(&tag).is_some());
        tags.stick(server, tag, Instant::now());
        assert!(
            tags.own(Some(server), Some("mp/ffa3"), |id| Held::of(id, Some(&me)))
                .is_some()
        );
        me.gif_tags = Some(Vec::new());
        assert!(
            tags.own(Some(server), Some("mp/ffa3"), |id| Held::of(id, Some(&me)))
                .is_none(),
            "a GIF tag the hub took off goes"
        );
    }

    #[test]
    fn a_free_tag_needs_no_grant_and_another_does() {
        let [normal, holo, solar] = &crate::cards::FINISHES;
        let none = Held::default();
        assert!(may_stick(Held::of("tag_gg", None), normal));
        assert!(
            !may_stick(Held::of("tag_gg", None), holo),
            "free tags are Normal"
        );
        assert!(!may_stick(Held::of("tag_ez", None), normal));
        assert_eq!(Held::of("tag_nope", None), none);
        let me = profile(&[("tag_ez", 3, 1, 0)], &[], &["credits"]);
        let ez = Held::of("tag_ez", Some(&me));
        assert_eq!(
            ez,
            Held {
                copies: 3,
                holo: 1,
                solar: 0
            }
        );
        assert!(may_stick(ez, normal) && may_stick(ez, holo) && !may_stick(ez, solar));
        assert_eq!(ez.finish("solar").id, "normal", "not held: Normal");
        assert_eq!(ez.finish("holo").id, "holo");
        assert_eq!(ez.finish("glitter").id, "normal");
        assert!(may_stick(Held::LOCAL, solar));
    }

    /// A card sticks while the hub lets its kind be a tag and the profile holds a revealed
    /// copy, in the finishes held; never on a hub before card tags.
    #[test]
    fn a_credits_card_sticks_in_the_finishes_held() {
        let faces = |id: &str, kind: &str| {
            crate::cards::faces::parse(
                id,
                &serde_json::json!({"name": id, "set": "sjk_alpha", "set_name": "SJK Alpha",
                    "number": 1, "kind": kind, "rarity": "mythical", "lore": "",
                    "art": format!("cards/{id}.jpg")})
                .to_string(),
            )
            .unwrap()
        };
        let _faces = crate::cards::faces::install_for_test(vec![
            faces("sjk_alpha_001", "credits"),
            faces("sjk_alpha_010", "meme"),
        ]);
        let cards = [
            ("sjk_alpha_001", "holo", 1),
            ("sjk_alpha_001", "normal", 2),
            ("sjk_alpha_010", "solar", 1),
        ];
        let me = profile(&[], &cards, &["credits"]);
        assert_eq!(
            Held::of("sjk_alpha_001", Some(&me)),
            Held {
                copies: 3,
                holo: 1,
                solar: 0
            }
        );
        assert_eq!(
            Held::of("sjk_alpha_010", Some(&me)),
            Held::default(),
            "a meme"
        );
        assert_eq!(
            Held::of("sjk_alpha_099", Some(&me)),
            Held::default(),
            "no face"
        );
        let before = profile(&[], &cards, &[]);
        assert_eq!(Held::of("sjk_alpha_001", Some(&before)), Held::default());
        assert_eq!(tag_cards(Some(&me)), ["sjk_alpha_001"]);
        assert!(tag_cards(Some(&before)).is_empty());
        assert!(tag_cards(None).is_empty());
    }

    #[test]
    fn a_card_id_ends_in_its_number_and_is_no_tag() {
        for id in ["sjk_alpha_000", "jof_123", "a_001"] {
            assert!(is_card_id(id), "{id}");
        }
        for id in [
            "tag_gg",
            "sjk_alpha_03",
            "sjk_alpha_0031",
            "SJK_001",
            "sjk-a_001",
            "",
        ] {
            assert!(!is_card_id(id), "{id}");
        }
        let placed = |finish: Option<&str>| sjk_identity::PlacedTag {
            id: "sjk_alpha_001".to_owned(),
            map: "mp/ffa3".to_owned(),
            at: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            spin: 0.0,
            finish: finish.map(str::to_owned),
        };
        assert!(decal(&placed(None)).is_some(), "a card lies on the world");
        assert_eq!(finish_of(&placed(None)).id, "normal");
        assert_eq!(finish_of(&placed(Some("solar"))).id, "solar");
        assert_eq!(finish_of(&placed(Some("chrome"))).id, "normal");
    }

    /// The tag's top, laid out from the normal and the turn alone, points where it
    /// should: up on walls whichever way they face, ahead on a floor.
    #[test]
    fn a_tag_stands_upright_on_walls_and_faces_the_player_on_floors() {
        let top = |normal: Vec3, spin: f32| -crate::decal_marks::texture_axes(normal, spin)[2];
        for normal in [
            Vec3::X,
            -Vec3::X,
            Vec3::Y,
            -Vec3::Y,
            Vec3::new(1.0, 1.0, 0.3).normalize(),
        ] {
            let spin = spin_for(normal, up_for(normal, Vec3::X));
            assert!((0.0..360.0).contains(&spin));
            let up = top(normal, spin);
            let want = (Vec3::Z - normal * normal.z).normalize();
            assert!(up.dot(want) > 0.999, "{normal} {up}");
            // Read from the front, the picture's left-to-right runs to the viewer's right.
            let right = (-normal).cross(want);
            assert!(crate::decal_marks::texture_axes(normal, spin)[1].dot(right) > 0.999);
        }
        for forward in [Vec3::X, Vec3::Y, Vec3::new(-1.0, 0.5, -0.4).normalize()] {
            let spin = spin_for(Vec3::Z, up_for(Vec3::Z, forward));
            let want = Vec3::new(forward.x, forward.y, 0.0).normalize();
            assert!(top(Vec3::Z, spin).dot(want) > 0.999, "{forward}");
        }
    }

    #[test]
    fn the_own_tag_goes_with_its_server_map_or_grant_and_waits_its_gap() {
        let server: std::net::SocketAddr = "1.2.3.4:29070".parse().unwrap();
        let other: std::net::SocketAddr = "1.2.3.4:29071".parse().unwrap();
        let placed = |id: &str| sjk_identity::PlacedTag {
            finish: None,
            id: id.to_owned(),
            map: "mp/ffa3".to_owned(),
            at: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            spin: 0.0,
        };
        let now = Instant::now();
        let mut tags = Tags::default();
        assert!(tags.may_stick(now).is_ok());
        tags.stick(server, placed("tag_gg"), now);
        assert!(tags.may_stick(now + GAP / 2).is_err());
        assert!(tags.may_stick(now + GAP).is_ok());
        let lacks = |id: &str| Held::of(id, None);
        let holds = |_: &str| Held::LOCAL;
        assert!(tags.own(Some(server), Some("mp/ffa3"), lacks).is_some());
        assert!(tags.own(Some(server), Some("mp/duel6"), lacks).is_none());
        tags.stick(server, placed("tag_ez"), now);
        assert!(tags.own(Some(other), Some("mp/ffa3"), holds).is_none());
        tags.stick(server, placed("tag_ez"), now);
        assert!(
            tags.own(Some(server), Some("mp/ffa3"), lacks).is_none(),
            "taken back"
        );
        // A Holo tag goes with the last Holo copy.
        let holo = sjk_identity::PlacedTag {
            finish: Some("holo".to_owned()),
            ..placed("tag_ez")
        };
        tags.stick(server, holo, now);
        let normal_only = |_: &str| Held {
            copies: 2,
            ..Held::default()
        };
        assert!(tags.own(Some(server), Some("mp/ffa3"), holds).is_some());
        assert!(
            tags.own(Some(server), Some("mp/ffa3"), normal_only)
                .is_none()
        );
        tags.stick(server, placed("tag_ez"), now);
        assert!(tags.clear());
        assert!(!tags.clear());
    }
}
