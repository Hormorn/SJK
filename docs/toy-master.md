# The Toy Master

Secret toys SJK's creator, the Toy Master, hides on maps for SJK players to find (Sol's
idea, 30/10/2026). The surprise is the point, so this repository holds only the
mechanism: a prop that counts `+use` presses and plays a show. What each toy is, its
model, its sound, its show and the achievement for finding it, lives on the SJK hub,
which tells a client only of the toys standing on its map and names an achievement only
to whoever unlocked it. Code: [toy_master.rs](../crates/sjk-viewer/src/toy_master.rs),
the hub calls in [sjk-identity's toy_master.rs](../crates/sjk-identity/src/toy_master.rs).

No game server knows of a toy. SJK clients alone draw it, it has no collision (players
walk through it), and players on other clients see someone pressing Use at nothing.

## What the hub sends

`GET /v1/secrets` (once registered, then every 10 minutes, and at once after the
creator hides or removes one) lists every placed toy: its id, the map file it stands in
(name and BSP checksum, as [mystery boxes](mystery-boxes.md)), a server when it is
placed for one only, where it stands and faces, and how it looks and plays:

- `model`, an MD3 at a game path, from the hub's [asset packs](identity.md#asset-packs) (the client
  downloads and mounts them; a pack that arrives during a match is drawn from the next
  map), grown by `scale`;
