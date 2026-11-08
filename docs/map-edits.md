# Map edits

SJK's own fixes to maps, drawn by SJK clients only (Sol's request, 18/10/2026): staff hide
a map's props and surfaces, place models and swap shaders in game, publish them through
the hub, and every SJK player sees them while `cg_mapEdits` is on (Settings > GAME > "SJK
map edits", on by default). Code: [map_edits](../crates/sjk-viewer/src/map_edits/mod.rs),
the hub copies in [sjk-identity's map_edits](../crates/sjk-identity/src/map_edits.rs).

## What an edit does

| Edit | Drawn | Not changed |
| --- | --- | --- |
| `hide_prop` | the map's `misc_model_static` prop no longer draws | its lamps, if it has any ([static lamps](rendering.md)) |
| `hide_surface` | a BSP surface draws nothing, and picks and impact marks skip it | the bake around it |
| `place` | an MD3 from the game data at a position, turned and scaled | lighting: entity light only, as props get |
| `remap` | a shader drawn instead of another wherever the map draws it | |
| `show_face` | one caulk, nodraw or sky side of a world brush, drawn with a shader | collision, the bake |
| `weather` | the map's rain, snow, fog and wind, in place of its own ([below](#weather)) | gameplay: weather is drawn only |
| `light` | a point light staff added, drawn as a dynamic light on every preset ([below](#lights)) | the baked lightmaps: nothing is baked again |
| `lamp` | one of the map's own real-time lamps (a fixture) turned off or moved, on Experimental only, from the map's next load ([below](#lights)) | the baked lightmaps, the lamp's surface (still drawn, still glowing) |

No edit changes a map's baked light: the lightmaps, the light grid and the vertex light
are the BSP's own, so a hidden lamp's glow stays baked on the walls round it (as a hidden
prop's), and an added light only adds to them.

Edits change what is drawn, never collision. The server keeps every brush and entity: a
hidden prop or surface with clip brushes is an invisible wall, and a placed model can be
walked and shot through. Server entities (breakables, doors, movers) are refused by
the hide tool: the game needs them.

Many maps keep no props of their own: the map compiler merges `misc_model`s into the
world, and only `misc_model_static` stays an entity (retail: none on ffa1-ffa4, 51 on
duel6, 84 on ffa5, 189 on siege_korriban). A merged model is hidden as surfaces:
`mapedit_hide group` takes the triangle-soup surfaces touching the one aimed at,
through shared corners (at most 256), which is usually the whole model.

Caulk and nodraw sides have no drawn surface at all: q3map2 makes none for them, so a
remap cannot reach them (`r_showBrushes` rebuilds them from the planes for the same
reason). A `show_face` edit names one side of a world brush by its BSP numbers (with
its shader, checked on load) and draws it with the shader chosen, rebuilt from its
brush's planes, textured as Radiant textures a new brush (the axial projection at scale
0.5, no shift or turn, the image's own size read from its header) and lit like a placed
model from the face's middle ([faces.rs](../crates/sjk-viewer/src/map_edits/faces.rs)).
Only sides of solid brushes are offered (caulk, nodraw, an invisible wall), never clip,
trigger or water. Each face is one small mesh, built once.

**The sky** (04/11/2026, Sol: "remap skybox faces" and "replace a skybox by another
skybox"). A click or aim on the sky selects the sky side of the brush behind it (the sky
surface the ray meets first, and the brush side it lies on, within 2 units: `world_notes`
`pick`), named as a sky with its shader. Two edits act on it, both kinds every client
already reads:

- **Cover a side**: a texture drawn on it is a `show_face` edit, as for caulk (rebuilt
  from the planes, Radiant's projection, entity light). It covers the sky: every program
  that draws on the sky's faces pushes their depth back by 1/32 of the way to the far
  plane ([rendering.md](rendering.md#sky-faces-depth)), so the face on the same plane
  always wins the depth test, with no z-fighting; a player only ever sees a sky side from
  its front, so nothing shows behind. A sky shader is refused for a face.
- **Swap the sky**: `mapedit_sky <sky shader>` (or `mapedit_remap` with a sky shader on a
  sky) is a `remap` edit from the map's sky shader to the new one, refused unless the
  target has `skyParms`. The sky pass follows remaps (`sky_remaps.rs`): the new box and its
  cloud stages are loaded, as a server's sky remap is. `mapedit_sky <sky> all` swaps every
  sky shader of the map in one step to undo; `-` puts the map's back (`mapedit_remap -` on
  a sky takes back the cover first, then the swap). Kept as authored: the sun direction
  and day tint of the map's first sky, the volumetric clouds (`r_clouds`), the global
  fog's clear colour; where a server's `misc_skyportal` view is drawn, the portal shows
  instead of any box, as in retail.

A client older than this release reads a document with a covered sky side and skips that
face alone (its face check refuses a sky side; the rest of the document applies), so
publish sky covers once a release carries them. Sky swaps already work on older clients.

## Weather

A `weather` edit (05/11/2026, Sol: "save remaps of weather too, synced with SJK, so I
can add rain, fog and stuff on some maps") keeps world effect commands for the map, the
ones `r_we` runs and maps name in their `fx_rain` and `fx_wind` entities: `heavyrain`,
`gustingwind`, `constantwind ( -5000 0 0 )`, `fog`, and SJK's own `mist`, the light
ground fog `r_weatherFog 2` lays ([rendering.md](rendering.md#weather)). They run in place
of the map's own weather, so a dry map gets rain and a rainy one snow. The edit is
`{"kind":"weather","commands":["heavyrain","gustingwind","mist"]}`, 1 to 16 commands of
at most 160 printable characters each, without `;` or `"`; a command the client does not
know is skipped when the weather is built, as a map's unknown effect name is. A map keeps
one weather edit per scope; on a server whose own weather edit exists, it wins over the
every-server one.

Who sees it: every SJK player with `cg_mapEdits` on (the default), as every edit; their
`r_weather 0` still turns weather off, and their own `r_weatherForce` replaces the
edit's as it replaces the map's. Like forced weather, an edit's weather stays off a map
with no sky surface at all, where it would fall indoors. The cover (rain under open sky
only, splashes, wet ground, puddles) is the map's, surveyed as for its own weather.

`mapedit_weather` with no words keeps the weather seen now: the forced weather (or the
edit's, or the map's), the `r_we` commands typed since the map loaded, and `mist` when
`r_weatherFog 2` lays its fog on a map without its own. So staff try weather with
`r_weatherForce`, `r_weatherFog 2` and `r_we`, then keep it. `mapedit_weather heavyrain,
gustingwind, mist` keeps the commands named (commas between them), `mapedit_weather -`
gives the map its own back. The panel's Weather page offers the same.

Clients from before this release do not know the kind and ignore the map's whole document
(`map edits ... ignored` in their log), so publish weather once players have the release
that reads it, and staff should not edit such a map on an older client: it would start
an empty document and a publish would replace the hub's list.

## Lights

Sol, 08/11/2026: "for mapedit_web I want to see lights so I can move them, delete them or
add new ones" ([lights.rs](../crates/sjk-viewer/src/map_edits/lights.rs)). Two kinds:

**Added lights** (`light`): `{"kind":"light","origin":[x,y,z],"color":[r,g,b],"radius":320,
"intensity":1}`, each colour channel 0 to 1 (not all 0), the radius 16 to 2048 units, the
intensity 0.05 to 8 (the colour is multiplied by it). Every SJK player with `cg_mapEdits`
on sees them, on every preset: each frame they go into the frame's point-light list, the
one `r_dynamiclight` lights (sabers, bolts, explosions) go through
([rendering.md](rendering.md#map-edits-lights)), so `r_dynamiclight 0` turns them off too.
That list holds 32 lights: at most 8 added lights a frame are drawn, those whose reach comes
nearest the view, in slots kept for them at the frame's start
(`PointLightList::reserve`), so effects never push an added light out and added lights
never push an effect out; a map with fewer added lights keeps fewer slots. They light as a
dynamic light does: stopped by the static world's walls in real-time lighting
(`r_dynamicLightShadows`), through walls on the baked presets as in the original game, no
shadows of actors. Lights are never merged: two can stand side by side. Each applies on
every server or one, as every edit.

**The map's lamps** (`lamp`): the real-time lighting's local lights, found from the map's
emissive surfaces, its props' glowing model surfaces, BSP flares and steady effects when
its world is built ([rendering.md](rendering.md#inferring-fixture-light-from-legacy-materials)).
Only the Experimental preset finds and draws them; on the others there is nothing to edit
and a lamp edit shows nothing. A big map has thousands of lamp patches (mp/ffa5 about ten
thousand), so they are grouped by **fixture**, the thing they come from, named by the BSP's
own numbers, stable for every load of that BSP: `surface:<n>` (a BSP surface, the patches
of its triangles; BSP flares too), `prop:<entity>:<n>` (surface `n` of the model of the
`misc_model_static` entity `entity`) and `effect:<entity>` (a steady `fx_runner`). A lamp
patch belongs to the surface of its first triangle; the vertex-coloured flare clusters
span surfaces and are not editable. The edit is
`{"kind":"lamp","fixture":"surface:1043","name":"textures/kejim/light","off":true,
"offset":[0,0,0]}`: turned off (its lamps are dropped) or moved by `offset` (each of its
lamps moved by as much, at most 4096 units along each axis), one edit per fixture and
scope, `name` its shader, model or effect, kept to name it. The surface or model still
draws and glows: only its light changes.

The lamps are found while the world is built, and their lookup grid, shadow atlas and
light cache are made from them then (seconds of a big map's load), so a lamp edit applies
when the map's world is built again: the next load of the map, or `vid_restart force`
([rendering.md](rendering.md#graphics-reload)). The build reads the map's document itself
(`lamp_edits_at_build`): the map's checksum, read as the file is loaded
(`session_content::load_bsp` notes it, `lights::note_loaded`), names the document, so a
player joining a server gets its lamp edits on the first load. Editing them live would
mean rebuilding all of that on every change; the page says which lamp edits wait.

| Command | Does |
| --- | --- |
| `mapedit_light [<x y z>] [<r g b> <radius> <intensity>]` | Add a light at the point, or 12 units in front of what is aimed at (the selection, else the crosshair's); a warm 320-unit light by default |
| `mapedit_light_set <id> <x y z> <r g b> <radius> <intensity>` | Set added light `id` exactly (the page's gizmo and fields), one step to undo |
| `mapedit_lamp <fixture> off\|on\|move <dx dy dz>\|reset` | Turn a lamp fixture off or on, move it to an offset from where the map has it, or take its edit away; refused for a fixture this world has no lamp of (unless it has an edit) |

`mapedit_delete <id>` removes either kind, and every change is one step to undo. The
in-game panel lists them on its Edits page (`Light: #ffd9a6 r320`, `Lamp off: light
(surface:1043)`), where Delete, Undo and Redo work; it has no page of its own for them, and
the in-game gizmo does not move a light. The browser editor's [Lights](#lights-in-the-page)
page is where they are seen and edited.

Clients from before this release do not know the kinds and ignore the map's whole document
(`map edits ... ignored` in their log; `an_unknown_kind_refuses_the_whole_document`), so
publish light and lamp edits once players have the release that reads them, and staff
should not edit such a map on an older client (it would start an empty document and a
publish would replace the hub's list). The hub needs nothing: it keeps the edits as sent
and checks only their shape and size (hub `PROTOCOL.md`, Map edits).

A remap edit wins over the map's own worldspawn remaps and the server's, so a fix made
for a server's remaps holds; a console `remapShader` made after them still wins
([rendering.md](rendering.md#server-shader-remaps)). `listRemaps` names them `edit`.

## Documents and servers

Each map's edits are one JSON document per map and BSP checksum (`CM_LoadMap`'s, the
one servers send as `sv_mapChecksum`), so they apply only to that exact file:
`GameData/SJK/map_edits/<map>_<checksum>.json`, `mp/ffa3` as `mp_ffa3_-1234.json`.
Entity and surface numbers are the BSP's own. Each edit has an id and applies on every
server, or on one server only (`"server": "ip:port"`): the same map can carry fixes for
all servers and others for one server's remaps or layout. The map's checksum is read on
a worker, and only when a document of the map is there or the editor is on.

The identity worker reads the hub's list every 10 minutes once registered, and at once
when the editor turns on, after a publish, when `mapedits_sync` (Settings > Game "Sync
map edits", any player) asks, and when the hub's map edits stamp changes: every presence
answer (polled every 5 seconds on a server) carries it, the sum of every map's revision,
which grows with every publish, delete or undo (hub `PROTOCOL.md`, 03/11/2026; an older
hub leaves it out and only the timed look remains). The first stamp seen on a run counts
as a change. So a publish reaches the players on a server within seconds. Each look
downloads the documents newer than its copies and removes the copies of documents the hub no longer lists. A copy changed on
this PC and not yet published (`"unpublished": true`) is never overwritten or removed.
The loaded map's document is read again when a copy changes. What each look did goes in
the log when it changes (`map edits: 3 maps on the hub, 1 copy changed`, or `map edits:
not in step with the hub: <why>`), and the panel shows a failure at its foot. A hub
without map edits answers 404, worded "This SJK hub does not keep map edits yet"
(`sjk_identity::map_edits::refusal`).

A publish replaces the hub's whole list for the map: two staff members editing one map
should take turns, or the later publish drops the other's edits.

## The editor (staff)

`mapedit` turns the editor on for SJK staff (the hub's staff flag), and so does the
in-game menu's Map editor icon (a staff key's, beside Staff tools). Bound to a key it is a
plain toggle (04/11/2026, Sol: "a clean on/off bind"): one press enters the editor, its
[panel](#the-panel) and its [free camera](#flying-in-the-editor), one press leaves it fully,
from any state, the panel's keyboard included (the panel hands that key to the editor
before anything else, unless a search field types the letter). While it is on the panel
always shows: there is no hidden half-state (before 04/11/2026 Escape or Aim hid the panel
to aim, with a strip at the top of the screen; both are gone). The world-note key still
selects what is under the crosshair (Shift with it adds to the selection), never a note;
a command without a selection acts on what is under the crosshair.

| Key | Does |
| --- | --- |
| The key bound to `mapedit` | Leave the editor (or enter it) |
| Right mouse button, held on the scene | Fly: the mouse looks, the movement keys fly ([below](#flying-in-the-editor)) |
| Click on the scene | Select what is under the pointer (a grabbed model: put it down) |
| Shift+click | Add the surface, hidden face or placed model under the pointer to the selection, or take it out |
| World-note key (Shift: add) | Select (add) what is under the crosshair, flying or not |
| Escape | Take back a gizmo's drag or transform; else put a grabbed model back; else drop the selection; else leave the editor |
| G, R, T; X, Y, Z; 1, 2, 3; L | The [gizmo](#gizmos)'s, only while the right button is not held |
| Ctrl+Z, Ctrl+Y | Undo, redo |

### Flying in the editor

In a live game the editor's view is the free camera (`freecam`, `cg_freeCamera`): the
editor turns it on when it starts and off when it leaves (not when it was on already), the
body staying at its server position ([map_edits/fly.rs](../crates/sjk-viewer/src/map_edits/fly.rs)).
As in Blender's and Unreal's editors the pointer stays free for the panel and the scene;
holding the right mouse button on the scene looks around with the mouse and flies with
the keys bound to the movement commands (W, A, S, D, Space and C up and down, by default
binds; AZERTY's Z, Q, S, D the same way), Shift four times faster, the wheel setting the
speed (by 1.25 a notch, a quarter to eight times; the answer line says it). The flight
goes at the body's speed times that, whether a walk key halves the move or not
(`Predictor::set_fake_noclip_boost`, client only). Releasing the button gives the pointer
back to the panel where it was. While it is held the in-game menu steps aside (its page
kept, the panel still drawn) so the keys and the mouse reach the game as in play, the talk
balloon down: only the keys bound to a movement command and the world-note key act, the
others wait (no chat, weapon, scoreboard or gizmo key mid-flight), the buttons attack
nothing, and losing the window's focus ends the flight.

The free camera needs the player alive, on foot, out of a duel and not following anyone
(`fake_noclip_allowed`). When it is refused the editor works from the player's own view
(the right button then turns and moves the player) and the panel's foot says why in one
line; the next press of the right button tries again (after a respawn, say). Exploring a
map without a server walks with its own movement.

| Command | Does |
| --- | --- |
| `mapedit [on\|off]` | Turn the editor on or off (alone: the other way) |
| `mapedit_panel` | Open [the panel](#the-panel) (turning the editor on) |
| `mapedit_hide [group]` | Hide the selected prop or surface (`group`: the whole merged model); on a placed model, remove it |
| `mapedit_place [model]` | Place an MD3 where you aim, facing you: the path given, or a copy of the selected model |
| `mapedit_turn [degrees]` | Turn the current placed model (15 by default); a multiple selection's placed models all turn about Z through their middle |
| `mapedit_move <up> \| <x y z>` | Move it (a multiple selection's placed models: all of them) |
| `mapedit_scale <factor>` | Scale it (0.01 to 64 times); a multiple selection's placed models grow about their middle, their origins spreading |
| `mapedit_grab` | It follows your aim until `mapedit_grab` again; meanwhile the mouse wheel turns it (15 degrees a notch, 1 with Shift) instead of changing weapon. One model at a time: refused on a multiple selection of placed models (G moves them) |
| `mapedit_set <id> <x y z> <pitch yaw roll> [<sx sy sz>]` | Set a placed model exactly (the [gizmos](#gizmos)' command); repeated in groups of ten (with the scale) it sets several at once, all or none, one step to undo |
| `mapedit_remap <shader> \| -` | Draw `shader` instead of the selected one; `-` removes that remap edit. On a caulk or nodraw face: draw that face with `shader`, `-` draws it as nothing again (as does `mapedit_hide`). On a sky: a texture covers that side, a sky shader swaps the sky |
| `mapedit_sky <sky> \| - [all]` | Swap the selected sky for sky shader `sky` (`all`: every sky of the map, without a selection); `-` puts the map's back |
| `mapedit_link plane\|surface` | The selected caulk, nodraw or sky face and the faces joined to it (Same plane, Connected surface, [as the page's](#the-browser-editor-creator-only-for-now)) become the selection |
| `mapedit_fill [shader]` | Draw the selected faces with the shader the drawn surfaces round them use (the nearby vote's first), or `shader`; one step to undo |
| `mapedit_weather [<command>, ...] \| -` | Keep the weather seen now for the map, or the world effect commands named (commas between); `-` gives the map its own weather back ([Weather](#weather)) |
| `mapedit_light`, `mapedit_light_set`, `mapedit_lamp` | Add and set lights, turn the map's lamps off or move them ([Lights](#lights)) |
| `mapedit_scope [all\|server]` | New edits for all servers (the default) or only this one |
| `mapedit_list`, `mapedit_delete <id>` | List, remove one |
| `mapedit_undo`, `mapedit_redo` | Undo the last change to the map's edits (a hide, a placing, a turn, a move, a remap, a delete), or redo it |
| `mapedit_publish` | Send the map's document to the hub for every SJK player |

With a multiple selection (more than one surface, face or placed model), `mapedit_hide`
hides its surfaces, draws its faces as nothing again and removes its placed models, and
`mapedit_remap` draws its faces with the shader and remaps its surfaces' shaders (`-` takes
those edits away; placed models keep theirs), each in one step to undo, as the page's batch
(`map_edit_many`); Hide whole model takes one surface alone. `mapedit_turn`, `_move` and
`_scale` act on its placed models together as one `mapedit_set`, one step to undo.

Every change is saved to the map's document at once and drawn at once; it reaches other
players only with `mapedit_publish`, whose answer (published with the hub's revision, or
why not) shows in the panel, the chat and the log. The current placed model is the last
placed, the placed model selected, or the one chosen in the Edits list. Undo keeps the
last 100 changes of the map ([editor.rs](../crates/sjk-viewer/src/map_edits/editor.rs),
`History`), forgotten when the map changes or the hub brings another staff member's
version.

## The panel

The editor's commands as an SJK UI panel (19/10/2026, Sol's request:
[sjk-ui.md](sjk-ui.md#map-editor) has its look), in
[panel.rs](../crates/sjk-viewer/src/map_edits/panel.rs). It shows, with the pointer free,
whenever the editor is on in a world (reopened by itself once a menu, the console or a
dialog over it closes); `mapedit_panel` (bindable) opens it too. A click on the scene left
of the panel selects what is under the pointer, Shift+click adds to the selection, the
right button held flies, Escape backs out one step at a time ([keys](#the-editor-staff)).
While the editor is on a selection never goes stale (a world note's waits 20 seconds),
until the map changes.

**Multiple selection** (04/11/2026, the page's in game): Shift+click (or Shift and the
world-note key) adds the surface, hidden face (caulk, nodraw or sky side) or placed model
under the pointer, or takes it out; a map prop or a game entity is selected alone (a
Shift+click on one says so), up to 1024 in all and 256 placed models (what one `mapedit_set`
sets, so a gizmo's drag stays one step to undo; `world_notes` `toggle_in`). Placed models
mix with surfaces and faces, as in the page. Each one is outlined and shaded as the
selection is, a placed model's outline following it as it moves; the panel names the
newest ("Hidden face and 11 more"). Hide, Remap (Draw, Cover side), Remove remap (Draw
nothing) and Fill act on all of them as one step to undo (Hide removes the placed models
too; a remap and Fill leave them); Same plane and Connected replace them with the faces
joined to the newest.

**Several placed models** (04/11/2026): with only placed models selected, Selected says
"3 models selected" and offers Remove all, Turn left and right, Up and Down, Left, Right,
Away and Closer, Smaller and Bigger, each acting on all of them about their middle in one
step to undo; Grab takes one model at a time (its button is off, `mapedit_grab` says to
use G). Place's placed-model buttons do the same on the set ("3 selected: these move them
all"). A set mixing placed models with surfaces or faces keeps the newest's page, says how
many placed models it holds ("2 placed models among them: G R T move them"), and its Hide
all takes everything. The [gizmo](#gizmos) stands at the set's middle.

| Page | Offers |
| --- | --- |
| Selected | what it is (map prop, placed model, surface, game entity, hidden face: a caulk or nodraw side nearer than any drawn surface, sky: a sky side) with its model and shader, and how many more are selected; on a sky the skies come first in the list, and its buttons are Swap sky (Cover side when a texture is chosen), Swap all skies and Take back; Hide (Remove for a placed model; Hide all for several), Hide whole model (a merged model's surfaces), Place a copy; on a hidden face or a sky side Same plane, Connected (`mapedit_link`) and Fill (`mapedit_fill`), the shader Fill would draw named under it with its share of the vote ("Fill: yavin/stone_walk (62 % nearby)"); Draw another shader: a search over the shaders a remap may draw (the world's `textures/` and the map objects' `models/`, shaders and images, never the player models'), the Recent rows first, opened on the selection's own folder with its own shader chosen, five rows; Remap (or a second click on the chosen row) and Remove remap. A name the list does not know is tried as typed |
| Place | every MD3 of the game data outside the player models, the Recent rows first, narrowed by a search, six rows; Place where you aimed (or a second click on the chosen row); the current placed model's Turn left and right (15 degrees), Up and Down (8 units), Left, Right, Away and Closer (8 units along the world axis nearest the view's right and ahead, so steps add up straight), Smaller and Bigger (by 1.1), Grab and Remove. Shift held makes the steps 1 degree, 1 unit and a tenth of the growth |
| Weather | the map's own weather and what an edit keeps here; Drizzle, Rain, Storm and Snow (`r_weatherForce`'s four, keeping the fog), Fog (`mist` on, or every fog off) and Wind (`wind` on, or every wind off), Keep what I see (`mapedit_weather`) and Map's own (`mapedit_weather -`); an ember line while your own `r_weatherForce` hides it |
| Edits | the map's edits, newest first, eleven rows, named shortly (`3: Placed: crate.md3`, `Hidden surface: floor (1043)`, `Remap: wall -> wall2`, a server's address when for one), those for another server dimmed; choosing a placed model makes it the one Place's buttons move; Delete the chosen one, Undo and Redo; the Recent rows' switch ("Recent: remembered" or off) and Forget recent |

Under them on every page: New edits for All servers or This server, the last command's
answer (ember when refused; Publish's comes when the hub answers; with nothing to say, a
failed look at the hub), Publish (gold while the map has edits not published) and Editor
off, then three lines of keys (Right button fly, Click select, Shift+Click add; G R T
gizmo, Esc back; Ctrl+Z undo and the key bound to `mapedit` to leave; while flying, the
movement keys, Shift and the wheel), and, when the free camera was refused, why, in ember.
A click on the scene selects what is under the pointer and shows it on Selected; on Place
it stays there and chooses where Place puts the model. Grab keeps the panel: the model
follows the pointer (the crosshair while flying), the wheel turning it, and a click on the
scene, the world-note key or Put down sets it there; Escape puts it back. Keys: Up and Down
(along a list too; in a search they choose in its list), Tab, Left and Right on the tabs
and the scope, Enter acts (in a search it places or remaps), Ctrl+Z undoes, Ctrl+Y (or
Ctrl+Shift+Z) redoes, Delete on an edit deletes it. Each button runs the command the
console would (`mapedit_hide`, `mapedit_place <model>`, `mapedit_link surface`,
`mapedit_fill`, ...), so the console, the panel and the page stay one editor.

## Gizmos

Blender's transform handles for placed models, in game and in the browser editor (Sol,
04/11/2026: "add gizmos on the web/ingame to move on an axis the selection or rotate it,
scale it, the classic stuff like in Blender";
[gizmo.rs](../crates/sjk-viewer/src/map_edits/gizmo.rs), the page's `sjk/gizmo.js` with
the same maths):

| Mode | Handles |
| --- | --- |
| Move | an arrow per axis (X red, Y green, Z blue), a square per plane (the colour of the axis it leaves out), the centre circle in the view's plane |
| Rotate | a ring about each axis (pitch, yaw and roll as JA's angles come out of it), and a white ring about the view's axis |
| Scale | an arm with a box end along each of the model's own axes (a placed edit keeps a scale per axis), the centre circle for all three |

The axes are the world's, or the model's own (Local, L); scale is always along the
model's own. A drag on a handle previews the change on the model only; the release sends
`mapedit_set` once with the exact origin, angles and scale: one step to undo and one write
of the document per drag, whatever the mouse did. Ctrl held snaps (8 units, 15 degrees, a
tenth of scale; the page's bar sets the steps, its browser keeps them), Shift makes the
drag fine (a tenth). The gizmo is drawn over everything, the same size on screen at any
distance, the handle under the pointer gold.

Keys, as Blender's where the editors leave them free: G moves, R rotates and T scales with
the mouse from where it is (S flies back in the page, so T; Blender's W, E, R would take
the page's W and E too); then X, Y or Z keeps to the world's axis, the same key again to
the model's own, a third time to none; Shift with it keeps to the plane without that axis;
a number typed (`-` flips it, Backspace takes a digit back) sets the amount: units, degrees
or a factor (with no axis: along X, about the view's axis, on all three); Enter (or Space,
or a click) puts it down, Esc or a right click takes it back. 1, 2 and 3 choose the
gizmo's mode, L the axes.

Several placed models move, turn and scale together about the middle of their origins (the
pivot, where the handles stand): each origin orbits the pivot as each model turns by as
much, and grows away from it as each model grows; one `mapedit_set` sets them all, one step
to undo (256 models at most, which the selection keeps to). Along an axis, each model
scales along its own axis nearest it. Local axes (L, or an axis key twice) are the newest
model's, the one clicked last (Blender's active object), in game and in the page alike.
Shift+click adds placed models to the multiple selection in both (each outlined); Esc
takes the whole transform back.

In game the gizmo shows with the panel (flying too), on the selected placed model, else
the current one (Place's), and a press on a handle in the scene drags it; G, R and T, 1, 2,
3 and L are the panel's keys unless a search field types or the right button is held (the
movement keys fly then; S is one of them, which is why scale is T), and while a transform
runs every key is its own and the right button takes it back instead of flying. The keys
lines at the panel's foot say `G R T gizmo`, and X Y Z, Enter and Esc while it transforms;
the answer line names the change as it goes (`Move 16.0 0.0 0.0 (typed 16)`). A transform
makes the model (a set's newest) the current one and keeps the selection, whose outline
follows the models (`world_notes` `follow_placed`, also after an undo). In game the gizmo
is on the multiple selection's placed models when it holds any, so a set mixing models and
faces moves its models.

In the page, the bar at the view's top left has Move, Rotate, Scale, Local and the three
snapping steps, and the change under way; the controls card lists the keys. Your own
boombox chosen on World gets the gizmo too, along the floor and about Z only: the end
stands it again there (`set_thing`, `place_boombox_at`, which puts it on the floor under
that spot), as Your boombox's click does. Mystery boxes and Toy Master toys get none: the
hub places and removes them but has no move, and a remove and a place would make another
box (a new id, its opened state and grand finds' records lost) or another toy (its
escort and hidden show started again).

## The browser editor (creator only for now)

`mapedit_web` edits the map in a web browser (03/11/2026, Sol's request). For now only
SJK's creator may use it (the hub's creator flag, Sol's key alone): the command refuses
everyone else, and the game checks the flag again on every request the page makes, so a
running page stops editing when it goes. The command turns the editor on, starts a small
HTTP server in the client on 127.0.0.1 at a free port and opens the default browser on
`http://127.0.0.1:<port>/?t=<token>`. Code:
[map_edits/web](../crates/sjk-viewer/src/map_edits/web/mod.rs).

The page draws the map the player is on in 3D with WebGL, with its shader scripts, its
`misc_model_static` props and its flares, and SJK's edits as the game draws them: hidden
props and surfaces left out, remap edits applied, placed models and shown faces drawn. It
starts at the player's view (My view in game goes back there). Drag looks, W A S D flies,
Q and E go down and up, Shift is faster, the wheel sets the speed. Its panel has the
in-game panel's pages: Selected (a click on the map selects what the game's own pick
finds along that ray; Hide, Hide whole model, Place a copy, a shader list with pictures
and Remap, Remove remap), Place (the models list; a click on the map places the chosen
one, then Turn, Up, Down, Left, Right, Away, Closer, Smaller, Bigger, Remove, Shift for
fine steps), Edits (the list, Delete, Undo, Redo; a double click flies to a placed
model) and World (below), and under them the scope (All servers, This server), the last
answer and Publish. H hides the selection, Ctrl+Z and Ctrl+Y undo and redo, Esc stops
placing.

The keys act only while the 3D view has focus (a click or a drag on it gives it, a button
of the panel hands it back): never while a text field has it or text of the panel is
selected, so the fields take Ctrl+A, C, V and X as every field does and the panel's text
(the summary, the facts, the lists, the answer) can be selected and copied. Before
03/11/2026 (second round) the panel was written anew with every look at the game's state,
about once a second, which dropped any text selected in it: that was why nothing could be
selected to copy. A text is now written only when it changed.

A small card in the view's corner lists the keys and mouse actions; `?` in the view (or a
click on its head) folds it, and the browser keeps it folded (`localStorage`, per port:
it unfolds again for a new port, which is harmless).

**The selection drawn** (Sol, 03/11/2026: "make it more obvious what we selected"): over
the map, last, with the depth test and pulled a little in front (polygon offset), the
selected surface's triangles tinted gold and pulsing with their outline (the outline's
hidden parts faint through what is in front), the rest of the model Hide whole model
takes fainter (`target.group` from the game, the same surfaces `mapedit_hide group`
takes); a prop or a placed model tinted with a box around it (its MD3's first frame's
box, read from its file's head); a hidden caulk or nodraw side as its polygon
(`target.points`, the game's `faces::polygon`). The page builds a surface's triangles
from the map file's own lists (`pos` and `tri` of its batch, a patch at its finest). The
panel names the selection: its kind, its edit id, model, entity, shader (the one aimed
at on a model), surface number and kind, how many surfaces the whole model is, brush
and side, the remap it is drawn with, and where.

**Recent textures and models** (Sol: "remember in general what textures we recently
used"): a Recent row at the top of the shader list and of the models list, newest first,
24 each: the shaders remaps, shown faces and fills drew and the models placed, by the
in-game editor or the page ([recent.rs](../crates/sjk-viewer/src/map_edits/recent.rs)). The
game keeps them in `map_edits/web_recent.json` of the SJK config folder, so they outlive
the page, the game and the server's random port (the page's own storage is per port). The
in-game panel's lists start with the same rows (04/11/2026), each marked Recent, those the
search keeps. "Remember recent textures" (both pages, and the panel's Edits page) off
empties them and keeps them empty until it is on again.

**Copy and paste** in the view: Ctrl+C copies the selection, a placed model or a prop
with its angles and scale, or a surface's or a face's shader (the remap's when one draws
it), as plain text (the model's or the shader's path) and as the page's own copy. Ctrl+V
pastes a model as a placed copy where the pointer is (`mapedit_place <model> <pitch yaw
roll> <sx sy sz>`, one step to undo) or a shader as a remap, or a shown face, of the
selection. A model or a shader path copied elsewhere pastes the same way.

**World** (Sol: "also boombox placement, the toymaster cat and box"): what other parts of
SJK stand on the map, drawn in the page with their models and listed: the player's own
boombox on the floor and the other players' ([client.md](client.md), `boombox.rs`), the
mystery boxes ([mystery-boxes.md](mystery-boxes.md)) and the Toy Master's toys
([toy-master.md](toy-master.md)). A click on one in the view or the list selects it (a
box drawn round it); a double click flies to it. Your boombox, Mystery box and Toy (of a
kind the hub names, for this server only, shown only where the creator is) place one
where the next click meets the floor, facing the view, through their own code as their
commands do (`place_boombox_at`, `mystery_box_place_at`, `toy_master_place_at`; the
commands aim from the player, the page gives the spot). Remove takes the boombox back on
the shoulder and asks the hub to remove a box or a toy (`mysterybox_remove <id>`,
`toymaster_remove <id>`). The boombox moves by being placed again; the hub has no move
for a box or a toy (it places and removes them), so neither does the page: remove one and
place it again. Boxes and toys need a game server, as their commands do, and stay the
creator's; the boombox follows its own rules (verified, not dead, spectating or
following).

**Invisible brushes** (round 3, Sol: "see brushes like `r_showBrushes`"): Brushes in the
view's corner, or B, draws the faces the game's [`r_showBrushes`](client.md#invisible-brushes)
draws, see-through in each kind's colour with their outlines, hidden behind what stands in
front of them, as in the game. The game sends them (`api/brushes`), sorted by its own code
(`brush_view::each_face`, [kinds.rs](../crates/sjk-viewer/src/brush_view/kinds.rs)): the
same kinds, names and colours, plus `water` for water, lava and slime volumes, which the
game's view leaves out. A card lists the kinds the map has with their counts, each on or
off, with Play (what `r_showBrushes 1` shows), All, Caulk (caulk, nodraw, invisible walls)
and None; the browser keeps the view and the kinds (`localStorage`). Labelled tiles are
not drawn, only colours. A click on a caulk or nodraw face it draws (from its front)
selects that face: the page names it to the game, which takes it unless a drawn surface
stands more than a unit in front of it ([faces.rs](../crates/sjk-viewer/src/map_edits/faces.rs)
`met`).

**Multiple selection**: Shift+click (or Ctrl+click) adds the surface or hidden face clicked
to the selection, or takes it out; the panel counts them and every one is drawn as the
selection is. A shader then applies to all of them at once, from the list, Remap, a Recent
tile, a paste or Fill: each face is drawn with it and each surface's shader is remapped
(the panel says so: a remap affects every surface with that shader, anywhere on the map).
Hide hides the surfaces, draws the faces as nothing again and removes the set's placed
models (`hide_many`'s `placed`, 04/11/2026; before, a set of placed models only could not
be hidden). Either is one step to undo, whatever the count (`apply_many`, `hide_many`:
`map_edit_many` in
[editor.rs](../crates/sjk-viewer/src/map_edits/editor.rs)). Esc stops placing, else drops
the selection.

**Linked caulk** (Sol: "detect all linked caulks"): on a selected caulk or nodraw face,
Same plane and Connected surface select every face joined to it that players see, at most
512 (the panel says when it stops there, and how many joined faces it left out as unseen).
Two faces join when they share a stretch of edge (within half a unit): on the same plane
facing the same way, or at any angle but back to back. A face counts as seen when what is
left of it, once the other solid world brushes touching it cut away what they cover or lie
flush against, keeps a square unit and lies in front of an open leaf of the map (a
visibility cluster, not the solid void), the way q3map2 keeps a side; faces join where
their seen parts meet ([linked.rs](../crates/sjk-viewer/src/map_edits/linked.rs), on the
game thread, the map's brushes indexed once per map).

**Fill with the nearby texture** (Sol: "replace visible caulks by nearby textures"): for the
selected faces the panel shows the shader the drawn surfaces round them use, with its
picture and its share, and the next five; Fill draws every face with the one chosen (the
first by default), one step to undo. Each drawn triangle of the world (planar surfaces,
the triangle soups of merged models and -meta, patches by their control grid) votes for
its shader with the length of edge it shares with a face, three times over when it lies on
the face's plane, plus, on the plane within 24 units, the root of its area; back sides
do not vote, nor do sky, nodraw, caulk, trigger, liquid, fog and see-through shaders by
their flags, nor scripts that make effects (blended or alpha-tested first stage, deforms,
polygon offset, no colour pass) ([nearby.rs](../crates/sjk-viewer/src/map_edits/nearby.rs)).

Every edit is a request the game thread runs as the in-game editor's command
(`mapedit_hide`, `mapedit_place <model>`, `mapedit_turn`, `mapedit_move x y z`, ...), so
it is drawn in the game at once, saved, undone and published exactly like the in-game
editor's, and the in-game panel shows its answer. The page reads the game's state about
once a second (`api/edits`: the map, the edits, what they draw, the selection, the view),
so a map change loads the new map in the page; the server stops with `mapedit off` and
when the game quits.

The drawing is VisionJK's web admin page (by Blackwolf, GPL-2.0, used with his approval;
VisionJK's panel comes from JKAdmin's by Lixsl), cut down to the map: its files under
[web/page/vjk](../crates/sjk-viewer/src/map_edits/web/page/vjk) are copies, each headed
with where it comes from and what SJK changed (the batches keep each surface's triangles so
an edit can hide one; the props keep their entity numbers), `models.js` is its MD3 loading
and drawing taken from three of its files, and `editor-remaps.js` is only the part that
draws remaps and the shaders' pictures. Players, the HUD, effects, weapons, server
entities, clips, routes and bots are left out: map edits change only what is drawn.
`web/page/sjk` is SJK's own (the stubs for the parts left out, the edits, the panel, the
camera and the frame). The page is put together at compile time from `index.html`'s
`<!--#include -->` lines ([page.rs](../crates/sjk-viewer/src/map_edits/web/page.rs)), no
build tools.

The server ([http.rs](../crates/sjk-viewer/src/map_edits/web/http.rs)) answers only on
127.0.0.1, only to requests that name it as `127.0.0.1:<port>` or `localhost:<port>` in
`Host` (a site that points a name of its own at 127.0.0.1 is refused), and only with the
random token: in the page's address, then as the `X-SJK-Token` header of every `api/`
request. A POST must come from the page (`Origin`) as JSON. Files are read only from the
game data, by a path the game data accepts (no `..`, no absolute path):

| Request | Answers |
| --- | --- |
| `GET api/map?name=<map>` | `maps/<map>.bsp` |
| `GET api/file?name=<path>` | a file of the game data (images, shader scripts, models) |
| `GET api/shaders` | `{"files": [...]}`: the shader scripts in the order the game reads them |
| `GET api/list?kind=dir&dir=&ext=` | the folders and files of a folder, as VisionJK's |
| `GET api/edits` | the game's state (above) |
| `GET api/lists` | the models to place, the shaders a remap may draw and `skies` (`{name, picture}`: every shader with `skyParms`, its box's `_ft` image or its first stage's) |
| `POST api/edit` | `{"op": "select"\|"clear"\|"hide"\|"place"\|"turn"\|"move"\|"scale"\|"remap"\|"sky"\|"scope"\|"current"\|"delete"\|"undo"\|"redo"\|"publish"\|"place_thing"\|"remove_thing"\|"apply_many"\|"hide_many"\|"set"\|"set_thing"\|"add_light"\|"set_light"\|"lamp", ...}`; `sky` takes `shader` and `all` (`mapedit_sky`) |
| `GET api/lamps` | `{"map", "realtime", "fixtures": [{id, kind, name, position, color, power, radius, lamps}], "applied": {"off": [id], "moved": [[id, [x, y, z]]]}}`: the lamp fixtures as this world found them (before edits) and the lamp edits it was built with |
| `GET api/recent` | `{"remember", "shaders", "models"}`: the Recent rows |
| `POST api/recent` | `{"op": "remember", "on": bool}` or `{"op": "clear"}` |
| `GET api/brushes` | `{"kinds": [{name, label, colour, mode, selects, count}], "faces": [[kind, brush, side, [x, y, z, ...], normal]]}` |
| `GET api/link?brush=&side=&grow=plane\|surface` | `{"faces": [{brush, side, shader, points, sky}], "buried", "capped", "limit"}` |
| `POST api/suggest` | `{"faces": [[brush, side], ...]}` answered `{"candidates": [{shader, weight, share}]}` |

`api/edits` also carries the selection's details (above) and `things`: each thing's kind,
id, label, whether it is the player's own, where it stands and its parts (model, origin,
angles, scale as the game draws it at rest), whether the game is on a server, and the
hub's toy kinds. `place_thing` takes `thing` (`boombox`, `mystery_box`, `toy`), the ray,
`kind` for a toy, `server` and `escort`; `remove_thing` takes `thing` and `id` (none for
the boombox). `add_light` takes the ray (`eye`, `dir`, `forward`), `color`, `radius` and
`intensity`, selects what the ray meets and runs `mapedit_light <r g b> <radius>
<intensity>`; `set_light` takes `id`, `origin`, `color`, `radius` and `intensity`
(`mapedit_light_set`); `lamp` takes `fixture`, `action` (`off`, `on`, `move`, `reset`) and
`offset` (`mapedit_lamp`); each is checked against the document's limits before the game
sees it. `api/edits` carries `lights` (the added lights drawn now) and `lights_budget`.
`select` may name `face` (`[brush, side]`, the brush view's face); `apply_many`
takes `faces` (`[brush, side]` each), `surfaces` and `shader` (`-` takes their face and remap
edits away), `hide_many` `faces` and `surfaces`, 1024 at most in all. The game checks each
face again (a caulk or nodraw side of a world brush) and each shader as a remap's. `set` (a
gizmo's drag) takes `edits`, each `{id, origin, angles, scale}`, 256 at most, and runs
`mapedit_set` with them; `set_thing` takes `thing` (`boombox` only), `origin` and `yaw`.

### Lights in the page

The Lights page and the view's Lights button (08/11/2026, [Lights](#lights);
[sjk/lights.js](../crates/sjk-viewer/src/map_edits/web/page/sjk/lights.js)) draw the added
lights as bulbs in their colour and the map's lamp fixtures as small cubes in theirs (grey
when off), faint through walls and full where seen, each the same size on screen at any
distance; the browser keeps the button's state. A click on a marker chooses it (an added
light before a lamp at the same spot) and opens the page; the chosen one is ringed in gold,
a light with the circles of its reach. On the page:

- A note on who sees what: added lights every SJK player on every preset, 8 at most at
  once, the nearest; the map's lamps on the Experimental preset only, a lamp edit from the
  map's next load or `vid_restart force`; baked lightmaps never change.
- **Add light** (the next click on the map puts one 12 units in front of what it meets,
  Esc stops) and **Add at view centre**; the new light is chosen.
- Added lights: the list (double click flies to one); the chosen one's colour (a colour
  picker), radius and intensity, each change one `set_light` (one step to undo); Delete
  (or the Delete key on this page) and Fly to it. The gizmo moves it (Move only: R and T say
  so), one `set_light` at the end of a drag, as placed models' `set`.
- Map lamps: the fixtures `api/lamps` lists (searchable by fixture or shader), each with
  its kind, name, patch count, power, reach and position; Turn off / Turn on, Reset and Fly
  to it; the gizmo moves it (`lamp` with `move` and the offset from where the map has it, or
  `reset` when put back). The marker stands where the edit puts it. A fixture whose edit
  the game's world was not built with is marked "(on reload)", and the page says it waits
  for the map's next load or `vid_restart force`. Read the lamps again asks the game anew
  (also when the page opens); without Experimental the list says that lamps are found there
  only.

**The sky in the page** (04/11/2026): a click on the sky selects its side as the game's
pick does (the panel names it Sky, with what it is swapped for). Swap the sky lists the
skies of the game data with their pictures, a search and a Recent row (the skies among the
recent shaders), with Swap this sky, Swap all skies and Map's sky; the page's sky is drawn
as the swap's (`remapOf` in `prepareScripted`). Draw another shader covers the side
instead (Cover side), drawn as a shown face over the sky. The brush view has a `sky` kind
(off by default, as it would cover the view; not in Caulk), whose faces a click selects;
Same plane and Connected surface link sky sides with sky sides only, and Fill votes for
them from the drawn walls round them as for caulk.

Not yet: the map's own and the server's remaps are not drawn in the page (only remap
edits); a mover is drawn where the map file has it; Grab has no page version; a box or a
toy cannot be moved, so has no gizmo (the hub has no move); a toy's show and a box's opening are not
played in the page (drawn at rest). The brush view draws no labelled tiles; a click selects
caulk and nodraw faces only (a clip or trigger is not an edit). Linked caulk's "seen" is a
cover and open-leaf test: a face in an open place players cannot reach (between a caulk
shell and the sky, say) counts as seen, and a face behind a detail brush q3map2 kept
counts as unseen. The vote looks at drawn surfaces only, not at what edits draw. The page
draws one sky, the map's first (a map with several shows the first one's swap only).


## Limits

- Only MD3s already in the player's game data can be placed; a custom model needs a pack
  the players have (an SJK hub pack).
- A hidden prop's lamps and a hidden surface's bake stay; a placed model casts no baked
  shadow. No edit changes the baked light; a lamp edit changes only the real-time lamps
  (Experimental), from the map's next load or `vid_restart force`.
- At most 8 added lights are drawn at once (the nearest); they are dynamic lights, so
  `r_dynamiclight 0` hides them and they cast no shadows of actors.
- Edits are not drawn in the menu world.
- The hub's side is the hub repository's `map-edits` branch (19/10/2026). Until the live
  hub runs it, it answers 404: Publish says the hub does not keep map edits yet, and
  players get no copies (found 28/10/2026: the branch had never been merged there).
- A shown face's texture always lies at Radiant's default; the map's own projection of
  that side is not in the BSP.
- Verified: unit tests (documents, scopes, the remap order, the folder's sync against a
  fake hub, the refusals' words, undo and the panel's lists), the hub round trip against
  a local hub (`staff_publish_map_edits_that_every_key_reads` in `hub_e2e.rs`), and the
  world shot `duel6_prop_hidden_copied_and_floor_remapped` (a duel6 prop hidden, a surface
  hidden, a copy placed, a wall remapped, then all taken back), and
  `duel6_map_editor_panel` for the panel; `caulk_face_shown` (02/11/2026) selects a
  duel6 caulk wall under the crosshair and draws it with a wall shader. Not tried in a
  game.
- The browser editor (03/11/2026): unit tests of its HTTP parsing, the Host, token and
  origin checks, the paths and the edit requests, an end-to-end test of the server on a
  socket with a fake game thread (the page, `api/map`, an edit, refusals), and the page in
  headless Chrome (SwiftShader) on retail duel6 served by the ignored test
  `serve_the_page_for_a_browser` (no script errors; the map, its props and the panel drawn;
  a hidden prop, a placed copy, hidden surfaces and a remap from a fake state drawn; a
  click's request reaching the server). Not tried with the game running.
- Its second round (03/11/2026): unit tests of the Recent rows' file (newest first, 24,
  off forgets, a restart, a bad file), the pasted copy's angles and scale, the things'
  and the Recent rows' requests, `api/recent` end to end with its refusals; in headless
  Chrome on duel6 with `SJK_MAPEDIT_WEB_DEMO` and `SJK_MAPEDIT_WEB_SELECT`
  (`surface`, `placed`, `face`): the selection drawn (a pixel difference against no
  selection), the controls card and its fold kept over a reload, panel text selected by a
  mouse drag staying selected over the polls, `?` and Ctrl+A in a field left to it, the
  Recent rows, the World list, a copy, and pastes sent as a place and a remap. Not tried
  with the game running: the game-thread side (the selection's details, the things'
  placing and removal, the Recent rows' recording) is untested in a running client.
- The gizmos (04/11/2026): unit tests of the maths (JA's axes against the engine's own
  rotation and back to angles, a ray against an axis and a plane, an arm, a square and a
  ring dragged on a made-up view with snapping and fine, local against world axes, G, R
  and T with X, Y, Z, typed numbers and their cancel, two models turned and scaled about
  their middle), of `mapedit_set` (its groups, all or none, two models in one step to
  undo), of the `set` and `set_thing` requests, and the key lines' width. In headless
  Chrome on duel6 (`SJK_MAPEDIT_WEB_SELECT=placed`; the fake game applies `set`): the gizmo
  drawn on the selected model, the X arrow hovered and dragged 40 units with a preview and
  one `set` at the release, G X 16 Enter, Esc taking a move back with nothing sent, R Z 90,
  T 2, a ring drag, two models moved together in one `set`, the boombox's G X 32 as one
  `set_thing` and T refused on it, no script errors. The world shot `duel6_map_editor_gizmo`
  draws the in-game gizmo of each mode on a placed model and drags its X arrow with pointer
  events (the preview 40 units along X; the release is refused there, a shot is not
  staff). Not tried in a running client: the keys and the drag with the real pointer, and
  `mapedit_set` from a real drag.
- Its third round (03/11/2026): unit tests of linking (a caulk floor of three brushes, a
  crate on it, a top buried under a lid, a floor two units lower: same plane takes the three
  tops and leaves the buried one out, connected surface goes down the step; the limit; the
  void), the vote (the floor round a hole beats a wall and a strip further off, back sides
  and far surfaces do not vote, the skipped flags), the batch (a fill of three faces and two
  shaders' remaps is one undo step; remove and hide by scope), and the new requests and
  routes end to end. On retail maps offline (`linked_caulk_and_nearby_textures_on_retail_maps`,
  ignored, release build): the cover test agrees with q3map2's kept drawn sides on 91 %
  (duel6) and 99 % (ffa3) of them; building the brush view takes 15-19 ms (10175 faces,
  1.1 MB of JSON on duel6; 14949, 1.6 MB on ffa3), the side index 18-32 ms, a link under
  0.1 ms on a plane and 8-20 ms for a connected surface (duel6's run into the 512 limit).
  In headless Chrome on duel6 (`SJK_MAPEDIT_WEB_VIEW` sets the view; the fake game picks,
  links and votes with the real code): B draws the brush view and its card (clip, solid,
  caulk, breakable), a click selects a caulk face, Shift+click adds a second and takes it
  out again, the suggestion names `textures/yavin/stone_walk`, Fill sends one `apply_many`
  with both faces, Connected surface selects 512 faces, Esc clears, a reload keeps the view,
  no script errors. Not tried with the game running.
- The browser editor's tools in game, the editor's flight and its toggle (04/11/2026):
  unit tests of the multiple selection (add, take out, a model alone, the limit), the
  batch split and its single undo step, `mapedit` as a toggle and Escape's order, the
  flight's speed steps and the keys that fly, the predictor's boost (its speed with and
  without a walk key), the Recent rows in the panel's lists, the key lines' width and the
  pages' new controls. The world shot `ingame_link_select_and_fill` on retail ffa3 (below,
  under Limits). Not tried in a running client: the flight with the real mouse and keys,
  the free camera's start and end with the editor, the bind leaving from the panel, and
  Shift+click with a real Shift.
- Several placed models in game (04/11/2026): unit tests of the set's maths (a pivot, G X X
  along the newest's own axis against G X along the world's, R Z 90 orbiting the origins
  and turning each model, T 2 spreading them, Esc keeping the start, one `mapedit_set` of
  ten numbers a model), the selection rules (placed models join and leave, mix with faces,
  a map prop still alone, 256 placed at most), a placed model's outline following it (moved,
  turned and halved), Remove all and Turn left on a set as one step to undo each, `hide_many`
  with `placed`, and the panel's pages ("3 models selected", the mixed line, Place on a
  set). The world shot `ingame_gizmo_on_a_set_of_placed_models` on retail ffa3: three
  crates Shift+clicked into the set, the rotate gizmo at their middle, R Z 90 previewed and
  put down as one step (each origin within 0.05 units of its turn about the pivot), the
  outlines following, an undo bringing all three back. Not tried in a running client: the
  keys and drags with the real pointer, and the page's `hide_many` with placed models.
- Lights (08/11/2026): unit tests of the kinds (round trip in the hub's shape, the limits,
  one lamp edit per fixture, an unknown kind refusing the whole document), the fixtures
  (written and read back, the lamps grouped one marker per fixture, turned off and moved),
  the reserved dynamic-light slots (effects and added lights never evict each other), the
  nearest 8 drawn, scopes, the BSP fingerprint, the requests and `api/lamps` end to end
  ([http_e2e.rs](../crates/sjk-viewer/src/map_edits/web/http_e2e.rs)). The world shot
  `added_light_drawn` on retail duel6 draws a red light 40 units over the floor ahead of the
  first spawn: 64,913 pixels changed (64,829 redder), 192 left changed once it is taken away.
  The page in headless Chrome on duel6 with the fake game's lights and three made-up lamp
  fixtures (`serve_the_page_for_a_browser`, `SJK_MAPEDIT_WEB_DEMO`): the Lights page and its
  note, Add at view centre choosing the new light, a radius change sent as `set_light`, G X 16
  Enter moving it 16 units in one `set_light`, a lamp cube clicked and chosen, turned off and
  marked as waiting for a reload, moved 32 along X by the gizmo, R refused on a light, the
  Delete key removing it, no script errors. Not tried
  in a running client: the commands from the panel or the page, a lamp edit applied by a
  map load or `vid_restart force` on Experimental (no world shot builds a world with them),
  and the fixtures of a retail map as `api/lamps` lists them.
- The sky (04/11/2026): unit tests (a sky side picked, checked and met on a synthetic map
  while a sky-flagged liquid is refused; the brush view's `sky` kind and a link that takes
  sky only; sky links never reach caulk; the sky list and its pictures; a sky swap as
  remap edits, all skies in one step, taken back; the `sky` request; a document with a
  covered sky side and a swap). World shots on retail ffa3: `sky_face_covered` selects
  `textures/mp/h_tatsky` (brush 2784 side 31232) under the crosshair as a sky and covers it
  with `textures/desert/s_wall1`: the middle 81x81 pixels changed 100 % from three eyes a
  fraction of a unit apart (no sky through it), 82 pixels differ once taken back;
  `sky_swapped` swaps it for `textures/ahog/thesky` (a community pk3's sky in the test
  game data): 887503 of 921600 pixels changed, 30 after taking it back. In headless Chrome
  on ffa3 (the fake game picks with the real code): a click on the sky selects it as Sky,
  the list holds 271 skies (195 with a picture), Swap this sky sends `mapedit_sky` and the
  page's sky changes (66 % of the view), Cover side draws the wall over it, Map's sky takes
  the swap back, Same plane says Linked sky; no script errors. Not tried in a running
  game: the panel's Sky page, `mapedit_sky` and the covers drawn in play.
