//! The wishlist (10/11/2026; `docs/cards.md`, "Fusion, deal of the day, wishlist"): up to
//! [`MAX`] saber shaders of the trade the player lacks, kept by the hub
//! (`economy.wishlist`, `PUT /v1/shop/wishlist`). The Shaders tab's card, the Forge's Craft
//! card and the shop's offers add a shader or take it off; the shop lists it and marks a
//! wished offer; and once a shop day a card says when a wished shader is among the day's
//! offers ([`crate::unlock_toast::Unlock::Wished`]). A hub that keeps no wishlist sends
//! none, and then nothing of it shows.
//!
//! Twice a second ([`crate::GpuState::follow_wishlist`]) the client also reads the shop
//! once a shop day on such a hub (the Forge reads it when it shows anyway), so the card and
//! the Forge's dot ([`crate::forge_alert`]) know the day's offers without the Forge open.
//! What the card told is kept in `wishlist_notice.txt` in the settings folder, beside
//! `identity.key`, per key, so a restart the same day does not tell it again.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The most shaders a wishlist holds.
pub(crate) const MAX: usize = sjk_identity::WISHLIST_MAX;

/// The file that keeps the last shop day the card told, in the settings folder.
pub(crate) const FILE: &str = "wishlist_notice.txt";
const HEADER: &str = "# The last shop day SJK told this identity a wished shader was on offer.";

/// The least time between two reads of the shop this asks for.
const READ_EVERY: Duration = Duration::from_secs(60);
/// After a read the hub refused, the time before asking again.
const READ_AGAIN: Duration = Duration::from_secs(30 * 60);

/// The wishlist the hub keeps; `None` from a hub that keeps none.
pub(crate) fn of(snapshot: &sjk_identity::Snapshot) -> Option<&[String]> {
    snapshot.economy.as_ref()?.wishlist.as_deref()
}

/// What adding (`on`) or taking off a shader does to the wishlist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Change {
    /// The wishlist it becomes.
    To(Vec<String>),
    /// It holds [`MAX`] already: one must go first.
    Full,
    /// Nothing changes (it is on, or off, already).
    Same,
}

/// What `on` (add) or off does with shader `id` to `list`.
pub(crate) fn toggle(list: &[String], id: &str, on: bool) -> Change {
    let held = list.iter().any(|wished| wished == id);
    match (on, held) {
        (true, true) | (false, false) => Change::Same,
        (true, false) if list.len() >= MAX => Change::Full,
        (true, false) => {
            let mut next = list.to_vec();
            next.push(id.to_owned());
            Change::To(next)
        }
        (false, true) => Change::To(
            list.iter()
                .filter(|wished| *wished != id)
                .cloned()
                .collect(),
        ),
    }
}

/// Whether a shader may go on the wishlist: of the trade (what the shop and the Forge
/// deal in) and not held (`held`). One on it already may always come off.
pub(crate) fn may_wish(skin: &crate::unlockables::Unlockable, held: bool) -> bool {
    crate::economy::in_trade(skin) && !held
}

/// The line players read when the wishlist is full.
pub(crate) fn full_words() -> String {
    format!("Your wishlist is full ({MAX} shaders): take one off first.")
}

/// Add (`on`) shader `id` to the live wishlist or take it off; the line to tell the
/// player when it cannot ([`Change::Full`], or no wishlist on this hub).
pub(crate) fn set(id: &str, on: bool) -> Option<String> {
    let change = crate::player_identity::with_snapshot(|snapshot| {
        of(snapshot).map(|list| toggle(list, id, on))
    })
    .flatten();
    match change {
        Some(Change::To(list)) => {
            let _ = crate::player_identity::with_service(|service| service.set_wishlist(list));
            None
        }
        Some(Change::Full) => Some(full_words()),
        Some(Change::Same) => None,
        None => Some("This SJK hub keeps no wishlist yet.".to_owned()),
    }
}

/// The first of the day's offers on the wishlist that the player can still buy.
pub(crate) fn wished_offer(
    shop: &sjk_identity::ShopState,
) -> Option<&'static crate::unlockables::Unlockable> {
    if !shop.message.is_empty() {
        return None;
    }
    shop.day
        .offers
        .iter()
        .filter(|offer| offer.wished && !offer.owned && !offer.bought)
        .find_map(|offer| crate::unlockables::find(&offer.id))
}

/// The last shop day the card told one key.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Told {
    key_id: String,
    day: String,
}

