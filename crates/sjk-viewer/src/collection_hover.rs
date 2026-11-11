//! The Collection's hover cards (`docs/sjk-ui.md`, "Hover cards"): resting the pointer on
//! a medal, an achievement, a shader, a toy or a card shows beside it what it is, how it
//! is obtained and where the player stands with it ([`crate::menu::sjk::hover`]). The
//! words come from the same tables as the rest of the page: a medal's
//! [`crate::medals::Medal::how_to_get`], an achievement's goal, a blade skin's
//! `how_to_get` ([`crate::unlockables`]), a card's odds ([`crate::cards::loot`]).

use super::*;
use crate::achievements::Source;
use crate::menu::sjk::hover::{Card, Line};
use crate::menu::sjk::{color, hover};

impl Panel {
    /// Follow the pointer and draw the card of the thing it has rested on, over the
    /// page: the page's last drawing.
    pub(super) fn hover_card(&mut self, frame: &crate::menu::sjk::Frame, inputs: &Inputs<'_>) {
        if self.tab == Tab::Forge && self.forge.ask.is_some() {
            // The card asking covers the page: nothing under it answers the pointer.
            return;
        }
        // The showcase card a click opened stands over the page in place of a hover card.
        if self.tab != Tab::Nameplates {
            self.showcase_open = false;
        }
        if self.showcase_open {
            self.nameplate_showcase(frame, inputs);
            return;
        }
        // So does the Extra features note, while its chip shows (the same shader): what
        // the extras are and what unlocks them, or, unlocked, a tick each.
        if self.extras_note
            && self.tab == Tab::Shaders
            && !self.finishers_shown
            && let Some(anchor) = self.ui.rect_for(EXTRAS_NOTE_TOKEN)
        {
            if let Some(card) = self.extras_card.clone() {
                hover::draw_note(&mut self.ui, frame, anchor, &card);
                return;
            }
            if !self.extras_rows.is_empty() {
                self.extras_ticks(frame, anchor);
                return;
            }
        }
        self.extras_note = false;
        // The New note a click opened stands over the page in place of a hover card.
        if self.tab != Tab::Shaders || self.finishers_shown {
            self.new_note = false;
        }
        if self.new_note
            && let Some(anchor) = self.ui.rect_for(NEW_NOTE_TOKEN)
        {
            let card = self.new_note_card();
            hover::draw_note(&mut self.ui, frame, anchor, &card);
            return;
        }
        let now = Instant::now();
        self.hover.follow(self.ui.hovered(), now);
        let Some((token, alpha)) = self.hover.showing(now) else {
            return;
        };
        let (Some(card), Some(anchor)) = (self.card_of(token, inputs), self.ui.rect_for(token))
        else {
            return;
        };
        hover::draw(&mut self.ui, frame, anchor, &card, alpha);
    }

