//! The SJK chat page (`docs/hub-chat.md`): the chat every SJK player shares through
//! the SJK hub, in full. The history with names, verified and staff marks and how
//! long ago each message came; a field to write in and Send; a small window of who is
//! online and who was seen most recently ([`people`]), or, with its List, every online
//! player as a row with the clients they read the chat with; and, for a chosen message, Mute
//! on this PC, Ban 24 h for a player the hub vouches for when the sender is one it does
//! not ([`crate::chat_ban`]), and for SJK staff Delete, Lift chat ban and Mute at the
//! hub. Resting the pointer on a
//! name, in the chat or the window, shows that player's sender card with Mute or
//! Unmute ([`crate::sender_card`], "Muting a player"). GIF beside Send opens the GIF
//! picker over the messages ([`gif_picker`]): favourites, GIPHY's search and the GIFs
//! seen in the chat; a chosen message's GIF can be made a favourite. Search needs the
//! player's own GIPHY key: without one the key card says how to get it and takes it.
//!
//! Opened by the main page's docked chat (Open chat), the `sjkchat` command or the
//! in-game SJK menu. Like the Staff page it lives in the console and has the SJK UI's
//! look ([`view`]). The field has the keyboard when the page opens: typing goes to
//! it, Enter sends, Up goes to the newest message; Up and Down move through the
//! messages, Page Up and Page Down scroll, Tab walks every control, Escape goes back
//! (or closes the GIF picker first).

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_identity::{ChatState, StaffRequest};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "sjk_chat_gif_picker.rs"]
mod gif_picker;
#[path = "sjk_chat_gif_sources.rs"]
mod gif_sources;
#[path = "sjk_chat_people.rs"]
mod people;
#[path = "sjk_chat_reactions.rs"]
mod reactions;
#[path = "sjk_chat_panel_view.rs"]
mod view;

const FIELD_TOKEN: u16 = 1_300;
const SEND_TOKEN: u16 = 1_301;
const MUTE_TOKEN: u16 = 1_302;
const DELETE_TOKEN: u16 = 1_303;
const HUB_MUTE_TOKEN: u16 = 1_304;
const HUB_UNMUTE_TOKEN: u16 = 1_305;
/// GIF beside Send (the picker), and the chosen message's Add GIF to favourites.
const GIF_TOKEN: u16 = 1_306;
const FAVOURITE_TOKEN: u16 = 1_307;
/// The chosen message's "Hide this GIF" / "Show this GIF again" ([`crate::chat_gifs::hidden`]).
const HIDE_GIF_TOKEN: u16 = 1_366;
/// Retry, beside the notice that says the hub did not take the last message.
const RETRY_TOKEN: u16 = 1_308;
/// The chosen message's Ban 24 h, and staff's Lift chat ban.
const BAN_TOKEN: u16 = 1_309;
const LIFT_TOKEN: u16 = 1_363;
/// Send only in SJK, under Send while a mod says SJK chat in this server's All chat.
const ONLY_TOKEN: u16 = 1_364;
/// The chosen message's Translate (`chat/translate.rs`).
const TRANSLATE_TOKEN: u16 = 1_367;
/// Translations the page keeps, the newest last.
const TRANSLATIONS_KEPT: usize = 16;
/// Messages on show, one token each.
const MESSAGE_BASE: u16 = 1_310;
const MESSAGES_SHOWN: usize = 40;
/// The sender card a name shows under the pointer, and its Mute or Unmute.
const CARD_TOKEN: u16 = 1_360;
const CARD_MUTE_TOKEN: u16 = 1_361;
const CARD_BAN_TOKEN: u16 = 1_362;
const CARD_TRADE_TOKEN: u16 = 1_365;
/// The names of the messages on show, one token each.
const NAME_BASE: u16 = 1_370;
/// The players in the who-is-online window, one token each.
const PERSON_BASE: u16 = 1_420;
/// The window's List / Faces, and the "+N" face (which opens the list too).
const LIST_TOKEN: u16 = 1_480;
/// Unblock beside each blocked player the side lists while no message is chosen.
const UNBLOCK_BASE: u16 = 1_470;
const UNBLOCK_SHOWN: usize = 6;
const PEOPLE_SHOWN: usize = people::ONLINE_FACES + people::RECENT_FACES;
/// Pictures' versions the page keeps for faces whose row does not carry one.
const PICTURES_KEPT: usize = 32;
/// Longest message, as the hub takes it.
const DRAFT_MAX: usize = crate::sjk_chat_field::MAX;
/// Messages Page Up and Page Down scroll by.
const PAGE: usize = 5;

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Send this message to the SJK chat.
    Send(String),
    /// Set a reaction, without sending chat text.
    React {
        message: u64,
        emoji: String,
        active: bool,
    },
    /// Send it to the SJK chat only, never said in the server's All chat.
    SendSjkOnly(String),
    /// Mute or unmute, on this PC, the player with this key, last seen with this name.
    Mute {
        key_id: String,
        name: String,
        muted: bool,
    },
    /// A click on Ban 24 h for the player with this key ([`crate::chat_ban::press`]:
    /// it arms the button, or sends the ban).
    Ban(String),
    /// A click on Trade for the player with this key ([`crate::player_trades::ask_by_key`]):
    /// the page closes and the trade window opens.
    Trade(String),
    /// A staff request to the hub.
    Staff(StaffRequest),
    /// Save this as the player's GIPHY key (`cl_giphyKey`), from the key card.
    GiphyKey(String),
    /// Ask the hub for the message of this id, whose plain text this is, in English
    /// (`chat/translate.rs`).
    Translate {
        message: u64,
        text: String,
    },
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// The chat, or `None` while the identity is not running.
    pub(crate) chat: Option<&'a ChatState>,
    /// The keys muted on this PC.
    pub(crate) muted: &'a [String],
    /// Every player blocked on this PC (the mute list), in the order they were blocked:
    /// listed with Unblock while no message is chosen.
    pub(crate) blocked: &'a [crate::chat_mutes::Mute],
    /// Whether the player's key is SJK staff.
    pub(crate) staff: bool,
    /// Whether `cl_sjkChat` is on.
    pub(crate) enabled: bool,
    /// Now, unix seconds, for how long ago messages came.
    pub(crate) now: u64,
    /// What the staff requests came to, for a staff key.
    pub(crate) staff_state: Option<StaffShown<'a>>,
    /// `cl_giphyKey`, the player's own GIPHY key for the picker's search (empty: none).
    pub(crate) giphy_key: &'a str,
    /// While a mod says SJK chat in this server's All chat, its words
    /// (`chat/server_copy.rs`): the page offers Send only in SJK.
    pub(crate) said_in_all: Option<crate::chat::server_copy::SaidInAll>,
    /// The languages never offered for translation while `cl_chatTranslate` is on;
    /// `None` while it is off, and the page offers no Translate.
    pub(crate) translate: Option<&'a crate::chat::translate::Skip>,
}

/// The part of `sjk_identity::StaffState` the page shows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StaffShown<'a> {
    /// Counts answers.
    pub(crate) serial: u64,
    pub(crate) message: &'a str,
    pub(crate) failed: bool,
    /// A request is on its way.
    pub(crate) busy: bool,
}

/// A translation the page asked for: the message, the request's serial, and the line
/// it came to (the translation or why there is none) once the hub answered.
#[derive(Debug)]
struct Translated {
    message: u64,
    serial: u64,
    line: Option<String>,
}

