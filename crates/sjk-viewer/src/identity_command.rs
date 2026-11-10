//! The `identity` console command: `identity` opens the Identity page, and its
//! words set the player's bio, list who the hub knows on this server, open the player's
//! SJK page in the browser and join this key to an account there (`docs/identity.md`).
//! There is no name to set: the hub takes the name the player plays under.

use crate::player_identity;
use sjk_identity::{Snapshot, Status};

/// Console command name.
pub(crate) const COMMAND: &str = "identity";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Your SJK identity: open its page; bio, key, adminkey, who, site and link subcommands";

const USAGE: &str =
    "identity [bio <text> | key | adminkey | who [slot] | site | link <code> [confirm]]";
/// The answer to `identity name`, which no longer exists.
const NAME_FROM_GAME: &str =
    "Your SJK name is the name you play under: change it with /name <text>.";

/// What the player asked for.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    /// No words: open or close the page.
    Toggle,
    /// Copy the key id to the clipboard (key ids never show in the game) and say where
    /// the key file is.
    Key,
    /// Copy the public key to the clipboard, never on screen, for a server owner's
    /// `admin_keys.txt` (`docs/identity.md`, "Admin key login").
    AdminKey,
    /// List the players the hub knows on this server, or one of them.
    Who(Option<u8>),
    /// Set the bio at the hub.
    Bio(String),
    /// Open the player's SJK page, signed in, in the browser.
    Site,
    /// Type a link code the SJK page made: first only whose account it joins; with
    /// `confirm`, this key joins it.
    Link { code: String, confirm: bool },
}

/// Read the command's words.
pub(crate) fn parse(args: &[String]) -> Result<Action, String> {
    let Some(word) = args.first() else {
        return Ok(Action::Toggle);
    };
    let text = || args[1..].join(" ");
    match word.to_ascii_lowercase().as_str() {
        "key" => Ok(Action::Key),
        "adminkey" => Ok(Action::AdminKey),
        "who" => match args.get(1) {
            None => Ok(Action::Who(None)),
            Some(slot) => slot
                .parse()
                .map(|slot| Action::Who(Some(slot)))
                .map_err(|_| format!("who takes a client number, not \"{slot}\"")),
        },
        "bio" => Ok(Action::Bio(text())),
        "site" | "page" => Ok(Action::Site),
        "link" => {
            let usage = || {
                "usage: identity link <code> [confirm], with the code your SJK page made".to_owned()
            };
            let code = args
                .get(1)
                .and_then(|code| sjk_identity::site::code(code))
                .ok_or_else(usage)?;
            let confirm = match args.get(2).map(|word| word.to_ascii_lowercase()) {
                None => false,
                Some(word) if word == "confirm" => true,
                Some(_) => return Err(usage()),
            };
            Ok(Action::Link { code, confirm })
        }
        "name" => Err(NAME_FROM_GAME.to_owned()),
        _ => Err(format!("usage: {USAGE}")),
    }
}

/// The lines `identity key` prints, `copied` when the key id went to the clipboard. The
/// key id itself never prints: key ids never show in the game, so a stream never shows
/// one.
pub(crate) fn key_lines(
    snapshot: Option<&Snapshot>,
    file: &std::path::Path,
    copied: bool,
) -> Vec<String> {
    match snapshot {
        Some(_) => vec![
            if copied {
                "Key id copied to the clipboard (it is never shown in the game).".to_owned()
            } else {
                "Key id hidden (it is never shown in the game); copying it failed.".to_owned()
            },
            format!(
                "The private key is in {}. Back it up: losing it loses this identity, and nobody else may have it.",
                file.display()
            ),
        ],
        None => vec!["No identity key yet: turn cl_identity on.".to_owned()],
    }
}

/// The lines `identity adminkey` prints, `copied` when the public key went to the
/// clipboard (`None` without an identity key); the key itself never prints.
pub(crate) fn admin_key_lines(copied: Option<bool>) -> Vec<String> {
    match copied {
        Some(true) => vec![
            "Admin key copied to the clipboard (it is never shown in the game).".to_owned(),
            "A server owner puts it in admin_keys.txt after a rank: council <key> <your name>."
                .to_owned(),
        ],
        Some(false) => {
            vec!["Admin key hidden (it is never shown in the game); copying it failed.".to_owned()]
        }
        None => vec!["No identity key yet: turn cl_identity on.".to_owned()],
    }
}

