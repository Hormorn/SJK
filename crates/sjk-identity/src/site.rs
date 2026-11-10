//! The players' site (`docs/identity.md`, "Your SJK page"): SJK's web page, where a
//! player signs in from the game (no password: their key is their account), joins their
//! other keys and sends videos for the SJK team's video bounties. The game asks the hub
//! for a one-use sign-in link to open in the browser, and types the link codes the site
//! makes (it names the account and asks before this key joins it). A code is never shown
//! in the game, where a stream could show it. The private key never leaves this PC.

use serde::Deserialize;

/// A one-use sign-in link (`POST /v1/site/sign-in`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SignInLink {
    /// The link to open in the browser.
    pub url: String,
    /// When it stops working, unix seconds.
    pub expires: i64,
}

/// What the hub says of a link code typed in the game (`POST /v1/site/link`): whose
/// account it joins this key to, and once confirmed how many keys that account has.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Linked {
    /// The account's display name (colour codes and all).
    pub account: String,
    /// Set once the key joined.
    #[serde(default)]
    pub keys: Option<u32>,
}

/// A link code as the player typed it: eight letters or digits, dashes and spaces
/// ignored; `None` when it cannot be one.
pub fn code(typed: &str) -> Option<String> {
    let code: String = typed
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    (code.len() == 8 && code.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| format!("{}-{}", &code[..4], &code[4..]))
}

/// What the player asked the site for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SiteAnswer {
    /// A sign-in link to open.
    SignIn(SignInLink),
    /// The account a typed code joins, before or after confirming.
    Link(Linked),
}

/// What became of the last site request: its serial, and the answer or why not, in the
/// player's words.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SiteOutcome {
    /// Counts outcomes, so each is acted on once.
    pub serial: u64,
    /// The answer, or why not in the player's words.
    pub result: Result<SiteAnswer, String>,
}

/// The player's words for a refused site request.
pub fn refusal(error: &crate::HubError) -> String {
    use crate::HubError;
    match error {
        HubError::Rejected { code, .. } if code == "no_site" => {
            "This SJK hub has no players' site yet.".to_owned()
        }
        HubError::Rejected { code, .. } if code == "no_code" => {
            "That code does not work: make a new one on your SJK page.".to_owned()
        }
        HubError::Rejected { code, .. } if code == "from_staff" => {
            "A staff key cannot join another account.".to_owned()
        }
        HubError::Rejected { message, .. } if !message.is_empty() => message.clone(),
        HubError::Rejected { code, .. } => format!("The SJK hub refused it ({code})."),
        HubError::Network(_) => "Cannot reach the SJK hub: try again in a moment.".to_owned(),
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub has no players' site yet.".to_owned()
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_as_typed() {
        assert_eq!(code("abcd-efgh").as_deref(), Some("ABCD-EFGH"));
        assert_eq!(code(" ABCD EFGH ").as_deref(), Some("ABCD-EFGH"));
        assert_eq!(code("ABCDEFG"), None);
        assert_eq!(code("ABCD-EFG!"), None);
    }

    #[test]
    fn answers_read() {
        let linked: Linked = serde_json::from_str(r#"{"account":"^1Sol"}"#).unwrap();
        assert_eq!(linked.keys, None);
        let linked: Linked = serde_json::from_str(r#"{"account":"Sol","keys":2}"#).unwrap();
        assert_eq!(linked.keys, Some(2));
    }
}
