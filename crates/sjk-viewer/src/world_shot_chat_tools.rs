//! World shots of the chat composer's row tools (`chat/row_tools.rs`): the × on a
//! hovered button, the + menu with the chats taken off the row, and the cog's quick
//! settings, over duel6 at 1080p. Off-screen renders, ignored like the other world
//! shots: they need a GPU adapter and the game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test --release -p sjk-viewer chat_tools_shot --
//! --ignored --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.

use super::*;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_chat_tools_shot() {
    for (step, name) in [(0, "hover-x"), (1, "plus-menu"), (2, "cog-menu")] {
        on_big_stack(move || {
            let cvars = [("cg_chatbox", "60000"), (crate::chat::tabs::SJK_CVAR, "0")];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], None, &cvars)
            else {
                return;
            };
            let _ = frame(&mut gpu, 1);
            let shots = menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
            let (yaw, pitch) = look(shots[0].from, shots[0].at);
            aim(&mut gpu, shots[0].from, yaw, pitch);
            if let Some(console) = gpu.console.as_mut() {
                console.set_open(false);
            }
            let now = std::time::Instant::now();
            gpu.chat.for_shot = true;
            for line in ["^7Kyle^7: ^2anyone up for a duel?", "^7Fox^7: ^2gg"] {
                gpu.chat.receive(
                    sjk_client::ServerEventKind::Chat,
                    line.to_owned(),
                    None,
                    now,
                );
            }
            gpu.chat.draft_for_shot("hello");
            let _ = frame(&mut gpu, 2);
            gpu.chat.row_tools_for_shot(step);
            let name = format!("duel6-chat-tools-{name}-1080p");
            println!("{}", shoot(&mut gpu, 6, &name).display());
        });
    }
}