/// The lines `identity who` prints.
pub(crate) fn who_lines(snapshot: &Snapshot, slot: Option<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    for player in &snapshot.players {
        if slot.is_some_and(|wanted| wanted != player.slot) {
            continue;
        }
        let name = if player.name.is_empty() {
            "(no name)"
        } else {
            &player.name
        };
        // No key id: key ids never show in the game.
        lines.push(format!(
            "{:>2}  {}{}",
            player.slot,
            name,
            if player.verified { "  VERIFIED" } else { "" }
        ));
        if slot.is_some() {
            match snapshot.profiles.get(&player.key_id) {
                Some(profile) if !profile.bio.is_empty() => {
                    let bio = sjk_identity::bio::for_display(&profile.bio);
                    lines.extend(bio.lines().map(|line| format!("    {line}")));
                }
                Some(_) => lines.push("    (no bio)".to_owned()),
                None => {
                    player_identity::look_up(&player.key_id);
                    lines.push("    fetching the bio; run this again in a moment".to_owned());
                }
            }
        }
    }
    if lines.is_empty() {
        lines.push(match slot {
            Some(slot) => format!("The hub knows nobody in slot {slot} here."),
            None => "The hub knows no SJK players on this server.".to_owned(),
        });
    }
    lines
}

/// Why a profile change cannot be sent now, if it cannot.
pub(crate) fn profile_blocker(snapshot: Option<&Snapshot>) -> Option<&'static str> {
    match snapshot.map(|snapshot| &snapshot.status) {
        None | Some(Status::Disabled) => Some("Identity is off: turn cl_identity on."),
        Some(Status::NoHub) => Some("There is no hub: set cl_hubUrl."),
        Some(Status::Online) => None,
        Some(_) => Some("Not connected to the hub yet; try again in a moment."),
    }
}

