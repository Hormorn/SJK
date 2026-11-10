//! The Jedi Knight II status HUD: `cg_hudFiles 2`, as EternalJK's `CG_DrawHUD`
//! draws it (`CG_DrawHealthJK2`, `CG_DrawArmorJK2`, `CG_DrawForcePowerJK2` and
//! `CG_DrawAmmoJK2`). Its pictures are EternalJK's (`assets/japro/gfx/hud`) and
//! are built into the client, so the HUD needs nothing installed. EternalJK's
//! scrolling static behind the metal frames is not drawn.
//!
//! Its selector bar, `CG_DrawIconBackground`: while the weapon, Force or
//! inventory selector shows, its background (retail's `gfx/hud/background`,
//! `_f` and `_i`) opens from the middle over 130 ms and the prongs at its ends
//! light and move in by 8 units; when the selector goes they close again, and the
//! unlit prongs stay at the ends.
//!
//! Positions are the 640x480 screen with each corner's 80-unit square kept to
//! its own edge, which is what EternalJK's `widthRatioCoef` does on wide screens.

use super::{AMMO_INDEX, AMMO_MAX, WHITE, infinite_ammo};
use super::{
    BLACK, Frame, HUD_GREEN, HUD_ORANGE, HUD_RED, MEDIUM_POINTS, Readout, Selector, TextRun, Timers,
};
use crate::menu_hud::layout::{Layout, Side};
use image::RgbaImage;
use std::fmt::Write as _;

/// What marks a picture name as one of these, in [`Layout::pictures`].
const PREFIX: &str = "sjk:jk2/";
/// `MAX_TICS` of the Force and ammunition arcs.
const TICS: usize = 14;
const SQUARE: f32 = 80.0;
const LEFT_X: f32 = 0.0;
const RIGHT_X: f32 = 560.0;
const TOP: f32 = 400.0;
/// Where the score ends and its top, as EternalJK's `SCREEN_WIDTH - 124`.
const SCORE_END: f32 = 516.0;
const SCORE_Y: f32 = 457.0;
/// At or below this share of maximum health the health tic blinks.
const HEALTH_BLINK: f32 = 0.20;
/// Milliseconds between the tics' blinks.
const BLINK_MS: i32 = 100;
/// Milliseconds the selector background takes to open or close.
const ICON_OPEN_MS: f32 = 130.0;
/// The selector background: `x2 + 60`, its width, and where its halves meet
/// (`y2 + 30`, with `x2 = 30`, `y2 = SCREEN_HEIGHT - 70`).
const ICON_X: f32 = 90.0;
const ICON_WIDTH: f32 = 460.0;
const ICON_MIDDLE: f32 = 440.0;
/// The halves' height when open.
const ICON_HEIGHT: f32 = 60.0;
/// The prongs: the left one's x (`x2 + 37`), the right one's right edge
/// (`SCREEN_WIDTH - (36 + x2)`), their top (`y2 - 10`), size and how far in
/// they move when open.
const PRONG_LEFT: f32 = 67.0;
const PRONG_RIGHT: f32 = 574.0;
const PRONG_TOP: f32 = 400.0;
const PRONG_WIDTH: f32 = 40.0;
const PRONG_HEIGHT: f32 = 80.0;
const PRONG_TRAVEL: f32 = 8.0;

macro_rules! pictures {
    ($($file:literal),+ $(,)?) => {
        [$(($file, include_bytes!(concat!("../../../assets/hud/jk2/", $file)) as &[u8])),+]
    };
}

