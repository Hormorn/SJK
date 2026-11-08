//! What the WHEEL OF DOOM does to its spinner (`docs/wheel-of-doom.md`): one effect at a
//! time on the player's own input and view, for its seconds, and the curse's pending
//! effect. Nothing here reaches the game server but the user commands the player sends
//! anyway: their moves, buttons and angles turned about, the saber's own toggle
//! (`sv_saberswitch`'s `generic_cmd`) to keep it off. Everything is a few numbers a
//! frame, no allocation.

use sjk_protocol::UserCommand;
use std::time::{Duration, Instant};

/// `BUTTON_ATTACK`, `BUTTON_WALKING` and `BUTTON_ALT_ATTACK` (`q_shared.h`).
const BUTTON_ATTACK: u16 = 1;
const BUTTON_WALKING: u16 = 16;
const BUTTON_ALT_ATTACK: u16 = 128;
/// `WP_SABER`.
const WP_SABER: u8 = 3;
/// The walk's `movespeed` (`CL_KeyMove`'s 46 with `BUTTON_WALKING`, as SJK's input).
const WALK_SPEED: i8 = 46;
/// How often the saber's toggle is sent while the saber is lit under No saber.
const SWITCH_EVERY: Duration = Duration::from_millis(750);
/// How often an attack held keeps quacking.
const QUACK_EVERY: Duration = Duration::from_millis(450);
/// The fields of view the two FOV effects force, horizontal degrees.
const TINY_FOV: f32 = 40.0;
const HUGE_FOV: f32 = 150.0;
/// How long the blind fades to black, and back at its end.
const BLIND_IN: f32 = 1.2;
const BLIND_OUT: f32 = 1.0;
/// How long the drunk sway takes to come and go.
const DRUNK_EASE: f32 = 1.5;
/// The longest a curse waits before it strikes.
pub(super) const CURSE_WITHIN: Duration = Duration::from_secs(60);

/// One bad thing the wheel may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    ReverseKeys,
    InvertMouse,
    Mirror,
    UpsideDown,
    Blind,
    Drunk,
    TinyFov,
    HugeFov,
    NoSaber,
    Quack,
    Walk,
    Bunny,
}

impl Effect {
    /// Every effect, the curse's pick among them.
    pub(crate) const ALL: [Self; 12] = [
        Self::ReverseKeys,
        Self::InvertMouse,
        Self::Mirror,
        Self::UpsideDown,
        Self::Blind,
        Self::Drunk,
        Self::TinyFov,
        Self::HugeFov,
        Self::NoSaber,
        Self::Quack,
        Self::Walk,
        Self::Bunny,
    ];

    /// How long it lasts.
    pub(crate) const fn lasts(self) -> Duration {
        Duration::from_secs(match self {
            Self::UpsideDown | Self::TinyFov | Self::HugeFov | Self::NoSaber | Self::Bunny => 20,
            Self::Blind => 15,
            _ => 30,
        })
    }

    /// Its name on the HUD's line.
    pub(crate) const fn hud(self) -> &'static str {
        match self {
            Self::ReverseKeys => "KEYS REVERSED",
            Self::InvertMouse => "MOUSE INVERTED",
            Self::Mirror => "MIRRORED",
            Self::UpsideDown => "UPSIDE DOWN",
            Self::Blind => "BLIND",
            Self::Drunk => "DRUNK",
            Self::TinyFov => "TUNNEL VISION",
            Self::HugeFov => "FISHEYE",
            Self::NoSaber => "NO SABER",
            Self::Quack => "QUACK",
            Self::Walk => "WALK ONLY",
            Self::Bunny => "BUNNY HOP",
        }
    }
}

/// The saber as the player state says: held, and lit (`saberHolstered` 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Saber {
    pub(crate) weapon: u8,
    pub(crate) holstered: u8,
}

impl Saber {
    fn held(self) -> bool {
        self.weapon == WP_SABER
    }
}

/// The spinner's effect and curse.
#[derive(Clone, Debug, Default)]
pub(crate) struct Effects {
    /// The effect running and when it started.
    active: Option<(Effect, Instant)>,
    /// A curse's effect and when it strikes.
    curse: Option<(Instant, Effect)>,
    /// The attack held in the last command, for the quacks.
    attack_held: bool,
    last_quack: Option<Instant>,
    /// Quacks to play.
    quacks: u8,
    last_switch: Option<Instant>,
    /// The drunk drift's yaw offset last frame, degrees.
    drift: f32,
}

impl Effects {
    /// Start `effect` at `now`, in place of any other.
    pub(crate) fn start(&mut self, effect: Effect, now: Instant) {
        self.active = Some((effect, now));
        self.drift = 0.0;
    }

