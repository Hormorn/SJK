//! The SJK UI's turn of the model (Sol's request, 10/10/2026: "rotate model with
//! mouse click"; `docs/sjk-ui.md`, Character): opened from a game, on every page
//! (Character, Saber and Force), dragging over the model turns it: the live preview's
//! camera goes round it ([`PlayerMenu::sjk_model_preview`]), and dragging with the right
//! button up or down brings it nearer or further (akr's idea, 09/11/2026). The menu map's
//! stage itself never turns (Creyon's tests, 12/10/2026: a raised blade turned end on to
//! the stage's camera looked far too long): there the first drag lifts the model off the
//! stage into that same preview, until the screen closes. The Saber page's shot over the
//! menu map looks at the thrown saber, so nothing turns there. The turn eases to where it
//! is sent (the Cape category turns the model's back to the camera), holds from page to
//! page, and faces front when the screen opens again. Under the model's feet, in the look
//! of the screen's keys, "Drag" and "Right drag" in key caps say what they do, on every
//! page. The hat and cape picker that lived here is the creator's Hat and Cape
//! categories now ([`super::sjk_creator`]).

use super::*;
use crate::menu::sjk::Frame;
use sjk_ui::UiEventKind;

/// The area over the model that turns it when dragged.
pub(super) const TURN_TOKEN: u16 = 2_400;
/// Degrees the model turns for a window pixel dragged (a full turn is about two
/// widths of the area at 1080 lines).
const DEGREES_PER_PIXEL: f32 = 0.45;
/// How fast an eased turn closes on where it is sent (per second).
const EASE: f32 = 9.0;
/// How much nearer a window pixel of right-button drag brings the model (natural log of
/// the framing's scale; 200 pixels is a little over twice as near).
const ZOOM_PER_PIXEL: f32 = 0.004;
/// The framing's scale at the nearest (the preview camera stops at a room of 0.5, and
/// the screen's room is 1.35 times this) and at the furthest.
const NEAREST: f32 = 0.4;
const FURTHEST: f32 = 1.3;

/// How far the model is turned from facing the camera, in degrees.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Turn {
    shown: f32,
    target: f32,
    /// Menu clock seconds of the last frame, for the easing.
    at: Option<f64>,
    /// How much nearer the model is brought (natural log of the framing's scale, 0 as
    /// it stands).
    nearer: f32,
    /// The pointer's last press was the right button's: a drag zooms.
    pub(super) zooming: bool,
    /// Dragged since the screen opened: on the menu map, the model has left the stage.
    lifted: bool,
}

impl Turn {
    /// Turn by a drag of `pixels` (window pixels, rightwards positive): at once.
    pub(super) fn drag(&mut self, pixels: f32) {
        self.target += pixels * DEGREES_PER_PIXEL;
        self.shown = self.target;
        self.lifted = true;
        self.keep_small();
    }

    /// Zoom by a right-button drag of `pixels` (window pixels, downwards positive, so
    /// upwards brings it nearer).
    pub(super) fn zoom(&mut self, pixels: f32) {
        self.nearer = (self.nearer - pixels * ZOOM_PER_PIXEL).clamp(-FURTHEST.ln(), -NEAREST.ln());
        self.lifted = true;
    }

    /// The framing's scale: below 1 nearer, above 1 further.
    pub(super) fn framing(self) -> f32 {
        (-self.nearer).exp()
    }

    /// Ease to `degrees` (0 faces the camera, 180 shows the back), the short way.
    pub(super) fn face(&mut self, degrees: f32) {
        self.target += (degrees - self.target + 180.0).rem_euclid(360.0) - 180.0;
    }

    /// Ease towards the target over the time since the last frame (`seconds`, the
    /// menu clock).
    pub(super) fn advance(&mut self, seconds: f64) {
        let elapsed = self.at.map_or(0.0, |at| (seconds - at).clamp(0.0, 0.25)) as f32;
        self.at = Some(seconds);
        let gap = self.target - self.shown;
        self.shown = if gap.abs() < 0.05 {
            self.target
        } else {
            self.shown + gap * (1.0 - (-EASE * elapsed).exp())
        };
        self.keep_small();
    }

