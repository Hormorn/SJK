//! World shots of the Profile screen's Holocrons tab ([`crate::console::holocrons_panel`])
//! over the live duel6 in the SJK UI, with the 3D holocron ([`crate::holocrons::stage`])
//! floating before the backdrop camera: made-up counts (a tier held, one held none of),
//! each tier chosen in turn, the four holocrons up close (held at two moments, and
//! locked), then a tier with the identity off, a 4:3, a 21:9 and a 4K
//! window, the page over the classic menus (where the world must still show) and the
//! page in a window with `ui_textScale 1.2`. They are off-screen renders (no game
//! window), ignored like the other world shots: they need a GPU adapter and the
//! installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer holocrons_tab -- --ignored
//! --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.

use super::*;
use crate::console::holocrons_panel::{Data, Recent, State};
use crate::profile_hub::Tab;

/// Made-up holdings: 12 uncommon, 3 rare, no legendary, 1 mythical; ten recent ones
/// (a gift with a long note among them), the next one a third of the way, 3 of 8 today.
pub(super) fn holdings() -> Data {
    let recent = |tier: usize, when: &str, note: Option<&str>| Recent {
        tier,
        when: when.to_owned(),
        gift: note.is_some(),
        note: note.unwrap_or_default().to_owned(),
        opened: false,
        trade: None,
        edition: if tier == 3 { "Beta" } else { "Alpha" }.to_owned(),
    };
    Data {
        counts: Some([12, 3, 0, 1]),
        recent: vec![
            recent(0, "10/10/2026 14:05", None),
            recent(
                3,
                "10/10/2026 09:12",
                Some("For finding the fog bug on duel6 and telling us"),
            ),
            recent(1, "09/10/2026 22:48", None),
            recent(0, "09/10/2026 20:15", None),
            recent(0, "09/10/2026 18:30", None),
            recent(1, "08/10/2026 23:01", None),
            recent(0, "08/10/2026 21:44", None),
            recent(1, "08/10/2026 19:09", None),
            recent(0, "07/10/2026 22:30", None),
            recent(0, "07/10/2026 20:02", None),
        ],
        next: "Next holocron in about 20 minutes of play".to_owned(),
        pace: String::new(),
        today: Some((3, 8)),
        fraction: Some(1.0 / 3.0),
        every: Some(1_800),
        can_open: true,
        ..Data::nothing(State::Known)
    }
}

/// Open the main menu over duel6 in `size`, with the tab on the Profile screen.
fn open_tab(size: [u32; 2], cvars: &[(&str, &str)], data: Data) -> Option<GpuState> {
    let menu = menu::ClientMenu::new(true, String::new());
    let (mut gpu, profile) = open("maps/mp/duel6.bsp", size, Some(menu), cvars)?;
    // The window's profile folder lives as long as the test.
    std::mem::forget(profile);
    let _ = frame(&mut gpu, 10);
    gpu.holocron_stage_seconds = Some(2.4);
    if let Some(console) = gpu.console.as_mut() {
        console.preview_holocrons(data, 0);
    }
    gpu.open_profile_hub(Tab::Holocrons);
    assert_eq!(gpu.profile_hub_tab(), Some(Tab::Holocrons));
    Some(gpu)
}

/// Choose tier `index` and let the holocron swap and grow into its look (the camera's
/// cut through dark on this toured map takes a second or two the first time).
fn choose(gpu: &mut GpuState, index: usize, frames: u32) {
    if let Some(console) = gpu.console.as_mut() {
        console.select_holocron(index);
    }
    let _ = frame(gpu, frames);
}

