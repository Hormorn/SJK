//! The HUD edit mode (`hud_edit`, Settings > HUD "Move HUD elements"; Lex's idea,
//! #ideas 29/10/2026): over the game, every movable HUD element that shows has an
//! outline and its name; the player drags one with the mouse, right-clicks one (or
//! points at it and presses Delete) to put it back, "Reset all" puts them all back, and
//! Done, Escape or Enter leave. Where the elements go is kept as
//! [`crate::hud::placement`] says: in their own position cvars when they have some, in
//! `cg_hudOffsets` otherwise. Only what the HUD draws at the moment can be moved: the
//! vote panel during a vote, the lagometer with `cg_lagometer` on, and so on.

use crate::console::ViewerConsole;
use crate::hud::placement::{Binding, Bounds, Element, OFFSETS_CVAR, Offsets, Unit};
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, kit, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{
    DrawCommand, FontWeight, InputEvent, PointerButton, Rect, TextAlign, UiEventKind, Vec2,
};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

pub(crate) const COMMAND: &str = "hud_edit";
pub(crate) const HELP: &str =
    "Move the HUD's elements with the mouse; hud_edit reset [element|all] puts them back";

const DONE_TOKEN: u16 = 1;
const RESET_TOKEN: u16 = 2;

/// The panel's place and size in the 1920x1080 frame: under the top's middle, clear
/// of the match timer.
const PANEL: [f32; 4] = [660.0, 300.0, 600.0, 150.0];
const BUTTON_HEIGHT: f32 = 42.0;
const DONE_WIDTH: f32 = 130.0;
const RESET_WIDTH: f32 = 160.0;

/// What the player did that the game must act on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Done,
}

/// Where a dragged element started: the pointer, and its offset or cvars' values.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    element: Element,
    from: [f32; 2],
    start: [f32; 2],
}

pub(crate) struct HudEdit {
    open: bool,
    /// Opened from the menus, which come back when it closes.
    from_menu: bool,
    /// The boxes shown this frame, which the pointer picks from.
    boxes: Bounds,
    hovered: Option<Element>,
    drag: Option<Drag>,
    pointer: [f32; 2],
    ui: MenuCanvas,
}

impl Default for HudEdit {
    fn default() -> Self {
        Self {
            open: false,
            from_menu: false,
            boxes: Bounds::default(),
            hovered: None,
            drag: None,
            pointer: [0.0; 2],
            ui: MenuCanvas::with_capacities(32, 900, 220),
        }
    }
}

/// The value an element's position starts a drag from: its offset, or its cvars.
fn start_of(console: &ViewerConsole, element: Element) -> [f32; 2] {
    match element.binding() {
        Binding::Free => Offsets::read(Some(console)).get(element),
        Binding::Cvars(axes) => axes.map(|axis| cvar_number(console, axis.cvar)),
    }
}

fn cvar_number(console: &ViewerConsole, name: &str) -> f32 {
    match console.cvar(name) {
        Some(sjk_shell::CvarValue::Float(value)) if value.is_finite() => *value as f32,
        Some(sjk_shell::CvarValue::Integer(value)) => *value as f32,
        _ => 0.0,
    }
}

/// The values that put `element` `pixels` away from where `start` had it.
pub(crate) fn moved(
    element: Element,
    start: [f32; 2],
    pixels: [f32; 2],
    viewport: [f32; 2],
) -> [f32; 2] {
    match element.binding() {
        Binding::Free => {
            let unit = Unit::Height480.pixels(viewport).max(1e-3);
            [start[0] + pixels[0] / unit, start[1] + pixels[1] / unit]
        }
        // Whole units: some of these cvars are read as integers.
        Binding::Cvars(axes) => std::array::from_fn(|axis| {
            axes[axis]
                .moved(start[axis], pixels[axis], viewport)
                .round()
        }),
    }
}

