//! The Collection's Nameplates tab: ornaments for the nameplate every player sees over
//! the player's head (`docs/client.md`, Nameplates). None exist yet, so the tab shows
//! where they will go: three empty places on the left (a crest before the name, a
//! frame round the plate, a trail under it) and, over the player's model beside the
//! page, their nameplate as a match draws it near in the style they chose (the holo
//! frame, the capsule or the classic plate, `cg_nameplateStyle`, with its bars while
//! they share), with dashed marks at those places.
//! Under the words beside it, Style chooses that look, and See and share nameplates
//! switches `cg_nameplate`, the one setting for seeing other players' nameplates and
//! sharing the player's own health, shield and Force
//! ([`crate::hud::nameplate::see_and_share`]).

use super::view::{LEFT_X, MODEL_AREA, RIGHT_X, glow};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};
use std::f32::consts::{PI, TAU};

/// The empty places: their top, size and the gap between them.
const PLACES_TOP: f32 = 300.0;
const PLACE: f32 = 240.0;
const PLACE_GAP: f32 = 30.0;
/// The nameplate's holo frame, at twice its design's size (its name 30 pixels), its
/// plate a little more opaque than in a match as it stands on the stage; the crest's
/// place before the name and the gap after it; and the bars' colours (the Radial HUD's
/// shield, health and Force, as the nameplates take them).
const NAME_SIZE: f32 = 30.0;
const PLATE_ALPHA: f32 = 0.78;
/// The classic plate's pixel at that size: its names are 18 of them at the default
/// `cg_nameplateScale` 0.5.
const CLASSIC_U: f32 = NAME_SIZE / 18.0;
const CREST: f32 = 28.0;
const PLATE_GAP: f32 = 12.0;
const SHIELD: Color = Color::new(0.25, 0.82, 0.5, 1.0);
const HEALTH: Color = Color::new(0.9, 0.28, 0.3, 1.0);
const FORCE: Color = Color::new(0.29, 0.66, 1.0, 1.0);

/// The words beside the model: their card's left edge, width and foot, and the room
/// inside it; a line of the description holds this many characters at the ordinary
/// text size, a line of the other words this many.
const WORDS_X: f32 = 1_360.0;
const WORDS_WIDTH: f32 = RIGHT_X - WORDS_X;
const WORDS_BOTTOM: f32 = 990.0;
const WORDS_PAD: f32 = 26.0;
const ABOUT_CHARS: f32 = 44.0;
const NOTE_CHARS: f32 = 50.0;
const ABOUT: &str = "What every SJK player sees over your head in a match: your picture and your name, and while you share, your shield, health and Force under them.";
const ORNAMENTS: &str =
    "Ornaments will go before your name, round your plate and under it: the dashed marks.";
/// The Style row's height, from its top to the See and share row's.
const STYLE_ROW: f32 = 52.0;
/// The See and share nameplates row's width, from its name to the switch's right edge:
/// the card's inside, less the ring round the row.
const SHARE_ROW: f32 = WORDS_WIDTH - WORDS_PAD * 2.0 - 28.0;
/// How long one pulse of the See and share nameplates row takes.
const SHARE_PULSE_SECONDS: f32 = 1.2;

/// How lit the See and share nameplates row is `seconds` into the page, 0 to 1 and back each
/// [`SHARE_PULSE_SECONDS`], eased at both ends.
fn share_pulse(seconds: f32) -> f32 {
    0.5 - 0.5 * (seconds * TAU / SHARE_PULSE_SECONDS).cos()
}

/// `name` without its colour codes (`^1`).
fn plain(name: &str) -> String {
    let mut plain = String::with_capacity(name.len());
    let mut characters = name.chars();
    while let Some(character) = characters.next() {
        if character == '^' {
            let _ = characters.next();
        } else {
            plain.push(character);
        }
    }
    plain
}

/// A dashed outline round `rect` (frame pixels).
fn dashed_rect(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y, width, height]: [f32; 4],
    colour: Color,
) {
    const DASH: f32 = 9.0;
    const STEP: f32 = 16.0;
    let mut along = 0.0;
    while along < width {
        let length = DASH.min(width - along);
        for top in [y, y + height - 1.2] {
            let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
                rect: frame.rect(x + along, top, length, 1.2),
                color: colour,
            });
        }
        along += STEP;
    }
    let mut down = 0.0;
    while down < height {
        let length = DASH.min(height - down);
        for left in [x, x + width - 1.2] {
            let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
                rect: frame.rect(left, y + down, 1.2, length),
                color: colour,
            });
        }
        down += STEP;
    }
}

