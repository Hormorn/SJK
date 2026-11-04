//! What the toys say when the player switches them, and `toy_help` (Blep's idea, Sol's
//! #ideas recap 29/10/2026; Creyon's test, 29/10/2026: short, on and off told apart by
//! colour, the explanations in a command of their own).
//!
//! - **Switched:** every toy with an on/off switch says its name and **ON** in green or
//!   **OFF** in red on the centre of the screen ([`switched`]): Illuminate, the cardboard
//!   box, Order 66, the Hax, the Drone, the Monitor, Box hunt and the Banana peel, and
//!   the Boombox put away (its start names the track instead). Said only for the
//!   player's own switch (a
//!   bind, the quick wheel, the Collection's Toys tab); a toy restored at start
//!   (`toy_saved`) or taken off by the game (a box lifted) says nothing.
//! - **`toy_help`** ([`COMMAND`]) lists every toy in the console, its name in yellow and,
//!   in white, what it does and how ([`help_lines`]).

/// The console command listing every toy.
pub(crate) const COMMAND: &str = "toy_help";
/// Its line in `cmdlist` and the command reference.
pub(crate) const HELP: &str = "List every toy: what it does and how to use it";

/// The words for toy `name` switched on (`true`) or off: its name, then ON in green or
/// OFF in red.
pub(crate) fn switched(name: &str, on: bool) -> String {
    format!("{name} {}", if on { "^2ON" } else { "^1OFF" })
}

/// Every toy: its name, its switch and what it does and how, in `toy_help`'s order.
pub(crate) const TOYS: [(&str, &str, &str); 14] = [
    (
        "Illuminate",
        "toy_illuminate",
        "a holocron floats by your shoulder and lights the way; SJK players see it",
    ),
    (
        "Cardboard box",
        "toy_box",
        "hide in a box, crouched; another player lifts it with Use",
    ),
    (
        "Order 66",
        "toy_order66",
        "everyone who has it on falls when someone gives the order (order66)",
    ),
    (
        "Hax",
        "toy_hax",
        "hold Use on a player who has it on to hack them and steer them for a few seconds",
    ),
    (
        "Ragdoll",
        "toy_ragdoll",
        "fall limp; press it again to get up",
    ),
    (
        "Drone",
        "toy_drone",
        "on, other players' drones may peek at you and follow you",
    ),
    (
        "Boombox",
        "toy_boombox",
        "carry a stereo playing your tracks to players close by, or stand it down with boombox_place; boombox lists them (verified players)",
    ),
    (
        "Monitor",
        "toy_monitor",
        "throw monitors with your throw key, hold it to charge; monitors knock you flying",
    ),
    (
        "Tag",
        "toy_tag",
        "stick a tag picture on the wall or floor you look at; every SJK player there sees it",
    ),
    (
        "Box hunt",
        "toy_boxhunt",
        "you may become the mark; kill the mark to win a free mystery box roll",
    ),
    (
        "Banana peel",
        "toy_banana",
        "drop peels with banana_drop; players who have it on and walk onto one slip and fall",
    ),
    (
        "Prop gun",
        "toy_propgun",
        "hold a gun from a pk3 instead of your weapon (propgun chooses it); its bolts knock down players who have it on",
    ),
    (
        "Fishing",
        "toy_fishing",
        "stand still and cast; press Use when the float dips to land a fish or junk for your fish log",
    ),
    (
        "Mining",
        "toy_mining",
        "strike rock with Use to break veins for ore; strike as the vein glints for a clean strike",
    ),
];

/// `toy_help`'s lines: a title, then one line a toy, its name in yellow, its switch, and
/// in white what it does.
pub(crate) fn help_lines() -> Vec<String> {
    let mut lines = Vec::with_capacity(TOYS.len() + 1);
    lines.push(
        "Toys (switch them with their command, the quick wheel or Collection > Toys):".to_owned(),
    );
    lines.extend(
        TOYS.iter()
            .map(|(name, command, does)| format!("^3{name}^7 ({command}): {does}")),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ON in green, OFF in red, after the toy's name, and short.
    #[test]
    fn a_switch_says_on_in_green_and_off_in_red() {
        assert_eq!(switched("Cardboard box", true), "Cardboard box ^2ON");
        assert_eq!(switched("Cardboard box", false), "Cardboard box ^1OFF");
        for (name, _, _) in TOYS {
            assert!(switched(name, true).len() <= 20, "{name}");
        }
    }

    /// `toy_help` names every toy command once, each name in yellow, and only real
    /// toy commands.
    #[test]
    fn toy_help_lists_every_toy_once() {
        let lines = help_lines();
        assert_eq!(lines.len(), TOYS.len() + 1);
        for command in [
            crate::illuminate::COMMAND,
            crate::cardboard_box::COMMAND,
            crate::order66::COMMAND,
            crate::hax::COMMAND,
            crate::ragdoll::COMMAND,
            crate::drone::COMMAND,
            crate::boombox::COMMAND,
            crate::monitor::COMMAND,
            crate::tags::COMMAND,
            crate::box_hunt::COMMAND,
            crate::banana::COMMAND,
            crate::propgun::COMMAND,
        ] {
            let naming: Vec<_> = lines
                .iter()
                .filter(|line| line.contains(&format!("({command})")))
                .collect();
            assert_eq!(naming.len(), 1, "{command}");
            assert!(naming[0].starts_with("^3"), "{command}");
        }
    }
}