/// The built-in pictures, in the order of the indices below.
const PICTURES: [(&str, &[u8]); 31] = pictures![
    "jk2hudleft.png",
    "jk2hudleft_innerframe.png",
    "jk2hudrightframe.png",
    "jk2hudright_innerframe.png",
    "armor1.png",
    "armor2.png",
    "armor_tic.png",
    "health.png",
    "health_tic.png",
    "force_tick1.png",
    "force_tick2.png",
    "force_tick3.png",
    "force_tick4.png",
    "force_tick5.png",
    "force_tick6.png",
    "force_tick7.png",
    "ammo_tick1.png",
    "ammo_tick2.png",
    "ammo_tick3.png",
    "ammo_tick4.png",
    "ammo_tick5.png",
    "ammo_tick6.png",
    "ammo_tick7.png",
    "saber_stylesfast.jpg",
    "saber_stylesmed.jpg",
    "saber_stylesstrong.jpg",
    "saber_stylesstaff.jpg",
    "prong_off.png",
    "prong_on_w.png",
    "prong_on_f.png",
    "prong_on_i.png",
];

/// The selector backgrounds, retail's own (`assets1.pk3`), after [`PICTURES`]:
/// `cgs.media.weaponIconBackground`, `forceIconBackground` and
/// `inventoryIconBackground`.
const BACKGROUNDS: [&str; 3] = [
    "gfx/hud/background",
    "gfx/hud/background_f",
    "gfx/hud/background_i",
];

const LEFT_FRAME: u16 = 0;
const LEFT_INNER: u16 = 1;
const RIGHT_FRAME: u16 = 2;
const RIGHT_INNER: u16 = 3;
const ARMOR_OUTER: u16 = 4;
const ARMOR_INNER: u16 = 5;
const ARMOR_TIC: u16 = 6;
const HEALTH: u16 = 7;
const HEALTH_TIC: u16 = 8;
const FORCE_TICK: u16 = 9;
const AMMO_TICK: u16 = 16;
const SABER_STYLE: u16 = 23;
const PRONG_OFF: u16 = 27;
/// The lit prongs, then the backgrounds, in [`Selector`] order.
const PRONG_ON: u16 = 28;
const BACKGROUND: u16 = PICTURES.len() as u16;

/// The offset of `selector`'s lit prong or background from the first one.
fn selector_index(selector: Selector) -> u16 {
    match selector {
        Selector::Weapon => 0,
        Selector::Force => 1,
        Selector::Inventory => 2,
    }
}

/// One arc tic: x, y, width (negative mirrors it) and height from the square's
/// corner, and which of the seven pictures it is. `forceTicPos`, `ammoTicPos`.
type Tic = (f32, f32, f32, f32, u16);

const FORCE_TICS: [Tic; TICS] = [
    (11.0, 41.0, 20.0, 10.0, 0),
    (12.0, 45.0, 20.0, 10.0, 1),
    (14.0, 49.0, 20.0, 10.0, 2),
    (17.0, 52.0, 20.0, 10.0, 3),
    (22.0, 55.0, 10.0, 10.0, 4),
    (28.0, 57.0, 10.0, 20.0, 5),
    (34.0, 59.0, 10.0, 10.0, 6),
    (46.0, 59.0, -10.0, 10.0, 6),
    (52.0, 57.0, -10.0, 20.0, 5),
    (58.0, 55.0, -10.0, 10.0, 4),
    (63.0, 52.0, -20.0, 10.0, 3),
    (66.0, 49.0, -20.0, 10.0, 2),
    (68.0, 45.0, -20.0, 10.0, 1),
    (69.0, 41.0, -20.0, 10.0, 0),
];

const AMMO_TICS: [Tic; TICS] = [
    (12.0, 34.0, 10.0, 10.0, 6),
    (13.0, 28.0, 10.0, 10.0, 5),
    (15.0, 23.0, 10.0, 10.0, 4),
    (19.0, 19.0, 10.0, 10.0, 3),
    (23.0, 15.0, 10.0, 10.0, 2),
    (29.0, 12.0, 10.0, 10.0, 1),
    (34.0, 11.0, 10.0, 10.0, 0),
    (47.0, 11.0, -10.0, 10.0, 0),
    (52.0, 12.0, -10.0, 10.0, 1),
    (58.0, 15.0, -10.0, 10.0, 2),
    (62.0, 19.0, -10.0, 10.0, 3),
    (66.0, 23.0, -10.0, 10.0, 4),
    (68.0, 28.0, -10.0, 10.0, 5),
    (69.0, 34.0, -10.0, 10.0, 6),
];

