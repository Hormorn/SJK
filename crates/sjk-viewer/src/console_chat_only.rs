//! The console's chat-only view (Hormorn's idea, #ideas 05/11/2026: "a mode that only
//! shows chat logs, instead of all game logs, to help us keep track of conversations").
//!
//! With `con_chatOnly` on ([`super::console_options::CHAT_ONLY_CVAR`], off by default)
//! the open console's scrollback lists only chat lines: the ones the game's chat, team
//! chat, whispers and SJK chat put there through [`ViewerConsole::push_chat_line`].
//! Every line is still kept, so switching it off shows them all again; a selection
//! copies the chat lines it spans alone. The notify lines over the game, the console
//! socket and `cl_logChat` are unchanged. The console says "Chat only" in its header
//! (or its corner, classic) while it is on.
use super::*;

impl ViewerConsole {
    /// Remember scrollback line `number` as chat, forgetting those the scrollback no
    /// longer keeps.
    pub(super) fn note_chat_line(&mut self, number: u64) {
        let first = self.first_kept_line();
        while self.chat_lines.front().is_some_and(|line| *line < first) {
            self.chat_lines.pop_front();
        }
        if self.chat_lines.back().is_none_or(|line| *line < number) {
            self.chat_lines.push_back(number);
        }
    }

    /// Whether scrollback line `number` is chat.
    pub(super) fn is_chat_line(&self, number: u64) -> bool {
        self.chat_lines.binary_search(&number).is_ok()
    }

    /// The number of the oldest line the scrollback keeps.
    fn first_kept_line(&self) -> u64 {
        self.shell.lines_written() - self.shell.lines().count() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn chat_lines_are_told_apart_from_the_rest() {
        let (_directory, mut console) = console();
        console.push_log("map loaded");
        let chat = console.shell.lines_written();
        console.push_chat_line("Sol^7: hi".into(), "Sol^7\u{19}: hi", None);
        console.push_log("^3cvar changed");
        let after = console.shell.lines_written();
        console.push_chat_line("Fox^7: hey".into(), "Fox^7\u{19}: hey", None);
        assert!(console.is_chat_line(chat));
        assert!(console.is_chat_line(after));
        assert!(!console.is_chat_line(chat - 1));
        assert!(!console.is_chat_line(chat + 1));
        assert!(
            !console
                .bool_cvar(super::super::console_options::CHAT_ONLY_CVAR)
                .unwrap()
        );
    }
}
