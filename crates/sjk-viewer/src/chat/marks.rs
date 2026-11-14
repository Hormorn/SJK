//! What SJK knows of the player in each slot, for their game chat lines
//! (`docs/hub-chat.md`, "How a line looks"): whether they play on SJK (the hub holds a
//! claim for that slot under the name the game shows), whether they are verified, and
//! their name as they wear it when the server dropped some of its characters, and the
//! client they play on (`chat/client_icons.rs`): its icon takes SJK's emblem's place, from
//! the claim, or alone without a tick for a slot where a player in SJK chat on a client
//! that claims nothing seems to sit (the scoreboard's name rule, `scoreboard/chat_readers.rs`).
//!
//! A game server carries names as bytes of the old code pages: a JoF JA+ server drops
//! an emoji from a name (`^0JoF J S^7olol 🦊` reached the chat as `^0JoF J S^7olol `,
//! Sol's report, 16/10/2026). An SJK player claims their slot at the hub with the name
//! as they typed it when it is the game's name plus such characters ([`restores`]), so
//! other SJK players show it whole, and only then: a claim never changes a name into
//! another one.

use sjk_identity::Snapshot;

/// What the chat shows of the player in one slot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SlotMark {
    /// The hub knows them in this slot: SJK's emblem (or their client's icon) goes before
    /// their name.
    pub(crate) sjk: bool,
    /// The client they play on when it is not SJK: its icon instead of SJK's emblem.
    pub(crate) client: Option<&'static super::client_icons::ClientIcon>,
    pub(crate) verified: bool,
    /// Their tick, as the hub sent it.
    pub(crate) tick: sjk_identity::Tick,
    /// Their trust (`ui_renderer::mark_texture`), none when verified or staff.
    pub(crate) trust: Option<sjk_identity::Trust>,
    /// Their name emoji, shown at the right of their name; empty for none.
    pub(crate) emoji: String,
    /// Their name as they wear it, when the game's lost characters of it.
    pub(crate) name: Option<String>,
    /// Their identity key, so the server's copy of an SJK chat message is hidden only
    /// when it comes from its sender (`chat/server_copy.rs`); empty when unknown.
    pub(crate) key_id: String,
}

/// Whether `worn` is `shown` with characters the game server dropped put back: the
/// two differ, and leaving out of `worn` every character the old code pages have no
/// byte for (an emoji, a kanji) gives `shown`, spaces at either end aside.
pub(crate) fn restores(worn: &str, shown: &str) -> bool {
    worn != shown && {
        let mut kept = String::with_capacity(worn.len());
        kept.extend(worn.chars().filter(|&character| {
            sjk_protocol::windows_1252_byte(character).is_some()
                || sjk_protocol::windows_1251_byte(character).is_some()
        }));
        kept.trim() == shown.trim()
    }
}

/// The name an SJK player claims their slot with: the name they wear (the `name`
/// setting) when it [`restores`] the one the game shows, else the game's.
pub(crate) fn claimed<'a>(worn: &'a str, shown: &'a str) -> &'a str {
    if restores(worn, shown) { worn } else { shown }
}

/// Every slot's mark from the hub's claims on this server: `shown` gives the name the
/// game shows in a slot, `own` the player's slot and `worn` the name they wear;
/// `unclaimed` (bits) the slots of players on a client that claims nothing
/// (`player_identity::unclaimed_slots`), who get its icon alone.
pub(crate) fn marks<'a>(
    snapshot: &Snapshot,
    shown: impl Fn(usize) -> Option<&'a str>,
    own: Option<usize>,
    worn: &str,
    unclaimed: u64,
) -> Vec<SlotMark> {
    let mut marks = vec![SlotMark::default(); crate::chat_mutes::SLOTS];
    for (slot, mark) in marks.iter_mut().enumerate() {
        let Some(name) = shown(slot) else {
            continue;
        };
        if own == Some(slot) {
            // The player's own line: the hub answered them, whatever it lists.
            if let Some(me) = &snapshot.me {
                *mark = SlotMark {
                    sjk: true,
                    client: None,
                    verified: me.verified,
                    tick: me.tick,
                    trust: me.trust,
                    emoji: sjk_identity::name_emoji::for_display(&me.emoji).to_owned(),
                    name: restores(worn, name).then(|| worn.to_owned()),
                    key_id: me.key_id.clone(),
                };
            }
            continue;
        }
        let Some(player) = u8::try_from(slot)
            .ok()
            .and_then(|slot| snapshot.badge(slot, name))
        else {
            if slot < 64 && unclaimed & 1 << slot != 0 {
                // A name proves nothing: the client's icon, no tick nor key.
                *mark = SlotMark {
                    sjk: true,
                    client: super::client_icons::unclaimed(),
                    ..SlotMark::default()
                };
            }
            continue;
        };
        *mark = SlotMark {
            sjk: true,
            client: super::client_icons::find(&player.client),
            verified: player.verified,
            tick: player.tick,
            trust: player.trust,
            emoji: sjk_identity::name_emoji::for_display(&player.emoji).to_owned(),
            name: restores(&player.claimed_name, name).then(|| player.claimed_name.clone()),
            key_id: player.key_id.clone(),
        };
    }
    marks
}

