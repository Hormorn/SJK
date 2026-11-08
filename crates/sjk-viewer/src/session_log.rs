//! The session log (`docs/client.md`, "Log requests"): every line of the crash trail
//! ([`crate::crash_report::trail`]: the console's lines without chat, colour codes
//! removed, and the `log::progress` diagnostics) is also written to `logs/session.log`
//! beside the configuration, and a start keeps the session before as
//! `logs/previous-session.log`. So when the game froze or closed, the lines of that
//! session are still there after a restart.
//!
//! The logs leave the PC only when the player sends them, answering a request of the
//! SJK team ([`gather`], `log_request_card`).

use std::fs::File;
use std::io::{LineWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, TryLockError};

/// The folder beside the configuration, and the two files in it.
const FOLDER: &str = "logs";
const CURRENT: &str = "session.log";
const PREVIOUS: &str = "previous-session.log";
/// Past this many bytes the session's file starts again, keeping the newest lines.
const FILE_MAX: u64 = 16 * 1024 * 1024;
/// What a request sends of each part at most, in bytes: the newest lines.
const CURRENT_SENT: u64 = 1536 * 1024;
const PREVIOUS_SENT: u64 = 512 * 1024;
const CRASH_SENT: u64 = 40 * 1024;
/// A last crash report older than this is not sent with the logs.
const CRASH_FRESH_SECS: u64 = 14 * 86_400;
/// Tries at the file's lock before a line is left out of the file (it stays in the
/// trail): another thread writes a line in microseconds, and a panic inside a write
/// must not hang the panic hook that logs.
const LOCK_TRIES: usize = 64;

struct Session {
    dir: PathBuf,
    file: Option<LineWriter<File>>,
    written: u64,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn lock() -> MutexGuard<'static, Option<Session>> {
    SESSION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn try_lock() -> Option<MutexGuard<'static, Option<Session>>> {
    for _ in 0..LOCK_TRIES {
        match SESSION.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => std::thread::yield_now(),
        }
    }
    None
}

/// `dd/mm/yyyy hh:mm UTC` of unix `seconds`.
fn stamp(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or(0);
    let day = seconds.rem_euclid(86_400);
    format!(
        "{} {:02}:{:02} UTC",
        crate::medals::date_text(seconds),
        day / 3_600,
        day % 3_600 / 60
    )
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Start the session's file in `logs/` beside the configuration, keeping the one
/// there as the previous session's, and write the lines kept so far (`earlier`).
/// Once per process; nothing is written in tests.
pub(crate) fn start(config_directory: &Path, earlier: &[String]) {
    if cfg!(test) {
        return;
    }
    let mut session = lock();
    if session.is_some() {
        return;
    }
    let dir = config_directory.join(FOLDER);
    let file = open(&dir);
    let mut started = Session {
        dir,
        file,
        written: 0,
    };
    started.line(&format!(
        "SJK {} session started {} (pid {})",
        crate::build_info::VERSION,
        stamp(unix_now()),
        std::process::id()
    ));
    for line in earlier {
        started.line(line);
    }
    *session = Some(started);
}

/// Keep the last session's file as the previous one's and open a new one.
fn open(dir: &Path) -> Option<LineWriter<File>> {
    std::fs::create_dir_all(dir).ok()?;
    let current = dir.join(CURRENT);
    if current.exists() {
        let _ = std::fs::rename(&current, dir.join(PREVIOUS));
    }
    File::create(current).ok().map(LineWriter::new)
}

impl Session {
    /// Write `line`; past [`FILE_MAX`] the file starts again with a line saying so.
    fn line(&mut self, line: &str) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        if self.written > FILE_MAX {
            let restarted = file.get_mut().set_len(0).and_then(|()| {
                file.get_mut().seek(SeekFrom::Start(0))?;
                writeln!(file, "[earlier lines of this session left out: over 16 MB]")
            });
            self.written = 0;
            if restarted.is_err() {
                self.file = None;
                return;
            }
        }
        if writeln!(file, "{line}").is_err() {
            self.file = None;
            return;
        }
        self.written += line.len() as u64 + 1;
    }
}

/// Write one trail line to the session's file, once it is started.
pub(crate) fn write(line: &str) {
    if let Some(mut session) = try_lock()
        && let Some(session) = session.as_mut()
    {
        session.line(line);
    }
}

/// The last `max` bytes of `path`, from the first whole line in them, and whether
/// earlier lines were left out. `None` when it cannot be read.
fn tail(path: &Path, max: u64) -> Option<(String, bool)> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let start = length.saturating_sub(max);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.take(max).read_to_end(&mut bytes).ok()?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0
        && let Some(end) = text.find('\n')
    {
        text.drain(..=end);
    }
    Some((text, start > 0))
}

/// What the session knows of the machine and where the player is, for the header.
pub(crate) struct Context {
    pub(crate) gpu: String,
    pub(crate) os: String,
    pub(crate) map: String,
    pub(crate) server: String,
}

