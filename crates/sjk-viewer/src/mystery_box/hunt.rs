//! The box hunt (Sol's idea, 27/10/2026; `docs/mystery-boxes.md`, "Box hunt"): players
//! with the Box hunt toy on (`crate::box_hunt`) take part. Every 10 minutes at most the hub
//! picks one of them on the server as the mark; another who kills the mark wins a free
//! mystery box roll. Only players with it on see the mark or can be it. The hub never
//! sees kills: this client tells it who killed its player when its player is the mark,
//! so no one can claim a kill they did not make. Each read says whether the player is
//! away (typing in the chat, in a menu, spectating, idle), so the hub never picks them
//! and hands their mark on after a minute (Sol, 31/10/2026). Each hunter has one life
//! (Sol, 01/11/2026): killed by the mark they are out of the chase, and a player caught
//! teleporting themselves or in noclip ([`super::hunt_watch`]) is out at once; this client tells
//! the hub both. Once every hunter is out, the mark wins credits. A mark caught cheating
//! or typing in the chat [`CHAT_LIMIT`] blows up (Sol, 02/11/2026): the hub gives a
//! hunter at random the free box roll a kill would have, everyone taking part sees the
//! explosion, and the mark's own client kills them. The mark sees the hunters still after them ringed red,
//! as they see the mark.

use super::label::{Label, Tone};
use crate::GpuState;
use glam::Vec3;
use sjk_identity::mystery_box::{self as wire, HuntEvent, HuntOut, HuntState};
use std::time::{Duration, Instant};

/// How often the hunt is read while the player takes part (a new mark is also told at
/// once by the hub's toy event).
const READ_EVERY: Duration = Duration::from_secs(20);
/// Soonest a read follows another asked by an event.
const READ_GAP: Duration = Duration::from_secs(2);
/// How long before the hub's end of a mark this client stops showing it: a kill told
/// after the end is not counted, so none is shown that could arrive late.
const END_EARLY: Duration = Duration::from_secs(1);
/// How high over a player's origin the mark's label stands, and the point it is seen by.
pub(super) const MARK_RISE: f32 = 70.0;
pub(super) const MARK_SIGHT: f32 = 20.0;
/// `MAX_CLIENTS`: slots of players.
const SLOTS: u16 = 32;
/// How long the mark may type in the chat before blowing up, and when they are warned
/// (the hub hands an away mark on after a minute; this comes first).
const CHAT_LIMIT: Duration = Duration::from_secs(30);
const CHAT_WARN: Duration = Duration::from_secs(20);
/// The sound source of a mark's explosion, past the alarm's.
const BLAST_SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(super::SOURCE_BASE | 0x1_0001);

/// The hunt as this client follows it.
#[derive(Default)]
pub(crate) struct Hunt {
    /// The hunt last read, and the server it was read on.
    state: Option<(String, HuntState)>,
    /// The last read taken from the identity snapshot.
    seen: u64,
    /// When the hunt was last asked for.
    asked: Option<Instant>,
    /// Asked for at once (an event changed it).
    stale: bool,
    /// The obituaries counted so far.
    obituaries: u64,
    /// The mark last told to the player, so each is told once.
    told: Option<u8>,
    /// When the mark last read ends, by this client's clock (a little before the hub's,
    /// [`END_EARLY`]): it is no longer shown nor told again after.
    ends: Option<Instant>,
    /// Whether the last read said the player was away; a change is read at once.
    away: Option<bool>,
    /// The mark the card at the top of the screen names, while they are the mark.
    card: Option<Card>,
    /// Where a box's price went when it blew up, told when its roll stops on the teddy
    /// bear rather than at the opening (Sol, 30/10/2026): when, centred or not, the text.
    laughs: Vec<(Instant, bool, String)>,
    /// The player is out of the told mark's chase (killed by the mark, or caught
    /// cheating): the mark is no longer shown to them, and nothing more is told.
    out: Option<HuntOut>,
    /// The player's own state and commands, watched for teleports and noclip.
    watch: super::hunt_watch::Watch,
    /// The map watched (its BSP's address): a new one starts the watch again.
    watched_map: usize,
    /// Since when the player, the mark, has been typing in the chat, and whether they
    /// were warned.
    typing: Option<(Instant, bool)>,
}

