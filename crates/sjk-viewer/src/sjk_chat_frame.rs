//! The per-frame hook of the SJK chat (`docs/hub-chat.md`): when the identity
//! service's chat changed, bring the game's chat feed in line with it (new messages,
//! deleted ones, why a message was refused), optionally put live messages in the
//! console, and play the SJK chat's sound for a new message, the player's own included
//! (`audio/sjk_chat_sound.rs`). Each frame costs one look at the chat's revision and the
//! mutes'; nothing is copied unless one changed.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// The environment variable that keeps the SJK chat off for one run without changing
/// the saved `cl_sjkChat`: the player appears offline in the menus (the hub lists a key
/// as online only while it reads the chat, `docs/hub-chat.md`). Any value but empty or
/// `0` turns it on.
pub(crate) const APPEAR_OFFLINE: &str = "SJK_APPEAR_OFFLINE";

static OFFLINE: AtomicBool = AtomicBool::new(false);

/// Read [`APPEAR_OFFLINE`] at start.
pub(crate) fn appear_offline_from_environment() {
    let on = std::env::var(APPEAR_OFFLINE).is_ok_and(|value| !value.is_empty() && value != "0");
    OFFLINE.store(on, Ordering::Relaxed);
    if on {
        crate::log::progress(format_args!(
            "SJK chat off for this run ({APPEAR_OFFLINE}): appearing offline"
        ));
    }
}

/// Whether this run keeps the SJK chat off whatever `cl_sjkChat` says.
pub(crate) fn appears_offline() -> bool {
    OFFLINE.load(Ordering::Relaxed)
}

/// Why the chat is off, for a refused Send.
pub(crate) fn off_reason() -> String {
    if appears_offline() {
        format!("SJK chat is off for this run ({APPEAR_OFFLINE}: appearing offline)")
    } else {
        "SJK chat is off (cl_sjkChat 1 turns it on)".to_owned()
    }
}

impl crate::console::ViewerConsole {
    /// Whether the SJK chat shows and is read: `cl_sjkChat` on, and this run not
    /// appearing offline ([`APPEAR_OFFLINE`]).
    pub(crate) fn sjk_chat_on(&self) -> bool {
        self.bool_cvar("cl_sjkChat") != Some(false) && !appears_offline()
    }
}

impl GpuState {
    /// Follow the SJK chat in the chat feed while `cl_sjkChat` is on.
    pub(crate) fn update_sjk_chat(&mut self) {
        self.send_server_copies(Instant::now());
        let Some(console) = self.console.as_ref() else {
            return;
        };
        if !console.sjk_chat_on() {
            // No hub chat at all: no clan chat buttons either.
            self.chat.set_clan_chats([false; 2]);
            return;
        }
        let feed = console.bool_cvar(FEED_CVAR) != Some(false);
        let in_console = console.bool_cvar(CONSOLE_CVAR) != Some(false);
        // The clan chats the player joined (`chat/tabs.rs`): read with the chat, a
        // button each in the composer.
        let clans =
            player_identity::follow_clan_chats(|clan| chat::tabs::join_choice(console, clan));
        self.chat.set_clan_chats(clans);
        self.chat.set_sjk_feed_visible(feed);
        let sound = sound_on(console);
        let mutes = player_mutes::revision();
        let Some(mark) = player_identity::with_chat(|chat| {
            (
                chat.revision,
                chat.outcome.as_ref().map_or(0, |outcome| outcome.serial),
                mutes,
            )
        }) else {
            return;
        };
        if !self.chat.sjk_changed(mark) {
            return;
        }
        // The mutes live under the service's lock too: read them before it is taken.
        let muted = player_mutes::muted_keys();
        let is_muted = |key: &str| muted.iter().any(|m| m == key);
        let now = Instant::now();
        let own_key = player_identity::own_key_id();
        // Quiet mode (`chat/quiet.rs`): only the player's own and their duel partner's
        // messages sound.
        self.chat.set_sjk_own_key(own_key.as_deref());
        let quiet = self.chat.quiet_keys();
        let hushed = |key: &str| {
            is_muted(key)
                || quiet
                    .as_ref()
                    .is_some_and(|keys| !keys.iter().any(|k| !k.is_empty() && k == key))
        };
        let chat = &mut self.chat;
        let update = player_identity::with_chat(|state| {
            let after = chat.sync_sjk_in_feed(state, is_muted, now, feed);
            chat.sjk_outcome(state.outcome.as_ref(), now);
            let joined = state
                .messages
                .iter()
                .filter(|message| after.is_some_and(|after| message.id > after));
            chat.follow_server_copies(joined, own_key.as_deref(), state.outcome.as_ref(), now);
            let news = after.is_some_and(|after| {
                chat::sjk::worth_a_sound(chat::sjk::merged(state), after, hushed)
            });
            let console_lines = if in_console {
                after.map_or_else(Vec::new, |after| {
                    chat::sjk::console_lines(chat::sjk::merged(state), after, is_muted)
                })
            } else {
                Vec::new()
            };
            (news, console_lines)
        });
        let Some((news, console_lines)) = update else {
            return;
        };
        if let Some(console) = self.console.as_mut() {
            for line in console_lines {
                console.push_chat_line(line.clone(), &line, None);
            }
        }
        if sound && news && self.chat.sjk_sound_due(now) {
            audio::ui_cues::post(audio::ui_cues::Cue::SjkChat);
        }
    }