/// How many pictures [`attach`] adds.
pub(in crate::menu_hud) const PICTURE_COUNT: usize = PICTURES.len() + BACKGROUNDS.len();

/// The units the `index`th of the pictures [`attach`] adds is drawn at, at most.
pub(in crate::menu_hud) fn largest(index: usize) -> f32 {
    if index < PICTURES.len() {
        SQUARE
    } else {
        ICON_WIDTH
    }
}

/// Add the pictures to `layout` and mark it as this HUD.
pub(in crate::menu_hud) fn attach(layout: &mut Layout) {
    layout.jk2 = Some(layout.pictures.len() as u16);
    layout
        .pictures
        .extend(PICTURES.iter().map(|(file, _)| format!("{PREFIX}{file}")));
    layout
        .pictures
        .extend(BACKGROUNDS.iter().map(|name| (*name).to_owned()));
}

/// Whether `name` is a built-in picture, and if so whether it blends additively
/// (the saber styles, black around their marks).
pub(in crate::menu_hud) fn additive(name: &str) -> Option<bool> {
    name.strip_prefix(PREFIX).map(|file| file.ends_with(".jpg"))
}

/// The built-in picture `name`, if it is one.
pub(in crate::menu_hud) fn decode(name: &str) -> Option<RgbaImage> {
    let file = name.strip_prefix(PREFIX)?;
    let (_, bytes) = PICTURES.iter().find(|(known, _)| *known == file)?;
    Some(image::load_from_memory(bytes).ok()?.to_rgba8())
}

/// `color` with its red, green and blue scaled by `amount`.
fn dimmed(color: [f32; 4], amount: f32) -> [f32; 4] {
    [
        color[0] * amount,
        color[1] * amount,
        color[2] * amount,
        color[3],
    ]
}

impl Frame {
    /// The left corner (armor and health) and the right one (Force, ammunition
    /// or saber style, and the score), from picture `base` on.
    pub(super) fn jk2_status(
        &mut self,
        base: u16,
        readout: &Readout,
        timers: &mut Timers,
        digits: u16,
        score_label: &str,
    ) {
        self.jk2_selector(base, readout, timers);
        let start = (self.picture_count, self.text_count);
        self.jk2_square(LEFT_X, Side::Left, base + LEFT_INNER);
        self.jk2_armor(base, readout, timers, digits);
        self.jk2_health(base, readout, timers, digits);
        self.jk2_square(LEFT_X, Side::Left, base + LEFT_FRAME);
        self.end_block(0, start);

        let start = (self.picture_count, self.text_count);
        self.jk2_square(RIGHT_X, Side::Right, base + RIGHT_INNER);
        self.jk2_tics(
            &FORCE_TICS,
            base + FORCE_TICK,
            readout.force as f32,
            100.0 / TICS as f32,
        );
        self.jk2_ammo(base, readout, digits);
        self.jk2_square(RIGHT_X, Side::Right, base + RIGHT_FRAME);
        self.jk2_score(readout, score_label);
        self.end_block(1, start);
    }

