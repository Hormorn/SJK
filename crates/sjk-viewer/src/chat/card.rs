//! The sender card in the game's chat (`docs/hub-chat.md`, "Muting a player"). While
//! the composer is open the pointer is free, and resting it on a name (an SJK chat
//! sender's, or the name of a player the server says sent a line) shows their card
//! ([`crate::sender_card`]) beside it, with Mute or Unmute. The card stays while the
//! pointer is on it, and for [`GRACE_MS`] after the pointer leaves both, so the way
//! from the name to the card's buttons never loses it. A muted player's lines are
//! hidden as ignored ones are.
//!
//! A click on the name pins the card (Bishop's and Sanjisan's idea, Sol's go 23/10/2026):
//! it stays, whatever the pointer does, until the name is clicked again, a click lands
//! off the card, Escape is pressed or the composer closes. For a player on the server
//! (not the player themselves) the card then heads its buttons with Whisper, which
//! turns the composer to a whisper to them ([`ChatOverlay::whisper_target`]): it goes
//! encrypted through the SJK hub when they take encrypted whispers, else as the
//! server's `tell`, as any whisper from the composer.

use super::*;
use crate::sender_card::{self, Person, Place};
use sjk_ui::Rect;

/// The card's pointer targets, after the composer's.
pub(super) const CARD: u16 = 120;
pub(super) const CARD_MUTE: u16 = 121;
pub(super) const CARD_BAN: u16 = 122;
pub(super) const CARD_SAFETY: u16 = 123;
pub(super) const CARD_WHISPER: u16 = 124;
pub(super) const CARD_REPORT: u16 = 125;
pub(super) const CARD_TRADE: u16 = 126;
pub(super) const CARD_MAP_BLOCK: u16 = 127;
pub(super) const CARD_STAFF_BLOCK: u16 = 128;
/// Every button on the card, which a press must find before the card under them.
pub(super) const BUTTONS: [u16; 8] = [
    CARD_TRADE,
    CARD_WHISPER,
    CARD_REPORT,
    CARD_SAFETY,
    CARD_BAN,
    CARD_MUTE,
    CARD_MAP_BLOCK,
    CARD_STAFF_BLOCK,
];
/// How long the card stays once the pointer is off the name and the card.
pub(super) const GRACE_MS: u64 = 400;

/// Whose name a feed token is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Who {
    /// An SJK chat line's sender, by the message's id at the hub.
    Hub(u64),
    /// A player on the server.
    Player(ChatTarget),
}

/// The card on show: built when the name under the pointer changes, drawn every frame.
pub(super) struct Card {
    pub(super) who: Who,
    pub(super) person: Person,
    pub(super) muted: bool,
    /// The name it is beside.
    pub(super) anchor: Rect,
    /// When the pointer left the name and the card, while it is off both.
    pub(super) left_ms: Option<u64>,
    /// A click on the name pinned it: it stays until dismissed.
    pub(super) pinned: bool,
}

/// A block for this map asked from a card, which the frame carries out
/// (`crate::map_blocks`): `everyone` for staff's block for every SJK player there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MapBlockRequest {
    pub(crate) slot: u8,
    pub(crate) name: String,
    pub(crate) everyone: bool,
    pub(crate) blocked: bool,
}

/// Mute or unmute asked from a card, which the frame carries out
/// (`crate::player_mutes::set_muted`): local only, nothing is sent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MuteRequest {
    pub(crate) key_id: Option<String>,
    pub(crate) name: String,
    pub(crate) muted: bool,
}

impl ChatOverlay {
    /// After the feed is laid out: show the card of the name under the pointer, keep
    /// it while the pointer is on the card, else hide it. Not while the composer is
    /// closed (`active` false).
    pub(super) fn follow_card(&mut self, active: bool, ms: u64) {
        if !active {
            self.card = None;
            return;
        }
        // A name hovered as this frame lays it out: the canvas keeps the widget that was
        // under the pointer when it last moved, which is another one once the feed
        // changed (a chat tab chosen with the pointer still on its button).
        let pointer = self.pointer_at;
        let hovered = (0..MAX_VISIBLE).find_map(|token| {
            let who = self.visible_people[token]?;
            let rect = self.ui.rect_for(token as u16);
            (self.ui.token_hovered(token as u16)
                && rect.zip(pointer).is_none_or(|(rect, at)| rect.contains(at)))
            .then_some((who, rect))
        });
        // A pinned card stays, following its name while the name is in view.
        if let Some(card) = self.card.as_mut().filter(|card| card.pinned) {
            if let Some((_, anchor)) = hovered.filter(|(who, _)| *who == card.who) {
                card.anchor = anchor.unwrap_or(card.anchor);
            } else if let Some(token) =
                (0..MAX_VISIBLE).find(|token| self.visible_people[*token] == Some(card.who))
                && let Some(anchor) = self.ui.rect_for(token as u16)
            {
                card.anchor = anchor;
            }
            card.left_ms = None;
        } else {
            self.hover_card(hovered, ms);
        }
        // Their picture's version, until the hub's players or their profile say it.
        if let Some(card) = self
            .card
            .as_mut()
            .filter(|card| card.person.avatar.is_none())
            && let Some(key_id) = &card.person.key_id
            && let Some((avatar, banner)) = crate::player_identity::card_appearance(key_id)
        {
            card.person.avatar = Some(avatar);
            card.person.banner = banner;
        }
    }

