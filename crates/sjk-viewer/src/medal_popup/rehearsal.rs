//! `debug_medal`: rehearse receiving SJK medals without the hub (`docs/client.md`,
//! "Identity"). It makes the list a hub profile would carry (`sjk_identity::Medal`:
//! id, count, rank, today's date and a note), which the pop-up reads as it reads the
//! hub's (`medals::awards`) and queues like a real arrival, so the card in a match, the
//! pop-up on a menu, the ceremony, the fanfare, Next and each rank's look can all be
//! tried offline. A rehearsed medal above rank I arrives as a promotion.
//!
//! Nothing is sent: the command never reaches the identity service. Rehearsed medals
//! are not written to `medals_seen.txt`, so a medal the SJK team gives later still
//! shows.
//!
//! ```text
//! debug_medal                          the ids and how to use it
//! debug_medal early_tester             one medal
//! debug_medal bug_hunter The fog      one medal, with a note
//! debug_medal early_tester r4          promoted to rank IV Holo
//! debug_medal early_tester ranks       ranks I to V, one after another
//! debug_medal all [note]               every medal, one after another
//! ```

use crate::medals::{Award, Medal, Rank};

/// The console command.
pub(crate) const COMMAND: &str = "debug_medal";
/// Its line in the command list and browser.
pub(crate) const HELP: &str = "Rehearse receiving SJK medals, nothing sent: debug_medal <id|all> [x<count>] [r<rank>|ranks] [note]";
/// The word that names every medal.
const ALL: &str = "all";
/// The word that rehearses every rank of the medals, I to V.
const RANKS: &str = "ranks";
/// The usage line.
const USAGE: &str = "usage: debug_medal <id|all> [x<count>] [r<rank>|ranks] [note]";
/// The most a rehearsed count may be.
const MOST: u32 = 999;

/// The command's usage and the medals it knows.
pub(crate) fn listing() -> Vec<String> {
    let mut lines = vec![
        USAGE.to_owned(),
        "Shows medals as if the SJK team had just given them: the pop-up on the main menu or the game menu (a card during a match), with its ceremony and sound. Nothing is sent to the SJK hub and medals_seen.txt is not changed.".to_owned(),
        "r<rank> (r1 to r5) rehearses a promotion to that rank; ranks shows ranks I to V one after another: I Issued, II Polished, III Engraved, IV Holo, V Solar.".to_owned(),
    ];
    for medal in Medal::ALL {
        let again = if medal.repeatable() {
            ", given again with x<count>"
        } else {
            ""
        };
        lines.push(format!("  {:<18} {}{again}", medal.id(), medal.name()));
    }
    lines.push(format!("  {ALL:<18} every medal, one after another"));
    lines
}

