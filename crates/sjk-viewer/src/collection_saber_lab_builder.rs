//! The Saber Lab tab's builder (`docs/client.md`, "Saber Lab"), the tab's first view: the
//! library of every effect a blade skin can draw on the left, by kind
//! ([`crate::blade_builder::EFFECTS`]), the blade's groups in the middle (its own make,
//! the effects always on, and the parts its wearer may switch off) and the model on the
//! right holding it. An effect is dragged from the library onto the blade, between its
//! groups, or back to the library to take it out; a click on one opens its fields
//! ([`super::saber_lab_editor`]) where the groups were, and a rest of the pointer on one
//! says what it draws. Each change goes to the lab folder's files through the editor
//! ([`crate::blade_editor::Editor`]); one that would not read is refused, saying why.

use super::view::{LEFT_X, RIGHT_X, left_lines};
use super::*;
use crate::blade_builder::{self as builder, Kind, Place, Shape};
use crate::blade_editor::Editor;
use crate::menu::sjk::hover::{self, Card, Line};
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};
use std::time::{Duration, Instant};

/// The builder's columns (frame pixels): the library and the groups under the lead line,
/// the model's room right of them.
const TOP: f32 = 300.0;
const BOTTOM: f32 = 988.0;
const LIBRARY_X: f32 = LEFT_X;
const LIBRARY_WIDTH: f32 = 456.0;
pub(super) const GROUPS_X: f32 = 588.0;
pub(super) const GROUPS_WIDTH: f32 = 560.0;
pub(super) const GROUPS_TOP: f32 = TOP;
/// Where a drag turns the camera round the model and the wheel brings it nearer.
const STAGE_AREA: [f32; 4] = [1_190.0, 340.0, RIGHT_X - 1_190.0, 600.0];
/// The stage's buttons' row, and the view's switch over it.
const STAGE_BUTTONS_Y: f32 = TOP + 10.0;
pub(super) const VIEW_SWITCH_RECT: [f32; 4] = [RIGHT_X - 200.0, 228.0, 200.0, 40.0];
/// The library's tabs and cards (two to a line).
const TABS_Y: f32 = TOP + 46.0;
const CARDS_Y: f32 = TOP + 80.0;
const CARD_WIDTH: f32 = (LIBRARY_WIDTH - 12.0) / 2.0;
const CARD_HEIGHT: f32 = 58.0;
const CARD_STEP: f32 = CARD_HEIGHT + 10.0;
/// An effect's chip on the blade: its height, the gap between two and its words' size.
const CHIP_HEIGHT: f32 = 32.0;
const CHIP_GAP: f32 = 8.0;
const CHIP_SIZE: f32 = 17.0;
/// A box of the groups: its padding, its title's height, and the gap to the next.
const BOX_PAD: f32 = 12.0;
const BOX_TITLE: f32 = 30.0;
const BOX_GAP: f32 = 12.0;
/// How far (frame pixels) a press moves before it is a drag.
const DRAG_SLOP: f32 = 6.0;
/// How long the pointer rests before an effect's card shows; how long a refusal shows.
const REST: Duration = Duration::from_millis(350);
const NOTE_FOR: Duration = Duration::from_secs(6);
/// The longest group name typed.
const NAME_MAX: usize = 24;
/// The builder's buttons: the view's switch, undo, redo, the copy into the lab, the
/// blade's arrows and the way back to its original.
pub(super) const VIEW_SWITCH: u16 = 3_040;
const UNDO_TOKEN: u16 = 3_041;
const REDO_TOKEN: u16 = 3_042;
const COPY_TOKEN: u16 = 3_043;
const PREVIOUS_TOKEN: u16 = 3_044;
const NEXT_TOKEN: u16 = 3_045;
const REVERT_TOKEN: u16 = 3_046;

