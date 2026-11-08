# Player identity and the SJK hub

SJK players can be recognised across servers without accounts or passwords. Each
install keeps an Ed25519 key; a small web service, the SJK hub, maps public keys to
the in-game names they have worn, an optional bio and a "verified" flag its operator
sets; and while a player is on a game server, their client tells the hub which slot
they are in, so other SJK clients can mark that player on the scoreboard and put a
verified tick after a verified player's nameplate ([Ticks](#ticks)). A player does nothing: the
key is made, registered and kept up to date on its own, under the name they play
with. The game works without the hub; the badges and profiles are an extra.

This page is the design and the current limits. The player-facing summary is
[client.md](client.md#identity).

## Pieces

| Piece | Where |
| --- | --- |
| Key file, signing, hub client, background service | [sjk-identity](../crates/sjk-identity/src/lib.rs) |
| Settings and live-session glue | [player_identity.rs](../crates/sjk-viewer/src/player_identity.rs), [identity_frame.rs](../crates/sjk-viewer/src/identity_frame.rs) |
| Scoreboard mark | [identity_mark.rs](../crates/sjk-viewer/src/scoreboard/identity_mark.rs) |
| Identity page, `identity` command | [identity_panel.rs](../crates/sjk-viewer/src/identity_panel.rs), [identity_command.rs](../crates/sjk-viewer/src/identity_command.rs) |
| Players page and player reports | [players.rs](../crates/sjk-viewer/src/ingame_menu/players.rs), [player_report.rs](../crates/sjk-viewer/src/player_report.rs) |
| Medals: catalogue, ribbons, pictures | [medals.rs](../crates/sjk-viewer/src/medals.rs), [medals/](../crates/sjk-viewer/src/medals/), [identity_panel_medals.rs](../crates/sjk-viewer/src/identity_panel_medals.rs) |
| New medal pop-up, its ceremony and `debug_medal` | [medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs), [medal_popup/](../crates/sjk-viewer/src/medal_popup/) |
| Bio rules (shared word for word with the hub) | [bio.rs](../crates/sjk-identity/src/bio.rs) |
| Profile page, `profile` command (the achievements and medals are the Collection's: `collection_panel.rs`, `collection` and `achievements` commands) | [profile_panel.rs](../crates/sjk-viewer/src/profile_panel.rs), [profile_panel_view.rs](../crates/sjk-viewer/src/profile_panel_view.rs), [console_profile_page.rs](../crates/sjk-viewer/src/console_profile_page.rs) |
| Profile card (main page, in-game menu) | [profile_card.rs](../crates/sjk-viewer/src/profile_card.rs) |
| Pictures: wire, cache, crop and scale, kept files, `sjkavatar` | [avatar.rs](../crates/sjk-identity/src/avatar.rs), [avatars.rs](../crates/sjk-viewer/src/avatars.rs), [avatars/picture.rs](../crates/sjk-viewer/src/avatars/picture.rs), [avatars/store.rs](../crates/sjk-viewer/src/avatars/store.rs), [avatar_command.rs](../crates/sjk-viewer/src/avatar_command.rs) |
| Staff requests, Staff page, `staff` command | [staff.rs](../crates/sjk-identity/src/staff.rs), [staff_panel.rs](../crates/sjk-viewer/src/staff_panel.rs), [staff_panel_view.rs](../crates/sjk-viewer/src/staff_panel_view.rs), [console_staff_page.rs](../crates/sjk-viewer/src/console_staff_page.rs) |
| Log requests: wire, session log, the player's card | [logs.rs](../crates/sjk-identity/src/logs.rs), [session_log.rs](../crates/sjk-viewer/src/session_log.rs), [log_request_card.rs](../crates/sjk-viewer/src/log_request_card.rs) |
| Achievements: catalogue, counts, tracker | [achievements.rs](../crates/sjk-viewer/src/achievements.rs), [achievements/tracker.rs](../crates/sjk-viewer/src/achievements/tracker.rs), [achievements_frame.rs](../crates/sjk-viewer/src/achievements_frame.rs) |
| Achievements: medallion look, unlock pop-up | [achievements/medallion.rs](../crates/sjk-viewer/src/achievements/medallion.rs), [achievement_toast.rs](../crates/sjk-viewer/src/achievement_toast.rs) |
| SJK chat and emotes | [hub-chat.md](hub-chat.md) |
| Unlocks and looks (blade skin, Illuminate) | [unlockables.md](unlockables.md), [looks.rs](../crates/sjk-viewer/src/looks.rs), [looks_frame.rs](../crates/sjk-viewer/src/looks_frame.rs) |
| Asset packs: list, download, cache | [assets.rs](../crates/sjk-identity/src/assets.rs) |
| Holocron drops: tiers, active flag, pop-up, chat lines, staff tools | [holocrons.md](holocrons.md), [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs), [holocron_popup.rs](../crates/sjk-viewer/src/holocron_popup.rs) |
| The hub itself and its protocol | repository Sol-Vulpes/SJK-hub (`PROTOCOL.md`) |

The hub is a separate repository because it is deployed on its own schedule. The
client and hub each carry the protocol types; `PROTOCOL.md` has a signed-request test
vector that both test suites check, so a drift in either shows as a failing test.

## What happens

1. With `cl_identity` on (the default), the first start creates `identity.key` in
   the settings folder beside `config.cfg`. With it off, no key is made.
2. With `cl_hubUrl` set, a worker thread registers the key at the hub (a request
   signed by the key, which proves the client holds it) with the in-game name the
   player wears (the `name` setting), and fetches its profile. When the name changes
   it registers again with the new one. The hub keeps each key's worn names (20 most
   recent, with when it first and last saw each), and a profile's name is the one
   worn last unless the operator gave the key another. Nobody chooses a hub name, so
   two keys may wear one name: the name proves nothing, the key does. A failed
   registration is tried again after 10 seconds, doubling up to 2 minutes. Until the
   hub answers, anything the player sends (SJK chat, emotes, reports, trades...) is
   refused with why (`Worker::not_connected`): the identity off (`cl_identity 1`), no
   hub (`cl_hubUrl`), or "not connected to the hub yet", with the last failure if
   there was one. Nothing goes to the hub then, so the error is its own
   (`HubError::NotConnected`), printed without a hub code: no "the hub refused it",
   and a chat ban refused this way says the same words. A new setting forgets the
   last hub's failure. Before 27/10/2026 every case said "is identity on, cl_identity 1?",
   which misled verified players who joined a server before the registration had been
   answered. Each change of the service's status goes to `session.log` once
   (`player_identity::log_status`): `identity: registering with the hub`, `identity:
   registered with the hub`, or `identity: hub request failed: ...` with the error (a
   hub's 429 shows as `the hub refused it (rate_limited)` or its quota's code).
3. While the client is in a live, non-local session, the thread repeats a *claim*
   every 45 seconds: "this key is in slot N of server S, shown as NAME". Claims
   live 90 seconds at the hub and are withdrawn when the player leaves or quits. A claim
   also says `"active": true` while the player is actively playing (in their own view,
   mid-match, out of the menus, with input in the last two minutes), which is how the hub
   counts play time toward the next holocron ([holocrons.md](holocrons.md#earning)); the
   flag is left out otherwise.
4. The thread reads the hub's list of claims for the server every 5 seconds (15 before
   27/10/2026; the hub gives the reads an allowance of their own since then). The
   scoreboard marks a row with SJK's emblem, in gold when verified, the player card
   shows the hub name, and a verified player's nameplate gets a gold badge after the
   name (`ui_renderer/verified_badge.rs`, read once a second), when a claim names
   that slot and its claimed name matches the name the game shows there (compared
   after lower-casing and dropping colour codes and symbols). The local player's own
   plate (`cg_nameplateSelf`) has the badge when their own key is verified. Since
   27/10/2026 the badge is the player's tick ([Ticks](#ticks)), and the scoreboard's and
   the player card's emblem stays white with the tick small on its foot, where the
   trust mark goes, instead of turning gold (Sol's call). A claim
   made from another client that takes part in SJK chat (the hub's `client` in each
   presence entry, since 27/10/2026: today `ejk`, JoF EternalJK) shows that client's
   icon in its own colours on the scoreboard and the player card in place of SJK's
   emblem, as its chat lines do ([hub-chat.md](hub-chat.md#how-a-line-looks),
   `cl_sjkChatClientIcons 0` gives everyone SJK's emblem); the tick still sits on its
   foot. Since 08/11/2026 (Sol's request, "the logos on the nameplates too with the
   tick") that player's nameplate shows the client's icon too, small after the tick
   (`player_identity::slot_clients`, read with the marks; none for SJK, whose players'
   plates need no emblem, and none with `cl_sjkChatClientIcons 0`). A client that reads the chat without claiming its slot (today JoF EternalJK,
   whose claims wait in JediofFreedom/JoF_EJK#405) gets its players marked by name
   alone (Sol's request, 08/11/2026, "assume they are on JoF EJK"): a scoreboard row
   no claim marks wears that client's icon when its name is the name of exactly one
   online reader of the chat (the feed's `people`: their hub name, or the name on their
   newest message), never a bot's row, and that player's nameplate shows the icon
   small after the name, as a claim from that client would, but without a tick
   (08/11/2026). The client assumed is the
   `unclaimed` entry of [chat/client_icons.rs](../crates/sjk-viewer/src/chat/client_icons.rs);
   the match is [scoreboard/chat_readers.rs](../crates/sjk-viewer/src/scoreboard/chat_readers.rs).
   Since a name proves nothing, the row gets the icon alone, no tick, trust mark,
   medals or emoji (on the nameplate too), and the player card and the game chat stay
   as they were. Left out:
   the player's own key, readers holding a claim on this server or playing a match (a
   claim marked active, which only SJK makes), readers whose newest message came from
   SJK, a reader whose name is on a row a claim marks, and a name two rows or two
   readers share. An SJK player out of a match elsewhere can still lend their name's
   icon to a namesake here, which is why it stays an icon. With
   `cl_sjkChatClientIcons 0` such rows get no mark, since SJK's emblem would say more
   than a name shows.
5. Once a claim is accepted the thread also tells the hub the player's *look* (the
   blade skin they wear and whether their Illuminate holocron is lit), which lives on
   the claim; other SJK clients on the server read it from the claims and the feed and
   draw it, under the same name rule as the badges ([Unlocks and
   looks](#unlocks-and-looks)).

Verification is the operator's alone (the hub's `verify` command or its operator API,
which list every key with its worn names); a player asks for nothing and sets nothing.

### Ticks

Since 27/10/2026 (Sol's request) a person's name shows one tick, which the hub works out
and sends as `tick` with profiles, presence, chat messages and the chat's people (the
hub's `PROTOCOL.md`, "Ticks"; `sjk_identity::Tick`, drawn through
`Tick::shown`, which turns a verified player's empty `tick` into gold). The highest
applies:

| Tick | Who | Look |
| --- | --- | --- |
| SJK creator | Sol | a sun whose twelve rays turn and over which a glint passes every four seconds |
| SJK staff | the SJK team | a silver starburst of sixteen points (since 29/10/2026, the seal before), edged in the SJK TEAM pill's teal, its tick teal |
| JKR creator | Bishop | the seal rusted, in the JKR CREATOR pill's rust |
| JoF | verified players of JoF (JoF Academy too) | the seal in cyan |
| Sith | verified players of the Sith clan `{{S}}` | the seal in deep crimson, its rim dark so it never reads as the chat ban's red disc |
| Jawa | verified players of the JAWA clan | the seal in leather brown |
| Gold | every other verified player | the gold seal |

A group's tick is staff's choice when they verify, never read from a name or clan tag
(the [clan emblems](#clan-tags) are): on the Staff page's Overview, the row of tick
buttons under Verify (Gold, JoF, Sith, Jawa, each with its tick; the one drawn now lit)
verifies the player with that tick, or changes it when they already are; Verify alone
keeps the group they had. A group shows only while the player is verified; staff and
the creators wear their own whatever it is. Hubs older than ticks send none, so every
verified player shows gold, as clients older than ticks show everyone. Since 03/11/2026
the JoF and Sith groups also open the clan chats ([hub-chat.md](hub-chat.md#clan-chats)):
verifying a player with one of those ticks lets them in, unverifying takes them out.

The ticks are computed at start like the gold seal
([tick_badge.rs](../crates/sjk-viewer/src/ui_renderer/tick_badge.rs), icon cells after
JoF EternalJK's). The sun is 40 cells, which the renderer steps through at 10 a second
when it lays out a frame's icons (`ui_renderer::animated`), so it moves wherever a tick
is drawn at no cost beyond any icon's. `SJK_TICKS_PREVIEW=<png>` with the
`tick_badge` tests writes every tick side by side. The player card writes VERIFIED in
the tick's colour (`ui_renderer::tick_accent`).

Nothing blocks a frame: the viewer compares settings and place with what the thread
was last told twice a second, and the scoreboard re-derives its marks only when the
hub's roster or its own rows change. While registered, the thread also reads the
player's own profile again every ten minutes, so the verified flag and medals the SJK
team changes reach a running client. It reads it sooner, at most once every 30
seconds, when its unlocks look out of date: the hub refused a blade skin
(`not_unlocked`), or the feed relayed a look of the player's own key, newer than the
last one sent, with another blade skin (staff took it back).

Opening the game menu (Escape on a server, since 27/10/2026) reads what its pages show
of the hub at once: the profile (achievements, medals, unlocks, kyber crystals), the
holocron progress and the trades, at most once every 15 seconds (`Service::menu_opened`,
`player_identity::follow_game_menu`), and sends achievement counts that changed since the
last sending, at most once every 30 seconds (the hub takes 40 in 10 minutes from an
address). A hub that refused one of them (an older hub, a quota) keeps its longer wait.

## What a badge proves

A badge says "a registered SJK key claimed this slot under this name". It does not
prove the player in that slot holds the key, because stock and JA+/JoF servers
publish only fixed fields of a player's userinfo to other clients (SJK's own server
does the same, `bridge_userinfo.rs`), so nothing in-band can carry a proof. The
hub limits the damage: a claim fails while another key holds the same slot under
the same name, display names that normalise to another's are refused, and the claimed
name must match what the game shows. The worst a false claim does is label someone
else's slot with the claimant's own profile; it cannot take another key's name or
verified flag. Badges are for recognition, not for granting anything.

A confirmed tier (SJK's own server adding a hub-signed ticket to the player string)
is planned, not implemented.

## Privacy

With `cl_identity` on and `cl_hubUrl` set the hub receives the player's public key
and in-game name at start and whenever the name changes, the game server address,
slot and in-game name for as long as they play, and sees their IP address. Claims
are deleted 90 seconds after they stop being repeated; profiles and the worn-name
history stay until the operator removes them. While a claim lives, it carries the
player's look (blade skin and Illuminate), dropped with the claim, and whether the
player is actively playing. Since holocrons (10/10/2026) the hub also keeps, until the
operator removes the identity, the key's active play time carried toward the next
holocron, when it last claimed and every holocron dropped for it (tier, time, source, a
staff note and the address it dropped from, for a per-address limit); the profile lists
the counts and recent holocrons publicly and legendary and mythical drops are told to
every SJK client with the key's hub name ([holocrons.md](holocrons.md#what-the-hub-stores-and-privacy)). Once registered, the client also asks for the hub's list of
asset packs at start and every 6 hours and downloads the packs it lacks, which tells the
hub which packs the key fetched and when ([Asset packs](#asset-packs)). With either setting
off the client sends nothing. Since 06/10/2026 `cl_hubUrl` defaults to `https://sjk.dfox.app` so players
set nothing: a default install makes a key and tells that hub where it plays and whether it is active. The
Identity page, the setting's help and the changelog say what is sent and that
`cl_identity 0` stops it.

Every request to the hub names the build in its `User-Agent`: `SJK/<version> (<commit>)`
(`build_info::hub_agent`; since 20/10/2026 with the commit, `SJK/<version>` before). The
hub keeps, per key, each agent it signed with and when it was first and last seen, until
the operator removes the identity, and checks a release's commit against the public
`sjk-v<version>` tags, so the hub can tell an SJK release from a changed build or
another client. It is what the game says it is, not proof.

A model list ([Installed models](#installed-models), since 15/10/2026) is sent once
registered, while `cl_shareModels` (default 1) is on: the names of the player model
folders the game has, signed with the key. The hub keeps the latest list per key until
it is replaced, the player turns the setting off (an empty list, which deletes it) or the
operator removes the identity. Anyone may ask the hub whether the player in a server's
slot has a given model.

A bug report (Escape, SJK, Report a bug) is sent only when the player presses Send (or
Ctrl+Enter; Enter in its box starts a new line, which the hub stores as a space): its text, the map, the client build, the game server's address and the in-game
name the player wears, signed with the player's key. The hub keeps it until the operator removes it.

A crash report ([Crash reports](#crash-reports)) is sent at the start after SJK
crashed, while `cl_crashReports` (default 1) and identity are on: the client build, the
operating system, the graphics adapter and driver, the map and game server, how long the
game ran, the error with where in the code it happened and the backtrace, and the last
console and diagnostic lines (chat lines are left out, the player's own folder is
replaced by `%USERPROFILE%`), with the in-game name the player wears, signed with the
player's key. The hub keeps the newest 5000 until the operator removes them. With
`cl_crashReports 0` nothing is sent and the report stays on the PC
(`crashes/last-crash.txt`).

Settings usage ([client.md](client.md#settings-usage), since 12/10/2026) is the one
thing the client sends with no identity at all, and whether identity is on or not:
while `cl_shareUsage` (default 1) is on, closing Settings sends which settings were
changed (their console names and values before and after as the rows show them, a
text setting's value never) and what was searched there, with the client build and
a random id drawn once per game run. No key, signature, name, server or map goes with
it, and the hub keeps no address for it. With `cl_shareUsage 0` nothing is sent.

Logs ([Log requests](#log-requests)) are sent only when the SJK team asked for them and
the player chose Send logs on the card that asks, or when the player sends them
unasked from the launcher's Doctor after seeing what goes. A request's logs hold the newest console and diagnostic
lines of this session and the one before (chat lines left out, the player's folder
replaced by `%USERPROFILE%`), the last crash report when it is under 14 days old, the
build, the operating system, the graphics adapter, the map and the game server, signed
with the player's key. Staff read them; the hub keeps the newest 300 requests. With
identity off nobody can ask. The session log itself (`logs/session.log` and
`logs/previous-session.log` beside the configuration) stays on the PC.

A player report (Escape, Players, or Escape, SJK, Report a player) is sent only by a
verified player who chooses a player and a reason and presses Send: the reason and its few
words, the reported player's slot and the name the game shows for them, the key the hub's
presence list shows in that slot (if any), the server's address and name, the map, the
build, the match clock and the in-game name the reporter wears, signed with the reporter's
key. The hub keeps it until the operator removes it.

A world note (`worldnote` twice, [client.md](client.md#player-card)) is sent
only when the player sends it, with `cl_identity` on: its text, the map, the build, the
game server's address, the in-game name the player wears, where the player stood (`setviewpos`), the aimed point, shader,
surface and entity, the shader remaps active then and the model aimed at, and a smaller copy of its screenshot (at most 1280 x 720), signed
with the player's key. The note dialog says so. The hub keeps it until the operator
removes it; the full note and screenshot stay on the player's PC either way.

A picture ([Pictures](#pictures)) is sent only when the player presses Use this
picture (or types `sjkavatar <file>` and then presses it): a 128-pixel square PNG the
client made from their file, signed with their key. Nothing else of the file goes:
not its name, its path or its metadata, and the hub writes its own copy holding the
pixels only. The picture is public, like the hub name: anyone may read it at
`/v1/avatar/<key id>`, and every SJK client that shows the player downloads it. The
hub keeps it until the player takes it down, the SJK team does, or the operator
removes the identity. Showing other players' pictures asks the hub for them by key
id, so the hub sees which players a client looks at (it already sees the servers it
plays on); they are kept in `avatars/` in the settings folder.

## Bug reports

[bug_report.rs](../crates/sjk-viewer/src/bug_report.rs) and
[report.rs](../crates/sjk-identity/src/report.rs). While the game menu is open, a Report a
bug button sits centred at the bottom of the screen (also Escape, SJK, Report a bug); it
closes the game menu and opens the text dialog (the SJK UI's main page has it too, on its
SJK page since 03/11/2026, Lex's idea: that report names the map `menu` and no server,
the map behind the menu being only its backdrop)
([text_dialog.rs](../crates/sjk-viewer/src/text_dialog.rs)): a panel in the middle of the
screen with a text box, a character count, Send and Cancel (Escape cancels too). The text
has an insertion point (Left, Right, Up, Down, Home, End, Backspace, Delete, Ctrl for
words, a click in the box) and a Send the rules refuse says why in a red band. As it is
typed or pasted, only letters and digits of any
script, spaces and `. , ! ? ' - : ( )` are kept, and since 19/10/2026 the characters
cvars and commands need, `_ / \ + = " ; [ ] < > #` (a hub from before then refuses
those); line breaks become spaces, up to 600
characters. Enter or Send checks the hub's rules (a refusal is shown in the panel, which
stays open) (10 to 600 characters, a few real words, no
long run of one character) and hands the report to the identity service, which sends
`POST /v1/report` signed with the player's key; the outcome (the hub's report number,
or why it refused, for example a quota) shows on the SJK UI's card, which waits for it
([sjk-ui.md](sjk-ui.md#report-a-bug-and-its-dialogs)), else as a centre print. The report carries the
in-game name the player wears (the service fills it from the `name` it already sends the
hub); the hub keeps it with the report and adds it to the key's worn names. The hub checks
everything again and limits reports per key (3 a day and 5 an hour; once verified 60 a
day and 20 an hour; none for staff; no repeat within a day), per address (3 in 10
minutes, 10 for a verified or staff key) and overall (500 a day, 5000 kept; raised
18/10/2026),
so a troll with fresh keys gets little through and nothing that is not plain words. The
operator reads them with the hub's `reports` command or `/admin/v1/reports`.

## Crash reports

[crash_report.rs](../crates/sjk-viewer/src/crash_report.rs) (the game's side) and
[crash.rs](../crates/sjk-identity/src/crash.rs) (the format, the folder and the sending),
since 11/10/2026. A crash leaves a report in `crashes/` beside the configuration
(`GameData/SJK` or `%APPDATA%\SJK`), in one of three ways:

- `panic`: the panic hook `install` sets in `main` (after the standard one, which still
  prints to stderr) writes the message, `file:line:column`, the thread, a backtrace and the
  last 150 lines of the trail. Any thread counts, also one whose panic the game survives
  (a model worker, a frame's submission); a session writes at most three, one per place.
- `error`: the fatal error `main` prints before closing (`crash_report::fatal_error`).
- `unclean`: each session keeps `crashes/running-<pid>.json`, written again every 30 s
  with where the player is and the last 60 lines; a clean exit removes it. The next start
  turns a marker left by a dead process that wrote no report into a report: an abort, a
  stack overflow, a driver or Windows ending the process, or the player killing it. On
  Windows a running game holds its marker open without sharing deletion, so a second
  game cannot take a live one's; elsewhere a marker written in the last 90 s counts as
  live.

The trail is every console line as it is kept (through `Shell::set_line_tap`), colour
codes removed, except chat lines (`push_chat_line` keeps them out) and the echo of a
chat message typed in the console (`say`, `say_team`, `tell`: `CHAT_COMMANDS` in
[crash_report.rs](../crates/sjk-viewer/src/crash_report.rs), and a mod's chat commands
on any server, JA+ tools' `say_team_mod`, `clansay`, `amsay` and `ampsay`
([mods.md](mods.md#passwords)); the scrollback keeps it),
and every
`log::progress` diagnostic. Each report is also written as plain text to
`crashes/last-crash.txt`, which a player can send by hand. Paths under the player's own
folder become `%USERPROFILE%`. Passwords never enter the trail: the console masks a
password typed or listed, and scrubs the secret cvars' values from every line, before
the tap sees it
([client](client.md#passwords-in-the-console)), so the trail, the session log, crash
reports and log uploads carry the mask.

With `cl_crashReports` on, the report waits in `crashes/pending/` (20 at most, the oldest
make room). Once registered, the identity service sends what waits (`POST /v1/crash`,
hub `PROTOCOL.md`, "Crash reports") at start and every hour, oldest first: one the hub
took, or already had (409), moves to `crashes/sent/` (the newest 10 stay); one it refuses
for good (400, 403, 413) is dropped; at the first it cannot take now (no answer, a limit,
a hub without crash reports) the rest wait 10 minutes. A report older than 14 days is
dropped unsent. Text is cut to the hub's limits first (message 4000 characters,
backtrace 16000, log 16000 keeping its newest lines, the body under 45 000 bytes), and a
name the hub refuses is sent empty instead. The console says when SJK closed
unexpectedly last time and when a report went. With `cl_crashReports 0` only
`last-crash.txt` is written, and identity off sends nothing either.

The hub groups reports by kind, the location's file and the message's first line (digits
aside), so one bug across builds is one group; staff read them on the Staff page's
Crashes tab ([Log requests](#log-requests)).

## Log requests

Since 11/10/2026 (Sol asked: friends with problems, their logs collected through the
hub, and crash reports on the Staff page). [logs.rs](../crates/sjk-identity/src/logs.rs)
(the wire), [session_log.rs](../crates/sjk-viewer/src/session_log.rs) (what is sent),
[log_request_card.rs](../crates/sjk-viewer/src/log_request_card.rs) (the player's card)
and the Staff page's Logs and Crashes tabs ([Staff](#staff)); hub `PROTOCOL.md`, "Log
requests".

**The session log.** Every line of the crash trail ([Crash reports](#crash-reports): the
console's lines without chat, colour codes removed, and the `log::progress` diagnostics)
is also written to `logs/session.log` beside the configuration, from the first line of
the session (the lines kept before the configuration was known are written first). A
start renames the last one `logs/previous-session.log`, so after a freeze or a crash the
bad session's lines survive the restart. Past 16 MB the file starts again with the
newest lines. It is written whatever the settings; nothing sends it by itself.

**Asking.** A staff member chooses the player on the Staff page, opens the Logs tab and
presses Request logs; the note field's text goes with it (one line, at most 200
characters). A player has at most one open request (the button is off while one is
open); a request nobody answered expires after 7 days.

**The player's side.** Once registered, the identity service asks the hub for the
player's open requests (`GET /v1/log-requests`) at the registration and every 2 minutes
(a hub that refuses, one from before log requests, is asked again an hour later). The
console says when one comes in ("<name> from the SJK team asks for your game's logs:
open the menu to answer"); the card ([sjk-ui.md](sjk-ui.md#log-request)) waits for the
main page or the game menu, never interrupting play, and says who asks, their note,
what the logs hold, that the Windows user name is left out and that nothing goes unless
the player chooses Send logs. Send logs (Enter) gathers the logs then
(`crash_report::log_upload`): a header (time, build, system, graphics adapter, map,
server), the newest 1.5 MiB of this session's log, 512 KiB of the previous one's (200 KB and 120 KB until 05/11/2026) and the
last crash report (40 KB, when under 14 days old: its head with the message and backtrace
whole, then its newest last lines), the player's folder replaced by
`%USERPROFILE%`; the service fits it to the hub's limits (2 000 000 characters, a body
under 4 MiB; the oldest lines go first, at a line) and sends it (`POST /v1/logs`).
Don't send (Escape) tells the hub (`POST /v1/logs/decline`). The request leaves the list
at once either way; one the hub could not be reached for comes back with the next
listing to answer again. The console says what became of the answer.

**Unasked logs.** Since 08/11/2026 (Sol asked, after a friend's game never started and
then froze on a white screen without sending anything) a player can send their logs
without a request: the launcher's Doctor has Send my logs, for a game that does not
start or freezes (sjk-launcher's `doctor.rs`). The player sees what goes and may write a
note; `Hub::send_own_logs` sends it (`LogUpload::own_body`, the note one line of at most
500 characters, `own_note`). The hub keeps it as a request already sent with no staff
member; the Staff page's Logs tab marks it "unasked" (`LogRequestRow::own`). The hub's
limits are its own: the same logs within a day refused, 3 an hour and 5 a day per
person, 10 a day per address, 100 a day for the hub (hub `PROTOCOL.md`, "Log requests").

**Reading.** The Logs tab lists the requests (newest first, 12 shown: player, state and
date, size, who asked; Refresh reads them again). Open on a sent one reads its logs
(`StaffQuery::Log`), which the page saves as `staff/sjk-logs-<id>-<name>.txt` beside the
configuration and opens in the system's text editor. The Crashes tab lists every
player's crash reports by kind (the hub's signature: kind, file, how many reports from
how many players, the last date and the message's first line); a kind lists its newest
reports (player, date, build), and Open saves one whole as
`staff/sjk-crash-<id>-<name>.txt` and opens it. Each of these is a signed staff request
the hub keeps in its staff log.

## Player reports

[players.rs](../crates/sjk-viewer/src/ingame_menu/players.rs) and
[player_report.rs](../crates/sjk-viewer/src/player_report.rs). The game menu's Players page
is a small scoreboard of everyone on the server: each connected client's name, side, score
and ping (the client asks the server for scores every two seconds while it shows) and what
the hub knows of them (an SJK player, or a verified one, from the presence list), sixteen a
page with More players... and Back under them. The SJK UI draws it as a table with the
chosen player's card on the right (main page > Players); the classic menus list it in the
SJK pop-up's Report a player. Enter on a player opens the Report page: seven
reasons (cheating, harassment or hate, griefing, exploiting a bug, an offensive name, spam
or advertising, something else) and Back. A reason closes the menu and opens the text
dialog for a few words (10 to 300 characters, the bug reports' alphabet and noise rules);
Send hands the report to the identity service, which sends `POST /v1/player-report`
signed with the player's key, and the outcome (the hub's number, or why it refused) shows
on the SJK UI's card, else as a centre print. The player is copied when chosen, so the
report names them as they were.

A player whom the reported one sent an encrypted whisper may report them whether verified
or not (25/10/2026): the chat card's Report opens the Report page for them, which says
"Their last encrypted whisper goes with it", and the report carries that whisper with
its sender's signature ([hub-chat.md](hub-chat.md#encrypted-whispers), "Reporting one").
The hub checks the signature (400 `whisper_unsigned`), takes it up to a day old, and
names the whisper's sender as the reported key (source `whisper`); the operator's panel
shows the whisper under the reason.

Otherwise only a verified SJK player may report (`player_identity::report_gate`): with the identity
off, the hub not answering yet, an unverified key or a game on this PC, the Players page
says why under its title (the classic pages in their first line), and the Report
page's reasons are dimmed, Back carrying the reason; yourself and bots cannot be reported
either. The service refuses an unverified key before anything is sent, and the hub checks
everything again: verified keys only, a live claim of the reporter on that server, nobody
reports themselves, 3 an hour and 10 a day per key, one report per key about one player a
day (the same server and name, or the same key), 20 a day about one player from everyone,
3 in 10 minutes per address, 300 a day and 5000 kept overall. The hub names the reported key
from its own live claim on that slot when the names match, else takes the client's. The
operator reads them with the hub's `player-reports` command or `/admin/v1/player-reports`
(also by reported key) and decides what to do.

## World notes

[world_notes.rs](../crates/sjk-viewer/src/world_notes.rs). A world note
([client.md](client.md#player-card)) is kept on the player's PC as before and, with the
identity on, also goes to the hub for the SJK team. The note dialog keeps the bug
reports' alphabet as it is typed, and Send checks the hub's note rules (3 to 4000
characters since 13/10/2026, 500 before; at least 2 letters, no long run of one
character). The identity service
sends `POST /v1/note` with the text and the selection: map, build, server, `view` (the
`setviewpos` x, y, z and yaw), the aimed point and normal, shader, BSP surface,
lighting, distance and the entity's class name (a name outside the hub's alphabet is
left out rather than the note refused), and the in-game name the player wears, as for a
report. Since 13/10/2026 it adds, only when the note has them (so a hub from before
still takes a note without them): `remapped`, the shader a remap draws over the noted
one; `remaps`, the remaps active when the note was taken (at most 64, the noted
shaders' first); and `model`, the model the note is about (path, class, entity number,
origin, skin and at most 32 surface shaders with their remaps). A hub from before
13/10/2026 refuses a note carrying them, or longer than 500 characters: the hub goes
first ([`report.rs`](../crates/sjk-identity/src/report.rs) `NOTE_MAX`, the hub's
`notes.rs`). The hub answers with the note's number, which a
centre print and the console show, or why it refused. The screenshot writer also makes a
smaller JPEG of the same pixels (`capture::preview_jpeg`: fitted within 1280 x 720,
quality lowered until under 380 KiB) and hands it to the service, which sends it with
`PUT /v1/note/<id>/image` once the hub took the note; a picture that fails leaves the
note as it is. The hub limits notes per key (20 a day, 200 once verified, 30 an hour, no
repeat of the same text about the same shader within a day), per address (40 notes and
pictures in 10 minutes) and overall (1000 a day, 3000 kept), and takes a picture only
from the note's sender, once, within 10 minutes. The operator lists notes with their
pictures with the hub's `notes` command or `/admin/v1/notes`.

With the classic menus (`ui_menuStyle classic`) the dialog and the button take the
classic+ look ([text_dialog_classic.rs](../crates/sjk-viewer/src/text_dialog_classic.rs),
[classic-plus.md](classic-plus.md#pages)): the in-game pop-up box with its title band,
the text in a retail list box, gold Send and Cancel, the description line under the box,
and a gold REPORT A BUG on retail's red band at the bottom of the canvas. With the SJK UI
the dialog is its pop-up card and there is no button (Report a bug is on the in-game
menu's row of icons): [sjk-ui.md](sjk-ui.md#report-a-bug-and-its-dialogs).

## Medals

With the JoF tools on, medals are called "Honors" by default
([mods.md](mods.md#honors-and-echoes), `jof.honorNames`).

A medal is recognition the SJK team gives a player by hand: for testing SJK,
contributing to its code, finding bugs or donating to SJK. Since 10/10/2026 (Sol's decision)
a medal may bring one saber shader for as long as the player holds it: Tester the
Hologram blade, Contributor the Runic blade, Bug Hunter the Glitch blade; Supporter
brings none. The hub derives
it from the medal and takes it back with the medal's last award, off the player's look
too ([unlockables.md](unlockables.md#decisions)). Nothing else comes with a medal: no
setting, power or right, on any server. Players do not ask for medals or choose them;
the SJK team gives them.

| Medal | Id | For | Given again |
| --- | --- | --- | --- |
| Tester | `early_tester` | Helped test SJK. | no |
| Contributor | `early_contributor` | Contributed to SJK's code. | no |
| Bug Hunter | `bug_hunter` | Found bugs that got fixed. | no (since 21/10/2026: bug hunting is rewarded with holocrons) |
| Supporter | `supporter` | Donated to SJK's development (more than 1 euro on Ko-fi, since 13/10/2026; [sjk.md](sjk.md#donations)): given by the hub itself when the Ko-fi message holds the donor's key id, else by the SJK team. | no |

Tester and Contributor were called Early Tester and Early Contributor until 27/10/2026
(Sol's decision): the names lost "Early" and their texts "in its early days", since the
team keeps giving them. Only the names players read changed; the ids stay
`early_tester` and `early_contributor`, as the hub stores and sends them.

Since 31/10/2026 (Sol's decision) the hub gives Tester by itself, by date, to every
player it sees on SJK who does not hold it yet: rank IV until Sunday 01/11/2026
included, then a week each of rank III (to 08/11), II (to 15/11) and I (to 22/11/2026).
From 23/11/2026 it is only given by hand again, and rank V is no longer earned. The
client needs no change: the medal arrives in the player's profile like any award.

### Medal ranks

Since 27/10/2026 (Sol's decision) every medal a player holds has a rank from I to V,
set by hand by the SJK team, which says how far the player went in what the medal is
for: a Tester at rank V is a top tester. A new medal starts at I. The ranks, each
keeping the lower ranks' look and adding one, in the medal's own colour where it makes
sense ([rank.rs](../crates/sjk-viewer/src/medals/rank.rs)):

| Rank | Name | Look where the medal is drawn large |
| --- | --- | --- |
| I | Issued | The picture as it is. |
| II | Polished | A band of metal light in the medal's colour sweeps across it every few seconds (the Legendary sheen's clock, drawn as the picture's own strips so it keeps to the medal's shape). |
| III | Engraved | The medal leans slightly towards the pointer, as a card does, with a soft glare following it over the medallion and a faint rim of its colour breathing round it; with no pointer on it, it sways gently by itself. |
| IV | Holo | The cards' Holo rainbow foil over the medallion. |
| V | Solar | The cards' Solar corona burning round the medallion's rim, and Mythical's motes rising over it. |

The effects play where a medal is drawn large: the Collection's Medals tab (the rail and
the medal up close, where rank III and up lean towards the pointer), the Identity page,
the profile page's Medals column and the new medal pop-up (which sway instead). They
follow `ui_rarityEffects`: with it off a medal stays still and wears its rank's device
on its ribbon instead. Small places show the device, like a real ribbon's: none for I,
then a bronze (II), silver (III) or gold (IV) pip and a small sun (V), drawn as shapes:
on the scoreboard's ribbon bars, and as a badge at the lower right of a medallion (the
player card, the Players page, the sender card, the classic+ Identity box, the staff
page). Text names a rank "Rank III Engraved": under a medal's name in the Collection,
beside its date ("Rank III Engraved · Given 07/10/2026") on the Identity and profile
pages and in the pop-up; lists of names (the pinned player card, the Players page) leave
it to the device.

The hub sends the rank with each medal, in the profile and the presence entry (`rank`,
1 to 5; missing from older hubs, read as 1; a value out of range is brought into it,
`sjk_identity::Medal::rank`). A rank rising shows the pop-up once more, as a promotion:
"Promoted to Holo" over the medal at its new rank (the card in a match says MEDAL
PROMOTED and "Promoted to Rank IV Holo"); a rank lowered shows nothing.
`medals_seen.txt` keeps the rank shown in a line of its own, `<id>.rank <rank>`, written
above rank I only: a file from before ranks has none and saw rank I, and an older build
keeps those lines as an unknown id's. Staff set a rank from the staff page
([Staff](#staff)); `debug_medal <id> r<rank>` and `debug_medal <id> ranks` rehearse it.

No medal is given again since 21/10/2026, when Bug Hunter stopped counting up: a count
above 1 from the hub reads as 1. The counting (`Medal::repeatable`, the "x2" labels,
Give +1, `debug_medal`'s `x<count>`) stays for a future medal that may be.

The hub lists a key's medals in its profile
(`"medals":[{"id","count","awarded","note","rank"}]`: the count, when it was last given,
a short note from the team, often empty, and the rank) and in its presence entry (`id`,
`count` and `rank` only), so the scoreboard needs no request per player.
Lists come in the table's order; a client shows only the ids it knows and older hubs
send none. The catalogue is [medals.rs](../crates/sjk-viewer/src/medals.rs): each medal's
name, description, ribbon colours and two pictures in `assets/medals` (`<id>.png`, the
whole medal on its ribbon, 512 square; `<id>_small.png`, the medallion alone, 128
square, for anything under about 64 pixels). A new medal is one entry there and two
pictures; new art is a file replacement. Supporter's pictures are Sol's pick of a generated sheet
(a gold-and-steel star with a sun gem on a crimson ribbon, 14/10/2026), its ribbon
lengthened and cut in a V onto the star's bail like the others (redone cleanly 17/10/2026). The JoF Clan medal (`jof_clan`) was dropped on
10/10/2026 for the [JoF clan tag](#clan-tags); a client ignores an old award of it as
an unknown id.

Where they show:

- The scoreboard, every style: up to three small ribbon bars after the SJK emblem, on
  rows whose claim the emblem trusts (the claimed name matches the name the game shows).
  They are coloured rectangles drawn from the catalogue (Tester amber with black
  stripes, Contributor navy with a white centre stripe, Bug Hunter emerald with
  black edges, Supporter violet with gold edges and a gold centre stripe), sized after the emblem; they take room from the name,
  never the columns, and fewer show where the name would keep less than half its room
  (the SJK UI: a third). Derived only when the roster or the rows change.
- The player card (`inspect`): the medallions in a row under the hub name; a pinned card
  names them too in small print, with a repeatable one's count.
- The SJK UI's Players page: the chosen player's card lists their medallions and names.
- The Identity page: the player's own medals, each with its whole picture, name and
  count, description, the date it was given (`dd/mm/yyyy`) and the team's note; with
  none, a line saying the SJK team gives medals for testing, contributing and more.
- The new medal pop-up ([medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs)): the
  first time the client sees a medal in the player's own profile, or a repeatable one's
  count rise, it shows it once, large, with its name, description, date and note, and
  several show one after another ("1 of 3"). It
  opens on the main menu, or when the game menu opens in a match, never over play: a
  medal that arrives during a match is announced once by the achievements' card at the
  top of the screen ([Achievements](#achievements); since 10/10/2026, Sol's request, in
  place of a centre print): "New medal", "Open the game menu to see it", the name (with
  its count), what it is for and the medallion's picture in a gold ring, with the
  achievements' chime. The pop-up takes the card back as it opens. While it shows, the
  menu under it is neither drawn nor given input. What was
  shown is kept in `medals_seen.txt` beside `identity.key` (the key id, then one
  `<id> <count>` a line; ids the build does not know are kept), so each medal and each
  new count shows once per identity, and an install that already held medals when it
  first read its profile still shows them. A medal counts as shown when its button is
  taken.

  Each medal arrives in a ceremony ([award.rs](../crates/sjk-viewer/src/medal_popup/award.rs),
  since 08/10/2026, Sol's request: animated like the achievements, with a sound). Over
  1.8 seconds the whole medal comes down into place on its ribbon, growing from a
  third of its size a little past its own and settling; as it lands (about 0.5 s) light
  bursts from the medallion: a glow, two gold rings running out, a flash, and twenty
  sparks thrown out and falling. A gold arc sweeps round it and stays as a thin ring,
  with a ring of ticks outside it that turns as slowly as the main page's ring; a band
  of light crosses the medal (its own picture drawn again brighter, so the light keeps
  to its shape); and the words fade up one group after another: what it is, the name,
  what it is for and the date, the note, then the button. While it waits it breathes:
  its glow swells and fades over 4.2 s, a softer band of light crosses it every 6 s
  and six sparkles twinkle round it. Enter, Space, Right, Escape or a click (on the
  button or anywhere else) during the entrance finishes it at once; once it stands
  still they take its button, Next (Close on the last): the menus' click plays and the
  medal lifts away and fades in a third of a second before the next comes. Everything
  is draw-list shapes of a fixed number (no allocation a frame); the ceremony waits up
  to 1.5 s for the medal's picture, which is decoded as soon as a medal is queued.
  Each medal's arrival plays the multiplayer game's own fanfare, `music/goodsmall.mp3`
  (its cgame's `happyMusic`, played to the player who becomes the Jedi Master), through
  the interface sounds; without the file (no game data) it is silent. The look follows
  the menus: the SJK UI's ([sjk-ui.md](sjk-ui.md#new-medal)) or the classic+ one
  ([classic-plus.md](classic-plus.md#pages)).
- `debug_medal` ([rehearsal.rs](../crates/sjk-viewer/src/medal_popup/rehearsal.rs))
  rehearses receiving medals offline: `debug_medal <id> [x<count>] [r<rank>] [note]` (a
  count for a repeatable medal only, and none is since 21/10/2026; a rank above 1 arrives
  as a promotion), `debug_medal <id> ranks` for the medal at ranks I to V one after
  another, `debug_medal all [note]` for every medal, `debug_medal` alone for the ids. It makes the list a hub profile would carry (dated today), which is read as the
  hub's is and queued like a real arrival, so the card in a match, the pop-up
  on a menu, the ceremony, the fanfare and Next can all be tried; the console closes so
  it shows at once over a menu. Nothing is sent to the hub, and rehearsed medals are
  never written to `medals_seen.txt`, so a medal the team gives later still shows (a
  real medal arriving replaces its rehearsal still waiting).
- The credits page's cards: not from the hub but from credits.txt's `medal:` lines,
  which use the same ids (Creyon and Lumaya wear Contributor), so a contributor's
  card shows its medals without a request; the SJK UI draws each whole with its name
  and description beside the card's lines, the other looks as chips (medallion and
  name) after the counts ([client.md](client.md#credits-page)).
- Not on nameplates.

Medals are public: anyone can read a key's profile and the presence list of a server,
so a player's medals, counts, dates and notes are visible to everyone, as their hub
name and verified flag are. The client sends nothing about medals.


## Installed models

Since 15/10/2026 the player card (`inspect`, [client.md](client.md#player-card)) of
another SJK player says whether they see the model you wear: "Sees your model", or in
amber "Can't see your model" (their game lacks it and draws its default in its place).

- The client lists the folders under `models/players` holding a `model.glm` or a rig
  model's `sjk_rig.cfg`, in the game data mounted at start, on a thread of its own
  ([installed_models.rs](../crates/sjk-viewer/src/installed_models.rs)), and sends them
  (`PUT /v1/models`, hub `PROTOCOL.md` "Models") once registered and again whenever the
  mounted game data changes, with `cl_shareModels` on; turning it off sends an empty
  list, which deletes the hub's copy. A server's downloads are not counted.
- The model you wear is the one the server publishes for your slot (`model` of your
  `CS_PLAYERS` string), else the `model` setting; its folder is asked about. A retail
  model (in the retail archives) is never asked about: every player has it.
- The card asks the hub (`GET /v1/models?server=..&slot=..&model=..`, unsigned) for the
  inspected slot and your model when it is built, and again at most every minute for the
  same slot and model. The answer counts only while the slot's claimed name matches the
  name the game shows, as for badges. No line when the hub has no list of theirs (an
  older client, `cl_shareModels` off), when you wear a retail model, or on your own card.
- The hub keeps lists per key, so a person playing on two PCs answers with the list the
  latest one sent.
- What a list says is the client's word: a modified client can send any list.

## Clan tags

A player whose name carries a known clan's tag gets the clan's emblem on the left of their
name. It is read from the name alone, so it shows what a player says, not something the
hub vouches for; it needs no hub and no SJK identity.
[jof_tag.rs](../crates/sjk-viewer/src/jof_tag.rs) holds the clans, the rule, the size and
the drawing. `cg_clanEmblems` (archived, default 1, Settings > HUD > Scoreboard > Clan emblems)
0 turns every emblem off.

The clans are the entries of `jof_tag::KNOWN`, each with its tag, its emblem, its tint,
the share of the emblem's square its ink spans and its rule (how its tag is read),
with the reason and who asked; supporting a clan means adding an entry and its emblem
(an icon atlas cell each, room for four; all four are used, so a fifth clan needs
`CLAN_ICON_CELLS` raised). Longer tags come first, so `JoFA` is read before `JoF`.

| Clan | Tag | Emblem | Rule |
| --- | --- | --- | --- |
| JAWA (Sol's request, 19/10/2026) | `(JAWA)` | `jawa-emblem.png`, the clan's forum icon (a Jawa's hood) in its own colours | exact, at the start (Sol, 22/10/2026; letters before): `(JAWA)Ken`, not `JAWAKen`, `[Jawa] Ken` or `Ken(JAWA)` |
| {{S}} (Sol's request, 22/10/2026) | `{{S}}` | `s-emblem.png`, the clan's red Sith emblem in its own colours | exact, at the start: `{{S}}Ken`, not `{{s}}Ken`, `{S}Ken` or `Ken{{S}}` |
| JoF Academy (Sol's request, 11/10/2026) | `JoFA` | `jofa-emblem.png`, the academy's HD logo in its own colours | letters; stuck to a name with the tag's capitals and a capitalised word after it (`JoFAKen`) |
| JoF, Jedi of Freedom | `JoF` | `jof-emblem.png`, the clan's green J o F logo in its own colours (Sol, 22/10/2026; a teal `#02B99D` J o F before, crimson until 11/10/2026) | letters; stuck to a name in any case, unless a second f follows (`JoFSanjiJ`, `jofsanji`; not `Joffrey`) |

The exact rule: the name, colour codes dropped, starts with the tag exactly as written,
case and signs included (`^3(JAWA)^7Ken` counts).

The letters rule: the tag's letters in any case, with no letter right before the
first (the name starts there or a separator comes first: a space, a bracket, a dot, a
digit...). Colour codes are dropped first. `{JoF}Name`, `jof.Name`, `[JOF] Name`,
`Name-JoF`, `^1J^7oF`, `JoFA}Ken`, `^5JoFA^7<3Toad` and `[jofa] Ken` count; `MrJoF` does
not, and `JoFAx` reads as JoF. Any Unicode letter counts as a letter. Each tag letter may
be written with a look-alike, as players do (Creyon's request, 11/10/2026): `jøf'p Evi`,
`J0F`, `Jöƒ`, `{J°F}` and `jøfå` count. The look-alikes are accented and stroked forms,
Greek, Cyrillic and full-width letters, `0`, `°` and `º` for the o and `Λ`, `Δ` for the A
(`looks_like`); a clan whose tag has other letters may need theirs added.

Where it shows: the game chat's lines with a sender (JA+ lines too, since 16/10/2026), SJK chat lines in the feed, the SJK UI's chat
dock and chat page, the scoreboard (classic and SJK, list rows and duel cards), the player
card (`inspect`) and the SJK UI Players page's card. Not on nameplates.

The emblems are 128-pixel squares in `assets/branding` ([README](../assets/branding/README.md)),
each shrunk (or enlarged) from the clan's logo by `scripts/clan_emblem.py` (cropped to its
ink, centred and scaled with Lanczos on premultiplied alpha).

## Unlocks and looks

Unlocks are cosmetic things a key owns, granted by the hub's operator or staff
(design, catalogue and status: [unlockables.md](unlockables.md)); the first is the Sun
blade. A profile lists them (`"unlocks":[{"id","granted","note"}]`, the catalogue's
order; older hubs send none, `Profile::unlocks`). Staff grant and take one back with
`StaffRequest::Unlock` and `StaffRequest::Relock` (`/v1/staff/unlock`,
`/v1/staff/relock`); the answer is the target's profile, which replaces the player's
own when it is theirs, as for medals.

A *look* is what a player wears that others draw: `{"saber":"..","illuminate":..}`, a
blade-skin unlock id or `""`, and whether the Illuminate holocron is lit.

- Sending: the viewer computes the own look twice a second from `cg_saberSkin`, kept
  only while the own profile lists that unlock and the client knows the id, and the
  local Illuminate, and hands it to `Service::set_look` when it changes. The worker
  keeps the latest and sends `POST /v1/look` once its claim is accepted, again when
  the look changes or the claim does (another server, slot or name), at most once a
  second (changes in between are coalesced, the latest wins). When what the hub holds
  is unknown (at the start, after a failed claim or a failed release, when an older
  claim may still be live with a look) the next accepted claim gets the look even if
  it is none. A `not_unlocked` or `bad_look` answer leaves that skin out (the look
  goes with `saber:""`, so Illuminate still syncs) until the profile's unlocks change,
  and a `not_unlocked` has the own profile read again soon (at most once every 30
  seconds); the key's `look_quota` (429) waits 1 second, the address's `rate_limited`
  (429), a refused signature (401, the clock still off after the one retry) and
  failures wait 10 seconds and go again; `not_on_server` asks for the claim again at
  once; another refusal (an older hub) is not repeated until the look or the claim
  changes. The second between two looks counts from the earlier one's answer, so they
  reach the hub in different whole seconds (the hub's quota counts whole seconds).
  `Service::set_look` leaves the look in a slot the worker reads between its other hub
  calls (`Worker::look_between`), so a slow call does not hold a toy back for a whole
  round of the worker's calls (the cardboard box's slow sync, 31/10/2026); the viewer
  logs each outcome as `SJK look sent: <n> ms after the change, the hub answered in
  <m> ms`. Leaving sends nothing: the release drops the look. `Snapshot::look_outcome` says what became of the last one.
- Receiving: presence entries carry `look` (`Presence::look`) and the feed carries look
  events (`Feed::looks`, `LookEvent`), queued for the viewer (`Service::take_looks`, the
  newest 64, handing over only those read for the viewer's server under the feed's
  current generation, `ReceivedLooks`; a new generation clears the viewer's hub looks). The viewer's `looks.rs` keeps one look per slot: the roster's when it
  changes, the feed's as they come (newer, so they win until the roster changes
  again), counted only while the game shows the claimed name in that slot, and cleared
  on another server. The local player's own look comes from its settings.
- Others' Illuminate shows by their left shoulder ([client.md](client.md#illuminate));
  blade skins are drawn by the saber renderer from `Looks::saber_skin_id`, with the
  parts their wearer switched off (`Look::saber_off`, sent only when it names one, as
  an older hub refuses the field; `Looks::options_off`,
  [unlockables.md](unlockables.md#options)) and the extras they switched on
  (`Look::saber_on` and `saber2_on`, since 26/10/2026, which the hub relays only while
  the wearer holds the card that unlocks them; [unlockables.md](unlockables.md#extras)).
- The player's toon shading choice for their own model (`cg_toonModel`, since
  08/11/2026) goes in the look as toy `toon_model`, so the hub needs nothing new: every
  SJK player who shows toon models (`r_toonModels`) draws that player's model in toon
  shading ([Toon models](rendering.md#toon-models)).

Looks are public to every SJK player on the server, as badges are.

## Map edits

Staff publish a map's edits to the hub, and the worker keeps every SJK player's copies
in `map_edits` beside the key: the hub's list every 10 minutes once registered (at once
on `Service::refresh_map_edits`, which the editor sends when it turns on, and after a
publish), each document newer than its copy downloaded, a copy the hub stopped listing
removed, and a copy with unpublished changes left alone. What the last look did lands in
`Snapshot::map_edits_note` (a summary, or why it fell short), the publish's answer in
`Snapshot::map_edits_published`, both worded by `map_edits::refusal` when the hub said
no. What the edits do is in [map-edits.md](map-edits.md).

## Mystery boxes

The worker reads the hub's mystery boxes once registered and every 10 minutes
(`Snapshot::mystery_boxes`), and sends openings and the creator's placings and removals
(`Service::open_mystery_box`, `place_mystery_box`, `remove_mystery_box`), whose answer
lands in `Snapshot::mystery_box` with the profile (credits) after. What a box does is in
[mystery-boxes.md](mystery-boxes.md).

## Asset packs

The art of the unlockable cosmetics (each blade skin's look and sounds) is not part of
SJK's code: the hub serves it as *asset packs*, PK3 files (`PROTOCOL.md`, "Assets"), so
every SJK client can draw every player's look. The identity worker keeps a copy of
each in `assets/` beside `identity.key` (`assets::FOLDER`), one `<name>.pk3` per pack,
once the viewer gives it that folder (`Service::keep_assets`); the viewer mounts the
packs it finds there (`assets::cached_packs`).

- `GET /v1/assets`, signed: the list, `{"packs":[{"name","size","sha256"}]}`, sorted by
  name. A pack name is 1 to 32 of `a` to `z`, `0` to `9` and `_`, a size at most 16 MiB
  and a hash 64 lowercase hex digits; a list breaking any of that is refused whole.
- `GET /v1/assets/<name>`, signed: the pack's bytes, read with a timeout of two minutes
  and no more than 16 MiB of them.

The worker reads the list only with the identity on, a hub and the key registered: at
once after the registration (so at start, and again after the identity is switched off
and on or the hub changes), then every 6 hours. It downloads a listed pack only when the
folder has no `<name>.pk3` of the listed size and SHA-256. A download must be the
listed size and hash, or it is refused and nothing is written; a good one goes to a
temporary file in the same folder (`.<name>.pk3.part`), flushed to disk, then renamed
over `<name>.pk3`, so the folder never holds a partial pack under a pack's name and an
error leaves the old one as it was. Packs the hub no longer lists stay in the folder.
`Snapshot::packs_revision` counts the packs written, so the viewer knows to mount them
again, and `Snapshot::assets_note` says what the last check did, for the log.

A check that falls short (the hub out of reach, a 5xx, too many downloads (429), a
refused signature (401), a pack unlike its listing, the disk) is tried again after a
minute, then two, doubling up to 6 hours; one that succeeds puts the next 6 hours on.
Another 4xx (an older hub without assets answers 404) waits the full 6 hours. Packs are
cosmetic: none of this changes the identity's status. Offline, or with the identity
off, nothing is downloaded and the packs already in the folder are used as they are.

The hub sees which packs a key downloads and when, and the address the requests come
from: they are signed by the key. It allows 20 downloads an hour per address, and the list counts
toward its general 30 requests a minute. Packs are SJK's, all rights reserved, served
only to registered keys; a client keeps them to draw looks and does not pass them on.

## Profile

The Profile page (the SJK UI's [Profile screen](sjk-ui.md#profile-screen), its SJK
Profile tab, opened by the main page's Profile and profile card and the in-game menu's
Profile; the classic menu's SJK page and in-game SJK menu; or the `profile` command) is the player's SJK profile as other players read it
on the hub, in the SJK UI's look in every menu style:

- who they are: the hub name with its colours, Verified by the SJK team or not yet,
  the date the key was registered (`Member since`) and up to three other
  names worn (the switch, the key and the hub are on the Identity page, in Settings,
  Network, since 09/10/2026); on the Profile screen Change name opens the Character
  tab's Name field, since the hub name is the `name` worn in game;
- their record, from the achievement counts kept on this PC (players defeated, saber
  kills, best streak, duels won, flags captured, maps and servers played, time
  played) and the last four achievements unlocked;
- the bio, written in a box of up to 6 lines (Enter saves, Shift+Enter starts a new
  line, Revert puts back the hub's copy), with its counts, the hub's answer and the
  rules in a line under it. The box has a caret, the report dialog's
  ([text_dialog_field.rs](../crates/sjk-viewer/src/text_dialog_field.rs), since
  23/10/2026 after Kushina's report; before, only the end could be written): a click
  places it, Left and Right (a word with Ctrl), Up and Down, Home and End (the
  text's ends with Ctrl) move it, typing, pasting, Backspace and Delete (a word with
  Ctrl) work at it, Ctrl+A selects all (typing replaces it) and Ctrl+C copies the
  selection or the whole bio. Its lines wrap and the caret measures in the player's
  menu text style;
- their medals (picture, name, date given; on its own only) and how many
  achievements are unlocked, with See the board, and how many saber shaders are owned,
  with See the collection: both open the [Collection](sjk-ui.md#collection), where the
  medals hang on a rail (every medal the client knows, the ones given first with their
  whole picture, when they were given, what they are for and the team's note, the
  others dark as not given yet) and the achievements stand as a wall of medallions by
  category (below; the `achievements` command opens it). With the identity off, or the hub out of reach, the page says so: the bio cannot
be written and the record and board show this PC's counts.

The player's picture stands at the top of the left column ([Pictures](#pictures)): a
click on it (or Enter on it) opens the picture panel in the bio's place.

### Name emoji

Since 16/10/2026 (Sol's request) a player may pick one emoji that everyone sees at the
right of their name, before the verified tick or trust mark: on SJK and game chat lines,
the main page's dock, the SJK chat page and its people, sender cards, the profile card
and the SJK scoreboard (not the nameplates, the classic scoreboard or other clients).
The Profile page's Name emoji button (Add a name emoji while none is set) opens the
emoji panel in the bio's place: a grid of 60 emoji (`EMOJI_CHOICES` in
`profile_panel.rs`), where a click, or Enter on the cell the arrows move to, sends it at
once; any other single emoji typed or pasted there (Windows' emoji panel, Win+.) is sent
the same way; Remove emoji takes it away. The hub keeps it with the profile and sends
it with profiles, presence, chat messages (the sender's emoji when they sent it) and the
feed's people (`emoji`, PROTOCOL.md "Profile fields").

Its rule (`sjk_identity::name_emoji`, the hub's copy word for word): exactly one Twemoji
picture, so one pictograph with the marks that change it (a skin tone, U+FE0F, a
subdivision flag's tags) or the pictographs U+200D joins to it, or two regional
indicators (a flag); no keycaps, no rude gesture (`REFUSED`), at most 16 characters.
The client checks it before sending and shows a hub's value only through the same rule.
The hub's owner can set or clear a player's emoji (admin API and panel). The profile
card in the bottom-left corner of the SJK UI's main page and in-game menu
([sjk-ui.md](sjk-ui.md#profile-card)) opens the page too.

### The bio's rules

A bio is free text every player can read, so it keeps to what SJK's fonts draw and
to nothing that hides, reorders or piles characters up. Before it is checked, line
ends become `\n`, each line's runs of spaces and tabs become one space and its ends
are trimmed, and runs of blank lines become one. Then it must hold at most 500
characters, at most 6 lines, no run of one character longer than 8, and only Latin
letters (with their accented and extended forms, Vietnamese included), Cyrillic
ones, Japanese ones (hiragana, katakana, kanji and `々`; the kanji are the Chinese
characters too) and Korean ones (Hangul syllables and the compatibility jamo, as in
`ㅋㅋ`), ASCII digits, the space, newlines, ``. , ! ? ' " - : ; ( ) [ ] & / + # @ % * _
= ~ < > | $``, Japanese and Chinese punctuation (`、。「」『』・！？（）～，：；《》“”‘’`),
emoji (the code points Twemoji's pictures are made of, generated into
[emoji.rs](../crates/sjk-identity/src/emoji.rs), with their joiners, selectors, skin
tones and tags only inside an emoji) and colour codes (`^` and a digit); Japanese
since 12/10/2026, Korean, Chinese punctuation and emoji since 13/10/2026. That refuses other symbols, combining
marks (no piled-up "Zalgo" text), zero-width and direction-changing characters, other
spaces, private-use and unassigned code points and control characters.

[bio.rs](../crates/sjk-identity/src/bio.rs) holds the rules, and the hub's copy is the
same code. They are applied three times: the bio box (and the Identity page's field)
cannot type what a bio cannot hold; the client checks the whole bio before sending it
and says which rule it breaks; the hub refuses a bio that breaks one (`bio_length`,
`bio_characters`, `bio_lines`, `bio_noise`). And whatever a hub sends back, the
client shows a bio only through `bio::for_display`, which drops what the rules
refuse, so a bio stored before the rules, or a foreign hub, cannot put anything else
on screen. A bio is plain text: nothing in it is ever a link or markup.

## Pictures

A player may show a small square picture beside their name (08/10/2026, Sol's
request). The hub's side is `PROTOCOL.md`, "Pictures", in Sol-Vulpes/SJK-hub: the
client needs a hub with it, or profiles carry no `avatar` and the stand-ins show.

**Choosing one.** The picture panel's **Browse...** (09/10/2026, Sol's request) opens
the system's file dialog, listing PNG, JPEG and TGA files (`file_dialog.rs`: the `rfd`
crate, 0.17.2; on Linux `zenity` or `kdialog` when installed, else the desktop portal,
reached through `libdbus` loaded at run time, so the build needs no GTK). The dialog
runs on a worker thread, so the game keeps drawing; while it is open the button reads
Stop waiting and a second press gives up on it, and the file chosen is read as a
dropped one. A picture file dropped on the window (a PNG, JPEG or TGA, known by its
extension, at any time; while the Profile page shows, any file but a `.cfg`, so the
page can say why it is no picture) or named to `sjkavatar <file>` opens the Profile
page (in the SJK UI the Profile screen's SJK Profile tab) on its picture panel. A worker thread reads the file (at most 16 MB) and decodes
it with the `image` crate the client already uses (at most 8192 pixels a side and 256
MB of memory), crops it to a square from its middle, scales it to 128 x 128 by area
averaging and writes it as a PNG of the pixels only (RGB when every pixel is opaque,
else RGBA).

The file is checked before anything is sent, whatever its name says
([avatars/picture.rs](../crates/sjk-viewer/src/avatars/picture.rs), tightened
09/10/2026): it must hold bytes and be a file; its content must be a PNG or JPEG (by
its signature) or, as a TGA has none, a file named `.tga` that decodes as one, so a
program, a GIF, a WebP or a BMP is refused whatever it is called; it must decode
whole (a PNG cut short or damaged fails its checksums, a JPEG must reach its end
marker after its first scan, as the decoder fills in a file cut short); it must be
64 to 8192 pixels a side (64, the hub's own least: a smaller picture made 128 across
is a blur) and at most four times as long as it is wide; and its middle square must
show something (not every pixel clear). The hub checks again: square PNGs of 64 to
1024 pixels and 256 KB, re-encoded.
The panel shows it large, round and ringed as everyone will see it, "This is how it
will look"; **Use this picture** sends it. Done, Escape or closing the page drops a
picture not sent. **Remove picture** (a second press within 3 seconds) or
`sjkavatar clear` takes the player's picture down. The panel says what went wrong in
the player's words: not a picture SJK reads ("use a PNG, JPEG or TGA"), an empty file,
too big, too small ("at least 64 pixels a side"), too narrow or too wide, nothing to
see, damaged or cut short, unreadable, the identity off or the hub out of reach, or
the hub's own refusal (too many changes, stopped by the SJK team).

**Sending.** The identity service's worker sends it, signed, once registered:
`PUT /v1/avatar` with the PNG (at most 256 KiB, refused before sending when larger),
`DELETE /v1/avatar` to take it down (`Service::set_avatar`, `Service::remove_avatar`;
the outcome in `Snapshot::avatar`). The hub answers with the profile, whose `avatar`
is the new version; the preview's pixels become that version's picture in the cache,
so it shows at once, everywhere, without a download. The hub takes a square PNG of 64
to 1024 pixels, re-encodes it at 128, and allows 6 changes an hour and 20 a day per
key.

**Showing.** Profiles and presence carry `avatar`, the picture's version (16 hex
digits of the SHA-256 of the hub's PNG), `""` for none. Pictures show on the profile
card, the Profile page, the in-game Players page's card of a player the hub knows on
the server, the Staff page's chosen player, and since 28/10/2026 the nameplates over
the players of a match ([client.md](client.md#nameplates)). A screen asks the cache
([avatars.rs](../crates/sjk-viewer/src/avatars.rs)) for a key's picture at a version
each frame it draws it; that compares a few strings and allocates nothing once asked.
The first ask hands the request to one worker thread (`sjk-avatars`), which reads the
picture kept on this PC or downloads `GET /v1/avatar/<key id>?v=<version>` (unsigned,
at most 256 KiB, a PNG), checks it is square, decodes it and cuts it round. Once a
frame, `avatars::service` takes what the worker finished and uploads at most four
pictures into the UI's icon atlas (three rows of 128-pixel cells: 47 pictures and the
preview). The cache holds 47 (31 until 28/10/2026, when the nameplates came to show
pictures: a full server's 31 other players and the player's own plate may be on screen
at once, which left the chat, the cards and the boards nothing; a fourth row would bring
the atlas to 8192 pixels, the largest texture the client asks for, so three); a new
picture takes the place of the one shown longest ago, never one drawn in the last two
frames. A screen that draws many a frame (the nameplates) asks for them all under one
lock (`avatars::textures`). Pictures made on this PC share the cells: the nameplates'
head icons of players without SJK, cut round, kept under a key from the model's name
and never asked of the hub (`avatars::insert_local`). A new version is a new picture. A picture
that could not be had is asked for again after two minutes, or at once when the hub's
address changes. Until a picture is there, and for a player without one, a stand-in
shows: the first letter of the name (past colour codes and symbols) on one of eight
colours picked from the key id, so a player has the same colour on every PC. A
verified player's picture is ringed in gold with their tick ([Ticks](#ticks)) at its foot.

**Kept on this PC.** Downloaded pictures are kept as the hub served them in
`avatars/<key id>-<version>.png` beside `identity.key`
([avatars/store.rs](../crates/sjk-viewer/src/avatars/store.rs)), so a picture is
downloaded once per version. The folder keeps at most 256 pictures and 8 MB: past
either, those used longest ago go (reading one counts as a use), and a key's older
versions go when a new one is kept. Only names of that exact form are read, written
or removed, so no text from the hub becomes a path. With the identity off nothing is
downloaded.

**Moderation.** Pictures are public and moderated by hand. A staff key's Staff page
has Take picture down for the chosen player (`/v1/staff/avatar-remove`, in the staff
log); stopping a key from uploading (`/v1/staff/avatar-block`,
`StaffRequest::AvatarBlock`) and the operator's commands are the hub's
(`ADMIN.md`). A picture taken down is gone: the key's `avatar` becomes `""` and
clients show the stand-in.

## Achievements

An achievement is a milestone of the player's own play. Since 21/10/2026 each brings a
kyber crystal once, when it unlocks ([holocrons.md](holocrons.md#kyber-crystals)): Uncommon
for most, Rare for the harder ones, Legendary for Duel Master, Veteran, Keeper of the
Holocrons and Full Set, Mythical for Legend of the Arena. The Collection's Achievements tab says it under the achievement up
close ("Reward: Rare Kyber Crystal" after the crystal's gem, in its colour), its hover card
ends with it, and the unlock card says it at the right of "ACHIEVEMENT UNLOCKED" in place
of the category when the hub unlocked it (not with the identity off, which brings no
crystal; see below). The hub's `reward` on each achievement wins over the client's own table
(`kyber::REWARDS`) once it sends one. The catalogue, in [achievements.rs](../crates/sjk-viewer/src/achievements.rs)
and in the hub (same ids):

| Achievement | Id | Goal | Counted by |
| --- | --- | --- | --- |
| First Blood, Centurion, Legend of the Arena | `first_blood`, `kills_100`, `kills_1000` | 1, 100, 1000 players defeated | client |
| Blademaster | `saber_kills_100` | 100 players defeated with a saber | client |
| Rampage, Unstoppable | `streak_5`, `streak_10` | 5, 10 players defeated in one life | client |
| Unlimited Power | `dark_side_25` | 25 players defeated by Force lightning or drain | client |
| Watch Your Step | `ledge_10` | 10 players finished by a fall, a pit or a hazard after the player hit or pushed them | client |
| Arsenal | `arsenal` | players defeated with 8 kinds of weapon | client |
| Duelist, Duel Master | `duel_wins_10`, `duel_wins_100` | 10, 100 duels won | client |
| Flag Runner | `captures_10` | 10 flags captured | client |
| Traveller, Galaxy Tour | `maps_10`, `maps_25` | 10, 25 different maps played | client |
| Server Hopper | `servers_5` | 5 different servers played on | client |
| Regular, Veteran | `hours_10`, `hours_100` | 10, 100 hours played on servers | client |
| Storyteller | `storyteller` | a bio | hub |
| Bug Reporter | `bug_reporter` | a bug report the hub took | hub |
| Surveyor (retired 26/10/2026) | `surveyor` | 5 world notes the hub took | hub |
| Decorated | `decorated` | a medal | hub |
| Collector, Archivist, Keeper of the Holocrons | `holocrons_10`, `holocrons_50`, `holocrons_200` | 10, 50, 200 holocrons opened (kyber crystals not counted) | hub |
| Touched by the Force (hidden) | `mythical_find` | a Mythical shader or card from a holocron | hub |
| Craftsman | `craft_blade` | a blade crafted from a card | hub |
| Master Craftsman | `craft_finishes` | a blade crafted from a card of every finish (3: Normal, Holo, Solar, `cards::FINISHES`); three crafts from cards of one finish count 1 | hub |
| Full Set | `full_set` | every Common and Uncommon card of any one set, in any finish, revealed copies only (sealed ones do not count) | hub |
| Executor | `order66_kill` | an Order 66 execute on another player who also has the toy | hub |
| Tactical Espionage | `box_minutes_10` | 10 minutes hidden in the cardboard box with another player close by | client |
| Flawless | `duel_flawless` | a duel won without taking damage | client |
| Close Call | `duel_close_call` | a duel won with less than 5 health left | client |
| Well Met | `duel_partners_25` | 25 different players duelled | client |
| Long Throw | `saber_throw_far` | a player defeated by a saber throw from 512 units or more | client |
| Return to Sender | `thermal_return` | a player defeated with their own thermal detonator | client |
| Payback | `monitor_payback` | during the player's Screen Saver, the player whose monitor raised it struck with a monitor | client |
| Power Surge (hidden) | `monitor_surge` | 3 different players struck with monitors during one Screen Saver | client |
| Blue, Yellow, Red, Dual, Staff Master | `style_blue_50`, `style_yellow_50`, `style_red_50`, `style_dual_50`, `style_staff_50` | 50 duels won with the saber in that style (`fd.saberAnimLevel` when the kill is made: 1 Fast, 2 Medium, 3 Strong, 6 two sabers, 7 staff; Desann's and Tavion's count towards none), in a private duel (saber only or full Force) or a duel game; kills out of a duel do not count | client |
| Sneaking Mission, Legendary Soldier | `box_minutes_30`, `box_minutes_60` | 30, 60 minutes hidden in the cardboard box with another player close by (Tactical Espionage's count) | client |
| Surprise! | `box_ambush` | the player who lifted the player's box defeated by the player within 3 seconds | client |
| Alert! | `box_spotted_10` | the boxes of 10 different players lifted | client |
| Wipe Them Out | `order66_purge` | 5 holders of the toy brought down by one Order 66 of the player's | client |
| I Am the Senate | `order66_victims_25` | 25 different players brought down by the player's Order 66s | client |
| Puppet Master, Double Agent, Total Control (hidden) | `hax_puppet`, `hax_double`, `hax_triple` | 1, 2, 3 players defeated by a player the Hax steers, in one hack (the hacker not counted) | client |
| Leap of Faith | `hax_fall` | a player the Hax steers dead by a fall, a pit or a hazard (by their own hand or the world's) | client |
| Script Kiddie, Elite Hacker | `hax_minutes_5`, `hax_minutes_15` | 5, 15 minutes steering players with the Hax | client |
| It's a Trap! | `hax_traps_5` | 5 hackers caught in the player's Hax trap | client |
| Eye in the Sky | `drone_strike` | the player the Drone watched defeated by the player within 10 seconds of it landing (the peek ended) | client |
| Special Delivery | `drone_monitor` | the player the Drone watched struck with the player's monitor within 5 seconds of it landing | client |
| DJ | `boombox_minutes_30` | 30 minutes of the player's boombox playing with an SJK player within its range | client |
| Block Party | `boombox_crowd` | 3 SJK players within the boombox's range at once | client |
| Tough Crowd (hidden) | `boombox_tough_crowd` | a player within the boombox's range dead by their own hand or the world's while it plays | client |
| Tech Support | `monitor_victims_25` | 25 different players struck with the player's monitors | client |
| Two for One, Strike! (hidden) | `monitor_double`, `monitor_triple` | 2, 3 players struck with one monitor | client |
| Long Distance Call | `monitor_far` | a player struck with a monitor 1575 units (40 metres, as the world ping measures) or more from where it was thrown, measured where it met the body (`achievements::MONITOR_FAR`) | client |
| Bounty Hunter, Big Game Hunter | `box_hunt_mark`, `box_hunt_marks_10` | 1, 10 box hunt marks killed (a free roll won or not) | hub |
| Still Standing, Most Wanted | `box_hunt_survive`, `box_hunt_survive_5` | 1, 5 marks stayed to the end without a hunter's kill, still in the hunt | hub |
| First Bite, Angler | `fish_first`, `fish_50` | 1, 50 fish landed with the Fishing toy ([client.md](client.md#fishing)) | client |
| Every Fish in the Pond | `fish_kinds` | every kind of fish landed (12, `fishing::FISH_KINDS`) | client |
| Echo Angler | `echo_angler` | 10 Echoes landed on the line | client |
| Old Whiskers (hidden) | `old_whiskers` | Old Whiskers, the great catfish, landed | client |


The fourteen after Decorated came on 26/10/2026 (Sol's list), with the categories
Collection and Toys; the five style masters on 29/10/2026 (Dante's idea in #ideas,
duels only at Slash's call); the toys' own from Sneaking Mission on, on 30/10/2026 (a
player's ideas, reworked with Sol), when Toys became a category for each toy on the
board's Fun page; the five Fishing ones on 29/10/2026 (Slash), in a Fishing category of
their own on the Fun page, after Box hunt. Each needs the same id in the hub's catalogue,
and the hub bounds how fast a fishing count may rise as it does the others (a bite at most
every 20 seconds). A hidden achievement (Touched by the Force) is a secret until it
is unlocked: the board, its hover card and the "Next up" list show "Secret" or "Secret
achievement", a `?` in its medallion, no goal and no count; unlocked, it shows like any
other (the unlock card names it). A retired achievement (Surveyor, since 26/10/2026)
can no longer be earned: it shows only to a player who holds it unlocked, with "No
longer obtainable", and is not listed at all, nor its progress, for anyone else. Hidden
and retired are flags of the client's catalogue (`Kind::hidden`, `Kind::retired`); the
hub's `"retired": true` on Surveyor's entry is not needed by the client.
Totals ("12 of 39 unlocked", the player card's "Achievements 12 / 39", the profile
card) count every obtainable achievement and the retired ones the player holds, so a
holder of Surveyor reads "of 40".

The Toy Master's achievements (since 30/10/2026, [toy-master.md](toy-master.md)) are not
in the client's catalogue: the hub sends one only once it is unlocked, with its `name`,
`description` and `group` (`toy_master`), fields no other achievement carries. The board
shows them in a "Toy Master" category, and totals count them like retired ones held:
in both numbers, only for whoever holds them.

What the client counts comes from its live matches on servers, never a demo, a game
it hosts or a server on the player's own machine (a loopback address: Create game and
`devmap` run `sjk-server` there, on a new port each time, #165)
([tracker.rs](../crates/sjk-viewer/src/achievements/tracker.rs), fed each
live snapshot by `achievements_frame.rs`):

- a kill is an obituary (`EV_OBITUARY`) whose attacker is the player's own client
  number (`GameState::client_num`, so watching someone else counts nothing) and whose
  victim is another player, not a teammate in a team game; its means of death says a
  saber, Force lightning or drain (`MOD_FORCE_DARK`), a fall or hazard (water, slime,
  lava, crush, falling, `trigger_hurt`: the game credits the player who pushed or hit
  the victim last), and the weapon kind for Arsenal (15 kinds);
- a streak counts kills since the player's own last death (or suicide), and the best
  one is kept;
- a duel is won by a kill of the opponent of the private duel the player state shows
  (`duelInProgress`, `duelIndex`, read the snapshot before too, as the duel ends with
  the kill), or any kill in a Duel or Power Duel game;
- captures are the player's own `persistant[PERS_CAPTURES]` rising (a rise of at most
  3 at once, with the same team; a team, map, server or slot change starts afresh);
- a map and a server count once the player is in the game there (not spectating, not
  following, not in the intermission), and time played is the time between such
  snapshots, at most a second at once;
- time in the cardboard box counts like time played, while the player hides in it (on,
  alive, playing: `CardboardBox::hiding`) and another player the game draws alive
  stands within 1024 units (`tracker::BOX_NEAR`, about 28 metres; 512 until 26/10/2026)
  in the snapshot;
- what the player took in a private duel is their own health and shield in the
  snapshots while the duel is on: either falling is damage (not during the duel's
  countdown, which may set them, nor health or shield above the maximum health wearing
  down); the last health seen in the duel is the one it was won with, as the game gives
  the winner full health when it ends. A win is Flawless when nothing fell and a Close
  Call under 5 health (`tracker::CLOSE_CALL_HEALTH`). In a Duel or Power Duel game the
  same is followed from the player's last spawn;
- a duel partner is the opponent of a private duel that ends with a death (won or lost:
  the obituary of either), or the other player of any kill or death in a Duel or Power
  Duel game, kept by the name the server shows without colours or punctuation
  (`sjk_identity::normal_form`), so a player who changes name counts again and two who
  share one count once;
- a long saber throw is a saber kill (the game gives a thrown saber's kills
  `MOD_SABER`, as a swing's) while the player's saber is in flight (`saberInFlight`, in
  that snapshot or the one before) with the victim 512 units or more away
  (`tracker::SABER_THROW_FAR`; a swing reaches under 100, a throw up to 700 per Saber
  Throw level);
- a thermal detonator sent back is a thermal kill (`MOD_THERMAL`, `MOD_THERMAL_SPLASH`)
  credited to the player while a thermal detonator the victim threw went off (left the
  snapshot) within the last second and none of the player's own did. The game credits
  a pushed or deflected missile to whoever sent it back (`G_ReflectMissile` sets
  `r.ownerNum`) but never sends a missile's owner to clients (`entityState_t::owner`
  stays 0 for missiles), so the client names a thermal detonator's thrower itself: the
  player nearest (within 96 units) to where its flight started when it is first seen,
  provided that start is at most 250 ms old. A thermal detonator first seen later (it
  came into view after leaving the hand, or after a bounce) has no thrower and counts
  nothing;
- Payback and Power Surge (29/10/2026, Sol's Screen Saver for the Monitor,
  [client.md](client.md#monitor)) are counted by the toy, not the snapshots' tracker
  (`achievements::note`), from the hits the struck players' own clients tell (their
  `hit_` event, never a hit only foreseen) on a server not on this machine: during the
  player's Screen Saver, a hit on the player whose monitor raised it counts Payback,
  and the third different player hit counts Power Surge, each once a Screen Saver;
- the toys' own (30/10/2026). The tracker weighs each obituary against what the toys
  tell it of the moment (`tracker::Toys`, `GpuState::toy_moments`): who lifted the
  player's box and when that reached this client, the player the Hax steers, the player
  the Drone last watched and when it landed (the peek ended, or moved to another player),
  and the SJK players within the range of the player's boombox while it plays. A kill of
  the lifter within 3 seconds (`tracker::BOX_AMBUSH`) counts Surprise!, once a lift; of
  the player the Drone watched within 10 seconds of its landing (`tracker::DRONE_STRIKE`),
  Eye in the Sky, once a landing. A kill by the player steered of anyone but themselves
  and the hacker counts towards the best of one hack (Puppet Master to Total Control);
  their death by a fall, a pit or a hazard by their own hand or the world's, Leap of
  Faith. Minutes steering and minutes with a listener in reach count as time played
  does, the most listeners at once is kept (Block Party), and a listener dead by their
  own hand or the world's counts Tough Crowd. The rest are counted by the toys
  (`achievements::note`, `note_best`, `note_name`): a box lifted (Alert!, by the name
  shown), a trap sprung on a hacker (It's a Trap!), the holders blown up in the window of
  the player's own Order 66 (Wipe Them Out, the most of one order, and I Am the Senate,
  by name), and the player's monitor's hits as the struck players' clients tell them
  (Tech Support by name, Two for One and Strike! by the players one monitor struck, Long
  Distance Call by how far it flew from its throw, and Special Delivery, a hit on the
  player the Drone watched within 5 seconds of its landing). The sets of players are
  kept by the name the server shows without colours (`sjk_identity::normal_form`), at
  most 512 each, in `achievements.json`'s `named`, and the toys' minutes' spare
  milliseconds in its `spare_ms`.

The counts are kept in `achievements.json` beside `identity.key` (written at most
every 20 seconds while they change, through a temporary file, and on the way out),
so they work with the identity off. With it on, the identity service sends the
counts of the achievements the client counts to the hub (`PUT /v1/achievements`, a
signed request, at most once a minute while they change, and unchanged again an hour
later when the hub held part back; changed counts go at the minute even after a hold,
so an achievement unlocked then reaches the hub within a minute). When the hub's answer
unlocks an achievement, the profile is read again within 30 seconds, so its kyber
crystal is listed at once rather than at the next ten-minute reading (reports 149 and
150, 26/10/2026). The hub keeps each key's best count, up to the goal, and when it was
reached. Because the hub cannot see a match, it takes the counts as the player's own
record and bounds how fast each may rise (an hourly allowance per achievement: what
is over is taken in a later hour); a count the client keeps is never lower than the
hub's, so another PC or a reinstall takes the hub's counts back. Achievements the hub
counts itself come from what the key did there and no request can set them.

Since 26/10/2026 the hub has the last word on the achievements the client counts. With
the identity on and a hub address (`player_identity::hub_on`, whether the hub answers
or not), one whose count reaches its goal here is not unlocked yet: it waits for the hub
(`Record::awaiting`, kept in `achievements.json`), which takes the count at its hourly
pace, unlocks it and grants its crystal minutes to hours later. Until then the board
and the hover card show it reached but "Waiting for the SJK hub" (`Standing::waiting`),
and nothing is announced. Once the hub's profile holds it unlocked (`unlocked` above 0),
the client announces it, with its crystal, once, at the next reading after it, also
when the hub unlocked it while the game was closed. An achievement the hub unlocked that
did not wait here (reached on another PC) is not announced. Before, the client unlocked
and announced it with "Reward: ... Kyber Crystal" at once, and players read that a
crystal had come while the hub had not unlocked it yet. With the identity off, or no hub
address, there is no hub to wait for: the client unlocks it and announces it at once,
with its category where the reward would stand, as no crystal comes; what was waiting
for the hub when the identity is turned off unlocks the same way. Achievements the hub
counts are unchanged.

When one of the client's achievements is unlocked as above, it writes "Achievement
unlocked" to the console and shows its pop-up
([achievement_toast.rs](../crates/sjk-viewer/src/achievement_toast.rs)): a card at the
top centre of the screen, clear of the crosshair, the chat and the HUD's corners, with
the board's medallion, "Achievement unlocked", the name, the category and what it
asked (a hidden one too: it is unlocked). It takes no input and pauses nothing, over play and over the menus alike. It
slides down and grows into place in under half a second while a gold ring sweeps
round the medallion, light bursts from it (a glow, a ring of light, sparks), its edge
flares and a glint crosses the card; it holds five seconds and fades out. Several
unlocks queue and show one after another. It waits while the console is open, a
large pop-up shows, the window is away (alt-tabbed out or minimised; since 10/10/2026,
Sol's request) or the player is in a private duel (since 20/10/2026, hub report #81: the
rewards play after the duel), and its clock runs only while it is drawn: a card hidden half way
resumes where it was when the player comes back, and a long frame (a hitch, a map
loading, a minimised window drawing nothing) moves it on by a tenth of a second at
most, so no card is spent unseen. Each pop-up plays `sound/interface/secret_area.mp3`, the
single-player game's sound for a secret area found (its game module plays it with the
`@SP_INGAME_SECRET_AREA` centre print), which multiplayer installs have in the shared
`assets0.pk3`; `cg_achievementSound 0` (Settings > Sound > Unlock sound) leaves it
out. The same card ([unlock_toast.rs](../crates/sjk-viewer/src/unlock_toast.rs))
announces a new medal ([Medals](#medals)) or holocron
([holocrons.md](holocrons.md#the-pop-up)) that arrives during a match, in the queue with
the achievements: "New medal" or "New holocron", "Open the game menu to see it" where the
category stands, the medallion's picture or the tier's icon, and a holocron's light in its
tier's colour. One arrival brings at most three cards, and the large pop-up that opens with
the game menu takes back the cards of what it shows. The new medal and holocron pop-ups'
entrances (and their fanfare) also wait for the window to come back. A newer SJK release
the update check finds has a card too ([client.md](client.md#updates)). The board has two
pages (since 30/10/2026, Sol: "the general/normal ones and the more specific ones / fun
ones", so it can keep growing): General, with Combat, Duels and flags, Journeys,
Community and Collection, and Fun, with a category for each toy (Cardboard box, Order
66, Hax, Drone, Boombox, Monitor, Box hunt; `Category::page`). The switch at the wall's
top right, or F, turns them; the board opens on General. Each page shows every
achievement listed on it as a wall of medallions in four bands of up to twelve (since
26/10/2026; each category's run under its name and count, short categories sharing a
band, the categories in their own order, `collection_achievements.rs`): a medallion
with the goal that fills with the count, gold once unlocked, and the name under it.
The arrows and Tab read the wall in its order; the one chosen shows up close on the
right with what to do, a bar and the count or the date it was unlocked, and "Next up"
the page's three nearest to unlocking. The total at the top counts both pages.

What this proves: an achievement is the player's own record, as a claim is the
player's own word. A modified client or an edited `achievements.json` can send counts
nothing happened for; the hourly allowances only slow that down. Achievements a
dedicated server would vouch for are not built.

## Staff

Staff is a flag on a player, like verified, that only the hub operator sets; a player
cannot ask for it or set it. Profiles carry it, so anyone can see who is staff. A
staff key gets the SJK team's tools in the game: the Profile page shows Staff tools,
as does the SJK UI in-game menu's row of icons (and the `staff` command opens them; for
any other key it says they are for staff).

The creator (since 20/10/2026) is SJK's own creator: a flag above staff that only the hub
operator sets, and that makes the person staff too. Profiles, chat messages and the
chat's people list carry it (`creator`, absent as false). Where staff show as such the
creator shows as the creator: the player card's pill reads SJK CREATOR (sun orange,
wider) instead of SJK TEAM, the sender card's status line says "SJK player, SJK
creator", and the Chat page's message line starts "Creator" instead of "Staff". The hub
refuses staff's tools on the creator (an error `creator`), so other staff cannot mute,
unverify, take from or merge them, nor delete their chat messages. The creator alone
makes players staff or takes it away from inside the game (since 20/10/2026, Sol's
call): on the Staff page's Overview, beside Verify, their own **Make staff** or
**Remove staff** button acts on a second press (`StaffRequest::SetStaff`, hub `POST
/v1/staff/staff`, logged and undoable by the owner). It never shows on the creator's
own player, and admin panel access stays the operator's.

Since 23/10/2026 (Sol's call) what takes back, clears or merges is the creator's alone
too: taking back medals, saber shaders and holocrons, credits and shards (given or taken),
clearing achievements, taking a picture down, unlinking and merging keys. The hub refuses
those requests from other staff (an error `creator_only`); on their Staff page those
buttons show dimmed, Tab skips them and a press does nothing (`owner_only` in
`staff_panel.rs`), and the Achievements and Keys tabs' status line says the tab is to read.

Since 08/11/2026 (Sol's call) every staff member gives, from the Staff page: medals (and
their rank), saber shaders, holocrons and kyber crystals (`gift` in `staff_panel.rs`; the
hub's `PROTOCOL.md`, "Staff"). Three limits keep it from becoming a source of loot, and the
creator has none of them:

- never to themselves (their own person, by any of their keys): the hub refuses it
  (`own_gift`), and on their own player the page dims every Give, Rank and Unlock with
  "Staff cannot give to themselves";
- only the saber shaders holocrons can bring (`droppable`): a Unique one, a medal's blade
  or a blade not opened to drops yet stays the creator's to give (`creator_only`); its row
  says "owner only" and its Unlock is dimmed;
- a number a day (the hub's `GIFTS_PER_DAY`, 20 to begin with, gifts undone not counted):
  past it the hub answers `gift_quota` and the page dims the gifts with "No gifts left
  today". The page reads how many are left the first time one of those two tabs opens
  after the page does (`StaffQuery::Gifts`) and counts down with each gift; the Medals and shaders tab says it under the medals, the Holocrons
  tab beside the tier chosen.

Taking back stays the creator's, but a staff member undoes their own gifts for a day, on
the **My gifts** tab (below): each gift is a row of the hub's staff log, and the undo
puts the player back as they were, through the same undo the owner uses (refused when
something changed since, a holocron opened for one). Every gift and every undo stays in
the staff log, and the owner can still undo everything a staff member did. Other staff keep
search, verify, chat moderation, picture blocks, log requests and the Logs, Crashes and
Chat bans tabs.

The JKR creator (since 22/10/2026) is the creator of JKR, the client SJK is built on
(Bishop): a flag only the hub operator sets (`sjk-hub jkr-creator KEY_ID`), carried by
Profiles as `jkr_creator` (absent as false). It is a mark and nothing else: no staff
rights, no tools, no protection. The player card shows it as a rust JKR CREATOR pill,
after SJK TEAM or SJK CREATOR when the person has one.

The Staff page ([staff_panel.rs](../crates/sjk-viewer/src/staff_panel.rs), the SJK
UI's look) finds players down its left: a name or part of one (colour codes and symbols
ignored, any of the 20 names the hub keeps for them) or a key id typed in the search
pill; Seen lately, the players the hub saw last; and since 08/11/2026 three scope chips
(`StaffRequest::SearchIn`, `sjk_identity::SearchScope`): **Here**, the SJK players on the
server this player is on (offered only while the hub sees them on one), **Online**, the
players in a game now on any server, and **Team**, the SJK team. The chip chosen stays
lit and the search pill then searches within it ("Find among the players playing now");
another press on it goes back to everyone. The hub answers 30 at most; the list shows 15
a page with `<` and `>`. Each row has the name, the flags and a second line: "you" on
the player's own, "playing here as <name>" (gold) or "playing now on <ip:port>, as
<name>" for a player in a game now (the search's `playing`), otherwise when the hub last
saw them in a game; and when a name search matched a name they wore before rather than the
one shown, "wore <that name>". A search that finds exactly one player chooses them, so
`staff <name or key id>` in the console opens the page on that player straight away. The
page shows the chosen one (the player themself until another is chosen, or with Me) on
the right:
their picture, name, flags, how many keys and when last seen beside the note field (the note sent with the next
medal, unlock, holocron, credits or shards, and with a log request, which everyone
reads), over a row of tabs (reworked 18/10/2026 at Sol's request, "manage all that's
needed better"; the page used to squeeze everything into three columns). Since
26/10/2026 no key id shows anywhere on it ([Settings and commands](#settings-and-commands)):
a key id typed or pasted into the search or the merge field shows as bullets and "(key
id)" (with the player's name when the list has them), linked keys are numbered, a chat
ban given by a player without a name says "a player", and the status line never names a
key ("Merged the other player into this one", "Unlinked the key: it starts afresh").

- **Overview**, where the page opens: the account (flags, trust, member since, last seen,
  keys counted, medals and unlockables held, achievements, unopened holocrons and kyber
  crystals, credits, shards and cards), Verify (`StaffRequest::Verify`; Unverify while the player is
  verified), the last six names the hub kept, and moderation: their picture (Take down,
  `StaffRequest::AvatarRemove`; Block pictures and Allow pictures,
  `StaffRequest::AvatarBlock`, [Pictures](#pictures)) and the SJK chat (Mute at the hub
  and Unmute, `StaffRequest::ChatMute`; Lift chat bans, `StaffRequest::ChatUnban`,
  [hub-chat.md](hub-chat.md)). The hub does not say whether a player's pictures are
  blocked or whether they are muted at the hub, so both ways are offered;
- **Medals and shaders** (Medals and unlocks until 08/11/2026): every medal of the
  catalogue with what the player holds (its
  medallion wearing its rank's device, the rank's name beside the date): a held medal's
  Rank button ("Rank III"; each press sets the next rank, V back to I,
  `StaffRequest::MedalRank`, `POST /v1/staff/medal-rank` with `key_id`, `medal` and
  `rank`, answered with the profile as an award is; since 27/10/2026, quiet like every
  staff change: only the player's own pop-up tells them; on a medal not held, since
  28/10/2026, each press picks the rank Give gives it at, shown as "gives at IV", sent as
  `rank` in `POST /v1/staff/award` only above I, back to I once given), Give
  (Give +1 for a repeatable one already held) and Take back (one award; a repeatable one
  counts down); beside them the saber shaders (every unlockable), with a filter over them
  (since 08/11/2026: a name or part of one, a number such as `42`, `042` or `#42`, a tier
  such as `mythical`, or the id; the list follows the typing) and **Held only**, each with
  its number, its tier or since when the player holds it: Unlock (`StaffRequest::Unlock`)
  and Relock (`StaffRequest::Relock`), in pages with `<` and `>`;
- **Holocrons and credits**: the player's holocrons and kyber crystals, the four holocron
  tiers and under them the four crystal tiers to choose, Give (`StaffRequest::HolocronGive`)
  and their twelve newest holocrons and crystals with Remove
  (`StaffRequest::HolocronRemove`, on a second press within 3 seconds)
  ([holocrons.md](holocrons.md#staff)); beside them their credits and kyber shards, an
  amount (digits only) and Give credits, Take credits, Give shards and Take shards
  (`StaffRequest::Credits` and `StaffRequest::Shards`, `POST /v1/staff/credits` and
  `/v1/staff/shards` with the amount as `delta`, negative to take; Take waits for a second
  press). The hub refuses taking more than they hold;
- **Achievements**: the player's achievements at the hub in two columns, each with
  Clear, and Clear all, which waits for a second press within 3 seconds;
- **Keys**: the player's main key and their linked keys (numbered, never shown), each
  with Unlink
  (`StaffRequest::Unlink`, which waits for a second press within 3 seconds), then Merge
  another key into this player ([Players with several keys](#players-with-several-keys));
- **Logs**: Request logs for the chosen player and the log requests with Open
  ([Log requests](#log-requests));
- then three tabs set apart, about every player: **Crashes**, every player's crash
  reports by kind, then one kind's with Open; **Chat bans**, the SJK chat bans in force,
  newest first (`StaffQuery::ChatBans`, `POST /v1/staff/chat-bans`), each with who
  banned them, until when and whether their address is banned too, and Lift
  (`StaffRequest::ChatUnban`; a ban that holds only an address has no key to name and
  cannot be lifted from here; a lift takes the player's bans off the list); and, since
  08/11/2026, **My gifts**, the staff member's own gifts of the last day, newest first
  (`StaffQuery::Gifts`, `POST /v1/staff/gifts`): what (a medal and its rank, a shader by
  number and name, a holocron or crystal tier), to whom, when and the note, and how many
  gifts are left today ("no daily limit" for the creator), each with Undo on a second
  press (`StaffRequest::GiftUndo`, `POST /v1/staff/gift-undo` with the gift's number; the
  answer is the player's profile, which replaces theirs on the page, and the gift is marked
  undone).

Each request's answer shows in the status line at the bottom left; one that answers a
profile replaces the chosen player's, and the player's own when it is theirs.

Each action is a request signed by the staff member's own key (`PROTOCOL.md`,
"Staff"); the hub refuses it from a key that is not staff, keeps a log of every staff
action, and limits a staff key to 120 requests an hour. Staff cannot make staff or change names
and bios.

Clearing achievements at the hub alone does not stick for the ones a client counts:
the player's game sends its counts again (clearing is the creator's alone at the hub,
above). So when the creator clears their own,
the client also forgets them (`achievements::forget`): the counter goes just below
the cleared achievement's goal, and every achievement on the same counter with a
higher goal is cleared with it (clearing First Blood clears Centurion and Legend of
the Arena, as the kill count goes to 0). The next kill, duel or map then unlocks it
again, which is how the unlock is tested. Clear all forgets every count on this PC.
Clearing another player's client-counted achievements only lasts until their game
sends its counts, which the page says.

### Players with several keys

A player is one main key and any number of linked keys (`Profile::keys`: the main key
first; older hubs send none). A linked key signs as its player: the hub answers the
player's id (`Profile::key_id`, the main key's) for it everywhere, in presence, the
feed and profiles, so the player has one name, one set of medals, unlocks,
achievements and holocrons, whichever PC they play from.

Keys get linked by a merge, for a player who reset their key (deleted `identity.key`
by accident, a new PC without the backup). On the Staff page, with the kept player
chosen (the one with the history, usually), the Keys and merge tab takes the key id
of the other player, pasted (Ctrl+V) or typed, or Pick in list and then a click on them
among the players found (the field shows bullets, never the key, and the player's name
when the list has them); the page says when it is not a key id, already this
player's, or a staff player (who is never merged away). Merge... only asks: a card
says who is kept and who is merged away, by name and when last seen (never by key), and
that it cannot be undone, with Cancel first under the keyboard and Merge for good
(`StaffRequest::Merge`). Everything of the merged-away player (medals, unlocks,
achievements, holocrons, names, reports, the picture when the kept player has none, a
verified mark) goes to the kept player and their keys become its linked keys; the hub's
answer replaces the kept player on the page and takes the merged-away one out of the
list. Unlink detaches one linked key: the key starts afresh the next time it is used
(a new player at the hub, as a new key would be), and nothing it brought moves back.

The client follows the person: once the hub answers the player's own profile,
`Snapshot::key_id` is the player's id, so a linked key still recognises itself in the
presence lists, its own holocron drops and looks in the feed, and on the Staff page.
`Snapshot::local_key_id` stays this PC's key id: the Identity page shows and copies it,
and what is kept per key on this PC (the medal and holocron pop-ups' seen lists) keeps
using it. When staff unlink the key this PC uses, the client registers it again at
once.

## Settings and commands

- `cl_identity` (default 1; Settings > Network > SJK identity) turns the feature on.
- `cl_hubUrl` (default `https://sjk.dfox.app`; Settings > Network > SJK hub) is the hub's address.
  It must be `https://host[:port]` with no path; plain `http://` is accepted for
  localhost only.
- `profile` opens the Profile page (again: closes it), in the SJK UI the Profile
  screen's SJK Profile tab; `collection` opens the Collection, `achievements` on its
  Achievements tab and `unlockables` on its Shaders tab (in the SJK UI the Collection
  screen, [sjk-ui.md](sjk-ui.md#collection)).
- `staff` opens the Staff page, for a staff key only (again: closes it); `staff <name or
  key id>` opens it searching for that player, and chooses them when the search finds
  only them.
- `sjkavatar <file>` reads a picture file and shows it on the Profile page, ready to
  send with Use this picture; `sjkavatar clear` takes the player's picture down;
  `sjkavatar` alone opens the picture panel ([Pictures](#pictures)).
- `cg_saberSkin` (archived, default empty) is the blade-skin unlock id the player
  wears; it applies, and is sent, only while the own profile lists it
  ([unlockables.md](unlockables.md)). `saberskin` lists the blade skins (owned or
  locked) and `saberskin <id>`/`none` sets it; `unlockables` opens the Collection's
  Shaders tab, where owned ones are equipped.
- `cl_shareModels` (default 1; Settings > Network > Share my model list) sends the
  player model folders the game has, so other SJK players' cards say whether they see
  the model you wear ([Installed models](#installed-models)); 0 makes the hub forget
  the list.
- `cl_crashReports` (default 1; Settings > Network > Send crash reports) sends crash
  reports to the hub at the next start ([Crash reports](#crash-reports)); 0 keeps them on
  the PC.
- `cl_shareUsage` (default 1; Settings > Network > Share settings usage) sends the
  settings changed and searched in Settings, anonymously, without the key
  ([client.md](client.md#settings-usage)); 0 sends nothing.
- `cl_sjkChat` (default 1; Settings > Chat > SJK chat) shows the SJK chat and reads
  it; `sjkchat` opens its page, `messagemode5` (I) its composer in a game, and
  `sjkemote <id>` sends an emote ([hub-chat.md](hub-chat.md)).
- `cg_achievementSound` (default 1; Settings > Sound > Unlock sound) plays the
  secret-area sound with each unlock card: an achievement's, or a new medal's or
  holocron's in a match.
- `debug_medal <id|all> [x<count>] [note]` shows made-up medals in the new medal
  pop-up, sending nothing ([Medals](#medals)).
- `debug_holocron <tier|all> [x<count>]` shows made-up holocrons in the drop pop-up,
  sending nothing ([holocrons.md](holocrons.md#debug_holocron)).
- The Identity page (Settings > Network > SJK identity key, main menu > SJK > IDENTITY
  in the classic menus, the in-game SJK menu, or the `identity` command) shows what the
  hub knows: the name worn now and up to three earlier ones, whether the key is
  verified, the key file's location and the players the hub knows here. The key id is
  always bullets and "(hidden)", and the file (whose path names the Windows account)
  until Show file is pressed, every time the page opens (not saved), so a player can
  open it on a stream (09/10/2026, Sol's request).

  **Key ids never show in the game** (26/10/2026, Sol's rule: "ALWAYS hide the keys,
  never show them in game"), the player's own or anyone else's: not on the Identity or
  Profile page, the Players cards, the player card, the report page, sender cards, the
  SJK chat page's side panel, the Staff page ([Staff](#staff)), nor in the console's
  `identity key`, `identity who` and `block` lists. Where a key used to show, a player
  goes by name (and the report page says "known" or "known, verified").
  `player_identity::is_others_key` only tells whose a key is (for chat bans). Its
  controls are optional: the on/off switch, a bio field with Save, a button that copies
  the key id (to the clipboard, never on screen), Show file. The words do the same
  without it: `identity bio <text>` sets the bio, `identity key` copies the key id to the
  clipboard and says where the key file is, `identity who [slot]` lists the players the
  hub knows here (with a slot, their bio). `identity name` explains that the name is the
  one played under (`/name`).

## The key file

`identity.key` holds the private key as two lines (`SJK-IDENTITY-1` and the base64url
seed). It is created once and never overwritten; a damaged file is reported and
left alone, and the feature stays off until it is restored, because replacing it
would silently end that identity. The player must back it up: it cannot be
recovered, and copying it to another PC makes that PC the same identity. On Unix it
is created readable by its owner only; on Windows it relies on the folder's default
permissions.

## Planned, not built

The hub is meant to grow: music and video. None of that exists. The SJK chat and the
emotes path ([hub-chat.md](hub-chat.md)) and the asset packs ([Asset packs](#asset-packs))
are built. In the SJK UI the Identity page opens from the Profile
page (its arc holds five entries), the in-game SJK menu and the `identity` command.
Achievements a dedicated server would vouch for, and pictures for the achievements,
are not made yet.

## Profile banners

A profile may equip an owned static cover, shared by its profile header and player
cards. Choose it in Collection > Banners; the hub validates and saves equipment.
Old or unknown covers use the neutral fallback. See [profile banners](profile-banners.md).
