//! Runtime projection feeding match information into the retained HUD.

use super::*;
use crate::hud::flag_status::ScoreRows;
use bytemuck::{Pod, Zeroable};

/// Append HUD text and overhead labels without duplicating the classic-font fallback.
pub(crate) fn append(
    gpu: &mut GpuState,
    classic: bool,
    _visibility: hud::HudVisibility,
    _layout: hud::HudLayout,
    _scale: f32,
    viewport: [f32; 2],
) {
    // Nameplates use the classic HUD font whatever `cg_classicHudFont` says,
    // and fall back to Inter when that font is not loaded.
    let power_icons = gpu.hud.power_icons();
    match &gpu.classic_hud_font {
        Some(font) => gpu.hud.nameplate.append(
            &gpu.chat,
            &power_icons,
            &mut gpu.classic_text_vertices,
            font,
            viewport,
        ),
        None => gpu.hud.nameplate.append(
            &gpu.chat,
            &power_icons,
            &mut gpu.text_vertices,
            &gpu.ui_font,
            viewport,
        ),
    }
    gpu.hud
        .identification
        .append(&gpu.chat, &mut gpu.text_vertices, &gpu.ui_font, viewport);
    gpu.hud
        .card
        .append(&mut gpu.text_vertices, &gpu.ui_font, viewport);
    if classic && let Some(font) = &gpu.classic_hud_font {
        gpu.hud.append(
            &mut gpu.game_fonts,
            &mut gpu.classic_text_vertices,
            font,
            viewport,
        );
    } else {
        gpu.hud.append(
            &mut gpu.game_fonts,
            &mut gpu.text_vertices,
            &gpu.ui_font,
            viewport,
        );
    }
}

/// Cut the head icon the nameplates want this frame, if any
/// ([`hud::nameplate::State::head_to_cut`]): the icon of a model worn by a player
/// without SJK, read from the game data, cut round and kept in the pictures' cache
/// ([`crate::avatars::insert_local`]); without one their plate shows their initial.
fn cut_nameplate_head(
    nameplate: &mut hud::nameplate::State,
    vfs: Option<&sjk_vfs::VirtualFileSystem>,
    shaders: &sjk_shader::ShaderCatalog,
    game: &sjk_protocol::GameState,
) {
    let Some((slot, key)) = nameplate.head_to_cut() else {
        return;
    };
    if crate::avatars::has_local(key) {
        return;
    }
    let Some(vfs) = vfs else {
        return;
    };
    let model = game
        .config_string(1131 + usize::from(slot))
        .and_then(|info| sjk_client::LegacyClientInfo::new(info).bytes("model"))
        .unwrap_or_default();
    match hud::portrait::icon_path(model)
        .and_then(|path| hud::icons::assets::decode(vfs, shaders, &path))
    {
        Some(icon) => {
            let mut rgba = icon.into_raw();
            crate::avatars::picture::round(&mut rgba, crate::ui_renderer::ICON_SIZE);
            crate::avatars::insert_local(key, rgba);
        }
        None => nameplate.head_missing(slot, key, game),
    }
}

/// GPU layout shared with `hud.wgsl`; appended parameters begin at byte 144.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct HudUniform {
    /// Resolved crosshair RGBA.
    pub(crate) crosshair_color: [f32; 4],
    /// Displayed health fraction.
    pub(crate) health_ratio: f32,
    /// Displayed armor fraction.
    pub(crate) armor_ratio: f32,
    /// Displayed Force fraction.
    pub(crate) force_ratio: f32,
    /// Menu backdrop selector.
    pub(crate) menu_open: f32,
    /// Reciprocal physical viewport width.
    pub(crate) inverse_width: f32,
    /// Reciprocal physical viewport height.
    pub(crate) inverse_height: f32,
    /// Selected menu row.
    pub(crate) menu_row: f32,
    /// Number of menu rows.
    pub(crate) menu_row_count: f32,
    /// Crosshair size; zero hides it.
    pub(crate) crosshair: f32,
    /// Global HUD visibility.
    pub(crate) hud_visible: f32,
    /// Legacy status-bar switch.
    pub(crate) status_visible: f32,
    /// Menu presentation variant.
    pub(crate) menu_kind: f32,
    /// Seconds since the UI epoch.
    pub(crate) menu_phase: f32,
    /// Damage direction, horizontal component.
    pub(crate) damage_x: f32,
    /// Damage direction, vertical component.
    pub(crate) damage_y: f32,
    /// Damage indicator opacity.
    pub(crate) damage_alpha: f32,
    /// Damage indicator intensity.
    pub(crate) damage_strength: f32,
    /// Retained health-bar rectangle.
    pub(crate) health_bar: [f32; 4],
    /// Retained armor-bar rectangle.
    pub(crate) armor_bar: [f32; 4],
    /// Retained Force-bar rectangle.
    pub(crate) force_bar: [f32; 4],
    /// Existing uniform tail padding.
    pub(crate) _padding: [f32; 3],
    /// Offset UV and dimensions used to size the crosshair.
    pub(crate) crosshair_parameters: [f32; 4],
    /// The damage indicator's colour when its alpha is above 0.5, else stock's red
    /// (`cg_hitMarkerShield`: green for a hit the shield took).
    pub(crate) damage_color: [f32; 4],
}

