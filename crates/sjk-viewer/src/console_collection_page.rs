//! Console side of the Collection page (see `collection_panel.rs`): opening it on a
//! tab, setting `cg_saberSkin` when it equips or unequips a shader and asking the viewer
//! to light the Illuminate holocron or put it out (the lit state is the viewer's,
//! mirrored here for the page: [`ViewerConsole::set_illuminate_lit`]), and routing keys
//! and pointer events to it, as for the Staff page. It is drawn in the SJK UI's look
//! whatever the menu style (`console_sjk_pages.rs`).

use super::collection_panel::{Backstage, Inputs, PanelAction, StageWish};
use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;
use crate::unlockables::{Holdings, SABER_SKIN_CVAR};
use sjk_ui::InputEvent;

/// The `collection` command.
pub(crate) const COLLECTION_COMMAND: &str = "collection";
pub(crate) const COLLECTION_HELP: &str =
    "Open your SJK collection: medals, achievements, shaders, toys and nameplates";
/// The `achievements` command.
pub(crate) const ACHIEVEMENTS_COMMAND: &str = "achievements";
pub(crate) const ACHIEVEMENTS_HELP: &str = "Open the Collection on your achievements";
/// The `unlockables` command.
pub(crate) const UNLOCKABLES_COMMAND: &str = "unlockables";
pub(crate) const UNLOCKABLES_HELP: &str =
    "Open the Collection on its shaders: saber looks you own or can unlock";

impl ViewerConsole {
    /// `collection`, `achievements` or `unlockables` in the classic menus: show the page
    /// on its own on `tab` (`None`: the tab shown last), or close it when it shows it.
    pub(crate) fn toggle_collection_panel(&mut self, tab: Option<Tab>) {
        let tab = tab.unwrap_or(self.collection_panel.tab());
        if self.open && self.collection_panel.is_open() && self.collection_panel.tab() == tab {
            self.close_collection_panel();
            return;
        }
        let owns_console = !self.open;
        self.show_collection_panel(tab, owns_console, false, ReturnTarget::MainMenu);
    }

    /// The Saber Lab started: show the Shaders tab on its own (the classic menus; the SJK
    /// UI's Collection screen is opened by `GpuState`) unless `hub` says it opened that
    /// screen, and choose the lab blade.
    pub(crate) fn show_saber_lab(&mut self, hub: bool) {
        let shown = self.open && self.collection_panel.is_open();
        if !hub && !(shown && self.collection_panel.tab() == Tab::Shaders) {
            let owns_console = !self.open;
            self.show_collection_panel(Tab::Shaders, owns_console, false, ReturnTarget::MainMenu);
        }
        self.collection_panel.choose_saber_lab();
    }

    /// Show the page on its own on the tab shown last (the classic menus).
    pub(crate) fn open_collection_panel(&mut self) {
        let tab = self.collection_panel.tab();
        let owns_console = !self.open;
        self.show_collection_panel(tab, owns_console, false, ReturnTarget::MainMenu);
    }

    /// The Collection's tab on show, or shown last (Medals the first time).
    pub(crate) fn collection_tab(&self) -> Tab {
        if self.holocrons_last {
            Tab::Holocrons
        } else {
            self.collection_panel.tab()
        }
    }

    /// Show the page on `tab`, as the SJK UI's Collection screen returning to `back`
    /// (`hub`) or on its own; closing it closes the console too when `owns_console`.
    pub(super) fn show_collection_panel(
        &mut self,
        tab: Tab,
        owns_console: bool,
        hub: bool,
        back: ReturnTarget,
    ) {
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.debug_panel.close();
        self.changelog.close();
        self.update_panel.close();
        self.config_import.close();
        self.credits.close();
        self.identity_panel.close();
        self.profile_panel.close();
        self.staff_panel.close();
        self.sjk_chat_panel.close();
        self.holocrons_panel.close();
        self.dead_key.settle();
        self.collection_panel.open(tab, owns_console, hub, back);
        let store = super::collection_panel::loadouts::Store::load(self.config_directory());
        self.collection_panel.set_loadouts(&store, Some(""));
    }

    fn close_collection_panel(&mut self) {
        if self.collection_panel.close() {
            self.set_open(false);
        }
    }

