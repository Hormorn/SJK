//! Selling the Fishing toy's junk to the SJK hub for credits (Sol's yes on SJK #278,
//! 03/11/2026): the Fishing card in Collection > Toys lists the catches and, when the hub
//! buys junk from this player, a "Sell junk" button; `fishing_sell` in the console does
//! the same. The hub pays a few credits a piece up to a small sum a day (`PROTOCOL.md`
//! there, "Fishing junk"; `sjk_identity::junk`), the most valuable pieces first, and answers what
//! it took: only that is counted sold in the fish log (`sold <id> <count>` lines), so the
//! counts of what was caught never change.

use crate::GpuState;
use crate::fishing::{CATCHES, Catch, Kind, Log};
use std::time::{Duration, Instant};

/// The console command that sells the junk held.
pub(crate) const COMMAND: &str = "fishing_sell";
/// The console command that prints the fish log in full.
pub(crate) const LOG_COMMAND: &str = "fishing_log";
/// How often the card asks the hub what it still pays, while it is on show.
pub(crate) const READ_EVERY: Duration = Duration::from_secs(30);
/// How long a sale waits for the hub's answer before another may be asked.
pub(crate) const ASK_TIMEOUT: Duration = Duration::from_secs(20);

/// The sale's state, kept with the toy (Fishing's junk, and since 05/11/2026 Mining's
/// rubble, [`crate::mining_sell`], by the same hub routes: one sale runs at a time, and
/// each toy acts only on the answer to its own).
#[derive(Debug, Default)]
pub(crate) struct Sale {
    /// A sale was asked then and waits for its answer.
    pub(crate) asked: Option<Instant>,
    /// The last answer acted on (or passed over, the other toy's).
    pub(crate) seen: u64,
    /// The last sale's result, for the card.
    pub(crate) line: Option<String>,
    /// When the hub was last asked what it pays.
    pub(crate) read_at: Option<Instant>,
}

impl Sale {
    /// Whether a sale asked waits for the hub's answer at `now`.
    pub(crate) fn waiting(&self, now: Instant) -> bool {
        self.asked
            .is_some_and(|asked| now.duration_since(asked) < ASK_TIMEOUT)
    }
}

/// What a toy's card shows of its log and the sale (the Fishing card's fish log, the
/// Mining card's mining log).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Card {
    /// The log's heading: "Fish log", "Mining log".
    pub(crate) title: &'static str,
    /// What sells, for the button and its notes: "junk", "rubble".
    pub(crate) goods: &'static str,
    /// "23 fish, 7 of 12 kinds", or that none were caught yet.
    pub(crate) summary: String,
    /// The fish caught, with their counts.
    pub(crate) fish: String,
    /// The junk held, with its counts (empty: none).
    pub(crate) junk: String,
    /// The card's one line of junk: how many pieces, and what they sell for now.
    pub(crate) junk_line: String,
    /// How many pieces of junk are held.
    pub(crate) pieces: u32,
    /// The Sell button: what it would pay now; `None` hides it (the hub does not buy
    /// junk from this player, has no junk selling, or cannot be reached).
    pub(crate) sell: Option<u32>,
    /// A sale waits for the hub.
    pub(crate) selling: bool,
    /// The last sale's result, or why the button does nothing.
    pub(crate) line: Option<String>,
}

/// A catch's name for the card: without its article, and before any comma ("Old
/// Whiskers, the great catfish" is Old Whiskers).
pub(crate) fn short_name(catch: &Catch) -> &'static str {
    let name = catch.name;
    let name = name
        .strip_prefix("a ")
        .or_else(|| name.strip_prefix("an "))
        .unwrap_or(name);
    name.split(',').next().unwrap_or(name)
}