/// A sketch of a holo frame in outline (frame pixels): the hexagon in `hex`, the plate
/// with its cut corner in `plate`.
fn sketch(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    hex: [f32; 4],
    plate: [f32; 4],
    colour: Color,
) {
    use crate::hud::nameplate_math::hexagon;
    let [x, y, width, height] = plate;
    let cut = 8.0_f32.min(height * 0.4);
    let outline = [
        [x, y],
        [x + width - cut, y],
        [x + width, y + cut],
        [x + width, y + height],
        [x, y + height],
    ];
    let corners = hexagon(sjk_ui::Rect::new(hex[0], hex[1], hex[2], hex[3]));
    for shape in [&outline[..], &corners[..]] {
        for (index, from) in shape.iter().enumerate() {
            let to = shape[(index + 1) % shape.len()];
            let _ = canvas.draw_list_mut().push(DrawCommand::Line {
                from: frame.point(from[0], from[1]),
                to: frame.point(to[0], to[1]),
                width: 1.5 * frame.s,
                color: colour,
            });
        }
    }
}

/// A dashed arc round `centre` (frame pixels) from `start` over `sweep` radians.
fn dashed_arc(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    centre: [f32; 2],
    radius: f32,
    (start, sweep): (f32, f32),
    colour: Color,
) {
    let dashes = ((radius * sweep.abs()) / 14.0).ceil().max(2.0) as usize;
    let each = sweep / dashes as f32;
    for dash in 0..dashes {
        let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
            center: frame.point(centre[0], centre[1]),
            radius: radius * frame.s,
            width: 1.5 * frame.s,
            start: start + each * dash as f32,
            sweep: each * 0.55,
            color: colour,
            knockout: None,
        });
    }
}

/// The bars a previewed plate holds: all three while the player shares, else none.
fn rows(share: bool) -> crate::hud::nameplate_math::Rows {
    crate::hud::nameplate_math::Rows {
        health: share,
        shield: share,
        force: share,
    }
}

/// `colour` at opacity `alpha`.
fn solid(colour: Color, alpha: f32) -> Color {
    Color::new(colour.r, colour.g, colour.b, alpha)
}

/// A filled convex polygon through `points` (frame pixels).
fn polygon(
    list: &mut sjk_ui::DrawList,
    frame: &Frame,
    points: &[[f32; 2]],
    colour: Color,
    texture: Option<(sjk_ui::TextureId, [[f32; 2]; 2])>,
) {
    let mut corners = [[0.0; 2]; sjk_ui::POLYGON_POINTS];
    for (corner, at) in corners.iter_mut().zip(points) {
        *corner = frame.point(at[0], at[1]);
    }
    let _ = list.push(DrawCommand::Polygon {
        points: corners,
        count: points.len().min(sjk_ui::POLYGON_POINTS) as u8,
        color: colour,
        texture,
    });
}

/// A line from `from` to `to` (frame pixels), `width` frame pixels thick.
fn line(
    list: &mut sjk_ui::DrawList,
    frame: &Frame,
    from: [f32; 2],
    to: [f32; 2],
    width: f32,
    colour: Color,
) {
    let _ = list.push(DrawCommand::Line {
        from: frame.point(from[0], from[1]),
        to: frame.point(to[0], to[1]),
        width: width * frame.s,
        color: colour,
    });
}

/// A rounded bar in `bar` (frame pixels) on `track`, filled to `share` in `colour`, with
/// the classic plate's outline when given its pixel (`outline`).
fn rounded_bar(
    list: &mut sjk_ui::DrawList,
    frame: &Frame,
    bar: sjk_ui::Rect,
    share: f32,
    colour: Color,
    track: Color,
    outline: Option<f32>,
) {
    let screen = frame.rect(bar.x, bar.y, bar.width, bar.height);
    let radius = screen.height * 0.5;
    let _ = list.push(DrawCommand::RoundedRect {
        rect: screen,
        radius,
        color: track,
    });
    let _ = list.push(DrawCommand::RoundedRect {
        rect: sjk_ui::Rect::new(
            screen.x,
            screen.y,
            (screen.width * share).max(screen.height),
            screen.height,
        ),
        radius,
        color: colour,
    });
    if let Some(u) = outline {
        let _ = list.push(DrawCommand::Border {
            rect: screen,
            radius,
            width: (u * frame.s).max(1.0),
            color: Color::new(1.0, 1.0, 1.0, 0.3),
        });
    }
}