    /// Whether JoF tools say SJK chat in this server's All chat: loaded (a JoF server,
    /// `mod_jof` on) and in a match on a game server (`docs/mods.md`, "SJK chat on JoF
    /// servers"). Nowhere else, and never in a build without `mod-jof`.
    pub(crate) fn sjk_chat_said_in_all(&self) -> bool {
        #[cfg(feature = "mod-jof")]
        {
            self.live_session
                .as_ref()
                .is_some_and(|session| !session.is_local())
                && self.console.as_ref().is_some_and(|console| {
                    console.mods.loaded(sjk_mod_jof::ID)
                        && console.bool_cvar("mod_jof").unwrap_or(false)
                })
        }
        #[cfg(not(feature = "mod-jof"))]
        {
            false
        }
    }

    /// While JoF tools say SJK chat in this server's All chat, what the player said on
    /// the SJK chat goes there too once the hub took it (`chat/server_copy.rs`); anywhere
    /// else it goes nowhere else, and nothing waits.
    fn send_server_copies(&mut self, now: Instant) {
        let said = chat::server_copy::take_said();
        if !self.sjk_chat_said_in_all() {
            self.chat.set_said_in_all(None);
            self.chat.stop_server_copies();
            return;
        }
        #[cfg(feature = "mod-jof")]
        {
            use sjk_mod_jof::sjk_chat;
            self.chat
                .set_said_in_all(Some(chat::server_copy::SaidInAll {
                    note: sjk_chat::COMPOSER_NOTE,
                    only: sjk_chat::ONLY_SJK,
                    only_title: sjk_chat::ONLY_SJK_TITLE,
                    only_why: &sjk_chat::ONLY_SJK_WHY,
                }));
            let server = self
                .live_session
                .as_ref()
                .map(|session| session.server_id());
            for (text, id) in self.chat.due_server_copies(said, server, now) {
                for command in sjk_mod_jof::sjk_chat::copy_commands(&text, id) {
                    if let Some(session) = self.live_session.as_mut()
                        && let Err(error) = session.send_reliable_command_from(
                            command.as_bytes(),
                            sjk_client::ReliableCommandOrigin::Chat,
                        )
                    {
                        eprintln!("failed to say an SJK chat message on the server: {error}");
                    }
                }
            }
        }
        #[cfg(not(feature = "mod-jof"))]
        let _ = (said, now);
    }
}

/// Whether the SJK chat's sound plays: `cl_sjkChatSound`, under the master chat sound
/// switch `cg_chatSounds`, which silences every chat sound (`chat_sounds.rs`).
fn sound_on(console: &console::ViewerConsole) -> bool {
    crate::chat_sounds::enabled(Some(console), crate::chat_sounds::Kind::Sjk)
}

/// `cl_sjkChatSound`: whether a new SJK chat message plays a sound.
pub(crate) const SOUND_CVAR: &str = "cl_sjkChatSound";

/// `cl_sjkChatFeed`: whether live SJK messages join the in-game general chat feed.
pub(crate) const FEED_CVAR: &str = "cl_sjkChatFeed";

/// `cl_sjkChatConsole`: whether live SJK messages enter console scrollback.
pub(crate) const CONSOLE_CVAR: &str = "cl_sjkChatConsole";