/// The kinds of `kind` in `log` with a count above 0 by `count`, as "name n" joined.
fn listed(log: &Log, kind: Kind, count: impl Fn(&Log, &str) -> u32) -> String {
    CATCHES
        .iter()
        .filter(|c| c.kind == kind)
        .filter_map(|c| {
            let n = count(log, c.id);
            (n > 0).then(|| format!("{} {n}", short_name(c)))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The card for `log`, the hub paying `payable` now and still paying so much today
/// (`None`: no Sell button), a sale waiting (`selling`) and the last result `line`.
pub(crate) fn card(
    log: &Log,
    payable: Option<(u32, u32)>,
    selling: bool,
    line: Option<String>,
) -> Card {
    let fish: u32 = CATCHES
        .iter()
        .filter(|c| c.kind == Kind::Fish)
        .map(|c| log.count(c.id))
        .sum();
    let (kinds, all) = log.species();
    let summary = if fish == 0 {
        "No fish caught yet.".to_owned()
    } else {
        format!("{fish} fish, {kinds} of {all} kinds")
    };
    let pieces: u32 = log.junk_held().values().sum();
    let piece = if pieces == 1 { "piece" } else { "pieces" };
    let junk_line = match payable {
        _ if pieces == 0 => "Junk: none".to_owned(),
        Some((pays, left)) => {
            format!("Junk: {pieces} {piece}, worth {pays} credits ({left} left today)")
        }
        None => format!("Junk: {pieces} {piece}"),
    };
    Card {
        title: "Fish log",
        goods: "junk",
        summary,
        fish: listed(log, Kind::Fish, Log::count),
        junk: listed(log, Kind::Junk, Log::held),
        junk_line,
        pieces,
        sell: payable.map(|(pays, _)| pays),
        selling,
        line,
    }
}

/// The result line of a sale that took `pieces` for `credits`.
pub(crate) fn sold_line(pieces: u32, credits: u32) -> String {
    let piece = if pieces == 1 { "piece" } else { "pieces" };
    format!("Sold {pieces} {piece} of junk for {credits} credits.")
}

impl GpuState {
    /// `fishing_log`: print the fish log in full to the console, every kind with its
    /// count, the junk still held and what was sold.
    pub(crate) fn print_fishing_log(&mut self) {
        let Some(directory) = self
            .console
            .as_ref()
            .map(|c| c.config_directory().to_owned())
        else {
            return;
        };
        let log = self.fishing.log_in(&directory);
        let shown = card(log, None, false, None);
        let sold = listed(log, Kind::Junk, Log::sold_count);
        let mut lines = vec![format!("Fish log: {}", shown.summary)];
        if !shown.fish.is_empty() {
            lines.push(format!("Fish: {}", shown.fish));
        }
        lines.push(if shown.junk.is_empty() {
            "Junk held: none".to_owned()
        } else {
            format!("Junk held: {}", shown.junk)
        });
        if !sold.is_empty() {
            lines.push(format!("Junk sold: {sold}"));
        }
        if let Some(console) = self.console.as_mut() {
            for line in lines {
                console.push_log(line);
            }
        }
    }

    /// `fishing_sell`, or the card's Sell button: sell the junk the log holds, once a sale
    /// is not waiting already.
    pub(crate) fn sell_fishing_junk(&mut self) {
        let now = Instant::now();
        // One sale at a time, the Mining card's too: the hub answers one.
        if self.fishing.sale.waiting(now) || self.mining.sale.waiting(now) {
            return;
        }
        let Some(directory) = self
            .console
            .as_ref()
            .map(|c| c.config_directory().to_owned())
        else {
            return;
        };
        let held = self.fishing.log_in(&directory).junk_held();
        let said = if held.is_empty() {
            "No junk to sell: fish some up first.".to_owned()
        } else {
            let open = crate::player_identity::with_snapshot(|snapshot| {
                snapshot
                    .junk
                    .as_ref()
                    .map(|junk| (junk.open, junk.payable(&held)))
            })
            .flatten();
            match open {
                Some((true, paid)) if paid > 0 => {
                    if crate::player_identity::with_service(|service| service.sell_junk(held))
                        .is_some()
                    {
                        self.fishing.sale.asked = Some(now);
                        self.fishing.sale.line = Some("Selling...".to_owned());
                        return;
                    }
                    "SJK online is off: the hub buys the junk.".to_owned()
                }
                Some((true, _)) => {
                    "The hub buys no more junk from you today: come back tomorrow.".to_owned()
                }
                _ => {
                    // Not known yet, or closed: ask again for the next try.
                    let _ = crate::player_identity::with_service(|service| service.read_junk());
                    "The hub does not buy junk from you now.".to_owned()
                }
            }
        };
        self.fishing.sale.line = Some(said.clone());
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            said,
            None,
            Instant::now(),
        );
    }

    /// The sale's frame: act on the hub's answer once, keep what it pays read while the
    /// card is on show, and give the console the card.
    pub(super) fn fishing_sale_frame(&mut self, now: Instant) {
        self.read_fishing_sale_answer();
        let shown = self
            .console
            .as_ref()
            .is_some_and(|console| console.fishing_card_shown());
        if !shown {
            return;
        }
        if self
            .fishing
            .sale
            .read_at
            .is_none_or(|at| now.duration_since(at) >= READ_EVERY)
            && crate::player_identity::with_service(|service| service.read_junk()).is_some()
        {
            self.fishing.sale.read_at = Some(now);
        }
        let Some(directory) = self
            .console
            .as_ref()
            .map(|c| c.config_directory().to_owned())
        else {
            return;
        };
        let selling = self.fishing.sale.waiting(now);
        let line = self.fishing.sale.line.clone();
        let log = self.fishing.log_in(&directory);
        let held = log.junk_held();
        let payable = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .junk
                .as_ref()
                .filter(|junk| junk.open)
                .map(|junk| (junk.payable(&held), junk.left))
        })
        .flatten();
        let card = card(log, payable, selling, line);
        if let Some(console) = self.console.as_mut() {
            console.set_fishing_card(card);
        }
    }

    /// Act on the hub's last answer to a sale, once: count what it took sold and say so.
    fn read_fishing_sale_answer(&mut self) {
        let seen = self.fishing.sale.seen;
        let Some((serial, outcome)) = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .junk_sale
                .as_ref()
                .filter(|(serial, _)| *serial != seen)
                .cloned()
        })
        .flatten() else {
            return;
        };
        self.fishing.sale.seen = serial;
        // The Mining card's sale, not this one's (`mining_sell`).
        if self.fishing.sale.asked.take().is_none() {
            return;
        }
        let said = match outcome {
            Ok(sold) => {
                let Some(directory) = self
                    .console
                    .as_ref()
                    .map(|c| c.config_directory().to_owned())
                else {
                    return;
                };
                let pieces = self.fishing.log_in(&directory).sell(&sold.sold);
                self.fishing.save_log(&directory);
                sold_line(pieces, sold.credits)
            }
            Err(why) => why,
        };
        self.fishing.sale.line = Some(said.clone());
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            said,
            None,
            Instant::now(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_lists_catches_and_what_is_held() {
        let mut log = Log::parse("silver_darter 3\nold_whiskers 1\nboot 4\nhilt 1\nsold boot 3\n");
        let shown = card(&log, Some((6, 9)), false, None);
        assert_eq!(
            shown.summary,
            format!("4 fish, 2 of {} kinds", crate::fishing::FISH_KINDS)
        );
        assert_eq!(shown.fish, "Silver darter 3 · Old Whiskers 1");
        assert_eq!(shown.junk, "old boot 1 · broken saber hilt 1");
        assert_eq!(shown.sell, Some(6));
        assert_eq!(shown.pieces, 2);
        assert_eq!(
            shown.junk_line,
            "Junk: 2 pieces, worth 6 credits (9 left today)"
        );
        assert_eq!(card(&log, None, false, None).junk_line, "Junk: 2 pieces");
        let empty = card(&Log::default(), None, false, None);
        assert_eq!(empty.summary, "No fish caught yet.");
        assert!(empty.fish.is_empty() && empty.junk.is_empty());
        // Selling everything leaves no junk listed.
        let all = log.junk_held();
        assert_eq!(log.sell(&all), 2);
        let sold = card(&log, Some((0, 0)), false, None);
        assert!(sold.junk.is_empty());
        assert_eq!(sold.junk_line, "Junk: none");
    }

    #[test]
    fn a_sale_says_what_it_took() {
        assert_eq!(sold_line(1, 5), "Sold 1 piece of junk for 5 credits.");
        assert_eq!(sold_line(4, 8), "Sold 4 pieces of junk for 8 credits.");
    }
}
