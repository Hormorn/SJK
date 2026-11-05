//! Bind-command interpretation for gameplay input.

use sjk_protocol::UserCommand;
pub(crate) mod alt_code;
pub(crate) mod dead_key;
pub(crate) mod flip_kick;
pub(crate) mod force_repeat;
pub(crate) mod idrive;
pub(crate) mod motion;
pub(crate) mod mouse_repeat;

pub(crate) mod force_abilities;
mod selection_commands;
pub(crate) mod settings;
mod state;
pub(crate) mod view_authority;

pub(crate) mod generic_commands;
mod key_events;
pub(crate) mod keys;

pub(crate) use generic_commands::{GENCMD_SABER_ATTACK_CYCLE, GENCMD_SABER_SWITCH};

/// Logical buttons consumed by user-command generation and free-camera motion.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GameButton {
    Forward,
    Back,
    MoveLeft,
    MoveRight,
    Up,
    Down,
    Speed,
    Scores,
    Left,
    Right,
    Lookup,
    Lookdown,
    Strafe,
    Mlook,
    Button(u8),
    /// `+force_stasis` (JoF EJK `cl_input.cpp`): the usercmd Stasis button, sent
    /// only where a JoF JA+ server granted Stasis ([`sjk_client::force_wheel`]).
    ForceStasis,
    /// `+duck` (JoF EJK `CG_NorollDown_f`): crouch without rolling. See
    /// [`GameplayInput::up_move`].
    Duck,
}

/// One-shot action emitted by a non-button bind command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputAction {
    MessageMode(bool),
    /// Whisper to the crosshair player, or last attacker when true.
    TargetMessage(bool),
    /// The SJK chat, through the SJK hub (`messagemode5`).
    SjkMessageMode,
    /// Admin chat, `amsay` (`messagemode6`, `chat/tabs.rs`).
    AdminMessageMode,
    /// A clan chat through the SJK hub (`messagemode7` JoF, `messagemode8` S,
    /// `chat/tabs.rs`).
    ClanMessageMode(sjk_identity::HubChannel),
    ToggleCamera,
    SaberToggle,
    SaberStyle,
    Weapon(u8),
    WeaponCycle(i8),
    /// `weapmelee`: select the fists (SJK's), local like the other weapon binds.
    WeaponMelee,
    /// Local force/inventory cycling; never a reliable server command.
    SelectionCycle(bool, i8),
    /// `forceselect <entry>`: select that Force wheel entry at once (the quick
    /// wheel's Force page); local, as cycling is.
    ForceSelect(u8),
    /// A capability-gated instant server ability.
    ForceAbility(u8),
    RequestScores,
    TeamMenu,
    Vote(bool),
    /// A `genCmds_t` value for the next user command (`force_throw`, `taunt`, ...).
    GenericCommand(u8),
    /// `centerview`: level the pitch (`codemp/client/cl_input.cpp:797`).
    CenterView,
    /// Apply the current cl_freelook value when the viewer handles mouse-look release.
    MlookReleased,
    /// `-grapple`: on JA+ servers EternalJK also taps `+use` to let go of the hook.
    GrappleReleased,
    /// `flipkick`: start JoF EJK's run of jump taps ([`flip_kick`]).
    FlipKick,
    /// `inspect`: pin the player card to the player under the crosshair, or unpin it.
    Inspect,
    /// `worldnote`: select the surface or mover under the crosshair, then write a note on
    /// it ([`crate::world_notes`]).
    WorldNote,
    /// `toy_illuminate` (or its old name `force_illuminate`): turn Illuminate's holocron
    /// on or off ([`crate::illuminate`]).
    Illuminate,
    /// `toy_box`: put the cardboard box on or take it off ([`crate::cardboard_box`]).
    CardboardBox,
    /// `toy_fishing`: get the fishing rod out or put it away ([`crate::fishing`]).
    Fishing,
    /// `toy_mining`: take the pickaxe in hand or put it away ([`crate::mining`]).
    Mining,
    /// `fishing_sell`: sell the fished junk to the hub ([`crate::fishing_sell`]).
    FishingSell,
    /// `fishing_log`: print the fish log in full to the console.
    FishingLog,
    /// `mining_sell`: sell the dug rubble to the hub ([`crate::mining_sell`]).
    MiningSell,
    /// `mining_log`: print the mining log in full to the console.
    MiningLog,
    /// `toy_order66`: turn the Order 66 toy on or off ([`crate::order66`]).
    Order66,
    /// `order66`: execute Order 66 for every player who has the toy on.
    ExecuteOrder66,
    /// `toy_hax`: turn the Hax toy on or off ([`crate::hax`]).
    Hax,
    /// `toy_ragdoll`: go limp, or get up ([`crate::ragdoll`]).
    Ragdoll,
    /// `toy_drone`: let drones follow the player, or not ([`crate::drone`]).
    Drone,
    /// `toy_boombox`: carry the stereo and play music, or not ([`crate::boombox`]).
    Boombox,
    /// `boombox_place`: stand the boombox on the floor, or pick it up ([`crate::boombox`]).
    PlaceBoombox,
    /// `toy_monitor`: throw monitors and be hit by them, or not ([`crate::monitor`]).
    Monitor,
    /// `monitor_throw`: throw a monitor ([`crate::monitor`]).
    ThrowMonitor,
    /// `+monitor_throw` (true) charges a throw while held, `-monitor_throw` throws it.
    ChargeMonitor(bool),
    /// `toy_boxhunt`: take part in the box hunt, or not ([`crate::box_hunt`]).
    BoxHunt,
    /// `toy_banana`: drop banana peels and slip on them, or not ([`crate::banana`]).
    Banana,
    /// `banana_drop`: drop a banana peel ([`crate::banana`]).
    DropBanana,
    /// `toy_propgun`: hold the chosen prop gun and fire it, or not ([`crate::propgun`]).
    PropGun,
    /// `toy_tag`: stick the chosen tag where the crosshair meets the world
    /// ([`crate::tags`]).
    StickTag,
    /// `toy_tag clear`: take the own tag off.
    ClearTag,
    /// `cameracontrol`: open Camera control, as F8 does
    /// ([`crate::ingame_menu::Page::Shot`]).
    CameraControl,
    /// `+ping` (its press) or `worldping`: ping the point under the crosshair
    /// ([`crate::world_ping`]).
    WorldPing,
}

/// Current logical gameplay-input state.
pub(crate) struct GameplayInput {
    /// Snapshot angle baseline, independent of button/focus resets.
    pub(crate) view_authority: view_authority::Authority,
    /// Selection survives acknowledgement/replay and ordinary input releases.
    pub(crate) selection: sjk_client::selection::Selection,
    /// `+useforce` on a selected JoF pseudo-slot, with its press edges.
    force_wheel_use: sjk_client::force_wheel::UseRemap,
    /// A `flipkick` run, stepped once per sent user command.
    pub(crate) flip_kick: flip_kick::FlipKick,
    held: [state::KeyState; 32],
    pub(crate) motion: motion::Motion,
    /// `cl_run`: the walk key toggles walking instead of running.
    always_run: bool,
    /// `cl_idrive` and `cl_idriveDelay`: the last-pressed key of a pair wins.
    idrive: idrive::Idrive,
    focused: bool,
    reset_after_buffer: bool,
}

