//! The Saber Lab tab's blade-skin editor (`docs/client.md`, "Saber Lab"): the skin the
//! tab shows, opened from the lab's folder ([`crate::blade_editor`]; a skin of the packs
//! copied there first), its sections and fields as rows in the tab's column under the
//! floating saber, the model and the saber showing each change as it is made. Up and
//! Down choose a row, Left and Right move a number by its step (Shift ten, Ctrl a tenth),
//! a drag on its slider sets it, Shift+Right and Shift+Left open and close every section;
//! Enter opens a section or a colour, flips a switch, adds a
//! section or types a number or a path (a digit starts typing too); Delete takes a
//! section or a list's item out, Insert copies an item; A goes to the sections to add;
//! Ctrl+Z and Ctrl+Y undo and redo; Esc is done. Each section says in a few words what
//! it draws ([`crate::blade_editor::about`]).

use super::saber_lab_builder::{GROUPS_TOP, GROUPS_WIDTH, GROUPS_X};
use super::saber_lab_tab::{LabBlade, lab_blades};
use super::view::RIGHT_X;
use super::*;
use crate::blade_editor::{Editor, Row, Value};
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::ordered_json::Step;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// Where the editor stands (frame pixels): its left edge, its width, its heading's line
/// (its status line and rows under it) and how many rows it shows at once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Column {
    x: f32,
    width: f32,
    top: f32,
    rows: usize,
}

impl Column {
    fn right(self) -> f32 {
        self.x + self.width
    }

    fn status_y(self) -> f32 {
        self.top + 32.0
    }

    fn list_top(self) -> f32 {
        self.top + 52.0
    }
}

/// Under the floating saber, on the stage's view; in the groups' place, on the builder's.
const STAGE_COLUMN: Column = Column {
    x: 1_240.0,
    width: RIGHT_X - 1_240.0,
    top: 560.0,
    rows: 13,
};
const BUILD_COLUMN: Column = Column {
    x: GROUPS_X,
    width: GROUPS_WIDTH,
    top: GROUPS_TOP,
    rows: 21,
};
const ROW_HEIGHT: f32 = 29.0;
/// A number's slider and its value, from the right edge.
const SLIDER_WIDTH: f32 = 150.0;
const VALUE_WIDTH: f32 = 86.0;
/// How far a level of rows is set in.
const INDENT: f32 = 18.0;
/// The editor's tokens: each row shown, its value, and the buttons.
const ROW_BASE: u16 = 3_400;
const VALUE_BASE: u16 = 3_430;
const DONE_TOKEN: u16 = 3_022;
const UNDO_TOKEN: u16 = 3_023;
const REDO_TOKEN: u16 = 3_024;
const REVERT_TOKEN: u16 = 3_029;
/// The Equip row's Edit button.
pub(super) const EDIT_TOKEN: u16 = 3_021;
/// The longest number or path typed.
const TYPED_MAX: usize = 63;
/// How far a drag on a number without a range moves it: steps a frame pixel.
const SCRUB: f64 = 0.25;

/// The editor open on the tab, and where it is: its column, the row chosen, the first
/// shown, what is being typed into the chosen row, and a drag on a number.
pub(super) struct LabEdit {
    editor: Editor,
    column: Column,
    row: usize,
    first: usize,
    typing: Option<String>,
    drag: Option<Drag>,
}

impl LabEdit {
    /// The editor the rows show.
    pub(super) fn editor(&self) -> &Editor {
        &self.editor
    }
}

/// A drag on number row `row`: from window x `from`, the number then `value`.
#[derive(Clone, Copy, Debug)]
struct Drag {
    row: usize,
    from: f32,
    value: f64,
}

