//! Chat translation on request (Creo's idea, 10/11/2026; `docs/client.md`, "Chat
//! translation"): a line the player chooses goes to the SJK hub, which asks its
//! translator for it in English. Nothing is translated on its own and nothing goes to
//! the game server.
//!
//! - `/tr` typed in the chat field (or the console's `tr`) translates the newest line in
//!   the feed that looks foreign: its language, guessed here offline (`whatlang`), is
//!   neither English nor one of `cl_chatTranslateSkip`'s. `/tr <text>` translates the
//!   text given.
//! - The SJK chat page offers Translate on the chosen message (`sjk_chat_panel.rs`).
//!
//! The translation shows under the line in the feed, dim, as `» (fr) ...`, and goes to
//! the console; why there is none (an older hub, a hub without a translator, the day's
//! or the month's limit) shows as a local notice. `cl_chatTranslate 0` turns it off.

use super::{Channel, ChatLine, ChatOverlay, layout};
use std::time::Instant;

/// `cl_chatTranslate`: whether chat lines may be translated (`/tr`, `tr`, the SJK chat
/// page's Translate).
pub(crate) const CVAR: &str = "cl_chatTranslate";
/// `cl_chatTranslateSkip`: the languages never offered, ISO 639-1 codes.
pub(crate) const SKIP_CVAR: &str = "cl_chatTranslateSkip";
/// `cl_chatTranslateSkip`'s default: English only.
pub(crate) const SKIP_DEFAULT: &str = "en";
/// The console command, and the chat field's `/tr`.
pub(crate) const COMMAND: &str = "tr";
/// Translations waiting for the hub at once; an older one is forgotten past it.
const ASKS_KEPT: usize = 8;
/// Words a line needs before its guessed language counts without a script of its own.
const FEW_WORDS: usize = 3;
/// How sure the guess must be for a line of [`FEW_WORDS`] or more.
const SURE_ENOUGH: f64 = 0.3;

/// The languages never offered for translation (`cl_chatTranslateSkip`), lower-case
/// ISO 639-1 codes. English is never offered whatever the list says.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Skip(Vec<String>);

impl Skip {
    /// The codes of `text`, separated by spaces or commas (`"en sk, cs"`); anything that
    /// is not two or three letters is left out.
    pub(crate) fn parse(text: &str) -> Self {
        let mut codes: Vec<String> = text
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .filter(|code| (2..=3).contains(&code.len()))
            .filter(|code| code.chars().all(|c| c.is_ascii_alphabetic()))
            .map(str::to_ascii_lowercase)
            .collect();
        codes.dedup();
        Self(codes)
    }

    /// Whether lines in the language of `code` (ISO 639-1) are never offered.
    pub(crate) fn skips(&self, code: &str) -> bool {
        code == "en"
            || self.0.iter().any(|kept| {
                kept == code
                    // Norwegian: whatlang knows Bokmål, players write `no`.
                    || (kept == "no" && code == "nb")
            })
    }
}

/// What the language guess said of a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Guess {
    /// ISO 639-1 code.
    pub(crate) code: &'static str,
    /// The line counts as in that language: sure enough for its length.
    pub(crate) sure: bool,
}

/// `text` as it is guessed and sent: no colour codes (`^1`), no emoji marks (private
/// use characters), spaces collapsed.
pub(crate) fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '^' && characters.peek().is_some_and(char::is_ascii_alphanumeric) {
            characters.next();
            continue;
        }
        if ('\u{E000}'..='\u{F8FF}').contains(&character) || character.is_control() {
            out.push(' ');
            continue;
        }
        out.push(character);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The language `plain` text looks to be in. A line of fewer than [`FEW_WORDS`] words
/// counts only when its script says so alone (Cyrillic, Greek, Han...) and the guess
/// is reliable; "gg", "lol" and names guess at random.
pub(crate) fn guess(plain: &str) -> Option<Guess> {
    let info = whatlang::detect(plain)?;
    let code = iso639_1(info.lang());
    let words = plain
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphabetic))
        .count();
    let sure = if words < FEW_WORDS {
        info.is_reliable() && info.script() != whatlang::Script::Latin
    } else {
        info.is_reliable() || info.confidence() >= SURE_ENOUGH
    };
    Some(Guess { code, sure })
}

/// The language of `plain` text when it counts as foreign: guessed surely, and neither
/// English nor skipped.
pub(crate) fn foreign(plain: &str, skip: &Skip) -> Option<&'static str> {
    guess(plain)
        .filter(|guess| guess.sure && !skip.skips(guess.code))
        .map(|guess| guess.code)
}

/// Whether the SJK chat page offers Translate for `plain` text: unless it is surely in
/// English or a skipped language (a short line the guess cannot tell is offered).
pub(crate) fn offered(plain: &str, skip: &Skip) -> bool {
    !plain.is_empty() && !guess(plain).is_some_and(|guess| guess.sure && skip.skips(guess.code))
}