    /// `CG_DrawIconBackground`: the selector background opening or closing and
    /// the prongs at its ends.
    fn jk2_selector(&mut self, base: u16, readout: &Readout, timers: &mut Timers) {
        let (percent, prong) = match readout.selector {
            Some((selector, age)) => {
                timers.icon_selector = selector;
                timers.icon_closed_at = None;
                let percent = if timers.icon_active {
                    1.0
                } else {
                    let percent = age as f32 / ICON_OPEN_MS;
                    if percent > 1.0 {
                        timers.icon_active = true;
                    }
                    percent.clamp(0.0, 1.0)
                };
                (percent, PRONG_ON + selector_index(selector))
            }
            None if timers.icon_active => {
                let closed = *timers.icon_closed_at.get_or_insert(readout.time);
                let percent = 1.0 - (readout.time - closed) as f32 / ICON_OPEN_MS;
                if percent < 0.0 {
                    timers.icon_active = false;
                    timers.icon_closed_at = None;
                }
                (percent.max(0.0), PRONG_OFF)
            }
            None => (0.0, PRONG_OFF),
        };
        if percent > 0.0 {
            let background = base + BACKGROUND + selector_index(timers.icon_selector);
            let height = ICON_HEIGHT * percent;
            // Two halves from the middle: the top one mirrored upwards.
            self.picture_at(
                [ICON_X, ICON_MIDDLE, ICON_WIDTH, -height],
                Side::Stretch,
                background,
                WHITE,
            );
            self.picture_at(
                [ICON_X, ICON_MIDDLE - 2.0, ICON_WIDTH, height],
                Side::Stretch,
                background,
                WHITE,
            );
        }
        let travel = PRONG_TRAVEL * percent;
        self.picture_at(
            [PRONG_LEFT + travel, PRONG_TOP, PRONG_WIDTH, PRONG_HEIGHT],
            Side::Left,
            base + prong,
            WHITE,
        );
        self.picture_at(
            [PRONG_RIGHT - travel, PRONG_TOP, -PRONG_WIDTH, PRONG_HEIGHT],
            Side::Right,
            base + prong,
            WHITE,
        );
    }

    fn jk2_square(&mut self, x: f32, side: Side, picture: u16) {
        self.picture_at([x, TOP, SQUARE, SQUARE], side, picture, WHITE);
    }

    fn jk2_tinted(&mut self, picture: u16, color: [f32; 4]) {
        self.picture_at([LEFT_X, TOP, SQUARE, SQUARE], Side::Left, picture, color);
    }

    /// `CG_DrawHealthJK2`.
    fn jk2_health(&mut self, base: u16, readout: &Readout, timers: &mut Timers, digits: u16) {
        let max = readout.max_health.max(1);
        let share = readout.health.min(max) as f32 / max as f32;
        self.jk2_tinted(base + HEALTH, dimmed(HUD_RED, share));
        let armor_share = (readout.armor - max / 2) as f32 / (max / 2).max(1) as f32;
        if share > HEALTH_BLINK {
            timers.health_flag = true;
        } else if timers.tick_flash_time < readout.time {
            timers.tick_flash_time = readout.time + BLINK_MS;
            timers.health_flag = if armor_share > 0.0 && armor_share < 0.5 {
                timers.armor_flag
            } else {
                !timers.health_flag
            };
        }
        if timers.health_flag {
            self.jk2_tinted(base + HEALTH_TIC, HUD_RED);
        }
        self.number_at(
            [LEFT_X + 16.0, TOP + 40.0, 6.0, 12.0],
            Side::Left,
            readout.health,
            HUD_RED,
            digits,
        );
    }

    /// `CG_DrawArmorJK2`: the outer ring is the upper half of the armor, the
    /// inner one the lower half.
    fn jk2_armor(&mut self, base: u16, readout: &Readout, timers: &mut Timers, digits: u16) {
        let max = readout.max_health.max(1);
        let half = (max / 2).max(1);
        let outer = ((readout.armor.min(max) - max / 2) as f32 / half as f32).max(0.0);
        self.jk2_tinted(base + ARMOR_OUTER, dimmed(HUD_GREEN, outer));
        let inner = if outer > 0.0 {
            1.0
        } else {
            readout.armor as f32 / half as f32
        };
        self.jk2_tinted(base + ARMOR_INNER, dimmed(HUD_GREEN, inner));
        if readout.armor == 0 {
            timers.armor_flag = false;
        } else if inner >= 0.5 {
            timers.armor_flag = true;
        } else if timers.tick_flash_time < readout.time {
            timers.tick_flash_time = readout.time + BLINK_MS;
            timers.armor_flag = !timers.armor_flag;
        }
        if timers.armor_flag {
            self.jk2_tinted(base + ARMOR_TIC, HUD_GREEN);
        }
        self.number_at(
            [LEFT_X + 32.0, TOP + 54.0, 6.0, 12.0],
            Side::Left,
            readout.armor,
            HUD_GREEN,
            digits,
        );
    }

