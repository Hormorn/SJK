//! Cached authoritative scoreboard projection rendered through `sjk-ui`.

mod afk;
pub(crate) use afk::CVAR as AFK_CVAR;
pub(crate) mod behavior;
pub(crate) mod chat_readers;
mod classic;
mod deaths;
mod icons;
pub(crate) mod identity_mark;
pub(crate) mod layout;
mod live_flags;
mod motion;
#[cfg(test)]
pub(crate) mod shot;
mod sjk;
pub(crate) mod style;

use crate::game_font::{GameFonts, RetailFont};
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextStyle, TextVertex, UiFont};
use sjk_client::{ClientSession, ScoreEntry};
use sjk_protocol::{GameState, InfoString};
use sjk_ui::DrawList;

const CS_PLAYERS: usize = 1_131;
/// `CS_LEVEL_START_TIME`, `CS_CLIENT_DUELISTS`, `CS_CLIENT_DUELHEALTHS`.
const CS_LEVEL_START_TIME: usize = 21;
const CS_CLIENT_DUELISTS: usize = 30;
const CS_CLIENT_DUELHEALTHS: usize = 31;
/// The board's text runs and draw commands a frame: 32 rows of up to seven
/// runs (a capture mode's, or any at intermission) and their shapes, with room for
/// three medal ribbon bars of four rectangles on every row.
const TEXT_SLOTS: usize = 320;
const DRAWS: usize = 1_536;