/// Keep `element` at `values` ([`start_of`]'s kind).
fn store(console: &mut ViewerConsole, element: Element, values: [f32; 2]) {
    match element.binding() {
        Binding::Free => {
            let mut offsets = Offsets::read(Some(console));
            offsets.set(element, values);
            console.set_cvar(OFFSETS_CVAR, &offsets.format());
        }
        Binding::Cvars(axes) => {
            for (axis, value) in axes.iter().zip(values) {
                if cvar_number(console, axis.cvar) != value {
                    console.set_cvar(axis.cvar, &format!("{value}"));
                }
            }
        }
    }
}

/// Put `element` back where its HUD puts it.
pub(crate) fn reset(console: &mut ViewerConsole, element: Element) {
    match element.binding() {
        Binding::Free => store(console, element, [0.0; 2]),
        Binding::Cvars(axes) => {
            for axis in axes {
                if let Some(default) = console.cvar_default(axis.cvar).map(|value| value.as_text())
                {
                    console.set_cvar(axis.cvar, &default);
                }
            }
        }
    }
}

/// Put every element back.
pub(crate) fn reset_all(console: &mut ViewerConsole) {
    for element in Element::ALL {
        if element.binding() != Binding::Free {
            reset(console, element);
        }
    }
    console.set_cvar(OFFSETS_CVAR, "");
}

/// The element whose box holds `point`, the smallest when boxes overlap (the radial
/// look's numbers sit inside its arcs' boxes).
fn picked(boxes: &Bounds, point: [f32; 2]) -> Option<Element> {
    let point = Vec2::new(point[0], point[1]);
    boxes
        .iter()
        .filter(|(_, rect)| rect.contains(point))
        .min_by(|(_, a), (_, b)| (a.width * a.height).total_cmp(&(b.width * b.height)))
        .map(|(element, _)| element)
}

impl HudEdit {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    fn open(&mut self, from_menu: bool) {
        self.open = true;
        self.from_menu = from_menu;
        self.drag = None;
        self.hovered = None;
    }

