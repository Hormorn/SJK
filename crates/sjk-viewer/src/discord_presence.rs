//! Discord Rich Presence: what the player is doing in SJK, on their Discord profile.
//!
//! Twice a second (`identity_frame.rs`) the client's state becomes an
//! [`Activity`]: the menu screen on show, a join, the server, map, mode, team and
//! player count in game, a demo or a map explored alone. [`discord_ipc`] hands it
//! to the Discord desktop app, which shows it; nothing is sent when Discord is
//! not running. `cl_discordPresence` turns it off (0) or leaves the server's name
//! out (2); a server with a password never shows its name. The server's address
//! is never shared.
//!
//! [`discord_ipc`]: crate::discord_ipc

use crate::client_state::ClientPhase;
use crate::discord_ipc::{Activity, ActivityButton, DiscordIpc};
use crate::profile_hub::Tab;
use sjk_protocol::{GameState, InfoString};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 0 off, 1 on, 2 on without the server's name.
pub(crate) const CVAR: &str = "cl_discordPresence";

/// The "Sol JK" application in Discord's developer portal: its name is the game
/// name Discord shows. Not a secret; every Rich Presence client carries its id.
const APPLICATION_ID: &str = "1561491096062922803";

/// The SJK logo on the project site, the activity's picture.
const LOGO: &str = "https://sol-vulpes.github.io/SJK/assets/sjk-logo-512.png";
const SITE: &str = "https://sol-vulpes.github.io/SJK/";

/// `CS_PLAYERS`, the first player config string.
const CS_PLAYERS: usize = 1_131;

struct State {
    ipc: Option<DiscordIpc>,
    /// What the elapsed timer counts for, and since when (unix seconds).
    since: Option<(String, u64)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    ipc: None,
    since: None,
});

/// Where the player is, as far as their presence goes.
#[derive(Debug, PartialEq, Eq)]
enum Place {
    Menus(Option<&'static str>),
    Joining,
    Loading(String),
    Exploring(String),
    Demo(String),
    Server(ServerView),
}

#[derive(Debug, PartialEq, Eq)]
struct ServerView {
    /// The server's name, colours stripped; empty when it is not shown.
    name: String,
    map: String,
    mode: &'static str,
    team: Team,
    /// Red and blue scores in a team game.
    scores: Option<[i32; 2]>,
    players: u32,
    slots: u32,
    /// The game runs on this PC (a local or hosted game).
    local: bool,
    /// The server's address: only told apart for the timer, never shown.
    address: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Team {
    Free,
    Red,
    Blue,
    Spectator,
}

impl crate::GpuState {
    /// Hand Discord what the player is doing now. Twice a second.
    pub(crate) fn follow_discord_presence(&self) {
        let mode = self
            .console
            .as_ref()
            .and_then(|console| console.integer_cvar(CVAR))
            .unwrap_or(1);
        let place = (mode != 0 && !APPLICATION_ID.is_empty()).then(|| self.presence_place(mode));
        let Ok(mut state) = STATE.lock() else { return };
        let Some(place) = place else {
            // Off: clear what Discord shows and let go of the pipe.
            state.ipc = None;
            state.since = None;
            return;
        };
        if state.ipc.is_none() {
            eprintln!("Discord status: on (cl_discordPresence {mode})");
            state.ipc = DiscordIpc::start(APPLICATION_ID);
        }
        let key = timer_key(&place);
        if state.since.as_ref().is_none_or(|(held, _)| *held != key) {
            state.since = Some((key, unix_now()));
        }
        let start = state.since.as_ref().map(|(_, at)| *at);
        let activity = activity(&place, start);
        if let Some(ipc) = state.ipc.as_mut() {
            ipc.set(Some(activity));
        }
    }

