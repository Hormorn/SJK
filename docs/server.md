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

A `jp_cinfo` bit is only announced for a rule the server plays, whatever its setting
says ([cinfo.rs](../crates/sjk-server-mod-japlus/src/cinfo.rs) is the table): single
player attacks, the new DFA, ledge grab, the alternate dimension, model scale,
`jp_noKATA`, the macro scan and the Jedi Outcast red DFA stay off, though JA+ turns
several of them on by default. JA+'s extra animations (`jp_newGLAAnims`) need its own
animation files, which SJK does not ship.

Not built yet, in the order planned: the rest of the Force changes (`jp_forceCombo`'s
push and pull grabs and its rage with speed, the flamethrower `jp_allowFlameThrower`);
then the emotes and player commands (`amkiss`, `amflip`, the hook, `/saber`...); then
the admin system, logged in with SJK identity keys, with JA+'s rank passwords only if
the owner allows them; then chat, highlander, blocking walls and model scale. Until then:

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
- `jp_saberNoForceDrain` stays off: it changes the Force the special attacks cost, which
  clients predict (VisionJK's own client predicts the cost whatever the setting, SJK's
  shared movement too), so it waits until clients can be told of it.
- `jp_flipkickScaleDMG` changes nothing: the server deals no flip kick damage yet, the base
  game's included (`forceKickFlip` is not set by the movement).
- JA+'s saber rules for ghosts, mind-tricked or sleeping players and the alternate
  dimension (`JP_SaberNoContact`) wait for those features; a blade does not cut grapple
  ropes yet. Neither does `/kill` know of JA+'s sleeping, ghosted or silenced players.
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
refusal that only the server adds bots (`@@@ONLY_ADD_BOTS_AS_SERVER`). Mod commands
(JoF EJK's and JA+'s `am*`, accounts, clans, races and the like) are not part of this
server. The table is
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

## Siege doors

The use key reaches a `PLAYER_USE` door (`ValidUseTarget`, `GlobalUse`). In siege,
`TryUse`'s `alliedTeam` rule keeps the door's `teamallow` team from using it unless
`g_ff_objectives` is set. Before the round begins, the use key does nothing.

A door that spawns locked carries `EF_SHADER_ANIM`. Its `s.frame` is 0 while locked and 1
once unlocked (`LockDoors`, `UnLockDoors`), so the client draws the lock lights by state.

`trigger_multiple` and `trigger_once` take their team from `team`, or else from the generic
`teamuser`/`alliedteam` spawn fields.