/// Chat remains available beside the scoreboard.
pub(crate) fn chat_visible(information: bool, history: bool) -> bool {
    information || history
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ScoreRow {
    pub(super) client_num: u8,
    pub(super) name: String,
    pub(super) team: u8,
    pub(super) score: i32,
    /// Deaths reported by a compatible server plugin, before display policy.
    pub(super) reported_deaths: Option<i32>,
    /// Optional count selected for display from the report or session observer.
    pub(super) deaths: Option<i32>,
    pub(super) ping: i32,
    pub(super) time: i32,
    /// The client has a `scores` row; clients still without one are listed
    /// as `N/A`, as `CG_TeamScoreboard` lists them.
    pub(super) has_score: bool,
    /// A bot (`skill` in its clientinfo).
    pub(super) bot: bool,
    /// Duel wins and losses (`w`, `l` in its clientinfo).
    pub(super) record: Option<(i32, i32)>,
    pub(super) powerups: u32,
    pub(super) defends: i32,
    pub(super) assists: i32,
    pub(super) captures: i32,
    /// The SJK hub knows this player under this name (`player_identity.rs`).
    pub(super) identity: Option<crate::player_identity::Tag>,
    /// The row's place in the server's `scores` order, which away rows leave.
    pub(super) order: u8,
    /// The name wears an away tag ([`afk::NAME_TAGS`]).
    pub(super) tagged: bool,
    /// Away from the keyboard ([`afk`]), with the whole minutes seen idle.
    pub(super) away: Option<u32>,
}

/// Cached scoreboard data and fixed retained presentation storage.
pub(crate) struct Scoreboard {
    cached_scores: Vec<ScoreEntry>,
    config_signature: u64,
    server_signature: u64,
    team_game: bool,
    map: String,
    mode: String,
    rows: Vec<ScoreRow>,
    ui: MenuCanvas,
    deaths: deaths::Deaths,
    afk: afk::Afk,
    hostname: String,
    max_clients: i32,
    gametype: i32,
    fraglimit: i32,
    /// The limits the SJK look words in its header.
    limits: sjk::Limits,
    style: style::ScoreboardStyle,
    motion: motion::Motion,
    icons: icons::HeadIcons,
    /// Who last killed the viewing player, named in `killer_name`.
    killer: Option<u16>,
    killer_name: String,
    /// The last siege round won that the snapshots carried (`EV_SIEGE_ROUNDOVER`) in this
    /// session, and the tracker's last one, which a new session does not take again.
    siege_round_over: Option<sjk_client::SiegeRoundOver>,
    siege_round_seen: Option<sjk_client::SiegeRoundOver>,
    /// The server ended the match ([`ServerIntermission`]), followed every frame.
    server_intermission: bool,
    intermission: ServerIntermission,
    context: behavior::Context,
    requests: behavior::Requests,
    /// The hub roster revision the rows' `identity` tags were derived from;
    /// `u64::MAX` when the rows were rebuilt and need tagging again.
    identity_revision: u64,
    /// A made-up match the world shots show without a server.
    #[cfg(test)]
    shot: Option<shot::Shot>,
}

impl Scoreboard {
    pub(crate) fn score_request_due(&mut self, time: i32) -> bool {
        self.requests.due(time)
    }

    pub(crate) fn new() -> Self {
        Self {
            cached_scores: Vec::new(),
            config_signature: 0,
            server_signature: 0,
            team_game: false,
            map: String::with_capacity(64),
            mode: String::with_capacity(32),
            rows: Vec::with_capacity(32),
            ui: MenuCanvas::with_capacities(TEXT_SLOTS, 96, DRAWS),
            deaths: deaths::Deaths::default(),
            afk: afk::Afk::default(),
            hostname: String::with_capacity(64),
            max_clients: 0,
            gametype: 0,
            fraglimit: 0,
            limits: sjk::Limits::default(),
            style: style::ScoreboardStyle::default(),
            motion: motion::Motion::default(),
            icons: icons::HeadIcons::default(),
            killer: None,
            killer_name: String::with_capacity(64),
            siege_round_over: None,
            siege_round_seen: None,
            server_intermission: false,
            intermission: ServerIntermission::default(),
            context: behavior::Context::default(),
            requests: behavior::Requests::default(),
            identity_revision: u64::MAX,
            #[cfg(test)]
            shot: None,
        }
    }

    /// Whether the scoreboard draws this frame: it fades in while `requested`
    /// and, once released, out. Nothing draws while it is not `allowed`.
    pub(crate) fn present(
        &mut self,
        console: Option<&crate::console::ViewerConsole>,
        requested: bool,
        allowed: bool,
    ) -> bool {
        self.style = style::ScoreboardStyle::from_console(console);
        self.motion
            .set_classic(self.style == style::ScoreboardStyle::Classic);
        #[cfg(test)]
        if self.shot.is_some() {
            return self.motion.present(true, true, std::time::Instant::now());
        }
        let allowed = allowed
            && (self.style != style::ScoreboardStyle::Classic
                || (!console.is_some_and(|c| c.bool_cvar("cl_paused") == Some(true))
                    && (!self.context.warmup || self.context.held)));
        let visible = self
            .motion
            .present(requested, allowed, std::time::Instant::now());
        if allowed
            && !visible
            && !self.context.dead
            && self.style == style::ScoreboardStyle::Classic
        {
            self.killer = None;
            self.killer_name.clear();
        }
        visible
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        self.ui.draw_list()
    }

    #[allow(clippy::too_many_arguments)]
    fn append(
        &mut self,
        session: &ClientSession,
        fonts: &mut GameFonts,
        options: Option<&crate::console::ViewerConsole>,
        flags: classic::FlagIcons,
        fps: Option<u32>,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
        _scale: f32,
    ) {
        self.refresh(session);
        for row in &mut self.rows {
            row.deaths = self.deaths.count(row.client_num, row.reported_deaths);
        }
        self.mark_away(afk::enabled(options), style::deaths_rank(options));
        live_flags::apply(
            &mut self.rows,
            session.latest_snapshot(),
            session.team_info().entries(),
            session.game_state(),
        );
        // Tags follow the hub's roster and the SJK chat's readers, not the frame:
        // derived again only when one of them or the rows changed.
        let revision = chat_readers::revision();
        if revision != self.identity_revision {
            self.identity_revision = revision;
            for row in &mut self.rows {
                row.identity = crate::player_identity::tag(row.client_num, &row.name);
            }
            chat_readers::mark(&mut self.rows);
        }
        let snapshot = session.latest_snapshot();
        let player = &snapshot.player;
        let local = player.client_num();
        let local_status = classic::LocalStatus {
            client: local,
            team: player.persistent[PERS_TEAM] as u8,
            rank: player.persistent[PERS_RANK],
            score: player.persistent[PERS_SCORE] as i32,
            ready: player.stats[STAT_CLIENTS_READY],
            intermission: player.movement_type() == PM_INTERMISSION,
            // The player's own slot, not a followed player's.
            quiet: if let Ok(own) = u16::try_from(session.game_state().client_num)
                && own < 32
                && options.is_some_and(|c| c.bool_cvar(crate::chat::quiet::CVAR) == Some(true))
            {
                1 << own
            } else {
                0
            },
        };
        // Named when the kill was observed, shown while you are dead.
        let killer = (self.killer.is_some()
            && !player.is_spectator()
            && (player.health() <= 0 || self.style == style::ScoreboardStyle::Classic))
            .then_some(self.killer_name.as_str());
        if self.style == style::ScoreboardStyle::Sjk {
            let game = session.game_state();
            let header = sjk::SjkHeader {
                map: &self.map,
                gametype: self.gametype,
                limits: self.limits,
                elapsed: level_elapsed(game, snapshot.server_time),
                team_scores: session.team_scores(),
                local: local_status,
                killer,
                // Siege's alone: two configuration strings read per frame otherwise.
                siege_winner: (self.gametype == sjk_game_jka::match_end::GT_SIEGE)
                    .then(|| sjk_client::siege_round_winner(game, self.siege_round_over))
                    .flatten(),
                match_over: local_status.intermission || self.server_intermission,
                duel: sjk::Duelists {
                    local: Some((player.health(), player.armor())),
                    ..duelists(game)
                },
                compact: style::compact(options),
                ids: style::client_ids(options),
                deaths_rank: style::deaths_rank(options),
                fps: fps.filter(|_| style::fps(options)),
            };
            // Measured in the families that will draw the text.
            let measure = match fonts.sjk_metrics() {
                Some((display, body)) => sjk::Measure {
                    display: Some(display),
                    body: Some(body),
                },
                None => sjk::Measure {
                    display: Some(font),
                    body: Some(font),
                },
            };
            sjk::build(
                &mut self.ui,
                &self.rows,
                &header,
                flags,
                measure,
                &mut self.motion,
                viewport,
            );
            self.ui.finish(u16::MAX);
            match fonts.sjk() {
                Some(families) => {
                    self.ui
                        .append_text_families(families, viewport, TextStyle::NEUTRAL);
                }
                None => {
                    self.ui
                        .append_text_styled(vertices, font, viewport, TextStyle::NEUTRAL);
                }
            }
            return;
        }
        let header = classic::ClassicHeader {
            hostname: &self.hostname,
            max_clients: self.max_clients,
            gametype: self.gametype,
            fraglimit: self.fraglimit,
            team_scores: session.team_scores(),
            local: local_status,
            killer,
            context: self.context,
            duel_result: classic_duel_result(
                session.game_state(),
                &self.rows,
                self.gametype,
                local_status.intermission,
            ),
        };
        classic::build(
            &mut self.ui,
            &self.rows,
            &header,
            style::ClassicOptions::from_console(options),
            &self.icons,
            flags,
            &mut self.motion,
            viewport,
        );
        self.ui.finish(u16::MAX);
        self.ui.append_text_routed(
            fonts,
            |_, text| Some(retail_font(text)),
            |_| crate::text::CodePalette::Game,
            vertices,
            font,
            viewport,
        );
    }

    /// Mark who is away and move them below the others of their list, in the
    /// server's order again once back; with `enabled` off, nobody is. With
    /// `deaths_rank` (`cg_scoreboardDeathsRank`) the others go by score minus deaths
    /// ([`style::rank_value`]), fewer deaths first on a tie, then the server's order.
    fn mark_away(&mut self, enabled: bool, deaths_rank: bool) {
        for row in &mut self.rows {
            row.away = if !enabled || row.bot || !row.has_score {
                None
            } else {
                self.afk.away(row.client_num).or(row.tagged.then_some(0))
            };
        }
        self.rows.sort_unstable_by_key(|row| {
            // Rows without a count, or still joining (no score row yet, whatever the
            // local counter holds), follow the counted ones.
            let ranked = deaths_rank && row.has_score && row.deaths.is_some();
            (
                row.away.is_some(),
                !ranked,
                if ranked {
                    -style::rank_value(row.score, row.deaths, true)
                } else {
                    0
                },
                if ranked { row.deaths.unwrap_or(0) } else { 0 },
                row.order,
            )
        });
    }

    fn refresh(&mut self, session: &ClientSession) {
        let signature = config_signature(session.game_state());
        let server_signature = session
            .game_state()
            .config_string(0)
            .map_or(0, byte_signature);
        if self.cached_scores == session.scores()
            && self.config_signature == signature
            && self.server_signature == server_signature
        {
            return;
        }
        self.cached_scores.clear();
        self.cached_scores.extend_from_slice(session.scores());
        self.config_signature = signature;
        self.server_signature = server_signature;
        self.team_game = is_team_game(session.game_state());
        self.rows.clear();
        self.identity_revision = u64::MAX;
        let game = session.game_state();
        for score in session.scores() {
            let (name, team) = client_identity(game, score.client_num);
            let extra = client_extra(game, score.client_num);
            self.rows.push(ScoreRow {
                client_num: score.client_num,
                name,
                team,
                score: score.score,
                reported_deaths: score.deaths,
                deaths: None,
                ping: score.ping,
                time: score.time_minutes,
                has_score: true,
                bot: extra.bot,
                record: extra.record,
                powerups: score.powerups,
                defends: score.defends,
                assists: score.assists,
                captures: score.captures,
                identity: None,
                order: 0,
                tagged: false,
                away: None,
            });
        }
        // Connected clients the scores do not list yet (`CG_TeamScoreboard`'s
        // "fake" rows); only the classic style shows them.
        for client in 0..32_u8 {
            let listed = session
                .scores()
                .iter()
                .any(|score| score.client_num == client);
            let connected = game
                .config_string(CS_PLAYERS + usize::from(client))
                .is_some_and(|bytes| !bytes.is_empty());
            if listed || !connected {
                continue;
            }
            let (name, team) = client_identity(game, client);
            let extra = client_extra(game, client);
            self.rows.push(ScoreRow {
                client_num: client,
                name,
                team,
                score: 0,
                reported_deaths: None,
                deaths: None,
                ping: 0,
                time: 0,
                has_score: false,
                bot: extra.bot,
                record: extra.record,
                powerups: 0,
                defends: 0,
                assists: 0,
                captures: 0,
                identity: None,
                order: 0,
                tagged: false,
                away: None,
            });
        }
        for (order, row) in self.rows.iter_mut().enumerate() {
            row.order = order as u8;
            row.tagged = afk::tagged(&row.name);
        }
        let server = server_info(game);
        self.map.clear();
        self.map.push_str(&server.0);
        self.mode.clear();
        self.mode.push_str(server.1);
        let info = game
            .config_string(0)
            .and_then(|v| std::str::from_utf8(v).ok())
            .and_then(|v| InfoString::parse(v).ok());
        self.hostname.clear();
        self.hostname.push_str(
            info.as_ref()
                .and_then(|info| info.get("sv_hostname"))
                .unwrap_or(""),
        );
        let integer = |key: &str| info.as_ref().and_then(|info| info.get_i32(key));
        self.max_clients = integer("sv_maxclients").unwrap_or(32);
        self.gametype = integer("g_gametype").unwrap_or(0);
        self.fraglimit = integer("fraglimit").unwrap_or(0);
        self.limits = sjk::Limits {
            frags: self.fraglimit,
            captures: integer("capturelimit").unwrap_or(0),
            duel_wins: integer("duel_fraglimit").unwrap_or(0),
            minutes: integer("timelimit").unwrap_or(0),
        };
    }
}

/// A config string as text.
fn config_text(game: &GameState, index: usize) -> Option<&str> {
    game.config_string(index)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
}

fn classic_duel_result<'a>(
    game: &GameState,
    rows: &'a [ScoreRow],
    gametype: i32,
    intermission: bool,
) -> Option<classic::DuelResult<'a>> {
    if !intermission || !matches!(gametype, 3 | 4) {
        return None;
    }
    let name = |client: u8| {
        rows.iter()
            .find(|row| row.client_num == client)
            .map(|row| row.name.as_str())
    };
    if let Some(winner) = config_text(game, 29)
        .and_then(|s| s.parse::<u8>().ok())
        .and_then(name)
    {
        return Some(classic::DuelResult::Winner(winner));
    }
    let clients = duelists(game).clients;
    Some(classic::DuelResult::Versus(
        name(clients[0]?)?,
        name(clients[1]?)?,
        if gametype == 4 {
            clients[2].and_then(name)
        } else {
            None
        },
    ))
}

