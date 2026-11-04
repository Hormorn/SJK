//! The Collection's Saber Lab tab (`docs/client.md`, "Saber Lab"), there while the lab is
//! on ([`crate::saber_lab`]): on the menu map's stage the player's model stands left of the
//! picture ([`crate::menu_backdrop::Shot::SaberLab`]) holding the lab's blade on the
//! player's own hilts, standing or swinging (the game's own starts, attacks and returns,
//! [`crate::player_menu::force_pose::lab_swings`]) as fast as the tab says, lit or off, its
//! sounds playing or muted; the camera turns round it and tilts over it with a drag on the
//! picture. The same saber floats on the right, turned by a drag as on Shaders, a still
//! trail behind it when ticked; under it the choices and what the lab last read. The
//! blade shown is the one worn, whatever the Shaders tab equipped: the lab's own shows
//! once its Equip wears it. Over a match the stage is not there, and the
//! tab says so.

use super::view::{RIGHT_X, left_lines};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::player_menu::SaberStyle;
use sjk_ui::{FontWeight, TextAlign};

/// Where the floating saber lies (frame pixels): the right of the picture, over the
/// choices, the model being on the left.
pub(crate) const LAB_SABER_AREA: [f32; 4] = [1_180.0, 300.0, RIGHT_X - 1_180.0, 220.0];
/// The floating saber's trail tick, over its area.
const TRAIL_TICK: [f32; 2] = [1_200.0, 278.0];
/// Where a drag turns the camera round the model: the picture left of the choices and
/// under the tabs.
const VIEW_AREA: [f32; 4] = [0.0, 300.0, 1_180.0, 700.0];
/// The column of choices under it.
const COLUMN_X: f32 = 1_240.0;
const COLUMN_WIDTH: f32 = RIGHT_X - COLUMN_X;
const FIRST_ROW: f32 = 562.0;
const ROW_STEP: f32 = 50.0;
/// The speed slider's track (frame pixels), the number right of it.
const SLIDER_X: f32 = RIGHT_X - 380.0;
const SLIDER_WIDTH: f32 = 290.0;
/// How far either side of the track a press still takes the knob.
const SLIDER_GRAB: f32 = 12.0;
/// The choices' tokens: the styles, the moves, the blade's arrows and Equip, the speed,
/// the blade on or off, the sound on or muted, the picture's drag. The lab's tokens (this
/// tab's, its builder's and its editor's) keep to 3_000-3_499, which the rest of the
/// Collection leaves free: the lab tab's clicks are read before the other tabs'.
const STYLE_BASE: u16 = 3_000;
const MOVES_BASE: u16 = 3_004;
const BLADE_PREVIOUS: u16 = 3_008;
const BLADE_NEXT: u16 = 3_009;
const LAB_EQUIP: u16 = 3_010;
const SPEED_TOKEN: u16 = 3_011;
const LIT_BASE: u16 = 3_012;
const SOUND_BASE: u16 = 3_014;
pub(super) const VIEW_TOKEN: u16 = 3_016;
const FLOURISH_TOKEN: u16 = 3_017;
const CAMERA_BASE: u16 = 3_018;
const TRAIL_TOKEN: u16 = 3_020;
const CLASH_TOKEN: u16 = 3_028;
/// The styles, in [`LabStage::style`]'s order, the moves, the blade's states and the
/// sound's.
const STYLES: [&str; 3] = ["Fast", "Medium", "Strong"];
const MOVES: [&str; 2] = ["Stand", "Swing"];
const LIT: [&str; 2] = ["Off", "On"];
const SOUND: [&str; 2] = ["On", "Muted"];
const CAMERA: [&str; 2] = ["Stage", "Third person"];
/// The swing speeds the slider spans (1: the game's), and a key's step.
const SLOWEST: f32 = 0.1;
const FASTEST: f32 = 2.0;
const SPEED_STEP: f32 = 0.1;
/// Radians a frame pixel of drag turns or tilts the camera.
const VIEW_PER_PIXEL: f32 = 0.006;
/// How far the camera tilts: a little under the shot's own height, nearly over the head.
const LOWEST_TILT: f32 = -0.2;
const HIGHEST_TILT: f32 = 1.25;
/// How near and far the wheel takes the camera (a share of the shot's own distance), and
/// a notch's step.
const NEAREST: f32 = 0.45;
const FARTHEST: f32 = 2.2;
const NOTCH: f32 = 0.9;

