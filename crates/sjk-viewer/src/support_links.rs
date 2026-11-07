//! Support SJK on the SJK UI's main page (`docs/sjk-ui.md`, "Support SJK"): a small
//! line top left with a link to each of SJK's donation pages, opened in the browser,
//! and × to hide it for good (`ui_supportLinks 0`; Settings > Interface shows it
//! again). It asks no server anything and promises nothing for a donation.
//!
//! Shelved since 07/11/2026 ([`SHELVED`], Sol's request): the line never shows and
//! its setting leaves Settings; `docs/sjk.md` "Donations" says how to bring it back.

use crate::console::ViewerConsole;

/// The line is shelved: hidden for everyone, whatever `ui_supportLinks` says.
pub(crate) const SHELVED: bool = true;

/// The setting that shows the line (on by default).
pub(crate) const CVAR: &str = "ui_supportLinks";

/// A donation page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Link {
    GitHubSponsors,
    KoFi,
}

impl Link {
    /// In the line's order.
    pub(crate) const ALL: [Self; 2] = [Self::GitHubSponsors, Self::KoFi];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::GitHubSponsors => "GitHub Sponsors",
            Self::KoFi => "Ko-fi",
        }
    }

    pub(crate) fn url(self) -> &'static str {
        match self {
            Self::GitHubSponsors => "https://github.com/sponsors/Sol-Vulpes",
            Self::KoFi => "https://ko-fi.com/soljk",
        }
    }
}

/// Whether the player shows the line.
pub(crate) fn shown(console: Option<&ViewerConsole>) -> bool {
    !SHELVED && console.is_none_or(|console| console.bool_cvar(CVAR) != Some(false))
}

/// Open `link`'s page in the player's browser.
pub(crate) fn open(link: Link) {
    crate::log::progress(format_args!("support: opening {}", link.url()));
    if !cfg!(test) {
        crate::update::open_page(link.url());
    }
}

/// Hide the line for good (its ×).
pub(crate) fn hide(console: &mut ViewerConsole) {
    console.set_cvar(CVAR, "0");
}
