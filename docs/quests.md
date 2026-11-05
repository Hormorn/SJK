# Quests

Quests (since 04/11/2026, wonner67's idea in #ideas): now and then the SJK hub gives one
player taking part a quest, with a minute to finish it, and a finished quest wins an
Echo (a holocron; Echoes are holocrons' JoF name, [terms](mods.md#honors-and-echoes)). The hub decides who gets a quest and checks it, as it does for the
[box hunt](mystery-boxes.md#box-hunt); every number is the hub's and provisional (Sol's
to tune). Code: `crates/sjk-viewer/src/quests.rs` (the toy, the watch, the words),
`crates/sjk-identity/src/quests.rs` (the hub's answers and events), the hub's
`PROTOCOL.md` "Quests".

## Taking part

The Quests toy (`toy_quests`: Collection > Toys, "Take quests"; the quick wheel's Toys
group, to add from Add a choice; Key bindings > Interaction "Toy: Quests"; saved in
`cl_toyQuests`) is the consent: only players with it on are given a quest, named in
one, or told of them. It travels in the look as toy `quests`, which is what the hub
reads. A read every 20 seconds says whether the player is away (typing in the chat, a
menu or the console, spectating, idle), as the box hunt's does, so an away player is
neither given a quest nor named in one. Quests run only on the box servers.

## The social quest

The first kind: hug (`amhug`) or kiss (`amkiss`) each of two other players taking part,
named by the hub, within 60 seconds. With 3 players at least in play, one quest at most
every 15 minutes on a server, and only to a player who may still win one that day: 3
quests a day from 09:00 in Paris, and the day's Echoes not all found (8 a day by
playing, [holocrons.md](holocrons.md)). A quest names only players its player has not
finished one with that day, so a few friends cannot farm each other (Slash,
05/11/2026); with too few of those, another player is given it, or nobody.

- The player given it sees it in the centre of the screen ("Quest! hug A and B within a
  minute, face them and use /amhug"); each target is told to stay close; everyone else
  taking part reads it in the chat.
- **Finding each other** (05/11/2026): a minute is short on a big map, so the quest's
  player sees, over each target still to do, the emote's name in gold ("HUG", "Quest:
  /amhug") and a gold arrow pointing at them through walls, and each target sees the same
  over the quest's player ("QUEST", "They must hug you") until they are done. These are
  the box hunt's label and arrow (`mystery_box/label.rs`): the label hidden behind walls
  and far away, the arrow over the player when in view, else on a ring round the
  crosshair pointing their way, while the game server sends that player.
- **The card** (05/11/2026): while the quest lasts, the player given it and its targets
  see a card at the top centre (under the box hunt's when both show), gold edged, its edge
  pulsing as it arrives: "QUEST · 42s LEFT" counting down by this client's clock, over
  what is left ("Hug Nib (done) and Sanji"; for a target "Al must hug you", then "Al
  hugged you"). The card hides with the world's words (menus, console, scoreboard,
  intermission).
- **Proof.** The hub never sees an emote. A target's client watches its own player
  state from the game server (never the prediction): the frame its player goes into the
  received half of the quest's emote (`BOTH_HUGGEE1` for a hug, `BOTH_KISSEE` for a
  kiss, both set by JA+ on the player hugged or kissed) with the quest's player within
  160 units of it, it tells the hub (`POST /v1/quests/done`, the quest's player's slot).
  Once per target and quest; a spectator or a player following another never reports.
  So nobody finishes a quest alone, and the player given it cannot claim it.
- Once every target has said so the hub drops the player an Echo as one earned by
  playing (it counts against the day's caps, per key and per address, so quests add no
  Echo to the day, they bring one sooner; History shows it noted `quest`), tells
  everyone taking part, and the player reads "Quest done! You found an Uncommon Echo" (or
  "No more Echoes today" past a cap); a quest that runs out is told as failed. Until
  05/11/2026 (never released) it paid 150 credits, which would have outweighed hours of
  play and been farmed among friends.

The emotes are JA+'s, so a social quest can be finished only on a JA+ server (JoF's
servers are). Anti-kiss (`japlus.antiAmKiss`) pushes a kisser away, but the kiss has
already started and counts.

## Not yet

A sound as a quest is given (each toy's sounds have a switch of their own in Settings >
Sound > Toys, which Quests does not have yet), more kinds (an emote before a player, a duel
against one), and quests for weekend events. A hunt is not a kind of its
own: the [box hunt](mystery-boxes.md#box-hunt) is one.
