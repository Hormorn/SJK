//! The composer row's own tools (Sol, 06/11/2026): an × on a hovered chat button takes
//! it off the row, a + puts back one taken off, and a cog opens quick settings (every
//! chat in view or only the chosen one, Tab skipping the chosen chat, opening on the
//! last chat). Each writes the same setting Settings > Chat shows
//! (`chat/tabs.rs`), so the two always agree; the frame saves it
//! ([`ChatOverlay::take_settings`]).
use super::tabs::{
    ADMIN_CVAR, FILTER_CVAR, JOF_CVAR, PM_CVAR, SITH_CVAR, SJK_CVAR, Shown, TEAM_CVAR,
};
use super::*;

/// The × after each removable button, by its place in the row's order (`tabs::ROW`).
pub(super) const CLOSE: u16 = 200;
/// The + that lists the chats taken off the row.
pub(super) const ADD: u16 = 207;
/// The cog that opens the quick settings.
pub(super) const SETTINGS: u16 = 208;
/// The open menu's first row; the others follow.
pub(super) const ITEM: u16 = 210;
/// The most rows a menu holds.
pub(super) const ITEM_MAX: usize = 8;
/// Every menu row's token.
pub(super) const ITEMS: std::ops::Range<u16> = ITEM..ITEM + ITEM_MAX as u16;
/// Every × token.
pub(super) const CLOSES: std::ops::Range<u16> = CLOSE..CLOSE + 7;

/// Which of the row's menus is open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RowMenu {
    Add,
    Settings,
}

/// One row of an open menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Item {
    /// Put this chat's button back in the row.
    Add(Channel),
    /// Only the chosen chat in view while typing (`true`), or every chat (`false`).
    Filter(bool),
    /// Tab skips this chat, its button kept.
    TabSkips(Channel),
    /// The chat key opens on the last chat sent on (`cg_chatRememberChannel`).
    Remember,
}

/// The setting that keeps `channel`'s button, for the switchable ones.
fn cvar(channel: Channel) -> Option<&'static str> {
    Some(match channel {
        Channel::Team => TEAM_CVAR,
        Channel::Sjk => SJK_CVAR,
        Channel::Admin => ADMIN_CVAR,
        Channel::Jof => JOF_CVAR,
        Channel::Sith => SITH_CVAR,
        Channel::Whisper => PM_CVAR,
        Channel::Global => return None,
    })
}

impl ChatOverlay {
    /// Whether `channel`'s button may be taken off the row: all but All.
    pub(super) fn closable(channel: Channel) -> bool {
        cvar(channel).is_some()
    }

    /// The chats the + can put back: usable here but taken off the row.
    pub(super) fn hidden(&self) -> impl Iterator<Item = Channel> + '_ {
        super::tabs::ROW
            .into_iter()
            .filter(|channel| Self::closable(*channel) && self.available(*channel))
            .filter(|channel| !self.offers(*channel))
    }

    /// The open menu's rows, in order.
    pub(super) fn menu_items(&self) -> Vec<Item> {
        let Some(menu) = self.input.as_ref().and_then(|input| input.menu) else {
            return Vec::new();
        };
        let mut items = match menu {
            RowMenu::Add => self.hidden().map(Item::Add).collect(),
            RowMenu::Settings => {
                let mut items = vec![Item::Filter(true), Item::Filter(false)];
                let channel = self.input.as_ref().map(|input| input.channel);
                if let Some(channel) = channel
                    && channel != Channel::Whisper
                    && Self::closable(channel)
                {
                    items.push(Item::TabSkips(channel));
                }
                items.push(Item::Remember);
                items
            }
        };
        items.truncate(ITEM_MAX);
        items
    }

    /// Whether `item` is the current choice (a menu row drawn as picked).
    pub(super) fn item_on(&self, item: Item) -> bool {
        match item {
            Item::Add(_) => false,
            Item::Filter(only) => self.options.tabs.filter == only,
            Item::TabSkips(channel) => self.options.tabs.of(channel) == Shown::NotInTab,
            Item::Remember => self.options.remember,
        }
    }

    /// Open `menu`, or close it when it is the open one.
    pub(super) fn toggle_menu(&mut self, menu: RowMenu) {
        if let Some(input) = self.input.as_mut() {
            input.pm_list = false;
            input.menu = (input.menu != Some(menu)).then_some(menu);
        }
    }

    /// The × on `channel`'s button: off the row, and a composer on it moves to All.
    pub(super) fn close_tab(&mut self, channel: Channel) {
        let Some(name) = cvar(channel) else {
            return;
        };
        self.set_shown(channel, Shown::Off);
        self.settings.push((name, "0"));
        if self
            .input
            .as_ref()
            .is_some_and(|input| input.channel == channel)
        {
            self.choose(Channel::Global);
        }
    }

    /// A row of the open menu.
    pub(super) fn pick_item(&mut self, index: usize) {
        let Some(item) = self.menu_items().get(index).copied() else {
            return;
        };
        match item {
            Item::Add(channel) => {
                if let Some(name) = cvar(channel) {
                    self.set_shown(channel, Shown::On);
                    self.settings.push((name, "1"));
                }
                // Nothing more to put back: the menu goes.
                if self.hidden().next().is_none()
                    && let Some(input) = self.input.as_mut()
                {
                    input.menu = None;
                }
            }
            Item::Filter(only) => {
                self.options.tabs.filter = only;
                self.settings
                    .push((FILTER_CVAR, if only { "1" } else { "0" }));
                self.scroll = 0;
                self.unread = 0;
                for line in &mut self.lines {
                    line.y = None;
                }
            }
            Item::TabSkips(channel) => {
                if let Some(name) = cvar(channel) {
                    let skip = self.options.tabs.of(channel) != Shown::NotInTab;
                    self.set_shown(channel, if skip { Shown::NotInTab } else { Shown::On });
                    self.settings.push((name, if skip { "2" } else { "1" }));
                }
            }
            Item::Remember => {
                self.options.remember = !self.options.remember;
                let value = if self.options.remember { "1" } else { "0" };
                self.settings.push((super::reply::REMEMBER_CVAR, value));
            }
        }
    }

    /// Keep `channel`'s button as `shown` until the frame reads the settings again.
    fn set_shown(&mut self, channel: Channel, shown: Shown) {
        let tabs = &mut self.options.tabs;
        match channel {
            Channel::Team => tabs.team = shown,
            Channel::Sjk => tabs.sjk = shown,
            Channel::Admin => tabs.admin = shown,
            Channel::Jof => tabs.jof = shown,
            Channel::Sith => tabs.sith = shown,
            Channel::Whisper => tabs.pm = shown.shown(),
            Channel::Global => {}
        }
    }

    /// The settings the row's tools changed since the last call, for the frame to save.
    pub(crate) fn take_settings(&mut self) -> Vec<(&'static str, &'static str)> {
        std::mem::take(&mut self.settings)
    }
}