impl Told {
    /// What `text` says was told `key_id`; nothing when it is another key's.
    pub(crate) fn parse(text: &str, key_id: &str) -> Self {
        let mut told = Self {
            key_id: key_id.to_owned(),
            day: String::new(),
        };
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'));
        if lines.next().and_then(|line| line.strip_prefix("key ")) != Some(key_id) {
            return told;
        }
        for line in lines {
            if let Some(day) = line.strip_prefix("day ") {
                day.trim().clone_into(&mut told.day);
            }
        }
        told
    }

    /// The file `directory` holds for `key_id`.
    pub(crate) fn load(directory: &Path, key_id: &str) -> Self {
        let text = std::fs::read_to_string(directory.join(FILE)).unwrap_or_default();
        Self::parse(&text, key_id)
    }

    /// The file's text.
    pub(crate) fn text(&self) -> String {
        format!("{HEADER}\nkey {}\nday {}\n", self.key_id, self.day)
    }

    /// Whether the card is still to be told on shop day `day` (once a day).
    pub(crate) fn due(&self, day: &str) -> bool {
        !day.is_empty() && self.day != day
    }

    /// Count shop day `day` as told.
    pub(crate) fn mark(&mut self, day: &str) {
        day.clone_into(&mut self.day);
    }
}

/// What the follow keeps between its calls.
#[derive(Default)]
struct Follow {
    told: Option<Told>,
    /// When the shop was last asked for, and the read's serial then.
    asked: Option<(Instant, u64)>,
}

static FOLLOW: Mutex<Option<Follow>> = Mutex::new(None);

/// Whether the shop should be read now for the wishlist and the dot: on a hub that keeps
/// a wishlist, while the player may trade, when the day's offers are not known (never
/// read, the day turned, or a refusal half an hour ago), at most once a minute.
fn read_due(
    snapshot: &sjk_identity::Snapshot,
    asked: Option<(Instant, u64)>,
    now: Instant,
) -> bool {
    if of(snapshot).is_none() || !crate::economy::may_act(snapshot) {
        return false;
    }
    let since = |at: Instant| now.saturating_duration_since(at);
    if asked.is_some_and(|(at, _)| since(at) < READ_EVERY) {
        return false;
    }
    match &snapshot.shop {
        None => true,
        Some(shop) if !shop.message.is_empty() => {
            asked.is_none_or(|(at, _)| since(at) >= READ_AGAIN)
        }
        Some(shop) => shop.seconds_left(now) == 0,
    }
}

impl crate::GpuState {
    /// Twice a second, with the identity's settings: read the shop when the wishlist and
    /// the dot need it, tell once a shop day when a wished shader is on offer, and set the
    /// Forge's dot.
    pub(crate) fn follow_wishlist(&mut self) {
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let directory = console.config_directory().to_path_buf();
        let now = Instant::now();
        let mut guard = FOLLOW
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let follow = guard.get_or_insert_with(Follow::default);
        let seen = player_snapshot(|snapshot| {
            let alert = crate::forge_alert::reasons(snapshot, crate::economy::fuse_need).any();
            let read = read_due(snapshot, follow.asked, now);
            let serial = snapshot.shop.as_ref().map_or(0, |shop| shop.serial);
            let tell = snapshot
                .shop
                .as_ref()
                .and_then(|shop| wished_offer(shop).map(|skin| (shop.day.day.clone(), skin)));
            (alert, read, serial, tell, snapshot.local_key_id.clone())
        });
        let Some((alert, read, serial, tell, key_id)) = seen else {
            crate::forge_alert::set(false);
            return;
        };
        crate::forge_alert::set(alert);
        if read && crate::player_identity::read_shop() {
            follow.asked = Some((now, serial));
        }
        let Some((day, skin)) = tell else {
            return;
        };
        if key_id.is_empty() {
            return;
        }
        let told = match &mut follow.told {
            Some(told) if told.key_id == key_id => told,
            slot => slot.insert(Told::load(&directory, &key_id)),
        };
        if told.due(&day) {
            told.mark(&day);
            let _ = std::fs::write(directory.join(FILE), told.text());
            self.unlock_toast
                .push(crate::unlock_toast::Unlock::Wished { skin });
        }
    }
}