fn overflowed(gpu: &GpuState) -> bool {
    gpu.console
        .as_ref()
        .is_some_and(crate::console::ViewerConsole::holocrons_page_overflowed)
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_holocrons() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
        ];
        let Some(mut gpu) = open_tab([1920, 1080], &cvars, holdings()) else {
            return;
        };
        let _ = frame(&mut gpu, 150);
        let mut images = Vec::new();
        for (index, tier) in crate::holocrons::TIERS.iter().enumerate() {
            choose(&mut gpu, index, 60);
            assert!(!overflowed(&gpu), "{}", tier.id);
            let path = shoot(&mut gpu, 2, &format!("duel6-holocrons-{}", tier.id));
            println!("{}", path.display());
            images.push(image::open(path).expect("the shot").to_rgba8());
        }
        println!(
            "{}",
            sheet(&images, 2, 960, "duel6-holocrons-tiers").display()
        );
        // A tier held none of, close up on the stage: dimmed, not hidden.
        choose(&mut gpu, 2, 60);
        let locked = frame(&mut gpu, 2);
        let stage = image::imageops::crop_imm(&locked, 120, 220, 800, 560).to_image();
        println!(
            "{}",
            sheet(&[stage], 1, 800, "duel6-holocrons-locked-stage").display()
        );
        // The identity off: nothing known, every tier dimmed, the reason said.
        if let Some(console) = gpu.console.as_mut() {
            console.preview_holocrons(Data::nothing(State::IdentityOff), 3);
        }
        let _ = frame(&mut gpu, 60);
        println!(
            "{}",
            shoot(&mut gpu, 2, "duel6-holocrons-identity-off").display()
        );
        assert!(!overflowed(&gpu));
        // Waiting for the hub.
        if let Some(console) = gpu.console.as_mut() {
            console.preview_holocrons(Data::nothing(State::Waiting), 1);
        }
        let _ = frame(&mut gpu, 4);
        println!(
            "{}",
            shoot(&mut gpu, 2, "duel6-holocrons-waiting").display()
        );
    });
}

/// The four holocrons up close (`docs/holocrons.md`, "The four holocrons"): each tier
/// held, at two moments (its parts moved between them), then each held none of (the
/// locked look, its parts at rest), cropped to the stage on one sheet.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_holocron_forms() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
        ];
        let mut held = holdings();
        held.counts = Some([12, 3, 2, 1]);
        let Some(mut gpu) = open_tab([1920, 1080], &cvars, held) else {
            return;
        };
        let _ = frame(&mut gpu, 150);
        let stage = |picture: &image::RgbaImage| {
            image::imageops::crop_imm(picture, 200, 230, 640, 640).to_image()
        };
        let mut moments = Vec::new();
        for seconds in [2.4, 4.1] {
            gpu.holocron_stage_seconds = Some(seconds);
            for index in 0..crate::holocrons::TIERS.len() {
                choose(&mut gpu, index, 60);
                moments.push(stage(&frame(&mut gpu, 2)));
            }
        }
        if let Some(console) = gpu.console.as_mut() {
            let mut none = holdings();
            none.counts = Some([0; 4]);
            console.preview_holocrons(none, 0);
        }
        gpu.holocron_stage_seconds = Some(2.4);
        for index in 0..crate::holocrons::TIERS.len() {
            choose(&mut gpu, index, 60);
            moments.push(stage(&frame(&mut gpu, 2)));
        }
        println!(
            "{}",
            sheet(&moments, 4, 480, "duel6-holocron-forms").display()
        );
    });
}

/// The engram (the Mythical holocron) and the four kyber crystals up close
/// (`docs/holocrons.md`, "Kyber crystals"): each held at two moments (the parts moved
/// between them), then each held none of, cropped to
/// the stage on one sheet, a row each.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_kyber_forms() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
        ];
        let Some(mut gpu) = open_tab([1920, 1080], &cvars, holdings()) else {
            return;
        };
        let _ = frame(&mut gpu, 150);
        let stage = |picture: &image::RgbaImage| {
            image::imageops::crop_imm(picture, 140, 130, 760, 760).to_image()
        };
        let forms = [
            "mythical",
            "kyber_uncommon",
            "kyber_rare",
            "kyber_legendary",
            "kyber_mythical",
        ];
        let mut moments = Vec::new();
        for (seconds, owned) in [(2.4, true), (4.1, true), (2.4, false)] {
            gpu.holocron_stage_seconds = Some(seconds);
            for id in forms {
                gpu.holocron_stage_request = crate::holocrons::stage::Request::of(id, owned);
                let _ = frame(&mut gpu, 60);
                moments.push(stage(&frame(&mut gpu, 2)));
            }
        }
        gpu.holocron_stage_request = None;
        println!(
            "{}",
            sheet(&moments, forms.len() as u32, 480, "duel6-kyber-forms").display()
        );
        // The engram alone, large, at three moments.
        let mut engram = Vec::new();
        for seconds in [1.0, 2.4, 5.3] {
            gpu.holocron_stage_seconds = Some(seconds);
            gpu.holocron_stage_request = crate::holocrons::stage::Request::of("mythical", true);
            let _ = frame(&mut gpu, 60);
            engram.push(stage(&frame(&mut gpu, 2)));
        }
        gpu.holocron_stage_request = None;
        println!("{}", sheet(&engram, 3, 640, "duel6-engram").display());
    });
}