impl Default for GameplayInput {
    /// `cl_run` defaults to 1 (`codemp/client/cl_main.cpp:2773`).
    fn default() -> Self {
        Self {
            view_authority: view_authority::Authority::default(),
            selection: sjk_client::selection::Selection::default(),
            force_wheel_use: sjk_client::force_wheel::UseRemap::default(),
            flip_kick: flip_kick::FlipKick::default(),
            held: [state::KeyState::default(); 32],
            motion: motion::Motion::default(),
            always_run: true,
            idrive: idrive::Idrive::default(),
            focused: true,
            reset_after_buffer: false,
        }
    }
}

impl GameplayInput {
    /// Whether the window has keyboard focus.
    pub(crate) fn is_focused(&self) -> bool {
        self.focused
    }

    /// Window focus belongs to the shell, so carry it across GPU world installs.
    pub(crate) fn inherit_focus(&mut self, previous: &Self) {
        self.focused = previous.focused;
        self.reset_after_buffer = previous.reset_after_buffer;
    }

    /// Losing focus releases held keys as `Key_ClearStates` does; holds typed at
    /// the console last until their `-` command.
    pub(crate) fn focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.release_keys();
            self.reset_after_buffer = true;
        }
    }

    /// Focus can change after a bind was queued but before its script executes.
    /// Drop that frame's key actions too, including a quick out-and-back.
    pub(crate) fn finish_buffered_input(&mut self) -> bool {
        let reset = std::mem::take(&mut self.reset_after_buffer) || !self.focused;
        if reset {
            self.release_keys();
        }
        reset
    }

    pub(crate) fn accepts_keyboard(&self, synthetic: bool) -> bool {
        self.focused && !synthetic
    }

    /// Drop every held input, including holds typed at the console.
    pub(crate) fn clear(&mut self) {
        self.held = [state::KeyState::default(); 32];
        self.motion.clear();
        self.flip_kick.stop();
    }

    /// Release keys when the console, a menu or chat takes the keyboard, as
    /// `Key_ClearStates` does, keeping holds typed at the console such as `+button12`.
    pub(crate) fn release_keys(&mut self) {
        for state in &mut self.held {
            state.release_keys();
        }
        self.motion.clear();
    }

    /// [`Self::release_keys`] but for the movement keys (forward, back, strafing, jump,
    /// crouch, walk), which keep their hold: the console opened with `cl_consoleMove`
    /// ([`crate::console_move`]).
    pub(crate) fn release_keys_but_movement(&mut self) {
        for (slot, state) in self.held.iter_mut().enumerate() {
            if !console_move_keeps(slot) {
                state.release_keys();
            }
        }
        self.motion.clear();
    }

    /// Set `button` in the next user command only, like a `+` and `-` one frame apart.
    pub(crate) fn tap(&mut self, button: GameButton) {
        self.held[button.slot()].pressed = true;
    }

    /// `lowjump` (JoF EJK's `CG_Lowjump_f`, `+moveup` then `-moveup` a frame later):
    /// jump held in the next user command only, so the jump is as short as a tap can
    /// make it. The server sees one command with jump pressed, which a quick finger
    /// could send too; the height is whatever the server's movement gives that tap.
    pub(crate) fn low_jump(&mut self) {
        self.tap(GameButton::Up);
    }

    /// Give `command` the `flipkick` run's jump state. Like EJK's `+moveup` and
    /// `-moveup`, the run overrides a held jump key, and its end lets go of it.
    pub(crate) fn apply_flip_kick(&mut self, command: &mut UserCommand, timing: flip_kick::Timing) {
        let jump = match self.flip_kick.step(timing) {
            flip_kick::Step::Idle => return,
            flip_kick::Step::Jump(jump) => jump,
            flip_kick::Step::End => {
                self.held[GameButton::Up.slot()].event(false, None, self.motion.now);
                false
            }
        };
        command.up_move = if jump { 127 } else { command.up_move.min(0) };
    }

    /// Latch `cl_run` (retail default 1); see `user_command`.
    pub(crate) fn set_always_run(&mut self, always_run: bool) {
        self.always_run = always_run;
    }

    /// Latch `cl_idrive` and `cl_idriveDelay`; see [`idrive`].
    pub(crate) fn set_idrive(&mut self, idrive: idrive::Idrive) {
        self.idrive = idrive;
    }

    /// The command about to be built was made `millis` before the frame's sample time
    /// (0 for the last or only command of a frame), so `cl_idriveDelay` is counted in
    /// command time. Reset it to 0 after the commands of a frame.
    pub(crate) fn set_command_age(&mut self, millis: u64) {
        self.motion.command_age = millis;
    }

    pub(crate) fn held(&self, button: GameButton) -> bool {
        self.held[button.slot()].active
    }

    /// Give `+useforce` and `+force_stasis` to a selected or granted JoF ability
    /// (`known` is the player's `forcePowersKnown`); returns the command's
    /// buttons and a server command to send with it.
    pub(crate) fn force_wheel_buttons(
        &mut self,
        buttons: u16,
        known: u32,
    ) -> (u16, Option<&'static str>) {
        let stasis = &self.held[GameButton::ForceStasis.slot()];
        let stasis = stasis.active || stasis.pressed;
        self.force_wheel_use.apply_with(
            buttons,
            self.selection.wheel_pseudo(),
            known,
            stasis,
            self.selection.extensions(),
        )
    }

    /// Return whether a buffered console command belongs to gameplay input.
    pub(crate) fn recognizes(command: &str) -> bool {
        let mut words = command.split_ascii_whitespace();
        let Some(name) = words.next().map(str::to_ascii_lowercase) else {
            return false;
        };
        button_for_command(&name).is_some()
            || generic_commands::generic_command(&name).is_some()
            || force_abilities::command_slot(&name).is_some()
            || matches!(
                name.as_str(),
                "messagemode"
                    | "messagemode2"
                    | "messagemode3"
                    | "messagemode4"
                    | "messagemode5"
                    | "messagemode6"
                    | "messagemode7"
                    | "messagemode8"
                    | "togglecamera"
                    | "sabertoggle"
                    | "saberstyle"
                    | "centerview"
                    | "weapnext"
                    | "weapmelee"
                    | "weapprev"
                    | "forcenext"
                    | "forceprev"
                    | "forceselect"
                    | "invnext"
                    | "invprev"
                    | "teammenu"
                    | "joinmenu"
                    | "vote"
                    | "weapon"
                    | "flipkick"
                    | LOW_JUMP_COMMAND
                    | "inspect"
                    | "worldnote"
                    | crate::world_ping::PRESS_COMMAND
                    | crate::world_ping::RELEASE_COMMAND
                    | crate::world_ping::COMMAND
                    | "toy_illuminate"
                    | "force_illuminate"
                    | crate::cardboard_box::COMMAND
                    | crate::fishing::COMMAND
                    | crate::mining::COMMAND
                    | crate::fishing_sell::COMMAND
                    | crate::fishing_sell::LOG_COMMAND
                    | crate::mining_sell::COMMAND
                    | crate::mining_sell::LOG_COMMAND
                    | crate::order66::COMMAND
                    | crate::order66::EXECUTE_COMMAND
                    | crate::hax::COMMAND
                    | crate::ragdoll::COMMAND
                    | crate::drone::COMMAND
                    | crate::boombox::COMMAND
                    | crate::boombox::PLACE_COMMAND
                    | crate::monitor::COMMAND
                    | crate::monitor::THROW_COMMAND
                    | crate::monitor::CHARGE_COMMAND
                    | crate::monitor::CHARGE_RELEASE
                    | crate::box_hunt::COMMAND
                    | crate::banana::COMMAND
                    | crate::banana::DROP_COMMAND
                    | crate::propgun::COMMAND
                    | crate::tags::COMMAND
                    | "cameracontrol"
            )
    }

    /// Build the protocol-26 command represented by the logical bind state.
    /// Intermission is deliberately NOT sanitized here: the stock client's
    /// `CL_CreateCmd` sends the raw input and `codemp/game/g_active.c:838-843`
    /// (`ClientIntermissionThink`) latches `BUTTON_ATTACK|BUTTON_USE_HOLDABLE`
    /// to mark the client ready to exit; bg_pmove ignores the movement.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn user_command(
        &self,
        server_time: i32,
        camera_pitch: f32,
        camera_yaw: f32,
        delta_angles: [i32; 3],
        weapon: u8,
        force_selection: u8,
        generic_command: u8,
    ) -> UserCommand {
        let mut buttons = 0_u16;
        for (index, state) in self.held[..16].iter().enumerate() {
            if state.active || state.pressed {
                buttons |= 1 << index;
            }
        }
        // `CL_KeyMove` (`codemp/client/cl_input.cpp:888-895`): running when
        // `in_speed.active ^ cl_run->integer`, otherwise `BUTTON_WALKING` with
        // `movespeed` 46 instead of 127. With `cl_run 1` the walk key walks.
        // `+duck` walks for the command after its press ([`Self::up_move`]).
        let walking = !(self.held(GameButton::Speed) ^ self.always_run)
            || self.held[GameButton::Duck.slot()].pressed;
        let movespeed = if walking { 46 } else { 127 };
        buttons = (buttons & !16) | u16::from(walking) * 16;
        UserCommand {
            server_time,
            angles: [
                super::angle_to_short(-camera_pitch).wrapping_sub(delta_angles[0]),
                super::angle_to_short(camera_yaw).wrapping_sub(delta_angles[1]),
                0_i32.wrapping_sub(delta_angles[2]),
            ],
            forward_move: self.axis(
                GameButton::Forward,
                GameButton::Back,
                movespeed,
                self.motion.forward,
            ),
            right_move: self.axis(
                GameButton::MoveRight,
                GameButton::MoveLeft,
                movespeed,
                self.motion.side
                    + if self.held(GameButton::Strafe) {
                        let (right, left) = self.idrive_pair(
                            GameButton::Right,
                            GameButton::Left,
                            self.fraction(GameButton::Right),
                            self.fraction(GameButton::Left),
                        );
                        movespeed as f32 * (right - left)
                    } else {
                        0.0
                    },
            ),
            up_move: self.up_move(movespeed),
            buttons,
            weapon,
            force_selection: self.selection.force.unwrap_or(force_selection),
            inventory_selection: self.selection.inventory.unwrap_or(255),
            generic_command,
            ..UserCommand::default()
        }
    }

    /// Jump and crouch, with `+duck` as JoF EJK plays it (`cg_consolecmds.c`
    /// `CG_NorollDown_f`): its press lets go of jump and walks for one frame, then
    /// it crouches like `+movedown`. Moving into a crouch rolls only from a running
    /// legs animation (`bg_pmove.c` `PM_Footsteps`), and one walking command
    /// replaces it with the walk; once crouched, the legs crouch instead.
    /// EJK walks with `+speed`, which runs under `cl_run 0`; SJK always walks.
    fn up_move(&self, speed: i8) -> i8 {
        let duck = &self.held[GameButton::Duck.slot()];
        if duck.pressed {
            return self
                .axis(GameButton::Up, GameButton::Down, speed, 0.0)
                .min(0);
        }
        // In EJK `+duck` presses `+movedown`, so the two share one key state: the
        // longer-held of the two is the crouch key, for `cl_idrive` too.
        let crouch = if self.movement_fraction(GameButton::Duck)
            > self.movement_fraction(GameButton::Down)
        {
            GameButton::Duck
        } else {
            GameButton::Down
        };
        self.axis(GameButton::Up, crouch, speed, 0.0)
    }

    /// Apply one command resolved by the bind table.
    pub(crate) fn apply(&mut self, command: &str) -> Option<InputAction> {
        if !self.focused {
            return None;
        }
        let mut words = command.split_ascii_whitespace();
        let name = words.next()?.to_ascii_lowercase();
        if let Some(button) = button_for_command(&name) {
            let pressed = name.starts_with('+');
            let key = words.next().and_then(|word| word.parse().ok());
            let time = words
                .next()
                .and_then(|word| word.parse().ok())
                .unwrap_or(self.motion.now);
            let changed = self.held[button.slot()].event(pressed, key, time);
            if changed && pressed && button == GameButton::Duck {
                // EJK's `-moveup`: a typed release, letting go of every jump key.
                self.held[GameButton::Up.slot()].event(false, None, time);
            }
            if !pressed && button == GameButton::Mlook {
                return Some(InputAction::MlookReleased);
            }
            if name == "-grapple" {
                return Some(InputAction::GrappleReleased);
            }
            return (changed && pressed && button == GameButton::Scores)
                .then_some(InputAction::RequestScores);
        }
        if let Some(value) = generic_commands::generic_command(&name) {
            return Some(InputAction::GenericCommand(value));
        }
        if let Some(slot) = force_abilities::command_slot(&name) {
            return Some(InputAction::ForceAbility(slot));
        }
        if let Some(action) = chat_action(&name) {
            return Some(action);
        }
        match name.as_str() {
            "togglecamera" => Some(InputAction::ToggleCamera),
            "sabertoggle" => Some(InputAction::SaberToggle),
            "saberstyle" => Some(InputAction::SaberStyle),
            "centerview" => Some(InputAction::CenterView),
            "weapnext" => Some(InputAction::WeaponCycle(1)),
            "weapmelee" => Some(InputAction::WeaponMelee),
            "weapprev" => Some(InputAction::WeaponCycle(-1)),
            "forcenext" => Some(InputAction::SelectionCycle(false, 1)),
            "forceprev" => Some(InputAction::SelectionCycle(false, -1)),
            "forceselect" => words
                .next()
                .and_then(|word| word.parse::<u8>().ok())
                .map(InputAction::ForceSelect),
            "invnext" => Some(InputAction::SelectionCycle(true, 1)),
            "invprev" => Some(InputAction::SelectionCycle(true, -1)),
            "teammenu" | "joinmenu" => Some(InputAction::TeamMenu),
            "flipkick" => Some(InputAction::FlipKick),
            LOW_JUMP_COMMAND => {
                self.low_jump();
                None
            }
            "inspect" => Some(InputAction::Inspect),
            "worldnote" => Some(InputAction::WorldNote),
            crate::world_ping::PRESS_COMMAND | crate::world_ping::COMMAND => {
                Some(InputAction::WorldPing)
            }
            // The old name still runs the toy, but is not listed (`selection_commands`).
            "toy_illuminate" | "force_illuminate" => Some(InputAction::Illuminate),
            crate::cardboard_box::COMMAND => Some(InputAction::CardboardBox),
            crate::fishing::COMMAND => Some(InputAction::Fishing),
            crate::mining::COMMAND => Some(InputAction::Mining),
            crate::fishing_sell::COMMAND => Some(InputAction::FishingSell),
            crate::fishing_sell::LOG_COMMAND => Some(InputAction::FishingLog),
            crate::mining_sell::COMMAND => Some(InputAction::MiningSell),
            crate::mining_sell::LOG_COMMAND => Some(InputAction::MiningLog),
            crate::order66::COMMAND => Some(InputAction::Order66),
            crate::order66::EXECUTE_COMMAND => Some(InputAction::ExecuteOrder66),
            crate::hax::COMMAND => Some(InputAction::Hax),
            crate::ragdoll::COMMAND => Some(InputAction::Ragdoll),
            crate::drone::COMMAND => Some(InputAction::Drone),
            crate::boombox::COMMAND => Some(InputAction::Boombox),
            crate::boombox::PLACE_COMMAND => Some(InputAction::PlaceBoombox),
            crate::monitor::COMMAND => Some(InputAction::Monitor),
            crate::monitor::THROW_COMMAND => Some(InputAction::ThrowMonitor),
            crate::monitor::CHARGE_COMMAND => Some(InputAction::ChargeMonitor(true)),
            crate::monitor::CHARGE_RELEASE => Some(InputAction::ChargeMonitor(false)),
            crate::box_hunt::COMMAND => Some(InputAction::BoxHunt),
            crate::banana::COMMAND => Some(InputAction::Banana),
            crate::banana::DROP_COMMAND => Some(InputAction::DropBanana),
            crate::propgun::COMMAND => Some(InputAction::PropGun),
            crate::tags::COMMAND => match words.next().map(str::to_ascii_lowercase).as_deref() {
                Some("clear" | "off") => Some(InputAction::ClearTag),
                _ => Some(InputAction::StickTag),
            },
            "cameracontrol" => Some(InputAction::CameraControl),
            "vote" => match words.next().map(str::to_ascii_lowercase).as_deref() {
                Some("yes" | "y" | "1") => Some(InputAction::Vote(true)),
                Some("no" | "n" | "0") => Some(InputAction::Vote(false)),
                _ => None,
            },
            "weapon" => words
                .next()
                .and_then(|word| word.parse::<u8>().ok())
                .map(InputAction::Weapon),
            _ => None,
        }
    }
}