/// What the last frame showed, for keys and clicks.
#[derive(Debug, Default)]
struct Shown {
    /// The messages on show, by token, as (id, sender's key), oldest first.
    messages: Vec<(u64, String)>,
    /// What a click on each message on show opens, for the SJK team's lines.
    officials: Vec<Option<crate::official::words::Target>>,
    /// The chosen message's id, its sender's key and name, and whether they are
    /// muted here.
    chosen: Option<(u64, String, String, bool)>,
    reaction_mine: [bool; 6],
    /// The chosen message's GIF, when it has one on show.
    chosen_gif: Option<crate::chat_gifs::GifId>,
    chosen_source: Option<crate::chat_gifs::GifId>,
    /// How many messages there are in all.
    total: usize,
    /// Whether the sender card's player is muted here.
    card_muted: bool,
    /// The blocked players listed with Unblock, in token order.
    blocked: Vec<crate::chat_mutes::Mute>,
    /// Whether the chosen message offers Ban 24 h.
    chosen_bannable: bool,
    /// Whether the chosen message offers Translate.
    chosen_translate: bool,
    /// The keys of the players in the who-is-online window, by token.
    people: Vec<String>,
    /// The keys of faces in the window drawn without knowing their picture: the
    /// chat's last senders, whose messages do not say ([`Panel::find_pictures`]).
    unpictured: Vec<String>,
    /// The window's rows while it lists every online player, where the wheel scrolls
    /// them.
    list_rect: Option<sjk_ui::Rect>,
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    ui: MenuCanvas,
    draft: String,
    /// The caret in `draft` ([`crate::sjk_chat_field::caret_key`]).
    pub(crate) caret: crate::console::line_edit::LineEdit,
    /// The last message handed to the hub, which Retry sends again.
    last_sent: String,
    /// The control the keyboard is on, by its token.
    focus: u16,
    /// The controls in the order Tab visits them, laid out by the last frame.
    order: Vec<u16>,
    /// The chosen message's id.
    selected: Option<u64>,
    /// Messages hidden below the list, scrolled up past.
    scroll: usize,
    /// The who-is-online window lists every online player as a row (its List) in place
    /// of the faces, over the chosen message; and the rows scrolled past.
    people_list: bool,
    people_scroll: usize,
    /// The staff answers' serial when the page last sent a staff request: answers
    /// past it are this page's.
    staff_after: Option<u64>,
    shown: Shown,
    /// The sender card on show.
    card: Option<Hovered>,
    /// The pictures' versions found for faces without one (key, version; empty for
    /// none), newest last.
    pictures: Vec<(String, String)>,
    /// Where the pointer last moved to, and the window's faces the frame before: the
    /// canvas knows the target under the pointer by its place in the frame's list, so
    /// when the faces change under a resting pointer it is looked for again.
    pointer: Option<sjk_ui::Vec2>,
    previous_people: Vec<String>,
    epoch: Instant,
    reaction_notice: reactions::Notice,
    /// The GIF picker.
    gifs: gif_picker::Picker,
    /// `cl_giphyKey` as the last frame had it, for the key card's Change GIPHY key.
    giphy_key: String,
    /// Send only in SJK's hover card.
    hover: crate::menu::sjk::hover::Hover,
    /// The last frame offered Send only in SJK to a staff key: Shift+Enter takes it.
    only_for_staff: bool,
    /// The translations the page asked for, the newest last ([`Panel::translation_sent`]).
    translations: Vec<Translated>,
    /// The language guess of the chosen message: its id, the skip list it was made
    /// with, and its plain text while it offers Translate.
    translate_guess: Option<(u64, crate::chat::translate::Skip, Option<String>)>,
    /// The chat to show in place of the live one, for a world shot.
    #[cfg(test)]
    pub(crate) preview: Option<ChatState>,
}

/// The sender card on show: the message whose sender it is about (or the player in the
/// who-is-online window), who, and the name it is beside. Where they are on the server
/// is asked outside the chat's lock ([`Panel::place_card`]), so it shows a frame later.
struct Hovered {
    source: CardSource,
    person: crate::sender_card::Person,
    anchor: sjk_ui::Rect,
    placed: bool,
    /// When the pointer left the name and the card, while it is off both: the card
    /// waits [`CARD_GRACE`] for the pointer on its way across the gap to it.
    left: Option<std::time::Instant>,
}

/// How long a sender card stays once the pointer is off its name and the card, so the
/// way from the name (or face) to the card's buttons never loses it (as the game's
/// chat, `chat::card::GRACE_MS`).
const CARD_GRACE: std::time::Duration = std::time::Duration::from_millis(400);

/// What a sender card on show is about.
#[derive(Clone, Debug, Eq, PartialEq)]
enum CardSource {
    /// The sender of the message with this id.
    Message(u64),
    /// The player with this key in the who-is-online window.
    Person(String),
}

