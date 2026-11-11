# Mystery box

The mystery boxes' chest, lid, sounds and the Box hunt toy's icon (SJK's own,
[mystery-boxes.md](../../../../docs/mystery-boxes.md)). The sounds are made by
[mystery_box_assets.py](../../../../scripts/mystery_box_assets.py) from synthesized tones
(SJK's own tune), the rest by [mystery_box_model.py](../../../../scripts/mystery_box_model.py)
and [teddy_bear_model.py](../../../../scripts/teddy_bear_model.py) from noise and shapes;
nothing is taken from another game, and only `mystery_box.shader` is written by hand. The
box's opening, roll and closing of 30/10/2026 and its laugh are Sol's, in the optional
SJK extras pack, not here.

| File | Game path | What |
| --- | --- | --- |
| `roll.wav` | `sound/sjk/mystery_box/roll.wav` | The roll: a music box tune slowing like a wound-down spring, its last chord at 5.2 s as the roll stops on what came out, 22050 Hz mono |
| `take.wav` | `sound/sjk/mystery_box/take.wav` | Taking what came out: a quick rising sparkle |
| `mystery_box.md3` | `models/sjk/mystery_box.md3` | The chest, 60 long (y) x 26 deep (x, its front facing +X) x 24 tall, its origin in the middle of its bottom: feet, a plinth, raised iron bands, corner posts, a rim, a lock plate and end handles, open at the top (dark walls and a floor inside); and a second surface, the halo: a band of light round its top rising 52 units |
| `mystery_box_lid.md3` | `models/sjk/mystery_box_lid.md3` | The lid, arched, banded, its origin on the hinge (the chest's back top edge), reaching +X; the client pitches it open |
| `teddy_bear.md3` | `models/sjk/teddy_bear.md3` | The teddy bear a box stops on when it blows up, about 18 units tall, facing +Y (made by [teddy_bear_model.py](../../../../scripts/teddy_bear_model.py)) |
| `teddy_bear.jpg` | `models/sjk/teddy_bear.jpg` | 256x256: its fur, light muzzle and belly, black eyes and nose, tan pads and red bow |
| `mystery_box.jpg` | `models/sjk/mystery_box.jpg` | 1024x1024: a long side (planks, iron bands, two question marks), the lid's top, an end, the dark inside, the iron edge |
| `mystery_box_glow.jpg` | `models/sjk/mystery_box_glow.jpg` | The same layout, black but for the question marks' glow |
| `mystery_box_halo.jpg` | `models/sjk/mystery_box_halo.jpg` | 128x256: the halo's light, soft warm rays brightest at the foot, fading to black at the top |
| `mystery_box.shader` | `shaders/sjk_mystery_box.shader` | `models/sjk/mystery_box`: lit as a player model is, the glow added on top, pulsing; `models/sjk/mystery_box_halo`: added, two-sided, its rays drifting, breathing; `models/sjk/teddy_bear`, lit the same; `models/sjk/box_loot`, lit with a sliding glint, and `models/sjk/box_loot_glow`, added, pulsing |
| `toy_boxhunt.png` | `gfx/sjk/toy_boxhunt.png` | The Box hunt toy's icon, 128x128: the chest's front |
| `box_loot.jpg` | `models/sjk/box_loot.jpg` | 256x256 swatches: the Ray Gun's steel, red fins and dark grip, the coins' gold and rim (made by [box_loot_model.py](../../../../scripts/box_loot_model.py), 11/11/2026) |
| `box_loot_glow.jpg` | `models/sjk/box_loot_glow.jpg` | 64x64: the green glow of the Ray Gun's tube and its bolt |
| `coins_small.md3`, `coins_medium.md3`, `coins_big.md3` | `models/sjk/coins_*.md3` | Heaps of 5, 14 and 30 gold coins, the credits a box pays (50, 200, 500) |
| `raygun.md3` | `models/sjk/raygun.md3` | The Ray Gun, about 14 long, X forward, origin in the grip: a steel barrel and round chamber, three red fins, a cone emitter, a dark grip, and a glowing green tube (its own surface) |
| `raygun_bolt.md3` | `models/sjk/raygun_bolt.md3` | Its bolt, a stretched glowing drop along +X |
| `raygun.gun` | `sjk/raygun.gun` | The Ray Gun as a prop gun (`propgun.rs`), written by hand |
| `raygun_fire.wav`, `raygun_impact.wav`, `raygun_raise.wav`, `raygun_shield.wav` | `sound/sjk/raygun/fire.wav`, `impact.wav`, `raise.wav`, `shield.wav` | The shot's zap, a bolt's crackle, the gun taken in hand, a struck player's shield rising (made by [box_loot_assets.py](../../../../scripts/box_loot_assets.py)) |