    /// Curse the player at `now`: one of [`Effect::ALL`] picked by `seed` strikes at a
    /// moment within [`CURSE_WITHIN`], also picked by `seed`. Returns the effect.
    pub(crate) fn curse(&mut self, now: Instant, seed: u32) -> Effect {
        let effect = Effect::ALL[seed as usize % Effect::ALL.len()];
        let mixed = seed.wrapping_mul(0x9E37_79B9) >> 8;
        let after = Duration::from_millis(u64::from(mixed) % (CURSE_WITHIN.as_millis() as u64));
        self.curse = Some((now + after, effect));
        effect
    }

    /// Move on to `now`: an effect past its time ends, and a curse whose moment came
    /// starts its effect, which is returned (to be told).
    pub(crate) fn tick(&mut self, now: Instant) -> Option<Effect> {
        if let Some((effect, started)) = self.active
            && now.saturating_duration_since(started) >= effect.lasts()
        {
            self.active = None;
        }
        let (at, effect) = self.curse?;
        if now < at {
            return None;
        }
        self.curse = None;
        self.start(effect, now);
        Some(effect)
    }

    /// Everything ends: a map change, a disconnect, a death.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// The effect running at `now` and its whole seconds left (rounded up).
    pub(crate) fn current(&self, now: Instant) -> Option<(Effect, u64)> {
        let (effect, started) = self.active?;
        let left = effect
            .lasts()
            .checked_sub(now.saturating_duration_since(started))?;
        (!left.is_zero()).then(|| (effect, left.as_secs_f32().ceil() as u64))
    }

    /// A curse waits to strike.
    pub(crate) fn cursed(&self) -> bool {
        self.curse.is_some()
    }

    /// Whether anything runs or waits.
    pub(crate) fn any(&self) -> bool {
        self.active.is_some() || self.curse.is_some()
    }

    fn running(&self, now: Instant) -> Option<(Effect, f32, f32)> {
        let (effect, started) = self.active?;
        let since = now.saturating_duration_since(started).as_secs_f32();
        let left = effect.lasts().as_secs_f32() - since;
        (left > 0.0).then_some((effect, since, left))
    }

    fn is(&self, effect: Effect, now: Instant) -> bool {
        self.running(now)
            .is_some_and(|(running, ..)| running == effect)
    }

    /// Turn the user command about for the effect running: the moves reversed or
    /// mirrored, the walk, the jump held, the saber kept off (and its attacks, which
    /// would light it, dropped); notes the attacks for the quacks.
    pub(crate) fn shape_command(&mut self, command: &mut UserCommand, saber: Saber, now: Instant) {
        let Some((effect, ..)) = self.running(now) else {
            self.attack_held = command.buttons & BUTTON_ATTACK != 0;
            return;
        };
        match effect {
            Effect::ReverseKeys => {
                command.forward_move = command.forward_move.saturating_neg();
                command.right_move = command.right_move.saturating_neg();
            }
            Effect::Mirror => command.right_move = command.right_move.saturating_neg(),
            Effect::Walk => {
                command.buttons |= BUTTON_WALKING;
                command.forward_move = command.forward_move.clamp(-WALK_SPEED, WALK_SPEED);
                command.right_move = command.right_move.clamp(-WALK_SPEED, WALK_SPEED);
            }
            Effect::Bunny => command.up_move = 127,
            Effect::NoSaber if saber.held() => {
                // `sv_saberswitch` would light it: only the wheel's own toggle goes.
                if command.generic_command == crate::input::generic_commands::GENCMD_SABER_SWITCH {
                    command.generic_command = 0;
                }
                if saber.holstered == 0 {
                    if self
                        .last_switch
                        .is_none_or(|at| now.saturating_duration_since(at) >= SWITCH_EVERY)
                    {
                        command.generic_command =
                            crate::input::generic_commands::GENCMD_SABER_SWITCH;
                        self.last_switch = Some(now);
                    }
                } else {
                    // An attack lights a holstered saber (`PM_Weapon`).
                    command.buttons &= !(BUTTON_ATTACK | BUTTON_ALT_ATTACK);
                }
            }
            Effect::Quack => {
                let attacking = command.buttons & (BUTTON_ATTACK | BUTTON_ALT_ATTACK) != 0;
                let due = attacking
                    && (!self.attack_held
                        || self
                            .last_quack
                            .is_none_or(|at| now.saturating_duration_since(at) >= QUACK_EVERY));
                if due {
                    self.quacks = self.quacks.saturating_add(1).min(2);
                    self.last_quack = Some(now);
                }
            }
            _ => {}
        }
        self.attack_held = command.buttons & (BUTTON_ATTACK | BUTTON_ALT_ATTACK) != 0;
    }