/// What the Saber Lab tab asks of the stage: the style the model holds a single saber in
/// (0 fast, 1 medium, 2 strong), whether it swings and how fast, whether the blade is lit
/// and with the game's flourish, whether its sounds play, and where the camera looks
/// from: the tab's shot or behind the model as in a match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LabStage {
    pub(crate) style: usize,
    pub(crate) swinging: bool,
    /// How fast the model moves, 1 the game's speed.
    pub(crate) speed: f32,
    pub(crate) lit: bool,
    /// Lighting and putting out the blade plays the game's draw and put-away moves.
    pub(crate) flourish: bool,
    pub(crate) muted: bool,
    /// The camera behind the model, as a match's third person has it.
    pub(crate) chase: bool,
    /// The floating saber shows a trail behind it, still, as a swing leaves one.
    pub(crate) trail: bool,
    pub(crate) view: LabView,
    /// How many clashes were asked for (the Clash button): each new one strikes the
    /// blades as a block or a wall would.
    pub(crate) clash: u32,
    /// The builder shows: its panels take the left, the model stands on the right.
    pub(crate) build: bool,
}

impl Default for LabStage {
    fn default() -> Self {
        Self {
            style: 1,
            swinging: true,
            speed: 1.0,
            lit: true,
            flourish: true,
            muted: false,
            chase: false,
            trail: false,
            view: LabView::default(),
            clash: 0,
            build: false,
        }
    }
}

/// The camera round the model, from the tab's shot: turned round it (radians, the drag
/// to the right turning it left), raised over it (radians, up positive) and its distance
/// (a share of the shot's own).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LabView {
    pub(crate) turn: f32,
    pub(crate) tilt: f32,
    pub(crate) distance: f32,
}

impl Default for LabView {
    fn default() -> Self {
        Self {
            turn: 0.0,
            tilt: 0.0,
            distance: 1.0,
        }
    }
}

impl LabView {
    /// Turn and tilt by a drag of `by` frame pixels.
    fn drag(&mut self, by: [f32; 2]) {
        self.turn = (self.turn - by[0] * VIEW_PER_PIXEL).rem_euclid(std::f32::consts::TAU);
        self.tilt = (self.tilt + by[1] * VIEW_PER_PIXEL).clamp(LOWEST_TILT, HIGHEST_TILT);
    }

    /// Come nearer for a notch up the wheel, back for one down.
    fn wheel(&mut self, notches: f32) {
        self.distance = (self.distance * NOTCH.powf(notches.signum())).clamp(NEAREST, FARTHEST);
    }
}

/// A choice of the Blade row: what the player wears, the stock blade, or a skin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum LabBlade {
    Worn,
    Stock,
    Skin(&'static str),
}

/// The Blade row's choices: what the player wears, the stock blade, the lab folder's
/// skins (its blade first, [`crate::saber_lab::blades`]), then every other blade skin of
/// the catalogue, the rarest first as Shaders lists them, owned or not.
pub(super) fn lab_blades() -> Vec<LabBlade> {
    let lab = crate::saber_lab::blades();
    let mut blades = vec![LabBlade::Worn, LabBlade::Stock];
    blades.extend(lab.iter().copied().map(LabBlade::Skin));
    blades.extend(
        crate::unlockables::blade_skins_by_tier()
            .iter()
            .filter(|skin| !lab.contains(&skin.id))
            .map(|skin| LabBlade::Skin(skin.id)),
    );
    blades
}

/// What a press on the tab holds until it is let go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum LabDrag {
    /// The speed slider's knob.
    Speed,
    /// The camera, the pointer last at (window pixels).
    View([f32; 2]),
}

impl Panel {
    /// The blade the tab shows: its choice of [`lab_blades`].
    pub(super) fn lab_blade(&self) -> LabBlade {
        let blades = lab_blades();
        blades[self.lab_blade.min(blades.len() - 1)]
    }

    /// What the tab asks of the stage while the lab is on.
    pub(super) fn lab_wish(&self) -> Option<StageWish> {
        crate::saber_lab::active().then(|| StageWish {
            // What the player wears at first (the Shaders tab equips any skin), so that
            // the lab's own shows once equipped (the user, 18/10/2026); or any blade the
            // Blade row previews, owned or not, drawn on this PC only.
            skin: match self.lab_blade() {
                LabBlade::Worn => PreviewSkin::Worn,
                LabBlade::Stock => PreviewSkin::Stock,
                LabBlade::Skin(id) => PreviewSkin::Skin(id),
            },
            // The builder's model holds the blade on the right: no saber floats.
            saber: (!self.lab_building()).then(|| self.spin.turn()),
            // The stage dresses both sabers in the lab's blade (`stage_skins`).
            on_second: false,
            on_both: true,
            holocron: false,
            lab: Some(LabStage {
                build: self.lab_building(),
                ..self.lab
            }),
        })
    }