#[cfg(test)]
impl ChatOverlay {
    /// For a world shot, after a frame drew the row: `0` points at Team (its × shows),
    /// `1` opens the + menu, `2` the cog's.
    pub(crate) fn row_tools_for_shot(&mut self, step: u8) {
        let token = match step {
            0 => interaction::TEAM,
            1 => ADD,
            _ => SETTINGS,
        };
        if let Some(rect) = self.ui.rect_for(token) {
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            self.pointer_at = Some(at);
            self.pointer(sjk_ui::InputEvent::PointerMove(at));
        }
        match step {
            1 => self.toggle_menu(RowMenu::Add),
            2 => {
                self.choose(Channel::Team);
                self.toggle_menu(RowMenu::Settings);
            }
            _ => {}
        }
    }
}

impl crate::GpuState {
    /// Save what the chat row's tools changed (`chat/row_tools.rs`).
    pub(crate) fn save_chat_settings(&mut self) {
        let settings = self.chat.take_settings();
        if !settings.is_empty()
            && let Some(console) = self.console.as_mut()
        {
            console.set_cvars(settings);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::{InputEvent, PointerButton, Vec2};

    fn frame(chat: &mut ChatOverlay) {
        chat.build(true, &crate::text::test_font(), [1920.0, 1080.0], 1_000);
    }

    fn centre(chat: &ChatOverlay, token: u16) -> Vec2 {
        let rect = chat.ui.rect_for(token).expect("drawn");
        Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }

    fn hover(chat: &mut ChatOverlay, token: u16) {
        frame(chat);
        let at = centre(chat, token);
        chat.pointer(InputEvent::PointerMove(at));
        frame(chat);
    }

    fn click(chat: &mut ChatOverlay, token: u16) {
        frame(chat);
        let at = centre(chat, token);
        chat.pointer(InputEvent::PointerMove(at));
        frame(chat);
        let button = PointerButton::Primary;
        chat.pointer(InputEvent::PointerPress {
            position: at,
            button,
        });
        chat.pointer(InputEvent::PointerRelease {
            position: at,
            button,
        });
    }

    fn open() -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        chat.admin_server = true;
        chat.open(false);
        chat
    }

    fn close_token(channel: Channel) -> u16 {
        CLOSE
            + super::super::tabs::ROW
                .iter()
                .position(|row| *row == channel)
                .unwrap() as u16
    }

    #[test]
    fn a_hovered_button_shows_its_x_which_takes_it_off_the_row() {
        let mut chat = open();
        frame(&mut chat);
        // No × until the button is hovered, and never one on All.
        assert!(chat.ui.rect_for(close_token(Channel::Team)).is_none());
        hover(&mut chat, interaction::TEAM);
        assert!(chat.ui.rect_for(close_token(Channel::Team)).is_some());
        hover(&mut chat, interaction::GLOBAL);
        assert!(chat.ui.rect_for(close_token(Channel::Global)).is_none());
        // Closing the chosen chat moves the composer to All.
        chat.choose(Channel::Team);
        hover(&mut chat, interaction::TEAM);
        click(&mut chat, close_token(Channel::Team));
        assert!(!chat.offers(Channel::Team));
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Global);
        assert_eq!(chat.take_settings(), [(TEAM_CVAR, "0")]);
        assert!(chat.take_settings().is_empty());
    }