    /// The card follows the pointer: the name under it, or the card itself.
    fn hover_card(&mut self, hovered: Option<(Who, Option<Rect>)>, ms: u64) {
        match hovered {
            Some((who, anchor)) => {
                let anchor = anchor.unwrap_or_default();
                match &mut self.card {
                    Some(card) if card.who == who => {
                        card.anchor = anchor;
                        card.left_ms = None;
                    }
                    _ => {
                        self.card = self.person(who).map(|(person, muted)| Card {
                            who,
                            person,
                            muted,
                            anchor,
                            left_ms: None,
                            pinned: false,
                        });
                    }
                }
            }
            None if std::iter::once(CARD)
                .chain(BUTTONS)
                .any(|token| self.ui.token_hovered(token)) =>
            {
                if let Some(card) = &mut self.card {
                    card.left_ms = None;
                }
            }
            // Off both: it waits a moment for the pointer on its way to the card.
            None => {
                if let Some(card) = &mut self.card {
                    let left = *card.left_ms.get_or_insert(ms);
                    if ms.saturating_sub(left) >= GRACE_MS {
                        self.card = None;
                    }
                }
            }
        }
    }

    /// A click on the name in feed token `token`, whose sender the press began on
    /// (`pressed`): pin their card, or unpin it when it is pinned already.
    pub(super) fn click_name(&mut self, token: usize, pressed: Option<Who>) {
        let Some(who) = self.visible_people.get(token).copied().flatten() else {
            return;
        };
        // A new line can move the names between press and release: never pin another.
        if pressed != Some(who) {
            return;
        }
        let shown = self
            .card
            .as_mut()
            .filter(|card| card.who == who)
            .map(|card| std::mem::replace(&mut card.pinned, true));
        match shown {
            // Pinned already: the click unpins it.
            Some(true) => self.card = None,
            Some(false) => {}
            None => {
                let anchor = self.ui.rect_for(token as u16).unwrap_or_default();
                self.card = self.person(who).map(|(person, muted)| Card {
                    who,
                    person,
                    muted,
                    anchor,
                    left_ms: None,
                    pinned: true,
                });
            }
        }
    }

    /// Drop a pinned card (Escape, a click elsewhere). False when none is pinned.
    pub(super) fn unpin_card(&mut self) -> bool {
        if self.card.as_ref().is_some_and(|card| card.pinned) {
            self.card = None;
            return true;
        }
        false
    }

    /// Who Whisper on a card goes to: the player in the slot, still the one the line
    /// named (an SJK chat sender by the slot their key is matched to here), and never
    /// the player themselves. `None` when they left, renamed or are not on the server.
    pub(super) fn whisper_target(&self, who: Who, place: Place) -> Option<ChatTarget> {
        let target = match who {
            Who::Player(target) => target,
            Who::Hub(_) => match place {
                Place::Slot(slot) | Place::SlotByName(slot) => {
                    self.roster.target(Some(u16::from(slot)))?
                }
                Place::Unknown | Place::Away => return None,
            },
        };
        (self.roster.name(target).is_some() && Some(target.slot()) != self.own_slot)
            .then_some(target)
    }

