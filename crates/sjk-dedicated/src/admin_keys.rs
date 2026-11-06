//! The server's half of SJK's admin key login (`sjk_protocol::key_login`) and the
//! one-time codes: the admins' public keys (`admin_keys.txt`), the server's host key
//! (`admin_host.key`), the hints, the signature check, and the codes the console makes
//! (`amcode`), which a client that cannot sign (the JA+ plugin, `jamp.exe`) types as
//! `amlogin <code>`.
//!
//! After VisionJK's admin login without a password (Blackwolf, GPL-2.0,
//! `codemp/game/auth/g_auth.c`): the idea and the threat model, SJK's own format and
//! code. The public keys are no secret; the host key is none either (it only tells a
//! client which server it signs for).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use sjk_protocol::key_login;
use sjk_server_mod::admin::Rank;
use std::time::{Duration, Instant};

/// The admins' keys, in the home directory: a line `<rank> <public key> [comment]`.
pub const KEYS_FILE: &str = "admin_keys.txt";
/// The server's host key, made once in the home directory (hex).
pub const HOST_FILE: &str = "admin_host.key";
/// How long a one-time code is good for, and how many wait at once.
pub const CODE_LIFE: Duration = Duration::from_secs(60);
const MAX_CODES: usize = 8;
/// A code's length and letters (no 0, O, 1 or I, which read alike).
const CODE_LENGTH: usize = 8;
const CODE_LETTERS: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// One admin's key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminKey {
    /// The rank the key logs in with.
    pub rank: Rank,
    /// Its Ed25519 public key.
    pub public: [u8; 32],
    /// What the line says after the key (who it is).
    pub comment: String,
}

/// A public key as `admin_keys.txt` writes it: the hub's unpadded base64url (what the
/// client's `identity adminkey` copies), or 64 hex digits.
pub fn parse_public(text: &str) -> Option<[u8; 32]> {
    key_login::from_hex(text.as_bytes())
        .or_else(|| {
            B64.decode(text.trim_end_matches('='))
                .ok()
                .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        })
        .filter(|public| VerifyingKey::from_bytes(public).is_ok())
}

/// The keys of `admin_keys.txt`'s `text`: blank lines and those starting with `#` are
/// skipped, a line that is not a rank and a key is told through `print` and skipped.
pub fn parse_keys(text: &str, print: &mut dyn FnMut(&[u8])) -> Vec<AdminKey> {
    let mut keys = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut words = line.split_whitespace();
        let rank = words
            .next()
            .and_then(|word| Rank::from_name(word.as_bytes()));
        let public = words.next().and_then(parse_public);
        match (rank, public) {
            (Some(rank), Some(public)) => keys.push(AdminKey {
                rank,
                public,
                comment: words.collect::<Vec<_>>().join(" "),
            }),
            _ => print(
                format!(
                    "{KEYS_FILE}:{}: not \"<council|knight|instructor|rcon> <public key> [comment]\", skipped\n",
                    number + 1
                )
                .as_bytes(),
            ),
        }
    }
    keys
}

/// `count` random bytes, `None` when the system has none.
pub fn random<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes)
}

/// The hints of `keys` for `nonce`, at most [`key_login::MAX_HINTS`].
pub fn hints(keys: &[AdminKey], nonce: &[u8; 32]) -> Vec<String> {
    keys.iter()
        .take(key_login::MAX_HINTS)
        .map(|key| {
            let digest = Sha256::digest(key_login::hint_input(nonce, &key.public));
            key_login::hex(&digest[..key_login::HINT_BYTES])
        })
        .collect()
}

/// Whether `signature` is `public`'s over `message` (strict Ed25519).
pub fn verify(public: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    VerifyingKey::from_bytes(public).is_ok_and(|key| {
        key.verify_strict(message, &Signature::from_bytes(signature))
            .is_ok()
    })
}

/// The host key in `admin_host.key`'s text.
pub fn parse_host(text: &[u8]) -> Option<[u8; 32]> {
    let text = std::str::from_utf8(text).ok()?.trim();
    key_login::from_hex(text.as_bytes())
}

/// The one-time codes waiting.
#[derive(Clone, Debug, Default)]
pub struct Codes {
    waiting: Vec<(String, Rank, Instant)>,
}

