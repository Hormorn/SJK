# SJK UI

The SJK UI (`ui_menuStyle sjk`) is SJK's own menu style: SJK's menus redesigned
from the ground up, drawn over the live map in SJK's own type. It keeps the
feeling of Jedi Academy's menus (gold for what you choose, holo blue line-work,
the turning ring round the emblem) and drops their window frames and boxes. It
is SJK's default menu style since 08/10/2026 (Sol's call, before every screen
had its version); [classic+](classic-plus.md) stays a choice and keeps getting
fixes. A profile saved with the old default `classic` moves to `sjk` once
([client.md](client.md#menu-style)).

Status (08/10/2026): the main page, Settings (with the key bindings),
Character, What's new, Update, Identity, Credits, Servers (the server browser),
the loading screen, the scoreboard, the in-game menu (reworked 09/10/2026), the
Profile screen, the Collection (10/10/2026), Camera control, the map editor's panel (19/10/2026), the report and note
dialog (Report a bug), the new medal pop-up and Create game with its map list
(10/10/2026, in every menu style) are done. Every other
screen opens in its classic+ version, which
covers the map as the classic style does.
The other style: First setup's Menu style row, Settings > Interface > Menu style, or
`ui_menuStyle classic`; restart for the style's map behind the main page.

**Every feature has its SJK UI (Sol's rule, 11/10/2026).** Anything a player
sees or uses that a change adds (a screen, page, pop-up, overlay, setting row or
browser) comes with its SJK UI version in the same change; a classic+-only
version is not accepted. The opposite is fine: a feature can exist in the SJK UI
alone, and then it draws its SJK look in every menu style, as Profile, the
Collection and Create game do. A new setting goes in the settings catalogue and
one of the SJK Settings categories; a new page is drawn with the kit
(`menu::sjk::kit`). A contribution that arrives with only a classic+ view gets
its SJK UI before it is merged. Fixes to an existing classic+ screen alone are
fine.

Loaded JoF tools can show a server-authored conversation in an SJK modal in every
menu style, only on a recognized JoF server: speaker and
body above five scrolling response rows, gold selection, a Leave conversation
control and keyboard hints. [JoF dialogue support](mods.md#dialogue-window-and-node-audio)
describes the server-owned quest boundary and voice lifecycle.

## Design

Sol chose the direction on 07/10/2026 from three mock-ups (a design canvas of
the main page in three directions, a Settings screen and the kit). The main page
is direction A, the ring: the emblem in a turning ring and the menu on an arc
round it, with the servers the player joined last in a column on its right, over
direction C's map (mp/duel6). (A first build combined A's ring with C's horizon
line and B's servers under Play; Sol went back to A the same day.)

Principles:

1. **Content floats on the world.** No boxes or window frames: structure comes
   from alignment, thin lines and dark fades at the screen's edges, over the
   live map.
2. **One memorable thing per screen.** On the main page, the ring and its gold
   arc, which turns to point at the chosen entry. Everything else stays quiet.
3. **Gold is the choice.** Gold marks what is chosen and the one main action of a
   screen; ember is only for leaving. Holo blue draws lines, never text that
   matters.
4. **Words, not capitals.** Navigation and labels are in sentence case.
5. **Keyboard first, pointer equal.** A scrollbar drawn with `MenuCanvas::scrollbar` (or
   registered with `scrollbar_area`, as Settings' lists are) is taken and dragged
   as well as wheeled (Foshin, 03/11/2026): its grip is at least 16 pixels wide, a
   press on the thumb keeps the point it took, one on the track brings the thumb
   there, and the screen passes its pointer events to `MenuCanvas::scrollbar_drag`
   first (the Profile screen's model grid and hilt lists, Settings' open lists).
   Every screen works with the arrows, Enter
   and Escape alone; hovering chooses, a click acts.
6. **One brand.** The colours and type are SJK's site's
   (`site/assets/style.css`), so the site, the release notes and the client
   read as one thing.

### Colours

Display values, like all 2D colours ([rendering.md](rendering.md)), in
`menu::sjk::color`:

| Token | Value | Use |
| --- | --- | --- |
| Space | `#060A14` | The ground and the fades; never pure black |
| Holo | `#A8CFFF` | Line-work: rules, ticks, outlines, the ring |
| Gold | `#E8B84A` | What is chosen; the screen's one main action |
| Gold bright | `#FFD97A` | Chosen text and marks |
| Text | `#E2E9F6` | Text |
| Muted | `#A9B5CB` | Secondary text |
| Quiet | `#8792AC` | An item that ends the session |
| Ember | `#FF7A3D` | Leaving: Quit |

### Type

Two bundled vector families, as on SJK's site, both under the SIL Open Font
License (licences beside the fonts in `crates/sjk-viewer/assets/fonts`):

- **Rajdhani** (SemiBold, Bold), the display family: navigation, titles, key caps,
  numbers. It lacks a few symbols (superscripts, fractions, ordinals, ¤, µ),
  which come from Exo 2 SemiBold.
- **Exo 2** (Regular, SemiBold), the body family: details, descriptions,
  everything else. Its static weights are instances of the variable font
  (`fonttools varLib.instancer`), since the rasterizer reads only a variable
  font's default instance.

Sizes are in pixels of a 1080-line screen and scale with the window's height:
the main page's entries 44 (chosen 68), server names 27, the player's name 26,
details 16 to 18, key hints 15 to 17; on Settings, its name 48, the categories
26, sub-headings 22, row names and descriptions 19, the detail's title 32 and
its facts 17. `ui_textScale` multiplies them from 0.8 to 1.4; Settings > General
in Simple or Interface in Advanced exposes it as **Menu text size**.

Names keep their colour codes (`^1`...), drawn in a palette lifted to read on
the navy ground (`text::CodePalette::Legible`, which every text in the UI's
families uses): black shows as grey, and red, green, blue and magenta are
lighter with the same hue; the other colours are the game's.

`ui_gameFont` (classic game fonts) does not change the SJK UI's type; it keeps
applying to the classic screens and the in-game text.

Text is centred on its rectangle's middle line (`menu::sjk::text`): the
renderer sets a run's line box from the rectangle's top, so the helper places
it from each family's measured metrics, the middle of the capitals leaning a
quarter toward the lower case (0.489 of the size down the line box for
Rajdhani, 0.563 for Exo 2; a test checks them against the bundled fonts).
Every SJK UI text goes through it, so a control's text rectangle is simply the
control's own.

### Motion

The ring turns once every four minutes and the sunburst behind the emblem
slowly the other way; the gold arc eases towards the chosen entry (about 0.3 s);
the camera behind glides through its tour (The map behind); a Settings slider
held over a match fades the screen round it away (0.15 s, after a 0.3 s hold); on the loading
screen a gold arc turns once every three seconds and the destination's
levelshot fades in (0.45 s); on Credits the sun (its sunburst, god rays and
sparks) keeps turning and rising, and the page's rows rise into place as it
opens or a fold unfolds; on the Collection's Shaders an owned blade skin's swatch
lives (its corona breathes, flame loops rise and a flare runs along it, as its file
says) and the model on the stage holds the chosen blade;
a new medal arrives in its ceremony and, the screen's one memorable thing, breathes
gently while it waits ([New medal](#new-medal)). Nothing else moves on its own.

## The map behind

With Menu pictures on (Settings > Interface, `ui_menuPictures`, to save video
memory; on by itself after a map runs out of it) the map behind is pictures of it
instead: one per tour shot, halfway through its glide, and one per shot the camera
flies to, crossfading in 0.9 s where the camera would cut or fly
([pictures.rs](../crates/sjk-viewer/src/menu_backdrop/pictures.rs)). The player on
the stage, the thrown saber and the holocron are still drawn in 3D, from the
picture's camera. Rerun `menu_pictures_generate`
([world_shot_menu_pictures.rs](../crates/sjk-viewer/src/world_shot_menu_pictures.rs))
after a shot or the look changes.

Behind the SJK UI the camera tours mp/duel6: ten authored shots
(`YAVIN_TRAINING_TOUR` in
[routes.rs](../crates/sjk-viewer/src/menu_backdrop/routes.rs), keyed by the map's
worldspawn message "Yavin Training Grounds"), each a 15-second glide from one
point to another while looking at a third, with a 0.9-second fade through the
UI's navy between them ([tour.rs](../crates/sjk-viewer/src/menu_backdrop/tour.rs)).
The tour opens on the map's own intermission view (the west wing's room looking
down the corridor at the tower), then plays the rest in a shuffled order that
changes every start and every round, never showing the same shot twice in a
row. The shots: the tower from three gardens, high; the east and south wings'
rooms; the tower from its foot; over the courtyard's wall at its base; the
south temple's stair; a diagonal path from the tower. They were placed with the
off-screen world shots and checked to glide through open air (the north wing's
view, full of leaves, and the west garden's, half a wall, were dropped).

On a map with a tour, a screen whose shot has a route is reached by fading out,
cutting to the shot and fading in, instead of flying there, since a tour's
shots are all over the map; every screen shows at once. The fade is a layer
over the world only (`ClientMenu::world_fade`), under the menus and the
console's pages. duel6 has two such shots, both on the south-west path from the
tower, between its stone benches, where the player's model stands
(`YAVIN_TRAINING_PLAYER`, floor at z 352, facing the camera): the player shot,
80 units off and turned so the model stands right of the screen's middle with
the tower behind; and the saber shot (`YAVIN_TRAINING_SABER`), a step closer,
the thrown saber floating between the camera and the model.

## Main page

[home.rs](../crates/sjk-viewer/src/menu/sjk/home.rs), drawn over the live map,
which is mp/duel6 when the client starts in the SJK UI
(`MenuStyle::boot_map`). It is laid out on a 16:9 frame of 1080-line pixels
centred in the window: a wider window shows more map at its sides, a narrower
one (4:3, 5:4) scales the frame down to its width.

- **Ring:** SJK's emblem (380 across) in a ring of tick marks, 312 in radius,
  centred at (620, 540). The ring is a light picture drawn by the emblem worker
  (`EmblemLayer::Ring`), tinted holo; a gold arc (24 degrees) on it points at the
  chosen entry, a dotted gold beam reaching towards it, and turns to the servers
  while they have the keyboard.
- **Arc:** the page's entries on an arc of radius 440 round the ring's right
  side, 16 degrees apart (12.5 for six, so the ends keep clear of the ring); the
  chosen one gold and larger, with a line saying what it opens. The pages:
  - Main: Play, Profile (the [Profile screen](#profile-screen) on its Character
    tab; it read Character until 09/10/2026), Collection (the
    [Collection](#collection), since 10/10/2026), Settings (the settings screen),
    SJK, Quit.
  - Play: Join a server (the browser), Create a game, Back.
  - SJK: What's new (the changelog), Update, Credits, Report a bug (the
    [report card](#report-a-bug-and-its-dialogs) over the menu, since 03/11/2026, Lex's
    idea), Back (the Profile screen is
    the main page's Profile, and its profile card). The arc holds six entries (a
    seventh curls back into the ring); the identity's key is in Settings, Network.
  - Quit: Quit to desktop (ember when chosen), Stay. It opens on Stay.
  Sub-pages have no title over the arc; their entries say where the player is.
- **Recent servers:** a column on the right (its rule at x 1470) of the servers
  the player joined last, newest first, up to four, kept in
  `recent_servers.json` beside `favorites.json` ([recent.rs](../crates/sjk-viewer/src/menu/sjk/recent.rs)):
  a server is recorded when a join reaches the game, with its name and map,
  except a game this client hosts. A map change updates that server's one row;
  the loading screen's map name is never treated as another server, and invalid
  rows written by older builds are ignored. Each shows its name (live from the server
  list, or as last seen), its players, map and ping while the server list has
  it, and when it was last played ("2 hours ago", "yesterday": relative, so no
  time zone is needed). Before any join, the column offers the JoF server
  (`135.125.145.49:29070`) under "Start here".
- **SJK chat:** docked under the servers (from y 700, [hub-chat.md](hub-chat.md)):
  "SJK chat" and the online count, the last messages sitting on a field in a box five
  one-row messages high, newest at the bottom (each the name, the verified tick for a
  verified sender and the text in the SJK chat's gold, wrapped onto as many rows as it
  needs since 10/10/2026, Sol's request: a long message pushes older ones out), the
  field, and Open chat in gold under
  it, which opens the SJK chat page. The same dock, drawn by the same code
  ([chat_dock.rs](../crates/sjk-viewer/src/menu/sjk/chat_dock.rs)), sits in the
  [in-game menu](#in-game-menu). Resting the pointer on a name shows the sender's
  [sender card](#sjks-pages) with Block
  ([hub-chat.md](hub-chat.md#blocking-a-player)), left of the dock, level with the name
  and above the keys (shapes never cover text: beside the name it lay over the message
  and the lines under it). It keeps clear of the dock, the keys, the arc's labels and
  the profile card bottom left in every window
  (`the_sender_card_keeps_clear_of_the_profile_card`); its
  targets are the page's own (60 to 64 the names, 70 the card, 71 Mute).
  Down past the last server reaches the field
  (the gold arc turns to it); Enter types and every key goes to the field until Enter
  sends or Escape stops. Not shown with `cl_sjkChat 0`.
- **Support SJK:** top left, clear of the ring (x 96 to 526, from y 58;
  [support_links.rs](../crates/sjk-viewer/src/support_links.rs), since 26/10/2026, Sol's
  request; it took the place of the donation goal shelved on 14/10/2026,
  [sjk.md](sjk.md#donations)): "Support SJK" in muted print, then two outlined pills,
  GitHub Sponsors and Ko-fi, each opening that page in the browser, and a round × that
  hides the line for good (`ui_supportLinks 0`). While × is under the pointer, a quiet
  line under them says that Settings, Interface, "Support SJK" brings it back. It asks
  no server anything. Pointer only: its targets are the page's 80 and 81 (the links)
  and 82 (×). Snapshot `sjk-home` (`menu_snapshot::sjk_home_snapshot`) and world shot
  `duel6-menu` (`world_shot::duel6_sjk_menu`).
- **Corners:** the player's [profile card](#profile-card), bottom left; the keys
  of the page, bottom centre; the version and a newer release found, bottom right.

Keys: Up and Down move along the arc; Enter takes the entry; Right (or Tab)
moves to the servers, Up and Down choose one, Enter joins it, Left or Escape
returns to the arc. Left from the arc chooses the profile card (the gold arc turns
to it), Enter opens the Profile screen on its SJK Profile tab, and Right, Up, Tab or
Escape return to the arc. Escape on a page returns to the main page, on the entry that
opened it; on the main page it opens Quit's page. The pointer chooses by
hovering and acts with a click. A server with a password opens the browser's
password prompt.

### Profile card

[profile_card.rs](../crates/sjk-viewer/src/profile_card.rs) (08/10/2026, Sol's
request: the name and circle in the corner show the SJK profile, with a picture, and
open it). Bottom left on the 16:9 frame, from (84, 932), 474 by 96:

- the player's picture, a disc 72 across at (96, 944): their picture once it is
  loaded, else the first letter of their name (past colour codes and symbols) on a
  colour picked from their key; ringed holo, gold when verified, bright gold when the
  card is chosen, with the verified badge at its foot for a verified key
  ([identity.md](identity.md#pictures));
- their name with its colours (Exo 2 at 27);
- a line of what the SJK hub says: "Verified" (gold) or "SJK player", the medals
  given ("2 medals", left out with none) and the achievements unlocked ("12/34
  achievements", of the obtainable ones and the retired ones held); without the hub it says "SJK identity off", "Connecting to the hub"
  or "SJK hub out of reach", still with this PC's achievements;
- their model and blade ("Kyle, blue saber", or "Kyle, Sun blade" while they wear a
  blade skin their profile holds, [unlockables.md](unlockables.md)); when the card is
  chosen, "Open your profile: picture, bio and record" in gold instead.

The pointer chooses it by hovering (a dark band with a gold edge round it) and a click
opens the Profile screen on its SJK Profile tab. What it says is gathered twice a second from the settings,
the identity service and the achievement counts (`profile_card::refresh`); a frame
reads it in place. The same card stands bottom left in the in-game menu.

## Character

[player_menu/sjk_view.rs](../crates/sjk-viewer/src/player_menu/sjk_view.rs): the
player screen, the [Profile screen](#profile-screen)'s Character, Saber and Force
tabs, opened by the main page's Profile and the in-game menu's Profile. The model
stands on duel6's
stage in the map's own light, holding the saber draft lit, and every change
shows on it at once (`menu_stage`). The screen is laid out
on the 16:9 frame, dark behind the form on the left and clear over the model.

- **Top:** the way back (Esc, "Main menu") and the player's name, with its
  colours, as the title; under it the pages as tabs, Character, Saber and
  Force, the one on show gold and underlined. As the Profile screen (the SJK UI
  always opens it so) the row holds the screen's four tabs instead
  ([Profile screen](#profile-screen)).
- **Character** (Sol's request, 12/10/2026: "a better character creator and also hat/cape
  picker, not just arrow things with text"; [sjk_creator.rs](../crates/sjk-viewer/src/player_menu/sjk_creator.rs)):
  Name (a field; Enter types) and Search (a field that filters what is listed), then a
  strip of categories, Model, a species' Head, Torso, Legs and Skin (only while the
  model is a species), Hat and Cape, over one grid of tiles for the one on show, and
  the Apply button. The tiles: the models' icons (eight a row), a species' part
  pictures, the skin colours as swatches, the hats and capes as tiles four to a line
  (None first); the one chosen is ringed gold, the pointer or the keys' tile white,
  the wheel scrolls. A title line names the category and counts what is listed (and
  "found" under a search); the models' line holds the team (Default, Red, Blue)
  as segments. **Customizable first** (Blep's idea, 06/11/2026: players did not find
  the species among the other models): the retail game's species (models built from a
  head, torso, legs and skin colour, as single player's character) come first under a
  "Customizable" heading with its count and "Choose the head, torso, legs and skin
  colour" beside it, and every species' tile, the community's too, carries a gold
  badge with three sliders in its top right corner (the classic menus' grid keeps
  retail's order, species after the characters). **Jedi Academy first:** the
  retail game's other models follow under a
  "Jedi Academy" heading with a count, then the high quality models (SJK rig models,
  [rig-models.md](rig-models.md)) under "High quality" when one is installed, then the
  rest of the community's under "Community" (the rule in [AGENTS.md](../AGENTS.md);
  `LegacyAssetCatalog::retail_characters`, `LegacyCharacter::rig`). The grid opens on
  the worn model: one named without a skin is its default skin (as the game reads it),
  and one the catalogue lacks opens on the first Default skin, never a team's.
  Keys: the rows are Name, Search, the strip, the team (models), the grid and Apply;
  Left and Right step the strip or the tile, Up and Down the tile by a line (and leave
  the grid at its ends), Enter on the strip goes to the grid, on a tile chooses it. The
  Model row that stepped to the next model of the grid is gone (Sol's decision,
  11/10/2026): the grid chooses the model. The Head row that drew as a model cycler
  after that removal is the Head category now.
- **Apply** (a friend's request, 12/10/2026): a model chosen (the grid, a species' head,
  torso, legs or skin colour, the team colour) shows on the stage at once but is not worn:
  the `model` and `char_color_*` cvars keep the old one until the button under the grid
  ("Apply", "Applied" and dimmed when they are the same; the Apply row, chosen after a
  pick so Enter wears it) writes them (`apply_look`). Leaving the screen without it
  drops the choice (`discard_look`, from `settle_force_page` once the screen is not
  shown). The name, hats and capes are written at once as before, and the classic
  menus write the model at once.
- **Turning the model** (Sol's request, 10/10/2026: "rotate model with mouse click";
  [sjk_outfit.rs](../crates/sjk-viewer/src/player_menu/sjk_outfit.rs)): opened from a
  game, on every page, dragging over the model turns it (about 0.45 degrees a pixel):
  the live preview's camera goes round it. Under the model's feet, drawn as the
  screen's keys are, "Drag" in a key cap and "turn the model" say so on every page.
  The turn holds from page to page; opening the screen again faces it front. On the
  menu map's stage the model does not turn (Creyon's tests, 12/10/2026: a raised blade
  turned end on to the stage's camera looked far too long). Where the model turns, it
  shows its back to the camera on the Cape category, and its front on the others. The
  chips picker of Creyon's #97 (10/10/2026) is the Hat and Cape categories now.
- **Saber:** Style as three buttons (Single, Staff, Dual); the hilts as a list
  under a sub-heading (Hilt, or Staff), in two columns, six lines on show,
  scrolled by the wheel or its bar, the chosen one gold with a dot and kept in
  view (in the middle when the list first shows); for Dual two lists side by
  side, Right hand and Left hand, three lines each (four until 23/10/2026). The hilts come in the game's
  load order, as JoF EJK lists them (`legacy_saber_load_order`), the retail game's
  first under a "Jedi Academy" line with their count and the community's after them
  under "Community" (`HiltLines`; the rule in [AGENTS.md](../AGENTS.md)). A hilt
  with no mesh (a community "empty hilt", only its `*blade1` tag, for a stance with the
  blade alone) shows its blades and no hilt; a hilt the stage cannot show is logged and the
  hand holds `single_1`, and one the game closed while loading is left out at the next
  start (`crashes/stage-hilt.txt`, `menu_stage/hilt_guard.rs`; hub report #214). The hilt search
  (10/10/2026, Sol's request) is a field at the right of the lists' heading line
  (beside Left hand for Dual), drawn as the Character page's Search ("Search
  hilts", then the words typed and "N found"): Enter or a click types in it, and
  the lists show only the hilts whose name or file name holds every word typed,
  ignoring case ("No hilt matches" when none does); one search filters both of
  Dual's lists, and Left and Right on a list step through its matches. Escape
  while typing clears it, as on Character; a search kept with Enter clears with
  Escape, and the next Escape leaves. Under the lists the Blade row (10/10/2026,
  Sol's request): the stock blade in the first blade's colour, then every blade
  skin the player's hub profile lists, each the Collection's swatch shrunk to 79
  by 44 (moving as there; [unlockables.md](unlockables.md#saber-tabs-blade-choice);
  a chroma in the first blade's colour, with its colour wheel in the swatch's top
  right corner, [unlockables.md](unlockables.md#chromas)),
  the one worn ringed gold, and under the row's name the name of the one under
  the pointer or worn; six show at a time (10/10/2026), scrolled so the worn one
  stands third where it can, with a gold chevron at a side that has more; with no skin owned, or the identity off, only the stock
  blade and a line saying why. Under it the Sounds row (23/10/2026, after Nib's idea): a
  Preview button, then a ‹ › control with the saber sounds chosen apart from the blade,
  "Follow shader (<the blade worn>)", "Stock sounds" or an owned shader's name
  (`cg_saberSounds`, [unlockables.md](unlockables.md#saber-sounds)); Preview, or Enter on
  the row (the keys line then says "Enter preview"), plays the choice's ignition and hum.
  Then the blade's colour as eight chips (the six
  stock colours, the custom one, which says "RGB" so that its tint is not taken for a
  stock colour or black, and last JA+'s black, colour 11, since 27/10/2026, hub report
  #130; ringed gold when chosen; the model then holds black blades, drawn as in game,
  not as the near-black tint, which the additive blade pass showed white), its red,
  green and blue sliders (a digit types the number), and the second blade's for Dual.
  The camera cuts to the saber shot, where the model throws the saber to float
  and turn before it. World shot: `world_shot::saber_skins::duel6_sjk_saber_page`
  (the search typed, a search finding nothing with no skin owned, the Sounds row, Dual; at 1080p,
  with `ui_textScale` 1.2, and at 4K with it).
- **Force:** the **Force templates** panel at the right offers separate
  **Jedi Academy** and **Community** selectors, retail first. Left/Right or
  either half of a selector loads that group's previous/next preset into the
  draft; Apply sends it. The community group includes the bundled JoF presets
  ([client.md](client.md#menu-style)). The mastery ("Jedi Master") and points left
  ("18 of 100 points left", and "not applied yet" while the draft differs)
  sit over a gold bar of the points left; the Light and Dark sides appear as two cards
  with their emblems; the powers in classic+'s groups under sub-headings,
  Neutral and Lightsaber down the left, the side's five on the right (the
  other side's left out, as on classic+): each its holocron, its name and its
  three levels as classic+ draws them (retail's `forcecircle` and `forcestar`,
  drawn by the UI): round marks numbered with what each level costs (0 when
  free), a ring until bought, a disc once bought, on a channel lit up to the
  power's level. Each group has its colour, its sub-heading's too: Neutral
  silver, the light side blue, the dark side red, Lightsaber green
  (`power_tint`). The bought discs glow. Hovering a level beyond the power's
  charges the channel up to it, a white spark running along it, the marks it
  would buy breathing; the points it would take show white at the bar's end
  (ember, marks and bar, when there are too few). A click on a level buys up to
  it, a right click on a level removes it (the power stands one below it); a
  level bought (by click or Right) sends a ring out from its mark. These are the only things that move on the
  page, and only answering the pointer or a purchase. Then Apply (gold while there
  is something to apply) across the left column under the Lightsaber group, with
  Start over and Discard side by side below it, clear of This server's lines. On a server, a power it
  turns off, holds at a level or does not use keeps its marks and has an ember
  note under its name (Off on this server, Fixed on this server, Team games
  only), and under the side's five, This server lists its rules: the highest
  rank and its points, free saber skills, the powers it turns off (past eight,
  how many), whether team powers work, and that turned-off powers stay
  pickable for full Force duels and an applied profile counts from the next
  respawn ([client.md](client.md#force-profile-on-a-server)).
- **The model shows the power** (12/10/2026, Creyon's idea; opened from a game only,
  where the live model stands): on the Force page the model beside the form shows the
  power under the pointer, while the pointer is on its row or one of its level marks,
  and stops when it leaves ([force_pose.rs](../crates/sjk-viewer/src/player_menu/force_pose.rs)).
  The picture fades out over its outer edge, so a beam, a puff or a blade reaching past
  it softens away instead of ending in a square cut.
  The preview is a picture of the model, not of the map the player is on: the model is
  lit as retail lit a menu model with no world (`R_SetupEntityLighting` without a light
  grid), and its effects play in an empty world of their own, so it looks the same on
  every map. The stage's pose is blended between animation frames (most clips run at
  20 a second), so it moves smoothly at any frame rate. The legs and the torso play their own
  animations ([menu_stage/tracks.rs](../crates/sjk-viewer/src/menu_stage/tracks.rs)), and
  a new one takes over from the pose on screen when it begins, held still, as Ghoul2
  blends from the frame it froze, so nothing steps or jerks, and a takeover begun while
  another is under way carries on from where that one is (dropping it snapped
  Lightning's three-frame start, Creyon, 13/10/2026): over 0.25 s, eased in and out,
  when the pointer moves to another power, and between the saber skills' moves as below. Each joint goes
  straight from one pose's place to the next's, as when Ghoul2 blends the matrices entry
  by entry, and the bone turns about it the short way at an even pace (its quaternions
  blended, as a GLA frame stores them; a GLA bone matrix carries the model's points, so the
  turn is about the joint's bind-pose place): a swinging arm and body neither shrink as if
  folding nor come off their joints (blending the turn alone while the matrix's offset went
  straight stretched the arms out and through the body, Creyon, 13/10/2026; a bone that is
  not a plain turn keeps its blended matrix squared up to its own axes' lengths and
  handedness). Between the frames of one animation (close together) each bone blends in
  its parent's frame; a takeover from one animation to another blends the two whole poses
  bone by bone as they stand in the model: two poses can hold a bone alike in the model
  but reach it differently (the game's swing starts turn the upper arm a half turn about
  itself from the stance's and the forearm back), and blended in the parent's frame the
  arm and saber swung a full turn round between them (Creyon, 13/10/2026: the saber
  flickering into the second swing; measured on the game's own animations, the blade's
  tip travelled about 315 units for a 13-unit move). Each animation keeps the speed it
  began at, so one
  fading out does not jump when the next plays at another. A held gesture (Grip's, Lightning's,
  Drain's) stays held while the pointer is on the power, as in the game.
  The gesture plays from a script ([menu_stage/force_motion.rs](../crates/sjk-viewer/src/menu_stage/force_motion.rs)
  `script_at`): a one-shot gesture once, straight back into the stance at an even pace
  over 0.3 s (keeping Push's and Pull's last frame stopped the hand dead before it went
  back to the saber, Creyon, 13/10/2026), then the stance for 1.5 s before it plays
  again, and the power's effect and shell only while it plays (back to back, Push, Pull,
  Mind Trick, the team powers and Absorb were used over and over, Creyon, 13/10/2026);
  Mind Trick's confusion and the shells of Protect, Absorb and the team powers staying
  on 2.5 s after the gesture, Rage's shell and spray 1 s after its long gesture, and
  then off until the next (the puffs
  only while the hand is out; Rage's and the team powers' were too short, Creyon,
  13/10/2026); Sense's shell, with no gesture, on 3 s and off
  1.5 s in turn; Lightning, Drain and Grip
  brought up (`BOTH_FORCELIGHTNING_START`, `BOTH_FORCE_DRAIN_START`), held 2.5 s with
  their beam or puffs, let go (their `_RELEASE`) and the wait (held for good, they
  never ended, Creyon, 13/10/2026);
  Speed's run fast with its two copies for 3 s, then at a run's own pace without them
  for 1.5 s, carrying on in its stride as it speeds up and slows down; Heal whole: down onto a knee, healing there 1.2 s, back up
  (`BOTH_FORCEHEAL_STOP`), then the wait. Every wait is 1.5 s (2.5 s was too long,
  Creyon, 13/10/2026). The gestures are the game's where
  multiplayer has one (Push and Mind Trick `BOTH_FORCEPUSH`, Pull, Grip's hold,
  Lightning, Drain `BOTH_FORCE_DRAIN_HOLD` as the JoF server plays it), Heal
  `BOTH_FORCEHEAL_START`, the team powers the quick heal, Rage, Protect and Absorb
  single player's poses the models carry, Speed a run (`BOTH_RUN1`, at 1.7 times its
  speed while Speed is on, as `CG_PlayerAnimation` runs a speeding player's legs), Saber Throw
  `BOTH_SABERPULL`, held out while the saber flies, as the game holds it while
  `saberInFlight` (it dropped before the saber was back, Creyon, 13/10/2026).
  It wears the power's shell as `CG_Player` tints it
  ([menu_stage/shell.rs](../crates/sjk-viewer/src/menu_stage/shell.rs)): Protect's
  green, Absorb's blue, Rage's red electric body, Team Heal's green and Team
  Energize's blue `ysalimarishell` (colour and alpha both the tint's), Sense's yellow
  `sightshell`. And it gives off the power's effect
  ([menu_stage/force_fx.rs](../crates/sjk-viewer/src/menu_stage/force_fx.rs)), from the
  game's own effect files in a small silent effect world of the preview's own, drawn
  with its camera into the layer its blades glow in, in their stock look (a map's
  recolours of effect pictures, such as one turning the push puffs red, are swapped
  out while the preview's are built): Lightning's beam from the hand
  (`force/lightningwide` at level 3), Drain's (`mp/drainwide` at 3), the push puffs
  for Push, Pull and the team powers, Grip's red and white puffs, JoF EJK's
  `force/rage2` and `force/heal2` sprays from the chest every 120 ms (now preloaded
  with the stock effects), Mind Trick's `force/confusion_old` over the head, and
  Speed's trail: two copies of the body 12 and 24 units behind the runner, seen
  through at alpha 100 and 50, behind him as the picture shows him (to the side when he
  faces the camera, where straight behind they hid behind the body) and a little
  farther from the camera. Each copy is drawn whole, as the body is, into a picture of
  its own, which the preview lays behind the body at its alpha; drawn see-through
  surface by surface, a copy's overlapping surfaces stacked and its face lost its jaw.
  Past the body nothing else counts: a shell's glow there leaves alpha near one over a
  colour that may be nearly black, and drew a dark rim. Jump plays a jump
  ([menu_stage/force_motion.rs](../crates/sjk-viewer/src/menu_stage/force_motion.rs))
  as the game plays one made by holding the jump key while standing still: the body,
  arms and all, takes off in `BOTH_JUMP1` and keeps its crouch the whole time in the
  air, then lands (`BOTH_LAND1`, as long as its four frames take, then straight into the
  stance), as Creyon's videos of the game show it (13/10/2026: the
  torso kept in the stance held the hands where a jumping player's are not), timed as long as a jump to the drafted level's
  height lasts under gravity 800 (66, 130, 226 or 418 units: about 2 seconds in the air
  at level 3), then standing a moment. The model is seen to rise along the jump, at most 8 units at its top (more took the head out of
  the picture), and come back down to where it stood,
  never lower, so the picture never zooms out. Its sabers go off at the take-off (0.15 s)
  and light again on landing (0.25 s): the game's jump holds the blade through the head
  (Creyon, 13/10/2026). Saber Throw throws the first saber out spinning flat at 800
  degrees a second, to the side and towards the camera, as far as 0.4 s at the game's
  400 units a second goes, but fast at first and slowing to its far end over 0.75 s, so
  it is out a while before it comes back into the hand (Creyon, 13/10/2026), then rests
  there 1.5 s. Saber Offense loops three attacks (down from the top, up from the bottom
  left as the game swings when moving left and back, `LS_A_BL2TR`, Creyon's pick, and
  from the left), each as the game chains one from
  the stance: its start (`BOTH_S3_S1_T_`), the attack, and its return, flowing into
  the next (`BOTH_A3_T__B_` then `BOTH_R3_B__S1`, ...), and Saber Defense raises the
  parries in turn (`BOTH_P1_S1_T_`, `_TR`, `_TL`, `_BL`, `_BR`), both at the game's
  own speed, as every preview plays (slowed, they felt slowed down, Creyon, 13/10/2026). Each swing hands over to the next as it reaches
  its last frame, taking over as the game does, in 100 ms at an even pace (Ghoul2's
  `BONE_ANIM_BLEND`; most of the game's swings start a few units off where the last
  ended), or straight on where the two meet; the longer eased takeover between them
  slowed each swing as it began and hurried it after (Creyon, 13/10/2026). One swing
  comes at a time, as the strong style fights: after each return the model settles over
  0.25 s into the strong style's stance (`BOTH_SABERSLOW_STANCE` with a single saber,
  Creyon, 13/10/2026: red, not the medium idle) and holds it
  0.3 s (1.5 s after the third), then the next start is eased in from it over 0.25 s
  (Creyon, 13/10/2026: back to the idle pose after each swing; waiting where the return
  ended looked frozen). A parry is
  two frames, so the takeover into it is the whole block: each is raised over 0.25 s,
  eased, kept 0.3 s as a block is held, and the stance taken again as slowly for 0.3 s
  (1.5 s after the fifth; the game's 100 ms made the blocks a snap, Creyon,
  13/10/2026); with a single saber the blocks come from and go back to the strong (red)
  style's stance `BOTH_SABERSLOW_STANCE` (Creyon, 13/10/2026); the staff (`A7`,
  `P7_S7`) and the dual sabers (`A6`, `P6_S6`) swing and block in their own, each back
  to its own stance.
- **Power box:** on the Force page, bottom right, the power under the pointer
  (or the keyboard) in big: its holocron, name, group, level, what it does in a
  line or two, what the server does with it when it limits it, the next
  level's price (or Mastered, Needs Saber offense 1...) and its levels'
  prices.
- **Caption:** beside the model, bottom right under a short gold rule, what the
  page shows of it: the model and its skin, the saber's style, hilt and blade,
  or (on the Force page with no power chosen) the side and rank.
- **Keys:** bottom right, the focused row's (Left Right change, Enter type or
  do it) and Tab with the next page's name.

The keys are the player screen's shared ones: Up and Down choose a row (on the Saber and
Force pages in the order they show them: the hilt search, both hilt lists, the
Blade row, the Sounds row and the blades' colours,
the Force groups' powers in their order, the other side's skipped), Left and
Right change it (a hilt list's choice moves one hilt, the Blade row's one
blade; the wheel over the Blade row and its ‹ › arrows browse it six blades at a time
without wearing one, 14/10/2026), Enter types or acts, Tab
and `[` `]` change page (on the Profile screen, its tabs: from Force on to SJK
Profile), Escape returns to the main page (leaving the Force page applies its
draft, see [client.md](client.md)). The pointer: a click on a control acts (‹ ›
by the half of the control it lands on, a chip picks the colour drawn under
it, a slider follows, a style button, a hilt, a blade, a level, the search
field types), a click elsewhere on a
row (a Force power's holocron or name too) only chooses it, a click on a tile
picks that model. The SJK view's own targets (levels, the Blade row's
blades, the Sounds row's Preview, style buttons, hilts and the lists' wheel areas, tokens from 1000) go to
`PlayerMenu::sjk_pointer` before the screen's shared pointer handling.

It is the player screen's shared state and controller with a view of its own:
the rows, tiles, tabs and back key answer to the screen's tokens, each row
registering its control before the whole row so the token's rectangle is the
control's.

Opened from a game (the in-game menu's Profile), where the menu map's stage
is not, the same screen shows with "Game menu" as its way back, over the match
dimmed by half, and the model stands right of the form in a live preview of its
own (`menu_stage::preview`, the classic pages' preview, asked for by
`PlayerMenu::sjk_model_preview` in the frame's `MODEL_AREA`): holding the saber
draft lit in its style's stance, held still at a three-quarter angle (28 degrees
to its right) rather than turning as retail's did, framed with 1.35 times
retail's room so a raised blade stays in the picture, on a soft shadow and a
thin gold line at its feet. `ClientMenu::stage_model` never stages a screen
opened from a game.

## SJK's pages

What's new, Update, Identity, Profile and Credits, which the main page's SJK page opens
(and their console commands; Profile as the [Profile screen](#profile-screen)'s
tab, the Collection as its own screen, Identity from Settings, Network, Credits and
What's new also from the in-game menu's row of icons), and the Staff, Collection and
SJK chat pages have the
SJK UI's look in this style (Profile, Staff, Collection and SJK chat in every style): drawn in its
families over the map darkened as Settings is (Update as a pop-up card, as First
setup), each with the way back (Esc, "Back") and its name at the top and its
keys bottom right. They stay the
console's pages (their state, keys, pointer and opening and closing are as in
the other looks); `ViewerConsole::set_sjk_pages` picks the look and the console
overlay routes their text to the UI's families (`console_sjk_pages.rs`).

- **What's new** ([changelog_sjk.rs](../crates/sjk-viewer/src/changelog_sjk.rs)):
  the releases down a lit rail on the left, newest first (name, then date or
  "Not released yet" in gold), the chosen one gold; beside them a reading
  column with its date and number of changes, its name, its introduction, then
  each change after a gold dot with its credit under it, scrolled by Page Up
  and Down or the wheel. Lines are wrapped by their measured width, and what a
  change puts in backticks is drawn as a chip, clickable as on the
  [News panel](#news-panel) (`inline_code.rs`).
- **Update** ([update_panel_sjk.rs](../crates/sjk-viewer/src/update_panel_sjk.rs)):
  a pop-up card (840 wide, as tall as what it says) over the map darkened all
  over, since 08/10/2026 (Sol asked for it with First setup's). Update (gold)
  and the version this is along its top, what the check found as a headline
  over its detail, the download's progress as a gold bar with its percentage,
  then the actions along its foot: Release notes and Check again on the left
  when offered, Close and the one Enter takes (gold: Install, Restart now,
  Check, Open release page) on the right.
- **Import** ([config_import_panel_sjk.rs](../crates/sjk-viewer/src/config_import_panel_sjk.rs)):
  the same pop-up card since 10/10/2026 (Sol saw the hero page left over).
  Import (gold) and the file's name along its top, a headline, then the parts the
  file holds (Name, Model, Field of view, Key bindings), one banded row each with
  what the file gives and its switch, and the note on keys SJK does not know; or
  what to do, why nothing was imported, or what changed. The actions along its
  foot: Other file... on the left, Cancel and Import (gold) on the right; without a
  file Close and Browse... (gold), which Enter takes
  ([client.md](client.md#importing-from-another-client)).
- **Identity** ([identity_panel_sjk.rs](../crates/sjk-viewer/src/identity_panel_sjk.rs)):
  the identity's state as a headline (the name the hub knows, or "Identity is
  off") over its lines, the switch sharing it with the hub, then while it is on
  the bio's field and Save (gold), Copy my key id, Show file (Hide file once shown)
  and Use the official hub, and the known players here under a sub-heading,
  verified ones marked gold. The key id is always bullets with "(hidden)", the key file
  until Show file, every time the page opens ([identity.md](identity.md#settings-and-commands)). A Medals
  column right of the page's lists the player's own medals once the hub answered
  (picture, name in gold, description, date given, the team's note), or says how
  medals come ([identity.md](identity.md#medals)).
- **Profile** ([profile_panel_view.rs](../crates/sjk-viewer/src/profile_panel_view.rs),
  08/10/2026, Sol's request: a real profile with the medals and an editable bio):
  drawn in this look in every menu style. On its own (the `profile` command with the
  classic menus) the top bar says Profile; in the SJK UI it is the Profile screen's
  SJK Profile tab, under the screen's title and row of tabs, moved down 44 pixels
  (`profile_panel::Mode`). Three columns: who the player is (their picture, a disc 104
  across with "Add a picture" or "Change picture" under it, beside the hub name in its
  colours at 38, verified, member since, other names; then, on the Profile screen,
  Change name (it opens the Character tab's Name field, the hub's name being the one
  worn in game; hub report #202, 30/10/2026), the Name emoji button and Staff tools
  for a staff key) over "Your record" (eight numbers in two columns) and "Unlocked lately"; "About
  you", the bio's box (620 by 300, Exo 2 at 18, wrapped by measured width, the end
  being written kept in view) with its counts, Revert and Save (gold) and the rules;
  Medals (pictures at 84, on its own only), Achievements (how many unlocked, a gold
  bar, See the board) and Collection (how many shaders owned at 30, or a line saying
  they are kept on the hub, and See the collection; Tab reaches it after See the
  board). See the board opens the [Collection](#collection) on Achievements, See the
  collection on Medals (as the Collection screen in the SJK UI, on its own in the
  classic menus). Until 10/10/2026 the page also held the achievements board and the
  medals as tabs. World shot: `world_shot::tests::duel6_sjk_profile`. The picture (a
  click, or Enter on it after Tab) opens the picture panel in the bio's place
  (08/10/2026): "Your picture", the picture 220 across (the one about to be sent,
  ringed bright gold, else the player's), beside it a headline (No picture yet,
  Reading the picture..., Your new picture, Sending..., Taking it down...), the file's
  name and the last message (gold, ember when something went wrong, the reason a
  picture was refused), under it Browse... (Choosing... while the system's file
  dialog is open) beside "Choose a picture file on this PC, or drop one on this
  window", what a picture may be and that everyone sees it, then Use this picture
  (gold), Remove picture (Press again while it waits) and Done. Escape returns to the
  bio first ([identity.md](identity.md#pictures)).
- **Staff** ([staff_panel_view.rs](../crates/sjk-viewer/src/staff_panel_view.rs),
  08/10/2026, for staff keys, [identity.md](identity.md#staff)): the top bar's
  search pill finds players; Me and Seen lately over the list of players found (14
  rows of 50, the chosen one banded, tags Staff and Verified in gold); the chosen
  player's picture (a disc 64 across), name at 38 and facts over the middle and
  right, and Take picture down at the right (dimmed without one); Medals: each medal's
  medallion (dim when not held) with its rank's device, name and count, rank and date
  given, a Rank button (held: the next rank each press; not held: the rank Give gives
  it at), Give (gold) and Take
  back, then the note's field (sent with the next medal or unlock); Unlockables under
  it: each unlockable of the catalogue, two to a row of 36 (all fourteen fit, since
  10/10/2026), its name at 16 (gold when held), "Since" its date or "Not held", and one
  button, Unlock (gold) when not held or Relock when held, as the medals' Give and
  Take back; Achievements: Clear all (Press again while it waits),
  each achievement with a count, its date or count and Clear, and a line on what
  clearing does. The last request's answer bottom left (gold, ember when refused).
  World shot: `world_shot::tests::duel6_sjk_staff` (the player's own, then another
  holding the Sun blade).
- **Collection**: its own screen since 10/10/2026, drawn in this look in every menu
  style ([Collection](#collection)); it replaces the Unlockables page.
- **SJK chat** ([sjk_chat_panel_view.rs](../crates/sjk-viewer/src/sjk_chat_panel_view.rs),
  08/10/2026, Sol's request, [hub-chat.md](hub-chat.md)): drawn in this look in every
  menu style. The online count in gold over a line on what the chat is; the messages
  down the left, newest at the bottom over the field and Send (gold), each one flowing
  line ([hub-chat.md](hub-chat.md#how-a-line-looks)): its name in its colours, the
  verified tick for a verified sender, and its text in the SJK chat's gold going on
  after them and wrapping when too long, with Staff and how long ago on the right of
  its first row; a chosen one is banded and shown on the right with Block on this PC, for
  a message with a GIF Add GIF to favourites beside Hide this GIF (one row) and Manage GIF
  source, and, for staff, two rows: Delete for everyone beside Lift chat ban, Mute at the
  hub beside Unmute at the hub, all above the keys line. "N older: Page Up" and "N
  newer: Page Down" mark the ends when it scrolls. Resting the pointer on a name shows
  the sender's sender card over the page, beside the name, with Block or Unblock
  ([hub-chat.md](hub-chat.md#blocking-a-player)). With no message chosen, the right
  column lists the players blocked on this PC, each with Unblock.
- **Sender card** ([sender_card.rs](../crates/sjk-viewer/src/sender_card.rs),
  08/10/2026, Sol's request): a name under the pointer in the game's chat (composer
  open), on the main page's dock and on the SJK chat page. A 340-wide navy card with a
  holo edge: their picture in a 48-wide disc (the [profile card](#profile-card)'s, their
  initial on their colour until it loads or when they have none), the name and the
  verified tick alone, "SJK player" (gold, with "SJK staff")
  or "Not known to the SJK hub", the hub name when it differs, the key, where they are
  on the server being played, their medals' medallions, Block or Unblock (gold edged) and
  what that does. It sits right of the name, or left of it at the screen's edge. In the
  game's chat a click on the name pins it (23/10/2026, after Bishop's and Sanjisan's
  ideas): it stays until the name is clicked again, a click lands off it, Escape or the
  composer closes; for a player on the server other than the player themselves it then
  heads its buttons with Whisper, the whole row in holo blue, above Block
  ([client.md](client.md), [chat/card.rs](../crates/sjk-viewer/src/chat/card.rs)). World
  shot: `chat::card::shots::chat_name_actions_shot` (1080p and 4K).
- **Test list** ([debug_panel_view.rs](../crates/sjk-viewer/src/debug_panel_view.rs),
  10/10/2026, Sol asked for every trace of the old hero look to go): the `debug_panel`
  command's personal test list ([sjk.md](sjk.md#debug-panel)), drawn in this look in
  every menu style and over the command browser when both are open. Top bar "Test
  list", the row of tabs (To test, Tested, All) under it, "N of M tested" in gold at the
  top right; the entries down the left in rows of 60 (x 96, 600 wide, from y 214 to
  950: tick box, title cut with an ellipsis by measured width, area in Rajdhani), the
  chosen one banded, its title and area gold; the reading column (x 790 to 1824): area
  in gold, the title at 36 on up to two lines, its tick with Tested or Not tested, then
  What changed, To test (numbered in gold) and Note (on a soft gold band with a gold
  bar), wrapped by measured width and cut with an ellipsis at y 950. The last tick's
  outcome (ember for a load or save error) bottom left; keys Space, Tab, Up Down, Esc
  bottom right. World shot: `world_shot::tests::duel6_debug_panel` (1080p and 4K with
  `ui_textScale 1.2`, `ui_menuStyle sjk` and `classic`).
- **Credits** ([credits_sjk.rs](../crates/sjk-viewer/src/credits_sjk.rs), since
  08/10/2026, Sol's request: restyle Credits but keep its sun): on the left SJK's
  emblem (220 across, centred at (300, 330)) is the page's sun, the one memorable
  thing: the classic page's two sunbursts turn opposite ways behind it, its
  golden god rays (`EmblemLayer::Godrays`, two sets against each other) reach out
  from it across the page and sparks rise through their light on the left; the
  reading column has a navy shade of its own (half), so the rays cross it softly.
  Under the emblem "The people who make SJK", then the sections down a lit
  rail, the one in view gold (a click scrolls to it), and Expand all (Collapse
  all once a fold is open) under them; the sections draw closer, their rows
  and type shrinking with the step, when there are too many for the rail to
  end above y 960 with Expand all. The step goes no lower than 30, so up to
  ten sections show; past that the rail shows a window of ten that moves only
  when the section in view leaves it, and only as far as brings it back (a
  click on a row never moves the rows), with three dots over the first row or
  under the last where sections are hidden.
  The column (x 640, 1120 wide, from y 140 to 960, scrolled by the arrows, Page Up and Down, Space, Home and End, the wheel and a
  thin holo scrollbar at x 1800) holds each section under a sub-heading:
  - Someone with a history: the name (Rajdhani 84 for Sol JK's section, 62 for
    the rest), the GitHub handle in gold at the right of its line, the role, the
    counts (changes when some are not pull requests, pull requests, commits) as
    gold numbers with their words, the links in gold after a holo dot, and the
    card's medals in a column on the right (the whole medal on its ribbon, 104
    across, its name in gold and what it is for). Then the folds as rows of 50,
    a plus (a gold minus when open) before Highlights or All work (All pull
    requests when all of it is), their count on the right, the band under the
    pointer. All work runs down a holo thread, each day a dot and its date over a
    rule; a row is the title, its pull request as a gold outlined tag (which
    opens it) and its commits; a row of several has a plus that unfolds them
    (hash in gold, subject, who made it when not the owner), a row of one a gold
    dot and its hash (it opens the commit).
  - Everyone else (Claude, the fonts, the references): cards two across the
    column, each under a thin holo rule: name, handle, role, medals (medallion
    and name), lines after gold dots, links.
  - Then the trademark notice.
  Keys bottom right: Up Down scroll, Tab section (Shift+Tab, `[` and `]` too), E
  expand all, Esc back. The layout is in frame pixels measured in the families
  (with the player's text style), so it holds at every window size and is laid
  out again only when a fold, the fonts or the style change. Pointer areas go to
  the rows the column shows whole, within the canvas's 96 (a test sweeps every
  scroll, folded and fully unfolded, at 1080p, 4K, 4:3 and 21:9). Pictures and
  arcs are kept out of the column's edges: the renderer clips a picture by
  squeezing it and leaves arcs whole, so the fold marks are rectangles and a
  medal's picture shows only while the column shows it whole.

## Console

With the SJK UI's menus the console (`con_style auto`) is the SJK UI's deck;
`con_style sjk` chooses it whatever the menus
([client.md](client.md#sjk-ui-console)). It is the classic console's grid and
keys in this style's colours, its gold `›` and caret, labels in its families
and key caps, on the console's own layer. The command browser (F3) is drawn in
this style with it, laid out as Settings (filters down a lit rail, entries, the
chosen entry's detail column), and its detail's text can be selected with the
mouse and copied
([client.md](client.md#useful-console-commands)). Two other designs, a
horizon (no panel edge, the navy fading out under a gold line) and a dock (an
outlined card with tabs), were offered beside it; Sol chose the deck
(08/10/2026), which reads best over a bright map and keeps the classic
console's full-width rows, and the other two were removed.
Outgoing whisper echoes use those same rows in both console looks: hovering one
shows the recipient name and client slot retained when the `tell` was sent. The
server echo is matched before the hover note is attached, so an incoming or
unmatched private line receives no recipient card
([client.md](client.md#console-styles)).

## Settings

[sjk_view.rs](../crates/sjk-viewer/src/settings/sjk_view.rs) draws it, and
[settings.rs](../crates/sjk-viewer/src/menu/sjk/settings.rs) lists its
categories and opens, switches and closes it. The main page's Settings opens it
on the category last shown (the mode's first the first time in a run); First
setup at start and the `firstsetup` command open First setup as a pop-up
([First setup](#first-setup)). It is laid out on the main page's 16:9 frame,
over the map darkened from the left (95 %) to the right (80 %), as the Settings
mock-up draws it.

Since 12/10/2026 (Sol's request) it has two modes ([mode.rs](../crates/sjk-viewer/src/settings/mode.rs),
`ui_settingsMode`, archived, `simple` or `advanced`): **Simple**, the default,
shows the settings that matter most in a few plain categories; **Advanced** shows
every setting. The main page, the game menu's Settings and a chip naming a
setting open the screen in the player's mode, on the category last shown or its
match of the same name in that mode (Advanced's Sound is Simple's Sound; a
category with no match opens the mode's first). A way to a category only the
other mode has (the classic Graphics page's renderer entry, First setup's All
settings, Interface's Quick wheel pages row, a classic Setup destination) shows
that mode for the visit and leaves the player's choice as it was.

- **Top:** the way back (an Esc key cap, "Main menu", also a click target) and
  the screen's name, top left; the Simple and Advanced segments (the mode on
  show gold), ending 28 pixels left of the search pill, which a click switches
  and stores; the search pill, top right ("Find a setting", its `/` key, then
  the number found). The segments answer to chrome slots 30 and 31
  (`menu::sjk::settings::MODE_SLOT`) on the rows and on Key bindings alike; F2
  (slot 29) switches to the other mode, on the category of the same name.
- **Rail:** the mode's categories down a lit holo line on the left, each with its
  settings icon. The one on show is gold with a gold bar on the line; none is lit
  while a search shows its results.
  - Simple: Display (resolution, render scale since 25/10/2026, display mode, vertical sync, FPS cap, field of
    view, brightness), Graphics (under Preset: the graphics preset dropdown;
    under Effects: weather, dynamic glow, impact marks, player shadows), Sound
    (effects and music volume, mute in background), Controls (sensitivity,
    invert mouse, always run), Key bindings, Gameplay (camera style, gameplay
    FPS, first-person weapon, saber trail, everyone as my model, Force Seeing
    aura, away emote and JA+ away emote, since 20/10/2026), HUD (under HUD: HUD look and scale; Crosshair: crosshair, its size,
    damage hitmarker; Over the game: nameplates, FPS readout, speedometer, kill
    feed, chat and its text size) and General (Menus: menu style and text size;
    From another client: import; JoF content: its card, since 16/10/2026, and its
    auto-update switch, and the same for the Sith cloud, since 31/10/2026; SJK
    online: check for updates, SJK chat, crash reports, settings usage, SJK
    identity key). They are `settings::Group::Simple`.
  - Advanced: First setup, Display, Graphics, Sound, Controls (Mouse until
    12/10/2026: it holds keyboard rows too), Key bindings, Gameplay, Interface,
    HUD, Chat, Quick wheel, Mods, Network (Chat and Mods since 12/10/2026, when
    Scoreboard became a sub-heading of HUD: 13 categories fit the rail above the
    keys' line, 14 would not).
  Network's SJK identity key row (09/10/2026, Sol's request: the key out of the
  Profile screen and hidden, for streaming) reads "Hidden: open to see" and Enter
  opens the Identity page over Settings ([SJK's pages](#sjks-pages)), which shows
  Settings again when it closes.
  Quick wheel wears the wheel's own icon, a ring of discs from its second board
  (`quick_wheel::catalog::WHEEL_ICON`; `settings_icons::texture` falls back to
  the wheel's icons), since 08/10/2026.
  - Advanced regroups the classic+ Setup page's groups and tabs by subject
    (`settings::Group::Advanced`, `Group::Display`, `Group::Graphics`,
    `Group::Network`), each category's classic+ panel named for the hand-over
    to the classic style. Graphics leads with a Quality heading (the graphics
    preset, impact marks and player shadows, out of Video) and then
    gathers the renderer's four tabs (image, lighting, shadows, weather) under
    their names; Display is Video's other rows under Screen, Frame rate (with
    the menus' own cap, Menu FPS cap, `com_maxfpsMenu`, since 25/10/2026) and
    View; Sound holds every sound, the game's under Game, then the unlock
    sound, the world ping's and the quick wheel's sounds under
    Notifications, and Chat sounds last (since 23/10/2026: the master switch, then
    for public chat, team chat, private messages and the SJK chat a switch, a sound
    list and a Play button, then the private message flash;
    [client.md](client.md#chat-sounds)); Gameplay is Game options without the packs and mods, with
    JoF EJK's camera framing and damping rows under Camera and prediction and,
    under Effects, Stock blades (`cg_stockBlades`, segments Off, Others, All,
    since 25/10/2026; [client.md](client.md#nameplates));
    Interface the menus and the console (the quick wheel's rows are its
    category's and Sound's); HUD its classic rows without the chat box, the FPS
    readout opening its Readouts (moved there from Video, as the first-person
    weapon moved to Gameplay's camera rows), and the scoreboard's rows under a
    Scoreboard heading; Chat the chat box (Chat box: visibility, text size,
    emojis, remembered channel, letter spacing and sender card size) and the SJK chat (SJK
    chat, its game-feed and console visibility, its GIFs, its video clips opt-in and
    the GIPHY key); Mods the asset browser (Packs), JA+ and
    JoF tools (Server mods), the JA+ and jaPRO plugin options under a heading each
    ([mods.md](mods.md#plugin-options)), JoF content's card and host (JoF content),
    invisible brushes (Map tools) and the showcase tools (Experimental); Interface's
    Console rows include retaining colour codes when console scrollback is copied;
    Network SJK online and Connection. A test checks that
    every setting is on exactly one of Advanced's categories. The classic+
    panels keep their groups as they were.
  - Key bindings shows every action under its group's sub-heading (Movement,
    Interaction, Weapons, Force powers, Other), its picture before its name
    where it has one, its two keys as caps on the right (a dash for an empty
    one); the cap awaiting a key turns gold ("Press a key") and the detail
    column says what to press. The detail column names its keys, default key,
    console command and what else its keys do. The search finds actions by
    name, command, key or group. Enter, or a click on the row or its first cap,
    awaits its first key; a click on its second cap, its second; Backspace
    clears its keys; Escape cancels the wait. It is the classic+ list of the
    key-binding editor (`keybind_editor/sjk_view.rs`).
- **Rows:** the open category's rows in a column (x 470 to 1270), 56 tall, under
  sub-headings (holo, with a rule after them): the name on the left, the control
  ending at 1226 and the reset arrow after it. Fourteen lines show; a longer
  category scrolls (wheel, scrollbar), keeping the focused row in view.
  - Controls, from the kit: a switch (gold with its knob right when on, On or
    Off after it); a slider (a gold-filled track and its number, which a click
    or a typed digit opens for typing); segments for a choice of up to three; a
    field with a caret for a longer list, the display mode, the resolution and
    the HUD (the last two open their pickers); a field for text; a Play button (a chat
    sound's preview: Enter or a click plays it).
  - A choice whose values are numbers or codes shows names on its segments, list,
    field and detail (`settings::help::CHOICE_NAMES`): Shader remaps Never, Not
    players, Always; Dynamic glow Off, On, Sabers; Light shafts Off to High;
    Weather quality Low to Ultra; Force weather The map's, Drizzle, Rain, Storm,
    Snow; and so on. These rows were sliders labelled "0 off / 1 map / 2 all"
    and the like until 12/10/2026; their cvars keep their numbers.
  - The resolution list is a pop-up card over the darkened map (x 600, 720
    wide, as tall as its rows, `settings/resolution_list.rs`), in every menu
    style since 10/10/2026 (it was the old hero form): Resolution and what the
    size means in the display mode, then up to 14 rows of a size and its aspect
    ratio, the desktop's size tagged, the one in use in gold with its dot and
    tag, the highlighted one on the band, a thin scroll mark when there are
    more; the keys' line under the card (Up/Down, PgUp/PgDn, Enter, Esc, whose
    cap is the pointer's way back). Home/End and the wheel move too; Enter or a
    click picks, Esc returns to the rows.
  - The HUD picker (HUD look's Enter) is a pop-up card the same way, in every
    menu style since 19/10/2026 (`settings/hud_picker.rs`; x 240, 1440 wide):
    HUD look and what it is, the HUDs' names on the left (up to 12 rows, the
    highlighted one on the band, the one in use in gold with its dot), the
    highlighted HUD's picture on the right (16:9, rounded, a thin holo edge)
    with its name and origin under it, and the keys' line under the card.
    Hovering a name shows its picture; SJK's radial and classic layouts are
    drawn on it by the live HUD ([client.md](client.md#hud-style)).
  - Slider peek (since 29/10/2026, after Blep's idea; Settings > Interface,
    `ui_sliderPeek`, on): in the game, holding a slider (its track or knob, not
    its number) for 0.3 s fades everything else away in 0.15 s, eased: the rail,
    the top bar, the other rows and sub-headings, the detail column, the keys and
    the darkening behind, leaving only that row (its band, name, slider and number)
    where it was over the live game, which keeps changing as the slider is dragged.
    Letting go (or the pointer leaving the window) brings the screen back in 0.15 s;
    a press again while it returns carries on from there, and a key shows it at
    once, as does the console opening, a card coming up over Settings or the window
    losing the focus (each takes the release away from Settings). A quick click never fades it, nor does it happen over the main menu's map
    or in First setup's pop-up ([slider_peek.rs](../crates/sjk-viewer/src/settings/slider_peek.rs)).
  - The focused row has a soft band and a gold bar at its left; a row changed
    from its default a gold dot after its name, and when focused the reset arrow.
    The dot is placed from the name's width as the body font draws it (the
    player's `ui_textScale` and `ui_letterSpacing` included), 8 pixels after
    its last letter; a name cut short at its column has it at the column's end.
    Key bindings' rows place theirs the same way.
  - A list opens under its field (over it near the bottom), its choice in use
    marked with a gold dot; the controls under it are left out while it is open.
- **Detail:** on the right (x 1360, 464 wide), the focused setting: its
  category's icon (a search result's own group's) and its name, what it does,
  then its default, range or choices, when a change applies (gold), where a
  search found it and its console name (holo). While the SJK renderer is on, every
  renderer setting (the graphics preset and Graphics' image, lighting, shadows
  and weather rows) has the alpha warning in gold under what it does: the
  renderer is in alpha, some maps can show glitches or run slower, report them
  from the SJK menu's Report a bug, Vanilla style is the safe choice. First setup adds
  how to import another client's .cfg.
- **Keys:** bottom right, the keys of what has the keyboard: the focused row's
  (Enter switch, Left Right change, Enter choices...), Backspace for the default
  when it changed, Tab for the next category, F2 for the other mode ("F2
  advanced" or "F2 simple"); a list's, a search's or a typed value's own while
  they are open.

Keys: Up and Down choose a row (Up from the first, or `/`, F3 or Ctrl+F, goes to
the search, its text selected so typing or a paste replaces it; the same keys
find an action on Key bindings);
Left and Right change it; Enter switches, opens a list or starts typing;
Backspace returns it to its default; Tab and `]` (`[` back) open the next
category of the mode's rail, past Key bindings; F2 shows the other mode; Escape
closes a list, clears a search, then returns
to the main page. Hovering a row chooses it; a click on a control acts, a click
on a row's name only chooses it (a switch flips from anywhere on its row); the
right button returns a row to its default.

The rows are a classic+ panel's (`settings::ClassicRows`), so the search over
every setting, the lists, the defaults and typed numbers are the classic+
panels' own; only the drawing and the frame are the SJK UI's. The search finds
every setting in both modes ([search.rs](../crates/sjk-viewer/src/settings/search.rs)),
its results under the names of Advanced's categories: Display, Graphics,
Graphics: image, Graphics: lighting, Graphics: shadows, Graphics: weather, Sound,
Controls, Gameplay, Interface, HUD (the scoreboard's rows too), Chat, Quick
wheel, Mods, Network, First setup. What the
player changes and searches here is recorded for [settings
usage](client.md#settings-usage). Changing Menu
style on the screen (it is on Interface) hands over to the classic+ panel of the
same group (Graphics: the renderer's image tab).

Gameplay's first row, under Asset packs, is Asset browser ("Browse packs";
`assetbrowser` in the console, `asset_pack_browser.rs`): Enter opens the
installed-pack browser, an SJK UI card in every menu style. It lists ten pack
rows at a time, with keyboard, wheel and click controls; each row says what the
pack will be at `fs_restart` or the next start (Enabled, Disabled, Required). The console owns
input while it is open, and closing it restores the previous console state. See
[installed PK3 choices](client.md#launch).

Mods' JoF content row ("Choose and download"; `jof.downloads` in the console,
`jof_content_card.rs`) opens the JoF content card the same way, an SJK UI card in
every menu style: the opt-in switch, the automatic updates' switch (since
31/10/2026), JoF's groups with a tick and an Auto tick each and what is here of them,
the download's progress, Check again and Download
([jof-content.md](jof-content.md#the-card)).

### First setup

At start (until it is ticked off) and from the `firstsetup` command, First setup
is a pop-up card over the map, not the Settings screen (Sol's request,
08/10/2026, with the tick that stops it opening always in view at its foot). [sjk_popup.rs](../crates/sjk-viewer/src/settings/sjk_popup.rs)
draws it; the rail's First setup category still shows the same rows in the
Settings screen.

- **Card:** 960 wide and 948 tall, centred, over the map darkened all over
  (78 %), with the browser prompts' glass, shadow and holo edge (`kit::card`).
- **Top:** "First setup", what it is for, and how to bring another client's
  .cfg over.
- **Rows:** First setup's rows as Settings draws them, its column moved into
  the card, under its sub-headings (Graphics first: the graphics preset, Vanilla
  style, SJK (recommended) or Experimental, whose list opens on Enter; see [graphics
  quality](client.md#graphics-quality); then Styles: menu and camera style).
  Eleven lines show; the rest scroll inside the card. The focused row's help
  sits in two lines under them (left out while a list is open).
- **Foot,** always in view under a rule: the "Don't show at start" tick box
  (`kit::tick`, gold with a dark tick when ticked) and its name on the left, All
  settings and Done (gold) on the right.
- **Keys:** under the card, bottom right: the focused row's, Tab all settings,
  Esc done.

The tick is First setup's own last row (`ui_hideFirstSetup`), pinned out of the
scrolling lines (`ClassicRows::pinned`): the keyboard reaches it after the last
row (Up from the first wraps to it), Enter or a click flips it, Backspace
returns it to its default. Done or Escape goes back to the main page; All
settings or Tab opens the Settings screen on First setup (Advanced, for the
visit, in Simple). The pop-up has no
search. Picking the classic style on its Menu style row goes on as that style's
First setup, and picking the SJK UI on the classic one comes back to the pop-up. In a game the SJK UI's First setup is still the classic panel.

### Quick wheel

The quick wheel's pages, edited (since 08/10/2026, Sol's request:
[client.md](client.md#quick-wheels) has what the wheel does). Its category
([wheel_editor.rs](../crates/sjk-viewer/src/settings/wheel_editor.rs), drawn by
[wheel_editor_view.rs](../crates/sjk-viewer/src/settings/wheel_editor_view.rs))
keeps Settings' top bar (without the search) and rail and fills the rows' and
detail's columns:

- **Pages** (x 470, 360 wide, in the wheel's order: Force, General, Toys, Weather
  by default), under a sub-heading: a row each (56 tall), its
  name and how many choices it has, the page whose choices show in gold (a gold
  bar at its left while the focus is elsewhere); the focused row has the band
  and, at its end, Rename, up, down and remove as small round controls (remove
  turns ember while it waits for a second Delete); the Force page's count reads
  "Your powers". Then "+ Add a page" in gold, "+ Add the Force page" in gold
  while that page has been removed, and Restore the default pages (muted; quiet
  "These are the default pages" when they are; ember "Enter again to restore
  them" while it waits for a second Enter). Under a "Sound"
  sub-heading a line below, "Wheel sounds" with the kit's switch
  (`cg_wheelSounds`; the pages column's last row: Enter, Space or a click
  switches it, and the right column says what it does).
- **Choices** (x 870, 400 wide), under "On <page>": each choice's picture (a holo
  ring without one; a custom command's is the `{•}` disc), its name and, muted
  on the right, its group or "Custom";
  the focused one's up, down and remove; then "+ Add a choice". The Force page
  has none: "Follows your Force powers" in gold and a muted paragraph on what
  fills it and how its powers act (since 10/10/2026). The Toys page (Illuminate)
  is an ordinary page: its choices are listed, changed and removed like General's,
  and the catalogue's Toys group, between Emotes and Weather, brings a deleted toy
  back (the ring shows it with the holocron's picture).
- **Right column** (the detail's, x 1360): the page's ring as the wheel draws
  it at 0.78 of its size, the focused choice highlighted, then facts (Key in
  gold, Bind and Runs in holo) and a line of help. For the Force page the ring
  is an example, a light-side build (nine powers) in the Force bar's pictures,
  Protect marked selected. Picking a choice: its title
  ("Change Rain", "Add to Weather") with Cancel, then the catalogue, 17 lines of
  40 under the groups' sub-headings, the highlighted line on the band, "On the
  page" in gold after actions the page has. A custom choice: its title, what a
  command can be, the Name and Command fields, then Picture (03/11/2026, Nib's idea):
  the choice's picture at 52 (its own, or `{•}`), Choose picture (Change picture once
  it has one; F3 too) and Remove, and a quiet line under them ("PNG or JPEG, 4 MB at
  most; a copy is kept in your SJK folder", or in ember why a picture was refused),
  then Cancel and Save (gold), which keeps the picture with the command
  ([client.md](client.md#quick-wheels)).
- **Keys:** bottom right, what has the keyboard's, and Tab for the next group.

Outside the SJK UI the editor draws the same columns on its own over the
classic+ or tabbed settings (the navy ground, "Esc Settings" and "Quick wheel"
as its title), moved 190 left where the rail would be. The editor has its own
canvas (1024 draw commands); every state of a full wheel fits it (a test).

## Servers

[browser.rs](../crates/sjk-viewer/src/menu/sjk/browser.rs): the server browser,
opened by Play's Join a server (and by the in-game menu's), over the map darkened
as Settings is, on the main page's 16:9 frame. The camera keeps touring behind it.

- **Top:** the way back (Esc, "Main menu", or "Game menu" from a game), the
  screen's name ("Servers") and the search pill ("Find a server or map", `/`,
  then the number found). The search matches names without their colour codes,
  and map names.
- **Left:** where the servers come from down a lit rail, All servers and
  Favourites with how many of each answered; under "Show", switches for empty,
  full and locked servers and the game type as a cycler (All, then each type);
  at its foot Refresh the list, Join by address and Add a favourite (Sol's
  request, 11/10/2026): its prompt takes an address and keeps it as a favourite,
  then shows the favourites and asks the servers again. Favourites are asked for
  their info directly, so one no master server lists (a private server) still
  answers in the list.
- **List:** a header line of sortable columns (Server, Mode, Map, Players,
  Ping; the sorted one gold with a sort mark after it, three stacked bars
  longest at the top when descending, at the bottom when ascending; a click
  sorts or flips it), then fourteen rows of 52 that scroll (wheel, scrollbar).
  A row: a gold dot for a favourite, the name in its colours (faded when
  nobody but bots plays), a gold padlock when it needs a password, the game type with
  the mod as a small tag (JA+, JAPRO, Mod; none for base), the map without
  `mp/`, players over capacity (the places a player without the private password
  can take: a server whose `getinfo` counts its `sv_privateClients` slots, which
  stock's does not, shows them off once the row is chosen and its `getstatus` names
  both; Kushina's and Blep's 30/32 servers that said full), four signal bars lit by the ping (four to 60 ms,
  three to 110, two to 180, one to 400) and the ping. The chosen row has the
  band and gold bar; favourites sort first. An empty list says why (the search,
  no favourite answered, the master server being asked, nothing answered, or
  the Show choices hiding everything).
- **Right:** the chosen server: its map's levelshot (cropped, never stretched:
  a square retail levelshot is a 4:3 picture), the map's name over its foot;
  the server's name in its colours (a second line goes on in the colour the
  first ended in); Mode, Players (with its bots), Ping and Mod (the server's
  own name for its mod once its status answers); its address, "Needs a
  password" in gold when it does, its limits in words ("30 frags, 20
  minutes"); Join (gold) and Add to favourites or Remove favourite; then
  "Playing now", its players with their colours and scores, bots muted, six
  at a time: past six the wheel over the list scrolls it two rows a notch (the
  server table stays put) and a thin bar on its right shows where the rows in view
  are; it starts at the top for each server (Creyon's request, 12/10/2026; it ended
  in "and n more").
- **Keys:** bottom right, what has the keyboard's; bottom left, how the list
  stands ("12 servers answered", "Asking the master server...").
- **Remembered passwords** (Sol's request, 11/10/2026): the password card has a
  "Remember on this PC" switch (on by default); joining with it on keeps the
  password for that server ([server_passwords.rs](../crates/sjk-viewer/src/server_passwords.rs),
  `server-passwords.json` beside `config.cfg`, plain text as `config.cfg` keeps the
  last `password`), and the card then opens with it typed, so Enter joins; joining
  with it off forgets it. The classic prompt remembers the same way, without a switch.
- **Prompts:** the password and the address open a card over the darkened map
  (the list is left out under it, since text draws over every shape): its
  title, a line saying what to type, the field (the password as dots), the
  address's error in gold, Cancel and Join or Connect (gold, dimmed while the
  field is empty).

Keys: the list has the shared browser keys (Up and Down, Page Up and Down, Home
and End, Enter joins, F favourite, A add a favourite by its address, Tab between
all servers and favourites, R refresh, C an address, `/` search, 1 to 5 sort); Left moves to the left column,
where Up and Down choose, Enter shows a source or flips a switch, Left and Right
step the game type, and Right or Escape return to the list. While the search is
typed, Enter, Down or Tab return to the list (keeping it); Escape clears it.
Escape on the list clears a search first, then returns to the main page. The
pointer: a click on a row chooses it and a second within 0.4 s joins it; a
header sorts; a switch's row flips it; the game type steps back on its left
half and on on its right half.

Joining shows the SJK UI's loading screen (Loading).

## Create game

[create_game_view.rs](../crates/sjk-viewer/src/menu/create_game_view.rs) and
[map_picker_view.rs](../crates/sjk-viewer/src/menu/map_picker_view.rs): the screen
that hosts a match on this machine with bots (Play's Create a game, the classic Solo
Game and Create Server), since 10/10/2026 in the SJK UI's look in every menu style,
as it has no classic+ version (the hero form it had is gone). It lies over the map
darkened as Settings is, on the main page's 16:9 frame.

- **Top:** the way back (Esc, "Main menu") and "Create game".
- **Left:** under "Match", banded rows of 56 with a cycler each: Game type, Map, Bots,
  Bot skill (Initiate to Jedi Master), the mode's limit (Frag limit, Capture limit, or
  Objectives for Siege, which cannot be changed) and Time limit; under "Server", Server
  mod as a cycler (Base game, the default, or JA+: the mod `sjk-server` runs,
  [JA+ mode](server.md#ja-mode)), Server name as a field (typing shows a cursor) and
  Allow LAN players as a switch with what it means ("This PC only"); then Start the
  game, the gold button ("Starting..." and dimmed while the server starts). Every
  choice is kept in an archived `ui_host*` cvar (`ui_hostServerMod` for the mod, by
  its id).
- **Right:** the chosen map's levelshot (2:1, cropped, never stretched) with its file
  name over its foot (a click opens the map list); its long name; four figures in two
  columns (Game type, with the mod after it when it is not the base game, "Duel, JA+";
  Bots with their skill, Limits in words, Who can join); a line
  saying who the match is for.
- **Bottom:** how the start stands at the left (starting, or why it failed); the
  chosen row's keys at the right.
- **Map list** (Enter on Map, or the picture): the way back is "Create game", the title
  "Choose a map", and the search pill holds what is typed (every printable key filters,
  Backspace erases), with how many were found. The header line says how many maps the
  game type has ("5 of 61 maps for Free for all") over a File column; fourteen rows of 52
  with the long name and the `mp/...` name (left out when they are the same), the chosen
  map marked with a gold dot, the highlighted one on the band, a scrollbar past
  fourteen; an empty list says nothing matches. On the right the highlighted map's
  levelshot, its name and the game types it is played in.

Keys and pointer are the screen's own as before: Up, Down (and Tab) choose, Left and
Right change, Enter starts, edits the name, flips the switch or opens the list, Escape
goes back; in the list Up, Down, Page Up and Down, Home and End move, Enter picks,
Escape returns. Hovering chooses a row; a click on a cycler's left half steps back and
on its right half on; the wheel moves the choice (or scrolls the list).

## Loading

[loading.rs](../crates/sjk-viewer/src/menu/sjk/loading.rs): what a join shows
from the browser, the main page's recent servers, an address or Create game
until the game is on screen, and a server's change of map. Its keys are the
classic screen's: Escape cancels (back to the browser, or to Create game for a
hosted game; on a server's change of map it leaves the server); after a failure
Escape, Enter or Space return to the browser. The pointer's target is the Esc
key cap at the bottom right only, not the whole screen as on the classic one,
so a stray click does not end a join.

- **Before the map is known** (an address typed or a recent server the list
  does not have; the server's info answer names the map within a second): the
  menu's map stays on show and its camera keeps touring, darkened towards the
  bottom and its left. A block at the bottom left: "Joining" small over the
  server's name large (Rajdhani 56, with its colours: the server list's or the
  recent servers' name, else the address), its address in holo, then a thin
  line 440 long that fills gold by step, and on the bottom row a slowly turning
  gold arc with the step in words. Esc "cancel" sits at the bottom right.
- **Once the map is known** (the browser row's map, the server's answer or the
  gamestate): the map's levelshot fades in over the whole window, never
  stretched (`levelshot::screen_fit`): one as wide as the window or narrower
  covers it, cut at its top and bottom (a square retail levelshot is a 4:3
  picture); a wider one (the HD packs' 2:1 shots, their title against the left
  edge) keeps its whole width with navy bands above and below. It is darkened a
  little all over, more towards the bottom and its left and a touch at the
  top. At the bottom left, from the top: "Joining", the map's own name from its
  worldspawn large (Rajdhani 124, 96 past 18 letters; the file name when it has
  none), its file name without `mp/` small, the server's name (Rajdhani 36, with its colours), then
  the classic screen's lines in words: the game type and limits ("Free for
  all, 30 frags, 20 minutes"; before the gamestate the server list's game type
  and players), the mod and the server's rules ("JA+ Mod v2.6 · Force mastery:
  Jedi Master · Saber only", "No Force powers", "Force-based teams", "Cheats
  on"), and the message of the day (up to two lines, without colour codes). The
  bottom row has the arc and the step, and a thin gold bar runs across the
  bottom of the frame.
- **Steps** (`loading::Step`): Asking the server, Connecting, Receiving the
  game state, Joining the game (the gamestate is in), then once the session is
  in hand Loading <map>, Preparing the graphics and Entering the game; a
  download in progress shows its own line instead ("Downloading x.pk3: 120 /
  900 KiB"). The line and bar fill one seventh a step and never run back within
  a join: on the menu's map the browser's map is first built as a preview,
  which the session's own build replaces, so the world's steps count only for
  the session's world.
- **A failure** keeps the layout of the state it happened in, with "Could not
  join" (or "Connection lost" once the session was in hand) over the name, the
  reason in words on the bottom row instead of the step, no line or bar, and
  Esc "back to servers".
- **A server's change of map**: "Next map" over the server's name. Until the
  new gamestate names the map, the block shows (the classic screen keeps the
  last map's levelshot meanwhile; this one does not), and Esc says "leave the
  server".
- **Starting a hosted game**: "Starting your game" over "Your game" (or the
  game's own name once its gamestate is in), without an address.

The world under the screen: on the menu's map the world keeps rendering under
it (the join's destination loads beside it, as for the classic style) until
the destination's levelshot has faded in, then it is left out
(`ClientMenu::sjk_loading_hides_world`, read by
`menu_backdrop::classic_hides_world`). On a server's world (a change of map, or
the joined world installed before the game takes over) the world is always
left out and the screen draws on the UI's navy ground; the levelshot fades in
over it.

## Scoreboard

[scoreboard/sjk.rs](../crates/sjk-viewer/src/scoreboard/sjk.rs): the scoreboard
held with Tab in a match (and shown at intermission), `cg_scoreboardStyle sjk`.
Its default, `auto`, shows it while `ui_menuStyle` is `sjk` and the classic
scoreboard otherwise; a profile's saved old default (`classic`) moves to it
once, a style chosen after that stays ([client.md](client.md#scoreboard-styles)). No panel: the columns float over
the game, which is darkened by the UI's navy (a third everywhere, deeper behind
the header and, from the chat column's edge, behind the columns). It fades in
and out as the classic board does, settling into place as it opens.

- **Header line:** across the top, as the other screens' top bar: the map
  without `mp/` (Rajdhani 48), then the mode and its limits in words ("Free for
  all · 30 frags · 20 minutes", as the browser words them); on the right the
  frame rate ("240 fps", the number in the clock's type and "fps" muted after it,
  the `cg_drawFPS` line's measured rate; Sol's request of 11/10/2026, on by default,
  `cg_scoreboardFps`), the time left in a timed match ("12:12 left", else "3:12
  played") and your place,
  the screen's one memorable thing, in gold ("3rd", "of 14" after it, "of 14,
  tied" on a tie, the server's place; with `cg_scoreboardDeathsRank` the place the #
  column gives your row, never tied; "Spectating" when you watch). Team games say "Your team leads
  by 2", "trails by", "Tied at 5" (or which team leads, for a spectator). Siege,
  which scores no team, says nothing there, and its team heads carry no score
  (the zeros meant nothing); once a round is over "Red team won" or "Blue team
  won" stands large (Rajdhani 56) in the winner's colour, centred over both
  teams, "Winner" in gold at the end of that team's head, and the whole window
  washed faintly in that colour, deeper towards the top (JoF's request of
  17/10/2026): the winner of the `EV_SIEGE_ROUNDOVER` seen when
  `CS_SIEGE_STATE` turned "3|time" (`sjk_client::siege_round_winner`), else,
  after the second round of a match whose teams switch, `CS_SIEGE_WINTEAM`; one
  who joined after another round's event sees no winner. A team game at its
  intermission tells its result as a siege round does (Slash's request of
  19/10/2026), from the moment the server sets `CS_INTERMISSION` (or this
  client's own `PM_INTERMISSION`; as cgame's `cg.intermissionStarted`, only a
  change of the string counts, and a new level start ends it, since a `map_restart`
  keeps the "1"): "Blue team won" over the blue wash and "Winner" at the end of that
  team's head, or "Teams tied" in white; the scores stay in the team heads, and
  the line on the right says nothing then. Under
  the map, "Killed by" and the name while you are dead.
- **Where:** the columns start at x 680 of the frame, right of the chat column
  the scoreboard keeps for messages and the composer (`scoreboard::layout`, 640
  pixels of a 1080-line window), and end at 1824. A compact board is narrower
  and centred (below).
- **Rows:** under muted column labels and a thin holo rule, one row each,
  52 tall at most, with a hairline under it: the place (Rajdhani; labelled "#";
  ties share it, 1, 2, 2, 4, as the server ranks them; with `cg_scoreboardDeathsRank` one
  number a row in the listed order, ties too, as JoF EJK lists its rows, away and joining
  rows taking none), with `cg_scoreboardIDs` (archived, off by default as stock JKA's
  scoreboard shows none; Settings > HUD > Scoreboard, "SJK scoreboard IDs") the
  client number as a small tag on a faint plate in a slot of its own just
  before the name, labelled "ID" (40 frame pixels, 26 in compact rows; gold on your
  row), the name with its colours (Exo 2), SJK's emblem right after it for a
  player the hub knows (gold when vouched for, as elsewhere), at intermission a
  gold "Ready", then the score (as `score/deaths` when `cg_scoreDeaths` provides
  deaths), the minutes played, and the ping as the browser's signal bars and the number.
  Your row has the UI's band with its gold bar, and its place, score, bars and
  ping in gold. Bots and players still joining are muted, with "Bot", "Joining"
  or "Connecting" for their ping.
- **Compact** (`cg_compactScoreboard`, Settings > HUD > Scoreboard > "Compact SJK
  scoreboard", on by default): rows are 32 tall at most and as thin as 20, and
  a list splits in two only when even 20 would not fit, so a full server (32 in
  free for all, or 32 on one team) stands in one column. Names and numbers scale
  with the thinner rows (names 16 to 20, numbers 17 to 22). The board is sized
  from what it shows, not the screen (`sjk::Board`): the name column is as wide
  as the longest name with its emblem, bars and "Ready", measured in the
  families that draw it, at least 140 and at most 320 frame pixels (a longer
  name ends in an ellipsis), the same for both teams; the first number's right
  edge stands 48 after it, each number column is its widest label plus 14 (48
  at least), and the ping column keeps its 100. A player's medal bars are not
  held to a third of the name column as on the full board: the column widens
  for all of them (three at most). The whole board is centred on the screen,
  or, where that would put it over the chat column, stands 40 right of the
  column (as near the middle as the chat allows); it never ends past 1824 or
  grows wider than the full board, the name column giving way (to 100 at the
  least). Deaths share the score column rather than adding another column. A duel's
  cards keep their size about that middle, the players
  waiting centred under them. The dim behind the columns follows the board,
  fading out on both sides, and "Watching" starts at its left edge. In the UI's
  families at 1080 lines (`scoreboard::shot` tests, on the world shot's made-up
  names), a free for all of 8 is 440 frame pixels wide and of 32 is 411,
  centred; a Team FFA of 6 a side is 835 and
  capture the flag of 6 a side 1142, both from 680 beside the chat column
  (four number columns and a player's three medal bars leave capture little to
  gain), against 1144 for the full board in every mode. Off, the sizes below
  apply and the board fills 680 to 1824 as before.
- **Free for all** (and Holocron, Jedi Master): one list by place; once rows
  would be thinner than 34 pixels (past 22 players) two lists side by side, 16
  each at 32. Compact keeps it one list (32 players at 23.6).
- **Team games:** the two teams side by side, red left and blue right, each
  under its head: the team's score in Rajdhani 76 (muted while it trails), the
  team's name in its muted colour with how many play, and a thin rule in that
  colour. Capture modes show score, captures, assists and defends (no minutes,
  for room) and a carried flag's icon before the name. A team too long for
  rows of 26 pixels (20 compact) ends with "and n more", keeping your row.
- **Duel and power duel:** the duelists (`CS_CLIENT_DUELISTS`, else the
  players not spectating) as two facing cards about the board's middle with
  "vs" between them: the name (Rajdhani 44), the score large (112, gold for
  you), the record and health in words ("3 wins · 1 loss · 87 health · 25
  shield"; your health and shield from your snapshot, an opponent's health when
  the server shares it, `g_showDuelHealths`), over a rule, gold under your
  card. A power duel's pair stack two smaller cards facing the lone duelist.
  Under them "Waiting to duel": everyone else in turn, with their record
  (compact: one list up to 21, then two).
- **Spectators:** one line at the bottom, "Watching" then their names without
  colours.
- **Away players** ([client.md](client.md#away-players), `cg_scoreboardAfk`):
  below the others of their list, without a place, the name faint with its
  colours at 40%, numbers muted and "AFK 7m" (or "AFK") in the ping column. The
  places above count them, so ties and the header's place still agree.

A full server fits the board's canvas (320 text runs, 1024 draw commands) in
every mode, at 1080 lines and 4K, 5:4, 4:3, 21:9 and 32:9 (unit tests, which
also pin the compact board's centring, its clearance of the chat column and
its widths). The look draws in the UI's
families, which load in every menu style; their metrics place what follows a measured run (the emblem after a
name, the header's right side).

## In-game menu

[sjk_view.rs](../crates/sjk-viewer/src/ingame_menu/sjk_view.rs) draws it,
[sjk_dock.rs](../crates/sjk-viewer/src/ingame_menu/sjk_dock.rs) its row of icons and
the match card's controls, [sjk_focus.rs](../crates/sjk-viewer/src/ingame_menu/sjk_focus.rs)
holds what the row and the card offer and how the keys move between them, and
[sjk_actions.rs](../crates/sjk-viewer/src/ingame_menu/sjk_actions.rs) what they do.
Escape in a match opens it; the match keeps drawing, under a dark fade from the left
edge (deep there, clear by the middle), on the main page's 16:9 frame. Sol asked for
fewer buttons on 09/10/2026: the list has six entries, Team and Vote moved onto the
match card, and Camera control and SJK's pages into a row of small icons (the SJK page
is gone); the same day Achievements became a tab of Profile, leaving six. On
10/10/2026 the Collection (medals, achievements, shaders, toys, nameplates) got an
entry of its own under Profile, making seven.

- **Arc:** a compact version of the main page's: the page's entries on an arc
  whose centre lies off the frame's left edge (radius 580, the middle entry at x
  440), 68 apart (50 for eleven or twelve, 36 for more, smaller type), the
  chosen one gold and larger with a line under it saying what it opens (while the
  list has the keyboard; white and without its line while the row or the card has
  it); a page's name stands small over its first entry (not on Leave). Beside them a lit holo rail
  with a gold mark that eases to the chosen entry, and inside the curve SJK's emblem
  in its turning ring. Leaving entries are ember when chosen; Back and Stay are
  quiet, as is the main page's Leave.
- **Pages:**
  - Main: Resume, Profile (the [Profile screen](#profile-screen), on the tab shown
    last, Character the first time), Collection (the [Collection](#collection), on
    the tab shown last, Medals the first time), Players, Settings, Servers, Leave.
  - Players: a small scoreboard in place of the arc ([identity.md](identity.md#player-reports)):
    the title and how many are on the server (or why the player cannot report, in
    ember), then a table from x 340 to 1330 under a darker fade: a team's colour
    mark, the name in its colours, score, ping ("bot" for a bot) and what the SJK
    hub knows ("Verified" in gold, "SJK" in holo, "You"), 37 apart, sixteen a page
    with More players... and Back; the chosen row has a gold band and edge. The
    card on the right becomes the chosen player's: name, slot and side, what the
    hub knows (key and hub name) and their medals (medallion and name, two a line;
    [identity.md](identity.md#medals)), score and ping, and "Enter: report this player"
    in gold, or why not after an ember mark. Enter opens Report.
  - Report a player: the seven reasons on the arc with what each covers, and Back;
    the player's card stays on the right. Without the right to report, the
    reasons are dimmed and passed over, and Back's line says why. A reason opens
    the report card ([Report a bug](#report-a-bug-and-its-dialogs)); Escape
    returns to the player's row.
  - Team (the J key's, `teammenu`): Join the game and Spectate, or in a team game
    Auto-join, Red team, Blue team (each with its colour and players) and Spectate;
    the side the player is on is quiet ("Your team", "You are watching") and passed
    over by the keys. In Siege the card's Class and side opens the class page in two
    steps, as the team page: "Choose your team" with Auto-join (`team free`: the
    side with fewer players, in the class the server settles; the cursor on it
    while the player watches), Red team and Blue team (each
    with its colour mark and players, "Your team" on the player's own, the cursor
    on it), Spectate (dimmed and passed over while the player watches) and Back; a
    side then lists its classes by name alone ("Red team classes", Next page past
    eight, Back to the team choice, where Escape also goes). The classic look draws
    the same two steps in its pop-up: the side step centred with the Team page's
    flags and player counts, the classes as a list. The shared rows, a row's detail
    after its label ([ingame_menu/siege.rs](../crates/sjk-viewer/src/ingame_menu/siege.rs)).
  - Call a vote (the card's link): its lists (map, game type, kick, warmup, limits)
    are the shared call-vote lists. Calling a vote returns to the match, as
    retail's pop-ups do.
  - Leave: Leave the server, Quit to desktop (ember when chosen), Stay. It opens on
    Stay, since its rows act at once. It has no title over the arc; its rows say
    what they do.
- **Row of icons:** under the emblem, from (96, 852), icons 54 across and 66 apart:
  Camera control (the settings `camera` icon; F8 opens it too), What's new (the
  wheel's `whats_new`), Credits (`credits`), Report a bug (`other_controls`, the
  tools), SJK chat (`text`, the quill) and, only for a staff key (the hub's `staff`
  flag, `profile_card::Summary::staff`), Map editor (`maps_own`, the globe) and Staff
  tools (`identity`), dimmer than the others. Each is drawn at 78 % (those two at 50 %)
  and, hovered or focused, whole with
  a gold ring (bright gold when focused); the one hovered or focused is named over
  the row in gold (Rajdhani 24) with its key cap (F8 for Camera control) and what it
  opens. Only on the main page. What's new, Credits, SJK chat and Staff tools open
  their console pages over the menu, which shows again when they close; Report a
  bug opens the report card; Camera control its panel; Map editor turns the editor on
  and opens its panel ([Map editor](#map-editor)).
- **SJK chat:** the main page's docked chat ([Main page](#main-page),
  [hub-chat.md](hub-chat.md#in-the-menus)), on the main page only and while
  `cl_sjkChat` is on: in the match card's column under its last control (x 1360, the
  card's 464 wide, from y 770, its Open chat line near the frame's foot, right of the
  keys; Sol's layout of 11/10/2026, which raised the card and left the middle of the
  screen to the match), over a soft dark pool of rounded layers so it reads over the
  match. Its messages, field, Open chat and sender cards behave as on the main page; a
  sender's card shows left of the dock, level with the name and above the keys, where
  the match is clear (shapes never cover text, so beside the name it would lie over the
  dock's lines). Without the identity it says "Turn the SJK identity on to chat", as
  the main page does. Typing in it, every key goes to the field before the console
  key and the game's bindings (`GpuState::sjk_chat_typing`, ahead of
  `route_console_key`); Escape stops typing, and leaving the menu drops the draft.
- **Profile card:** the main page's [profile card](#profile-card), bottom left at
  the same place, under the row of icons, on every page whose arc leaves it room
  (every page of up to twelve entries; not Players and Report a player, whose table
  and player card take the screen). Hovering lights it, a click opens the Profile
  screen on its SJK Profile tab. The Players page's player card shows the chosen
  player's picture (or their initial) before their name when the hub knows them.
- **Match card:** on the right (x 1360, 464 wide, from y 218) over its own fade: the
  server's name with its colours and its address, a rule, the map without
  `mp/`, its mode and limits in words ("FFA, 30 frags, 20 minutes"), then the
  numbers: the player's score and place ("3rd", "of 14"; "Tied" when tied) or,
  in a team game, both teams' scores (the player's marked "Your team") and the
  player's score, and the clock (time left of the time limit, or played); under
  a rule how many play, watch and fit. It reads the live session each frame and
  the server's info once a second; without a server (a map explored alone) it
  hides, with its controls. On the main page it carries the controls:
  - **The vote on**, over the card from y 66: "A vote is on" in gold with the count
    ("4 yes, 1 no") at the right, the question (Rajdhani 30, two lines at most), and
    Yes and No as gold kit buttons (140 by 40). Voting returns to the match, as
    retail's pop-up does.
  - **Your side**, from y 600 under the numbers: "Your side" and the side (Red team
    or Blue team in its colour, Playing, or Watching), then the kit's buttons, three
    to the card's width: Join red and Join blue (a mark of the side's colour on each)
    and Spectate in a team game, Join and Spectate otherwise, Class and side (200
    wide, Siege's class page) and Spectate in Siege. The side the player is on is
    dimmed and passed over. A side sends `team red`, `team blue`, `team free` or
    `team s` and returns to the match, as the Team page does.
  - **Call a vote**, quiet at the card's foot while no vote is on (muted Rajdhani 21,
    gold and underlined when hovered or focused, with "Map, mode, kick, limits"
    after it), opening the call-vote page.
- **Keys:** bottom centre, what has the keyboard: the list's (Up Down choose, Tab
  icons and match, or icons, match, chat with the chat docked, Enter open, Esc
  resume), the row's (Left Right choose, Tab match, Enter open, Esc list), the card's
  (Arrows choose, Tab list or chat, Enter vote, join, spectate or open, Esc list), the
  chat's (Enter type, Tab list, Esc list; while typing Enter send, Esc stop typing,
  Backspace erase), and a page's (Up Down choose, Enter open, report on Players, write
  on Report a player, Esc back).

Keyboard first: on the main page Up and Down move along the list (passing over
nothing: every entry can be taken); Right from the list enters the card on its first
control; Tab moves the keyboard from the list to the row of icons (its first), from
there to the card (its first control that can be taken), then to the docked chat's
field and back to the list, Shift+Tab the other way, a part with nothing to offer
skipped. In the row Left and Right move between the icons (Right from the last
reaches the chat), Up returns to the list. On the card Left and Right move along a
line of controls (the vote's Yes and No, the side's buttons; Left from a line's first
returns to the list), Up and Down between its lines, nearest the control's place (Down
from the last reaches the chat). On the chat's field Up returns to the list, Left
reaches the row's last icon, Right the card's last control. Enter acts on what has the
keyboard (on the chat it starts typing); Escape stops typing first, returns to the
list from the row, the card or the chat, and from the list resumes the match. A control that goes away (a
vote ends, Staff tools for a key no longer staff) gives the keyboard to the card's
first control, or the list. Escape on a page returns to the main page on the entry
that opened it, or on the card control that did (the side's buttons from Team and
Siege, Call a vote from the call-vote page; a list returns to its row of Call a
vote). The pointer: hovering an entry, an icon or a control gives it the keyboard, a
click acts. Settings and Servers hand over to their screens over the match and come
back on their entry, Settings opened on the category last shown, both with "Game
menu" as their way back; Profile hands over to the Profile screen, which comes back
on Profile. Server info and Controls have no entries: the card shows the server and
Settings holds the key bindings. The Report a bug button the other looks put at the
bottom is left out; the row of icons has it.

The classic menus keep their own in-game bar (`classic.rs`): its Join, Vote, Call
Vote, Profile and SJK pop-up are unchanged, and F8 (or `cameracontrol`) opens Camera
control there.

## Profile screen

[profile_hub.rs](../crates/sjk-viewer/src/profile_hub.rs) (09/10/2026, Sol's requests:
one Profile in the menu for the character and the SJK profile, then one row of tabs
for all of it; 10/10/2026 the achievements, medals and unlockables moved to the
[Collection](#collection)). Existing screens shown as one, under one row of four tabs:
Character, Saber and Force (the player screen's pages) and SJK Profile (the console's
Profile page: picture, bio, record, the achievements and collection in brief). Each
keeps its state, layout, keys and pointer; the row is the player screen's own tabs
grown to four. The same module serves the Collection screen's row
(`profile_hub::Screen`): a tab belongs to one screen, Ctrl+Tab stays within it.

- **Row:** under the title (the player's name, with the way back before it: "Game
  menu" or "Main menu"), from x 96 at y 146, the names in Rajdhani at 26, 48 apart,
  measured from the font (`text::display_width`); the one on show gold with a gold
  underline, one hovered brighter. Whichever screen shows draws it at the same place
  (`profile_hub::tabs`, `profile_hub::header`), so switching between the player
  screen and the console's page keeps the title and the row still; the console's
  page moves what it shows down 44 pixels under it.
- **Keys:** Ctrl+Tab shows the next tab, Ctrl+Shift+Tab the one before, wrapping,
  from any tab and even while a field is typed in. On Character, Saber and Force, Tab
  (Shift+Tab back), `]` and `[` walk all four, as they walked the three pages
  (unless a field there is typed in); on SJK Profile Tab moves within the page, as
  everywhere in the UI. Each tab's keys line names the next tab. A click on a tab
  shows it; the click never reaches the screen under the row (the switch hit-tests the
  row itself, `profile_hub::tab_at`). Escape leaves as each tab's own way back does:
  to the game menu on its Profile entry (the console's page closes over the game
  menu, which waits under it), or to the main page.
- **SJK Profile tab:** the page without its top bar's title (the keyboard starts on
  the picture), its medals left to the Collection and its Identity settings button
  gone (the identity is in Settings, Network); Change name, before the Name emoji and
  Staff tools buttons, opens the Character tab's Name field (`PanelAction::EditName`);
  See the board and See the collection open the Collection screen on Achievements and
  Medals.
- **Opened by:** the in-game menu's Profile (the tab shown last in the run, Character
  the first time; the tab a console page turned to itself counts too) and its profile
  card (SJK Profile); the main page's Profile (Character, on the menu map's stage) and
  its profile card (SJK Profile). In the SJK UI the `profile` command opens it on SJK
  Profile (again to close it), returning to the main page from the menus, else to
  the game menu; with the classic menus it opens the page on its own, as before.

## Collection

[collection_panel.rs](../crates/sjk-viewer/src/collection_panel.rs) and its views
(`collection_view.rs`, `collection_medals.rs`, `collection_achievements.rs`,
`collection_shaders.rs`, `collection_toys.rs`, `collection_nameplates.rs`,
`collection_cards.rs`, `collection_forge.rs`), 10/10/2026,
Sol's request: take the achievements, medals and unlockables out of the Profile screen
into a screen of their own, "a real collection, things players can collect in the
future", with tabs Medals, Achievements, Shaders (the blade skins, renamed: saber now,
body later), Toys and Nameplates (ornaments for the nameplate), then Holocrons (the
loot-box drops, built the same day as a Profile tab and moved here), since 21/10/2026
the Forge (the shop, crafting and breaking shaders down) and, since 12/10/2026, Cards
(what opening holocrons brings, [cards.md](cards.md)), and since 27/10/2026 Tags after Toys
(the tags `toy_tag` sticks, [client.md](client.md#tag): a wall of tiles, the ones not held
dimmed, the chosen one on a card with Choose, which sets `cg_tag`; with nine tabs the row
shrinks its names when even its narrowest gaps leave them too wide; since 31/10/2026 the
wall scrolls, a line a wheel notch, the arrows (and Page Up, Page Down, Home, End) keeping
the choice in view with its heading, a bar at its side; the credits cards a hub lets be tags
follow the tags under "Credits cards"; a tile badges its Holo and Solar copies, and the card
offers Normal, Holo and Solar chips, the finishes held, shown over the picture, Choose then
setting `cg_tagFinish` too), and since 31/10/2026
History last (what came, was opened, spent and traded, [cards.md](cards.md#history)); while
the Saber Lab is on, a tab after History, Saber Lab ([client.md](client.md#saber-lab)). Sol picked the
direction from a design canvas (`the vault`): every thing has its place from the
start, the ones the player lacks dimmed; each tab shows its things on the left and
the one chosen up close on the right.

It is one console page (like the Profile page), drawn in this look in every menu style:
in the SJK UI the main page's and the in-game menu's Collection (its way back "Main
menu" or "Game menu"), else the `collection`, `achievements` and `unlockables`
commands open it on its own ("Back"); the `holocrons` command opens its Holocrons tab
(the Holocrons page). It remembers its tab and what is chosen on each
for the run.

- **Top:** the way back and "Collection" as the title; at the top right how many of
  all things the player holds ("15 of 30 collected") over a thin bar as wide as
  those words, a stretch to each tab in the tabs' order, filled gold as far as that
  tab's share held (one tick a thing until 21/10/2026, a barcode at two hundred things). Under the title the row of tabs
  (`profile_hub::collection_header`), each name with its count after it in small
  Rajdhani ("2/3", "11/34", how many holocrons for Holocrons; nothing while unknown or empty), the one on show gold and
  underlined. Drawn in Inter (the families not loaded, the classic menus), the row
  makes room for the wider face. Over the map (Shaders, Toys, Nameplates) a band of
  the page's dark stands behind the title, the tally and the tabs, solid to y 200 and
  fading out by 290, as on the Holocrons page (25/10/2026, Sol: the tabs must read over
  any map, the bright pillar of duel6 included).
- **Fit (25/10/2026, Sol's pick from the Collection revamp's design canvas):** every tab
  keeps to one frame: its things at x 96 to 960 (they grow down and scroll, never
  sideways into the stage), the stage's column x 1000 to 1320 (the model, the lone
  saber or the 3D holocron), the one chosen at x 1360 to 1824 (from 1000 on Shaders,
  with no model) on a dark card, and the keys' line kept clear. A card's words are never
  cut: it grows upward to hold them, wrapped for the menu text size. Behind the stage
  the backdrop camera widens its field in a window narrower than 16:9
  (`menu_stage::frame_fit_fov`), so the frame sees what a 16:9 window does and the model
  stands in its column at 4:3 too (it used to drift to x 1200 to 1420 in frame pixels,
  under the words and the keys).
- **Medals:** "2 of 4 medals" (or why they are not known) over a line; every medal
  the client knows hangs by its ribbon from a lit holo rail (x 96 to 1000, y 336, a
  bead at each end), up to 226 apart, the ones given first in colour on a soft gold
  glow, the others dark; under each its name (with the count for one given again) and
  "Given dd/mm/yyyy", "Given 2 times" or "Not given yet". The one chosen has a gold
  mark on the rail at its clasp and a brighter glow; up close on the right (x 1120 to
  1824) the whole medal at 400 on its glow, its name at 54 (gold when given), when it
  was given, what it is for with the saber shader it brings ("Comes with the Glitch
  blade, a saber shader."; nothing for a medal that brings none, Supporter), the team's note in quotes under "From the SJK team", and
  who sees it. A dotted rail and "More medals will hang here as the SJK team adds
  them." stand under the rail. Left and Right choose; hovering chooses.
- **Achievements:** "11 of 21 unlocked" with a gold bar; then a wall, a row to each
  category (Combat, Duels and flags, Journeys, Community) under its name in its colour
  and "5 of 9", each achievement's medallion (`achievements::medallion`, radius 33,
  104 apart: the ring filling with the count in the category's colour, filled and
  gold once unlocked, the goal inside) with its name under it, the one chosen ringed
  bright gold. Up close on the right: the medallion at radius 104 on a glow, the name
  at 50, the category, what to do, a bar with "19 / 25" and "6 more to go" or
  "Unlocked dd/mm/yyyy"; under it "Next up", the three locked ones nearest to
  unlocking with their medallion, bar and count; and how the counts are kept. It opens
  on the locked one nearest to unlocking. The arrows choose (Up and Down to the next
  category's, near the column).
- **Shaders** (the wall and the list since 25/10/2026,
  [collection_shaders.rs](../crates/sjk-viewer/src/collection_shaders.rs)): "14 of 166
  owned" (or why that is not known) over a line, beside the view switches (three bars:
  the list; four squares: the wall) and the kinds as pills: Saber with its count, and
  Body ("soon", not offered yet). Under the line Owned (O), Chroma (C, the shaders that
  take the wearer's colour) and the search field (`/`, Ctrl+F) to the list's right
  edge; under them the tiers' strip: All, then each tier the player may see, the rarest
  first (Unique's first of all, only for a player who holds one), each its mark and name
  in its colour, "2/12" held and a bar of that share; a click or T (Shift+T back) shows
  that tier alone, the one shown lit gold. The stock blade is always the first thing
  (its strip in the player's `color1`), then every blade skin, the rarest tier first
  (Mythical, Legendary, Rare, Uncommon, Common, the holocrons' tiers and colours; a
  Unique shader is never listed, found by the search, counted in "N of M owned", the
  Saber pill, the tab's count or the tally for a player who does not hold one,
  [unlockables.md](unlockables.md#unique-shaders)). Since 29/10/2026 the player's
  favourites stand first after the stock blade, under a gold "Favourites" heading of their
  own on the wall and in the list by tier (simply first in the list's other orders), a
  Favourites chip (Shift+F, after Chroma, only on a hub that keeps marks) shows them alone,
  and the headline is followed by the New count ("2 new" after a New sticker) and Mark all
  seen (N) while a shader is New ([Marks](#marks)). Two views, V or the switches, kept
  in `ui_shaderList` (0 the wall, everyone's start; 1 the list; it replaced
  `ui_shaderView`, whose 0 was the old rack, dropped from saved profiles):
  - **The wall** (from y 400): a heading to each tier ("Mythical" in its colour, "2 of
    12", a line in its colour; the stock blade's "Stock") and its tiles six to a line
    (135 wide, 100 tall at the ordinary text size, taller at a larger one), up to thirty
    shown. A tile: its tier's band along the top and frame, the blade as a strip
    (`swatch::strip`: the live picture when owned, else a still blade in the skin's own
    colours, dimmed when not held, so the ones not held are told apart; grey while its
    pack has not come), the name without "blade" on two lines at most, its state (Worn,
    Yours, "Yours x3", Not owned: "Locked" until 29/10/2026, when the padlock became the
    player's own lock) and the chroma's wheel; its marks: the New sticker on its top edge,
    the star and the padlock on a dark pill over the blade's right end, a New one outlined
    in gold; the chosen tile lifted over a shadow in a white outline. Left and Right step a tile (on into the next tier's), Up
    and Down a line of tiles, to the same column or the nearest, past the headings.
  - **The list** (from y 400): column heads (Shader, Tier and Yours sort it; "#", the
    blade's number, only while `debug_collectibles` shows numbers; Look, Smith and,
    while the hub's economy is known, Forge), then a row of 40 to each: the strip, the
    chroma's wheel and the name, the tier's mark and name in its colour, the smith, what
    the player holds and the Forge's word (what a copy breaks down for, or what crafting
    costs, a blade card's lower price, quiet past the player's kyber shards), the New
    sticker and the star and padlock at the name column's end. By tier a heading leads
    each tier's rows (and the favourites'); by name or by what is held, none. S steps the order
    (Tier, Name, Yours, then Number while numbers show), a click on a head sets it, the
    order on show gold with three bars; the shader chosen stays chosen whatever the
    order or the view.
  Both scroll a line at a time just enough to keep the chosen one in view (a pointer
  over them never scrolls them), the wheel a line a notch, the chosen one kept in view
  in its column, a thin bar at the right saying where; from Rare up the tiles and the
  card carry the tier's animated effects ([unlockables.md](unlockables.md#rarity-effects),
  `ui_rarityEffects`). The
  model holds the chosen blade, a locked one too ("Preview on your saber: not yours
  yet" over the stage): the preview is drawn by this screen only and never sent
  (`collection_panel::PreviewSkin`, `menu_stage`'s `skin_override`). Its card at the
  foot of the right (x 1000 to 1824, its foot at y 990, as tall as its words, its top
  no higher than 436, the lone saber's room above it and "Drag to turn" just over it):
  the tier's mark and name, the edition, the name (gold when owned) with its state as
  a tag (a New sticker after the tier while it is New), its facts, what it is in full,
  "Yours since dd/mm/yyyy, from the SJK team" with the note (", with your Bug Hunter
  medal" when a medal brings it) or "How to get it: ...", and Equip (gold), Unequip or
  "Wear the stock blade"; at the foot's right end, on a hub that keeps marks, the star and
  the padlock as two square toggles (44 wide, lit gold while set; the padlock dimmed for a
  shader not held, which cannot be locked). F stars or unstars the shader chosen, L locks
  or unlocks it. It opens on what is worn; Enter equips or unequips (`cg_saberSkin`). Under an owned skin whose file
  offers parts (11/10/2026): "Parts" and a tick for each in two columns (three past
  four); a click or the number keys 1 to 6 switch one (`cg_saberSkinOptions`), the
  model's blade following at once and, worn, every SJK player's view of it
  ([unlockables.md](unlockables.md#options)).
- **Toys** (rows since 25/10/2026,
  [collection_toys.rs](../crates/sjk-viewer/src/collection_toys.rs)): "10 toys", things
  to use in a match, and how many are on now. Illuminate's holocron, the cardboard box
  (since 16/10/2026), Order 66 (since 17/10/2026), the Hax (since 21/10/2026), the
  Ragdoll (since 23/10/2026), the Drone (since 24/10/2026), the Boombox (since
  25/10/2026, verified players), the Monitor (since 26/10/2026), Box hunt (since
  27/10/2026) and the Banana peel (since 29/10/2026), a row of 64 to each under column
  heads (Toy, Your key, On; ten rows fit over the keys, 3 apart): its icon, its
  name and a line of what it does, the key bound to its command ("Not bound" when none)
  and its own switch, which runs that toy in place and chooses it; the chosen row framed
  gold on a soft gold ground. The arrows and Tab choose; a click on a row chooses it.
  With Illuminate chosen the holocron floats lit by the model on the stage
  (`illuminate::submit_stage_holocron`); the box has no 3D stand-in on the stage, nor
  have Order 66, the Hax, the Ragdoll, the Drone, the Boombox and the Monitor. On a dark
  card beside
  the model (x
  1360 to 1824, its foot at y 990, as tall as its words): "Toy · everyone's", its name,
  what it is in full, its keys (Order 66 two: the join key, `toy_order66`, and the order
  key, `order66`; the Monitor two: its key, `toy_monitor`, and the throw key,
  `+monitor_throw`, held to charge; the Banana peel two: its key, `toy_banana`, and the
  drop key, `banana_drop`), each a button: a click (or K for the first) waits for a key ("Press
  a key...", Escape lets go, Backspace clears), which binds it to the command in place
  of its first key as the key-binding editor does (`rebind_action`, never a locked key);
  "Quick wheel: On its Toys page", a line more for some (the Hax: hold Use on a player
  who has it on; the Ragdoll: its key again gets up), then its switch, "Holocron lit",
  "In the box", "In the order", "Hacker", "Limp", "Peek me", "Play", "Hit me", "Hunt" or
  "Slip on peels": on while
  the toy is on, Enter or a click runs it (the `toy_illuminate`, `toy_box`,
  `toy_order66`, `toy_hax`, `toy_ragdoll`, `toy_drone`, `toy_boombox`, `toy_monitor`,
  `toy_boxhunt` or `toy_banana` toggle, set to the state wanted;
  the viewer owns the state, the console mirrors it each frame). Under it, for a toy other
  players are seen with, a second, smaller switch, "See others' holocrons", "See others'
  boxes", "See others fall limp", "See others' drones", "See and hear others' boomboxes",
  "See monitors" or "See banana peels" (29/10/2026): off, theirs are hidden on this PC
  ([client.md](client.md#illuminate), the toys' table). The Hax's card has its trap's
  two switches there instead (since 03/11/2026, Lex's idea; they were in Settings >
  Gameplay): "Trap: a hacker blows up with you" (`cl_haxTrap`) and "Trap while away"
  (`cl_haxAway`); a Settings search for them (hax, trap) lists them under "Collection >
  Toys > Hax", where they still switch. The keys line says
  "Enter: light" or "Enter: put out", "Enter: hide" or "Enter: come out", "Enter: turn
  on" or "Enter: turn off", "Enter: play" or "Enter: put away", and "K: bind a key" (while a key waits: "Esc: cancel", "Backspace: clear the
  key"). Hovering Order 66's row shows how to turn it on and that `order66` executes it;
  the Hax's, how to turn it on and that `+use` held on a holder hacks them.
- **Nameplates:** none exist yet. "No nameplates yet"; three empty places, dashed, each
  with a drawing of where its kind will go on the plate: Crest (before the name),
  Frame (round the plate), Trail (under the plate). Over the model's head (projected
  from the stage model's head with the last frame's view, `GpuState::frame_view`; over
  the preview's head without a stage) the player's nameplate as a match draws it near
  (in the style chosen, `cg_nameplateStyle`, [client.md](client.md#nameplates), drawn
  with the match's layout at twice its size). The holo frame: their picture (or their
  initial on their colour) in the hexagon, the name in its colours and, while See &
  share nameplates is on, the gold corner and the shield, health and Force bars (else the
  dotted line). The capsule: the picture and the name in the pill, gold-edged with the
  tray of bars under it while sharing. The classic plate: the name over the framed plate
  of bars while sharing, alone otherwise. Dashed marks stand at the three places (the
  crest's before the name).
  Its words stand on a dark card beside the model (x 1360 to 1824, its foot at y 990,
  as tall as they need, since 25/10/2026). Under them, **Style** (31/10/2026: Holo,
  Capsule, Classic, segments whose click sets `cg_nameplateStyle`) and **See & share nameplates** (a switch; a click or Enter flips
  it, its row pulsing gold every 1.2 s, a ring round it and a glow behind the switch)
  puts `cg_nameplate` in plain view: the one setting for seeing other players'
  nameplates and sharing your health, shield and Force
  ([client.md](client.md#sharing-and-estimates)). A quiet line under it says what on or
  off means (or, with the identity off, that sharing needs it).
- **Loadouts** (03/11/2026, Lex's idea,
  [collection_loadouts.rs](../crates/sjk-viewer/src/collection_loadouts.rs)): on Shaders,
  Tags and Nameplates an outlined "Loadouts (P)" button under the tally (x 1624 to 1824,
  y 206; on Shaders it lies in the stage saber's drag room, `SABER_AREA`, which leaves
  it out, hub report #245) opens a pop-up card (`kit::card`, x 440 to 1480, y 170 to 910, over the scrim;
  the page is left out under it): "Loadouts" in gold, "Kept on this PC", the headline
  "Save your look, wear it again in one click" and what a loadout keeps, then five slots,
  a row of 86 each: the number, the name in gold while chosen (or "Empty slot") and a line
  of what it keeps (hilts, shaders, tag), and Wear (gold), Save here, Rename and Empty.
  Save here over a kept loadout and Empty ask for a second press (the ember line under the
  rows says so); the line otherwise says what the last press did ("Wearing Duels").
  Rename turns the name into a field (Enter keeps it, Escape cancels, Ctrl+A, Ctrl+C,
  Ctrl+V and a held Backspace as in every field, at most 24 characters). Keys: Up and
  Down (or 1 to 5) choose, Enter wears (saves into an empty slot), S saves here, R or F2
  renames, Delete empties, Escape closes; under the card the keys line says so. A
  loadout keeps `saber1`, `saber2`, `cp_sbRGB1`, `cp_sbRGB2`, `color1`, `color2` (the hat
  and cape ride on them), `cg_saberSkin`, `cg_saberSkin2`, `cg_saberSkinOptions`,
  `cg_nameplateStyle`, `cg_tag` and `cg_tagFinish`; Wear sets back those that differ, a
  shader no longer held showing stock until held, as `saberskin` sets one. The slots are
  `loadouts.json` in the profile folder, never sent to the hub. Unit tests (file round
  trip, wear sets only what differs, second presses, rename field, the card's fit) and
  world shot `world_shot::collection::duel6_sjk_loadouts`.
- **Holocrons** ([holocrons_panel.rs](../crates/sjk-viewer/src/holocrons_panel.rs),
  [holocrons_panel_view.rs](../crates/sjk-viewer/src/holocrons_panel_view.rs),
  [holocrons.md](holocrons.md#the-holocrons-tab), 10/10/2026, Sol's holocron drops): a
  console page of its own (the Collection's sixth tab; the Collection's row is drawn on it,
  its counts read from the identity, `collection_panel::row_labels`), with the world left clear on the left: a 3D holocron turns
  and bobs there in the look of the tier chosen, with that tier's colour of point light
  (the backdrop camera parks on its own shot: duel6's garden, a cut through dark on the
  tour; the classic style does not hide the world under it); under it the tier's name at 46
  in its colour, a padlock and "None held yet: shown dimmed" for a tier held none of, whose
  holocron shows dark and drained. On the right (x 1000 to 1824, over a dark panel): how
  many holocrons the player holds at 38 (or "Identity is off", "No hub is set",
  "Contacting the hub..." with the reason under it), the four tiers as rows of 60 (the
  tier's picture, its name at 27 in its colour, "60% of drops", the count held at 32 with
  "held" under it, or a dash before the hub answers, a padlock by a zero; the chosen row
  tinted in its colour with a white ring; the counts are the holocrons not opened yet),
  the player's credits at the panel's top right (the credits' mark and the sum, "credits"
  under), the chosen tier's description and in gold what opening one gives ("Opening it
  gives a sealed card and 30 to 50 credits."), "Next holocron in about 20 minutes of play" with "3 of 8 in 24 h" and
  a gold bar, and "Recent holocrons" (times in UTC): the newest ten in two columns, each a
  small picture, the tier's name, `dd/mm/yyyy HH:MM` and, for a gift, "Gift from the SJK
  team" with the note; an opened one dimmed with "Opened". Under the caption, centred on
  the holocron, Open (gold, the oldest of the tier chosen) and Open all (n) (outlined, up
  to twenty), dimmed when there is nothing to open; their ceremony takes the whole window
  ([cards.md](cards.md#in-the-client)). Loot table (outlined, at the right of what the
  tier holds; L) shows every chance over the whole window, Back or Escape returns
  ([holocrons.md](holocrons.md#loot-table)). Keys: Up and Down (Left and Right, Tab, the wheel,
  1 to 4, Home, End) choose the tier, which swaps the 3D holocron at once (it shrinks away
  and the new look grows in); Enter opens one, A opens all (a card asks "Open all 7 holocrons?" first, Open all or
  Cancel, [cards.md](cards.md#in-the-client)), L shows the loot table; a click on a row chooses it;
  Ctrl+Tab goes on to the Forge; Escape leaves. Shop (or S) shows the Forge on its Shop
  (the shop was an overlay of this tab until 21/10/2026). The `holocrons` command opens it as the
  Collection's tab (again to close); the SJK Profile tab's Holocrons block has See holocrons
  (the classic menus' Profile page too, opening the page on its own). Until the
  Collection (10/10/2026, the same day) it was the Profile screen's eighth tab. The classic Profile
  page lists three medals, not four, to make room for that block. World shots:
  `world_shot::holocrons_tab` (`duel6_sjk_holocrons`: each tier, one held none of, the
  identity off, waiting; `duel6_sjk_holocrons_windows`: 4:3, 21:9, 4K, `ui_textScale 1.2`,
  the classic style). Not tried in a game.
- **The Forge** (21/10/2026, Sol's request: "a better interface for the shop, for crafting
  sabers and for disassembling them", a tab of its own;
  [collection_forge.rs](../crates/sjk-viewer/src/collection_forge.rs) for the state, keys
  and requests, [collection_forge_view.rs](../crates/sjk-viewer/src/collection_forge_view.rs)
  for the look): the Collection's tab between Holocrons and Cards, its count the player's
  kyber shards. Under the row, three section chips, **Shop**, **Craft** and **Break down**
  (the last with how many duplicate copies wait), the one on show gold; at the top right
  the player's kyber shards and credits large, each mark and sum over its word, with how
  each is earned in its hover card. Tab and Shift+Tab (or 1, 2, 3) change section, the
  arrows choose, Enter or Space presses the chosen thing's button, the pointer chooses by
  resting and acts by a click; Ctrl+Tab and Escape as on every tab.
  - **Shop:** "Your offers of 21/10/2026" and "New offers in 5 h 12 min, at midnight in
    Paris" (counting down; once due the Forge reads the hub again, once a day), the four
    offers as large cards (408 by 444) across the page: the tier's frame, wash and
    effects, the tier, the live blade, its name, its price beside the kyber shards' mark,
    its state ("Bought today: it is yours", "You hold it already", "1 200 kyber shards
    more needed") and Buy. Under them **Holocrons for credits**, a wide card each: the
    holocron's picture on its tier's glow, its name in its colour, what it holds (or how
    many credits are missing), its price and Buy. Showing the Shop reads the hub's day
    ("Reading the shop..." until it answers).
  - **Craft:** every saber shader of the trade the player does not hold at all (a card's
    or a medal's copy counts as held), its favourites first (a wish list, since
    29/10/2026: F stars the one chosen), then the commonest first so what the shards reach
    leads,
    as a grid of cards (five by three) or a list (six rows), switched by V or the two marks
    at the chips' right (`ui_shaderList`, as on Shaders: the grid first); chips for All and each tier (T
    steps them), Affordable (A) and a search field ("Search shaders", `/` or Ctrl+F; Ctrl+A,
    Ctrl+C, Ctrl+V and a held Backspace, `text_entry.rs`). Each card: the live blade, the
    name, the tier and the cost beside the shard's mark (quiet when out of reach). On the
    right the one chosen big: its tier and edition, its live blade 150 high, its name at
    46, its facts (takes your colour, its smith), what it is, its parts, then "Crafting
    costs" and "You hold" side by side, the shards left after or how many more are needed,
    and Craft. A copy of the shader's blade card lowers the cost everywhere it shows, and
    "Crafting costs (Holo card -50 %)" and the button ("Craft  (shard)  150  Holo card
    -50 %") name the card ([cards.md](cards.md#sjk-blades)). The swatches here and on Shop
    and Break down are the Collection's (`collection_swatch.rs`): with the menu pictures on
    they show each shader's drawing instead of the live blade.
  - **Break down:** the player's own copies of shaders of the trade (by grant, drops,
    crafts or purchases; a medal's or a card's copy is never listed), its favourites first,
    then duplicates with an "x3" badge, last copies after with a "Last" one; chips,
    Duplicates (D), Favourites (Shift+F, on a hub that keeps marks) and the search as on
    Craft. Each card or row carries its marks: the New sticker, the star and the padlock
    over the blade, and "Worn" for a blade either saber (or staff half) wears. On the right
    the one chosen: its name with the star and the padlock beside it (F, L), its copies
    ("3 copies", "Your only copy", Worn, New), "A copy gives" and "You will hold", and
    **Break down a copy +10**; beside it **Break down all duplicates (7) +100** (B), keeping
    one of each. A locked shader (29/10/2026) is listed with its padlock, its button reads
    "Locked", dimmed, with "Locked: you keep it until you unlock it (L)." over it, and all
    duplicates leave its copies out (the chip's count too). A hub that says `shatter_last`
    (21/10/2026) lists last copies too: their button reads "Break down your last copy" and
    opens a card in ember, "This is your only copy", the blade, what it gives, that it
    leaves the collection, and what crafting it back would cost, Keep it (focused) and
    Break it down. When the player wears it, on either saber, the card says so first, in
    ember ("You wear this shader now: breaking it down puts your saber back to the stock
    blade."), with a "You wear it" pill by the title; once the hub answers, the client sets
    `cg_saberSkin` back to the stock blade, and `cg_saberSkin2` to `stock` when the second
    saber wore it by its own choice. All duplicates ask with a card too, "Break down all
    duplicates?", the shaders that give copies up with how many (a locked one left out, and
    a line saying how many are kept), the total, Cancel and Break them down (focused). A
    hub without the abilities lists duplicates only and offers no Break down all.
  - **Asking twice:** Buy, Craft and Break down a copy turn into "Confirm: ..." on the
    first press; a second within five seconds sends it ("Buying...", "Crafting...",
    "Breaking down..." until the hub answers), and the hub's answer reads beside the button
    ("Kamino blade broken down: +5 kyber shards.") or under the offer.
  - **When it cannot trade:** the identity off, no hub or no answer yet, or a hub with no
    economy ("The Forge comes with the hub's shop") show a quiet place with the reason in
    place of the section; while the hub keeps opening to staff or this build is older than
    its minimum client, the lists still show and their buttons are dimmed with the reason.
  - The Holocrons tab's Shop (or S) shows the Forge on its Shop, and the Shaders tab's card
    leads to it: "Craft in the Forge" for a shader of the trade not held, "Break down in the
    Forge" for one of the player's own copies, the shader chosen there. A held blade card's
    "Craft the ... in the Forge" on the Cards tab does the same for its shader. World shots:
    `world_shot::forge_shots` (`duel6_sjk_forge_shots`: every section and state, the cards
    asking; `duel6_sjk_forge_windows`: 4:3 and 4K).
- **Cards** ([collection_cards.rs](../crates/sjk-viewer/src/collection_cards.rs),
  [cards.md](cards.md#in-the-client), 12/10/2026; a binder since 19/10/2026): "5 of 13
  cards found" with each set's progress under it ("SJK Alpha 5 / 13"), the kyber shards
  and credits at the right of the headline (x up to 1080); the filter's chips at y 300
  (All, Owned, Missing; "Rarity: Any", "Kind: Any", "Finish: Any", a click each on to the
  next; O, T, K, F) with "N of M cards" at the grid's right; every card of the packs as a
  tile (140 wide, six by three from y 340, scrolling a line a wheel notch, a bar at the
  right, each naming its card under its picture with its rarity at its foot): sealed
  copies first, a tile per rarity, then the cards by set and number in their best
  finish's light with an "x3" pill, the ones not found yet greyed; Reveal under the grid
  at y 958 with its outcome beside it; and the tile chosen up close on the right (470
  wide from y 150) with its name, the copies by finish and edition (or "Not found yet"),
  its rarity, kind and set (no card shows its number), and when the first was found (or
  its odds). The arrows (Up and Down a line), Tab and the pointer choose; Ctrl+Tab goes
  on to Medals. Enter, Space, I or a click on a tile or on the card up close looks
  closer: the card 660 wide over the darkened tab with its words at its right, the
  arrows or the wheel to the previous or next tile, Escape or a click to put it back. The row's count is the copies held. World shots: `world_shot::cards_shots`
  (`duel6_sjk_card_binder` with the hub's card pack).
- **History** ([collection_history.rs](../crates/sjk-viewer/src/collection_history.rs),
  [collection_history_view.rs](../crates/sjk-viewer/src/collection_history_view.rs),
  [cards.md](cards.md#history), 31/10/2026): "History" and its line, the filter's chips
  at y 320 (All, Drops, Opened, Credits, Shards, Trades, Boxes) with what the read is
  doing at their right (or why it failed, in ember), then a list of 15 rows 41 high from y
  352, full width with a bar at the right when it scrolls: each row a chip of its kind in
  its tier's colour, the sentence, the credits and shards it moved (gains green, spends
  red) and the date on the right; an Open all is one row with a gold + (- once open) whose
  lines stand indented under it, each with a bar of its tier's colour. Load older ends the
  list while the hub has more. An empty list says "Nothing yet", a read under way
  "Reading your history...", a failure "History unavailable" and why. Left and Right
  filter, Up, Down, Page Up, Page Down, Home and End choose, Enter (or a click) opens a
  row or loads older, R reads again, the wheel scrolls three rows; hovering chooses. It
  reads the newest page each time it is shown. Ctrl+Tab goes on to Medals.
- **The model:** behind Shaders, Toys and Nameplates the page darkens only the left
  and the bottom. From the main page the backdrop camera goes to the player's shot
  (`ClientMenu::set_collection_stage`, which first reads the model, sabers and
  cosmetics from the cvars when the player screen is closed) and the model stands on
  the stage (`Backstage::World`); over a match, and in the classic menus, it shows in
  the page's live preview at (760, 110, 820, 840) (`Backstage::Preview`, as the
  in-game Character tab's). Without either the page draws the swatch or the picture
  itself. The words beside the model stand on cards of their own at every window shape,
  and the backdrop camera keeps the model in the stage's column (Fit, above).
- **Keys and pointer:** the keys line names what the arrows and Enter do on the tab,
  Ctrl+Tab and the next tab, Esc back. Ctrl+Tab (Ctrl+Shift+Tab back) changes tab, as
  on the Profile screen; a click on a tab shows it; hovering a thing chooses it; a
  click on Equip, Unequip, the toy or its switch acts.

### Marks

[shader_marks.rs](../crates/sjk-viewer/src/shader_marks.rs) and
[collection_marks_view.rs](../crates/sjk-viewer/src/collection_marks_view.rs),
29/10/2026: the ideas of the players Atrosex (mark the shader worn in Break down, and say
so on the last copy's card) and Kushina (favourites and a lock), extended by Sol. Every
saber shader the Shaders tab, the Forge and the trade window list carries its marks:

- **Worn:** a blade the player wears: the first saber's (`cg_saberSkin`) or the second
  saber's (or the staff's other half's) own choice (`cg_saberSkin2`). The Shaders tab
  keeps its Worn and "2nd blade" states; the Forge and the trade window say Worn.
- **Favourite** (the star, U+E029) and **Locked** (the padlock, U+E022): the player's,
  kept at the SJK hub (its `PROTOCOL.md`, "Collection marks": `GET /v1/holocrons`' `marks`,
  `POST /v1/unlocks/marks`), so they follow the player to any PC and every key of theirs.
  F and L (or the chosen shader's two toggles) set and clear them on Shaders and in the
  Forge; a favourite may be any shader (a wish list), a lock only one the player holds.
  Favourites are listed first everywhere and a Favourites chip (Shift+F) shows them alone.
  The hub holds the lock: it refuses to break a locked shader down (one copy, the last
  one) and to let it go in a trade, and skips it when every duplicate goes; the client
  shows those buttons dimmed with "Locked" rather than failing. A change shows at once
  (`Service::set_mark`) and the hub's answer replaces it; a refusal is said in the console
  and the hub's marks come back with its next answer. The star and the padlock show only on a hub
  whose economy says `marks` (`sjk_identity::MARKS_ABILITY`), detected as `shatter_last`
  is, so against an older hub nothing of them shows and nothing is asked.
- **New:** a shader the player holds and has not looked at since they got it
  (holocrons, gifts, trades, crafts): a gold "New" sticker, outlined in gold on the wall
  (a gold edge down its row in the list) while it is not the one chosen (Creyon,
  31/10/2026), and the Shaders tab's "2 new"
  after the headline. Local, on this PC: `shader-seen.txt` beside `config.cfg`, one shader
  id a line (`shader_marks::FILE`), read at start and written at each change. With no file
  yet, everything held the first time the hub says what the player holds is seen, so a
  player never finds every shader New; a shader no longer held leaves the list (New again
  when it comes back). A New one clicked on Shaders or in the Forge is seen at once, its
  sticker gone (Creyon, 31/10/2026); one reached with the keys is seen once it has stayed
  chosen a second (`shader_marks::Look`, `LOOK_MS`; Sol's decision, 29/10/2026). Resting the pointer never chooses a shader (31/10/2026;
  hovering chose one before), so it never sees one either. Turning to another tab or closing the page
  sees the shader chosen only if it had already stayed its second; Mark all seen (N) sees
  them all. Nothing is New
  while the debug collectibles show.

The marks are read again only when what they come from changes (the hub's marks, the
own profile, the seen-list, the blades worn), never every frame. World shots:
`world_shot_forge::duel6_sjk_collection_marks` (Break down as a grid and a list, a locked
shader chosen, the Favourites chip, the last copy of the worn blade asking, the Shaders
tab's wall, hover card and list). Not tried in a game.

### Hover cards

[menu/sjk/hover.rs](../crates/sjk-viewer/src/menu/sjk/hover.rs) and
[collection_hover.rs](../crates/sjk-viewer/src/collection_hover.rs), 12/10/2026, Sol's
request: every achievement and every thing in the Collection says, under the pointer, how
it is obtained. When the pointer rests on a thing for 0.3 s a small card fades in beside
it (on its right when there is room, else on its left, kept inside the frame): the
thing's name in its colour, how it is obtained, then where the player stands with it.
Moving to another thing starts again; a key hides the card until the pointer moves. It
takes no input.

| Thing | How it is obtained | Then |
| --- | --- | --- |
| Medal | `Medal::how_to_get` (given by the SJK team, and for what) | the blade it brings while held; "Yours: given dd/mm/yyyy" or "Not given to you yet" |
| Achievement | its goal | who counts it (SJK in matches, or the hub); "Unlocked dd/mm/yyyy" or "n of goal so far" |
| Shader | the stock blade is everyone's; a blade skin's `how_to_get` | its tier; "Yours since dd/mm/yyyy", or "Not yours yet" (try it on here), or why holdings are not known (never a Unique one the player lacks); then a line for each of its marks, what it means ([Marks](#marks)) |
| Star, padlock, Favourites chip, Mark all seen | what a favourite, a lock, the chip and New are, kept where | whether the shader chosen is starred or locked and the key that changes it; a padlock on a shader not held says it cannot be locked |
| Toy | everyone's, and where to light it | "Yours" |
| Card (a binder tile) | from opening holocrons: how many holocrons found hold a card of its rarity ([loot table](holocrons.md#loot-table)), one of how many of that rarity | its rarity, kind and set, no number (a sealed tile: picked when revealed); the copies by finish and edition and when the first was found, or "Not found yet" |
| Holocron tier (Holocrons tab) | found by playing, the pace and the daily cap | the tier's share of drops and staff gifts; how many held |

Nameplates have no things yet, so no cards. Text draws over every shape, so the card
asks its canvas to leave out the page's glyphs under it (`MenuCanvas::cover`, dropping
those glyph quads in the text pass); its own text draws in full. Any SJK UI screen can
use it: follow the hovered token, then `hover::draw` as the page's last drawing.

World shots: `world_shot::collection::duel6_sjk_collection` (every tab from the main
page, the Storm previewed, a 4:3 window, the in-game preview, and the page on its own
in the classic menus; the blade skins from `SJK_TEST_PACKS`) and
`duel6_sjk_hover_cards` (a card on a thing of each tab and on a holocron tier).

## Camera control

[shot/sjk_view.rs](../crates/sjk-viewer/src/ingame_menu/shot/sjk_view.rs): the
panel for framing shots ([client.md](client.md#camera-control) has what it
does), opened by the in-game menu's camera icon (its row of icons), F8, the
`cameracontrol` command or the quick wheel's Camera control. Sol renamed it from Shot
controls and asked for this look on 08/10/2026. Its purpose is the scene, so the
match draws undimmed over most of the window, and the panel is a column down the
right edge:

- **Frame:** a 1080-line frame whose right edge is the window's, not the main
  page's centred one, so the column hugs the edge on any window (x 1440, 416
  wide) and a wider window shows more scene. Under it a navy fade, clear at x
  1200, 72 % at 1400, 86 % at the edge, the full height; nothing else darkens the
  scene.
- **Viewfinder:** the screen's one memorable thing. Thin holo corner marks at
  the window's corners (the picture once the panel hides) and small crosses where
  its thirds cross, for placing the subject.
- **Top:** "Camera control" (Rajdhani 42), what it is for, then the pages as tabs,
  Camera and Sun, the one on show gold and underlined (Character's tabs).
- **Camera page:** View from (Back, Front, Left, Right as kit buttons), then
  Framing: Angle, Pitch, Distance and Height.
- **Sun page:** Sunlight: the sun's direction and elevation, a line on what the
  angles mean and one on what holds the sun now (white when Camera control does),
  and Reset sun as a full-width button; without real-time lighting or open sky,
  why the sun cannot be set here.
- **Motion** (both pages, at the same place): Move duration and Orbit speed,
  Orbit, Stop and Reset (Ease back on the Sun page), the Live preview and HUD
  switches, then Hide panel (gold, Move and hide while live preview is off) and
  Close.
- **Sliders:** a row of 56: the name (Exo 2 19) and the number (Rajdhani 21, to a
  tenth, its unit in Exo 2, since Rajdhani's degree sign reads as an apostrophe)
  on one line, the kit's slider across the whole column under them. The
  track's pointer area is exactly the drawn track, so a click or a drag lands
  where it shows; the number is its own area (a click types it).
- **Keys:** two lines at the column's foot, right-aligned: what has the keyboard
  (Left Right adjust and Enter type on a slider, Enter show, switch or act
  elsewhere, Enter apply and Esc cancel while a number is typed), Up Down and F8
  Esc.

The focused control has the kit's band (a slider's or switch's whole row, a tab)
or a button's white edge. It is the panel's state, tokens, keys and pointer
(`shot::Panel`), drawn by another view: `InGameMenu::append_sjk` builds it when
the page is `Page::Shot`, in every menu style (the classic menus' right-edge
panel was removed on 10/10/2026).

## Map editor

[panel_view.rs](../crates/sjk-viewer/src/map_edits/panel_view.rs): the staff map editor's
panel ([map-edits.md](map-edits.md#the-panel) has what it does), shown whenever `mapedit`
is on (the in-game menu's Map editor icon turns it on too), in every menu style
(19/10/2026, Sol's request; made easier 28/10/2026 and 04/11/2026, Sol's requests). It is Camera
control's column: the 1080-line frame on the window's right edge, the column at x 1440,
416 wide, over the same navy fade, the scene left of x 1400 undimmed, where a click
selects and the right button held flies (the selection's green outline is the screen's
one memorable thing; a multiple selection outlines every member). While the editor flies
the column stays drawn, its keys lines turned to the flight's.

- **Top:** "Map editor" (Rajdhani 42), the map and its state (no edits yet, N edits not
  published, or published with the hub's revision), then the pages as tabs: Selected,
  Place, Edits, Weather (Camera control's tabs, 28 apart so four fit the column).
- **Selected:** what it is (Rajdhani 26; "Hidden face and 11 more" for several), its model
  or shader and its detail (Exo 2 16, muted, two lines; on faces the second is Fill's
  shader and share in gold), a row of kit buttons (two; three on a hidden face or a sky:
  Same plane, Connected, Fill), then Draw another shader: the kit's search pill (with its
  count), a list of five rows as Place's (names without `textures/` or `models/`, the
  selection's own shader in quiet print unless chosen, the Recent rows first with a quiet
  "Recent" at their right and a line under the last), and Remap and Remove remap. With
  nothing selected, how to select and to fly. On a sky (04/11/2026) the kind is "Sky", the
  skies lead the list after the Recent rows, and its last row has three buttons: Swap sky
  (Cover side when a texture is chosen), Swap all skies (only with a sky chosen) and Take
  back.
- **Place:** the kit's search pill, a list of six rows (34 each, the chosen one gold with
  a gold tick at its left, paths without `models/`, the Recent rows first), Place where you
  aimed across the column, then Placed model: its line and three rows of four buttons
  (turn and lift, the nudges, scale with Grab, Put down while grabbing, and Remove).
- **Edits:** a list of eleven rows, an edit for another server in quiet print; Delete,
  Undo and Redo (each dimmed with nothing to do); then the Recent rows' switch ("Recent:
  remembered" or "Recent: off") and Forget recent.
- **Weather** (05/11/2026, Sol's request): "This map's weather" heading, the map's own
  weather (muted) and what an edit keeps here (gold; "Kept for this server" when the
  edit is this server's), an ember note while the player forces weather; then Choose: a
  row of four (Drizzle, Rain, Storm, Snow, the preset in force a gold pill), Fog and Wind
  (on: a gold pill), Keep what I see and Map's own, and a muted note of who sees it.
- **Foot:** New edits for and the kit's segments (All servers, This server) at y 800, the
  last answer (ember when refused, or a failed look at the hub) at 840, Publish (gold) and
  Editor off at 898, and three lines of keys from 954, 30 apart (RMB fly, Click select,
  Shift+Click add; G R T gizmo, Esc back, with Shift's fine steps on Place; Ctrl+Z undo
  and the cap of the key bound to `mapedit`, "leave"; while flying W A S D, Space C, Shift,
  Wheel; while grabbing Click, Wheel, Esc; while transforming the gizmo's), then, when the
  free camera was refused, why, in ember Exo 2 15 at 1044. A page's controls end above it
  (a test checks every page at four window shapes, and that every key line fits the
  column). The Aim button and the strip over play (while the panel was hidden to aim) went
  on 04/11/2026: the panel no longer hides while the editor is on.

It is a page of the in-game menu (`Page::MapEdit`), drawn by `InGameMenu::append_own_page`
from the panel's state, which lives with the map's edits; its keys come before the game
menu's (`GpuState::map_edit_key`), its pointer through `map_edit_pointer`. World shot:
`duel6_map_editor_panel` (`world_shot_map_edits.rs`).

## Ghost editor

[editor_view.rs](../crates/sjk-viewer/src/ghost_replay/editor_view.rs): the staff ghost
editor's column ([client.md](client.md#ghost-editor) has what it does), opened by
`ghost edit` (01/11/2026, Sol's request), in every menu style. It is the map editor's
column: the 1080-line frame on the window's right edge, the column at x 1440, 416 wide,
over the same navy fade, the scene left of x 1400 undimmed, where the run's path is the
screen's one memorable thing (holo blue, the new start a gold ring with its facing line).

- **Top:** "Ghost editor" (Rajdhani 42) and how many ghosts, how many on this map.
- **Ghosts** (heading at 166): a list of six rows from 182 (34 each), the name left and its
  map and length right (Exo 2 14), another map's quiet, the open one gold with the list's
  gold tick. Under it at 412 the copy's name (the kit's field, 260 wide) and Duplicate.
- **Cut** (heading at 494, "keeps N s of M s" right of it): the timeline at 510 (36 high)
  on the list's ground, the slices standing (holo 14 %), moving (50 %), in the air (85 %),
  the bubble (ember, faint while it is to be cleared), the cut parts under navy at 70 %,
  the handles as gold bars (the start ember when it is not clean); the clean starts as a
  green line at 550. Start and End rows of 56 at 566 and 626 as Camera control's sliders
  (name Exo 2 19, value Rajdhani 21), then First move, Last move and Reset at 694, and
  at 744 "Starts mid-move" in ember, or the map a run of another map shows its path on.
- **Foot:** "Clear the chat bubble" with how many times and the kit's switch at 800, the
  last answer (ember when refused) or "Not saved" at 840, Save (gold), Play, Look and Close
  at 898, two lines of keys from 964 (Left Right snap, Shift free, Page Up Page Down on a
  handle; Enter, Ctrl+S, Up Down elsewhere; Esc look).
- **Strip:** while looking about, a navy card at the top centre (the map editor had one
  until 04/11/2026): "Ghost editor" in gold, which
  path is drawn, and Esc "back to the editor".

It is a page of the in-game menu (`Page::GhostEdit`), drawn by
`InGameMenu::append_own_page` from the editor's state on `Ghost`; its keys come before the
game menu's (`GpuState::ghost_edit_key`), its pointer through `ghost_edit_pointer` (a press
on the timeline or a track starts a drag that keeps to it). The path and the strip are the
editor's own canvas, a layer under the in-game menu's. World shot: `duel6_ghost_editor`
(`world_shot_ghost_editor.rs`).

## Report a bug and its dialogs

[text_dialog_sjk.rs](../crates/sjk-viewer/src/text_dialog_sjk.rs) draws the
text dialog as the SJK UI's pop-up card (Sol's request, 08/10/2026). One dialog
serves three things, and all three take this look with the SJK UI's menus:
Report a bug (the in-game menu's row of icons, and the main page's SJK page, where the
report names the map `menu` and no server), a player report's few words
(Players, a player, a reason) and a world note (`worldnote` twice,
[client.md](client.md#player-card)). The classic+ look stays with the classic
menus ([identity.md](identity.md#bug-reports)).

- **Card:** 920 wide and 592 tall, centred on the frame, with the browser
  prompts' glass, shadow and holo edge (`kit::card`), over the scene darkened
  all over as under the other cards (78 %); a note's scene is darkened by half,
  so the surface it is about still shows round the card. The menus and every
  text drawn before it are left out under it.
- **Top:** what it is in gold (Report a bug, Report a player, Note for Claude)
  and, muted on the right, where it goes ("To the SJK team, signed with your
  identity", "Kept on this PC with a screenshot"); the question as the headline
  (Rajdhani 40: "What went wrong?", the player and the reason, "What should
  change here?"), cut with an ellipsis when too long; a line or two of
  guidance (for a note, the surface it is about).
- **Field:** six lines of the text (Exo 2 19) wrapped by the font's width in the
  player's text style (`ui_textScale`, `ui_letterSpacing`; the wrap, the caret
  and the clicks measure as the renderer draws, `text_dialog_field.rs`, which the
  Profile page's bio shares, its line breaks starting new lines); the
  lines scroll to keep the caret in view; a gold caret bar at the insertion
  point while it has the keyboard, lit steadily while typing; "Type here" while
  empty (held clear of the caret); outlined white while focused.
- **Note:** a note's card is taller (756) and stays centred: under the guidance a
  model row names the model the note is about ("MODEL" in gold, its path) with
  Clear model at the right, or, without one, says a click on the scene round the
  card notes what is there; the field shows ten lines (13/10/2026). A press on the
  scene round the card selects what is under the pointer for the note.
- **Under it:** the rules ("Letters, digits, spaces and . , ! ? ' - : ( ) _ / \ + = " ;
  [ ] < > # only") and the count against the limit (600, 300 for a player report, 4000 for
  a note; gold at the limit), then why a Send is refused in a warm red band just
  above the buttons: after a Send was refused, and already while Send has the
  keyboard or the pointer and the text would not pass.
- **Foot:** Cancel and Send (gold, dimmed until the text would pass the hub's
  rules; Enter or a click then says why).
- **Keys:** under the card, bottom right: with the field's keyboard Ctrl Enter
  send, Enter new line, Ctrl V paste; on a button Enter send (or cancel); Tab next,
  Esc cancel.

After Send a report keeps its card; the other looks close and show the outcome
as a centre print, as the card does when it was closed first:

- **Sending:** "Sending...", a turning gold arc and "Waiting for the SJK hub" at
  the foot, the text dimmed, Close.
- **Sent:** "Sent. Thank you!" over the hub's number ("The SJK team has it as
  report #12."), a gold tick, Done (gold).
- **Not sent:** "Not sent" over the reason as a sentence: the identity off
  ("The SJK identity is off (cl_identity 1 turns it on)"), the hub out of reach
  or its refusal (a quota); "Your text is kept", Edit (gold: back to the text as
  it was) and Close.

A note closes on Send in every look, since its screenshot is taken of the next
frame; its outcome stays a centre print and a console line.

Keys: Tab and Shift+Tab move between the field, Send and Cancel (and a note's
Clear model, last); Ctrl+Enter sends from anywhere; in the field Enter starts a new
line (Groark, hub #227: a report went off mid-sentence; the hub stores the line breaks as
spaces), on Send it sends (on Cancel, cancels; on Clear model, clears it); Space acts on a button and types in the field; Escape
cancels. In the field the text has an insertion point: Left and Right move it
(with Ctrl by words), Up and Down by wrapped lines (from the first line to the
start, from the last to the end), Home and End to the ends of the line (with Ctrl
of the text), Backspace and Delete remove at it (with Ctrl a word), typing and
pasting insert at it within the same limits, and a click puts it between the
glyphs under the pointer. With Send or Cancel focused the arrows walk the buttons. After Send, Enter or Space takes the focused button (Close, Done or
Edit), Tab moves between Edit and Close, Escape closes. The pointer: a click on
the field gives it the keyboard (on a failed report's text, edits it), on a
button acts.

## New medal

[medal_popup/sjk_view.rs](../crates/sjk-viewer/src/medal_popup/sjk_view.rs): the pop-up
a medal from the SJK team arrives in ([identity.md](identity.md#medals)), on the main
page or over the game menu. Sol asked on 08/10/2026 for its Next button in the SJK
UI's style (it was the retired modern style's) and for the medal to arrive animated,
with a sound. No card: the medal floats in the middle of the scene darkened by the
UI's navy (80 %, coming in over 0.3 s with the sitting's first medal), as the
screen's one memorable thing, on the main page's 16:9 frame.

- **Top:** "New medal" in gold (Rajdhani 24; "New medal · 1 of 3" when several
  wait; "Promoted to Holo" when its rank rose, [Medal ranks](identity.md#medal-ranks))
  over "From the SJK team" (muted, Exo 2 17).
- **Medal:** its whole picture in its rank's light, 340 across, from y 172, centred; behind its
  medallion a soft gold glow, a thin gold ring and a ring of holo ticks turning once
  every four minutes, as the main page's ring does.
- **Words:** the name with its count in gold (Rajdhani 56, centre line at y 600),
  what it is for (Exo 2 20, up to two lines), "Rank III Engraved · Given dd/mm/yyyy"
  (muted, 16), and
  the team's note in quotes (Exo 2 19, up to five lines of 880, wrapped by measured
  width).
- **Button:** the kit's button (gold, 200 by 46), Next while more wait, Close on the
  last, centred under the words; its pointer area is the pill. A click anywhere
  else acts as it too.
- **Keys:** bottom right, Enter next (or close).

The ceremony ([award.rs](../crates/sjk-viewer/src/medal_popup/award.rs), shared with
the classic+ look in retail's gold): the medal comes down into place growing from a
third of its size and lands a little past it (ease-out, about 0.5 s), light bursts
from the medallion (glow, two gold rings, a flash, twenty falling sparks), a gold arc
sweeps the ring, a band of light crosses the medal, and the words fade up and rise
into place one group after another, all within 1.8 s. Waiting, the glow breathes,
a softer band crosses every 6 s and six sparkles twinkle. Enter, Space, Right,
Escape or a click finish the entrance at once, then take the button: the medal lifts
away and everything fades in 0.32 s. Only the medal moves on its own, and only
because it arrived. Every moment stays under 260 of the canvas's 320 draw commands
and its 32 text runs, with the button above the keys inside the window (a test
sweeps every medal with no note and the longest ones, alone and first of several,
nine moments, 1080p, 4K, 4:3, 21:9 and 720 lines, in the families and in Inter).

## Graphics reload

[graphics_reload_card.rs](../crates/sjk-viewer/src/graphics_reload_card.rs): the pop-up
that offers the [graphics reload](client.md#graphics-reload) once a setting the
graphics read only at their start changed (Sol asked on 10/10/2026 for a warning and a
reload instead of a restart, after turning Ultra low, now Vanilla graphics, on
mid-game changed nothing). It
shows on the main page or over the game menu, never inside Settings, so several rows
can be changed first, and after the medal and holocron pop-ups; the menus under it are
not drawn and take no input. The same look in every menu style, on the 16:9 frame:

- **Card:** the kit's card over its scrim, 760 wide from x 580, y 372, as tall as
  its words.
- **Words:** "Graphics" in gold (Rajdhani 22), "Reload the graphics?" (Rajdhani 36),
  then up to three lines of Exo 2 18 (muted): what changed ("Vanilla graphics", "Balanced
  graphics quality", or up to three settings by name, then "and N more") "applies
  after a graphics reload", and that the map (or the menu map) loads again in a few
  seconds, on a server without leaving it.
- **Buttons:** Later (outlined, 150) and Reload now (gold, 200), right-aligned along
  the foot; keys bottom right: Enter reload now, Esc later.
- **Reloading:** on the main page the card stays while the menu map is built again,
  without its buttons: "Reloading the graphics..." and that it takes a few seconds.
  On a server the loading screen shows instead.

## Small graphics cards

[small_card_notice.rs](../crates/sjk-viewer/src/small_card_notice.rs): the pop-up that
tells a player on a card under 3.5 GiB (integrated GPUs included) what SJK lightened at
start (Sol's choice, 25/10/2026, after the low-end audit): Vanilla style instead of the
untouched SJK (recommended) preset, other players' blades stock (`cg_stockBlades 1`) and
a 60 FPS menu cap (`com_maxfpsMenu 60`), each only from its default and once a card
(`r_smallCardPreset` keeps the MiB it was considered for;
[rendering.md](rendering.md#gpu-memory)). It shows once a start on the main page, after
the medal, holocron and reload cards and before the low-FPS card; the menus under it are
not drawn and take no input. The same look in every menu style, on the 16:9 frame:

- **Card:** the kit's card over its scrim, 800 wide from x 560, y 340, as tall as its
  words.
- **Words:** "Graphics" in gold (Rajdhani 22), "Lighter graphics for your card"
  (Rajdhani 36), then up to five lines of Exo 2 18 (muted): the card's video memory,
  what changed (and "with smaller textures" when the texture detail is lowered), and
  that each can be changed in Settings.
- **Buttons:** Back to SJK graphics (outlined, 270) and Keep (gold, 150),
  right-aligned along the foot; keys bottom right: Enter keep, Backspace back to SJK
  graphics (Escape and Space keep too). Back to SJK graphics puts the preset back with
  the player's kept values (as the renderer command does) and turns the stock blades and
  the menu cap off where SJK set them; the reload card then offers to apply the preset.

## Low FPS help

[fps_help_card.rs](../crates/sjk-viewer/src/fps_help_card.rs): the pop-up of the
[low-FPS help](client.md#low-fps-help) (Sol asked on 10/10/2026 for a suggestion when
a map's lighting is heavy, and a `help_fps` command to tell players). It shows on the
menus or over the game menu, which it opens when it comes by itself in a match and
closes again with it, after the medal, holocron and reload cards; the menus under it
are not drawn and take no input. The same look in every menu style, on the 16:9 frame:

- **Card:** the kit's card over its scrim, 800 wide from x 560, y 330, as tall as
  its words.
- **Words:** "FPS" in gold (Rajdhani 22), then (Rajdhani 36) "This map is heavy on
  SJK's lighting" when offered, "Low FPS?" when asked, "Vanilla graphics are on" under
  them; up to six lines of Exo 2 18 (muted): this map's GPU frame and lighting times
  and the FPS they allow, when measured, then what Vanilla graphics turn off and that the
  map loads again, on a server without leaving it.
- **Foot:** the kit's tick and "Don't suggest this again" at the left; Not now
  (outlined, 150) and Use Vanilla graphics (gold, 260; Turn Vanilla graphics off when on) at
  the right; keys bottom right: Esc not now, Enter use Vanilla (turn off). The tick
  takes a click only, and Space does nothing (a jump took the offer, hub report #194,
  29/10/2026); no key or click counts for 0.6 s after the card opens. "EJK graphics" was the name until 12/10/2026.

## Alpha notice

[alpha_notice_card.rs](../crates/sjk-viewer/src/alpha_notice_card.rs): the pop-up a
player gets at the first start (Sol's request, 16/10/2026). It says SJK is an alpha
build of a free and open-source project with bugs and unfinished parts, how to report
them, and that it comes with no warranty and owes the player nothing. There is no way
past it but ticking "I understand" and Continue, or Quit. `ui_alphaNotice` (archived)
holds the notice's version the player accepted; raising `VERSION` in the code shows it
again after the words change, and `ui_alphaNotice 0` shows it again. It shows over the
main menu or the game menu, never in the middle of play, before every other card, and
First setup waits under it; the menus under it are not drawn and take no input. The
same look in every menu style, on the 16:9 frame:

- **Card:** the kit's card over its scrim, 900 wide from x 510, centred, as tall as its
  words.
- **Words:** "Alpha build" in gold (Rajdhani 22), "Welcome to Sol JK" (Rajdhani 36),
  then Exo 2 18 (muted): an alpha of a free and open-source client, expect bugs and
  unfinished parts, some collectibles shown but not unlockable yet (a work in
  progress, no need to ask Sol); report on the SJK Discord or with Report a bug, this icon in the
  game menu.
- **Report row:** Report a bug as the game menu's row of icons draws it (the
  `other_controls` picture at 54, ringed gold while hovered), its name and "In the game
  menu (Escape)" beside it; a click opens the Report a bug dialog, and the card comes
  back when the dialog closes. On the right the invite
  (discord.gg/dGBdMygvJw, the permanent invite to #welcome) and Join the SJK Discord
  (outlined, opens it in the browser).
- **Terms:** a fun project of Sol and the contributors, no warranty (as the GPL says),
  nobody owes support, fixes or features.
- **Ticks:** three promises, gold until ticked, each line its own hit region:
  "I understand that SJK is a work in progress that may have bugs, that it comes with
  no warranty, and that I am owed nothing", "I won't DM Sol to ask how the game works:
  I will do my best to figure it out myself", and "I won't DM Sol bugs or ideas: I will
  report them with Report a bug or on the SJK Discord" (Sol's request, 16/10/2026).
- **Buttons:** Quit (outlined, 130) and Continue (gold, 200, greyed until all three are
  ticked). Keys bottom right: Space ticks the next promise, Enter continue; 1 to 3 flip
  a promise; Escape does nothing.

## Log request

[log_request_card.rs](../crates/sjk-viewer/src/log_request_card.rs): the pop-up a
player gets when the SJK team asks for their game's logs
([identity.md](identity.md#log-requests)); Sol asked on 11/10/2026 for a way to
collect a friend's logs through the hub. It shows on the main page or over the game
menu, never in the middle of play (the console says a request came in), after the
medal, holocron, reload and FPS cards and not while the window is away; the menus
under it are not drawn and take no input. The same look in every menu style, on the
16:9 frame:

- **Card:** the kit's card over its scrim, 800 wide from x 560, y 300, as tall as
  its words.
- **Words:** "SJK team" in gold (Rajdhani 22), "Send your game's logs?" (Rajdhani 36),
  then up to nine lines of Exo 2 18 (muted): who asks and their note in quotes, what
  the logs hold (console lines of this session and the one before without chat, the
  last crash report, build, system and graphics card), that the Windows user name is
  left out, and that nothing is sent unless the player chooses Send logs.
- **Buttons:** Don't send (outlined, 170) and Send logs (gold, 200), right-aligned along
  the foot; focus starts on Don't send. Keys bottom right: Enter send logs, Esc don't
  send; Space does nothing, so sending takes a deliberate key.

## Announcements and reward rounds

[official/window.rs](../crates/sjk-viewer/src/official/window.rs), 25/10/2026: the SJK
team's windows ([hub-chat.md](hub-chat.md#announcements-and-reward-rounds)). One SJK
pop-up card over the darkened screen at a time, the same look in every menu style, on the
16:9 frame; the menus under it are not drawn and take no input. A window the player opened
(a click on a line, `announcement`, `rewards`, `announce`) shows at once, in the middle of
play too; a command closes the console first, while a line clicked on the chat page opens
its window over the page, which is back once the window closes. While a window shows the
cursor is free and the window takes every key and click (held keys are let go); the claim
window opens on the main menu or over the game menu, after the medal,
holocron, reload, FPS and log request cards and before the News panel, never in the
middle of play. Every card is 920 wide from x 500, as tall as what it holds, centred; its
head is the mark (SJK's logo, or the gift box U+E028 in gold for rewards, 52), a kicker in
gold (Rajdhani 19) and on the right the date (`dd/mm/yyyy HH:MM`, UTC) or the round; then
the title (Rajdhani 32, two lines at most). A click outside the card closes the
announcement and the round list.

- **Announcement:** read on the [News panel](#news-panel) since 31/10/2026.
- **Round list:** "REWARD ROUND 1 OF 3", the round's title and note, then each person
  rewarded on a row of 34, alternate rows banded: the clan emblem for a known tag, the
  name with its colour codes ("Someone" when the hub has none) and the verified seal in a
  column of 290, the reason (muted, cut at 64 characters) beside it. Eleven rows at once,
  Up, Down and the wheel scroll; Older and Newer (Left, Right) change the round. No
  amounts.
- **Claim:** "CONTRIBUTOR REWARD" with the round's title, "You received", "From the SJK
  team, thank you.", then a row of 46 per item: each holocron tier's icon (its gem without
  one) and "2 Rare Holocrons" in the tier's colour, in the tiers' order; the credits' sign
  and "500 credits"; the kyber shard and "25 kyber shards"; then "Why: <reason>". Claim
  (gold, Enter; "Claiming..." while the hub answers) and Later (Esc). The hub's refusal
  shows over the buttons in ember.
- **Composer** (the creator's `announce`): Kind (News, Release), Version (a release's),
  Title and Text fields, Tab between them; the fields take Ctrl+A, Ctrl+C, Ctrl+V and a held
  Backspace ([text_entry.rs](../crates/sjk-viewer/src/text_entry.rs)); Enter in the text
  makes a new line (a paste keeps its lines), with a count of characters and lines under
  it. Under them the chat line as players will read it (SJK's logo, "SJK" in gold, the
  words). Send (or Ctrl+Enter) checks the hub's rules and asks to confirm; Send now
  (Enter) sends, Back (Esc) returns; the hub's words show if it refuses, and the window
  closes once it took it.

## Encrypted whisper

[e2e_whisper_card.rs](../crates/sjk-viewer/src/e2e_whisper_card.rs): the pop-up a
player gets when an [encrypted whisper](hub-chat.md#encrypted-whispers) could not go
encrypted (Sol's decision, 22/10/2026: ask, never send it unencrypted silently), and
the one that shows a safety code. It opens the game menu when play is under way (the
composer closes) and shows over it, after the other cards; closing the last one closes
the menu it opened. What waits shows oldest first, and what belongs to a server the
player has left is dropped. The same look in every menu style, on the 16:9 frame:

- **Card:** the kit's card over its scrim, 800 wide from x 560, y 330, as tall as its
  words.
- **Words:** "Encrypted whisper" in gold (Rajdhani 22), "Send it unencrypted?"
  (Rajdhani 36), then up to seven lines of Exo 2 18 (muted): to whom, why the hub did
  not take it, the whisper in quotes (its first 120 characters), and that the game
  server and whoever runs it can read it if sent unencrypted.
- **Buttons:** Don't send (outlined, 170) and Send unencrypted (gold, 250),
  right-aligned along the foot; focus starts on Don't send. Keys bottom right: Enter
  send unencrypted, Esc don't send; Space does nothing.
- **Safety code:** "Encrypted whispers", then "Safety code with <name>" (or "<name>'s
  key changed"), the code's twelve groups of five digits in three rows of four (Rajdhani
  40, rows 46 apart; gold once verified), and the words: compare it
  outside the game, then mark them verified; once verified, that SJK warns if their key
  changes; for a changed key, that whispers to them wait for a new check. Buttons Close
  and Mark verified (gold, Enter); once verified, Close is gold (Enter and Esc) and
  Remove the check is outlined, by click only. The chat's player card offers it with a
  full-width Safety code button in the whispers' pink, under Block.

## Trading

[trade_window.rs](../crates/sjk-viewer/src/trade_window.rs) and
[player_trades.rs](../crates/sjk-viewer/src/player_trades.rs) (prepared 22/10/2026, off
until the hub turns trading on): two SJK players swap unopened holocrons and duplicate
saber shaders through the SJK hub, which checks both sides twice and holds every limit
(the hub's PROTOCOL.md, "Trades between players"). Every number the window shows comes
from the hub: its rules, today's counts and what each item is worth. The same look in
every menu style, on the 16:9 frame:

- **Starting:** a gold Trade button (140 wide) on the game menu's Players page, on the
  player card of the player chosen in the table, with what it does beside it on one line
  ("Swap holocrons and duplicate shaders", or "Swap duplicate shaders" while holocrons do
  not trade; a test measures every such line in the UI's fonts), dimmed with why when it
  cannot be (trading closed, an offline server), Trade on a player's card in the game's chat and the SJK chat page (beside
  Block, the two even halves; Ban 24 h then takes a row of its own; only while the hub
  lets the player trade), Trade on the game menu's Players page after the report reasons (with
  why not when it cannot be: yourself, a bot, a player the hub does not know, trading
  closed), or `trade <player>`.
- **The toast:** while a trade runs and the window is closed, a card 660 wide at the top
  centre (y 64) over play, which takes no input: the other player's picture (or initial)
  in a gold ring, their name in its colours, "Nib wants to trade" (Rajdhani 24) with the request's seconds in gold, a muted
  line, a thin gold bar running down smoothly (to the fraction of a second, not in steps),
  and the key: Esc answer (or Esc open), 40 above the card's foot, clear of the bar. It also tells
  for six seconds how a trade ended, or why a request was refused. A finished trade says
  "Your new items are in your collection" (or "item is", for one) only when the other
  side gave something; one they put nothing into says "You gave your offer to <name> and
  received nothing" (hub report #208, `PlayerTrade::ending`), the name with its clan marks
  (`« » •`, as the window's headline shows it). The toast keeps its words on one line and
  never cuts them: it is 660 wide, and wider, still centred, when the headline (measured in
  the display font, with room for the clock, key hint or Open trade beside it) or the line
  needs it, up to 1800. What the window and
  the toast already said goes on to the next map's world with the shell: a world that
  began without it took the hub's last refusal for a new one and showed it again at every
  map change (Creyon's screenshot, 26/10/2026: "Two accounts on one connection cannot
  trade with each other." over a loading screen). The game menu does not go on, so the
  window no longer counts as having opened it; after a map change a running trade's
  window comes back with Esc, and its Close leaves the menu the player opened.
- **Reaching it in game:** the pointer is free only in the menus, so the window shows
  over the game menu: Esc in play opens it while a trade runs; Esc in it shows the game
  menu under it (the trade goes on, the toast at the top says so) and Esc there goes back
  to play. Whenever the pointer is free (the game menu, or the chat's composer, open) the
  toast is a button: Open trade (gold) and a click anywhere on it bring the window back.
  Asking from a card or the command opens it at once. A request from another player
  never opens it by itself, in game or in the main menu: only the toast shows until its
  Open trade, or Esc in play (Sol's review, 24/10/2026: it took every key over the menus,
  and Enter accepted). Esc in play means the game menu opening from play, so a request
  that came during play stays a toast when the player leaves for the main menu (Sol's
  review, 25/10/2026). The other cards come first.
- **From the main menu:** no game is needed: the toast and the window show over the main
  menu too (the pointer is free there, so the toast is always a button), and Close keeps
  the window closed until the toast's Open trade.
- **Chatting while trading** (hub report #215): the window keeps the keyboard (Enter
  for its gold button, Esc to close it) but lets the chat keys through: a key bound to
  `messagemode` to `messagemode6` (Chat, Team chat, the whispers, SJK chat, Admin chat)
  (and `messagemode7`, `messagemode8`, the clan chats, through the same composer)
  opens the composer, the window staying open under it. In a game the chat draws after
  the window, over its shade, its recent lines too, so the other player's answers show;
  the composer has the keys until it sends (Enter) or closes (Esc), and the window's
  buttons still take clicks. Over the main menu the SJK chat key opens the SJK chat page,
  and the window comes back when the page closes.
- **Blocked players** (Sol's decision, 24/10/2026): a request from a player blocked on
  this PC is declined without a word (no toast, no window), and no card offers Trade
  with them (the Players page says "You blocked this player"); `trade <player>` and a
  request by key refuse them too ("you blocked this player", Sol's review, 25/10/2026).
- **The SJK chat page:** its sender card offers Trade too (by the player's SJK key); the
  page closes and the window opens over the game or the main menu.
- **Request:** the kit's card, 800 wide from x 560, y 300, over a light shade (the match
  shows round it): the SJK logo, "Trade request" in gold, "Nib wants to trade"
  (Rajdhani 36, their name in its colours) with the seconds left, what a trade is, the bar, Decline (outlined) and Accept (gold, Enter).
  The asker's reads "Waiting for Nib" with Cancel request. A request waiting for a player
  does not hold them at the hub (its review of 24/10/2026): they may be asked by others
  and ask someone themselves, and taking one trade closes the rest, which say so ("Nib is
  trading with someone else."). Asking a player again soon after they said no, or sending
  many requests, gets the hub's answer to wait (`asked_recently`, `asking_too_often`), in
  words that name no number.
- **Open trade:** a card 1160 by 780 (its design size; the player's Trade window size
  setting, `cl_tradeWindowSize`, draws it at 80, 90 (the default) or 100 percent about the
  frame's middle). Its head: the SJK logo on its own (64 wide), "Trading with" in gold, and
  the other player's picture (or initial) and name as they wear it, colours and all; when
  it closes without a change on the right. Two columns, You give and Nib gives (their name
  in its colours), each with what it is worth on average in kyber shards, six tile slots
  (78 square: the holocron's picture and tier, or the blade in its tier's colour and its
  name, ringed in that colour) and its state (Choosing..., Ready, Confirmed), a gold swap
  badge between them. Under a hairline, "Your holocrons and duplicates" ("Your duplicates"
  while the hub trades no holocrons yet, `rules.holocrons` false: duplicates come first, and
  the toast then says "Duplicate shaders."): up to 18 tiles,
  those that may go in first, the favourites first among them, most worth first (a click
  puts one in; a click on an offered one takes it out). Since 29/10/2026 a shader's tile
  carries its marks ([Marks](#marks)), drawn from shapes and plain words as the window's
  font draws bytes: a star and a padlock at its top left, "Worn" at its top right, a New
  sticker over its top edge; a locked shader is dimmed with the ones that wait and takes
  no click, a gold line under the items saying "Locked shaders stay out of trades: unlock
  them in the Collection.", and a Favourites chip at the heading's right (on a hub that
  keeps marks) shows the favourites alone. A holocron received in a trade within the hub's resale lock is shown
  dimmed with a gold timer pill in place of its name, counting down ("6d 23h", "5h 12m"),
  and a gold warning card under the items says "Holocrons received in a trade wait before
  they can be traded again: each shows how long."; one the day's limits hold back is
  dimmed too, a gold line saying why. Neither answers a click; the gold line names the
  tiers given ("You have given today's Rare and Mythical holocrons. More tomorrow."). No
  text names one of the hub's numbers (a limit, the lock, the idle time, the trust rule):
  the hub tunes them without a client update (Sol's review). Beside them, Daily
  limits: Holocrons (no row while holocrons do not trade) and Duplicates, each Received and
  Given as done / most (ember once reached); one of each tier a day each way (the hub's
  `holocron_tier_day`) shows only in that gold line. The
  limits run for a trading day of 24 hours from the player's first completed trade (the
  same for every time zone), so under them, while one runs, "Daily reset in" with a gold
  clock ticking down ("14:07:33", the hub's `resets_at`); then every count resets. Then
  the hub's refusal in ember, "Nib is giving this as a gift: nothing is asked in return."
  in green, or an uneven warning in gold, and Cancel trade, Not ready and the gold button:
  Ready, then Confirm in 3, 2, 1 and Confirm trade once both are ready. Keys bottom right:
  Enter main button, Esc close.
- **A gift:** one side may give nothing. Ready on a gift the player gives asks once more:
  a gold-ringed card in the items' place, "Give it as a gift?", what they give, that
  nothing comes back and that it cannot be undone, with Go back and Yes, give it (gold).
  A change of the offers drops the question.
- **Recent holocrons:** on the Profile screen's Holocrons tab, a holocron received in a
  trade says so under its name: "Received from Nib in a trade", and while the hub's lock
  runs "- tradable again in 6d 23h" (no countdown before the hub said how long).
- **Ended:** "Traded with Nib" (or ended) and how, with Close, while the window was open.
  The swap happens at the second confirmation, so only that player's answer from the hub
  carries their new profile. The first to confirm learns of it from the trade list (the
  feed's word, or the 15-second read): the list that first shows a trade done reads the
  own profile and the holocron progress at once, so the holocrons and duplicates given
  leave the collection, and those received arrive, without waiting for the ten-minute
  profile read (Sol, in game, 25/10/2026).
- **Cost:** the views are built only when what they show changes (a new list or step from
  the hub, the own profile, the players known) and the window's parts (the player's items,
  the limits) only while the window can show, under one short identity lock; between, only
  their clocks run on, and the window keeps its copy. With trading off for the player, the
  client reads the trade list only every 30 minutes, not every ten (Sol's review,
  24/10/2026), so trading switched on at the hub reaches running clients without a
  restart (Sol's review, 25/10/2026); registering again or the feed's word of a trade
  reads it at once.

## News panel

[news_panel.rs](../crates/sjk-viewer/src/news_panel.rs) with
[news_panel_frame.rs](../crates/sjk-viewer/src/news_panel_frame.rs), 31/10/2026 (Sol:
the what's-new card and the announcements "are kinda the same", so one panel for both,
with working switches, pictures, and links into the UI that only the SJK team writes).
It replaced the what's-new card (12/10/2026) and the announcement card (25/10/2026). One
SJK pop-up over the darkened screen, the same look in every menu style, on the 16:9
frame; the menus under it are not drawn and take no input, and while it shows the cursor
is free and it takes every key and click.

- **When it opens on its own:** on the main menu or over the game menu, never in the
  middle of play, after the medal, holocron, reload, FPS, log-request and claim cards and
  not while the window is away. After an update, on its "Settings to review" (when the
  releases since the one seen name any) or its recap: `cl_whatsNewSeen` holds the
  version whose panel the player closed, as the card's did; a new installation (no
  marker, First setup not yet ticked away) writes the marker and shows nothing, and a
  local build no release lists shows nothing. And when an announcement came that the
  player has not read, once a run for each: `announcements_seen.txt` keeps `read`, the
  highest announcement number seen on the panel ([official/seen.rs](../crates/sjk-viewer/src/official/seen.rs));
  a file from before counts what the chat showed as read, and a new key counts what the
  hub lists first as read, so a fresh install opens nothing. Closing the panel counts
  every announcement it listed as read and writes the update's marker.
- **Asked for**, it shows at once, over play and over the console: `news` (its newest
  entry), `whatsnew` (the newest recap, "Unreleased" included, as a preview that writes
  nothing; its settings to review switch), `announcement [n]` and a click on an
  announcement's line in the SJK chat ("Read more").
- **Settings to review** (17/10/2026): `= <cvar>[ reset]: <sentence>` in the changelog
  ([sjk.md](sjk.md#changelog)), every release since the seen one. A `reset` setting is put
  back to its default when the update's panel is offered, once per version
  (`cl_whatsNewReset`, written as soon as it is applied).
- **Left, the entries,** ten at a time (the wheel scrolls; Up and Down change the entry):
  "Settings to review" first, then the hub's announcements and the six newest releases'
  recaps, newest first by date. Each row: its kicker ("SETTINGS TO REVIEW" in gold, "SJK
  RELEASE", "FROM THE SJK TEAM", "PREVIEW"; Rajdhani 14), its date on the right, its name
  (Rajdhani 21), a gold dot while unread. A release's announcement (kind `release`, its
  version a release the changelog lists) goes on that release's entry, its text above the
  recap; one for a release this build does not list is an entry of its own with Update.
- **Right, the entry:** its kicker in gold and its date (dd/mm/yyyy, with HH:MM for an
  announcement), its title (Rajdhani 34), then its text 1124 wide, by lines that scroll
  (the wheel, Page Up and Down; a gold thumb on the right shows where). Recap lines are
  a heading with its icon (`HEADING_ICONS`) over its sentence; a release without a recap
  shows its introduction (six lines, "...") or says its changes are in the full
  changelog; "By ..., after ..." closes a release in gold. Announcement lines are
  paragraphs, `- ` bullets (a gold dot, wrapped lines indented) and three kinds of
  markup ([sjk_identity::news_markup](../crates/sjk-identity/src/news_markup.rs)):
  - **Links**, `[Label](target)`, drawn semibold in the holo colour and underlined, gold
    under the pointer. `setting:<cvar>` opens Settings searched for it (the SJK UI's,
    or the classic Setup panel); `open:<page>` opens a place of
    [ui_links.rs](../crates/sjk-viewer/src/ui_links.rs)'s `PAGES` table (Settings,
    Collection, Achievements, Holocrons, Profile, SJK identity, SJK chat, Emotes, Toys,
    Model viewer, Contributor rewards, Changelog, Credits, Update, and with `mod-jof` JoF
    content and Sith cloud), each by a command of this client. A link names a place,
    never a command, and one this build does not know reads as its label.
  - **Pictures**, `[[image:<id>]]` on a line of their own: drawn at their size within the
    pane (at most 470 high), a framed "Picture loading..." until they arrive
    ([news_images.rs](../crates/sjk-viewer/src/news_images.rs): a worker reads them from
    `news/<id>` beside the settings, else downloads them from the hub and keeps them
    there; three textures of their own, given back as 1x1 after 600 frames unused, and
    at most four kept decoded). A picture the hub took down is left out.
  - **Switches**, `[[switch:<cvar>]]` on a line of their own: the setting's Settings
    name in gold and its switch, as a setting to review (a cvar Settings has no plain
    on/off row for is left out).
  Commands and cvars in backticks are chips (`inline_code`), as in the changelog.
- **Switches** read the console each frame as the Settings rows do (a number cvar is on
  when nonzero; Sol's report, 31/10/2026: one held in a number cvar did nothing and always
  showed Off). A click or the row's number key (1 to 9) flips it and the panel stays.
- **Foot:** Full changelog (outlined), Update (gold, on a release newer than this build:
  install a release waiting, else the Update page) and Close (gold otherwise); keys
  bottom right: 1-9 switch, Up and Down entry, PgUp and PgDn scroll, U update, C full
  changelog, Esc (also Enter, Space, Backspace) close. A chip or a link closes the panel
  and follows it.

## Symbols

[symbols.rs](../crates/sjk-viewer/src/symbols.rs), the font
[SJKSymbols.ttf](../crates/sjk-viewer/assets/fonts/SJKSymbols.ttf) and its builder
[scripts/sjk_symbols.py](../scripts/sjk_symbols.py), 13/10/2026, Sol's request: SJK's own
symbol font, to mark qualities in coloured text and to grow. Sol first asked about
kanji; they read as cyberpunk rather than Star Wars, and Japanese players read them as
words. Aurebesh belongs to Lucasfilm, so SJK draws its own symbols. Sol picked the
holocron family out of three drafts.

Each symbol is a character in Unicode's Private Use Area, so any text can hold one and
draws it in its own colour; the client draws it from SJK Symbols through the wide-glyph
atlas, after each face's own fonts ([rendering.md](rendering.md#characters-beyond-the-code-page)).
A symbol keeps its code point for good; a new one takes the next free point of its
group in the script (`SYMBOLS`) and gets its name in `symbols.rs`.

| Code point | Symbol | Meaning |
| --- | --- | --- |
| U+E000 | empty octagon | Common |
| U+E001 | octagon round a diamond core | Uncommon |
| U+E002 | octagon with one upright bar | Rare |
| U+E003 | octagon with a cross round a core | Legendary |
| U+E004 | the Legendary mark in a second, outer octagon | Mythical |
| U+E005 | Mythical's two octagons round a solid four-point star | Unique (25/10/2026) |
| U+E010 | a card's outline | Normal finish |
| U+E011 | a card with two slanting bands | Holo finish |
| U+E012 | a card with a sun at its heart | Solar finish |
| U+E020 | a cube seen from above a corner | a holocron: found in one |
| U+E021 | a medallion on a ribbon's V | a medal: comes with one |
| U+E022 | a padlock | locked: since 29/10/2026 the player's own lock on a saber shader ([Marks](#marks)); a part an extra card unlocks |
| U+E023 | a tick | yours |
| U+E024 | a card with a second behind it | held more than once |
| U+E025 | a smith's hammer | its saber smith |
| U+E026 | a turning arrow | drag to turn |
| U+E027 | a lightsaber held up | worn |
| U+E028 | a gift box with a bow | from the SJK team (also a reward round's chat line and the claim window, 25/10/2026) |
| U+E029 | a five-pointed star | its edition; a favourite (29/10/2026, [Marks](#marks)) |
| U+E030 | a long crystal, a ridge down its middle | Kyber shards (17/10/2026) |

- **Drawn from strokes:** the script draws every symbol from lines with round ends,
  polygon rings and diamonds in font units (1000 to the em, capitals 700 high, the
  symbol 780 high on their middle), turns the strokes into outlines and merges them
  (fontTools and skia-pathops), so the font is clean TrueType built the same way every
  time; `python scripts/sjk_symbols.py --check` says when the committed font is out of
  date, and a test checks every symbol `symbols.rs` names is in it. The strokes are
  about two pixels thick in 16 to 20 pixel text.
- **Where they show:** a card's corner and its footer's rarity (the face), the sealed
  back's rarity, the loot table's rarity and finish headings, its kyber shards' heading and its cards' rows, the
  Shaders tab's tier on each row. The rarity's word and colour stay beside the symbol:
  it is never the only sign.
- **Not in it:** the credits' sign, Lucasfilm's, stays drawn from shapes
  ([cards.md](cards.md#in-the-client)).

## Implementation

- `ui_menuStyle sjk` (`menu::style::MenuStyle::Sjk`) is the default beside
  `classic`. Every screen without an SJK UI version opens its classic one; the
  main page dispatches to `menu::sjk::home`.
  `ClientMenu::sjk_screen` says when one of the SJK UI's own screens is on show
  (the main page, Settings without a picker open, Character, Servers, the
  loading screen, and Create game in every menu style): the map is drawn under it (the loading screen leaves it out
  itself when it has to)
  (`classic_hides_world` leaves it out) and its text goes to the UI's families
  (`append_sjk_screen`).
- The controls are drawn by `menu::sjk::kit` (the band, switch, slider,
  segments, field, list, reset arrow, changed dot, sub-heading, search pill,
  lit rail, cycler, colour chips, button and rank pips), in frame pixels
  (`menu::sjk::Frame`).
- The player screen's style: `PlayerMenu::set_sjk` (from `set_menu_art`) draws
  the screen in the SJK UI's view; `ClientMenu::sjk_screen` covers the
  player phase then, so the map and the stage model show under it.
- Settings is the classic+ panel's state with another view: the menu opens a
  category's rows as the classic+ panels do, keeps `classic_panel` empty and
  marks the SJK UI's screen open (`sjk::settings::SettingsPage`); the rows'
  results (`SettingsResult::Classic` for a category, `ClassicCycle` for Tab) are
  read as the rail's. Pointer tokens are the classic+ panel's (rows, values,
  segments, reset, list, search, scroll), the rail's categories its chrome
  tokens.
- Servers is the browser's state with another view: `ServerBrowser` and its
  pointer tokens (rows, headers, the tabs for the sources, the filter strip's
  for Show, the prompts') are the other styles'. Its keys go first to
  `ClientMenu::sjk_browser_key`, which handles the search, the left column
  (`sjk::browser::BrowserPage`) and Left, and leaves the rest to the shared
  `browser_key`; its pointer to `sjk_browser_pointer` before the shared one.
  The chosen server's levelshot comes through the create-game screen's cache
  (`CreateGame::service_levelshot_for`, which also gives its size).
- Create game is `menu::create_game::CreateGameMenu`'s state with a new view
  (`CreateGameMenu::append`, its text to a `TextTarget`, from `append_sjk_screen`
  or, without the families, `append_overlay`); its pointer tokens are the rows'
  indices, Start's, the way back's (`create_game::BACK_TOKEN`, 900) and the list's
  slots, and a cycler row registers its control before the row so a click steps the
  way the half under the pointer points. Because Create game has no classic+ version,
  `sjk_screen` covers it in the classic style too, and `game_font::prepare` loads the
  families whenever an `sjk_screen` is on show. `world_shot::create_game` renders it.
- The loading screen is the classic loading screen's state with another view:
  `classic::loading::ClassicLoading` (fed by the join, `set_game` and
  `sync_classic_loading`) also keeps the session's world stage and whether the
  session is in hand (`set_world`), a count of joins (`generation`), whether
  the load named its map (`named_map`) and the gamestate's facts in words
  (`sjk::loading::Facts`). The screen's own memory
  (`sjk::loading::LoadingPage`) is the furthest step of the join and the
  levelshot's fade. Its levelshot comes through the same cache, serviced by
  `upload_menu_images` as for the classic screen.
- The scoreboard is the scoreboard module's state with another look:
  `style::ScoreboardStyle::Sjk` (resolved from `cg_scoreboardStyle` and
  `ui_menuStyle`) builds its rows with `scoreboard::sjk::build` on the board's
  own canvas, through the HUD's path (`scoreboard::append_overlay`), not the
  menu's; the text goes to the UI's families (`append_text_families`), or Inter
  until they load. `game_font::prepare` loads the families when the scoreboard
  wants them too. The ping bars are the browser's (`browser::signal`).
- The in-game menu keeps the in-game menu's pages, rows and actions
  (`ingame_menu::InGameMenu`, `Page`): `InGameMenu::is_sjk` picks the SJK UI's
  look (`is_classic` no longer covers it), `sjk_view::prepare` writes the rows
  it words its own way (Main, Team, Leave) with their hints, and
  the shared rows (Siege, the call-vote lists) keep theirs, a "label  /
  detail" row splitting into label and hint. `GpuState::activate_sjk_ui_row`
  acts on its own rows before the shared actions; `back_or_close_game_menu`
  goes to `sjk_view::parent` (the entry that opened the page). The keys pass
  over rows that cannot be taken (`InGameMenu::sjk_step`); a screen handed
  over to returns on its entry (`InGameMenu::return_row`, read by
  `MenuAction::ReturnToGameMenu`). The main page's row of icons and card controls
  are `sjk_focus`'s model (`Icon`, `Control`, `Controls::for_match`, `Focus`,
  `step`), built each frame from the view (`View::staff` from the profile card's
  summary) and kept on `InGameMenu` for the keys (`GpuState::sjk_main_key`) and the
  pointer (`sjk_main_pointer`, tokens 800 for the icons and 820 for the controls).
  The docked SJK chat is `menu::sjk::chat_dock`, which the main page uses too: its
  `Dock` (draft, sender card, the messages shown) and `DockCache` live on
  `InGameMenu`, `InGameMenu::sync_chat` follows `cl_sjkChat` each frame, and its
  tokens are `sjk_view::CHAT_TOKENS` (940 to 948). The
  match card (`sjk_view::Card`) is refreshed from the session by
  `GpuState::refresh_game_menu_card`. Settings
  opened from it is `ClientMenu::open_sjk_settings_from_game`; Settings' and
  Key bindings' top bar take their way back from `settings::Rail::back`. The 2D
  pass now also runs while the game menu is open without a session
  (`frame_overlays.rs`), which an explored map's game menu needed too.
- Fonts: `text::load_family` rasterizes a family's two faces into one atlas, as
  Inter's, taking glyphs a face lacks from its fallback family. The SJK UI's
  families load at the first frame in every menu style (since 10/10/2026: SJK's
  own pages have only this look), rasterized at 1.5x (glyphs 144
  pixels tall, sharp at 4K), into GPU layers kept by `game_font::GameFonts`.
  `MenuCanvas::set_family` tags each text run with its family
  (`menu_widgets::TextFamily`), and `append_text_families` routes the runs to
  their family's layer. Until the families are loaded the page draws in Inter.
- Snapshots: `menu_snapshot::sjk_home_snapshot` and `sjk_settings_snapshot` draw
  the screens over the JoF HD wide levelshot of mp/duel6, each family from its
  own atlas. `world_shot::tests::duel6_sjk_menu` renders the real frames: the
  client built without a window on duel6, the SJK UI over the touring camera;
  `duel6_sjk_browser` the browser on made-up servers (one answered status),
  a search and the password prompt; `duel6_sjk_loading` the loading screen on
  a made-up join of the JoF server (before the map is known, loading mp/ffa3,
  failed with and without the map known, a server's change of map on the navy
  ground);
  `duel6_sjk_scoreboard` the scoreboard on
  made-up matches (`scoreboard::shot`: free for all with 14 and with 30
  players, the 30 again without compact rows, capture the flag, a duel, a
  power duel, and a 4:3 window), drawn
  without a server;
  `duel6_sjk_ingame` the in-game menu over
  duel6 on a made-up match (`Card::for_shot`, `InGameMenu::sjk_for_shot`):
  the main page without a vote and with one, the keyboard in the row of icons, the
  side's buttons in a CTF, Staff tools shown, a spectator in an FFA, a Siege, Team
  in a CTF, the installed maps to vote for, Leave, a 4:3 window and Settings opened
  from it; `duel6_sjk_chat_dock` the docked chat on made-up messages, on the main page
  (and typing) and in the in-game menu (a sender's card, typing with a vote on), at
  1080p, 4:3 and 4K with the plain text style and `ui_textScale 1.2`; `duel6_sjk_profile_screen` the Profile screen opened from the game menu
  on its Character tab, Ctrl+Tab through the four tabs and round, the game menu's
  Profile opening again on the tab left last, the picture panel with Browse..., a 4:3
  window, and from the main page its Character tab on the menu map's stage, each
  canvas checked not to have run out of room; `duel6_sjk_collection` (in
  `world_shot_collection.rs`) the Collection;
  `duel6_camera_control` Camera control over duel6 on a made-up match: the
  in-game menu with its camera icon focused, the panel's Camera and Sun pages, a number
  typed, the Sun page where the sun cannot be set, a 4:3 window and the classic
  menus' bar and panel;
  `duel6_quick_wheel` the quick wheel's ring over duel6 (General, the change to
  Toys half-way, Toys with the holocron lit, Weather, the middle, 4:3, in Inter,
  and three full pages of the second board's icons, `duel6-wheel-icons-1` to `-3`)
  and `duel6_quick_wheel_settings` its Settings category (the Force page first, the
  Toys page, the sound switch off, a choice, the catalogue, a custom choice, a new
  page) and the editor over the classic+ settings; `duel6_quick_wheel_force` the Force page
  with the powers set for the shot (`duel6-wheel-force-light`, `-dark`, `-every`
  and `-every-2` for Force 2, `-none`, `-no-game`, and the light build at 4K with
  `ui_textScale 1.2`);
  `duel6_sjk_credits` Credits over duel6: the top, Creyon's and Lumaya's
  panels with their folds open and their medals, Creyon's work unfolded, and
  the end of the page.
- The report card is the text dialog's state with another look
  (`text_dialog::Look::Sjk`, set from `ui_menuStyle` by `sync_menu_style`): its
  text, focus, pointer tokens and the hub's rules are the other looks'. A
  report's card holds a `Phase` (writing, sending, sent, failed); the senders
  (`send_bug_report`, `send_player_report`) and their pollers hand it the
  identity service's outcome with `TextDialog::answer`, which shows it when
  the card waits for it and else leaves the centre print to them.
  `GpuState::append_text_dialog` clears every text batch before it (the Inter
  and classic HUD vertices, `GameFonts::clear_text`) and routes the card's text
  to the families; the client menu's draw list is left out under it, as under
  a new medal. Two fixes came with it: the 2D pass now runs while the dialog is
  open without a session or menu (a note written on a map explored alone was
  not drawn), and each layer of the 2D pass starts alpha blended
  (`ui_renderer::art::begin_layer`): a layer ending on the emblem's light, as
  the main page does, drew the next layer's shapes additively. The pointer
  reaches the dialog before the menus, as the keys did. `duel6_sjk_report`
  renders it: the empty report, a long text, a refusal, sending, sent, not
  sent (the identity off, through the client's own path), a player report, a
  note, a 4:3 window and over the main page.
- The new medal pop-up is the medal pop-up's state, queue, keys and pointer with a
  look per menu style (`MedalPopup::set_style`, from `sync_menu_style`):
  `GpuState::append_medal_popup` clears every text batch before it, as for the report
  card, and routes the SJK UI's text to the families (Inter until they load), the
  classic+ look's to the menus' font. `menu_snapshot::medals_snapshot` draws it over
  duel6's levelshot at moments of its ceremony held still, and
  `world_shot::tests::duel6_medal_popup` over the live main page in both looks.
- The quick wheel's ring ([ring.rs](../crates/sjk-viewer/src/quick_wheel/ring.rs))
  is drawn on the HUD's layer with a canvas of its own, its text routed to the
  families when they are loaded (`quick_wheel::append`), else to Inter; it does
  not load them itself.

## Chat media sources

SJK chat's GIF picker includes a Sources tab in every menu style. It edits the
profile's exact-host trust list, keeps GIPHY by default, and requires an explicit
Trust source action after typing or selecting a host. A selected message's Manage
GIF source action opens the same editor. Discovered media CDN hosts are listed for
separate approval; removing a source stops cached playback. See
[trusted media sources](hub-chat.md#trusted-media-sources) for formats and privacy.
The field uses the shared text-entry handler, including selection, clipboard and
held-key editing.

## Profile banners

Collection > Banners shows six static covers (the neutral cover and five collectible
ones), copy counts and a profile preview, with keyboard/pointer equip and unequip.
The Profile header, own profile card and chat sender cards share the same native art,
with central cropping and a readability veil. These SJK controls work in every menu
style. See [profile banners](profile-banners.md) for rewards and compatibility.

## Chat reactions

The SJK chat page shows emoji reaction counts beside a message's time. Selecting
a public message exposes six reaction buttons under its sender in Chosen message.
The player's own reactions are highlighted; a second activation removes one. These
SJK controls are used in every menu style.

Shared chat reactions (offline fixture; heart selected by the reader):

![SJK chat reactions](images/sjk-chat-reactions.png)

## Incoming chat colour

Settings > Chat includes **Incoming chat colour**, a named choice shared with
the classic settings. Original preserves authored colours; the other ten choices
force the game server's public All message bodies while keeping sender names and non-message controls distinct.
Team/admin chat, whispers and the separate SJK channel keep their own colours.

Incoming chat colour, cyan selected (offline fixture):

![Incoming chat colour](images/incoming-chat-colour.png)

## Away chat bubble

Settings > Chat in Advanced mode offers **Chat bubble while away**, shared with
the classic HUD settings. `cl_unfocusedChatbox` controls both unfocused and
minimized windows; there is no separate minimized switch. The default remains
on, saved unfocused choices are kept, and the update's what's-new card offers
the same switch. Actual chat, console and menu key catchers retain their normal
bubble. See [talk balloons](client.md#talk-balloons).

![One away bubble switch in SJK Settings](images/away-chat-settings.png)
## Game chat logs

Settings > Chat in Advanced mode includes **Save game chat logs**, an on/off row
shared with the classic HUD settings. It is off by default, as in stock clients,
and keeps saved choices. Its help names the profile's `chatlogs/chat.log` and explains that
received game-server public, team and private chat is saved before display filters.
The release's what's-new card offers the same switch; console value 2 retains
synchronized writes. See [chat logs](client.md#chat-logs).

![Save game chat logs in SJK Settings](images/chat-logs-settings.png)


## Plan

The next screens, in order; each gets snapshot tests before it replaces its
classic version:

1. The dialogs (the import page). The report box
   ([Report a bug](#report-a-bug-and-its-dialogs)) and Credits are done
   (08/10/2026).
2. Settings' search finding key bindings too (it finds settings; Key bindings'
   finds keys).
3. Create a game: done (10/10/2026, in every menu style).

`sjk` became the default `ui_menuStyle` before these were done. mp/duel6 has
its tour, player stage and saber shot.

The Servers detail panel offers **Copy IP** beside Join and Favourites. It copies
the selected endpoint, including its port, and reports clipboard success or failure
in the status line. Ctrl+C does the same outside text fields and dialogs.
