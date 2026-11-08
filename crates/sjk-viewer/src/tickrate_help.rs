//! `help_tickrate`: what players mean by "tick rate", printed to the console for
//! anyone to read (`docs/networking.md`, "User commands and move packets"). Three
//! rates get the name: the frames drawn, the user commands sent and the server's
//! snapshots. The text says how SJK times commands, how it sends them on servers
//! known to cap them (JoF's, [`crate::server_caps`]) and why (Heimdall's kicks),
//! what that costs and how a player keeps even steps there; then what this session
//! uses. Sol asked for it on 08/11/2026 as a reminder of the open question about
//! JoF's pacing, and as something to tell players to type.

use crate::server_caps::ServerCap;
use sjk_client::command_rate::Pacing;

/// Console command that prints the explanation.
pub(crate) const COMMAND: &str = "help_tickrate";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Explain tick rate: frames, commands and snapshots, and how SJK sends commands on JoF";

/// The explanation, a console line each (the console wraps them).
const EXPLANATION: &[&str] = &[
    "^5Tick rate: frames, commands and snapshots",
    "^3Three rates get called the tick rate",
    "Frames: what your screen draws a second (com_maxfps).",
    "Commands: your inputs, sent to the server. Each one moves you for as long as it \
     lasts, so their length is how your jumps, strafes and acceleration feel (gameplay_fps; \
     help_gameplay_fps explains it).",
    "Snapshots: what the server sends back a second (its sv_fps, often 20 to 40). They are \
     how you see the others, not how you move.",
    "^3How SJK sends commands",
    "In the original game every frame made one command, so com_maxfps 125, 142, 250 or 333 \
     was also your physics. SJK makes commands on an even clock of their own instead: \
     gameplay FPS 125 sends one every 8 ms, whatever you draw.",
    "^3On JoF",
    "JoF's servers take 125 commands a second at most, what JoF EJK's cl_cmdratecap sends. \
     Since 02/11/2026 SJK sends them there the way JoF EJK does: at most one a frame, 125 a \
     second on average. A command then lasts as long as the frames that made it, not 8 ms: \
     at 350 FPS they are 5 to 11 ms long, so your movement does not get the steady steps \
     of 125. JoF EJK moves the same way.",
    "Why: Heimdall, JoF's anti-cheat, kicked SJK players with \"Set /cl_timeNudge 0 in your \
     console!\". One command a frame was the first guess and did not stop the kicks. The \
     cause was command times a frame newer than other clients' (fixed on 07/11/2026). By \
     its maintainer's account Heimdall kicks for commands that run ahead of the server, not \
     for ones that trail it, as the even clock's do when a frame is slow.",
    "To keep even 8 ms steps on JoF: gameplay_fps_nocap 1 (Settings > Display > Ignore \
     server caps) with gameplay_fps 125. Keep it at 125: lifting the cap also lets more \
     than 125 commands a second through, which made JoF unplayable. Below 125 FPS it has \
     not been tried against Heimdall yet: report a kick if you get one.",
    "Still open: SJK may go back to the even clock on JoF for everyone once that is known.",
];

