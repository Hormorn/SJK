//! The shaders and models the map editor used lately, newest first, for the Recent rows
//! of the browser editor's lists and of the in-game panel's (Sol, 03/11/2026: "remember
//! in general what textures we recently used"). Kept in a small file of the map edits'
//! folder (`map_edits/web_recent.json` in the SJK config folder, its name from when only
//! the page had them), not in the page: the page's origin changes with the server's
//! random port, so its own storage would forget.
//!
//! A remap, a shown face or a fill adds its shader, a placed model its model, whether
//! the in-game editor or the page made it. "Remember recent textures" off empties both
//! lists and keeps them empty until it is on again.

use super::document::Kind;
use crate::GpuState;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The file's name in the map edits' folder.
pub(crate) const FILE: &str = "web_recent.json";
/// Names each list keeps.
pub(crate) const KEEP: usize = 24;
/// The longest name kept (a document keeps 160 at most).
const MAX_NAME: usize = 160;

/// The lists and the switch.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Recent {
    /// "Remember recent textures": on by default.
    #[serde(default = "on")]
    pub(crate) remember: bool,
    #[serde(default)]
    pub(crate) shaders: Vec<String>,
    #[serde(default)]
    pub(crate) models: Vec<String>,
}

fn on() -> bool {
    true
}

impl Default for Recent {
    fn default() -> Self {
        Self {
            remember: true,
            shaders: Vec::new(),
            models: Vec::new(),
        }
    }
}

/// Put `name` first in `list`, once (whatever its case), keeping [`KEEP`]. Whether the
/// list changed.
fn push(list: &mut Vec<String>, name: &str) -> bool {
    let name = name.trim().replace('\\', "/");
    if name.is_empty() || name.len() > MAX_NAME || name.chars().any(char::is_control) {
        return false;
    }
    if list
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case(&name))
    {
        return false;
    }
    list.retain(|known| !known.eq_ignore_ascii_case(&name));
    list.insert(0, name);
    list.truncate(KEEP);
    true
}

impl Recent {
    /// The lists in `directory`'s file; the defaults when it is missing or unreadable.
    pub(crate) fn load(directory: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(directory.join(FILE)) else {
            return Self::default();
        };
        let mut recent: Self = serde_json::from_str(&text).unwrap_or_default();
        // A hand-edited file is held to the same limits.
        for list in [&mut recent.shaders, &mut recent.models] {
            let names = std::mem::take(list);
            for name in names.iter().rev() {
                push(list, name);
            }
        }
        if !recent.remember {
            recent.shaders.clear();
            recent.models.clear();
        }
        recent
    }

    /// Write the lists to `directory`'s file.
    pub(crate) fn save(&self, directory: &Path) -> Result<(), String> {
        std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let path = directory.join(FILE);
        let text = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, text)
            .and_then(|()| std::fs::rename(&temporary, &path))
            .map_err(|error| format!("{}: {error}", path.display()))
    }

    /// A shader a remap or a shown face drew; whether the list changed.
    pub(crate) fn used_shader(&mut self, shader: &str) -> bool {
        self.remember && push(&mut self.shaders, shader)
    }

    /// A model placed; whether the list changed.
    pub(crate) fn used_model(&mut self, model: &str) -> bool {
        self.remember && push(&mut self.models, model)
    }

    /// Turn remembering on or off; off empties the lists.
    pub(crate) fn set_remember(&mut self, remember: bool) {
        self.remember = remember;
        if !remember {
            self.clear();
        }
    }

    /// Forget the lists (the switch stays).
    pub(crate) fn clear(&mut self) {
        self.shaders.clear();
        self.models.clear();
    }
}

impl GpuState {
    /// The Recent lists, read from their file the first time.
    pub(crate) fn map_edit_recent(&mut self) -> &mut Recent {
        if self.map_edits.recent.is_none() {
            let loaded = self
                .map_edits_folder()
                .map(|folder| Recent::load(&folder))
                .unwrap_or_default();
            self.map_edits.recent = Some(loaded);
        }
        self.map_edits.recent.get_or_insert_with(Default::default)
    }

    fn map_edit_recent_save(&mut self) {
        let (Some(folder), Some(recent)) = (self.map_edits_folder(), &self.map_edits.recent) else {
            return;
        };
        if let Err(error) = recent.save(&folder) {
            crate::log::progress(format_args!("map edits: recent list not saved: {error}"));
        }
        // The panel's lists show them.
        self.map_edits.panel.stale();
    }

    /// The Recent lists as the page reads them (`api/recent`).
    pub(crate) fn map_edit_recent_json(&mut self) -> serde_json::Value {
        serde_json::to_value(&*self.map_edit_recent()).unwrap_or(serde_json::Value::Null)
    }