    /// What the card of the thing `token` stands for says; `None` for anything else (a
    /// tab, a button).
    pub(super) fn card_of(&self, token: u16, inputs: &Inputs<'_>) -> Option<Card> {
        let within = |base: u16, count: usize| {
            token
                .checked_sub(base)
                .map(usize::from)
                .filter(|index| *index < count)
        };
        if self.tab == Tab::Forge {
            return self.forge_card(token, inputs);
        }
        if token == TOY_MASTER_TOKEN {
            return Some(toy_master_card(inputs.standings));
        }
        if let Some(index) = within(MEDAL_BASE, self.medals_shown) {
            let (_, shown) = medals::showing(inputs);
            return shown.get(index).map(medal_card);
        }
        if let Some(index) = within(ACHIEVEMENT_BASE, inputs.standings.len().min(WALL_MAX)) {
            // The wall's medallions are the standings' in order: the catalogue's, then
            // the Toy Master's the player found.
            let standing = &inputs.standings[index];
            let kind = standing.kind;
            let standing = Some(standing);
            // A retired one not held has no card (nor a place on the wall).
            if kind.retired && standing.is_none_or(|standing| !standing.listed()) {
                return None;
            }
            return Some(achievement_card(kind, standing));
        }
        if let Some(row) = within(SHADER_BASE, self.shader_count) {
            let skin = self.skin_at(row);
            // Never the card of a shader the player may not see (an unheld Unique one; the
            // list leaves it out already).
            if skin.is_some_and(|skin| !inputs.holdings.shows(skin)) {
                return None;
            }
            return Some(shader_card(skin, &inputs.holdings, &self.marks));
        }
        if self.tab == Tab::Shaders {
            let chosen = self
                .skin_at(self.shader)
                .map(|skin| (skin.id, self.shader_list_owned(skin.id)));
            let which = match token {
                FAVOURITE_TOKEN => Some(MarkToken::Favourite),
                LOCK_TOKEN => Some(MarkToken::Lock),
                FILTER_FAVOURITE_TOKEN => Some(MarkToken::Favourites),
                SEEN_ALL_TOKEN => Some(MarkToken::SeenAll),
                _ => None,
            };
            if let Some(which) = which {
                return mark_card(which, &self.marks, chosen);
            }
            if token == SHADER_WISH_TOKEN {
                return self
                    .shader_wish
                    .map(|(_, on, full)| wish_card(on, full, true));
            }
        }
        if token == TOY_TOKEN {
            return Some(Card {
                title: "Illuminate holocron".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Light it here, from the quick wheel's Toys \
                         page or with a key bound to toy_illuminate.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == BOX_TOKEN {
            return Some(Card {
                title: "Cardboard box".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Get in here, from the quick wheel's Toys \
                         page or with a key bound to toy_box.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == RAGDOLL_TOKEN {
            return Some(Card {
                title: "Ragdoll".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Go limp here, from the quick wheel's Toys \
                         page or with a key bound to toy_ragdoll; press it again to get up.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == DRONE_TOKEN {
            return Some(Card {
                title: "Drone".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it: freecam and peek fly it. Turn it on here, \
                         from the quick wheel's Toys page or with a key bound to toy_drone, \
                         to let others peek at you.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == BOOMBOX_TOKEN {
            return Some(Card {
                title: "Boombox".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every verified SJK player has it. Carry it here, from the quick \
                         wheel's Toys page or with a key bound to toy_boombox; the music is \
                         yours, sound/boombox/<name>.mp3 in a pk3.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == BOX_HUNT_TOKEN {
            return Some(Card {
                title: "Box hunt".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every 10 minutes one player with Box hunt on becomes the mark. Kill \
                         the mark to win a free mystery box roll. Only players with it on see \
                         the mark or can be it.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == BANANA_TOKEN {
            return Some(Card {
                title: "Banana peel".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Turn it on here or with a key bound to \
                         toy_banana, then drop peels with your drop key. Players who have it \
                         on slip on the ones they walk onto.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == QUESTS_TOKEN {
            return Some(Card {
                title: "Quests".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Turn it on here, from the quick wheel's Toys \
                         page or with a key bound to toy_quests: now and then you may be given \
                         a quest, such as hugging two players within a minute, for an Echo.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == FISHING_TOKEN {
            return Some(Card {
                title: "Fishing".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Get the rod out here, from the quick \
                         wheel's Toys page or with a key bound to toy_fishing, and stand \
                         still: water ahead, or a pond where there is none.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == MONITOR_TOKEN {
            return Some(Card {
                title: "Monitor".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Turn it on here, from the quick wheel's Toys \
                         page or with a key bound to toy_monitor, then throw (hold the throw key \
                         to charge) at players who have it on too.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == HAX_TOKEN {
            return Some(Card {
                title: "The Hax".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Turn it on here, from the quick wheel's Toys \
                         page or with a key bound to toy_hax; hold Use on a player who has it \
                         on to hack them.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if token == ORDER_TOKEN {
            return Some(Card {
                title: "Order 66".to_owned(),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "Every SJK player has it. Turn it on here, from the quick wheel's Toys \
                         page or with a key bound to toy_order66; order66 executes it.",
                    ),
                    Line::status(crate::editions::name(crate::editions::ALPHA)),
                    Line::held("Yours"),
                ],
            });
        }
        if let Some(index) = within(CARD_BASE, self.cards_shown) {
            return self.card_tiles.get(index).map(tile_card);
        }
        None
    }
}

/// A medal's card: how the SJK team gives it, the blade it brings, and whether the
/// player holds it.
fn medal_card(showing: &medals::Showing) -> Card {
    let medal = showing.medal;
    let mut lines = vec![Line::how(crate::terms::say(medal.how_to_get()))];
    if let Some(blade) = medal.blade().and_then(crate::unlockables::find) {
        lines.push(Line::status(format!(
            "Brings the {} while you hold it.",
            blade.name
        )));
    }
    lines.push(Line::status(crate::editions::name(medal.edition())));
    lines.push(match &showing.award {
        Some(award) if award.given().is_empty() => Line::held("Yours"),
        Some(award) => Line::held(format!("Yours: {}", award.given().to_lowercase())),
        None => Line::status("Not given to you yet"),
    });
    Card {
        title: medal.name().to_owned(),
        colour: color::GOLD_BRIGHT,
        lines,
    }
}

/// An achievement's card: what to do, who counts it, how far the player is, and the kyber
/// crystal it brings, in the crystal's colour ([`crate::kyber::reward`]). A secret not
/// unlocked yet keeps its name, what it asks and the count to itself; a retired one says
/// it is no longer obtainable; one reached here says it waits for the SJK hub
/// ([`crate::achievements::Standing::waiting`]).
/// The Toy Master's "?" (`Panel::toy_master_badge`): what it is, and the toys the player
/// found, by their achievements' names (the hub names them only to whoever found them).
fn toy_master_card(standings: &[crate::achievements::Standing]) -> Card {
    let found: Vec<&str> = super::toys::toy_master_found(standings)
        .map(|standing| standing.kind.name)
        .collect();
    let mut lines = vec![
        Line::how("The Toy Master hides toys on maps. Find one and press Use on it."),
        Line::status("Each one found is a secret achievement."),
    ];
    if found.is_empty() {
        lines.push(Line::status("None found yet."));
    } else {
        lines.push(Line::held(format!("Found: {}", found.join(", "))));
    }
    Card {
        title: "???".to_owned(),
        colour: crate::achievements::medallion::tint(crate::achievements::Category::ToyMaster),
        lines,
    }
}

fn achievement_card(
    kind: &'static crate::achievements::Kind,
    standing: Option<&crate::achievements::Standing>,
) -> Card {
    let done = standing.is_some_and(|standing| standing.unlocked.is_some());
    let secret = kind.hidden && !done;
    let counted = match kind.source {
        _ if kind.retired => crate::achievements::RETIRED_NOTE,
        Source::Client(_) => "Counted by SJK in your matches on servers.",
        Source::Hub => "Counted by the SJK hub.",
    };
    let counted = format!("{counted} {}.", crate::editions::name(kind.edition));
    let status = match standing {
        Some(standing) => match standing.unlocked {
            Some(at) if at > 0 => Line::held(format!("Unlocked {}", crate::medals::date_text(at))),
            Some(_) => Line::held("Unlocked"),
            None if standing.waiting => Line::status(crate::achievements::WAITING_NOTE),
            None if kind.goal > 1 && !secret => Line::status(format!(
                "{} of {} so far",
                kind.amount(standing.progress.min(kind.goal)),
                kind.amount(kind.goal)
            )),
            None => Line::status("Not unlocked yet"),
        },
        None => Line::status("Not unlocked yet"),
    };
    let (title, how) = if secret {
        (
            crate::achievements::SECRET_NAME,
            crate::achievements::SECRET_DESCRIPTION,
        )
    } else {
        (kind.name, kind.description)
    };
    let (title, how) = (crate::terms::say(title), crate::terms::say(how));
    Card {
        title: title.into_owned(),
        colour: if done {
            color::GOLD_BRIGHT
        } else {
            crate::achievements::medallion::tint(kind.category)
        },
        lines: [Line::how(how), Line::status(counted), status]
            .into_iter()
            .chain(crate::kyber::reward(kind.id).map(|tier| Line {
                words: crate::kyber::reward_line(tier).to_owned(),
                colour: tier.colour,
            }))
            .collect(),
    }
}

/// `words` with how many are held after it when there are several ("Yours, 2 held").
fn held_count(words: &str, count: u32) -> String {
    if count > 1 {
        format!("{words}, {count} held")
    } else {
        words.to_owned()
    }
}

/// A shader's card: the stock blade (`None`) or a blade skin, how it is obtained, and
/// whether the player holds it.
pub(super) fn shader_card(
    skin: Option<&'static crate::unlockables::Unlockable>,
    holdings: &crate::unlockables::Holdings<'_>,
    marks: &crate::shader_marks::Marks,
) -> Card {
    let Some(skin) = skin else {
        return Card {
            title: "Stock blade".to_owned(),
            colour: color::HOLO,
            lines: vec![
                Line::how("Every player has it: the plain blade, in your saber's colour."),
                Line::held("Yours"),
            ],
        };
    };
    let status = match (holdings.unlock(skin.id), holdings.reason()) {
        (Some(unlock), _) if crate::debug_collectibles::added(unlock) => {
            Line::held(crate::debug_collectibles::NOTE.to_owned())
        }
        (Some(unlock), _) => match crate::medals::date_text(unlock.granted) {
            date if date.is_empty() => Line::held(held_count("Yours", unlock.count)),
            date => Line::held(held_count(&format!("Yours since {date}"), unlock.count)),
        },
        (None, Some(reason)) => Line::status(reason),
        (None, None) => Line::status("Not yours yet: you can still try it on here."),
    };
    // Its marks after (29/10/2026): worn, New, the star, the padlock, each with what it
    // means.
    let marked = crate::shader_marks::hover_lines(marks, skin.id)
        .into_iter()
        .map(Line::held);
    Card {
        title: skin.name.to_owned(),
        colour: skin.tier.colour(),
        lines: vec![
            Line::how(crate::terms::say(skin.how_to_get)),
            // The blade number only while `debug_collectibles` is on (Sol, 21/10/2026).
            Line::status(if holdings.debug() {
                format!(
                    "Blade {}. {} blade skin. {}.",
                    skin.number_text(),
                    skin.tier.label(),
                    crate::editions::name(skin.edition)
                )
            } else {
                format!(
                    "{} blade skin. {}.",
                    skin.tier.label(),
                    crate::editions::name(skin.edition)
                )
            }),
            Line::status(format!("Saber smith: {}", skin.smith)),
            status,
        ]
        .into_iter()
        .chain(marked)
        .collect(),
    }
}

/// The hover card of a wishlist toggle (10/11/2026): what the wishlist is, and what a click
/// does now: the shader is on it (`on`), the list is `full`, or the shader `may` go on it
/// (one of the trade the player lacks).
pub(super) fn wish_card(on: bool, full: bool, may: bool) -> Card {
    Card {
        title: format!("{}  Wishlist", crate::symbols::GIFT),
        colour: color::GOLD_BRIGHT,
        lines: vec![
            Line::how(format!(
                "Up to {} saber shaders you lack, kept on the SJK hub. A wished shader in your \
                 shop's offers is marked, and a card tells you once a day.",
                crate::wishlist::MAX
            )),
            if on {
                Line::held("On your wishlist: a click takes it off.")
            } else if full {
                Line::status(crate::wishlist::full_words())
            } else if may {
                Line::held("A click adds it.")
            } else {
                Line::status("Only a shader of the trade you lack can be wished for.")
            },
        ],
    }
}

/// Which of the marks' controls a hover card is for (29/10/2026).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MarkToken {
    /// The chosen shader's star.
    Favourite,
    /// The chosen shader's padlock.
    Lock,
    /// The favourites-only chip.
    Favourites,
    /// Mark all seen.
    SeenAll,
}

/// The hover card of a marks' control: what the star, the padlock, the chip or Mark all
/// seen does, for the shader `chosen` (its id, and whether the player holds it); `None`
/// while the hub keeps no marks (the controls do not show), but for Mark all seen.
pub(super) fn mark_card(
    which: MarkToken,
    marks: &crate::shader_marks::Marks,
    chosen: Option<(&str, bool)>,
) -> Option<Card> {
    if !marks.kept && which != MarkToken::SeenAll {
        return None;
    }
    let card = match which {
        MarkToken::Favourite => {
            let on = chosen.is_some_and(|(id, _)| marks.favourite(id));
            Card {
                title: format!("{}  Favourite", crate::symbols::EDITION),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "A star: your favourites are listed first, and the Favourites chip \
                         (Shift+F) shows them alone. Kept on the SJK hub, on every PC you \
                         play on.",
                    ),
                    Line::held(if on {
                        "A favourite: F or a click unstars it."
                    } else {
                        "F or a click stars it."
                    }),
                ],
            }
        }
        MarkToken::Lock => {
            let (on, owned) =
                chosen.map_or((false, false), |(id, owned)| (marks.locked(id), owned));
            Card {
                title: format!("{}  Lock", crate::symbols::LOCK),
                colour: color::GOLD_BRIGHT,
                lines: vec![
                    Line::how(
                        "A padlock: a locked shader is never broken down in the Forge (not \
                         one copy, not with all duplicates) nor offered in a trade until you \
                         unlock it. The SJK hub holds the lock.",
                    ),
                    if on {
                        Line::held("Locked: L or a click unlocks it.")
                    } else if owned {
                        Line::held("L or a click locks it.")
                    } else {
                        Line::status("Only a shader you hold can be locked.")
                    },
                ],
            }
        }
        MarkToken::Favourites => Card {
            title: "Favourites".to_owned(),
            colour: color::GOLD_BRIGHT,
            lines: vec![
                Line::how("Shows your starred shaders alone (Shift+F); again shows them all."),
                Line::status("Star a shader with F, or its star beside the chosen one."),
            ],
        },
        MarkToken::SeenAll => Card {
            title: "New".to_owned(),
            colour: color::GOLD_BRIGHT,
            lines: vec![
                Line::how(
                    "Shaders you got since you last looked (holocrons, gifts, trades, \
                     crafts) carry a New sticker until you choose one, here or in the Forge.",
                ),
                Line::status("Kept on this PC only. Mark all seen (N) clears them all."),
            ],
        },
    };
    Some(card)
}

/// A tile's card on the Cards tab: what the card is, how it is obtained and the odds,
/// then the copies held by finish and edition and when the first was found, or that it
/// is not found yet.
fn tile_card(tile: &crate::cards::binder::Tile) -> Card {
    use crate::cards::binder::Slot;
    let faces = crate::cards::faces::current();
    let odds = match tile.face(&faces) {
        Some(face) if face.rank.is_some() => cards::card_odds(face, &faces),
        _ => cards::rarity_odds(tile.rarity, &faces),
    };
    let mut lines = Vec::with_capacity(4);
    match (&tile.slot, tile.face(&faces)) {
        (_, Some(face)) => {
            lines.push(Line::how(odds));
            // Each perk after its mark, as the tile's badges show it.
            for perk in face.perks() {
                lines.push(Line::status(format!("{}  {}", perk.mark, perk.words)));
            }
            // No number, whatever the kind (Sol, 20/10/2026): it stays the card's id.
            lines.push(Line::status(format!(
                "{} card. {}, {}.",
                face.tone().name,
                face.kind.name(),
                face.set_name
            )));
        }
        (Slot::Sealed, _) => {
            lines.push(Line::how(
                "Sealed: a card of this rarity whose face is picked when it is revealed,                  into the first set that has cards of it.",
            ));
            lines.push(Line::status(odds));
        }
        _ => {
            lines.push(Line::how("Its face is not in your card pack yet."));
            lines.push(Line::status(format!("{} card.", tile.rarity.name)));
        }
    }
    if tile.owned() {
        lines.push(Line::held(tile.breakdown()));
        let date = crate::medals::date_text(tile.first());
        if !date.is_empty() {
            lines.push(Line::status(format!("First found {date}")));
        }
    } else {
        lines.push(Line::status("Not found yet"));
    }
    Card {
        title: tile.name(&faces),
        colour: if tile.owned() {
            tile.tone(&faces).colour
        } else {
            color::MUTED
        },
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_medal_says_how_it_is_given() {
        for medal in crate::medals::Medal::ALL {
            let card = medal_card(&medals::Showing { medal, award: None });
            assert!(card.lines[0].words.starts_with("Given by the SJK team"));
            let names_a_blade = card.lines.iter().any(|line| line.words.contains("blade"));
            assert_eq!(names_a_blade, medal.blade().is_some(), "{medal:?}");
            assert_eq!(card.lines.last().unwrap().words, "Not given to you yet");
        }
    }

    /// A secret's card keeps its name, what it asks and its count until it is unlocked;
    /// a retired one held says it is no longer obtainable.
    #[test]
    fn a_secret_card_says_nothing_until_unlocked_and_a_retired_one_says_so() {
        let kind = crate::achievements::find("mythical_find").unwrap();
        let locked = crate::achievements::Standing {
            kind,
            progress: 0,
            unlocked: None,
            waiting: false,
        };
        let card = achievement_card(kind, Some(&locked));
        assert_eq!(card.title, crate::achievements::SECRET_NAME);
        assert_eq!(card.lines[0].words, crate::achievements::SECRET_DESCRIPTION);
        assert_eq!(card.lines[2].words, "Not unlocked yet");
        assert!(
            card.lines
                .iter()
                .all(|line| !line.words.contains("Mythical in"))
        );
        let open = crate::achievements::Standing {
            unlocked: Some(1_791_641_100),
            ..locked
        };
        let card = achievement_card(kind, Some(&open));
        assert_eq!(card.title, "Touched by the Force");
        assert_eq!(card.lines[0].words, kind.description);
        let surveyor = crate::achievements::find("surveyor").unwrap();
        let held = crate::achievements::Standing {
            kind: surveyor,
            progress: 5,
            unlocked: Some(1_791_641_100),
            waiting: false,
        };
        let card = achievement_card(surveyor, Some(&held));
        assert_eq!(card.title, "Surveyor");
        assert!(card.lines[1].words.starts_with("No longer obtainable."));
        assert_eq!(card.lines[2].words, "Unlocked 10/10/2026");
    }

    #[test]
    fn every_achievement_says_what_to_do_and_how_far() {
        for kind in &crate::achievements::ALL {
            let card = achievement_card(kind, None);
            let how = if kind.hidden {
                crate::achievements::SECRET_DESCRIPTION
            } else {
                kind.description
            };
            assert_eq!(card.lines[0].words, how);
            assert_eq!(card.lines.len(), 4);
            let reward = crate::kyber::reward(kind.id).expect("every achievement brings one");
            assert_eq!(card.lines[3].words, format!("Reward: {}", reward.name));
            assert_eq!(card.lines[3].colour, reward.colour);
        }
        let kind = crate::achievements::ALL
            .iter()
            .find(|kind| kind.goal == 100)
            .unwrap();
        let standing = crate::achievements::Standing {
            kind,
            progress: 37,
            unlocked: None,
            waiting: false,
        };
        let card = achievement_card(kind, Some(&standing));
        assert_eq!(card.lines[2].words, "37 of 100 so far");
        let done = crate::achievements::Standing {
            unlocked: Some(1_791_641_100),
            ..standing
        };
        let card = achievement_card(kind, Some(&done));
        assert_eq!(card.lines[2].words, "Unlocked 10/10/2026");
        assert_eq!(card.colour, color::GOLD_BRIGHT);
    }

    #[test]
    fn every_shader_says_how_to_get_it() {
        let none = crate::shader_marks::Marks::default();
        let card = shader_card(None, &crate::unlockables::Holdings::Known(&[]), &none);
        assert_eq!(card.title, "Stock blade");
        for skin in crate::unlockables::blade_skins_by_tier() {
            let card = shader_card(Some(skin), &crate::unlockables::Holdings::Known(&[]), &none);
            assert_eq!(card.lines[0].words, skin.how_to_get);
            assert_eq!(card.lines[2].words, format!("Saber smith: {}", skin.smith));
            assert!(card.lines[3].words.starts_with("Not yours yet"));
            assert_eq!(card.lines.len(), 4, "no marks");
        }
        let held = [sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: String::new(),
            count: 1,
            medal: None,
            card: None,
        }];
        let sun = crate::unlockables::find("saber_sun").unwrap();
        let card = shader_card(
            Some(sun),
            &crate::unlockables::Holdings::Known(&held),
            &none,
        );
        assert!(card.lines[3].words.starts_with("Yours since "), "{card:?}");
        assert_eq!(card.lines[1].words, "Mythical blade skin. Alpha Edition.");
        // Its marks follow, each saying what it means.
        let marks = crate::shader_marks::Marks::shown(
            &["saber_sun"],
            &["saber_sun"],
            &["saber_sun"],
            Some("saber_sun"),
        );
        let card = shader_card(
            Some(sun),
            &crate::unlockables::Holdings::Known(&held),
            &marks,
        );
        let words: Vec<_> = card.lines[4..]
            .iter()
            .map(|line| line.words.as_str())
            .collect();
        assert_eq!(words.len(), 4, "{words:?}");
        assert!(words[0].contains("Worn") && words[1].starts_with("New"));
        assert!(words[2].contains("Favourite") && words[3].contains("Locked"));
        // Debug collectibles on: the blade number shows as well.
        let held = [sjk_identity::Unlock {
            note: crate::debug_collectibles::NOTE.to_owned(),
            ..held[0].clone()
        }];
        let card = shader_card(
            Some(sun),
            &crate::unlockables::Holdings::Known(&held),
            &none,
        );
        assert_eq!(
            card.lines[1].words,
            "Blade 001. Mythical blade skin. Alpha Edition."
        );
    }

    /// The marks' controls explain themselves; without the hub's marks only Mark all seen
    /// has a card, and the padlock says a shader not held cannot be locked.
    #[test]
    fn the_marks_controls_say_what_they_do() {
        let marks = crate::shader_marks::Marks::shown(&["saber_sun"], &[], &[], None);
        let star = mark_card(MarkToken::Favourite, &marks, Some(("saber_sun", true))).unwrap();
        assert!(star.lines[1].words.contains("unstars"));
        let lock = mark_card(MarkToken::Lock, &marks, Some(("saber_storm", false))).unwrap();
        assert!(lock.lines[0].words.contains("never broken down"));
        assert_eq!(lock.lines[1].words, "Only a shader you hold can be locked.");
        let none = crate::shader_marks::Marks::default();
        assert!(mark_card(MarkToken::Favourite, &none, None).is_none());
        assert!(mark_card(MarkToken::Favourites, &none, None).is_none());
        assert!(mark_card(MarkToken::SeenAll, &none, None).is_some());
    }

    #[test]
    fn a_tile_says_where_it_comes_from_and_what_is_held() {
        let stacks = crate::cards::stacks(&[
            sjk_identity::CardStack {
                card: None,
                rarity: "mythical".to_owned(),
                finish: "solar".to_owned(),
                edition: "alpha".to_owned(),
                count: 1,
                first: 1_791_641_100,
                last: 1_791_641_100,
            },
            sjk_identity::CardStack {
                card: None,
                rarity: "mythical".to_owned(),
                finish: "normal".to_owned(),
                edition: "alpha".to_owned(),
                count: 2,
                first: 1_791_700_000,
                last: 1_791_700_000,
            },
        ]);
        let faces = crate::cards::faces::Faces::of_list(Vec::new());
        let tiles = crate::cards::binder::tiles(&stacks, &faces);
        let card = tile_card(&tiles[0]);
        assert_eq!(card.title, "Sealed Mythical cards");
        assert_eq!(
            card.lines[1].words,
            "From holocrons: about 1 in 1 004 holocrons found holds a Mythical card (0.0996%)."
        );
        assert_eq!(
            card.lines[2].words,
            "3 copies: 1 Solar, 2 Normal · Alpha Edition"
        );
        assert_eq!(card.lines[3].words, "First found 10/10/2026");
    }
}