/// What [`ChatOverlay::follow_marks`] last worked the marks out from: the hub's
/// players' revision, who is in each slot, the name the player wears and the slots of
/// players on a client that claims nothing.
pub(super) type MarksKey = (
    u64,
    [Option<sjk_client::ChatTarget>; crate::chat_mutes::SLOTS],
    String,
    u64,
);

impl super::ChatOverlay {
    /// Keep the slots' marks in step with the hub's claims, the server's players and
    /// the name the player wears: worked out again only when one of them changed, so a
    /// frame costs a comparison. Outside a game the marks are cleared.
    pub(crate) fn follow_marks(
        &mut self,
        revision: u64,
        worn: &str,
        own: Option<usize>,
        live: bool,
        unclaimed: u64,
    ) {
        if !live {
            if self.shown_for_shot() {
                // A world shot has no session; it sets its marks itself.
                return;
            }
            self.slot_marks.clear();
            self.marks_key = None;
            return;
        }
        let roster = self.roster_targets();
        if self
            .marks_key
            .as_ref()
            .is_some_and(|(seen, targets, name, readers)| {
                *seen == revision && *targets == roster && name == worn && *readers == unclaimed
            })
        {
            return;
        }
        let shown = |slot: usize| {
            let name = self.player_label(slot as u16);
            (!name.is_empty()).then_some(name)
        };
        self.slot_marks = crate::player_identity::with_snapshot(|snapshot| {
            marks(snapshot, shown, own, worn, unclaimed)
        })
        .unwrap_or_default();
        self.marks_key = Some((revision, roster, worn.to_owned(), unclaimed));
    }

