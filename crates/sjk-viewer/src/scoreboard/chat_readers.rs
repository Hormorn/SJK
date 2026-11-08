//! SJK chat readers whose client claims nothing (`docs/identity.md`, "What happens"): a
//! scoreboard row the hub holds no claim for gets the icon of the client such players are
//! taken to be on (`chat::client_icons::unclaimed`, today JoF EternalJK) when its name is
//! the name of exactly one online SJK chat reader. Sol's request, 08/11/2026: JoF EJK
//! players talk in SJK chat but showed no mark in game, since that client does not claim
//! its slot yet.
//!
//! A name proves nothing, so the row gets the icon alone: no tick, trust mark, medals or
//! emoji, and the player card and the game chat's marks stay as they were. Left out: bots,
//! the player's own key, readers holding a claim on this server or playing a match (a
//! claim marked active, which only SJK makes), readers whose newest message came from SJK,
//! a reader whose name is on a row a claim already marks, and any name two rows or two
//! readers share.

use super::ScoreRow;
use crate::player_identity::Tag;
use sjk_identity::{ChatState, Snapshot, names_match};

/// What changes the marks: the hub's roster and the chat (its readers, its messages).
pub(super) fn revision() -> u64 {
    crate::player_identity::revision()
        ^ crate::player_identity::with_chat(|chat| chat.revision)
            .unwrap_or(0)
            .rotate_left(32)
}

/// Give the rows no claim marks the guessed client's icon where the chat's readers seem
/// to sit.
pub(super) fn mark(rows: &mut [ScoreRow]) {
    let Some(icon) = crate::chat::client_icons::unclaimed() else {
        return;
    };
    let Some((own, claimed)) = crate::player_identity::with_snapshot(claims) else {
        return;
    };
    let shown: Vec<(u8, &str, bool)> = rows
        .iter()
        .filter(|row| !row.bot)
        .map(|row| (row.client_num, row.name.as_str(), row.identity.is_some()))
        .collect();
    let slots =
        crate::player_identity::with_chat(|chat| matched(&shown, &readers(chat, &own, &claimed)))
            .unwrap_or(0);
    for row in rows
        .iter_mut()
        .filter(|row| row.client_num < 64 && slots & 1 << row.client_num != 0)
    {
        row.identity = Some(Tag {
            client: Some(icon),
            ..Tag::default()
        });
    }
}

/// The player's own key ids (the key and its person's) and the keys claiming on this
/// server.
fn claims(snapshot: &Snapshot) -> (Vec<String>, Vec<String>) {
    let mut own = vec![snapshot.key_id.clone()];
    own.extend(snapshot.me.as_ref().map(|me| me.key_id.clone()));
    let claimed = snapshot
        .players
        .iter()
        .map(|player| player.key_id.clone())
        .collect();
    (own, claimed)
}

/// The names each online reader who may be on a client that claims nothing goes by: their
/// hub name and the name on their newest message.
fn readers<'a>(chat: &'a ChatState, own: &[String], claimed: &[String]) -> Vec<[&'a str; 2]> {
    let Some(people) = &chat.people else {
        return Vec::new();
    };
    people
        .online
        .iter()
        .filter(|person| {
            !person.playing && !own.contains(&person.key_id) && !claimed.contains(&person.key_id)
        })
        .filter_map(|person| {
            let newest = chat
                .messages
                .iter()
                .rev()
                .find(|message| message.key_id == person.key_id && message.holocron.is_none());
            // SJK claims its slot: its reader is somewhere else.
            if newest.is_some_and(|message| message.client == "sjk") {
                return None;
            }
            Some([
                person.name.as_str(),
                newest.map_or("", |message| message.name.as_str()),
            ])
        })
        .collect()
}

/// The slots of `rows` (slot, name shown, a claim marks it) where exactly one of
/// `readers` seems to sit: a reader one row only names, untagged, whose name no other
/// reader goes by.
fn matched(rows: &[(u8, &str, bool)], readers: &[[&str; 2]]) -> u64 {
    let goes_by =
        |names: &[&str; 2], shown: &str| names.iter().any(|name| names_match(name, shown));
    let mut slots = 0_u64;
    for names in readers {
        let mut hits = rows.iter().filter(|(_, shown, _)| goes_by(names, shown));
        let (Some(&(slot, shown, tagged)), None) = (hits.next(), hits.next()) else {
            continue;
        };
        let sharing = readers.iter().filter(|other| goes_by(other, shown)).count();
        if !tagged && sharing == 1 && slot < 64 {
            slots |= 1 << slot;
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reader_marks_the_one_row_with_their_name() {
        let rows = [(0, "^1Vinry", false), (3, "Kyle", false)];
        assert_eq!(matched(&rows, &[["Vinry", ""]]), 1);
        // The name on their newest message counts too (a hub name of their own).
        assert_eq!(matched(&rows, &[["Vin", "^7kyle"]]), 1 << 3);
        assert_eq!(matched(&rows, &[["Nobody", ""]]), 0);
    }

    #[test]
    fn a_shared_or_claimed_name_marks_nothing() {
        // Two rows wear the name.
        assert_eq!(
            matched(&[(0, "Kyle", false), (1, "kyle", false)], &[["Kyle", ""]]),
            0
        );
        // Two readers go by it.
        assert_eq!(
            matched(&[(2, "Kyle", false)], &[["Kyle", ""], ["x", "Kyle"]]),
            0
        );
        // A claim already marks the row.
        assert_eq!(matched(&[(2, "Kyle", true)], &[["Kyle", ""]]), 0);
        // An empty name names no one.
        assert_eq!(matched(&[(2, "", false)], &[["", ""]]), 0);
    }

    #[test]
    fn sjk_readers_and_claims_are_left_to_the_claims() {
        let person = |key: &str, name: &str, playing: bool| sjk_identity::Person {
            key_id: key.to_owned(),
            name: name.to_owned(),
            playing,
            ..Default::default()
        };
        let mut chat = ChatState {
            people: Some(sjk_identity::People {
                online: vec![
                    person("own", "Me", false),
                    person("claims", "Here", false),
                    person("playing", "Busy", true),
                    person("sjk", "Talker", false),
                    person("ejk", "Jof", false),
                ],
                recent: Vec::new(),
            }),
            ..Default::default()
        };
        let said = |key: &str, client: &str| -> sjk_identity::ChatMessage {
            serde_json::from_value(serde_json::json!({
                "id": 1, "at": 0, "key_id": key, "name": "Worn", "client": client, "text": "hi"
            }))
            .unwrap()
        };
        chat.messages.push_back(said("sjk", "sjk"));
        chat.messages.push_back(said("ejk", "ejk"));
        let readers = readers(&chat, &["own".to_owned()], &["claims".to_owned()]);
        assert_eq!(readers, [["Jof", "Worn"]]);
    }
}