impl Panel {
    /// The Nameplates tab.
    pub(super) fn nameplates(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        self.lead(
            frame,
            "No nameplates yet",
            "Ornaments for your nameplate, the name every player sees over your head.",
        );
        let places = [
            ("Crest", "Before your name"),
            ("Frame", "Round your plate"),
            ("Trail", "Under your plate"),
        ];
        for (index, (name, place)) in places.into_iter().enumerate() {
            let x = LEFT_X + index as f32 * (PLACE + PLACE_GAP);
            self.empty_place(frame, index, x, name, place);
        }
        super::toys::room_for_more(
            &mut self.ui,
            frame,
            716.0,
            "Nothing to collect here yet: the first ones come later.",
        );
        let at = self.plate_spot(frame);
        self.nameplate(frame, inputs, at);
        self.nameplate_words(frame, inputs);
    }

    /// Empty place `index` from `x`: a dashed niche with a drawing of where its kind
    /// goes, its name and place under it.
    fn empty_place(&mut self, frame: &Frame, index: usize, x: f32, name: &str, place: &str) {
        let s = frame.s;
        let line = color::alpha(color::HOLO, 0.55);
        let faint = color::alpha(color::HOLO, 0.28);
        dashed_rect(&mut self.ui, frame, [x, PLACES_TOP, PLACE, PLACE], faint);
        let middle = [x + PLACE * 0.5, PLACES_TOP + PLACE * 0.5];
        let bar = |canvas: &mut crate::menu_widgets::MenuCanvas, rect: [f32; 4]| {
            let [bx, by, bw, bh] = rect;
            let _ = canvas.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(bx, by, bw, bh),
                radius: 3.0 * s,
                width: 1.5 * s,
                color: faint,
            });
        };
        match index {
            // A crest: a ring with a mark inside, between a plate's picture and its
            // name.
            0 => {
                let left = middle[0] - 90.0;
                sketch(
                    &mut self.ui,
                    frame,
                    [left, middle[1] - 20.0, 44.0, 40.0],
                    [left + 32.0, middle[1] - 16.0, 148.0, 32.0],
                    faint,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                    center: frame.point(left + 62.0, middle[1]),
                    radius: 12.0 * s,
                    width: 2.0 * s,
                    start: 0.0,
                    sweep: TAU,
                    color: line,
                    knockout: None,
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(left + 58.0, middle[1] - 4.0, 8.0, 8.0),
                    radius: 2.0 * s,
                    color: line,
                });
                bar(&mut self.ui, [left + 84.0, middle[1] - 7.0, 82.0, 14.0]);
            }
            // A frame round the plate and its bars, with corner marks.
            1 => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(middle[0] - 76.0, middle[1] - 36.0, 152.0, 72.0),
                    radius: 10.0 * s,
                    width: 2.0 * s,
                    color: line,
                });
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: frame.rect(
                            middle[0] + dx * 84.0 - 3.0,
                            middle[1] + dy * 44.0 - 3.0,
                            6.0,
                            6.0,
                        ),
                        radius: 3.0 * s,
                        color: line,
                    });
                }
                sketch(
                    &mut self.ui,
                    frame,
                    [middle[0] - 66.0, middle[1] - 18.0, 40.0, 36.0],
                    [middle[0] - 38.0, middle[1] - 22.0, 104.0, 44.0],
                    faint,
                );
                for row in 0..3 {
                    let width = if row == 2 { 52.0 } else { 70.0 };
                    bar(
                        &mut self.ui,
                        [
                            middle[0] - 16.0,
                            middle[1] - 2.0 + row as f32 * 7.0,
                            width,
                            3.0,
                        ],
                    );
                }
            }
            // A trail sweeping under the plate.
            _ => {
                sketch(
                    &mut self.ui,
                    frame,
                    [middle[0] - 62.0, middle[1] - 52.0, 34.0, 30.0],
                    [middle[0] - 40.0, middle[1] - 50.0, 100.0, 26.0],
                    faint,
                );
                for (radius, alpha) in [(64.0, 0.55), (48.0, 0.4)] {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                        center: frame.point(middle[0], middle[1] - 40.0),
                        radius: radius * s,
                        width: 2.0 * s,
                        start: PI * 0.25,
                        sweep: PI * 0.5,
                        color: color::alpha(color::HOLO, alpha),
                        knockout: None,
                    });
                }
            }
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(x, PLACES_TOP + PLACE + 16.0, PLACE, 2.0),
            color: color::alpha(color::HOLO, 0.22),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x, PLACES_TOP + PLACE + 30.0, PLACE, 30.0),
            22.0 * s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{place}"),
            frame.rect(x, PLACES_TOP + PLACE + 62.0, PLACE, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
    }

    /// Where the nameplate stands, in frame pixels: over the model's head on the
    /// stage (the renderer says where), over the preview's, or high in the column.
    fn plate_spot(&self, frame: &Frame) -> [f32; 2] {
        match self.backstage {
            Backstage::World { head: Some(head) } => [
                (head[0] - frame.origin[0]) / frame.s,
                (head[1] - frame.origin[1]) / frame.s,
            ],
            Backstage::World { head: None } | Backstage::Preview { .. } => {
                let [x, y, width, _] = MODEL_AREA;
                [x + width * 0.5, y + 70.0]
            }
            // The stage's column, where the model would stand.
            Backstage::None => [1_160.0, 380.0],
        }
    }

    /// The player's nameplate as a match draws it near, its bottom at `at`, in the style
    /// they chose, with the bars while they share (See and share nameplates), and the
    /// dashed marks where ornaments will go: the crest before the name, the frame round
    /// the plate, the trail under it.
    fn nameplate(&mut self, frame: &Frame, inputs: &Inputs<'_>, at: [f32; 2]) {
        use crate::hud::nameplate::Style;
        let name = crate::profile_hub::title(inputs.name);
        let name_width = crate::text::display_width(&plain(name), NAME_SIZE);
        let (bounds, [crest_x, crest_y]) = match inputs.nameplate_style {
            Style::Holo => self.holo_plate(frame, inputs, at, name, name_width),
            Style::Capsule => self.capsule_plate(frame, inputs, at, name, name_width),
            Style::Classic => self.classic_plate(frame, inputs, at, name, name_width),
        };
        let mark = color::alpha(color::HOLO, 0.8);
        // Before the name, the crest's place.
        dashed_arc(
            &mut self.ui,
            frame,
            [crest_x, crest_y],
            CREST * 0.5,
            (0.0, TAU),
            mark,
        );
        // Round the plate, the frame's place, and under it the trail's.
        dashed_rect(
            &mut self.ui,
            frame,
            [
                bounds.x - 9.0,
                bounds.y - 9.0,
                bounds.width + 18.0,
                bounds.height + 18.0,
            ],
            mark,
        );
        dashed_arc(
            &mut self.ui,
            frame,
            [at[0], at[1] - 30.0],
            60.0,
            (PI * 0.28, PI * 0.44),
            mark,
        );
    }

    /// The player's name from `x` with its capitals' middle on `middle`, after the
    /// crest's place.
    fn plate_name(&mut self, frame: &Frame, name: &str, [x, middle]: [f32; 2], width: f32) {
        let line_height = NAME_SIZE * 1.3;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{name}"),
            frame.rect(
                x + CREST + PLATE_GAP,
                middle - line_height * 0.5,
                width + 40.0,
                line_height,
            ),
            NAME_SIZE * frame.s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
    }

    /// The player's picture filling `shape` (frame pixels; a hexagon, else the disc in
    /// `rect`): theirs once loaded, else their initial on their colour.
    fn plate_picture(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        rect: sjk_ui::Rect,
        hexagon: bool,
    ) {
        use crate::hud::nameplate_math as math;
        let name = crate::profile_hub::title(inputs.name);
        let me = inputs.snapshot.and_then(|snapshot| {
            snapshot.me.as_ref().map(|me| {
                (
                    snapshot.key_id.as_str(),
                    me.avatar.as_str(),
                    me.name.as_str(),
                )
            })
        });
        let (key_id, version, hub_name) = me.unwrap_or(("", "", name));
        let texture = crate::avatars::texture(key_id, version);
        let fill = texture.map_or_else(
            || crate::profile_card::stand_in_colour(key_id, hub_name),
            |_| Color::new(1.0, 1.0, 1.0, 1.0),
        );
        let list = self.ui.draw_list_mut();
        if hexagon {
            polygon(
                list,
                frame,
                &math::hexagon(rect),
                fill,
                texture.map(|texture| (texture, math::HEX_UV)),
            );
        } else {
            let screen = frame.rect(rect.x, rect.y, rect.width, rect.height);
            let _ = match texture {
                Some(texture) => list.push(DrawCommand::TexturedQuad {
                    rect: screen,
                    texture,
                    color: fill,
                }),
                None => list.push(DrawCommand::RoundedRect {
                    rect: screen,
                    radius: screen.width * 0.5,
                    color: fill,
                }),
            };
        }
        if texture.is_none() {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", crate::profile_card::initial(hub_name)),
                frame.rect(rect.x, rect.y, rect.width, rect.height),
                rect.height * 0.55 * frame.s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
    }

    /// The holo frame (frame pixels), its picture in its hexagon and the name, with the
    /// gold corner and the bars while they share, else the dotted line: its bounds and
    /// where the crest's place is centred.
    fn holo_plate(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        [x, bottom]: [f32; 2],
        name: &str,
        name_width: f32,
    ) -> (sjk_ui::Rect, [f32; 2]) {
        use crate::hud::nameplate as plate;
        use crate::hud::nameplate_math::{self as math, Holo, HoloSpec};
        let share = inputs.share;
        // The crest's place stands before the name, inside the name's run.
        let holo = Holo::new(&HoloSpec {
            centre_x: x,
            bottom,
            size: NAME_SIZE,
            name_width: CREST + PLATE_GAP + name_width,
            picture: true,
            mark: false,
            client: false,
            duel: false,
            rows: rows(share),
            detail: 1.0,
            proximity: 1.0,
            least_bar: 2.0,
            viewport: [f32::MAX, f32::MAX],
        });
        let k = holo.k;
        let list = self.ui.draw_list_mut();
        let edge = plate::HOLO_EDGE;
        // The plate: a little more opaque than in a match, standing on the stage.
        let outline = holo.outline();
        polygon(
            list,
            frame,
            &outline,
            math::plate_fill(edge, PLATE_ALPHA),
            None,
        );
        let p = holo.plate;
        let mut y = p.bottom() - 1.5;
        while y > p.y + 1.0 {
            let cut = (holo.cut - (y - p.y)).max(0.0);
            line(
                list,
                frame,
                [p.x + 1.0, y],
                [p.right() - 1.0 - cut, y],
                1.0,
                solid(edge, plate::SCAN_ALPHA),
            );
            y -= math::SCAN * k;
        }
        let width = math::EDGE * k;
        let half = width * 0.5;
        let [top_left, cut_start, cut_end, bottom_right, bottom_left] = outline;
        for (from, to) in [
            (
                [top_left[0], top_left[1] + half],
                [cut_start[0], cut_start[1] + half],
            ),
            (
                [cut_start[0], cut_start[1] + half],
                [cut_end[0] - half, cut_end[1]],
            ),
            (
                [cut_end[0] - half, cut_end[1]],
                [bottom_right[0] - half, bottom_right[1]],
            ),
            (
                [bottom_right[0], bottom_right[1] - half],
                [bottom_left[0], bottom_left[1] - half],
            ),
            (
                [bottom_left[0] + half, bottom_left[1] - half],
                [top_left[0] + half, top_left[1] + width],
            ),
        ] {
            line(list, frame, from, to, width, edge);
        }
        if share {
            let [start, corner, end] = holo.gold();
            for (from, to) in [(start, corner), (corner, end)] {
                line(
                    list,
                    frame,
                    from,
                    to,
                    math::GOLD_WIDTH * k,
                    plate::SHARED_GOLD,
                );
            }
            // Shield, health and Force, as the HUD's bars colour them: shield and health
            // in segments.
            let track = solid(edge, plate::TRACK_ALPHA);
            for (bar, colour, share, segmented) in [
                (holo.bars.shield, SHIELD, 0.45, true),
                (holo.bars.health, HEALTH, 0.8, true),
                (holo.bars.force, FORCE, 0.65, false),
            ] {
                let Some(bar) = bar else {
                    continue;
                };
                let (on, off) = if segmented {
                    (math::SEGMENT * k, math::SEGMENT_GAP * k)
                } else {
                    (bar.width, 0.0)
                };
                let filled = bar.x + bar.width * share;
                for (start, end) in math::segments(bar.x, bar.width, on, off) {
                    for (from, to, colour) in [
                        (start, end.min(filled), colour),
                        (start.max(filled), end, track),
                    ] {
                        if to > from {
                            let _ = list.push(DrawCommand::SolidRect {
                                rect: frame.rect(from, bar.y, to - from, bar.height),
                                color: colour,
                            });
                        }
                    }
                }
            }
        } else {
            let dotted = holo.dotted;
            for (start, end) in
                math::dashes(dotted.x, dotted.width, math::DOT * k, math::DOT_GAP * k)
            {
                let _ = list.push(DrawCommand::SolidRect {
                    rect: frame.rect(start, dotted.y, end - start, dotted.height),
                    color: solid(color::QUIET, 0.5),
                });
            }
        }
        // The picture in its hexagon.
        let hex = holo.hex.unwrap_or(p);
        polygon(list, frame, &math::hexagon(hex), plate::HOLO_HEX, None);
        self.plate_picture(frame, inputs, math::inset(hex, 2.0 * k), true);
        self.plate_name(frame, name, [holo.name.x, holo.middle], name_width);
        let bounds = sjk_ui::Rect::new(hex.x, p.y, p.right() - hex.x, p.height);
        (bounds, [holo.name.x + CREST * 0.5, holo.middle])
    }

    /// The capsule (frame pixels): the pill with the picture and the name, gold-edged
    /// with the tray of bars under it while they share: its bounds and where the crest's
    /// place is centred.
    fn capsule_plate(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        [x, bottom]: [f32; 2],
        name: &str,
        name_width: f32,
    ) -> (sjk_ui::Rect, [f32; 2]) {
        use crate::hud::nameplate::capsule::{
            Capsule, CapsuleSpec, NEUTRAL_EDGE, PILL_FILL, TRACK,
        };
        let share = inputs.share;
        let capsule = Capsule::new(&CapsuleSpec {
            centre_x: x,
            bottom,
            size: NAME_SIZE,
            name_width: CREST + PLATE_GAP + name_width,
            picture: true,
            mark: false,
            client: false,
            duel: false,
            rows: rows(share),
            detail: 1.0,
            proximity: 1.0,
            least_bar: 2.0,
            viewport: [f32::MAX, f32::MAX],
        });
        let k = capsule.k;
        let s = frame.s;
        let fill = Color {
            a: PLATE_ALPHA,
            ..PILL_FILL
        };
        let edge = if share {
            crate::hud::nameplate::SHARED_GOLD
        } else {
            NEUTRAL_EDGE
        };
        let list = self.ui.draw_list_mut();
        let mut bounds = capsule.pill;
        // The tray first, so the pill closes its top.
        if let Some(tray) = capsule.tray {
            let corner = capsule.tray_corner(&tray) * s;
            let rect = tray.frame;
            let screen = frame.rect(rect.x, rect.y, rect.width, rect.height);
            let _ = list.push(DrawCommand::RoundedRect {
                rect: screen,
                radius: corner,
                color: fill,
            });
            let _ = list.push(DrawCommand::Border {
                rect: screen,
                radius: corner,
                width: k * s,
                color: edge,
            });
            for (bar, colour, share) in [
                (tray.shield, SHIELD, 0.45),
                (tray.health, HEALTH, 0.8),
                (tray.force, FORCE, 0.65),
            ] {
                let Some(bar) = bar else {
                    continue;
                };
                rounded_bar(list, frame, bar, share, colour, TRACK, None);
            }
            bounds.height = rect.bottom() - bounds.y;
        }
        let pill = capsule.pill;
        let screen = frame.rect(pill.x, pill.y, pill.width, pill.height);
        let _ = list.push(DrawCommand::RoundedRect {
            rect: screen,
            radius: screen.height * 0.5,
            color: fill,
        });
        let _ = list.push(DrawCommand::Border {
            rect: screen,
            radius: screen.height * 0.5,
            width: k * s,
            color: edge,
        });
        if let Some(picture) = capsule.picture {
            self.plate_picture(frame, inputs, picture, false);
        }
        self.plate_name(frame, name, [capsule.name.x, capsule.middle], name_width);
        (bounds, [capsule.name.x + CREST * 0.5, capsule.middle])
    }

    /// The classic plate (frame pixels): the name centred over a dark plate of rounded
    /// bars framed in gold while they share (the name alone otherwise): its bounds and
    /// where the crest's place is centred.
    fn classic_plate(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        [x, bottom]: [f32; 2],
        name: &str,
        name_width: f32,
    ) -> (sjk_ui::Rect, [f32; 2]) {
        use crate::hud::nameplate::classic::Stack;
        let u = CLASSIC_U;
        let s = frame.s;
        let share = inputs.share;
        let line = NAME_SIZE * 1.15;
        let stack = share.then(|| Stack::new(x, bottom, rows(share), u, [f32::MAX, f32::MAX]));
        let top = stack.map_or(bottom, |stack| stack.frame.y) - line;
        // The crest and the name, centred together over the plate.
        let run = CREST + PLATE_GAP + name_width;
        let left = x - run * 0.5;
        let middle = top + line * 0.5;
        let mut bounds = sjk_ui::Rect::new(left, top, run, line);
        if let Some(stack) = stack {
            let list = self.ui.draw_list_mut();
            let rect = stack.frame;
            let screen = frame.rect(rect.x, rect.y, rect.width, rect.height);
            let _ = list.push(DrawCommand::RoundedRect {
                rect: screen,
                radius: 4.0 * u * s,
                color: Color::new(0.03, 0.04, 0.06, PLATE_ALPHA),
            });
            let _ = list.push(DrawCommand::Border {
                rect: screen,
                radius: 4.0 * u * s,
                width: 1.6 * u * s,
                color: crate::hud::nameplate::SHARED_GOLD,
            });
            for (bar, colour, share) in [
                (stack.shield, SHIELD, 0.45),
                (stack.health, HEALTH, 0.8),
                (stack.force, FORCE, 0.65),
            ] {
                let Some(bar) = bar else {
                    continue;
                };
                rounded_bar(
                    list,
                    frame,
                    bar,
                    share,
                    colour,
                    Color::new(0.0, 0.0, 0.0, 0.6),
                    Some(u),
                );
            }
            let left = bounds.x.min(rect.x);
            let right = bounds.right().max(rect.right());
            bounds = sjk_ui::Rect::new(left, top, right - left, rect.bottom() - top);
        }
        self.plate_name(frame, name, [left, middle], name_width);
        (bounds, [left + CREST * 0.5, middle])
    }

    /// What the nameplate is, beside the model, and the See and share nameplates switch,
    /// on a dark card at the foot of the column right of the stage (25/10/2026: words
    /// never stand on the bare map), as tall as its words need.
    fn nameplate_words(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let k = crate::text::style::current().scale.max(0.5);
        let note = share_note(inputs);
        let lines = |body: &str, chars: f32| {
            crate::menu::sjk::wrap(body, (chars / k).max(12.0) as usize).count() as f32
        };
        let height = WORDS_PAD
            + 26.0
            + 62.0 * k.max(1.0)
            + 8.0
            + lines(ABOUT, ABOUT_CHARS) * 18.0 * 1.42 * k
            + 4.0
            + lines(ORNAMENTS, NOTE_CHARS) * 16.0 * 1.42 * k
            + 26.0
            + STYLE_ROW
            + 48.0
            + lines(note, NOTE_CHARS) * 16.0 * 1.42 * k
            + WORDS_PAD;
        let top = WORDS_BOTTOM - height;
        let area = frame.rect(WORDS_X, top, WORDS_WIDTH, height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 16.0 * s,
            color: color::alpha(color::SPACE, 0.88),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 16.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::HOLO, 0.3),
        });
        let x = WORDS_X + WORDS_PAD;
        let width = WORDS_WIDTH - WORDS_PAD * 2.0;
        let mut y = top + WORDS_PAD;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Nameplate"),
            frame.rect(x, y, width, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 26.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Your nameplate"),
            frame.rect(x, y, width, 62.0 * k.max(1.0)),
            48.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 62.0 * k.max(1.0) + 8.0;
        y = super::toys::words(
            &mut self.ui,
            frame,
            ABOUT,
            [x, y, width],
            ABOUT_CHARS,
            18.0,
            color::TEXT,
        );
        y += 4.0;
        y = super::toys::words(
            &mut self.ui,
            frame,
            ORNAMENTS,
            [x, y, width],
            NOTE_CHARS,
            16.0,
            color::MUTED,
        );
        y += 26.0;
        self.style_row(frame, inputs, [x, y]);
        y += STYLE_ROW;
        self.share_switch(frame, inputs, [x, y, width], note);
    }

    /// Style: the nameplates' look, one segment per style, from `y`.
    fn style_row(&mut self, frame: &Frame, inputs: &Inputs<'_>, [x, y]: [f32; 2]) {
        use crate::hud::nameplate::Style;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Style"),
            frame.rect(x, y, 120.0, 30.0),
            22.0 * frame.s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let labels = Style::ALL.map(Style::label);
        let current = Style::ALL
            .iter()
            .position(|style| *style == inputs.nameplate_style)
            .unwrap_or(0);
        kit::segments(
            &mut self.ui,
            frame,
            x + SHARE_ROW,
            y + 15.0,
            &labels,
            current,
            false,
            STYLE_BASE,
        );
    }

    /// See and share nameplates: its name, the switch and what it does, from `y`. The row
    /// pulses, a gold ring round it and a glow behind the switch, so it is not missed.
    fn share_switch(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        [x, y, width]: [f32; 3],
        note: &str,
    ) {
        let s = frame.s;
        let seconds = self.epoch.elapsed().as_secs_f32();
        #[cfg(test)]
        let seconds = self.shot_seconds.unwrap_or(seconds);
        let pulse = share_pulse(seconds);
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(x - 14.0, y - 9.0, SHARE_ROW + 28.0, 48.0),
            radius: 12.0 * s,
            width: (1.5 + pulse) * s,
            color: color::alpha(color::GOLD, 0.25 + 0.65 * pulse),
        });
        glow(
            &mut self.ui,
            frame,
            [x + SHARE_ROW - 60.0, y + 15.0],
            40.0 + 10.0 * pulse,
            color::GOLD,
            0.15 + 0.35 * pulse,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("See & share nameplates"),
            frame.rect(x, y, SHARE_ROW - 100.0, 30.0),
            22.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let right = x + SHARE_ROW;
        let left = kit::switch(&mut self.ui, frame, right, y + 15.0, inputs.share, true);
        self.ui.hit_region(
            SHARE_TOKEN,
            frame.rect(left - 4.0, y - 4.0, right - left + 8.0, 38.0),
        );
        let _ = super::toys::words(
            &mut self.ui,
            frame,
            note,
            [x, y + 48.0, width],
            NOTE_CHARS,
            16.0,
            color::QUIET,
        );
    }
}