/// Other windows and styles: 4:3, 21:9, 4K, `ui_textScale 1.2`, and the page over the
/// classic menus (the world stays visible behind it).
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_holocrons_windows() {
    on_big_stack(|| {
        let sjk = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
        ];
        for (size, name, tier) in [
            ([1440, 1080], "4x3", 2),
            ([2560, 1080], "21x9", 3),
            ([3840, 2160], "4k", 1),
        ] {
            let Some(mut gpu) = open_tab(size, &sjk, holdings()) else {
                return;
            };
            let _ = frame(&mut gpu, 150);
            choose(&mut gpu, tier, 60);
            assert!(!overflowed(&gpu), "{name}");
            println!(
                "{}",
                shoot(&mut gpu, 2, &format!("duel6-holocrons-{name}")).display()
            );
        }
        let styled = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::text::style::SCALE_CVAR, "1.2"),
        ];
        if let Some(mut gpu) = open_tab([1920, 1080], &styled, holdings()) {
            let _ = frame(&mut gpu, 150);
            choose(&mut gpu, 1, 60);
            println!("{}", shoot(&mut gpu, 2, "duel6-holocrons-styled").display());
        }
        // The classic menus: the page opens on its own (`holocrons`), and the world,
        // which classic's pages hide, shows behind it.
        let classic = [("ui_menuStyle", "classic")];
        let menu = menu::ClientMenu::new(true, String::new());
        let Some((mut gpu, _profile)) =
            open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &classic)
        else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        gpu.holocron_stage_seconds = Some(2.4);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_holocrons(holdings(), 0);
            console.open_holocrons_panel();
        }
        choose(&mut gpu, 3, 90);
        println!(
            "{}",
            shoot(&mut gpu, 2, "duel6-holocrons-classic").display()
        );
    });
}

/// Open all's question (`docs/cards.md`, "In the client"): sixteen held (a batch, so the
/// line about the simpler animation), then three (each its full opening), then 34 (twenty
/// of them, the rest for later), and 4K.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_holocrons_open_all_question() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
        ];
        let sealed = |counts: [u64; 4]| {
            let mut next = 0;
            counts.map(|count| {
                let ids: Vec<u64> = (next..next + count).collect();
                next += count;
                ids
            })
        };
        let cases: [(&str, [u64; 4], [u32; 2]); 4] = [
            ("sixteen", [12, 3, 0, 1], [1920, 1080]),
            ("three", [2, 1, 0, 0], [1920, 1080]),
            ("thirty-four", [25, 6, 2, 1], [1920, 1080]),
            ("4k", [12, 3, 0, 1], [3840, 2160]),
        ];
        for (name, counts, size) in cases {
            let data = Data {
                counts: Some(counts.map(|count| count as u32)),
                sealed: sealed(counts),
                ..holdings()
            };
            let Some(mut gpu) = open_tab(size, &cvars, data) else {
                return;
            };
            let _ = frame(&mut gpu, 90);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_ask_all();
            }
            let _ = frame(&mut gpu, 4);
            assert!(!overflowed(&gpu), "{name}");
            println!(
                "{}",
                shoot(&mut gpu, 2, &format!("duel6-holocrons-open-all-{name}")).display()
            );
        }
    });
}

/// A build older than the hub's minimum client (`docs/holocrons.md`, "Opening"): the
/// holocrons still listed, the update notice in place of Open, Open all and Shop, at
/// 1080p and 4K.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_holocrons_update_needed() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
        ];
        for (name, size) in [("1080p", [1920, 1080]), ("4k", [3840, 2160])] {
            let data = Data {
                sealed: [(0..12).collect(), (12..15).collect(), Vec::new(), vec![15]],
                can_open: false,
                update: Some("2026.1022.1".to_owned()),
                ..holdings()
            };
            let Some(mut gpu) = open_tab(size, &cvars, data) else {
                return;
            };
            let _ = frame(&mut gpu, 90);
            assert!(!overflowed(&gpu), "{name}");
            println!(
                "{}",
                shoot(&mut gpu, 2, &format!("duel6-holocrons-update-{name}")).display()
            );
        }
    });
}

