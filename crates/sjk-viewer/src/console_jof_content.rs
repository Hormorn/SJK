//! Console ownership and settings for the content cards: JoF content and the
//! Sith cloud, one card each with its own settings, and their automatic
//! updates ([`ViewerConsole::content_auto`]).
use super::*;
use crate::jof_content::card::{AUTO_FIRST, Action, Card, Names};
use crate::unlock_toast::Unlock;
use crate::{jof_content, sith_cloud};
use sjk_jof_content::{AutoGroups, Selection};

/// Which content card.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cloud {
    Jof,
    Sith,
}

const CLOUDS: [Cloud; 2] = [Cloud::Jof, Cloud::Sith];

impl Cloud {
    /// The collection's settings.
    fn names(self) -> &'static Names {
        match self {
            Self::Jof => &jof_content::NAMES,
            Self::Sith => &sith_cloud::NAMES,
        }
    }
}

impl ViewerConsole {
    pub(super) fn content_card(&mut self, cloud: Cloud) -> &mut Card {
        match cloud {
            Cloud::Jof => &mut self.jof_content,
            Cloud::Sith => &mut self.sith_cloud,
        }
    }

    /// The open content card, if any.
    pub(super) fn open_cloud(&self) -> Option<Cloud> {
        CLOUDS.into_iter().find(|&cloud| match cloud {
            Cloud::Jof => self.jof_content.open,
            Cloud::Sith => self.sith_cloud.open,
        })
    }

    fn content_settings(
        &self,
        cloud: Cloud,
        game_data: &std::path::Path,
    ) -> sjk_jof_content::job::Settings {
        match cloud {
            Cloud::Jof => jof_content::job_settings(self, game_data),
            Cloud::Sith => sith_cloud::job_settings(self, game_data),
        }
    }

    /// Open the card over whatever is on screen; a first look starts a refresh.
    pub(crate) fn open_jof_content(&mut self, game_data: &std::path::Path) {
        self.open_content(Cloud::Jof, game_data);
    }

    pub(crate) fn open_sith_cloud(&mut self, game_data: &std::path::Path) {
        self.open_content(Cloud::Sith, game_data);
    }

    /// The player's choice of groups and files.
    fn content_selection(&self, names: &Names) -> Selection {
        Selection::parse(self.text_value(names.groups).unwrap_or("default"))
            .with_files(self.text_value(names.files).unwrap_or(""))
    }

    fn content_auto_groups(&self, names: &Names) -> AutoGroups {
        AutoGroups::parse(self.text_value(names.auto_groups).unwrap_or(""))
    }

    fn open_content(&mut self, cloud: Cloud, game_data: &std::path::Path) {
        let owns_console = !self.open;
        let names = cloud.names();
        let on = self.bool_cvar(names.switch).unwrap_or(false);
        let auto = self.bool_cvar(names.auto).unwrap_or(false);
        let auto_groups = self.content_auto_groups(names);
        let selection = self.content_selection(names);
        let folder = match cloud {
            Cloud::Jof => jof_content::folder(self),
            Cloud::Sith => sith_cloud::folder(game_data),
        };
        let mut card =
            std::mem::replace(self.content_card(cloud), Card::new(&jof_content::card::JOF));
        card.set_auto(auto, auto_groups);
        let refresh = card.open(game_data, on, selection, &folder);
        if refresh {
            card.refresh(self.content_settings(cloud, game_data));
        }
        self.set_open(false);
        self.set_open(true);
        card.owns_console = owns_console;
        card.open = true;
        *self.content_card(cloud) = card;
    }