    /// Face the camera at once.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn degrees(self) -> f32 {
        self.shown
    }

    /// Keep both angles within a turn of zero, together.
    fn keep_small(&mut self) {
        let turns = (self.target / 360.0).trunc();
        if turns != 0.0 {
            self.target -= turns * 360.0;
            self.shown -= turns * 360.0;
        }
    }
}

impl PlayerMenu {
    /// Whether the screen turns the model with the pointer: the SJK UI's, on each of its
    /// pages opened from a game; over the menu map on the pages whose shot shows the model.
    pub(super) fn model_turns(&self) -> bool {
        self.is_sjk()
            && (self.return_target == ReturnTarget::InGame || self.page != ProfilePage::Saber)
    }

    /// Over the menu map, whether a drag has taken the model off the stage into the live
    /// preview ([`Self::sjk_model_preview`]).
    pub(super) fn model_lifted(&self) -> bool {
        self.turn.lifted && self.return_target == ReturnTarget::MainMenu && self.model_turns()
    }

    /// The model's turn where the page turns it ([`Self::model_turns`]), in degrees;
    /// 0 elsewhere.
    pub(crate) fn model_turn(&self) -> f32 {
        if self.model_turns() {
            self.turn.degrees()
        } else {
            0.0
        }
    }

    /// The pointer on the turn area; `None` leaves it to the screen.
    pub(super) fn outfit_pointer(
        &mut self,
        kind: UiEventKind,
        token: u16,
        delta: Option<sjk_ui::Vec2>,
    ) -> Option<PlayerMenuResult> {
        if token != TURN_TOKEN {
            return None;
        }
        if kind == UiEventKind::Drag
            && let Some(delta) = delta
        {
            if self.turn.zooming {
                self.turn.zoom(delta.y);
            } else {
                self.turn.drag(delta.x);
            }
        }
        Some(PlayerMenuResult::None)
    }

    /// Under the model's feet, centred on `centre_x` from `top` (frame pixels), on every
    /// page that turns it: the keys' look of the screen ([`crate::menu::sjk::key_hint`]),
    /// "Drag" and "Right drag" in key caps and what they do.
    pub(super) fn sjk_turn_hint(&mut self, frame: &Frame, centre_x: f32, top: f32) {
        if !self.model_turns() {
            return;
        }
        let s = frame.s;
        let (turn, turn_action) = (&["Drag"][..], "turn the model");
        let (zoom, zoom_action) = (&["Right drag"][..], "zoom");
        let gap = 16.0 * s;
        let width = crate::menu::sjk::key_hint_width(turn, turn_action, s)
            + gap
            + crate::menu::sjk::key_hint_width(zoom, zoom_action, s);
        let [centre, y] = frame.point(centre_x, top);
        let end = crate::menu::sjk::key_hint(
            &mut self.canvas,
            turn,
            turn_action,
            centre - width * 0.5,
            y,
            s,
        );
        crate::menu::sjk::key_hint(&mut self.canvas, zoom, zoom_action, end + gap, y, s);
    }

