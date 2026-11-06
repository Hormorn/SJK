# Client

Build instructions are in [development.md](development.md). SJK needs an installed
Jedi Academy `GameData` directory with the retail `base/assets*.pk3` files.

## Launch

The body shield flash uses two shader passes by default (`cg_shieldBrightness 2`,
Settings > Game > Shield hit brightness); the default was four, then one. Profiles
on an old default (`4` or `1`) move to `2` once (`cg_shieldBrightnessDefaultVersion`
2); other strengths are preserved, and any later choice, including `1`, stays saved.
The shield sphere option and damage-event timing are unchanged.

`cg_hitMarker` (Settings > HUD > Damage hitmarker) controls the red
damage-direction indicator near the crosshair when you take damage. It defaults
to `0` (off); `cg_hitMarker 1` enables it immediately and saves the choice.
It does not change damage camera kick, hit sounds or damage-event timing.

Settings > Sound > Hit sounds (`cg_hitsounds`, TaystJK's and JoF EJK's, Lex's request,
03/11/2026; off by default) plays a sound beside you each time the server counts a hit
of yours on another player (`PERS_HITS` rises, as `CG_CheckLocalSounds` reads it) and
another when it falls, a hit on a teammate; the server counts both
(`G_Damage`), so nothing is guessed. 1 to 4 choose the sound: jaPRO's and EJK's
`sound/effects/hitsound.wav` to `hitsound4.wav` (and `hitsoundteam.wav`) when a pack
brings them, else short tones SJK makes in memory, a tick, a blip, two notes and a knock,
and a falling buzz for a teammate
([hit_sounds.rs](../crates/sjk-viewer/src/audio/hit_sounds.rs)); before 03/11/2026 the
option was silent without such a pack. 5 and 6 change the saber's hit sounds instead
(the stock `saberhit.wav` only, or one of the four at random), as in TaystJK. The sounds
follow the effects volume (`s_volume`).
With Settings > HUD > Green on shield hits (`cg_hitMarkerShield`, off by default,
AngeloPapaya's idea, 03/11/2026) the indicator is green instead of red when the hit took
your shield: the armour in the snapshot carrying the damage event is lower than in the one
before (`G_Damage` takes from armour first). A hit that took health only, or came with no
armour left, stays red. It is the indicator of the damage you take: a hit you land shows
nothing on the screen, and the server tells the attacker only the target's health and
armour of the last hit (`PERS_ATTACKEE_ARMOR`, which `cg_hitsounds` listens to beside
`PERS_HITS`), so nothing here colours hits on others
([damage_feedback.rs](../crates/sjk-viewer/src/damage_feedback.rs)).

Settings > Mods > Asset browser or `assetbrowser` opens SJK's installed PK3 browser (also discoverable through F3
in the console's command browser). Select a pack with the arrow keys or wheel;
Enter, Space or a click toggles it. Choices save immediately and apply at
`fs_restart` or the next client start. The page lists configured game directories and supplemental
`EternalJK` packs, including disabled packs, and protects retail
`base/assets0.pk3` through `assets3.pk3` from disabling.

`fs_restart` mounts the installed game files again without restarting SJK
([fs_restart.rs](../crates/sjk-viewer/src/fs_restart.rs)), as the original game's
`FS_Restart` searched its paths again: PK3s added or changed since the start, and the
pack choices above, reach the lists of installed content (the profile's models and
sabers, Create game's maps, the key bindings' pictures) and the scripts `exec` reads
from packs. The mount runs on a worker; the console says how many PK3s it found. The
map on show keeps the files it was built from, as each map load mounts the files afresh
anyway, so the next map has them. `fs_game`, `fs_basegame`, `fs_homepath` and the other
search settings whose help says so still apply at the next start.

The archived `fs_disabledPaks` setting stores a JSON list of portable
`directory/filename.pk3` names. Names are case-insensitive, and identical names
in an optional content home follow the same choice. Disabling skips the entire
installed archive, including supplemental crosshairs, emoji and cosmetics;
enabled packs retain their original search priority. Loose files, built-in
content, hub unlockable packs and server-selected download-cache archives are
outside this installed-pack policy. Files are never renamed or removed.

Server `CS_MODELS` entries that no entity currently uses are not parsed or
uploaded when the map starts. A newly referenced rigid model is prepared on
one background worker and installed into that world's GPU buffers when ready;
until then its draw is absent. Failed models are tried once per world, and a
map change discards outstanding results. Map props, stock weapons/items and
effect dependencies still preload. Player models already load for the current
roster rather than the entire installed model catalogue; this change does not
defer player bodies or reload files edited on disk.

Local weapon and movement event sounds come from prediction or player-state
events. The view client's ordinary snapshot entity is excluded from the
entity-sound path, following OpenJK multiplayer, so a shot received before
local prediction plays once. Remote and temporary entity events retain their
independent latches, including during demo playback and spectating.

Held multiplayer pistols use distinct world models: weapon slot 4 (the blaster
pistol, commonly replaced by a DL-44 pack) uses
`models/weapons2/blaster_pistol/blaster_pistol_w.glm`; slot 16 (the old Bryar)
uses `models/weapons2/briar_pistol/briar_pistol_w.glm`. These match OpenJK
`codemp/game/bg_misc.c` item definitions, independently of the first-person
view models. Mounted replacement packs keep their normal priority.

The client program is `sjk` and the dedicated server `sjk-server` (`.exe` on
Windows). Developer and diagnostic environment variables are named `SJK_*`
(`SJK_TRACE_*`, `SJK_LAMP_*`, `SJK_GPU_*` and the like). On Windows both programs
carry SJK's icon and call themselves "SJK" and "SJK dedicated server" in
their version information; the client also sets the icon on its window (title bar
and taskbar on Windows, the window icon on X11; Wayland has none). See
[assets/branding](../assets/branding/README.md).

Put `sjk` (`sjk.exe` on Windows) inside the installed game's
`GameData` folder, beside `base/`, then launch it to open the main menu. A shortcut
can use any working directory. No path settings are required. Keep
`sjk-server` (`sjk-server.exe` on Windows) beside the client for Create game
and local `devmap`. The server is its own process, so the client's console does not
reach it once it runs; the console's `g_dismember` and `g_forceRegenTime` (the
milliseconds a point of Force takes to come back, 200 by default as in the
reference; servers often run 30 to 50) are handed to it at launch, as are
EternalJK's `g_saberDuelForceRegenTime` (the same pace in a private duel) and
`g_forceDuelForceRegenTime` (in a full-force duel, kept for when SJK has one). As in
EternalJK, a private duel takes its own pace, not `g_forceRegenTime`: a host who sets only
`g_forceRegenTime 50` still refills at 200 ms in private duels. 0 refills every
millisecond; a negative value is not handed over.

From that folder:

```sh
./sjk
```

Join a server directly:

```sh
./sjk --connect 127.0.0.1:29071
```

Without an explicit positional GameData argument, discovery checks these locations
in order and uses the first containing `base/assets0.pk3` and `base/assets3.pk3`:

1. `JKA_GAME_DATA`, if set and nonempty.
2. The executable's directory, then its `GameData` subdirectory.
3. The saved `fs_gameData` setting.
4. The working directory, its `GameData` subdirectory, then its
   `Star Wars Jedi Knight - Jedi Academy/GameData` subdirectory.
5. The existing Linux Steam and `~/Games/Jedi Academy/GameData` locations.

Thus a drop-in installation wins over a saved location from another installation,
while an explicit environment or positional path still overrides it. Invalid
discovery candidates are skipped; an invalid explicit positional path is an error.
Discovery does not change the working directory or move any game data.

For a binary kept separately, `JKA_GAME_DATA=/path/to/GameData ./sjk` opens
the main menu. Positional launch also accepts a map path and optional player-model
directory (`sjk /path/to/GameData maps/mp/ffa3.bsp`); it remains a direct
world/viewer launch, whereas no arguments opens the main menu. Demo playback
retains its explicit GameData argument.
See [launch.rs](../crates/sjk-viewer/src/launch.rs) and
[app_launch.rs](../crates/sjk-viewer/src/app_launch.rs).

The client has a server browser, player and settings screens, in-game menus,
console, HUD, screenshots, demo recording/playback and a Create game flow.
Presence here describes implemented surfaces; validation limits are in
[status.md](status.md).

Create game starts a child `sjk-server`, normally found beside the client.
Set `JKA_DEDICATED` to its executable path if installed elsewhere. The child
lifetime is managed by the client and defaults to local access; see
[local_server.rs](../crates/sjk-viewer/src/local_server.rs). The server mod
chosen on the screen (`ui_hostServerMod`, Base game by default, or JA+) is
handed over as `--set g_mod <id>`, every time, so a `g_mod` the server
archived never picks it; the choices are `local_server::SERVER_MODS` (id and
name), checked by a test against the server's own `server_mods::SERVER_MODS`
([JA+ mode](server.md#ja-mode)). `devmap` always runs the base game.

Map previews (`levelshots/<map>.jpg`, `.tga` or `.png`) keep the resolution they
ship in, so an HD levelshot pack stays sharp in a large preview and on the
classic loading screen. They are decoded with their mip chain on a worker thread
([levelshot.rs](../crates/sjk-viewer/src/menu/levelshot.rs)) and drawn from
their own texture ([ui_renderer/levelshot.rs](../crates/sjk-viewer/src/ui_renderer/levelshot.rs)),
stretched to the 4:3 frame as the stock UI draws them. Images longer than 4096
pixels are reduced to that; decoded previews are cached up to 64 MiB.

## Menu style

`ui_menuStyle` (Settings, GAME tab, "Menu style") picks the layout of the main
and in-game menus: `sjk` (default), the SJK UI; or `classic` (or `1`), which is
close to the retail multiplayer menus in layout and flow without porting their
`.menu` scripts. An unknown value falls back to `sjk`. The retail 640x480 layout
is fitted to the window height and centred.

SJK's own classic pages keep the retail look and add modern help inside it; the
rules are in [Classic+ menus](classic-plus.md).

Before 08/10/2026 SJK's default was `classic`, and `config.cfg` saves every
archived cvar, so every existing profile carries it. The console's start moves a
saved `classic` once to `sjk` and marks the profile (`ui_menuStyleDefaultVersion`
1, archived); a style chosen after that, `classic` included, stays
([console_session.rs](../crates/sjk-viewer/src/console_session.rs)). The style is
also the second row of [First setup](#first-setup), under Styles with the camera
style.

SJK began with a third, native style, `modern` (or `0`): a column of entries over
the live map, with its own browser, settings tabs, in-game menu, scoreboard
(`cg_scoreboardStyle modern`), console (`con_style modern`) and HUD layout
(`cg_hudStyle modern`), and a gate on mp/ffa3 that opened onto the server being
joined. It is gone. The console's start puts a saved `modern` (or `0`) in any of
those settings back to its default (the SJK UI, `auto`, `auto` and `game`) and
drops the settings only it read (`ui_accent`, `con_lineSpacing`, `con_maxLines`,
`con_datetime`, and since 10/10/2026 `ui_menuContrast`) from the profile
(`retire_modern_ui` in
[console_session.rs](../crates/sjk-viewer/src/console_session.rs)). The modern
style's full-screen form look, which some screens kept after it, is gone too
(10/10/2026, Sol's request that no trace of it stay): every screen or page with
no classic+ version of its own has the SJK UI's look in every menu style. That is
Create game and its map picker ([SJK UI](sjk-ui.md#create-game)), the Update and
Import pages, Camera control, the test list, the resolution list (a pop-up card)
and, for a way to settings without a classic+ panel (a main-menu destination, the
classic renderer route, First setup's fallback), the SJK UI's Settings or Key
bindings. The SJK UI's fonts load in every menu style for them.

The SJK UI is SJK's own menus drawn over the live map in SJK's type
([SJK UI](sjk-ui.md)). It has its main page: the emblem in a
turning holo ring with the menu on an arc round it (Play, Character, Settings,
SJK and Quit, the first, fourth and last opening pages of their own on the
ring), and the servers joined last in a column on the right, joined with one
click; and its Settings: every category down a lit rail, the rows with switches,
sliders and segments under sub-headings, the focused setting explained on the
right and the search at the top, the key bindings one of its categories; and
its Character, the character standing on the map's path below the tower beside
the form; What's new, Update and Identity in the same look; and its Servers, the
server browser with where the servers come from and which show on the left, the
sortable list in the middle and the chosen server's map, numbers and players on
the right; and its in-game menu, a compact arc of six entries (Resume, Profile,
Players, Settings, Servers, Leave) over a dark fade on the match's left,
a row of small icons under the emblem (Camera control, What's new, Credits, Report a
bug, SJK chat, and Staff tools for a staff key), and a card of the match (server, map,
mode and limits, score and place, clock, players) on the right with the vote on (Yes,
No), the player's side and its buttons (Join red, Join blue, Spectate) and Call a
vote; and its Profile screen, one row of tabs: Character, Saber, Force, SJK Profile,
Achievements, Medals and Collection (Ctrl+Tab).
Tab moves the keyboard between the list, the row and the card
([sjk-ui.md](sjk-ui.md#in-game-menu)). Every other screen opens
in its classic+ version. A client started in the SJK UI stands on mp/duel6
instead of mp/ffa3.

The classic main menu has the retail pages, entries and order, with SJK's
Settings and SJK in place of retail's Controls and Setup:

- Main: Play and Profile on the left, Settings and SJK on the right, Exit below.
  Exit and Escape ask before quitting.
- Play: Solo Game, Join Server, Create Server, Play Demo and Rules. Solo Game
  and Create Server both open Create game, which hosts a local match with bots.
- Settings is retail's Controls and Setup as one screen of option panels with two
  tabs, KEY BINDINGS and OPTIONS (described below). It opens on OPTIONS.
- SJK lists SJK's own screens in the Play page's layout: Changelog, Credits and
  Update.
- Every sub-page repeats the navigation row (Play, Profile, Settings, SJK) and
  has Back and Exit.
- Profile opens the retail profile pages (`player`, `player2`, `saber`), which
  edit the same drafts as the SJK UI's Character and write them at once:
  - Profile: name, team colour and the head grid (six 64-unit cells per row),
    Custom to character creation, APPLY on to lightsaber creation, Exit. SJK
    sets Custom beside a Force button (retail's in-game `configforce` art) to
    the right of the grid, with the Force profile at a glance under them
    (mastery, side, points left and the holocrons of the powers with a level,
    a pip per level; a Jedi with more powers than fit gets smaller holocrons,
    never a missing one), then JoF EJK's Cosmetics button and what is worn. SJK
    also adds a model search right of Team Color: Enter or a click starts
    typing, and the grid then lists only the models of the team whose
    `model/skin` name (a species' model name) contains every typed word,
    ignoring case, with the count at the field's end; Enter or a click
    elsewhere keeps the search, Escape clears it, and it is cleared when the
    screen opens. The description line names the model under the pointer.
    Retail's Character Model, The Force and Saber bars overhung their boxes by
    two units (the in-game one also stood above its box with its title off
    centre); SJK sets each flush on its box with the title centred, and the
    in-game APPLY on the bottom band it sits on.
  - Character creation: species, skin tint swatches, the Head, Torso and Legs
    lists, Back and APPLY. Entering it from an ordinary character puts on the
    first species, as retail's Custom did.
  - Lightsaber creation: saber type, the hilt list (two for Dual Sabers; in
    every menu style the hilts come in the game's load order, as EternalJK and
    JoF EJK list them: the highest-priority pk3 first, each file's hilts in
    order, not sorted), the
    six blade colour swatches (two rows for Dual), Apply, and Apply back to the
    main menu. SJK adds JoF EJK's custom colour: a red, green and blue slider
    per saber (`ingame_saber.menu`'s "RGB Color Creation"), right of the live
    sabers in the lower box under a "Custom color" heading with a chip of the
    colour (framed white while it is the saber's own), or under each saber's
    swatches in the in-game window. The sliders show the blade as it is drawn,
    a stock colour's own tint included; a click or drag on a bar, the wheel
    over it or Left/Right (steps of 5) makes the colour the saber's own:
    `color1`/`color2` 6 (`SABER_RGB`) and the tint in `cp_sbRGB1`/`cp_sbRGB2`
    (`r | g << 8 | b << 16`), as JoF EJK's `UI_UpdateSaberColor` writes them.
    A swatch brings a stock colour back.
    A changed saber (hilt pick here, `set saber1 ...` or JoF EJK's `saber`
    command in the console or a quick-wheel custom choice) is
    applied at once on JA+ and jaPRO-lineage servers: after the userinfo flush
    the client sends `saber <saber1>` for a single saber or `saber <saber1>
    <saber2>` for a pair, as JoF EJK's `CG_Saber_f` does, at most once a
    second and once per changed selection. Stock and unknown servers read
    `saber1`/`saber2` only at spawn, so there the change waits for the next
    respawn. The server may refuse the command (jaPRO: standing still, FFA
    only, not in a duel) and says so in a `print`; it is not retried by itself, but
    the `saber` command asks again on every run, the same pick too, as `CG_Saber_f`
    does (Kushina's report #132: a refused switch to two sabers left the cvars on the
    pick, so running it again sent nothing).
  - Force: retail's in-game `ingame_playerforce` window, here on both
    frames, editing the same draft as the SJK UI's Force tab. It keeps retail's
    frame, title band, gold mastery line, blue and red side bars and level
    stars, each numbered with what that level costs (`UI_DrawForceStars`:
    `forcestarN` once bought, `forcecircleN` before), and retail's templates
    down the left: the side's `forcecfg/light` or `forcecfg/dark` `.fcf`
    files from the game data (retail's Blademaster, Knight and others, and
    any pack's, under their own capitalised names) and the player's own.
    Retail templates come first under **Jedi Academy**, followed by **Community**;
    saved templates are tagged and sort first within their group. SJK bundles
    JoF EJK #23's 22 community presets (13 light, 9 dark), preserving their names,
    colours and values; installed packs and saved files can override them.
    A click or Left/Right loads a template
    into the draft, legalized at the draft's rank as `UI_ForceConfigHandle`
    does, and the row stays filled until the draft is edited. In the SJK UI,
    the Force page's **Force templates** panel has Jedi Academy and Community
    selectors; Left/Right or clicking either half steps through that group.
    Templates edit the draft; Apply sends it. Name and Save
    Template write the draft to `forcecfg/<side>/<name>.fcf` in the client's
    user folder (beside `config.cfg`). The window is as wide as the in-game
    profile. SJK lays the powers out in two columns, the neutral powers over the saber skills and the chosen
    side's five beside them, each with its holocron (`gfx/mp/f_icon_*`), whole
    whenever the power can be bought (Saber Defend or Throw without Attack
    keeps its holocron at 55% and its stars a readable grey, where retail's
    near-black hid their costs), and
    adds Light and Dark cards with the side emblems, a points meter that shows
    a hovered star's price (red when the points left cannot pay it) and a
    panel for the hovered or focused power: its holocron, level, the next
    level's cost or why it cannot be bought (other side, Saber Attack 1
    needed), every level's cost and what it has taken, or what the server does
    with it. On a server, the names of the powers it turns off, holds at a
    level or does not use are amber, the mastery line says when the rank is
    the server's highest, and the panel with nothing focused says how many
    powers the server turns off (see [Force profile on a
    server](#force-profile-on-a-server)). A click
    on a power raises it a level and the right button lowers it, as retail's
    did; a click on a star sets that level (on the power's top star, one
    below). Left and Right step the focused power. Reset, Discard and Apply
    Powers act on the draft; the points line says "not applied" until it is
    written, and leaving the Force window (Escape, Back, another page, the
    screen closing) applies it, as retail's did on close.
    The main menu shows the window over the backdrop with the navigation row
    and Back.
  - Cosmetics (JoF EJK's `ingame_cosmetics`): the installed hats and
    capes side by side and the live model wearing them in a column on the
    right, the worn row filled.
    A click wears a piece and a second takes it off; the wheel and Left/Right
    move through a list and Enter wears. Show Cosmetics cycles `cg_cosmetics`
    (On, Only Me, Off), Remove All takes both off and Apply returns to the
    profile. An empty list says where cosmetics come from; when JoF's
    catalogue has pieces that are not installed the list ends with its note.
    See [Hats and capes](#hats-and-capes).

  Character creation shows the live model where retail drew its
  `ITEM_TYPE_MODEL`: the player's model, wearing their hat and cape, walking
  in place (`BOTH_WALK1`) while the view turns round it at retail's
  `model_rotation 50` (20 degrees a second), drawn without sabers as retail's
  was; the cosmetics window shows it standing (`BOTH_STAND1`) in a column
  beside the lists, where JoF EJK drew its model. The model's portrait shows
  until the first preview frame is drawn. Lightsaber creation shows the
  sabers alone in the band under its boxes, as retail spun the bare hilt
  model (`isSaber` items, `Item_Model_Paint`): lit, laid on their side and
  turning about their length at retail's `model_rotation 20` (50 degrees a
  second, from `model_angle 180`), each centred on its whole length so a
  staff fills the band as a single saber does, the second of a pair below
  the first; a drawn hilt and blade stand in until the first frame. See
  [model preview](rendering.md#classic-model-preview). The part
  lists show each variant's icon (`models/players/<species>/icon_<part>`,
  `.jpg`, `.png` or `.tga`) as retail did, its name when there is none, and
  the swatches are the species' tint base (`gfx/menus/players/<species>/`
  `*tintbase`) multiplied by each `playerchoice.txt` colour, as the swatch
  shaders draw them (a flat colour without the image). The species being
  edited is loaded into 64 cells of the UI icon atlas of its own (four rows,
  about 4 MiB), again when the species changes. Escape returns to the
  profile page, then to the menu. A click
  on a cell of the head grid, the part, tint and hilt lists or the blade
  swatches picks that cell; each list registers its own pointer region before
  its cells, since the menu canvas gives the pointer to the region registered
  last.

Both browsers ask two master servers at once and list every server either knows,
each once ([discover.rs](../crates/sjk-viewer/src/server_browser/discover.rs)):
`cl_master` (`master.jkhub.org:29060`) and `cl_master2` (`master.ouned.de:29060`,
the second master JKHub recommends; empty asks only the first), both in Settings >
Network. A master that fails is passed over while the other answers. With
`master.jkhub.org` alone, servers that list themselves only with the other master
were missing (Sol's report, 11/10/2026).

Join Server opens retail's join-server screen (`ui/jamp/joinserver.menu`) on
the same browser as the SJK UI's, so the list, favourites, filters and sorting
carry over between styles. Labels and buttons are in capitals:

- GET NEW LIST and REFRESH LIST both fetch the master list again, as retail's
  `RefreshServers` behind both did.
- The selectors box: SOURCE (INTERNET or FAVORITES; Tab also switches),
  FILTER (retail's mod filter row is the client's text filter over names and maps;
  `/` or a click starts typing, Escape ends), TYPE (game-type filter), and the
  VIEW EMPTY, VIEW FULL and VIEW LOCKED toggles (the archived
  `ui_browserShow*` cvars; VIEW LOCKED stands where retail's data rate was).
- The list: SERVER NAME, MAP NAME, PLYRS, TYPE and PING columns over retail's
  row bands and column frames, ten 26-unit rows, the sorted column filled and
  its header white with a ^ or v for the direction. Clicking a header sorts by
  it, again reverses it. TYPE adds JA+, JAPRO or MOD where the client detects the
  server's mod, and `*P` for a locked server; favourites carry a gold `*`.
  The wheel and the scrollbar along the right edge scroll the list.
- The secondary row: CONNECT IP (where retail had NEW FAVORITE; the client's
  direct connect), ADD FAVORITE or DEL. FAVORITE for the selected server, and SERVER
  INFO, a pop-up of the server's published settings and players (Escape or a
  click closes it). PASSWORD and FIND PLAYER are dimmed: the client asks for the
  password when a locked server is joined, in a retail-style prompt, and has
  no player search yet.
- BACK returns to the Play page, EXIT to the quit page (not shown when the
  browser was opened from the game menu), JOIN joins the selected server (a
  double-click on a row too).

Server names, rules and players' names arrive in the game's byte charset, not UTF-8:
the browser reads them byte by byte as the game reads chat (UTF-8 when the bytes are
UTF-8), so `»`, `«`, `§` or `·` show as themselves. Until 11/10/2026 a server whose host
name had such a byte was dropped from the list (its info was refused as not UTF-8), and
players' names showed `?` for them.

The status line under the list shows the fetch progress where retail showed
the refresh time, and the description line shows the hovered item's
description. The screen is in
[menu/classic/browser.rs](../crates/sjk-viewer/src/menu/classic/browser.rs).

Both browsers list what one refresh finds: the master's list, up to 1,000 servers
(256 cut off the end of a long list, whose servers never showed; stock keeps 2,048),
each asked for its info at once, the answers already in read after every 32 requests
so they do not pile up in the socket, and every answer within 0.9 seconds of the last
request listed ([lib.rs](../crates/sjk-network/src/lib.rs) `query_server_infos`). A
server that has not answered by then is asked twice more, 0.6 seconds apart: a lost
packet, or a busy server's `getinfo` rate limit (`SVC_Info`), left it out of that
refresh, so the count changed from one refresh to the next and a favourite came and
went. A dead server's "port unreachable", which Windows reports as a reset on the next
read, no longer ends the refresh: it used to, so on Windows every server that had not
answered by then was missing, a different set each time (the engine ignores it too,
`NET_GetPacket`).

Retail entries SJK has no screen for yet (Play Demo, Rules) are shown dimmed,
and their description line says so; retail's Mods and Defaults are left out of
Settings (Backspace restores one setting's default).

Settings' two tabs sit on the panel's title band, KEY BINDINGS on the left and
OPTIONS on the right; a click switches, and Tab walks through the groups of both
tabs in turn. Each tab keeps its group list down the left and shows the chosen
group's items in the panel beside it, opening on First setup (OPTIONS) and Movement
(KEY BINDINGS). The items are the same settings and key bindings as the tabbed
settings screen's, laid out the retail way (labels set against a column at retail
`textalignx`, the value after them, the open group's entry in white) with
classic+'s own controls, below. A group with more items than the
panel holds scrolls: the wheel over its items moves the list one item per notch
(three on a key-binding group), a thin bar along the panel's right edge shows the
position and can be dragged, and Up and Down keep the selected item in view.

SJK makes these panels [classic+](classic-plus.md): a search field and the items
fill the panel's upper part (up to 10 rows on the main menu and 8 in game, where
retail's taller panel held 15 and 12; larger menu text can show fewer) and a
detail box under them describes the focused item. For a setting
it shows the full name and value, what the setting does
([help.rs](../crates/sjk-viewer/src/settings/help.rs), one or two lines for
every setting), its default, its range (or how many choices it has), "applies
after a restart", "applies after a graphics reload" or "applies on the next map",
and its console name. For a key
binding it shows the keys, the console command, which other actions those keys
also do, and the default key. Row labels drop their bracketed notes and their
"(restart)", "(reload)" or "(next map)", which the detail box says instead; a setting that
applies later has a gold `*` after its label, and a row changed from its default
(a binding off its default key) has a gold dot after its label. Backspace, the
right button or the focused row's gold reset arrow (at the row's right end)
returns the setting to its default (resolution and display mode excepted). The
description line names the keys of the focused row, and the detail box's title
carries the icon of the setting's group (or of a binding's category).

Classic+ draws the items with its own controls on retail's blue and gold
(Sol's request, 07/10/2026; [classic-plus.md](classic-plus.md)), instead of
retail's YES/NO text and `menu/new` slider art:

- Labels are in sentence case ("Menu text size"), in retail's option blue; values
  are brighter. The focused row's label turns white over a soft band with a gold
  bar at its left end. Headings, tabs and group names keep retail's capitals.
- An on/off setting is a switch: a pill, gold with its knob right when on, dark
  with its knob left when off, and On or Off after it. A click on its row, Enter,
  Space, Left or Right flips it.
- A number is a slim rail filled gold up to a round knob (ringed while the row is
  focused), with its value centred in a small frame after it. Clicking or dragging
  along the row sets it, Left and Right step it, typing or Enter edits it exactly.
- A choice of two or three values (menu style, menu contrast, console style,
  scoreboard style) shows them all side by side, the one in use filled gold; a
  click on one sets it, Left and Right step.
- A longer list (the display mode, the resolution, the HUD) shows its value in
  a field with a small caret; a click or Enter opens the list.
  A text setting shows its text in a field, typed in place.
- The longer groups are divided by sub-headings: Game options (items and models,
  effects, camera and prediction), Interface (menus, console), HUD (layout,
  crosshair, names and cards, nameplates, readouts) and First setup (graphics,
  styles, display, aim, sound, HUD, nameplates, Force shells, SJK).

Retail split video and the Force binds over two pages because a page held few
items; classic+ panels scroll and explain the focused item, so SJK shows each as
one group and regroups the settings screen's GAME, HUD, HUD+ and TEXT tabs by subject
([settings/groups.rs](../crates/sjk-viewer/src/settings/groups.rs)):

- OPTIONS lists First setup, Graphics, Sound and Gameplay (Sol's grouping,
  06/10/2026). First setup is the FIRST SETUP tab's rows as a group (see
  [First setup](#first-setup)); Sound is AUDIO. Graphics and Gameplay open pages
  of their own in the same layout, their groups down the left, the OPTIONS tab
  still marked, and Back (or Escape) returning to OPTIONS; in game they are the
  same pop-up.
  - Graphics: Video is the whole VIDEO tab (graphics quality, resolution,
    display, frame rate, field of view, marks, shadows, gamma); Image, Lighting and Shadows are the
    renderer settings' tabs; Weather is the renderer settings' WEATHER tab
    (weather, density, quality, forced weather, ground fog, clouds; see
    [Weather](rendering.md#weather)).
  - Gameplay: Mouse is the CONTROLS settings (retail's Mouse/Joystick, moved
    here from the key bindings). Game Options holds the gameplay rows (simple
    items, forced models, saber and speed trails, aura shell, shader remaps,
    third-person camera style and damping, prediction smoothing). Interface
    gathers the menus' and console's look (menu style, contrast, game fonts, menu
    text size and spacing, console style and text size); HUD the HUD style, files
    and scale, status, weapon bar, crosshair and its size, names, nameplates,
    timer, speedometer, lagometer and its scale, chat and ground readout;
    Scoreboard its style, the SJK one's compact rows, client numbers, head
    icons and small rows. Network
    follows.
- KEY BINDINGS: every binding is in one list, under the headings Movement,
  Interaction, Weapons, Force powers (retail's two Force pages as one) and Other.
  The categories down the left jump to their heading, and the one holding the
  selected action is marked as the list scrolls. The weapon rows name their
  weapon (Saber / melee, Blaster pistol, ... Explosives) rather than `weapon N`;
  Melee (fists), unbound until a key is picked, is `weapmelee`
  ([Useful console commands](#useful-console-commands)).

Every group down the left (both tabs, Graphics and Gameplay, main menu and in-game
pop-up) has an icon at the left end of its row, opposite the label: SJK's own grey
metal discs in the quick wheels' style (holocron faces from 13/10/2026 until Sol
brought the discs back on 15/10/2026), generated by Sol
([assets/settings](../crates/sjk-viewer/assets/settings/README.md),
`Entry::icon`). The open or hovered group's icon is at full strength, the others
dimmed.

The search field on the panel's first row finds across the whole tab. `/`, Up
from the first row, or a click gives it the keyboard; typing filters as it goes,
with the number found at the row's end; Enter, Down or Tab go to the results and
Escape clears the text, then leaves the field (an Escape in the panel clears a
search before it leaves). On OPTIONS it searches every option of every group,
the renderer's included, by name, console name, description and group
([settings/search.rs](../crates/sjk-viewer/src/settings/search.rs)), and shows
the results under their groups' headings, the detail box adding "In <group>";
opening a group clears the search. On KEY BINDINGS it finds actions by name,
console command, category or bound key (`space` finds Jump).

The two tabs' searches reach each other: while text is typed, the description line
says how many entries of the other tab match it ("3 key bindings match too: click the
KEY BINDINGS tab"), and a search with no result here names them in the list. Clicking
the other tab (or Tab, or `[` and `]`, across the tab boundary) carries the search text
to it, so one query walks every setting and binding
([menu/classic.rs](../crates/sjk-viewer/src/menu/classic.rs), `sync_cross_search`).

A list (a choice of more than three values, the display mode) does not change
on a click or Enter: they open a dropdown under the value, a rounded list over a
soft shadow with the value in use in gold and marked with a dot, and nothing
changes until a choice is applied with Enter or a click. Up and Down move in it;
Escape, Backspace or a click elsewhere close it unchanged. A list of more than
eight choices (a PC's installed fonts run to hundreds) shows eight at a time with
a thumb on its edge: the wheel scrolls it, Page Up, Page Down, Home and End jump,
and the highlight stays in view; only the shown choices take click tokens, so a
long list never reaches other controls' tokens. Left and Right still
step the value directly. The rows the dropdown covers leave their values out
while it is open, since text draws over every shape.

Key bindings carry the retail HUD picture of what they select or use
([keybind_editor/icons.rs](../crates/sjk-viewer/src/keybind_editor/icons.rs)):
each weapon's `gfx/hud/w_icon_*`, each Force power's holocron
(`gfx/mp/f_icon_*`), the items' `i_icon_*` (bacta, seeker, sentry, force field,
binoculars), the saber toggle and style, and the team menu's flag. They sit in
a column between the labels and the keys and, larger, in the detail box. They
are read from the player's game data on a worker thread when the key bindings
first open, into two atlas rows of their own; a picture the game data lacks is
left out.

Up and Down move through the items, Left and Right change a value, Enter flips a
switch or opens a list's dropdown, and typing or Enter on a number edits it exactly; clicking
a slider sets it. Tab moves to the next group (on KEY BINDINGS, Left and Right
do too). A key binding reads "A OR B" (retail's `KEYBIND_OR`,
raised to capitals with the key names as retail's `BindingFromName` does) or `???`
when unbound; Enter or a click waits for the new key, shown in red with
retail's "Enter new key, or ESC to cancel, BACKSPACE to clear.", and Backspace
clears every key of the action. Escape closes the page to the main page (the
Graphics and Gameplay pages to OPTIONS).

The classic in-game bar's Settings opens the same panels as retail's
`ingame_setup` and `ingame_controls` pop-ups, with the two tabs on the pop-up's
title band: a box under the bar with the group list and panel at their in-game
positions and no navigation row, closing back to the bar.

The classic in-game menu (Escape during a match) is the retail top bar: About,
Join, Profile, Add Bot, Settings (retail's Controls and Setup), Vote, Call Vote
and Exit, after SJK's own narrower SJK button at its left end. Each opens a pop-up under it or the matching screen. SJK's pop-up holds
SJK's own screens, for now the [changelog page](#changelog-page)
([ingame_menu/sjk.rs](../crates/sjk-viewer/src/ingame_menu/sjk.rs) lists them);
the SJK UI's in-game menu has them in its row of icons instead
([sjk-ui.md](sjk-ui.md#in-game-menu)), its Join and Vote on the match card.
About shows the server info. Join picks
a team, or opens the class page in Siege (the team first, then that team's
classes, as in the SJK UI); in a team game Team Red and Team Blue
carry their flag (`gfx/hud/mpi_rflag`, `mpi_bflag`) and the team's player count. Vote is Yes/No. Call Vote opens the
call-vote lists. Its map list is the server's own when its serverinfo carries one
(`sv_maplist`), else, as the stock in-game vote screen (`loadArenas`,
`UI_MapCountByGameType`), the installed maps the arena files (`scripts/arenas.txt`,
`scripts/*.arena`, from every mounted pk3) allow for the server's `g_gametype`, team
FFA listing the FFA maps and Capture the Ysalamiri the CTF ones; each row shows the
arena's long name and the map, the retail maps first, then the community's, each by
name. Only when no arena names an installed map does it list every installed
`maps/mp` map. Until 26/10/2026 it listed every `.bsp` in path order, cut at 128, so
with JoF's content the community maps at `maps/*.bsp` (and single-player levels) filled
it before the retail `maps/mp` ones (Kushina's report #136;
[callvote.rs](../crates/sjk-viewer/src/ingame_menu/callvote.rs)). Exit offers Main Menu, Restart Match and Quit Program, each
with a Yes/No confirmation. Profile opens the retail in-game profile window
(`ingame_player`: name, team colour, head grid, Custom, Saber and the Force
summary, then `ingame_player2` and `ingame_saber`); its Apply returns to the
match. The Force box shows the side's emblem beside retail's mastery, side and
points lines and the known powers' holocrons, and its `configforce` button
opens the Force window; the Cosmetics button sits under Custom, as in JoF EJK.
Its Join Red, Join Blue and Spectate buttons are left to the Join tab. Settings opens
the option panels described above. Siege swaps in Objectives and V Chat as retail does. Add Bot,
Objectives, V Chat and Restart Match are dimmed with a note, because the client
cannot add bots or restart a match it does not host. Left and Right move along
the bar; Escape closes a pop-up, then the menu. The Server browser entry and the
[Camera control](#camera-control) icon, which retail did not have, are in the
SJK UI only (F8 and `cameracontrol` open Camera control in either style).

With the player's retail game data mounted, the classic menus draw its own
artwork: the backdrop, side glyph columns, ring, windows, logo, sub-page frames,
button glow, list glow (`menu_buttonback2`), slider bar and thumb
(`menu/new`), in-game bar and pop-up boxes from `gfx/menus`. The art is decoded
once on a worker thread the first time the classic style is used, and the UI
renderer uploads it into one texture per image, separate from the shared UI icon
atlas. Its bind group changes only between draw runs that need a different
texture, so layer order is kept. Additively blended retail images (glow, title
band, bar) are converted to alpha at decode time. A missing image falls back to
the client's own shapes. Retail assets are never bundled.

The art moves as retail's shaders move it (`shaders/ui.shader`); the `.menu`
scripts themselves only swap pages at once and show or hide the glows. Retail's
main page plays `video/ja01` (`gfx/menus/videologo`) in its ring; SJK shows its
own emblem there instead and does not read the video: the gold
starburst with the JK blade, centred in the centre window's opening and drawn
over the frames, 176 of the 640x480 canvas's units across (about 400 pixels at
1080 lines, 790 at 2160), with or without the retail art. Its orange core and
ring breathe (glow strength 0.08 to 0.45 every 4.2 seconds) and the blade's cyan
lights shimmer (0.30 ± 0.15 from two sines of 0.9 and 0.37 seconds), drawn as
additive glow layers over the still emblem. The emblem is bundled
([assets/branding](../assets/branding/README.md), drawn by
[menu/emblem.rs](../crates/sjk-viewer/src/menu/emblem.rs)) and decoded with its
mip chain on a worker thread when the first menu shows. The SJK UI's main page
shows it too, in its turning holo ring ([SJK UI](sjk-ui.md)). The ring
turns 5 degrees a second (`tcMod rotate 5`), the side glyph columns climb over
their `menu_side_text_b` backdrop (`tcMod scroll 0 0.025`), and a quarter of
`env_logo` drifts through the logo's translucent letters between an opaque and a
blended pass of the logo, as its three shader stages do. The button and list
glows, the title band and the in-game bar flicker: retail multiplies the screen
under them by four scrolling layers of `gfx/hud/static_menu`, which the renderer
reproduces by recomposing those small images with the noise on the CPU each frame
they are drawn, over the piece alone because the UI blends with alpha. The
motion clock and curves, the emblem's included, are in
[motion.rs](../crates/sjk-viewer/src/menu/art/motion.rs). The focused entry's
text pulses between white and 80% of it, as `Item_TextColor` does
(`PULSE_DIVISOR` 75 ms); the open group's or page's entry stays steady white.
Labels, buttons, titles and option values are in retail's capitals; descriptions,
typed text, vote-list names and the about values keep their case.

Outside a match the classic style draws no world. The main pages are opaque
over the retail background (the main page's emblem fills the centre gap,
the sub-pages' gap stays dark), and the screens they open (the tabbed settings,
key bindings, Player) get the retail backdrop beneath them; the
classic server browser draws its own. The frame
then clears instead of rendering the map, its secondary views and flares; the
boot map is still loaded, because the menu world is what joins build on, and
switching to the SJK UI shows it again. Create game is the SJK UI's screen in
this style too, over the map. Not loading it at all in the classic
style is a possible follow-up. Over a live match the in-game menu and the
screens it opens leave the game visible, as retail's do.

Joins and server map changes show retail's loading screens. Until the gamestate
arrives it is the connect screen
(`ui/jamp/connect.menu`, `UI_DrawConnectScreen`): `menu/art/unknownmap_mp`,
"Connecting to <address>" (or "Starting up..." when the client hosts the game)
and "Awaiting connection...", "Awaiting challenge..." or "Awaiting
gamestate...", following the join worker's phases. Then it is cgame's
information screen (`CG_DrawInformation`, `CG_LoadBar`): the map's
`levelshots/<map>` over the window (cropped top and bottom on a wide one, the
unknown-map art without a levelshot), "Loading... <what>" or "Awaiting
snapshot...", and the server's lines in retail order: host name, Pure Server,
message of the day, game name, the map's long name, cheats, game type, limits,
force rules and the game type's rules, worded from the player's `MP_INGAME`
strings. The LED bar along the bottom (`gfx/hud/mp_levelload`, `load_tick`,
`load_tick_cap`) has retail's nine ticks; the client lights them from its own load
(gamestate, map parse, world build, world ready, session) rather than cgame's
registration steps. Colour codes in the host name are dropped. The destination
world is adopted only once it is built from the session's own gamestate with the
session in hand, and the player never walks a
preview world: the screen stays until the map is live. Escape or a click
cancels, as before. A failed join shows the connect screen with the reason; a
retail-style error page (`error.menu`) is not drawn yet. The loading screen is
in [loading.rs](../crates/sjk-viewer/src/menu/classic/loading.rs).

The profile pages are in
[player_menu/classic.rs](../crates/sjk-viewer/src/player_menu/classic.rs), with
entries and retail geometry in
[layout.rs](../crates/sjk-viewer/src/player_menu/classic/layout.rs), drawing in
[view.rs](../crates/sjk-viewer/src/player_menu/classic/view.rs) (the Force page
in [force_page.rs](../crates/sjk-viewer/src/player_menu/classic/force_page.rs),
the cosmetics window in
[cosmetics_page.rs](../crates/sjk-viewer/src/player_menu/classic/cosmetics_page.rs))
and pointer routing in
[pointer.rs](../crates/sjk-viewer/src/player_menu/classic/pointer.rs).
The main menu code is in [menu/classic.rs](../crates/sjk-viewer/src/menu/classic.rs): the
page tables are in [pages.rs](../crates/sjk-viewer/src/menu/classic/pages.rs),
types and geometry in [layout.rs](../crates/sjk-viewer/src/menu/classic/layout.rs)
and drawing in [view.rs](../crates/sjk-viewer/src/menu/classic/view.rs). The
option panels' frame is [panel.rs](../crates/sjk-viewer/src/menu/classic/panel.rs);
their items are drawn by
[settings/classic_view.rs](../crates/sjk-viewer/src/settings/classic_view.rs) and
[keybind_editor/classic_view.rs](../crates/sjk-viewer/src/keybind_editor/classic_view.rs).
The
in-game version is in
[ingame_menu/classic.rs](../crates/sjk-viewer/src/ingame_menu/classic.rs), with
[classic_view.rs](../crates/sjk-viewer/src/ingame_menu/classic_view.rs) and
[classic_actions.rs](../crates/sjk-viewer/src/ingame_menu/classic_actions.rs).
The artwork is loaded in [menu/art.rs](../crates/sjk-viewer/src/menu/art.rs) and
bound in [ui_renderer/art.rs](../crates/sjk-viewer/src/ui_renderer/art.rs). The
style is read in [style.rs](../crates/sjk-viewer/src/menu/style.rs). Shared exits
are in [destination.rs](../crates/sjk-viewer/src/menu/destination.rs).

Planned follow-ups, each a new page or screen module, following the retail
`ui/jamp` menus:

- Classic versions of screens the classic pages open in the SJK UI's look, or
  do not have yet: Join Server's `findplayer` and `createfavorite` pop-ups, Create
  Server (`createserver`, `advancedcreateserver`),
  Solo Game (`quickgame`).
- Retail option items SJK has no setting for (video quality presets, colour
  depth, geometric and texture detail, EAX, languages) are left out of the
  panels, and the video restart confirmation is not needed.
- On the profile pages: portraits for every model (the atlas holds 207, so
  species after the characters show none).
- The screens with no SJK equivalent yet: Play Demo (`demo`), Rules
  (`rules*`), Mods, Defaults, Add Bot (`ingame_addbot`), Siege objectives and
  voice chat, and the error page (`error`).
- The retail fonts (`ui_gameFont`, a separate change), and the main page's
  hover captions (`*_undertext`, drawn in the `aurabesh` font).

The player screen's Character and Saber pages write their cvars as soon as a
value changes. The Force page edits a draft instead: Apply writes `forcepowers`
once, in the stock format and legalized as before, and Discard returns to the
applied profile. Leaving the Force page, however it is left (another page or
tab, Back, Escape, the screen closing for the game), applies it as retail's
Force menu did on close: the draft when it changed, else the profile
`forcepowers` already holds, so a server is told to read it (`forcechanged`)
either way, unless Apply sent it during that visit
(`PlayerMenu::settle_force_page`, run once a frame). The unchanged profile is
sent as the player wrote it, not as the page fitted it to a server's lower
rank. Before 12/10/2026 leaving dropped an unapplied draft, and players who
edited and closed the screen kept their old powers. While a draft is pending,
the page says it is not applied yet (the SJK UI's points line, the classic Force
window's status). Power icons (`gfx/mp/f_icon_*`) and side emblems
(`gfx/hud/mpi_jlight`, `gfx/hud/mpi_dklight`) come from the installed game data;
without them the page shows text only. They take icon-atlas cells of their own
after the HUD's, so the character grid keeps all 207 of its icon cells. See
[force.rs](../crates/sjk-viewer/src/player_menu/force.rs).

The Character page's grid (and the classic head grid) can list far more models
than those 207 cells: an installation with community packs lists over 900. The
cells are therefore a cache ([model_icons.rs](../crates/sjk-viewer/src/player_menu/model_icons.rs)):
a tile asks for its icon when it is drawn, a worker thread decodes it (without
the shared image cache, so the whole catalogue is never held at full size) into
a free cell or the one drawn least recently, and tiles on screen keep theirs.
Before SJK did this, every model past the 207th was a black tile. A model whose
icon file cannot be decoded shows its name in the tile instead. Every world (the
menu map, each server map) has a UI renderer and icon atlas of its own while the
menu moves between them, so when the menu meets a renderer it has not uploaded
into (`ShapeRenderer::id`), its image caches (model, Force, part and key-binding
icons, the map and HUD previews) forget what they uploaded and load it again
(`ClientMenu::follow_renderer` in [menu.rs](../crates/sjk-viewer/src/menu.rs)).
Before that, tiles cached in an earlier world drew black after joining, changing
map or disconnecting. Tiles answer to
the pointer by their place on screen, so a long list never runs into other
controls' pointer tokens. The Search row under Team colour (Enter to type)
filters the grid as the classic profile's search does (see above).

### Force profile on a server

`forcepowers` is the player's own profile. On a server, every userinfo sends it
fitted to the server's rules from `CS_SERVERINFO` (`g_maxForceRank`, free saber
skills when every other weapon is off, the game type), as stock
`BG_LegalizedForcePowers` would fit it. Whether saber skills are free is then
taken from the server's own `EV_SET_FREE_SABER` event, which stock cgame keeps in
`ui_freeSaber` for the Force menu, once one arrives, and kept through the same
server's map changes (a disconnect or another address forgets it):
`CS_SERVERINFO` may lack `g_weaponDisable` (keys past `MAX_INFO_STRING` are
dropped). Until either tells, a profile that only fits with free saber skills is
sent fitted with them, so the first life is not cut by a guess; stock sends the
profile unfitted and the server fits it. An event that alters the profile sent
has it sent again with `forcechanged`. The fitting keeps team
powers and the powers `g_forcePowerDisable` turns off: the server drops unavailable powers from its own
copy without calling the profile illegal, so sending them never parks the player in
spectator, and a mod that gives them back in a full Force duel finds them
(jaPRO lets the powers its `g_forcePowerDisableFFA` holds back work in duels).
Apply on a server sends the new profile and then `forcechanged`, as stock
`UI_UpdateClientForcePowers` does: the server reads it at once while you
spectate and at your next respawn in play (it prints that it will). A
`forcepowers` typed in the console or set by a config on a server is sent fitted
the same way and followed by `forcechanged` too, and the console says so (or why
nothing was sent: the server's rules are not known yet, or a Force-profile reply
is already waiting and covers it); a change made before joining is sent by the
join. Before this a console change reached the server's copy only at the next
join or menu Apply. When a server still parks you in spectator over your profile
(`nfr <rank> 1 <team>`, which a mod can send for its own reasons), SJK answers
as the stock Force menu would, `forcechanged` with the team you last chose from
the game menu, and if you are still spectating it asks for that team again up
to three times, 5.5 seconds apart (a server refuses a second team change within
five seconds), then says to use the join menu. Not in duel or power duel, where
waiting as a spectator is the queue.

Opened on a server, the Force page takes the server's highest rank and free
saber skills, which decide whether the server keeps the profile it is sent. What
the server only will not let be used stays buyable and is marked: its disabled
powers ("Off on this server"; Jump "fixed" at 1 and Saber Offense and Defense
at 3, as the server holds them), and team powers outside team games ("Server
decides", since modified servers can grant them there). They cost points like
any other, so the profile still fits where they work. Team powers are kept in
the sent profile even in FFA, so a server
that permits them can read the purchased levels; stock servers still remove
them in non-team games without treating the profile as illegal. Team Heal
remains light-side and Team Energize dark-side. The SJK UI's Force page lists
the server's rules under the side's powers (This server: the highest rank and
its points, free saber skills, the
powers it turns off or, past eight, how many, and the server-dependent team-power
notice); off a server it says a server's rules show there once you join, and team powers can
be bought. A mod's own limits that are not in `CS_SERVERINFO` (jaPRO's
`g_forcePowerDisableFFA`) cannot be shown. The rules are in
[force_profile_negotiation.rs](../crates/sjk-client/src/force_profile_negotiation.rs)
and the page's handling in
[force.rs](../crates/sjk-viewer/src/player_menu/force.rs).

## Player models

Player model eligibility follows JoF EJK #126: `notInMP 1` (any nonzero integer)
in `models/players/<model>/settings.txt`, or a model named in any visible
`ext_data/modelblacklist/*.txt`, excludes it from both profile pickers and
replaces it with Kyle when a player would wear it. This includes your model
and forced ally/enemy models. Fallback precedes team-skin selection, so team
players still wear the appropriate red or blue Kyle. NPCs load the same
restricted model normally.
Same-named files follow ordinary VFS priority; distinct blacklist files merge.
Unreadable, empty or oversized optional files are ignored (settings below
4096 bytes, lists below 16384 bytes). Metadata is cached for the mounted world;
remount/restart after changing pack files.

Settings > Gameplay > Items and models in either menu style includes
**Player model blacklist** (`cg_modelBlacklist`, empty by default): names
separated by spaces, commas or semicolons, case-insensitive; a `/skin` suffix
matches the whole model. **Respect model metadata** (`cg_modelEligibility`, on)
can be switched off to ignore pack-authored restrictions; your own blacklist
still applies. Both preferences are archived and apply to player presentation
and the next profile catalogue, without rewriting your saved model or changing
server gameplay. Retail choices remain first under Jedi Academy, with their
filtered count, and Community follows.

A player's or NPC's appearance (`models/players/<model>/<skin>`) loads its
`model.glm`, the skeleton the mesh names (`<name>.gla`), that skeleton's
`animation.cfg` and a skin; if the model cannot be loaded or built, the client
draws Kyle for that player instead
([player_assets.rs](../crates/sjk-viewer/src/player_assets.rs),
[actor_load.rs](../crates/sjk-viewer/src/actor_load.rs)). A folder holding
`sjk_rig.cfg` is a [rig model](rig-models.md): a glTF with its own skeleton, drawn in
place of a stock driver model whose skeleton still carries the animations. That includes a model
changed during the match (`/model`, or a new player in a used slot), as
`CG_LoadClientInfo` falls back to `DEFAULT_MODEL`: the player is not left in
their previous model, and the log says `client N Kyle in place of <model>`. A
model that failed is not loaded again until the next map
([clientinfo_refresh.rs](../crates/sjk-viewer/src/clientinfo_refresh.rs)). A model
that changes during the match (a join, `/model`, a forced-model or mute change) is
read on a worker, at most three at once: its files, its mesh and its textures
decoded there, then uploaded by the render thread, one model a frame
([player_loads.rs](../crates/sjk-viewer/src/player_loads.rs)); until then the player
keeps the model they had, and a new player shows the stand-in actor. Loaded inside
one frame before 10/10/2026, it was a stutter players felt on every join. Files are read as
rd-vanilla and the retail cgame read them, so a model EternalJK draws and
animates is not swapped for Kyle:

- Skins are read with rd-vanilla's `CommaParse` loop (`RE_RegisterIndividualSkin`,
  `tr_skin.cpp`): tokens in surface/shader pairs, comments, missing commas, stray
  text and bytes outside UTF-8 change nothing about what loads, `tag_` entries
  are skipped, `_off` is stripped from surface names and a skin keeps at most 128
  entries ([skin.rs](../crates/sjk-model/src/skin.rs)). A surface named twice takes
  its shader from its first entry (`RenderSurfaces` stops at the first match) but is
  shown or hidden by its last: `G2_SetSurfaceOnOffFromSkin` (`G2_surfaces.cpp`)
  applies every entry in turn, `*off` switching the surface off and any other shader
  back on (never an `_off` cap, which the model keeps off). Until 26/10/2026 the
  first entry decided both, so skins that name every part with its texture and then
  switch an outfit's spare parts off further down (Yuna's and Kushina's) showed all
  their outfits at once, and a cap switched off and then on again was missing
  (Rollembones' report, #148).
- A skin never costs the model (`CG_RegisterClientModelname`, `cg_players.c`):
  when the requested skin is missing, has a missing part or names no surface,
  the model wears `model_default.skin`, and without one its surfaces' own
  shaders. A name is a three-part skin only when it has `|` and says `head`,
  `torso` and `lower`; the console notes each fallback
  ([player_skin.rs](../crates/sjk-viewer/src/player_skin.rs)).
- An NPC's skin, the one appended to its model string, follows
  `CG_HandleAppendedSkin` (`cg_players.c:8519-8566`) instead: any `|` makes it three
  parts, and a part no file holds keeps the parts read before it, as the renderer's
  skin cache gives every NPC after the first (`RE_RegisterSkin`, `tr_skin.cpp`). JoF
  Shuttle's alien younglings name `head_a3|torso_a3|` (no lower part) and have no
  `model_default.skin`, so they drew grey with their GLM's `[nomaterial]` shaders
  until 02/11/2026 (Kushina's report).
- A surface is drawn only while all its GLM flags are clear (`RenderSurfaces`,
  rd-vanilla `tr_ghoul2.cpp`): a tag (`*r_hand`, `*hip_l`, `*head_front`: the bolt
  flag) is never drawn, whatever the skin names for it. Community models often give
  their tags the shader `[nomaterial]`, which drew as small magenta arrows on the
  chest, the belt and the face until 15/10/2026 (Nib's report)
  ([lib.rs](../crates/sjk-model/src/lib.rs) `skin_with_matrices`).
- One leading slash on the mesh's skeleton name is dropped, as the filesystem
  drops it (`FS_FOpenFileRead`); vertex weights adding up past one are used as
  written (`G2_GetVertBoneWeight`); a mesh whose header bone count differs from
  its skeleton's loads when its bone references name skeleton bones.
- `animation.cfg` lines that name no animation, and sequences with no frames,
  are left out as `BG_ParseAnimationFile` leaves them; a table that then names
  nothing holds frame 0, as `G2_TransformBone` does.

Still drawn as Kyle: a model whose `model.glm` or skeleton is not installed or is
not version 6, a mesh referencing a bone past its skeleton, an `animation.cfg`
line naming an animation with other than five fields, and a model whose standing
animation lies past its skeleton's frames (rd-vanilla clamps such frames to 0).
SJK is more lenient than retail in two places: it keeps non-humanoid skeletons
and models without the hand, head or lumbar bolts the retail cgame checks for a
player, and a surface a skin does not name draws the mesh's own shader where
rd-vanilla draws its default shader.

SJK mounts `base` and, when set, `fs_basegame` and `fs_game`. EternalJK mounts its
own `EternalJK` folder by default (`fs_basegame EternalJK`), so skins shipped
there, such as `jedi/model_rgb.skin` in `japro-assets.pk3`, exist for EternalJK
and fall back to the default skin in SJK. Setting `fs_basegame EternalJK` and
restarting mounts it in SJK as well, together with that folder's menu, HUD and
string files.

The ignored test `player_model_scan` loads every installed model and skin through
this path without opening a window and compares each with what rd-vanilla and the
retail cgame would do:

```sh
JKA_GAME_DATA="/path/to/GameData" cargo test --release -p sjk-viewer \
    player_model_scan -- --ignored --nocapture
```

`JKA_MODEL_SCAN_GAMES=EternalJK` (comma-separated) mounts further folders above
`base` and reports only the models with files there. The table goes to
`target/parity-reports/player-models/`; see
[player_model_scan.rs](../crates/sjk-viewer/src/player_model_scan.rs).

## Hats and capes

SJK wears JoF EJK's free-choice cosmetics
([cosmetics.rs](../crates/sjk-viewer/src/cosmetics.rs)): every player wears
whichever hat and cape they like. A hat is any `.md3`
in `models/cosmetics/hats/`, a cape any in `models/cosmetics/capes/`
(`models/players/hats/` and `capes/` when the new folders are empty, as older
packs used them); names of JoF's catalogue are found in either. Seven of JoF's hats
(pumpkin, cap, cringe, fedora, santahat, sombrero, tophat) moved to the new folder but
their models still name `models/players/hats/<name>` for their surfaces; JoF EJK finds
those in jaPRO's `japro-assets.pk3`, which holds its own copies of their pictures and
`japro_hats.shader`. SJK takes those files alone from the EternalJK folder, as it takes
the crosshairs, and where that pack is not installed copies the same picture from the
new folder under the old name (`cosmetics::old_folder_pictures`), so they draw as in
JoF EJK (hub report #192, 28/10/2026: they drew grey; JoF's `jof_hats.shader` in the
new folder, which no model names, is left unused as there: its pumpkin glows). Hats
built inside out (JoF's McFox and McSnowFox, most of their triangles wound away from
their middle) are drawn two-sided, their picture lit as an unscripted surface is
(`cosmetics::inside_out_shaders`), as JoF EJK shows them solid; their file sorts before
every other script, so a script defining the same name keeps its own culling. Each mount
lists and parses the hats' models once for both (`cosmetics::piece_models`). A name is at
most 13 letters, digits, `_` or `-` and does not start with a digit. JoF's
launcher installs its pack (`zzz_jof_cosmetics.pk3`) in the `EternalJK` folder:
SJK mounts every PK3 there that has models in `models/cosmetics/hats/` or
`capes/`, or JoF's client pictures in `gfx/jof/` (`jofclient-assets.pk3`, which
also holds the sounds JoF servers play, see [Force wheel](#force-wheel)),
below all other content, without mounting the rest of that folder
(its menus, HUD and strings come only with `fs_basegame EternalJK`, see
[Player models](#player-models)); the log names each pack. A pack in `base`
works too ([asset_search_paths.rs](../crates/sjk-viewer/src/asset_search_paths.rs)).

What a player wears travels in the saber colour keys, as JoF EJK sends it:
`color1 "4santahat"` is blue blade 4 wearing the hat `santahat`, and `color2`
carries the cape. SJK's `color1`/`color2` are therefore text cvars read with
`atoi`; the profile writes them as `<colour><name>`, the saber page keeps the
name when it changes the colour, and the userinfo carries the name after the
digits only when it follows the rule above
([jof_cosmetics.rs](../crates/sjk-client/src/jof_cosmetics.rs)). Servers copy
the keys to the `c1`/`c2` clientinfo untouched (cut to 15 bytes), and other
clients' `atoi` reads only the colour. SJK reads another player's blade
colour with `atoi` too: before, `c1 "8santahat"` failed to parse and drew blue.

Each player's pieces are resolved when their actor is built or their
clientinfo changes: the names, the models (loaded mid-match like a
configstring model, on first use) and the fitting offset from
`settings/cosmetics/<hats|capes>/<name>.cosmetic`, JoF's and TaystJK's JSON of
per-model and per-skin `xOffset`/`yOffset`/`zOffset` (exact keys, else the
longest `prefix*` key; the skin's entry wins, the model's applies with
`"modelFallback": true`). Each evaluated pose then reads the `*head_top` and
`*back` bolts of the worn slots only. A piece is drawn with the body as
`CG_DrawCosmeticOnPlayer` places it: the bolt's axes, two units down its up
axis, plus the offset along the world axes, never on the dead, the
mind-tricked, monster-held or scaled model, and on the local first-person
player only in mirrors and portals. Cloaked players' pieces are hidden unless
the viewer has active Sight; your own cloaked pieces stay hidden even with
Sight, after JoF EJK #172. The held flag alone suppresses them, even when the
holder's attachment is unavailable. A piece this client does not have is not drawn.
`cg_cosmetics` (archived, 1) draws everyone's (1), only yours (2) or none (0).

`cosmetics hats` and `cosmetics capes` list the installed pieces, with a
number or a name they wear one (the same again takes it off), `cosmetics
clear` takes both off and `cosmetics visibility [off|on|onlyme]` shows or sets
`cg_cosmetics` ([command.rs](../crates/sjk-viewer/src/cosmetics/command.rs)).
A colour number typed alone (`color1 2` in the console, a bind or a script, `set` and
`seta` too) keeps the worn piece after it, as JoF's own colour commands do; a value
with a name after its digits (`2tophat`, `4none`) is set as it is
([typed_colour.rs](../crates/sjk-viewer/src/cosmetics/typed_colour.rs)). Each
write uses the piece worn at that command's position in the buffer: `color1
3newhat; color1 2` ends at `2newhat`; `color1 2; color1 4none` ends at `4none`.
Queries and `vstr` expansion see earlier writes immediately, including across
`wait` boundaries; normal userinfo notifications and `seta`/`setu` flags apply.
JoF's
`saberColor <r> <g> <b>` (or `<blade> <r> <g> <b>`, or six values for the two blades)
makes the blades custom RGB, colour 6 with `cp_sbRGB1`/`cp_sbRGB2`, keeping the pieces
too ([saber_colour_command.rs](../crates/sjk-viewer/src/saber_colour_command.rs)).
JoF's `saber <saber1>` (one saber or a staff, `saber2` becomes `none`) and
`saber <saber1> <saber2>` (two sabers) are the client's own too (`CG_Saber_f`): they
write `saber1`/`saber2`, which Profile > Saber shows and every userinfo sends, and
JA+ and jaPRO-lineage servers get their `saber` command from that change, as from the
menu; with no name it prints the sabers held, and an unknown hilt changes neither
([saber_hilt_command.rs](../crates/sjk-viewer/src/saber_hilt_command.rs),
[useful console commands](#useful-console-commands)). Sent to the
server as is, it swapped the saber in play but left the cvars on the menu's pick, which
the next map brought back (Kushina's report #126). Every run sends the server's
command, the same pick too (#132).
The classic profile's Cosmetics window does the same, and the SJK UI's
Character page has Hat and Cape rows (None, then each installed piece; Enter or a
click opens a picker of named chips, [sjk-ui.md](sjk-ui.md#character)). The menu stage
model wears what `color1` and `color2`
name, placed on the bolts of the pose it is skinned with like its hilts
([menu_stage/cosmetics.rs](../crates/sjk-viewer/src/menu_stage/cosmetics.rs)),
unless `cg_cosmetics` is 0.

Every installed hat and cape is open to everyone: nothing is unlocked or
granted by a server. jaPRO's race-unlock hats (the `c5` clientinfo,
`cp_cosmetics`, `cosmetics unlocks`) are left out on purpose.

## Team saber colours

`cg_saberTeamColors` (archived, default 1; Settings > Advanced > Gameplay and classic+
Game options, **Team saber colours**) is JoF EJK's rule (`cg_players.c:8096-8111`,
`cg_xcvar.h:411`): in team gametypes (team FFA, CTF, CTY; not siege, not jedi-vs-merc,
never NPCs) a red player's blades are drawn in the stock red and a blue player's in
the stock blue, whatever `c1`..`c4` say: both sabers, held and thrown, a custom RGB
or a JA+ black blade included, and the player card's swatches. At 0 a blade keeps its
own colour unless it is the other team's (blue on red, red on blue) or past purple
(RGB, black and the other extended colours), as in EJK. Free players, spectators and
other game types keep each player's own colours. Fixes hub report 70 (random saber
colours in team FFA). An SJK blade shader stays worn and takes the team colour as its
base, as it takes the player's own colour elsewhere (a chroma turns to red or blue,
[unlockables.md](unlockables.md#chromas); integrator's choice, 17/10/2026, Sol may
revisit). See [team_sabers.rs](../crates/sjk-viewer/src/team_sabers.rs) and
`sjk_client::legacy_saber_team_colors`.

`cg_ctfTeamSabers` (archived, default 0; same pages, **CTF team saber colours**) goes
further in capture the flag and capture the ysalamiri: every red or blue player's blades
in their team's stock colour with no blade shader over them, muted players' default
saber included, whatever `cg_saberTeamColors` says. From SJK pull request #113 (Slash),
made an option at the merge (Sol's choice, 17/10/2026).

Verified by unit tests (team FFA, CTF and CTY force red and blue, free players and
spectators untouched, FFA, duel, siege and jedi-vs-merc keep the choice, the `0`
semantics per saber, black blades replaced per hand, the card's swatches); not tried
in the game.

### Siege class saber colours

In Siege a class file may force its sabers' colours (`sabercolor`, `saber2color`,
`bg_saga.c:874-893`), as stock does for its Jedi and Sith classes. cgame applies them
(`CG_NewClientInfo`, `cg_players.c:2492-2537`): the class named by a player's
`siegeclass` replaces the colour they chose, primary and secondary separately, and a
class without one leaves the player's own. SJK does the same client-side
(`sjk_client::legacy_siege_class_blades`, [siege_sabers.rs](../crates/sjk-viewer/src/siege_sabers.rs)):
it reads the installed class files once per Siege map (in the order SJK's server reads them) and
works each player's colours out again only when the serverinfo or a player's
configstring changes. The colours apply to held and thrown blades and the player
card's swatches, replace a JA+ black blade, and keep an SJK blade shader worn over
them, as the team colours do. Blocking a player still leaves their class colour (the
game sets it, not the player), as it leaves a team colour. Only cgame forces the
colour: the server, stock and SJK's own, sends `c1`/`c2` as the player set them.
Other game types ignore the class. No setting: it is stock behaviour. Verified by
unit tests (a class with `sabercolor 5` purple, a class without one and the other
game types untouched, the class files read once per map); not tried in a Siege
match.

## JA+ black sabers

On JA+/jaPRO and TaystJK servers, saber colour number 11 is shown as a black
blade with an alpha-blended trail and no blade light or bloom. It works for
both hands and thrown sabers. The archived `cg_blackSabers` setting is on by
default, under classic Settings > Game Options and SJK Advanced > Gameplay.
Turning it off uses orange. The existing `cp_pluginDisable` bit 3 also disables
black blades when set: Settings > Mods, Black sabers (JA+ or jaPRO plugin options), or
`japlus.plugin 3` switches it ([plugin options](mods.md#plugin-options)). The normal
plugin-mask default (1536) allows black blades.
The SJK UI's Profile > Saber offers black as a chip of its own, the last, after the
custom chip, which writes colour 11 (hub report #130, 27/10/2026): before, the
page had no black, and a saved 11 read and was written back as the custom colour. A
stored custom tint that is black's own near-black (`BLACK_RGB`, which an earlier test
build saved for black) counts as none: the custom chip showed it as a second black chip
that gave a white blade, and now shows and starts from the stock blue (`stored_rgb`,
Dave's test, 28/10/2026). Any other tint is the player's, however dark. Collection's
stock-blade swatch reads the whole `color1`, so black shows black there too, and a custom
colour in its `cp_sbRGB1` tint (it showed white; Neffie, 03/11/2026). The stage model
behind the Collection and Profile follows `color1`, `color2` and the tints as well as the
hilts, so a colour set elsewhere (`saberColor`, a config) shows without touching a slider. The classic page keeps retail's six swatches.

Other servers show colour 11 as before: purple, or the player's RGB tint when
one is set. (JoF EJK shows orange there; SJK keeps its earlier colour.) Ordinary RGB colours, including
an RGB tint of zero, retain their existing behavior; SJK blade skins retain
priority over the server's blade colour. Installed community black glow/core/
trail images are used when available; otherwise SJK generates black alpha
profiles locally. No extra content download is required. See
[rendering](rendering.md#ja-black-sabers) and
[verification](status.md#ja-black-sabers).

SJK's own server (devmap, Create game) carries black blades too, as JoF EJK shows
them on its own devmap, where JoF's JA+-based game module runs: the server says
`sjk\1` in its serverinfo and the families of `black_sabers::FAMILIES` (today `sjk`)
count as supported (27/10/2026). The blade, its halo and its trail are drawn as in
[rendering](rendering.md#ja-black-sabers), with EternalJK's own pictures when its
folder has them.

## EternalJK animation fixes

Players' animations are remapped before they are shown, as EternalJK's `CG_Player`
does: without a saber in hand, two-handed, dual and staff runs and walks play as the
ordinary ones; the old Bryar's `BOTH_STAND1` (servers without `g_fixWeaponAttackAnim`,
such as JoF's, fire it that way, arm hanging between shots) shows as `BOTH_ATTACK2`
with the arm raised; the concussion rifle's `BOTH_ATTACK2` shows as `BOTH_ATTACK3`.
NPCs are left as the server sends them.

A track starting the clip the other track already plays picks up its frame, as
`CG_SetLerpFrameAnimation` does against the legs' or the lumbar's Ghoul2 frame, except a
torso restarting the clip it already plays (a `torsoFlip`): stock clears `beginFrame`
there (`ci->torsoAnim == newAnimation`) and the swing starts over. Without that exception a
tauntaun rider holding attack, whose `BOTH_VT_ATR_S` restarts on both tracks each second
(`PM_VehicleWeaponAnimate`), picked up the legs' finished frame and showed one swing for
many swing sounds (Slash, hub report #123). See
[animation_selection.rs](../crates/sjk-client/src/animation_selection.rs).

The local player's predicted animation also starts over when a held animation's timer
ran out and came back up on the same clip and flip
([local_actor_state.rs](../crates/sjk-viewer/src/local_actor_state.rs)). A frame shows
the last command, so the one-command stand between two held melee kicks can go unseen,
and a JA+ 2.5 server restarts the kick with no stand and no flip (a demo of 30/10/2026:
`BOTH_A7_KICK_F` timer 4, then 692 a snapshot later, `legsFlip` never set). The torso
then kept the finished kick's clock, the legs started their kick on its frame, and the
kick stood frozen while the server kept kicking (Slash, 30/10/2026). Other players are
shown the server's animation, which carries no timers, so this does not reach them.

## Tavion and Desann idle poses

Tavion and Desann saber styles use their distinct JoF EJK standing poses by
default, with no setting: Tavion uses `BOTH_P1_S1_T_`, Desann `BOTH_STAND9`.
This applies to snapshot and predicted actors in every UI style. The client
replaces only each style's ordinary idle clip; other custom ready animations,
attacks and movement clips stay as supplied. Unlit or thrown sabers and other
weapons retain their ordinary presentation. Already distinct server-sent idle
poses pass through unchanged.

The change is visual: simulation, prediction inputs, attack timers and protocol
26 remain unchanged. Source choices follow [JoF EJK #47](https://github.com/JediofFreedom/JoF_EJK/pull/47).
Live appearance with installed animations remains unverified; see
[status](status.md#tavion-and-desann-idle-poses).

## Wall grab facing

A player grabbing a wall to jump off it (Force jump 3, pushing into a wall in the
air: `BOTH_FORCEWALLREBOUND_*`, then `BOTH_FORCEWALLHOLD_*` while the hold lasts)
is drawn facing the wall as its animation needs
([wall_hold_pose.rs](../crates/sjk-viewer/src/wall_hold_pose.rs)). The model takes
`vectoyaw(wall normal) + yawAdjust` from OpenJK's `PM_AdjustAngleForWallJump`:
forward grabs face the wall; side and backward grabs keep their animation's side
offset. A missed model-facing check keeps the rebound's last facing, and leaving
the rebound returns the model to the view. This applies to local and remote models;
the model check uses world brushes.

The camera follows the server profile by default (`cg_wallGrabLock 0`): stock
locks the view during the grab, while JA+ leaves mouse and keyboard look free.
On JA+ (including JoF servers using that profile), looking around while holding
jump aims the kick-off; this is an intentional server feature.

**Lock wall-grab camera** is in SJK Settings > Advanced > Gameplay and classic
Setup > Game options. `cg_wallGrabLock 1` opts into locking on JA+ too. With it
on, the camera uses the shared 128-unit upright-wall check against world and mover
collision to face the wall with the rebound side's offset, including frames with
only a snapshot seed. With it off nothing is re-aimed: a stock grab's view is the
one its movement slice or snapshot already turned to the wall.
Look input is consumed during a lock so it cannot build up or spill out after
release; movement and jump buttons still reach the command. Turning the option
off restores the profile's policy. Stock remains locked with either value.

The optional lock changes the local view used to build subsequent commands, so
JA+ players who enable it give up free mouse aiming during the hold. It does not
change the server's movement rules, replay state, delta angles or velocity.
Missing the wall keeps the current held view; releasing the grab ends the
optional hold. Model facing remains independent of the camera option.

## Dismemberment and disintegration

Both follow EternalJK's cgame (`cg_ents.c`, `cg_players.c`) and rd-vanilla's renderer.

- **Cut-off limbs.** A server with `g_dismember` above 0 sends each cut limb as its
  own entity. With `cg_dismember` 1 (no heads or waists) or 2 and up (everything) the
  client copies the owner's model for the limb, draws only the limb and its cap, and
  turns the limb off on the owner with the stump's cap on; a cut right arm, right hand
  or waist takes the weapon with it. Both cuts smoke, and a flying limb trails smoke.
  A body left behind keeps the missing limbs; the player is whole again once alive.
  `cg_dismember` defaults to 2 (EternalJK: 0), so a server that enables
  dismemberment shows every cut limb (0 shows none, and a config that saved 0
  keeps it). `g_dismember` (0 to 100) set in the
  console is handed to games this client hosts (Create game and `devmap`).
- **Disintegration** (`EF_DISINTEGRATION`): a disruptor kill, or a corpse shot or cut
  until it gives way, freezes the pose and burns the body away from the hit point:
  the model is eaten away with a blackened edge and `gfx/effects/burn` glows on what
  remains, with `disruptor/death_smoke` for the first second. A player is gone after
  1.5 s; a body draws until the server removes it. The stage program does rd-vanilla's
  per-vertex work (`RB_CalcDisintegrateColors`, `RB_CalcDisintegrateVertDeform`).
  It is always on, with no cvar, as in EternalJK and OpenJK (`CG_Player` checks only
  `EF_DISINTEGRATION`). Its colours are final: real-time sun light and fullbright
  leave them as they are, as the stage's colour generators do (`entity_control.x` 3).
  They replace every generator, the player tint (`char_color_*`) too, so a tinted
  model burns in its untinted colours, as rd-vanilla's `killGen` does. A corpse slashed
  until it gives way (`body_die`) burns from its own origin, the middle of the body,
  and is removed a second later (`BodyRid`): it looks cut in half and loses its tint
  for that second in EternalJK too (Nib's hub report 122, 22/10/2026).
- **Body fade** (`EV_BODYFADE`): 30 s after a body fell the server sends the event
  (`BodySink`) and unlinks the body 4 s later (`BodyRid`). As in OpenJK's and EternalJK's
  `CG_General` (`cg_ents.c`, `bodyFadeTime`), the body then thins out (forced alpha
  from 254 down by 0.08 a millisecond, gone after 2.54 s) while the body drawn again
  with its side's shader brightens over it (`RF_RGB_TINT` rising to 200): the light
  side's is `gfx/misc/personalshield`, the dark side's `gfx/effects/sabers/red_glow`.
  `personalshield` blends `GL_DST_COLOR GL_ONE` on display values in rd-vanilla (the scene
  times one plus the stage's colour); SJK's scene is linear, so the light side's glow sets
  `DISPLAY_FILTER_ADD` and the stage program writes the sRGB decode of (1 + display colour), less 1, for it
  (hub report #190: blended as written it showed only as a faint blue speckle). Two
  limits: an 8-bit scene (`r_sceneHdr 0`, the Vanilla style) clamps what a stage writes
  to 1, so there each stage lifts the scene at most twice in linear light, less than
  rd-vanilla's brightest; with HDR nothing saturates at display white as rd-vanilla's
  8-bit framebuffer does, so the brightest texels can go past it (into bloom).
  At 2.513 s that glow burns away from the body's origin (`RF_DISINTEGRATE2`), with the
  Force Sight sound for the light side and lightning for the dark. For the first 3.4 s
  the light side hums (`saberhum1`, for 2.2 s, at most once per 100 ms as EternalJK
  plays it) and the dark side's body is drawn red with a random one of the two electric
  shaders, crackling now and then (a one-in-ten roll once per 100 ms, EternalJK's fix
  for the loud body sound). The side is `ircg`'s fourth operand (no `ircg`: the
  dark side, as cgame's zeroed `teamPowerType`). The event also turns off the body's
  `head_eyes_mouth` surface, back on when the mesh is given to another body. The
  client keeps each body's start, seen again in a body slot that never left the
  snapshot once its event field went back to 0 (`CG_CheckEvents`), and forgets every
  start when the server clock goes back (a new gamestate)
  ([body_fade.rs](../crates/sjk-client/src/body_fade.rs)) and draws the layers in place
  of the body ([body_fade.rs](../crates/sjk-viewer/src/body_fade.rs)); held weapons and
  hilts drawn apart from the body stop drawing when the fade starts. SJK's own server
  sent the event as 125 (`EV_PUSHED1`) before 24/10/2026, and kept the body 18 s after
  it (the 18 s think commented out in `g_client.c`); it sends 100 now and unlinks the
  body 4 s after it. On High Quality (rig) models, see-through parts show as squares
  in the glow: an override shader drawn over an actor replaces each surface's material,
  alpha test included (force shells use the same path).

Model cap surfaces (`*_cap_*`) are carried in every actor mesh, hidden until a cut.

## Third-person camera

Third-person framing follows OpenJK multiplayer `CG_OffsetThirdPersonView`.
The camera uses a four-unit collision hull against solid/terrain/player-clip
surfaces and presented inline models, including moving doors and platforms and
the permanent ones the server sends only in its baselines, such as the `misc_bsp`
map pieces a server places in a map's void: stock `CG_BuildSolidList` adds them
from `cg_permanents` (`cg_predict.c`), so the camera stops at their walls. The
crosshair name trace uses the same list, so such a wall hides a player behind it.
In a free spectator camera it still names the player under the crosshair, and a
primary click while aimed at that player sends `follow <client>` to enter their
view. The click is consumed instead of running the primary-button bind; a click
with no visible player under the crosshair keeps its ordinary binding.
Player and vehicle bodies do not obstruct this camera trace. When geometry
collapses the camera onto its target, the view uses the intended forward direction
instead of constructing an undefined look-at matrix.

Pitch limits, pitch-offset direction and turn-dependent damping follow the
multiplayer reference. How fast the camera closes the gap follows JoF EJK's
`cg_cameraFPS` (default 125, as EternalJK): the damping cvars apply per frame of
that rate whatever the real frame rate, and the ideal point's own movement is
compensated (EternalJK `CG_DampPosition`), so the camera stays close behind a
fast-moving player. As in EternalJK the damping is timed by the predicted
player's command time, the clock the focus moves on; timed by the presentation
clock it stuttered. Stock multiplayer applies them once per 50 ms step, against
8 ms per step at the default 125, so the same damping value closes the gap in
steps 6.25 times shorter; `cg_cameraFPS` below 15 restores the stock behaviour.
View changes, teleports, followed-player changes and mounting/dismounting reset
the presentation history. The ordinary range, height, angle and damping cvars
remain available; `cg_thirdPersonHorzOffset` controls the stock sideways offset.
`cg_cameraStyle` chooses whether the damping applies ([Camera style](#camera-style)).

A dead player is always seen in third person, whatever the camera choice
(EternalJK `cg_view.c:3098`), and the death camera is fixed: it turns to the
player state's dead yaw (`STAT_DEAD_YAW`, towards the killer) and keeps the pitch
the player had at death, as pmove stops updating a dead player's view angles
(`bg_pmove.c:10159`). Moving the mouse while dead no longer tilts it
([camera.rs](../crates/sjk-viewer/src/camera.rs) `dead_view_angles`,
[scope.rs](../crates/sjk-viewer/src/scope.rs) `third_person_wanted`).

A fall to death (a pit's `trigger_hurt` that fades the screen to black, the
player state's `fallingToDeath`) does not take the camera down with the player: as
in EternalJK, the third-person camera's focus stays where the fall began
(`CG_CalcIdealThirdPersonViewTarget`, `cg_view.c:286`), and the view itself is moved
to that point, turned from the computed eye towards the falling player
(`cg_view.c:3224-3233`), in first person too. It lasts until the flag clears at the
respawn ([fall_camera.rs](../crates/sjk-viewer/src/fall_camera.rs)). A demo's
detached cameras are left alone.

The decaying prediction error (`cg_errorDecay`) moves the third-person camera's
focus before its collision and damping traces, as `CG_CalcViewValues`
(`cg_view.c`) adds it to the view origin before `CG_OffsetThirdPersonView`. A
correction next to a wall is therefore traced like any other focus movement
instead of shifting the finished camera into the wall. First-person and other
views still add it to the finished view origin.

Vehicle appearances cache their `.veh` camera settings when loaded. Mounted
views use the authored range, height, pitch and sideways offsets, including the
pitch-dependent and fighter-strafe adjustments. Vehicle targets follow immediately
and camera damping follows the stock vehicle policy. These are presentation
changes; movement, vehicle physics and protocol serialization are unchanged.

The locally piloted vehicle is drawn at the same predicted origin and angles as
its movement, following `CG_AddPacketEntities` and `CG_CalcEntityLerpPositions`.
Its rider seat uses that same root with the current animated driver bolt. This
avoids the camera advancing ahead while the mount and rider trail behind on
snapshot interpolation. Remote vehicles, passengers without local vehicle
prediction, and demo playback retain their snapshot presentation. The camera's
animated seat is evaluated every rendered frame, on its own skeleton posed as the cgame
poses the vehicle (`CG_SetLerpFrameAnimation`): a new vehicle animation starts, blended
over 100 ms, once presentation reaches the snapshot that brought it. Command replay keeps
the server's pose at the newest snapshot's time; read from that skeleton, which starts a
new animation ahead of presentation, the seat showed its first frame unblended and then
jumped back, and an AT-ST pilot's camera jumped at every walk, run and stand change
(Kushina, 21/10/2026; [prediction_ride.rs](../crates/sjk-viewer/src/prediction_ride.rs)).
Prediction-error smoothing uses
the vehicle root while piloting, avoiding false corrections from rider animation.
The movement collision adapter excludes the piloted vehicle's own snapshot body
and its owned objects, matching the stock skip/ownership rules; dismounted and
foreign vehicles remain solid.
Snapshot replay also retains the same ride's local vehicle timers, including turbo
expiry and recharge, while refreshing its networked state. A different vehicle,
pilot or definition starts with fresh local state. This prevents exhausted boost
input from predicting a new burst after every snapshot.

The decaying prediction error moves the camera's focus before the
collision traces, as `CG_CalcViewValues` adds it to the view origin before
`CG_OffsetThirdPersonView`, rather than shifting the finished camera, which could
put it inside a wall. `cg_thirdPersonAlpha` is not implemented; stock multiplayer
has no automatic fade when the camera nears the player. See
[camera.rs](../crates/sjk-viewer/src/camera.rs) and
[camera_motion.rs](../crates/sjk-viewer/src/camera_motion.rs).

### Camera style

`cg_cameraStyle` (archived; Settings > Gameplay > Game options, "Camera and
prediction", and First setup's third row, under Styles) picks how the camera
follows the player ([camera_style.rs](../crates/sjk-viewer/src/camera_style.rs)).
Settings names the values MP camera (`ejk`) and SP camera (`sjk`), Sol's names
of 26/10/2026 (EJK and SJK before); the cvar values stay, so saved profiles keep
their choice.

| Value | Camera |
| --- | --- |
| `ejk` (default, MP camera) | Sits at its ideal place every frame: no camera or target damping, whatever the damping cvars say. |
| `sjk` (SP camera) | Eases towards its ideal place with `cg_thirdPersonCameraDamp` and `cg_thirdPersonTargetDamp`, as described above, so it trails a moving or turning player a little. |

An unknown value selects `ejk`. `ejk` became the default on 08/10/2026 (Sol's
choice; `sjk` was the default before). Every profile had saved `sjk`, so the
console's start moves a saved `sjk` once to `ejk` and marks the profile
(`cg_cameraStyleDefaultVersion`); an `sjk` picked after that stays.

`ejk` is JoF EJK's camera as Sol plays it (07/10/2026). The camera code is the same
in both clients: EternalJK's damping with `cg_cameraFPS 125`, range 80, height 16
and damping 0.3 and 0.5 are SJK's defaults too. But EternalJK switches both
dampings off while a strafe helper style is drawn (`cg_strafeHelper` bits 0-3 or
13; `CG_UpdateThirdPersonTargetDamp` and `CG_UpdateThirdPersonCameraDamp` in
`cg_view.c`), and Sol's JoF EJK profile has one on (`cg_strafeHelper 2242`), so
there the camera never trails. Range, height, angles, collision, vehicle framing
and the first/third person choice are the same in both styles; Sol's JoF EJK
profile also sets `cg_thirdPersonRange 100`, which SJK leaves to that cvar. SJK's
`sjk` style does not apply EternalJK's strafe helper rule.

### Camera settings

Settings > Advanced > Gameplay > Camera and prediction, and Classic+ Setup >
Game options, expose the camera controls from JoF EJK's
`assets/japro/ui/jamp/ingame_camera.menu` at official alpha commit `9822a9d6`
(`1.6.1-alpha.347`; the file is unchanged since its initial `52e068d0` import). The
shared settings rows cover the first-person weapon FOV; EJK's damping timing
presets (Default 125, Smooth (SP) 30 and BaseJKA); third-person distance, angle,
pitch, height and horizontal offset; and camera/target damping. Their ranges and
defaults match that menu. The ordinary world FOV stays under Display, the HUD
switch under HUD, and first/third-person switching remains the Camera key rather
than a persistent option. The damping rows (timing preset, camera and target damping)
only act under `cg_cameraStyle sjk`; the default `ejk` style keeps its own camera.

### Camera control

Camera control (named Shot controls until 08/10/2026) is a panel for framing
screenshots and recordings in a match or a demo
([ingame_menu/shot.rs](../crates/sjk-viewer/src/ingame_menu/shot.rs)). The game
menu's camera icon opens it (the SJK UI's row of icons; the classic bar has no
entry), as do F8 while F8 has no binding, the `cameracontrol` command (bindable
under Controls > Other) and the quick wheel's Camera control. It drives the
console's presentation director, the same one as the `demo_camera` and
`demo_sun` commands, which keep their names: nothing it does reaches the
server, prediction or the player's input.

- **Camera page:** View from Back, Front, Left or Right; Angle (-180 to 180),
  Pitch (-80 to 80), Distance (16 to 512) and Height (-64 to 160) of the
  third-person camera, starting from where it is (`cg_thirdPersonAngle`,
  `cg_thirdPersonPitchOffset`, `cg_thirdPersonRange`,
  `cg_thirdPersonVertOffset`). A change turns third person on. The pose stays until
  Reset, but one of those cvars changed meanwhile (the console, a bind, Settings) takes
  its place in it at once, as EternalJK applies it; the other three stay as set here
  (Rollembones' report, 22/10/2026: `cg_thirdPersonRange` did nothing after the camera
  had been set here).
- **Sun page:** the sun's direction (0 to 360) and elevation (-20 to 90), when
  the view has real-time lighting (`r_dayNight 1` at launch) and open sky, what
  holds the sun now (the day settings, Camera control, or returning), and Reset
  sun, which hands it back to the day settings at once.
- **Motion:** Move duration (0.2 to 20 seconds) and Orbit speed (-30 to 30
  degrees a second); Orbit turns the camera (or the sun) round, Stop holds it,
  Reset (the Sun page's Ease back) eases back to the settings over the move's
  duration.
- **Live preview** (on by default) shows each change at once; off, changes wait
  for Move and hide, which moves there over the move's duration. **HUD**
  switches `cg_draw2D`. **Hide panel** (Move and hide) closes the panel and
  applies; **Close** closes it without moving.

Keys: Up, Down and Tab move from control to control (a slider's number and track
are one stop), Left and Right step a slider, a digit types its value (Enter
applies, Escape cancels), Enter or Space acts, F8 or Escape hides the panel. The
pointer drags a slider's track, a click on its number types it, and a click acts
on a button, a tab or a switch. The panel has the SJK UI's look in every menu
style ([SJK UI](sjk-ui.md#camera-control)); the classic menus' right-edge panel
was removed on 10/10/2026.

## Ghost replay

`ghost` records the player's own run and plays it back later on the same server, to
film a showcase: the player records a run once, then replays it while another player
films ([ghost_replay.rs](../crates/sjk-viewer/src/ghost_replay.rs)).

`ghost` and [`mimic`](#mimic) are experimental, for SJK staff keys only (Sol,
19/10/2026: not for everyone to use), and off by default: Settings > Mods >
"Showcase tools (SJK staff)" (`cl_experimentalShowcase`, archived; "Showcase tools" in
the classic menus) turns both on for a staff key. What players see stays vague (Sol):
the row, its help and the commands' help say only "showcase tools", and neither
command is in the changelog or credits. For another key either command answers
`<command>: showcase tools are for SJK staff`, and the switch does nothing;
off, either command says so and where to turn it on; switching it off ends a replay or a copy, and a recording
is saved. They have no other menu entry.

| Command | Does |
| --- | --- |
| `ghost record <name>` | Records every command from the next one: view, movement keys, buttons, weapon, Force selection and generic actions, with the position and speed after each |
| `ghost stop` | Ends a recording and saves it, or ends a replay |
| `ghost play <name>` | Walks to the recorded start, then plays the run |
| `ghost list` | Lists the saved runs with their maps and lengths |
| `ghost info <name>` | A run's map, length, start (position and facing, to teleport there) and how much of it the server confirmed |
| `ghost edit [name]` | Opens the [ghost editor](#ghost-editor) on `name` (else the first run of this map) |
| `ghost` | Help and what it is doing |

Runs are text files in `ghosts/` beside `config.cfg` (`<name>.ghost`, letters,
digits, `-` and `_`): a header with the map and start, then one line per command
(time, view, keys, buttons, selections, then the origin, velocity and ground after
it). While recording, each command keeps the predicted state after it until a
snapshot acknowledges that command; the server's state then replaces it. A recording
ends on its own at ten minutes, on death or spectating, on flight, or on leaving or
changing map, and is saved.

**Replaying.** Movement is deterministic: the same start and the same commands with
the same durations give the same run whatever the ping, because the server steps each
command by its own time (`ClientThink_real`). A replay sends the recorded commands with
fresh stamps; the view the server derives (`cmd.angles + delta_angles`) is the
recorded one, so a different `delta_angles` (after a respawn or a teleport) does not
turn it. The camera follows the replayed view. Commands the clock passes over (a
slower frame) give their presses to the next one, so a one-command jump or Force press
is not lost. Force-wheel reliable commands (Stasis, Repulse, Dash) are not recorded.

**The start.** `ghost play` refuses a run recorded on another map or a start more
than 192 units away (it prints the start, to go or teleport there). Closer, it walks
there at walking speed in the current view, switches to the recorded weapon, faces
the recorded view and stands still for a few commands, then begins. A start reached
within half a unit and standing still replays the run untouched. Start recordings
standing still too.

**Drift.** The server owns the position, so a replay corrects drift (a command
lost twice merges into the next one, a shove) through the movement keys alone, and
only on the ground:

- The keys turn, up to 20 degrees, keeping the largest key's size so the speed
  stays (`PM_CmdScale`); the view stays the recorded one. They first take out a
  sideways speed, then close the offset at a unit a second per unit of drift (at
  most 20). Drift under 2 units with a sideways speed under 1.5 is left alone:
  Pmove rounds the velocity to whole units after every command and friction cannot
  shrink a small sideways speed past that rounding, so a tiny correction would leave
  a sideways speed for the whole run.
- In the air nothing steers: air control (`pm_airaccelerate` 1) adds less than the
  rounding takes back. What a jump carries is the speed it left the ground with,
  which the ground steering has already matched.
- Behind or ahead along the path by more than 2 units on steady ground (the same
  keys as the next command), the recorded clock waits a command or skips one, up to
  a second in all.

A replay stops, giving control back, at the end of the run, when the player presses
any movement key or button, takes damage, dies, spectates, starts flying
(`freecam`, `fakenoclip`) or a duel begins, or when it drifts more than 48 units
sideways or 64 vertically ("lost the run"). It refuses to start in a duel or within
five seconds of taking damage. The last line says how many corrections it made and
the worst drift.

The tests run the real Pmove on a synthetic course (a platform, four pillars over a
pit, a landing) with a scripted pillar-jumping run: replayed from its start it ends
where the recording did; walked to from 30 units away with the server losing the
second takeoff's command and a command in the air, it lands every pillar and ends
0.4 units from the recording; shoved sideways at 60 units a second on the platform,
it ends 7 units off where an unsteered replay ends 46 off. Not yet tried on a live
server.

### Ghost editor

`ghost edit [name]` opens a column down the window's right edge, in a game or on an
explored map, to cut a run and clear its chat bubble
([ghost_replay/editor.rs](../crates/sjk-viewer/src/ghost_replay/editor.rs), its look in
[sjk-ui.md](sjk-ui.md#ghost-editor)). Like `ghost`, it is for staff keys with the showcase
tools on; switching them off, leaving the game or changing map closes it. Sol asked for it
on 01/11/2026.

- **Ghosts:** every `.ghost` file with its map and length, read from the header and the
  last line only; another map's runs are quiet, an unreadable file says so. Up and Down
  or a click open one; a cut not saved is dropped, and the answer line says so.
- **Duplicate** writes the run as it is cut under the name in its field (the first free
  `<name>_2`, `<name>_3`...) and opens the copy; the original is untouched. An existing
  name is refused.
- **Cut:** a timeline of the run in 104 slices (standing, moving, in the air, the bubble,
  the loudest in each slice), a green strip under it where a replay can begin cleanly,
  the cut parts dimmed, and Start and End rows with the times. A replay always begins
  standing on the start ([the start](#ghost-replay)), so the start snaps to commands the
  player stood still on the ground before (under one unit a second): Left and Right step
  a tenth of a second to the next such command, Page Up and Page Down a second, a drag on
  the timeline or a row's track lands on the nearest one. Shift places it anywhere (one
  command a step), and the start then shows in ember with "Starts mid-move". The end goes
  anywhere after the start. **First move** puts the start on the command where the player
  first moves (a key, a pressed button, speed or the air), cutting the console's closing
  and the wait; **Last move** puts the end on the command the player stops on; **Reset**
  keeps the whole run.
- **Clear the chat bubble** (on): the saved run has no `BUTTON_TALK`. It is set while the
  console, chat or a menu is open (`CL_CmdButtons`), as when `ghost record` and `ghost stop`
  are typed, and a replay sends it again, so the bubble showed over the replay. Clearing it
  changes no movement: those commands carry no movement keys. The commands are kept, not
  deleted, so the run's timing never changes (Sol's choice). A replay still shows the
  bubble while the replaying player's own console is open.
- **Save** overwrites the run with its cut after a second press (the first says so);
  **Play** runs `ghost play` on the saved run and closes the editor, or shows why it
  refuses (a cut not saved, the start too far); **Look** or Escape hides the column to move
  about with the path drawn and a strip at the top, and Escape brings it back; **Close**
  turns the editor off. Ctrl+S saves.

The edited run keeps commands from the first to the last kept, their times begun again at
zero, and its start is the state after the command before the first (position, view,
speed, weapon), so `ghost play` walks to the new start. The path is the run's feet (24
units under the origin), a point every 12 units or more (1200 at most), the kept part holo
blue, the cut part grey, the bubble ember when it is kept; a gold ring of 16 units marks the
new start with a line the way it faces, a white ring the end. It is drawn over the scene
without depth (seen through walls), only for a run of the map the player is on.

## Mimic

`mimic <player> [delay] [mirror|copy] [walk]` copies another player's moves live: their view,
movement, jumps, crouches, saber attacks, stance and holster
([mimic.rs](../crates/sjk-viewer/src/mimic.rs)). It is meant for a saber showcase with
two players facing each other. The player is a slot number or part of a name (as
`tell` takes it); the delay goes from 0 to 10 seconds (`0.5`, `2s`); `mirror`, the
default, copies like a reflection, and `copy` keeps the leader's sides; `walk` walks
always, for a choreography done at walking pace, instead of walking only while the
leader moves slower than 140. `mimic off` stops it and `mimic` alone says what it does.
Like `ghost`, it is experimental and off until Settings > Mods > "Experimental:
showcase tools" turns it on ([Ghost replay](#ghost-replay)).

The client never sees another player's keys, only what they lead to: each snapshot
carries their origin and velocity (`trBase`, `trDelta`), view angles (`apos.trBase`),
ground entity, linked box (`solid`'s top tells a crouch, 16, from standing, 40), saber
move, stance (`fireflag`), holster and weapon. The mimic keeps these samples, plays them
back on its own timeline after the delay (interpolated, extrapolated at most 100 ms past
the newest) and rebuilds commands:

- **Facing each other.** Where both stood and faced when it began are the anchors. The
  leader's motion since then, in their own frame, becomes the mimic's in its frame: the
  leader stepping toward the mimic makes the mimic step toward them. `mirror` swaps
  left and right, so a step to the leader's left is a step to the mimic's right and both
  stay face to face; their yaw turns mirror too. Pitch is copied.
- **Movement.** On the ground the keys aim for the leader's velocity, read 80 ms ahead
  where the delay leaves samples there (so a turn starts when theirs did, not an
  acceleration later), plus a return toward their mapped position at 4 units a second
  per unit beyond 2, with part of the gap to the current velocity added to take a
  sideways speed out. A slight backward component is dropped, since any backward key
  slows the player (`BG_AdjustClientSpeed`: 0.75, 0.45-0.75 in an attack). Walking
  pace (under 140), or always with `walk`, sets `BUTTON_WALKING` for the walk
  animation, and every key is then scaled within 64: Pmove clears the walk past half a
  key, "to avoid proxy no-footsteps cheats" (`bg_pmove.c:10490-10494`), so a mimic
  catching up with a larger key would run. A saber attack is chosen by the keys' signs,
  so walking attacks keep their direction. In the air the keys
  follow the leader's horizontal acceleration (air control pushes along the keys), not
  the offset.
- **Jumps and crouches.** A takeoff faster than 100 upward is a jump, held while the
  leader rises (a Force jump's height follows), let go for a command first if jump is
  still held. A crouched box crouches once the mimic is under walking pace: crouching
  while running rolls (`PM_Footsteps`), so a crouch waits for the mimic to slow down,
  unless the leader moves faster than 200 and is rolling too.
- **Saber.** The leader's saber move names the controls that ask for it, the inverse of
  `PM_SaberAttackForMovement`: a directional attack, its start or a transition into it
  holds attack with the keys of the quadrant it starts from (right and forward for the
  top-left start, right alone for the left, forward for the overhead); `mirror` swaps
  the quadrants and the specials' left and right forms. Those keys are pressed only where
  a move is chosen, starting from the ready or a return and in the last 24 ms of an
  attack (`PM_WeaponLightsaber`), where the leader 32 ms ahead says what comes next; in
  between the keys follow. Specials hold their controls throughout: the backstabs (back),
  the lunge and spins (crouch and forward), the DFA, flip over and jump attacks (jump
  and forward), the cartwheels and butterflies (jump and a side), the backflip, the
  kicks (alternate attack and a direction), the katas (both buttons); the others hold
  attack. A bounce keeps the buttons; parries, knockaways and locks, the fight's and not
  inputs, let go. A different stance presses `saberAttackCycle` (every 400 ms, eight
  tries for a stance this player lacks) and a different holster `sv_saberswitch`.

With no delay the copy runs as soon as the leader is seen, which is after their ping,
the snapshot interval and this client's ping, and nothing ahead is known. It stops on any
key or button the player presses, death, spectating, flight, a duel, leaving, a map
change or the leader out of sight for three seconds; it refuses to start in a duel, on
oneself or during a ghost replay. Taking damage does not stop it: blades meet in a
mirrored showcase.

The tests run the real Pmove for both players on a flat floor: a scripted leader turns,
walks, strafes, jumps, crouch-walks and attacks (two chained attacks, then an overhead),
with snapshots every 24 ms reaching the mimic 60 ms late, the mimic facing them 128 units
away. With delays of 0, 0.3 and 1 second the mirror does every saber move mirrored (top
right, from the left, then the overhead), each attack starting 4 to 24 ms after it
should, jumps and crouches, turns its view 30 degrees the other way and ends within 2
units of the mirrored spot; during the run it stays within 10 units with a delay and 31
with none (a full-speed leader cannot be caught up). `copy` keeps the sides. With `walk`
and a leader doing the same choreography at walking pace (keys of 46, walk held,
attacks included) every command holds the walk with no key past 64, the attacks come
mirrored 4-24 ms late, it crouches, tracks within 8 units with no delay and 5 with 0.3
seconds, and ends within 2. Not yet tried in a game.

## Sound memory

Every sound is decoded once, on the audio decode worker, to mono PCM at the mixer's
44.1 kHz and kept for the session in the mixer's bank
([sjk-audio](../crates/sjk-audio/src/lib.rs)). Since 25/10/2026 the bank keeps 16-bit
samples, not 32-bit floats: half the memory. A 16-bit source (nearly all retail and
community sounds) is kept exactly; a resampled sound (most retail WAVs are 22.05 kHz)
or a float WAV is rounded to the nearest 16-bit step, at most 1/65536 of
full scale off (a float WAV louder than full scale is clipped there). The mixer turns
each sample back into a float as it reads it, with the 1/32768 step folded into the
gain it already multiplies by, so mixing costs what it did and still never allocates.
WAVs and MP3s decode straight to 16-bit mono (a WAV into room for its header's
length), so decoding a long track no longer holds float copies of it as well.

Music is no longer kept forever. A map's tracks are decoded as music, under handles of
their own outside the shared sound cache (`MusicTrack` in
[audio_assets.rs](../crates/sjk-viewer/src/audio_assets.rs)), and when the music is
chosen again (the next map, a `CS_MUSIC` change, `music`, `stopmusic`, a map without
music) the tracks the new choice does not use are released. The release travels the
decode queue behind the track's registration; the audio callback stops whatever plays
the sound, takes its PCM out of the bank (the handle then plays nothing and is never
reused) and hands it back over a ring to the decode worker, which frees it. The
callback never frees memory and takes no lock. A file another sound decoded first is
shared and never released this way: the duel music (`music/mp/duel.mp3`) stays
decoded for the session, as do effects and voices. A duel suspends the map's track
without releasing it, and a dynamic music level keeps its states' tracks until the
level changes. A map change only stops the old music: the next map's prefetch may hold
the same tracks, so they are released when its music is chosen.

Retail figures, mono at 44.1 kHz (`crates/sjk-audio/tests/retail_memory.rs`, ignored;
`JKA_GAME_DATA=<GameData> cargo test --release -p sjk-audio --test retail_memory --
--ignored --nocapture`): the 302 effect WAVs under `sound/`, 31.5 MiB of files, took
127.1 MiB as floats and take 63.5 MiB; a music track averages 9.7 MiB as floats and
4.8 MiB now, the longest (`yavtemp_explore`) 47.2 and 23.6 MiB. Before, every map's
music stayed decoded on top of the last one. `soundinfo` prints the decoded total;
with `SJK_TRACE_AUDIO=1` the audio trace adds it (`pcm`), what was freed (`freed`) and
releases kept for want of room (`kept`, expected 0).

Unit tests: `pcm16_tests` in sjk-audio (a 16-bit WAV kept exactly; voices, loops and
music within 1/32768 of the float samples they replaced; a released sound hands back
its PCM and stops playing; mixing and releasing make no allocator call, checked with
the test-only `assert_no_alloc` allocator) and `music_memory_tests` in the viewer (a
new track frees the old one on the decode worker, a shared one stays, prefetched music
is music). Not yet listened to in a game.

## Sound listener

The mixer hears from one place a frame, as cgame's `S_Respatialize` from
`refdef.vieworg` and its axes (`cg_view.c:3245`): `GpuState::finish_audio_frame` puts
the listener at the camera with the camera's axes, and every sound is panned and
attenuated from there. Until 19/10/2026 each new snapshot also moved it, for the rest of
that frame, to the snapshot's player origin and view angles (`GameAudio::observe_snapshot`);
the audio thread mixes while the frame is built, so the blocks mixed in between panned
sounds near the player differently from the others whenever they moved, and the saber's
hum, a loop on the view, crackled (Toad's report). Unit test:
`a_snapshot_leaves_the_listener_where_the_frame_put_it`; not yet heard in a game. With
[3D sound](#3d-sound) on, the listener stands at the player's head instead, still with
the camera's axes.

## 3D sound

`s_spatial` (Settings > Sound, 3D sound; off by default, archived; Sol's, 03/11/2026,
after the idea of VisionJK's `v_spatial` by Blackwolf) gives the sounds of the level
three things the 2003 mixer lacks, in SJK's own mixer (no Steam Audio, no library):

- **Hearing from the head.** The listener is the player's predicted origin raised to
  the camera's height, kept between crouching and standing eye height (12 to 36 units),
  so in first person the eyes and in third person the head, not the camera circling it;
  left and right stay the camera's. A camera more than 160 units from that point (a
  spectator, the drone, the free camera) or no predicted player puts it back at the
  camera, as VisionJK's `S_SpatialHearing` does.
- **Occlusion.** About 15 times a second the render thread traces from that point to the
  world sounds heard in the last 8 seconds (48 at most, 16 a tick, three point rays each:
  straight, 24 units higher, 24 units to the side), against the map's solid brushes and
  the doors and lifts where they are shown (`LocalPrediction::presented_solids`, each
  tried only when the ray crosses its box). The share of rays blocked is the source's
  occlusion, 0 to 1; a wall within 16 units of the sound is the one it is on (an impact)
  and does not count, and sources past 2000 units are not traced. The mixer glides each
  voice and loop toward it over about 50 ms and blends the sound with itself through a
  one-pole lowpass at 700 Hz and 0.45 of the gain, so a fully occluded sound is quieter
  and dull. The player's own, listener-relative and global sounds are never occluded.
- **Reverb.** 32 rays round the hearing point (a Fibonacci sphere, 8 a tick, so a new
  room every quarter second) measure the room: the share that meets a wall within 2048
  units (not the sky) and their mean distance give a size (64 units a cupboard, 1024 a
  hall) and a wet level (at most 0.6, less the more open), which a small Schroeder
  reverb in the mixer (four combs and two allpasses an ear, Freeverb's lengths) glides
  to over 0.4 s. Its input is a send (a fixed buffer of 4096 frames in the mixer)
  that only voices and loops placed in the world and fading with distance feed, the
  player's own placed sounds included; menu cues, the announcer, listener-relative and
  global sounds and the music stay dry. Out of a map the room is dry.

Only numbers cross to the audio thread, on the usual command queue
(`AudioCommand::SpatialEffects`, `SourceOcclusion`, `Reverb`;
[audio_spatial.rs](../crates/sjk-viewer/src/audio_spatial.rs), the mixer's side in
[spatial_fx.rs](../crates/sjk-audio/src/spatial_fx.rs)). No behaviour is keyed to a
sound's file name and no material is read. Every table is fixed: the tick, the mixer
and the reverb allocate nothing (checked with `assert_no_alloc`). Off, nothing is
traced or sent and the mixer takes its plain path, its output bit for bit what it was
(`switched_off_the_mix_is_bit_identical_whatever_was_measured`). Measured on Windows
11 in release (`spatial_mix_cost`, ignored test), a 128-frame block with 128 voices and
32 loops busy took about 23 us off and 53 us on, against its 2.9 ms of playback. Unit
tests: the hearing point, occlusion from rays and its glide, muffling, loops, the room
from its rays, the reverb bounded and dying away, a larger room ringing longer. Not yet
heard in a game.

## Weather sounds

`s_weatherSounds` (Settings > Sound, Rain and wind sounds; off by default as it changes
how stock Jedi Academy sounds, archived; Sol's request, 05/11/2026) plays the game's own
outdoor loops with the weather ([weather_sound.rs](../crates/sjk-viewer/src/weather_sound.rs),
[audio_weather.rs](../crates/sjk-viewer/src/audio_weather.rs)): `rainy_lp` and
`windy1_lp` of `sound/ambience/outdoor/` (`assets0.pk3`), in both ears with the frame's
loops, registered the first time they are heard.

- Rain is as loud as the rain clouds' haze against a downpour's: `heavyrain` a full loop
  (0.6 of the effects volume), `rain` 0.57 of it, `lightrain` 0.32; snow, sand and fog
  make no rain sound.
- Wind is as loud as the wind's speed across the ground against a gale of 3000 units a
  second (the gusting wind's strongest, 0.5 of the effects volume), so it rises and falls
  with the gusts; `constantwind ( -5000 0 0 )` blows at full.
- Shelter: the weather cover's columns ([rendering.md](rendering.md#weather)) within 192
  units of the listener, every 64, count by nearness; a column counts as open when its
  weather comes down to 64 units above the listener and its sky is above them. Under open
  sky the loops play fully, under a roof or indoors a fifth (the rain on the roof), in a
  doorway about two thirds. Before the cover is surveyed, and on a map without sky whose
  own weather falls everywhere, it is heard as open; weather chosen for a map without sky
  (forced or a map edit's) is neither drawn nor heard.
- Levels ease over 0.6 seconds; frozen weather and `r_weather 0` are silent. The listener
  is the camera's position, so a free camera hears the weather where it flies.

The loops are not positioned, so `s_spatial`'s muffling does not apply to them: shelter
lowers them instead.

With them (05/11/2026): **thunder** after each lightning strike (`r_weatherLightning`,
[rendering.md](rendering.md#weather)), the game's `thunder_close1`/`2` for a near strike,
`thunder1` to `4` for a far one, 0.45 to 0.9 loud and at least half of it under a roof (it
carries indoors better than rain), and **single gusts**: when the wind rises past 55% of a
gale one of the game's `windy_a` to `_d` plays, 0.35 to 0.8 loud by the gust's strength
and the shelter, at most every 4 seconds and only once the wind has fallen below 40% again.
They play in both ears on their own source, after the loops are registered.

## Muted sounds

`s_muteSounds` (Settings > Sound > Muted sounds, empty by default; akr's and Neffie's
request, 03/11/2026) takes words, and no sound whose file path holds one of them plays,
whatever plays it: an item's or power-up's loop (`ysalamiri`), the loops a server puts on
players (`hacking`, `supp_healed`, `fry`), a map's machine, a mod's sound. Matching
ignores case: `s_muteSounds "ysalamiri hacking"`. The test sits where the mixer is fed
(`AudioOutput::send`): one bit per loaded sound, worked out again only when the words
change or new sounds load, so a played sound costs one bit test
([muted_sounds.rs](../crates/sjk-viewer/src/audio/muted_sounds.rs)). One player's sounds
are muted with `mutesounds <player>` ([hub-chat.md](hub-chat.md#muting-a-players-sounds)).

## Sound sonar

`cg_soundSonar` (Settings > Sound, Sound sonar; off by default; Dawson's idea for deaf
and hard-of-hearing players) draws a short arc round the crosshair toward each world
sound the client plays, ahead being up: light blue steps (`CHAN_BODY`), orange shots
and swings (`CHAN_WEAPON`), green voices, yellow for the rest (explosions, clashes,
Force, items, toys). Louder at the listener (the sound's own volume and falloff) is
thicker and brighter; each fades over a second, and one source's sounds of a kind share
an arc. It is fed where the audio output sends a positional one-shot
(`AudioOutput::send`, [sound_sonar.rs](../crates/sjk-viewer/src/sound_sonar.rs), a
24-entry ring), so it shows nothing the speakers do not get: muted players, `cg_footsteps
0` and sounds past their falloff stay hidden, and the player's own, listener-relative,
ambient, announcer, menu and music sounds and loops are left out. The volume cvars do
not hide it, since it is for players who cannot hear. It needs an audio output: with
`s_initsound 0` or no sound device nothing is sent, so the sonar shows nothing. Its
memory is cleared when the setting goes off and when the output stops the effects (a
new map's gamestate) or everything (`soundstop`); out of a game it draws nothing. A
deliberate choice: the arcs give a sound's full direction round the player, ahead,
behind and to either side, which stereo panning alone does not, as other games' sound
visualisers do for accessibility; they add nothing the speakers do not get. Drawn with
the HUD (`cg_drawHud`), following `cg_hudScale`. Unit tests: the bearing, the ring, the
filters, the fade and falloff, the arc's place and scale.

## Voice volume

`s_volumeVoice` (Settings > Sound, Voice volume; 0 to 1, default 1) is the loudness of
voices: every sound on the voice channels (`CHAN_VOICE`, `CHAN_VOICE_ATTEN`,
`CHAN_VOICE_GLOBAL`): pain, death, taunts, jump and land grunts, gasps and a voice
command where its speaker stands (its radio copy in the listener's ears is on
`CHAN_MENU1`, an effect, `cg_event.c` `EV_VOICECMD_SOUND`). As in retail, voices take it in place of `s_volume` (the effects volume), not
on top of it: `S_PaintChannels` mixes them at `s_volumeVoice * volume` and the rest at
`s_volume * volume` (`snd_mix.cpp`), and the OpenAL path sets their gain from
`s_volumeVoice` alone (`snd_dma.cpp`). Until 26/10/2026 SJK registered the cvar but
mixed voices at `s_volume` (Rollembones, hub report #141), so at the defaults (effects
0.5, voices 1) voices now sound twice as loud as before against the effects, as in
retail and EternalJK.

The sound adapter puts the three voice channels on one mixer channel
(`normalize_voice_channel`, as the game's channel stomp groups them), and the mixer
gives that channel its own gain (`Mixer::set_channel_gain`, a generic bus in
[sjk-audio](../crates/sjk-audio/src/lib.rs)): `AudioCommand::Gains` carries effects,
music and voice ([audio_controls.rs](../crates/sjk-viewer/src/audio_controls.rs)
`VOICE_CHANNEL`). Loops and music do not take it. The interface's own sounds (menu cues,
previews) play on channels past the game's (`interface_channel`), so none takes the
voice volume. A muted window silences voices too. Unit tests:
`a_channel_gain_replaces_the_master_gain_for_that_channel_only` (sjk-audio),
`voices_take_the_voice_volume_and_other_sounds_the_effects_volume` (viewer).

## Jetpack fuel bar

While the player's jetpack burns, a tall bar at the right edge of the screen shows the
fuel left in dark red, as JoF EJK's `CG_DrawJetpackFuel` draws it (20 by 100 units, 8 in
from the right, 260 down). It hides with the HUD (`cg_drawHud`) and follows the HUD scale
(`cg_hudScale`), keeping its right edge and centre height. In a
[devmap](#development-maps) game the `jetpack` command gives or takes away the jetpack;
as in the original game, the use key in the air switches it on (and off while it burns),
and jump climbs.
See
[status](status.md#jetpack-fuel-bar).

## Jetpack sounds

Settings > Audio (classic) and Settings > Advanced > Sound (SJK UI) offer
**Jetpack ignition sound** and **Jetpack hover sound**, matching JoF EJK #43.
Both default to `1`; `2` selects the alternate. The archived console cvars
are `cg_jetpackOnSound` and `cg_jetpackHoverSound`. As in EJK, values <= 1
use the default and values > 1 use the alternate; zero does not mute them.

| Choice | Ignition | Hover |
| --- | --- | --- |
| `1` (default) | `sound/chars/boba/bf_land.mp3` | `sound/chars/boba/jethover.wav` |
| `2` | `sound/chars/boba/jeton.wav` | `sound/chars/boba/bf_jetpack_lp.wav` |

Shutdown always uses `sound/chars/boba/bf_blast-off.mp3`; EJK has no shutdown
selector. The additional `sound/effects/fire_lp.wav` loop while flaming is
independent of the hover choice. A hover change applies on the next audio
frame; changing ignition during flight does not restart it. Choices apply to
every audible jetpack, including your own.

Start/stop sounds use the wearer's actual source and position on `CHAN_AUTO`,
including EJK #87's later attribution/channel correction. All variants are
preloaded/registered before frame playback; choosing them does not read or
decode sound assets during a frame. See [rendering](rendering.md) for the
existing pack, flame and visibility behavior, and
[status](status.md#jof-ejk-jetpack-sound-choices) for verification limits.

## Saber ignition and retraction sounds

Two sources voice a saber turning on or off, as in OpenJK `codemp`. The server
plays the hilt's own sound when a held saber is toggled and sends
`EV_SABER_UNHOLSTER` when an attack ignites a holstered one. Drawing the saber
from another weapon (fists, a gun), or putting a lit one away, is voiced by cgame
alone: `CG_CheckPlayerG2Weapons` for the local view and `CG_Player` for everyone
else compare the weapon whose model the player holds with the current one
(`cg_weapons.c:2498-2580`, `cg_players.c:8932-9001`). SJK mirrors that once per
rendered frame
([sound_saber_switch.rs](../crates/sjk-client/src/sound_saber_switch.rs)): the
local view from the predicted state in live play, from the snapshot's playerstate
in demos and when following, and other players from the presented snapshot. Each
hilt's `soundOn` or `soundOff` from `ext_data/sabers/*.sab` (`enemy_saber_on` and
`enemy_saber_off` when a definition gives none) plays on `CHAN_AUTO` from the
player's entity; a second saber adds its own, and a two-handed hilt drops a second
one. `EV_CHANGE_WEAPON` has no select sound for the saber, so nothing is doubled.
`EV_SABER_UNHOLSTER` plays the player's own hilts too; an NPC's keeps the stock
`saberon`, and so does a player whose clientinfo names no hilt (no server sends
one). A player wearing a [blade skin](#blade-skins) also has its skin's single
ignition or switching-off sound play once for the switch or the unholster, over
both hilts' sounds, which keep playing; each hilt's `soundOn`/`soundOff`, then
the defaults above. The server's toggle plays the stock sound and the skin's over
it, which the switch path never adds to, since the held weapon does not change.

cgame's quirks are kept. A single saber's ignition also plays `enemy_saber_on`,
the default sound of its removed second slot, as the server's toggle does. A
caught saber is silent, and so is drawing it again from fists after it was knocked
away. A player who comes into view or respawns holding a saber is silent, but one
who left view with a lit saber and returns with a gun is heard retracting it. The
local player hears the ignition on a map's first frame, on joining from
spectator, on respawning with a saber after dying with another weapon and on
choosing another hilt with the saber out. A severed arm, which takes the weapon
model off in cgame, is not modelled.

Unlike cgame, SJK caps how often one player's saber is heard turning on or off, so a
bound or scripted toggle spam does not fill the air
([sound_saber_toggle_limit.rs](../crates/sjk-client/src/sound_saber_toggle_limit.rs)):
four toggles play back to back, then one per half second while the spam goes on,
and the budget refills at that pace. The server's toggle (taken as the nearest
player's, as for blade skins), `EV_SABER_UNHOLSTER` and the switch path share it;
all the sounds of one toggle (both hilts, a skin's) play or are dropped together,
and NPCs are not capped. Only the sounds are dropped: the saber still lights and
goes out as the server says. A staff put away over a JA+ back
(`cg_holsteredStaffSound`, [audio_holstered_staff.rs](../crates/sjk-viewer/src/audio_holstered_staff.rs))
shares it too: the shutdown played at the reach pays for the toggle, and its weapon
switch copy, dropped there as the late one, gives back what it paid, so each
put-away is paid once whichever comes first, and a staff weapon switch loop is
capped like any other.

Saber definitions are resolved in the game's file-list order, keeping the first
case-insensitive name. Mounted replacements of the same `.sab` file still win
through the VFS. The shared catalog supplies appearance, sounds and prediction;
a later duplicate name must not replace its combat restrictions or special moves.

`fd.saberAnimLevelBase` is no wire field, so the client works it out for every
command it predicts, as `CG_PredictPlayerState` does before `Pmove`
(`cg_predict.c:1335-1347` in EternalJK and JoF EJK): the current style, unless one
blade or saber is off (`saberHolstered` 1), when it is the staff base for a first
saber with blades and the dual base for a second saber alone
([pmove_saber_base.rs](../crates/sjk-game-jka/src/pmove_saber_base.rs); the hands
come from clientinfo and the `.sab` files, `legacy_saber_hands`). Without it a half
staff or one saber of a pair counted as "sabers off": the swing became a draw, the
server's `saberHolstered` 1 undid it and `EV_SABER_UNHOLSTER` played again. This
follows the reference's rule from reading it; it has not been run against a live
dual or staff in the fast style.

## Animation sounds and voice variants

Footsteps and authored swing/spin sounds follow the evaluated lower/upper Ghoul2
frames and the model's `animevents.cfg`, including the shared skeleton table and
`include` directives. The client reads supported sound assets during appearance
loading and queues decoding on the audio worker; ordinary frame playback performs
no file reads. A world build shares the skeleton and event assets across matching
appearances. Active actors release their prefetched encoded bytes after registration.

The local player is shown the predicted animation, or the server's (a ping behind)
while a saber move is not predicted (`local_actor_state`). Its cues go through
`animation_events::Handover` (hub report #171, 27/10/2026): the server's animation
replayed the swing just heard and the way back to the predicted one landed past the
next swing's key, which then stayed silent. Each track remembers its last three heard
animations (clip and revision). The revision flips only when the same animation
restarts, so a clip seen again after another (A, R, A) counts as a new run, heard from
its own frame; handed back to the predicted one, a run heard past the predicted frame
is such an older run and is ignored. Handed to the server's, a track keeps back its cues
while the server shows one of them, until it reaches the frame already heard, or for
`HOLD_MILLIS` (1 s); an animation the prediction never played (a parry, a deflect, a
saber lock) is heard at once, and so is the server's as soon as it leaves the one held.
Handed back to the predicted one, it plays the keys the prediction passed unheard, at
most `CATCH_UP_FRAMES` (12) back. Looping clips (running, standing) are neither held
nor caught up, so footsteps carry on across the hand-over. During a blend the outgoing
clip's revision is taken from the track's transition.

Ground-contact events trace beneath the animated foot and select the authored
walk/run sound bank for the surface material. Ground the map gives no material
(no `MATERIAL_*` bits, which codemp plays as stone) takes its bank from the hit
shader's file name in SJK (`footsteps::material_from_name`: sand but not sandstone,
snow, grass, gravel, mud, dirt, carpet, wood or plank): retail leaves much of its
sand untagged, such as `mp/siege_desert`'s `siege/siege2sand`, where Sol asked for
sand steps (world note, 07/10/2026). The collision trace reports the brush side's
or patch's shader for this (`CollisionTrace::shader`); `cg_footstepGuess 0` turns the guess
off (stone, as retail), and it applies to dust and prints too. As in `_PlayerFootStep`, a
foot in water (`MASK_WATER`) makes no step. `cg_footsteps 0` mutes them. Frame
latches prevent repeated playback while an animation frame is held; absent actors,
teleports, backwards seeks and paused map changes reset the cursor. First-person
local actors use the same evaluated timing. This restores the blue-stance taunt's
spin sounds and authored melee/kick swing cues. Custom saber `spinSound` and
`swingSound1`–`3` override the standard animation samples.

Taunt, flourish and gloat voice choices advance per accepted event, with fallbacks
based on samples that actually resolved. The expanded taunt bank is used in FFA
as in TaystJK, rather than restricting ordinary FFA taunts to `taunt.wav`. Selection
is replay-stable but is not the legacy global random stream; repeats remain
possible. Animation selection, movement, saber timing and network events are
unchanged. Gameplay event actions remain outside this audio adapter.

Server-injected player sounds are read from authoritative snapshot external
events even while movement prediction runs. Held events play once, clearing to
zero rearms them, and switching the followed player seeds its pending event and
its last two predictable events (as CG_TransitionPlayerState does)
without replaying them, following JoF EJK #123. New events after the switch still
play from the new player. This changes audio latches, not movement or event encoding.

### Force presentation controls

Both menu styles offer **Force animation effects** (`cg_forceAnimFX`, on) for
Rage, Heal, Quick Heal and Grip-victim puffs. Plugin settings (`cp_pluginDisable`
bit 5) can also disable them and the combined cyan Protect/Absorb colour; the
separate green/blue shells remain. The local animation switch leaves the
Protect/Absorb colour preference and ordinary Push/Grip-caster effects alone.

**Empower glow** (`cg_empowerEffect`) defaults to -1, following the server's
`jp_empowerEffect`: 0 full body, 1 arms, 2 hands. Set 0/1/2 to override that shape,
or 3 to hide the Empower glow (SJK only; a server cannot ask for it).
Only Empower-marked body blur is trimmed; ordinary Push stays full body.
Mindtrick hides Force effects and flamethrower audio from the affected viewer;
active Sight reveals them. These presentation rules apply independently of menu
style and do not change Force gameplay or protocol encoding. Installed-asset
visual/audio comparison remains unverified.

### Repeated model voices

Settings > Sound exposes **Taunt anti-spam** in SJK UI and classic+.
`cg_tauntAntiSpam` defaults to 1, following JoF EJK #287/#374: each player's
entire taunt family shares a hold lasting the previous decoded sample's duration
(2500 ms when duration is unavailable). Gasps hold for 2000 ms; jump, roll and
landing voices each have an independent 1000 ms hold. Generic/entity custom-voice
events share these timers, so changing the event route does not bypass them.
The filter does not suppress animations, body/fall impacts, pain/death, weapons,
NPCs or world sounds. `cg_tauntAntiSpam 0` disables it. Map changes and backwards
demo seeks reset the holds, and so does a player slot whose `CS_PLAYERS` string
empties or takes another name, so a newcomer in a reused slot is not held by the
previous occupant's taunt. Duration comes from the decode worker; event playback
does not read files, decode samples or lock the audio callback.

### Map music synchronization

**Map music sync** (`cg_musicSync`, on) is in Sound settings in both menu styles.
After joining or leaving a private duel, file-based map music resumes from the time
elapsed since its track began instead of restarting. Like JoF EJK's
`cgs.musicStartTime` (EJK #159), that start is the level start for the map's own
track and the server time of the last `CS_MUSIC` change for a track set mid-level
(`target_play_music`); a numeric third `CS_MUSIC` token, when a server sends one,
states the start outright. The decode/callback queue delay is included in
the start position. The native sequencer plays a distinct intro once, then wraps
the loop; a single repeated file has the same phase as JoF EJK #161. Dynamic music
keeps its own transitions and does not use the file-track offset. Turning sync off
starts at the beginning. Manual `music` commands also start at the beginning.

An empty server `CS_MUSIC` means silence, including when a duel ends; it does not
fall back to BSP worldspawn music. Offline world viewing still uses worldspawn.
Map handoff stops the old track before preparing the next one (JoF EJK #103/#146).
Actual peer synchronization and live listening remain unverified; clock/sequence
checks use synthetic PCM and already-decoded snapshot time.

### Dust and footprints

A step on mud, dirt, sand or snow also kicks up dust and leaves a footprint, as
`_PlayerFootStep` does with retail's default `cg_footsteps 3` (`cg_players.c:2889-3086`,
[footprints.rs](../crates/sjk-viewer/src/footprints.rs)): the material's effect
(`materials/mud`, `materials/sand` for dirt and sand, `materials/snow`) along the ground's
normal, and the print (`footstep_r`, `footstep_l`, `footstep_heavy_r` or `footstep_heavy_l`
for a run, radius 6) turned with the legs. Gravel kicks up dust and leaves no print. A
print is a lasting cgame mark (`CG_ImpactMark` with `temporary` false): it is drawn for
ten seconds, darkening over the last one, at most 256 mark polys with the oldest mark
given up for a new one (`CG_AddMarks`, `CG_AllocMark`; the third list of
[decal_store.rs](../crates/sjk-viewer/src/decal_store.rs)), and `cg_marks 0` drops it.
The left foot's shader mirrors the right's picture with `tcMod transform -1 0 0 1 1 0`,
which effect stages now read as a mirror.

SJK saved `cg_footsteps` as an on/off `1`, which retail reads as "sounds only", so the
dust and prints have their own switch, `cg_footprints` (on; Settings > Sound,
"Footprints and dust"); `cg_footsteps 0` still turns off all three. Retail's untagged
sand (`mp/siege_desert`'s `siege/siege2sand`) only gets prints through the texture-name
guess above.

### Jump and landing steps

Retail's `_humanoid/animevents.cfg` steps in its side, back and Force landings but not
in the jumps or the forward landing (`BOTH_LAND1`), so a plain jump on sand makes no
sand sound. SJK adds both feet at the second frame of `BOTH_JUMP1`, `BOTH_JUMPBACK1`,
`BOTH_JUMPLEFT1`, `BOTH_JUMPRIGHT1`, the four `BOTH_FORCEJUMP*1` and `BOTH_LAND1`, as run
steps with their sound, dust and print (`JUMP_LAND_STEPS` in
[footsteps.rs](../crates/sjk-client/src/animation_events/footsteps.rs)), unless the
model's own table already steps somewhere in that animation. `cg_jumpFootsteps 0`
(Settings > Sound, "Jump and landing steps") keeps retail's steps alone.

## Mover models

A brush entity a map gives a `model2` key draws that model too: the game registers it as
the mover's `modelindex2` (`InitMover`, `SP_func_usable`, `g_mover.c`) and `CG_Mover`
draws it at the mover's origin and angles, scaled by `iModelScale` when set
(`cg_ents.c:3469-3479`). `t2_rancor`'s bar tables are such movers: `func_usable` clip
brushes, with no drawn face, carrying `models/map_objects/nar_shaddar/table01.md3`, so
until 13/10/2026 SJK showed no tables at all (Sol's world note, 08/10/2026). The rule is
`legacy_mover_model2` in [entity_models.rs](../crates/sjk-client/src/entity_models.rs);
[mover_models.rs](../crates/sjk-viewer/src/mover_models.rs) loads the models (the
baselines' at map load, later ones on first use) and draws them with the rigid entity
models, for the snapshot's movers and the permanent ones. `EF_NODRAW` hides the model as
it hides the brush. Multiplayer ignores a `func_usable`'s `modelAngles` (single player's
key) and leaves out a `.glm` `model2`; so does SJK. Tables that start off
(`spawnflags 1`) stay hidden unless a script brings them back.

The native server sends a `func_usable` the same way: at its origin (`s.pos.trBase`),
with its `model2` registered, solid to players and traces where it stands (it used to be
neither placed nor solid), and the use key fires one only when the map made it
`PLAYER_USE` (spawnflag 64) and not `INACTIVE` (128), as `ValidUseTarget` checks. One that
starts off is not sent (`SVF_NOCLIENT`, `EF_NODRAW`); it used to carry `EF_PERMANENT`
(bit 7) in the place of `EF_NODRAW` (bit 8).

Used, by the use key or by name (`G_UseTargets`, a `target_counter`, a breakable's
`target`), a `func_usable` runs `func_usable_use`: it goes away (no contents, not sent)
and fires its `target`, or comes back solid and fires its `target2`; an `ALWAYS_ON` one
(spawnflag 8) fires its `target` and is out of use for its `wait`, for good without one.
Until 17/10/2026 a named use never reached one, so a wall a map took down after an
objective (siege_korriban's red crystal, as reported) stayed solid. Not ported: the
shader-frame step of one with `endframe`, its use script, spawnflag 16's wait for a
clear space, and one used by being shot (`health`).

## Door and mover sounds

A brush entity (a door, lift, button or breakable) has its entity origin at the
world origin unless the map gave it an origin brush, so codemp plays its sounds
from the middle of its model (`CG_SetEntitySoundPosition`, `cg_ents.c:118-132`).
SJK does the same: an event sound on a brush entity, such as a door's start and
end sounds (`EV_PLAYDOORSOUND`), and the position the mixer moves the entity's
sounds to on every snapshot are its origin plus its inline model's midpoint
(`LegacySoundAdapter::sound_origin` in
[sound_events.rs](../crates/sjk-client/src/sound_events.rs), with the midpoints
taken from the map when its sound tables are built). A mover's loop already came
from the same point. That origin is the entity's `pos.trBase`, where codemp
evaluates its trajectory, so a door that turns back halfway sounds from where
its move began.

The start and end sounds come from the mover's soundset (`CS_AMBIENT_SET`): `G_PlayDoorSound`
raises `EV_PLAYDOORSOUND` (71) on a door, plat or button with one, and scripts and
breakables raise `EV_BMODEL_SOUND` (73); both play the soundset's sound for the stage, and
`EV_PLAYDOORSOUND` nothing without a soundset (`CG_PlayDoorSound`). Until 12/10/2026 SJK
answered only 73, taking it for the door event, so doors with a soundset were silent on
every server (Creyon's report: some maps' doors).

## Vehicle gunfire

A vehicle's own `EV_FIRE_WEAPON` and `EV_ALT_FIRE` (an `ET_NPC` of `CLASS_VEHICLE`)
play nothing, as in `CG_EntityEvent` (`cg_event.c:2751-2760`, `:2779-2784`): they used
to play the flash sound of whichever ordinary gun the vehicle's `weapon` field named.
A rider's own events are unchanged. Stock gives a vehicle's guns their sound through
the `muzzleFX` effect that `EV_VEH_FIRE` plays bolted to the vehicle
(`CG_VehMuzzleFireFX`), and SJK plays it since 22/10/2026
([vehicle_fire.rs](../crates/sjk-viewer/src/vehicle_fire.rs)): the event (on the
volley's last projectile, or a temp entity) names the vehicle in `owner` and the
muzzles that fired in `trickedentindex`; each fired muzzle plays the `.vwp` `muzzleFX`
of its weapon (`weapMuzzleN`, else the turret whose `turretNMuzzleM` names it) at its
`*muzzleN` (else `*flashN`) bolt as the vehicle is posed that frame, facing along the
bolt. A vehicle not drawn that frame plays it where the event entity is. Stock bolts
the effect so it follows the muzzle; here it starts there. The effect's `Sound`
primitive is the gun's firing sound, played like every effect sound at full volume
(`S_StartSound` with `SOUND_MAXVOL`; it was seven tenths before, so every effect's
sound is now as loud as stock). Kushina's report (21/10/2026): vehicle fire was quiet,
the projectile's flight loop and impacts being all that was heard. JoF EJK skips
these flashes while the viewer is in a private duel; stock does not, nor does SJK.

A vehicle weapon's shot that hits plays the `.vwp` `impactFX` alone, in place of the
impact of the hand weapon it is carried as (`CG_VehicleWeaponImpact`, called by
`EV_MISSILE_HIT`, `EV_MISSILE_MISS` and `EV_MISSILE_MISS_METAL`): a missile flagged
`EF_JETPACK_ACTIVE` names its vehicle weapon in `otherEntityNum2`
([impact_events.rs](../crates/sjk-client/src/impact_events.rs),
[impact_spawn.rs](../crates/sjk-viewer/src/impact_spawn.rs)). The game sends any shot
with a `width` or `height` as `WP_ROCKET_LAUNCHER`, so before 03/11/2026 the AT-ST's
red laser (`atst_laser`, 8 by 8) burst in a rocket's explosion instead of
`atst/wall_impact` (Kushina's report). A weapon without `impactFX` keeps the carrier's.

A destroyed vehicle (`EF_DEAD`, until it blows up: the AT-ST's `explosionDelay` is 2.45
s) burns: `effects/ships/dest_burning.efx` at its origin every 50 ms, as the dead branch
of `CG_VehicleEffects` plays it, animals excepted
([vehicle_burning.rs](../crates/sjk-viewer/src/vehicle_burning.rs)). A vehicle still
running shows no damage in stock but a fighter's: its damaged surfaces (`brokenLimbs`)
smoke with the `.veh` file's `injureFX` and `dmgFX` at `*nosedamage`, `*exhaust1`,
`*r_wingdamage` and `*l_wingdamage`, which SJK does not draw yet, nor the trails and
exhausts (`trailFX`, `exhaustFX`, `turboFX`) of the same function.

## Electrified players' crackle

A player whose `ps.electrifyTime` (`emplacedOwner` on other players) is ahead of the
clock wears the electric shell and crackles: on each frame the shell is drawn, one draw
in ten starts `sound/effects/energy_crackle.wav` on the player (`CG_Player`,
`cg_players.c:11060-11098`). A DEMP2 hit sets that time, and a JoF server sets it for
about four seconds when an admin logs in with `amlogin` (its only login sound: the rest
is a print). SJK had the shell but not the sound. It now plays it, your own in first
person too, but rolls on the 8 ms effect cadence instead of every rendered frame, so it
crackles about seven times a second at any frame rate, as stock does at
`com_maxfps 125`. A muted player and one hidden from you by a mind trick make no crackle
([electrify.rs](../crates/sjk-viewer/src/electrify.rs)). JoF EJK's skip while a saber
deflects lightning (`CG_LightningDeflectionActive`) is not followed, as the shell does
not follow it either.

## Bright flashes

Settings > Game options (SJK UI: Advanced > Gameplay, Effects; Simple > Gameplay) has
**Reduce bright flashes** (`cg_reduceFlashes`, off by default), added for photosensitive
players after Piza's hub report #138 ("a BIG white flash when two sabers clash"). On,
nothing bright flashes over the view:

- no saber clash flare, the burst of `gfx/effects/saberFlare` drawn over the screen where
  sabers meet (`CG_SaberClashFlare`, 150 ms, up to 2.35 times 600 virtual units across
  close by: at a few metres it covers most of the screen);
- no effect `Flash` primitive (`CFlash`, [effect_flash.rs](../crates/sjk-viewer/src/effect_flash.rs)),
  the white screen flash of a DEMP2 alt shock, a rocket, a concussion blast and the
  AT-ST's and ships' explosions (every retail effect with one);
- no effect particle drawn with the clash flare's picture. Retail's saber block effect
  never uses it, but packs that remake the block as a flash do (JoF's
  `JoF_HDMovieSaberClash.pk3` makes `effects/saber/saber_block.efx` one 55-unit flare of
  it at every block), so it goes whatever pack supplies the effect.

The clash flare also follows EternalJK's switches (JoF EJK `CG_SaberClashFlare`):
**Saber clash flare** (`cg_saberClash`, on) and **Clash flare size**
(`cg_saberClashSize`, 0 to 2, 1; the picture's size times it, 0.001 at least as
EternalJK's). Reduce bright flashes wins over both. The viewer reads them once a frame
([saber_clash_flare.rs](../crates/sjk-viewer/src/saber_clash_flare.rs)); the effect
spawner and submission read the reduce switch from an atomic the frame sets.

**Blade core thickness** (`cg_saberCoreSize`, archived, 0.5 to 2, default 1; Settings >
Game options, under Clash flare size) draws the white core down a lit blade thinner or
thicker: retail's core radius (a third of the blade's, with its flicker, `CG_DoSaber`)
times the setting, for every blade in the world, held or thrown, plain or skinned. The
glow, its hilt sprite and a skin's glints keep their size, and 2 still keeps the core
inside the glow. Previews in menus (Profile, the Collection, the forge) keep retail's
core. JoF EJK's nearest is `cg_shaderSaberCore` (not archived, 0.625), which scales
the core of its SFX blades only; SJK draws no SFX blades, so the setting has a name of
its own ([saber_core.rs](../crates/sjk-viewer/src/saber_core.rs)).

**Saber hit sparks** (`cg_fleshSparks`, archived, 0 to 7, default 7; Settings > Game
options, Effects) is EternalJK's count of the red sparks a saber hit on a body plays
(JoF EJK `cg_event.c`, `EV_SABER_HIT`, each `FX_PlayEffectID` behind
`cg_fleshSparks.integer > N`). At 7 every hit plays what the original game plays: six
`saber/blood_sparks_mp` for the special hit (`eventParm` 16), one
`saber/blood_sparks_50_mp` for a really big hit (2), one `saber/blood_sparks_25_mp`
for a moderate hit (3) and three `saber/blood_sparks_mp` for any other body hit.
Lower, a plain body hit plays two sparks at 6, one at 5 and none from 4; the moderate
hit's spark goes from 3, the big hit's from 2; the special hit keeps one spark at 1,
without its sound (every other hit keeps its sound), and none at 0. A saber hit on something else
than a body keeps its `saber/saber_cut`. Read once a snapshot
([impacts.rs](../crates/sjk-viewer/src/impacts.rs) `saber_hit`, the sound in
`LegacySoundAdapter::set_flesh_sparks`). Unit tests:
`flesh_sparks_gate_each_saber_hit_on_a_body`,
`flesh_sparks_below_two_silence_only_the_special_saber_hit`. Not tried in a game.

Compared with retail: the flare's timing, size, the behind-the-view and wall tests
follow `CG_SaberClashFlare` exactly ([sjk-client saber_clash_flare.rs](../crates/sjk-client/src/saber_clash_flare.rs)),
and a `Flash` follows `CFlash`'s distance and direction scale. One difference is not
settled: retail draws the flare in 2D after the scene, SJK draws it as a quad in the
scene's effect pass, so with bloom on (`r_sceneBloom`) it may bloom where retail's never
does. Not compared side by side in game. Unit tests:
`the_flare_follows_its_settings_and_reduced_flashes`,
`only_the_clash_flare_picture_is_a_flash_picture`.

## Renderer settings

SJK's own cvars have engine names: rendering ones are `r_*` (`r_sceneHdr`,
`r_toneCurve`, `r_sceneBloom`, `r_superSample`, `r_renderScale`, `r_actorSunShadows`,
`r_sunShadow*`, `r_dayNight`, `r_liveLighting`, ...), the ground HUD is
`cg_groundHud` and the dedicated server's are `g_npcNav` and `g_stockRules`.
The ground HUD (health, shield and Force at the player's feet in third person)
stands in for the status HUD; a zoom that shows the view in first person (the
disruptor's scope, the binoculars) keeps the status hidden rather than bringing it
back for the zoom (`Presentation::Covered` in `ground_hud/readout.rs`).
Names rend2 or EternalJK use with another meaning are avoided.

These rendering cvars have their own four groups. The SJK UI's Settings
gathers them in its Graphics category; with the classic menu style they are
panels of the Setup page's GRAPHICS page (in the main menu and the in-game
pop-up), after Video. A renderer entry without a panel of its own opens the SJK
UI's Settings on Graphics, and backing out shows the classic panel again (in
game too); the old tabbed renderer page reached from a "Renderer" row at the end
of VIDEO is gone (10/10/2026). The groups are:

| Tab | Settings |
| --- | --- |
| IMAGE | HDR scene and exposure, eye adaptation (`r_autoExposure`) and its range in EV, filmic tone curve, bloom, dynamic glow (`r_DynamicGlow` 0-3), No glow on sabers (`r_dynamicGlowNoSabers`) and the glow's blur style (`r_dynamicGlowStyle`), FXAA, supersampling (`r_superSample`), render scale (`r_renderScale`, also in the SJK UI's Simple Display; see [Render scale](rendering.md#render-scale-and-supersampling)), soft particles, ambient occlusion (`r_ssao`), sunbeam dust (`r_dustMotes`), per-pixel model lighting, toon models (`r_toonModels`, see [Toon models](rendering.md#toon-models)), reflection probes (`r_cubeMapping`), floor mirrors (`r_floorReflections`), normal and specular maps (`r_normalMapping`, `r_specularMapping`), parallax mapping (`r_parallaxMapping`) and its depth (`r_parallaxStrength`), emission maps (`r_emissiveMaps`), their strength (`r_emissionStrength`) and glow halo (`r_emissiveGlow`) |
| LIGHTING | Sun and sky (`r_dayNight`), live lighting tier, time of day, time of day from the PC's clock (`r_dayClock`), day length, sunlight brightness, ambient fill and its corner shading, indirect boost, emission-map lights (`r_emissiveLights`), light shafts (`r_volumetrics`) and their clarity |
| SHADOWS | World and character sun shadows, shadow resolution, sharp and close cascade distances, filter taps, slit closing, contact shadows |
| WEATHER | Weather (`r_weather`), its density (`r_weatherDensity`), quality (`r_weatherQuality`), forced weather (`r_weatherForce`), ground fog (`r_weatherFog`), volumetric clouds (`r_clouds`) and lightning in storms (`r_weatherLightning`); see [Weather](rendering.md#weather) |

Rows marked "(reload)" are read only when the graphics start and apply after a
[graphics reload](#graphics-reload); "(next map)" applies when a map loads; the
rest apply immediately.
Changing a value saves it like any other setting. Switches over numeric cvars
show ON/OFF and write 1/0. Defaults are unchanged (see
[Default visual profile](rendering.md#default-visual-profile)). Diagnostics such
as `r_dayDebug` stay console-only, as do the speeds and key of
[eye adaptation](rendering.md#eye-adaptation) and the distance inside which parallax
stops growing near the camera (`r_parallaxNearDistance`, [Parallax](rendering.md#parallax));
the ground HUD stays on the HUD
tab, and exclusive fullscreen (`r_exclusiveFullscreen`) stays on VIDEO's
display-mode row. Eye adaptation holds still while this page is open, so
exposure changes made here show at once instead of being eased.
See [catalog.rs](../crates/sjk-viewer/src/settings/catalog.rs).

### Graphics quality

The first row of VIDEO, of [First setup](#first-setup) (under Graphics), of the SJK
UI's Graphics (Advanced, under Quality) and of Simple's Graphics (under Preset) is
the graphics preset ([graphics_quality.rs](../crates/sjk-viewer/src/graphics_quality.rs)),
a dropdown of three presets (Sol's request, 16/10/2026; until then a Renderer row,
Vanilla or SJK (alpha), and an SJK quality row of four levels, from 12/10/2026):

- **Vanilla style**: the original game's look at the most frames per second, every
  SJK extra off, as EternalJK draws it with its defaults (the level called EJK from
  10/10/2026, Ultra low before, then Vanilla). See [Vanilla style and
  EternalJK](#vanilla-style-and-eternaljk).
- **SJK (recommended)**: Sol's own setup (as Sol had it on 16/10/2026): SJK's image
  (HDR, bloom, dynamic glow, FXAA, ambient occlusion, per-pixel model lighting,
  emission maps and their halo) over the maps' baked lighting, without the sun,
  shadows, reflections, material maps, light shafts or weather extras. A new
  profile (no `config.cfg` when SJK starts, applied in
  [app_launch.rs](../crates/sjk-viewer/src/app_launch.rs)) starts on it, and
  Backspace on the row returns to it.
- **Experimental**: everything the SJK renderer draws (sun and sky, sun shadows,
  reflection probes, floor mirrors, normal, specular and parallax maps, light
  shafts, the richest weather), still in alpha. Its values are the renderer cvars'
  registered defaults, the [default visual profile](rendering.md#default-visual-profile)
  the renderer was tuned for (the former High).

The volumetric clouds (`r_clouds`, Settings > Graphics > WEATHER, "Volumetric clouds") are
no preset's (Sol, 05/11/2026): off by default and left as the player sets them under every
preset, so choosing a preset never turns them on or off. Until then Experimental turned them
on; the release that changed it asks players once on its what's-new card (`= r_clouds
reset`), starting from off.

While the settings are not Vanilla style's, the detail column of every renderer
setting carries the alpha warning in gold ([sjk-ui.md](sjk-ui.md#settings)).

The preset is not saved on its own: the row shows whichever preset the settings
match, and Custom once one of them has been changed on its own; stepping (Left,
Right) from Custom starts at the preset the settings are nearest, and a step stops
at either end. A renderer setting's row reads its value in the preset in use (or,
for Custom, the nearest) as its default (`graphics_quality::row_default`), so a
preset's rows do not carry the changed dot, a row changed on its own does, and its
Backspace leads back to the preset. The `graphicsquality` command names the preset,
or sets it with `graphicsquality vanilla|sjk|experimental` (a preset's name works
too; `ejk` and `ultralow` still name Vanilla style). Profiles on one of the former
levels read Experimental (High) or Custom (Performance, Balanced, Ultra).

| Setting | Vanilla style | SJK (recommended) | Experimental |
| --- | --- | --- | --- |
| HDR scene, bloom, FXAA, per-pixel model lighting | off | on | on |
| Soft particles | off | off | on |
| Ambient occlusion (`r_ssao`) | off | on | on |
| Dynamic glow (`r_DynamicGlow`) | off | on | on |
| Light flares (`r_flares`) | off | on | on |
| Sunbeam dust (`r_dustMotes`) | 0 | 0 | 1 |
| Reflection probes, floor mirrors | off | off | on |
| Normal, specular and parallax maps | off | off | on |
| Emission maps and their halo | off | on | on |
| Sun and sky (`r_dayNight`) | off | off | on |
| Emission-map lights (`r_emissiveLights`) | 0 | 0 | 1 |
| Light shafts (`r_volumetrics`) | 0 | 0 | 3 |
| World and character sun shadows | off | off | on |
| Shadow resolution | 1024 | 1024 | 2048 |
| Shadow filter taps | 8 | 8 | 16 |
| Weather density | 1 | 1 | 2 |
| Weather quality | 0 | 0 | 2 |
| Volumetric clouds | off | off | on |

### Vanilla style and EternalJK

Vanilla style follows EternalJK's defaults on its default renderer, `rd-eternaljk`
(built from `codemp/rd-vanilla`; JoF EJK's `cl_main.cpp` `DEFAULT_RENDER_LIBRARY`),
surveyed on 26/10/2026 (Sol's request) from JoF EJK's `rd-vanilla/tr_init.cpp`,
`shared/sdl/sdl_window.cpp` and `cgame/cg_xcvar.h`:

| EternalJK default | SJK under Vanilla style |
| --- | --- |
| `r_DynamicGlow 0` | Off (the preset) |
| `r_flares 0` | Off (the preset; SJK's own default is 1) |
| No HDR, bloom, FXAA (`r_ext_multisample 0`), SSAO, soft particles, material or emission maps, sun, shadows or light shafts | All off (the preset); emission maps are SJK's own `_e` textures |
| Lighting per vertex on models | `r_modelPixelLight 0` (the preset) |
| The map's weather at the original density | Density 1, quality 0 (the preset); no clouds (`r_clouds` 0, every preset's default) |
| `r_picmip 0`, `r_textureMode GL_LINEAR_MIPMAP_LINEAR`, 16x anisotropy, no texture compression | SJK's defaults already |
| `r_dynamiclight 1`, `r_drawfog 2`, `r_surfaceSprites 1`, `r_gamma 1` | SJK's defaults already |
| `cg_shadows 1`, `cg_marks 1`, `cg_saberTrail 1`, `cg_fov 90` | SJK's defaults already (not graphics settings) |

What SJK cannot match: rd-vanilla's own cvars (`r_overBrightBits 0`,
`r_mapOverBrightBits 0`, `r_lodbias`, `r_lodscale 16`, `r_subdivisions 4`,
`r_ambientScale 0.6`, `r_texturebits`) have no counterpart in SJK's renderer, which
draws the maps' lighting in its own way. Vertical sync (`r_swapInterval`) and the
window are left to the player, as for every preset. Sol's own JoF EJK profile differs
from these defaults: rd-vulkan with dynamic glow and 8x multisampling.

Under Vanilla style and SJK (recommended), with no sun, light shafts or sun shadows,
a map loads without building its lamps (see [rendering](rendering.md#default-visual-profile)), which no pass reads there.

The `renderer` command keeps the former Renderer row's switch: `renderer vanilla`
keeps the player's values of every setting in the table (`r_ultraLowRestore`,
archived, its name from before the renames) before applying Vanilla style;
`renderer sjk` puts them back, custom values included, or SJK (recommended) when
Vanilla style was picked as a preset and nothing was kept. The [low-FPS
help](#low-fps-help) offers the same switch. Like the preset, it needs a [graphics
reload](#graphics-reload) for the lighting to change. Everything else is left as the
player set it: the FPS cap, vertical sync, resolution, supersampling and render scale
(which multiply or divide the pixels drawn, so no preset sets them), the taste settings (exposure,
time of day, tone curve, glow style, parallax depth, emission strength), contact
shadows (still being worked out) and whether weather and fog show at all. Rows
marked "(reload)" apply after a graphics reload, so a new preset's shadows, light
shafts, HDR, FXAA, sun and sky, reflection probes and material maps show once the
graphics reload, and emission-map lights on the next map (or the reload, which loads
the map again); the rest change at once. A preset is saved with one write of the
profile. How many frames each preset gains on a given GPU is not yet measured.

### Graphics reload

Some renderer settings are read only when the graphics start: HDR scene, FXAA,
supersampling, render scale, sun and sky, sun shadows and their quality, light shafts, normal,
specular, parallax and emission maps, reflection probes and texture filtering,
so the graphics presets too. They apply with a
graphics reload rather than a restart of SJK
([graphics_reload.rs](../crates/sjk-viewer/src/graphics_reload.rs), [how it
works](rendering.md#graphics-reload)):

- `vid_restart` reloads the graphics when one of them changed, as it restarted
  the renderer in the original game (it still reapplies the display settings
  first). `vid_restart force` (Tab completes `force`) reloads them even when none
  changed, as the original game's `vid_restart` always restarted the renderer: the
  world is built again on a new graphics context with the same settings, in the
  same places and under the same limits as a reload with a change. Plain
  `vid_restart` still skips the reload when nothing changed (it only reapplies the
  display settings), and the reload card and the FPS help card, which queue it after
  a change, keep using it.
- When one of them changed and the player is back on the main menu or the game
  menu (not in Settings, so changing several rows is not interrupted), a pop-up
  card in the SJK UI's look, in every menu style, says what changed ("Vanilla
  graphics apply after a graphics reload", or the settings by name) with **Reload now**
  (Enter) and **Later** (Escape). Later puts it away until another such setting
  changes; it waits for the medal and holocron pop-ups and the console.
- On a server (or in a local game) the map loads again under the loading screen,
  as on a map change to the same map, in a few seconds (ffa3 took 5.6 to 7.2 s
  off-screen, see [status](status.md)); the connection stays and the player stands still meanwhile,
  so the server does what it does when a client goes quiet for that long (a door
  the player held open may close until the player is back).
- On the main menu the menu map loads again behind the card ("Reloading the
  graphics...", about 3 s for duel6), and the menu carries on over the new world.
  After a match the menu map is built again quietly, with the graphics of the
  moment, when the menus come back to it.
- Not during a demo, a map being loaded or joined, or a map explored without a
  server: there the graphics reload with the next map.

The console notices say so: `HDR setting changed: reload the graphics to apply
(vid_restart)`.

### Low FPS help

SJK's real-time lighting (lamps, sun, sky light, shadows, light shafts) is most of
what a heavy map costs: on `JoFTemple` at 4K it was 5.6 of the frame's 8.1 ms before
its floor fix, and turning its parts off one at a time barely changed it
([status](status.md#joftemple-frame-rate-and-ultra-low)). Vanilla graphics (EJK
graphics until 12/10/2026) turn it off as
a whole. The low-FPS help
([fps_help.rs](../crates/sjk-viewer/src/fps_help.rs)) offers them when it matters:

- Six seconds after a server's map is in (loading's last work is over by then), the
  GPU times of eight sampled frames, one every 32 frames
  ([gpu_phases.rs](../crates/sjk-viewer/src/gpu_phases.rs), timestamps when the GPU
  has them), give the median frame and the part of it spent in the `light-...`
  sections. The check is done once per map (each new world), not under Vanilla,
  and gives up 30 s after the map came in. Whether the graphics are Vanilla is read
  only while the check could still run, and again only once an archived cvar has
  changed (every preset value is one, `ViewerConsole::archive_revision`): reading
  the preset's 25 values every frame took 184 allocator calls a frame at the SJK
  preset (25/10/2026).
- It offers Vanilla graphics when the frame misses the player's FPS target and the
  lighting is at least 40% of it. The target is the FPS cap (`com_maxfps`, or the
  monitor's rate for auto), kept between 60 and 144: an uncapped player is offered
  them below 144 FPS of GPU time, not because the frame could be faster still. The
  log says what it found: `fps help: frame 8.07 ms, SJK lighting 5.64 ms, target 144
  FPS: Vanilla graphics offered` (or `fine`).
- The offer is an SJK pop-up card over the game menu, which it opens and closes
  again with it ([sjk-ui.md](sjk-ui.md#low-fps-help)): this map's numbers, what
  Vanilla graphics turn off, **Use Vanilla graphics** (Enter: the renderer choice,
  then `vid_restart`;
  the map loads again and the player stays on the server) or **Not now** (Escape),
  and a **Don't suggest this again** tick (a click, `cg_fpsHelp 0`). It comes in the
  middle of play, so Space, D and the other keys a player plays with choose nothing
  on it, and for 0.6 s after it opens it takes no key or click at all: a jump once
  switched to Vanilla graphics (hub report #194).
- `help_fps` opens the same card at any time, on the menus or in a match, with the
  numbers once the frames it times while open come in. With Vanilla graphics on, its
  button turns them off and the player's settings come back.

The 40% share and the 60 to 144 target were chosen from JoFTemple's numbers before
and after its fix (offered at 8.1 ms with 5.6 of lighting, fine at 3.6 ms); they are
not yet checked on other GPUs or maps.

## First setup

The tabbed settings screen's last tab, FIRST SETUP (called Quick setup before
06/10/2026)
([quick.rs](../crates/sjk-viewer/src/settings/quick.rs)), gathers the settings worth
choosing on a first start: first, under a "From another client" heading, Import from
a client ([Importing from another client](#importing-from-another-client)) and the
Gameplay FPS ([Gameplay FPS](#gameplay-fps)), which players coming from another
client ask about most (Sol, 11/10/2026); under a Graphics heading, the graphics
preset ([Graphics quality](#graphics-quality)); under a Styles heading, the menu style
(SJK or Classic) and the camera style (EJK or SJK, [Camera
style](#camera-style));
then resolution, display mode, vsync, field of view, mouse
sensitivity and inversion, always run, effects and music volume, the HUD look and
scale, the crosshair, the nameplates and their bars, Force bar and power icons, the
Force aura and the combined Protect+Absorb shell, and the update and identity
opt-ins, then its own "Don't show at start" row (`ui_hideFirstSetup`), ending in a
Key bindings row. Its other rows are the catalogue's own, looked up by cvar, so a
change there is the same change the other tabs make; it is the last tab so the
other tabs keep their numbers. The SJK UI pop-up's line under its title points at
the first row.

With the classic menus the same rows are the first group of the
Setup page, FIRST SETUP (`Group::Quick`, [groups.rs](../crates/sjk-viewer/src/settings/groups.rs)),
drawn as a classic+ option panel like the others, with search, descriptions and
defaults, and opens the FIRST SETUP tab only if that panel cannot open. The SJK UI shows
them as a pop-up card over the map (since 08/10/2026,
[sjk_popup.rs](../crates/sjk-viewer/src/settings/sjk_popup.rs),
[sjk-ui.md](sjk-ui.md#first-setup)): the rows scroll inside it, and "Don't show
at start" is a tick box pinned at its foot, always in view, beside All settings
(the Settings screen on its First setup category, which keeps the rows too) and
Done. Picking another menu style on its Menu style row keeps First setup on show in
the new style: the classic Setup page's FIRST SETUP group or the SJK UI's
pop-up.

At every start, the first time the main menu is up and the menu style is known, it
opens in the active style, until the player ticks "Don't show at start"
(`ui_hideFirstSetup` 1, archived); Escape leaves it for that run. (Before
07/10/2026 it opened once, on the first start, marked by `ui_quickSetup`, which is
no longer read.) The `firstsetup` console command opens it too, over the main
menu or from a running game, in the active style
([first_setup.rs](../crates/sjk-viewer/src/menu/first_setup.rs)). Its old name
`quicksetup` was removed on 08/10/2026, with no hidden alias, so `quit` is the only
command a `q` completes to. Not yet run in a game window.

### Importing from another client

First setup's first row, Import from a client (its `firstsetup import` row, in
every menu style), opens the Import page on the configs the clients installed beside
SJK wrote, newest first, at most four: retail Jedi Academy and JA+
(`jampconfig.cfg`), OpenJK (`openjk.cfg`), EternalJK and JoF EJK (`eternaljk.cfg`)
and TaystJK (`taystjk.cfg`), in any folder of GameData (portable installs) or of
`Documents/My Games/OpenJK`, `EternalJK` or `TaystJK`, SJK's own folder left out
([config_import.rs](../crates/sjk-viewer/src/config_import.rs) `detect`). Each row
names the client (and the mod folder when it is not the client's own), the player
name the config holds and the day it was saved; Enter or a click reads it. Configs
holding nothing to import are not offered. With none found, the row opens the
system's file dialog as before.

A `.cfg` file dropped on the window (winit's `DroppedFile`), at any time, opens the
Import page over whatever is on show (a picture file, PNG, JPEG or TGA, opens the
Profile page's picture panel instead, as does any other file dropped while that page
shows: [Identity](#identity))
([config_import.rs](../crates/sjk-viewer/src/config_import.rs),
[config_import_panel.rs](../crates/sjk-viewer/src/config_import_panel.rs)). The
page's Browse (B, or its footer button) opens the system's file dialog, listing
`.cfg` files ([file_dialog.rs](../crates/sjk-viewer/src/file_dialog.rs), on a worker
thread, as the profile picture's Browse), and the file chosen is read as a dropped one.
On Linux it asks `zenity`, then `kdialog`, before `rfd`'s desktop portal, which never
answers on desktops whose portal has no file chooser; while the dialog is open the
button reads Stop waiting and gives up on it, so a dialog that never showed no longer
leaves Browse greyed out. The screen's text says so. `firstsetup import <path>` does
the same from the console (a path's words may be left unquoted, or quoted as
Explorer's Copy as path gives them: the path is taken as typed, without the
console's backslash escapes, and one pair of surrounding quotes is dropped), and
`firstsetup import` alone opens the page without a file. The file is read, never run: only `seta`/`set`/`sets`/`setu`, a
bare `name`, `model` or `cg_fov`, `bind`, `unbind` and `unbindall` count (of the
cvars, `name`, `model`, the `char_color_*` tint, `cg_fov`, `com_maxfps` and JoF EJK's
`cl_cmdratecap`), `//`
comments are skipped, and a file that is not UTF-8 is read as Latin-1, as the
legacy game wrote names. Files over 1 MB, other extensions, and configs holding
none of the parts are refused with the reason.

The page lists the parts found, each ticked at first: Name (`name`), Model
(`model`, with `char_color_red`/`_green`/`_blue` when the file sets them), Field
of view (`cg_fov`), FPS cap (`com_maxfps`, the frames drawn) and Key bindings.
Space, Left/Right or a click ticks a row; Enter copies the ticked parts into the
profile (also logged to the console), then asks for the gameplay FPS with what the
file's frame rate meant for its movement picked ([Gameplay FPS](#gameplay-fps)):
Auto, which follows the FPS cap just imported (333 moves like 333), or 125 when it
had `cl_cmdratecap 1`, as JoF EJK moved at 125 whatever it drew. Use sets it, Skip
(Escape) leaves it; either way the page ends on everything that changed. A file that clears every binding first (`unbindall`, as a saved
`jampconfig.cfg` does) replaces the player's table: the keys the binding editor
locks keep their binding, and SJK's own actions get their default key back when
the file left both the action and its key free. A file without `unbindall` only
adds its keys. Bindings on keys SJK does not know are left out and counted. The
page is the SJK UI's pop-up card in every menu style, as Update's
([config_import_panel_sjk.rs](../crates/sjk-viewer/src/config_import_panel_sjk.rs),
[SJK UI](sjk-ui.md)). With no file yet or after a refused one, Enter browses.

### Gameplay FPS

In the original game the frame rate is also the movement: every rendered frame makes
one user command, and the movement code's rounding makes jumps, strafes and
acceleration differ at 125, 142, 250 or 333 FPS, so players lock `com_maxfps` at the
one they are used to. SJK makes commands on their own clock
([networking.md](networking.md#user-commands-and-move-packets)). The gameplay FPS
(`gameplay_fps`, archived) sets it: on Auto (0, the default) the clock follows the
FPS cap, so the movement is what the other clients give at that cap, and a server that
fixes the step (`pmove_fixed`) uses its own, and the servers known to cap commands
(JoF's, at 125 a second; the list in
[server_caps.rs](../crates/sjk-viewer/src/server_caps.rs)) keep their cap unless
Settings > Display > Ignore server caps (`gameplay_fps_nocap 1`) lifts it; a number from
20 to 1000 keeps that
movement whatever the FPS cap. Players kept asking Sol how the frame rate and the
movement relate (11/10/2026), so it is asked where they arrive: First setup's second
row, the step after an import, Settings > Display > Gameplay FPS beside the FPS cap
(Simple's Gameplay has it too)
(Left/Right step through Auto, 125, 142, 250 and 333, Enter opens the card), and the
`help_gameplay_fps` command ([gameplay_fps.rs](../crates/sjk-viewer/src/gameplay_fps.rs)).

The card ([config_import_panel_fps.rs](../crates/sjk-viewer/src/config_import_panel_fps.rs),
the Import page's step, an SJK UI pop-up in every menu style) explains it in three
lines, says what the imported config had and why that is picked, and offers Auto ("your
FPS cap" under it), the four presets (JoF EJK under 125, "your old one" or "in use"
under the matching one) and Other: typing digits (top row or keypad) fills Other with any value from 20
to 1000 and says what it moves like (400 moves in 2 ms steps, 500 a second, as stock
rounds `1000 / fps`). Left/Right choose, Enter uses, Escape cancels (Skip after an
import). Its notes: a `pmove_fixed` server moves everyone in its own step (named when
the current server does), and the frames drawn are the FPS cap, shown with its value.
When a server does not take the inputs at a gameplay FPS above 125 (snapshots keep
saying it ran none of our commands, or keep correcting the prediction;
[networking.md](networking.md#user-commands-and-move-packets)), the card opens by
itself, once per server and value, with a line in orange saying the inputs are not
going through at that rate and 125 picked.
`help_gameplay_fps` prints the explanation and the value in use to the console and
opens the card; `help_gameplay_fps <fps>` or `gameplay_fps <fps>` sets it (0 for Auto).
The settings row shows Auto with the cap it follows (`Auto (125)`), and the server's
step beside the value while a `pmove_fixed` server overrides it.

## Slider values

Every slider in Settings (including the classic Setup panels), the saber RGB
controls (including the second saber), and Camera control supports direct
numeric entry. Click its displayed value, select the row and press Enter, or
just type a number on the selected row: the draft starts with what was typed.
Enter applies it; Escape cancels. Left/Right, Home/End, Backspace and Delete
edit the draft. Space still steps a selected slider instead of opening entry.
Hovering does not move an edit to another setting. Text settings such as the
master server keep their edit on the row that opened it, and Enter writes that
setting.

Pressing anything else while a numeric draft is open applies the draft when it
is a valid number and discards an invalid one, and integer sliders round a typed
fraction (142.6 becomes 143).

Manual values respect the slider bounds but do not snap to its drag increment:
for example, the FPS cap accepts 142 and FOV accepts 97.5. A draft takes
digits, one decimal point (a comma counts as one), a minus sign only first and
only on sliders that go below zero (the FPS cap's `-1` is AUTO), and at most ten
characters; a held key does not repeat typed characters. Invalid or empty input
stays open with a red underline and does not change the setting. Dragging and
arrow adjustment outside editing retain their existing behavior. Values they
write are rounded to the step's decimals, so a 0.05-step slider stores 0.35
rather than 0.35000000000000003.

## Key bindings

Settings > Key bindings binds two keys per action: click a slot (or select it
and press Enter), then press a key or mouse button, or turn the mouse wheel to
bind `MWHEELUP` or `MWHEELDOWN` (a notch only scrolls the list when no slot is
waiting; trackpad deltas under half a notch do not bind). A key already bound
elsewhere moves to the new action. Escape, or a click with the left or right
mouse button, cancels a pending capture without binding anything; the click is
consumed, so it activates nothing under the pointer. Retail
(`Item_Bind_HandleKey`) bound any key pressed while waiting, so a stray click
could take `+attack` off `MOUSE1`.

`MOUSE1`, `MOUSE2` and `ESCAPE` are locked in the editor: they cannot be
captured, a slot holding one cannot be rebound or cleared there, and locked
keys are drawn muted. Reset defaults restores them. The console's `bind` and
`unbind` commands are unrestricted. See
[keybind_editor.rs](../crates/sjk-viewer/src/keybind_editor.rs).

The console can also bind an arbitrary pair of keys as one chord, using the
JoF EJK syntax `bind K+L kill`. Hold either key and press the other to run the
chord. The first key's ordinary bind has already run; the chord replaces the
ordinary bind of the second key. For a chord bound to a `+command`, releasing
that second key sends its matching `-command`. The order in the command does
not matter (`K+L` and `L+K` name the same chord); `unbind K+L`, `bind K+L`,
`bindlist`, `unbindall` and `config.cfg` handle it like any other bind. There
are at most 256 chords. A literal plus key can be the first key as `++K`.

The settings editor shows an existing chord as one occupied slot and can clear
or replace it, but captures one physical key at a time. Create a chord in the
console. This follows JoF EJK commit `b6623f51` (`codemp/client/cl_keys.cpp`),
including first-match table order when several held chords could fire.

Keyboard releases retire held chord keys before a newly opened menu or text
field handles them. Thus a chord whose command opens the chat composer, such as
`bind SHIFT+U reply`, does not leave either key held internally and cannot be
retriggered later by pressing only Shift or U. Releases without a corresponding
binding press are ignored rather than synthesizing a button release.

## Development maps

Run `devmap mp/ffa3` in the client console to start and join an owned local
FFA server with cheats enabled, no bots and no match limits. Other installed
maps work too, including `devmap t2_rancor`; `maps/` and `.bsp` are optional.
The command appears in console completion/help. It uses the same `sjk-dedicated`
binary lookup as Create game (`JKA_DEDICATED` overrides the adjacent binary).

This starts a fresh game on loopback, without master-server advertising. Once
launched, it replaces the current connection; it never asks a remote server to
change maps or allow cheats. Missing map names are reported before leaving the
current game. Disconnecting, cancelling the join, or exiting stops the owned
server. Ordinary Create game launches still leave cheats disabled.

The native server takes all of OpenJK's player commands, the cheats among them:
`noclip`, `god`, `notarget`, `give` (also an item by classname), `giveother`,
`killother`, `setviewpos`, `t_use`, `npc`, `levelshot`, `thedestroyer` and
`debugBMove_*`, plus `where`, `forcechanged` and `voice_cmd` (see its
[cheat commands](server.md#cheat-commands) and
[other client commands](server.md#other-client-commands)). Run `noclip`, `god` or
`notarget` again to turn it off; spawning again clears all three. Normal servers
still require their own cheat permission. A command the server does not know
answers `Unknown command`; mod commands such as JoF EJK's or JA+'s `amtele` are
not there, as the local server runs no mod.

## Invisible brushes

`r_showBrushes` draws the brushes a map is built with but never shows (clip,
trigger, ladder, nodrop…) see-through over the world, the way Radiant shows them:
each face in its kind's colour with a framed tile carrying its name, 64 units to
a tile, projected on the face's main axis as Radiant's default projection is, and
each face outlined. **0** is off (the default), **1** shows the kinds that change
play, **2** every kind. Settings > Game options > Map tools > Invisible brushes sets
it in both menu styles. `r_showBrushesTypes` replaces the set with the kinds it
lists (spaces or commas between names), for instance `r_showBrushesTypes "clip
trigger"`. It works on any server: the client loads every brush for its own
collision already.

| Name | Shows | Set |
| --- | --- | --- |
| `clip` | Player and NPC clip (`textures/system/clip`) | 1 |
| `playerclip` | Player clip only (`blockplayer`) | 1 |
| `npcclip` | NPC clip only (`blocknpc`) | 1 |
| `shotclip` | Stops shots, not players (`shotonlyclip`) | 1 |
| `solid` | Solid with no drawn face: invisible walls (`physics_clip`, `nodraw_solid`) | 1 |
| `trigger`, `teleport`, `push`, `hurt` | `trigger_*` entities, by classname; other trigger-textured brushes are `trigger` | 1 |
| `ladder` | Ladders | 1 |
| `nodrop` | Death pits and other nodrop volumes | 1 |
| `slick` | Slick floors | 1 |
| `cushion` | No falling damage (`cushion`) | 1 |
| `door` | `func_door` and `func_door_rotating` brushes | 1 |
| `lift` | `func_plat` | 1 |
| `button` | `func_button` | 1 |
| `train` | `func_train` | 1 |
| `endless` | `func_rotating`, `func_pendulum` and `func_bobbing`, which never stop | 1 |
| `weather` | `outside` / `inside` weather volumes | 2 |
| `botclip` | Bot navigation hints (`do_not_enter`) | 2 |
| `caulk` | Caulk faces | 2 |
| `nodraw` | Every other face that draws nothing (hint, skip, areaportal, origin…) | 2 |
| `breakable` | `func_breakable` and `func_glass` | 2 |
| `travel` | The box a door, lift or button moves through, from `reach()` in [mover_occlusion.rs](../crates/sjk-viewer/src/mover_occlusion.rs) | 2 |

A face counts when its shader has `surfaceparm nodraw`; its kind comes from its
brush's contents and its own surface flags as q3map2 compiled them
([kinds.rs](../crates/sjk-viewer/src/brush_view/kinds.rs)). Caulk and an invisible
wall share their flags, so a shader whose name contains `caulk` is caulk. Fog, water,
lava, slime and terrain volumes are left out, as they show anyway. Retail does not
ship Radiant's `textures/system` editor images, so the tiles are SJK's own, written
with the bundled Rajdhani Bold.

The faces are rebuilt from each brush's planes
([brush_winding.rs](../crates/sjk-bsp/src/brush_winding.rs)) the first time the view
is switched on for a map, never before, and drawn in the overlay pass after the
world: tested against the scene depth without writing it, lifted 0.35 units off
their plane toward the camera so a clip brush flush with a wall still shows, from
both sides ([brush_view.rs](../crates/sjk-viewer/src/brush_view.rs),
[brush_view.wgsl](../crates/sjk-viewer/src/brush_view.wgsl)). Brushes of entities
of other entities are left out (the map draws them). Movers are drawn once, where the
map file puts them, not where they are now: a door open at that moment still shows
closed, and its `travel` box shows how far it goes. A brush model with an origin brush
(rotating doors, trains) is stored relative to its `origin` key, so its faces are shifted
back by it; a `func_train` shows where the map file puts it, not at its first path
corner. A brush behind a wall stays hidden behind it. The view is off on the menu map.

Checked 11/10/2026 with world shots (`world_shot_brushes.rs`, `SJK_BRUSH_MAPS`,
`SJK_BRUSH_MODE`) on `mp/ffa3` (clips, a trigger, solid) and `mp/ffa5` (its death pit
as `hurt`, clips); not yet in a game. `mp/ffa5` wraps much of its detail in
`physics_clip`, so `solid` shows white over those walls at 1: that is where players
collide.
The browser map editor shows the same faces, sorted by the same code, plus liquid
volumes ([map-edits.md](map-edits.md#the-browser-editor-creator-only-for-now)).

## Map entities

`r_showEntities` marks the point entities a map file places, through walls, with
their class names (and target names; a light adds its intensity). **0** is off (the
default), **1** shows lights, spawns, origins, items and targets, **2** every kind.
`r_showEntitiesTypes` replaces the set with the kinds it lists (spaces or commas), for
instance `r_showEntitiesTypes "light spawn"`. Settings > Game options > Map entities
sets the mode. It works on any server: the entity lump is in the BSP the client loads.

| Name | Shows | Set |
| --- | --- | --- |
| `light` | `light` and `light_*` | 1 |
| `spawn` | `info_player_*`, `team_CTF_*`, `info_jedimaster_start` | 1 |
| `origin` | `info_null`, `info_notnull`, `target_position`, `target_location`, `path_corner`, `ref_tag` | 1 |
| `item` | `item_*`, `weapon_*`, `ammo_*` | 1 |
| `target` | every other `target_*` | 1 |
| `sound` | `target_speaker`, `ambient_*` | 2 |
| `effect` | `fx_*` | 2 |
| `misc` | `misc_*` (props, models) | 2 |
| `other` | any other entity with an origin and no brush model | 2 |

The kind comes from the class name prefix ([entity_view.rs](../crates/sjk-viewer/src/entity_view.rs));
brush entities (`*N` models) are `r_showBrushes`'. A marker is a coloured square on the
entity's origin; the 60 nearest within 1500 units carry their label, and at most 220
within 6000 units are drawn. The markers are drawn in the HUD layer, so they are not
depth tested and show through walls. The list is read from the entity lump the first
time the view is on for a map.

A world note picks a marker: the one the view ray passes within about 1.4 degrees of
beats the wall or model behind it. The note records the entity (number, class name,
target name, origin) and the outline is a box round its origin.

## Talk balloons

Opening chat, the console or a menu sends the stock talk button and disables
other movement input while that keyboard catcher is active. A window you alt-tab
away from or minimize sends an automatic away bubble too. **Chat bubble while
away** in SJK Settings > Chat (Advanced; classic menus: HUD) controls both away
states through `cl_unfocusedChatbox` (archived, default 1). Set it to 0 to disable
the automatic bubble in either state. Actual open chat, console and menu key
catchers keep their normal talk flag.

The separate `cl_minimizedChatbox` setting is removed. Old saved entries of that name
are discarded on profile load; the saved `cl_unfocusedChatbox` choice is used
for both states. The update's what's-new card offers this shared switch.

Players carrying the talk flag have a chatbubble over their heads; the connection-trouble icon
takes priority when the server marks a lost connection. These are upright frame
billboards using the existing [sprite orientation](rendering.md#billboard-icons).
Their texture opacity is preserved near walls even with soft particles enabled.
A chat field open over a map change stays open, its draft and channel with it, and
so does the balloon, as in EternalJK, where a server's map change resets no key
catcher (Nib's report #134): the field goes to the new map's chat, and the commands
sent while the map loads carry the talk button while it is open
([resident_world.rs](../crates/sjk-viewer/src/resident_world.rs)). The chat box's
lines start again with the new map, as a new map's cgame starts its chat box empty.
Your own bubble is visible in third person, not the first-person view. Mind-tricked
players, NPC talk flags and intermission do not show talk balloons. The balloons,
pickup icons and hook ropes have their own share of the effect particle pool, so a
scene full of effects no longer hides every balloon at once. Siege voice command
icons remain unimplemented. See
[player_sprites.rs](../crates/sjk-viewer/src/player_sprites.rs) and
[pmove_talk.rs](../crates/sjk-game-jka/src/pmove_talk.rs).

## Joining and changing maps

A join from the menu prepares the destination world while the connection runs:
map preparation and connection are independent, and the loading screen (the
classic one or the SJK UI's) shows how far both are. The world is adopted once it
is built from the joined session's own gamestate with the session in hand, so
the player never walks a preview; a matching verified world is reused when the
server session becomes ready, and joining does not build it twice. Escape or a
click on the loading screen cancels.

On a server map change, gameplay pauses and a loading notice is shown over the
previous view until the destination and a fresh active snapshot are ready. The
client then adopts the server's spawn state. Match-end intermission uses the
server's normal camera, scoreboard and ready-to-exit controls. Player bodies and
vehicles are hidden there, matching codemp; scripted non-vehicle NPCs remain visible. Local gameplay
continuation on the old map is suspended while that feature is developed further.
The viewer no longer prepares an in-process native game for every loaded world.

CPU map preparation and GPU resource installation remain on background workers,
using the existing GPU context. Archive checksum inventory reads ZIP directories
without decompressing every asset. Matching same-map restarts can still reuse
the prepared world, and the join adopts its already-built destination. The
waiting connection sends neutral commands during map loading and is kept separate
from the displayed old world, so new-map entities cannot appear in the wrong BSP.
Texture mip preparation and lamp extraction use bounded CPU workers; repeated
texture loads can reuse a bounded process-local mip cache. See
[load-time rendering preparation](rendering.md#load-time-texture-and-light-preparation)
for cache limits and unchanged output semantics. Server readiness still
contributes to the time before play begins.

First entry into the destination before a server player exists retains
movement-only exploration with frozen brush collision.

Server console output (`print`) goes exclusively to the console, including match
statistics, command replies and server announcements. It never enters chat history.
Each line of a print becomes its own console row, without the empty row a
server's closing newline would leave. Global, team and private chat retain their
conversation overlay, and are also kept in the console scrollback but not among
its notify lines, as stock cgame echoes chat with the `*` print prefix that
`CL_ConsolePrint` keeps out of the notify area. Center-print
gameplay notices keep their separate HUD presentation. As `CG_DrawCenterString`
does, a centre-print row longer than 50 characters wraps at its last space; retail
counts bytes of its one-byte code page, so the limit counts characters, and a name
with `×` or `é` (two bytes in UTF-8) in a duel challenge wraps like any other. The scoreboard continues
to use structured server scores rather than parsing printed statistics tables.

`pmonly` toggles **Game PM-only chat** for the current map; `pmonly on`/`off`
(and 1/0) select it explicitly. The same default-off switch is in both settings
styles (`cg_pmOnlyChat`). Only game-server tells with the stock private marker
before the sender name remain in the overlay and produce a chat notification;
lookalike names, colours and quoted message bodies cannot mark a public line as
private. Sent tell echoes qualify too. Existing history is hidden, not erased,
and returns when the switch is off; console history/logs, centre notices and
SJK hub chat are independent. Moving to another map resets the choice; a graphics
reload preserves it. Unmarked private formats from other mods are not guessed.

`quiet` toggles **quiet mode** (`cg_quietMode`, not archived; Settings > Chat > Quiet
mode, both styles; `quiet on`/`off` and 1/0), after Lex Deifaber's idea of 30/10/2026,
and says ON or OFF in the console. Until it is turned off or the map changes, the chat box
hides public, team, SJK and admin chat, their beeps and the SJK chat sound, and keeps
them out of the console's alert words, but for whispers (server tells, encrypted SJK
whispers), the player's own lines (the gamestate's slot, never a followed player's) and
the private duel partner's (`ps.duelIndex` while the player's own `ps.duelInProgress`, kept
10 seconds after the duel ends for the "gg", until they leave or rename; on the SJK chat,
the key the hub knows in that slot). Lines no player's name starts (server notices) and the
SJK team's announcements still show, and so do centre prints; a line two players' names fit
is a player's and is hidden. A public line of a player whose name starts with the tell
marker is not taken for a tell (this holds for `pmonly` too). The console scrollback keeps
every line. Whether a line passes is decided when it arrives, so turning it off brings the
hidden history back ([quiet.rs](../crates/sjk-viewer/src/chat/quiet.rs), the shared rules
in [quiet_chat.rs](../crates/sjk-client/src/quiet_chat.rs)). The SJK scoreboard writes a muted
"Quiet" after the player's own name while it is on; showing it on other players' rows
needs the hub to relay the flag (for example a `quiet` field in the look of
`POST /v1/look`, given back in presence and look events), which it does not yet.

Chat remains connected to the real server during intermission, including
global/team/private composer messages and console/bound chat commands. Messages
received during background loading are retained; its loading notice hides the HUD. The scoreboard reserves a separate left column for messages and the
composer, temporarily overriding chat position/width while scores are visible.
Chat visibility/lifetime settings still apply. Typing captures gameplay input as
usual, and the ordinary chat layout returns after the scoreboard closes.

Escape opens the normal menu during exploration. Disconnect/cancel abandons the
pending connection and builds the main-menu world again (the last world stays under
the menus for the few seconds that takes; see
[rendering.md](rendering.md#gpu-memory)). Connection and asset
errors still show an error message. Downloads retain their existing policy and
limits; progress is available in the console. This does not eliminate disk,
shader, network or download latency, and direct command-line startup is separate
from the already-open menu's transition path.

Implementation: [resident worlds](../crates/sjk-viewer/src/resident_world.rs),
[early exploration](../crates/sjk-viewer/src/resident_walk.rs),
[world handoff](../crates/sjk-viewer/src/session_transition.rs) and
[destination preparation](../crates/sjk-viewer/src/portal.rs).

A built world (`GpuState`, about 600 KiB) crosses from the install worker to the
main thread boxed: through the worker's channel, `WorldInstallPoll::Ready`,
`take_world`, `poll_world_install` and `adopt_world`. Moved by value, each step
was a stack copy, and the main thread's nested frames (`window_event` 3.1 MB,
`Destination::poll`, `WorldInstallTask::poll` and the channel's `try_recv` about
1.9 MB each) overflowed its 8 MiB while joining JoF's public server (02/11/2026).
Building the menu's first world on the main thread came within 2 MiB of the same
8 MiB with a clean profile and overflowed it with JoF content loaded (03/11/2026),
so the client reserves 64 MiB for its main thread on Windows and macOS
([`build.rs`](../crates/sjk-viewer/build.rs)). Linux raises its inherited soft stack
limit to the same allowance before building the client, without lowering a larger
limit or changing the hard limit; a restrictive hard limit or failed adjustment is
logged ([`main_stack.rs`](../crates/sjk-viewer/src/platform/main_stack.rs)). The client's start must fit in a quarter of
that: [startup_stack.rs](../crates/sjk-viewer/src/startup_stack.rs) checks it, run
before every merge into `main` (AGENTS.md).

Losing window focus releases held gameplay controls and discards gameplay actions
already queued for that frame. Synthetic key events on refocus cannot re-press a
held modifier such as Alt. This allows a saber throw already sent to the server
to finish normally after Alt+Tab.

### Timed player peek

`/peek <client ID|name> [seconds]` temporarily follows a received player from
behind, with wall collision, as JoF EJKSol does. The default is 5 seconds,
maximum 60. Names ignore colours/case; an unambiguous fragment also works.
Quote names containing spaces, for example `/peek "Reborn Master" 5`.
Empty or colour-only queries are rejected. As in JoF, scope and crosshair
overlays still use the local player aim. Monitoring only received entities
respects server PVS, but may show surroundings outside your line of sight.
`/peek` alone uses the crosshair player; `/peek off` or a duration of 0 cancels.
Local flight (`fakenoclip` or `freecam`) ends a peek; stop flight before peeking.
Since 24/10/2026 Peek is the [Drone](#drone)'s: on a server it watches only a player
whose Drone switch is on (in a local game, anyone), stops when they turn it off, and
other SJK players see the drone behind the watched player.
The local body stays at its player position and the view gun is suppressed.
The camera cancels when the timer ends, the target slot/name changes, or the
map changes. Missing snapshot data pauses the view; it resumes if data returns
before expiry. Players outside the
received snapshot cannot be watched. Player input and server authority remain
unchanged. The command is bindable under Settings > Key bindings > Other.

### Chat logs

**Save game chat logs** in Settings > Chat (Advanced SJK UI; classic menus: HUD)
controls archived `cl_logChat`. It defaults to 0, off, as in stock clients;
saved choices are kept. The update's what's-new card also offers the
switch. `cl_logChat 0` disables recording, `1` appends normally, and the console's
`2` synchronizes each entry to disk. The Settings switch writes 0 or 1.

Received game-server public, team and private chat is appended to
`chatlogs/chat.log` beside the active profile's `config.cfg`, before display
filters such as local player blocks. Each entry is one line prefixed with its
Unix timestamp in seconds; numeric game colour codes and control bytes are removed,
Unicode is preserved, and a restart appends to the same file. The file is created
on the first received chat message. Other server console text, the separate SJK
hub chat and encrypted whispers are outside this game-chat log.

File I/O happens only when a chat event arrives. A write failure is reported once
in the console and stops attempts until logging is switched off and back on;
that retry works even when no chat arrives while the switch is off.

### Chat in other alphabets

Chat, names and every other text show Russian, Japanese, Chinese, Korean and colour
emoji (and any character a bundled font has) instead of `?` (Sol's requests,
12/10/2026; drawn from the wide-glyph and emoji atlases,
[rendering.md](rendering.md#characters-beyond-the-code-page)). An emoji is typed as
any character (Windows' emoji panel, Win+.); skin tones, families and flags draw as
one picture, and the caret and Backspace take them whole.
Russian travels as EternalJK sends it, in Windows-1251, so SJK and EternalJK players
with `se_language russian` read each other; `cl_charset` (Settings > Chat > Other
clients' text: Auto, Western, Cyrillic) says how text from other clients is read
when the guess is wrong ([player text](networking.md#player-text)). Japanese,
Chinese, Korean and emoji go as UTF-8: SJK players read them, retail and EternalJK
players see other characters. The chat field stops where the server would cut the
line (149 bytes as sent: 149 Russian letters, about 49 Japanese, Chinese or Korean
characters). The hub's SJK channel and bios take them too since 12/10/2026 (Cyrillic
they already took; the letter rules they share,
[identity.md](identity.md#the-bios-rules)); every SJK player reads them.

### Encrypted whispers

A whisper to another SJK player goes through the SJK hub end-to-end encrypted while
`cl_e2eWhispers` is on (default; Settings > Network > Encrypted whispers): in its own
pink, marked ENCRYPTED, with its own sound, and a card asks before sending one the hub
could not take through the game server. See [hub-chat.md](hub-chat.md#encrypted-whispers).

### Chat sounds

Each kind of chat message has its own sound switch and sound (Sol, after Turin's and
Nib's ideas, 23/10/2026; [chat_sounds.rs](../crates/sjk-viewer/src/chat_sounds.rs)),
in Settings > Sound > Chat sounds (Advanced), every row in the settings search:

| Kind | Switch (default on) | Sound (default `default`) | Default sound |
| --- | --- | --- | --- |
| Public chat (`chat`, `lchat`) | `cg_chatBeep` | `cl_chatSoundPublic` | JKA's `sound/player/talk` |
| Team chat (`tchat`, `ltchat`) | `cg_teamChatBeep` | `cl_chatSoundTeam` | talk; EJK's `button_11` under `cg_chatSounds 2` |
| Private messages | `cg_privateChatBeep` | `cl_chatSoundPrivate` | talk for a tell (EJK's `button_15` under `cg_chatSounds 2`), the SJK whisper sound for an encrypted whisper |
| SJK chat | `cl_sjkChatSound` | `cl_chatSoundSjk` | SJK's variant of talk ([hub-chat.md](hub-chat.md#a-sound-for-a-new-message)) |

- **The master switch** is EternalJK's `cg_chatSounds` (Chat sounds: Off, On, Distinct
  (EJK)): 0 silences every kind; 2 makes team chat and tells play JoF EternalJK's own
  sounds (`cg_main.c` registers `button_11` and, without JoF's `commlink_off`,
  `button_15`; `cg_servercmds.c` plays them) while their sound is Default.
- **Private messages** are a server's tells and SJK's encrypted whispers. A tell arrives
  as public chat (`chat`, or `lchat` to a teammate with a location) whose text starts
  with the stock private marker `[` (`G_Say`'s `SAY_TELL`); a player cannot type
  that byte, so a line made to look like a tell stays public. JA+, JoF and the other
  servers SJK knows send tells that way, so one test serves them all
  (`chat::private_chat_command`, which `cg_pmOnlyChat` uses too); the sound adapter
  gives a tell its own beep event (`PrivateChatBeep`). Before, `cg_chatBeep 0` muted
  tells with public chat; now `cg_chatBeep 0` with `cg_privateChatBeep 1` keeps only
  the private messages heard (Turin's case).
- **The sounds:** Default, JKA chat (`talk`), SJK chat and SJK whisper (SJK's two
  variants of talk), EJK team beep (`button_11`), EJK private beep (`button_15`), Blip
  (`interface/update`), Select (`interface/sub_select`), Button (`interface/button1`),
  Tick (`interface/menuroam`): the retail game's short sounds of `assets0.pk3`, read from
  the player's game data (SJK ships none). Choosing one in Settings plays it; each kind
  has a Play row, and `chatsoundpreview <public|team|private|sjk>` plays the same, its
  chosen sound even while its switch is off. An unknown value is the default.
- **How they play:** a game server's beep as the game's does (beside the listener on
  `CHAN_LOCAL_SOUND`, a muted player's never), the chosen sound in place of the
  adapter's; the SJK chat's and an encrypted whisper's through the interface cues.
- **Taskbar flash:** `cl_privateChatFlash` (archived, default 1; Private message flash)
  flashes the game's taskbar button when a private message arrives while the window is
  unfocused or minimised (winit's `request_user_attention`, Informational: Windows
  flashes it until the window comes back; a no-op where the system has none): a tell
  from a player not muted here, never the server's echo of the player's own, or an
  encrypted whisper. Name mentions in public chat already flash with EternalJK's
  `con_notifywords`.

### Chat player actions

U defaults to crosshair-target private chat (`messagemode3`): aim at a player,
press U, then compose a tell addressed to that player. While following a player as a
spectator, U whispers to the player followed, wherever the crosshair is, as JoF
EternalJK's `Con_MessageMode3_f` does (Kushina's report, 22/10/2026). Global chat remains on
Y (`messagemode`), and team chat on T (`messagemode2`). I opens the SJK chat
(`messagemode5`, [hub-chat.md](hub-chat.md)); a new profile, or a whole config table
imported at First setup, gets I when it is free and nothing is bound to
`messagemode5`; an existing profile binds it in Settings > Key bindings > Other.
Existing profiles gain
U when it is free and no custom crosshair-chat key exists. Version-2 profiles
from the earlier incorrect global-U default repair U to `messagemode3` and
restore Y global chat if needed and free; other occupied keys stay unchanged.
This migration advances `cl_bindDefaultsVersion` to 3. Use
`bind u messagemode3` to select U explicitly. No message is sent until Enter.

On the SJK UI's Character page a model the player's colour shows on (one of its shaders
reads the entity colour: `rgbGen entity`, `oneMinusEntity` or `lightingDiffuseEntity`,
whatever its skin is called), or worn with an `rgb` skin (the ones JoF EJK's RGB team
colour lists), gets Red, Green and Blue sliders under the grid, as JoF EJK's profile has (Creyon, 29/10/2026;
[model_rgb.rs](../crates/sjk-viewer/src/player_menu/model_rgb.rs)): 0 to 255, set by the
arrows, a click or drag on the track, or a typed number; the stage model shows the colour
at once (the shader stages that read the entity colour, as `CG_Player` hands them
`customRGBA`), the grid's pictures keep their own, and Apply wears it with the model
(`char_color_*`). Other models show no sliders.

`amcolor <red> <green> <blue>` (EternalJK's `amColor`) sets the player tint, the archived
`char_color_red`, `_green` and `_blue` userinfo cvars, each 0 to 255, so the server gets
it at once and it is kept for the next connect; bare, it prints the current tint
([am_color.rs](../crates/sjk-viewer/src/am_color.rs)). Before, `amcolor` went to the
server as an unknown command and the tint was lost on reconnect.

`reply <message>` (JoF EternalJK's) whispers to the last player who whispered to
you; a bare `reply`, bindable as "Reply to last whisper" in Settings > Key bindings >
Other (unbound by default), opens the composer on that whisper instead. Tab in the
composer reaches the same whisper after SJK (All, Team, SJK, the last whisperer).
Your own whispers, which the server echoes back, do not count; when nobody has
whispered, or the player left or renamed, `reply` says so and sends nothing
([chat/reply.rs](../crates/sjk-viewer/src/chat/reply.rs)).

`clientlist` (also `showplayerid`, JoF EJK's `showPlayerId` spelling) lists each player
in the console: the client number, a team letter (F free, R red, B blue, S spectator),
the name and ` (bot)` after bots, then `Listed N clients`. Commands that take a client
number use it. `clientlistinfo` gives JoF EJK's human-only detailed view: model and
skin, gender when advertised, saber names and colours, team model group in team games,
and non-default body RGB when that player's current snapshot state is available
([player_ids.rs](../crates/sjk-viewer/src/player_ids.rs)).

### Chat substitutions

Game-chat messages expand JoF EJK's substitutions when sent, from the console,
binds, the chat composer in either menu style, team chat, tells and `reply`.
Write each token as a separate word; matching ignores case. For example,
`say_team "Health %H% shield %S% Force %F%"` sends the current values.

| Token | Value |
| --- | --- |
| `%H%` | Health |
| `%S%` | Shield/armor |
| `%F%` | Force reserve |
| `%W%` | Current weapon's EJK display name |
| `%A%` | Current weapon's ammo pool, including its negative sentinel |
| `%P%` | SJK lagometer's latest snapshot latency estimate in milliseconds |
| `%U%` | Horizontal speed in units per second, rounded to an integer |
| `%UKM%` | Horizontal speed using EJK's km/h conversion, one decimal |
| `%UM%` | Horizontal speed using EJK's mph conversion, one decimal |
| `%T%` | Local time, 12-hour clock with AM/PM |
| `%T2%` | Local time, 24-hour clock |

Unknown tokens, embedded tokens such as `health=%H%`, and ordinary percent signs
stay literal. Expansion precedes chat colouring, quoting and the outgoing byte
limit. It runs only for messages the player sends to the game server; received
messages and the separate SJK hub chat are unchanged. Player values follow the
viewed snapshot, with predicted health, Force, weapon, ammo and velocity when
available; model/map transitions do not borrow the previous player's prediction.
See [chat/substitutions.rs](../crates/sjk-viewer/src/chat/substitutions.rs).

The chat key (`messagemode`, Y by default) opens the composer on global chat, as in
EternalJK, so it never inherits a previous team, SJK or whisper destination (SJK #181).
`cg_chatRememberChannel` (default 0, Settings > HUD > Remember chat channel; Sol's
decision 20/10/2026 to keep it as an option, off) makes it open on the channel of the
last message sent from it instead: All, Team, SJK, or a whisper to a player still in
the game (All once they leave). That lasts while SJK runs. The team, crosshair,
last-attacker and SJK keys always open on their own channel.

The composer's row of channels (29/10/2026;
[chat/tabs.rs](../crates/sjk-viewer/src/chat/tabs.rs)) is All, then Team, SJK, the clan
chats JoF and S once joined ([hub-chat.md](hub-chat.md#clan-chats), 03/11/2026), Admin
and PM, each behind its own setting in Settings > Chat (`cg_chatTabTeam`,
`cg_chatTabSjk`, `cg_chatTabJof`, `cg_chatTabS`, `cg_chatTabAdmin` 1 by default,
`cg_chatTabPm` 0): a channel switched off (0) leaves the row and Tab skips it, but its
key still opens it; since 03/11/2026 (Sol's request) Not in Tab (2) keeps the button
while Tab skips it, so a player keeps Team or Admin clickable out of the cycle (Tab
from a channel it skips goes on to the next one it stops on). The clan chats' keys are
`messagemode7` (JoF) and `messagemode8` (S). Admin sends `amsay`, which only the
server's admins read (JoF EJK `Cmd_Amsay_f`, `SAY_ADMIN`; they see it as `<Admin>`), and
shows only on JA+ and jaPRO servers; its key is `messagemode6` (Controls > Admin chat, no
key by default), which says so on other servers. Leaving one moves a composer on Admin to
All, and a remembered Admin or hidden channel opens on All. PM whispers to the last
player who whispered you; left-clicked again the next one, right-clicked the one before
(since 04/11/2026, Creo's idea, the recipient has a line of its own under the row, the
whole chat's width: "< To Fox  2/3 >", the name in its colours and cut with "..." only
when even that is too narrow; `<`, `>`, Alt+Left and Alt+Right step between the players
who whispered you, and a click on the name lists them, newest first, in a dark
see-through panel right above the name, Escape closing it before the composer; `chat/view/recipient.rs`),
or says how to whisper when no one has. The words sit an even 22 px apart, each as wide
as itself. Each button is a tab
too (Dave, 29/10/2026): while the composer is on Team, SJK, Admin or a whisper, the feed
shows only that channel's messages (whispers both ways, encrypted ones and the server's
private lines included; admin lines are jaPRO's `<Admin>(name)`), All shows everything,
and closing the composer shows everything again; `cg_chatTabFilter` 0 (Settings > Chat >
Tabs hide other chat) keeps the feed whole while typing: every chat in view, each line
labelled with its channel (TEAM, SJK, JoF, S), the button picking only where the
message goes.

The row has its own tools since 06/11/2026 (Sol's request;
[chat/row_tools.rs](../crates/sjk-viewer/src/chat/row_tools.rs), drawn in
[chat/view/row_tools.rs](../crates/sjk-viewer/src/chat/view/row_tools.rs) as strokes,
not glyphs, set level with the middle of the words' capitals and as tall as them
(`UiFont::capital_middle`, `capital_height`; 06/11/2026). Pointing at any button but All shows a small × right after its word; a
click takes it off the row (its setting to 0, as Settings > Chat would), and a
composer on it moves to All. After the words, a + (only while a usable chat is off the
row: PM by default, Admin only on servers that have it, a clan chat only once joined)
lists those chats in their colours, and a click puts one back (1); the menu closes
once none is left. A cog opens quick settings: Show only this chat and Show every chat
(`cg_chatTabFilter` 1 or 0, the picked one lit), Tab skips the chosen chat (its
setting 2 or 1; not offered on All or PM) and Open on the last chat
(`cg_chatRememberChannel`), each On or Off; the menu stays open for the next change.
Both menus open in a dark panel above their button, over the feed; a press anywhere
else, the same button again, or Escape (before the composer) closes them. The frame
saves each change to the profile (`GpuState::save_chat_settings`), so Settings and the
row always agree.

Open the chat composer with your chat binding: the cursor is free while composing.
Resting the pointer on a name (a sender's, or an SJK chat sender's) shows their
sender card (picture, name, hub, medals) beside it, with Block or Unblock: a block is
kept on this PC (`chat-mutes.txt`) and, besides their chat, draws them as Kyle with the
default saber, silences every sound they cause and ignores their emotes, toys and player
card; `block <player>` and `unblock <player|all>` do it from the console
([hub-chat.md](hub-chat.md#blocking-a-player)). The card lies over the feed: the feed's
text under it is left out (`MenuCanvas::cover`, honoured by the routed game-font path
too since 18/10/2026; before, the messages showed through the card). Name hover fits
the visible username glyph bounds. There is no standing player-options hint or success
notice. Opening the composer exposes history even when passive chat is hidden with
`cg_chatbox 0`.

Clicking a name pins its sender card (23/10/2026, Bishop's and Sanjisan's requests, Sol's
go; [chat/card.rs](../crates/sjk-viewer/src/chat/card.rs)): the card stays whatever the
pointer does, until the name is clicked again, a click lands off the card and off every
name, Escape is pressed (the first Escape closes the card, the next the composer) or the
composer closes. For a player on the server other than yourself it heads its buttons with
**Whisper**, above Block: the composer turns to a whisper to them, the draft kept, sent as
any whisper from the composer (encrypted through the SJK hub when they take encrypted
whispers, else the server's `tell <slot>`). Whisper names the roster observation the
line was matched to: a player who left, renamed or whose slot another took has no
Whisper, and a press made before the roster changed says "Player left or changed
identity". An SJK chat line's sender is whispered in the slot their key (or name) is
matched to here ("On this server, slot N"); one not on the server has none. Lines are
matched to players as before: the server's sender slot, else the one connected player
whose name, as the game shows it, starts the line; two players wearing one name leave
such a line unclickable. The card's Safety code button also answers clicks now (before,
the press was taken for the card's and the click was lost).

There is no Add friend: SJK has no friends list (the hub keeps none, and the local friend
star of the old name dropdown, `chat-friends.txt`, went with it on 18/10/2026, Sol's
decision), and there is no View profile either: the card is the profile view (picture,
hub name, key, medals, where they are), while the player card (`inspect`) needs the
player in sight. The old dropdown (whisper, a per-map ignore, the friend star, copy
name) stays gone; an existing `chat-friends.txt` is not read.

Copying a link from the chat (03/11/2026, AngeloPapaya's idea;
[chat/links.rs](../crates/sjk-viewer/src/chat/links.rs)): while the composer is open,
resting the pointer on a web link in a chat line (a word starting `http://`, `https://`
or `www.`, its closing punctuation and the colour codes round it left out) lights it as
a name under the pointer is lit, on every row it wraps onto, and Ctrl+C (or Ctrl+Insert)
copies the link, without its colour codes, instead of the draft; "Link copied" shows
under the field for a second and a half. A selection in the draft goes first: with text
selected there, Ctrl+C copies that as in every field. Off a link Ctrl+C copies the draft
as before. Game and SJK chat lines alike; a muted line, a find's words and a GIF shown as
"GIF" have no link to copy (with GIFs off the link shows, and copies, as typed). The chat
box is the same in SJK UI and classic, so the feature is too. Links are laid out only in
frames that draw the feed with the composer open, in a fixed array of 32 pieces.

The draft shares the console's UTF-8 caret and selection rules: arrows/Home/End,
Ctrl+arrows and Ctrl+Backspace/Delete, Shift-selection, Ctrl+A/C/X/V,
Ctrl+Insert to copy and Shift+Insert to paste. Click places the caret, drag selects,
and double-click selects a token. Selected text is highlighted; typing/pasting
replaces it. Clipboard text keeps colour escapes, strips controls and obeys the
1,200-character storage bound and the smaller four-message submission bound. Pasting
never sends a message. On submission, a draft that exceeds the game server's 149-byte
escaped-message budget is sent completely as at most four messages, split at a word
where possible and at a character boundary for one oversized word. A fifth piece is
refused; commands entered outside the composer get a warning instead. This is shared
by All, Team, Admin, whispers and declared mod-chat commands, including an encrypted
whisper the player chooses to send unencrypted from its failure card. An encrypted
whisper within the hub's 900-byte limit remains one message. The last `^` colour of
each piece starts the next one, within its 149-byte budget.
Dead keys type as in
the console (see below): `^` then a digit is a colour code, `^` then `e` is `ê`.

Actions use server-provided sender slots and current roster generations. Old
messages cannot address a replacement after an observed departure/name change;
an invalid whisper recipient (the channel `reply` or Tab chose) leaves the draft open. A line the server does not
attribute (JA+) is its sender's when it starts with exactly one connected player's
name as the game shows it, then `: ` (or `(name): `, `[name]: `), since 16/10/2026
([hub-chat.md](hub-chat.md#game-chat-lines-look-the-same)); other unattributed lines
stay unclickable. `reply` also reads an unattributed `[name]: ` whisper whose name
(colour codes aside) is exactly a connected player's, as JoF EternalJK does.
Legacy servers provide no authenticated account identity; unobserved same-name
slot reuse cannot be distinguished. No transport or protocol encoding changed.

The leader/opponent portrait and its name/score occupy the top-right corner,
with a 32-unit top margin and the existing 40-unit right margin at 1080p (scaled
with the HUD). Optional snapshot diagnostics and inventory flow below that block.
Visibility and server-selected leader/opponent rules are unchanged. There is no team
overlay ([Team outlines and logos](#team-outlines-and-logos)).

`cg_drawInventory` (archived, default **0**) controls the permanent list of
carried holdable items below that block, following JoF EJK #40's quieter default.
Enable **Inventory list** in classic Settings > HUD or SJK Settings > Advanced >
HUD to show it. The brief inventory selector still appears when cycling items,
whether this permanent list is on or off. Trip-mine weapons are not part of the
holdable list. Existing saved values remain in effect.

## Speedometer placement

`cg_speedometerX` is the speed label's right-edge anchor in the 640-wide virtual
screen, as in JoF EternalJK's `CG_Speedometer`; its default is 132. With the
game-data HUD selected, JoF's HUD-specific clearance is then applied: 8 units
left for `cg_hudFiles 0`, 56 for the text HUD (`1`) and 42 for the JK2 HUD (`2`).
The text HUD gives 4 units back when the armor value exceeds three digits, as
JoF does to clear the wider armor counter.
`cg_speedometerY` sets the baseline and `cg_speedometerSize` sets the scale.
Earlier SJK builds mistook X for the left edge of a centred 240-unit field and
omitted the HUD clearance, placing the label too far right. The archived cvar
values themselves are unchanged.

## Movement keys

`cg_movementKeys 1` (default 0; Settings > HUD > **Movement keys**, Advanced) draws
JoF EternalJK's movement keys (jaPRO's `CG_MovementKeys`, JoF EJK's `cg_draw.c`),
[movement_keys.rs](../crates/sjk-viewer/src/hud/movement_keys.rs): jump, forward and
crouch over left, back and right, each lit while it is held.

- **Whose keys.** Playing, they are the command being made this frame, so an analog
  move lights its direction too. Following a player or watching a demo, the keys are
  guessed from the player state as JoF guesses them: the movement direction
  (`movementDir`) while moving at 9 units a second or more; jump while the jump is
  held (`PMF_JUMP_HELD`) or while rising faster than the frame before, more than
  8 units a second, not gripped, with the origin more than 1 unit over standing
  height above the floor (a trace straight down, `PM_GroundDistance2`); crouch while
  ducked (`PMF_DUCKED`) or rolling (`BOTH_ROLL_F`, `_B`, `_L`, `_R`).
- **Where.** `cg_movementKeysX` (465) and `cg_movementKeysY` (432) are the block's top
  left corner in the 640 by 480 screen, cut to whole units as JoF reads them;
  `cg_movementKeysSize` (1) scales each key's 16-unit side. As EternalJK's
  `widthRatioCoef` does, the keys stay square on wide screens and keep their distance
  from the right edge. With the game-data HUD and one of JoF's numbered HUDs
  (`cg_hudFiles` 1 to 3) they move as JoF moves them (`JOF_HUD_OFFSETS`): 51 units
  right for HUD 1, 26 right and 3 up for HUD 2, 18 left for HUD 3; then 12 down
  without a score line (`cg_drawScore 0`, or Power Duel), or, with team scores
  (`cg_drawScore` 2 or more in a team game other than Siege), 14 up and 23 left for
  HUD 1, 12 left for the others. A menu file path such as `ui/jahud.txt` moves
  nothing. `cg_hudScale` does not size them, as in JoF.
- **When.** With the HUD (`cg_drawHud`, `cg_draw2D`) and `cg_drawStatus` on, for a
  living player not on the spectator team, on foot, and not under the scoreboard,
  as `CG_DrawStats` draws them.
- **Look.** jaPRO's pictures (`gfx/hud/keys/<key>_on` and `_off`), from any pack that
  has all twelve, EternalJK's `japro-assets.pk3` included: only those pictures are
  mounted from the `EternalJK` folder, as for its crosshairs. Without them SJK draws
  its own keys in the same places: a rounded cap with an arrow (jump an arrow up from
  a floor line, crouch an arrow down from a ceiling), dim while released and bright
  while held.

Not covered: jaPRO's race mode moving the keys as no score line does (SJK draws no
strafe guides on jaPRO servers either), and the guess's floor trace sees only the
map, not movers or players under the followed player. Riding a vehicle hides them,
where JoF keeps them when the vehicle has no HUD menu of its own.

## Lagometer

`cg_lagometer` (archived, default 0, Settings > HUD > **Lagometer**) draws the
network graph: frame interpolation in its top third (yellow above the middle line
while the view runs ahead of the latest snapshot, blue below while it interpolates)
and snapshot latency at its bottom (green, yellow when the server held a snapshot
back for rate, red for a dropped one). It takes JoF EJK's values (`cg_draw.c`,
`CG_DrawLagometer` and `CG_DrawSpeedGraph`):

- `1` (**Graph**): the graph on its backdrop, as SJK always drew it; an old config's
  `cg_lagometer 1` looks the same;
- `2` (**Graph + speed**): the same, with the average ping and the average
  interpolation of its 48 newest columns written on a line just above it (ping at the
  left, interpolation at the right, `%.0f` and `%04.1f` as EJK writes them; a dropped
  snapshot counts as the highest ping seen so far), and a speed graph on a backdrop of
  its own 56 of the graph's 48 units above it, higher by as much as that line needs: a
  green column per frame for the horizontal speed, the height worth 3000 units a
  second, the current speed at its top left;
- `3` (**No backdrop**): the same without either backdrop, the speed graph three times
  as tall (its bottom where mode 2's is), its number only while moving.

EJK draws the speed graph only with its `cg_speedometer` speed graph bit; SJK has no
such bit and draws it with modes 2 and 3 (Sol's decision, 29/10/2026).

The numbers do not keep EJK's proportion of the graph (`ocr_a` at half size, 10.5 of
its 48 units): SJK's graph is 45 pixels at 1080 lines with the default `cg_hudScale`
(0.7) where EJK's is 108, so they were tiny at `cg_lagometerScale 1`. They are drawn at
EJK's own size instead (Sol's decision and request, 29/10/2026): a 34-pixel line at 1080
lines before `cg_hudScale`, so 23.8 pixels with the default HUD scale (EJK's is 23.6),
where SJK Chat's digits (`ui_gameFont`) stand about 13 pixels tall, as the HUD's
"Score" capitals. They grow with the HUD (the screen's height, `cg_hudScale`) and
`cg_lagometerScale`, mode 3's speed number alike. The ping and interpolation stand on
their own line just above the graph, so the graph keeps its size in every mode. The line
is at least 4.7 lines wide (a three-digit ping and "-99.9" side by side), wider than the
graph and centred on it, and stays inside the screen: shifted in at the sides, and just
below the graph when the graph sits too near the top for it (the speed graph, above the
screen's top then, as in EJK, is not moved). See
[hud/lagometer.rs](../crates/sjk-viewer/src/hud/lagometer.rs) and its tests
(`mode_1_is_the_graph_alone`, `the_averages_follow_ejk`,
`the_numbers_are_readable_above_the_graph_at_every_scale`,
`the_digits_are_about_13_pixels_and_both_numbers_fit_their_line`,
`at_the_screen_edges_the_numbers_stay_inside`). Not compared side by side with EJK in
game.

`cg_lagometerScale` changes the lagometer's width and height: `1` (default) is
normal size, `0.5` half size, `2` double size. Settings > HUD > **Lagometer scale**
offers 0.25–4 in both menu interfaces; values outside that range are clamped when
drawn. The setting is archived and takes effect immediately; it scales the numbers
(from their own base size) and the speed graph with the graph.

Scaling keeps the graph's existing bottom-right corner fixed, including custom
`cg_lagometerX` and `cg_lagometerY` offsets. It applies to every built-in HUD and
custom layouts that use the `lagometer` binding, on top of their layout dimensions.
It preserves the number and order of visible samples and the graph's millisecond
ranges, so a larger graph does not show a longer history.

## Strafe helper

`cg_strafeHelper` draws guides for strafe-jumping with JKA's movement physics, as
JAPRO-derived clients do; its options are bits, listed and toggled by the
`strafehelper` command (`strafehelper 1` switches the lines style on). Two styles
are drawn, one at a time (the command switches between them as JoF EJK's does):

- option 1, the lines (JoF EJK's "updated style", `DrawStrafeLine` and
  `CG_StrafeHelper` in JoF's `cg_draw.c`): one line per enabled key, from the bottom
  centre of the screen up to just above the crosshair (virtual height 230), aimed at
  where the best heading for that key projects from the camera at 256 units
  (`cg_strafeHelperPrecision`). Keys: W (option 5, two lines), WA (6), WD (7), A (8),
  D (9) and the rear A/D lines (10); the centre line (11) is drawn only for other
  physics, so not here. The line of the keys held takes `cg_strafeHelperActiveColor`;
  the others keep JoF's tints at `cg_strafeHelperInactiveAlpha`. They show in first
  and third person, airborne and on the ground (JKA's ground acceleration 10 and
  friction 6), once the horizontal speed reaches the player's speed;
  `cg_strafeHelperCutoff` hides that many virtual pixels from the bottom (at most up
  to 20 below the line top), `cg_strafeHelperLineWidth` sets the thickness;
- option 2, CGAZ markers (TaystJK's `hud_strafehelper.c`): short ticks across the
  crosshair at the same headings, airborne only.

The strafe aura (`cg_strafeAura`, Settings > HUD+ > "Strafe aura", off) is SJK's own
guide, drawn on its own or with a helper style, while you jump or fall (on the ground
too with `cg_strafeAuraGround`, "Aura on the ground"). Where the lines mark one
heading per key, it shows the speed every heading gains: an arc over the crosshair,
its top the view and each point a turn of that many degrees (left on the left, 60
each side), glowing in Force blue with what one move would add there with the keys
held (`PM_Accelerate`: the push along the wish direction is `wishspeed` less the
speed already along it, at most the acceleration; `sjk_ui::strafe_gain`). The best
heading, where the push first reaches the acceleration, is a white-hot seam: short
of it nothing is gained, so the arc is dark; past it the glow thins and fades with
the cosine; red marks headings that slow you down. It is one draw command,
`DrawCommand::StrafeAura`, shaded per pixel by `ui_shapes.wgsl` (wisps flowing
towards the best heading, a pulse while the view gains at least 90% of the best),
with `sjk_ui::strafe_aura_shade` as the CPU reference the menu snapshots draw
(`strafe_aura_snapshot`). The tick at the top turns blue while the view gains at least
90% of the best (red when it loses). It uses the lines' physics, frame rate
(`cg_strafeHelper_FPS`), speed gate and restrictions; `cg_strafeAuraRadius` (virtual
pixels of 480 lines, 40) and `cg_strafeAuraAlpha` (1) set its size and opacity.

Settings > HUD+ > "Strafe lines" switches option 1 (a derived switch: on replaces
the other style and adds WA and WD when no key is picked, off clears it). The
default value, 3008, picks keys but no style, so nothing is drawn; JoF EJK's
profile value `2242` (lines, WA, WD, centre) draws the same two lines here. Nothing
is drawn on JAPRO servers (their serverinfo names `japro`), whose movement styles
change the physics, while in a vehicle, zoomed or in a special movement type. The
helper's sound, accel bar, Weze, Warsow and original styles are not drawn.

## Followed player's name

While following (spectating) someone, and throughout demo playback, the HUD names
the player whose view it shows in the top left, as JoF EternalJK's `CG_DrawFollow`
does (stock Jedi Academy centred "following" and the name at the top): the name with
its colour codes, white otherwise, its line top at (4, 27) of the 640x480 screen and
`FONT_MEDIUM` at 0.85 (SJK Menu with "Classic game fonts", the HUD font otherwise), placed
in the HUD's 1080-line frame like the other HUD text
([follow.rs](../crates/sjk-viewer/src/hud/follow.rs)). While it shows, the warmup
line ("WAITING FOR PLAYERS", "STARTS IN") is not drawn, as `CG_Draw2D` skips
`CG_DrawWarmup` then. It follows `cg_drawHud` and `cg_draw2D` and steps aside with
the rest of the HUD while the scoreboard is held (hub report 69).

## Scoreboard styles

`cg_scoreboardStyle` (Settings > HUD > Scoreboard > "Scoreboard style") picks the
scoreboard layout. While either scoreboard is held, the HUD steps
aside as for the quick wheel (status, weapon, timers, vote and kill lines,
crosshair, lagometer, speedometer, warmup line, Force and item
selectors, movement guides, the red damage-direction flash, the ground readout
and the game-data HUD; chat stays, `scoreboard::hides_hud`), unless
`cg_drawScores 0` leaves the key drawing nothing:

- `auto`, the default of a new profile: the SJK UI's scoreboard while
  `ui_menuStyle` is `sjk`, the classic one with the classic menus (what SJK
  showed before the choice existed);
- `sjk`, the SJK UI's scoreboard whatever the menus
  ([SJK UI](sjk-ui.md#scoreboard)): columns floating over the darkened game in
  the UI's type, the teams side by side, the duelists as facing cards. Its
  compact rows (`cg_compactScoreboard`, on by default) keep a full server in
  one column, in a board as wide as its names, centred clear of the chat; its
  header shows the frame rate left of the clock (`cg_scoreboardFps`, on by
  default; Settings > HUD > Scoreboard > "Scoreboard frame rate");
- `classic`, the retail scoreboard as EternalJK-derived clients such as JoF EJK
  draw it (below).

Without the key the board opens only at intermission (`PM_INTERMISSION`, 7): not on
death (SJK's choice), and not while a JA+ emote holds the player in `PM_FREEZE` (6),
which once read as intermission and kept the board up for the emote's whole length
(`scoreboard/behavior.rs`).

A mistyped value gives the classic board. `config.cfg` saves every archived
setting, so every profile from before `auto` existed had the old default
`classic` saved: the console's start moves a saved `classic` once to `auto`
(`cg_scoreboardStyleDefaultVersion`), which draws the same classic board unless
the menus are the SJK UI. A look chosen after that stays. The classic board
([classic.rs](../crates/sjk-viewer/src/scoreboard/classic.rs), after
`CG_DrawOldScoreboard`/`CG_DrawClientScore` in `cg_scoreboard.c`):

- The header shows "Killed by" while you are dead, otherwise the player count
  (`cg_drawScoreboardPlayerCount`: 1 host name and counts, 2 counts only, 0 off;
  team games show "N vs. M", your team first), and below it your place ("2nd
  place (of 7) with 18", place in its retail colour) or the team lead.
- Columns are Name, Score, Ping, Time and, with `cg_showClientIDs` (on by
  default), the client ID; CTF shows Score, C, A, D, Ping and Time, and duels with
  a frag limit show wins/losses. Score shows score/deaths when `cg_scoreDeaths`
  provides deaths. With JoF EJK's `cg_scoreDeaths` modes, 0 hides them, 1 uses a
  JA+/jaPRO server's appended value only (the default, as EJK), 2 uses that value and
  falls back to the session-local counter when the server sends none, so kills/deaths
  show on every server, and 3 always uses the local counter. The local counter knows
  only the deaths seen since you joined. As in EJK, duels and CTF never show deaths, and the local counter
  starts again for everyone on `map_restart` or a new level (a new
  `CS_LEVEL_START_TIME`), and for one player when they change team or leave
  (`scoreboard/deaths.rs`). Bots show `BOT` for their ping; clients still connecting show
  `-`, and connected clients the scores do not list yet show `N/A`.
- **Rank by score - deaths** (`cg_scoreboardDeathsRank`, archived, off; Settings >
  HUD > Scoreboard, both looks; #ideas recap of 29/10/2026): where the board shows
  deaths, players are listed by their score minus their deaths, fewer deaths first on
  a tie, then the server's order; rows without a count or still joining follow, and
  away players stay below the others. The SJK board numbers its rows in that order, one
  number each. Off, the server's order and places as before, ties sharing a place. Neither EternalJK nor JoF EJK has it
  ([style.rs](../crates/sjk-viewer/src/scoreboard/style.rs) `rank_value`).
- Team games list the leading team first over a translucent team band, then the
  spectators; free-for-all lists the players, then the spectators. Your row is
  highlighted in your rank's colour (1st blue, 2nd red, 3rd yellow, else grey) and
  is added at the bottom if the list does not reach it. Flag carriers show the
  flag icon before their row; at intermission, ready players are marked `READY`.
- Rows are 25 units of the 480-line screen, 15 units once more than 12 scores
  arrive (always with `cg_smallScoreboard` and in CTF), and 12 units in that small
  format above 20 connected clients. The connected roster controls the raised
  header independently of received scores. Normal small lists have a 19-row
  budget; crowded lists have 32. Section spacing, empty sections, losing-team
  reservations and the appended local row follow the pinned JoF reference,
  rather than an additional screen-bottom clip. `cg_drawScoreboardIcons`
  (on by default) shows
  each player's head icon (`models/players/<model>/icon_<skin>`), decoded when a
  model changes, two per frame at most.
- Positions follow the 640x480 screen fitted to the window height, spreading up
  to 1.25x horizontally on wide windows while text keeps its proportions. Names
  and headings are sized like retail `ergoec` text and numbers like `ocr_a`.

The classic board's interaction follows
[JoF EJK revision bd5e202d](https://github.com/JediofFreedom/JoF_EJK/blob/bd5e202dfedd49500f1b7aadd655619bac152167/codemp/cgame/cg_scoreboard.c)
(`CG_DrawOldScoreboard`) and its
[chat/HUD rules](https://github.com/JediofFreedom/JoF_EJK/blob/bd5e202dfedd49500f1b7aadd655619bac152167/codemp/cgame/cg_draw.c).
It opens instantly when held, on death and at intermission. Releasing the key or
respawning retains the board for a 200 ms fade if neither death nor intermission
keeps it open. Warmup suppresses the board unless the score key is held, matching
JoF's warmup gate. Menus, console, pause and
`cg_drawScores 0` suppress it.
Rows change places instantly. The SJK style keeps its animated opening and rows.

Classic moves chat to virtual x=8, bottom y=475 on the 640x480 screen, retaining
the configured font, width, lines and message expiry. Above 25 connected players,
holding scores hides the chat feed; release restores it, even when the automatic
death or intermission board remains open. That automatic crowded board limits the
list to 24 rows and adds a **HOLD TAB TO VIEW FULL SCOREBOARD** hint. Its local-row
append follows the reference too: a missed playing client is appended except
on the blue team. Holding the key shows the full list. Center prints stay
hidden through the scoreboard fade. SJK's chat composer stays inside the window
and can still be used independently of the feed.

Classic follows JoF's selective HUD visibility: holding scores or dying hides
the playing client's status, weapon/Force/inventory selection and crosshair;
votes, telemetry and independent power indicators remain visible. Spectators
retain their crosshair while holding scores. Intermission hides the gameplay
HUD. The SJK scoreboard keeps its whole-HUD suppression while held and its
held/intermission-only opening policy. Team rows reserve space for the losing
side, with connecting
clients listed after scored clients. An intermission duel winner takes precedence
over the next duel's participants and the killer/player-count header.

[Score requests](https://github.com/JediofFreedom/JoF_EJK/blob/bd5e202dfedd49500f1b7aadd655619bac152167/codemp/cgame/cg_consolecmds.c)
share a two-second gate with intermission refreshes; an explicit stale classic
request clears cached rows until the server responds. Demo key presses neither
request nor clear scores. The roster cache also tracks connected clients absent
from a score response, so joins, departures and team changes update the layout.

SJK retains its ping colours, faint alternating stripes and identity marks, and
its existing meaning of `cg_drawScores` as the bound-board enable switch (JoF
uses it for the small HUD scores). Server-plugin deaths without a verified plugin
handshake, Siege class/winner presentation and recorded demo score rows remain
existing compatibility gaps; this change does not synthesize those server facts.
The retained text and draw storage is reserved for 32 clients.

### Away players

Both scoreboards set away-from-keyboard players apart (`cg_scoreboardAfk`,
Settings > HUD > Scoreboard > "Away players apart", on by default; Sol's request
of 18/10/2026, as often half a server stands idle). Nothing comes from the server
beyond what every client receives, so it works everywhere
([scoreboard/afk.rs](../crates/sjk-viewer/src/scoreboard/afk.rs)):

- A player in view whose view angles (`apos.trBase`, the player state's
  `viewangles`) and weapon have not changed for 3 minutes is away. Someone
  playing moves the mouse all the time, so a still view is the sign; position is
  not read, since players push idle ones around.
- Time out of view counts once the player is seen again with the same view: it
  could not come back to the same angle by chance. A player never seen, or seen
  only briefly, counts as present, so the board can miss someone away but does
  not mark someone playing.
- A chat line from them (the sender slot `G_SayTo` sends, or the line's name on
  servers that leave it out), a weapon change, a score gain or a new name brings
  them back. A respawn or teleport (`EF_TELEPORT_BIT` toggled) and a new map's
  clock start a fresh watch that keeps the idle time; while `EF_DEAD` the view
  follows the killer and is not read.
- A name wearing `[AFK]` (the `afk` command's prefix), `(AFK)`, `<AFK>`,
  `{AFK}` or `|AFK|` is away at once (`afk::NAME_TAGS`, colours and case
  ignored).
- Yourself, bots and players still joining are never marked.

Away players drop below the others of their list, back to their place once
present. The SJK look draws them unnumbered, their name faint with its colours,
and "AFK 7m" (the minutes seen idle, or "AFK" alone) where the ping goes; the
places above them still count them, so they match the header's "3rd of 14"
([SJK UI](sjk-ui.md#scoreboard)). The classic look draws their row faint and
"AFK" left of it, where `READY` goes at intermission.

Limits: someone who goes away out of your sight stays present until seen, and an
anti-AFK bind that keeps turning (`+left`) reads as playing. The view angles are
the player state's own (`BG_PlayerStateToEntityState` copies `viewangles`), which
`codemp` pmove only holds still for an idle player; a mod that turns players
itself would count as activity.

## HUD style

`cg_hudStyle` chooses `game`, the status HUD of the game's own menu files (the
original Jedi Academy HUD, or a custom HUD pack that replaces `ui/hud.menu`),
or SJK's own `classic` or `radial` layout. The default is `game`, the classic
HUD, and a saved `cg_hudStyle` is kept (but the retired `modern`, see
[Menu style](#menu-style)). `cg_hudFiles`
names the menu list, `ui/jahud.txt` by default; `1` gives the text-only HUD and
EternalJK's `3`/`4` name its elegance and JoF HUD lists when those files are
installed. `cg_hudPack` names the PK3 whose HUD to use when several replace
`ui/hud.menu` (`assets1.pk3` is the original); empty, the one installed last
wins, as in the game. See [game-data HUD](rendering.md#game-data-hud) for what
is drawn.

The `radial` layout's status meters (the pills, the arcs and their numbers round the
crosshair) can be placed (Lex's idea, 29/10/2026: too close to the character):
`cg_hudRadialSpread` (0 to 250 HUD pixels, Settings "Radial HUD spacing") moves the
left meters left and the right ones right, and `cg_hudRadialHeight` (-20 to 40 per
cent of the screen, "Radial HUD height") moves them all up, or down below zero. The
crosshair, its name, the timer and the rest of the HUD stay put. Both are 0 by default,
the layout as bundled ([hud.rs](../crates/sjk-viewer/src/hud.rs), `RadialPlacement`,
which moves each widget of `RADIAL_STATUS` by its side: an arc's from its sweep, the
rest from their offset).

### What the HUD switch hides

`cg_drawHud 0` (Settings > HUD > HUD, the quick wheel's HUD on/off, a
`toggle cg_drawHud` bind) hides the parts of the screen whose switch is on (Blep's idea,
03/11/2026; [hud/toggle.rs](../crates/sjk-viewer/src/hud/toggle.rs)), each a row under
HUD in Settings > HUD:

| Cvar | Default | Settings | Hides |
| --- | --- | --- | --- |
| `cg_hudHideElements` | 1 | HUD off: elements | health, shield, Force, ammo, weapon and item rows, timers, vote and warmup lines, the kill feed, the lagometer, the game-data HUD and the rest of the layout |
| `cg_hudHideCrosshair` | 1 | HUD off: crosshair | the crosshair and the name under it |
| `cg_hudHideNames` | 1 | HUD off: names | nameplates, names over players, team logos and outlines |
| `cg_hudHideChat` | 0 | HUD off: chat | the chat box |

The defaults are what `cg_drawHud 0` always hid, so nothing changes until a player
ticks otherwise; with all four off, the HUD switch hides nothing. `cg_draw2D 0` still
hides everything, and the scoreboard, menus, intermission and the quick wheel hide the
HUD as before. Unit tests cover the rule and the defaults; not checked in a running game.

### Moving HUD elements

The HUD edit mode moves the HUD's elements with the mouse (Lex's idea, #ideas
29/10/2026). Settings > HUD "Move HUD elements" (or the `hud_edit` command, in a match
or a demo) closes the menus and outlines every movable element that shows, with its
name, over the game; a small "Move the HUD" panel holds Reset all and Done. Drag an
element to move it; right-click it, or point at it and press Delete, to put it back;
Escape, Enter or Done leave (back to the game menu when the mode was opened from it).
`hud_edit reset <element>` and `hud_edit reset all` do the same from the console
([hud_edit.rs](../crates/sjk-viewer/src/hud_edit.rs)).

An element with position cvars of its own is moved by changing them, in their units,
so the console and the mode agree: the lagometer (`cg_lagometerX`, `cg_lagometerY`),
the movement keys (`cg_movementKeysX`, `cg_movementKeysY`), the speedometer
(`cg_speedometerX`, `cg_speedometerY`), the kill feed (`cg_killfeedX`, `cg_killfeedY`),
the personal score (`cg_drawScoreX`, `cg_drawScoreY`) and the chat (`cg_chatBoxX`,
`cg_chatBoxHeight`); a reset sets their defaults. The others keep an offset in
`cg_hudOffsets`, `id:x,y` pairs in 480-line units (the screen's height over 480 pixels a
unit on both axes, right and down positive), an element left out being where its HUD
puts it: `vitals` (health and armour), `force` (Force and saber style, and the radial
look's ammo, which sits in its right pill), `weapon` (weapon name and ammo), `timer`,
`vote`, `jetpack`, `vehicle`, `selection` (the weapon, Force and inventory selectors),
`flags` (CTF flags and team scores) and `follow` (the followed player's name)
([placement.rs](../crates/sjk-viewer/src/hud/placement.rs)). The layouts' widgets move
in the layout pass, the rest by moving what they drew
(`sjk_ui::DrawList::translate_from`); with the game-data HUD, `vitals` moves its left
menu block and `force` its right one (Force, ammo and score line), whatever
`cg_hudScale` is. Offsets apply on top of the radial sliders above.

Not movable yet: the crosshair (`cg_crosshairX`/`Y` move it by hand), its name, the
warm-up and interruption notices, the frame rate and timer in the upper right, the
power-up and inventory icons, the text-only game HUD (`cg_hudFiles 1`), the stock
vehicle HUD drawn from the game's pictures, centre prints and the scoreboard. Only
what shows at the moment can be dragged: the vote panel during a vote, the lagometer
with `cg_lagometer` on, the jetpack bar with a jetpack, and so on. The kill feed is
the exception (AngeloPapaya's idea, 03/11/2026): while the mode is open and no kill
shows, it shows a sample, "Killer [saber] Victim", to drag.

The upper right stacks the readouts as JoF EJK's `CG_DrawUpperRight` does, in every
HUD style ([console_overlay.rs](../crates/sjk-viewer/src/console_overlay.rs)): with
`cg_drawFPS` the frame rate ("125fps", JoF EJK's big console characters, measured over
half a second) and, smaller under it, the frame times ("host 2.44 ms · p99 3.83 · max
4.87 · 1% low 115"; Sol's request, 11/10/2026: the long single line, tidier); then with
`cg_drawTimer` with `cg_drawTimerUpperRight` (off by default) the match clock ("0:04.419"
with `cg_drawTimerMsec`, counting down with `cg_drawTimerCountdown`), right-aligned 5 of
480 from the edge, under `cg_drawUpperRight`. Without that option the layouts keep their own
clock at the top centre (Sol's choice, 12/10/2026); with it that clock steps aside. The
game-data HUD draws no clock of its own, so `cg_drawTimer` showed nothing there until the
option existed (Creyon's report, 11/10/2026). The HUD's snapshot line (`cg_drawSnapshot`) and
inventory stand under these readouts, which used to write over the snapshot line. The
kill feed stands under the stack.

In SJK's own layouts, health and armour over the maximum (125 health at spawn, up
to 199 armour from a large shield picked up at 99) show on their meter: the meter
holds one maximum, and what lies over it is a thinner band down the middle in a
deeper shade of the meter's colour, from the start, as on the nameplates. The
game-data HUD draws the original pictures and does not.

Settings > HUD > "HUD look" shows the HUD in use; Left and Right step through
every HUD that can be used and Enter (or a click) opens the HUD picker
([hud_picker.rs](../crates/sjk-viewer/src/settings/hud_picker.rs)): the list on
the left, a picture of the highlighted HUD on the right, drawn for a sample
player over a retail levelshot, and its PK3 or list file under it, the HUD in
use marked. The list holds each installed PK3 that ships `ui/hud.menu` in
install order (the game's own archives as "Jedi Academy", others by file name
without a `zz_` load-order prefix), every other `ui/*hud*.txt` list that
describes a HUD, the text-only HUD, and SJK's radial and classic layouts, which
the live HUD's own code draws over the same levelshot for the same player, so
hovering their names shows them as a match does. Arrows, the wheel or the
pointer highlight; Enter or a click uses the HUD and closes; Escape closes.
Since 19/10/2026 the picker is the SJK UI's pop-up card over the darkened map
in every menu style, as the resolution list is ([sjk-ui.md](sjk-ui.md#settings));
it was a boxed panel in retail's or the theme's colours. "Game HUD files" stays on the tab for lists the picker does not
find.

## Use prompt

What the Use key does at what the player faces (Sol's idea, 30/10/2026), drawn to the
right of the crosshair with a gap (56 pixels at 1080 lines; 34 at first), as many games show it
([use_prompt.rs](../crates/sjk-viewer/src/use_prompt.rs)): a dark key cap naming the
first key bound to `+use` as the key bindings show it ("E", "Mouse 2"; "Use" when none
is), edged in gold, the key centred in it by its ink rather than its line box, then the action in white and a line under it in grey, each over a
dark shadow, in the headline font (Fira Sans Condensed, as centre prints). A prompt that
cannot be acted on now greys its cap and action. It follows the crosshair's middle
(`cg_crosshairX`/`Y` and the dynamic crosshair) and stays on the screen at its edge; it
is hidden with the words in the world (menus, console, scoreboard, intermission).

A feature sets the prompt each frame it applies (`GpuState::want_use_prompt`); the frame
draws the last one set and forgets it. The [mystery boxes](mystery-boxes.md#opening-one)
are its first user, in place of their centre prints.

## Capture the flag HUD

In Capture the Flag and Capture the Ysalamiri every HUD style shows the flags' state
([flag_status.rs](../crates/sjk-viewer/src/hud/flag_status.rs)), reading what
the original cgame reads:

- Flag icons at the left (`cg_drawFlagStatus`, on by default, Settings > HUD), as
  `CG_DrawFlagStatus` places them (32 units at x 2, y 328 and one icon up): the enemy flag
  (`gfx/hud/mpi_bflag` or `mpi_rflag`, the `_ys` pictures in CTY) while your team holds it,
  taken or dropped, and your own flag's `mpi_*flag_x` picture while the enemy holds it. The
  state is `CS_FLAGSTATUS` (config string 23: red then blue, `0` at base, `1` taken, `2`
  dropped); the caption beside each says DROPPED, or for a taken flag its carrier's name
  (TAKEN while the client does not know who). The carrier comes from the viewed player's
  state, a player in the snapshot, a teammate's `tinfo` row, a `scores` row, then the last
  carrier seen while the flag stays taken. `scores` is the only source that names an enemy
  out of sight, and it comes with a scoreboard press, so the scoreboard named a carrier the
  HUD still called TAKEN; only rows that arrived after the flag was taken count
  (`ClientSession::scores_revision`), as older ones may name an earlier carrier. The name
  is formatted again only when the carrier or their
  client info changes, and keeps its colours under `cg_drawCrosshairNamesColours`.
- While the local player carries a flag (`PW_REDFLAG`, `PW_BLUEFLAG` or `PW_NEUTRALFLAG`
  in the player state, whatever the status string says) the enemy flag's icon stands on a
  gold plate with "YOU HAVE THE FLAG". The upper-right powerup icon of `CG_DrawPowerupIcons`
  still follows `cg_drawPowerupIcons` and `cg_drawUpperRight`; it now uses the ysalamiri
  pictures in CTY only (it took CTF for CTY).
- The team scores in every team game but Siege (`cg_drawTeamScores`, on by default): two
  team boxes under the clock at the top centre, red left and blue right, from `CS_SCORES1`
  and `CS_SCORES2` (`-` while the server has not sent one), in the SJK UI's look in every
  HUD style (the design canvas of 24/10/2026): the UI's navy under the team's muted colour
  (`menu::sjk::color::RED_TEAM` and `BLUE_TEAM`, the SJK scoreboard's), a rule of that
  colour along the bottom. Beside a flag the score stands at the inner edge, as wide as its
  digits (smaller past 66 pixels, so "300" or "-12" fit); without flags (team FFA) it has
  the whole box, centred. `cg_teamScoresBoxes 0` (Settings > HUD, "SJK team score boxes")
  brings back the classic look: "RED 3" and "BLUE 5" plates under the clock and the flag
  icons at the left. With the scores on, the layouts' score line drops its trailing team
  scores (`cg_drawScore 2`), which they replace.
- In Capture the Flag and Capture the Ysalamiri, with `cg_drawFlagStatus` too, each box
  holds its team's flag at the outer edge and says where it is: "At base", its carrier
  (named at once from the broadcast `EV_CTFMESSAGE` that `Team_TouchEnemyFlag` raises with
  the taker's number, `CTFMESSAGE_PLAYER_GOT_FLAG`, even out of sight; then as the left
  captions, "Taken" while unknown), "You" in gold for the flag you carry, or for a dropped
  flag the seconds before it goes home ("18 s"), with a gold arc round the flag running
  down. While you follow another player, their flag shows their name, not "You", and the
  line below stays out. A dropped flag goes home by itself 30 s after it fell (`LaunchItem`
  in OpenJK `codemp/game/g_items.c`, kept by JoF EJK and jaPRO; jaPRO's Tribes weapons wait
  45 s, which the client cannot tell, so there the count ends early and the box says
  "Dropped" for the last 15 s): the count starts when the client sees the flag fall, so a
  flag already lying when you join, or one still there after 30 s, just says "Dropped". A
  new gamestate (map change, restart, another server) or the server's clock going back
  forgets the drops and takers seen before. An away flag shows its taken picture
  (`mpi_*flag_x`), or its own picture faint in a pack without one. While you carry a flag, a
  line under the boxes says "Take it home", in gold, or "Your flag is away" while your own
  flag is not at base (no capture until it is). The boxes then replace the left icons and
  the carried flag's powerup pictures (bottom left and in the upper-right column), which
  return with `cg_teamScoresBoxes 0` or `cg_drawTeamScores 0`. The boxes are compact: 168 by
  42 pixels of a 1080-line screen, the score 36; `cg_teamScoresScale` (Settings > HUD, "Team
  score box size", 0.5 to 2, default 1) makes them smaller or larger, under the clock as
  before.

The scoreboards (classic and SJK UI) draw a flag icon before a carrier's name. A
`scores` reply is only as fresh as the last scoreboard press, so each frame the rows'
flag bits follow live state: the viewed player's own state, a player in the snapshot, a
teammate's `tinfo` row, then the `scores` row; `CS_FLAGSTATUS` clears a flag that is back
at its base or dropped
([live_flags.rs](../crates/sjk-viewer/src/scoreboard/live_flags.rs)). SJK's own server
sends the carriers' powerup bits in `scores` (`bridge_ranks.rs`), as
`DeathmatchScoreboardMessage` does.

A flag carrier wears the flag on the back (`CG_PlayerFlag`). The local carrier's
flag comes from its player state, as the server never sends a client its own entity,
so it shows in third person
([flag_carrier.rs](../crates/sjk-viewer/src/flag_carrier.rs)).

## Vote panel

While a vote runs (`cg_drawVote`, on by default) the HUD shows it at the top centre, in every HUD
style ([vote.rs](../crates/sjk-viewer/src/hud/vote.rs), geometry in
[menu_widgets/vote.rs](../crates/sjk-viewer/src/menu_widgets/vote.rs)): a 560-pixel (1080-line)
card, centred on the panel's anchor, with a thin bar of the 30 seconds left
along its top, VOTE (or TEAM VOTE) and the seconds left, the proposal in large centred text,
a bar of the YES / NO share of the votes cast, the YES count in green at the left and the NO
count in red at the right, and under each the keys bound to `vote yes` / `vote no` (the console
command when nothing is bound). A team vote's card stands under the global one. It is drawn as SJK's
menu cards are (navy plate, holo edge, gold mark and time bar), in the HUD's font even under
`ui_gameFont`, and stands under the match timer. The HUD edit mode drags it during a vote.

`testvote` shows a made-up vote on the panel for 30 seconds, its counts growing, for
seeing and moving the panel where no vote can run (alone on a devmap a vote passes at
once, as bots do not vote); `testvote team` adds a team vote's card, `testvote off` ends it.
Only the HUD draws it: nothing is sent to the server, and a real vote keeps its card.

## Kill feed

`cg_killfeed` (on by default, Settings > HUD > "Kill feed") lists the last kills
at the top right of the HUD, or where the player moved it (the HUD edit mode, or
`cg_killfeedX` leftward and `cg_killfeedY` downward in 640x480 units), in every HUD style
([kill_feed.rs](../crates/sjk-viewer/src/hud/kill_feed.rs)). Each entry is
`killer [icon] victim`, or `[skull] victim` for a suicide or a death to the world
(a fall, lava, a trigger), on a dark plate; an entry where the viewed player
killed or died has a stronger plate and a white outline. Names are read from the
players' info when the kill arrives, as the console's kill message is, and keep
their colour codes; they are not tinted by team, as the console, chat and
scoreboard draw them. A name too long for its room ends in an ellipsis.

The icon is the HUD's picture for the means of death: the weapon's own
`gfx/hud/w_icon_*` (the saber's `w_icon_lightsaber`), mapped from each `MOD_*` to
the weapon whose shots carry it in OpenJK's `g_weapon.c` (the sentry gun has its
item picture), or an installed icon pack's `hud/mod/*` picture, which takes its
place. A dark Force kill (`MOD_FORCE_DARK`, lightning or grip) without such a
picture shows Force Grip's holocron when only the killer's grip was active in the
snapshot of the kill (`ObituaryEvent::attacker_force`), else Force Lightning's, and a player
knocked to their death (`KILLED_FORCETOSS`) Force Push's. Causes no weapon deals
(water, lava, crushing, telefrags, unknown causes) show the skull, which is drawn
from shapes, not a game picture. A weapon whose picture did not load shows a
short word instead (`SABER`, `ROCKET`, `FORCE`).

Up to five entries show, the newest at the top so it is always right under the
readouts above; each holds five seconds of server time and fades out over one.
The feed stands under whatever the HUD already
draws at the top right — the FPS readout, duel portrait, snapshot,
inventory and powerup column — and never reaches left of the vote panel's right
edge, so it does not cover the top centre's vote and timer. It is part of the
HUD's draw list, so it hides whenever the HUD does: scoreboard held,
`cg_drawHud 0` or `cg_draw2D 0`, intermission, menus and the console. Moved
(AngeloPapaya's idea, 03/11/2026), it may stand anywhere: once its right edge is less
than 200 pixels (1080-line) right of the vote panel's edge, it no longer keeps clear of
the top centre but takes a column as wide as its longest entry (two names at their
longest, the widest cause word and the gaps), held inside the screen; when that column
lies mostly in the screen's left half, the entries line up on its left edge instead of
its right, and the top right's readouts no longer push it down. Names draw
in the HUD's font (Inter, or SJK HUD with `cg_classicHudFont`), which the feed
measures them in.

| Cvar | Default | Effect |
| --- | --- | --- |
| `cg_killfeed` | 1 | Show the feed |
| `cg_killfeedX`, `cg_killfeedY` | 0 | Move it left and down, in 640x480 units |
| `cg_killfeedTextSize` | 0.8 | Name size; 0.8 is the normal size, 0 uses it |
| `cg_killfeedIconSize` | 12 | Icon size in 640x480 units; 0 uses 18 |
| `cg_killfeedColors` | 0 | Tint icons by cause of death (TaystJK's colours) |

The feed replaces an earlier one-line obituary at the same cvar, which was off by
default; a profile that saved that 0 has it moved once to 1
(`cg_killfeedDefaultVersion`), and that feed's `cg_killfeedAlignment` and
`cg_killfeedReverse` are gone.

## Nameplates

`cg_nameplate` (off by default since 17/10/2026; Settings "See & share nameplates") draws MMO-style
nameplates over players, and shares your own health, shield and Force with the other
SJK players of your server ([Sharing and estimates](#sharing-and-estimates))
([nameplate.rs](../crates/sjk-viewer/src/hud/nameplate.rs); layout in
[nameplate_math.rs](../crates/sjk-viewer/src/hud/nameplate_math.rs)): 2D HUD shapes
and text at the projected point 8 units above the head of the player's box (decoded
from `entityState_t::solid`, so a crouching player's plate drops). Names come from the
chat roster with their colour codes and are drawn in the classic HUD font (Inter when
that font is not loaded), whatever `cg_classicHudFont` says. A dark letter (luminance
under 0.3: black `^0`, and the game's dark blue) keeps its colour, edged in a pale halo,
the glyph drawn a step out all round in a light tint instead of its drop shadow, so a
black name reads on the see-through plate (`text::CodePalette::Haloed`, Sol's pick of
28/10/2026 over lightening the colour). The halo is faint, each of its eight copies 18%
opaque (38% until 29/10/2026, when Sol found the outline too strong).
`cg_nameplateOutline` (Settings "Name outline", Lex's idea, 29/10/2026: some names hard
to read on busy backgrounds) chooses the edge: 0 Off, the drop shadow only
(`CodePalette::Game`); 1 Faint, the default, the halo above (`Haloed`); 2 Strong, the
halo at 38% and every other letter ringed by eight dark copies (0.45 opaque each)
instead of its shadow (`Outlined`, `text::Edge`).
`cg_darkTextOutline` (default 0; Settings > Chat "White edge on dark text", which
applies to the nameplates too; Sol's request, 03/11/2026: the halo reads as a glow) swaps
the halo for a crisp white outline, the same eight copies in white at 85% each
(`CodePalette::white_outlined`): with Faint, dark letters in white and the rest in
their shadow (`WhiteEdged`); with Strong, dark letters in white and the rest in the
dark outline (`TwoTone`). Off keeps the shadow only. It applies to the chat too (see
`cg_chatBoxOutline`).

**Styles** (31/10/2026, Sol's decision to let players choose among the designs SJK has
had): `cg_nameplateStyle` (Settings "Nameplate style", and the Style row on the
Collection's Nameplates tab, [sjk-ui.md](sjk-ui.md#collection)) picks the plates' look,
`hud::nameplate::Style`: 0 Holo, the holo frame below (the default, the look since
28/10/2026); 1 Capsule; 2 Classic. What a plate says is the same in all three (the
name and its mark, the power icons, the duel badge, the bars and the sharing and duel
rules below); only its shape differs, and the classic plate has no picture.

- **Capsule** (design E of 28/10/2026, the look that morning,
  [capsule.rs](../crates/sjk-viewer/src/hud/nameplate/capsule.rs)): every plate is a pill
  26 design pixels tall (the design's name 13), dark navy 55% opaque, or the team's
  colour made deep in team games (`capsule::team_fill`, 62%), edged gold for a sharer,
  a pale grey otherwise, the duel's colour (1.5 pixels) in a duel, a paler gold on NPC
  plates. Left to right it holds the round picture (20 design pixels near, smaller far;
  the same pictures as the holo frame's), the name with its mark and, at its right end
  in a duel, the duel badge. A sharer's bars stand on a tray 104 wide tucked under it
  (shield 2, health 3 and Force 2 design pixels, rounded, on a faint track), the pill
  widening and rising to hold it as the bars fade in; a player who does not share, and
  a duellist, have the pill alone. Far away the pill fades, leaving the picture and the
  name centred over the player. The power icons stand over the pill's middle in square
  cells (22.5 design pixels).
- **Classic** (the plates until 28/10/2026,
  [classic.rs](../crates/sjk-viewer/src/hud/nameplate/classic.rs)): no picture. The
  name is centred over the player, the duel badge before it and the mark after it,
  level with its capitals. Nearer, a dark plate (62% opaque, 112 HUD pixels wide) fades
  in under the name, which rises over it: rounded bars (shield 5, health 7, Force 4
  pixels) on dark tracks, each outlined, the plate framed in gold for a sharer (a
  team's colour as a thin line inside it) and in the team's colour otherwise. A player
  who does not share gets grey bars the whole way, so they read as hidden rather than
  empty, as before 28/10/2026; a duel opponent's bars never show. The power icons, as
  tall as the name, stand over it. The weapon icon the classic plates had is not back:
  it went in every style on 28/10/2026.

The rest of this section describes the holo frame; the bars, badges, modes and rules
hold in every style.

**The holo frame** (28/10/2026, design C of the nameplate designs Sol chose from, in
place of the capsule, design E, the same day; `cg_nameplateStyle 0`): every plate, bars or not, is a hexagon
holding the player's picture over the left end of a plate whose top-right corner is cut
(`nameplate_math::Holo`), so a player who does not share still has a plate. The plate
holds their name with its mark after it and, under it, the bars of a sharer, or a dim
dotted line (3 design pixels on, 4 off) on a plate without bars. Until 28/10/2026 the
weapon they held stood after the name (`cg_nameplateWeapon`, dropped from saved
profiles): Sol found it told nothing. The frame's colour says who they are: the holo
blue (`rgba(110, 210, 255)`) for an SJK player, grey for a player without SJK (one the
hub does not know on this server), the team's red or blue in team games, the duel's
colour in a duel, a paler gold on NPC plates. The plate is that colour made dark (a
tenth of it, `nameplate_math::plate_fill`), 55% opaque (the design's 78%: Sol asked for
more see-through plates), under faint scan lines of the frame's colour every 3 design
pixels (not on a grey frame, an NPC's or in a duel), and edged in it; a player who
shares has a gold corner along its top to the cut and down its right side. The
measures are the design's, scaled by the name's size over its 15 pixels (an 18-pixel
name at 1080 lines with the default `cg_nameplateScale` 0.5): a plate at least 150
wide, 40 tall with bars (a 44 by 40 hexagon overlapping it by 12) and 30 without (38
by 34, overlapping by 10). The hexagon and the plate are convex polygons, a draw
command new for them (`DrawCommand::Polygon`, [rendering.md](rendering.md#submission-and-lighting-work-reduction)).

- **Picture:** the player's SJK hub picture in the hexagon, two design pixels inside the
  frame's colour ([identity.md](identity.md#pictures)). The pictures are kept cut round,
  as the chat and the cards show them; the hexagon takes the part inside the disc, its
  corners just inside it (`nameplate_math::HEX_UV`). an SJK player without one, or before it loads,
  gets their stand-in, the initial of their hub name on their colour; a player without
  SJK gets their model's head icon (`models/players/<model>/icon_<skin>`, the player
  card's), cut round; a model without one (one this PC does not have) gives the initial
  of their name instead. Verified players get no gold ring round the picture: the seal
  after the name says it. NPCs have no picture. Which slots the hub knows is read once a
  second with the marks (`player_identity::slot_pictures`: the claim's name matching the
  name shown there, your own slot from your profile); each frame every plate's picture
  is asked of the pictures' cache under one lock (`avatars::textures`). A head icon is
  decoded and cut on the frame thread, one a frame at most and each model once while it
  stays in the cache (`hud_runtime::cut_nameplate_head`, kept with
  `avatars::insert_local`); one that could not be had is asked again after 2 seconds.
- **Far:** the plate fades out with the bars' ramp, leaving a small hexagon (17 by 15
  design pixels) and the name, at 85%, centred over the player above a hairline in the
  frame's colour, 46 design pixels long. Plates shrink with
  distance (to 60% at the range) and fade over the last quarter of `cg_nameplateRange`
  (3000 units).
- **Near:** inside `cg_nameplateNear` (1000 units) the plate fades in, and for a
  player whose bars show (a sharer, an NPC, a teammate the server reports) it grows from
  30 to 40 design pixels as the bars fade in under the name and the dotted line fades
  out. The shield bar is on top, then health, then Force (3, 5 and 2 design pixels on a
  track of the frame's colour, never under 2 pixels at 1080 lines); shield and health
  are cut into segments of about 9 design pixels with 2 between, stretched so the bar
  holds whole ones (`nameplate_math::segments`), Force is whole. The bars take the HUD's
  own colours (the layout's
  `armor_ratio`, `health_ratio` and `force_ratio` widgets: the Radial HUD's green
  shield, red health and blue Force; the retail green, red and light blue for the
  game-data HUD, which draws pictures). Health is always that red, whatever is left.
- **Empty shield:** a player's shield bar is always there; known to be empty it is
  drawn broken, grey dashes (4 design pixels, 3 between) the bar's height.
- **Overheal and overshield:** health and shield go to twice the maximum (125 at
  spawn; 199 shield from a large shield picked up at 99). The bar shows 0 to the
  maximum; what lies over it is a second, inner band in a deeper shade of the same
  colour, from the left (125 health: a full red bar with a deep red first quarter).
  Both decay a point a second back to the maximum, as the server does.
- **Unsure:** a health or shield estimate whose range spans 60 or more (a player
  first seen, or back from long out of view) dims and gets a yellow "?" over it until
  a pain, a shield hit or another clue narrows it.
- **Uncertainty:** an estimated bar is a range ([estimate.rs](../crates/sjk-viewer/src/hud/estimate.rs)):
  the lowest and highest the value can be and the best guess. The bar fills to the
  guess, and a grey haze, thickest at the guess and fading out towards either bound,
  covers the uncertain stretch, so a loose estimate reads as a blurred edge and an
  exact one as a sharp edge.
- **Verified badge:** a player the SJK hub's operator vouches for (see
  [Identity](#identity)) gets a gold seal with a white tick after the name, drawn at
  start into an icon cell ([verified_badge.rs](../crates/sjk-viewer/src/ui_renderer/verified_badge.rs)).
  The name's capitals are centred on the name's row as the plate's font lays them
  out (`UiFont::capital_middle`; the HUD font hangs its glyphs low in the line), and the
  seal with them.
  Which slots are verified is read from the identity service once a second.
- **Duel badge:** a player in a private duel (`bolt1`, your own `duelInProgress`
  on your own plate) gets two crossed lightsabers, white blades in a coloured glow on a
  disc filled and ringed in the same colour (so the colours tell apart at a nameplate's
  size), on the hexagon's bottom-right corner; the frame takes the badge's colour, its
  dotted line too, and a duellist's plate has no bars. **The two duellists of one duel
  share a colour** (blue, orange, green, magenta, yellow, cyan, red, in the order duels
  start), so in a crowd the matching badges show who duels whom. The pairs come
  from [duel_pairs.rs](../crates/sjk-viewer/src/hud/duel_pairs.rs):
  - the server's broadcast when a duel starts, `"<challenged> @@@PLDUELACCEPT
    <challenger>!"` to every client (`Cmd_EngageDuel_f`, `g_cmds.c`; jaPRO and TaystJK,
    as JoF runs, add `^7` after each name and the duel type after the `!`, such as
    `(Saber) [Ranked]`), its names matched
    against `CS_PLAYERS` (decoded as the print is, exactly, else without colour codes; a
    name two players share pairs nobody);
  - your own duel, from your player state.

  Nothing is guessed: a duellist whose opponent SJK was never told (a duel that began
  before you joined, or a mod that words the message otherwise) wears a neutral grey
  badge rather than a colour that might pair the wrong two.

  A pair keeps its colour until the snapshot shows one of the two no longer duelling (a
  pair named while out of view waits for them; one named but shown not duelling lapses
  after 3 s), and every pair is forgotten with a new gamestate (map or server) or a demo
  seek. Each colour is its own icon
  cell, drawn at start ([duel_badge.rs](../crates/sjk-viewer/src/ui_renderer/duel_badge.rs)).
  The badge sits level with the name's capitals like the verified badge, and stays at
  least 90% opaque on a far plate whose name dims, smaller with the far hexagon.
  `cg_nameplateDuel` (on; Settings >
  Quick > Nameplates > Duel badge) turns it off, and it shows with the name, so
  `cg_nameplate 0` hides it too. The duellist's body is drawn as JoF EJK draws it (see
  [Entity render effects](rendering.md#entity-render-effects)).
- `cg_duelHideOthers` (1, Auto; Settings > Advanced > Gameplay > Hide others in
  duel) hides everyone but you and your duel partner while you, or the player you
  follow, are in a private duel, as JoF EJK does (EternalJK `CG_Player`,
  `cg_players.c:11983-11990`). The rule is per server family, in the table
  `DUEL_HIDE_SERVERS` ([duel_hide.rs](../crates/sjk-client/src/duel_hide.rs)): JA+
  hides unless your `cp_pluginDisable` sets bit 1 (Settings > Mods, See others in
  duels, or `japlus.plugin 1`; SJK's default 1536 leaves it clear, Off), since a JA+
  server sends bystanders to a
  plugin client and expects it to hide them; jaPRO (TaystJK) always hides; other
  servers do not. Other players' [drones](#drone) are hidden in a duel on every
  server, unless the setting is 0. `2` (Always) hides on every server, `0` (Off) never, and they are
  then drawn very dark as in stock (`legacy_duel_tint`). Hiding leaves them out of
  what is presented, as if the server had not sent them: their bodies, held sabers
  with blade glow, trails and lights, thrown sabers (by the saber's
  `genericenemyindex` owner), Force effects, sounds and loops, nameplates, every NPC
  but your own vehicle, and the one-shot events EJK skips for a non-duellist
  (`cg_event.c`: footsteps, falls, jumps, rolls, taunts, saber swings, hits and blocks
  by the wielder, unholster, zoom, teleport in and out, pain and death; table
  `EVENT_ACTORS`). Missiles are hidden by their `owner` only on a family whose table
  entry says the server sets it (jaPRO, `CG_DuelCull`); stock and JA+ leave it unset,
  so their shots stay. Prediction (duellers already pass through bystanders,
  [duel_isolation.rs](../crates/sjk-client/src/duel_isolation.rs)), the HUD,
  scoreboard, kill feed and recorded demos keep the snapshot as received; demo
  playback hides by the same setting.
- `cg_duelStockBlades` (off; Settings > Gameplay > Normal blades in duels, and Game
  options), when on, draws every player's blade as the stock one, in the colour they set
  (a stock colour or RGB) and with the stock sounds, instead of their blade skin, while you, or the
  player you follow, are in a private duel, or on a Duel or Power Duel server
  (`g_gametype` 3 or 4), Sol's request of 21/10/2026: nothing round a blade hides where it
  is or how it swings ([duel_blades.rs](../crates/sjk-viewer/src/duel_blades.rs),
  `SaberSkins::set_plain`). Team and black blades follow their own settings as usual.
  Off by default (Creyon: skins stay on in duels unless a player asks). It changes only
  what you see; the other duellist sees what their own setting says.
- `cg_stockBlades` (0, Off; Settings > Advanced > Gameplay > Effects > Stock blades, and
  the classic Game options: Off, Others, All), Sol's request of 25/10/2026 for weak PCs
  (GTX 1050/1060 class, integrated GPUs): `1` draws other players' blades as stock blades in the colour they set (a stock
  colour or RGB), without their blade skin; `2` every blade, yours included. A blade skin
  costs about 0.4 to 0.6 ms of GPU time on an RTX 5080 once one is in view (a heavy
  signature up to 6 ms), so several milliseconds on a weak GPU on a full saber server.
  Everything the skin draws goes with it (its shading, afterimages, glints, petals, heat
  haze, black-hole lenses, solid parts, its trail and light colours), and the skin passes
  are skipped once no skinned blade is left in the frame
  ([rendering.md](rendering.md#saber-blade-skins)). Thrown sabers follow their owner.
  Sounds stay as each player chose them. The menus keep showing skins: the Character and
  Collection stage, the cards and the blade pictures never take them from this setting
  ([stock_blades.rs](../crates/sjk-viewer/src/stock_blades.rs),
  `SaberSkins::set_stock`). With `1`, a player you follow is another player. It changes
  only what you see. Team, black and duel blades follow their own settings as usual.
- `cg_hideCorpsesInDuel` (on; Settings > Advanced > Gameplay > Hide corpses in
  duel) keeps dead bodies out of the presentation world while the local player is
  in a private duel. It affects `ET_BODY` only: live players and gameplay state are
  unchanged, and bodies remain visible outside a private duel. Set it to `0` to
  keep bodies visible during duels too. The archived preference also applies to
  demo playback.
- `cg_hidePickupsInDuel` (on; beside the corpse option) similarly hides
  `ET_ITEM` and `ET_HOLOCRON` presentation during a private duel. That covers
  health, shield, weapons, ammo, powerups and Force holocrons without changing
  their server state or pickup behavior. Set it to `0` to keep them visible.
- **Pilots in a vehicle:** a pilot sealed in a `hideRider` vehicle (a fighter, a
  walker) is `Ghost`ed with `SVF_NOCLIENT` (`g_vehicles.c:1924-1940`), so no snapshot
  carries his entity and the world never presents his body. Like stock's crosshair
  target name (`CG_DrawCrosshair`), the plate follows the vehicle's `owner` while it
  is below `MAX_CLIENTS`: the name and team come from his `CS_PLAYERS` string and the
  plate hangs over the vehicle's presented position and box top. Only the plate, with
  his name and team's colour, shows (no picture, bars or icons: the client has no
  state of him). A rider
  whose entity is sent keeps his own plate, anchored to the vehicle if his body is
  hidden (`nameplate_math::anchor`, `hidden_pilot`).
- `cg_nameplateSelf` (off): your own plate over your head in third person, with your
  real health, shield and Force (the server sends you those) and your badge when your
  key is verified. In first person it is not drawn: it would sit in
  the camera.
- **Modes (V):** the `nameplates` command, bound to V by default (Controls > Other >
  "Nameplate mode"; an existing config gets it when V is free), cycles off, names
  only, target, everyone, and round again, showing the mode's name near the top of the
  screen for a moment; `nameplates off|names|target|all` picks one. Target shows bars
  (and the power icons) only on the player you aim at, kept for three
  seconds after the crosshair leaves them. Names only is
  just the names. The modes set `cg_nameplate` and `cg_nameplateBars` (0 names, 3
  target, 2 everyone); 1 (allies only) stays reachable in the settings.
- `cg_nameplateBars`: 0 none, 1 allies only, 2 everyone (default), 3 target; never on
  a duel opponent ([Sharing](#sharing-and-estimates)). `cg_nameplateScale`
  sets the text size, `cg_nameplateAlpha` (Settings "Nameplate opacity", 0.1 to 1,
  default 1) how solid the whole plate is in every style, times its distance and wall
  fade, `cg_nameplateWalls` shows
  players behind walls at 35% opacity instead of fading them (the same BSP trace as
  before, from the rendered eye; changes ease over 120 ms), and `cg_nameplateNpcs`
  adds NPC plates (class name; at most 16; vehicles skipped). Their health bar
  requires level 3 Sense to be active, unless the mapper explicitly enabled the
  NPC's health display (`showhealth`, sent as `shouldtarget`). NPC names still
  show without Sense; ordinary bar-mode settings still apply. This applies in
  every HUD and menu style, including demo playback.
- `cg_nameplateHideDuelers` (off; "Hide duellers" under Nameplates in SJK Settings >
  Advanced > HUD and classic Settings > HUD): hide the entire plate (name, bars and
  icons) on players the server marks as participating in a private duel. This also
  hides your own third-person plate during a private duel. It uses the player's
  snapshot duel marker, not the Duel/Power Duel game type or team. Players whose
  entities are omitted from snapshots, such as sealed vehicle pilots, have no
  available private-duel marker and keep their plates.
- `cg_nameplateIcons` (on): up close, a row of the holocron icons of the Force powers a
  player has on stands over the plate's right end, each on a square cell (55% opaque,
  corners of 2 design pixels, edged in the frame's colour at half strength, the icon a
  pixel in from its edge), 22.5 design pixels a side (the design's 15, half as large
  again), at most four, dark side first (lightning, grip, drain, rage, then protect,
  absorb, speed, heal, team force, mind trick, sight). They come from
  `forcePowersActive`, so every player's powers are known without a request; jump,
  push, pull and the saber powers are left out because they last a moment. The pictures
  are the Force bar's, loaded with the map. From 28/10/2026 to that evening they sat on
  smaller round chips (15) with a pale blue ring; Sol had the square cells, as before
  the capsules, back, half as large again.
- `cg_nameplateDebug` logs, every two seconds, what the server sends about each other
  player (health, `tinfo`, active powers, weapon and stance) and the estimates (health,
  armour and Force as `guess[low..high]`) to `logs\last-client.log`, with the regen
  pace in use and where it came from, the server's Force-related info keys, whether
  its pain events carry real health, and your own real Force next to the estimate
  for yourself.
- While a menu is open the plates hide: their text is in the classic stream, which
  draws over the menus' text.

### Sharing and estimates

Another player's health, shield and Force bars show only when that player shares
them (decided 15/10/2026; since 17/10/2026 with no estimates on anyone else), and
seeing any of them means sharing your own, through the hub (Sol's rule, 15/10/2026):

- **See & share nameplates** is one setting, `cg_nameplate` (17/10/2026, off by default;
  Settings > Interface, First setup, and the switch on the Collection's Nameplates tab,
  [sjk-ui.md](sjk-ui.md#collection)): with nameplates on, whatever bars they show
  (names only too), your client sends your real health, shield and Force (from the
  snapshot the server sends you, with its server time) to the SJK hub, which passes
  them to the other SJK players of your server ([hub-chat.md](hub-chat.md#vitals));
  off, you see no nameplates and nothing is sent (`hud::nameplate::see_and_share`). It
  needs `cl_identity` and a live claim, so it starts a few seconds after joining. It
  replaces `cg_nameplateShare` ("Share my bars"), which the client held on while
  nameplates showed bars: two settings for one value, and in Settings the row looked
  stuck off, since the hold put the cvar back the next frame while the row still read
  OFF. A `cg_nameplateShare` line in an older `config.cfg` no longer does anything.
  Every profile had saved nameplates on (the old default), so the release asks players
  to choose it again (`= cg_nameplate reset: ...` in the changelog): the what's-new card
  puts it back off and opens on it with its switch, and the player turns it on again
  knowing that it shares ([sjk-ui.md](sjk-ui.md#news-panel)).
- **Duels** (Milamber's rule, 18/10/2026): nobody sees a duel opponent's health,
  shield and Force, and a duellist sends nobody theirs, whatever either side's
  settings (`hud::nameplate::Duel`). In a private duel (`duelInProgress`) the opponent
  is the one your player state names (`duelIndex`); in the Duel game type every other
  player in the arena; in Power Duel the other side (`dt` in `CS_PLAYERS`); a
  spectator duels nobody. The opponent's plate keeps its name and icons but
  shows no bars (not the grey ones either), in every bar mode; target mode no longer
  shows bars on the duel opponent. While you duel your client sends the hub no values
  (as when dead), so nobody on the server, bystanders and a Power Duel partner
  included, sees your bars until the duel ends; the hub still lists you as sharing, so
  you keep seeing bystanders'. The server's own duel health (`g_showDuelHealths`,
  `CS_CLIENT_DUELHEALTHS`, on the scoreboard and the duel opponent display) is the
  server's choice and is not affected.
- **Tied to the hub:** shared bars on other players show only while the
  hub's last answer (under 6 seconds old) lists you sharing in your slot. With
  `cl_identity` off, no hub, no claim yet or the hub down, other players get no such
  bars; the server's own (teammates' `tinfo`, NPCs) stay. The hub itself gives the
  values only to players who share: a post without `share` gets who shares, not
  their numbers.
- **Players who do not share** (Sol's rule, 16/10/2026 for SJK players, 17/10/2026
  for everyone, bots and players without SJK too): no bars for any viewer, health,
  shield or Force. Nothing overrides it, not even a teammate's `tinfo`. Their plate
  stays, with its picture, name and power icons, and the dim dotted line under the name
  instead of bars (until 28/10/2026 they had grey bars, full length and empty, which
  the classic style keeps; the capsule shows the pill alone).
  NPCs keep their bars.
- **Sharers:** their health and shield bars are the estimates below, corrected with
  every value they share ([estimate.rs](../crates/sjk-viewer/src/hud/estimate.rs) `History`). A
  shared value is up to about a second old by the time it arrives, so it re-bases the
  estimate as it stood at that server time: what was seen since (a hit, a pain, a
  heal) is kept on top, and the doubt the estimate had then is dropped. With nothing
  seen since, the bar is exact; it never waits for the next value to move. Their
  Force bar is the last Force they shared, not estimated (17/10/2026,
  `hud::nameplate::shared_force`), so it moves with each value, about four times a second.
- **Gold corner:** the plate of a player who shares (See & share nameplates) has a
  corner in the verified badge's gold, along its top to the cut and down its right
  side (until 28/10/2026 the capsule's whole edge was gold); in team games the frame
  takes the team's colour. Your own plate (`cg_nameplateSelf`) gets it
  too while the hub lists you sharing. NPC plates (`cg_nameplateNpcs`) are edged a paler
  gold.
- `cg_nameplateEstimate` ("Bars on everyone", estimates on every player without
  SJK) was removed on 17/10/2026, when they turned grey. A client from before may
  still tell the hub it sees everyone's bars: their player card (`inspect`) says, in
  amber, "Sees your health, shield and Force". A sharer's card says "Shares their
  health, shield and Force".
- The hub cannot check what a client says: a modified client could share false
  values or estimate without saying so. The protocol keeps honest clients honest.

Where the numbers come from, and what is not known:

- **Teammates** in team games: health and shield from the server's team overlay data
  (the `tinfo` command), exact. The client asks for it (`teamoverlay` userinfo)
  whenever nameplate bars are on; SJK draws no team overlay of its own.
- **NPCs** (and any entity whose `entityState_t::health`/`maxhealth` the server
  sets): exact. The SJK server sets them for NPCs, breakables and emplaced guns, not
  for clients; stock JKA does not either.
  Ordinary NPC nameplate health is displayed only with active Sense 3, with the
  mapper-enabled exception above. This is a display rule; it does not change
  which health fields the server sends or estimate missing NPC health.
- **Sharers' health and shield** are estimated
  ([vitals_estimate.rs](../crates/sjk-viewer/src/hud/vitals_estimate.rs)) once per
  snapshot (every one, so no event is missed) from what the server sends everyone,
  with its own rules:
  - spawn at a quarter over the maximum health with a quarter of it as shield (125
    and 25; 100 and none in Duel), learnt from your own spawns for mods that change
    it; health and shield over the maximum lose a point a second;
  - `EV_PAIN` carries the health left after a hit of ten or more (at most every
    700 ms): exact. `EV_SHIELD_HIT` carries the shield a hit took: exact.
    `EV_SABER_HIT` sizes the damage (under 5, under 20, more); a missile striking
    (`EV_MISSILE_HIT`) is a hit of unknown size, except JA+'s grapple hook (the only
    missile fired as `WP_STUN_BATON`), which does exactly one point. With no pain after a hit outside the
    700 ms, it was under ten. A hit with no shield flash met no shield;
  - when you hit someone, the server tells you their health and shield before the
    hit (`PERS_ATTACKEE_ARMOR`), matched to them when only one player fits (your
    saber's victim, your duel opponent, or the only estimate that allows the values);
  - falls (`EV_FALL`/`EV_ROLL`, exact), medpack and shield pickups
    (`EV_ITEM_PICKUP`), Force heal (its sound at the player: 5 to 25), team heal
    (`EV_TEAM_POWER` names everyone it reached: 50, 33 or 25 each by how many),
    deaths (`EF_DEAD`, `EV_OBITUARY`) and the respawn after one; a spawn with no
    death seen toggles `EF_TELEPORT_BIT`, as a teleport does, so it only raises the
    high bound;
  - drain, lightning and grip, rebuilt shot by shot (see **Force streams** below):
    drain heals the drainer by what it takes, lightning does 1 or 2 a shot (twice
    two-handed at level 3) to the shield first, whose flash counts it, then the
    health, and a bolt seen landing with no flash means no shield was left; grip
    does 2 a second past the shield (its crushing blow is a pain);
  - rage costs 2 health every 150, 300 or 450 ms by level (the guess level 3), down
    to 1, and halves every blow and fall before the shield; protect keeps 40, 60 or
    80% of a blow off the health (the guess 80%) and pays for it with Force (not
    estimated);
  - private duels (`EV_PRIVATE_DUEL`, 1 at the start, 0 at the end): on JA+ both
    duellists are set to 100 health and 100 shield when it starts, and the winner to
    100 and 25 when it ends, whatever they had left; on stock JKA the start changes
    nothing and the winner is healed to the maximum. A duel ending within a second of
    a death was won; one ending otherwise (called off, too far apart) changes
    nothing. The start values are learnt from your own duels once you have had one,
    for servers that set others (jaPRO's `g_duelStartHealth`);
  - JA+'s chat protection: a player with the chat balloon up (`EF_TALK`, whatever
    opened it) takes no damage unless in the middle of an action (a saber swing,
    its transitions or a special, a kick, a punch or a melee grab, read from the
    saber move and the torso and legs animations). On JA+ a hit seen on such a player
    without a pain or a shield flash takes nothing, and a fall takes nothing; a pain
    or a shield flash still counts, since the server only sends one when damage landed;
  - a player first seen, or back after time out of view, gets a wide range that the
    next pain closes.

  Servers can hide the pain values: JAPro's `g_stopHealthESP` (off by default) sends a
  fixed 50 or no pain event at all, and scrambles `PERS_ATTACKEE_ARMOR`. At 2 it plays
  the pain sound instead (`EV_ENTITY_SOUND` of `*pain25` to `*pain100`, chosen by the
  health left), which bounds the health to its quarter: 25 or less, 50 or less, 75 or
  less, or more. Your own pain
  events are checked against your real health: until one has been, pain values are
  trusted but their absence proves nothing; if they do not match (or every pain says
  50), they are ignored and a pain only says ten or more landed. JA+ is closed source
  and not verified.
- **Force** is not estimated (17/10/2026): a sharer's Force bar is the value they
  share, and a player who shares nothing has the grey one. The estimate that rebuilt
  every player's pool from their entity state (`force_estimate.rs`: the regeneration
  pace measured from your own pool, each power's price and running cost, force jumps,
  saber specials, team powers, absorb and refused specials) was removed with its
  setting, `cg_nameplateForce` ("Force bar (estimated)"); the bar shows whenever the
  plate shows bars.
- **Force streams** ([force_streams.rs](../crates/sjk-viewer/src/hud/force_streams.rs))
  rebuild drain, lightning and grip shot by shot, as `ForceShootDrain`,
  `ForceShootLightning` and `DoGripAction` do it. Drain and lightning shoot every
  50 ms: level 3 reaches everyone within 512 units (lightning 300) of the caster's
  origin, in a 60-degree cone along its view and in clear sight of the map's walls,
  levels 1 and 2 the first player on a 2048-unit line. A drain shot takes 2, 3 or 4
  Force by level, heals the drainer as much, stops the victim's refill for 800 ms,
  and costs the drainer 5 (at levels 1 and 2 only when the line found a player) and
  its own refill for 500 ms; while drain stays on the drainer holds at least 20. A
  lightning shot does 1 or 2. Against an absorb that is up, what the levels'
  difference leaves gets through and a point of Force comes back a shot. A grip holds
  the first player on a 256-unit line from the gripper's eyes when it starts and
  hurts them 2 then and every second. Other players' levels are not sent: the guess
  is level 3, the bounds the dearest and cheapest level that reach. Your own levels
  are your Force profile's, so your drain and lightning on others are exact.
  `EV_FORCE_DRAINED` (at most every 400 ms a victim) says a victim lost at least a
  drain shot, and a renewed electrification (`emplacedOwner`, which lightning sets
  800 ms ahead) while somebody casts lightning that a bolt landed, even where the
  geometry missed it. Doors, movers and other players in the arc's line of sight are
  not checked.

  Your own drain is measured, not rebuilt: the server sends you your pool, which
  each shot takes 5 from, and your health, which each shot raises by what it took
  (below your maximum). Your victims lose what you gained (at the maximum, the shots
  you paid for times what a shot takes), shared among those in reach, else those
  the drained event names, else the player most in front. Your drains also teach
  the server's pace (milliseconds a shot) and a shot's strength, which then apply to
  everyone's drain and lightning: servers built on the original game (JA+) shoot
  every server frame rather than every 50 ms. Each of your drains writes one line
  to the log (`nameplate drain:` with its shots, heal, victims, how they were found
  and the pace).

`cg_drawPlayerNames` keeps TaystJK's plain overhead names (0 off, 1 names, 2 adds a
health strip, text only, off by default); they are hidden while nameplates are on.
Both stand above an ally's team logo
([Team outlines and logos](#team-outlines-and-logos)). At most 32 players and 16 NPCs
are tagged and nothing is allocated per frame. Not tested in game yet; the estimates
are unit-tested against synthetic snapshots only. The holo frame (28/10/2026) is
unit-tested for its layout and checked in the off-screen nameplate snapshot
(`menu_snapshot::nameplate_snapshot`, with the game data), not yet in a match.

## World pings

A world ping marks a point in the map for other SJK players on the same server, as
modern games ping (Sol's request, 13/10/2026): a gold ring at the point, seen through
walls, with the pinger's name above it and the distance under it, a short sound, and
gone after 7 seconds (it fades over the last 1.5). Players on other clients and game
servers see nothing: pings travel through the SJK hub ([hub-chat.md](hub-chat.md#pings)).
Code: [world_ping.rs](../crates/sjk-viewer/src/world_ping.rs) (rules, limits, drawing)
and [world_ping_frame.rs](../crates/sjk-viewer/src/world_ping_frame.rs) (the press, what
arrives, the HUD pass).

- **Pinging:** `+ping` (Settings > Key bindings > Interaction > World ping; the middle
  mouse button, `MOUSE3`, in a new profile and in one where it is free, as retail binds it
  to the saber style, which SJK binds to L) pings on the press; `worldping` does the same
  from the console or a [quick wheel](#quick-wheels) choice (`ping` is the engine's server
  query). The point is where the crosshair's ray meets the map: the dynamic crosshair's
  (`cg_dynamicCrosshair`, the player's aim from the muzzle, which a third-person camera
  with its own pitch or tilt does not look along), else a ray from the camera through the
  screen's centre (static crosshair, free camera), to 16,384 units against solid brushes (not clips; the sky and nothing in reach say
  "Nothing to ping there"). It shows at once for the pinger, a paler gold, and goes to
  the hub in whole units; in a local game nothing is sent.
- **Who sees it** ([`Scope`](../crates/sjk-viewer/src/world_ping.rs)): in a game without
  teams (FFA, Holocron, Jedi Master, Duel) every SJK player on the server; in a team game
  (Team FFA, Siege, CTF, CTY) the pinger's team only; in Power Duel the pinger's side
  (the lone duelist, or the pair). Spectators see the players' pings in a game without
  teams, never a team's, and spectators' pings reach only other spectators, so a
  spectator cannot guide a player. The sender names its scope from its own team; each
  reader shows a ping only when the game shows, in the ping's slot, the name the
  sender's hub claim was made under (as badges and emotes check) and the team that scope
  belongs to, and when its own team sees that scope. A ping sent before a team change and
  read after it is dropped.
- **Limits:** a player sends at most one ping a second and four in ten seconds ("Too many
  pings: wait a moment"), under the hub's five in ten. On arrival each sender is held to
  one every half second and five in ten seconds whatever the hub let through, at most two
  of a sender's pings show (a third replaces their oldest) and 24 in all. The sound plays
  at most every 0.3 seconds. When the hub refuses a ping (no identity, not claimed on the
  server yet), the chat says why in an `SJK ping:` line, once in 30 seconds for the same
  words.
- **Hiding and muting:** `cg_worldPings` (archived, default 1; Settings > HUD > World
  pings > Others' world pings) 0 hides other players' pings; one's own still show and
  are still sent. A player blocked on this PC ([hub-chat.md](hub-chat.md#blocking-a-player))
  shows no pings. `cg_worldPingSound` (archived, default 1; Settings > HUD, and in the
  SJK UI's Advanced Sound under Notifications) plays `sound/interface/update` from the
  game's own data, the player's own pings included.
- **On screen:** the ring, its centre dot and, for the first 0.8 seconds, a ring that
  grows out of it; a point off the screen or behind the camera sits on the screen's edge
  (44 pixels in at 1080p) toward it, smaller, with only its distance. The distance is from
  the camera, in metres of 39.37 units (a unit an inch). Pings hide with the HUD's
  information: in the menus, the console, the scoreboard, the intermission and with
  `cg_draw2D 0`. Another server, or leaving it, clears them.
- **Not built:** SJK has no minimap or radar to show them on; a ping cannot name a player
  or an item (only a point); the team rule is enforced by honest clients only, since the
  hub knows no teams ([hub-chat.md](hub-chat.md#pings)).

World shot: `world_shot::pings::duel6_world_pings` (1080p and 4K).

## Version label

The client draws its build label small at the top centre of every frame, in
menus and in play, so a screenshot says which build it shows:
`SJK <version> · <dd/mm/yyyy HH:MM> · <commit>`, the release version (a date
version such as `2026.1005.1`, or `dev` for a local build), the source commit's date and time on the 24-hour
clock, and its short hash ([version_overlay.rs](../crates/sjk-viewer/src/version_overlay.rs),
[build_info.rs](../crates/sjk-viewer/src/build_info.rs)). The top centre is free in
the stock and game-data HUDs (gauges at the bottom corners, the FPS counter and
timer at the top right, notify and vote lines at the top left). It is left out
while the console is open, which shows the version in its own corner.
`cg_drawVersion 0` (Settings > HUD > "Version and date") hides it. The log names
the same build at startup (`build: SJK ...`). How the version is decided is in
[SJK conventions](sjk.md#version).

## Changelog page

The main menu's Changelog entry (SJK > What's new in the SJK UI; SJK >
CHANGELOG on the classic page), the in-game SJK pop-up's Changelog and the
`changelog` console command show every SJK release from [CHANGELOG.md](../CHANGELOG.md), built into the client
([changelog.rs](../crates/sjk-viewer/src/changelog.rs), parsed by
[changelog_data.rs](../crates/sjk-viewer/src/changelog_data.rs)). Releases are
listed newest first on the left, "Unreleased" (the changes on `main` since the
last release) on top; the right pane shows the selected release's introduction
and its changes, wrapped to the pane, each followed by its credit in capitals
(the SJK UI's look draws what a change puts in backticks as a chip; a click opens
Settings on that setting or types it on the console's input line).
Up and Down (or a click) pick a release; Page Up, Page Down, the wheel over the
pane and its scrollbar scroll it; Escape closes. Like the `debug_panel` list the
page lives in the console and is drawn in its place, so it opens over the menus
and in a match, and closes the console with itself when it opened it. With
`ui_menuStyle classic` it takes the classic+ look of the command browser's
pop-up ([changelog_classic.rs](../crates/sjk-viewer/src/changelog_classic.rs)):
the in-game pop-up box and title band, a retail list box of releases, a detail
box with the release and its credits in gold, a Close button and the
description line, in the menus' retail font. How the file is kept is in
[SJK conventions](sjk.md#changelog). A release's `> Heading: sentence` recap lines are
not drawn here: they feed the [News panel](sjk-ui.md#news-panel), the pop-up
shown once after an update (`cl_whatsNewSeen` remembers the version; `whatsnew` previews
the newest recap).

## Updates

The client looks for a newer SJK release when it starts and from the Update page
([update.rs](../crates/sjk-viewer/src/update.rs) does the work,
[update_panel.rs](../crates/sjk-viewer/src/update_panel.rs) draws the page).

- `cl_autoUpdate` (default 1; Settings > Network > Check for updates)
  asks `api.github.com/repos/Sol-Vulpes/SJK/releases/latest` at start-up on
  a worker thread, and again every 30 minutes while SJK runs as long as nothing new
  was found (or the check failed), so a release made during a match reaches the
  players in it as its card without a restart (since 10/10/2026). A newer version shows on the main menus' version line
  ("update 2026.1010.1 available"). The check sends the request GitHub needs and
  nothing else; turn it off to send none.
- That line is clickable (since 19/10/2026, Slash's request;
  `update::notice` and `update::click_notice`). The SJK UI's main page shows "SJK
  2026.1010.1 is ready" with an Install button under the version, bottom right; the
  classic menu's version line reads "update 2026.1010.1 available: click to install".
  A click starts the download at once, as Install on the Update page does; the line
  then shows "Downloading the update... 42%" (the SJK UI with a progress bar) and
  "Installing the update...", and once installed offers Restart, which quits so the
  new version starts. A folder that cannot update itself offers Download, which opens
  the release page; an install that fails says so with Details, which opens the
  Update page with the reason. A failed check (GitHub unreachable) shows nothing.
- When the check finds a newer release, a card says so once a session at the top of
  the screen, over the menus or a match, the achievements' card
  ([identity.md](identity.md#achievements); since 10/10/2026, Sol's request): "Update
  available", "SJK 2026.1011.1", "Click to install" and "Downloads now; it starts
  when SJK restarts" (or, for a folder that cannot update itself, "Click for its page"
  and "This folder cannot update itself: see its page"), with SJK's emblem in the SJK
  UI's holo blue and no sound. While the pointer is free (over the menus, not in a
  match) a click on the card starts the download, as Install does, and opens the Update
  page with its progress; for that folder it opens the release page (since 19/10/2026,
  Slash's request; `GpuState::update_card_pointer`). It waits while the window is away, and the Update page opening takes it back
  (a release found while the page shows gets no card). `debug_update [version] [manual]`
  shows the card for a made-up release (2026.1231.1 without one; `manual` for the
  folder that cannot update itself) and checks and installs nothing.
- Main menu > Update (classic: SJK > UPDATE) or the
  `update` command opens the page (with the SJK UI's menus, a pop-up card over
  the map: [sjk-ui.md](sjk-ui.md#sjks-pages)). It shows the state and offers Install (Enter),
  Check again (C) and Release notes (N); Escape closes. Opening it with nothing
  checked yet checks at once.
- Install downloads `SJK-<version>-<platform>.zip`, refuses it unless its SHA-256
  is the one in the release's `SJK-<version>-SHA256SUMS-<platform>.txt`, writes the
  ZIP's files beside the running program as `<name>.new` and renames each running
  file to `<name>.old` before the new one takes its name (a Windows program can be
  renamed while it runs, not overwritten). A failed step puts the old files back.
  Only plain file names are taken from the ZIP, and the Mac's `SJK.app` bundle,
  which is written and swapped whole (the folder beside the bundle is the install
  folder; a Mac client from before the bundle cannot install a bundled release and
  says so). The `.old` files (and `SJK.app.old`) are deleted at
  the next start. Settings in `GameData/SJK/` are not touched.
- The new version starts when the client exits, after it saved its settings:
  "Restart now" on the page quits, or the player keeps playing and restarts later.
  The program's path is read once at start-up: on Linux `current_exe` follows the
  running file, so after its rename it named `<name>.old` and the restart started
  the old version again (fixed 16/10/2026).
- Nothing is installed without Install. When the program's folder is not writable
  (a Program Files install) the page offers the release page instead.
- A local build says `dev`, which has no release number to compare, so it reports
  that and never offers an update. `cl_updateAs 2026.1001.1` makes it check as if
  it were that release, to try the page without a release build.

## Credits page

The main menu's Credits entry (SJK > Credits in the SJK UI; SJK > CREDITS on the classic page), the in-game SJK pop-up's Credits and the
`credits` console command show who makes SJK, from
[credits.txt](../crates/sjk-viewer/assets/credits.txt), built into the client
([credits.rs](../crates/sjk-viewer/src/credits.rs), parsed by
[credits_data.rs](../crates/sjk-viewer/src/credits_data.rs)), and everyone's
work from [credits_history.txt](../crates/sjk-viewer/assets/credits_history.txt),
which `scripts/credits_history.py` writes from git and GitHub. Each section of
the file (Sol JK, Contributors, Art, Made with, Origins) is a heading with its people.

- Everyone with a history (Sol, Bishop, Creyon, Lumaya) has a panel across the
  column: the name large (the first section's larger still), the role, chips
  counting their changes, pull requests and commits (Sol's also the contributors'
  pull requests reviewed and merged, REVIEWED & MERGED), their links, and two folds.
  HIGHLIGHTS holds the card's own lines; ALL WORK (ALL PULL REQUESTS when all of
  it is pull requests) lists every feature and pull request newest first under
  the day it was merged into main, each row with its pull request's number
  (JKR #48, SJK #23) and how many commits it has. A row of several commits
  unfolds into them: hash, subject, and who made it when that is not the owner.
- Every fold is closed when the page opens. A fold's rows fade in one after the
  other as it opens; EXPAND ALL in the footer opens every fold at once and then
  reads COLLAPSE ALL.
- Clicking a pull request's number opens it on GitHub, clicking a commit (or a
  one-commit row) opens the commit on SJK's GitHub, a GitHub handle opens that
  profile and a link its address (`update::open_page`, https only). They brighten
  under the pointer and take clicks only while the page shows them whole, up to
  84 targets a frame, under the menu canvas's limit.
- The rest (Claude, the fonts, the reference clients) sit on cards in rows of up
  to three with their lines shown.
- A card's medals (credits.txt `medal:` lines, the hub's ids:
  [identity.md](identity.md#medals)) show on it: a chip with the medallion and the
  name after the counts (under the role on a card without a history). Creyon and
  Lumaya wear Contributor.

The page is animated from the clock alone. Golden god rays turn slowly down from
above the top of the screen, two sets against each other so the shafts shimmer
where they cross, sparks rise through them, and SJK's emblem breathes in front of
two sunbursts turning opposite ways, above a title a glint crosses: the look of
SJK's site (`site/assets/effects.js`). The rays are two light pictures the emblem
module draws on its worker at start (`EmblemLayer::Sunburst` and `Godrays` in
[emblem.rs](../crates/sjk-viewer/src/menu/emblem.rs)), added as light by the UI
renderer's additive pipeline and turned by their texture coordinates
(`emblem::rays`). Panels rise into place as the page opens and their edges glow
in turn. With the classic menus it takes retail's gold and blue and the menus'
retail font; with the SJK UI it has that UI's look, the emblem as a sun on the
left with the sections under it and the people in a column on the right
([sjk-ui.md](sjk-ui.md#sjks-pages)). Arrow keys, Page Up and Page Down, Space,
the wheel and the scrollbar at the right edge scroll it, gliding to the place
asked; Tab (or `]`) scrolls to the next section and Shift+Tab (or `[`) to the one
before, round to the first past the last; E opens every fold or closes them;
Escape, Enter or CLOSE closes it. Like the changelog it lives in the console, so it opens over
the menus and in a match. How the files are kept is in
[SJK conventions](sjk.md#credits).

## Settings usage

Since 12/10/2026 (Sol's request) the client tells the SJK hub, anonymously, which
settings players change and what they search for in Settings, so the team can see
which settings matter and what players look for
([settings_usage.rs](../crates/sjk-viewer/src/settings_usage.rs), recorded by
[settings/usage.rs](../crates/sjk-viewer/src/settings/usage.rs), the wire format in
[sjk-identity's usage.rs](../crates/sjk-identity/src/usage.rs)).

- `cl_shareUsage` (archived, default 1; Settings > Network > Share settings usage,
  and Simple's General) turns it on. At 0 nothing is sent and nothing is kept.
- A change records the setting's console name (or its row's stand-in, such as
  `renderer`), the category it was shown under (a search's group for a result),
  the mode on show (simple or advanced; a classic+ panel counts as advanced), its
  value before and after as the row shows them (On, Off, a choice's name, a
  number) and the search showing, if one was. A text setting (an address, the
  GIPHY key) sends `(text)`, never its value. Only the row the player acted on is
  recorded, though a quality level changes many rows at once. A change of the same
  setting within 4 s updates the last event (a slider dragged, a key held), and one
  brought back where it started is dropped.
- A search is recorded once it settles: the longest text typed in one go (a
  shorter correction is not recorded again) and how many settings it finds, when
  the field is left, a result is changed or Settings closes.
- What was recorded goes to the hub (`POST /v1/usage/settings`, unsigned) when
  Settings closes, also from Key bindings, on a thread of its own: batches of up to
  100 events with the build and a random id drawn once per run, so one run's events
  read together without linking runs. No identity key, signature, name or server is
  sent. Up to 400 events wait (the oldest go first); what the hub cannot take is
  dropped, not retried, as these are statistics. `cl_hubUrl` empty sends nothing.

The hub's side (the endpoint, its storage without the sender's address, and the
operator's summary of the most changed settings and searches) is built on a branch
of the hub and not deployed yet, so nothing reaches it until it is; nothing of this
was tested in game.

## Discord status

Since 18/10/2026 (Sol's request) the client shows what the player is doing on their
Discord profile, as Discord's Rich Presence
([discord_presence.rs](../crates/sjk-viewer/src/discord_presence.rs), the pipe in
[discord_ipc.rs](../crates/sjk-viewer/src/discord_ipc.rs)). It talks only to the
Discord desktop app on the same PC, over its local pipe (`\.\pipe\discord-ipc-N`
on Windows, a `discord-ipc-N` socket in the runtime or temporary folder elsewhere,
the Flatpak and Snap folders included), to every Discord app that answers: Discord,
PTB and Canary can run side by side signed in to different accounts, and each shows
the status on its own. Without Discord running nothing happens and the client looks
again every 20 seconds; the log says which apps it reached.

- `cl_discordPresence` (archived, default 1; Settings > Network > Discord status,
  and Simple's General): 0 off (clears the status), 1 on, 2 on without the server's
  name. A server with a password (`g_needpass`) never shows its name, and the
  server's address is never sent.
- Shown, twice a second from the identity frame and sent only when it changes (at
  most once every 5 seconds, under Discord's limit of 5 per 20 seconds): in the menus,
  the screen on show (Browsing servers, Settings, Character creator, a Profile or
  Collection tab); while joining, the map loading; in game, the server's name (colour
  codes stripped), map, mode, team with the team scores (or Spectating), and the
  players on the server out of its slots; a local game, a demo or a map explored
  alone. The elapsed timer counts from entering the menus, a server, a demo or a map.
- The picture is the SJK logo on the project site, and a "Get SJK" button opens the
  site. There is no join button: SJK has no link scheme (`sjk://`) yet.
- The game name Discord shows is the "Sol JK" application's in Discord's developer
  portal, whose id is `APPLICATION_ID` in `discord_presence.rs`; while it is empty the
  feature stays off. `SJK_DISCORD_APP=<id> cargo test -p sjk-viewer --bins
  discord_app_takes_an_activity -- --ignored` checks the pipe against a running
  Discord.

## Identity

SJK keeps an identity key and can mark other SJK players on a server with SJK's
emblem on the scoreboard and in a card beside them; the design, limits and privacy
are in [identity.md](identity.md).

- `cl_identity` (default 1; Settings > Network > SJK identity) makes the key
  (`identity.key` beside `config.cfg`) the first time it is on and lets the client
  talk to the hub. Off sends nothing and makes no key.
- `cl_hubUrl` (default `https://sjk.dfox.app`; Settings > Network > SJK hub) is the
  hub's `https://` address. Empty means no hub, so nothing is sent.
- The Identity page is where a player sets all of this up without a command:
  Settings > Network > SJK identity key, main menu > SJK > IDENTITY (classic menus),
  the in-game SJK menu, or the `identity` command
  ([identity_panel.rs](../crates/sjk-viewer/src/identity_panel.rs)). It shows the file
  to back up (hidden behind bullets until Show file, each time the page opens, so it can
  be opened on a stream; the key id is always bullets, as no key id ever shows in the
  game, [identity.md](identity.md#settings-and-commands)), whether the hub's operator vouches for the
  player, the hub's status and the players it knows on the current server, and has
  - a switch for `cl_identity` ("Share my identity with the SJK hub", ON or OFF);
  - an optional bio field (500 characters at most), saved to the hub with Save or Enter.
    There is no name field: the hub takes the name the player plays under (`name`), and
    the page shows it with up to three earlier names. A new player has nothing to do;
  - "Copy my key id" (for the operator to verify the player);
  - "Show file" (then "Hide file"): shows where the key file is;
  - "Use the official hub", shown only while `cl_hubUrl` is another address (a hub tried
    on this PC, say): it sets `cl_hubUrl` back to `https://sjk.dfox.app`. A saved
    `cl_hubUrl` wins over the default, so an old address stays until it is changed.

  With the classic menus (`ui_menuStyle classic`) the page takes the classic+ look
  ([identity_panel_classic.rs](../crates/sjk-viewer/src/identity_panel_classic.rs)): the
  retail pop-up and title band, option rows with the highlight band, text fields in list
  boxes, gold buttons that glow, and a description line under the box.

  Tab, Shift+Tab and the arrow keys move between the controls, Enter or Space works the
  switch and the buttons, letters type into the focused field (Ctrl+V pastes) and Escape
  closes; the pointer works too. The bio field follows the hub's copy until the player
  types in it. The commands remain: `identity bio <text>` changes
  the bio (`identity name` says the name is the one played under), `identity key` copies
  the key id to the clipboard (never printing it) and says where the file is,
  `identity who [slot]` lists known players (with a slot, that player's bio).
- The Profile page (in the SJK UI the Profile screen's SJK Profile tab, opened by its
  main page's Profile and profile card and the in-game menu's Profile; the classic
  menu's SJK page and in-game SJK menu; or the `profile` command) shows the player's
  profile as others read it on the hub: name, verified, member since, other names,
  medals (on its own only), the bio (written there, up to 6 lines under the hub's
  rules), their record from the achievement counts and how many achievements and
  shaders they hold. It
  has the SJK UI's look in every menu style; See the board and See the collection
  open the Collection (`collection`, `achievements`, `unlockables`;
  [sjk-ui.md](sjk-ui.md#collection)): medals, achievements, saber shaders
  ([unlockables.md](unlockables.md#shaders-tab)), toys, nameplates and holocrons
  (`holocrons`, [holocrons.md](holocrons.md#the-holocrons-tab)), a screen of its own in
  the SJK UI ([identity.md](identity.md#profile)); See holocrons opens its Holocrons.
  The SJK UI's main page and in-game menu show a profile card bottom left (picture,
  name, verified, medals and achievements unlocked) that opens it with a click
  ([sjk-ui.md](sjk-ui.md#profile-card)).
- A picture: choose a PNG, JPEG or TGA with Browse... on the Profile page's picture
  panel (the system's file dialog), drop one on the window or type `sjkavatar <file>`
  (a path's words may be left unquoted); it is checked (64 to 8192 pixels a side, at
  most four times as long as wide, something visible, a whole file of one of those
  formats whatever its name; every refusal says why), shown cropped to a square and
  made 128 pixels across, and Use this picture sends it to the hub, where everyone
  sees it. `sjkavatar clear` (or Remove picture there, pressed twice) takes it
  down and `sjkavatar` alone opens the picture panel. Other players' pictures are
  downloaded when first shown and kept in `avatars/` beside `identity.key` (at most
  256 pictures, 8 MB) ([identity.md](identity.md#pictures)).
- Blade skins: `cg_saberSkin` and `saberskin` choose the one worn, shown only while the
  hub profile owns it ([Blade skins](#blade-skins)).
- Achievements are counted in matches on servers and kept in `achievements.json`
  beside `identity.key`, sent to the hub with the identity on; an unlock says so in the
  console and with a card at the top of the screen, the same card that announces a new
  medal or holocron during a match ([identity.md](identity.md#achievements)).
  Settings > Sound > Unlock sound (`cg_achievementSound`) turns its chime off.
- The scoreboard (every style) draws SJK's emblem at the end of the name of a
  player the hub knows (the SJK UI's right after the name), in gold when the
  hub's operator vouches for them, then up to three ribbon bars for the medals the
  SJK team gave them ([identity.md](identity.md#medals)); fewer bars show where the
  name would run short of room. It trusts
  a claim only when the claimed name matches the name the game shows in that slot.
- The Identity page lists the player's own medals: in the SJK UI a column right of
  its own, each with its
  picture, name, description, the date it was given (`dd/mm/yyyy`) and the team's note;
  the classic+ box names them in its status with their medallions. With none, a line
  says how medals come. A medal new to the client shows once in a pop-up on the main
  menu or when the game menu opens (during a match a card at the top of the screen says
  it came first), arriving in a short ceremony with the game's
  Jedi Master fanfare (`music/goodsmall.mp3`, silent without game data); Enter, Space,
  Right, Escape or a click finish the entrance, then take Next or Close. It has the
  SJK UI's look with its menus and the classic+ look with the classic ones
  ([identity.md](identity.md#medals), [sjk-ui.md](sjk-ui.md#new-medal)).
- `debug_medal <id> [x<count>] [r<rank>] [note]` (`debug_medal all [note]` for every
  medal, `debug_medal <id> ranks` for one at ranks I to V one after another, alone it
  lists the ids: `early_tester`, `early_contributor`, `bug_hunter`, `supporter`; a count
  is only for a repeatable medal, and none is since 21/10/2026; a rank above 1 arrives as
  a promotion, [identity.md](identity.md#medal-ranks)) shows made-up medals as if the SJK team had just given them, through
  the same queue, card (in a match), pop-up, ceremony and sound, to try them without the
  hub. It sends nothing and never writes `medals_seen.txt`; the console
  closes so the pop-up shows at once on the main menu (in a match, open the game
  menu).
- `debug_holocron <tier|all> [x<count>]` (alone it lists the tiers `uncommon`, `rare`,
  `legendary` and `mythical` with their odds) shows made-up holocron drops as if the hub
  had just dropped them, through the same queue, card (in a match), pop-up, ceremony and
  sound, and puts the first one's chat line in the game's feed, to try them without the
  hub. It
  sends nothing and never writes `holocrons_seen.txt`
  ([holocrons.md](holocrons.md#debug_holocron)). A holocron the hub drops while you play
  shows the same way, once, and appears as a chat line in its tier's colour
  ([holocrons.md](holocrons.md)).
- SJK chat: one conversation for every SJK player, through the hub, in games and in
  the menus ([hub-chat.md](hub-chat.md)). In a game, I (`messagemode5`) opens the
  composer on the SJK channel (Tab cycles All, Team and SJK); hub messages show in the
  chat feed after SJK's emblem, each one flowing line with its text in the SJK chat's own gold
  and the verified tick after a verified sender's name
  ([hub-chat.md](hub-chat.md#how-a-line-looks)). The SJK UI's main page docks it under Recent servers;
  `sjkchat`, the dock's Open chat and the in-game SJK menu open its page. Resting the
  pointer on a name in the chat (composer open), on the dock or on the page shows the
  player's sender card with Block: a blocked player's chat is hidden and, on the server,
  they are drawn as Kyle with the default saber and nothing they cause is heard, on
  this PC only ([hub-chat.md](hub-chat.md#blocking-a-player)). `cl_sjkChat 0` (Settings > Network > SJK
  chat) hides it and stops the reading. `sjkemote <id>` sends an emote the SJK players
  on the server see (`sjkemote` alone lists the installed ones; none ship yet).
- Back up `identity.key`: losing it loses the identity.

### Player card

Look at a player, keeping the view steady, and a card appears beside their hips
([player_card.rs](../crates/sjk-viewer/src/hud/player_card.rs)): a leader runs from a
dot at the player's hips (10 units above the origin, near the humanoid skeleton's
pelvis; a crouch lowers it by half the box's drop) to the card, centred on it. While the
card is up a ring in the card's accent colour (red or blue by team, else SJK's light
blue), fading with it, runs around the outside of the player's body and only the
outside: no line where an arm crosses the chest
([card_outline.rs](../crates/sjk-viewer/src/card_outline.rs)). After the scene the body
is drawn again, with its usual vertex path, into a one-channel mask against the scene's
depth, so only the part in view counts and nothing shows through walls; the 2D layer
then draws every pixel outside the mask within 2 pixels of a 1080-line screen (scaled
with the height, 4 at 2160; it was 4 and 8 until Sol halved it on 13/10/2026) in a
scissor around the player, after anti-aliasing so the colour is the card's
(11/10/2026). The team outlines share the ring
([Team outlines and logos](#team-outlines-and-logos)); on the card's player the
card's colour wins. Hats, capes and sabers are not outlined. The ignored
world shot `card_outline_on_a_demo`
([world_shot_card_outline.rs](../crates/sjk-viewer/src/world_shot_card_outline.rs))
plays the demo `SJK_TEST_DEMO` names off screen, pins the card on another player and
renders the ring from an orbit around them and from behind the recorder. It shows what
the server already publishes to every client (name with its colour codes, model and its
head icon, saber hilts with their blade colours, the hat and cape worn, bot skill; not
the duel wins and losses), the saber shader they wear (a live swatch, as the Collection
draws it, and its name; only when it draws on their blade on this PC: worn by their
look, its pack loaded; a second saber's or a staff's other half's own shader, when it
is not the first's, on a row of its own named "(2nd blade)" or "(other half)") and, when the hub knows the player, their profile picture in
place of the head icon (the icon then sits small on its lower left corner), SJK's
emblem, their hub name, a gold VERIFIED, a teal SJK TEAM pill for staff, a gem of the
rarest holocron tier they hold with its name and the medallions of the medals the SJK
team gave them ([identity.md](identity.md#medals)). A pinned card adds what takes
reading: the medals' names in small print (a repeatable one with its count),
"Achievements N / 21" with the three rarest unlocked as small medallions (the highest
step of their ladder first, the latest first among equals), the first line of their bio
and "In SJK since dd/mm/yyyy". The profile is asked of the hub when the card is built
for a player, at most every ten minutes a player and never per frame; until it comes,
and from a hub that does not send a field, that line is left out (10/10/2026). It adds
nothing a glance at the scoreboard would not: no health, Force or position.

- `cg_playerCard` (default 1; Settings > HUD+ > Player card) says what opens it: `0`
  (**Off**) nothing, neither a look nor `inspect`; `1` (**On look and key**) a steady
  look or `inspect`; `2` (**Key only**, Blep's idea, 29/10/2026) `inspect` alone, so
  looking at a player never brings their card up. The setting was on/off until then:
  configs saved it as 0 or 1, which keep their meaning (`player_card::Opens`, tests
  `the_setting_reads_three_ways_and_old_configs_keep_their_meaning` and
  `each_mode_opens_the_card_its_own_way`).
- `cg_playerCardDelay` (default 1.5; Settings > HUD+ > Card delay; only with
  `cg_playerCard 1`, as its help says) is the seconds
  the crosshair must stay on the player before the card fades in. Turning the view
  more than 6 degrees, or losing the player for over 0.3 seconds, starts the wait
  again and the card fades out.
- `cg_playerCardOutline` (default 1; Settings > HUD+ > Card outline) draws the outline.
- On another SJK player's card, under what they share: "Sees your model", or in amber
  "Can't see your model", from the model lists SJK clients send the hub
  (`cl_shareModels`, [identity.md](identity.md#installed-models)); no line when the hub
  has no list of theirs or you wear a retail model.
- `inspect` (Settings > Key bindings > Interaction > Inspect player; bound to X by default,
  or `bind <key> inspect`) pins the card to the player under the crosshair at once, with
  no wait, with `cg_playerCard` 1 or 2. It stays when you look away or the player moves, follows them (kept inside the
  screen), and a second press or Escape hides it (Escape then does not open the game
  menu); it also drops when the player leaves or the map changes. After hiding it, the
  card does not return until the crosshair leaves that player. Behind you, the card is not drawn but stays pinned.
- **In sight only** (17/10/2026, Sol's request): no card, by looking or by `inspect`,
  for a player farther than 1200 units from the camera or with a wall between the
  camera and both their head and their hips (the nameplates' trace, solid and terrain
  brushes). A pinned card fades out while its player is out of sight that way and
  comes back if they return within 3 seconds; after that the pin drops
  (`player_card::SIGHT_RANGE`, `LOST_MS`).
- World notes: `worldnote` (Settings > Key bindings > Interaction > World note; bound
  to O by default, or `bind <key> worldnote`; until 10/10/2026 `inspect` did it when no
  player was under the crosshair) selects the world surface under the crosshair, or the mover whose bounds the view ray
  meets first, and names it in a centre print (shader, BSP surface, lightmap or vertex
  lighting, distance). When a remap (the map's, the server's or a local `remapShader`)
  draws another shader over the surface, the print, `notes.txt` and `notes.jsonl`
  (`surface.remapped_to`) name it too (`<shader> remapped to <target>`): on a server
  what the player saw is often the remap's target (08/10/2026). Since 13/10/2026 the hub
  gets both (`shader` and `remapped`), and every note keeps the remaps active when it
  was taken, as `listRemaps` lists them without those a later remap hides
  (`remaps`: source `map`, `server` or `local`, `from`, `to`). A model drawn from the
  map's entities comes first when the view ray meets one of its triangles before the
  world (13/10/2026): any entity naming an MD3 (`misc_model_static` props, which the
  client draws from the map, and breakables or other server entities where the map
  places them), read once per map on a press and tested on its first frame, turned and
  scaled as the entity says. The print names its class, path and first shader (with
  its remap); the note keeps `model` (path, class, entity number, targetname, origin,
  angles, scale, skin key, and every surface's shader with `remapped_to`), names the
  model's entity in `entity` and leaves `surface` out. A GLM model or a server entity
  that moved is not picked. While selected, and while its note is written, the surface's
  triangle edges (a mover's bounds; a model's folds, or its turned bounds when it has
  more than 512 triangles) are traced over the view in flickering green dots
  (at most 600 a frame) over a green scanline shade of its triangles (horizontal strips
  with a brighter wave running down them, at most 800 a frame; cut where the surface
  passes behind the eye, so a floor underfoot keeps its visible part). Escape before the second press drops the selection (it does not
  open the game menu then). A second press within 20 seconds opens the text dialog
  ([text_dialog.rs](../crates/sjk-viewer/src/text_dialog.rs)): a panel in the middle of
  the screen with the subject, a text box of wrapped lines (ten in the SJK UI's card,
  scrolling to keep the caret in view; up to 4000 characters since 13/10/2026, 500
  before), a character count, Send and Cancel; Tab moves between them. A note about a
  model shows the model with Clear model (Tab reaches it last), which notes what is
  behind the model instead; a click on the scene round the dialog selects what is under
  the pointer (a model too) for the note, the text kept. With the
  classic menus it takes the classic+ look of the in-game pop-ups, eight lines in a
  retail list box with Clear left of Send ([identity.md](identity.md#bug-reports)); with
  the SJK UI it is its
  pop-up card over the scene, darkened by half so the selection still shows round it
  ([sjk-ui.md](sjk-ui.md#report-a-bug-and-its-dialogs)). The selection's outline also
  shows over a map explored without a server. As it is typed the text
  keeps the bug reports' alphabet (letters, digits, spaces and
  `. , ! ? ' - : ( ) _ / \ + = " ; [ ] < > #`), and
  Send needs 3 characters with 2 letters. Enter or
  Send appends the note to `notes.jsonl` (one JSON object: map, UTC time,
  `setviewpos`, camera pose, hit point and normal, draw surface, collision flags,
  nearest map entity, and since 13/10/2026 `model` and `remaps`, screenshot path; older
  lines lack the new keys) and a readable line to `notes.txt` in the config
  directory, and takes a silent JPEG of the same view (`note_<unix seconds>.jpg` in the
  screenshots folder; JPEG screenshots drop the alpha channel, which the encoder
  refused before, so `screenshotjpeg` works too). With `cl_identity` on, the note and a
  smaller copy of the screenshot also go to the SJK hub for the SJK team, and a centre
  print says whether it took them ([identity.md](identity.md#world-notes)). Escape or
  Cancel drops it. [world_notes.rs](../crates/sjk-viewer/src/world_notes.rs).
- The crosshair scan that names players under the crosshair finds the target. The card
  is anchored in the world beside the top of the player's box (the box height the server
  sends, as the nameplates use), a body width clear of them in screen pixels, so it
  neither floats above nor overlaps the player at any range. It goes to the player's left
  near the right edge of the screen and hides under the scoreboard, menus, the console
  and intermission.
- The head icon is the scoreboard's (`models/players/<model>/icon_<skin>`, one atlas cell
  per client slot), resolved when a card is shown even with the scoreboard closed. Hats and
  capes are models without pictures, so they are named in text (`c1`, `c2` after the colour
  digits).
- A hub bio is not shown yet.

### Team outlines and logos

In team games (Team FFA, Siege, Capture the Flag, Capture the Ysalamiri) every enemy
gets a red ring around the outside of their body and every teammate a green one, the
player card's ring at its width (Sol's request, 13/10/2026;
[team_outline.rs](../crates/sjk-viewer/src/hud/team_outline.rs),
[card_outline.rs](../crates/sjk-viewer/src/card_outline.rs)). Like the card's, a ring
follows only the part of a body that is in view: the bodies are drawn into the mask
against the scene's depth, so nobody is outlined through a wall and the rings show
nothing the screen does not. Each body writes its group (card, enemy, teammate) into
the one-channel mask through the blend constant, and the ring takes the colour of the
nearest body; the ring is drawn only inside a box around each outlined player, boxes
that overlap merged so no pixel is drawn twice, with about one mask lookup per pixel of
its circumference (8 to 16 a step). Nobody is outlined in free-for-all, duel or power
duel, while spectating freely (a followed player's team is the viewer's), nor a dead,
cloaked or mind-tricking player, nor anyone off screen. Hats, capes and sabers are not
outlined. The rings hide with the overhead labels (menus, the scoreboard,
intermission, `cg_drawHud 0`, `cg_draw2D 0`).

- `cg_teamOutline` (default 1; Settings > HUD+ > Team outlines, and Simple > HUD)
  turns them on.

Over each teammate's head stands their team's logo, as JoF EJK floats it
(`CG_PlayerFloatSprite` in `CG_Player`): the Imperial logo (`sprites/team_red`) over a
red teammate, the Rebel one (`sprites/team_blue`) over a blue one, every player's while
spectating freely, the duel partner's icon (`gfx/mp/pduel_icon_double`) in power duel
and the master's foes in Jedi Master, as stock does; Siege's per-map team icons are not
read, so Siege shows the red and blue logos, stock's fallback
([team_logo.rs](../crates/sjk-viewer/src/hud/team_logo.rs)). EJK's sprite is 20 units
across in the world and hidden by the depth; SJK's logo is a picture on the screen as
large as a 24-unit sprite, held between 20 and 40 pixels of a 1080-line screen so it
stays readable far away and never outgrows a name up close, its foot just over the head
(over the talk balloon or connection icon when one floats there), and a trace from the
eye to that point hides it behind walls. The nameplate, or the plain overhead name,
stands above it. It hides with the nameplates (menus, the scoreboard, intermission,
`cg_drawHud 0`).

- `cg_drawFriend` (default 1; Settings > HUD+ > Team logos) turns
  them on. It was a small coloured square until 13/10/2026.

The team overlay at the top right (`cg_drawTeamOverlay`, teammates' names, health and
location) was removed the same day at Sol's request: the outlines and logos show who
is who. The setting and its position settings (`cg_drawTeamOverlayX`, `Y`, `Scale`,
`Weapons`) are no longer registered (a saved line becomes an unused cvar), a layout's
`team_rows` widget draws nothing, and the client asks the server for the team data
only for the nameplates' bars.

## Server dialogue and quests

The JoF dialogue window is an optional JoF client-mod feature. It requires JoF
tools to be loaded on a recognized JoF server. See
[JoF dialogue and node audio](mods.md#dialogue-window-and-node-audio) for controls,
settings, response validation and the server-owned quest boundary.

## Force wheel

With the `game` and `classic` HUD styles, `forcenext`/`forceprev` show JoF
EternalJK's Force wheel: retail's Force selection bar (`CG_DrawForceSelect`),
the selected power large in the middle above the HUD with up to three
neighbours on each side and its name under it, for 1.4 seconds as in retail.
It scales with `cg_hudScale`; the `radial` style keeps its list of names. On a
JoF JA+ server that grants them, the wheel also holds Stasis and Repulse (after
Sense) and Dash (before Speed), from spare `forcePowersKnown` bits, as in JoF EJK
([force_wheel.rs](../crates/sjk-client/src/force_wheel.rs)). They are never sent
as the selected power: with one selected, `+useforce` engages Stasis (the
usercmd button the server reads) or sends `force_repulse` or `force_dash` once
per press, and the selection stays until the server takes the ability away. The
binds `force_dash`, `+force_stasis` and `force_repulse` (Settings > Key
bindings > Force powers) work without the wheel; `+force_stasis` does nothing
where Stasis is not granted. On JA+, merc mode shows Lightning as the
flamethrower until the player is seen using real lightning. Repulse, Dash and
the flamethrower use JoF EJK's pictures from `EternalJK/jofclient-assets.pk3`;
Stasis has none and shows Jump's. Power names are retail's (Dark Rage, Sense).
SJK's own [Illuminate](#illuminate) is a toy, not a Force power: it is not on the wheel.
See [hud/force_wheel.rs](../crates/sjk-viewer/src/hud/force_wheel.rs).

### Destruction client controls

Destruction is a JoF client-mod feature, active only with `mod_jof` on a recognized
JoF server. See [JoF Destruction controls](mods.md#destruction-client-controls)
for the mod setting, grant gates, binding and wheel behavior.

### Server-confirmed Stasis and Repulse sounds

These confirmations belong to the JoF client mod, require its server opt-in and
are inactive on other servers. See [JoF EJK opt-in and confirmation sounds](mods.md#ejk-server-opt-in-and-firing-confirmations)
for settings, `jofejk` scope and the installed-asset requirement.

### JA+ flamethrower presentation

On a JA+ server (gamename `ja+`, `japlus` or `ja_plus`; abyssmod and UA Galaxy,
which share JA+'s dialect, are left out until their flag meanings are verified), firing the flamethrower shows its flame stream and
hit effects, uses the Lightning-hold upper-body pose, and plays its positional
flame sound until firing stops. Local predicted firing and remote players use
the existing snapshot flags; ordinary Lightning and Drain retain their effects.
The server owns damage, fuel and all weapon rules.

`cg_flameThrower` (archived; Settings > Game options in classic, Settings >
Advanced > Gameplay in SJK, **Flamethrower effects**) selects Auto (`-1`, default),
Off (`0`) or JA+ (`1`, force support on a server using the same flags). The
documented capability table is in
[japlus_flame.rs](../crates/sjk-viewer/src/japlus_flame.rs); it detects the server
independently of the optional JA+ command mod. The flame graphs and sound must
be present in installed game data, such as the JoF EJK client-assets pack.
SJK does not bundle them. The custom `mp/fthrw_bobafire` stream has priority;
if absent, SJK uses the retail `boba/fthrw` stream with its forward direction
reversed; both missing: stock Lightning. A missing sound stays silent. The flame
sound follows the same detection as the effects, refreshed when the server info
changes. Impact bursts follow presentation time between snapshots, including
during demo playback.

`forceselect <n>` (SJK's) selects wheel entry `n` at once, by its
`forcePowers_t` number (18 Stasis, 19 Repulse, 20 Dash, 21 Destruction), as if
`forcenext` had stopped on it: a real power becomes the selection the user
command sends, a pseudo-slot the one `+useforce` uses, and the bar shows it. An
entry not on the player's wheel, or spectating, selects nothing
([`Selection::select`](../crates/sjk-client/src/selection.rs)). The quick wheel's
[Force page](#force-page) runs it.

## Illuminate

Illuminate is a toy, SJK's own and free for every player (Sol's request, 10/10/2026:
"not a force power, but just a toy"): a holocron that floats by the player's left
shoulder, turning slowly and bobbing, with a warm point light (300 units) that
lights the way in dark maps. No game server knows of it: its lit state travels
through the SJK hub as part of the player's look ([unlockables.md](unlockables.md)),
so other SJK players on the same server see it by that player (below); players on
stock clients do not. It is not a Force power and is on no Force wheel, bar or
list: the `toy_illuminate` command lights it or puts it out, from a bind
(Settings > Key bindings > Interaction > Toy: Illuminate, unbound until a key is
picked), from the quick wheel's [Toys page](#toys-page), or from the switch on the
Collection's Toys tab ([SJK UI](sjk-ui.md#collection)). Every toy is a
`toy_<name>` command ([sjk.md](sjk.md#names)). Switching any toy says so on the centre of
the screen, its name and ON in green or OFF in red ("Illuminate ON"), and `toy_help` lists
every toy in the console, its name in yellow and what it does and how in white
([toy_switch_words.rs](../crates/sjk-viewer/src/toy_switch_words.rs)); a toy restored at
start, or taken off by the game, says nothing.

**Other players' toys, hidden on request** (Sol's request, 29/10/2026): each toy other
players are seen with has a second switch on its card in the Collection's Toys tab, and
a row in Settings > Game options > Toys, that hides theirs on this PC alone, for a player
who does not like them; on by default, archived ([toy_sight.rs](../crates/sjk-viewer/src/toy_sight.rs)).
The player's own toy still shows, and nothing changes for the toys' rules.

| Toy | Switch | Cvar | Hidden |
| --- | --- | --- | --- |
| Illuminate | See others' holocrons | `cg_toyIlluminateShow` | their holocron and its light |
| Cardboard box | See others' boxes | `cg_toyBoxShow` | their box (not lifted either) |
| Ragdoll | See others fall limp | `cg_toyRagdollShow` | their falls: their bodies as the game shows them |
| Drone | See others' drones | `cg_toyDroneShow` | their drones, names and sounds (who peeks at you is still told) |
| Boombox | See and hear others' boomboxes | `cg_toyBoomboxShow` | their stereo (on the shoulder or the floor) and its music |
| Monitor | See monitors | `cg_toyMonitorShow` | every monitor and its sounds, yours too ([Monitor](#monitor)) |
| Banana peel | See banana peels | `cg_toyBananaShow` | every peel and its sounds, yours too ([Banana peel](#banana-peel)) |
| Prop gun | (console only) | `cg_toyPropGunShow` | their guns, bolts and sounds ([Prop gun](#prop-gun)) |

Order 66, the Hax and Box hunt act only on players who have them on, so they have none.
The holocron, the box and the boombox are hidden as if those players had them off here
(`Looks::set_unseen`); the drone's and the ragdoll's relayed events are left undrawn.

It is always there, with no setting that enables it, and it is out at every start:
whether it is lit is the client's run only, not saved. The old Force wheel setting
`cg_illuminate` is gone; a profile that still holds it (0, off the wheel, or 1, on
it) has it dropped when it loads, so neither leaves the holocron lit or unusable.
The command was `force_illuminate` while it was a Force wheel entry; that name
still runs the toy, so a key bind or quick wheel choice saved under it works (the
bind is renamed to `toy_illuminate` when the profile loads, so the key-binding
editor lists it), but it is not listed in the console's completion or help.

The holocron appears and goes out over 0.3 seconds, trails a moving player by at
most 20 units and jumps with a teleport. It goes out while the player is dead,
spectating, following someone or at the intermission, and comes back at the player.
In first person only its light shows (the cube is drawn in mirrors, like the body).
Its light is added first each frame, so a full light list never drops it, but
`r_dynamiclight 0` puts it out with the others.

Other SJK players' holocrons show when their look says lit (with `cl_identity` on and
a hub; their claimed name must match the name the game shows in their slot, as for
badges). Each floats by that player's left shoulder, placed from their entity's
interpolated origin, eye height (crouching lowers it) and view yaw, with the same fade,
bob, turn and trailing as one's own (each slot's bob and turn a little out of step).
The cube always shows for another player, in first and third person, while the game
draws them; dead, hidden, cloaked or out of the snapshot, it goes out where it was.
All the cubes show, but only the four nearest the camera add their light, so the
frame's 32 lights stay for the weapons and sabers.

The cube, its two pictures (lit metal, and the emblem alone for the glowing
stage), its shader and its icon are bundled and mounted below all game
data, so a PK3 with the same paths replaces them (the picture is `gfx/sjk/toy_illuminate.png`). The same cube also has four
loot-box tiers (uncommon green, rare blue, legendary purple, mythical gold and
shiny): other pictures and shaders on the one model, with their icons and point-light
colours, bundled the same way. Sol generated the art; see
[assets/holocron](../crates/sjk-viewer/assets/holocron/README.md) and
[illuminate.rs](../crates/sjk-viewer/src/illuminate.rs).

## Cardboard box

The cardboard box is a toy, SJK's own and free for every player (Sol's idea,
16/10/2026): a shipping carton to hide in, Metal Gear style. `toy_box` puts it on or
takes it off, from a bind (Settings > Key bindings > Interaction > Toy: Cardboard box,
unbound until a key is picked), from the quick wheel's [Toys page](#toys-page) or from
the Collection's Toys tab ([SJK UI](sjk-ui.md#collection)). It is off at every start.
Switching it says "Cardboard box ON" or "OFF" ([Illuminate](#illuminate)); a box lifted by
another player goes without it. Putting it on plays its sound (one's own at once, in both ears; another's where they
stand when their look gains the box, but not for the boxes already in the looks in the
first 5 seconds on a server).

- **In the box** the player crouches: while it is on and the player is alive and in the
  match, every user command holds crouch (`upmove` -127, as a held `+movedown`, over a
  jump), so the server sees an ordinary crouching player and movement stays stock. Its
  music loops on the player, in both ears for one's own and placed on the player for
  others (on channel 14, its own, so it cuts no game sound).
- **Flying** the free camera or `fakenoclip` (hub report #152, 27/10/2026), the box
  holds no crouch: the commands' `upmove` steers the flight there, and a held crouch
  sank the camera. One's own carton stays on the body and keeps the body's facing, not
  the camera's; it is drawn for mirrors only while the camera is inside it (a flight
  starts in the body), and the player does not count as hiding.
- **Crouched in the box:** every SJK client draws a boxed player who stands as
  crouching (`BOTH_CROUCH1IDLE` over standing legs, `BOTH_STAND*` and the saber stances'
  `BOTH_SABER*_STANCE`, and over the torso when it
  stands too; `cardboard_box::crouched_in_box`, `actor_pose_steps.rs`; Dave's test of
  #152, 28/10/2026). The box asks the server to crouch, but `PmoveSingle` zeroes a
  talking player's moves, `upmove` included (EternalJK `bg_pmove.c:12441-12448` at
  a40e793; JoF EJK `bg_pmove.c:13108-13115` at 390f756), so the server stands up a
  boxed player with the chat or console open or the free camera's talk balloon, which
  stays so the player keeps its protection. Any other move (a lunge, a kick, a roll, a
  knockdown, which a boxed player can still make) plays as the server sends it. Who is
  boxed is noted before the actors are posed (`note_boxed_players`), so the pose
  follows the carton on the same frame. A crouch keeps the origin, so the pose sits
  where a real crouch would. One's own eye stays at crouch height (12) too while hidden
  in the box, so opening the menu or the chat no longer lifts the view
  (`keep_boxed_eye`, 28/10/2026). Rig models crouch too. Other clients see the server's
  player, as they never see the box.
- **Others see it** through the SJK hub: the box is a toy in the player's look
  (`toys: ["cardboard_box"]`, [unlockables.md](unlockables.md); `looks::TOYS`), so every
  SJK player on the server draws the carton over that player (their entity's
  interpolated origin, floor and view yaw) and hears the music, while the game draws
  them: dead, hidden, cloaked or out of the snapshot, neither shows. Players on stock
  clients see a crouching player. In first person one's own carton is drawn only in
  mirrors, like the body.
- **Spotted:** an SJK player who presses `+use` facing a boxed player (within 96
  units, 60 degrees either side of where they face; the nearest) lifts the box: the
  spotted sound plays where they stand and the box goes at once on the lifter's screen,
  and the hub relays a toy event (`POST /v1/toy`, `cardboard_box` `spotted`, the
  target's slot, [hub-chat.md](hub-chat.md#toy-events)). The boxed player's client
  takes the box off and plays the sound; every other SJK client hides it and plays the
  sound where the player stands. A lift seen here holds until that player's look drops
  the box, at most 15 seconds. The `+use` itself still reaches the server as always.
- **Taken off or lifted**, while its player is still drawn, the box tips over on its
  side (0.45 s, pivoting on its bottom edge, faster as it goes): away from the lifter
  when spotted, else the way the player faced. It lands with the drop sound, lies there
  5 seconds and shrinks away; at most 8 lie about at once. A box that goes with its
  player (dead, cloaked, out of sight) does not fall. One's own in first person shows
  only in mirrors until it has tipped 0.5 radians, out of the view inside it.
- **Dying in the box** plays its death sound: one's own in both ears, another's where
  they fell, on every SJK client that sees the death (the player's entity turning
  `EF_DEAD` while their look has the box). The box stays on until the respawn, so the
  others still know whose death it was, and is lost with the life.

The carton (an open-bottomed box of 46 x 46 x 44 units, drawn from both sides), its
picture, shader and icon are bundled and mounted below all game data, so a PK3 with
the same paths replaces them; they are drawn by
[cardboard_box_assets.py](../scripts/cardboard_box_assets.py), nothing taken from
another game ([assets/cardboard_box](../crates/sjk-viewer/assets/cardboard_box/README.md)).
The sounds are game paths that SJK does not ship, `sound/sjk/cardboard_box/music`,
`equip`, `spotted`, `drop` and `death` (`.wav`, else `.mp3`); where one is missing,
nothing plays. See
[cardboard_box.rs](../crates/sjk-viewer/src/cardboard_box.rs).

## Order 66

Order 66 is a meme toy, SJK's own and free for every player (Sol's idea, 17/10/2026):
every player who has it on dies at once, blown apart, when one of them executes the
order. `toy_order66` turns it on or off, from a bind (Settings > Key bindings >
Interaction > Toy: Order 66, unbound until a key is picked), from the quick wheel's
[Toys page](#toys-page) or from the Collection's Toys tab ([SJK UI](sjk-ui.md#collection)).
It stays as last switched, across deaths, maps, servers and restarts of the game, until
the player turns it off (since 22/10/2026: saved in the archived `cl_toyOrder66`,
[toy_saved.rs](../crates/sjk-viewer/src/toy_saved.rs)). `order66` (Interaction > Execute
Order 66, or the Toys page) gives the order; only a player whose own toy is on, alive in
the match on a server, can give it. Switching the toy says "Order 66 ON" or "OFF"
([Illuminate](#illuminate)).

- **Who takes part:** the players whose toy is on. It is a toy in their look
  (`toys: ["order66"]`, [unlockables.md](unlockables.md); `looks::TOYS`), so every SJK
  player on the server knows who is in. Players on stock clients, and SJK players with
  the toy off, are never touched.
- **The order** travels through the SJK hub as a [toy event](hub-chat.md#toy-events):
  toy `order66`, event `execute_<moment>`, the moment it falls due in the game server's
  time (milliseconds), 3 seconds after it was given. Every client on a server shares
  that clock through its snapshots, so each counts down to the same moment however long
  its way to the hub took. A client drops an order heard more than half a second after
  its moment, one more than 10 seconds ahead, one whose sender's claim does not match
  the name the game shows, and one less than 30 seconds after the last order it took on
  that server: one order counts every 30 seconds, whoever sends it. The sender's own
  client refuses sooner ones ("Order 66 again in N s"). Without the hub
  (`cl_identity 0`) the order reaches only the sender, and the console says so.
- **The countdown:** for a player in the order, the centre of the screen says
  "EXECUTE ORDER 66", who gave it and the seconds left (3, 2, 1), with the thermal
  detonator's warning beep each second; the order itself plays
  `sound/sjk/order66/order.wav`, a game path SJK does not ship (nothing plays where it is
  missing).
- **The moment:** each client sends the stock `kill` command for its own player, so the
  server sees ordinary suicides (a point lost in FFA, as with `kill`). It does not when
  its toy is off by then, when the player is spectating, following someone, at the
  intermission or already dead, in a duel (a private one, or a duel or power duel
  game), or carrying a flag ("Spared: you are in a duel", "you carry the flag"). These
  are game rules, not server cases. The order is called off when its sender's look no
  longer has the toy.
- **The blast:** a player whose toy is on and who dies from 0.1 seconds before the
  moment to 2 seconds after blows up on every SJK client that sees it (their entity
  turning `EF_DEAD`; the own player and the one followed from the player state). The
  thermal detonator's explosion (`thermal/explosion`, its sound, flash and shake) goes
  off where they stand, and the body flies apart: the head, both arms, both legs and
  the trunk each become a piece, cut as [dismemberment](#dismemberment-and-disintegration)
  cuts a limb (the limb's cap on the piece, the stump's on what is left). The pieces
  are thrown out from the body and up (the head highest), spinning, fall under gravity,
  bounce off the world's walls and floors a little slower each time, trail smoke while
  they fly, lie still on a floor, and after 6 seconds sink away. Nothing is left
  standing: the body is hidden, weapon and hat too, until the player is alive again,
  and the corpse the server leaves at the respawn copies it, so it stays hidden as well.
- **Confetti:** where dismemberment is off (`cg_dismember 0`) the body bursts into
  confetti instead of pieces: six colours of paper scraps thrown up and falling
  (`effects/sjk/order66_confetti.efx`).
- **Limits:** pieces are made only for players within 3000 units of the camera and up
  to 48 at once (eight players); past that a player still blows up, without pieces. A
  piece is a limb mesh of the player's model: while the order counts down, the client
  puts up to six per nearby player's model in the limb pool, one upload a frame, so the
  blast itself uploads nothing (it may upload up to six a frame when the pool falls
  short). A [rig model](rig-models.md) has no surfaces to cut: it gets the explosion and
  stays whole. The cuts take a surface by its name or, failing that, its name and
  `_off`, as some models name what they leave off by default (`jedi_hf`'s head variants
  sit under `head_off`, its caps are `..._cap_torso_off`); a server's cut keeps stock's
  names.

The icons (the toy's red "66" badge, the order's blast) and the confetti scraps are
drawn by [order66_assets.py](../scripts/order66_assets.py), nothing taken from another
game or a film; the effect and its shader are written by hand
([assets/order66](../crates/sjk-viewer/assets/order66/README.md)). They are bundled and
mounted below all game data. See [order66.rs](../crates/sjk-viewer/src/order66.rs) and
`GpuState::blow_apart` in [dismember.rs](../crates/sjk-viewer/src/dismember.rs).

## Hax

The Hax is a meme toy, SJK's own and free for every player (Sol's idea, 21/10/2026): a
player who has it on hacks another who has it on, then steers them for a few seconds.
`toy_hax` turns it on or off, from a bind (Settings > Key bindings > Interaction > Toy:
Hax, unbound until a key is picked), from the quick wheel's [Toys page](#toys-page) or
from the Collection's Toys tab ([SJK UI](sjk-ui.md#collection)). It stays as last
switched, across maps, servers and restarts, until the player turns it off (saved in
the archived `cl_toyHax`, as Order 66 in `cl_toyOrder66`). Switching it says "Hax ON" or "OFF"
([Illuminate](#illuminate)).

- **Who takes part:** the players whose toy is on. It is a toy in their look
  (`toys: ["hax"]`, [unlockables.md](unlockables.md); `looks::TOYS`), so the hacker's
  client knows who may be hacked. Players on stock clients, and SJK players with the toy
  off, are never touched.
- **Not on a player typing or away:** a hack does not begin on a player whose chat is
  open (their talk balloon) or whom the scoreboard marks away (still for 3 minutes, or an
  away tag in their name): "<name> is typing: no hacking them now". A hacked player's own
  client also refuses the seize while they are away, idle or alt-tabbed as the away emote
  counts it ("Hack failed: <name> is away"). Opening the chat once the hack is under way
  does not stop it; turning the toy off does.
- **The hack:** hold `+use` facing a holder of the toy within reach (the cardboard
  box's, 96 units, in front). The siege hack bar fills under the crosshair with the
  stock hacker icon (`gfx/mp/c_icon_tech`, [hack_bar.rs](../crates/sjk-viewer/src/hud/hack_bar.rs))
  and the stock hacking loop plays (`sound/player/hacking.wav`), for 3 seconds. Letting
  go of `+use`, the target walking more than 160 units away or out of the hacker's
  front, or the hacker dying ends it. The hacked player sees "<name> is hacking you!
  Get away", the same bar, and hears the loop from the hacker: walking away is the
  counter.
- **The control:** once the bar is full, the hacked player's client decides. It takes
  the seize only from a player the game shows under the name they claimed, not blocked
  here, whose look has the toy, while its own toy is on and its player plays (not
  spectating or dead, not in a duel, not carrying a flag, its view not held by the
  server: [held view](#held-view)) and was not hacked in the last
  15 seconds, and is not away (idle or alt-tabbed as the [away emote](#away-emote)
  counts it, or an away tag in their name); else the hacker reads why ("Hack failed:
  <name> is in a duel", "is away"). Taken, for
  5 seconds the hacker's movement keys, jump and crouch, walk, attack and alt attack and
  view become the hacked player's: their client sends them as its own user commands,
  keeping the weapon they held, with no Force power, weapon change, saber style change,
  chat or command. Both hear the stock mind trick (`sound/weapons/force/distract.wav`,
  `distractstop.wav` at the end); the hacked player reads "HACKED: <name> has your
  controls", their view turned by the hacker. The hacker's camera goes behind the hacked
  player, turned by the hacker's mouse at once (the player follows after the relay's
  way); the hacker's own player stands still where they were, open to attack, and their
  view comes back when it ends. The hacker may hack again after 15 seconds.
- **The digital rain:** while steered, the hacked player is wreathed in green glyphs (0,
  1 and half-width katakana) falling on a ring round them from feet to head, emitted
  every 70 ms where they stand so the rain follows them (`effects/sjk/hax_glyphs.efx`,
  [assets/hax](../crates/sjk-viewer/assets/hax/README.md)). The hacked player and the
  hacker see it at once; every other SJK player of the server sees it through a feed
  [toy event](hub-chat.md#toy-events) the hacked player sends (`hacked_<until>`, the game
  server time the steering ends; `freed` when they break free sooner), which may come a
  second or two late. Nothing is drawn round a player blocked here.
- **The trap:** with **Hax trap** on (the Hax's card in Collection > Toys since 03/11/2026,
  Settings > Game options > Toys before; `cl_haxTrap`, off by default), a hack that completes on the player springs a trap instead of handing
  over control (Sol's idea, 23/10/2026): both read "IT'S A TRAP!" with Order 66's beep
  each second, and two seconds later both clients send `kill` and both players blow up
  as in [Order 66](#order-66) (the explosion, then the pieces, or confetti with
  `cg_dismember 0`). The trapped player's client tells everyone else through a feed toy
  event (`trap_<moment>`, to the hacker), so every SJK client of the server blows the
  two up too; one that hears it late still blows up the two if they died in the window.
  The trap is not shown anywhere, unless **Hax trap while away** (`cl_haxAway`) is on
  too: then the player may be hacked while away (idle, alt-tabbed or tagged), which
  their look must tell hackers' clients (`toys: ["hax_away"]`), so a player hackable
  while away is known to be trapped. Typing in the chat still keeps a player safe.
- **Breaking free:** `toy_hax` off frees the hacked player at once; so do dying, a duel,
  and the hacker going quiet for 1.5 seconds (after 0.4 seconds without news the player
  stands still). The hacker's toy off, death or duel ends it for both.
- **The way:** no client can move another player on a server, so the hacked player's
  own client plays the hacker's controls. They go through the SJK hub's
  [toy relay](hub-chat.md#toy-relay), not the toy events, which wait for the feed's next
  poll: the hacker's client sends a frame (keys, buttons, view) when they change and
  every 50 ms at least, at most one per 30 ms; a frame still waiting to be sent is
  replaced by the next. Every SJK client on a game server keeps a long poll open for
  the events meant for it (since 23/10/2026, for the [Ragdoll](#ragdoll); before, only
  holders of the toy). The hack's start and end, the seize and the answers travel the same
  way. The steering lags by the hacker's way to the hub, the hub's to the hacked
  player and their ping, and the hacker sees the result after their own ping too. Without
  the hub (`cl_identity 0`), or with a hub from before the relay, nothing is hacked.
  A new map on the same server restarts its clock: a hack under way then ends, and the
  cooldown and the immunity, timed on the old clock, are forgotten.

The icon (a terminal with a green ">_") and the rain's glyphs are drawn by
[hax_assets.py](../scripts/hax_assets.py) from shapes and the bundled JetBrains Mono and
Noto Sans JP;
the sounds and the hack bar's icon are the retail game's. See
[hax.rs](../crates/sjk-viewer/src/hax.rs).

## Ragdoll

The Ragdoll is a meme toy, SJK's own and free for every player (Sol's idea,
23/10/2026): the player's body goes limp and falls where physics takes it, as in the
sandbox games, though the player is not dead. `toy_ragdoll` is a toggle: once to go
limp, again to get up, from a bind (Settings > Key bindings > Interaction > Toy:
Ragdoll, unbound until a key is picked), from the quick wheel's [Toys page](#toys-page)
or from the Collection's Toys tab ([SJK UI](sjk-ui.md#collection)). Nothing is saved:
every fall ends with the map or the server.

- **The body:** a limp humanoid of 15 points (pelvis, chest, head, shoulders, elbows,
  hands, hips, knees, feet) held by sticks, the trunk braced as one block, knees bending
  forward and elbows back (Verlet integration, [body.rs](../crates/sjk-viewer/src/ragdoll/body.rs)).
  A resized model (`iModelScale`: a youngling, a Jawa, the JA+ size pickup) is drawn
  at its size, scaled about the pelvis (#223; it was drawn full size), though the body
  it is simulated with keeps a full-size player's measures.
  It starts standing where the player stood, turned their way, with their speed, and
  pushed off balance (back, to a side, the knees buckling), so a running or jumping
  player tumbles on. It falls at `g_gravity`'s 800, slides and bounces a little on the
  world's brushes, and sleeps once it has lain still for a second (or after 40 seconds
  whatever it does), until a mover stirs it or something strikes it. It is the size the
  player is drawn (since 03/11/2026, Rollembones's report #223): a player the server made
  small or big (`iModelScale`, a youngling or a jawa) falls as a body that much smaller
  or bigger, its points, sticks and boxes scaled about the feet, and the model is drawn
  at that scale on it, so it lies on the floor as a stock-sized one does (before, the
  body was stock-sized and the model drawn full size on it).
- **Its physics** (since 26/10/2026, with the [Monitor](#monitor)): the points weigh what
  the body's parts weigh (the trunk 26 of its 71, a hand 1.2), so the sticks move a hand
  more than the pelvis and a limb flails while the trunk holds its course. A surface
  holds a point by Coulomb friction (0.8 of what the point presses into it, plus 1.5 % a
  step besides), so a slow point stays put (a body lies on a slope up to about 35
  degrees) and a body thrown hard skids and tumbles on rather than stopping dead. The
  limbs keep out of each other: every two points no stick holds stay as far apart as
  their sizes (a leg no longer passes through the other, nor a hand through the head).
  A point goes 3,500 units a second at most (was 2,500); its trace sweeps the whole way.
- **Blows** (since 26/10/2026, the [Monitor](#monitor)'s): a body takes a push at one of
  its points at a given step (`Strike`), as a solid would: its centre of mass gains the
  push, and the push off the centre turns it about its inertia as it lies (24 radians a
  second at most), the struck limb whipping on top. A blow to the head turns the body
  heels over head, one to the legs sweeps them from under it. A sleeping body wakes for
  one.
- **Movers:** since 25/10/2026 lifts, doors, trains and the map's other brush movers are
  in its way too ([scene.rs](../crates/sjk-viewer/src/ragdoll/scene.rs)). Each stands
  where its trajectory puts it at the step's own moment, read from the first snapshot
  at or after that moment, so every client places it alike. A point lying on a mover
  goes with it (a lift takes the body up, a train along, a turning platform round) and
  keeps the mover's speed when it leaves it; a mover that moves into a point pushes it
  along (a closing door shoves the body). A sleeping body wakes when a mover it lies on
  starts, or one comes into it.
- **The model:** the player's own model is drawn posed by it
  ([skin.rs](../crates/sjk-viewer/src/ragdoll/skin.rs)): the `_humanoid` skeleton is cut
  into eleven rigid parts (pelvis, chest, head, upper arms, forearms, thighs, shins),
  each turned as the points it spans, each hung from its parent at the model's own
  joint, so a model of any proportions stays in one piece. The saber, the hat and cape
  and the other bolts follow the posed bones. A model without the stock skeleton is
  drawn as animated.
- **The camera:** behind the body's pelvis, turned by the mouse (as the Hax's hacker's).
  Getting up gives the view back as it was. Flying the free camera while limp, the
  flight keeps its own view and the player's input: the real body is not walked to the
  ragdoll until it ends, in the sent commands or the frame preview between them, and
  rising or the speed key does not get the player up. The ragdoll stays drawn when the
  camera comes near it (28/10/2026; before, the camera did not move and the ragdoll
  could vanish). Plain `fakenoclip` keeps the ragdoll's camera, having no view of its
  own.
- **The real body:** the game server knows nothing of it, so the player's player stays
  there: while limp, the client walks it to where the ragdoll lies (its own user
  commands, running far off, walking near, stopping within 12 units), so that the
  player gets up there. A limp player sends no button and cannot attack. Players on
  stock clients see a player walking about.
- **Names follow the body** (Sol's ask, 26/10/2026): a limp player's nameplate, their own
  plate in third person (`cg_nameplateSelf`) and their overhead name
  (`cg_drawPlayerNames`) hang over the ragdoll as drawn this frame, not over the real
  body walking to it: traced to its pelvis, above its highest point (the head's top
  standing, just over the body lying), whether they fell by themselves or a monitor
  knocked them down (`ragdoll::shown_top`).
- **Getting up:** `toy_ragdoll` again; a new jump, attack, alt attack or taunt
  (`+button3`, `taunt`, `bow`, `meditate`, `flourish`, `gloat`) from 0.5 s after the
  fall, whether they fell by themselves or a monitor knocked them down (Sol's ask,
  27/10/2026: `Ragdoll::wake`, read from the command before the walk takes its
  buttons; one held as they fell counts only once let go and pressed again, and a
  press never begins a fall); or at once when the player is hurt ("Ouch! You
  got up"), dies, spectates, is in a duel, carries a flag or is hacked (a [Hax](#hax)
  hack on a limp player left the body on the floor, Rollembones's report #167), or
  when the server holds their view (a JA+ kiss, kick or knockdown, a melee grapple or
  throw, a saber lock: [held view](#held-view)). Hurt is a new damage feedback from the server (the player
  state's `damageEvent`, which `P_DamageFeedback` moves on at each hurt); comparing
  health instead woke a player who fell again after a hit at once (Sanji's report
  #135). Nobody goes limp in a duel ("No ragdoll in a duel"), with the flag, while
  spectating or dead, in a [Hax](#hax) hack, while the server holds the view ("No
  ragdoll right now") or in a vehicle ("No ragdoll in a vehicle"): a monitor's hit
  left a limp pilot unable to drive or get out (Rollembones's report #168).
- **Everyone sees the same fall:** no pose is ever sent. The faller's client relays
  one start event to each SJK player on the server through the hub's
  [toy relay](hub-chat.md#toy-relay), one every 35 ms: `fall_<moment>_<x>_<y>_<z>_<vx>_<vy>_<vz>_<yaw>`,
  the game server time it began, the origin, speed (whole units) and yaw (degrees),
  then `_<scale>` (the percentage, 1 to 1023) for a player not drawn at the stock size
  (since 03/11/2026; a stock-sized fall reads as before, while clients from before then
  drop a scaled one).
  Every client runs the same simulation from it, in fixed 16 ms steps of the game
  server's clock (the turn from a table of whole degrees, plain arithmetic, so every
  client gets the same numbers), and a client that hears it late runs the steps it
  missed at once: the body is where the faller sees it. A player who comes onto the
  server during a fall (as the hub's looks tell) is told too. Getting up relays
  `up_<moment>`; a client that missed it lets the ragdoll go when the faller dies,
  respawns, leaves, or their real body stays more than 320 units from it for 3
  seconds without coming 24 units nearer (since 26/10/2026: the faller's client walks
  the real body to a ragdoll thrown far, which takes longer). A blow to the body is
  relayed after the fall,
  `hit_<fall moment>_<step>_<joint>_<px>_<py>_<pz>_<thrower>_<number>_<flight step>`
  (the push in units a second; the last three name the monitor), to every player told
  of the fall, and to a late comer after the fall; at most 24 a fall. Since 26/10/2026
  the order is: the thrower of a blow first (fall and blow back to back, 35 ms apart),
  then each player the fall and its blows in a row, a player partly told before one not
  told yet, the nearest to the body first; a new blow reaches everyone already told of
  the fall before a late comer hears the whole of it. A client that hears of a blow whose step its body already ran runs the body
  again from its start with every blow (`Body::rerun`, fast-forwarded at once), so it
  lies where the struck player's does. Clients before 26/10/2026 run the older physics
  and ignore blows: their view of a fall differs. On a game server, every SJK client keeps its relay poll open for this. A
  sender counts only when the game shows the name they claimed, and never one blocked
  here. Without the hub (`cl_identity 0`) nobody else sees the fall.
- **The sounds:** going limp plays a flatline (`sound/sjk/ragdoll/flatline.wav`, two
  monitor beeps then the long tone, SJK's own, made by
  [ragdoll_assets.py](../scripts/ragdoll_assets.py)), to the player and at the body for
  the others; a PK3 that has the same path replaces it. Since 26/10/2026 a player's
  flatline is heard at most once every 15 seconds (`FLATLINE_GAP`, per player, on each
  client); falls within that gap are silent, the toy itself is not held. Since 25/10/2026 every part of
  the body is heard on its own, as in the sandbox games
  ([sounds.rs](../crates/sjk-viewer/src/ragdoll/sounds.rs)): the trunk, the head and
  each arm and leg knock when they hit the world (110 units a second into it, 150 for a
  limb), each at most every 120 ms and at most three parts of a body in a frame, louder
  the harder (a limb a quarter quieter). A knock is two sounds at once: the flesh (the
  trunk the game's body falls, `sound/player/bodyfall_human1.mp3` to `3`, or slams from
  420 units a second, `sound/effects/body_slam1.mp3`, `2`; a limb or the head the melee
  punches, `sound/weapons/melee/punch1.mp3` to `4`, a limb's hard one the kicks,
  `kick1.mp3` to `5`, the head's `sound/effects/thud.mp3`) and what it hit: the
  footstep of its material (stone, metal, wood, grass, sand, snow, mud, gravel,
  carpet..., the footsteps' own families and name guess, a hard knock the run's). A part
  dragged along a surface faster than 120 units a second scrapes on it (its footsteps,
  quiet, at most every 220 ms a body).

The icon (a limp figure slumped against a wall) is drawn by the same script. World shots:
`world_shot::ragdoll_shots::ragdoll_on_a_demo` (with `SJK_TEST_DEMO`). See
[ragdoll.rs](../crates/sjk-viewer/src/ragdoll.rs).

## Held view

A game server holds a player's view in some moves: it rewrites `delta_angles` every
frame so that the view stays where it puts it. SJK knows these cases from one table,
`server_holds_view` in
[prediction_policy.rs](../crates/sjk-game-jka/src/prediction_policy.rs): a melee grapple
(`BG_InGrappleMove`'s animations) or throw (`HANDEXTEND_PRETHROWN`, `POSTTHROWN`), a
saber lock, and on a JA+ server JoF EJK's `CG_JAPlusViewLockedState` (kissing, the
backflip kick's victim, the stab, hanging from a ledge, kicked down, getting up)
([held_view.rs](../crates/sjk-viewer/src/held_view.rs)).

- Knockdowns and get-ups are held or not by server family, as no serverinfo says which:
  EternalJK's cgame holds them and some JA+ servers do, so every JA+ server does in SJK
  unless its family is in `FREE_KNOCKDOWN_VIEW` in
  [held_view.rs](../crates/sjk-viewer/src/held_view.rs) (one entry per family with its
  evidence, families told apart by
  [known_servers.rs](../crates/sjk-mod/src/known_servers.rs)), which sets
  `JaPlusRules::free_knockdown_view` for the pmove and `server_holds_view`. Its one
  entry is JoF's servers: in two demos of their knockdowns (27/10/2026,
  `BOTH_KNOCKDOWN1`, `BOTH_GETUP1`, the get-up rolls, `BOTH_FORCE_GETUP_B1`)
  `delta_angles` never changed and the view followed the mouse. Counted as held there,
  `cg_heldViewLook 0` locked the camera through them (Kushina's report #169), and the
  pmove's JA+ freeze held the view through `BOTH_GETUP1`, so every snapshot pulled the
  camera back to the server's lagging view (Rollembones' report #164). The get-up still
  holds the body everywhere. `cg_japlusFreeKnockdownView` (archived, console only)
  overrides the table: -1 by the family (the default), 0 held on every JA+ server, 1
  free on every one. `cg_noPredict 1` still takes the server's angles through every
  `CG_JAPlusViewLockedState`, as EternalJK does.
- A JA+ emote (`PM_FREEZE`) never holds the view, even one that plays a knockdown's or a
  kiss's animation (`amneo` is `BOTH_FORCE_GETUP_B4`, `amkiss` the kiss): the base game
  turns a frozen player's view, and the mouse stopped turning the camera through them
  (vini's report #166).

While the view is held:

- the mouse turns the view as in the base game while Free look when held
  (`cg_heldViewLook`, Settings > Game options and Advanced > Gameplay) is on, the
  default since Sol found the locked camera worse than the shake; switched off, the
  mouse does not turn it (the motion is dropped), as a view turned under the server's
  rewrites shakes, kicked down by a JA+ kata or a melee kick (Rollembones' report
  #146). A free camera and fake noclip are not held;
- the [Ragdoll](#ragdoll) gets the player up and does not start, and a [Hax](#hax) hack
  ends or is refused (the hacker reads "is busy"): their commands carry angles of their
  own, so every rewrite turned the camera on by the gap between the two views and it
  spun (Rollembones' report #139, a kiss; Foshin's #140, a grapple).

## Drone

The Drone (Sol's idea, 24/10/2026, grown from Slash's Peek work in #199) makes the free
camera and Peek a toy: a little training remote, Jedi Academy's own seeker ball
(`models/items/remote.md3`, at the game's own size), that every other SJK player on
the server sees fly, the pilot's name over it (white, shadowed, within 2,400 units and
not behind a wall), coming out with the game's remote hiss.

- `freecam` (the detached free camera, [status.md](status.md#detached-free-camera)) flies the drone: it is where
  the camera is, turned the way it looks.
- `peek` ([timed player peek](#timed-player-peek)) sends it to a player: it hovers
  where the peeking camera stands, behind them and above (`peek::behind`).

Both commands, their binds and their quick wheel choice are unchanged; they fly the
drone whatever the switch. The switch, `toy_drone` (a bind, the quick wheel's Toys page,
Collection > Toys, "Peek me"), is the consent to be peeked: on a server `peek` refuses a
player whose look does not show the toy ("has not let drones follow them"), and a peek
under way ends as soon as the watched player turns it off ("Name sent your drone away":
their client relays `away` to the pilot at once, and again at each repeat of a drone
watching them while it is off; failing that, the pilot's client ends it when their look
shows it off, "Your drone was sent away").
The watched player does not see the drones watching them (they would hover in their
back); a line near the top of their screen names who watches while it lasts ("Name is
spectating you with the Drone", names joined when several do). A player in a private
duel, or following one, does not see the drones of pilots outside it (Sanji's report
#145: they distracted the duel), on every server since drones are SJK's own, unless
`cg_duelHideOthers` is 0 ([Nameplates](#nameplates)). It stays as last switched, across maps, servers and restarts
(`cl_toyDrone`, archived). In a local game no look is shared, so anyone may be peeked
and nobody sees a drone.

No game server knows of it. The pilot's client tells every other SJK player where the
drone is through the toy relay (`PROTOCOL.md`, "Toy relay";
[hub-chat.md](hub-chat.md#toy-relay)): `fly_<x>_<y>_<z>_<yaw>` (whole units, degrees)
while the camera flies, `peek_<slot>` while it watches a player, `land` when it is put
away. One event goes every 50 ms, to the players in turn: a player hears the drone's
place again once it moved 2 units or turned 3 degrees, or every second while it holds
still; so with two players each hears of it twenty times a second at most, with ten
twice. A drone not heard of for 3 seconds goes. Each client glides the drones it shows to
the latest place (and, for a peek, finds the place behind the watched player itself from
what it shows of them), so they move smoothly between events. The pilot's own drone is
not drawn on their screen (it is their camera). Players on stock clients see nothing.
The icon (a training remote hovering) is drawn by `scripts/drone_assets.py`. See
[drone.rs](../crates/sjk-viewer/src/drone.rs).

## Boombox

The Boombox (Sol's idea, 25/10/2026) is a toy for verified SJK players: a big 80s stereo
on the shoulder, playing music from the player's own files to the SJK players standing
close by. It works like emotes: the music is in pk3s, and SJK passes only its name.

- **Tracks** are `sound/boombox/<id>.mp3` (or `.wav`) in any pk3 or the game folder;
  `<id>` is 1 to 24 of `a` to `z`, `0` to `9` and `_`, and shows with spaces for its `_`.
  SJK ships none.
- **`toy_boombox`** (a bind, the quick wheel's Toys page, Collection > Toys, "Play") puts
  the stereo on the shoulder and plays the chosen track (the first at first), or puts it
  away. `boombox` in the console lists the tracks; `boombox <id>`, `next`, `prev`, `play`
  and `stop` choose and play them (Settings > Key bindings > Interaction has "Boombox:
  next track"). When a track ends the next one plays, the first after the last. It is off
  at every start and on every new server. Starting names the track on the centre of the
  screen; putting it away says "Boombox OFF" in red.
- **Verified players only** (Sol's rule, 25/10/2026): on a server only a player whose key
  the SJK hub verified may turn it on ("The boombox is for verified SJK players"), and every
  SJK client ignores the boombox, music and stereo, of a player the hub does not show as
  verified. In a local game anyone may carry it.
- **Heard close by:** Settings > Game options > Toys, "Boombox range"
  (`cg_toyBoomboxRange`, 128 to 1000 units, default 400; a player is about 64 tall) sets
  how far the player's own carries. It plays at full volume within a third of that and
  fades to nothing at the edge; a listener further away (64 units more once it plays)
  does not play it at all. The carrier hears their own in both ears. Settings > Sound >
  Toys, "Boombox music" (`s_toyBoomboxSound`), silences every boombox on this PC, and a
  player blocked here is never heard.
- **Others hear it** only when they have the same file: the same id and size. A listener
  without it reads once in the console which track it is
  (`Name's boombox plays <id>, which you do not have`).
- **On the floor** (03/11/2026, after VisionJK's boombox by Blackwolf, who agreed):
  `boombox_place` (Collection > Toys "Place key", the quick wheel's "Place boombox",
  Settings > Key bindings > Interaction "Boombox: place or pick up") stands the stereo
  on the floor 40 units ahead of the player, short of a wall, facing the way they look
  (on the floor under them when there is none ahead; "Nowhere to put the boombox down
  here" when neither is near), putting the toy on first if it was off; pressed again it
  goes back on the shoulder ("Boombox back on your shoulder"). Not while dead,
  spectating or following. One box a player. The music then plays from the box (its
  owner hears it there too, no longer in both ears), with the same range and fade, and
  quieter behind it: right behind, 45% as loud as in front, beside it as loud (the
  audio fades a track by distance only, so behind the box it plays from further away
  along the same line, [heard_from](../crates/sjk-viewer/src/boombox.rs)). The box goes
  when picked up, with the toy turned off, on a new map or server, and for the others
  when its owner leaves the server, drops off the hub or is blocked here.
  `cg_toyBoomboxShow` 0 hides others' boxes and their music, as their shoulder stereo.
  Staff cannot remove another player's box yet (that needs the hub).

No game server knows of it, and the hub never carries sound. The stereo is a toy in the
player's look (`toys: ["boombox"]`), so every SJK player on the server draws it over the
carrier's right shoulder (on the `*back` bolt, upright, facing where the body faces; not
on the dead; one's own in first person only in mirrors). What plays travels through the
[toy relay](hub-chat.md#toy-relay): `play_<moment>_<bytes>_<range>_<id>` names the track,
its file's size, the game server's time it began at and how far it carries; `stop` ends
it. The carrier's client tells each SJK player within the range and 128 units more, one
event every 50 ms, again whenever one comes back within reach and every 15 seconds; a
track not told again for 40 seconds stops. A box on the floor is told to every SJK
player on the server, before its music (the players within the music's reach first) and
again every 15 seconds: `put_<x>_<y>_<z>_<yaw>`, its floor point in whole units (each
client finds the floor under it itself) and the way it faces in degrees; `take` picks it
up, and a box not told again for 40 seconds goes. Clients from before 03/11/2026 ignore
both and keep the stereo on its carrier's shoulder. A listener starts the track as far in as the
server's time says, so everyone hears the same part of it whenever they arrive: the file
is read and decoded when first needed (not before), the start waits for the decoder and
then skips the time it waited (`AudioCommand::PlayTrack`), and a track nothing plays any
more is freed. The music plays on the protected emote channel (a full voice table drops
every other sound first) but on a source of its own per player, so an emote never cuts
it. The stereo's model, picture, shader and icon are bundled, made by
[boombox_assets.py](../scripts/boombox_assets.py) from shapes (nothing taken from another
game), and mounted below all game data, so a PK3 with the same paths replaces them. See
[boombox.rs](../crates/sjk-viewer/src/boombox.rs).

## Monitor

The Monitor (Sol's idea, 26/10/2026, after the old sandbox-game joke of hurling a big CRT
monitor at someone) is a toy for every SJK player: throw an old square monitor, and one
that hits a player who has the toy on too knocks them down as a [ragdoll](#ragdoll) and
sends them flying, the way the monitor struck them.

- **`toy_monitor`** (a bind, the quick wheel's Toys page, Collection > Toys, "Hit me")
  is the switch, both the right to throw and the consent to be hit (Sol's rule: always
  opt in). It stays as last switched, across maps, servers and restarts
  (`cl_toyMonitor`, archived). A monitor passes through a player without it.
- **`monitor_throw`** (the Toys page's Throw monitor, the console) throws one from the
  eyes, the way the player looks, at 1,100 units a second plus the player's own speed,
  tumbling end over end; one every 1.2 seconds at most, four of a player's out at once
  (the oldest goes). Without the toy on it says "Turn the Monitor toy on to throw one";
  nobody throws while limp, dead, spectating, in a duel, with the flag, hacked, held by
  the server or in a vehicle ("No monitor right now"), and nobody in a vehicle is
  knocked down.
- **The charged throw** (Sol's ask, 26/10/2026): `+monitor_throw`, the held bind
  (Settings > Key bindings > Interaction > Throw monitor, the Collection's Throw key),
  charges while held and throws on the release, from 1,100 units a second for a tap to
  2,600 once held a second. A ring round the crosshair fills clockwise from the top,
  yellow turning orange, white when full, and the thrower hears a tube's hum rise to a
  ready ping (`charge.wav`), stopped on the release. Pressed during the cooldown, it
  begins charging once the cooldown ends; it is put down when the player may no longer
  throw, or held 6 seconds (a release lost to the window). A throw leaving the hand
  faster than 1,800 is heard by everyone as a charged one, a thump and a deep whoosh
  (`throw_hard.wav`), from the speed its event carries: no new event, so the clients
  before it see the same flight (they hear the plain whoosh). A key bound to the plain
  `monitor_throw` alone before is bound to `+monitor_throw` when the profile loads; a
  bind running it among other commands stays, throwing at once.
- **The monitor** is a solid of eight corners held by sticks between every two
  ([flight.rs](../crates/sjk-viewer/src/monitor/flight.rs)), the same Verlet steps as the
  Ragdoll's body: it bounces off the world (more than a body, plastic and glass), slides
  with friction, topples over an edge and comes to rest on a face. It lies 10 seconds,
  then shrinks away; one still flying after 20 seconds stops.
- **The hit** is decided by the struck player's client alone, against its own body: a
  standing player's box (lower crouched), or a limp one's ragdoll points. The monitor
  must come at them, faster than 250 units a second against their own speed. The body
  falls limp (if standing) and takes a [blow](#ragdoll) at its point nearest the
  monitor: the monitor's speed against the body's times 1.5, 260 upwards, 3,000 units a
  second at most (the hardest blow a ragdoll takes, 2,600 before the charged throw). A
  monitor thrown at someone standing still sends them off at about 1,650 units a second,
  a fully charged one at 3,000. The monitor's way over each step is tried every 6 units,
  so a fast one does not pass through a limb between two steps. A ragdoll can be struck again where it lies or flies (not for
  0.2 seconds after a blow, nor by the same monitor for 0.6). The thrower's own monitor
  can hit them once it is 0.4 seconds out of the hand. The monitor bounces back off the
  body (half the body's push, back, and 150 up).
- **Busy players are not hit** (Sol's ask, 26/10/2026): a player typing (a key catcher
  open: the chat, the console or a menu, what sends the talk balloon's `BUTTON_TALK`,
  `GpuState::talk_button`) or away (alt-tabbed or idle as the away emote counts it, or an
  away tag in their name: the Hax's `hax_own_away`) is not struck; the monitor passes
  through. Other clients foresee no hit on a player showing the talk balloon
  (`EF_TALK`) or whom their scoreboard counts away (`seen_away`).
- **Foreseen hits** (Sol's ask, 26/10/2026, after hits showed a ping or more late):
  the struck player's client alone decides a hit, and its word reaches the others a
  ping or more later (and after its turn in the relay's 35 ms queue). So every SJK
  client also tries each monitor against the other players who have the toy on (their
  look lists `monitor` since then), as it shows them (the presented body's box, or
  their ragdoll), with the same rules (a step no older than 0.3 seconds, the same
  immunity, never the thrower themselves, nobody dead, in a duel or blocked). On a hit
  it knocks the body down there and then (a fall from where it shows them, with the
  blow), bounces the monitor and plays the bonk. Their own word then takes its place:
  their `fall_` replaces the foreseen fall, keeping the foreseen blow until their
  `hit_` replaces it (`Body::without`), and the monitor's foreseen bounce gives way to
  the real one (`Flight::without`). Without their word within 1.2 seconds
  (`ragdoll::GUESS_WAIT`: they dodged on their screen, carry the flag, or stand where
  the client did not show them), the blow is undone: a foreseen fall ends and they
  stand again, a foreseen blow leaves a fall they were already in. One foreseen blow
  waits at a time for a player.
- **The hit marker:** when the struck player's blow reaches the thrower's client (the
  `hit_` event, so a ping or so after the hit), or at once when the thrower's client
  foresees it, four short strokes stand out diagonally
  from the crosshair and fade over 0.35 seconds, spreading a little, red for a blow of
  2,400 or more (a charged throw), white otherwise, and the thrower hears a bright tick
  (`hit.wav`). Not for a hit on themselves, nor one heard more than 1.5 seconds after it
  happened. The marks go with the crosshair (`cg_drawCrosshair` 0 hides them)
  ([monitor_marks.rs](../crates/sjk-viewer/src/hud/monitor_marks.rs)).
- **Unseen in duels, or on request** (Sol's ask, 26/10/2026): while the player, or the
  player they follow, is in a private duel, and on Duel and Power Duel servers, no monitor
  is drawn or heard, and no charge ring or hit marker shows (nobody throws or is hit in a
  duel anyway; this hides the monitors of the players around it). Settings > Game options >
  Toys, "Show monitors", and the Monitor's card in the Collection's Toys tab, "See
  monitors" (`cg_toyMonitorShow`, archived, default 1), at 0 hides every
  monitor and its sounds in the world on this PC, the player's own too; their charge ring,
  charge hum, hit marker and tick stay. Neither changes who may be hit: a player with the
  toy on and monitors hidden can still be struck.
- **The Screen Saver** (Sol's idea, 29/10/2026): a player struck by 5 monitors of other
  players within 20 seconds is untouchable by monitors for 30 seconds, and may still
  throw their own ([saver.rs](../crates/sjk-viewer/src/monitor/saver.rs)). A monitor counts
  once however often it strikes (one juggling a body), the player's own never, and the
  count starts afresh once it rises. The fifth hit still lands; from then every monitor
  passes through the player, standing or limp, as through a player without the toy. The
  centre of the screen says "SCREEN SAVER: monitors pass through you for 30 seconds".
  A gold shell wraps the body: Force Absorb's (`gfx/misc/personalshield`) tinted gold,
  as many passes as `cg_shieldBrightness`, fading in, pulsing gently, and flickering
  over its last 3 seconds. A CRT's degauss is heard as it rises (`saver_on.wav`) and its
  power-off as it ends (`saver_off.wav`), the player's own in both ears, another's where
  they are. It ends at its time, or sooner with the player's death or a new map (the
  game clock going back). Others learn of it through a feed
  [toy event](hub-chat.md#toy-events) the player sends to the one who raised it
  (`monitor` `saver_<until>`, the game server time it ends; `saver_off` if the player
  dies first): their clients draw the shell and foresee no hit on the player meanwhile,
  and drop it at its time or the player's death. Clients from before it see no shell
  and may foresee a hit that the player's own client refuses, which they undo as any
  hit whose word does not come. No shell shows in a duel or with `cg_toyMonitorShow` 0.
  It brings two achievements ([identity.md](identity.md#achievements)): Payback and
  the hidden Power Surge.
- **Getting up:** a fall a monitor began ends by itself 2.5 seconds after the body came
  to rest, once the real body has walked to it (or 6 seconds later whatever it did);
  `toy_ragdoll` gets up at once, and being hurt does as for the Ragdoll.
- **The sounds** are SJK's own, synthesized by
  [monitor_assets.py](../scripts/monitor_assets.py): a whoosh as it leaves the hand
  (`sound/sjk/monitor/throw.wav`), a hollow plastic thunk with the glass ringing when it
  strikes a body (`bonk.wav`), a clunk and rattle when it knocks on the world harder than
  160 units a second (`knock.wav`, at most every 120 ms a monitor, louder the harder:
  from 0.55 to full at 600). A knock carries farther than the game's own sounds, at full
  volume to 768 units and silent past about 2,000 (`KNOCK_REACH`): with the game's
  falloff (silent past 1,500, half at 900) the knocks of a monitor landing far from
  the listener went unheard (Sol, 28/10/2026). Tests throw a monitor at a floor and a
  wall, in shapes and in a map's brushes, and count the knocks heard.
  Settings > Sound > Toys, "Monitor sounds" (`s_toyMonitorSound`), silences them.

No game server knows of it: the struck player's real body stays where it was and walks
to the ragdoll, as for the Ragdoll; players on stock clients see nothing. Every SJK client
on the server sees the same throw: the thrower's client relays
`throw_<number>_<moment>_<x>_<y>_<z>_<vx>_<vy>_<vz>` (the monitor's number among the
thrower's, 0 to 99; the game server time; where it leaves the hand and how fast, whole
units) through the [toy relay](hub-chat.md#toy-relay) to each SJK player in turn, the
nearest first, one every 40 ms, and each runs the same flight from it in 16 ms steps of
the game server's clock, fast-forwarding a throw heard late (up to 8 seconds). A hit
travels as the Ragdoll's `hit_` event, which names the monitor, so every client bounces
it off the body at the same step (a flight that already ran past it runs again from its
throw). A sender counts only when the game shows the name they claimed, and never one
blocked here. The model (a beige CRT, 20 x 17 x 18 units), its picture, shader, icon and
sounds are bundled, made by the script from shapes, noise and synthesized tones (nothing
taken from another game), and mounted below all game data, so a PK3 with the same paths
replaces them. World shots: `world_shot::monitor_shots::monitor_on_a_demo` (with
`SJK_TEST_DEMO`). See [monitor.rs](../crates/sjk-viewer/src/monitor.rs).

## Banana peel

The Banana peel (Kushina's idea, 29/10/2026, which Sol took up) is a toy for every SJK
player: drop a banana peel on the floor, and a player who has the toy on too and walks
onto it slips, cartoon style, and falls as a [ragdoll](#ragdoll).

- **`toy_banana`** (a bind, Settings > Key bindings > Interaction > Toy: Banana peel, the
  quick wheel's Toys group, Collection > Toys, "Slip on peels") is the switch, both the
  right to drop peels and the consent to slip on them (the Monitor's rule: always opt
  in). It stays as last switched, across maps, servers and restarts (`cl_toyBanana`,
  archived), and says "Banana peel ON" or "OFF" on the centre of the screen, as every
  toy does ([Illuminate](#illuminate)). A player without it walks over peels.
- **`banana_drop`** (Interaction > Drop banana peel, the Collection's Drop key, the quick
  wheel's Drop banana peel) drops one: standing (slower than 80 units a second along the
  floor) 20 units ahead of the feet the way the player looks, moving 72 units ahead the
  way they move, short of a wall. It leaves the hand 32 units over the floor, tumbles
  through a short arc for 0.4 seconds and lands with a soft flop (`drop.wav`) on the floor
  under that spot, lying along it (a floor steeper than about 45 degrees takes none, nor
  the sky: "Nowhere to drop a peel here"). Without the toy on it says "Turn the Banana
  peel toy on to drop one"; nobody drops while limp, dead, spectating, in a duel, with the
  flag, hacked, held by the server or in a vehicle ("No banana peel right now"), the
  Ragdoll's own rules.
- **Limits** (anti-spam, named in [banana.rs](../crates/sjk-viewer/src/banana.rs)): one
  drop every 2 seconds; three peels down a player, a fourth taking the oldest's place;
  each lies 2 minutes, then shrinks away; none within 96 units of a spawn point (the map's
  `info_player_deathmatch`, `info_player_start`, duel, Siege and CTF team spawns, read
  from its entities: "Not so close to a spawn point"). Every peel goes with a new map or
  another server, and a player's go when they turn the toy off or leave.
- **The slip** is decided by the slipping player's client alone, as the Monitor's hit is,
  against its own predicted player: on the ground, moving at least 40 units a second along
  it (a crouched walk still slips), their feet within 14 units of a peel that has landed
  (no more than 12 units over it nor 8 under it). The peel must be one they came at: a
  peel that lands under someone waits until they have been more than 26 units from it,
  and the dropper's own cannot trip them for 1.5 seconds after it lands (a peel tossed
  ahead while running would trip its dropper at once). The rules that keep a player from
  dropping keep them from slipping. They hear a cartoon slip, a squeaky skid and a slide
  whistle zipping up and dropping (`slip.wav`, in both ears; at the peel for everyone
  else), and their body goes limp as the Ragdoll's, taking a blow at a foot along the way
  they walked and upwards (40 and 110 units a second), which throws the feet up over the
  head and lays the body on its back, sliding on a little with the speed they had. The
  peel is gone for everyone. They get up as from a monitor's hit: a new jump, attack or
  taunt, `toy_ragdoll`, being hurt, or by themselves 2.5 seconds after the body comes to
  rest.
- **Unseen in duels, or on request:** while the player, or the player they follow, is in a
  private duel, and on Duel and Power Duel servers, no peel is drawn or heard. Settings >
  Game options > Toys, "Show banana peels", and the toy's card in the Collection's Toys
  tab, "See banana peels" (`cg_toyBananaShow`, archived, default 1), at 0 hides every peel
  and its sounds in the world on this PC, the player's own too. Neither changes who may
  slip: a player with the toy on and peels hidden still slips on the ones they walk onto.
  Settings > Sound > Toys, "Banana peel sounds" (`s_toyBananaSound`), silences them.

No game server knows of it. While it is on, the player's look lists it, as the other
toys' do (`toys: ["banana"]`, since 30/10/2026, when the hub's looks went from eight toy
ids to 32; [unlockables.md](unlockables.md); `looks::TOYS`). The peels go otherwise:
every SJK client on the server learns of a peel through the
[toy relay](hub-chat.md#toy-relay). The dropper's client tells each SJK player in turn,
the nearest first, one event every 50 ms, and the ones who come later while it lies:
`drop_<number>_<moment>_<x>_<y>_<z>_<dx>_<dy>_<dz>_<yaw>` (the peel's number among the
dropper's, 0 to 99; the game server time it left the hand; where it lands, whole units;
the hand's offset from there, 128 at most; its turn in degrees). Each client finds the
floor under that spot itself and lays the peel on it, and ignores a peel by a spawn
point. The slipping player's client tells every SJK player `slip_<dropper>_<number>`, and
a client that hears it plays the slip at the peel and takes it away (a drop heard after
its slip stays away); the dropper's client tells `clear` to everyone it told of a peel
when the toy goes off, and every client takes a dropper's peels away once the hub's
presence no longer shows them on the server. The fall travels as the Ragdoll's, its blow
(`hit_...`) naming the peel's dropper and no monitor (`ragdoll::UNTHROWN`), so clients
from before the toy see the same fall. A sender counts only when the game shows the name
they claimed, and never one blocked here. Players on stock clients see nothing. The peel
(about 19 units long and 12 wide: its closed end with the stem's stub, four strips
fanning out, two curling up),
its picture, shader, icon and sounds are SJK's own, made by
[banana_assets.py](../scripts/banana_assets.py) from parametric strips, noise and
synthesized tones (nothing taken from another game), and mounted below all game data, so
a PK3 with the same paths replaces them
([assets/banana](../crates/sjk-viewer/assets/banana/README.md)). World shots:
`world_shot::banana_shots::banana_peels_ahead_of_the_first_spawn` (`SJK_BANANA_MAPS`),
`banana_peel_toys_card`, and `banana_slip_on_a_demo` (with `SJK_TEST_DEMO`). See
[banana.rs](../crates/sjk-viewer/src/banana.rs) and
[banana_frame.rs](../crates/sjk-viewer/src/banana_frame.rs).

## Prop gun

The Prop gun (Sol's idea, 01/11/2026) is a toy for every SJK player: a fake weapon held
instead of the real one, whose bolts knock down the players who have the toy on too, as
the [Monitor](#monitor) does. SJK ships no gun: each is a file in a pk3 the player adds
themselves, and other players see and hear a gun only when they have the same file.

- **The guns** are `propguns/<id>.gun` in any pk3 or the game folder; `<id>` is 1 to 24
  of `a` to `z`, `0` to `9` and `_`. One setting a line, its name then its values (`//`
  starts a comment):

  | Setting | Values | Default |
  | --- | --- | --- |
  | `name` | the name said when it is taken out | the id |
  | `view`, `world` | the first-person and third-person models (MD3), needed | |
  | `viewOffset`, `worldOffset` | x y z on the model's own axes (X forward, Y left, Z up) | 0 0 0 |
  | `viewAngles`, `worldAngles` | pitch yaw roll, degrees | 0 0 0 |
  | `viewScale`, `worldScale` | how much bigger | 1 |
  | `fire` | a shot others hear (several lines: one each shot in turn) | |
  | `fireOwn` | the shooter's own shot, in both ears (else `fire`) | |
  | `impact` | where a bolt meets the world or a body (several lines) | |
  | `raise` | played when the gun is taken out | |
  | `boltModel`, `boltScale` | the bolt's model along its way (none: its light only) | none, 1 |
  | `boltColor`, `boltLight` | its light's colour (0 to 1) and reach | 0.4 1 0.3, 120 |
  | `boltSpeed`, `boltLife` | units a second, milliseconds | 1800, 3000 |
  | `impactEffect` | an effect where it meets the world or a body (`repeater/wall_impact`) | none |
  | `fireEffect` | an effect where it leaves, along its way, at each shot | none |
  | `boltEffect` | an effect started behind the bolt every 25 ms as it flies (a trail) | none |
  | `boltSound` | a sound looped at the bolt as it flies (8 heard at once at most) | none |
  | `refire` | milliseconds between two shots | 350 |
  | `push` | how hard a bolt pushes the body it strikes, units a second | 900 |

  A file with a fault is skipped and the console says which line. The models load with
  the map, so a gun added during a match is drawn from the next one.
- **`toy_propgun`** (a bind, the console) is the switch, both the right to fire and the
  consent to be hit (always opt in). It stays as last switched, across maps, servers and
  restarts (`cl_toyPropGun`, archived), and says the gun's name and ON, or "Prop gun OFF".
  **`propgun`** lists the guns; `propgun <id>`, `next` and `prev` choose one (`cl_propGun`,
  archived; the first gun at first).
- **Held:** while it is on, the gun is drawn instead of whatever the player holds, saber
  included. In first person it floats ahead, right and down of the eyes, moved by the
  view bob and the view-model FOV (it does not follow a hand rig's animations), and
  `cg_drawGun 0` hides it. In third person it hangs on the right hand (`*r_hand`), where
  the real weapon would, turned as the game's own world guns are (the world model has the
  view model's axes); the body keeps the animations the server plays (a saber stance
  with a saber out).
- **Firing:** attack and alt attack never reach the game server while a gun is held
  ([live_session.rs](../crates/sjk-viewer/src/live_session.rs)); a press, or attack held,
  fires once every `refire`. The bolt leaves from the first-person gun's muzzle (the view
  model's `tag_flash`, as drawn), else from by the eyes (16 ahead, 5 right, 5 down), towards what the crosshair meets, and flies straight at its speed until it meets the
  world (its impact sound and effect) or a body, or for its life. Nobody fires while limp,
  dead, spectating, in a duel, held by the server or in a vehicle (the Ragdoll's rules).
- **The hit** is decided by the struck player's client alone, against its own body (a
  standing player's box, or a limp one's ragdoll points), when a bolt passes within 6
  units of it, tried every 4 units of its way. The body falls limp as the Ragdoll's and
  takes a blow at the point nearest, along the bolt's way at `push` with 220 upwards
  (3,000 units a second at most). Not by the player's own bolts, not twice by one bolt, not
  within 0.25 seconds of a blow, and not while typing or away (the Monitor's rules). The
  fall travels as the Ragdoll's, its blow naming the shooter and no monitor
  (`ragdoll::UNTHROWN`).
- **Seen, or not:** `cg_toyPropGunShow` (archived, default 1) at 0 hides other players'
  guns, bolts and their sounds on this PC; `s_toyPropGunSound` silences every Prop gun
  sound. In a duel, or following one, no gun is drawn and no bolt flies.

No game server knows of it, and the hub never carries a gun's files. What a player holds
and fires goes through the [toy relay](hub-chat.md#toy-relay), one event every 30 ms,
the player's newest shots first: `hold_<id>` (told to each SJK player, again every 15
seconds; forgotten after 40 seconds untold), `put`, and
`fire_<number>_<moment>_<x>_<y>_<z>_<dx>_<dy>_<dz>_<id>` (the shot's number among the
player's, 0 to 99; the game server time; where it left, whole units; its way, in
thousandths). Each client flies the same bolt from its moment and drops a shot heard more
than 1.5 seconds late. A sender counts only when the game shows the name they claimed,
and never one blocked here. Players on stock clients see nothing. Not in the Collection's
Toys tab, the quick wheel or the settings yet. World shot:
`world_shot::propgun_shots::propgun_on_a_demo` (with `SJK_TEST_DEMO` and
`SJK_TEST_PROPGUN`, the id of a gun in the game data). See
[propgun.rs](../crates/sjk-viewer/src/propgun.rs) and
[propgun_frame.rs](../crates/sjk-viewer/src/propgun_frame.rs).

## Tag

The Tag (Sol's idea, 27/10/2026, after TF2's sprays) is a toy for every SJK player: stick
a premade picture on the wall or floor you look at, and every SJK player on the same map
sees it there.

- **The tags** are unlocks of the hub (its `PROTOCOL.md`,
  "Unlocks"; the client's list is [tags.rs](../crates/sjk-viewer/src/tags.rs)), each with a
  tier as shaders have. Three are everyone's from the start (SJK, GG, Fox); the others drop
  from holocrons ([holocrons.md](holocrons.md)), duplicates stacking. There are no uploads
  (Sol's decision, 27/10/2026): Sol picks every tag, new ones from rounds on Discord, and
  each needs a new id in both catalogues and its picture in the hub's `sjk_tags` pack.
  Players propose them in the Discord's `#tag-proposals` channel (28/10/2026; a pinned
  post says how: one idea per post, voted with reactions, a bold square PNG, nothing
  taken from another game), which the tag card in Collection > Tags points to.
- **Finishes and credits cards** (31/10/2026, Sol's decisions): a tag a holocron drops
  comes Holo (4 %) or Solar (0.5 %) now and then, whatever the holocron's tier
  ([holocrons.md](holocrons.md)); the hub counts those copies apart. The credits cards the
  player holds (SJK Alpha's kind `credits`, [cards.md](cards.md#faces-and-the-first-set))
  stick as tags too: the card's picture as a sticker, its corners rounded and a rim of the
  card's colour round it. A tag or a card sticks in a finish the player holds a copy in
  (Normal needs any copy; a free tag is only Normal). Both need a hub whose profiles list
  `tag_cards` (the card kinds it lets be tags); an older hub refuses them, so the client
  offers neither there and Collection > Tags lists no cards.
- **Tag copies** (Blep's idea, 03/11/2026, with a hub whose economy says `tag_copies`):
  the card of a dropped tag held twice or more offers Break down (one copy of the finish
  shown into kyber shards: the hub's `economy.tags`, by tier times the finish, more for
  Holo and Solar) and Craft (two spare copies of one tier and finish, this tag's first,
  for a tag of that tier the player holds none of, or none in that finish, in that
  finish), each on a second press; the player always keeps one copy of every tag, and the
  free tags never take part. Craft is greyed with the reason when every tag of the tier
  is held in that finish or two spare copies are lacking. The hub's answer reads under the
  buttons ("Holo Gonk tag broken down: +25 kyber shards.", "Crafted: Kyber tag. It is
  yours."). Spare copies trade too: the trade window lists them with their finish beside
  the duplicate shaders, counted as duplicates in the day's limits
  ([tag_copies.rs](../crates/sjk-viewer/src/tag_copies.rs)).
- **Choosing:** Collection > Tags shows every tag, then the credits cards under their own
  heading, the ones the player lacks dimmed and the Holo and Solar copies held as badges;
  the wall scrolls (the wheel, the arrows keeping the choice in view). The card on the right
  offers Normal, Holo and Solar (the finishes held), shown over the picture; Choose (or
  Enter) sets `cg_tag`, the tag or card `toy_tag` sticks (default `tag_sjk`), and
  `cg_tagFinish`, its finish (`normal`, `holo` or `solar`; one not held sticks Normal). A
  finish chip of the tag in use changes its finish at once.
- **Sticking:** `toy_tag` (a bind, Settings > Key bindings > Interaction "Toy: Tag", the
  quick wheel's Toys page) traces the crosshair 128 units past the player's eye into the
  world (in third person with `cg_dynamicCrosshair 0` the trace starts at the camera, so
  the camera's distance is added, at most 256; Kushina's tags did not go up) and sticks the
  chosen tag where it lands: upright on a wall, its top away from the player on a floor,
  56 units wide. One tag a player: a new one replaces the last, at most one every 10
  seconds. `toy_tag clear` takes it off. A tag the player does not hold is refused, and
  so is a spot no tag holds on (below): "A tag does not hold on that surface".
- **The spray** (Sol's request, 03/11/2026): a tag going up plays a spray can, the ball
  rattled three times then two hisses (`sound/sjk/tags/spray.wav`, synthesized by
  [tag_sounds.py](../scripts/tag_sounds.py), bundled), heard where the tag lands. Another
  player's is heard when their look changes to a new tag on this map, not when their look
  arrives already carrying one (joining, coming back) nor while `cg_toyTags 0` hides them.
  `s_toyTagSound 0` silences it.
- **Others see it** through the player's look: the tag rides in it (`tag`, the tag's id,
  map, point, surface normal and turn, and its finish when not Normal), so players who come
  later see it too, and the hub refuses a tag the key does not hold, or not in that finish
  (`tag_not_unlocked`). Clients from before 31/10/2026 draw a finished tag plain and a
  card's nothing. It stays until the player
  sticks another, clears it, changes map or server, or leaves. A client draws it only on
  the same map. `cg_toyTags 0` hides other players' tags on this PC; a muted or blocked
  player's never show.

A tag is a lasting decal with a slot of its own per player
([decal_store.rs](../crates/sjk-viewer/src/decal_store.rs)): projected once when it
changes (as impact marks are, over corners and patches), kept until it goes, never pushed
out by impact marks, and kept with `cg_marks 0`. Since 03/11/2026 (Sol: tags glitched on
odd surfaces) a tag keeps off surfaces it cannot sit on as a flat picture
([`tag_unfit`](../crates/sjk-viewer/src/decal_marks.rs)): moving ones (`deformVertexes`,
flags, water waves), see-through ones (a first stage blended or `alphaFunc` cut: glass,
force fields, grates, leaves, or sorted after the opaque), decals laid over a wall
(`polygonOffset`: the wall under takes the tag once instead of twice) and sky or fog. It
also projects with buffers of its own, 512 fragments over at most 128 surfaces (an impact
mark keeps the reference's 128 over 64), so a curved patch or a face cut in many triangles
no longer cuts the picture short (it was 48). Two seams on one flat wall no longer cut a
tag in half either: tags (not marks) go on models the compiler merged into the world, with
the curves' facing test, and a flat face stored without a normal (no lightmap) is judged by
its vertices' normal instead of being skipped (`sjk-bsp` `mark_surfaces`, marks too). Since 31/10/2026 (Sol: tags were blurry in
the game) tags are drawn by a pass of their own
([tag_render.rs](../crates/sjk-viewer/src/tag_render.rs), [rendering.md](rendering.md#tags)),
each picture at its full size (a tag enlarged to 512 pixels, a card's sticker 1024), not as
a 128-pixel tile of the effects' atlas, with its finish shaded per pixel: Holo a rainbow foil
whose light follows the viewer's angle, with a glitter, and Solar the Sun's corona burning
round the picture's own outline. The pictures (`gfx/tags/<id>.png`) are art in the hub's
`sjk_tags` pack, the first ones placeholders until Sol's art (256 pixels: the pass takes
any square up to 1024, so a sharper cut shows sharper); a card's is its picture in the card
packs. The toy's spray-can icon is bundled, made by
[tag_assets.py](../scripts/tag_assets.py). The pictures are put into the UI atlas from the
hub's packs for the Collection and the holocron opening
([tag_icons.rs](../crates/sjk-viewer/src/tag_icons.rs)), 128 pixels a side; a card's shows
from the card faces' textures. See [tags.rs](../crates/sjk-viewer/src/tags.rs),
[tags_frame.rs](../crates/sjk-viewer/src/tags_frame.rs) and
[collection_tags.rs](../crates/sjk-viewer/src/collection_tags.rs).

## Fishing

Fishing (Sanji's idea in #ideas, 29/10/2026; Slash's rules) is a toy for every SJK player: a rod in the hand, a
bucket at the feet and a float on the water, and a bite to hook now and then. It gives
nothing the economy counts: catches go to the player's own fish log, and the Echoes it
brings up are the ones the hub drops anyway.

- **`toy_fishing`** (a bind, the quick wheel's Toys page, Collection > Toys, "Rod out")
  gets the rod out or puts it away. The player must stand still: moving, getting hurt,
  dying or leaving the game puts it away. The hands go empty (`weapmelee`), and the upper
  body holds the rod in the rifles' ready pose (`TORSO_WEAPONREADY3`) while the legs stand.
  In first person the own rod is held in view, low on the right, and the one in the hand
  shows only in mirrors.
- **Where it casts:** into water ahead (72, 56 or 40 units in front, the first that has
  it), else onto ground ahead no more than 48 below the feet, where a small pond is drawn
  for the cast: the toy fishes anywhere, since few maps have water. The pond is drawn
  with the installed game's own swamp textures (Hidden Evil's lake layers over the stock
  water, Yavin's mud, mossy rock and ground plants), so the game must be installed.
- **Bites** come every 20 to 60 seconds: the float dips, a splash plays, and `+use` within
  0.8 to 1.2 seconds hooks it. Too soon scares the fish (the next bite waits 10 to 20
  seconds more); too late lets it go. A hooked bite comes up empty 15% of the time, with
  junk 30% (six kinds) and a fish 55% (twelve kinds, from the Silver darter to Old
  Whiskers, the great catfish, one fish in about 106), reeled in to the tip hanging by
  its mouth, and said in the centre of the screen. Each catch is counted in
  `fishing_log.txt` in the settings folder (`<id> <count>` a line, and `last <id>`).
- **The fish** each have their own model and painted skin (`fishing_fish_<id>.md3`, one
  fish atlas): the slim Silver darter, the deep spiny Mud snapper, the spotted
  big-mouthed Swamp gulper, the barred Striped bonefish, the Glow perch with its glowing
  spots, the long Spiny pike with spines down its back, the scaled golden Sun carp, the
  Moon eel, the Kyber minnow with crystals on its back, the flat Ghost ray, the red and
  white Temple koi and Old Whiskers with its long whiskers. The **bucket** holds the last
  fish caught (kept in the log, so it is there the next time the rod comes out): a short
  one floats on its side on the water, a longer one stands head down with its tail out
  over the rim. Other players' buckets are empty, since their catches are not sent.
- **Echoes:** when the hub drops the player an Echo while they fish, its card waits, and
  a golden bite comes 5 to 15 seconds later (1.5 seconds to hook it, two tries); hooked,
  the holocron comes up on the line and its card shows. Missed twice, or with the rod put
  away, the card shows all the same. Fishing never brings an Echo the hub did not drop:
  the caps stay as they are ([holocrons.md](holocrons.md)).
- **Achievements:** First Bite (a fish), Angler (50 fish), Every Fish in the Pond (every kind), Echo Angler (10
  Echoes on the line) and Old Whiskers (hidden), counted on servers only, as the others
  ([identity.md](identity.md#achievements)).
- **Sounds:** Settings > Sound > Toys, "Fishing" (`s_toyFishingSound`).
- **The fish log on the card** (03/11/2026): the Fishing card in Collection > Toys says
  how many fish were caught and of how many kinds, and in one line the junk held ("Junk:
  7 pieces, worth 12 credits (8 left today)"). `fishing_log` in the console prints the
  log in full: every fish and junk with its count, the junk held and what was sold.
- **Selling junk** (03/11/2026, Sol's yes on PR #278; the hub's `PROTOCOL.md` "Fishing junk",
  `sjk_identity::junk`): the card's "Sell junk (+N credits)" button, and `fishing_sell`
  in the console, sell the junk held to the hub. The hub pays a few credits a piece (1
  to 5) up to 15 credits in 24 hours per person and per address, the most valuable
  pieces first, and answers what it took: only that is counted sold, as `sold <id>
  <count>` lines in `fishing_log.txt` (older clients skip them), so the counts of what
  was caught, and the achievements, never change. The card asks the hub what it still
  pays (`GET /v1/fishing/junk`) when shown and every 30 seconds after; the button shows
  only while the hub says this player may sell (the operator's switch, off until Sol
  turns it on) and is greyed with nothing payable or no junk. One sale at a time
  (`fishing_sell.rs`). Fish never sell: they stay in the log for the achievements.
- **Bite cue** (03/11/2026, for players with the sound off): Settings > Sound > Toys,
  "Fishing bite cue" (`cg_fishingBiteCue`: 0 off, 1 when the sound is off, the default,
  2 always). While shown, a bite puts a pulsing "!" over the float (gold for an Echo's
  bite), drawn as the mystery boxes' words are, and "Bite! Press Use!" (an Echo:
  "Something shines under the float!") on the centre of the screen; a fish's bite let
  go says "It got away.". "The sound is off" means the Fishing sounds switched off, the
  toys' or the game's effects volume at 0, or the game muted while away
  (`GpuState::fishing_cue_shown`); a sound muted by name (`s_muteSounds`) or no sound
  device does not count, so set 2 there.

No game server knows of it. The rod is a toy in the player's look (`toys: ["fishing"]`),
so other SJK players draw it in the fisher's hand, with the line to a float where their
own client finds water or ground ahead of that player; players on stock clients see a
player standing still. The models, picture, shader and icon are bundled, made by
[fishing_assets.py](../scripts/fishing_assets.py) from shapes (nothing taken from another
game), and mounted below all game data. See
[fishing.rs](../crates/sjk-viewer/src/fishing.rs).

## Toy sounds

Settings > Sound > Toys (Sol's request, 25/10/2026) sets how the toys sound on this PC:
"Toy volume" (`s_toyVolume`, 0 to 2, default 1, times the effects volume) scales every
toy's sounds, and a switch per toy silences that toy's sounds alone, the player's own
and everyone else's (`s_toyBoxSound`, `s_toyOrder66Sound`, `s_toyHaxSound`,
`s_toyRagdollSound`, `s_toyDroneSound`, `s_toyBoomboxSound`, `s_toyMonitorSound`,
`s_toyMysteryBoxSound` (the [mystery boxes](mystery-boxes.md)), `s_toyBananaSound` (the
[Banana peel](#banana-peel), 29/10/2026), `s_toyTagSound` (the [Tag](#tag)'s spray,
03/11/2026), `s_toyMasterSound` (the toys [the Toy
Master](toy-master.md) hides, 30/10/2026), `s_toyFishingSound`
(the [Fishing](#fishing) toy), all
on by default,
archived). The toy still
runs and is still seen; nothing is sent to anyone. The Illuminate holocron makes no
sound. Every toy sound goes through `GameAudio::play_toy`, and the box music and the
Hax loop through `send_toy_loop`, which read the settings each frame, so a change is
heard at once, loops included. See [toy_sounds.rs](../crates/sjk-viewer/src/toy_sounds.rs).

## Holstered saber hilts

Settings > Game offers **Holstered saber hilts**, **Holster server support**,
**Staff handoff point** and **Staff handoff sounds** in both SJK and classic
menus. `cg_drawHolsteredSaber` defaults on. Support (`cg_holsteredSaberSupport`)
defaults to `1`, automatic JA+ detection; `0` disables support and `2` forces
the documented JA+ ownership/plugin contract for a compatible server. Forcing
it on an unrelated server can misinterpret that server's ownership flags.

Stowed single hilts sit at the right hip; dual hilts use both hips. A single
staff goes across the back when JA+ plugin 9 is enabled, with a hip fallback
for models without a back tag. `cp_pluginDisable` is a disable mask: bit 9
(`512`) disables the staff back reach, and bit 29 (`536870912`, EJK's
`JAPRO_PLUGIN_HOLSTEREDSABERS`; bit 13 until 29/10/2026, which is JA+'s single-player
cartwheel) disables holstered hilts altogether. Settings > Mods switches both as Staff
on the back and Holstered sabers ([plugin options](mods.md#plugin-options)). SJK
preserves its existing default mask `1536`, so the default staff placement remains
at the hip. This feature does not enable
JA+ animation plugins or install their animation assets.

With plugin 9 enabled and usable JA+ reach animations installed, the staff
changes between back and hand at fraction `cg_holsteredStaffSwap` (default
`0.45`) of the actual torso frame range. It changes once per playing, including
the reverse put-away. Blades stay off before the drawing hand reaches the
hilt and retract during put-away. `cg_holsteredStaffSound` (default on) aligns
the registered hilt and blade-skin ignition/shutdown sounds with that reach;
missing or interrupted reaches have a bounded ignition fallback. Without the
reach clip, the normal weapon change controls the attachment.

Holsters are hidden on dead, disintegrating, mounted, mind-tricked or undrawn
players and on a first-person player's own/followed body. A cloaked player's
hilts require the viewer's Force Sight; your own cloaked hilts stay hidden.
A thrown primary hilt is never duplicated on the belt. Missing attachment
points affect their own hilt independently. The reference's Jedi NPC class
list is supported too.

Console placement overrides are archived, case-insensitive three-number
vectors, with finite values bounded to +/-3600; invalid vectors use defaults:

| Cvar | Default | Meaning |
| --- | --- | --- |
| `cg_holsteredSaberPos` | `0 3.5 0` | Hip offset: bolt right, forward, up; opposite side for the second hilt |
| `cg_holsteredSaberAng1` | `0 0 270` | Primary hip pitch, yaw, roll |
| `cg_holsteredSaberAng2` | `0 0 270` | Secondary hip pitch, yaw, roll |
| `cg_holsteredStaffPos` | `0 0 0` | Back offset: bolt forward, right, up |
| `cg_holsteredStaffAng` | `0 0 150` | Back pitch, yaw, roll |

These are client presentation options; they change no gameplay, wire format
or animation timing. See [rendering](rendering.md#holstered-saber-hilts) for
attachment ownership and [status](status.md#holstered-saber-hilts) for limits.

## Blade skins

A blade skin is an unlockable saber look ([unlockables.md](unlockables.md)): the Sun
(`saber_sun`), Storm (`saber_storm`), Void (`saber_void`), Frost (`saber_frost`) and
Prism (`saber_prism`) blades. Its look and sounds are a blade-skin file and
sound files in a pack the SJK hub delivers ([unlockables.md](unlockables.md#packs)),
drawn by the generic renderer described in
[rendering.md](rendering.md#saber-blade-skins); with no pack the skin is the stock
blade. `cg_saberSkin` (archived, default
empty) holds the skin the player wears by unlock id; empty or an unknown id is the
stock blade. It shows only while the player's own hub profile lists that unlock
(`Looks::own_saber_skin`, read twice a second) and its pack is loaded: with the
identity off, no hub, no answer yet, the unlock missing or no pack, the stock blade
shows, in the hand, in first person, thrown and on the Character page. Other players'
skins are their looks from the hub ([unlockables.md](unlockables.md#receiving)), copied
into the per-client table (`GpuState::sync_saber_skins`) only when the looks or the
loaded skins change.

`saberskin` lists the blade skins, owned (since when) or locked (how to get it), and
which is worn; `saberskin <id>` or `saberskin none` sets `cg_saberSkin` (a locked one
is kept and shows once unlocked). `unlockables` opens the Collection on its Shaders
tab (also the Profile page's See the collection), where owned skins are equipped and
unequipped and any is previewed on the player's own saber
([sjk-ui.md](sjk-ui.md#collection)).

A player may choose other saber sounds than their skin's (`cg_saberSounds`, Profile >
Saber > Sounds: follow the shader, the stock sounds, or any owned shader's; every SJK
player hears the choice, [unlockables.md](unlockables.md#saber-sounds)); what follows
holds for the set they are heard with. A skin brings its own sounds, heard for each player wearing it, over the stock ones,
which keep playing (the skin's sound goes on a channel of its own, so that it does not
cut the stock one), or, with `s_saberSkinSounds 2` (Settings > Sound > Blade skin
sounds: Only skin; 1, With normal, is the default), in their place, or, with
`s_saberSkinSounds 0` (Off), not at all, every player heard with the stock sounds
(the table of who wears which set is passed empty), so a skin's own sounds are heard clearly
(Creyon's request, 18/10/2026; a thrown saber hums with the skin's alone too since
03/11/2026, #233): its ignition for
`EV_SABER_UNHOLSTER`, its ignition or switching off once when the player draws the
saber from another weapon or puts a lit one away
([saber ignition sounds](#saber-ignition-and-retraction-sounds)), and for a general sound that is a stock `saberon*`/`saberoff*`
or `enemy_saber_on`/`enemy_saber_off`, or any `.sab` `soundOn`/`soundOff` (the game's `G_Sound` gives these no owner, so
the player whose origin is nearest the sound, within 64 units, is taken; the toggle
sounds every hilt, a single saber's removed second slot too, and the skin's sound
plays once over them all, or with Only skin is the whole toggle), its hum
once beside the sabers' hum loops, and three swings for `EV_SABER_ATTACK` and the
animation `saberhup` cues, over a hilt's own `swingSound`s
([saber_sound_overrides.rs](../crates/sjk-client/src/saber_sound_overrides.rs)).
The viewer registers the skins' sounds with the gamestate's sound tables and passes
the per-client table every frame. With no skin worn every path is the stock one. A
thrown saber hums its owner's skin too: over the flying saber entity's `loopSound` (the
stock hum or the hilt's `soundLoop`), while the owner it names
(`genericenemyindex`) wears one. A skin's sounds are the game paths its file names, read from its pack below all game
data (`VirtualFileSystem::with_lower`), so a PK3 with the same paths replaces them. They
are registered when the gamestate's tables are built and again, at once, when a pack
arrives mid-session (`GameAudio::follow_blade_skins`, which also decodes a skin's
sounds afresh in case the new pack changed them); the per-client table then names the
new sets. The Sun's sounds have not been checked by ear in a game.

### Saber Lab

The Saber Lab ([saber_lab.rs](../crates/sjk-viewer/src/saber_lab.rs), 18/10/2026) makes and
tunes blade skins from a folder of loose files, reloaded as they are saved: no pack, no
restart, no rebuild. `saberlab <folder>` starts it (a path may hold spaces), `saberlab off`
stops it and `saberlab` alone says what it reads; `SJK_SABER_LAB=<folder>` starts it at
launch, for a shortcut.

- **The folder** is laid out as a pack: `skins/blades/<id>.bladeskin`, its additions files
  ([unlockables.md](unlockables.md#blade-skin-files)), and the images and sounds they name.
  It is mounted over the hub's packs (`sjk_packs::mount`), so a file there replaces a
  pack's of the same name: `saber_sun.bladeskin` tunes a copy of the Sun. A file named
  `saber_lab.bladeskin` is the **lab blade** (`saber_lab::BLADE`), a skin of its own the
  catalogue knows only while the lab is on, for a new skin with no catalogue entry yet.
- **Reloading:** the folder is looked at every half second (names, sizes and modification
  times of at most 512 files); a change mounts the packs again, the renderer takes the new
  skins on its next frame and their sounds are registered again, as when the hub sends a
  pack. A reload takes about 20 ms. A file saved with a UTF-8 byte-order mark (Notepad,
  PowerShell) reads as one without.
- **Mistakes:** a file that does not read, or whose images or sounds do not load, keeps its
  skin's last good look (`LoadedSkins::keep`), and the Shaders tab's lead says which file
  and why (`corona.reach must be 1 to 2, not 9`), as does the console. The lead otherwise
  names the folder and how long ago it was read.
- **The preview:** starting the lab opens the Collection's Shaders tab (the SJK UI's
  Collection screen, or the classic menus' page) with the lab blade chosen, the row after
  the stock blade whatever the filter, on the lone saber the tab turns
  ([sjk-ui.md](sjk-ui.md#collection)). Any skin of the folder can be chosen there.
- **The Saber Lab tab** (18/10/2026, [collection_saber_lab.rs](../crates/sjk-viewer/src/collection_saber_lab.rs)):
  while the lab is on the Collection's row ends with it. On the menu map's stage the
  player's model stands left of the picture (`menu_backdrop::Shot::SaberLab`; duel6 has
  a route of its own, other maps take the player's shot) holding the blade chosen, in
  the style chosen (Fast, Medium, Strong: `BOTH_SABERFAST_STANCE`, `BOTH_STAND2`,
  `BOTH_SABERSLOW_STANCE`), standing or swinging: each style's three attacks chained as
  the game chains them, the start from the stance, the attack and the return
  (`force_pose::STYLE_ATTACKS`, `BOTH_S1_S1_T_` / `BOTH_A1_T__B_` / `BOTH_R1_B__S1` and so
  on). A copy of the same saber floats on the right (`LAB_SABER_AREA`, four fifths of
  its width long at most), turned by a drag as on Shaders, its hilt in a buffer of its
  own (`StageSaber::float_instance_buffer`). Under it: Style and Moves as segments, the
  blade with arrows among the folder's skins, Equip, and the lab's status in the lead.
  Keys: 1 to 3 the style, Space stand or swing, Left and Right the blade, Enter equip.
  Over a match the tab shows no stage and says so; the stage is the main menu's only.
- **Hilts, speed, ignition, sound and camera** (18/10/2026): the model holds the player's
  own hilts (`saber1`, `saber2`, read as the Collection opens), as the Shaders tab's lone
  saber does; a staff or two sabers swing in their own style (`force_pose::lab_swings`,
  the `BOTH_A7_*` and `BOTH_A6_*` attacks) and the Style row says so. A Speed slider
  (10% to 200%, `-` and `=`) plays the animations and the moves' script at that pace
  (`actor.rate`, `MenuStage::motion_clock`), each swing following the last the moment
  it ends, with no wait in the stance between at any speed (the user, 18/10/2026:
  `force_motion::moves_at`'s `pause` 0 in the lab; the Force page's previews keep
  theirs). Saber Off and On (`I`) retract and light
  both sabers a frame's growth at a time, as a match does (`saber::Extension` per blade,
  `StageSaber::extension`); with Flourish ticked (`F`, on at first) the model plays the
  game's draw or put-away as it does (`LS_DRAW`, `LS_PUTAWAY`: `BOTH_STAND1TO2` and
  `BOTH_STAND2TO1`, a staff's `BOTH_S1_S7` and `BOTH_S7_S1`, two sabers' `BOTH_S1_S6`
  and `BOTH_S6_S1`; `force_pose::lab_flourish`), the swings waiting for it, and stands
  holstered (`BOTH_STAND1`) while the blade is off. Sound On and Muted (`M`): the
  ignition and retraction (the skin's own, else the hilt's `soundOn`/`soundOff`, else
  `saberon.mp3` and `enemy_saber_off.wav`; a file no pack holds is passed over for the
  next), a swing as each attack begins as a match plays `EV_SABER_ATTACK` (the hilt's
  `swingSound*`, else `saberhup1` to `saberhup8`, the skin's swing on top), and what the
  model's `animevents.cfg` cues (the spins and specials' swings), none of the swings while
  the blade is off (`menu_stage/lab.rs`). The ordinary attacks have no swing in
  `animevents.cfg`: listening for those alone, the tab was silent at first. The hum does
  not play yet. A drag on the picture left of the choices turns the camera round the
  model and tilts it over it, the wheel brings it nearer (45% to 220% of the shot's
  distance), `R` puts it back (`LabView`, `menu_backdrop::orbit`): the whole view turns
  about the model's chest, so the model stays where the shot frames it, and the camera
  stops short of a wall between. View Third person (`T`) puts the camera behind the
  model as a match's third person does (`cg_thirdPersonRange`, `cg_thirdPersonVertOffset`
  and `cg_thirdPersonAngle` over the eye, `DEFAULT_VIEWHEIGHT`, looking level;
  `menu_backdrop::behind`), the drag and the wheel turning it round that point. Both ease
  in over 0.6 s and out as the tab is left (`menu_stage::lab::Ease`, `LabCamera`).
- **Trails and the blade shown** (18/10/2026): the stage's blades leave a match's motion
  trail as the model swings (`StageSaber::trails`, fed to the match's
  `saber_trail::SegmentPool`; `cg_saberTrail 0` turns it off). How long it lasts is the
  game's `saberMoveData` trail length of the move whose animation the torso plays, found
  in the move table with the style's animation group (`sabers::trail_length`: starts,
  attacks and returns 200 ms, transitions and parries 150, stances, the draw and the
  put-away none), before `CG_AddSaberBlade` divides it by five. Trail, ticked over the
  floating saber (`L`), gives it a still trail: an attack's trail held as a blade swinging
  evenly leaves it, eight slices over 0.45 rad about the camera's forward through its
  grip, each as old as its place (`saber_trail::still_trail`, drawn that frame only,
  `SegmentPool::add_still`). The blade on the model and the floating one is the one worn
  (`cg_saberSkin`, whatever Shaders equipped), so the lab's own shows once the tab's
  Equip wears it. The Blade row (Left and Right) starts there, then previews the stock
  blade, the lab folder's skins and every other blade skin of the catalogue, the rarest
  first, owned or not (`LabBlade`, `lab_blades`; drawn on this PC only, as Shaders
  previews a locked one). Equip wears what may be worn (a lab skin, or one the profile
  lists) and takes off what is worn; a blade not owned is only previewed, and the line
  under the row says which.
- **The blade-skin editor** (18/10/2026, [blade_editor.rs](../crates/sjk-viewer/src/blade_editor.rs),
  [collection_saber_lab_editor.rs](../crates/sjk-viewer/src/collection_saber_lab_editor.rs)):
  Edit (`E`) on the Equip row opens the skin the Blade row shows (a skin of the packs is
  copied into the lab's folder first, "Copy to edit": the lab then draws its copy in place
  of the pack's, on this PC only) as its six files, the skin file and its five additions
  files, read in their own order ([ordered_json.rs](../crates/sjk-viewer/src/ordered_json.rs)).
  Its sections and fields are rows in the column under the floating saber: a number with
  a slider over the range its check allows (`blade_skin_file::ranges` records each
  `within` the checks make, over every skin of the packs and the one edited), a colour
  with a swatch and its channels, a switch, a path; a section of another file shows that
  file. Each section says in a few words what it draws (`blade_editor::about`: "black
  holes", "lightning arcs"; only words, and a test fails when the format has a section
  they do not word). There is one shader (`saber.wgsl`) and one format: a section is a
  part of that shader a skin switches on by holding it, so the editor covers the format,
  read from the format itself ([blade_skin_shape.rs](../crates/sjk-viewer/src/blade_skin_shape.rs),
  no list of its own; the user, 18/10/2026: "if there is a rule to the system then
  that's what I want the editor to cover"). A skin read with a probe member in it is
  refused with the members the format allows there, in its order; a value of the wrong
  type with the type wanted (a struct, an array of a length, a sequence, an integer, a
  string, a switch); a value out of its range with the range. The sections the skin
  lacks are listed to add under a heading ("Add a section", `A` goes there), in the
  format's order; one is added as the first shipped skin holding it has it (in that
  skin's file) or, when none does, as the format makes it: its fields of their types,
  built from those words, and its values moved into their ranges until it reads, in the
  newest additions file. Built so, every section reads on its own but for those a rule
  binds beyond its ranges (the image paths, `motes.size × stretch`, `orbit` needing
  `star` or `holes`, an option's id), whose words the editor then shows. Up and Down choose a row, Left and Right move a number
  by its step (a hundredth of its range, 1, 2 or 5 times a power of ten; Shift ten,
  Ctrl a tenth) or open and close a section, Shift+Right and Shift+Left every section, a
  drag on a slider sets it (a number without a range is scrubbed), Enter opens, flips,
  adds or types (a digit types a number at once), Delete takes a section or a list's
  item out, Insert copies an item, Ctrl+Z and Ctrl+Y undo and redo (200 changes, a drag
  as one), Ctrl+S writes at once, Esc or Done closes. Each change is laid over and
  checked as the game reads the files; when it reads it replaces the skin's look at once
  (`sjk_packs::edit`, `LoadedSkins::edited`: the one skin, its images made again only
  when their profile or image changed, uploaded alone, `SaberGpu::upload_skins`'s
  `only`), the model, the floating saber and the trail showing it the next frame; when
  it does not, the check's words show and the last good look stays. The files are written
  0.7 s after the last change (an additions file left empty is taken away) and the lab
  is told (`saber_lab::wrote`), so it does not mount the packs again for them; a file
  saved by another editor meanwhile is read again when nothing waits.
- **Smoke from the emitter** (18/10/2026): the format's eighth set, `smoke` in
  `<id>.bladeextra6` ([unlockables.md](unlockables.md#blade-skin-files)), drawn as the
  game's particles let go where each blade's emitter is (`saber_smoke::Emitter`, kept per
  blade in a match's `BladeState` and the stage's `StageSaber`, the floating copy's
  apart): while lit and, put out, for its `after` seconds, the puffs rising and fading in
  the air so a swing trails them. At first the puffs were made but not drawn: the effect
  atlas holds only the map's effects' shaders, so a skin's smoke shader is now added to it
  (`GpuState::admit_smoke_shaders`). Wisps (the user asked for wispy white smoke): many
  small faint puffs a second chain into a thin strand, and the stream's sway (`sway`,
  `sway_rate`) curls it; the lab's skin lets 70 a second go of the stock
  `gfx/effects/jk_smoke5`, white, 1 to 4.5 units, opacity 0.3. The stock soft
  `alpha_smoke2` barely shows at all, and the game's wisp `smokeTrail` is made for
  trails: additive, it summed to white blobs, and its scrolling smeared in the atlas.
  Then a ribbon renderer (the user, 18/10/2026: "an actual ribbon renderer for wispy
  smoke"): `ribbon`, beside `smoke` in the same file, strands through points let go one
  after another, each drifting on its own, drawn by `saber_ribbons::Gpu` with the blades'
  pass (after them) and counted in the effect layer's bounds. The lab's skin now smokes
  with a white ribbon alone: standing, for 1.5 s once lit and 3.5 s once put out, but not
  swinging (`idle` 1, `swinging` 0; the user, 18/10/2026: trails "too distracting while
  swinging", with the choice of smoking the whole time, `swinging` 1). Then sparks
  (`sparks`, the same file; the user: "sparks flying out of the emitter a bit upwards,
  crackling"): 2.5 bursts a second lit, 14 for 1.2 s once lit, 8 falling to none over
  0.8 s once put out, up to five sparks each, 75 units a second in a 35° cone, falling at
  420, 0.3 wide streaking 5 units; at first 0.7 wide, they read as fat ovals. `aim` turns
  the cone from the world's up (0) to along the blade (1; the lab's skin), the user asking
  for sparks shot out along the blade. More lab blades: a folder's
  `saber_lab_<name>.bladeskin` is the lab blade "Saber Lab: <name>", worn on this PC as
  the lab blade is (`saber_lab::EXTRA_PREFIX`); the user's first blade is kept so as
  `saber_lab_half_broken`. Space, I, M, T, F
  and L still swing, light, mute, turn the camera, flourish and trail while it is open.
- **The builder** (30/10/2026, the user's design: "a very visual way of working with"
  the effects; [blade_builder.rs](../crates/sjk-viewer/src/blade_builder.rs),
  [collection_saber_lab_builder.rs](../crates/sjk-viewer/src/collection_saber_lab_builder.rs)):
  the tab's first view; Stage controls (`B`) shows the choices above, Builder comes back.
  The camera turns until the model stands right of the panels (`LabStage::build`,
  `menu_backdrop::framed_yaw`: the model's chest at 78% of the frame across, from the
  menu's field of view, eased as the other views are) and no saber floats. On the left
  the library: every effect a skin file can draw (`blade_builder::EFFECTS`, 48: the 39
  sections an option may switch, the orbit, and the blade's own make), by kind on five
  tabs (Around, On the blade, Emitter, Figures, Core; the Kills tab went with the burn,
  now a finisher, 04/11/2026, [unlockables.md](unlockables.md#finishers)), a card each with its name
  and a few words, or where it is on the blade; the pointer resting on a card or a chip
  for 0.35 s shows its card (`hover::draw`) with what it draws and what a click or a drag
  does. In the middle the blade's groups (`blade_builder::Shape`): The blade itself (the
  sections every skin has; never grouped, never taken off), Always on (the effects in
  no option), each group (an option of the file: its name, a click renames it, Enter
  keeps the name; its switch, the wearer's own choice as on Shaders; `×` ungroups it,
  its effects always on) and room for a new one while there are fewer than six. An
  effect is dragged (6 frame pixels from where it was pressed) from the library onto the
  blade, between Always on and the groups (at most four in one), onto New group (named
  after it), or back to the library to take it off; Delete takes off the one the pointer
  rests on; a click on a library card puts it on Always on, a click on a chip opens its
  fields where the groups were (the editor's rows, that section alone open; Done or Esc
  comes back). Each change goes through the editor (`Editor::add_section`,
  `remove_section`, `set_options`), is checked as the game reads the files, and is
  refused with the check's words when it would not read (an orbit with nothing to
  circle, a glint with black holes); where it may not go is said under the pointer while
  dragging. `options` is written in the newest file a client must read to know every
  section it names, at least `.bladeextra` (`write_options`), and taken out of the
  others, so an older client reads no option it could not check. An orbit goes with its
  glint or holes, shown after them. A blade of the packs is copied into the lab's folder
  first (Copy into the lab); a signature's or a hand-ported design's is not built, its
  shader drawing most of it. Tried by the user on 30/10/2026; three blades they edited
  had to be put back by hand, so (the user, 01/11/2026: "when you start editing a blade,
  it keeps a copy of the original so you can always revert it") the editor, before it
  first writes a blade's files, copies them as they are into the lab folder's
  `originals/<id>/` (`blade_editor::ORIGINALS`, outside `skins/blades` so they are not
  read as skins; an additions file goes before the skin file, so a copy cut short is
  made again, and nothing is written if the copy fails). That copy is never written
  over: the first edit's original stays until the folder is deleted. Revert, on the
  builder's status line and the fields' column, puts it back as a change (Undo takes it
  back), and a file back as it was is copied from the kept one, byte for byte.
- **Wearing:** while the lab is on, the lab blade and every skin the folder holds a file
  for count as yours on this PC (`saber_lab::owns`): Equip wears them, in the hand, in a
  match and on the stage, and the card says "Yours in the Saber Lab, on this PC only".
  Only the blade drawn here follows it: the look sent to the hub keeps your real unlocks
  (`looks_frame.rs`, as debug collectibles do), so the hub never hears of a lab skin and
  other players see your usual blade.

Checked in the client on Windows: the lab started from `SJK_SABER_LAB`, a save reloaded
within half a second, an out-of-range value was named and the last look kept, and the
fixed file read again.

## Quick wheels

SJK's quick wheel ([quick_wheel.rs](../crates/sjk-viewer/src/quick_wheel.rs)):
hold the key, a ring of choices opens in the middle of the screen, move the mouse
towards one and let go to run it. The wheel has pages (Force, General, Toys and
Weather unless the player changed them); while it is open, the mouse wheel and buttons change
page. Letting go with the mouse still near the middle, or Escape, runs nothing;
opening the console, chat or a menu closes the wheel. While it is open the mouse
moves its pointer instead of the view and the HUD steps aside as `cg_drawHud 0`
would have it (status, weapon, timers, vote and kill lines, crosshair and the
game-data HUD; chat and nameplates stay, `QuickWheel::hides_hud`); the movement
keys keep working. A choice is the console command it stands for, so it does exactly
what typing it would.

### Binds and pages

| Bind | Opens |
| --- | --- |
| `+wheel`, Q by default | On the page shown last in this run (the first page, Force by default, in a new run) |
| `+wheel weather`, R by default | On Weather |
| `+wheel <page>` | On that page, named by its id or its name (case and spaces ignored), e.g. `+wheel general` or `+wheel toys` |

Q was `+wheel general` until 08/10/2026; a profile that saved that default has
it moved once to `+wheel` (`cl_wheelBindVersion`), and a page bound on Q after
that, or on another key, stays.

Q and R are in Settings > Key bindings > Other (Quick wheel, and Quick wheel,
Weather); their defaults are bound only where a profile has nothing on Q or R. A
page keeps its id (`force`, `general`, `toys`, `weather`, or what a new page's
first name made of it) when it is renamed, so a bind keeps working; a bind naming a page since
removed opens the page shown last. A second wheel key pressed while the wheel is
open changes it to that key's page.

While the wheel is open:

- **Scroll down or right click:** the next page; **scroll up or left click:** the
  previous one, wrapping round. Right is on and left is back, as the next and
  previous pages' names stand right and left of the ring; down is on, as down a
  list. A trackpad's small scroll steps add up to a notch (40 counts).
- The pointer stays where it is, so the same move picks on the new page; the
  new page turns in (0.17 s), its choices from a little way round the ring.
- The scroll and the left and right buttons do nothing else: no weapon change, no
  attack. Their releases are the wheel's too, while an attack held before the
  wheel opened is released as usual.

### Force page

The Force page (Sol's request, 10/10/2026;
[force_page.rs](../crates/sjk-viewer/src/quick_wheel/force_page.rs)) is a default
page, and since 10/10/2026 (Sol: "make the force wheel the first wheel to appear by
default") the first one, so a new run's bare `+wheel` opens on it. It is live: as
the wheel opens it holds the Force powers the player can use right now, read from
the latest snapshot as the [Force bar](#force-wheel) reads them
(`forcePowersKnown`, set for a power with a level above 0), with JoF JA+'s Dash,
Stasis and Repulse where the server grants them. Jump and the saber powers are
passive and left out, and so is [Illuminate](#illuminate), which is not a Force
power: it is on the [Toys page](#toys-page). Each choice shows the Force bar's picture and
name (Stasis borrows Jump's, merc mode's Lightning is the Flamethrower), and the
power selected now wears the gold dot.

| Power | Choosing it runs |
| --- | --- |
| Push, Pull, Speed, Sense | `forceselect <n>; force_throw` (`force_pull`, `force_speed`, `force_seeing`) |
| Heal, Protect, Absorb, Mind Trick, Team Heal | `forceselect <n>; force_heal` (`force_protect`, `force_absorb`, `force_distract`, `force_healother`) |
| Dark Rage, Team Energize | `forceselect <n>; force_rage` (`force_forcepowerother`) |
| Grip, Lightning, Drain, Stasis | `forceselect <n>` only |
| Dash, Repulse | `forceselect <n>; force_dash` (`force_repulse`) |

An instant power is used at once by its own command (the `genCmds_t` usercmd
values retail's F1-F12 binds send, JoF's server commands, SJK's toggle) and
selected, as the Force bar would leave it, so the Use Force key uses it next. A
held power cannot be held from a wheel that runs on letting go, so choosing Grip,
Lightning, Drain or Stasis only selects it for `+useforce`.

The order is fixed, so a power keeps its place round the ring while the build
does: the neutral powers (Push, Pull, Speed, Sense), the light side's (Heal,
Protect, Absorb, Mind Trick, Team Heal) or the dark side's (Grip, Lightning, Dark
Rage, Drain, Team Energize), each in the retail default keys' order (F1 to F12),
then Dash, Stasis and Repulse. A usual build is 9 powers; the page holds 12, its
pictures drawn at ten-twelfths of their size so the gap between them stays. Past
12 (an admin's every-power build on JA+, up to 17) the rest continue on a second page right after it, "Force 2", reached by the
same scroll or click; a bare `+wheel` that last showed Force 2 opens on Force when
the second page is gone. Shrinking 17 onto one ring would make every picture
half-size for a build almost nobody has, and an inner ring would put two choices
in one direction from the middle, which the wheel's pointer cannot tell apart.

Out of a game (or in a demo, where the choices could not act), spectating or
following, and with no usable power, the page stays in its place and says "No
Force powers here", so the pages keep their order and their dots.

### Toys page

The Toys page (Sol's request, 10/10/2026: "make a toy wheel too for the force
illuminate and future toy") is a default page named Toys, `+wheel toys`, for
[Illuminate](#illuminate) and future toys. It is an ordinary, editable page, not a
live one: its choices come from the [catalogue](#choices)'s Toys group, one entry
per toy, so a toy deleted from the page is added again in Settings > Quick wheel
(Add a choice, Toys), and a future toy is one catalogue entry. Its default choice
is Illuminate, which runs `toy_illuminate` (the toggle the bind uses) with the
Illuminate holocron's picture. Its gold dot marks a lit holocron (the client's own
state; no cvar holds it). Since 16/10/2026 the default page also has the
[Cardboard box](#cardboard-box) (`toy_box`, marked while the player is in it), and
since 17/10/2026 [Order 66](#order-66) (`toy_order66`, marked while it is on, then
Execute Order 66, `order66`), since 21/10/2026 the [Hax](#hax) (`toy_hax`, marked
while it is on), since 23/10/2026 the [Ragdoll](#ragdoll) (`toy_ragdoll`, marked
while the player is limp), and since 26/10/2026 the [Monitor](#monitor) (`toy_monitor`,
marked while it is on, then Throw monitor, `monitor_throw`); a wheel saved before keeps
its Toys page as it was, and gains them from Add a choice, Toys. The page holds ten, so
later toys are in the catalogue's Toys group only, to add from there: Box hunt
(`toy_boxhunt`), the Tag (`toy_tag`) and since 29/10/2026 the
[Banana peel](#banana-peel) (Banana peel, `toy_banana`, marked while it is on, and Drop
banana peel, `banana_drop`) and [Fishing](#fishing) (`toy_fishing`, marked while the rod
is out).

### GIF wheel

The GIF wheel (Sol's request, 12/10/2026: "maybe a wheel even of gifs / favorite
gifs") is the quick wheel opened by `+gifwheel` (Settings > Keys > Other > GIF wheel,
unbound by default) on the player's favourite GIFs from the SJK chat's
[GIF picker](hub-chat.md#the-gif-picker), newest first: ten a page, up to eight pages
("Favourite GIFs", then "Favourite GIFs 2" and on), each choice the GIF's preview
moving, drawn whole on a dark disc in its place on the ring, and "GIF 1", "GIF 2"... in
the middle (the number `sjkgif` takes). Letting go on one runs `sjkgif <id>`: the GIF
goes to the SJK chat ([From a bind](hub-chat.md#from-a-bind)). Its pages are made as it
opens ([`gif_pages`](../crates/sjk-viewer/src/quick_wheel.rs)), are not in
`wheel.json` and are not editable; a bare `+wheel` still opens on the page shown last
before it. Without favourites it says "No favourite GIFs yet". A GIF not loaded yet (or
with `cl_sjkChatGifs 0`) shows a disc with its name; it is sent all the same.

### Look

The ring is drawn in the SJK UI's look ([ring.rs](../crates/sjk-viewer/src/quick_wheel/ring.rs),
[SJK UI](sjk-ui.md)): the choices on a navy band (deep enough to read over any
scene) edged with holo line-work and a tick between choices; the highlighted
choice grown inside a gold ring with a gold glow behind it and a gold arc on the
band's rim; a gold arc on the middle disc pointing where the mouse points. In the
middle: the page's name, the highlighted choice's name in gold, and a dot for
each page (the page on show a gold bar). With more than one page, the previous
and next pages' names stand left and right of the ring on navy pills, with the
button that goes there ("Left click", "Right click"), and "Scroll to change page"
under it. A gold dot outside the band marks choices in effect. Text is in the
SJK UI's families (Rajdhani, Exo 2), which load in every menu style (Inter
until they have). Sizes follow the window's height.

The choices are round icons in the style of the game's neutral Force icons
(SJK's own, generated by Sol on two boards; [assets/wheel](../crates/sjk-viewer/assets/wheel/README.md)).
Every action has its own since 08/10/2026, and every custom command shares one,
braces round a dot (`{•}`), so several custom choices on a page look alike: the
middle names the highlighted one. A choice whose picture is missing shows its
name on a navy disc with a holo rim (two lines at most). A page holds at most 10
choices (ten icons fill the ring with a gap between each, the highlighted one
grown; the Force page holds 12, drawn smaller); an empty page says "Nothing here
yet".

### Sounds

The wheel plays the game's own menu sounds
([ui_cues.rs](../crates/sjk-viewer/src/audio/ui_cues.rs)), quieter than the
menus play them, since the wheel is used during matches:

| When | Sound | Volume | Where retail plays it |
| --- | --- | --- | --- |
| The page changes (scroll, a click, a second wheel key) | `sound/interface/sub_select` | 0.4 | The controls menu's sub-tabs |
| The highlight moves to another choice, from none too | `sound/interface/menuroam` | 0.5 | Every menu's focus sound; the Force power screen as powers are picked |
| The chosen choice runs, on letting go | `sound/interface/button1` | 0.5 | Every menu button (the SJK menus' click plays it at 0.9) |

Moving within a choice, or back to the middle, plays nothing; a page change
plays only its own sound, though the pointer then points at another page's
choice. Letting go on nothing, Escape and the wheel closing for the console, chat
or a menu play nothing: nothing ran. The sounds go through the effects volume
(`s_volume`, and silence when the window is muted while unfocused), each on its
own interface channel, so a quick run of moves cuts the last tick short instead
of piling up. A PK3 replacing one of these files (JoF's cosmetic mod replaces
`button1`) replaces it in the wheel and the menus alike.

`cg_wheelSounds` (archived, default 1) turns them off: Settings > Quick wheel's
Sound row, which the classic+ menus' editor has too, or Interface's "Quick
wheel sounds" row. The wheel reads it as it opens.

### Choices

The catalogue ([catalog.rs](../crates/sjk-viewer/src/quick_wheel/catalog.rs)),
by its groups in Settings:

| Group | Actions (console command) |
| --- | --- |
| Camera | Third person (`togglecamera`), Free camera (`freecam`), Camera control (`cameracontrol`) |
| Interface | Nameplates (`nameplates`, cycles), HUD on/off, Screenshot (`screenshotJPEG`), Game menu (`togglemenu`), First setup (`firstsetup`), Match timer, FPS counter, Speedometer, Lagometer (`toggle cg_...`), Console (`toggleconsole`), What's new (`changelog`) |
| Player | Cosmetics, AFK, Team menu (`teammenu`), Spectate (`team spectator`), Respawn (`kill`), Inspect player, Engage duel, Saber style (`saberAttackCycle`), Saber on/off (`sv_saberswitch`) |
| Emotes | Taunt, Bow, Meditate, Flourish, Gloat, Sync emote (`sjkemote sync`), Stop emote (`sjkemote stop`), then the installed emotes (`sjkemote <id>`, [hub-chat.md](hub-chat.md#emotes-in-the-client)) |
| Toys | Illuminate (`toy_illuminate`, marked while the holocron is lit), Cardboard box (`toy_box`, marked while the player is in it), Order 66 (`toy_order66`, marked while it is on), Execute Order 66 (`order66`) |
| Weather | The map's own weather, drizzle, rain, storm, snow (`r_weatherForce`), ground fog everywhere (`r_weatherFog` 1/2), clouds, weather on/off |
| Time of day | Day (`r_dayHour 12`), Night (`r_dayHour 0`), with the spare sun and moon icons |
| Votes, demos and server | Vote yes, Vote no, Record demo (`record`), Stop recording (`stoprecord`), Reconnect |

A custom choice is a name (up to 24 characters) and any console command (up to
200), several separated by `;`, run with all its arguments (`japlus.guntele 300`,
`amtele 3`) as typing it would; the tests pin that it keeps them through the form,
`wheel.json` and `wheel_commands.json`.

A custom choice may show a picture of the player's own in place of `{•}` (Nib's idea,
03/11/2026; [pictures.rs](../crates/sjk-viewer/src/quick_wheel/pictures.rs)): its form's
Choose picture (or F3) opens the system's file dialog for a PNG or JPEG of at most 4 MB
and 4096 pixels a side; the picture is fitted into a wheel icon (a 128-pixel square, its
shape kept, on transparency) and kept as a small PNG in `wheel_icons/` in the profile
folder, named by its content, so the original can move or be deleted. Remove gives the
stock picture back. Which command shows which picture is kept in `wheel_icons.json`
beside `wheel.json` (`{"version": 1, "icons": {"say gg": "<16 hex digits>.png"}}`, only
names of that form taken); a choice is matched by its command, so the same command on
two pages shows the same picture, and it takes precedence over an emote's own picture.
Eight different pictures at most are in use at once, the room left in the UI atlas; the
form refuses a ninth, saying so. A picture that no longer decodes is reported to the log
and its choice shows `{•}`. Unit tests cover the checks, the fitting, the file and the
limit; the dialog itself was not opened in a test.

### Settings

In the SJK UI, Settings has a Quick wheel category (after HUD; see
[SJK UI](sjk-ui.md#quick-wheel)); Interface's "Quick wheel pages" row goes to it.
With the classic+ menus that row (Interface, and the tabbed settings' GAME tab)
opens the same editor on its own, in the SJK UI's look over the settings, Escape
returning to the row ([wheel_editor.rs](../crates/sjk-viewer/src/settings/wheel_editor.rs)).

- **Pages** (left): each page with how many choices it has ("Your powers" for
  the Force page), the one whose choices show in gold; Add a page (at most 8),
  Add the Force page (only while it has been removed: it goes back after General,
  else last), then Restore the default pages (Force, General, Toys, Weather; Enter
  twice, or two clicks); under them, Sound's "Wheel sounds" switch
  (`cg_wheelSounds`: Enter, Space or a click). Up and Down choose (the choices
  follow), Enter or Right goes to the page's choices, F2 or Rename names it,
  Shift (or Ctrl) with Up or Down moves it, Delete twice removes it (not the
  last one). A new page is named at once ("Page 4" if left blank); names are 20
  characters at most. The Force page is renamed, moved and removed the same way,
  but has no choices to go to: the middle column says they follow your Force
  powers and how they act, and the preview is an example (a light-side build).
- **Choices** (middle): the page's choices in ring order (the first at the top,
  then clockwise), each with its picture and group, then Add a choice. Enter
  changes a choice from the catalogue (a custom one opens its form), Shift with
  Up or Down moves it round the ring, Delete removes it, Left returns to the pages.
- **Right column:** a live preview of the page's ring, the focused choice
  highlighted, then the page's key and bind (`+wheel <id>`) or what the choice
  runs. Picking a choice shows the catalogue there instead (Up and Down, Enter,
  Escape; actions already on the page say so), then under Your own the custom
  commands named before and Custom command last, with a
  search field right of its title that has the keyboard as it opens: typing keeps the actions,
  emotes and named commands whose name or group (a named command's command too)
  holds every word (Custom command always stays),
  held letters and Backspace repeat, Ctrl+A, Ctrl+C and Ctrl+V work as in every
  field, Backspace on an empty search closes it; a custom
  choice's form has its name and command fields, Save and Cancel.
- **Named commands** (after Dante's idea): a custom command saved from its form is
  remembered and offered under Your own in every page's catalogue, newest first,
  one per command (the same command saved again takes its new name and moves
  first), 24 at most (the oldest go). Delete on the highlighted one, or its cross,
  forgets it; choices already on pages keep it. They are kept in
  `wheel_commands.json` beside `wheel.json` (see below).
- The pointer: hovering a row focuses it (a page row without showing that page,
  which a click does), the focused row's small controls act (Rename, up, down,
  remove), a click on a choice or Add a choice opens the catalogue, a click on a
  line of it picks.

Every change is saved at once.

### Kept in wheel.json

The pages are kept in `wheel.json` in the profile folder
([pages.rs](../crates/sjk-viewer/src/quick_wheel/pages.rs)), written (whole, then
moved over the old file) when the player changes them; without the file the
wheel has Force, General, Toys and Weather, and Restore the default pages removes it so
a later version's defaults reach the profile. A JSON file rather than cvars: a page
is a list, a custom choice a free console command (quotes, semicolons) that a
`config.cfg` line would have to escape and a cvar's length would cap.

```json
{ "version": 3, "pages": [
  { "id": "force", "name": "Force", "kind": "force" },
  { "id": "general", "name": "General", "choices": ["third_person", "nameplates", "hud"] },
  { "id": "toys", "name": "Toys", "choices": ["illuminate"] },
  { "id": "duels", "name": "Duels", "choices": ["duel", {"name": "Ready", "command": "ready"}] }
] }
```

A choice is an action's id or a `{name, command}` object; the Force page is
`"kind": "force"`, without choices (one at most: a second is left out). Reading
keeps what it can: unknown action ids, unnamed pages and incomplete custom choices
are left out, pages past 8 and choices past 10 cut, a repeated id numbered; a
file that is not JSON, or has no page, reads as the defaults (and is kept until
the next change). Version 2 brought the Force page: a file of version 1 (or none)
gets it once as it is read, after the page whose id is `general` (else last), and
is written back, so a player who removes the page afterwards keeps it removed; a
wheel of 8 pages is left as it is (Settings' Add the Force page, after removing a
page, or Restore, brings it).

Version 3 (10/10/2026) made the Force page the first and added the Toys page, an
ordinary page of the catalogue's Toys group. A file older than 3 whose pages are
exactly what version 2 had as its defaults (General, Force and Weather, nothing
renamed or changed in them) was never customised: it is removed and the wheel has
the new defaults, in the new order. Any other file keeps its pages and their order
and gains the Toys page once, after its Force page (else last), and is written back
as version 3; a Toys page removed afterwards stays removed, a wheel of 8 pages is
left alone (the file is not rewritten) and a page of the player's own called Toys
keeps its id, the new one taking `toys2`.

The named custom commands are kept apart, in `wheel_commands.json` in the same
folder, so Restore the default pages (which removes `wheel.json`) keeps them:

```json
{ "version": 1, "commands": [ {"name": "Ready", "command": "ready"} ] }
```

Newest first, one per command, 24 at most; an entry that is not a complete
`{name, command}` object is left out. Without the file, the custom choices already
on the pages are remembered as it is first read (installed emotes, `sjkemote <id>`,
aside: the catalogue lists them already) and the file is written; once it exists,
even empty, it is what is remembered.

## Configuration and content

The default writable client folder is `GameData/SJK/`, under the selected game
installation (see [storage.rs](../crates/sjk-viewer/src/platform/storage.rs)). It
is independent of the executable's location and working directory. The client creates it automatically. Important files include:

| File or folder | Contents |
| --- | --- |
| `config.cfg` | Settings and key bindings |
| `marks.txt` | Marked map positions, views and notes |
| `favorites.json` | Favorite servers |
| `jakey` | Persistent client identity key |
| `hud.json` | Optional custom HUD |
| `wheel.json` | The quick wheel's pages, once changed ([Quick wheels](#kept-in-wheeljson)) |
| `wheel_commands.json` | The quick wheel's named custom commands ([Quick wheels](#kept-in-wheeljson)) |
| `screenshots/`, `demos/` | Screenshots and recordings |
| `chatlogs/`, `qconsole.log` | Logs when enabled |

`config.cfg` and the scripts `exec` runs are read as UTF-8, or, when a file is not
valid UTF-8, one character per byte (Latin-1), as Jedi Academy and EternalJK save
names and chat binds; a leading byte-order mark (Notepad) is dropped
([config.rs](../crates/sjk-shell/src/config.rs) `decode_config_text`). So another
client's config runs with `exec`, and a `jampconfig.cfg` copied over `config.cfg`
loads: its `unbindall` and `unbind` lines are accepted and bindings on keys SJK has
no name for are dropped. Any other line still stops the load (and saving, so the
file is not overwritten). The [Import page](#importing-from-another-client) is the
way that takes only the name, model, field of view and bindings.

If `GameData/SJK/` cannot be written, the client uses its per-user folder:
`%APPDATA%\SJK\` on Windows, `~/Library/Application Support/SJK/` on macOS, or
`$XDG_CONFIG_HOME/SJK/` (otherwise `~/.config/SJK/`) elsewhere. The chosen folder
is shared by all profile consumers for the entire session. Startup reports it;
the console's `path` command also lists it. The fallback uses the profile in
that per-user folder; the per-user and portable profiles are not synced.
See [platform.rs](../crates/sjk-viewer/src/platform.rs) and
[storage.rs](../crates/sjk-viewer/src/platform/storage.rs).
Settings can be changed in the client, or `config.cfg` edited in a text editor, even
while the client runs. The shell remembers the texts it loaded and wrote
([config_watch.rs](../crates/sjk-shell/src/config_watch.rs)) and looks at the file's
size and time once a second, and again before each save and at exit, once its own
writes have landed. A file holding a text the client did not write was changed from
outside: it is loaded as at start (its cvars, and its bindings replace the table), the
console says so, and later saves build on it. A file deleted while the client runs
stays deleted: nothing is written for the rest of that run, so the next start is on
the defaults (the console says so, and settings changed before the restart are not
kept); `writeconfig` still writes it on request. Until 29/10/2026 the client wrote
the values it held over such an edit at its next save or at exit, so an edited or
deleted config came back with the old name, field of view or camera at the next start
(hub report #199, and Kushina's `autoexec.cfg` workaround for it).

Autosaving writes `config.cfg` only when an archived cvar or a binding changed:
key presses, `+`/`-` buttons and unarchived cvars never touch the file. A change
is saved once the archived state has held still for 2 seconds
(`CONFIG_SAVE_DELAY`), so a dragged slider or a script setting many cvars writes
once. The text is built on the main thread and written, through a temporary file
and a rename, by one background thread that takes the saves in order. The file
is written at once, on the main thread, when the client exits (every exit drops
the console), on `writeconfig`, and before `exec config.cfg` reads it back. Until
10/10/2026 every command frame, so every bound key press, rewrote the whole file
on the main thread. See [config_saver.rs](../crates/sjk-shell/src/config_saver.rs)
and `Shell::autosave` in [shell.rs](../crates/sjk-shell/src/shell.rs).

`writeconfig <name>` (or `write <name>`) saves the archived cvars and bindings to
`<name>.cfg` beside `config.cfg`, where `exec <name>` finds it, as OpenJK's
`Com_WriteConfig_f` does: `.cfg` is added to a name without an extension, any other
extension is refused, and the name stays inside that folder (no absolute path, no
`..`). Until 16/10/2026 a name was a path from the process's working folder, without
`.cfg`: on Linux the file went to the home folder and `exec` could not read it back
(a player's bug report). See `config_file_path` in
[shell_builtins.rs](../crates/sjk-shell/src/shell_builtins.rs).

`com_maxfps` defaults to `-1` (AUTO in Settings > Display); `0` is uncapped. The
old default, 1000, saved in every existing profile, is reset to AUTO once on
first launch (marker `com_maxfpsDefaultVersion`); a cap chosen afterwards is
kept. The default is not saved to the configuration. On the slider
AUTO is the rail's left end: arrows step AUTO, 0, 25, 50 and so on, and typing
`-1` selects it. An uncapped
client saturates the GPU; screen recorders and streamers sharing it then skip
frames (OBS reported 83% skipped for encoding lag against an uncapped client at
4K). See [runtime_settings.rs](../crates/sjk-viewer/src/runtime_settings.rs).

AUTO caps frames at stock's 125 unless `com_maxfpsMonitor` (archived, default 0;
Settings > Display > Detect refresh rate) is 1: then AUTO is the refresh rate of
the monitor holding the window, rounded to whole hertz and re-read once a second,
or 125 when the monitor reports none. At 0 the monitor's rate is never read, so
AUTO caps as it does for a monitor that reports none. Sol asked for the detection
to be off by default (08/10/2026). The setting changes only what AUTO means: a
`com_maxfps` the player set stays as set. It is new, so no profile has saved it
and its default reaches existing profiles without a migration.

A window without focus has caps of its own: `com_maxfpsUnfocused` (SJK's default
30; EternalJK's 0 keeps the normal cap) and `com_maxfpsMinimized` (50), where 0
uses `com_maxfps`. They replace the normal cap while they apply, minimized first,
so an alt-tabbed client does not render a 4K scene at full rate.

`com_maxfpsMenu` (archived, default 0; Settings > Advanced > Display > Frame rate >
Menu FPS cap, and the classic Video tab), Sol's request of 25/10/2026 for weak PCs,
caps frames while only the menus run: no live game, no demo and no server connection
waiting on its map (`GpuState::menus_only` in
[runtime_settings.rs](../crates/sjk-viewer/src/runtime_settings.rs)). That is the main
menu and its pages (Collection, Profile, Settings...), the menu world behind them and
the connecting card until the server's game state arrives. The game menu over a match
keeps the game's cap, since the game runs on under it, and so do a map loading for a
server (its connection is serviced and the load's steps handed on once a frame) and demo
playback. It lowers the normal cap, never raises it (an uncapped `com_maxfps 0` is
lowered too), and 0 keeps the normal cap, so nothing changes for anyone who leaves it. The unfocused and minimized caps still
replace it while they apply. The gameplay FPS's Auto follows `com_maxfps` alone.

`snd_mute_losefocus` (archived, 1, EternalJK's name and default; Settings > Sound
> Mute in background) silences every sound, music included, while the window is
unfocused or minimized. EternalJK pauses its sound device; SJK sets the effects
and music gains to zero and keeps mixing, so music and loops carry on silently and
come back where they are. The change is applied from the focus and minimize
events themselves, since a minimized window may draw no frame; see
[console_window_options.rs](../crates/sjk-viewer/src/console_window_options.rs).

The game never waits for good on a sound device that stopped. Windows ends an
output stream when its device goes away: a headset or Bluetooth speaker that
switches itself off after a while of silence (which a muted background window
plays), a monitor's speakers while the display sleeps, a change of the device's
format. The stream's callback then no longer empties the mixer's queue of 8,192
commands, and the render thread used to wait for room in it without end, so the
game stopped answering seconds later and Alt+Tab never brought it back. A full
queue now waits at most 250 ms, once; then commands are dropped (every frame sends
its state again) until the queue drains, and the log says `audio output stopped
taking sound`. The game goes on silently; `snd_restart` opens the output again, on
the device that is then the default. The decode worker keeps sounds in order and
waits for room as before, but stops when the output is dropped, so `snd_restart` and
quitting do not wait on it. See [audio_feed.rs](../crates/sjk-viewer/src/audio_feed.rs).

The output follows the system's default device (#210, 03/11/2026): plugging headphones
in left the game on the speakers until a restart, since Windows keeps the old device
working. Every 1.5 s the output compares the default device's name with the one it
plays on and, when it changed or the stream rendered nothing since the last look (once,
not at every look), moves the mixer to the default as it is: the stream is dropped, the
mixer comes back from the device's thread (a wrapper hands it back as it is dropped)
and a new stream plays it, so sounds, loops and music go on and only the old device's
buffer is lost. A device that does not open leaves the mixer waiting for the next look;
with no device at all the current stream stays. Two identical devices swapped for each
other are not told apart. See [audio_device.rs](../crates/sjk-viewer/src/audio_device.rs).

The Video tab's Display mode row offers Windowed, Borderless fullscreen and,
where the windowing system supports it, Exclusive fullscreen (Wayland does not,
and macOS is kept to borderless: winit's exclusive mode there captures the display,
which takes Cmd+Tab away, and panics inside AppKit's redraw when capturing it or
setting its mode fails; a Mac player's game aborted on choosing fullscreen, 25/10/2026).
Stock `r_fullscreen` keeps its meaning, fullscreen on or off, and Alt+Enter still
toggles it. `r_exclusiveFullscreen` chooses the kind: 0 (default) is a borderless
window at the desktop size, 1 switches the monitor to the `r_resolution` video
mode. Stock JA's fullscreen is always the exclusive kind. Choosing Windowed leaves `r_exclusiveFullscreen` alone, so Alt+Enter
returns to the last fullscreen kind. Exclusive fullscreen without a monitor mode
of that size falls back to borderless.

Exclusive fullscreen is paused while the window lacks focus: winit makes an
exclusive window topmost and keeps the monitor's video mode, which left the game
over the desktop after Alt+Tab or Win+D. On focus loss the window goes windowed
(restoring the desktop mode and the window's earlier placement), then minimises to
the taskbar, as SDL does for a fullscreen window in OpenJK (a window left on the
desktop was hub report #131; `minimise_after_focus`), and `r_resolution` is not
re-requested; on focus gain, such as restoring from the taskbar, it enters the
exclusive mode again. A loss within 0.75 s of a display change is ignored so
a focus flicker caused by the switch cannot loop. Borderless and windowed are
unaffected. The logic is in `apply_display` and `display_focus_changed`
([runtime_settings.rs](../crates/sjk-viewer/src/runtime_settings.rs)) and
`suspended_after_focus` ([display.rs](../crates/sjk-viewer/src/settings/display.rs));
the unit tests cover the decision, not Windows' behaviour, which is unverified
in the real client (a player reported Alt+Tab and Win+D failing in exclusive mode).

Enter or a click on the Resolution row opens a list of the monitor's video-mode
sizes, grouped by aspect ratio with the monitor's own first, then the other
groups by how close their shape is to it (on a 16:9 monitor 16:10, then 3:2,
4:3, 5:4; since 10/10/2026, before which they ran narrow to wide), largest first
in each, and the size in use highlighted. Windowed and borderless also list the classic presets that fit the
monitor and a custom `r_resolution`; borderless fullscreen always fills the
desktop and uses the size only when windowed. `apply_display` asks the window for
`r_resolution` only while it is windowed and winit reports it not fullscreen
(`window_size_request` in [display.rs](../crates/sjk-viewer/src/settings/display.rs)),
so a size picked in fullscreen waits for the return to a window. It asked a
fullscreen window for the size too, after `vid_restart` or a pick: winit's macOS
`request_inner_size` sets the content size inside the fullscreen Space, which left
the game a small window in the middle of a black screen (Glen's report,
29/10/2026); the Mac's green button, which does not change `r_fullscreen`, counts
as fullscreen too. `r_highDpi` (Video tab and Simple Display, "High DPI") says what
`r_resolution` counts: the screen's pixels (1, the default except on a Mac) or its points
(0, the Mac's default, as EternalJK's window there, SDL without high DPI): `window_size`
asks winit for a `LogicalSize` then, so 1280x720 is a 1280x720-point window on a Retina
screen rather than a quarter of one. The window was made at 1280x720 pixels before
(29/10/2026), 640x360 points on a Mac. The Resolution list divides the monitor's sizes by
its scale in points (`MonitorModes::counted_in_points`: a 3024x1964 desktop is 1512x982);
exclusive video modes always count pixels. A change resizes a windowed window at once
(`window_size_request` compares the size with how it was counted); the world's resolution
follows on the graphics reload ([rendering](rendering.md#render-scale-and-supersampling)).
Left and Right step the row within
its aspect-ratio group. See [display.rs](../crates/sjk-viewer/src/settings/display.rs)
and [resolution.rs](../crates/sjk-viewer/src/settings/resolution.rs).

`fs_game`, `fs_basegame` and `fs_homepath` configure content search paths; restart
the client after changing them. Search precedence and shader protection are owned
by [asset_search_paths.rs](../crates/sjk-viewer/src/asset_search_paths.rs).

Downloaded content is stored separately from the retail installation and config.
The default is `%LOCALAPPDATA%\SJK\downloads\base` on Windows,
`~/Library/Application Support/SJK/downloads/base` on macOS, and elsewhere
`$XDG_DATA_HOME/SJK/downloads/base`, falling back to
`~/.local/share/SJK/downloads/base`. `SJK_DOWNLOAD_HOME` overrides the download
root (the implementation appends `base`). See
[download_store.rs](../crates/sjk-viewer/src/download_store.rs).

Server reference lists use OpenJK's positional common-prefix rule: extra pak
names or checksums without a counterpart are ignored, including when one list
is empty. A name token without a `/` is the rest of the name before it (stock
servers write every name as `game/basename` and separate them with spaces), so
`base/{JoF}Sentinel Nexus` keeps its own checksum instead of shifting every later
name onto its neighbour's, as JoF's lists did in March 2026 (21 names for 20
checksums). Download comparison and session cache selection share
[the compatibility parser](../crates/sjk-client/src/referenced_paks.rs), so a
connection cannot pass one check only to fail the other on list length.
Paired checksums, download paths and file contents remain validated; advertised
BSP checksums are still enforced. This does not change pure-server proofs.

References are not a mandatory client install manifest. When the server sets
`sv_allowDownload 0`, or the client sets `cl_allowDownload 0`, UDP transfers are
skipped and joining proceeds with available content. Missing directory names,
unsafe download names and retail packs never produce a download request.
Pak names may hold spaces, braces, brackets, apostrophes and `!,&~` (JoF's
`{JoF}'s-GWing`); quotes, separators, `;`, `%`, `$`, wildcards and control bytes
are refused. The request is sent quoted (`download "base/{JoF}Sentinel Nexus.pk3"`),
since a stock server reads the name as `Cmd_Argv(1)`. A pak the server refuses
(it answers with a reason) or that fails verification (size caps, checksum, a
broken archive) is skipped with a `session content: skipped download` log line and
the join goes on with the others; before 12/10/2026 any one failure ended the
connection. 30 s without a block still ends it, as the server has stopped
answering ([download.rs](../crates/sjk-client/src/download.rs)).
Unavailable referenced archives do not abort world mounting; only locally
available checksum matches are selected from the cache. The actual map must
still exist and match its advertised BSP checksum. HTTP downloading remains
unsupported, and this policy does not disable pure-server admission checks.

A server that lists no paks still names its map. When no installed content has
`maps/<mapname>.bsp`, the newest download holding it is mounted (with the policy
below), so a map downloaded once stays playable on servers that stopped listing
their paks; JoF's servers send `sv_allowdownload 0` and an empty list since at least
October 2026. A map that is installed is never replaced this way.

### What a download may bring

A downloaded pak only adds content, and only to the sessions whose server
references it (or whose map it alone has). The archive is kept exactly as
downloaded, because its checksums are what servers compare (pure servers
included); what the policy refuses is hidden when it is mounted
(`VirtualFileSystem::mount_pk3_filtered`), so nothing reads, lists or `exec`s it:

| Hidden | Examples |
| --- | --- |
| Programs and host scripts | `cgamex86.dll`, `.so`, `.exe`, `.bat`, `.ps1`, `.js`, `.qvm` |
| Console scripts | every `.cfg` (`autoexec.cfg`, `mpdefault.cfg`) but the model tables `models/**/animation.cfg`, `animevents.cfg` and `sounds.cfg`; ICARUS `.ibi` |
| Menus | everything under `ui/` and any `.menu` (menus run commands) |
| Archives | a `.pk3`, `.zip`, `.7z` or `.rar` inside the pak |
| Replacements | any file the game data or an installed pack already has (sounds, textures, models, levelshots, sabers) |

Two kinds are exceptions to the replacement rule. A map (`.bsp`) may replace an
installed one of the same name, as the server runs it (JoF's FFA3 Night ships as
`maps/mp/ffa3.bsp`); the session still checks it against the server's map
checksum when one is sent. Shader scripts always mount, and their definitions are
screened one by one: the shader catalogue is loaded from the game and installed
packs as before, then a download's scripts may add new shaders but never redefine
one those define, nor one named after an installed texture (an implicit shader
every map using that texture would take up). Each download logs what it lost
(`session content: download ... stripped of 1 scripts, 2 replacements (...)` and
`shader authority: download ... may not redefine N shaders`). See
[download_policy.rs](../crates/sjk-viewer/src/download_policy.rs),
[session_content.rs](../crates/sjk-viewer/src/session_content.rs) and
[shader_authority.rs](../crates/sjk-viewer/src/shader_authority.rs).

A map that changes retail textures for its own look loses those changes: the
installed texture is used. Installed packs (in `GameData`) are the player's own
choice and are not screened.

## Key names and binds

`bind`, the controls editor and the config name keys as stock JA does on Windows:
by what the active keyboard layout prints on them. On a French AZERTY keyboard
`bind w +forward` is the key labelled W, and the key left of W is named `<`.
Character keys take the layout's unshifted character, including non-ASCII ones
such as `é`, `ù` or `²`; a dead key is named by its accent, so AZERTY's `^` key is
`^`. The digit row keeps `0`-`9` on every layout, keys without a character
(arrows, F-keys, keypad, modifiers) keep their stock names, and letter keys of
non-Latin layouts such as Cyrillic keep their US letter, so the default binds
still reach a key there. A release runs the binds of the name its press had.
Menu navigation keys (W/A/S/D beside the arrows) stay positional. See
[keys.rs](../crates/sjk-viewer/src/input/keys.rs) and
[key_names.rs](../crates/sjk-shell/src/key_names.rs).

Key names are shown in capitals, as retail's controls menu shows them
(`BindingFromName` upper-cases with `Q_strupr`): the key binding editor, the
vote prompt and the console's `bind`, `unbind` and `bindlist` output read `W`,
`SPACE`, `MOUSE1`. Only ASCII letters change, so layout names such as `é` keep
their character. `config.cfg` keeps the saved spelling (`bind "w" ...`), and
`bind` accepts names in any case. See
[key_names.rs](../crates/sjk-shell/src/key_names.rs).

## Colour codes

Text draws `^0` to `^9` as OpenJK's ten-entry colour table does: `^0`–`^7` are
the retail colours, `^8` is orange and `^9` grey (retail wrapped them onto black
and red). The table is `quake_color` in [text.rs](../crates/sjk-viewer/src/text.rs).

## Console styles

`con_style` (Settings, TEXT tab, "Console style") picks how the console looks:

| Value | Look |
| --- | --- |
| `auto` (default) | `sjk` while the menus are the SJK UI (`ui_menuStyle sjk`), else `classic` |
| `sjk` | The SJK UI's deck ([SJK UI console](#sjk-ui-console)) |
| `classic` | EternalJK's console (`cl_console.cpp`, `cl_keys.cpp`) |

A mistyped value acts as `auto`, as do `horizon` and `dock`, two SJK UI designs
removed on 08/10/2026 once Sol chose the deck, and the retired `modern` (or `0`),
which a profile's start also resets ([Menu style](#menu-style)). Every profile had saved the old default
`classic`, so a saved `classic` moves once to `auto` (`con_styleDefaultVersion`,
as `cg_scoreboardStyle` did): it still gives the classic console unless the
menus are the SJK UI; a `classic` chosen afterwards stays. Input editing, mouse
selection, completion, the `]cmd` echo, history (Up and Down) and the F3 browser
work the same in both looks, which share EternalJK's keys below. See
[console_classic.rs](../crates/sjk-viewer/src/console_classic.rs),
[console_sjk.rs](../crates/sjk-viewer/src/console_sjk.rs),
[console_backdrop.rs](../crates/sjk-viewer/src/console_backdrop.rs) and
[console_options.rs](../crates/sjk-viewer/src/console_options.rs).

An outgoing whisper's server echo does not say who received it. SJK remembers
the target when its final `tell` command is successfully queued, matches the
private `[name]: body` echo from the local player by its plain message body, and
attaches that historical target to the console line. This does not require the
optional `0x19` chat marker, which compatible servers may omit. Hovering any
wrapped row of that line shows `Whisper sent to name (#slot).` in either console
style. The metadata is bounded with the scrollback, remains the name used at
send time if the roster changes, and is not inferred for unmatched or incoming
private messages. See
[console_whisper_hover.rs](../crates/sjk-viewer/src/console_whisper_hover.rs).

### Classic console

Text sits on a grid of character cells drawn with the console font, without the
glyph shadow other UI text has. The console font is JetBrains Mono, a bundled
vector font rasterized once at startup like Inter
([console_font.rs](../crates/sjk-viewer/src/text/console_font.rs)), in place of
retail's `gfx/2d/charsgrid_med` bitmap, whose 8 by 16 texel glyphs looked heavy
and blocky magnified at 1440p and 4K (SJK draws no bitmap fonts:
[sjk.md](sjk.md#fonts)). It keeps the character set's cell: every character
advances half the line height, and the font's ascent and descent are centred in
the cell. The retail character set had no Windows-1252 typographic characters
(`€`, `’`, `‘`, `…`, bytes 0x80..=0x9E); the classic console still gives them no
cell, like colour codes, so rows close up as in EternalJK, while the text keeps
them. The classic console loads the font whether `ui_gameFont` is on or not; if it
cannot be loaded the cells use Inter.
A cell is 8 by 16 pixels at 1080 lines and `con_scale 1`, grows with the window
height like the rest of the UI (with the console's 0.75 floor) and is rounded to
whole pixels. EternalJK's cells are 8 by 16 screen pixels times `con_scale`, so
`con_scale 0.5` at 2160 lines matches EternalJK at 4K with its default scale. A
row holds the screen width in cells minus two, and column `c` is drawn `c + 1`
cells from the left edge, as `Con_DrawSolidConsole` does.

The background is the game's `console` shader, read from the shader scripts and
the PK3s like any other, so a pack that overrides it wins as it does in
EternalJK: the retail one scrolls a star field (alpha blended) and adds the
pulsing Jedi Academy logo; a cosmetic pack such as JoF's draws an opaque picture
with scrolling stars over it. Each stage keeps its `blendFunc`, its `tcMod`
(`scroll`, `scale`, `rotate`, `transform`, `stretch`) and its `rgbGen` and
`alphaGen` (`wave`, `const`, `vertex`); other generators draw at full strength,
and an `animMap` shows its first frame. Below full height `alphaGen vertex` is
`con_opacity`, so the retail stars fade with it while an opaque first stage stays
opaque; at full height it is 1. Without the shader the console is a dark navy
panel. The background reaches `480 × fraction − 2` units of the 640×480 virtual
screen, with a bar two units tall in EternalJK's `console_color` (0.509, 0.609,
0.847) under it. `con_ratioFix` (1, archived, EternalJK's name and meaning) shows
the middle of the picture (`t` from `1 − k` to `k`, `k` = 4:3 over the screen's
aspect) when the console is half the screen or less on a wide screen, instead of
squashing the whole picture; set 0 for custom backgrounds made for a squashed
fit.

The classic console has its own 2D layer, drawn after every other 2D element,
text included, and its own text last, so menu, chat, HUD and frame-rate text
never shows through an opaque console.

| Key | Classic console |
| --- | --- |
| Console key (the key under Escape, or `cl_consoleKeys` with `cl_consoleUseScanCode 0`) | Opens to `con_height` (0.5) |
| Ctrl + console key | Opens full screen |
| Shift + console key | Opens a quarter of the screen; to `con_height` with `con_shiftHeight 1` |
| Shift+Escape | Opens to `con_height`; a quarter of the screen with `con_shiftHeight 1`; Escape closes |
| Page Up, Page Down, mouse wheel | Scroll back or forward two rows; ten with Ctrl |
| Ctrl+Home, Ctrl+End | Oldest row, newest row |
| Up, Down, keypad 8 and 2 (Num Lock off), Shift+wheel, Ctrl+P, Ctrl+N | Command history (32 commands) |
| Ctrl+L | Clear the scrollback |
| Insert | Toggle overstrike: typing replaces the character after the cursor |

Shift's heights follow EternalJK's `CL_KeyDownEvent` by default; `con_shiftHeight`
(archived, off; Settings > Interface, "Swap Shift console size") swaps
them, so a player with a taller `con_height` gets it on Shift plus the console key.
A normal console opening follows `con_height` live, so changing it while the
console is open resizes it in both styles. Ctrl plus the console key and the
quarter-screen opening keep their full-screen and quarter-screen overrides; an
opening that follows `con_height` clears those overrides. A `toggleconsole` bind reuses the last opening mode,
following the current `con_height` when that mode was normal. The console
slides at `scr_conspeed` screens per second and is drawn from its first pixel.
A disconnected client whose menu
is closed shows the console full screen, as `Con_DrawConsole` does; the map
viewer without a menu does not.

Scrollback rows are drawn from three cells above the console's bottom edge up to
the top of the screen. Lines start white (`^7`),
error lines light red, and a colour code carries on into the rows a line wraps
onto. Words wrap as `CL_ConsolePrint` wraps them: a word that fits on a row but
not in what is left of the current one, or would end exactly at its edge, starts
the next row; a word longer than a row breaks at the edge (EternalJK breaks such
a word early, at an odd point); colour codes take no room. With `con_timestamps 2`
(EternalJK's layout) every row, wrapped ones included, starts with the local time
its line was written in grey, `HH:MM:SS` and a space, and the text wraps in the
columns after it; `1` also stamps the notify lines and `0` leaves no stamp column.
Scrolled back, a row of `^` every four columns in the bar colour sits under the
rows, and new output does not move the view.

The input row is two cells above the bottom edge: the local time in green in
columns 1 to 8, `]` in column 10, then the input as typed, colour codes shown and
applied as EternalJK draws them (`^1a` shows `^1a` in red; it was all white before),
and a cursor that blinks every 256 ms: a bar on the cell's
bottom rows or, in overstrike mode, a block, both solid shapes in the retail
character set's cursor proportions (Inter cells use `_` and a box). The
input scrolls sideways to keep the cursor on screen. In the bottom-right corner
the version line ends one cell from the edge, two and a half rows up, and the
local day, date and time sit under it at the edge, both in the bar colour. Where
EternalJK prints `asctime` on a 12-hour clock (`Sun Oct  4 10:52:10 PM`), SJK writes
the date day first on the 24-hour clock (`Sun 04/10/2026 22:52:10`; see
[SJK conventions](sjk.md#dates-and-times)).

Closed, the console can draw notify lines, the console feed, while a game, a demo
or a map walk runs and no menu has focus: of the last `con_notifylines` rows, those
written within `con_notifytime` seconds and not quiet (chat), from the top edge of
the screen, one cell plus `cl_conXOffset` pixels from the left. The feed is off by
default and `con_drawNotify 1` (archived; Settings > Interface > Console feed, the
TEXT tab in the tabbed screen) turns it on (Sol's request, 10/10/2026). What it
shows: server `print`s (joins, team changes, votes, command replies, server
announcements), the one-line obituary of each kill, the client's own messages
(errors, demo and download notices, cvar replies). Chat never goes through it: it
keeps its chat box (`cg_drawChat`), and centre prints, the kill feed
(`cg_killfeed`) and the vote panel are drawn by the HUD, so all of them
stay with the feed off. The open console still shows every line. Dragging the mouse selects
scrollback text and Ctrl+C copies it; the selection does not include the time
column. By default copied text is plain. `con_copyColors 1` (archived; Settings >
Interface > Copy console colours) retains its original `^0`-`^9` colour codes so
it can be pasted back into the game with the same colours. A selection whose
first text uses the console's implicit white starts with `^7`, rather than
inheriting a chat channel's green or another colour when pasted.

### SJK UI console

`sjk`, the deck, is the classic console with another skin
([console_sjk.rs](../crates/sjk-viewer/src/console_sjk.rs)): the same grid of
JetBrains Mono cells, word wrap, scrollback, history, keys, overstrike, mouse
selection and notify lines, drawn in the SJK UI's colours
([sjk-ui.md](sjk-ui.md#colours)) without the `console` shader. Rows are a fifth
further apart than the classic console's (19 pixels at 1080 lines), the cells
centred in them; text starts in the SJK UI's text colour, error lines in a soft
red, time stamps quiet, and colour codes use its legible palette. The input row
starts with a gold `›`, the caret is a thin gold bar (a gold block in overstrike
mode), and while the caret ends a unique command or cvar name being typed the
rest of the name shows faintly after it, as Tab would complete it. Scrolled back,
the bottom row says how many newer rows there are; a thin bar right of the rows
shows the part on screen, gold while scrolled back. Labels are in the SJK UI's
families (Rajdhani and Exo 2), drawn on the console's own layer so nothing shows
through; they load with the look, and the console font stands in until they
have. After Ctrl+C the header says how many characters were copied. Below full
height the panel follows `con_opacity`. The notify lines stay the classic ones.

The deck is a full-width navy panel (the world shows faintly through): a header
with "Console" underlined gold, the version, the date and the time; the input
on a band with the SJK UI's gold bar; key hints (Tab, Up and Down, Page Up and
Down, F3, Ctrl+C); a gold rail fading into a holo line along its bottom edge.

A click on the F3 hint opens the command browser, which is drawn in the SJK
UI's look with the deck (see below). Verified by unit tests (the looks' names,
`auto` and the retired names, the saved value's move, the grid, caret and ghost
completion) and by off-screen frames of every look over mp/duel6 at 1080 lines
and 4K (`world_shot::tests::duel6_console_styles`); not yet used in a running
client.

Local time comes from the operating system's time zone rules, daylight saving
included ([local_time.rs](../crates/sjk-shell/src/local_time.rs)); the scrollback's
stamps and the log file's `[HH:MM:SS]` are local time too. Differences from EternalJK besides those above: the input never runs past
the screen's right edge (EternalJK's can).

## Passwords in the console

`password <pw>` typed while a join is still trying, or on the screen that says why it
failed (a wrong or missing password, a full server), joins the same server again with
it (headshotsmcgee117's report, 03/11/2026: only `connect <ip>;password <pw>` worked), as
stock's client sends its next `connect` with the new userinfo (`CL_CheckForResend`). The
password a join already sent changes nothing ([connection.rs](../crates/sjk-viewer/src/connection.rs),
`rejoin_on_new_password`).

The console never prints a password: what it prints is kept in the scrollback,
`qconsole.log` and the crash trail that the session log, crash reports and log uploads
are made from ([identity](identity.md#crash-reports)).
[secret.rs](../crates/sjk-shell/src/secret.rs) in `sjk-shell` names what stock Jedi
Academy and SJK hold secret: cvars whose name contains `password` (`rconPassword`,
`password`, `g_password`, `sv_password`, `cl_consoleSocketPassword`) or `giphykey`
(`cl_giphyKey`), which get the `SECRET` cvar flag when registered. A server family's
passwords belong to its client mod, which declares them in a table of its own
([mods.md](mods.md#passwords)): JA+ tools add the commands `amlogin`, `login`,
`register`, `changepassword`, `clanpass` and `clanwhois`, whose arguments are a
password, and `loginpass` (`japlus.loginPass1-3`). The client adds every table its
build carries at startup, before any cvar or line, whatever the server and the mod's
switch: `amlogin <password>` typed offline, or in a bind or a config run at startup,
is masked like the rest. Three layers keep them out:

- every description and listing (a cvar's value, `print`, `cvarlist`, `cvar_modified`,
  completion's listing, `userinfo`, the cvar browser) shows a set secret value as
  `********` through `Cvar::shown_value`, and a command line a bind or a cvar the
  player created holds with its password masked (`autologin = "amlogin ********"`);
  a registered cvar such as `name` is shown as it is, never read as a command;
- the echo of a typed line masks a password command's arguments, a secret cvar's new
  value (`]rconpassword ********`, also after `set`, `toggle`, `ifCvar` and the
  arithmetic commands) and the password in a command line given to `set`/`seta`, a
  cvar, `bind` (`]bind P "amlogin ********"`), or carried by `rcon`, `cmd`,
  `strSub`, `delay`/`waitf` or an `ifCvar` branch; a line with an open quote is
  masked from its first password word on;
- every kept line, and every command output a frontend gets back, is scrubbed of the
  current values of the secret cvars of at least four characters, the longest first
  (so `jedi2024` is not left as `********2024` beside `jedi`), so a path the
  first two miss (`strSub echo $rconPassword$`, a mod printing userinfo) prints
  `********` too.

The values themselves are unchanged: the cvar and the bind keep them, the command and
the server still get them (`bind P amlogin <password>` logs in as before), and
`config.cfg` still saves the archived ones and binds as plain text. The cvar browser
treats a secret, and a cvar whose shown value has a password masked, alike: it edits
it from an empty field drawn as one `*` per character, keeps it when Enter confirms
the empty field, and copies it by name, so the mask is never typed back as its value
and the value is never drawn. There is no setting to show them again (`secret.rs` says why), and
lines printed before this change (an older session log, a crash report already
waiting) are not rewritten.

Covered by unit tests of the tables (the JA+ one in its crate), each layer and the
cvar browser's edit and copy, a client test that types `amlogin`, binds it and sets
`japlus.loginPass1` off any server with JA+ tools off and finds no password in a kept
or tapped line,
and a shell test that types, queries, lists, binds, holds in a cvar, `strSub`s and
`delay`s passwords, each path its own so the scrub cannot hide a gap in the others,
and finds none in a printed line, returned output or tap line while the cvar and binds
keep them; not exercised in a live game.

## Console socket

`cl_consoleSocket <port>` lets a program on the same computer read the console
and run commands, as JoF EJK's setting of the same name does; apps written for
it (Sol's Archive, a chat quote bot) work unchanged. It is off (`0`) by default.
`cl_consoleSocketPassword` is required: the socket does not open without one and
starts as soon as one is set, so a web page that writes to a loopback port cannot
run console commands (JoF EJK allowed an empty password). Both cvars are archived,
the password in plain text like `password`. The `consolesocket` command explains
the feature and shows its state.

```
cl_consoleSocketPassword mySecret123
cl_consoleSocket 29071
```

The protocol is lines of text on `127.0.0.1:<port>`, which only this computer can
reach. The app sends the password as its first line (anything else closes the
connection and nothing runs) and is answered `cl_consoleSocket: authenticated`.
Then every console line the app has not seen is streamed to it as it prints, ended
by `
`, with `^n` colour codes; lines printed before it authenticated are not
sent. Each line the app sends runs as a console command, as if typed (`say hi`
chats). Text uses Windows-1252 bytes, like chat on the wire
([legacy_text.rs](../crates/sjk-protocol/src/legacy_text.rs)), or Windows-1251
for Russian ([player text](networking.md#player-text)); other text is sent as
UTF-8.

Chat lines carry the stock `0x19` separator before the colon (`Name^7<0x19>: hi`),
which is how an app tells chat from other output; the console shows the line
without it, so the socket is given the server's raw text instead
(`push_chat_line` in [console_socket.rs](../crates/sjk-viewer/src/console_socket.rs)).

Up to 4 apps connect at once, a command line is at most 1023 characters (a longer
one is discarded whole), and an app that stops reading is dropped after 256 KB of
unread output; the game never waits for it. The listener, framing and limits are
[console_socket.rs](../crates/sjk-shell/src/console_socket.rs) in `sjk-shell`; the
viewer feeds it once per frame from `run_console_command_buffer`. Covered by unit
tests over real loopback connections (authentication, limits, chat separator,
command execution); not yet run against a game session or Sol's Archive itself.

## Useful console commands

Every command and cvar the client knows, with its help, default and Settings row, is
in the generated [command reference](development.md#command-reference).

`saber <hilt> [<second hilt>]` ([saber_hilt_command.rs](../crates/sjk-viewer/src/saber_hilt_command.rs),
21/10/2026, Creyon) sets the hilts held, as JoF EJK players type them: `saber kyle` one
saber, `saber kyle kyle` two, `saber dual_4` a staff (`saber1`, and `saber2` set to `none`
when no second is named); `saber` alone says what is held. Both names must be installed
`.sab` sabers (any case; `none` for the second) before either cvar changes, so a misspelt
second hilt leaves both as they were (review of 22/10/2026); with no game data read, the
names go unchecked. On JA+ (JoF's servers too) and jaPRO-lineage servers the new sabers
also go to the server as its own `saber` command, so they apply without a respawn: not
from this command but from `sjk_client::SaberChangeNotifier`, which follows the two cvars
whoever sets them, with its list of the servers that take it
(`profile_accepts_saber_command`). The Collection's stage follows
`saber1` and `saber2` while it is open (`SaberMenu::follow`), so its preview, and its switch
for two sabers or a staff, change at once.

The console key opens and closes either console style and never types its
character, as in EternalJK (`IN_TranslateSDLToJKKey` and `IN_IsConsoleKey` turn
it into `A_CONSOLE`). By default (`cl_consoleUseScanCode 1`, EternalJK's) it is
the physical key under Escape on every layout, so Hungarian, where that key types
`0`, has one; on layouts where it types `^` (German) it needs Shift, leaving `^`
for colour codes, and `cl_consoleShiftRequirement` can require Shift elsewhere.
Profiles that saved the earlier default 0 are moved to 1 once
(`cl_consoleKeyDefaultVersion`). With `cl_consoleUseScanCode 0` the console key is
a `cl_consoleKeys` character such as `~` or `²` instead. Holding it toggles once. Escape and Shift+Escape close
the console too, and so does a key bound to `toggleconsole` that prints nothing;
a printable key bound to it types while the console is open
([console_keyboard.rs](../crates/sjk-viewer/src/console_keyboard.rs)). While the
chat composer is open, the console key (or a non-printing key bound to
`toggleconsole`, or Shift+Escape) opens the console over it, as stock's console key
goes before the message field (`cl_keys.cpp:1318`) and leaves the field and its draft
under the console (`Con_ToggleConsole_f` only adds `KEYCATCH_CONSOLE`): typing goes on
when the console closes. The chat is not drawn while the console is open, which stock
draws last, as text would otherwise draw over the console's shapes. The composer keeps
what a player types: a letter or digit (the key under Escape types `0` on
Hungarian), a printable key bound to `toggleconsole`, and Shift with a key that opens
the console alone, so `~` can be written; where the console key needs Shift (a `^`
layout, `cl_consoleShiftRequirement`), Shift opens it (`console_key_leaves_chat`). Settings > Interface > Console over chat (`con_overChat`, on, archived) turns this off: the composer then takes the console key as it did before 29/10/2026 (Sol's request on PR #255); the chat stays hidden while the console is open either way. On layouts with dead keys
(`^` on French AZERTY and German QWERTZ, `'` on US International), the console
line and the chat draft show a dead key at the caret at once and replace it with
what the platform composes on the next key, so typing reads as on a layout without
dead keys: `^` then `1` gives the colour code `^1`, `^` then `e` gives `ê`, and `^`
then Space gives `^`. Backspace removes only the shown `^`; Enter sends it. A dead
`^` that xkb composes into a superscript digit (`¹`) becomes `^1` again, since a
caret before a digit is a colour code. See
[dead_key.rs](../crates/sjk-viewer/src/input/dead_key.rs). The browser's search
field takes a dead `^` as a literal colour prefix. Opening or closing the console
clears the window's pending accent composition, so a dead toggle key such as `^`
on a German layout does not combine with the next letter.

Alt codes type as in other Windows programs: hold Alt, type a number on the
numeric keypad and release Alt. A number starting with 0 is a Windows-1252 code
(`Alt+0248` gives `ø`), any other a code page 437 one (`Alt+21` gives `§`,
`Alt+130` gives `é`), counted modulo 256; a code naming a control character types
nothing. They work wherever text is typed (the console line and browser search,
the chat draft, menu fields) and not during play, where the keypad keeps its
bindings; the keypad digits of a code reach no field, so the classic console's
keypad 8/2 do not walk the history. AltGr and Ctrl+Alt do not compose. Windows
composes these itself, but winit drops the resulting character, so the client
composes them with the same rules
([alt_code.rs](../crates/sjk-viewer/src/input/alt_code.rs)). Codes without a
leading 0 always use code page 437, the US OEM page, even where Windows uses
another OEM page (850 in much of Western Europe).

`alias <name> <commands>` names a command line, as Quake II's `alias` does (the
original Jedi Academy has none; Lex's idea, 04/11/2026): after `alias duel "say
ready; +use"`, typing `duel`, a bind to it (`bind f duel`), an `exec` script or a
quick wheel command runs `say ready` and `+use` where `duel` stood. Quote the
commands when they hold a `;`, or the rest of the line runs at once. A `+name` alias
with a `-name` one works as a held key: `alias +zoom "cg_fov 40"`, `alias -zoom
"cg_fov 80"`, `bind mouse3 +zoom` zooms while the button is held. Names ignore case,
are up to 32 characters without spaces, quotes or `;`, and cannot be a command or a
setting the client knows (`alias bind ...` is refused; a server's command, such as
`kill`, can be). `alias` alone or `aliaslist [<part of a name>]` lists them,
`alias <name>` shows one, `unalias <name>` and `unaliasall` remove them; listings hide
passwords as `bindlist` does, and Tab completes alias names. An alias may run other
aliases; one that runs itself, or a chain 16 deep (the `exec` limit), stops with a
console message, so a looping alias with `wait` stops too, unlike Quake II's. Aliases
are kept for the next start in `aliases.cfg` beside `config.cfg`, not in it, so a build
without aliases still loads that config; a line of `aliases.cfg` that cannot be read
is reported and skipped
([alias.rs](../crates/sjk-shell/src/alias.rs),
[shell_alias.rs](../crates/sjk-shell/src/shell_alias.rs)).

`r_we <command>` runs one weather command on the current map, as the original
renderer's `r_we` does: `r_we heavyrain`, `r_we snow`, `r_we heavyrainfog`,
`r_we constantwind ( 0 -800 0 )`, `r_we clear` (an unknown command lists them).
It adds to the map's own weather until the next map; see
[Weather](rendering.md#weather).

`connect host:port`, `disconnect` and `reconnect` control the session.
`record`, `stoprecord`, `demo` and `playdemo` control demos.
`ghost record`, `ghost stop` and `ghost play` record and replay the player's own run
([Ghost replay](#ghost-replay)); `mimic <player> [delay] [mirror|copy] [walk]` copies another
player's moves live, `walk` to walk always ([Mimic](#mimic)).
`modelviewer` opens a model viewer (no menu entry): every installed model in a list the
search narrows (each word must be in the name, case ignored): each player and NPC body
with every skin it has, then every rigid `.md3` and `.glm` under `models/` (items,
weapons, map objects, vehicles, hats), the retail game's first. A player stands on the
stage beside the list, or in the live preview over a match, as the Collection's Shaders
tab shows the player's model; a rigid model replaces it, scaled to a player's height
and turning slowly. Type to search, Up/Down, Page Up/Down, Home/End or the wheel
choose, Escape closes. The list is read from the mounted files when the page opens
([model_viewer_scan.rs](../crates/sjk-viewer/src/model_viewer_scan.rs)); a model that
cannot be drawn is logged and leaves the stage empty. It changes no setting and draws in
the SJK UI's look in every menu style
([model_viewer.rs](../crates/sjk-viewer/src/model_viewer.rs),
[prop.rs](../crates/sjk-viewer/src/menu_stage/prop.rs)).
`screenshot` and `screenshotJPEG` request captures; `condump filename` saves
console output. A capture copies the final swapchain image into a buffer that is
mapped over the next frames; frames keep running meanwhile, and the readback stays
in [screenshot.rs](../crates/sjk-viewer/src/screenshot.rs)'s state until its own map
callback lands (dropping it earlier aborts the map). The picture is written opaque, as
rd-vanilla reads it (`GL_RGB`): the scene's alpha holds what the shader stages left (a
tinted player skin's tint mask), which a PNG would otherwise show see-through (hub
report #155). See [console registration](../crates/sjk-viewer/src/console_session.rs)
and [file commands](../crates/sjk-viewer/src/console_files.rs) for argument handling.

`tell <player> <message>` takes a slot number or, as in EternalJK, a name or a
unique part of one, ignoring case and colour codes; an exact name wins over longer
names containing it. The client sends the stock `tell <slot>` command; when no
player or several players match, it lists them and sends nothing. See
[console_tell.rs](../crates/sjk-viewer/src/console_tell.rs).

Held actions such as `+button12` work from binds, cfg files and the console. A
hold typed at the console lasts until its `-` command, as in the stock client:
opening the console, a menu or chat, or losing window focus, releases held keys
but not typed holds; `in_restart` and session changes release both. With Settings >
Controls > Move with console open (`cl_consoleMove`, off by default as in stock,
AngeloPapaya's idea, 03/11/2026), the movement keys held as the console opens (forward,
back, strafing, jump, crouch, walk) keep their hold and their key repeats are not typed;
letting one go stops it. Commands made while the console is the only thing open then
leave `BUTTON_TALK` out, which the server's movement would read as "stand still"
(`PmoveSingle` zeroes the moves of a talking command), so no talk balloon shows either.
A key pressed with the console open still types: the console keeps the keyboard, so a
command starting with w, a, s or d types as it should, and changing direction needs the
console closed. See [console_move.rs](../crates/sjk-viewer/src/console_move.rs). `+grapple` is
EternalJK's name for `+button12`, the JA+/JaPRO grapple hook; on a JA+ server
releasing it also taps `+use`, as EternalJK does. See
[input.rs](../crates/sjk-viewer/src/input.rs).

`flipkick` is JoF EJK's flip-kick bind: one press starts a run of jump taps, jump
held in one user command and released in the next, overriding a held jump key and
letting go of it when the run ends. EJK counts frames; SJK counts user commands,
which it makes every 8 ms, EJK's 125 fps frame
([networking.md](networking.md#user-commands-and-move-packets)), so one frame is
one command and `cg_fkDuration` (default 50) lasts 0.4 s.
`cg_fkFirstJumpDuration` holds the first jump for that many frames and
`cg_fkSecondJumpDelay` starts the second jump at that frame (both 0). A server
forbids it with bit 7 (`RESTRICT_FLIPKICKBIND`) of serverinfo `restricts`. See
[flip_kick.rs](../crates/sjk-viewer/src/input/flip_kick.rs).

`lowjump` is JoF EJK's low jump bind (`CG_Lowjump_f`, AngeloPapaya's idea, 03/11/2026;
Settings > Key bindings > Movement > Low jump (EJK), unbound by default): jump is held in
the next user command only and released in the one after, the shortest hop a tap can
make (EJK sends `+moveup`, then `-moveup` a frame later). It is not a cheat: the server
gets one command with jump pressed, which a quick finger sends too, and the height is
what the server's own movement gives that tap (with Force jump, letting go early cuts the
jump short). A jump key held at the same time keeps holding. See
[input.rs](../crates/sjk-viewer/src/input.rs) (`GameplayInput::low_jump`).

`weapmelee` (SJK's; Settings > Key bindings > Weapons > Melee (fists), unbound by
default, Sol's request of 10/10/2026) selects the melee weapon, the fists
(`WP_MELEE`), by name. The numbered binds cannot: `weapon 1` is the saber while the
player holds one (`CG_Weapon_f`), and with the saber already selected it switches
the saber on or off instead; the fists otherwise come only from cycling the weapon
keys. The toggle is sent only once the predicted `weaponTime` has run out, as JoF
EJK reads it, not the snapshot's, which is a round trip behind (a swing, the
holster's 400 ms); a press before that does nothing, as in `CG_Weapon_f`, and never
picks another weapon (hub report 172). A pressed `generic_cmd` (this toggle, a bound
`force_*` or `use_*`) goes on every user command until a packet takes them, as
`CL_FinishMove` keeps it until `CL_WritePacket`, so one command the server skips does
not lose it; the server's debounce takes it once.
`weapmelee` never selects the saber and never toggles: it does nothing when
the fists are already selected, when the player does not hold them, while
following someone or on an emplaced gun. The stun baton, `WP_STUN_BATON`, is another
weapon and is not chosen. Like `weapnext` it only changes the client's selection
(the user command sends it); the rule is
[`legacy_melee_weapon`](../crates/sjk-client/src/weapon_selection.rs).

Weapon selection belongs to the viewed player. When an accepted snapshot changes
that player, enters/leaves follow or spectator mode, or rewinds the timeline,
SJK resets the retained weapon choice to the snapshot's equipped weapon and
refreshes the selection display time before the next user command. This follows
JoF EJK #39's `CG_TransitionPlayerState` reset and also covers native transitions
that keep the same client slot. Old pickup/no-ammo event baselines and predicted
acknowledgements do not carry into the new view. Ordinary snapshots preserve a
pending local weapon choice; map/session presentation resets clear the retained
choice and display timer. Both HUD/menu styles share this behavior. See
[weapon_selection_view.rs](../crates/sjk-viewer/src/weapon_selection_view.rs).

`cl_idrive` is JoF EJK's "last key wins" input (archived, default 0): while both
keys of a movement pair are held, the one pressed last moves the player instead of
the two cancelling out. `1` resolves every pair (forward/back, strafe left/right
including `+strafe` with the turn keys, jump/crouch), `2` only jump/crouch; keys
pressed in the same millisecond still cancel, as in EJK. `cl_idriveDelay` is SJK's
own (archived, default 0, at most 1000): for that many milliseconds after the newer
key's press, and again after it is let go, while the older key is still held, the
pair stays neutral, so a reversal is never instant on servers that penalise one. The
delay is counted per command, so a frame that makes several commands (below 125 FPS)
spaces them out. Both apply from the next frame and are in Settings > Controls. See
[idrive.rs](../crates/sjk-viewer/src/input/idrive.rs).

`cl_forceRepeat` (archived, default 1, Settings > Controls) makes a held push or pull
key use its power again as soon as it can, as JoF EJK does: there, as in every OpenJK
client, the system's key repeats run a plain bind command again (`CL_ParseBinding` on
every press), while SJK drops them for binds. While a key or mouse button whose bind
runs `force_throw` or `force_pull` is held, every user command no press filled carries
that power's `generic_cmd`; the last such key pressed wins. The server decides when it
fires: codemp takes push and pull on every command (`lastGenCmd` spares only those
two) and `ForceThrow` refuses while the hand is still out or the power is not usable,
so it fires on the first command after the cooldown. The other powers' keys do not
repeat: speed, rage, protect, absorb and seeing are toggles a second command switches
off, and heal, mind trick and the team powers would spend the pool unasked. A key's
release stops it even when a menu or the chat took the release, losing the window
stops every key, and nothing is sent while the console, chat or a menu has the
keyboard. `+useforce` does not repeat (the server waits for its release). An alias
that runs push is not followed. The table of repeated commands, with why each is safe,
is in [force_repeat.rs](../crates/sjk-viewer/src/input/force_repeat.rs).

`in_mouseRepeat` (archived, default 0, Settings > Controls > **Hold mouse 4/5
repeats**) is EternalJK's `in_mouserepeat` (JoF EJK `sdl_input.cpp`,
`IN_ProcessEvents`): while mouse 4 is held, and mouse 5 unless the value is 2, its key
goes down again and again, as a held keyboard key's system repeats do in EJK. A bind to
a plain command runs it again; a `+` command ignores the repeated press, as
`IN_KeyDown` ignores a repeating key. `1` repeats both buttons, `2` mouse 4 only (EJK:
"don't spam mouse5"). One deliberate difference (Sol's decision, 29/10/2026): EJK
repeats once every other frame, 125 presses a second at 250 FPS and more above, each
possibly a command to the server, which servers that cap commands (JoF's) see as a
flood; SJK repeats 20 times a second at any frame rate, at most once a frame, on the
console's clock. Only a press that went to the binds repeats, only while the pointer is
the game's and no console, chat or menu has the keys; a release stops it wherever it
went, and losing the window stops both. See
[mouse_repeat.rs](../crates/sjk-viewer/src/input/mouse_repeat.rs) and its tests
(`twenty_presses_a_second_at_any_frame_rate`, `the_setting_picks_the_buttons`).

`+duck` is JoF EJK's crouch without a roll (`CG_NorollDown_f`). Its press lets go
of jump and walks for one user command, then it crouches like `+movedown`, sharing
its key state. Crouching while moving rolls only from a running legs animation
(`bg_pmove.c` `PM_Footsteps`); the walking command replaces it with the walk, and
once crouched the legs crouch instead. EJK walks with `+speed`, which runs under
`cl_run 0`; SJK walks whatever `cl_run` is. A roll on landing while crouched
(`PM_CrashLand`) is not prevented, in EJK either. Under `cl_idrive` `+duck` is the
crouch key of the jump/crouch pair, as `+movedown` is. Unlike EJK, whose `-duck`
also lets go of every `+movedown` and `+speed` key, `-duck` releases only itself. See
[input.rs](../crates/sjk-viewer/src/input.rs) (`up_move`).

A `+` bind on the mouse wheel acts on each notch, whose press and release arrive
together (`bind MWHEELDOWN +moveup` jumps once a notch; Sol's decision, 21/10/2026, after
Midn8Runner's report). As in the original game, a button (`+attack`, `+use`) pressed and
let go between two user commands is still sent in the next one (`wasPressed`). For
movement the original game counts only the milliseconds a key was down
(`IN_KeyUp`, `CL_KeyState`), and a notch is down for none, so on the wheel `+moveup`,
`+forward` and the other movement binds did nothing there, in OpenJK, JoF EJK and
ioquake3 alike. SJK moves the whole next user command for a press let go in the same
millisecond, and only for that: a key held even 1 ms keeps its share of the command.
See `KeyState::tapped` in [state.rs](../crates/sjk-viewer/src/input/state.rs).

`cg_gripVictimFX` (archived, default **on**) shows red puffs near choking players'
necks, following JoF EJK's Grip victim effect. Turn **Grip victim effects** off in
classic Settings > Game options or SJK Settings > Advanced > Gameplay to hide
them. Your viewed player's puffs stay hidden in first person and while zoomed;
other players and detached cameras retain theirs. A missing neck uses the posed
head as fallback; models with neither show no puff. The choking animation is a
visual marker, so the effect can also accompany other uses of that animation.
It changes no Grip damage, timing, targeting or movement. See
[rendering](rendering.md) for the reference and effect details.

`cp_pluginDisable` is the archived userinfo cvar a JA+ server reads to switch its
client-plugin features off for this client (most set bits disable one, a few enable
one). Its default, 1536, disables the holstered saber and ledge grab, which need JA+
animations SJK does not have. JA+ and TaystJK/jaPRO servers receive it from the
connect packet on, and a change sends a userinfo update ([networking.md](networking.md)).
Settings > Mods shows the options SJK honours as switches named after their feature
(JA+ and jaPRO plugin options), and `japlus.plugin` and `japro.plugin` list and switch
them as JoF EJK's `plugin` does; the JA+ tools mod also lists a JA+ server's options
(`japlus.serverconfig`). See [plugin options](mods.md#plugin-options).

The console input line has a caret, drawn as stock's underscore: Left and Right
move it, Ctrl+Left and Ctrl+Right by word, Home and End to either end, and Shift
with any of them selects. Backspace and Delete remove a character, or a word with
Ctrl; words end at anything but a letter or digit, so `cg_drawFPS` and
`127.0.0.1` are edited a piece at a time. Ctrl+A selects the line, Ctrl+X cuts,
and Ctrl+V or Shift+Insert pastes, replacing a selection. Typing inserts at the
caret, a long line scrolls sideways to keep it in view, and Enter runs the whole
line. Dragging the mouse over console output selects it, a double click selects
one whitespace-separated word (a whole `host:port`), Shift+click extends a
selection and a click elsewhere clears it; in the input line the mouse places the
caret and selects the same way. Ctrl+C (or Ctrl+Insert) copies selected output
without colour codes, else the selected input, else the whole input line, or the
last `viewpos` or `mark` answer when the line is empty. Up and Down stay history.
On Windows the clipboard is reached through PowerShell with UTF-8 in both
directions ([clipboard.rs](../crates/sjk-viewer/src/clipboard.rs)), so pasted or
copied symbols such as `€`, `’` or `×` keep their characters, including under
PowerShell's Constrained Language Mode (with `clip` as a last resort for copy). A
copy waits up to 1.5 s for the tool to finish and reports failure if it fails; a
paste waits for a copy still being set.
On Linux, SJK first reads and owns the X11 or Wayland clipboard directly. X11 and
Wayland compositors exposing the data-control protocol therefore need neither
`xclip` nor `wl-clipboard`; XWayland covers common remaining Wayland sessions. Those
desktop tools remain fallbacks: `wl-copy`/`wl-paste` are used only in a Wayland
session and `xclip` only with an X11 display. An installed tool for the other desktop
type is skipped, and an immediate startup failure falls through to the next clipboard
backend. Copy notices are shown only after a backend accepts the text.
See [console_editing.rs](../crates/sjk-viewer/src/console_editing.rs) and
[console_selection.rs](../crates/sjk-viewer/src/console_selection.rs). The input
line and output rows are drawn and measured on the console's character grid
([console_classic.rs](../crates/sjk-viewer/src/console_classic.rs)), so the caret,
highlights and mouse hits follow the drawn cells at any `con_scale`.

Tab completes the command or cvar name being typed, after a leading `/` or `\` and
after the last `;`. A unique name completes with a trailing space; otherwise the
input extends to the longest shared prefix and the matching commands and cvars,
with cvar values, are listed as EternalJK prints them (`PrintMatches`,
`PrintCvarMatches`): a grey `Cmd` or `Cvar` label, the white name, a cvar's value
in grey quotes and the description in green. Up to 16 matches also show their
descriptions; longer listings end with the match count instead. Typed lines, and
the line a listing completes, are echoed into the scrollback as `]cmd`, the
prompt character straight before the text as in stock. Both are shell behaviour
and the same in either console style. Enter strips one leading `/` or
`\` from the line, then applies the same completion while `cl_allowEnterCompletion`
is set, without listing when the input is already a full name. Nothing strips a
slash on a command after `;`, so completing that command drops it. Command
boundaries follow the shell's quote and escape rules, and no completion happens
inside an open quote. Tab also completes the file name after `exec`, `execq`,
`writeconfig` and `write` (`Cmd_CompleteCfgName`): the `.cfg` files at the top of
the profile's folder, then those at the root of the search paths, matched without
case and each name once. A unique file completes with a trailing space; several
extend the typed text to their shared start, keeping its letter case, and are
listed as grey `File` rows (`PrintFileMatches`). Tab after `devmap` completes the
map (`SV_CompleteMapName`) from every `maps/*.bsp` and `maps/*/*.bsp` of the game
data, named as `devmap` takes them (`mp/ffa3`): a map matches by its whole name or,
when nothing typed has a `/`, by the name after its folder, so `devmap siege_ho`
completes to `devmap mp/siege_hoth `; `devmap` and Tab lists every map as `Map`
rows. Tab after `sjkemote` completes the installed emotes' ids (`sjkemote aut`
completes to `sjkemote autumn_tea `) and the command's words `stop`, `sync`,
`mute`, `unmute` and `help`, listed as `Emote` rows; the catalogue loads on the
first Tab if nothing has loaded it yet. The client's commands with a completed
argument are the `ARGUMENT_SOURCES` table of
[console.rs](../crates/sjk-viewer/src/console.rs), one entry per command beside the
shell's own (`exec`, `devmap`). Enter does not complete arguments, only the first
argument completes, and no other command's argument is completed yet. See
[shell_completion.rs](../crates/sjk-shell/src/shell_completion.rs) and
[console_scripting.rs](../crates/sjk-viewer/src/console_scripting.rs).

F3 in the open console, or the bindable `consolebrowser` command, opens a browser of
every command and cvar with its description, and each cvar's value and default. Typing
searches names, then descriptions; Tab cycles All, Commands, Cvars and Changed (cvars
away from their default). Enter edits the selected cvar in place and applies it, or
starts a console line with the selected command; Delete restores a cvar's default;
Escape cancels an active edit first; otherwise Escape or F3 returns to the console.
The clickable Apply, Cancel and Filter controls follow the same actions as the
keyboard. Read-only cvars are listed but not edited. The
browser covers the whole frame: underlying menu shapes/text, chat and the FPS
counter are suppressed, including every font batch. See
[console_browser.rs](../crates/sjk-viewer/src/console_browser.rs).

With the classic console (`con_style classic`) the browser is drawn classic+
([classic-plus.md](classic-plus.md)): the in-game pop-up's retail box and title
band over the dimmed screen, the four filters and Edit (Insert, Apply), Default,
Filter and Close as retail gold buttons with the `menu_buttonback` glow, a retail
list box whose selected row sits on `menu_blendbox2` (a changed cvar's value in
gold with its default beside it), a detail box with the selected entry's kind,
value, default and whole description, and the description line under the box.
It uses the menus' retail font when `ui_gameFont` is on, and works without the
retail art. Keys, pointer and wheel act as in the SJK UI's look
([console_browser_classic.rs](../crates/sjk-viewer/src/console_browser_classic.rs)).

With the SJK UI's console (`con_style sjk`, or `auto` with the SJK UI's
menus) the browser has the SJK UI's look, laid out as its
Settings over the darkened frame
([console_browser_sjk.rs](../crates/sjk-viewer/src/console_browser_sjk.rs)): Esc
"Back", "Commands and cvars" and the search pill at the top; the four filters
(All, Commands, Cvars, Changed) down a lit rail with how many entries each shows
for the search; the entries (name, description, the value at the right and a
gold dot when it differs from the default; a command's kind); and the chosen
entry's detail column: its kind, name, value, default and whole description,
then Edit value (Insert for a command, Apply while typing), Default and Copy.
The detail's name, value, default and description can be selected with the
mouse: a drag selects across lines, a double click selects a word
([text_select.rs](../crates/sjk-viewer/src/text_select.rs)); the selection lasts
while the same entry is shown. In every look Ctrl+C (or Ctrl+Insert) copies the
selected text, or else the chosen entry as a console line (a command's name, a
cvar's `name value`, the value quoted when empty or spaced), and the footer says
what was copied. Mouse selection is the SJK UI's look's only; the classic+ look
copies the chosen entry.

For graphics controls and diagnostics, see [rendering.md](rendering.md).

## Text size and spacing

The Settings screen's TEXT tab holds three archived cvars, beside the console
style and the console feed (`con_drawNotify`, [above](#classic-console)). Menu text
draws as before at the defaults.

| Cvar | Default | Range | Effect |
| --- | --- | --- | --- |
| `ui_textScale` | 1 | 0.8 to 1.4 | Text size on menu screens and the in-game menu |
| `ui_letterSpacing` | 0 | -0.05 to 0.15 | Extra space after each letter of menu text, as a fraction of the text size; the console's character grid keeps its cells |
| `con_scale` | 1 | above 0 (menu: 0.5 to 2) | Size of the whole console: its character cells, margins and rows |

Menu text grows or shrinks about the centre of its line. SJK Settings' option
rows have room for the largest size; long Profile and Collection tab rows close
their gaps as the text grows. Classic+ option panels grow their rows above size
1 and show fewer at once through their existing scrolling, so their labels,
controls and pointer targets do not overlap. Other layout positions stay
unchanged. The style is applied where retained text commands become glyph quads, see
[text/style.rs](../crates/sjk-viewer/src/text/style.rs). These settings do not
affect chat, the scoreboard or the HUD. Letter spacing stays 0: Inter's average
advance relative to its x-height (0.89) is already close to stock's cells (0.8),
Inter's own size-specific tracking at this size is +0.002 em, and tighter text
would run digits and `il1` together. A profile's `con_lineSpacing` and
`con_maxLines`, which only the retired modern console read, are dropped at its
start ([Menu style](#menu-style)).

`cg_chatBoxFontSize` and `cg_centerSize` match JoF EternalJK's absolute sizes at
the same value (since 19/10/2026; before, SJK kept stock's proportions at a smaller
size, about 0.59 of EternalJK's chat and 0.44 of its centre prints). The HUD's virtual
480 lines fill the window's height, 2.25 pixels each at 1080p:

- Chat: EternalJK's `CG_ChatBox_DrawStrings` draws `ocr_a` at scale
  `0.65 * cg_chatBoxFontSize` in rows `CHATBOX_FONT_HEIGHT` 20 x 0.65 virtual pixels
  apart. `ocr_a`'s line is 21 retail pixels and its capitals 12 (`fonts/ocr_a.fontdat`
  and its atlas), so at 1 the line box is 30.7 px at 1080p, capitals 17.6 px and rows
  29.25 px. SJK draws that line box (it was 18 px), with rows 20/21 of it apart: SJK Chat
  keeps `ocr_a`'s layout, and Inter's capitals, 0.60 of its box, stand 18.5 px. A
  sender's name line advance keeps its size; the
  composer (channel tabs, draft, caret) grows by the same 1.706 so its draft stays a
  little larger than the feed's text, as before; an SJK line's GIF stays three rows
  high. See
  [chat/layout.rs](../crates/sjk-viewer/src/chat/layout.rs).
- Centre prints (drawn in SJK's headline font, Fira Sans Condensed, [rendering.md](rendering.md#menu-text)): `CG_DrawCenterString` draws `ergoec` (`FONT_MEDIUM`, a 22-pixel line,
  13-pixel capitals, point size 20) at scale `cg_centerSize` and advances
  `20 * cg_centerSize + 6` virtual pixels a row. SJK draws a 49.5 px line box at 1080p
  (it was 22 px) and the same advance, 58.5 px at 1 (it was 30); SJK Menu keeps
  `ergoec`'s layout. See [chat/view.rs](../crates/sjk-viewer/src/chat/view.rs).

`cg_chatBoxLines` (default 5, archived) is how many messages the chat feed shows
together, 1 to 24: EternalJK draws at most `MAX_CHATBOX_ITEMS` (24) items, counting
messages rather than wrapped rows, and so does SJK. Until 23/10/2026 SJK stopped at 8,
so a larger value showed no more (Creyon's report #129). The feed also stops at the top
of the screen. See [chat.rs](../crates/sjk-viewer/src/chat.rs) `MAX_VISIBLE`.

`cg_chatBoxFontSize` (default 0.85, archived; SJK Advanced > Chat > **Chat text size**,
0.15 to 2 in steps of 0.05) scales the chat's text, row spacing, the gap between
messages, the composer and inline GIFs. 1 is EternalJK's size; the default, a 26 px
line at 1080p (about 1.45 on the scale before 19/10/2026), is Sol's choice.

`cg_chatBoxLineSpacing` (default 2, archived; SJK Advanced > Chat > **Chat line
spacing** and the classic+ HUD page, 0 to 16 in steps of 1) is the room between two
messages in 1080p pixels at text size 1 (it was a fixed 8 before 19/10/2026). A
wrapped message's own rows stay `ROW_PITCH` apart.
The wrapping width stays in its configured screen position, so larger text wraps
sooner. The setting is also on Simple > HUD and the classic+ HUD page.
`cg_centerSize` (default 1, archived, console only) scales centre prints from 0 to 2;
EternalJK stops at 1, and as there 0 draws at 1.

A profile saved before 19/10/2026 keeps the sizes it showed, once
([chat/text_sizes.rs](../crates/sjk-viewer/src/chat/text_sizes.rs), marker
`cg_textSizeEjkVersion`): a saved value other than 1 is divided by how much larger the
new 1 is (1.706 for the chat, 2.25 for centre prints) and clamped to the range, so 1.25
becomes 0.733. A saved 1 was the old default and takes today's default, as in a new
profile. A profile saved later that day while the chat's default was 1 (marker 1) and
still at 1 also takes 0.85 (marker 2).

`cg_chatBoxLetterSpacing` (default -0.5, archived; also Settings > Chat > Chat letter
spacing, -2 to 8 in steps of 0.5) adds that many 1080p pixels after each character of
a message body, scaled with the viewport and `cg_chatBoxFontSize`; below zero packs
the letters closer; the default -0.5 tightens the fixed-width game chat font a
little, and 0 is each font's own spacing. Wrapping measures the same spacing (an emoji picture keeps its
own width), so a wider setting wraps sooner. Sender names, tags and the composer
keep their own spacing.

`cg_chatCardScale` (default 1, archived; 0.5 to 2; also Settings > Chat > Chat card
size, and HUD in the classic views) sizes the player card a chat name
shows while the composer is open, on top of the chat's own scale (the viewport and
`cg_chatBoxFontSize`): 0.8 draws it a fifth smaller, 1.5 half as big again. The card
stays 0.4 s after the pointer leaves the name and the card, so the way to its buttons
never loses it.

Two settings help the chat read over a bright map (Creo's idea, 29/10/2026: the chat
is often hard to read over light ground, vanilla graphics too). Both are off by
default, so the chat looks as it did until a player turns them on.

`cg_chatBoxBackdrop` (default 0, archived; Settings > Chat > **Chat backdrop (%)**, and
HUD in the classic views, 0 to 100 in steps of 10) puts a dark band behind each drawn
row of a chat message, that many per cent opaque, in the SJK UI's navy
(`menu::sjk::color::SPACE`). A band is as long as what its row shows (on a line's
first row the sender's emblems, name, tick and label, then the text and emoji
pictures, and the ellipsis of a cut last row) plus half its height at each end, which
is round; it is nine tenths of the row pitch high and centred on the row's letters, so
two rows' bands never overlap and nothing is darkened beside a short row. Its opacity is
the setting's times its line's, so it slides in, moves and fades out with its line and
goes with it; a GIF under an SJK line gets none, and every band is drawn before the
lines, so the emblem and other pictures stay over the bands. It follows the chat's size,
spacing, position, the scoreboard's chat column and the composer's history. See
[chat/view/backdrop.rs](../crates/sjk-viewer/src/chat/view/backdrop.rs).

`cg_chatBoxOutline` (default 0, archived; Settings > Chat > **Chat text outline**, and
HUD in the classic views) draws every letter of the chat feed and of the chat field
(the channel row and what you type) inside a dark outline instead of the 1 px drop
shadow: the text renderer's ring of eight copies a step out all round, as the
nameplates' Strong outline draws (`text::CodePalette::DarkOutlined`, per text run in
`ChatOverlay::palette_of`). A black letter (`^0`) keeps the faint light halo the
nameplates give dark names instead, as an outline as dark as it would hide it. It costs
nine glyph quads a letter instead of two. Centre prints, the sender card and the
combat numbers keep their shadow. A text batch (each font's layer, the Inter HUD text)
held 32 768 vertices until 03/11/2026, some 600 outlined letters, and a busy chat ran
past it: the letters after the cap were dropped, whole lines with them (Creo, hub #211).
A batch now holds up to 196 608 (`text::MAX_TEXT_VERTICES`), its GPU buffer starting at
the old size and doubling the first frame that needs more
([text/vertex_buffer.rs](../crates/sjk-viewer/src/text/vertex_buffer.rs)).

`cg_darkTextOutline` (default 0, archived; Settings > Chat > **White edge on dark
text**, shared with the nameplates) edges the dark letters of the feed and the field
(luminance under 0.3: black, dark blue, red) in a crisp white outline: alone, the rest
keep their shadow (`CodePalette::WhiteEdged`); with `cg_chatBoxOutline`, the rest keep
the dark outline (`TwoTone`), so black letters get white instead of the faint halo.

A message too long for one row wraps onto up to four, each drawn as its own text,
and every row after the first starts in the colour code in force where the row
before it ended, as if the message were one line: a green `say` (the server puts
`^2` in front of its text) stays green on every row, and a code set on one row
reaches the next. A row's carried code is kept with its range when the message is
wrapped (`Wrapped::carry`, [chat/layout.rs](../crates/sjk-viewer/src/chat/layout.rs)),
written in front of the row as `^<digit>` (`text::Carry`), which takes no room, so
no wrap moves; a code is never cut between its `^` and its digit. A message with no
codes is in the chat's base colour on every row, and the rows after an emoji picture
keep it too. The console's scrollback and notify lines carry the colour in the same
way (`wrap` in [console_classic.rs](../crates/sjk-viewer/src/console_classic.rs)).

### Percent signs and quotes in chat

The engine turns every `%` of a command into `.` on its way to the server
(`MSG_ReadString`), and a `"` would end the quoted message. Messages from the chat
composer and `tell` are therefore sent as EternalJK sends them (`Message_Key`,
`cl_keys.cpp:896-930` at JoF EternalJK bd5e202): `%` as `°/.` (the degree sign is
byte 0xB0) and `"` as `''`. The chat box shows those back as `%` and `"`, as
EternalJK's `CG_ChatBox_AddString` does (`cg_draw.c:9897-9916`), so SJK and
EternalJK players see each other's `%` and quotes; a client without this shows
`°/.` and `''`. As in EternalJK, the console's copy of a chat line keeps the
escapes, and two apostrophes typed together show as a quote. Each piece of a long
draft counts its escaped legacy bytes (`%` takes three bytes, `"` two) and stays at
or below 149 bytes; neither a Unicode character nor either escape is cut, and the
active colour is repeated at the start of the next piece
([chat.rs](../crates/sjk-client/src/chat.rs) `chat_commands`, `chat_unescape`).

### Chat emojis

`cg_chatBoxEmojis 1` (default 0, also Settings > Chat > Chat emojis) shows JoF
EternalJK's chat emojis: each `gfx/emoji/*.png` is drawn in place of its name in
messages that arrive while the option is on, and shows while it stays on. Names are
made as JoF EternalJK makes them (`CG_LoadEmojis`, `cg_main.c:2767-2827` at
bd5e202): the file name without `.png`, a backtick for `:`, `~` for `>` and `!`
before a letter for upper case, so `` `poop`.png `` is `:poop:` and `#~`!D.png` is
`#>:D`. File names over 26 characters are skipped with a warning and at most 256
load. A message replaces names outside colour codes, the first listed emoji that
matches winning, up to 32 per message (`CG_ChatBox_AddString`,
`cg_draw.c:10078-10204`); a picture is 17/20 of a row high and text after it keeps
the row's colour. A name whose picture does not decode leaves a blank. `listEmojis`
prints the names. JoF's 175 pictures ship in `japro-assets.pk3` in the `EternalJK`
folder, which SJK does not mount whole (its menus and HUD would replace SJK's); the
`gfx/emoji/*.png` of that folder's PK3s are mounted on their own, above `base`, as
EternalJK mounts its folder
([asset_search_paths.rs](../crates/sjk-viewer/src/asset_search_paths.rs)).

Differences from JoF EternalJK: its 2048-byte folder list holds about 150 of JoF's
175 names, depending on the archive's order, so the rest never show there, while all
load here; its loader switches the option off when it is on as a map loads
(`!fileCnt < 1`, a slip for `fileCnt < 1`), while here the list always loads; it
also replaces names in the sender's name, which here is drawn as text on its own
line; an empty name (a file called `!.png`) ends its matching for every later emoji,
while here that file is skipped; a picture that does not load blanks it and every
later picture of its message, while here only its own place is blank; and its folder
sits below an `fs_game` mod, whose own emoji picture of the same name wins, while
here the `EternalJK` pictures sit above every game folder, as the crosshair pictures
do. See [chat/emoji.rs](../crates/sjk-viewer/src/chat/emoji.rs).

## Vehicle and creature assets

Vehicle appearances use the shared multiplayer `.veh` parser, including its
comment handling, model/skin selection and camera fields. Example definitions
inside block comments cannot replace the real vehicle model.

Vehicle projectile trails resolve their vehicle-weapon index through `.vwp`
definitions in model-registration order. Their authored EFX and optional rigid
models are preloaded when the vehicle is registered; an effect-only laser does
not acquire the ordinary rocket model. Custom vehicle flight-loop sounds remain
a separate audio gap.

Community NPCs need their model PK3 mounted by both the server and client. An NPC
definition alone cannot supply a missing mesh. SJK does not distribute those
packs. The animation-error isolation described in [rendering.md](rendering.md#actor-animation-failures)
protects other actors from malformed custom clips, but does not repair the clip.

## NPC sabers and voices

NPCs (`ET_NPC`, entity numbers 32 and up) have no clientinfo, so their sabers and
voices come from their own entity rather than a player slot (hub report 68, a JoF
server on `JoFCrucible`). Sources are JoF EJK `codemp/cgame`.

- **Thrown sabers.** The server tags a flying saber with `genericenemyindex = owner
  + 1024` for a player and an NPC alike (`w_saber.c:6939`). The client accepts any
  owner entity number, not only the 32 client slots, and draws an NPC's thrown
  saber with its `npcSaber1` hilt and blade colour
  ([thrown_sabers.rs](../crates/sjk-client/src/thrown_sabers.rs)); before
  17/10/2026 an NPC's thrown saber was invisible while its hand was empty.
- **Blade colours.** With `boltToPlayer` zero, each NPC blade wears its `.sab`
  `saberColor` (`CG_AddSaberBlade`, `cg_players.c:8049-8064`; `saberColor random`
  is one of orange to purple, steady per NPC); before 17/10/2026 it was always red.
  With `boltToPlayer` non-zero both sabers read their packed colour, and one whose
  three bits are zero gets stock's `-1`, drawn blue (`CG_DoSaber`'s `default`).
- **No saber entity.** An NPC whose `saberEntityNum` is zero has its primary saber
  neither in hand nor lit; only a second saber is drawn (`cg_players.c:14210-14212`),
  as for a saber knocked away. Players are not checked this way yet
  ([npc_identity.rs](../crates/sjk-client/src/npc_identity.rs),
  [actor_world_submission.rs](../crates/sjk-viewer/src/actor_world_submission.rs)).
- **Voices.** `*pain25`, `*death1`, `*jump1` and the other custom sounds of an NPC
  (or a vehicle, which is an NPC) come from its own sound folders: the server names
  each as `*$<dir>` in `CS_SOUNDS` and points the NPC's `csSounds_Std`, `_Combat`,
  `_Extra` and `_Jedi` at it (`NPC_stats.c`, `CG_HandleNPCSounds` in
  `cg_servercmds.c:718`). As `CG_CustomSound` (`cg_players.c:191-322`), each name
  is looked up in the four tables in order (standard, combat, extra, Jedi) and
  played from `sound/chars/<dir>/misc/<name>` of the set the NPC names for that
  table: death, jump, pain, falling, choke, gasp, land and taunt from the standard
  set (its choke wins over the combat duplicates), anger, victory, confuse,
  pushed and `ffwarn`/`ffturn` from the combat set, chase, cover, detected, lost,
  outflank, escaping, giveup, look, sight, sound and suspicious from the extra
  set, and combat, jdetected, taunt1-3, jchase, jlost, deflect, gloat and
  pushfail from the Jedi set. The vocal events 116-124 and 131-132 (combat), 133-168
  (extra) and 169-189 (Jedi) play them, as do `*name` entries in the sound table.
  An NPC with no set for a sound, or whose file is missing, is silent: unlike a
  player it has no `mp_generic` fallback and never borrows the player duel bank.
  Sets load when their `CS_SOUNDS` slot changes, not per event
  ([npc_sounds.rs](../crates/sjk-client/src/npc_sounds.rs)). Before 17/10/2026
  every NPC spoke with the voice of player slot 31.
- **Who speaks.** As stock, `EV_ENTITY_SOUND` (79) speaks with the entity its
  `clientNum` names (`cg_event.c:4174`): `G_EntitySound` sends a pit's `*falling1`
  and Grip's `*choke1-3` and `*gasp` this way, on a temporary entity, so they keep
  the player's voice (or the NPC's, when it names one). Every other event speaks
  with its own entity number, an NPC's banks above the player slots (a forwarded
  event borrowing those of the NPC it names), so a `*name` on a temporary entity's
  `EV_GENERAL_SOUND` stays silent as in stock. Voice commands (75) keep
  `groundEntityNum`. The sound's source stays the speaker's own number, so a
  blocked player's voice stays muted.

## Shader remap controls

Server map recolors and material replacements are off by default since 18/10/2026
(Sol's decision): `cg_serverRemaps` (Settings > GAME > "Server remaps", on/off,
default off) lets them apply, and the release that brought it asks every player on
its what's-new card. With it on, `cg_remaps` filters them as in EternalJK: SJK
defaults to `cg_remaps 2`, EternalJK's default, which includes player textures.
`cg_remaps 1` excludes `models/players/` sources from both configstrings and
reliable server commands, after JoF EJK #297; `cg_remaps 0` disables server
remaps. Worldspawn and your explicit local console remaps remain independent.
Settings > GAME > "Shader remaps" sets the same cvar; both apply at once, and
`listRemaps` says when server remaps are off.
`cg_remapsBlockedMaps` lists maps where server remaps are ignored, as with
`cg_remaps 0` on those maps only: map names separated by spaces, commas or
semicolons (`mp/ffa4 mp/duel6`; `maps/` and `.bsp` are optional, capitals do not
matter; typed in the console, a list with semicolons needs quotes). `blockRemaps [map]` adds a map and `unblockRemaps [map]` removes it, the
loaded one when no map is named; the change applies at once, and `listRemaps` says
when the loaded map is blocked. The list is saved with the other settings.
`cg_remapsAllowedMaps` is the other way round: maps whose server remaps apply,
filtered by `cg_remaps`, even while `cg_serverRemaps` is 0; `allowRemaps [map]` adds a
map and `unallowRemaps [map]` removes it. A map in both lists is blocked.
Settings > GAME > "Remaps on this map" (`cg_serverRemapsMap`, not saved) shows
and sets the loaded map's choice: `default` (as `cg_serverRemaps`), `on` (in the
allowed list) or `off` (in the blocked list). It is rewritten from the lists on each
map, and setting it moves the map between the lists, so each map keeps its choice.
Settings > GAME > "SJK remap blocks" (`cg_remapsShared`, on by default) follows a
list of maps the SJK team keeps on the hub (since 05/11/2026), where server remaps are
ignored as with `cg_remaps 0`. The game keeps the list it last received in
`cg_remapsSharedMaps`, so it applies from the start. A map you set `on` or `off` in
"Remaps on this map" keeps your choice; only `default` follows the shared list, and
`listRemaps` says when the shared list blocks the loaded map. SJK's creator changes the
list with `syncBlockRemaps [map]` and `syncUnblockRemaps [map]`.
A map's own worldspawn remaps always apply. `listRemaps` lists the map's, the
enabled server and the temporary local remaps in the order they were applied, each
labelled `map:`, `server:` or `local:`, and marks those a later remap overrides.
`remapShader <old> <new>` replaces a shader locally for the loaded map; remapping
it to itself restores the original. The latest remap of a shader wins, so a later
server remap replaces a local one. `clearRemaps`, EternalJK's renderer command,
drops every active remap whatever its origin, the map's included, without telling
the server; the world draws its own shaders until the server sends new remaps (a
shader-state change or a `remapShader` command) or a new map loads. As in
EternalJK, destination time offsets are kept. See [rendering](rendering.md#server-shader-remaps)
for scope and remaining limitations. These controls do not edit map geometry.

### Incoming chat colour

`cg_chatColor` forces only the game server's public **All** message bodies.
Team chat, plain/encrypted whispers, admin chat and the separate SJK chat channel
(including reward drops and official notices) retain their original colours,
even when they appear together in the All feed. The setting is local and archived:
it never changes sent text or the chat log. Sender names keep their own colours,
and emoji pictures, channel marks, timestamps, centre prints and the composer
keep their normal appearance. Existing public history changes without reconnecting.

The default 0 keeps original colours; 1 red, 2 green, 3 yellow, 4 blue, 5 cyan,
6 magenta, 7 white, 8 orange, 9 grey and 10 black force one colour, ignoring inline
colour codes in the body while retaining message fading. Invalid values fall back
to original colours. Settings > Chat offers the same named choices (also in classic
HUD settings). It is independent of outgoing `cl_colorstring` and chat cleaning.

### Copying a server address

The SJK and classic server browsers offer **Copy IP** for the selected server.
Ctrl+C copies the selection while no text field or dialog is open; in a field it
still copies that field. The copied endpoint includes the port (and brackets for
IPv6), with no password or command. The browser reports success or clipboard failure.
`copyServerIP` copies the connected server's endpoint from the console; disconnected
clients report that no server is connected.

[SJK browser with Copy IP](images/copy-server-ip.png).