    /// Who `who` is, from the line or the server and what the hub and the mute list
    /// say, and whether they are muted.
    fn person(&self, who: Who) -> Option<(Person, bool)> {
        let person = match who {
            Who::Hub(id) => {
                let (line, hub) = self.lines.iter().find_map(|line| {
                    line.hub
                        .as_ref()
                        .filter(|hub| hub.id == id)
                        .map(|hub| (line, hub))
                })?;
                let place = crate::player_mutes::place(Some(&hub.key_id), &line.name);
                let mark = match place {
                    Place::Slot(slot) | Place::SlotByName(slot) => {
                        crate::player_identity::hub_mark(slot, self.player_label(u16::from(slot)))
                    }
                    _ => None,
                };
                Person {
                    name: line.name.clone(),
                    key_id: Some(hub.key_id.clone()),
                    hub_name: None,
                    verified: hub.verified,
                    tick: hub.tick,
                    staff: hub.staff,
                    creator: hub.creator,
                    trust: hub.trust,
                    emoji: hub.emoji.clone(),
                    banner: "",
                    avatar: mark.as_ref().map(|mark| mark.avatar.clone()),
                    medals: mark.map(|mark| mark.medals).unwrap_or_default(),
                    place,
                    note: None,
                    safety: false,
                    report: false,
                    // On this server and not blocked here: it can be asked to trade.
                    trade: matches!(place, Place::Slot(_) | Place::SlotByName(_))
                        && !crate::player_trades::blocked(&hub.key_id, &line.name)
                        && crate::player_trades::trading_open(),
                    map_block: None,
                }
            }
            Who::Player(target) => {
                let name = self.roster.display_name(target)?;
                let slot = u8::try_from(target.slot()).ok()?;
                // The hub's claim on the slot, under the name shown there, says whose
                // key it is (the badges' rule); without one they are muted by name.
                let mark = crate::player_identity::hub_mark(slot, name);
                Person {
                    name: name.to_owned(),
                    key_id: mark.as_ref().map(|mark| mark.key_id.clone()),
                    hub_name: mark
                        .as_ref()
                        .map(|mark| mark.name.clone())
                        .filter(|hub| !sjk_identity::names_match(hub, name)),
                    verified: mark.as_ref().is_some_and(|mark| mark.verified),
                    tick: mark
                        .as_ref()
                        .map_or(sjk_identity::Tick::None, |mark| mark.tick),
                    staff: false,
                    creator: false,
                    trust: mark.as_ref().and_then(|mark| mark.trust),
                    emoji: mark
                        .as_ref()
                        .map(|mark| mark.emoji.clone())
                        .unwrap_or_default(),
                    place: if mark.is_some() {
                        Place::Slot(slot)
                    } else {
                        Place::SlotByName(slot)
                    },
                    safety: mark.is_some() && self.takes_encrypted(target.slot()),
                    report: mark.is_some() && self.reportable(target.slot()),
                    trade: mark
                        .as_ref()
                        .is_some_and(|mark| !crate::player_trades::blocked(&mark.key_id, name))
                        && crate::player_trades::trading_open(),
                    banner: "",
                    avatar: mark.as_ref().map(|mark| mark.avatar.clone()),
                    medals: mark.map(|mark| mark.medals).unwrap_or_default(),
                    note: None,
                    map_block: None,
                }
            }
        };
        let mut person = person;
        if let Place::Slot(slot) | Place::SlotByName(slot) = person.place {
            person.map_block = Some(self.map_block_row(slot));
        }
        let muted = crate::player_mutes::is_muted(person.key_id.as_deref(), &person.name);
        Some((person, muted))
    }

    /// Draw the card on show beside its name, its targets over the feed.
    pub(super) fn build_card(&mut self, font: &UiFont, g: &layout::Geometry, viewport: [f32; 2]) {
        let Some(card) = &self.card else {
            return;
        };
        let measure = crate::sjk_chat_look::Measure::new(font, crate::text::TextStyle::NEUTRAL);
        let u = g.scale * self.options.card;
        let whisper = self
            .whisper_target(card.who, card.person.place)
            .map(|_| CARD_WHISPER);
        let size = sender_card::size(&card.person, whisper.is_some(), u);
        let origin = sender_card::beside(card.anchor, size, viewport, 4.0 * u);
        // Text draws over every shape: the feed's text under the card is left out, or
        // the messages showed through it (Sol, 18/10/2026).
        self.ui
            .cover(Rect::new(origin[0], origin[1], size[0], size[1]));
        sender_card::draw(
            &mut self.ui,
            &sender_card::Card {
                person: &card.person,
                muted: card.muted,
                measure: Some(&measure),
            },
            origin,
            u,
            sender_card::Tokens {
                card: CARD,
                mute: CARD_MUTE,
                ban: CARD_BAN,
                safety: CARD_SAFETY,
                report: CARD_REPORT,
                whisper,
                trade: CARD_TRADE,
                map_block: CARD_MAP_BLOCK,
                staff_block: CARD_STAFF_BLOCK,
            },
        );
    }

    /// Whisper on the card: the composer turns to a whisper to them (encrypted when
    /// they take it, `chat/interaction.rs`'s send), the draft kept, the card closed.
    pub(super) fn press_card_whisper(&mut self) {
        let Some(card) = &self.card else {
            return;
        };
        let target = self.whisper_target(card.who, card.person.place);
        self.card = None;
        let Some(input) = &mut self.input else {
            return;
        };
        match target {
            Some(target) => {
                input.channel = Channel::Whisper;
                input.recipient = Some(target);
                self.notice = "";
            }
            None => self.notice = "Player left or changed identity. Choose a name again.",
        }
    }

    /// The card's blocks for this map of the player in `slot`, as the frame last said
    /// ([`ChatOverlay::set_map_blocks`]).
    fn map_block_row(&self, slot: u8) -> sender_card::MapBlockRow {
        let bit = 1u32.checked_shl(u32::from(slot)).unwrap_or(0);
        sender_card::MapBlockRow {
            own: self.map_blocks.0 & bit != 0,
            staff: self.map_blocks.2.then_some(self.map_blocks.1 & bit != 0),
        }
    }

