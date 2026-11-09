# Screenshots

The site's gallery shows the images listed in `manifest.json`, in order; the
feature panels and the home page use some of them too (`tour-1.jpg` to
`tour-4.jpg` are the home page's backdrop, `main-menu.jpg`, `scoreboard.jpg`,
`collection-shaders.jpg`, `collection-toys.jpg`, `servers.jpg`, `profile.jpg` and
`settings.jpg` the feature panels').

They are the client's own off-screen renders ([world shots](../../crates/sjk-viewer/src/world_shot.rs)),
taken without the build label:

```sh
WORLD_SHOT_NO_VERSION=1 JKA_GAME_DATA=<GameData> \
  cargo test --release -p sjk-viewer <test> -- --ignored --exact --test-threads=1
```

`<test>` is the test's full name, `world_shot::tests::<name>` for most of them;
`cargo test --release -p sjk-viewer -- --list --ignored` lists them. The
Collection's shots draw the blades from the SJK hub's packs: set `SJK_TEST_PACKS`
to the hub's `assets/dist`.

| File | Test | Render |
| --- | --- | --- |
| `tour-1.jpg`, `tour-2.jpg`, `tour-3.jpg`, `tour-4.jpg` | `duel6_site_backdrops` | `site-tour-1`, `-4`, `-6`, `-2` |
| `main-menu.jpg`, `settings.jpg`, `key-bindings.jpg` | `duel6_sjk_menu` | `duel6-menu`, `-menu-settings`, `-menu-keys` |
| `collection-shaders.jpg`, `collection-toys.jpg` | `world_shot::collection::duel6_sjk_collection` | `duel6-collection-shaders`, `-toys-lit` |
| `servers.jpg` | `duel6_sjk_browser` | `duel6-browser` |
| `force.jpg` | `duel6_player_sjk` | `duel6-player-sjk-force` |
| `profile.jpg` | `duel6_sjk_profile_screen` | `duel6-profile-screen-profile` |
| `loading.jpg` | `duel6_sjk_loading` | `duel6-loading-map` |
| `scoreboard.jpg`, `scoreboard-ctf.jpg` | `duel6_sjk_scoreboard` | `duel6-scoreboard-ffa`, `-ctf` |
| `ingame-menu.jpg` | `duel6_sjk_ingame` | `duel6-ingame` |
| `credits.jpg` | `duel6_sjk_credits` | `duel6-page-credits` |

The renders land in `target/world-shots/`; save them here as JPEG (quality 84,
1920 px wide). Leave out renders that show third-party mod content from your game
data (the Character tab's model grid shows every installed model's icon, and its
Saber tab lists installed hilt packs by name; the old `character.jpg` was dropped for
that), and renders of features no release has yet (on 25/10/2026, the Holocrons
tab's kyber crystals).

## The launcher's pictures

`launcher-home.jpg`, `launcher-content.jpg`, `launcher-pk3s.jpg`, `launcher-doctor.jpg`
and `launcher-tools.jpg` (the SJK Launcher section, not the gallery) are the SJK
Launcher's own window, saved by a debug build of
[sjk-launcher](https://github.com/Sol-Vulpes/sjk-launcher) with
`SJK_LAUNCHER_SHOT=<file.bmp>` and `SJK_LAUNCHER_TAB=sjk|content|pk3s|doctor|tools`
(`SJK_LAUNCHER_SEARCH=assets SJK_LAUNCHER_PK3=assets1.pk3` for the PK3s one, so the
list shows the game's and SJK's own PK3s rather than installed mods), 1800 px wide,
JPEG quality 84. Close the launcher you use first (a second start only brings up the
running one), take them with Developer mode off, and paint over the key id on the
Doctor's identity line: no key id goes on the site.

Game screenshots are fine to show, but do not add extracted game files
(textures, logos, fonts) to the site.
