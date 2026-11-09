//! Presentation-side consumers for typed reliable server-command actions.

use super::Localization;
use crate::chat::ChatOverlay;
use crate::console::ViewerConsole;
use sjk_client::{BaseServerCommandEvent, ClientSession, LegacyWorldAdapter};
use std::time::Instant;

/// Drain reliable text/UI/presentation actions without retaining frame state.
#[allow(clippy::too_many_arguments)]
pub(super) fn consume(
    session: &mut ClientSession,
    localization: &Localization,
    chat: Option<&mut ChatOverlay>,
    mut console: Option<&mut ViewerConsole>,
    mut adapter: Option<&mut LegacyWorldAdapter>,
    watch: &mut crate::clientinfo_refresh::ClientInfoWatch,
    duels: &mut crate::hud::duel_pairs::DuelPairs,
    #[cfg_attr(not(feature = "mod-jof"), allow(unused_mut, unused_variables))] mut audio: Option<
        &mut crate::GameAudio,
    >,
) {
    if let Some(chat) = chat {
        consume_messages(session, localization, chat, console.as_deref_mut(), duels);
    } else {
        session.drain_events().for_each(drop);
    }

    // SJK's admin key login, asked once per level where a server offers it.
    crate::admin_key_login::update(session);
    while let Some(event) = session.pop_base_command_event() {
        match event {
            BaseServerCommandEvent::UnhandledCommand {
                arguments,
                first_observation,
            } => {
                #[allow(unused_mut)]
                let mut handled = false;
                if arguments
                    .first()
                    .is_some_and(|name| name == sjk_protocol::key_login::COMMAND.as_bytes())
                {
                    handled = true;
                    if let Some(console) = &mut console {
                        let folder = console.config_directory().to_path_buf();
                        for line in crate::admin_key_login::answer(session, &arguments, &folder) {
                            console.push_log(line);
                        }
                    }
                }
                #[cfg(feature = "mod-jof")]
                if let Some(console) = &mut console
                    && console.mods.loaded(sjk_mod_jof::ID)
                    && console.bool_cvar("mod_jof").unwrap_or(false)
                    && arguments
                        .first()
                        .is_some_and(|name| name == sjk_mod_jof::dialogue::COMMAND)
                {
                    handled = true;
                    if let Some(event) = sjk_mod_jof::dialogue::parse(&arguments) {
                        console.dialogue_event(event);
                    }
                }
                #[cfg(feature = "mod-jof")]
                if !handled {
                    handled = crate::mods::force_feedback::dispatch(
                        &arguments,
                        console.as_deref(),
                        audio.as_deref_mut(),
                        session.latest_snapshot(),
                    );
                }
                if !handled
                    && first_observation
                    && let Some(console) = &mut console
                    && console.integer_cvar("developer").unwrap_or(0) != 0
                {
                    let name = arguments.first().map(|name| String::from_utf8_lossy(name));
                    console.push_log(format!(
                        "^3Unknown server command: {}",
                        name.as_deref().unwrap_or("")
                    ));
                }
            }
            BaseServerCommandEvent::ForceRank(update) => {
                if let Some(console) = &mut console {
                    console.apply_force_rank(update, session.game_state(), Instant::now());
                }
            }
            BaseServerCommandEvent::RestoreGhoul2(restore) => {
                if restore.immediate_body.is_none()
                    && let Some(adapter) = &mut adapter
                {
                    adapter.restore_client_ghoul(session.game_state(), restore);
                }
            }
            BaseServerCommandEvent::CopyBody(body) => {
                if let Some(adapter) = &mut adapter {
                    adapter.copy_body_identity(&body);
                }
                watch
                    .body_commands
                    .push(BaseServerCommandEvent::CopyBody(body));
            }
            BaseServerCommandEvent::KillGhoul2(number) => {
                if let Some(adapter) = &mut adapter {
                    adapter.kill_body_identity(number);
                }
                watch
                    .body_commands
                    .push(BaseServerCommandEvent::KillGhoul2(number));
            }
            BaseServerCommandEvent::SiegeClassSelect => {
                if let Some(console) = &mut console {
                    console.request_siege_class();
                }
            }
            BaseServerCommandEvent::SiegeProfileMenu => {
                if let Some(console) = &mut console {
                    console.request_siege_profile();
                }
            }
        }
    }
    if let Some(console) = &mut console {
        console.flush_userinfo(session, Instant::now());
    }
}