    /// Block for this map (`everyone` false) or for everyone (staff) on the card: ask
    /// the frame, which works the card's row out again next frame.
    pub(super) fn press_card_map_block(&mut self, everyone: bool) {
        let Some(card) = &mut self.card else {
            return;
        };
        let (Some(row), Place::Slot(slot) | Place::SlotByName(slot)) =
            (&mut card.person.map_block, card.person.place)
        else {
            return;
        };
        let blocked = if everyone {
            let Some(staff) = &mut row.staff else {
                return;
            };
            *staff = !*staff;
            *staff
        } else {
            row.own = !row.own;
            row.own
        };
        self.map_block_request = Some(MapBlockRequest {
            slot,
            name: card.person.name.clone(),
            everyone,
            blocked,
        });
    }

    /// The frame's map blocks: the slots the player blocked for the map, the ones staff
    /// blocked for everyone, and whether the player is staff.
    pub(crate) fn set_map_blocks(&mut self, own: u32, staff: u32, is_staff: bool) {
        self.map_blocks = (own, staff, is_staff);
    }

    /// What a card asked for this map, once.
    pub(crate) fn take_map_block_request(&mut self) -> Option<MapBlockRequest> {
        self.map_block_request.take()
    }

    /// Mute or Unmute on the card: ask the frame to change the list.
    pub(super) fn toggle_card_mute(&mut self) {
        let Some(card) = &mut self.card else {
            return;
        };
        card.muted = !card.muted;
        self.mute_request = Some(MuteRequest {
            key_id: card.person.key_id.clone(),
            name: card.person.name.clone(),
            muted: card.muted,
        });
    }

    /// Ban 24 h on the card ([`crate::chat_ban`]): arms it, or sends the ban.
    pub(super) fn press_card_ban(&self) {
        if let Some(key_id) = self
            .card
            .as_ref()
            .and_then(|card| card.person.key_id.as_deref())
        {
            crate::chat_ban::press(key_id);
        }
    }

    /// Safety code on the card: ask the frame to show it.
    pub(super) fn press_card_safety(&mut self) {
        if let Some(Card {
            who: Who::Player(target),
            ..
        }) = &self.card
        {
            self.safety_request = Some(target.slot());
            self.card = None;
        }
    }

    /// Report on the card: ask the frame to open the report, their whisper with it.
    pub(super) fn press_card_report(&mut self) {
        if let Some(Card {
            who: Who::Player(target),
            ..
        }) = &self.card
        {
            self.report_request = Some(target.slot());
            self.card = None;
        }
    }

    /// Trade on the card: ask the frame to ask them (the slot they are in).
    pub(super) fn press_card_trade(&mut self) {
        let slot = match self.card.as_ref().map(|card| (card.who, card.person.place)) {
            Some((Who::Player(target), _)) => Some(target.slot()),
            Some((_, Place::Slot(slot) | Place::SlotByName(slot))) => Some(u16::from(slot)),
            _ => None,
        };
        if let Some(slot) = slot {
            self.trade_request = Some(slot);
            self.card = None;
        }
    }

    /// What a card asked, once.
    pub(crate) fn take_mute_request(&mut self) -> Option<MuteRequest> {
        self.mute_request.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{ChatMessage, ChatState};
    use sjk_ui::{InputEvent, PointerButton, Vec2};

    const VIEWPORT: [f32; 2] = [1920.0, 1080.0];
    /// A key no other test mutes: the mute list is the process's own.
    const KEY: &str = "c0ffee00c0ffee01";

    fn centre(rect: Rect) -> Vec2 {
        Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }

    /// The composer open over one SJK line from `KEY` and one unattributed game line.
    fn chat() -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let state = |messages| ChatState {
            messages,
            loaded: Some(1),
            ..ChatState::default()
        };
        chat.sync_sjk(&state(Default::default()), |_| false, now);
        chat.receive(ServerEventKind::Chat, "Fox: hi".to_owned(), None, now);
        let message = ChatMessage {
            reactions: Default::default(),
            id: 7,
            at: 0,
            key_id: KEY.to_owned(),
            name: "^2Sol".to_owned(),
            verified: true,
            tick: sjk_identity::Tick::None,
            staff: true,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: "gg all".to_owned(),
            holocron: None,
            official: None,
            channel: Default::default(),
        };
        chat.sync_sjk(&state([message].into_iter().collect()), |_| false, now);
        chat.open(false);
        chat
    }

