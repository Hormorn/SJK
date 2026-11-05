//! Quests (`toy_quests`; wonner67's idea in #ideas, 04/11/2026): now and then the SJK
//! hub gives one player taking part a quest with a minute to finish it. The first kind,
//! the social quest, asks them to hug (JA+ `amhug`) or kiss (`amkiss`) each of the
//! players it names; a finished quest wins a holocron (an Echo on JoF's servers, as one
//! earned by playing: within the day's cap), a few times a day.
//!
//! - **Opt-in:** the switch (`toy_quests`, the key bindings) is the consent to take part:
//!   only players with it on are given a quest, named in one, or told of them. It stays
//!   as last switched ([`crate::toy_saved::QUESTS_CVAR`]).
//! - **Told:** the switch travels in the player's look ([`TOY`] among its toys), which is
//!   what the hub reads when it gives a quest.
//! - **Proof:** the hub never sees an emote. This client, when its player is a target
//!   still to do, watches its own player state: the frame the game server puts it in the
//!   hugged or kissed half of the pair (`BOTH_HUGGEE1`, `BOTH_KISSEE`, set by the server,
//!   never predicted) with the quest's player standing next to it, it tells the hub who
//!   did it (`POST /v1/quests/done`). Nobody finishes a quest alone.
//!
//! Every number on the hub's side is provisional (Sol's to tune).

use crate::GpuState;
use sjk_identity::quests::{self as wire, Emote, Quest, QuestEvent, QuestState};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// The toy's console command: the switch (take part, or not).
pub(crate) const COMMAND: &str = "toy_quests";
/// The toy's id in looks: the hub gives quests among the players whose look has it.
pub(crate) const TOY: &str = wire::TOY;
/// The toy's icon (key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_quests.png";

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 1] = [(ICON, include_bytes!("../assets/quests/toy_quests.png"))];

/// How often quests are read while the player takes part (a new one is also told at
/// once by the hub's toy event).
const READ_EVERY: Duration = Duration::from_secs(20);
/// Soonest a read follows another asked by an event.
const READ_GAP: Duration = Duration::from_secs(2);
/// How long before the hub's end of a quest this client stops counting it: a report
/// after the end is not counted.
const END_EARLY: Duration = Duration::from_secs(1);
/// How far the quest's player may stand from a target for its hug or kiss to be theirs,
/// in game units (the pair stands face to face, closer than this).
const REACH: f32 = 160.0;
/// `ANIM_TOGGLEBIT`: flipped when the same animation restarts.
const ANIM_TOGGLEBIT: u16 = 0x800;
/// `PMF_FOLLOW`: the player state is another player's, followed.
const PMF_FOLLOW: u16 = 0x1000;

/// The hugged and the kissed halves of JA+'s paired emotes, as the wire numbers
/// animations.
static HUGGED: LazyLock<Option<u16>> = LazyLock::new(|| animation("BOTH_HUGGEE1"));
static KISSED: LazyLock<Option<u16>> = LazyLock::new(|| animation("BOTH_KISSEE"));

fn animation(name: &str) -> Option<u16> {
    sjk_game_jka::legacy_animation_index(name).and_then(|index| u16::try_from(index).ok())
}

/// The animation that shows the player received `emote`.
fn received(emote: Emote) -> Option<u16> {
    match emote {
        Emote::Hug => *HUGGED,
        Emote::Kiss => *KISSED,
    }
}

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Quests", FILES.iter().copied())?;
    Ok(())
}

/// The toy's state, and the quests as this client follows them.
#[derive(Default)]
pub(crate) struct Quests {
    on: bool,
    /// The quests last read, and the server they were read on.
    state: Option<(String, QuestState)>,
    /// The last read taken from the identity snapshot.
    seen: u64,
    /// When quests were last asked for, and whether at once (an event changed them).
    asked: Option<Instant>,
    stale: bool,
    /// Whether the last read said the player was away; a change is read at once.
    away: Option<bool>,
    /// The quest last told, by its player's slot and end, so each is told once, and when
    /// (its card pulses as it arrives).
    told: Option<(u8, i64)>,
    told_at: Option<Instant>,
    /// When the quest last read ends, by this client's clock ([`END_EARLY`] before the
    /// hub's).
    ends: Option<Instant>,
    /// Whether the player was in the received half of the quest's emote last frame, so
    /// each hug or kiss is told once.
    received: bool,
}

