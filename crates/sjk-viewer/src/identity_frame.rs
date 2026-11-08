//! The per-frame hook of player identity (`player_identity.rs`): twice a second,
//! hand the service the player's settings and where they are playing, the new
//! medal pop-up the medals of the player's own profile (`medal_popup.rs`), the
//! achievements their counts (`achievements_frame.rs`), the picture cache where
//! pictures come from (`avatars.rs`) and the profile card what it shows
//! (`profile_card.rs`), and the hub whether the player is actively playing
//! (`holocrons_frame.rs`), and the holocron odds the hub's (`loot_odds.rs`). Each frame it also lets the chat feed follow the SJK chat
//! (`sjk_chat_frame.rs`), keeps crash reports up to date and sent
//! (`crash_report.rs`), tells Discord what the player is doing
//! (`discord_presence.rs`), starts and ends emotes (`emotes_frame.rs`) and the away emote
//! (`away_emote.rs`), shows the pings
//! relayed for this server (`world_ping_frame.rs`), keeps the
//! players' looks (`looks_frame.rs`), follows the players muted on this PC
//! (`muted_players_frame.rs`) and what SJK knows of each slot's player for the game
//! chat (`chat/marks.rs`), notes the Profile screen's tab on show
//! (`profile_hub.rs`), hands the hub the player models the game has
//! (`installed_models.rs`), tells the hub's service when the game menu opens, so its
//! pages read the hub again, puts the pictures that finished loading into the UI's atlas and
//! gives the UI the frames of the SJK chat's GIFs on screen (`chat_gifs.rs`), and
//! carries the encrypted whispers (`e2e_whispers.rs`) and runs the content
//! collections' automatic updates when due (`console_jof_content.rs`).

use super::*;
use sjk_identity::Settings;

