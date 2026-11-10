//! World shots of the Collection screen ([`crate::console::collection_panel`]) over the
//! live duel6 in the SJK UI, on a made-up profile (two medals, the Sun blade owned and
//! worn, some achievements): from the main page, every tab, the model on the menu map's
//! stage behind Shaders (the Sun, then the locked Storm previewed on the saber), Toys
//! (the Illuminate holocron by it) and Nameplates (the plate over its head, in each
//! style); the same
//! in a 4:3 window; then from the game menu over a match, the model in the page's live
//! preview; and the page on its own as the classic menus open it. Then the hover cards:
//! one on a thing of each tab, and one on a tier of the Holocrons tab. The blade skins'
//! looks come from the packs `SJK_TEST_PACKS` names. Ignored like the other world shots.

use super::*;
use crate::player_menu::ReturnTarget;
use crate::profile_hub::{Screen, Tab};

/// White wisps from the emitter: a smoke ribbon, the stream swaying and curling, torn
/// along by its noise.
const WISPS: &str = r#"{
  "ribbon": {
    "color": [1.0, 1.0, 1.0], "opacity": 0.55, "rate": 40.0, "after": 3.5, "life": 3.0,
    "width": 0.6, "growth": 5.0, "rise": 9.0, "follow": 0.3, "sway": 4.0,
    "sway_rate": 0.4, "curl": 3.0, "detail": 0.8, "softness": 0.6, "idle": 1.0,
    "swinging": 0.0, "swing_speed": 120.0, "ignite": 1.5, "along": 0.02
  },
  "sparks": {
    "shader": "gfx/misc/spark4", "color": [1.6, 1.1, 0.7], "opacity": 1.0, "rate": 2.5,
    "ignite": 1.2, "ignite_rate": 14.0, "after": 0.8, "after_rate": 8.0, "burst": 5,
    "speed": 75.0, "speed_jitter": 0.6, "spread": 35.0, "aim": 1.0, "gravity": 420.0, "life": 0.45,
    "size": 0.3, "length": 5.0, "along": 0.02
  }
}"#;

/// The made-up grant.
fn sun() -> sjk_identity::Unlock {
    sjk_identity::Unlock {
        holo: 0,
        solar: 0,
        id: "saber_sun".to_owned(),
        granted: 1_791_336_225,
        note: "Thanks for testing the blades with us".to_owned(),
        count: 1,
        medal: None,
        card: None,
    }
}

/// A made-up profile with the achievements half done.
fn preview() -> crate::console::profile_panel::Preview {
    let mut preview = super::tests::profile_preview();
    let held: Vec<sjk_identity::Achievement> = crate::achievements::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| sjk_identity::Achievement {
            id: kind.id.to_owned(),
            progress: if index % 2 == 0 {
                kind.goal
            } else {
                kind.goal * (index as u64 % 7 + 1) / 9
            },
            goal: kind.goal,
            unlocked: if index % 2 == 0 { 1_791_336_225 } else { 0 },
            reward: None,
            name: None,
            description: None,
            group: None,
        })
        .collect();
    preview.standings = crate::achievements::standings(&held);
    preview
}

/// The Collection's page ran out of room.
fn overflowed(gpu: &GpuState) -> bool {
    gpu.console
        .as_ref()
        .is_some_and(crate::console::ViewerConsole::profile_pages_overflowed)
}

