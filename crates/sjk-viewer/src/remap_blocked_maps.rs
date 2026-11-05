//! Per-map choices for server shader remaps.
//!
//! `cg_remapsBlockedMaps` lists map names (`mp/ffa4 mp/duel6`), separated by spaces,
//! commas or semicolons (a console command needs the list in quotes to hold
//! semicolons), on which server remaps are ignored, as with `cg_remaps 0` there.
//! `cg_remapsAllowedMaps` lists maps on which they apply even while
//! `cg_serverRemaps` is off (Lex's idea). A map in both lists is blocked. A listed
//! map still applies its own worldspawn remaps and local `remapShader` overrides;
//! only the server's table is affected.
//!
//! `cg_serverRemapsMap` (`default`, `on` or `off`, not archived) shows and sets the
//! loaded map's choice, for the Settings row: it is rewritten from the lists when
//! the map or a list changes, and setting it rewrites the lists. The frame reads a
//! cached answer, refreshed only after one of these changes.

/// Maps whose server remaps are ignored, archived with the player's settings.
pub(crate) const CVAR: &str = "cg_remapsBlockedMaps";
/// Maps whose server remaps apply while `cg_serverRemaps` is off, archived.
pub(crate) const ALLOWED_CVAR: &str = "cg_remapsAllowedMaps";
/// The loaded map's [`Choice`], by name; not archived.
pub(crate) const MAP_CVAR: &str = "cg_serverRemapsMap";
/// `cg_serverRemaps`: servers' shader remaps apply only while it is on (off by default
/// since 18/10/2026); `cg_remaps` then filters them as in EternalJK.
pub(crate) const SERVER_CVAR: &str = "cg_serverRemaps";

/// The map a list entry or a loaded BSP path names: `maps/mp/ffa4.bsp` and
/// `mp/ffa4` are both `mp/ffa4`.
pub(crate) fn map_name(path: &str) -> &str {
    let name = path.trim().trim_start_matches(['/', '\\']);
    let name = name
        .get(..5)
        .filter(|prefix| {
            // Bytes, not str slices: a multi-byte character may straddle byte 4.
            let prefix = prefix.as_bytes();
            prefix[..4].eq_ignore_ascii_case(b"maps") && matches!(prefix[4], b'/' | b'\\')
        })
        .map_or(name, |_| &name[5..]);
    name.len()
        .checked_sub(4)
        .filter(|&end| {
            name.get(end..)
                .is_some_and(|ext| ext.eq_ignore_ascii_case(".bsp"))
        })
        .map_or(name, |end| &name[..end])
}

fn same_map(a: &str, b: &str) -> bool {
    let fold = |byte: u8| match byte {
        b'\\' => b'/',
        byte => byte.to_ascii_lowercase(),
    };
    a.len() == b.len() && a.bytes().zip(b.bytes()).all(|(a, b)| fold(a) == fold(b))
}

fn entries(list: &str) -> impl Iterator<Item = &str> {
    list.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .map(map_name)
        .filter(|entry| !entry.is_empty())
}

/// Whether `list` names the map at `path`.
pub(crate) fn lists(list: &str, path: &str) -> bool {
    let map = map_name(path);
    !map.is_empty() && entries(list).any(|entry| same_map(entry, map))
}

/// `list` with `map` added; `None` when it is already there.
pub(crate) fn with(list: &str, map: &str) -> Option<String> {
    let map = map_name(map);
    (!lists(list, map)).then(|| entries(list).chain([map]).collect::<Vec<_>>().join(" "))
}

