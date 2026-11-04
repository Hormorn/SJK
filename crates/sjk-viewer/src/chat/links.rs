//! Links in the chat box (`docs/client.md`, "Copying a link from the chat"; AngeloPapaya's
//! idea, 03/11/2026). While the composer is open (the pointer is free), each web link a
//! shown line holds (`http://`, `https://` or `www.`) is laid out where its characters
//! are drawn, a row at a time; the pointer on one lights it as a name under the pointer
//! is lit, and Ctrl+C (or Ctrl+Insert) copies that link, without its colour codes,
//! instead of the draft ([`super::ChatOverlay::copy_choice`]). "Link copied" shows
//! under the field for [`COPIED_MS`].
//!
//! The places are kept in a fixed array ([`SPOTS`]) rebuilt by each frame that draws
//! the feed with the composer open: no allocation, nothing done while the chat is shut.

use crate::text::{TextFace, UiFont, visible_text_width_style};
use sjk_ui::{Rect, Vec2};
use std::ops::Range;

/// Link pieces (a link on one row) followed in one frame; more are left out.
pub(super) const SPOTS: usize = 32;
/// How long "Link copied" shows under the field.
pub(super) const COPIED_MS: u64 = 1_500;
/// The note a copied link leaves under the field.
pub(super) const COPIED_NOTE: &str = "Link copied";

/// What starts a link, compared without case.
const STARTS: [&str; 3] = ["https://", "http://", "www."];

/// The byte ranges of the web links in `text`, in order: a word (between white space)
/// that starts with `http://`, `https://` or `www.`, after any colour codes and opening
/// brackets or quotes, up to the word's end or an emoji mark, its closing punctuation
/// and trailing colour codes left out.
pub(super) fn find(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    words(text).filter_map(|word| link_in(text, word))
}

/// The byte ranges of `text`'s words, between white space.
fn words(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut at = 0;
    std::iter::from_fn(move || {
        let rest = &text[at..];
        let start = at + rest.find(|c: char| !c.is_whitespace())?;
        let end = text[start..]
            .find(char::is_whitespace)
            .map_or(text.len(), |len| start + len);
        at = end;
        Some(start..end)
    })
}

/// The link word `word` of `text` holds, if it is one.
fn link_in(text: &str, word: Range<usize>) -> Option<Range<usize>> {
    let mut start = word.start;
    loop {
        let rest = &text[start..word.end];
        if colour_code_at(rest) {
            start += 2;
        } else if rest.starts_with(['(', '<', '[', '"', '\'']) {
            start += 1;
        } else {
            break;
        }
    }
    let rest = &text[start..word.end];
    let scheme = STARTS.iter().find(|scheme| {
        rest.get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
    })?;
    // An emoji mark ends the link: marks are never part of a web address.
    let mut end = start
        + rest
            .find(|c| super::emoji::mark_index(c).is_some())
            .unwrap_or(rest.len());
    loop {
        let link = &text[start..end];
        let bytes = link.as_bytes();
        if bytes.len() >= 2
            && bytes[bytes.len() - 2] == b'^'
            && bytes[bytes.len() - 1].is_ascii_digit()
        {
            end -= 2;
        } else if link.ends_with(['.', ',', ';', ':', '!', '?', '"', '\'', '>', ']', '}'])
            || (link.ends_with(')') && link.matches(')').count() > link.matches('(').count())
        {
            end -= 1;
        } else {
            break;
        }
    }
    (plain_len(&text[start..end]) > scheme.len()).then_some(start..end)
}

/// The length of `text` without its colour codes, counted without allocating.
fn plain_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut len = 0;
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'^' && bytes.get(at + 1).is_some_and(u8::is_ascii_digit) {
            at += 2;
        } else {
            len += 1;
            at += 1;
        }
    }
    len
}

/// Whether `text` starts with a colour code, `^` and a digit.
fn colour_code_at(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 2 && bytes[0] == b'^' && bytes[1].is_ascii_digit()
}

/// The width `text` takes in a body row of `size` with `spacing` after each character,
/// as [`super::view`]'s `body_row` draws it: its runs of text, and an emoji's room for
/// each mark between them. Colour codes take none.
pub(super) fn drawn_width(font: &UiFont, text: &str, size: f32, spacing: f32) -> f32 {
    let scale = size / font.height.max(1.0);
    let mut width = 0.0;
    let mut rest = text;
    loop {
        let at = rest
            .find(|c| super::emoji::mark_index(c).is_some())
            .unwrap_or(rest.len());
        width += visible_text_width_style(font, &rest[..at], scale, TextFace::Regular, spacing);
        let Some(mark) = rest[at..].chars().next() else {
            return width;
        };
        width += super::emoji::advance(size);
        rest = &rest[at + mark.len_utf8()..];
    }
}

