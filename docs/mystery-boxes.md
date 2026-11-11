# Mystery boxes

Boxes SJK's creator places on maps, which any SJK player opens for credits (Sol's idea,
27/10/2026). The hub keeps where each box stands, takes the price, rolls what is inside
and pays it; the client draws the boxes on the map file they were placed on, opens the one
the player faces with `+use`, and shows what came out to the opener and to the SJK players
around. Code: [mystery_box.rs](../crates/sjk-viewer/src/mystery_box.rs), the hub calls in
[sjk-identity's mystery_box.rs](../crates/sjk-identity/src/mystery_box.rs).

No game server knows of a box. It is drawn by SJK clients only, has no collision (players
walk through it), and players on other clients see someone pressing Use at nothing.

The box is SJK's own (bundled, `assets/mystery_box`, made by
[mystery_box_model.py](../scripts/mystery_box_model.py)): a long wooden chest bound in
iron, 60 x 26 x 24 units, a pale question mark glowing (pulsing) on each half of its two
long sides, and a lid hinged at the back that swings open while it rolls and closes once
what came out is taken. Since 30/10/2026 (Sol: "not just that rectangle") it stands on
four iron feet under a plinth, its three iron bands and four corner posts stand out of the
planks, an iron rim runs round its top, a lock plate hangs on its front and a handle on
each end, and its lid arches, banded too. Since 31/10/2026 it is open under its lid, dark
walls and a floor inside (Sol: the top looked sealed with wood; it was closed with
planks and the bands' tops crossed it), and a faint halo of warm light rises 52 units
round its top, its rays drifting slowly, so a box is found from afar (Sol's idea; a second
surface of the chest's model, `models/sjk/mystery_box_halo`, added and two-sided).

Words stand in the world over every box, in the headline font (Fira Sans Condensed, as
centre prints), for every SJK player who sees it (hidden behind
walls, fading out past 1000 units, gone past 1400): "MYSTERY BOX" and its price (or "Free
roll: press Use" for a player holding one), "Rolling..." while it rolls, then what came
out (the credits, a holocron's or crystal's tier, the Ray Gun...; the opener's also says
to press Use to take it).

## Opening one

Standing at a box (within 80 units across the floor and 96 up or down) and facing it
(within 60 degrees), the player sees the [Use prompt](client.md#use-prompt) by the
crosshair: the key bound to `+use` in a cap, then "Open mystery box" over its price or
"Free roll" (since 30/10/2026; a centre print said it once before). `+use` asks the hub to
open it; the hub refuses without the price, within the cooldown or when the box is gone,
and the refusal shows in the centre of the screen. What the client knows already, the
prompt says instead, its cap greyed: "Your turn again in N s", "100 credits: not
enough", "Opening..." while the hub answers, "Someone is opening it" at another player's
roll; it says "Take it" over the player's own prize, and nothing while their own roll
runs or at a box that blows up.

**The cooldown** (Sol, 30/10/2026): a player opens one box every 15 seconds (3 before,
the hub's `cooldown`), so a roll and its taking, about 8 seconds, leave the box free a
while for another player. The client knows its own last opening and says so on the Use
prompt without asking the hub.

An opened box plays its opening sound and its roll while it rattles and things rise out of
it and roll over it, changing fast, then slower, like a wound-down spring. At 5.2 seconds
(5.5 before 30/10/2026), on the roll's last note, it stops on what came out and plays a
chime, or a fanfare (the jackpot, the Mystery blade, a kyber crystal). As its lid closes
(taken, or after 10 seconds) it plays its closing sound.

The opening, the roll and the closing are Sol's sounds of 30/10/2026, in the optional SJK
extras pack (`unofficial_sjk_extras.pk3`, the Sith cloud's `extras`:
`sound/sjk/mystery_box/extras_open.mp3`, `extras_roll.mp3`, `extras_close.mp3`), not
bundled. A player without the pack hears a crate sliding open (retail) and a little music
box tune (SJK's own, bundled: `assets/mystery_box`, made by
[mystery_box_assets.py](../scripts/mystery_box_assets.py), its last chord at 5.2 s), and
no closing sound. Both rolls end together, so every client stops at the same time.

| Outcome | Stops on |
| --- | --- |
| `coins` | a small heap of gold coins |
| `double` | a pile of gold coins |
| `jackpot` | a big pile of gold coins |
| `explode` | a teddy bear (a thermal detonator before 30/10/2026) |
| `holocron` | the tier's own holocron, every part of it, for everyone (since 11/11/2026 the hub's event names the tier; SJK's placeholder holocron for others before) |
| `kyber` | the tier's own kyber crystal, for everyone |
| `shards` | three cyan kyber crystals; seven, bigger, for the opener's pile of more than 15 shards |
| `raygun` | the Ray Gun |
| `blade` | a saber hilt; for an opener who holds the blade already, the shards it pays instead |

Before 11/11/2026 (Sol: "replace the crystals for credits with actually a pile of coins")
`empty` stopped on a handful of bolts, `refund` on a blue crystal, `double` on a green
one, `jackpot` on a red one and `again` on a little mystery box; an older hub's `empty`
and `again` still do. The coins, the Ray Gun and its bolt are SJK's own, bundled, made by
[box_loot_model.py](../scripts/box_loot_model.py).

What rolls before it stops is only these prizes (Sol, 29/10/2026: the game's weapons
rolled with them before, which no box gave), but the teddy bear, which only comes at the
end (30/10/2026): the three piles of coins, both piles of shards, the hilt, the Ray Gun,
the four tiers' holocrons, Uncommon to Mythical, and the Rare and Mythical kyber crystals,
each drawn as the Holocrons tab draws it, its parts posed and turning
(`mystery_box::HOLOCRONS`, `KYBERS`; Sol, 11/11/2026: "we should see the kyber models and
holos too", no placeholder). `world_shot::mystery_box_shots::duel6_mystery_box_holocrons`
shows the four holocrons, `duel6_mystery_box_kybers` the four crystals,
`duel6_mystery_box_rework` the coins and the Ray Gun, and `duel6_mystery_box_loot` the
shards, the little box and the hilt.

The opener is told what came out in the centre of the screen. It floats over the box,
turning slowly, until they take it with `+use` at the box (a sparkle; a chat line gives
the credits left) or for 10 seconds, when it is taken for them. Meanwhile the box is busy:
another player's Use prompt there says someone is opening it. The hub pays at the opening
itself, so walking away loses nothing. The other SJK players on the server see and hear
the same roll through the hub's toy event (`mystery_box`, `open_<box>_<outcome>`). When
the opener takes what came out, their client tells each other SJK player on the server
through the toy relay (`mystery_box`, `take_<box>`, one player every 50 ms), and their box
closes too, with the take and closing sounds (30/10/2026; before, they saw it float for
the 10 seconds). Only the opener's take closes it; a player who misses it sees it go at
the 10 seconds.

Every sound plays at the box on a sound source of the box's own (`0x5200_0000` and its id),
never an entity's: the audio moves an entity's sounds to where it is, so box sounds once
followed the player on slot 0 (bug report #185).

Settings > Sound > Toys, "Mystery box sounds" (`s_toyMysteryBoxSound`), silences them.

The price and the odds are the hub's (`GET /v1/mystery-boxes`), PROVISIONAL, Sol's to
tune. Since 11/11/2026 (Sol: never empty loot, never the 100 credits back, no free
rerolls, much more likely at least a green holocron since a box costs about one, a very
rare kyber crystal):

| Outcome | Chance | Pays |
| --- | --- | --- |
| `holocron` | 45.5 % | a holocron, Uncommon (green) or better |
| `coins` | 15 % | 50 credits |
| `double` | 6 % | 200 credits |
| `jackpot` | 1 % | 500 credits |
| `explode` | 12 % | a DOOM coin; the price goes to a hunter |
| `shards` | 12 % | 10 to 25 kyber shards |
| `raygun` | 7 % | the Ray Gun for 30 seconds |
| `blade` | 1 % | the Mystery blade (60 shards if you hold it) |
| `kyber` | 0.5 % | a kyber crystal: 60 % Uncommon, 30 % Rare, 9 % Legendary, 1 % Mythical |

On average a box pays back about 25 of its 100 credits in coins, beside a holocron nearly
one time in two. Before 11/11/2026: `empty` 36 %, `refund` (the price back) 24 %,
`double` 5 %, `jackpot` 1 %, `explode` 12 %, `holocron` 10 %, `shards` 6 %, `again` (a
free roll) 5 %, `blade` 1 %, paying back 39 of the 100 credits (Sol,
29/10/2026 in the evening: 79 before, which a player rolling a hundred times barely felt;
double was 15%, jackpot 5%, empty 29% and explode 5%). A holocron came one time in fifty
before 29/10/2026, with empty 43% and refund 30%. The last three came the same day (Sol: "loot you
can land on"), their odds taken from empty (39% before) and refund (26%). A
newer hub's outcome the client does not know is shown by its credits alone.

**It blows up** (`explode`, Sol's idea of 28/10/2026, after Call of Duty zombies' boxes):
the roll stops on a teddy bear (since 30/10/2026; a thermal detonator before; SJK's own,
bundled, made by [teddy_bear_model.py](../scripts/teddy_bear_model.py)), and the box
laughs, "a ghost having fun" (`LAUGH`, Sol's laugh of 29/10/2026,
`sound/sjk/mystery_box/laugh.mp3`, in the optional SJK extras pack; a player without the
pack hears the retail gloat, `sound/chars/alora/misc/gloat1.mp3`). 1.5 seconds later, with
a detpack's explosion (`detpack/explosion`, `sound/weapons/detpack/explode.mp3`) where
the opener stands, it kills them: their own client sends the game's `kill`, which the
server carries out, so every player sees them die (Sol, 30/10/2026; before, their client
threw them down as a [ragdoll](client.md#ragdoll) and they got up). Not in a duel, with
the flag, or wherever the Ragdoll could not strike them (`Ragdoll::strikable`). The
laugh plays at the box for every SJK player who sees the roll (before 30/10/2026, to
every hunter wherever they were, at the opening). The hub hands what they paid (nothing
for a free roll) to another player on the server with the Box hunt toy on, at random, one
in play when there is one (since 03/11/2026, hub reports #189 and #204: before, a hunter
who had just left or sat in a menu could get it unseen, and the credits looked lost);
with nobody else hunting there, it is lost. The opener reads where it went as it blows
up; the one who got the credits reads it in the centre of the screen and the other
hunters in the chat once the roll stops on the bear (the hub's box hunt event
`laugh_<opener>_<to>_<credits>`, which arrives at the opening, held for the roll).

**The teddy bear brings a DOOM coin** (Sol, 11/11/2026): the opener of a box that blows up
gets a DOOM coin besides, the only way to spin a [WHEEL OF DOOM](wheel-of-doom.md); the
centre says "+1 DOOM coin".

**A holocron** (`holocron`, 28/10/2026): the roll stops on the tier's holocron, and the hub
gives one of any tier, rolled on the holocrons' own odds ([holocrons.md](holocrons.md));
it counts against no daily cap and cannot be traded (source `box`). The opener reads its
tier over the box and in the centre, and finds it in the Collection; since 11/11/2026 the
other players see the tier's own holocron too (the hub's event `open_<box>_holocron_<tier>`).

**A kyber crystal** (`kyber`, 11/11/2026, very rare): the hub gives a crystal of a tier
rolled on the box's own odds (source `box`), opened in the Collection as any crystal
([holocrons.md](holocrons.md#kyber-crystals)); everyone sees the tier's crystal over the
box (`open_<box>_kyber_<tier>`), with the jackpot's fanfare.

**The Ray Gun** (`raygun`, Sol's idea of 11/11/2026, after Call of Duty zombies' box): the
roll stops on a ray gun of SJK's own (a simple retro one: a steel barrel, red fins, a
glowing green tube), which jumps into the opener's hand at once (no Use needed) for 30
seconds. It is a [Prop gun](client.md#prop-gun) the client bundles
(`sjk/raygun.gun`, never chosen with `propgun`): held instead of the real weapon, in
first and third person, whether the Prop gun toy is on or not, attack firing a green bolt
every 0.45 seconds that every SJK player on the server sees through the toy relay. A
player a bolt strikes falls as a ragdoll, any SJK player in play (not typing, away, dead,
in a duel or with the flag; the Prop gun toy need not be on), once: a green shell, the
Monitor's Screen Saver's in green, wraps them for 30 seconds, and every Ray Gun's bolt
passes through them meanwhile ("ZAPPED!"). Their own client decides the hit, and tells
the hub (`POST /v1/raygun/hit`), which pays the shooter 25 credits when the struck player
has the Box hunt toy on (at most 8 players a gun) and shows the shell to everyone (its
`raygun` event `hit_<victim>_<credits>`); the shooter reads "RAY GUN: <name> down, +25
credits". Players out of the box hunt fall but pay nothing.

**Shards, a free roll, the Mystery blade** (`shards`, `again`, `blade`, 29/10/2026): a
pile of 10 to 25 kyber shards to spend in the shop; a free roll, spent on the next box
the player opens like one won in the box hunt (the hub's answer says the free rolls left,
`free`, and the client keeps that count; no box gives one since 11/11/2026, Sol: "it's
just waste of time", but the box hunt still does); or the boxes' own saber shader, the Mystery
blade (`saber_mystery`, blade 169, Legendary), which no holocron drops, the shop never
sells and nobody trades. A player who holds it already gets 60 shards instead, what
shattering a Legendary shader gives. The blade fanfares like the jackpot.

## Placing one (SJK's creator)

| Command | Does |
| --- | --- |
| `mysterybox_place [server]` | Place a box where you aim, facing you: on every server running this map file, or with `server` on this one only |
| `mysterybox_remove [id]` | Remove the box nearest you (within 256 units), or the one with that id |
| `mysterybox_list` | The boxes on this map, their ids and distance, and the price (anyone) |

The map editor's Place page (`mapedit_panel`), under Things (09/11/2026; before, a Mystery
box and a Remove box button beside Place model), and the browser map editor's World page
([map-edits.md](map-edits.md#the-browser-editor-creator-only-for-now)) list the boxes and
place one where a click meets the floor (following the editor's scope, All servers or This
server), or remove the chosen one, through the same code (`mystery_box_place_at`, `mysterybox_remove <id>`); the hub has no move. Placing needs the map's BSP checksum (as map edits do, `sv_mapChecksum`): the box
stands in that exact map file only. The hub takes the creator's request only; every
placing and removal goes to the staff log. The client reads the hub's list once registered,
every 10 minutes, and at once after a placing, a removal or a box that is gone.

Some of its sounds are retail (the crate sliding open, the chime, the fanfare, the locked
door, the box hunt's alarm, the gloat); the tune and the take are SJK's own; the opening,
roll, closing and laugh of the extras pack are Sol's.

## Box hunt

Players opt in with the Box hunt toy (`toy_boxhunt`, Collection > Toys, the quick wheel;
look toy `box_hunt`); with it off a player is never the mark and is told nothing of the
hunt. Among the players of a server with it on and in play (at least 3), the hub picks a
mark at random for 3 minutes, one mark at most every 10 minutes (Sol, 29/10/2026: from
one pick to the next; the hunt rests in between, also when the mark is killed or leaves).

**In play** (Sol, 31/10/2026; before, 2 players with the toy on, away or not): every read
of the hunt says whether the player is away: typing in the chat, in a menu, the console
or a dialog, spectating or following someone, at the intermission, or idle two minutes
(what holocrons count as playing, [holocrons.md](holocrons.md), and the chat besides). A
change is read at once (2 seconds apart at least). The hub picks nobody away, and needs 3
hunters in play; a hunter whose reads stopped for a minute counts as away. A mark away for
a minute since being picked hands it on: another hunter in play becomes the mark at once,
for a whole 3 minutes, told as any new mark is, when 3 others are in play; else they stay
the mark.

The mark is told in the centre of the screen ("You are the mark!"),
the other hunters by a chat line, and they see "MARK" in red over the mark's head
(hidden behind walls) and the mark's body ringed in red where they see it, the team
outlines' enemy ring (`hud::team_outline`, 29/10/2026), in every game type and whatever
`cg_teamOutline` says. While the game server sends the mark (they are in this player's
snapshot), a red arrow points at them through walls (Sol, 31/10/2026): over their head,
under the label, when they are in view, else on a ring round the crosshair pointing
their way. Every hunter, the mark too, hears the game's alarm
(`sound/effects/mpalarm.wav`, Sol's pick, 28/10/2026; with "Mystery box sounds") as a new
mark is picked, and a small card at the top centre of the screen names the mark (in
the headline font, "BOX HUNT · THE MARK" over the name) or, red, tells the player "YOU
ARE THE MARK"; its words take the menus' lifted colours (`text::CodePalette::Legible`), so
a black name reads on the dark card, and their shadow is the name without its colour
codes; its red edge flashes for 2 seconds as it arrives, and it stays until the
mark is hunted or the hub has no mark or another one. The card and the alarm are
[hunt.rs](../crates/sjk-viewer/src/mystery_box/hunt.rs) and
[label.rs](../crates/sjk-viewer/src/mystery_box/label.rs); the card draws with the words
in the world, so it is hidden with them (menus, console, scoreboard, intermission). A hunter who kills the mark wins a free mystery box roll: the
next box they open costs nothing (the centre says "press Use for your free roll" at a
box). The hunt then rests until 10 minutes after that mark was picked (a minute at
least). A hunter who wins nothing (they hold 3 unspent or won 5 that day) reads why in
the centre too. The day begins at 09:00 in Paris, as the holocrons' caps' (since
03/11/2026, hub report #228: a rolling 24 hours counted the evening before against
"today", so a player stopped after 3 or 4 kills one morning) (31/10/2026; before, in the chat only, so it looked like a kill that won
nothing).

**One life** (Sol, 01/11/2026): the hunters of a mark are its roster: those in play when
it is picked, and any who join in play while it lasts. Each has one life. A hunter the
mark kills is out of that hunt: their own client tells the hub (`POST /v1/box-hunt/out`,
`dead`, from the obituary: only a kill by the mark counts, not a fall, `/kill` or another
player, so the mark has to earn it), the mark no longer shows for them (no label, ring or
arrow), the card says "YOU'RE OUT", and killing the mark after they respawn wins nothing.
When every hunter still there is out, one at least killed by the mark, the mark wins 100
credits per hunter counted (ledger reason `box_hunt`, note `survived`; History says "Box
hunt: you outlasted every hunter"), at most 3 times a day (from 09:00 in Paris; after that, nothing),
and the hunt rests as after a kill. A hunter who left the server, took the toy off or was
away a minute stands in nobody's way and is not counted. The card counts the hunters
left ("2 OF 3 HUNTERS LEFT", from the hub's read). Hunters out only for cheating hand the
mark nothing: one must have been killed.

**Cheating** (Sol, 01/11/2026): a player in a hunt, mark or hunter, who teleports
themselves or flies in noclip is out of it at once. The hub sees neither, so the player's
own client watches its own player state and its own commands
([hunt_watch.rs](../crates/sjk-viewer/src/mystery_box/hunt_watch.rs)). Only the player's
own doing counts (since 03/11/2026, hub reports #235 and #237: a mark an admin brought to
them blew up, and map teleporters took hunters out): the game's `EF_TELEPORT_BIT` flipping
while they stay alive is a cheat only within 5 seconds of a command of theirs that moves
them (the `OWN_MOVES` table: JA+'s `amtele` in the forms that move the sender, which
`japlus.guntele`, `japlus.guntelewall`, `japlus.goto` and `japlus.teleoffset` send, and
`setviewpos`, which `japlus.recall` sends; 5 seconds for JA+'s Star Trek teleport, which
beams the player about 2 seconds late). The client keeps the last 16 reliable commands it
released to the server with when each left (`sjk_client::sent_commands`). `PM_NOCLIP`
counts after the player's own `noclip` on that map, and SJK's own fake noclip
(`fakenoclip`, the free camera) always. So does JA+'s `amghost` on the player themselves, sent
during the chase (hub report #240; a ghost shows in nothing the client receives, so a player
ghosted before the chase began is not caught). The map's teleporters, a spawn after dying,
spectating or following someone, and an admin moving the player (`amtele <player> ...`, a
bring) are not cheats; nor is the player's own `amtele <slot> ...` moving someone else. A
hunter caught is out as one killed is (`cheat`; the others read "is out for cheating").
Out of a hunt (resting, no mark), teleports are free. Before 03/11/2026 any flip of the bit
counted unless the player landed within 128 units of a point the map names for its
teleporters or a spawn point, which missed teleporters a server adds and every admin
teleport.

**The mark blows up** (Sol, 02/11/2026): a mark caught cheating, or typing in the chat
for 30 seconds straight (warned "Leave the chat!" at 20; their client watches it and says
`chat`), blows up and the hunt ends. A hunter still in the chase picked at random by the
hub (another hunter there when none is) wins the free mystery box roll killing the mark
would have, within the same limits (3 unspent, 5 a day); the mark loses no credits.
Everyone taking part
sees the box's explosion where the mark stands (when their game server sends the mark)
and hears it; the mark's own client sends the game's `kill`, as a box that blows up on
its opener does. The mark reads "BOOM!" and who gets the roll, the hunter picked "Free
mystery box roll" (or why not), the others a chat line. The card telling a player they
are the mark says so. Other away (a menu, the console, idle) still hands the mark on
after a minute, with nothing lost.

**The mark sees the hunters** (Sol, 02/11/2026): the mark's read of the hunt carries the
slots of the hunters still in the chase (`chasers`), and the mark's client rings them in
red as the hunters' clients ring the mark ([team_outline.rs](../crates/sjk-viewer/src/hud/team_outline.rs)).
A hunter out (the hub's `out_` event) loses the ring at once; one who joins the chase
gets it by the mark's next read (20 seconds at most). No label or arrow over them.

The hub never sees kills, so the mark's own client says who killed it
(`POST /v1/box-hunt/killed`, from the game's obituary): a hunter cannot claim a kill it
did not make, only the mark could fail to tell. It tells it once its player is the mark
by the hub's event, before its next read (31/10/2026; a kill in those first seconds went
untold), and tries twice. The hub counts a kill only before the mark's end, so a client
stops showing the mark a second before it, by the seconds left the hub's read says
(`left`; by the end time and its own clock for an older hub), and reads again then: it
showed the mark up to 20 seconds after their end before 31/10/2026, and a hunter who
killed them then won nothing (a player's report). A hunter keeps at most 3 free rolls and
wins at most 5 a day. The client reads the hunt (`POST /v1/box-hunt`) every 20 seconds
while the toy is on or a box stands on the map, and at once after the hub's toy events
(`box_hunt`: `mark_<slot>`, `hunted_<slot>_<won>`, and since 01/11/2026 `out_<slot>_dead`,
`out_<slot>_cheat`, `survived_<slot>_<credits>_<hunters>`, and since 02/11/2026
`blown_<slot>_<cheat|chat>_<to>_<won>` for the mark, `to` 255 for nobody).
All the numbers are the hub's, PROVISIONAL.

## Limits

- The hub cannot tell where a player stands: the reach is the client's.
- An outcome pays credits, blows the opener up or gives a holocron; other effects need a
  client release for each new kind.
- The Ray Gun's hits are decided by the struck player's own client: a modified client
  could refuse to fall, and two friends could pay each other 200 credits a gun (8 hits of
  25). Others see the shell when the hub's feed reaches them, a few seconds late.
- The explosion is SJK's only: no game server knows of it, so it hurts nobody else, and
  players on other clients see the opener die (the server's `kill`) with no box or
  explosion. A server that refuses `kill`, or counts it against the score, does so.
- Three friends with the toy on could take turns being killed for free rolls; the
  3-minute mark, the pause and the 5 a day cap how much that pays. The same friends could
  let the mark kill them for its credits: only the mark's kills count, and 3 wins a day
  cap it (at most 300 x hunters a day). A mark blowing up on purpose hands a friend a
  free roll, as letting them kill it would: the same 3 unspent and 5 a day cap it.
- The chat watch is the mark's own client, as the cheat watch is: a modified client could
  keep quiet.
- Teleports and noclip are seen by the player's own client: a modified client could keep
  quiet. A teleport during a long lag spike can be missed (two flips of the bit between
  snapshots). A map teleporter taken within 5 seconds of the player's own teleport command
  counts as a cheat, and a teleport command a mod names otherwise than `OWN_MOVES` does not
  (one more entry each).
- Away is what the player's own client says: a modified client could say it is in play.
  The client of 31/10/2026 sends `away`, which a hub before that day refuses (its read
  takes no other field): the hub goes first.
- The arrow shows the mark only while the game server sends them, which it does by what
  the player could see (the PVS), not across the whole map.
- Verified: unit tests (the pick of the faced box, which boxes stand on a map file and
  server, the roll slowing to its prize, the stop, the take and the take after 10
  seconds, the lid, the words over a box, the hunt's words and reads, the result lines,
  the mark by the event before a read, the cheat watch (the player's own teleport, not the map's or an admin's, the `amtele` forms, noclip asked or forced, fake noclip, a spawn, a new map), the out and outlasted words and card, the blown words, the chat warning and limit, the hunters ringed for the mark, the mark's end by the hub's seconds left or its
  clock; on the hub, the mark picked among hunters in play only and 3 of them, the away
  mark handed on after a minute (or kept with too few others), the kill counted only
  from the mark and for a hunter still in the chase, a hunter out by the mark's kill or a cheat, the mark's win once all are out (not for cheaters alone, nor past 3 a day, nor held up by one who left), a hunter's 5 wins a day counted from 09:00 in Paris, a box's price going to a hunter in play first, the mark's cheat or chat blowing it up and a hunter in the chase winning a free roll (within the limits), the mark alone reading its hunters, the pause, the free roll spent; on the hub, the odds, placing and
  removing as the creator only, the price, the roll, the cooldown and another server's
  box). Not tried in a game; the hub side is deployed (27/10/2026).