    /// The slot whose player sent a line the server did not attribute, as JA+ servers
    /// send chat: the one player whose name, as the game shows it, starts the line,
    /// then `: ` (or the team's and a whisper's brackets). `None` when no player or
    /// more than one (two players wearing one name) fits.
    pub(super) fn line_sender(&self, display: &str) -> Option<sjk_client::ChatTarget> {
        sjk_client::line_sender(display, None, &self.roster).target()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_the_server_cut_an_emoji_from_is_restored() {
        assert!(restores("^0JoF J S^7olol 🦊", "^0JoF J S^7olol "));
        assert!(restores("^0JoF J S^7olol 🦊", "^0JoF J S^7olol"));
        assert!(restores("🦊Sol", "Sol"));
        assert!(restores("Sol ❤️ 日本", "Sol  "));
        assert_eq!(claimed("^1Sol🦊", "^1Sol"), "^1Sol🦊");
    }

    #[test]
    fn a_claim_never_changes_a_name() {
        // The same name, or one the server kept whole.
        assert!(!restores("Sol", "Sol"));
        assert!(!restores("Sol 🦊", "Sol 🦊"));
        // Letters the server could carry are not the server's loss.
        assert!(!restores("Solol 🦊", "Sol"));
        assert!(!restores("^1Sol 🦊", "^2Sol"));
        assert!(!restores("Sol(2) 🦊", "Sol"));
        assert!(!restores("Søl 🦊", "Sol"));
        assert_eq!(claimed("Solol", "Sol"), "Sol");
    }

    /// A claim from JoF EternalJK gives its icon with the player's tick; a slot where a
    /// player in SJK chat on a client that claims nothing seems to sit gets the icon alone.
    #[test]
    fn a_line_shows_its_players_client_with_the_claims_tick_only() {
        let mut snapshot =
            sjk_identity::Snapshot::blank(&sjk_identity::Identity::from_seed([7; 32]));
        let claim = |slot: u8, name: &str, client: &str| -> sjk_identity::Presence {
            serde_json::from_value(serde_json::json!({
                "slot": slot, "claimed_name": name, "key_id": format!("{slot:016x}"),
                "name": name, "verified": true, "client": client,
            }))
            .expect("a claim")
        };
        snapshot.players = vec![claim(1, "Vinry", "ejk"), claim(2, "Sol", "")];
        let names = ["", "Vinry", "Sol", "Figo", "Kyle"];
        let shown = |slot: usize| names.get(slot).copied().filter(|name| !name.is_empty());
        let marks = marks(&snapshot, shown, None, "", 1 << 3);
        let ejk = super::super::client_icons::find("ejk");
        assert!(ejk.is_some());
        assert_eq!(
            (marks[1].sjk, marks[1].client, marks[1].verified),
            (true, ejk, true)
        );
        assert_eq!((marks[2].sjk, marks[2].client), (true, None));
        assert_eq!(
            (marks[3].sjk, marks[3].client, marks[3].verified),
            (true, ejk, false)
        );
        assert!(marks[3].key_id.is_empty());
        assert_eq!(marks[4], SlotMark::default());
    }

    /// The game's chat with SJK players' lines among others (`docs/hub-chat.md`, "How a
    /// line looks"), rendered off screen at 1080p and 4K.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn game_lines_with_marks() {
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
                let names = [
                    (0, "^0JoF J S^7olol "),
                    (3, "^0(^9JoF^8A^0)^0Fi^7go^0(^8A^0)"),
                    (5, "dart_mario"),
                    (7, "^3D^5avy ^3J^5ones"),
                ];
                for (slot, name) in names {
                    game.replace_config_string(
                        1131 + slot,
                        format!("n\\{name}\\t\\0").into_bytes(),
                    )
                    .unwrap();
                }
                gpu.chat.update_roster(&game);
                gpu.chat.slot_marks = vec![SlotMark::default(); crate::chat_mutes::SLOTS];
                gpu.chat.slot_marks[0] = SlotMark {
                    sjk: true,
                    client: None,
                    verified: true,
                    tick: sjk_identity::Tick::None,
                    trust: None,
                    emoji: "⚔️".to_owned(),
                    name: Some("^0JoF J S^7olol 🦊".to_owned()),
                    key_id: String::new(),
                };
                // JoF EternalJK: Figo claims (icon and tick), dart_mario reads SJK chat
                // without claiming (icon alone).
                let ejk = crate::chat::client_icons::find("ejk");
                gpu.chat.slot_marks[3] = SlotMark {
                    sjk: true,
                    client: ejk,
                    verified: true,
                    tick: sjk_identity::Tick::Jof,
                    ..SlotMark::default()
                };
                gpu.chat.slot_marks[5] = SlotMark {
                    sjk: true,
                    client: ejk,
                    ..SlotMark::default()
                };
                gpu.chat.slot_marks[7] = SlotMark {
                    sjk: true,
                    client: None,
                    verified: false,
                    tick: sjk_identity::Tick::None,
                    trust: None,
                    emoji: "🐺".to_owned(),
                    name: None,
                    key_id: String::new(),
                };
                let now = std::time::Instant::now();
                let state = |messages| sjk_identity::ChatState {
                    messages,
                    loaded: Some(1),
                    ..sjk_identity::ChatState::default()
                };
                gpu.chat
                    .sync_sjk(&state(Default::default()), |_| false, now);
                for line in [
                    "^0JoF J S^7olol ^7: ^2gg, nice duel",
                    "^0(^9JoF^8A^0)^0Fi^7go^0(^8A^0)^7: ^2^8SA means sith assassin?",
                    "dart_mario^7: ^2otherwise gg xD",
                    "^3D^5avy ^3J^5ones^7: ^2duel?",
                    "^3Server^7: the next map is ffa3",
                ] {
                    gpu.chat.receive(
                        sjk_client::ServerEventKind::Chat,
                        line.to_owned(),
                        None,
                        now,
                    );
                }
                gpu.chat.receive(
                    sjk_client::ServerEventKind::TeamChat,
                    "(^0JoF J S^7olol ^7): ^5push left".to_owned(),
                    None,
                    now,
                );
                let message = sjk_identity::ChatMessage {
                    reactions: Default::default(),
                    id: 2,
                    at: 0,
                    key_id: "00000000000000ab".to_owned(),
                    name: "^0JoF J S^7olol 🦊".to_owned(),
                    verified: true,
                    tick: sjk_identity::Tick::None,
                    staff: false,
                    creator: false,
                    trust: None,
                    emoji: String::new(),
                    client: String::new(),
                    text: "and this one goes through the SJK chat".to_owned(),
                    holocron: None,
                    official: None,
                    channel: Default::default(),
                };
                gpu.chat
                    .sync_sjk(&state(vec![message].into_iter().collect()), |_| false, now);
                let path = crate::world_shot::shoot(&mut gpu, 10, &format!("chat-marks-{suffix}"));
                println!("{}", path.display());
            });
        }
    }
}