/// A group's box on a line of them: its group (none for New group), its chips' keys and
/// rectangles, and the bottom of its last chip.
type LaidBox = (Option<usize>, Vec<&'static str>, Vec<[f32; 4]>, f32);

/// A thing of the builder's that answers the pointer, laid out each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Spot {
    /// A library tab (its kind's place in [`Kind::ALL`]).
    Tab(usize),
    /// An effect's card in the library.
    Card(&'static str),
    /// An effect's chip on the blade.
    Chip(&'static str),
    /// A group's name, its switch and its way out.
    Name(usize),
    Switch(usize),
    Ungroup(usize),
    /// Where an effect may be dropped.
    Zone(Place),
}

/// What the builder holds from frame to frame.
#[derive(Default)]
pub(super) struct LabBuild {
    /// The editor on the blade built, while it is one of the lab folder's (the fields'
    /// rows hold it while they are open).
    pub(super) editor: Option<Editor>,
    /// A blade the editor could not open, and why: not tried again while it is shown.
    failed: Option<(&'static str, String)>,
    /// The library's tab, a place in [`Kind::ALL`].
    kind: usize,
    /// What answers the pointer, laid out this frame (frame pixels), the topmost last;
    /// the frame they were laid out in.
    spots: Vec<(Spot, [f32; 4])>,
    frame: Option<Frame>,
    /// The pointer (frame pixels), what a press is on and where it was pressed.
    pointer: Option<[f32; 2]>,
    press: Option<(Spot, [f32; 2])>,
    /// The effect being dragged.
    drag: Option<&'static str>,
    /// What the pointer rests on, since when.
    rest: Option<(Spot, Instant)>,
    /// The group being named, and the name typed.
    naming: Option<(usize, String)>,
    /// Why the last change was refused, and when.
    note: Option<(String, Instant)>,
    /// The blade's effects as last laid out, and each group's switch.
    shape: Shape,
    on: Vec<bool>,
}

/// Why the builder cannot build the blade shown.
enum Unbuilt {
    /// The stock blade: no file.
    Stock,
    /// A signature's or a hand-ported design's: its own shader draws most of it.
    Locked,
    /// A skin of the packs, not of the lab's folder yet.
    Elsewhere,
    /// Its files would not open.
    Failed(String),
}

impl LabBuild {
    /// Save and let go of the editor.
    pub(super) fn close(&mut self) {
        if let Some(mut editor) = self.editor.take() {
            editor.save();
        }
        self.drag = None;
        self.press = None;
        self.naming = None;
    }

    /// Note a refusal, to show a while.
    fn refuse(&mut self, why: impl Into<String>) {
        self.note = Some((why.into(), Instant::now()));
    }

    /// The spot under frame point `at`, the topmost first; zones only when `zones`.
    fn spot_at(&self, at: [f32; 2], zones: bool) -> Option<Spot> {
        self.spots
            .iter()
            .rev()
            .filter(|(spot, _)| zones || !matches!(spot, Spot::Zone(_)))
            .find(|(_, [x, y, width, height])| {
                at[0] >= *x && at[0] < x + width && at[1] >= *y && at[1] < y + height
            })
            .map(|(spot, _)| *spot)
    }

    /// The zone under frame point `at`.
    fn zone_at(&self, at: [f32; 2]) -> Option<Place> {
        self.spots
            .iter()
            .rev()
            .filter_map(|(spot, rect)| match spot {
                Spot::Zone(place) => Some((*place, rect)),
                _ => None,
            })
            .find(|(_, [x, y, width, height])| {
                at[0] >= *x && at[0] < x + width && at[1] >= *y && at[1] < y + height
            })
            .map(|(place, _)| place)
    }

    /// Where `spot` was laid out this frame.
    fn rect_of(&self, spot: Spot) -> Option<[f32; 4]> {
        self.spots
            .iter()
            .find(|(laid, _)| *laid == spot)
            .map(|(_, rect)| *rect)
    }
}

/// Whether skin `id` is drawn by a shader of its own, which the builder cannot rebuild.
fn locked(id: &str) -> bool {
    crate::blade_signatures::of(id).is_some() || crate::blade_lab_designs::of(id).is_some()
}

/// An effect's name, or its key when the builder does not know it.
fn name_of(key: &str) -> &str {
    builder::effect_of(key).map_or(key, |effect| effect.name)
}

/// The chips of `effects` in their order, an orbit after the glint or holes it goes with.
fn chips(effects: &[String], orbit: bool) -> Vec<&'static str> {
    let mut chips = Vec::new();
    for key in effects {
        let Some(effect) = builder::effect_of(key) else {
            continue;
        };
        chips.push(effect.key);
        if orbit && matches!(effect.key, "star" | "holes") {
            chips.push("orbit");
        }
    }
    chips
}

/// A chip's width for `name`.
fn chip_width(name: &str) -> f32 {
    crate::text::display_width(name, CHIP_SIZE) * crate::text::style::current().scale.max(0.5)
        + 26.0
}

/// Lay chips for `keys` in rows `width` wide from (`x`, `y`): each chip's rectangle, and
/// the bottom of the last row (`y` when there are none).
fn flow(keys: &[&'static str], x: f32, y: f32, width: f32) -> (Vec<[f32; 4]>, f32) {
    let mut rects = Vec::with_capacity(keys.len());
    let (mut at, mut row) = (x, y);
    for key in keys {
        let chip = chip_width(name_of(key)).min(width);
        if at > x && at + chip > x + width {
            at = x;
            row += CHIP_HEIGHT + CHIP_GAP;
        }
        rects.push([at, row, chip, CHIP_HEIGHT]);
        at += chip + CHIP_GAP;
    }
    let bottom = if keys.is_empty() {
        y
    } else {
        row + CHIP_HEIGHT
    };
    (rects, bottom)
}

fn push(canvas: &mut MenuCanvas, command: DrawCommand) {
    let _ = canvas.draw_list_mut().push(command);
}

/// A dark glass panel behind a column, over `rect` (frame pixels).
fn glass(canvas: &mut MenuCanvas, frame: &Frame, [x, y, width, height]: [f32; 4]) {
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 16.0 * frame.s,
            color: color::alpha(color::SPACE, 0.72),
        },
    );
}

/// A box of the groups over `rect`: filled `fill`, edged `edge` (dashed boxes are edged
/// only).
fn boxed(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    [x, y, width, height]: [f32; 4],
    fill: Color,
    edge: Color,
) {
    push(
        canvas,
        DrawCommand::RoundedRect {
            rect: frame.rect(x, y, width, height),
            radius: 12.0 * frame.s,
            color: fill,
        },
    );
    push(
        canvas,
        DrawCommand::Border {
            rect: frame.rect(x, y, width, height),
            radius: 12.0 * frame.s,
            width: 1.5 * frame.s,
            color: edge,
        },
    );
}

/// How a chip shows.
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    /// Always on, or a library card's.
    Plain,
    /// In a group.
    Gold,
    /// The blade's own make.
    Core,
    /// Lifted by the pointer.
    Lifted,
}

/// An effect's chip over `rect`, named `name`, ringed while `hovered`.
fn chip(
    canvas: &mut MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    name: &str,
    tone: Tone,
    hovered: bool,
) {
    let [x, y, width, height] = rect;
    let (fill, edge, words) = match tone {
        Tone::Plain => (
            Color::new(0.05, 0.08, 0.15, 0.95),
            color::alpha(color::HOLO, 0.32),
            color::TEXT,
        ),
        Tone::Gold => (
            Color::new(0.09, 0.08, 0.05, 0.95),
            color::alpha(color::GOLD, 0.5),
            color::TEXT,
        ),
        Tone::Core => (
            Color::new(0.05, 0.07, 0.12, 0.9),
            color::alpha(color::HOLO, 0.2),
            color::MUTED,
        ),
        Tone::Lifted => (
            Color::new(0.14, 0.11, 0.05, 0.98),
            color::GOLD_BRIGHT,
            color::GOLD_BRIGHT,
        ),
    };
    let edge = if hovered {
        color::alpha(color::GOLD_BRIGHT, 0.9)
    } else {
        edge
    };
    boxed(canvas, frame, rect, fill, edge);
    text(
        canvas,
        TextFamily::Display,
        format_args!("{name}"),
        frame.rect(x, y, width, height),
        CHIP_SIZE * frame.s,
        words,
        FontWeight::Semibold,
        TextAlign::Center,
    );
}

