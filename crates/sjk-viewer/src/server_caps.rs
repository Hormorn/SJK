//! Server families known to take user commands no faster than a step of their own,
//! though their serverinfo does not say so (they are not `pmove_fixed`). Families
//! are told apart by `sjk_mod::known_servers`; this table says only what each takes.
//! The gameplay FPS (`gameplay_fps.rs`, `sjk_client::command_rate::command_step`)
//! never makes commands faster than such a server takes, unless the player lifts the
//! caps with [`OVERRIDE_CVAR`]. Servers that drop fast inputs and are not listed open
//! the gameplay FPS card by themselves (`input_health.rs`); one found that way
//! belongs in `known_servers` and here. A family may also take fewer repeated
//! commands a packet than `cl_packetdup` asks ([`ServerCap::max_packet_dup`]),
//! which [`PACKET_DUP_OVERRIDE_CVAR`] alone lifts.

/// 1 lifts every command step in [`KNOWN`]: the gameplay FPS applies as set.
pub(crate) const OVERRIDE_CVAR: &str = "gameplay_fps_nocap";

/// 1 lifts every [`ServerCap::max_packet_dup`]: `cl_packetdup` applies as set. Apart
/// from [`OVERRIDE_CVAR`], as an anti-cheat that wants fewer repeats kicks whatever
/// gameplay FPS the player chose.
pub(crate) const PACKET_DUP_OVERRIDE_CVAR: &str = "cl_packetdup_nocap";

/// What a server family takes: its shortest command step, how commands are paced,
/// and how many earlier packets a packet may repeat.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ServerCap {
    /// The family's id in `sjk_mod::known_servers`.
    pub(crate) family: &'static str,
    /// Its name, as the log and the card show it (the family's).
    pub(crate) name: &'static str,
    /// The shortest step it takes, in milliseconds.
    pub(crate) step: i32,
    /// How a frame slower than the step spreads its commands: the family's own
    /// client's way, which its anti-cheat expects.
    pub(crate) pacing: sjk_client::command_rate::Pacing,
    /// The most earlier packets whose commands a packet may repeat (`cl_packetdup`),
    /// when the family takes fewer than the player may set; `None` takes any.
    pub(crate) max_packet_dup: Option<usize>,
}

impl ServerCap {
    /// Commands a second at most.
    pub(crate) fn per_second(&self) -> i32 {
        1000 / self.step.max(1)
    }
}

/// The server families known to cap commands.
pub(crate) const KNOWN: &[ServerCap] = &[
    // JoF's servers report `pmove_fixed` 0, yet 1000 commands a second made the
    // game unplayable there (Sol, 11/10/2026); JoF EJK sends them 125 a second
    // (`cl_cmdratecap`).
    ServerCap {
        family: "jof",
        name: "JoF",
        step: sjk_client::command_rate::COMMAND_MILLIS,
        // One command a frame stamped now, as `cl_cmdratecap`: Heimdall, JoF's
        // anti-cheat, kicked SJK players for a time nudge while the grid's catch-up
        // commands went out stamped up to 32 ms in the past (Krieger, Slash,
        // 02/11/2026).
        pacing: sjk_client::command_rate::Pacing::OnePerFrame,
        // No repeated commands: Milamber, who maintains Heimdall, told Sol that
        // `cl_packetdup` is what it kicked SJK players for (Turin's kicks,
        // 04/11/2026). JoF EJK repeats one packet whatever its `cl_packetdup`
        // (`CL_WritePacket`, `outgoingSequence - 1 - 1`).
        max_packet_dup: Some(0),
    },
];

/// The known cap of `server`'s family, if any.
pub(crate) fn find(server: &sjk_mod::Server) -> Option<&'static ServerCap> {
    let family = server.family()?;
    KNOWN.iter().find(|cap| cap.family == family.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(kind: sjk_mod::ServerKind, info: &str) -> sjk_mod::Server {
        sjk_mod::Server {
            kind,
            address: "127.0.0.1:29070".into(),
            info: info.into(),
        }
    }

    #[test]
    fn jof_servers_are_capped_and_others_are_not() {
        use sjk_mod::ServerKind::{BaseJka, JaPlus};
        let jof = find(&server(JaPlus, r"\gamename\JA+ Mod v2.5\V\2.5B0")).unwrap();
        assert_eq!((jof.name, jof.step, jof.per_second()), ("JoF", 8, 125));
        assert_eq!(jof.pacing, sjk_client::command_rate::Pacing::OnePerFrame);
        assert_eq!(jof.max_packet_dup, Some(0));
        assert!(find(&server(JaPlus, r"\V\2.4B7")).is_none());
        assert!(find(&server(BaseJka, r"\V\2.5B0")).is_none());
    }

    #[test]
    fn every_cap_names_a_known_family_and_a_real_step() {
        for cap in KNOWN {
            assert!((1..=50).contains(&cap.step), "{}", cap.name);
            assert!(
                cap.max_packet_dup
                    .is_none_or(|dup| dup < sjk_client::command_history::MAX_PACKET_DUP),
                "{}: a cap at the stock clamp caps nothing",
                cap.name
            );
            let family = sjk_mod::KNOWN_SERVERS
                .iter()
                .find(|family| family.id == cap.family)
                .unwrap_or_else(|| panic!("{} is no known family", cap.family));
            assert_eq!(family.name, cap.name);
        }
    }
}
