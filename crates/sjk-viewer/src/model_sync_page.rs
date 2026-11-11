//! The Model sync page (`docs/model-sync.md`): the models this game has and those
//! shared on the Sith cloud, with where each stands and its Share, Update and Download
//! buttons, a search over the names, and what model sync said last.
//!
//! Opened by the `modelsync` command, Settings > Mods' "Model sync" row and the
//! Profile page's Share my model. It lives in the console and is drawn in place of it
//! in the SJK UI's look whatever the menu style (`model_sync_page_view.rs`), like the
//! content cards. The work is [`crate::model_sync`]'s: the page reads its rows and
//! hands the console a command to queue (`sharemodel <name>`, `getmodel <name>`);
//! `console_model_sync.rs` is its console side.

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use crate::model_sync::{PageRow, RowAction};
use sjk_ui::{InputEvent, UiEventKind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "model_sync_page_view.rs"]
mod view;

/// Rows the list shows at a time.
const ROWS: usize = 10;
/// Longest search the field takes.
const SEARCH_MAX: usize = 40;
/// Buttons a row has at most (an upload and a download).
const ROW_BUTTONS: usize = 2;
/// Pointer tokens: a row's buttons are `slot * ROW_BUTTONS + button`, the rows
/// `ROW_TOKEN + slot`.
const ROW_TOKEN: u16 = 100;
const SEARCH_TOKEN: u16 = 500;
const REFRESH_TOKEN: u16 = 501;
const WHEEL_TOKEN: u16 = 910;
/// How often the rows are read again while nothing said they changed (a model
/// installed meanwhile, the catalogue read late).
const REREAD: Duration = Duration::from_secs(3);

/// What has the keyboard: the search, a button of the chosen row, Refresh or Close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Search,
    Row(usize),
    Refresh,
    Close,
}

/// What the console does after the page handled an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageAction {
    None,
    Close,
    /// Read the Sith cloud's list again.
    Refresh,
    /// Queue `<action's command> <model>`.
    Run(RowAction, String),
}

pub(crate) struct Page {
    open: bool,
    /// The page opened the console, so closing the page closes it too.
    owns_console: bool,
    /// Whether the page about to open takes the console over from the one it replaces.
    handed: Option<bool>,
    game_data: PathBuf,
    rows: Vec<PageRow>,
    /// Each row's status words and lower-case name, made when the rows are read.
    statuses: Vec<String>,
    lowered: Vec<String>,
    query: String,
    /// The rows the search keeps, as indexes into `rows`.
    shown: Vec<usize>,
    /// The chosen row of `shown`, and the first one on screen.
    selected: usize,
    first: usize,
    focus: Focus,
    /// The player may share (a JoF or Sith tick, or staff).
    can_share: bool,
    has_index: bool,
    busy: bool,
    auto_on: bool,
    /// What model sync said last, newest last.
    lines: Vec<String>,
    /// [`crate::model_sync::changes`] when the rows were read, and when.
    seen: Option<u64>,
    read_at: Option<Instant>,
    /// A world shot's or test's rows: the live ones are not read over them.
    #[cfg(test)]
    frozen: bool,
    ui: MenuCanvas,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            open: false,
            owns_console: false,
            handed: None,
            game_data: PathBuf::new(),
            rows: Vec::new(),
            statuses: Vec::new(),
            lowered: Vec::new(),
            query: String::new(),
            shown: Vec::new(),
            selected: 0,
            first: 0,
            focus: Focus::Search,
            can_share: false,
            has_index: false,
            busy: false,
            auto_on: false,
            lines: Vec::new(),
            seen: None,
            read_at: None,
            #[cfg(test)]
            frozen: false,
            ui: MenuCanvas::with_text_capacity(128),
        }
    }
}