- `sound`, the sound its show plays, which may come from a pack a player does not have
  (the Sith cloud's optional [extras](sith-cloud.md)): without it the show plays
  silent;
- `show`: how long it shrinks away, stays gone and pops back, how long it spins and its
  fastest spin; since 02/11/2026 also its steps (below), how long they last
  (`length`), how far round the toy the players take part (`reach`), and the model of
  the ring drawn on the ground to show it (`ring`, with its own radius,
  `ring_radius`).

The creator's list also names each toy's kind and the kinds there are.

## Finding one

Standing at a toy (within 80 units across the floor and 96 up or down) and facing it
(within 60 degrees), each `+use` press counts. Nothing says what a toy does: it is a
surprise. Facing one shows the [Use prompt](client.md) by the crosshair, as at a
[mystery box](mystery-boxes.md), with only "???" for its action (Sol, 01/11/2026); the
cap greys once the toy takes no more presses, and no prompt stands while the toy is
shrunk away in its show. The client sends the presses on the toy twice a second at most
(`POST /v1/secrets/use`, at most 10 a time), with the server and the map load
(the game's `sv_serverid`, which every player of a server reads alike).

- The player's first press on a kind of toy finds it: the hub unlocks that toy's hidden
  achievement and its kyber crystal, and the client announces it as any achievement
  (the unlock card and a console line).
- The presses of every player on the server add up, towards a number of presses the hub
  rolls for each map load and never tells. The presses that reach it start the toy's
  show, once for that map load: the toy shrinks away (`sound/player/teleout.mp3` and the
  `mp/spawn` sparkle), stays gone, pops back past its size and settles
  (`sound/player/telein.mp3`, the sparkle again) and spins, faster and faster, then
  slowing to a stop, while its sound plays from where it stands (heard to 1400 units).
  The hub tells the server's other SJK players through its toy event (`secrets`,
  `show_<id>`), so they see and hear the same show.
- While it spins, the players' models around it dance (Sol, 01/11/2026): each turns
  with the toy and squishes in step with its hops, flattened by up to 30 % and as much
  wider, feet kept on the floor, harder as the spin speeds up. Within 320 units fully,
  fading out to none at 512; vehicles and limp players are left out. It is the drawing
  alone (`toy_master::dance_actor`, from `actor_world_submission`): where the game has
  them and where they face never change, and the local player sees their own model
  dance only in third person.
- Once a toy played its show on a map load, presses on it count no more until the next
  (a player who has not found it yet still finds it with one press).
- A player who arrives after a show started (connected or loaded the map late) joins it
  where it is (since 03/11/2026): the hub's presence answer, read every few seconds,
  lists the shows under way on the server (`shows`: the toy, the map load, how long ago
  it started, the drops' seed and letters and the ones taken), and the client starts
  the show that far in (`ToyMaster::join`), past its vanish and pop when they are over,
  its sound from as far in (`PlayTrack`, as the boombox's), its drops thrown as they
  were with the ones taken gone. A toy that played or joined a show on this map load is
  not joined again.
- The show's sound plays on a channel of its own kept from other sounds
  (`TOY_SHOW_CHANNEL`, protected as the boombox's): a song of minutes is never stolen
  by a busy fight, so a player far away when it started hears it from where it is when
  they walk up.
- The steps and the voice are timed on the sound, which starts as the toy pops back:
  `track.rs` reads them the pop's length (`pop`) ahead of the lead (since 03/11/2026;
  before, they came 450 ms late).

## A show in steps

Since 02/11/2026 a show may go on in steps (`Show::steps`, Sol's design for the first
toy, timed on its sound): each from a moment after the pop, when the sound starts, to the
next, saying how fast the toy spins (`spin`, turns a second, 0 still) and what else goes
on (`toy_master/track.rs`). A show without steps plays as above; a client older than
steps plays a stepped show's spin for `spin` only.

- `dance`: the players within `reach` dance as above (fading out over 48 units more),
  turning as fast as the toy.
- `ring`: the ring model lies on the ground round the toy, grown to the reach, turning at
  a quarter of the toy's speed. It is flat at the toy's floor: ground higher than that
  hides part of it.
- `float`: the toy rises and drifts, tumbling, and every player within reach as the step
  begins floats with no gravity (`toy_master/float.rs`): drawn as the Ragdoll toy draws a
  limp player (`ragdoll::set_floating`, a body of the ragdoll's own with
  `Body::weightless`), pushed up and away and gently now and then, turning with the
  toy while it spins. Only their looks float: their view, aim and real body are
  untouched. When the floating ends the bodies fall back to the floor, and 3.5 seconds
  later the players are drawn standing again. Each client floats them by itself, from
  the show's seed and the slot.
- `spit`: the toy throws out the show's drops, spread evenly over the step (below).

A show may also have a voice (`Show::voice`, since 03/11/2026): the spans, from and to in
milliseconds after its sound starts, when its sound sings. Then the toy spins only within
them, at its step's speed, and stands still between, starting and stopping over 90 ms
(`VOICE_RAMP`): the dancers and the ring, which turn with it, stop too. The other steps'
flags (dance, ring, float, spit) keep their own times, so the drops are spread as before.
A client older than the voice ignores it and spins all through its steps. The hub takes
the spans from the toy's song with its voice separated out (Demucs).

Changes are eased: a spin's speed over 0.7 s, the dance and the ring over 0.5 s, the
toy's rise over 1.5 s and its landing over 0.9 s. The toy's sound plays for the whole
show and stops at its end.

## Drops

A show with drops gives real rewards, the hub's to roll (Sol, 02/11/2026: "modest",
and no cap: whoever takes a drop first has it). Right after `show_<id>` the hub tells
the server's SJK players what the show throws: `drops_<id>_<seed>_<letters>`, a letter a
drop, `c` credits, `s` kyber shards, `h` a holocron; how much each gives is told only to
its taker. Every client throws the drops alike from the seed
(`toy_master/drops.rs`): drop *i* of *n* leaves the toy's mouth at
`lead + from + (to - from) * (2i + 1) / 2n` milliseconds into the show (the `spit` step
from `from` to `to`; `Show::drop_time`, the hub reckons it the same way), flies in fixed
16 ms steps with gravity, bounces off the map and slides to rest, the same everywhere.
A credits drop is drawn as a green crystal, kyber shards as a kyber crystal, a holocron
as SJK's holocron.

A drop at rest, within 64 units across the floor and 80 up or down and faced, shows the
Use prompt ("Take the credits", "Take the kyber shards", "Take the holocron"); Use asks
the hub for it (`POST /v1/secrets/take`) and greys the prompt for 3 seconds while it
answers. The first to ask gets it: the console says what it gave, and the hub tells
every SJK player there (`taken_<id>_<drop>`), whose clients stop drawing it. A drop
someone else took first, or one gone, is refused in a line. Drops wait a minute after
the show ends. History lists them as taken from a Toy Master's toy.

Every sound plays on a source of the toy's own (`0x5300_0000` and its id), and the
show's sound, played twice as loud as its recording (`SHOW_VOLUME`, since 02/11/2026),
stops when the show ends or the player leaves the map. The sounds are
looked up again when the game data changes, so a pack that brings the show's sound
during a session (the Sith cloud's extras) is heard from its next map. A show whose sound
is not in the game data plays silent and says so in the log (`toy master: the show's
sound ... is not in the game data`). Settings > Sound >
Toys, "Toy Master sounds" (`s_toyMasterSound`), silences them.

## The "?" and the achievements

Collection > Toys has a "?" at the right of its column heads, the Toy Master's: its
hover card says the Toy Master hides toys on maps, that each one found is a secret
achievement, and names the ones the player found. It turns gold once they found one.

A Toy Master's achievement is not in the client's catalogue (`achievements::ALL`):
a public catalogue cannot keep a secret. The hub sends it with the player's other
achievements only once it is unlocked, with its name, what it was for and its group,
`toy_master` (`sjk_identity::Achievement::name`, `description`, `group`), and the client
keeps it for the session (`achievements::secret_kind`). The Achievements wall shows the
ones found in a "Toy Master" category on its Fun page, after the toys', on the first band
with room ("2 found"); they count in the unlocked total and in the total alike, so nothing tells
how many there are. Other players' cards count the ones they found the same way.

## Hiding one (SJK's creator)

| Command | Does |
| --- | --- |
| `toymaster_place <kind> [server]` | Hide a toy of that kind where you aim, facing you: on every server running this map file, or with `server` on this one only |
| `toymaster_remove [id]` | Take away the toy nearest you (within 256 units), or the one with that id |
| `toymaster_escort [id] on\|off` | Show the toy nearest you (or the one with that id) only on the server you are on, or to everyone again |
| `toymaster_list` | The kinds there are, and the toys on this map with their ids and distance |

`toymaster_place <kind> [server] escort` hides one shown only where the creator is
(since 03/11/2026): the hub lists an escorted toy to anyone else only when they are on
the server the creator is on, for that server alone, and the presence answer's
`secrets` stamp changes when the creator comes or goes (or a toy is placed, taken away
or escorted), so the clients there read the toys at once and the toy appears or goes
within seconds. A client from before then never sees an escorted toy. `toymaster_list`
says which are escorted.

The browser map editor's World page ([map-edits.md](map-edits.md#the-browser-editor-creator-only-for-now))
shows the toys and hides one of a kind (for this server, escorted) where a click meets the
floor, or takes one away, through the same code (`toy_master_place_at`, `toymaster_remove
<id>`); the hub has no move.

All of these are the creator's only. Placing needs the map's BSP checksum (as map edits
do, `sv_mapChecksum`). The hub takes the creator's request only; every placing and
removal goes to the staff log.

## Limits

- The hub cannot tell where a player stands: the reach is the client's, so a modified
  client could press a toy from anywhere. A toy's show comes at most once every 10
  minutes on a server whatever map load a client names.
- A toy can only do what the client's show knows: shrink away, pop back and spin to a
  sound. A toy that does something else needs a client release for that behaviour (named
  for what it does, never for the toy).
- Verified: unit tests (the show's phases and spin, the presses gathered and sent, the
  faced toy, the toys of a map file, a show's sounds once each; the hub-named
  achievements, the wall's bands; on the hub, placing and removing as the creator only,
  a find unlocking once, every player's presses adding up to one show per map load, the
  gap between shows, a toy placed for one server), and world shots of the "?" and the
  wall (`duel6_sjk_toy_master`) and of a toy in each phase from a hub pack
  (`duel6_toy_master`, with `SJK_TEST_TOY_PACK` and `SJK_TEST_TOY_MODEL`). Not tried in
  a game with a hub yet.