    /// The style the stage's model holds its hilts in: a staff and two sabers swing in
    /// their own, not in the tab's.
    pub(crate) fn set_lab_held(&mut self, held: SaberStyle) {
        self.lab_held = held;
    }

    /// Step the blade by `by` among [`lab_blades`], wrapping.
    pub(super) fn step_lab_blade(&mut self, by: isize) {
        let count = lab_blades().len() as isize;
        let at = (self.lab_blade as isize).min(count - 1);
        self.lab_blade = (at + by).rem_euclid(count) as usize;
    }

    /// Put the camera behind the model or back on the shot, looking from where either
    /// starts.
    fn set_lab_chase(&mut self, chase: bool) {
        if chase != self.lab.chase {
            self.lab.chase = chase;
            self.lab.view = LabView::default();
        }
    }

    /// Step the speed by `by`, on the slider's steps.
    fn step_lab_speed(&mut self, by: f32) {
        self.lab.speed = snap_speed(self.lab.speed + by);
    }

    /// Wear the blade shown, or the stock blade when it is worn; nothing for what is worn
    /// already shown as such, or a blade the player cannot wear (it is only previewed).
    pub(super) fn lab_equip(&self, setting: &str) -> PanelAction {
        let setting = setting.trim();
        match self.lab_blade() {
            LabBlade::Worn => PanelAction::None,
            LabBlade::Stock => PanelAction::Wear(""),
            LabBlade::Skin(id) if id.eq_ignore_ascii_case(setting) => PanelAction::Wear(""),
            LabBlade::Skin(id) if self.lab_wearable => PanelAction::Wear(id),
            LabBlade::Skin(_) => PanelAction::None,
        }
    }

