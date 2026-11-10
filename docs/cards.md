# Cards and credits: what a holocron holds

**Status (12/10/2026): opening is built, cards are sealed.** Opening a holocron
gives one *sealed* card and some credits. A sealed card has its rarity, finish and
edition but no face yet: the faces come with the first set of cards, and every
sealed card is revealed into that set then. Credits are kept and shown. Since
17/10/2026 the economy is prepared, not deployed ([Shards and the shop](#shards-and-the-shop)):
kyber shards beside credits, shattering duplicates, crafting, a daily shop that spends
shards, and holocrons bought with credits (Sol's decisions of 17/10/2026). [Holocron drops](holocrons.md) give the holocrons. The wire is the hub's
`PROTOCOL.md` ("Holocrons", "Opening", "Shards and the shop") in Sol-Vulpes/SJK-hub.

Collecting cards grants nothing: no power, setting or right, as with medals
([identity.md](identity.md#medals)). Cards never trade (Sol, 23/10/2026: collections
will bring rewards); players may swap duplicate shaders, and later unopened holocrons, with
each other ([sjk-ui.md](sjk-ui.md#trading)), and there is never any real money in the
system. Hats and capes stay free for everyone and are never sold or dropped
([client.md](client.md#hats-and-capes)).

## Decisions

| Question | Decision |
| --- | --- |
| What opening gives | One card and credits per holocron (Sol, 12/10/2026). |
| Cards before the art exists | The card is sealed: rarity, finish and edition are rolled at opening; the face is chosen when the first set is out (Sol, 12/10/2026). |
| Where a sealed card is revealed | Into the first set only, so Alpha cards stay launch-set cards (my pick, Sol's go-ahead). |
| What the holocron's tier does | A higher tier raises the odds of rarer cards, better finishes and more credits ([Odds](#odds)). |
| Finishes | Normal, Holo, Solar (the rarest: an animated sunlit shimmer). |
| Editions | A stamp apart from the finish: **Alpha Edition** while the alpha lasts. Every other collectible carries one too ([Editions](unlockables.md#editions)). Since 06/11/2026 holocrons and kyber crystals carry one as well, stamped by the hub when given: an Alpha one brings Alpha things only, a Beta one Beta or Alpha ([Alpha and Beta](unlockables.md#alpha-and-beta-the-collections)). Cards are not gated by it yet (every face is the SJK Alpha set) and a copy keeps the Alpha Edition stamp. The rest of the holocron seasons design (Beta tables with guaranteed prizes, pity, no duplicate faces; branch `personal/beta-season`) is not built. |
| Credits' sign | The galactic credit's, the slanted seven crossed by two bars (Sol's pick, from Wikipedia); drawn from shapes. |
| What credits will buy | An Uncommon (150) or a Rare (400) holocron, outside the daily caps; Legendary and Mythical ones are only ever earned (Sol, 17/10/2026, [Holocrons for credits](#holocrons-for-credits)); prepared, not deployed. Until then the shop sold for credits. |
| Duplicates | Cards keep a count ("x3"). A droppable shader held twice or more can be shattered into kyber shards, one copy at a time, never the last (Sol, 17/10/2026; it replaced selling duplicates for credits). |
| Kyber shards | A second currency (Sol, 17/10/2026): from shattering duplicates, a holocron's pile of shards and a card's pack; spent to craft the droppable shader the player chooses and in the shop's four daily offers ([Shards and the shop](#shards-and-the-shop)). |
| Drop rates | Start stingy; buffing later is easy on the hub and nerfing is not. |
| Seasons | A holocron keeps the season it dropped in, and opens with its tables and edition; Beta starts with a release, per player (Sol, 05/11/2026, [Seasons](#seasons)). |
| Money, trading | No money: free and non-commercial. Cards never trade; duplicate shaders, and later unopened holocrons, trade between players for each other only ([Trading](sjk-ui.md#trading)). |

## Opening

All decided by the hub, in one transaction:

1. `POST /v1/holocrons/open` with `{"ids":[..]}` (1 to 20 of the player's own
   holocrons not opened yet, signed). For each the hub rolls the card's rarity and
   finish and the credits, stores the sealed copy (with the current edition, the
   holocron and its tier) and a line in the credits ledger, and marks the holocron
   opened. All or nothing: 404 `not_found` (not the player's), 409
   `holocron_opened` (opened already, from another PC of the person's), 400
   `bad_ids`.
2. It answers what each brought (`opened`: holocron, tier, card's id, rarity,
   finish, edition, credits) and the person's profile after.
3. The profile carries `credits` (the balance) and `cards` (the copies, one entry per
   card, rarity, finish and edition, with a count, the first and last time; `card` is
   null while sealed). A holocron's entry carries `opened`; `holocron_counts` counts
   only the unopened ones, so a client shows what can still be opened.

Opening counts toward no cap, and an opened holocron still counts toward the day's
drop caps. Removing an opened holocron (staff) leaves its card and credits.

**Staff only for now.** Since 12/10/2026 (Sol's request, while opening is worked on)
the hub lets only staff open holocrons unless its operator opens it to everyone
(`--opening everyone`, or `SJK_HUB_OPENING=everyone`, on the hub; no client release is
needed). Anyone else gets 403 `opening_staff_only` and keeps their holocrons. The
progress answer (`GET /v1/holocrons`) says `can_open`; the Holocrons tab hides Open and
Open all, and Enter and A do nothing, when it is false (or not read yet), and says
"Opening comes soon: keep your holocrons, they will wait." in their place.

**Minimum client.** Since 21/10/2026 the hub's operator can also want a recent SJK
for opening, revealing, shattering, crafting and the shop (the hub's "Minimum client",
Sol's choice: holocrons open for everyone from the release that has it). The hub then
refuses any client below that release, or one that does not say it is SJK, with 426
`client_update_required` and the message "Update SJK to open holocrons: this needs SJK
<version> or newer."; staff and dev builds (`SJK/dev`, Sol's `play` builds) pass. Reading
holdings stays open, so an old client still lists its holocrons and cards. Clients from
before show the hub's message as it is: on the opening's "Nothing was opened" page after
Open, and on the shop's message line. Since 21/10/2026 the client reads `min_client` from
the progress answer and compares its own version number by number
(`build_info::meets_min`; a dev build always meets it): a build older than it treats
`can_open` as false (no Open, Open all, Shop, Reveal or shatter) and the Holocrons tab says
"Update SJK to open holocrons: this needs SJK <version> or newer." in place of the
buttons. A refusal with that code reads the same (`update_notice` in `sjk-identity`).
World shot: `world_shot::holocrons_tab::duel6_sjk_holocrons_update_needed`.

## Odds

The hub's tables (`src/cards.rs` there); the holocron's tier picks the row. Players see
them, with the chances per holocron found, on the Holocrons tab's loot table
([holocrons.md](holocrons.md#loot-table)); the client's copy (`cards::ODDS`,
`cards::CREDITS`) is pinned to these by a test.

Rarity of the card (Mythical since 19/10/2026, Sol's decision: its share taken from each
tier's Legendary one, the rest unchanged):

| Holocron | Common | Uncommon | Rare | Legendary | Mythical |
| --- | --- | --- | --- | --- | --- |
| Uncommon | 70 % | 24 % | 5.5 % | 0.4 % | 0.1 % |
| Rare | 50 % | 33 % | 15 % | 1.8 % | 0.2 % |
| Legendary | 30 % | 38 % | 26 % | 5.5 % | 0.5 % |
| Mythical | 10 % | 30 % | 45 % | 13 % | 2 % |

A client counts five rarities only since 19/10/2026: an older one finds the hub's
five-rarity tables do not fit and keeps showing its own copy (the four-rarity odds), and
leaves Mythical copies out of the Cards tab (an id it does not know).

Finish of the copy:

| Holocron | Normal | Holo | Solar |
| --- | --- | --- | --- |
| Uncommon | 97 % | 3 % | 0 % |
| Rare | 90 % | 10 % | 0 % |
| Legendary | 70 % | 29 % | 1 % |
| Mythical | 40 % | 55 % | 5 % |

Credits, any whole number in the range, equally likely:

| Holocron | Credits | On average |
| --- | --- | --- |
| Uncommon | 10 to 20 | 15 |
| Rare | 30 to 50 | 40 |
| Legendary | 80 to 120 | 100 |
| Mythical | 250 to 350 | 300 |

The averages below describe the legacy built-in fallback, before a supporting hub
supplies its tag/banner shares. The live loot table recalculates from the hub's
actual odds. [Profile banners](profile-banners.md#echo-rewards-and-ownership) use a
proposed 10% share taken from cards, with no card or credits on a banner opening.
This reduces card frequency and average credits; shader/tag/shard-pile shares stay.

Under those fallback tables, about 35 credits a card opening on average; counting the openings that bring a shader or a
pile of kyber shards instead (no credits), about 17.5 credits a holocron found, so about 140
a day under the holocrons' daily cap of 8.

Since 17/10/2026 (prepared, not deployed) one opening in ten, in every tier, brings a pile of
kyber shards instead of the card and its credits, taken from the card's share (the shader's
odds stay): any whole number in the range, equally likely.

| Holocron | Shard pile | Shards | On average a holocron |
| --- | --- | --- | --- |
| Uncommon | 10 % | 5 to 15 | 1 |
| Rare | 10 % | 10 to 25 | 1.75 |
| Legendary | 10 % | 25 to 50 | 3.75 |
| Mythical | 10 % | 100 to 200 | 15 |

The piles are a consolation: about once a day of capped play, a pile is about one
duplicate of the holocron's own level shattered, a Mythical holocron's as much as a
Mythical shader's shattering.

**The card pack** (Sol's request, 17/10/2026, prepared on hub branch `feat/card-pack`, not
deployed): a card is the most common and least wanted prize, so it comes as a pack, a few
kyber shards beside its credits (about a fifth of the tier's pile), rolled after the card,
any whole number in the range, only while the economy is on:

| Holocron | Shards with a card | On average a holocron |
| --- | --- | --- |
| Uncommon | 1 to 3 | 1.2 |
| Rare | 2 to 5 | 1.75 |
| Legendary | 5 to 10 | 3.4 |
| Mythical | 20 to 40 | 12 |

The hub sends the range as the loot tables' `card_shards`; the opening answer's `shards`
then rides with the `card` (a pile has no card). Provisional, the hub's `src/loot.rs`. The client shows the ranges
([cards.rs](../crates/sjk-viewer/src/cards.rs) `CREDITS`, the hub's copy, tested);
the hub decides. Rolls use the operating system's entropy, as drops do.

## Seasons

Designed 05/11/2026 (Sol's request); built 10/11/2026 (hub branch `beta-season`, client
`personal/holocron-seasons`), not deployed: nothing changes for players until the hub is
deployed and Beta opened (Order of work, step 3). A **season** is a holocron's loot
tables. **Alpha** is every table above; **Beta** is the next one, made so that opening a
Rare, Legendary and above all a Mythical holocron always feels worth it.

### Rules

- **A holocron's season is its edition** ([Editions](unlockables.md#editions)), stamped by
  the hub since 06/11/2026: no second stamp.
- **A holocron keeps the season it dropped in.** The hub stamps it when it drops;
  opening it later always uses that season's tables and stamps its prizes with that
  season's edition ([Editions](unlockables.md#editions)). An Alpha holocron opened in
  Beta is still Alpha: holding on to holocrons gains nothing and loses nothing (Sol: "no
  rewarding hoarding").
- **Beta starts with a release, per player.** The hub stamps Beta on a drop when the
  claim comes from the Beta release or newer (the client version the hub already keeps per
  key, [identity.md](identity.md)); an older client keeps finding Alpha holocrons. There is
  no switch date.
- **Opening a Beta holocron needs the Beta release** (the existing `min_client` gate,
  per season): an older client lists it but cannot open it, and says to update.
- Bought holocrons (credits) and staff gifts take the season of the client that asked,
  as drops do. Trades keep the holocron's season.
- The loot table shows the tables of the season the player's next drop will have, with
  a switch to see the other season's.

### Beta tables

Every holocron gives **guaranteed prizes**; what were alternatives to the card (tag,
pile of kyber shards, banner) become **bonus rolls** on top.

| Holocron | Guaranteed | Bonus rolls |
| --- | --- | --- |
| Uncommon | As in Alpha: one prize (a shader, or a card and credits) | As in Alpha |
| Rare | A card, **Uncommon or better**, and credits; a shader, **Uncommon or better** | One: tag 25 %, shard pile 25 %, nothing 50 % |
| Legendary | A card, **Rare or better**, and credits; a shader, **Rare or better** | One: tag, shard pile or banner |
| Mythical | A card, **Holo or better**, and credits; a shader, **Legendary or better** | Two: tag, shard pile, banner or a second card |

Card rarity and finish (Beta; Uncommon as Alpha):

| Holocron | Common | Uncommon | Rare | Legendary | Mythical | Normal | Holo | Solar |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Rare | 0 % | 70 % | 25 % | 4 % | 1 % | 88 % | 12 % | 0 % |
| Legendary | 0 % | 0 % | 70 % | 25 % | 5 % | 65 % | 33 % | 2 % |
| Mythical | 0 % | 0 % | 50 % | 38 % | 12 % | 0 % | 85 % | 15 % |

Credits and the card pack's shards stay as in Alpha. The guaranteed shader's tier has
Beta's own odds (Sol, 10/11/2026: "I feel like I should have dropped more"; Mythical
holocrons are rare enough to give a good chance of the best):

| Holocron | Uncommon | Rare | Legendary | Mythical |
| --- | --- | --- | --- | --- |
| Rare | 65 % | 28 % | 6 % | 1 % |
| Legendary | | 55 % | 35 % | 10 % |
| Mythical | | | 60 % | 40 % |

Beta kyber crystals get their own odds too, at least as good as the Beta holocron of
their tier (Common to Mythical): Uncommon 30/45/20/4/1 %, Rare 0/55/33/9.5/2.5 %,
Legendary 0/0/45/40/15 %, Mythical 0/0/0/50/50 %. Alpha holocrons and crystals keep their
odds. Provisional: tuned after a week of real Beta openings.

### Pity

Per person, counted over **Beta** holocrons opened (as built: an Alpha opening leaves it
alone, so nobody starts Beta with a run already owed; a choice for Sol to confirm), shown on
a Beta holocron's hover card and on the loot table's Beta page:

- a prize of **Legendary or better** (shader or card) at the latest every **15**
  holocrons opened (about two days at the daily cap);
- a **Mythical** prize (shader or card) at the latest every **80** (about ten days).

A natural one resets its counter. The last opening of a run rolls its prize's tier from
the floor up: the guaranteed shader's (every Beta tier from Rare has one since 10/11). A Beta Uncommon holocron counts but is never raised, so a run it would end
waits for the next Rare or better one. Tags, banners and shards do not count.

### No duplicate faces

Legendary and Mythical holocrons pick a card face the player does not hold yet; a
duplicate only once they hold every face of that rarity. Shaders already prefer the ones
not held; one held in every tier of the floor gives the next tier up, or a pile of shards
as large as a Mythical shader's shattering when none is left.

### What it costs the economy

A Mythical holocron drops about once in 67 (1.5 %), so at the daily cap one every eight
days or so, each with a Legendary or better shader: a Legendary shader in the shop costs
about 29 days of capped play. Accepted (my recommendation, 05/11/2026): Mythical drops
are capped at one a day, and shaders avoid duplicates, so it fills a collection rather than
the shard bank. Watch the shop's Legendary sales after a week.

### As built (10/11/2026)

- Hub `src/seasons.rs` (tables, floors, bonus shares, pity), `src/db/seasons.rs` (the
  opening), table `holocron_pity`; Beta openings need the economy on. Bonus shares: a
  Legendary holocron's tag, shard pile or banner a third each, a Mythical one's tag, pile,
  banner or second card a quarter each (the second card without credits); while banners
  do not drop their share is shared out among the others. A guaranteed shader is one the
  person lacks first (the rolled tier, then above, then below to the floor). A Beta card
  copy is stamped `beta`. Wire: the hub's `PROTOCOL.md`, "Seasons".
- Client: `Prize::all` shows each prize of a Beta holocron as its own reveal, its best
  last; `seasons.rs` writes the lines; the loot table's B key shows the Beta page; a Beta
  holocron's hover card says what it guarantees and the pity; `debug_open mythical beta`
  rehearses one.
- Open for Sol: confirm the tables, the shader-floor economy risk below, pity counted on
  Beta openings only, and when to open Beta (with Creyon's blades made droppable).

### Order of work

1. Hub: `season` on holocrons (existing ones Alpha), the version gate at drop, tables per
   season, guaranteed prizes and bonus rolls in one opening, pity counters, the face pick.
2. Client: the opening answer with several prizes (the ceremony shows them one by one,
   the best last), the season on a holocron (a small stamp), the loot table per season,
   the pity line, the Beta edition in `editions.rs`.
3. Release, then the hub's Beta gate set to that version.

## Credits

Credits are a ledger at the hub (`credit_ledger`: each change with its reason,
`holocron`, `holocron_buy`, `staff`, and `shop` from before the shop sold for shards, and
what it refers to); the balance is its sum and never
goes below 0. Staff correct a balance with `POST /v1/staff/credits` (logged), the
operator with `give-credits`; the Staff page's Holocrons and credits tab gives and takes
credits and kyber shards ([identity.md](identity.md#staff)).

## Shards and the shop

**Prepared 17/10/2026, not deployed** (hub branch `feat/economy`, client branch
`personal/economy`; Sol's decisions of 17/10/2026, after his direction of 12/10/2026 for a
shop that fixes bad luck). Every number here is provisional, in one table each at the hub
(`src/economy.rs`, `src/loot.rs`), sent to clients with the progress (`economy` and the
loot tables' `shard_pile`, `shards` and `card_shards`), the client keeping a copy for before
(`economy::BUILT_IN`, `loot_odds::BUILT_IN`, pinned by tests).

- **The trade.** Only saber shaders holocrons drop take part, less any a medal brings
  (Glitch, Hologram and Runic today). Both sides read it from their catalogues (the
  droppable flag, the medals' blades), never from a list, so a shader made droppable
  joins by itself. Achievements grant nothing today; one that brings a shader must be
  left out the same way. Cards are luck only: never sold, crafted or shattered. Hats and
  capes are free for everyone and never take part.
- **Kyber shards** are one currency for every shader (my choice for simplicity; Sol's
  question had offered "shards of one shader", see the open points), a ledger at the hub
  like credits (`shard_ledger`, reasons `shatter`, `holocron`, `craft`, `shop`, `staff`),
  never below 0, on the profile as `shards`.
- **Shattering** ("breaking down" in the Forge) turns one copy into shards, one copy a
  request: a duplicate (a shader held twice or more by drops or grants; a medal's or a
  card's copy does not count), and since 21/10/2026 the last copy too when the client asks
  (`"last":true`, a hub whose `economy.abilities` says `shatter_last`; Sol's promise in the
  2026.1021.1 notes of "breaking down any saber shader you own, not just duplicates
  (except those that come with a card)"), for the same yield: fair, as the shard value
  does not change with the copy. A copy only a card brings stays with the card (409
  `comes_with_card`), a medal's blade is never of the trade, and a last copy worn comes off
  the hub's live look and, once the hub answers, the client's `cg_saberSkin`. Every
  duplicate at once (`POST /v1/unlocks/shatter-duplicates`, `shatter_duplicates`) keeps one
  of each. The client offers both only where the hub says so, so against an older hub the
  Forge breaks down duplicates only.
- **Crafting** makes a shader of the trade the player does not hold, for its cost; a copy
  of its blade card lowers the cost ([SJK Blades](#sjk-blades), since 21/10/2026).
- **Who may:** like opening, everything follows the hub's opening setting: while opening
  is for staff, others get `opening_staff_only` and the Shop button is not shown.

| Shader tier | Shattering gives | Crafting costs | Shop price |
| --- | --- | --- | --- |
| Common | 5 shards | 40 shards | 30 shards |
| Uncommon | 10 | 100 | 75 |
| Rare | 25 | 300 | 225 |
| Legendary | 60 | 800 | 600 |
| Mythical | 150 | 2 000 | 1 500 |

Why these numbers:

- **Crafting against shattering:** eight to thirteen duplicates of a tier craft one of it,
  so shattering never mills one shader into another of the same tier. Shards come in at
  about 1.7 a holocron from the piles, and up to about 3.9 more once every shader dropped is
  a duplicate: a Common craft is then about 7 holocrons, a Mythical one about 360 (some 45
  days of capped play), against about 3 800 holocrons to drop one given Mythical shader.
- **The shop's prices** (Sol, 17/10/2026: the shop sells for shards): three quarters of the
  craft cost, so the day's offers are the cheaper road and crafting the dearer one to the
  shader the player chooses, and buying one to shatter never gains. Before, in credits
  (250, 600, 1 500, 4 000 and 10 000), for the reason that follows.
- **The credit prices before:** the rule of 12/10/2026 (the credits earned by the time 80 % of
  players would have dropped the item: 35 credits times `1.6 / p` holocrons) does not give
  sane numbers with today's catalogue: per holocron found a given Common drops at about
  1.95 %, an Uncommon at 0.75 %, a Rare at 1.38 % (only three Rares share their tier), a
  Legendary at 0.27 % and a Mythical at 0.026 %, so it prices them about 2 900, 7 500,
  4 100, 20 600 and 212 000 credits: a Rare cheaper than an Uncommon, and a Mythical some 750
  days of capped play. So the prices are per tier instead, rising with it: at about 17.5
  credits a holocron, 14, 34, 86, 229 and 571 holocrons (2, 4, 11, 29 and 71 days of capped
  play).

### The shop

- **Four offers a day**, each player's own (21/10/2026, the same for every player before):
  the hub skips the shaders they hold, but one bought from the shop that day stays listed
  as bought, and a shader gained during the day redraws only its own offer. One Common, one Uncommon, one Rare and
  a Legendary three days in four, a Mythical the fourth (Sol, 17/10/2026: with four
  Mythical shaders an even share brought each back about every eight days), each a
  different shader of the trade; a tier with none left takes the nearest that has one.
- **The day** runs from midnight to midnight in Paris (Europe/Paris, the EU's summer-time
  rule). The hub deals it from the day, the person and a secret it made once in its database
  (SHA-256, one stream an offer),
  so nothing is stored per day, a restart keeps the offers, and nobody, even with the hub's
  code, can tell a later day's.
- **Once a day, never a duplicate** (my decision): a player buys each offer at most once a
  day (`already_bought`), and the hub refuses a shader they hold (`already_owned`): shards
  never buy a duplicate, the shop being a fix for missing shaders. Crafting refuses one held
  the same way, so the only road to a duplicate is luck, and shattering is there for it
  (Sol asked whether bought shaders should be unbreakable instead; with prices above the
  shatter yield there is nothing to block, so no).
- A purchase is a line of the shard ledger (reason `shop`, the shader's id as its note),
  which is also how the hub tells what was bought today.

### Holocrons for credits

- **Credits buy holocrons** (Sol, 17/10/2026, once the shop sold for shards): an Uncommon
  for 150 credits and a Rare for 400; Legendary and Mythical holocrons are only ever earned.
  At about 17.5 credits a holocron opened, that is about one more holocron for every nine
  opened.
- A bought holocron is the player's at once (source `bought`), opens like any other, and
  never counts against the daily caps (8 a key, 16 an address, 1 Mythical).
- `POST /v1/holocrons/buy` `{"tier"}`; `not_for_sale` for another tier, `not_enough_credits`.
  The hub sends the prices with the progress (`economy.holocrons`), the client keeping a copy
  for before (`economy::BUILT_IN`); a hub that sends none sells none, and the shop shows no
  holocrons.

### The Forge

Since 21/10/2026 (Sol's request: a better interface for the shop, crafting and breaking
shaders down, in a tab of its own) everything kyber shards and credits trade is the
Collection's Forge tab, between Holocrons and Cards: Shop (the day's four offers and the
holocrons for credits), Craft (every shader of the trade the player lacks) and Break down
(the player's own copies, the last one too on a hub that takes it, and every duplicate at
once). The look, keys and states are in [sjk-ui.md](sjk-ui.md#collection); the requests
and refusals in the hub's `PROTOCOL.md` ("Shards and the shop"). Checked by unit tests and
world shots (`world_shot::forge_shots`); not tried in a game.

Since 29/10/2026 (collection marks, the ideas of the players Atrosex and Kushina extended
by Sol; [sjk-ui.md](sjk-ui.md#marks), the hub's `PROTOCOL.md` "Collection marks"): Break
down marks the shader the player wears (on either saber or staff half) and its last
copy's card says so first; the player's favourites are listed first in Craft and Break
down, with a Favourites chip on Break down; and a shader the player locked is never broken
down, one copy or the last, nor counted among all duplicates, nor offered in a trade, until
they unlock it. The hub holds the lock (409 `locked`), so a client cannot get round it; the
Forge shows the button dimmed with "Locked". Against a hub without marks, nothing of the
star or the padlock shows.

Fixed 22/10/2026, #117 (Nib: the Forge flickered on the last lines of Craft): Craft's grid
on its Legendary and Mythical lines, its fifteen swatches drawn rather than live pictures
(the menu pictures on, or before a live picture is ready), took up to about 2,100 draw
commands on some frames of their animation, past the Collection canvas's 2,048, so the
last commands of those frames were dropped and the page flickered. The canvas now holds
4,096 (`collection_panel::DRAW_COMMANDS`) and a frame's UI shapes 32,768 vertices, twice
before (the page alone reached about 11,600 of the 16,384). The world shot
`duel6_sjk_forge_drawn_swatches_fit` steps through those lines and their animation and
fails with 2,048. Not tried in a game; whether Nib's game had the menu pictures on is
not known.

### Open points for Sol

Answered by Sol on 17/10/2026: one generic currency; the piles, shatter and craft tables
kept for the launch; the shop in shards; buying or crafting a held shader refused; the
Mythical offer one day in four; credits buy Uncommon and Rare holocrons. Left: tuning
after a week of real openings, and a trade-up (spare copies of a tier for one of the next,
proposed, not built).

No player text promises any of this until it is deployed.

## Faces and the first set

Since 12/10/2026 (Sol's first card, "Toxiee and Sol on JoF Memorial", its art by "Sol
(AI-assisted)": the names in the title follow the picture, Toxiee on the left) a sealed
copy can become a real card. A card's `artist` line says how its art was made; the
footer reads "Art: <artist>".

The first collection is **SJK Alpha** (set `sjk_alpha`, Sol's decision of 19/10/2026; it
was "Sol lore" before). Its numbers 000 to 012 are the credits: Sol, Bishop (the author
of JKR, which SJK forked), Creyon, the early team and the projects and people SJK builds
on; `sjk_alpha_010` is the Toxiee meme card (Legendary, it was `sol_lore_001`), and
`sjk_alpha_000`, Sol's, brings the Sun blade while held (20/10/2026). Sol edits the cards
on the live hub since 29/10/2026 (from 20/10/2026 with a card editor on their PC), so the
hub's catalogue is the list as it stands, not this page.
`sjk_alpha_000`, Sol's, brings the Sun blade while held (20/10/2026), and `sjk_alpha_002`,
Creyon's, the Moon blade (26/10/2026, Creyon's blade, the Sun's sister). Sol edits the cards
himself since 20/10/2026 with the hub's Card Studio (`scripts/card_studio.py`), so the
hub's `assets/dist/cards.json` is the list as it stands, not this page.

**Credits cards (20/10/2026, Sol's decision):** so that no credit reads as ranked above
another, every credits card is Mythical (the hub's import and pack build refuse any other
rarity). Since the same day no card shows its number in the game, whatever its kind
(Sol's decision: "it's not a good thing"): not on its face, its tile, the Cards tab's
close-up, its hover card or an opening, which name its kind and collection instead ("Meme,
SJK Alpha"). The number still makes the card's id and orders the cards in the binder.

**Credits cards as tags (31/10/2026, Sol's decision):** a revealed copy of a credits card
lets the player stick the card on walls with the Tag toy ([client.md](client.md#tag)): its
picture as a sticker, in the finish of a copy held (Normal from any copy, Holo or Solar
from a copy in it). The hub names the card kinds it lets be tags in every profile
(`tag_cards`, its `tags::CARD_KINDS`: `credits`) and checks the copy when a look places
the card; Collection > Tags lists them under the tags. The card is kept, as crafting for
less keeps it: sticking spends nothing.

A card's kind says what it is about and reads on its strip: `community` (Community),
`place` (Place), `personality` (Personality), `game` (Jedi Academy), since
19/10/2026 `credits` (Credits: the people the project owes) and `meme` (Meme), and since
20/10/2026 `blade` (Blade: a saber shader, [SJK Blades](#sjk-blades)), and since
01/11/2026 `clan` (Clan, Sol's decision: a clan's cards, the clan being the card's
collection, so JoF's cards read "Clan · JoF" and each further clan gets a set of its own;
the JoF cards were `community` before). Its
rarity is one of `common`, `uncommon`, `rare`, `legendary` and `mythical`. A face whose
kind or rarity a client does not know is left out on its own (named in the log) while the
pack's other faces still read: a client from before 19/10/2026 shows no Bishop face, and
the Toxiee card's copies read "Unknown card sjk_alpha_010" there under their sealed back.

A card file may name an unlockable a copy brings, `"grants": "saber_sun"` (optional,
since 20/10/2026, Sol's decision: Sol's card 000 brings the Sun blade). The key
holds that unlockable for as long as it holds a copy of the card and loses it with the
last one, as a medal's blade goes with the medal: the hub lists it in the profile's
`unlocks` with `"card": "sjk_alpha_000"` (`Unlock::card` in
[wire.rs](../crates/sjk-identity/src/wire.rs), absent from older hubs). Unlike a medal's,
such a blade may also drop from holocrons ([unlockables.md](unlockables.md#holocron-shader-drops)):
the card is a second way to get it, and a dropped copy wins over the card in the profile
(it reads as the player's own and trades; the card's never does). The
client reads `grants` into the face (`Face::grants`, [faces.rs](../crates/sjk-viewer/src/cards/faces.rs)):
the card's hover card and its close-up on the Cards tab add "Brings the Sun blade while
you hold it.", for an unlockable the client knows. The catalogue keeps the same pairs in
`unlockables::CARD_BLADES` ([unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)),
so it names a blade's card without the packs; a new card that
brings a blade adds an entry there and to its file. The Shaders tab's owned line then
reads "Yours since dd/mm/yyyy, with your Sol card" (the face's name, or the card's id
without one), and its source facts "Holocron drop" and "Sol card".

A copy in a given finish may also unlock a blade's extras (26/10/2026, Sol: "the Solar card
unlocks all the other customizations"): a Solar copy of SJK Alpha 000 unlocks the Sun
blade's Flare trail, Swing embers and Heat wake ([unlockables.md](unlockables.md#extras)).
The client reads the pair from the blade's signature (`@extras`) and the hub from its own
table (`cards::EXTRAS_GATES`); a new pair goes in both.

### SJK Blades

**Since 20/10/2026 (Sol's decision)** a second collection, **SJK Blades** (set
`sjk_blades`), holds one card for each saber shader holocrons drop (the droppable ones,
less the medals' blades), of kind `blade`. Its number is the blade's own (Sun is 001),
in its id only: no card shows its number (see the credits cards above). A blade card's file names its shader,
`"blade": "saber_sun"` (optional, read like `grants`: blank is none, an id the client does
not know is kept but named nowhere; `Face::blade` in
[faces.rs](../crates/sjk-viewer/src/cards/faces.rs)).

- **The Solar bonus:** a Solar copy of a blade card also drops its shader to the player, a
  real drop like a holocron's. The hub does it when the copy is opened (the opening's entry then carries both
  `card` and `unlock`, the shader in a holocron drop's shape `{"id","name","count"}`) and
  when a sealed Solar copy is revealed into a blade card (the reveal's entry carries an
  optional `unlock` of the same shape). Older hubs send neither and older answers still
  read (`Opened::unlock`, `Revealed::unlock` in
  [wire.rs](../crates/sjk-identity/src/wire.rs)).
- **The hub picks the set first:** with a set as large as the droppable shaders, picking
  evenly among every card of a rarity would almost always land on a blade card, so the
  hub first picks one of the sets that have a card of the rarity rolled, then a card in
  it (the hub's rule; the client shows whatever card it is given).
- **Crafting for less** (Sol, 21/10/2026): while the player holds a revealed copy of a
  blade card and not its shader, crafting the shader costs less by the best copy's
  finish, and the card is kept (cards stay luck only, never spent):

  | Best copy | Crafting costs | Mythical (2 000) | Rare (300) |
  | --- | --- | --- | --- |
  | Normal | 75 %, the shop's price | 1 500 | 225 |
  | Holo | 50 % | 1 000 | 150 |
  | Solar | 50 % (it dropped the shader once already) | 1 000 | 150 |

  The hub decides (`economy::BLADE_CARD_CRAFT`, sent as `economy.blade_card_craft`, and
  the craft answer's `blade_card`); a hub before takes the full cost and sends no table,
  read as no discount ([economy.rs](../crates/sjk-viewer/src/economy.rs) `blade_cards`,
  `craft_price`). The Forge's Craft lists the lowered cost, its "Crafting costs" reads
  "Crafting costs (Holo card -50 %)" and its button "Craft  (shard)  150  Holo card -50 %";
  the Shaders tab's line beside "Craft in the Forge" says the same, and the blade card's
  hover card adds "Any copy crafts the Sun blade for less." Up close on the Cards tab, a
  held blade card has a button under its words (which then keep to one line): "Craft the
  Sun blade in the Forge" while the shader is of the trade and the player lacks it, which
  shows the Forge's Craft on it, else "Go to the Sun blade", which turns to the Shaders tab
  on it; either clears a filter that would hide it. The shop's prices do not change.
- **Perk marks** (21/10/2026, Sol's request): each perk a card's copies bring has a mark
  from SJK Symbols, in `cards::faces::PERKS` (`Face::perks`; a new perk is one row there):
  a lightsaber when it brings a blade while held (`grants`), the Solar finish's sun when
  its Solar copies drop one, a smith's hammer when its copies craft one for less. On the
  Cards tab each is a round badge over the tile's top left corner (dimmer on a card not
  found yet), the close-up's line under the copies held ends with the marks, and the
  hover card starts each perk's line with its mark.
- **In the client:** a blade card's hover card and its close-up on the Cards tab add "Its
  Solar copies also drop the Sun blade." (`Face::solar_drop`, for a shader the client
  knows); the binder's kind filter offers "Kind: Blade" once the packs have one. The
  opening and the reveal show the blade a Solar copy brought ([The ceremony](#in-the-client)).

- **Where cards are made:** on the live hub since 29/10/2026, which keeps each card's
  picture (1020 x 850, the 6:5 art window, framed as it will show) and words (`number`,
  `collection`, `kind`, `rarity`, `title`, `description`, optional `artist`). Before,
  Sol kept them in `card_import/` at the repository's root, which git ignores
  (`card_import/README.txt` says the format: `cards/card1.png` and `cards/card1.txt`, the
  full paintings in `sources/`, a white canvas and a frame overlay in `templates/`).
- **How they reach players:** the hub serves each card's `cards/<id>.card` JSON and
  `cards/<id>.jpg` (id `<set>_<NNN>` such as `sjk_alpha_001`) in its `sjk_cards` pack,
  and since 29/10/2026 in `sjk_cards_2`, `sjk_cards_3`... once one pack is full (the
  client reads every pack's `cards/` alike, and a card pack once listed stays listed).
  The hub serves them like the blade skins' (`GET /v1/assets`), so every SJK client
  downloads them into `assets/` beside `identity.key` and mounts them. Since 29/10/2026 a
  card edited, added or retired changes the hub's catalogue and card packs at once: no
  hub restart and no client release; clients take the new packs at their next check (at
  start, then every 6 hours). Until then the hub's `scripts/import_cards.py` and
  `scripts/build_packs.py` turned `card_import/` into `sjk_cards.pk3` and the catalogue
  `cards.json`, shipped with a hub restart; they now only seed a new hub.
- **The hub picks:** opening a holocron rolls the copy's rarity and finish as before,
  then picks a card of that rarity from the catalogue (since 20/10/2026 a set first, then
  a card in it, [SJK Blades](#sjk-blades)); a rarity with no card yet stays sealed. `POST /v1/cards/reveal` gives the person's sealed copies of a
  rarity that has cards a card each (oldest first). Both follow who may open
  holocrons (staff only for now).
- **Each card has one rarity** (Bishop is Mythical, the Toxiee card Legendary): the
  first set needs cards of every rarity before every copy can be revealed.

### Ranks

Since 31/10/2026 (Sol's decision, for the JoF collection) a card may show a rank in
place of its rarity: its file carries `rank` (the name players read, 1 to 24
characters, "Leader") and `rank_colour` (`#RRGGBB`), always together. Its `rarity`
stays and only sets its odds (JoF's Leaders are Mythical, its Council Legendary): a
ranked card never shows a rarity's name, mark or colour. Its rank's name stands at its
foot and on its tile, "Rank card" in its hover (no mark), its rank's colour edges its
plate, tile and badges, its drawn scene's backdrop, the opening's title and flourish and
the grand find's chat line ("a Leader card"; the hub's `finds` carry the card's id).
A rarity chip leaves ranked cards out of the binder, and their hover gives the card's
own odds ("about 1 in N holocrons found holds this card": the rarity's share, split
among the sets that have cards of it, then the set's cards of it). A sealed copy is not a
card yet and still shows its rolled rarity, as does the crate's spin. Ranks are data on
the card, so a new rank needs no client release (`cards::Tone`, `cards::faces::Rank`).
A rank without its colour, or a bad one, is left out (logged) and the card shows its
rarity. Older clients read past the two keys and show the rarity.

### Scenes

Since 30/10/2026 (Sol's idea) a card's file may name a `scene`: the client then draws
the card's picture itself, from the player's own Jedi Academy data, instead of the
pack's picture. Nothing of the game travels: no one ships, uploads or hosts it, each
client draws its own copy once and keeps it as a JPEG in `card_scenes/` beside the
packs' folder, named by the card and 16 hex of the scene's hash (and the drawing's
version, `cards::scene::VERSION`), so a changed scene is drawn again. The pack's
picture stays: older clients draw it, and this one shows it until the drawing is ready,
or for good this session when the game data lacks a model or the drawing fails (named
in the log). The JKA Characters and JKA Arsenal sets use scenes.

A scene is one line of at most 200 ASCII characters: up to three figures joined by
`+`, each a kind, a name and options, and the camera's options anywhere.

| Part | Meaning |
| --- | --- |
| `player <model>[/<skin>]` | a player model of `models/players/`, the game's own whatever rig model a pack lays over it |
| `anim=<NAME>` and `at=<0..1>` | the pose: an `animation.cfg` animation and how far through it (its idle at its start by default) |
| `saber=<hilt>`, `saber2=<hilt>` | a `.sab` hilt (`single_1`, `dual_1` for a staff, `luke`) in the right and left hand |
| `blade=<colour>`, `blade2=<colour>` | the blades' colours: red, orange, yellow, green, blue or purple (blue; the second as the first) |
| `model <path>` | any `.md3` or `.glm` under `models/` (a weapon's world model, an item) |
| `yaw`, `pitch`, `zoom` | the camera: turned round the figures from in front (-180 to 180, 25 by default), raised (-60 to 60, 8) and brought nearer (0.5 to 3, 1) |

For example `player luke anim=BOTH_STAND2 saber=luke blade=green`,
`player jedi_tf + player jedi_zf` or `model models/weapons2/disruptor/disruptor_w.glm`.
Weapons draw from their world models (`_w.glm`): the first-person models of most are
made of tagged parts and come out in pieces.

The figures stand side by side, framed by their own camera so they fill most of the
6:5 window, lit from the camera's upper left whatever the map, and drawn as the model
preview draws (`menu_stage/preview.rs`'s encode, with the blades through its blade
layer); the picture is read back and laid over the card's backdrop (a deep space
gradient with a glow of the rarity's colour behind the figures and round their
outline, so dark costumes stand out). Surfaces that write no depth (glass, a bacta
tank's) come out clear. One drawing at a time, recorded and submitted on its own inside
validation scopes, so a broken model fails that picture alone. Code:
[cards/scene.rs](../crates/sjk-viewer/src/cards/scene.rs) (the grammar, the cache,
the queue, the camera, the backdrop) and
[card_scenes.rs](../crates/sjk-viewer/src/card_scenes.rs) (the drawing). World shot:
`world_shot::card_scenes_shots::card_scenes_drawn_from_the_game_models`
(`SJK_SCENE` draws one scene, `SJK_SCENES` a file of `name<TAB>scene` lines, as
contact sheets of twelve).

## In the client

| Piece | Where |
| --- | --- |
| Wire types (`credits`, `cards`, `opened`, the opening's answer), the request, the outcome | [wire.rs](../crates/sjk-identity/src/wire.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [service.rs](../crates/sjk-identity/src/service.rs) (`Service::open_holocrons`, `Snapshot::opening`) |
| Rarities, finishes, editions, the credits' ranges, a profile's stacks | [cards.rs](../crates/sjk-viewer/src/cards.rs) |
| A card drawn from shapes, the finishes' light | [cards/look.rs](../crates/sjk-viewer/src/cards/look.rs) |
| The credits' mark | [cards/mark.rs](../crates/sjk-viewer/src/cards/mark.rs) |
| Open, Open all and the ceremony on the Holocrons tab | [holocrons_panel_opening.rs](../crates/sjk-viewer/src/holocrons_panel_opening.rs), [holocrons_panel.rs](../crates/sjk-viewer/src/holocrons_panel.rs), [holocrons_panel_view.rs](../crates/sjk-viewer/src/holocrons_panel_view.rs) |
| Kyber shards, shattering, crafting and the tables | [economy.rs](../crates/sjk-viewer/src/economy.rs); wire and requests in [wire.rs](../crates/sjk-identity/src/wire.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [service.rs](../crates/sjk-identity/src/service.rs) (`Service::shatter`, `craft`, `read_shop`, `buy`, `Snapshot::shop`, `Snapshot::trade`) |
| The kyber shard's mark | [cards/shard.rs](../crates/sjk-viewer/src/cards/shard.rs) |
| The Forge: the shop, crafting and breaking down (the shop was the Holocrons tab's overlay, `holocrons_panel_shop.rs`, until 21/10/2026) | [collection_forge.rs](../crates/sjk-viewer/src/collection_forge.rs), [collection_forge_view.rs](../crates/sjk-viewer/src/collection_forge_view.rs) |
| The Collection's History tab (31/10/2026): what came, was opened, spent and traded, `GET /v1/history` ([History](#history)) | [history.rs](../crates/sjk-identity/src/history.rs), [service.rs](../crates/sjk-identity/src/service.rs) (`Service::read_history`, `Snapshot::history`); [collection_history.rs](../crates/sjk-viewer/src/collection_history.rs), [collection_history_view.rs](../crates/sjk-viewer/src/collection_history_view.rs) |
| The Collection's Cards tab | [collection_cards.rs](../crates/sjk-viewer/src/collection_cards.rs); its tiles, filter and sets' progress in [cards/binder.rs](../crates/sjk-viewer/src/cards/binder.rs) |
| `debug_open` | [cards/rehearsal.rs](../crates/sjk-viewer/src/cards/rehearsal.rs) |
| The faces from the packs, their pictures decoded on a worker when drawn | [cards/faces.rs](../crates/sjk-viewer/src/cards/faces.rs), mounted by [sjk_packs.rs](../crates/sjk-viewer/src/sjk_packs.rs) |
| The pictures' textures, for the faces on screen | [ui_renderer/card_art.rs](../crates/sjk-viewer/src/ui_renderer/card_art.rs) |
| Reveal (`Service::reveal_cards`, `Snapshot::reveal`) | [service.rs](../crates/sjk-identity/src/service.rs), [player_identity.rs](../crates/sjk-viewer/src/player_identity.rs) |

- **Rarities' colours:** Common silver `#B4C0D4`, then the holocron tiers' green, blue,
  purple and gold for Uncommon, Rare, Legendary and Mythical.
- **A card** is drawn, not a picture: a dark plate edged in its rarity's colour, the
  holocron's gem at its heart in a ring, the rarity's name above, "Sealed" under the
  gem and the edition stamped at the foot.
- **The finishes** (19/10/2026, Sol's request) are shaded per pixel: one command a card,
  `DrawCommand::CardShine`, one quad the UI renderer shades in `ui_shapes.wgsl` (mode 6),
  with [shine.rs](../crates/sjk-ui/src/shine.rs) (`sjk_ui::card_shine_shade`) as the
  CPU reference the menu snapshots draw and the unit tests check. It cuts itself to the
  card's rounded corners and stays inside the rarity's edge; a clip (the binder's
  scrolling) cuts its quad without moving its pattern.
  - **Holo**: a faint pearl over the card and a rainbow foil whose hue runs smoothly,
    red to violet, across a slant through the light, swaying gently as foil does,
    strongest about the light and fading far from it, with a thin white glint and its
    soft halo at its middle. No bands: neighbouring pixels differ by under 3 % (the
    tests walk a line across it). Its sweep crosses the card in about four seconds,
    eased, then waits off it for about three before the next pass (the foil's faint
    rest stays).
  - **Solar** looks like the Sun blade (its colours are the hub's `saber_sun`): a corona
    burns along the card's edge, a white-gold core at the very edge, gold inside graded
    to the blade's hot and cool reds as it reaches in; its reach (about 6 % of the
    card's width, more where it flares) and brightness are stirred by two octaves of
    drifting granulation, sharp flame tongues licking further in, and three flares
    running round the card, coming and going; the blade's quick shimmer flickers it and
    sparks rise through it. Over the face only a warm wash from the top and a soft band
    of sunlight passing over (the sweep's, slower), so the picture and the words stay
    clear; its edge is gold.
  - **Shapes:** a sealed card or a face takes the same commands in every finish but
    one, the shine (before: 8 for Holo, the bands, glint and pearl, and 8 for Solar,
    the wash, three arcs, the band of sunlight, the embers and the breathing edge, each
    stroke cut to the corners into a polygon). A face is 11 shapes (13 with the lore
    box) plus 1 in Holo or Solar, against 19 (21) before.
- **Tilt (13/10/2026, Sol's request):** while the pointer rests on a card of the Cards
  tab (the one up close or one of the grid), the card turns towards it in perspective,
  up to about 9 degrees at its edges, the side under the pointer receding as if pressed;
  its light follows the pointer across it (Holo's foil and glint, a little brighter, Solar's
  band of sunlight) and a soft round glare sits under the pointer. The card is drawn as
  usual under `DrawCommand::PushTransform` (a projective transform the UI renderer
  applies to every shape vertex and text glyph drawn inside it, after clipping;
  `look::tilt_matrix`).
- **Smooth hover (20/10/2026, Sol's request):** the pointer's hold on each card eases in
  and out over about a third of a second (`tile_shine::HoverEase`): the tilt, the
  foil's brightness and the glare grow and fade with it, and the light glides from
  where its sweep was to the pointer and back instead of jumping. Each card eases on
  its own, so the card the pointer leaves settles while the next one rises. The glare
  is shaded too (`ShineKind::Glare`, one quad, a gaussian), not four stacked discs.
- **Reusable on any tile:** [tile_shine.rs](../crates/sjk-viewer/src/tile_shine.rs)
  holds the finishes for any rounded tile, not only cards: `tile_shine::push(list, rect,
  radius, kind, hover, seconds)` lays Holo or Solar (or nothing) and the glare over
  `rect`, and a page keeps one `HoverEase` and asks it each frame for each tile's
  `Hover` (`update(key, rect, pointer, seconds)`). A saber shader's or a medal's tile
  gets the same look with those two calls (and `look::tilt_matrix` for the lean).
- **A revealed card** shows its face as a trading card: its name and its rarity's
  symbol ([sjk-ui.md](sjk-ui.md#symbols)) on top, its picture in a gold-edged 6:5 window (dark until the
  picture is decoded and uploaded), a strip with its kind and set ("Meme ·
  SJK Alpha") and its edition and finish, its lore in a box, and at the foot its artist
  and its rarity (no number, 20/10/2026). The plate, the rarity's edge and the finishes' light are the sealed card's.
  A card narrower than 300 frame pixels shows only the name over its picture; below
  200, a binder tile names itself under its picture instead (see the Cards tab), while
  other small cards (the opening's finds, named beside them) show the picture alone.
  A copy whose face is not in the packs reads "Unknown card <id>" and keeps its sealed
  back.
- **The lore fills its box** (20/10/2026, Sol's request: a short lore left most of
  the box empty): the box keeps its size and place, and the lore takes the largest text
  size, up to 1.25 times the smallest, whose lines fit the room seven smallest lines
  take, tried in eight steps (`look::lore_fit`); it stands centred in the box, both
  ways. A lore of one or two lines is centred, a longer one reads from the left; one
  that needs more than seven lines at the smallest size is cut at seven. Every face
  drawn (the Cards tab, the card looked at closely, the opening) does this, and the
  shapes are the same as before, only the words move.
- **The pictures** are kept as the pack's files (15 MB for the hub's 333 faces) and
  decoded only when a screen draws their face (`faces::request`, since 25/10/2026): on
  a worker thread, cut to 6:5 about their middle if they are not, squeezed into a square
  with its mip chain, 1024 pixels at full texture detail and 512 from `r_picmip` 1 (the
  close-up's window is about 605 x 504 pixels at 1080p), and drawn back into the 6:5
  window (which undoes the squeeze). The renderer uploads two a frame and keeps no more
  than 64 (the least recently drawn go first), dropping one no frame has drawn for a
  minute; the worker keeps no copy once it is taken. 512 faces at most. Until 25/10/2026
  every picture was decoded when the packs were mounted and uploaded whether drawn or
  not, in every world: the hub's 333 faces held 1.8 GB of video memory (2.7 GB with the
  allocator's blocks) and 1.8 GB of system memory, from seconds after start. A GTX 1060
  3GB ran out of video memory in the main menu with them (the hub's crash reports,
  `label = 'SJK menu emblem'`), and they added 1.8 GB to every larger card's map
  changes; the menu on pictures at r_picmip 2 now holds 539 MiB with the packs mounted,
  as without them ([rendering.md](rendering.md#gpu-memory), `card_faces_vram_audit`).
  Card pictures are labelled `SJK card art` and medal pictures `SJK medal art` in `vram`
  and crash reports (they shared the emblem's label before), and a picture the decoder
  panics on fails alone: the worker goes on with the next (both after Slash's #206).
- **The kyber shard's mark** (17/10/2026) is a long crystal with a point at each end: a
  square turned on its point (one thick stroke) stretched upright by a transform, its left
  facet the same square again in the full kyber cyan (`shard::KYBER`) under a clip, its
  right one deeper, a light ridge between them and a glint near the top. Eight commands.
  The SJK Symbols font has it too (U+E030, [sjk-ui.md](sjk-ui.md#symbols)).
- **The credits' mark** is the sign's outline in shapes: the crossbar and four short
  bars as rectangles, the slanting leg one stroke cut level at the bar's top and the
  foot. Eight commands, sharp at every size.
- **The Holocrons tab:** the player's kyber shards and credits stand at the panel's top
  right, each mark and sum with its word under it, the chosen tier says what opening one
  gives ("Opening it gives a sealed card and 30 to 50 credits, or a saber shader (40%) or
  kyber shards (10%)."), and under the holocron's caption **Open** (the oldest of the tier
  chosen, Enter), **Open all (n)** (up to twenty of all tiers, the oldest first, A) and,
  while the hub lets the player open, **Shop** (S), which shows the Collection's Forge on
  its Shop ([The Forge](#the-forge)). Opened holocrons stay in the recent
  list, dimmed, marked "Opened".
- **Open all asks first** (21/10/2026, Sol's request): when more than one holocron would
  open, a card over the darkened tab asks "Open all 7 holocrons?" ("Open 20 of your 34
  holocrons?" past twenty, with a line that the rest wait for the next time). More than
  three open as a batch, so it says they open together in a quicker and simpler animation
  than one by one (five small strips at a time, the finds laid out at the end, without
  each card's own reveal); two or three play their full ceremony one after another and it
  says that. **Open all** (gold, focused first) and **Cancel**: Enter or Space presses the
  focused one, Tab, Left and Right move the focus, Escape cancels, a click answers. With
  one holocron, A opens it at once. There is no "don't ask again": no other confirmation
  in SJK keeps one ([holocrons_panel_ask_all.rs](../crates/sjk-viewer/src/holocrons_panel_ask_all.rs)).
  The batch stays simpler on purpose (measured 21/10/2026): the full ceremony is about 11
  seconds a holocron (unseal 1.7, strip 6, pack 3.1), over three and a half minutes for
  twenty. Its strips' 92-pixel light tiles peak at 1,633 of the canvas's 2,400 draw
  commands for twenty; drawn in full detail they would peak at 2,995 (the layout's CPU
  time stays near 0.12 ms a frame either way), for detail hardly visible at that size,
  and still without each card's pack, flip and words. The canvas limit was kept.
- **The shop** is the Collection's Forge since 21/10/2026 ([The Forge](#the-forge)); from
  17/10/2026 it was an overlay of this tab, over the darkened page as the loot table is.
- **The ceremony** is drawn by the tab over its own page, which is not drawn under it
  (text draws over every shape). Since 17/10/2026 it opens the holocron itself, as a
  loot crate opens
  ([holocrons_panel_unseal.rs](../crates/sjk-viewer/src/holocrons_panel_unseal.rs)):
  - **While the hub answers** (15 seconds at most, then "The SJK hub did not answer")
    the tab's 3D holocron ([holocrons.md](holocrons.md#the-holocrons-tab)), now the one
    asked for in its own look, glides in 0.6 seconds from its place on the page to the
    middle of the window, humming round and breathing its light, with "Opening..." under
    it. The world shows round it through a vignette: four straight fades from the
    window's edges to a clear box about it (the UI renderer's arcs are strokes, not
    fills, and take no clip, so rings could not make it).
  - **The unseal** (1.7 seconds): it spins up (to about 42 radians a second more),
    swells, shakes a few pixels at the end and its light grows to three and a half
    times the tier's; rays of the tier's colour break out of its seams, more, longer and
    faster as it charges, its name above it; then it pops and is gone in a flash of the
    tier's colour with a white heart, which fades over the strip in 0.45 seconds. It
    builds on the Force jump's charge (`jumpbuild`) and cracks open on the game's own
    holocron pick-up sound (`sound/player/holocron.wav`). The motion
    (`holocrons::stage::Motion`: where, turn, swell, light, shown) is worked out from
    the ceremony's clock alone and read by the world each frame; gone, the holocron
    grows back from nothing in its place when the ceremony ends.
  - **The crate spin** (6 seconds; since 14/10/2026): a strip of what its tier can drop,
    cards and droppable shaders, runs past a gold marker with a soft tick a tile
    (`menuroam`) and slows over 88 % of it to a stop on what the hub rolled, the last
    tile taking about two seconds; the strip is made up round the real prize and the
    stop lands anywhere up to 45 % of a tile from its middle, a close call near its
    edge but always inside it (the strip never shows a near miss that was not). Each
    tile stands over a band of its colour, as a loot crate's, and at speed its colour
    streaks across it and trails it. At rest (`sub_select`) the prize lifts and lights
    up in its colour, breathing, the others dim to a third, and a grand prize (a
    Legendary, Mythical or Solar card, a Legendary or Mythical shader) turns slow rays
    behind it.
  - **A card shows as its credits** on the strip (Sol's request, 18/10/2026): a card is
    the commonest and least wanted prize, so its tile is a stack of three credit coins
    with the amount ("+42 credits"), washed from above in the holocron's colour, and the
    card tucked face down in its top right corner, turned a little, its back in the
    holocron's colour with the holocron's gem (edged in gold for Solar, a rainbow across it
    for Holo), so a strip of cards is never grey. The prize's amount is what the hub gave;
    a neighbour's is a whole number in the tier's credits range (Uncommon 10 to 20, Rare 30
    to 50, Legendary 80 to 120, Mythical 250 to 350, from the loot tables), the same each
    time for the same holocron, drawn apart from which neighbours come up. The tile's edge
    and its band stay in the card's rarity colour (gold for Solar), and a grand card keeps
    its rays. No shards on the tile: they show on the pack once the strip stops.
  - **A pile of kyber shards** (17/10/2026) is a tile of its own on the strip: "Kyber
    shards", three crystals in a kyber glow and the tier's range ("25 to 50"), or for the
    prize how many ("+37"); stacked, the crystals alone. Its entrance is the burst, then
    three crystals growing in in a kyber glow and the shards counting up beside the shard
    mark, then "37 kyber shards", "From a Legendary Holocron", what shards craft and how many
    the player holds now. Among many opened, the finds show it ("37 kyber shards", "For
    crafting"), the shards counted beside the credits at the top, a pack's shards with them
    (a card's note "+24 credits, +3 shards"). A client from before the
    piles skips such an opening (nothing it knows: an opening of piles alone says "Nothing
    this client knows came out."); the profile carries the shards either way.
  - **The entrance:** the burst in its tier's light (rings, turning rays, the gem
    flaring away), then the card's pack (18/10/2026, 3.1 seconds): the credits' tile, as
    the strip showed it, rises where the strip stopped with the card tucked in its corner
    and its credits counting up; it slides aside to the left while the card slides out of
    its corner to the middle, growing and straightening, and flips over (it narrows
    edge-on and opens on its front, its rarity and finish or its face); a Holo, Solar,
    Legendary or Mythical card flips into a flourish, a glow, a ring running out and rays
    turning (gold for Solar, the rainbow for Holo, its rarity's colour for a plain
    Legendary or Mythical);
    then the kyber shards' tile slides out on the right and counts up (none while the
    hub's economy is off). Then its words: "Rare card, Holo", "Alpha Edition. From a Rare
    Holocron", what the finish is, that it is sealed, "1 of 3", Next or Done. A shader instead of a card shows its
    blade framed in its tier's colour, its name, "Common saber shader. From a Rare
    Holocron", what it looks like and how many the player holds. A Solar blade card's
    blade (20/10/2026) comes with its card: its tile (the blade framed in its tier's
    colour, "+Sun blade", "saber shader, new" or "x2") slides out on the right just after
    the shards' (under it when both came), and in the card's words "...and the Sun blade
    drops with it! Yours now: wear it in the Collection." (or "You hold 2 of it now.") in
    the blade's colour takes the place of what the finish is. A Solar finish or a
    Legendary or Mythical card plays the medals' fanfare, another card the unlocks' chime.
  - **Opening more than three**
    ([holocrons_panel_batch.rs](../crates/sjk-viewer/src/holocrons_panel_batch.rs)):
    the first holocron cracks open ("12 holocrons"), then their strips run five at a
    time, stacked, each named by its holocron's tier at its left and marked in its
    colour, each stopping 0.35 seconds after the one above (3.6 seconds for the top);
    the next five follow on their own ("Holocrons 6 to 10 of 12"). The stacked tiles are
    light: a plate washed from below in the prize's colour, a card's credits (two coins,
    the amount and the card tucked in its corner, its plate alone, in the holocron's
    colour, with three rainbow bands for Holo; 18/10/2026) or a shader's blade. A grand
    prize plays the fanfare as its strip stops. Then everything found is laid out, the
    finest first, ten to a row, each coming in on its beat, with its name and its
    credits or how many the player holds (a Solar blade card adds "+Sun blade" under its
    credits, in the blade's colour), the credits they brought together, and Done.
  - **Grand finds** are told to everyone: a hub that sends the feed's `finds` (hub branch
    `feat/grand-finds`, not deployed yet) relays a Legendary, Mythical or Solar card, or a
    Legendary or Mythical shader, to every SJK player as a chat line, "Sol opened a
    Mythical Holocron: Phoenix blade!" ([holocrons.md](holocrons.md#chat-lines)).
  - Enter, Space, Right or a click skip to the burst (to the strips' stops, then the
    next five, then the finds, for many), finish the entrance, then go on; Escape ends
    it (what was opened is the hub's already). A refusal says why ("That holocron was
    already opened."; a hub from before opening: "This SJK hub cannot open holocrons
    yet.") and the profile is read again soon.
- **The Cards tab** (the Collection's seventh), a binder since 19/10/2026 (Sol's
  request): every card the packs have a face of, found or not, so the collection reads
  as a binder with its gaps. Nothing in it knows a set: sets, cards, rarities and kinds
  come from the faces.
  - **The headline** says "5 of 13 cards found", the line under it each set's progress
    ("SJK Alpha 5 / 13", four sets at most, then "2 more sets"); with no card yet,
    "Open a holocron on the Holocrons tab" after it. The kyber shards and credits stand
    at its right. Without faces in the packs it says how many cards are held, as before.
  - **The tiles** (140 frame pixels wide, six to a line, three lines shown from y 340;
    since 20/10/2026, Sol's request: seven at 122 were too small to tell apart):
    first the sealed copies, one tile per rarity, the rarest first (their back in the best
    finish held); then every card by set and number, one tile whatever the copies held,
    in the light of the best finish held (Solar, then Holo, then Normal) with its edition;
    a card not found yet is greyed: a grey plate and edge, its picture darkened under a
    grey veil, its words quiet, no finish light (`look::Card::dim`). Each card's tile
    names it under its picture (`look::Card::label`, 20/10/2026), centred on up to two
    lines that end in an ellipsis when the name runs on, with its rarity at its foot
    (its symbol and name), in grey for a card not found yet, so tiles whose pictures look
    alike (the placeholder art) tell apart. Last, copies of a
    card whose face is not in the packs ("Unknown card <id>", the sealed back). A tile
    holding more than one copy carries a pill at its top right, "x3", edged gold when its
    best is Solar. More lines than three scroll: the wheel a line a notch, the arrows keep
    the chosen tile in view, Page Up and Page Down move three lines (the column kept),
    Home and End go to the first and last tile; a bar at the grid's right shows where,
    and under its right end "Rows 4-6 of 9" (both only while it scrolls; the keys' hint
    then adds PgUp PgDn). Reveal and that line stand under the grid at y 958.
  - **The filter** above the tiles: All, Owned and Missing, then a chip each for the
    rarity ("Rarity: Legendary", in its colour), the kind ("Kind: Meme", only kinds the
    faces have) and the finish held ("Finish: Holo": a copy in it is held), each a click
    on to the next value and back to Any; O, T, K and F do the same (with Shift, back).
    They combine; a kind leaves out the sealed and unknown tiles. "N of M cards" at the
    row's right counts the cards shown of the packs'; "No card matches these filters."
    when none does. A filter's change goes back to the first tile.
  - **Up close** on the right (437 frame pixels wide, so a face's lore reads at 1080p),
    its top under the row of tabs: standing in the row, it took the Forge tab's clicks
    when a large text style stretched the row under it (Kushina's report #124,
    23/10/2026):
    the tile chosen, its name (in its rarity's colour, gold for Solar), the copies held
    ("3 copies: 1 Solar, 2 Normal · Alpha Edition") or "Not found yet", the rarity, kind,
    set ("Legendary · Meme · SJK Alpha"; a sealed tile: that a card
    is picked for each when it is revealed), then when the first was found, or for a card
    not found yet how it is obtained with its odds. A card not found yet shows greyed.
    A click on it looks at it closely.
  - **Every finish held** (01/11/2026, Sol's request: not only the best): when the
    copies of the tile chosen are in more than one finish, chips Normal, Holo and Solar
    ("Solar  x1", one not held dimmed and inert) stand for the copies' line up close and
    under it looked at closely, the finish shown ringed gold; a click on one, or V
    (Shift+V back), shows the card in that finish (and that copy's edition). Choosing
    another tile shows its best again; the tiles keep their best finish.
  - **Looking closer** (20/10/2026, Sol's request: the cards should be easier to see):
    Enter, Space or I, a click on a tile or on the card up close shows the chosen tile's
    card large over the darkened tab (660 frame pixels wide, nearly the frame's height,
    at its left), tilting under the pointer in its finish's light, and at its right
    where it stands ("Card 12 of 344", among the tiles the filter shows), its name, the
    copies held, what it is and when the first was found or how it is obtained. The tab
    is not drawn under it, and the header's words are covered
    (`MenuCanvas::cover`). Left and Right (or Up and Down, Page Up and Page Down, the
    wheel) go to the previous or next tile, stopping at the ends, Home and End to the
    first and last; Escape, Enter, Space, I or a click anywhere put it back, on the tile
    it was left on (the grid scrolls to it). The keys' hint then reads "Left Right
    previous, next", "V finish" when more than one is held, and "Esc put back".
  - **Hovering** a tile shows its hover card ([sjk-ui.md](sjk-ui.md#hover-cards)): how a
    card of its rarity comes ("From holocrons: about 1 in 136 holocrons found holds a
    Legendary card (0.734%), one of the 9 Legendary cards."), what it is ("Legendary
    card. Meme, SJK Alpha."), then the copies by finish and edition and "First
    found 12/10/2026", or "Not found yet".
  - **Reveal N sealed cards** (or R) stands under the grid when the hub lets the player
    open holocrons and the packs have faces of a sealed copy's rarity; it says
    "Revealing..." until the hub answers, then a line beside it says how many were
    revealed or why not, and the blade Solar blade cards dropped ("1 card revealed, and
    the Sun blade drops with it!", or "3 cards revealed, and 2 blades drop with them!"). The arrows (Left and Right a tile, Up and Down a line), Tab and
    the pointer choose; Enter looks closer. The tab's count in the row is the copies held.
- **The ceremony** shows a card the hub picked with its face, bigger (400 wide) and
  higher, its name as the title, "Legendary, Holo. Alpha Edition. From a Mythical
  Holocron" and its kind and collection ("Meme, SJK Alpha") in place of the sealed line.
- **The player card's** rarest holocron gem counts opened holocrons too, as the
  counts now hold only the unopened ones.
- `debug_open <tier|all> [x<count>] [normal|holo|solar|shader|shards]` plays the ceremony
  with made-up cards (or shaders, or piles of kyber shards) on the Holocrons tab; nothing is
  sent or kept. A card comes as a pack, with made-up shards from the tier's card shards. A rarity the packs have
  faces of shows one of them, as the hub would pick.

## History

Since 31/10/2026 (Sol: "a general inventory history: what they got, opened, what they
spent credits and shards on, and the drops"), the Collection's last tab, History, lists
the player's economy as the hub keeps it, newest first.

The hub answers `GET /v1/history` (signed as `GET /v1/holocrons` is; its `PROTOCOL.md`,
"History") with `{"lines":[...],"more":bool,"before":<unix s>}`: a page is 100 lines
plus the rest of its oldest second (one second's lines are never split), and while
`more` is true `GET /v1/history?before=<before>` brings the older ones. A line has `at`
and `kind`:

| Kind | What it is | Its fields |
| --- | --- | --- |
| `came` | a holocron or kyber crystal came (not through a trade: the trade's line says it; one found then traded away stays) | `holocron`, `tier`, `source` (`play`, `staff`, `achievement`, `bought`, `box`, `reward`), `note`, `price` (what a bought one cost, its row's amount), `traded` |
| `opened` | the player opened one | `holocron`, `tier`, `credits`, `shards`, `card` (`card`, null while sealed; `rarity`, `finish`; or null), `unlock` ("" for none) |
| `credits` | a credits ledger line no other kind covers | `delta` (signed), `reason` (`staff`, `reward`, `mystery_box` with `note` `explode_gift`), `note` |
| `shards` | a kyber shard ledger line no other kind covers | `delta`, `reason` (`shatter`, `craft`, `shop` with the shader's id as `note`, `staff`, `reward`), `note` |
| `box` | a mystery box opened | `outcome` ([mystery-boxes.md](mystery-boxes.md)), `price` (0 a free roll), `won`, `shards` |
| `trade` | a finished trade | `trade`, `with` (`name` with colour codes, `verified`), `gave` and `got` (the trade items of `GET /v1/trades`, `value` included) |

The client skips a line of a kind it does not know, or one it cannot read, and takes a
missing field as empty, so the hub can add kinds. No key id comes ([No keys in
game](identity.md)). An unknown key gets 403 `not_registered`; a hub from before the
history answers 404, and the tab says "This SJK hub keeps no history yet."

The tab reads the newest page each time it is shown (and on R), and Load older, the
list's last row while the hub has more, the next page (`HistoryState::take` adds a page
only when it follows what was read, so a second press adds nothing twice). A row is one
line: a chip of its kind (Drop, Opened, Credits, Shards, Box, Trade) in the tier's colour
(holocrons, crystals) or the kind's, a sentence ("Opened a Rare Holocron: card Sol
(Holo), Sun blade", "Bought a holocron", "Shattered Sun blade", "Mystery box: double,
paid 100, won 200", "Traded with Nib: gave a Rare Holocron, got Sun blade"), what it moved
("+120 credits", "-400 credits", "+15 shards", gains green and spends red) and when, in
this PC's time (`dd/mm/yyyy HH:MM`). Consecutive openings of one second (Open all) are one
row, "Opened 37 Holocrons (5 Rare, 32 Uncommon): 30 cards, 7 unlocks" with their totals,
and so are holocrons that came in one second the same way; Enter or a click opens it on
its lines. The filter (All, Drops, Opened, Credits, Shards, Trades, Boxes; Left and Right)
works on what was read: Credits and Shards keep every line that moved them, openings and
boxes included. The rows' words are made once for what was read, the filter and the rows
opened, not each frame. World shot: `world_shot::collection::duel6_sjk_collection_history`.

## Planned, not built

1. The rest of SJK Alpha: Common, Uncommon and Rare cards (only Mythical and Legendary
   ones exist; drafts for the others wait for art), so every sealed copy can be revealed; a reveal ceremony (the copies turn
   over one by one, as an opening's cards turn in).
2. Deploying the economy ([Shards and the shop](#shards-and-the-shop)) and the Shaders
   tab's Shatter and Craft. The Staff page's credits and shards control is built
   (18/10/2026); shards wait for the hub's `/v1/staff/shards`.
3. Set completion feeding achievements the hub counts.
4. The Beta season: holocrons stamped with their season, guaranteed prizes and bonus rolls, pity, no duplicate faces ([Seasons](#seasons)).

Suggested first sets, about 20 to 30 cards: Places (planets, the Jedi Academy, the
game's maps), Personalities (figures of the lore), Jedi Academy (Force powers,
saber forms, hilts, moments of the game), Community (players and servers, with their
consent).

## The art and the rights

Sol makes the pictures, with AI tools, in one shared style. Cards of the lore draw
on Lucasfilm and Disney properties and Jedi Academy itself, and the credits' sign is
Lucasfilm's, so the system stays strictly non-commercial. AI-made art is generally
not protected by copyright, so card packs cannot carry the "all rights reserved"
claim the blade skins' art does; the credits say the pictures are AI-made. Community
cards use a player's name and picture, or a stylised emblem, only with their consent,
never an AI likeness of a real person's face, and staff can take one down at the
player's request.

## Privacy

A person's cards and credits are public like their medals: anyone can read a
profile. The hub stores which copies a person holds and when each was obtained, and
every change to their credits with its reason.

## Checked and not checked

Verified (12/10/2026, Windows 11): the hub's tests on branch `feat/holocron-opening`
(odds tables, rolls, the API's opening of one and twenty, refusals that change
nothing, merges moving cards and credits, linked keys, staff credits); the client's
`cargo test --locked --workspace` (the wire, the worker's opening and refusals, the
lists, stacks and credits' words, the mark's and a card's fixed commands, the
rehearsal's words, the tab's keys, the ceremony's flow, timeout and room on the
canvas in three windows, the Cards tab's states and room, the renderer's cut of a
stroke). World shots `world_shot::cards_shots` (off-screen renders on duel6, made-up
data) were looked at: the tab with Open and the credits, a Holo card's entrance
moment by moment, each finish, the wait, a refusal, the Cards tab at 1080p and 4:3
and with none. Not tried: against the real hub (it is not deployed), in a game, the
keys and clicks in a running client, the motion (the shots are stills).

The economy (17/10/2026, Windows 11, prepared): the hub's tests (the tables, the trade
read from the catalogues, Paris's day across summer time, the offers stable and different
by day and secret, shattering never the last copy, crafting and buying once with enough,
two crafts with shards for one, refusals that change nothing, the gate, a pile from an
opening, staff and operator changes with their undo, the shop's secret kept across a
reopening, the API's shapes and errors); the client's unit tests (the wire with and without
the new fields, the service's trades and shop read, the tables' copy and the hub's, the
hooks' choices, the strip's shard share, the odds' figures, the shop's keys, confirm,
countdown and room in four windows). World shots `duel6_sjk_economy_shots` and
`duel6_sjk_economy_shop_windows` (off-screen, duel6, made-up data, the hub's packs mounted)
were looked at: the tab with both balances and Shop, the loot table's shards column, the
shop with a Confirm, while reading and after a purchase, in 4:3, 21:9 and 4K, a pile at
rest on the strip, coming in and its words, twelve opened with three piles at rest and laid
out, the Cards tab's balances; `duel6_sjk_card_pack`: a sealed Rare pack sliding out,
counting and with its words, one with a face, and the pack in 4:3. Not tried: against a running hub, in a running client, a
purchase's or a turned day's flow live.

A card's credits on the strip (18/10/2026, Windows 11, `personal/credits-roll`): unit tests
for the strip's credits (the prize's kept, every neighbour's in its tier's range and not
all alike, in every tier) and every moment of a single opening and of twenty at once,
twenty Holo cards among them, fitting the canvas in three windows. World shots
`duel6_sjk_card_pack` (the hub's packs mounted) were looked at: an Uncommon strip running
and at rest, a Rare and a Mythical one at rest (a Legendary Solar card's rays), the Rare
Holo pack rising, sliding aside as the card slides out, edge-on in its flip, flipped, its
rainbow flourish, a Solar one's gold flourish, the shards counting, its words, one with a
face and the words in 4:3; `duel6_sjk_crate_open` and `duel6_sjk_economy_shots` for the
stacked strips' light tiles at rest (their peak, 1 569 of the canvas's 2 400 shapes). Not
tried: in a running client (no motion seen; the flip plays no sound of its own).

The crate opening (17/10/2026, Windows 11): unit tests for the 3D holocron's motion
(glide, hum carried into the unseal, spin-up, swell, light, shake bounds, gone in the
flash), its handover (the one asked for, then the one opened, back in its place after),
the strip's stop always inside the prize's tile in both strip sizes and its speed curve,
the finds' order and grand ones, the batch's moments and skips, Open all's flow, and every
moment of a single opening (every tenth of a second) and of twenty at once (every
quarter) fitting the canvas in three windows. World shot `duel6_sjk_crate_open`
(off-screen, duel6, made-up prizes, no blade-skin packs so shader tiles show plain
blades) was looked at: the wait, the unseal early, charged and flashing, the strip out
of the flash, fast, crawling and at rest, a Solar Legendary card and a Mythical shader at
rest, and a batch of twelve cracking open, running, stopping, at rest, its last round and
its finds coming in and laid out. Not tried: in a running client (no motion seen, no
sounds heard: the cue files are checked to exist in the retail `assets0.pk3` only), with
the hub's packs, at 4K or 4:3 in a shot (the canvas test covers them).

The binder (19/10/2026, Windows 11, `personal/card-binder`): unit tests for its tiles
(sealed first by rarity, every card in set and number order found or not, unknown cards
last, the best finish, the copies' breakdown by finish and edition, each set's progress),
its filters (combined, cycled both ways, kinds from the faces), the tab's room on the
canvas at 1080p, 4K and 4:3 with up to 65 tiles, the arrows, the wheel and a filter, and
a tile's hover card. World shot `duel6_sjk_card_binder` (off-screen, duel6, made-up
holdings, the hub's `sjk_cards` pack mounted) was looked at in 1080p and 4:3: all the
cards with a hover card on one held in two finishes, Owned, Missing Legendary ones with a
hover card, Finish Holo, and a missing card chosen. Not tried: in a running client, more
cards than three lines (the unit tests scroll; SJK Alpha has 13), the keys and the wheel
live.

The finishes and paging (19/10/2026, Windows 11, `personal/card-finishes`): unit tests for
the shine (nothing outside the rounded card, Holo's foil without steps, Solar's corona on
the edge and the face nearly clear, its motion), the shader validated by naga, a card's
fixed commands, and the binder's Page Up, Page Down, Home, End and its rows line. World
shots looked at (off-screen, duel6, the hub's `sjk_cards` pack mounted):
`duel6_sjk_card_finish_motion`, a Solar and a Holo card up close with the tile chosen in the
grid at three moments each, and a Solar card at rest in the opening at three moments;
`duel6_sjk_card_binder_many`, a made-up catalogue of 59 cards in three sets (SJK Alpha's and
two made-up, reusing its pictures) scrolled to a tile in the middle, in 1080p and 4:3, and
174 cards in six sets for the progress line ("2 more sets"). Not tried: in a running client
(the motion between the stills, the frame time with a full binder of Solar tiles; each
shine is one quad, about 470 by 660 pixels up close, a few noise lookups a pixel).

The blade cards (20/10/2026, Windows 11, `personal/blade-cards`): unit tests for the card
file's `blade` and kind, the wire's opening with a card and a shader together and a reveal
with and without one, the prize a Solar blade card makes and its line, the reveal's words,
and every moment of a single opening with a blade card's pack (its shards and blade tiles)
and of twenty with such cards fitting the canvas in three windows. World shot
`duel6_sjk_blade_card` (off-screen, duel6, the hub's packs mounted, a made-up "Sun blade"
card borrowing SJK Alpha 000's picture, as the packs have no blade card yet) was looked at:
the blade card's pack with its shards and blade tiles sliding out, its words in 1080p and
4:3, five opened at once laid out, and the card up close on the Cards tab with its hover
card. Not tried: against the hub's change, in a running client.

The card pages (20/10/2026, Windows 11, `personal/card-pages`): unit tests for the lore's
fit (a short lore at the largest size and centred, a longer one smaller and always within
the box, an overlong one cut at seven lines, a larger player text size growing it less), a
small card's label (only when asked, two lines, its rarity and no number, the ellipsis),
no number on a big face, and the tab's
keys with six to a line and looking closer (Enter, the arrows to the last tile, Escape
putting it back without closing the Collection, the view fitting the canvas in three
windows). World shots looked at (off-screen, duel6, the hub's `assets/dist` packs with
their 344 cards mounted), before and after: `duel6_sjk_card_binder` in 1080p, 4:3 and
4K (the binder, its filters, a hover card, and looking closer at the Toxiee card, a card
with a two-line lore and one with a six-line lore) and `duel6_sjk_card_binder_many`, and
`duel6_sjk_card_faces` (the opening's card with its fitted lore). Not tried: in a running
client (the clicks, the wheel and the keys there), the 150 % display scale itself (the
layout follows the window's height only, so 4K draws the 1080p layout twice as sharp).