/// Whether the button in `slot` is a movement key [`GameplayInput::release_keys_but_movement`]
/// keeps.
fn console_move_keeps(slot: usize) -> bool {
    [
        GameButton::Forward,
        GameButton::Back,
        GameButton::MoveLeft,
        GameButton::MoveRight,
        GameButton::Up,
        GameButton::Down,
        GameButton::Duck,
        GameButton::Speed,
    ]
    .iter()
    .any(|button| button.slot() == slot)
}

/// JoF EJK's low jump bind ([`GameplayInput::low_jump`]).
pub(crate) const LOW_JUMP_COMMAND: &str = "lowjump";

fn button_for_command(command: &str) -> Option<GameButton> {
    let name = command.strip_prefix(['+', '-'])?;
    if let Some(index) = name
        .strip_prefix("button")
        .and_then(|n| n.parse::<u8>().ok())
    {
        return (index < 16).then_some(GameButton::Button(index));
    }
    match name {
        "left" => Some(GameButton::Left),
        "right" => Some(GameButton::Right),
        "lookup" => Some(GameButton::Lookup),
        "lookdown" => Some(GameButton::Lookdown),
        "strafe" => Some(GameButton::Strafe),
        "mlook" => Some(GameButton::Mlook),
        "forward" => Some(GameButton::Forward),
        "back" => Some(GameButton::Back),
        "moveleft" => Some(GameButton::MoveLeft),
        "moveright" => Some(GameButton::MoveRight),
        "moveup" => Some(GameButton::Up),
        "movedown" => Some(GameButton::Down),
        // JoF EJK `cg_consolecmds.c` (`+duck`): crouch without rolling.
        "duck" => Some(GameButton::Duck),
        "speed" => Some(GameButton::Speed),
        "attack" => Some(GameButton::Button(0)),
        "altattack" => Some(GameButton::Button(7)),
        "use" => Some(GameButton::Button(5)),
        // `+useforce` is the stock name; `+force` is the pre-step-73e spelling.
        "useforce" | "force" => Some(GameButton::Button(9)),
        "force_grip" => Some(GameButton::Button(6)),
        "force_lightning" => Some(GameButton::Button(10)),
        "force_drain" => Some(GameButton::Button(11)),
        // EternalJK cg_consolecmds.c:1003-1015 (`+grapple`): `+button12`, the
        // JA+/JaPRO grapple hook.
        "grapple" => Some(GameButton::Button(12)),
        "force_stasis" => Some(GameButton::ForceStasis),
        "scores" => Some(GameButton::Scores),
        _ => None,
    }
}