/// The part of a tab's name in a shot's file name.
fn part(tab: Tab) -> String {
    tab.label().to_lowercase().replace(' ', "-")
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_collection() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
            // Never the machine's own Discord.
            (crate::discord_presence::CVAR, "0"),
        ];
        for size in [[1920, 1080], [1440, 1080], [2560, 1080]] {
            let suffix = match size[0] {
                1920 => "",
                1440 => "-4x3",
                _ => "-21x9",
            };
            let Some((mut gpu, _profile)) = open(
                "maps/mp/duel6.bsp",
                size,
                Some(menu::ClientMenu::new(true, String::new())),
                &cvars,
            ) else {
                return;
            };
            // The shots have no hub: they own the unlocks, as a profile listing them would.
            gpu.looks.shot_owns_unlocks = true;
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_profile(preview());
                console.preview_collection(Some(vec![sun()]), 0.8);
                console.open_profile_hub_page(Tab::Medals, ReturnTarget::MainMenu);
            }
            assert_eq!(gpu.profile_hub_tab(), Some(Tab::Medals));
            for _ in 0..240 {
                let _ = frame(&mut gpu, 1);
                if crate::medals::art::decoded().is_some() {
                    break;
                }
            }
            for tab in &Screen::Collection.tabs()[..5] {
                gpu.profile_hub_show_for_shot(*tab);
                assert_eq!(gpu.profile_hub_tab(), Some(*tab));
                // The camera goes to the player's shot behind the model's tabs.
                let frames = if tab.index() >= 2 { 150 } else { 8 };
                gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                let name = format!("duel6-collection-{}{suffix}", part(*tab));
                println!("{}", shoot(&mut gpu, frames, &name).display());
                assert!(!overflowed(&gpu), "{tab:?}{suffix}");
            }
            if size[0] != 1920 {
                // The list (`ui_shaderList 1`) in the narrower or wider window.
                gpu.profile_hub_show_for_shot(Tab::Shaders);
                if let Some(console) = gpu.console.as_mut() {
                    let _ =
                        console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "1");
                }
                let name = format!("duel6-collection-shaders-list{suffix}");
                println!("{}", shoot(&mut gpu, 30, &name).display());
                assert!(!overflowed(&gpu), "the list{suffix}");
                if let Some(console) = gpu.console.as_mut() {
                    let _ =
                        console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
                }
                continue;
            }
            // Toys with the holocron lit (`toy_illuminate`): the switch is on.
            gpu.profile_hub_show_for_shot(Tab::Toys);
            gpu.illuminate.set_on(true);
            gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
            println!(
                "{}",
                shoot(&mut gpu, 60, "duel6-collection-toys-lit").display()
            );
            gpu.illuminate.set_on(false);
            // The nameplate over the model's head (the holo frame), without and with See
            // and share nameplates: the dotted line, then the gold corner and the bars.
            gpu.profile_hub_show_for_shot(Tab::Nameplates);
            for (share, name) in [
                ("0", "duel6-collection-nameplates"),
                ("1", "duel6-collection-nameplates-shared"),
            ] {
                if let Some(console) = gpu.console.as_mut() {
                    let _ = console.set_cvar("cg_nameplate", share);
                }
                gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                println!("{}", shoot(&mut gpu, 60, name).display());
                assert!(!overflowed(&gpu), "{name}");
            }
            // The other styles (`cg_nameplateStyle`), sharing: the capsule, the classic.
            for (style, name) in [
                ("1", "duel6-collection-nameplates-capsule"),
                ("2", "duel6-collection-nameplates-classic"),
            ] {
                if let Some(console) = gpu.console.as_mut() {
                    let _ = console.set_cvar(crate::hud::nameplate::STYLE_CVAR, style);
                }
                gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                println!("{}", shoot(&mut gpu, 60, name).display());
                assert!(!overflowed(&gpu), "{name}");
            }
            if let Some(console) = gpu.console.as_mut() {
                let _ = console.set_cvar("cg_nameplate", "0");
                let _ = console.set_cvar(crate::hud::nameplate::STYLE_CVAR, "0");
            }
            // A locked one, previewed on the saber. Rows are counted as a player holding no
            // Unique shader sees the list (the shots below hold none).
            gpu.profile_hub_show_for_shot(Tab::Shaders);
            let storm = 1 + crate::unlockables::Holdings::Known(&[])
                .listed()
                .position(|skin| skin.id == "saber_storm")
                .expect("the Storm blade");
            if let Some(console) = gpu.console.as_mut() {
                console.collection_shader_for_shot(storm);
            }
            println!(
                "{}",
                shoot(&mut gpu, 30, "duel6-collection-shaders-preview").display()
            );
            // The list (`ui_shaderList 1`), a few more owned, a Mythical chosen.
            if let Some(console) = gpu.console.as_mut() {
                let owned = [
                    "saber_sun",
                    "saber_void",
                    "saber_storm",
                    "saber_runic",
                    "saber_glitch",
                ]
                .map(|id| sjk_identity::Unlock {
                    id: id.to_owned(),
                    ..sun()
                })
                .to_vec();
                console.preview_collection(Some(owned), 0.8);
                let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "1");
                console.collection_shader_for_shot(2);
            }
            println!(
                "{}",
                shoot(&mut gpu, 30, "duel6-collection-shaders-list").display()
            );
            assert!(!overflowed(&gpu), "the list");
            // The Spectral owned and chosen: its six parts in three columns, its smoky
            // edge switched off (`cg_saberSkinOptions`), on the model too.
            let spectral = 1 + crate::unlockables::Holdings::Known(&[])
                .listed()
                .position(|skin| skin.id == "saber_spectral")
                .expect("the Spectral blade");
            if let Some(console) = gpu.console.as_mut() {
                let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
                let owned = ["saber_sun", "saber_spectral"]
                    .map(|id| sjk_identity::Unlock {
                        id: id.to_owned(),
                        ..sun()
                    })
                    .to_vec();
                console.preview_collection(Some(owned), 0.8);
                let _ = console.set_cvar(crate::saber_skin_options::CVAR, "saber_spectral.smoke");
                console.collection_shader_for_shot(spectral);
            }
            println!(
                "{}",
                shoot(&mut gpu, 60, "duel6-collection-shaders-parts").display()
            );
            assert!(!overflowed(&gpu), "the parts");
            // Turned by a drag: its tip swung towards the viewer and rolled.
            if let Some(console) = gpu.console.as_mut() {
                console.collection_saber_turn_for_shot(Some(
                    crate::console::collection_panel::SaberTurn {
                        yaw: -0.9,
                        roll: 1.1,
                    },
                ));
            }
            println!(
                "{}",
                shoot(&mut gpu, 20, "duel6-collection-shaders-turned").display()
            );
            if let Some(console) = gpu.console.as_mut() {
                console.collection_saber_turn_for_shot(None);
            }
            // The hub's economy known: a Kamino held three times points to Break down in
            // the Forge beside Equip; a locked Echo Base to Craft in the Forge.
            let row = |id: &str| {
                1 + crate::unlockables::Holdings::Known(&[])
                    .listed()
                    .position(|skin| skin.id == id)
                    .expect("the shader")
            };
            if let Some(console) = gpu.console.as_mut() {
                let owned = vec![
                    sun(),
                    sjk_identity::Unlock {
                        id: "saber_tidal".to_owned(),
                        count: 3,
                        ..sun()
                    },
                ];
                console.preview_collection(Some(owned), 0.8);
                console.collection_trade_for_shot(true);
                console.collection_shader_for_shot(row("saber_tidal"));
            }
            println!(
                "{}",
                shoot(&mut gpu, 20, "duel6-collection-shaders-shatter").display()
            );
            if let Some(console) = gpu.console.as_mut() {
                console.preview_collection(Some(vec![sun()]), 0.8);
                console.collection_shader_for_shot(row("saber_echo"));
            }
            println!(
                "{}",
                shoot(&mut gpu, 20, "duel6-collection-shaders-craft").display()
            );
            if let Some(console) = gpu.console.as_mut() {
                console.collection_trade_for_shot(false);
            }
            if let Some(console) = gpu.console.as_mut() {
                let _ = console.set_cvar(crate::saber_skin_options::CVAR, "");
                console.preview_collection(Some(vec![sun()]), 0.8);
            }
            // The largest menu text (`ui_textScale 1.4`): the wall, the list and the toys
            // still fit, rows and tiles grown, words wrapped.
            for (tab, view, name) in [
                (Tab::Shaders, "0", "duel6-collection-text14-wall"),
                (Tab::Shaders, "1", "duel6-collection-text14-list"),
                (Tab::Toys, "0", "duel6-collection-text14-toys"),
            ] {
                gpu.profile_hub_show_for_shot(tab);
                if let Some(console) = gpu.console.as_mut() {
                    let _ = console.set_cvar("ui_textScale", "1.4");
                    let _ =
                        console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, view);
                }
                println!("{}", shoot(&mut gpu, 30, name).display());
                assert!(!overflowed(&gpu), "{name}");
            }
            if let Some(console) = gpu.console.as_mut() {
                let _ = console.set_cvar("ui_textScale", "1");
                let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
            }
            // Ctrl+Tab goes on to Holocrons (the Holocrons page, under the same row), on
            // to the Forge and to Cards, on the same screen.
            gpu.profile_hub_show_for_shot(Tab::Nameplates);
            gpu.profile_hub_next_for_shot();
            assert_eq!(gpu.profile_hub_tab(), Some(Tab::Holocrons));
            println!(
                "{}",
                shoot(&mut gpu, 150, "duel6-collection-holocrons").display()
            );
            assert!(!overflowed(&gpu), "Holocrons");
            gpu.profile_hub_next_for_shot();
            assert_eq!(gpu.profile_hub_tab(), Some(Tab::Forge));
            gpu.profile_hub_next_for_shot();
            assert_eq!(gpu.profile_hub_tab(), Some(Tab::Cards));
        }
        // From the game menu, over a "match" (no server): the live preview.
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(false, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let shots = menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
        let (yaw, pitch) = look(shots[3].from, shots[3].at);
        aim(&mut gpu, shots[3].from, yaw, pitch);
        gpu.is_menu_world = false;
        gpu.game_menu = true;
        if let Some(console) = gpu.console.as_mut() {
            console.close_for_connection();
            console.preview_profile(preview());
            console.preview_collection(Some(vec![sun()]), 0.8);
        }
        // A match's view: `cg_fov 80` widened for 16:9 renders about 64 degrees up the
        // window (`GpuState::scope_fov`), narrower than the menu map's 90.
        gpu.field_of_view = 64.3;
        let _ = frame(&mut gpu, 20);
        gpu.open_collection_from_game();
        assert_eq!(gpu.profile_hub_tab(), Some(Tab::Medals));
        for tab in [Tab::Achievements, Tab::Shaders, Tab::Toys, Tab::Holocrons] {
            gpu.profile_hub_show_for_shot(tab);
            let name = format!("duel6-collection-in-game-{}", part(tab));
            println!("{}", shoot(&mut gpu, 40, &name).display());
            assert!(!overflowed(&gpu), "in game {tab:?}");
        }
        // Escape returns to the game menu, on Collection.
        if let Some(console) = gpu.console.as_mut() {
            console.close_profile_hub_page();
        }
        assert!(gpu.game_menu, "back on the game menu");
        assert_eq!(
            gpu.game_menu_row,
            crate::ingame_menu::sjk_view::Entry::Collection.index()
        );
        // On its own, as the classic menus' `collection` opens it.
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &[("ui_menuStyle", "classic")],
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection_alone(Tab::Shaders, Some(vec![sun()]), 0.8);
        }
        println!(
            "{}",
            shoot(&mut gpu, 30, "duel6-collection-classic").display()
        );
        assert!(!overflowed(&gpu), "on its own");
    });
}