/// Milliseconds since the level started (`CS_LEVEL_START_TIME`), at
/// `server_time`, as the HUD's match timer counts them.
fn level_elapsed(game: &GameState, server_time: i32) -> Option<i32> {
    config_text(game, CS_LEVEL_START_TIME)
        .and_then(|start| start.trim().parse::<i32>().ok())
        .map(|start| server_time.saturating_sub(start).max(0))
}

/// Whether `CS_INTERMISSION` reads "1": set the moment a limit is hit, as siege's round
/// end is told by its own configuration string, so the result does not wait on this
/// client's `PM_INTERMISSION` (a spectator's or follower's may never carry it, and a mod
/// may hold it back).
fn intermission_set(game: &GameState) -> bool {
    config_text(game, sjk_game_jka::match_end::CS_INTERMISSION)
        .is_some_and(|text| text.trim().parse::<i32>().is_ok_and(|value| value != 0))
}

/// The server's end of the match as cgame takes it (`cg.intermissionStarted`): on when
/// `CS_INTERMISSION` turns on, off again at a new level start (`CG_MapRestart`). The
/// game never clears the string, and a `map_restart` (a single-map server's
/// `nextmap`, "map_restart 0") keeps it, so the string alone would call the next match
/// over too; a "1" already there when the level starts or this client joins is not
/// taken either, as in cgame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ServerIntermission {
    level_start: Option<i32>,
    set: bool,
    started: bool,
}