/// The chat a bound command opens: `messagemode` (All) and `messagemode2` (Team),
/// `messagemode3`/`messagemode4` (whispers), `messagemode5` (SJK chat) and
/// `messagemode6` (Admin chat), `messagemode7` and `messagemode8` (the clan chats);
/// `None` for any other command.
pub(crate) fn chat_action(command: &str) -> Option<InputAction> {
    let name = command.split_ascii_whitespace().next()?;
    let action = match name.to_ascii_lowercase().as_str() {
        "messagemode" => InputAction::MessageMode(false),
        "messagemode2" => InputAction::MessageMode(true),
        "messagemode3" => InputAction::TargetMessage(false),
        "messagemode4" => InputAction::TargetMessage(true),
        "messagemode5" => InputAction::SjkMessageMode,
        "messagemode6" => InputAction::AdminMessageMode,
        "messagemode7" => InputAction::ClanMessageMode(sjk_identity::HubChannel::Jof),
        "messagemode8" => InputAction::ClanMessageMode(sjk_identity::HubChannel::Sith),
        _ => return None,
    };
    Some(action)
}

impl super::GpuState {
    pub(crate) fn apply_input_action(&mut self, action: Option<InputAction>) {
        self.refresh_force_support();
        match action {
            Some(InputAction::TargetMessage(attacker)) => self.targeted_chat(attacker),
            Some(InputAction::MessageMode(team)) if self.live_session.is_some() => {
                self.gameplay_input.release_keys();
                // The chat key opens on All, or on the last channel used with
                // `cg_chatRememberChannel` 1 (`chat/reply.rs`).
                if team {
                    self.chat.open(true);
                } else {
                    self.chat.open_chat();
                }
                self.sync_cursor_policy();
            }
            Some(InputAction::SjkMessageMode) if self.live_session.is_some() => {
                self.gameplay_input.release_keys();
                self.chat.open_sjk();
                self.sync_cursor_policy();
            }
            Some(InputAction::AdminMessageMode) if self.live_session.is_some() => {
                self.gameplay_input.release_keys();
                self.chat.open_admin();
                self.sync_cursor_policy();
            }
            Some(InputAction::AdminMessageMode) => {}
            Some(InputAction::ClanMessageMode(clan)) if self.live_session.is_some() => {
                self.gameplay_input.release_keys();
                self.chat.open_clan(clan);
                self.sync_cursor_policy();
            }
            // The clan chats show only in a game's chat.
            Some(InputAction::ClanMessageMode(_)) => {}
            // Without a game the composer has nowhere to show: the page has it.
            Some(InputAction::SjkMessageMode) => {
                if let Some(console) = &mut self.console {
                    console.open_sjk_chat_panel();
                }
                self.sync_cursor_policy();
            }
            // The choice, not the derived `third_person`: during a zoom it decides what
            // the camera is once the zoom ends (`GpuState::update_zoom_view`).
            Some(InputAction::ToggleCamera) => self.third_person_choice = !self.third_person_choice,
            Some(InputAction::SaberToggle) => {
                self.pending_generic_command = GENCMD_SABER_SWITCH;
            }
            Some(InputAction::SaberStyle) => {
                self.pending_generic_command = GENCMD_SABER_ATTACK_CYCLE;
            }
            Some(InputAction::GenericCommand(value)) => self.pending_generic_command = value,
            // Camera pitch is already delta-adjusted; wire encoding subtracts delta_angles.
            // Thus physical pitch zero encodes -SHORT2ANGLE(delta), cl_input.cpp:797.
            Some(InputAction::CenterView) => self.camera_pitch = 0.0,
            Some(InputAction::MlookReleased) => {
                if self
                    .console
                    .as_ref()
                    .map_or(!self.gameplay_input.motion.freelook, |console| {
                        !console.bool_cvar("cl_freelook").unwrap_or(true)
                    })
                {
                    self.camera_pitch = 0.0;
                }
            }
            Some(InputAction::GrappleReleased) => {
                if self.live_session.as_ref().is_some_and(|session| {
                    matches!(
                        session.compat_profile(),
                        sjk_client::CompatProfile::JaPlus { .. }
                    )
                }) {
                    self.gameplay_input.tap(GameButton::Button(5));
                }
            }
            Some(InputAction::Inspect) => self.hud.card.inspect(),
            Some(InputAction::WorldNote) => self.world_note_press(),
            Some(InputAction::WorldPing) => self.world_ping_press(),
            Some(InputAction::Illuminate) => self.toggle_illuminate(),
            Some(InputAction::CardboardBox) => self.toggle_cardboard_box(),
            Some(InputAction::Fishing) => self.toggle_fishing(),
            Some(InputAction::Mining) => self.toggle_mining(),
            Some(InputAction::FishingSell) => self.sell_fishing_junk(),
            Some(InputAction::FishingLog) => self.print_fishing_log(),
            Some(InputAction::MiningSell) => self.sell_mining_rubble(),
            Some(InputAction::MiningLog) => self.print_mining_log(),
            Some(InputAction::Order66) => self.toggle_order66(),
            Some(InputAction::ExecuteOrder66) => self.execute_order66(),
            Some(InputAction::Hax) => self.toggle_hax(),
            Some(InputAction::Ragdoll) => self.toggle_ragdoll(),
            Some(InputAction::Drone) => self.toggle_drone(),
            Some(InputAction::Boombox) => self.toggle_boombox(),
            Some(InputAction::PlaceBoombox) => self.place_boombox(),
            Some(InputAction::Monitor) => self.toggle_monitor(),
            Some(InputAction::StickTag) => self.stick_tag(),
            Some(InputAction::ClearTag) => self.clear_tag(),
            Some(InputAction::ThrowMonitor) => self.throw_monitor(),
            Some(InputAction::ChargeMonitor(pressed)) => self.charge_monitor(pressed),
            Some(InputAction::BoxHunt) => self.toggle_box_hunt(),
            Some(InputAction::Banana) => self.toggle_banana(),
            Some(InputAction::DropBanana) => self.drop_banana(),
            Some(InputAction::PropGun) => self.toggle_propgun(),
            Some(InputAction::FlipKick) => {
                let restricted = self.live_session.as_ref().is_some_and(|session| {
                    session
                        .game_state()
                        .config_string(0)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                        .and_then(|text| sjk_protocol::InfoString::parse(text).ok())
                        .and_then(|info| info.get_i32("restricts"))
                        .is_some_and(|bits| bits & flip_kick::RESTRICT_BIT != 0)
                });
                if !restricted {
                    self.gameplay_input.flip_kick.start();
                }
            }
            Some(InputAction::Weapon(weapon)) => {
                // A weapon chosen (the saber's 1 among them) puts the pickaxe away
                // (`mining`), and is taken out as ever.
                self.mining.weapon_chosen();
                self.select_weapon(weapon);
            }
            Some(InputAction::WeaponCycle(direction)) => {
                self.mining.weapon_chosen();
                self.cycle_weapon(direction);
            }
            Some(InputAction::WeaponMelee) => self.select_melee(),
            Some(InputAction::SelectionCycle(inventory, direction)) => {
                let snapshot = self
                    .live_session
                    .as_ref()
                    .map(|s| s.latest_snapshot())
                    .or_else(|| self.demo_session.as_ref().map(|s| s.latest_snapshot()));
                if let Some(snapshot) = snapshot {
                    let use_held = self.gameplay_input.held(GameButton::Button(5));
                    let time = if self.live_session.is_some() {
                        self.server_clock.server_time(std::time::Instant::now())
                    } else {
                        snapshot.server_time
                    };
                    self.gameplay_input.selection.cycle(
                        Some(&snapshot.player),
                        time,
                        inventory,
                        direction,
                        use_held,
                    );
                    // CG_Draw2D shows only the most recent selector.
                    self.weapon_selected_at = None;
                }
            }
            Some(InputAction::ForceSelect(slot)) => {
                if let Some(session) = &self.live_session {
                    let time = self.server_clock.server_time(std::time::Instant::now());
                    if self.gameplay_input.selection.select(
                        &session.latest_snapshot().player,
                        time,
                        slot,
                    ) {
                        // CG_Draw2D shows only the most recent selector.
                        self.weapon_selected_at = None;
                    }
                }
            }
            Some(InputAction::ForceAbility(slot)) => {
                if let Some(session) = &mut self.live_session {
                    let extensions = self.gameplay_input.selection.extensions();
                    let command = if extensions.iter().any(|a| a.slot == slot) {
                        let known = self
                            .gameplay_input
                            .selection
                            .known(&session.latest_snapshot().player);
                        sjk_client::force_wheel::valid_with(known, slot, extensions)
                            .then(|| force_abilities::command_for(slot))
                            .flatten()
                    } else {
                        // Not active here: forwarded unchanged, as before the mod owned it.
                        force_abilities::command_for(slot)
                    };
                    if let Some(command) = command
                        && let Err(error) = session.send_reliable_command_from(
                            command.as_bytes(),
                            sjk_client::ReliableCommandOrigin::Input,
                        )
                    {
                        eprintln!("failed to send {command}: {error}");
                    }
                }
            }
            Some(InputAction::RequestScores) => {
                crate::scoreboard::behavior::request_scores(self);
            }
            Some(InputAction::CameraControl)
                if self.live_session.is_some() || self.demo_session.is_some() =>
            {
                self.open_shot_panel();
            }
            Some(InputAction::TeamMenu) if self.live_session.is_some() => {
                self.release_pointer();
                self.game_menu = true;
                self.game_menu_page = crate::ingame_menu::Page::Team;
                self.game_menu_row = 0;
            }
            Some(InputAction::Vote(yes)) => {
                let command: &[u8] = if yes { b"vote yes" } else { b"vote no" };
                if let Some(session) = &mut self.live_session
                    && let Err(error) = session.send_reliable_command_from(
                        command,
                        sjk_client::ReliableCommandOrigin::Input,
                    )
                {
                    eprintln!("failed to cast vote: {error}");
                }
            }
            Some(
                InputAction::MessageMode(_) | InputAction::TeamMenu | InputAction::CameraControl,
            )
            | None => {}
        }
    }