/// The card at the top of the screen naming the mark (Sol's idea, 28/10/2026), from the
/// moment the player is told of them until they are hunted or the hub picks another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Card {
    pub(super) slot: u8,
    /// Their worn name, colour codes and all.
    pub(super) name: String,
    /// The player is the mark.
    pub(super) own: bool,
    /// When they were told: the card pulses as it arrives.
    pub(super) since: Instant,
    /// The hunters still in the chase and all of them, as the hub last said.
    pub(super) hunters: Option<(u32, u32)>,
    /// The player is out of the chase.
    pub(super) out: bool,
}

impl Hunt {
    fn mark(&self) -> Option<&wire::HuntMark> {
        self.state
            .as_ref()
            .filter(|(_, state)| state.hunting)
            .and_then(|(_, state)| state.mark.as_ref())
    }

    /// The card to draw: the told mark's, while the player takes part.
    pub(super) fn card(&self) -> Option<&Card> {
        self.card.as_ref()
    }

    /// A new mark told at `now`: the card names them.
    fn tell(&mut self, slot: u8, own: u8, name: &str, now: Instant) -> (bool, String) {
        self.told = Some(slot);
        self.out = None;
        self.card = Some(Card {
            slot,
            name: name.to_owned(),
            own: slot == own,
            since: now,
            hunters: None,
            out: false,
        });
        mark_words(slot, own, name)
    }

    /// The mark was hunted, or its time is up: no mark until the hub picks the next one,
    /// so the last read does not tell the old one again.
    fn hunted(&mut self) {
        self.told = None;
        self.card = None;
        self.ends = None;
        self.out = None;
        self.typing = None;
        if let Some((_, state)) = &mut self.state {
            state.mark = None;
        }
    }

    /// Whether the player is the mark: the last read says so, or the hub's event did
    /// before a read could (the kill is told either way; the hub checks it).
    fn marked(&self, own: u8) -> bool {
        self.mark().is_some_and(|mark| mark.slot == own)
            || self.card.as_ref().is_some_and(|card| card.own)
    }

    /// The hunters still after the player, the mark on slot `own`, as the hub last said:
    /// a bit a slot.
    fn chasers(&self, own: u8) -> u32 {
        let Some((_, state)) = self.state.as_ref().filter(|_| self.marked(own)) else {
            return 0;
        };
        state
            .chasers
            .iter()
            .filter(|slot| u16::from(**slot) < SLOTS && **slot != own)
            .fold(0, |bits, slot| bits | 1 << slot)
    }

    /// The hunter on `slot` is out: no longer ringed for the mark.
    fn chaser_out(&mut self, slot: u8) {
        if let Some((_, state)) = &mut self.state {
            state.chasers.retain(|chaser| *chaser != slot);
        }
    }

    /// Whether the player, the mark on slot `own` still in the chase, has stayed in the
    /// chat too long at `now` (`typing` or not): the words of a warning first.
    fn watch_chat(&mut self, typing: bool, now: Instant) -> (bool, Option<String>) {
        if !typing {
            self.typing = None;
            return (false, None);
        }
        let (since, warned) = *self.typing.get_or_insert((now, false));
        let long = now.saturating_duration_since(since);
        if long >= CHAT_LIMIT {
            return (true, None);
        }
        if !warned && long >= CHAT_WARN {
            self.typing = Some((since, true));
            return (
                false,
                Some(format!(
                    "Leave the chat!\nThe mark blows up after {} seconds in it",
                    CHAT_LIMIT.as_secs()
                )),
            );
        }
        (false, None)
    }

    /// Whether the hunt should be read at `now`.
    fn due(&self, now: Instant) -> bool {
        self.asked.is_none_or(|asked| {
            let since = now.saturating_duration_since(asked);
            since >= READ_EVERY || (self.stale && since >= READ_GAP)
        })
    }
}

/// When `mark`, read at `now` (`unix` seconds by this client's clock), ends by this
/// client's clock, [`END_EARLY`] before the hub's: from the seconds the hub says are
/// left, or from its end time for a hub that does not say them.
fn mark_ends(mark: &wire::HuntMark, now: Instant, unix: i64) -> Instant {
    let left = mark
        .left
        .unwrap_or_else(|| u64::try_from(mark.until - unix).unwrap_or(0));
    (now + Duration::from_secs(left))
        .checked_sub(END_EARLY)
        .unwrap_or(now)
        .max(now)
}

/// What the player is told when slot `slot` (named `name`) becomes the mark; `own` is
/// the player's slot.
fn mark_words(slot: u8, own: u8, name: &str) -> (bool, String) {
    if slot == own {
        (
            true,
            "You are the mark!\nOutlast every hunter for 100 credits each\n\
             Teleport or sit in the chat and you blow up"
                .to_owned(),
        )
    } else {
        (
            false,
            format!(
                "Box hunt: {name}^7 is the mark. Kill them for a free mystery box roll; \
                 you have one life"
            ),
        )
    }
}

