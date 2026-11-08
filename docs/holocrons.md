# Holocron drops

A holocron is loot a player earns by playing SJK. The SJK hub rolls and stores them and
the client shows them. Since 12/10/2026 a holocron can be opened: it gives a sealed card
and credits ([cards.md](cards.md)), and nothing else (no power or setting). This page is the design, the decisions and what
is checked; the wire is the hub's `PROTOCOL.md` ("Holocrons") in Sol-Vulpes/SJK-hub,
decided together with the client and built in parallel. Players read the pop-up, the chat
lines and the Collection's Holocrons tab.

With the JoF tools on, holocrons are called "Echoes" and medals "Honors" by default
([mods.md](mods.md#honors-and-echoes), `jof.honorNames`).

## Pieces

| Piece | Where |
| --- | --- |
| Wire types, the `active` claim flag, `GET /v1/holocrons`, drop events, staff requests | [wire.rs](../crates/sjk-identity/src/wire.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [staff.rs](../crates/sjk-identity/src/staff.rs) |
| Progress in the snapshot, `Service::set_active`, the worker's early profile reading | [service.rs](../crates/sjk-identity/src/service.rs) |
| Drops joined to the chat's messages, told to the worker when they are the player's own | [feed.rs](../crates/sjk-identity/src/feed.rs) |
| The one client list of tiers, the player's counts, recent holocrons, progress | [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs) |
| What counts as actively playing, the input clock | [holocrons/activity.rs](../crates/sjk-viewer/src/holocrons/activity.rs), [holocrons_frame.rs](../crates/sjk-viewer/src/holocrons_frame.rs), input hooks in [viewer_app.rs](../crates/sjk-viewer/src/viewer_app.rs) |
| The drop pop-up, its ceremony and look | [holocron_popup.rs](../crates/sjk-viewer/src/holocron_popup.rs), [holocron_popup/view.rs](../crates/sjk-viewer/src/holocron_popup/view.rs) |
| `holocrons_seen.txt` | [holocrons/seen.rs](../crates/sjk-viewer/src/holocrons/seen.rs) |
| The chat lines' words and the tier gem | [holocrons/line.rs](../crates/sjk-viewer/src/holocrons/line.rs), [holocrons/gem.rs](../crates/sjk-viewer/src/holocrons/gem.rs), [sjk_chat_look.rs](../crates/sjk-viewer/src/sjk_chat_look.rs) |
| The tiers' pictures in the UI atlas | [holocrons/icons.rs](../crates/sjk-viewer/src/holocrons/icons.rs), [ui_renderer/icons.rs](../crates/sjk-viewer/src/ui_renderer/icons.rs), [hud/icons.rs](../crates/sjk-viewer/src/hud/icons.rs) |
| The Holocrons tab (the page, its drawing, the console side) | [holocrons_panel.rs](../crates/sjk-viewer/src/holocrons_panel.rs), [holocrons_panel_view.rs](../crates/sjk-viewer/src/holocrons_panel_view.rs), [console_holocrons_page.rs](../crates/sjk-viewer/src/console_holocrons_page.rs) |
| The loot table (the page, the odds worked out) | [holocrons_panel_loot.rs](../crates/sjk-viewer/src/holocrons_panel_loot.rs), [cards/loot.rs](../crates/sjk-viewer/src/cards/loot.rs) |
| The tab's 3D holocron, the camera's shot behind it | [holocrons/stage.rs](../crates/sjk-viewer/src/holocrons/stage.rs), [menu_backdrop/routes.rs](../crates/sjk-viewer/src/menu_backdrop/routes.rs) |
| `debug_holocron` | [holocrons/rehearsal.rs](../crates/sjk-viewer/src/holocrons/rehearsal.rs) |
| Kyber crystals: their tiers, odds, pity and achievement rewards | [kyber.rs](../crates/sjk-viewer/src/kyber.rs) |
| Give holocron and Remove on the Staff page | [staff_panel.rs](../crates/sjk-viewer/src/staff_panel.rs), [staff_panel_view.rs](../crates/sjk-viewer/src/staff_panel_view.rs) |
| The hub's rolls, caps and storage | repository Sol-Vulpes/SJK-hub (`PROTOCOL.md`, `src/holocrons.rs`) |

## Tiers

Four tiers, the same ids, names and order in the hub's `src/holocrons.rs` and in the
client's [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs); a test on each side pins
the list. A client shows only the tiers it knows.

| Id | Name | Colour | Odds |
| --- | --- | --- | --- |
| `uncommon` | Uncommon Holocron | green `#2FC77A` | 60% (600 per mille) |
| `rare` | Rare Holocron | blue `#2E7BFF` | 28% (280) |
| `legendary` | Legendary Holocron | purple `#A64DFF` | 10.5% (105) |
| `mythical` | Mythical Holocron | gold `#FFC933` | 1.5% (15) |

The colours are none of the game's `^n` codes, in the game's palette or the SJK UI's
lifted one (at least 0.28 away in RGB, tested), and the four are at least 0.4 apart from
one another for players who tell hues apart poorly. Mythical gold is near the SJK chat's
own gold (`#F5C756`, 0.14 away) because both are gold; a drop line stands apart from a
message by its gem, its wording and having no sender column. Each tier's picture is
`gfx/sjk/holocron_<tier>.png`, bundled with the client and mounted below all game data
(the list in [illuminate.rs](../crates/sjk-viewer/src/illuminate.rs)); where it is missing
the tier draws as a gem made of shapes ([holocrons/gem.rs](../crates/sjk-viewer/src/holocrons/gem.rs)).
Since 17/10/2026 the picture is the tier's own holocron (below), drawn from its parts.

### The four holocrons

Since 17/10/2026 each tier is an object of its own (Sol's pick from the proposals' second
set), not one cube in four colours:

| Tier | Holocron | What moves |
| --- | --- | --- |
| Uncommon | The Jedi cube (Sol's sculpted model, 08/11/2026): a steel-framed cube with riveted corners, glowing green glass on each face round a raised emblem plate, baked onto a 300-triangle low poly with a normal map | nothing of its own |
| Rare | The Great Holocron: a dodecahedron, a raised pentagon in each face with the Jedi Order's emblem and runes, lit edges | a broken halo, tilted, turning and bobbing around it |
| Legendary | The Sith pyramid (Sol's sculpted model, 05/11/2026): a dark frustum on a carved plinth, glowing purple runes and lines down its sides, baked from a high-poly model onto a 58-triangle low poly with a normal map | its cap floats over it, turning and bobbing; a glowing eye turns between them |
| Mythical | The engram (21/10/2026; until then the kyber prism, now the [Kyber crystal](#kyber-crystals); Sol's sculpted model since 08/11/2026): a lumpy gem of 38 engraved triangular gold plates, each its own slab (its face baked from the high poly with a normal map) with glowing seams between them and a white-hot core inside | the shell turns about its length while it slowly tumbles over; the plates breathe out and back (up to 0.4 units, the core showing between them at the top of the breath); the seams and core pulse; streaks of reflection sweep the plates as they turn; eight sparks circle it |

They are made by `scripts/holocron_assets.py` with the shapes in `scripts/holocron_forms.py`:
a model per part (the body, which stands still against the holocron, and each moving
part, modelled about its own centre), each also in the locked look, and the parts' table
`holocron_parts.rs` (where each part rests and how it bobs, orbits and turns), which the
client includes ([holocrons/stage.rs](../crates/sjk-viewer/src/holocrons/stage.rs),
`FORMS`), so the two never disagree; the script also writes the list of bundled files the
client mounts (`holocron_files.rs`). The Jedi cube's face is the shipped face in the tier's
colour; the Great Holocron's pentagon and the kyber crystal come from Sol's pictures
(17/10/2026) in the asset folder's `art/`, fitted and painted in the tier's colours by the
script, and the Jedi cube, the Sith pyramid and the engram are Sol's sculpted models there
(`art/jedi_cube/`, `art/sith_pyramid/`, `art/gold_engram/`: each a low poly with its
baked colour, glow and normal maps)
(`--art <key>=<picture>` tries another; the folder's
[README](../crates/sjk-viewer/assets/holocron/README.md)). The four are sized to fill
about the same room on screen.

Until 08/11/2026 the Jedi cube was drawn by the script (a plateau on each face with the
shipped face on it, a lamp over each) and the engram was thirty plates drawn by the script.
The sculpted engram keeps that engram's life: Sol's model is split into its tiles (the
faces of the high poly near its convex hull, cut apart by the grooves), each tile a flat
plate of its outline, grown a little, baked from the high poly into one picture
(`art/gold_engram/`, an object per plate). The script makes each plate a slab with walls
and an inner face in the seams' glow, every vertex of a plate carrying the plate's normal,
so the shader's `deformVertexes wave` moves the whole slab out along it as one. A soft halo
sprite was tried on the old engram (`deformVertexes autosprite`), but on the stage a
surface whose only stage is added light did not show, so the halo comes from the glow pass
around the seams and core instead.

### Kyber crystals

Since 21/10/2026 the client also has the kyber crystals' models (the crystal logic and UI
are their own work): the kyber prism that was the Mythical holocron's form, in each tier's
colours and light, with the hub's tier ids. The commoner the plainer:

| Id | Colour | What it has |
| --- | --- | --- |
| `kyber_uncommon` | green `#2FC77A` | the crystal in its steel sheath, one gimbal ring with four lamps, four motes |
| `kyber_rare` | blue `#2E7BFF` | the same, six motes |
| `kyber_legendary` | purple `#A64DFF` | a second gimbal ring, eight motes |
| `kyber_mythical` | gold `#FFC933` | both rings, ten motes (the old Mythical holocron's full look) |

The crystal turns, the rings tumble, the motes circle; each has its locked look and its
icons (`gfx/sjk/holocron_kyber_<tier>.png` and `_small.png`, 256 and 64 pixels). The 3D
stage draws one by the hub's id: `holocrons::stage::Request::of("kyber_rare", owned)` (or
`Request { tier: slot, owned }`), the same way and in the same place as a holocron;
`stage::FORMS` lists every form it can draw by slot: 0-3 the holocron tiers, 4-7 the kyber
crystals (uncommon, rare, legendary, mythical), each with its light. `holocrons::icons`
uploads the crystals' icons into the slots' cells (`icons::texture(slot)`) and marks them
for `icons::is_loaded(slot)`.

The Holocrons tab's tier rows carry the tiers' rarity effects (10/10/2026): from Rare up
a breathing frame, Legendary's sheen, Mythical's running spark and motes, faint for a
tier not held ([unlockables.md](unlockables.md#rarity-effects), off with
`ui_rarityEffects 0`).

## Kyber crystals

Since 21/10/2026 (Sol's decisions; client branch `personal/kyber-crystals`, the hub's in
parallel, neither deployed): a loot of its own beside the holocrons, which stay exactly as
they are. A **kyber crystal** comes only from an achievement, one per achievement and once
per person, and opening one always gives a saber shader. Four tiers, the holocrons' names
with "Kyber Crystal" (`kyber_uncommon` "Uncommon Kyber Crystal" to `kyber_mythical`), in
the holocron tier's colour and light; every tier keeps a small chance of a Mythical shader.
The client's list is [kyber.rs](../crates/sjk-viewer/src/kyber.rs) (`kyber::TIERS`, reusing
`holocrons::Tier`, `Tier::is_kyber`).

- **At the hub** a crystal is a row of the holocrons table with a crystal tier, source
  `achievement` and an empty note: listed, opened (`POST /v1/holocrons/open`) and given or
  removed by staff as holocrons are. It is never dropped by play, never sold, never in the
  caps or the holocron drop odds. The client keeps it out of everything about holocrons
  found by play: `Tier::from_id` and `holocrons::entries` know holocrons only (the player
  card, the counts, the Collection row), the tab's "16 holocrons", "x% of drops" and recent
  list leave crystals out. `holocrons::any` and `Entry::from_wire` know both, for the tab,
  the arrival and the chat. Crystal counts are counted from the profile's list (the hub
  lists sealed rows first), not from `holocron_counts`.
- **Odds** come with `GET /v1/holocrons` as a top-level `kyber` object (not in `loot`, so
  older clients' loot parsing is untouched): `tiers`, each crystal tier's `shader_tier` per
  mille over common to mythical, and `pity` (`every`, `since`). `kyber::BUILT_IN` is the
  client's copy, shown before the hub's first answer and for older hubs, pinned by a test;
  the hub's replace it only when they fit (the four tiers in order, five shares summing to
  a thousand). Provisional, Sol's to tune:

  | Crystal | Common | Uncommon | Rare | Legendary | Mythical |
  | --- | --- | --- | --- | --- | --- |
  | Uncommon | 45% | 40% | 12% | 2.5% | 0.5% |
  | Rare | 20% | 40% | 30% | 8.5% | 1.5% |
  | Legendary | 0% | 25% | 45% | 25% | 5% |
  | Mythical | 0% | 0% | 30% | 50% | 20% |

- **Pity**, shown to the player: `since` is the person's crystals opened since the last
  that gave Legendary or better; the `every`-th (10) in a row without one is forced to
  Legendary or Mythical. The tab says "Legendary or better guaranteed within 7 crystals"
  (`kyber::pity_line`; "Your next crystal is Legendary or better, guaranteed" at the last;
  "at least every 10 crystals" before the hub said). Opening a crystal makes the identity
  service read the progress again at once, so the line follows.
- **The shader** is picked by the hub: the tier rolled from the crystal's odds (pity may
  force it), then among the droppable shaders of that tier the ones the person does not
  hold first (holding them all, any, and the copy stacks), a tier with none falling back as
  a holocron's shader does. Medal shaders never come out.
- **Rewards** (`kyber::REWARDS`, pinned against `achievements::ALL`): Uncommon for
  `first_blood`, `streak_5`, `ledge_10`, `duel_wins_10`, `captures_10`, `maps_10`,
  `servers_5`, `hours_10`, `storyteller`, `bug_reporter`, `holocrons_10`, `craft_blade`,
  `order66_kill`, `box_minutes_10`, `duel_close_call`, `saber_throw_far`, `fish_first`; Rare for
  `kills_100`, `saber_kills_100`, `streak_10`, `dark_side_25`, `arsenal`, `maps_25`,
  `surveyor` (retired 26/10/2026: its holders keep their crystal), `decorated`,
  `holocrons_50`, `mythical_find`, `craft_finishes`, `duel_flawless`, `duel_partners_25`,
  `thermal_return`, the five style masters (`style_blue_50`, `style_yellow_50`,
  `style_red_50`, `style_dual_50`, `style_staff_50`), the four Fishing ones (`fish_50`,
  `fish_kinds`, `echo_angler`, `old_whiskers`); Legendary for `duel_wins_100`, `hours_100`, `holocrons_200`,
  `full_set`; Mythical for `kills_1000`. A hub
  that sends `reward` on an achievement replaces the client's word for it
  (`kyber::follow_rewards`, lock-free, read by the boards every frame). The hub grants a
  crystal when the achievement unlocks and backfills those already unlocked; clearing an
  achievement takes its crystal back while unopened. Crystals cannot be traded, sold,
  bought or broken down; grants are not announced, grand finds from opening one are.
- **In the client**: the Holocrons tab's row of crystal tiles, their hover card and loot
  table ([below](#the-holocrons-tab)); the arrival of a granted crystal as a gift's
  ([the pop-up](#the-pop-up)); the chat's grand find line ([chat lines](#chat-lines)); the
  achievement's reward on the Achievements tab, its hover card and its unlock card
  ([identity.md](identity.md#achievements)); `debug_holocron` and `debug_open` take the
  crystal tiers.
- **The 3D crystal**: the tab draws a crystal by its slot (`Tier::slot`, 4 to 7 after the
  holocrons' 0 to 3) through the stage's `Request { tier, owned }`. Until the crystal models
  come (the branch `personal/engram-holocron` makes them, and the Mythical holocron's new
  engram, from the kyber prism the Mythical holocron wears today), the stage keeps any slot
  past the holocrons to the Mythical holocron's parts and light, so a crystal shows as
  today's kyber prism in gold; its tiles and arrival draw the tier's gem, the icons' bits 4
  to 7 being unset. Not built: the reveal seating the crystal in a hilt and igniting the
  blade (the shader's entrance is the holocron one's).

## Loot table

Every chance a holocron brings, as players see it on the Holocrons tab's loot table
(12/10/2026, Sol's request; Loot table or L). The client works it out from the hub's
tables, which `GET /v1/holocrons` sends under `loot` (14/10/2026, [`loot_odds.rs`](../crates/sjk-viewer/src/loot_odds.rs)):
the tiers' odds, the card's odds and credits ([cards.md](cards.md#odds)) and the shader
odds. Until the hub's first answer, or from a hub older than them, it shows its own copy
(`loot_odds::BUILT_IN`); tables that do not fit what the client can show (other tiers,
rarities or finishes, shares not summing to a thousand) are ignored the same way. Tier
odds read elsewhere ("10.5% of drops") come from the same tables. A card's rarity and its finish are rolled apart, so a kind is the two chances
multiplied; "per holocron found" weighs each tier by how often it drops. Since 14/10/2026 an
opening may bring a saber shader instead of the card and its credits (30% of Uncommon
holocrons to 50% of Mythical ones, [unlockables.md](unlockables.md)): the card columns of a
tier are the card's odds when it holds one, the "any holocron" shares are of the cards
found, and the credits' averages and the second table are per holocron found, shaders
counted (`cards::loot::card_in`). Since 17/10/2026 (prepared, not deployed) one opening in
ten may bring a pile of kyber shards instead ([cards.md](cards.md#shards-and-the-shop)): the
tables' `shard_pile` and `shards`, a "Kyber shards" column (the chance and the range, the
average under it, a dash from a hub without piles), and the card's share counts them too.
A card comes with a few shards beside its credits (the card pack, `card_shards`,
[cards.md](cards.md#odds)).
Since 27/10/2026 (the hub's `tag-toy` branch) an opening may bring a tag instead of the card
(`tag` in the tables, 150 per mille in every tier, taken from the cards' share; the tag's
tier rolled as a shader's; [client.md](client.md#tag)); this client's own copy of the
tables keeps no tags, so the figures below are without them. Since 31/10/2026 (the hub's
`tag-cards` branch, Sol's decision: stingy and flat) a dropped tag comes Holo 4 % of the
time and Solar 0.5 %, in every tier (`tag_finish` in the tables, per mille, Holo then
Solar; none from hubs before), said in the opening ("Holo Gonk tag") and kept apart in the
profile; a Solar tag is a grand find, its chat line saying so ("Sol opened a Rare Holocron:
Solar Gonk tag!").
The figures below are with the piles.

Per holocron:

| Holocron | Drop chance | Common | Uncommon | Rare | Legendary | Mythical | Normal | Holo | Solar | Credits (average) | Kyber shards (average) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Uncommon | 60% | 70% | 24% | 5.5% | 0.4% | 0.1% | 97% | 3% | 0% | 10 to 20 (9) | 10%: 5 to 15 (1.0) |
| Rare | 28% | 50% | 33% | 15% | 1.8% | 0.2% | 90% | 10% | 0% | 30 to 50 (20) | 10%: 10 to 25 (1.8) |
| Legendary | 10.5% | 30% | 38% | 26% | 5.5% | 0.5% | 70% | 29% | 1% | 80 to 120 (45) | 10%: 25 to 50 (3.8) |
| Mythical | 1.5% | 10% | 30% | 45% | 13% | 2% | 40% | 55% | 5% | 250 to 350 (120) | 10%: 100 to 200 (15.0) |
| Any holocron found | 100% | 60.9% | 27.5% | 10.1% | 1.33% | 0.18% | 92.3% | 7.56% | 0.14% | 10 to 350 (18) | 10%: 5 to 200 (1.7) |

Every card, per holocron found:

| Card | Normal | Holo | Solar |
| --- | --- | --- | --- |
| Common (60.9%) | 31.8% (1 in 3) | 1.9% (1 in 53) | 0.0172% (1 in 5 822) |
| Uncommon (27.5%) | 13.9% (1 in 7) | 1.34% (1 in 75) | 0.027% (1 in 3 710) |
| Rare (10.1%) | 4.78% (1 in 21) | 0.774% (1 in 129) | 0.0258% (1 in 3 878) |
| Legendary (1.33%) | 0.58% (1 in 173) | 0.148% (1 in 677) | 0.0065% (1 in 15 388) |
| Mythical (0.18%) | 0.0815% (1 in 1 228) | 0.0173% (1 in 5 770) | 0.0008% (1 in 119 581) |

The page: over the darkened tab, the title, what it is and the hub's pace ("One holocron
about every 30 minutes of active play on a server, at most 8 a day (it resets at 9:00, Paris
time)", once the hub has said;
else "Holocrons are found by playing on a server."); "Per holocron", the first table, each
tier's row in its colour with its picture, the row for any holocron found in gold under a
rule, credits as the range with the average under it, a chance of none quiet; "Every card,
per holocron found", the second table as a grid of cells in the rarity's colour, each its
share and "1 in N" (left out above one in two); "How it rolls", notes: the rolls made
apart, credits and piles of shards rolled evenly, the rarest card ("a Mythical Solar:
about 1 in 119 581 holocrons", worked out), how the card is picked among its rarity's (sealed while the rarity has none), gifts given and not found. Back
(or Escape, L, Enter) goes back to the tab. Shares read with three figures at most, halves
rounded up (`cards::loot::percent`). A change to the hub's tables needs the client's copy
changed with it: until then the page shows the old odds.

On a kyber crystal's tile the page is the crystals' table instead ("Loot table: kyber
crystals"): per crystal tier the chance of each saber shader tier, a row of how many
droppable shaders each tier holds, and notes on how the shader is picked, the pity (with
the person's own count) and where crystals come from.

## Earning

- Every claim to the hub carries `"active": true` only when the player is actively
  playing. The client says so when it is in a live, non-local game of someone else's, in
  the player's own view (not spectating or following), mid-match (not at the intermission
  or the scoreboard freeze), out of the menus (the game menu, the console, a dialog, a
  pop-up or the main menu) and had input (a key or mouse button pressed, the wheel, the
  mouse moving) in the last two minutes. A claim without the flag, which an older client
  sends, earns nothing; `active: false` is left out of the body altogether.
- Only SJK earns (hub, 27/10/2026, Sol's rule): the hub reads `active` as false unless the
  claim's User-Agent is an SJK release whose commit matches its `sjk-v` tag, an SJK
  development build, or a version newer than every tag it has read (any `SJK/` version
  before it has read them). Other clients in SJK chat, JoF's EternalJK among them, claim
  their slot for their mark on the scoreboard and earn nothing. The agent is a hint, not
  proof: a client that copies SJK's agent is not stopped by it.
- On each claim renewal with `active: true` the hub adds the time since its last claim of
  the key, at most 60 seconds, to the key's active time, and `last_tick` moves on with every
  claim. The client renews every 45 seconds, so a renewal counts in full and sending claims
  faster counts only real elapsed time. At 1800 seconds the hub drops a holocron and keeps
  the rest. Progress persists in the hub's database across sessions. A hub started with
  `--holocron-every <secs>` drops sooner, for tests; the caps stay.
- **Play signals** (03/11/2026, Rollembones' idea, Sol: "better activity checkers (not
  chat)"): an active claim also carries `"play"`, what the player did since the hub last
  accepted a claim: `moved` (the horizontal distance their own origin travelled while
  alive in their own view, a step over 512 units in one snapshot left out as a teleport
  or respawn), `attacks` (attack and alt attack presses in the commands sent), `hits`
  (rises of the game's hit counter, `PERS_HITS`), `hurt` (snapshots whose health or
  armour fell while the last attacker, `PERS_ATTACKER`, is another client: falls and
  self damage are not counted), `kills` and `deaths` (obituaries against or by another
  client), and since 04/11/2026 `bites` (bites of the fishing toy hooked in time, so a
  player standing at the water earns; headshots' question in #ideas). The counts are kept
  only while the claim says `active`, sent with the next
  one and cleared once the hub accepted it (a failed claim keeps them for the next).
  [holocrons/signals.rs](../crates/sjk-viewer/src/holocrons/signals.rs) reads them from
  each live snapshot and command, with atomics and no allocation. The hub decides the
  pace: full while the claim shows moves or a fight and a fight came in the last 10
  minutes, and the same ("peaceful", 100 % since Sol's call on 03/11/2026; the hub
  sends the share) while moving without a fight in that time, the same while standing
  still within 5 minutes of a hooked bite ("fishing"; a bite comes at a random time and
  a press of use without one scares the fish off, so a held key earns nothing), none
  otherwise standing still, none for counts no one could make in that time (the hub's
  `PROTOCOL.md`). The
  Holocrons tab says when the last claim earned less than full pace ("Half pace: no
  fight in the last 10 minutes. Fight, get hit or duel for full pace", "Not earning:
  move around or fight to earn holocrons", "... move around, fight or fish ..." from a
  hub that counts fishing), from `activity` in `GET /v1/holocrons`.
- **Catch-up** (03/11/2026, Rollembones' idea, Sol: "x2 faster the next day"): when the
  previous day's drops by play fell short of the cap, the hub gives the day a catch-up
  pool, the shortfall, at most 4 (`catchup` in `GET /v1/holocrons`: `pool`, `left`,
  `every_secs`, `max`). While some are left a holocron needs half the play (15 minutes);
  catch-up drops count toward the day's 8, so a day never brings more. The Holocrons tab
  says "Next holocron in about N minutes of play (catch-up x2)" and "Catch-up x2: 3 more
  holocrons at double speed today" under the bar; its bar fills toward the shorter wait.
  Hubs before leave `catchup` and `activity` out and the tab shows neither.
- The tier is rolled by the hub from a system random source.
- Caps, all the hub's, per day: at most 8 drops per key (staff gifts excluded); at most 1
  mythical per key (another becomes legendary); at most 16 per address across all keys.
  Since 29/10/2026 (Sol's decision) the day runs from 09:00 to 09:00 in Paris, summer time
  included, and every count starts again at 09:00; before, the caps looked back a rolling
  24 hours, so each drop freed its room 24 hours after it dropped. A drop a cap holds back
  waits (the active time stays at 1800) and happens at the first renewal after the cap
  frees, so a player who reached the limit and kept playing finds one at their first
  renewal after 09:00; nothing is lost.
- A staff gift (`source: "staff"`) counts toward no cap and is not announced.

The client reads the progress with `GET /v1/holocrons` (`progress_secs`, `every_secs`, the
day's `today` and `daily_cap`, and since 29/10/2026 `resets_at`, when the hub's next day
begins) at the registration, every five minutes, soon after a drop
of its own and when a screen asks ([`holocrons::refresh`](../crates/sjk-viewer/src/holocrons.rs),
at most once every 30 seconds). A hub from before holocrons refuses it (404); it is asked again
in an hour and the identity's status never shows it. Any other failure (unreachable, too
many requests, a restart) is tried again after two minutes (before 29/10/2026 it also
waited an hour, and the Esc menu's reading could not bring it forward). The remaining time is as the hub last
said and does not count down by itself.

## What the hub stores, and privacy

The hub stores each key's active play time (a total carried toward the next holocron) and
the time of its last claim, and every holocron dropped for a key: its tier, time, source
(`play`, `staff`, or `bought` with credits since 17/10/2026, prepared), a staff note and
the address (or a hash of it) it dropped from, for
the per-address cap. The profile lists a key's counts and its recent holocrons publicly,
as it does its medals, and legendary and mythical drops are told to every SJK client in
the feed with the key's hub name. Uncommon and rare ones reach only their owner. Old
clients ignore all of it. [identity.md](identity.md#privacy) says what a default install
sends.

## In the client

### The pop-up

A holocron new to the client shows once, large, on a darkened screen in its tier's colour:
"New holocron" (and "1 of 3"), where it came from (dropped while playing, or a gift from
the SJK team), its name, when it was found (`dd/mm/yyyy HH:MM`, UTC), how rare it is and,
for a gift, the team's note. It follows the medal pop-up
([identity.md](identity.md#medals), [medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs)):

- It opens on the main menu, or when the game menu opens in a match, never over play. A
  holocron that arrives during a match is announced once by a card at the top of the
  screen, the achievements' ([identity.md](identity.md#achievements),
  [unlock_toast.rs](../crates/sjk-viewer/src/unlock_toast.rs); since 10/10/2026, Sol's
  request, in place of a centre print): "New holocron", "Open the game menu to see it", the
  name in the tier's light, "Found while playing: 28% of drops are Rare" (or "A gift from
  the SJK team"; for a kyber crystal "NEW KYBER CRYSTAL" and "Earned from an achievement",
  or "Earned: Centurion" when the row's note names the achievement), the tier's icon (its gem without one) in a ring that sweeps round, and the
  burst, sparks and edge in the tier's colour, with the achievements' chime
  (`cg_achievementSound`). It takes no input and leaves after about six seconds; one
  arrival brings at most three cards (a reinstalled PC finding twenty shows three), and the
  pop-up shows them all. When the pop-up opens it takes back the holocron cards still
  showing or waiting. While it shows, the menu under it is neither drawn nor given input. A
  medal on show or waiting goes first.
- `cl_holocronNotifications` (archived, default 1; Settings > General > Holocron
  notifications, Network > SJK online in the classic views; Lukas, SJK #179) controls
  only the live card during play. 1 (After
  duels) gives it as above; during a private duel it waits, as every achievement, medal
  and holocron card does (hub report #81, `unlock_toast::held_in_duel`), and plays once
  the duel ends. 2 (Always) lets the holocron cards show during a duel too, ahead of the
  other cards held there. 0 (Off) gives none: cards showing or waiting are taken back,
  and those not given yet never are, so turning it back on replays nothing. In every
  mode the holocron itself still waits and its large pop-up opens normally when the
  player opens the game menu.
- A kyber crystal the hub grants arrives the same way, as a gift does: "New kyber crystal",
  "Earned from an achievement" (or "Earned: <achievement>" when the hub names it in the
  row's note, which is then not quoted), its name, when, and "Earned from achievements.
  Always a saber shader." in place of the drop odds; its own picture
  (`gfx/sjk/holocron_kyber_<tier>`, the atlas cells 4 to 7) since 27/10/2026, its gem only
  when that picture is missing (until then the pop-up always drew the gem, which read as a
  plain holocron: report 150).
- Each arrives in a ceremony of the medal's timing and light in the tier's colour (it comes
  down into place, rings and sparks run out, a ring of ticks turns, a glow breathes while it
  waits, the words fade up one group after another) with the multiplayer game's fanfare
  (`music/goodsmall.mp3`, the medals' cue; silent without game data). Enter, Space, Right,
  Escape or a click finish the entrance, then take Next (Close on the last). The look is the
  SJK UI's in its families, Inter until they load, and the same under the classic menus.
- Everything is draw-list shapes of a fixed number: no allocation a frame, and a canvas of
  320 commands holds the busiest moment.
- `holocrons_seen.txt` in the settings folder, beside `identity.key`, holds the key id and
  the highest holocron number shown, written when a holocron's button is taken. A number
  above it is new. With no file for the key (a reinstall, a new PC) everything the profile
  holds is new, and the newest 20 show once, oldest first. A holocron of a tier the client
  does not know is left out and never counted as shown. Holocron numbers only rise at the
  hub, so a staff removal never brings an old one back.
- The identity worker reads the own profile again soon (at most once every 30 seconds) when
  a drop for the player's own key arrives in the feed, and the pop-up sees the new holocron
  within half a second of the profile arriving.
- A contributor reward's holocrons (25/10/2026, [hub-chat.md](hub-chat.md#announcements-and-reward-rounds))
  are gifts: claiming the reward brings the profile that holds them, and they arrive
  through this pop-up as any gift does.

### Chat lines

A drop is one SJK chat line in the tier's colour with a small gem where the verified tick
goes, in all three places the SJK chat shows ([hub-chat.md](hub-chat.md#how-a-line-looks)):

- The game's chat feed and the docked chat: `Sol found a Legendary Holocron!` (the hub
  name without colour codes, "Someone" when it has none), and for the player's own
  `You found an Uncommon Holocron.`; a `!` ends the two grand tiers' lines, a `.` the others.
  There is no sender column, no colon and no tick.
- The chat page: the name in its column, the gem, then `found a Legendary Holocron!`, with
  no colon; a tier the page does not know reads `found a holocron.`. A name with the JoF
  clan's tag keeps its emblem before it there ([identity.md](identity.md#clan-tags)); the
  feed and the dock word the name inside the sentence, so they draw no emblem.
- A muted player's drops are not shown, and a drop of an unknown tier is left out of the
  feed and the dock. A drop is not a message, though it shares the feed's numbers and is
  kept with the chat's last 200.
- Legendary and mythical drops are delivered to every reader and uncommon and rare ones to
  their owner only, by the hub; a fresh reader gets only the drops of the last 60 seconds,
  so a joiner is not replayed history. With `cl_sjkChat 0` no line is kept, but the
  player's own drop still makes the worker read the profile.
- **Grand finds** (since 17/10/2026, with a hub that sends the feed's `finds`): when a
  holocron someone opened gives a Legendary, Mythical or Solar card, or a Legendary or
  Mythical shader, every SJK player gets a line like a drop's, in the holocron's colour with its
  gem: `Sol opened a Mythical Holocron: Phoenix blade!`, `Fox opened an Uncommon
  Holocron: a Legendary card!`, `You opened a Rare Holocron: an Uncommon card, Solar!`
  (the shader's name plain and at most 40 characters, the card's rarity and its finish
  unless plain). A kyber crystal's find reads the same with its name and colour: `Sol
  opened a Rare Kyber Crystal: Phoenix blade!` (the event's tier is the crystal's). A
  crystal's grant has no line. The identity service relays a find as a drop line whose mark carries
  the prize (`DropMark::find`, [feed.rs](../crates/sjk-identity/src/feed.rs)); the
  words are [holocrons/line.rs](../crates/sjk-viewer/src/holocrons/line.rs)'s. Muted
  players' finds and unknown tiers are left out as drops are.
- **How a find looks** (01/11/2026, Sol's request): in all three views a drop's or
  find's mark is the holocron's or crystal's own picture (`gfx/sjk/holocron_<tier>`,
  [holocrons/icons.rs](../crates/sjk-viewer/src/holocrons/icons.rs), drawn 1.3 times a
  tick's side), the gem only where the picture is missing; in the game's chat it starts
  the line, in place of SJK's emblem. In the game's chat the prize's words are drawn in
  the prize's own colour (the blade's tier, a card's rarity or rank), the rest of the
  line in the holocron's; a find of a blade skin the client knows draws that blade after
  the picture, small and live as the Collection's swatches draw it, fading with its
  line. The hub's `item` (the shader's unlock id) comes through as `FindMark::skin`; a
  hub without it gives the colour from the shader's tier and no blade. While the
  composer is open, resting the pointer on the blade shows it large in a card above the
  line (its tier in its colour, its name, the blade), as long as the pointer stays on
  it ([chat/find_blade.rs](../crates/sjk-viewer/src/chat/find_blade.rs)).
  `debug_find [tier] [blade id or #number]` puts a made-up find of a blade in the feed
  (a Legendary Kyber Crystal and a Mythical blade by default), nothing sent.

### Staff

The Staff page ([identity.md](identity.md#staff)) has a Holocrons and kyber crystals
section on its Holocrons and credits tab: the chosen player's unopened counts of both, the
four holocron tiers and under them the four kyber crystal tiers to choose (the choice
ringed in its colour, and named under the chips: "Give: a Rare Kyber Crystal, opened to a
saber shader"), Give (with the note field, `StaffRequest::HolocronGive`, `POST
/v1/staff/holocron-give`, which takes a crystal tier id such as `kyber_rare` as well as a
holocron's; 26/10/2026, Sol's request) and the player's twelve newest holocrons and
crystals (number, date, how it came: play, gift, trade or earned, and whether opened) with
Remove on a second press (`StaffRequest::HolocronRemove`, `POST
/v1/staff/holocron-remove`). A crystal given by staff is a sealed row of source `staff`:
clearing an achievement never takes it back, and it opens like an earned one. The hub's answer is the
target's profile and replaces the chosen player's, and the player's own when it is theirs.
Since 08/11/2026 every staff member gives holocrons and crystals, never to themselves and a
number a day, and undoes their own gifts of the day on the My gifts tab; Remove stays the
creator's ([identity.md](identity.md#staff)).

### `debug_holocron`

`debug_holocron <tier|all> [x<count>]` (alone it lists the tiers and the odds) rehearses
drops offline: `all` queues one of each tier, `rare x3` three of one (1 to 20). They go
through the same queue, card (in a match), pop-up, ceremony and sound, and the first one's
chat line goes into the game's feed as the player's own. A crystal tier (`kyber_rare`, or
`kyber` for one of each) rehearses a crystal's arrival, earned from an achievement, with no
chat line. `debug_open kyber_rare x3` (or `kyber`) plays a crystal's opening, each a
droppable shader its odds can give. The console closes so the pop-up
shows at once on a menu, or the cards over play in a match. Nothing is sent to the hub, and rehearsed holocrons (numbered from
4,000,000,000) are never written to `holocrons_seen.txt`.

### The Holocrons tab

The Collection's sixth tab in the SJK UI ([sjk-ui.md](sjk-ui.md#collection); the Profile
screen's eighth until the Collection was made, the same day); with the classic menus the
Profile page's See holocrons or the `holocrons` command open the same page on its own, in
the SJK UI's look as the Collection page does. Left, the world is left
clear for one 3D holocron; right, over a dark panel, the four tiers (their picture, name in
the tier's colour, how many are held or a dash while the hub has not answered, and the odds
of a drop), since 21/10/2026 the kyber crystals ([above](#kyber-crystals): a "Kyber
crystals" heading with the pity line at its right, then a tile per crystal tier with its
gem, the tier in its colour and how many are held, chosen like a row; its hover card says
"Earned from achievements. Always a saber shader.", that each achievement brings one, the
pity and how many are held), what the chosen tier is (for a crystal what opening one gives,
by shader tier), "Next holocron in about N minutes of play" with a bar
and "n of 8 today" (once the cap is reached, "Limit reached: resets in about 5 h 20 min",
the time until the hub's `resets_at`, counted from the hub's clock when it answered;
"Limit reached for today" from a hub that does not say it. Before 29/10/2026 the page
worked the end of the hub's rolling 24 hours out of the play drops the profile lists, and
a holocron received in a trade, listed as a play drop but not counted by the hub, put it
off: the page could say hours while the next one came at once. A player's
report (#77, 19/10/2026) had read "8 of 8 today" as a calendar day that had not reset), the newest eight holocrons (ten until the crystals took a row; tier, `dd/mm/yyyy HH:MM` in UTC, and for a gift
"Gift from the SJK team" with the note) and, when the identity is off, no hub is set or the
hub has not answered, the reason ("Holocrons need the SJK identity"). Since 12/10/2026 the
counts are the holocrons not opened yet, the player's credits (and since 17/10/2026 their
kyber shards) stand at the panel's top right, the chosen tier says what opening one gives,
Open and Open all (and Shop, the Collection's Forge on its Shop, while the hub lets the player open, [cards.md](cards.md#the-forge))
stand under the holocron's caption and opened holocrons are marked in the recent list; opening and its
ceremony are in [cards.md](cards.md#in-the-client). While the hub keeps opening to staff
(`can_open` false, [cards.md](cards.md#opening)) a line says opening comes soon in place of
the buttons; when this build is older than the hub's minimum client (since 21/10/2026) the
line says to update to that release instead.

- **Keys and pointer:** Up and Down (or Left and Right, Tab and Shift+Tab, the wheel, 1 to
  4 for the holocrons and 5 to 8 for the crystals, Home and End) choose the tier, the
  crystals coming after the holocrons; Escape goes back; a click on a row chooses it; Ctrl+Tab
  is the Profile screen's. Enter opens the oldest holocron (or crystal) of the tier chosen, A opens up
  to twenty of that kind (every holocron, or every crystal: a crystal's opening strip shows
  shaders only, by the crystal's odds), asking first when more than one would open ([cards.md](cards.md#in-the-client))
  (both only when the hub lets the player open). L, or Loot table at the right of what the chosen tier holds, shows the
  [loot table](#loot-table); S, or Shop, the Forge's Shop ([cards.md](cards.md#the-forge)).
- **The 3D holocron** is a normal object of the world pass, not a stage of the Character
  tab: an instance of each part's model of the chosen look ([the four
  holocrons](#the-four-holocrons)), posed from the time, and a point light in the tier's colour
  ([holocrons/stage.rs](../crates/sjk-viewer/src/holocrons/stage.rs)), floating before the
  camera on the ray through the clear left of the 16:9 frame (`STAGE_AT`), 64 units ahead,
  scaled to a fixed size on screen in every window shape. It turns and bobs as Illuminate's
  does (`illuminate::idle_rotation`, `idle_bob`). While a holocron is opened it is that
  one: it glides to the middle of the window, spins up and is gone in a flash, then grows
  back in its place ([cards.md](cards.md#in-the-client), `holocrons::stage::Motion`). A new tier shrinks the old look away in
  0.12 s and grows the new one in 0.32 s. While the tab shows, the backdrop camera parks on
  its own shot (`Shot::Holocrons`, a route on duel6, where the toured camera cuts to it;
  other maps keep the main vantage), and the classic menu style stops hiding the world
  under it. Over a match (the game menu's Profile screen) it floats before the game's
  camera. The Character tab's stage was not used because that stage is the player model's
  (one actor with a pose, sabers and cosmetics) and is not drawn in a match, while the tab
  needs a rigid model that swaps by tier and works in both.
- **A tier of its own** has models of its own. A surface's shader is named by its model
  file, so the shared `holocron.md3` (shader `models/sjk/holocron`) would draw the base look
  for every tier whatever shader was given. `scripts/holocron_assets.py` writes each tier's
  parts (`holocron_<tier>.md3`, the body, and `holocron_<tier>_<part>.md3`), naming the
  tier's face, its light, and the shared steel and dark metal; they are mounted with the
  other files and loaded with the map's rigid models. Tests check each part names shaders of
  its own look, every shader and picture is mounted, and a moving part is modelled about its
  centre. Illuminate's holocron is unchanged.
- **A tier held none of** (or not known) shows the locked look and not nothing, so all four
  can be seen: the same shape (`holocron_<tier>_locked.md3` and each part's `_locked.md3`)
  with every material drained of colour and dark (`holocron_locked.shader`), no glow, its
  parts at rest, lit by a light a fifth as strong in the tier's colour; a padlock stands over
  the tier's name and by its row's count, and the caption says "None held yet: shown dimmed".
- The hub's counts, list and progress are copied from the identity's snapshot twice a second
  while the page is open (`Data::of`), never per frame, and the hub is asked for fresh
  progress when the page opens (at most every 30 seconds).

## For other screens: the client's list

[holocrons.rs](../crates/sjk-viewer/src/holocrons.rs) is the one list the rest of the
client reads; the Collection's Holocrons tab is built on it.

- `TIERS: [Tier; COUNT]`, `tiers()`, `Tier::from_id` / `by_id(id)`, `colour(id)`, `name(id)`;
  a `Tier` has `id`, `name`, `colour` (`colour_alpha(a)`), `per_mille`, `odds` text, `index`,
  `icon_path()`, `article()` and `is_announced()`.
- `entries(list)` (known tiers only, in the hub's order, an `Entry` has `id`, `tier`,
  `dropped`, `gift`, `note` and `when()`) and `counts_of(counts)` read a profile's holocrons;
  the Holocrons tab reads the own profile and `snapshot.holocrons` (the progress) once
  in `holocrons_panel::Data::of`.
- `refresh()` (ask the hub again when a tab opens)
  and `next_text(&progress, lifts_in)` ("Next holocron in about 20 minutes of play", or
  "Limit reached: resets in about 5 h 20 min" with `lifts_in` from
  `cap_lifts_in(&progress, now)`, else "Limit reached for today").
- `icons::texture(index)` is the atlas cell of a tier's picture, uploaded with the HUD's
  pictures; `hud.holocron_icons` says which loaded (`icons::is_loaded(index)` for the pages the
  console draws). `gem::draw(list, centre, half, colour, alpha, rows)` draws the fallback.
- `Tier::light` is the colour of the tier's point light and `Tier::about()` what the tab says of
  the tier. The 3D holocron's models are `stage::TIER_MODELS` and `stage::LOCKED_MODEL`.

## Editions

Since 06/11/2026 (Sol's collections, [unlockables.md](unlockables.md#alpha-and-beta-the-collections)):
every holocron and kyber crystal carries the **edition** the hub stamped on it when it was
given, `"edition": "alpha"` or `"beta"` on each row of the profile's `holocrons` list (the
wire `Holocron::edition`, [wire.rs](../crates/sjk-identity/src/wire.rs); a hub before then
sends none, read as `alpha`, `wire::EDITION_FIRST`). It keeps it for good, through trades
too. Opening one brings only things of its edition or an older one (the hub's
`editions::within(item, holocron)`): an Alpha
one only Alpha things, a Beta one Beta or Alpha. The hub stamps it at every give (play
drop, staff or operator gift, purchase, mystery box, toy, kyber crystal), Beta only while
its switch `--beta-min-client VERSION|off` (`SJK_HUB_BETA_MIN_CLIENT`, off by default)
names a release and the person's newest client meets it; until then every one is Alpha.

The client keeps it on `holocrons::Entry::edition`; the Holocrons tab shows it before each
recent row's date ("Beta · 06/11/2026 14:05"), on a tier's hover card (the editions of the
unopened ones held, `Data::editions_line`: "Alpha Edition", or "2 Alpha, 1 Beta"), and in
both loot tables' notes.

## Checked and not checked

Verified (10/10/2026, Windows 11, `cargo test --locked --workspace`): the wire types parse
(profiles with and without the new fields, drop events, the progress answer); claims carry
`active` only when true; the worker reads progress after registering and every five
minutes, treats a hub without holocrons as cosmetic and reads the profile and progress
soon at most twice a minute after an own drop; the feed merges drops with messages in id
order and tells the worker once per own drop; staff give and remove send the protocol's
fields; the catalogue, its colours and the pop-up's queue, order, first-read cap, seen file,
rehearsal, ceremony and layout (every tier, with and without a picture, a note, in five
window shapes, in the families and in Inter); the three chat views' lines; the staff page's
requests; the activity rules against player states. Not tried: any of it against a running
hub (`crates/sjk-identity/tests/hub_e2e.rs` has an ignored test that needs one), in a game,
or with the art of the tiers (the pop-up draws the gem until the pictures are mounted); the
input hooks and the 500 ms comparison are not covered by a test of their own.

The Holocrons tab (10/10/2026): unit tests for its keys, wheel and clicks, the stage request
for the chosen tier (and the locked look), the data from every identity state, the twice a second
read, every state at 1080p, 4K, 4:3 and 21:9 on the Profile screen and on its own, the 3D
holocron's placement in those windows and its swap, and the tier models' and shaders'
mounting (`illuminate::tests::every_part_names_shaders_of_its_look_and_they_are_mounted`).
The four holocrons (17/10/2026): `holocrons::stage::tests` check the parts' table, poses and
models (since 21/10/2026 also the kyber crystals', `Request::of` and the forms' lights), the
world shot `duel6_sjk_kyber_forms` (21/10/2026) shows the engram and the four kyber crystals
held at two moments and locked, and the engram alone at three moments (looked at on this
machine; the breath was checked by a test build with the plates pushed far out), and the
world shot `duel6_sjk_holocron_forms` shows each tier held at two moments and
locked (looked at on this machine, with `duel6_sjk_crate_open`); not tried in a running
client, so the parts' motion has only been seen as stills. The world shots
`world_shot::holocrons_tab::duel6_sjk_holocrons` and `duel6_sjk_holocrons_windows` (off-screen
renders on duel6 with made-up holdings) were rendered and looked at on this machine: the tab
in the SJK UI with each tier chosen, a tier held none of, the identity off, and 4:3, 21:9, 4K,
`ui_textScale 1.2` and the classic style. Not tried: on another map than duel6, with real hub
data, the keys and clicks in a running client, or the swap's motion (the shots are stills).

Over a match (10/10/2026, Sol's report: the holocron sat too far right in a game): it is
placed from the view the frame renders (`GpuState::frame_view`: the third-person camera
too) with that view's own vertical field. It took the renderer's vertical field for the
horizontal `cg_fov` and narrowed it again, so its offset to the left came out about three
quarters of what it should (and it floated before the eye, not the third-person camera).
The unit test projects it back through the view at 90 and 64 degrees in four windows, and
the world shot `duel6_sjk_collection` renders it from the game menu at a match's field.

Kyber crystals (21/10/2026): `kyber::tests` (tiers, built-in odds and the hub's, pity line,
rewards against the achievements, the arrival's words), the tab's data and opening
(`crystals_are_counted_apart_and_open_on_their_own`), the shader-only strip, the
rehearsals, the chat line and the unlock card; the world shots
`world_shot::holocrons_tab::duel6_sjk_kyber_crystals` (the tiles, a crystal chosen and one
held none of, its hover card, its loot table, the strip and the shader's entrance) and
`world_shot::collection::duel6_sjk_kyber_achievements` were rendered and looked at. Not
tried against a hub that sends crystals.

## Planned, not built

What opened holocrons will also give: the cards' faces ([cards.md](cards.md#planned-not-built)).
The shop (in kyber shards), the kyber shards and the holocrons credits buy (an Uncommon
for 150, a Rare for 400, source `bought`, outside the caps) are live since 21/10/2026, in the
Collection's Forge ([cards.md](cards.md#the-forge)).
Saber shaders drop already
([unlockables.md](unlockables.md#holocron-shader-drops)).

## Profile banner prizes

Openings may award a counted profile banner instead of a card and credits, when
the hub supplies a banner share. Other prize shares stay unchanged, and crystals
still give shaders only. The reveal shows the cover and links its use to Collection
> Banners. See [profile banners](profile-banners.md#echo-rewards-and-ownership) for
the first catalogue, proposed balance and inventory rules.
