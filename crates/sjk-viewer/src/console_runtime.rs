//! Per-frame bridge from portable command buffering to viewer actions.

use super::*;

const MAX_ACTIONS_PER_FRAME: usize = 64;

impl GpuState {
    /// Execute cfg and bind scripts once per rendered frame.
    pub(crate) fn run_console_command_buffer(&mut self, audio: &mut Option<GameAudio>) {
        // A chip naming a setting (the changelog page's, the what's-new card's) opens
        // Settings on it.
        self.open_chip_settings();
        // Held mouse 4 and 5 repeat into the binds while the pointer and keys are the
        // game's, as their presses went (`in_mouseRepeat`).
        let mouse_repeat = self.pointer_captured && !self.text_has_keyboard();
        let Some(console) = &mut self.console else {
            return;
        };
        if mouse_repeat {
            console.repeat_held_mouse_buttons();
        }
        // The Collection's Toys tab lights or puts out the holocron through the console.
        if let Some(on) = console.take_illuminate_request() {
            self.illuminate.set_on(on);
        }
        if let Some(on) = console.take_cardboard_box_request() {
            self.cardboard_box.set_on(on);
        }
        if let Some(on) = console.take_order66_request() {
            self.order66.set_on(on);
        }
        if let Some(on) = console.take_hax_request() {
            self.hax.set_on(on);
        }
        if let Some(on) = console.take_ragdoll_request() {
            self.ragdoll.set_on(on);
        }
        if let Some(on) = console.take_drone_request() {
            self.drone.set_on(on);
        }
        if let Some(on) = console.take_monitor_request() {
            self.monitor.set_on(on);
        }
        if let Some(on) = console.take_box_hunt_request() {
            self.box_hunt.set_on(on);
        }
        if let Some(on) = console.take_banana_request() {
            self.banana.set_on(on);
        }
        if let Some(on) = console.take_fishing_request()
            && on != self.fishing.on()
        {
            self.fishing.toggle();
        }
        if let Some(on) = console.take_mining_request()
            && on != self.mining.on()
        {
            // As `toggle_mining`: `+use` is both toys', so the rod goes away first.
            if on && self.fishing.on() {
                self.fishing.toggle();
            }
            self.mining.toggle();
        }
        // The boombox checks who may carry it, so it is switched once the console is let
        // go; the Fishing card's Sell junk and the Mining card's Sell rubble too.
        let boombox_request = console.take_boombox_request();
        let sell_junk = console.take_fishing_sell_request();
        let sell_rubble = console.take_mining_sell_request();
        let mut actions = [None; MAX_ACTIONS_PER_FRAME];
        console.advance_command_frame();
        console.pump_console_socket();
        let mut action_count = 0;
        let gameplay = &mut self.gameplay_input;
        for command in console.drain_input() {
            if let Some(action) = gameplay.apply(&command) {
                actions[action_count] = Some(action);
                action_count += 1;
            }
        }
        console.execute_buffered_frame(self.live_session.as_mut(), |command, _| {
            if !input::GameplayInput::recognizes(command) {
                return false;
            }
            if let Some(action) = gameplay.apply(command)
                && action_count < actions.len()
            {
                actions[action_count] = Some(action);
                action_count += 1;
            }
            true
        });
        if gameplay.finish_buffered_input() {
            action_count = 0;
        }
        if console.take_quit() {
            self.quit_requested = true;
        }
        for action in actions[..action_count].iter().copied().flatten() {
            self.apply_input_action(Some(action));
        }
        if let Some(on) = boombox_request
            && on != self.boombox.on()
        {
            self.toggle_boombox();
        }
        if sell_junk {
            self.sell_fishing_junk();
        }
        if sell_rubble {
            self.sell_mining_rubble();
        }
        // `toy_illuminate` may have flipped it: tell the Toys tab. Order 66, the Hax, the
        // Drone, the Monitor, Box hunt and the Banana peel stay as last switched, across
        // maps, servers and restarts (`toy_saved`).
        if let Some(console) = self.console.as_mut() {
            let [order66, hax, drone, monitor, box_hunt, banana, propgun] = &mut self.toys_saved;
            let on = order66.sync(console, crate::toy_saved::ORDER66_CVAR, self.order66.on());
            self.order66.set_on(on);
            let on = hax.sync(console, crate::toy_saved::HAX_CVAR, self.hax.on());
            self.hax.set_on(on);
            let on = drone.sync(console, crate::toy_saved::DRONE_CVAR, self.drone.on());
            self.drone.set_on(on);
            let on = monitor.sync(console, crate::toy_saved::MONITOR_CVAR, self.monitor.on());
            self.monitor.set_on(on);
            let on = box_hunt.sync(console, crate::toy_saved::BOX_HUNT_CVAR, self.box_hunt.on());
            self.box_hunt.set_on(on);
            let on = banana.sync(console, crate::toy_saved::BANANA_CVAR, self.banana.on());
            self.banana.set_on(on);
            let on = propgun.sync(console, crate::toy_saved::PROPGUN_CVAR, self.propgun.on());
            self.propgun.set_on(on);
            console.set_illuminate_lit(self.illuminate.lit());
            console.set_cardboard_box_on(self.cardboard_box.on());
            console.set_order66_on(self.order66.on());
            console.set_hax_on(self.hax.on());
            console.set_ragdoll_on(self.ragdoll.on());
            console.set_drone_on(self.drone.on());
            console.set_boombox_on(self.boombox.on());
            console.set_monitor_on(self.monitor.on());
            console.set_box_hunt_on(self.box_hunt.on());
            console.set_banana_on(self.banana.on());
            console.set_fishing_on(self.fishing.on());
            console.set_mining_on(self.mining.on());
        }
        let connection = self
            .console
            .as_mut()
            .and_then(console::ViewerConsole::take_connection_action);
        if let Some(action) = connection {
            self.apply_console_connection_action(action);
        }
        let screenshot = self
            .console
            .as_mut()
            .and_then(console::ViewerConsole::take_screenshot_request);
        if let Some(request) = screenshot {
            self.screenshots.request(request);
        }
        let demo = self
            .console
            .as_mut()
            .and_then(console::ViewerConsole::take_demo_action);
        if let Some(action) = demo {
            self.apply_console_demo_action(action);
        }
        self.run_client_commands(audio);
        // Save settings changed this frame or earlier, once they hold still.
        if let Some(console) = self.console.as_mut() {
            console.persist();
            // A release made while playing shows its update card within half an hour.
            console.recheck_for_updates();
        }
    }
}
