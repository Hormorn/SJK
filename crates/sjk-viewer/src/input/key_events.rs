//! Native keyboard routing into UI and the console command buffer.
use super::alt_code;
use crate::*;

impl GpuState {
    pub(crate) fn keyboard(&mut self, event: KeyEvent) {
        // Whoever takes the release, retire it from the binding stream before
        // routing. A chord such as `SHIFT+U reply` can open the chat composer on
        // its press; the composer then owns both releases, but they must still
        // clear the chord or either key would trigger it alone next time.
        if event.state == ElementState::Released
            && let PhysicalKey::Code(code) = event.physical_key
            && let Some(console) = &mut self.console
        {
            console.queue_bound_key(code, crate::input::keys::key_name(&event), false);
        }
        let typing = self.text_has_keyboard();
        match self.alt_code.key(&event, typing) {
            alt_code::Step::Pass => self.route_keyboard(event),
            alt_code::Step::Withhold => {}
            alt_code::Step::Type(character) => {
                let typed = alt_code::typed_event(&event, character);
                self.route_keyboard(event);
                self.route_keyboard(typed);
            }
        }
    }

    /// A text field has the keyboard: the console, the chat composer or a menu,
    /// rather than the gameplay bindings.
    pub(crate) fn text_has_keyboard(&self) -> bool {
        self.console
            .as_ref()
            .is_some_and(console::ViewerConsole::is_open)
            || self.chat.is_typing()
            || self.text_dialog.is_open()
            || self
                .client_menu
                .as_ref()
                .is_some_and(|menu| menu.is_visible())
            || self.game_menu
    }

