//! Shared vote card geometry for the HUD and headless evidence.

use sjk_ui::Rect;

/// One vote's card, sized in hero design pixels (1080 lines).
pub(crate) struct VoteLayout {
    /// The dark plate behind the card.
    pub(crate) plate: Rect,
    /// Time left, as a thin bar along the top.
    pub(crate) time: Rect,
    /// Vote kind at the left, seconds left at the right.
    pub(crate) label: Rect,
    /// The proposal, centred.
    pub(crate) proposal: Rect,
    /// YES / NO share of the votes cast.
    pub(crate) split: Rect,
    /// YES count at the left, NO count at the right.
    pub(crate) counts: Rect,
    /// The keys for YES at the left and NO at the right.
    pub(crate) keys: Rect,
}

impl VoteLayout {
    /// Card width in design pixels.
    pub(crate) const WIDTH: f32 = 560.0;
    /// Card height in design pixels.
    pub(crate) const HEIGHT: f32 = 186.0;

    /// Lay out a card whose top left is `rect`'s, `rect.width` wide.
    pub(crate) fn new(rect: Rect, scale: f32) -> Self {
        let pad = 18.0 * scale;
        let x = rect.x + pad;
        let width = rect.width - 2.0 * pad;
        let row = |top: f32, height: f32| Rect::new(x, rect.y + top * scale, width, height * scale);
        Self {
            plate: Rect::new(rect.x, rect.y, rect.width, Self::HEIGHT * scale),
            time: row(12.0, 4.0),
            label: row(22.0, 28.0),
            proposal: row(54.0, 40.0),
            split: row(104.0, 8.0),
            counts: row(118.0, 30.0),
            keys: row(150.0, 22.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_stay_inside_the_plate_in_order() {
        let layout = VoteLayout::new(Rect::new(100.0, 40.0, VoteLayout::WIDTH, 0.0), 1.0);
        let rows = [
            layout.time,
            layout.label,
            layout.proposal,
            layout.split,
            layout.counts,
            layout.keys,
        ];
        let bottom = layout.plate.y + layout.plate.height;
        for pair in rows.windows(2) {
            assert!(pair[0].y + pair[0].height <= pair[1].y);
        }
        for row in rows {
            assert!(row.x >= layout.plate.x && row.y >= layout.plate.y);
            assert!(row.x + row.width <= layout.plate.x + layout.plate.width);
            assert!(row.y + row.height <= bottom);
        }
    }
}