    /// `CG_DrawAmmoJK2`: the saber's style, or the ammunition and its arc.
    fn jk2_ammo(&mut self, base: u16, readout: &Readout, digits: u16) {
        const SABER: u8 = 3;
        if readout.weapon == 0 {
            return;
        }
        if readout.weapon == SABER {
            let style = match readout.saber_style {
                1 => 0,
                2 => 1,
                3 => 2,
                6 | 7 => 3,
                _ => return,
            };
            self.picture_at(
                [RIGHT_X, TOP, SQUARE, SQUARE * 0.5],
                Side::Right,
                base + SABER_STYLE + style,
                WHITE,
            );
            return;
        }
        let ammo_type = AMMO_INDEX
            .get(usize::from(readout.weapon))
            .copied()
            .unwrap_or_default();
        let value = readout.ammo[ammo_type];
        if value < 0 || infinite_ammo(readout.weapon) {
            if let Some(run) = self.text() {
                run.text.push_str("--");
                *run = TextRun {
                    text: std::mem::take(&mut run.text),
                    x: RIGHT_X + 40.0,
                    y: TOP + 20.0,
                    size: MEDIUM_POINTS * 0.9,
                    color: HUD_ORANGE,
                    centred: true,
                    side: Some(Side::Right),
                    end_limit: None,
                };
            }
            self.jk2_tics(&AMMO_TICS, base + AMMO_TICK, 1.0, 0.0);
            return;
        }
        self.number_at(
            [RIGHT_X + 30.0, TOP + 26.0, 6.0, 12.0],
            Side::Right,
            value,
            HUD_ORANGE,
            digits,
        );
        self.jk2_tics(
            &AMMO_TICS,
            base + AMMO_TICK,
            value as f32,
            AMMO_MAX[ammo_type] as f32 / TICS as f32,
        );
    }

    /// One arc, filled from its last tic: a full tic is white, the next one
    /// dimmed by what is left of it, and the rest black.
    fn jk2_tics(&mut self, tics: &[Tic; TICS], base: u16, mut value: f32, inc: f32) {
        for &(x, y, width, height, picture) in tics.iter().rev() {
            let color = if value <= 0.0 {
                BLACK
            } else if value < inc {
                dimmed(WHITE, value / inc)
            } else {
                WHITE
            };
            self.picture_at(
                [RIGHT_X + x, TOP + y, width, height],
                Side::Right,
                base + picture,
                color,
            );
            value -= inc;
        }
    }