    /// A quack to play now, once each.
    pub(crate) fn take_quack(&mut self) -> bool {
        let quack = self.quacks > 0;
        self.quacks = self.quacks.saturating_sub(1);
        quack
    }

    /// Turn this frame's mouse look (`[yaw, pitch]`, radians) about: inverted, mirrored,
    /// or drifting while drunk.
    pub(crate) fn shape_look(&mut self, delta: [f32; 2], now: Instant) -> [f32; 2] {
        let Some((effect, since, left)) = self.running(now) else {
            return delta;
        };
        match effect {
            Effect::InvertMouse => [-delta[0], -delta[1]],
            Effect::Mirror => [-delta[0], delta[1]],
            Effect::Drunk => {
                let drift = 7.0 * drunk_ease(since, left) * (since * 0.37).sin();
                let step = drift - self.drift;
                self.drift = drift;
                [delta[0] + step.to_radians(), delta[1]]
            }
            _ => delta,
        }
    }

    /// The field of view forced at `now`, horizontal degrees.
    pub(crate) fn fov(&self, now: Instant) -> Option<f32> {
        match self.running(now)?.0 {
            Effect::TinyFov => Some(TINY_FOV),
            Effect::HugeFov => Some(HUGE_FOV),
            _ => None,
        }
    }

    /// How the view is turned at `now`: its roll (degrees, clockwise as seen), and a
    /// sway of its pitch and yaw (degrees) that the aim does not follow.
    pub(crate) fn view_turn(&self, now: Instant) -> Option<[f32; 3]> {
        let (effect, since, left) = self.running(now)?;
        match effect {
            Effect::UpsideDown => Some([180.0, 0.0, 0.0]),
            Effect::Drunk => {
                let ease = drunk_ease(since, left);
                Some([
                    9.0 * ease * (since * 0.9).sin(),
                    3.0 * ease * (since * 1.3).sin(),
                    4.0 * ease * (since * 0.7 + 1.0).sin(),
                ])
            }
            _ => None,
        }
    }

    /// How dark the blind is at `now`, 0 to 1: it fades in, holds and fades out, never
    /// flashing.
    pub(crate) fn blind(&self, now: Instant) -> Option<f32> {
        let (effect, since, left) = self.running(now)?;
        (effect == Effect::Blind).then(|| (since / BLIND_IN).min(left / BLIND_OUT).clamp(0.0, 1.0))
    }

    /// The screen is mirrored at `now`.
    pub(crate) fn mirrored(&self, now: Instant) -> bool {
        self.is(Effect::Mirror, now)
    }
}