/// What this session uses, for the lines after the explanation.
pub(crate) struct Now {
    /// The FPS cap (`com_maxfps`, Auto resolved; 0 is uncapped).
    pub(crate) frame_cap: u32,
    /// `gameplay_fps` (0 is Auto).
    pub(crate) gameplay_fps: i64,
    /// Milliseconds between commands.
    pub(crate) step: i32,
    pub(crate) pacing: Pacing,
    /// The server's `pmove_msec` while it fixes the step.
    pub(crate) pmove_fixed: Option<i32>,
    /// The server's known cap, and whether the player lifted it.
    pub(crate) cap: Option<(&'static ServerCap, bool)>,
    /// On a server.
    pub(crate) connected: bool,
}

/// The explanation, then what `now` uses.
pub(crate) fn lines(now: &Now) -> Vec<String> {
    let mut lines: Vec<String> = EXPLANATION.iter().map(|line| (*line).to_owned()).collect();
    lines.push("^3Yours right now".to_owned());
    lines.push(match now.frame_cap {
        0 => "Frames: uncapped (com_maxfps).".to_owned(),
        cap => format!("Frames: {cap} a second at most (com_maxfps)."),
    });
    lines.push(format!(
        "Gameplay FPS: {} (gameplay_fps).",
        crate::gameplay_fps::describe(now.gameplay_fps)
    ));
    let per_second = 1000 / now.step.max(1);
    if !now.connected {
        lines.push(format!(
            "Commands on a server: one every {} ms, {per_second} a second, unless the server \
             fixes or caps them; type {COMMAND} on one to see.",
            now.step
        ));
        return lines;
    }
    lines.push(match now.pacing {
        Pacing::Grid => format!(
            "Commands: one every {} ms, {per_second} a second, on the even clock.",
            now.step
        ),
        Pacing::OnePerFrame => format!(
            "Commands: at most one a frame, {per_second} a second on average, as JoF EJK's."
        ),
    });
    if let Some(msec) = now.pmove_fixed {
        lines.push(format!(
            "This server moves everyone in {msec} ms steps (pmove_fixed): it decides the step."
        ));
    }
    if let Some((cap, lifted)) = now.cap {
        lines.push(if lifted {
            format!(
                "{} servers take {} a second at most; you lifted that (gameplay_fps_nocap 1).",
                cap.name,
                cap.per_second()
            )
        } else {
            format!(
                "{} servers take {} a second at most; SJK keeps to it.",
                cap.name,
                cap.per_second()
            )
        });
    }
    lines
}

impl crate::GpuState {
    /// `help_tickrate`: the explanation and this session's rates, to the console.
    pub(crate) fn tickrate_help_command(&self) -> Result<Vec<String>, String> {
        let console = self.console.as_ref().ok_or("Console unavailable")?;
        let cap = console.server_cap();
        let now = Now {
            frame_cap: console.frame_cap(),
            gameplay_fps: console.gameplay_fps(),
            step: console.command_step(),
            pacing: console.command_pacing(),
            pmove_fixed: console.pmove_fixed_msec(),
            cap: cap.map(|cap| (cap, console.server_caps_lifted())),
            connected: self.live_session.is_some(),
        };
        Ok(lines(&now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(connected: bool) -> Now {
        Now {
            frame_cap: 350,
            gameplay_fps: 125,
            step: 8,
            pacing: Pacing::Grid,
            pmove_fixed: None,
            cap: None,
            connected,
        }
    }

    fn jof() -> &'static ServerCap {
        crate::server_caps::KNOWN
            .iter()
            .find(|cap| cap.family == "jof")
            .expect("JoF's cap is known")
    }

    #[test]
    fn the_explanation_covers_the_three_rates_jof_and_the_way_back() {
        let text = lines(&now(false)).join("\n");
        for needle in [
            "com_maxfps",
            "help_gameplay_fps",
            "sv_fps",
            "cl_cmdratecap",
            "Heimdall",
            "gameplay_fps_nocap 1",
            "Ignore server caps",
            "Still open",
        ] {
            assert!(text.contains(needle), "missing {needle:?}");
        }
        // The command and its help name each other.
        assert!(text.contains(COMMAND));
        assert!(HELP.contains("JoF"));
    }

    #[test]
    fn off_a_server_it_says_what_commands_would_be() {
        let lines = lines(&now(false));
        let last = lines.last().expect("lines");
        assert!(last.starts_with("Commands on a server: one every 8 ms, 125 a second"));
        assert!(lines.contains(&"Frames: 350 a second at most (com_maxfps).".to_owned()));
        assert!(
            lines.contains(&"Gameplay FPS: 125 (8 ms steps) (gameplay_fps).".to_owned()),
            "{lines:?}"
        );
    }

    #[test]
    fn on_jof_it_says_one_a_frame_and_whether_the_cap_is_lifted() {
        let mut on_jof = now(true);
        on_jof.pacing = Pacing::OnePerFrame;
        on_jof.cap = Some((jof(), false));
        let lines = lines(&on_jof);
        assert!(lines.contains(
            &"Commands: at most one a frame, 125 a second on average, as JoF EJK's.".to_owned()
        ));
        assert_eq!(
            lines.last().map(String::as_str),
            Some("JoF servers take 125 a second at most; SJK keeps to it.")
        );

        // Lifted: the even clock, and the line says the player lifted it.
        let mut lifted = now(true);
        lifted.cap = Some((jof(), true));
        let lines = super::lines(&lifted);
        assert!(
            lines
                .contains(&"Commands: one every 8 ms, 125 a second, on the even clock.".to_owned())
        );
        assert!(
            lines
                .last()
                .is_some_and(|line| line.contains("you lifted that"))
        );
    }

    #[test]
    fn a_fixed_step_server_says_it_decides_the_step() {
        let mut fixed = now(true);
        fixed.pmove_fixed = Some(8);
        assert!(
            lines(&fixed)
                .iter()
                .any(|line| line.contains("8 ms steps (pmove_fixed)"))
        );
    }
}