    fn presence_place(&self, mode: i64) -> Place {
        if let Some(demo) = self.demo_session.as_ref() {
            return Place::Demo(map_of(demo.game_state()));
        }
        if self.resident.exploring() {
            return Place::Exploring(short_map(&self.resident.map).to_owned());
        }
        let menu = self.client_menu.as_ref();
        if self.join_task.is_some() || menu.is_some_and(|menu| menu.is_connecting()) {
            let map = menu.map(|menu| short_map(menu.loading_map())).unwrap_or("");
            return if map.is_empty() {
                Place::Joining
            } else {
                Place::Loading(map.to_owned())
            };
        }
        if let Some(session) = self
            .live_session
            .as_ref()
            .filter(|_| self.live_map_installed)
        {
            let game = session.game_state();
            let info = server_info(game);
            let get = |key: &str| info.as_ref().and_then(|info| info.get(key)).unwrap_or("");
            let number = |key: &str| info.as_ref().and_then(|info| info.get_i32(key));
            let gametype = number("g_gametype");
            let password = number("g_needpass").is_some_and(|value| value != 0);
            let name = if mode == 2 || password {
                String::new()
            } else {
                plain(get("sv_hostname"))
            };
            let player = &session.latest_snapshot().player;
            let team = match player.team() {
                _ if player.is_spectator() => Team::Spectator,
                1 => Team::Red,
                2 => Team::Blue,
                3 => Team::Spectator,
                _ => Team::Free,
            };
            let team_game = gametype.is_some_and(|gametype| gametype >= 6);
            return Place::Server(ServerView {
                name,
                map: map_of(game),
                mode: crate::server_browser::gametype_name(gametype),
                team,
                scores: team_game.then(|| session.team_scores()),
                players: count_players(game),
                slots: number("sv_maxclients")
                    .and_then(|slots| u32::try_from(slots).ok())
                    .unwrap_or(0),
                local: session.is_local(),
                address: session.server().to_string(),
            });
        }
        Place::Menus(self.presence_screen())
    }

    /// The menu screen on show, in a few words.
    fn presence_screen(&self) -> Option<&'static str> {
        if let Some(tab) = self.profile_hub_tab() {
            return Some(tab_words(tab));
        }
        match self.client_menu.as_ref()?.phase() {
            ClientPhase::Browser => Some("Browsing servers"),
            ClientPhase::Settings | ClientPhase::Keybinds => Some("In Settings"),
            ClientPhase::Player => Some("Character creator"),
            ClientPhase::CreateGame => Some("Setting up a game"),
            _ => None,
        }
    }
}

fn tab_words(tab: Tab) -> &'static str {
    match tab {
        Tab::Character => "Character creator",
        Tab::Saber => "Choosing a saber",
        Tab::Force => "Choosing Force powers",
        Tab::Profile => "On their profile",
        // Static words for Discord, so the JoF names are spelled out (`crate::terms`).
        Tab::Medals if crate::terms::renamed() => "Looking at honors",
        Tab::Medals => "Looking at medals",
        Tab::Achievements => "Looking at achievements",
        // The Saber Lab is a development tab: told as the shaders are.
        Tab::Shaders | Tab::SaberLab => "Looking at saber shaders",
        Tab::Toys => "Looking at toys",
        Tab::Tags => "Looking at tags",
        Tab::Banners => "Choosing a profile banner",
        Tab::Nameplates => "Looking at nameplates",
        Tab::Holocrons if crate::terms::renamed() => "Opening echoes",
        Tab::Holocrons => "Opening holocrons",
        Tab::Forge => "In the Forge",
        Tab::Cards => "Looking at cards",
        Tab::History => "Looking at their history",
    }
}

/// What the timer counts: the same server (or menus, or demo) keeps its start.
fn timer_key(place: &Place) -> String {
    match place {
        Place::Menus(_) => "menus".into(),
        Place::Joining | Place::Loading(_) => "joining".into(),
        Place::Exploring(map) => format!("explore {map}"),
        Place::Demo(map) => format!("demo {map}"),
        Place::Server(server) => format!("server {}", server.address),
    }
}