/// What a draft typed in the chat field asks: `None` when it is no `/tr`, otherwise
/// the text to translate (`None` inside for the newest foreign line).
pub(crate) fn typed(draft: &str) -> Option<Option<&str>> {
    let draft = draft.trim_start();
    let rest = draft.strip_prefix('/')?;
    let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    if !rest[..word_end].eq_ignore_ascii_case(COMMAND) {
        return None;
    }
    let text = rest[word_end..].trim();
    Some((!text.is_empty()).then_some(text))
}

/// The line a translation is for, as the feed holds it: when it came and its text.
type LineKey = (u64, String);

/// Translations waiting for the hub.
#[derive(Debug, Default)]
pub(crate) struct Asks(Vec<(u64, Option<LineKey>)>);

/// A translation's line: `» (fr) Hi`.
pub(crate) fn line(translation: &sjk_identity::Translation) -> String {
    let from = if translation.from.is_empty() {
        "?"
    } else {
        translation.from.as_str()
    };
    format!("\u{bb} ({from}) {}", translation.text)
}

/// A translation's line in the feed and the console, dim.
pub(crate) fn shown(translation: &sjk_identity::Translation) -> String {
    format!("^9{}", line(translation))
}

impl ChatOverlay {
    /// The newest line in the feed that looks foreign ([`foreign`]): its key, for the
    /// translation to go under it, and its plain text, to send.
    pub(crate) fn last_foreign(&self, skip: &Skip) -> Option<(LineKey, String)> {
        self.lines
            .iter()
            .rev()
            .filter(|line| !line.muted && !line.translated)
            .find_map(|line| {
                let text = plain(&line.body);
                foreign(&text, skip)?;
                Some(((line.received_ms, line.body.clone()), text))
            })
    }

    /// Note that the hub was asked, under `serial`, for the line of `key` (none for a
    /// text given with `/tr <text>`).
    pub(crate) fn translation_asked(&mut self, serial: u64, key: Option<LineKey>) {
        if self.translations.0.len() == ASKS_KEPT {
            self.translations.0.remove(0);
        }
        self.translations.0.push((serial, key));
    }

    /// Whether a translation is waiting for the hub: only then does the frame look.
    pub(crate) fn translations_waiting(&self) -> bool {
        !self.translations.0.is_empty()
    }

    /// Forget the waiting translations (the identity stopped).
    pub(crate) fn forget_translations(&mut self) {
        self.translations.0.clear();
    }

    /// Show the outcomes in `state` of the translations waiting: each translation under
    /// its line (at the end when the line is gone or there was none), or why there is
    /// none as a notice. Returns the lines for the console.
    pub(crate) fn follow_translations(
        &mut self,
        state: &sjk_identity::ChatState,
        now: Instant,
    ) -> Vec<String> {
        let mut console = Vec::new();
        let mut waiting = std::mem::take(&mut self.translations.0);
        waiting.retain(|(serial, key)| {
            let Some(outcome) = state
                .translations
                .iter()
                .find(|outcome| outcome.serial == *serial)
            else {
                return true;
            };
            let text = match &outcome.result {
                Ok(translation) => shown(translation),
                Err(error) => format!("^3Translate: {error}"),
            };
            console.push(text.clone());
            self.put_translation(text, key.as_ref(), now);
            false
        });
        self.translations.0 = waiting;
        console
    }

    /// Say `text` in the feed as a local line, never sent: a translation's notice.
    pub(crate) fn translation_notice(&mut self, text: &str, now: Instant) {
        self.put_translation(format!("^3Translate: {text}"), None, now);
    }

    /// Put `text` in the feed under the line of `key`, or at the end.
    fn put_translation(&mut self, text: String, key: Option<&LineKey>, now: Instant) {
        let at = key.and_then(|(ms, body)| {
            self.lines
                .iter()
                .rposition(|line| line.received_ms == *ms && line.body == *body)
        });
        let original = at.and_then(|at| self.lines.get(at));
        let line = ChatLine {
            name: String::new(),
            body: text,
            sender: None,
            channel: original.map_or(Channel::Global, |line| line.channel),
            server_private: false,
            received_ms: self.millis(now),
            muted: false,
            emojis: Vec::new(),
            wrap: layout::Wrapped::default(),
            y: None,
            hub: None,
            gif: None,
            encrypted: false,
            verified: false,
            recipient: None,
            loud: original.is_none_or(|line| line.loud),
            translated: true,
        };
        match at {
            Some(at) if at + 1 < self.lines.len() => {
                if self.lines.len() == super::HISTORY_LIMIT {
                    self.lines.pop_front();
                    self.lines.insert(at, line);
                } else {
                    self.lines.insert(at + 1, line);
                }
            }
            _ => self.push_line(line, false),
        }
    }
}