    /// A key on the tab: 1 to 3 the style, Space stands or swings, I lights or puts out
    /// the blade, F the flourish, C clashes, M mutes, minus and plus the speed, T the
    /// camera behind the model or on the shot, R the camera back, Left and Right the
    /// blade, Enter equips; `None` leaves it to the page's other keys.
    pub(super) fn lab_key(&mut self, key: KeyCode) -> Option<PanelAction> {
        match key {
            KeyCode::Digit1 | KeyCode::Numpad1 => self.lab.style = 0,
            KeyCode::Digit2 | KeyCode::Numpad2 => self.lab.style = 1,
            KeyCode::Digit3 | KeyCode::Numpad3 => self.lab.style = 2,
            KeyCode::Space => self.lab.swinging = !self.lab.swinging,
            KeyCode::KeyI => self.lab.lit = !self.lab.lit,
            KeyCode::KeyF => self.lab.flourish = !self.lab.flourish,
            KeyCode::KeyC => self.lab.clash = self.lab.clash.wrapping_add(1),
            KeyCode::KeyL => self.lab.trail = !self.lab.trail,
            KeyCode::KeyT => self.set_lab_chase(!self.lab.chase),
            KeyCode::KeyM => self.lab.muted = !self.lab.muted,
            KeyCode::Minus | KeyCode::NumpadSubtract => self.step_lab_speed(-SPEED_STEP),
            KeyCode::Equal | KeyCode::NumpadAdd => self.step_lab_speed(SPEED_STEP),
            KeyCode::KeyR => self.lab.view = LabView::default(),
            KeyCode::KeyE => self.open_lab_editor(),
            KeyCode::KeyB => self.switch_lab_view(),
            KeyCode::ArrowLeft => self.step_lab_blade(-1),
            KeyCode::ArrowRight => self.step_lab_blade(1),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                return Some(self.lab_equip(&self.lab_setting));
            }
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// A click on one of the tab's choices; `None` for any other token.
    pub(super) fn lab_click(&mut self, token: u16) -> Option<PanelAction> {
        let choice = |base: u16, count: usize| {
            token
                .checked_sub(base)
                .map(usize::from)
                .filter(|choice| *choice < count)
        };
        if let Some(style) = choice(STYLE_BASE, STYLES.len()) {
            self.lab.style = style;
        } else if let Some(moves) = choice(MOVES_BASE, MOVES.len()) {
            self.lab.swinging = moves == 1;
        } else if let Some(lit) = choice(LIT_BASE, LIT.len()) {
            self.lab.lit = lit == 1;
        } else if let Some(sound) = choice(SOUND_BASE, SOUND.len()) {
            self.lab.muted = sound == 1;
        } else if let Some(camera) = choice(CAMERA_BASE, CAMERA.len()) {
            self.set_lab_chase(camera == 1);
        } else if token == FLOURISH_TOKEN {
            self.lab.flourish = !self.lab.flourish;
        } else if token == TRAIL_TOKEN {
            self.lab.trail = !self.lab.trail;
        } else if token == CLASH_TOKEN {
            self.lab.clash = self.lab.clash.wrapping_add(1);
        } else if token == BLADE_PREVIOUS {
            self.step_lab_blade(-1);
        } else if token == BLADE_NEXT {
            self.step_lab_blade(1);
        } else if token == LAB_EQUIP {
            return Some(self.lab_equip(&self.lab_setting));
        } else {
            return None;
        }
        Some(PanelAction::None)
    }

    /// A pointer event on the tab's slider or picture: a press on the slider takes its
    /// knob, one on the picture the camera, until let go; the wheel over the picture
    /// brings the camera nearer or takes it back. `None` leaves the event to the page.
    pub(super) fn lab_pointer(&mut self, event: InputEvent) -> Option<PanelAction> {
        let on = |panel: &Self, token: u16, at: sjk_ui::Vec2| {
            panel
                .ui
                .rect_for(token)
                .is_some_and(|rect| rect.contains(at))
        };
        match event {
            InputEvent::PointerPress { position: at, .. } if self.backstage != Backstage::None => {
                if on(self, SPEED_TOKEN, at) {
                    self.lab_drag = Some(LabDrag::Speed);
                    self.lab_speed_at(at.x);
                } else if on(self, VIEW_TOKEN, at) {
                    self.lab_drag = Some(LabDrag::View([at.x, at.y]));
                } else {
                    return None;
                }
            }
            InputEvent::PointerMove(at) => match self.lab_drag? {
                LabDrag::Speed => self.lab_speed_at(at.x),
                LabDrag::View([x, y]) => {
                    let scale = self.scale.max(0.01);
                    self.lab.view.drag([(at.x - x) / scale, (at.y - y) / scale]);
                    self.lab_drag = Some(LabDrag::View([at.x, at.y]));
                }
            },
            InputEvent::PointerRelease { .. } | InputEvent::PointerLeave => {
                self.lab_drag.take()?;
            }
            InputEvent::PointerWheel {
                position: at,
                delta,
            } if delta.y != 0.0 && on(self, VIEW_TOKEN, at) => self.lab.view.wheel(delta.y),
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// Set the speed where the pointer is along the slider (window pixels).
    fn lab_speed_at(&mut self, x: f32) {
        let Some(rect) = self.ui.rect_for(SPEED_TOKEN) else {
            return;
        };
        let grab = SLIDER_GRAB * self.scale;
        let ratio = ((x - rect.x - grab) / (rect.width - grab * 2.0).max(1.0)).clamp(0.0, 1.0);
        self.lab.speed = snap_speed(SLOWEST + (FASTEST - SLOWEST) * ratio);
    }

    /// Show the model in `style`, swinging or not, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_for_shot(&mut self, style: usize, swinging: bool) {
        self.lab.style = style.min(STYLES.len() - 1);
        self.lab.swinging = swinging;
    }

    /// Look at the model from `view`, its blade lit or not, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_view_for_shot(&mut self, view: LabView, lit: bool) {
        self.lab.view = view;
        self.lab.lit = lit;
    }

    /// Preview blade skin `id` on the Blade row, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_blade_for_shot(&mut self, id: &str) {
        if let Some(at) = lab_blades()
            .iter()
            .position(|blade| matches!(blade, LabBlade::Skin(skin) if *skin == id))
        {
            self.lab_blade = at;
        }
    }

