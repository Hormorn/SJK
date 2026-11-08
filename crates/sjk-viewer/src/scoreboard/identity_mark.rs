//! The mark after a player's name on the scoreboard when the SJK hub knows them
//! (`player_identity.rs`): SJK's emblem in soft white (or, for a player who claimed
//! from another client with its own icon, that icon in its own colours,
//! `chat::client_icons`), then up to six ribbon bars for the medals the SJK team gave
//! them (`medals.rs`), plain coloured rectangles sized after the emblem. Their tick
//! (gold, a group's, staff's, a creator's) or, without one, their trust mark sits small
//! on the emblem's foot (`ui_renderer::mark_texture`; the emblem was gold for a verified
//! player until 27/10/2026). The marks take room from the name, never from the
//! columns: where the name would keep less than half its room, fewer bars show.

use crate::medals::{Medal, Medals};
use crate::player_identity::Tag;
use sjk_ui::{Color, DrawCommand, DrawList, Rect};

const REGISTERED: Color = Color::new(0.92, 0.94, 1.0, 0.9);
/// Most ribbon bars a row shows: every medal of today's catalogue (four), with room
/// for two more.
const RIBBONS_MAX: usize = 6;
/// A bar's width, height and the gap before it, as fractions of the emblem's side.
const RIBBON_WIDTH: f32 = 1.05;
const RIBBON_HEIGHT: f32 = 0.42;
const RIBBON_GAP: f32 = 0.2;
/// The tick's or trust mark's side on the emblem's foot, as a share of the emblem's.
const TRUST_SIDE: f32 = 0.55;
/// The share of the name's room the marks may take.
const MARKS_SHARE: f32 = 0.5;

/// The emblem's texture: SJK's, or the icon of the client the player claimed from
/// (`chat::client_icons`).
fn texture(tag: Tag) -> sjk_ui::TextureId {
    crate::chat::client_icons::texture(tag.client)
}

/// The emblem's tint. Another client's icon keeps its own colours.
pub(super) fn tint(tag: Tag) -> Color {
    if texture(tag) != crate::ui_renderer::LOGO_TEXTURE {
        Color::new(1.0, 1.0, 1.0, 1.0)
    } else {
        REGISTERED
    }
}

/// A ribbon bar's width and height, and the gap before it, for an emblem `side` across.
pub(super) fn ribbon_size(side: f32) -> (f32, f32, f32) {
    (side * RIBBON_WIDTH, side * RIBBON_HEIGHT, side * RIBBON_GAP)
}

/// How many of `medals` show as bars after an emblem `side` across when the marks may
/// take `room` pixels in all (the emblem's included): at most six, fewer where they
/// do not fit.
pub(super) fn ribbon_count(medals: Medals, side: f32, room: f32) -> usize {
    let (width, _, gap) = ribbon_size(side);
    let mut count = medals.len().min(RIBBONS_MAX);
    while count > 0 && side + count as f32 * (gap + width) > room {
        count -= 1;
    }
    count
}

/// The width the emblem and `count` bars take.
pub(super) fn marks_width(side: f32, count: usize) -> f32 {
    let (width, _, gap) = ribbon_size(side);
    side + count as f32 * (gap + width)
}

/// Ribbon bar `medal` over `rect`: its base colour, then its stripes.
pub(crate) fn ribbon(medal: Medal, rect: Rect, mut push: impl FnMut(DrawCommand)) {
    let ribbon = medal.ribbon();
    push(DrawCommand::SolidRect {
        rect,
        color: ribbon.base,
    });
    for stripe in ribbon.stripes {
        push(DrawCommand::SolidRect {
            rect: Rect::new(
                rect.x + rect.width * stripe.at,
                rect.y,
                rect.width * stripe.width,
                rect.height,
            ),
            color: stripe.color,
        });
    }
}

