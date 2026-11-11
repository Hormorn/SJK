//! World shots of the playstyle card ([`crate::playstyle_card`]) over duel6's main menu
//! under the SJK UI: as it opens with nameplates off, then with picks ticked. Off-screen
//! renders (no game window), ignored like the other world shots: they need a GPU adapter
//! and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer playstyle_card_shot --
//! --ignored --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.

use super::*;
use crate::playstyle_card::Choice;
use std::time::Duration;
use winit::keyboard::KeyCode;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn playstyle_card_shot() {
    on_big_stack(|| {
        crate::playstyle_card::reset_for_shot();
        let menu = menu::ClientMenu::new(true, String::new());
        let alpha = crate::alpha_notice_card::VERSION.to_string();
        let cvars = [
            ("ui_menuStyle", "sjk"),
            (crate::settings::quick::HIDE_CVAR, "1"),
            (crate::alpha_notice_card::CVAR, alpha.as_str()),
            ("cg_nameplate", "0"),
            (crate::playstyle::GOOFING_CVAR, "1"),
        ];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
        else {
            return;
        };
        let _ = frame(&mut gpu, 20);
        gpu.ui_epoch -= Duration::from_millis(2_000);
        println!("{}", shoot(&mut gpu, 4, "playstyle-card").display());
        assert!(gpu.playstyle_card.is_open(), "the card shows at start");
        for key in [
            KeyCode::Digit1,
            KeyCode::Digit3,
            KeyCode::Digit6,
            KeyCode::Digit0,
        ] {
            let _ = gpu.playstyle_card.handle_key_code_for_test(key);
        }
        println!("{}", shoot(&mut gpu, 2, "playstyle-card-ticked").display());
        let choice = gpu.playstyle_card.handle_key_code_for_test(KeyCode::Enter);
        assert_eq!(choice.map(|(choice, _)| choice), Some(Choice::Save));
        gpu.playstyle_card_choice(choice);
        let _ = frame(&mut gpu, 2);
        assert!(!gpu.playstyle_card.is_open(), "saved, the card is gone");
        let console = gpu.console.as_ref().unwrap();
        assert_eq!(
            console.bool_cvar(crate::playstyle::SABER_DUELS_CVAR),
            Some(true)
        );
        assert_eq!(console.bool_cvar(crate::playstyle::FFA_CVAR), Some(true));
        assert_eq!(
            console.bool_cvar(crate::playstyle::GOOFING_CVAR),
            Some(true)
        );
        assert_eq!(console.bool_cvar("cg_nameplate"), Some(true));
        assert_eq!(console.integer_cvar(crate::playstyle_card::CVAR), Some(1));
    });
}
