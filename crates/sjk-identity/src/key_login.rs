//! The player's half of SJK's admin key login (`sjk_protocol::key_login`): whether a
//! server's hints name this key, the signature over the login message, and which host
//! key each server address had (`admin_hosts.txt`, beside the key file).
//!
//! The key signs only the login message, which starts with its own fixed line, so no
//! server can make it sign anything else. The host keys are VisionJK's protection
//! against a server that passes another server's nonce on (`codemp/game/auth/bg_auth.h`):
//! the first login at an address keeps that server's host key, and from then on the
//! client signs for that address only with that key, and never with a key it keeps for
//! another address.

use crate::keys::Identity;
use sha2::{Digest, Sha256};
use sjk_protocol::key_login;

/// The file beside the key that keeps each address's host key, a line each.
pub const HOSTS_FILE: &str = "admin_hosts.txt";
/// At most this many addresses are kept; the oldest go first.
const MAX_HOSTS: usize = 256;

/// What a server sent with its nonce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Challenge {
    /// Good for one answer.
    pub nonce: [u8; 32],
    /// The server's host key.
    pub host: [u8; 32],
    /// One per admin key of the server ([`hint`]).
    pub hints: Vec<[u8; key_login::HINT_BYTES]>,
}

impl Challenge {
    /// `sjk_amkey nonce <nonce> <host> <hint>...`'s words after `nonce`.
    pub fn parse(words: &[&[u8]]) -> Option<Self> {
        let (nonce, rest) = words.split_first()?;
        let (host, hints) = rest.split_first()?;
        Some(Self {
            nonce: key_login::from_hex(nonce)?,
            host: key_login::from_hex(host)?,
            hints: hints
                .iter()
                .take(key_login::MAX_HINTS)
                .filter_map(|hint| key_login::from_hex(hint))
                .collect(),
        })
    }
}

/// The hint of `public` for `nonce`.
pub fn hint(nonce: &[u8; 32], public: &[u8; 32]) -> [u8; key_login::HINT_BYTES] {
    let digest = Sha256::digest(key_login::hint_input(nonce, public));
    let mut hint = [0; key_login::HINT_BYTES];
    hint.copy_from_slice(&digest[..key_login::HINT_BYTES]);
    hint
}

/// What `admin_hosts.txt` says of an address and a host key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostCheck {
    /// Neither is in it: the first login there, which keeps the key.
    New,
    /// The address with this key.
    Same,
    /// The address with another key: the server is another one now, or someone stands
    /// between.
    Changed,
    /// The key, for another address: a server that passes another one's nonce on.
    Elsewhere,
}

/// The file's lines, `(address, host key)`, in order, oldest first.
fn hosts(text: &str) -> Vec<(String, [u8; 32])> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let address = words.next()?;
            let host = key_login::from_hex(words.next()?.as_bytes())?;
            Some((address.to_owned(), host))
        })
        .collect()
}

/// What the file `text` says of `address` and `host`.
pub fn check_host(text: &str, address: &str, host: &[u8; 32]) -> HostCheck {
    let known = hosts(text);
    if let Some((_, kept)) = known
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(address))
    {
        return if kept == host {
            HostCheck::Same
        } else {
            HostCheck::Changed
        };
    }
    if known.iter().any(|(_, kept)| kept == host) {
        HostCheck::Elsewhere
    } else {
        HostCheck::New
    }
}

/// The file `text` with `address` kept with `host`, at its end.
pub fn keep_host(text: &str, address: &str, host: &[u8; 32]) -> String {
    let mut known: Vec<(String, [u8; 32])> = hosts(text)
        .into_iter()
        .filter(|(known, _)| !known.eq_ignore_ascii_case(address))
        .collect();
    known.push((address.to_owned(), *host));
    let skip = known.len().saturating_sub(MAX_HOSTS);
    known
        .iter()
        .skip(skip)
        .map(|(address, host)| format!("{address} {}\n", key_login::hex(host)))
        .collect()
}

