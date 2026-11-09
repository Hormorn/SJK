# Dedicated server

`sjk-server` is SJK's native headless server (the `sjk-dedicated` crate). It does
not launch another engine. Build it alongside the client as described in [development.md](development.md).

## Local game

Use an unused loopback port for development:

```sh
./target/release/sjk-server \
  --game-data /path/to/GameData \
  --map mp/ffa3 \
  --bind 127.0.0.1:29071 \
  --hostname "SJK local" \
  --gametype ffa \
  --team-auto-join \
  +set dedicated 1
```

Then connect the client to `127.0.0.1:29071`. `dedicated 1` selects LAN operation
without master-server advertising. Keep automated checks local and free of human
players. For remote hosting, explicitly choose an externally reachable bind
address and open the chosen UDP port.

Always supply `--game-data` for a playable map. Without it the process can
advertise a map name but does not load that map's content.

## Siege class catalogues

Installed Siege content follows JoF EJK #253's expanded catalogue capacities:
the first 1024 `.scl` files and 256 `.team` files in load order are considered.
A theme has up to 16 sequential `classN` slots (`class1` through `class16`),
stopping at the first missing key; missing class references retain their slot.
Oversized/refused files still occupy a global listing slot. The named limits
live in the `sjk-game-jka` Siege compatibility parser; owned registries remain
vectors. The client class selector uses the same capacities in both UI styles,
so it does not exceed those catalogue capacities. These content limits
change no protocol-26 field, class loadout, movement or combat timing.

## Options and configuration

The authoritative option parser and usage text are in
[main.rs](../crates/sjk-dedicated/src/main.rs).

| Option | Meaning |
| --- | --- |
| `--map NAME` | Initial map, such as `mp/ffa3` |
| `--maps NAME,NAME` | Rotating list of subsequent maps |
| `--gametype NAME` | `ffa`, `holocron`, `jm`, `duel`, `powerduel`, `team`, `siege`, `ctf`, `cty` |
| `--fraglimit N`, `--timelimit MINUTES` | Match limits |
| `--wire-clients N` | Protocol-26 client slots, 0–32 |
| `--peers N` | Native player capacity, separate from wire slots |
| `--team-auto-join` | Place newcomers in play without the initial spectator selection |
| `--home DIRECTORY` | Writable server config home |
| `--set NAME VALUE` | Set a console variable |
| `--quit-on-eof` | Stop when a supervising parent's stdin pipe closes |
| `--cheats` | Enable the native server's development commands |

These are implemented option surfaces, not a guarantee of complete parity for
every game type or map. See [status.md](status.md).

`+set` and other `+COMMAND` arguments are supported. Startup applies `+set`
values before loading `mpdefault.cfg`, `sjk_server.cfg` and `autoexec.cfg`, then
runs the command-line `+` commands in order. With `--home`, `exec` checks its
`base` directory first and archived variables are saved in `base/sjk_server.cfg`.
Without a home, this archive is not written.

Type commands such as `status`, `map mp/ffa3` and `quit` on stdin. Remote console
commands require an explicitly configured `rconpassword`; never commit that
password. The console implementation lives in
[bridge_console.rs](../crates/sjk-dedicated/src/bridge_console.rs) and the server's
[command buffer](../crates/sjk-dedicated/src/command_buffer.rs).

Bots retain their slots and bot identity across map changes and `map_restart`.
They enter the new map immediately because they have no network handshake.
Player session data survives, while transient state and entity handles are reset
for the new world; saber entities are allocated from the new map's pool.

The [networking page](networking.md) explains the boundary between native entity
ownership, game behavior and the legacy endpoint.

## JA+ mode