/// Where a line's body rows are drawn this frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct RowPlace {
    /// The first row's top-left.
    pub(super) origin: [f32; 2],
    /// Room the sender's name and marks take before the first row's text.
    pub(super) indent: f32,
    pub(super) size: f32,
    pub(super) spacing: f32,
    /// Row pitch.
    pub(super) row: f32,
}

/// One piece of a link: the line it is in, the link's byte range in the line's body
/// and where this piece is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Spot {
    /// Index of the line in the overlay's history.
    pub(super) line: usize,
    /// When that line arrived, to tell it from another one taking its index.
    pub(super) received_ms: u64,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) rect: Rect,
}

impl Spot {
    /// Whether `other` is a piece of the same link.
    fn same_link(&self, other: &Spot) -> bool {
        (self.line, self.received_ms, self.start) == (other.line, other.received_ms, other.start)
    }
}

/// The link pieces of the frame's feed.
pub(super) struct Spots {
    spots: [Spot; SPOTS],
    len: usize,
}

impl Default for Spots {
    fn default() -> Self {
        Self {
            spots: [Spot::default(); SPOTS],
            len: 0,
        }
    }
}

impl Spots {
    pub(super) fn clear(&mut self) {
        self.len = 0;
    }

    /// Lay out the links of line `line` (arrived at `received_ms`), whose `body` is
    /// drawn in `rows` at `place`.
    pub(super) fn add_line(
        &mut self,
        font: &UiFont,
        (line, received_ms): (usize, u64),
        body: &str,
        rows: &[Range<usize>],
        place: RowPlace,
    ) {
        for link in find(body) {
            for (index, row) in rows.iter().enumerate() {
                let start = link.start.max(row.start);
                let end = link.end.min(row.end);
                if start >= end {
                    continue;
                }
                if self.len == SPOTS {
                    return;
                }
                let left = place.origin[0] + if index == 0 { place.indent } else { 0.0 };
                let x =
                    left + drawn_width(font, &body[row.start..start], place.size, place.spacing);
                let width = drawn_width(font, &body[start..end], place.size, place.spacing);
                self.spots[self.len] = Spot {
                    line,
                    received_ms,
                    start: link.start,
                    end: link.end,
                    rect: Rect::new(
                        x,
                        place.origin[1] + index as f32 * place.row,
                        width,
                        place.row,
                    ),
                };
                self.len += 1;
            }
        }
    }

    /// The link piece under `point`.
    pub(super) fn at(&self, point: Vec2) -> Option<Spot> {
        self.spots[..self.len]
            .iter()
            .find(|spot| spot.rect.contains(point))
            .copied()
    }

    /// The first piece laid out, for tests.
    #[cfg(test)]
    pub(super) fn first(&self) -> Option<Spot> {
        self.spots[..self.len].first().copied()
    }

    /// Every piece of `spot`'s link, on each row it is drawn on.
    pub(super) fn pieces(&self, spot: Spot) -> impl Iterator<Item = Rect> + '_ {
        self.spots[..self.len]
            .iter()
            .filter(move |other| other.same_link(&spot))
            .map(|other| other.rect)
    }
}

