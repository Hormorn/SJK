//! SJK's admin key login: what an SJK client signs to log in as an admin on a server
//! that offers it, and how the two sides write the exchange. It travels in ordinary
//! reliable commands (`sjk_amkey ...`), so nothing of protocol 26's framing changes, and
//! no other client ever sees it: a server offers it in its server info
//! ([`SERVER_INFO_KEY`]) and only an SJK client asks.
//!
//! The exchange, every value lower-case hex:
//!
//! ```text
//! client  sjk_amkey hello
//! server  sjk_amkey nonce <nonce> <host key> <hint> <hint> ...
//!         sjk_amkey ok <rank> kept      (the player has its key's rank already)
//!         sjk_amkey fail <why>
//! client  sjk_amkey sign <public key> <signature> <address>
//! server  sjk_amkey ok <rank> | sjk_amkey fail <why>
//! ```
//!
//! The nonce is 32 random bytes, good for one answer within [`NONCE_MSEC`]. A hint per
//! admin key ([`hint_input`]) lets a client tell whether its key is one of them without
//! naming it: a player who is no admin there sends nothing, so a server never learns
//! the keys of its other players. The signed [`message`] starts with a fixed line, so
//! the signature is worth nothing for anything else the key signs (the hub's requests,
//! whispers); it holds the nonce (against a replay), the server's host key and its
//! address as the client sees it (against a server that passes another server's nonce
//! on to its players) and the client number (against another slot using it).
//!
//! The idea is VisionJK's admin login without a password (Blackwolf, GPL-2.0,
//! `codemp/game/auth/bg_auth.h`), the format is SJK's own.

/// The command both sides send.
pub const COMMAND: &str = "sjk_amkey";
/// The server-info key a server offering the key login sets (to `1`).
pub const SERVER_INFO_KEY: &str = "sjk_amkey";
/// The first line of every signed message.
pub const MESSAGE_PREFIX: &[u8] = b"SJK admin key login 1\n";
/// The first line of every hint's input.
pub const HINT_PREFIX: &[u8] = b"SJK admin key hint 1\n";
/// How long a nonce waits for its answer.
pub const NONCE_MSEC: i32 = 30_000;
/// The bytes of a hint kept (of its SHA-256).
pub const HINT_BYTES: usize = 8;
/// At most this many hints in one nonce command, which must stay a command's size.
pub const MAX_HINTS: usize = 40;

/// The message a client signs and a server checks.
pub fn message(nonce: &[u8; 32], host: &[u8; 32], address: &str, client: u16) -> Vec<u8> {
    let mut out = MESSAGE_PREFIX.to_vec();
    for (index, part) in [
        hex(nonce),
        hex(host),
        address.to_owned(),
        client.to_string(),
    ]
    .iter()
    .enumerate()
    {
        if index > 0 {
            out.push(b'\n');
        }
        out.extend_from_slice(part.as_bytes());
    }
    out.push(b'\n');
    out
}

/// What is hashed (SHA-256, the first [`HINT_BYTES`] kept) into the hint of one admin
/// key for one nonce.
pub fn hint_input(nonce: &[u8; 32], public: &[u8; 32]) -> Vec<u8> {
    [HINT_PREFIX, nonce, public].concat()
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}

/// Exactly `N` bytes of hex (either case), or `None`.
pub fn from_hex<const N: usize>(text: &[u8]) -> Option<[u8; N]> {
    if text.len() != N * 2 {
        return None;
    }
    let digit = |byte: u8| char::from(byte).to_digit(16);
    let mut out = [0_u8; N];
    for (slot, pair) in out.iter_mut().zip(text.chunks_exact(2)) {
        *slot = (digit(pair[0])? << 4 | digit(pair[1])?) as u8;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_is_fixed_text_with_every_part_on_its_line() {
        let message = message(&[1; 32], &[0xab; 32], "127.0.0.1:29070", 3);
        let text = String::from_utf8(message).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "SJK admin key login 1");
        assert_eq!(lines[1], "01".repeat(32));
        assert_eq!(lines[2], "ab".repeat(32));
        assert_eq!(lines[3], "127.0.0.1:29070");
        assert_eq!(lines[4], "3");
        assert!(text.ends_with("3\n"));
    }

    #[test]
    fn hex_goes_both_ways() {
        assert_eq!(hex(&[0, 0x7f, 0xff]), "007fff");
        assert_eq!(from_hex::<3>(b"007FfF"), Some([0, 0x7f, 0xff]));
        assert_eq!(from_hex::<3>(b"007ff"), None);
        assert_eq!(from_hex::<1>(b"zz"), None);
    }

    #[test]
    fn a_hint_names_the_key_and_the_nonce() {
        let input = hint_input(&[2; 32], &[3; 32]);
        assert!(input.starts_with(HINT_PREFIX));
        assert_eq!(input.len(), HINT_PREFIX.len() + 64);
    }
}