/// The list a hub profile would carry for `args` (an id or `all`, an optional
/// `x<count>`, an optional `r<rank>` or `ranks`, then the note's words), given at `now`
/// (unix seconds). With `ranks` each medal comes five times, ranks I to V.
pub(crate) fn parse(args: &[String], now: i64) -> Result<Vec<sjk_identity::Medal>, String> {
    let Some((first, rest)) = args.split_first() else {
        return Err(USAGE.to_owned());
    };
    let first = first.to_ascii_lowercase();
    let medals: Vec<Medal> = if first == ALL {
        Medal::ALL.to_vec()
    } else {
        let medal = Medal::from_id(&first).ok_or_else(|| {
            let ids: Vec<&str> = Medal::ALL.iter().map(|medal| medal.id()).collect();
            format!("Unknown medal '{first}': {} or {ALL}", ids.join(", "))
        })?;
        vec![medal]
    };
    let (count, rest) = match rest.split_first() {
        Some((word, rest)) if count_word(word).is_some() => (count_word(word), rest),
        _ => (None, rest),
    };
    let (ranks, note): (Vec<Rank>, _) = match rest.split_first() {
        Some((word, note)) if word.eq_ignore_ascii_case(RANKS) => (Rank::ALL.to_vec(), note),
        Some((word, note)) => match rank_word(word) {
            Some(rank) => (vec![rank?], note),
            None => (vec![Rank::Issued], rest),
        },
        None => (vec![Rank::Issued], rest),
    };
    let count = match count {
        Some(Ok(count)) => count,
        Some(Err(error)) => return Err(error),
        None => 1,
    };
    if count > 1 && medals.iter().all(|medal| !medal.repeatable()) {
        let repeatable: Vec<&str> = Medal::ALL
            .iter()
            .filter(|medal| medal.repeatable())
            .map(|medal| medal.name())
            .collect();
        let takes = if repeatable.is_empty() {
            "no medal takes a count".to_owned()
        } else {
            format!("only {} takes a count", repeatable.join(", "))
        };
        return Err(format!("{} is given only once; {takes}", medals[0].name()));
    }
    let note = note.join(" ");
    Ok(medals
        .into_iter()
        .flat_map(|medal| ranks.iter().map(move |rank| (medal, *rank)))
        .map(|(medal, rank)| sjk_identity::Medal {
            id: medal.id().to_owned(),
            count: if medal.repeatable() { count } else { 1 },
            awarded: now,
            note: note.clone(),
            rank: rank.number(),
        })
        .collect())
}

/// `r<rank>` read: `None` when `word` is not one, else the rank or why it is refused.
fn rank_word(word: &str) -> Option<Result<Rank, String>> {
    let digits = word.strip_prefix(['r', 'R'])?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(match digits.parse::<u8>() {
        Ok(rank) if (1..=5).contains(&rank) => Ok(Rank::from_wire(rank)),
        _ => Err("A rank is r1 to r5".to_owned()),
    })
}

/// `x<count>` read: `None` when `word` is not one, else the count or why it is
/// refused.
fn count_word(word: &str) -> Option<Result<u32, String>> {
    let digits = word.strip_prefix(['x', 'X'])?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(match digits.parse::<u32>() {
        Ok(count) if (1..=MOST).contains(&count) => Ok(count),
        _ => Err(format!("A count is x1 to x{MOST}")),
    })
}

/// What the console says once `awards` are queued.
pub(crate) fn queued(awards: &[Award]) -> Vec<String> {
    let names: Vec<String> = awards
        .iter()
        .map(|award| match award.rank {
            Rank::Issued => award.label(),
            rank => format!("{} ({})", award.label(), rank.title()),
        })
        .collect();
    vec![
        format!("Rehearsing: {}.", names.join(", ")),
        "It shows on the main menu or when the game menu opens (a card says so during a match). Nothing is sent to the SJK hub.".to_owned(),
    ]
}

/// The awards `list` makes, one for each entry, ranks rehearsed one after another
/// included: each read as the hub's profile is ([`crate::medals::awards`]).
pub(crate) fn awards(list: &[sjk_identity::Medal]) -> Vec<Award> {
    list.iter()
        .flat_map(|entry| crate::medals::awards(std::slice::from_ref(entry)))
        .collect()
}