impl crate::GpuState {
    /// Translate `text`, or the newest foreign line in the feed when `None` (`/tr` and
    /// `tr`): the hub is asked, and the translation shows once it answers. Why nothing
    /// was asked is the error.
    pub(crate) fn translate_chat(&mut self, text: Option<&str>) -> Result<(), String> {
        let console = self.console.as_ref();
        if console.is_some_and(|console| console.bool_cvar(CVAR) == Some(false)) {
            return Err(format!("translation is off ({CVAR} 1 turns it on)"));
        }
        let skip = Skip::parse(
            console
                .and_then(|console| console.text_cvar(SKIP_CVAR).ok())
                .unwrap_or(SKIP_DEFAULT),
        );
        let (key, text) = match text {
            Some(text) => (None, plain(text)),
            None => {
                let (key, text) = self
                    .chat
                    .last_foreign(&skip)
                    .ok_or("no foreign line in the chat to translate (/tr <text> takes one)")?;
                (Some(key), text)
            }
        };
        if sjk_identity::translate::check(&text).is_none() {
            return Err(format!(
                "nothing to translate (1 to {} characters)",
                sjk_identity::translate::TEXT_MAX
            ));
        }
        let serial = crate::player_identity::translate(text)
            .ok_or("translation needs the SJK identity (cl_identity 1)")?;
        self.chat.translation_asked(serial, key);
        Ok(())
    }

    /// Show the translations the hub answered (each frame; nothing to do while none
    /// waits), in the feed and the console.
    pub(crate) fn update_chat_translations(&mut self) {
        if !self.chat.translations_waiting() {
            return;
        }
        let now = Instant::now();
        let chat = &mut self.chat;
        let Some(lines) =
            crate::player_identity::with_chat(|state| chat.follow_translations(state, now))
        else {
            self.chat.forget_translations();
            return;
        };
        if let Some(console) = self.console.as_mut() {
            for line in lines {
                console.push_log(line);
            }
        }
    }
}