impl Panel {
    /// Whether the Saber Lab tab shows its builder.
    pub(super) fn lab_building(&self) -> bool {
        self.tab == Tab::SaberLab && !self.lab_stage_view && self.backstage != Backstage::None
    }

    /// Show the builder or the stage's choices, the editor of the other let go.
    pub(super) fn switch_lab_view(&mut self) {
        self.close_lab_editor();
        self.lab_build.close();
        self.lab_stage_view = !self.lab_stage_view;
    }

    /// Keep the editor on the blade shown, opening it as one of the lab folder's comes;
    /// or why the builder cannot build it.
    fn follow_build_blade(&mut self) -> Result<(), Unbuilt> {
        let Some(id) = self.lab_edit_target() else {
            self.lab_build.close();
            return Err(Unbuilt::Stock);
        };
        if locked(id) {
            self.lab_build.close();
            return Err(Unbuilt::Locked);
        }
        if !crate::blade_editor::in_lab(id) {
            self.lab_build.close();
            return Err(Unbuilt::Elsewhere);
        }
        // The fields' rows hold the editor while they are open.
        if self.lab_edit.is_some() {
            return Ok(());
        }
        let build = &mut self.lab_build;
        if build
            .editor
            .as_ref()
            .is_some_and(|editor| editor.id() == id)
        {
            return Ok(());
        }
        if let Some((failed, why)) = &build.failed
            && *failed == id
        {
            return Err(Unbuilt::Failed(why.clone()));
        }
        build.close();
        match Editor::open(id) {
            Ok(editor) => {
                build.editor = Some(editor);
                build.failed = None;
                Ok(())
            }
            Err(why) => {
                build.failed = Some((id, why.clone()));
                Err(Unbuilt::Failed(why))
            }
        }
    }

    /// The builder, under the tab's lead line.
    pub(super) fn lab_builder(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let now = Instant::now();
        self.lab_build.spots.clear();
        self.lab_build.frame = Some(*frame);
        if self
            .lab_build
            .note
            .as_ref()
            .is_some_and(|(_, at)| now.saturating_duration_since(*at) > NOTE_FOR)
        {
            self.lab_build.note = None;
        }
        let built = self.follow_build_blade();
        let skin = self.lab_edit_target();
        // The fields' rows hold the editor while they are open.
        let editor = self
            .lab_build
            .editor
            .as_ref()
            .or_else(|| self.lab_edit.as_ref().map(|edit| edit.editor()));
        let (present, shape) = match (&built, editor) {
            (Ok(()), Some(editor)) => {
                let present = editor.present();
                let shape = Shape::of(&present, &editor.options());
                (present, shape)
            }
            _ => (Vec::new(), Shape::default()),
        };
        self.lab_build.on = shape
            .groups
            .iter()
            .map(|group| {
                skin.is_some_and(|skin| {
                    crate::saber_skin_options::is_on(inputs.options, skin, &group.id)
                })
            })
            .collect();
        self.lab_build.shape = shape.clone();
        self.build_library(frame, &present, &shape, built.is_ok());
        if self.lab_edit.is_some() {
            glass(
                &mut self.ui,
                frame,
                [
                    GROUPS_X - 18.0,
                    TOP - 10.0,
                    GROUPS_WIDTH + 36.0,
                    BOTTOM - TOP + 10.0,
                ],
            );
            self.lab_editor(frame);
        } else {
            self.build_groups(frame, &shape, &present, built, skin);
        }
        self.build_stage(frame);
        // What the pointer rests on, for its card.
        let resting = self
            .lab_build
            .pointer
            .and_then(|at| self.lab_build.spot_at(at, false))
            .filter(|spot| matches!(spot, Spot::Card(_) | Spot::Chip(_)));
        let build = &mut self.lab_build;
        if build.rest.map(|(spot, _)| spot) != resting {
            build.rest = resting.map(|spot| (spot, now));
        }
    }