    #[test]
    fn the_plus_lists_the_hidden_chats_and_puts_one_back() {
        let mut chat = open();
        // PM is off the row by default: the + offers it.
        assert_eq!(chat.hidden().collect::<Vec<_>>(), [Channel::Whisper]);
        chat.close_tab(Channel::Sjk);
        chat.close_tab(Channel::Admin);
        chat.take_settings();
        click(&mut chat, ADD);
        assert_eq!(
            chat.menu_items(),
            [
                Item::Add(Channel::Sjk),
                Item::Add(Channel::Admin),
                Item::Add(Channel::Whisper)
            ]
        );
        frame(&mut chat);
        assert!(chat.ui.text_runs().any(|text| text == "SJK"));
        click(&mut chat, ITEM);
        assert!(chat.offers(Channel::Sjk));
        assert_eq!(chat.take_settings(), [(SJK_CVAR, "1")]);
        click(&mut chat, ITEM);
        // The last one back closes the menu, and the + goes.
        click(&mut chat, ITEM);
        assert!(chat.offers(Channel::Whisper));
        assert_eq!(chat.take_settings(), [(ADMIN_CVAR, "1"), (PM_CVAR, "1")]);
        assert_eq!(chat.input.as_ref().unwrap().menu, None);
        frame(&mut chat);
        assert!(chat.ui.rect_for(ADD).is_none());
        // Admin only where the server has it: not offered elsewhere.
        chat.close_tab(Channel::Admin);
        chat.set_admin_server(false);
        assert_eq!(chat.hidden().count(), 0);
    }

    #[test]
    fn the_cog_switches_the_filter_tab_skipping_and_remembering() {
        let mut chat = open();
        chat.choose(Channel::Team);
        click(&mut chat, SETTINGS);
        assert_eq!(
            chat.menu_items(),
            [
                Item::Filter(true),
                Item::Filter(false),
                Item::TabSkips(Channel::Team),
                Item::Remember
            ]
        );
        assert!(chat.item_on(Item::Filter(true)));
        click(&mut chat, ITEM + 1);
        assert!(!chat.options.tabs.filter && chat.item_on(Item::Filter(false)));
        click(&mut chat, ITEM + 2);
        assert_eq!(chat.options.tabs.of(Channel::Team), Shown::NotInTab);
        // Tab now skips Team, its button stays.
        assert!(chat.offers(Channel::Team));
        assert_eq!(chat.next_channel(Channel::Global), Some(Channel::Sjk));
        click(&mut chat, ITEM + 3);
        assert!(chat.options.remember);
        assert_eq!(
            chat.take_settings(),
            [
                (FILTER_CVAR, "0"),
                (TEAM_CVAR, "2"),
                (super::super::reply::REMEMBER_CVAR, "1")
            ]
        );
        // The menu stays open for the next change; the cog again closes it.
        assert_eq!(chat.input.as_ref().unwrap().menu, Some(RowMenu::Settings));
        click(&mut chat, SETTINGS);
        assert_eq!(chat.input.as_ref().unwrap().menu, None);
        // On All, no Tab row: All is always in Tab.
        chat.choose(Channel::Global);
        chat.toggle_menu(RowMenu::Settings);
        assert_eq!(chat.menu_items().len(), 3);
    }

    #[test]
    fn a_press_elsewhere_or_escape_closes_the_menu() {
        let mut chat = open();
        click(&mut chat, SETTINGS);
        click(&mut chat, interaction::TEAM);
        assert_eq!(chat.input.as_ref().unwrap().menu, None);
        assert_eq!(chat.input.as_ref().unwrap().channel, Channel::Team);
        chat.toggle_menu(RowMenu::Settings);
        chat.edit_key(
            winit::keyboard::KeyCode::Escape,
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            None,
        );
        assert!(chat.is_typing() && chat.input.as_ref().unwrap().menu.is_none());
    }

    #[test]
    fn the_frame_saves_what_the_tools_changed() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut chat = open();
        chat.close_tab(Channel::Team);
        console.set_cvars(chat.take_settings());
        assert_eq!(console.integer_cvar(TEAM_CVAR), Some(0));
        chat.configure(Some(&console));
        assert!(!chat.offers(Channel::Team));
    }
}