impl crate::GpuState {
    /// `identity <words>`: the subcommands that are not "open the page".
    pub(crate) fn identity_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let action = parse(args)?;
        let snapshot = player_identity::snapshot();
        match action {
            Action::Toggle => Ok(Vec::new()),
            Action::Key => {
                let file = self
                    .console
                    .as_ref()
                    .map(|console| console.config_directory().join("identity.key"))
                    .unwrap_or_default();
                let copied = snapshot.as_ref().is_some_and(|snapshot| {
                    crate::console::clipboard::copy(&snapshot.local_key_id)
                });
                Ok(key_lines(snapshot.as_ref(), &file, copied))
            }
            Action::AdminKey => Ok(admin_key_lines(player_identity::with_service(|service| {
                crate::console::clipboard::copy(&service.admin_key_text())
            }))),
            Action::Who(slot) => match &snapshot {
                Some(snapshot) => Ok(who_lines(snapshot, slot)),
                None => Err("Identity is off: turn cl_identity on.".to_owned()),
            },
            Action::Site => {
                if let Some(reason) = profile_blocker(snapshot.as_ref()) {
                    return Err(reason.to_owned());
                }
                player_identity::with_service(sjk_identity::Service::site_sign_in);
                Ok(vec!["Asking the hub for your sign-in link...".to_owned()])
            }
            Action::Link { code, confirm } => {
                if let Some(reason) = profile_blocker(snapshot.as_ref()) {
                    return Err(reason.to_owned());
                }
                player_identity::with_service(|service| service.site_link(code, confirm));
                Ok(vec![if confirm {
                    "Joining this key to that account...".to_owned()
                } else {
                    "Asking the hub whose account this code is...".to_owned()
                }])
            }
            Action::Bio(text) => {
                if let Some(reason) = profile_blocker(snapshot.as_ref()) {
                    return Err(reason.to_owned());
                }
                player_identity::set_bio(text);
                Ok(vec![
                    "Sent to the hub; the Identity page shows the result.".to_owned(),
                ])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{Presence, Profile};
    use std::collections::HashMap;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            loot: Vec::new(),
            min_client: None,
            opening: None,
            reveal: None,
            local_key_id: "0123456789abcdef".to_owned(),
            status: Status::Online,
            key_id: "0123456789abcdef".to_owned(),
            me: Some(Profile {
                tag_cards: Vec::new(),
                gif_tags: None,
                credits: 0,
                cards: Vec::new(),
                keys: Vec::new(),
                key_id: "0123456789abcdef".to_owned(),
                key: String::new(),
                name: "Sol".to_owned(),
                banner: "".into(),
                bio: "hi".to_owned(),
                verified: false,
                tick: sjk_identity::Tick::None,
                staff: false,
                creator: false,
                jkr_creator: false,
                trust: None,
                emoji: String::new(),
                created: 0,
                names: Vec::new(),
                medals: Vec::new(),
                achievements: Vec::new(),
                avatar: String::new(),
                unlocks: Vec::new(),
                holocron_counts: Default::default(),
                holocrons: Vec::new(),
                shards: 0,
            }),
            server: None,
            map_blocks: Vec::new(),
            remap_blocks: None,
            remap_blocks_revision: 0,
            players: vec![Presence {
                slot: 3,
                claimed_name: "^1Fox".to_owned(),
                key_id: "fedcba9876543210".to_owned(),
                name: "Fox".to_owned(),
                verified: true,
                tick: sjk_identity::Tick::None,
                trust: None,
                emoji: String::new(),
                medals: Vec::new(),
                avatar: String::new(),
                look: None,
                whisper_key: None,
                client: String::new(),
            }],
            profiles: HashMap::new(),
            model_checks: HashMap::new(),
            profiles_revision: 0,
            me_revision: 0,
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            avatar: None,
            look_outcome: None,
            packs_revision: 0,
            map_edits_revision: 0,
            map_edits_note: None,
            map_edits_published: None,
            mystery_boxes: None,
            mystery_boxes_revision: 0,
            mystery_box: None,
            junk: None,
            junk_sale: None,
            wheel_of_doom: None,
            wheel_of_doom_spin: None,
            box_hunt: None,
            quests: None,
            secrets: None,
            secrets_revision: 0,
            secret_shows: None,
            secret_shows_revision: 0,
            secret: None,
            assets_note: None,
            crashes_sent: 0,
            crash_note: None,
            holocrons: None,
            log_requests: Vec::new(),
            log_answer: None,
            economy: None,
            kyber: None,
            seasons: None,
            shop: None,
            trade: None,
            tag_trade: None,
            marks: None,
            marks_revision: 0,
            mark_refusal: None,
            wishlist_refusal: None,
            trades: None,
            history: None,
            announcements: Vec::new(),
            announcements_revision: 0,
            rewards: None,
            rewards_revision: 0,
            claim: None,
            announce: None,
            news_image: None,
            site: None,
            public_key: String::new(),
        }
    }

    #[test]
    fn words_become_actions() {
        assert_eq!(parse(&words("")), Ok(Action::Toggle));
        assert_eq!(parse(&words("key")), Ok(Action::Key));
        assert_eq!(parse(&words("ADMINKEY")), Ok(Action::AdminKey));
        assert!(
            admin_key_lines(Some(true))
                .concat()
                .contains("admin_keys.txt")
        );
        assert_eq!(parse(&words("WHO")), Ok(Action::Who(None)));
        assert_eq!(parse(&words("who 3")), Ok(Action::Who(Some(3))));
        assert_eq!(
            parse(&words("bio Sol the Fox")),
            Ok(Action::Bio("Sol the Fox".to_owned()))
        );
        assert_eq!(parse(&words("bio")), Ok(Action::Bio(String::new())));
        // The name is the in-game one: the old word explains where it went.
        assert_eq!(parse(&words("name Sol")), Err(NAME_FROM_GAME.to_owned()));
        assert!(parse(&words("who x")).is_err());
        assert!(
            parse(&words("frobnicate"))
                .unwrap_err()
                .starts_with("usage")
        );
    }