impl ServerIntermission {
    /// Follow this frame's configuration strings; whether the match is over.
    fn observe(&mut self, game: &GameState) -> bool {
        let start = config_text(game, CS_LEVEL_START_TIME)
            .and_then(|start| start.trim().parse::<i32>().ok());
        let set = intermission_set(game);
        if start != self.level_start {
            *self = Self {
                level_start: start,
                set,
                started: false,
            };
        } else if set != self.set {
            self.set = set;
            self.started = set;
        }
        self.started
    }
}

/// Who duels (`CS_CLIENT_DUELISTS`, `-1` for nobody) and their health when the
/// server shares it (`CS_CLIENT_DUELHEALTHS`, `g_showDuelHealths`).
fn duelists(game: &GameState) -> sjk::Duelists {
    let mut duel = sjk::Duelists::default();
    if let Some(text) = config_text(game, CS_CLIENT_DUELISTS) {
        for (slot, value) in text.split('|').take(3).enumerate() {
            duel.clients[slot] = value
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|client| *client < 32);
        }
    }
    if let Some(text) = config_text(game, CS_CLIENT_DUELHEALTHS) {
        for (slot, value) in text.split('|').take(2).enumerate() {
            duel.healths[slot] = value
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|health| *health >= 0);
        }
    }
    duel
}