    /// Show the floating saber's still trail or not, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_trail_for_shot(&mut self, trail: bool) {
        self.lab.trail = trail;
    }

    /// Put the camera behind the model or back on the shot, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_chase_for_shot(&mut self, chase: bool) {
        self.set_lab_chase(chase);
    }

    /// The token the tab's focus starts on.
    pub(super) fn lab_focus(&self) -> u16 {
        STYLE_BASE + self.lab.style as u16
    }

    /// The tab's keys for the line at the bottom.
    pub(super) fn lab_keys(&self, keys: &mut Vec<(&'static [&'static str], &str)>) {
        if self.lab_edit.is_some() {
            self.lab_edit_keys(keys);
            return;
        }
        if self.lab_building() {
            self.lab_build_keys(keys);
            return;
        }
        if self.lab_held == SaberStyle::Single {
            keys.push((&["1", "2", "3"], "style"));
        }
        keys.push((
            &["Space"],
            if self.lab.swinging { "stand" } else { "swing" },
        ));
        keys.push((&["I"], if self.lab.lit { "switch off" } else { "ignite" }));
        keys.push((&["M"], if self.lab.muted { "sound" } else { "mute" }));
        keys.push((
            &["T"],
            if self.lab.chase {
                "stage"
            } else {
                "third person"
            },
        ));
        keys.push((&["Left", "Right"], "blade"));
        keys.push((&["Enter"], "equip"));
    }

    /// The Saber Lab tab.
    pub(super) fn saber_lab(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        self.lab_setting.clear();
        self.lab_setting.push_str(inputs.setting);
        let setting = inputs.setting.trim();
        let choice = self.lab_blade();
        // Whether the player may wear the blade shown: one of the lab's on this PC, or one
        // their profile lists.
        self.lab_wearable = match choice {
            LabBlade::Worn => false,
            LabBlade::Stock => true,
            LabBlade::Skin(id) => {
                crate::saber_lab::owns(id)
                    || crate::unlockables::blade_skin(id)
                        .is_some_and(|skin| inputs.holdings.can_wear(skin))
            }
        };
        let status = crate::saber_lab::status().unwrap_or_default();
        self.lead(frame, crate::profile_hub::SABER_LAB, &status);
        // The stage is the menu map's: over a match there is no model to show.
        if self.backstage == Backstage::None {
            left_lines(
                &mut self.ui,
                frame,
                "The Saber Lab's stage is on the main menu: leave the match to see the saber held, swung and floating.",
                [COLUMN_X, FIRST_ROW, COLUMN_WIDTH],
                (52, 3),
                18.0,
                color::TEXT,
                TextAlign::Start,
            );
            return;
        }
        // The builder (the tab's first view) or the stage's choices, the switch between them
        // over the picture's top right.
        let building = self.lab_building();
        kit::button(
            &mut self.ui,
            frame,
            super::saber_lab_builder::VIEW_SWITCH_RECT,
            if building {
                "Stage controls"
            } else {
                "Builder"
            },
            false,
            true,
            false,
            super::saber_lab_builder::VIEW_SWITCH,
        );
        if building {
            self.lab_builder(frame, inputs);
            return;
        }
        // A drag on the picture turns the camera round the model; one that starts on the
        // floating saber turns the saber.
        let [x, y, width, height] = VIEW_AREA;
        self.ui
            .hit_region(VIEW_TOKEN, frame.rect(x, y, width, height));
        let [x, y, width, height] = LAB_SABER_AREA;
        self.ui
            .hit_region(SABER_TOKEN, frame.rect(x, y, width, height));
        // The floating saber's still trail, ticked over it.
        let [x, y] = TRAIL_TICK;
        kit::tick(&mut self.ui, frame, x, y, self.lab.trail, false);
        self.ui
            .hit_region(TRAIL_TOKEN, frame.rect(x - 4.0, y - 16.0, 120.0, 32.0));
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Trail"),
            frame.rect(x + kit::TICK_BOX + 10.0, y - 12.0, 80.0, 24.0),
            17.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        // The editor, open, takes the column under the floating saber.
        if self.lab_edit.is_some() {
            self.lab_editor(frame);
            return;
        }
        let label = |panel: &mut Self, words: &str, y: f32| {
            text(
                &mut panel.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(COLUMN_X, y - 14.0, 160.0, 28.0),
                21.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::Start,
            );
        };
        let value = |panel: &mut Self, words: &str, y: f32, colour, align| {
            text(
                &mut panel.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(RIGHT_X - 380.0, y - 15.0, 380.0, 30.0),
                21.0 * s,
                colour,
                FontWeight::Semibold,
                align,
            );
        };
        let mut y = FIRST_ROW;
        label(self, "Style", y);
        match self.lab_held {
            SaberStyle::Single => kit::segments(
                &mut self.ui,
                frame,
                RIGHT_X,
                y,
                &STYLES,
                self.lab.style,
                false,
                STYLE_BASE,
            ),
            // A staff and two sabers have the one style each.
            held => value(
                self,
                &format!("{}: its own", held.label()),
                y,
                color::TEXT,
                TextAlign::End,
            ),
        }
        y += ROW_STEP;
        label(self, "Moves", y);
        kit::segments(
            &mut self.ui,
            frame,
            RIGHT_X,
            y,
            &MOVES,
            usize::from(self.lab.swinging),
            false,
            MOVES_BASE,
        );
        y += ROW_STEP;
        label(self, "Speed", y);
        kit::slider(
            &mut self.ui,
            frame,
            SLIDER_X,
            y,
            SLIDER_WIDTH,
            (self.lab.speed - SLOWEST) / (FASTEST - SLOWEST),
            self.lab_drag == Some(LabDrag::Speed),
        );
        self.ui.hit_region(
            SPEED_TOKEN,
            frame.rect(
                SLIDER_X - SLIDER_GRAB,
                y - 18.0,
                SLIDER_WIDTH + SLIDER_GRAB * 2.0,
                36.0,
            ),
        );
        value(
            self,
            &format!("{}%", (self.lab.speed * 100.0).round()),
            y,
            color::TEXT,
            TextAlign::End,
        );
        y += ROW_STEP;
        label(self, "Saber", y);
        // The flourish's tick, left of the blade's state.
        let tick = RIGHT_X - 380.0;
        kit::tick(&mut self.ui, frame, tick, y, self.lab.flourish, false);
        self.ui.hit_region(
            FLOURISH_TOKEN,
            frame.rect(tick - 4.0, y - 18.0, 150.0, 36.0),
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Flourish"),
            frame.rect(tick + kit::TICK_BOX + 10.0, y - 12.0, 110.0, 24.0),
            17.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Start,
        );
        kit::segments(
            &mut self.ui,
            frame,
            RIGHT_X,
            y,
            &LIT,
            usize::from(self.lab.lit),
            false,
            LIT_BASE,
        );
        y += ROW_STEP;
        label(self, "Sound", y);
        // The Clash button, left of the sound's choice: strikes the blades as a block does.
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X - 380.0, y - 20.0, 120.0, 40.0],
            "Clash",
            false,
            true,
            false,
            CLASH_TOKEN,
        );
        kit::segments(
            &mut self.ui,
            frame,
            RIGHT_X,
            y,
            &SOUND,
            usize::from(self.lab.muted),
            false,
            SOUND_BASE,
        );
        y += ROW_STEP;
        label(self, "View", y);
        kit::segments(
            &mut self.ui,
            frame,
            RIGHT_X,
            y,
            &CAMERA,
            usize::from(self.lab.chase),
            false,
            CAMERA_BASE,
        );
        y += ROW_STEP;
        label(self, "Blade", y);
        let skin_name = |id: &str| {
            crate::unlockables::blade_skin(id).map_or("the stock blade", |skin| skin.name)
        };
        let name = match choice {
            LabBlade::Worn => format!("Yours: {}", skin_name(setting)),
            LabBlade::Stock => "Stock blade".to_owned(),
            LabBlade::Skin(id) => skin_name(id).to_owned(),
        };
        let arrow = 42.0;
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X - 380.0, y - 20.0, arrow, 40.0],
            "<",
            false,
            true,
            false,
            BLADE_PREVIOUS,
        );
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X - arrow, y - 20.0, arrow, 40.0],
            ">",
            false,
            true,
            false,
            BLADE_NEXT,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(RIGHT_X - 380.0 + arrow, y - 15.0, 380.0 - arrow * 2.0, 30.0),
            21.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        y += ROW_STEP;
        let worn = match choice {
            LabBlade::Worn => true,
            LabBlade::Stock => setting.is_empty(),
            LabBlade::Skin(id) => id.eq_ignore_ascii_case(setting),
        };
        let lab_skin = matches!(choice, LabBlade::Skin(id) if crate::saber_lab::owns(id));
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X - 200.0, y - 21.0, 200.0, 42.0],
            if worn && choice != LabBlade::Stock {
                "Unequip"
            } else {
                "Equip"
            },
            !worn,
            match choice {
                LabBlade::Worn => false,
                LabBlade::Stock => !worn,
                LabBlade::Skin(_) => worn || self.lab_wearable,
            },
            false,
            LAB_EQUIP,
        );
        let note = match choice {
            LabBlade::Worn => "What you wear. Left and Right preview the others.",
            _ if worn => "Worn, and shown here.",
            _ if lab_skin => "Previewing. Equip wears it on this PC only.",
            _ if self.lab_wearable => "Previewing one of yours. Equip to wear it.",
            _ => "Previewing a blade you do not own: here only.",
        };
        let (edit, editable) = self.lab_edit_button();
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X - 340.0, y - 21.0, 130.0, 42.0],
            edit,
            false,
            editable,
            false,
            saber_lab_editor::EDIT_TOKEN,
        );
        // Why the editor could not open, else what the blade shown is.
        let (note, colour) = match &self.lab_edit_problem {
            Some(why) => (why.clone(), color::EMBER),
            None => (note.to_owned(), color::QUIET),
        };
        left_lines(
            &mut self.ui,
            frame,
            &note,
            [COLUMN_X, y - 18.0, COLUMN_WIDTH - 350.0],
            (32, 2),
            14.0,
            colour,
            TextAlign::Start,
        );
        y += ROW_STEP - 4.0;
        left_lines(
            &mut self.ui,
            frame,
            "Drag the picture to look round the model, the wheel to come nearer, R to look as at first. A save in the lab's folder shows at once.",
            [COLUMN_X, y, COLUMN_WIDTH],
            (66, 2),
            15.0,
            color::QUIET,
            TextAlign::Start,
        );
    }
}