/// `read` of the live identity's snapshot while the hub is set and the player's profile
/// known; `None` otherwise.
fn player_snapshot<R>(read: impl FnOnce(&sjk_identity::Snapshot) -> R) -> Option<R> {
    crate::player_identity::with_snapshot(|snapshot| {
        (snapshot.me.is_some()
            && !matches!(
                snapshot.status,
                sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
            ))
        .then(|| read(snapshot))
    })
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn the_wishlist_holds_three_and_says_when_full() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            toggle(&empty, "saber_aurora", true),
            Change::To(ids(&["saber_aurora"]))
        );
        assert_eq!(toggle(&empty, "saber_aurora", false), Change::Same);
        let two = ids(&["saber_aurora", "saber_tidal"]);
        assert_eq!(toggle(&two, "saber_tidal", true), Change::Same);
        let three = ids(&["saber_aurora", "saber_tidal", "saber_sakura"]);
        assert_eq!(
            toggle(&two, "saber_sakura", true),
            Change::To(three.clone())
        );
        assert_eq!(toggle(&three, "saber_phoenix", true), Change::Full);
        assert_eq!(
            toggle(&three, "saber_tidal", false),
            Change::To(ids(&["saber_aurora", "saber_sakura"])),
            "one comes off, the order kept"
        );
        assert!(full_words().contains('3'));
        let skin = crate::unlockables::find("saber_aurora").unwrap();
        assert!(may_wish(skin, false));
        assert!(!may_wish(skin, true), "held already");
        for medal in crate::medals::Medal::ALL {
            if let Some(id) = medal.blade() {
                assert!(!may_wish(crate::unlockables::find(id).unwrap(), false));
            }
        }
    }

    #[test]
    fn the_card_is_told_once_a_shop_day_and_the_file_keeps_it() {
        let mut told = Told::parse("", "aa");
        assert!(told.due("2026-11-10"));
        assert!(!told.due(""), "no day, nothing to tell");
        told.mark("2026-11-10");
        assert!(!told.due("2026-11-10"));
        assert!(told.due("2026-11-11"));
        let again = Told::parse(&told.text(), "aa");
        assert_eq!(again, told);
        assert!(
            !again.due("2026-11-10"),
            "a restart the same day stays quiet"
        );
        // Another key's file tells this one afresh.
        assert!(Told::parse(&told.text(), "bb").due("2026-11-10"));
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join(FILE), told.text()).unwrap();
        assert_eq!(Told::load(directory.path(), "aa"), told);
    }

    #[test]
    fn the_wished_offer_is_one_still_to_buy() {
        let mut shop = crate::console::collection_panel::forge::sample_day(3_600);
        assert_eq!(wished_offer(&shop), None);
        shop.day.offers[1].wished = true;
        assert_eq!(
            wished_offer(&shop).map(|skin| skin.id),
            Some(shop.day.offers[1].id.as_str())
        );
        shop.day.offers[1].bought = true;
        assert_eq!(wished_offer(&shop), None, "bought today");
        shop.day.offers[2].wished = true;
        shop.day.offers[2].owned = true;
        assert_eq!(wished_offer(&shop), None, "held");
        shop.day.offers[3].wished = true;
        shop.message = "down".to_owned();
        assert_eq!(wished_offer(&shop), None, "a refused read");
    }

    #[test]
    fn the_shop_is_read_once_a_day_on_a_hub_with_a_wishlist() {
        let now = Instant::now();
        let mut snapshot = crate::console::collection_panel::forge::sample_snapshot(Vec::new());
        snapshot.shop = None;
        // A hub without a wishlist: never.
        if let Some(economy) = snapshot.economy.as_mut() {
            economy.wishlist = None;
        }
        assert!(!read_due(&snapshot, None, now));
        if let Some(economy) = snapshot.economy.as_mut() {
            economy.wishlist = Some(Vec::new());
        }
        assert!(read_due(&snapshot, None, now), "never read");
        assert!(
            !read_due(&snapshot, Some((now, 0)), now),
            "asked a moment ago"
        );
        snapshot.shop = Some(crate::console::collection_panel::forge::sample_day(3_600));
        let later = now + READ_EVERY * 2;
        assert!(
            !read_due(&snapshot, Some((now, 1)), later),
            "the day's offers known"
        );
        snapshot.shop = Some(crate::console::collection_panel::forge::sample_day(0));
        assert!(read_due(&snapshot, Some((now, 1)), later), "the day turned");
        let mut refused = crate::console::collection_panel::forge::sample_day(3_600);
        refused.message = "down".to_owned();
        snapshot.shop = Some(refused);
        assert!(!read_due(&snapshot, Some((now, 1)), later));
        assert!(read_due(&snapshot, Some((now, 1)), now + READ_AGAIN));
    }
}