impl Quests {
    /// `toy_quests`: on or off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// On (`true`) or off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the toy is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// The quest under way, while the player takes part.
    fn quest(&self) -> Option<&Quest> {
        self.state
            .as_ref()
            .filter(|(_, state)| state.questing)
            .and_then(|(_, state)| state.quest.as_ref())
    }

    /// The quest is over (finished, failed or its time up): none until the next read.
    fn over(&mut self) {
        self.ends = None;
        self.received = false;
        if let Some((_, state)) = &mut self.state {
            state.quest = None;
        }
    }

    /// Whether quests should be read at `now`.
    fn due(&self, now: Instant) -> bool {
        self.asked.is_none_or(|asked| {
            let since = now.saturating_duration_since(asked);
            since >= READ_EVERY || (self.stale && since >= READ_GAP)
        })
    }
}

/// When `quest`, read at `now`, ends by this client's clock, [`END_EARLY`] before the
/// hub's.
fn quest_ends(quest: &Quest, now: Instant) -> Instant {
    (now + Duration::from_secs(quest.left))
        .checked_sub(END_EARLY)
        .unwrap_or(now)
        .max(now)
}

/// `names` joined for a sentence: `a`, `a^7 and b`, `a^7, b^7 and c`.
fn listed(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{}^7 and {last}", rest.join("^7, ")),
    }
}

/// What the player on slot `own` is told when the player on `slot` (`name`) is given a
/// quest to `emote` the players `targets` (their names): centred or not, and the text.
fn given_words(
    own: u8,
    slot: u8,
    name: &str,
    emote: Emote,
    targets: &[(u8, String)],
) -> (bool, String) {
    let verb = emote.verb();
    let names: Vec<String> = targets.iter().map(|(_, name)| name.clone()).collect();
    if slot == own {
        return (
            true,
            format!(
                "^3Quest!^7 {verb} {}^7 within a minute\nFace them and use /{}",
                listed(&names),
                emote.command()
            ),
        );
    }
    if targets.iter().any(|(target, _)| *target == own) {
        return (
            true,
            format!("^3Quest:^7 {name}^7 must {verb} you\nStay close to them!"),
        );
    }
    (
        false,
        format!(
            "^3Quest:^7 {name}^7 must {verb} {}^7 within a minute",
            listed(&names)
        ),
    )
}

/// What the player on slot `own` is told when the player on `slot` (`name`) finished
/// the quest and won a holocron of `tier` (None past the day's cap), an Echo on JoF's
/// servers ([`crate::terms`]).
fn won_words(own: u8, slot: u8, name: &str, tier: Option<&str>) -> (bool, String) {
    let (centre, text) = match (slot == own, tier) {
        (true, None) => (true, "^3Quest done!^7\nNo more holocrons today".to_owned()),
        (true, Some(tier)) => (
            true,
            format!("^3Quest done!^7\nYou found {}", tier_holocron(tier)),
        ),
        (false, _) => (false, format!("^3Quest:^7 {name}^7 finished their quest")),
    };
    (centre, crate::terms::say(&text).into_owned())
}