/// `persistant[]` and `stats[]` indices (`bg_public.h`) and `PM_INTERMISSION`.
const PERS_SCORE: usize = 0;
const PERS_RANK: usize = 2;
const PERS_TEAM: usize = 3;
const STAT_CLIENTS_READY: usize = 7;
const PM_INTERMISSION: u8 = 6;

impl Scoreboard {
    /// Watch the players for the away marks ([`afk`]); every frame, scores and
    /// chat counters included, whether the board shows or not.
    pub(crate) fn observe_afk(
        &mut self,
        snapshot: &sjk_protocol::Snapshot,
        game: &GameState,
        scores: &[ScoreEntry],
        spoken: &[u32; 32],
    ) {
        self.afk.observe(snapshot, game, scores, spoken);
    }

    /// Whether `client`, shown as `name`, is away as the scoreboard marks it: seen still
    /// for 3 minutes, or wearing an away tag in their name (`afk`).
    pub(crate) fn seen_away(&self, client: u8, name: &str) -> bool {
        self.afk.away(client).is_some() || afk::tagged(name)
    }

    /// The resolved model icon of `client`'s slot.
    pub(crate) fn head_icon(&self, client: u8) -> Option<sjk_ui::TextureId> {
        self.icons.texture(client)
    }

    /// Resolve one client's model icon for the player card, with the scoreboard closed.
    pub(crate) fn ensure_head_icon(
        &mut self,
        game: &GameState,
        client: u8,
        vfs: &sjk_vfs::VirtualFileSystem,
        shaders: &sjk_shader::ShaderCatalog,
        renderer: &crate::ui_renderer::ShapeRenderer,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        self.icons.ensure(game, client, |path, texture| {
            let Some(pixels) = crate::hud::icons::assets::decode(vfs, shaders, path) else {
                return false;
            };
            renderer.upload_icon(queue, texture, pixels.as_raw());
            true
        });
    }
}