/// The rarity effects (`rarity_fx`) in motion: the Shaders grid with every tier owned
/// at moments a third of a second apart over one sheen's period (the files
/// `duel6-rarity-grid-<n>`), the list at two moments, and the Holocrons tab.
/// The Loadouts card over the Collection (Lex's idea, 03/11/2026), two slots kept, at
/// 1080 lines and 4:3; the Shaders tab's Loadouts button under the tally first.
/// The Shaders tab's Finishers page (Sol, 04/11/2026): Disintegrate held and worn, at
/// 1080 lines and 4:3.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_finishers() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
            (crate::finishers::CVAR, "finisher_disintegrate"),
        ];
        let disintegrate = sjk_identity::Unlock {
            id: "finisher_disintegrate".to_owned(),
            ..sun()
        };
        for size in [[1920, 1080], [1440, 1080]] {
            let suffix = if size[0] == 1920 { "" } else { "-4x3" };
            let Some((mut gpu, _profile)) = open(
                "maps/mp/duel6.bsp",
                size,
                Some(menu::ClientMenu::new(true, String::new())),
                &cvars,
            ) else {
                return;
            };
            gpu.looks.shot_owns_unlocks = true;
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_profile(preview());
                console.preview_collection(Some(vec![sun(), disintegrate.clone()]), 0.8);
                console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
                console.preview_finishers();
            }
            gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
            let name = format!("duel6-collection-finishers{suffix}");
            println!("{}", shoot(&mut gpu, 120, &name).display());
            assert!(!overflowed(&gpu), "{suffix}");
        }
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_loadouts() {
    use crate::console::collection_panel::loadouts::{Request, Store};
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let mut store = Store::default();
        let look = |cvar: &str| match cvar {
            "saber1" => "single_1".to_owned(),
            "saber2" => "single_2".to_owned(),
            "cg_saberSkin" => "saber_sun".to_owned(),
            "cg_tag" => "gonk".to_owned(),
            _ => String::new(),
        };
        store.apply(&Request::Save(0), look);
        store.apply(&Request::Rename(0, "Duels".to_owned()), look);
        store.apply(&Request::Save(2), |cvar| match cvar {
            "saber1" => "dual_1".to_owned(),
            _ => String::new(),
        });
        for size in [[1920, 1080], [1440, 1080]] {
            let suffix = if size[0] == 1920 { "" } else { "-4x3" };
            let Some((mut gpu, _profile)) = open(
                "maps/mp/duel6.bsp",
                size,
                Some(menu::ClientMenu::new(true, String::new())),
                &cvars,
            ) else {
                return;
            };
            gpu.looks.shot_owns_unlocks = true;
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_profile(preview());
                console.preview_collection(Some(vec![sun()]), 0.8);
                console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
            }
            gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
            let name = format!("duel6-collection-loadouts-button{suffix}");
            println!("{}", shoot(&mut gpu, 120, &name).display());
            if let Some(console) = gpu.console.as_mut() {
                console.preview_loadouts(&store);
            }
            let name = format!("duel6-collection-loadouts{suffix}");
            println!("{}", shoot(&mut gpu, 8, &name).display());
            assert!(!overflowed(&gpu), "{suffix}");
        }
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_rarity_effects() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let owned: Vec<sjk_identity::Unlock> = crate::unlockables::ALL
            .iter()
            .filter(|skin| !matches!(skin.id, "saber_molten" | "saber_frost" | "saber_banner"))
            .map(|skin| sjk_identity::Unlock {
                id: skin.id.to_owned(),
                ..sun()
            })
            .collect();
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection(Some(owned.clone()), 0.0);
            console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        if let Some(console) = gpu.console.as_mut() {
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
            // Void, a Mythical, chosen: its white outline beside the worn Sun's gold.
            console.collection_shader_for_shot(2);
        }
        let _ = frame(&mut gpu, 150);
        for step in 0..12 {
            let seconds = step as f32 / 3.0;
            if let Some(console) = gpu.console.as_mut() {
                console.preview_collection(Some(owned.clone()), seconds);
            }
            let name = format!("duel6-rarity-grid-{step:02}");
            println!("{}", shoot(&mut gpu, 2, &name).display());
            assert!(!overflowed(&gpu), "the grid at {seconds}");
        }
        if let Some(console) = gpu.console.as_mut() {
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "1");
            console.collection_shader_for_shot(1);
        }
        for (step, seconds) in [0.4_f32, 2.2].into_iter().enumerate() {
            if let Some(console) = gpu.console.as_mut() {
                console.preview_collection(Some(owned.clone()), seconds);
            }
            let name = format!("duel6-rarity-list-{step}");
            println!("{}", shoot(&mut gpu, 2, &name).display());
            assert!(!overflowed(&gpu), "the list at {seconds}");
        }
        gpu.profile_hub_show_for_shot(Tab::Nameplates);
        gpu.profile_hub_next_for_shot();
        assert_eq!(gpu.profile_hub_tab(), Some(Tab::Holocrons));
        println!(
            "{}",
            shoot(&mut gpu, 150, "duel6-rarity-holocrons").display()
        );
        assert!(!overflowed(&gpu), "Holocrons");
    });
}

