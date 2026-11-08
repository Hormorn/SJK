//! Pointer interaction for the spawn list: a click on a row chooses it and runs it (so a
//! spawn takes one click), the Spawn button runs the chosen one, a tab switches the list,
//! the wheel scrolls it wherever the pointer is and the scrollbar drags.

use super::{Panel, PanelAction, Row, Step, TABS, WHEEL_ROWS};
use sjk_ui::{InputEvent, UiEventKind};

/// The way back (the top bar's Esc cap).
pub(super) const BACK_TOKEN: u16 = 900;
/// The filter's pill.
pub(super) const SEARCH_TOKEN: u16 = 901;
/// The Spawn (or Run) button and its key hint.
pub(super) const RUN_TOKEN: u16 = 904;
/// Wheel target over the detail pane.
pub(super) const PANE_WHEEL_TOKEN: u16 = 906;
/// Draggable thumb beside the rows.
pub(super) const SCROLLBAR_TOKEN: u16 = 910;
/// Tab `i` answers to `TAB_BASE + i`.
pub(super) const TAB_BASE: u16 = 500;
/// Row `slot` on screen answers to `ROW_BASE + slot`.
pub(super) const ROW_BASE: u16 = 1_000;
/// Most rows drawn at once.
pub(super) const ROW_LIMIT: usize = 100;

impl Panel {
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind == UiEventKind::Wheel {
            let direction = event.delta.map_or(0, |delta| -delta.y.signum() as isize);
            self.scroll_by(direction * WHEEL_ROWS);
            return PanelAction::None;
        }
        let Some(token) = event.token else {
            return PanelAction::None;
        };
        if event.kind == UiEventKind::Drag && token == SCROLLBAR_TOKEN {
            if let (Some(point), Some(track)) = (event.position, self.ui.rect_for(SCROLLBAR_TOKEN))
            {
                self.scroll_to_ratio((point.y - track.y) / track.height);
            }
            return PanelAction::None;
        }
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match token {
            BACK_TOKEN => PanelAction::Close,
            RUN_TOKEN => self.apply(Step::Run),
            ROW_BASE.. => {
                let row = self.first + usize::from(token - ROW_BASE);
                if matches!(self.visible.get(row), Some(Row::Item(_))) {
                    self.selected = row;
                    return self.apply(Step::Run);
                }
                PanelAction::None
            }
            TAB_BASE.. if usize::from(token - TAB_BASE) < TABS.len() => {
                self.set_tab(usize::from(token - TAB_BASE));
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_tokens_never_meet_the_fixed_ones() {
        for fixed in [
            BACK_TOKEN,
            SEARCH_TOKEN,
            RUN_TOKEN,
            PANE_WHEEL_TOKEN,
            SCROLLBAR_TOKEN,
        ] {
            assert!(fixed < ROW_BASE);
            assert!(fixed >= TAB_BASE + TABS.len() as u16);
        }
        assert!(ROW_BASE as usize + ROW_LIMIT <= u16::MAX as usize);
    }
}