impl GpuState {
    /// Tell the identity service about the settings and the live session. Does
    /// nothing between its twice-a-second turns.
    pub(crate) fn update_identity(&mut self) {
        player_identity::follow_game_menu(self.game_menu);
        self.remember_profile_hub_tab();
        self.update_sjk_chat();
        self.update_emotes();
        self.update_away_emote();
        self.update_world_pings();
        self.update_muted_players();
        self.update_chat_marks();
        avatars::service(&self.ui_shapes, &self.queue);
        if let Some(console) = self.console.as_ref() {
            chat_gifs::set_enabled(console.bool_cvar(chat_gifs::CVAR) != Some(false));
            chat_gifs::set_video(console.bool_cvar(chat_gifs::VIDEO_CVAR) == Some(true));
            chat_gifs::favourites::load(console.config_directory());
            chat_gifs::hidden::load(console.config_directory());
            crate::shader_marks::load(console.config_directory());
            chat_gifs::sources::load(console.config_directory());
            crate::jof_tag::set_shown(console.bool_cvar(crate::jof_tag::CVAR) != Some(false));
            crate::chat::client_icons::set_shown(
                console.bool_cvar(crate::chat::client_icons::CVAR) != Some(false),
            );
            crate::terms::set_renamed(
                console.bool_cvar(&crate::mods::switch("jof")) == Some(true)
                    && console.bool_cvar(crate::terms::CVAR) == Some(true),
            );
        }
        chat_gifs::service(&mut self.ui_shapes, &self.device, &self.queue);
        // The own look is read twice a second, and at once when the worn skins change (the
        // Collection's Equip), so the preview never shows the side before it.
        let skins_changed = self.console.as_ref().is_some_and(|console| {
            let text = |name| console.text_cvar(name).unwrap_or_default();
            let (first, second) = (
                text(crate::unlockables::SABER_SKIN_CVAR),
                text(crate::looks::SECOND_CVAR),
            );
            self.looks.own_settings_changed(first, second)
        });
        // A tag just stuck or taken off goes at once too.
        let due = player_identity::due() || skins_changed | self.tags.take_changed();
        // The shaders seen on this PC follow what the player holds (New marks), on the
        // identity's turns only.
        if due {
            let _ = player_identity::with_snapshot(crate::shader_marks::follow);
        }
        self.update_looks(due);
        self.update_tags();
        // The hub's toy events for this server, taken once for every toy.
        let toy_events = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(|session| player_identity::take_toy_events(Some(session.server())))
            .unwrap_or_default();
        // The relay's, the same: on a game server the relay is read all the while, for
        // the falls of the Ragdoll anyone may show.
        let server = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(ClientSession::server);
        player_identity::set_relay_listen(server.is_some());
        let relay_events = server
            .map(|server| player_identity::take_relay_events(Some(server)))
            .unwrap_or_default();
        self.update_cardboard_box(Instant::now(), &toy_events);
        self.update_order66(&toy_events);
        self.update_hax(&toy_events, &relay_events);
        self.update_ragdoll(&relay_events);
        self.update_monitor(&relay_events, &toy_events);
        self.update_banana(&relay_events);
        self.update_propgun(&relay_events);
        self.update_drone(&relay_events);
        self.update_boombox(&relay_events);
        self.update_mystery_boxes(&toy_events, &relay_events);
        self.update_wheel_of_doom(&toy_events);
        self.update_quests(&toy_events);
        self.update_toy_master(&toy_events);
        self.update_e2e_whispers();
        self.update_player_trades();
        #[cfg(feature = "mod-jof")]
        self.update_content_auto();
        if !due {
            return;
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let settings = Settings {
            enabled: console.bool_cvar("cl_identity") == Some(true),
            hub_url: console
                .text_cvar("cl_hubUrl")
                .unwrap_or_default()
                .trim()
                .to_owned(),
        };
        let name = console
            .text_cvar("name")
            .unwrap_or_default()
            .trim()
            .to_owned();
        let location = self.live_session.as_ref().and_then(|session| {
            player_identity::location(
                session.server(),
                session.is_local(),
                session.game_state(),
                &name,
            )
        });
        let chat = console.sjk_chat_on();
        // Pictures are read from the hub the identity talks to, and only while it is on.
        let pictures_from = if settings.enabled {
            settings.hub_url.as_str()
        } else {
            ""
        };
        avatars::configure(Some(console.config_directory()), pictures_from);
        crate::news_images::configure(Some(console.config_directory()), pictures_from);
        player_identity::apply(console.config_directory(), settings, name, location, chat);
        self.offer_installed_models();
        let (packs_revision, assets_note) = player_identity::packs();
        crate::sjk_packs::follow_identity(packs_revision, assets_note);
        profile_card::refresh(console);
        self.offer_medals();
        self.offer_holocrons();
        self.follow_official();
        let _ = player_identity::with_snapshot(|snapshot| {
            log_look_outcome(snapshot.look_outcome.as_ref());
            crate::loot_odds::follow(&snapshot.loot);
            crate::economy::follow(snapshot.economy.as_ref());
            crate::kyber::follow(snapshot.kyber.as_ref());
            if let Some(me) = &snapshot.me {
                crate::kyber::follow_rewards(&me.achievements);
            }
        });
        self.update_achievements();
        self.update_holocron_activity();
        self.follow_crash_reports();
        self.follow_discord_presence();
    }

    /// Keep what the game chat shows of each slot's player (SJK's emblem, the verified
    /// tick, the name as worn) in step with the hub's claims (`chat/marks.rs`).
    fn update_chat_marks(&mut self) {
        let live = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local());
        let own = live.and_then(|session| usize::try_from(session.game_state().client_num).ok());
        let worn = self
            .console
            .as_ref()
            .and_then(|console| console.text_cvar("name").ok())
            .unwrap_or_default()
            .trim();
        self.chat
            .follow_marks(player_identity::revision(), worn, own, live.is_some());
    }

    /// Hand the identity the player models the game has and the one the player wears,
    /// for the player card's model line (`installed_models.rs`).
    fn offer_installed_models(&self) {
        if let Some(vfs) = self.vfs.as_ref() {
            installed_models::follow(vfs);
        }
        let (Some(models), Some(console)) = (installed_models::models(), self.console.as_ref())
        else {
            return;
        };
        let share = console.bool_cvar(installed_models::CVAR) != Some(false);
        player_identity::offer_models(&models, share);
        // What the server published for the player wins over the setting (a forced
        // team model, a mod's own).
        let published = self.live_session.as_ref().and_then(|session| {
            let game = session.game_state();
            let slot = usize::try_from(game.client_num).ok()?;
            let info = game.config_string(player_identity::CS_PLAYERS + slot)?;
            sjk_client::LegacyClientInfo::new(info)
                .text("model")
                .map(str::to_owned)
        });
        let model = published.or_else(|| console.text_cvar("model").ok().map(str::to_owned));
        let own = model
            .as_deref()
            .and_then(installed_models::folder)
            .filter(|folder| !models.retail.contains(folder));
        player_identity::set_own_model(own);
    }
}

/// Log each look the hub was sent (it carries the toys others see), once: how long it
/// waited after the change and whether the hub took it, for slow toy sync reports.
fn log_look_outcome(outcome: Option<&sjk_identity::ReportOutcome>) {
    static LOGGED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let Some(outcome) = outcome else { return };
    if LOGGED.swap(outcome.serial, std::sync::atomic::Ordering::AcqRel) == outcome.serial {
        return;
    }
    let verdict = if outcome.sent { "sent" } else { "refused" };
    crate::log::progress(format_args!("SJK look {verdict}: {}", outcome.message));
}
