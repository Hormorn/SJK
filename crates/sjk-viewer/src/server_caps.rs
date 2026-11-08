//! Server families known to take user commands no faster than a step of their own,
//! though their serverinfo does not say so (they are not `pmove_fixed`). Families
//! are told apart by `sjk_mod::known_servers`; this table says only what each takes.
//! The gameplay FPS (`gameplay_fps.rs`, `sjk_client::command_rate::command_step`)
//! never makes commands faster than such a server takes, unless the player lifts the
//! caps with [`OVERRIDE_CVAR`]. Servers that drop fast inputs and are not listed open
//! the gameplay FPS card by themselves (`input_health.rs`); one found that way
//! belongs in `known_servers` and here.

/// 1 lifts every cap in [`KNOWN`]: the gameplay FPS applies as set.
pub(crate) const OVERRIDE_CVAR: &str = "gameplay_fps_nocap";

/// A server family's shortest command step.
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
    },
];

/// Server families whose commands carry the frame's own clock reading instead of the
/// previous frame's (`sjk_client::ServerClock::command_time`, as stock and JoF EJK).
/// The previous frame's reading matches every other client, which anti-cheats such
/// as JoF's Heimdall compare against; SJK's own server runs no such check, so its
/// players keep the fresher stamp (Sol, 07/11/2026).
pub(crate) const FRESH_STAMPS: &[&str] = &["sjk"];

/// 1 keeps the frame's own clock reading on every server, as on [`FRESH_STAMPS`]'
/// (Sol, 07/11/2026).
pub(crate) const FRESH_STAMPS_CVAR: &str = "cl_freshStamps";

/// Whether commands on a server of `family` carry the previous frame's clock
/// reading: every server but [`FRESH_STAMPS`]'.
pub(crate) fn stamps_previous_frame(family: Option<&sjk_mod::KnownServer>) -> bool {
    !family.is_some_and(|family| FRESH_STAMPS.contains(&family.id))
}

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
        assert!(find(&server(JaPlus, r"\V\2.4B7")).is_none());
        assert!(find(&server(BaseJka, r"\V\2.5B0")).is_none());
    }

    #[test]
    fn only_sjk_servers_stamp_with_the_frames_own_reading() {
        use sjk_mod::ServerKind::{BaseJka, JaPlus, JaPro};
        let family = |kind, info| server(kind, info).family();
        assert!(!stamps_previous_frame(family(
            BaseJka,
            r"\gamename\basejka\sjk\1"
        )));
        assert!(stamps_previous_frame(family(BaseJka, r"\gamename\basejka")));
        assert!(stamps_previous_frame(family(JaPlus, r"\V\2.5B0")));
        assert!(stamps_previous_frame(family(JaPlus, r"\V\2.4B7")));
        assert!(stamps_previous_frame(family(JaPro, "")));
        for id in FRESH_STAMPS {
            assert!(sjk_mod::KNOWN_SERVERS.iter().any(|family| family.id == *id));
        }
    }

    #[test]
    fn every_cap_names_a_known_family_and_a_real_step() {
        for cap in KNOWN {
            assert!((1..=50).contains(&cap.step), "{}", cap.name);
            let family = sjk_mod::KNOWN_SERVERS
                .iter()
                .find(|family| family.id == cap.family)
                .unwrap_or_else(|| panic!("{} is no known family", cap.family));
            assert_eq!(family.name, cap.name);
        }
    }
}
