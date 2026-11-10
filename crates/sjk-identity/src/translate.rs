//! Chat translation on request: a line the player chooses goes to the hub's
//! `POST /v1/translate` (`PROTOCOL.md`, "Translate"), which asks its translator and
//! answers in English. The client never holds a translator key.
//!
//! The answers of the last [`CACHE_KEPT`] lines are kept in memory, so the same line
//! clicked twice asks the hub once.

use crate::hub::HubError;
use serde_json::Value;
use std::collections::VecDeque;

/// Longest text the hub translates, in characters.
pub const TEXT_MAX: usize = 300;
/// Translations kept in the client's cache.
pub const CACHE_KEPT: usize = 32;
/// Outcomes kept for the viewer to pick up by serial.
pub const OUTCOMES_KEPT: usize = 16;

/// A line the hub put into English.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Translation {
    /// The English text.
    pub text: String,
    /// The language the translator found, an ISO 639-1 code such as `fr`.
    pub from: String,
    /// The hub answered from its own cache.
    pub cached: bool,
    /// Translations the key has left today, when the hub says.
    pub left: Option<u32>,
}

/// Why a line was not translated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TranslateError {
    /// The hub is from before translations (404).
    NeedsUpdate,
    /// The hub has no translator (503 `translate_off`).
    Off,
    /// The key used its translations for the day (429 `translate_daily`), until this
    /// unix time when the hub says.
    Daily(Option<i64>),
    /// The hub's translator budget for the month is spent (503 `translate_quota`).
    Monthly,
    /// The translator failed (502 `translate_failed`).
    Failed,
    /// The text is empty or too long (400 `bad_text`, or refused before sending).
    BadText,
    /// Any other failure: the hub not reached, not connected, an odd answer.
    Hub(HubError),
}

impl TranslateError {
    /// The error of a refused or failed `POST /v1/translate`, `until` being the
    /// answer's `until` when it had one.
    pub fn from_hub(error: HubError, until: Option<i64>) -> Self {
        match &error {
            HubError::Rejected { status, code, .. } => match (status, code.as_str()) {
                (_, "translate_off") => Self::Off,
                (_, "translate_daily") => Self::Daily(until),
                (_, "translate_quota") => Self::Monthly,
                (_, "translate_failed") => Self::Failed,
                (_, "bad_text") => Self::BadText,
                // A hub before the route answers its generic 404.
                (404, _) => Self::NeedsUpdate,
                _ => Self::Hub(error),
            },
            HubError::Protocol(why) if why.starts_with("status 404") => Self::NeedsUpdate,
            _ => Self::Hub(error),
        }
    }
}

impl std::fmt::Display for TranslateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedsUpdate => f.write_str("Translation needs an updated SJK hub"),
            Self::Off => f.write_str("This SJK hub does not translate"),
            Self::Daily(Some(until)) => write!(
                f,
                "Daily translation limit reached, until {}",
                crate::ban::local_day_time(*until)
            ),
            Self::Daily(None) => f.write_str("Daily translation limit reached"),
            Self::Monthly => f.write_str("The hub's monthly translation limit is reached"),
            Self::Failed => f.write_str("Translation failed; try again later"),
            Self::BadText => f.write_str("Nothing to translate (1 to 300 characters)"),
            Self::Hub(error) => write!(f, "Translation failed: {error}"),
        }
    }
}

impl std::error::Error for TranslateError {}

/// What became of one translation asked for ([`crate::Service::translate`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslateOutcome {
    /// The serial [`crate::Service::translate`] gave the request.
    pub serial: u64,
    /// The text sent, as checked by [`check`].
    pub text: String,
    /// The translation, or why there is none.
    pub result: Result<Translation, TranslateError>,
}

/// The text as sent: trimmed, 1 to [`TEXT_MAX`] characters; `None` otherwise.
pub fn check(text: &str) -> Option<&str> {
    let text = text.trim();
    let chars = text.chars().count();
    (1..=TEXT_MAX).contains(&chars).then_some(text)
}

/// The translation in a `200` answer of `POST /v1/translate`.
pub fn parse_answer(answer: &Value) -> Result<Translation, HubError> {
    let text = answer["text"]
        .as_str()
        .ok_or_else(|| HubError::Protocol("the translation answer has no text".to_owned()))?;
    Ok(Translation {
        text: text.to_owned(),
        from: answer["from"].as_str().unwrap_or_default().to_owned(),
        cached: answer["cached"].as_bool().unwrap_or(false),
        left: answer["left"]
            .as_u64()
            .map(|left| u32::try_from(left).unwrap_or(u32::MAX)),
    })
}