/// The hover cards: one on a thing of each tab (a medal, an achievement half done, a
/// locked blade skin, the toy, a card) and one on a tier of the Holocrons tab, each shot
/// alone and all in one sheet.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_hover_cards() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let cards = vec![sjk_identity::CardStack {
            card: None,
            rarity: "legendary".to_owned(),
            finish: "holo".to_owned(),
            edition: "alpha".to_owned(),
            count: 2,
            first: 1_791_641_100,
            last: 1_791_641_100,
        }];
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection(Some(vec![sun()]), 0.8);
            console.preview_cards(cards, 1_340, 85);
            console.open_profile_hub_page(Tab::Medals, ReturnTarget::MainMenu);
        }
        for _ in 0..240 {
            let _ = frame(&mut gpu, 1);
            if crate::medals::art::decoded().is_some() {
                break;
            }
        }
        let mut images = Vec::new();
        for (tab, index) in [
            (Tab::Medals, 2),
            (Tab::Achievements, 5),
            (Tab::Shaders, 3),
            (Tab::Toys, 0),
            (Tab::Cards, 0),
        ] {
            gpu.profile_hub_show_for_shot(tab);
            if let Some(console) = gpu.console.as_mut() {
                console.collection_hover_for_shot(Some(index));
            }
            let frames = if tab.index() >= 2 { 150 } else { 8 };
            let name = format!("duel6-hover-{}", part(tab));
            let path = shoot(&mut gpu, frames, &name);
            println!("{}", path.display());
            assert!(!overflowed(&gpu), "{tab:?}");
            images.push(image::open(path).expect("the shot").to_rgba8());
            if let Some(console) = gpu.console.as_mut() {
                console.collection_hover_for_shot(None);
            }
        }
        gpu.profile_hub_show_for_shot(Tab::Nameplates);
        gpu.profile_hub_next_for_shot();
        assert_eq!(gpu.profile_hub_tab(), Some(Tab::Holocrons));
        if let Some(console) = gpu.console.as_mut() {
            console.preview_holocrons(super::holocrons_tab::holdings(), 1);
            console.holocron_hover_for_shot(Some(3));
        }
        let path = shoot(&mut gpu, 150, "duel6-hover-holocrons");
        println!("{}", path.display());
        assert!(!overflowed(&gpu), "Holocrons");
        images.push(image::open(path).expect("the shot").to_rgba8());
        println!("{}", sheet(&images, 2, 960, "duel6-hover-sheet").display());
    });
}

/// The Blade Lab's swatches (15/10/2026) on the Shaders list, every shader owned: the
/// list scrolled to each of eleven in turn (the Reactor, Soap Bubble, Neon, Hourglass, Fiber
/// Optic, Monarch, Quantum, Sketch, Comic, Quicksilver and Obsidian Edge), held on the
/// stage, at two moments.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_lab_swatches() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_reactor"),
            ("name", "^1Sol^7Vulpes"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let owned: Vec<sjk_identity::Unlock> = crate::unlockables::ALL
            .iter()
            .map(|skin| sjk_identity::Unlock {
                id: skin.id.to_owned(),
                ..sun()
            })
            .collect();
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection(Some(owned.clone()), 0.0);
            console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
        }
        gpu.profile_hub_show_for_shot(Tab::Shaders);
        let row = |id: &str| {
            1 + crate::unlockables::blade_skins_by_tier()
                .iter()
                .position(|skin| skin.id == id)
                .expect("in the catalogue")
        };
        for id in [
            "saber_reactor",
            "saber_bubble",
            "saber_neon",
            "saber_hourglass",
            "saber_fiber",
            "saber_monarch",
            "saber_quantum",
            "saber_sketch",
            "saber_comic",
            "saber_quicksilver",
            "saber_obsidian",
        ] {
            if let Some(console) = gpu.console.as_mut() {
                console.collection_shader_for_shot(row(id));
            }
            let _ = frame(&mut gpu, 60);
            for seconds in [1.0_f32, 3.3] {
                if let Some(console) = gpu.console.as_mut() {
                    console.preview_collection(Some(owned.clone()), seconds);
                }
                let name = format!("duel6-lab-swatches-{}-{seconds}", &id[6..]);
                println!("{}", shoot(&mut gpu, 2, &name).display());
                assert!(!overflowed(&gpu), "{id} at {seconds}");
            }
        }
    });
}

/// The Collection with `debug_collectibles` on for a staff key: the made-up profile
/// dressed with every shader, medal, achievement and card (`debug_collectibles.rs`), on
/// the tabs that show them.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_collection_debug() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_quantum"),
            ("name", "^1Sol^7Vulpes"),
            (crate::debug_collectibles::CVAR, "1"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let mut preview = super::tests::profile_preview();
        if let Some(me) = preview.snapshot.me.as_mut() {
            me.staff = true;
        }
        preview.snapshot = crate::debug_collectibles::dressed(true, preview.snapshot);
        let me = preview.snapshot.me.clone().expect("the made-up profile");
        preview.standings = crate::achievements::standings(&me.achievements);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview);
            console.preview_collection(Some(me.unlocks.clone()), 0.8);
            console.open_profile_hub_page(Tab::Medals, ReturnTarget::MainMenu);
        }
        for _ in 0..240 {
            let _ = frame(&mut gpu, 1);
            if crate::medals::art::decoded().is_some() {
                break;
            }
        }
        for tab in [Tab::Medals, Tab::Achievements, Tab::Shaders, Tab::Cards] {
            gpu.profile_hub_show_for_shot(tab);
            let frames = if tab == Tab::Shaders { 150 } else { 30 };
            gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
            let name = format!("duel6-collection-debug-{}", part(tab));
            println!("{}", shoot(&mut gpu, frames, &name).display());
            assert!(!overflowed(&gpu), "{tab:?}");
        }
    });
}

/// Kyber crystals on the Achievements tab (21/10/2026): the achievement up close says the
/// crystal it brings, and its hover card ends with it, in the crystal's colour.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_kyber_achievements() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.open_profile_hub_page(Tab::Achievements, ReturnTarget::MainMenu);
        }
        let path = shoot(&mut gpu, 8, "duel6-kyber-achievements");
        println!("{}", path.display());
        assert!(!overflowed(&gpu));
        // The hover card of Legend of the Arena (a Mythical crystal), then Centurion.
        for id in ["kills_1000", "kills_100"] {
            let index = crate::achievements::ALL
                .iter()
                .position(|kind| kind.id == id)
                .expect("an achievement");
            if let Some(console) = gpu.console.as_mut() {
                console.collection_hover_for_shot(Some(index));
            }
            let path = shoot(&mut gpu, 8, &format!("duel6-kyber-achievement-hover-{id}"));
            println!("{}", path.display());
            assert!(!overflowed(&gpu), "{id}");
        }
    });
}