    fn build(chat: &mut ChatOverlay) {
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000);
    }

    fn texts(chat: &ChatOverlay) -> Vec<String> {
        chat.ui.text_runs().map(str::to_owned).collect()
    }

    /// The text of each line a click would translate (`chat/translate.rs`).
    fn offered(chat: &ChatOverlay) -> Vec<(String, Rect)> {
        chat.visible_bodies
            .iter()
            .flatten()
            .map(|&(index, _, rect)| (chat.lines[index].body.clone(), rect))
            .collect()
    }

    #[test]
    fn a_click_on_a_lines_text_translates_it_and_one_on_its_name_opens_the_card() {
        let mut chat = chat();
        // This client's own notice is never offered.
        chat.notice("^3SJK chat: not sent".to_owned(), Instant::now());
        build(&mut chat);
        let bodies = offered(&chat);
        let texts: Vec<&str> = bodies.iter().map(|(body, _)| body.as_str()).collect();
        // The server's unattributed line and the SJK line, not the notice.
        assert_eq!(texts.len(), 2, "{texts:?}");
        assert!(texts.contains(&"gg all") && !texts.iter().any(|text| text.contains("not sent")));
        let (_, sjk) = bodies
            .iter()
            .find(|(body, _)| body == "gg all")
            .unwrap()
            .clone();
        click(&mut chat, centre(sjk));
        let asked = chat.take_translate_click().expect("the click asks");
        assert_eq!(asked.text, "gg all");
        assert!(chat.card.is_none(), "the text is no name");
        // Asked once: the line is not offered again.
        build(&mut chat);
        assert!(!offered(&chat).iter().any(|(body, _)| body == "gg all"));
        // The name still opens the sender card, and asks nothing.
        let token = (0..MAX_VISIBLE)
            .find(|token| chat.visible_people[*token] == Some(Who::Hub(7)))
            .expect("the SJK line's name");
        let name = chat.ui.rect_for(token as u16).expect("the name's target");
        chat.pointer(InputEvent::PointerMove(centre(name)));
        build(&mut chat);
        click(&mut chat, centre(name));
        assert!(chat.card.is_some());
        assert!(chat.take_translate_click().is_none());
    }

    #[test]
    fn own_lines_translations_and_a_switched_off_chat_offer_nothing() {
        let mut chat = chat();
        chat.set_sjk_own_key(Some(KEY));
        build(&mut chat);
        let texts: Vec<String> = offered(&chat).into_iter().map(|(body, _)| body).collect();
        assert_eq!(
            texts,
            ["Fox: hi"],
            "the player's own SJK line is not offered"
        );
        // A translation under a line is not offered either.
        chat.notice("^9(fr) hi".to_owned(), Instant::now());
        build(&mut chat);
        assert_eq!(offered(&chat).len(), 1);
        chat.set_translate_on(false);
        build(&mut chat);
        assert!(offered(&chat).is_empty(), "cl_chatTranslate 0");
    }

    #[test]
    fn resting_on_a_name_shows_the_sender_card_with_mute() {
        let mut chat = chat();
        build(&mut chat);
        assert!(chat.card.is_none());
        // Only the SJK line's name is a target: the game line names nobody.
        let names: Vec<_> = (0..MAX_VISIBLE)
            .filter_map(|token| chat.visible_people[token].zip(Some(token)))
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        let (who, token) = names[0];
        assert_eq!(who, Who::Hub(7));
        let name = chat.ui.rect_for(token as u16).expect("the name's target");
        chat.pointer(InputEvent::PointerMove(centre(name)));
        build(&mut chat);
        let card = chat.card.as_ref().expect("the card");
        assert_eq!(card.person.key_id.as_deref(), Some(KEY));
        assert!(card.person.verified && card.person.staff && !card.muted);
        let texts = texts(&chat);
        for wanted in ["^2Sol", "SJK player, SJK staff", "Block"] {
            assert!(
                texts.iter().any(|text| text == wanted),
                "{wanted:?} in {texts:?}"
            );
        }
        let drawn = chat.ui.rect_for(CARD).expect("the card's target");
        assert!(drawn.x >= name.right(), "beside the name");
        // Onto the card and its Mute: it stays, and a click asks for the mute.
        let mute = chat.ui.rect_for(CARD_MUTE).expect("Mute");
        chat.pointer(InputEvent::PointerMove(centre(mute)));
        build(&mut chat);
        assert!(chat.card.is_some(), "kept while the pointer is on it");
        let mute = chat.ui.rect_for(CARD_MUTE).expect("Mute");
        let (position, button) = (centre(mute), PointerButton::Primary);
        chat.pointer(InputEvent::PointerPress { position, button });
        chat.pointer(InputEvent::PointerRelease { position, button });
        assert_eq!(
            chat.take_mute_request(),
            Some(MuteRequest {
                key_id: Some(KEY.to_owned()),
                name: "^2Sol".to_owned(),
                muted: true,
            })
        );
        assert_eq!(chat.take_mute_request(), None, "once");
        assert!(
            chat.card.as_ref().is_some_and(|card| card.muted),
            "now Unblock"
        );
        // Away from both: it waits a moment for the pointer, then goes. Closed, the
        // feed has no targets and no card.
        chat.pointer(InputEvent::PointerMove(Vec2::new(1_900.0, 10.0)));
        build(&mut chat);
        assert!(chat.card.is_some(), "kept on the way to the card");
        chat.build(true, &crate::text::test_font(), VIEWPORT, 1_000 + GRACE_MS);
        assert!(chat.card.is_none());
        chat.input = None;
        build(&mut chat);
        assert!(chat.card.is_none() && chat.visible_people.iter().all(Option::is_none));
    }

    #[test]
    fn a_muted_slots_lines_are_hidden_and_shown_again() {
        let mut chat = ChatOverlay::new();
        let mut game = sjk_protocol::GameState::empty_local(0);
        game.replace_config_string(1_131 + 3, b"n\\^1Dark^7Lord\\t\\0".to_vec())
            .expect("a player");
        chat.update_roster(&game);
        let now = Instant::now();
        // Attributed by the server, and a JA+ line naming nobody but starting with the
        // name the game shows in slot 3 (its colours aside).
        chat.receive(
            ServerEventKind::Chat,
            "^1Dark^7Lord^7: one".to_owned(),
            Some(3),
            now,
        );
        chat.receive(
            ServerEventKind::Chat,
            "DarkLord^7: two".to_owned(),
            None,
            now,
        );
        chat.receive(ServerEventKind::Chat, "Fox: three".to_owned(), None, now);
        assert!(chat.lines.iter().all(|line| !line.muted));
        chat.set_muted_slots(1 << 3);
        let muted: Vec<bool> = chat.lines.iter().map(|line| line.muted).collect();
        assert_eq!(muted, [true, true, false]);
        chat.receive(
            ServerEventKind::Chat,
            "^1Dark^7Lord^7: four".to_owned(),
            Some(3),
            now,
        );
        assert!(chat.lines[3].muted, "new lines too");
        chat.set_muted_slots(0);
        assert!(chat.lines.iter().all(|line| !line.muted));
    }

    /// A blocked player's chat stays out of the console as it stays out of the chat
    /// box, attributed by the server or not (hub report #191); others' chat does not.
    #[test]
    fn a_muted_slots_chat_stays_out_of_the_console() {
        let mut chat = ChatOverlay::new();
        let mut game = sjk_protocol::GameState::empty_local(0);
        game.replace_config_string(1_131 + 3, b"n\\^1Dark^7Lord\\t\\0".to_vec())
            .expect("a player");
        chat.update_roster(&game);
        assert!(
            !chat.sent_by_muted("^1Dark^7Lord^7: one", Some(3)),
            "nobody blocked"
        );
        chat.set_muted_slots(1 << 3);
        assert!(chat.sent_by_muted("^1Dark^7Lord^7: one", Some(3)));
        assert!(
            chat.sent_by_muted("DarkLord^7: two", None),
            "a JA+ line by its name"
        );
        assert!(!chat.sent_by_muted("Fox: three", None));
    }

    /// The composer open over game lines, with "Fox" in slot 3 and us in slot 0.
    fn game_chat(players: &[(usize, &str)]) -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        let mut game = sjk_protocol::GameState::empty_local(0);
        for (slot, name) in players {
            game.replace_config_string(1_131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                .expect("a player");
        }
        chat.update_roster(&game);
        chat
    }

    fn click(chat: &mut ChatOverlay, at: Vec2) {
        let button = PointerButton::Primary;
        chat.pointer(InputEvent::PointerMove(at));
        chat.pointer(InputEvent::PointerPress {
            position: at,
            button,
        });
        chat.pointer(InputEvent::PointerRelease {
            position: at,
            button,
        });
    }

    /// The feed token of the one name the feed shows.
    fn only_name(chat: &ChatOverlay) -> (Who, Rect) {
        let names: Vec<_> = (0..MAX_VISIBLE)
            .filter_map(|token| chat.visible_people[token].zip(Some(token)))
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        let (who, token) = names[0];
        (
            who,
            chat.ui.rect_for(token as u16).expect("the name's target"),
        )
    }

    #[test]
    fn a_click_pins_the_card_and_whisper_turns_the_composer_to_them() {
        let mut chat = game_chat(&[(0, "Me"), (3, "^1Fox")]);
        chat.receive(
            ServerEventKind::Chat,
            "^1Fox^7: hi".to_owned(),
            Some(3),
            Instant::now(),
        );
        chat.open(false);
        build(&mut chat);
        let (who, name) = only_name(&chat);
        let Who::Player(target) = who else {
            panic!("{who:?}")
        };
        assert_eq!(target.slot(), 3);
        click(&mut chat, centre(name));
        build(&mut chat);
        assert!(chat.card.as_ref().is_some_and(|card| card.pinned));
        // Pinned, it stays with the pointer away, past the grace.
        chat.pointer(InputEvent::PointerMove(Vec2::new(1_900.0, 10.0)));
        chat.build(
            true,
            &crate::text::test_font(),
            VIEWPORT,
            1_000 + 2 * GRACE_MS,
        );
        assert!(chat.card.is_some(), "pinned");
        assert!(texts(&chat).iter().any(|text| text == "Whisper"));
        let whisper = chat.ui.rect_for(CARD_WHISPER).expect("Whisper");
        let block = chat.ui.rect_for(CARD_MUTE).expect("Block");
        assert!(whisper.bottom() <= block.y, "Whisper heads the buttons");
        click(&mut chat, centre(whisper));
        assert!(chat.card.is_none());
        let input = chat.input.as_ref().expect("the composer stays");
        assert_eq!(input.channel, Channel::Whisper);
        assert_eq!(input.recipient, Some(target));
        // The draft goes as the server's whisper (no encrypted slots here).
        chat.input.as_mut().unwrap().insert("hello");
        let sent = chat.edit_key(
            winit::keyboard::KeyCode::Enter,
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter),
            None,
        );
        assert!(matches!(sent, ChatInputResult::Submit(command) if command == "tell 3 \"hello\""));
    }

    #[test]
    fn escape_and_a_click_elsewhere_let_a_pinned_card_go() {
        let mut chat = game_chat(&[(0, "Me"), (3, "Fox")]);
        chat.receive(
            ServerEventKind::Chat,
            "Fox: hi".to_owned(),
            Some(3),
            Instant::now(),
        );
        chat.open(false);
        build(&mut chat);
        let (_, name) = only_name(&chat);
        click(&mut chat, centre(name));
        build(&mut chat);
        assert!(chat.card.is_some());
        // Escape: the card first, the composer stays.
        chat.edit_key(
            winit::keyboard::KeyCode::Escape,
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            None,
        );
        assert!(chat.card.is_none() && chat.is_typing());
        // A second click on the name unpins it; a click on empty feed does too.
        click(&mut chat, centre(name));
        build(&mut chat);
        assert!(chat.card.as_ref().is_some_and(|card| card.pinned));
        click(&mut chat, centre(name));
        assert!(chat.card.is_none(), "clicked again");
        click(&mut chat, centre(name));
        build(&mut chat);
        click(&mut chat, Vec2::new(1_900.0, 10.0));
        assert!(chat.card.is_none(), "clicked elsewhere");
    }

    /// Safety code is a button of its own: a click on it is its, not the card's
    /// (before 23/10/2026 the press was taken for the card's and the click was lost).
    #[test]
    fn a_click_on_safety_code_asks_for_it() {
        let mut chat = game_chat(&[(0, "Me"), (3, "Fox")]);
        let fox = chat.roster.target(Some(3)).expect("Fox");
        chat.open(false);
        chat.card = Some(Card {
            who: Who::Player(fox),
            person: Person {
                name: "Fox".to_owned(),
                safety: true,
                ..Person::default()
            },
            muted: false,
            anchor: Rect::new(10.0, 300.0, 40.0, 10.0),
            left_ms: None,
            pinned: true,
        });
        build(&mut chat);
        let safety = chat.ui.rect_for(CARD_SAFETY).expect("Safety code");
        click(&mut chat, centre(safety));
        assert_eq!(chat.take_safety_request(), Some(3));
    }

    /// Trade is a button of its own: a click on it asks to trade (before 25/10/2026 the
    /// press was taken for the card's and the click was lost).
    #[test]
    fn a_click_on_trade_asks_to_trade() {
        let mut chat = game_chat(&[(0, "Me"), (3, "Fox")]);
        let fox = chat.roster.target(Some(3)).expect("Fox");
        chat.open(false);
        chat.card = Some(Card {
            who: Who::Player(fox),
            person: Person {
                name: "Fox".to_owned(),
                trade: true,
                ..Person::default()
            },
            muted: false,
            anchor: Rect::new(10.0, 300.0, 40.0, 10.0),
            left_ms: None,
            pinned: true,
        });
        build(&mut chat);
        let trade = chat.ui.rect_for(CARD_TRADE).expect("Trade");
        click(&mut chat, centre(trade));
        assert_eq!(chat.take_trade_request(), Some(3));
        assert!(chat.card.is_none(), "the card makes way for the request");
    }

    /// Block for this map and, for staff, for everyone are buttons of their own: a click
    /// asks to block (before 29/10/2026 the press was taken for the card's, as Trade's
    /// once was, and the click was lost).
    #[test]
    fn a_click_on_the_block_buttons_asks_to_block() {
        for (token, everyone) in [(CARD_MAP_BLOCK, false), (CARD_STAFF_BLOCK, true)] {
            let mut chat = game_chat(&[(0, "Me"), (3, "Fox")]);
            let fox = chat.roster.target(Some(3)).expect("Fox");
            chat.open(false);
            chat.card = Some(Card {
                who: Who::Player(fox),
                person: Person {
                    name: "Fox".to_owned(),
                    place: Place::Slot(3),
                    map_block: Some(sender_card::MapBlockRow {
                        own: false,
                        staff: Some(false),
                    }),
                    ..Person::default()
                },
                muted: false,
                anchor: Rect::new(10.0, 300.0, 40.0, 10.0),
                left_ms: None,
                pinned: true,
            });
            build(&mut chat);
            let button = chat.ui.rect_for(token).expect("block button");
            click(&mut chat, centre(button));
            let request = chat.take_map_block_request().expect("a block request");
            assert_eq!(
                (request.slot, request.everyone, request.blocked),
                (3, everyone, true)
            );
        }
    }

    #[test]
    fn whisper_needs_the_same_player_still_here_and_not_ourselves() {
        let mut chat = game_chat(&[(0, "Me"), (3, "Fox")]);
        let fox = chat.roster.target(Some(3)).expect("Fox");
        let me = chat.roster.target(Some(0)).expect("us");
        assert_eq!(
            chat.whisper_target(Who::Player(fox), Place::Slot(3)),
            Some(fox)
        );
        assert_eq!(chat.whisper_target(Who::Player(me), Place::Slot(0)), None);
        // An SJK chat sender, by the slot their key is matched to here.
        assert_eq!(
            chat.whisper_target(Who::Hub(7), Place::SlotByName(3)),
            Some(fox)
        );
        assert_eq!(chat.whisper_target(Who::Hub(7), Place::Away), None);
        assert_eq!(chat.whisper_target(Who::Hub(7), Place::Unknown), None);
        assert_eq!(
            chat.whisper_target(Who::Hub(7), Place::Slot(9)),
            None,
            "nobody there"
        );
        // Fox renames (or leaves and another takes the slot): the old line's Fox is gone.
        let mut game = sjk_protocol::GameState::empty_local(0);
        game.replace_config_string(1_131, b"n\\Me\\t\\0".to_vec())
            .expect("us");
        game.replace_config_string(1_131 + 3, b"n\\Wolf\\t\\0".to_vec())
            .expect("another");
        chat.update_roster(&game);
        assert_eq!(chat.whisper_target(Who::Player(fox), Place::Slot(3)), None);
        // A card pinned before offers no Whisper, and a press says why.
        chat.open(false);
        chat.card = Some(Card {
            who: Who::Player(fox),
            person: Person::default(),
            muted: false,
            anchor: Rect::new(10.0, 10.0, 40.0, 10.0),
            left_ms: None,
            pinned: true,
        });
        build(&mut chat);
        assert!(chat.card.is_some() && chat.ui.rect_for(CARD_WHISPER).is_none());
        chat.press_card_whisper();
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Global);
        assert!(!chat.notice.is_empty());
    }

    /// Two players wearing one name: a line the server did not attribute names nobody,
    /// so it has no card and no actions; one it attributed keeps its slot.
    #[test]
    fn a_shared_name_is_only_clickable_on_lines_the_server_attributed() {
        let mut chat = game_chat(&[(0, "Me"), (3, "^1Fox"), (4, "^2Fox")]);
        let now = Instant::now();
        chat.receive(ServerEventKind::Chat, "Fox^7: one".to_owned(), None, now);
        chat.open(false);
        build(&mut chat);
        assert!(chat.visible_people.iter().all(Option::is_none));
        chat.receive(
            ServerEventKind::Chat,
            "^2Fox^7: two".to_owned(),
            Some(4),
            now,
        );
        build(&mut chat);
        let (who, _) = only_name(&chat);
        assert!(matches!(who, Who::Player(target) if target.slot() == 4));
    }
}

