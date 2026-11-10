# Rendering and UI

The wgpu renderer lives in `sjk-viewer`. JKA content enters through owned BSP,
model and shader data; GPU resources stay in the viewer. The renderer consumes
BSP geometry, PVS visibility, lightmaps, shader stages and legacy models.

## Where to work

Unused server-advertised rigid models load on first reference through
`config_string_refresh::model_demand`. CPU model parsing, texture decoding and
mip chains run on a single worker per world (`warm_entity_materials`, as for
players' models); upload and geometry/material installation stay with the GPU
owner. Idle scans borrow existing entity appearances and use a sorted set of
attempted models, without per-frame name allocation. The prepared textures go up
one a frame (an HD pack's texture with its mips is near 90 MB to copy), then the
model's geometry and materials, and new pipelines compile on a
`sjk-model-pipelines` worker before the model is drawn: `falcon2.md3` with an HD
pack, off screen on duel6 in a release build, held its worst frame 209 ms, now
31 ms. Models remain resident until the world is replaced. Models wanted by name
take the same queue (`GpuState::want_model`): the hats and capes players wear and the
models an effect needs, which used to load inside the frame that asked (6 to 10 ms a hat
on an RTX 5080, each new hat a joining player wore; #236). A new effect's shaders
rebuild the effect atlas on a `sjk-effect-atlas` worker (`particle_atlas::Pending`); the
frame only uploads the finished atlas. Built in the frame it held 210 ms as JoF's NPCs
spawned, and a shader with no picture had it rebuilt at every new effect; a shader a
build was started for is not built for again in that world. NPC bodies and vehicles
(`$swoop`, `$tauntaun`, an admin's spawned NPC) load the same way through
`npc_refresh::npc_loads`, one NPC at a time, the NPC unseen until its model is in.
Vehicle projectile, effect and cosmetic models still load whole on the render
thread (`load_config_model`), as do NPC saber hilts not yet loaded.

| Area | Entry point |
| --- | --- |
| GPU/device setup | [gpu_context.rs](../crates/sjk-viewer/src/gpu_context.rs) |
| World loading | [world_load.rs](../crates/sjk-viewer/src/world_load.rs) |
| World materials | [world_materials.rs](../crates/sjk-viewer/src/world_materials.rs) |
| Optional material maps | [material_maps.rs](../crates/sjk-viewer/src/material_maps.rs) |
| Main scene passes | [main_scene_pass.rs](../crates/sjk-viewer/src/main_scene_pass.rs) |
| Secondary views | [scene_views.rs](../crates/sjk-viewer/src/scene_views.rs) |
| Sun and real-time lighting | [sun_shadows.rs](../crates/sjk-viewer/src/sun_shadows.rs) |
| Movers in lamp shadows | [mover_occlusion.rs](../crates/sjk-viewer/src/mover_occlusion.rs) |
| Dynamic lights and walls | [dynamic_light_shadows.rs](../crates/sjk-viewer/src/dynamic_light_shadows.rs) |
| Post processing | [post_aa.rs](../crates/sjk-viewer/src/post_aa.rs) |
| Dynamic glow | [post_glow.rs](../crates/sjk-viewer/src/post_glow.rs), [glow_pass.rs](../crates/sjk-viewer/src/glow_pass.rs) |
| Eye adaptation | [post_exposure.rs](../crates/sjk-viewer/src/post_exposure.rs) |
| Frame timing | [frame_pacing.rs](../crates/sjk-viewer/src/frame_pacing.rs) |
| HUD integration | [hud.rs](../crates/sjk-viewer/src/hud.rs) |
| Material map generator (tool) | [sjk-materialgen](../crates/sjk-materialgen/src/lib.rs) |

The normal BSP path supports additional lighting, shadows, GI probes, ambient
occlusion, reflections and post processing. Feature presence does not establish
correctness on every map or GPU. Preserve the ordinary BSP/material path when
working on optional effects and validate shared WGSL programs on an actual GPU.

wgpu picks the graphics backend; the client requests none. On Windows Vulkan is enumerated
first, and `WGPU_BACKEND=dx12` selects DX12, which compiles shaders with FXC
unless `dxcompiler.dll` is on the `PATH`. FXC can only assign a runtime-indexed
vector or matrix component (`v[i] = ...`) by unrolling the loops around it, and
fails (error X3511) when any of those loops has a runtime trip count. Write such
updates as whole-vector operations, as the skinning loop in
[gpu_skinning.wgsl](../crates/sjk-viewer/src/gpu_skinning.wgsl) does;
[world_shader_fxc_tests.rs](../crates/sjk-viewer/src/world_shader_fxc_tests.rs)
checks the world programs for this pattern.

Material compilation merges a shader's first two stages into one multitextured
pass under rd-vanilla's `CollapseMultitexture` rules, and never a later pair. The
colour generators are compared after `ParseStage` defaults: an unset rgbGen is
identity, or identityLighting when the blend source is `GL_ONE` or `GL_SRC_ALPHA`
([stage_colour.rs](../crates/sjk-shader/src/stage_colour.rs),
[world_stage_collapse.rs](../crates/sjk-viewer/src/world_stage_collapse.rs)).

Programs are embedded with `include_str!`, so they carry the checkout's line
endings. Code that patches a program by text with a pattern spanning a line break
normalises it first with [wgsl_source.rs](../crates/sjk-viewer/src/wgsl_source.rs);
`.gitattributes` keeps `*.wgsl` LF in new checkouts.

## Server shader remaps

The client consumes multiplayer `CS_SHADERSTATE` on joining and live updates,
and reliable `remapShader` commands, including during demo playback. Replacements
use the shared world/model material compiler: stage images, blending, alpha tests,
texture animation, scrolling, waves and deforms are replaced together. Shader
names are case-insensitive and extension-independent. Aliases resolve one hop;
remapping a shader to itself restores it. The destination's time offset is shared
by its users and subtracted from shader time; server offsets are parsed like C
`atof`. Removing an entry from the server's configstring alone does not undo it,
matching stock cgame.

A target the client cannot draw (no shader script and no image of that name) leaves
the surface as it was, as stock does, and is logged once a map:
`shader remap ignored: missing <target> (not tried again on this map)`
([world_remap_material.rs](../crates/sjk-viewer/src/world_remap_material.rs)
`MissingTargets`). Remaps are applied again whenever the world registers a material it
had not drawn yet (a player's new skin, a prop loaded mid-match), and each slot of a
remapped model asks again; before 18/10/2026 every one of them searched the game data
and wrote that line again. JoF's night ffa3 (`dl_JoF_MapFFA3Night`, a `trigger_always`
firing `target_relay`s on `remap_kyle`) remaps the stock Kyle model's seven shaders to
`models/players/kyle_new/`, a pack the map does not carry: without it, about 170
lookups and log lines made each skin change a 400 ms hitch (`config-refresh` in the
[hitch lines](#gameplay-hitch-lines)). The list lives with the world, as its shader
catalog and game data do, so the next map checks again. Effect atlas remaps already
skip a target they could not load (`effect_remaps.rs`, `unloadable`); a sky remap to a
missing target costs one catalog lookup.

A replacement keeps the lighting of the surface it replaces: a lightmapped world
surface samples its own lightmap page, a vertex-lit one its vertex light, and an
entity material its entity light. Here SJK departs from rd-vanilla, whose
`R_RemapShader` draws the target as first registered: with `LIGHTMAP_NONE` when the
world never used it (a `$lightmap` stage becomes the white image, an unscripted
texture is lit by `rgbGen lightingDiffuse`), or with whichever lightmap page another
surface gave it. Either way the remapped surface ignored the map's light, uniformly
bright or patchy (08/10/2026). A target shader without a `$lightmap` stage still
shows only the lighting its stages ask for. A vertex-coloured stage (`rgbGen vertex`
or `exactVertex`) on a lightmapped surface shows that surface's baked vertex light,
which q3map2 stores beside the lightmap, as in rd-vanilla; it is marked as baked
light like a vertex-lit surface's, so real-time lighting replaces it. Before, a
`q3map_onlyvertexlighting` target such as `textures/yavin/stonewall2_vertex` kept
the static bake under real-time lighting, brighter than its neighbours and blind to
live light (`mp/ffa4` on a JA+ server).

An unlit environment shine over relit paint (an additive `tcGen environment` stage
with identity colour on a lightmapped or vertex-lit surface, such as
`textures/yavin/obsidian_templebasic`'s `alphaGen const 0.25` layer) follows the live
light in real-time lighting: its colour is scaled by the luminance of the light the
surface now receives over that of its bake (the lightmap texel, or the vertex colour),
at most 1 (`relighting::RELIT_SHINE`, [world_lighting_mode.wgsl](../crates/sjk-viewer/src/world_lighting_mode.wgsl)).
Retail adds the shine to paint lit by the bake. Real-time lighting relit the paint, six
times darker than its bake on the `mp/ffa1` wall `vjun/newfloor_vjun` is remapped to on
JoF, while the shine kept its authored strength and was added in linear light: it made
80% of the wall's brightness (0.103 of 0.126 in display values, where a display-space
sum as retail draws it would add 0.062) and hid the bricks behind a milky sheen (Sol's
world note, 12/10/2026: "way too reflective"). With the rule it adds 0.025 and the
bricks show. In baked lighting nothing changes: there the shine adds 0.055 where
retail would add 0.061. The stage carries the lightmap coordinates in its unused
second bundle's coordinates to read the bake.

The map's own worldspawn remaps apply when its world loads, as rd-vanilla
`R_LoadEntities` applies them: every key starting with `remapshader` (case-sensitive,
so `remapshader2` too) whose value is `old;new`, split at the first `;`. A value
without `;`, or an empty key or value, ends the scan as in C. Both shaders must
exist. These remaps are not gated by `cg_remaps` and last as long as the loaded
map. `vertexremapshader` keys apply in their place only under `r_vertexLight 1`
([vertex lighting](#vertex-lighting-r_vertexlight)).

Map, server and local remaps of one shader follow rd-vanilla, where every source
writes the same remapped-shader slot: the latest remap wins. Worldspawn remaps come
first; a later server remap replaces a local one, including a local restore, and
each `CS_SHADERSTATE` update applies all its entries again, as `CG_ShaderStateChanged`
does. The session keeps the server state and stamps each entry; the displayed map
keeps its worldspawn and local remaps and picks the latest of the three.

SJK's map edits ([map-edits.md](map-edits.md)) are a fourth source, `edit`: they win
over the worldspawn and server remaps of a shader, and a console remap made after every
server one still wins over them.
`cg_serverRemaps` (default **0**, since 18/10/2026) must be on for any server remap
to apply; it reads as `cg_remaps 0` while off. Then `cg_remaps` uses **0** to disable server remaps, **1** to exclude
`models/players/` sources in both configstrings and reliable `remapShader`
commands, and **2** (the default) to include all server entries. Mode 1 follows
JoF EJK #297; it previously retained Tayst's reliable-command exception. An
excluded player command also leaves the mode-1 destination time offset alone.
Worldspawn and explicit local console remaps are independent. The preference
applies live without reloading the map; an excluded server remap reveals the
earlier remap it replaced.
`cg_remapsBlockedMaps` lists maps on which SJK ignores server remaps, as `cg_remaps 0`
does there; worldspawn and local remaps still apply. `cg_remapsAllowedMaps` lists maps
whose server remaps apply even with `cg_serverRemaps 0` (a blocked map stays
blocked), and `cg_serverRemapsMap` shows and sets the loaded map's choice
(`default`, `on`, `off`) for Settings. The frame reads a cached answer,
refreshed when one of these cvars or the loaded map changes
([remap_blocked_maps.rs](../crates/sjk-viewer/src/remap_blocked_maps.rs)).
SJK's creator also keeps a shared list on the hub (since 05/11/2026; `PROTOCOL.md`
"Remap blocks"), given with presence and kept in the archived `cg_remapsSharedMaps`, so
it holds at the next start before the hub answers. While `cg_remapsShared` is on (the
default), a listed map whose own choice is `default` reads as `cg_remaps 0`; a map set
`on` or `off` keeps its own choice. `syncBlockRemaps` / `syncUnblockRemaps [map]`
change the list (creator only, `POST /v1/staff/remap-block`).
`listRemaps` lists the map's, the enabled server and the local remaps in the order
they were applied, with their source, and marks those a later remap overrides.
`remapShader <old> <new>` sets a temporary local remap for the loaded map, without
sending anything to the server or saving it to config. `clearRemaps` drops every
active remap, the map's included, until the server sends new entries.

Material recompilation and draw/fog/table invalidation happen on changes, not
per frame. Late-loaded entity materials also receive the current remaps.
A slot's first replacement keeps its loaded stages and classification aside. When
the slot maps to its own shader again at time offset zero they are put back, not
recompiled, so it matches its load state; only replaced slots keep a copy. A
nonzero offset on a slot's own shader compiles a replacement. Map
replacement/reconnection isolates server state. Authored sky-box remaps replace
the sky images while retaining the existing day/night policy.

Effect atlas shader names are looked up regardless of capitals, as shader names
are case-insensitive in the engine: the atlas keys them in lower case, and the saber
clash flare asks for `gfx/effects/saberFlare`.

Impact marks (decals) are drawn before every other effect, as rd-vanilla sorts mark
shaders (`sort decal`) ahead of blended effects, so an explosion's fire and smoke
cover its own scorch mark. Effect atlas stages honour `alphaGen const` and a grey
`rgbGen const` (JoF's HD scorch marks are 15% grey at 80% opacity). A coloured
`rgbGen const` is ignored: effect layers carry one brightness, so only a grey
constant is taken. Both constants are applied as a flat brightness and alpha
multiplier on the effect's own colour and alpha, not a replacement of them as in
rd-vanilla. The atlas has five mip levels: each 128-pixel picture sits in a
160-pixel cell whose 16-pixel border repeats its edge texels, so filtering at the
smallest level never reaches the next picture, and each level averages its 2x2
parents weighted by alpha. The effect mips are generated premultiplied, unlike the
plain `box_mip_chain` of the other mipmapped path.

World decals (`polygonOffset` shaders, sort `decal`) are drawn with rd-vanilla's
-1/-2 depth bias. A decal whose stages write no depth (`underlay` in
[world_materials.rs](../crates/sjk-viewer/src/world_materials.rs): nearly all of them,
which multiply or blend over their wall) is drawn in a phase of its own, after the opaque
world and before the opaque world's fog pass (`draw_decals`, in the main view, floor
mirrors, portals and reflection captures). The wall's `Equal` fog draw then covers the
wall and its decal alike, so a decal fades into fog with its wall. The decal's own fog
pass is left out: with the decal's bias and no depth of its own, its `Equal` test meets
nothing, in rd-vanilla too (whose `RB_FogPass` runs before the offset is disabled), and
until 13/10/2026 such a decal kept its full strength in fog, taking about 40% of the fog
of its wall (`textures/decals/mp_imperial_symbol` on `mp/ctf4`). A decal that writes
depth keeps its biased fog pass, which meets its own depth. Flicker of decals outside fog
is not explained by this; it is open in [status](status.md).

An effect atlas stage takes the effect's own colour (its EFX `rgb` and the fade of an
effect without `useAlpha`) except where it multiplies the scene (`GL_DST_COLOR` with
`GL_ZERO` or `GL_SRC_COLOR`) without `rgbGen vertex`, `exactVertex` or `entity`
(`ParticleLayerSample::tint`). Such a stage's white or mid-grey texels leave the scene
unchanged only at full colour: the repeater's alt-fire burst (`concussion.efx`) fades
its `rgbGen identity` shock balls, and until 11/10/2026 SJK drew them as dark squares.
rd-vanilla gives the effect's colour to no stage without one of those `rgbGen`s; SJK
still lets additive and blended stages take it. A stage that draws nothing
(`GL_ZERO GL_ONE`, the `clear` shader that servers remap effects onto to hide them)
gives its shader an entry without stages: it used to fall back to alpha blending and
draw its white image as an opaque square.

Effects drawn from the effect atlas (EFX particles, missile trails, muzzle
flashes, beams, impact marks and blob shadows) follow the same remaps and local
overrides, as rd-vanilla `RB_BeginSurface` swaps the shader for every surface.
A remapped atlas entry samples a copy of its target's original stages, with the
destination's time offset subtracted from the effect's shader time; a target
missing from the atlas is loaded first, and one that cannot be loaded leaves the
effect unchanged. This runs after the world applies a remap change, never per
frame. Removing or self-remapping an entry, or `cg_remaps 0`, restores the
original stages; aliases do not chain.

This is shader replacement, not BSP editing. Existing server entity and sub-BSP
presentation use their separate paths. Collision and baked lightmaps are unchanged;
the modern renderer's extracted lamps, GI and sealed BSP shadow boundaries are
not rebuilt by a live remap. Worldspawn remaps use the same replacement path after
the world loads, so that extraction also sees the map's original shaders.
HUD and 2D pictures, saber blades and trails, generated surface sprites,
detached menu previews and mirror/portal classification do not yet follow
arbitrary shader remaps.
Sky remaps do not turn ordinary geometry into new sky portals. Broad community-map
and multi-lightmap registration parity remain to be verified.

Implementation: [compatibility state](../crates/sjk-client/src/shader_remaps.rs),
[map and local remaps](../crates/sjk-viewer/src/world_map_remaps.rs),
[event-time material updates](../crates/sjk-viewer/src/world_shader_remaps.rs),
[replacement compilation](../crates/sjk-viewer/src/world_remap_material.rs),
[effect atlas entries](../crates/sjk-viewer/src/effect_remaps.rs).

## Missing stage images

A scripted shader whose `map`, `clampmap` or `animMap` frame names an image that is not
there is dropped as rd-vanilla drops it
([world_missing_stages.rs](../crates/sjk-viewer/src/world_missing_stages.rs), decided
once when the material is built): `ParseStage` fails on it (`tr_shader.cpp:1218-1243`,
`:1275-1279`; `$lightmap`, `$whiteimage` and `*white` are never missing), parsing stops
there, and `R_FindShader` flags the shader as a default one (`:3460-3463`). World
surfaces then draw the default shader (`tr_bsp.cpp:339`), as do shaders named inside a
model; a skin keeps the stages before the missing one (`tr_skin.cpp:168`), none leaving
the surface undrawn. SJK's materials do not say whether a model's shader came from its
skin, so model surfaces (`LIGHTMAP_NONE`) take the skin's way and lit world surfaces the
default shader. The log prints retail's "R_FindImageFile could not find" once per image
and shader. The default shader, and an implicit shader whose own image is missing, show
retail's default image (`R_CreateDefaultImage`, `tr_image.cpp:1279-1307`): a 16x16 box of
grey 32 with a white border. Until 20/10/2026 the missing stage itself was drawn with a
magenta checker over the rest of the shader.

## External lightmaps

A map compiled with q3map2's `-external` (usually with `-lightmapsize 512`) stores no
lightmap tiles in its BSP: the lightmap lump is empty, and surfaces index pages shipped
beside the map as `maps/<name>/lm_NNNN.tga`, as EJK's `R_FindLightmap`
(`rd-vanilla/tr_shader.cpp`) loads them. `sjk-bsp` keeps one lightmap slot per index,
each with its own size: a 128x128 tile from the file, or an empty slot when the lump is
empty (indices then only need to stay below four per surface record, which bounds the
slots by what was read). [external_lightmaps.rs](../crates/sjk-viewer/src/external_lightmaps.rs)
fills the empty slots from the pk3s right after each BSP is parsed (the world, the
session map and `misc_bsp` instances) and logs how many pages it found; a page that is
missing or does not decode leaves its surfaces on the unlit mid-grey lightmap. Before
27/10/2026 such a map was refused ("Surfaces record 0 references 50, outside 0..0"),
as `baronshed_share` was.

## Light styles

q3map2 bakes every light with a `style` (switchable lights grouped by `targetname`,
styles 32 and up, and the animated styles 1 to 31) into lightmaps of its own: a surface
lists up to four styles, each with a page and coordinates. rd-vanilla draws a copy of the
surface's `$lightmap` stage for each further style, on that style's page, coloured by the
style and added (`FinishShader`, `tr_shader.cpp:2905-2955`; `CGEN_LIGHTMAPSTYLE`), and
sums a vertex-lit surface's colours per style (`ComputeFinalVertexColor`,
`tr_surface.cpp`). Style colours come from `CS_LIGHT_STYLES` (`CG_RunLightStyles`,
`cg_light.c`): no pattern is white, `"a"` dark, `"z"` full, one letter per 50 ms. The
game sets a switchable light's style when it spawns (`SP_light`, `g_misc.c`): on unless
`START_OFF` (spawnflag 4), and its `use` toggles it.

SJK's world draws one lightmap per surface and, until 02/11/2026, drew only style 0, so
a map lit by switchable lights showed almost black: JoF Shuttle's walls had 6 of 255 in
style 0 and about 165 in its `mainlights`, `left` and `right` styles (Kushina's report).
68 of the 193 retail and JoF maps checked have styles (`kor1`, `kor2`, `t1_fatal`,
`t3_bounty`, `t3_stamp`, `jofmemorial`, the JoF missions and others); the other 125 load
byte for byte as before.

Now `sjk-bsp` gives every styled surface a lightmap block of its own holding the sum, on
pages added after the file's, and points the surface's first lightmap there
([light_styles.rs](../crates/sjk-bsp/src/light_styles.rs)). q3map2 gives every style's
block the size of the surface's own, so the vertices give the whole-texel offset from
one to the other; surfaces of one q3map2 block share the new block, which has a
one-texel border repeating its edge. The file's pages are never written: q3map2 shares
one block between surfaces whose light is alike (often one black in style 0), and a sum
written there lit unstyled surfaces sharing it too. A style whose block does not line up
is left out (64 surface styles on `t3_bounty`, 3 on `t3_stamp`), and the load log says
how many ([lightmap_styles.rs](../crates/sjk-viewer/src/lightmap_styles.rs)). The map
loads with every style white; in a game the world composes a block again whenever a
colour it uses changes and uploads its rectangle
([world_light_styles.rs](../crates/sjk-viewer/src/world_light_styles.rs)).

Checked 02/11/2026 over every map of the retail paks and the JoF content: the file's
pages unchanged, no unstyled surface moved, and off-screen world shots of JoF Shuttle's
cabin (baked lighting) went from almost black to lit, the parts without styles unchanged.

Limits: this is baked lighting (`r_dayNight 0`, the SJK and Vanilla presets); real-time
lighting replaces the lightmaps and shows no switchable light, as before. Vertex-lit
surfaces keep the colours they loaded with (all styles on). SJK's server does not spawn
`light` entities yet, so on its own servers every switchable light stays on, where a
retail game starts `START_OFF` lights off. `misc_bsp` instances keep their styles at full
strength. The light grid already reads styles as white
([entity_lighting.rs](../crates/sjk-viewer/src/entity_lighting.rs)).

## Vertex lighting (`r_vertexLight`)

A surface q3map2 lit by vertex (`LIGHTMAP_BY_VERTEX`, lightmap number -3: `-vertex`
builds, `q3map_nolightmap` or `q3map_onlyvertexlighting` shaders, the water and lights
of `oot`) takes rd-vanilla `FinishShader`'s rule for its shader
(`world_stage::material_stages`, `tr_shader.cpp:2878-2905`): the first `map $lightmap`
stage takes the vertex colours (`rgbGen exactVertex`, alpha left as the vertex's); a
leading one is dropped instead and the stage after it, made opaque, takes them. Every
other `$lightmap` stage of such a shader draws the white image, as `ParseStage` gives
it. Before 03/11/2026 those stages drew SJK's grey fallback lightmap, so a vertex-lit
surface lost its bake and showed at half brightness (`oot`'s 9 765 vertex-lit
surfaces, hub report #176).

`r_vertexLight 1` (archived, read when a map loads, as rd-vanilla's latched cvar) lights
the whole world that way: every surface takes `LIGHTMAP_BY_VERTEX` and its vertex
styles (`sjk_bsp::Bsp::light_by_vertex`, after `ShaderForShaderNum`), so the rule above
applies to every shader, and the worldspawn's `vertexremapshader` keys apply with the
`remapshader` ones. Unlike rd-vanilla, dynamic lights stay on, and the lightmap pages
are still loaded (unused). Change it, then load the map again.

## Projected stage coordinates (`tcGen vector`)

`tcGen vector ( sx sy sz ) ( tx ty tz )` gives a stage the vertex position dotted with
two axes as its texture coordinates (rd-vanilla `TCGEN_VECTOR`, `ComputeTexCoords`), so
an image is laid over a whole area in world space: `Forgotten_City_02` drapes its
caustics over every crystal and glass panel this way. Until 27/10/2026 `sjk-shader` read
it as the base coordinates, and those caustics, with their `tcMod scale 0.05`, were
stretched twenty times over each surface. The generator is now
`TextureGenerator::Vector` with `ShaderStage::texture_vectors`; the GPU stage carries the
axes (`texture_vectors`, [geometry_stage.wgsl](../crates/sjk-viewer/src/geometry_stage.wgsl))
and `stage_runtime.wgsl` projects the world position before the tcMods. Only the primary
bundle has axes, so a `tcGen vector` stage is never collapsed into a multitexture pair.
An entity's surface projects from its placed position, where rd-vanilla uses model space.
The ignored world shot `map_spawn_views` renders any map's spawns (`SJK_SHOT_MAP`).

## Video stages (`videoMap`)

A stage with `videoMap <name>` plays a RoQ video on its surface, as rd-vanilla's
`CIN_PlayCinematic(..., CIN_loop | CIN_silent | CIN_shader)` does: looping, without
sound, at the frame rate in the file's header (30 when it says 0). A bare name is
looked up in `video/`, and `.roq` is added to a name without an extension. Retail
uses it for `textures/video/*` (the Raven logo, the briefing screens) and community
maps and servers use it for screens and holo news; JA+ servers remap world
textures to such shaders. Until 08/10/2026 the client ignored the keyword, so a
video stage drew the shader's own name as an image, which does not exist: the
magenta checker Sol found on `mp/ffa1`'s `vjun/hangar_console` on a server
(world note, 07/10/2026).

[cinematic_roq.rs](../crates/sjk-viewer/src/cinematic_roq.rs) decodes RoQ:
codebooks of 2x2 and 4x4 cells and frames coded per 8x8 and 4x4 block as
unchanged, moved from the previous frame, a codebook cell scaled up or split
further, converted with Quake III's full-range BT.601. Audio and the rare JPEG
intra frames are skipped (a JPEG frame keeps the picture). Load gives the stage
the first frame under a texture key of its own (`$video:<path>`);
[world_videos.rs](../crates/sjk-viewer/src/world_videos.rs) keeps that texture as one
updatable layer without mips and, each frame, decodes what the clock asks for (at
most six frames, then it skips ahead) and uploads the newest picture. A missing or
unreadable video shows the missing image (retail's default box), as a missing image does.

Verified: unit tests decode synthetic RoQ files (codebook cells, motion, split
blocks, looping, a truncated frame) and parse the keyword; an off-screen world shot
of `mp/ffa1` with `vjun/hangar_console` remapped locally to `textures/video/raven`
showed the retail Raven logo video playing on the console (08/10/2026). Not
verified: the server remap Sol met (its target is not known), long community
videos, and the cost with many videos on one map (every video is decoded and
uploaded whether or not it is in view, as rd-vanilla runs every cinematic).

## Actor animation failures

Actor animation failures are isolated to the affected mesh. An invalid clip or
pose leaves that actor's last uploaded pose in place, suppresses its animation
audio for the failed frame, and logs once until evaluation succeeds again.
Other actors still evaluate and upload their joint palettes. This includes
custom NPC packs whose animation ranges exceed their skeleton's frame count;
the renderer does not rewrite their files or relax frame bounds checks. The
per-actor work lives in
[actor_pose_steps.rs](../crates/sjk-viewer/src/actor_pose_steps.rs). Which
models load at all, and when a player is drawn as Kyle instead, is described in
[player models](client.md#player-models).

## Load-time texture and light preparation

World installation prepares unique mipmapped texture arrays on up to four CPU
workers before assembling material stages. Prepared batches are limited to
64 MiB of mip pixels; a single larger animated array is processed alone. GPU
uploads and material-cache mutation remain on the installer. Non-mipmapped
filter modes retain their existing upload path. Layer resizing and mip filtering
are unchanged.

The process-local CPU mip cache retains at most 128 MiB of source and mip pixel
storage and 1,024 entries. Keys include target dimensions and retained decoded
image identities, whose upstream cache checks mounted content, so replacing a
texture cannot select a stale chain by filename. Active uploads may temporarily
retain evicted data. These budgets do not describe total process memory. Cache
locking and allocation happen during loading, not during drawing.

With texture compression on (`r_textureCompression`, [GPU memory](#gpu-memory)) the
workers also encode each array's kept levels (those `r_picmip` leaves) into BC1 or BC3
blocks ([texture_compression.rs](../crates/sjk-viewer/src/texture_compression.rs)), with
the CPU encoders of the `block_compression` crate (MIT, a pure Rust port of Intel's ISPC
Texture Compressor kernels; its wgpu compute path is not built), and the mip cache keeps
the blocks, one entry per texture detail, in place of the chains they were encoded from.
The model warm-up worker (`warm_entity_materials`, players' and NPCs' models) encodes the
same way. The render thread only uploads blocks: a texture it prepares itself, which no
worker prepared, stays RGBA8 (`mips::upload` outside a world build). Encoding costs about
21 ms of one core per megapixel for BC1 and BC3 (Sol's PC, release build); mp/ffa3's
textures are 240 MP with their mips at r_picmip 0 with the HD packs installed, 5.1 s of one
core. `texture_compression_load` ([world_shot_texture_compression.rs](../crates/sjk-viewer/src/world_shot_texture_compression.rs))
builds each map twice per process, one process per setting, 25/10/2026 (RTX 5080, 4
workers, two rounds; seconds, texture preparation and the whole world build):

| Map, r_picmip | Preparation off | Preparation on | Build off | Build on |
| --- | --- | --- | --- | --- |
| mp/ffa3, 0 | 0.7 to 1.0 | 2.7 to 2.8 | 3.6 to 4.9 | 6.0 to 6.3 |
| mp/siege_desert, 0 | 1.3 to 1.9 | 4.5 to 4.8 | 4.7 to 6.0 | 8.4 to 9.2 |
| mp/ffa3, 1 | 0.6 to 0.7 | 1.0 to 1.2 | 3.4 to 3.5 | 3.9 to 4.0 |
| mp/siege_desert, 1 | 1.0 to 1.1 | 2.0 to 2.1 | 3.9 to 4.5 | 5.7 to 5.8 |
| mp/ffa3, 2 | 0.6 to 0.8 | 0.7 to 0.9 | 3.4 to 4.2 | 3.9 to 4.5 |
| mp/siege_desert, 2 | 1.2 to 1.4 | 1.2 to 1.7 | 4.7 to 5.9 | 4.7 to 6.5 |

(r_picmip 1: one round.) The second build of a map is no quicker either way: with the HD
packs the cache's budget,
which counts each entry's source images (16 MiB for a 2048-pixel one), holds few of their
textures, blocks or chains.

The decoded images themselves ([decoded_image_cache.rs](../crates/sjk-viewer/src/decoded_image_cache.rs),
one per file name and content, full size whatever `r_picmip`) were never dropped until
25/10/2026: every texture of every map, model and HUD seen in a session stayed in
memory. Once the images held grow 128 MiB past what the last sweep kept, the next image
decoded sweeps out every image only the cache holds; an image a world's shared
textures, the mip cache or a loader still holds stays, so an address used as a key is
never freed and reused, and textures shared across a map change are shared as before.
A model's warm-up (`warm_entity_materials`) returns its decoded images with its
textures (`WarmTextures`), held until the model's materials are appended: held by the
cache alone, any image decoded meanwhile could sweep them (two 4096x4096 RGBA images
are the whole 128 MiB margin), and the append decoded them again on the render thread,
743 ms for `creyonwolf` in Sol's 08/11/2026 log (`model install … materials 736.6`).
`map_changes_vram` prints the bytes held: four maps cycled three times at r_picmip 2
(an install with HD texture packs) held 1750 MiB from the third map
on before, 906 to 1178 MiB after (the current map's own images), with the same video
memory and the same 302 shared textures. Not holding the images in the shared textures
as well (keys by content rather than address) would free only 200 to 350 MiB more of
that: the rest is held by other world structures.

Lamp extraction uses up to four workers for ordinary maps as well as large ones,
preserving emitter output order. Each emitter reuses sorted neighboring patch
IDs while its spatial cell and patch count remain unchanged; live patch moments
and the lowest-eligible-patch selection are preserved. Emission masks use an
exact single-cell reduction for aligned power-of-two blocks and the previous
area-overlap calculation for other dimensions. These changes reduce preparation
work without reducing source count, texture resolution or lighting quality.

A material whose emission mask would cut it into more than 4,096 patches is cooked
again from a coarser mask, one level (four times fewer texels) then two, and last
from its mean, which has no limit (09/10/2026, `lamp_emitters.rs` `COARSER`; the log
says `a material over 4096 patches cooked ...`). The dark gaps of a tiled strip-light
texture keep its patches apart, and each sample tests every patch of the crowded
cells: one `textures/imperial/dpred_striplight` material of `JKLevel1` took 63 s to
make 51,957 patches. `JKLevel1` now loads in 28 s instead of 91 s at 4K on an RTX
5080 (107,118 lamps become 40,682) and the strip lights' room looks the same in a
paired capture; only maps with such materials change (the most one material of
`mp/ffa5` makes is 942, of `JoFTemple` 2,399). Merging patches over a per-material
budget was tried and dropped: merged sources sat behind walls and darkened rooms.

### Pipelines compiled at load

A render pipeline created while a frame is recorded stalls that frame while the
driver compiles its stage program: about half a second per program on a cold RADV
shader cache, by the note in [world_materials.rs](../crates/sjk-viewer/src/world_materials.rs),
far less once the driver's cache holds it. World installation runs on the
`sjk-world-install` thread while the previous world or the loading notice is shown,
so it compiles there what the new map's first frames draw:

- the depth-tested pipeline of every key the map's world and model materials use, the
  no-depth one of model keys and the forced-alpha variants of model stages (log:
  `compiled N pipeline keys`);
- in real-time lighting, where the static world's opaque stages draw through the stage
  table ([stage_table.rs](../crates/sjk-viewer/src/stage_table.rs)), the table's program
  and both lighting variants (scene and live emission) of every stage it holds: the
  static-world variant for materials with static surfaces, the entity variant for
  models (log: `Stage table: compiled N pipelines at load`);
- in real-time lighting, the light pass's depth-priming pipelines and, on a map with a
  lamp cache, its cached receiver, light and direct-lamp pipelines (log: `Light pass:
  depth priming ... compiled at load`).

Until 08/10/2026 the last two groups were compiled in the first frame that drew them,
after the map had gone live: the first frames of a real-time map compiled the table
program and one pipeline per visible key, and more as new surfaces came into view. A
load now takes longer by those compiles instead. Still compiled in a frame, on first
use: glow variants, the lamp cache's bake pipelines (in the first lit frame, with the
bake), the clouds' and weather's programs (first frame with sky or weather), models
loaded mid-match (their entity pipelines as they load, their table pipelines when
first drawn) and keys only a later remap uses. Not measured on a GPU; compare the
`[+ms]` stamps of these log lines and the `frame-budget` maxima after a map load with
`SJK_FRAME_BUDGET=1` (see [status](status.md)).

## Selected controls

| Cvar | Behavior |
| --- | --- |
| `r_dayNight` | Real-time lighting, the Solar Renderer's core (Settings "Real-time lighting", "Sun and sky" until 10/11/2026): a live sun, sky and lamps in place of the baked light, with the lamp cache, GI voxels, light buffer and model sun it installs; implies world sun shadows (the GI probes need the far cascade); default 1, applies after a [graphics reload](#graphics-reload) |
| `r_bouncedLight` | With real-time lighting, light bounces through the GI probes (`gi_probes.rs`); 0 leaves the probes out (the voxel scene stays: lamp visibility needs it); default 1, applies after a graphics reload (10/11/2026) |
| `r_lampShadows` | With real-time lighting, the nearest lamps' shadow maps (`lamp_shadows`); 0 leaves them out; default 1, applies after a graphics reload (10/11/2026) |
| `r_daySky` | Without real-time lighting, the sky still shows `r_dayHour` turning with `r_dayMinutes` (the sky's day tint, as `r_dayClock` does for the PC's hour), and its night darkens the exposure (`day_live::night_tint` of the set hour); default 0, applies after a graphics reload (10/11/2026). World shot `solar_day_sky_without_live_light`: ground brightness 100, 79 and 64 of 255 at 12:00, 19:30 and 23:00 on ffa3 |
| `r_liveLighting` | Lighting tier; default 0. Tier 1 retains world shadow casters between frames; tier 0 also uses available baked indirect light. Applies at map load |
| `r_dayHour` | Solar hour; default 12 (noon, SJK); updated live when day/night resources are installed |
| `r_dayMinutes` | Minutes per simulated day; 0 holds the hour |
| `r_dayClock` | The hour follows the PC's local clock (read once a second, `day_live::PC_CLOCK_CVAR`), over `r_dayHour` and `r_dayMinutes`; default 0 (off); live. Without Sun and sky (`r_dayNight 0`, the SJK and Vanilla presets) the night is a darker exposure instead: factor 1 by day, 0.4 from 20:30 to 05:00, eased through dusk (18:00) and dawn (to 07:00), applied after eye adaptation in the scene resolve, so the HUD keeps its brightness (`day_live::night_tint`, `Params::night`), with eye adaptation brightening less as the night deepens and not at all in full night (`post_exposure::day_share`), so the dark sky does not undo it; and the sky alone follows the PC's hour, the sky stage's day tint installed without sun or shadows (`day::Settings::sky`, `sky_stage::Runtime::follow_day_clock`, written only when the hour changes), from the next map load or graphics reload; with the clock switched off again the sky shows noon, which leaves the authored sky as it is (`day_live::Clock::sky_values`). The tint's pipelines are built inside `gpu_guard::capture` (`day_sky_guard.rs`, hub report #219): one wgpu rejects leaves the authored sky, and `crashes/day-sky.txt`, written before the compile and removed after, makes the start after a session that closed while compiling leave the tint out once, with a console line, then try it again |
| `r_indirectBoost` | Live sky/bounce illumination multiplier, 0–4; default 1. Does not amplify direct lights or add bounce iterations |
| `r_ambientFill` | Live material-lighting floor in dark areas, 0–0.2; default 0.025. Fades as existing illumination increases |
| `r_ambientFillOcclusion` | Fraction of ambient occlusion applied to the readability fill, 0–1; default 1 preserves the previous response. Real indirect lighting keeps full occlusion |
| `r_sceneHdr` | Scene precision: 0 display format, 1 RGBA16F (default); applies after a graphics reload |
| `r_superSample` | Scene supersampling, 1 (default, off), 2 or 3 per axis; applies after a graphics reload. See [Render scale and supersampling](#render-scale-and-supersampling) |
| `r_renderScale` | The world's size in percent of the window per axis, 50 to 100 (default) in steps of 5, stretched to the window; ignored while `r_superSample` is above 1; applies after a graphics reload. See [Render scale and supersampling](#render-scale-and-supersampling) |
| `r_sharpen` | Scene sharpening, 0 (default, off) to 100 %; live, the HUD and menus untouched. See [Sharpening](#sharpening) |
| `r_highDpi` | 1 the window's size (`r_resolution`) and the world count the screen's pixels, 0 its points; default 0 on a Mac, 1 elsewhere. The window resizes at once, the world after a graphics reload. See [Render scale and supersampling](#render-scale-and-supersampling) |
| `r_hdrExposure` | Base exposure multiplier, 0.25–4; live. Eye adaptation adjusts around it; `r_sceneHdr 0` ignores it |
| `r_autoExposure` | Eye adaptation, default 1 (on); 0 holds `r_hdrExposure`. Live; see [eye adaptation](#eye-adaptation) |
| `r_autoExposureMin`, `r_autoExposureMax` | Adaptation range in EV around the base: -2–0 (default -0.5) and 0–2 (default 1). Live |
| `r_autoExposureToBright`, `r_autoExposureToDark` | Seconds to settle when the view gets brighter (default 0.4) or darker (default 2.5); 0 is instant. Live, console only |
| `r_autoExposureKey` | Metered scene luminance shown at the base exposure, 0.03–0.8, default 0.18; higher is brighter. Live, console only |
| `r_dustMotes` | Dust in godrays, 0 (off) to 1 (default, SJK); live; renderer IMAGE tab; requires `r_volumetrics` |
| `r_weather` | The map's rain, snow and mist, 1 (default) or 0; live; renderer WEATHER tab. See [Weather](#weather) |
| `r_weatherDensity` | Weather particle count, 0.25–4; 1 is the original game's, default 2; live; renderer WEATHER tab |
| `r_weatherQuality` | Weather quality, 0 low to 3 ultra, default 2; live; renderer WEATHER tab. See [Weather](#weather) |
| `r_weatherForce` | Weather on every map with sky instead of the map's: 0 (default) the map's, 1 drizzle, 2 rain, 3 storm, 4 snow; live |
| `r_weatherFog` | Ground fog: 0 none, 1 the map's (default), 2 on every map with sky; live |
| `r_clouds` | Volumetric clouds over open sky, 0 (default, under every graphics preset) or 1; live |
| `r_weatherLightning` | Lightning in storms, 0 (default) or 1; live; renderer WEATHER tab. See [Weather](#weather) |
| `r_weatherReflections` | What wet surfaces mirror: 0 (default) by the quality (ultra: everything, else the sky), 1 the sky's colour, 2 the world in puddles (ultra), 3 the world in every wet surface; live; renderer WEATHER tab |
| `r_weatherPuddles` | Standing water on flat ground at ultra, 0 none to 2 flooded, default 1; live; renderer WEATHER tab |
| `r_weatherRainOpacity` | The rain streaks' opacity, 0.25–2, default 1; live; renderer WEATHER tab |
| `r_weatherHaze` | Falling weather's haze and the spray a downpour raises, 0–2, default 1; live; renderer WEATHER tab |
| `r_weatherFogDensity` | The ground fog's thickness, 0.25–3, default 1; live; renderer WEATHER tab |
| `r_weatherSnowCover` | Snow settles on what it falls on, 0 (default) or 1; live; renderer WEATHER tab |
| `r_normalMapping` | Normal maps on lightmapped world surfaces (rend2 convention) and on models whose shader names one (`normalMap`); default 1 (rend2: 0), applies after a graphics reload |
| `r_specularMapping` | Specular, roughness and metalness maps on the same surfaces; default 1 (rend2: 0), applies after a graphics reload |
| `r_parallaxMapping` | Parallax from the height in `_nh`/`normalHeightMap` images; needs `r_normalMapping`; default 1 (at `r_parallaxStrength` 0.1; rend2: 0), applies after a graphics reload |
| `r_parallaxStrength` | Depth of parallax, a multiplier on the stage's `parallaxDepth` (rend2's 0.05 without one), 0 (flat) to 1.575 in steps of 1/40; default 0.1 (Sol's choice: generated height is a guess from paint, and the full depth swam), live, archived; Settings slider 0–1 |
| `r_parallaxNearDistance` | Units from a surface's plane inside which its parallax stops growing on screen as the camera closes in (see [Parallax](#parallax)), 0 (off) to 60 in steps of 4; default 24, live, archived, console only |
| `r_materialMapsDebug` | Material-mapped surfaces only: 1 mapped normal as colour, 2 tint by maps found, 3 normal-map relief, 4 reflection probes alone, 5 without reflection probes, 6 emission maps alone, 7 parallax reach; default 0, live, not archived |
| `r_emissiveMaps` | Emission maps (`<texture>_e`, SJK's) on lightmapped world surfaces; default 1, applies after a graphics reload. See [Emission maps](#emission-maps) |
| `r_emissionStrength` | Brightness of emission maps, 0 (off) to 7.97 in steps of 1/32; default 1, live, archived |
| `r_emissiveGlow` | Dynamic-glow halo around emitting texels (with `r_DynamicGlow 1`); default 1, live, archived |
| `r_emissiveLights` | Emission-mapped surfaces as lamps of real-time lighting, a power multiplier 0 (off) to 4; default 1, applies when a map loads, archived |
| `r_lampLights` | Multiplier, 0 to 8, on the light real-time lighting infers from a fixture's glow overlay (a stage with `glow` or an authored `_glw`/`_glow` image) and from additive vertex-coloured flares (`world_vertex_lamps.rs`); default 4, applies when a map loads, archived. Declared surface lights, self-lit paint and emission maps keep their power |
| `r_normalMapStrength` | Multiplier on the normal maps' relief (their x/y slope, after rend2's `normalScale`), 0–3.98 in steps of 1/64; default 1, live, archived. Material-mapped surfaces and model normal maps; the floor mirrors' lookup keeps the authored relief |
| `r_cubeMapping` | Reflection probes on specular-mapped surfaces (rend2's name and meaning); default 1, needs `r_specularMapping`, applies after a graphics reload |
| `r_cubeMapSize` | Reflection probe face size, a power of two 32–512; default 128, applies after a graphics reload |
| `r_floorReflections` | Polished floors mirror the scene (see [Floor reflections](#floor-reflections)); default 1, live |
| `r_toonModels` | Show the toon shading players chose for their own model (`cg_toonModel`), 1 (default) or 0 every model in stock shading; live, archived. See [Toon models](#toon-models) |
| `r_DynamicGlow` | Halo around `glow` shader stages: 0 off, 1 on (default), 2 saber blades only, 3 the glow alone (debug); live. See [Dynamic glow](#dynamic-glow) |
| `r_dynamicGlowStyle` | Glow blur: 1 EternalJK rd-vulkan's (default), 0 retail rd-vanilla's; live |
| `r_dynamicGlowNoSabers` | 1 keeps saber blades and trails out of the dynamic glow, everything else keeps it; default 0, live, archived |
| `r_dynamicLightShadows` | Walls stop dynamic lights (sabers, bolts, explosions) in real-time lighting, 1 (default, SJK) or 0 (they light through, as retail); live, console only, archived. See [Dynamic lights and walls](#dynamic-lights-and-walls) |

See [day_night.rs](../crates/sjk-viewer/src/day_night.rs),
[sun_shadow_settings.rs](../crates/sjk-viewer/src/sun_shadow_settings.rs) and
[post_hdr.rs](../crates/sjk-viewer/src/post_hdr.rs). A lighting tier alone does not
enable the day/night system. HDR here describes the scene buffer and display
mapping, not a claim of HDR monitor output.

Dust motes ([dust_motes.rs](../crates/sjk-viewer/src/dust_motes.rs)) appear only
in the current main-view godray volume. Each mote samples local sunlit scattering
and subtracts the same wide slice mean used by volumetric clarity. This is a
sample at the mote's depth, not accumulated brightness along the whole screen
ray: a beam farther away cannot light dust in dark air in front of it. Shadowed
cells and uniformly lit air at clarity 1 produce no dust. Beam radiance supplies
both colour and a smooth visibility weight, so the camera's light-grid sample
no longer controls dust brightness. No active volume, no sunlight, or a hidden
volumetric pass means no dust. Secondary views remain excluded.

The shader still generates up to 2048 world-space motes in a wrapping 640-unit
cube, with distance and near-eye fades, depth testing and peak opacity 0.35.
`r_dustMotes` controls density and opacity; it does not enable volumetrics implicitly.
The renderer settings' IMAGE tab calls it **Sunbeam dust**. Shared froxel parameters and depth mapping
keep the sampling coordinates aligned with godrays. The pass reuses existing
volume textures and slice means after their current-frame computation, with no
extra volume, readback or per-frame CPU light-grid sample. Uniform writes use
`FrameQueue`. Dust work is included in the `volumetrics` GPU phase, so compare
that phase with dust off/on at the same view and settings.

Per-map tuning remains unimplemented. Motes behind glass do not inherit its
tint, and their beam boundaries have the resolution of the selected volumetric
grid. Owner visual acceptance and populated-match readability remain open.

Beam-dust verification on 2026-10-02 used Linux / Rust 1.96.1, Ryzen 5 5500
and Radeon RX 9060 XT (RADV). Formatting, locked workspace build/tests and the
release viewer build passed. An external GPU probe executing the production
sampling function checked dark cells, uniform haze, a coloured beam isolated to
a distant depth, out-of-volume coordinates and points behind the eye. Dark/haze
and excluded samples returned zero; the distant beam did not light the nearer
sample. The godray shader's existing calculations are unchanged; only its
parameter ABI and depth mapping moved to a shared include.

External native 1280×720 captures covered dust off/on on `ffa3` and `ffa1`,
volumetrics disabled, and SDR output. With volumetrics disabled, off/on captures
differed by at most 3/255 per channel; the missing volume bypasses the dust draw.
The sampled combined volumetric/dust GPU phase was 0.133 → 0.138 ms on `ffa3`
and 0.136 → 0.140 ms on `ffa1`. These short fixed-view runs used a frozen shader
clock for capture, omit populated-match workload, and establish integration and
indicative GPU cost only. No verification hooks or fixtures are shipped.

These and the other SJK rendering cvars can also be changed in the client's
renderer settings (Settings > Graphics, in Advanced; see
[client.md](client.md#renderer-settings)), with the same ranges and timing.

## Geometry and pipelines added during a match

Models that come in during a match (a player's new model, a hilt, a model a server
names later) join the shared vertex, index and quad buffers
([shared_geometry.rs](../crates/sjk-viewer/src/shared_geometry.rs)). The buffers are
made at map load with room to spare, a quarter more than the map and at least 128k
vertices and 384k indices (about a dozen player models), so a model is written into
that room; only past it are they made again, with room to spare again, the old
contents copied on the GPU and the frame's submit waited for. Before 10/10/2026 they
grew by exactly the new model, so every model copied the whole map's geometry.

A model's materials are compiled when it is installed, with their pipelines; their
glow variants too since 10/10/2026, and the map's at load
(`Runtime::warm_glow`, [world_glow.rs](../crates/sjk-viewer/src/world_glow.rs)),
rather than on the first frame something glows. A player's model (the async loads
of `player_loads.rs`) no longer compiles its pipelines on the render thread: its
materials are appended without them (`append_entity_materials_deferred`),
`Runtime::pipeline_jobs` lists the entity, glow and fog variants they lack, a worker
compiles them (`world_pipeline_jobs.rs`), and the model is put on its player once
`install_pipelines` has filled the slots. Before, a model bringing new pipeline keys
cost 30 to 85 ms in one frame in Sol's 10/10/2026 logs (3 to 6 ms a pipeline); the fog
pipelines are now compiled on first use like the others. Hilts, NPCs and models a
server names later still compile on the render thread.

The menu stage's model (the model picker, the profile's pages, the classic preview
and the Collection's) loads the same way since 05/11/2026
([menu_stage/load.rs](../crates/sjk-viewer/src/menu_stage/load.rs)): a worker reads
the model, skins its first pose and prepares its textures (`warm_entity_materials`),
the render thread uploads one texture a frame, then compiles its materials without
their pipelines (`Runtime::compile_detached_deferred`) and makes its buffers, and a
second worker compiles the pipelines before it is shown. The model on the stage stays
until the new one is ready; one worker reads at a time, so scrolling past models loads
the one the picker stops on. Built in one frame, it cost 100 to 485 ms a model in
Turin's 04/11/2026 log as he scrolled models in a match (`view+menu+audio`), and no
command leaves the client during such a frame ([networking.md](networking.md),
Heimdall's kicks). A windowless client (world shots) takes every step in the frame.

The same worker also builds the model's texture mip chains (`warm_entity_materials`
with `mipmapped`), and the render thread uploads them under their material keys
(`Runtime::upload_prepared_texture`, one a frame) before appending the materials, as the map
loader does (`world_texture_prepare.rs`). Built on the render thread, a chain cost
6.5 ms for a 1024x1024 texture and 25 ms for a 2048x2048 one (release build): 56 and
182 ms hitches for two player models in Sol's log, with their pipelines already off
the thread.

Players' models now go up as models a server names do: one prepared texture a frame,
then the materials and geometry ([player_loads.rs](../crates/sjk-viewer/src/player_loads.rs)).
All of a model's textures in one frame were the `config-refresh` hitches of Slash's
24/10/2026 log on a built-in Radeon: 255 to 425 ms for `zzzpiza/original`, 150 to 166 ms
for `solfox` and `azuki`, each 11 to 19 ms after `client N now wears`. A material list
appended once in a world is reused by every later install of the same list
(`ClientInfoWatch::appended`): Kyle standing in for each model this PC lacks still cost
32 to 55 ms a time with its textures cached. A reused list still asks for what its
pipelines lack (`Runtime::reused_pipeline_jobs`), so a second player on a model whose
pipelines are still compiling for the first waits too rather than compiling them on its
first draw, and textures already up cost no frame. The worker also builds the model's body
copy, which a later frame of its own puts in the body pool drawing with the model's
materials (as one hilt mesh serves every player): a player's first death with a model
built that copy on the render thread (`corpse_actors.rs`), the 35 to 310 ms
`pose+skin+upload` hitches of the same log at kills. Only a body the player will leave is
prepared: bodies wear the player's own model (`GpuState::body_appearance`), so under
`cg_forceModel` or a team model override no copy is pooled and the first death builds it
as before. The pool keeps one copy per appearance worn in the world, geometry only (the
materials are the model's), until the map changes: a player cycling skins adds one each.
An install over 4 ms logs `model install <model>: X ms (materials, geometry for N
vertices, skin)`, a pooled body `body copy <model>: ...`. Not measured since.

## Which graphics card

On a laptop with two graphics cards (an integrated one and an NVIDIA or AMD one) the
driver chooses the card when the program starts. Before 29/10/2026 some such laptops
started `sjk.exe` on the integrated card and the game closed at once, with nothing on
screen (SimpleSmokie, 28/10/2026; Nib found that setting `sjk.exe` to the NVIDIA card in
the NVIDIA Control Panel fixed it). Three things handle it now:

- **The drivers' hints.** `sjk.exe` exports `NvOptimusEnablement` and
  `AmdPowerXpressRequestHighPerformance`, both 1, which NVIDIA Optimus and AMD
  switchable graphics read to start a program on the fast card
  ([gpu_hints.c](../crates/sjk-viewer/src/gpu_hints.c), compiled by the client's
  `build.rs` on Windows only: the client's Rust forbids the unmangled statics; `/INCLUDE`
  keeps the object in the link). A player's own choice in the driver's or Windows'
  settings still wins.
- **Another card when one fails.** The device opens on the adapter wgpu picks for high
  performance; if that fails, every other adapter that can draw to the window is tried,
  discrete before integrated and Vulkan before DX12, each failure written to the log
  ([gpu_context.rs](../crates/sjk-viewer/src/gpu_context.rs), `open_device`).
- **A message when nothing opens.** A start that fails shows a dialog with the error and
  what to try (setting `sjk.exe` to the fast card, updating the driver) before the
  game closes, as well as the crash report (`main.rs`, `show_start_failure`); a release
  build on Windows has no console. `SJK_NO_START_DIALOG` set leaves the dialog out, for
  unattended runs: the release's package smoke check, whose client fails on purpose,
  hung on the macOS runner until it timed out (29/10/2026).

Not verified on a laptop with two cards: the exports are checked in the built
`sjk.exe`'s export table, the fallback and the dialog only by building.

## Graphics reload

The settings read only when the graphics context is made (`r_sceneHdr`, `r_fxaa`,
`r_superSample`, `r_renderScale`, `r_dayNight`, the sun-shadow cvars and `r_volumetrics`, the
material-map controls with `r_cubeMapping`/`r_cubeMapSize`, and texture filtering)
used to need a restart. They now apply with a graphics reload
([graphics_reload.rs](../crates/sjk-viewer/src/graphics_reload.rs)), offered by a
pop-up card and run by `vid_restart` (see
[client.md](client.md#graphics-reload) for the player's side):

- The device, queue, window and surface live for the process. A reload makes a new
  `gpu_context::Context` on them with those settings read again
  (`Context::resampled`, a new context id); the live settings' shared handles
  (colour, dynamic lights, soft particles, dust, weather, exposure, SSAO) carry over.
- The world on show is then built again on the new context, which is what a map
  change already does on the old one: every pipeline, scene target, shadow map,
  lamp set and material is the new world's own, so nothing built for the old
  settings survives. On a server it is a map change to the same map
  (`retain_world_for_reload`): the session waits in the resident world, the map is
  parsed from the session's gamestate after the next snapshot, installed on the
  new context (`GpuState::next_context`), and the shell handed over; no prepared
  world stands in for it and it counts as no map change. A world without a session
  (the menu map) loads its map again in the background while the old world stays
  on show, and takes the shell over when built (`poll_rebuild`); the menu world,
  dropped during a match, is built again the same way when the menus come back
  to it, on the context of the moment (`GpuState::rebuild_menu_world`).
- Whether a reload would change anything is a comparison of what the running
  context holds with what a context made now would read, done when one of those
  cvars changes (their change callbacks call `graphics_reload::notice`); reading
  for that comparison latches nothing (`material_maps::Settings::read`), so the
  material-map and probe notices keep comparing with the running values.
  `vid_restart force` skips that comparison (`graphics_reload::wanted`): the reload
  runs with no change too, on a resampled context that reads the same settings, and
  the console and log name it "forced, no setting changed".

Measured off-screen on an RTX 5080 at 1920x1080 (release build, see
[status](status.md)): duel6's menu world rebuilt in about 3 s; on a local server
ffa3 took 5.6 to 7.2 s from `vid_restart` to play, with the connection kept.

## Render scale and supersampling

[render_scale.rs](../crates/sjk-viewer/src/render_scale.rs) sizes the scene's raster
against the window with one number, the scene's size in percent of the window per axis
(`Context::render_scale`, read when the graphics context is made, so a change applies with
a [graphics reload](#graphics-reload)):

- **Supersampling**, `r_superSample` 2 or 3: 200 or 300 %. The scene is drawn larger and
  box-resolved, every covered texel averaged
  ([render_scale.wgsl](../crates/sjk-viewer/src/render_scale.wgsl)). It falls back a
  factor at a time to what the device's texture size and a 512 MiB budget for four
  colour/depth pairs allow (`supported_bytes`), then to native.
- **Render scale**, `r_renderScale` 50 to 99 (25/10/2026, for integrated GPUs and GTX
  1050-class cards): the scene is drawn smaller and stretched over the window
  ([render_scale_up.wgsl](../crates/sjk-viewer/src/render_scale_up.wgsl)) with a bilinear
  filter, then a light contrast-adaptive sharpen that keeps RCAS's limits (AMD FidelityFX
  Super Resolution 1, MIT): the negative lobe of four stretched samples one scene texel
  away, at a fixed sharpness of 0.5, the largest that pushes no channel past its
  neighbours' darkest or brightest, so edges get crisper without halos. It works on colour
  compressed into [0, 1) (c / (1 + its largest channel), reversed after), so HDR values
  past 1 keep the limits. The size rounds to the nearest pixel (1920x1080 at 75 %:
  1440x810).
- **Supersampling wins**: at `r_superSample` 2 or 3 the render scale is ignored
  (`requested`); the two never combine. The graphics reload card names the side that moved
  ("supersampling" above 100 %, "render scale" below).
- **High DPI**, `r_highDpi` (29/10/2026): at 0, the default on a Mac, the percent counts
  the window's points, not its pixels: `in_points` divides it by the window's scale factor
  when the scene target is installed, after `supported_bytes`, so 100 % on a Retina screen
  (scale 2) draws the world at half its pixels each way and 50 % at a quarter, as EternalJK
  draws it there (SDL without high DPI), while the 2D layer keeps every pixel. Supersampling
  2 becomes native; 3 (150 %) has no whole factor and is native too. A screen without
  scaling, or `r_highDpi 1` (the default elsewhere), keeps the pixels. The factor is read
  from the window on every install, so moving the window to a screen of another scale
  (which resizes it in pixels) installs the scene again at the new one. Glen's Mac lost
  frames once fullscreen worked: a Retina MacBook's screen is about six times the pixels
  of the 1280x720 window he had played in.
- **100 %**, the default, makes no scene target (`GpuState::render_scale` stays `None`):
  the frame is the one from before the setting existed. A unit test pins the default; the
  world shot `render_scale_shots` at 100 differs from main's build as much as two runs of
  main differ from each other.

Everything scene-sized follows the scene, not the window: the main depth, the colour target
the world draws into, the secondary views' targets (floor mirrors, sky and map portals), the
effect layer, the dynamic-glow image, the card outline mask and the real-time light buffer
(half the scene each way at native and below; supersampling divides its factor out, so
there the buffer stays at half the window). The resolve, box or stretch, writes the
window-sized image the shared resolve (`post_aa.rs`) reads, so FXAA, bloom, eye adaptation
and the display pass run at the window's size after it; the effect layer and the glow are
sampled by texture coordinate there. The 2D layer (HUD, menus, text, console) is drawn
after, at the window's size, against a window-sized `SJK HUD depth`, and pointer
coordinates stay the window's. The ground HUD, drawn in the world, rebuilds its occlusion
test from the scene's size (`ground_hud.rs`; until 25/10/2026 it used the window's, wrong
under supersampling too). Screenshots copy the window's image: the stretched world under a
full-size HUD. A window resize, a map change, the menu world and a graphics reload make the
targets again at the new size (`install_render_scale_for`).

Measured 25/10/2026 off-screen on an RTX 5080 (Vulkan, release build), mp/ffa3 from its
start, median GPU time of 19 or 20 sampled main frames (`render_scale_shots` with
`SJK_GPU_PHASES=1`; Experimental leaves out the reflection probes' 0.5 to 0.7 ms capture,
which does not depend on the scale):

| Window, preset | 100 % | 75 % | 50 % |
| --- | --- | --- | --- |
| 1920x1080, Vanilla style | 0.100 ms | 0.099 ms | 0.079 ms |
| 1920x1080, SJK (recommended) | 0.571 ms | 0.456 ms | 0.293 ms |
| 1920x1080, Experimental | 0.928 ms | 0.800 ms | 0.668 ms |
| 3840x2160, SJK (recommended) | 2.061 ms | 1.453 ms | 0.933 ms |
| 3840x2160, Experimental | 3.101 ms | 1.717 ms | 1.284 ms |

The stretch itself costs 0.02 ms at 1920x1080 and 0.06 to 0.08 ms at 3840x2160 on this
card (the `overlays+post+hud` phase). What does not shrink is drawn at a fixed size: the
volumetric froxel grid, shadow maps, probes, and the post-processing at the window's size.
A weak GPU spends a larger share of its frame on the pixels the scale removes, but no weak
GPU has been measured yet.

## Sharpening

`r_sharpen` (04/11/2026, Lex's idea in #ideas; Settings > Graphics > Image) sharpens the
world against the softness FXAA and filtered textures leave. 0, the default, is the stock
image: the resolve (`post_aa.wgsl`) is built without the sharpening taps
(`SHARPEN` override off), so it costs nothing. Above 0 the shared resolve samples the
scene's four neighbours of each pixel once more and applies RCAS's contrast-adaptive
lobe (AMD FidelityFX Super Resolution 1, MIT), as the render scale's stretch does
([above](#render-scale-and-supersampling)): the negative lobe is the largest that pushes no
channel below the neighbours' darkest or above their brightest, so edges get crisper
without dark or bright halos; 100 % is RCAS's full limit (a lobe of at most 0.1875). It
works on the resolve's display values, after HDR's shoulder and the eye adaptation, and
sharpens the pixel FXAA resolved where FXAA ran. The HUD, menus and text are drawn after
the resolve and are never sharpened; a gamma ramp then moves to the display pass, as with
FXAA or bloom. Moving the slider uploads the 16-byte amount
(`post_color.rs`, `set_controls`); only turning it on or off rebuilds the resolve. With
the render scale below 100 % the stretch's own fixed sharpen (0.5) still runs before it.
The world shot `sharpen_shots` renders mp/ffa3 at 0, 50 and 100 %.

## GPU memory

A world's GPU memory belongs to its `GpuState`: map textures (uncompressed
RGBA8 arrays with mips, 0.6 GB on duel6 and 1.1 GB on mp/siege_desert), material
maps, lamp light caches, probes, its screen-size targets (saber glow, FXAA, depth,
glow, at 4K about 0.8 GB) and its own font and UI atlases (about 0.2 GB, the font
atlases 40 MiB of it at 1920x1080). On Windows that memory also shows in the
process's RAM in Task Manager.

- The menu world is dropped when a server world is installed and built again from
  its map once the client is back in the menu off a server
  ([menu_world.rs](../crates/sjk-viewer/src/menu_world.rs)); keeping it parked
  during the match held a second world, about 4.8 GB of the process at 4K on
  duel6. The last server's world stays under the menus for the few seconds it
  takes (duel6: about 2 to 4 s). A join in flight is not "off a server", and a
  rebuild starts at most once every 5 s: in 2026.1010.2 a server map change that
  timed out made the client reconnect, the join cancelled each rebuild and the next
  frame started one again, 13,865 map reads in Sol's log, which stalled the join and
  could exhaust memory.
- Menu pictures (`ui_menuPictures`, [menu_pictures.rs](../crates/sjk-viewer/src/menu_pictures.rs))
  build the menu world from an empty map, so the menus hold almost none of this:
  the backdrop is a 3840x2160 picture per view, taken off screen
  (`world_shot_menu_pictures.rs` `menu_pictures_generate`, with the scene bloom off,
  since the client blooms the picture again), decoded on a worker to the size the
  screen shows and drawn first in the world pass, turned back into scene colour
  under the frame's exposure (as the effect layer's `scene_value`). The camera
  stands at the picture's camera with its field of view, narrowed on screens wider
  than 16:9 so they never see past it, which lines the 3D stage up with the picture.
  A world install that runs out of memory turns the setting on
  ([status](status.md#menu-pictures-and-readable-world-install-failures)).
- Largest texture (`r_maxTextureSize`, SJK, 2048 by default, 0 for none; Settings,
  "Largest texture") leaves off the mip levels of a texture larger than its size, so
  the longest side kept is that size or less, whatever `r_picmip` (`mips::skipped`
  takes the larger of the two). It takes every path `r_picmip` takes, and also the
  `nopicmip` stages and sky boxes. Added 08/11/2026 for community player models with
  4096x4096 textures (`creyonwolf`): at 2048 such a texture is a quarter of its video
  memory and upload. The whole chain is still decoded and built on a worker, so the
  cap does not shorten that part of a load. From the next map, as `r_picmip`.
- Texture detail (`r_picmip`, 0 to 3, as in Jedi Academy,
  [world_texture_mips.rs](../crates/sjk-viewer/src/world_texture_mips.rs)) leaves a
  material texture's largest mip levels off the card: each step halves its sides, a
  quarter of its memory, while its longest side stays at 128 pixels or more (smaller
  textures keep their size). The mip chains are still built whole on the processor;
  `upload_chains` skips levels, so the setting takes effect for textures uploaded
  after it changes (the next map). Since 25/10/2026 a texture takes the same size
  whatever path it goes through (`mips::skipped`): the non-mipmapped path
  (`r_textureMode GL_LINEAR`) and the sky boxes upload the shrunk size through
  `upload_array`, and the material maps skip the same levels of their box-filtered
  chains ([Material maps](#material-maps)); before, all three stayed at full size. As in
  rd-vanilla, the stages of a shader that says `nopicmip` or `nomipmaps` keep their
  images' size: the parser marks each stage parsed after the keyword
  (`ShaderStage::no_picmip`, as `ParseStage` reads `shader.noPicMip`), and its texture
  key ends with `$nopicmip;` (`mips::NO_PICMIP_KEY`), so it never shares an upload with
  the shrunk copy. Sky boxes take the setting even then, as `ParseSkyParms` loads them
  (retail's `textures/siege/desert` sky says `nomipmaps`); at 2 a 1024-pixel box is
  256 pixels a face, 1.5 MiB instead of 24, and the sky looks softer. SJK still builds
  mips for a `nomipmaps` shader. Once menu pictures are on, each world install that runs
  out of memory raises it one step, to 2 at most (`menu_pictures::after_out_of_memory`).
  Not measured on a 3 GB card yet; map textures are about 0.6 GB of duel6 and 1.1 GB of
  siege_desert at full size. The saber glows and cores follow it too
  (`mips::shrunk`, in `saber::material`): eight 2048-pixel glows and their half-float
  column integrals held 300 MiB on every map and in the menu; at 2 they hold 28 MiB.
  The menu on pictures measured 1082 MiB at 1920x1080 and r_picmip 2, 801 MiB after
  (`menu_pictures_vram` in world_shot_vram.rs, 21/10/2026, after Vin's 3 GB card ran
  out in the main menu). The rest of that menu: the game font atlases (128 MiB as
  RGBA, 32 MiB since they are R8, below), the UI icon atlas (56 MiB) and the world's
  shadow, GI and froxel targets (about 200 MiB) a picture menu does not draw.
- Texture compression (`r_textureCompression`, 0 by default, Settings > Graphics > Image;
  [texture_compression.rs](../crates/sjk-viewer/src/texture_compression.rs), 25/10/2026)
  keeps the textures of the world and model materials block-compressed: BC1 (half a byte
  a texel, an eighth of RGBA8) for an array whose texels are all opaque, BC3 (a byte, a
  quarter) for one with a translucent texel, in the sRGB forms that decode as
  `Rgba8UnormSrgb` does. One format per array: an animated stage with one translucent
  frame is BC3 throughout. The array's level zero after `r_picmip` must be whole 4x4
  blocks and at least 128 pixels long (`compress::fits`), or it stays RGBA8; levels under
  4 texels a side are padded to a whole block (edge texels repeated) and written at their
  physical size. Like `r_picmip` it applies to the textures uploaded after it changes (the
  next map), and the shared map textures' key holds it. Not compressed: devices without
  `TEXTURE_COMPRESSION_BC` (GLES; Apple Silicon has the formats), video stages, sky boxes
  and the non-mipmapped path (`upload_array`; a sky's gradients band in BC1), material
  maps (normal maps would need BC5 and the shader rebuilding their third component), and
  everything outside the world materials (UI, menu art, fonts, atlases). The stage
  table's binding array takes BC and RGBA8 views side by side: wgpu checks only the
  sample type (filterable float for both) and the view dimension. Where the encoding
  runs and what it costs at load: [Load-time texture and light
  preparation](#load-time-texture-and-light-preparation).

  `low_end_vram_audit` with `SJK_AUDIT_COMPRESSION=0,1`, 25/10/2026, 1920x1080, release
  build, RTX 5080 on Vulkan, the HD packs installed (MiB in use, off / on):

  | Map, r_picmip | Vanilla style | SJK (recommended) | Experimental |
  | --- | --- | --- | --- |
  | mp/ffa3, 0 | 1594 / 785 | 1622 / 812 | 2021 / 1207 |
  | mp/ffa3, 1 | 660 / 455 | 688 / 483 | 1083 / 878 |
  | mp/ffa3, 2 | 428 / 372 | 456 / 400 | 851 / 795 |
  | mp/siege_desert, 0 | 1757 / 809 | 1785 / 837 | 2405 / 1457 |
  | mp/siege_desert, 1 | 701 / 462 | 728 / 489 | 1349 / 1110 |
  | mp/siege_desert, 2 | 438 / 374 | 465 / 402 | 1086 / 1022 |
  | Menu on pictures, 0 | 999 / 671 | 1027 / 698 | 1284 / 956 |
  | Menu on pictures, 2 | 371 / 346 | 398 / 374 | 656 / 631 |

  The material textures themselves (labels "SJK optional mipmapped material", RGBA8, and
  "SJK compressed mipmapped material"): ffa3 928 MiB in 335 arrays, then 118 MiB in 304
  compressed arrays and 0.5 MiB in the 31 kept (small ones); siege_desert 1091 then 142;
  the menu's models 377 then 48; at r_picmip 1 236 then 31, 276 then 36, 97 then 13; at
  r_picmip 2 66 then 10, 75 then 10, 29 then 5. So the setting saves about 0.8 to
  0.95 GB a world at full texture detail, about 0.2 GB at r_picmip 1 and 25 to 65 MiB at
  r_picmip 2, where the map textures are already small. Full detail compressed (ffa3 at Vanilla, 785 MiB) still
  holds more than r_picmip 1 uncompressed (660 MiB: the saber glows and the sky follow
  r_picmip too, 300 and 24 MiB at 0), but every texture keeps its full size; r_picmip 1
  compressed holds 455.

  Quality, `texture_compression_quality`: frames from the same three cameras of each map
  (its start, looking down, turned aside) at the SJK preset, whose frames repeat exactly
  (two runs off: 0 pixels differ). Off against on: ffa3 41.9 to 44.5 dB over the frame
  (nearly every pixel moves, by at most 29 to 45 of 255), siege_desert 47.8 to 49.8 dB (at
  most 27 to 50); the sky does not change. The frames' middles doubled side by side
  (`tc-<map>-<view>-crop.png` in `target/world-shots`) show no difference to the eye. Each
  texture's level zero against its RGBA8 one (`compress::decode`): ffa3's 287 BC1 arrays
  39.1 dB on average (median 38.0, 5th percentile 32.4), its 17 BC3 ones 39.1;
  siege_desert's 212 BC1 39.5 (median 38.4, 5th percentile 33.1), 22 BC3 39.4. The worst
  are noisy or hard-edged patterns: `models/weapons2/golan_arms/projectilemain.jpg`
  27.3 dB, `textures/rocky_ruins/rancor_floor.jpg` 28.8,
  `textures/factory/cat_floor2.png` (BC3) 29.1, `textures/kejim/kej_lights_glow.jpg`
  30.3; doubled beside the originals (`tc-<map>-worst-<n>.png`) they look the same. No
  BC1 texel came out translucent (flat blocks keep index 0). BC7, measured beside them
  with the crate's alpha "ultra fast" settings, was about 10 dB better on the opaque
  arrays at twice BC1's size and 19 times its encoding time (400 ms of one core per
  megapixel), and 5 to 7 dB better on the translucent ones at BC3's size, 17 times
  slower; hence BC1 and BC3. With the floor at 64 pixels `gfx/effects/chrome.jpg` (64x64,
  an environment map) was the second worst at 28.4 dB; arrays under 128 pixels hold
  almost nothing (0.5 MiB for ffa3's 31) and stay RGBA8.

  Not decided: a default. The numbers argue for it on cards that already get a lower
  texture detail (under 4.5 GiB, `video_memory.rs`): 200 to 240 MiB less at r_picmip 1
  for 0.4 to 1 s more texture preparation per map; and for 6 to 8 GB cards at full
  detail, 0.8 to 0.95 GB less per world, once the 1.8 to 3.5 s of preparation it adds here
  is measured on a 4 to 6 core processor (or the encoding moves to the GPU,
  `block_compression`'s wgpu 30 compute path, or to a disk cache).
- The font atlases are single-channel `R8Unorm` textures
  ([gpu_texture.rs](../crates/sjk-viewer/src/gpu_texture.rs)
  `create_r8_texture_mipmapped`): every code-page atlas holds only glyph coverage (or
  a distance field), and the text colour comes from the vertices, so `text.wgsl` and
  `ground_hud.wgsl` read it from `.r`. Until 25/10/2026 they were white RGBA8 textures
  read through alpha, four times the memory for the same values. At 1920x1080 the five
  game font atlases (SJK Menu, SJK Chat, JetBrains Mono and the SJK UI's two
  families) went from 128 to 32 MiB, the Inter UI atlas from 21.3 to 5.3 MiB and the
  classic HUD atlas from 10.7 to 2.7 MiB: 120 MiB less in every world, the menu's
  included. `low_end_vram_audit` at r_picmip 2 (MiB in use, Vanilla style / SJK /
  Experimental): mp/ffa3 626 / 670 / 1065 before, 506 / 550 / 947 after; the menu on
  pictures 512 / 539 / 797 before, 392 / 419 / 677 after. The Inter atlas grows with
  the display scale (4096x4096 at 200 %: 85 MiB as RGBA, 21 MiB as R8, computed). The
  CPU copies the atlases are built and mipmapped from are one channel too. Text draws
  the same: the text pipeline drew the same glyph quads at six scales (every mip
  level) from the old RGBA upload and from the R8 one, for all eight atlases and a
  distance field, with no pixel different. The emoji atlas stays RGBA (colour
  pictures) and the wide-glyph atlas was already R8.
- Work on the render thread after a world is built runs inside wgpu error scopes
  ([gpu_guard.rs](../crates/sjk-viewer/src/gpu_guard.rs)) where it can fail alone: the
  menu stage's model (the creator's head, torso and legs), which then is not shown and
  drops the textures it uploaded from the material cache, and players' prepared
  textures, which are then not cached. wgpu's default handler panics on any error no
  scope captures, so an out-of-memory elsewhere (a player's geometry joining the
  shared buffers, entity materials) still ends the game: a 3 GB GTX 1060 crashed
  changing legs in the creator (21/10/2026). That panic names what failed
  (`gpu_guard::name_uncaptured`): `wgpu error: Out of Memory (In Device::create_texture,
  label = '...')`, then the causes and the video memory SJK held (allocator report), so a
  crash report says which texture a small card could not take; wgpu's own said only
  `Out of Memory` (Vin's crash in the main menu, 21/10/2026).
- At start SJK reads the card's video memory before the first map
  ([video_memory.rs](../crates/sjk-viewer/src/video_memory.rs), through
  [sjk-gpu-memory](../crates/sjk-gpu-memory/src/lib.rs): DXGI's high-performance
  adapter, its `DedicatedVideoMemory` and budget; nothing off Windows). Under 3.5 GiB
  (an integrated GPU's few hundred MB too) it sets texture detail 2, under 4.5 GiB 1,
  and menu pictures on, each only from its default, once a card (`r_videoMemoryChecked`
  keeps the MiB it chose for). The size and budget go into the log and crash reports.
  Under 3.5 GiB it also lightens the graphics once a card (`r_smallCardPreset`, since
  25/10/2026): Vanilla style when the preset is the untouched SJK (recommended) one,
  other players' blades stock and a 60 FPS menu cap when those are off, then says so
  in a card on the main page with Keep and Back to SJK graphics
  ([sjk-ui.md](sjk-ui.md#small-graphics-cards)). It runs before the graphics context
  is made, so the first frame already draws with them. Vanilla and SJK (recommended)
  differ by only about 45 MiB of video memory; this is for frame time (ambient
  occlusion, HDR and glow at full resolution) and the skins' GPU work.
  The size also sets the lamp light cache's budget
  ([below](#experimentals-lamp-memory)).
- With menu pictures on, the menus' saber shader swatches show their drawings
  (`collection_swatch.rs`) instead of the live blade pictures (`blade_pictures.rs`, a
  832x1664 target of three textures, about 17 MB, and each signature blade's
  pipeline), and that target is freed. Live, a frame draws only the pictures that need
  it: the hovered or chosen ones every frame, the others 30 times a second and six a
  frame at most ([unlockables.md](unlockables.md#shaders-tab)).
- The secondary views make their targets the first frame each is drawn, not with the
  world (`Lazy` in [scene_view_gpu.rs](../crates/sjk-viewer/src/scene_view_gpu.rs),
  `Runtime::provide_targets` in [scene_views.rs](../crates/sjk-viewer/src/scene_views.rs)).
  The floor mirrors' (any polished plane), the sky portal's (any sky shader) and the map
  portal's (any `portal` face) were each a scene-format colour target and a depth target
  at the scene's size, 25 MiB at 1920x1080 with HDR and four times that at 4K, and most
  were never drawn: `r_floorReflections` is 0 at the Vanilla and SJK presets, the sky
  portal draws only once the server sends `CS_SKYBOXORG` for a `misc_skyportal` (12 retail
  maps have one, among them mp/duel6, mp/duel9 and mp/siege_hoth; mp/ffa3 and
  mp/siege_desert have sky shaders and none), and no retail map has a `portal` face
  (community maps do, with their `misc_portal_surface`). A target is kept once made and
  made again at a resize or another scene scale (`r_superSample`, `r_renderScale`); the floor mirrors' is dropped when
  `r_floorReflections` goes to 0. One the card has no room for (captured, `gpu_guard`)
  leaves its view undrawn, logged once, rather than ending the game. Their labels now say
  which view they serve (`SJK floor mirror view`/`depth`, `SJK sky portal view`/`depth`,
  `SJK map portal view`/`depth`). The wide-glyph and emoji atlases (10.7 MiB each) wait
  the same way for the first character beyond the code page or emoji
  ([below](#characters-beyond-the-code-page)). With real-time lighting, a map with
  polished planes still keeps the mirrors' light-buffer images whether mirrors are on or
  not (`configure_light_preservation`).
- The render scale ([above](#render-scale-and-supersampling)) shrinks the scene-sized
  targets with its square: at 50 % of 1920x1080 the main depth, the effect layer's two
  images and the glow image hold 2.3 MiB each instead of 8.4, the render-scale scene adds
  4.7 MiB (HDR), and a window-sized `SJK HUD depth` (8.4 MiB) stays for the 2D layer; the
  window-sized resolve images (`SJK FXAA scene`, 25.3 MiB for two) do not change.
  `low_end_vram_audit` on mp/ffa3 at r_picmip 2 with `SJK_SHOT_CVARS=r_renderScale=50`
  (25/10/2026, MiB in use, 100 % / 50 %): Vanilla style 428 / 421, SJK (recommended)
  456 / 445, Experimental 851 / 805 (its light buffer and lighting images shrink too); the
  menu on pictures 371 / 363, 398 / 387 and 656 / 616. It saves GPU time far more than
  memory; texture detail (`r_picmip`) is the memory setting.
- A map change still builds the next world while the last one lives. With
  `MemoryHints::Performance` (128 to 256 MiB blocks) the blocks the old world
  shared with the new one stayed partly used once it was dropped, about 2 GB of
  reserved but unused memory after a few map changes; the device asks for 4 to
  16 MiB blocks instead ([gpu_context.rs](../crates/sjk-viewer/src/gpu_context.rs)).
  Larger resources get blocks of their own and are freed whole.

Measured off-screen at 4K on an RTX 5080 (release build): duel6's menu world,
then mp/siege_desert, mp/ffa3, mp/siege_desert, mp/ffa3, as map changes on one
device. Before: 11.0 to 12.5 GB private bytes once on a server (the GPU allocator
reserving 5.2 to 5.9 GB). After: 7.2 to 7.4 GB (4.1 to 4.2 GB reserved for 3.3 to
3.4 GB in use). Old worlds were freed in both (the resource counts went back down
each time): it was not a leak. Each world also kept about 1 GB of CPU memory
after loading; most of it was the decoded images, which no longer pile up from map to
map ([Load-time texture and light preparation](#load-time-texture-and-light-preparation)).

The players' "world installer stopped unexpectedly" (18/10/2026: 33 hub crash
reports, `wgpu error: Out of Memory` in `create_texture` on `sjk-world-install`, on
NVIDIA cards of 3 to 8 GB) is that peak, not a leak. The ignored world shot
`map_changes_vram` ([world_shot_vram.rs](../crates/sjk-viewer/src/world_shot_vram.rs))
installs mp/ffa3, mp/ffa5, mp/siege_desert and mp/duel6 three times over at
1920x1080 on one device and prints the allocator's live and reserved bytes after each
change, and the labels that grew. On Sol's PC (release build, a fresh profile):

| Map | In use after | Peak during the build | Reserved at the peak |
| --- | --- | --- | --- |
| mp/duel6 | 1.87 GB | 4.22 GB | 3.9 GB |
| mp/ffa3 | 2.19 GB | 3.85 GB | 4.6 GB |
| mp/ffa5 | 2.54 GB | 4.51 GB | 5.0 GB |
| mp/siege_desert | 2.55 GB | 4.87 GB | 5.1 GB |

Every map holds the same bytes on each cycle (duel6 1869 MiB after the first and the
last), and reserved memory settles back to 2.5 to 3.4 GB. With the desktop and other
programs, a 6 GB card has no room for two worlds of the larger maps. About 0.55 GB of
each world was the same whatever the map and was built again for each one: the saber
glow column integrals (205 MiB), the game font atlas (128 MiB then, RGBA), the saber glow images
(96 MiB), the UI icon atlas (50 MiB), the effect atlas (39 MiB) and the classic menu
art (32 MiB). Map textures that two maps share (retail `textures/` among them) were
uploaded again too.

The worlds alive at once on one context now share those textures
([shared_textures.rs](../crates/sjk-viewer/src/shared_textures.rs)). A world being
built (`new_with_context`, on its thread) takes the copy another world holds, keyed by
what the texture is made from: the saber glow and core images, a game font's atlas, the
effect atlas's pixels, a static menu art piece, and for map textures the decoded images'
identities (one `Arc` per mounted file and content, from
[decoded_image_cache.rs](../crates/sjk-viewer/src/decoded_image_cache.rs)) with the
array's size. A shared map texture skips its mip preparation too. Each entry is freed
with the last world that took it, so a map change never keeps the old map's own
textures. Not shared: the UI icon atlas and animated menu art, which a running world
rewrites; textures a world makes after its build (models and effects that load during a
match, video frames, the non-mipmapped filtering path); material maps, whose converted
images each build makes again, so only a hash of their pixels could key them (and a
server's old map leaves before the next is built, below); and anything across a graphics
reload, whose new context may make textures differently and starts its own cache. A
shared map texture's key holds the levels texture detail leaves off, so a world built
after `r_picmip` changed takes its own size.

| Map | Peak before | Shared assets | Shared map textures too |
| --- | --- | --- | --- |
| mp/ffa3 | 3849 MiB | 3479 MiB | 3119 MiB |
| mp/ffa5 | 4507 MiB | 4133 MiB | 3773 MiB |
| mp/siege_desert | 4870 MiB | 4496 MiB | 4136 MiB |
| mp/duel6 | 4220 MiB | 3846 MiB | 3481 MiB |

Memory in use after each change is unchanged, and the worst reserved peak went from
5.1 to 4.4 GB. The last duel6 of the run, built on 282 shared textures, differs from the
same run with sharing off in 791 of 2,073,600 pixels; two runs with sharing off differ
in 477 (animation timing).

On a server, the old map now leaves the card before the next one is built
([session_transition.rs](../crates/sjk-viewer/src/session_transition.rs), `unload_task`):
when a map change, a switch to another server or a graphics reload starts loading a
map, an empty world (the empty map of the menu pictures) is built on the worker while
the next map's file is read, takes the shell over and drops the old world; the next
map is built beside it. The loading screen covers the world either way. A world
without a map still holds about 1.4 GB at 1920x1080 (screen targets, shadow maps,
the shared textures above), so it saves the old map's own part only, and it costs a
second build: about 3 s in release, 1.4 s of it hidden behind the file read, about
2.2 s more per map change. Demos keep the old way. `map_change_unloaded_vram`
(`world_shot_vram.rs`) measures it, 22/10/2026 at 1920x1080:

| ffa3 alone | ffa3 + siege_desert | Empty world | Empty world + siege_desert |
| --- | --- | --- | --- |
| 2208 MiB | 3719 MiB | 1436 MiB | 3045 MiB |

The world shots above build their client from a fresh profile, whose renderer cvars are
their defaults: the Experimental preset, not the SJK (recommended) one players start on.
`SJK_SHOT_CVARS` (`name=value`, comma separated) sets more cvars on any world shot's
profile, after the shot's own (`world_shot::world_input`).
`low_end_vram_audit` (`world_shot_vram.rs`) builds each map at each preset
(`graphics_quality::preset_cvars`) and texture detail and prints every label, the
allocator's reserved bytes and what DXGI says the process uses (`VideoMemory::usage`);
`card_faces_vram_audit` mounts the SJK packs of `JKA_GAME_DATA/SJK/assets`, which the
other shots never do. Measured 25/10/2026 at 1920x1080 (MiB in use):

| Map, r_picmip 0 / 2 | Vanilla style | SJK (recommended) | Experimental |
| --- | --- | --- | --- |
| mp/ffa3 | 1769 / 603 | 1814 / 648 | 2213 / 1049 |
| mp/siege_desert | 1915 / 596 | 1952 / 632 | 2572 / 1253 |
| Menu on pictures | 1140 / 512 | 1168 / 539 | 1425 / 797 |

The r_picmip 2 column is after the sky boxes followed texture detail (25/10/2026); each
map held 22 MiB more before (its sky box, 24 MiB at any setting), and nothing else
changed: at r_picmip 0 the labels are the same and the Vanilla and SJK frames are
pixel-identical before and after (Experimental's differ as much as two runs of one
build do, from its moving dust and noise). `SJK_AUDIT_GENERATED=1` mounts the player's
generated material pack (`zzz_sjk_materials.pk3`), which no other shot does. With it,
Experimental's normal, specular and emission maps held 884 MiB on ffa3 (218 maps) and
427 MiB on siege_desert (85) whatever the texture detail, SJK's emission maps 9 and
16 MiB; once they followed r_picmip, at 2 they hold 58 and 27 MiB (SJK's about 1), and
the maps hold 1190 MiB instead of 2038 and 1327 instead of 1765 (SJK: 649 instead of
680, 633 instead of 670).

The presets that leave the real-time lighting off differ by about 45 MiB; Experimental
adds its lamp light cache (up to 360 MiB on ffa5), fixture visibility, GI, froxels and
sun shadow maps. DXGI's count is the allocator's reserved bytes plus about 200 MiB, and
stays level over 12 map changes. With the hub's 333 card faces mounted, the menu on
pictures at r_picmip 2 went from 539 to 2316 MiB in use (3235 MiB reserved): every face
was uploaded at 1024 pixels in every world (see [cards.md](cards.md)), which the hub's
crash reports named (`label = 'SJK menu emblem'`, a GTX 1060 3GB in the main menu).
Since then only the faces a screen draws are decoded and uploaded (at most 64 kept): the
same menu holds 539 MiB with the packs mounted, siege_desert 654 MiB instead of 2431,
and the map change's peak is 1105 MiB instead of 2882. Card pictures now carry their own
label, `SJK card art` (medals `SJK medal art`), so `vram` and crash reports no longer put
them under the emblem's.
The table's numbers predate the single-channel font atlases (above), which take about
120 MiB off every cell (measured at r_picmip 2 on mp/ffa3 and the menu on pictures).

With the secondary views' targets and the text atlases made only when drawn (above), the
same audit at r_picmip 2 (25/10/2026, MiB in use, before / after):

| Map, r_picmip 2 | Vanilla style | SJK (recommended) | Experimental |
| --- | --- | --- | --- |
| mp/ffa3 | 626 / 571 | 670 / 598 | 1065 / 993 |
| mp/siege_desert | 618 / 580 | 654 / 608 | 1279 / 1232 |
| Menu on pictures | 512 / 490 | 539 / 518 | 797 / 775 |

ffa3 lost its floor mirror and sky portal targets and both atlases, siege_desert its sky
portal target and the atlases, the menu the atlases; the main view's `SJK depth` is the
only depth target left. The audit's camera sees no polished floor, so at Experimental
(`r_floorReflections 1`) the mirror target is only made once one comes into view.

### Experimental's lamp memory

Players on 4 to 8 GB cards ran out of video memory at map loads on Experimental, where
Vulkan fails outright. Its real-time lighting held three large map-lifetime resources,
cut on 25/10/2026:

- The fixture visibility atlas (`SJK all-fixture static visibility`,
  [lamp_visibility.rs](../crates/sjk-viewer/src/lamp_visibility.rs)) keeps each texel in
  16 bits, two texels to an `R32Uint` texel, instead of an `R32Float` each: 64 MiB at
  most instead of 128, at the same resolution. A texel is not a visibility fraction but
  the ray parameter of the first blocker in its direction, which each filter tap
  compares with the receiver's plane, so it needs precision in world units. The 16 bits
  are steps of 1/65534 of the longest hit the lamp can have (its tracing limit along a
  diagonal), 0.03 units for a lamp of 1000, rounded to the nearest step but never more
  than a quarter unit nearer than traced: the half-unit bias that keeps a receiver from
  shadowing itself loses at most half (`lamp_visibility_common.wgsl`). A miss has its own
  code. `R16Float` would be the plain choice, but WebGPU does not guarantee it as a
  storage format (wgpu would need `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` on every
  device), and half floats were tried: their step grows with the distance (half a unit
  from 512 units), and rounding up by it let measurably more light through on lit walls
  (mp/siege_hoth's mover shots 0.18/255 brighter on average, up to 7/255); rounding up
  by the fixed step still left 0.03/255, rounding to the nearest step is as close as two
  runs of one build. A door tile keeps its movers' covered rectangle in its first
  stored texel (two border texels), a byte per bound. The lowest atlas resolution is now
  2 (tiles stay an even number of texels across); 1 was only reached past two million
  lamps.
- The lamp lists (`SJK lamps`, [lamp_lights.rs](../crates/sjk-viewer/src/lamp_lights.rs))
  hold two 16-bit lamp indices to a word when a map has at most 65,536 lamps (every
  retail map), otherwise one 32-bit index as before (`lamp_list_lamp` in
  `lamp_lights.wgsl`). Refining crowded cells had made mp/ffa5's lists 34.6 million
  entries for 10,023 lamps, 132 of the buffer's 150 MiB. The lists and their order are
  the same, so every lamp sum is too.
- The lamp light cache's budget follows the card (`lamp_cache::budget`, from
  `video_memory::dedicated`): 32 MiB per GiB of the card's own memory, at least 64 MiB,
  and the old fixed 512 MiB from 16 GiB up or when the size is unknown (off Windows).
  It counts the light layers; with material maps the direction layers add half as much
  again. A cache over its budget halves its resolution: mp/ffa5's 45 layers go from
  1024² (360 MiB, 540 with directions) to 512² (90, 135) on a card of 8 GB or less
  (a 256 MiB budget, 192 at 6 GB, 128 at 4 GB), mp/siege_desert's 26 from 208 to 52 MiB
  at 6 GB or less. A coarser cache keeps its accuracy, as before on large maps: the steep
  pass sends texels whose light bends too sharply for interpolation to direct
  evaluation, and so does a surface mapped more than 16 units a cache texel. It costs
  speed instead: on an RTX 5080 at 1280×720 the light pass of the three ffa5 lamp views
  (`world_shot::lamp_light`) took 0.223, 0.170 and 0.425 ms at 1024² and 0.222, 0.170 and
  0.499 ms at 512². Tests take `SJK_VIDEO_MEMORY_MIB` for the card's size.

`low_end_vram_audit` at 1920×1080 and r_picmip 2, Experimental, MiB in use (`main`
7389dc91, then this change as built for cards of each size; Sol's is 16 GB):

| Map | Before | 16 GB | 8 GB | 6 and 4 GB |
| --- | --- | --- | --- | --- |
| mp/ffa5 | 1314 | 1212 | 925 | 925 |
| mp/siege_desert | 1086 | 1023 | 1019 | 851 |

Per label: ffa5's atlas 58.9 → 29.5 MiB and lamps 149.9 → 83.8, siege_desert's atlas
128.7 → 64.3 and lamps 10.9 → 8.0, and the caches as above. The totals move by a few
MiB between runs of one build (wgpu's indirect-draw validation buffers, 1 MiB each); no
other label changed, nor did the Vanilla and SJK presets or the menu on pictures.
Images, compared with two runs of `main` (world shots are not time-deterministic): the
audit frames, `world_shot::lamp_light` on ffa5 and ffa1 and `movers_shadow_lamps` on
mp/siege_hoth (door tiles) differ from `main` as much as `main` from itself on a 16 GB
card (the worst sheet: at most 4/255, 0.017/255 brighter on average, against 3/255 and
0.006 between two `main` runs). With the smaller cache, most views stay within the
cache's tolerance; one wall in siege_hoth's sheet 59 changes by up to 56/255, because at
512² it is lit directly: there the 1024² cache had been off by up to 53/255 (30,355
pixels over 8/255 against `SJK_LAMP_CACHE=0`, 3,036 at 512²).

Looked at and left: the two froxel volumes (23.6 MiB each, RGBA16F, alpha unused; the
RGB-only `Rg11b10Ufloat` is not a storage format WebGPU guarantees and has a 6-bit
mantissa for the integrated sums), the GI probe depth moments (23 to 26 MiB, the mean
and mean square of each direction's distance in two floats; half floats overflow past
256 units squared and lose the variance to cancellation) and the six 16 MiB sun shadow
targets (2048² `Depth32Float`; 16 bits over the far cascade's map-wide fit would show
acne).

## Eye adaptation

SJK adapts the exposure to what the camera sees, as eyes do: in a dark area the
view brightens over a few seconds, and stepping or looking into bright light
darkens it a little, quickly, before it settles. This is a deliberate choice
with a cost: a fixed exposure keeps camera contents from changing how visible
another player is, whereas brightening a dark room shows a player in its
shadows sooner, and darkening after a bright sky hides one for a moment. SJK therefore keeps the range small by default
(-0.5 to +1 EV around `r_hdrExposure`), puts the switch on the Renderer page
(IMAGE tab, "Eye adaptation"), and `r_autoExposure 0` restores the fixed
exposure exactly.

How it works ([post_exposure.rs](../crates/sjk-viewer/src/post_exposure.rs),
[post_exposure.wgsl](../crates/sjk-viewer/src/post_exposure.wgsl)):

- After the main view's resolve, a compute pass meters the scene target in
  8×8-pixel cells (four bilinear taps each) into a 64-bin histogram of log2
  luminance, weighting the centre of the screen four times the edges. It reads
  the scene before exposure, the legacy effect layer, HUD and text, so sabers,
  particles and the interface never drive it, and its result cannot feed back.
- A one-thread pass averages the histogram between the 30th and 97th
  percentiles (dark corners and small bright sources such as lamps and sun
  glints are ignored), takes the target `log2(r_autoExposureKey) − mean`,
  clamps it to the range and moves the current value towards it in EV:
  `e += (target − e)(1 − exp(−dt/τ))`, with τ `r_autoExposureToBright` when the
  view got brighter and `r_autoExposureToDark` otherwise. Near-black cells are
  left out; a frame with almost nothing else (a cleared or black frame) changes
  nothing.
- The result, `r_hdrExposure × 2^e`, stays in a 16-byte GPU buffer that the
  resolve and the effect layer's encode and write-back read the next frame.
  There is no readback, and a frame records two fixed compute dispatches and
  one 32-byte parameter upload through `FrameQueue`; everything is created with
  the scene targets.

The adaptation snaps to its target instead of easing when the view cuts: on a
new map or scene targets (map load, `vid_restart`, resize, toggling bloom or the
tone curve), when the followed player changes, on respawn and teleport
(`EF_TELEPORT_BIT`), and on entering spectating or intermission. It holds still
while the world is hidden or replaced (classic menus, loading, the hyperspace
flash) and while the Renderer settings page is open, so `r_hdrExposure`
comparisons there are not blurred. Without a match, demo or explored map (the
menu world) it holds plain `r_hdrExposure`. Demos have no seek, so starting one
is a map load. Black frames never count, so a first cleared frame cannot drive
the exposure to the maximum.

With `r_sceneHdr 0` the scene is 8-bit and already clipped at white, so
darkening would only grey its highlights: the minimum is 0 EV (it only
brightens), `r_hdrExposure` still does not apply, and brightening uses a
shoulder that keeps white at white (identity up to an exposed value of 0.5,
then a rational curve). Darkening needs `r_sceneHdr 1`, the default.

Menus, HUD, text, levelshots and the display gamma pass are never exposed.
Screenshots and captures show the adapted image, as the screen does.

Verification (2026-10-04, Windows 11, RTX 5080, Vulkan and DX12, headless
compute/render probes outside the repository; no client was run): with exposure
1 the new resolve (HDR and 8-bit, FXAA on and off) and the effect layer's encode
and write-back (all three encodings) matched the previous programs byte for
byte, as did HDR at exposure 1.5 against the old fixed constant. Metering a
uniform mid-grey (0.18) scene gave a mean of log2 0.18 within one bin (target
+0.002 EV); two stops darker clamped to +1 EV; a black frame left the state and
its pending snap unchanged; a 2% patch at 50× brightness did not move the
target; an 8-bit scene decoded to the same mean; the smoothing followed the
expected exponentials. The 8-bit curve was monotonic, identity at 1 and
inverted to within 1.4e-6. Timestamps for both passes on a synthetic scene were
about 0.01 ms at 1920×1080 and 0.02–0.03 ms at 3840×2160. Unit tests cover the
cvars and their ranges, the smoothing step, the 8-bit clamp, the snap and freeze
decisions, and translate the three programs to SPIR-V and HLSL. How it looks in
play, the default key on real maps and its cost inside a full frame are
unverified; the key was chosen so a typical sunlit outdoor view (`mp/ffa3`)
stays near 0 EV and needs in-game tuning.

## Volumetric silhouette coverage

The volumetric injection pass averages lighting over visible air samples within
each cell. Samples behind the scene surface are excluded from both the sum and
the count; visible samples in shadow still count as zero illumination. Counting
hidden samples as darkness reduced the background's light beside a foreground
player or geometry edge. The final composite still integrates only to the
pixel's surface depth, and wholly hidden cells remain empty. Grid sizes, sample
counts, shadow filtering and clarity settings are unchanged.

External Linux/RADV release captures on RX 9060 XT, based on `a993436` plus local
changes, reproduced the fringe on `mp/ffa5`. Player-present/absent and volume-on/off
comparisons isolated a 6-level RGB loss just outside the helmet; the correction
reduced it to zero. A nearby outside strip's mean negative difference fell from
4.415 to 0.001 levels. The camera matches the owner's mark, but the preview actor
placement is approximate because marks do not retain actor state.

At 1280×720, 600-frame throughput measurements were 1.116 ms before and 1.112 ms
after; at 3840×2160 they were 3.375 and 3.380 ms. Increasing the old grid to quality
3 instead measured 1.471 ms at 720p and retained a narrower fringe. These are
single-actor scene measurements with another client open, not GPU-isolated or
populated-match performance certification. Paired captures also cover `mp/ffa3`,
the marked `T2_Rancor` interior and 24 camera turns on FFA5 at 4K. The correction
does not remove all finite-grid undersampling; broader maps, motion and owner
playtesting remain required.

## Inferring fixture light from legacy materials

Prefer compiler-authored `q3map_surfacelight` power and `q3map_lightimage` masks.
Where that information is absent, fixed, non-sky, undeformed materials can supply
emission through view-independent additive stages. The source classifier uses the
shader's blend, colour generation, glow flag and actual texture mask; a texture
name containing "light" is not sufficient. Environment reflections, entity-coloured
stages, unresolved images and ordinary opaque paint do not become inferred lamps.

Explicit `glow` stages on these undeclared fixtures now use a white-radiance
fallback of 16, compared with 4 for unmarked additive decoration. This distinguishes
an authored luminous fixture from a generic additive effect. It is an artistic
fallback, not a recovered physical intensity: the flag carries no wattage. Declared
surface lights and effect-sprite extraction retain their previous gains. Both mean
energy and the spatial emission mask use the same gain; black housing and gaps stay
dark, and alpha covers still attenuate the source. Inference happens during loading,
without a new per-frame classification pass or texture-name exceptions.

The owner-marked `mp/ffa_mtd` view exposed insufficient inferred power rather than
unrecognized ceiling fixtures. Its `textures/massassi/light7` shader has an explicit
additive glow but no compiler surface-light power. The 12×12-unit faces generated
about 298 units of patch power with a 277-unit influence radius: their contribution
was already fading strongly around floor distance. The new inference gives the
sampled patch about 1,191 power and a 550-unit radius. Disabling candidate importance
filtering or static shadows in an external diagnostic barely changed that view;
neither diagnostic change is included in production.

Verification (2026-10-03, local change based on `8f692ac`): workspace build,
all nine existing Cargo tests and doc tests, formatting, and the optimized owner
build passed. Eight external source-policy cases checked explicit glow versus
unmarked/declared fallback, shared mask energy, dark texels and entity/environment/
opaque/missing-image exclusions. Linux/RADV release captures at 1280×720 covered
the marked custom-map view and stock `ffa3`/`ffa1`; the custom view retained its
dark atmosphere with more local illumination. In that map the inferred source
count grew from 27,465 to 27,576, grid references from 5.79 to 5.96 million, and
the static visibility atlas from 121.5 to 122.2 MiB. Settled sampled GPU totals
were approximately 0.582→0.568 ms there, 0.506→0.505 ms on `ffa3`, and
0.742→0.715 ms on `ffa1`. These sparse samples with a separate owner client still
running show no observed regression in those views; they do not establish a
speedup or certify populated-match performance. Windows runtime and wider
custom-map coverage remain unverified. No test harness or assets were added to Git.

The fallback cannot recover compiler-stripped point lights or confidently identify
a lamp drawn only into ordinary diffuse paint. Improving those cases needs additional
authored evidence or a separately validated inference method. No claims of complete
custom-map light recovery are made.

### Static model fixtures

`misc_model_static` lamps now enter the same source collection as BSP fixtures.
Previously their model materials could display glowing bulbs while none of their
placed triangles supplied room lighting. Resolve each model once at map load,
reuse the existing static-placement parser, and integrate the luminous surfaces
with their placed position, rotation, inverse-transpose normal and scale. Each
placement keeps its own emitting area. Moving/network entities, pickups and
actors do not become permanent sources. The resulting lights use the existing
lamp grid, static visibility, receiver cache and probe-bounce path; there is no
new per-frame discovery or model loading.

Unlit opaque replacements can hide earlier shader stages. Source inference now
examines the surviving stage suffix, so an opaque luminous face following an
additive stage is eligible. Bright texels qualify an otherwise unlit fixture even
when dark housing lowers the whole-image average; power still uses the complete
area-weighted mask. Ordinary diffuse/vertex/entity-lit paint remains excluded.
So is paint with a visible `alphaGen lightingSpecular` stage (09/10/2026): a shine
computed from the light reaching the surface means the author meant it lit, even when
an opaque stage hides its lightmap stage by mistake and it shows fullbright.
`JoFTemple`'s two statues were such paint; their dense meshes made 44,065 of the
map's 58,124 lamps, and leaving them out took its load at 4K on an RTX 5080 from
23 s to 16 s, with the frame unchanged (about 12 ms). They still show fullbright;
they no longer light the room around them.
A model surface's luminous pieces share a range calculated from their combined
power, so texture subdivision does not give every small piece a prematurely
short reach. Their summed energy is unchanged. This shared range is per placed
model surface; it does not combine all instances across the map or change BSP
surface-light range policy.

Verification (2026-10-03, local change based on `8f692ac`, on top of the inferred
glow/readability preview): release Linux/RADV captures at 1280×720 covered both
owner marks on `t2_rancor` and stock `mp/ffa1`/`mp/ffa3`. Static models were also
rendered in the external captures. Rancor gained 3,485 model patches (4,340 total
lights became 7,825); grid references rose from 755,662 to 1,289,445. Both marked
rooms gained localized illumination and remained dark. The sampled `ffa1` image
changed by at most 1/255, and `ffa3` by at most 2/255. External policy checks
covered hidden shader stages, unlit versus diffuse paint, bright inserts in dark
masks, unchanged integrated flux and combined fixture reach. Workspace build,
Cargo tests, formatting and the optimized owner build passed. A paired Rancor
capture with day/night disabled was byte-identical before and after.

Settled GPU medians, from 11–12 samples per view, were 0.893→1.036 ms and
0.822→0.839 ms at the Rancor marks, 0.722→0.721 ms at `ffa1`, and
0.521→0.527 ms at `ffa3`. The additional Rancor sources have a measurable cost.
A separate owner game was running; these are fixed-view GPU samples, not
exclusive-device total-frame or populated-match certification. Windows runtime
and other model-heavy maps remain unverified.

This repairs missing source participation, not all dark-room readability.
`t2_rancor` also requested baked ambient light (`ambient 20`); that background
illumination is not reconstructed by this change. Its inferred fixtures still
lack authored physical power, unlike several strong surface lights in `ffa1`.
Prop housing has not been added to the immutable world occluder geometry, and
moving/triggered model lamps remain outside this static-source path. No global
exposure, sun, ambient-fill or authored-light-power change is part of this step.

### GI traversal correction

The voxel GI tracer now measures a positive forward distance to the next cell
boundary for either ray direction. Previously negative-direction rays used a
signed coordinate difference divided by an absolute direction, producing hits
behind the ray origin and incorrect surface/depth samples. Traversal also checks
the maximum range before accepting an occupied cell, preventing hits beyond the
requested distance. This changes the probe tracer, not the separate triangle
visibility tracer used for direct lamp shadows.

Verification (2026-10-03, local change based on `8f692ac`, including the preceding
fixture preview): an external GPU harness compared the actual WGSL against an
independent double-precision ray/AABB reference. All 8,302 cases pass, covering
both axis directions, all diagonal octants, random oblique hits/misses, and ranges
below/at/above intersections. The preceding shader failed 3,174 of these cases.
Workspace build, all nine existing Cargo tests and doc tests, formatting, and an
optimized owner build passed. No diagnostic or test code was added to production.

Fixed-camera release GPU comparisons used the same 1280×720 settings, hour,
exposure, lamp data and indirect gain before and after. Both marked `t2_rancor`
images were byte-identical. The `ffa1` comparison changed by at most 1/255;
`ffa3` had a small local difference (mean absolute channel difference 0.0105/255,
maximum 10/255). Thus this is a verified correctness fix, not evidence that
Rancor's low visibility is solved. GPU readback confirms its live probes do update
and contain nonzero lighting; room-wide receiver coverage and effective bounce
strength still need investigation before deciding on another brightness change.
Rancor's 208-unit probe spacing aligns with its 16-unit voxel grid, so the old
sign error vanishes at those ray origins. The corrected range check still changes
some distant probe values. At the earlier `mp/ffa_mtd` mark (624-unit probe spacing,
64-unit voxels), the correction does change local illumination: mean absolute RGB
difference 0.0358/255, maximum 38/255. It does not broadly brighten that room.
Paired readback on Rancor changed the L0 coefficient of 11,026 of 29,541 live
probes above a 1e-7 threshold, while the closest probes to both marks remained
essentially unchanged.

Settled GPU medians (11–13 samples per view) were 1.038→1.038 ms and
0.825→0.825 ms at the Rancor marks, 0.742→0.733 ms at `ffa1`, and
0.512→0.533 ms at `ffa3`, and 0.639→0.649 ms at `mp/ffa_mtd`. A separate owner client was running. These sparse
fixed-view GPU results do not establish a speedup, exclusive-device performance,
31-player frame times or Windows behavior. The owner approved publication of the combined lighting/transition preview;
this does not close the remaining dark-interior investigation.

## Movers in lamp shadows

Doors, lifts, `func_static` brushes and the other inline movers block the map's lamps
in real-time lighting, at the pose the client draws them with: a closed door keeps a
lamp's light in its room, an open one lets it through
([mover_occlusion.rs](../crates/sjk-viewer/src/mover_occlusion.rs)). Lamp shadows
otherwise come from a trace of the static world only, done once when the map loads.

- **Which lamps.** At load each mover gets the world box it can move through, from its
  entity's spawn keys: `func_door` and `func_button` from their spawn bounds to where
  they slide (`SP_func_door`'s `G_SetMovedir` and `lip`), `func_plat` down by its
  `height`, brushes that never move (`func_static`, `func_breakable`, `func_usable`,
  `func_glass`, `func_wall`) their spawn bounds. Other movers (trains, rotating and
  bobbing ones) get their bounds grown on every side by their largest extent. A lamp
  whose reach touches that box, and that sees into it past the static world (a CPU ray
  from the lamp gets through to one of 27 points spread through the box), gets a door
  tile. Movers that never stop (`func_bobbing`, `func_rotating`, `func_pendulum`) get
  none and block no lamp (09/10/2026): each of their pose changes traced the tiles
  again and re-baked the cache around them every frame for good. On `JoFTemple` (4K,
  RTX 5080) its 15 bobbing platforms cost about 4.7 ms a frame, mostly the floor
  mirrors' lighting (3.4 → 0.7 ms); the frame went from 12.1 to 7.4 ms with them
  moving. They still cast sun shadows.
- **Door tiles.** They sit after the lamps' own tiles in the static visibility atlas,
  at the same resolution: only as many as fit without lowering it (the most powerful
  lamps first when there are more). Each holds the lamp's distances to the movers alone,
  traced on the GPU against the movers near that lamp at their current pose; a
  receiver's lamp visibility is the world tile's times the door tile's. The trace also
  records the pixel rectangle the movers cover, in two border texels the filter never
  reads (the tile's first stored texel, a byte per bound, since the atlas went to 16 bits
  a texel: [Experimental's lamp memory](#experimentals-lamp-memory)); a receiver whose
  filter footprint lies outside it skips the door tile.
- **Poses.** Movers in the snapshot block light where they are drawn; one hidden or
  broken (not drawn) blocks nothing. A mover no snapshot has shown yet stands at its
  baseline's spawn pose; one with neither blocks nothing. A mover missing from a
  snapshot that would hold it blocks nothing either: the server removed it (a broken
  `func_breakable`) or hid it with `SVF_NOCLIENT` (`func_usable` or `func_wall` switched
  off), which no snapshot shows as hidden. "Would hold it" is the server's test, from
  the player's eye: one of the clusters of the mover's reach box (the first 16 it touches)
  is in the eye's PVS and one of its areas is open in the snapshot's area mask. A mover
  left out because its place is out of view keeps its last pose, and `EF_PERMANENT`
  movers (never sent) keep theirs. A pose change queues the mover's door tiles again;
  about 2^19 rays are traced per frame (31 tiles at 128², 120 at 64²), oldest first, so
  a moving lift spreads its cost and its final pose is always traced. The pose kept is
  the one last traced, until the mover is a hundredth of a unit or about a hundredth of
  a degree from it, so slow motion adds up to a trace and a still mover is never traced
  again.
- **Lamp cache.** The static lamp cache is baked with the door tiles as they are. When
  tiles are traced again, the cache texels their movers can shadow are baked again: at
  load, each door tile finds the cache surfaces in its lamp's reach that lie behind its
  movers (the cone from the lamp around the mover's box) and lists the 64×64-texel
  chunks under them (`mover_occlusion::cache_regions`). A traced tile marks its chunks
  stale; a refresh takes at most 16 of them (`CHUNKS_PER_FRAME`, 65,536 texels) round
  the map from where the last one stopped, so a lift that keeps moving cannot starve
  the rest, joins neighbours along a row and, per layer, clears and lights only those
  rectangles, drawing only the surfaces under them, with the steep pass over them grown
  by one texel and the rim pass over them alone: the rest of the layer is untouched.
  While tiles keep coming the cache follows every 100 ms, every frame once the queue is
  empty. A busy map's door shadows then lag a few frames behind its movers instead of
  costing the frame what they touched. Until 09/11/2026 a refresh baked one rectangle
  round everything stale on each of four layers and ran the steep and rim passes over
  every 2048² layer it touched: 16 ms of a 4K frame on `T2_Rancor` (7,977 lamps, 61
  movers). The tests `a_refresh_never_bakes_more_than_its_chunks_a_frame` and
  `a_mover_that_keeps_moving_cannot_starve_the_rest_of_the_map` hold the budget. The
  rule for every renderer system that updates part of its data each frame (door tiles,
  GI probes, this cache): bound it by work done (rays, probes, texels), never by a
  count of items whose size depends on the map.
- **Sun.** The view and close cascades already drew movers every frame. The far
  cascade, redrawn only when the sun turns, now also redraws when movers have moved,
  at most every 250 ms, and once more after they stop. Every shadow-casting mover is
  tracked for it, whether or not a lamp gave it a door tile, so it follows movers on
  maps without lamps too. Volumetric light and GI probes read it.
- GI probes sample lamp visibility through the same atlas, so their lamp bounce sees
  the movers too, as the probes refresh (a few hundred a frame).

`SJK_MOVER_OCCLUSION=0` leaves movers out of lamp shadows (no door tiles), for
same-binary comparisons; the far cascade still follows them.
A lightmapped surface goes into the lamp cache when its lightmap is at most four times
coarser than the cache's 4-unit spacing (`lamp_cache.rs` `COARSEST`, three before
09/10/2026); coarser ones are lit lamp by lamp every frame. `JoFTemple`'s great hall
floor is 3.6 times coarser: lit directly it took 4.2 ms of a 7.5 ms frame at 4K on an
RTX 5080 (Sol's in-game run with every real-time term off still measured a 5.7 ms
light pass), cached the frame is 3.6 ms, and the spawn view differs by at most 8/255
(0.45 % of pixels by more than 4). `mp/ffa5`, `mp/ffa3` and `JKLevel1` were unchanged
(3.1, 3.2 and 2.5 ms) and the cache kept its 519 MiB on `JoFTemple`. The cache's budget
follows the card's video memory since 25/10/2026: on a card under 16 GB a large map's
cache can be a resolution coarser, with more surfaces lit directly
([Experimental's lamp memory](#experimentals-lamp-memory)).

The ignored world shot `world_shot::movers::movers_shadow_lamps` renders the movers with
the most door lamps closed, open and with every mover hidden, from beside a lamp, and
with `SJK_MOVER_TIMING=1` holds views for GPU timing (see its rustdoc).

Cost, measured on `mp/siege_hoth` (482 door tiles of 1,481 lamps, 68 movers) in a
release build at 960×540, Linux, Radeon RX 9060 XT: the visibility atlas grew from 95.5
to 127.6 MiB; finding the lamps and the cache regions took about 100 ms at load. Median
light pass at the start camera 0.290 → 0.302 ms; beside a closed hangar door 0.650 →
0.853 ms, the door's own surfaces being lit directly; the tracer took 0.10 ms a frame
while that door was shown and hidden every frame, 0.003 ms at rest.

Memory beyond the atlas: when door tiles re-bake the lamp cache, the bake's scratch
targets stay after the first bake (two Depth32 targets, the RGBA16F rim target and, with
light directions, the RGB10A2 rim directions): 20 bytes per texel of the cache's layer
resolution, 20 MiB at 1024², 80 MiB at 2048² (16 bytes a texel without directions). The
`Lamp light cache: bake encoded` log line names the amount. Maps without door tiles, and
`SJK_MOVER_OCCLUSION=0`, free them after the first bake as before. The `mp/siege_hoth`
figure above does not include them; it was not measured.

Not covered: a train or lift that travels outside its box does not shadow lamps it
reaches there; alpha-tested and blended mover faces let light through, like those of
the static world; movers do not stop dynamic lights (the static world does, see
[Dynamic lights and walls](#dynamic-lights-and-walls)); baked lightmaps
(`r_dayNight 0`) are unchanged.

## Dynamic lights and walls

Saber glow, bolts, explosions, Force effects and the other dynamic lights (the frame's
point-light list, at most 32) stop at the static world in real-time lighting: a saber by a
wall no longer lights the floor of the room behind it, nor the corridor round a corner
([dynamic_light_shadows.rs](../crates/sjk-viewer/src/dynamic_light_shadows.rs)).
Before, a surface took a dynamic light by distance and facing alone
([point_lights.wgsl](../crates/sjk-viewer/src/point_lights.wgsl)), so every floor, ceiling
and wall within its radius that faced it was lit, whatever stood between.

- **Tiles.** Each frame, before any view draws, every light gets a 28×28 octahedral tile
  (the lamps' mapping): per direction, how far the light gets before the static triangles
  of the lamp visibility stop it, in 255ths of its reach (radius plus 8 units), and the
  normal of what stops it. Glass, grates and sky let it through, as for the lamps. The GPU
  traces them (784 rays a light, 25,088 for a full list, one ray per invocation) and
  copies them into the
  point-light uniform block after the CPU's part (58,208 bytes in all), so the programs
  read them without a new binding. A frame that traces none reads every light unshadowed.
- **Receivers.** World surfaces, their material-map highlights and models lit per pixel
  (`r_modelPixelLight`) take four bilinear taps of the light's tile. A tap is clear when
  its ray gets as far as the receiver, or past the receiving plane (the lamps' test, so
  grazing floors never shadow themselves). Otherwise what stopped it decides: a receiver
  in front of that face (the other face of a crease, the next step of a stair) keeps the
  light, one behind it (a wall between) loses it. Taps that find a wall between also
  discount those that only passed beside a face, so a floor running on under a wall does
  not light the room behind it. The geometric normal is used, never a normal map's.
- **Edges.** Shadows are soft over about a texel (6.4°); in the tests their edge reaches a
  tenth of the distance into the shadow and a fifth into the light. Seen past a wall's
  foot almost edge-on, a large light (an explosion 90 units from the wall) leaves a trace
  within a texel of the foot behind it: under 3% of its strength, 0.1% of what used to
  come through.

`r_dynamicLightShadows 0` lights through walls again for comparisons. `SJK_GPU_PHASES`
times the trace as `dlight-shadows`. Unit tests follow the shaders' arithmetic in Rust on
small scenes (a wall, a corner, stairs, a light just inside the face it hit) and validate
the programs.

Timed on 09/11/2026 off screen at 3840×2160 on mp/ffa3 under the renderer's defaults
(`world_shot_dynamic_light_cost.rs`, `dynamic_light_shadow_cost`), the trace costs the same
for 2 lights as for 32: it is bound by how long one ray takes, not by how many there are.
Tracing two texels in a row per invocation took 0.38-0.39 ms; one texel each, packed in
pairs through workgroup memory, takes 0.23 ms, with the same picture. Skipping lights the
camera cannot see was tried and dropped: on this card it saved nothing, and mirrors,
portals and probe captures would each have needed every light anyway. Depth writes left
off for draws the depth prime already covers (to let the card reject hidden pixels
before shading) saved 0.012 of `opaque`'s 0.53 ms, too little for a pipeline variant per
path; `frame_section_cost` in the same file times every section to compare two builds.

Not covered: baked lightmaps (`r_dayNight 0`) build no triangles, so dynamic lights pass
walls there as in the original game; movers are not in the triangles, so a closed door lets
dynamic light through; models lit once each (`r_modelPixelLight 0`) are unchanged; dynamic
lights still cast no shadows of actors or movers.

### Map edits' lights

SJK's map edits ([map-edits.md](map-edits.md#lights), 08/11/2026) reach the lighting two
ways:

- **Added lights** go into this list each frame, after every other light, into slots kept
  for them when the list is cleared: `PointLightList::reserve` (as many as the map has, 8
  at most, `map_edits::lights::BUDGET`) makes `push` stop that many short of 32, and only
  `push_reserved` fills them. So a frame of explosions keeps its 24 or more lights and the
  added lights their slots; a slot left unfilled is simply unused. Of more than 8, those
  whose reach comes nearest the view are drawn. They are ordinary (not radiant) lights, so
  every preset draws them, with the walls stopping them in real-time lighting as above.
- **The map's lamps** each carry their fixture (`lamp_lights::Fixture`): the BSP surface of
  a patch's first triangle (the world draws' index ranges resolve it,
  `world_material_build::resolve_fixtures`), a prop's entity and model surface, or a steady
  effect's entity; vertex-coloured flare clusters span surfaces and carry none. When the
  world is built, the fixtures are summed up for the browser editor
  (`lamp_lights::summarise`: one marker per fixture, its lamps' power-weighted middle), then
  the map's lamp edits drop the lamps of the fixtures turned off and move those of the
  fixtures moved (`LampEdits::apply`), before the grid, the visibility atlas and the light
  cache are made from them. Without lamp edits the lamps, their order and so every lamp sum
  are as before. A change applies when the world is built again (a map load, or
  `vid_restart force`); nothing is rebuilt live.

## Indirect lighting and dark-area readability

These controls affect the day/night real-time material-lighting path, including
world surfaces, actors and reflection receivers. They do not change the ordinary
baked-lighting path, direct sunlight, lamp intensity, emissive materials or exposure.
The indirect multiplier includes the probe sky contribution and the sky fallback;
it is not a multiplier exclusively on secondary bounces. It is applied at the
receiver after probe sampling, so boosted energy never feeds back into the probe
solver. World-space probe visibility and wall rejection remain unchanged.

Defaults retain the previous lighting response. Start a comparison with
`r_indirectBoost 2`, leaving `r_ambientFill 0.025` and
`r_ambientFillOcclusion 1`. For a separate, stronger readability comparison, try
fill `0.05` and fill occlusion `0.5`. These are experimental settings, not new
recommended defaults. Restore all three values to `1`, `0.025`, `1` respectively
for the original response. Controls are live and archived.

Fill occlusion only changes the small artificial fill: at `0.5`, fully occluded
fill retains half its unoccluded contribution. Physical sky/bounce stays fully
occluded. Fill remains material-modulated, so black materials stay black. A room
with almost no indirect energy may change little under the multiplier alone;
blindly increasing it can brighten outdoor shade before fixing that room.
Local exposure and player-specific contrast effects are not part of this
experiment; [eye adaptation](#eye-adaptation) is separate and scales the result.

Verification of the local change based on `8f692ac` (2026-10-03): Linux/Vulkan,
Radeon RX 9060 XT, external release captures at 1280×720, fixed 11:00 sun.
The outdoor `mp/ffa3` and indoor `mp/ffa1` views exercised live changes and floor
reflections; a Kyle preview also rendered under the live controls in `ffa1`.
Returning both controls to 1 reproduced each starting image exactly.
Against separately compiled preceding shaders, baseline captures differed by at
most 19/255 on `ffa3` (mean absolute channel error 0.00057/255) and 1/255 on `ffa1`
(mean 0.0000125/255); cross-build images are not claimed bit-identical.
An external 1,024-case GPU check passed baseline agreement, isolated indirect gain,
bounded fill and zero-energy behavior with fill disabled. HDR/tier 0,
SDR/tier 2 and day/night-disabled paths rendered successfully; the latter stayed
byte-identical across the live control changes. Workspace build,
tests and formatting, and the optimized owner client build passed.

Sampled steady-state light-pass medians for old/new default shaders were
0.212/0.214 ms on `ffa3` and 0.157/0.158 ms on `ffa1`; enabling the controls showed
no consistent additional cost at this resolution. A separate owner game remained
running, so these are non-exclusive GPU checks, not a performance certification.
Populated combat visibility, broader map coverage and 31-player performance remain
owner-playtest/benchmark work. No new shadow lights, passes or probe rays were added.

Without real-time lighting, sun shadows reach the world as a second pass over its diffuse
surfaces that multiplies in the shade and tests for equal depth against the stage program.
That pass also runs when light shafts alone turned the world shadows on. Its position is
`@invariant` like every other equal-depth redraw: without it a driver may round the depth
differently, and the shade would miss a speckle of the floor that changes as the camera
moves. That is the likely cause of hub report #193 (floors flickering with light shafts
on), not a verified one: the flicker did not reproduce on the test machine, where frames
with and without the qualifier are identical.

Sun-shadow filtering compensates for the receiver surface's slope across the
full texel footprint of a linear depth comparison. A half-texel allowance only
covers samples midway between texel centers and can produce bands of false
self-shadowing on flat surfaces. Keep this allowance tied to the cascade's
texel size and receiver slope; increasing a fixed world-space offset can detach
shadows from walls and ground contacts.

The full-footprint correction was checked against the preceding shader from
`4a8fe31` with external release captures on Linux/RADV (Radeon RX 9060 XT), at
1920×1080 with HDR, lighting tier 0, 2048-pixel shadow maps and a fixed 9:00 sun.
The reported `mp/ffa3` view and a nearby camera position lost the ground bands
while retaining the ship and wall shadows; an `mp/ffa1` interior comparison
showed no obvious regression. GPU frame means were 3.336 → 3.341 ms for the
nearby `ffa3` view and 4.122 → 4.125 ms for `ffa1` (64 measured frames each).
These isolated captures are not a full gameplay benchmark or coverage of every
map, sun angle, graphics backend or shadow quality setting.

Sun-shadow cascades share a world-space reconstruction footprint while blending.
The close transition covers the last half of its axial range; the view/far
transition covers the last quarter. The minimum footprint is based on 1.5 texels
of the active cascade, interpolated toward the finer map during a transition.
This avoids cross-fading independently sharp and soft versions of an edge.
The accepted transition refinement added approximately 0.074 ms of GPU work in
the marked 4K ship view on Linux/RADV (Radeon RX 9060 XT, tier 0, HDR,
2048-pixel shadow maps, 16 base taps, fixed 11:00 sun). It does not establish
complete temporal invariance.

Static world and moving casters now keep separate depths in the close and view
cascades. A nearby player cannot replace a distant building's depth in the world
filter's separation estimate. Receivers multiply the independently filtered
visibilities. This approximates their union; it is not exact area-light visibility
for multiple blocker depths. Volumetric lighting samples both depths at each tap.
Tiers 0/1 reuse the existing static maps without copying them under moving casters.
Tier 2 refreshes separate world maps each frame, adding two depth textures
(32 MiB at 2048 pixels, 128 MiB at 4096) and two render-pass boundaries.

World penumbra width still grows with caster-to-receiver separation. The blocker
search bilinearly reconstructs positive gaps and coverage from 16 positions,
correcting for the receiver plane at each texel. Its reach is 24 world units,
expanded if needed for the minimum reconstruction footprint; it no longer clips
the close cascade's broad penumbrae at twelve tiny shadow texels. Reconstruction
uses a truncated Gaussian disk to reduce the visible rim of an equal-weight disk.
`r_sunShadowTaps` (4–32) supplies the base count, with up to four times that budget
for broad filters and a fractional final tap for continuous count changes.
The full-texel slope correction and small normal offset remain unchanged.

Moving casters use the shared contact footprint, independently of the building's
penumbra. Their shadows therefore remain comparatively sharp even when a moving
caster is high above its receiver. Actor-only shadow mode retains separation-based
filtering. Neither approach resolves all shadow-map occlusion or sampling limits.

The preceding combined-map RMS filter was rejected in owner playtesting: the
reported ground-fixed boundary and player interaction remained visible, especially
when structures were far from the surface receiving their shadow. The current
separation/filter changes were checked at the third `mp/ffa3` mark
`(-425.691, -1061.451, 73.832)`, yaw `132.718`, pitch `38.102`, with an external
release harness carrying the production shadow code from the working tree based
on `980e693`. Linux/RADV captures on the same RX 9060 XT at 1080p and 4K,
HDR, tier 0, 2048 maps, 16 base taps and fixed 11:00 sun showed a smoother broad
edge. A frozen Kyle model was moved through seven positions in the actual dynamic
caster pass. GPU attribute readback found no visibility increase on unchanged
receivers when adding the actor (over 515,000 compared pixels per position).
The previous combined-map filter's maximum increase was only 0.000119 in this
particular probe; the more visible improvement is the distinct player shadow
instead of its inheriting the building's broad blur. This is not a complete
reproduction of every reported player interaction.

Earlier wall/ship views and a seven-position camera approach were also captured.
Tier 2, day/night disabled and actor-only modes passed GPU smoke checks. At 4K,
64-frame means compared with the preceding combined-map filter were
1.579 → 2.100 ms for the light pass and 4.212 → 4.700 ms for GPU work excluding
capture/readback, approximately 0.49 ms added. These are empty-scene measurements
on an active desktop, with release compilation running concurrently, not a
populated-match benchmark. Workspace build/tests, formatting and the production
release build passed; Cargo runs no bundled regression tests. The owner accepted
the release playtest on 2026-10-02. Broader live-animation checks, other GPUs and
exhaustive quality/map coverage remain open.

## Toon models

Toon shading (Sol's request, 02/11/2026) draws a model lit by `rgbGen lightingDiffuse`
or `lightingDiffuseEntity` in a stylized, cartoon-like way, a look in the spirit of
current stylized shooters made from the game's own light.

Since 08/11/2026 it is each player's choice for their own model (Sol: some models
look better with it, some without): `cg_toonModel` (off by default; the Profile's
Character tab, "Toon shading", or Settings > Graphics > Image, "Toon on my model")
goes in the player's hub look as toy `toon_model`
([identity.md](identity.md#unlocks-and-looks)), so every SJK player draws that model
as its wearer chose. `r_toonModels` (Settings > Graphics > Image, "Toon models", on
by default) says whether this PC shows the choices: 0 draws every model in stock
shading. A player without SJK, or whose hub look has not come, keeps stock shading, as
do NPCs, items and the view weapon. The choice covers the player's body, hats, capes
and what it holds ([toon_choice.rs](../crates/sjk-viewer/src/toon_choice.rs): bit 28
of the instance's `view_flags`, which `entity_vertex` turns into an entity light `a`
of 2 that `toon_model` reads), and the menu's own model (`sync_stage_toon`), so the
Character tab shows the switch at once.

- **Bands.** In stock lighting, `toon_diffuse` (`model_light_grid.wgsl`) replaces
  `RB_CalcDiffuseColor`'s smooth falloff: the side facing the grid's light direction
  gets a lit tone, the far side a shadow tone, with a half step between and soft
  edges (`toon_steps`). The tones are 0.65x and 1.15x the model's mean light (ambient
  plus half the directed light) rather than ambient and ambient plus directed,
  because many maps' grids give a weak direction and stock's two ends barely differ.
  It runs per pixel even with `r_modelPixelLight 0`: bands per vertex would follow
  the triangles.
- **Real-time lighting.** There a model's light is one sum with the sun's shadows in
  it (`model_sun_color`, `sun_realtime_buffer.wgsl`), so `toon_lit` steps its
  brightness in halves, never below 0.6 of the light, keeping its hue.
- **Rim.** The silhouette, where the surface turns away from the camera, gets half the
  model's light again on top (`toon_rim`), the bright edge of the style.
- **Colour.** The textured result of those stages is a fifth more saturated
  (`toon_colour`, `world_lighting_mode.wgsl`); other stages of the same shader
  (additive shine, glow) are unchanged.

`r_toonModels` is bits 1..3 of the scene-light block's model word
(`point_lights.metadata.y`, beside `r_modelPixelLight` in bit 0), so it switches live
with no pipeline rebuild; the players' choices are read once a frame from the looks.
Cost: a few arithmetic operations per pixel of a toon model.

Limits: JA skins have shading painted in, so the result reads as stylized JA rather
than flat cartoon colours; there are no ink outlines yet; a player's corpse after they
respawn is a body of its own and keeps stock shading. `world_shot_toon.rs` renders stock and toon side by
side from a demo (`SJK_TEST_DEMO`, an ignored test).

## Entity render effects

NPC nameplate health uses JoF EJK
[#80](https://github.com/JediofFreedom/JoF_EJK/pull/80)'s visibility rule: active
level 3 Sense is required for an ordinary NPC, while mapper `showhealth`
(`shouldtarget`) bypasses the gate. The server's decoded Sense-rank field supplies
the level; live prediction supplies active-power bits when present, otherwise
the viewed player state does (including demos). Names remain visible, and player
bar consent, non-NPC map entities and ordinary bar-mode settings keep their rules.
The shared nameplate renderer applies this in every HUD/menu style. SJK's older
overhead-label pass handles players only; this change adds no separate crosshair
health widget. Wire codecs and server NPC behavior are unchanged.

JoF EJK's animation-driven Force sprays run in live play for the local player,
other players and NPCs ([#58](https://github.com/JediofFreedom/JoF_EJK/pull/58),
with [#89](https://github.com/JediofFreedom/JoF_EJK/pull/89)'s full-body gate).
Rage uses `force/rage2`, Heal `force/heal2`, every 120 ms while both animation
tracks match and speed is below 250 units/s. They follow `*chestg`, then
`thoracic`, then `lower_lumbar`, falling back to the actor's origin. Quick Heal
uses Push-blur puffs at the left hand, falling back to the actor's origin when
the attachment is absent. The hand effect uses SJK's reference 8 ms cadence
instead of multiplying emissions with rendering FPS. Local animation selection
uses prediction, falling back to player state; remote players and NPCs use their
own snapshot state. Timer storage belongs to the rendered world's effect pool;
inactive animations reset it and backward demo time restarts it.

The local player's body-push blur and Grip hand puffs are hidden in first person
and zoom, while remote actors keep them. This follows the local visibility intent
of [JoF #90](https://github.com/JediofFreedom/JoF_EJK/pull/90), including Grip-victim
puffs. Rage/Heal sprays and Quick Heal remain animation-driven in every camera
mode, as in #58.
These presentation rules apply in every UI style and change no Force gameplay,
movement, animation timing or protocol encoding.

### Portable shields

The force field a Merc places (`HI_SHIELD`) is an `ET_SPECIAL` entity with no model,
drawn by `CG_Special` → `FX_DrawPortableShield` (`cg_ents.c`). Until 26/10/2026 SJK
drew nothing for it, so players were blocked by a wall they could not see (blec-bee's
report, #133). [portable_shields.rs](../crates/sjk-viewer/src/portable_shields.rs)
reads its shape from `time2` (axis bit 24, height bits 16-23, the widths either side
of its origin in bits 8-15 and 0-7), its look from `otherEntityNum2` (red team
`gfx/misc/red_portashield`, otherwise blue) and `trickedentindex` (the `*_dmgshield`
look while it is hit), skips it under `EF_NODRAW`, and adds stock's 50 ms
`LE_OLINE` copy on the reference 8 ms cadence (`effect_cadence.rs`), so the additive
copies overlap as at 125 FPS. The copy is a new effect primitive,
`PrimitiveShape::OrientedLine` (`RB_SurfaceOrientedLine`'s upright quad, s across the
height and t along the field), drawn by the effect geometry's quad stamp, which the
GPU expansion handles too. World shot: `world_shot::portable_shield_shots`.

### Scaled players' held items

A player or NPC the server scales (`iModelScale`, also used for small models such as
the Jawa and community younglings, whose meshes are full size) keeps what it holds in
its hand: `G2API_GetBoltMatrix` scales a bolt's position by the model scale and keeps
its axes, and the bolted saber or gun is drawn with the wearer's `modelScale`. Until
26/10/2026 SJK scaled the body but put the hilt, the gun, the blade's start, a JA+
hook's rope and the left-hand Force effects at the unscaled hand, above a small
player's hand (g0dchris' report, #144). `saber::Attachment::scaled` and
`BladeSocket::scaled` now apply the scale; the blade keeps the saber's length, as
`CG_AddSaberBlade` does. Holstered hilts already followed it.

### Emplaced guns

An emplaced gun's top follows its gunner's view as in `CG_General`
(`cg_ents.c:1392-1416`): while the server marks the gun worked (`activeForcePass`
`NUM_FORCE_POWERS + 1`, a non-player, non-vehicle entity), its model's `Bone02` takes
the gunner's view angles with `BONE_ANGLES_REPLACE` about `NEGATIVE_Y`, `NEGATIVE_X`,
`POSITIVE_Z`, the pitch held to 40 degrees down and the yaw less the gun's own. The
gunner's view is this client's own for the local player, the player's interpolated
view angles for anyone else. Ghoul2 keeps a replaced bone's angles, so a gun keeps its
last aim after its gunner leaves; one no one has worked stays at its bind pose, barrels
to its right, as in the retail game.

Rigid models are drawn as instanced bind-pose meshes, so a turned gun is drawn as two
parts cut from its model
([emplaced_swivel.rs](../crates/sjk-viewer/src/emplaced_swivel.rs)): the surfaces bound
to `Bone02` or its descendants (the top and both barrels of `turret_chair.glm`), carried
by the bone's replaced matrix ([`replace_bone_matrix`](../crates/sjk-model/src/bone_angles.rs),
`G2_Generate_Matrix` and `G2_TransformBone`'s replace step), and the rest, the stand.
Here SJK departs from Ghoul2, which leaves the stand (on `Bone01`) still: the stand
turns with the top's yaw, upright, about the vertical through `Bone02`. Its brace
carries the handles the gunner holds, behind the top at the bind pose; left still, they
show off to the gun's side once the top turns. The parts
load through the model loader the first time a gun is worked. The model's animation is
taken at its bind pose, as the static mesh is; Ghoul2's frame 0 of `mp.gla` differs from
it by under a tenth of a unit. The muzzle flash and the dying gun's red pulse and
damaged skin are not changed by this.

### E-Webs

An E-Web set down (`ET_GENERAL` with `models/map_objects/hoth/eweb_model.glm`: the
holdable or a single-player map's `emplaced_eweb`) is turned by the server, as in
`CG_General`'s `CG_G2ServerBoneAngles`: its bone slots name `cannon_Yrot` and
`cannon_Xrot` by their `CS_G2BONES` strings, with the angles `EWeb_SetBoneAngles` gave
them, `BONE_ANGLES_POSTMULT` about the axes in `boneOrient` (forward `NEGATIVE_X`, right
`NEGATIVE_Z`, up `POSITIVE_Y`). The gun is drawn in three parts cut from its model
([eweb_view.rs](../crates/sjk-viewer/src/eweb_view.rs)), each triangle going with the bone
its first vertex leans on most: the stand (the base and the tripod's legs), the top
(`cannon_Yrot`) and the barrel (`cannon_Xrot` and below). Each part is moved by its bone's
matrix in the skeleton's frame 0 with the two turns applied ([`BoneOverridePose`](../crates/sjk-model/src/bone_override.rs)),
the gun standing unfolded on its legs with its feet 24 units below its origin, and
turned first by Ghoul2's root quarter turn (`GHOUL2_ROOT`, the one the server reads its
muzzle through; without it the barrel points to the gun's right). The
unfolding (frames 4-20) and the firing frames (2-4) that `EF_G2ANIMATING` asks for are
not played, and the pickup (`ET_ITEM`) is drawn whole as before. The parts load through
the model loader the first time a gun is seen; until they have, the gun is drawn whole.

While the local player works an E-Web (`CG_UsingEWeb`: the emplaced gun's weapon, its
`emplacedIndex` naming a `WP_NONE` entity) its movement is not predicted: the server
moves it behind the turning gun every frame, so its place is interpolated from the
snapshots and the mouse kept, as `cg_noPredict 1` does
([interpolated_view.rs](../crates/sjk-viewer/src/interpolated_view.rs)). Predicted, the
gunner shook as the gun turned.

### Force presentation controls

`cg_forceAnimFX` (on) controls animation-driven Rage, Heal, Quick Heal and Grip-victim
FX in every UI style. `cp_pluginDisable` bit 5 also disables these new effects,
following EJK #130, and disables the combined cyan Protect/Absorb shell. In that
case the existing green/blue shells remain available, following #168; Absorb is
already shown by default in SJK. The local animation switch does not change the
separate `cg_spProtAbsColor` preference or ordinary Push/Grip-caster effects.

Empower-marked body blur (`EF_EMPOWERED` with `EF_BODYPUSH`) follows the advertised
`jp_empowerEffect` scope: 0 full body, 1 hands plus radius joints, 2 hands only,
after #132. `cg_empowerEffect` defaults to -1 (server); 0/1/2 override it, and the
local 3 hides it (an advertised 3 is not in the contract and stays full body). Both
settings interfaces expose the controls. Ordinary Push always uses the full
cached joint set. The named contract/scope tables in
[force_fx_policy.rs](../crates/sjk-viewer/src/force_fx_policy.rs) document the
reference; missing or unknown modes use full body. No new pose, per-frame
allocation or wire field is needed. Installed-model/GPU parity remains unverified.

cgame custom shaders on actors (force shells, pickup placeholders) are extra
instances of the same mesh in [entity_materials.rs](../crates/sjk-viewer/src/entity_materials.rs).
The same path also draws rd-vanilla's `RF_FORCE_ENT_ALPHA`: the mesh keeps its
own surface shaders, the stage program replaces vertex alpha with the
instance's alpha, and each stage uses an alpha-blended, depth-tested,
non-depth-writing variant of its pipeline
([world_forced_alpha.rs](../crates/sjk-viewer/src/world_forced_alpha.rs)).
Like rd-vanilla's fixed `GL_State`, the variant has no alpha test: the stage
program is specialized to skip it, so a GE128 cut-out whose forced alpha is
below one half stays visible, and blending still hides its transparent texels.
These draws close the blended entity list, like the stock post-render queue,
but particle effects still composite after them. Model materials compile these
variants at map load (depth-tested only); stages without an alpha test share
keys with ordinary blended stages.

Private duels are drawn as JoF EJK's `CG_Player` draws them (EternalJK
`cg_players.c`; [duel_look.rs](../crates/sjk-client/src/duel_look.rs), applied in
[actor_world_submission.rs](../crates/sjk-viewer/src/actor_world_submission.rs)):

- Outside every duel, with `cg_duelGhost 1` (default; JoF plays with EternalJK's
  `cg_stylePlayer` bit `JAPRO_STYLE_VFXDUELERS`), a duellist is drawn only as a
  see-through blue figure, `sjk/duelFigure`, in place of its body
  ([duel_ghost.rs](../crates/sjk-viewer/src/duel_ghost.rs)). The shader is EternalJK's
  `gfx/effects/duelShader` (`assets/japro/shaders/japro_players.shader`) under SJK's own
  name, so a pack's copy never replaces it: the Force shell's texture added in dark blue
  through `tcGen environment`, a faint body-lit red wash that writes depth, and a pulsing
  blue added over all, at 0.6 of full rather than EternalJK's 0.9 (at 0.9 the figure read
  as solid in SJK). Its plain `textures/colors/red` and `blue` are mounted below game
  data in case an installation lacks them.
- With `cg_duelGhost 0`, the duellist is drawn flat dark grey (`RF_RGB_TINT` at 50 50
  50) under the `gfx/misc/sightbubble` shell at JoF EJK's 100 100 255
  ([force_overlays.rs](../crates/sjk-client/src/force_overlays.rs)), a lighter blue than
  stock's 50 50 255.
- In a duel, everyone but the two duellists is drawn at a fifth of their colour
  (`RF_RGB_TINT`). The two duellists look as they are: stock's distance-lit
  `powerups/forceshell` on them is EternalJK's `cg_stylePlayer` shell bit, off by
  default.

The tint goes only on the body as drawn: the shells over it and the outline mask
use the untinted instance, since stock clears `RF_RGB_TINT` before each shell.

Tried and dropped in playtest: drawing the bubble several times (it floods the body
flat blue) and SJK-made glows over or in place of the bubble (a chrome shell, an even
blue wash); each read as a hologram, and JoF EJK's look turned out to be the figure.

A shield hit (`EV_SHIELD_HIT`, timer in `shield_hit.rs`) is drawn as the body
re-drawn with `gfx/misc/personalshield`, like single player. `cg_shieldSphere 1`
draws stock multiplayer's `halfShieldShell` sphere instead
([force_overlay_submission.rs](../crates/sjk-viewer/src/force_overlay_submission.rs)).
The shader blends `GL_DST_COLOR GL_ONE`, a bare multiply of the pixels behind it, so the body
shell is faint; it is drawn `cg_shieldBrightness` times (default 2) and fades over its own
hit's length (`LegacyShieldHit::body_brightness`), where the sphere keeps stock's fixed 2 s.
The body shell's tint is weighted to green (`(0.3b, b, 0.3b)`), as the texture itself reads blue.

Force Protect is a green `gfx/misc/forceprotect` shell on the body and Force Absorb
a blue `gfx/misc/personalshield` one, drawn on every player holding Absorb (JoF EJK's
`cg_alwaysShowAbsorb`, always on here; stock draws it only on your own body and on
team-power hits). With `cg_spProtAbsColor 1` (default, JoF EJK's `cg_spprotabscolor`),
a player with both Protect and Absorb active gets a single cyan protect shell instead
of the green and blue pair, as single player draws it. Another player's Absorb is read
from their entity's own power bit, so both the combined and the plain shell show on them
([force_overlays.rs](../crates/sjk-client/src/force_overlays.rs)). JoF EJK's
base-enhanced server check is not ported.

`CG_DrawPlayerSphere`'s spheres surround a living player, in `CG_Player`'s order: the
Ysalamiri shell (in Capture the Ysalamiri, the flag carrier's red or blue one), Force
Boon, Enlightenment (dark over light) and the green spawn-protection shell of
`EF_INVULNERABLE`. Each is `models/weaphits/testboom.md3` with its `powerups/*shell`
shader, 9 units above the origin, turned to face the rendered view
([player_spheres.rs](../crates/sjk-client/src/player_spheres.rs),
[player_sphere_submission.rs](../crates/sjk-viewer/src/player_sphere_submission.rs)).
Your own shows in first person too, as in stock. The refraction pass behind them
(`effects/refract_2`, `cg_renderToTextureFX`) is not drawn, nor is JA+'s coloured
spawn bubble style.

A player with `EF_JETPACK` wears `models/weapons2/jetpack/model.glm` on the body's
`*chestg` bolt, faded and hidden with a mind trick like a held gun. While
`EF_JETPACK_ACTIVE` is set, `effects/boba/jet` plays at the pack's `torso_ljet` and
`torso_rjet` bolts on the 8 ms effect cadence (twice each with `EF_JETPACK_FLAMING`),
the selected ignition sounds when the jets start and `bf_blast-off` when they
stop, and the selected hover loop (plus `fire_lp` while flaming) joins the frame's loops
([jetpack.rs](../crates/sjk-viewer/src/jetpack.rs),
[jetpack_submission.rs](../crates/sjk-viewer/src/jetpack_submission.rs),
`cg_players.c:11843-11935`). A player who mind-tricks you shows no flames and makes no
jet sound. The chest attachment is initialized with the body's resting pose,
including the shared fallback body, and equipping resolves the current pose
immediately rather than waiting for the next pose upload. The stock pack's bind
pose is already identity; personal replacement packs may omit its flame bolts.
The jet graph joins the startup effect catalogue, so a replacement PK3's own
flame/smoke shaders and nested dependencies are available before flight. Tail length
uses the authored curve flags: a negative constant stays constant instead of
shrinking toward an unused end value. A shader remapped to `clear` supplies no
layers, including on exhaust tails; only an unknown shader gets the fallback spark.
These rules are shared by the other EFX users, without a jetpack-specific renderer.

Flight prediction now implements the equipped `PM_JETPACK` path described in
[networking.md](networking.md#equipped-jetpack-prediction). In first person the
pack remains hidden with the body, as in the reference.

JoF EJK #43's `cg_jetpackOnSound` and `cg_jetpackHoverSound` (both default 1)
select `bf_land`/`jeton` and `jethover`/`bf_jetpack_lp` respectively. The named
asset tables and integer-choice semantics live in
[jetpack_sounds.rs](../crates/sjk-client/src/jetpack_sounds.rs); both hover
samples are registered at load, with fixed-capacity loop selection per frame.
Start/stop use the actual wearer's source on `CHAN_AUTO`, following EJK #87,
with normal positional falloff. The shared settings expose both choices in
SJK and classic menus; see [client](client.md#jetpack-sounds).

`EV_PLAYER_TELEPORT_IN/OUT` play `mp/spawn`, and `EV_BECOME_JEDIMASTER` plays
`mp/jedispawn`, where the player's box (mins z -16, maxs z 40) lands when dropped up to
4096 units against `MASK_SOLID` (terrain included), as `cg_event.c` does
(`:2409-2430`, `:2699-2751`), not at the player's centre; over a void no effect plays.
The effect's forward axis is straight up (`ang = (0, 0, 1)`), whichever way the player
faces. Its green beam is drawn by `org2fromTrace` lines: `CFxScheduler::CreateEffect`
ends such a line where a trace from its origin along the forward axis meets a solid,
at most 16,384 units away (`FxScheduler.cpp:1392-1418`), so the beam stands from the
floor to the ceiling or sky.
SJK traces every `org2fromTrace` line once, before it is first drawn
([effect_geometry.rs](../crates/sjk-viewer/src/effect_geometry.rs)
`resolve_traced_streak`), with `org2isOffset` moving the untraced end as electricity
does. Lines used to take only their authored `origin2`, so a traced line had no length
and drew as a camera-facing square at the floor. Trip mine beams keep their own cached
trace. Stock's `CG_Trace` also stops the dropped box on solid entities such as a lift;
SJK drops it onto the world only, and `traceImpactFx` on a traced line is not played.
The beam's emitter climbs at 3000 units/s and plays `env/beam_lights` every 100 units: a
green light and an `FxRunner` with `count 0 0.6` running `env/beam_bolts` (two
`Electricity` bolts traced out to the walls, `count 0 0.6` each). `CFxScheduler::PlayEffect`
rounds every primitive's count, runners included, so each runner fires about one time in
six, and a run plays one random `playFx` entry (`CMediaHandles::GetHandle`). SJK used to
play every runner's entries once whatever its count, which gave the spawn beam about six
times retail's lightning ([effect_runtime.rs](../crates/sjk-viewer/src/effect_runtime.rs)).

The Force Speed afterimages use it: two copies of the actor in its current pose
at alpha 100 and 50, spaced by `(int)(6 * speed * 0.004)` units along the
recent path, while the entity has `PW_SPEED` and `cg_speedTrail` is nonzero
([speed_trail.rs](../crates/sjk-viewer/src/speed_trail.rs), after
`cg_players.c:10841-10906`). Copies are excluded from shadow casting. The
mind-trick fade below suppresses them, out and back in, as in stock. The
`PW_SPEED` saber trail
(`cg_players.c:7319`) is not implemented. This has passed unit tests only;
appearance has not yet been checked on a GPU against the stock client.

Force hand and body effects
([force_power_submission.rs](../crates/sjk-viewer/src/force_power_submission.rs))
follow `CG_Player`: the Lightning (`activeForcePass` 1-3) and Drain (4-6:
`mp/drain`, `mp/drainwide` at level 3) beams from the left hand, the Push/Pull
or Grip puffs there while `PW_DISINT_4` is set, and the body push blur for
`EF_BODYPUSH`. The local player's effects come from its predicted player state,
since stock rebuilds the local entity from `cg.predictedPlayerState` and the
server never sends it; in first person they start at the local body's left
hand, and Grip's puffs are third-person only. Mindtrick hides these Force effects,
the animation-driven sprays and the flamethrower stream/audio from the tricked viewer, after EJK #93. Active Force
Sight defeats that hiding, as it does for the actor. Jetpack visibility/audio
already uses the same rule. Existing emitted particles retain their normal lifetime.

Grip victim puffs (`cg_gripVictimFX`, archived, default on) follow JoF EJK
#33 and the later #90 victim path: `BOTH_CHOKE3` on the legs marks a choking
actor, including while the torso transitions. Two red `gfx/effects/forcePush`
puffs move in opposite directions at 55 units/s, live for 120 ms, fade their
RGB from 60/8/8 to black and grow from radius 8 to 10 (`CG_AddPuff`). They use
the existing 8 ms emission cadence to bound density at higher frame rates.
The posed `cervical` joint is preferred, with head-minus-eight-world-units as
fallback and a two-unit forward offset. Missing joints suppress the effect.
Joint indices are cached at model load; the existing pose supplies positions.
The viewed player's puffs are hidden in first person or while zoomed; other
players and detached cameras retain them. This visual animation marker does
not identify a Grip attacker or change gameplay. See
[grip_victim.rs](../crates/sjk-viewer/src/grip_victim.rs).
JA+ flamethrower presentation uses the documented table in
[japlus_flame.rs](../crates/sjk-viewer/src/japlus_flame.rs), with the archived
`cg_flameThrower` Auto/Off/JA+ override. `eFlags` bit 17 during a Lightning pass
selects `mp/fthrw_bobafire` at the same left-hand attachment and 8 ms cadence;
when missing, the preloaded retail `boba/fthrw` graph is used instead (JoF EJK
[#67](https://github.com/JediofFreedom/JoF_EJK/pull/67)). Its forward direction is
reversed with a half-turn around local up. SJK's quaternion frame also reverses
the side axis, whereas EJK flips forward alone; asymmetric community replacements
need live comparison before claiming exact appearance parity. When both graphs
are missing, stock Lightning stays, without repeated load attempts during actor
submission;
the humanoid upper body uses `BOTH_FORCELIGHTNING_HOLD` with the standard 100 ms
blend. `ET_GENERAL` entities with `eFlags2` bit 8 replay `env/fire_wall` after
the reference 100 ms interval. Impact timers use presentation time, so they
advance between snapshots and restart on a backward demo seek. The positional
`bf_flame.mp3` sample's 2.9 s repeats retain snapshot server time. The weapon
channel is stopped when firing ends, the player disappears or is muted, or the
session ends. Models missing the named pose or hand keep the safe existing
fallback; effect graphs and audio must exist in installed data. No server
weapon or protocol rules are changed.

EFX `bounce` and `intensity` are one key in retail: both set a primitive's
single elasticity value (default 0.1) and its physics flag (`FxTemplate.cpp:44`,
`:448-458`, `:2128`). A particle bounces by it, an electricity bolt uses it as
its jaggedness (`FxScheduler.cpp:1502-1508`) and a camera shake as its strength.
SJK applies it as stock, so Drain's `bounce 0.8 2` bolts get that jaggedness
rather than the default 0.1, which draws them almost straight. `elasticity` and
`chaos` are not retail keys and are ignored.

A `Flash` primitive's colour is scaled as `CFlash::Init` scales it when it starts:
by its distance from the rendered view, quadratically to nothing at 900 units, and to
nothing beyond 60 degrees off the view axis past 100 units (brighter within them). It is
then drawn as `CFlash::Draw` draws it, on the viewer rather than where it went off: stock
places an `RT_SPRITE` 12 units along the view axis with radius `12 * tan(fov_x / 2)`
(its size curve ignored), and `gfx/effects/whiteFlash` adds `$whiteimage` with
`depthFunc disable`, so it covers the view's width (and more than its height) over
everything. SJK draws it as a clip-space quad of those extents (`entity.wgsl` kind 6,
the clash flare's path) in the shader's blend slot. Unscaled, `demp2/altdetonate`'s
2048-unit flash lit the whole screen wherever a DEMP2 shock went off; drawn as a
billboard at the shock, it was hidden by nearer walls and tinted the sky behind it up to
a hard edge (Slash, hub report #125;
[effect_flash.rs](../crates/sjk-viewer/src/effect_flash.rs)). A flash within 100 units
behind the viewer now shows, as in stock. `localizedFlash` (a 2D picture at the effect's
screen position) is not modelled; no retail effect uses it.

A primitive's colour start and end are picked once when it spawns (`GetRGB_Colors`). An
rgb `random` flag then scales the whole colour by one random number each frame
(`CParticle::UpdateRGB`, `CLight::UpdateRGB`): SJK drew one per channel, which turned the
DEMP2's white flash magenta, yellow and purple frame to frame (and tinted the other
retail effects with the flag: lava, `env/powerbolt`, `mrancor/breath`, ship burners).

With that effect, `FX_DEMP2_AltDetonate` (`fx_demp2.c:263-282`) leaves the DEMP2 alt
fire's bubble, an `LE_FADE_SCALE_MODEL` local entity, and SJK draws it as
`CG_AddFadeScaleModel` does (Slash, hub report #157;
[demp2_shell.rs](../crates/sjk-viewer/src/demp2_shell.rs)): `models/items/sphere.md3`
with `gfx/effects/demp2shell`, unrotated at the event's origin for 800 ms, its axes
scaled by the event's `weapon` (the charge's units doubled) times the cube of the
elapsed fraction, the vertical by half, and its `shaderRGBA` falling from white to black
on that curve. The bubble's `RF_VOLUMETRIC` (`tr_shade.cpp:1164-1189`) is an instance
`view_flags` bit (26) read by `stage_runtime.wgsl`: every vertex colour channel is that
red times one less the fourth power of the model normal's dot with the view axis (zero
under 0.2), replacing the stage's generators as disintegration's colours do, so the rim
glows and the middle stays clear. Up to 32 bubbles live at once.

Effect particles share one pool of 4,096 slots, plus 256 kept for per-frame
billboards (talk balloons, pickup icons, hook ropes)
([particle_types.rs](../crates/sjk-viewer/src/particle_types.rs)). Stock caps its
FX list at 1,800 (`MAX_EFFECTS`, `FxPrimitives.h`) and, once full, frees and reuses
`effectList[0]` for every new primitive (`FX_GetValidEffect`, `FxUtil.cpp`), so each
new puff replaces the last and rocket trails vanish in a barrage. SJK instead makes
room at the start of each frame: when fewer than 256 to 1,024 slots are free (the
amount grows after a frame that ran out), the effect particles closest to the end of
their lives are removed early, so new trail puffs and impacts always spawn and old
smoke trails get shorter
([particle_room.rs](../crates/sjk-viewer/src/particle_room.rs)). The instance buffer
holds every slot's billboard at eight shader stages; it used to hold 1,024 instances
in all and dropped the newest particles first.

Two console diagnostics trace a wrong-looking or missing effect; both are 0 by
default, not archived, and change nothing while off
([effect_debug.rs](../crates/sjk-viewer/src/effect_debug.rs)):

| Cvar | Effect |
| --- | --- |
| `fx_debug` | Logs each effect as it plays, at most once a second per effect name, with every component's kind, life, size and shaders. The effect atlas also logs once per shader that an effect shader has no image, when it is built or expanded while the cvar is on |
| `cg_debugMissiles` | Once a second logs the missiles in the snapshot (number, weapon, flags, `otherEntityNum2`, trajectory type) and whether their trail effects loaded |

Both follow the cvar every frame, so they stop when it is set to 0 or the
session ends.
Mind Trick follows `CG_Player` (EternalJK `cg_players.c:10191-10345`, stock
code; [mind_trick.rs](../crates/sjk-client/src/mind_trick.rs)). A player who
tricked the viewer fades out at 0.5 alpha per millisecond from 255 (about half
a second), drawn with `RF_FORCE_ENT_ALPHA` like the speed afterimages, then is
hidden: no body, held weapon or held saber (a thrown saber still shows), no
shells or afterimages. It casts no blob shadow while the trick lasts. When the
trick ends it fades back in at
1 per millisecond. Its Force beam and hand puffs stay visible throughout, since
stock draws them before its mind-trick cut-off. A player unseen for over a
second starts again from opaque. The viewer's active Force Sight, at any level,
sees through every trick (`CG_IsMindTricked`); the server also ends the trick.
The trickster sees `force/confusion_old` over the head (`*head_top`, else
`ceyebrow`) of each player it tricked, unless that player's Sight is active.
Deviations: a held saber's hilt stays opaque during the fade (blades are opaque
in stock too), and a fading body casts no sun shadow.

Set `SJK_FRAME_BUDGET=1` for frame-work and GPU-phase diagnostics (the main view's
light pass shows as `light-cache`, `light-receivers`, `light-shade`, then `light-pass`
for the direct-lamp list); `perfmark <text>`
writes a timestamped `perf-mark <text>` line to the client log, so a cfg script of
settings, `wait`s and marks splits one run into steps (09/10/2026). Measurements
must name the build mode, GPU, resolution, settings, map and population. Separate
loading/shader warmup from steady frames and CPU work from GPU timings. The
500+ FPS target remains open; neither a single GPU timestamp nor an uncapped
empty scene demonstrates it.

#### Frame timings readout and log

The same measurements in game, without a variable (Sol's request, 09/11/2026, to work
on the graphics settings; [frame_timings.rs](../crates/sjk-viewer/src/frame_timings.rs)).
Both are off by default and live in Settings > HUD > Readouts:

- `r_drawTimings` ("Frame timings"): a panel at the top right of the screen (since
  09/11/2026; the HUD edit mode moves it, `timings` in `cg_hudOffsets`), in the SJK
  UI's look in every menu style, over everything but the full-frame console, so it
  can be watched while settings change in the menus. 1 lists the graphics card's
  sections of the frame (GPU), 2 the processor's eight heaviest host phases (CPU)
  under them. Each row is the mean over the last half second and the worst frame of
  it, with a bar for its share of the frame; a section of a quarter of the frame or
  more is named in gold. Sections under 0.005 ms are left out. Without timestamp
  queries (the driver lacks them) the panel says so and lists the CPU phases.
- `r_logTimings` ("Log frame timings"): `logs/timings.log` beside the configuration
  (the [session log](client.md)'s folder), one line a second, the session's first
  log keeping the one before as `logs/previous-timings.log`; it starts again past
  16 MB. The header names the version, the date (UTC) and the adapter. Before a
  line, the window's size, the map and the graphics settings are written whenever
  they change: every Video and renderer row's cvar on a `settings` line the first
  time, then `set <cvar> <old> -> <new>` and `preset <name>` for each change. So a
  run that turns one setting off and on reads back as before and after.
  `perfmark <text>` writes `mark <text>` in it too.

A log line reads, with means in milliseconds and the totals as mean/worst:

```
t=12.0 fps=243 frames=243 cpu=2.100/3.500 gpu=3.420/4.100 samples=81 | gpu: uploads=0.100 ... | cpu: pose+skin+upload=0.500 ...
```

`gpu:` lists the sections in frame order, `cpu:` the host phases heaviest first
(the `frame_budget.rs` names, the `SJK_FRAME_BUDGET` report's). The CPU total is the
frame's host work with its blocking calls: `acquire-wait` and `present-call` are the
processor waiting for the graphics card or the display, so a frame heavy there is
held by the GPU or vertical sync, not by the processor's own phases. A graphics
reload (`vid_restart`) builds the renderer again: the log carries on in the same file
under a new header and a full `settings` line. While either runs, the
profiler ([gpu_phases.rs](../crates/sjk-viewer/src/gpu_phases.rs)) samples every frame
whose previous readback has landed (about one in two or three; `samples` counts them)
instead of one in 32; the `gpu-phases` report and the low-FPS help keep their one in
32. The host phases are every frame's. Nothing is allocated per frame: the panel's
text has fixed storage, and a log line is formatted once a second.

Each section ends at its mark, so it holds what was encoded since the one before:

| Section | Holds (where it is marked) |
| --- | --- |
| `uploads` | the frame's start: instance packing and uploads (`main.rs`) |
| `dlight-shadows` | dynamic lights' wall shadow tiles (`world_material_draw.rs`) |
| `views` | the portal view and the sky portal's view (`scene_view_render.rs`) |
| `door-tiles`, `cascades`, `probes`, `lamp-shadows`, `projector-shadows` | the sun's shadow casters in parts: door tiles, cascades, the reflection probes' refresh, lamp and projector shadows (`sun_shadows.rs`) |
| `actor-casters` | the rest of the sun's casters (`main_scene_pass.rs`) |
| `light-prepass`, `light-occlusion`, `light-cache`, `light-receivers`, `light-shade`, `light-pass` | SJK's real-time lighting: the main view's light pass in parts, `light-pass` the direct-lamp list (`light_buffer.rs`, `light_receivers.rs`) |
| `world-opaque`, `rain-wet` | under rain only: the opaque world, then its wet surfaces |
| `opaque` | the opaque pass (world, models, sky faces, clouds); marked with ambient occlusion, sun shadows or floor reflections on |
| `ambient-correction` | ambient occlusion (`r_ssao`) and the sun's shadows on the world |
| `floor-save`, `floor-planes`, `floor-reflections` | floor reflections (`r_floorReflections`); the first two are marked empty when they are off |
| `water` | the water surfaces |
| `scene-world`, `effect-encode` | effects merged into the scene pass (`particle_draw.rs`) |
| `scene-pass` | the rest of the scene pass: decals, fog, blended surfaces and models, effects |
| `glow` | the dynamic glow (`r_DynamicGlow`) |
| `volumetrics` | light shafts (with sun shadows on) and dust motes |
| `overlays+post+hud` | the resolve and post-processing (FXAA, bloom, eye adaptation, the display pass), then the HUD, menus and text |
| `capture` | the WHEEL OF DOOM's mirror and console screenshots |

A section's name shows only on the frames that mark it, so with a feature off its
time goes to the next section that is marked (without rain, the opaque world is in
`opaque`). Verified by unit tests (the windows' means and worsts, the log lines, the
settings' changes, the panel's storage) and the world shot `frame_timings_panel`
(`world_shot_frame_timings.rs`); not yet run in the client.

#### Gameplay hitch lines

Without any variable, the client logs stutters during play
([hitch_log.rs](../crates/sjk-viewer/src/hitch_log.rs), 10/10/2026). It keeps the
last 128 frame intervals (start to start, what the player sees) and calls one a hitch
when it is over 33 ms and over three times their median. The line names the three
largest parts of the frame that ran long: its host phases (the `frame_budget.rs`
names, always measured) and `between-frames`, time outside that frame's measured work
(event handling, a pacing wait, the OS):

    hitch: 87.3 ms (median 4.1 ms) config-refresh 71.2, between-frames 9.0, hud 3.1

At most one such line a second; every hitch still counts towards
`hitches: N in the last 60 s, worst X ms`, written after each 60 s of gameplay that
had one. Only gameplay counts: a live map on screen past its first frame, no load
or transition under way, the window focused. Two seconds after a map change, a
resize, a focus change, `vid_restart` or a graphics reload are ignored and start a
new median, and an interval over 5 s is a pause (window drag, sleep), not a hitch.
The lines go to stderr with the other `log::progress` lines (`play.bat` keeps them
in `logs\last-client.log`) and to the console scrollback without notify lines, so a
player can read them in the console or, with `logfile 1`, in `qconsole.log` in the
client folder (`GameData/SJK/`) and send that file.

The first lines from a busy server (10/10/2026) showed `snapshot+reconcile` near 52 ms
each time a player joined, whatever their model: the player sounds and the saber hums,
refreshed on every clientinfo change, each read the saber definitions again, listing
the 73,494 files of 354 PK3s and parsing every `.sab` (30 ms a time in a release
build). The local player's prediction did the same twice on its own changes
(`config-refresh`). The definitions are now read once per set of mounts
(`sjk_client::shared_saber_definitions`, keyed by
`VirtualFileSystem::mount_identities`), at world load, as the game reads them once a
map (`WP_SaberLoadParms`); a mid-match call takes under a microsecond.

### First-person view weapon

The first-person gun hangs on its `_hand.md3` tag rig, as `CG_AddViewWeapon` places
it ([first_person_weapon.rs](../crates/sjk-viewer/src/first_person_weapon.rs), policy
in [view_weapon.rs](../crates/sjk-client/src/view_weapon.rs)). Melee shows no
weapon: `WP_MELEE` registers no hand rig (`CG_RegisterWeapon`), so stock hangs the
baton view model of its item on handle 0, whose identity tag puts it at the view
origin with all of its geometry behind the eye. SJK used to draw it on the stun
baton's rig, so first-person melee showed a baton.

The view model follows EternalJK's `cg_fovViewmodel` (default 80; retail has no such
cvar and behaves as 0): the hand's forward axis is scaled so the weapon looks as at
that FOV, and the gun, barrels and muzzle socket go through the same transform
([view_weapon_offsets.rs](../crates/sjk-viewer/src/view_weapon_offsets.rs)). Details and
limits are in [status.md](status.md#first-person-weapon-field-of-view).

### Zoom, scope and camera shakes

A zoom puts the view in first person, in the order of EternalJK's
`CG_DrawActiveFrame` (`cg_view.c:3101-3131`): a living player on an emplaced gun is
not forced, saber or melee zoom (binoculars) is, riding a vehicle is not, any other
zoom (the disruptor) is
([scope.rs](../crates/sjk-viewer/src/scope.rs), `ZoomView::forces_first_person`).
The player's camera choice is kept apart from what a frame renders: `third_person`
is derived from the choice and the zoom every frame, so nothing is rewritten and the
choice returns when the zoom ends; toggling the camera during a zoom changes the
choice. In a live session the zoom is read from the predicted state, or from the
snapshot while following another player (`cg_predict.c:952`); in a demo only the
player's own cameras follow the recorded zoom, not spectate, look-at, orbit or free.
SJK does not force third person for emplaced guns, vehicles, knockdowns, grapples or
falls, so there the zoom leaves the choice alone.

The scope mask is looked up as the image `gfx/2d/cropcircle2`, in the engine's image
order (`.jpg`, `.png`, `.tga`, as rd-common's `R_LoadImage`), instead of through its
shader, whose other stage drew a full-screen white picture. JoF's HD scope
(`cropcircle2.png`) shows only with that order in place.

The electrobinoculars (zoom mode 2) have their own view, as `CG_DrawZoomMask` draws
it ([scope_binoculars.rs](../crates/sjk-viewer/src/scope_binoculars.rs)): the
scrolling blue static and scanlines of `gfx/2d/binocularWindow`, the heading strip
(a number every ten degrees, the view's yaw plus 180 plus 200, sliding with the
fraction, and a green marker under each), the strip's overlay, the pulsing circle,
the frame `gfx/2d/binMask`, the arrow down the left edge that sinks as the zoom
closes in (`zoomFov` 80 to 0 over 162 virtual pixels) and the top triangle that flips
now and then. Each picture is its stage image with its `zoom.shader` blend (filter,
additive or alpha) and `tcMod`; the frame is stretched over the window like the
disruptor's mask. The heading digits, chunky bitmap pictures in stock, are drawn by the
HUD in its vector font, centred in stock's 24x14 cells, and only inside the strip's
black box, where stock's frame would have hidden the others. Until 16/10/2026 the
binoculars zoomed with no view at all (a player's bug report). JoF's team readout
beside them and its JA Pro race-mode exception are not ported. World shot:
`world_shot::tests::duel6_binoculars` (the pictures only: the HUD's digits need a game).

Effect camera shakes are measured from the rendered view, as `CG_DoCameraShake`
measures from `cg.refdef.vieworg`, so a short-range shake reaches a first-person eye
but not the third-person camera behind the player. `cg_screenShake` (archived,
default 2) gates them: 0 off, nonzero on. EternalJK's level 2 also shakes the camera
for a weapon charging or just fired (`cg_weapons.c:793,801,2466`) and 0 turns off the
damage view kick (`cg_view.c:1145`); neither is implemented. The viewer's own muzzle
flash never shakes the camera (JoF's HD muzzle effects carry a radius-60
`CameraShake`; JoF EternalJK showed none when firing in the contributor's recordings,
in first or third person); other players' flashes and all other effects shake by
distance as in the reference.

The local player's muzzle flash starts with the predicted shot, as `CG_FireWeapon` runs
for predicted events, and the snapshot that confirms the shot does not start it again
([muzzle_effects.rs](../crates/sjk-client/src/muzzle_effects.rs)). The flash replays its
effect every frame for 30 ms, and an effect's `Sound` plays each time: JoF's HD muzzle
effects carry the weapon's fire sound, which so lands on the event's own fire sound. Until
12/10/2026 the local flash waited for the snapshot, a round trip after the shot, so every
gun sounded twice (Creyon's report). Other players' flashes still start from the snapshot.

## JA+ black sabers

The compatibility policy in [black_sabers.rs](../crates/sjk-viewer/src/black_sabers.rs)
takes server support from the session's `sjk_client::CompatProfile` (JA+ and
TaystJK/jaPRO; no second identifier table) and reads the raw `CS_PLAYERS` colour
indices, then applies `cg_blackSabers` and disable-mask bit 3 on supported servers
(orange when black is off); elsewhere colour 11 keeps its ordinary fallback. The
serverinfo/clientinfo scan, a fixed two-word hand mask over the 32 client slots, is
redone only when configstring 0 or a player's configstring changes; the cvars are
read each frame; held and thrown sabers consume it without
allocating or changing runtime equipment ownership. Muted players retain their
replacement colours. The default allows the feature, matching JoF EJK #50's
corrected disable-bit direction.

A separate blade material and alpha pass preserve the retail/RGB additive
paths and every existing blade-skin slot. Black trails use alpha blending with
the reference shader's two identical stages combined into one, retaining the
existing trail lifetime and UV fade. Black blades/trails are excluded from the
dynamic-glow image and emit no blade light. The renderer resolves installed
`gfx/effects/sabers/blackglow`, `blackcore` and `blacktrail` images at load time,
taking EternalJK's own from its folder when installed (jaPRO's `japro-assets.pk3`,
only those three pictures, as for its crosshairs), or generates black alpha
coverage: the neutral profiles for the glow and core, and for the trail a picture
shaped as jaPRO's `blacktrail` (solid over the first two thirds of U, which is a
slice's age as it scrolls, fading by 0.8; solid along V but for its ends). The trail
used to fall back to the core profile, solid only in a narrow band of U, so a swing
showed a translucent banded fan, clear by the blade, where JoF EJK draws a solid wedge
(Dave's test, 27/10/2026). The capsule's glow takes stock's sprite overlap: the chain
(`RB_SurfaceSaberGlow`) lays a sprite of the blade's radius every 0.65 radii, so a point
is under about 2 / 0.65 of them, and the halo's alpha is `1 - (1 - a)^2.2` (a little
under that full overlap, which read as too much glow in game, 28/10/2026) instead
of one sprite's, which made the black blade thinner than EJK's. Offscreen renders on
llvmpipe of a 150-degree, 60 ms sweep and a 40-unit blade, before and after, with the
generated and with jaPRO's pictures, matched EJK's solid wedge and halo width; the
capsule and generated pictures are still not a claim of pixel parity with EJK's sprite
chain. See
[status](status.md#ja-black-sabers) for verification and limits.

## First-person saber body

First-person saber mode draws the local posed body and arms instead of hiding
the entire player. Head variants and TIE pilot hoses are masked as in JoF EJK's
`CG_ForceFPLSPlayerModel`; hilts and blades continue to use the animated hand
bolts. The mask is removed when returning to third person. Hats/capes remain
hidden in first person. Head surfaces are also hidden in mirrors while this
mode is active. A retail `mp/duel1` GPU smoke check covered the blade view and
restored third-person head; arm framing and community models remain unverified.
The body is shown only for a living player in their own view, excluding
spectators/follow and dead flags. Shared fallback meshes stay hidden until
the local model loads. The body and head mask use one filtered equipment
decision; head descendants such as hair and helmets are masked too.

## Saber blade skins

A blade skin replaces a saber blade's colour (not its hilt) with a look of its own: the
Sun, Storm, Void, Frost and Prism blades and, since 10/10/2026, the Unstable, Molten,
Spectral, Glitch, Hologram, Runic, Chameleon, Banner and Heartbeat blades, SJK unlockables
([unlockables.md](unlockables.md#blade-skins)).
The renderer is generic and data-driven: a skin's look is a blade-skin file in a pack
the SJK hub delivers ([unlockables.md](unlockables.md#blade-skin-files)), and no skin's
values are in the code. [saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs) holds
the loaded skins (`LoadedSkins`, at most 256, numbered in id order) and the per-client
table of who wears which (filled from the hub's looks; ids and names are
[unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)'s); `BladeColor::Skin` carries
the skin's number, trail and light wherever a blade colour is chosen (in the hand,
thrown, first person, the menu stage). A skin whose pack is not loaded is the player's
stock blade. So is a blade the local player chose to draw stock: a duel's
(`cg_duelStockBlades`, `SaberSkins::set_plain`) or, for weak PCs, other players' or every
blade (`cg_stockBlades` 1 or 2, `SaberSkins::set_stock`,
[stock_blades.rs](../crates/sjk-viewer/src/stock_blades.rs)). `SaberSkins::get` then
gives the world's blade no skin, so nothing below is drawn for it (no skinned instance,
afterimage, glint, petal or trail and light colour of the skin), and the haze, lens,
horizon and solid passes, which need a skinned blade in the frame, are skipped once none
is left. The menus' preview takes the player's own skin from `SaberSkins::local` and the
cards and pictures from the loaded skins, so they keep showing it. A chroma
([unlockables.md](unlockables.md#chromas)) takes its saber's colour:
`SkinColor::worn_with` sets the turn from the skin's hue to the colour's, the instance
carries it (`chroma`, location 8), and `saber.wgsl` turns the skin's finished
glow and core round the grey axis by it (`skin_turn_hue`); the light and trail turn on
the CPU (`SkinColor::light_color`, `trail_color`).

- **Material.** Each loaded skin is the saber material of its number after the six
  retail pairs and the neutral RGB pair (`saber_rgb.rs` `SKIN_MATERIAL` = 7, sixteen
  slots, which hold the neutral pair until a skin loads there). Its glow/core pair is
  the pack's images, or generated from the file's two profiles by the same code as the
  neutral pair (`generated_glow`/`generated_core`; the neutral pair's bytes are
  unchanged and pinned by a test). Retail and RGB blades keep their materials and
  shading unchanged.
- **Parameters.** The skins' parameters are a read-only storage array of 256 `Skin` structs of 62
  `vec4`s (`SkinUniform`, 253952 bytes in all since 14/10/2026; a uniform of 16 before, which the
  16 KiB limit capped; bind group 2 of the
  saber pipelines),
  written only when skins load (`saber_gpu::Runtime::upload_skins`), never per frame.
  Each instance's material slot selects its skin. The lanes of a section a file leaves
  out (arcs, motes, hue) are zeros, which draw nothing, and the code skips them.
- **Animation.** `saber.wgsl` colours and animates a skin from two per-instance values,
  presentation seconds wrapped at 1024 s and a per-blade seed
  (`Instance::with_animation`; the seed follows the entity, saber and blade, so a thrown
  blade keeps its hand's) and its parameters: granulation (one or two octaves of value
  noise drifting toward the tip), flame tongues licking outward at the corona's rim,
  shimmer waves, up to 8 flare tracks each sending a bright knot from hilt to tip once a
  cycle, lit on the cycles whose draw passes the file's threshold, brightening and
  widening the corona, and the core's breathing. The corona is graded from the file's
  rim colours to its inside colour; the core is the file's hot colour with a fringe
  between two colours. The glow capsule may reach `corona.reach` (1 to 2) times the
  stock one; the effect bounds allow for 2 (`MAX_GLOW_REACH`). `fragment_glow` shades
  the glow the same way, so the dynamic glow's bloom follows the flares and arcs.
- **Arcs, motes and hue** (09/10/2026, generic, optional). Lightning arcs: up to 4 per
  skin (`MAX_ARCS`), each struck again `rate` times a second at a random place from
  hashes of its strike number and the blade's seed: a filament whose offset from the
  axis bulges `sin(π t)` out to one side over its span (or, for tip strikes, leaves
  from just below the tip and dies out in the air past it) plus two octaves of
  piecewise-linear noise (straight runs between random corners) re-drawn `jitter` times
  a second, drawn as a Gaussian line with a halo; a filament thinner than a pixel
  (`fwidth` of the glow's across coordinate, taken in both fragment entry points before
  any branch) widens to one and dims, so it does not break up at a distance. Motes: a
  field of cells along the blade and out from it (each column staggered), drifting with
  time, a share of the cells holding a speck with a smooth compact falloff, twinkling,
  shown between two radii and optionally only toward the tip; one cell is read per
  fragment. Both use the blade's coordinates round its tip and fade out before the
  quad's edge instead of being cut. Hue: the glow's colour (with its arcs and motes) and
  the core's fringe turned round the grey axis by time, distance along and distance out,
  negative channels clipped (the blend adds); the light turns with the blade's middle
  (`SkinColor::light_at`, its CPU copy `saber_skins::turn_hue`).
- **Round end** (09/10/2026). For skins only, the core line narrows on a quarter circle
  over its last `core.tip` half-widths (`skin_tip_taper`) to a rounded point and is cut
  at that edge over a pixel (the sampler would otherwise smear the texture's border out
  to the quad's square corners). That pixel is the straight line's, stretched as the
  line is (`core_stretch`), not the derivative of the stretched coordinate: on the tip's
  last row the stretch runs to a hundred within one pixel, the derivative blew up with
  it and the cut left half the width lit, a bar across every skin's tip (fixed
  11/10/2026); past the tip the corona widens about the tip as about
  the shaft, and its distance out (for its grading, its tongues, motes and hue) is
  measured from the tip, its tongues running on round it. The stock line ends flat (as
  `RB_SurfaceLine` does), hidden in its glow; a skin's brighter, wider fringe showed
  the flat end as a square. Along the shaft the skin's look is unchanged.
- **Later sections** (10/10/2026, generic, optional; [unlockables.md](unlockables.md#blade-skin-files)).
  Sputter: the glow's width wanders by noise per side (a frayed edge), and on random
  draws the blade's length is cut short for a moment (glow and core fade past the cut).
  Glitch: per block along the blade and per draw, a jump sideways (glow and core move)
  and a flash; red and blue are shaded with the glow moved apart (three evaluations of
  the glow, only for a skin with the section; the core samples its texture three times).
  Scan: scan lines darkening glow and core, a hollow inside, a wireframe of two side
  lines and rings round the blade (not behind the hilt), a jitter of the whole
  projection. Pulse: a double Gaussian beat on the clock brightening glow and core and
  widening the glow; the light beats with it (`saber_skins::Beat`, the CPU copy).
  Embers: drip places along the blade each dropping an accelerating ember along the
  world's down projected into the blade's plane (`down`, a flat varying from the vertex
  stage); where gravity runs along the blade they are pushed off to a side; a fragment
  inverts the path to read three places a side. Veins: ridged value noise along and
  across, bright in the glow's inside and the core. Team and ambient tints: the glow and
  the core's fringe drawn toward a colour at their own strongest channel's level, the
  colour being the wearer's team's (from the instance) or the light where the blade is
  (the hilt end sampled from the light grid once a frame after the entities are lit,
  `saber_persona::light_instances`, raised in saturation in the shader). Glyphs: the
  wearer's name (`CS_PLAYERS`, colour codes and signs left out, digits folded, at most
  18 letters, "SJK" without one; the own `name` on the menu stage) as SJK's own stroke
  glyphs, each letter a fixed three to seven of sixteen strokes on a 3 × 3 grid
  (`glyph_bits`, its CPU copy in `saber_persona.rs` pinned to differ for every letter),
  scrolling toward the tip.
- **Third set** (11/10/2026, generic, optional; [unlockables.md](unlockables.md#blade-skin-files)).
  The glint is an instance of its own (`Instance::star`, colour alpha `STAR` = -1, made
  by `saber::skin_extras` wherever skinned blades are submitted: hand, thrown, menu
  stage, world shots): a camera-facing square round the tip as wide as the rays are
  long, drawn by the main and the dynamic glow pipelines; its rays (every other one
  shorter) taper and soften to a pixel, turn with time and the seed and twinkle on two
  sines. With an `orbit` (the fourth set, 11/10/2026) there is an instance per glint
  (`Instance::stars`, glint n's alpha `STAR - n`; `star_motion.w` the count and
  `star_orbit` its place, at, radius, rate and tilt): the vertex shader puts glint n
  `at` units below the tip on a circle round the axis, its angle `time × rate + seed +
  n / count` turns measured from the camera's side of the blade (the near side at a
  quarter turn, so the circle reads as a ring from any side and never jumps as the blade
  turns), the tilt lifting the near side; every other glint is 0.7 the size, each
  twinkles on its own seed, and one behind the blade and close to its axis shows at 0.3
  (`GLINT_BEHIND`, carried to the fragment in `shaft`). Each glint's bounds are the
  point on the axis widened by the circle's reach. A skin with `holes` (11/10/2026) draws
  black holes in the glints' places, with the same instances (`hole_color`,
  `hole_shape`: size, lens, spin, the quad's reach): in the glow passes a thin photon ring
  just outside the horizon and a two-armed swirl round it (`hole_ring`); with the haze,
  before any glow, the lens (`vertex_hole`, `fragment_lens`, a pipeline of its own drawn
  after the haze's so it lies over it, alpha-blended): the scene image sampled as a
  point mass bends light, `p − p̂ E²/max(r, R)` with `E` the ring's radius, fading to the
  scene unbent at the quad's edge; and after the blades, in the main pass, the model
  preview and the dynamic glow image, the horizon (`fragment_shade`, alpha-blended
  black), so a hole in front of the blade hides its glow and bloom, and one behind it
  close to its axis (`shaft`, how hidden it is) lets the glow show. The effect layer's
  merge keeps a black pixel black (`post_aa.wgsl` `with_effects`). Wisps are drawn in the glow (`skin_wisps`): the world's up in the blade's plane
  (`down`, as the embers), a point's height above the nearest point of the blade and
  its distance aside give a column that widens as it rises and fades by the top, filled
  by rising, warped two-octave noise (`skin_rising`). The glow's quad reaches further by
  the skin's room (the wisps' rise, the haze's reach, when on; `skin_room`, and
  `Instance::room` for the bounds), while the glow's own parts (embers, arcs, motes)
  keep fading at the glow's edge, so switching wisps or haze off changes nothing else.
  That room draws only the wisps, so past the glow's own reach (the capsule widened
  `corona.reach` times, round the tip and behind the hilt) `skin_past_glow` skips the
  glow and its parts, for a skin whose glow neither moves nor reaches further (no
  glitch, scan, copies, glyphs or figures); and the wisps skip their noise where their
  column cannot show (below the blade, past the top, far aside). Up close the room
  covers much of the screen (17/10/2026, hub report 63).
  The echo is a dimmed glow instance (`Instance::echo`) swaying on two slow waves
  beside the blade, the tip more than the hilt, leaning a little, drawn with the whole
  look. Afterimages (`Instance::ghost`, 17/10/2026, hub report 64) carry the
  afterimages' own section bit (`OFF_GHOSTS`, never a section switched off since the
  CPU makes them) with their fade below 255: the vertex stage gives them the stock
  quad (no `corona.reach` swell, no room) and `afterimage_glow` draws the plain capsule
  in the corona's colours at their middle heat (granulation at its middle, no flare,
  the tongues' mean), beating, tinted and turned, with none of the noise or parts, in
  both glow passes. Before, each of a swing's four drew the whole look over the widest
  quad twice a frame; one fainter than 0.06 is no longer drawn (`saber_ghosts.rs`). Heat haze is a
  pipeline of its own (`vertex_haze`, `fragment_haze`, no blending, depth-tested): first
  in the effect pass (`GpuState::draw_particle_tail`), while the effect layer still
  holds the scene, each skinned glow quad with haze on (the vertex stage folds every
  other instance away) writes the scene image (`effect_layer.rs` `original`, bound with
  a linear sampler in group 3) read at an offset: rising noise times the haze's
  strength in world units, strongest by the glow and above it and nothing at its reach,
  turned into pixels by the inverse of the quad's own derivatives and held under 2 % of
  the screen. Its edge is the scene unchanged, so the merge sees no seam; effects drawn
  after add over it.
- **The fifth set** (15/10/2026, `skin_band`, `skin_segments`, `skin_film`, `skin_blobs`,
  `skin_front`; `Skin` grows to 75 lanes). Bands are cells along the blade (slanted by
  `twist` times the place across, in capsule radii), soft over a pixel; in one the
  corona's colour mixes toward the band's and its brightness is scaled, the glow narrows
  by `pinch` (taken on the axis so rings stay round) and the core line is recoloured on
  its own axis. Segments multiply glow and core: the section's draw (hashed per section
  and per `1 / rate` seconds) and soft dark gaps between sections, none at either end.
  The film recolours the glow at its own level with an HSV colour of three octaves of
  noise and a slow wave, clearing the inside and lifting a rim at 0.8 radii. Blobs are
  metaballs in the projected blade (along compressed by 0.75), each term squared so
  blobs keep apart until they touch, cut by a smoothstep at 1 and kept inside the
  column; two pools sit at the hilt and the tip. The front's height
  (`Front::height` on the CPU gives the same) climbs, holds and falls each period, its
  edge wandering by noise; below it glow and core are recoloured, and `veins` keeps the
  skin's veins to that part.
- **The sixth set** (15/10/2026, the Blade Lab's figures; `Skin` grows to 90 lanes),
  drawn in the glow on the blade's place along it (unclamped, so heads and tips run past
  the hilt and the tip) and across it: the sketch's outlines are two noisy curves round
  the axis and its hatching diagonal cells, redrawn on a stepped clock; the hourglass
  narrows the glow's width (`0.2 + 0.8 ×` its profile) and draws sand levels, a stream
  and a rim; the serpent is a sine round the axis whose cosine says in front or behind
  (in front it dims the glow and the core under it), with an ellipse head, eyes and a
  tongue; strands are lines `x = spread × along^1.5` with bright ends; butterflies are
  four ellipse wings each round a resting place, flown out on a loop for a share of
  their cycle; wings are feather segments fanned from near the hilt; copies call the
  glow again at offsets growing along the blade (and add a thin core line each). Lines
  thinner than a pixel widen to it and dim (`skin_line`). The butterflies' and wings'
  reach is the quad's room (`skin_room`, `BladeSkinDef::room`).
- **The seventh set: solid blades** (15/10/2026; `Skin` grows to 95 lanes). A pipeline
  of its own (`vertex_solid`, `fragment_solid`, alpha-blended, depth-tested, no depth
  write) draws after the blades, with the black holes' horizons, in the main pass, the
  model preview and the dynamic glow image (`SaberGpu::draw_horizons`): the vertex stage
  keeps a skin's glow quad when its ink or chrome is on, for the blade itself, and folds
  every other instance away. Ink: a body (a capsule, or drawn to a point; flat at the
  hilt) as a signed distance in the blade's units, filled with its colour and a glossy
  streak, and an outline as a band round that distance, both soft over a pixel. Chrome:
  the same body rippled and swollen by beads (metaball-like bumps), round at the tip,
  coloured by a made-up surrounding read with its normal across (dark ground, bright
  sky, a key highlight, a rim), tinted by the light where the blade is. The halftone is
  the glow's: outside 0.35 to 0.6 radii it is printed as dots sized by its level.
- **Options.** An instance carries its wearer's mask of sections switched off
  (`Instance::off`, attribute 9, two words: `OFF_*` bits, then `OFF2_*` for the eighth
  set's; [unlockables.md](unlockables.md#options)); `skin_at` reads the skin with those
  sections' lanes zeroed, so they draw as a file without them. Of the eighth set it zeroes
  only the unrest's: the CPU makes the rest.
- **Persona and afterimages.** A skinned instance carries four more `u32`s
  (`Instance::persona`, attribute 7): the surroundings' light (three bytes) and an
  afterimage's brightness, the team and the name's letters
  ([saber_persona.rs](../crates/sjk-viewer/src/saber_persona.rs)); who sits in each slot
  is read twice a second with the looks (`GpuState::update_looks`). Afterimages
  ([saber_ghosts.rs](../crates/sjk-viewer/src/saber_ghosts.rs)): each blade's state keeps
  up to four poses, one every `spacing` ms; a skin with `ghosts` submits the last ones
  as extra glow-only instances, dimmer each, skipping poses the blade has not moved
  from. No allocation. A thrown saber leaves them too (23/10/2026, Kushina's hub report
  127: the throw left no afterimages): its flying blade keeps its poses in the world, a
  signature's (`GhostSpec::wearer`) as well, so they trail along its spin and flight
  (`thrown_saber.rs` `flight_wake`). The poses and the blossoms' last tip are forgotten when
  the blade goes from the hand to flight or back (`BladeState::fly`): kept in the wearer's
  frame in one and in the world in the other, they would stand far off.
- **Trail and light.** The skin's file gives its trail colour, its light colour (with
  the stock gain) and the light's flicker (an amount and up to two waves, phased per
  hilt).

Per frame this adds one 8-byte vertex attribute per blade and nothing else on the
CPU; the noise is only evaluated for skinned blades. A skin with arcs and motes costs,
per glow fragment, a bounded loop of at most 4 arcs (about 9 hashes each, fewer for an
arc not lit or not there) and about 6 hashes for its mote cell, on top of the earlier
granulation, flares and tongues; retail and RGB blades only gain one `fwidth`. Not
measured on a GPU yet. The world shot `duel6_sun_blade`
([world_shot_saber_skins.rs](../crates/sjk-viewer/src/world_shot_saber_skins.rs)),
given the hub's pack through `SJK_TEST_PACKS`, draws the Sun between a stock orange and
a stock blue blade, the Sun close up every 0.3 s, and the Character page's model holding
it; reviewed by eye on one GPU (08/10/2026), and again after the move to blade-skin
files against shots of the earlier built-in Sun (same day), not on other GPUs or in a
live match. The round end, arcs, motes and hue (09/10/2026) were checked by naga's
validation, unit tests of CPU copies of the tip's and the hue's maths (which also check
the shader holds the same expressions) and images rendered by a scratch CPU copy of the
skin shading, side on, on a machine without a GPU; not yet by a world shot. The later
sections (10/10/2026) were checked by naga, unit tests and the world shot
`duel6_second_blades` (the nine new skins standing side by side, each up close at eight
moments, the Spectral's afterimages, the Runic spelling a name, the Banner outside a team
and in red and blue, the Chameleon in three places' light), rendered on one GPU and
reviewed by eye; their cost not measured, not seen in a live match.

## Holstered saber hilts

The actor submission uses the same map-owned `HiltCatalog` meshes for held
and holstered hilts. It creates no second Ghoul2/model instances. Actor-owned
hip bone/back surface indices are resolved on model load, and visible holsters
reuse the existing CPU/GPU pose matrices. Their fixed bolt cache follows rig
retargeting and model scale. Missing left/right attachments are independent;
a missing `*back` tag falls back to the right hip. The bolts are evaluated only
while a stowed hilt is drawn, not while the saber is in hand; the staff handoff
only checks that the model has a `*back` tag. With the feature off (setting,
plugin bit or a server without the contract) an actor costs one flag test. The
contract detected from `gamename` is cached until configstring 0 changes. Shared
hilt object groups reserve two slots for each of 64 saber carriers (32 clients
plus as many NPCs) on initial/dynamic model load; held and thrown hilts may grow
them, while frame submission skips a stowed hilt past that capacity.

[holstered_sabers.rs](../crates/sjk-viewer/src/holstered_sabers.rs) holds the
named JA+ ownership/plugin contract and its player override.
[holstered_saber_pose.rs](../crates/sjk-viewer/src/holstered_saber_pose.rs)
implements the `rfemurX`/`lfemurX` hip bases and `*back` tag, with hip-tag
fallbacks. [holstered_staff.rs](../crates/sjk-viewer/src/holstered_staff.rs)
samples the installed torso override and latches each forward/reverse reach
after crossing the handoff fraction. It does not synthesize an animation.
Clientinfo changes invalidate the latch and give sound synchronization a new
identity; model replacement and map teardown drop actor-owned state.

[audio_holstered_staff.rs](../crates/sjk-viewer/src/audio_holstered_staff.rs)
keeps bounded client slots with separate hilt and blade-skin sound layers,
using the existing adapter's registered handles. Ignition waits for the
reach (150 ms to appear, at most 2000 ms held). Shutdown at the put-away reach
suppresses its late copy for 2000 ms. Only a staff carrier with a back reach
reports a slot. A slot that is disabled or released (death, mount, cloak, an
actor no longer submitted, a replaced player) plays its held ignitions and
any shutdown the reach has not already played instead of dropping them; a
server-time rewind and map changes discard them. Asset decoding remains outside these
frame paths. The settings and visibility rules are in
[client.md](client.md#holstered-saber-hilts).

The reference is JoF EJK #42, #81, #95, #267 (`f21eee8`) and #300: its
`codemp/cgame/cg_players.c` holster placement, visibility, frame latch and
sound rules. The native cache ownership replaces its additional Ghoul2 slots.
Mounted bodies are suppressed explicitly even though that reference's vehicle
check was commented out. Source comparison and unit checks do not establish
in-game visual, audible or populated-server performance parity.

## Blade Lab designs

Fourteen blade skins came from Creyon's Blade Lab, a three.js page in which each was
first drawn by its own shader: Peppermint, Soap Bubble, Lava Lamp, Sketch, Hourglass,
Comic, Quicksilver, Reactor, Obsidian Edge, Bleeding Kyber, Monarch, Phoenix, Serpent
and Quantum. Their skin files describe them with the parts every skin shares, which
came out far from the lab's look (Creyon's review of 17/10/2026, with videos of both).
[blade_lab_designs.rs](../crates/sjk-viewer/src/blade_lab_designs.rs) names them in a
table (`DESIGNS`: the skin, its number, the lab's glow reach `R`, how far its particles
go, whether it has a solid pass, the lab's colour), and while `cg_saberLabDesigns` is on
(archived, default 1; **Blade Lab designs** under Game options in both settings
interfaces) each is drawn by the lab's own shader, ported to `saber.wgsl` (`lab_*`):

- **Units.** The lab works in metres on a 1.08 m blade; a stock blade is 40 units, so a
  unit is 0.027 m (`LAB_METRE`, `blade_lab_designs::METRE`). `lab_place` turns the glow
  quad's place into the lab's: along the blade as on the blade itself (foreshortening
  undone), across and the distance to the blade's axis as on screen. Each blade starts
  4 cm inside its hilt (`LAB_START`), as a stock glow covers the hilt's mouth, and is
  drawn narrower than in the lab by its `LAB_THIN` so it is a stock blade's thickness
  and fits its hilt (Creyon's test, 18/10/2026: a gap above the hilt, blades too thick).
- **The skin.** `LoadedSkin::shown` is the file without its sections (the design draws
  its own parts, and the afterimages, echo, glints and pulse the CPU makes go too) and
  with the lab's colour for its trail and light; its uniform carries the design in its
  last lane (`design`: number, reach, room, solid). Eleven are chromas (all but Phoenix,
  Obsidian Edge and Quicksilver): `lab_turn` turns only the parts that make sense in
  another colour (the stripes, the wax, the sand, the plasma at rest, Bleeding Kyber's
  pure part, Serpent's and Monarch's blade), keeping the red corruption, the snake, the
  butterflies and the overheat. The sounds stay the file's. With `cg_saberLabDesigns 0` the files draw them
  again; the change uploads the skins anew (`sync_saber_skins`).
- **Glow.** `fragment_main` draws a design's glow quad with `lab_glow` (the dynamic glow,
  `fragment_glow`, takes only `LAB_BLOOM` of it, capped, so a white core does not bloom
  into a blob that hides the effects): the
  lab's blade (`lab_blade`, the lab's `fx` for that design, core and all) faded out at its
  reach, and its particles (`lab_motes`: sugar glints, bubbles that pop, wax specks,
  doodled stars, spilled sand, vented sparks, edge sparks, pollen, flames and burning
  feathers, green motes, the probability cloud), each a slot reborn every life at a new
  place, never narrower than a pixel. The core line draws nothing for a design.
- **Solid pass.** `vertex_solid` keeps a design with a solid pass, and `lab_solid` draws
  the comic blade's cel-shaded body in its black outline and its words (VMMMM! on
  lighting and switching off, SHWOOM!/VWOSH!/ZZAKT! on a swing, each swing of a chain its
  own, KRAKK!/SHZAK!/BZZT! on a clash, in a five-by-six font), the quicksilver mirror column
  with its beads and drops, the flat obsidian blade, Bleeding Kyber's drops and the
  monarchs (Felucia's spores), which sit on the blade, lift off gently on a swing, drift
  along it and land again once it is still. A design whose words outlast its blade
  (`blade_lab_designs.rs` `LINGERS`: the comic blade, 1.6 seconds) keeps, from when its
  blade begins going in, a whole-length instance with a swing lane of -1 that draws only
  its switching-off word (`saber_submission.rs`, `lab_lingering`), so the word floats on
  after the blade has gone and can be read. Its vertex stage stands that instance up the
  screen (the camera's screen-up direction with the view direction taken out), so the word
  faces the camera from every side and from above (Creyon's report of 20/10/2026: standing
  upright in the world, it read only from in front and lay edge on seen from above).
- **Motion.** A blade's swing, the seconds since it was lit, since it last clashed and
  since its last swing began reach the shader per instance (`motion`, location 10;
  [saber_motion.rs](../crates/sjk-viewer/src/saber_motion.rs)), kept with the blade's
  trail state: the swing follows the tip's speed about the hilt as the lab's did (a full
  swing at its 5.5 m/s; running with the blade still is no swing), a swing begins above
  an idle stance's sway, only while its wearer's saber is in an attack (`saberMove`,
  `in_attack` or `in_special`: a run's arm sway is none, Creyon's round 14; without a
  snapshot the tip must have turned some 40 degrees within 0.45 seconds instead), and for
  the designs that answer each swing (`blade_lab_designs.rs` `EACH_SWING`: the comic
  blade's words) again every second of a chain without a pause; for the others a chain
  counts from its first swing, a clash is one whose flare latched within 12 units of the blade
  (`impact_spawn.rs` `note_clash`), and a blade seen growing out of its hilt is lit.
  A thrown blade carries on the same state (`thrown_saber.rs` `flight_motion`, 23/10/2026,
  hub report 127): its spin is its swing, measured in the world, and as no attack is known
  in flight the tip must turn for a swing to begin. Menu blades are still.
- **Changed from the lab** after Creyon's tests: each blade has light at its emitter
  (`design_color`, as a stock blade's hilt sprite) so no gap shows above the hilt; the
  hourglass's glass closes in a dome over the tip and its sand runs in 24 seconds, not 8;
  the serpent climbs; the reactor (a mythical) heats over a 30-second cycle (calm,
  heating over 6 seconds, a critical red with a slow strobe, a vent flash and cooling)
  with a coloured core that reads from afar, magnetic field lines winding round it, arcs
  between its rings while hot and a glowing emitter coil; Quantum gained a standing
  wave, tunnelling blips and a collapse flash on each swing, and its ghost copies (an
  option its wearer may switch off: `Ghost copies`, the `copies` section bit) show
  whether it is held or swung; bubbles are larger and brighter; the comic words come
  with swings and clashes only, larger and held 1.8 seconds. The Phoenix's wings,
  which grew out of the blade, are not drawn. The lab's bloom pass is not; its halos are
  the shader's own.

An ignored GPU test (`blade_lab_designs_gpu_test.rs`) renders every design still, mid-
swing and after a clash, checks the blade is lit and the picture's corner dark, and saves
the pictures to the folder `SJK_LAB_PREVIEW` names.

## Blade Lab signature shaders

Creyon's Blade Lab grew into a collection of more than a hundred blades, each drawn by
its own **signature shader** on top of SJK's skin shading, which the lab runs as a GLSL
port of `saber.wgsl` (19/10/2026: the twenty-one mythicals first, then six of his reworked
Blade Lab designs; round 8 replaced Nine Lives, World Tree and Elemental with Carbonite,
the Uneti Tree and World Shift and added the Sith Holocron and the Exogorth: twenty-nine).
A signature is three functions (and two more, below), each handed what the skin's file drew
at that point and returning the blade's own: `sig_glow(g, b, r, px)` the glow's light,
`sig_core(c, b, px)` the core line's, `sig_solid(o, b, px)` a layer blended over the glow;
`b` is the place in units (across, along: 0 at the hilt, the blade's length at the tip),
`r` the stock glow's radius there, `px` a pixel. They read the lab's uniforms: `uTime`,
`uLength`, `uSwing`, `uClash`, `uIgn`, `uAge`, `uSeed`, `uFx`.

- **Effects round the blade (round 8, 19/10/2026).** Built as Sol's own blades are: the
  blade itself is the skin file's glow and core, which `sig_glow` and `sig_core` only
  brighten, tint or mark (so it fits the hilt and fills the emitter as a stock blade
  does), and solid parts (Carbonite's slab, the Sith Holocron's crystal, the Exogorth's
  gullet) are drawn by `sig_solid` over it as Sol's ink and chrome are. Everything round
  the blade is drawn by two more hooks, `sig_vol(ro, rd, px)` (added light) and
  `sig_vol_solid(ro, rd, px)` (a blended layer), handed the view ray in the blade's own
  frame (`sig_ray`: y along the blade from the hilt, z along a direction across it fixed
  to the hilt, x across both): sparks, flames, bolts, rings, puffs of mist and shards are placed
  round the blade in 3D and each is drawn as a soft piece facing the camera (the ray's
  distance to a point or segment, `rayPt`, `raySeg`, `bolt3` in `lab.glsl`), as Sol's
  arcs and the Void's orbiting black holes are; no volume is marched. `sig_edge` fades
  everything out before the quad's edges, and `uFx` is 0 when the wearer turns the
  effects off. `@solid 1` marks a signature with `sig_solid` or `sig_vol_solid`.
  A solid layer is cut just past the tip the blade has (so a crust follows the blade as it
  grows); `@solidpast 1` (`Signature::solid_past`, `SIG_SOLID_PAST_TIP`) leaves a fully
  grown blade's solid layer uncut there, for one that closes itself past the point: Angel's
  cloud bank rounds off in a billowed crown a little above the tip, where the cut drew a
  flat top seen up close (Creyon, 5/11/2026).
  `@nearpull <units>` (`Signature::near_pull`, `SIG_NEAR_PULL`) sets how far a `@solidnear`
  layer's depth is pulled toward the camera (14 by default, so a wide plane does not slice
  its wearer); Angel pulls 3, as its bank covered its wearer's head and face. Its bank is also
  narrow from the hand up to head height and thins away with the camera within some 17 units
  of the blade (Creyon, 6/11/2026).
- **A fixed frame and the full length (round 9, 20/10/2026).** Until round 9 the frame's x
  pointed to the camera's side, so every effect turned round the blade with the view: in
  Creyon's test they raced round it whenever he turned the mouse. The hand path now stamps
  each blade instance with a direction across it that turns with the hilt (the socket's
  direction's orthonormal turned by the weapon's rotation) and the blade's full length
  (`Instance::with_frame`, vertex attribute 11, `frame`); the vertex stage passes the
  direction octahedron-packed in `down.zw` and the length in `axis.w`, and `sig_ray` builds
  the frame from it (a thrown blade or the menu stage, without one, from a fixed world
  direction). `sig_full` places the three-dimensional effects on the full length and
  shows them only once the blade is nearly all out (85 to 100 % of it), so nothing slides
  up the blade while it ignites (it did, fast, with `uLength` growing).
- **Flat as Sol's, only orbits in 3D (round 10, 20/10/2026).** Creyon: Sol's blades "only have
  parts that are 3D". `sig_vol` and `sig_vol_solid` are now handed a flat view (`sig_flat`): a
  ray straight through the blade's picture at its place `b`, from a little above (rings round
  the blade read as ellipses), so everything they draw is a flat picture round the blade
  facing the camera, as Sol's arcs, flares and wisps are, the same from every side. Only what
  circles the blade is drawn along the true view ray in the hilt's frame, by two more hooks,
  `sig_orbit` (light) and `sig_orbit_solid` (a layer): the Eclipse's suns, Obsidian Edge's
  shards, Liquid Beskar's orbiting droplets, Carbonite's floating shards, the Kyber Cluster's
  crystals and the Between Worlds duel. `tools/sig3d.py` in the Blade Lab draws the same way.
- **Each its own, every part switchable (round 11, 20/10/2026).** Creyon: the mythicals all
  had the same lines (the shared beams) and each effect must switch off on its own. A
  signature now also wears Sol's lightning and prominences (`arcs`), souls, sparks and
  starlines (`motes`) and a colour turn (`hue`) from its file, each with its own pattern, and
  no longer shares the beams. Its options (`Signature::options`, at most six): its own parts,
  named by `// @part` header lines and read in the shader with `part(k)` (the wearer's
  choices ride on section bits a signature never uses, `SIG_PART_BITS`, gathered into
  `uParts` by `sig_take`), then Sol's sections its file has (named by `// @arcs` and like
  lines), then the swing and clash bursts (the pulse bit). A section named like a part already
  listed joins that part's switch (#115: Holonet Feed's own flecks and its file's motes were both
  "Data flecks" and listed twice; a test checks no signature lists a name twice). An id that
  hashes onto a taken choice bit gets a number. The old `Effects round the blade` and `Ignition show` are gone.
- **Nothing moves with the view (round 11).** Effects still raced when the view turned, the
  wearer ran or the blade lit: many clocks ran at a rate that followed the swing (`uTime` times
  `a + b × uSwing` jumps the phase by the clock times the change), and turning the view or
  running turned the blade about its hilt in the world, which counted as a swing. The swing
  is now measured in the wearer's own frame (`BladeMotion::record_turned`, the blade's
  direction turned back by the body's rotation), no clock's rate follows the swing, and
  `uLength` is the full length from the start (`sig_take`), so nothing placed along the blade
  slides as it grows.
- **Lower tiers and Sol's pieces (round 11).** A signature keeps its file's echo, glints
  (`star`, `orbit`) and veins as well as its afterimages, haze, wisps and embers, offered as
  options, and the built-in file's ones win over an older pack file (`Signature::adopt`). The
  legendaries use them with fewer flat effects than the mythicals; `lab.glsl` adds `lvl`,
  `recol`, `bands` and `fxFall` (rain, snow, streaks) for them.
- **Round 12 (Creyon's test of round 11, 19/10/2026).** What a signature's glow draws past
  the tip it has grown to waits for the blade to be out (`sig_full`; Star Dragon's coils stood
  at full length while it lit). A signature's afterimages are kept in its wearer's own frame
  (`GhostSpec::wearer`, `GhostTrail::instances_placed`): turning the view or running leaves
  none, only the blade's own swing does. Sol's arcs and motes on a signature fade out at its
  room instead of the stock glow's width, which had cut its prominences off (`faded` in the
  arcs' block; their tip leaps keep their old reach). A signature's smoke (wisps) uses finer,
  more curled noise with a soft edge (it looked square), and `lab.glsl` adds `fxSmoke` (soft
  flat smoke), `fxStreams`, `fxFlakes` and `fxRingWaves`. `uKeep` is light a signature adds
  in its own colours after a chroma's turn: Between Worlds' white and red duel and Bleeding
  Kyber's red corruption (which is now a chroma: its pure part takes the wearer's colour) keep
  their colours. Creyon's blades lost their clash rings, their tip glints and the rings that
  were not their idea; the legendaries were reworked with three or four switchable effects
  each, and Wraithlight's aura softened. A signature's swing (`uSwing`) now counts only a real swing
  (a smoothstep from 0.45 of the measured swing): the arm's sway while running and the torso
  following the view had stirred the effects that answer a swing.
- **Round 12b (Creyon's second test, 19/10/2026).** Sol's lightning and prominences (`arcs`) on
  a signature are drawn on two sheets through the blade's axis fixed to the hilt (`sig_arcs`,
  `sig_ray`'s frame) instead of the picture facing the camera, which kept them on the same side
  of the screen whatever the view, so they swung round the blade as he turned or ran; a sheet
  seen edge on fades and the other carries them. The ignored GPU test's `SJK_SIGNATURE_ORBIT`
  renders five views from a moving camera to check that effects stay with the blade. A
  signature's echo is its blade alone, faded (it drew its own effects at full strength and cut
  off). Smoke: `fxSmoke` and a signature's wisps (`sig_rising`) use broad, turned octaves so the
  value noise's grid never shows, and draw none over the tip. Kyber Cluster's crystals are
  hidden behind the blade. Star Dragon became the Ashla Radiance blade (same id), Balance was
  redrawn as a silver blade between a ribbon of light and one of violet shadow, Ilum Aurora
  Storm's curtains are soft sheets instead of bars, Between Worlds and Wraithlight are calmer.
- **Sol's Sun (20/10/2026).** The Sun blade (001) became a signature of Sol's own,
  `sun.glsl`, over its file's corona, flares and tongues. It is the first to answer the
  blade going out: `uAge` at or past `GOING_OUT` (2000, `saber_motion.rs`) cools its
  glow and core to a deep red sunset as the blade slides in, mirroring the sunrise it
  lights with (deep red warming to white gold over 0.9 s, a white edge climbing at
  `uLength × uIgn`). Its dawn flare waits in `sig_vol` until the blade is out (`sig_full`).
  Coronal rain (`su_rain`) replaced `fxRays` (Sol: no straight spikes): three loops arch
  off the blade with six beads each sliding down both legs. A clash draws an eclipse
  (`su_moon`): a disc darkens glow and core where it lands, with a ring and an `fxFlare`
  diamond at its edge near a third of the way through.
- **Round 15 (Creyon's review of 20/10/2026).** No two blades share a look: twelve blades that
  drew the Aurodium's wavy strands (`fxStreams`, which the Aurodium keeps) got their own effect
  instead: Balance the scales of the Force (a beam tipping slowly, a sun of light on one end and
  an orb of shadow on the other), the Jedi Spirits three spirits climbing round the blade in a
  spiral (drawn in 3D, `sig_orbit`), Dathomir Blood Moon a red moon with a shadow sliding over it,
  Force Echo a scale of time marks lit one after another, Hyperlane lane beacons in a chase,
  Mustafar Eruption straight lava runnels, Naboo Core rising plasma globes, Royal Naboo golden
  scrolls of embroidery, Scarif Tide waves rolling up it, Unknown Regions a far spiral galaxy,
  the Ashla Radiance soft rays of light (`fxRays`), the Kyber Cluster and the Beskar Forge none
  (a glint on the forge's steel). Hyperspace's coils turn slowly (two at 0.22 turns a second,
  were four at 1.4) and its tunnel twists slower; its and the Ashla Radiance's effects take the
  wearer's colour with the blade (`sig_vol` returns them rather than adding them to `uKeep`).
  Between Worlds' duellists are bigger, further out and brighter. Felucia's spores each live a
  cycle on the clock alone (grow on the blade, rest, lift off smoothly and drift away, fade),
  so running or a chain of swings can no longer snap them back (they read `uSince`); it is no
  longer a chroma. The Bleeding Kyber's red dots became drops that swell and fall in streaks and
  hot embers arcing off its front. The Star Map draws a different figure each chart (a route, a
  ring, a triangle, a spiral, a diamond, a crown, a lightning route). Other blades that looked
  alike were parted: the Superlaser and Witchfire lost the green lightning (their file's `arcs`)
  that made them look like the Sith Alchemy blade, the Kashyyyk Monsoon (falling leaves instead)
  and Bespin Storm lost the lightning the storm blades shared, the Starforge's flames are a star's
  white-gold rather than the Phoenix's orange, the Nebula Song's rings of song (the Singing Kyber
  has rings) became clouds of rose, violet and teal, and Hyperspace lost its coils (the Reactor
  and the Uneti Tree have coils). Fifteen blades whose glow hid their effects draw it at 60 to
  80 % (a `sig_glow` wrapping their own): the Reactor, Hyperspace, the Dyson Sphere, the Kyber
  Heart, the Bleeding Kyber, the Event Horizon, the Nebula Song, the Sith Holocron, the
  Starforge, the Uneti Tree, the Witchfire, Mustafar Eruption, Hoth Avalanche, Yavin Vines and
  Twin Suns. Legendaries richer than the rares (Creyon: each tier up must look better): the
  Assassin Droid adds red targeting lasers that lock on and two armoured gear rings turning
  round it (3D), Liquid Beskar waves of forge heat in its mirror, glowing edges and molten drops
  off its tip, the Beskar Forge a quench after each strike (jets of steam, an afterglow ring)
  and more sparks; the Superlaser's surges swell into balls of plasma and a charge gathers at
  its focus. Of the rares only the Data Disc changed (Creyon: it looked cheap): three discs
  turning, the middle one the other way, with hubs and written sectors, and packets of data
  streaming up between them. Round 15b (Creyon: mythicals must not look cheap, Balance for
  one): Balance's blade is split, white-gold on one side and violet on the other, with rays of
  light fanning out of the light's side, tendrils of shadow off the dark's and larger scales on
  chains; the Ashla Radiance opens two wings of light; three wraiths dart round and through the
  Wraithlight (3D, with its mist and echoes option); the Jedi Spirits glow like a Force ghost.
- **Round 16 (Creyon's review of 20/10/2026).** Effects round the blade are Sol's way: what circles
  it is drawn in true 3D (`sig_orbit`), so it reads from every side. Sol's motes on a signature run
  straight on past the tip (along and across from `blade_place`) instead of round the capsule's
  end, whose seam cut a mote crossing the tip in half (Witchfire, Uneti Tree, Ashla Radiance,
  Nebula Song, Kyber Heart). `Signature::adopt` takes every figure, lab and solid section from the
  built-in file too: the hub pack's older additions for a hand-ported design (Death Star Plans'
  pencil sketch, drawn from inside the hilt) reached the signature and ran through the hilt in
  third person and the preview. Felucia's spores answer a new motion lane, `stir`
  ([saber_motion.rs](../crates/sjk-viewer/src/saber_motion.rs) `BladeMotion::stir`, for the
  designs in `blade_lab_designs.rs` `STIRRED`): how much the blade moves in the world, running as
  much as swinging, rising in 0.3 s and falling in 1.4 s, so the spores rest on the blade while
  idle, swarm round it in 3D following it while it moves, and settle back smoothly. Mortis's realm
  turns every second time its ghost blades fold back into it (8 s a fold, a flash as it turns).
  Hyperspace's coils are back, three, turning at 0.25 a second in 3D. Between Worlds follows a
  picture of the World Between Worlds' paths: a translucent blue blade with sparkling edges and a
  dimmed core, paths of light crossing the void, nebula clouds, the duel kept. The Holdo Maneuver
  gains engine glows at the hilt that spool up before the ram; Wraithlight cold spirit fire;
  the Ashla Radiance orbs of light circling it; the Bleeding Kyber lightning that crawls over the
  bleeding part and flickers fast (fewer, forked bolts lashing out) and, on its pure part, beams
  of light and sparks of both colours where they meet. Legendaries: Balance redone (two faces
  turning round a grey blade, bands of light and shadow passing, a wave of each meeting in a
  flash, a sun and an orb of shadow circling opposite each other), Twin Suns' sunset beams, Exegol
  Rift's vortex of the dark side (3D) with bolts into the blade, Scarif Tide redone (sunlit
  caustics, a waterspout in 3D, bubbles, spray), Royal Naboo a jewelled circlet in 3D instead of
  the costly scrolls, Starkiller Frost at 65 %, Krayt Pearl redone (nacre, pearls in 3D), Unknown
  Regions a hyperspace storm in 3D instead of the galaxy, Blood Moon its streams back with the moon
  circling it in 3D, Liquid Beskar tendrils of liquid metal instead of forge heat, Beskar Forge
  molten beskar poured down it instead of side steam. Rares: Sith Thorn (veins, a thorned vine in
  3D), Jakku Sandstorm (a dust devil in 3D), Dac Reef (a school of fish in 3D), Puzzle Holocron
  (a holocron cube in 3D), Christophsis Crystal (crystals all round in 3D, facets).
- **Round 17 (Creyon's game test of round 16, 20/10/2026).** Sol's motes on a signature fade out
  over the blade's last units, none past its tip, and stay within 3.2 units of it (they rose far
  above the tip and went through the wearer's head: Wraithlight, Uneti Tree, Witchfire, Ashla
  Radiance, Singing Kyber, Nebula Song, Kyber Heart, Exogorth, Event Horizon, Big Bang, Ilum
  Aurora Storm, Abyssal Eye, Sith Alchemy). The Jedi Spirits' climbing spirits stop below the tip
  and a spirit's trail is its own climb only (it reached back to the end of the last, which
  stuttered at the tip). `lab.glsl` `bladeSeen` hides what circles the blade behind it (Balance's
  orbs, Blood Moon's moon, Krayt Pearl's pearls showed through it); `fxSmoke` is drawn out to 2.7
  widths (it was cut at 1.8, where its sides showed as straight lines: Beskar Forge's quench
  steam, Twin Suns' heat). Twin Suns' sunbeams fade out well inside its room, without dust. Scarif
  Tide's sparkles twinkle smoothly (they jumped 8 times a second) over one web of caustics;
  Between Worlds' edge glitter likewise. Liquid Beskar's sparks are silver; Beskar Forge has no
  pour (it drew a line above the tip). Royal Naboo: interlaced filigree round a crimson thread and
  a golden silk veil in 3D. Dac Reef redone as one coral: a porous stone body over the light
  (`sig_solid`), its pores showing the blade, polyps breathing in their rims. Puzzle Holocron
  redone as a lattice of panels turning row by row round its core. The Deflector Shield's dome
  closes over the tip. No figures in Creyon's blades (Dac Reef's fish, Puzzle Holocron's cube
  and Royal Naboo's jewels are gone).
- **Round 18 (Creyon, 21/10/2026: soften, do not remove).** The Bleeding Kyber's pure part draws its
  light at 45 % (was 70 %), its aura, beams and front at about half, and its red corruption
  stronger, so the pure light no longer grows over the red part. Every layer of Mustafar Eruption
  is kept and softened (flames, drips, sparks, fountains, ash); Event Horizon's rings, Exegol Rift's
  storm, dust and vortex bolts, Wraithlight's spirit fire and frost, Beskar Forge's flames,
  Starkiller Frost's mist and Holdo Maneuver's engines and haze are each a little softer.
- **Round 19 (Creyon, 21/10/2026: everything in 3D, nothing cut, mythicals more mythical).**
  `sig_vol` and `sig_vol_solid` now take the true view ray in the hilt's frame, as `sig_orbit`
  does (Creyon, from above: flat effects followed the camera), so every effect round the blade is
  the same thing from every side; `sig_flat` is gone. What was laid out in the blade's flat
  picture now stands round the blade: `fxSmoke` is a cloud marched in six steps, `fxStreams` and
  `fxFlakes` go all round the blade, a new `turnY` turns a piece laid out in the picture to its
  own place round the blade (Between Worlds, Data Disc, Force Echo, Hyperlane, Imperial
  Schematics, Jedi Texts, Holdo Maneuver, Moisture Farm, Mustafar Eruption, Sith Thorn, Star
  Dragon's wings in a shallow V, Victory Fireworks as spheres), Star Map's figures wrap round a
  cylinder, the Deflector Shield is a true shell with a dome, Ilum Aurora Storm's curtains hang on
  cylinders, Temple Sunbeam's and Twin Suns' shafts and Jakku's and Bespin's gusts are slabs and
  streaks at their own depths. Nothing is cut at the quad's edge: `uReach` (set by `sig_take`) is
  how far the quad shows an effect in full, and the lab's pieces fade (`roomFade`) or stay inside it
  (Reactor's shock ring was cut where it left that room; it now races out to 6.5 units). Sol's
  arcs that do not leap from the tip fade out over the blade's last units, so no prominence passes
  the tip (Eclipse, Twin Suns, Dyson Sphere); Eclipse's own loops end below it. The ignored GPU
  test has a cut-off check (`SJK_SIGNATURE_EDGES`: each moment drawn again without `sig_edge`, the
  difference marked) and closer orbit views with two from high above; run over all 128 it found only
  Kessel Spice's crystals, drawn out to the quad's edge, now kept inside its glow. Every mythical signature
  (`SIG_MYTHICAL`, set by `Signature::module_source` from the catalogue tier) awakens when lit (a
  surge up the blade, a flare and a ring at the tip), breathes (a slow wave of light up it) and
  rings out on a clash, in its own light's colour (`sig_mythic`). Blades: Mortis's effects reach
  down to the hilt (ghost blades from low on it, a second Father's ring, the Daughter's beams, the
  Son's red smoke); Liquid Beskar's tendrils became rings of liquid beskar rippling up it (beskar
  has none); Yavin Vines and the coils and streams of Aurodium, Blood Moon, Event Horizon and Uneti
  Tree start at the hilt; Jakku's dust devil turns a third as fast; Bespin Storm's gas giant bands
  swirl round it as a cloud with tibanna gas rising; Starforge draws the star down a funnel into the
  forge (star siphon) and throws a forge nova every fourth blow. Kyber Heart and Christophsis
  Crystal are chromas. The Clone Wars design (`lab_comic_*`) gets five parts its wearer can switch
  off: a see-through comic panel of halftone dots and speed lines behind a swing, action bursts of
  impact lines on the blade (no bubble behind the words, which stay as they were), clone markings
  (212th orange and 501st blue stripes sliding up its body), a holo flicker (a second in nine as a
  blue hologram) and cartoon smoke puffs and spark stars (with its particles). With its comic panel on, its swing
  trail is comic speed lines instead of the blur (`saber_trail.wgsl` style 4,
  `SkinColor::comic_trail`): tapered streaks in the blade's colour, more toward the tip, and a bold
  line where the tip went (Creyon: comic lines instead of afterimages). A signature's three
  pipelines are built side by side when a blade needs them now (`saber_gpu.rs`), so effects
  come in sooner when a blade is first worn.
- **Round 20 (Creyon, 21/10/2026: the pictures, the glow up close, the launch).** The
  Collection's pictures look from 20 000 units away, and the effects' sums lost their precision
  there and smeared across the blades (bars, dashes, things the blades do not have): `sig_ray` now
  starts a far camera's ray a hundred units before the blade. The pictures are seen a little from
  the tip's side, as before round 19, so rings read as rings, and their effects' frame is a
  wearer's. That frame (`saber_submission.rs`) is now the wearer's forward square to the blade
  (the hilt model's own only as the blade comes round to point forward), so what spreads to a
  blade's sides spreads to the wearer's left and right (Ashla Radiance's wings were one at the
  side and one elsewhere). Close up, a signature's glow round the blade (not its core) softens to
  about half within a few metres (`near`), and a signature reaches the bloom at 55 %
  (`SIG_BLOOM`), so its effects are not hidden behind its glow. Abyssal Eye's iris and pupil are
  a true eyeball in three dimensions (`ae_eyeball`), watching whoever looks at it. Hyperlane,
  Nebula Song, Deflector Shield and Quasar are chromas. Signature pipelines are built by two
  threads from a queue (`BuildQueue` in `saber_gpu.rs`): a blade worn or in the preview first,
  then the Collection's pictures (until 21/10/2026 also every loaded signature from launch, resting between those
  (round 19's four at once stuttered the game at launch; with every build alike the preview's
  waited behind the pictures and most blades seemed not to load); since 21/10/2026 a signature is
  built only once a blade draws it). A skin's glow/core pair is likewise uploaded only once a blade
  draws it (`Runtime::upload_drawn`): all 165 of the hub's skins up front were about 45 MiB of
  video memory on every world (128-pixel generated glows). A signature now builds only its
  glow pipeline and, with a solid layer, its solid one; its bloom is the shared pipeline's, its
  blade's glow alone. The queue and the pipelines are the graphics context's
  (`saber_gpu::Signatures`, held by `gpu_context::Context`), shared by every world built on it:
  until 21/10/2026 each map's world started its own queue and threads (which never ended) and
  built every loaded signature again, about 130 builds a map change.
  Also in round 20: Abyssal Eye's reality cracks tear out and fade where they reached instead
  of shrinking back into the blade; Puzzle Holocron's rows read (darker panels, a tone a row,
  bolder seams, a softer core); Thermal Fuse is a braided gold cord burning down from the tip,
  ember red and smoking behind the point, popping at the hilt; Senate Glass's blade is itself a
  stained-glass column (`sig_solid`) with coloured light round it; a signature's veins start at
  the hilt's mouth (`sig_from_mouth`: Cockpit Console's circuits reached below the hilt);
  Takodana Rain gets rings rippling out where drops land and Starfighter Canopy slanting
  streaks and landing lights, so the two no longer look alike; Victory Fireworks is a chroma whose
  fireworks keep their colours.
- **Round 21 (Creyon, 21/10/2026: loading, Clone Wars in 3D, fixes).** Signature modules are
  slimmed before they are built (`slim` in `blade_signatures.rs`: the designs' glow and solid
  pass emptied, as a signature skin is never a design, and every function its pipelines do not
  reach left out), a signature builds only its glow pipeline and, with a solid layer, its solid one,
  and the build queue runs two or three builders (Creyon: at launch the effects took minutes to come
  in). A skin reload (entering a server) keeps every signature already built, whatever number it
  loads at (Creyon's log: 233 builds for 128 blades). The Clone Wars blade's toon body, its cartoon smoke and its spark stars are drawn in three
  dimensions along the true view ray (`comic_body3`, `comic_puffs3`); its words, panel and impact
  lines stay as they were. Sith Thorn's petals, Otoh Gunga's bubbles, Cantina Lights' spots of
  light, Holo-Recording's dashes and Imperial Schematics' rungs no longer lie in one flat sheet
  (seen edge on they were slivers or hid in the blade). Puzzle Holocron's lattice is a solid shell
  over the core with its turning row lit up; Senate Glass's panes light up row by row as a wave of
  sunlight rises.
- **Round 22 (Creyon, 21/10/2026: nothing seen through the blade).** The lab's `rayPt`,
  `raySeg` and `ringD`, the signatures' own ring helpers, their rings across the blade
  (`rayAt`), `fxSmoke` and the Clone Wars puffs, stars and mythical rings in `saber.wgsl`
  (`ray_behind_blade`) hide whatever lies behind the blade's body along the ray (`behindBlade`);
  what lies on the blade itself is never hidden, and bounds and the blade's own axis use the raw
  distances (`rayPtRaw`, `raySegRaw`). Liquid Beskar's rings no longer show through the blade, and
  the Clone Wars halftone no longer reaches below the hilt.
- **Round 23 (players' reports after the 2026.1021.1 release: pixels at the tip, hard lines).**
  A skin's core is sampled with the derivatives of its coordinates before the rounded tip narrows
  them (`core_coordinates_at` with `core_breath`; the shaft's, where the two are the same): in the
  tip the narrowing runs to a hundred times within a pixel and picked the core image's smallest
  mip, which drew a bright rim of pixels round the end of every signature's core, in the game, the
  Collection and the holocron cards. A signature's core fades out before its quad's sides
  (`skin_core`): the core image's fringe is still a tenth as bright at its border, which drew a
  straight edge along both sides of the line up close and a faint box past the narrowed tip. Round
  22's hiding behind the blade is soft: `behindBlade` (and `ray_behind_blade`) fades in as the view
  turns from along the blade instead of switching off at once, and ends before the tip where the
  blade narrows; a hidden piece's distance goes through `hideD` (`hide_d`), which narrows and dims
  it over the blade's soft edge, instead of adding a thousand times the hidden share, which cut it
  at a hard edge and drew a box round the tip whenever a blade pointed at the camera; the
  signatures' own ring code and `bladeSeen` do the same. `fxFlare` is skipped only where its widest
  halo has faded (it ended at a hard ring), and Hypermatter's and the Uneti Tree's pulses fall off
  over their fronts instead of ending at a line across the blade, as does the Starfighter
  Canopy's. The other early-outs that cut glow at a line wait until it has faded: `fxFlakes`
  (a dome seen along Yavin Vines), `fxCoils`, `fxRays`, `fxArcs` and `fxFall` in the lab, and
  Balance's, Holo-Recording's and Orto Plutonia's own; `fxSmoke` takes a step at most about a
  third of the cloud's width, up to 12 (a long path along the blade was sampled six units
  apart). The blocky designs keep their look with edges smoothed over about a pixel (`aaStep`,
  `cellMix` and `aaSquare` in `lab.glsl`): Code Cylinder, Canto Bight, Flimsiplast, Death Star
  Plans, Cargo Stack, Holonet Feed and Targeting Display. Code Cylinder's and Flimsiplast's
  stripes still alias seen right along the blade.
  Sol's review (22/10/2026): the softened pulses are scaled back to their old peak (Hypermatter's
  and the Starfighter Canopy's by 2.22, the Uneti Tree's second pulse 0.96 for 0.6), as their
  fronts' fall-off had dimmed them to 45 and 63 percent. The hiding behind the blade no longer
  ends 1.4 units before the tip (Exegol Rift's tip flare and Quasar's jet showed through it): its
  width narrows with the rounded tip (a quarter circle over the last 1.4 units) to the point, so
  nothing behind the tip shows and no dark round end is drawn past it. The tip fix needs the
  core's coordinates without the tip's narrowing as well (`filter_uv`), on every skinned blade,
  not only the signatures. Both come from one skin lookup per pixel (`core_line`, which also
  gives the glitch's colour split): the first version ran `skin_at`, `skin_wave`, `skin_glitch`
  and `skin_jitter` a second time for them, which the timing below counts. In
  Creyon's game (third person and free camera, in front of a wall) Obsidian Edge still drew a box
  round its tip: its edges' orange bloom stopped at once level with the tip (`inside` was a step;
  it fades round the tip and into the hilt now), and its heat haze showed a stale strip cut at
  straight edges, as the haze (and a black hole's lens) bends the effect layer's `original` image
  up to 2 (6) percent of the screen outside the effects' rectangle, where that image keeps an
  older frame: the rectangle (`effect_bounds`) now has that much more room on every side on a
  frame that draws a haze or a lens (`HAZE_REACH`, `LENS_REACH`, `Bounds::bend`); other frames
  keep the tight rectangle. Hypermatter's pulses come out of the hilt softly too.
  The timing (Creyon's RTX 5070, Windows 11, `signature_gpu_cost_against_stock` with the 12
  heaviest signatures, 33 blades a frame, `main` and this branch timed in turns, twice each, the
  lower kept): the first version cost the 12 together 12 to 19 percent more than `main`, most of
  it in the four arc blades (Sith Holocron, Quantum, Carbonite, Reactor, a fifth to a third more),
  as `fxArcs` drew every bolt wherever its arc's glow could reach. Each of an arc's three bolts is
  now drawn only near its own middle, its glow made to end 1.7 past it (`arcGlow`); `fxFlare` is
  cut at 12 sizes (18 cost too much) and `fxSmoke` at 12 steps (16 cost the smoke blades a sixth
  more seen along them). After that the 12 cost 2.0 (game), 1.8 (close) and 2.8 (along) percent
  more than `main`, with the stock blade's own time (the stand-in) 3 to 4 percent more, so about
  the same: Sith Holocron, Quantum and Carbonite 7 to 19 percent less, the others within a tenth.
  Reactor still cost 13 to 17 percent more: its lightning was worked out while weighted to
  nothing (cold and still, over half its cycle), and the arcs' outer bound used the arcs' asked
  reach, not the reach the blade's room caps them at (`min(reach, 0.6*uReach)`). With both
  fixed, timed again the same way: Reactor costs 3 percent less to 1 percent more than `main`,
  Sith Holocron, Quantum and Carbonite 7 to 19 percent less; the stock blade's own time is 3 to 4
  percent more in every run (the tip's second skin lookup and the softer hiding). The second
  lookup was removed after this timing (`core_line`, Sol's review of 24/10/2026); not timed again.
- **The big effects (round 9).** `lab.glsl` holds shared pieces every mythical draws its
  own way (flat since round 10): `fxRays` (beams shooting out of the blade and fading),
  `fxArcs` (Storm-like lightning leaving the blade, arcing out on any side and coming
  back, with a fork), `fxSparks`, `fxCoils` (strands of light wound round it with knots
  running along them), `fxMotes`, `fxShock` (a ring racing out), `fxFlare` (a star facing
  the camera), `fxAura` and the ring distance `ringD`. Each mythical's round-8 effects are
  kept (`r8_vol`) and its own layer of these added on top (`sig_vol`).
- **Source.** The lab's GLSL is the source, kept in
  [blade_signatures/](../crates/sjk-viewer/src/blade_signatures/) (`<name>.glsl`, the lab's
  helpers in `lab.glsl`, which also holds the shared lightning bolt, Storm-like arc loop
  and helix helpers) beside its WGSL, which
  [scripts/blade_signatures.py](../scripts/blade_signatures.py) writes (`--check` fails
  when one is out of date). The script reads the lab's GLSL subset, infers every
  expression's type and writes readable WGSL (scalar builtin arguments splatted, `mod`
  a floor modulo, `?:` a `select`, `out` parameters pointers, assigned parameters local
  copies, swizzle assignments split, names `saber.wgsl` already takes renamed). Each
  translation was checked against naga's own GLSL front end: every signature rendered
  through both over a grid of blade places at four moments gave the same colours
  (largest difference 0.001), on Linux with lavapipe.
- **Drawing.** [blade_signatures.rs](../crates/sjk-viewer/src/blade_signatures.rs) names
  the signature skins (`SIGNATURES`); a loaded skin with one carries `LAB_SIGNATURE`
  (100) in its `design` lane, with its `@room` (units, in the lane's metres; its solid parts are cut at the tip the blade has
  grown to, and once it is all out only past its rounded tip, 27/10/2026) and `@solid`
  (`@speed <factor>` sets how fast a signature's motion runs, `uTime` through `SIG_SPEED` in
  `sig_take`, 1 by default; its cycles and moments (`uClock`), lighting, swings and clashes keep
  real time)
  (`@moments own` keeps a mythical out of `sig_mythic`: the Sun draws its own sunrise and
  eclipse),
  and its chroma's own hue in `design_color.w` (-1 for none). A signature replaces a
  hand-ported design of the same skin (Monarch, Obsidian Edge, Quantum, Quicksilver,
  Reactor, Serpent, Peppermint, Soap Bubble, Lava Lamp, Sketch, Hourglass, Fiber Optic,
  Phoenix and, since round 9, the Bleeding Kyber now draw Creyon's reworked lab shaders;
  round 9 rebuilt the Reactor, Mortis and Bleeding Kyber Sol's way and gave Obsidian Edge
  and Liquid Beskar their pieces in 3D). `saber.wgsl` draws it by
  the skin's own passes, as the lab does: `skin_glow_at`, `skin_core` and
  `fragment_solid` each hand their result to the signature (`sig_take` sets the uniforms
  from the instance: its clock and seed, length, `motion`'s swing, the clash fading at
  2.2 a second, the age, and the ignition over its first 0.42 s), the glow quad reaches
  the signature's room and the past-glow shortcut is off for it, its core never runs past
  the tip, and the file's own glow stays round the emitter as a stock blade's covers the
  hilt's mouth. The `saber.wgsl` every other blade uses holds stand-ins that change
  nothing between `// @signature begin` and `// @signature end`; a signature skin's own
  module is that file with them replaced by `lab.wgsl` and its own (`module_source`).
- **The place on the blade.** The glow quad's `blade` coordinates are measured as if the
  quad were seen square on; under a close perspective (first person, looking along the
  blade) they stray far from the blade itself, so the lab's blades started well above the
  hilt and ran on in a line past the tip there (Creyon's test, 19/10/2026). The quad now
  passes the view ray and the blade's axis (`ray`, `hilt_at`, `axis`, locations 13 to 15),
  and `blade_place` finds the true place: the point on the axis nearest the ray (along) and
  the ray's distance from it (across). The signatures and the hand-ported designs
  (`lab_place`, which no longer undoes a foreshortening, and whose blades now start 1
  unit inside the hilt, `LAB_START`, as a stock core does) use it.
- **Colour.** A chroma signature takes its wearer's colour outright (`sig_recolour`):
  every coloured part becomes the wearer's hue, at least well saturated, and only its
  white-hot parts stay white; turning the hue round left the lab's many-coloured and pale
  blades hardly changed. Most are chromas; Balance, World Shift, the Sith Holocron, the Exogorth,
  Felucia Spores and Force Echo (round 15) keep their
  own colours, as do Obsidian Edge, Quicksilver, Mortis (day and night) and the Bleeding
  Kyber (its red corruption). A signature's solid layer may keep its own colours where it
  sets `uOwn` (0 to 1; the Abyssal Eye's amber eye, round 9).
  `uTurn` is the chroma's turn (0 with none), for a blade that colours parts of itself under an RGB
  (Lotho Minor Lavaglass). `liveArcs` draws lightning that moves every frame and fades in and out
  (`fxArcs` holds a shape and swaps it); `tipFade` and `tipFadeB` end effects in a soft dome round the
  tip, where a cut on height alone would show as a straight line.
- **Options.** Every signature skin offers its wearer `Effects round the blade` (off: only
  the blade itself, nothing past a stock glow's width or the tip), `Swing and clash
  bursts` (off: swings and clashes set nothing off) and `Ignition show` (off: lit at
  once), `Heat haze`, `Smoke` (wisps) and `Embers` when its file has them, which the
  renderer draws round it as round Sol's blades (round 9: they were stripped before, so
  the Starforge never had the Sun's heat), and `Afterimages` (`Signature::shown`); a hand-ported
  design offers `Particles` and `Swing and clash bursts` (and Quantum its ghost copies).
  They ride on the section bits these skins have no other use for (arcs, motes, hue).
- **Pipelines.** [saber_gpu.rs](../crates/sjk-viewer/src/saber_gpu.rs) builds every loaded
  signature skin's glow, bloom and solid pipelines from its module one after another on a
  thread of its own as soon as the skins load (`sjk-blade-signatures`), so a blade worn
  later is ready; one a blade wears before its turn comes is built at once on a thread of
  its own. That skin's run of instances is drawn with them once they exist; until then
  the shared pipelines draw it from its file alone. The log names how long each took, or
  why it failed. `cg_saberLabDesigns 0` turns the signatures off as it does the designs.
- **Time.** A signature never speeds its clock up with the swing (`uTime` times a rate
  that follows `uSwing` jumps the phase by the clock times the change, and the game's
  clock runs on for minutes: Starforge's forge raced, 19/10/2026); swings change how much
  is shown, not how fast.
- **Files and sounds.** A signature skin whose file is in no pack is drawn from the file
  built into the client (`<name>.bladeskin`: the lab's base colours with the signature's
  `@palette`, and a heat `haze` for the hot ones: Starforge, Quasar, Dyson Sphere, Star
  Dragon, Superlaser, Witchfire, World Shift and Big Bang), so they show before the hub's
  pack carries them. Its sounds are at
  `sound/sjk/sabers/<name>/`, synthesized by the Blade Lab's sound scripts; until a pack
  carries them only the stock sounds play. A pack's older file of a skin that is now a
  signature keeps its images and colours, but the sounds, haze, wisps and embers are always
  the built-in file's (`Signature::adopt`, round 9: the Sith Alchemy still hissed with the
  snake design's sounds from the hub's pack).

Two ignored GPU tests render the signatures through their own pipelines and real glow
and core images: `blade_signatures_gpu_test.rs` still, mid-swing, after a clash,
igniting and green for a chroma, checking the blade is lit and the corners dark, saving
the sheets to `SJK_SIGNATURE_PREVIEW` (`SJK_SIGNATURES` picks some); and
`blade_views_gpu_test.rs` each signature and design beside a stock blade side on, from a
third-person camera and from first person looking along the blade, saving to
`SJK_VIEWS` (`SJK_VIEW_SKINS` picks some).

`python scripts/blade_cost.py` ranks blades by GPU cost (31/10/2026, to find the top-tier
blades that eat the frame): it runs `signature_gpu_cost_against_stock` once per scene
(`game`, `close`, `along`) with `SJK_COST_TIERS` (Legendary and Mythical by default,
`--tiers all`, or `--ids`) and writes `target/blade-cost/blade-cost.md`, each blade's
milliseconds per frame and multiple of the stock blade in every scene, worst first, beside
the CSV. With `SJK_COST_TIERS` the test measures blades with no signature too (drawn by the
shared pipelines, their files from the hub's packs in `SJK_TEST_PACKS`; without them they
are named as skipped), and every blade's frame draws the passes its file adds as
`saber_gpu.rs` does: solid, black hole horizon and lens, heat haze (into the same target,
bending a blank scene image, where the game draws haze and lenses into the effect layer in
their own pass). Left out: the glow pass (one shared pipeline for every skin), trails,
afterimages and lights. GPU timestamps on a non-exclusive GPU: compare blades within one
run.

First optimizations from it (31/10/2026), each leaving every signature sheet (still, orbit and
tip, 132 blades) within 1/255 of `main`'s:

- `behindBlade` (how far the blade's body hides a piece) ran inside every `rayPt` and `raySeg`,
  a few hundred times a pixel on the heaviest blades, though most of it depends on the ray
  alone. `saber.wgsl` now calls `labRayPrepare(ray.o, ray.d)` before a signature's volume and
  solid parts; the lab keeps the ray's part in `var<private>` variables (the translator writes
  a non-`const` module variable as one) and `behindBlade` reads it for that ray, returning at
  once where the ray never passes behind the body; another ray (a shifted one) goes through
  `behindBladeFull`, the old body. The cached path keeps the old multiplication order: `hideD`
  divides by `1 - h`, which magnifies the last digit a thousandfold where a piece is nearly
  hidden (a reordered product moved Big Bang's clash ring contour by 31/255).
- World Shift drew at a third of its speed with the cache on an RTX 5080 (Vulkan), and still
  did with the cache switched off by a constant or reached through a wrapper, though the same
  arithmetic in one function did not. Its header says `@raycache off`: `module_source` renames
  `behindBladeFull` to `behindBlade` in its module, so its pieces call the old body and the kept
  one goes uncalled (`Signature::ray_cache`, test `world_shift_moon_and_vulptex_go_without_the_ray_cache`).
  Since 1/11/2026 (update 15 of Creyon's round) the Moon and Vulptex Spiritfire do too: Creyon's
  RTX fell to 60-70 frames a second on them though on llvmpipe they cost less than Abyssal Eye in
  every scene, the pattern World Shift showed; not yet timed on his GPU.
- `bolt3` takes `px` and skips its ten segments when the whole bolt (within half its length and
  0.71 of its zigzag of its middle) is more than six units and three pixels off the ray: every
  caller turns its distance into Gaussian glows at most 1.5 wide, under a ten-thousandth there.
- Exogorth: the grit's throat flash is worked out only in the last tenth of its way in, where
  it shows, and a fang the ray passes further from than its thickness and half a pixel is
  skipped.

Timed with `scripts/blade_cost.py`, `main` and the branch in one sitting (RTX 5080, Vulkan,
1920x1080): the 48 Legendary and Mythical blades' total went from 75.1 to 58.2 ms in the
`game` scene, 40.8 to 32.0 in `close` and 32.6 to 25.6 in `along` (each blade worn by all 33
blades of the scene). Bleeding Kyber 3.57 to 1.49 ms, Abyssal Eye 3.03 to 1.37, Sith Holocron
2.52 to 1.32, Exogorth 4.06 to 2.92, Between Worlds 3.86 to 3.05; World Shift and the five
blades without a signature unchanged. Not timed on another GPU or backend.

A second round (31/10/2026) gave the blades' own loops the same kind of exact skip: a piece's
reach from a point or the axis is known, and past it plus three widths of its widest glow (and
three pixels) it adds under a ten-thousandth. Reactor's magnetic loops, Felucia Spores'
filaments and spores, Between Worlds' streams (which draw nothing past 2 anyway), Starforge's
granules and embers, Exogorth's saliva strands and Nebula Song's ribbons. Sheets again within
1/255 of the original `main`'s. Timed against `main` in one sitting (`game` / `close`): Felucia
Spores 2.06 to 1.22 / 1.17 to 0.68 ms, Reactor 2.03 to 1.26 / 1.10 to 0.63, Between Worlds 2.78
to 2.33 / 1.60 to 1.29, Starforge 1.86 to 1.65 / 1.03 to 0.92, Nebula Song 0.98 to 0.83 and
Exogorth 1.51 to 1.32 in `close`; untouched blades moved up to a tenth either way in that run,
so smaller changes are noise.

The smoke (`fxSmoke`, in 40 signatures) is not where the time goes: switched off, the 48 top
blades' `game` total fell by 2.9 of 54.6 ms, 0.29 ms at most for one blade (Starkiller Frost).
Its noise has no exact shortcut, so it is left as it is. Measuring while the client runs is
unreliable: during one run the stand-in blade went from 0.57 to 2.57 ms and the switched-off
helpers seemed to cost time; check the stand-in line of each run.

Shells and coarse outlines (1/11/2026, Creyon's Star Wars round, blades 170 to 270): Zeffo
Thunderchorus fell from 125 to 30 frames a second in his game when he zoomed in on it, its loops
of 3D bolts run for every pixel of a quad that filled the screen. Two kinds of change, timed with
`scripts/blade_cost.py` (scene `close`, llvmpipe; the Sun 20.8 to 27.2 ms in the same runs):

- **Shells** (`sh*` at the end of `lab.glsl`): `shT` finds where the view ray crosses a cylinder
  of radius R round the axis (at most twice), `shAt` weighs each crossing (none behind the camera
  or hidden by the body, fading where the ray grazes the cylinder and over soft ends, so its
  outline never shows) and gives its place on the shell, and a pattern is read there: motes,
  flakes, glints, streaks, strands, sheaths of flowing light, fire, veins, bolts drawn in the
  shell's own flat coordinates, curtains, caustics, facets, and `shSmokeA` for a solid layer. A
  quadratic and a few noise reads a pixel; particles live in cells at least eight sizes across,
  so none is cut at its cell's edge. Zeffo Thunderchorus, redrawn on them, 63.4 to 15.4 ms.
- **Coarse outlines first** in `rxRibbon`, `rxTongue` and `rxHelix`: four chords of the curve
  first; every point of it lies within their sagitta (and its sway's bend), so a ray past that
  and 3.2 widths of its widest glow (under a ten-thousandth) skips the 10 to 24 segments
  (`rxHelix` returns the chords' distance less their error, a lower bound, past three units).
  `rxMist` and `rxMistSpin` read their second wave first and skip the first where it cannot be
  dense; `rxStarCloud` skips a star before its twinkle where the ray is past its glow. The same
  pictures. Utapau Whirlwind 168.3 to 33.1 ms, Zonama Starlight 66.7 to 27.4, Nur Shadowflame 57.3
  to 26.6, Nihilus Hunger 48.6 to 25.2, Cantonica Ascension 48.5 to 25.4, Bastila Battle
  Meditation 47.2 to 25.5; Dume Wolfspirit (its tails read off a three-point outline first, six
  segments for eight) 49.1 to 40.3. Not timed on a real GPU here: run the script on main and the
  branch in one sitting before review, as AGENTS.md asks.

### Falling blossoms

Built 21/10/2026 for the Varykino Blossom (blade 166, Kushina's idea: "the afterimage like
the spectral blade has, could be these falling blossoms"). A signature may shed real petals
when it is swung, which its own quad could not hold: they are particles in the world
([saber_petals.rs](../crates/sjk-viewer/src/saber_petals.rs)).

- **Declared** by the signature's header: `// @petals {"part":3,"per_unit":0.07,"life":2.6,
  "size":1.6,"fall":38.0,"drift":14.0}`. `part` is the signature's own part (its `// @part`
  line, from 0) that switches them, so the wearer turns them off in the Collection like any
  other part and every SJK player then sees none; `per_unit` petals for each unit the tip
  travels, `life` seconds, `size` units, `fall` and `drift` units a second. A test checks
  every `@petals` line parses, is in range and names one of its parts.
- **Shed** in `saber_submission.rs`: each blade keeps its tip (`PetalEmitter` in its
  `BladeState`) in its wearer's frame, as the signatures' afterimages are, so turning the
  view or running sheds none, only the blade's own swing; the tip is the whole length's, so
  a blade growing out or going in sheds none either. At 0.07 a unit a hard swing (its tip at
  600 to 1000 units a second) sheds 40 to 70 a second, about 150 alive a blade. The owed share carries over
  frames; at most 6 a blade a frame and 256 waiting (`StateSlab::petals`); a jump in time
  or a pause of more than 200 ms forgets the tip. Each petal starts on the blade's outer two
  thirds with a fortieth of the tip's speed (the swing's breath, at most 40 units a second),
  sinking and drifting sideways.
- **Spawned** by `saber_petals::frame` (after the contacts and rain steam in
  `entity_lights.rs`) as billboard particles of the bundled petal pictures
  (`assets/petals`, drawn by `scripts/petal_assets.py`: three tints of one cherry petal,
  `shaders/sjk_petals.shader`, mounted below game data as Order 66's confetti), turning as
  they sink, stopping where they land on the floor (`apply_physics`, no bounce) and fading
  over their last third. They count against the particle pool like any effect.
  Their three shaders are in `effect_assets::CODE_SHADERS`: the frame's effect materials load
  only the shaders listed there or named by an effect file, and without them the petals drew
  the stand-in sprite (yellow dots, Sol's first test, 21/10/2026). A thrown
  saber sheds them along its spin and flight, its tip measured in the world (23/10/2026,
  hub report 127).
- **Embers** (`"kind": "ember"`, 26/10/2026, the Sun's Swing embers extra,
  [unlockables.md](unlockables.md#extras)): sparks flung off along the swing (`fling`, the
  share of the tip's speed a shed piece leaves with: 0.16 for them, a petal's 0.025 when
  absent; at most 1600 times it, units a second), rising as hot air does and flying on
  through the world rather than settling. One white spark picture (`ember`, drawn by the
  same script) drawn added to what is behind it (`GL_SRC_ALPHA GL_ONE`), the particle's
  colour cooling from white gold to deep red and its size shrinking to a third over its
  life. An extra's `sheds` is read as a `@petals` line without its part, for a skin that
  sheds nothing of its own.
- **Paper cranes** (25/10/2026, the Thousand Cranes): `"kind": "crane"` on the line (the
  default is `"blossom"`) and `"rise"`, units a second it climbs (default 0), turn the petals
  into paper cranes that climb away from the swing instead of sinking: `// @petals
  {"part":4,"kind":"crane","per_unit":0.035,"life":2.4,"size":2.6,"fall":0.0,"drift":12.0,
  "rise":42.0}`. Each is one crane picture in white paper (`crane.png`, flying right, and
  `crane_left.png`, mirrored, drawn by `scripts/petal_assets.py` from the same triangles as
  the shader's crane) tinted a colour of paper by the particle (`rgbGen vertex`, the eight
  of the shader's flock), kept nearly upright, and flying on through the world
  (`apply_physics` off) until it fades. Their two shaders are in `CODE_SHADERS` with the
  petals'.
- **Ash** (25/10/2026, the Hearth): `"kind": "ash"` sheds flakes of ash that drift down and
  settle as the petals do: a ragged grey flake (`ash_grey.png`) and, one in three, the same
  flake with its edge still glowing orange (`ash_ember.png`), both drawn by
  `scripts/petal_assets.py`, tumbling as they fall.
- **Moondust** (26/10/2026, the Moon, Creyon's): `"kind": "moon"` sheds moondust, switched with
  its "Moon glitter" part: `// @petals {"part":1,"kind":"moon",
  "per_unit":0.6,"life":3.4,"size":1.1,"fall":0.0,"drift":20.0,"rise":30.0,"moving":0.8}`.
  Specks of moondust with four short rays (`moon_dust.png`,
  white light added with `blendFunc GL_SRC_ALPHA GL_ONE` and tinted by the particle a colour
  of the blade's moonlight: silver, white, lilac or purple, turned to the wearer's saber colour
  on a chroma by the skin's turn, `PetalSpec::turn`, which only the shedding blade sets) that float up through the world as
  the cranes do, drawn by `scripts/petal_assets.py`, shrinking a little as they fade. At first
  half were ragged flakes of moon rock that dropped and settled; Creyon did not want stones
  falling out of the saber, and that picture is gone.
- **Shed while moving** (26/10/2026): a line's optional `moving` sheds that many for each unit
  the tip travels through the world beyond its swing (the wearer running, jumping or riding:
  the world travel less the swing's), as well as the swing's `per_unit`. 0 (the default)
  keeps the old rule, so a turn of the view or a run sheds nothing; the Moon blade's is 0.8.

## Saber trails

A blade grows and shrinks as it is switched on and off after
`BG_SI_SetLengthGradual` (`Extension` in [saber.rs](../crates/sjk-viewer/src/saber.rs)).
A hilt with blades of another length (single to staff, a new saber) gives a lit blade
its own length at once; until 16/10/2026 the old length stayed until the blade was
switched off and on (players' bug reports).

[saber_trail.rs](../crates/sjk-viewer/src/saber_trail.rs) follows codemp
`CG_AddSaberBlade` and `CTrail`: every frame at least 3 ms after the last, a
blade adds one slice from its remembered muzzle and tip to the current ones.
A slice lives `trailLen / 5` ms of the current `saberMove` (30–40 ms for most
moves, 40 ms when the move authors none) and fades by scrolling the clamped
blur texture, not by alpha. The short visible arc is stock behavior; frame rate
changes the slice count, not the arc's duration. Slices split along new tip to
old muzzle as `CTrail::Draw` does. A blade skin's blur trail is drawn in the skin's trail colour throughout:
stock adds the white `blurcore` untinted, which washed every skin's trail out to white
(Creyon's report, 17/10/2026), so for a skin the glow and the core both take the trail colour,
deepened (normalised to its brightest channel and squared) so a pale trail colour does
not add up to white (vertex style 3, `saber_trail::vertex_style`). Stock blades, black blades
and the sword trail are unchanged.

A skin's trail may be other than the blur (`SkinColor::trail_kind`, `blade_extras::TrailKind`):
the Clone Wars design's comic speed lines (style 4), or what its signature's `@trail` line or
one of its extras ([unlockables.md](unlockables.md#extras)) asks for. Built 26/10/2026 for
the Sun blade (Sol: a sun or fire trail; 1 the default, the Solar card unlocking the others):

- **Plasma** (style 5, every Sun's, `// @trail plasma`): white gold where the blade just
  passed, cooling through the trail colour to deep red as the slice ages, brightest toward
  the tip with a white-hot seam where the tip went, its trailing edge torn into flames by
  two octaves of value noise laid in the world (so they run on unbroken from one slice to
  the next) and rising with the slice's age. The colours come from the trail colour
  normalised to its brightest channel, so another blade's plasma cools from its own.
- **Flare** (style 6, the Sun's Flare trail extra): the plasma, and past the tip flames and
  thin magnetic loops licking off it, as prominences off the Sun's limb: its slice reaches
  `saber_trail::FLARE_REACH` (10 units) past the tip, the texture's V going below 0 there,
  and each vertex carries that V at the far edge (`Vertex::past`, attribute 4).
- **Heat wake** (`saber_trail::WAKE`, a bit beside the style, the Sun's Heat wake extra):
  what is behind the trail shimmers, drawn by `fragment_wake` in the blades' heat-haze run
  (`SaberGpu::draw_haze`, through the haze's bind group of the effect layer's scene image,
  before the black holes' lenses): no blending, the scene read at an offset of up to 2.5
  units (never more than 2 % of the screen, `HAZE_REACH`, the room the effect rectangle
  keeps), gone at the slice's edges and as it ages, so the merge sees no seam.

Built 27/10/2026 for the Moon blade (Creyon, after Sol's Sun trails: the Sun's night sister):

- **Moonlight** (style 7, every Moon's, `// @trail moonlight`): a soft silver ribbon where the
  blade just passed, light rippling across it as moonlight on water (the plasma's world noise,
  slower), cooling through the trail colour (lilac, the wearer's on a chroma) to deep violet,
  its trailing edge dissolving into stardust: specks in world cells (`moon_glitter`), drifting
  down as they age where the plasma's flames rise, twinkling out; a fine silver seam where the
  tip went.
- **Eclipse** (style 8, the Moon's Eclipse trail extra): the ribbon hollowed into a ring of
  light along its edges, as the corona round an eclipsed moon, white within and the trail
  colour without, a diamond-ring flash where the tip just passed, stardust still drifting.
- **Starfall** (style 9, the Moon's Starfall trail extra): a faint moonlight ribbon with
  shooting stars in nine lanes across the band, each a bright head and a fading silver tail.

The plasma, flare, moonlight, eclipse, starfall and a wake live three times as long as the blur (`LONG_LIFE`: 90 to 120 ms
for most moves): the cooling and the shimmer need that time to be seen. Each slice's age
reaches the fragment shader in the colour's alpha, which the additive blend ignores. Verified
by unit tests (the styles and the wake bit from the Sun's choices, the shader validated by
naga) and the world shot `duel6_sun_trails` (a swing over the head as the stock blue, the
Sun's plasma, the Flare trail, the Heat wake and all extras, `SaberSkins::shot_swings`;
with `SJK_TEST_PACKS` naming the hub's packs): the plasma reads gold and see-through with
a flame-torn edge, the flare's red loops lick past the tip; the wake's bend is faint in a
still. Not yet looked at in game.

With `cg_saberContact` on, the tip stops at
the first world surface the blade enters
([saber_trail_edge.rs](../crates/sjk-viewer/src/saber_trail_edge.rs)); stock
also stops it at solid brush entities, which the client does not trace yet. A flying
primary saber trails and shares the owner's blade state, as in stock. A blade that is
out keeps its remembered edge at the hilt without drawing (`Trail::hold`), as stock's
`dontDraw` call does for a staff's extra blade or a second saber switched off, and
forgets a skin's afterimages; SJK does it for the primary blade too, which stock leaves
where it went out. Otherwise the first swing after a blade lit again within two seconds
drew one slice, and the afterimages, from where it went out to where it was (Foshin's
hub report 116, 21/10/2026: switching sabers with a signature shader). Not yet
drawn: the extra trails stock adds while `PW_SPEED` is set with `cg_speedTrail`
and during super-break win animations.

Blade/wall contact ([saber_contacts.rs](../crates/sjk-viewer/src/saber_contacts.rs))
shortens both the visible glow and core to the world trace's hit point, as
OpenJK `codemp`'s `CG_AddSaberBlade` does. This works with trails disabled and
with `noWallMarks`; `cg_saberContact 0` disables the cutoff. The cutoff only
changes the frame's render instances, so leaving the wall restores the blade's
extension length. Solid brush entities such as movers are not traced yet.
Wall contact also
plays a wall-hit sound once a blade has stayed in the wall since the previous
frame, at most every 100 ms per blade. Like stock's `S_StartSound(..., -1,
CHAN_WEAPON, ...)`, all wall hits share one source and channel, so each new hit
replaces the previous one instead of overlapping it.

Static sun cascades carry a small min/max depth atlas: one pair of full-precision
bounds per 64×64 shadow texels, with separate view, close and far layers. A layer
is rebuilt immediately after its source map changes. Receivers bound the entire
possible filter footprint, including bilinear neighbours and receiver-plane
variation. Only footprints proved fully lit or fully blocked bypass the blocker
search and Gaussian reconstruction; uncertain footprints run the existing filter.
Moving-caster filtering is unchanged. At 2048 resolution the atlas occupies
24 KiB. Coalesced tile reads keep rebuilding practical in tier 2 as well as the
held-cascade modes. Golden-angle sample directions are constant shader data;
sample counts, radii, weights and cascade transitions are unchanged.

Real-time world shading skips baked-lightmap texture reads when the composition
hook replaces their RGB. Uploaded BSP lightmaps and their fallback have alpha
one; authored image alpha and animation still follow their normal paths.
The expensive main-view and floor-reflection material passes also prime depth for static surfaces
whose first stage guarantees opaque coverage. Deforms, sprites, alpha tests,
polygon offsets and special depth/blend modes keep their ordinary path. Depth
priming reuses the existing visibility ranges and indirect argument storage,
with direct draws as a fallback. It is limited to active real-time lighting;
reflections use their own camera, depth target, receiver frustum and scissor.

The main view's PVS source cluster is the leaf of the camera actually used for the picture
(`view_position`, as `refdef.vieworg` in stock), not of the player's eye. In third person
the camera sits behind and above the eye, often in another cluster; taking the eye's cluster
culled walls the camera could see. Reflection, portal and scene views already used their own
eye.

A `portal` face (sort `portal`) shows a remote camera or a mirror when a `misc_portal_surface`
entity (`ET_PORTAL`, 8) lies within 64 units of its plane, as stock's
`R_GetPortalOrientations` requires; the entity's `origin2` is the `misc_portal_camera`
([scene_views.rs](../crates/sjk-viewer/src/scene_views.rs),
[scene_portal_math.rs](../crates/sjk-viewer/src/scene_portal_math.rs)). Until 19/10/2026
the match looked for entity type 7 (`ET_BEAM`), so no network portal was ever found and
such faces showed only their own texture: the grey `textures/lgnd_misc/camera` screen of
JoF's `mp/ffa3` security camera, for example. The portal view's target is made the first
frame a face selects a view ([GPU memory](#gpu-memory)). The ignored world shot
`scene_view_targets` ([world_shot_scene_views.rs](../crates/sjk-viewer/src/world_shot_scene_views.rs))
renders through each secondary view (floor mirrors on ffa3, duel6's sky portal, the mirrors
of `sjk_shaders_e`) and prints which targets a world holds; `scene_view_targets_resize`
checks the mirror target follows `r_superSample` and a resize (with
`SJK_SHOT_CVARS=r_superSample=1,r_renderScale=50`, the render scale).

Camera-range caches also retain their PVS/area selection independently of the
camera frustum. Turning or moving within a cluster rechecks bounds but reuses
the same candidate indices; a source-cluster, area-mask or PVS-mode change
invalidates that selection. Storage is reserved at map load, at one index per
static draw. Unbounded and blended materials keep direct traversal.

External release replay checks compared these changes with `b4debe4` on
Linux/RADV, Ryzen 5 5500 and Radeon RX 9060 XT. Each recording contains 31
player profiles on `mp/ffa3` or `mp/ffa1`; 19–23 and 16–27 actor groups,
respectively, were evaluated during the measured routes. Settings were render
scale 1, HDR, day/night held at 11:00, lighting tier 0, 2048 shadow maps, 16 base
taps, volumetrics 1 and anisotropy 16. After five seconds of replay warmup,
6,660 frames sampled a 20-second route at 333 simulated frames per second.

| Replay / resolution | GPU mean before → after | Total frame mean before → after |
| --- | --- | --- |
| `mp/ffa3`, 3840×2160 | 5.302 → 4.575 ms | 5.473 → 4.746 ms |
| `mp/ffa1`, 3840×2160 | 6.714 → 5.808 ms | 6.894 → 5.983 ms |
| `mp/ffa3`, 2560×1080 | 2.026 → 1.845 ms | 2.499 → 2.474 ms |

These are offscreen full-frame replays, with no per-frame readback or capture
copy and no live network/audio output. They show a CPU limit at the smaller
resolution, not achievement of the 2 ms total-frame goal. Native presentation,
live matches, other GPUs and exhaustive community-content coverage remain open.

External GPU instrumentation compared the conservative shadow result with the
full filter at 656,749,476 accepted samples across both maps, moving sunlight,
tier 2, 1025-pixel shadow maps with 32 base taps, and 4K output. Maximum
visibility error was 0.00000012; none exceeded the 0.000002 verification threshold.
Twenty 4K scene snapshots across both routes retained the reference appearance;
one snapshot's kill-feed expiry differed with wall-clock timing. Baked lighting
and actor-only captures matched byte for byte. Fullbright, lightmap debug and
the non-table/direct-draw fallback also passed GPU and image checks. These
finite samples are not exhaustive image equivalence for every view or material.

Formatting, locked workspace build/tests and the release client build passed.
Cargo still runs no bundled regression suite. A native 1280×720 `mp/ffa3` static
map run rendered for a 45-second process lifetime without panic or GPU validation
error, using an isolated config. This is an integration smoke check, not a
populated-match performance result. The combined retained changes also completed a
45-second native `ffa1` crowd-replay process run with an isolated configuration
and no panic or GPU validation error. Concurrent compilation makes that latter
run an integration check only.

A follow-up CPU cache comparison used the same routes/settings at 2560×1080:
`ffa3` world-pass encoding fell from 0.565 to 0.513 ms and total frame mean from
2.462 to 2.420 ms. `ffa1` encoding fell from 0.884 to 0.866 ms; its total mean
was essentially unchanged (3.119 to 3.131 ms). An external verifier matched
3,178,666 cached/direct range results across the replays and forced area-mask,
source-cluster and missing-PVS transitions. Twenty further 4K captures retained
the scene appearance, with small pixel differences and the known wall-clock
kill-feed difference. This is finite coverage, not a universal cache proof.

Floor-reflection depth priming was compared in alternating order on the first
10 seconds of the `ffa1` route at 4K (3,330 frames per run, two runs per variant).
Mean GPU time was 5.453 → 5.417 ms; the reflected-plane phase was
1.116 → 1.081 ms. On the full 20-second route at 2560×1080, total frame means
were 3.135 → 3.094 ms. Twenty 4K captures across both maps showed only small
pixel differences (maximum 8/255), with the shadow filter and reflection
resolution unchanged. Paired 4K `ffa3` runs measured essentially unchanged
GPU time (4.547 versus 4.550 ms), with almost no floor-reflection work on that
route. Other mirror/portal types retain their previous path.

Particle stage selection borrows the atlas and evaluates stages as they are
consumed, retaining the existing eight-stage cap, order, animation, waveform,
texture transforms and missing-shader fallback. This avoids filling and copying
eight samples for every effect, including callers that only need the first.
The sampling implementation lives outside the frame-orchestration module.
Paired 3,330-frame `ffa3` runs at 2560×1080 reduced billboard preparation from
about 0.067 to 0.047 ms and effect geometry preparation from 0.034 to 0.029 ms.
Total-frame differences were within run variation; this is a CPU phase result.
An external comparison matched 3,360 stage samples bit for bit across both
loaded atlases and missing, empty and over-capacity shader cases.


Entity sorting caches each source material's immutable shader sort and first-stage
pipeline keys at material creation, including late custom materials. It preserves
missing-material defaults, opaque tie-breaking and back-to-front blend order;
per-frame draw records and allocation behavior are unchanged. Alternating
6,660-frame runs at 2560×1080 reduced instance preparation from about 0.092 to
0.078 ms on `ffa3` and 0.085 to 0.075 ms on `ffa1`. Total-frame differences were
within run variation. An external comparison matched 1,362,200 ordered draw
entries against the original comparator across both routes. Twenty further 4K
captures retained the scene appearance (one pixel differed by 12/255; all others
by at most 8/255).
The draw queue also classifies stage-major eligibility once when a draw is added,
rather than repeating material lookups in every colour pass. Missing materials
and depth-less draws retain their previous handling, and the 32-byte draw size is
unchanged on the measured 64-bit build. Two alternating 6,660-frame runs per
variant reduced CPU world-pass encoding by roughly 0.011 ms on `ffa3` and
0.016 ms on `ffa1`; total means fell by 0.034 and 0.026 ms respectively. The
same external verifier checked every cached classification and all material keys,
including missing-key defaults, across both replays; 20 captures showed only
small pixel differences (maximum 13/255 at one pixel).

Volumetric integration carries each slice's end depth into the following slice
instead of recomputing the same boundary. Sample locations and accumulated
scattering retain their existing arithmetic. Paired 3,330-frame 4K runs on both
routes saved about 0.0033 ms in the volumetric phase; 20 captures retained the
scene appearance, with small pixel differences (maximum 12/255 at one pixel).

Ambient occlusion reuses the fixed 16 sample directions and radii, preserving their
f32 expressions, sample order, reach and strength. Depth reconstruction omits the
ray normalization that cancels in its intersection ratio and scales the degeneracy
guard accordingly. Paired 3,330-frame 4K runs reduced the AO pass from 0.369 to
0.344 ms on `ffa3` and 0.354 to 0.330 ms on `ffa1`; total GPU means fell by
0.023 and 0.026 ms. Twenty captures retained the scene appearance, with one pixel
differing by 12/255 and all others by at most 8/255. These are measurements on the
same Vulkan setup, not exhaustive equivalence across all maps and backends.

## Dynamic glow

Shader stages marked `glow` get a blurred halo, as in stock Jedi Academy and
EternalJK (`r_DynamicGlow`). Stock draws the frame's surfaces a second time with
only their glowing stages into a black image that shares the scene's depth, blurs
it and adds it to the frame (rd-vanilla `tr_backend.cpp:1736-1800`). SJK does the
same for the main view:

1. **Glow flags.** The parser keeps each stage's `glow`. A hardware pass carries the
   flag of its first source stage: `CollapseMultitexture` moves only texture
   bundles, so a glowing stage merged under a non-glowing lightmap stage does not
   glow, as in stock ([world_stage_collapse.rs](../crates/sjk-viewer/src/world_stage_collapse.rs)).
   A material glows when any pass does (`hasGlow`). Effect shaders keep the flag per
   stage; saber blades glow and their cores do not (`sabers.shader`: the `*_glow`
   sprites, `saberBlur` and `swordTrail` carry `glow`, the `*_line` cores do not).
2. **Glow pass** ([glow_pass.rs](../crates/sjk-viewer/src/glow_pass.rs),
   [world_glow.rs](../crates/sjk-viewer/src/world_glow.rs)). After the main view's
   effects, the glowing passes of visible world surfaces (opaque, then blended) and
   the glowing stages of the frame's entities draw into a scene-sized 8-bit image,
   cleared to black, with the scene's depth attached read-only. They use the scene
   pass's vertex paths, bind groups and blend with glow variants of the stage
   pipelines (the image's sRGB view, no depth write), compiled on first use like
   the scene's entity pipelines. Glowing billboards, cylinders, lines and
   electricity, saber blades and trails then draw through the image's plain view
   with their ordinary effect-layer pipelines. Within each effect blend slot the
   glowing layers are emitted after the others, so the glow pass draws one tail
   range per slot; this only reorders layers within one slot, which leaves additive
   and modulating blends unchanged (alpha-blended layers stay back to front within
   each part). `r_dynamicGlowNoSabers 1` leaves the saber part out (the blades' glow
   capsules with their skins' glints and black-hole horizons, and the trails), so only
   the rest of the scene glows; the sabers' own drawing in the scene and the effect
   layer is unchanged ([`Live::sources`](../crates/sjk-viewer/src/post_glow.rs)).
   Nothing glowing on screen means no pass, no blur and no composite
   work; a frame only walks the glowing world passes and the entity queue to decide.
3. **Blur** ([post_glow.rs](../crates/sjk-viewer/src/post_glow.rs)), in 8-bit images
   that clamp after every pass as stock's framebuffer copies did:
   - `r_dynamicGlowStyle 1` (default), rd-vulkan's: a four-level pyramid at 1/2,
     1/4, 1/8 and 1/16 of the window, each level a three-tap horizontal then
     vertical blur of the previous level, taps 1.2 texels apart, weights 6/16 and
     5/16 each raised by 0.15 (`blur.frag`, `vk_pipelines.cpp:1713`). The four
     levels are summed and scaled by `r_DynamicGlowIntensity - 1` (within 0.01-4,
     `vk_pipelines.cpp:1519-1523`) at half size, then added. This is what EternalJK
     players with `cl_renderer rd-vulkan` see; rd-vulkan sums the levels at full
     size, so the three smaller levels are slightly softer here.
   - `r_dynamicGlowStyle 0`, rd-vanilla's `RB_BlurGlowTexture`: `r_DynamicGlowPasses`
     passes at `r_DynamicGlowScale` of the window (or `r_DynamicGlowWidth` x
     `r_DynamicGlowHeight` when both are positive), each summing four diagonal taps
     (0.1 + pass x `r_DynamicGlowDelta`) texels away, weighted
     `r_DynamicGlowIntensity` / 4; the first pass reads the full-size image.
4. **Composite** in the final resolve ([post_aa.wgsl](../crates/sjk-viewer/src/post_aa.wgsl),
   `with_glow`), after the effect layer and before the `r_gamma` ramp, on display
   values with `r_sceneHdr` 0 or 1: retail with `r_DynamicGlowSoft 1` screens
   (`e + g - e*g`, `GL_ONE, GL_ONE_MINUS_SRC_COLOR`); otherwise the glow is added and
   clamped (rd-vulkan always adds). `r_DynamicGlow 0` builds none of this.

| Cvar | Default | Behavior |
| --- | --- | --- |
| `r_DynamicGlow` | 1 | 0 off, 1 on, 2 saber blades only (JoF EternalJK), 3 the blurred glow without the scene (debug); live, 0 frees the images |
| `r_dynamicGlowStyle` | 1 | 1 rd-vulkan pyramid, 0 retail kernel; rebuilds the blur |
| `r_dynamicGlowNoSabers` | 0 | 1 keeps saber blades and trails out of the glow image (g0dchris' idea), with any style; with `r_DynamicGlow 2` nothing glows; live |
| `r_DynamicGlowIntensity` | 1.13 | Retail per-pass gain; rd-vulkan scales its level sum by Intensity - 1; live |
| `r_DynamicGlowPasses` | 5 | Retail passes, 1-32; live |
| `r_DynamicGlowDelta` | 0.8 | Retail tap spread added per pass; live |
| `r_DynamicGlowSoft` | 1 | Retail screen composite (1) or additive (0); live |
| `r_DynamicGlowScale` | 0.25 | Retail blur size relative to the window; rebuilds the blur |
| `r_DynamicGlowWidth`, `r_DynamicGlowHeight` | 0 | Retail blur size in pixels when both are positive; rebuild |

All are archived and keep stock's names (lookups ignore case, so the retail menu's
`r_dynamicglow` is the same cvar); `r_dynamicGlowStyle` and `r_dynamicGlowNoSabers`
are SJK's. Stock defaults `r_DynamicGlow` to 0; SJK turns it on. The renderer
settings' IMAGE tab has rows for `r_DynamicGlow`, `r_dynamicGlowNoSabers` (No glow on
sabers) and `r_dynamicGlowStyle`. SJK's classic menus have no
counterpart of the retail Setup page's glow toggle.

Emission-mapped world stages are drawn too, writing only their emission (see
[Emission maps](#emission-maps)). Not drawn into the glow image: the sky, flares, the
menu stage, and secondary views (portals, sky portals, floor reflections), which show
no glow yet. Stock's
glow pass fogs towards black, SJK's does not, so glow inside fog is brighter than
stock's. Decals never glow. The image holds unexposed display values, equal to the
scene's at `r_hdrExposure 1`: an exposure belongs on the world's glowing stages
before they are encoded (see `world_format` in post_glow.rs), not on effects, which
are display values already.

Cost: a frame that glows clears and fills a scene-sized RGBA8 image (33 MB at
3840x2160) and redraws only the glowing passes; rd-vulkan's blur then runs eight
small passes and the level sum (about 2.8 million pixels at 4K, three or four
texture reads each), retail's five passes at a quarter size. The images take about
55 MB at 4K with rd-vulkan's style and 39 MB with retail's. These are estimates, not
measurements: no GPU timing has been recorded. Unit tests cover the glow flags
(Tavion's possessed skin, olol, the collapse quirk), the saber blade/core split,
cvar parsing and the kernels; the programs are validated with naga. On-screen
appearance is unverified.

## Material maps

The optional material maps follow OpenJK rend2 (`codemp/rd-rend2`), so rend2
texture packs apply without conversion. Stage keywords (`ParseStage` in
`tr_shader.cpp`) are parsed by
[sjk-shader](../crates/sjk-shader/src/material.rs): `normalMap`,
`normalHeightMap`, `specMap`/`specularMap`, the packed `rmoMap`, `moxrMap` and
`ormMap` families, and `specularReflectance`, `specularExponent`, `gloss`,
`roughness`, `normalScale`, `specularScale`, `parallaxDepth` and `parallaxBias`.
SJK adds emission maps (`<diffuse>_e`, below), which rend2 does not have.
Their order-dependent overrides are kept. rend2 selects a packed layout by
comparing the image name with the keyword, so `rmosMap`, `mosrMap` and `ormsMap`
load the three-channel layouts; SJK does the same. Without keywords,
[the lookup](../crates/sjk-viewer/src/material_map_images.rs) tries `<diffuse>_nh`
then `_n` for normals and `_specGloss`, ioquake3's `_s`, `_rmo` then `_orm` for
specular, as in rend2's `CollapseStagesToGLSL`. ioquake3's typed
`stage normalMap` stages are not supported. With the cvars off, the parser
records the keywords and nothing else changes: no image lookup, layout,
buffer or pipeline is created. SJK turns the cvars on by default (parallax at a tenth of its depth); without a
pack (or keywords) a map load only checks the candidate names in the file
index, and no layout, buffer, pipeline or reflection probe is created.

Maps apply to lightmapped world surfaces (static and inline movers) whose
lightmap and diffuse stages collapse into one opaque pass, and to the paint of a
glossy base (below). On the retail
`mp/ffa1`, `mp/ffa3` and `mp/duel1` this covers 84–87% of world triangles;
`mp/siege_hoth` covers 52%. Since 07/10/2026 they also apply to vertex-lit world
surfaces (terrain, `_phong` sand and rock, `q3map_onlyvertexlighting` shaders) whose
first stage is opaque `rgbGen vertex`/`exactVertex` paint: in baked lighting the
vertex colour stands in for the lightmap texel (`material_map_vertex_light`, the same
response), in real-time lighting the light buffer is read through
`material_map_lightmap` as for lightmapped paint. Detail stages and blended terrain
layers over such paint, stacks that do not collapse, effect stages, deforms, sprites
and models (MD3, Ghoul2) keep their authored shading, so a mapped terrain base can
differ from an unmapped layer blended over it.

**Model normal maps** (SJK, 05/11/2026,
[material_map_models.rs](../crates/sjk-viewer/src/material_map_models.rs)): a model stage
lit per entity (`rgbGen lightingDiffuse` or `lightingDiffuseEntity`, not collapsed) that
names its normal map with `normalMap` is lit per pixel through it. Only the keyword
counts: a `_n` image found next to a model's texture is never applied, so stock and
community models are unchanged (the first user is the Legendary holocron). The map rides
in the stage's idle second slot, uploaded linear with a box-filtered mip chain (never
block-compressed: the `$normal:` cache key keeps it out of the texture workers), and the
stage program (`model_normal_prepare` in `stage_runtime.wgsl`) builds the tangent frame
from the screen-space derivatives of the position and the picture coordinates, since MD3
and Ghoul2 vertices have no tangents; green runs along t, downward, as the world's maps
read it. Stage flag 16 in `wave_functions.w` turns it on and `secondary_animation.xy` holds
the `normalScale`. The bent normal feeds the per-pixel model diffuse (on with the map even
without `r_modelPixelLight`), toon bands and, in real-time lighting, the sun highlight,
the sky rim and the local lamps; the light buffer is still matched by the face's own
normal. `r_normalMapping` and `r_normalMapStrength` govern it as they do the world's.
A glossy base is Korriban's wall stack (`korriban/door`, `entrance_top`,
`os_outsidebased`, `os_basic_pillarb`): an environment stage, the paint alpha-blended
over it (its alpha, about 0.84 on the gold inlays, lets the shine through) and the
lightmap filtered over both, `lightmap × (paint × a + shine × (1 − a))` in three passes
that rd-vanilla and rend2 do not collapse. SJK draws the same colour in two lightmap
collapses ([world_stage_collapse.rs](../crates/sjk-viewer/src/world_stage_collapse.rs),
`glossy_base`): the shine times the lightmap, opaque, then the paint times the lightmap
blended over it with the paint's alpha. The paint pass is an ordinary lightmapped
diffuse pair, so it takes maps and the generator writes them; the alpha stays a mask
over the shine, never transparency. Any other colour, a wave, an alpha test or a
different depth function keeps the three passes. Since 13/10/2026; before, these walls
took no maps and overrides could not reach them (Sol's world notes, 07/10/2026).
Each material-mapped stage compiles to its own pipeline key and a second bind
group; ordinary stages keep their pipelines, groups and stage-table records.

Map load computes [vertex frames](../crates/sjk-viewer/src/material_map_frames.rs)
for the flattened world only when a stage has maps: a tangent with handedness,
averaged over the triangles of patch and triangle-soup vertices, and the
light-grid direction (`R_CalcVertexLightDirs`/`R_LightDirForPoint`). They use 8
bytes per vertex. Building them for `mp/ffa3` (114,088 vertices) took about 24 ms
in a release test build. Maps are uploaded as linear RGBA8 with box-filtered mips,
less the levels texture detail (`r_picmip`, [GPU memory](#gpu-memory)) leaves off the
diffuse texture: since 25/10/2026 a 1024-pixel normal map at r_picmip 2 is its 256-pixel
level, as its diffuse texture is. As in rend2 (`ParseStage`, `CollapseStagesToGLSL`),
the maps of a `nopicmip` or `nomipmaps` shader's stage keep their full size.

Shading lives in [material_maps.wgsl](../crates/sjk-viewer/src/material_maps.wgsl):

- Baked lighting: rend2's lightmap response, with the light-grid direction in
  place of a deluxemap. The texel is taken as arriving along that direction,
  divided by the face's own cosine (at most 4x) and received by the mapped normal;
  the remainder stays ambient. A flat normal map reproduces the texel. Retail BSPs
  have no deluxemaps, so this is an approximation.
- Real-time lighting (`r_dayNight 1`): the sun share of the half-resolution
  light buffer is moved to the mapped normal per pixel, using the visibility the
  buffer keeps. It fades out toward the terminator, so mapped bumps never light a
  face turned from the sun or a shadowed texel. The rest (lamps, probe bounce and
  sky) is redistributed the same way through a direction target (below). Without a
  specular map, the existing sun highlight and sky rim use the mapped normal.
- Specular maps use rend2's two paths: spec/gloss, and occlusion, roughness,
  metalness and specular with the albedo as metal colour. Highlights use rend2's GGX
  `CalcSpecular` for the sun or grid direction and for dynamic lights. They are
  added after the albedo and dynamic-light modulation. Occlusion darkens only the
  ambient share and the probe reflection.
- With specular maps, surfaces reflect the nearest reflection probe (below), with
  rend2's split-sum `CalcIBLContribution`; without a captured probe, real-time
  lighting keeps its sky rim.
- Parallax marches rend2's view ray through the height with limits rend2 lacks
  ([Parallax](#parallax)): the offset is at most depth / 0.35, the depth fades out
  where it would no longer show, and near the camera it stops growing on screen.
  `r_parallaxStrength` scales the depth live (lighting-mode bits 2–7, which decode
  to the default 0.1 when empty); SJK draws a tenth of it by default, 0 flattens it.
- Specular anti-aliasing (Kaplanyan and Hoffman; Tokuyoshi and Kaplanyan's bound):
  the mapped normal's screen-space variation is added to the squared roughness
  (at most 0.18), so bumps finer than a pixel widen highlights and blur the
  reflection instead of sparkling.
- The probe reflection follows half the mapped tilt: generated normal maps guess
  relief from paint, and at full tilt every guessed bump warped the reflected
  room as the view moved. Highlights keep the full mapped normal.

### Parallax

rend2's `RayIntersectDisplaceMap` marches the view ray through the depth in the normal
map's alpha (16 linear and 8 binary steps) and offsets the texture by depth / cos, which
grows without bound toward grazing views; Sol saw generated relief swim.
`material_map_parallax` in [material_maps.wgsl](../crates/sjk-viewer/src/material_maps.wgsl)
keeps the march and limits it:

- **Offset.** At most depth / 0.35 (Welsh's offset limiting).
- **Far.** The depth fades out where all of it would move the texture by less than half
  a pixel on screen (gone at an eighth), below 8.6° above the surface (gone at 2.9°) and
  where the sampler reads mip levels 2 to 4 (4 to 16 texels a pixel, at the level a 16×
  anisotropic sampler picks), which no longer hold the relief. Until 08/10/2026 it faded
  below 20° (gone at 8.6°) and from 1.5 to 4 texels a pixel along the longer side of the
  pixel's footprint, which a grazing view stretches: floors lost their parallax a few
  metres ahead. The first limit grows with the depth and the resolution, so deeper
  relief and 4K keep parallax farther. Where the new limits keep less
  than the old ones did (views close to head-on, far away), the old ones moved the
  texture by less than half a pixel, in every view the tests sweep.
- **Near.** Closer to the surface's plane than `r_parallaxNearDistance` (default 24
  units), the depth shrinks in proportion to the camera's distance from that plane, so
  the parallax keeps the size on screen it had at that distance. Without the limit it
  grows as 1/distance: the third-person camera pressed 4 units from a wall saw six times
  the parallax it had at 24 units, swimming as the camera moved and stretching the
  texture over every relief edge. A first-person eye stays 15 units from a wall and keeps
  15/24 of the depth; floors (60 units below a standing eye, 36 crouched) keep all of it.
  `r_parallaxNearDistance 0` turns the limit off, for comparison.
- **Steps.** Two linear steps per texel of the mip level the ray crosses, 4 to 24, then 6
  binary steps and rend2's final interpolation: a pixel reads the height at most 11 times
  (4 steps) to 31 (24 steps), against rend2's 25. A shallow or distant relief takes the
  fewest; a deep one seen up close the most, so its layers no longer show.
- **Mip seams.** The maps and the diffuse image are read at the offset coordinates with
  the coordinates' own screen derivatives. Implicit derivatives took in the offset's,
  whose jump at a relief edge picked a blurred mip level along the edge: a seam that grew
  as the camera came closer and the texture was magnified.

How far the whole depth reaches, before and after, in units from the camera, at the
default depth for a texture repeating every 128 units (retail's scale 0.5) with a
1024-texel map (an HD pack's; a 256-texel map in brackets), 1920 pixels across a 90°
view:

| Surface | Whole depth to | Gone by |
| --- | --- | --- |
| Floor, standing eye (60 units up) | 90 (170) → 410 | 160 (340) → 920 |
| Wall seen at 45° | 140 (560) → 570 (910) | 340 (1,360) → 1,920 (3,480) |

At 3840 pixels across, walls keep it about twice as far and floors to the 2.9° limit
(about 1,200 units). The numbers come from
[a model of the shader's limits](../crates/sjk-viewer/src/material_map_parallax_tests.rs),
whose tests also check that the limits fade without a ring, that the near limit holds the
parallax on screen, that the steps stay within 4 to 24, and that the shader keeps the
modelled arithmetic. Cost: surfaces past the old limits now march too, mostly with 4
linear steps (at most 11 reads); estimated, not measured.

`r_materialMapsDebug 7` shows the reach on parallax stages: red is the share of the
depth the far limits keep, green the share the near limit keeps, blue the linear steps
out of 24. Yellow is the whole depth; it turns green where distance or a grazing view
fades it out and red where the camera's closeness holds it back.

### Lamp and bounce direction in real-time lighting

On a map with material maps the light pass also writes a half-resolution RGBA8
direction target beside the light buffer
([sun_realtime.wgsl](../crates/sjk-viewer/src/sun_realtime.wgsl), `DirectedLight`):
the dominant direction of the non-sun light (octahedral), the share of it that
arrives from that direction and the lamps' part of that share. Lamps contribute
the luminance-weighted sum of their directions (each lamp's shadowed irradiance
times the unit vector to its centre); the probes contribute their L1 irradiance,
whose `a + b cos` form gives `2b cos` (at most the whole irradiance) as the
directional part. Both shares are applied to the light as finally shown (after the
lamp response, gain, fill and occlusion). The lamp cache bakes the same lamp vector
per texel into an RGB10A2 layer beside its light (`bake_light_directed`), so cached
receivers read one more bilinear sample instead of walking their lamp lists.

The material program reads the direction through the same depth- and normal-aware
weights as the light (`material_map_buffered` in
[material_maps_realtime.wgsl](../crates/sjk-viewer/src/material_maps_realtime.wgsl))
and treats the directional share as the baked path treats a lightmap texel: divided
by the face's own cosine (at most 4x), received by the mapped normal, fading toward
the face's terminator; the remainder stays ambient, so a flat map reproduces the
buffered light. The lamps' part casts GGX highlights along that direction with the
stage's roughness and metalness (`material_map_shade_light`); bounce and sky cast
none (they belong to the reflection probes). Baked lighting is unchanged.
`r_dayDebug 1024` (live) leaves the non-sun light as the light pass evaluated it, for
side-by-side comparison. Normal maps
therefore respond under lamps and in bounce-lit interiors, not only in sunlight.

Cost: only maps with a material-mapped stage get the target and the cache layer;
others compile and run exactly as before. The target costs 4 bytes per light-buffer
texel (8 MiB at a 3840×2160 window, twice that with floor-mirror images), the cache
layer half the lamp cache's size again (the load log prints the total, `Lamp light
cache: ... with directions`). Per frame the light pass writes the extra target and
does a little more arithmetic per lamp and probe; material-mapped pixels read four
more texels. Estimated, not measured: a few hundredths of a millisecond at 4K on a
current GPU. A single dominant direction cannot represent two lamps on opposite
sides; their vectors cancel and the light stays ambient, which is the safe failure.

### Reflection probes

Specular-mapped surfaces reflect prefiltered cube maps captured in the map, after
rend2's cubemaps ([reflection_probes.rs](../crates/sjk-viewer/src/reflection_probes.rs),
[reflection_capture.rs](../crates/sjk-viewer/src/reflection_capture.rs)). They exist
only with `r_specularMapping` and `r_cubeMapping` on, on a map with specular-mapped
stages; otherwise nothing is placed, allocated or captured.

- **Placement.** Probes stand at the map's `misc_cubemap` entities, else at every
  player spawn (deathmatch, start, duel and CTF spawns) lifted to eye height, else at
  the intermission spot, as rend2 picks the first class that has any. Points in solid
  space are dropped, points within 256 units merge into their mean, and at most 64
  are kept by farthest-point selection. Each probe measures its room with six axis
  traces against the world brushes (32–2048 units per side).
- **Assignment.** Every specular-mapped surface takes, among its four nearest probes,
  the nearest one a trace from the surface reaches unobstructed (else the nearest),
  as rend2 assigns cubemaps per surface. The index rides in the spare byte of the
  vertex frames, so the shader needs no search.
- **Parallax.** The reflected ray is intersected with the probe's room box and the
  cube is read toward that point from the probe (Lagarde's box-projected cubemap).
  rend2 approximates the room by a sphere of one radius; a box fits Quake's
  axis-aligned rooms and keeps floors and walls aligned with their reflections. A
  surface outside its probe's box reads the plain reflected direction.
- **Capture.** Faces are rendered through the ordinary scene path before the main
  view's light pass, with this frame's sun cascades: in real-time lighting the light
  pass lights the face in the light buffer's corner, as floor mirrors do; in baked
  lighting the faces show the lightmaps. Sky, opaque and blended world surfaces and
  movers are drawn; players, items, effects and fog are not, and captured surfaces do
  not reflect probes themselves (no reflections of reflections). After map load one
  whole probe is captured per frame until all are done; surfaces use the sky rim until
  their probe is ready.
- **Filtering.** Each captured cube is box-downsampled, then every level of its slot
  in an RGBA16F cube array is GGX-prefiltered with filtered importance sampling (64
  samples, rend2's `prefilterEnvMap.glsl`), down to 4×4 texels for roughness 1. The
  split-sum BRDF table (64², rend2's `R_CreateEnvBrdfLUT`) is computed once on the CPU.
- **Shading.** The cube's level `roughness × last level`, times `F0 × scale + bias`
  from the BRDF table, times occlusion, added after the albedo like the other
  highlights; F0 is the specular colour (packed maps: the dielectric value mixed toward
  the albedo by metalness).
- **Time of day.** When the sun turns by half a degree, or the sun, sky colour, light
  scale or indirect gain change by 2%, all probes are refreshed one face per frame
  (384 frames for 64 probes); each keeps its old content until its new faces are
  filtered. With a running day clock (`r_dayMinutes`) reflections therefore trail the
  sun by a few seconds.

Cost: the cube array takes about 1.05 MiB per probe at 128² (6 faces, 6 levels of
RGBA16F), plus a scratch cube and a 128² capture target (about 1.2 MiB); 64 probes
take about 65 MiB, a typical FFA map with 15–30 spawn clusters 16–32 MiB. The load
log prints `Reflection probes: N at 128x128, ...`. Capturing costs each of the first
frames after load six small scene renders (a light pass on a 64² corner and a 128²
colour pass each) and one filter, comparable to six floor mirrors; refreshing costs
one face per frame. Per pixel, specular-mapped surfaces take two more texture reads
and a box intersection. These are estimates from the pass structure; nothing was run
on a GPU. `r_materialMapsDebug 4` shows the reflections alone, 5 the scene without
them, live.

As in rend2, frames come from the untransformed texture coordinates (`tcMod`
rotation misaligns them) and an `animMap` stage uses its first frame's maps.
Material-mapped stages stay off the stage table (which only real-time mode
uses): the CPU colour pass draws them with per-stage bind groups and pipelines.

The controls are sampled when the GPU context is created (at start or by a
[graphics reload](#graphics-reload)). A change after that logs `<cvar> changed:
reload the graphics to apply (vid_restart)`; the
configuration file setting them at startup does not. Each map load logs what
it found, for example `material maps (normal+specular+parallax): 429 stages,
429 normal, 57 parallax, 429 specular; frames for 77283 vertices in 18 ms`, or
`no stage of this map has maps` when the cvars are on and nothing was found.

### What to expect, and checking it

`r_materialMapsDebug` is live and draws only material-mapped stages (one uniform
branch in the material program; the ordinary programs do not change): 1 shows the
mapped world-space normal as colour, 2 tints each stage by the maps it found (red
parallax, green normal, blue specular, so normal plus specular is cyan), 3 shows
the normal map's relief, four times its departure from the face, on grey, and 7 how much
of the parallax depth each pixel keeps ([Parallax](#parallax)).
Surfaces without maps keep their ordinary look in every view, so 2 shows at a
glance which surfaces take maps.

The baked response is subtle by construction. rend2 credits all of a lightmap
texel to one direction, so a normal tilted by a small angle α changes the texel
by about tan θ · α, where θ is the angle between the face and the light-grid
direction. A face whose grid direction is more than 78° from its normal falls
back to that normal (`R_LightDirForPoint`) and then only darkens by 1 − cos α. On
the retail `mp/ffa3`, 59% of lightmapped vertices have a usable grid direction
(55% on `mp/duel1`), with a mean tan θ of 1.5 there. Maps generated for ffa3
and duel1 by `sjk-materialgen` (strength 1) are gentle: their normals tilt 3.8°
on average (90th percentile 4–17° per image), so lighting changes by about 5%
on the surfaces that respond. A local headless render (not committed) of six
ffa3 spawn views at 960×540 (Vulkan, RTX 5080), with the generated maps and an
HD texture pack, measured against the cvars off: normal maps alone changed
pixels by 0.06–1.8/255 on average (at most 36/255), specular maps by
0.6–2.6/255 (at most 11/255; the generated `_rmo` maps average roughness 0.68
and metalness 0.02, so highlights are faint). Parallax changed pixels by up to
23/255 on average, but by shifting the texture: it reads as the same texture,
not as relief. Turning the cvars on in baked lighting therefore
shows no obvious change with these maps; that is the model, not a missing draw.
Authored rend2 packs with stronger normal maps respond in proportion to their
tilt.

Real-time lighting shows normal maps far more clearly: the sun share is moved
per pixel, and a low sun lights floors at grazing angles. Indoors, lamps light
walls and floors at grazing angles too, and their share now follows the mapped
normal as well. With `r_dayNight 1`
and `r_dayHour 7`, the sun stands 15° high, so a floor's tan θ is about 3.7
(2.4 at the default hour 7.5) against 1.5 for baked grid directions. This
estimate is from the formulas above; no real-time render of real content has
been measured.

Unit tests cover keyword parsing, lookup order and conversions (synthetic
in-memory images), frames (planar, mirrored and curved), the stage selection
and the pipeline key. naga validates both material programs. A headless Vulkan
render of a synthetic quad on an RTX 5080 (Windows 11, external harness) checked
the response in both modes. Flat maps matched the ordinary program exactly, tilted
normals brightened and darkened as computed (green follows +t, rend2's
convention), and normal maps did not lighten a shadowed buffer texel.

The same harness timed one full-screen 3840×2160 layer (64 runs, medians):
baked lighting 0.103 ms ordinary, 0.196 with a normal map, 0.219 with a
specular map as well and 0.50 with parallax. With the light buffer, those passes
took 0.226, 0.30, 0.34 and 0.54 ms. Measure a real scene with `SJK_FRAME_BUDGET=1`
in a release build: compare the same map, view and population with the cvars on
and off. No authored rend2 pack was available for testing. Generated maps on
real ffa3 data were rendered headless in baked lighting only (above); real-time
lighting on real content, an in-game image and the frame cost in a match remain
unverified.

### Emission maps

A light panel, lamp or screen painted into an ordinary lightmapped texture shows only
as bright as the light falling on it. An emission map makes it glow: `<diffuse>_e`
next to the diffuse image (found like rend2's automatic maps, through
[the lookup](../crates/sjk-viewer/src/material_map_images.rs)) holds the emitted
colour, sRGB-encoded like the diffuse image; black emits nothing. rend2 has no
emission map and no stage keyword for one (its `glow` stage flag only feeds its glow
buffer), so the name is SJK's; it matches the `_e` emission images some community
packs already ship for model textures (which this lookup, on world surfaces only, does
not read). [sjk-materialgen](#generating-material-maps) writes them.

- **Which surfaces.** The same stages as the other material maps (lightmap and
  diffuse collapsed into one opaque pass). A shader that already shows light of its
  own over its paint (a `glow` texture stage, or an additive or `GL_DST_COLOR GL_ONE`
  one) takes no emission map, whatever images exist, so its light is never drawn
  twice; the generator skips the same shaders. A declared light fixture
  (`q3map_surfacelight`) whose overlay does not `glow` is the exception and takes
  one: retail's ceiling lamps and light strips (`mp/s_ylight_red`,
  `mp/s_squareslight_y`, `mp/s_tracklight4`, `mp/s_bluestrip`) add their overlay at
  the paint's brightness or only brighten what the lightmap lit, so in a dark room
  they read as grey paint and do not bloom. Sol asked for those fixtures to emit in
  ten world notes (07/10/2026); since 08/10/2026 (generation 6) they do. `r_emissiveMaps` is on by default because it
  only acts where a pack has `_e` images; with nothing else enabled, a stage with an
  emission map takes the material program with a flat normal and no specular map,
  which reproduces the ordinary lit colour exactly, and the map needs no vertex frames
  and no light-direction target.
- **Surface.** The material program samples the map at the diffuse coordinates and
  adds it after lighting, highlights and dynamic lights (`material_map_finish` in
  [material_maps.wgsl](../crates/sjk-viewer/src/material_maps.wgsl)): no lightmap,
  light buffer, shadow or ambient occlusion dims it. `r_emissionStrength` scales it
  live (bits 24-31 of the scene lighting-mode word, so no buffer changes). It is
  added in the scene's linear units, so HDR, `r_hdrExposure` and eye adaptation treat
  it like any other bright surface. `r_fullbright` and `r_lightmap` show none.
- **Halo.** A stage with an emission map and no authored `glow` is drawn into the
  [dynamic glow](#dynamic-glow) image through a glow variant of the material program
  that writes only its emission, so only the emitting texels get a halo.
  `r_emissiveGlow 0` leaves these stages out of the glow pass (a bit test per glowing
  pass and frame); a stage with an authored `glow` keeps glowing as before.
- **Authored glow.** Lamps whose shader adds a `glow` overlay (`byss/lights`,
  `rooftop/groundlight`, `taspir/light_ceiling`, `taspir/t_wall5`) take no emission map
  and the generator writes none, even with `emission=on` in the overrides: the overlay is
  already their visible light and bloom, and in real-time lighting the
  [fixture inference](#inferring-fixture-light-from-legacy-materials) already makes them
  sources (16 times the overlay's mean colour). Sol's notes asking these lamps for a
  "stronger light" (13/10/2026) are about that inferred power, which an emission map
  would not change. Additive vertex-coloured flares (`flares/solid_blue`, `solid_red`,
  `rgbGen exactVertex`, no lightmap) are neither: their colour is per vertex, unknown at
  material load, so they light nothing.
- **Light, baked lighting.** The lightmaps already hold the light of the map's
  sources, and an emission map adds none: in baked lighting it is only seen.
- **Light, real-time lighting.** With `r_dayNight 1` the lightmaps are replaced by live
  light, so a surface whose emission map is its only light becomes a source.
  `compile_material` ([world_material_compile.rs](../crates/sjk-viewer/src/world_material_compile.rs))
  turns the map into the same textured emission field the
  [fixture inference](#inferring-fixture-light-from-legacy-materials) builds from
  glow stages: the map's colour times 16 (the radiance an explicit glow stage of an
  undeclared fixture gets) times `r_emissiveLights`, laid out by the diffuse stage's
  texture transforms. The existing lamp extraction then integrates it over the
  surface (texture-aware patches merged within 96 units, finite rectangles, the lamp
  grid, static visibility and the lamp cache), and the material's emission enters
  the GI voxels like any fixture's, so probes see it too; GI rays that hit a lamp
  face see no emission, as for other fixtures. A material that already emits (a
  declared `q3map_surfacelight`, an inferred glow fixture or self-lit paint) keeps its
  own light and its emission map adds none. Emission-map lamps are extracted
  separately and capped at the 1,024 most powerful per map (`lights::keep_brightest`
  in [material_maps.rs](../crates/sjk-viewer/src/material_maps.rs)); the load log
  prints `Emission maps: N area lights from M materials (K before the cap of 1024)`.

Cost: without `_e` images, map load does one more image lookup per collapsed stage
and nothing else changes. With them, each emitting stage reads one more texture
(uploaded as sRGB RGBA8 with mips) and draws through the material program; the glow
pass, when one of them is on screen, redraws those stages and runs the blur (see
[Dynamic glow](#dynamic-glow)); in real-time lighting each added lamp costs what an
inferred fixture costs, bounded by the cap and by the lamp grid's per-cell selection.
Load time grows by the patch extraction of the emitting surfaces. Estimates from the
code paths; nothing was measured on a GPU. `r_materialMapsDebug 6` shows the emission
maps alone (black on mapped stages without one).

Unit tests cover the lookup (synthetic images), the shader rule, the light gate and
the lamp cap, the emission field's mean and mask, the strength and glow bits, and naga
validation of the material program and its glow variant in both lighting modes. No
emission map has been rendered: appearance, halo and the light added in real-time
lighting are unverified.

## Tags

Players' tags ([client.md](client.md#tag)) have a pass of their own since 31/10/2026
([tag_render.rs](../crates/sjk-viewer/src/tag_render.rs),
[tag_render.wgsl](../crates/sjk-viewer/src/tag_render.wgsl); Sol: tags were blurry in
the game). They were impact-mark decals with decal shaders, whose pictures the effects'
atlas squeezes into 128-pixel tiles: a 256-pixel tag kept a quarter of its texels,
stretched over 56 units of wall.

- **Geometry:** the decal store still projects each tag onto the world once
  ([decal_store.rs](../crates/sjk-viewer/src/decal_store.rs), `tag_fragments`); the pass
  rebuilds its vertices (a triangle list, the picture's axes and the finish on each
  vertex) only when the store's tags version changes, so a frame with tags on show uploads
  16 bytes (the finishes' clock).
- **Pictures:** each picture on show has textures of its own: the picture (`Rgba8Unorm`,
  its stored values as the effects' are, mipmapped by alpha-weighted averaging so the
  transparent surroundings do not darken its edge) and its outline (`R8Unorm`, the signed
  distance to where its alpha crosses a half, from a quarter of its edge outside to as far
  inside, an exact Euclidean transform). A tag is made square and a power of two from 512
  to 1024 pixels, a smaller one enlarged once with Catmull-Rom (alpha-weighted, so no dark
  fringe) rather than left for the GPU to magnify; a card is a 1024 sticker (its 6:5
  picture at 92 % of the width, corners rounded, a rim of its colour, the outline the
  rounded box's own). `r_picmip` 1 or more halves them. They are built on a worker thread
  the first time one is on show (inline in tests), drawn from the next frame, let go 60 s
  after they were last drawn and all dropped when the packs are mounted anew. Sampling is
  trilinear and anisotropic (the adapter's limit, up to 16).
- **Finishes** (`tag_render.wgsl`), the cards' (`sjk_ui::shine`, `ui_shapes.wgsl`) at the
  scale of a 512-pixel picture: Holo's rainbow foil on the picture only, its light crossing
  where the viewer's angle puts it (it slides as one walks past) and drifting, stronger and
  with more bands than a card's, plus a glitter of points catching the light; Solar's
  corona along the outline, inside the picture, and its glow outside fading within a few of
  its reaches and to nothing before the decal's edge. Normal is the picture alone.
- **Drawn** in the effect layer right after the world's marks (`particle_draw.rs`), so
  heat haze, weather and effects lie over them; depth-tested with the marks' polygon offset
  (`qglPolygonOffset(-1, -2)`), never writing depth, alpha blended, fullbright as the marks
  are. The main view's effect rectangle takes them in (`effect_bounds.rs`); mirror and
  portal views draw them too.

Cost: one draw per tag on show with its picture ready, a few hundred vertices at most;
Holo and Solar shade a few dozen operations a pixel (Solar three noise lookups and three
flares) over the tags' own pixels only. Video memory: about 1.75 MB a tag and 7 MB a card
sticker, for the distinct pictures on show. World shot:
`world_shot::tags::duel6_tags_on_walls_and_floor` (a Normal and a Holo tag, a Holo and a
Solar credits card, a Solar tag on the floor; `SJK_TEST_TAGS` names the hub's
`assets/dist`). Unit tests cover the outline's distances, the pictures' sizes and
edges, and the sticker. Not measured: frame time with many finished tags on show.

## Floor reflections

Polished floors show a real mirror image of the scene
([floor_reflections.rs](../crates/sjk-viewer/src/floor_reflections.rs)). A floor
qualifies at map load when its shader asks for polish the stock way, with a
`tcGen environment` stage, and is otherwise plain opaque world paint: sorted opaque,
no light emission, sky, deforms, glow or alpha test
([floor_reflection_planes.rs](../crates/sjk-viewer/src/floor_reflection_planes.rs)).
Its triangles must lie in one plane (within 0.02 units) facing up (normal z ≥ 0.7);
coplanar faces share one mirror. The load log prints `Floor mirrors: N polished
planes, M with material maps`.

Each frame keeps at most six mirrors, the planes covering most of the screen: a plane
enters at 1% of the screen and leaves below 0.7%, so it does not flicker at the
threshold. Every mirror re-renders the whole scene (sky, world, players, effects)
from the reflected camera at half resolution into a shared target, cropped to the
plane's screen rectangle (the target is made the first frame a mirror is drawn and
dropped when `r_floorReflections` is turned off, see [GPU memory](#gpu-memory)); in real-time lighting it is lit in its own light-buffer
images, so the main view's light stays intact. The finish
([floor_reflection.wgsl](../crates/sjk-viewer/src/floor_reflection.wgsl)) blends the
image over the floor with a Schlick rim from 0.12 head-on to 0.70 at grazing angles,
blurred by a 3×3 kernel whose reach follows roughness 0.4; strength falls by
`1 − 0.6 × roughness`.

With material maps the finish follows the floor material's maps: the normal map bends
the mirror lookup (the change of the reflected ray, mirrored back through the floor
and projected at an assumed 48 units, at most the margin below) and the specular
map's roughness sets the blur (at most 0.6) and its occlusion dims the reflection.
Planes with maps render 1.2 roughness units of margin around their rectangle instead
of 0.4 so the bent and wider lookups stay inside the mirror image. The maps are read
with the stage's untransformed texture coordinates; floors whose diffuse stage
scrolls or scales (`tcMod`) read them slightly misaligned. Floors without maps draw
exactly as before.

`r_floorReflections 0` (live, archived; Settings > Graphics > Image in Advanced, "Floor
mirrors") leaves polished floors with their ordinary material and renders no mirror.
The environment variable `SJK_FLOOR_REFLECTIONS=0`, which predates the cvar, still
forces them off whatever the cvar says; `SJK_FLOOR_COMMANDS=0` disables only the GPU
visibility commands that skip hidden mirrors. Measured mirror costs are in the
sections above (depth priming, light-buffer preservation); the material-map finish
adds two texture reads per mirrored floor pixel and the wider margin.

## Submission and lighting work reduction

The renderer records uploads with their frame and hands completed batches to a
submission thread. It finishes outstanding encoder work, applies those uploads,
submits and presents in order while the render thread prepares the next frame.
Only one handed-off frame can remain outstanding. With an offscreen scene target,
swapchain acquisition happens after world recording; direct-to-surface rendering
still acquires its image first. Resize, out-of-band submissions and teardown wait
for the outstanding batch, and so does a world handing the window over to the next
one (a map change, a graphics reload, the menu world coming back after leaving a
server): both draw to the one surface, and configuring it while the old world's frame
was still being presented made wgpu refuse the configure and close the game. A
renderer's teardown also joins its submit thread, so the queue clone it holds (and
with it the last handle to the window) is never released on that thread after the
event loop has gone, which made winit panic on exit from exclusive fullscreen. `SJK_SUBMIT_THREAD=0` selects inline submission as a
fallback. See [frame_queue.rs](../crates/sjk-viewer/src/frame_queue.rs),
[frame_split.rs](../crates/sjk-viewer/src/frame_split.rs) and
[frame_target.rs](../crates/sjk-viewer/src/frame_target.rs).

Upload staging reuses byte and operation storage after warmup. Queue clones share
a synchronized recording; this replaces immediate wgpu upload work on the render
thread, rather than making all queue operations lock-free. Large load-time writes
can bypass recording once pending submissions have completed.

The crosshair is retail's `gfx/2d/crosshair*` picture, drawn by the HUD draw list
as a textured quad ([hud/crosshair.rs](../crates/sjk-viewer/src/hud/crosshair.rs));
the in-game HUD shader draws its procedural cross only when that picture is missing.
The rocket launcher's lock-on ([hud/rocket_lock.rs](../crates/sjk-viewer/src/hud/rocket_lock.rs))
is drawn as JoF EJK's `CG_DrawRocketLocking`: from `ps.rocketLockIndex` and
`ps.rocketLockTime`, one red `gfx/2d/wedge` slice per 75 ms (150 in Siege) around the
target, each turned another 45 degrees about the ring's centre with the
`TexturedQuadTurned` draw command (`R_DrawRotatePic`'s turn about a corner), up to
eight, then the additive `gfx/2d/lock` pulsing over them (its brightness becomes its
coverage, as the HUD blends by alpha). Each new slice plays
`sound/weapons/rocket/tick.wav` at the target and the eighth `lock.wav`. The ring stays
square on wide screens; a teammate's lock is not drawn in team games; vehicle weapons'
own lock times are not covered.
`DrawCommand::Polygon` (28/10/2026, for the nameplates' holo frame) fills a convex
polygon of up to six corners (`sjk_ui::POLYGON_POINTS`): a fan of triangles from its
middle, each carrying its distance to its outer edge so the shader (`ui_shapes.wgsl`,
mode 7) fades the last pixel inside, and with a texture the picture is spread over the
polygon's box from the texture coordinates given for its corners. Like explicit
texture coordinates, it is not clipped. `menu_snapshot` draws it the same way on the CPU.
The in-game HUD shader is restricted to crosshair and damage-indicator regions.
Intersecting regions become one rectangle so translucent pixels blend once.
Menus and the shader's status-bar fallback retain full-screen coverage.

Uncached deferred-light receivers are collected into a pixel list and shaded by
four compute lanes per receiver. Cached receivers keep their existing path. The
indirect dispatch uses bounded rows to support large light buffers; the final lamp
sum can differ slightly in floating-point rounding from a serial sum. Receiver
depth identifies valid texels, allowing attribute and light targets to retain data
outside regions that will be overwritten or sampled. Floor mirrors use separate
light images, preserving the main view without save/restore copies.

Static sun-shadow bounds form an exact min/max mip hierarchy, starting at 8-texel
tiles. A receiver chooses a level covering its filter footprint with at most four
tiles. It skips the existing filter only when those conservative bounds determine
the result; shadow radii, tap counts, cascades and visual settings are unchanged.


Verification on 2026-10-02 compared this change against `f3f3db2`, using external
release replay instrumentation on Linux, Ryzen 5 5500 and Radeon RX 9060 XT
(RADV). Each 2560×1080 run measured 3,330 frames after replay warmup, with a
31-player roster and unchanged graphics settings; visible/submitted actors ranged
from 20–23 on `ffa3` and 16–27 on `ffa1`.

| Route | Mean total frame | Mean GPU | Total-frame p99 |
| --- | --- | --- | --- |
| `mp/ffa3` | 2.464 → 1.822 ms | 1.780 → 1.697 ms | 3.517 → 3.316 ms |
| `mp/ffa1` | 3.042 → 2.157 ms | 2.337 → 1.986 ms | 3.757 → 4.263 ms |

These paired offscreen runs establish a mean improvement on the sampled routes,
not universal sub-2 ms performance or improved tail latency. The `ffa1` p99 was
higher despite its lower mean; longer native play and more hardware remain open.
They exclude live networking/audio and window presentation. Comparison captures
at 2560×1080 and 3840×2160 retained the scene appearance: maximum channel error
was 4/255 on `ffa3`, 1/255 on `ffa1` outside its wall-clock kill-feed text, and
3/255 in the 4K `ffa1` snapshot. This is finite image coverage.

External checks covered overlapping HUD regions and viewport bounds at five
resolutions through 8K, indirect-dispatch boundary cases and 7,308 shadow-bound
footprints including non-power-of-two maps. Formatting, locked workspace build
and tests, and release client/server builds passed. Verification harnesses and
reports are kept outside the source repository.
Two 45-second native `ffa1` replay process runs also passed without panic or GPU
validation errors: threaded submission with HDR/FXAA, and inline submission with
HDR/FXAA disabled to exercise direct surface acquisition. Both used isolated
1280×720 settings; these are integration checks, not performance measurements.

## Weather

SJK draws the rain, snow, dust and blowing mist that maps ask for
([weather.rs](../crates/sjk-viewer/src/weather.rs)). A map's `fx_rain`, `fx_snow`,
`fx_wind` and `fx_spacedust` make the server register effect names starting with
`*` (`*heavyrain`, `*heavyrainfog`, `*constantwind ( -5000 0 0 )`); the client runs
them as world effect commands in slot order, as cgame's `CG_ParseWeatherEffect` and
the renderer's `RE_WorldEffectCommand` do, and again whenever an effect name
changes. A world without a server (the menu backdrop) takes them from its own
entities. `r_we <command>` adds one from the console until the next map. A map edit's
weather ([map-edits.md](map-edits.md#weather), synced through the hub) runs its commands
in place of the map's, under the player's own `r_weatherForce`; SJK adds one command for
it, `mist`, the light ground fog `r_weatherFog 2` lays (`Mist::DEFAULT`). The clouds
keep the original parameters: counts, sizes, gravity, colours, mass ranges, spawn
boxes, five clouds and ten wind zones at most
([weather_effects.rs](../crates/sjk-viewer/src/weather_effects.rs)). Gusting wind
eases at the original 10 units per update, stepped at 60 Hz rather than once per
frame ([weather_wind.rs](../crates/sjk-viewer/src/weather_wind.rs)). The map's
global fog (`textures/fogs/rail` on `t1_rail`) is the fog described elsewhere on
this page; weather does not change it.

**Cover.** Weather exists only under open sky, above the first surface below it
([weather_cover.rs](../crates/sjk-viewer/src/weather_cover.rs)). For each 16-unit
column the client walks down from the top of the world to the first open point of
a visibility cluster whose upward trace hits something: a sky surface (`SURF_SKY`)
opens the column, any other ceiling covers it. A downward trace against solids and
liquids then finds the floor. Rain therefore stops on roofs, ledges, the ground and
water, never shows indoors, and is cut per pixel at eaves and windows. The
original's inside and outside brushes (`system/inside`, `system/outside`, 51 on
`t1_rail`) within the map's `misc_weather_zone` boxes trim that span as `COutside`
reads them; the original relied on them alone, so a map without them rained
indoors. A map with no sky surface at all leaves weather everywhere, as the
original does without marks. Brush entities (doors, lifts, `func_static`) are not
surveyed.

A worker thread surveys 32×32-column tiles nearest the camera first and caches
them for the map ([weather_cover_map.rs](../crates/sjk-viewer/src/weather_cover_map.rs));
an `Rgba32Float` texture holds a 4096-unit window around the camera, addressed
modulo its size, and a column not surveyed yet reads as covered. Nothing is
surveyed on a map without weather. Measured locally on 06/10/2026 (release build,
Windows): 2.5 µs per column on `t1_rail` and 18 µs on `hoth2`, so a whole window
(65 536 columns) takes 0.16 s and 1.2 s of worker time, the camera's own tile
16 and 18 ms.

The fog reaches further than the window (6000 units), so a second worker surveys a
far cover once per map ([weather_cover_far.rs](../crates/sjk-viewer/src/weather_cover_far.rs))
into a texture of its own: 256×256 columns centred on the map's box, 64 units wide
or as wide as the box needs (80 on `T2_Rogue`). A surveyed column that finds none
of the map's air (it lies past the walls) is marked void, and so is a window column
not surveyed yet. The fog reads the far cover beyond the window and in void columns;
far columns past the walls all take the map's median open floor up to its highest
sky. Nothing in either cover depends on the camera. Measured locally on 07/10/2026
(release build, Windows): 1.5 s on `T2_Rogue` (16 576 × 13 816 units), 0.18 s on
`siege_hoth`, under 20 ms on `ffa5` and `duel6`; until it arrives there is no fog
beyond the window.

**Drawing** ([weather.wgsl](../crates/sjk-viewer/src/weather.wgsl),
[weather_gpu.rs](../crates/sjk-viewer/src/weather_gpu.rs)). Particles have no
buffers: each is generated from its instance number in a box around the camera,
anchored in the world and carried by the flow the CPU integrates per cloud and
mass. Their speed is the original's terminal one, 7/3 of force over mass (the
original keeps 0.7 of its velocity each frame), so `t1_rail`'s rain flies at about
60° in its 5000-unit wind. They are drawn into the display-space effect layer after
the impact marks and before the other effects, sabers included; `RB_RenderWorldEffects`
draws them after every surface, but SJK's effects write no depth, so the weather drawn
after them lay over every blade and smoke puff in front of the world behind: the haze,
measured to that wall, greyed sabers and smoke as if they stood at it, and rain streaked
across them (Sol, 11/10/2026). Drawn before, a drop nearer than a blade is hidden by it
instead. Snow, dust and sand keep the
original's blending; rain and splashes do not: the original adds grey, which a dense
storm piles up into white, so SJK blends each streak over the scene as a faint
blue-grey of the light at the camera, some drops catching more of it than others.
SJK adds:

- Rain streaks along the velocity, at least about a pixel wide (a thinner one is
  fainter instead), and a fainter far layer out to three times the original range.
- Splashes where rain meets the ground or water, within the near box and 1200 units
  of the eye's height. Each is geometry in the world, not a sprite, so it keeps its
  shape and parallax from any angle: a ring flush with the ground over a darker wet
  spot, a crown of water as eight curved wall segments that flare out, rise and
  collapse (a thicker beaded rim, brighter where a segment is seen edge-on), and six
  drops thrown out on parabolic arcs as short streaks. On water the ring is two
  widening ripples, the crown lower and the drops higher. Past 400 units only three
  drops are drawn, past 700 only the ring. Like the streaks, splashes are anchored
  in the world: one stays where it struck while the camera moves.
- Fades at the box edges, near the eye and into the global fog; light from the
  camera's light-grid sample, so rain is dimmer at night; soft edges where mist
  meets geometry.

**Volumetric fog** (`fragment_volume` in weather.wgsl). One full-screen pass before
the particles (and so before the effects, which it no longer covers) marches each pixel's ray to the surface it shows (at most 6000 units)
and counts only open-sky air: the cover keeps fog out from under roofs as it keeps
rain out. Falling weather leaves a haze in that air (half the light lost over about
8000 units in drizzle, 4300 in rain, 2500 in a storm, 3500 in snow; values in
`weather_effects.rs`). The fog commands (`fog`, `heavyrainfog`, `light_fog`) become
ground fog instead of the original's drifting smoke sprites: densest at each
column's floor, thinning over 110 to 180 units, billowing through the noise volume
and drifting with the wind; `light_fog` keeps its blue-green. Beyond the window it
lies on the far cover's floors, and past the map's walls as one level bank, so
distant fog and haze reach the horizon and keep their height wherever the player
stands or jumps. (Until 07/10/2026 the air beyond the window was floored at the
column under the camera: stepping off a roof over a street on `T2_Rogue` moved all
distant fog down 500 units, and a bright band at eye height came and went.)
`r_weatherFog` decides the ground fog: 0 none (no fog sprites either), 1 the map's
(default), 2 also a light fog on every map with sky.

The march runs at half the scene's size (05/11/2026, `fragment_volume_half`, into an
`Rg16Float` image of its own, freed when no fog is drawn): each texel stands for a 2x2
block of the depth and marches to the block's nearest point or its farthest, in a
checkerboard, keeping its alpha and how far its ray went. The full-size pass blends the
four texels round each pixel by nearness and by how close their ray's length is to its
own (a joint bilateral blend, tolerance 5% of the length and 16 units), the closest one
alone where none is close, so the fog keeps to the edges of a pole or a player in front
of a foggy street instead of haloing them. A ray takes a step per 110 units, at least
four and at most the quality's count, so a wall near the eye costs four steps, the
horizon all of them; the ground fog's noise is read only where the fog has not thinned
below 1% of its floor's, and the march stops once 99.7% of the light is lost. Towards
the sun the fog is lighter and away from it darker (forward scattering, Henyey-Greenstein
with g 0.35, `sun_glow`: half of it at the sun's full strength, a third of that in a
full storm), and the rain catches the light between the eye and the sun (up to 1.8 times
as bright in a 30-degree cone). Measured on 05/11/2026 (release, Windows, RTX 5080, off-screen at 2560x1440 on `T2_Rogue` in a forced storm, `t2_rogue_storm_timing`, medians of the effect pass `scene-pass`, which holds the fog with the rain and the splashes): 0.266 ms before and 0.105 after at medium, 0.385 and 0.125 at high, 0.84 to 1.05 and 0.137 at ultra. A game ran on the same GPU meanwhile, so whole-frame times varied; the pass times held.

**Quality** (`r_weatherQuality`, [weather_settings.rs](../crates/sjk-viewer/src/weather_settings.rs)):

| Level | Draws |
| --- | --- |
| 0 low | Near streaks, the original's fog sprites; clouds with 10 samples |
| 1 medium | Splashes, volumetric fog with 10 samples a ray at most, wet surfaces; clouds 16 |
| 2 high (default) | The far rain layer, fog 16, spray over the ground, water running down slopes and walls; clouds 24 |
| 3 ultra | More far rain and splashes, fog 24, puddles with rain rings, every wet surface mirroring the world (`r_weatherReflections`); clouds 40 |

**Wet surfaces** (`fragment_wet` in weather.wgsl, from `r_weatherQuality` 1). Rain
wets what it falls on, in one full-screen pass drawn into the scene right after the
opaque world and before the players, so the depth it reads holds only the world and
players and models stay dry. Each pixel rebuilds its point and face from the depth
(the neighbour on each axis nearer in depth, so edges keep their face) and is wet
where the air 6 units in front of it is under open sky: the cover, blended between
the four nearest columns so a roof's shelter ends in a soft line, and the far cover
beyond the window. Floors under roofs, the undersides of ledges and indoor surfaces
stay dry; faces the rain slants onto get wetter, the lee side of a wall drier.
Drizzle wets about half as much as a downpour (`SOAKING_HAZE`), snow not at all, and
a map without sky (no cover) is never wet. Wet surfaces darken by up to 36% as water
fills their pores, and a film mirrors the overcast sky (the clouds' skylight, dimmer at
night and in a storm) with water's Fresnel term, strongest at grazing angles and only
where the mirrored ray points above the horizon. Level 2 adds running water
(`running` in weather.wgsl, reworked 05/11/2026, Sol: "the rain flowing on angled planes,
it's not good enough"): on every slope from about 10 degrees a thin film, glossier in
bands that run down and broken up by the noise; on walls and slopes past about 30 degrees
rivulets, in lanes 14 units wide across the way down, two lanes in five carrying one along
a line that wanders across its lane as it goes down (fixed on the surface: the water
keeps to its paths), in stretches of 240 to 460 units with dry gaps, at least about a
pixel wide (fainter instead); along each rivulet three to seven drops a scroll's wrap run
down, a bright head with a short tail, catching the overcast light, until they are too
small to see. Everything runs down at 110 units a second of height, the scroll and the
film's change kept wrapped on the CPU (520 units, one noise tile; the drops' spacing
divides it), so nothing jumps when it wraps.

The way down is laid in one of 16 headings round the vertical: across it runs level
along the surface, down it runs the height, so a lane follows the slope's fall line on a
ramp facing any way, not only on walls facing x or y. The face's normal, which the depth
gives a little unsteadily, only chooses the heading nearest its own (and blends the two
nearest within a twentieth of a turn of half way); a flat face's heading never changes,
so the water stays put however the camera moves. Until 05/11/2026 the streaks were one
noise stretched 40 by 520 units, laid across y or x and blended between the two by the
normal, so a ramp facing a diagonal showed two crossed patterns and gentle slopes none
at all. (The version before 08/10/2026 took its coordinates from the normal times world
positions in the thousands and its scroll from the time since the map loaded times a
slope-dependent speed: it flickered and slid as the camera moved.) Running water fades out
between 600 and 1600 units and at grazing views, where it would shimmer. Level 3 adds puddles on
flat ground (about a third of it, in noise patches 1100 units across), darker and an
almost full mirror, with thin rain rings within 700 units. The pass blends
`scene × alpha + colour`; the price is that light painted into a wet wall's texture
dims with it (by at most about a sixth on a wall). Not drawn: wet models.

**Puddles' mirror** (level 3, 05/11/2026, Sol: "take a look at the water effects of
VisionJK and import some of it, to have nice water on the ground when there is rain";
[weather_mirror.rs](../crates/sjk-viewer/src/weather_mirror.rs), `mirrored` in
weather.wgsl). VisionJK has no rain-driven water of its own (its weather is the stock
`tr_WorldEffects`); its water gun's wet ground (`wet.frag`, by Blackwolf) mirrors the room
in its pools with a screen-space march, which SJK's puddles now use. Right after the
opaque world, before the wet pass, the scene is copied at half its size (one bilinear tap
per four texels, `fragment_mirror_copy`, about 17 MB at 3840×2160 while it rains at
ultra, freed otherwise). Each puddle pixel within 2500 units (fading from 1500) marches
its mirrored ray through the depth: 32 steps from 2 units, each 1.18 times the last
(about 2000 units in all), a step counting as a hit when the ray lies behind the drawn
world by less than twice the step and 16 units (deeper, it passes behind something
nearer the eye and goes on), then five halvings round the hit; the copy is read there.
A ray that meets nothing within the march takes what lies that way far off: the spot on
the screen of the eye plus the ray's direction, where the screen shows the sky or a
surface farther than the march went (a distant building; a nearer surface there stands in
front of the ray and is refused). The copy is the world before the fog, so what the puddle
mirrors is greyed towards the overcast sky by the rain's haze over the distance the light
came. A hit fades within 8% of the screen's edge and over the march's last 600 units.
Where something is found the reflectance is VisionJK's pond blend, `clamp(2 F + 0.2, 0,
0.92)` with F water's Fresnel term, so a puddle mirrors a fifth even seen from above and
almost all at grazing angles; where nothing is, the puddle keeps the sky-only mirror it
has without the copy. (The first version, 05/11/2026, gave unfound rays the overcast
colour at the pond's reflectance and looked for the sky 100 000 units away, past the far
plane, so it never found it: puddles turned into flat pale slabs. Its edges also ended in
a band of 3% of the noise, softened to 6%.) Within 700 units the rain rings tilt
its normal (their slope, a third of it), so the reflection ripples where drops land.
Players and models are drawn after the wet pass and are not in the copy: puddles mirror
the world and its sky, not the players. The copy needs a scene target the client can
sample (the render-scale or post-AA scene); a frame drawn straight to the window keeps
the sky alone. The world shot `t2_rogue_storm_puddles` (`world_shot_weather.rs`) renders
four views of a storm on `T2_Rogue` with and without the mirror. Measured on 08/10/2026 (release, Windows, RTX
5080, off-screen at 3840×2160 on `T2_Rogue` in a forced storm, `SJK_GPU_PHASES` marks
`world-opaque`/`rain-wet`): 0.19–0.26 ms a frame at levels 1 and 2, 0.27–0.37 ms at 3,
in GPU frames of 5.5–7.9 ms.

**Looks of 09/11/2026** (Sol: "the rain, weather, fog to have more options to look even
better (while keeping perfs in mind) and the water flowing on wall too, maybe with some
reflection, smarter ways for it to happen"; `personal/weather-looks`):

- *Uneven wet ground.* The ground is no longer one even film: a broad noise (the puddles'
  1100-unit one) and the foot of higher ground beside a pixel (the cover's four columns,
  `shelter` in weather.wgsl: a column whose floor stands 12 units or more above the point)
  make it damp, glossy (film up to 0.42 instead of 0.22) or, at ultra, a puddle; water
  gathers at the foot of walls and kerbs. `r_weatherPuddles` scales the standing water.
- *Smarter running water.* Each rivulet starts a little under the wall's lip (8 to 160
  units, from the column behind the face), widens by half towards the wall's foot where the
  water has gathered, and the bottom 4 to 34 units of a wall are soaked darker and glossier
  by the rain splashing off the ground.
- *Reflections in every wet surface* (`r_weatherReflections` 3, the default at ultra). The
  puddles' copy of the scene now carries up to four levels, each half the last
  (`Mirror::encode_blur`); a wet street or wall marches a shorter, coarser ray (16 steps
  from 4 units growing by 1.38, three halvings, `FILM_*`) and reads a softer level the
  rougher its water and the farther it is, while a puddle keeps its sharp march. Its
  strength is `clamp(1.4 F + 0.04, 0, 0.8)` times how much water lies there, so lit windows
  and signs streak across a soaked street. A rivulet's surface leans across it, so it
  mirrors a sliver of the world beside it. 2 keeps the world to the puddles, 1 the sky's
  colour alone (no copy at all, as at high).
- *Spray.* A downpour raises a thin spray over the ground it beats on (from high; thinning
  to a third every 10 units, density `SPRAY_DENSITY` times the square of the rain's share
  of a storm), worked out exactly along each pixel's ray over the floor where the ray ends
  (`spray_over`), drifting in gusts through the noise. It shows where the view skims the
  ground and costs one noise read per pixel in the fog's blend.
- *Rain in waves.* The streaks thicken and thin in two trains (1100 and 430 units long, a
  quarter turn apart) carried by the wind, up to half again as dense at full wind, a little
  even in calm air in a storm (`rain_waves`).
- *Settled snow* (`r_weatherSnowCover`, off). Snow settles in about 6 seconds on what faces
  up under open sky, deeper in drifts (a 420-unit noise) and against walls, grainy, lit by
  the overcast sky (`SNOW_TINT`); it melts away as fast once the snow stops. Drawn by the wet
  pass, so it runs at every quality from low when the snow falls.
- *Sliders*: `r_weatherRainOpacity` for the streaks, `r_weatherHaze` for the haze and spray,
  `r_weatherFogDensity` for the ground fog.

Measured on 09/11/2026 (release, Windows, RTX 5080, off-screen at 2560x1440 on `T2_Rogue`
in a forced storm, `t2_rogue_storm_timing`, medians of 14 samples per view; main 1a485b087
then the branch, one after the other): the wet pass `rain-wet` 0.13-0.14 ms -> 0.15-0.16 at
high, 0.27-0.28 -> 0.36 at ultra with every wet surface mirroring, 0.29-0.30 with puddles
alone and 0.18-0.20 with the sky alone; the effect pass `scene-pass` (fog, spray, rain)
0.12-0.13 -> 0.15-0.16 at high. World shots `t2_rogue_wet_reflections` and
`t2_rogue_snow_cover` (`world_shot_weather.rs`).

**Forced weather** (`r_weatherForce`): 1 drizzle (`lightrain`), 2 rain with a
random wind, 3 a storm (`heavyrain`, gusting wind), 4 snow with wind, in place of the
map's weather; `r_we` commands still add to it. Forced weather is drawn only on maps
with sky, so it never falls indoors on a map without one; so is a map edit's weather.

**Clouds** ([weather_clouds.rs](../crates/sjk-viewer/src/weather_clouds.rs),
[clouds.wgsl](../crates/sjk-viewer/src/clouds.wgsl), `r_clouds`, off by default and no
graphics preset's since 05/11/2026: a player's own choice).
A layer of cloud 6000 units above the camera and 3200 deep covers every map with sky
faces, except maps whose weather is space dust. It is drawn on the visible sky faces
right after the sky, with the sky faces' own geometry and depth test, half as far
pushed back as the sky (see [Sky faces' depth](#sky-faces-depth)), so it never
covers the world: each pixel marches its view ray through the layer (fading out
towards the horizon and with distance), with two looks towards the sun for its
shadowing, a forward-scattering rim, darker dense insides and skylight. The sun is
the lighting passes' (the day clock's, or Camera control's) or else the sky's
authored one; night leaves only a dim skylight. The layer drifts across the world at
a slow breeze plus half the weather's wind; rain and snow raise the cover from 0.42
to up to 0.92 and darken it. Its colour is in the scene's light units (the sky's
radiance scale), so it goes through bloom, exposure and tone mapping with the sky.

Fog and clouds share one noise volume ([weather_noise.rs](../crates/sjk-viewer/src/weather_noise.rs)):
64³ RGBA8, red Perlin-Worley noise for the shapes, green to alpha inverted Worley
noise at three frequencies for the eroded edges, after Schneider's published cloud
technique (2015) and written for SJK. It tiles in every direction, is made once per
run on a worker thread, and the clouds and the fog's billows appear when it is ready.

Rain and wind can be heard with `s_weatherSounds` ([client.md](client.md#weather-sounds)),
quieter under a roof by the same cover.

**Lightning** (`r_weatherLightning`, off by default; 05/11/2026, Sol: "add more
atmospheric stuff"; [weather_lightning.rs](../crates/sjk-viewer/src/weather_lightning.rs)).
In a storm (a haze at least three quarters of a downpour's: `heavyrain`, the map editor's
Storm, `r_weatherForce 3`) a strike comes every 7 to 22 seconds (a fiercer storm more
often; the first 4 seconds in): two or three pulses 70 to 230 ms apart, each falling to a
third in 70 ms, brighter when it falls near, over within 0.9 s. Its light is not a light
in the world: it lights what the storm puts in the air and on the ground, the fog and haze
(3.5 times as bright at a full flash), the falling rain (2.5 times), what wet surfaces and
puddles mirror (5 times) and the volumetric clouds from within. The world's own lighting
and the skybox stay as they are; instead the fog pass lays a cool white veil of the flash's
light over the view, up to 12% of it over a wall near the eye and 55% where the ray runs
6000 units or more into the open sky (`fragment_volume`, the flash in `rain.w`), so the
storm's air lights up as it does seen through rain. Its thunder follows after the time the sound takes to come (0.25
to 4 s: the game's `thunder_close` for a strike within a quarter of the range, else
`thunder1` to `4`, quieter the farther), with `s_weatherSounds`
([client.md](client.md#weather-sounds)). `cg_reduceFlashes` keeps a flash to a third of
its light, easing in and out over 0.15 s instead of flickering. Frozen weather holds the
strike; weather chosen for a map without sky (forced, or a map edit's) never flashes; the
multiplayer game has no lightning of its own to follow (single player's `fx_rain` flag is
ignored there), so nothing on a server asks for it.

`r_weather 0` turns weather off (the clouds stay with `r_clouds`);
`r_weatherDensity` scales the counts (1 is the original's, SJK's default 2).

`cg_saberRainSteam` adds steam and a sizzle to exposed saber blades during rain:
`1` (default) for everyone, `2` for your own held and thrown sabers, `0` off.
Both settings interfaces offer **Saber rain steam** under Game options / Effects.
The presentation follows [JoF EJK PR #257](https://github.com/JediofFreedom/JoF_EJK/pull/257):
each blade samples a point between 10% and 100% of its visible length every
300–600 ms and plays the stock `saber/fizz` effect upward. Dual and multi-blade
hilts have independent timers; throwing or catching retains the primary blade's
timer. Ghosts, trails, hidden blades and menu previews do not emit rain steam.
The sampled point must be dry and outside indoor volumes, with an upward point
trace reaching sky (or escaping from a marked outside volume). Static roofs and
presented solid brush models, including doors and permanent BSP instances, block
it. Frozen weather and repeated presentation times do not emit new strikes;
rewinding a demo resets the deadlines.

SJK reads its active weather emitters, including `r_weatherForce` and `r_we`, so
`rain`, `acidrain`, `lightrain` and `heavyrain` can cause steam. Snow, fog and wind
alone cannot. `r_weather 0` suppresses it. `clear` removes rain while preserving
freeze; `die` also clears freeze. This affects presentation only.

Secondary views (portals, mirrors, sky portals) show no weather or clouds. Not
implemented: the outside camera shake (`outsideshake`), acid rain's pain hint, lightning
lighting the world itself (see Lightning above), wind zones with bounds, and clouds
casting shadows on the world. A map's skybox may already paint clouds; the volumetric ones go over them.
Full visual and cost verification of the weather remains incomplete. The cover,
commands, wind, settings,
noise, cloud and fog parameters and the translation of both shaders have unit
tests; how fog and clouds look, and what they cost at 4K, is untested. Off-screen
world shots of `T2_Rogue` with `r_weatherFog 2` (07/10/2026), from cameras 24 units
apart either side of a roof edge, showed the same distant fog from both after the
far cover and the band flipping before it. Compare `r_clouds 0` and
`r_weatherQuality 0` against the defaults to see the cost.

## Sky scenery and hillside orientation

Local correction on 2026-10-03, based on `8f692ac` plus the preceding lighting
preview, addresses the three reported `t1_danger` views.

Indirect draw storage now resets during scene preparation, before either sky or
main-view commands are recorded. Resetting in the main sun-caster pass could reuse
an earlier sky view's buffer region: queued main-view uploads then changed the
sky draw arguments on camera turns. The static world's identity instance also
permits sky-portal rendering, matching the table path when drawing directly or
falling back from indirect draws. Entity and mover visibility rules are unchanged.

Lighting now orients smoothed normals using the triangle plane reconstructed from
fragment position derivatives. Testing the smooth normal's own dot product with
the eye incorrectly flipped visible hillside normals at grazing angles, creating
camera-dependent dark bands. Pre-pass normals, receiver lighting, lamp-cache side
selection and material upsampling use the same geometric-side decision. This does
not change sun-shadow filter widths, exposure or source intensity.

External release GPU captures on Linux/RADV, Radeon RX 9060 XT, at 1280×720,
day hour 10.6 and unchanged owner graphics settings established:

- The unwanted dark bands disappeared in both marked terrain views. Disabling AO,
  contact shadows or sun maps separately did not remove the old bands.
- A 24-view sky test captured the first frame of each alternating camera turn,
  covering offsets through ±36 degrees. Restoring only the old buffer reset
  reproduced missing mountains in 12 views (over 100,000 affected pixels each).
  Corrected indirect and direct draws agreed within 11/255 maximum channel error;
  only one pixel across the 24 comparisons exceeded 8/255.
- The sampled `mp/ffa1` view differed by at most 1/255. The previous `mp/ffa3`
  structure-shadow mark differed by at most 22/255, with only 28 pixels above
  8/255. The sampled day/night-disabled `mp/ffa3` image was byte-identical.
- Settled GPU medians for the distant and close terrain views were respectively
  0.713 → 0.713 ms and 0.695 → 0.691 ms (11–12 samples per side). These fixed-view,
  nonexclusive measurements are not a populated-match performance certification.

Formatting, locked workspace build, all nine existing tests and doc-test targets,
and the optimized owner client build passed. Native owner confirmation and wider
map/hardware coverage remain open. Verification overlays and assets stay outside
versioned source.

### Sky faces' depth

The main view shades the sky on its faces after the opaque world, depth-tested, so the
sky still covers whatever lies behind a face. rd-vanilla instead draws its box at the far
plane, where it never covers the world. The faces' depth is computed from absolute world
coordinates, which round it by some tenths of a percent of its distance: a ceiling built
a unit or two under the sky brush it lies against (`jofmemorial`, z 832 under a sky face
at 834, seen 2300 units away) lost the depth test to the sky in places, and with the
clouds on, which were drawn a fixed depth step nearer than the sky, it showed as white
sky flickering as the camera moved (Sol's note of 12/10/2026). Every program that draws
on the sky faces now pushes their depth back by 1/32 of the way to the far plane
(`sky_face_clip`, `SKY_FACE_PUSH`): the sky faces and their depth mask
([sky_stage.wgsl](../crates/sjk-viewer/src/sky_stage.wgsl)), the sky portal's composite
on them (`sky_portal_main` in [scene_view.wgsl](../crates/sjk-viewer/src/scene_view.wgsl),
a pipeline of its own beside the map portals') and the clouds, at half that push
([clouds.wgsl](../crates/sjk-viewer/src/clouds.wgsl)). The push is about 3% of the
distance, ten times the rounding; geometry that close behind a sky face now shows
through it, as everything behind the faces does in rd-vanilla.
[sky_face_depth_tests.rs](../crates/sjk-viewer/src/sky_face_depth_tests.rs) checks the
programs agree and, with the client's projection in f32 at the jofmemorial note, that the
pushed sky and clouds stay behind the ceiling over 2000 camera positions (unpushed, the
sky came in front at 17 of them).

### Sky shader stages (cloud layers)

A sky shader's own stages are its cloud layer: rd-vanilla's `RB_StageIteratorSky` draws
the outer box, then `R_BuildCloudData` draws the stages over it with the generic stage
iterator, on the box's faces (the bottom one left out), with texture coordinates where the
view ray meets a dome `cloudheight` above a world of radius 4096 (`R_InitSkyTexCoords`,
the arc cosines of the hit point's unit x and y). Until 27/10/2026 SJK drew only the box,
so a scrolling sky such as `baronshed_share`'s `textures/DF2_baronshed/beach_sky`
(`tcMod scroll` clouds under a mask) stood still. The sky program now composites the
stages per pixel in `with_clouds` ([sky_stage.wgsl](../crates/sjk-viewer/src/sky_stage.wgsl)),
in the box pass and on the sky faces: the dome coordinates from the view direction, each
stage's tcMods (scroll, scale, rotate, turb, stretch, transform, as the world's stages
apply them), `rgbGen`/`alphaGen` const and wave, the alpha test, and its `blendFunc`
computed in the shader over the colour so far; the first stage without a blendFunc covers
the box, as in retail. [sky_clouds.rs](../crates/sjk-viewer/src/sky_clouds.rs) packs up to
eight stages and one image each (an `animMap` shows its first frame; a stage whose image
is missing is left out). Retail interpolates the dome over a coarse grid per face; the
per-pixel form is the exact curve. The ignored test `baronshed_clouds_move` renders the
beach sky twice, four seconds apart, from open ground it finds under that sky.

### Sky without a box under global fog

A sky shader may have neither an outer box nor stages: `taspir1`'s `textures/skies/taspir1`
is `skyParms - 512 -` alone, under its worldspawn fog `textures/fogs/taspir1_fog`
(`fogparms ( 0.8 0.7 0.4 ) 4096`, `distancecull 4096`). rd-vanilla draws nothing there;
what shows is the clear colour, which is the global fog's `parms.color` whenever the map
has one (`RB_DrawBuffer` and `RB_BeginDrawingView`, tr_backend.cpp:547-553,1812-1818), so
the sky is the fog and fogged geometry fades into it. SJK cleared the world pass to its
night tint instead, so that sky came out dark blue and distant fogged walls stood against
it as sand-coloured cut-outs, as if the fog were see-through (hub reports #179, #180,
#184). The world pass now clears to the global fog colour, unquantized as the reference
uses it ([fog_volumes.rs](../crates/sjk-viewer/src/fog_volumes.rs) `Table::global_color`,
[frame_target.rs](../crates/sjk-viewer/src/frame_target.rs) `world_load`); maps without
global fog keep the night tint, and boxes and clouds still cover the clear as before.

## UI ownership

`sjk-ui` provides renderer-independent retained widgets. The viewer supplies GPU
and text integration and binds client state to the HUD. Layouts are data in
[assets/hud](../crates/sjk-viewer/assets/hud); menus and HUD may be modern while
movement, combat and network behavior remain compatible.

HUD text sizes and `px` layout units scale with the HUD factor: viewport height
over 1080, clamped to 2/3..2.5, times `cg_hudScale`. The crosshair name keeps
stock's size in that frame: `CG_DrawCrosshairNames` draws `ergoec` (point size 20)
at scale 1.0 in the 640x480 screen, so its line is 45 px at 1080p and 60 px at
1440p, times the layout's `type_scale`. The classic layout uses 0.9 because
`arialnb` has taller capitals per line than `ergoec`; Inter uses 1.0. Its
position is stock's too: the line top sits at y = 170 of the 480-line screen,
above the crosshair, so both layouts place it 157.5 px above the centre in the
1080-line frame. That matches stock wherever the HUD factor equals the height
over 1080 (720 to 2700 lines at `cg_hudScale 1`); outside it, the name keeps
the HUD's clamped frame, like the other HUD text.

2D layouts are authored in pixels of a 1080-line screen and scale with the
window height ([ui_scale.rs](../crates/sjk-viewer/src/ui_scale.rs)), as retail's
640×480 virtual screen did: 1440 lines draw them at 1.33× and 2160 lines at 2×.
The scale is clamped to 0.6–2.5 (648–2700 lines); the console keeps a 0.75
floor, the HUD 2/3 and the frame-rate and weapon labels 0.85. `cg_hudScale` and
`con_scale` multiply it. The operating system's display
scale (Windows scaling) is not applied: a fullscreen window already covers
the display, so it would count the density twice, and in a small window on a
scaled desktop it would push 1080-line layouts past the window edges. It only
sets the resolution the bundled Inter font is rasterized at, and text sized in
that font's own units is converted from line heights so it does not depend on it.

### Classic model preview

The classic profile's live model ([menu_stage/preview.rs](../crates/sjk-viewer/src/menu_stage/preview.rs))
is the menu stage's actor drawn into a target of its own: a colour texture in
the scene's format and a `Depth32Float` depth at the preview's size on screen
(sides rounded up to 16 pixels, at most 1024), so the world material
pipelines draw into it unchanged, with a camera of its own that frames the
whole body from in front and turns round it. `preview.wgsl` then writes the
8-bit texture the UI draws as `PREVIEW_TEXTURE`: display values as the
effect layer's `display` makes them for each scene format, at a neutral
exposure, with coverage from depth (an edge pixel averages its covered 3x3
neighbours and takes their share as alpha). On lightsaber creation the lit
blades of the actor's hilts go through the game's blade renderer into an
8-bit texture of their own (its own instance buffer, against the model's
depth), and the encode adds them: over the body they add to it, beyond it
their brightest channel becomes the alpha, so the glow shows over any page.
The passes run after the scene and before the UI, only while a classic page
shows a preview.

The actor is the stage's, marked preview-only so the world pass and the
saber blade list leave it out; it wears no sabers on character creation and
in the cosmetics window. Lightsaber creation shows its hilts alone
([menu_stage/showcase.rs](../crates/sjk-viewer/src/menu_stage/showcase.rs)):
the actor is not drawn (nor animated) and only lends its position and light
sample; each hilt lies along the camera's horizontal, its first blade to the
right, centred on its whole length (mesh and every used blade's root and tip,
measured along that blade once when the hilt loads), turned about its own
blade line, the second of two 14 units under the first, and the camera
stands back just far enough for the longest saber across the preview's
width. On the main menu it
stands on the backdrop's stage when the map has one; in a match, at the
local player's origin, so the map's light grid and lights shade it as the
player is shaded. It wears the stage's cosmetics. Not drawn into it: saber
trails, dynamic glow, shadows and fog.

A failure turns the preview off rather than the client: the pipeline and
textures are made inside wgpu validation and out-of-memory scopes, and the
first frame is a probe recorded in an encoder of its own and submitted
inside a scope (`FrameQueue::submit`, once). On any error the log says why
and the profile keeps the model's portrait for the rest of the session.

### UI colour model

The 2D layer (text, retained UI shapes, the shader HUD, the menu-file HUD and the
scope) works in display values, as retail's 2D drawing did: a colour is the
sRGB value shown on screen, and alpha mixes display values. `^1` is pure red,
the theme's accent `#FF6A3D` shows as `#FF6A3D`, and a black text shadow at 0.55 over
mid-grey shows 0.225 as in retail. The world, its resolve, bloom, HDR, the effect
layer and the in-world ground HUD stay in linear light.
[ui_target.rs](../crates/sjk-viewer/src/ui_target.rs) holds the model:

- Every 2D pipeline targets the display format without its sRGB encode
  (`Bgra8UnormSrgb` becomes `Bgra8Unorm`) and draws in its own pass after the
  scene resolve.
- With `r_gamma 1` that pass writes the swapchain image through a UNORM view.
  The surface is configured with that view format, and each frame makes one
  extra view object of the acquired image.
- With another `r_gamma`, or on an adapter without `SURFACE_VIEW_FORMATS`
  (Vulkan without `VK_KHR_swapchain_mutable_format`, GLES), the scene resolves
  into the display intermediate, the 2D layer draws through that texture's UNORM
  alias, and the display pass applies the ramp to world and UI together, as
  retail's hardware gamma did. Without aliasing this costs one full-screen pass
  at `r_gamma 1`.
- The float `r_hdr` target never receives 2D draws: HDR is encoded by the
  resolve before the 2D pass.
- Pictures sampled by the 2D layer (icon atlas, classic menu art,
  SJK's menu emblem, levelshots, menu-file HUD art, scope art) are `Rgba8Unorm`,
  so their texels are not decoded.
- SJK's menu emblem adds its two glow layers as light (`src * alpha + dst`) in
  display values, through a second, additively blended pipeline of the shape
  renderer; everything else in the 2D layer is alpha blended. Font atlases contribute only alpha, which no format
  decodes, so the Inter atlas stays shared with the ground HUD.

Colours chosen by eye for the earlier linear model were re-authored so neutral
text keeps its on-screen lightness: each grey or white text colour is now the
value it used to show (the theme's foreground 0.94, 0.97, 1.0 became 0.973,
0.987, 1.0 and its muted 0.60, 0.68, 0.76 became 0.798, 0.843, 0.886), and a
translucent one also gained opacity to keep its lightness over a dark backing:
dimmed labels are now 0.77 to 0.96 opaque, disabled ones 0.58 to 0.63. Chromatic colours (accents, team and status colours, `^` codes,
retail menu values) keep their authored values and so show at full saturation.
Scrims and other translucent fills keep theirs: dark ones look darker, and faint
white washes (separators, borders, hover fills) look fainter than before.

### Menu text

Every menu screen and page draws its text on the SJK UI's cards, bands and panels
([sjk-ui.md](sjk-ui.md)) or the classic+ panels, so no screen writes straight over
the live map any more. The old form screens' readability scrim and its setting
(`ui_menuContrast`, `menu_widgets/contrast.rs` and `hero.rs`) were removed with
them on 10/10/2026, and a saved `ui_menuContrast` is dropped from the profile.

UI text uses the bundled Inter font, rasterized once per display scale in
[text.rs](../crates/sjk-viewer/src/text.rs). Two options switch surfaces to
the game's own fonts, drawn with bundled vector replacements of the retail bitmaps
([sjk.md](sjk.md#fonts)): `cg_classicHudFont` draws the status HUD with SJK HUD
(retail `arialnb`), and `ui_gameFont` ("Classic game fonts", on by default) draws
every surface the retail game drew with its own fonts in that
font, following OpenJK `codemp`:

| Retail font | Drawn with | Surfaces |
| --- | --- | --- |
| `ergoec` (`FONT_MEDIUM`) | SJK Menu | Menus, crosshair name, followed player's name, warmup text, match timer, enemy info, scoreboard names and headings |
| `ocr_a` (`FONT_SMALL`) | SJK Chat | Chat box and typing line, weapon/Force/inventory selection names, scoreboard numbers |
| Console character set (`gfx/2d/charsgrid_med`) | JetBrains Mono | Console and notify lines, FPS, snapshot, vote, connection interrupted |

Centre prints are SJK's headline text: they draw in Fira Sans Condensed
(`text::HEADLINE`, `RetailFont::Headline` in [game_font.rs](../crates/sjk-viewer/src/game_font.rs))
with `ui_gameFont` on or off, as do the words in the world over mystery boxes and the box
hunt's mark ([mystery-boxes.md](mystery-boxes.md)). It loads with the SJK UI's families;
until it does (or if it fails), they draw in Inter. Centre prints keep `ergoec`'s line
(`CENTER_FONT_1080`) grown by `HEADLINE_GROWTH` (1.07), so their capitals stay as tall as
SJK Menu's (a test measures both fonts).

The routing is per text run: the HUD maps its text ids in
[text_values.rs](../crates/sjk-viewer/src/hud/text_values.rs), chat marks its
centre-print rows, and the scoreboard sends text made only of digits, `-` and `/`
to the small font. Everything else, including the command browser and overhead
names, stays on Inter (or SJK HUD for the status HUD). SJK Menu, SJK Chat and SJK
HUD keep the retail `.fontdat` layout: [game_fonts.py](../scripts/game_fonts.py)
draws every glyph at 64 font units per retail pixel with the retail advance and
position (SJK Chat's OCR-A glyphs centred in the retail advance), and [retail_font.rs](../crates/sjk-viewer/src/text/retail_font.rs)
rasterizes each font once at 6 raster pixels per retail pixel into a mipmapped
coverage atlas, with the line height and baseline the `.fontdat` header gave
(`ergoec` 22 and 17, `ocr_a` 21 and 17, `arialnb` a 14-pixel line with the
baseline at its bottom). Slots hold the Windows-1252 character of their byte, and
a byte the font lacks draws `.`, as `RE_Font_DrawString` drew for a glyph with no
width. The console character set's surfaces draw with JetBrains
Mono, bundled and rasterized once at 96 pixels per em into a mipmapped coverage
atlas ([console_font.rs](../crates/sjk-viewer/src/text/console_font.rs)), because
SJK draws no bitmap fonts ([sjk.md](sjk.md#fonts)). It keeps the cell of
`SCR_DrawSmallChar` and `CG_DrawChar`: a line 16 units tall, every character
advancing 8 (the console is monospaced, the em sized so the font's advance fills
the cell), the ascent and descent centred, and a space drawing nothing. The console keeps its
own sizes (`con_scale`, row pitch), and its caret, selection and pointer hits
measure the same fixed advance it draws with. The game fonts load when a world is
installed with the option on, or on first use, from
[game_font.rs](../crates/sjk-viewer/src/game_font.rs); they need no game data,
and a font that fails to load leaves its surfaces on Inter.
`cg_chatFont` and `con_font` (Settings: Chat font, Console font; empty is SJK's own) name a
font family installed on the PC for the chat box and the console
([system_font.rs](../crates/sjk-viewer/src/text/system_font.rs)): the system font folders
(Windows `Fonts` and the per-user one, macOS system and user folders, and on Linux the
user's `fonts` folders and each `XDG_DATA_DIRS` folder's `fonts`, as on NixOS and Flatpak,
following symlinked folders and visiting each real folder once) are listed
once on a worker from each file's `name` table, and a chosen family's 256 code-page slots
are rasterized once on a worker into its own coverage atlas (chat proportional with the
family's bold face; the console in its 8 by 16 cell, each glyph centred and the em sized so
the widest ASCII character fits). A slot the font lacks comes from the default (Inter,
JetBrains Mono), characters beyond the code page and emoji from the default's wide sources,
and a missing or unreadable font keeps the default. A glyph whose bounds are not finite or
exceed 4 ems counts as missing, the line is held to 1 to 1.6 ems, and a font whose atlas
would pass 8192 texels is refused. `con_font` draws the console's own grid (classic and SJK UI
consoles); the HUD's console-font text and notify lines keep JetBrains Mono. A changed value
loads on one worker, the font on show staying until the new one is ready, and rebuilds only
that atlas. The console font is drawn after all other text, so the
console stays on top. The classic console (`con_style classic`, see
[client.md](client.md#classic-console)) goes further: its background, bar and
text are a layer of their own
([console_backdrop.rs](../crates/sjk-viewer/src/console_backdrop.rs)) drawn after
every other 2D element, so text under an opaque console is hidden. The SJK UI's
console ([client.md](client.md#sjk-ui-console)) draws on the same layer:
vertical and horizontal fades (quads with a colour per corner) under its solid
quads, and labels in the SJK UI's two families after the console's own text,
from one vertex buffer drawn in three runs, each with its atlas.

Before the bundled fonts, the retail atlases (256–512 texels on the long side)
were converted at load into signed distance fields by
[sdf.rs](../crates/sjk-viewer/src/text/sdf.rs) and drawn by the text shader's
`fragment_sdf`, so magnified text kept sharp edges. That path remains for any
atlas under 2048 texels, but every font SJK draws now rasterizes into a
4096-texel coverage atlas, so none uses it.

Every code-page atlas (Inter, the SJK UI's Rajdhani and Exo 2, SJK Menu, SJK Chat,
SJK HUD, JetBrains Mono) and a distance field made from one hold a single value per
texel, so they are one channel: a `GrayImage` on the CPU (the menu snapshots raster
from it too) and an `R8Unorm` texture with its mips on the GPU, which `text.wgsl`'s
`fragment_main` and `fragment_sdf` and the ground HUD read from `.r`; the colour
comes from the vertices. They were white RGBA8 textures read through alpha until
25/10/2026, four times the memory ([GPU memory](#gpu-memory)).

In the Inter atlas, byte 0xAC (`¬`) is an exception: the retail `ergoec` and
`ocr_a` fonts draw it as the boxed "WSI fonts" foundry logo, which players use in
names, so the atlas takes that glyph instead of Inter's not-sign
([logo_glyph.rs](../crates/sjk-viewer/src/text/logo_glyph.rs)). It comes from the
bundled SJK Menu, where it is a vector glyph traced like the rest of the font: it
is rasterized once at four times SJK Menu's raster size, scaled so SJK Menu's `H`
matches Inter's cap height, and spliced into both faces at atlas build and DPI
rebuild; nothing is read or rasterized per frame. SJK Menu and SJK Chat draw the
logo themselves, SJK HUD a not-sign as `arialnb` did, and the console font
JetBrains Mono's `¬`. Outgoing chat and names send `¬` as the single byte 0xAC
([player text](networking.md#player-text)), so other clients draw the logo too.

### Characters beyond the code page

Every font atlas holds the 256 characters of Windows-1252, one per byte. Any other
character, Cyrillic from a Russian player ([player text](networking.md#player-text))
or Japanese, Chinese, Korean or an emoji sent as UTF-8 by an SJK player, drew `?`
until 12/10/2026. Letters now come from one shared wide-glyph atlas
([wide.rs](../crates/sjk-viewer/src/text/wide.rs)), filled the first time each
character is shown, and emoji from a colour atlas beside it
([below](#emoji)):

- Each face names the fonts that lend it wide glyphs, in order: its own family when
  it has Cyrillic (Inter, Exo 2, JetBrains Mono), else the closest that does (Exo 2
  for Rajdhani and SJK Menu, JetBrains Mono for SJK Chat's OCR-A, Inter SemiBold for
  SJK HUD), then Noto Sans JP for kana and kanji (and most hanzi: 5,279 of Big5's
  5,401 common Traditional characters, 4,548 of GB 2312's 6,763 Simplified ones),
  Noto Sans SC for the other 2,215 GB 2312 hanzi, and Noto Sans KR for Hangul (all
  11,172 syllables and the compatibility jamo), then SJK Symbols for SJK's own
  symbols in the Private Use Area ([sjk-ui.md](sjk-ui.md#symbols)). A shared hanzi
  takes Noto Sans JP's Japanese form. A character none of them has draws the face's `?`; joiners,
  variation selectors and tags outside an emoji take no room.
- A wide glyph is scaled so its font's capitals match the face's `H` and sits on the
  face's baseline, so `Ж` is as tall as `H` beside it and a kanji takes the size Noto
  Sans JP gives it next to its own Latin capitals. The classic console shrinks a glyph
  wider than its cell (a kanji) into the cell, keeping one character per cell.
- The first lookup reads the glyph's advance from the font, so layout is right at
  once, and queues it. Once a frame, before the text vertices are uploaded,
  queued glyphs are outlined with `ab_glyph` (which reads outlines on demand;
  `fontdue` would outline all 16,000 of Noto Sans JP's glyphs when loading it) for at
  most 2 ms and packed into the atlas, so a new character shows from the next frame.
  A frame with no new character does no work beyond a hash lookup per wide glyph.
- The atlas is one 4096x2048 R8 texture with the code-page atlases' four mip levels,
  glyphs rasterized at 96 pixels per em on an 8-texel grid so every level's copy of
  a cell is whole texels, also kept on the CPU for menu snapshots. When it is full
  it starts over and the glyphs on screen come back over the next frames. It is made
  with its first glyph: until then the text bind groups carry a 1x1 stand-in, and they
  are all made again when it or the emoji atlas is made (`GpuState::update_wide_text`),
  so a world where no one writes beyond the code page never holds either.
- Every text bind group carries it at binding 2
  ([wide_texture.rs](../crates/sjk-viewer/src/text/wide_texture.rs)); a glyph's
  vertices hold `u` offset by 2, and `text.wgsl` samples the wide atlas for those
  (coverage, also under the distance-field entry), so every batch keeps its own
  atlas and draw order.

#### Emoji

Emoji are Twemoji's colour pictures (17.0.3, 4,009 of them, CC-BY 4.0), packed by
[twemoji_pack.py](../scripts/twemoji_pack.py) into `assets/emoji` (the 72-pixel PNGs
one after another, 4.2 MB, and an index) and drawn by
[emoji.rs](../crates/sjk-viewer/src/text/emoji.rs):

- An emoji is often several code points drawn as one picture: a skin tone after a
  hand, people joined by U+200D into a family, two regional indicators into a flag,
  a digit with U+FE0F and U+20E3 into a keycap. The layout walk (`glyph_at`) takes the
  longest sequence of the pack at each place as one glyph, U+FE0F ignored when
  matching (as Twemoji's own lookup does) and taken into the glyph it follows, so
  measuring, wrapping, the caret and Backspace treat it as one character. A character
  with a Windows-1252 byte (`©`, `™`, a digit) is an emoji only with U+FE0F or U+20E3
  after it, so plain text never turns into pictures.
- An emoji is a square one em of the face wide (the em taken from its capitals),
  sitting a tenth of itself below the baseline, with a twentieth either side.
- Its picture is decoded the first time it is shown, within the same 2 ms a frame as
  the wide glyphs, into a 2048x1024 RGBA atlas (made with its first picture, 253
  pictures, display values as every
  picture the 2D layer samples, mips averaged by alpha), bound at binding 3; its
  vertices hold `u` offset by 4. `text.wgsl` draws its own colours faded by the text's
  alpha, and black for the shadow quad (and for black text, `^0`).

### Game-data HUD

`cg_hudStyle game` (the default) replaces the engine-drawn health, armor, Force
and ammo widgets with the status HUD the game's own menu files describe, as retail Jedi
Academy draws it ([menu_hud.rs](../crates/sjk-viewer/src/menu_hud.rs)).
`cg_hudFiles` (retail default `ui/jahud.txt`) names a list of `loadMenu` files;
the stock list loads `ui/hud.menu`, so a PK3 that replaces that file (a custom
HUD pack) or a list naming other menus changes the HUD with no SJK-specific
format. A nonzero integer selects the stock text-only HUD; as in EternalJK, `0`
is the default list, `2` is the Jedi Knight II HUD (`menu_hud/frame/jk2.rs`,
EternalJK's pictures built into the client, drawn in place of the status menus,
without its scrolling static) and `3`/`4` name `ui/elegance_hud.txt`/`ui/jof_hud.txt`. A
missing list falls back to the default one, as `CG_LoadMenus` does; files
without a `lefthud` or `righthud` menu leave the engine-drawn HUD in place.
The text HUD's strings start where retail puts them, but SJK's vector font is
wider than retail's, so a right-hand string (the six-letter saber styles, the
ammo count) that would pass 4 units from the screen's right edge moves left to
end there, and the HUD's text never wraps at the screen's edge (hub reports
75/76).

When several PK3s replace `ui/hud.menu`, the one mounted last wins, and packs
usually replace the retail pictures (`gfx/hud/hudleft` ...) under the same names
too. `cg_hudPack` names the PK3 whose HUD to use instead: the HUD's menus and
pictures are then read from the files without the PK3s mounted after it that
ship `ui/hud.menu` (`VirtualFileSystem::without_mounts`), as if they were not
installed, so `assets1.pk3` gives the original HUD with a HUD pack still
installed. An empty value, or a PK3 that is not mounted, reads the files as
installed. The settings' HUD picker lists the choices
([choices.rs](../crates/sjk-viewer/src/menu_hud/choices.rs)) and shows each
game-data HUD composited on the CPU at 1280x720 for a sample player (health 74,
armor 46, Force 63, medium saber style) over a dimmed levelshot read from the
game's own `assets*.pk3`, with the HUD's text drawn by the menu
([preview.rs](../crates/sjk-viewer/src/menu_hud/preview.rs)); it renders on a
worker thread and is uploaded into a texture of its own. SJK's radial and classic
layouts get the backdrop alone, and the picker draws them over it each frame with
`HudOverlay::layout` for a screen the picture's size (`HudOverlay::preview_values`
settles the same sample player): the shapes are moved onto the picture, the text is
drawn in the menu's type, and the HUD's pictures (crosshair, icons) are left out.
`menu_snapshot::sjk_hud_picker_snapshot` writes the card with the retail HUD and both
layouts highlighted to `target/menu-snapshots/sjk-hud-picker*.png`.

The reader ([parse.rs](../crates/sjk-viewer/src/menu_hud/parse.rs)) keeps the
window fields HUDs use (`name`, `rect`, `visible`, `style`, `background`,
`forecolor`, `backcolor`) and skips other keywords with their arguments.
Drawing follows OpenJK codemp `CG_DrawHUD` and its helpers in `cg_draw.c`
([frame.rs](../crates/sjk-viewer/src/menu_hud/frame.rs)): the menus' visible
filled/shader backgrounds (`Menu_Paint`), `scanline` and `frame`, four tics per
meter with the partial one faded, the low-armor blink of the last armor tic,
three-cell numbers from the `gfx/2d/numbers/t_*` digits (`CG_DrawNumField`),
the saber style or ammo (grey while firing, yellow just after a pickup, red when
empty, `--` for weapons without ammo) and the score line. A HUD that provides
`saberstyle_desann`, `_tavion`, `_dual` or `_staff` gets that picture for the
style, as EternalJK draws them; otherwise the stock fast/medium/strong mapping
applies. The status HUD shows only for a living, non-spectating player without
the scoreboard, with `cg_draw2D`, `cg_drawHud` and `cg_drawStatus` on.

Items keep their 4:3 shape on wider screens: `righthud` items scale from the
right edge and every other menu from the left, as EternalJK's widescreen
correction (`cl_ratioFix`, `widthRatioCoef`) places them; at 4:3 this is
retail's plain 640x480 stretch. `cg_hudScale` grows the HUD from its bottom
corner. Pictures resolve through shader scripts and are packed once per load
into one atlas, each stored no larger than a 2160-line screen draws it (at most
512 texels); additive shaders (`blendFunc GL_ONE GL_ONE`) draw after the
alpha-blended ones. Score and `--` text use the HUD font (Inter, or `arialnb`
with `cg_classicHudFont`) rather than retail's `ergoec`. Not drawn: siege HUD
menus, the out-of-Force flash, and item text or owner-draw fields, which the retail
and the checked custom HUDs do not use.

The pilot's vehicle HUD, on a swoop, a walker or an animal alike, is stock's under
`cg_hudStyle game` when the loaded menus define `swoopvehiclehud` (the retail
`ui/hud.menu` does): its pictures (`vehicle_frame`, `vehicle_grid`, the hull, armour,
turbo and ammunition ticks) are drawn at the menu's rectangles, kept to the middle of a
wide screen and grown by `cg_hudScale` from the bottom like the rest of the game-data
HUD, with the turbo bar and the weapons-linked mark in `whiteShader` white
([menu_hud/frame.rs](../crates/sjk-viewer/src/menu_hud/frame.rs) `vehicle_hud`).
Stock's item names are kept: `shield_tic*` show the hull and `armor_tic*` the shields.
The text HUD of a nonzero `cg_hudFiles` (the HUD picker's Text only) shows the speed,
the hull and the ammunition as three numbers in a row at the bottom centre, in the text
HUD's font (`Frame::simple_hud`): the speed 80 units left of the yellow hull in
`ICON_BLUE` (light red while the turbo flashes), the ammunition 80 units right in
`HUD_ORANGE` (`first / second` for two weapons, nothing without a weapon), also when the
vehicle hides its rider and the player's numbers give way. SJK's own looks show the same
three through their layout documents' `vehicle_*` widgets (bindings `vehicle_ratio`,
`vehicle_speed_ratio`, `vehicle_ammo_ratio`, `vehicle_value`, `vehicle_speed_value`,
`vehicle_ammo_value`, `vehicle_text`, `vehicle_speed_text`, `vehicle_ammo_text`, shown by
`draw_vehicle`, the ammunition's by `draw_vehicle_ammo`, which needs a weapon): `radial` a
yellow hull arc under the crosshair, on the health arc's circle between the left and
right meters (132 to 48 degrees, four segments, the same track and shadow), and on the
armour arc's inner circle a cyan speed arc (132 to 96 degrees, filling towards the
middle) and an orange ammunition arc (48 to 84), with the speed, hull and ammunition
numbers in a row under the outer arc; the radial placement moves them all up and down
with the rest of the status but never spreads them apart. `classic` the centre panel
described below. A vehicle with a `turboRecharge`, whose turbo time the ride prediction
gives (a live game, not a demo), also shows its turbo (`vehicle::Turbo`): the recharge
fraction in red-orange while it recharges, full and green once ready, full and bright
while it burns. As a widget's colour is fixed, each look has one turbo meter per phase,
shown by `draw_vehicle_turbo_charging`, `draw_vehicle_turbo_ready` and
`draw_vehicle_turbo_burning` (binding `vehicle_turbo_ratio`), and its word by
`draw_vehicle_turbo` (`vehicle_turbo_value`: the charge in percent, `READY` or `BOOST`,
colour coded; `vehicle_turbo_text` the `TURBO` line): `radial` a thin arc just inside the
speed arc with the word under the speed number, `classic` its row of the centre panel. The text HUD puts the same word 160 units left of the centre, in red,
green or white. Without a turbo nothing of it shows and nothing moves. The speed's meter is `speed` over `speedMax` (the turbo passes the
maximum and reads full), the ammunition's one weapon's `ammo` over its `ammoMax` or two
weapons' together. With a game-data HUD that has no vehicle menu, SJK draws its own
([hud/vehicle.rs](../crates/sjk-viewer/src/hud/vehicle.rs)): a vector strip at the
bottom centre laid out on `swoopvehiclehud`'s 640x80 units (hull above, speed, shield
and ammunition below), scaled by the window height and `cg_hudScale`, with its numbers
in the HUD font. The stock HUD and the strip follow `CG_DrawVehicleHud`
(`cg_draw.c`): hull `stats[STAT_HEALTH]`
of `armor` in 12 tics, shield `stats[STAT_ARMOR]` of `shields` in five, `speed` of
`speedMax` in five (red and flashing every 200 ms while the turbo burns), `ammo[0]` and
`ammo[1]` of `weap1AmmoMax` and `weap2AmmoMax` (one row, or two of four tics for two
weapons, as stock loops), the turbo recharge bar (green once ready) and the
weapons-linked mark. A tic is full while the value covers it, faded by the covered part
for one tic and absent beyond. The maxima come from the vehicle's `.veh` definition,
read with the vehicle's model; the values from the snapshot's vehicle player state
(only the pilot receives it), with the predicted ride's speed and turbo time where
prediction runs. A vehicle with `hideRider` takes the player's status, weapon and
game-data HUD away, as stock does. In a vehicle the crosshair is doubled (when
`cg_dynamicCrosshair` is below 2, and the size is scaled) and is the `.veh`'s
`crosshairShader` picture when it names one. Not drawn: the damage icons of
`vehicledamagehud` and `enemyvehicledamagehud`, the no-ammo warning flash and the
weapons-linked sound.

`cg_hudStyle classic` selects SJK's classic layout in either font: three groups
along the bottom with no panel behind them (`classic.json`), level with each other; the
text keeps its drop shadow and the bars a dark track, so they read on light and dark
ground alike. Bottom left
(`vitals_*`, the HUD edit mode's Vitals) the `HEALTH` and `ARMOR` rows, a label, a bar
and the number; bottom right (`forcebox_*`, Force) the Force row mirrored. In the middle
(`weaponbox_*`, Weapon) on foot the held weapon's name (`weapon_name`, which does not
fade) over an `AMMO` row with the count, or the saber's style, shown by
`draw_weapon_panel` (a weapon that counts ammunition or a saber, no vehicle, and not
while the weapon selection row, drawn in the same place, shows); unarmed, nothing. While
piloting, a taller centre group (`vehicle_*`, Vehicle) in two columns: `HP`, `SPD` and
`TURBO` rows (the turbo's bar in its phase's colour; without a turbo the row stays,
greyed with `--`), and `AMMO 1` and `AMMO 2` with each weapon's number over a segmented
bar of ten cells (`vehicle_ammo1_*`, `vehicle_ammo2_*`; a slot without a weapon greyed
with `--` and an empty track). All three sit on the bottom edge of the screen, their lowest line 24 units above it. Fixed words
come from `label_*` bindings (`data_source::LABELS`), segmented bars from a meter's
`segments` style (`widgets::segment_cells`). Under `game`,
SJK's default layout ([default.json](../crates/sjk-viewer/assets/hud/default.json),
or a `hud.json` beside the configuration) draws the crosshair, team rows, votes,
timer and lagometer, and the status too when the game HUD's files give
none (the [kill feed](client.md#kill-feed) is drawn by the HUD itself in every style); with `cg_classicHudFont` on, the classic layout does instead. A saved
`modern`, the retired layout of that name, is reset to `game` at start.

`cg_hudStyle radial` (picker name "SJK radial") is SJK's own take on the TheRisqe Radial
HUD, drawn by the engine with no PK3: health (red, outer) and armor (green, inner) as
arcs left of the screen centre, Force (blue, outer) and ammunition (amber, inner) right of
it, each cut into four segments that fill in turn, the whole meter on one rounded dark
shadow band that follows its curve (the widget's `border` and `border_width`, a margin round
the bars). Widgets paint in document order, so the pills come first in the layout, then each
meter's shadow and bars, then the text. A shadow is drawn with a knockout (`ArcStyle::knockout`,
the pill's extents, the `knockout` field of `DrawCommand::Arc`): it leaves out the stripe the pill
already darkens, so pill and shadows are one shape instead of two translucent layers stacking
into a darker one: the shader adds only what the pill leaves of the stroke's coverage, so the
two show one uniform alpha, with no seam on the pill's anti-aliased edge (`knockout_coverage`
and `knockout_remainder` in [arc.rs](../crates/sjk-ui/src/arc.rs) are the reference). The look
is laid out for the bundled font (Inter) and uses it whatever `cg_classicHudFont` says: a text
widget's `align` puts each number toward its bars, 5 px from their shadow whatever its length,
and the numbers are lowered 2 px to centre Inter's digits on the pill. The rings are centred 0.69 of the screen
height down, below the crosshair, where TheRisqe's bars sit (their picture's middle is 92.6 of
480 lines under the centre). Each side has a pill running through its bars at their middle
height, with the numbers on it: health outside the left bars and armor inside them,
ammunition inside the right bars and Force outside. With a saber the ammunition's bars
become one full line in the saber style's colour (the `style_ratio` binding; Fast blue,
Medium yellow, Strong red, Dual green, Staff magenta, as TheRisqe's style pictures are,
plus Desann violet and Tavion cyan) and
the style's name replaces the number, in the same colour. The weapon-name transient rests in
the hollow between the bars, above the pills.
It is the layout document [radial.json](../crates/sjk-viewer/assets/hud/radial.json): the
default layout's other widgets (crosshair, team rows, votes, timer, lagometer)
plus `arc` widgets, which `hud.json` overrides cannot yet replace for this style.
Widgets are placed in 1080-line pixels that grow with the window height and `cg_hudScale`;
the ring group's drop is a widget's `offset_fraction` instead, a fraction of the screen's
height (`y`) or width (`x`) added to its pixel `offset`, so it stays at 0.69 of the height
at every resolution, aspect ratio and HUD scale, including past the UI scale's limits.
The arcs are the new `sjk-ui` draw command `DrawCommand::Arc`: one quad per stroke whose
fragment shader (`ui_shapes.wgsl`, mode 3) takes the signed distance to a round-capped arc,
so they stay smooth at any resolution and scale with the HUD scale. Segment geometry and the
distance function are in [arc.rs](../crates/sjk-ui/src/arc.rs) (unit-tested; the shader
evaluates the same expression); the ammunition ratio is the weapon's pool over
`ammoData[].max`, doubled with the Double Ammo rune ([radial.rs](../crates/sjk-viewer/src/hud/radial.rs)).
Health pulses red at 25 or less, as the default and classic layouts do. Health and armour run to twice
the maximum (`hud/update.rs`); over it, the same segments are stroked again from the start,
0.55 of the stroke wide, in a deeper shade of the fill (`nameplate_math::saturated`); the
default and classic layouts' meters draw the same as an inner band (`overflow_band`). The shader's
own bars (`hud.wgsl`) are clamped to one maximum. `menu_snapshot` renders the sample
states (`radial_hud_snapshot` only these) to `target/menu-snapshots/hud-radial-*.png` with
a CPU copy of the shader; `hud-radial-overheal` is 125 health over 199 armour.

In every HUD style, a weapon change shows retail's weapon selection row for
1.4 s (`WEAPON_SELECT_TIME`), as `CG_DrawWeaponSelect` draws it and JoF EternalJK
keeps it square on wide screens ([weapon_select.rs](../crates/sjk-viewer/src/weapon_select.rs)):
the selected weapon's icon (80 units) between 40-unit icons 12 units apart, the
centre at 320 and the icons at `y` 400 and 420 of 480, with Concussion between
Flechette and Rocket, empty thermal detonators and trip mines left out, and an
empty weapon drawn with its `_na` icon (the saber with its staff or dual icon).
EternalJK's icons per side apply: 3 up to 16:10, 5 up to 16:9 and 7 wider, one
more with the text HUD. The name is retail's (`SP_INGAME_<item>` from
`strings/english/sp_ingame.str`, such as "E11-Blaster Rifle") in gold
(0.875, 0.718, 0.121), `FONT_SMALL` at scale 1 with its baseline 6 units above
the bottom (`ocr_a` with the game fonts on, otherwise the bundled font at that
size). Units are those of the game HUD, `cg_hudScale` included, growing from the
bottom centre. The row needs `cg_draw2D`, a living player who is not spectating,
following or on an emplaced gun, and no held scoreboard; a Force or inventory
cycle after it hides it, since `CG_Draw2D` shows only the most recent selector.

SJK's own layouts (default, classic and radial) draw their weapon name as a transient
that holds 0.8 s and fades over 0.6 s, as long as the row; while the row shows they
hide it, since the row names the weapon, so it only appears when the row cannot (for
example while following a player). The ammunition count, the ammunition arc and
the pill of the radial layout stay visible during the row (the classic layout's weapon
panel gives way to it), as in EternalJK, where nothing hides the ammunition while
`CG_DrawWeaponSelect` draws. The small held-weapon icon beside the ammo count shares
the name's timing and is hidden while the row shows; there is no ammo picture at the
bottom centre. The Force power timer icons keep showing during the row. Unlike
retail, the row's name has SJK's text shadow.

## Billboard icons

Frame billboard icons follow OpenJK's `RT_SPRITE` image orientation: texture v=0
belongs at the top of the quad. Their local v is reflected before the shader's
scale/scroll transform; ordinary FX billboards retain their existing convention.
This covers simple-item icons and player-status icons when submitted through
that path. The owner reported an inverted talk balloon during the player-icon
PR playtest. An external probe compared the corrected production transform with
OpenJK `RB_AddQuadStampExt`: four corners with three scale/scroll transforms
matched, while ordinary FX transforms were unchanged. Native visual confirmation
of the correction remains pending.

World icons (talk/connection and simple-item sprites) preserve their authored
texture alpha even when `r_softParticles` is enabled. They use a distinct
instance kind to bypass the 16-unit intersection fade, while retaining ordinary
depth testing, orientation and bounded effect-pass coverage. Smoke and other FX
quads still soften against nearby surfaces. Applying the smoke fade to icons
made opaque bubble interiors transparent near geometry.

## Generating material maps

`sjk-materialgen` writes normal, height and roughness/metalness/occlusion maps
for the world textures of the player's own installation, in the rend2 naming
that the optional material maps (`r_normalMapping`, `r_specularMapping`,
`r_parallaxMapping`) look up next to a diffuse image, and SJK's emission maps
(`_e`, `r_emissiveMaps`). It runs offline and only reads the game data; [its crate documentation](../crates/sjk-materialgen/src/lib.rs)
and `--help` are the reference.

```sh
cargo run --release -p sjk-materialgen -- --maps mp/ffa3,mp/duel1
```

- **Input.** GameData is found like the client finds it (`--game-data`,
  `JKA_GAME_DATA`, the config's `fs_gameData`, then the usual Steam paths).
  `base`, an optional `--fs-game` directory and `SJK_CONTENT` are mounted in
  the client's order, case-insensitive. The tool's own earlier output and every
  other generated pack (one with a `sjk-materialgen` or `jkr-materialgen`
  manifest, whatever its name) are left out, so an old pack's maps never count as
  a texture's existing ones. The installed maps (or `--maps`) supply the shaders actually drawn: BSP
  shader lumps and surfaces plus the shader scripts.
- **Selection** ([select.rs](../crates/sjk-materialgen/src/select.rs)). A texture
  qualifies when a shader draws it on lightmapped surfaces with lightmap and
  diffuse stages that collapse into one opaque pass. These are the stages the
  renderer gives maps to. Skipped, each with a reason in the manifest: sky,
  fog, liquids, nodraw/clip/system shaders, interface and 2D images,
  lightmaps, blend-only effects, `deformVertexes`, glowing, animated,
  environment-mapped and non-plain colour diffuse stages, alpha-tested foliage
  (grates are allowed), images without relief (flat colours) and textures that
  already have rend2 maps. With an existing normal map or specular map only the
  missing kind is written. A shader whose diffuse pair is followed by its own
  `tcGen environment` stage (stock chrome and polished floors) qualifies; the
  environment stage marks the texture polished (below).
  Korriban's glossy walls (`korriban/door`, `entrance_top`, `os_outsidebased`,
  `os_basic_pillarb`) draw an environment stage first, their paint over it with alpha
  blending (the alpha masks the shine) and the lightmap last. The client draws that
  glossy base as two lightmap collapses ([Material maps](#material-maps)), so its paint
  qualifies (`select::glossy_base_paint`) and overrides reach it (since 13/10/2026). Its
  shine is masked to the inlays and drawn by its own pass, so it does not mark the
  texture polished.
- **Generation** ([generate.rs](../crates/sjk-materialgen/src/generate.rs)),
  deterministic and wrap-around, so tiling textures stay seamless. Height comes
  from luminance, high-passed twice at 1/8 of the texture to suppress baked
  lighting gradients. It is then weighted by scale band (the class's fine-detail
  weight on the two finest bands: metal keeps a quarter, painted panels 0.4, so
  the grain of the HD texture packs does not ripple their reflections) and
  normalised. Bright is high unless the paint says otherwise: retail textures are
  painted lit from above, so where the light rims of the painting fall on the
  edges of the dark regions rather than the bright ones (dark studs on a lighter
  plate, inset panels lighter than their frame), the height is turned upside down
  (`painted_relief`, below -0.1; never for alpha-tested textures or ones that are
  more than 40% near-black, whose dark parts are holes). On the retail MP maps
  about one texture in twenty is turned; most HD textures show too little painted
  light to tell, and the overrides' `relief=` decides those.
  Normals are Scharr slopes (red +s, green +t down the image, rend2's frame),
  scaled by the class strength. Above 512 texels they are taken per 1/512 of
  the texture, so high-resolution replacements do not turn texel noise into
  steep bumps. The packed map
  holds roughness from the class plus local variation, brightness and cavities;
  metalness only on bright, unsaturated texels; and cavity occlusion. Source
  resolution is kept unless `--max-size` caps it, and alpha-tested textures
  keep their alpha in the normal map.
- **Classes** ([classes.rs](../crates/sjk-materialgen/src/classes.rs)): one table
  of strength, parallax, roughness, metalness and occlusion per class. A class is
  chosen by the BSP material id (`q3map_material`), then path keywords (`concrete`
  and `cement` first, so a vent in a concrete wall stays concrete; a keyword inside
  a word that only looks like it does not count: `background` is not ground,
  `rocket` not rock, `rustic` not rust), then `surfaceparm metalsteps`, then the
  texture set: the Imperial, Rebel and
  industrial sets (`imperial`, `byss`, `kejim`, `vjun`, `hoth`, `bespin` and
  others) are `panel`, painted metal (metalness 0.35, roughness 0.5), and
  `korriban`, `yavin`, `rift` and `rocky_ruins` stone. Names add metal words
  (beam, brace, casing, hull, tank, barrel, hangar, elevator), electronics
  (control, switch, onoff, terminal) and panel words (panel, trim, door, plate,
  lift, gate, walkway, crate: Star Wars crates are metal containers, not wood). Stone, tiles and ground get `<texture>_nh` (height in
  alpha for parallax); the rest get `<texture>_n`. Metal gets no height: a
  height guessed from paint made metal panels swim. A texture whose shader has a
  `tcGen environment` stage is
  polished: its roughness is at most 0.25 with half the variation.
- **Metal.** Metalness 0.8 and roughness 0.3 (±0.3 across the texture), for the
  reflection probes ([Reflection probes](#reflection-probes)). In rend2's packed
  path metal loses its diffuse share and takes its reflection from the probe;
  0.8 rather than 1 keeps a fifth of the diffuse light where the heuristics call
  painted or grimy parts metal, and metalness still falls on dark and saturated
  texels. Without probes (`r_cubeMapping 0`) such metal reads darker than its
  retail look. Earlier packs used 0.3 and 0.45, when nothing reflected the room
  into metal.
- **Emission** ([emission.rs](../crates/sjk-materialgen/src/emission.rs)). A texture
  gets `<texture>_e` when there is evidence that it gives light, strongest first: the
  overrides file; `q3map_surfacelight` on a shader drawing it (`q3map_lightRGB` only
  colours such a light and `q3map_lightImage` only averages an image for its colour,
  so neither counts alone); an authored glow image next to it (`<texture>_glow`,
  `<texture>glow`, `<texture>_glw`, the black-backed overlays retail shaders add with
  `blendFunc add` and `glow`); a fixture or screen word in its file name (`light`,
  `lamp`, `bulb`, `neon`, `screen`, `monitor`, `display`, `console`, `computer`,
  `comp_`, `holo`, `glow`; not `lightning`, `highlight`, `flight`, `lightgr…`,
  `lightsab…`, `clamp`, `switch`, nor names with an `off`, `broken`, `dead`, `unlit`
  or `dark` part); the BSP material `computer`; or a control's name (`switch`,
  `control`, `onoff`, `keypad`, `keyport`, `terminal`, `button`, `comm_`, `locked`),
  whose painted indicator lights emit: only saturated, clearly bright texels, and no
  map when more than 15% of the texture would emit (that is paint, not lights). Textures whose every shader already
  shows light (a `glow` stage, or an additive or `GL_DST_COLOR GL_ONE` one on a shader
  without `q3map_surfacelight`; see [Emission maps](#emission-maps)) and textures with an
  `_e` image get none. With a glow image, the emission is that image. Otherwise the
  texels that emit are near-white or saturated ones clearly brighter than most of the
  texture (above its median value plus 0.2, at least 0.6; near-white needs 0.75);
  surface lights and overrides take any texel above the median plus 0.1 (at least
  0.45); a texture whose median is 0.8 or brighter is a light panel whose bright texels
  all emit. The emitted colour is the texel's, lifted so the brightest emitting part
  reaches about full brightness (at most 3x). Nothing luminous, no map; the manifest
  records each decision (`emission`: evidence, `written` or the reason, coverage, gain).
- **Overrides** ([overrides.rs](../crates/sjk-materialgen/src/overrides.rs)). A text
  file of per-texture rules fixes what the heuristics get wrong. Each line is a
  path pattern (the diffuse image without extension, case-insensitive, `*` and `?`
  wildcards) followed by `class=`, `roughness=`, `metalness=` (0–1),
  `strength=` (0.1–4, multiplies the class's normal strength: more for a texture
  whose shapes read flat, less for overdone relief), `height=on|off`, `relief=inverted|normal|auto` (dark parts high, bright parts
  high, or the painted light decides) or `emission=on|off|<strength>` (0–4, the
  emitted colour's multiplier; 0 is off); `#` starts a comment. Every matching line applies, in order
  (for `emission` the last that sets it):

  ```text
  textures/mp/floor*        class=tiles roughness=0.2
  textures/kor_*/*metal*    metalness=0.9 height=on
  textures/x/lightwall      emission=off
  textures/x/console2       emission=1.5
  textures/desert/s_floor1  relief=inverted
  textures/taspir/trim_walk strength=2.5
  ```

  The file is `sjk-materialgen-overrides.txt`
  ([overrides_file.rs](../crates/sjk-materialgen/src/overrides_file.rs)). The tool
  reads `--overrides FILE` (none with `--no-overrides`), else the file next to the
  output pk3, else the one in the client's `generated` folder (`GameData/SJK/generated`
  or the per-user one), else one left in an earlier version's folder
  (`%APPDATA%\jkr\generated`), which a run that writes a pack copies to the client's
  folder, keeping the old file. Every run prints which file it read and how many
  rules it has, or `NONE FOUND` with every place it looked (and a warning); the
  manifest records the same (`overrides`, `overrides_found`, `overrides_rules`) and,
  per texture, the lines that applied, its class, roughness and metalness, and whether
  it was polished. Until 13/10/2026 only `--overrides` and the output's folder were
  read: when the default output moved from `%APPDATA%\jkr\generated` to
  `GameData/SJK/generated` (12/10/2026), Sol's rules stayed behind and the 12/10 pack
  was built without them, silently.
- **Specular layout.** The tool writes `<texture>_rmo` (red roughness, green
  metalness, blue occlusion) rather than `_specGloss`. The heuristics produce
  roughness and metalness directly, and the packed path takes the metal colour
  from the albedo and a 0.04 dielectric reflectance by itself, without rend2's
  SDR gloss conversion.
- **Output.** One pk3 of PNGs plus `sjk-materialgen/manifest.json` (every
  source, its outputs, class, maps and shaders, skipped shaders with reasons,
  all settings). The archive is deterministic. The default path is
  `generated/zzz_sjk_materials.pk3` in the client's folder: `GameData/SJK` when the
  client keeps its `config.cfg` there, else the per-user SJK folder
  (`%APPDATA%\SJK` on Windows). The tool refuses to write anywhere else in the game
  installation. Output written elsewhere needs `SJK_CONTENT` set to its directory.
- **Loading** ([generated_packs.rs](../crates/sjk-viewer/src/generated_packs.rs)).
  The client mounts every pk3 in both `generated` folders last, above every game
  directory, `SJK_CONTENT` and a server's listed or downloaded packs (it mounts them
  again after those), so no other pack replaces a generated map. When a `generated`
  folder holds a pack, an older generated pack left in a game directory (a
  `zzz_jkr_materials.pk3` or `zzz_sjk_materials.pk3` copied into `base` by hand,
  known by its manifest) is left out, and the log names it, so its stale maps do not
  fill in for textures the new pack decided against. `--dry-run` lists the choices (with polished textures
  and applied override lines), and `--limit` takes only the most-used textures.

**Regenerating.** The manifest records the generation of the tuning
(`"generation": 7` since crates are painted metal, concrete is concrete, the overrides
are found in the client's folder and `strength=` scales relief; 6 declared light
fixtures without a glowing overlay emit, 5
maps for vertex-lit paint and indicator lights, 4 the relief orientation and smoother
metal, 3 emission maps; packs without it are generation 1). With
material maps or emission maps on, the client logs `material maps: the generated pack
is generation 1 of sjk-materialgen, this client expects 7 ...` once when the mounted
pack is older. Emission decisions depend on every map read: a texture drawn plainly on
one map and through a glowing shader on another gets its `_e`, and the client ignores
it where the shader glows.
Run the generator again; without `--out` it replaces the pack in the client's
`generated` folder:

```sh
cargo run --release -p sjk-materialgen
```

The tool leaves every generated pack out of its input, so an old pack's maps do not
count as existing rend2 maps. Restart the client after regenerating. On 12/10/2026
the pack in Sol's `GameData/base` (`zzz_jkr_materials.pk3`) was still generation 5,
which is why the light fixtures noted on 07/10 still did not glow there, and its
name had kept it in the input of later runs, so 61 textures of `mp/ffa1` counted as
already having maps.

The generated images are derived from retail textures. They stay on the
player's machine and must not be shared, uploaded or committed; the tests use
synthetic images only.

Heuristics fail where luminance is not relief. Painted stripes, signs and
decals become bumps. Lighting baked at panel scale (bright bevel edges, dark
undersides) becomes a ridge and a groove instead of a raised panel. Dark
grime reads as a dent, and smooth gradients within an eighth of the texture
remain. Unit tests cover flat input, slope direction, seams, gradient
suppression, alpha, class lookup, skip rules on synthetic scripts and the pk3
layout. A read-only run on `mp/ffa3` and `mp/duel1` of a Windows installation
with high-resolution texture packs took 11 s for 110 textures (160 MiB) and is
described in the generator's pull request. No in-game image has been checked.

JA+ grapple ropes ([grapple_rope.rs](../crates/sjk-viewer/src/grapple_rope.rs))
follow EternalJK's `CG_Missile`: on a JA+ server each `WP_STUN_BATON` missile is a
hook, drawn as `CG_TestLine` draws it, a one-unit-wide near-black (RGB 6, 0, 0)
`white` line from the hooked player's right hand to the hook, rebuilt every frame
as a frame-billboard streak and hidden while the local player duels. The hooks are
gathered once per frame into a fixed table. The effect atlas builds a generated
white tile for `$whiteimage`/`*white` stages so the `white` shader keeps its alpha
blend instead of falling back to the additive spark. The JA+ client plugin's hook
model is not drawn, as in EternalJK.

A saber knocked out of the hand or dropped after a throw (`saberKnockDown` turns its entity
into an `ET_MISSILE` with `WP_SABER`) is drawn as `CG_Missile` draws it: its hilt, the
`CS_MODELS` model the server names (`DEFAULT_SABER_MODEL` without one), with no blade,
turned along its flight as other missiles are, and nothing while `EF_NODRAW`
([projectiles.rs](../crates/sjk-viewer/src/projectiles.rs)). Before 12/10/2026 a missile
saber drew nothing, so a dropped saber vanished until its owner pulled it back.

## Shader test maps

[shader_testmap.py](../scripts/shader_testmap.py) builds galleries of every
retail world shader (`textures/...` in `base/assets*.pk3`) so shader rendering
can be checked, and compared with EternalJK, pad by pad. It writes `.map`
files, label atlases and a shader, compiles the maps with q3map2 from
NetRadiant-custom (`-game ja`) and packs `sjk_shadertest.pk3`; `--install`
copies it into `GameData/base`. The module docstring lists the options.

```sh
python scripts/shader_testmap.py --install
```

- **Maps.** `sjk_shaders_a` to `_e` hold one section per shader file, in file
  name order, each opened by a yellow plaque; `sjk_shaders_s` puts each sky
  shader on the ceiling of its own room. Play them with `devmap <map>` and
  `noclip`; the wall behind the spawn point lists every section with the
  `setviewpos` that leads to it.
- **Pads.** A shader goes on every face of a 160-unit panel over a label with
  its ID (`A001`), name and kind. Water, lava, slime and surface-sprite shaders
  are floor tiles, fog shaders 160-unit fog cubes (at most 28 per map: q3map2
  and the renderer's sort key hold about 30 fogs), and `portal` shaders get a
  `misc_portal_surface` (a mirror). Tool shaders (`nodraw`, `origin`,
  `areaportal`, `hint`, `skip`) are left out. `index.txt` next to the pk3
  maps every ID to its shader and file.
- **Duplicates.** A name defined in several shader files keeps the last file
  in name order, as ioq3's `ScanAndLoadShaderFiles` does; its label says
  `DUP`.
- **Compile.** q3map2 reads only the shader files a `shaderlist.txt` names and
  the retail lists miss some, so the script writes a complete one for the
  compile (it is not packed).

## Default visual profile

New profiles use the owner-approved rendering setup: day/night enabled at a fixed
noon (`r_dayHour 12`), volumetrics quality 3, actor/world sun shadows at 2048 resolution and
16 filter taps, and lighting tier 0 (available baked indirect light under the
live sun). Shadow gap closure and screen-space contact shadows are off.
The scene uses HDR with exposure 1 and SJK's eye adaptation (-0.5 to +1 EV), FXAA,
SSAO at strength 4, trilinear mipmapping
and 16× anisotropy where supported. Bloom is on (`r_sceneBloom 1`); the optional
LDR tone curve is off. Sunbeam dust is on at full density (`r_dustMotes 1`) and
shows only inside the godrays of `r_volumetrics`.
Dynamic glow is on with rd-vulkan's blur (stock defaults it off).
Soft particles, per-pixel model diffuse lighting and full rendering resolution
remain enabled. Material maps (`r_normalMapping`, `r_specularMapping`,
`r_parallaxMapping` at a tenth of its depth, `r_parallaxStrength 0.1`) and reflection probes (`r_cubeMapping 1`, 128²) are on, but
take effect only where a pack such as the [generated one](#generating-material-maps)
supplies maps; without one nothing is drawn differently or created. Noon, bloom,
dust and material maps are SJK's defaults (Sol's own settings). Emission maps
(`r_emissiveMaps 1`) are on too and only act where a pack has `_e` images.
These are ordinary cvar defaults, not a config imported at launch. They are the
Experimental [graphics preset](client.md#graphics-quality) (High until 16/10/2026),
which sets the costly ones together, and the look the SJK renderer (alpha) is tuned
for. A new profile (no `config.cfg` at start) begins on the SJK (recommended)
preset instead (Vanilla from 12/10/2026 to 16/10/2026): SJK's image over the maps'
baked lighting, without the sun, shadows, reflections or material maps; the cvars'
defaults stay these, so an existing profile keeps its settings.

When sun and sky, light shafts and both sun shadows are off (the Vanilla style and
SJK (recommended) presets; before 16/10/2026 the Performance and Vanilla levels), a map loads without extracting its lamps (`Lamps: none` in the
log): no pass reads them there, the shadow runtime is never installed and the
world shows its lightmaps. Lamp extraction, its grid and refinement took about
4.5 s of `JoFTemple`'s load on 09/10/2026 (see [status](status.md#joftemple-frame-rate-and-ultra-low)).

Saved values take precedence, including explicitly disabled effects. The client
saves every archived setting, so a `config.cfg` written before a default changed
keeps the old value (for example `r_dayHour 11` or `r_sceneBloom 0`); a fresh
profile is needed to see the new defaults. Existing
profiles are not silently migrated (the one exception is the old `com_maxfps`
default, see [client](client.md)). Resolution/window mode, input and keyboard
layout, FPS caps, audio levels, HUD/crosshair preferences, player identity,
server history, credentials and filesystem locations retain their independent
defaults. Stale MSAA/light-scale entries are not part of this profile; the active
antialiasing path is FXAA.

External release/Vulkan validation on RX 9060 XT checks 36 graphics values with
an empty config, an explicit equivalent config and an existing override config.
Fresh FFA5 and FFA1 scenes rendered successfully. See [status](status.md) for the
image comparison result and validation limits. No personal config or assets are
included in the repository.

## Community override shader scripts

The shared shader catalogue reads installed `shaders/*.oshader` files as ordinary
Quake 3 shader syntax, following JoF EJK #138. Their definitions override `.shader`
definitions regardless of filename ordering. Among override files the first
normalized path wins, and the first definition in an override file wins. Normal
`.shader` ordering remains unchanged. Strict tools and fault-isolated client loading
share this policy; malformed override scripts still produce diagnostics.

A shader named twice in one script, `.shader` or `.oshader`, keeps its first
definition, as rd-vanilla's `FindShaderInShaderText` finds the first in the text
(`definition_order` in `sjk-shader/src/catalog.rs`). `maps/oot.bsp`'s
`shaders/oot.shader` names `textures/oot/wtr_0_yuka_02_4` twice; the second, an
unfinished copy without its lightmap stage, drew the sand floor under the shallow
water as a dark see-through plane that a saber did not light (Sol's hub reports,
27/10/2026).

Existing source-authority and downloaded-content restrictions remain in force:
retail-authority protection wins when enabled (`fs_protectShaders`, on by
default; changing it requires restart), and downloaded scripts may add new
shaders but cannot replace installed definitions. This is format support in
the shared renderer, independent of menu style; no new setting is required.
Rend2's separate `.omtr` companion selection is not implemented. Installed-map/GPU
appearance with community override packs remains unverified.