/// The ISO 639-1 code of a language whatlang knows.
fn iso639_1(lang: whatlang::Lang) -> &'static str {
    use whatlang::Lang;
    match lang {
        Lang::Epo => "eo",
        Lang::Eng => "en",
        Lang::Rus => "ru",
        Lang::Cmn => "zh",
        Lang::Spa => "es",
        Lang::Por => "pt",
        Lang::Ita => "it",
        Lang::Ben => "bn",
        Lang::Fra => "fr",
        Lang::Deu => "de",
        Lang::Ukr => "uk",
        Lang::Kat => "ka",
        Lang::Ara => "ar",
        Lang::Hin => "hi",
        Lang::Jpn => "ja",
        Lang::Heb => "he",
        Lang::Yid => "yi",
        Lang::Pol => "pl",
        Lang::Amh => "am",
        Lang::Jav => "jv",
        Lang::Kor => "ko",
        Lang::Nob => "nb",
        Lang::Dan => "da",
        Lang::Swe => "sv",
        Lang::Fin => "fi",
        Lang::Tur => "tr",
        Lang::Nld => "nl",
        Lang::Hun => "hu",
        Lang::Ces => "cs",
        Lang::Ell => "el",
        Lang::Bul => "bg",
        Lang::Bel => "be",
        Lang::Mar => "mr",
        Lang::Kan => "kn",
        Lang::Ron => "ro",
        Lang::Slv => "sl",
        Lang::Hrv => "hr",
        Lang::Srp => "sr",
        Lang::Mkd => "mk",
        Lang::Lit => "lt",
        Lang::Lav => "lv",
        Lang::Est => "et",
        Lang::Tam => "ta",
        Lang::Vie => "vi",
        Lang::Urd => "ur",
        Lang::Tha => "th",
        Lang::Guj => "gu",
        Lang::Uzb => "uz",
        Lang::Pan => "pa",
        Lang::Aze => "az",
        Lang::Ind => "id",
        Lang::Tel => "te",
        Lang::Pes => "fa",
        Lang::Mal => "ml",
        Lang::Ori => "or",
        Lang::Mya => "my",
        Lang::Nep => "ne",
        Lang::Sin => "si",
        Lang::Khm => "km",
        Lang::Tuk => "tk",
        Lang::Aka => "ak",
        Lang::Zul => "zu",
        Lang::Sna => "sn",
        Lang::Afr => "af",
        Lang::Lat => "la",
        Lang::Slk => "sk",
        Lang::Cat => "ca",
        Lang::Tgl => "tl",
        Lang::Hye => "hy",
        Lang::Cym => "cy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_client::ServerEventKind;

    #[test]
    fn the_skip_list_takes_spaces_and_commas() {
        let skip = Skip::parse(" SK, cs;de  x toolong 12 ");
        assert_eq!(skip, Skip(vec!["sk".into(), "cs".into(), "de".into()]));
        assert!(skip.skips("sk") && skip.skips("de") && skip.skips("en"));
        assert!(!skip.skips("fr"));
        assert!(Skip::parse("").skips("en"), "English is never offered");
        assert!(Skip::parse("no").skips("nb"));
    }

    #[test]
    fn the_guess_finds_french_german_and_spanish_and_leaves_english() {
        let skip = Skip::parse(SKIP_DEFAULT);
        let lines = [
            (
                "Salut, est-ce que tu viens au duel ce soir avec nous ?",
                "fr",
            ),
            ("Hallo, kommst du heute Abend noch zum Duell mit uns?", "de"),
            ("Hola, vienes al duelo esta noche con nosotros?", "es"),
        ];
        for (line, code) in lines {
            assert_eq!(foreign(line, &skip), Some(code), "{line}");
        }
        assert_eq!(
            foreign("Hey, are you coming to the duel tonight with us?", &skip),
            None
        );
        assert_eq!(
            foreign(lines[0].0, &Skip::parse("en fr")),
            None,
            "a skipped language is not foreign"
        );
    }

    #[test]
    fn short_lines_are_not_taken_for_foreign() {
        let skip = Skip::parse(SKIP_DEFAULT);
        for line in ["gg", "lol wp", "salut", "^1hola ^7amigo", "ok :)"] {
            assert_eq!(foreign(&plain(line), &skip), None, "{line}");
        }
        // But they may still be translated on request, as the page offers it.
        assert!(offered("salut", &skip));
    }

    #[test]
    fn plain_text_has_no_colour_codes_nor_marks() {
        assert_eq!(plain("^1Sa^7lut \u{E001}  toi^"), "Salut toi^");
    }

    #[test]
    fn slash_tr_is_read_from_the_draft() {
        assert_eq!(typed("/tr"), Some(None));
        assert_eq!(typed("  /TR   "), Some(None));
        assert_eq!(typed("/tr  hola amigo "), Some(Some("hola amigo")));
        assert_eq!(typed("/trade"), None);
        assert_eq!(typed("tr hola"), None);
        assert_eq!(typed("hello /tr"), None);
    }

    fn feed() -> ChatOverlay {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        for line in [
            "Salut, est-ce que tu viens au duel ce soir avec nous ?",
            "gg",
            "ok see you there in a minute",
        ] {
            chat.receive(ServerEventKind::Chat, line.to_owned(), None, now);
        }
        chat
    }

    #[test]
    fn slash_tr_picks_the_newest_foreign_line_and_its_translation_goes_under_it() {
        let mut chat = feed();
        let (key, text) = chat.last_foreign(&Skip::parse("en")).expect("foreign");
        assert!(text.starts_with("Salut"));
        chat.translation_asked(7, Some(key));
        let mut state = sjk_identity::ChatState::default();
        let console = chat.follow_translations(&state, Instant::now());
        assert!(console.is_empty() && chat.translations_waiting());
        state
            .translations
            .push_back(sjk_identity::TranslateOutcome {
                serial: 7,
                text,
                result: Ok(sjk_identity::Translation {
                    text: "Hi, are you coming to the duel tonight?".into(),
                    from: "fr".into(),
                    cached: false,
                    left: Some(49),
                }),
            });
        let console = chat.follow_translations(&state, Instant::now());
        assert_eq!(
            console,
            ["^9\u{bb} (fr) Hi, are you coming to the duel tonight?"]
        );
        assert!(!chat.translations_waiting());
        let bodies: Vec<&str> = chat.lines.iter().map(|line| line.body.as_str()).collect();
        assert_eq!(bodies[1], console[0], "under its line: {bodies:?}");
        assert!(chat.lines[1].translated);
        // The translation is not foreign itself, and the French line is taken again.
        let (_, again) = chat.last_foreign(&Skip::parse("en")).unwrap();
        assert!(again.starts_with("Salut"));
    }

    #[test]
    fn why_there_is_no_translation_is_a_notice_at_the_end() {
        let mut chat = feed();
        chat.translation_asked(3, None);
        let mut state = sjk_identity::ChatState::default();
        state
            .translations
            .push_back(sjk_identity::TranslateOutcome {
                serial: 3,
                text: "hola".into(),
                result: Err(sjk_identity::TranslateError::Off),
            });
        let console = chat.follow_translations(&state, Instant::now());
        assert_eq!(console, ["^3Translate: This SJK hub does not translate"]);
        assert_eq!(chat.lines.back().unwrap().body, console[0]);
    }
}
