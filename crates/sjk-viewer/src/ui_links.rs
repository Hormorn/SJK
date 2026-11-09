//! Where an announcement's links lead (`docs/sjk-ui.md`, "News panel"; Sol's request,
//! 31/10/2026: "links into the UI ... when a player clicks on it, it brings them
//! directly to a place in the UI"). Only the SJK team writes announcements, so only they
//! make links ([`sjk_identity::news_markup`]); a link names a place, never a command:
//!
//! - `setting:<cvar>` opens Settings searched for that setting (as a chip does);
//! - `open:<page>` opens one of the places in [`PAGES`], each a command of this client
//!   that opens a window. A page this build does not know draws as plain text, so an
//!   older client shows the label and does nothing.
//!
//! A new place is a new row of [`PAGES`]: its id (lowercase letters, digits, `-`, `_`),
//! the name the announce window's link picker shows, and the command it runs.

use sjk_identity::news_markup::Target;

/// One place a link can open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Page {
    /// `open:<id>`.
    pub(crate) id: &'static str,
    /// What the link picker calls it, and the label it suggests.
    pub(crate) name: &'static str,
    /// The console command that opens it; empty for Settings (opened on its own).
    pub(crate) command: &'static str,
}

/// The places links open, in the order the link picker lists them.
pub(crate) const PAGES: &[Page] = &[
    Page {
        id: "settings",
        name: "Settings",
        command: "",
    },
    Page {
        id: "collection",
        name: "Collection",
        command: "collection",
    },
    Page {
        id: "achievements",
        name: "Achievements",
        command: "achievements",
    },
    Page {
        id: "holocrons",
        name: "Holocrons",
        command: "holocrons",
    },
    Page {
        id: "profile",
        name: "Profile",
        command: "profile",
    },
    Page {
        id: "identity",
        name: "SJK identity",
        command: "identity",
    },
    Page {
        id: "chat",
        name: "SJK chat",
        command: "sjkchat",
    },
    Page {
        id: "emotes",
        name: "Emotes",
        command: "emotes",
    },
    Page {
        id: "toys",
        name: "Toys",
        command: "toy_help",
    },
    Page {
        id: "models",
        name: "Model viewer",
        command: "modelviewer",
    },
    Page {
        id: "rewards",
        name: "Contributor rewards",
        command: "rewards",
    },
    Page {
        id: "site",
        name: "My SJK page (video bounties)",
        command: "identity site",
    },
    Page {
        id: "changelog",
        name: "Changelog",
        command: "changelog",
    },
    Page {
        id: "credits",
        name: "Credits",
        command: "credits",
    },
    Page {
        id: "update",
        name: "Update",
        command: "update",
    },
    #[cfg(feature = "mod-jof")]
    Page {
        id: "jof",
        name: "JoF content",
        command: "jof.downloads",
    },
    #[cfg(feature = "mod-jof")]
    Page {
        id: "sith",
        name: "Sith cloud",
        command: "sith.downloads",
    },
];

/// What following a link does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Follow {
    /// Open Settings searched for this (empty: Settings as it opens).
    Settings(String),
    /// Run this command of the client's.
    Command(&'static str),
}

/// What following `target` does, or `None` for a place this build does not know (a
/// setting Settings has no row for, a page not in [`PAGES`]).
pub(crate) fn follow(target: &str) -> Option<Follow> {
    match Target::parse(target)? {
        Target::Setting(cvar) => {
            crate::settings::has_row(cvar).then(|| Follow::Settings(cvar.to_owned()))
        }
        Target::Open(id) => {
            let page = PAGES.iter().find(|page| page.id == id)?;
            Some(if page.command.is_empty() {
                Follow::Settings(String::new())
            } else {
                Follow::Command(page.command)
            })
        }
    }
}

/// Whether `target` leads somewhere in this build: the link draws as a link.
pub(crate) fn known(target: &str) -> bool {
    follow(target).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_a_well_formed_unique_id() {
        for (index, page) in PAGES.iter().enumerate() {
            let target = format!("open:{}", page.id);
            assert_eq!(
                Target::parse(&target),
                Some(Target::Open(page.id)),
                "{}",
                page.id
            );
            assert!(
                PAGES[..index].iter().all(|other| other.id != page.id),
                "{} twice",
                page.id
            );
        }
    }

    /// Every page's command is one the console knows, so a link never types something
    /// that does nothing.
    #[test]
    fn every_page_runs_a_command_the_console_has() {
        let directory = tempfile::tempdir().unwrap();
        let console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        for page in PAGES.iter().filter(|page| !page.command.is_empty()) {
            assert!(
                console.has_command(page.command.split_whitespace().next().unwrap_or("")),
                "{} runs {}, which the console does not have",
                page.id,
                page.command
            );
        }
    }

    #[test]
    fn links_lead_to_settings_or_a_page_and_nowhere_else() {
        assert_eq!(
            follow("setting:cg_nameplate"),
            Some(Follow::Settings("cg_nameplate".to_owned()))
        );
        assert_eq!(
            follow("open:settings"),
            Some(Follow::Settings(String::new()))
        );
        assert_eq!(
            follow("open:collection"),
            Some(Follow::Command("collection"))
        );
        assert_eq!(follow("open:nowhere"), None);
        assert_eq!(follow("setting:not_a_setting"), None);
        assert_eq!(follow("quit"), None);
    }
}