/// `list` without `map`; `None` when it was not there.
pub(crate) fn without(list: &str, map: &str) -> Option<String> {
    let map = map_name(map);
    lists(list, map).then(|| {
        entries(list)
            .filter(|entry| !same_map(entry, map))
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// What a map does with server remaps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Choice {
    /// As `cg_serverRemaps` says.
    #[default]
    Default,
    /// Applied, filtered by `cg_remaps`, even with `cg_serverRemaps 0`.
    On,
    /// Ignored.
    Off,
}

impl Choice {
    /// The values of [`MAP_CVAR`], in the Settings row's order.
    pub(crate) const NAMES: [&'static str; 3] = ["default", "on", "off"];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Default => Self::NAMES[0],
            Self::On => Self::NAMES[1],
            Self::Off => Self::NAMES[2],
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "default" | "" => Some(Self::Default),
            "on" | "1" => Some(Self::On),
            "off" | "0" => Some(Self::Off),
            _ => None,
        }
    }

    /// The choice the two lists give `map`; a blocked map stays blocked.
    pub(crate) fn of(blocked: &str, allowed: &str, map: &str) -> Self {
        if lists(blocked, map) {
            Self::Off
        } else if lists(allowed, map) {
            Self::On
        } else {
            Self::Default
        }
    }

    /// The server remap mode: `server_on` is `cg_serverRemaps`, `filter` `cg_remaps`.
    pub(crate) fn mode(self, server_on: bool, filter: i64) -> i64 {
        match self {
            Self::Off => 0,
            Self::On => filter,
            Self::Default if server_on => filter,
            Self::Default => 0,
        }
    }

    /// The lists with `map` set to this choice, `(blocked, allowed)`, each `None`
    /// when unchanged.
    pub(crate) fn apply(
        self,
        blocked: &str,
        allowed: &str,
        map: &str,
    ) -> (Option<String>, Option<String>) {
        match self {
            Self::Default => (without(blocked, map), without(allowed, map)),
            Self::On => (without(blocked, map), with(allowed, map)),
            Self::Off => (with(blocked, map), without(allowed, map)),
        }
    }
}

/// The change counts of [`CVAR`], [`ALLOWED_CVAR`] and [`MAP_CVAR`].
pub(crate) type Revisions = [u64; 3];

/// What [`Cache::update`] wants written after a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sync {
    /// Nothing to write.
    None,
    /// The map or a list changed: show this choice in [`MAP_CVAR`].
    Show(Choice),
    /// [`MAP_CVAR`] was set: write this choice into the lists.
    Store(Choice),
}

/// The loaded map's choice, as of the last changes and map path seen.
#[derive(Default)]
pub(crate) struct Cache {
    revisions: Option<Revisions>,
    map: String,
    choice: Choice,
}

impl Cache {
    pub(crate) fn choice(&self) -> Choice {
        self.choice
    }

    /// Notice changes; `cvars` reads `(blocked, allowed, map cvar)` only after one.
    /// A [`MAP_CVAR`] change alone, on the same map, is the player's choice; any
    /// other change takes the choice from the lists.
    pub(crate) fn update<'a>(
        &mut self,
        revisions: Revisions,
        map: &str,
        cvars: impl FnOnce() -> (&'a str, &'a str, &'a str),
    ) -> Sync {
        let same_map = self.map == map;
        let seen = self.revisions.replace(revisions);
        if seen == Some(revisions) && same_map {
            return Sync::None;
        }
        let only_map_cvar = same_map && seen.is_some_and(|seen| seen[..2] == revisions[..2]);
        if !same_map {
            self.map.clear();
            self.map.push_str(map);
        }
        let (blocked, allowed, shown) = cvars();
        let loaded = !map_name(map).is_empty();
        if only_map_cvar && loaded {
            if let Some(choice) = Choice::parse(shown) {
                self.choice = choice;
                return Sync::Store(choice);
            }
        }
        self.choice = if loaded {
            Choice::of(blocked, allowed, map)
        } else {
            Choice::Default
        };
        if shown == self.choice.name() {
            Sync::None
        } else {
            Sync::Show(self.choice)
        }
    }

    /// Take `revisions` as seen, after this module wrote the cvars itself.
    pub(crate) fn seen(&mut self, revisions: Revisions) {
        self.revisions = Some(revisions);
    }
}

impl crate::GpuState {
    /// `cg_remaps` for the loaded map, after its choice and `cg_serverRemaps`.
    pub(crate) fn remap_mode(&mut self) -> i64 {
        let Some(console) = self.console.as_mut() else {
            return 1;
        };
        let map = self.world_load_map.as_str();
        let sync = self
            .remap_blocked_maps
            .update(console.remap_maps_revisions(), map, || {
                let text = |name| console.text_cvar(name).unwrap_or("");
                (text(CVAR), text(ALLOWED_CVAR), text(MAP_CVAR))
            });
        match sync {
            Sync::None => {}
            Sync::Show(choice) => {
                console.set_cvar(MAP_CVAR, choice.name());
            }
            Sync::Store(choice) => {
                let text = |name| console.text_cvar(name).unwrap_or("").to_owned();
                let (blocked, allowed) = (text(CVAR), text(ALLOWED_CVAR));
                let (blocked, allowed) = choice.apply(&blocked, &allowed, map);
                for (name, value) in [(CVAR, blocked), (ALLOWED_CVAR, allowed)] {
                    if let Some(value) = value {
                        console.set_cvar(name, &value);
                    }
                }
                // "1" or "OFF" typed in the console is shown by its name.
                if console.text_cvar(MAP_CVAR).ok() != Some(choice.name()) {
                    console.set_cvar(MAP_CVAR, choice.name());
                }
            }
        }
        if sync != Sync::None {
            self.remap_blocked_maps.seen(console.remap_maps_revisions());
        }
        self.remap_blocked_maps
            .choice()
            .mode(console.server_remaps_on(), console.remap_filter())
    }