/// `an Uncommon holocron`, `a Rare holocron` and so on, for a tier's id.
fn tier_holocron(tier: &str) -> String {
    let mut letters = tier.chars();
    let named: String = letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default();
    let article = if named.starts_with(['A', 'E', 'I', 'O', 'U']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {named} holocron")
}

/// What the player on slot `own` is told when the quest of the player on `slot`
/// (`name`) ran out.
fn failed_words(own: u8, slot: u8, name: &str) -> (bool, String) {
    if slot == own {
        (true, "^3Quest failed^7\nTime is up".to_owned())
    } else {
        (false, format!("^3Quest:^7 {name}^7 ran out of time"))
    }
}

/// A player of the quest this client's player should find, marked over their head with an
/// arrow pointing their way (`mystery_box::label`): the quest's targets still to do for
/// its player, the quest's player for a target.
pub(crate) struct Marked {
    pub(crate) slot: u8,
    pub(crate) headline: String,
    pub(crate) line: String,
}

/// The quest's card at the top of the screen (`mystery_box::label`, under the box hunt's):
/// a caption with the seconds left, and what the player is to do or wait for.
pub(crate) struct QuestCard {
    pub(crate) caption: String,
    pub(crate) line: String,
    /// The player is the quest's own (a brighter plate).
    pub(crate) own: bool,
    /// When it was told: it pulses as it arrives.
    pub(crate) since: Instant,
}

/// Who `own` marks in `quest` asking for `emote`: its targets still to do for its player,
/// its player for a target still to do.
fn marked(own: u8, quest: &Quest, emote: Emote) -> Vec<Marked> {
    if quest.slot == own {
        return quest
            .targets
            .iter()
            .filter(|target| !target.done)
            .map(|target| Marked {
                slot: target.slot,
                headline: emote.verb().to_uppercase(),
                line: format!("Quest: /{}", emote.command()),
            })
            .collect();
    }
    if quest.awaits(own) {
        return vec![Marked {
            slot: quest.slot,
            headline: "QUEST".to_owned(),
            line: format!("They must {} you", emote.verb()),
        }];
    }
    Vec::new()
}

/// What the quest's card says to `own`, `left` seconds before the quest ends: None for a
/// player it does not name.
fn card_words(own: u8, quest: &Quest, emote: Emote, left: u64) -> Option<(String, String)> {
    let caption = format!("QUEST \u{b7} {left}s LEFT");
    if quest.slot == own {
        let names: Vec<String> = quest
            .targets
            .iter()
            .map(|target| {
                if target.done {
                    format!("{}^7 (done)", target.name)
                } else {
                    target.name.clone()
                }
            })
            .collect();
        let verb = emote.verb();
        let line = format!(
            "{}{} {}^7",
            verb[..1].to_uppercase(),
            &verb[1..],
            listed(&names)
        );
        return Some((caption, line));
    }
    let target = quest.targets.iter().find(|target| target.slot == own)?;
    let line = if target.done {
        format!("{}^7 {} you", quest.name, emote.done())
    } else {
        format!("{}^7 must {} you", quest.name, emote.verb())
    };
    Some((caption, line))
}

impl GpuState {
    /// The quest's players this client's player should find, while it takes part and the
    /// quest lasts.
    pub(crate) fn quest_marked(&self) -> Vec<Marked> {
        let Some((own, quest, emote)) = self.quest_in_play(Instant::now()) else {
            return Vec::new();
        };
        marked(own, quest, emote)
    }

    /// The quest's card at `now`, while the player takes part and the quest names them.
    pub(crate) fn quest_card(&self, now: Instant) -> Option<QuestCard> {
        let (own, quest, emote) = self.quest_in_play(now)?;
        let ends = self.quests.ends?;
        let left = ends.saturating_duration_since(now).as_secs_f32().ceil() as u64;
        let (caption, line) = card_words(own, quest, emote, left)?;
        Some(QuestCard {
            caption,
            line,
            own: quest.slot == own,
            since: self.quests.told_at.unwrap_or(now),
        })
    }

    /// The player's slot, the quest under way and its emote, while the player takes part,
    /// the quest is a social one and its end has not come by this client's clock.
    fn quest_in_play(&self, now: Instant) -> Option<(u8, &Quest, Emote)> {
        if !self.quests.on() || self.quests.ends.is_none_or(|ends| now >= ends) {
            return None;
        }
        let session = self.live_session.as_ref()?;
        let own = u8::try_from(session.game_state().client_num).ok()?;
        let quest = self.quests.quest()?;
        Some((own, quest, quest.social()?))
    }
}

impl GpuState {
    /// `toy_quests`: on or off, said on the centre of the screen.
    pub(crate) fn toggle_quests(&mut self) {
        self.quests.toggle();
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            crate::toy_switch_words::switched("Quests", self.quests.on()),
            None,
            Instant::now(),
        );
    }

    /// The frame's quests: read them, take the hub's events of them (`events`, this
    /// server's toy events), and tell the hub when the quest's player hugged or kissed
    /// this client's player, a target.
    pub(crate) fn update_quests(&mut self, events: &[sjk_identity::ToyEvent]) {
        let now = Instant::now();
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.quests.state = None;
            self.quests.told = None;
            self.quests.over();
            return;
        };
        let server = session.server().to_string();
        let Ok(own) = u8::try_from(session.game_state().client_num) else {
            return;
        };
        let on = self.quests.on();
        let away = !self.actively_playing(now) || self.chat.is_typing();
        let quests = &mut self.quests;
        if on && quests.away != Some(away) {
            quests.away = Some(away);
            quests.stale = true;
        }
        // A read for this server.
        let read = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .quests
                .as_ref()
                .filter(|(serial, read_on, _)| *serial != quests.seen && *read_on == server)
                .cloned()
        })
        .flatten();
        let mut lines = Vec::new();
        if let Some((serial, read_on, mut state)) = read {
            quests.seen = serial;
            if let Some(quest) = state.quest.as_ref().filter(|_| state.questing) {
                let ends = quest_ends(quest, now);
                if ends > now && quest.social().is_some() {
                    quests.ends = Some(ends);
                } else {
                    state.quest = None;
                }
            }
            quests.state = Some((read_on, state));
        }
        if quests
            .state
            .as_ref()
            .is_some_and(|(read_on, _)| *read_on != server)
        {
            quests.state = None;
            quests.told = None;
            quests.over();
        }
        if quests.ends.is_some_and(|ends| now >= ends) {
            quests.over();
            quests.stale = true;
        }
        // A quest read before its event arrived is told as the event would.
        if on
            && let Some(quest) = quests.quest()
            && quests.told != Some((quest.slot, quest.until))
            && let Some(emote) = quest.social()
        {
            let targets: Vec<(u8, String)> = quest
                .targets
                .iter()
                .map(|target| (target.slot, target.name.clone()))
                .collect();
            let words = given_words(own, quest.slot, &quest.name, emote, &targets);
            quests.told = Some((quest.slot, quest.until));
            quests.told_at = Some(now);
            lines.push(words);
        }
        for event in events.iter().filter(|event| event.toy == wire::TOY) {
            let Some(said) = wire::parse_event(&event.event) else {
                continue;
            };
            quests.stale = true;
            if !on {
                continue;
            }
            match said {
                // Told from the read that follows (it has the names and the end).
                QuestEvent::Given { .. } => {}
                QuestEvent::Done { target, .. } => {
                    if let Some((_, state)) = &mut quests.state
                        && let Some(quest) = &mut state.quest
                    {
                        for named in quest.targets.iter_mut().filter(|t| t.slot == target) {
                            named.done = true;
                        }
                    }
                }
                QuestEvent::Won { slot, tier } => {
                    quests.over();
                    lines.push(won_words(own, slot, &event.claimed_name, tier.as_deref()));
                }
                QuestEvent::Failed(slot) => {
                    quests.over();
                    lines.push(failed_words(own, slot, &event.claimed_name));
                }
            }
        }
        // The player, a target still to do, received the quest's emote this frame from
        // its player standing beside them: told once.
        let awaited = quests
            .quest()
            .filter(|quest| on && quest.awaits(own))
            .and_then(|quest| Some((quest.slot, received(quest.social()?)?)));
        let snapshot = session.latest_snapshot();
        let receiving = awaited.is_some_and(|(by, animation)| {
            let player = &snapshot.player;
            let shown = [player.leg_animation(), player.torso_animation()]
                .into_iter()
                .any(|shown| shown & !ANIM_TOGGLEBIT == animation);
            let watching = player.movement_flags() & PMF_FOLLOW != 0
                || player.movement_type() == sjk_game_jka::PM_SPECTATOR;
            let own_at = glam::Vec3::from_array(player.origin());
            let beside = snapshot.entities.iter().any(|entity| {
                entity.number() == u16::from(by)
                    && glam::Vec3::from_array(entity.trajectory_base()).distance(own_at) <= REACH
            });
            shown && !watching && beside
        });
        let report = (receiving && !quests.received)
            .then_some(awaited)
            .flatten()
            .map(|(by, _)| by);
        quests.received = receiving;
        if report.is_some()
            && let Some((_, state)) = &mut quests.state
            && let Some(quest) = &mut state.quest
        {
            for named in quest.targets.iter_mut().filter(|t| t.slot == own) {
                named.done = true;
            }
        }
        let read_now = on && quests.due(now);
        if read_now || report.is_some() {
            quests.asked = Some(now);
            quests.stale = false;
        }
        if let Some(by) = report {
            crate::player_identity::with_service(|service| {
                service.quest_done(server.clone(), by, away);
            });
        } else if read_now {
            crate::player_identity::with_service(|service| {
                service.read_quests(server.clone(), away);
            });
        }
        if !on {
            quests.told = None;
            quests.away = None;
            quests.over();
        }
        for (centre, text) in lines {
            self.chat.receive(
                if centre {
                    sjk_client::ServerEventKind::CenterPrint
                } else {
                    sjk_client::ServerEventKind::Print
                },
                text,
                None,
                now,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quest(done: bool) -> Quest {
        Quest {
            kind: "social".to_owned(),
            slot: 4,
            name: "Al".to_owned(),
            emote: "hug".to_owned(),
            targets: vec![
                wire::QuestTarget {
                    slot: 2,
                    name: "Bo".to_owned(),
                    done,
                },
                wire::QuestTarget {
                    slot: 7,
                    name: "Cy".to_owned(),
                    done: false,
                },
            ],
            until: 100,
            left: 42,
            reward: "echo".to_owned(),
        }
    }

    /// The quest's player marks its targets still to do; a target marks the quest's player
    /// until done; anyone else marks nobody.
    #[test]
    fn each_player_marks_whom_they_should_find() {
        let slots = |own, quest: &Quest| -> Vec<u8> {
            marked(own, quest, Emote::Hug)
                .iter()
                .map(|m| m.slot)
                .collect()
        };
        assert_eq!(slots(4, &quest(false)), vec![2, 7]);
        assert_eq!(slots(4, &quest(true)), vec![7]);
        assert_eq!(slots(2, &quest(false)), vec![4]);
        assert!(slots(2, &quest(true)).is_empty());
        assert!(slots(9, &quest(false)).is_empty());
        let over = &marked(4, &quest(false), Emote::Kiss)[0];
        assert_eq!(
            (over.headline.as_str(), over.line.as_str()),
            ("KISS", "Quest: /amkiss")
        );
    }

    #[test]
    fn the_card_counts_down_and_says_what_is_left() {
        let (caption, line) = card_words(4, &quest(true), Emote::Hug, 42).unwrap();
        assert_eq!(caption, "QUEST \u{b7} 42s LEFT");
        assert_eq!(line, "Hug Bo^7 (done)^7 and Cy^7");
        let (_, line) = card_words(7, &quest(true), Emote::Kiss, 5).unwrap();
        assert_eq!(line, "Al^7 must kiss you");
        let (_, line) = card_words(2, &quest(true), Emote::Hug, 5).unwrap();
        assert_eq!(line, "Al^7 hugged you");
        assert!(card_words(9, &quest(true), Emote::Hug, 5).is_none());
    }

    /// The emotes' animations are the game's paired halves, the kissed one the number
    /// anti-kiss answers.
    #[test]
    fn the_received_halves_are_the_games() {
        assert!(HUGGED.is_some());
        assert_eq!(*KISSED, Some(1493));
        assert_ne!(*HUGGED, *KISSED);
    }

    #[test]
    fn names_list_as_a_sentence() {
        let names = |list: &[&str]| list.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>();
        assert_eq!(listed(&names(&["a"])), "a");
        assert_eq!(listed(&names(&["a", "b"])), "a^7 and b");
        assert_eq!(listed(&names(&["a", "b", "c"])), "a^7, b^7 and c");
    }

    #[test]
    fn each_player_is_told_what_concerns_them() {
        let targets = [(2, "Bo".to_owned()), (7, "Cy".to_owned())];
        let (centre, text) = given_words(4, 4, "Al", Emote::Hug, &targets);
        assert!(centre && text.contains("hug Bo^7 and Cy") && text.contains("/amhug"));
        let (centre, text) = given_words(7, 4, "Al", Emote::Kiss, &targets);
        assert!(centre && text.contains("Al^7 must kiss you"));
        let (centre, text) = given_words(9, 4, "Al", Emote::Hug, &targets);
        assert!(!centre && text.contains("Al^7 must hug Bo^7 and Cy"));
        let found = |tier| won_words(4, 4, "Al", tier).1;
        assert!(found(Some("uncommon")).ends_with("You found an Uncommon holocron"));
        assert!(found(Some("rare")).ends_with("You found a Rare holocron"));
        assert!(found(None).contains("No more holocrons"));
        assert!(!failed_words(9, 4, "Al").0);
    }
}