/// What the player on `own` is told when the hunter on `slot` (named `name`) is out of
/// the chase, killed by the mark or caught `cheat`ing.
fn out_words(slot: u8, own: u8, cheat: bool, name: &str) -> (bool, String) {
    match (slot == own, cheat) {
        (true, false) => (
            true,
            "The mark got you!\nYou're out of this box hunt".to_owned(),
        ),
        (true, true) => (
            true,
            "Out of this box hunt\nTeleporting or noclip is cheating".to_owned(),
        ),
        (false, false) => (
            false,
            format!("Box hunt: the mark got {name}^7, who is out"),
        ),
        (false, true) => (false, format!("Box hunt: {name}^7 is out for cheating")),
    }
}

/// What the player on `own` is told when the mark on `slot` (named `name`) outlasted
/// every hunter, `hunters` of them, and won `credits`.
fn survived_words(slot: u8, own: u8, credits: u64, hunters: u32, name: &str) -> (bool, String) {
    if slot != own {
        return (
            false,
            format!("Box hunt: {name}^7 outlasted every hunter and won {credits} credits"),
        );
    }
    let hunters = if hunters == 1 {
        "1 hunter".to_owned()
    } else {
        format!("{hunters} hunters")
    };
    if credits > 0 {
        (
            true,
            format!("You outlasted every hunter!\n{credits} credits for {hunters}"),
        )
    } else {
        (
            true,
            "You outlasted every hunter!\nNo credits: you won enough today".to_owned(),
        )
    }
}

/// What the player on `own` is told when the mark on `slot` (named `name`) blew up,
/// caught cheating or in the `chat` too long, and the hunter on `to` (named `to_name`)
/// was picked for a free box roll, which they `won` or not.
fn blown_words(
    (slot, own): (u8, u8),
    chat: bool,
    (to, won): (Option<u8>, bool),
    name: &str,
    to_name: &str,
) -> (bool, String) {
    let won = won && to.is_some();
    if slot == own {
        let why = if chat {
            "You stayed in the chat too long"
        } else {
            "Teleporting or noclip is cheating"
        };
        let after = if won {
            format!("{to_name}^7 gets a free box roll")
        } else {
            "The hunt is off".to_owned()
        };
        return (true, format!("BOOM! {why}\n{after}"));
    }
    if to == Some(own) {
        return (
            true,
            if won {
                "The mark blew up!\nFree mystery box roll".to_owned()
            } else {
                "The mark blew up!\nNo free roll: you hold the most unspent, or won enough today"
                    .to_owned()
            },
        );
    }
    let why = if chat {
        "for staying in the chat"
    } else {
        "for cheating"
    };
    if won {
        (
            false,
            format!(
                "Box hunt: the mark, {name}^7, blew up {why}, and {to_name}^7 gets a free box roll"
            ),
        )
    } else {
        (
            false,
            format!("Box hunt: the mark, {name}^7, blew up {why}. The hunt is off"),
        )
    }
}

/// What a hunter is told when the box the player on slot `opener` (named `opener_name`)
/// opened blew up and `credits` went to the hunter on `to` (named `to_name`); `own` is
/// the player's slot. Nothing for the opener: their own box says it.
fn laugh_words(
    opener: u8,
    to: Option<u8>,
    credits: u64,
    own: u8,
    opener_name: &str,
    to_name: &str,
) -> Option<(bool, String)> {
    if opener == own {
        return None;
    }
    Some(match to {
        Some(to) if to == own && credits > 0 => (
            true,
            format!("{opener_name}^7's box blew up!\nYou get their {credits} credits"),
        ),
        Some(_) if credits > 0 => (
            false,
            format!(
                "Box hunt: {opener_name}^7's box blew up, and {to_name}^7 gets their {credits} credits"
            ),
        ),
        _ => (false, format!("Box hunt: {opener_name}^7's box blew up")),
    })
}

