//! Shared bounded numeric drafts for settings, saber colours and Camera control.

use super::MenuCanvas;
use crate::console::line_edit::{LineEdit, Motion};
use crate::menu::sjk::{color, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};
use winit::keyboard::KeyCode;

/// Separate targets keep clicks on a value from moving its slider. Row `row`'s
/// value answers to `VALUE_BASE + row`, clear of every other token of the
/// screens that use it: Settings' search lists all its rows (over 300) at once.
pub(crate) const VALUE_BASE: u16 = 20_000;
/// Most rows a screen gives typed values to.
pub(crate) const VALUE_ROWS: u16 = 1_000;

pub(crate) fn value_row(token: u16) -> Option<usize> {
    (VALUE_BASE..VALUE_BASE + VALUE_ROWS)
        .contains(&token)
        .then(|| usize::from(token - VALUE_BASE))
}

pub(crate) enum EditResult {
    Pending,
    Cancel,
    Commit(f64),
}

/// Longest draft SJK accepts, in characters.
const MAX_CHARS: usize = 10;
/// Most digits a draft takes after its decimal point.
const MAX_DECIMALS: usize = 3;

/// The SJK field's type size, the size of the number it replaces.
const SJK_TEXT: f32 = 21.0;
/// Room for a digit in the SJK field, a little over the widest at [`SJK_TEXT`].
const SJK_DIGIT: f32 = 10.0;
/// Room for a decimal point, a minus sign and the caret.
const SJK_POINT: f32 = 5.0;
const SJK_MINUS: f32 = 7.0;
const SJK_CARET: f32 = 4.0;
/// The SJK field's padding left of the text, and how far it passes the number's right edge.
const SJK_PAD_LEFT: f32 = 10.0;
const SJK_PAD_RIGHT: f32 = 8.0;

/// Owns the target row so pointer hover cannot redirect an unfinished edit.
pub(crate) struct NumericEdit {
    pub(crate) row: usize,
    text: String,
    caret: LineEdit,
    min: f64,
    max: f64,
    integer: bool,
    invalid: bool,
}

impl NumericEdit {
    pub(crate) fn new(row: usize, text: String, min: f64, max: f64, integer: bool) -> Self {
        let mut caret = LineEdit::default();
        caret.select_all(&text);
        Self {
            row,
            text,
            caret,
            min,
            max,
            integer,
            invalid: false,
        }
    }

    /// SJK: entry opened by typing on a selected slider, starting with `typed`
    /// instead of the current value. `None` when `typed` cannot start a number
    /// for this slider.
    pub(crate) fn typed(
        row: usize,
        typed: &str,
        min: f64,
        max: f64,
        integer: bool,
    ) -> Option<Self> {
        let mut edit = Self::new(row, String::new(), min, max, integer);
        edit.type_text(typed).then_some(edit)
    }

    /// The draft's value when it is valid, as Enter would apply it.
    pub(crate) fn committed(&self) -> Option<f64> {
        self.value()
    }

    /// SJK's stricter filter: digits, one decimal point (a comma counts as one),
    /// a minus sign only first and only when the slider goes below zero, at
    /// most [`MAX_CHARS`] characters, no more whole digits than the slider's
    /// limit has and at most [`MAX_DECIMALS`] decimals. True when anything was typed.
    fn type_text(&mut self, typed: &str) -> bool {
        let mut accepted = false;
        for character in typed.chars() {
            let character = if character == ',' { '.' } else { character };
            let (rest, at) = match self.caret.selection(&self.text) {
                Some(range) => {
                    let mut rest = self.text.clone();
                    rest.replace_range(range.clone(), "");
                    (rest, range.start)
                }
                None => (self.text.clone(), self.caret.cursor(&self.text)),
            };
            let fits = rest.chars().count() < MAX_CHARS && !(at == 0 && rest.starts_with('-'));
            let allowed = match character {
                '0'..='9' => true,
                '.' => !rest.contains('.'),
                '-' => self.min < 0.0 && at == 0,
                _ => false,
            };
            let mut candidate = rest;
            candidate.insert(at, character);
            if fits && allowed && self.within_range(&candidate) {
                let mut buffer = [0; 4];
                self.caret.insert(
                    &mut self.text,
                    character.encode_utf8(&mut buffer),
                    MAX_CHARS,
                );
                accepted = true;
            }
        }
        accepted
    }

    /// Whether `draft` has no more whole digits than the slider's largest
    /// value and no more than [`MAX_DECIMALS`] decimals.
    fn within_range(&self, draft: &str) -> bool {
        let limit = self.min.abs().max(self.max.abs()).trunc();
        let whole_digits = format!("{limit:.0}").len();
        let unsigned = draft.strip_prefix('-').unwrap_or(draft);
        let (whole, decimals) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        whole.len() <= whole_digits && decimals.len() <= MAX_DECIMALS
    }