/// Refresh retained HUD data and project movement guides through this frame's camera.
pub(crate) fn update(
    gpu: &mut GpuState,
    view_position: Vec3,
    view: (Vec3, Vec3, f32),
    presentation_time: i32,
    intermission: bool,
) {
    gpu.refresh_force_support();
    let (view_target, view_up, vertical_fov) = view;
    hud::tints::update(gpu, view_position, presentation_time, intermission);
    hud::vehicle_feed::sample(gpu, presentation_time);
    let scoreboard = crate::scoreboard::hides_crosshair(gpu, presentation_time);
    // Plates and logos stay off any open board, classic's included (held or after death).
    let labels_hidden = crate::scoreboard::hides_hud(gpu)
        || gpu.gameplay_input.held(input::GameButton::Scores)
        || scoreboard
        || intermission
        || hud::toggle::hides(gpu.console.as_ref(), hud::toggle::Part::Names)
        || gpu
            .console
            .as_ref()
            .is_some_and(|c| !c.bool_cvar("cg_draw2d").unwrap_or(true));
    gpu.hud.identification.sample(gpu.console.as_ref());
    gpu.hud.nameplate.sample(gpu.console.as_ref());
    gpu.hud.card.sample(gpu.console.as_ref());
    gpu.hud.team_logo.sample(gpu.console.as_ref());
    gpu.hud.team_outline.sample(gpu.console.as_ref());
    // Nameplate text is in the classic stream, which draws over the menus' text.
    let plates_hidden = labels_hidden
        || gpu.game_menu
        || gpu
            .client_menu
            .as_ref()
            .is_some_and(|menu| menu.is_visible());
    // A nameplate replaces the plain overhead names.
    let labels_hidden = labels_hidden || gpu.hud.nameplate.enabled();
    // `CG_DrawRocketLocking` belongs to `CG_Draw2D`: none at intermission or with
    // `cg_draw2D 0`.
    let lock_hidden = intermission
        || gpu
            .console
            .as_ref()
            .is_some_and(|c| !c.bool_cvar("cg_draw2d").unwrap_or(true));
    let camera = hud::identification::Camera {
        eye: view_position,
        target: view_target,
        up: view_up,
        fov: vertical_fov,
        viewport: [
            gpu.configuration.width as f32,
            gpu.configuration.height as f32,
        ],
    };
    if let Some(session) = &gpu.live_session {
        let snapshot = session.latest_snapshot();
        gpu.scoreboard.observe_deaths(
            &gpu.obituaries,
            snapshot,
            session.game_state(),
            gpu.console.as_ref(),
        );
        gpu.scoreboard.observe_afk(
            snapshot,
            session.game_state(),
            session.scores(),
            gpu.chat.spoken(),
        );
        // Team logos first: the overhead names and plates stand above them.
        gpu.hud.team_logo.update(
            snapshot,
            session.game_state(),
            &gpu.live_world,
            i64::from(presentation_time),
            camera,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            plates_hidden,
            |logo| gpu.hud.icons.team_logo(logo),
        );
        let hunt_red = gpu.box_hunt_outlined();
        gpu.hud.team_outline.update(
            snapshot,
            session.game_state(),
            &gpu.live_world,
            i64::from(presentation_time),
            camera,
            plates_hidden,
            hunt_red,
        );
        let logo_tops = gpu.hud.team_logo.tops();
        gpu.hud.identification.set_logo_tops(logo_tops);
        gpu.hud.nameplate.set_logo_tops(logo_tops);
        let duel_colours = gpu.hud.duel_pairs.update(snapshot, session.game_state());
        gpu.hud.nameplate.set_duel_colours(duel_colours);
        gpu.hud.identification.update(
            snapshot,
            session.game_state(),
            &gpu.live_world,
            i64::from(presentation_time),
            camera,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            labels_hidden,
        );
        gpu.hud
            .nameplate
            .refresh_marks(i64::from(presentation_time), || {
                let game_state = session.game_state();
                (
                    player_identity::slot_marks(game_state),
                    player_identity::slot_pictures(game_state),
                )
            });
        gpu.hud
            .nameplate
            .refresh_shared(i64::from(presentation_time), || {
                let game_state = session.game_state();
                (
                    player_identity::shared_vitals(game_state),
                    player_identity::sharing_live(game_state),
                )
            });
        player_identity::share_vitals(gpu.hud.nameplate.sharing(Some(snapshot)));
        let detached_flight = gpu.free_camera_active();
        gpu.hud.nameplate.set_own_origin(gpu.third_person.then(|| {
            gpu.local_prediction
                .predicted_state()
                .filter(|_| !detached_flight)
                .map_or(snapshot.player.origin(), |state| state.origin)
        }));
        gpu.hud.nameplate.update(
            snapshot,
            session.game_state(),
            &gpu.live_world,
            session.team_info(),
            i64::from(presentation_time),
            camera,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            gpu.local_prediction
                .predicted_state()
                .filter(|_| !detached_flight)
                .map(|state| state.force_powers_active),
            plates_hidden,
        );
        cut_nameplate_head(
            &mut gpu.hud.nameplate,
            gpu.vfs.as_deref(),
            &gpu.shaders,
            session.game_state(),
        );
        gpu.hud.rocket_lock.update(
            hud::rocket_lock::input(
                snapshot,
                session.game_state(),
                &gpu.live_world,
                presentation_time,
                lock_hidden,
            ),
            camera,
        );
        gpu.hud
            .update_family(snapshot, session.game_state(), gpu.console.as_ref());
        gpu.hud.update_flag_carriers(
            snapshot,
            session.game_state(),
            session.team_info().entries(),
            ScoreRows {
                rows: session.scores(),
                revision: session.scores_revision(),
            },
        );
        let ja_plus = matches!(
            session.compat_profile(),
            sjk_client::CompatProfile::JaPlus { .. }
        );
        gpu.hud.update_selection(
            &gpu.gameplay_input.selection,
            &snapshot.player,
            presentation_time,
            ja_plus,
        );
        gpu.lagometer
            .add_frame(presentation_time - snapshot.server_time);
        let mut ray = crosshair_scan::dynamic::ray(
            gpu.console.as_ref(),
            &snapshot.player,
            gpu.local_prediction.predicted_state(),
            session.game_state(),
            camera,
            gpu.third_person,
            snapshot.player.weapon(),
        );
        if ray.projected {
            ray.distance = gpu.crosshair_scan.distance_cull.unwrap_or(6000.0);
        }
        let crosshair = gpu.crosshair_scan.scan(
            snapshot,
            session.game_state(),
            ray,
            presentation_time,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            crosshair_scan::Suppression {
                scoreboard,
                intermission,
            },
        );
        // The nameplates' target mode keeps the player aimed at for a few seconds.
        gpu.hud.nameplate.set_focus(
            gpu.crosshair_scan.hit_now(presentation_time),
            i64::from(presentation_time),
        );
        gpu.hud.targeting.dynamic_offset = if ray.projected {
            Some(
                crosshair_scan::dynamic::offset(camera, gpu.crosshair_scan.endpoint)
                    .unwrap_or([2.0; 2]),
            )
        } else {
            None
        };
        let head_icons = &gpu.scoreboard;
        let (looks, skins) = (&gpu.looks, &*gpu.blade_skins);
        let hilts = gpu.saber_hilts.as_ref();
        let own_slot = crate::looks::ViewSlots::of(session.game_state(), &snapshot.player).own;
        // A player blocked on this PC gets no card, and a pinned one loses their profile.
        let blocked = gpu.muted_players.slots();
        gpu.hud.card.update(hud::player_card::Input {
            seen: gpu
                .crosshair_scan
                .hit_now(presentation_time)
                .filter(|&client| !blocked.contains(usize::from(client))),
            bsp: &gpu.bsp,
            scratch: &mut gpu.trace_scratch,
            game: session.game_state(),
            team_sabers: crate::team_sabers::Settings::of(gpu.console.as_ref()),
            class_sabers: gpu
                .siege_sabers
                .forced(Some(session.game_state()), gpu.vfs.as_deref()),
            world: &gpu.live_world,
            now: presentation_time,
            camera,
            hidden: plates_hidden,
            hub: &|slot, name| {
                if blocked.contains(usize::from(slot)) {
                    None
                } else {
                    player_identity::hub_info(slot, name)
                }
            },
            // What players share changes their card too.
            hub_revision: player_identity::revision()
                ^ player_identity::vitals_flags(session.game_state()).rotate_left(17)
                ^ u64::from(blocked.muted).rotate_left(40),
            entities: &snapshot.entities,
            icon: &|slot| head_icons.head_icon(slot),
            shader: &|slot| {
                let pair = holds_pair(session.game_state(), hilts, slot);
                card_shaders(looks, skins, own_slot, slot, pair)
            },
            skins,
            looks_revision: looks.revision() ^ skins.generation().rotate_left(32),
            profiles_revision: player_identity::profiles_revision(),
        });
        if let (Some(client), Some(vfs)) = (
            gpu.hud
                .card
                .shown_client()
                .and_then(|client| u8::try_from(client).ok()),
            &gpu.vfs,
        ) {
            gpu.scoreboard.ensure_head_icon(
                session.game_state(),
                client,
                vfs,
                &gpu.shaders,
                &gpu.ui_shapes,
                &gpu.queue,
            );
        }
        gpu.hud.update(
            session,
            presentation_time.max(0) as u64,
            gpu.local_prediction.predicted_state(),
            gpu.console.as_ref(),
        );
        // An empty kill feed shows a sample to drag in the HUD edit mode.
        gpu.hud.kill_feed.set_preview(gpu.hud_edit.is_open());
        gpu.hud.update_match_information(
            session.game_state(),
            presentation_time,
            &gpu.obituaries,
            snapshot.player.client_num(),
            &gpu.lagometer,
            crosshair,
            !session.is_local()
                && sjk_client::connection_interrupted(presentation_time, snapshot.server_time),
            hud::follow_shown(snapshot.player.movement_flags(), false),
        );
    } else if let Some(session) = &gpu.demo_session {
        let snapshot = session.snapshot_at_or_before(presentation_time);
        gpu.scoreboard.observe_deaths(
            &gpu.obituaries,
            snapshot,
            session.game_state(),
            gpu.console.as_ref(),
        );
        gpu.scoreboard
            .observe_afk(snapshot, session.game_state(), &[], gpu.chat.spoken());
        gpu.hud.team_logo.update(
            snapshot,
            session.game_state(),
            session.world(),
            i64::from(presentation_time),
            camera,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            plates_hidden,
            |logo| gpu.hud.icons.team_logo(logo),
        );
        gpu.hud.team_outline.update(
            snapshot,
            session.game_state(),
            session.world(),
            i64::from(presentation_time),
            camera,
            plates_hidden,
            0,
        );
        let logo_tops = gpu.hud.team_logo.tops();
        gpu.hud.identification.set_logo_tops(logo_tops);
        gpu.hud.nameplate.set_logo_tops(logo_tops);
        let duel_colours = gpu.hud.duel_pairs.update(snapshot, session.game_state());
        gpu.hud.nameplate.set_duel_colours(duel_colours);
        gpu.hud.identification.update(
            snapshot,
            session.game_state(),
            session.world(),
            i64::from(presentation_time),
            camera,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            labels_hidden,
        );
        gpu.hud
            .nameplate
            .set_own_origin(gpu.third_person.then(|| snapshot.player.origin()));
        gpu.hud.nameplate.update(
            snapshot,
            session.game_state(),
            session.world(),
            &sjk_client::TeamInfoTable::default(),
            i64::from(presentation_time),
            camera,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            None,
            plates_hidden,
        );
        cut_nameplate_head(
            &mut gpu.hud.nameplate,
            gpu.vfs.as_deref(),
            &gpu.shaders,
            session.game_state(),
        );
        gpu.hud.rocket_lock.update(
            hud::rocket_lock::input(
                snapshot,
                session.game_state(),
                session.world(),
                presentation_time,
                lock_hidden,
            ),
            camera,
        );
        gpu.hud
            .update_family(snapshot, session.game_state(), gpu.console.as_ref());
        // A demo keeps no `tinfo` or `scores`: the carriers come from the snapshot.
        gpu.hud
            .update_flag_carriers(snapshot, session.game_state(), &[], ScoreRows::default());
        // Only merc mode needs the server's mod; read it from the demo's gamestate then.
        let ja_plus = snapshot.player.entity_flags() & 0x1000 != 0
            && matches!(
                sjk_client::CompatProfile::from_game_state(session.game_state()),
                sjk_client::CompatProfile::JaPlus { .. }
            );
        gpu.hud.update_selection(
            &gpu.gameplay_input.selection,
            &snapshot.player,
            presentation_time,
            ja_plus,
        );
        gpu.lagometer
            .add_frame(presentation_time - snapshot.server_time);
        let mut ray = crosshair_scan::dynamic::ray(
            gpu.console.as_ref(),
            &snapshot.player,
            None,
            session.game_state(),
            camera,
            gpu.third_person,
            snapshot.player.weapon(),
        );
        if ray.projected {
            ray.distance = gpu.crosshair_scan.distance_cull.unwrap_or(6000.0);
        }
        let crosshair = gpu.crosshair_scan.scan(
            snapshot,
            session.game_state(),
            ray,
            presentation_time,
            &gpu.bsp,
            &mut gpu.trace_scratch,
            crosshair_scan::Suppression {
                scoreboard,
                intermission,
            },
        );
        // The nameplates' target mode keeps the player aimed at for a few seconds.
        gpu.hud.nameplate.set_focus(
            gpu.crosshair_scan.hit_now(presentation_time),
            i64::from(presentation_time),
        );
        gpu.hud.targeting.dynamic_offset = if ray.projected {
            Some(
                crosshair_scan::dynamic::offset(camera, gpu.crosshair_scan.endpoint)
                    .unwrap_or([2.0; 2]),
            )
        } else {
            None
        };
        let head_icons = &gpu.scoreboard;
        let (looks, skins) = (&gpu.looks, &*gpu.blade_skins);
        let hilts = gpu.saber_hilts.as_ref();
        let own_slot = crate::looks::ViewSlots::of(session.game_state(), &snapshot.player).own;
        gpu.hud.card.update(hud::player_card::Input {
            seen: gpu.crosshair_scan.hit_now(presentation_time),
            bsp: &gpu.bsp,
            scratch: &mut gpu.trace_scratch,
            game: session.game_state(),
            team_sabers: crate::team_sabers::Settings::of(gpu.console.as_ref()),
            class_sabers: gpu
                .siege_sabers
                .forced(Some(session.game_state()), gpu.vfs.as_deref()),
            world: session.world(),
            now: presentation_time,
            camera,
            hidden: plates_hidden,
            hub: &player_identity::hub_info,
            // What players share changes their card too.
            hub_revision: player_identity::revision()
                ^ player_identity::vitals_flags(session.game_state()).rotate_left(17),
            entities: &snapshot.entities,
            icon: &|slot| head_icons.head_icon(slot),
            shader: &|slot| {
                let pair = holds_pair(session.game_state(), hilts, slot);
                card_shaders(looks, skins, own_slot, slot, pair)
            },
            skins,
            looks_revision: looks.revision() ^ skins.generation().rotate_left(32),
            profiles_revision: player_identity::profiles_revision(),
        });
        if let (Some(client), Some(vfs)) = (
            gpu.hud
                .card
                .shown_client()
                .and_then(|client| u8::try_from(client).ok()),
            &gpu.vfs,
        ) {
            gpu.scoreboard.ensure_head_icon(
                session.game_state(),
                client,
                vfs,
                &gpu.shaders,
                &gpu.ui_shapes,
                &gpu.queue,
            );
        }
        gpu.hud
            .update_player(&snapshot.player, presentation_time.max(0) as u64);
        gpu.hud.update_demo_votes(
            session.game_state(),
            &snapshot.player,
            presentation_time.max(0) as u64,
            gpu.console.as_ref(),
        );
        gpu.hud.update_match_information(
            session.game_state(),
            presentation_time,
            &gpu.obituaries,
            snapshot.player.client_num(),
            &gpu.lagometer,
            crosshair,
            false,
            hud::follow_shown(snapshot.player.movement_flags(), true),
        );
    }
    let game = gpu
        .live_session
        .as_ref()
        .map(|s| s.game_state())
        .or_else(|| gpu.demo_session.as_ref().map(|s| s.game_state()));
    {
        if let Some(game) = gpu
            .resident
            .session
            .as_ref()
            .map(|s| s.game_state())
            .or(game)
        {
            gpu.chat.update_roster(game);
        } else {
            gpu.hud.identification.list.clear();
            gpu.hud.nameplate.clear();
            gpu.hud.card.clear();
            gpu.hud.team_logo.clear();
            gpu.hud.team_outline.clear();
        }
    }
    if let Some(game) = game {
        let snapshot = gpu
            .live_session
            .as_ref()
            .map(|s| s.latest_snapshot())
            .or_else(|| {
                gpu.demo_session
                    .as_ref()
                    .map(|s| s.snapshot_at_or_before(presentation_time))
            });
        if let Some(snapshot) = snapshot {
            gpu.hud
                .targeting
                .classify(gpu.crosshair_scan.color, snapshot.player.weapon());
            gpu.hud.enemy_info.update(
                game,
                snapshot,
                gpu.console.as_ref(),
                &gpu.chat,
                labels_hidden,
                gpu.live_session.as_ref().map_or(&[], |s| s.scores()),
            );
            if let Some(vfs) = &gpu.vfs {
                gpu.hud.enemy_info.update_portrait(
                    game,
                    vfs,
                    &gpu.shaders,
                    &gpu.ui_shapes,
                    &gpu.queue,
                );
            }
            gpu.chat.observe_combat(
                snapshot,
                game,
                &gpu.obituaries,
                gpu.console.as_ref(),
                camera,
                presentation_time,
            );
        }
    }
    let japro = game
        .and_then(|g| g.config_string(0))
        .is_some_and(|info| info.windows(5).any(|w| w.eq_ignore_ascii_case(b"japro")));
    let keys = gpu
        .live_session
        .as_ref()
        .and_then(|_| gpu.local_prediction.guide_input());
    gpu.hud.restrict_guides(!japro, keys);
    // The movement keys: the own command while playing, a guess while following or in
    // a demo, with the map's floor under the player for a guessed jump.
    let viewed = gpu
        .live_session
        .as_ref()
        .map(|s| (s.latest_snapshot(), false))
        .or_else(|| {
            gpu.demo_session
                .as_ref()
                .map(|s| (s.snapshot_at_or_before(presentation_time), true))
        });
    if let Some((snapshot, demo)) = viewed {
        let player = &snapshot.player;
        let own = gpu
            .local_prediction
            .movement_keys_input()
            .filter(|_| !demo && !hud::movement_keys::following(player));
        let (bsp, scratch) = (&gpu.bsp, &mut gpu.trace_scratch);
        gpu.hud
            .movement_keys
            .update(player, game, own, gpu.console.as_ref(), |origin| {
                // `PM_GroundDistance2`: straight down 4096 units against solid map.
                let mut end = origin;
                end[2] -= 4096.0;
                let trace = bsp.trace_box_with(scratch, origin, end, sjk_bsp::Aabb::POINT, 1);
                Vec3::from_array(origin).distance(Vec3::from_array(trace.end_position))
            });
    }
    gpu.hud.guide_camera(
        view_target - view_position,
        view_up,
        vertical_fov,
        gpu.configuration.width as f32 / gpu.configuration.height as f32,
    );
}

