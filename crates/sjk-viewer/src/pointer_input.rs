//! Platform pointer conversion, overlay routing, and cursor-grab policy.

use super::*;
use sjk_ui::{InputEvent, PointerButton, UiEventKind, Vec2};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::window::CursorGrabMode;

/// Mouse-look settings latched from `sensitivity` and `m_invert` each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MouseLook {
    /// Scales both axes (`cl_sensitivity`, `cl_input.cpp:1155-1159`).
    pub(crate) sensitivity: f32,
    /// Degrees of yaw per mouse count before sensitivity (`m_yaw`).
    pub(crate) yaw_scale: f32,
    /// Degrees of pitch per mouse count before sensitivity (`m_pitch`).
    pub(crate) pitch_scale: f32,
    /// Pull down to look up.
    pub(crate) invert: bool,
}

impl Default for MouseLook {
    fn default() -> Self {
        Self {
            sensitivity: 5.0,
            yaw_scale: 0.022,
            pitch_scale: 0.022,
            invert: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CursorMode {
    Free,
    Captured,
}

/// Cursor policy updates the OS cursor only when its desired state changes.
pub(crate) struct CursorPolicy {
    focused: bool,
    applied: Option<CursorMode>,
    ignore_next_motion: bool,
}

impl CursorPolicy {
    pub(crate) const fn new() -> Self {
        Self {
            focused: true,
            applied: None,
            ignore_next_motion: false,
        }
    }

    pub(crate) const fn desired(&self, live: bool, overlay: bool) -> CursorMode {
        if self.focused && live && !overlay {
            CursorMode::Captured
        } else {
            CursorMode::Free
        }
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn take_ignored_motion(&mut self) -> bool {
        std::mem::take(&mut self.ignore_next_motion)
    }

    fn finish_transition(&mut self, desired: CursorMode, was_captured: bool, captured: bool) {
        if was_captured && !captured {
            self.ignore_next_motion = true;
        }
        self.applied = Some(desired);
    }
}

/// How long after the window gets the focus a press counts as the click that gave it.
/// Windows delivers the two together; the rest allows for a frame or a hitch between.
const REFOCUS_CLICK: std::time::Duration = std::time::Duration::from_millis(250);

/// The mouse press that brings the window back into focus (hub report #281): the pointer
/// is captured again by then, so without this it would reach the game as an attack.
#[derive(Default)]
pub(crate) struct RefocusClick {
    gained: Option<std::time::Instant>,
    swallowing: Option<MouseButton>,
}

impl RefocusClick {
    fn gained(&mut self, now: std::time::Instant) {
        self.gained = Some(now);
    }

    /// Whether this press is the focus click, which the game must not see.
    fn press(&mut self, button: MouseButton, now: std::time::Instant) -> bool {
        let refocus = self
            .gained
            .take()
            .is_some_and(|gained| now.duration_since(gained) <= REFOCUS_CLICK);
        self.swallowing = refocus.then_some(button);
        refocus
    }

    /// Whether this release ends a focus click.
    fn release(&mut self, button: MouseButton) -> bool {
        if self.swallowing == Some(button) {
            self.swallowing = None;
            return true;
        }
        false
    }
}

impl GpuState {
    pub(crate) fn pointer_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = [position.x as f32, position.y as f32];
        self.route_pointer(InputEvent::PointerMove(Vec2::new(
            self.cursor_position[0],
            self.cursor_position[1],
        )));
    }

    pub(crate) fn pointer_left(&mut self) {
        self.route_pointer(InputEvent::PointerLeave);
    }

    pub(crate) fn pointer_button(&mut self, button: MouseButton, state: ElementState) {
        // Whoever takes the release, a held push or pull button stops repeating.
        if state == ElementState::Released
            && let Some(key) = input::keys::name(input::keys::Source::Mouse(button))
            && let Some(console) = &mut self.console
        {
            console.force_repeat_button_up(key);
        }
        // While the map editor flies every button is its own: the right one's release
        // ends the flight, and none attacks.
        if self.map_edit_flying() {
            if button == MouseButton::Right && state == ElementState::Released {
                self.map_edit_fly_stop();
            }
            return;
        }
        // An open quick wheel takes the left and right buttons: they change its
        // page, and neither attacks.
        let side = match button {
            MouseButton::Left => Some(crate::quick_wheel::Button::Left),
            MouseButton::Right => Some(crate::quick_wheel::Button::Right),
            _ => None,
        };
        if let Some(side) = side
            && self.quick_wheel.button(
                side,
                state == ElementState::Pressed,
                std::time::Instant::now(),
            )
        {
            return;
        }
        if state == ElementState::Pressed
            && self
                .client_menu
                .as_ref()
                .is_some_and(|menu| menu.is_visible())
            && match (&mut self.client_menu, &mut self.console) {
                (Some(menu), Some(console)) => menu.handle_mouse_binding(button, console),
                _ => false,
            }
        {
            return;
        }
        let Some(event) = normalize_button_event(button, state, self.cursor_position) else {
            if self.pointer_captured {
                self.route_stock_mouse_button(button, state == ElementState::Pressed);
            }
            return;
        };
        let button = normalize_button(button).expect("normalized event has a supported button");
        if self.route_pointer(event) {
            return;
        }
        if button == PointerButton::Primary && state == ElementState::Pressed {
            if !self.pointer_captured {
                self.capture_pointer();
                return;
            }
        }
        // The click that brought the window back only does that: no attack.
        let stock_button = match button {
            PointerButton::Primary => MouseButton::Left,
            PointerButton::Secondary => MouseButton::Right,
            PointerButton::Middle => MouseButton::Middle,
        };
        let refocus_click = if state == ElementState::Pressed {
            self.refocus_click
                .press(stock_button, std::time::Instant::now())
        } else {
            self.refocus_click.release(stock_button)
        };
        if refocus_click {
            return;
        }
        if button == PointerButton::Primary {
            if state == ElementState::Released && std::mem::take(&mut self.spectator_follow_click) {
                return;
            }
            if state == ElementState::Pressed {
                self.spectator_follow_click = false;
                if self.follow_aimed_spectator() {
                    self.spectator_follow_click = true;
                    return;
                }
            }
        }
        self.route_stock_mouse_button(
            match button {
                PointerButton::Primary => MouseButton::Left,
                PointerButton::Secondary => MouseButton::Right,
                PointerButton::Middle => MouseButton::Middle,
            },
            state == ElementState::Pressed,
        );
    }

    pub(crate) fn pointer_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = normalize_wheel(delta);
        // An open quick wheel takes the scroll: it changes page, not weapon.
        if self.quick_wheel.is_open() {
            self.quick_wheel
                .scrolled(delta.y, std::time::Instant::now());
            return;
        }
        // While the map editor flies the wheel sets the flight's speed.
        if self.map_edit_flying() {
            if let Some(up) = wheel_binding_direction(delta.y) {
                self.map_edit_fly_wheel(up);
            }
            return;
        }
        // A model the map editor grabbed turns with the wheel instead of changing weapon.
        if let Some(up) = wheel_binding_direction(delta.y)
            && !self.map_edit_panel_open()
        {
            let fine = self
                .console
                .as_ref()
                .is_some_and(|console| console.shift_held());
            if self.map_edit_wheel(up, fine) {
                return;
            }
        }
        if let Some(up) = wheel_binding_direction(delta.y)
            && self
                .client_menu
                .as_ref()
                .is_some_and(|menu| menu.is_visible())
            && match (&mut self.client_menu, &mut self.console) {
                (Some(menu), Some(console)) => menu.handle_wheel_binding(up, console),
                _ => false,
            }
        {
            return;
        }
        let event = InputEvent::PointerWheel {
            position: Vec2::new(self.cursor_position[0], self.cursor_position[1]),
            delta,
        };
        if self.route_pointer(event) {
            return;
        }
        self.route_gameplay_wheel(delta.y);
    }

    pub(crate) fn pointer_focus(&mut self, focused: bool) {
        self.cursor_policy.set_focus(focused);
        self.gameplay_input.focus(focused);
        if focused {
            self.refocus_click.gained(std::time::Instant::now());
        }
        if !focused {
            // The window that took the focus takes the right button's release too.
            self.map_edit_fly_stop();
            // The window that took the focus takes the slider's release too.
            if let Some(menu) = &mut self.client_menu {
                menu.cancel_slider_peek();
            }
            self.pending_generic_command = 0;
            self.generic_command_sent = 0;
            if let Some(console) = &mut self.console {
                console.clear_force_repeat();
            }
            // The released keys reach the server with the next command, at once.
            self.packet_pacer.reset();
        }
        self.sync_cursor_policy();
    }

    pub(crate) fn pointer_motion(&mut self, delta: (f64, f64)) {
        if !self.pointer_captured || self.cursor_policy.take_ignored_motion() {
            return;
        }
        // An open quick wheel takes the mouse: it moves the wheel's pointer, not the view.
        if self.quick_wheel.is_open() {
            self.quick_wheel.moved([delta.0 as f32, delta.1 as f32]);
            return;
        }
        // Accumulate raw counts; filtering and acceleration run once per command frame.
        self.gameplay_input.motion.raw[0] += delta.0 as f32;
        self.gameplay_input.motion.raw[1] += delta.1 as f32;
    }

    pub(crate) fn capture_pointer(&mut self) {
        self.sync_cursor_policy();
    }

    pub(crate) fn release_pointer(&mut self) {
        self.apply_cursor_mode(CursorMode::Free);
        self.gameplay_input.release_keys();
    }

    /// Free the pointer as [`Self::release_pointer`] does, keeping the movement keys
    /// held ([`crate::console_move`]).
    pub(crate) fn release_pointer_keeping_movement(&mut self) {
        self.apply_cursor_mode(CursorMode::Free);
        self.gameplay_input.release_keys_but_movement();
    }

    /// Keyboard catchers also advertise BUTTON_TALK, as CL_CmdButtons does.
    pub(crate) fn key_catcher_active(&self) -> bool {
        self.catcher_active(true)
    }

    /// [`Self::key_catcher_active`], the console counted or not.
    fn catcher_active(&self, console: bool) -> bool {
        self.game_menu
            || self.hud_edit.is_open()
            || self.text_dialog.is_open()
            // The SJK team's window, which may show in the middle of play.
            || self.official.window.is_open()
            || self.news.is_open()
            || self.chat.is_typing()
            || (console
                && self
                    .console
                    .as_ref()
                    .is_some_and(|console| console.is_open()))
            || self
                .client_menu
                .as_ref()
                .is_some_and(|menu| menu.is_visible())
    }

    /// Commands carry `BUTTON_TALK`: a key catcher is open, or the window is away
    /// (alt-tabbed or minimised, see [`crate::console::ViewerConsole::window_talk`]).
    /// With `cl_consoleMove` the open console alone does not ([`crate::console_move`]).
    pub(crate) fn talk_button(&self) -> bool {
        self.catcher_active(!self.console_keeps_moving())
            || self
                .console
                .as_ref()
                .is_some_and(|console| console.window_talk())
    }

    pub(crate) fn sync_cursor_policy(&mut self) {
        let overlay = self.key_catcher_active();
        let desired = self.cursor_policy.desired(
            self.live_session.is_some() || self.resident.exploring(),
            overlay,
        );
        self.apply_cursor_mode(desired);
    }

    fn apply_cursor_mode(&mut self, desired: CursorMode) {
        if self.cursor_policy.applied == Some(desired) {
            return;
        }
        let Some(window) = &self.window else {
            self.pointer_captured = false;
            self.cursor_policy.finish_transition(desired, false, false);
            return;
        };
        let was_captured = self.pointer_captured;
        self.pointer_captured = match desired {
            CursorMode::Free => {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
                false
            }
            CursorMode::Captured => {
                let captured = window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                    .is_ok();
                window.set_cursor_visible(!captured);
                captured
            }
        };
        self.cursor_policy
            .finish_transition(desired, was_captured, self.pointer_captured);
    }

    fn route_pointer(&mut self, event: InputEvent) -> bool {
        // The Profile screen's tabs, over whichever of its screens shows.
        if !self.text_dialog.is_open() && self.profile_hub_pointer(event) {
            return true;
        }
        // The SJK team's window, over the console too.
        if self.official.window.is_open() {
            self.official_pointer(event);
            return true;
        }
        // And the News panel.
        if self.news.is_open() {
            self.news_pointer(event);
            return true;
        }
        if self
            .console
            .as_ref()
            .is_some_and(|console| console.is_open())
        {
            if let Some(console) = &mut self.console {
                console.handle_pointer(event);
            }
            return true;
        }
        // The HUD edit mode takes the pointer over the game it edits.
        if self.hud_edit.is_open() && !self.text_dialog.is_open() {
            self.hud_edit_pointer(event);
            return true;
        }
        // The dialog is drawn over everything and takes the keys first; the pointer too.
        if self.text_dialog.is_open() {
            let action = self.text_dialog.handle_pointer(event);
            self.apply_dialog_action(action);
            return true;
        }
        if self.alpha_card.is_open() {
            let choice = self.alpha_card.handle_pointer(event);
            self.alpha_card_choice(choice);
            return true;
        }
        if self.medal_popup.is_open() {
            self.medal_popup.handle_pointer(event);
            return true;
        }
        if self.holocron_popup.is_open() {
            self.holocron_popup.handle_pointer(event);
            return true;
        }
        if self.graphics_reload.card.is_open() {
            let choice = self.graphics_reload.card.handle_pointer(event);
            self.reload_card_choice(choice);
            return true;
        }
        if self.small_card_notice.is_open() {
            let choice = self.small_card_notice.handle_pointer(event);
            self.small_card_notice_choice(choice);
            return true;
        }
        if self.playstyle_card.is_open() {
            let choice = self.playstyle_card.handle_pointer(event);
            self.playstyle_card_choice(choice);
            return true;
        }
        if self.emote_help.card.is_open() {
            let choice = self.emote_help.card.handle_pointer(event);
            self.emote_help_choice(choice);
            return true;
        }
        if self.emote_picker.card.is_open() {
            let choice = self.emote_picker.card.handle_pointer(event);
            self.emote_picker_choice(choice);
            return true;
        }
        if self.e2e_whispers.card.is_open() {
            let choice = self.e2e_whispers.card.handle_pointer(event);
            self.e2e_whisper_card_choice(choice);
            return true;
        }
        if self.mod_notice.card.is_open() {
            let choice = self.mod_notice.card.handle_pointer(event);
            self.mod_notice_choice(choice);
            return true;
        }
        if self.trades.window.is_open() {
            let choice = self.trades.window.handle_pointer(event);
            self.trade_window_choice(choice);
            return true;
        }
        // The trade toast, over the game menu or the chat, when the pointer is free.
        if self.trade_toast_pointer(event) {
            return true;
        }
        if self.fps_card.is_open() {
            let choice = self.fps_card.handle_pointer(event);
            self.fps_card_choice(choice);
            return true;
        }
        if self.log_request_card.is_open() {
            let choice = self.log_request_card.handle_pointer(event);
            self.log_request_card_choice(choice);
            return true;
        }
        // The update card, over the menus: a click on it installs the update.
        if self.update_card_pointer(event) {
            return true;
        }
        if self
            .client_menu
            .as_ref()
            .is_some_and(|menu| menu.is_visible())
        {
            let action = match (&mut self.client_menu, &mut self.console) {
                (Some(menu), Some(console)) => menu.handle_pointer(event, console),
                _ => menu::MenuAction::None,
            };
            self.apply_client_menu_action(action);
            return true;
        }
        if self.game_menu {
            // The SJK UI has no Report a bug button: it is on its SJK page.
            if !matches!(
                self.game_menu_page,
                GameMenuPage::Shot | GameMenuPage::MapEdit | GameMenuPage::GhostEdit
            ) && !self.in_game_menu.is_sjk()
                && self.text_dialog.launcher_pointer(event)
            {
                self.open_bug_report();
                return true;
            }
            if self.game_menu_page == GameMenuPage::Shot {
                self.shot_pointer(event);
                return true;
            }
            if self.game_menu_page == GameMenuPage::MapEdit {
                self.map_edit_pointer(event);
                return true;
            }
            if self.game_menu_page == GameMenuPage::GhostEdit {
                self.ghost_edit_pointer(event);
                return true;
            }
            if let Some((kind, row)) = self.in_game_menu.pointer(event) {
                // The SJK UI's profile card opens the Profile screen on its Profile tab.
                if row == usize::from(crate::ingame_menu::sjk_view::CARD_TOKEN)
                    && self.in_game_menu.is_sjk()
                {
                    if kind == UiEventKind::Activate {
                        self.open_profile_hub_from_game(Some(crate::profile_hub::Tab::Profile));
                    }
                    return true;
                }
                // Trade on the Players page's player card.
                if row == usize::from(crate::ingame_menu::sjk_view::TRADE_TOKEN)
                    && self.in_game_menu.is_sjk()
                {
                    if kind == UiEventKind::Activate {
                        self.trade_with_menu_player();
                    }
                    return true;
                }
                if self.sjk_main_pointer(kind, row) {
                    return true;
                }
                if let Some(tab) = crate::ingame_menu::classic::bar_tab(row) {
                    self.classic_bar_pointer(kind, tab);
                    return true;
                }
                if row < self.game_menu_row_count()
                    && matches!(kind, UiEventKind::HoverEnter | UiEventKind::Hover)
                {
                    self.game_menu_row = row;
                    // Hovering an entry gives the list the keyboard.
                    self.in_game_menu.focus = crate::ingame_menu::sjk_focus::Focus::List;
                }
                if row < self.game_menu_row_count()
                    && kind == UiEventKind::Activate
                    && self.in_game_menu.activation_allowed(row)
                {
                    self.game_menu_row = row;
                    self.activate_game_menu_row();
                }
            }
            return true;
        }
        if self.chat.is_typing() {
            self.chat.pointer(event);
            self.save_chat_settings();
            return true;
        }
        false
    }

    fn route_stock_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        let Some(key) = input::keys::name(input::keys::Source::Mouse(button)) else {
            return;
        };
        if let Some(console) = &mut self.console {
            console.queue_bound_script(key, pressed);
        }
    }

    /// Follow the player directly under a free spectator camera's crosshair.
    fn follow_aimed_spectator(&mut self) -> bool {
        let Some(session) = &mut self.live_session else {
            return false;
        };
        let aimed = self
            .crosshair_scan
            .aimed_client(session.latest_snapshot().server_time);
        let Some(target) = spectator_follow_target(
            session.latest_snapshot().player.is_spectator(),
            session.game_state().client_num,
            aimed,
        ) else {
            return false;
        };
        let command = format!("follow {target}");
        if let Err(error) = session.send_reliable_command_from(
            command.as_bytes(),
            sjk_client::ReliableCommandOrigin::Input,
        ) {
            eprintln!("failed to follow crosshair player {target}: {error}");
        }
        true
    }

    fn route_gameplay_wheel(&mut self, vertical: f32) {
        if self.live_session.is_none() || self.chat.is_typing() || self.game_menu {
            return;
        }
        if vertical == 0.0 {
            return;
        }
        let key = input::keys::name(input::keys::Source::Wheel(vertical > 0.0)).unwrap();
        if let Some(console) = &mut self.console {
            console.queue_bound_script(key, true);
            console.queue_bound_script(key, false);
        }
    }
}

fn normalize_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

fn spectator_follow_target(spectator: bool, local_client: i32, aimed: Option<u16>) -> Option<u16> {
    let local_client = u16::try_from(local_client).ok()?;
    aimed.filter(|target| spectator && *target < 32 && *target != local_client)
}

pub(crate) fn normalize_button_event(
    button: MouseButton,
    state: ElementState,
    position: [f32; 2],
) -> Option<InputEvent> {
    let button = normalize_button(button)?;
    let position = Vec2::new(position[0], position[1]);
    Some(match state {
        ElementState::Pressed => InputEvent::PointerPress { position, button },
        ElementState::Released => InputEvent::PointerRelease { position, button },
    })
}

/// Smallest normalized vertical wheel delta that binds a waiting key slot: half
/// a notch (a [`MouseScrollDelta::LineDelta`] notch normalizes to 40), so the
/// small pixel deltas of a trackpad do not bind by accident.
const WHEEL_BIND_MIN_DELTA: f32 = 20.0;

/// The direction a normalized wheel delta binds while a slot waits for a key:
/// `Some(true)` for `MWHEELUP` (positive y, away from the player, as
/// `route_gameplay_wheel` maps it), `Some(false)` for `MWHEELDOWN`, `None` below
/// [`WHEEL_BIND_MIN_DELTA`].
fn wheel_binding_direction(vertical: f32) -> Option<bool> {
    (vertical.abs() >= WHEEL_BIND_MIN_DELTA).then_some(vertical > 0.0)
}

fn normalize_wheel(delta: MouseScrollDelta) -> Vec2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vec2::new(x * 40.0, y * 40.0),
        MouseScrollDelta::PixelDelta(position) => Vec2::new(position.x as f32, position.y as f32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notch_binds_but_a_tiny_trackpad_delta_does_not() {
        let vertical = |delta| normalize_wheel(delta).y;
        assert_eq!(
            wheel_binding_direction(vertical(MouseScrollDelta::LineDelta(0.0, 1.0))),
            Some(true)
        );
        assert_eq!(
            wheel_binding_direction(vertical(MouseScrollDelta::LineDelta(0.0, -1.0))),
            Some(false)
        );
        for pixels in [0.0, 1.0, -3.5, 19.0, -19.9] {
            let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, pixels));
            assert_eq!(wheel_binding_direction(vertical(delta)), None, "{pixels}");
        }
        let swipe = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 60.0));
        assert_eq!(wheel_binding_direction(vertical(swipe)), Some(true));
    }

    #[test]
    fn only_a_free_spectator_can_follow_the_aimed_player() {
        assert_eq!(spectator_follow_target(true, 2, Some(7)), Some(7));
        assert_eq!(spectator_follow_target(false, 2, Some(7)), None);
        assert_eq!(spectator_follow_target(true, 2, None), None);
        assert_eq!(spectator_follow_target(true, 2, Some(2)), None);
        assert_eq!(spectator_follow_target(true, 2, Some(32)), None);
    }
}

#[cfg(test)]
mod refocus_click_tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn the_click_that_gives_the_focus_never_reaches_the_game() {
        let now = Instant::now();
        let mut click = RefocusClick::default();
        click.gained(now);
        assert!(click.press(MouseButton::Left, now + Duration::from_millis(5)));
        assert!(click.release(MouseButton::Left));
        // The next click is an ordinary one, press and release.
        assert!(!click.press(MouseButton::Left, now + Duration::from_millis(50)));
        assert!(!click.release(MouseButton::Left));
    }

    #[test]
    fn a_later_click_after_alt_tabbing_back_still_attacks() {
        let now = Instant::now();
        let mut click = RefocusClick::default();
        click.gained(now);
        assert!(!click.press(MouseButton::Left, now + Duration::from_secs(1)));
        assert!(!click.release(MouseButton::Left));
    }

    #[test]
    fn a_lost_release_does_not_eat_a_later_one() {
        let now = Instant::now();
        let mut click = RefocusClick::default();
        click.gained(now);
        assert!(click.press(MouseButton::Left, now));
        // A menu took the release; the next press starts clean.
        assert!(!click.press(MouseButton::Left, now + Duration::from_secs(2)));
        assert!(!click.release(MouseButton::Left));
    }
}