impl Panel {
    /// The skin the Edit button opens (and the builder builds): the one the Blade row
    /// shows, or the worn one; the stock blade has none.
    pub(super) fn lab_edit_target(&self) -> Option<&'static str> {
        match self.lab_blade() {
            LabBlade::Skin(id) => Some(id),
            LabBlade::Worn => {
                crate::unlockables::blade_skin(self.lab_setting.trim()).map(|skin| skin.id)
            }
            LabBlade::Stock => None,
        }
    }

    /// The Edit button's words, and whether it may be pressed.
    pub(super) fn lab_edit_button(&self) -> (&'static str, bool) {
        match self.lab_edit_target() {
            Some(id) if crate::blade_editor::in_lab(id) => ("Edit", true),
            Some(_) => ("Copy to edit", true),
            None => ("Edit", false),
        }
    }

    /// Open the editor on the skin the tab shows, copying a skin of the packs into the
    /// lab's folder first; why not, if it cannot.
    pub(super) fn open_lab_editor(&mut self) {
        self.lab_edit_problem = None;
        let Some(id) = self.lab_edit_target() else {
            self.lab_edit_problem = Some("The stock blade has no file to edit.".to_owned());
            return;
        };
        let opened = crate::blade_editor::copy_into_lab(id).and_then(|()| Editor::open(id));
        match opened {
            Ok(editor) => {
                // Show the skin edited, whatever the row showed (the worn one).
                if let Some(at) = lab_blades()
                    .iter()
                    .position(|blade| *blade == LabBlade::Skin(id))
                {
                    self.lab_blade = at;
                }
                self.lab_edit = Some(LabEdit {
                    editor,
                    column: STAGE_COLUMN,
                    row: 0,
                    first: 0,
                    typing: None,
                    drag: None,
                });
            }
            Err(why) => self.lab_edit_problem = Some(why),
        }
    }

    /// Open the fields of effect `key` on the builder, in its groups' place: the builder's
    /// editor, that section alone open and at the top.
    pub(super) fn open_lab_fields(&mut self, key: &str) {
        let Some(editor) = self.lab_build.editor.take() else {
            return;
        };
        let mut edit = LabEdit {
            editor,
            column: BUILD_COLUMN,
            row: 0,
            first: 0,
            typing: None,
            drag: None,
        };
        edit.editor.open_all(false);
        if let Some(row) = edit
            .editor
            .rows()
            .into_iter()
            .find(|row| row.section() && row.path == [Step::Key(key.to_owned())])
        {
            edit.editor.toggle_open(&row);
        }
        let rows = edit.editor.rows();
        if let Some(at) = rows
            .iter()
            .position(|row| row.section() && row.path == [Step::Key(key.to_owned())])
        {
            edit.row = at;
            edit.first = at;
        }
        edit.keep_shown(rows.len());
        self.lab_edit = Some(edit);
    }

    /// Close the editor, its files written; on the builder, its editor goes back to it.
    pub(super) fn close_lab_editor(&mut self) {
        if let Some(mut edit) = self.lab_edit.take() {
            edit.editor.save();
            if edit.column == BUILD_COLUMN && self.lab_build.editor.is_none() {
                self.lab_build.editor = Some(edit.editor);
            }
        }
    }

    /// Each frame: the editors write their files a moment after the last change.
    pub(super) fn follow_lab_editor(&mut self) {
        if let Some(edit) = &mut self.lab_edit {
            edit.editor.follow(Instant::now());
        }
        if let Some(editor) = &mut self.lab_build.editor {
            editor.follow(Instant::now());
        }
    }

    /// A key while the editor is open; `None` leaves it to the tab (Space, I, M and the
    /// like still swing, light and mute).
    pub(super) fn lab_edit_key(
        &mut self,
        event: &KeyEvent,
        key: KeyCode,
        shift: bool,
        control: bool,
    ) -> Option<PanelAction> {
        let edit = self.lab_edit.as_mut()?;
        let rows = edit.editor.rows();
        let chosen = rows
            .get(edit.row.min(rows.len().saturating_sub(1)))
            .cloned();
        if let Some(typed) = &mut edit.typing {
            match key {
                KeyCode::Escape => edit.typing = None,
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let typed = edit.typing.take().unwrap_or_default();
                    if let Some(row) = &chosen {
                        commit(&mut edit.editor, row, &typed);
                    }
                }
                _ => {
                    crate::text_entry::edit(
                        typed,
                        key,
                        event.text.as_deref(),
                        shift,
                        control,
                        TYPED_MAX,
                        true,
                    );
                }
            }
            return Some(PanelAction::None);
        }
        let page = edit.column.rows as isize - 1;
        let step = if shift {
            10.0
        } else if control {
            0.1
        } else {
            1.0
        };
        match key {
            KeyCode::Escape => {
                self.close_lab_editor();
                return Some(PanelAction::None);
            }
            KeyCode::ArrowUp => edit.choose(-1, rows.len()),
            KeyCode::ArrowDown => edit.choose(1, rows.len()),
            KeyCode::PageUp => edit.choose(-page, rows.len()),
            KeyCode::PageDown => edit.choose(page, rows.len()),
            KeyCode::Home => edit.choose(-(rows.len() as isize), rows.len()),
            KeyCode::KeyA if !control => {
                if let Some(at) = Editor::adds_row(&rows) {
                    edit.row = at + 1;
                    // The heading at the top, the sections under it in view.
                    edit.first = at;
                }
            }
            KeyCode::End => edit.choose(rows.len() as isize, rows.len()),
            KeyCode::KeyZ if control && shift => edit.editor.redo(),
            KeyCode::KeyZ if control => edit.editor.undo(),
            KeyCode::KeyY if control => edit.editor.redo(),
            KeyCode::KeyS if control => edit.editor.save(),
            KeyCode::ArrowLeft | KeyCode::ArrowRight
                if shift
                    && !chosen
                        .as_ref()
                        .is_some_and(|row| matches!(row.value, Value::Number { .. })) =>
            {
                edit.editor.open_all(key == KeyCode::ArrowRight);
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                let by = if key == KeyCode::ArrowLeft {
                    -step
                } else {
                    step
                };
                match &chosen {
                    Some(row) if matches!(row.value, Value::Number { .. }) => {
                        edit.editor.nudge(row, by, true);
                    }
                    Some(row) if matches!(row.value, Value::Switch(_)) => edit.editor.flip(row),
                    // Right opens a group, Left closes it, as a tree does.
                    Some(row)
                        if is_open(row) != (key == KeyCode::ArrowRight)
                            && matches!(row.value, Value::Group { .. } | Value::Colour { .. }) =>
                    {
                        edit.editor.toggle_open(row);
                    }
                    _ => {}
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let Some(row) = &chosen {
                    edit.enter(row);
                }
            }
            KeyCode::Delete => {
                if let Some(row) = chosen.as_ref().filter(|row| row.section() || row.item()) {
                    edit.editor.remove(row);
                }
            }
            KeyCode::Insert => {
                if let Some(row) = chosen.as_ref().filter(|row| row.item()) {
                    edit.editor.duplicate(row);
                }
            }
            _ => {
                // A digit, a point or a minus on a number starts typing it.
                let typed = event.text.as_deref().unwrap_or_default();
                let numeric = typed.len() == 1
                    && typed
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == '-');
                match &chosen {
                    Some(row) if numeric && matches!(row.value, Value::Number { .. }) => {
                        edit.typing = Some(typed.to_owned());
                    }
                    _ => return None,
                }
            }
        }
        let count = edit.editor.rows().len();
        edit.keep_shown(count);
        Some(PanelAction::None)
    }

    /// A click on the editor's rows or buttons; `None` for any other token.
    pub(super) fn lab_edit_click(&mut self, token: u16) -> Option<PanelAction> {
        if token == EDIT_TOKEN {
            self.open_lab_editor();
            return Some(PanelAction::None);
        }
        let edit = self.lab_edit.as_mut()?;
        match token {
            DONE_TOKEN => {
                self.close_lab_editor();
                return Some(PanelAction::None);
            }
            UNDO_TOKEN => edit.editor.undo(),
            REDO_TOKEN => edit.editor.redo(),
            REVERT_TOKEN => edit.editor.revert(),
            _ => {
                let shown = edit.column.rows;
                let slot = token
                    .checked_sub(ROW_BASE)
                    .filter(|slot| usize::from(*slot) < shown)
                    .or_else(|| {
                        token
                            .checked_sub(VALUE_BASE)
                            .filter(|slot| usize::from(*slot) < shown)
                    })?;
                let index = edit.first + usize::from(slot);
                let rows = edit.editor.rows();
                let row = rows.get(index)?.clone();
                let again = edit.row == index;
                edit.row = index;
                edit.typing = None;
                // A click on a group, a colour, a switch or a section to add does what
                // Enter does; on a number or a path, a second click types it.
                match row.value {
                    Value::Number { .. } | Value::Text(_) if !again => {}
                    _ => edit.enter(&row),
                }
            }
        }
        let count = edit.editor.rows().len();
        edit.keep_shown(count);
        Some(PanelAction::None)
    }

    /// A pointer event over the editor: a press on a number's slider sets it and holds it
    /// until let go, a drag on a number without one moves it; the wheel scrolls the rows.
    /// `None` leaves the event to the tab.
    pub(super) fn lab_edit_pointer(&mut self, event: InputEvent) -> Option<PanelAction> {
        let scale = self.scale.max(0.01);
        let edit = self.lab_edit.as_mut()?;
        match event {
            InputEvent::PointerPress { position: at, .. } => {
                let slot = (0..edit.column.rows).find(|slot| {
                    self.ui
                        .rect_for(VALUE_BASE + *slot as u16)
                        .is_some_and(|rect| rect.contains(at))
                })?;
                let index = edit.first + slot;
                let rows = edit.editor.rows();
                let row = rows.get(index)?;
                let Value::Number { value, range, .. } = row.value else {
                    return None;
                };
                edit.row = index;
                edit.typing = None;
                edit.editor.begin();
                if range.is_some() {
                    let share = slider_share(&self.ui, slot, at.x);
                    edit.editor.set_share(row, share, false);
                }
                edit.drag = Some(Drag {
                    row: index,
                    from: at.x,
                    value,
                });
            }
            InputEvent::PointerMove(at) => {
                let drag = edit.drag?;
                let rows = edit.editor.rows();
                let row = rows.get(drag.row)?;
                match row.value {
                    Value::Number { range: Some(_), .. } => {
                        let slot = drag.row.saturating_sub(edit.first);
                        let share = slider_share(&self.ui, slot, at.x);
                        edit.editor.set_share(row, share, false);
                    }
                    Value::Number { .. } => {
                        let moved = f64::from((at.x - drag.from) / scale) * SCRUB;
                        let value = drag.value + moved * Editor::step(row);
                        edit.editor.set_number(row, value, false);
                    }
                    _ => {}
                }
            }
            InputEvent::PointerRelease { .. } | InputEvent::PointerLeave => {
                edit.drag.take()?;
            }
            InputEvent::PointerWheel {
                position: at,
                delta,
            } if delta.y != 0.0 => {
                let over = (0..edit.column.rows).any(|slot| {
                    self.ui
                        .rect_for(ROW_BASE + slot as u16)
                        .is_some_and(|rect| rect.contains(at))
                });
                if !over {
                    return None;
                }
                let count = edit.editor.rows().len();
                let last = count.saturating_sub(edit.column.rows);
                edit.first = if delta.y > 0.0 {
                    edit.first.saturating_sub(2)
                } else {
                    (edit.first + 2).min(last)
                };
            }
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// The editor's keys for the line at the bottom.
    pub(super) fn lab_edit_keys(&self, keys: &mut Vec<(&'static [&'static str], &str)>) {
        keys.push((&["Up", "Down"], "row"));
        keys.push((&["Left", "Right"], "change"));
        keys.push((&["Enter"], "open / type"));
        keys.push((&["Del"], "remove"));
        keys.push((&["A"], "add a section"));
        keys.push((&["Ctrl", "Z"], "undo"));
        keys.push((&["Esc"], "done"));
    }

    /// The editor, in its column: under the floating saber, or in the builder's groups'
    /// place.
    pub(super) fn lab_editor(&mut self, frame: &Frame) {
        let s = frame.s;
        let Some(edit) = self.lab_edit.as_mut() else {
            return;
        };
        let column = edit.column;
        let (column_x, column_width, right_x) = (column.x, column.width, column.right());
        let rows_shown = column.rows;
        let list_top = column.list_top();
        let rows = edit.editor.rows();
        edit.keep_shown(rows.len());
        let name = crate::unlockables::blade_skin(edit.editor.id())
            .map_or(edit.editor.id(), |skin| skin.name);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Editing {name}"),
            frame.rect(column_x, column.top - 14.0, column_width - 260.0, 28.0),
            21.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let (undo, redo) = edit.editor.can_undo();
        for (index, (label, enabled, token)) in [
            ("Undo", undo, UNDO_TOKEN),
            ("Redo", redo, REDO_TOKEN),
            ("Done", true, DONE_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            kit::button(
                &mut self.ui,
                frame,
                [
                    right_x - 252.0 + index as f32 * 86.0,
                    column.top - 16.0,
                    80.0,
                    32.0,
                ],
                label,
                token == DONE_TOKEN,
                enabled,
                false,
                token,
            );
        }
        let (status, colour) = match edit.editor.problem() {
            Some(why) => (
                format!("Does not read: {why}. The last good look stays."),
                color::EMBER,
            ),
            None if edit.editor.unsaved() => ("Shown; writing the files…".to_owned(), color::QUIET),
            None if edit.editor.revertable() => (
                "Saved. Revert puts it back as it was before your first change.".to_owned(),
                color::QUIET,
            ),
            None => (
                "Shown and saved in the lab's folder.".to_owned(),
                color::QUIET,
            ),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(
                column_x,
                column.status_y() - 10.0,
                column_width - 108.0,
                20.0,
            ),
            14.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        kit::button(
            &mut self.ui,
            frame,
            [right_x - 96.0, column.status_y() - 13.0, 96.0, 26.0],
            "Revert",
            false,
            edit.editor.revertable(),
            false,
            REVERT_TOKEN,
        );
        let chosen = edit.row;
        let typing = edit.typing.clone();
        let first = edit.first;
        for (slot, row) in rows
            .iter()
            .enumerate()
            .skip(first)
            .take(rows_shown)
            .map(|(index, row)| (index - first, row))
        {
            let top = list_top + slot as f32 * ROW_HEIGHT;
            let middle = top + ROW_HEIGHT * 0.5;
            let index = first + slot;
            if index == chosen {
                kit::band(
                    &mut self.ui,
                    frame,
                    [column_x - 12.0, top, column_width + 12.0, ROW_HEIGHT],
                );
            }
            self.ui.hit_region(
                ROW_BASE + slot as u16,
                frame.rect(column_x - 12.0, top, column_width + 12.0, ROW_HEIGHT),
            );
            let x = column_x + row.depth as f32 * INDENT;
            let (mark, label_colour, weight) = match row.value {
                Value::Group { open, .. } | Value::Colour { open, .. } => (
                    if open { "- " } else { "+ " },
                    color::TEXT,
                    FontWeight::Semibold,
                ),
                Value::Add => ("+ add ", color::GOLD, FontWeight::Regular),
                Value::Heading => ("", color::GOLD_BRIGHT, FontWeight::Semibold),
                _ => ("", color::MUTED, FontWeight::Regular),
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{mark}{}", row.label),
                frame.rect(
                    x,
                    middle - 11.0,
                    column_width - (x - column_x) - SLIDER_WIDTH - VALUE_WIDTH,
                    22.0,
                ),
                16.0 * s,
                label_colour,
                weight,
                TextAlign::Start,
            );
            let value_rect = [
                right_x - SLIDER_WIDTH - VALUE_WIDTH,
                top + 2.0,
                SLIDER_WIDTH + VALUE_WIDTH,
                ROW_HEIGHT - 4.0,
            ];
            if index == chosen
                && let Some(typed) = &typing
            {
                kit::field(
                    &mut self.ui,
                    frame,
                    [right_x - 200.0, top + 1.0, 200.0, ROW_HEIGHT - 2.0],
                    format_args!("{typed}"),
                    true,
                    true,
                );
                continue;
            }
            // A section's note, wider than a value: from past the longest section name.
            let note = |panel: &mut Self, row: &Row, colour: Color| {
                text(
                    &mut panel.ui,
                    TextFamily::Body,
                    format_args!("{}", section_note(row)),
                    frame.rect(column_x + 150.0, middle - 10.0, column_width - 150.0, 20.0),
                    15.0 * s,
                    colour,
                    FontWeight::Regular,
                    TextAlign::End,
                );
            };
            let value_text = |panel: &mut Self, words: &str, colour: Color| {
                text(
                    &mut panel.ui,
                    TextFamily::Body,
                    format_args!("{words}"),
                    frame.rect(
                        right_x - VALUE_WIDTH - SLIDER_WIDTH,
                        middle - 11.0,
                        SLIDER_WIDTH + VALUE_WIDTH,
                        22.0,
                    ),
                    16.0 * s,
                    colour,
                    FontWeight::Regular,
                    TextAlign::End,
                );
            };
            match &row.value {
                Value::Number {
                    value,
                    range,
                    whole,
                } => {
                    let [x, y, width, height] = value_rect;
                    self.ui
                        .hit_region(VALUE_BASE + slot as u16, frame.rect(x, y, width, height));
                    if let Some((low, high)) = range {
                        let share =
                            ((*value - f64::from(*low)) / f64::from(high - low).max(1e-9)) as f32;
                        kit::slider(
                            &mut self.ui,
                            frame,
                            right_x - VALUE_WIDTH - SLIDER_WIDTH + 8.0,
                            middle,
                            SLIDER_WIDTH - 24.0,
                            share,
                            index == chosen,
                        );
                    }
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!("{}", shown_number(*value, *whole)),
                        frame.rect(right_x - VALUE_WIDTH, middle - 11.0, VALUE_WIDTH, 22.0),
                        16.0 * s,
                        color::TEXT,
                        FontWeight::Regular,
                        TextAlign::End,
                    );
                }
                Value::Switch(on) => value_text(self, if *on { "On" } else { "Off" }, color::TEXT),
                Value::Text(words) => value_text(self, &shorten(words, 26), color::TEXT),
                // A section says what it draws, and its file when it is an additions
                // file's.
                Value::Group { .. } if row.section() => note(self, row, color::QUIET),
                Value::Group { count, .. } => value_text(self, &format!("{count}"), color::QUIET),
                Value::Colour { rgb, .. } => {
                    // The channels left of the swatch.
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!("{:.2} {:.2} {:.2}", rgb[0], rgb[1], rgb[2]),
                        frame.rect(
                            right_x - VALUE_WIDTH - SLIDER_WIDTH,
                            middle - 11.0,
                            SLIDER_WIDTH + VALUE_WIDTH - 38.0,
                            22.0,
                        ),
                        16.0 * s,
                        color::MUTED,
                        FontWeight::Regular,
                        TextAlign::End,
                    );
                    let peak = rgb.iter().copied().fold(1.0_f32, f32::max);
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: frame.rect(right_x - 28.0, middle - 9.0, 28.0, 18.0),
                        radius: 4.0 * s,
                        color: Color::new(rgb[0] / peak, rgb[1] / peak, rgb[2] / peak, 1.0),
                    });
                }
                Value::Add => note(self, row, color::MUTED),
                Value::Heading => {}
            }
        }
        if rows.len() > rows_shown {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}-{} of {}",
                    first + 1,
                    (first + rows_shown).min(rows.len()),
                    rows.len()
                ),
                frame.rect(
                    column_x,
                    list_top + rows_shown as f32 * ROW_HEIGHT + 2.0,
                    column_width,
                    18.0,
                ),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
    }

    /// Open the editor on skin `id` of the lab's folder, its sections open, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn lab_edit_for_shot(&mut self, id: &'static str) {
        if let Ok(mut editor) = Editor::open(id) {
            editor.open_all(true);
            self.lab_edit = Some(LabEdit {
                editor,
                column: STAGE_COLUMN,
                row: 1,
                first: 0,
                typing: None,
                drag: None,
            });
        }
    }
}