/// Pixel rectangles (x, y, width, height) the in-game HUD program (`hud.wgsl`) can draw
/// into. Outside a menu it draws only the crosshair and the damage ring and discards
/// every other pixel, so the full-screen pass is scissored to these instead of shading
/// the whole screen; `None` keeps the full screen.
pub(crate) type HudScissors = Option<[Option<[u32; 4]>; 2]>;

impl HudUniform {
    /// The rectangles for this uniform on a `width` x `height` target.
    pub(crate) fn scissors(&self, width: u32, height: u32) -> HudScissors {
        if self.menu_open > 0.5 || self.status_visible > 0.5 || width == 0 || height == 0 {
            return None;
        }
        let [w, h] = [width as f32, height as f32];
        // Shader UVs run right and up from the bottom-left corner; rows run down.
        let rectangle = |center: [f32; 2], half: [f32; 2]| -> Option<[u32; 4]> {
            let left = ((center[0] - half[0]) * w - 2.0).floor().clamp(0.0, w);
            let right = ((center[0] + half[0]) * w + 2.0).ceil().clamp(0.0, w);
            let top = ((1.0 - center[1] - half[1]) * h - 2.0)
                .floor()
                .clamp(0.0, h);
            let bottom = ((1.0 - center[1] + half[1]) * h + 2.0).ceil().clamp(0.0, h);
            (right > left && bottom > top).then_some([
                left as u32,
                top as u32,
                (right - left) as u32,
                (bottom - top) as u32,
            ])
        };
        let visible = self.hud_visible > 0.5;
        let mut crosshair = None;
        if visible && self.crosshair > 0.0 {
            let [x, y, scale_x, scale_y] = self.crosshair_parameters;
            // Degenerate scales would put the crosshair test everywhere.
            if !(scale_x > 0.0 && scale_y > 0.0 && x.is_finite() && y.is_finite()) {
                return None;
            }
            // The outline reaches 14/26 of the crosshair size on each axis.
            let reach = 14.0 / 26.0 * self.crosshair.max(0.001);
            crosshair = rectangle([0.5 + x, 0.5 + y], [reach / scale_x, reach / scale_y]);
        }
        let mut damage = None;
        if visible && self.damage_alpha > 0.0 {
            let outer = 0.092 + 0.004 * self.damage_strength;
            let aspect = self.inverse_height / self.inverse_width;
            if !(aspect > 0.0 && outer.is_finite()) {
                return None;
            }
            damage = rectangle([0.5, 0.5], [outer / aspect, outer]);
        }
        // A HUD draw runs the entire shader, so overlapping scissors would blend
        // the crosshair twice while taking damage. Draw their union once instead.
        if let (Some(a), Some(b)) = (crosshair, damage)
            && a[0] < b[0] + b[2]
            && b[0] < a[0] + a[2]
            && a[1] < b[1] + b[3]
            && b[1] < a[1] + a[3]
        {
            let left = a[0].min(b[0]);
            let top = a[1].min(b[1]);
            crosshair = Some([
                left,
                top,
                (a[0] + a[2]).max(b[0] + b[2]) - left,
                (a[1] + a[3]).max(b[1] + b[3]) - top,
            ]);
            damage = None;
        }
        Some([crosshair, damage])
    }
}

