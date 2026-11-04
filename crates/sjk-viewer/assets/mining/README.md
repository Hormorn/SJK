# Mining

The Mining toy (SJK's own, [client.md](../../../../docs/client.md#mining)) puts a pickaxe
in the player's hand, and boulders come up out of the ground round them, to break for
ore or rubble. Everything here is drawn by [mining_assets.py](../../../../scripts/mining_assets.py)
from tubes, lumps and flat colours (nothing taken from another game), so none is edited by
hand except `mining.shader`. The rock is drawn with the installed game's own Yavin rock
texture: the shader names it, and it comes from the player's base assets as a map's does,
so nothing of it is in this folder.

| File | Game path | What |
| --- | --- | --- |
| `toy_mining.png` | `gfx/sjk/toy_mining.png` | The toy's icon (quick wheel, key bindings, Collection), 128x128 |
| `mining.png` | `models/sjk/mining.png` | The colour atlas, 128x128, a flat colour a 16x16 cell |
| `mining_glint.png` | `models/sjk/mining_glint.png` | The glint's star, 64x64, drawn added |
| `mining_pick.md3` | `models/sjk/mining_pick.md3` | A pickaxe along +X, 34 units from its butt (the origin) to its head, its spike down (-Z) and its chisel up |
| `mining_boulder.md3` | `models/sjk/mining_boulder.md3` | A boulder with crystals, its origin in the middle of where it meets the ground, standing up along +X |
| `mining_glint.md3` | `models/sjk/mining_glint.md3` | Three crossed squares round the origin, the glint's star on each |
| `mining_rubble.md3` | `models/sjk/mining_rubble.md3` | A broken lump of rock round its origin: junk |
| `mining_ore_<id>.md3` | `models/sjk/mining_ore_<id>.md3` | Each of the nine ores (`<id>` its id in the mining log): a lump of rock with that ore's crystals |
| `mining.shader` | `shaders/sjk_mining.shader` | `models/sjk/mining`: lit flat colours; `models/sjk/mining_glow`: the same, full bright; `models/sjk/mining_rock`: the rock; `models/sjk/mining_glint`: the glint, added |

The client mounts them in memory below all game data ([mining.rs](../../src/mining.rs)), so a
PK3 that has the same paths replaces them. The toy's sounds are retail game paths.