fn activity(place: &Place, start: Option<u64>) -> Activity {
    let mut activity = Activity {
        large_image: LOGO.into(),
        large_text: "Sol JK".into(),
        start,
        buttons: vec![ActivityButton {
            label: "Get SJK".into(),
            url: SITE.into(),
        }],
        ..Activity::default()
    };
    match place {
        Place::Menus(screen) => {
            activity.details = "In the menus".into();
            activity.state = screen.unwrap_or_default().into();
        }
        Place::Joining => activity.details = "Joining a server".into(),
        Place::Loading(map) => {
            activity.details = "Joining a server".into();
            activity.state = format!("Loading {map}");
        }
        Place::Exploring(map) => {
            activity.details = "Exploring a map".into();
            activity.state = map.clone();
        }
        Place::Demo(map) => {
            activity.details = "Watching a demo".into();
            activity.state = map.clone();
        }
        Place::Server(server) => {
            activity.details = if server.local {
                "Playing a local game".into()
            } else if server.name.is_empty() {
                "Playing online".into()
            } else {
                server.name.clone()
            };
            let mut state = format!("{} · {}", server.map, server.mode);
            match (server.team, server.scores) {
                (Team::Spectator, _) => state.push_str(" · Spectating"),
                (Team::Red, Some([red, blue])) => {
                    state.push_str(&format!(" · Red team {red}-{blue}"))
                }
                (Team::Blue, Some([red, blue])) => {
                    state.push_str(&format!(" · Blue team {blue}-{red}"))
                }
                _ => {}
            }
            activity.state = state;
            activity.party = Some((server.players, server.slots));
        }
    }
    activity
}

fn server_info(game: &GameState) -> Option<InfoString> {
    game.config_string(0)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| InfoString::parse(text).ok())
}

fn map_of(game: &GameState) -> String {
    let info = server_info(game);
    short_map(
        info.as_ref()
            .and_then(|info| info.get("mapname"))
            .unwrap_or(""),
    )
    .to_owned()
}

/// `maps/mp/ffa3.bsp` or `mp/ffa3` as players say it: `ffa3`.
fn short_map(map: &str) -> &str {
    let map = map.strip_prefix("maps/").unwrap_or(map);
    let map = map.strip_suffix(".bsp").unwrap_or(map);
    map.strip_prefix("mp/").unwrap_or(map)
}

/// Players on the server, spectators included.
fn count_players(game: &GameState) -> u32 {
    (0..32)
        .filter(|client| {
            game.config_string(CS_PLAYERS + client)
                .is_some_and(|bytes| !bytes.is_empty())
        })
        .count() as u32
}

/// A server name without its `^` colour codes.
fn plain(text: &str) -> String {
    let stripped = sjk_game_jka::client_view::strip_colours(text.as_bytes());
    String::from_utf8_lossy(&stripped).trim().to_owned()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, team: Team, scores: Option<[i32; 2]>) -> Place {
        Place::Server(ServerView {
            name: name.into(),
            map: "ffa3".into(),
            mode: "CTF",
            team,
            scores,
            players: 14,
            slots: 32,
            local: false,
            address: "127.0.0.1:29070".into(),
        })
    }

    #[test]
    fn maps_read_as_players_say_them() {
        assert_eq!(short_map("maps/mp/ffa3.bsp"), "ffa3");
        assert_eq!(short_map("mp/duel6"), "duel6");
        assert_eq!(short_map("t3_bounty"), "t3_bounty");
    }

    #[test]
    fn a_server_shows_its_name_map_mode_team_and_players() {
        let value = activity(&server("JoF Duel", Team::Blue, Some([3, 5])), Some(1)).to_json();
        assert_eq!(value["details"], "JoF Duel");
        assert_eq!(value["state"], "ffa3 · CTF · Blue team 5-3");
        assert_eq!(value["party"]["size"], serde_json::json!([14, 32]));
        assert_eq!(value["buttons"][0]["label"], "Get SJK");
    }

    #[test]
    fn a_hidden_name_says_online_and_a_spectator_says_so() {
        let value = activity(&server("", Team::Spectator, Some([0, 0])), None).to_json();
        assert_eq!(value["details"], "Playing online");
        assert_eq!(value["state"], "ffa3 · CTF · Spectating");
    }

    #[test]
    fn the_timer_keeps_running_on_the_same_server() {
        let a = server("A", Team::Red, Some([1, 2]));
        let b = server("A", Team::Blue, Some([4, 2]));
        assert_eq!(timer_key(&a), timer_key(&b));
        assert_ne!(timer_key(&a), timer_key(&Place::Menus(None)));
    }
}
