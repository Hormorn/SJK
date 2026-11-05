//! Guard on the main thread's stack (`build.rs`, `MAIN_STACK_BYTES`, 64 MiB): the
//! client's start, the menu's world built and its first frames drawn, must fit in a
//! quarter of it. The menu world alone came within 2 MiB of the former 8 MiB with a
//! clean profile and overflowed it with JoF content (03/11/2026); the reserve now
//! leaves room, and this test says when a change eats into it, before players crash
//! at start. A real profile (JoF content, more pk3s) needs a few MiB more than this
//! clean one. An overflow aborts the whole test binary (`STATUS_STACK_OVERFLOW`).

/// A quarter of the main thread's 64 MiB.
const BUDGET_BYTES: usize = (crate::stack_size::MAIN_STACK_BYTES / 4) as usize;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn the_client_starts_in_a_quarter_of_the_main_stack() {
    std::thread::Builder::new()
        .name("startup stack budget".into())
        .stack_size(BUDGET_BYTES)
        .spawn(|| {
            let menu = crate::menu::ClientMenu::new(true, String::new());
            let (mut gpu, _profile) = crate::world_shot::open(
                "maps/mp/duel6.bsp",
                [1920, 1080],
                Some(menu),
                &[("ui_menuStyle", "sjk")],
            )
            .expect("a GPU adapter");
            let _ = crate::world_shot::frame(&mut gpu, 30);
        })
        .expect("the startup thread")
        .join()
        .expect("the client started");
}