/// What the line under See and share nameplates says on or off now.
fn share_note(inputs: &Inputs<'_>) -> &'static str {
    match (inputs.enabled, inputs.share) {
        (false, _) => "Needs the SJK identity (Settings > Network) to reach other players.",
        (true, true) => {
            "You see other players' nameplates, and your health, shield and Force go to the SJK players of your server."
        }
        (true, false) => "Off: you see no nameplates, and nothing of yours is sent.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pulse starts dark, is fully lit halfway through and dark again at the end.
    #[test]
    fn the_share_row_pulses_from_dark_to_lit_and_back() {
        assert!(share_pulse(0.0) < 1e-6);
        assert!((share_pulse(SHARE_PULSE_SECONDS * 0.5) - 1.0).abs() < 1e-6);
        assert!(share_pulse(SHARE_PULSE_SECONDS) < 1e-5);
    }

    /// The empty places, the nameplate and its words fit over the keys with the model
    /// on the stage, in a preview and without it, at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_places_and_the_plate_fit_over_the_keys() {
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            let head = Frame::new(viewport).point(1_150.0, 420.0);
            for backstage in [
                Backstage::None,
                Backstage::World { head: Some(head) },
                Backstage::Preview { ready: false },
            ] {
                let inputs = super::super::tests::inputs(unlockables::Holdings::Known(&[]), "");
                let mut panel = Panel::new();
                panel.open(Tab::Nameplates, true, true, ReturnTarget::MainMenu);
                panel.set_backstage(backstage);
                panel.build(&inputs, &font, viewport);
                assert!(!panel.ui.overflowed(), "{viewport:?}");
                let frame = Frame::new(viewport);
                let keys = frame.point(0.0, super::super::view::KEYS_Y)[1];
                let switch = panel.ui.rect_for(SHARE_TOKEN).expect("the share switch");
                assert!(switch.bottom() <= keys, "{viewport:?}");
                for command in panel.ui.draw_list().commands() {
                    if let DrawCommand::Text { rect, .. } = command {
                        assert!(rect.bottom() <= keys || rect.y >= keys, "{rect:?}");
                        assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                    }
                }
            }
        }
    }
}