/// The logs a request sends: a header, the newest lines of this session and of the
/// previous one, and the last crash report when it is recent; the player's own folder
/// shown as `%USERPROFILE%`.
pub(crate) fn gather(context: &Context, home: &str) -> sjk_identity::LogUpload {
    let dir = lock().as_ref().map(|session| session.dir.clone());
    let mut logs = format!(
        "SJK logs, sent {}\nBuild: {}\nSystem: {}\nGraphics: {}\n",
        stamp(unix_now()),
        crate::build_info::VERSION,
        context.os,
        if context.gpu.is_empty() {
            "not known yet"
        } else {
            &context.gpu
        },
    );
    if !context.map.is_empty() {
        logs.push_str(&format!("Map: {}\n", context.map));
    }
    if !context.server.is_empty() {
        logs.push_str(&format!("Server: {}\n", context.server));
    }
    match &dir {
        Some(dir) => {
            for (title, path, max) in [
                ("This session", dir.join(CURRENT), CURRENT_SENT),
                ("The session before", dir.join(PREVIOUS), PREVIOUS_SENT),
            ] {
                logs.push_str(&section(title, &path, max));
            }
            if let Some(crashes) = dir.parent().map(|parent| parent.join("crashes")) {
                let path = crashes.join("last-crash.txt");
                let fresh = std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age.as_secs() < CRASH_FRESH_SECS);
                if fresh {
                    logs.push_str(&crash_section(&path, CRASH_SENT));
                }
            }
        }
        None => logs.push_str("\n(no session log: the game has not started one)\n"),
    }
    sjk_identity::LogUpload {
        logs: sjk_identity::crash::without_home(&logs, home),
        build: crate::build_info::VERSION.to_owned(),
        os: context.os.clone(),
        gpu: context.gpu.clone(),
    }
}

/// One part of the logs: a title line, then the file's newest lines.
fn section(title: &str, path: &Path, max: u64) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    match tail(path, max) {
        Some((text, cut)) => format!(
            "\n===== {title} ({name}{}) =====\n{text}{}",
            if cut { ", newest lines" } else { "" },
            if text.ends_with('\n') || text.is_empty() {
                ""
            } else {
                "\n"
            }
        ),
        None => format!("\n===== {title} ({name}): none =====\n"),
    }
}

/// The last crash report: its head (kind, build, card, message, backtrace) whole, then
/// as many of its last lines as fit. A tail alone kept only the console lines before the
/// crash and lost what crashed (Vin's, 21/10/2026).
fn crash_section(path: &Path, max: u64) -> String {
    const TITLE: &str = "The last crash report";
    const TRAIL: &str = "\nLast lines:\n";
    let Ok(text) = std::fs::read_to_string(path) else {
        return section(TITLE, path, max);
    };
    let max = usize::try_from(max).unwrap_or(usize::MAX);
    let (head, trail) = match text.find(TRAIL) {
        Some(at) => text.split_at(at + TRAIL.len()),
        None => (text.as_str(), ""),
    };
    if text.len() <= max || head.len() >= max {
        return section(TITLE, path, max as u64);
    }
    let mut start = trail.len().saturating_sub(max - head.len());
    while !trail.is_char_boundary(start) {
        start += 1;
    }
    let mut newest = &trail[start..];
    if start > 0
        && let Some(end) = newest.find('\n')
    {
        newest = &newest[end + 1..];
    }
    let end = if newest.ends_with('\n') { "" } else { "\n" };
    format!("\n===== {TITLE} (last-crash.txt, newest of its last lines) =====\n{head}{newest}{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sjk-session-log-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_start_keeps_the_last_session_as_the_previous_one() {
        let dir = temp_dir("rotate");
        std::fs::write(dir.join(CURRENT), "old session\n").unwrap();
        let mut session = Session {
            file: open(&dir),
            dir: dir.clone(),
            written: 0,
        };
        session.line("new line");
        assert_eq!(
            std::fs::read_to_string(dir.join(PREVIOUS)).unwrap(),
            "old session\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(CURRENT)).unwrap(),
            "new line\n"
        );
        // Over the size, the file starts again with the newest lines.
        session.written = FILE_MAX + 1;
        session.line("after");
        assert_eq!(
            std::fs::read_to_string(dir.join(CURRENT)).unwrap(),
            "[earlier lines of this session left out: over 16 MB]\nafter\n"
        );
        drop(session);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tail_starts_at_a_whole_line() {
        let dir = temp_dir("tail");
        let path = dir.join("x.log");
        std::fs::write(&path, "first line\nsecond\nthird\n").unwrap();
        assert_eq!(
            tail(&path, 1_000),
            Some(("first line\nsecond\nthird\n".to_owned(), false))
        );
        assert_eq!(tail(&path, 10), Some(("third\n".to_owned(), true)));
        assert_eq!(tail(&dir.join("none.log"), 10), None);
        let part = section("This session", &path, 10);
        assert_eq!(
            part,
            "\n===== This session (x.log, newest lines) =====\nthird\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_crash_report_keeps_its_head_and_its_newest_lines() {
        let dir = temp_dir("crash");
        let path = dir.join("last-crash.txt");
        let head = "SJK crash report (panic)\nUptime: 18 s\n\nMessage:\nwgpu error: Out of Memory\n\nLast lines:\n";
        let trail: String = (0..100).map(|n| format!("line {n}\n")).collect();
        std::fs::write(&path, format!("{head}{trail}")).unwrap();
        let part = crash_section(&path, (head.len() + 30) as u64);
        assert!(
            part.contains("Message:\nwgpu error: Out of Memory\n"),
            "{part}"
        );
        assert!(part.ends_with("line 98\nline 99\n"), "{part}");
        assert!(!part.contains("line 0\n"), "{part}");
        // A small report goes whole.
        let whole = crash_section(&path, 1_000_000);
        assert!(whole.contains("line 0\n") && whole.contains("Out of Memory"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stamps_are_european_and_utc() {
        assert_eq!(stamp(1_791_720_000), "11/10/2026 12:00 UTC");
    }
}