/// The Achievements tab's 35 (26/10/2026) in six categories, at 1080 lines and 4:3: the
/// made-up profile with Touched by the Force still a secret and the retired Surveyor
/// held, then the hover cards of the secret, of Surveyor and of a new client count
/// (`duel6-more-achievements-*`); then a profile without Surveyor, where it is gone.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_more_achievements() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let index = |id: &str| {
            crate::achievements::ALL
                .iter()
                .position(|kind| kind.id == id)
                .expect("an achievement")
        };
        let with = |surveyor: bool| {
            let mut preview = preview();
            let mut held: Vec<sjk_identity::Achievement> = preview
                .standings
                .iter()
                .map(|standing| sjk_identity::Achievement {
                    id: standing.kind.id.to_owned(),
                    progress: standing.progress,
                    goal: standing.kind.goal,
                    unlocked: standing.unlocked.unwrap_or(0),
                    reward: None,
                    name: None,
                    description: None,
                    group: None,
                })
                .collect();
            for achievement in &mut held {
                match achievement.id.as_str() {
                    "mythical_find" => (achievement.progress, achievement.unlocked) = (0, 0),
                    "surveyor" if surveyor => {
                        (achievement.progress, achievement.unlocked) = (5, 1_791_336_225);
                    }
                    "surveyor" => (achievement.progress, achievement.unlocked) = (3, 0),
                    _ => {}
                }
            }
            preview.standings = crate::achievements::Record::default().standings(&held);
            preview
        };
        for (size, label) in [([1920, 1080], "1080"), ([1440, 1080], "4x3")] {
            let Some((mut gpu, _profile)) = open(
                "maps/mp/duel6.bsp",
                size,
                Some(menu::ClientMenu::new(true, String::new())),
                &cvars,
            ) else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_profile(with(true));
                console.preview_collection(Some(vec![sun()]), 0.8);
                console.open_profile_hub_page(Tab::Achievements, ReturnTarget::MainMenu);
            }
            let path = shoot(&mut gpu, 8, &format!("duel6-more-achievements-{label}"));
            println!("{}", path.display());
            assert!(!overflowed(&gpu), "{label}");
            if label != "1080" {
                continue;
            }
            for id in ["mythical_find", "surveyor", "thermal_return"] {
                if let Some(console) = gpu.console.as_mut() {
                    console.collection_hover_for_shot(Some(index(id)));
                }
                let path = shoot(&mut gpu, 8, &format!("duel6-more-achievements-hover-{id}"));
                println!("{}", path.display());
                assert!(!overflowed(&gpu), "{id}");
            }
            if let Some(console) = gpu.console.as_mut() {
                console.collection_hover_for_shot(None);
                console.preview_profile(with(false));
                console.preview_collection(Some(vec![sun()]), 0.8);
            }
            let path = shoot(&mut gpu, 8, "duel6-more-achievements-no-surveyor");
            println!("{}", path.display());
            assert!(!overflowed(&gpu));
        }
    });
}

/// Frames [`duel6_blade_pictures_cost`] times a view.
const COST_FRAMES: usize = 200;
/// Rounds over its views.
const COST_ROUNDS: usize = 5;

/// What the Collection's live blade pictures (`blade_pictures.rs`) cost the menu frame
/// (25/10/2026): every shader owned, the Shaders grid (fifteen pictures) and list (six),
/// and Medals (none) for reference, each frame waited for on the GPU, the median of
/// [`COST_FRAMES`] frames, best of [`COST_ROUNDS`] rounds; then a shot of the grid and of
/// the list (`duel6-blade-pictures-<view>`). Wall-clock times of whole frames: only the
/// differences say something, and only on this machine.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_blade_pictures_cost() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::unlockables::SABER_SKIN_CVAR, "saber_sun"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let owned: Vec<sjk_identity::Unlock> = crate::unlockables::ALL
            .iter()
            .map(|skin| sjk_identity::Unlock {
                id: skin.id.to_owned(),
                ..sun()
            })
            .collect();
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection(Some(owned), 0.0);
            console.open_profile_hub_page(Tab::Shaders, ReturnTarget::MainMenu);
        }
        let views = [
            ("wall", Tab::Shaders, "0"),
            ("list", Tab::Shaders, "1"),
            ("medals", Tab::Medals, "0"),
        ];
        let show = |gpu: &mut GpuState, tab: Tab, view: &str| {
            gpu.profile_hub_show_for_shot(tab);
            if let Some(console) = gpu.console.as_mut() {
                let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, view);
            }
        };
        let mut times = [f64::MAX; 3];
        for _ in 0..COST_ROUNDS {
            for ((_, tab, view), best) in views.iter().zip(&mut times) {
                show(&mut gpu, *tab, view);
                let _ = frame(&mut gpu, 30);
                let mut frames: Vec<f64> = (0..COST_FRAMES)
                    .map(|_| {
                        let start = std::time::Instant::now();
                        let _ = gpu.render(&mut None);
                        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                        start.elapsed().as_secs_f64() * 1_000.0
                    })
                    .collect();
                frames.sort_by(f64::total_cmp);
                *best = best.min(frames[COST_FRAMES / 2]);
            }
        }
        let reference = times[2];
        for ((name, ..), median) in views.iter().zip(times) {
            println!(
                "{name:8} median {median:6.3} ms (best round), {:+6.3} ms over Medals",
                median - reference
            );
        }
        for (name, tab, view) in &views[..2] {
            show(&mut gpu, *tab, view);
            let path = shoot(&mut gpu, 30, &format!("duel6-blade-pictures-{name}"));
            println!("{}", path.display());
        }
    });
}