/// Receive communication from the remote endpoint without touching local actors.
pub(crate) fn consume_messages(
    session: &mut ClientSession,
    localization: &Localization,
    chat: &mut ChatOverlay,
    mut console: Option<&mut ViewerConsole>,
    duels: &mut crate::hud::duel_pairs::DuelPairs,
) {
    chat.update_roster(session.game_state());
    chat.configure(console.as_deref());
    // Quiet mode lets the private duel partner's chat through (`chat/quiet.rs`).
    let snapshot = session.latest_snapshot();
    chat.observe_duel(&snapshot.player, i64::from(snapshot.server_time));
    // Duel starts, read once the events are drained (they borrow the session).
    let mut duel_starts = Vec::new();
    // The server's copy of an SJK chat message waits for the message while the SJK chat
    // reads (`chat/server_copy.rs`); one whose message did not come shows as it came.
    let now = Instant::now();
    let sjk_chat = console
        .as_deref()
        .is_some_and(|console| console.bool_cvar("cl_sjkChat") != Some(false));
    let mut events = chat.released_server_copies(now);
    for event in session.drain_events() {
        if !(sjk_chat && chat.hold_server_copy(&event, now)) {
            events.push(event);
        }
    }
    for event in events {
        let text = localization.translate(&event.text);
        // A blocked player's chat stays out of the console and its alerts, as it stays
        // out of the chat box (hub report #191).
        let blocked = matches!(
            event.kind,
            sjk_client::ServerEventKind::Chat | sjk_client::ServerEventKind::TeamChat
        ) && chat.sent_by_muted(&text, event.sender);
        // Quiet mode keeps the line in the console but out of its alerts.
        let quieted = matches!(
            event.kind,
            sjk_client::ServerEventKind::Chat | sjk_client::ServerEventKind::TeamChat
        ) && chat.quiets(&text, event.sender, event.is_private);
        if !blocked
            && !quieted
            && let Some(console) = &mut console
        {
            console.notify_server_text(&event.kind, &event.text);
        }
        if event.kind == sjk_client::ServerEventKind::Print
            && event.text.contains(crate::hud::duel_pairs::ACCEPT)
        {
            duel_starts.push(event.text.clone());
        }
        if let Some(console) = &mut console {
            console.log_chat(&event.kind, &text);
        }
        // stderr, not stdout: the launcher captures only stderr, so anything
        // the server tells us -- a drop reason above all -- was being written
        // where nobody would ever read it.
        crate::log::progress(format_args!(
            "server {:?}: {}",
            event.kind,
            text.replace('\n', " "),
        ));
        match event.kind {
            // CG_Print_f: console scrollback and its notify lines, never the chat
            // box. Servers end a print with a newline; one row per line keeps it
            // from leaving an empty row behind or sharing one notify slot.
            sjk_client::ServerEventKind::Print => {
                if let Some(console) = &mut console {
                    for line in print_lines(&text) {
                        console.push_log(line);
                    }
                }
            }
            // CG_ChatBox_AddString echoes chat with the `*` prefix, so the
            // console keeps every chat line while the notify lines leave it to
            // the chat box.
            sjk_client::ServerEventKind::Chat | sjk_client::ServerEventKind::TeamChat => {
                let own_whisper = chat.own_whisper_body(&event.kind, &text, event.sender);
                let private = event.is_private;
                if !blocked && let Some(console) = &mut console {
                    console.push_chat_line(
                        sjk_client::chat_display_text(&text),
                        &text,
                        own_whisper.as_deref(),
                    );
                }
                chat.receive_classified(
                    event.kind,
                    text,
                    event.sender,
                    event.is_private,
                    Instant::now(),
                );
                // A tell, not the server's echo of the player's own: the taskbar
                // flashes while the window is away (`chat_sounds.rs`).
                if private
                    && own_whisper.is_none()
                    && chat.newest_is_open_private()
                    && let Some(console) = &mut console
                {
                    console.note_private_message();
                }
            }
            sjk_client::ServerEventKind::CenterPrint => {
                chat.receive_classified(
                    event.kind,
                    text,
                    event.sender,
                    event.is_private,
                    Instant::now(),
                );
            }
        }
    }
    for text in &duel_starts {
        duels.observe_print(text, session.game_state());
    }
}

/// Console rows of one server `print`: one per `\n`, without the final empty
/// row a terminating newline leaves, and without carriage returns.
fn print_lines(text: &str) -> impl Iterator<Item = &str> {
    text.strip_suffix('\n')
        .unwrap_or(text)
        .split('\n')
        .map(|line| line.trim_end_matches('\r'))
}

#[cfg(test)]
mod tests {
    use super::print_lines;

    #[test]
    fn print_lines_split_rows_and_drop_the_trailing_newline() {
        let rows: Vec<_> = print_lines("first\r\n\nthird\n").collect();
        assert_eq!(rows, ["first", "", "third"]);
        assert_eq!(print_lines("single").collect::<Vec<_>>(), ["single"]);
        let disconnect: Vec<_> = print_lines("Padawan^7 disconnected\n").collect();
        assert_eq!(disconnect, ["Padawan^7 disconnected"]);
    }
}