/// The translations of the last lines asked for, newest last.
#[derive(Clone, Debug, Default)]
pub struct Cache {
    kept: VecDeque<(String, Translation)>,
}

impl Cache {
    /// The translation of `text` when it is kept; it becomes the newest.
    pub fn get(&mut self, text: &str) -> Option<Translation> {
        let at = self.kept.iter().position(|(kept, _)| kept == text)?;
        let entry = self.kept.remove(at)?;
        let translation = entry.1.clone();
        self.kept.push_back(entry);
        Some(translation)
    }

    /// Keep the translation of `text`, dropping the oldest past [`CACHE_KEPT`].
    pub fn put(&mut self, text: &str, translation: Translation) {
        self.kept.retain(|(kept, _)| kept != text);
        if self.kept.len() >= CACHE_KEPT {
            self.kept.pop_front();
        }
        self.kept.push_back((text.to_owned(), translation));
    }

    /// Lines kept.
    pub fn len(&self) -> usize {
        self.kept.len()
    }

    /// Whether no line is kept.
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rejected(status: u16, code: &str) -> HubError {
        HubError::Rejected {
            status,
            code: code.to_owned(),
            message: String::new(),
        }
    }

    #[test]
    fn a_success_answer_reads_text_language_cache_and_left() {
        let answer =
            json!({"text": "Hi, are you coming?", "from": "fr", "cached": true, "left": 49});
        assert_eq!(
            parse_answer(&answer).unwrap(),
            Translation {
                text: "Hi, are you coming?".into(),
                from: "fr".into(),
                cached: true,
                left: Some(49),
            }
        );
        let bare = parse_answer(&json!({"text": "Hello"})).unwrap();
        assert_eq!(
            (bare.from.as_str(), bare.cached, bare.left),
            ("", false, None)
        );
        assert!(matches!(
            parse_answer(&json!({"from": "fr"})),
            Err(HubError::Protocol(_))
        ));
    }

    #[test]
    fn each_error_code_has_its_meaning() {
        let of = |error| TranslateError::from_hub(error, Some(1_800_000_000));
        assert_eq!(of(rejected(404, "not_found")), TranslateError::NeedsUpdate);
        assert_eq!(
            of(HubError::Protocol("status 404: no route".into())),
            TranslateError::NeedsUpdate
        );
        assert_eq!(of(rejected(503, "translate_off")), TranslateError::Off);
        assert_eq!(
            of(rejected(429, "translate_daily")),
            TranslateError::Daily(Some(1_800_000_000))
        );
        assert_eq!(
            of(rejected(503, "translate_quota")),
            TranslateError::Monthly
        );
        assert_eq!(
            of(rejected(502, "translate_failed")),
            TranslateError::Failed
        );
        assert_eq!(of(rejected(400, "bad_text")), TranslateError::BadText);
        assert!(matches!(
            of(rejected(401, "unauthorized")),
            TranslateError::Hub(_)
        ));
        assert!(matches!(
            of(HubError::Network("down".into())),
            TranslateError::Hub(_)
        ));
        assert!(
            TranslateError::Daily(Some(1_800_000_000))
                .to_string()
                .starts_with("Daily translation limit reached, until ")
        );
        assert_eq!(
            TranslateError::NeedsUpdate.to_string(),
            "Translation needs an updated SJK hub"
        );
    }

    #[test]
    fn the_text_is_trimmed_and_bounded() {
        assert_eq!(check("  salut  "), Some("salut"));
        assert_eq!(check("   "), None);
        assert!(check(&"é".repeat(TEXT_MAX)).is_some());
        assert!(check(&"é".repeat(TEXT_MAX + 1)).is_none());
    }

    #[test]
    fn the_cache_keeps_the_newest_lines() {
        let mut cache = Cache::default();
        let made = |text: &str| Translation {
            text: text.to_owned(),
            from: "fr".into(),
            cached: false,
            left: None,
        };
        for n in 0..CACHE_KEPT + 1 {
            cache.put(&format!("ligne {n}"), made(&format!("line {n}")));
        }
        assert_eq!(cache.len(), CACHE_KEPT);
        assert!(cache.get("ligne 0").is_none(), "the oldest went");
        assert_eq!(cache.get("ligne 1").unwrap().text, "line 1");
        cache.put("ligne 1", made("again"));
        assert_eq!(cache.len(), CACHE_KEPT);
        assert_eq!(cache.get("ligne 1").unwrap().text, "again");
    }
}