/// The Tags tab (27/10/2026) on the made-up profile with two copies of a Rare tag, one
/// of them Holo, and a Solar Gonk tag, and (31/10/2026) a hub that lets credits cards be
/// tags: a Holo and a Normal copy of SJK Alpha 001; `cg_tag` on the Rare tag in Holo.
/// The free ones held, the rest locked; the wall scrolled to show the cards too. The
/// pictures come from the packs `SJK_TEST_PACKS` names (the hub's `assets/dist`, with
/// `sjk_tags.pk3` and `sjk_cards.pk3`).
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_collection_tags() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::tags::CVAR, "tag_ez"),
            (crate::tags::FINISH_CVAR, "holo"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        let ez = sjk_identity::Unlock {
            id: "tag_ez".to_owned(),
            count: 2,
            holo: 1,
            ..sun()
        };
        let gonk = sjk_identity::Unlock {
            id: "tag_gonk".to_owned(),
            count: 1,
            solar: 1,
            ..sun()
        };
        let mut profile = preview();
        if let Some(me) = profile.snapshot.me.as_mut() {
            me.tag_cards = vec!["credits".to_owned()];
            me.cards = [("holo", 1), ("normal", 2)]
                .map(|(finish, count)| sjk_identity::CardStack {
                    card: Some("sjk_alpha_001".to_owned()),
                    rarity: "mythical".to_owned(),
                    finish: finish.to_owned(),
                    edition: "alpha".to_owned(),
                    count,
                    first: 1_791_336_225,
                    last: 1_791_336_225,
                })
                .to_vec();
        }
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(profile);
            console.preview_collection(Some(vec![sun(), ez, gonk]), 0.8);
            console.open_profile_hub_page(Tab::Tags, ReturnTarget::MainMenu);
        }
        assert_eq!(gpu.profile_hub_tab(), Some(Tab::Tags));
        println!("{}", shoot(&mut gpu, 20, "duel6-collection-tags").display());
        assert!(!overflowed(&gpu));
        // The card tag chosen, its Holo shown, the wall scrolled down to it.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_tag_for_shot("sjk_alpha_001", 1);
        }
        println!(
            "{}",
            shoot(&mut gpu, 20, "duel6-collection-tags-card").display()
        );
        assert!(!overflowed(&gpu));
    });
}

/// The Tags tab's GIF tags (10/11/2026) on the made-up profile, verified, with GIF tags
/// on: a blank tag held and one GIF tag made (a made-up animation standing in for its
/// GIPHY GIF). First the GIF tag's card, then the blank tag's with a link pasted and Make
/// pressed once.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_collection_gif_tags() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::tags::GIF_SHOW_CVAR, "1"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        crate::chat_gifs::set_walls(true);
        let made = crate::chat_gifs::GifId::new("TestGif1").unwrap();
        let pasted = crate::chat_gifs::GifId::new("TestGif2").unwrap();
        let animation = crate::chat_gifs::decode::test_animation([320, 200], 8, 120);
        crate::chat_gifs::insert_wall_for_shot(&made, &animation, 360);
        crate::chat_gifs::insert_wall_for_shot(&pasted, &animation, 600);
        let blank = sjk_identity::Unlock {
            id: crate::tags::BLANK.to_owned(),
            count: 1,
            ..sun()
        };
        let mut profile = preview();
        if let Some(me) = profile.snapshot.me.as_mut() {
            me.verified = true;
            me.gif_tags = Some(vec![sjk_identity::GifTag {
                id: "TestGif1".to_owned(),
                made: 1_794_300_000,
            }]);
        }
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(profile);
            console.preview_collection(Some(vec![sun(), blank]), 0.8);
            console.open_profile_hub_page(Tab::Tags, ReturnTarget::MainMenu);
        }
        // The wall lists the GIF tag once a frame drew it.
        let _ = frame(&mut gpu, 2);
        if let Some(console) = gpu.console.as_mut() {
            console.collection_tag_for_shot("gif:TestGif1", 0);
        }
        println!(
            "{}",
            shoot(&mut gpu, 20, "duel6-collection-gif-tag").display()
        );
        assert!(!overflowed(&gpu));
        if let Some(console) = gpu.console.as_mut() {
            console.collection_tag_for_shot(crate::tags::BLANK, 0);
        }
        let _ = frame(&mut gpu, 2);
        if let Some(console) = gpu.console.as_mut() {
            console.collection_gif_link_for_shot(pasted);
        }
        println!(
            "{}",
            shoot(&mut gpu, 20, "duel6-collection-blank-tag").display()
        );
        assert!(!overflowed(&gpu));
    });
}

/// The History tab (31/10/2026) on made-up lines: an Open all of five gathered (opened on
/// its lines), single openings with a card and a shader, drops, a purchase, a shatter, a
/// mystery box and a trade, then Load older.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_collection_history() {
    use crate::console::collection_panel::history::tests::{came, ledger, opened};
    use sjk_identity::history::{BoxLine, HistoryCard, HistoryLine, OpenedLine, TradeLine};
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        let now = 1_793_440_000;
        let mut lines = vec![
            HistoryLine::Opened(OpenedLine {
                at: now,
                tier: "legendary".into(),
                credits: 240,
                card: Some(HistoryCard {
                    card: Some("sjk_alpha_000".into()),
                    rarity: "legendary".into(),
                    finish: "holo".into(),
                }),
                unlock: "saber_sun".into(),
                ..OpenedLine::default()
            }),
            {
                let HistoryLine::Came(mut bought) = came(now - 60, "rare", "bought") else {
                    unreachable!()
                };
                bought.price = 400;
                HistoryLine::Came(bought)
            },
        ];
        lines.extend((0..5).map(|index| {
            opened(
                now - 3_600,
                ["uncommon", "rare", "uncommon", "mythical", "rare"][index],
                40 + index as i64 * 15,
                if index == 3 { 12 } else { 0 },
            )
        }));
        lines.extend([
            HistoryLine::Shards(ledger(now - 7_200, 15, "shatter", "saber_sun")),
            HistoryLine::Box(BoxLine {
                at: now - 9_000,
                outcome: "double".into(),
                price: 100,
                won: 200,
                shards: 0,
            }),
            HistoryLine::Box(BoxLine {
                at: now - 9_100,
                outcome: "explode".into(),
                price: 100,
                ..BoxLine::default()
            }),
            HistoryLine::Trade(TradeLine {
                at: now - 86_400,
                trade: 3,
                with: sjk_identity::history::HistoryPartner {
                    name: "^1Nib^7bles".into(),
                    verified: true,
                },
                gave: vec![sjk_identity::TradeItem {
                    kind: "holocron".into(),
                    tier: "rare".into(),
                    ..Default::default()
                }],
                got: vec![sjk_identity::TradeItem {
                    kind: "shader".into(),
                    id: Some("saber_sun".into()),
                    tier: "mythical".into(),
                    ..Default::default()
                }],
            }),
            came(now - 90_000, "kyber_rare", "achievement"),
            came(now - 100_000, "rare", "play"),
            came(now - 100_000, "uncommon", "play"),
            HistoryLine::Credits(ledger(now - 120_000, 500, "reward", "Bug hunting week")),
            came(now - 130_000, "mythical", "staff"),
        ]);
        let state = sjk_identity::HistoryState {
            serial: 1,
            lines,
            more: true,
            before: now - 130_000,
            message: String::new(),
        };
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection(Some(vec![sun()]), 0.8);
            console.preview_history(Some(std::sync::Arc::new(state)));
            console.open_profile_hub_page(Tab::History, ReturnTarget::MainMenu);
        }
        assert_eq!(gpu.profile_hub_tab(), Some(Tab::History));
        let _ = frame(&mut gpu, 2);
        // Open the gathered row (the fourth) and choose it.
        if let Some(console) = gpu.console.as_mut() {
            console.history_activate_for_shot(3);
        }
        println!(
            "{}",
            shoot(&mut gpu, 20, "duel6-collection-history").display()
        );
        assert!(!overflowed(&gpu));
    });
}