    /// The library: a tab to each kind of effect, the effects of the one chosen as cards,
    /// those on the blade saying where they are.
    fn build_library(&mut self, frame: &Frame, present: &[String], shape: &Shape, ready: bool) {
        let s = frame.s;
        glass(
            &mut self.ui,
            frame,
            [
                LIBRARY_X - 18.0,
                TOP - 10.0,
                LIBRARY_WIDTH + 36.0,
                BOTTOM - TOP + 10.0,
            ],
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("EFFECTS"),
            frame.rect(LIBRARY_X, TOP, LIBRARY_WIDTH, 28.0),
            22.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        // Dropped on the library, an effect of the blade is taken out.
        let dragging_on = self
            .lab_build
            .drag
            .filter(|key| present.iter().any(|present| present == key));
        let pointer = self.lab_build.pointer;
        let library = [
            LIBRARY_X - 18.0,
            TOP - 10.0,
            LIBRARY_WIDTH + 36.0,
            BOTTOM - TOP + 10.0,
        ];
        self.lab_build
            .spots
            .push((Spot::Zone(Place::Library), library));
        if let Some(key) = dragging_on {
            let over = pointer.is_some_and(|at| self.lab_build.zone_at(at) == Some(Place::Library));
            let fine = builder::may_go(shape, key, Place::Library).is_ok();
            let [x, y, width, height] = library;
            push(
                &mut self.ui,
                DrawCommand::Border {
                    rect: frame.rect(x, y, width, height),
                    radius: 16.0 * s,
                    width: 2.0 * s,
                    color: match (fine, over) {
                        (true, true) => color::GOLD_BRIGHT,
                        (true, false) => color::alpha(color::GOLD, 0.45),
                        (false, true) => color::EMBER,
                        (false, false) => color::alpha(color::EMBER, 0.3),
                    },
                },
            );
        }
        let chosen = self.lab_build.kind.min(Kind::ALL.len() - 1);
        // Each tab its name's width; all of them narrowed together if they would not fit.
        let natural = |label: &str| {
            crate::text::display_width(label, 18.0) * crate::text::style::current().scale.max(0.5)
                + 18.0
        };
        let total: f32 = Kind::ALL
            .iter()
            .map(|kind| natural(kind.label()) + 4.0)
            .sum();
        let fit = (LIBRARY_WIDTH / total).min(1.0);
        let mut x = LIBRARY_X;
        for (index, kind) in Kind::ALL.into_iter().enumerate() {
            let label = kind.label();
            let width = natural(label) * fit;
            let active = index == chosen;
            let hovered = pointer
                .is_some_and(|at| self.lab_build.spot_at(at, false) == Some(Spot::Tab(index)));
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(x, TABS_Y - 14.0, width, 28.0),
                18.0 * fit * s,
                if active {
                    color::GOLD_BRIGHT
                } else if hovered {
                    color::TEXT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            if active {
                push(
                    &mut self.ui,
                    DrawCommand::RoundedRect {
                        rect: frame.rect(x + 6.0, TABS_Y + 13.0, width - 12.0, 2.5),
                        radius: 1.0 * s,
                        color: color::GOLD,
                    },
                );
            }
            self.lab_build
                .spots
                .push((Spot::Tab(index), [x, TABS_Y - 16.0, width, 34.0]));
            x += width + 4.0 * fit;
        }
        let kind = Kind::ALL[chosen];
        for (index, effect) in builder::of_kind(kind).enumerate() {
            let x = LIBRARY_X + (index % 2) as f32 * (CARD_WIDTH + 12.0);
            let y = CARDS_Y + (index / 2) as f32 * CARD_STEP;
            let rect = [x, y, CARD_WIDTH, CARD_HEIGHT];
            let on = present.iter().any(|key| key == effect.key);
            let hovered = pointer.is_some_and(|at| {
                self.lab_build.spot_at(at, false) == Some(Spot::Card(effect.key))
            });
            let lifted = self.lab_build.drag == Some(effect.key) && !on;
            boxed(
                &mut self.ui,
                frame,
                rect,
                if hovered || lifted {
                    color::alpha(color::HOLO, 0.12)
                } else {
                    color::alpha(color::HOLO, 0.05)
                },
                if on {
                    color::alpha(color::GOLD, 0.55)
                } else if hovered {
                    color::alpha(color::GOLD_BRIGHT, 0.8)
                } else {
                    color::alpha(color::HOLO, 0.22)
                },
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", effect.name),
                frame.rect(x + 12.0, y + 6.0, CARD_WIDTH - 24.0, 26.0),
                19.0 * s,
                if ready { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let (words, colour) = if !on {
                (effect.short.to_owned(), color::QUIET)
            } else if kind == Kind::Core {
                ("On the blade".to_owned(), color::GOLD)
            } else if let Some(group) = shape.group_of(effect.key) {
                (format!("In {}", shape.groups[group].name), color::GOLD)
            } else if effect.key == "orbit" {
                ("With its glints".to_owned(), color::GOLD)
            } else {
                ("Always on".to_owned(), color::GOLD)
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{words}"),
                frame.rect(x + 12.0, y + 32.0, CARD_WIDTH - 24.0, 20.0),
                13.5 * s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
            self.lab_build.spots.push((Spot::Card(effect.key), rect));
        }
        left_lines(
            &mut self.ui,
            frame,
            "Drag an effect onto the blade, into a group or out again; rest on one to read what it draws.",
            [LIBRARY_X, BOTTOM - 46.0, LIBRARY_WIDTH],
            (60, 2),
            14.0,
            color::QUIET,
            TextAlign::Start,
        );
    }

    /// The groups: the blade's name and arrows, then its own make, the effects always on,
    /// each group and room for a new one; or why the blade shown cannot be built.
    fn build_groups(
        &mut self,
        frame: &Frame,
        shape: &Shape,
        present: &[String],
        built: Result<(), Unbuilt>,
        skin: Option<&'static str>,
    ) {
        let s = frame.s;
        glass(
            &mut self.ui,
            frame,
            [
                GROUPS_X - 18.0,
                TOP - 10.0,
                GROUPS_WIDTH + 36.0,
                BOTTOM - TOP + 10.0,
            ],
        );
        let name = match skin {
            Some(id) => crate::unlockables::blade_skin(id).map_or(id, |skin| skin.name),
            None => "Stock blade",
        };
        let arrow = 38.0;
        kit::button(
            &mut self.ui,
            frame,
            [GROUPS_X, TOP - 2.0, arrow, 36.0],
            "<",
            false,
            true,
            false,
            PREVIOUS_TOKEN,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(
                GROUPS_X + arrow + 10.0,
                TOP - 2.0,
                GROUPS_WIDTH - 260.0,
                36.0,
            ),
            24.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        kit::button(
            &mut self.ui,
            frame,
            [GROUPS_X + GROUPS_WIDTH - 196.0, TOP - 2.0, arrow, 36.0],
            ">",
            false,
            true,
            false,
            NEXT_TOKEN,
        );
        let (undo, redo) = self
            .lab_build
            .editor
            .as_ref()
            .map_or((false, false), Editor::can_undo);
        kit::button(
            &mut self.ui,
            frame,
            [GROUPS_X + GROUPS_WIDTH - 150.0, TOP - 2.0, 72.0, 36.0],
            "Undo",
            false,
            undo,
            false,
            UNDO_TOKEN,
        );
        kit::button(
            &mut self.ui,
            frame,
            [GROUPS_X + GROUPS_WIDTH - 72.0, TOP - 2.0, 72.0, 36.0],
            "Redo",
            false,
            redo,
            false,
            REDO_TOKEN,
        );
        let mut y = TOP + 46.0;
        if let Err(unbuilt) = built {
            let words = match &unbuilt {
                Unbuilt::Stock => "The stock blade has no file to build. The arrows (or Left and Right) choose another blade; the lab's own are first.".to_owned(),
                Unbuilt::Locked => "This blade's own shader draws most of it (a signature, or a design of the Blade Lab's), so the builder cannot take it apart. Its parts switch on the Shaders tab.".to_owned(),
                Unbuilt::Elsewhere => "This blade comes from the packs. Copy it into the lab's folder to build on it: the copy is drawn in its place on this PC only, the pack's left as it is.".to_owned(),
                Unbuilt::Failed(why) => format!("Its files do not open: {why}"),
            };
            left_lines(
                &mut self.ui,
                frame,
                &words,
                [GROUPS_X, y, GROUPS_WIDTH],
                (64, 4),
                16.0,
                color::MUTED,
                TextAlign::Start,
            );
            if matches!(unbuilt, Unbuilt::Elsewhere) {
                kit::button(
                    &mut self.ui,
                    frame,
                    [GROUPS_X, y + 104.0, 240.0, 42.0],
                    "Copy into the lab",
                    true,
                    true,
                    false,
                    COPY_TOKEN,
                );
            }
            return;
        }
        // Why the files do not read, why the last change was refused, else where they are.
        let revertable = self
            .lab_build
            .editor
            .as_ref()
            .is_some_and(Editor::revertable);
        let (status, colour) = match (
            self.lab_build.editor.as_ref().and_then(Editor::problem),
            &self.lab_build.note,
        ) {
            (Some(why), _) => (format!("Does not read: {why}"), color::EMBER),
            (None, Some((why, _))) => (why.clone(), color::EMBER),
            (None, None) if self.lab_build.editor.as_ref().is_some_and(Editor::unsaved) => {
                ("Shown; writing the files…".to_owned(), color::QUIET)
            }
            (None, None) if revertable => (
                "Saved. Revert puts it back as it was before your first change.".to_owned(),
                color::QUIET,
            ),
            (None, None) => ("Saved in the lab's folder.".to_owned(), color::QUIET),
        };
        left_lines(
            &mut self.ui,
            frame,
            &status,
            [GROUPS_X, y - 8.0, GROUPS_WIDTH - 108.0],
            (64, 2),
            14.0,
            colour,
            TextAlign::Start,
        );
        kit::button(
            &mut self.ui,
            frame,
            [GROUPS_X + GROUPS_WIDTH - 96.0, y - 11.0, 96.0, 26.0],
            "Revert",
            false,
            revertable,
            false,
            REVERT_TOKEN,
        );
        y += 30.0;
        let orbit = present.iter().any(|key| key == "orbit");
        let dragged = self.lab_build.drag;
        let pointer = self.lab_build.pointer;
        let over = pointer.and_then(|at| self.lab_build.zone_at(at));
        let hovered_chip = pointer.and_then(|at| match self.lab_build.spot_at(at, false) {
            Some(Spot::Chip(key)) => Some(key),
            _ => None,
        });
        // A box's edge while an effect is dragged: gold where it may go, the one under
        // the pointer bright; ember under the pointer where it may not.
        let edge_of = |place: Place, rest: Color| -> Color {
            let Some(key) = dragged else {
                return rest;
            };
            let fine = builder::may_go(shape, key, place).is_ok();
            match (fine, over == Some(place)) {
                (true, true) => color::GOLD_BRIGHT,
                (true, false) => color::alpha(color::GOLD, 0.45),
                (false, true) => color::EMBER,
                (false, false) => rest,
            }
        };
        let inner = GROUPS_WIDTH - BOX_PAD * 2.0;
        // The blade's own make.
        let core = chips(&shape.core, false);
        let (rects, bottom) = flow(&core, GROUPS_X + BOX_PAD, y + BOX_TITLE, inner);
        let height = (bottom - y).max(BOX_TITLE + CHIP_HEIGHT) + BOX_PAD;
        boxed(
            &mut self.ui,
            frame,
            [GROUPS_X, y, GROUPS_WIDTH, height],
            color::alpha(color::HOLO, 0.03),
            edge_of(Place::Core, color::alpha(color::HOLO, 0.18)),
        );
        self.lab_build
            .spots
            .push((Spot::Zone(Place::Core), [GROUPS_X, y, GROUPS_WIDTH, height]));
        self.box_title(
            frame,
            "THE BLADE ITSELF",
            "every blade has it",
            [GROUPS_X, y],
            color::MUTED,
        );
        for (key, rect) in core.iter().zip(rects) {
            chip(
                &mut self.ui,
                frame,
                rect,
                name_of(key),
                Tone::Core,
                hovered_chip == Some(*key),
            );
            self.lab_build.spots.push((Spot::Chip(key), rect));
        }
        y += height + BOX_GAP;
        // The effects always on.
        let always = chips(&shape.always, orbit);
        let (rects, bottom) = flow(&always, GROUPS_X + BOX_PAD, y + BOX_TITLE, inner);
        let height = (bottom - y).max(BOX_TITLE + CHIP_HEIGHT) + BOX_PAD;
        boxed(
            &mut self.ui,
            frame,
            [GROUPS_X, y, GROUPS_WIDTH, height],
            color::alpha(color::HOLO, 0.05),
            edge_of(Place::Always, color::alpha(color::HOLO, 0.3)),
        );
        self.lab_build.spots.push((
            Spot::Zone(Place::Always),
            [GROUPS_X, y, GROUPS_WIDTH, height],
        ));
        let count = shape.always.len();
        self.box_title(
            frame,
            "ALWAYS ON",
            &format!("{count} effect{}", if count == 1 { "" } else { "s" }),
            [GROUPS_X, y],
            color::HOLO,
        );
        if always.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("Drop an effect here to have it always on."),
                frame.rect(GROUPS_X + BOX_PAD, y + BOX_TITLE, inner, CHIP_HEIGHT),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        for (key, rect) in always.iter().zip(rects) {
            let lifted = dragged == Some(*key);
            chip(
                &mut self.ui,
                frame,
                rect,
                name_of(key),
                if lifted { Tone::Core } else { Tone::Plain },
                hovered_chip == Some(*key),
            );
            self.lab_build.spots.push((Spot::Chip(key), rect));
        }
        y += height + BOX_GAP;
        // The groups, two to a line, and room for one more.
        let column = (GROUPS_WIDTH - BOX_GAP) / 2.0;
        let inner = column - BOX_PAD * 2.0;
        let mut cells: Vec<Option<usize>> = (0..shape.groups.len()).map(Some).collect();
        if shape.groups.len() < builder::MAX_GROUPS {
            cells.push(None);
        }
        let naming = self.lab_build.naming.clone();
        for line in cells.chunks(2) {
            // A line is as tall as its taller box.
            let laid: Vec<LaidBox> = line
                .iter()
                .enumerate()
                .map(|(side, cell)| {
                    let x = GROUPS_X + side as f32 * (column + BOX_GAP);
                    let keys = cell
                        .map_or_else(Vec::new, |index| chips(&shape.groups[index].effects, orbit));
                    let (rects, bottom) = flow(&keys, x + BOX_PAD, y + BOX_TITLE + 6.0, inner);
                    (*cell, keys, rects, bottom)
                })
                .collect();
            let bottom = laid
                .iter()
                .map(|(_, _, _, bottom)| *bottom)
                .fold(y + BOX_TITLE + 6.0 + CHIP_HEIGHT, f32::max);
            let height = bottom - y + BOX_PAD + 14.0;
            for (side, (cell, keys, rects, _)) in laid.into_iter().enumerate() {
                let x = GROUPS_X + side as f32 * (column + BOX_GAP);
                let rect = [x, y, column, height];
                let Some(index) = cell else {
                    // Room for a new group.
                    let place = Place::NewGroup;
                    boxed(
                        &mut self.ui,
                        frame,
                        rect,
                        color::alpha(color::GOLD, 0.02),
                        edge_of(place, color::alpha(color::GOLD, 0.3)),
                    );
                    self.lab_build.spots.push((Spot::Zone(place), rect));
                    let left = builder::MAX_GROUPS - shape.groups.len();
                    text(
                        &mut self.ui,
                        TextFamily::Display,
                        format_args!("+ NEW GROUP"),
                        frame.rect(x + BOX_PAD, y + 6.0, inner, 24.0),
                        17.0 * s,
                        color::GOLD,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                    left_lines(
                        &mut self.ui,
                        frame,
                        &format!(
                            "Drop an effect here: a part its wearer can switch off. {left} left."
                        ),
                        [x + BOX_PAD, y + BOX_TITLE + 4.0, inner],
                        (34, 2),
                        13.5,
                        color::QUIET,
                        TextAlign::Start,
                    );
                    continue;
                };
                let group = &shape.groups[index];
                let place = Place::Group(index);
                boxed(
                    &mut self.ui,
                    frame,
                    rect,
                    color::alpha(color::GOLD, 0.05),
                    edge_of(place, color::alpha(color::GOLD, 0.45)),
                );
                self.lab_build.spots.push((Spot::Zone(place), rect));
                // Its name, or the name being typed; how full it is; its switch; its way out.
                let name_rect = [x + BOX_PAD, y + 4.0, inner - 112.0, 26.0];
                match &naming {
                    Some((named, typed)) if *named == index => kit::field(
                        &mut self.ui,
                        frame,
                        [
                            name_rect[0] - 6.0,
                            name_rect[1],
                            name_rect[2] + 6.0,
                            name_rect[3],
                        ],
                        format_args!("{typed}"),
                        true,
                        true,
                    ),
                    _ => text(
                        &mut self.ui,
                        TextFamily::Display,
                        format_args!("{}", group.name.to_uppercase()),
                        frame.rect(name_rect[0], name_rect[1], name_rect[2], name_rect[3]),
                        17.0 * s,
                        color::GOLD_BRIGHT,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    ),
                }
                self.lab_build.spots.push((Spot::Name(index), name_rect));
                let on = self.lab_build.on.get(index).copied().unwrap_or(true);
                let right = x + column - BOX_PAD - 22.0;
                let switch_x = kit::switch(&mut self.ui, frame, right, y + 17.0, on, false);
                self.lab_build.spots.push((
                    Spot::Switch(index),
                    [switch_x - 4.0, y + 2.0, right - switch_x + 8.0, 30.0],
                ));
                let out = [x + column - BOX_PAD - 16.0, y + 6.0, 20.0, 22.0];
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("×"),
                    frame.rect(out[0], out[1], out[2], out[3]),
                    20.0 * s,
                    if pointer.is_some_and(|at| {
                        self.lab_build.spot_at(at, false) == Some(Spot::Ungroup(index))
                    }) {
                        color::EMBER
                    } else {
                        color::QUIET
                    },
                    FontWeight::Regular,
                    TextAlign::Center,
                );
                self.lab_build.spots.push((Spot::Ungroup(index), out));
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{} of {}", group.effects.len(), builder::MAX_IN_GROUP),
                    frame.rect(x + BOX_PAD, y + height - 21.0, inner, 16.0),
                    12.5 * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::End,
                );
                for (key, rect) in keys.iter().zip(rects) {
                    let lifted = dragged == Some(*key);
                    chip(
                        &mut self.ui,
                        frame,
                        rect,
                        name_of(key),
                        if lifted { Tone::Core } else { Tone::Gold },
                        hovered_chip == Some(*key),
                    );
                    self.lab_build.spots.push((Spot::Chip(key), rect));
                }
            }
            y += height + BOX_GAP;
        }
    }

    /// A box's title at its top left and a quieter word after it.
    fn box_title(
        &mut self,
        frame: &Frame,
        title: &str,
        after: &str,
        [x, y]: [f32; 2],
        colour: Color,
    ) {
        let s = frame.s;
        let width =
            crate::text::display_width(title, 17.0) * crate::text::style::current().scale.max(0.5);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(x + BOX_PAD, y + 4.0, width + 4.0, 24.0),
            17.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{after}"),
            frame.rect(x + BOX_PAD + width + 14.0, y + 5.0, 300.0, 22.0),
            13.5 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The model's room: a drag there turns the camera, the wheel brings it nearer; the
    /// stage's buttons over it.
    fn build_stage(&mut self, frame: &Frame) {
        let [x, y, width, height] = STAGE_AREA;
        self.ui.hit_region(
            super::saber_lab_tab::VIEW_TOKEN,
            frame.rect(x, y, width, height),
        );
        self.lab_stage_buttons(frame, STAGE_BUTTONS_Y);
    }

    /// Over everything else: the dragged effect under the pointer and why it may not go
    /// where it is, or the card of the effect the pointer rests on.
    pub(super) fn lab_build_overlay(&mut self, frame: &Frame) {
        if !self.lab_building() || self.lab_edit.is_some() {
            return;
        }
        let s = frame.s;
        let build = &self.lab_build;
        if let (Some(key), Some([px, py])) = (build.drag, build.pointer) {
            let width = chip_width(name_of(key));
            let rect = [px - width * 0.5, py - CHIP_HEIGHT * 0.5, width, CHIP_HEIGHT];
            let why = build
                .zone_at([px, py])
                .and_then(|place| builder::may_go(&build.shape, key, place).err());
            let words = why.map_or(0.0, |why| {
                (crate::text::display_width(why, 15.0) * 0.85 + 24.0).min(420.0)
            });
            let at = [px + width * 0.5 + 10.0, py - 14.0, words, 28.0];
            // The page's words under both are left out (one cover a frame).
            let right = if why.is_some() {
                at[0] + at[2]
            } else {
                rect[0] + rect[2]
            };
            self.ui.cover(frame.rect(
                rect[0] - 4.0,
                rect[1] - 4.0,
                right - rect[0] + 8.0,
                rect[3] + 8.0,
            ));
            chip(&mut self.ui, frame, rect, name_of(key), Tone::Lifted, false);
            if let Some(why) = why {
                boxed(
                    &mut self.ui,
                    frame,
                    at,
                    Color::new(0.1, 0.04, 0.02, 0.96),
                    color::EMBER,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{why}"),
                    frame.rect(at[0] + 10.0, at[1], at[2] - 20.0, at[3]),
                    14.0 * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
            }
            return;
        }
        let Some((spot, since)) = build.rest else {
            return;
        };
        let rested = Instant::now().saturating_duration_since(since);
        if rested < REST || build.press.is_some() {
            return;
        }
        let key = match spot {
            Spot::Card(key) | Spot::Chip(key) => key,
            _ => return,
        };
        let (Some(effect), Some([x, y, width, height])) =
            (builder::effect_of(key), build.rect_of(spot))
        else {
            return;
        };
        let on = build
            .editor
            .as_ref()
            .is_some_and(|editor| editor.present().iter().any(|present| present == key));
        let how = match (effect.kind, on, spot) {
            (Kind::Core, _, _) => "Every blade has it. A click opens its fields.",
            (_, false, _) => {
                "Drag it onto the blade: always on, or into a group. A click puts it on always."
            }
            (_, true, Spot::Card(_)) => "On the blade. A click opens its fields.",
            _ if key == "orbit" => {
                "Goes with its glint or holes. A click opens its fields; dragged to the library, it comes off."
            }
            _ => {
                "A click opens its fields. Drag it to another group, or to the library to take it off."
            }
        };
        let card = Card {
            title: effect.name.to_owned(),
            colour: if on { color::GOLD_BRIGHT } else { color::HOLO },
            lines: vec![Line::how(effect.about), Line::status(how)],
        };
        let fade = ((rested - REST).as_secs_f32() / 0.12).min(1.0);
        hover::draw(
            &mut self.ui,
            frame,
            frame.rect(x, y, width, height),
            &card,
            fade,
        );
    }

    /// A click on one of the builder's buttons; `None` for any other token.
    pub(super) fn lab_build_click(&mut self, token: u16) -> Option<PanelAction> {
        match token {
            VIEW_SWITCH => self.switch_lab_view(),
            _ if !self.lab_building() => return None,
            UNDO_TOKEN => {
                if let Some(editor) = &mut self.lab_build.editor {
                    editor.undo();
                }
            }
            REDO_TOKEN => {
                if let Some(editor) = &mut self.lab_build.editor {
                    editor.redo();
                }
            }
            REVERT_TOKEN => {
                if let Some(editor) = &mut self.lab_build.editor {
                    editor.revert();
                }
            }
            COPY_TOKEN => {
                if let Some(id) = self.lab_edit_target()
                    && let Err(why) = crate::blade_editor::copy_into_lab(id)
                {
                    self.lab_build.failed = Some((id, why));
                }
            }
            PREVIOUS_TOKEN => self.step_lab_blade(-1),
            NEXT_TOKEN => self.step_lab_blade(1),
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// A pointer event on the builder: a press on an effect, a tab or a group's name,
    /// switch or way out takes it; moved far enough, an effect is dragged, and let go it
    /// drops where the pointer is; let go where it was pressed, it is a click. `None`
    /// leaves the event to the tab.
    pub(super) fn lab_build_pointer(&mut self, event: InputEvent) -> Option<PanelAction> {
        if !self.lab_building() || self.lab_edit.is_some() {
            return None;
        }
        let frame = self.lab_build.frame?;
        let at = |position: sjk_ui::Vec2| {
            [
                (position.x - frame.origin[0]) / frame.s,
                (position.y - frame.origin[1]) / frame.s,
            ]
        };
        match event {
            InputEvent::PointerMove(position) => {
                let at = at(position);
                let build = &mut self.lab_build;
                build.pointer = Some(at);
                if let Some((Spot::Card(key) | Spot::Chip(key), from)) = build.press
                    && build.drag.is_none()
                    && (at[0] - from[0]).hypot(at[1] - from[1]) > DRAG_SLOP
                {
                    build.drag = Some(key);
                    build.naming = None;
                }
                None
            }
            InputEvent::PointerPress { position, .. } => {
                let at = at(position);
                let build = &mut self.lab_build;
                build.pointer = Some(at);
                let spot = build.spot_at(at, false)?;
                build.press = Some((spot, at));
                Some(PanelAction::None)
            }
            InputEvent::PointerRelease { position, .. } => {
                let at = at(position);
                let build = &mut self.lab_build;
                build.pointer = Some(at);
                let (pressed, _) = build.press.take()?;
                if let Some(key) = build.drag.take() {
                    if let Some(place) = build.zone_at(at) {
                        self.drop_effect(key, place);
                    }
                    return Some(PanelAction::None);
                }
                // Let go off what was pressed: nothing.
                if build.spot_at(at, false) != Some(pressed) {
                    return Some(PanelAction::None);
                }
                Some(self.click_spot(pressed))
            }
            InputEvent::PointerLeave => {
                let build = &mut self.lab_build;
                build.pointer = None;
                build.press = None;
                build.drag.take().map(|_| PanelAction::None)
            }
            _ => None,
        }
    }

    /// What a click on `spot` does.
    fn click_spot(&mut self, spot: Spot) -> PanelAction {
        match spot {
            Spot::Tab(index) => self.lab_build.kind = index,
            Spot::Card(key) => {
                let on =
                    self.lab_build.editor.as_ref().is_some_and(|editor| {
                        editor.present().iter().any(|present| present == key)
                    });
                if on {
                    self.open_lab_fields(key);
                } else {
                    let place = if builder::effect_of(key)
                        .is_some_and(|effect| effect.kind == Kind::Core)
                    {
                        Place::Core
                    } else {
                        Place::Always
                    };
                    self.drop_effect(key, place);
                }
            }
            Spot::Chip(key) => self.open_lab_fields(key),
            Spot::Name(index) => {
                let name = self
                    .lab_build
                    .shape
                    .groups
                    .get(index)
                    .map(|group| group.name.clone());
                if let Some(name) = name {
                    crate::text_entry::select_all(&name);
                    self.lab_build.naming = Some((index, name));
                }
            }
            Spot::Switch(index) => {
                let (Some(skin), Some(group)) = (
                    self.lab_build.editor.as_ref().map(Editor::id),
                    self.lab_build.shape.groups.get(index),
                ) else {
                    return PanelAction::None;
                };
                let on = self.lab_build.on.get(index).copied().unwrap_or(true);
                return PanelAction::SkinOption {
                    skin,
                    option: group.id.clone(),
                    on: !on,
                };
            }
            Spot::Ungroup(index) => {
                let build = &mut self.lab_build;
                if let Some(editor) = &mut build.editor {
                    let options = builder::ungrouped(&editor.options(), index);
                    if let Err(why) = editor.set_options(&options) {
                        build.refuse(why);
                    }
                }
            }
            Spot::Zone(_) => {}
        }
        PanelAction::None
    }

    /// Drop effect `key` at `place`: put on the blade from the library, moved between
    /// its groups, or taken off back in the library; refused, saying why, where it may
    /// not go or where the file would not read.
    fn drop_effect(&mut self, key: &'static str, place: Place) {
        let build = &mut self.lab_build;
        let Some(editor) = &mut build.editor else {
            return;
        };
        let on = editor.present().iter().any(|present| present == key);
        if let Err(why) = builder::may_go(&build.shape, key, place) {
            build.refuse(why);
            return;
        }
        let options = editor.options();
        let result = match (on, place) {
            (false, Place::Library) | (true, Place::Core) => Ok(()),
            (true, Place::Library) => {
                editor.remove_section(key, &builder::moved(&options, key, Place::Library))
            }
            (false, Place::Core) => editor.add_section(key, &options),
            (false, _) => editor.add_section(key, &builder::moved(&options, key, place)),
            (true, _) => editor.set_options(&builder::moved(&options, key, place)),
        };
        if let Err(why) = result {
            let name = name_of(key);
            build.refuse(match place {
                Place::Library => format!("{name} cannot come off: {why}."),
                _ if !on => format!("{name} cannot go on: {why}."),
                _ => format!("{name} cannot move there: {why}."),
            });
        } else {
            build.note = None;
        }
    }

    /// A key on the builder: the name being typed takes every key; Ctrl+Z and Ctrl+Y undo
    /// and redo, Delete takes the effect the pointer rests on off the blade, Escape lets
    /// go of a drag. `None` leaves it to the tab (the stage's keys).
    pub(super) fn lab_build_key(
        &mut self,
        event: &KeyEvent,
        key: KeyCode,
        shift: bool,
        control: bool,
    ) -> Option<PanelAction> {
        if !self.lab_building() || self.lab_edit.is_some() {
            return None;
        }
        let build = &mut self.lab_build;
        if let Some((index, typed)) = &mut build.naming {
            match key {
                KeyCode::Escape => build.naming = None,
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let (index, typed) = (*index, typed.clone());
                    build.naming = None;
                    if let Some(editor) = &mut build.editor {
                        match builder::renamed(&editor.options(), index, &typed) {
                            Some(options) => {
                                if let Err(why) = editor.set_options(&options) {
                                    build.refuse(why);
                                }
                            }
                            None => {
                                build.refuse("A group's name is 1 to 24 letters, digits or marks.")
                            }
                        }
                    }
                }
                _ => {
                    crate::text_entry::edit(
                        typed,
                        key,
                        event.text.as_deref(),
                        shift,
                        control,
                        NAME_MAX,
                        true,
                    );
                }
            }
            return Some(PanelAction::None);
        }
        match key {
            KeyCode::Escape if build.drag.is_some() => {
                build.drag = None;
                build.press = None;
            }
            KeyCode::KeyZ if control && shift => {
                build.editor.as_mut()?.redo();
            }
            KeyCode::KeyZ if control => {
                build.editor.as_mut()?.undo();
            }
            KeyCode::KeyY if control => {
                build.editor.as_mut()?.redo();
            }
            KeyCode::KeyB => self.switch_lab_view(),
            KeyCode::KeyE if !control => self.open_lab_fields(""),
            KeyCode::Delete => {
                let key = match build.pointer.and_then(|at| build.spot_at(at, false)) {
                    Some(Spot::Chip(key)) => key,
                    _ => return Some(PanelAction::None),
                };
                self.drop_effect(key, Place::Library);
            }
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// The builder's keys for the line at the bottom.
    pub(super) fn lab_build_keys(&self, keys: &mut Vec<(&'static [&'static str], &str)>) {
        if self.lab_build.naming.is_some() {
            keys.push((&["Enter"], "name it"));
            keys.push((&["Esc"], "keep the name"));
            return;
        }
        keys.push((&["Del"], "take off"));
        keys.push((&["Ctrl", "Z"], "undo"));
        keys.push((
            &["Space"],
            if self.lab.swinging { "stand" } else { "swing" },
        ));
        keys.push((&["I"], if self.lab.lit { "switch off" } else { "ignite" }));
        keys.push((&["C"], "clash"));
        keys.push((&["Left", "Right"], "blade"));
        keys.push((&["B"], "stage controls"));
    }
}