/// The saber shaders the player in `slot` wears, as their blades draw them here: the first
/// saber's, then the second saber's (or a staff's other half's) when they hold one
/// (`pair`, [`holds_pair`]). The local player's own (`own_slot`, gated by their unlocks)
/// or the looks' for another, each only when its pack is loaded. For the player card.
fn card_shaders(
    looks: &crate::looks::Looks,
    skins: &crate::saber_skins::LoadedSkins,
    own_slot: Option<usize>,
    slot: u8,
    pair: bool,
) -> [Option<&'static str>; 2] {
    let slot = usize::from(slot);
    let (first, second) = if own_slot == Some(slot) {
        (looks.own_saber_skin(), looks.own().second)
    } else {
        (looks.saber_skin_id(slot), looks.second_skin(slot))
    };
    let second = match second {
        crate::looks::SecondSkin::Same => first,
        crate::looks::SecondSkin::Own(id) => id,
    };
    let loaded = |id: Option<&'static str>| skins.get(id?).map(|skin| skin.id);
    [loaded(first), loaded(second.filter(|_| pair))]
}

/// Whether the player in `slot` holds something a second shader goes on, as the world
/// draws it (`saber_skins::skin_of_blade`): a second saber, or one saber with another half
/// (a staff's, not a crossguard's side blades).
fn holds_pair(
    game: &sjk_protocol::GameState,
    hilts: Option<&crate::saber::HiltCatalog>,
    slot: u8,
) -> bool {
    let [first, second] = sjk_client::legacy_client_saber_names(game, u16::from(slot));
    second.is_some()
        || first
            .zip(hilts)
            .is_some_and(|(name, hilts)| hilts.get_any_case(&name).has_other_half())
}