`g_mod japlus` starts the server as a JA+ 2.4 Build 7 server instead of the base game
(`base`, the default). Set it on the command line (`+set g_mod japlus`) or in
`sjk_server.cfg`; it is read once, at start, as a JKA server's game module is chosen by
its `fs_game`, and a later change waits for the next start (the console says so). An
unknown name runs the base game and says which mods there are. The client's
[Create game](sjk-ui.md#create-game) has a Server mod row (Base game or JA+) and starts
its server with `--set g_mod <id>` (`base` for the base game); its `devmap` always
runs the base game.

JA+ is closed source. The reference is VisionJK
([github.com/Blackwolf1337/visionjk](https://github.com/Blackwolf1337/visionjk),
GPL-2.0), Blackwolf's rebuild of the JA+ 2.4 B7 server module from its Linux binary,
compared with the original in random games. SJK ports its behaviour, not its code.

What the mode does so far:

- **It calls itself JA+.** The server info has `gamename` `JA+ Mod v2.4 B7`, `V`
  `2.4B7` and `jp_cinfo`, and `getinfo` reports the mod folder `japlus`, so server
  browsers, SJK, EternalJK and the JA+ plugin all take it for a JA+ server. SJK's
  own `sjk 1` key stays. The client's tables of server families treat it as a JA+
  server, not as family `sjk` (`sjk_mod::known_servers`).
- **JA+'s settings.** The 187 `jp_*` cvars of JA+, at JA+'s defaults, so a JA+
  `server.cfg` loads unchanged. Only those in the table below do something yet.
- **JA+ movement, as clients predict it.** Every player moves by the shared movement's
  JA+ rules (`sjk_game_jka::pmove_japlus`), selected from the same `jp_cinfo` bits the
  clients read, so no client is pulled back for a rule it was told about: the rules
  every JA+ server plays (free taunts, the held kicks and kiss, the staff's standing
  front kick, melee that an attack with the item key does not cancel, wall runs from Force flips) and the ones its
  settings switch. A changed setting reaches the server info and every player's moves
  at once.
- **JA+'s Force rules.** Every Force update and throw, the NPCs' too, runs by the rules
  the mod's settings select (`sjk_game_jka::force_rules`, the `ServerMod::force` hook);
  the base game's are the default. They change what the server sends, so no client
  predicts them, but the grip speed, which every JA+ client reads from the server info.
- **No OpenJK animation fixes.** JA+ is built on the 1.01 game code: `CS_LEGACY_FIXES`
  is 0 and the server runs none of them.
- **JA+ saber damage**, for players' and NPCs' blades
  ([saber_damage_japlus.rs](../crates/sjk-game-jka/src/saber_damage_japlus.rs)). Only the
  server deals damage, so nothing here changes what clients predict. SJK's server always
  runs `d_saberSPStyleDamage 1` (the base game's own value), so `jp_alterDMG` alone picks
  JA+'s damage: 0 keeps the SP-style damage of the base game, any other value gives each
  saber move the damage of JA+'s move table, ramped over its animation as JA+ does, and
  the specials' extra weights (JA+'s own moves, such as its new spin DFA, are not played
  here). As on JA+ with the SP style, each of a swing's traces that strikes adds its
  move's damage to the frame's blow, so a fast swing through a body sums several. The
  multiplayer style JA+ plays with `d_saberSPStyleDamage 0` (its `jk2.cfg` preset: box
  traces, blocks without a saber met, one hit per 100 ms) is not built. On every JA+
  server a player under the chat bubble cuts nobody, and a blade that meets something it
  cannot hurt (a wall, a saber) draws `jp_reduceSaberBlock`'s chance of passing through.
  The ramp is worked out in `float`, as an SSE build of the code does: where the original
  Linux module's x87 FPU truncates a product of exactly 0.7 one point lower (110 at 0.7
  is 76 there), SJK deals 77.
- **The `/kill` frag.** A `/kill` (or any suicide) within five seconds of being pushed or
  hit gives the frag to whoever pushed or hit last, as a fall does; `/kill` does nothing
  during a duel, nor in team games without `jp_allowTeamKill`.
- **JA+ private duels** (`sjk_game_jka::duel::DuelRules`, from VisionJK's `g_cmds.c`
  `Cmd_EngageDuel_f`, `g_active.c` duel think and `japlus/g_japlus_dim.c`):
  - *Isolation.* With `jp_DuelAlpha` 0 or more, in a free-for-all or team deathmatch,
    duellists and everybody else do not block, hit or push each other: every trace and
    move of a player leaves out the players it may not touch (`G_TempUnlink`,
    `JP_Trace`), so missiles fly through them too. The challenge's own line still stops
    at a duellist, as JA+'s does. SJK predicts it on JA+ servers from the same server
    info ([networking.md](networking.md)), as the JA+ plugin does.
  - *The challenge.* Any weapon challenges, the saber is needed to accept (both
    sabers in hand and not thrown); none with a jetpack or the cloak ("You have to
    disable your jetpack in order to duel."), in a melee grapple or from a spectator;
    the game types it is refused in are said (`@@@NODUEL_GAMETYPE`), and a duellist
    waits 2 seconds after a duel (`@@@CANTDUEL_JUSTDID`), unless challenged.
  - *The start.* Both get 100 health, a full Force pool and `jp_duelStartArmor`; the
    sabers go off and stay off when the duel begins (the duellists light them).
  - *`engage_FullForceDuel`*: a challenge whose duel lets the Force work between its two
    (every power usable, powers on each other, any damage from the opponent), told as
    JA+ tells it ("is challenging you in Full Force"); the acceptor takes the
    challenger's kind. The duel key's challenge is an ordinary one.
  - *The end.* The winner's health and armour before it is healed are told as
    `jp_endDuelBroadcastMSG` says; it is healed and protected as before, and keeps 25
    armour.
- **JA+'s emotes and player commands** (the `ServerMod::command` table,
  [commands.rs](../crates/sjk-server-mod-japlus/src/commands.rs); carried out in
  [bridge_mod_commands.rs](../crates/sjk-dedicated/src/bridge_mod_commands.rs), from
  VisionJK's `japlus/g_japlus_cmds.c`, `g_japlus_info.c` and `g_japlus_tele.c`). They come
  before the game's own commands, as JA+'s do, so `jetpack` is JA+'s and not the cheat;
  at the intermission each is refused (`@@@CANNOT_TASK_INTERMISSION (name) `). Every
  answer is JA+'s, word for word, its typos too.
  - *The 27 emotes*: `amkneel`, `amdie`, `amdie2`, `amfinishinghim`, `ampower`, `amnod`,
    `amshake`, `amhiltThrow1`, `amhiltThrow2`, `amcomeon`, `amwon`, `amhello`, `sleep`,
    `amatease`, `amsurrender`, `amsit` to `amsit4`, `ambeg`, `amwait`, `amhips`,
    `amvictory`, `amneo`, `ambreakdance`, `ambreakdance2`, `amnoisy`. Each plays a retail
    `_humanoid` pose as a taunt (`HANDEXTEND_TAUNT`) for a living player with free hands,
    not lying, getting up, swinging or on the jetpack; `amdie` and `amdie2` only in a
    free-for-all outside a duel, `amfinishinghim` lights a holstered saber first,
    `amvictory` needs the saber, lights it and plays the gloat's taunt event. The sitting,
    kneeling and lying poses are refused in team games ("this Emot is not compatible with
    this gametype!.").
  - *Held until a button*: those poses, and meditation, keep the player still with the
    view free until an attack, a Force button, the hook's (`+button12`) or a jump, which
    plays the pose's own get-up (`sjk_game_jka::pmove_japlus_emotes`, VisionJK
    `bg_pmove.c:10789-10823,11125-11196`). SJK's client predicts this on every JA+ server
    ([networking.md](networking.md)). The JA+ plugin does too; for the clients that do not,
    the server freezes the body (`PM_FREEZE`) as `jp_fixEmot` says: `amatease`, which no
    client holds, always.
  - *`amhug`*: with the player 40 units ahead along the view (not duelling, not in a
    grapple), both sabers put away, the two turn to each other and play the retail hug
    (`BOTH_HUGGER1`, `BOTH_HUGGEE1`). Not in team games.
  - *`amkiss`*: JA+ kisses with its own animations (`jp_newGLAAnims`), which SJK does not
    ship, so it answers as JA+ does without them ("the new GLA animations are not loaded
    on this server or the admin disallowed them!.").
  - *`amflip`*: on the ground, the saber in hand turns on or off with its style's
    show-off (the draw, the holster, the staff's and the duals' show-offs), JA+'s on-sound
    both ways.
  - *`amknockmedown`*, *`amdropsaber`*: in a free-for-all only, the player falls (700 ms,
    a hop and the Force jump's sound) or its saber flies out of its hand; refused in a
    duel, and as JA+ sends it (a command of its own, not a print) where the setting is off.
  - *`jetpack`*: takes the jetpack item (when it is off) or gives it on the ground; never
    in siege, the duels and capture the flag, nor in team deathmatch, nor to a Jedi of
    Jedi against mercenaries.
  - *`amcolor R G B`*, *`sabercolor N R G B`*: the model's tint (`char_color_*`, 768
    keeps the model's) or a saber's RGB colour (`cp_sbRGB<N>`, `color<N>` 6) written into
    the player's userinfo and published; `sabercolor` only for a client naming the JA+
    plugin 1.1 or later in `cjp_client` (SJK sends `1.4 SJK`).
  - *`saber [saber1 [saber2]]`*: the sabers changed while playing, the second `none` when
    not named, with the stance they bring (`G_UpdateSaberFromUserinfo`,
    `G_ClampSaberLevel`); not in a duel, nor in siege and the duels, nor swinging, and no
    more often than `jp_allowSaberSwitch` milliseconds.
  - *`ammove`*, *`amDMGs`*, *`duelWhois`*: JA+'s moves list (JA+'s own, moves SJK does not
    play included), its damage settings, the duels being fought.
  - *`refuseTele`*: toggled and said; admins' `amtele` then refuses to move the player.
  - The server console's `serverTime` and `modVersion` (`ServerMod::console_command`).
- **JA+'s grappling hook** (`+button12`; `sjk_game_jka::japlus_hook`, the server's part in
  [bridge_hook.rs](../crates/sjk-dedicated/src/bridge_hook.rs), from VisionJK's
  `japlus/g_japlus_hook.c` and its calls in `g_active.c`, `g_main.c`, `g_missile.c` and
  `w_saber.c`; the `ServerMod::hook` hook, none on the base game):
  - *The hook.* Held down, the key fires a missile from the muzzle along the view at
    `jp_hookSpeed`, at most once per `jp_hookFloodProtect` ms, with 10 fuel or more, not
    in the 1.5 s after a spawn, nor duelling, dead, swinging, riding, gripped, in a melee
    grapple, flying or with the jetpack on; the throw's pose (`BOTH_RESISTPUSH`) and
    sound. A client plugin user's (`cjp_client` a version of 1.0 or more, SJK's own
    `1.4 SJK` included, and every bot) is the stun baton's missile the plugin draws the
    rope to (SJK's client draws it, [grapple_rope.rs](../crates/sjk-viewer/src/grapple_rope.rs)),
    with a twin pistol bolt beside it; anyone else's is a pistol bolt.
  - *Taking hold.* It holds where it strikes the world, or the middle of the player or
    NPC it strikes (one point of damage), snapped as the JA+ module snaps (the decimals
    cut off), and pulls its owner there: `PMF_GRAPPLE_PULL` and the anchor in
    `ps.lastHitLoc`. A hook on a body moves to its middle once, 100 ms on, then every
    five minutes, as JA+'s does; it flies 10 seconds at most and lets go on the sky.
  - *The rope.* The pull and the hang are the shared movement's
    ([pmove_grapple.rs](../crates/sjk-game-jka/src/pmove_grapple.rs), switched on by
    `ModMovement::grapple`), which every JA+ client predicts. Letting go of the key
    drops the hook, or for a plugin user who had taken hold hangs on the rope
    (`EF_GRAPPLE_SUSPEND`, entity flag bit 16), walking neither hangs nor pulls, and the
    key again pulls again. Use lets go (after the move), as do death, a spawn, a duel,
    a teleport and leaving.
  - *Fuel.* The jetpack's: 1 every 200 ms while pulled, `jp_hookSpeed / 800 + 1` while the
    hook flies out, nothing while hanging; out of fuel, it lets go. With the jetpack
    off and no hook drawing, it recharges as before.
  - *Cut ropes.* Another player's swinging blade (5 units about it), a shot (10, its own
    shooter's too beyond 60 units of the hand) and a thrown saber (25 going out, 10
    coming back) cut a rope from the owner's right hand to the hook: it lets go, its
    fuel is spent, no hook for 10 seconds, the jetpack's switch locked for 3, a flash
    where it parted, and a fall in the next 10 seconds is the cutter's. Not with
    `dmflags` 512, and not across a private duel's isolation.
- **JA+'s admin system** ([Admins](#ja-admins) below).

| Setting | `jp_cinfo` bit | Effect |
| --- | --- | --- |
| `jp_allowflipkick` 1 (default) | 0 | Wall flips and runs push off players |
| `jp_fixRoll` 1 (default) to 3 | 1 to 3 | The roll fix levels, as SJK predicts them on JA+ servers; a grip may start while rolling or back flipping |
| `jp_improveYellowDFA` 1 (default) | 4 | The medium style's shorter flip over, without the spin |
| `jp_slideOnPlayer` 1 | 5 | Standing on a player slides, as in the base game |
| `jp_noAutoReplier` 1 | 14 | Only asks the JA+ plugin to keep its automatic replies off |
| `dmflags` 256 | | The base game's collision boxes (the dual jump attack's feet) |
| `jp_alterDMG` 1 or 2 | | The saber moves' damage from JA+'s move table: with 1 from the `jp_*Attack` settings (a range whose Min is above its Max keeps JA+'s own), with 2 always JA+'s own; 1 also halves a blow at a player holding a saber (70 % for a 35 hit), outside siege, unless it is a special or thrown |
| `jp_SingleStrong*`, `jp_SingleMedium*`, `jp_SingleFast*`, `jp_SingleKata*`, `jp_Staff*`, `jp_Dual*`, `jp_RollStab*`, `jp_StabDown*`, `jp_NewPull*`, `jp_NewSpinDfa*` | | With `jp_alterDMG 1`: each move's lowest and highest damage (`...Min`, `...Max`), or its flat damage (`...OtherAttack`, `jp_StaffDualSpinAttack`, `jp_DualKataAttack`) |
| `jp_SaberThrowScaleDMG` | | With `jp_alterDMG 1`: the thrown saber's damage scaled |
| `jp_reduceSaberBlock` 0 to 100 | | The percent chance that a blade passes through the saber it meets |
| `jp_allowSaberTouchDMG` 1 | | An idle blade's touch hurts (1), not only a returning one's |
| `jp_allowTeamKill` 1 | | `/kill` works in team games |
| `jp_gripSpeedScale` 0.8 (default) | | The run speed kept while gripping, within 0.4 and 1.01; every JA+ client, SJK's too, reads it from the server info |
| `jp_pushPullKnockDown` 1 (default) | | Push and pull reach a player lying knocked down or getting up |
| `g_friendlyFire` 1 | | Push and pull reach teammates too (the base game never throws them) |
| `jp_fixAbsorb` 2.0 (default) | | Absorbing a grip, lightning or drain gains `cost * level / 3 * jp_fixAbsorb` Force; 0 is the base game's `(cost / 3) * level` |
| `jp_pushAll` 1 (default), 2 | | Push and pull fling the items they reach (not weapons nor powerups), back home in 30 seconds; 2 dropped flags too. Latched: waits for the next map |
| `jp_gripOpenSaber` 0 (default) to 2 | | A gripped player's saber: 0 off, 1 only a thrown one off, 2 knocked out of the hand with 35 % of the Force or less |
| `jp_cloakFuel` 1 (default) | | The cloak drains its battery; 0 keeps it on for ever |
| `jp_DuelAlpha` 0 or more (default 100), -1 | | Duellists and the others pass through each other; -1 switches it off (the alpha itself is the clients') |
| `jp_allowTeamDuel` 1 (default) | | Private duels in team deathmatch, against the other team |
| `jp_removeDistanceDuelLimit` 1 (default) | | A duel no longer ends at 1024 units apart |
| `jp_autoDuelBOW` 1 (default) | | Both duellists bow when the duel is accepted |
| `jp_duelStartArmor` 100 (default) | | The armour both duellists start with |
| `jp_endDuelBroadcastMSG` 1 (default), 2, 0 | | The winner's health and armour: everyone, the two, nobody |
| `jp_privateDuelForceRegen` 200 (default) | | Milliseconds a Force point takes to come back in a private duel |
| `jp_emotDisallow` 0 (default) | | The emotes refused, a bit each (`amkneel` 1 ... `amnoisy` 28, `amhug` 13) |
| `jp_fixEmot` 1 (default), 0 | | 1 freezes the held emotes only while a player's client has no JA+ plugin (bots count as having it); 0 always |
| `jp_allowAmKnockmedown` 1 (default) | | `amknockmedown` works |
| `jp_allowAmDropSaber` 1 | | `amdropsaber` works (0 by default) |
| `jp_allowSaberSwitch` 1500 (default) | | Milliseconds between two `/saber` changes; 0 refuses them |
| `jp_allowJetpack` 1 | | The `jetpack` command gives the jetpack (0 by default). Latched: waits for the next map |
| `jp_allowHook` 1 (default), 2, 0 | | The grappling hook: 1 in free-for-all (and team deathmatch with `g_jediVmerc`), 2 only for `g_jediVmerc`'s mercenaries, 0 none. Latched: waits for the next map |
| `jp_hookSpeed` 800 (default) | | The hook's speed; while it flies out it costs `jp_hookSpeed / 800 + 1` fuel every 200 ms (the pull is always 800) |
| `jp_hookFloodProtect` 750 (default) | | Milliseconds between two hooks |
| `dmflags` 512 | | The hook costs no fuel, and no rope is cut |

A `jp_cinfo` bit is only announced for a rule the server plays, whatever its setting
says ([cinfo.rs](../crates/sjk-server-mod-japlus/src/cinfo.rs) is the table): single
player attacks, the new DFA, ledge grab, the alternate dimension, model scale,
`jp_noKATA`, the macro scan and the Jedi Outcast red DFA stay off, though JA+ turns
several of them on by default. JA+'s extra animations (`jp_newGLAAnims`) need its own
animation files, which SJK does not ship.

### JA+ admins

JA+'s admin system, from VisionJK's `codemp/game/japlus/g_japlus_admin.c`: the generic
part (ranks, rights, the "usable on" rule, the abuse counter) is
[sjk-server-mod's admin.rs](../crates/sjk-server-mod/src/admin.rs), JA+'s table, settings
and texts are [its admin.rs](../crates/sjk-server-mod-japlus/src/admin.rs), and the
server runs them in [bridge_admin.rs](../crates/sjk-dedicated/src/bridge_admin.rs). The
base game has no admin system: its players are never admins and these commands are
unknown there.

- **Ranks.** Council, knight and instructor, each with the mask of commands it may use
  (`jp_councilAllowedCMD`, `jp_knightAllowedCMD`, `jp_instructorAllowedCMD`); and the
  rcon rank, every right, which only a key or a one-time code gives. A command's bit is
  its place in JA+'s list, from 1: amkick, amban, amShowMotd, amorigin, amslap, amsleep,
  amwake, amghost, amdenyvote, amprotect, amsilence, amunsilence, amUnForceAltDim,
  amForceAltDim, amempower, ammerc, ammindtrick, then amtele, amstatus, amwhois, ammap,
  ampsay, npc, amrename, amlockteam, amforceteam, amkillvote, amvstr, amweather,
  amlogout, amtelemark, ampoll, amseeGhost, admCmdPrev, admCmdNext, playerPrev,
  playerNext, admCmdEXE. A mask holds bits 0 to 31; amorigin, amlogout, amtelemark and
  the commands from ampoll on are every admin's whatever the mask says. The rank is kept
  over a map change and lost on a disconnect.
- **Logging in.** `amlogin <password or code>`. JA+'s rank passwords (`jp_councilPass`,
  `jp_knightPass`, `jp_instructorPass`) count only with `japlus.adminPasswords 1` (SJK's
  setting, default 0, Sol's decision of 06/11/2026); without it only a one-time code
  logs in this way, and SJK clients log in with their key ([Key login](#ja-key-login)).
  With passwords on, the JA+ plugin's own login (`cp_login` in its userinfo) is tried at
  each begin, as JA+ does. A wrong password or code takes the rank away (JA+'s
  behaviour) and counts as a wrong try: after three, the player waits 30 seconds, then
  twice as long after each one more, at most 30 minutes (VisionJK's, not JA+'s). The
  rank's message (`jp_*LoginMSG`, `%s` the name, `\n` a line break) is told to everyone;
  the log gets `<name> is logged as an <rank>.` and the console a line saying how
  (password, code or key). `amlogout` ends it. `aminfo` lists the commands the rank may
  use, in one line rather than JA+'s line per command, then every player's (JA+'s
  list, the saber damage lines left out).
- **On whom.** `jp_admUsableOn` as JA+ reads it: -1 on everybody, -2 on players
  without a rank only, 0 or more on players without a rank and on admins at least that
  many levels below (0: the same rank or below). JA+ compares level numbers, so a player
  of the rcon rank (level 1) is reached as a player is. On oneself always.
- **Abuse.** With `jp_antiAdmAbuseTimer` above 0, an admin may use at most
  `jp_antiAdmAbuseCmdsNumber` commands on others in that many seconds; one more is
  refused, with the wait told, and with `jp_antiAdmAbuseNumberB4ban` above 0 the first
  abuse admin-bans the admin (JA+ starts the count of abuses left at 0, so the setting's
  number itself is never counted down from): rank lost, address added to
  `jp_banAdminIPs`. The rcon rank is never counted.
- **Admin bans.** `jp_banAdminIPs` (masks as `addip` takes them, a space after each)
  lists the addresses that may not log in, read as each level starts. The server console
  has `admBan <player>` (bans or unbans the player's address), `admAddIP <mask>`,
  `admRemoveIP <mask|all>` and `admListIP`.
- **The log.** Every admin command a player runs is logged as JA+ logs it (`ADMIN CMD
  EXECUTED : <command> executed by <name>`), but the rcon rank's and the console's.
- **The server console** (and rcon) runs the commands JA+ lets it run, with every right
  ([the commands](#ja-admin-commands)), and `admGrant <player> <council|knight|instructor|no>`
  and `admToggle <bit> <council|knight|instructor>` (one bit of a mask flipped and told
  to everyone; with no argument the list of bits).

### JA+ key login

An SJK client logs its player in as an admin with its identity key
([identity.md](identity.md#admin-key-login)); other clients use a one-time code. Both
are SJK's own, after VisionJK's login without a password (`codemp/game/auth/`), the
code in [admin_keys.rs](../crates/sjk-dedicated/src/admin_keys.rs) and
[bridge_admin.rs](../crates/sjk-dedicated/src/bridge_admin.rs).

- **`admin_keys.txt`** in the home directory's `base` (beside `sjk_server.cfg`): a line
  per admin, `<council|knight|instructor|rcon> <public key> [comment]`, the key as the
  player's `identity adminkey` copies it (or 64 hex digits); `#` starts a comment line, a
  wrong line is told on the console and skipped. It is read as every level starts and by
  `amkeys` on the console, which also lists it (public keys are no secret there).
- **`admin_host.key`**, beside it: the server's host key, 32 random bytes made the first
  time the file has keys. No secret: it tells a client which server it signs for. If it
  cannot be written, it is new at every start, and clients that kept the old one refuse
  to sign until the player removes its line.
- **The offer.** With keys, the server info has `sjk_amkey 1`; without, nothing, and no
  client asks. The exchange (`sjk_amkey hello`, `nonce`, `sign`, `ok`, `fail`) is
  [key_login.rs](../crates/sjk-protocol/src/key_login.rs): a nonce is good for one
  answer and 30 seconds; a hello comes with a hint per key (40 at most) so that only an
  admin's client answers; the signature covers the nonce, the host key, the address the
  client sees and its client number. Five hellos per 30 seconds; a wrong signature or an
  unknown key counts as a wrong try, as a wrong password does. A player who has its
  key's rank already (kept over a map change) is answered `ok <rank> kept`, not logged
  in or announced again.
- **`japlus.adminHosts`** (empty by default): the addresses the server is reached by,
  `"1.2.3.4:29070 192.168.0.5:29070"`. When set, a login signed for another address is
  refused (`fail address`), which stops a server passing this one's nonce on to its own
  players even for a client that never logged in here before; set it on a public
  server.
- **One-time codes**, for clients that cannot sign (the JA+ plugin, `jamp.exe`, other
  clients): `amcode <council|knight|instructor|rcon>` on the server console prints a code
  of eight letters and digits, which logs in once as `amlogin <code>` within a minute
  (any letter case), with or without `japlus.adminPasswords`.
- The rcon rank has every right, and JA+ has no login message for it: only the player
  is told.

### JA+ admin commands

What the commands do, from VisionJK's `g_japlus_admin.c`, `g_japlus_admin2.c`,
`g_japlus_fx.c` and `g_japlus_tele.c`, carried out in
[bridge_admin.rs](../crates/sjk-dedicated/src/bridge_admin.rs) and
[bridge_admin_effects.rs](../crates/sjk-dedicated/src/bridge_admin_effects.rs). A player
is named by client number, by a part of its name (colours and case ignored, the first
match) or, where a command takes everybody, by -1. Each needs its bit in the rank's mask
(the rcon rank and the console need none) and, on another player, `jp_admUsableOn`'s
leave; those marked "abuse" count for the abuse counter. The texts are JA+'s: the
`jp_msg*` settings (`jp_msgTarget*` for the player, `jp_msgBroadcast*` for the others with
`%s` its name, `jp_msgAll*` when it was for everybody), centre printed, a typed `\n` a
line break; nobody but the player is told when an admin of the rcon rank ran it. JA+'s
default masks leave the council without `amslap`, `ammindtrick`, `npc` and `amweather`.

| Command | Console | Effect |
| --- | --- | --- |
| `amkick <player>` | yes | Kicked (abuse) |
| `amban <player>` | yes | Its address into `g_banIPs` (`addip`), then kicked (abuse) |
| `amtele <player> <x> <y> <z> <yaw>`, `amtele <x> <y> <z> <yaw>`, `amtele <player> <to player>`, `amtele <to player>`, `amtele` | the forms with a player | Teleported (abuse): to the place, 60 units beside and 50 above the other player facing yaw 180 (JA+'s), or to the tele mark. Free-for-all without `g_jediVmerc` only. Refused over lava or slime (5000 units down) or over a linked hurting trigger near the ground below. With `jp_starTrekTeleport 1` (the default) a playing player is cloaked, beamed two seconds later without the flashes and uncloaked two seconds after that; otherwise at once, with the flashes. The speed is cleared, as JA+'s own teleport does. The player is told it may `refuseTele` |
| `amtelemark` | | Where `amtele` alone goes, told as `(x y z)` |
| `amorigin [player]` | | Where the player is |
| `refuseTele` | | Any player: the admins' teleports refused, or accepted again |
| `amslap <player>` | yes | Knocked down for a second and thrown in a random direction (175 or 200 units a second, 170 up) with a punch sound, once in 15 seconds a player but by the rcon rank (abuse). Free-for-all only |
| `amsleep <player\|-1>` | yes | Lying, kept down every second, hurt by nothing that spares god mode, no private duel (abuse). Free-for-all only |
| `amwake <player\|-1>` | yes | Up again |
| `amprotect [player]` | yes, with a player | Protected (`EF_INVULNERABLE`): no other player nor NPC hurts it until it swings a saber or fires; again to take it away (abuse, counted on oneself too, as in JA+). Free-for-all only |
| `amsilence <player\|-1>`, `amunsilence <player\|-1>` | yes | No `say`, `say_team`, `tell` or `amsay` (`gc` still works) and no rename, or the end of it |
| `amsay <text>` | | Any player not silenced: chat (`^3<Admin>`) that only the admins and the sender read; logged as `say_admin:` |
| `ampsay <player\|-1> <text>` | yes | A centre print and a console line |
| `amstatus` | yes | Every player with its JA+ plugin version (`cjp_client`; a bot shows JA+'s latest) |
| `amwhois` | yes | The admins and their ranks, told to everyone (to the asker alone when it has the rcon rank) |
| `ammap <gametype\|HL> <map>` | yes | `jp_highlander`, `g_gametype` and `map` queued, `nextmap` kept; free-for-all (0), duel (3), power duel (4), team (6), siege (7), CTF (8), or `HL` (free-for-all with `jp_highlander 1`, which does nothing yet) |
| `amrename <player> <name>` | yes | Renamed, and kept from renaming itself for 30 seconds |
| `amlockteam <s\|f\|b\|r>` | yes | A team locked or unlocked (`jp_teamLock` bits 1 to 4, told to everyone): `team` then refuses it with JA+'s texts, an automatic choice takes the other team, and the duels never lock the spectators or the game |
| `amforceteam <player\|-1> <team>` | yes | Moved to the team, past the locks |
| `amkillvote` | yes | The vote running fails and nothing of it is carried out |
| `amdenyvote <player>` | yes | No `callvote` ("You are not allowed to callVote"), or the right back |
| `ammotd` | | Any player: the message of the day (`jp_motd`) centre printed for `jp_motdtime` seconds with the seconds left |
| `amShowMotd <player\|-1>` | yes | The same for other players (abuse) |

What the admin system leaves out, each waiting for a whole feature: `amghost`,
`amempower`, `ammerc`, `ammindtrick`, `amForceAltDim`/`amUnForceAltDim` (the alternate
dimension), `npc`, `amweather`, `ampoll`, `amvstr`, `amseeGhost`, the admin guns and
script (`gun*`, `admCmdEXE`, `admCmdPrev`/`Next`, `playerPrev`/`Next`), the rcon rank's
own `freeze`, `size`, `jpclient`, `amfollow`, `amplayefx` and `amtest`, `say_team_mod`'s
routes into `amsay`/`ampsay`, the login effect (`jp_admLoginEffect`), the flood
exemption bit, the message of the day shown at every join, and the saber damage lines of
`aminfo`. Typed, they stay unknown commands; their bits stay in the masks. A kick or
Force use does not end `amprotect` (a swing or a shot does), a player's grapple is not
checked before a teleport, and `teleTime`'s five seconds of Force protection after one
are not kept. There is no security log: the login lines go to the console.

Not built yet, in the order planned: the rest of the Force changes (`jp_forceCombo`'s
push and pull grabs and its rage with speed, the flamethrower `jp_allowFlameThrower`);
then `amAltDim`, `immortal`, `toggleFlame` and the admin commands that need a whole feature
(ghosts, empower, mindtrick, polls); then chat, highlander, blocking walls and model scale.
Until then:

- What the JA+ duels leave out: the 3 seconds of walking through players after a duel
  (VisionJK's own `EF_PASS_PLAYERS` bit: the JA+ plugin and EternalJK have no rule for
  it, so they would be pulled back), the saber throw in a full-force duel (the movement's
  check, which every client predicts, still forbids it), the Force push and pull of a
  duellist's missiles (VisionJK `w_force.c:3796`), the alternate dimension, the duel's
  `DUEL END` log line, and `jp_DuelAlpha`'s alpha (drawn by the clients). VisionJK
  traces some lines with JA+'s list and some without; SJK leaves the isolated players
  out of every trace of a player but the challenge's, so a bystander's blaster bolt or
  disruptor shot also goes through a duellist rather than being spent on it.
- `jp_highlander`, `jp_forceCombo` and the other announced settings change nothing on
  the server.
- What the emotes and player commands leave out: `amkiss` (JA+'s own animations, see
  above) and JA+'s own draw and holster animations for `amflip`; the sleeping, ghosted and alternate-dimension states JA+'s checks read (none is
  built, so they never refuse); the JA+ hook's button ends an emote but there is no hook
  yet. `amcolor`, `sabercolor` and `saber` change the server's copy of the userinfo, not
  the endpoint's (`trap->SetUserinfo`), so a `dumpuser` shows the client's own.
- `jp_saberNoForceDrain` stays off: it changes the Force the special attacks cost, which
  clients predict (VisionJK's own client predicts the cost whatever the setting, SJK's
  shared movement too), so it waits until clients can be told of it.
- `jp_flipkickScaleDMG` changes nothing: the server deals no flip kick damage yet, the base
  game's included (`forceKickFlip` is not set by the movement).
- JA+'s saber rules for ghosts, mind-tricked or sleeping players and the alternate
  dimension (`JP_SaberNoContact`) wait for those features. Neither does `/kill` know of
  JA+'s sleeping, ghosted or silenced players.
- What the hook leaves out: the alternate dimension's and the sleeping or knocked-down
  admin states' bans (not built), JA+'s model scale (every rope's owner is at 100 %), the
  game-only rule of VisionJK's movement that a player with the hook out or its key held
  does no flip kick (`bg_pmove.c:2038`: no client predicts it, so it would pull them
  back), JA+'s `BUTTON_HOOK` cleared during a melee grapple, and the hook's
  `userInt1`, which names the twin (one bit on the wire). The ropes the frame's shots
  cross are cut once every missile has run, not between two of them. The original's
  snap puts an anchor a unit or two from VisionJK's (`docs/japlus/differences.md`):
  SJK follows the original. Not compared with a JA+ server: the hook's flight, hold and
  fuel numbers are VisionJK's.
- A suicide is credited to a player or NPC still in the game only; the base game's own
  credit for a crush, a trigger's hurt or an unknown death to the last pusher is not
  built (a fall to death is).
- `jp_pushAll` flings only an item standing there: one taken and waiting for its
  respawn is left alone (the JA+ module flings its hidden entity and puts its respawn off
  to 30 seconds), a flung item comes back at rest, and NPCs' pushes fling nothing. Items
  dropped by players are never flung but for the flags.
- `jp_gripOpenSaber` 2 knocks a gripped player's saber out of its hand, not an NPC's,
  whose saber goes off.
- VisionJK's movement slows a rage's recovery with speed on too, and stacks rage with
  speed under `jp_forceCombo` 1 (`codemp/game/bg/bg_pmove.c:8893-8903`); SJK's shared
  movement keeps the base game's, left for the `jp_forceCombo` step.
- The roll fix follows TaystJK's reading of the levels, which SJK's prediction uses on JA+
  servers. VisionJK's port of the JA+ module differs: a roll starts from 30000 squared
  speed at every level and also while gripping at level 1, and level 3 slows a held-back
  roll otherwise. A fix belongs in the shared movement
  ([pmove_roll.rs](../crates/sjk-game-jka/src/pmove_roll.rs)), so that the prediction on
  real JA+ servers gains it too.

The hooks a mod has are in [sjk-server-mod](../crates/sjk-server-mod/src/lib.rs), the
JA+ mod in [sjk-server-mod-japlus](../crates/sjk-server-mod-japlus/src/lib.rs), and the
server's table of mods in [server_mods.rs](../crates/sjk-dedicated/src/server_mods.rs).
Supporting another mod is an entry there.

## Cheat commands

Cheats follow `sv_cheats`, which only the server sets: `--cheats` turns it on at
startup, the server console's `devmap <map>` loads a map with it on and `map`
loads one with it off. The client's [devmap](client.md#development-maps) starts
an owned local game with cheats on.

Players type these in the client console. Without cheats the server answers
with the stock "cheats are not enabled" message (`@@@NOCHEATS`). `give`, `t_use`,
`npc`, `noclip`, `god`, `notarget`, `jetpack`, `giveother`, `killother`, `levelshot`,
`thedestroyer` and `debugBMove_*` also need a living player in the game
(`@@@MUSTBEALIVE`), as in OpenJK's `ClientCommand`; `setviewpos` also works for
spectators. `give`, `noclip`, `god`, `notarget`, `jetpack`, `giveother` and `levelshot`
are refused during intermission.

| Command | Effect |
| --- | --- |
| `give all` | Every holdable item, full health, armour and Force power, every usable weapon and 999 of each ammo type |
| `give health [N]`, `give armor [N]` (or `shield`) | Health (1 to the maximum) or armour (0 to the maximum health), full without `N` |
| `give force [N]` | Force power, 0–100, full without `N` |
| `give weapons`, `give weaponnum N` | Every usable weapon, or the weapon with number `N` |
| `give ammo [N]` | `N` (0–999, 999 without `N`) of every ammo type |
| `give excellent` (also `impressive`, `gauntletaward`, `defend`, `assist`) | One more of that award |
| `give CLASSNAME` | The item with that classname (`item_seeker`, `weapon_rocket_launcher`, `ammo_thermal`…, without case), taken as if walked over (`Touch_Item`): what the player cannot carry is not taken. Flags are not given, and a powerup (`item_quad`, `item_enviro`…) gives nothing, as `pick_up` skips them |
| `giveother PLAYER WHAT…` | `give` for another living player, by slot number or name ("Target must be alive to use this command." otherwise) |
| `killother PLAYER` | Kill another living player, as its own `kill` would |
| `t_use NAME` | Fire every target named `NAME` as the player (`Cmd_TargetUse_f`), as mappers test triggers and scripts |
| `setviewpos X Y Z YAW` | Teleport to the position, facing `YAW` |
| `noclip` | Toggle flying through the world (see [Noclip](#noclip)) |
| `god` | Toggle god mode (`FL_GODMODE`, "godmode ON/OFF"): no damage, the player's own included, though blows still push; only damage that ignores protection (`DAMAGE_NO_PROTECTION`, such as a `trigger_hurt` with that spawnflag) still hurts |
| `notarget` | Toggle notarget (`FL_NOTARGET`, "notarget ON/OFF"): NPCs, vehicle turrets and map turrets no longer take the player for an enemy and drop it if they had. Bots still do, as their reference enemy checks (`PassStandardEnemyChecks`) never read the flag |
| `levelshot` | For a player on the server's own machine (`localhost` or a loopback address, `pers.localClient`): the intermission begins and the client is sent `clientLevelShot`, which the SJK client does not act on |
| `thedestroyer` | Light the saber in hand when it is holstered (`Cmd_ToggleSaber_f`) |
| `debugBMove_Forward BOT` (also `_Back`, `_Left`, `_Right`, `_Up`) | Toggle a forced move for the bot in slot `BOT` (`Bot_SetForcedMovement`); it only moves the bot while `bot_forgimmick` is set, as in OpenJK |
| `jetpack` | SJK's own: give the jetpack with a full tank, or take it away (switched off, the tank full again, so the HUD's fuel bar goes too). Fly it as stock does: the use key in the air lights it (`tryJetPack`), jump to climb, use again to put it out |

A spawn (respawn or team change) clears `noclip`, `god` and `notarget`, as
`ClientSpawn` clears `client->noclip` and `ent->flags`; `kill` ends god mode too
(`G_Kill`).

`give` follows OpenJK's `G_Give`. An item given by name raises no pickup event,
since the reference's item entity exists only for the touch: no pickup sound and no
automatic weapon switch. See
[give.rs](../crates/sjk-game-jka/src/give.rs),
[bridge_commands.rs](../crates/sjk-dedicated/src/bridge_commands.rs),
[bridge_cheats.rs](../crates/sjk-dedicated/src/bridge_cheats.rs),
[bridge_client_commands.rs](../crates/sjk-dedicated/src/bridge_client_commands.rs),
[client_commands.rs](../crates/sjk-game-jka/src/client_commands.rs) and
[bridge_teleport.rs](../crates/sjk-dedicated/src/bridge_teleport.rs).

## Other client commands

Besides chat, votes, teams, following and the cheats, the server takes the rest of
OpenJK's `ClientCommand` table:

| Command | Effect |
| --- | --- |
| `where` | Print the player's origin in whole units, `(x y z)` (`Cmd_Where_f`) |
| `forcechanged [TEAM]` | Sent by the Force menu: a spectator's Force is read again from the userinfo at once; a player is told the change waits for its next spawn (`@@@FORCEPOWERCHANGED`) and spawns with the new Force next time (`forceDoInit`). Given a team outside the duels, it goes on as `team TEAM`. A siege or power duel respawn does not read it again yet |
| `voice_cmd SOUND` | In team games, one of the 29 siege voice sounds (`req_medic`, `tac_hold`…, `bg_customSiegeSoundNames`), heard by everyone from the speaker (`EV_VOICECMD_SOUND`); a spectator is told `@@@NOVOICECHATASSPEC` |

A command not in that table (`commands[]` in OpenJK's `g_cmds.c`) gets
`Unknown command <name>`, as OpenJK and JoF EJK answer. `addbot` gets the stock
refusal that only the server adds bots (`@@@ONLY_ADD_BOTS_AS_SERVER`). A server mod
adds its own: `engage_FullForceDuel` and the admin commands in [JA+ mode](#ja-admins).
Other mods' commands (JoF EJK's, accounts, clans, races and the like) are not part of
this server. The table is
[client_commands.rs](../crates/sjk-game-jka/src/client_commands.rs).

## Noclip

With cheats enabled (`--cheats` or server-console `devmap`), a living player can
run `noclip` in the client console to toggle flying through the world. The client
also offers [devmap](client.md#development-maps) for starting an owned local game.
The server refuses noclip during intermission, for spectators/dead players, or
with cheats disabled. Spawning again clears it.

Noclip uses the existing OpenJK-compatible movement and snapshot prediction.
It skips item/trigger contacts, drowning and ordinary damage, retaining the
reference's earlier DEMP2 shock handling. OpenJK's command scaling quirk remains:
vertical input alone does not accelerate without forward/sideways input.
See [bridge_cheats.rs](../crates/sjk-dedicated/src/bridge_cheats.rs) and
[noclip.rs](../crates/sjk-game-jka/src/noclip.rs).

## NPC definition packs

With `g_stockRules 0` (the default), native NPC loading reads every listed
`ext_data/npcs/*.npc` file without the stock filename-list or aggregate-text
limits. This allows larger collections of community definitions; it does not
raise the number of NPCs alive at once or change protocol 26 limits.

`g_stockRules 1` keeps the reference's 2,048-byte filename listing and 256 KiB
aggregate-text bound, including the existing empty-definition fallback on a
text-size load error. Both profiles preserve VFS listing order and installed
content precedence. Reload the map after changing the installed definitions.
See [map.rs](../crates/sjk-dedicated/src/map.rs).

## Vehicle boarding and sand creatures

Landing on an empty animal or speeder requests boarding using the multiplayer
rider, vehicle-type and team checks. Empty vehicles marked `SUSPENDED` also allow
boarding while standing on them, including fighters and walkers. These are
server decisions; a client connected to another server uses that server's rules.

With `g_stockRules 0` (the default), `npc spawn sand_creature` and
`npc spawn sand_creature_fast` use native burrowing AI. Set the rule before
loading the map; the NPC roster keeps that selection for the map's lifetime.
`g_stockRules 1` preserves multiplayer's existing generic NPC behavior.
Move clear of the spawn location so the creature can finish spawning.

This extension takes its movement sensing, hunting timeout, breach/attack
animations and sand effects from the single-player sand creature. It hunts moving,
grounded players and NPCs, passes beneath bodies while still colliding with map
solids, breaches, and ambushes close targets. Standing still or leaving the ground
breaks its movement tracking. A successful catch uses ordinary multiplayer damage
and respawn handling, rather than the single-player swallowed-player state.

It is not a campaign-AI parity claim: scripted alerts/turf constraints, thermal
bait, victim attachment/swallowing, and single-player camera effects are not
implemented. Pursuit uses local steering and ordinary NPC movement; complex
obstacles still need broader map playtesting. No new protocol fields or SP entity
flags are sent, and no new assets are required for the retail sand creature.

## E-Webs

The E-Web holdable is the multiplayer game's (`ItemUse_UseEWeb`, `EWeb_Create`,
`EWebThink`, `g_items.c`): the use key on it sets the gun down 48 units ahead when its box
is clear there and lands on the world within 18 units (else the "shieldcon_empty" sound),
and puts it away again a second later at the earliest. While on it the player holds the
emplaced gun's weapon (`ps.emplacedIndex`), is kept 32 units behind the gun's top, which
turns after its view by at most four degrees a frame, and fires a 20-damage turret bolt
(`MOD_TURBLAST`) from the muzzle every 100 ms while fire is held. Backing off or
moving too far off the ground puts it back in the inventory. It has 200 of health, kept
between deployments; shot to pieces, or when its owner dies or leaves, it blows up (90
within 128, `MOD_SUICIDE`) and the holdable is gone for good. The muzzle and the gunner's
place are read on the server's own copy of `eweb_model.glm`, posed at frame 0 with the two
bone turns (`map_turret_model::TurretPose`), so they do not follow the unfolding or firing
frames. The gun does not fall (`G_RunExPhys` is not run); it is set down on the ground.

The single-player maps' `emplaced_eweb` (hoth2 places two), which the multiplayer game
has no spawn for, stands where the map puts it with its own `health` (250 by default),
`splashDamage` (40) and `splashRadius` (100); its `INVULNERABLE` flag (4) makes it
take no damage. Anyone standing up, alive and on foot who faces its way (a dot of 0.75)
takes it with the use key, as `eweb_can_be_used` asks, and works it as the holdable is.
The use key again, backing off or dying lets go; the gun stays. Not ported: its `usescript`,
`painscript` and `deathscript`, `INACTIVE` with `target_activate`, NPCs taking it, the wreck
and its smoke (the gun is gone once it blows up).

## Siege doors

The use key reaches a `PLAYER_USE` door (`ValidUseTarget`, `GlobalUse`). In siege,
`TryUse`'s `alliedTeam` rule keeps the door's `teamallow` team from using it unless
`g_ff_objectives` is set. Before the round begins, the use key does nothing.

A door that spawns locked carries `EF_SHADER_ANIM`. Its `s.frame` is 0 while locked and 1
once unlocked (`LockDoors`, `UnLockDoors`), so the client draws the lock lights by state.

`trigger_multiple` and `trigger_once` take their team from `team`, or else from the generic
`teamuser`/`alliedteam` spawn fields.