fn drunk_ease(since: f32, left: f32) -> f32 {
    (since / DRUNK_EASE).min(left / DRUNK_EASE).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(forward: i8, right: i8, buttons: u16) -> UserCommand {
        UserCommand {
            forward_move: forward,
            right_move: right,
            buttons,
            ..UserCommand::default()
        }
    }

    fn running(effect: Effect) -> (Effects, Instant) {
        let now = Instant::now();
        let mut effects = Effects::default();
        effects.start(effect, now);
        (effects, now + Duration::from_secs(1))
    }

    #[test]
    fn reversed_keys_swap_forward_back_and_left_right() {
        let (mut effects, now) = running(Effect::ReverseKeys);
        let mut moved = command(127, -127, 0);
        effects.shape_command(&mut moved, Saber::default(), now);
        assert_eq!((moved.forward_move, moved.right_move), (-127, 127));
        let mut most = command(-128, 0, 0);
        effects.shape_command(&mut most, Saber::default(), now);
        assert_eq!(most.forward_move, 127);
    }

    #[test]
    fn the_inverted_mouse_turns_both_ways_round() {
        let (mut effects, now) = running(Effect::InvertMouse);
        assert_eq!(effects.shape_look([0.1, -0.2], now), [-0.1, 0.2]);
    }

    #[test]
    fn the_mirror_swaps_strafe_and_yaw_and_flips_the_screen() {
        let (mut effects, now) = running(Effect::Mirror);
        let mut moved = command(127, 127, 0);
        effects.shape_command(&mut moved, Saber::default(), now);
        assert_eq!((moved.forward_move, moved.right_move), (127, -127));
        assert_eq!(effects.shape_look([0.1, 0.2], now), [-0.1, 0.2]);
        assert!(effects.mirrored(now));
    }

    #[test]
    fn walk_only_walks_and_bunny_holds_the_jump() {
        let (mut effects, now) = running(Effect::Walk);
        let mut moved = command(127, -127, 0);
        effects.shape_command(&mut moved, Saber::default(), now);
        assert_eq!((moved.forward_move, moved.right_move), (46, -46));
        assert_ne!(moved.buttons & BUTTON_WALKING, 0);
        let (mut effects, now) = running(Effect::Bunny);
        let mut still = command(0, 0, 0);
        effects.shape_command(&mut still, Saber::default(), now);
        assert_eq!(still.up_move, 127);
    }

    #[test]
    fn no_saber_puts_it_out_and_keeps_it_out() {
        let (mut effects, now) = running(Effect::NoSaber);
        let lit = Saber {
            weapon: WP_SABER,
            holstered: 0,
        };
        let mut first = command(0, 0, 0);
        effects.shape_command(&mut first, lit, now);
        assert_eq!(
            first.generic_command,
            crate::input::generic_commands::GENCMD_SABER_SWITCH
        );
        // Not again at once: the server takes a moment.
        let mut next = command(0, 0, 0);
        effects.shape_command(&mut next, lit, now + Duration::from_millis(10));
        assert_eq!(next.generic_command, 0);
        let out = Saber {
            weapon: WP_SABER,
            holstered: 2,
        };
        let mut attack = command(0, 0, BUTTON_ATTACK | BUTTON_ALT_ATTACK);
        attack.generic_command = crate::input::generic_commands::GENCMD_SABER_SWITCH;
        effects.shape_command(&mut attack, out, now);
        assert_eq!((attack.buttons, attack.generic_command), (0, 0));
    }

    #[test]
    fn quack_quacks_on_each_attack() {
        let (mut effects, now) = running(Effect::Quack);
        effects.shape_command(&mut command(0, 0, BUTTON_ATTACK), Saber::default(), now);
        assert!(effects.take_quack());
        assert!(!effects.take_quack());
        // Held: again after a while, not every frame.
        effects.shape_command(
            &mut command(0, 0, BUTTON_ATTACK),
            Saber::default(),
            now + Duration::from_millis(10),
        );
        assert!(!effects.take_quack());
        effects.shape_command(
            &mut command(0, 0, BUTTON_ATTACK),
            Saber::default(),
            now + QUACK_EVERY,
        );
        assert!(effects.take_quack());
    }

    #[test]
    fn the_view_effects_turn_the_view_and_force_the_fov() {
        let (effects, now) = running(Effect::UpsideDown);
        assert_eq!(effects.view_turn(now), Some([180.0, 0.0, 0.0]));
        let (effects, now) = running(Effect::TinyFov);
        assert_eq!(effects.fov(now), Some(TINY_FOV));
        let (effects, now) = running(Effect::HugeFov);
        assert_eq!(effects.fov(now), Some(HUGE_FOV));
        let (effects, start) = running(Effect::Blind);
        let start = start - Duration::from_secs(1);
        assert_eq!(effects.blind(start), Some(0.0));
        assert_eq!(effects.blind(start + Duration::from_secs(5)), Some(1.0));
        assert!(
            effects
                .blind(start + Duration::from_millis(14_500))
                .unwrap()
                < 1.0
        );
    }

    #[test]
    fn an_effect_ends_on_its_timer_or_cleared() {
        let start = Instant::now();
        let mut effects = Effects::default();
        effects.start(Effect::Walk, start);
        assert_eq!(effects.current(start), Some((Effect::Walk, 30)));
        assert_eq!(
            effects.current(start + Duration::from_millis(29_100)),
            Some((Effect::Walk, 1))
        );
        effects.tick(start + Effect::Walk.lasts());
        assert_eq!(effects.current(start + Effect::Walk.lasts()), None);
        assert!(!effects.any());
        // A map change, a disconnect or a death clears it, and a curse waiting.
        effects.start(Effect::Bunny, start);
        effects.curse(start, 3);
        effects.clear();
        assert!(!effects.any());
        let mut moved = command(127, 0, 0);
        effects.shape_command(&mut moved, Saber::default(), start);
        assert_eq!((moved.forward_move, moved.up_move), (127, 0));
    }

    #[test]
    fn a_curse_strikes_with_a_bad_effect_within_a_minute() {
        let start = Instant::now();
        for seed in 0..200_u32 {
            let mut effects = Effects::default();
            let effect = effects.curse(start, seed.wrapping_mul(2_654_435_761));
            assert!(Effect::ALL.contains(&effect));
            assert!(effects.cursed() && effects.current(start).is_none());
            assert_eq!(effects.tick(start + CURSE_WITHIN), Some(effect));
            assert_eq!(
                effects.current(start + CURSE_WITHIN).map(|(e, _)| e),
                Some(effect)
            );
            assert!(!effects.cursed());
        }
    }
}