#[cfg(test)]
mod tests {
    use super::card_shaders;
    use crate::looks::{Looks, SecondSkin, Worn};
    use crate::saber_skins::{
        LoadedSkins,
        tests::{loaded_pair, loaded_sample},
    };

    fn look(saber: &str, saber2: Option<&str>) -> sjk_identity::Look {
        sjk_identity::Look {
            saber: saber.to_owned(),
            illuminate: false,
            saber_off: Vec::new(),
            toys: Vec::new(),
            saber2: saber2.map(str::to_owned),
            saber2_off: Vec::new(),
            saber_on: Vec::new(),
            saber2_on: Vec::new(),
            saber_sounds: String::new(),
            tag: None,
            finisher: String::new(),
        }
    }

    #[test]
    fn the_card_names_a_shader_only_when_it_draws_on_the_blade() {
        let loaded = loaded_sample(1);
        let card_shader = |looks: &Looks, skins: &LoadedSkins, own_slot, slot| {
            card_shaders(looks, skins, own_slot, slot, false)[0]
        };
        let tidal = look("saber_tidal", None);
        let mut looks = Looks::default();
        looks.replace_roster([(3, "Fox", Some(&tidal))]);
        looks.rebuild(|slot| (slot == 3).then(|| "Fox".to_owned()));
        // Another player, worn by the looks (which the hub relays only when owned).
        assert_eq!(card_shader(&looks, &loaded, None, 3), Some("saber_tidal"));
        // Nobody's look, or one whose pack is not loaded here.
        assert_eq!(card_shader(&looks, &loaded, None, 4), None);
        assert_eq!(
            card_shader(&looks, &LoadedSkins::of(Vec::new(), 1), None, 3),
            None
        );
        // The local player: their own look, gated by their unlocks.
        looks.set_own(Some(1), Worn::own("saber_tidal", "", |_| true, false));
        assert_eq!(
            card_shader(&looks, &loaded, Some(1), 1),
            Some("saber_tidal")
        );
        looks.set_own(Some(1), Worn::own("saber_tidal", "", |_| false, false));
        assert_eq!(card_shader(&looks, &loaded, Some(1), 1), None);
        looks.set_own(Some(1), Worn::own("", "", |_| true, false));
        assert_eq!(card_shader(&looks, &loaded, Some(1), 1), None);
    }