    #[test]
    fn site_and_link_codes() {
        assert_eq!(parse(&words("site")), Ok(Action::Site));
        assert_eq!(
            parse(&words("link abcd-efgh")),
            Ok(Action::Link {
                code: "ABCD-EFGH".to_owned(),
                confirm: false
            })
        );
        assert_eq!(
            parse(&words("link ABCDEFGH CONFIRM")),
            Ok(Action::Link {
                code: "ABCD-EFGH".to_owned(),
                confirm: true
            })
        );
        assert!(parse(&words("link")).is_err());
        assert!(parse(&words("link ABC")).is_err());
        assert!(parse(&words("link ABCD-EFGH yes")).is_err());
    }

    #[test]
    fn site_answers_never_print_a_link_or_a_code() {
        use sjk_identity::site::{Linked, SignInLink, SiteAnswer};
        let signed = crate::identity_frame::site_lines(&Ok(SiteAnswer::SignIn(SignInLink {
            url: "https://site.test/login/secret".to_owned(),
            expires: 0,
        })));
        assert!(!signed.concat().contains("secret"));
        let preview = crate::identity_frame::site_lines(&Ok(SiteAnswer::Link(Linked {
            account: "Sol".to_owned(),
            keys: None,
        })))
        .concat();
        assert!(preview.contains("Sol") && preview.contains("confirm"));
        let joined = crate::identity_frame::site_lines(&Ok(SiteAnswer::Link(Linked {
            account: "Sol".to_owned(),
            keys: Some(2),
        })));
        assert!(joined.concat().contains("2 keys"));
    }

    #[test]
    fn who_lists_slots_and_marks_verified_players() {
        let lines = who_lines(&snapshot(), None);
        assert_eq!(lines, [" 3  Fox  VERIFIED"], "no key id");
        assert_eq!(
            who_lines(&snapshot(), Some(9)),
            ["The hub knows nobody in slot 9 here."]
        );
        let mut empty = snapshot();
        empty.players.clear();
        assert_eq!(
            who_lines(&empty, None),
            ["The hub knows no SJK players on this server."]
        );
    }

    #[test]
    fn who_with_a_slot_shows_a_fetched_bio() {
        let mut shown = snapshot();
        shown.profiles.insert(
            "fedcba9876543210".to_owned(),
            Profile {
                tag_cards: Vec::new(),
                gif_tags: None,
                credits: 0,
                cards: Vec::new(),
                keys: Vec::new(),
                key_id: "fedcba9876543210".to_owned(),
                key: String::new(),
                name: "Fox".to_owned(),
                banner: "".into(),
                bio: "line one\nline two".to_owned(),
                verified: true,
                tick: sjk_identity::Tick::None,
                staff: false,
                creator: false,
                jkr_creator: false,
                trust: None,
                emoji: String::new(),
                created: 0,
                names: Vec::new(),
                medals: Vec::new(),
                achievements: Vec::new(),
                avatar: String::new(),
                unlocks: Vec::new(),
                holocron_counts: Default::default(),
                holocrons: Vec::new(),
                shards: 0,
            },
        );
        let lines = who_lines(&shown, Some(3));
        assert_eq!(lines[1..], ["    line one", "    line two"]);
    }

    #[test]
    fn identity_key_never_prints_the_key_id() {
        let lines = key_lines(
            Some(&snapshot()),
            std::path::Path::new("identity.key"),
            true,
        );
        assert!(lines[0].contains("copied"), "{lines:?}");
        assert!(lines.iter().all(|line| !line.contains("0123456789abcdef")));
    }

    #[test]
    fn profile_changes_wait_for_an_online_hub() {
        assert!(profile_blocker(None).is_some());
        let mut state = snapshot();
        assert_eq!(profile_blocker(Some(&state)), None);
        for status in [Status::Disabled, Status::NoHub, Status::Registering] {
            state.status = status;
            assert!(profile_blocker(Some(&state)).is_some());
        }
    }
}