    fn content_action(&mut self, cloud: Cloud, action: Action) {
        let names = cloud.names();
        match action {
            Action::None => {}
            Action::Close => {
                let card = self.content_card(cloud);
                card.open = false;
                if card.owns_console {
                    self.set_open(false);
                }
            }
            Action::Switch(on) => {
                self.set_cvar(names.switch, if on { "1" } else { "0" });
                if cloud == Cloud::Jof {
                    jof_content::follow(self);
                }
            }
            Action::Auto(on) => {
                self.set_cvar(names.auto, if on { "1" } else { "0" });
                if on {
                    // Turned on now: the first check soon, not in an hour.
                    self.content_card(cloud)
                        .schedule(Instant::now(), AUTO_FIRST);
                }
            }
            Action::AutoGroups(groups) => {
                self.set_cvar(names.auto_groups, &groups);
            }
            Action::Choice { groups, files } => {
                self.set_cvar(names.groups, &groups);
                self.set_cvar(names.files, &files);
            }
            Action::Check => {
                let game_data = self.content_card(cloud).game_data().to_owned();
                let settings = self.content_settings(cloud, &game_data);
                self.content_card(cloud).refresh(settings);
            }
            Action::Download => {
                let game_data = self.content_card(cloud).game_data().to_owned();
                let settings = self.content_settings(cloud, &game_data);
                self.content_card(cloud).download(settings);
            }
        }
    }

    pub(super) fn jof_content_key(&mut self, event: &KeyEvent) -> bool {
        if self.model_sync_key(event) {
            return true;
        }
        let Some(cloud) = self.open_cloud() else {
            return false;
        };
        let action = self.content_card(cloud).key(event);
        self.content_action(cloud, action);
        true
    }

    pub(super) fn jof_content_pointer(&mut self, event: sjk_ui::InputEvent) -> bool {
        if self.model_sync_pointer(event) {
            return true;
        }
        let Some(cloud) = self.open_cloud() else {
            return false;
        };
        let action = self.content_card(cloud).pointer(event);
        self.content_action(cloud, action);
        true
    }

    /// The open content card's drawing, if one is open.
    pub(crate) fn content_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        if let Some(draw_list) = self.model_sync_draw_list() {
            return Some(draw_list);
        }
        match self.open_cloud()? {
            Cloud::Jof => Some(self.jof_content.draw_list()),
            Cloud::Sith => Some(self.sith_cloud.draw_list()),
        }
    }

    /// Each frame, for every collection: start its automatic update when due
    /// (its switch and automatic switch on, no job of its card at work), and give
    /// the card of one that just changed files. Two comparisons while none is due
    /// or running; the settings are read only when one is.
    pub(crate) fn content_auto(
        &mut self,
        now: Instant,
        game_data: &std::path::Path,
    ) -> [Option<Unlock>; 2] {
        CLOUDS.map(|cloud| self.content_auto_of(cloud, now, game_data))
    }

    fn content_auto_of(
        &mut self,
        cloud: Cloud,
        now: Instant,
        game_data: &std::path::Path,
    ) -> Option<Unlock> {
        let card = self.content_card(cloud);
        if let Some(outcome) = card.auto_finished() {
            let title = card.title();
            for failure in &outcome.failures {
                crate::log::progress(format_args!(
                    "{title}: automatic update: {failure}; tried again in an hour"
                ));
            }
            return outcome.changed().then(|| Unlock::Content {
                title,
                files: u32::try_from(outcome.downloaded).unwrap_or(u32::MAX),
                bytes: outcome.downloaded_bytes,
                removed: u32::try_from(outcome.removed).unwrap_or(u32::MAX),
            });
        }
        if !card.auto_due(now) {
            return None;
        }
        let names = cloud.names();
        let on = self.bool_cvar(names.switch).unwrap_or(false);
        let auto = self.bool_cvar(names.auto).unwrap_or(false);
        let groups = self.content_auto_groups(names);
        if !on || !auto || groups == AutoGroups::Only(Default::default()) {
            return None;
        }
        // The player's Download (or a look) is at work: the next hour's turn.
        if self.content_card(cloud).busy() {
            return None;
        }
        let selection = self.content_selection(names);
        let settings = self.content_settings(cloud, game_data);
        self.content_card(cloud)
            .start_auto(settings, selection, groups);
        None
    }

    /// Close both content cards, and the Model sync page.
    pub(crate) fn close_content(&mut self) {
        self.jof_content.open = false;
        self.sith_cloud.open = false;
        self.model_sync_page.close();
    }
}