    #[test]
    fn the_card_names_the_second_sabers_shader_only_while_there_is_one() {
        let loaded = loaded_pair(1);
        let mut looks = Looks::default();
        let (same, own, stock) = (
            look("saber_tidal", None),
            look("saber_tidal", Some("saber_frost")),
            look("", Some("saber_frost")),
        );
        let unloaded = look("saber_tidal", Some("saber_glitch"));
        looks.replace_roster([
            (2, "Fox", Some(&same)),
            (3, "Kit", Some(&own)),
            (4, "Vix", Some(&stock)),
            (5, "Tod", Some(&unloaded)),
        ]);
        let names = ["", "", "Fox", "Kit", "Vix", "Tod"];
        looks.rebuild(|slot| {
            names
                .get(usize::from(slot))
                .filter(|name| !name.is_empty())
                .map(|name| (*name).to_owned())
        });
        let shaders = |slot, pair| card_shaders(&looks, &loaded, None, slot, pair);
        // No saber of its own: the second wears the first's.
        assert_eq!(shaders(2, true), [Some("saber_tidal"); 2]);
        assert_eq!(shaders(3, true), [Some("saber_tidal"), Some("saber_frost")]);
        // A stock first and a shader on the second.
        assert_eq!(shaders(4, true), [None, Some("saber_frost")]);
        // A second whose pack is not loaded here draws stock.
        assert_eq!(shaders(5, true), [Some("saber_tidal"), None]);
        // Nothing a second goes on: none, whatever the look says.
        assert_eq!(shaders(3, false), [Some("saber_tidal"), None]);
        assert_eq!(shaders(4, false), [None; 2]);
        // The local player: their own second, gated by their unlocks as the first.
        let mut worn = Worn::own("saber_tidal", "", |_| true, false);
        worn.second = SecondSkin::Own(Some("saber_frost"));
        looks.set_own(Some(1), worn);
        assert_eq!(
            card_shaders(&looks, &loaded, Some(1), 1, true),
            [Some("saber_tidal"), Some("saber_frost")]
        );
        worn.second = SecondSkin::Own(None);
        looks.set_own(Some(1), worn);
        assert_eq!(
            card_shaders(&looks, &loaded, Some(1), 1, true),
            [Some("saber_tidal"), None]
        );
    }
}