/// Made-up holdings with kyber crystals (`docs/holocrons.md`, "Kyber crystals"): two
/// Uncommon, one Rare and one Mythical held, the pity at 3 of 10.
pub(super) fn crystal_holdings() -> Data {
    Data {
        crystals: Some([2, 1, 0, 1]),
        crystal_sealed: [vec![40, 41], vec![42], Vec::new(), vec![43]],
        // One of the Uncommon crystals is Beta, the others Alpha (06/11/2026).
        editions: vec![
            (40, "alpha".to_owned()),
            (41, "beta".to_owned()),
            (42, "alpha".to_owned()),
            (43, "alpha".to_owned()),
        ],
        pity: Some(crate::kyber::Pity {
            every: 10,
            since: 3,
        }),
        ..holdings()
    }
}

/// The kyber crystals on the Holocrons tab: their row of tiles under the holocrons with
/// the pity line, a Rare crystal chosen (the stage falls back to the Mythical holocron's
/// model until the crystals' own come), one held none of, a crystal's hover card, its
/// loot table, then a crystal's opening: the strip of shaders only, at rest, and the
/// shader's entrance. Off-screen renders on duel6 with made-up holdings.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_kyber_crystals() {
    on_big_stack(|| {
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            ("name", "^1Sol^7Vulpes"),
            (crate::discord_presence::CVAR, "0"),
        ];
        let Some(mut gpu) = open_tab([1920, 1080], &cvars, crystal_holdings()) else {
            return;
        };
        let _ = frame(&mut gpu, 150);
        let rare = crate::kyber::TIERS[1].slot;
        let mut images = Vec::new();
        let mut keep = |gpu: &mut GpuState, frames: u32, name: &str| {
            let path = shoot(gpu, frames, name);
            assert!(!overflowed(gpu), "{name}");
            println!("{}", path.display());
            images.push(image::open(path).expect("the shot").to_rgba8());
        };
        choose(&mut gpu, rare, 60);
        keep(&mut gpu, 2, "duel6-kyber-rare");
        choose(&mut gpu, crate::kyber::TIERS[2].slot, 60);
        keep(&mut gpu, 2, "duel6-kyber-legendary-none");
        choose(&mut gpu, rare, 30);
        if let Some(console) = gpu.console.as_mut() {
            console.holocron_hover_for_shot(Some(rare));
        }
        keep(&mut gpu, 30, "duel6-kyber-hover");
        if let Some(console) = gpu.console.as_mut() {
            console.holocron_hover_for_shot(None);
            console.preview_loot(true);
        }
        keep(&mut gpu, 4, "duel6-kyber-loot");
        if let Some(console) = gpu.console.as_mut() {
            console.preview_loot(false);
        }
        // A Rare crystal opened: the strip (shaders only) at rest, then the entrance.
        let opened = |id: &str, name: &str| sjk_identity::Opened {
            banner: None,
            holocron: 42,
            tier: "kyber_rare".to_owned(),
            card: None,
            credits: 0,
            unlock: Some(sjk_identity::OpenedUnlock {
                finish: None,
                id: id.to_owned(),
                name: name.to_owned(),
                count: 1,
            }),
            shards: 0,
            extra: Vec::new(),
            pity: None,
        };
        let prize = || {
            crate::console::holocrons_panel::opening::Prize::of(&opened(
                "saber_phoenix",
                "Phoenix blade",
            ))
            .expect("a crystal's prize")
        };
        for (elapsed, name) in [
            (-5.0, "duel6-kyber-open-strip"),
            (-0.2, "duel6-kyber-open-rest"),
            (3.5, "duel6-kyber-open-shader"),
        ] {
            if let Some(console) = gpu.console.as_mut() {
                console.preview_opening(Some(vec![prize()]), elapsed, 3.1);
            }
            keep(&mut gpu, 40, name);
        }
        if let Some(console) = gpu.console.as_mut() {
            console.preview_opening(None, 0.0, 0.0);
        }
        println!("{}", sheet(&images, 2, 960, "duel6-kyber-sheet").display());
    });
}