#[cfg(test)]
mod shots {
    use super::*;

    /// A click on a chat name over a duel6 view: the pinned sender card with Whisper,
    /// Block and the rest, rendered off screen at 1080p and 4K.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn chat_name_actions_shot() {
        for (size, suffix) in [([1920, 1080], "1080p"), ([3840, 2160], "4k")] {
            crate::world_shot::on_big_stack(move || {
                let Some((mut gpu, _profile)) = crate::world_shot::open(
                    "maps/mp/duel6.bsp",
                    size,
                    None,
                    &[("cg_chatbox", "60000"), ("cg_chatboxlines", "8")],
                ) else {
                    return;
                };
                gpu.chat.for_shot = true;
                let mut game = sjk_protocol::GameState::empty_local(0);
                for (slot, name) in [(0, "^1Sol^7Vulpes"), (3, "^1Fox"), (5, "dart_mario")] {
                    game.replace_config_string(
                        1131 + slot,
                        format!("n\\{name}\\t\\0").into_bytes(),
                    )
                    .unwrap();
                }
                gpu.chat.update_roster(&game);
                let now = Instant::now();
                for (line, slot) in [
                    ("dart_mario^7: ^2gg", 5),
                    ("^1Fox^7: ^2anyone up for a duel on ffa3?", 3),
                    ("dart_mario^7: ^2in 5 min", 5),
                ] {
                    gpu.chat
                        .receive(ServerEventKind::Chat, line.to_owned(), Some(slot), now);
                }
                gpu.chat.open(false);
                let _ = crate::world_shot::frame(&mut gpu, 4);
                let token = (0..MAX_VISIBLE)
                    .find(|token| {
                        matches!(gpu.chat.visible_people[*token],
                            Some(Who::Player(target)) if target.slot() == 3)
                    })
                    .expect("Fox's name in the feed");
                let who = gpu.chat.visible_people[token];
                gpu.chat.click_name(token, who);
                assert!(gpu.chat.card.as_ref().is_some_and(|card| card.pinned));
                let path =
                    crate::world_shot::shoot(&mut gpu, 6, &format!("chat-name-actions-{suffix}"));
                println!("{}", path.display());
            });
        }
    }
}