    /// `blockRemaps`, `unblockRemaps`, `allowRemaps` and `unallowRemaps` (`command`
    /// in lower case): set a map's choice, the loaded map by default.
    pub(crate) fn remap_block_command(
        &mut self,
        command: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        let (choice, undo, usage) = match command {
            "blockremaps" => (Choice::Off, false, "usage: blockRemaps [map]"),
            "unblockremaps" => (Choice::Off, true, "usage: unblockRemaps [map]"),
            "allowremaps" => (Choice::On, false, "usage: allowRemaps [map]"),
            _ => (Choice::On, true, "usage: unallowRemaps [map]"),
        };
        let map = match args {
            [] => map_name(&self.world_load_map).to_owned(),
            [map] => map_name(map).to_owned(),
            _ => return Err(usage.into()),
        };
        if map.is_empty() {
            return Err(format!("No loaded map; {usage}"));
        }
        let console = self.console.as_mut().ok_or("Console unavailable")?;
        let text = |name| {
            console
                .text_cvar(name)
                .map(str::to_owned)
                .map_err(|error| error.to_string())
        };
        let (blocked, allowed) = (text(CVAR)?, text(ALLOWED_CVAR)?);
        let list = if choice == Choice::Off {
            CVAR
        } else {
            ALLOWED_CVAR
        };
        let current = Choice::of(&blocked, &allowed, &map);
        let target = match (undo, current == choice) {
            (false, true) => return Ok(vec![format!("{map} is already in {list}")]),
            (true, false) => return Ok(vec![format!("{map} is not in {list}")]),
            (false, false) => choice,
            (true, true) => Choice::Default,
        };
        let (blocked, allowed) = target.apply(&blocked, &allowed, &map);
        for (name, value) in [(CVAR, blocked), (ALLOWED_CVAR, allowed)] {
            if let Some(value) = value {
                if !console.set_cvar(name, &value) {
                    return Err(format!("could not set {name}"));
                }
            }
        }
        Ok(vec![match target {
            Choice::Off => format!("Server shader remaps ignored on {map}"),
            Choice::On => {
                format!("Server shader remaps applied on {map}, even with {SERVER_CVAR} 0")
            }
            Choice::Default => format!("Server shader remaps on {map} follow {SERVER_CVAR}"),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_names_drop_the_folder_and_extension() {
        assert_eq!(map_name("maps/mp/ffa4.bsp"), "mp/ffa4");
        assert_eq!(map_name("MAPS\\mp/ffa4.BSP"), "mp/ffa4");
        assert_eq!(map_name("/maps/mp/ffa4.bsp"), "mp/ffa4");
        assert_eq!(map_name("mp/ffa4"), "mp/ffa4");
        assert_eq!(map_name("mapsmp/ffa4"), "mapsmp/ffa4");
        assert_eq!(map_name(""), "");
        // A multi-byte character across byte 4 (the cvar is archived: this must not panic).
        assert_eq!(map_name("mp/\u{e9}t\u{e9}"), "mp/\u{e9}t\u{e9}");
    }

    #[test]
    fn listed_maps_match_regardless_of_case_and_separators() {
        let list = "mp/duel6, MP/FFA4;mp/ctf1";
        assert!(lists(list, "maps/mp/ffa4.bsp"));
        assert!(lists(list, "maps\\mp\\duel6.bsp"));
        assert!(lists(list, "mp/ctf1"));
        assert!(!lists(list, "maps/mp/ffa3.bsp"));
        assert!(!lists(list, "maps/mp/ffa.bsp"));
        assert!(!lists("", "maps/mp/ffa4.bsp"));
        assert!(!lists(list, ""));
    }

    #[test]
    fn adding_and_removing_keep_the_other_maps() {
        assert_eq!(with("", "maps/mp/ffa4.bsp").as_deref(), Some("mp/ffa4"));
        assert_eq!(
            with("mp/duel6", "mp/ffa4").as_deref(),
            Some("mp/duel6 mp/ffa4")
        );
        assert_eq!(with("mp/duel6 MP/FFA4", "mp/ffa4"), None);
        assert_eq!(
            without("mp/duel6,MP/FFA4 mp/ctf1", "maps/mp/ffa4.bsp").as_deref(),
            Some("mp/duel6 mp/ctf1")
        );
        assert_eq!(without("mp/duel6", "mp/ffa4"), None);
    }

    #[test]
    fn choices_come_from_the_lists_and_a_block_wins() {
        assert_eq!(Choice::of("", "", "mp/ffa4"), Choice::Default);
        assert_eq!(Choice::of("", "mp/ffa4", "maps/mp/ffa4.bsp"), Choice::On);
        assert_eq!(Choice::of("mp/ffa4", "", "mp/ffa4"), Choice::Off);
        assert_eq!(Choice::of("mp/ffa4", "mp/ffa4", "mp/ffa4"), Choice::Off);
    }

    #[test]
    fn modes_follow_the_choice_then_cg_server_remaps() {
        assert_eq!(Choice::Default.mode(false, 2), 0);
        assert_eq!(Choice::Default.mode(true, 2), 2);
        assert_eq!(Choice::On.mode(false, 1), 1);
        assert_eq!(Choice::On.mode(true, 2), 2);
        assert_eq!(Choice::Off.mode(true, 2), 0);
    }

    #[test]
    fn applying_a_choice_moves_the_map_between_lists() {
        assert_eq!(
            Choice::On.apply("mp/ffa4 mp/duel6", "", "mp/ffa4"),
            (Some("mp/duel6".into()), Some("mp/ffa4".into()))
        );
        assert_eq!(
            Choice::Off.apply("", "mp/ffa4", "maps/mp/ffa4.bsp"),
            (Some("mp/ffa4".into()), Some(String::new()))
        );
        assert_eq!(
            Choice::Default.apply("mp/duel6", "mp/ffa4", "mp/ffa4"),
            (None, Some(String::new()))
        );
        assert_eq!(Choice::Default.apply("", "", "mp/ffa4"), (None, None));
    }

    #[test]
    fn names_parse_back() {
        for choice in [Choice::Default, Choice::On, Choice::Off] {
            assert_eq!(Choice::parse(choice.name()), Some(choice));
        }
        assert_eq!(Choice::parse(" ON "), Some(Choice::On));
        assert_eq!(Choice::parse("0"), Some(Choice::Off));
        assert_eq!(Choice::parse("maybe"), None);
    }

    #[test]
    fn the_cache_shows_the_lists_choice_on_each_map() {
        let mut cache = Cache::default();
        let sync = cache.update([0, 0, 0], "maps/mp/ffa4.bsp", || ("", "mp/ffa4", "default"));
        assert_eq!(sync, Sync::Show(Choice::On));
        cache.seen([0, 0, 1]);
        let sync = cache.update([0, 0, 1], "maps/mp/ffa4.bsp", || panic!("read again"));
        assert_eq!(sync, Sync::None);
        assert_eq!(cache.choice(), Choice::On);
        // The next map has no choice: "default" again.
        let sync = cache.update([0, 0, 1], "maps/mp/ffa3.bsp", || ("", "mp/ffa4", "on"));
        assert_eq!(sync, Sync::Show(Choice::Default));
        // Already shown: nothing to write.
        let mut cache = Cache::default();
        let sync = cache.update([0, 0, 0], "maps/mp/ffa3.bsp", || ("", "", "default"));
        assert_eq!(sync, Sync::None);
    }

    #[test]
    fn setting_the_map_cvar_stores_the_choice() {
        let mut cache = Cache::default();
        cache.update([0, 0, 0], "maps/mp/ffa4.bsp", || ("", "", "default"));
        let sync = cache.update([0, 0, 1], "maps/mp/ffa4.bsp", || ("", "", "off"));
        assert_eq!(sync, Sync::Store(Choice::Off));
        assert_eq!(cache.choice(), Choice::Off);
        // A word the cvar does not know shows the lists' choice again.
        let sync = cache.update([0, 0, 2], "maps/mp/ffa4.bsp", || ("", "", "maybe"));
        assert_eq!(sync, Sync::Show(Choice::Default));
        // A list change is not the player's choice: the lists win.
        let sync = cache.update([1, 0, 3], "maps/mp/ffa4.bsp", || ("", "", "off"));
        assert_eq!(sync, Sync::Show(Choice::Default));
        // With no map loaded the cvar stays "default".
        let mut cache = Cache::default();
        cache.update([0, 0, 0], "", || ("", "", "default"));
        let sync = cache.update([0, 0, 1], "", || ("", "", "on"));
        assert_eq!(sync, Sync::Show(Choice::Default));
    }
}