    fn cycle_weapon(&mut self, direction: i8) {
        let current = self.selected_weapon.unwrap_or_else(|| {
            self.live_session
                .as_ref()
                .map_or(3, |session| session.latest_snapshot().player.weapon())
        });
        let Some(session) = &self.live_session else {
            return;
        };
        let inventory =
            sjk_client::LegacyWeaponInventory::from_player_state(&session.latest_snapshot().player);
        let selected = sjk_client::legacy_cycle_weapon(&inventory, current, direction);
        if selected != current {
            self.select_weapon_exact(selected);
        }
    }

    /// `weapmelee`: select the fists (`WP_MELEE`) by name, from the saber or
    /// any weapon. Unlike `weapon 1` it never picks the saber nor switches it on
    /// or off, and with the fists already selected, or none held, it does nothing
    /// ([`sjk_client::legacy_melee_weapon`]).
    fn select_melee(&mut self) {
        let Some(session) = &self.live_session else {
            return;
        };
        let player = &session.latest_snapshot().player;
        let current = self.selected_weapon.unwrap_or_else(|| player.weapon());
        let inventory = sjk_client::LegacyWeaponInventory::from_player_state(player);
        if let Some(weapon) = sjk_client::legacy_melee_weapon(&inventory, current) {
            self.select_weapon_exact(weapon);
        }
    }