impl Panel {
    /// Set the number at `path` of the edited skin's file, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_edit_set_for_shot(&mut self, path: &str, value: f64) {
        if let Some(edit) = &mut self.lab_edit {
            edit.editor.set_for_shot(path, value);
        }
    }
}

impl Panel {
    /// Close the edited skin's sections and show the ones to add, adding section `add`
    /// when given, for a world shot.
    #[cfg(test)]
    pub(crate) fn lab_edit_adds_for_shot(&mut self, add: Option<&str>) {
        let Some(edit) = &mut self.lab_edit else {
            return;
        };
        edit.editor.open_all(false);
        if let Some(add) = add {
            let rows = edit.editor.rows();
            if let Some(row) = rows
                .iter()
                .find(|row| row.label == add && row.value == Value::Add)
            {
                edit.editor.add(row);
            }
        }
        let rows = edit.editor.rows();
        if let Some(at) = Editor::adds_row(&rows) {
            edit.row = at + 1;
            edit.first = at.saturating_sub(4);
        }
    }
}

impl LabEdit {
    /// Choose the row `by` rows on, within the `count` there are.
    fn choose(&mut self, by: isize, count: usize) {
        let last = count.saturating_sub(1) as isize;
        self.row = (self.row as isize + by).clamp(0, last.max(0)) as usize;
    }

