//! Console side of the `spawn_panel` list (see `spawn_panel.rs`): the toggle, running the
//! line it picks, and routing keys and pointer events to it while it is open. The shared
//! console files only call into here, to keep merges with upstream small.

use super::spawn_panel::{self, PanelAction, catalogue::Catalogue};
use super::*;
use sjk_ui::InputEvent;

impl ViewerConsole {
    /// `spawn_panel`: show the list in front of the console with the names `vfs` defines,
    /// opening the console if needed, or close it (and the console, if the panel opened
    /// it).
    pub(crate) fn toggle_spawn_panel(&mut self, vfs: Option<&sjk_vfs::VirtualFileSystem>) {
        if self.open && self.spawn_panel.is_open() {
            self.close_spawn_panel();
            return;
        }
        let owns_console = !self.open;
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.changelog.close();
        self.config_import.close();
        self.credits.close();
        self.debug_panel.close();
        self.dead_key.settle();
        let catalogue = vfs.map(Catalogue::scan).unwrap_or_default();
        self.spawn_panel.open(catalogue, owns_console);
    }

    fn close_spawn_panel(&mut self) {
        crate::text_entry::deselect();
        if self.spawn_panel.close() {
            self.set_open(false);
        }
    }

    fn spawn_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_spawn_panel(),
            PanelAction::Run(line) => {
                if let Err(error) = self.queue_command(&line) {
                    self.push_log(format!("^1spawn_panel: {error}"));
                }
            }
        }
    }

    /// Whether the key event's binds run `spawn_panel`.
    fn bound_to_spawn_panel(&self, event: &KeyEvent) -> bool {
        let winit::keyboard::PhysicalKey::Code(key) = event.physical_key else {
            return false;
        };
        let Some(key_name) = crate::input::keys::name(crate::input::keys::Source::Key(key)) else {
            return false;
        };
        self.shell
            .binds
            .commands_for_event(key_name, true)
            .is_ok_and(|commands| {
                commands
                    .iter()
                    .any(|command| command.eq_ignore_ascii_case(spawn_panel::COMMAND))
            })
    }

    /// Give a pressed key to the open panel; false when the panel is closed. A key
    /// bound to `spawn_panel` closes it, so the same key toggles it, unless it types
    /// into the filter.
    pub(super) fn spawn_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.spawn_panel.is_open() {
            return false;
        }
        let types = event
            .text
            .as_deref()
            .is_some_and(|text| !text.is_empty() && text.chars().all(|c| !c.is_control()));
        if !event.repeat
            && event.state == winit::event::ElementState::Pressed
            && !types
            && self.bound_to_spawn_panel(event)
        {
            self.close_spawn_panel();
            return true;
        }
        let action = self.spawn_panel.handle_key(event, self.shift, self.control);
        self.spawn_panel_action(action);
        true
    }

    /// Give a pointer event to the open panel; false when the panel is closed.
    pub(super) fn spawn_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.spawn_panel.is_open() {
            return false;
        }
        let action = self.spawn_panel.handle_pointer(event);
        self.spawn_panel_action(action);
        true
    }

    /// Draw the panel in place of the console, in the SJK UI's look, its text to
    /// `target` (`console_sjk_pages.rs` routes it).
    pub(super) fn append_spawn_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        self.spawn_panel.append_sjk(target, viewport);
    }

    /// Open the list on a few names, for a world shot.
    #[cfg(test)]
    pub(crate) fn spawn_panel_for_shot(&mut self) {
        if !self.open {
            self.set_open(true);
        }
        self.spawn_panel.open_for_shot();
    }

    /// The panel's draw list while it is shown.
    pub(super) fn spawn_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.spawn_panel.is_open()).then(|| self.spawn_panel.draw_list())
    }
}
