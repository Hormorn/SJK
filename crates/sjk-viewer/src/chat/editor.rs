//! Chat drafts use the console's UTF-8 caret, selection and word editing rules.

use super::Channel;
use crate::console::line_edit::{LineEdit, Motion};
use crate::input::dead_key::{DeadKey, TypingField, keep_caret};
use sjk_client::{CHAT_DRAFT_CHARS, ChatTarget};
use std::ops::Range;
use winit::keyboard::{Key, KeyCode};

pub(super) struct Editor {
    pub(super) text: String,
    pub(super) edit: LineEdit,
    pub(super) channel: Channel,
    pub(super) recipient: Option<ChatTarget>,
    /// The list of the players who whispered you, opened from the recipient's name
    /// (`chat/view/recipient.rs`).
    pub(super) pm_list: bool,
    /// The row's + or cog menu, when one is open (`chat/row_tools.rs`).
    pub(super) menu: Option<super::row_tools::RowMenu>,
    pub(super) layout: super::editing::DraftLayout,
    /// Dead key shown at the caret until its composition arrives.
    pub(super) dead: DeadKey,
}

impl Editor {
    pub(super) fn new(channel: Channel) -> Self {
        Self {
            text: String::with_capacity(CHAT_DRAFT_CHARS),
            edit: LineEdit::default(),
            channel,
            recipient: None,
            pm_list: false,
            menu: None,
            layout: super::editing::DraftLayout::default(),
            dead: DeadKey::default(),
        }
    }

    /// Type a pressed character key, showing a dead key until it composes.
    pub(super) fn type_key(&mut self, logical: &Key, text: Option<&str>) {
        let mut dead = self.dead;
        dead.type_key(self, logical, text);
        self.dead = dead;
    }

    /// Insert typed or pasted text into the bounded multi-message draft.
    pub(super) fn insert(&mut self, value: &str) {
        self.edit.insert_fitting(&mut self.text, value, |text| {
            text.chars().count() <= CHAT_DRAFT_CHARS
                && sjk_client::chat_commands(sjk_client::ChatDestination::Global, text).is_ok()
        });
    }

    pub(super) fn key(&mut self, key: KeyCode, control: bool, shift: bool) -> bool {
        let left = if control {
            Motion::WordLeft
        } else {
            Motion::Left
        };
        let right = if control {
            Motion::WordRight
        } else {
            Motion::Right
        };
        match key {
            KeyCode::ArrowLeft => self.edit.motion(&self.text, left, shift),
            KeyCode::ArrowRight => self.edit.motion(&self.text, right, shift),
            KeyCode::Home => self.edit.motion(&self.text, Motion::Home, shift),
            KeyCode::End => self.edit.motion(&self.text, Motion::End, shift),
            KeyCode::Backspace => self.edit.delete(&mut self.text, left),
            KeyCode::Delete => self.edit.delete(&mut self.text, right),
            _ => return false,
        }
        true
    }
}

impl TypingField for Editor {
    fn line(&self) -> &str {
        &self.text
    }

    fn caret(&self) -> usize {
        self.edit.cursor(&self.text)
    }

    fn insert(&mut self, text: &str) {
        Editor::insert(self, text);
    }

    fn remove(&mut self, range: Range<usize>) {
        let caret = keep_caret(self.edit.cursor(&self.text), &range);
        self.text.replace_range(range, "");
        self.edit.place(&self.text, caret, false);
    }
}

#[cfg(test)]
mod length_tests {
    use super::*;

    #[test]
    fn the_field_keeps_at_most_four_messages() {
        let mut editor = Editor::new(Channel::Global);
        editor.insert(&"%".repeat(CHAT_DRAFT_CHARS + 100));
        let commands = sjk_client::chat_commands(sjk_client::ChatDestination::Global, &editor.text)
            .expect("the editor refuses a fifth message");
        assert_eq!(commands.len(), sjk_client::MAX_CHAT_PIECES);
        let full = editor.text.clone();
        editor.insert("%");
        assert_eq!(editor.text, full);
    }
}