    /// Leave the mode; whether the menus should come back.
    fn close(&mut self) -> bool {
        self.open = false;
        self.drag = None;
        self.hovered = None;
        std::mem::take(&mut self.from_menu)
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Escape and Enter leave, Delete or Backspace put back the element pointed at;
    /// every other key is swallowed, so nothing moves the player while editing.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        console: &mut ViewerConsole,
    ) -> Option<Action> {
        if event.state != ElementState::Pressed || event.repeat {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };
        match code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => Some(Action::Done),
            KeyCode::Delete | KeyCode::Backspace => {
                if let Some(element) = self.hovered.filter(|_| self.drag.is_none()) {
                    reset(console, element);
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn handle_pointer(
        &mut self,
        event: InputEvent,
        console: &mut ViewerConsole,
        viewport: [f32; 2],
    ) -> Option<Action> {
        let on_panel = self.ui.hovered().is_some();
        let routed = self.ui.pointer(event);
        if let Some(routed) = routed
            && routed.kind == UiEventKind::Activate
        {
            match routed.token {
                Some(DONE_TOKEN) => return Some(Action::Done),
                Some(RESET_TOKEN) => {
                    reset_all(console);
                    return None;
                }
                _ => {}
            }
        }
        match event {
            InputEvent::PointerMove(position) => {
                self.pointer = [position.x, position.y];
                if let Some(drag) = self.drag {
                    let pixels = [position.x - drag.from[0], position.y - drag.from[1]];
                    store(
                        console,
                        drag.element,
                        moved(drag.element, drag.start, pixels, viewport),
                    );
                } else {
                    self.hovered = picked(&self.boxes, self.pointer);
                }
            }
            InputEvent::PointerPress { position, button } if !on_panel => {
                self.pointer = [position.x, position.y];
                let element = picked(&self.boxes, self.pointer);
                match (button, element) {
                    (PointerButton::Primary, Some(element)) => {
                        self.drag = Some(Drag {
                            element,
                            from: self.pointer,
                            start: start_of(console, element),
                        });
                    }
                    (PointerButton::Secondary, Some(element)) => reset(console, element),
                    _ => {}
                }
            }
            InputEvent::PointerRelease { .. } | InputEvent::PointerLeave => {
                self.drag = None;
            }
            _ => {}
        }
        None
    }

    /// Lay out the outlines, their names and the panel over the game, for `boxes`,
    /// what the HUD drew this frame.
    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2], boxes: Bounds) {
        self.boxes = boxes;
        if let Some(drag) = self.drag
            && boxes.get(drag.element).is_none()
        {
            // What was dragged went away (a vote ended): nothing to move.
            self.drag = None;
        }
        if self
            .hovered
            .is_some_and(|element| boxes.get(element).is_none())
        {
            self.hovered = None;
        }
        let frame = Frame::new(viewport);
        let s = frame.s;
        let active = self.drag.map(|drag| drag.element).or(self.hovered);
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        for (element, rect) in boxes.iter() {
            let lit = active == Some(element);
            let pad = 4.0 * s;
            let rect = Rect::new(
                rect.x - pad,
                rect.y - pad,
                rect.width + pad * 2.0,
                rect.height + pad * 2.0,
            );
            let draw = ui.draw_list_mut();
            let _ = draw.push(DrawCommand::RoundedRect {
                rect,
                radius: 6.0 * s,
                color: color::alpha(
                    if lit { color::GOLD } else { color::HOLO },
                    if lit { 0.16 } else { 0.06 },
                ),
            });
            let _ = draw.push(DrawCommand::Border {
                rect,
                radius: 6.0 * s,
                width: if lit { 2.0 } else { 1.5 } * s,
                color: color::alpha(
                    if lit { color::GOLD_BRIGHT } else { color::HOLO },
                    if lit { 1.0 } else { 0.7 },
                ),
            });
            // The name over the box, or inside it at the top of the screen.
            let line = 22.0 * s;
            let y = if rect.y >= line {
                rect.y - line
            } else {
                rect.y + 2.0 * s
            };
            text(
                ui,
                TextFamily::Display,
                format_args!("{}", element.label()),
                Rect::new(rect.x, y, rect.width.max(160.0 * s), line),
                16.0 * s,
                if lit { color::GOLD_BRIGHT } else { color::TEXT },
                FontWeight::Semibold,
                TextAlign::Start,
            );
        }
        let [x, y, width, height] = PANEL;
        kit::card(ui, &frame, PANEL);
        let margin = 28.0;
        text(
            ui,
            TextFamily::Display,
            format_args!("Move the HUD"),
            frame.rect(x + margin, y + 18.0, width - margin * 2.0, 34.0),
            26.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!(
                "{}",
                if boxes.iter().next().is_some() {
                    "Drag an outlined element. Right-click one to put it back."
                } else {
                    "Nothing movable shows right now."
                }
            ),
            frame.rect(x + margin, y + 56.0, width - margin * 2.0, 28.0),
            17.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let foot = y + height - BUTTON_HEIGHT - 20.0;
        let right = x + width - margin;
        kit::button(
            ui,
            &frame,
            [right - DONE_WIDTH, foot, DONE_WIDTH, BUTTON_HEIGHT],
            "Done",
            true,
            true,
            false,
            DONE_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [
                right - DONE_WIDTH - 14.0 - RESET_WIDTH,
                foot,
                RESET_WIDTH,
                BUTTON_HEIGHT,
            ],
            "Reset all",
            false,
            true,
            false,
            RESET_TOKEN,
        );
        let hints: [(&[&str], &str); 2] = [(&["Esc"], "done"), (&["Del"], "reset")];
        let [mut hint_x, hint_y] = frame.point(x + margin, foot + (BUTTON_HEIGHT - 24.0) * 0.5);
        for (keys, label) in hints {
            hint_x = key_hint(ui, keys, label, hint_x, hint_y, s) + 16.0 * s;
        }
        ui.finish(DONE_TOKEN);
        target.append(&self.ui, viewport);
    }
}

impl crate::GpuState {
    /// `hud_edit` opens the mode over the game; `hud_edit reset <element>` or
    /// `hud_edit reset all` (or `hud_edit reset`) puts elements back.
    pub(crate) fn hud_edit_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        match args {
            [] => {}
            [word, rest @ ..] if word.eq_ignore_ascii_case("reset") => {
                let console = self.console.as_mut().ok_or("No console")?;
                return match rest {
                    [] => {
                        reset_all(console);
                        Ok(vec!["Every HUD element is back in its place".into()])
                    }
                    [name] if name.eq_ignore_ascii_case("all") => {
                        reset_all(console);
                        Ok(vec!["Every HUD element is back in its place".into()])
                    }
                    [name] => {
                        let element = Element::from_id(name).ok_or_else(|| {
                            let names: Vec<_> = Element::ALL.iter().map(|e| e.id()).collect();
                            format!("No HUD element {name}; one of: {}", names.join(" "))
                        })?;
                        reset(console, element);
                        Ok(vec![format!("{} is back in its place", element.label())])
                    }
                    _ => Err(format!("usage: {COMMAND} [reset [element|all]]")),
                };
            }
            _ => return Err(format!("usage: {COMMAND} [reset [element|all]]")),
        }
        if self.live_session.is_none() && self.demo_session.is_none() {
            return Err("Join a game first: the HUD shows only in a match".into());
        }
        if let Some(console) = &mut self.console {
            console.set_open(false);
        }
        let mut from_menu = std::mem::take(&mut self.game_menu);
        if let Some(menu) = &mut self.client_menu
            && menu.is_visible()
        {
            // Settings opened from the game menu close to the game; the game menu
            // comes back when the mode closes.
            from_menu |= menu.leave_settings_for_game();
        }
        self.hud_edit.open(from_menu);
        self.release_pointer();
        self.sync_cursor_policy();
        Ok(Vec::new())
    }

    /// Whether the mode shows this frame: it closes by itself when the match does.
    pub(crate) fn prepare_hud_edit(&mut self) -> bool {
        if self.hud_edit.is_open() && self.live_session.is_none() && self.demo_session.is_none() {
            self.hud_edit.close();
            self.sync_cursor_policy();
        }
        self.hud_edit.is_open()
    }

    /// Draw the outlines and the panel, for what the HUD drew this frame.
    pub(crate) fn append_hud_edit(&mut self, viewport: [f32; 2]) {
        let mut boxes = self.hud.element_bounds;
        boxes.add(
            Element::Chat,
            Some(crate::chat::options::box_rect(
                self.console.as_ref(),
                viewport,
            )),
        );
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.hud_edit.append(target, viewport, boxes);
    }

    pub(crate) fn hud_edit_key(&mut self, event: &KeyEvent) {
        let Some(console) = &mut self.console else {
            return;
        };
        let action = self.hud_edit.handle_key(event, console);
        self.hud_edit_action(action);
    }

    pub(crate) fn hud_edit_pointer(&mut self, event: InputEvent) {
        let viewport = [
            self.configuration.width as f32,
            self.configuration.height as f32,
        ];
        let Some(console) = &mut self.console else {
            return;
        };
        let action = self.hud_edit.handle_pointer(event, console, viewport);
        self.hud_edit_action(action);
    }

    fn hud_edit_action(&mut self, action: Option<Action>) {
        if action == Some(Action::Done) {
            if self.hud_edit.close() && self.live_session.is_some() {
                self.game_menu = true;
                self.game_menu_page = crate::ingame_menu::Page::Main;
                self.game_menu_row = 0;
            }
            self.sync_cursor_policy();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    /// A drag keeps a free element's offset in `cg_hudOffsets` and a cvar-bound one's
    /// place in its own cvars; resets put them back.
    #[test]
    fn dragging_keeps_the_place_and_reset_puts_it_back() {
        let (_directory, mut console) = console();
        let viewport = [1_920.0, 1_080.0];
        // 45 pixels right and 90 up at 1080 lines is 20 units right, 40 up.
        let start = start_of(&console, Element::Vitals);
        assert_eq!(start, [0.0, 0.0]);
        store(
            &mut console,
            Element::Vitals,
            moved(Element::Vitals, start, [45.0, -90.0], viewport),
        );
        assert_eq!(console.text_cvar(OFFSETS_CVAR).unwrap(), "vitals:20,-40");
        assert_eq!(
            Offsets::read(Some(&console)).get(Element::Vitals),
            [20.0, -40.0]
        );
        // The lagometer is 48 units from the right and 144 from the bottom by default:
        // 45 pixels left and down is 20 units more from the right, 20 less from the bottom.
        let start = start_of(&console, Element::Lagometer);
        assert_eq!(start, [48.0, 144.0]);
        store(
            &mut console,
            Element::Lagometer,
            moved(Element::Lagometer, start, [-45.0, 45.0], viewport),
        );
        assert_eq!(start_of(&console, Element::Lagometer), [68.0, 124.0]);
        // The chat's own cvars.
        let start = start_of(&console, Element::Chat);
        store(
            &mut console,
            Element::Chat,
            moved(Element::Chat, start, [30.0, 0.0], viewport),
        );
        assert_eq!(
            start_of(&console, Element::Chat),
            [start[0] + 10.0, start[1]]
        );
        reset(&mut console, Element::Lagometer);
        assert_eq!(start_of(&console, Element::Lagometer), [48.0, 144.0]);
        store(&mut console, Element::Jetpack, [5.0, 5.0]);
        reset(&mut console, Element::Vitals);
        assert_eq!(console.text_cvar(OFFSETS_CVAR).unwrap(), "jetpack:5,5");
        reset_all(&mut console);
        assert_eq!(console.text_cvar(OFFSETS_CVAR).unwrap(), "");
        assert_eq!(start_of(&console, Element::Chat), start);
    }

    /// The clock (05/11/2026) drags like the speedometer: its centre and bottom in
    /// `cg_clockX`/`cg_clockY`, and a reset puts it back.
    #[test]
    fn the_clock_drags_into_its_own_cvars() {
        let (_directory, mut console) = console();
        let viewport = [1_920.0, 1_080.0];
        let start = start_of(&console, Element::Clock);
        assert_eq!(start, [320.0, 476.0]);
        // 300 pixels left (100 of 640) and 450 up (200 of 480) at 1920x1080.
        store(
            &mut console,
            Element::Clock,
            moved(Element::Clock, start, [-300.0, -450.0], viewport),
        );
        assert_eq!(start_of(&console, Element::Clock), [220.0, 276.0]);
        assert_eq!(Element::from_id("clock"), Some(Element::Clock));
        reset(&mut console, Element::Clock);
        assert_eq!(start_of(&console, Element::Clock), [320.0, 476.0]);
    }

    /// The pointer picks the smallest box it is in.
    #[test]
    fn the_pointer_picks_the_innermost_box() {
        let mut boxes = Bounds::default();
        boxes.add(Element::Vitals, Some(Rect::new(0.0, 0.0, 400.0, 400.0)));
        boxes.add(Element::Weapon, Some(Rect::new(100.0, 100.0, 50.0, 20.0)));
        assert_eq!(picked(&boxes, [110.0, 110.0]), Some(Element::Weapon));
        assert_eq!(picked(&boxes, [10.0, 10.0]), Some(Element::Vitals));
        assert_eq!(picked(&boxes, [500.0, 10.0]), None);
    }
}
