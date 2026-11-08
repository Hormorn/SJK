//! The spawn list, toggled with the `spawn_panel` console command: every NPC and vehicle
//! the game data defines and a few cheat commands, picked from a list instead of typed.
//!
//! On a map with cheats on (`devmap mp/ffa3`) spawning means typing `npc spawn <name>` or
//! `npc spawn vehicle <name>` and knowing the name. The panel lists the names
//! ([`catalogue`]) on three tabs, NPCs, Vehicles and Cheats, the retail game's first
//! under "Jedi Academy" and the community's under "Community". Typing filters the tab,
//! Enter or a click runs the chosen line as if typed in the console, and the panel stays
//! open for the next one. The server decides as for a typed line: without cheats it
//! refuses, as it says in the console.
//!
//! Like the `debug_panel` test list it lives in the console and is drawn in place of it,
//! in the SJK UI's look in every menu style (`spawn_panel_view.rs`, routed by
//! `console_sjk_pages.rs`), pointer input in `spawn_panel_pointer.rs`. Run with the
//! console closed (a bound key), it opens the console behind itself and closes it again
//! when the panel closes.

use crate::menu_widgets::MenuCanvas;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "spawn_panel_catalogue.rs"]
pub(crate) mod catalogue;
#[path = "spawn_panel_pointer.rs"]
mod pointer;
#[path = "spawn_panel_view.rs"]
mod view;

use catalogue::{CHEATS, Catalogue, Group};

/// Console command that toggles the panel; keys bound to it toggle it too.
pub(crate) const COMMAND: &str = "spawn_panel";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Toggle the spawn list: NPCs, vehicles and cheats to run with a click (needs cheats, devmap)";

/// The tabs; `Tab` and `Shift+Tab` cycle them.
const TABS: [&str; 3] = ["NPCs", "Vehicles", "Cheats"];
const NPCS: usize = 0;
const VEHICLES: usize = 1;
const CHEAT_TAB: usize = 2;
/// The headings of the retail names and the community's.
const RETAIL_HEADING: &str = "Jedi Academy";
const COMMUNITY_HEADING: &str = "Community";
/// Rows one wheel notch scrolls.
const WHEEL_ROWS: isize = 3;
/// Most characters the filter takes.
const QUERY_MAX: usize = 48;

/// What the console does after the panel handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    /// Close the panel.
    Close,
    /// Run this line as if typed in the console.
    Run(String),
}

/// A keyboard command, separated from winit events so it can be tested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    Move(isize),
    Page(isize),
    Run,
    Tab(isize),
    /// Escape: clear the filter, or close when it is empty.
    Back,
}

/// One row of the list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Row {
    /// A group's heading, never selected.
    Heading(&'static str),
    /// Item `index` of the tab: a name of its group, or a cheat.
    Item(usize),
}

/// What item `index` of tab `tab` shows and runs: its name (or the cheat's title), and
/// its line as a fixed start and the rest, so drawing it allocates nothing.
struct Item<'a> {
    title: &'a str,
    prefix: &'static str,
    rest: &'a str,
}

/// Item `index` of tab `tab` of `catalogue` (a free function, so the view can hold it
/// while it draws on the panel's canvas).
fn item(catalogue: &Catalogue, tab: usize, index: usize) -> Item<'_> {
    match tab {
        NPCS | VEHICLES => {
            let (group, prefix) = if tab == NPCS {
                (&catalogue.npcs, "npc spawn ")
            } else {
                (&catalogue.vehicles, "npc spawn vehicle ")
            };
            let name = group.names[index].as_str();
            Item {
                title: name,
                prefix,
                rest: name,
            }
        }
        _ => Item {
            title: CHEATS[index].title,
            prefix: "",
            rest: CHEATS[index].command,
        },
    }
}