impl Identity {
    /// Whether `challenge` holds this key's hint.
    pub fn named_by(&self, challenge: &Challenge) -> bool {
        let own = hint(&challenge.nonce, &self.public_key());
        challenge.hints.contains(&own)
    }

    /// The answer to `challenge` from client `client` of the server at `address`, as the
    /// client sees it: `sjk_amkey sign <public key> <signature> <address>`.
    pub fn key_login_answer(&self, challenge: &Challenge, address: &str, client: u16) -> String {
        let message = key_login::message(&challenge.nonce, &challenge.host, address, client);
        let signature = self.sign_bytes(&message);
        format!(
            "{} sign {} {} {address}",
            key_login::COMMAND,
            key_login::hex(&self.public_key()),
            key_login::hex(&signature)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, VerifyingKey};

    #[test]
    fn a_key_answers_only_the_hint_that_names_it() {
        let identity = Identity::from_seed([7; 32]);
        let other = Identity::from_seed([8; 32]);
        let nonce = [1; 32];
        let challenge = Challenge {
            nonce,
            host: [2; 32],
            hints: vec![hint(&nonce, &other.public_key())],
        };
        assert!(!identity.named_by(&challenge));
        assert!(other.named_by(&challenge));
        // The same key with another nonce has another hint.
        assert_ne!(
            hint(&nonce, &identity.public_key()),
            hint(&[9; 32], &identity.public_key())
        );
    }

    #[test]
    fn the_answer_is_a_signature_of_the_login_message() {
        let identity = Identity::from_seed([7; 32]);
        let challenge = Challenge {
            nonce: [1; 32],
            host: [2; 32],
            hints: Vec::new(),
        };
        let answer = identity.key_login_answer(&challenge, "127.0.0.1:29070", 5);
        let words: Vec<&str> = answer.split(' ').collect();
        assert_eq!(words[..2], ["sjk_amkey", "sign"]);
        assert_eq!(words[4], "127.0.0.1:29070");
        let public: [u8; 32] = key_login::from_hex(words[2].as_bytes()).unwrap();
        let signature: [u8; 64] = key_login::from_hex(words[3].as_bytes()).unwrap();
        let message = key_login::message(&[1; 32], &[2; 32], "127.0.0.1:29070", 5);
        let key = VerifyingKey::from_bytes(&public).unwrap();
        assert!(
            key.verify_strict(&message, &Signature::from_bytes(&signature))
                .is_ok()
        );
        // Not for another slot.
        let other = key_login::message(&[1; 32], &[2; 32], "127.0.0.1:29070", 6);
        assert!(
            key.verify_strict(&other, &Signature::from_bytes(&signature))
                .is_err()
        );
    }

    #[test]
    fn a_challenge_reads_from_its_words() {
        let nonce = "01".repeat(32);
        let host = "02".repeat(32);
        let hint = "03".repeat(8);
        let words = [nonce.as_bytes(), host.as_bytes(), hint.as_bytes(), b"bad"];
        let challenge = Challenge::parse(&words).unwrap();
        assert_eq!(challenge.nonce, [1; 32]);
        assert_eq!(challenge.hints, vec![[3; 8]]);
        assert!(Challenge::parse(&words[..1]).is_none());
    }

    #[test]
    fn hosts_are_kept_per_address() {
        let (a, b) = ([1; 32], [2; 32]);
        assert_eq!(check_host("", "1.2.3.4:29070", &a), HostCheck::New);
        let text = keep_host("", "1.2.3.4:29070", &a);
        assert_eq!(check_host(&text, "1.2.3.4:29070", &a), HostCheck::Same);
        assert_eq!(check_host(&text, "1.2.3.4:29070", &b), HostCheck::Changed);
        assert_eq!(check_host(&text, "5.6.7.8:29070", &a), HostCheck::Elsewhere);
        assert_eq!(check_host(&text, "5.6.7.8:29070", &b), HostCheck::New);
        let text = keep_host(&text, "1.2.3.4:29070", &b);
        assert_eq!(text.lines().count(), 1);
        assert_eq!(check_host(&text, "1.2.3.4:29070", &b), HostCheck::Same);
    }
}