    pub(super) fn select_weapon(&mut self, slot: u8) {
        let Some(session) = &self.live_session else {
            return;
        };
        // `CG_Weapon_f` uses slot 1 as the saber toggle when the saber is
        // already selected (`cg_weapons.c:1538-1545`), and a press while
        // `weaponTime` runs does nothing, rather than picking another weapon.
        // JoF EJK reads the predicted state there, not the snapshot (hub report
        // 172): a snapshot's `weaponTime` is a round trip old, still running (a
        // swing, the holster's 400 ms) when the server would already take it.
        let snapshot = &session.latest_snapshot().player;
        let (weapon, weapon_time) = self
            .local_prediction
            .predicted_state()
            .map_or((snapshot.weapon(), snapshot.weapon_time()), |state| {
                (state.weapon, state.weapon_time)
            });
        if slot == 1 && weapon == 3 {
            if weapon_time < 1 {
                self.pending_generic_command = GENCMD_SABER_SWITCH;
            }
            return;
        }
        let inventory =
            sjk_client::LegacyWeaponInventory::from_player_state(&session.latest_snapshot().player);
        if let Some(weapon) = sjk_client::legacy_direct_weapon(
            &inventory,
            session.latest_snapshot().player.weapon(),
            slot,
        ) {
            self.select_weapon_exact(weapon);
        }
    }