impl super::ChatOverlay {
    /// The link piece under the pointer, unless the sender card lies over it there.
    pub(super) fn link_under_pointer(&self) -> Option<Spot> {
        let at = self.pointer_at?;
        let on_card = self.card.is_some()
            && self
                .ui
                .rect_for(super::card::CARD)
                .is_some_and(|rect| rect.contains(at));
        (!on_card).then(|| self.links.at(at)).flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links(text: &str) -> Vec<&str> {
        find(text).map(|range| &text[range]).collect()
    }

    #[test]
    fn finds_web_links_between_words() {
        assert_eq!(
            links("see https://example.com/a?b=1 and http://x.org now"),
            ["https://example.com/a?b=1", "http://x.org"]
        );
        assert_eq!(links("WWW.Example.com"), ["WWW.Example.com"]);
        assert_eq!(links("HTTPS://SHOUT.COM"), ["HTTPS://SHOUT.COM"]);
        assert!(links("no links here, not even http:// or www.").is_empty());
        assert!(links("ftp://elsewhere.net mailto:me@x.com https:/half").is_empty());
        assert!(links("").is_empty());
        // Letters of any length round and in a link.
        assert_eq!(links("é https://ü.de/ç é"), ["https://ü.de/ç"]);
        assert_eq!(links("https://x.de/é^1é"), ["https://x.de/é^1é"]);
    }

    #[test]
    fn leaves_out_the_punctuation_round_a_link() {
        assert_eq!(links("(https://x.com/page)."), ["https://x.com/page"]);
        assert_eq!(links("\"www.x.com\","), ["www.x.com"]);
        assert_eq!(links("go to https://x.com!"), ["https://x.com"]);
        // A bracket the link opened is its own.
        assert_eq!(
            links("https://en.wikipedia.org/wiki/Saber_(film))"),
            ["https://en.wikipedia.org/wiki/Saber_(film)"]
        );
    }

    #[test]
    fn leaves_out_the_colour_codes_round_a_link_but_keeps_those_inside() {
        let text = "^2look ^5https://x.com/^3a^7 ^7done";
        assert_eq!(links(text), ["https://x.com/^3a"]);
        // What a copy gives: the link without its codes.
        let range = find(text).next().unwrap();
        assert_eq!(
            crate::text::Plain(&text[range]).to_string(),
            "https://x.com/a"
        );
    }

    #[test]
    fn an_emoji_mark_ends_a_link() {
        let mark = char::from_u32(0xE000).unwrap();
        let text = format!("https://x.com{mark} hi");
        assert_eq!(links(&text), ["https://x.com"]);
    }

    /// Test glyphs advance 8 units at height 12 ([`crate::text::test_font`]).
    fn place(indent: f32) -> RowPlace {
        RowPlace {
            origin: [100.0, 200.0],
            indent,
            size: 12.0,
            spacing: 0.0,
            row: 20.0,
        }
    }

    #[test]
    fn a_link_is_where_its_characters_are_drawn() {
        let font = crate::text::test_font();
        let body = "^1hey www.a.com";
        let mut spots = Spots::default();
        spots.add_line(
            &font,
            (3, 7),
            body,
            std::slice::from_ref(&(0..body.len())),
            place(40.0),
        );
        // After the indent and "hey " (four characters, the code taking no room).
        let spot = spots
            .at(Vec2::new(100.0 + 40.0 + 32.0 + 1.0, 205.0))
            .unwrap();
        assert_eq!((spot.line, spot.received_ms), (3, 7));
        assert_eq!(&body[spot.start..spot.end], "www.a.com");
        assert_eq!(spot.rect, Rect::new(172.0, 200.0, 72.0, 20.0));
        // Off the link: on "hey", past its end, or on the row under it.
        assert_eq!(spots.at(Vec2::new(150.0, 205.0)), None);
        assert_eq!(spots.at(Vec2::new(245.0, 205.0)), None);
        assert_eq!(spots.at(Vec2::new(180.0, 221.0)), None);
    }

    #[test]
    fn a_link_broken_over_two_rows_is_one_link_on_both() {
        let font = crate::text::test_font();
        let body = "go https://abcdef.gh tail";
        // The wrap broke the link after "https://abc".
        let rows = [0..14, 14..body.len()];
        let mut spots = Spots::default();
        spots.add_line(&font, (0, 0), body, &rows, place(16.0));
        let first = spots
            .at(Vec2::new(100.0 + 16.0 + 24.0 + 4.0, 210.0))
            .unwrap();
        let second = spots.at(Vec2::new(104.0, 230.0)).unwrap();
        assert_eq!((first.start, first.end), (second.start, second.end));
        assert_eq!(&body[first.start..first.end], "https://abcdef.gh");
        // The second row starts at the line's left, not after the indent.
        assert_eq!(second.rect, Rect::new(100.0, 220.0, 6.0 * 8.0, 20.0));
        let pieces: Vec<_> = spots.pieces(second).collect();
        assert_eq!(pieces, [first.rect, second.rect]);
    }

    #[test]
    fn an_emoji_before_a_link_moves_it_by_its_room() {
        let font = crate::text::test_font();
        let mark = char::from_u32(0xE000).unwrap();
        let body = format!("a{mark} https://x.yz");
        let mut spots = Spots::default();
        spots.add_line(
            &font,
            (0, 0),
            &body,
            std::slice::from_ref(&(0..body.len())),
            place(0.0),
        );
        let x = 100.0 + 8.0 + super::super::emoji::advance(12.0) + 8.0;
        let spot = spots.at(Vec2::new(x + 1.0, 201.0)).unwrap();
        assert_eq!(spot.rect.x, x);
    }

    #[test]
    fn no_more_pieces_than_the_frame_keeps() {
        let font = crate::text::test_font();
        let body = "www.a.com ".repeat(SPOTS + 5);
        let mut spots = Spots::default();
        spots.add_line(
            &font,
            (0, 0),
            &body,
            std::slice::from_ref(&(0..body.len())),
            place(0.0),
        );
        assert_eq!(spots.len, SPOTS);
        spots.clear();
        assert_eq!(spots.at(Vec2::new(101.0, 201.0)), None);
    }
}