    /// Enter validates without step quantization; Escape discards the draft.
    /// A held key repeats editing keys but never types characters.
    pub(crate) fn key(&mut self, key: KeyCode, text: Option<&str>, repeat: bool) -> EditResult {
        match key {
            KeyCode::Escape => return EditResult::Cancel,
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let Some(value) = self.value() {
                    return EditResult::Commit(value);
                }
                self.invalid = true;
                return EditResult::Pending;
            }
            KeyCode::ArrowLeft => self.caret.motion(&self.text, Motion::Left, false),
            KeyCode::ArrowRight => self.caret.motion(&self.text, Motion::Right, false),
            KeyCode::Home => self.caret.motion(&self.text, Motion::Home, false),
            KeyCode::End => self.caret.motion(&self.text, Motion::End, false),
            KeyCode::Backspace => self.caret.delete(&mut self.text, Motion::Left),
            KeyCode::Delete => self.caret.delete(&mut self.text, Motion::Right),
            _ => {
                if let Some(text) = text {
                    if text == "\u{1}" {
                        self.caret.select_all(&self.text);
                    } else if !repeat {
                        self.type_text(text);
                    }
                }
            }
        }
        self.invalid = false;
        EditResult::Pending
    }

    /// The typed number clamped to the slider; integer sliders round a typed
    /// fraction (142.6 is 143) rather than refusing it.
    fn value(&self) -> Option<f64> {
        let value = self.text.trim().parse::<f64>().ok()?;
        let value = if self.integer { value.round() } else { value };
        value.is_finite().then(|| value.clamp(self.min, self.max))
    }

    /// Where the SJK field stands for a value drawn at `number`: its right
    /// edge stays put and it widens leftwards, over the slider, as the draft
    /// grows, so the digits keep their size.
    pub(crate) fn sjk_field(&self, number: Rect, s: f32) -> Rect {
        let digits = SJK_CARET
            + self
                .text
                .chars()
                .map(|character| match character {
                    '.' => SJK_POINT,
                    '-' => SJK_MINUS,
                    _ => SJK_DIGIT,
                })
                .sum::<f32>();
        let width = ((digits + SJK_PAD_LEFT + SJK_PAD_RIGHT) * s).max(number.height * 1.5);
        Rect::new(
            number.right() + SJK_PAD_RIGHT * s - width,
            number.y,
            width,
            number.height,
        )
    }

    /// The draft in SJK UI: a dark pill outlined gold (ember while invalid),
    /// replacing the value drawn at `number`.
    pub(crate) fn draw_sjk(&self, ui: &mut MenuCanvas, number: Rect, s: f32) {
        let field = self.sjk_field(number, s);
        let radius = field.height * 0.5;
        let edge = if self.invalid {
            color::EMBER
        } else {
            color::GOLD
        };
        let selected = self.caret.selection(&self.text).is_some();
        let list = ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect: field,
            radius,
            color: color::alpha(color::SPACE, 0.96),
        });
        if selected {
            let inset = 4.0 * s;
            let band = Rect::new(
                field.x + inset,
                field.y + inset,
                field.width - inset * 2.0,
                field.height - inset * 2.0,
            );
            let _ = list.push(DrawCommand::RoundedRect {
                rect: band,
                radius: band.height * 0.5,
                color: color::alpha(color::HOLO, 0.32),
            });
        }
        let _ = list.push(DrawCommand::Border {
            rect: field,
            radius,
            width: 1.5 * s,
            color: edge,
        });
        let cursor = self.caret.cursor(&self.text);
        let marker = if selected { "" } else { "|" };
        let left = field.x + SJK_PAD_LEFT * s;
        text(
            ui,
            TextFamily::Display,
            format_args!("{}{marker}{}", &self.text[..cursor], &self.text[cursor..]),
            Rect::new(left, number.y, number.right() - left, number.height),
            SJK_TEXT * s,
            if self.invalid {
                color::EMBER
            } else {
                color::TEXT
            },
            FontWeight::Regular,
            TextAlign::End,
        );
        ui.set_family(TextFamily::Body);
    }

    /// Compact inline editing, using retained text storage instead of frame allocations.
    pub(crate) fn draw(&self, ui: &mut MenuCanvas, rect: Rect, scale: f32) {
        let color = if self.invalid {
            Color::new(1.0, 0.3, 0.25, 1.0)
        } else {
            ui.theme().accent
        };
        if self.caret.selection(&self.text).is_some() {
            ui.accent_bar(rect, Color::new(color.r, color.g, color.b, 0.2));
        }
        let cursor = self.caret.cursor(&self.text);
        let marker = if self.caret.selection(&self.text).is_some() {
            ""
        } else {
            "|"
        };
        let size = (15.0 * scale).min(rect.width / (self.text.len() + 1) as f32);
        ui.text_fmt_aligned(
            format_args!("{}{marker}{}", &self.text[..cursor], &self.text[cursor..]),
            rect,
            size,
            color,
            FontWeight::Semibold,
            0.0,
            TextAlign::End,
        );
        ui.edit_underline(rect, color, scale);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(edit: &mut NumericEdit) -> Option<f64> {
        match edit.key(KeyCode::Enter, None, false) {
            EditResult::Commit(value) => Some(value),
            _ => None,
        }
    }

    #[test]
    fn typing_opens_a_draft_with_that_character() {
        let mut edit = NumericEdit::typed(0, "1", 0.0, 2000.0, true).unwrap();
        edit.key(KeyCode::Digit4, Some("4"), false);
        edit.key(KeyCode::Digit2, Some("2"), false);
        assert_eq!(commit(&mut edit), Some(142.0));
        assert!(NumericEdit::typed(0, "x", 0.0, 2000.0, true).is_none());
        assert!(NumericEdit::typed(0, "-", 0.0, 2000.0, true).is_none());
        assert!(NumericEdit::typed(0, "-", -1.0, 2000.0, true).is_some());
    }

    #[test]
    fn integer_sliders_round_a_typed_fraction() {
        let mut edit = NumericEdit::typed(0, "142.6", 0.0, 2000.0, true).unwrap();
        assert_eq!(commit(&mut edit), Some(143.0));
    }

    #[test]
    fn auto_rows_take_minus_one() {
        let mut edit = NumericEdit::new(0, "60".to_owned(), -1.0, 2000.0, true);
        edit.key(KeyCode::Minus, Some("-"), false);
        edit.key(KeyCode::Digit1, Some("1"), false);
        assert_eq!(commit(&mut edit), Some(-1.0));
    }

    #[test]
    fn the_filter_takes_one_point_one_leading_minus_and_ten_characters() {
        let mut edit = NumericEdit::typed(0, "1", -10.0, 1e12, false).unwrap();
        edit.key(KeyCode::Period, Some("."), false);
        edit.key(KeyCode::Comma, Some(","), false);
        edit.key(KeyCode::Minus, Some("-"), false);
        edit.key(KeyCode::KeyE, Some("e"), false);
        assert_eq!(edit.text, "1.");
        let mut long = NumericEdit::typed(0, "1", -10.0, 1e12, false).unwrap();
        for _ in 0..20 {
            long.key(KeyCode::Digit5, Some("5"), false);
        }
        assert_eq!(long.text.len(), MAX_CHARS);
        let mut comma = NumericEdit::typed(0, "0", 0.0, 1.0, false).unwrap();
        comma.key(KeyCode::Comma, Some(","), false);
        comma.key(KeyCode::Digit5, Some("5"), false);
        assert_eq!(commit(&mut comma), Some(0.5));
    }

    #[test]
    fn the_sjk_field_widens_left_and_fits_a_full_draft() {
        let number = Rect::new(500.0, 100.0, 52.0, 30.0);
        let short = NumericEdit::typed(0, "1", 0.0, 1e12, false).unwrap();
        let long = NumericEdit::typed(0, "1234567890", 0.0, 1e12, false).unwrap();
        let (narrow, wide) = (short.sjk_field(number, 1.0), long.sjk_field(number, 1.0));
        assert_eq!(narrow.right(), wide.right());
        assert!(wide.x < narrow.x);
        assert!(wide.width >= MAX_CHARS as f32 * SJK_DIGIT + SJK_CARET);
        assert!(narrow.width <= number.height * 1.5);
    }

    #[test]
    fn the_sjk_field_has_room_for_every_character_it_takes() {
        use crate::text::{DISPLAY, TextFace, load_family, visible_text_width_style};
        let loaded = load_family(&DISPLAY, 1.0, None).expect("a bundled family");
        let font = &loaded.font;
        let scale = SJK_TEXT / font.height;
        let width = |text| visible_text_width_style(font, text, scale, TextFace::Regular, 0.0);
        for digit in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            assert!(width(digit) <= SJK_DIGIT, "{digit}: {}", width(digit));
        }
        assert!(width(".") <= SJK_POINT, "{}", width("."));
        assert!(width("-") <= SJK_MINUS, "{}", width("-"));
        assert!(width("|") <= SJK_CARET, "{}", width("|"));
    }

    #[test]
    fn the_slider_range_limits_the_digits() {
        let typed = |max: f64, text: &str| {
            let mut edit = NumericEdit::new(0, String::new(), 0.0, max, false);
            edit.type_text(text);
            edit.text
        };
        assert_eq!(typed(130.0, "12345"), "123");
        assert_eq!(typed(2000.0, "12345"), "1234");
        assert_eq!(typed(1.0, "0.12345"), "0.123");
        assert_eq!(typed(1.0, "25"), "2");
    }

    #[test]
    fn held_keys_do_not_type() {
        let mut edit = NumericEdit::typed(0, "1", 0.0, 2000.0, true).unwrap();
        edit.key(KeyCode::Digit1, Some("1"), true);
        assert_eq!(edit.text, "1");
    }

    #[test]
    fn committed_reports_only_valid_drafts() {
        let edit = NumericEdit::typed(0, "7", 0.0, 2000.0, true).unwrap();
        assert_eq!(edit.committed(), Some(7.0));
        let point = NumericEdit::typed(0, ".", 0.0, 2000.0, false).unwrap();
        assert_eq!(point.committed(), None);
    }
}
