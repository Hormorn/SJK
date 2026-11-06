//! SJK's admin key login, the client's part (`docs/identity.md`, "Admin key login"): on
//! a server that offers it (`sjk_amkey 1` in its server info, a JA+ server of SJK's own
//! with admin keys), the client asks once per level, and answers with its identity key
//! only when the server's hints name that key, so a server where the player is no
//! admin never learns the key. What the player then sees is the server's own login
//! message (JA+'s "is logged as an admin council"); nothing here shows a key.
//!
//! The format is `sjk_protocol::key_login`; the signature and the host keys kept per
//! address are `sjk_identity::key_login`, the key the identity service's.

use crate::player_identity;
use sjk_client::ClientSession;
use sjk_identity::key_login::{Challenge, HOSTS_FILE, HostCheck, check_host, keep_host};
use sjk_protocol::key_login;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Mutex;

/// What tells one connection's level from another: the server, its level (`serverId`),
/// the gamestate's checksum feed and client number, and the session itself.
type Level = (SocketAddr, i32, i32, i32, usize);

/// The level the client last asked on, so it asks once per level and connection.
static ASKED: Mutex<Option<Level>> = Mutex::new(None);

/// Whether a server's `CS_SERVERINFO` offers the key login.
pub(crate) fn offered(server_info: &[u8]) -> bool {
    sjk_protocol::info_value(server_info, key_login::SERVER_INFO_KEY.as_bytes()) == Some(b"1")
}

/// Once a frame: on a server that offers it, `sjk_amkey hello` once per level, while the
/// player has an identity key.
pub(crate) fn update(session: &mut ClientSession) {
    let state = session.game_state();
    let level = (
        session.server(),
        session.server_id(),
        state.checksum_feed,
        state.client_num,
        std::ptr::from_ref(session) as usize,
    );
    {
        let Ok(mut asked) = ASKED.lock() else {
            return;
        };
        if *asked == Some(level) {
            return;
        }
        *asked = Some(level);
    }
    let server_info = session.game_state().config_string(0).unwrap_or_default();
    if !offered(server_info) || player_identity::with_service(|_| ()).is_none() {
        return;
    }
    let command = format!("{} hello", key_login::COMMAND);
    let _ = session.send_reliable_command(command.as_bytes());
}

/// What to do with the host key `host` the server at `address` sent, by the client's
/// `admin_hosts.txt` text: sign (and the file's new text, if it changes), or the line
/// that says why not.
pub(crate) fn host_step(
    text: &str,
    address: &str,
    host: &[u8; 32],
) -> Result<Option<String>, &'static str> {
    match check_host(text, address, host) {
        HostCheck::Same => Ok(None),
        HostCheck::New => Ok(Some(keep_host(text, address, host))),
        HostCheck::Changed => Err(
            "^3Admin key login: this server's host key is not the one it had, so your key did not answer. If its owner made a new one, remove its line from admin_hosts.txt beside your identity key.",
        ),
        HostCheck::Elsewhere => Err(
            "^3Admin key login: this server sent another server's host key, so your key did not answer (a server passing another one's login on).",
        ),
    }
}

/// A server's `sjk_amkey ...` command (`arguments`, its name first): a nonce answered,
/// or a refusal for the console. `config_directory` holds the identity key and
/// `admin_hosts.txt`.
pub(crate) fn answer(
    session: &mut ClientSession,
    arguments: &[Vec<u8>],
    config_directory: &Path,
) -> Vec<String> {
    let words: Vec<&[u8]> = arguments.iter().map(Vec::as_slice).collect();
    match words.get(1).copied().unwrap_or_default() {
        b"nonce" => {
            let Some(challenge) = Challenge::parse(&words[2..]) else {
                return Vec::new();
            };
            // No admin here: nothing goes back, not even the key.
            if player_identity::with_service(|service| service.key_login_named(&challenge))
                != Some(true)
            {
                return Vec::new();
            }
            let address = session.server().to_string();
            let file = config_directory.join(HOSTS_FILE);
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            match host_step(&text, &address, &challenge.host) {
                Err(line) => return vec![line.to_owned()],
                Ok(Some(kept)) => {
                    let _ = std::fs::write(&file, kept);
                }
                Ok(None) => {}
            }
            let client = u16::try_from(session.game_state().client_num).unwrap_or(u16::MAX);
            if let Some(command) = player_identity::with_service(|service| {
                service.key_login_answer(&challenge, &address, client)
            }) {
                let _ = session.send_reliable_command(command.as_bytes());
            }
            Vec::new()
        }
        b"fail" => {
            let why = String::from_utf8_lossy(words.get(2).copied().unwrap_or_default());
            match why.as_ref() {
                // The server has no keys after all, or this one was answered already.
                "none" | "late" => Vec::new(),
                why => vec![format!("^3Admin key login refused by the server ({why}).")],
            }
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_server_that_offers_it_is_asked() {
        assert!(offered(br"\sv_hostname\x\sjk_amkey\1\sjk\1"));
        assert!(!offered(br"\sv_hostname\x\sjk_amkey\\sjk\1"));
        assert!(!offered(br"\sv_hostname\x\sjk\1"));
    }

    #[test]
    fn a_host_key_is_kept_and_then_held_to() {
        let (a, b) = ([1; 32], [2; 32]);
        let kept = host_step("", "1.2.3.4:29070", &a).unwrap().unwrap();
        assert_eq!(host_step(&kept, "1.2.3.4:29070", &a), Ok(None));
        assert!(host_step(&kept, "1.2.3.4:29070", &b).is_err());
        assert!(host_step(&kept, "5.6.7.8:29070", &a).is_err());
        assert!(host_step(&kept, "5.6.7.8:29070", &b).unwrap().is_some());
    }
}