/// The Toy Master's "?" on the Toys tab (30/10/2026): plain, then with a toy found, its
/// hover card naming it, and the found toy's medallion on the Achievements wall, in a
/// category of its own after the catalogue's.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_toy_master() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.preview_collection(Some(vec![sun()]), 0.8);
            console.open_profile_hub_page(Tab::Toys, ReturnTarget::MainMenu);
        }
        let mut images = Vec::new();
        gpu.profile_hub_show_for_shot(Tab::Toys);
        if let Some(console) = gpu.console.as_mut() {
            console.collection_hover_for_shot(Some(10));
        }
        let path = shoot(&mut gpu, 60, "duel6-toy-master-none");
        println!("{}", path.display());
        images.push(image::open(path).expect("the shot").to_rgba8());
        let mut found = preview();
        let held = sjk_identity::Achievement {
            id: "secret_shot".to_owned(),
            progress: 1,
            goal: 1,
            unlocked: 1_791_641_100,
            reward: Some("kyber_rare".to_owned()),
            name: Some("Found in a shot".to_owned()),
            description: Some("You found the Toy Master's toy of the shots.".to_owned()),
            group: Some(sjk_identity::wire::TOY_MASTER.to_owned()),
        };
        let mut all: Vec<sjk_identity::Achievement> = found
            .standings
            .iter()
            .map(|standing| sjk_identity::Achievement {
                id: standing.kind.id.to_owned(),
                progress: standing.progress,
                goal: standing.kind.goal,
                unlocked: standing.unlocked.unwrap_or(0),
                reward: None,
                name: None,
                description: None,
                group: None,
            })
            .collect();
        all.push(held);
        found.standings = crate::achievements::standings(&all);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(found);
            console.preview_collection(Some(vec![sun()]), 0.8);
            console.collection_hover_for_shot(Some(10));
        }
        let path = shoot(&mut gpu, 30, "duel6-toy-master-found");
        println!("{}", path.display());
        images.push(image::open(path).expect("the shot").to_rgba8());
        gpu.profile_hub_show_for_shot(Tab::Achievements);
        if let Some(console) = gpu.console.as_mut() {
            console.collection_hover_for_shot(Some(crate::achievements::ALL.len()));
        }
        let path = shoot(&mut gpu, 30, "duel6-toy-master-wall");
        println!("{}", path.display());
        assert!(!overflowed(&gpu), "the wall");
        images.push(image::open(path).expect("the shot").to_rgba8());
        println!(
            "{}",
            sheet(&images, 1, 960, "duel6-toy-master-sheet").display()
        );
    });
}