/// Append the current server scoreboard.
pub(crate) fn append_overlay(gpu: &mut crate::GpuState, viewport: [f32; 2], scale: f32) {
    #[cfg(test)]
    if gpu.scoreboard.shot.is_some() {
        shot::append(gpu, viewport);
        return;
    }
    let Some(session) = gpu.resident.session.as_ref().or(gpu.live_session.as_ref()) else {
        return;
    };
    let options = gpu.console.as_ref();
    let classic = gpu.scoreboard.style == style::ScoreboardStyle::Classic;
    if classic
        && style::ClassicOptions::from_console(options).icons
        && let Some(vfs) = &gpu.vfs
    {
        let (shaders, renderer, queue) = (&gpu.shaders, &gpu.ui_shapes, &gpu.queue);
        gpu.scoreboard
            .icons
            .update(session.game_state(), |path, texture| {
                let Some(pixels) = crate::hud::icons::assets::decode(vfs, shaders, path) else {
                    return false;
                };
                renderer.upload_icon(queue, texture, pixels.as_raw());
                true
            });
    }
    let flags = classic::FlagIcons {
        red: gpu.hud.icons.powerup(4),
        blue: gpu.hud.icons.powerup(5),
        neutral: gpu.hud.icons.powerup(6),
    };
    let fps = gpu.frame_pacer.fps();
    gpu.scoreboard.append(
        session,
        &mut gpu.game_fonts,
        options,
        flags,
        fps,
        &mut gpu.text_vertices,
        &gpu.ui_font,
        viewport,
        scale,
    );
}

/// The font retail's scoreboard draws `text` with: `CG_DrawClientScore`
/// (`cg_scoreboard.c`) paints names and headings with `FONT_MEDIUM` and the
/// score, ping and time numbers with `FONT_SMALL`.
fn retail_font(text: &str) -> RetailFont {
    let numeric = !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'/'));
    if numeric {
        RetailFont::Small
    } else {
        RetailFont::Medium
    }
}

fn server_info(game_state: &GameState) -> (String, &'static str) {
    let info = game_state
        .config_string(0)
        .and_then(|v| std::str::from_utf8(v).ok())
        .and_then(|v| InfoString::parse(v).ok());
    let Some(info) = info else {
        return ("UNKNOWN MAP".to_owned(), "MATCH");
    };
    let map = info.get("mapname").unwrap_or("UNKNOWN MAP").to_owned();
    let mode = match info.get_i32("g_gametype") {
        Some(0) => "FFA",
        Some(3) => "DUEL",
        Some(4) => "POWER DUEL",
        Some(6) => "TEAM FFA",
        Some(7) => "SIEGE",
        Some(8) => "CTF",
        Some(9) => "CTY",
        _ => "MATCH",
    };
    (map, mode)
}

fn client_identity(game_state: &GameState, client_num: u8) -> (String, u8) {
    let (name, team) = game_state
        .config_string(CS_PLAYERS + usize::from(client_num))
        .map_or_else(
            || (format!("Client {client_num}"), 3),
            |bytes| client_identity_from_config(bytes, client_num),
        );
    // A blocked player's name reads BLOCKED (`muted_players.rs`).
    if crate::muted_players::shown_muted(client_num) {
        return (crate::muted_players::LABEL.to_owned(), team);
    }
    (name, team)
}
fn client_identity_from_config(bytes: &[u8], client_num: u8) -> (String, u8) {
    let info = sjk_client::LegacyClientInfo::new(bytes);
    (
        info.bytes("n")
            .or_else(|| info.bytes("name"))
            .filter(|n| !n.is_empty())
            .map_or_else(
                || format!("Client {client_num}"),
                // Latin-1, not UTF-8: see `sjk_client::decode_legacy`.
                |v| sjk_client::decode_legacy(v).into_owned(),
            ),
        info.integer("t")
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(3),
    )
}

/// Clientinfo facts beyond name and team.
struct ClientExtra {
    bot: bool,
    record: Option<(i32, i32)>,
}

fn client_extra(game_state: &GameState, client_num: u8) -> ClientExtra {
    let info = game_state
        .config_string(CS_PLAYERS + usize::from(client_num))
        .map(sjk_client::LegacyClientInfo::new);
    let integer = |key: &str| info.as_ref().and_then(|info| info.integer(key));
    ClientExtra {
        bot: info
            .as_ref()
            .is_some_and(|info| info.bytes("skill").is_some()),
        record: integer("w").zip(integer("l")),
    }
}

