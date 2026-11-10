# Architecture

SJK implements the client and server as native Rust programs. The current game
is Jedi Academy multiplayer; engine services use owned worlds and explicit
interfaces rather than a process-wide legacy "current map". SJK implements game
rules itself rather than hosting the original game DLLs.

## Boundaries

1. Authoritative simulation and client presentation have different owners and
   lifetimes. A rendered world can remain resident during a network transition.
2. Protocol 26 is an adapter. Wire client/entity numbers, configstrings and
   fixed packet limits must not become universal engine identities or capacities.
3. JKA rules and format constraints belong in compatibility crates. The generic
   server owns worlds and entity handles; the dedicated bridge supplies gameplay
   and maps those handles to the legacy endpoint.
4. Content paths are normalized, case-insensitive virtual paths. Asset ownership
   is explicit; loading another map must not silently replace global asset state.
   A read ignores one leading `/` or `\`, as `FS_FOpenFileRead` does; mounted
   names, archive entries and writes still refuse it.
5. Platform code and GPU resources stay at application/integration boundaries.
   UI layout and audio mixing have independent engine interfaces.
6. Downloaded content is handled through bounded storage operations. Remote paths
   must not become unrestricted local filesystem paths.

These are maintenance constraints. Existing compatibility dependencies do not
justify spreading JKA-specific constants into unrelated engine services.

## Resident client worlds

The viewer separates its displayed world from a connection waiting for a map.
`session_transition::resident::State` parks that transport outside `live_session`,
so snapshot consumers cannot render new-map entities against the retained BSP.
During a live map change the old world is only a backdrop beneath a loading
notice: it has no local gameplay authority. Intermission remains owned by the
remote session, including its normal camera, scores, chat and ready controls.
The viewer does not depend on the dedicated-server crate or construct a native
game while loading maps. Early entry into a joined world before a server player
exists still uses the movement-only predictor.

The waiting transport sends neutral commands on a separate timer and continues
receiving lifecycle events and messages. A matching prepared map can be reused
on a same-map restart; attachment resets presentation and anchors prediction to
the remote snapshot. Wire codecs remain unchanged.

CPU preparation and GPU installation own immutable destination inputs. Superseding
transitions signal cancellation before discarding their channels: CPU preparation
checks after mounting, BSP parsing and scene construction, and before every shader
script read; GPU construction checks between build stages. This releases a cancelled
worker's VFS at its next boundary instead of letting it retain all mounted PK3 files
until an obsolete build completes. Only a completed world is adopted. A prepared
gamestate's content selection must match before attaching a session without rebuilding,
and a restart must receive a fresh snapshot before attachment. A graphics reload takes
the same path to build the world on show again on a new graphics context (same device
and surface, the start-only renderer settings read anew; see
[rendering.md](rendering.md#graphics-reload)). A join from the menu hands over
the destination world prepared while it connected
([portal.rs](../crates/sjk-viewer/src/portal.rs)) rather than constructing a
duplicate at entry. The menu world is dropped once a server world is installed
and built again from its map after disconnection or cancellation
([menu_world.rs](../crates/sjk-viewer/src/menu_world.rs),
[rendering.md](rendering.md#gpu-memory)).

PK3 checksum inventory uses the validated ZIP central directory, retaining archive
entry order, CRCs and zero-length filtering. It does not visit/decompress every
payload during connection; normal asset reads remain responsible for payload and
local-header validation. See [pk3_fingerprint.rs](../crates/sjk-vfs/src/pk3_fingerprint.rs).

## Source map

All 32 workspace crates are listed in [Cargo.toml](../Cargo.toml).

| Crate | Responsibility |
| --- | --- |
| [sjk-viewer](../crates/sjk-viewer/src/main.rs) | Client executable, GPU, window/input, menus and integration |
| [sjk-dedicated](../crates/sjk-dedicated/src/main.rs) | Server executable, operator console and game bridge |
| [sjk-server](../crates/sjk-server/src/lib.rs) | Generic authoritative world/entity ownership |
| [sjk-runtime](../crates/sjk-runtime/src/lib.rs) | Engine-native world and presentation state |
| [sjk-game-jka](../crates/sjk-game-jka/src/lib.rs) | JKA movement, combat and game behavior |
| [sjk-client](../crates/sjk-client/src/lib.rs) | Client sessions, prediction and snapshot presentation data |
| [sjk-network](../crates/sjk-network/src/lib.rs) | Transport, discovery and legacy client/server sessions |
| [sjk-protocol](../crates/sjk-protocol/src/lib.rs) | Wire codecs and compatibility data, without sockets |
| [sjk-vfs](../crates/sjk-vfs/src/lib.rs) | Loose-file and PK3 virtual filesystem |
| [sjk-bsp](../crates/sjk-bsp/src/lib.rs) | Owned RBSP map data and collision queries |
| [sjk-scene](../crates/sjk-scene/src/lib.rs) | Renderer-neutral scene construction |
| [sjk-model](../crates/sjk-model/src/lib.rs) | MD3 and Ghoul2 model/animation data |
| [sjk-shader](../crates/sjk-shader/src/lib.rs) | Legacy shader-script parsing and resolution |
| [sjk-entity](../crates/sjk-entity/src/lib.rs) | Map entity dictionaries |
| [sjk-effect](../crates/sjk-effect/src/lib.rs) | Raven effect definitions |
| [sjk-nav](../crates/sjk-nav/src/lib.rs) | Navigation graphs and queries with game-supplied world access |
| [sjk-icarus](../crates/sjk-icarus/src/lib.rs) | Script interpretation with host-provided game operations |
| [sjk-audio](../crates/sjk-audio/src/lib.rs) | Sound storage, spatialization and mixing, the 3D sound's head filters and convolution |
| [sjk-acoustics](../crates/sjk-acoustics/src/lib.rs) | The 3D sound's acoustics: the level's walls as a ray-traced mesh, what reaches the listener from each sound, the room's echo ([client.md](client.md#3d-sound)) |
| [sjk-ui](../crates/sjk-ui/src/lib.rs) | Retained widgets, layout, input and draw commands |
| [sjk-shell](../crates/sjk-shell/src/lib.rs) | Cvars, bindings and command processing |
| [sjk-mod](../crates/sjk-mod/src/lib.rs) | The interface client mods are written against ([mods.md](mods.md)) |
| [sjk-mod-japlus](../crates/sjk-mod-japlus/src/lib.rs), [sjk-mod-jof](../crates/sjk-mod-jof/src/lib.rs) | The JA+ and JoF tools mods |
| [sjk-server-mod](../crates/sjk-server-mod/src/lib.rs) | The hooks a server mod has in the dedicated server ([server.md](server.md#ja-mode)) |
| [sjk-server-mod-japlus](../crates/sjk-server-mod-japlus/src/lib.rs) | JA+ for the dedicated server (`g_mod japlus`) |
| [sjk-jof-content](../crates/sjk-jof-content/src/lib.rs) | JoF content: JoF's repository manifest, checked downloads and the JoF folder ([jof-content.md](jof-content.md)) |
| [sjk-game-stores](../crates/sjk-game-stores/src/lib.rs) | Jedi Academy installs that Steam (every library) and GOG know of, for the client's launch discovery and the SJK Launcher ([client.md](client.md#launch)) |
| [sjk-materialgen](../crates/sjk-materialgen/src/lib.rs) | Offline tool: local material maps from installed textures |
| [sjk-mediafoundation](../crates/sjk-mediafoundation/src/lib.rs) | Windows only: silent H.264 MP4 chat clips through Media Foundation, one of the two crates allowed `unsafe` |
| [sjk-gpu-memory](../crates/sjk-gpu-memory/src/lib.rs) | Windows only: the graphics card's video memory and budget from DXGI, for the small-card settings; the other crate allowed `unsafe` |

## Main flows

Client: network session → decoded snapshots → client/game compatibility →
owned presentation state → viewer rendering, UI and audio. Local prediction uses
shared movement rules; it does not replace server authority.

Server: UDP → legacy endpoint →
[game bridge](../crates/sjk-dedicated/src/bridge.rs) → game simulation and native
world storage → per-client legacy replication.

Assets: VFS → compatibility parsers → owned map/model/shader data → scene and
application resources. BSP geometry remains the collision input for JKA maps.

The menu moves with the application shell between resident worlds, but its asset
catalogues belong to one immutable VFS generation. Attaching a new world invalidates
the model-icon workers and the cosmetics and Force-template catalogues before retaining
the new VFS; otherwise those caches keep every PK3 of an old world open across map and
server changes. VFS generations and live PK3 sources are counted independently and
logged at world attachment, disconnect and reconnect, so a short attachment overlap is
distinguishable from a generation whose owner never releases it. Unix clients also ask
the host for the permitted soft open-file limit at startup (respecting macOS's
per-process ceiling); failure is logged and remains non-fatal. Windows is unchanged.