    /// Keep the chosen row among the `count` there are, and shown.
    fn keep_shown(&mut self, count: usize) {
        let shown = self.column.rows;
        self.row = self.row.min(count.saturating_sub(1));
        if self.row < self.first {
            self.first = self.row;
        } else if self.row >= self.first + shown {
            self.first = self.row + 1 - shown;
        }
        self.first = self.first.min(count.saturating_sub(shown));
    }

    /// What Enter does on `row`: open or close a group or a colour, flip a switch, add a
    /// section, or type a number or a path.
    fn enter(&mut self, row: &Row) {
        match &row.value {
            Value::Group { .. } | Value::Colour { .. } => self.editor.toggle_open(row),
            Value::Switch(_) => self.editor.flip(row),
            Value::Add => self.editor.add(row),
            Value::Heading => {}
            Value::Number { value, whole, .. } => {
                let typed = shown_number(*value, *whole);
                crate::text_entry::select_all(&typed);
                self.typing = Some(typed);
            }
            Value::Text(words) => {
                crate::text_entry::select_all(words);
                self.typing = Some(words.clone());
            }
        }
    }
}

/// What section `row` draws and, for an additions file's, that file.
fn section_note(row: &Row) -> String {
    let about = crate::blade_editor::about(&row.label).unwrap_or_default();
    match (about, row.file) {
        (about, 0) => about.to_owned(),
        ("", file) => crate::blade_editor::FILES[file].to_owned(),
        (about, file) => format!("{about} · {}", crate::blade_editor::FILES[file]),
    }
}