fn is_team_game(game_state: &GameState) -> bool {
    game_state
        .config_string(0)
        .and_then(|v| std::str::from_utf8(v).ok())
        .and_then(|v| InfoString::parse(v).ok())
        .and_then(|v| v.get_i32("g_gametype"))
        .is_some_and(|v| v >= 6)
}
fn config_signature(game_state: &GameState) -> u64 {
    // The connected roster also supplies fake rows and player counts. Watching
    // only clients already in `scores` misses joins and departures between
    // score responses, and leaves JoF's roster-dependent layout stale.
    (0..32).fold(0xcbf29ce484222325_u64, |mut hash, client| {
        if let Some(bytes) = game_state
            .config_string(CS_PLAYERS + client)
            .filter(|bytes| !bytes.is_empty())
        {
            hash ^= client as u64;
            hash ^= byte_signature(bytes);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    })
}
fn byte_signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

/// Automatic intermission scores and the ordinary held scoreboard share one layout.
pub(crate) fn requested(gpu: &crate::GpuState, intermission: bool) -> bool {
    behavior::context(gpu, intermission)
        .requested(style::ScoreboardStyle::from_console(gpu.console.as_ref()))
}

/// Sample before presentation so death, warmup and roster changes apply together.
pub(crate) fn update_context(gpu: &mut crate::GpuState, intermission: bool) {
    gpu.scoreboard.context = behavior::context(gpu, intermission);
}

/// SJK replaces the whole HUD while held; classic uses selective JoF visibility.
pub(crate) fn hides_hud(gpu: &crate::GpuState) -> bool {
    style::ScoreboardStyle::from_console(gpu.console.as_ref()) == style::ScoreboardStyle::Sjk
        && gpu.gameplay_input.held(crate::input::GameButton::Scores)
        && gpu
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar("cg_drawScores"))
            .unwrap_or(true)
}