impl Codes {
    /// A new code for one login with `rank`, made from `random` bytes at `now`; the
    /// oldest goes when too many wait.
    pub fn make(&mut self, rank: Rank, random: [u8; CODE_LENGTH], now: Instant) -> String {
        self.waiting
            .retain(|(_, _, made)| now.saturating_duration_since(*made) <= CODE_LIFE);
        if self.waiting.len() >= MAX_CODES {
            self.waiting.remove(0);
        }
        let code: String = random
            .iter()
            .map(|byte| char::from(CODE_LETTERS[usize::from(byte & 31)]))
            .collect();
        self.waiting.push((code.clone(), rank, now));
        code
    }

    /// The rank of the code `typed` is (any letter case) if it waits still, the code
    /// used up by it; every code is looked at, a match or not.
    pub fn take(&mut self, typed: &[u8], now: Instant) -> Option<Rank> {
        self.waiting
            .retain(|(_, _, made)| now.saturating_duration_since(*made) <= CODE_LIFE);
        if typed.len() != CODE_LENGTH {
            return None;
        }
        let mut found = None;
        for (index, (code, rank, _)) in self.waiting.iter().enumerate() {
            let differs = code
                .bytes()
                .zip(typed)
                .fold(0, |differs, (a, b)| differs | (a ^ b.to_ascii_uppercase()));
            if differs == 0 {
                found = Some((index, *rank));
            }
        }
        let (index, rank) = found?;
        self.waiting.remove(index);
        Some(rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn the_key_file_takes_the_hubs_form_and_hex() {
        let (a, b) = (
            key(1).verifying_key().to_bytes(),
            key(2).verifying_key().to_bytes(),
        );
        let text = format!(
            "# admins\n\ncouncil {} Sol the owner\nRCON {}\nknight nothing\nbogus {}\n",
            B64.encode(a),
            key_login::hex(&b),
            B64.encode(a)
        );
        let mut told = Vec::new();
        let keys = parse_keys(&text, &mut |line| told.extend_from_slice(line));
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].rank, Rank::Council);
        assert_eq!(keys[0].public, a);
        assert_eq!(keys[0].comment, "Sol the owner");
        assert_eq!(keys[1].rank, Rank::Rcon);
        assert_eq!(keys[1].public, b);
        let told = String::from_utf8(told).unwrap();
        assert!(told.contains(":5:") && told.contains(":6:"), "{told}");
    }

    #[test]
    fn a_signature_is_checked_strictly() {
        let signing = key(3);
        let public = signing.verifying_key().to_bytes();
        let message = key_login::message(&[1; 32], &[2; 32], "127.0.0.1:29186", 0);
        let signature = signing.sign(&message).to_bytes();
        assert!(verify(&public, &message, &signature));
        let other = key_login::message(&[1; 32], &[2; 32], "127.0.0.1:29187", 0);
        assert!(!verify(&public, &other, &signature));
        assert!(!verify(
            &key(4).verifying_key().to_bytes(),
            &message,
            &signature
        ));
    }

    #[test]
    fn hints_differ_by_key_and_nonce() {
        let keys = [
            AdminKey {
                rank: Rank::Knight,
                public: key(1).verifying_key().to_bytes(),
                comment: String::new(),
            },
            AdminKey {
                rank: Rank::Knight,
                public: key(2).verifying_key().to_bytes(),
                comment: String::new(),
            },
        ];
        let first = hints(&keys, &[1; 32]);
        assert_eq!(first.len(), 2);
        assert_ne!(first[0], first[1]);
        assert_ne!(first, hints(&keys, &[2; 32]));
        assert_eq!(first[0].len(), key_login::HINT_BYTES * 2);
    }

    #[test]
    fn a_code_works_once_and_for_a_minute() {
        let now = Instant::now();
        let mut codes = Codes::default();
        let code = codes.make(Rank::Knight, [0, 1, 2, 3, 4, 5, 6, 31], now);
        assert_eq!(code, "ABCDEFG9");
        assert_eq!(codes.take(b"abcdefg9", now), Some(Rank::Knight));
        assert_eq!(codes.take(b"ABCDEFG9", now), None);
        let code = codes.make(Rank::Council, [9; 8], now);
        assert_eq!(
            codes.take(code.as_bytes(), now + CODE_LIFE + Duration::from_secs(1)),
            None
        );
        assert_eq!(codes.take(b"short", now), None);
    }

    #[test]
    fn the_host_key_is_hex() {
        assert_eq!(
            parse_host(format!("{}\n", "ab".repeat(32)).as_bytes()),
            Some([0xab; 32])
        );
        assert_eq!(parse_host(b"nope"), None);
    }
}
