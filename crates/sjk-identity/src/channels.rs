//! The hub chat's channels (Sol-Vulpes/SJK-hub `PROTOCOL.md`, "Clan chats", since
//! 03/11/2026, Sol's decision from Lex's idea): the SJK chat every SJK player shares, and
//! two clan chats beside it, JoF's and the Sith clan's (`{{S}}`). The hub decides who may
//! read and write a clan chat (a verified player of that clan's tick group only, or
//! staff: a JoF player never reads S nor an S player JoF, Sol 05/11/2026)
//! and sends a reader only the channels it asks for ([`wanted_query`]); the feed lists the
//! clan chats the reader may read ([`ListedChannel`]), with the ones it should join by
//! default (`home`).

use serde::Deserialize;

/// One channel of the hub's chat.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HubChannel {
    /// The SJK chat (the hub leaves the field out).
    #[default]
    #[serde(rename = "")]
    Sjk,
    /// JoF's clan chat.
    Jof,
    /// The Sith clan's chat (`{{S}}`), shown as S.
    Sith,
    /// A channel a later hub added; its messages are dropped.
    #[serde(other)]
    Other,
}

/// The clan chats, in the order the hub lists them and the composer offers them.
pub const CLANS: [HubChannel; 2] = [HubChannel::Jof, HubChannel::Sith];

impl HubChannel {
    /// Its id on the wire: empty for the SJK chat.
    pub fn wire(self) -> &'static str {
        match self {
            Self::Sjk | Self::Other => "",
            Self::Jof => "jof",
            Self::Sith => "sith",
        }
    }

    /// The short name lines and buttons show: `SJK`, `JoF`, `S`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sjk | Self::Other => "SJK",
            Self::Jof => "JoF",
            Self::Sith => "S",
        }
    }

    /// Whether it is one of the clan chats.
    pub fn is_clan(self) -> bool {
        matches!(self, Self::Jof | Self::Sith)
    }
}

/// A clan chat the reader may read, as the feed lists it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct ListedChannel {
    /// The clan chat.
    pub id: HubChannel,
    /// The reader's home channel (their clan's, or every one for staff): joined unless
    /// the player left it.
    #[serde(default)]
    pub home: bool,
}

/// The feed request's `channels` value for the clan chats `wanted`, `None` for none.
pub fn wanted_query(wanted: &[HubChannel]) -> Option<String> {
    let ids: Vec<&str> = wanted
        .iter()
        .filter(|channel| channel.is_clan())
        .map(|channel| channel.wire())
        .collect();
    (!ids.is_empty()).then(|| ids.join(","))
}

/// The clan chats to read: those `listed` (the ones the hub lets the player read) the
/// player joined. `choice` gives the player's choice for one: `Some(true)` joined,
/// `Some(false)` left, `None` the default, which joins the home channels.
pub fn joined(
    listed: &[ListedChannel],
    choice: impl Fn(HubChannel) -> Option<bool>,
) -> Vec<HubChannel> {
    listed
        .iter()
        .filter(|channel| channel.id.is_clan() && choice(channel.id).unwrap_or(channel.home))
        .map(|channel| channel.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_read_from_the_wire_and_unknown_ones_stand_apart() {
        let read = |json: &str| serde_json::from_str::<HubChannel>(json).unwrap();
        assert_eq!(read("\"\""), HubChannel::Sjk);
        assert_eq!(read("\"jof\""), HubChannel::Jof);
        assert_eq!(read("\"sith\""), HubChannel::Sith);
        assert_eq!(read("\"later\""), HubChannel::Other);
        assert_eq!(HubChannel::Sith.label(), "S");
    }

    #[test]
    fn home_channels_are_joined_unless_left_and_others_only_when_chosen() {
        let listed = [
            ListedChannel {
                id: HubChannel::Jof,
                home: true,
            },
            ListedChannel {
                id: HubChannel::Sith,
                home: false,
            },
        ];
        assert_eq!(joined(&listed, |_| None), [HubChannel::Jof]);
        assert_eq!(
            joined(&listed, |channel| (channel == HubChannel::Sith)
                .then_some(true)),
            [HubChannel::Jof, HubChannel::Sith]
        );
        assert!(joined(&listed, |_| Some(false)).is_empty());
        // Nothing the hub does not list, whatever the player chose.
        assert!(joined(&[], |_| Some(true)).is_empty());
        assert_eq!(wanted_query(&[]), None);
        assert_eq!(
            wanted_query(&[HubChannel::Jof, HubChannel::Sith]).as_deref(),
            Some("jof,sith")
        );
    }
}
