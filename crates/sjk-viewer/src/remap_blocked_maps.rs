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
//!
//! SJK's creator also keeps a shared list on the hub (`PROTOCOL.md`, "Remap blocks",
//! since 05/11/2026), given with presence. This client keeps it in
//! `cg_remapsSharedMaps` (archived, so it holds from the start of the next run) and,
//! while `cg_remapsShared` is on (the default, Sol's choice), ignores server remaps on
//! those maps whose own choice is `default`. A map the player set `on` keeps them.
//! `syncBlockRemaps` / `syncUnblockRemaps [map]` change the shared list (creator only).

/// Maps whose server remaps are ignored, archived with the player's settings.
pub(crate) const CVAR: &str = "cg_remapsBlockedMaps";
/// Maps whose server remaps apply while `cg_serverRemaps` is off, archived.
pub(crate) const ALLOWED_CVAR: &str = "cg_remapsAllowedMaps";
/// The loaded map's [`Choice`], by name; not archived.
pub(crate) const MAP_CVAR: &str = "cg_serverRemapsMap";
/// Whether the hub's shared list applies (on by default), archived.
pub(crate) const SHARED_CVAR: &str = "cg_remapsShared";
/// The hub's shared list as presence last gave it, written by the client, archived.
pub(crate) const SHARED_LIST_CVAR: &str = "cg_remapsSharedMaps";
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

/// The change counts of [`CVAR`], [`ALLOWED_CVAR`], [`MAP_CVAR`], [`SHARED_LIST_CVAR`]
/// and [`SHARED_CVAR`].
pub(crate) type Revisions = [u64; 5];

/// What [`Cache::update`] reads after a change.
pub(crate) struct Lists<'a> {
    pub blocked: &'a str,
    pub allowed: &'a str,
    /// [`MAP_CVAR`] as it stands.
    pub shown: &'a str,
    /// The shared list, or `None` while [`SHARED_CVAR`] is off.
    pub shared: Option<&'a str>,
}

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
    /// Whether the shared list applies and names the loaded map.
    shared: bool,
    /// When the hub's shared list was last looked at, and its revision then.
    polled: Option<(std::time::Instant, u64)>,
}

impl Cache {
    pub(crate) fn choice(&self) -> Choice {
        self.choice
    }

    /// Whether the shared list ignores the loaded map's server remaps: it applies and
    /// names the map, whose own choice is `default`.
    pub(crate) fn shared_block(&self) -> bool {
        self.shared && self.choice == Choice::Default
    }

    /// The server remap mode, as [`Choice::mode`] after the shared list.
    pub(crate) fn mode(&self, server_on: bool, filter: i64) -> i64 {
        if self.shared_block() {
            0
        } else {
            self.choice.mode(server_on, filter)
        }
    }