    /// Register the turn area over the model (before everything else, so any
    /// control drawn over it keeps the pointer).
    pub(super) fn outfit_turn_area(&mut self, frame: &Frame, area: [f32; 4]) {
        let [x, y, width, height] = area;
        self.canvas
            .hit_region(TURN_TOKEN, frame.rect(x, y, width, height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::DrawCommand;

    #[test]
    fn a_drag_turns_at_once_and_a_face_eases_the_short_way() {
        let mut turn = Turn::default();
        turn.drag(100.0);
        assert!((turn.degrees() - 45.0).abs() < 1e-3);
        turn.drag(-100.0);
        assert!(turn.degrees().abs() < 1e-3);
        // From 170 the back (180) is ten degrees on, the front 170 back.
        turn.drag(170.0 / DEGREES_PER_PIXEL);
        turn.face(180.0);
        turn.advance(0.0);
        turn.advance(0.2);
        assert!(
            (170.0..=180.0).contains(&turn.degrees()),
            "{}",
            turn.degrees()
        );
        for frame in 1..60 {
            turn.advance(0.2 + f64::from(frame) / 60.0);
        }
        assert!((turn.degrees() - 180.0).abs() < 0.1, "{}", turn.degrees());
        // From -170 the back is -180, not 180 the long way.
        turn.reset();
        turn.drag(-170.0 / DEGREES_PER_PIXEL);
        turn.face(180.0);
        for frame in 0..120 {
            turn.advance(f64::from(frame) / 60.0);
        }
        assert!((turn.degrees() + 180.0).abs() < 0.1, "{}", turn.degrees());
    }

    fn draw(menu: &mut PlayerMenu) {
        let font = crate::text::load_modern(1.0, None).unwrap().font;
        let mut vertices = Vec::new();
        menu.append_sjk(
            crate::menu::sjk::TextTarget::Inter(&mut vertices, &font),
            [1920.0, 1080.0],
            1.0,
        );
    }

    /// Whether the last frame drew `wanted`.
    fn shows(menu: &PlayerMenu, wanted: &str) -> bool {
        menu.canvas.draw_list().commands().iter().any(|command| {
            matches!(command, DrawCommand::Text { text, .. }
                if menu.canvas.stored_text(*text) == wanted)
        })
    }

    fn centre(menu: &PlayerMenu, token: u16) -> sjk_ui::Vec2 {
        let rect = menu.canvas.rect_for(token).expect("the target");
        sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }

    /// The SJK UI's Character page, with two hats and a cape installed.
    fn character_page() -> (PlayerMenu, ViewerConsole, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "cosmetics",
            [
                ("models/cosmetics/hats/tophat.md3", b"x".to_vec()),
                ("models/cosmetics/hats/santahat.md3", b"x".to_vec()),
                ("models/cosmetics/capes/royalcape.md3", b"x".to_vec()),
            ],
        )
        .unwrap();
        let mut menu = PlayerMenu::new();
        menu.set_sjk(true);
        menu.cosmetics
            .open(&console, Some(&std::sync::Arc::new(vfs)));
        menu.return_target = ReturnTarget::InGame;
        menu.set_page(ProfilePage::Character);
        draw(&mut menu);
        (menu, console, directory)
    }

    /// Opened from a game, dragging over the model turns it as far as the pointer went,
    /// on every page, the preview camera going round it the other way, and the hint under
    /// the model stays.
    #[test]
    fn dragging_over_the_model_turns_it() {
        let (mut menu, mut console, _directory) = character_page();
        let drag = |menu: &mut PlayerMenu, console: &mut ViewerConsole, pixels: f32| {
            let start = centre(menu, TURN_TOKEN);
            let end = sjk_ui::Vec2::new(start.x + pixels, start.y);
            let button = sjk_ui::PointerButton::Primary;
            for event in [
                sjk_ui::InputEvent::PointerMove(start),
                sjk_ui::InputEvent::PointerPress {
                    position: start,
                    button,
                },
                sjk_ui::InputEvent::PointerMove(end),
                sjk_ui::InputEvent::PointerRelease {
                    position: end,
                    button,
                },
            ] {
                let _ = menu.handle_pointer(event, console);
            }
        };
        assert!(shows(&menu, "turn the model"), "the hint under the model");
        drag(&mut menu, &mut console, 100.0);
        assert!(
            (menu.model_turn() - 45.0).abs() < 0.01,
            "{}",
            menu.model_turn()
        );
        assert_eq!(menu.selected, 0, "the drag chose no row");
        let angle = menu.sjk_model_preview().and_then(|preview| preview.angle);
        assert_eq!(angle, Some(-28.0 - 45.0));
        draw(&mut menu);
        assert!(shows(&menu, "turn the model"), "it stays once dragged");
        // The Saber and Force pages turn it too, from where it stands.
        for (page, pixels, turned) in [
            (ProfilePage::Saber, -40.0, 27.0),
            (ProfilePage::Force, -60.0, 0.0),
        ] {
            menu.set_page(page);
            draw(&mut menu);
            assert!(shows(&menu, "turn the model"), "{page:?}");
            drag(&mut menu, &mut console, pixels);
            assert!(
                (menu.model_turn() - turned).abs() < 0.01,
                "{page:?}: {}",
                menu.model_turn()
            );
        }
    }

    /// A drag on the model with `button`, `by` window pixels.
    fn drag_with(
        menu: &mut PlayerMenu,
        console: &mut ViewerConsole,
        button: sjk_ui::PointerButton,
        by: sjk_ui::Vec2,
    ) {
        let start = centre(menu, TURN_TOKEN);
        let end = sjk_ui::Vec2::new(start.x + by.x, start.y + by.y);
        for event in [
            sjk_ui::InputEvent::PointerMove(start),
            sjk_ui::InputEvent::PointerPress {
                position: start,
                button,
            },
            sjk_ui::InputEvent::PointerMove(end),
            sjk_ui::InputEvent::PointerRelease {
                position: end,
                button,
            },
        ] {
            let _ = menu.handle_pointer(event, console);
        }
    }

    /// Over the menu map the model stands on the stage until a drag lifts it into the
    /// live preview; the Saber page's shot lets it go; opening the screen again puts it
    /// back on the stage.
    #[test]
    fn a_drag_over_the_menu_map_lifts_the_model_off_the_stage() {
        let (mut menu, mut console, _directory) = character_page();
        menu.open(&console, ReturnTarget::MainMenu);
        draw(&mut menu);
        assert!(shows(&menu, "turn the model"), "the hint over the menu map");
        assert!(menu.sjk_model_preview().is_none(), "still on the stage");
        let primary = sjk_ui::PointerButton::Primary;
        drag_with(
            &mut menu,
            &mut console,
            primary,
            sjk_ui::Vec2::new(100.0, 0.0),
        );
        let preview = menu.sjk_model_preview().expect("lifted into the preview");
        assert_eq!(preview.angle, Some(-28.0 - 45.0));
        menu.set_page(ProfilePage::Saber);
        assert!(menu.sjk_model_preview().is_none(), "the saber's shot");
        draw(&mut menu);
        assert!(menu.canvas.rect_for(TURN_TOKEN).is_none());
        assert!(!shows(&menu, "turn the model"));
        menu.open(&console, ReturnTarget::MainMenu);
        assert!(menu.sjk_model_preview().is_none(), "reopened on the stage");
        assert_eq!(menu.model_turn(), 0.0);
    }

    /// Dragging with the right button up brings the model nearer without turning it,
    /// down takes it further, and both stop at their ends.
    #[test]
    fn a_right_drag_zooms_and_does_not_turn() {
        let (mut menu, mut console, _directory) = character_page();
        assert!(shows(&menu, "zoom"), "the hint");
        let as_framed = menu.sjk_model_preview().unwrap().room;
        let secondary = sjk_ui::PointerButton::Secondary;
        drag_with(
            &mut menu,
            &mut console,
            secondary,
            sjk_ui::Vec2::new(80.0, -100.0),
        );
        let nearer = menu.sjk_model_preview().unwrap().room;
        assert!(nearer < as_framed, "{nearer} of {as_framed}");
        assert_eq!(menu.model_turn(), 0.0, "the right button does not turn");
        drag_with(
            &mut menu,
            &mut console,
            secondary,
            sjk_ui::Vec2::new(0.0, -5_000.0),
        );
        let nearest = menu.sjk_model_preview().unwrap().room;
        assert!((nearest - as_framed * NEAREST).abs() < 1e-3, "{nearest}");
        assert!(nearest >= 0.5, "the preview camera's own limit");
        drag_with(
            &mut menu,
            &mut console,
            secondary,
            sjk_ui::Vec2::new(0.0, 5_000.0),
        );
        let furthest = menu.sjk_model_preview().unwrap().room;
        assert!((furthest - as_framed * FURTHEST).abs() < 1e-3, "{furthest}");
        // The left button still turns afterwards.
        let primary = sjk_ui::PointerButton::Primary;
        drag_with(
            &mut menu,
            &mut console,
            primary,
            sjk_ui::Vec2::new(100.0, 0.0),
        );
        assert!((menu.model_turn() - 45.0).abs() < 0.01);
    }

    /// The Hat row opens the picker: None and the hats as chips, a click wears one,
    /// Tab shows the capes with the model's back to the camera, Escape closes it.
    #[test]
    fn whole_turns_are_dropped() {
        let mut turn = Turn::default();
        turn.drag(800.0 / DEGREES_PER_PIXEL);
        assert!((turn.degrees() - 80.0).abs() < 1e-2, "{}", turn.degrees());
    }

    // The tokens stay apart, and clear of the screen's others: the hilts end at 1,700
    // (`sjk_view.rs`), the classic pages' colour sliders below 2,000 (`saber_rgb.rs`).
    const _: () = assert!(crate::profile_hub::TOKEN + 64 <= TURN_TOKEN);
    const _: () = assert!(TURN_TOKEN < super::sjk_creator::CATEGORY_BASE);
}