    fn route_keyboard(&mut self, event: KeyEvent) {
        // Escape closes an open quick wheel without choosing, and goes no further.
        if self.quick_wheel.is_open()
            && event.state == winit::event::ElementState::Pressed
            && event.physical_key == PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
        {
            self.quick_wheel.cancel();
            return;
        }
        // The console key precedes the message catcher as in stock (`cl_keys.cpp:1318`);
        // `console_key_leaves_chat` says which keys the composer keeps.
        let console_open = self
            .console
            .as_ref()
            .is_some_and(console::ViewerConsole::is_open);
        let typed = matches!(event.physical_key, PhysicalKey::Code(_));
        // The SJK team's window takes every key while it shows, over the console too (an
        // announcement read from the chat page or a command).
        if self.official.window.is_open() {
            self.official_key(&event);
            return;
        }
        // And the News panel, over the console too when it was asked for.
        if self.news.is_open() {
            self.news_key(&event);
            return;
        }
        // `cl_consoleMove`: a movement key held from before the console opened keeps
        // moving the player; its repeats are not typed (`console_move`).
        if console_open
            && event.repeat
            && let PhysicalKey::Code(code) = event.physical_key
            && self
                .console
                .as_ref()
                .is_some_and(|console| console.holds_bound_key(code))
            && self.console_keeps_moving()
        {
            return;
        }
        if !console_open && self.text_dialog.is_open() {
            let action = self.text_dialog.handle_key(&event);
            self.apply_dialog_action(action);
            return;
        }
        // The alpha notice takes every key until it is accepted.
        if !console_open && self.alpha_card.is_open() {
            let choice = self.alpha_card.handle_key(&event);
            self.alpha_card_choice(choice);
            return;
        }
        // A new medal on show takes every key until it is closed.
        if !console_open && self.medal_popup.is_open() {
            self.medal_popup.handle_key(&event);
            return;
        }
        // So does a holocron drop.
        if !console_open && self.holocron_popup.is_open() {
            self.holocron_popup.handle_key(&event);
            return;
        }
        // So does the graphics reload card.
        if !console_open && self.graphics_reload.card.is_open() {
            let choice = self.graphics_reload.card.handle_key(&event);
            self.reload_card_choice(choice);
            return;
        }
        // And the small-card notice.
        if !console_open && self.small_card_notice.is_open() {
            let choice = self.small_card_notice.handle_key(&event);
            self.small_card_notice_choice(choice);
            return;
        }
        // And the emote help card.
        if !console_open && self.emote_help.card.is_open() {
            let choice = self.emote_help.card.handle_key(&event);
            self.emote_help_choice(choice);
            return;
        }
        // And the emote picker, which takes the typing for its search.
        if !console_open && self.emote_picker.card.is_open() {
            self.emote_picker_key(&event);
            return;
        }
        // And the encrypted whisper card.
        if !console_open && self.e2e_whispers.card.is_open() {
            let choice = self.e2e_whispers.card.handle_key(&event);
            self.e2e_whisper_card_choice(choice);
            return;
        }
        // And a mod's notice.
        if !console_open && self.mod_notice.card.is_open() {
            let choice = self.mod_notice.card.handle_key(&event);
            self.mod_notice_choice(choice);
            return;
        }
        // And the trade window, but for the chat: its keys open the composer over the
        // window, which then has the typing (Enter sends, Escape closes it) so the
        // players can talk while they trade (#215).
        if !console_open && self.trades.window.is_open() && !self.chat.is_typing() {
            if self.trade_window_chat_key(&event) {
                return;
            }
            let choice = self.trades.window.handle_key(&event);
            self.trade_window_choice(choice);
            return;
        }
        // The HUD edit mode takes every key but the console's.
        if !console_open && self.hud_edit.is_open() && !self.text_dialog.is_open() {
            if self
                .console
                .as_ref()
                .is_some_and(|console| console.leaves_chat_for_console(&event))
            {
                self.route_console_key(&event);
                self.sync_cursor_policy();
            } else {
                self.hud_edit_key(&event);
            }
            return;
        }
        // And the low-FPS card.
        if !console_open && self.fps_card.is_open() {
            let choice = self.fps_card.handle_key(&event);
            self.fps_card_choice(choice);
            return;
        }
        // And the log request card.
        if !console_open && self.log_request_card.is_open() {
            let choice = self.log_request_card.handle_key(&event);
            self.log_request_card_choice(choice);
            return;
        }
        // Ctrl+Tab changes the Profile screen's tab, whichever screen shows it.
        if self.profile_hub_key(&event) {
            return;
        }
        if !console_open && typed && self.chat.is_typing() {
            // The console key opens the console over the composer, which keeps its
            // draft for when the console closes, as stock's message field does.
            if self
                .console
                .as_ref()
                .is_some_and(|console| console.leaves_chat_for_console(&event))
            {
                self.route_console_key(&event);
                self.sync_cursor_policy();
                return;
            }
            self.chat_key(&event);
            return;
        }
        // Typing in the SJK UI game menu's docked chat: the console key and the
        // bindings type there too.
        if !console_open && typed && self.sjk_chat_typing(&event) {
            return;
        }
        if self.route_console_key(&event) {
            return;
        }
        if self
            .client_menu
            .as_ref()
            .is_some_and(|menu| menu.is_visible())
        {
            let action = match (&mut self.client_menu, &mut self.console) {
                (Some(menu), Some(console)) => menu.handle_key(&event, console),
                _ => menu::MenuAction::None,
            };
            self.apply_client_menu_action(action);
            return;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return;
        };
        if self.chat.is_typing() {
            self.chat_key(&event);
            return;
        }
        if self.shot_key(&event) || self.map_edit_key(&event) || self.ghost_edit_key(&event) {
            return;
        }
        if self.game_menu {
            if event.state != ElementState::Pressed || event.repeat {
                return;
            }
            // The SJK UI's main page: its list, row of icons and match card.
            if self.sjk_main_key(key) {
                return;
            }
            match key {
                // The SJK UI's arc passes over rows that cannot be taken.
                KeyCode::ArrowUp
                | KeyCode::KeyW
                | KeyCode::ArrowDown
                | KeyCode::KeyS
                | KeyCode::Tab
                    if self.in_game_menu.is_sjk() =>
                {
                    let forward = !matches!(key, KeyCode::ArrowUp | KeyCode::KeyW);
                    self.game_menu_row = self.in_game_menu.sjk_step(
                        self.game_menu_page,
                        self.game_menu_row,
                        self.game_menu_row_count(),
                        forward,
                    );
                }
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    let count = self.game_menu_row_count();
                    self.game_menu_row = self
                        .in_game_menu
                        .navigate(sjk_ui::AbstractAction::Previous)
                        .filter(|row| *row < count)
                        .unwrap_or_else(|| self.game_menu_row.checked_sub(1).unwrap_or(count - 1));
                }
                KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => {
                    let count = self.game_menu_row_count();
                    self.game_menu_row = self
                        .in_game_menu
                        .navigate(sjk_ui::AbstractAction::Next)
                        .filter(|row| *row < count)
                        .unwrap_or((self.game_menu_row + 1) % count);
                }
                KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::KeyA | KeyCode::KeyD
                    if self.in_game_menu.is_classic() =>
                {
                    self.classic_menu_sideways(matches!(key, KeyCode::ArrowRight | KeyCode::KeyD));
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    if self.in_game_menu.activation_allowed(self.game_menu_row) {
                        self.activate_game_menu_row();
                    }
                }
                KeyCode::Escape => self.back_or_close_game_menu(),
                _ => {}
            }
            return;
        }
        // Escape unpins a pinned player card, or drops a waiting `worldnote` selection,
        // before it opens the game menu.
        if key == KeyCode::Escape
            && event.state == ElementState::Pressed
            && (self.hud.card.cancel() || self.world_notes.cancel_selection())
        {
            return;
        }
        if key == KeyCode::Escape
            && event.state == ElementState::Pressed
            && !self
                .console
                .as_ref()
                .is_some_and(|c| c.has_key_binding("ESCAPE"))
        {
            // While the ghost editor is on, Escape brings its column back.
            if self.ghost_editor_looking() {
                self.open_ghost_panel();
                return;
            }
            self.release_pointer();
            if self.live_session.is_some() || self.resident.exploring() {
                self.game_menu = true;
                self.game_menu_page = GameMenuPage::Main;
                self.game_menu_row = 0;
                self.in_game_menu.focus = crate::ingame_menu::sjk_focus::Focus::List;
                self.in_game_menu.stop_chat_typing();
            }
            return;
        }
        if event.repeat {
            // Stock runs a held key's down-only bind again on each repeat, so a held
            // `weapnext` keeps cycling; `+commands` stay held (hub report #309).
            if event.state == ElementState::Pressed
                && let Some(console) = &mut self.console
            {
                console.repeat_bound_key(key);
            }
            return;
        }
        // Releases were handled before routing, including the ones swallowed by
        // a menu or text field. Only presses reach this gameplay-binding path.
        if event.state == ElementState::Pressed
            && let Some(console) = &mut self.console
        {
            let name = crate::input::keys::key_name(&event);
            console.queue_bound_key(key, name, true);
        }
    }
}