pub(crate) struct Panel {
    open: bool,
    /// The panel opened the console, so closing the panel closes it too.
    owns_console: bool,
    catalogue: Catalogue,
    tab: usize,
    /// The filter typed, kept per tab.
    query: [String; 3],
    /// The tab's rows the filter keeps.
    visible: Vec<Row>,
    /// Index into `visible`, always an item while there is one.
    selected: usize,
    /// First row on screen.
    first: usize,
    /// Rows that fit, as laid out by the last `append_sjk`.
    rows: usize,
    /// The last line run, shown at the bottom left.
    status: String,
    ui: MenuCanvas,
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            catalogue: Catalogue::default(),
            tab: NPCS,
            query: Default::default(),
            visible: Vec::new(),
            selected: 0,
            first: 0,
            rows: 1,
            status: String::new(),
            ui: MenuCanvas::with_text_capacity(128),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the panel with the names of `catalogue`; `owns_console` when the console
    /// was closed before it. The tab and the filters stay as they were left.
    pub(super) fn open(&mut self, catalogue: Catalogue, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        if catalogue != self.catalogue {
            self.catalogue = catalogue;
            self.first = 0;
            self.selected = 0;
        }
        self.rebuild_visible();
    }

    /// Hide the panel; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Handle a key while the panel is open; `shift` and `control` are the console's.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        shift: bool,
        control: bool,
    ) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        let step = match key {
            KeyCode::Escape if !event.repeat => Step::Back,
            KeyCode::ArrowUp => Step::Move(-1),
            KeyCode::ArrowDown => Step::Move(1),
            KeyCode::PageUp => Step::Page(-1),
            KeyCode::PageDown => Step::Page(1),
            KeyCode::Home => Step::Move(isize::MIN),
            KeyCode::End => Step::Move(isize::MAX),
            KeyCode::Enter | KeyCode::NumpadEnter if !event.repeat => Step::Run,
            KeyCode::Tab if !event.repeat => Step::Tab(if shift { -1 } else { 1 }),
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => {
                return PanelAction::None;
            }
            _ => {
                let query = &mut self.query[self.tab];
                if crate::text_entry::edit(
                    query,
                    key,
                    event.text.as_deref(),
                    shift,
                    control,
                    QUERY_MAX,
                    true,
                ) {
                    self.filter_changed();
                }
                return PanelAction::None;
            }
        };
        self.apply(step)
    }

    fn apply(&mut self, step: Step) -> PanelAction {
        match step {
            Step::Back if self.query[self.tab].is_empty() => return PanelAction::Close,
            Step::Back => {
                self.query[self.tab].clear();
                crate::text_entry::deselect();
                self.filter_changed();
            }
            Step::Move(rows) => self.move_selection(rows),
            Step::Page(pages) => self.move_selection(pages * self.rows.max(1) as isize),
            Step::Run => return self.run_selected(),
            Step::Tab(direction) => {
                let count = TABS.len() as isize;
                self.set_tab((self.tab as isize + direction).rem_euclid(count) as usize);
            }
        }
        PanelAction::None
    }

    /// The tab's group of names (none for the Cheats tab).
    fn group(&self) -> Option<&Group> {
        match self.tab {
            NPCS => Some(&self.catalogue.npcs),
            VEHICLES => Some(&self.catalogue.vehicles),
            _ => None,
        }
    }

    /// How many items the tab has before filtering.
    fn item_count(&self) -> usize {
        self.group().map_or(CHEATS.len(), |group| group.names.len())
    }

    /// The line item `index` of the tab runs.
    fn line(&self, index: usize) -> String {
        let item = item(&self.catalogue, self.tab, index);
        format!("{}{}", item.prefix, item.rest)
    }

    /// Whether item `index` matches the tab's filter (any case, anywhere in the name,
    /// or in a cheat's title or command).
    fn matches(&self, index: usize) -> bool {
        let query = self.query[self.tab].trim();
        if query.is_empty() {
            return true;
        }
        let query = query.to_ascii_lowercase();
        let has = |text: &str| text.to_ascii_lowercase().contains(&query);
        match self.group() {
            Some(group) => has(&group.names[index]),
            None => has(CHEATS[index].title) || has(CHEATS[index].command),
        }
    }

    /// Index into `visible` of the selected item.
    fn selected_item(&self) -> Option<usize> {
        match self.visible.get(self.selected) {
            Some(Row::Item(index)) => Some(*index),
            _ => None,
        }
    }

    /// The tab's rows the filter keeps, a heading over each group shown; the
    /// selection kept on its item when it is still shown, else on the first item.
    fn rebuild_visible(&mut self) {
        let keep = self.selected_item();
        self.visible.clear();
        let retail = self.group().map(|group| group.retail);
        for index in 0..self.item_count() {
            if !self.matches(index) {
                continue;
            }
            if let Some(retail) = retail {
                let heading = if index < retail {
                    RETAIL_HEADING
                } else {
                    COMMUNITY_HEADING
                };
                if !self.visible.contains(&Row::Heading(heading)) {
                    self.visible.push(Row::Heading(heading));
                }
            }
            self.visible.push(Row::Item(index));
        }
        self.selected = keep
            .and_then(|keep| self.visible.iter().position(|&row| row == Row::Item(keep)))
            .or_else(|| {
                self.visible
                    .iter()
                    .position(|row| matches!(row, Row::Item(_)))
            })
            .unwrap_or(0);
        self.reveal_selection();
    }

    fn filter_changed(&mut self) {
        self.selected = 0;
        self.first = 0;
        self.rebuild_visible();
        // A new filter starts on its first match.
        self.selected = self
            .visible
            .iter()
            .position(|row| matches!(row, Row::Item(_)))
            .unwrap_or(0);
        self.reveal_selection();
    }

    fn set_tab(&mut self, tab: usize) {
        if tab == self.tab {
            return;
        }
        self.tab = tab % TABS.len();
        crate::text_entry::deselect();
        self.filter_changed();
    }

    /// The line the selected item runs, kept as the status.
    fn run_selected(&mut self) -> PanelAction {
        let Some(index) = self.selected_item() else {
            return PanelAction::None;
        };
        let line = self.line(index);
        self.status = format!("Ran: {line}");
        PanelAction::Run(line)
    }

    /// Move the selection by `rows` items (negative = up), clamped to the list;
    /// headings are passed over.
    fn move_selection(&mut self, rows: isize) {
        let items: Vec<usize> = (0..self.visible.len())
            .filter(|&at| matches!(self.visible[at], Row::Item(_)))
            .collect();
        let Some(current) = items.iter().position(|&at| at == self.selected) else {
            return;
        };
        let target = current.saturating_add_signed(rows).min(items.len() - 1);
        self.selected = items[target];
        self.reveal_selection();
    }

    /// Scroll so the selection is on screen, with the heading above a group's first
    /// item when it fits.
    fn reveal_selection(&mut self) {
        let rows = self.rows.max(1);
        let top = if self.selected > 0
            && matches!(self.visible.get(self.selected - 1), Some(Row::Heading(_)))
        {
            self.selected - 1
        } else {
            self.selected
        };
        self.first = self
            .first
            .min(top)
            .max((self.selected + 1).saturating_sub(rows));
    }

    /// Scroll by `rows` (negative = up) without moving the selection.
    fn scroll_by(&mut self, rows: isize) {
        let max_first = self.visible.len().saturating_sub(self.rows);
        self.first = self.first.saturating_add_signed(rows).min(max_first);
    }

    /// Scroll so the list shows `ratio` (0 = top, 1 = bottom) of the rows.
    fn scroll_to_ratio(&mut self, ratio: f32) {
        let max_first = self.visible.len().saturating_sub(self.rows);
        self.first = (ratio.clamp(0.0, 1.0) * max_first as f32).round() as usize;
    }

    /// Open on a few names with the second spawned, for a world shot.
    #[cfg(test)]
    pub(crate) fn open_for_shot(&mut self) {
        let names = |names: &[&str], retail| Group {
            names: names.iter().map(|name| name.to_string()).collect(),
            retail,
        };
        let catalogue = Catalogue {
            npcs: names(
                &[
                    "alora",
                    "boba_fett",
                    "chewie",
                    "cultist",
                    "Desann",
                    "jedi_hm1",
                    "reborn",
                    "stormtrooper",
                    "tavion",
                    "my_mandalorian",
                ],
                9,
            ),
            vehicles: names(&["Swoop", "Tauntaun", "X-Wing"], 3),
        };
        self.open(catalogue, false);
        self.apply(Step::Move(1));
        let _ = self.apply(Step::Run);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue() -> Catalogue {
        let names = |names: &[&str], retail| Group {
            names: names.iter().map(|name| name.to_string()).collect(),
            retail,
        };
        Catalogue {
            npcs: names(&["alora", "reborn", "stormtrooper", "my_trooper"], 3),
            vehicles: names(&["Swoop", "tie-fighter"], 2),
        }
    }

    fn panel() -> Panel {
        let mut panel = Panel::new();
        panel.open(catalogue(), true);
        panel
    }

    fn typed(panel: &mut Panel, text: &str) {
        panel.query[panel.tab].push_str(text);
        panel.filter_changed();
    }

    #[test]
    fn the_npcs_come_under_their_headings_and_the_first_is_chosen() {
        let panel = panel();
        assert_eq!(
            panel.visible,
            [
                Row::Heading(RETAIL_HEADING),
                Row::Item(0),
                Row::Item(1),
                Row::Item(2),
                Row::Heading(COMMUNITY_HEADING),
                Row::Item(3),
            ]
        );
        assert_eq!(panel.selected, 1);
    }

    #[test]
    fn enter_spawns_the_chosen_npc_and_moving_skips_headings() {
        let mut panel = panel();
        assert_eq!(
            panel.apply(Step::Run),
            PanelAction::Run("npc spawn alora".into())
        );
        panel.apply(Step::Move(3));
        assert_eq!(panel.visible[panel.selected], Row::Item(3));
        assert_eq!(
            panel.apply(Step::Run),
            PanelAction::Run("npc spawn my_trooper".into())
        );
        assert_eq!(panel.status, "Ran: npc spawn my_trooper");
        panel.apply(Step::Move(isize::MIN));
        assert_eq!(panel.selected, 1);
    }

    #[test]
    fn vehicles_spawn_as_vehicles_and_cheats_run_their_command() {
        let mut panel = panel();
        panel.apply(Step::Tab(1));
        assert_eq!(
            panel.apply(Step::Run),
            PanelAction::Run("npc spawn vehicle Swoop".into())
        );
        panel.apply(Step::Tab(1));
        assert!(
            !panel
                .visible
                .iter()
                .any(|row| matches!(row, Row::Heading(_)))
        );
        assert_eq!(panel.apply(Step::Run), PanelAction::Run("noclip".into()));
        panel.apply(Step::Tab(1));
        assert_eq!(panel.tab, NPCS);
    }

    #[test]
    fn typing_filters_the_tab_and_escape_clears_before_closing() {
        let mut panel = panel();
        typed(&mut panel, "TROOP");
        assert_eq!(
            panel.visible,
            [
                Row::Heading(RETAIL_HEADING),
                Row::Item(2),
                Row::Heading(COMMUNITY_HEADING),
                Row::Item(3),
            ]
        );
        assert_eq!(panel.selected, 1);
        panel.apply(Step::Tab(2));
        typed(&mut panel, "kill");
        assert_eq!(panel.visible.len(), 2);
        assert_eq!(
            panel.apply(Step::Run),
            PanelAction::Run("npc kill all".into())
        );
        assert_eq!(panel.apply(Step::Back), PanelAction::None);
        assert_eq!(panel.visible.len(), CHEATS.len());
        assert_eq!(panel.apply(Step::Back), PanelAction::Close);
        // The NPCs' filter waited on its tab.
        panel.apply(Step::Tab(1));
        assert_eq!(panel.query[NPCS], "TROOP");
    }

    #[test]
    fn nothing_matching_runs_nothing() {
        let mut panel = panel();
        typed(&mut panel, "zzz");
        assert!(panel.visible.is_empty());
        assert_eq!(panel.apply(Step::Run), PanelAction::None);
        panel.apply(Step::Move(1));
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn scrolling_stays_in_range_and_shows_a_groups_heading() {
        let mut panel = panel();
        panel.rows = 2;
        panel.apply(Step::Move(isize::MAX));
        assert_eq!(panel.selected, 5);
        assert_eq!(panel.first, 4);
        panel.apply(Step::Move(isize::MIN));
        assert_eq!(panel.first, 0);
        panel.scroll_to_ratio(1.0);
        assert_eq!(panel.first, 4);
        panel.scroll_by(isize::MIN / 2);
        assert_eq!(panel.first, 0);
        assert!(panel.close());
        assert!(!panel.close());
    }
}