impl Page {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page with an empty search, its rows read again at once;
    /// `owns_console` when it opened the console.
    pub(crate) fn open(&mut self, owns_console: bool, game_data: &Path) {
        self.open = true;
        self.owns_console = owns_console;
        game_data.clone_into(&mut self.game_data);
        self.query.clear();
        self.focus = Focus::Search;
        self.selected = 0;
        self.first = 0;
        self.seen = None;
        self.refilter();
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    /// The next opening takes the console over from the page it replaces:
    /// `owns_console` is whether that one had opened it.
    pub(crate) fn hand_console(&mut self, owns_console: bool) {
        self.handed = Some(owns_console);
    }

    pub(crate) fn take_handed(&mut self) -> Option<bool> {
        self.handed.take()
    }

    pub(crate) fn game_data(&self) -> &Path {
        &self.game_data
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// What may change every frame: whether the player may share, a job is at work,
    /// and `cl_autoModels`.
    pub(crate) fn set_live(&mut self, can_share: bool, busy: bool, auto_on: bool) {
        #[cfg(test)]
        if self.frozen {
            return;
        }
        if can_share != self.can_share {
            self.can_share = can_share;
            self.settle_focus();
        }
        self.busy = busy;
        self.auto_on = auto_on;
    }

    /// Whether the rows should be read again: model sync's `changes` moved on, or
    /// they were read a while ago.
    pub(crate) fn wants_rows(&self, changes: u64, now: Instant) -> bool {
        #[cfg(test)]
        if self.frozen {
            return false;
        }
        self.seen != Some(changes)
            || self
                .read_at
                .is_none_or(|at| now.duration_since(at) >= REREAD)
    }

    /// The rows read ([`crate::model_sync::page_rows`]), with whether the cloud's list
    /// was read, the lines said and the `changes` they were read at.
    pub(crate) fn set_rows(
        &mut self,
        rows: Vec<PageRow>,
        has_index: bool,
        lines: Vec<String>,
        changes: u64,
        now: Instant,
    ) {
        self.seen = Some(changes);
        self.read_at = Some(now);
        self.has_index = has_index;
        self.lines = lines;
        if rows != self.rows {
            let chosen = self.chosen().map(|row| row.name.clone());
            self.statuses = rows.iter().map(|row| row.status(has_index)).collect();
            self.lowered = rows.iter().map(|row| row.name.to_lowercase()).collect();
            self.rows = rows;
            self.refilter();
            if let Some(at) = chosen.and_then(|name| {
                self.shown
                    .iter()
                    .position(|&index| self.rows[index].name == name)
            }) {
                self.select(at);
            }
        } else {
            self.statuses = self.rows.iter().map(|row| row.status(has_index)).collect();
        }
    }

    /// Show these rows and states in place of the live ones (world shots and tests).
    #[cfg(test)]
    pub(crate) fn preview(
        &mut self,
        rows: Vec<PageRow>,
        can_share: bool,
        busy: bool,
        lines: Vec<String>,
    ) {
        self.frozen = false;
        self.set_live(can_share, busy, false);
        self.set_rows(rows, true, lines, 0, Instant::now());
        self.frozen = true;
    }

    /// The chosen row, if any.
    fn chosen(&self) -> Option<&PageRow> {
        self.rows.get(*self.shown.get(self.selected)?)
    }

    /// The buttons of the row at `index` of `rows`, in order.
    fn buttons_of(&self, index: usize) -> impl Iterator<Item = RowAction> + use<> {
        let actions = self
            .rows
            .get(index)
            .map_or([None; ROW_BUTTONS], |row| row.actions(self.can_share));
        actions.into_iter().flatten()
    }

    /// How many buttons the chosen row has.
    fn chosen_buttons(&self) -> usize {
        self.shown
            .get(self.selected)
            .map_or(0, |&index| self.buttons_of(index).count())
    }

    /// Keep the rows whose name has every word of the search.
    fn refilter(&mut self) {
        let words: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        self.shown = (0..self.rows.len())
            .filter(|&index| words.iter().all(|word| self.lowered[index].contains(word)))
            .collect();
        self.select(self.selected);
    }

    fn select(&mut self, row: usize) {
        self.selected = row.min(self.shown.len().saturating_sub(1));
        if self.selected < self.first {
            self.first = self.selected;
        }
        if self.selected >= self.first + ROWS {
            self.first = self.selected + 1 - ROWS;
        }
        self.first = self.first.min(self.shown.len().saturating_sub(ROWS));
        self.settle_focus();
    }

    /// A row button that no longer exists gives the focus to the row's last one, or to
    /// the search.
    fn settle_focus(&mut self) {
        if let Focus::Row(button) = self.focus {
            let count = self.chosen_buttons();
            self.focus = if count == 0 {
                Focus::Search
            } else {
                Focus::Row(button.min(count - 1))
            };
        }
    }

    fn step(&mut self, by: isize) {
        self.select(self.selected.saturating_add_signed(by));
    }

    /// Up or Down: choose another row, the keyboard on its buttons when it has any.
    fn step_row(&mut self, by: isize) {
        if let Focus::Search | Focus::Refresh | Focus::Close = self.focus {
            self.focus = Focus::Row(0);
        }
        self.step(by);
    }

    /// The keyboard's stops in order: the search, the chosen row's buttons, Refresh,
    /// Close.
    fn stops(&self) -> Vec<Focus> {
        let mut stops = vec![Focus::Search];
        stops.extend((0..self.chosen_buttons()).map(Focus::Row));
        stops.extend([Focus::Refresh, Focus::Close]);
        stops
    }

    /// Tab, Shift+Tab, Left, Right: the next stop `by` along, round.
    fn cycle(&mut self, by: isize) {
        let stops = self.stops();
        let at = stops
            .iter()
            .position(|&stop| stop == self.focus)
            .unwrap_or(0);
        let next = (at as isize + by).rem_euclid(stops.len() as isize) as usize;
        self.focus = stops[next];
    }

    /// Press button `button` of the chosen row.
    fn press_row(&self, button: usize) -> PageAction {
        let Some(&index) = self.shown.get(self.selected) else {
            return PageAction::None;
        };
        match self.buttons_of(index).nth(button) {
            Some(action) => PageAction::Run(action, self.rows[index].name.clone()),
            None => PageAction::None,
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        shift: bool,
        control: bool,
    ) -> PageAction {
        if event.state != ElementState::Pressed {
            return PageAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PageAction::None;
        };
        self.key(key, event.text.as_deref(), event.repeat, shift, control)
    }

    /// A pressed `key` that typed `text` (`repeat` when held).
    fn key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
        repeat: bool,
        shift: bool,
        control: bool,
    ) -> PageAction {
        match key {
            KeyCode::Escape if !repeat => return PageAction::Close,
            KeyCode::Escape => {}
            KeyCode::Enter | KeyCode::NumpadEnter if !repeat => {
                return match self.focus {
                    Focus::Search => self.press_row(0),
                    Focus::Row(button) => self.press_row(button),
                    Focus::Refresh => PageAction::Refresh,
                    Focus::Close => PageAction::Close,
                };
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {}
            KeyCode::Tab => self.cycle(if shift { -1 } else { 1 }),
            KeyCode::ArrowLeft => self.cycle(-1),
            KeyCode::ArrowRight => self.cycle(1),
            KeyCode::ArrowUp => self.step_row(-1),
            KeyCode::ArrowDown => self.step_row(1),
            KeyCode::PageUp => self.step_row(-(ROWS as isize)),
            KeyCode::PageDown => self.step_row(ROWS as isize),
            KeyCode::Home if self.focus != Focus::Search => self.select(0),
            KeyCode::End if self.focus != Focus::Search => self.select(usize::MAX),
            _ => {
                if crate::text_entry::edit(
                    &mut self.query,
                    key,
                    text,
                    shift,
                    control,
                    SEARCH_MAX,
                    true,
                ) {
                    self.focus = Focus::Search;
                    self.refilter();
                }
            }
        }
        PageAction::None
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PageAction {
        let Some(event) = self.ui.pointer(event) else {
            return PageAction::None;
        };
        match event.kind {
            UiEventKind::Wheel => {
                let direction = event.delta.map_or(0, |delta| -delta.y.signum() as isize);
                self.step(direction * 3);
                PageAction::None
            }
            UiEventKind::Activate => match event.token {
                Some(BACK_TOKEN) => PageAction::Close,
                Some(REFRESH_TOKEN) => {
                    self.focus = Focus::Refresh;
                    PageAction::Refresh
                }
                Some(SEARCH_TOKEN) => {
                    self.focus = Focus::Search;
                    crate::text_entry::clicked(&self.query);
                    PageAction::None
                }
                Some(token) if usize::from(token) < ROWS * ROW_BUTTONS => {
                    let slot = usize::from(token) / ROW_BUTTONS;
                    let button = usize::from(token) % ROW_BUTTONS;
                    let row = self.first + slot;
                    if row >= self.shown.len() {
                        return PageAction::None;
                    }
                    self.select(row);
                    self.focus = Focus::Row(button);
                    self.settle_focus();
                    self.press_row(button)
                }
                Some(token) if (ROW_TOKEN..ROW_TOKEN + ROWS as u16).contains(&token) => {
                    let row = self.first + usize::from(token - ROW_TOKEN);
                    if row < self.shown.len() {
                        self.select(row);
                    }
                    PageAction::None
                }
                _ => PageAction::None,
            },
            _ => PageAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::sjk::TextTarget;
    use winit::keyboard::KeyCode;

    fn row(name: &str) -> PageRow {
        PageRow {
            name: name.to_owned(),
            cloud: None,
            fetched: None,
            host_pack: false,
            installed: true,
            worn: false,
        }
    }

    /// The rows of the tests and the world shot: one of each case.
    pub(crate) fn sample_rows() -> Vec<PageRow> {
        vec![
            PageRow {
                worn: true,
                cloud: Some(3),
                ..row("solfox")
            },
            row("ahsoka_rebels"),
            PageRow {
                host_pack: true,
                ..row("darth_revan")
            },
            PageRow {
                cloud: Some(2),
                installed: false,
                ..row("grogu")
            },
            PageRow {
                cloud: Some(4),
                fetched: Some(2),
                ..row("kylo_tros")
            },
            PageRow {
                cloud: Some(1),
                fetched: Some(1),
                ..row("vinfox")
            },
        ]
    }

    fn page(can_share: bool) -> Page {
        let mut page = Page::default();
        page.open(false, Path::new("."));
        page.preview(sample_rows(), can_share, false, Vec::new());
        page
    }

    fn labels(page: &Page, name: &str) -> Vec<&'static str> {
        let index = page.rows.iter().position(|row| row.name == name).unwrap();
        page.buttons_of(index).map(RowAction::label).collect()
    }

    fn draw(page: &mut Page, viewport: [f32; 2]) {
        let font = crate::text::load_modern(1.0, None).unwrap().font;
        let mut vertices = Vec::new();
        page.append_sjk(TextTarget::Inter(&mut vertices, &font), viewport);
    }

    /// Press `code`, which types `text`.
    fn press(page: &mut Page, code: KeyCode, text: Option<&str>) -> PageAction {
        page.key(code, text, false, false, false)
    }

    #[test]
    fn each_row_offers_the_buttons_its_state_allows() {
        let page = page(true);
        // Installed but never fetched: the cloud's version can still be taken.
        assert_eq!(labels(&page, "solfox"), ["Update", "Download"]);
        assert_eq!(labels(&page, "ahsoka_rebels"), ["Share"]);
        assert_eq!(labels(&page, "darth_revan"), ["Update"]);
        assert_eq!(labels(&page, "grogu"), ["Download"]);
        assert_eq!(labels(&page, "kylo_tros"), ["Update", "Download"]);
        assert_eq!(labels(&page, "vinfox"), ["Update"]);
        let page = super::tests::page(false);
        assert_eq!(labels(&page, "solfox"), ["Download"]);
        assert!(labels(&page, "ahsoka_rebels").is_empty());
        assert_eq!(labels(&page, "grogu"), ["Download"]);
        assert_eq!(labels(&page, "kylo_tros"), ["Download"]);
    }

    #[test]
    fn the_status_says_where_each_model_stands() {
        let page = page(true);
        let status = |name: &str| {
            let index = page.rows.iter().position(|row| row.name == name).unwrap();
            page.statuses[index].clone()
        };
        assert_eq!(status("solfox"), "On the cloud: v3");
        assert_eq!(status("ahsoka_rebels"), "Not shared");
        assert_eq!(status("darth_revan"), "In the Sith cloud's packs");
        assert_eq!(status("kylo_tros"), "Downloaded v2, v4 on the cloud");
        assert_eq!(status("vinfox"), "Downloaded v1");
        assert_eq!(row("x").status(false), "Not checked yet");
    }

    #[test]
    fn the_search_keeps_the_names_with_every_word() {
        let mut page = page(true);
        assert_eq!(page.shown.len(), 6);
        for (code, text) in [
            (KeyCode::KeyF, "f"),
            (KeyCode::KeyO, "o"),
            (KeyCode::KeyX, "x"),
        ] {
            let _ = press(&mut page, code, Some(text));
        }
        assert_eq!(page.query, "fox");
        assert_eq!(page.shown.len(), 2);
        let _ = press(&mut page, KeyCode::Backspace, None);
        assert_eq!(page.query, "fo");
        page.query = "nobody".into();
        page.refilter();
        assert!(page.chosen().is_none());
        assert_eq!(page.press_row(0), PageAction::None);
    }

    #[test]
    fn keys_move_between_the_buttons_and_enter_presses_them() {
        let mut page = page(true);
        // Down to kylo_tros (the fifth row): Update then Download.
        for _ in 0..4 {
            let _ = press(&mut page, KeyCode::ArrowDown, None);
        }
        assert_eq!(page.focus, Focus::Row(0));
        assert_eq!(
            press(&mut page, KeyCode::Enter, None),
            PageAction::Run(RowAction::Update, "kylo_tros".into())
        );
        let _ = press(&mut page, KeyCode::Tab, None);
        assert_eq!(
            press(&mut page, KeyCode::Enter, None),
            PageAction::Run(RowAction::Download, "kylo_tros".into())
        );
        let _ = press(&mut page, KeyCode::Tab, None);
        assert_eq!(press(&mut page, KeyCode::Enter, None), PageAction::Refresh);
        let _ = press(&mut page, KeyCode::ArrowRight, None);
        assert_eq!(press(&mut page, KeyCode::Enter, None), PageAction::Close);
        // Down to vinfox, which has one button: the focus goes to a button that exists.
        let _ = press(&mut page, KeyCode::ArrowDown, None);
        assert_eq!(page.focus, Focus::Row(0));
        assert_eq!(press(&mut page, KeyCode::Escape, None), PageAction::Close);
    }

    #[test]
    fn a_click_on_a_button_runs_it_and_the_page_hands_back_its_console() {
        let mut page = page(true);
        draw(&mut page, [1920.0, 1080.0]);
        // grogu is the fourth row: its one button, Download.
        let rect = page.ui.rect_for(3 * ROW_BUTTONS as u16).expect("a button");
        let position = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let button = sjk_ui::PointerButton::Primary;
        let mut actions = Vec::new();
        for event in [
            InputEvent::PointerMove(position),
            InputEvent::PointerPress { position, button },
            InputEvent::PointerRelease { position, button },
        ] {
            actions.push(page.handle_pointer(event));
        }
        assert!(actions.contains(&PageAction::Run(RowAction::Download, "grogu".into())));
        let mut page = Page::default();
        page.open(true, Path::new("."));
        assert!(page.close());
        page.open(false, Path::new("."));
        assert!(!page.close());
    }

    #[test]
    fn the_page_and_its_buttons_fit_common_windows() {
        let mut rows = sample_rows();
        rows.extend((0..40).map(|n| row(&format!("a_model_with_a_rather_long_name_{n:02}"))));
        let lines: Vec<String> = (0..6)
            .map(|n| {
                format!(
                    "^3Model sync: line {n} that says a good deal about what happened, at length."
                )
            })
            .collect();
        for viewport in [
            [1280.0, 720.0],
            [1366.0, 768.0],
            [1920.0, 1080.0],
            [1280.0, 1024.0],
            [2560.0, 1080.0],
            [3840.0, 2160.0],
        ] {
            let mut page = Page::default();
            page.open(false, Path::new("."));
            page.preview(rows.clone(), true, true, lines.clone());
            draw(&mut page, viewport);
            let window = sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]);
            for token in (0..(ROWS * ROW_BUTTONS) as u16)
                .chain(ROW_TOKEN..ROW_TOKEN + ROWS as u16)
                .chain([SEARCH_TOKEN, REFRESH_TOKEN, BACK_TOKEN])
            {
                let Some(rect) = page.ui.rect_for(token) else {
                    continue;
                };
                assert!(
                    rect.x >= window.x
                        && rect.y >= window.y
                        && rect.right() <= window.right() + 0.5
                        && rect.bottom() <= window.bottom() + 0.5,
                    "{token} at {rect:?} leaves {viewport:?}"
                );
            }
            // Every row slot has its region, and the row buttons stay inside their row.
            for slot in 0..ROWS as u16 {
                let row = page.ui.rect_for(ROW_TOKEN + slot).expect("a row");
                for button in 0..ROW_BUTTONS as u16 {
                    if let Some(rect) = page.ui.rect_for(slot * ROW_BUTTONS as u16 + button) {
                        assert!(
                            rect.x >= row.x && rect.right() <= row.right() + 0.5,
                            "{slot}/{button} leaves its row at {viewport:?}"
                        );
                        assert!(rect.y >= row.y && rect.bottom() <= row.bottom() + 0.5);
                    }
                }
            }
            // The two buttons of a row do not overlap.
            let slot = page
                .shown
                .iter()
                .position(|&index| page.rows[index].name == "kylo_tros")
                .unwrap() as u16;
            let update = page.ui.rect_for(slot * 2).unwrap();
            let download = page.ui.rect_for(slot * 2 + 1).unwrap();
            assert!(update.right() <= download.x, "{viewport:?}");
        }
    }

    #[test]
    fn colour_codes_leave_the_lines_and_a_warning_is_told() {
        assert_eq!(
            view::plain("^3Model sync: ^7grogu: refused."),
            (true, "Model sync: grogu: refused.".to_owned())
        );
        assert_eq!(
            view::plain("Model sync: grogu shared."),
            (false, "Model sync: grogu shared.".to_owned())
        );
    }
}
