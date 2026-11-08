//! Client icons (`docs/hub-chat.md`, "How a line looks"): an SJK chat message sent from
//! another client that takes part in SJK chat starts with that client's icon instead of
//! SJK's emblem. The hub reads the client from the sender's User-Agent
//! (`ChatMessage::client`). The clients are the entries of [`KNOWN`], read by generic
//! code: supporting one means adding an entry and its icon, not code. [`CVAR`] 0 gives
//! every message SJK's emblem.

use std::sync::atomic::{AtomicBool, Ordering};

/// 0 shows SJK's emblem before every SJK chat message.
pub(crate) const CVAR: &str = "cl_sjkChatClientIcons";

/// A client whose messages start with its own icon. Its players wear it on the
/// scoreboard and the player card too, in place of SJK's emblem (presence's `client`).
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ClientIcon {
    /// `ChatMessage::client` as the hub sends it.
    pub(crate) client: &'static str,
    /// The client's name, for the docs and the tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) name: &'static str,
    pub(crate) texture: sjk_ui::TextureId,
    /// Why it is here, and the evidence.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) why: &'static str,
    /// A game client in SJK chat that does not claim its slot: a scoreboard row the hub
    /// holds no claim for, whose name is an online SJK chat reader's, is taken to be on
    /// it ([`unclaimed`], `scoreboard/chat_readers.rs`). One entry at most.
    pub(crate) unclaimed: bool,
}

pub(crate) const KNOWN: &[ClientIcon] = &[
    ClientIcon {
        client: "ejk",
        name: "JoF EternalJK",
        texture: crate::ui_renderer::EJK_TEXTURE,
        why: "Takes part in SJK chat with the same keys and identity files \
              (JediofFreedom/JoF_EJK#398, SJK #194); the hub marks its messages \"ejk\". \
              Its released builds read the chat without claiming their slot (the claims \
              wait in JediofFreedom/JoF_EJK#405), and it is the only game client in SJK chat \
              besides SJK, which claims: Sol, 08/11/2026, \"assume they are on JoF EJK\"",
        unclaimed: true,
    },
    ClientIcon {
        client: "launcher",
        name: "SJK Launcher",
        texture: crate::ui_renderer::LAUNCHER_TEXTURE,
        why: "Its chat window signs with the player's own identity.key from SJK's settings \
              folder (Sol-Vulpes/sjk-launcher, src/chat.rs); the hub marks its messages \
              \"launcher\" from the User-Agent SJKLauncher/<version> (SJK-hub clients.rs, \
              27/10/2026)",
        unclaimed: false,
    },
];

/// The icon for messages from `client`, `None` for SJK's emblem.
pub(crate) fn find(client: &str) -> Option<&'static ClientIcon> {
    KNOWN.iter().find(|icon| icon.client == client)
}

/// The client a player who reads SJK chat but holds no claim is taken to be on, while
/// [`CVAR`] is on (with it off such a player gets no mark: SJK's emblem would say more
/// than a name shows).
pub(crate) fn unclaimed() -> Option<&'static ClientIcon> {
    KNOWN
        .iter()
        .find(|icon| icon.unclaimed)
        .filter(|_| SHOWN.load(Ordering::Relaxed))
}

static SHOWN: AtomicBool = AtomicBool::new(true);

/// Follow [`CVAR`] (once a frame).
pub(crate) fn set_shown(on: bool) {
    SHOWN.store(on, Ordering::Relaxed);
}

/// The texture before a message from `icon`'s client: its own while [`CVAR`] is on,
/// else SJK's emblem.
pub(crate) fn texture(icon: Option<&'static ClientIcon>) -> sjk_ui::TextureId {
    icon.filter(|_| SHOWN.load(Ordering::Relaxed))
        .map_or(crate::ui_renderer::LOGO_TEXTURE, |icon| icon.texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_unique_and_says_why() {
        for (index, icon) in KNOWN.iter().enumerate() {
            assert!(!icon.why.is_empty() && !icon.name.is_empty());
            assert_ne!(icon.client, "sjk", "SJK's own messages keep its emblem");
            assert!(
                KNOWN[..index]
                    .iter()
                    .all(|other| other.client != icon.client),
                "{}",
                icon.client
            );
        }
        assert!(find("ejk").is_some() && find("launcher").is_some());
        assert!(
            KNOWN.iter().filter(|icon| icon.unclaimed).count() <= 1,
            "one client is assumed at most"
        );
        assert!(find("").is_none() && find("sjk").is_none());
    }
}
