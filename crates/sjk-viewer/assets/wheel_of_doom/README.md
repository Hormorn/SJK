# WHEEL OF DOOM

The WHEEL OF DOOM toy (SJK's own, [wheel-of-doom.md](../../../../docs/wheel-of-doom.md)).
Everything here is a placeholder drawn by
[wheel_of_doom_model.py](../../../../scripts/wheel_of_doom_model.py) (model, pictures,
icon) and [wheel_of_doom_assets.py](../../../../scripts/wheel_of_doom_assets.py) (sounds)
from shapes and synthesized sound, nothing taken from another game, so none is edited by
hand except `wheel_of_doom.shader`. Sol's final model replaces the files one for one: the
contract is in [wheel-of-doom.md](../../../../docs/wheel-of-doom.md#the-model).

| File | Game path | What |
| --- | --- | --- |
| `toy_wheelofdoom.png` | `gfx/sjk/toy_wheelofdoom.png` | The toy's icon (Collection > Toys, key bindings), 128x128 |
| `wheel_of_doom.jpg` | `models/sjk/wheel_of_doom.jpg` | The atlas, 1024x1024: the wheel's face top left, strips for its back, rim, iron, the pointer and the stand |
| `wheel_of_doom_glow.jpg` | `models/sjk/wheel_of_doom_glow.jpg` | What glows, added by the shader |
| `wheel_of_doom_stand.md3` | `models/sjk/wheel_of_doom_stand.md3` | The stand and the pointer, origin on the floor under the wheel, face along +X |
| `wheel_of_doom_wheel.md3` | `models/sjk/wheel_of_doom_wheel.md3` | The wheel, origin on its axle, turned about its X axis |
| `wheel_of_doom.shader` | `shaders/sjk_wheel_of_doom.shader` | `models/sjk/wheel_of_doom`: lit, the glow added, both sides drawn |
| `tick.wav`, `stop.wav`, `fanfare.wav`, `quack.wav` | `sound/sjk/wheel_of_doom/*.wav` | A peg passing the pointer, the stop, the jackpot's fanfare, the Quack effect |

The client mounts them in memory below all game data
([wheel_of_doom.rs](../../src/wheel_of_doom.rs)), so a PK3 with the same paths replaces
them.