    fn jk2_score(&mut self, readout: &Readout, score_label: &str) {
        if readout.power_duel {
            return;
        }
        let Some(run) = self.text() else {
            return;
        };
        let _ = if readout.duel {
            write!(
                run.text,
                "{score_label}: {}/{}",
                readout.score, readout.fraglimit
            )
        } else {
            write!(run.text, "{score_label}: {}", readout.score)
        };
        *run = TextRun {
            text: std::mem::take(&mut run.text),
            x: SCORE_END,
            y: SCORE_Y,
            size: MEDIUM_POINTS * 0.7,
            color: WHITE,
            centred: false,
            side: Some(Side::Right),
            end_limit: Some(SCORE_END),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saberist() -> Readout {
        Readout {
            health: 74,
            max_health: 100,
            armor: 46,
            force: 50,
            weapon: 3,
            saber_style: 2,
            ..Readout::default()
        }
    }

    fn drawn(readout: &Readout) -> Frame {
        let mut frame = Frame::default();
        frame.jk2_status(100, readout, &mut Timers::default(), 200, "Score");
        frame
    }

    fn ammo_arc(frame: &Frame) -> Vec<super::super::Picture> {
        frame.pictures[..frame.picture_count]
            .iter()
            .filter(|p| (100 + AMMO_TICK..100 + SABER_STYLE).contains(&p.picture))
            .copied()
            .collect()
    }

    #[test]
    fn every_picture_decodes() {
        for (file, _) in PICTURES {
            assert!(decode(&format!("{PREFIX}{file}")).is_some(), "{file}");
        }
        assert_eq!(decode("gfx/hud/hudleft"), None);
    }

    #[test]
    fn saber_styles_are_additive_and_the_rest_blend() {
        assert_eq!(additive("sjk:jk2/saber_stylesmed.jpg"), Some(true));
        assert_eq!(additive("sjk:jk2/health.png"), Some(false));
        assert_eq!(additive("gfx/hud/health"), None);
    }

    #[test]
    fn the_medium_style_shows_for_a_saber_and_no_ammunition() {
        let frame = drawn(&saberist());
        let pictures = &frame.pictures[..frame.picture_count];
        assert!(pictures.iter().any(|p| p.picture == 100 + SABER_STYLE + 1));
        assert!(ammo_arc(&frame).is_empty());
    }

    #[test]
    fn half_the_force_lights_half_the_arc_from_the_right() {
        let frame = drawn(&saberist());
        let arc: Vec<_> = frame.pictures[..frame.picture_count]
            .iter()
            .filter(|p| (100 + FORCE_TICK..100 + AMMO_TICK).contains(&p.picture))
            .collect();
        assert_eq!(arc.len(), TICS);
        assert_eq!(arc[0].color, WHITE);
        assert_eq!(arc[0].rect[2], -20.0);
        assert_eq!(arc[TICS - 1].color, BLACK);
    }

    #[test]
    fn a_blaster_draws_its_count_and_keeps_to_the_right() {
        let mut readout = saberist();
        readout.weapon = 5;
        readout.ammo[2] = 150;
        let frame = drawn(&readout);
        let digits = frame.pictures[..frame.picture_count]
            .iter()
            .filter(|p| p.picture >= 200)
            .count();
        assert_eq!(digits, 2 + 2 + 3);
        let arc = ammo_arc(&frame);
        assert_eq!(arc.len(), TICS);
        assert!(arc.iter().all(|p| p.side == Side::Right));
        assert_eq!(arc[0].color, WHITE);
        assert_eq!(arc[TICS - 1].color, BLACK);
    }

    #[test]
    fn the_pistol_shows_dashes_and_a_full_arc() {
        let mut readout = saberist();
        readout.weapon = 2;
        let frame = drawn(&readout);
        assert!(
            frame.texts[..frame.text_count]
                .iter()
                .any(|run| run.text == "--")
        );
        assert!(ammo_arc(&frame).iter().all(|p| p.color == WHITE));
    }

    #[test]
    fn the_health_tic_blinks_when_nearly_dead() {
        let mut readout = saberist();
        readout.health = 10;
        readout.armor = 0;
        let mut frame = Frame::default();
        let mut timers = Timers::default();
        let mut seen = [false; 2];
        for step in 0..6 {
            readout.time = 1_000 + step * 101;
            frame.clear();
            frame.jk2_status(100, &readout, &mut timers, 200, "Score");
            let lit = frame.pictures[..frame.picture_count]
                .iter()
                .any(|p| p.picture == 100 + HEALTH_TIC);
            seen[usize::from(lit)] = true;
        }
        assert_eq!(seen, [true, true]);
    }

    fn selector_pictures(frame: &Frame) -> Vec<super::super::Picture> {
        frame.pictures[..frame.picture_count]
            .iter()
            .filter(|p| (100 + PRONG_OFF..100 + PICTURE_COUNT as u16).contains(&p.picture))
            .copied()
            .collect()
    }

    fn with_selector(selector: Option<(Selector, i32)>, time: i32, timers: &mut Timers) -> Frame {
        let readout = Readout {
            selector,
            time,
            ..saberist()
        };
        let mut frame = Frame::default();
        frame.jk2_status(100, &readout, timers, 200, "Score");
        frame
    }

    #[test]
    fn the_unlit_prongs_wait_at_the_ends_with_no_selector() {
        let frame = with_selector(None, 1000, &mut Timers::default());
        let pictures = selector_pictures(&frame);
        assert_eq!(pictures.len(), 2);
        assert!(pictures.iter().all(|p| p.picture == 100 + PRONG_OFF));
        assert_eq!(pictures[0].rect, [67.0, 400.0, 40.0, 80.0]);
        assert_eq!(pictures[0].side, Side::Left);
        assert_eq!(pictures[1].rect, [574.0, 400.0, -40.0, 80.0]);
        assert_eq!(pictures[1].side, Side::Right);
    }

    #[test]
    fn the_force_selector_opens_its_own_background_over_130_ms() {
        let mut timers = Timers::default();
        let frame = with_selector(Some((Selector::Force, 65)), 1065, &mut timers);
        let pictures = selector_pictures(&frame);
        let force_background = 100 + BACKGROUND + 1;
        let halves: Vec<_> = pictures
            .iter()
            .filter(|p| p.picture == force_background)
            .collect();
        assert_eq!(halves.len(), 2);
        assert!(halves.iter().all(|p| p.side == Side::Stretch));
        assert_eq!(halves[0].rect, [90.0, 440.0, 460.0, -30.0]);
        assert_eq!(halves[1].rect, [90.0, 438.0, 460.0, 30.0]);
        let prongs: Vec<_> = pictures
            .iter()
            .filter(|p| p.picture == 100 + PRONG_ON + 1)
            .collect();
        assert_eq!(prongs.len(), 2);
        assert_eq!(prongs[0].rect[0], 71.0);
        assert_eq!(prongs[1].rect[0], 570.0);
    }

    #[test]
    fn the_background_closes_after_the_selector_goes() {
        let mut timers = Timers::default();
        let open = with_selector(Some((Selector::Inventory, 400)), 1400, &mut timers);
        let inventory_background = 100 + BACKGROUND + 2;
        assert!(
            selector_pictures(&open)
                .iter()
                .any(|p| p.picture == inventory_background && p.rect[3] == 60.0)
        );
        // The selector stops showing at 2000; half closed 65 ms later.
        let _ = with_selector(None, 2000, &mut timers);
        let closing = with_selector(None, 2065, &mut timers);
        let pictures = selector_pictures(&closing);
        assert!(
            pictures
                .iter()
                .any(|p| p.picture == inventory_background && p.rect[3] == 30.0)
        );
        assert!(
            pictures
                .iter()
                .any(|p| p.picture == 100 + PRONG_OFF && p.rect[0] == 71.0)
        );
        let closed = with_selector(None, 2200, &mut timers);
        let pictures = selector_pictures(&closed);
        assert_eq!(pictures.len(), 2);
        assert_eq!(pictures[0].rect[0], 67.0);
    }

    #[test]
    fn a_new_selection_while_open_keeps_it_open() {
        let mut timers = Timers::default();
        let _ = with_selector(Some((Selector::Weapon, 300)), 1300, &mut timers);
        let frame = with_selector(Some((Selector::Force, 0)), 1310, &mut timers);
        assert!(
            selector_pictures(&frame)
                .iter()
                .any(|p| p.picture == 100 + BACKGROUND + 1 && p.rect[3] == 60.0)
        );
    }
}
