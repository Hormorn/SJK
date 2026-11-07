# SJK conventions

Rules that belong to SJK as a project: names, defaults, versions, releases,
credits. [AGENTS.md](../AGENTS.md) and [development.md](development.md) hold the
general contributor rules.

## Branches

- `main` of [Sol-Vulpes/SJK](https://github.com/Sol-Vulpes/SJK) is SJK. It is
  updated by merging, never rebased.
- A change is named `personal/<topic>`, starts from `main` and is merged back
  without a pull request. Contributors' changes arrive as pull requests on SJK
  ([development.md](development.md#branches-commits-and-pull-requests)).

## Names

- Everything a player sees says SJK: the programs `sjk` and `sjk-server`, the
  release ZIPs, the `GameData/SJK/` folder and messages.
- New settings get neutral engine names (`r_*`, `cg_*`, `cl_*`, ...), never a
  `sjk_` prefix, and must not collide with EternalJK or rend2 settings.
- A client mod's commands and settings start with its id and a dot
  (`japlus.guntele`, `jof.commands`), and its switch is `mod_<id>`
  ([mods.md](mods.md)).
- Every toy is a `toy_<name>` command (`toy_help` lists them all; `toy_illuminate`, `toy_box`,
  `toy_order66`, `toy_hax`, `toy_ragdoll`, `toy_drone`, `toy_boombox`, `toy_monitor`,
  `toy_boxhunt`, `toy_banana`); a
  toy has no Force wheel entry, and one with a setting names it `cg_toy<Name>...`.
  Illuminate, the cardboard box and Order 66 have none: they are always available and
  on only for the client's run. A toy others see has an id in the look's `toys`
  (`cardboard_box`, `order66`). A toy may have a command of its own besides
  (`order66`, which gives the order; `monitor_throw`, which throws a monitor, and
  `+monitor_throw`, its held bind, which charges the throw; `banana_drop`, which drops a
  banana peel; `boombox_place`, which stands the boombox on the floor).
- The crates are SJK's too: `crates/sjk-*`, packages `sjk-*` (`cargo build -p
  sjk-viewer -p sjk-dedicated`) and Rust paths `sjk_*`. Environment variables for
  diagnostics are `SJK_*`; the game-data and server paths keep the neutral
  `JKA_GAME_DATA` and `JKA_DEDICATED`.
- SJK's emblem (Sol's) is its picture: the SJK UI's and classic main menus, the
  programs' and window's icons, the README, release notes and site. Every image
  of it is generated from one original by the scripts in
  [assets/branding](../assets/branding/README.md); regenerate them rather than
  editing one by hand.
- Public text ([README](../README.md), [CREDITS.md](../CREDITS.md), the site)
  presents SJK as Sol's: Sol develops it and sets its direction, and Sol comes
  first (Sol's rule, 06/10/2026). SJK stands on its own (Sol's decision,
  08/10/2026): JKR, the engine SJK began from, and Bishop, its creator, appear only
  in the credits (the Origins section, last in CREDITS.md and on the credits page), the
  copyright notice and one footnote in the README and on the site. Code, wiki
  pages and in-game text describe SJK without comparing it to JKR. Keep CREDITS.md
  current when SJK gains notable work.

## Defaults

New settings default to the stock base Jedi Academy experience (Sol's rule,
30/10/2026; [AGENTS.md](../AGENTS.md)): what changes how the game looks, sounds or
plays compared with stock JKA is opt-in, and only Sol moves a default away from stock
(the table below). #292, server remaps on by default, was declined under it.

A new profile starts with Sol's own choices. Defaults apply only to settings a
`config.cfg` has not saved: an existing profile keeps its values and nothing is
migrated, except where the table says so. Where each is documented:

| Setting | Default | See |
| --- | --- | --- |
| `ui_menuStyle` | `sjk` (the SJK UI), saved `classic` moved to `sjk` once, saved `modern` reset | [Menu style](client.md#menu-style) |
| `ui_gameFont` | on | [UI ownership](rendering.md#ui-ownership) |
| `cg_scoreboardStyle` | `auto` (SJK UI's with its menus, else `classic`), saved `modern` reset | [Scoreboard styles](client.md#scoreboard-styles) |
| `cg_compactScoreboard` | on (SJK UI's scoreboard in one column, as wide as its names, centred) | [SJK UI scoreboard](sjk-ui.md#scoreboard) |
| `cg_scoreboardAfk` | on (away players faint below the others on both scoreboards) | [Away players](client.md#away-players) |
| `cg_drawTimer` | on | [status.md](status.md#gameplay-and-interface-defaults) |
| `cg_teamOutline` | on (red rings on enemies, green on teammates, in team games) | [Team outlines and logos](client.md#team-outlines-and-logos) |
| `cg_killfeed` | on, saved 0 moved to 1 once | [Kill feed](client.md#kill-feed) |
| `cg_mapEdits` | on (SJK's own map fixes from the hub) | [Map edits](map-edits.md) |
| `cg_serverRemaps` | off (servers' shader remaps; the map's own apply), asked on the 18/10 what's-new card | [Shader remap controls](client.md#shader-remap-controls) |
| `cg_remapsShared` | on (Sol, 05/11/2026): server remaps ignored on the maps of the creator's shared list, unless the player chose the map | [Shader remap controls](client.md#shader-remap-controls) |
| `cg_hitMarker` | off (red damage-direction indicator) | [Client](client.md#launch) |
| `cg_shieldBrightness` | 2 passes, old defaults 1 and 4 moved to 2 once | [Client](client.md#launch) |
| `cg_duelGhost` | on: duellists as JoF EJK's blue figures (off: dark under the bubble) | [Rendering](rendering.md#entity-render-effects) |
| `cg_duelHideOthers` | 1, Auto: in a private duel, players outside it hidden on JA+ (unless `cp_pluginDisable` bit 1) and jaPRO, as JoF EJK | [Client](client.md#nameplates) |
| `cg_nameplateStyle` | 0, Holo (the holo frame; 1 the capsule, 2 the classic plate) | [Nameplates](client.md#nameplates) |
| `cg_dismember` | 2 | [Dismemberment](client.md#dismemberment-and-disintegration) |
| `snaps` | 120 (slider to 125) | [status.md](status.md#gameplay-and-interface-defaults) |
| `com_maxfpsUnfocused` | 30 | [Configuration](client.md#configuration-and-content) |
| `com_maxfpsMonitor` | off (`com_maxfps` AUTO caps at 125, the monitor's rate unread) | [Configuration](client.md#configuration-and-content) |
| `cl_maxpackets` | 125 | [User commands and move packets](networking.md#user-commands-and-move-packets) |
| `gameplay_fps` | 0, Auto: movement at the FPS cap, a `pmove_fixed` server's step wins | [Gameplay FPS](client.md#gameplay-fps) |
| `gameplay_fps_nocap` | 0: the known server caps (JoF's 125) hold | [User commands and move packets](networking.md#user-commands-and-move-packets) |
| `cjp_client` | empty: `1.4 SJK` on JA+, `1.4JAPRO` on jaPRO (its client-support switch); only a staff key's value is sent | [Networking](networking.md) |
| `rate` | 90000 (stock 25000), saved 25000 moved to 90000 once | [User commands and move packets](networking.md#user-commands-and-move-packets) |
| rendering profile | noon, bloom, dust, material maps | [Default visual profile](rendering.md#default-visual-profile) |
| `cl_consoleUseScanCode` | 1, saved 0 moved to 1 once | [Useful console commands](client.md#useful-console-commands) |
| `mod_japlus`, `mod_jof` | on (each mod loads on its servers only), saved 0 moved to 1 once | [Client mods](mods.md) |
| `con_drawNotify` | off (no console feed at the top left; chat keeps its box) | [Classic console](client.md#classic-console) |
| `cl_experimentalShowcase` | off (the experimental `ghost` and `mimic` commands, staff keys only) | [Ghost replay](client.md#ghost-replay) |
| `ui_sliderPeek` | on: over a match, a Settings slider held a moment fades the rest of the screen away | [Settings](sjk-ui.md#settings) |
| `cl_sjkChatJof`, `cl_sjkChatS` | -1, Auto: a verified JoF or S player reads their own clan's chat, staff both (Sol's spec, 03/11/2026); the other only when chosen | [Clan chats](hub-chat.md#clan-chats) |
| `ui_supportLinks` | on, but shelved since 07/11/2026: Support SJK's links never show ([Donations](#donations)) | [Main page](sjk-ui.md#main-page) |

The HUD look (`cg_hudStyle game`) and the console (`con_style auto`: the SJK UI's
console with its menus, else the classic one; a saved `classic` moved once to
`auto`) are SJK defaults too, documented with their pages; a saved `modern` in
either is reset at start, as the modern style is retired
([Menu style](client.md#menu-style)).

## Fonts

SJK draws no bitmap fonts (Sol's rule, 07/10/2026): text uses vector fonts,
bundled under an open licence (OFL or Apache) and rasterized once at startup, as
Inter is ([text.rs](../crates/sjk-viewer/src/text.rs)). Retail bitmap fonts are
replaced, by an open-licence look-alike or by converting them to vector outlines
where that is possible, never added. A bundled font ships with its licence next to
it in `crates/sjk-viewer/assets/fonts` and is credited in CREDITS.md and
credits.txt.

| Retail bitmap | Used for | Replacement |
| --- | --- | --- |
| `gfx/2d/charsgrid_med` | Console, notify lines, cgame small/big strings | JetBrains Mono ([console_font.rs](../crates/sjk-viewer/src/text/console_font.rs)) |
| `ocr_a` | Chat, selection names, scoreboard numbers (`ui_gameFont`) | SJK Chat: OCR-A (public domain) set to the retail widths ([retail_font.rs](../crates/sjk-viewer/src/text/retail_font.rs)) |
| `ergoec` | Menus, scoreboard names (`ui_gameFont`) | SJK Menu: traced from the JoF HD pack's atlas |
| `ergoec` | Centre prints, and the words in the world (over mystery boxes, the mark), whatever `ui_gameFont` says | Fira Sans Condensed SemiBold and Bold (OFL, subset to Windows-1252 with fontTools; the rest from Exo 2), Sol's pick of open fonts close to `ergoec` (28/10/2026) |
| `arialnb` | Classic status HUD (`cg_classicHudFont`) | SJK HUD: traced from the JoF HD pack's atlas |
| `logo` glyph (0xAC) | The "WSI fonts" logo in Inter's atlas | SJK Menu's vector logo ([logo_glyph.rs](../crates/sjk-viewer/src/text/logo_glyph.rs)) |

Characters outside the 256 code-page slots (Cyrillic, Japanese, Chinese, Korean)
come from the same vector fonts, Cyrillic from Inter, Exo 2 or JetBrains Mono, kana
and kanji from Noto Sans JP (OFL), subset to kana, kanji, CJK punctuation and
full-width forms (3 MB), the GB 2312 hanzi Noto Sans JP lacks from Noto Sans SC
(0.5 MB) and Hangul from Noto Sans KR (1.1 MB), all three subset with fontTools.
Emoji are Twemoji's colour pictures (CC-BY 4.0), not a font
([rendering.md](rendering.md#characters-beyond-the-code-page)).

SJK Menu, SJK Chat and SJK HUD keep the retail layout (advances, glyph positions,
line height and baseline), so every surface lays out as it did with the bitmaps.
[game_fonts.py](../scripts/game_fonts.py) rebuilds them from a player's install
(the retail `.fontdat` metrics in `assets1.pk3`, the HD atlases of
`JoF_HDFonts&Icons.pk3`) and Sauter's `OCRA.ttf`, none of which are committed:
`ergoec` and `arialnb` are traced with light smoothing; OCR-A gets the retail
widths, each glyph centred in its advance (the retail atlas cells are padded
unevenly, so following them spaced the even OCR-A letters unevenly), and the Latin-1 letters and symbols it lacks are composed from its own
glyphs (accented capitals squashed under the accent, as its own `Ñ` is), except
seven (`ß þ Þ ð § ¶ ¤`) traced from the HD `ocr_a`. Sol chose the traced menu and
HUD fonts and OCR-A Regular from side-by-side comparisons (07/10/2026). The JoF
pack's author is unknown and it states no licence: Sol chose to bundle the
tracings on the assumption that it is shared freely, crediting the pack and
removing them at the author's request.

The first build (07/10/2026) placed most glyphs wrongly: fontTools'
`removeOverlaps` draws through the glyph set, which moves each outline to its
`hmtx` left bearing, and the bearings were still 0, so every glyph with
overlapping contours (most of OCR-A, about half of the traced fonts) landed flush
left while the rest kept their place. `game_fonts.py` now sets the bearings from
the outlines before that step.
[SJK-fonts.txt](../crates/sjk-viewer/assets/fonts/SJK-fonts.txt) records this
next to the fonts.

## Copyright notice

[NOTICE](../NOTICE) is SJK's copyright notice: Sol-Vulpes, Bishop-R and the JKR
contributors, under GPL-2.0-only. Bishop-R and the JKR contributors hold the
copyright of the code SJK began from, so the line keeps naming them. The client
([notice.rs](../crates/sjk-viewer/src/notice.rs), printed to its log and console
by `app_launch.rs`) and the dedicated server (`NOTICE` in its `main.rs`) announce
the copyright and the absence of warranty at startup, so GPLv2 section 2(c)
requires modified versions to keep announcing them. Keep the two copies of the
lines in step. SJK packages put `NOTICE` first in `LICENSES-SJK.txt`. The SJK
names and emblem are not licensed with the code (see `NOTICE`).

## Version

One file decides the programs' version: [build_version.rs](../scripts/build_version.rs),
which both programs' `build.rs` include. Releases are numbered by date and a
counter, `YYYY.MMDD.N`: the first release of 05/10/2026 is `2026.1005.1`, a second
one that day `2026.1005.2` (Sol's choice, 05/10/2026). The form sorts, is a valid
Cargo and Windows version, and is tagged `sjk-v2026.1005.1`; people see the date
itself day first in the label below. A release build gets the version of its tag
through `SJK_VERSION`, which the release workflow sets; any other build says `dev`,
so a local build never passes for a release. The Cargo package version (`0.1.0`)
is not the release number. The source commit's short hash and committer
time come from git when the source is a checkout, else from `SJK_COMMIT` and
`SJK_COMMIT_TIME`, else they are left out. A build is redone when `HEAD` moves;
uncommitted edits keep the last commit's label. The client shows
`SJK <version> · <dd/mm/yyyy HH:MM> · <commit>` at the top of the screen
([client.md](client.md#version-label)) and in its log, the menus and console show
`SJK <version>`, and both startup notices carry the version.

## Dates and times

Dates that players see, and dates in SJK's own text (release notes, Discord posts,
SJK's own pages), are written day first, `dd/mm/yyyy`, and times on the 24-hour
clock, `HH:MM`, with no AM or PM (Sol's rule, 05/10/2026). The version label and the
classic console's clock follow it. Machine formats are exempt: git tags, ISO 8601
timestamps in logs and JSON, and file names meant to sort.

## Automation

| Workflow | Runs on | Does |
| --- | --- | --- |
| [CI](../.github/workflows/ci.yml) | Pull requests, pushes to `main` | Formatting, workspace build, tests and clippy |
| [Pages](../.github/workflows/pages.yml) | Pushes to `main` changing `site/` or the workflow | Publishes `site/` to https://sol-vulpes.github.io/SJK/ |
| [SJK release](../.github/workflows/release.yml) | Tags `sjk-v<version>` | Builds and publishes the release ZIPs |

The site is static: `index.html`, `assets/style.css` and `assets/site.js` (download
entry, release recap and log, platform steps, contributor counts, screenshot gallery,
donation card). It wears the SJK UI's look ([sjk-ui.md](sjk-ui.md), Sol's request of
09/10/2026). Its layout since 25/10/2026 is the one Sol chose from three proposals:
the home page keeps the client's main page, SJK's emblem in its turning ring, with the
title, a short introduction and a gold Download button beside it (the first version's
menu arc, whose entries grew and moved under the pointer, was hard to use; the plain
column after it gave way to the fixed navigation bar, clear over the home page and
solid below it), the latest release's recap where the client lists recent servers,
and a profile card for Sol with Sol's GitHub picture (the initial shows if it does not
load), over shots of the client's tour of Yavin Training Grounds that cross-fade every
15 seconds. The home page wraps to one column on narrow screens. What SJK brings is
laid out as the client's Settings (a rail of categories, rows, a detail column with a
screenshot); Why SJK, the gallery, What's new, Install and Credits follow.

The script fills what changes from GitHub, and the page keeps working links without
it:

- **Download:** the latest published release (the releases API); the button takes
  the visitor's platform (Windows, Linux, or Mac from the browser's description;
  phones and tablets get Windows) and names it, and the line under it links the other
  platforms' ZIPs and the release's files and checksums.
- **Recap and What's new:** `CHANGELOG.md` read from `main` on
  raw.githubusercontent.com: the home page shows the latest release's recap lines
  (`> Heading: sentence`, at most six), What's new the last five releases GitHub has
  published, each with its date, introduction, recap and a link to its changes in
  the changelog. The format is the client's ([Changelog](#changelog)); a release's
  notes written before its tag stay off the site until the release is published.
- **Install:** one platform at a time, the visitor's first, with buttons for the
  others; without the script all three are listed.
- **Credits:** each contributor card (`data-author`, the GitHub login) shows its
  merged pull requests in SJK, from one GitHub search over all of them (`is:merged`,
  so a pull request Sol merged by hand counts only once GitHub marks it merged). The
  search allows 10 requests a minute without a token, so the counts are kept an hour
  in the visitor's browser; without an answer the cards say "Pull requests" and link
  the list. A new contributor needs a card in `index.html`.

Its pictures are the client's own off-screen renders
([screenshots/README.md](../site/screenshots/README.md)). The sun behind the
emblem (a warm glow, uneven god rays and an 18-ray sunburst turning against each
other) is CSS (`.sun` in `style.css`), so every browser shows it; it once was a
WebGPU layer, which headless Chrome cannot capture in a screenshot.
`assets/effects.js` adds drifting dust and a saber trail on the pointer with the
MIT-licensed
[Shaders](https://github.com/shader-effects-inc/shaders) library, vendored as
`assets/vendor/shaders/shaders-4.0.0.js` with its license so no other server is
contacted; its telemetry is turned off. When the visitor asks for reduced motion
(Windows' "Animation effects" off does), the scene is drawn once and held still,
without the saber trail; the backdrop stays on its first shot and the sun stops turning. Without WebGPU
the script adds nothing. To
update the library, replace the vendored file with a release's
`dist/js/bundle.js` and check that `disableTelemetry` still stops its telemetry.

Visits are counted with [GoatCounter](https://www.goatcounter.com) (no cookies, no
personal data; the footer says so), whose ISC-licensed `count.js` is vendored as
`assets/vendor/goatcounter/count.js` with its license, so the script is not fetched
from another server; only the counts go to `sjkclient.goatcounter.com` (the
`data-goatcounter` address in `site/index.html`; Slash holds that counter for now, and
another counter only needs that address changed). Links posted on video
platforms carry `?utm_source=<platform>` (and `utm_campaign`), which GoatCounter lists
as the visit's source, so the dashboard shows which platform brings visitors. The page's
`rel="canonical"` link makes them all count as the one page `/SJK/`. To
update the script, replace the vendored file with GoatCounter's current
`public/count.js`.

Release tags exist only on SJK. They are cut by Sol or, since 03/11/2026, by the
maintainers Sol gave write access to SJK (Slash and Milamber, and Lumaya since
05/11/2026), who may also review pull requests and merge or push to `main`, as
[reviewing.md](reviewing.md) says. The release title carries the stage, "Sol JK 2026.1005.1 (Alpha)" (`STAGE` in the workflow).
Alphas are not marked as pre-releases, so the site's download link
(`releases/latest`) finds them; their name and notes say "Alpha" instead.

Changes to `main` and to release tags only add: GitHub rulesets refuse a force-push
or deletion of `main` and a deletion or move of any `sjk-v*` tag, for everyone. A
mistake is fixed by a new commit, and a bad release by the next release
(`.N + 1`), never by rewriting.

### Cutting a release

1. **Hub first.** If `main` holds merges whose notes say "needs hub ... deployed
   first", check with Sol that the hub is deployed before tagging; only Sol deploys
   the hub.
2. **Changelog.** In [CHANGELOG.md](../CHANGELOG.md) rename `## Unreleased` to
   `## <version> | <dd/mm/yyyy>`, the version `YYYY.MMDD.N` ([Version](#version)), and
   write its recap, three to six `> Heading: sentence` lines in player words
   ([Changelog](#changelog)). Do not leave an empty `## Unreleased` above it: the
   test refuses a section with no change; the next merge adds it back.
3. **Check.** `cargo test -p sjk-viewer --bin sjk changelog_data` must pass (it checks the
   credits, the dates and the recap).
4. **Commit and tag.** Commit as `Release <version>`, push `main`, then tag that
   commit and push the tag: `git tag sjk-v<version>` and
   `git push origin sjk-v<version>`. The [SJK release](../.github/workflows/release.yml)
   workflow builds and publishes the ZIPs; check it is green and the release page
   has its files.
5. **Announce.** Every release gets an SJK chat announcement (below): send its draft
   to Sol, who posts it. A Discord ping is only for releases Sol calls big.

## Donations

SJK takes donations on two pages: Sol's GitHub Sponsors page,
https://github.com/sponsors/Sol-Vulpes (since 25/10/2026), and Ko-fi,
https://ko-fi.com/soljk (Sol's page, 13/10/2026). They are offered in two places:

- the GitHub repository's Sponsor button, GitHub Sponsors only (Sol's choice), through
  [.github/FUNDING.yml](../.github/FUNDING.yml), with "Sponsorships" ticked in the
  repository's settings (General, Features);
- Support SJK, a small line at the top left of the SJK UI's main page (since
  26/10/2026, Sol's request; [sjk-ui.md](sjk-ui.md#main-page)): a link to each page and
  ×, which hides it for good (`ui_supportLinks 0`; Settings, Interface, "Support SJK"
  shows it again). Both addresses are in
  [support_links.rs](../crates/sjk-viewer/src/support_links.rs); it asks no server
  anything. Shelved since 07/11/2026 (Sol's request): `support_links::SHELVED` hides
  the line for everyone and its Settings row is gone. To bring it back, set `SHELVED`
  to false and restore the "Support SJK" row in `settings/catalog.rs`, its place in
  `settings/groups.rs` (Interface) and its line in `settings/help.rs`.

Nothing is sold: a donation brings no feature and no advantage in the game, and
neither place promises anything for one.

### The shelved goal

From 13/10/2026 the site and the main page showed a live monthly goal on Ko-fi
("80 € of 200 € this month", for a full Claude plan used only for SJK) and offered the
Supporter medal for more than 1 euro. Sol shelved both on 14/10/2026. What is left:

- Site: the Support SJK section and its two menu links carry `hidden`, and `site.js`
  skips the hub (`DONATION.shelved`). To bring it back: `DONATION.shelved = false` and
  the three `hidden` attributes in `index.html`. Its Ko-fi address is in
  `DONATION.url` and in the section's link. The section shows the medallion from
  `site/assets/medal-supporter.png`, a copy of the client's `supporter_small.png`:
  replace both when the art changes.
- Client: the goal card and its reader of the hub were removed on 26/10/2026, when
  Support SJK took their place (`donation_goal.rs` and `sjk_identity::donations` are
  in git history before that date). The Collection lists the Supporter medal only to
  players who hold it (`collection_medals.rs`).
- Hub: unchanged. Ko-fi calls its webhook for every payment and the hub sums them for
  `GET https://sjk.dfox.app/v1/donations/goal` (the hub's `PROTOCOL.md`,
  "Donations"). Ko-fi has no API to read a goal, so the goal is repeated in the hub's
  configuration, which the hub operator keeps. A Ko-fi donation of more than 1 euro
  with the donor's SJK key id in its message (Settings, Network, SJK identity key,
  Copy key id) gives the Supporter medal ([identity.md](identity.md#medals)) as soon
  as Ko-fi calls; without one, or in another currency, the SJK team matches it by
  hand.

## Changelog

[CHANGELOG.md](../CHANGELOG.md) lists every release, newest first, each change
closed by its credit: who made it (Sol, a contributor) and "after <client>" when it
follows another client's behaviour (EternalJK, JoF EJK). The client builds the
file in and shows it on its changelog page (main menu > Changelog, or the
`changelog` command; [client.md](client.md#changelog-page)), and its tests reject
a change without a credit, a date that is not `dd/mm/yyyy` or a non-ASCII
character. The merge that brings a player-visible change into `main` adds its line
under "Unreleased"; tagging a release renames that section to
`<version> | <dd/mm/yyyy>` and its lines become the release notes' "New since"
list. The Discord changelog posts are made from the same file.

Every release carries a short recap (Sol's rule, 12/10/2026): three to six lines
`> <Heading>: <one sentence>` between the release's title and its changes, in player
words, each heading at most 28 characters and each sentence at most 160, grouped by
what a player notices (servers, gameplay, tools, cards) rather than commit by commit.
Put a command or cvar the player can type in backticks (`` `whatsnew` ``), in a recap
or a change: the card and the changelog page draw it in a chip the player can click.
The client's News panel shows them to a player the first time they open the game
after updating ([sjk-ui.md](sjk-ui.md#news-panel)). Write the recap when the
release is cut, from the "Unreleased" lines, in the same commit that renames the
section; the tests reject a dated release from 12/10/2026 on without one, and the
release notes' introduction may repeat it. A player who skips releases sees the recaps
of each, so a recap must read on its own.

Every release is also announced in SJK chat (Sol's rule, 25/10/2026), small ones too:
an announcement of kind `release` with the release's version, titled "SJK <version>",
its text (the card a click on the line opens, [hub-chat.md](hub-chat.md)) filled in
full from the recap and the notable changes in player words, never a title alone. The
draft is shown to Sol, then posted on the hub's host with
`sjk-hub announce --release <version> "SJK <version>" <file>` once the release's
download is live. Staff tools stay out of it, as they stay out of the changelog.

A release that changes what a setting does, or its default, asks players to choose it
again with a line `= <cvar>: <one sentence>` beside the recap, before the changes
(17/10/2026), in place of a migration marker cvar of its own: the News panel
opens on a "Settings to review" page listing every such setting of the releases the
player has not seen, each with its Settings name, the sentence and a switch
([sjk-ui.md](sjk-ui.md#news-panel)). `= <cvar> reset: ...` also puts the setting back
to its default when the card is offered (once per version, `cl_whatsNewReset`), so a value every profile saved under the old
meaning does not carry over unchosen (`cg_nameplate`, when it became See & share
nameplates). The cvar must be an on/off row in Settings, and the sentence at most 160
characters; the tests reject anything else. Write the line under "Unreleased" with the
change, so it is part of the release it ships in.

## Credits

[credits.txt](../crates/sjk-viewer/assets/credits.txt) feeds the client's credits
page ([client.md](client.md#credits-page)); [CREDITS.md](../CREDITS.md) stays
the full written record. Sol develops SJK alone since October 2026; Bishop is
credited for JKR, SJK's starting point, in the Origins section, last in both (Sol's
decision, 18/10/2026: most of JKR's code has since been replaced). Sol's card shows
the contributors' pull requests Sol reviewed and merged as a "REVIEWED & MERGED"
chip beside the pull requests and commits: the client counts every "SJK #" piece
of the other cards' history, so it needs no hand count.

Credits are kept for everything (Sol's rule, 06/10/2026): every merge into
`main` that brings someone's work updates credits.txt, CREDITS.md and the
changelog line's credit in the same change. A person gets a card in the merge
that brings their first change (Creyon with SJK pull request #2), and their card
gains lines, with the pull request numbers, as they keep contributing; Sol's own
card gains a line when a notable feature lands. Cards carry the person's GitHub
handle (`github:`, opens their profile when clicked) and may carry `link:`
addresses (https only, optionally `Label | https://...`) and `medal:` lines, the
medals the SJK team gave them, by the hub's ids (`early_contributor`;
[identity.md](identity.md#medals)), shown on the card with their pictures and
names. Sections are free: a later "Supporters" section is a `== Supporters`
heading and its cards, with no code. The tests reject a card without a role, an
unknown key, a link that is not https, an unknown medal id, a medal listed twice
(Bug Hunter included: it is held once since 21/10/2026) or a non-ASCII character.

Under each card the page folds that person's whole history: every feature and
pull request merged into `main`, with its commits. It comes from
[credits_history.txt](../crates/sjk-viewer/assets/credits_history.txt), which
[credits_history.py](../scripts/credits_history.py) writes from git and, through
`gh`, GitHub's pull request titles and authors; the file is never edited by
hand. Every merge into `main` regenerates it: merge, run
`python scripts/credits_history.py` in the merged checkout, and amend the merge
commit with the file (`git commit --amend`), so the merge carries its own
history and no extra commit appears in it; `--check` says whether the file is
current. A new contributor goes into the script's `PEOPLE` (their git name and
GitHub login) along with their card; the tests reject a person in the history
without a card. The script's `QUIET` names the topics it leaves out, with every
commit their merge brought: since 25/10/2026, Sol's hand-given Unique blades
(`personal/thousand-cranes`, `personal/hearth`, `personal/hearth-full`), which show
only to their holders ([unlockables.md](unlockables.md#unique-shaders)).

## Debug panel

The `debug_panel` console command lists SJK's changes and how to test them, from
[debug_panel.txt](../crates/sjk-viewer/assets/debug_panel.txt). It is Sol's
personal test list. Update it in the merge that brings a change into SJK's `main`.

Since 10/10/2026 the panel is a full-frame SJK UI page in every menu style, as the
Profile and SJK chat pages are ([sjk-ui.md](sjk-ui.md#sjks-pages)): the way back and
"Test list" at the top left, the tabs To test, Tested and All under them, how many are
tested at the top right; the entries down the left (tick box, title, area), the chosen
one on a band; its details in a reading column on the right (area, title, its tick,
What changed, To test numbered in gold, Note on a gold band), the last tick's outcome
bottom left and the keys bottom right. Up and Down (Page Up, Page Down, Home, End)
choose, Space or Enter (or a click on a tick box or the Space key hint) ticks, Tab and
Shift+Tab change the tab, the wheel scrolls the list wherever the pointer is, Escape
closes. Ticks stay saved by id in `debug_panel_tested.txt`. It still draws over the
command browser when both are open. World shot:
`world_shot::tests::duel6_debug_panel`.