/// Seconds since 1970 now, for the date a rehearsed medal was given.
pub(crate) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    const NOW: i64 = 1_791_456_000;

    #[test]
    fn one_medal_with_its_note_and_date() {
        let list = parse(&words("early_tester Thanks for testing ^3early"), NOW).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "early_tester");
        assert_eq!((list[0].count, list[0].awarded), (1, NOW));
        assert_eq!(list[0].note, "Thanks for testing ^3early");
        // Read as the hub's list is: the note plain, the date given.
        let awards = crate::medals::awards(&list);
        assert_eq!(awards[0].note, "Thanks for testing early");
        assert_eq!(awards[0].given(), "Given 08/10/2026");
        let ids = parse(&words("EARLY_CONTRIBUTOR"), NOW).unwrap();
        assert_eq!(ids[0].id, "early_contributor");
        assert!(ids[0].note.is_empty());
    }

    #[test]
    fn a_count_for_a_repeatable_medal_only_and_none_is() {
        // Bug Hunter was the repeatable one until 21/10/2026: now it is given once.
        let error = parse(&words("bug_hunter x3 The fog"), NOW).unwrap_err();
        assert_eq!(
            error,
            "Bug Hunter is given only once; only Videographer takes a count"
        );
        let list = parse(&words("bug_hunter x1 The fog"), NOW).unwrap();
        assert_eq!((list[0].count, list[0].note.as_str()), (1, "The fog"));
        assert!(parse(&words("early_tester x2"), NOW).is_err());
        assert!(parse(&words("bug_hunter x0"), NOW).is_err());
        assert!(parse(&words("bug_hunter x1000"), NOW).is_err());
        // A word that only looks like one is part of the note.
        let list = parse(&words("bug_hunter xylophone"), NOW).unwrap();
        assert_eq!((list[0].count, list[0].note.as_str()), (1, "xylophone"));
    }

    #[test]
    fn all_gives_every_medal_in_the_catalogues_order() {
        let list = parse(&words("all Well done"), NOW).unwrap();
        let ids: Vec<&str> = list.iter().map(|medal| medal.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "early_tester",
                "early_contributor",
                "bug_hunter",
                "supporter",
                "videographer"
            ]
        );
        assert!(list.iter().all(|medal| medal.note == "Well done"));
        let counts: Vec<u32> = list.iter().map(|medal| medal.count).collect();
        assert_eq!(counts, [1, 1, 1, 1, 1]);
        // A count goes to the repeatable medal only (Videographer).
        let counted = parse(&words("all x2 Well done"), NOW).unwrap();
        let counts: Vec<u32> = counted.iter().map(|medal| medal.count).collect();
        assert_eq!(counts, [1, 1, 1, 1, 2]);
    }

    #[test]
    fn a_rank_or_every_rank_one_after_another() {
        let list = parse(&words("early_tester r4 Top tester"), NOW).unwrap();
        assert_eq!((list.len(), list[0].rank), (1, 4));
        assert_eq!(list[0].note, "Top tester");
        assert_eq!(parse(&words("early_tester"), NOW).unwrap()[0].rank, 1);
        let ranks = parse(&words("bug_hunter ranks"), NOW).unwrap();
        let numbers: Vec<u8> = ranks.iter().map(|medal| medal.rank).collect();
        assert_eq!(numbers, [1, 2, 3, 4, 5]);
        let awards = awards(&ranks);
        assert_eq!(awards.len(), 5, "not folded into one");
        assert_eq!(awards[4].rank, Rank::Solar);
        assert!(queued(&awards)[0].contains("Bug Hunter (Rank V Solar)"));
        assert_eq!(parse(&words("all ranks"), NOW).unwrap().len(), 25);
        assert!(parse(&words("early_tester r6"), NOW).is_err());
        assert!(parse(&words("early_tester r0"), NOW).is_err());
        // A word that only looks like one is part of the note.
        let list = parse(&words("early_tester rocks"), NOW).unwrap();
        assert_eq!((list[0].rank, list[0].note.as_str()), (1, "rocks"));
    }

    #[test]
    fn an_unknown_id_names_the_known_ones_and_alone_it_lists_them() {
        let error = parse(&words("gold_star"), NOW).unwrap_err();
        assert!(error.contains("early_tester") && error.contains("bug_hunter"));
        assert!(error.contains(ALL));
        assert!(parse(&[], NOW).is_err());
        let listing = listing();
        for medal in Medal::ALL {
            assert!(listing.iter().any(|line| line.contains(medal.id())));
        }
        assert!(listing.iter().any(|line| line.contains("Nothing is sent")));
        assert!(COMMAND.starts_with("debug_") && COMMAND == COMMAND.to_ascii_lowercase());
    }
}