    pub(super) fn select_weapon_exact(&mut self, weapon: u8) {
        if let Some(session) = &self.live_session {
            let inventory = sjk_client::LegacyWeaponInventory::from_player_state(
                &session.latest_snapshot().player,
            );
            if !sjk_client::legacy_weapon_selectable(&inventory, weapon) {
                return;
            }
            self.selected_weapon = Some(weapon);
            self.weapon_selected_at = Some(std::time::Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTON_12: u16 = 1 << 12;

    fn buttons(input: &GameplayInput) -> u16 {
        input.user_command(0, 0.0, 0.0, [0; 3], 0, 0, 0).buttons
    }

    #[test]
    fn messagemode5_opens_the_sjk_chat() {
        let mut input = GameplayInput::default();
        assert!(GameplayInput::recognizes("messagemode5"));
        assert_eq!(
            input.apply("messagemode5"),
            Some(InputAction::SjkMessageMode)
        );
    }

    #[test]
    fn ping_binds_ping_on_the_press_and_stay_off_the_server() {
        let mut input = GameplayInput::default();
        for command in ["+ping", "+ping 147 1000", "worldping", "WORLDPING"] {
            assert!(GameplayInput::recognizes(command), "{command}");
            assert_eq!(
                input.apply(command),
                Some(InputAction::WorldPing),
                "{command}"
            );
        }
        // The key's release is the bind's own: kept, doing nothing.
        assert!(GameplayInput::recognizes("-ping 147 1100"));
        assert_eq!(input.apply("-ping 147 1100"), None);
        // `ping` is the engine's server query, not a world ping.
        assert!(!GameplayInput::recognizes("ping"));
    }

    #[test]
    fn toy_tag_sticks_and_toy_tag_clear_takes_it_off() {
        let mut input = GameplayInput::default();
        for (command, action) in [
            ("toy_tag", InputAction::StickTag),
            ("toy_tag clear", InputAction::ClearTag),
            ("toy_tag OFF", InputAction::ClearTag),
            ("toy_banana", InputAction::Banana),
            ("banana_drop", InputAction::DropBanana),
            ("toy_propgun", InputAction::PropGun),
            ("boombox_place", InputAction::PlaceBoombox),
        ] {
            assert!(GameplayInput::recognizes(command), "{command}");
            assert_eq!(input.apply(command), Some(action), "{command}");
        }
    }

    #[test]
    fn a_held_monitor_throw_charges_on_the_press_and_throws_on_the_release() {
        let mut input = GameplayInput::default();
        for (command, action) in [
            ("+monitor_throw 30 1000", InputAction::ChargeMonitor(true)),
            ("-monitor_throw 30 1900", InputAction::ChargeMonitor(false)),
            ("monitor_throw", InputAction::ThrowMonitor),
        ] {
            assert!(GameplayInput::recognizes(command), "{command}");
            assert_eq!(input.apply(command), Some(action), "{command}");
        }
    }

    #[test]
    fn toy_illuminate_toggles_the_holocron_and_the_old_name_still_does() {
        let mut input = GameplayInput::default();
        for command in ["toy_illuminate", "TOY_ILLUMINATE", "force_illuminate"] {
            assert!(GameplayInput::recognizes(command), "{command}");
            assert_eq!(input.apply(command), Some(InputAction::Illuminate));
        }
        // No other `toy_` or `force_` name is the toy.
        assert!(!GameplayInput::recognizes("toy_unknown"));
        assert_eq!(input.apply("toy_unknown"), None);
        assert_eq!(input.apply("force_illuminated"), None);
        // Window focus lost: a bind pressed then does nothing, as for every command.
        input.focus(false);
        assert_eq!(input.apply("toy_illuminate"), None);
    }

    #[test]
    fn weapmelee_is_a_local_weapon_command() {
        let mut input = GameplayInput::default();
        assert!(GameplayInput::recognizes("weapmelee"));
        assert!(GameplayInput::recognizes("WEAPMELEE"));
        assert_eq!(input.apply("weapmelee"), Some(InputAction::WeaponMelee));
        // The saber / melee bind is its own command and keeps its meaning.
        assert_eq!(input.apply("weapon 1"), Some(InputAction::Weapon(1)));
    }

    #[test]
    fn forceselect_names_a_wheel_entry() {
        let mut input = GameplayInput::default();
        assert!(GameplayInput::recognizes("forceselect 3"));
        assert_eq!(
            input.apply("forceselect 19"),
            Some(InputAction::ForceSelect(19))
        );
        assert_eq!(input.apply("forceselect"), None);
        assert_eq!(input.apply("forceselect push"), None);
    }

    #[test]
    fn typed_button_hold_survives_the_console_taking_the_keyboard() {
        let mut input = GameplayInput::default();
        assert!(GameplayInput::recognizes("+button12"));
        input.apply("+button12");
        // The console consumes the Enter release and clears key states.
        input.release_keys();
        assert_ne!(buttons(&input) & BUTTON_12, 0);
        input.apply("-button12");
        input.finish_command();
        assert_eq!(buttons(&input) & BUTTON_12, 0);
    }

    #[test]
    fn bound_button_is_released_with_the_keys() {
        let mut input = GameplayInput::default();
        input.apply("+button12 77 0");
        input.release_keys();
        input.finish_command();
        assert_eq!(buttons(&input) & BUTTON_12, 0);
    }

    #[test]
    fn grapple_holds_button_12_and_reports_its_release() {
        let mut input = GameplayInput::default();
        assert!(GameplayInput::recognizes("+grapple"));
        assert_eq!(input.apply("+grapple 77 0"), None);
        assert_ne!(buttons(&input) & BUTTON_12, 0);
        assert_eq!(
            input.apply("-grapple 77 10"),
            Some(InputAction::GrappleReleased)
        );
        input.finish_command();
        assert_eq!(buttons(&input) & BUTTON_12, 0);
    }

    /// One 8 ms command ending at `now`, returning its forward move.
    fn forward_move(input: &mut GameplayInput, now: u64) -> i8 {
        let look = crate::pointer_input::MouseLook {
            sensitivity: 5.0,
            yaw_scale: 0.022,
            pitch_scale: 0.022,
            invert: false,
        };
        input.sample_motion(now, look);
        let forward = input
            .user_command(0, 0.0, 0.0, [0; 3], 0, 0, 0)
            .forward_move;
        input.finish_command();
        forward
    }

    #[test]
    fn idrive_reverses_to_the_last_pressed_key_after_its_delay() {
        let mut input = GameplayInput::default();
        input.set_idrive(idrive::Idrive {
            mode: 1,
            delay_millis: 16,
        });
        forward_move(&mut input, 100);
        input.apply("+forward 17 100");
        assert_eq!(forward_move(&mut input, 108), 127);
        input.apply("+back 31 108");
        assert_eq!(forward_move(&mut input, 116), 0);
        assert_eq!(forward_move(&mut input, 124), -127);
        input.apply("-back 31 124");
        // Letting the newer key go keeps the pair neutral for the delay too.
        assert_eq!(forward_move(&mut input, 132), 0);
        assert_eq!(forward_move(&mut input, 140), 127);
    }

    /// Commands every `step` ms after `now` until one moves by `until`; the moves
    /// before it, and the time of the last command.
    fn moves_until(
        input: &mut GameplayInput,
        mut now: u64,
        step: u64,
        until: i8,
    ) -> (Vec<i8>, u64) {
        let mut before = Vec::new();
        loop {
            now += step;
            let moved = forward_move(input, now);
            if moved == until {
                return (before, now);
            }
            before.push(moved);
            assert!(before.len() < 100, "never reached {until}: {before:?}");
        }
    }

    #[test]
    fn idrive_keeps_its_neutral_gap_at_every_command_step() {
        // A command is made every `step` ms (125, 142, 250 and 333 FPS): the commands
        // strictly inside the delay after a press, or after the newer key's release,
        // are neutral, so neither reversal is ever instant.
        for step in [8_u64, 7, 4, 3] {
            for delay in [16_u64, 50] {
                let gap = (delay.div_ceil(step) - 1) as usize;
                let mut input = GameplayInput::default();
                input.set_idrive(idrive::Idrive {
                    mode: 1,
                    delay_millis: delay,
                });
                forward_move(&mut input, 1000);
                input.apply("+forward 17 1000");
                let (_, mut now) = moves_until(&mut input, 1000, step, 127);
                for _ in 0..10 {
                    now += step;
                    assert_eq!(forward_move(&mut input, now), 127);
                }
                input.apply(&format!("+back 31 {now}"));
                let (before, reached) = moves_until(&mut input, now, step, -127);
                assert_eq!(before, vec![0; gap], "press, step {step} delay {delay}");
                now = reached;
                for _ in 0..10 {
                    now += step;
                    assert_eq!(forward_move(&mut input, now), -127);
                }
                input.apply(&format!("-back 31 {now}"));
                let (before, _) = moves_until(&mut input, now, step, 127);
                assert_eq!(before, vec![0; gap], "release, step {step} delay {delay}");
            }
        }
    }

    #[test]
    fn idrive_counts_its_delay_in_command_time_not_frame_time() {
        // A 16 ms frame makes two commands, stamped 8 ms apart. The press was at
        // 2000: the first command (made at 2008) is inside the 16 ms delay, the second
        // (2016) is not. Evaluated at the frame's time both would be 2016.
        let mut input = GameplayInput::default();
        input.set_idrive(idrive::Idrive {
            mode: 1,
            delay_millis: 16,
        });
        let look = crate::pointer_input::MouseLook {
            sensitivity: 5.0,
            yaw_scale: 0.022,
            pitch_scale: 0.022,
            invert: false,
        };
        input.sample_motion(1000, look);
        input.apply("+forward 17 1000");
        input.sample_motion(2000, look);
        input.finish_command();
        input.apply("+back 31 2000");
        input.sample_motion(2016, look);
        let mut moves = Vec::new();
        for age in [8, 0] {
            input.set_command_age(age);
            moves.push(
                input
                    .user_command(0, 0.0, 0.0, [0; 3], 0, 0, 0)
                    .forward_move,
            );
        }
        input.set_command_age(0);
        assert_eq!(moves, [0, -127]);
    }

    #[test]
    fn tap_sets_a_button_for_one_command() {
        let mut input = GameplayInput::default();
        input.tap(GameButton::Button(5));
        assert_ne!(buttons(&input) & (1 << 5), 0);
        input.finish_command();
        assert_eq!(buttons(&input) & (1 << 5), 0);
    }

    #[test]
    fn lowjump_jumps_for_one_command_at_every_command_step() {
        for step in [8, 7, 4, 3] {
            let mut input = GameplayInput::default();
            step_command(&mut input, 1000);
            assert!(GameplayInput::recognizes("lowjump"));
            assert_eq!(input.apply("lowjump"), None);
            assert_eq!(step_command(&mut input, 1000 + step).2, 127, "step {step}");
            assert_eq!(
                step_command(&mut input, 1000 + 2 * step).2,
                0,
                "step {step}"
            );
        }
    }

    #[test]
    fn lowjump_leaves_a_held_jump_key_held() {
        let mut input = GameplayInput::default();
        step_command(&mut input, 1000);
        input.apply("+moveup 57 1000");
        input.apply("lowjump");
        assert_eq!(step_command(&mut input, 1008).2, 127);
        assert_eq!(step_command(&mut input, 1016).2, 127, "the key still holds");
    }

    #[test]
    fn console_move_keeps_movement_keys_only() {
        let mut input = GameplayInput::default();
        step_command(&mut input, 1000);
        input.apply("+forward 17 1000");
        input.apply("+moveleft 30 1000");
        input.apply("+attack 200 1000");
        input.release_keys_but_movement();
        let held = step_command(&mut input, 1008);
        assert_eq!(held.1, 127, "still running forward");
        assert_eq!(buttons(&input) & 1, 0, "attack let go");
        assert!(input.held(GameButton::MoveLeft));
        // Their own release still stops them.
        input.apply("-forward 17 1008");
        assert_eq!(step_command(&mut input, 1016).1, 0);
        input.release_keys();
        assert!(!input.held(GameButton::MoveLeft));
    }

    const BUTTON_WALKING: u16 = 1 << 4;

    fn command(input: &GameplayInput) -> UserCommand {
        input.user_command(0, 0.0, 0.0, [0; 3], 0, 0, 0)
    }

    #[test]
    fn duck_walks_one_command_then_crouches() {
        let mut input = GameplayInput::default();
        assert!(GameplayInput::recognizes("+duck"));
        input.apply("+forward 17 0");
        input.apply("+duck 46 0");
        // The walk that takes the legs out of their running animation, so no roll.
        let first = command(&input);
        assert_ne!(first.buttons & BUTTON_WALKING, 0);
        assert_eq!((first.forward_move, first.up_move), (46, 0));
        input.finish_command();
        let second = command(&input);
        assert_eq!(second.buttons & BUTTON_WALKING, 0);
        assert_eq!((second.forward_move, second.up_move), (127, -127));
        input.apply("-duck 46 10");
        input.finish_command();
        assert_eq!(command(&input).up_move, 0);
    }

    #[test]
    fn duck_lets_go_of_jump_and_walks_under_cl_run_0() {
        let mut input = GameplayInput::default();
        input.set_always_run(false);
        input.apply("+speed 42 0");
        input.apply("+moveup 57 0");
        input.apply("+duck 46 0");
        assert!(!input.held(GameButton::Up));
        let first = command(&input);
        assert_ne!(first.buttons & BUTTON_WALKING, 0, "EJK's +speed would run");
        assert_eq!(first.up_move, 0);
        input.finish_command();
        assert_eq!(command(&input).up_move, -127);
    }

    #[test]
    fn duck_and_crouch_share_the_crouch() {
        let mut input = GameplayInput::default();
        input.apply("+movedown 99 0");
        input.apply("+duck 46 0");
        assert_eq!(
            command(&input).up_move,
            -46,
            "still crouched, at walking speed"
        );
        input.finish_command();
        input.apply("-duck 46 10");
        assert_eq!(command(&input).up_move, -127);
    }

    /// One command ending at `now`, returning (walking, forward move, up move).
    fn step_command(input: &mut GameplayInput, now: u64) -> (bool, i8, i8) {
        let look = crate::pointer_input::MouseLook {
            sensitivity: 5.0,
            yaw_scale: 0.022,
            pitch_scale: 0.022,
            invert: false,
        };
        input.sample_motion(now, look);
        let c = command(input);
        input.finish_command();
        (c.buttons & BUTTON_WALKING != 0, c.forward_move, c.up_move)
    }

    #[test]
    fn cl_idrive_still_resolves_jump_and_crouch() {
        // With cl_idrive 1 or 2 a crouch pressed after a held jump wins.
        for mode in [1, 2] {
            let mut input = GameplayInput::default();
            input.set_idrive(idrive::Idrive {
                mode,
                delay_millis: 0,
            });
            step_command(&mut input, 1000);
            input.apply("+moveup 57 1000");
            assert_eq!(step_command(&mut input, 1008).2, 127);
            input.apply("+movedown 99 1008");
            assert_eq!(
                step_command(&mut input, 1016).2,
                -127,
                "mode {mode}: crouch pressed last wins"
            );
        }
    }

    #[test]
    fn duck_walks_one_command_then_crouches_at_every_command_step() {
        // 125, 142, 250 and 333 FPS command steps, with the press anywhere in a step.
        for step in [8_u64, 7, 4, 3] {
            for offset in [0_u64, 1, step - 1] {
                let mut input = GameplayInput::default();
                let mut now = 1000;
                step_command(&mut input, now);
                input.apply(&format!("+forward 17 {now}"));
                for _ in 0..5 {
                    now += step;
                    assert_eq!(step_command(&mut input, now), (false, 127, 0));
                }
                input.apply(&format!("+duck 46 {}", now + offset));
                now += step;
                assert_eq!(
                    step_command(&mut input, now),
                    (true, 46, 0),
                    "step {step} offset {offset}"
                );
                for _ in 0..10 {
                    now += step;
                    assert_eq!(
                        step_command(&mut input, now),
                        (false, 127, -127),
                        "step {step} offset {offset}"
                    );
                }
                input.apply(&format!("-duck 46 {}", now + offset));
                now += step;
                let released = step_command(&mut input, now);
                assert!(released.2 <= 0 && !released.0, "step {step}: {released:?}");
                now += step;
                assert_eq!(
                    step_command(&mut input, now),
                    (false, 127, 0),
                    "step {step} offset {offset}"
                );
            }
        }
    }

    #[test]
    fn a_jump_pressed_while_ducking_wins_under_cl_idrive() {
        // EJK: +duck's typed +movedown carries no press time, so a jump pressed while
        // ducking wins under cl_idrive; without it the two cancel.
        for (mode, expected) in [(0, 0), (1, 127), (2, 127)] {
            let mut input = GameplayInput::default();
            input.set_idrive(idrive::Idrive {
                mode,
                delay_millis: 0,
            });
            step_command(&mut input, 1000);
            input.apply("+duck 46 1000");
            step_command(&mut input, 1008);
            assert_eq!(step_command(&mut input, 1016).2, -127);
            input.apply("+moveup 57 1016");
            assert_eq!(step_command(&mut input, 1024).2, expected, "mode {mode}");
        }
    }

    fn look() -> crate::pointer_input::MouseLook {
        crate::pointer_input::MouseLook {
            sensitivity: 5.0,
            yaw_scale: 0.022,
            pitch_scale: 0.022,
            invert: false,
        }
    }

    /// A wheel notch: the bind's `+` and `-` with one key id and timestamp, as
    /// `route_gameplay_wheel` queues them.
    fn wheel_notch(input: &mut GameplayInput, bind: &str, time: u64) {
        input.apply(&format!("+{bind} 184 {time}"));
        input.apply(&format!("-{bind} 184 {time}"));
    }

    #[test]
    fn a_wheel_notch_on_moveup_jumps_for_one_command_at_every_command_step() {
        for step in [8, 7, 4, 3] {
            let mut input = GameplayInput::default();
            step_command(&mut input, 1000);
            // Between two frames, and sampled in the frame it arrived in.
            wheel_notch(&mut input, "moveup", 1000 + step - 1);
            assert_eq!(step_command(&mut input, 1000 + step).2, 127, "step {step}");
            assert_eq!(
                step_command(&mut input, 1000 + 2 * step).2,
                0,
                "step {step}"
            );
            input.sample_motion(1000 + 3 * step, look());
            wheel_notch(&mut input, "moveup", 1000 + 3 * step);
            assert_eq!(command(&input).up_move, 127, "step {step}");
        }
    }

    #[test]
    fn a_wheel_notch_lasts_until_a_command_is_made() {
        // Frames faster than the command step keep the notch, as EJK's wasPressed.
        let mut input = GameplayInput::default();
        let look = look();
        input.sample_motion(1000, look);
        input.finish_command();
        wheel_notch(&mut input, "moveup", 1001);
        input.sample_motion(1002, look);
        input.sample_motion(1004, look);
        input.sample_motion(1008, look);
        assert_eq!(command(&input).up_move, 127);
    }

    #[test]
    fn a_wheel_notch_presses_buttons_and_moves() {
        let mut input = GameplayInput::default();
        step_command(&mut input, 1000);
        wheel_notch(&mut input, "attack", 1004);
        wheel_notch(&mut input, "use", 1004);
        wheel_notch(&mut input, "forward", 1004);
        input.sample_motion(1008, look());
        let c = command(&input);
        assert_eq!(c.buttons & 0b10_0001, 0b10_0001);
        assert_eq!(c.forward_move, 127);
        input.finish_command();
        let c = command(&input);
        assert_eq!((c.buttons & 0b10_0001, c.forward_move), (0, 0));
    }

    #[test]
    fn a_short_key_tap_still_moves_for_its_time() {
        // A press with time down keeps stock's share of the command: 2 of 8 ms.
        let mut input = GameplayInput::default();
        step_command(&mut input, 1000);
        input.apply("+moveup 57 1002");
        input.apply("-moveup 57 1004");
        assert_eq!(step_command(&mut input, 1008).2, 31);
    }
}