#[path = "scoreboard/hud_policy.rs"]
mod hud_policy;
pub(crate) use hud_policy::{hides_crosshair, visibility};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_classic_death_scores_keep_the_killer_until_respawn_and_close() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar(style::CVAR, "classic");
        let mut board = Scoreboard::new();
        board.context.dead = true;
        board.killer = Some(7);
        board.killer_name.push_str("Attacker");
        assert!(board.present(
            Some(&console),
            board.context.requested(style::ScoreboardStyle::Classic),
            true
        ));
        assert_eq!(board.killer, Some(7));
        assert!(board.present(Some(&console), true, true));
        board.context.dead = false;
        assert!(board.present(Some(&console), true, true));
        board.motion.hide();
        assert!(!board.present(Some(&console), false, true));
        assert_eq!(board.killer, None);
        assert!(board.killer_name.is_empty());
    }

    #[test]
    fn duel_intermission_names_take_priority_and_invalid_winners_fall_back() {
        let mut board = Scoreboard::new();
        board.show_for_shot(shot::Match::PowerDuel);
        let mut game = GameState::empty_local(0);
        game.replace_config_string(29, b"5".to_vec()).unwrap();
        game.replace_config_string(30, b"5|0|1".to_vec()).unwrap();
        let name = |client| {
            board
                .rows
                .iter()
                .find(|row| row.client_num == client)
                .unwrap()
                .name
                .as_str()
        };
        assert_eq!(
            classic_duel_result(&game, &board.rows, 4, true),
            Some(classic::DuelResult::Winner(name(5)))
        );
        assert_eq!(classic_duel_result(&game, &board.rows, 4, false), None);
        assert_eq!(classic_duel_result(&game, &board.rows, 0, true), None);
        game.replace_config_string(29, b"-1".to_vec()).unwrap();
        assert_eq!(
            classic_duel_result(&game, &board.rows, 4, true),
            Some(classic::DuelResult::Versus(name(5), name(0), Some(name(1))))
        );
        assert_eq!(
            classic_duel_result(&game, &board.rows, 3, true),
            Some(classic::DuelResult::Versus(name(5), name(0), None))
        );
        game.replace_config_string(30, b"-1|0|1".to_vec()).unwrap();
        assert_eq!(classic_duel_result(&game, &board.rows, 4, true), None);
    }

    #[test]
    fn the_match_is_over_once_the_server_sets_cs_intermission() {
        let mut game = GameState::empty_local(0);
        let mut over = ServerIntermission::default();
        game.replace_config_string(CS_LEVEL_START_TIME, b"1000".to_vec())
            .unwrap();
        assert!(!over.observe(&game));
        game.replace_config_string(22, b"1".to_vec()).unwrap();
        assert!(over.observe(&game));
        // A `map_restart` keeps the string but starts a new level: not over.
        game.replace_config_string(CS_LEVEL_START_TIME, b"90000".to_vec())
            .unwrap();
        assert!(!over.observe(&game));
        assert!(!over.observe(&game));
        // A new map clears it; "0" is not over either.
        game.replace_config_string(22, Vec::new()).unwrap();
        assert!(!over.observe(&game));
        game.replace_config_string(22, b"0".to_vec()).unwrap();
        assert!(!over.observe(&game));
        game.replace_config_string(22, b"1".to_vec()).unwrap();
        assert!(over.observe(&game));
        // Joining while it already reads "1" does not end the match.
        let mut joined = ServerIntermission::default();
        assert!(!joined.observe(&game));
    }

    #[test]
    fn roster_signature_tracks_clients_without_score_rows() {
        let mut game = GameState::empty_local(0);
        let empty = config_signature(&game);
        game.replace_config_string(CS_PLAYERS + 7, br"\n\Joining\t\3".to_vec())
            .unwrap();
        let joined = config_signature(&game);
        assert_ne!(empty, joined);
        game.replace_config_string(CS_PLAYERS + 7, br"\n\Joining\t\0".to_vec())
            .unwrap();
        assert_ne!(joined, config_signature(&game));
        game.replace_config_string(CS_PLAYERS + 7, Vec::new())
            .unwrap();
        // Empty and absent player configstrings describe the same roster.
        assert_eq!(empty, config_signature(&game));
    }

    /// The SJK look's clock and duelists come from config strings.
    #[test]
    fn the_level_clock_and_the_duelists_are_read_from_config_strings() {
        let mut game = GameState::empty_local(0);
        assert_eq!(level_elapsed(&game, 90_000), None);
        assert_eq!(duelists(&game), sjk::Duelists::default());
        let mut set = |index, value: &str| {
            game.replace_config_string(index, value.as_bytes().to_vec())
                .unwrap();
        };
        set(CS_LEVEL_START_TIME, "30000");
        set(CS_CLIENT_DUELISTS, "4|7");
        set(CS_CLIENT_DUELHEALTHS, "87|-1|!");
        assert_eq!(level_elapsed(&game, 90_000), Some(60_000));
        // A clock behind the level's start (a restart) reads zero.
        assert_eq!(level_elapsed(&game, 10_000), Some(0));
        let duel = duelists(&game);
        assert_eq!(duel.clients, [Some(4), Some(7), None]);
        assert_eq!(duel.healths, [Some(87), None]);
        // Nobody duelling.
        let mut game = GameState::empty_local(0);
        game.replace_config_string(CS_CLIENT_DUELISTS, b"-1|-1".to_vec())
            .unwrap();
        assert_eq!(duelists(&game).clients, [None; 3]);
    }

    #[test]
    fn numbers_use_the_small_font_and_names_the_medium_one() {
        for text in ["12", "-3", "7/2"] {
            assert_eq!(retail_font(text), RetailFont::Small);
        }
        for text in ["", "^1Padawan", "PING", "SCORE / DEATHS", "2fast"] {
            assert_eq!(retail_font(text), RetailFont::Medium);
        }
    }

    fn ranked_row(client: u8, score: i32, deaths: Option<i32>) -> ScoreRow {
        ScoreRow {
            client_num: client,
            name: format!("Player{client}"),
            team: 0,
            score,
            reported_deaths: deaths,
            deaths,
            ping: 50,
            time: 5,
            has_score: true,
            bot: false,
            record: None,
            powerups: 0,
            defends: 0,
            assists: 0,
            captures: 0,
            identity: None,
            order: client,
            tagged: false,
            away: None,
        }
    }

    fn clients(board: &Scoreboard) -> Vec<u8> {
        board.rows.iter().map(|row| row.client_num).collect()
    }

    /// `cg_scoreboardDeathsRank`: score minus deaths, fewer deaths on a tie, rows
    /// without a count after; off, the server's order.
    #[test]
    fn rows_rank_by_score_minus_deaths_when_asked() {
        let mut board = Scoreboard::new();
        board.rows = vec![
            ranked_row(0, 10, Some(8)), // 2
            ranked_row(1, 8, Some(1)),  // 7
            ranked_row(2, 9, Some(2)),  // 7, more deaths
            ranked_row(3, 12, None),    // no count
            ranked_row(4, 3, Some(0)),  // 3
        ];
        board.mark_away(false, false);
        assert_eq!(clients(&board), [0, 1, 2, 3, 4]);
        board.mark_away(false, true);
        assert_eq!(clients(&board), [1, 2, 4, 0, 3]);
        board.mark_away(false, false);
        assert_eq!(
            clients(&board),
            [0, 1, 2, 3, 4],
            "back to the server's order"
        );
        // Still joining: no score row, though the local counter has its deaths.
        let mut joining = ranked_row(5, 0, Some(5));
        joining.has_score = false;
        board.rows.insert(2, joining);
        board.mark_away(false, true);
        assert_eq!(clients(&board), [1, 2, 4, 0, 3, 5], "the joining row last");
    }
}