impl Panel {
    /// Type `draft` and show Send only in SJK's hover card in full, for a world shot.
    #[cfg(test)]
    pub(crate) fn only_sjk_for_shot(&mut self, draft: &str) {
        self.draft = draft.to_owned();
        self.hover.pin(Some(ONLY_TOKEN));
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            ui: MenuCanvas::with_capacities(260, 160, 1_600),
            draft: String::new(),
            caret: crate::console::line_edit::LineEdit::default(),
            last_sent: String::new(),
            focus: FIELD_TOKEN,
            order: Vec::with_capacity(64),
            selected: None,
            scroll: 0,
            people_list: false,
            people_scroll: 0,
            staff_after: None,
            shown: Shown::default(),
            card: None,
            pictures: Vec::new(),
            pointer: None,
            previous_people: Vec::with_capacity(PEOPLE_SHOWN),
            epoch: Instant::now(),
            reaction_notice: reactions::Notice::default(),
            gifs: gif_picker::Picker::new(),
            giphy_key: String::new(),
            hover: Default::default(),
            only_for_staff: false,
            translations: Vec::new(),
            translate_guess: None,
            #[cfg(test)]
            preview: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page, the keyboard in the field; `owns_console` when the console was
    /// closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.focus = FIELD_TOKEN;
        self.scroll = 0;
        self.people_scroll = 0;
        self.gifs.close();
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    /// The page sent a staff request while the staff answers' serial was `serial`.
    pub(crate) fn staff_sent(&mut self, serial: u64) {
        self.staff_after = Some(serial);
    }

    /// The hub was asked for message `message` in English under `serial`
    /// ([`sjk_identity::Service::translate`]): the translation shows under it once the
    /// hub answers.
    pub(crate) fn translation_sent(&mut self, message: u64, serial: u64) {
        self.translations.retain(|kept| kept.message != message);
        if self.translations.len() == TRANSLATIONS_KEPT {
            self.translations.remove(0);
        }
        self.translations.push(Translated {
            message,
            serial,
            line: None,
        });
    }

    /// Take what the hub answered for the translations still waiting.
    fn follow_translations(&mut self, chat: Option<&ChatState>) {
        let Some(chat) = chat else { return };
        for kept in self
            .translations
            .iter_mut()
            .filter(|kept| kept.line.is_none())
        {
            if let Some(outcome) = chat
                .translations
                .iter()
                .find(|outcome| outcome.serial == kept.serial)
            {
                kept.line = Some(match &outcome.result {
                    Ok(translation) => crate::chat::translate::line(translation),
                    Err(error) => format!("Translate: {error}"),
                });
            }
        }
    }

    /// What message `id`'s translation came to, or that it is on its way.
    fn translation_of(&self, id: u64) -> Option<&str> {
        self.translations
            .iter()
            .find(|kept| kept.message == id)
            .map(|kept| kept.line.as_deref().unwrap_or("Translating..."))
    }

    /// Say on the sender card on show where its player is on the server being played
    /// (`place`, from their key and name, once) and their picture's version (`picture`,
    /// from their key, until it is known). Called before the chat's lock is taken.
    pub(crate) fn place_card(
        &mut self,
        place: impl FnOnce(Option<&str>, &str) -> crate::sender_card::Place,
        picture: impl FnOnce(&str) -> Option<(String, &'static str)>,
    ) {
        let Some(card) = self.card.as_mut() else {
            return;
        };
        if !card.placed {
            card.person.place = place(card.person.key_id.as_deref(), &card.person.name);
            card.placed = true;
        }
        if card.person.avatar.is_none()
            && let Some(key_id) = &card.person.key_id
            && let Some((avatar, banner)) = picture(key_id)
        {
            card.person.avatar = Some(avatar);
            card.person.banner = banner;
        }
    }

    /// Find the pictures of the faces the last frame drew without one, through
    /// `picture` (a key's picture version, empty for none, `None` while not known),
    /// outside the chat's lock as [`Panel::place_card`] does.
    pub(crate) fn find_pictures(&mut self, picture: impl Fn(&str) -> Option<String>) {
        for key in std::mem::take(&mut self.shown.unpictured) {
            if self.pictures.iter().any(|(known, _)| *known == key) {
                continue;
            }
            if let Some(version) = picture(&key) {
                if self.pictures.len() == PICTURES_KEPT {
                    self.pictures.remove(0);
                }
                self.pictures.push((key, version));
            }
        }
    }

    /// The picture version found for `key_id`'s face, if any.
    fn picture_of(&self, key_id: &str) -> Option<&str> {
        self.pictures
            .iter()
            .find(|(key, _)| key == key_id)
            .map(|(_, version)| version.as_str())
    }

    /// Choose the message with `id`, for a world shot.
    #[cfg(test)]
    pub(crate) fn choose_for_shot(&mut self, id: u64) {
        self.selected = Some(id);
    }

    /// Rest the pointer on the face of the `index`th player of the who-is-online
    /// window, as the last frame laid it out, or on the field for `None`, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn hover_person_for_shot(&mut self, index: Option<u16>) {
        let token = index.map_or(FIELD_TOKEN, |index| PERSON_BASE + index);
        if let Some(rect) = self.ui.rect_for(token) {
            let centre = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let _ = self.handle_pointer(InputEvent::PointerMove(centre));
        }
    }

    /// Show the who-is-online window as its list (or its faces), for a world shot.
    #[cfg(test)]
    pub(crate) fn people_list_for_shot(&mut self, list: bool) {
        self.people_list = list;
        self.people_scroll = 0;
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Open the GIF picker on tab `tab` (0 Favourites, 1 Search, 2 Seen in chat), the
    /// words `typed` searched, the keyboard on the second tile, for a world shot.
    #[cfg(test)]
    pub(crate) fn open_gifs_for_shot(&mut self, tab: usize, typed: &str) {
        let _ = self.gifs.open();
        self.gifs.choose_tab(gif_picker::Tab::ALL[tab]);
        self.gifs.typed = typed.to_owned();
        self.gifs.search_now();
        if self.gifs.tab == gif_picker::Tab::Sources {
            self.gifs.sources.typed = typed.to_owned();
            self.focus = gif_sources::FIELD;
        } else {
            self.focus = gif_picker::TILE_BASE + 1;
        }
    }

    /// Open the GIF picker's key card with `typed` in its field, the keyboard on it,
    /// for a world shot.
    #[cfg(test)]
    pub(crate) fn open_key_card_for_shot(&mut self, typed: &str) {
        let _ = self.gifs.open();
        self.gifs.choose_tab(gif_picker::Tab::Search);
        self.gifs.open_key_card(typed);
        self.focus = gif_picker::KEY_FIELD_TOKEN;
    }

    /// Answer the picker's search on its way with `ids` of `total`, for a world shot.
    #[cfg(test)]
    pub(crate) fn answer_gif_search_for_shot(
        &mut self,
        ids: Vec<crate::chat_gifs::GifId>,
        total: usize,
    ) {
        self.gifs
            .searcher
            .answer_for_test(Ok(crate::chat_gifs::search::Page { ids, total }));
    }

    /// Open or close the GIF picker; the keyboard goes into it, or back to the field.
    fn toggle_gifs(&mut self) {
        if self.gifs.open {
            self.gifs.close();
            self.focus = FIELD_TOKEN;
        } else if crate::chat_gifs::enabled() {
            self.focus = self.gifs.open();
        }
    }

    /// What a click (or Enter) on one of the picker's controls does; `None` for a token
    /// that is not the picker's.
    fn activate_gifs(&mut self, token: u16) -> Option<PanelAction> {
        use gif_picker::{
            KEY_ASK_TOKEN, KEY_FIELD_TOKEN, KEY_LATER_TOKEN, KEY_OPEN_TOKEN, KEY_SAVE_TOKEN,
            SEARCH_TOKEN, STAR_BASE, TAB_BASE, TILE_BASE, TILES, Tab,
        };
        if !self.gifs.open {
            return None;
        }
        if self.gifs.tab == Tab::Sources && self.gifs.sources.activate(token) {
            self.focus = if token == gif_sources::TRUST || token == gif_sources::REMOVE {
                token
            } else {
                gif_sources::FIELD
            };
            return Some(PanelAction::None);
        }
        match token {
            KEY_ASK_TOKEN => {
                self.gifs.open_key_card(&self.giphy_key);
                self.focus = KEY_FIELD_TOKEN;
                return Some(PanelAction::None);
            }
            KEY_FIELD_TOKEN | KEY_OPEN_TOKEN | KEY_SAVE_TOKEN | KEY_LATER_TOKEN
                if self.gifs.key_card.is_some() =>
            {
                self.focus = token;
                match token {
                    KEY_OPEN_TOKEN if !cfg!(test) => {
                        crate::update::open_page(crate::chat_gifs::search::KEY_PAGE);
                    }
                    KEY_SAVE_TOKEN => {
                        if let Some(key) = self.gifs.save_key() {
                            self.focus = SEARCH_TOKEN;
                            return Some(PanelAction::GiphyKey(key));
                        }
                        self.focus = KEY_FIELD_TOKEN;
                    }
                    KEY_LATER_TOKEN => {
                        self.gifs.put_off_key_card();
                        self.focus = SEARCH_TOKEN;
                    }
                    _ => {}
                }
                return Some(PanelAction::None);
            }
            _ => {}
        }
        if let Some(index) = token
            .checked_sub(TAB_BASE)
            .map(usize::from)
            .filter(|index| *index < Tab::ALL.len())
        {
            self.gifs.choose_tab(Tab::ALL[index]);
            self.focus = TAB_BASE + index as u16;
            return Some(PanelAction::None);
        }
        if token == SEARCH_TOKEN {
            self.focus = SEARCH_TOKEN;
            self.gifs.search_now();
            return Some(PanelAction::None);
        }
        if token == gif_picker::UNHIDE_TOKEN && self.gifs.tab == Tab::Hidden {
            if let Some(id) = self.gifs.hidden_chosen.take() {
                crate::chat_gifs::hidden::set(&id, false);
            }
            self.focus = TAB_BASE + Tab::Hidden.index() as u16;
            return Some(PanelAction::None);
        }
        if (TILE_BASE..TILE_BASE + TILES as u16).contains(&token) {
            let id = self.gifs.tile(token)?.clone();
            if self.gifs.tab == Tab::Hidden {
                // Chosen: Show this GIF again acts on it.
                self.focus = token;
                self.gifs.hidden_chosen = Some(id);
                return Some(PanelAction::None);
            }
            match gif_picker::with_gif(&self.draft, &id) {
                Some(draft) => {
                    self.draft = draft;
                    self.caret.to_end(&self.draft);
                    self.gifs.close();
                    self.focus = FIELD_TOKEN;
                }
                None => {
                    self.gifs.notice =
                        Some("No room for the GIF's link: make the message shorter first");
                }
            }
            return Some(PanelAction::None);
        }
        if (STAR_BASE..STAR_BASE + TILES as u16).contains(&token) {
            if self.gifs.tab == Tab::Hidden {
                return Some(PanelAction::None);
            }
            if let Some(id) = self.gifs.tile(TILE_BASE + (token - STAR_BASE)).cloned() {
                let favourite = crate::chat_gifs::favourites::contains(&id);
                crate::chat_gifs::favourites::set(&id, !favourite);
            }
            return Some(PanelAction::None);
        }
        None
    }

    /// A key while the GIF picker is open, before the page's own; `None` lets the page
    /// have it.
    fn edit_gifs(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
        shift: bool,
        control: bool,
    ) -> Option<PanelAction> {
        use gif_picker::{
            KEY_FIELD_TOKEN, KEY_LATER_TOKEN, SEARCH_TOKEN, TAB_BASE, TILE_BASE, TILES, Tab,
        };
        if !self.gifs.open {
            return None;
        }
        if self.gifs.tab == Tab::Sources
            && key == KeyCode::ArrowDown
            && (TAB_BASE..TAB_BASE + Tab::ALL.len() as u16).contains(&self.focus)
        {
            self.focus = gif_sources::FIELD;
            return Some(PanelAction::None);
        }
        if self.gifs.tab == Tab::Sources && self.focus == gif_sources::FIELD {
            match key {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    return self.activate_gifs(gif_sources::TRUST);
                }
                KeyCode::Escape | KeyCode::Tab => {}
                _ => {
                    self.gifs.sources.edit(key, text, shift, control);
                    return Some(PanelAction::None);
                }
            }
        }
        if self.gifs.key_card.is_some() {
            // The key card has the keyboard: Escape is Not now, Enter in the field saves,
            // and the field takes typing, Ctrl+A, Ctrl+C, Ctrl+V and Backspace.
            let in_field = self.focus == KEY_FIELD_TOKEN;
            return match key {
                KeyCode::Escape => self.activate_gifs(KEY_LATER_TOKEN),
                KeyCode::Enter | KeyCode::NumpadEnter if in_field => {
                    self.activate_gifs(gif_picker::KEY_SAVE_TOKEN)
                }
                KeyCode::Tab
                | KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown => None,
                KeyCode::Space if !in_field => None,
                _ if in_field => {
                    self.gifs.edit_key(key, text, shift, control);
                    Some(PanelAction::None)
                }
                _ => Some(PanelAction::None),
            };
        }
        let on_tile = (TILE_BASE..TILE_BASE + TILES as u16).contains(&self.focus);
        let on_tabs = (TAB_BASE..TAB_BASE + Tab::ALL.len() as u16).contains(&self.focus);
        let searching = self.focus == SEARCH_TOKEN;
        if searching && self.gifs.before_edit(key, text, shift, control) {
            return Some(PanelAction::None);
        }
        if searching && crate::text_entry::is_copy(key, text, control) {
            if !self.gifs.typed.is_empty() {
                let _ = crate::console::clipboard::copy(&self.gifs.typed);
            }
            return Some(PanelAction::None);
        }
        match key {
            KeyCode::Escape => {
                self.toggle_gifs();
            }
            KeyCode::PageUp => self.gifs.scroll(-(gif_picker::ROWS as isize)),
            KeyCode::PageDown => self.gifs.scroll(gif_picker::ROWS as isize),
            KeyCode::Enter | KeyCode::NumpadEnter if searching => self.gifs.search_now(),
            KeyCode::Backspace if searching => self.gifs.backspace(),
            KeyCode::ArrowDown if searching || on_tabs => {
                if self.gifs.tile(TILE_BASE).is_some() {
                    self.focus = TILE_BASE;
                }
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight if on_tabs => {
                let at = self.gifs.tab.index();
                let count = Tab::ALL.len();
                let next = if key == KeyCode::ArrowRight {
                    (at + 1) % count
                } else {
                    (at + count - 1) % count
                };
                return self.activate_gifs(TAB_BASE + next as u16);
            }
            KeyCode::Tab => return None,
            _ if searching => self.gifs.type_key(key, text, shift, control),
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
                if on_tile =>
            {
                let (dx, dy) = match key {
                    KeyCode::ArrowLeft => (-1, 0),
                    KeyCode::ArrowRight => (1, 0),
                    KeyCode::ArrowUp => (0, -1),
                    _ => (0, 1),
                };
                self.focus = match self.gifs.step(self.focus, dx, dy) {
                    Some(token) => token,
                    None if self.gifs.tab == Tab::Search => SEARCH_TOKEN,
                    None => TAB_BASE + self.gifs.tab.index() as u16,
                };
            }
            KeyCode::KeyF if on_tile && !control => {
                return self.activate_gifs(gif_picker::STAR_BASE + (self.focus - TILE_BASE));
            }
            KeyCode::Slash if !searching && self.focus != FIELD_TOKEN => {
                let _ = self.activate_gifs(TAB_BASE + Tab::Search.index() as u16);
                self.focus = SEARCH_TOKEN;
            }
            KeyCode::ArrowUp if self.focus == FIELD_TOKEN => {
                self.focus = if self.gifs.tile(TILE_BASE).is_some() {
                    TILE_BASE
                } else {
                    self.gifs.first_control()
                };
            }
            _ => return None,
        }
        Some(PanelAction::None)
    }

    /// What Enter (or a click) on `token` does.
    fn activate(&mut self, token: u16) -> PanelAction {
        if let Some(action) = reactions::activate(self, token) {
            return action;
        }
        if token == CARD_TOKEN {
            return PanelAction::None;
        }
        if let Some(action) = self.activate_gifs(token) {
            return action;
        }
        if token == gif_sources::OPEN {
            self.open_sources(
                self.shown
                    .chosen_source
                    .as_ref()
                    .map(|id| id.host().to_owned()),
            );
            return PanelAction::None;
        }
        if token == GIF_TOKEN {
            self.toggle_gifs();
            return PanelAction::None;
        }
        if token == FAVOURITE_TOKEN {
            // The chosen message's GIF; the keyboard stays on the button.
            self.focus = token;
            if let Some(id) = self.shown.chosen_gif.clone() {
                let favourite = crate::chat_gifs::favourites::contains(&id);
                crate::chat_gifs::favourites::set(&id, !favourite);
            }
            return PanelAction::None;
        }
        if token == HIDE_GIF_TOKEN {
            // Hidden or shown again on this PC; the keyboard stays on the button.
            self.focus = token;
            if let Some(id) = self.shown.chosen_gif.clone() {
                let hidden = crate::chat_gifs::hidden::contains(&id);
                crate::chat_gifs::hidden::set(&id, !hidden);
            }
            return PanelAction::None;
        }
        if token == CARD_TRADE_TOKEN {
            return self
                .card
                .as_ref()
                .filter(|card| card.person.trade)
                .and_then(|card| card.person.key_id.clone())
                .map_or(PanelAction::None, PanelAction::Trade);
        }
        if token == CARD_BAN_TOKEN {
            // The card's Ban 24 h, only drawn when offered; the keyboard stays.
            return self
                .card
                .as_ref()
                .and_then(|card| card.person.key_id.clone())
                .map_or(PanelAction::None, PanelAction::Ban);
        }
        if token == CARD_MUTE_TOKEN {
            // The card's Mute or Unmute; the keyboard stays where it was.
            let Some(card) = &self.card else {
                return PanelAction::None;
            };
            return PanelAction::Mute {
                key_id: card.person.key_id.clone().unwrap_or_default(),
                name: card.person.name.clone(),
                muted: !self.shown.card_muted,
            };
        }
        if let Some(index) = token
            .checked_sub(UNBLOCK_BASE)
            .filter(|index| usize::from(*index) < UNBLOCK_SHOWN)
        {
            // Unblock a player of the list; the keyboard stays on the list.
            let Some(blocked) = self.shown.blocked.get(usize::from(index)).cloned() else {
                return PanelAction::None;
            };
            return PanelAction::Mute {
                key_id: blocked.key_id.unwrap_or_default(),
                name: blocked.name,
                muted: false,
            };
        }
        if let Some(index) = token
            .checked_sub(NAME_BASE)
            .filter(|index| usize::from(*index) < MESSAGES_SHOWN)
        {
            // A click on a name chooses its message, as one on the row does.
            return self.activate(MESSAGE_BASE + index);
        }
        if token == LIST_TOKEN {
            // List and Faces (and the "+N" face) swap what the window shows.
            self.people_list = !self.people_list;
            self.people_scroll = 0;
            self.focus = LIST_TOKEN;
            return PanelAction::None;
        }
        if let Some(index) = token
            .checked_sub(PERSON_BASE)
            .filter(|index| usize::from(*index) < PEOPLE_SHOWN)
        {
            // A click on a player in the window chooses their newest message on show
            // (in the list it does nothing, so the list stays); the card shows under the
            // pointer either way.
            if self.people_list {
                return PanelAction::None;
            }
            let key = self.shown.people.get(usize::from(index));
            let newest = key.and_then(|key| {
                self.shown
                    .messages
                    .iter()
                    .rposition(|(_, sender)| sender == key)
            });
            if let Some(at) = newest {
                return self.activate(MESSAGE_BASE + at as u16);
            }
            return PanelAction::None;
        }
        self.focus = token;
        let chosen = self.shown.chosen.clone();
        match token {
            FIELD_TOKEN => PanelAction::None,
            SEND_TOKEN | ONLY_TOKEN => {
                let text = self.draft.trim().to_owned();
                // Send only in SJK is laid out only while it is offered.
                if text.is_empty() || (token == ONLY_TOKEN && !self.order.contains(&ONLY_TOKEN)) {
                    return PanelAction::None;
                }
                self.draft.clear();
                self.focus = FIELD_TOKEN;
                self.last_sent.clone_from(&text);
                if token == ONLY_TOKEN {
                    PanelAction::SendSjkOnly(text)
                } else {
                    PanelAction::Send(text)
                }
            }
            RETRY_TOKEN => {
                // Only laid out while a refusal and the text are there.
                if self.last_sent.is_empty() || !self.order.contains(&RETRY_TOKEN) {
                    return PanelAction::None;
                }
                self.focus = FIELD_TOKEN;
                PanelAction::Send(self.last_sent.clone())
            }
            MUTE_TOKEN => match chosen {
                Some((_, key_id, name, muted)) => PanelAction::Mute {
                    key_id,
                    name,
                    muted: !muted,
                },
                None => PanelAction::None,
            },
            TRANSLATE_TOKEN => match (
                chosen,
                self.translate_guess
                    .as_ref()
                    .filter(|_| self.shown.chosen_translate)
                    .and_then(|(_, _, offer)| offer.clone()),
            ) {
                // Only offered (laid out) while the message may be translated.
                (Some((message, ..)), Some(text)) => PanelAction::Translate { message, text },
                _ => PanelAction::None,
            },
            BAN_TOKEN => match chosen {
                // Only offered (laid out) for a sender who may be banned.
                Some((_, key_id, ..)) if self.shown.chosen_bannable => PanelAction::Ban(key_id),
                _ => PanelAction::None,
            },
            DELETE_TOKEN | LIFT_TOKEN | HUB_MUTE_TOKEN | HUB_UNMUTE_TOKEN => {
                // Only offered (laid out) for staff; the hub refuses others too.
                let Some((id, key_id, ..)) = chosen.filter(|_| self.order.contains(&token)) else {
                    return PanelAction::None;
                };
                PanelAction::Staff(match token {
                    DELETE_TOKEN => StaffRequest::ChatDelete { id },
                    LIFT_TOKEN => StaffRequest::ChatUnban { key_id },
                    _ => StaffRequest::ChatMute {
                        key_id,
                        muted: token == HUB_MUTE_TOKEN,
                    },
                })
            }
            token if (MESSAGE_BASE..MESSAGE_BASE + MESSAGES_SHOWN as u16).contains(&token) => {
                // The SJK team's line: a click (or Enter) reads it.
                if let Some(Some(target)) =
                    self.shown.officials.get(usize::from(token - MESSAGE_BASE))
                {
                    crate::official::request(*target);
                    return PanelAction::None;
                }
                self.select_message(token);
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// Put the keyboard on message `token` and choose it, without opening an SJK team's
    /// line (Up and Down pass over it).
    fn select_message(&mut self, token: u16) {
        self.focus = token;
        // The chosen message shows where the list was.
        self.people_list = false;
        if let Some((id, _)) = self.shown.messages.get(usize::from(token - MESSAGE_BASE)) {
            self.selected = Some(*id);
        }
    }

    /// Move the keyboard to the next control (or the one before) of the last frame.
    fn step(&mut self, forward: bool) {
        if self.order.is_empty() {
            return;
        }
        let at = self.order.iter().position(|token| *token == self.focus);
        let next = match (at, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % self.order.len(),
            (Some(at), false) => (at + self.order.len() - 1) % self.order.len(),
        };
        self.focus = self.order[next];
    }

    /// Choose message `index` of those on show, scrolling when it is past an end.
    fn choose(&mut self, index: isize) {
        let count = self.shown.messages.len() as isize;
        if count == 0 {
            return;
        }
        if index < 0 {
            // Older than the oldest on show: scroll up by one, if there is more.
            if self.scroll + self.shown.messages.len() < self.shown.total {
                self.scroll += 1;
                self.selected = None;
            }
            self.select_message(MESSAGE_BASE);
            return;
        }
        if index >= count {
            if self.scroll > 0 {
                self.scroll -= 1;
                self.select_message(MESSAGE_BASE + (count - 1) as u16);
            } else {
                self.focus = FIELD_TOKEN;
            }
            return;
        }
        self.select_message(MESSAGE_BASE + index as u16);
    }

    /// A key.
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
        // Held keys repeat in the fields (Backspace); elsewhere a key acts once.
        let field = [
            FIELD_TOKEN,
            gif_picker::SEARCH_TOKEN,
            gif_picker::KEY_FIELD_TOKEN,
            gif_sources::FIELD,
        ];
        if event.repeat && !field.contains(&self.focus) {
            return PanelAction::None;
        }
        self.hover.keyed();
        self.edit(key, event.text.as_deref(), shift, control)
    }

    /// A pressed `key` that typed `text`, with Shift and Ctrl held or not; in the field,
    /// Ctrl+V and Shift+Insert paste ([`crate::sjk_chat_field`]).
    fn edit(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
        shift: bool,
        control: bool,
    ) -> PanelAction {
        if let Some(action) = self.edit_gifs(key, text, shift, control) {
            return action;
        }
        let typing = self.focus == FIELD_TOKEN;
        if typing && crate::text_entry::before_edit(&mut self.draft, key, text, shift, control) {
            return PanelAction::None;
        }
        if typing && crate::text_entry::is_copy(key, text, control) {
            if !self.draft.is_empty() {
                let _ = crate::console::clipboard::copy(&self.draft);
            }
            return PanelAction::None;
        }
        let in_list = (MESSAGE_BASE..MESSAGE_BASE + MESSAGES_SHOWN as u16).contains(&self.focus);
        match key {
            // Escape closes the window's list first.
            KeyCode::Escape if self.people_list => self.people_list = false,
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(!shift),
            KeyCode::PageUp => {
                let most = self.shown.total.saturating_sub(1);
                self.scroll = (self.scroll + PAGE).min(most);
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_sub(PAGE),
            KeyCode::ArrowUp if typing => self.choose(self.shown.messages.len() as isize - 1),
            KeyCode::ArrowUp | KeyCode::ArrowDown if in_list => {
                let at = (self.focus - MESSAGE_BASE) as isize;
                self.choose(if key == KeyCode::ArrowUp {
                    at - 1
                } else {
                    at + 1
                });
            }
            KeyCode::ArrowDown if !typing => self.step(true),
            KeyCode::ArrowUp => self.step(false),
            KeyCode::Enter | KeyCode::NumpadEnter if typing => {
                // The keyboard stays in the field, sent or not. Staff's Shift+Enter
                // sends only in SJK where a mod says SJK chat in All chat.
                let action = self.activate(if shift && self.only_for_staff {
                    ONLY_TOKEN
                } else {
                    SEND_TOKEN
                });
                self.focus = FIELD_TOKEN;
                return action;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => return self.activate(self.focus),
            KeyCode::Space if !typing => return self.activate(self.focus),
            _ if typing
                && crate::sjk_chat_field::caret_key(
                    &mut self.draft,
                    &mut self.caret,
                    key,
                    control,
                ) => {}
            _ if typing => {
                crate::sjk_chat_field::key_at(
                    &mut self.draft,
                    &mut self.caret,
                    key,
                    text,
                    shift,
                    control,
                );
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        if let InputEvent::PointerWheel { delta, .. } = event {
            if self.gifs.open {
                // Over the picker, the wheel scrolls its tiles.
                if delta.y != 0.0 {
                    self.gifs.scroll(if delta.y > 0.0 { -1 } else { 1 });
                }
                return PanelAction::None;
            }
            let over_list = self.shown.list_rect.is_some_and(|rows| {
                self.pointer.is_some_and(|at| {
                    at.x >= rows.x
                        && at.x <= rows.right()
                        && at.y >= rows.y
                        && at.y <= rows.bottom()
                })
            });
            if over_list {
                // Over the window's list, the wheel scrolls its rows (the view keeps
                // the scroll within them).
                if delta.y > 0.0 {
                    self.people_scroll = self.people_scroll.saturating_sub(1);
                } else if delta.y < 0.0 {
                    self.people_scroll += 1;
                }
                return PanelAction::None;
            }
            if delta.y > 0.0 {
                let most = self.shown.total.saturating_sub(1);
                self.scroll = (self.scroll + 1).min(most);
            } else if delta.y < 0.0 {
                self.scroll = self.scroll.saturating_sub(1);
            }
            return PanelAction::None;
        }
        if let InputEvent::PointerMove(at) = event {
            self.pointer = Some(at);
            self.hover.moved();
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(token) => self.activate(token),
            None => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every button of the page answers to a token of its own: two sharing one would
    /// click each other (Trade and Lift did, 22/10/2026).
    #[test]
    fn every_token_is_its_own() {
        let mut tokens: Vec<u16> = vec![
            FIELD_TOKEN,
            SEND_TOKEN,
            MUTE_TOKEN,
            DELETE_TOKEN,
            HUB_MUTE_TOKEN,
            HUB_UNMUTE_TOKEN,
            GIF_TOKEN,
            FAVOURITE_TOKEN,
            RETRY_TOKEN,
            BAN_TOKEN,
            LIFT_TOKEN,
            CARD_TOKEN,
            CARD_MUTE_TOKEN,
            CARD_BAN_TOKEN,
            CARD_TRADE_TOKEN,
            ONLY_TOKEN,
            HIDE_GIF_TOKEN,
            TRANSLATE_TOKEN,
        ];
        tokens.extend((0..MESSAGES_SHOWN as u16).map(|index| MESSAGE_BASE + index));
        let mut sorted = tokens.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), tokens.len(), "{tokens:?}");
        assert!(tokens.iter().all(|token| *token < NAME_BASE));
    }
    /// The chosen message's Hide this GIF hides it on this PC and the same button shows
    /// it again; in the picker's Hidden tab a click on a tile only chooses it, Show this
    /// GIF again shows that one again, and its tile has no star (F does nothing there).
    #[test]
    fn a_gif_is_hidden_and_shown_again_one_by_one() {
        use crate::chat_gifs::hidden;
        use gif_picker::{STAR_BASE, TILE_BASE, Tab};
        let id = crate::chat_gifs::GifId::new("PanelHide01xyz").unwrap();
        let other = crate::chat_gifs::GifId::new("PanelHide02xyz").unwrap();
        let mut panel = Panel::new();
        panel.shown.chosen_gif = Some(id.clone());
        assert_eq!(panel.activate(HIDE_GIF_TOKEN), PanelAction::None);
        assert!(hidden::contains(&id));
        assert_eq!(panel.focus, HIDE_GIF_TOKEN, "the keyboard stays on it");
        assert_eq!(panel.activate(HIDE_GIF_TOKEN), PanelAction::None);
        assert!(!hidden::contains(&id), "shown again");
        hidden::set(&id, true);
        hidden::set(&other, true);
        panel.gifs.open = true;
        panel.gifs.choose_tab(Tab::Hidden);
        panel.gifs.ids = vec![other.clone(), id.clone()];
        let favourite = crate::chat_gifs::favourites::contains(&id);
        let _ = panel.activate_gifs(STAR_BASE + 1);
        assert_eq!(
            crate::chat_gifs::favourites::contains(&id),
            favourite,
            "no star"
        );
        let _ = panel.activate_gifs(TILE_BASE + 1);
        assert!(hidden::contains(&id), "a click only chooses it");
        assert_eq!(panel.gifs.hidden_chosen, Some(id.clone()));
        let _ = panel.activate_gifs(gif_picker::UNHIDE_TOKEN);
        assert!(!hidden::contains(&id), "Show this GIF again shows it");
        assert!(hidden::contains(&other), "the other stays hidden");
        assert_eq!(panel.gifs.hidden_chosen, None);
        hidden::set(&other, false);
    }

    use sjk_identity::ChatMessage;

    pub(super) fn message(id: u64, key: &str, text: &str) -> ChatMessage {
        ChatMessage {
            reactions: Default::default(),
            id,
            at: 1_791_300_000 + id as i64 * 60,
            key_id: key.to_owned(),
            name: format!("^{}Player {id}", id % 8),
            verified: id.is_multiple_of(3),
            tick: sjk_identity::Tick::None,
            staff: id.is_multiple_of(5),
            creator: false,
            trust: None,
            emoji: String::new(),
            client: String::new(),
            text: text.to_owned(),
            holocron: None,
            official: None,
            channel: Default::default(),
        }
    }

    pub(super) fn chat(count: u64) -> ChatState {
        ChatState {
            translations: Default::default(),
            clan: Default::default(),
            channels: Vec::new(),
            reaction: None,
            own_key: None,
            messages: (1..=count)
                .map(|id| message(id, &format!("{id:016x}"), &format!("message {id}")))
                .collect(),
            revision: 1,
            online: 9,
            people: None,
            live: true,
            outcome: None,
            ban: None,
            loaded: Some(1),
        }
    }

    pub(super) fn inputs<'a>(chat: &'a ChatState, muted: &'a [String], staff: bool) -> Inputs<'a> {
        Inputs {
            chat: Some(chat),
            muted,
            blocked: &[],
            staff,
            enabled: true,
            now: 1_791_400_000,
            staff_state: None,
            giphy_key: "",
            said_in_all: None,
            translate: None,
        }
    }

    /// The panel after one frame, so its controls are known.
    pub(super) fn drawn(panel: &mut Panel, inputs: &Inputs<'_>) {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build_with(inputs, &fonts.font, [1920.0, 1080.0]);
    }

    fn press(panel: &mut Panel, key: KeyCode, text: Option<&str>) -> PanelAction {
        panel.edit(key, text, false, false)
    }

    #[test]
    fn the_page_lists_messages_newest_at_the_bottom_and_scrolls() {
        let chat = chat(60);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        let ids: Vec<u64> = panel.shown.messages.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.last(), Some(&60), "the newest at the bottom");
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(ids.len() < 60, "a page of them");
        // Up from the field chooses the newest; Page Up scrolls older ones in.
        press(&mut panel, KeyCode::ArrowUp, None);
        assert_eq!(panel.selected, Some(60));
        press(&mut panel, KeyCode::PageUp, None);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert_eq!(panel.shown.messages.last().map(|(id, _)| *id), Some(55));
        press(&mut panel, KeyCode::PageDown, None);
        assert_eq!(panel.scroll, 0);
    }

    #[test]
    fn up_and_down_move_through_the_messages() {
        let chat = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        press(&mut panel, KeyCode::ArrowUp, None);
        press(&mut panel, KeyCode::ArrowUp, None);
        assert_eq!(panel.selected, Some(2));
        press(&mut panel, KeyCode::ArrowDown, None);
        assert_eq!(panel.selected, Some(3));
        // Down from the newest goes back to the field.
        press(&mut panel, KeyCode::ArrowDown, None);
        assert_eq!(panel.focus, FIELD_TOKEN);
    }

    #[test]
    fn send_only_in_sjk_is_offered_only_where_sjk_chat_is_said_in_all_chat() {
        const WORDS: crate::chat::server_copy::SaidInAll = crate::chat::server_copy::SaidInAll {
            note: "note",
            only: "Send only in SJK",
            only_title: "Only in SJK chat",
            only_why: &["why"],
        };
        let chat = chat(1);
        let type_in = |panel: &mut Panel| {
            for c in "hi".chars() {
                press(panel, KeyCode::KeyA, Some(&c.to_string()));
            }
        };
        // Not offered: the button is not there and Shift+Enter is a plain Send.
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], true));
        type_in(&mut panel);
        assert_eq!(panel.activate(ONLY_TOKEN), PanelAction::None);
        // The next frame puts the keyboard back in the field.
        drawn(&mut panel, &inputs(&chat, &[], true));
        assert_eq!(
            panel.edit(KeyCode::Enter, Some("\r"), true, false),
            PanelAction::Send("hi".into())
        );
        // Offered to everyone: the button sends only in SJK.
        let offered = Inputs {
            said_in_all: Some(WORDS),
            ..inputs(&chat, &[], false)
        };
        drawn(&mut panel, &offered);
        type_in(&mut panel);
        assert_eq!(
            panel.activate(ONLY_TOKEN),
            PanelAction::SendSjkOnly("hi".into())
        );
        // Shift+Enter is staff's only.
        type_in(&mut panel);
        assert_eq!(
            panel.edit(KeyCode::Enter, Some("\r"), true, false),
            PanelAction::Send("hi".into())
        );
        let staff = Inputs {
            said_in_all: Some(WORDS),
            ..inputs(&chat, &[], true)
        };
        drawn(&mut panel, &staff);
        type_in(&mut panel);
        assert_eq!(
            panel.edit(KeyCode::Enter, Some("\r"), true, false),
            PanelAction::SendSjkOnly("hi".into())
        );
    }

    #[test]
    fn translate_on_the_chosen_message_shows_the_english_under_it() {
        let mut chat = chat(2);
        chat.messages[0].text = "Salut, est-ce que tu viens au duel ce soir avec nous ?".into();
        chat.messages[1].text = "Hey, are you coming to the duel tonight with us?".into();
        let skip = crate::chat::translate::Skip::parse("en");
        let offered = |chat: &ChatState, panel: &mut Panel, id: u64| {
            panel.selected = Some(id);
            let on = Inputs {
                translate: Some(&skip),
                ..inputs(chat, &[], false)
            };
            drawn(panel, &on);
            panel.order.contains(&TRANSLATE_TOKEN)
        };
        let mut panel = Panel::new();
        panel.open(true);
        // Not for a line surely in English, nor with cl_chatTranslate off.
        assert!(!offered(&chat, &mut panel, 2));
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(!panel.order.contains(&TRANSLATE_TOKEN));
        assert!(offered(&chat, &mut panel, 1));
        assert_eq!(
            panel.activate(TRANSLATE_TOKEN),
            PanelAction::Translate {
                message: 1,
                text: chat.messages[0].text.clone(),
            }
        );
        panel.translation_sent(1, 5);
        assert!(offered(&chat, &mut panel, 1));
        assert!(panel.ui.text_runs().any(|run| run == "Translating..."));
        chat.translations.push_back(sjk_identity::TranslateOutcome {
            serial: 5,
            text: chat.messages[0].text.clone(),
            result: Ok(sjk_identity::Translation {
                text: "Hi, are you coming to the duel tonight?".into(),
                from: "fr".into(),
                cached: false,
                left: Some(49),
            }),
        });
        assert!(offered(&chat, &mut panel, 1));
        let runs: Vec<String> = panel.ui.text_runs().map(str::to_owned).collect();
        assert!(
            runs.iter()
                .any(|run| run == "\u{bb} (fr) Hi, are you coming to the duel tonight?"),
            "{runs:?}"
        );
    }

    #[test]
    fn send_takes_the_draft_and_clears_it() {
        let chat = chat(1);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        for c in "gg wp".chars() {
            let text = c.to_string();
            assert_eq!(
                press(&mut panel, KeyCode::KeyA, Some(&text)),
                PanelAction::None
            );
        }
        // Keys that would act elsewhere type here.
        press(&mut panel, KeyCode::KeyS, Some("s"));
        press(&mut panel, KeyCode::Backspace, Some("\u{8}"));
        assert_eq!(
            press(&mut panel, KeyCode::Enter, Some("\r")),
            PanelAction::Send("gg wp".into())
        );
        assert!(panel.draft.is_empty());
        assert_eq!(
            press(&mut panel, KeyCode::Enter, Some("\r")),
            PanelAction::None
        );
        // The draft holds at most what the hub takes.
        for _ in 0..DRAFT_MAX + 50 {
            press(&mut panel, KeyCode::KeyA, Some("a"));
        }
        assert_eq!(panel.draft.chars().count(), DRAFT_MAX);
        assert_eq!(press(&mut panel, KeyCode::Escape, None), PanelAction::Close);
    }

    #[test]
    fn a_chosen_message_can_be_muted_on_this_pc() {
        let chat = chat(2);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(!panel.order.contains(&MUTE_TOKEN), "nothing chosen yet");
        let _ = panel.activate(MESSAGE_BASE + 1);
        assert_eq!(panel.selected, Some(2));
        drawn(&mut panel, &inputs(&chat, &[], false));
        let key = format!("{:016x}", 2);
        let mute = |muted| PanelAction::Mute {
            key_id: key.clone(),
            name: "^2Player 2".to_owned(),
            muted,
        };
        assert_eq!(panel.activate(MUTE_TOKEN), mute(true));
        let muted = [key.clone()];
        drawn(&mut panel, &inputs(&chat, &muted, false));
        assert_eq!(panel.activate(MUTE_TOKEN), mute(false));
    }

    #[test]
    fn the_blocked_players_are_listed_with_unblock_while_nothing_is_chosen() {
        let chat = chat(2);
        let mut panel = Panel::new();
        panel.open(true);
        let blocked: Vec<crate::chat_mutes::Mute> = (0..9)
            .map(|index| crate::chat_mutes::Mute {
                key_id: (index == 0).then(|| "0123456789abcdef".to_owned()),
                name: format!("^1Blocked {index}"),
            })
            .collect();
        let with = Inputs {
            blocked: &blocked,
            ..inputs(&chat, &[], false)
        };
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1024.0, 768.0]] {
            panel.build_with(&with, &fonts.font, viewport);
            let runs: Vec<String> = panel.ui.text_runs().map(str::to_owned).collect();
            assert!(
                runs.iter().any(|run| run == "Blocked on this PC (9)"),
                "{runs:?}"
            );
            assert!(
                runs.iter()
                    .any(|run| run.contains("more: block in the console")),
                "{runs:?}"
            );
            let shown = (0..UNBLOCK_SHOWN as u16)
                .filter(|index| panel.order.contains(&(UNBLOCK_BASE + index)))
                .count();
            assert!(shown >= 3, "{viewport:?}: {shown}");
            let last = panel
                .ui
                .rect_for(UNBLOCK_BASE + shown as u16 - 1)
                .expect("the last Unblock");
            assert!(
                last.y + last.height <= viewport[1],
                "{viewport:?}: {last:?}"
            );
        }
        assert_eq!(
            panel.activate(UNBLOCK_BASE),
            PanelAction::Mute {
                key_id: "0123456789abcdef".to_owned(),
                name: "^1Blocked 0".to_owned(),
                muted: false,
            }
        );
        assert_eq!(
            panel.activate(UNBLOCK_BASE + 1),
            PanelAction::Mute {
                key_id: String::new(),
                name: "^1Blocked 1".to_owned(),
                muted: false,
            },
            "by name"
        );
        // A chosen message takes the place: no list.
        let _ = panel.activate(MESSAGE_BASE + 1);
        drawn(&mut panel, &with);
        assert!(!panel.order.contains(&UNBLOCK_BASE));
        assert_eq!(panel.activate(UNBLOCK_BASE), PanelAction::None);
    }

    #[test]
    fn resting_on_a_name_shows_the_sender_card_with_mute() {
        use crate::sender_card::Place;
        use sjk_ui::{InputEvent, Rect, Vec2};
        let centre = |rect: Rect| Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let chat = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(panel.card.is_none());
        // The second message's name, not its row.
        let name = panel.ui.rect_for(NAME_BASE + 1).expect("a name");
        let row = panel.ui.rect_for(MESSAGE_BASE + 1).expect("its row");
        assert!(
            name.width < row.width * 0.5,
            "the name alone: {name:?} {row:?}"
        );
        let _ = panel.handle_pointer(InputEvent::PointerMove(centre(name)));
        drawn(&mut panel, &inputs(&chat, &[], false));
        let card = panel.card.as_ref().expect("the card");
        assert_eq!(card.source, CardSource::Message(2));
        assert_eq!(card.person.name, "^2Player 2");
        assert_eq!(card.person.place, Place::Unknown, "placed outside the lock");
        // Where they are comes from outside the chat's lock, once.
        let asked = std::cell::Cell::new(0);
        let place = |key: Option<&str>, name: &str| {
            asked.set(asked.get() + 1);
            assert_eq!(
                (key, name),
                (Some(format!("{:016x}", 2).as_str()), "^2Player 2")
            );
            Place::SlotByName(5)
        };
        // Its picture's version is asked until it is known: not yet, then none.
        let pictures = std::cell::Cell::new(0);
        let picture = |answer: Option<&str>| {
            let pictures = &pictures;
            let answer = answer.map(|version| (version.to_owned(), ""));
            move |key: &str| {
                assert_eq!(key, format!("{:016x}", 2));
                pictures.set(pictures.get() + 1);
                answer
            }
        };
        panel.place_card(place, picture(None));
        panel.place_card(place, picture(Some("")));
        panel.place_card(place, picture(Some("0123456789abcdef")));
        assert_eq!((asked.get(), pictures.get()), (1, 2));
        assert_eq!(
            panel.card.as_ref().unwrap().person.avatar.as_deref(),
            Some("")
        );
        drawn(&mut panel, &inputs(&chat, &[], false));
        let texts: Vec<&str> = panel.ui.text_runs().collect();
        for wanted in ["On this server, slot 5 (by name)", "Block"] {
            assert!(texts.contains(&wanted), "{wanted:?} in {texts:?}");
        }
        let card = panel.ui.rect_for(CARD_TOKEN).expect("the card's target");
        assert!(
            card.x >= name.right() || card.right() <= name.x,
            "beside the name"
        );
        // Its Mute asks for this sender by key and name; muted, it offers Unmute.
        let key = format!("{:016x}", 2);
        let mute = |muted| PanelAction::Mute {
            key_id: key.clone(),
            name: "^2Player 2".to_owned(),
            muted,
        };
        assert_eq!(panel.activate(CARD_MUTE_TOKEN), mute(true));
        let button = panel.ui.rect_for(CARD_MUTE_TOKEN).expect("Mute");
        let _ = panel.handle_pointer(InputEvent::PointerMove(centre(button)));
        let muted = [key.clone()];
        drawn(&mut panel, &inputs(&chat, &muted, false));
        assert!(panel.card.is_some(), "kept while the pointer is on it");
        assert!(panel.ui.text_runs().any(|text| text == "Unblock"));
        assert_eq!(panel.activate(CARD_MUTE_TOKEN), mute(false));
        // A click on the name chooses its message.
        let _ = panel.activate(NAME_BASE + 1);
        assert_eq!(panel.selected, Some(2));
        // Away from both: it waits a moment for the pointer on its way, then goes.
        let _ = panel.handle_pointer(InputEvent::PointerMove(Vec2::new(4.0, 1_070.0)));
        drawn(&mut panel, &inputs(&chat, &muted, false));
        assert!(panel.card.is_some(), "kept on the way to the card");
        if let Some(card) = panel.card.as_mut() {
            card.left = card.left.map(|at| at - CARD_GRACE);
        }
        drawn(&mut panel, &inputs(&chat, &muted, false));
        assert!(panel.card.is_none());
    }

    /// Trade on the sender card: reached from the name, it stays under the pointer and a
    /// click asks for a trade with that sender's key (Creyon, 22/10/2026: it vanished).
    #[test]
    fn trade_on_the_card_is_reached_and_clicked() {
        use sjk_ui::{InputEvent, PointerButton, Rect, Vec2};
        let centre = |rect: Rect| Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        crate::player_trades::TRADING_FOR_TEST.set(true);
        let chat = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        let name = panel.ui.rect_for(NAME_BASE + 1).expect("a name");
        let _ = panel.handle_pointer(InputEvent::PointerMove(centre(name)));
        drawn(&mut panel, &inputs(&chat, &[], false));
        drawn(&mut panel, &inputs(&chat, &[], false));
        let trade = panel
            .ui
            .rect_for(CARD_TRADE_TOKEN)
            .expect("Trade on the card");
        let block = panel.ui.rect_for(CARD_MUTE_TOKEN).expect("Block");
        assert!(
            (trade.y - block.y).abs() < 0.5,
            "beside Block: {trade:?} {block:?}"
        );
        assert!((trade.width - block.width).abs() < 0.5, "as wide as Block");
        // The pointer goes onto Trade: the card stays, and the click asks.
        let at = centre(trade);
        let _ = panel.handle_pointer(InputEvent::PointerMove(at));
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(panel.card.is_some(), "kept under the pointer");
        assert_eq!(
            panel.ui.rect_for(CARD_TRADE_TOKEN),
            Some(trade),
            "where it was"
        );
        let button = PointerButton::Primary;
        let _ = panel.handle_pointer(InputEvent::PointerPress {
            position: at,
            button,
        });
        let action = panel.handle_pointer(InputEvent::PointerRelease {
            position: at,
            button,
        });
        crate::player_trades::TRADING_FOR_TEST.set(false);
        assert_eq!(action, PanelAction::Trade(format!("{:016x}", 2)));
    }

    /// Ban 24 h shows only for a player the hub vouches for, and only on a sender it
    /// does not vouch for (neither verified nor staff).
    #[test]
    fn ban_is_offered_to_trusted_players_on_unverified_senders() {
        let chat = chat(6);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        // Message 4: neither verified nor staff.
        let _ = panel.activate(MESSAGE_BASE + 3);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(!panel.order.contains(&BAN_TOKEN));
        assert_eq!(panel.activate(BAN_TOKEN), PanelAction::None);
        crate::chat_ban::trust_for_test(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        assert!(panel.order.contains(&BAN_TOKEN));
        assert!(
            panel
                .ui
                .text_runs()
                .any(|text| text == crate::chat_ban::BAN)
        );
        assert_eq!(
            panel.activate(BAN_TOKEN),
            PanelAction::Ban(format!("{:016x}", 4))
        );
        // Message 3 is verified, message 5 staff: no ban.
        for index in [2, 4] {
            let _ = panel.activate(MESSAGE_BASE + index);
            drawn(&mut panel, &inputs(&chat, &[], false));
            assert!(!panel.order.contains(&BAN_TOKEN), "message {}", index + 1);
            assert_eq!(panel.activate(BAN_TOKEN), PanelAction::None);
        }
        crate::chat_ban::trust_for_test(false);
    }

    #[test]
    fn staff_actions_only_for_staff() {
        let chat = chat(4);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], false));
        let _ = panel.activate(MESSAGE_BASE + 3);
        drawn(&mut panel, &inputs(&chat, &[], false));
        for token in [DELETE_TOKEN, LIFT_TOKEN, HUB_MUTE_TOKEN, HUB_UNMUTE_TOKEN] {
            assert!(!panel.order.contains(&token));
            assert_eq!(panel.activate(token), PanelAction::None);
        }
        drawn(&mut panel, &inputs(&chat, &[], true));
        let key = format!("{:016x}", 4);
        assert_eq!(
            panel.activate(DELETE_TOKEN),
            PanelAction::Staff(StaffRequest::ChatDelete { id: 4 })
        );
        assert_eq!(
            panel.activate(LIFT_TOKEN),
            PanelAction::Staff(StaffRequest::ChatUnban {
                key_id: key.clone()
            })
        );
        assert_eq!(
            panel.activate(HUB_MUTE_TOKEN),
            PanelAction::Staff(StaffRequest::ChatMute {
                key_id: key.clone(),
                muted: true
            })
        );
        assert_eq!(
            panel.activate(HUB_UNMUTE_TOKEN),
            PanelAction::Staff(StaffRequest::ChatMute {
                key_id: key,
                muted: false
            })
        );
    }

    #[test]
    fn tab_walks_every_control() {
        let chat = chat(3);
        let mut panel = Panel::new();
        panel.open(true);
        drawn(&mut panel, &inputs(&chat, &[], true));
        let _ = panel.activate(MESSAGE_BASE);
        drawn(&mut panel, &inputs(&chat, &[], true));
        for token in [
            FIELD_TOKEN,
            SEND_TOKEN,
            MESSAGE_BASE,
            MUTE_TOKEN,
            DELETE_TOKEN,
        ] {
            assert!(panel.order.contains(&token), "{token}");
        }
        let start = panel.focus;
        for _ in 0..panel.order.len() {
            panel.edit(KeyCode::Tab, Some("\t"), false, false);
        }
        assert_eq!(panel.focus, start, "Tab comes round");
        panel.edit(KeyCode::Tab, Some("\t"), true, false);
        assert_ne!(panel.focus, start);
    }
}
