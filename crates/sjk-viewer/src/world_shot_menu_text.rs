//! World shots of the Collection's menu text fixes of 10/11/2026 (`docs/sjk-ui.md`,
//! Collection): the Cards tab's headline, the Forge's section chips, "i" chip and
//! balances, the Shaders tab's New sticker and "?" note, a full shader card (the Sun
//! blade) with its Locked features chip and note, and the Nameplates tab with Showcase
//! current set and its card. At 1080p with `ui_textScale 1.2` and `ui_letterSpacing
//! 0.06`, where labels measured without the letter spacing were cut or ran into each
//! other. Off-screen renders, ignored like the other world shots: they need a GPU adapter,
//! the installed game data named by `JKA_GAME_DATA` and the hub's packs in
//! `SJK_TEST_PACKS` (the card faces, the Sun blade's extras).
//!
//! Run: `JKA_GAME_DATA=<GameData> SJK_TEST_PACKS=<packs> cargo test --release -p
//! sjk-viewer menu_text_shots -- --ignored --test-threads=1 --nocapture`; the pictures are
//! in `target/world-shots`.

use super::*;
use crate::console::collection_panel::forge::{self, Section};
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;

/// The SJK menu over duel6 in the text style the fixes were made for.
fn cvars() -> [(&'static str, &'static str); 6] {
    [
        // No Discord status from a test.
        (crate::discord_presence::CVAR, "0"),
        ("ui_menuStyle", "sjk"),
        (crate::settings::quick::HIDE_CVAR, "1"),
        ("name", "^1Sol^7Vulpes"),
        (crate::text::style::SCALE_CVAR, "1.2"),
        (crate::text::style::TRACKING_CVAR, "0.06"),
    ]
}

/// A made-up copy of shader `id`.
fn held(id: &str) -> sjk_identity::Unlock {
    sjk_identity::Unlock {
        holo: 0,
        solar: 0,
        id: id.to_owned(),
        granted: 1_791_336_225,
        note: String::new(),
        count: 1,
        medal: None,
        card: None,
    }
}

/// The shaders held: the Sun blade (worn, its extras locked: no Solar copy of its card)
/// and others, all but the Sun blade New.
const HELD: [&str; 8] = [
    "saber_sun",
    "saber_phoenix",
    "saber_tidal",
    "saber_circuit",
    "saber_aurora",
    "saber_frost",
    "saber_sakura",
    "saber_glitch",
];

/// Card `n` of SJK Alpha, of `rarity`, in its normal finish.
fn alpha(n: u32, rarity: &str) -> sjk_identity::CardStack {
    sjk_identity::CardStack {
        card: Some(format!("sjk_alpha_{n:03}")),
        rarity: rarity.to_owned(),
        finish: "normal".to_owned(),
        edition: "alpha".to_owned(),
        count: 1,
        first: 1_791_641_100,
        last: 1_791_641_100,
    }
}

/// Open the Collection on `tab`, a frame later shown.
fn show(gpu: &mut GpuState, tab: Tab) {
    if let Some(console) = gpu.console.as_mut() {
        console.open_profile_hub_page(tab, ReturnTarget::MainMenu);
    }
    gpu.profile_hub_show_for_shot(tab);
    let _ = frame(gpu, 2);
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_menu_text_shots() {
    on_big_stack(|| {
        super::saber_skins::mount_test_packs();
        let menu = menu::ClientMenu::new(true, String::new());
        let Some((mut gpu, profile)) =
            open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars())
        else {
            return;
        };
        // The window's profile folder lives as long as the test.
        std::mem::forget(profile);
        gpu.looks.shot_owns_unlocks = true;
        let _ = frame(&mut gpu, 10);
        let keep = |gpu: &mut GpuState, frames: u32, name: &str| {
            let path = shoot(gpu, frames, &format!("duel6-menu-text-{name}"));
            println!("{}", path.display());
        };
        if let Some(console) = gpu.console.as_mut() {
            console.preview_profile(super::tests::profile_preview());
            console.preview_collection(Some(HELD.iter().map(|id| held(id)).collect()), 1.3);
            console.preview_cards(
                vec![
                    alpha(0, "mythical"),
                    alpha(4, "legendary"),
                    alpha(10, "legendary"),
                ],
                1_340,
                5_698,
            );
            console.preview_forge(Some(forge::sample_data(5_698, 1_340, 5 * 3_600, true)));
            console.collection_marks_for_shot(Some(crate::shader_marks::Marks::shown(
                &[],
                &[],
                &HELD[1..],
                Some("saber_sun"),
            )));
            console.collection_trade_for_shot(true);
            let _ = console.set_cvar(crate::console::collection_panel::SHADER_VIEW_CVAR, "0");
        }
        // The Cards tab: the headline, the sets' line and (before) the balances.
        show(&mut gpu, Tab::Cards);
        keep(&mut gpu, 30, "cards");
        // The Forge: the section chips and the balances, on the Shop and on Craft.
        if let Some(console) = gpu.console.as_mut() {
            console.forge_section_for_shot(Section::Shop);
        }
        show(&mut gpu, Tab::Forge);
        keep(&mut gpu, 30, "forge-shop");
        if let Some(console) = gpu.console.as_mut() {
            console.forge_section_for_shot(Section::Craft);
        }
        keep(&mut gpu, 30, "forge-craft");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_hover_for_shot(Some(usize::from(forge::INFO_TOKEN)));
        }
        keep(&mut gpu, 30, "forge-craft-info");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_hover_for_shot(None);
        }
        // The Shaders tab: the headline's New count, the Sun blade's full card.
        show(&mut gpu, Tab::Shaders);
        // The Sun blade, the first Mythical (the one held listed first), after the stock
        // blade.
        if let Some(console) = gpu.console.as_mut() {
            console.collection_shader_for_shot(1);
        }
        keep(&mut gpu, 30, "shaders-sun");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_notes_for_shot(true, false, false);
        }
        keep(&mut gpu, 30, "shaders-new-note");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_notes_for_shot(false, true, false);
        }
        keep(&mut gpu, 30, "shaders-locked-note");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_notes_for_shot(false, false, false);
        }
        // The Nameplates tab: the plate (before, over the model's head) or Showcase
        // current set and its card.
        show(&mut gpu, Tab::Nameplates);
        keep(&mut gpu, 30, "nameplates");
        if let Some(console) = gpu.console.as_mut() {
            console.collection_notes_for_shot(false, false, true);
        }
        keep(&mut gpu, 30, "nameplates-showcase");
    });
}
