//! Completion/help metadata for locally dispatched selection commands.

/// Register metadata; execution is intercepted by GameplayInput before shell fallback.
pub(super) fn register(shell: &mut sjk_shell::Shell) -> Result<(), sjk_shell::CommandError> {
    for (name, help) in [
        (
            "forcenext",
            "Select next usable Force power (Use held: inventory)",
        ),
        (
            "forceprev",
            "Select previous usable Force power (Use held: inventory)",
        ),
        (
            "forceselect",
            "Select Force wheel entry N (forcePowers_t number; 18-20 JoF's)",
        ),
        (
            "weapmelee",
            "Select the melee weapon (fists), never the saber; does nothing if already selected",
        ),
        ("invnext", "Select next inventory item"),
        ("invprev", "Select previous inventory item"),
        (
            "toy_illuminate",
            "Toy: turn the Illuminate holocron's light on or off",
        ),
        (
            crate::cardboard_box::COMMAND,
            "Toy: hide in a cardboard box, or come out (+use on a boxed player lifts theirs)",
        ),
        (
            crate::fishing::COMMAND,
            "Toy: fish facing water, or put the rod away (+use when the float bobs under hooks the catch)",
        ),
        (
            crate::mining::COMMAND,
            "Toy: take a pickaxe in hand, or put it away (+use strikes rock; strike as it glints)",
        ),
        (
            crate::order66::COMMAND,
            "Toy: Order 66 on or off (every player who has it on dies when one of them executes it)",
        ),
        (
            crate::order66::EXECUTE_COMMAND,
            "Execute Order 66: in 3 seconds every player with the toy on blows up (toy_order66 first)",
        ),
        (
            crate::hax::COMMAND,
            "Toy: Hax on or off (hold +use on a player who has it on to hack them and steer them)",
        ),
        (
            crate::ragdoll::COMMAND,
            "Toy: go limp as a ragdoll, or get up (other SJK players see the same fall)",
        ),
        (
            crate::boombox::COMMAND,
            "Toy: Boombox on or off (a stereo on your shoulder playing your boombox tracks to the SJK players close by; verified players)",
        ),
        (
            crate::boombox::PLACE_COMMAND,
            "Stand your boombox on the floor in front of you (putting it on first), its music playing from there; again picks it back up",
        ),
        (
            crate::drone::COMMAND,
            "Toy: Drone on or off (on, players may peek at you; freecam and peek fly a drone others see)",
        ),
        (
            crate::monitor::COMMAND,
            "Toy: Monitor on or off (on, you throw monitors with monitor_throw and they knock you flying)",
        ),
        (
            crate::tags::COMMAND,
            "Toy: stick your tag (cg_tag) where you look, on a wall or the floor; toy_tag clear takes it off (other SJK players on the map see it)",
        ),
        (
            crate::monitor::THROW_COMMAND,
            "Throw a monitor the way you look (the Monitor toy on); it sends players who have it on flying",
        ),
        (
            crate::banana::COMMAND,
            "Toy: Banana peel on or off (on, you drop peels with banana_drop and slip on the ones you walk onto; only players with it on slip)",
        ),
        (
            crate::propgun::COMMAND,
            "Toy: Prop gun on or off (on, you hold the gun propgun chose instead of your weapon, attack fires its bolts, and others' bolts knock you down; only players with it on are hit)",
        ),
        (
            crate::banana::DROP_COMMAND,
            "Drop a banana peel at your feet, or tossed ahead when you move (the Banana peel toy on); players who have it on slip on it",
        ),
        (
            crate::box_hunt::COMMAND,
            "Toy: Box hunt on or off (on, you may become the mark (one every 10 minutes), and killing the mark wins you a free mystery box roll; only players with it on take part)",
        ),
    ] {
        shell.commands.register(name, help, |_| {
            Err(sjk_shell::CommandError::Handler(
                "Viewer input dispatcher required".into(),
            ))
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The toy is listed, with its help; the old name runs it (the input dispatcher
    /// takes it, `input.rs`) but is registered nowhere, so completion and `cmdlist`
    /// never show it.
    #[test]
    fn the_toy_is_listed_and_its_old_name_is_not() {
        let mut shell = sjk_shell::Shell::new(
            sjk_shell::CvarRegistry::default(),
            sjk_shell::BindTable::default(),
        );
        register(&mut shell).unwrap();
        assert!(shell.commands.contains("toy_illuminate"));
        assert!(!shell.commands.contains("force_illuminate"));
        let help = shell
            .commands
            .iter()
            .find(|(name, _)| *name == "toy_illuminate")
            .map(|(_, help)| help)
            .unwrap();
        assert!(!help.contains("cg_illuminate") && !help.contains("Force wheel"));
        let forceselect = shell
            .commands
            .iter()
            .find(|(name, _)| *name == "forceselect")
            .map(|(_, help)| help)
            .unwrap();
        assert!(!forceselect.contains("Illuminate"));
    }
}