/// The Saber Lab tab over the live duel6: the model on the left of the picture holding the
/// lab blade (the test sample, from a folder of its own), the same saber floating on the
/// right over the choices; standing in the medium style, then swinging in each style.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_saber_lab() {
    on_big_stack(|| {
        let folder = tempfile::tempdir().unwrap();
        let blades = folder.path().join(crate::blade_skin_file::FOLDER);
        std::fs::create_dir_all(&blades).unwrap();
        std::fs::write(
            blades.join("saber_lab.bladeskin"),
            crate::blade_skin_file::tests::SAMPLE,
        )
        .unwrap();
        // And white wisps of smoke from its emitter.
        std::fs::write(blades.join("saber_lab.bladeextra6"), WISPS).unwrap();
        // Or, when SJK_SHOT_LAB names a lab folder, its lab blade's files instead: copies,
        // so the shot's edits never touch that folder.
        if let Some(folder) = std::env::var_os("SJK_SHOT_LAB") {
            let theirs = std::path::Path::new(&folder).join(crate::blade_skin_file::FOLDER);
            for entry in std::fs::read_dir(&theirs).into_iter().flatten().flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("saber_lab.") {
                    std::fs::copy(entry.path(), blades.join(&name)).unwrap();
                }
            }
        }
        crate::saber_lab::start(folder.path()).expect("the lab's folder");
        // Mounted after the lab started, so its folder lies over the packs.
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1920, 1080],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            crate::saber_lab::stop();
            return;
        };
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(preview());
            console.open_profile_hub_page(Tab::SaberLab, ReturnTarget::MainMenu);
            console.collection_lab_for_shot(1, false);
            // The tab shows the blade worn: the lab's, equipped as its Equip does.
            crate::unlockables::wear(console, crate::saber_lab::ID);
        }
        assert_eq!(gpu.profile_hub_tab(), Some(Tab::SaberLab));
        println!(
            "{}",
            shoot(&mut gpu, 150, "duel6-saber-lab-stand").display()
        );
        assert!(!overflowed(&gpu), "standing");
        // Swinging, the blades trail as in a match; the floating one shows its still trail.
        for (style, name) in [(0, "fast"), (1, "medium"), (2, "strong")] {
            if let Some(console) = gpu.console.as_mut() {
                console.collection_lab_for_shot(style, true);
                console.collection_lab_trail_for_shot(true);
            }
            for (frames, moment) in [(25, "a"), (25, "b")] {
                let shot = format!("duel6-saber-lab-{name}-{moment}");
                println!("{}", shoot(&mut gpu, frames, &shot).display());
            }
        }
        // The editor open on the lab's blade, then its glow turned red as an edit does.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_for_shot(1, false);
            console.collection_lab_edit_for_shot(crate::saber_lab::ID);
        }
        println!(
            "{}",
            shoot(&mut gpu, 30, "duel6-saber-lab-editor").display()
        );
        if let Some(console) = gpu.console.as_mut() {
            for (path, value) in [
                ("corona.rim_cool.0", 1.0),
                ("corona.rim_cool.1", 0.05),
                ("corona.rim_cool.2", 0.0),
                ("corona.rim_hot.0", 1.0),
                ("corona.rim_hot.1", 0.3),
                ("corona.rim_hot.2", 0.1),
                ("corona.inner.0", 1.0),
                ("corona.inner.1", 0.2),
                ("corona.inner.2", 0.1),
            ] {
                console.collection_lab_edit_set_for_shot(path, value);
            }
        }
        println!(
            "{}",
            shoot(&mut gpu, 30, "duel6-saber-lab-editor-edited").display()
        );
        // The sections it may add, then black holes added.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_edit_adds_for_shot(None);
        }
        println!(
            "{}",
            shoot(&mut gpu, 10, "duel6-saber-lab-editor-adds").display()
        );
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_edit_adds_for_shot(Some("holes"));
        }
        println!(
            "{}",
            shoot(&mut gpu, 30, "duel6-saber-lab-editor-holes").display()
        );
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_for_shot(2, true);
        }
        // A blade the made-up profile does not own (it holds the Sun), previewed: the
        // rarest other of the catalogue.
        let rarest = crate::unlockables::blade_skins_by_tier()
            .iter()
            .find(|skin| skin.id != sun().id)
            .expect("a blade skin besides the Sun");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_blade_for_shot(rarest.id);
        }
        println!(
            "{} ({})",
            shoot(&mut gpu, 30, "duel6-saber-lab-preview").display(),
            rarest.id
        );
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_blade_for_shot(crate::saber_lab::ID);
        }
        // Round the model and over it, then nearer from behind; and the blade going out.
        for (view, name) in [
            (
                crate::console::collection_panel::LabView {
                    turn: 1.2,
                    tilt: 0.7,
                    distance: 1.0,
                },
                "over",
            ),
            (
                crate::console::collection_panel::LabView {
                    turn: 3.0,
                    tilt: 0.15,
                    distance: 0.6,
                },
                "behind",
            ),
        ] {
            if let Some(console) = gpu.console.as_mut() {
                console.collection_lab_view_for_shot(view, true);
            }
            let shot = format!("duel6-saber-lab-{name}");
            println!("{}", shoot(&mut gpu, 60, &shot).display());
        }
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_for_shot(1, false);
            console.collection_lab_view_for_shot(Default::default(), true);
        }
        let _ = frame(&mut gpu, 60);
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_view_for_shot(Default::default(), false);
        }
        // With the flourish: put away, then away and holstered, then drawn again.
        println!(
            "{}",
            shoot(&mut gpu, 10, "duel6-saber-lab-retracting").display()
        );
        println!(
            "{}",
            shoot(&mut gpu, 90, "duel6-saber-lab-holstered").display()
        );
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_view_for_shot(Default::default(), true);
        }
        println!(
            "{}",
            shoot(&mut gpu, 12, "duel6-saber-lab-igniting").display()
        );
        // Behind the model as in a match, swinging; then looking down on it there.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_chase_for_shot(true);
            console.collection_lab_for_shot(2, true);
        }
        for (frames, moment) in [(60, "a"), (20, "b")] {
            let shot = format!("duel6-saber-lab-third-person-{moment}");
            println!("{}", shoot(&mut gpu, frames, &shot).display());
        }
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_view_for_shot(
                crate::console::collection_panel::LabView {
                    turn: 0.0,
                    tilt: 0.5,
                    distance: 1.0,
                },
                true,
            );
        }
        println!(
            "{}",
            shoot(&mut gpu, 50, "duel6-saber-lab-third-person-over").display()
        );
        // The emitter's smoke: standing lit, then put out (still smoking), then gone.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_chase_for_shot(false);
            console.collection_lab_for_shot(1, false);
            console.collection_lab_trail_for_shot(false);
            console.collection_lab_view_for_shot(
                crate::console::collection_panel::LabView {
                    turn: 0.0,
                    tilt: 0.0,
                    distance: 0.6,
                },
                true,
            );
        }
        println!(
            "{}",
            shoot(&mut gpu, 150, "duel6-saber-lab-smoke-lit").display()
        );
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_view_for_shot(
                crate::console::collection_panel::LabView {
                    turn: 0.0,
                    tilt: 0.0,
                    distance: 0.6,
                },
                false,
            );
        }
        // A skin's emitter sputtering once the blade has drawn in (`flash.die`, about a
        // second on), then still smoking.
        for (frames, moment) in [(48, "a"), (2, "b"), (2, "c"), (2, "d")] {
            let shot = format!("duel6-saber-lab-sputter-{moment}");
            println!("{}", shoot(&mut gpu, frames, &shot).display());
        }
        println!(
            "{}",
            shoot(&mut gpu, 1, "duel6-saber-lab-smoke-after").display()
        );
        println!(
            "{}",
            shoot(&mut gpu, 300, "duel6-saber-lab-smoke-gone").display()
        );
        crate::saber_lab::stop();
    });
}

/// The Saber Lab tab's sounds over the live duel6, mixed by the game's own mixer without a
/// device: the swings the model's animations cue, and the blade put out and lit again.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_saber_lab_sounds() {
    on_big_stack(|| {
        let folder = tempfile::tempdir().unwrap();
        let blades = folder.path().join(crate::blade_skin_file::FOLDER);
        std::fs::create_dir_all(&blades).unwrap();
        std::fs::write(
            blades.join("saber_lab.bladeskin"),
            crate::blade_skin_file::tests::SAMPLE,
        )
        .unwrap();
        crate::saber_lab::start(folder.path()).expect("the lab's folder");
        super::saber_skins::mount_test_packs();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
        ];
        let Some((mut gpu, _profile)) = open(
            "maps/mp/duel6.bsp",
            [1280, 720],
            Some(menu::ClientMenu::new(true, String::new())),
            &cvars,
        ) else {
            crate::saber_lab::stop();
            return;
        };
        let (output, mut source) = crate::audio_output::AudioOutput::test_output();
        let mut audio = Some(crate::GameAudio::with_output(output));
        let mut loudest = |gpu: &mut GpuState, audio: &mut Option<crate::GameAudio>, frames| {
            let mut loudest = 0.0_f32;
            for _ in 0..frames {
                let _ = gpu.render(audio);
                std::thread::sleep(std::time::Duration::from_millis(16));
                for _ in 0..1_500 {
                    loudest = loudest.max(source.next().unwrap_or(0.0).abs());
                }
            }
            loudest
        };
        let quiet = loudest(&mut gpu, &mut audio, 60);
        println!("before: {quiet}");
        if let Some(console) = gpu.console.as_mut() {
            console.open_profile_hub_page(Tab::SaberLab, ReturnTarget::MainMenu);
            console.collection_lab_for_shot(2, true);
        }
        let swings = loudest(&mut gpu, &mut audio, 240);
        assert_eq!(quiet, 0.0, "the menu is silent before");
        println!("swings: {swings}");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_for_shot(2, false);
        }
        let _ = loudest(&mut gpu, &mut audio, 60);
        if let Some(console) = gpu.console.as_mut() {
            console.collection_lab_view_for_shot(Default::default(), false);
        }
        let off = loudest(&mut gpu, &mut audio, 60);
        println!("off: {off}");
        crate::saber_lab::stop();
        assert!(swings > 0.01, "no swing heard");
        assert!(off > 0.01, "no retraction heard");
    });
}
