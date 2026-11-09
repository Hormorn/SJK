# WHEEL OF DOOM

A toy any SJK player spins for credits (Sol's design of 07/11/2026). The hub takes the
price, keeps the cooldown and rolls where the wheel lands; the client stands a big dark
wheel before the spinner, spins it, and does what it landed on to the spinner's own keys,
mouse and view, or blows them up, curses them, or gives a Mythical holocron. Its name is
written in capitals wherever a player reads it. Code:
[wheel_of_doom.rs](../crates/sjk-viewer/src/wheel_of_doom.rs) (the wheel, the spin, the
words, the sounds), [effects.rs](../crates/sjk-viewer/src/wheel_of_doom/effects.rs) (what
it does to the spinner), [draw.rs](../crates/sjk-viewer/src/wheel_of_doom/draw.rs) (the
words, the price card, the HUD line, Blind's darkness),
[mirror.rs](../crates/sjk-viewer/src/wheel_of_doom/mirror.rs) (Mirror's flip), and the hub
calls in [sjk-identity's wheel_of_doom.rs](../crates/sjk-identity/src/wheel_of_doom.rs).

**Out of order since 09/11/2026** (Sol: it is bugged): the hub has the wheel off (its
`--wheel-of-doom` switch, off by default). It stays in Collection > Toys, the quick wheel
and the bindings, but every press says "The WHEEL OF DOOM is out of order for now." in the
centre of the screen, with no price card and nothing paid. The client reads that from
`GET /v1/wheel-of-doom`'s `open` (false), and asks again at most every 5 seconds while it
is off, so the wheel spins again without a restart once the hub opens it; a client from
before that change gets the same words from the hub's `wheel_off` refusal at the second
press. To bring it back, set the hub's switch to `everyone` (`staff` to test a fix).

No game server knows of the wheel. It is drawn by SJK clients only and has no collision;
what it does reaches the server only as the user commands the player sends anyway (and,
on `explode`, the game's `kill`).

## Spinning it

`toy_wheelofdoom` (Collection > Toys, its row's "Spin" switch; the quick wheel's Toys
group, to add to a page; or a key bound to it in
Settings > Controls, "Toy: WHEEL OF DOOM"; listed by `toy_help`) works in two presses so
nobody pays by mistake:

1. The first shows the price by the crosshair, on a dark card edged in red: "WHEEL OF
   DOOM" and "100 credits: run it again to spin" (from the Collection, whose page covers
   the crosshair, the centre of the screen says it too).
2. A second within 5 seconds asks the hub to spin (`POST /v1/wheel-of-doom/spin`, signed,
   `{"server":"ip:port"}`).

The client refuses before asking, in the centre of the screen: off a game server (a local
game included), while the hub has not answered the last spin, in a duel, dead, spectating,
with the flag or wherever the [Ragdoll](client.md#ragdoll) could not strike the player
(`Ragdoll::strikable`), during the cooldown it knows ("Your next spin in N s"), and without
the price in credits. The hub refuses on its own (shown in the centre too):
`wheel_cooldown` (its `left` seconds, read from its message, set the client's cooldown, so
the next press says the right wait), `not_enough_credits`, `not_on_server`,
`client_update_required`, and the opening gates (`not_registered`, `opening_staff_only`)
in the hub's words. The price, the cooldown and the segments in order come from
`GET /v1/wheel-of-doom`, read the first time the command runs (then at most every 5
seconds while unread). The cooldown is the hub's (300 seconds, per person, across all
servers); the client starts its own count at the answer.

The wheel rises 110 units before the spinner (nearer, down to 48, when a wall is closer),
on the floor, its face to them, and spins 6 seconds: four whole turns and the last to its
segment, fast at first and slowing to a stop (a cubic ease out, as the mystery box's roll
slows), a tick each time a peg passes the pointer. It stops on the hub's segment, plays the
stop and, on a bad outcome, the laugh, on the jackpot the fanfare; it stands 4 seconds more
and shrinks away in 0.6. Words stand over it for every SJK player who sees it (hidden
behind walls, fading past 1100 units, gone past 1600), in the headline font: "WHEEL OF
DOOM" and "Spinning...", then the outcome's name and what it does. The spinner reads the
same in the centre of the screen.

The other SJK players on the server see the same wheel before the spinner through the
hub's toy event (`wheel_of_doom`, `spin_<outcome>`, from the spinner's slot), where their
game shows the spinner (not when the server does not send them). The feed is read every
few seconds, so they see it start up to that late; their wheel lands on the outcome's
segment by the hub's list (the client's own order when the list is unread). The effects
are the spinner's alone.

## The outcomes

16 segments, index = order round the wheel, segment 0 under the pointer at angle 0, the
others clockwise. The odds are the hub's.

| Segment | Outcome | What happens to the spinner |
| --- | --- | --- |
| 0 | `mythical` | The gold jackpot: a Mythical holocron (source `wheel`, uncapped, untradeable, as box ones). Fanfare, "MYTHICAL HOLOCRON! In your Collection". The profile the hub answers with brings it; the holocrons are read again at once, and it arrives as any new holocron does (the top card in a match, the pop-up in the menu; [holocrons.md](holocrons.md#the-pop-up)); the hub's feed announces it to everyone as a Mythical drop |
| 1 | `nothing` | "Nothing happens..." |
| 2 | `reverse_keys` | 30 s: forward and back, left and right swapped in the user command |
| 3 | `invert_mouse` | 30 s: the mouse's pitch and yaw both turned the wrong way |
| 4 | `mirror` | 30 s: the whole picture flipped left to right, and the strafe and yaw swapped, so it plays mirrored |
| 5 | `upside_down` | 20 s: the view rolled 180 degrees |
| 6 | `blind` | 15 s: the screen fades to black (1.2 s in, 1 s out, never flashing) but for a small pinhole round the crosshair |
| 7 | `drunk` | 30 s: the view sways and rolls slowly (9 degrees of roll at most) and the aim drifts a little (7 degrees, slowly), easing in and out over 1.5 s |
| 8 | `tiny_fov` | 20 s: field of view 40 |
| 9 | `huge_fov` | 20 s: field of view 150 |
| 10 | `no_saber` | 20 s: a lit saber is put out (the saber's own toggle, `sv_saberswitch`'s `generic_cmd`, every 0.75 s while it is lit) and cannot be lit (attacks, which light it, and the toggle are dropped while it is out) |
| 11 | `quack` | 30 s: a quack on every attack press, and every 0.45 s while one is held (SJK's own synthesized quack, heard by the spinner alone) |
| 12 | `walk` | 30 s: forced walk (`BUTTON_WALKING`, moves at the walk's 46) |
| 13 | `bunny` | 20 s: jump held all the while |
| 14 | `explode` | As a mystery box that blows up: the laugh, then 1.5 s after the stop the detpack's explosion where the spinner stands (for every SJK player who sees it) and the spinner's client sends the game's `kill`; never in a duel, with the flag or wherever the Ragdoll could not strike |
| 15 | `curse` | "CURSED... Something bad comes within a minute": one of the twelve effects above (not mythical, nothing, explode or curse) strikes at a random moment within 60 s ("The curse strikes: ...!") |

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
spinner's slot), the quack in the spinner's ears.

| When | Sound |
| --- | --- |
| A peg passes the pointer | `sound/sjk/wheel_of_doom/tick.wav` (bundled) |
| The wheel stops | `sound/sjk/wheel_of_doom/stop.wav` (bundled) |
| A bad outcome (an effect, explode, curse) | The mystery box's laugh: Sol's `sound/sjk/mystery_box/laugh.mp3` in the optional SJK extras pack, else the retail gloat `sound/chars/alora/misc/gloat1.mp3` |
| The jackpot | `sound/sjk/wheel_of_doom/fanfare.wav` (bundled) |
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
  face shows, looks along the model's +X; the client turns it to face the spinner.
- The wheel's origin is its pivot, the middle of its axle. The client sets it 64 units over
  the stand's origin (`AXLE` in `wheel_of_doom.rs`; a model with another height changes that
  constant) and turns it about the model's X axis only, clockwise as seen from the front
  (from +X looking back).
- Segments: 16, 22.5 degrees each. At angle 0, segment 0 (`mythical`) is centred straight up
  (+Z), under the pointer at the top; segment k is centred k x 22.5 degrees clockwise of the
  top as seen from the front (segment 1 to the right of the top, with +Y on the viewer's
  right). The pegs sit on the dividers, half a segment either side of each middle (the ticks
  are timed on them).
- The pointer is part of the stand, at the top, pointing down at the wheel's rim.

## Limits

- The wheel is SJK's only: players on other clients see someone standing there, and the
  wheel has no collision.
- Others see a spin when the hub's feed reaches them (every few seconds), and only while
  their game shows the spinner.
- The effects are the spinner's own client's: a modified client could ignore them. They
  cost a few comparisons a frame and allocate nothing; the words are written into fixed
  strings.
- No saber uses the saber's own toggle in the user command; a mod that ignores it, or a
  staff or dual saber half lit, may keep a blade on.
- Mirror flips the HUD too, and needs a surface the client can copy from.
- Bunny holds the jump even while the player types in the chat.
- Verified: unit tests (the outcomes and their order, each segment stopping under the
  pointer, the spin slowing to its stop, the ticks, the stop, the laugh, the fanfare, the
  blast after explode and once, the wheel going after its hold, an unknown outcome shown by
  name and harmless, the refusals (a duel, the cooldown and its seconds, too few credits,
  off a server), the effects' input transforms (reverse, invert, mirror, walk, bunny, no
  saber, quack), the view effects (upside down, the FOVs, blind's fade), timers ending and
  clearing (a new map, a death), the curse picking a bad effect within a minute; on the
  identity side the wheel's and a spin's answers, the feed event, the refusals and the
  cooldown's seconds). Not tried in a game, nor against the hub (its side is on the hub's
  `wheel-of-doom` branch, 07/11/2026): the wheel's look and placement, the ticks' timing,
  the Mirror pass on a GPU, the pinhole's look, the saber toggle on real servers.
