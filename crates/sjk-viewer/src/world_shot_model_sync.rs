//! A world shot of the Model sync page ([`crate::console::model_sync_page`]) over
//! duel6's main menu under the SJK UI, with made-up rows (one of each state) for a
//! player who may share. An off-screen render (no game window), ignored like the other
//! world shots: it needs a GPU adapter and the installed game data named by
//! `JKA_GAME_DATA`. The page's ask for the Sith cloud's list goes to a local port that
//! answers nothing.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer model_sync_page_shot --
//! --ignored --test-threads=1 --nocapture`; the picture is in `target/world-shots`.

use super::*;
use crate::model_sync::PageRow;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn model_sync_page_shot() {
    on_big_stack(|| {
        let menu = menu::ClientMenu::new(true, String::new());
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::sith_cloud::URL, "http://127.0.0.1:9/repository"),
        ];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
        else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        let row = |name: &str| PageRow {
            name: name.to_owned(),
            cloud: None,
            fetched: None,
            host_pack: false,
            installed: true,
            worn: false,
        };
        let mut rows = vec![
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
        ];
        rows.extend(
            [
                "mandalorian",
                "obi_kenobi_old",
                "revan_reborn",
                "shaak_ti_cw",
            ]
            .map(row),
        );
        let lines = vec![
            "Model sync: solfox packed (14 files, 9.2 MB); asking the SJK hub...".to_owned(),
            "Model sync: solfox shared, version 3. Players with it download it from their next map."
                .to_owned(),
            "^3Model sync: grogu: the download did not match its listing.".to_owned(),
        ];
        let game_data = gpu.game_data.clone();
        let console = gpu.console.as_mut().expect("a console");
        console.toggle_model_sync(&game_data);
        console.preview_model_sync(rows, true, lines);
        let path = shoot(&mut gpu, 6, "model-sync-page");
        println!("{}", path.display());
        assert!(gpu.console.as_ref().unwrap().model_sync_open());
    });
}
