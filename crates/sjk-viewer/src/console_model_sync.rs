//! Console side of the Model sync page (see `model_sync_page.rs`): the `modelsync`
//! toggle, reading the page's rows from [`crate::model_sync`] when they changed,
//! queueing the commands its buttons ask for, and routing keys and pointer events to it
//! while it is open. It is drawn in place of the console like the content cards
//! (`console_jof_content.rs`), in the SJK UI's look whatever the menu style.

use super::model_sync_page::PageAction;
use super::*;
use sjk_ui::InputEvent;

impl ViewerConsole {
    /// `modelsync`: show the page, opening the console if needed, or close it (and the
    /// console, if the page opened it).
    pub(crate) fn toggle_model_sync(&mut self, game_data: &std::path::Path) {
        if self.open && self.model_sync_page.is_open() {
            self.close_model_sync();
            return;
        }
        let owns_console = self.model_sync_page.take_handed().unwrap_or(!self.open);
        self.show_model_sync(owns_console, game_data);
    }

    /// Open the page in the place of another page that closed (the Profile page's Share
    /// my model): through `modelsync`, whose handler knows the game data, keeping
    /// whether that page had opened the console.
    pub(crate) fn queue_model_sync_page(&mut self, owns_console: bool) {
        self.model_sync_page.hand_console(owns_console);
        if let Err(why) = self.queue_command(crate::model_sync::PAGE) {
            self.model_sync_page.take_handed();
            self.push_log(format!("^3Model sync: {why}"));
        }
    }

    /// Show the page; closing it closes the console too when `owns_console`. It asks
    /// the Sith cloud for its list at once.
    pub(crate) fn show_model_sync(&mut self, owns_console: bool, game_data: &std::path::Path) {
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.debug_panel.close();
        self.spawn_panel.close();
        self.model_viewer.close();
        self.changelog.close();
        self.update_panel.close();
        self.config_import.close();
        self.credits.close();
        self.identity_panel.close();
        self.profile_panel.close();
        self.staff_panel.close();
        self.collection_panel.close();
        self.holocrons_panel.close();
        self.sjk_chat_panel.close();
        self.asset_browser.open = false;
        self.jof_content.open = false;
        self.sith_cloud.open = false;
        self.dead_key.settle();
        self.model_sync_page.open(owns_console, game_data);
        crate::model_sync::refresh_index(self, game_data);
        self.model_sync_live();
    }

    /// Whether the page shows.
    pub(crate) fn model_sync_open(&self) -> bool {
        self.open && self.model_sync_page.is_open()
    }

    fn close_model_sync(&mut self) {
        if self.model_sync_page.close() {
            self.set_open(false);
        }
    }

    /// Give the page what changed: the live states every time, its rows only when
    /// model sync said something changed (or a few seconds went by).
    fn model_sync_live(&mut self) {
        let can_share = crate::model_sync::can_share(self.bool_cvar("cl_identity") == Some(true));
        let auto_on = self.bool_cvar(crate::model_sync::AUTO) == Some(true);
        self.model_sync_page
            .set_live(can_share, crate::model_sync::busy(), auto_on);
        let changes = crate::model_sync::changes();
        let now = Instant::now();
        if !self.model_sync_page.wants_rows(changes, now) {
            return;
        }
        let worn = crate::player_identity::own_model_folder();
        let rows = crate::model_sync::page_rows(self.model_sync_page.game_data(), worn.as_deref());
        self.model_sync_page.set_rows(
            rows,
            crate::model_sync::has_index(),
            crate::model_sync::recent_lines(),
            changes,
            now,
        );
    }

    fn model_sync_action(&mut self, action: PageAction) {
        match action {
            PageAction::None => {}
            PageAction::Close => self.close_model_sync(),
            PageAction::Refresh => {
                let game_data = self.model_sync_page.game_data().to_owned();
                crate::model_sync::refresh_index(self, &game_data);
            }
            PageAction::Run(action, model) => {
                // The command prints how it went, which the page shows too.
                let line = format!("{} {model}", action.command());
                if let Err(why) = self.queue_command(&line) {
                    self.push_log(format!("^3Model sync: {why}"));
                }
            }
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn model_sync_key(&mut self, event: &KeyEvent) -> bool {
        if !self.model_sync_page.is_open() {
            return false;
        }
        self.model_sync_live();
        let action = self
            .model_sync_page
            .handle_key(event, self.shift, self.control);
        self.model_sync_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn model_sync_pointer(&mut self, event: InputEvent) -> bool {
        if !self.model_sync_page.is_open() {
            return false;
        }
        let action = self.model_sync_page.handle_pointer(event);
        self.model_sync_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_model_sync(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        self.model_sync_live();
        self.model_sync_page.append_sjk(target, viewport);
    }

    /// The page's draw list while it is shown.
    pub(super) fn model_sync_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        self.model_sync_open()
            .then(|| self.model_sync_page.draw_list())
    }

    /// Show `rows` in place of the live ones, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_model_sync(
        &mut self,
        rows: Vec<crate::model_sync::PageRow>,
        can_share: bool,
        lines: Vec<String>,
    ) {
        self.model_sync_page.preview(rows, can_share, false, lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_opens_over_the_console_and_closes_the_console_it_took() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        // The list the page asks for goes nowhere: a local port that answers nothing.
        console.set_cvar(crate::sith_cloud::URL, "http://127.0.0.1:9/repository");
        console.toggle_model_sync(directory.path());
        assert!(console.is_open());
        assert!(console.model_sync_open());
        assert!(console.sjk_page_open(), "drawn in the SJK UI's look");
        assert!(console.covers_frame());
        assert!(console.model_sync_draw_list().is_some());
        console.toggle_model_sync(directory.path());
        assert!(!console.is_open());
        assert!(!console.model_sync_open());
        // Opened over an open console, it leaves the console open when it closes.
        console.set_open(true);
        console.toggle_model_sync(directory.path());
        console.toggle_model_sync(directory.path());
        assert!(console.is_open());
        assert!(!console.model_sync_open());
    }
}
