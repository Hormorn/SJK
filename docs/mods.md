# Client mods

A client mod owns its commands, settings and optional community presentation,
kept out of the core client. Each one loads only on the servers it is made for, so a player who never
joins one sees none of its commands. The mods are built into the client; the line
they are written against is the [sjk-mod](../crates/sjk-mod/src/lib.rs) crate.

## Rules

- **Names.** Everything a mod adds starts with its id and a dot: `japlus.guntele`,
  `japlus.loginServer1`, `jof.commands` (Sol's choice, 11/10/2026). So does every
  setting the core keeps for one server family alone (`japlus.awayEmote`, Sol,
  24/10/2026): the prefix names the subject, not who owns the code. Typing the id
  and a dot, then Tab, lists the mod's commands. Three kinds of names stay as they
  are: server commands (`amtele`, `amkick`, emotes), which the server parses, so
  binds made for other clients still work and a mod only completes and describes
  them; userinfo and serverinfo keys (`cjp_client`, `cp_pluginDisable`, `jp_cinfo`),
  which the server reads; and settings shared by every server, even when one
  family is their only table entry today (`cg_flameThrower`, `amcolor`). A
  setting's earlier name, or the one JoF EternalJK gives it, stays accepted as an
  alias (`cg_antiAmKiss` sets `japlus.antiAmKiss`): a config made for it still works
  and the value is saved under the new name (`ClientMod::old_setting_names`).
- **Loading.** A mod's commands exist only on the servers it is made for: JA+
  tools on every JA+ server, JoF tools on JoF servers (a JoF server is a JA+ one, so
  both load there). Joining one loads the mod and leaving it unloads it, and Tab
  completes its commands only while it is loaded. A mod's command typed elsewhere
  says which servers it works on. A mod's own settings exist whether it is loaded
  or not, so saved values are kept.
- **The switch.** Each mod has an archived on/off setting, `mod_<id>`. It is on by
  default (Sol's choice, 11/10/2026). In Settings it is a row of Game options ("JA+
  tools", "jaPRO tools", "JoF tools"). With the switch off, the mod does not load even on its servers.
  Profiles saved while the mods were off by default are switched on once
  (`mod_defaultVersion`). `mods` lists the mods: loaded, on (waiting for its
  servers) or off.
- **What stays in the core.** A mod adds commands a player types. What a server
  needs to be played correctly is not a mod, with one exception: jaPRO's own
  movement styles and their ports belong to jaPRO tools (Sol, 20/10/2026), as each
  is jaPRO's alone. JA+ movement and saber prediction,
  `cjp_client`, `cp_pluginDisable` and its default, RGB sabers and cosmetics stay in
  the core client ([networking.md](networking.md)) and work with every mod off.
  JA+ flamethrower presentation also stays in the core; its detection and player
  override are documented under [flamethrower presentation](client.md#ja-flamethrower-presentation).
- **Community presentation.** The Destruction extension, the JoF dialogue window and
  JoF's firing-confirmation sounds belong to `sjk-mod-jof`, not the base compatibility
  profile. The mod owns their definitions, parsers and state; optional viewer
  bridges draw/play their output only while JoF tools are loaded. Base sessions
  forward unhandled reliable operands without interpreting community commands.
  These additions are compiled out of the viewer without `mod-jof`.
- **What a mod can do.** A mod sees the game only through
  [`Host`](../crates/sjk-mod/src/lib.rs): it can send a reliable server command,
  read the server's kind, address and serverinfo, the predicted player, the crosshair
  trace's end point and player, the roster with where each player is drawn, sweep
  the player's standing box through the map's world brushes (`trace_player`), and
  read or set a setting. It does not reach the renderer, prediction or the wire.
  Typed mod settings distinguish text, booleans and integer choices. A mod may
  declare scoped userinfo fields through the host; the existing adapter validates
  and encodes them, and removes them when their mod/setting no longer permits them.
  Commands run on the main thread from the client command queue
  ([mods.rs](../crates/sjk-viewer/src/mods.rs),
  [mods_host.rs](../crates/sjk-viewer/src/mods_host.rs)).
- **Passwords.** A mod declares the passwords its servers take
  ([`Secrets`](../crates/sjk-mod/src/lib.rs): commands whose arguments are one, parts of
  cvar names whose value is one) and its chat commands, whose typed lines stay out of
  crash reports and logs as received chat does. Unlike its commands, these do not
  wait for its servers: the client adds every built-in mod's table to the console's
  masking ([secret.rs](../crates/sjk-shell/src/secret.rs)) at startup, before any
  cvar or line, so they are masked offline, in binds and configs run at startup, on
  every server and with the mod off ([Passwords](#passwords)).
- **Building without them.** Each mod is a crate behind a default feature of
  `sjk-viewer` (`mod-japlus`, `mod-japro`, `mod-jof`, which brings JoF content too); `cargo build
  -p sjk-viewer --no-default-features` builds a client without any.

Mods are not loaded from files. A native library has no stable interface between
Rust builds and a downloaded one would run with the player's rights. If mods
made by other people are ever wanted, the plan is a sandboxed (WebAssembly) host
for the same `Host` interface. Nothing of that is built.

## JA+ tools (`japlus`)

[sjk-mod-japlus](../crates/sjk-mod-japlus/src/lib.rs). The teleports are JoF
EternalJK's (`codemp/cgame/cg_consolecmds.c`), renamed. The client works out
where to go, then sends JA+'s `amtele`; the server checks the admin rights, so
they do nothing for a player who is not a JA+ admin. Positions are sent with six
decimals, as EternalJK's `%f` does.

| Command | From EJK | What it does |
| --- | --- | --- |
| `japlus.guntele [distance] [yaw offset]` | `teleGun` | With no arguments: to the crosshair trace's end point, 24 units up, keeping your yaw. With a distance: that far straight ahead from your predicted origin, level (your yaw only, so looking down never puts you under the floor), stopping 2 units short of the first wall or step your standing box meets on the way, or your crouched box under a low ceiling (the server keeps whole units); the yaw turned by the offset. From inside a wall (noclip) it goes to the crosshair's point instead, or nowhere. EJK follows the view pitch included and goes through walls; SJK departs from it here at the player's request (hub report #218) |
| `japlus.guntelewall <distance> [yaw offset]` | `teleGun` | That far along your view from your predicted origin, pitch included (look up or down to go through a ceiling or floor), through any wall in the way: EJK's `teleGun` with a distance, and `japlus.guntele` before #218, kept beside it for players who used it to pass walls (hub report #232). Nothing is traced, so you may land inside something solid; the yaw turned by the offset. Works from a quick wheel choice with its distance (`japlus.guntelewall 300`) |
| `japlus.bring [id\|name\|gun] [distance] [yaw offset]` | `get` | Brings a player (the one under the crosshair without a name or with `gun`) in front of you, 100 units by default, facing you. Without a distance it lands level with you, 24 up |
| `japlus.goto [id\|name\|gun] [distance] [yaw offset]` | `goto` | Teleports you in front of a player as it is drawn, facing it. With only a name you land 24 above its sent position. A player not drawn for a second or more is reached with `amtele <id>` |
| `japlus.teleoffset x [y [z]]` | `amTeleOffset` | Moves you by whole units from your predicted origin |
| `japlus.mark`, `japlus.recall` | `PTelemark`, `PTele` | Keeps your position and yaw in whole units; recall sends `setviewpos`, which only a server that allows it runs |
| `japlus.autologin` | `autoLogin` | Sends `amlogin <password>` with the password saved for the current server in `japlus.loginServer1-3` / `japlus.loginPass1-3`. A server without a port is `:29070`, and a host name is looked up when the command runs. Like EJK it runs only when asked, not during the intermission, and the passwords are saved as plain text |
| `japlus.serverconfig` | `serverconfig` | Lists a JA+ server's `jp_cinfo` options (flip kick, roll fix mode, DFA variants, kata, ledge grab, alternate dimension); on jaPRO/TaystJK asks the server |
| `japlus.plugin [number]` | `plugin`, `pluginDisable` | Lists JA+'s [plugin options](#plugin-options) as On or Off, or switches one by EJK's number |

A player is named by client number, by name (colours and case ignored) or by a
fragment of exactly one name. On a JA+ server the mod also completes JA+'s server
commands: the admin set, clan commands, duels and emotes, from EternalJK's `gcmds[]`.

| Setting | What it does |
| --- | --- |
| `japlus.loginServer1-3`, `japlus.loginPass1-3` | `japlus.autologin`'s servers and passwords |
| `japlus.antiAmKiss` | [Anti-kiss](#anti-kiss); `cg_antiAmKiss` is its alias |
| JA+ plugin options | Switches for bits of `cp_pluginDisable` ([Plugin options](#plugin-options)) |

EternalJK's `amColor` is the core's `amcolor`: the tint is stock JKA userinfo, so it
works on every server and is not a JA+ tool (an earlier `japlus.color` duplicate was
removed on 24/10/2026).

### JA+ in SJK

Where each JA+ part lives, so a change to JA+ support starts in one place. The mod
holds what a player types; the core holds what a JA+ server needs to be played and
works with `mod_japlus 0` ([What stays in the core](#rules)).

| Part | Where | Name |
| --- | --- | --- |
| Admin teleports, auto login, server options | [sjk-mod-japlus](../crates/sjk-mod-japlus/src/lib.rs) | `japlus.*` commands |
| Admin key login on SJK's JA+ servers | [admin_key_login.rs](../crates/sjk-viewer/src/admin_key_login.rs) (core: it needs the identity key) | `identity adminkey`, [identity.md](identity.md#admin-key-login) |
| Plugin options: the table, the command, the Settings switches | [plugin.rs](../crates/sjk-mod-japlus/src/plugin.rs), bridged by [plugin_rows.rs](../crates/sjk-viewer/src/settings/plugin_rows.rs) | `japlus.plugin` |
| Anti-kiss | [anti_kiss.rs](../crates/sjk-mod-japlus/src/anti_kiss.rs), bridged by [anti_emote.rs](../crates/sjk-viewer/src/anti_emote.rs) | `japlus.antiAmKiss` |
| Passwords the console masks | [secrets.rs](../crates/sjk-mod-japlus/src/secrets.rs) | |
| Server command completion | [server_commands.rs](../crates/sjk-mod-japlus/src/server_commands.rs) | |
| Away emote on JA+ servers | [away_emote.rs](../crates/sjk-viewer/src/away_emote.rs) (core) | `japlus.awayEmote` |
| Plugin identity and userinfo (`cjp_client`, `cp_pluginDisable`, `cp_clanPwd`) | [compat_profile.rs](../crates/sjk-client/src/compat_profile.rs) (core) | server keys |
| Movement and saber prediction | [networking.md](networking.md) (core) | |
| Flamethrower presentation | [japlus_flame.rs](../crates/sjk-viewer/src/japlus_flame.rs) (core, contract table) | `cg_flameThrower` |
| Holstered sabers' JA+ contract | [holstered_sabers.rs](../crates/sjk-viewer/src/holstered_sabers.rs) (core, contract table) | `cg_holsteredSaberSupport` |
| Plugin bit for black blades | [actor_world_submission.rs](../crates/sjk-viewer/src/actor_world_submission.rs) (core) | `cp_pluginDisable` |

The core parts are entries in tables shared by several server families (AGENTS.md,
no hardcoded special cases), which is why they are not moved into the mod. Settings
shows the mod's switch and its settings together under Mods > Server mods (Game
options in classic); the away emote stays beside the SJK one under Gameplay.

Before the mod, `serverconfig` and `pluginDisable` were core commands. A bare
`serverconfig` is now sent to the server like any unknown command, which
jaPRO/TaystJK servers answer. TaystJK's completion lists it. EJK's `plugin` is
`japlus.plugin` on JA+ and `japro.plugin` on jaPRO ([Plugin options](#plugin-options)).

### Anti-kiss

[anti_kiss.rs](../crates/sjk-mod-japlus/src/anti_kiss.rs) is JoF EternalJK's
`cg_antiAmKiss` (`CG_DrawActiveFrame`, `cg_view.c:2635-2640`; SJK #190, Lukas), as
`japlus.antiAmKiss` (EternalJK's name is an alias). Archived and off by default, under
Settings > Mods > Server mods, after the JA+ tools switch (Game options in classic).
When another player's `amkiss` puts you in the kiss-victim animation (`BOTH_KISSEE`, in
the server's player state or the predicted one), it uses Force Push once: one push per
interaction, so the initiator's animation is not disturbed over and over. Spectators and
a player following another never push. Its table, `SERVER_CASES`, holds the families it
answers on (JA+, JoF's included) with the evidence.

The client only bridges it ([anti_emote.rs](../crates/sjk-viewer/src/anti_emote.rs)):
each frame, while JA+ tools are loaded and the setting is on, it hands the mod the
player state and the predicted animations, and puts the generic command the mod names
(`force_throw`) in the next user command, as a bound key would; a command the player
pressed that frame goes first. With `mod_japlus 0`, on another server or in a build
without `mod-japlus` it does nothing (the setting does not exist in such a build). Unit
tests cover the detection, the latch and the server table in the mod, and the client
pins the mod's animation, follow and spectator numbers to its own tables. Lukas checked
the push live on a JoF JA+ server before the move into the mod; not checked since.

### Passwords

[secrets.rs](../crates/sjk-mod-japlus/src/secrets.rs) is JA+ tools' password table,
each entry with its source: `amlogin` (JA+ admin login, JoF's too), `login`,
`register` and `changepassword` (JoF EternalJK's accounts), `clanpass` (JA+ clan
chat) and `clanwhois` (JoF), whose arguments are masked, and `loginpass`, which makes
`japlus.loginPass1-3` secret cvars. Its chat commands, `say_team_mod`, `clansay`,
`amsay` and `ampsay`, keep a line typed with one out of the session log, crash
reports and log uploads, and divide a message over the server chat limit into complete
word-boundary pieces through the same outgoing chat path as stock chat
([identity.md](identity.md#crash-reports)). The console shows `amlogin ********` in its echo,
listings and binds, and keeps no password in the scrollback, `qconsole.log`, the
session log, crash reports or log uploads ([client.md](client.md#passwords-in-the-console)).
The table applies from startup whatever the server and `mod_japlus`, and no setting
turns an entry off; a build without `mod-japlus` does not mask these words.

## jaPRO tools (`japro`)

[sjk-mod-japro](../crates/sjk-mod-japro/src/lib.rs) loads on every server SJK sees as
jaPRO or its TaystJK lineage (the `TaystJk` profile). It owns jaPRO's movement styles
([movement_styles.rs](../crates/sjk-mod-japro/src/movement_styles.rs)).

A jaPRO server lets each player pick a style (`/move`) and sends it in
`ps.stats[13]` (`STAT_MOVEMENTSTYLE`, read as EternalJK's `PM_GetMovePhysics` does:
Swoop on a vehicle, the co-op styles folded onto their base). SJK's pmove is stock
JKA's, so only Siege, JKA, Swoop and Speed (the server's `ps.speed`, no pmove branch)
move as it predicts. CPM, QW, Q3, PJK, WSW, the rocket-jump styles, Jetpack, SP, Slick
and BotCPM have physics of their own: predicting them as JKA snapped the view back
every frame (hub reports #97/#98). While the mod is loaded, the viewer asks it each
frame ([interpolated_view.rs](../crates/sjk-viewer/src/interpolated_view.rs),
`japro_unpredicted`, no allocation or lock); a style it does not model is shown as the
server has it, as `cg_noPredict 1` does, keeping the mouse, and the log names the style
once.

| Name | What it does |
| --- | --- |
| `japro.styles` | Lists the styles and which ones SJK predicts |
| `japro.predictMoveStyles` | Off by default. On, every style is predicted as JKA, the earlier behaviour (Settings > Mods, "Predict all styles") |
| `japro.plugin [number]` | JoF EJK's `plugin` on jaPRO: lists jaPRO's [plugin options](#plugin-options) as On or Off, or switches one by EJK's number ([plugin.rs](../crates/sjk-mod-japro/src/plugin.rs)) |

With `mod_japro 0`, or a build without `mod-japro`, every style is predicted as JKA.

**Porting a style.** Its physics go in this crate beside the table, one module per
family (CPM first: Q3, Slick and BotCPM build on it; then QW and PJK, SP, WSW with its
dash and wall-jump timers, the rocket-jump styles and Jetpack), and the viewer's
prediction calls them through the same bridge. The reference is JoF EternalJK's
`codemp/game/bg_pmove.c` (every `moveStyle` branch). A style turns `predicted: true`
only once a recorded jaPRO demo predicts it without misses.

Other jaPRO-only behaviour still in the core and candidates for this mod: the race
mode reads of `ps.stats[11]` (chat, dynamic crosshair, enemy info, Ysalamiri tint)
and jaPRO's entry in the duel-hiding table.

## JoF tools (`jof`)

[sjk-mod-jof](../crates/sjk-mod-jof/src/lib.rs). A JoF server is a JA+ server
whose serverinfo `V` is `2.5B0`, as JoF EternalJK's `CL_JoFTrustedServer` checks.
The mod completes the commands only JoF servers run: the `gun*` admin set, which
acts on the player you aim at (the server works out who), `jetpack`,
`toggleFlame`, `refuseTele`, `immortal`, `sleep`, and the commands JoF's `amInfo`
lists under "JoF JA+ Commands" (`amHelp`, `amTips`, `amEmotes`, `amShop`,
`amCredits`). The `am*` player commands and emotes JoF EJK registered with them
(`amKnockMeDown`, `amDropSaber`, `amAtEase` to `amWon`) are JA+ ones: JA+ tools
complete them (Sol, 18/10/2026). `jof.commands` lists the JoF set and says
whether the current server is a JoF one.

JoF tools also bring [JoF content](jof-content.md) (`jof.downloads`, `jof.content*`):
JoF's maps, skins, hilts and packs downloaded from JoF's repository, as JoF's launcher
does, off until the player turns it on. Unlike the commands, it does not wait for a
JoF server: the card and its folder work from the menus, and `mod_jof` does not
switch it.

Not ported: `binoScan` and `cg_pickupConfirm`; these request additional
server data/protocol behavior which this mod does not yet implement.

### EJK server opt-in and firing confirmations

`jof.ejkOptIn` is an archived JoF mod opt-in, on by default. With it on, the loaded mod asks the
existing userinfo adapter to send `jofejk=1` to a recognized JoF server. It is
never a global `CVAR_USERINFO` value: other JA+, jaPRO, base, disconnected and
mod-off sessions omit the field. The setting and `mod_jof` changes mark userinfo
dirty; removal uses the normal next rate-limited userinfo update (up to one
second), while local handlers stop immediately. The mod's settings appear under
Mods in SJK Settings and Game options in classic.

**This is a server-extension opt-in, not a local sound mute.** A nonempty
`jofejk` value asks the server to treat the player as EJK-extension capable;
`jofejk=0` would still be nonempty, so opting out removes the key. Public EJK
sources explicitly document Stasis/Repulse firing confirmations behind it
([cg_xcvar.h](https://github.com/JediofFreedom/JoF_EJK/blob/beta/codemp/cgame/cg_xcvar.h),
[cg_servercmds.c](https://github.com/JediofFreedom/JoF_EJK/blob/beta/codemp/cgame/cg_servercmds.c)).
Those client sources do not enumerate the private JoF server's other checks of
`jofejk`. Sol (who wrote JoF EJK's side of it) confirmed on 18/10/2026 that the JoF
server gates all of its abilities on it, plus some quirks that are server-side
only; that is why it is on by default, as JoF EJK players have it. In
particular this switch does not advertise `binoScan` or `cg_pickupConfirm`:
those separately opt into binocular/party data and versioned pickup confirmation,
and are still not implemented here. The existing `cjp_client` identity stays SJK.

`jof.e2eWhispers` (archived, off by default; 25/10/2026, JoF's request) turns encrypted
whispers back on on JoF servers. Without it, whispers there go through the game server,
where JoF moderates them (against advertising), and the player takes no encrypted
whispers there either. The client reads it whatever the mod's switch: the family table is
`e2e_whispers::PLAIN_FAMILIES` ([hub-chat.md](hub-chat.md#encrypted-whispers)).

`jof.abilitySounds` (archived, on by default) controls only local Stasis/Repulse
confirmation audio; muting it does not withdraw the broader server opt-in.
The JoF mod owns the documented command/sound table and independent 250 ms gates
in [feedback.rs](../crates/sjk-mod-jof/src/feedback.rs). Only server `stasis` and
`repulse` confirmations trigger installed local channel-1 sounds; key presses
do not. Gates reset on unload/map changes and handle server-time rewinds.
Audio is loaded only when the JoF mod and both opt-ins permit playback, including
when enabled after joining. Missing installed audio stays silent. Turning the
feature off stops queued/current confirmations. EJK #117 has no separate Dash
confirmation cue. Nothing is bundled or downloaded, and base SJK owns none of
these command names, sounds or repeat rules.

### Honors and Echoes

`jof.honorNames` (archived, on by default; 26/10/2026, JoF's request) calls medals
"Honors" and holocrons "Echoes" (an Honor, an Echo) in every menu and message while
`mod_jof` is on, on every server and in the menus too, not only on JoF servers. Off, or
with `mod_jof 0`, they keep SJK's names. Only the client's own words change: what the
hub, players or a map wrote (achievements' hub text, announcements, names, chat, the
"Holocron FFA" game type, the lore holocrons in shader names, the Illuminate holocron
toy) keeps its words. The words and the switch are
[terms.rs](../crates/sjk-viewer/src/terms.rs); the setting is in Settings, Mods.

### SJK chat on JoF servers

[sjk-mod-jof/sjk_chat.rs](../crates/sjk-mod-jof/src/sjk_chat.rs) says what the player
says on the SJK chat in a JoF server's All chat too, during a match, once the hub took
it, as JoF EternalJK does. JoF asked for it so that SJK chat does not split their
players away from All chat (Sol's decision, 22/10/2026, after Milamber's #194). It is
part of JoF tools and has no switch of its own: with `mod_jof 0`, on any other server,
in local games and in the menus, nothing is said anywhere but the SJK chat, and the base
client holds no server-specific rule for it. The mod owns the `say` commands (tag, id at
the end, pieces of 149 bytes) and the words below; the viewer bridges them while the mod
is loaded (`sjk_chat_frame.rs`, `GpuState::sjk_chat_said_in_all`). How copies show once
for SJK chat players, labelled ALL/SJK, is the core's and works on every server:
[hub-chat.md](hub-chat.md#said-on-the-game-server-too).

So players used to a private SJK chat are not misled:

- **The notice.** The first time the player is in a match on a JoF server with SJK chat
  on, a card over the game menu says SJK chat is said in All chat here, that JoF asked
  for it, and that elsewhere it stays as it was. Got it (Enter or Escape, taken only
  0.6 s after the card opens, so a jump or a key meant for the game does not) turns
  `jof.sjkChatNotice` (archived, on by default) off; turning it back on shows the card
  on the next JoF server. The card is the core's generic mod notice
  ([mod_notice_card.rs](../crates/sjk-viewer/src/mod_notice_card.rs)): a mod gives the
  words and the setting, the core draws it in the SJK UI's look in every menu style.
- **The note.** While the in-game composer is on the SJK channel there, a small red box
  on the channels' row reads "Also said in All chat: everyone on this server reads it".

What stays in SJK chat only, even there (Sol, 22/10/2026):

- **GIFs.** A message with a GIF (a GIPHY link, a favourite sent with `sjkgif`) is never
  said on the server: its link would only spam the server's other players
  (`player_identity::may_go_to_server`).
- **Send only in SJK.** The SJK chat page offers every player a Send only in SJK button
  under Send while copies apply; its hover card says JoF asked for SJK chat to be said
  in All chat so the community does not split, so a message only in SJK should be
  justified (`player_identity::chat_sjk_only`).
- **Staff's Shift.** A staff key holding Shift while sending (Shift+Enter in the in-game
  composer or on the page) sends only in SJK.

### Destruction client controls

Destruction's contract, name, installed icon and capability policy live in
[sjk-mod-jof/destruction.rs](../crates/sjk-mod-jof/src/destruction.rs).
It is offered only on a recognized JoF server with `mod_jof 1`, nonzero
`g_forceDestruction` and the living player's known bit 21, outside spectator/follow.
`jof.destructionSupport` is an archived mod setting: Auto (`-1`) follows the flag,
Off (`0`) disables it, and Grant contract (`1`) accepts the bit without the flag.
The override cannot load the mod on another server or grant a power.

The Force bar, SJK selector, quick wheel and key-binding editor support the entry.
`force_destruction` keeps existing binds working; completion loads with the mod.
Where the entry is not active (another server, the mod or the setting off), the
command goes to the server unchanged, as it did before the mod owned it.
`+useforce` sends it once per press. Slot 21 is local-only and never becomes the
network Force selection. Loaded extensions use EJK #338's anchor order; base SJK
keeps its original wheel and never treats bit 21 as Destruction. Turning the mod
off or leaving clears that local selection. The optional icon is read/uploaded
only on first activation in the installed world; absent assets use Lightning.
Settings > Mods (SJK) and Game options (classic) expose the preference.

### Dialogue window and node audio

[sjk-mod-jof/dialogue.rs](../crates/sjk-mod-jof/src/dialogue.rs) owns EJK
#301/#307/#342's `jof_dialogue` parser, bounded node state and serial-safe replies.
The SJK window opens only on recognized JoF servers with `mod_jof 1` and
`jof.dialogue 1` (archived, on by default). It uses SJK's UI in every menu style;
Settings > Mods and classic Game options expose the preference.

The server supplies speaker/body text and up to twelve choices, five visible at
a time. Arrows or mouse wheel scroll without wrapping; hover selects, click or
Enter answers, 1-9 and 0 choose responses 1-10, and Escape/Leave conversation
cancels. Opening releases held gameplay keys. Replies are generated as
`dialogueresponse <serial> <choice>`, never executed as scripts; answer retries
are limited to one per second. Stale commands and duplicate show messages are
ignored. Replacing/closing the node clears unsent old replies. Disconnects, map
changes and disabling/unloading JoF tools reset the modal.

An optional `show` sound plays once from installed VFS data on
`ENTITYNUM_NONE`/`CHAN_VOICE_GLOBAL`, listener-relative. A replacement or close
stops the previous voice, including queued decodes. Missing audio leaves the
window usable. Paths are bounded and reject absolute/traversal forms. Nothing is
bundled or downloaded; ordinary silent show messages remain valid. Other servers,
mod-off sessions and builds without `mod-jof` do not interpret this extension,
open its window, send responses or load its node audio. Demos send no replies.
The remote server retains all quest conditions, stages, rewards and persistence;
this is no native SJK quest server system.

## Plugin options

`cp_pluginDisable` is the userinfo bit mask a JA+ or jaPRO server reads, and the clients
made for them read too, to switch plugin features for one player (the setting and its
default, 1536, are the core client's: [networking.md](networking.md)). Players used to
type a number (`cp_pluginDisable 1536`); since 29/10/2026 (Kushina's idea) Settings > Mods
shows each option SJK can honour as a switch, under JA+ plugin options and jaPRO plugin
options (classic: Game options), and `japlus.plugin` and `japro.plugin` are JoF
EternalJK's `plugin` command.

- **No double negatives** (Sol, 29/10/2026). Most bits switch a feature off when set, a
  few switch one on. Every switch and every line of the commands is named after the
  feature and says whether it is on: "Black sabers: On", never "No black sabers: Off".
  Which way each bit works comes from where JoF EternalJK reads it (its cgame, and the
  jaPRO server code it carries); the tables keep the evidence with each entry
  ([sjk_mod::plugin_options](../crates/sjk-mod/src/plugin_options.rs)).
- **Per family, as EternalJK.** Each mod holds its family's table in EternalJK's order
  (`japlusPluginDisables[]`, `japroPluginDisables[]`, `cg_consolecmds.c`), so an option
  has the number EternalJK's `plugin` gives it: `japlus.plugin 3` is black sabers, as in
  EJK. With no argument the command lists the offered options with their state, then the
  ones SJK does not offer; with a number it switches that option, or says why it cannot.
- **Only what works in SJK.** An option whose bit changes how the server moves the player
  is hidden while SJK's movement prediction does not follow it (switching it would snap
  the view back), and one that only changes what a client draws is offered only if SJK
  draws that difference. Hidden options stay in the table with the reason, so offering
  one later is a change of its entry.
- **One bit each.** A switch flips its bit and keeps every other one, so a value set by
  hand or imported from another client keeps working, bits no option names included.
  A profile that never touched it keeps sending 1536. The cvar is a userinfo one: a
  change in a match reaches the server with the next userinfo update, as a name change
  does.

| Family | # | Bit | Switch (on while the bit is) | Offered |
| --- | --- | --- | --- | --- |
| JA+ | 0 | 0 | New drain effect (set) | No: SJK draws the stock Drain only |
| JA+ | 1 | 1 | See others in duels (set) | Yes, read by `cg_duelHideOthers` Auto ([duel_hide.rs](../crates/sjk-client/src/duel_hide.rs)) |
| JA+ | 2 | 2 | End-of-duel camera (clear; nothing reads it, unverified) | No: the JA+ plugin's own camera |
| JA+ | 3 | 3 | Black sabers (clear) | Yes ([black_sabers.rs](../crates/sjk-viewer/src/black_sabers.rs)) |
| JA+ | 4 | 4 | Auto replier (clear) | No: the JA+ plugin's own |
| JA+ | 5 | 5 | New Force effects (clear) | Yes ([force_fx_policy.rs](../crates/sjk-viewer/src/force_fx_policy.rs)) |
| JA+ | 6 | 6 | New death messages (clear) | No: nothing reads it, what it changes is unverified |
| JA+ | 7 | 7 | Force Sense screen effect (set) | No: SJK does not draw the Sense ring yet |
| JA+ | 8 | 8 | Alternate dimension effect (clear) | No: the JA+ plugin's own look |
| JA+ | 9 | 9 | Staff on the back (clear) | Yes ([holstered_sabers.rs](../crates/sjk-viewer/src/holstered_sabers.rs)); the JA+ server then plays the reach over the shoulder |
| JA+ | 10 | 10 | Ledge grab (clear) | No: movement |
| JA+ | 11, 12 | 11, 12 | New DFA, attack and alt attack (clear) | No: movement |
| JA+ | 13 | 13 | Single-player cartwheel (clear) | No: movement |
| JA+ | 14 | 14 | Download redirect (clear; name and label disagree, unverified) | No: SJK downloads from the game server only |
| JA+ | 15 | 29 | Holstered sabers (clear) | Yes ([holstered_sabers.rs](../crates/sjk-viewer/src/holstered_sabers.rs)) |
| jaPRO | 0 | 3 | Black sabers (clear) | Yes |
| jaPRO | 1 | 19 | Force jumps (clear; set, held jump bunny hops where the server lets players choose) | Yes, since 30/10/2026: the prediction follows it ([pmove_japro.rs](../crates/sjk-game-jka/src/pmove_japro.rs), Lumaya's PR #282) |
| jaPRO | 2, 3 | 20, 21 | Rolls, Cartwheels (clear) | No: movement |
| jaPRO | 4 | 22 | Jawa run (set) | No: the run is picked by movement code SJK does not predict |
| jaPRO | 5 | 23 | Respawn where you fell, after a duel death (clear) | Yes: a server respawn |
| jaPRO | 6 | 24 | Checkpoints on screen (clear) | Yes: the server's centre print |
| jaPRO | 7 | 25 | Checkpoints in chat (set) | Yes |
| jaPRO | 8 | 26 | Damage numbers (clear) | Yes: prints, chat or score plums |
| jaPRO | 9 | 27 | Shots from the centre (set) | Yes: the server's muzzle point |
| jaPRO | 10 | 28 | Checkpoints in console (set; in place of the chat) | Yes |

Bits 15 to 18 (ja++'s no kata, butterfly, stab and DFA) are on neither of EternalJK's
lists and are left out. Several switches sit beside a client setting of their own,
as in EJK: `cg_blackSabers` (JA+ black sabers), `cg_drawHolsteredSaber` (Holstered
saber hilts) and `cg_forceAnimFX` (Force animation effects), each of which can turn the
feature off too, and `cg_duelHideOthers` (Hide others in duel), on whose Auto See
others in duels decides.

The Settings rows are derived switches named after the command that switches them
(`japlus.plugin 3`), with the table's help line
([plugin_rows.rs](../crates/sjk-viewer/src/settings/plugin_rows.rs)). Like a mod's
settings they show whether the mod is on or not, and exist only in builds with the mod.

## Verification

Unit tests cover the mod crates (teleport positions, player lookup, plugin options and
their polarity, the `plugin` listing and switching, login matching, JoF detection) and
the client's switch, completion and queue (`mods::tests`, `console_command::tests`),
the plugin option rows (`settings::plugin_rows::tests`: default and existing masks,
switching, the userinfo) and `cp_pluginDisable` marking the userinfo for sending
(`console_session::tests`). Nothing has been checked on a JA+ server yet: the teleports
need a server where the tester is a JA+ admin, and the plugin options have not been
switched in a match.