    /// Notice changes; `cvars` reads the [`Lists`] only after one. A [`MAP_CVAR`]
    /// change alone, on the same map, is the player's choice; any other change takes
    /// the choice from the lists.
    pub(crate) fn update<'a>(
        &mut self,
        revisions: Revisions,
        map: &str,
        cvars: impl FnOnce() -> Lists<'a>,
    ) -> Sync {
        let same_map = self.map == map;
        let seen = self.revisions.replace(revisions);
        if seen == Some(revisions) && same_map {
            return Sync::None;
        }
        let only_map_cvar = same_map
            && seen.is_some_and(|seen| seen[..2] == revisions[..2] && seen[3..] == revisions[3..]);
        if !same_map {
            self.map.clear();
            self.map.push_str(map);
        }
        let Lists {
            blocked,
            allowed,
            shown,
            shared,
        } = cvars();
        let loaded = !map_name(map).is_empty();
        self.shared = loaded && shared.is_some_and(|shared| lists(shared, map));
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
    /// The mode [`Self::remap_mode`] will give `map` once it is loaded, read from the
    /// lists without writing anything: a world built on a worker applies the server's
    /// remaps with it, and its first live frame checks them with the real mode.
    pub(crate) fn remap_mode_for(&self, map: &str) -> i64 {
        let Some(console) = self.console.as_ref() else {
            return 1;
        };
        let loaded = !map_name(map).is_empty();
        let text = |name| console.text_cvar(name).unwrap_or("");
        let choice = if loaded {
            Choice::of(text(CVAR), text(ALLOWED_CVAR), map)
        } else {
            Choice::Default
        };
        let shared = loaded
            && console.bool_cvar(SHARED_CVAR) != Some(false)
            && lists(text(SHARED_LIST_CVAR), map);
        if shared && choice == Choice::Default {
            return 0;
        }
        choice.mode(console.server_remaps_on(), console.remap_filter())
    }

    /// `cg_remaps` for the loaded map, after its choice and `cg_serverRemaps`.
    pub(crate) fn remap_mode(&mut self) -> i64 {
        let Some(console) = self.console.as_mut() else {
            return 1;
        };
        follow_hub_list(&mut self.remap_blocked_maps, console);
        let map = self.world_load_map.as_str();
        let sync = self
            .remap_blocked_maps
            .update(console.remap_maps_revisions(), map, || {
                let text = |name| console.text_cvar(name).unwrap_or("");
                Lists {
                    blocked: text(CVAR),
                    allowed: text(ALLOWED_CVAR),
                    shown: text(MAP_CVAR),
                    shared: (console.bool_cvar(SHARED_CVAR) != Some(false))
                        .then(|| text(SHARED_LIST_CVAR)),
                }
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
            .mode(console.server_remaps_on(), console.remap_filter())
    }

    /// `syncBlockRemaps` and `syncUnblockRemaps` (`command` in lower case): SJK's
    /// creator adds a map (the loaded one by default) to the hub's shared list, or
    /// takes it off.
    pub(crate) fn remap_shared_command(
        &mut self,
        command: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        let (blocked, usage) = match command {
            "syncblockremaps" => (true, "usage: syncBlockRemaps [map]"),
            _ => (false, "usage: syncUnblockRemaps [map]"),
        };
        if !crate::player_identity::is_creator() {
            return Err("Only SJK's creator changes the shared remap blocks".to_owned());
        }
        let map = match args {
            [] => map_name(&self.world_load_map),
            [map] => map_name(map),
            _ => return Err(usage.into()),
        }
        .replace('\\', "/")
        .to_ascii_lowercase();
        if map.is_empty() {
            return Err(format!("No loaded map; {usage}"));
        }
        let request = sjk_identity::StaffRequest::RemapBlock {
            map: map.clone(),
            blocked,
        };
        if !crate::player_identity::staff(request) {
            return Err("The SJK hub is off (cl_identity 0)".to_owned());
        }
        Ok(vec![if blocked {
            format!("Asked the SJK hub to ignore server remaps on {map} for every SJK player")
        } else {
            format!("Asked the SJK hub to take {map} off the shared remap blocks")
        }])
    }
}

/// How often the hub's shared list is looked at, at most.
const POLL_EVERY: std::time::Duration = std::time::Duration::from_millis(500);

/// Copy the hub's shared list into [`SHARED_LIST_CVAR`] when presence gave a new one;
/// looked at twice a second, it allocates only on a change.
fn follow_hub_list(cache: &mut Cache, console: &mut crate::console::ViewerConsole) {
    let now = std::time::Instant::now();
    if cache
        .polled
        .is_some_and(|(polled, _)| now.duration_since(polled) < POLL_EVERY)
    {
        return;
    }
    let seen = cache.polled.map_or(0, |(_, revision)| revision);
    let Some((revision, maps)) = crate::player_identity::remap_blocks(seen) else {
        cache.polled = Some((now, seen));
        return;
    };
    cache.polled = Some((now, revision));
    let list = maps.join(" ");
    if console.text_cvar(SHARED_LIST_CVAR).ok() != Some(list.as_str()) {
        console.set_cvar(SHARED_LIST_CVAR, &list);
    }
}

impl crate::GpuState {
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

    /// The player's own lists, no shared one.
    fn own<'a>(blocked: &'a str, allowed: &'a str, shown: &'a str) -> Lists<'a> {
        Lists {
            blocked,
            allowed,
            shown,
            shared: None,
        }
    }

    #[test]
    fn the_cache_shows_the_lists_choice_on_each_map() {
        let mut cache = Cache::default();
        let sync = cache.update([0, 0, 0, 0, 0], "maps/mp/ffa4.bsp", || {
            own("", "mp/ffa4", "default")
        });
        assert_eq!(sync, Sync::Show(Choice::On));
        cache.seen([0, 0, 1, 0, 0]);
        let sync = cache.update([0, 0, 1, 0, 0], "maps/mp/ffa4.bsp", || panic!("read again"));
        assert_eq!(sync, Sync::None);
        assert_eq!(cache.choice(), Choice::On);
        // The next map has no choice: "default" again.
        let sync = cache.update([0, 0, 1, 0, 0], "maps/mp/ffa3.bsp", || {
            own("", "mp/ffa4", "on")
        });
        assert_eq!(sync, Sync::Show(Choice::Default));
        // Already shown: nothing to write.
        let mut cache = Cache::default();
        let sync = cache.update([0, 0, 0, 0, 0], "maps/mp/ffa3.bsp", || {
            own("", "", "default")
        });
        assert_eq!(sync, Sync::None);
    }

    #[test]
    fn setting_the_map_cvar_stores_the_choice() {
        let mut cache = Cache::default();
        cache.update([0, 0, 0, 0, 0], "maps/mp/ffa4.bsp", || {
            own("", "", "default")
        });
        let sync = cache.update([0, 0, 1, 0, 0], "maps/mp/ffa4.bsp", || own("", "", "off"));
        assert_eq!(sync, Sync::Store(Choice::Off));
        assert_eq!(cache.choice(), Choice::Off);
        // A word the cvar does not know shows the lists' choice again.
        let sync = cache.update([0, 0, 2, 0, 0], "maps/mp/ffa4.bsp", || own("", "", "maybe"));
        assert_eq!(sync, Sync::Show(Choice::Default));
        // A list change is not the player's choice: the lists win.
        let sync = cache.update([1, 0, 3, 0, 0], "maps/mp/ffa4.bsp", || own("", "", "off"));
        assert_eq!(sync, Sync::Show(Choice::Default));
        // With no map loaded the cvar stays "default".
        let mut cache = Cache::default();
        cache.update([0, 0, 0, 0, 0], "", || own("", "", "default"));
        let sync = cache.update([0, 0, 1, 0, 0], "", || own("", "", "on"));
        assert_eq!(sync, Sync::Show(Choice::Default));
    }

    #[test]
    fn the_shared_list_blocks_maps_left_at_default_while_followed() {
        let shared = |shown, list| Lists {
            blocked: "",
            allowed: "",
            shown,
            shared: list,
        };
        let mut cache = Cache::default();
        cache.update([0; 5], "maps/mp/ffa4.bsp", || {
            shared("default", Some("mp/ffa4"))
        });
        assert!(cache.shared_block());
        assert_eq!(cache.mode(true, 2), 0);
        // The hub took it off the list.
        cache.update([0, 0, 0, 1, 0], "maps/mp/ffa4.bsp", || {
            shared("default", Some(""))
        });
        assert!(!cache.shared_block());
        assert_eq!(cache.mode(true, 2), 2);
        // Back on the list, but the player stopped following it.
        cache.update([0, 0, 0, 2, 1], "maps/mp/ffa4.bsp", || {
            shared("default", None)
        });
        assert_eq!(cache.mode(true, 2), 2);
        // Following again: a map the player set on keeps its remaps.
        let mut cache = Cache::default();
        cache.update([0; 5], "maps/mp/ffa4.bsp", || Lists {
            blocked: "",
            allowed: "mp/ffa4",
            shown: "on",
            shared: Some("mp/ffa4"),
        });
        assert_eq!(cache.choice(), Choice::On);
        assert!(!cache.shared_block());
        assert_eq!(cache.mode(false, 1), 1);
        // A change of the shared list is not the player's choice for the map.
        let mut cache = Cache::default();
        cache.update([0; 5], "maps/mp/ffa4.bsp", || shared("default", Some("")));
        let sync = cache.update([0, 0, 1, 1, 0], "maps/mp/ffa4.bsp", || {
            shared("off", Some(""))
        });
        assert_eq!(sync, Sync::Show(Choice::Default));
    }
}