    fn collection_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_collection_panel(),
            PanelAction::Wear(id) => crate::unlockables::wear(self, id),
            PanelAction::WearFinisher(id) => {
                let _ = self.set_cvar(crate::finishers::CVAR, id);
            }
            PanelAction::WearSecond(id) => {
                let _ = self.set_cvar(crate::looks::SECOND_CVAR, id);
            }
            PanelAction::WearBoth { first, second } => {
                let _ = self.set_cvar(crate::looks::SECOND_CVAR, second);
                crate::unlockables::wear(self, first);
            }
            PanelAction::SkinOption { skin, option, on } => {
                let setting = self
                    .text_cvar(crate::saber_skin_options::CVAR)
                    .unwrap_or_default()
                    .to_owned();
                let _ = self.set_cvar(
                    crate::saber_skin_options::CVAR,
                    &crate::saber_skin_options::with(&setting, skin, &option, on),
                );
            }
            PanelAction::Shatter(id) => {
                let _ = crate::economy::shatter(id);
            }
            PanelAction::ShatterLast(id) => {
                let _ = crate::economy::shatter_last(id);
            }
            PanelAction::ShatterAll => {
                let _ = crate::economy::shatter_all();
            }
            PanelAction::Craft(id) => {
                let _ = crate::economy::craft(id);
            }
            PanelAction::Buy(id) => {
                let _ = crate::player_identity::buy(&id);
            }
            PanelAction::BuyHolocron(tier) => {
                let _ = crate::economy::buy_holocron(&tier);
            }
            PanelAction::RevealCards => {
                let _ = crate::player_identity::reveal_cards();
            }
            PanelAction::SeeAndShare(on) => {
                self.set_cvar("cg_nameplate", if on { "1" } else { "0" });
            }
            PanelAction::NameplateStyle(style) => {
                self.set_cvar(
                    crate::hud::nameplate::STYLE_CVAR,
                    &style.value().to_string(),
                );
            }
            PanelAction::ToySight { cvar, seen } => {
                self.set_cvar(cvar, if seen { "1" } else { "0" });
            }
            PanelAction::Loadout(request) => self.loadout_request(&request),
            PanelAction::ToyOption { cvar, on } => {
                self.set_cvar(cvar, if on { "1" } else { "0" });
            }
            PanelAction::ShaderView(grid) => {
                self.set_cvar(
                    super::collection_panel::SHADER_VIEW_CVAR,
                    if grid { "0" } else { "1" },
                );
            }
            PanelAction::Bind { command, key } => self.rebind_action(command, 0, &key),
            PanelAction::Unbind(command) => self.clear_action(command, 0),
            PanelAction::Mark { id, mark, on } => {
                // The snapshot shows it at once; the hub's answer (or refusal) follows.
                let _ = crate::player_identity::with_service(|service| {
                    service.set_mark(id, mark, on);
                });
            }
            PanelAction::Illuminate(on) => {
                self.pending_illuminate = Some(on);
                // The page shows it at once; the viewer applies it next frame.
                self.illuminate_lit = on;
            }
            PanelAction::CardboardBox(on) => {
                self.pending_cardboard_box = Some(on);
                self.cardboard_box_on = on;
            }
            PanelAction::Order66(on) => {
                self.pending_order66 = Some(on);
                self.order66_on = on;
            }
            PanelAction::Hax(on) => {
                self.pending_hax = Some(on);
                self.hax_on = on;
            }
            PanelAction::Ragdoll(on) => {
                self.pending_ragdoll = Some(on);
                self.ragdoll_on = on;
            }
            PanelAction::Drone(on) => {
                self.pending_drone = Some(on);
                self.drone_on = on;
            }
            PanelAction::Boombox(on) => {
                self.pending_boombox = Some(on);
                self.boombox_on = on;
            }
            PanelAction::Fishing(on) => {
                self.pending_fishing = Some(on);
                self.fishing_on = on;
            }
            PanelAction::Mining(on) => {
                self.pending_mining = Some(on);
                self.mining_on = on;
            }
            PanelAction::SellJunk => self.pending_fishing_sell = true,
            PanelAction::SellRubble => self.pending_mining_sell = true,
            PanelAction::Monitor(on) => {
                self.pending_monitor = Some(on);
                self.monitor_on = on;
            }
            PanelAction::BoxHunt(on) => {
                self.pending_box_hunt = Some(on);
                self.box_hunt_on = on;
            }
            PanelAction::Banana(on) => {
                self.pending_banana = Some(on);
                self.banana_on = on;
            }
            PanelAction::Banner(id) => {
                let _ = crate::player_identity::with_service(|service| {
                    service.set_banner(id.to_owned())
                });
            }
            PanelAction::ChooseTag(id, finish) => {
                self.set_cvar(crate::tags::CVAR, &id);
                self.set_cvar(crate::tags::FINISH_CVAR, finish);
            }
            PanelAction::ShatterTag(copy) => crate::player_identity::shatter_tag(copy),
            PanelAction::CraftTags(copies) => crate::player_identity::craft_tags(copies),
            PanelAction::Hub(tab) => {
                // A tab of another page: the Holocrons page.
                let back = self.collection_panel.back();
                if self.collection_panel.is_hub() {
                    self.open_profile_hub_page(tab, back);
                } else if tab == Tab::Holocrons {
                    let owns_console = self.collection_panel.close();
                    self.show_holocrons_panel(owns_console);
                    self.holocrons_last = true;
                }
            }
        }
    }

    /// Whether the Illuminate holocron is lit, as the viewer last said.
    fn illuminate_lit(&self) -> bool {
        self.illuminate_lit
    }

    /// The viewer says whether the holocron is lit, once a frame, for the Toys tab.
    pub(crate) fn set_illuminate_lit(&mut self, lit: bool) {
        self.illuminate_lit = lit;
    }

    /// The Toys tab asked to light the holocron (`true`) or put it out, once.
    pub(crate) fn take_illuminate_request(&mut self) -> Option<bool> {
        self.pending_illuminate.take()
    }

    /// The viewer says whether the player is in the cardboard box, once a frame.
    pub(crate) fn set_cardboard_box_on(&mut self, on: bool) {
        self.cardboard_box_on = on;
    }

    /// The Toys tab asked to get in the cardboard box (`true`) or out, once.
    pub(crate) fn take_cardboard_box_request(&mut self) -> Option<bool> {
        self.pending_cardboard_box.take()
    }

    /// The viewer says whether the Order 66 toy is on, once a frame.
    pub(crate) fn set_order66_on(&mut self, on: bool) {
        self.order66_on = on;
    }

    /// The Toys tab asked to turn Order 66 on (`true`) or off, once.
    pub(crate) fn take_order66_request(&mut self) -> Option<bool> {
        self.pending_order66.take()
    }

    /// The viewer says whether the Hax toy is on, once a frame.
    pub(crate) fn set_hax_on(&mut self, on: bool) {
        self.hax_on = on;
    }

    /// The Toys tab asked to turn the Hax on (`true`) or off, once.
    pub(crate) fn take_hax_request(&mut self) -> Option<bool> {
        self.pending_hax.take()
    }

    /// The viewer says whether the player is limp (the Ragdoll), once a frame.
    pub(crate) fn set_ragdoll_on(&mut self, on: bool) {
        self.ragdoll_on = on;
    }

    /// The Toys tab asked to go limp (`true`) or get up, once.
    pub(crate) fn take_ragdoll_request(&mut self) -> Option<bool> {
        self.pending_ragdoll.take()
    }

    /// The viewer says whether the Drone is on, once a frame.
    pub(crate) fn set_drone_on(&mut self, on: bool) {
        self.drone_on = on;
    }

    /// The Toys tab asked to turn the Drone on (`true`) or off, once.
    pub(crate) fn take_drone_request(&mut self) -> Option<bool> {
        self.pending_drone.take()
    }

    /// The viewer says whether the Boombox is carried, once a frame.
    pub(crate) fn set_boombox_on(&mut self, on: bool) {
        self.boombox_on = on;
    }

    /// The Toys tab asked to carry the Boombox (`true`) or put it away, once.
    pub(crate) fn take_boombox_request(&mut self) -> Option<bool> {
        self.pending_boombox.take()
    }

    /// The viewer says whether the fishing rod is out, once a frame.
    pub(crate) fn set_fishing_on(&mut self, on: bool) {
        self.fishing_on = on;
    }

    /// The Toys tab asked to get the rod out (`true`) or put it away, once.
    pub(crate) fn take_fishing_request(&mut self) -> Option<bool> {
        self.pending_fishing.take()
    }

    /// The viewer says whether the pickaxe is in hand, once a frame.
    pub(crate) fn set_mining_on(&mut self, on: bool) {
        self.mining_on = on;
    }

    /// The Toys tab asked to take the pickaxe (`true`) or put it away, once.
    pub(crate) fn take_mining_request(&mut self) -> Option<bool> {
        self.pending_mining.take()
    }

    /// Whether the Fishing card is on show (the Collection open on Toys, Fishing chosen):
    /// the viewer gives it its catches then ([`Self::set_fishing_card`]).
    pub(crate) fn fishing_card_shown(&self) -> bool {
        self.collection_panel.is_open()
            && self.collection_panel.tab() == Tab::Toys
            && self.collection_panel.fishing_chosen()
    }

    /// The viewer gives the Fishing card its catches and sale.
    pub(crate) fn set_fishing_card(&mut self, card: crate::fishing_sell::Card) {
        self.fishing_card = card;
    }

    /// Whether the Mining card is on show (the Collection open on Toys, Mining chosen).
    pub(crate) fn mining_card_shown(&self) -> bool {
        self.collection_panel.is_open()
            && self.collection_panel.tab() == Tab::Toys
            && self.collection_panel.mining_chosen()
    }

    /// The viewer gives the Mining card its finds and sale.
    pub(crate) fn set_mining_card(&mut self, card: crate::fishing_sell::Card) {
        self.mining_card = card;
    }

    /// The Mining card's Sell rubble button was pressed, once.
    pub(crate) fn take_mining_sell_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_mining_sell)
    }

    /// The Fishing card's Sell junk button was pressed, once.
    pub(crate) fn take_fishing_sell_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_fishing_sell)
    }

    /// The viewer says whether the Monitor toy is on, once a frame.
    pub(crate) fn set_monitor_on(&mut self, on: bool) {
        self.monitor_on = on;
    }

    /// The Toys tab asked to turn the Monitor on (`true`) or off, once.
    pub(crate) fn take_monitor_request(&mut self) -> Option<bool> {
        self.pending_monitor.take()
    }

    /// The viewer says whether Box hunt is on, once a frame.
    pub(crate) fn set_box_hunt_on(&mut self, on: bool) {
        self.box_hunt_on = on;
    }

    /// The Toys tab asked to turn Box hunt on (`true`) or off, once.
    pub(crate) fn take_box_hunt_request(&mut self) -> Option<bool> {
        self.pending_box_hunt.take()
    }

    /// The viewer says whether the Banana peel is on, once a frame.
    pub(crate) fn set_banana_on(&mut self, on: bool) {
        self.banana_on = on;
    }

    /// The Toys tab asked to turn the Banana peel on (`true`) or off, once.
    pub(crate) fn take_banana_request(&mut self) -> Option<bool> {
        self.pending_banana.take()
    }

    /// Carry out what the Collection's Loadouts card asked, and show the slots again with
    /// what it did.
    fn loadout_request(&mut self, request: &super::collection_panel::loadouts::Request) {
        use super::collection_panel::loadouts::{Request, Store};
        let directory = self.config_directory().to_path_buf();
        let mut store = Store::load(&directory);
        let (changed, set) = store.apply(request, |cvar| {
            self.text_value(cvar).unwrap_or_default().to_owned()
        });
        if changed {
            store.save(&directory);
        }
        for (cvar, value) in &set {
            let _ = self.set_cvar(cvar, value);
        }
        let name = |slot: usize| {
            store.slots[slot]
                .as_ref()
                .map_or_else(String::new, |loadout| loadout.name.clone())
        };
        let note = match request {
            Request::Wear(slot) if set.is_empty() => format!("You already wear {}", name(*slot)),
            Request::Wear(slot) => format!("Wearing {}", name(*slot)),
            Request::Save(slot) => format!("Your look is saved as {}", name(*slot)),
            Request::Rename(slot, _) if changed => format!("Renamed {}", name(*slot)),
            Request::Rename(..) => String::new(),
            Request::Clear(slot) => format!("Slot {} is empty", slot + 1),
        };
        self.collection_panel.set_loadouts(&store, Some(&note));
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn collection_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.collection_panel.is_open() {
            return false;
        }
        let illuminate = self.illuminate_lit();
        let action = self
            .collection_panel
            .handle_key(event, self.shift, self.control, illuminate);
        self.collection_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn collection_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.collection_panel.is_open() {
            return false;
        }
        let illuminate = self.illuminate_lit();
        let action = self.collection_panel.handle_pointer(event, illuminate);
        self.collection_panel_action(action);
        true
    }

    /// What the page wants the stage to show this frame, while it shows.
    pub(crate) fn collection_stage_wish(&self) -> Option<StageWish> {
        if let Some(wish) = self.model_viewer_wish() {
            return Some(wish);
        }
        (self.open && !self.config_import.is_open())
            .then(|| self.collection_panel.stage_wish())
            .flatten()
    }

    /// Where the model stands behind the page this frame.
    pub(crate) fn set_collection_backstage(&mut self, backstage: Backstage) {
        self.model_viewer.set_backstage(backstage);
        self.collection_panel.set_backstage(backstage);
    }

    /// The style the stage's model holds its hilts in, for the Saber Lab tab.
    pub(crate) fn set_collection_lab_held(&mut self, held: crate::player_menu::SaberStyle) {
        self.collection_panel.set_lab_held(held);
    }

    /// The stock blade's colour, from `color1`, for its swatch.
    fn stock_blade(&self) -> sjk_ui::Color {
        // The whole number, as the game reads it: a black blade's 11 read as 1 (orange).
        let index = self
            .text_value("color1")
            .map(|value| sjk_client::split_color_value(value).0)
            .and_then(|colour| u8::try_from(colour).ok())
            .unwrap_or(4);
        // A custom (RGB) blade in its own tint, as the game draws it (0 reads as red), not
        // white (Neffie: an RGB blade did not show as chosen).
        let tint = self
            .integer_cvar("cp_sbRGB1")
            .and_then(|packed| u32::try_from(packed).ok())
            .unwrap_or(0);
        crate::player_menu::saber_color(index, sjk_client::unpack_saber_rgb(tint))
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_collection_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        self.collection_panel.follow_skins();
        // Copied out so the page can borrow itself mutably while it draws.
        let setting = self
            .text_cvar(SABER_SKIN_CVAR)
            .unwrap_or_default()
            .to_owned();
        let name = self.text_value("name").unwrap_or_default().to_owned();
        let tag = self
            .text_cvar(crate::tags::CVAR)
            .unwrap_or_default()
            .to_owned();
        let tag_finish = self
            .text_cvar(crate::tags::FINISH_CVAR)
            .unwrap_or_default()
            .to_owned();
        let finisher = self
            .text_cvar(crate::finishers::CVAR)
            .unwrap_or_default()
            .to_owned();
        let illuminate = self.illuminate_lit();
        let cardboard_box = self.cardboard_box_on;
        let order66 = self.order66_on;
        let hax = self.hax_on;
        let ragdoll = self.ragdoll_on;
        let drone = self.drone_on;
        let boombox = self.boombox_on;
        let monitor = self.monitor_on;
        let box_hunt = self.box_hunt_on;
        let banana = self.banana_on;
        let fishing = self.fishing_on;
        let mining = self.mining_on;
        let fishing_card = self.fishing_card.clone();
        let mining_card = self.mining_card.clone();
        let stock = self.stock_blade();
        // Tiles unless the list is chosen (`ui_shaderList 1`).
        let grid = self.integer_cvar(super::collection_panel::SHADER_VIEW_CVAR) != Some(1);
        let toy_keys = if self.collection_panel.tab() == Tab::Toys {
            self.toy_keys()
        } else {
            Vec::new()
        };
        let toys_unseen =
            super::collection_panel::toys_unseen(|cvar| crate::toy_sight::seen(Some(self), cvar));
        let toy_options =
            super::collection_panel::toy_options(|cvar| self.bool_cvar(cvar) == Some(true));
        let options = self
            .text_cvar(crate::saber_skin_options::CVAR)
            .unwrap_or_default()
            .to_owned();
        let second = self
            .text_cvar(crate::looks::SECOND_CVAR)
            .unwrap_or_default()
            .to_owned();
        // What the model holds (two sabers, a staff), or two sabers when `saber2` names a
        // second hilt and the model has not been drawn yet.
        let pair = match crate::saber_skins::local_pair() {
            crate::saber_skins::Pair::None
                if self.text_value("saber2").is_some_and(|saber2| {
                    !saber2.is_empty() && !saber2.eq_ignore_ascii_case("none")
                }) =>
            {
                crate::saber_skins::Pair::TwoSabers
            }
            pair => pair,
        };
        let enabled = self.bool_cvar("cl_identity") == Some(true);
        let share = crate::hud::nameplate::see_and_share(self);
        let nameplate_style = crate::hud::nameplate::Style::of(
            self.integer_cvar(crate::hud::nameplate::STYLE_CVAR)
                .unwrap_or(0),
        );
        let snapshot =
            crate::player_identity::collection_snapshot(crate::debug_collectibles::wanted(self));
        let held = snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.me.as_ref())
            .map(|me| me.achievements.clone())
            .unwrap_or_default();
        let standings = crate::achievements::standings(&held);
        #[cfg(test)]
        if let Some(unlocks) = self.collection_panel.preview.take() {
            let profile = self.profile_preview_snapshot();
            let standings = self.profile_preview_standings().unwrap_or(standings);
            self.follow_forge(true, profile.as_ref());
            let inputs = Inputs {
                enabled: true,
                snapshot: profile.as_ref(),
                holdings: Holdings::Known(&unlocks),
                setting: &setting,
                standings: &standings,
                illuminate,
                cardboard_box,
                order66,
                hax,
                ragdoll,
                drone,
                boombox,
                monitor,
                box_hunt,
                banana,
                fishing,
                mining,
                fishing_card: Some(&fishing_card),
                mining_card: Some(&mining_card),
                name: &name,
                stock,
                grid,
                options: &options,
                second: &second,
                pair,
                share,
                nameplate_style,
                toy_keys: &toy_keys,
                toys_unseen,
                toy_options,
                tag: &tag,
                tag_finish: &tag_finish,
                finisher: &finisher,
            };
            self.collection_panel.append_sjk(&inputs, target, viewport);
            self.collection_panel.preview = Some(unlocks);
            return;
        }
        self.follow_forge(enabled, snapshot.as_ref());
        let inputs = Inputs {
            enabled,
            snapshot: snapshot.as_ref(),
            holdings: Holdings::of(enabled, snapshot.as_ref()),
            setting: &setting,
            standings: &standings,
            illuminate,
            cardboard_box,
            order66,
            hax,
            ragdoll,
            drone,
            boombox,
            monitor,
            box_hunt,
            banana,
            fishing,
            mining,
            fishing_card: Some(&fishing_card),
            mining_card: Some(&mining_card),
            name: &name,
            stock,
            grid,
            options: &options,
            second: &second,
            pair,
            share,
            nameplate_style,
            toy_keys: &toy_keys,
            toys_unseen,
            toy_options,
            tag: &tag,
            tag_finish: &tag_finish,
            finisher: &finisher,
        };
        self.collection_panel.append_sjk(&inputs, target, viewport);
    }

    /// The keys bound to each of the toys' commands, as players read them ("F5", "F5 /
    /// K"), in the order of `collection_toys::COMMANDS`; empty for none.
    fn toy_keys(&self) -> Vec<String> {
        super::collection_panel::TOY_COMMANDS
            .iter()
            .map(|command| {
                let mut shown = String::new();
                for key in self.keys_for_command(command) {
                    if !shown.is_empty() {
                        shown.push_str(" / ");
                    }
                    sjk_shell::key_names::push_display_key(&mut shown, &key);
                }
                shown
            })
            .collect()
    }

    /// Give the Forge what the hub said this frame, read the shop when it asks, and take a
    /// shader whose last copy was just broken down off the saber when it is worn: the
    /// first saber goes back to the stock blade, and since 29/10/2026 a second saber (or the
    /// staff's other half) wearing it by its own choice too.
    fn follow_forge(&mut self, enabled: bool, snapshot: Option<&sjk_identity::Snapshot>) {
        let known = snapshot.is_some_and(|snapshot| snapshot.economy.is_some());
        self.collection_panel.set_trade_known(known);
        let data = if self.collection_panel.tab() == Tab::Forge {
            super::collection_panel::forge::ForgeData::read(enabled)
        } else {
            Default::default()
        };
        self.collection_panel.set_forge(data);
        if self
            .collection_panel
            .forge_shop_due(std::time::Instant::now())
        {
            let _ = crate::player_identity::read_shop();
        }
        // The History tab's read (31/10/2026), asked by its last frame.
        if let Some(before) = self.collection_panel.take_history_ask()
            && !crate::player_identity::read_history(before)
        {
            self.collection_panel.history_not_sent();
        }
        // A star or a padlock the hub refused (29/10/2026): said once; the hub's marks
        // come back with its next answer.
        if let Some(words) = self.collection_panel.take_mark_refusal(snapshot) {
            self.push_log(format!("^3Collection: {words}"));
        }
        if let Some(id) = self.collection_panel.take_forge_unequip() {
            if self
                .text_cvar(crate::looks::SECOND_CVAR)
                .is_ok_and(|worn| worn.trim() == id)
            {
                let _ = self.set_cvar(crate::looks::SECOND_CVAR, crate::looks::STOCK_SECOND);
            }
            if self.text_cvar(SABER_SKIN_CVAR).is_ok_and(|worn| worn == id) {
                crate::unlockables::wear(self, "");
            }
        }
    }

    /// Show the Collection on the Forge's `section`, as the Holocrons tab's Shop does.
    pub(crate) fn open_forge(
        &mut self,
        section: super::collection_panel::forge::Section,
        back: ReturnTarget,
    ) {
        self.collection_panel.show_forge(section);
        self.open_profile_hub_page(Tab::Forge, back);
    }

    /// Show `data` on the Forge in place of the live identity's (`None` lets go), for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn preview_forge(
        &mut self,
        data: Option<super::collection_panel::forge::ForgeData>,
    ) {
        self.collection_panel.preview_forge(data);
    }

    /// Show `state` on the History tab in place of the identity's (`None` lets go), for
    /// a world shot.
    #[cfg(test)]
    pub(crate) fn preview_history(
        &mut self,
        state: Option<std::sync::Arc<sjk_identity::HistoryState>>,
    ) {
        self.collection_panel.preview_history(state);
    }

    /// Open or close the History tab's row `index` and choose it, for a world shot.
    #[cfg(test)]
    pub(crate) fn history_activate_for_shot(&mut self, index: usize) {
        self.collection_panel.history_activate_for_shot(index);
    }

    /// Show the Forge's `section`, for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_section_for_shot(
        &mut self,
        section: super::collection_panel::forge::Section,
    ) {
        self.collection_panel.show_forge(section);
    }

    /// Put the Forge in the state a world shot wants (see `Panel::forge_for_shot`).
    #[cfg(test)]
    pub(crate) fn forge_for_shot(&mut self, chosen: usize, armed: bool, ask: bool, asked: bool) {
        self.collection_panel
            .forge_for_shot(chosen, armed, ask, asked);
    }

    /// Choose shader `id` in the Forge's section on its next frame, for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_want_for_shot(&mut self, id: &str) {
        self.collection_panel.forge_want_for_shot(id);
    }

    /// Type `query` in the Forge's search (and its own chip), for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_search_for_shot(&mut self, query: &str, only: bool) {
        self.collection_panel.forge_search_for_shot(query, only);
    }

    /// Light Break down's Favourites chip (or not), for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_favourites_for_shot(&mut self, on: bool) {
        self.collection_panel.forge_favourites_for_shot(on);
    }

    /// Choose shader `id` on Break down and ask before its last copy goes, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn forge_ask_last_for_shot(&mut self, id: &str) {
        self.collection_panel.forge_ask_last_for_shot(id);
    }

    /// Show `unlocks` (`None`: the live ones) whenever the page opens, its swatches drawn
    /// at `seconds`, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_collection(
        &mut self,
        unlocks: Option<Vec<sjk_identity::Unlock>>,
        seconds: f32,
    ) {
        self.collection_panel.preview = unlocks;
        self.collection_panel.shot_seconds = Some(seconds);
    }

    /// Open the Collection's Loadouts card over its page with `store`'s slots, for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn preview_loadouts(&mut self, store: &super::collection_panel::loadouts::Store) {
        self.collection_panel.open_loadouts_for_shot(store);
    }

    /// Show the Shaders tab's Finishers page, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_finishers(&mut self) {
        self.collection_panel.show_finishers();
    }

    /// Open the page on its own on `tab` with `unlocks` in place of the own profile's,
    /// its swatches drawn at `seconds`, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_collection_alone(
        &mut self,
        tab: Tab,
        unlocks: Option<Vec<sjk_identity::Unlock>>,
        seconds: f32,
    ) {
        self.toggle_collection_panel(Some(tab));
        self.preview_collection(unlocks, seconds);
    }

    /// Show `cards` and `credits` on the Cards tab in place of the own profile's, for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn preview_cards(
        &mut self,
        cards: Vec<sjk_identity::CardStack>,
        credits: u64,
        shards: u64,
    ) {
        self.collection_panel.preview_cards = Some((cards, credits, shards));
    }

    /// Choose the Tags tab's tag or card `id` with finish `finish` shown (a place in
    /// `cards::FINISHES`), the wall brought to it, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_tag_for_shot(&mut self, id: &str, finish: usize) {
        self.collection_panel.choose_tag_for_shot(id, finish);
    }

    /// Choose the Cards tab's tile `index` (in the list its filter shows), for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn collection_card_for_shot(&mut self, index: usize) {
        self.collection_panel.choose_card_for_shot(index);
    }

    /// Look at the Cards tab's chosen tile closely (or put it back), for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_inspect_for_shot(&mut self, on: bool) {
        self.collection_panel.inspect_card_for_shot(on);
    }

    /// Set the Cards tab's filter, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_card_filter_for_shot(&mut self, filter: crate::cards::binder::Filter) {
        self.collection_panel.card_filter_for_shot(filter);
    }

    /// Tilt the Cards tab's card `key` (0 up close) as though the pointer rested at `at`
    /// on it, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_tilt_for_shot(&mut self, key: usize, at: [f32; 2]) {
        self.collection_panel.tilt_card_for_shot(key, at);
    }

    /// Choose the Toys tab's toy `index` (0 Illuminate), for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_toy_for_shot(&mut self, index: usize) {
        self.collection_panel.choose_toy_for_shot(index);
    }

    /// Show the hover card of the Collection tab's thing `index`, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_hover_for_shot(&mut self, index: Option<usize>) {
        self.collection_panel.hover_for_shot(index);
    }

    /// Let the Shaders tab show its way to the Forge (`known`), for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_trade_for_shot(&mut self, known: bool) {
        self.collection_panel.trade_known_for_shot(known);
    }

    /// Show `marks` (worn, favourite, locked, New) on the Shaders tab and the Forge in
    /// place of the live ones (`None` lets go), for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_marks_for_shot(&mut self, marks: Option<crate::shader_marks::Marks>) {
        self.collection_panel.marks_for_shot(marks);
    }

    /// Hold the Shaders tab's lone saber turned so (`None` lets go), for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_saber_turn_for_shot(
        &mut self,
        turn: Option<super::collection_panel::SaberTurn>,
    ) {
        self.collection_panel.hold_saber_for_shot(turn);
    }

    /// Show the Saber Lab tab's model in `style` (0 fast to 2 strong), swinging or not, for
    /// a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_for_shot(&mut self, style: usize, swinging: bool) {
        self.collection_panel.lab_for_shot(style, swinging);
    }

    /// Look at the Saber Lab tab's model from `view`, its blade lit or not, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_view_for_shot(
        &mut self,
        view: crate::console::collection_panel::LabView,
        lit: bool,
    ) {
        self.collection_panel.lab_view_for_shot(view, lit);
    }

    /// Open the Saber Lab's editor on skin `id`, its sections open, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_edit_for_shot(&mut self, id: &'static str) {
        self.collection_panel.lab_edit_for_shot(id);
    }

    /// Show the sections the Saber Lab's edited skin may add, adding `add` when given, for
    /// a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_edit_adds_for_shot(&mut self, add: Option<&str>) {
        self.collection_panel.lab_edit_adds_for_shot(add);
    }

    /// Set the number at `path` of the skin the Saber Lab edits, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_edit_set_for_shot(&mut self, path: &str, value: f64) {
        self.collection_panel.lab_edit_set_for_shot(path, value);
    }

    /// Preview blade skin `id` on the Saber Lab tab, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_blade_for_shot(&mut self, id: &str) {
        self.collection_panel.lab_blade_for_shot(id);
    }

    /// Show the Saber Lab's floating saber's still trail or not, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_trail_for_shot(&mut self, trail: bool) {
        self.collection_panel.lab_trail_for_shot(trail);
    }

    /// Put the Saber Lab's camera behind the model or back on the shot, for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_lab_chase_for_shot(&mut self, chase: bool) {
        self.collection_panel.lab_chase_for_shot(chase);
    }

    /// Choose the Collection's shader row `row` (0 the stock blade), for a world shot.
    #[cfg(test)]
    pub(crate) fn collection_shader_for_shot(&mut self, row: usize) {
        self.collection_panel.choose_shader_for_shot(row);
    }

    /// Select a banner in a windowless UI fixture.
    #[cfg(test)]
    pub(crate) fn collection_banner_for_shot(&mut self, index: usize) {
        self.collection_panel.choose_banner_for_shot(index);
    }

    /// Whether the Profile page's, the Collection page's or the Holocrons page's last
    /// frame ran out of room on its canvas, for the world shots.
    #[cfg(test)]
    pub(crate) fn profile_pages_overflowed(&self) -> bool {
        self.profile_panel.overflowed()
            || self.collection_panel.overflowed()
            || self.holocrons_panel.overflowed()
    }

    /// The page's draw list while it is shown.
    pub(super) fn collection_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.collection_panel.is_open()).then(|| self.collection_panel.draw_list())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #230's review: Collection's stock-blade swatch read only the first digit of
    /// `color1`, so a black blade (11) showed orange (1).
    #[test]
    fn the_stock_blade_swatch_reads_the_whole_colour() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let tint = [40, 200, 90];
        let packed = sjk_client::pack_saber_rgb(tint);
        console.set_cvar("cp_sbRGB1", &packed.to_string());
        for (value, index) in [("11", 11), ("1", 1), ("6", 6), ("4", 4)] {
            console.set_cvar("color1", value);
            assert_eq!(
                console.stock_blade(),
                crate::player_menu::saber_color(index, tint),
                "{value}"
            );
        }
        // The custom blade is the player's tint, not white.
        console.set_cvar("color1", "6");
        assert_eq!(
            console.stock_blade(),
            sjk_ui::Color::new(40.0 / 255.0, 200.0 / 255.0, 90.0 / 255.0, 1.0)
        );
    }

    #[test]
    fn the_page_opens_alone_and_its_actions_set_the_cvars_and_ask_for_the_holocron() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.open_profile_panel();
        console.toggle_collection_panel(Some(Tab::Shaders));
        assert!(console.collection_panel.is_open());
        assert!(!console.profile_panel.is_open());
        assert_eq!(console.collection_tab(), Tab::Shaders);
        assert!(console.sjk_page_open(), "drawn in the SJK UI's look");
        assert!(console.covers_frame());
        assert!(console.collection_stage_wish().is_some());
        console.collection_panel_action(PanelAction::Wear("saber_sun"));
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "saber_sun");
        console.collection_panel_action(PanelAction::Wear(""));
        assert_eq!(console.text_cvar(SABER_SKIN_CVAR).unwrap(), "");
        // The holocron's state is the viewer's: the page asks, shows the answer at
        // once and the viewer takes the request once.
        console.set_illuminate_lit(false);
        assert_eq!(console.take_illuminate_request(), None);
        console.collection_panel_action(PanelAction::Illuminate(true));
        assert!(console.illuminate_lit());
        assert_eq!(console.take_illuminate_request(), Some(true));
        assert_eq!(console.take_illuminate_request(), None);
        console.collection_panel_action(PanelAction::Illuminate(false));
        assert!(!console.illuminate_lit());
        assert_eq!(console.take_illuminate_request(), Some(false));
        // It was opened with the console already open (the Profile page's).
        console.collection_panel_action(PanelAction::Close);
        assert!(!console.collection_panel.is_open());
        assert!(console.is_open());
        assert_eq!(console.collection_stage_wish(), None);
        // From a closed console: again on the tab shown last; then it closes the
        // console it took over.
        console.set_open(false);
        console.toggle_collection_panel(None);
        assert_eq!(console.collection_tab(), Tab::Shaders);
        assert!(console.is_open());
        console.toggle_collection_panel(Some(Tab::Shaders));
        assert!(!console.is_open(), "it closes the console it took over");
    }
}