/// The emblem and the first `count` of `medals` as bars, from `x` rightwards, centred
/// on `middle`.
pub(super) fn draw_marks(
    x: f32,
    middle: f32,
    side: f32,
    tag: Tag,
    count: usize,
    mut push: impl FnMut(DrawCommand),
) {
    push(DrawCommand::TexturedQuad {
        rect: Rect::new(x, middle - side * 0.5, side, side),
        texture: texture(tag),
        color: tint(tag),
    });
    if let Some(mark) = crate::ui_renderer::mark_texture(tag.verified, tag.tick, tag.trust) {
        let small = side * TRUST_SIDE;
        push(DrawCommand::TexturedQuad {
            rect: Rect::new(
                x + side - small * 0.8,
                middle + side * 0.5 - small * 0.9,
                small,
                small,
            ),
            texture: mark,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
    }
    let (width, height, gap) = ribbon_size(side);
    let mut at = x + side + gap;
    for (medal, _) in tag.medals.iter().take(count) {
        let bar = Rect::new(at, middle - height * 0.5, width, height);
        ribbon(medal, bar, &mut push);
        // Its rank's device on it, as a real ribbon's (`medals/rank.rs`).
        crate::medals::rank::ribbon_device(tag.medals.rank(medal), bar, &mut push);
        at += width + gap;
    }
}

/// The emblem and the bars that fit in the right end of `name` (a row's name
/// rectangle), the emblem `side` wide, centred vertically; returns `name` without the
/// room they take.
pub(super) fn push(list: &mut DrawList, name: Rect, side: f32, tag: Tag) -> Rect {
    let count = ribbon_count(tag.medals, side, name.width * MARKS_SHARE);
    let width = marks_width(side, count);
    draw_marks(
        name.x + name.width - width,
        name.y + name.height * 0.5,
        side,
        tag,
        count,
        |command| {
            let _ = list.push(command);
        },
    );
    Rect::new(
        name.x,
        name.y,
        (name.width - width - side * 0.25).max(0.0),
        name.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(verified: bool, medals: &[&str]) -> Tag {
        let list: Vec<sjk_identity::Medal> = medals
            .iter()
            .map(|id| sjk_identity::Medal {
                id: (*id).to_owned(),
                count: 1,
                awarded: 0,
                note: String::new(),
                rank: 1,
            })
            .collect();
        Tag {
            verified,
            tick: sjk_identity::Tick::None,
            trust: None,
            emoji: Default::default(),
            medals: Medals::from_wire(&list),
            client: None,
        }
    }

    #[test]
    fn a_verified_players_tick_sits_on_a_white_emblem() {
        assert_eq!(tint(tag(true, &[])), tint(tag(false, &[])));
        let sith = Tag {
            tick: sjk_identity::Tick::Sith,
            ..tag(true, &[])
        };
        let mut list = DrawList::new(16);
        push(&mut list, Rect::new(0.0, 0.0, 200.0, 30.0), 20.0, sith);
        let textures: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::TexturedQuad { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        assert_eq!(
            textures,
            [
                crate::ui_renderer::LOGO_TEXTURE,
                crate::ui_renderer::tick_texture(sjk_identity::Tick::Sith).unwrap()
            ]
        );
    }

    #[test]
    fn another_clients_players_wear_its_icon_untinted() {
        let ejk = Tag {
            client: crate::chat::client_icons::find("ejk"),
            ..tag(true, &[])
        };
        let mut list = DrawList::new(16);
        push(&mut list, Rect::new(0.0, 0.0, 200.0, 30.0), 20.0, ejk);
        let Some(DrawCommand::TexturedQuad { texture, color, .. }) = list.commands().first() else {
            panic!("a textured quad");
        };
        assert_eq!(*texture, crate::ui_renderer::EJK_TEXTURE);
        assert_eq!(*color, Color::new(1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn the_emblem_sits_at_the_right_end_and_the_name_leaves_room() {
        let name = Rect::new(10.0, 20.0, 200.0, 30.0);
        let mut list = DrawList::new(16);
        let text = push(&mut list, name, 20.0, tag(false, &[]));
        let Some(DrawCommand::TexturedQuad { rect, .. }) = list.commands().first() else {
            panic!("a textured quad");
        };
        assert_eq!(
            (rect.x + rect.width, rect.y + rect.height * 0.5),
            (210.0, 35.0)
        );
        assert!(
            text.x + text.width < rect.x,
            "the name ends before the emblem"
        );
    }

    #[test]
    fn ribbons_follow_the_emblem_and_end_at_the_names_right_edge() {
        let name = Rect::new(10.0, 20.0, 300.0, 30.0);
        let mut list = DrawList::new(64);
        let text = push(
            &mut list,
            name,
            20.0,
            // Unverified, so no tick sits on the emblem's foot among the rectangles.
            tag(false, &["early_tester", "bug_hunter"]),
        );
        let rects: Vec<Rect> =
            list.commands()
                .iter()
                .map(|command| match command {
                    DrawCommand::TexturedQuad { rect, .. }
                    | DrawCommand::SolidRect { rect, .. } => *rect,
                    other => panic!("unexpected {other:?}"),
                })
                .collect();
        let emblem = rects[0];
        // The tester's base and three stripes, then the bug hunter's base and edges.
        assert_eq!(rects.len(), 1 + 4 + 3);
        let first = rects[1];
        let last = rects[5];
        assert!(first.x > emblem.x + emblem.width, "after the emblem");
        assert!(last.x > first.x + first.width, "one after the other");
        assert!(
            (last.x + last.width - 310.0).abs() < 1e-3,
            "ends at the edge"
        );
        assert!(text.x + text.width < emblem.x);
    }

    #[test]
    fn every_medal_shows_a_bar_and_fewer_when_the_name_is_short() {
        let all = tag(
            false,
            &[
                "early_tester",
                "early_contributor",
                "bug_hunter",
                "supporter",
            ],
        );
        assert_eq!(ribbon_count(all.medals, 20.0, 1_000.0), 4);
        assert_eq!(ribbon_count(all.medals, 20.0, marks_width(20.0, 4)), 4);
        assert_eq!(
            ribbon_count(all.medals, 20.0, marks_width(20.0, 4) - 1.0),
            3
        );
        assert_eq!(ribbon_count(all.medals, 20.0, marks_width(20.0, 2)), 2);
        assert_eq!(ribbon_count(all.medals, 20.0, 25.0), 0);
        assert_eq!(ribbon_count(tag(false, &[]).medals, 20.0, 1_000.0), 0);
        let mut list = DrawList::new(64);
        let narrow = Rect::new(0.0, 0.0, 80.0, 20.0);
        let text = push(&mut list, narrow, 16.0, all);
        assert!(text.width >= 0.0);
        assert!(
            list.commands().len() < 1 + 4 + 2 + 3,
            "not every bar fits a short name"
        );
    }
}
