# WHEEL OF DOOM

Big dark wheels SJK's creator places on maps (Sol's design of 07/11/2026; placed since
11/11/2026), which any SJK player spins with Use for a DOOM coin, the only way to spin
one: a [mystery box](mystery-boxes.md) that blows up (the teddy bear) brings a coin. The
hub keeps where the wheels stand, spends the coin, keeps the cooldown and rolls where the
wheel lands; the client draws the wheels, spins the one the player faces to the end of its
music, and does what it landed on to the spinner's own keys, mouse and view, or blows
them up, curses them, gives the coin back or a holocron of any tier. Its name is written in
capitals wherever a player reads it. Code:
[wheel_of_doom.rs](../crates/sjk-viewer/src/wheel_of_doom.rs) (the wheels, the spin, the
sounds, the creator's commands), [effects.rs](../crates/sjk-viewer/src/wheel_of_doom/effects.rs) (what
it does to the spinner), [draw.rs](../crates/sjk-viewer/src/wheel_of_doom/draw.rs) (the
words over the wheels, the HUD line, Blind's darkness),
[mirror.rs](../crates/sjk-viewer/src/wheel_of_doom/mirror.rs) (Mirror's flip), and the hub
calls in [sjk-identity's wheel_of_doom.rs](../crates/sjk-identity/src/wheel_of_doom.rs).

**Before 11/11/2026** the wheel was a toy: `toy_wheelofdoom` stood one before the
spinner, who paid 100 credits on a second press, every 5 minutes, 16 segments, a 6-second
spin. From 09/11/2026 it was out of order (the hub's `--wheel-of-doom` switch off, Sol:
bugged). Sol, 11/11/2026: "make the wheel of doom placeable, add a holo of every type on
it, only way to play it is with a wheel of doom coin and it plays the WHEEL OF DOOM music
fully and lands on the result at the end of it, only a few seconds CD, synced with the
hub". The hub's switch is `everyone` by default again; `off` still shows "out of order".

No game server knows of the wheels. They are drawn by SJK clients only and have no
collision; what they do reaches the server only as the user commands the player sends
anyway (and, on `explode`, the game's `kill`).

## Placing one (SJK's creator)

As a mystery box is placed (the same hub checks, the map file's checksum, the staff log):

| Command | Does |
| --- | --- |
| `wheelofdoom_place [server]` | Place a wheel where you aim, its face towards you: on every server running this map file, or with `server` on this one only |
| `wheelofdoom_remove [id]` | Remove the wheel nearest you (within 256 units), or the one with that id |
| `wheelofdoom_list` | The wheels on this map, their ids and distance, and your DOOM coins (anyone) |

The client reads the wheels with the mystery boxes, every 10 minutes and at once after a
placing or a removal (`GET /v1/wheel-of-doom`, which also says the player's coins). At
most 8 wheels a map are kept. The map editor has no button for them yet.

## Spinning one

Standing at a wheel (within 120 units across the floor and 96 up or down) and facing it
(within 60 degrees), the player sees the [Use prompt](client.md#use-prompt): "Spin the
WHEEL OF DOOM" over "1 DOOM coin (you hold N)", or greyed with why not ("No DOOM coin: the
teddy bear brings one", "Someone is spinning it", "Your turn again in N s", "Out of order
for now"). Use asks the hub (`POST /v1/wheel-of-doom/spin`, `{"wheel":<id>,"server":"ip:port"}`);
the client refuses first, in the centre of the screen, in a duel, dead, spectating, with
the flag or wherever the [Ragdoll](client.md#ragdoll) could not strike the player, while
the hub has not answered the last spin, while the wheel spins, during the cooldown it knows
and without a coin. The hub refuses on its own too (shown in the centre): `no_doom_coin`,
`wheel_busy` (with the seconds left), `wheel_cooldown`, `no_such_box`, `not_on_server`,
`wheel_off`, `client_update_required`. The cooldown is the hub's: 40 seconds from a spin's
start, a few past its end (per person, across all wheels); a wheel takes one spin at a time.

`toy_wheelofdoom` (Collection > Toys, its "Coins" switch; the quick wheel; a key in
Settings > Controls) says the player's DOOM coins and how many wheels stand on the map.

A spin lasts 34 seconds, the length of Sol's WHEEL OF DOOM music, which plays at the wheel
for everyone (`sound/sjk/wheel_of_doom/music.mp3`, in the optional SJK extras pack; without
it the wheel spins to its ticks alone). The wheel turns 14 times and the rest of the way to
its segment from where it last stopped, fast at first and crawling at the end (a quartic
ease out), so who wins stays open to the last note; a tick plays each time a peg passes
the pointer. It stops on the hub's segment as the music ends, plays the stop and, on a bad
outcome, the laugh, on a holocron or the coin back the fanfare, and keeps its words 6
seconds more. The wheel then rests where it landed. Words stand over each wheel for every
SJK player who sees it (hidden behind walls, fading past 1100 units, gone past 1600), in
the headline font: "WHEEL OF DOOM" and "A DOOM coin a spin: press Use", "Spinning...", then
the outcome's name and what it does. The spinner reads the same in the centre.

The other SJK players on the server see the same wheel spin through the hub's toy event
(`wheel_of_doom`, `land_<wheel>_<outcome>`, from the spinner's slot), when their feed
brings it (every few seconds); their wheel lands on the outcome's segment by the hub's list
(the client's own order when the list is unread). The effects are the spinner's alone.

## The outcomes

Segments: 20 (16 before 11/11/2026), index = order round the wheel, segment 0 under the
pointer at angle 0, the others clockwise, the holocrons spread among the bad ones. The
odds are the hub's, PROVISIONAL: a holocron of some tier 29% of the time.

| Segment | Outcome | Chance | What happens to the spinner |
| --- | --- | --- | --- |
| 0 | `mythical` | 1 % | The gold jackpot: a Mythical holocron (source `wheel`, uncapped, untradeable, as box ones), Sol's exception of 07/11/2026. Fanfare, "MYTHICAL HOLOCRON! In your Collection". The holocrons are read again at once, and it arrives as any new holocron does ([holocrons.md](holocrons.md#the-pop-up)); the hub's feed announces it to everyone as a Mythical drop |
| 1 | `reverse_keys` | 4 % | 30 s: forward and back, left and right swapped in the user command |
| 2 | `holo_uncommon` | 17 % | An Uncommon (green) holocron, as the Mythical one: source `wheel`, uncapped, untradeable |
| 3 | `invert_mouse` | 4 % | 30 s: the mouse's pitch and yaw both turned the wrong way |
| 4 | `explode` | 8 % | As a mystery box that blows up: the laugh, then 1.5 s after the stop the detpack's explosion where the spinner stands (for every SJK player who sees it) and the spinner's client sends the game's `kill`; never in a duel, with the flag or wherever the Ragdoll could not strike |
| 5 | `holo_rare` | 8 % | A Rare (blue) holocron, the same |
| 6 | `mirror` | 4 % | 30 s: the whole picture flipped left to right, and the strafe and yaw swapped, so it plays mirrored |
| 7 | `upside_down` | 4 % | 20 s: the view rolled 180 degrees |
| 8 | `doom_coin` | 5 % | "DOOM COIN BACK! Spin it again": the coin spent comes back |
| 9 | `blind` | 4 % | 15 s: the screen fades to black (1.2 s in, 1 s out, never flashing) but for a small pinhole round the crosshair |
| 10 | `holo_legendary` | 3 % | A Legendary (purple) holocron, the same |
| 11 | `drunk` | 4 % | 30 s: the view sways and rolls slowly (9 degrees of roll at most) and the aim drifts a little (7 degrees, slowly), easing in and out over 1.5 s |
| 12 | `tiny_fov` | 4 % | 20 s: field of view 40 |
| 13 | `nothing` | 5 % | "Nothing happens..." |
| 14 | `huge_fov` | 4 % | 20 s: field of view 150 |
| 15 | `no_saber` | 4 % | 20 s: a lit saber is put out (the saber's own toggle, `sv_saberswitch`'s `generic_cmd`, every 0.75 s while it is lit) and cannot be lit (attacks, which light it, and the toggle are dropped while it is out) |
| 16 | `quack` | 4 % | 30 s: a quack on every attack press, and every 0.45 s while one is held (SJK's own synthesized quack, heard by the spinner alone) |
| 17 | `walk` | 4 % | 30 s: forced walk (`BUTTON_WALKING`, moves at the walk's 46) |
| 18 | `bunny` | 4 % | 20 s: jump held all the while |
| 19 | `curse` | 5 % | "CURSED... Something bad comes within a minute": one of the twelve effects above (not mythical, nothing, explode or curse) strikes at a random moment within 60 s ("The curse strikes: ...!") |

A small line under the top of the HUD, in the wheel's red, names the effect running and its
seconds left ("WHEEL OF DOOM · MIRRORED · 23 s"), or "CURSED..." while a curse waits. One
effect runs at a time. Every effect ends on its timer, on a map change or another server, on
a disconnect and on the player's death (a curse waiting too). An outcome a newer hub knows
and this client does not is shown over the wheel by its name ("BIG PARTY") with "Nothing
happens...", and does nothing.

Mirror flips the finished frame, the HUD included: once it is drawn, the window's image is
copied into a texture of its size and drawn back mirrored (a mirrored projection would turn
the culling inside out). The texture is made the first frame it is needed and let go when
the effect ends. A surface that cannot be copied from (no `COPY_SRC`) keeps the picture as
it is; the controls are still mirrored.

## Sounds

Every sound plays at the wheel on a sound source of the wheel's own (`0x5600_0000` and the
wheel's id), the quack in the spinner's ears.

| When | Sound |
| --- | --- |
| The spin, from its start to its stop | Sol's music, `sound/sjk/wheel_of_doom/music.mp3`, in the optional SJK extras pack (none without it) |
| A peg passes the pointer | `sound/sjk/wheel_of_doom/tick.wav` (bundled) |
| The wheel stops | `sound/sjk/wheel_of_doom/stop.wav` (bundled) |
| A bad outcome (an effect, explode, curse) | The mystery box's laugh: Sol's `sound/sjk/mystery_box/laugh.mp3` in the optional SJK extras pack, else the retail gloat `sound/chars/alora/misc/gloat1.mp3` |
| A holocron, the coin back | `sound/sjk/wheel_of_doom/fanfare.wav` (bundled) |
| Explode | The detpack's `sound/weapons/detpack/explode.mp3` (retail) |
| Quack | `sound/sjk/wheel_of_doom/quack.wav` (bundled) |

The bundled ones are SJK's own, synthesized by
[wheel_of_doom_assets.py](../scripts/wheel_of_doom_assets.py). Settings > Sound > Toys,
"WHEEL OF DOOM sounds" (`s_toyWheelOfDoomSound`, on by default), silences them all on this
PC.

## The model

The wheel is a placeholder made by [wheel_of_doom_model.py](../scripts/wheel_of_doom_model.py),
bundled in [assets/wheel_of_doom](../crates/sjk-viewer/assets/wheel_of_doom/README.md) and
mounted below the game data, so a PK3 with the same paths replaces it. Sol's final model
keeps this contract:

- Two MD3 files: `models/sjk/wheel_of_doom_stand.md3` (everything that stands still: the
  stand, the axle, the pointer) and `models/sjk/wheel_of_doom_wheel.md3` (the wheel that
  turns, its pegs on it). One surface each in the placeholder, both with the shader
  `models/sjk/wheel_of_doom` (`shaders/sjk_wheel_of_doom.shader`: the atlas
  `models/sjk/wheel_of_doom.jpg` lit as a player model, its glow
  `models/sjk/wheel_of_doom_glow.jpg` added, both sides drawn). Any number of surfaces and
  shaders may replace them; the client draws whole models.
- Scale: game units, as drawn (the client scales by 1). The placeholder's wheel is 40 units
  in radius and 4 thick, its axle 64 units over the floor, the pointer's arm 116 high.
- The stand's origin is on the floor under the wheel's middle. Its front, the side the wheel's
  face shows, looks along the model's +X; the client turns it the way the creator placed it.
- The wheel's origin is its pivot, the middle of its axle. The client sets it 64 units over
  the stand's origin (`AXLE` in `wheel_of_doom.rs`; a model with another height changes that
  constant) and turns it about the model's X axis only, clockwise as seen from the front
  (from +X looking back).
- Segments: 20, 18 degrees each (16 of 22.5 before 11/11/2026). At angle 0, segment 0
  (`mythical`) is centred straight up (+Z), under the pointer at the top; segment k is
  centred k x 18 degrees clockwise of the top as seen from the front (segment 1 to the right of the top, with +Y on the viewer's
  right). The pegs sit on the dividers, half a segment either side of each middle (the ticks
  are timed on them).
- The pointer is part of the stand, at the top, pointing down at the wheel's rim.

## Limits

- The wheels are SJK's only: players on other clients see someone standing there, and the
  wheels have no collision.
- Others see a spin when the hub's feed reaches them (every few seconds late), so their
  music and stop come that much later than the spinner's.
- The effects are the spinner's own client's: a modified client could ignore them. They
  cost a few comparisons a frame and allocate nothing; the words are written into fixed
  strings.
- No saber uses the saber's own toggle in the user command; a mod that ignores it, or a
  staff or dual saber half lit, may keep a blade on.
- Mirror flips the HUD too, and needs a surface the client can copy from.
- Bunny holds the jump even while the player types in the chat.
- The music is in the extras pack; the Sith cloud's copy is Sol's to upload. Sol tests it
  from `GameData/base/zz_sjk_wheel_of_doom_sounds_local.pk3`.
- At most 8 wheels a map; the map editor's pages do not list them yet.
- Verified: unit tests (the 20 outcomes and their order, each segment stopping under the
  pointer from any rest, the 34-second spin slowing to its stop, the music, the ticks, the
  stop, the laugh, the fanfare, the blast after explode and once, the rest where it landed,
  an unknown outcome shown by name and harmless, the refusals and the Use prompt (a duel,
  busy, the cooldown, no coin, out of order), the wheel faced, the wheels of a map file and
  server, the effects' input transforms and timers, the curse; on the identity side the
  wheels, coins, a spin's answer, the feed event and the refusals; on the hub the coins, the
  busy wheel, the cooldown, each holocron segment, the coin back, placing and removing).
  `duel6_wheel_of_doom_stopped` renders the 20-segment wheel. Not tried in a game: the
  music's timing against the stop, the Use prompt, the creator's placing.