/// Whether a group's or a colour's rows show.
fn is_open(row: &Row) -> bool {
    matches!(
        row.value,
        Value::Group { open: true, .. } | Value::Colour { open: true, .. }
    )
}

/// Put `typed` in `row`: a number when it reads as one, a path as written.
fn commit(editor: &mut Editor, row: &Row, typed: &str) {
    match row.value {
        Value::Number { .. } => {
            if let Ok(value) = typed.trim().parse::<f64>() {
                editor.set_number(row, value, true);
            }
        }
        Value::Text(_) => editor.set_text(row, typed.trim()),
        _ => {}
    }
}

/// Where window x `x` is along the slider of the row shown in `slot`, 0 to 1.
fn slider_share(ui: &crate::menu_widgets::MenuCanvas, slot: usize, x: f32) -> f64 {
    let Some(rect) = ui.rect_for(VALUE_BASE + slot as u16) else {
        return 0.0;
    };
    // The track lies over the value region's first part, the number after it.
    let scale = rect.width / (SLIDER_WIDTH + VALUE_WIDTH);
    let start = rect.x + 8.0 * scale;
    let width = (SLIDER_WIDTH - 24.0) * scale;
    f64::from(((x - start) / width.max(1.0)).clamp(0.0, 1.0))
}

/// A number as the row shows it: whole, or with up to four decimals.
fn shown_number(value: f64, whole: bool) -> String {
    if whole {
        return format!("{}", value.round() as i64);
    }
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0');
    match text.strip_suffix('.') {
        Some(whole) => format!("{whole}.0"),
        None => text.to_owned(),
    }
}

/// `words` cut to `most` characters, an ellipsis ending what was cut.
fn shorten(words: &str, most: usize) -> String {
    if words.chars().count() <= most {
        return words.to_owned();
    }
    let kept: String = words.chars().take(most - 1).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_show_short() {
        assert_eq!(shown_number(0.5, false), "0.5");
        assert_eq!(shown_number(2.0, false), "2.0");
        assert_eq!(shown_number(0.123456, false), "0.1235");
        assert_eq!(shown_number(3.0, true), "3");
        assert_eq!(
            shorten("sound/weapons/saber/saberhup1.mp3", 12),
            "sound/weapo…"
        );
    }
}