impl GpuState {
    /// The frame's box hunt: read the hunt, take the hub's events of it (`events`, this
    /// server's toy events), tell the hub who killed the player when they are the mark.
    pub(super) fn update_box_hunt(
        &mut self,
        events: &[sjk_identity::ToyEvent],
        server: &str,
        own: u8,
        now: Instant,
    ) {
        let on = self.box_hunt.on();
        // Away: not playing (a menu, the console, spectating, idle) or typing in the chat.
        let away = !self.actively_playing(now) || self.chat.is_typing();
        let hunt = &mut self.mystery_boxes.hunt;
        if on && hunt.away != Some(away) {
            hunt.away = Some(away);
            hunt.stale = true;
        }
        // A read for this server.
        let read = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .box_hunt
                .as_ref()
                .filter(|(serial, read_on, _)| *serial != hunt.seen && read_on == server)
                .cloned()
        })
        .flatten();
        if let Some((serial, read_on, mut state)) = read {
            hunt.seen = serial;
            self.mystery_boxes.free = state.free;
            if let Some(mark) = state.mark.as_ref().filter(|_| state.hunting) {
                let unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |since| since.as_secs() as i64);
                let ends = mark_ends(mark, now, unix);
                // A mark about to end is not shown (nor told again) at all.
                if ends > now {
                    hunt.ends = Some(ends);
                } else {
                    state.mark = None;
                }
            }
            // The hub says the player is out of this chase (a read after they reported it,
            // or joining back).
            if let Some(out) = state.out.as_deref() {
                hunt.out.get_or_insert(match out {
                    "cheat" => HuntOut::Cheat,
                    _ => HuntOut::Dead { by: 0 },
                });
            }
            hunt.state = Some((read_on, state));
            // The hub has no mark now, or another one (told below): not this card.
            let read_slot = hunt.mark().map(|mark| mark.slot);
            if hunt
                .card
                .as_ref()
                .is_some_and(|card| Some(card.slot) != read_slot)
            {
                hunt.card = None;
            }
        }
        if hunt
            .state
            .as_ref()
            .is_some_and(|(read_on, _)| read_on != server)
        {
            hunt.state = None;
            hunt.told = None;
            hunt.card = None;
            hunt.ends = None;
        }
        // The mark's time is up: shown no more (a kill would not count), and the hunt is
        // read again for what comes next. The same player picked again later is told.
        if hunt.ends.is_some_and(|ends| now >= ends) {
            hunt.hunted();
            hunt.stale = true;
        }
        let told_before = hunt.told;
        let mut lines = Vec::new();
        // Marks that blew up, for their explosions.
        let mut blown = Vec::new();
        for event in events.iter().filter(|event| event.toy == wire::HUNT_TOY) {
            let Some(said) = wire::parse_hunt_event(&event.event) else {
                continue;
            };
            hunt.stale = true;
            if !on {
                continue;
            }
            match said {
                HuntEvent::Mark(slot) if hunt.told != Some(slot) => {
                    lines.push(hunt.tell(slot, own, &event.claimed_name, now));
                }
                HuntEvent::Hunted { slot, won } => {
                    hunt.hunted();
                    lines.push(if slot == own && won {
                        (
                            true,
                            "You hunted the mark!\nFree mystery box roll".to_owned(),
                        )
                    } else if slot == own {
                        // In the centre too, so a hunter who won nothing knows why.
                        (
                            true,
                            "You hunted the mark!\nNo free roll: you hold the most unspent, \
                             or won enough today"
                                .to_owned(),
                        )
                    } else {
                        (
                            false,
                            format!("Box hunt: {}^7 hunted the mark", event.claimed_name),
                        )
                    });
                }
                HuntEvent::Out { slot, cheat } => {
                    hunt.chaser_out(slot);
                    if slot == own {
                        hunt.out.get_or_insert(if cheat {
                            HuntOut::Cheat
                        } else {
                            HuntOut::Dead { by: 0 }
                        });
                    }
                    lines.push(out_words(slot, own, cheat, &event.claimed_name));
                }
                HuntEvent::Blown {
                    slot,
                    chat,
                    to,
                    won,
                } => {
                    hunt.hunted();
                    blown.push(slot);
                    let to_name = to
                        .and_then(|to| self.chat.player_name(u16::from(to)))
                        .unwrap_or("a hunter")
                        .to_owned();
                    lines.push(blown_words(
                        (slot, own),
                        chat,
                        (to, won),
                        &event.claimed_name,
                        &to_name,
                    ));
                }
                HuntEvent::Survived {
                    slot,
                    credits,
                    hunters,
                } => {
                    hunt.hunted();
                    lines.push(survived_words(
                        slot,
                        own,
                        credits,
                        hunters,
                        &event.claimed_name,
                    ));
                }
                HuntEvent::Laugh {
                    opener,
                    to,
                    credits,
                } => {
                    let opener_name = self
                        .chat
                        .player_name(u16::from(opener))
                        .unwrap_or("Someone")
                        .to_owned();
                    if let Some((centre, text)) =
                        laugh_words(opener, to, credits, own, &opener_name, &event.claimed_name)
                    {
                        hunt.laughs.push((now + super::ROLL, centre, text));
                    }
                }
                HuntEvent::Mark(_) => {}
            }
        }
        // The mark read without its event (joining mid-hunt): told once too.
        let read_mark = hunt.mark().map(|mark| (mark.slot, mark.name.clone()));
        if on
            && let Some((slot, name)) = read_mark
            && hunt.told != Some(slot)
        {
            lines.push(hunt.tell(slot, own, &name, now));
        }
        // Where the price of a box that blew up went, once its roll has stopped (the box
        // laughs there, `super::MysteryBoxes::tick`).
        let due = hunt.laughs.iter().filter(|(at, ..)| *at <= now).count();
        lines.extend(
            hunt.laughs
                .drain(..due)
                .map(|(_, centre, text)| (centre, text)),
        );
        // A new mark rings the hunt's alarm, for the player alone.
        if hunt.told.is_some() && hunt.told != told_before {
            self.mystery_boxes.cues.push(super::Cue {
                path: super::HUNT_MARK,
                origin: None,
                source: super::HUNT_SOURCE,
            });
        }
        let hunt = &mut self.mystery_boxes.hunt;
        // The player, the mark, killed by another player: the hub is told by whom.
        let decoded = self.obituaries.decoded();
        let new = decoded.saturating_sub(hunt.obituaries).min(8) as usize;
        hunt.obituaries = decoded;
        let marked = on && hunt.marked(own);
        let killer = (0..new)
            .filter_map(|offset| self.obituaries.feed().newest(offset))
            .find(|kill| {
                kill.target == u16::from(own)
                    && kill.attacker < SLOTS
                    && kill.attacker != kill.target
            })
            .map(|kill| kill.attacker as u8);
        // In a chase (the mark, or a hunter still in it): killed by the mark, or caught
        // teleporting or in noclip, the player is out.
        let chased = hunt.mark().map(|mark| mark.slot).or(hunt.told);
        let in_chase = on && chased.is_some() && hunt.out.is_none();
        let map = std::sync::Arc::as_ptr(&self.bsp) as usize;
        if hunt.watched_map != map {
            hunt.watched_map = map;
            hunt.watch.reset();
        }
        let fake_noclip = self.local_prediction.fake_noclip();
        let cheated = self.live_session.as_ref().is_some_and(|session| {
            hunt.watch
                .read_commands(session.sent_commands().iter(), own);
            let snapshot = session.latest_snapshot();
            let sample = super::hunt_watch::Sample::of(
                &snapshot.player,
                snapshot.server_time,
                own,
                fake_noclip,
            );
            hunt.watch.step(sample, now)
        });
        // The mark typing in the chat too long blows up.
        let (chatted, warning) = hunt.watch_chat(marked && in_chase && self.chat.is_typing(), now);
        lines.extend(warning.map(|text| (true, text)));
        let out = if !in_chase {
            None
        } else if cheated {
            Some(HuntOut::Cheat)
        } else if chatted {
            Some(HuntOut::Chat)
        } else {
            killer
                .filter(|by| !marked && Some(*by) == chased)
                .map(|by| HuntOut::Dead { by })
        };
        if let Some(out) = out {
            hunt.out = Some(out);
        }
        // The card follows the chase: hunters left, and whether the player is out.
        let left = hunt
            .mark()
            .and_then(|mark| Some((mark.alive?, mark.hunters?)));
        let player_out = hunt.out.is_some();
        if let Some(card) = &mut hunt.card {
            card.hunters = left.or(card.hunters);
            card.out = player_out;
        }
        // Read when due: while taking part, while a box stands here (free rolls), or
        // when the hunt is not known yet.
        let wanted = on || !self.mystery_boxes.here.is_empty();
        let read_now = wanted && hunt.due(now);
        if read_now {
            hunt.asked = Some(now);
            hunt.stale = false;
        }
        if let Some(out) = out {
            hunt.asked = Some(now);
            crate::player_identity::with_service(|service| {
                service.box_hunt_out(server.to_owned(), out, away);
            });
        } else if let (true, Some(by)) = (marked, killer) {
            hunt.asked = Some(now);
            crate::player_identity::with_service(|service| {
                service.box_hunt_killed(server.to_owned(), by, away);
            });
        } else if read_now {
            crate::player_identity::with_service(|service| {
                service.read_box_hunt(server.to_owned(), away);
            });
        }
        if !on {
            self.mystery_boxes.hunt.told = None;
            self.mystery_boxes.hunt.card = None;
            self.mystery_boxes.hunt.away = None;
        }
        for slot in blown {
            self.box_hunt_blast(slot, own);
        }
        for (centre, text) in lines {
            if centre {
                self.mystery_box_centre(text);
            } else {
                self.mystery_box_print(text);
            }
        }
    }

    /// The players ringed in red ([`crate::hud::team_outline`]), a bit a client slot,
    /// while the player takes part and is in the chase: the mark for a hunter, and for
    /// the mark the hunters still after them (Sol, 02/11/2026).
    pub(crate) fn box_hunt_outlined(&self) -> u32 {
        let hunt = &self.mystery_boxes.hunt;
        if !self.box_hunt.on() || hunt.out.is_some() {
            return 0;
        }
        let Some(own) = self
            .live_session
            .as_ref()
            .and_then(|session| u8::try_from(session.game_state().client_num).ok())
        else {
            return 0;
        };
        if hunt.marked(own) {
            return hunt.chasers(own);
        }
        hunt.mark()
            .filter(|mark| u16::from(mark.slot) < SLOTS)
            .map_or(0, |mark| 1 << mark.slot)
    }

    /// The mark on `slot` blew up (the player's slot `own`): the box's explosion where
    /// they stand, when this client sees them, and when they are this client's player,
    /// the game's `kill` (as a box that blows up on its opener; not where the Ragdoll could
    /// not strike them either).
    fn box_hunt_blast(&mut self, slot: u8, own: u8) {
        let mine = slot == own;
        let stands = if mine {
            self.local_prediction
                .predicted_state()
                .map(|state| Vec3::from_array(state.origin))
        } else {
            self.box_hunt_player_origin(slot)
        };
        if let Some(stands) = stands {
            self.mystery_boxes.blasts.push(stands);
            self.mystery_boxes.cues.push(super::Cue {
                path: super::BLAST_SOUND,
                origin: Some(stands.to_array()),
                source: BLAST_SOURCE,
            });
        }
        if mine
            && self.ragdoll.strikable()
            && let Some(session) = self.live_session.as_mut()
            && let Err(error) = session.send_reliable_command(b"kill")
        {
            crate::log::progress(format_args!("box hunt: kill not sent: {error}"));
        }
    }

    /// Where the player on `slot` stands, when the game server sends them.
    pub(super) fn box_hunt_player_origin(&mut self, slot: u8) -> Option<Vec3> {
        let session = self.live_session.as_ref()?;
        let sent = session
            .latest_snapshot()
            .entities
            .iter()
            .any(|entity| entity.number() == u16::from(slot));
        if !sent {
            return None;
        }
        let time = self.presentation_clock.sample(Instant::now());
        let presented = self
            .live_world
            .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))?;
        Some(Vec3::from_array(presented.sample(time).translation))
    }

    /// Where the mark stands, while the player takes part, someone else is the mark and
    /// the game server sends them (in this player's snapshot).
    pub(super) fn box_hunt_mark_origin(&mut self) -> Option<Vec3> {
        if !self.box_hunt.on() || self.mystery_boxes.hunt.out.is_some() {
            return None;
        }
        let slot = self.mystery_boxes.hunt.mark()?.slot;
        if i32::from(slot) == self.live_session.as_ref()?.game_state().client_num {
            return None;
        }
        self.box_hunt_player_origin(slot)
    }

    /// The mark's label over their head (`origin`, [`Self::box_hunt_mark_origin`]): the
    /// point it stands at, the point it is seen by, and the label.
    pub(super) fn box_hunt_mark_label(origin: Vec3) -> (Vec3, Vec3, Label) {
        (
            origin + Vec3::Z * MARK_RISE,
            origin + Vec3::Z * MARK_SIGHT,
            Label {
                headline: "MARK".to_owned(),
                line: "Kill for a free box roll".to_owned(),
                tone: Tone::Mark,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_told_to_themselves_and_to_hunters() {
        let (centre, words) = mark_words(3, 3, "Kyle");
        assert!(centre);
        assert!(words.starts_with("You are the mark!"));
        let (centre, words) = mark_words(3, 5, "Kyle");
        assert!(!centre);
        assert!(words.contains("Kyle^7 is the mark"));
    }

    #[test]
    fn a_hunter_out_and_a_mark_who_outlasted_them_are_told() {
        assert_eq!(
            out_words(4, 4, false, "Jan"),
            (
                true,
                "The mark got you!\nYou're out of this box hunt".to_owned()
            )
        );
        assert!(out_words(4, 4, true, "Jan").1.contains("cheating"));
        assert_eq!(
            out_words(4, 2, true, "Jan"),
            (false, "Box hunt: Jan^7 is out for cheating".to_owned())
        );
        assert_eq!(
            survived_words(1, 1, 300, 3, "Kyle"),
            (
                true,
                "You outlasted every hunter!\n300 credits for 3 hunters".to_owned()
            )
        );
        assert!(
            survived_words(1, 1, 0, 3, "Kyle")
                .1
                .contains("won enough today")
        );
        assert_eq!(
            survived_words(1, 2, 200, 2, "Kyle"),
            (
                false,
                "Box hunt: Kyle^7 outlasted every hunter and won 200 credits".to_owned()
            )
        );
    }

    #[test]
    fn a_mark_who_blew_up_is_told_to_everyone() {
        let (centre, words) = blown_words((1, 1), false, (Some(4), true), "Kyle", "Jan");
        assert!(centre);
        assert_eq!(
            words,
            "BOOM! Teleporting or noclip is cheating\nJan^7 gets a free box roll"
        );
        assert_eq!(
            blown_words((1, 1), true, (None, false), "Kyle", "a hunter").1,
            "BOOM! You stayed in the chat too long\nThe hunt is off"
        );
        assert_eq!(
            blown_words((1, 4), true, (Some(4), true), "Kyle", "Jan"),
            (true, "The mark blew up!\nFree mystery box roll".to_owned())
        );
        assert!(
            blown_words((1, 4), true, (Some(4), false), "Kyle", "Jan")
                .1
                .contains("No free roll")
        );
        assert_eq!(
            blown_words((1, 6), true, (Some(4), true), "Kyle", "Jan"),
            (
                false,
                "Box hunt: the mark, Kyle^7, blew up for staying in the chat, and Jan^7 gets a free box roll"
                    .to_owned()
            )
        );
        assert_eq!(
            blown_words((1, 6), false, (None, false), "Kyle", "a hunter").1,
            "Box hunt: the mark, Kyle^7, blew up for cheating. The hunt is off"
        );
    }

    #[test]
    fn the_mark_in_the_chat_is_warned_then_blows_up() {
        let now = Instant::now();
        let mut hunt = Hunt::default();
        assert_eq!(hunt.watch_chat(true, now), (false, None));
        assert_eq!(
            hunt.watch_chat(true, now + Duration::from_secs(10)),
            (false, None)
        );
        let (out, warning) = hunt.watch_chat(true, now + CHAT_WARN);
        assert!(!out && warning.is_some_and(|text| text.starts_with("Leave the chat!")));
        assert_eq!(hunt.watch_chat(true, now + CHAT_WARN), (false, None));
        assert_eq!(hunt.watch_chat(true, now + CHAT_LIMIT), (true, None));
        // Leaving the chat starts the count again.
        assert_eq!(hunt.watch_chat(false, now + CHAT_LIMIT), (false, None));
        assert_eq!(hunt.watch_chat(true, now + CHAT_LIMIT * 2), (false, None));
    }

    #[test]
    fn the_mark_sees_the_hunters_still_after_them() {
        let now = Instant::now();
        let mut hunt = Hunt {
            state: Some((
                "server".to_owned(),
                HuntState {
                    hunting: true,
                    mark: Some(wire::HuntMark {
                        slot: 2,
                        ..wire::HuntMark::default()
                    }),
                    chasers: vec![1, 2, 5, 40],
                    ..HuntState::default()
                },
            )),
            ..Hunt::default()
        };
        assert_eq!(hunt.chasers(2), 1 << 1 | 1 << 5);
        // A hunter does not.
        assert_eq!(hunt.chasers(5), 0);
        hunt.chaser_out(5);
        assert_eq!(hunt.chasers(2), 1 << 1);
        hunt.tell(2, 2, "Sol", now);
        hunt.hunted();
        assert_eq!(hunt.chasers(2), 0);
    }

    #[test]
    fn a_new_mark_or_its_end_puts_the_player_back_in_the_chase() {
        let now = Instant::now();
        let mut hunt = Hunt::default();
        hunt.tell(4, 2, "Kyle", now);
        hunt.out = Some(HuntOut::Dead { by: 4 });
        hunt.tell(5, 2, "Jan", now);
        assert_eq!(hunt.out, None);
        hunt.out = Some(HuntOut::Cheat);
        hunt.hunted();
        assert_eq!(hunt.out, None);
    }

    #[test]
    fn a_box_that_blew_up_is_told_to_the_other_hunters() {
        // The hunter who got the credits, in the centre.
        let (centre, words) = laugh_words(2, Some(4), 100, 4, "Kyle", "Jan").unwrap();
        assert!(centre);
        assert_eq!(words, "Kyle^7's box blew up!\nYou get their 100 credits");
        // Another hunter, in the chat.
        let (centre, words) = laugh_words(2, Some(4), 100, 6, "Kyle", "Jan").unwrap();
        assert!(!centre);
        assert!(words.contains("Jan^7 gets their 100 credits"));
        // Nobody got them, or a free roll passed nothing.
        assert_eq!(
            laugh_words(2, None, 0, 6, "Kyle", "Kyle").unwrap().1,
            "Box hunt: Kyle^7's box blew up"
        );
        assert!(!laugh_words(2, Some(4), 0, 4, "Kyle", "Jan").unwrap().0);
        // The opener's own box tells them.
        assert_eq!(laugh_words(2, Some(4), 100, 2, "Kyle", "Jan"), None);
    }

    #[test]
    fn a_told_mark_gets_the_card_until_hunted() {
        let now = Instant::now();
        let mut hunt = Hunt::default();
        let (centre, _) = hunt.tell(4, 2, "^1Kyle", now);
        assert!(!centre);
        let card = hunt.card().expect("a card");
        assert_eq!(
            (card.slot, card.name.as_str(), card.own),
            (4, "^1Kyle", false)
        );
        // The player as the mark: their own card.
        hunt.tell(2, 2, "Sol", now);
        assert!(hunt.card().is_some_and(|card| card.own));
        // Hunted: no card, and the last read's mark is not told again.
        hunt.state = Some((
            "server".to_owned(),
            HuntState {
                hunting: true,
                mark: Some(wire::HuntMark {
                    slot: 2,
                    name: "Sol".to_owned(),
                    until: 0,
                    ..wire::HuntMark::default()
                }),
                ..HuntState::default()
            },
        ));
        hunt.hunted();
        assert!(hunt.card().is_none() && hunt.told.is_none() && hunt.mark().is_none());
    }

    #[test]
    fn the_player_is_the_mark_by_the_read_or_the_event() {
        let now = Instant::now();
        let mut hunt = Hunt::default();
        assert!(!hunt.marked(2));
        // Told by the hub's event, before any read.
        hunt.tell(2, 2, "Sol", now);
        assert!(hunt.marked(2));
        hunt.tell(4, 2, "Kyle", now);
        assert!(!hunt.marked(2));
        hunt.hunted();
        hunt.state = Some((
            "server".to_owned(),
            HuntState {
                hunting: true,
                mark: Some(wire::HuntMark {
                    slot: 2,
                    ..wire::HuntMark::default()
                }),
                ..HuntState::default()
            },
        ));
        assert!(hunt.marked(2) && !hunt.marked(3));
    }

    #[test]
    fn a_mark_ends_a_little_before_the_hub_says() {
        let now = Instant::now();
        let mark = |until, left| wire::HuntMark {
            until,
            left,
            ..wire::HuntMark::default()
        };
        // The seconds left, whatever this clock says.
        assert_eq!(
            mark_ends(&mark(0, Some(30)), now, 5_000),
            now + Duration::from_secs(29)
        );
        // An older hub: its end by this clock.
        assert_eq!(
            mark_ends(&mark(1_030, None), now, 1_000),
            now + Duration::from_secs(29)
        );
        // Over already.
        assert_eq!(mark_ends(&mark(900, None), now, 1_000), now);
    }

    #[test]
    fn the_hunt_is_read_every_while_and_soon_after_an_event() {
        let now = Instant::now();
        let mut hunt = Hunt::default();
        assert!(hunt.due(now));
        hunt.asked = Some(now);
        assert!(!hunt.due(now + Duration::from_secs(5)));
        assert!(hunt.due(now + READ_EVERY));
        hunt.stale = true;
        assert!(!hunt.due(now + Duration::from_secs(1)));
        assert!(hunt.due(now + READ_GAP));
    }
}