    /// "Remember recent textures" on or off (`Some`), or forget the lists (`None`).
    pub(crate) fn map_edit_set_recent(&mut self, remember: Option<bool>) {
        let recent = self.map_edit_recent();
        match remember {
            Some(on) => recent.set_remember(on),
            None => recent.clear(),
        }
        self.map_edit_recent_save();
    }

    /// Shader `shader` was drawn (a remap, a shown face, a fill): first in the list.
    pub(crate) fn map_edit_remember_shader(&mut self, shader: &str) {
        if self.map_edit_recent().used_shader(shader) {
            self.map_edit_recent_save();
        }
    }

    /// After editor command `name` (with `args`) did its work: a remap's or a shown
    /// face's shader and a placed model go first in the Recent rows.
    pub(super) fn map_edit_remember(&mut self, name: &str, args: &[String]) {
        let used = match (name, args.first()) {
            ("mapedit_remap" | "mapedit_sky", Some(shader)) if shader != "-" => {
                Some((true, shader.clone()))
            }
            ("mapedit_place", _) => {
                let current = self.map_edits.editor.current();
                self.map_edits
                    .document
                    .as_ref()
                    .and_then(|document| document.edits.iter().find(|e| Some(e.id) == current))
                    .and_then(|edit| match &edit.kind {
                        Kind::Place { model, .. } => Some((false, model.clone())),
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some((shader, used)) = used else {
            return;
        };
        let recent = self.map_edit_recent();
        let changed = if shader {
            recent.used_shader(&used)
        } else {
            recent.used_model(&used)
        };
        if changed {
            self.map_edit_recent_save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_comes_first_once_and_the_list_is_kept_short() {
        let mut recent = Recent::default();
        assert!(recent.used_shader("textures/a/one"));
        assert!(recent.used_shader("textures/a/two"));
        assert!(!recent.used_shader("textures/a/two"), "already first");
        assert!(recent.used_shader("TEXTURES/A/ONE"));
        assert_eq!(recent.shaders, ["TEXTURES/A/ONE", "textures/a/two"]);
        for n in 0..40 {
            recent.used_model(&format!("models/m{n}.md3"));
        }
        assert_eq!(recent.models.len(), KEEP);
        assert_eq!(recent.models[0], "models/m39.md3");
        assert!(!recent.used_shader(""));
        assert!(!recent.used_shader(&"x".repeat(MAX_NAME + 1)));
        assert!(recent.used_model("models\\back\\slash.md3"));
        assert_eq!(recent.models[0], "models/back/slash.md3");
    }

    #[test]
    fn off_forgets_and_keeps_nothing_until_on_again() {
        let mut recent = Recent::default();
        recent.used_shader("textures/a/one");
        recent.set_remember(false);
        assert!(recent.shaders.is_empty());
        assert!(!recent.used_shader("textures/a/two"));
        assert!(!recent.used_model("models/a.md3"));
        assert!(recent.shaders.is_empty() && recent.models.is_empty());
        recent.set_remember(true);
        assert!(recent.used_model("models/a.md3"));
        recent.clear();
        assert!(recent.remember && recent.models.is_empty());
    }

    #[test]
    fn the_file_survives_a_restart_and_a_bad_one_is_the_default() {
        let directory = std::env::temp_dir().join(format!(
            "sjk-mapedit-recent-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(Recent::load(&directory), Recent::default());
        let mut recent = Recent::default();
        recent.used_shader("textures/a/one");
        recent.used_model("models/a.md3");
        recent.save(&directory).unwrap();
        assert_eq!(Recent::load(&directory), recent);
        // Off is kept, with nothing listed.
        recent.set_remember(false);
        recent.save(&directory).unwrap();
        let loaded = Recent::load(&directory);
        assert!(!loaded.remember && loaded.shaders.is_empty());
        // A hand-edited file: duplicates and too many names are trimmed; a broken one
        // reads as the default.
        let many: Vec<String> = (0..30).map(|n| format!("textures/t{n}")).collect();
        let shaders = [many, vec!["textures/t0".to_owned()]].concat();
        let text = serde_json::json!({ "shaders": shaders });
        std::fs::write(directory.join(FILE), text.to_string()).unwrap();
        let loaded = Recent::load(&directory);
        assert!(loaded.remember);
        assert_eq!(loaded.shaders.len(), KEEP);
        assert_eq!(loaded.shaders[0], "textures/t0");
        std::fs::write(directory.join(FILE), "{not json").unwrap();
        assert_eq!(Recent::load(&directory), Recent::default());
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