impl Panel {
    /// The stage's buttons in a row ending at the right edge, centred on `y`: Clash, the
    /// blade on or off, standing or swinging.
    pub(super) fn lab_stage_buttons(&mut self, frame: &Frame, y: f32) {
        kit::segments(
            &mut self.ui,
            frame,
            RIGHT_X,
            y,
            &MOVES,
            usize::from(self.lab.swinging),
            false,
            MOVES_BASE,
        );
        let lit_right = RIGHT_X - kit::segments_width(&MOVES) - 16.0;
        kit::segments(
            &mut self.ui,
            frame,
            lit_right,
            y,
            &LIT,
            usize::from(self.lab.lit),
            false,
            LIT_BASE,
        );
        let clash_right = lit_right - kit::segments_width(&LIT) - 16.0;
        kit::button(
            &mut self.ui,
            frame,
            [clash_right - 110.0, y - 20.0, 110.0, 40.0],
            "Clash",
            false,
            true,
            false,
            CLASH_TOKEN,
        );
    }
}

/// `speed` on the slider's steps, within its span.
fn snap_speed(speed: f32) -> f32 {
    ((speed / 0.05).round() * 0.05).clamp(SLOWEST, FASTEST)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys choose the style, stand or swing, light or put out the blade, mute, and
    /// leave the others to the page; a click on a choice does the same.
    #[test]
    fn keys_and_clicks_choose_the_style_and_the_moves() {
        let mut panel = Panel::new();
        assert_eq!(panel.lab_key(KeyCode::Digit1), Some(PanelAction::None));
        assert_eq!(panel.lab.style, 0);
        let swinging = panel.lab.swinging;
        assert_eq!(panel.lab_key(KeyCode::Space), Some(PanelAction::None));
        assert_eq!(panel.lab.swinging, !swinging);
        assert_eq!(panel.lab_key(KeyCode::Escape), None);
        assert_eq!(panel.lab_click(STYLE_BASE + 2), Some(PanelAction::None));
        assert_eq!(panel.lab.style, 2);
        assert_eq!(panel.lab_click(MOVES_BASE), Some(PanelAction::None));
        assert!(!panel.lab.swinging);
        assert_eq!(panel.lab_click(MOVES_BASE + 1), Some(PanelAction::None));
        assert!(panel.lab.swinging);
        assert_eq!(panel.lab_click(STYLE_BASE + 3), None, "no fourth style");
        assert_eq!(panel.lab_focus(), STYLE_BASE + 2);
        assert!(panel.lab.lit && !panel.lab.muted);
        panel.lab_key(KeyCode::KeyI);
        panel.lab_key(KeyCode::KeyM);
        assert!(!panel.lab.lit && panel.lab.muted);
        panel.lab_click(LIT_BASE + 1);
        panel.lab_click(SOUND_BASE);
        assert!(panel.lab.lit && !panel.lab.muted);
        // The flourish ticks on and off; the third person puts the view back as it starts.
        assert!(!panel.lab.trail);
        panel.lab_key(KeyCode::KeyL);
        assert!(panel.lab.trail);
        panel.lab_click(TRAIL_TOKEN);
        assert!(!panel.lab.trail);
        assert!(panel.lab.flourish);
        panel.lab_key(KeyCode::KeyF);
        assert!(!panel.lab.flourish);
        panel.lab_click(FLOURISH_TOKEN);
        assert!(panel.lab.flourish);
        panel.lab.view.drag([50.0, 50.0]);
        panel.lab_key(KeyCode::KeyT);
        assert!(panel.lab.chase);
        assert_eq!(panel.lab.view, LabView::default());
        panel.lab_click(CAMERA_BASE);
        assert!(!panel.lab.chase);
    }

    /// The Blade row starts on what the player wears, then the stock blade, then every
    /// blade skin of the catalogue once; Equip wears only what the player may wear, the
    /// rest being previewed, and takes off what is worn.
    #[test]
    fn the_blade_row_previews_every_blade_and_equips_only_what_may_be_worn() {
        let blades = lab_blades();
        assert_eq!(blades[..2], [LabBlade::Worn, LabBlade::Stock]);
        for skin in crate::unlockables::blade_skins_by_tier() {
            let found = blades
                .iter()
                .filter(|blade| **blade == LabBlade::Skin(skin.id))
                .count();
            assert_eq!(found, 1, "{}", skin.id);
        }
        let mut panel = Panel::new();
        assert_eq!(panel.lab_blade(), LabBlade::Worn);
        assert_eq!(panel.lab_equip("saber_sun"), PanelAction::None);
        panel.lab_key(KeyCode::ArrowRight);
        assert_eq!(panel.lab_blade(), LabBlade::Stock);
        assert_eq!(panel.lab_equip("saber_sun"), PanelAction::Wear(""));
        panel.lab_key(KeyCode::ArrowLeft);
        panel.lab_key(KeyCode::ArrowLeft);
        assert_eq!(panel.lab_blade(), *lab_blades().last().unwrap(), "it wraps");
        panel.lab_blade = blades
            .iter()
            .position(|blade| *blade == LabBlade::Skin("saber_storm"))
            .unwrap();
        panel.lab_wearable = false;
        assert_eq!(panel.lab_equip(""), PanelAction::None, "previewed only");
        panel.lab_wearable = true;
        assert_eq!(panel.lab_equip(""), PanelAction::Wear("saber_storm"));
        assert_eq!(panel.lab_equip("saber_storm"), PanelAction::Wear(""));
    }

    /// The speed steps by the keys within the slider's span, the game's speed among them.
    #[test]
    fn the_speed_steps_within_its_span() {
        let mut panel = Panel::new();
        assert_eq!(panel.lab.speed, 1.0);
        panel.lab_key(KeyCode::Minus);
        assert!((panel.lab.speed - 0.9).abs() < 1e-4);
        for _ in 0..40 {
            panel.lab_key(KeyCode::Equal);
        }
        assert_eq!(panel.lab.speed, FASTEST);
        for _ in 0..40 {
            panel.lab_key(KeyCode::NumpadSubtract);
        }
        assert!((panel.lab.speed - SLOWEST).abs() < 1e-4);
    }

    /// A drag turns the camera all the way round but tilts it only so far; the wheel
    /// brings it only so near; R puts it back.
    #[test]
    fn the_view_turns_round_tilts_and_comes_nearer_within_bounds() {
        let mut panel = Panel::new();
        panel.lab.view.drag([-100.0, 0.0]);
        assert!((panel.lab.view.turn - 0.6).abs() < 1e-4);
        panel.lab.view.drag([0.0, 10_000.0]);
        assert_eq!(panel.lab.view.tilt, HIGHEST_TILT);
        panel.lab.view.drag([0.0, -10_000.0]);
        assert_eq!(panel.lab.view.tilt, LOWEST_TILT);
        for _ in 0..50 {
            panel.lab.view.wheel(1.0);
        }
        assert_eq!(panel.lab.view.distance, NEAREST);
        panel.lab_key(KeyCode::KeyR);
        assert_eq!(panel.lab.view, LabView::default());
    }

    /// The floating saber's area is clear of the choices under it and inside the frame,
    /// and the picture's drag is left of both.
    #[test]
    fn the_floating_saber_stands_over_the_choices() {
        let [x, y, width, height] = LAB_SABER_AREA;
        assert!(y + height < FIRST_ROW - 30.0);
        assert!(x + width <= RIGHT_X && x >= 1_000.0);
        assert!(VIEW_AREA[0] + VIEW_AREA[2] <= COLUMN_X.min(x));
    }
}
