# SJK chat and emotes

SJK chat is one conversation for every SJK player, carried by the SJK hub
(`cl_hubUrl`, sjk.dfox.app) instead of the game server: it reaches SJK players on
any server, or in the menus with no server at all, and players on stock clients
never see it. Emotes ride the same hub connection so SJK players on one game server
see each other's emotes; the emotes themselves (animations, sounds, art) are
separate work, and SJK only has the path for them so far. The feed also carries the
players' looks (blade skin and Illuminate, [unlockables.md](unlockables.md)), so it
reads on a game server even with the chat off.

Status (08/10/2026): built on `personal/sjk-chat` (client) and `feat/chat-emotes`
(Sol-Vulpes/SJK-hub), designed with Sol the same day; the hub's side is not deployed.
See [Verification](#verification) for what was checked.

The hub's side is described in Sol-Vulpes/SJK-hub (`PROTOCOL.md`, "Chat", "Emotes" and
"The feed"). Chat and emotes are signed by the player's identity key and need
`cl_identity 1` ([identity.md](identity.md)).

## Decisions

| Question | Decision |
| --- | --- |
| Who may write | Any registered SJK key; reading the feed needs a registered key too. |
| History | In the hub's memory only: the last 200 messages, gone on a restart. |
| Menus | A docked box on the SJK UI's main page and in-game menu, and a full SJK chat page. |
| Emote ids | Open: the hub relays any well-formed id; the catalogue is client data. |

## Pieces

| Piece | Where |
| --- | --- |
| Text rules (word for word with the hub's `src/chat.rs`) | [chat.rs](../crates/sjk-identity/src/chat.rs) |
| Clan chats: channels, who joins ([Clan chats](#clan-chats)) | [channels.rs](../crates/sjk-identity/src/channels.rs), [chat/tabs.rs](../crates/sjk-viewer/src/chat/tabs.rs) |
| Wire types (`ChatMessage`, `Emote`, `LookEvent`, `Feed`) | [wire.rs](../crates/sjk-identity/src/wire.rs) |
| Requests (`chat`, `emote`, `feed`) | [hub.rs](../crates/sjk-identity/src/hub.rs) |
| The feed thread and `ChatState` | [feed.rs](../crates/sjk-identity/src/feed.rs), [service.rs](../crates/sjk-identity/src/service.rs) |
| Viewer glue | [player_identity.rs](../crates/sjk-viewer/src/player_identity.rs) |
| Blocking a player: the list, matching slots, `block`/`unblock` | [chat_mutes.rs](../crates/sjk-viewer/src/chat_mutes.rs), [player_mutes.rs](../crates/sjk-viewer/src/player_mutes.rs), [block_command.rs](../crates/sjk-viewer/src/block_command.rs) |
| Chat bans: the words and the hub's answer | [ban.rs](../crates/sjk-identity/src/ban.rs) |
| Chat bans: Ban 24 h, armed then confirmed | [chat_ban.rs](../crates/sjk-viewer/src/chat_ban.rs) |
| Blocking a player: drawn as Kyle, silenced | [muted_players.rs](../crates/sjk-viewer/src/muted_players.rs), [muted_players_frame.rs](../crates/sjk-viewer/src/muted_players_frame.rs), [audio_mute.rs](../crates/sjk-viewer/src/audio_mute.rs) |
| Blocking for this map, and staff blocks for everyone | [map_blocks.rs](../crates/sjk-viewer/src/map_blocks.rs), `StaffRequest::MapBlock` in [staff.rs](../crates/sjk-identity/src/staff.rs), `MapBlock` in [wire.rs](../crates/sjk-identity/src/wire.rs) |
| Sender card (pointer on a name) | [sender_card.rs](../crates/sjk-viewer/src/sender_card.rs), [chat/card.rs](../crates/sjk-viewer/src/chat/card.rs) |
| In-game SJK channel | [chat/sjk.rs](../crates/sjk-viewer/src/chat/sjk.rs), [chat/view/sjk_line.rs](../crates/sjk-viewer/src/chat/view/sjk_line.rs), [sjk_chat_frame.rs](../crates/sjk-viewer/src/sjk_chat_frame.rs) |
| Said on the game server too (JoF servers) | [chat/server_copy.rs](../crates/sjk-viewer/src/chat/server_copy.rs); the copy's tag and end in [sjk-client chat.rs](../crates/sjk-client/src/chat.rs) (`SJK_COPY_TAG`, `sjk_copy_*`); its `say` commands and the notice's words in [sjk-mod-jof sjk_chat.rs](../crates/sjk-mod-jof/src/sjk_chat.rs); sent by `sjk_chat_frame.rs`, the card in [mod_notice_card.rs](../crates/sjk-viewer/src/mod_notice_card.rs) |
| Other clients' icons | [chat/client_icons.rs](../crates/sjk-viewer/src/chat/client_icons.rs) (`KNOWN`, `cl_sjkChatClientIcons`) |
| How a line looks everywhere (gold, tick, flow) | [sjk_chat_look.rs](../crates/sjk-viewer/src/sjk_chat_look.rs) |
| The dock (main page, in-game menu) | [chat_dock.rs](../crates/sjk-viewer/src/menu/sjk/chat_dock.rs), placed by [home.rs](../crates/sjk-viewer/src/menu/sjk/home.rs) and [ingame_menu/sjk_view.rs](../crates/sjk-viewer/src/ingame_menu/sjk_view.rs) |
| Typing and pasting in the page's and docks' field | [sjk_chat_field.rs](../crates/sjk-viewer/src/sjk_chat_field.rs) |
| SJK chat page | [sjk_chat_panel.rs](../crates/sjk-viewer/src/sjk_chat_panel.rs), [sjk_chat_panel_view.rs](../crates/sjk-viewer/src/sjk_chat_panel_view.rs), [console_sjk_chat_page.rs](../crates/sjk-viewer/src/console_sjk_chat_page.rs) |
| Emotes | [emotes.rs](../crates/sjk-viewer/src/emotes.rs), [emotes_frame.rs](../crates/sjk-viewer/src/emotes_frame.rs) |
| GIFs from GIPHY: links, fetching, decoding, the cache, drawing | [chat_gifs.rs](../crates/sjk-viewer/src/chat_gifs.rs) and its `link`, `fetch`, `decode`, `draw` modules; textures in [gif_textures.rs](../crates/sjk-viewer/src/ui_renderer/gif_textures.rs) |
| The GIF picker: tabs, tiles, picking; favourites; GIPHY's search | [sjk_chat_gif_picker.rs](../crates/sjk-viewer/src/sjk_chat_gif_picker.rs); [chat_gifs/favourites.rs](../crates/sjk-viewer/src/chat_gifs/favourites.rs); [chat_gifs/search.rs](../crates/sjk-viewer/src/chat_gifs/search.rs) |
| Announcements and reward rounds: wire types, rules, requests | [news.rs](../crates/sjk-identity/src/news.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [feed.rs](../crates/sjk-identity/src/feed.rs), [service.rs](../crates/sjk-identity/src/service.rs) |
| Announcements and reward rounds: lines, seen file, windows, commands | [official.rs](../crates/sjk-viewer/src/official.rs) and its `words`, `seen`, `window` modules |

## Hub protocol (additions to version 1)

Old clients ignore all of it, so it stays `/v1/`.

- A message in the feed carries `client` (hub since 25/10/2026): `sjk` when it was
  sent with SJK's User-Agent, `ejk` with JoF EternalJK's (`EJK/...`), left out for
  any other (`ChatMessage::client`, empty then). It picks the line's emblem ([How a
  line looks](#how-a-line-looks)).
- `POST /v1/chat` `{"text":"..","name":".."}` (`name`, the in-game name worn, is
  optional and joins the key's worn names). The text: one line (`chat_lines`), runs
  of spaces made one and the ends trimmed, 1 to 300 characters (`chat_length`; 150
  until 16/10/2026, Sol's request for longer messages; a
  client from before then shows a longer message's first 150) of the
  bio's alphabet with `^` only before a digit (`chat_characters`), no run of one
  character longer than 8 (`chat_noise`). A key sends at most one message every 2
  seconds and 15 a minute, a verified or staff key one every second and 30 a minute
  (`chat_quota`), not the same text twice within 30 seconds
  (`chat_duplicate`); a muted key is refused (`chat_muted`).
- `POST /v1/emote` `{"server":"ip:port","emote":".."}`: the key needs a live claim on
  the server (`not_on_server`), and the hub takes the slot and claimed name from it,
  so a key emotes only for the slot it claims. Ids are 1 to 32 of `a`-`z`, `0`-`9`,
  `_` (`bad_emote`); at most one every 2 seconds, every second for a verified
  or staff key (`emote_quota`). An emote lives 10
  seconds.
- `POST /v1/ping` `{"server":"ip:port","pos":[x,y,z],"scope":".."}` (since 13/10/2026,
  [Pings](#pings)): as an emote, the key needs a live claim on the server and the hub
  takes the slot and claimed name from it; at most 5 in 10 seconds (`ping_quota`), kept
  10 seconds, in the feed's `pings` for that server's readers.
- `POST /v1/toy` `{"server":"ip:port","toy":"..","event":"..","target":<slot>}` (since
  16/10/2026, [Toy events](#toy-events)): as a ping, from the slot the key's live claim
  holds; at most one a second and 10 a minute (`toy_quota`), kept 10 seconds, in the
  feed's `toys` for that server's readers.
- `POST /v1/relay` (a toy event's body) and `GET /v1/relay?server=..&after=..&wait=..`
  (since 21/10/2026, [Toy relay](#toy-relay)): toy events for one player at once, 40 a
  second per key (`relay_quota`), kept 3 seconds, never in the feed; the read answers
  `{"events":[..],"next"}` with the events for the reader's own claimed slot.
- `GET /v1/feed?after=<id>&server=<ip:port>&wait=<0..25>`, signed. Messages, emotes
  and deletions share one id sequence. The hub answers as soon as it holds something
  newer than `after` for this reader, else after `wait` seconds:
  `{"next","chat":[..],"emotes":[..],"looks":[..],"deleted":[..],"online"}` (`looks`,
  the server's look events of the last 60 seconds, since unlocks and looks). `after` 0,
  or above the newest id (the hub restarted), gives the newest 50 messages, the
  server's looks of the last 60 seconds (looks are state) and no emotes.
  A key reads at most 40 times a minute (`feed_quota`); the feed (300 a minute) and chat
  and emotes (60 a minute) have per-address allowances of their own, so several players
  sharing an address fit and talking never spends what claims need.
- Moderation: a `chat_muted` flag on keys (the operator's `chat-mute` and
  `chat-unmute`, `{"chat_muted":true}` in the operator API) and two staff requests,
  `/v1/staff/chat-delete` `{"id"}` and `/v1/staff/chat-mute` `{"key_id","muted"}`,
  logged in the staff log (a deleted message's text with it).
- Chat bans ([Chat bans](#chat-bans), since 16/10/2026): `POST /v1/chat/ban`
  `{"key_id":".."}`, signed, answers `{"until":<unix s>,"address":<bool>}`; refused
  `not_verified` (403, the banner is neither verified nor staff), `not_found` (404),
  `target_verified` (403, the target is verified or staff), `bad_target` (400, oneself).
  `POST /v1/chat` then refuses the banned player `chat_banned` (403) with `until`. Staff
  lift bans with `/v1/staff/chat-unban` `{"key_id"}` (answer `{"lifted":<n>}`).

- Clan chats (03/11/2026, [below](#clan-chats)): `POST /v1/chat` takes an optional
  `"channel":"jof"|"sith"` (400 `bad_channel`, 403 `not_in_channel`); the feed takes
  `channels=jof,sith` and its messages carry `channel` (left out for SJK chat); every
  answer to a player who may read one lists `channels`
  (`[{"id":"jof","home":true},..]`). The hub's side is `PROTOCOL.md`, "Clan chats".
- Announcements and reward rounds (25/10/2026, [below](#announcements-and-reward-rounds)):
  `GET /v1/announcements`, `GET /v1/rewards`, `POST /v1/rewards/claim` `{"id":n}` and the
  creator's `POST /v1/creator/announce`; the feed's `announcements`
  (`[{"id":<feed id>,"announcement":{..}}]`) and `rounds`
  (`[{"id":<feed id>,"round","at","title","count"}]`), every reader told, kept 5 minutes,
  a fresh reader getting those of the last 60 seconds, left out when none. The hub's
  side is `PROTOCOL.md`, "Announcements" and "Reward rounds".

## Client service

- Sending (`Service::chat`, `Service::emote`) goes through the identity worker; the
  rules are checked before anything is sent, as a bio's are. What became of the last
  message or emote is `ChatState::outcome`.
- `Service::start_with_feed` starts a second thread, `sjk-hub-feed`, with its own HTTP
  client (timeout 40 seconds), so the worker's claims and reports never wait behind
  the long poll. It reads only while the identity is on and the hub answered the
  registration, and then while the player is on a game server (whatever `cl_sjkChat`
  says, for the looks and emotes) or `cl_sjkChat` is on (`Service::set_chat`); in the
  menus with the chat off it makes no request. It polls at most every half second (2
  seconds before 27/10/2026; a hub from before then answers the faster polls with 429
  `feed_quota`, and the feed keeps to 2 seconds for that hub) and backs off (2 to 60 seconds) when the hub fails. With the chat on it keeps the last
  200 messages, the online count and whether the last poll reached the hub
  (`ChatState`; `loaded` says when the hub being read first answered, so the game's
  feed marks that backlog instead of replaying it); with the chat off it keeps none of
  them, so `ChatState` stays as an idle feed leaves it and nothing shows, and turning
  the chat on starts again from the backlog. Either way it queues up to 64 received
  emotes (`Service::take_emotes`) and 64 looks (`Service::take_looks`). A new hub, or
  ids that go backwards (the hub restarted), start from the backlog; another game
  server starts again from `after` 0, whose answer brings that server's looks of the
  last minute. The looks keep the server they were read for and the reading's
  generation (it changes with the hub, the server or the identity going off or on):
  `Service::take_looks` hands over only the current generation's for the viewer's
  server, so a poll under way at a change brings no old look.
- Shutting down tells the feed and vitals threads to stop; they end after their
  request without being waited for.

## In the game

- The chat overlay has a fourth channel, SJK. `messagemode5` (I by default in a new
  profile or a whole imported config, when I is free and nothing is bound to
  `messagemode5`; an existing profile binds it in Settings > Key bindings > Other)
  opens the composer on it; Tab cycles All, Team and SJK, and the clan chats joined
  ([Clan chats](#clan-chats)), then the last whisperer
  ([client.md](client.md#chat-player-actions)); each button may stay out of Tab. The ordinary chat key (`messagemode`,
  Y by default) opens on global chat, or on SJK when the last message went there
  with `cg_chatRememberChannel` 1 (off by default). Enter on SJK hands the text to the hub;
  during a match the game server's chat gets it too ([Said on the game server
  too](#said-on-the-game-server-too)).
- Hub messages join the chat feed as one flowing line ([How a line
  looks](#how-a-line-looks)): SJK's emblem, small, the name, the verified tick for a
  verified sender, then the message in the SJK chat's gold. Texts go through
  `chat::for_display`, names through `chat::name_for_display`: a name keeps every
  character a Jedi Academy name can draw, all of Windows-1252 that prints (`{ }`,
  `\`, `` ` ``, Latin-1's symbols such as guillemets and the section sign, the
  bullet, dagger and trade mark), so clan tags show as worn (Sol's request,
  10/10/2026); the soft hyphen and what reorders or prints nothing stay out, as from
  a message, which keeps the bio's alphabet. Messages staff delete leave the feed,
  and muting a
  player ([Muting a player](#blocking-a-player)) hides their lines already there. While
  the composer is open, resting the pointer on a sender's name shows their profile
  card with Mute or Unmute, and resting it on a link lets Ctrl+C copy the link
  ([client.md](client.md#chat-player-actions)). Joining a game
  does not replay the hub's backlog in the feed (the dock and the page show it). A
  refusal (quota, rules) shows as an `SJK chat:` line. Each live message from an unblocked player is
  also kept in the console scrollback with a textual yellow `SJK` tag (the console
  cannot draw the feed's emblem); it uses the quiet chat path, so it does not enter
  the closed-console notify feed or crash reports. The initial backlog is not copied
  into the console.
- `cl_sjkChat` (default 1, Settings > Chat > SJK chat) shows SJK chat and runs the
  feed; 0 hides it. On a game server the feed still reads, for the players' looks and
  emotes, but no message reaches the game's feed, the dock or the page; in the menus it
  stops.
- `cl_sjkChatFeed` (default 1, Settings > Chat > SJK chat in game feed) controls only
  whether live SJK messages join the in-game general chat. Turning it off removes its
  existing SJK lines and consumes later messages without replaying them when it is
  turned on again. The dock at the right of the Escape menu and the full SJK chat page
  remain visible, and the hub's reason for refusing a message the player sent still
  shows in the feed (the in-game composer has nowhere else to say it).
- `cl_sjkChatConsole` (default 1, Settings > Chat > SJK chat in console) controls only
  whether live SJK messages are copied into console scrollback.
- `SJK_APPEAR_OFFLINE` (environment, any value but empty or `0`; 19/10/2026) keeps the
  chat off for that run as `cl_sjkChat 0` does, without saving it
  (`sjk_chat_frame::appears_offline`; every reader of the switch goes through
  `ViewerConsole::sjk_chat_on`). In the menus the feed is not read, so after a minute
  the hub no longer lists the player as online: the player appears offline while
  testing. The Settings row still shows the saved `cl_sjkChat`.

## Said on the game server too

On JoF servers only, what the player says on the SJK chat during a match also goes to
that server's All chat, so its players on other clients read it. JoF asked for it so
that SJK chat does not split their players away from All chat (Sol's decision,
22/10/2026, after Milamber's #194). It lives in JoF tools ([mods.md](mods.md#sjk-chat-on-jof-servers)):
on every other server, in local games and in the menus, SJK chat goes nowhere else, and
the base client has no server-specific rule for it. JoF EternalJK says its SJK chat
lines on the server the same way, and the two keep a message from showing twice to SJK
chat players there ([chat/server_copy.rs](../crates/sjk-viewer/src/chat/server_copy.rs)).
The hub takes no part in it.

- **Saying it.** Every field that sends SJK chat (the composer, the docks, the page,
  `sjksay`) goes through `player_identity::chat`, which notes the text. While JoF tools
  are loaded (a JoF server, `mod_jof` on) and the player is in a match there, the copy
  goes once the hub took the message and the player's own message came back in the feed
  with the hub's id: as `say`, starting with `^3SJK^2 ` (`sjk_client::SJK_COPY_TAG`) so
  the server's players on any client read it as an SJK line (`Name: SJK text`, SJK in
  gold), and ended with that id in colour codes no client draws
  (`sjk_client::sjk_copy_marker`: `^9`, then `^` and the number of digits, then `^` and
  each digit, so id 12 ends the line with `^9^2^1^2`). A server keeps 149 bytes of a
  line, so a longer message goes as several, cut between words, each with the tag and
  the end; the reliable channel sends them a second apart
  (`sjk_mod_jof::sjk_chat::copy_commands`). Only messages the hub took go: one it
  refused, or did not answer in 10 seconds, never reached SJK chat and is not said on
  the server either. A copy for a match that ended goes nowhere. A message with a GIF,
  one sent with the page's Send only in SJK, and a staff key's sent with Shift held
  stay in SJK chat ([mods.md](mods.md#sjk-chat-on-jof-servers)).
- **Telling the player.** The first time the player is in a match on a JoF server with
  SJK chat on, a card over the game menu says that SJK chat is said in All chat there
  and that JoF asked for it ([mod_notice_card.rs](../crates/sjk-viewer/src/mod_notice_card.rs),
  `jof.sjkChatNotice`, turned off by Got it; on again, it shows on the next JoF
  server). While the in-game composer is on the SJK channel there, a small red note on
  the channels' row says "Also said in All chat: everyone on this server reads it".
- **Showing it once.** While `cl_sjkChat` is on, on any server, a global chat line from
  the server that ends so waits up to 2.5 seconds (`server_copy::HOLD`) for the SJK
  chat message of that id: when it comes, it shows as an SJK chat line (with its
  client's emblem, whole), labelled ALL/SJK, and the server's lines of it are neither
  shown, logged nor copied to the console; one that comes after its message is dropped
  too. A copy is hidden only when it holds the message's words (its letters and digits,
  as the server's old code pages keep them; a piece of a long message counts) and, when
  the hub knows who plays in the copy's slot, comes from the message's sender
  (`SlotMark::key_id`). Any other copy, or one whose message did not come in time,
  shows as it came: a line may show twice, never vanish. A line without the end is never
  held. The server's copy plays no chat beep (`sjk-client` `local_sounds.rs`): the SJK
  chat's own sound announces the message.
- Players on other clients see the copy as the player's normal chat line; the end's
  colour codes draw nothing.

## A sound for a new message

A new SJK chat message plays a sound (Sol's request, 10/10/2026), in the menus and in a
game, wherever the chat is on. Before this, SJK chat messages played nothing. The
player's own messages play it too since 11/10/2026 (Sol's request), when they join the
feed: the hub took them, so the sound says the message went out.

- **The sound** is a variant of JKA's chat sound, made from the game's own file when
  SJK starts: the player's `sound/player/talk` (`.wav`, else `.mp3`) is read through the
  game's file system and decoded as every sound is, then played 1.122 times faster (so
  higher and shorter, read through a band-limited windowed-sinc kernel), cut to 0.60
  seconds with its last 0.15 faded out linearly, given two echoes 70 and 140 ms later
  (0.3 and 0.15 of it beside 0.8 of itself, all times 0.6, not fed back) and brought to
  the talk sound's own level (times 1.5494): 0.74 seconds at 44.1 kHz
  ([sjk_chat_sound.rs](../crates/sjk-viewer/src/audio/sjk_chat_sound.rs), `variant`). It
  is made once, on the audio's decode worker, when the audio first meets the game data
  (`configure_audio`), and kept in memory only. Nothing of JKA's audio ships with SJK;
  game data without the file makes no sound, said once in the log.
- **When:** a message newer than what the feed had already seen joins it
  (`ChatOverlay::sync_sjk`, [chat/sjk.rs](../crates/sjk-viewer/src/chat/sjk.rs)), so the
  backlog the hub sends at start, on a reconnection or from a new hub plays nothing, as
  it is not replayed in the game's feed either. The player's own messages play it when
  they join the feed, sent from the composer, a dock or the page (a refused message never
  joins, so it plays nothing); not for a player muted on this PC, not for holocron drops
  or emotes. At most once a second
  (`chat::sjk::SOUND_GAP`), so a burst plays it once.
- **How it plays:** as the game's chat beep does, beside the listener at full volume
  under `s_volume`, through the interface cues' mailbox (`ui_cues::Cue::SjkChat`) on a
  channel of its own, so it never cuts a menu sound or the game's beep.
- **Switches:** `cl_sjkChatSound` (archived, default 1; Settings > Sound > Chat sounds,
  SJK chat messages, since 23/10/2026 with the other chat kinds), and `cl_chatSoundSjk`
  picks another sound in its place ([client.md](client.md#chat-sounds)). The master
  chat sound switch `cg_chatSounds` gates it too: 0 silences every chat sound, and SJK
  chat is chat. With `cl_sjkChat 0` no message arrives, so nothing plays.

## In the menus

- SJK UI main page: the chat is docked under Recent servers. Its last messages (the
  verified tick after a verified sender's name) sit on the field, newest at the bottom,
  with the online count by its name. The box keeps its height (five one-row messages)
  and each message takes as many rows as it needs (Sol's request, 10/10/2026: a long
  one was cut to one row), so a long message pushes older ones out; a message taller
  than the whole box (a long message, or a shorter one at a larger `ui_textScale`) shows its first rows
  with its name, the last one ending in an ellipsis, and Open chat shows the rest.
  Resting the pointer on a name shows the sender's sender card with Mute, left of the
  dock and above the keys (beside the name it covered the dock's lines). Down past the
  last server (or a click) reaches the field; Enter types (every key goes to the
  field), Enter sends, Escape stops. Open chat opens the page. A line under the field
  says why a message could not go.
- SJK UI in-game menu (Escape in a match): the same dock, drawn by the same code
  ([chat_dock.rs](../crates/sjk-viewer/src/menu/sjk/chat_dock.rs)), in the match card's
  column under its controls ([sjk-ui.md](sjk-ui.md#in-game-menu)). Tab
  reaches its field after the card; Enter types, and while typing every key goes to
  the field, the console key and the game's bindings included; Escape stops typing.
  The sender card shows left of the dock and says where the player is on the server;
  it stays 0.6 seconds after the pointer leaves the name, so the pointer can cross to it.
  The classic in-game menu has no dock.
- The SJK chat page (a console page in the SJK UI's look in every menu style): the
  history, newest at the bottom, each a flowing line (name, tick, text) with Staff and
  how long ago on the right of its first row ("3 minutes ago", as the main page's
  servers say, so no time zone is needed); the field (the keyboard is there when the page opens), Send and the
  character count. Up from the field chooses the newest message, Up and Down move,
  Page Up and Page Down scroll, Tab walks every control. Resting the pointer on a name
  shows the sender's sender card with Mute or Unmute; the page's text whose ink reaches
  the card is left out while it shows, since text draws over every shape and the times
  and messages showed through it (Sol's report, 11/10/2026). A chosen message offers Mute on
  this PC ([Muting a player](#blocking-a-player)); for staff, Delete for everyone, Mute at the hub and Unmute
  at the hub (the client does not know a key's mute flag); under them, what the request
  came to (sending, done, or the hub's refusal). It opens from the dock's
  Open chat, `sjkchat`, `messagemode5` outside a game, the classic in-game SJK menu's
  SJK chat and the SJK UI in-game menu's SJK chat icon.
  The page draws a copy of the chat taken under the identity's lock, never the chat
  itself: a lock held while drawing froze the game. The side panel names no key id
  (none shows in the game, [identity.md](identity.md#settings-and-commands)).
- Pasting (Sol's request, 10/10/2026): Ctrl+V, or Shift+Insert as in the console and
  the game's chat, pastes the clipboard's text into the page's field and the docks'
  (main page and in-game menu), through the console's clipboard tools, at the caret,
  where typing goes. It is kept as the
  hub would take it ([sjk_chat_field.rs](../crates/sjk-viewer/src/sjk_chat_field.rs)):
  line breaks, tabs and other spaces become one space and a run of them one, characters
  outside the message alphabet (`chat_characters`: emoji, `{ }`, `\`, what prints
  nothing) are dropped, a `^` stays only before a digit, and the paste stops at 300
  characters (a colour code counts as its two and is never cut in half). Typed keys
  are not filtered, as before; the rules still refuse what was typed before sending.
  Ctrl+A selects the whole draft (typing or a paste then replaces it, Backspace clears
  it) and Ctrl+C copies it, on the page's field and the GIF picker's; Ctrl+X does
  nothing (no partial selection). The game's
  own chat composer (`messagemode`, SJK channel included) already had all four, with a
  caret and a selection ([chat/editing.rs](../crates/sjk-viewer/src/chat/editing.rs)).
- The caret (Rollembones' report, 22/10/2026): the page's and the docks' fields move it
  as the console does ([console_line_edit.rs](../crates/sjk-viewer/src/console_line_edit.rs)):
  Left and Right by a character, Ctrl+Left and Ctrl+Right by a word, Home and End to
  the ends; typing and pasting go in at it, Backspace and Delete (Ctrl: a word) take
  what is before or after it. The page draws it as `|` (blinking at the end, steady
  inside the text) and the docks as `_` at the end and `|` inside; a long draft shows
  its end, or the part from the caret on when the caret is further back. The GIF
  picker's search keeps its caret at the end.

## GIFs from GIPHY

A GIPHY link in an SJK chat message shows its GIF (Sol's decision, 10/10/2026: "GIPHY
links should just display the GIF in chat (not through the hub)"). The hub carries only
the text, as for any message; each reader's client fetches the GIF from GIPHY itself.
Only GIPHY: Tenor's API shut down on 30/06/2026.

- **Links recognised** ([link.rs](../crates/sjk-viewer/src/chat_gifs/link.rs)): `https://`
  (or `http://`) with the host compared whole and lower-cased, no user or port:
  `giphy.com/gifs/<slug>-<id>` and `giphy.com/gifs/<id>` (also `stickers/`, and
  `www.giphy.com`), `giphy.com/embed/<id>`, `media.giphy.com` and `media0` to
  `media4.giphy.com` `/media/<id>/<file>`, with or without the `v1.<token>` segment of
  GIPHY's share links (`/media/v1.Y2lk.../<id>/giphy.gif?cid=...`), and
  `i.giphy.com/<id>.gif` (`.webp`) or `i.giphy.com/media/[v1.<token>/]<id>/<file>`. The
  query and fragment are ignored; punctuation ending a sentence after the link is not
  part of it; short links (`gph.is`) are not followed. The id must be 6 to 40 ASCII
  letters and digits with a capital or a digit among them (a slug word such as
  `birthday` is not an id). A message shows at most one GIF, its first link's; that link
  reads `GIF` in the line, and other links stay text.
- **What is fetched for GIPHY** ([fetch.rs](../crates/sjk-viewer/src/chat_gifs/fetch.rs)): never
  the pasted address. SJK rebuilds `https://media.giphy.com/media/<id>/200.gif` (the
  rendition 200 pixels high GIPHY makes for every GIF; a typical one is 0.2 to 0.6 MB)
  and checks it again before the request; only when GIPHY answers 404 for it,
  `https://media.giphy.com/media/<id>/giphy.gif`, the original. For an id it does not
  know, GIPHY answers the original with 200 and a "not found" picture of its own, marked
  with an `x-retry-not-found-metric` header (seen 10/10/2026), which SJK takes as a 404.
  HTTPS only, no redirect followed, 10 seconds for the whole request, at most 8 MiB read
  (more is refused), `Accept: image/gif`, the user agent `SJK/<version>`. No cookie, no
  key, nothing about the player is sent.
- **Decoding** ([decode.rs](../crates/sjk-viewer/src/chat_gifs/decode.rs)), on the
  `sjk-chat-gifs` worker thread, never on the frame thread, with the `image` crate's GIF
  decoder (its `gif` feature): a canvas over 2048 pixels a side is refused; each frame is
  composed as the file says and scaled to fit 480 pixels a side; an animation over 240
  frames or 32 MiB of RGBA keeps its first frame only, still. Delays under 20 ms are
  shown as 100 ms, as browsers do.
- **The cache** ([chat_gifs.rs](../crates/sjk-viewer/src/chat_gifs.rs)): decoded GIFs
  stay in memory only, by id, so a GIF in several messages loads once: at most 48 GIFs
  and 128 MiB of pixels, those used least recently dropped first (never one on screen).
  Nothing is written to disk. A GIF that could not be had is asked for again after five
  minutes. Twenty-four GIFs can be drawn at once, each in a texture of its own made at its size
  ([gif_textures.rs](../crates/sjk-viewer/src/ui_renderer/gif_textures.rs)); once a frame
  each gets the frame of its animation, uploaded only when it changes, so a GIF on screen
  costs one small texture write per frame change and none otherwise. Every copy of a GIF
  plays in step.
- **How it shows** ([draw.rs](../crates/sjk-viewer/src/chat_gifs/draw.rs)): under the
  message's text, on the SJK chat page 160 pixels high (at 1080p), on the docks 48 and in
  the in-play chat box 54, as
  wide as its shape makes it (a column too narrow makes it narrower and lower). While it
  loads, a quiet box of that height says "Loading GIF", so nothing moves when it comes;
  when it could not be had, one quiet line says "GIF unavailable". The docks' box keeps
  its height: a message with a GIF takes the room of three one-row messages, and older
  ones make room; a message taller than the box with its GIF shows its text without it.
- **In the chat box during play** ([chat/feed_gif.rs](../crates/sjk-viewer/src/chat/feed_gif.rs),
  drawn by `chat/view.rs`, the one path that draws the chat box: in play, beside the
  scoreboard and over the history while the composer is open): an SJK line's link reads
  `GIF` and the GIF shows under its text, three of the box's rows high (88 pixels at 1080p
  and chat font size 1, 176 at 4K; it grows with `cg_chatBoxFontSize` as the rows do) and
  within its column. It takes the room of its rows, so later lines start under it, and it
  is drawn at its line's place and alpha: it slides in, scrolls and fades out with the
  line (`cg_chatBox`), and shows again with the history when the composer opens. The
  newest line, when too tall for the box with its GIF, shows its text without it. Only
  SJK lines: a game server's chat line keeps its GIPHY link as text and nothing is
  fetched for it. A line keeps its text both ways, so switching `cl_sjkChatGifs` changes
  the lines already in the box.
- **Muted players**: their GIFs are never fetched. The docks leave their lines out and
  the page shows "Muted on this PC" in place of the text, so their links are never read
  for a GIF (`chat_gifs::for_message`); the chat box shows "Messages hidden on this
  client" and never asks for a muted line's GIF.
- **Switch**: `cl_sjkChatGifs` (archived, default 1; Settings > Chat > SJK chat GIFs,
  under SJK chat, so in every settings screen and settings search). 0 keeps links
  as text and fetches nothing. With `cl_sjkChat 0` no message shows, so nothing is
  fetched either. Video clips have their own opt-in, `cl_sjkChatVideo` (default 0,
  under it: [Trusted media sources](#trusted-media-sources)).
- **The chat's rules** let GIPHY links through: `:`, `/`, `.`, `-`, `_`, `?`, `=`, `&`
  and `%` are in the bio's punctuation, so `chat::check` and `chat::for_display` keep
  them unchanged (a test sends each shape through both). A message is at most 300
  characters (150 before 16/10/2026), though: GIPHY's share links with all their tracking words
  (`/media/v1.<long token>/<id>/giphy.gif?cid=...&ep=...&rid=...&ct=g`) can be longer
  still and are refused (`chat_length`); the page's link (`giphy.com/gifs/<slug>-<id>`) or the
  media link without its query fits. Pasting (Ctrl+V) writes every GIPHY link as its
  shortest address, `https://giphy.com/gifs/<id>` (`link::shortened`), so a copied share
  link fits; a link typed by hand is sent as typed.

## Trusted media sources

Open **SJK chat > GIF > Sources** in any menu style. GIPHY stays trusted by default.
Other sites are opt-in: paste an HTTPS link or type a hostname, then press **Trust
source**. Selecting a message offers **Manage GIF source...**, which fills the same
field; selecting a host or message never approves it. Remove a source with **Remove**.
The list is local to this profile (`gif-sources.txt` beside `config.cfg`, one exact
host per line, at most 64). An empty file trusts none; deleting it restores the
GIPHY default next start. An unreadable existing file disables media sources.
`cl_sjkChatGifs 0` disables all inline media requests and playback, including search.

Untrusted links remain text. Each client fetches directly from sources it trusts;
the chat hub carries only the message. A source can see the client's IP address and
requested URL, including its query. Trust applies to the whole exact host, so any
content hosted there may play automatically. No adult providers, addresses or
provider-specific APIs are included. This is a content/privacy preference, not age
verification or a guarantee about the site's content or legal requirements.

Generic HTTPS links support direct GIF files, WebP pictures (animated or still) and
silent AVC/H.264 MP4 files (Windows only, below), plus
one HTML page advertising media through Open Graph (`og:video`, with `og:image` as a
fallback), a `video`/`source` element, or `link rel=image_src`. SJK never runs scripts
or uses browser sessions, cookies or credentials. Sites needing a private API,
login, JavaScript or another codec remain links/unavailable. Generic websites are
not searchable through the GIPHY search tab. Favourites preserve their original
HTTPS URL, including any query needed to fetch it; they do not grant trust.

Redirects and media CDN hosts need their own trust. Up to four requests are allowed
per resolution, each HTTPS-only, standard port, credential-free, with a ten-second
timeout and an 8 MiB response cap (HTML: 256 KiB). The cap was 4 MiB until
18/10/2026 (Sol's decision): the GIF a Tenor page names is often 4 to 5 MB, and the
memory is bounded by decoding's own limits either way. Discovered untrusted hosts
appear under **Blocked media hosts** in Sources without being contacted; select one
and press Trust source to approve it. The client log names such a host too ("its
media is on <host>, not trusted"). IP literals/local names are refused, and generic
requests filter the actual resolved addresses to public IPv4/native global IPv6.
Generic media does not use environment proxies, which could resolve private targets
elsewhere. No referrer or GIPHY API key is sent to generic sources.

Once a page has been read, its media is asked for without accepting HTML: a media
host can answer a browser-like request with its own viewer page instead of the file.
Tenor does (seen 18/10/2026): a `tenor.com/view/...` page names its GIF on
`media1.tenor.com`, which serves HTML to a request accepting it. A Tenor link needs
`tenor.com` and `media1.tenor.com` trusted, plus `media.tenor.com` for its MP4 clip
when video clips are on.

**Video clips are opt-in, off by default** (Sol's condition for using Media
Foundation, 18/10/2026): `cl_sjkChatVideo` (archived, default 0; Settings > Chat
(classic+: Network) > Play chat video clips (Windows), under SJK chat GIFs; the Sources
page says so too). Its help reads: "Off by default. MP4 clips from the internet are
read by Windows' decoder, outside SJK's safe Rust: a crafted clip could exploit it."
The reason: GIF and WebP are decoded by SJK's own memory-safe Rust, but an MP4 file,
which any trusted site can serve, is parsed by Windows' MPEG-4 source and H.264
decoder, native code outside SJK's control, so a flaw there could be exploited by a
crafted video. While it is off, MP4 behaves exactly as where Media Foundation is
missing (below), and Media Foundation is not even loaded. Switching it either way
takes effect without a restart: like removing a source, it starts a new policy
revision, so decoded media and queued work are dropped and new resolutions follow
the switch. On Linux the row stays off and cannot be switched on.

SJK ships no video codec (Sol's decision, 18/10/2026: no C in the project). The worker
decodes MP4 without playing audio through Windows Media Foundation
([sjk-mediafoundation](../crates/sjk-mediafoundation/src/lib.rs)): its MPEG-4 source
reads the clip from memory, its H.264 decoder and the source reader's video processor
give RGB32 pictures, which SJK turns into RGBA. COM and Media Foundation start once
per media worker thread and shut down with it. `mfplat.dll` and `mfreadwrite.dll` are
loaded at run time from System32, so the client also starts on a Windows N edition
without the Media Feature Pack. There, on Linux, and wherever Media Foundation does not
start or has no H.264 decoder, MP4 is unavailable for the run, said once in the log
(with `cl_sjkChatVideo 0` it is unavailable too, without a log line):
MP4 links stay text, a `video/` response is refused before its body is downloaded,
`video/mp4` leaves the Accept header and HTML discovery takes the page's `og:image`
instead of its video. Only ordinary (nonfragmented) AVC MP4 is supported; HEVC, AV1,
WebM, script players and streaming playlists are unsupported. Before Media Foundation
sees the file, a bounded walk of its boxes refuses fragments, external data references,
extended or zero box sizes, oversized sample tables and files without an AVC sample
entry; the reader's own picture size must not exceed 2048 pixels a side. Frames fit
480 pixels, with at most 240 frames, 32 MiB of retained pixels and five seconds of
decode work checked between samples. Long clips loop their retained beginning.

WebP is decoded in Rust (`image-webp`) into the same frames as GIFs, with the GIF
limits: a canvas over 2048 pixels a side is refused, frames fit 480 pixels, and an
animation over 240 frames or 32 MiB of pixels keeps its first frame, still. Delays
under 20 ms show as 100 ms.

Removing a source clears all cached media and changes the job revision: queued and
finished work from the old policy cannot appear. Already-started network traffic
cannot be recalled, but its result is discarded. Existing feed and dock links are
restored on the next UI update. GIPHY's existing fixed-host adapter remains the
only default provider; removing `giphy.com` disables that adapter and its search.

Implementation: [sources.rs](../crates/sjk-viewer/src/chat_gifs/sources.rs),
[discover.rs](../crates/sjk-viewer/src/chat_gifs/discover.rs),
[video.rs](../crates/sjk-viewer/src/chat_gifs/video.rs),
[decode.rs](../crates/sjk-viewer/src/chat_gifs/decode.rs) (GIF and WebP) and the shared
[SJK source editor](../crates/sjk-viewer/src/sjk_chat_gif_sources.rs).

## The GIF picker

Sol's request, 12/10/2026: "a gif picker to the SJK chat and a way to also save gifs as
favorites". Decided with Sol the same day: search uses GIPHY's API; favourites stay on
this PC. The key is the player's own (Sol, 12/10/2026: "make it they only use their own
gif key and we give them an easy popup with the link and easy steps"): SJK has none, as a
new GIPHY key allows 100 requests an hour, too few for every player to share.

- **Where**: the SJK chat page only. GIF, between the field and Send, opens it (and
  closes it); it takes the messages' place until it closes, so the field, Send and the
  right column stay. The docks have Open chat for it. With `cl_sjkChatGifs 0` GIF is
  off and the picker does not open.
- **Tabs**: Favourites (opened on when there are any, else Search), Search, and Seen in
  chat: the GIFs of the chat's messages, newest first, each once, at most 60, muted
  senders' left out. Seen in chat and Favourites ask GIPHY's API nothing.
- **Tiles**: five columns by three rows, each GIF whole and animated in its tile.
  A tile shows GIPHY's preview file, `200w_d.gif` (200 pixels wide at a lower frame rate,
  a third of `200.gif`'s bytes, seen 12/10/2026), falling back to `200.gif`, then the
  original, on a 404, with the same rules as a message's GIF (rebuilt address, 8 MiB,
  no redirect). Tiles are cached apart from messages' GIFs, in the same cache: it holds
  48 GIFs and 128 MiB of pixels now, 24 can be drawn at once, and three workers fetch, so
  a page of tiles fills together; a tile scrolled away before its turn is not fetched.
- **Picking** (a click, or Enter on a tile) writes the GIF's shortest link
  (`https://giphy.com/gifs/<id>`, 37 characters at most for a 14-letter id) at the end of
  the message, in place of any GIPHY link already in it, closes the picker and gives the
  field the keyboard: Enter sends, so text can go with it. When the link does not fit
  the 300 characters, the picker says so and nothing changes.
- **Favourites**: the star in a tile's corner (shown on the tile under the pointer or
  the keyboard, gold on a favourite), or F on a tile, adds or removes it; the chosen
  message has Add GIF to favourites (or Remove GIF favourite) under Mute when its line
  shows a GIF, with Hide this GIF beside it ([Hidden GIFs](#hidden-gifs)).
  Kept in `gif-favourites.txt` beside `config.cfg`: one id a line, newest first, at
  most 500 (adding past that lets the oldest go), written through a temporary file at
  every change. Nothing is sent anywhere
  ([favourites.rs](../crates/sjk-viewer/src/chat_gifs/favourites.rs)).
- **Search** ([search.rs](../crates/sjk-viewer/src/chat_gifs/search.rs)): the field
  (Search tab) asks `api.giphy.com/v1/gifs/search` for the words, or
  `/v1/gifs/trending` with none, 25 a page, `rating=pg-13`, from this PC: GIPHY's terms
  ask for requests from the client and forbid proxying them, so the hub is not used.
  The words are tidied (lower case, spaces made one, 50 characters at most) and asked
  700 ms after the last key, or at once with Enter. Only the GIFs' ids are read from the
  answer; nothing else in it is used. The next page is asked when the grid's last row
  comes on show (Page Down, the wheel, the arrows), up to 100 GIFs a search. One request
  at a time, HTTPS only, no redirect, 10 seconds, 1 MiB read at most. Answers are kept
  15 minutes and refusals 60 seconds (32 kept), so going back to a search asks nothing.
  "Powered by GIPHY" stands under the grid, as GIPHY's terms ask.
- **The key**: `cl_giphyKey` (archived, empty by default; Settings > Network > GIPHY
  key, under SJK chat GIFs), the player's own, free from GIPHY's developer dashboard.
  Search asks nothing without it; Favourites and Seen in chat need none. GIPHY gives a
  new key 100 requests an hour. A refused key (401, 403) and a spent hour (429) are said
  in the picker; the key travels in the request's address and is never logged. A new
  key forgets the refusals kept, so it is tried at once.
- **The key card**: opening Search without a key shows a card in the page's place (the
  page's other parts are not drawn under it, since text draws over every shape): why
  search needs the player's own key, three steps (open GIPHY's developer site and log in
  or sign up; Create an API Key, choose API, any name; copy the key, paste it with
  Ctrl+V, Save key), Open developers.giphy.com (`search::KEY_PAGE`, in the browser), the
  field the key is pasted into (Ctrl+A, Ctrl+C, Ctrl+V, a held Backspace), Save key and
  Not now. Save key takes 16 to 64 letters and digits (GIPHY's keys are 32), else the
  card says it doesn't look like a key; it sets `cl_giphyKey` and saves the profile, and
  the search goes. Not now (or Escape) puts the card off until the picker's Get a GIPHY
  key opens it again; a refused key shows Change GIPHY key, which opens it on the key
  saved. Keys: Ctrl+V paste, Enter save, Tab next, Esc not now.
- **Keys**: Tab walks the tabs, the search field, the tiles, the field, GIF and Send;
  Left and Right change tab on the tabs; the arrows move through the tiles, scrolling,
  and Up from the top row goes to the search field (or the tabs); Down from the search
  field or the tabs goes to the first tile; Up from the message field goes to the tiles;
  `/` goes to the search field; Page Up and Page Down scroll a page of rows and the wheel
  one row; Escape closes the picker (a second Escape leaves the page). The keys line
  says Enter pick, F favourite, PgUp/PgDn scroll, Esc close GIFs. The search field
  takes Ctrl+A (typing then replaces the words), Ctrl+V and a held Backspace as the
  message field does.

## Hidden GIFs

A player can hide a GIF they do not want to see (#ideas recap of 29/10/2026). The chosen
message on the SJK chat page has **Hide this GIF** beside its favourites button, the two
on one row as Staff's are (alone, as **Show this GIF again**, once hidden) for a message whose link is from a trusted source,
while GIFs are on and its sender is not muted. A hidden GIF never shows
(`chat_gifs::shows`): everywhere SJK chat shows GIFs (the SJK chat page, the menu docks,
the chat feed, and the picker's Favourites, Search and Seen in chat) its line reads
**GIF hidden** in place of the link (`link::HIDDEN_LABEL`), no picture is drawn, and it is
never fetched for a line. Lines already worded follow at once
(`chat_gifs::shown_revision`). Its favourites button is not offered while it is hidden.

The picker's **Hidden** tab (between Seen in chat and Sources) lists the hidden GIFs,
newest first, with their pictures (the one place they show, so a player can tell them
apart; `chat_gifs::show_hidden_preview`): a click on a tile, or Enter on it, chooses it
(a gold edge), and **Show this GIF again**, standing where Search's field does, shows
that GIF again, the others staying hidden. Its tiles have no star. With none hidden it says
how to hide one. `gif_unhideall` shows every hidden GIF again and says how many.

The list is kept on this PC in `gif-hidden.txt` beside `config.cfg`, in the favourites'
format (one id or approved-source link a line, newest first, at most 500, written
through a temporary file at every change); nothing is sent anywhere
([hidden.rs](../crates/sjk-viewer/src/chat_gifs/hidden.rs)).

## From a bind

Sol's request, 12/10/2026: "a cmd like say ... so people can make binds of premade
messages or gifs ... + a cmd we can bind to open the sjk chat big in one press while
in game". Three commands, for binds, quick wheel choices and the console, in a game or
in the menus ([console_sjk_chat_page.rs](../crates/sjk-viewer/src/console_sjk_chat_page.rs)):

- `sjksay <text>` says the text in the SJK chat, as the page's Send does (the hub's rules
  and limits apply). It is tidied as a paste is (spaces made one, characters the chat
  refuses left out, 300 characters at most). The console reads `//` as the start of a
  comment, so a link written in `sjksay` is cut: GIFs go with `sjkgif`.
- `sjkgif <which> [text]` sends a GIF, with the text before it if any: `which` is a
  favourite's number (1 the newest, as the GIF wheel numbers them), a GIF's id, or a
  quoted GIPHY link. `bind F5 "sjkgif 1 gg"` sends the newest favourite after "gg".
- `sjkchat` opens the SJK chat page, big, over the game or the menus, and closes it:
  Settings > Keys > Other > SJK chat page (unbound by default; I is still the game
  chat's SJK channel, `messagemode5`).

Quick wheel pages take `sjksay` and `sjkgif` choices like any command (Settings > Quick
wheel, a custom choice), and `+gifwheel` opens the
[GIF wheel](client.md#gif-wheel) on the favourites.

## Who is online

The SJK chat page (not the docks, which stay small) has a small window at the top of its
right column, over the chosen message (Sol's request, 10/10/2026: "a small window on the
SJK chat, only when it is big, to quickly see who is online/active, and the most recent
active players that are offline"). It is drawn by
[sjk_chat_panel_view.rs](../crates/sjk-viewer/src/sjk_chat_panel_view.rs) from what
[sjk_chat_people.rs](../crates/sjk-viewer/src/sjk_chat_people.rs) derives.

Players show as faces, not names (Sol's request, 10/10/2026: "a little circle with their
profile picture in it, maybe just that actually, and the letter if no picture, with
mouse hover to see the profile card"): a disc with the player's hub picture, from the
picture cache the cards use ([avatars.rs](../crates/sjk-viewer/src/avatars.rs)), or,
until it loads and for a player without one, the first letter of their name (colour
codes aside) on their own colour, as their card and profile draw it
(`profile_card::stand_in_colour`). Verified and staff marks are left to the card.

- **Online** (the heading carries the hub's count): the keys that read the feed in the
  last 60 seconds, the same keys `online` counts, so an SJK client with the chat on, in
  the menus or in a game, or one on a game server with the chat off. A gold dot at the
  foot of a face marks a player holding a live claim marked active (in a match,
  playing, as holocrons count it). Playing players come first, then by name (colours
  and symbols aside), each key once. Faces of 40 pixels at 1080p wrap nine to a row, two
  rows at most: with more than 18 players the last face says "+N" of the rest.
- **Recently active**: keys that read the feed before that, not online now, most
  recently seen first, up to eight smaller, dimmed faces in one row. No time is written
  on them: the card says it ("Seen 11 minutes ago", "Seen yesterday", as the servers'
  list words it); an online player's card says "Online now" or "Online now, in a match".
- Resting the pointer on a face shows the player's sender card with Mute or Unmute, as a
  name in the chat does, and rings the face in gold; the card opens left of the window,
  so the window's other faces do not show through it. A click chooses that player's
  newest message on show, if any. Muted players stay, their faces greyed. The player's
  own key is listed too.
- The window is as tall as what it shows, so Chosen message sits right under it.
- **A hub that does not list them** (the `people` below absent, as from the deployed hub
  today): the Online part keeps the count and says "This hub counts them but does not
  say who", and the second part becomes **Recently in chat**: the faces of the last
  eight senders, newest first, each once (holocron drops are not messages), their card
  saying when they last spoke. A message does not carry its sender's picture version,
  so the page asks, after the frame and outside the chat's lock, the profiles the
  client knows (the hub's players on the server, else a profile fetched once per key, as
  a sender card does) and keeps what it finds for 32 keys.
- Chat off, identity off or the hub not answered yet: the window says it shows once the
  hub answers.

The list rides on the feed the client already reads, so it needs no request of its own.
The client keeps the last list it was given (`ChatState::people`) until the hub gives
another, and forgets it when the chat is turned off or another hub is read.

### Proposed feed addition: `people`

Not in the hub yet; the client is built against it and falls back as above without it.

    "people":{"online":[Person],"recent":[Person]}
    Person: {"key_id":"..","name":"..","verified":false,"staff":false,"avatar":"<version>",
             "seen":<unix s>,"playing":false}

- Optional in a feed answer, so it stays `/v1/`. The hub adds it to a key's answer when
  it last gave that key the list 20 seconds ago or more (the first answer after `after`
  0 always has it), so the list costs one answer in a few and a reader's list is at most
  about 45 seconds old. It never wakes a waiting poll: it rides on the next answer.
- `online`: every key that read the feed in the last 60 seconds (as `online` counts),
  at most 50, `playing` set for a key holding a live claim with `active` true.
- `recent`: keys whose last feed read was more than 60 seconds and less than 7 days
  ago, most recent `seen` first, at most 8.
- `name`, `verified`, `staff` and `avatar` as in a Profile (the display name);
  `seen` is the key's last feed read. A key the operator removed is not listed.
- Nothing new is stored: the hub already keeps, in memory, when each key last read
  the feed for the count; it only needs to keep that past the minute (in memory, up to
  7 days, so a restart forgets it) and look up the profile fields.

## How a line looks

Every SJK chat line looks the same wherever it shows, in the game's chat, on the dock
and on the page ([sjk_chat_look.rs](../crates/sjk-viewer/src/sjk_chat_look.rs), Sol's
request, 08/10/2026):

- The message is drawn in the SJK chat's gold, `#F5C756` (`sjk_chat_look::GOLD`), a
  colour none of the game's codes give (`^0` to `^9`, in the game's palette or the SJK
  UI's lifted one; a test keeps it far from all of them), so it stands apart from
  every game chat line. The message's own colour codes are dropped
  (`sjk_chat_look::message_text`), so all of it is gold; the sender's name keeps its
  codes over the game's white.
- The sender's name emoji ([identity.md](identity.md#name-emoji), since 16/10/2026)
  stands right after the name, before the tick, on every line, the dock and the page
  (`sjk_chat_look::emoji_room`); a game line's sender the hub knows in that slot shows
  theirs too.
- In the game's chat a line starts with SJK's emblem, small (1.35 times the text's
  height, in its own colours, `ui_renderer::LOGO_TEXTURE`; Sol's request, 11/10/2026),
  where it used to read the letters SJK. A holocron drop or grand find starts with the
  holocron's or crystal's picture instead (01/11/2026,
  [holocrons.md](holocrons.md#chat-lines)). The dock and the page, which show only SJK
  chat, have no mark. A message sent from another client that takes part in SJK chat
  starts with that client's icon instead, at the same size: the clients are the
  entries of [chat/client_icons.rs](../crates/sjk-viewer/src/chat/client_icons.rs)
  (`KNOWN`, each with why and its evidence; today JoF EternalJK, `client` `ejk`,
  `ui_renderer::EJK_TEXTURE`, `assets/branding/jof-ejk-icon.png`, 21/10/2026, and the
  SJK Launcher's chat window, `client` `launcher`, `ui_renderer::LAUNCHER_TEXTURE`,
  `assets/branding/sjk-launcher-icon.png`, 27/10/2026: the launcher signs with the
  player's own `identity.key`, so its lines are the player's, marked with its icon).
  `cl_sjkChatClientIcons 0` (Settings, "Other clients' icons") gives every message
  SJK's emblem. The same icon marks that client's players on the scoreboard and the
  player card, once the client claims its slot (presence's `client`, 27/10/2026,
  [identity.md](identity.md#what-happens)).
- In the game's chat an SJK chat line is labelled after the name, where All chat lines
  have none and Team chat reads TEAM: SJK for a message only in SJK chat, ALL/SJK for
  one also said in the server's All chat ([Said on the game server too](#said-on-the-game-server-too);
  Sol's request, 22/10/2026). A holocron drop has no label.
- A verified sender has their tick after the name (the nameplates' seal, gold or their
  group's, staff's or a creator's: [Ticks](identity.md#ticks),
  `ui_renderer::mark_texture`) and no word: no "SJK VERIFIED" or "Verified" tag. Staff
  stays written on the page.
- The line flows as one line and wraps only when it is too long, as a game chat line
  does. In the game it used to break straight away: the feed drew a sender's name and
  its tag on a row of their own and the message under it (`NAME_ADVANCE` in
  `chat/view.rs`), while the servers' chat lines, which carry the name in their text,
  flowed. Now the first row holds SJK's emblem, the name, the tick and a colon, and the
  message goes on after them (`Wrapped::update_indented` in `chat/layout.rs`, drawn by
  `chat/view/sjk_line.rs`); a first word too wide for what is left of that row starts
  the next. The page and the docks lay their lines out the same way, measured in their
  body family (`sjk_chat_look::flow`, `flow_each` for the docks, which lay out every
  frame without allocating). A message's own colour codes are dropped, so its wrapped
  rows need no colour carried over; the name, which keeps its codes, stays on the
  first row.

### Game chat lines look the same

Since 16/10/2026 (Sol's request: the JoF tag, the SJK emblem and the verified tick in
the game's chat too, the two chats told apart only by colour), a game chat line whose
sender is known flows as an SJK line does, drawn by the same code
([chat/view/sjk_line.rs](../crates/sjk-viewer/src/chat/view/sjk_line.rs)): SJK's emblem when the hub holds a claim for the sender's slot under
the name the game shows there (the scoreboard badges' rule; for the player's own line,
once the hub answered them), the clan emblem for a known clan's tag
([identity.md](identity.md#clan-tags)), the name in its colour codes over white, the
verified tick for a verified sender, the channel (TEAM, WHISPER) small in its colour,
a colon, then the message. Only the message's colour differs: the SJK chat's gold, or
the game line's own colour codes over white (`^2` green on most servers). Before, the
name sat on a row of its own with the message under it, and JA+ lines, which the server
sends as text with no sender, showed no emblem at all.

- **Whose line**: the sender the server names, or for a line it does not (JA+, JoF), the
  one player whose name, as the game shows it, starts the line before `: ` (or inside
  `(...): ` and `[...]: `), colour codes as sent (`ChatOverlay::line_sender`,
  [chat/marks.rs](../crates/sjk-viewer/src/chat/marks.rs)). Two players wearing one name
  leave the line unattributed. An attributed line's name shows the sender card
  (the click menu it also opened was removed on 18/10/2026).
- **What each slot shows** (`chat::marks::SlotMark`): worked out from the hub's
  claims, the server's players and the player's `name` only when one of them changes
  (`ChatOverlay::follow_marks`, every frame from `identity_frame.rs`), so a frame costs a
  comparison. With the identity off nobody has SJK's emblem or a tick.
- **Names the server cut**: a game server carries names as bytes of the old code pages,
  and a JoF JA+ server drops what has none: `^0JoF J S^7olol 🦊` came back in the chat
  as `^0JoF J S^7olol ` (Sol's report, 16/10/2026; the server's own echo in the log).
  So an SJK player claims their slot with the name as they typed it (the `name`
  setting) when leaving out of it every character without a Windows-1252 or
  Windows-1251 byte gives the game's name, spaces at either end aside
  (`chat::marks::claimed`); otherwise with the game's name, as before. The claim still
  matches that slot everywhere (`names_match` keeps letters and digits only). Other SJK
  players' game chat then shows the name from the claim, under the same test
  (`chat::marks::restores`), so a claim adds back only what the server dropped and never
  makes a name another. Players on other clients, and SJK players with the identity off,
  see the server's name. The scoreboard, nameplates and cards still show the server's.

A holocron drop (`docs/holocrons.md`) is a line of its own kind in the same three
places: one sentence with no sender column, no colon and no tick, in its tier's colour
(`holocrons::TIERS`, none of the game's codes either), with a small gem where the tick
goes (`sjk_chat_look::gem_mark`): `Sol found a Legendary Holocron!`, and for the player's
own `You found an Uncommon Holocron.` The identity service's feed relays a drop as a
`ChatMessage` with an empty text and `holocron` set, in the feed's order; the game's chat,
the dock and the page word it from the tier (`holocrons::line`) and leave out a tier they
do not know. A muted player's drops are not shown.

## Clan chats

Since 03/11/2026 (Sol's decision, from Lex's idea): two chats beside SJK chat, JoF's
and the Sith clan's (`{{S}}`, shown as S), carried by the hub as SJK chat is, so they
last across maps and servers (and, like it, are lost when the hub restarts). The hub
decides who may use them (Sol-Vulpes/SJK-hub `PROTOCOL.md`, "Clan chats"), from the
group staff gave the player's verified tick ([Ticks](identity.md#ticks)), never from a
name or a clan tag:

| Who | May read and write | Joined by default |
| --- | --- | --- |
| a verified JoF player (tick group `jof`) | JoF only | JoF |
| a verified S player (tick group `sith`) | S only | S |
| staff (and Sol) | JoF and S | both |
| everyone else, gold ticks included | neither: SJK chat only, as before | none |

Until 05/11/2026 each clan could also join the other's chat; Sol closed that (hub
`clan-chat-own`): a JoF player never reads or writes S, an S player never JoF.

- **Joining** ([channels.rs](../crates/sjk-identity/src/channels.rs),
  [chat/tabs.rs](../crates/sjk-viewer/src/chat/tabs.rs)): every feed answer lists the
  clan chats the player may read and which are home (`channels`); Settings > Chat
  (and Network) "JoF clan chat" and "S clan chat" (`cl_sjkChatJof`, `cl_sjkChatS`:
  -1 Auto, the default, joins a home channel; 1 Joined; 0 Left) choose. The feed asks
  the hub for the joined ones (`channels=jof,sith`); joining or leaving reads the
  feed again from its backlog, under a new reading, so the joined chat's history is
  marked and not replayed. A player the hub lists none for sees no row and no button
  whatever the settings say. A client from before clan chats asks for none, so it
  never shows a clan line as SJK chat.
- **In the game**: a joined chat has its button in the composer's row after SJK
  (All, Team, SJK, JoF, S, Admin, PM; `cg_chatTabJof`, `cg_chatTabS`), its keys
  `messagemode7` and `messagemode8` (Controls > JoF clan chat, S clan chat, no key by
  default), and Tab stops on it. Enter sends it to the hub with its channel, never to
  the game server (no copy in a JoF server's All chat). Its lines flow as SJK chat's,
  emblem, name, tick, emoji, labelled `JoF` or `S` where SJK lines say `SJK`, the
  message in the chat's own colour: JoF's light cyan `#8FDDF5`, S's rose red `#FA8C9E`
  (`chat::view::clan_colour`; a test keeps both far from the game's colour codes, the
  SJK gold and the encrypted pink). They play the SJK chat's sound and go to the
  console with a `^5JoF` or `^1S` tag (`cl_sjkChatConsole`). `cl_sjkChatFeed 0` hides
  SJK chat's lines from the game's feed but not the clan chats', which show nowhere
  else; `cl_sjkChat 0` turns all three off.
- **Not on the docks or the SJK chat page**: those stay SJK chat, as they were; a clan
  chat reads only in the game's chat, so its messages cannot be reacted to from this
  client yet (the hub takes reactions on them from the players who may read them).
- **The rules that hold**: blocking a player hides their clan lines and sounds as their
  SJK chat ones (the same list); the hub's mute, the text rules, the quota (shared
  with SJK chat) and duplicates (per channel) hold. Chat bans hold too, but only ever
  hold players who are neither verified nor staff, so none of a clan chat's; the limits
  for unverified players and Ban 24 h have nobody to apply to there. Encrypted whispers
  and their verified-only start are whispers, not a channel, and stay as they are.
  Staff delete a clan line as any (the panel marks it JoF or S).

## Chat bans

Sol's request, 16/10/2026: "any verified person can ban someone that isn't verified
for 24h from the SJK chat, no reason needed, no cooldown on it. It should ban the key +
the IP". The hub does the banning (Sol-Vulpes/SJK-hub, `PROTOCOL.md`, "Chat bans"); the
client offers it and says what came of it.

- **Who may ban whom**: a player the hub vouches for (their profile says verified or
  staff) may ban a player it does not (neither verified nor staff), with a key, and not
  themselves. The hub checks all of it again; the client only offers it there
  (`chat_ban::offered`), and the worker refuses to send it from a key that is neither
  (`Worker::send_chat_ban`, `not_verified`).
- **What a ban does** (the hub's rule): 24 hours without writing in SJK chat for the
  person of that key, and for any key the hub does not vouch for writing from the
  address it last saw them use; a verified key from that address still writes. No reason
  is asked and there is no wait between bans; a later ban sets the end 24 hours from
  then. The banned player still reads the chat. Staff lift bans (Lift chat ban).
- **Where**: Ban 24 h beside Mute on the sender card (in the game's chat while the
  composer is open, on the main page's dock and the in-game menu's, on the SJK chat
  page) and beside Mute on this PC under the SJK chat page's chosen message. Staff have
  Lift chat ban beside Delete for everyone.
- **One click is not enough** ([chat_ban.rs](../crates/sjk-viewer/src/chat_ban.rs)):
  the first click arms the button, which reads Confirm ban, with "Click again: no SJK
  chat for them for 24 hours" under it in ember; only a second click on the same player
  within 4 seconds (`chat_ban::ARM`) sends it. This guards against a stray click; it is
  no wait between bans. While the request is on its way the button reads Banning...;
  then the line under it says what came of it: "Banned from SJK chat until 17/10 14:32"
  in gold, in local time, day first on the 24-hour clock, or the hub's refusal worded
  for players (`ban::refusal`): "Verified players and staff cannot be banned", "Only
  verified players can ban from SJK chat", "This SJK hub cannot ban from the chat yet"
  for a hub without the route. The state is one for the whole game, so every card shows
  the same button for a player.
- **The banned player**: their message is refused with `chat_banned`, and the refusal
  reads "A verified player banned you from SJK chat until 17/10 14:32"
  (`ban::banned_message`, from the refusal's `until`), where every refusal shows: an
  `SJK chat:` line in the game, the docks' line under the field, the page's notice.

## Limits for unverified players

Sol's request, 16/10/2026: "an unverified person can only send one message every 30s
and they can't send gifs or any links". A key the hub neither verifies nor made staff
(trusted ones too) sends one SJK chat message every 30 seconds and no message holding a
link: `://`, `www.` or a word that reads as a domain (`discord.gg/abc`; `chat::has_link`,
word for word the hub's). A GIF is a link in the message, so it is refused too. The
worker refuses both before anything goes (`Worker::send_chat`): "Players who are not
verified send one message every 30 seconds: wait 18 s", "Only verified players can send
links and GIFs in SJK chat"; the hub refuses them again (`chat_quota`, `chat_links`).
The composers clear the field on Send, so a refused message has to be typed again.

## Trust marks

Sol's request, 16/10/2026: an "in progress verified" mark, an empty tick for unknown
players that turns green after 30 days, which staff can upgrade to verified; a chat ban
resets the timer and shows a red mark for a day. Sol's answers to the design: only staff
may ban a green player, green players may not ban, only the banned player's timer
resets (not others on their address), and 10 hours of play are needed besides the 30
days. The hub works the level out (Sol-Vulpes/SJK-hub, `PROTOCOL.md`, "Trust") and
sends `trust` with profiles, presence, chat messages and the feed's people for a player
who is neither verified nor staff; the client draws it
(`ui_renderer::mark_texture`, [trust_badge.rs](../crates/sjk-viewer/src/ui_renderer/trust_badge.rs)).

- **New**: an empty light grey ring with a grey tick. Every key from the day it
  registers, and again after a chat ban ends.
- **Trusted**: a round green disc with a white tick, round where the verified seal is
  scalloped so the two never read alike. 30 days since registering or since the last
  chat ban that was not lifted, and 10 hours of active play in that time (as holocrons
  count it). Staff verify players by hand as before; green is a hint, not a step the hub
  takes by itself. Only staff may chat-ban a green player (`chat_ban::offered` hides
  Ban 24 h from the others; the hub refuses `target_trusted`), and a ban on another
  player's address does not hold them.
- **Banned**: a red disc with a white cross while a chat ban holds the player (24 hours).
  A ban that staff lift does not count: lifting it gives the time back.
- **Where**: after the name on SJK and game chat lines, on the docks and the chat page
  (lines, the chosen message), on sender cards, on nameplates, small on the foot of the
  scoreboard's SJK emblem, at the foot of the profile card's picture; the player card
  writes NEW, TRUSTED or CHAT BANNED where VERIFIED goes, the in-game menu's player rows
  say Trusted or Chat banned, and the Profile page's line under the name says where the
  player stands ("New: green after 30 days and 10 hours of play").
- A hub before trust sends none, and a level a later hub adds reads as none
  (`Trust::Unknown`): no mark, as before.

## Blocking a player

A player can be blocked on this PC (Sol's request, 08/10/2026, as "muting"; named
blocking and made to cover emotes, toys and player cards on 18/10/2026, Sol's request:
"never see their chat no more ... emotes blocked, everything"). It is local: nothing goes
to the hub or the game server, and the game is unchanged; only what this client shows
and plays changes. The code still says "mute" (`chat_mutes.rs`, `muted_players.rs`).

- **Where:** rest the pointer on a name and the player's sender card shows beside it
  ([sender_card.rs](../crates/sjk-viewer/src/sender_card.rs)), in the SJK UI's look
  wherever it is: in the game's chat while the composer is open (the pointer is free
  then), on an SJK chat sender's name or on the name of a player the server says sent
  the line ([chat/card.rs](../crates/sjk-viewer/src/chat/card.rs)); on the main page's
  dock; and on the SJK chat page. The card stays while the pointer moves onto it. It
  shows their picture (or their initial, as the profile card draws it,
  `profile_card::avatar`; the version comes from the hub's players on the server, else
  their profile, asked of the hub once, outside any lock:
  `player_identity::avatar_version`), the name and, for a verified player, the
  verified tick; whether the hub knows
  them (and staff); their hub name when it differs from the one shown; their key; their
  medals' medallions; where they are on the server being played ("On this server, slot
  5", or "slot 5, matched by name"); and Block or Unblock. In the game's chat a click on
  the name pins the card and, for a player on the server, adds Whisper above Block
  ([client.md](client.md)). The Players page's card could
  not be reused: it is that page's right column, with score and ping. The page's
  chosen message keeps its Block on this PC, which uses the same list. The console's
  `block <player>` blocks a player of the server being played (`target` for the one
  under the crosshair, a slot number or part of the name shown), `block` alone lists
  the blocked players, and `unblock <player|all>` lifts a block by a name of the list
  (away from the server too) or a player here ([block_command.rs](../crates/sjk-viewer/src/block_command.rs)).
- **The list on the SJK chat page:** while no message is chosen, the side column lists
  "Blocked on this PC (N)" under its hint, each with Unblock, as many as fit above the
  keys' line (six at most; a line says how many more `block` lists).
- **What blocking does:** their SJK chat lines are hidden (the page shows "Blocked on this
  PC", the dock leaves them out, the game's feed hides them as ignored lines are) and so
  are their game chat lines once they are matched to a slot, in the chat box and in the
  console, whose scrollback and alert words (`con_notifywords`) leave them out as the
  chat box does (`ChatOverlay::sent_by_muted`, after bleck-bee's and Kushina's hub report
  #191: the line was hidden in the chat box but showed in the console; the chat log
  `cl_logchat` still keeps it), and the console's kill lines name them BLOCKED as the
  kill feed does. Lines the server writes itself (a join, JA+'s "has defeated") come as
  finished text and keep the name the server put in them. While they are on the
  server, their model is drawn as `kyle/default` (Kyle's red or blue skin when a team
  game colours the teams, so the teams still show), each saber they hold or throw as
  `single_1` in the default blue (`color1` 4), with no hat or cape; a body they leave
  stays so. Their look ([unlockables.md](unlockables.md#receiving)) is not drawn
  either: no blade skin (so none of its sounds) and no lit Illuminate holocron
  (`Looks::set_muted`). Nothing whose source is their entity, or that the sound events name them as
  the cause of, is played: footsteps, jumps, pain, death, taunts and voice, weapon fire
  and charging, saber swings, hum, hits and blocks (`otherEntityNum2`), Force sounds,
  their voice commands, the hum and wall scrapes of their sabers, and the chat beep of
  their messages ([audio_mute.rs](../crates/sjk-viewer/src/audio_mute.rs),
  `LegacySoundDecision::cause`). A beep names the slot the server appends to the chat
  command; where it appends none (JA+ and JoF), the one player whose name starts the
  line, then `: ` (or a team line's or a whisper's brackets), as the feed attributes
  the line (sjk-client `local_sounds.rs` `chat_sender`, after blec-bee's hub report
  #147: the line was hidden but its beep played). A line two players' names fit names
  nobody, so its beep plays even when the feed hides it. Their missiles' flight and impact sounds and the
  sounds of effect files stay: the protocol does not say whose they are. The test is a
  bit mask of slots, so it costs nothing while nobody is muted.
- **Their name reads BLOCKED** (since 26/10/2026, Sol's request) wherever this client
  draws a player's name from the game: the scoreboard, the kill feed, nameplates, the
  crosshair's name and the followed player's (`muted_players::label`, the muted slots
  kept for them by `muted_players::set_shown`). A staff saber of theirs is drawn as the
  stock staff (`dual_1`), two sabers as two `single_1`, one as `single_1`
  (`HiltCatalog::is_staff`, from the saber's definition).
- **Everything else of theirs:** their emotes do not play (one playing when they are
  blocked ends; `emotes_frame.rs`, beside `sjkemote mute`'s emote-only list), their
  Order 66 orders and cardboard box spots are ignored (`order66.rs`,
  `cardboard_box.rs`), no player card shows when you aim at them and a card pinned
  before loses their hub profile (`hud_runtime.rs`), their world pings are not shown
  (`world_ping_frame.rs`), their toys and looks are not drawn (`looks.rs`), their GIFs
  are never fetched (`chat_gifs.rs`) and their holocron lines go with their chat. Their
  name reads BLOCKED (above), but for the server's own messages (`entered the game`,
  votes): that text is the game server's.
- **The list:** `chat-mutes.txt` beside `config.cfg`, one player a line: their SJK key
  (or `-` for a player the hub does not know) and the name they were last seen with
  ([chat_mutes.rs](../crates/sjk-viewer/src/chat_mutes.rs),
  [player_mutes.rs](../crates/sjk-viewer/src/player_mutes.rs)). It is read at start
  and written at every change, so mutes last across sessions (the page's mute used to
  last for the session only). At most 1024 players.
- **Matching a muted player to a slot**, as the scoreboard's badges do
  ([identity.md](identity.md#what-happens)): a slot whose claim at the hub was made
  under the name the game shows there is that key's. A slot no such claim covers is
  matched by name: the muted name and the shown one compared lower-cased, without
  colour codes and symbols (`sjk_identity::names_match`). That is weaker: anyone
  wearing the name is muted, and a muted player who renames is not; the card says
  "matched by name", and for a player the hub does not know, "By name, colours aside:
  anyone wearing it". A slot another key claims is never matched by name, and the
  player's own slot never is. A game chat line the server does not attribute (JA+
  servers) is its sender's when it starts with the name one player shows, then `: `
  (or a team line's or a whisper's brackets; [Game chat lines look the
  same](#game-chat-lines-look-the-same)), so it is hidden when that player is muted; a
  line two players' names fit stays unattributed and is hidden when it starts with a
  muted slot's name. The muted slots are worked out only when the
  list, the hub's claims or the server's players change
  ([muted_players.rs](../crates/sjk-viewer/src/muted_players.rs)), as a bit mask the
  renderer, the chat feed and the sound filter test.

## Blocking for this map

Since 26/10/2026 (Sol's request: "temporary, on the current map, synced between players
... only staff members can do the sync thing, everyone else it's personal"), a player can
be blocked until the server's map changes, as the block above does it (Kyle, silent, no
chat, encrypted whispers or emotes, named BLOCKED), with nothing written to `chat-mutes.txt`
([map_blocks.rs](../crates/sjk-viewer/src/map_blocks.rs)). It works for any player of
the server, an SJK player or not.

- **Who:** anyone blocks a player for themselves (`block <player> map`, or the sender
  card's Block for this map). SJK staff choose, on the card, between Block this map (for
  them alone) and Block for all, or type `block <player> everyone`: the hub keeps that
  block (`POST /v1/staff/map-block`, `PROTOCOL.md` "Map blocks") and gives it with
  presence to every SJK client on the server. The hub refuses it from a player not
  staff, from staff not on that server, on their own slot, and on a slot an SJK staff
  player claims (403 `staff_target`); it goes to the staff log as `map_block`.
- **Opting out:** `cl_staffBlocks` (Settings > Network > SJK chat, "Follow SJK staff
  blocks", on by default) 0 shows staff-blocked players as themselves; the player's own
  blocks stay. `unblock <player>` lifts a staff block for that player alone, until the
  blocked player leaves.
- **What a block names:** a slot and the name the game showed there, for one map load
  (`sv_serverid`, which every client of the server sees the same and which changes at
  every map load). A block follows that player through renames while they keep the slot
  and ends when the slot empties (they left: the next player there is not blocked) or the
  map load changes; a staff block also when staff lift it (Unblock all, `unblock
  <player> everyone`) or when the hub's list for the server is replaced by another map
  load's or forgotten (4 hours after its last change). A staff block applies on a client
  while the name in its slot matches the one staff saw (colours aside,
  `sjk_identity::names_match`), or the player that client saw match renamed since; so a
  client that joins after the rename does not follow it. The player's own slot is never
  blocked.
- **How it reaches the frame:** the muted slots (`muted_players.rs`) are the PC list's
  and the map's together (`MapBlocks::resolve`, worked out when the roster, the hub's
  presence or the map blocks change), so everything a block does follows from the same
  mask. A staff block shows within the presence poll (5 seconds) on the other clients,
  at once on the staff player's (the service polls presence again after the request).
- **Seen:** `block` alone lists the map's blocks after the PC's ("this map, by you", "by
  SJK staff, for everyone"); the sender card of a player on the server shows Block for
  this map (Unblock this map) and, to staff, Block this map and Block for all (Unblock
  map, Unblock all) above Block ([sender_card.rs](../crates/sjk-viewer/src/sender_card.rs),
  `MapBlockRow`).

## Muting a player's sounds

Since 03/11/2026 (akr's and Neffie's request) a player's sounds can be muted alone,
without the rest of a block: `mutesounds <player>` (`target` for the one under the
crosshair, a slot number or part of the name shown, as `block`) silences everything the
player causes, exactly what a block silences (`audio_mute.rs`: their footsteps, voice,
pain, taunts, weapon fire, saber swings, hum and hits, Force, a thrown saber, their
emotes' and toys' sounds and the beep of their chat), while their chat, model and name
stay as they are ([sound_mutes.rs](../crates/sjk-viewer/src/sound_mutes.rs)). It lasts
until the map changes or they leave, with the bookkeeping of a block for this map (it
follows them through renames); `mutesounds` alone lists them and
`unmutesounds <player|all>` lifts it. Nothing is sent to the server or the hub.

Settings > Chat > Ignore mutes sounds (`cl_ignoreMutesSounds`, off by default) ties it to
the servers' own ignore: an `ignore <player>` the player types on a JA+ or jaPRO server
(the server then hides that player's chat) also mutes their sounds, and `ignore` again,
which the servers toggle, lifts it. The client reads the commands it released to the
server (`sjk_client::sent_commands`), so the command still goes to the server unchanged;
the first look after a start skips what was sent before. On a server without `ignore`
the server answers that it does not know the command, and the sounds are muted all the
same.

What was there before: blocking (above) already silenced a player completely, but it also
hides their chat and draws them as Kyle; Settings > Sound had volumes for effects, music,
voices and emotes, toys' sounds by toy (Collection > Toys), chat sounds by kind,
footsteps, jetpack, blade skin and holstered-staff sounds, but nothing for one player's
sounds alone nor for a sound outside those groups, such as an item's or power-up's loop.
The second gap is `s_muteSounds` ([client.md](client.md#muted-sounds)).

## Emotes in the client

- `sjkemote <id>` plays an emote while on a server (named apart from the `emote`
  command some server mods have): it starts on the player at once and is sent to the
  hub unless its file says `local`; the hub's echo of it is not started again.
  `sjkemote` alone lists the catalogue.
- The catalogue is data: `emotes/<id>.emote` files in the game's file system (pk3s or
  the folder), each a JSON object: `name` (required, at most 32 characters), `torso`
  and `legs` animation names, `length_ms` (100 to 60000, required unless `loop` is
  true), `loop`, `sound`, `clip` (an animation of a [rig model](rig-models.md#emotes-the-rigs-own-clips),
  played on its own skeleton), `local` (played on this client only, never sent),
  `moving` (keeps playing while the player moves), `icon` (a PNG, JPG or TGA picture
  the quick wheel shows for it, drawn on a blue disc), `sound_ms` (the sound's length: a
  looping emote plays it again each time it ends), `sound_loop` and `sound_loop_ms`
  (music in two parts: the loop follows the sound and repeats while the emote plays) and `rig` (the rig model folder the clip comes from;
  other rig models with the same joint names borrow it). The ids `stop`, `sync`,
  `quiet`, `loud`, `mute`, `unmute` and `none` are reserved: `sjkemote stop` ends the
  player's emote here and, relayed, for the others; `quiet` and `loud` are the away
  emote's (below).
  Unknown fields and bad values are refused with the file named in the log. None ship
  with SJK; the rig model add-on writes them for a model's own animations.
- `help_emotes` (or `sjkemote help`; since 16/10/2026) shows an SJK pop-up card
  ([emote_help.rs](../crates/sjk-viewer/src/emote_help.rs)) listing the `sjkemote`
  commands and how to put emotes on the quick wheel: one command to give a player who
  asks. It opens the game menu under it on a map and closes it again with the card;
  `help` is a reserved id. World shot: `emote_help_card`.
- Other players' emotes, on this screen only (since 16/10/2026; nothing is sent):
  `sjkemote stop all` ends every other player's emote and `sjkemote stop <player>` one
  player's, once: their next emote plays. `sjkemote mute <player>` ends that player's
  emote and drops the ones the hub relays from their identity key (so it follows them
  to another slot or name) until `sjkemote unmute <player>` or `unmute all`, or the
  game closes (`emotes::State::muted`, not saved); `mute` alone lists them. Only SJK
  players can be muted, as only they send emotes. `<player>` is a slot number, part of
  the name shown (a whole name wins when several match), or `target`, the player under
  the crosshair.
- `sjkemote sync [player]` (since 15/10/2026; the quick wheel's Sync emote) joins
  another player's emote in step: the named player (a slot number or part of the name
  shown), else the one under the crosshair, else the nearest emoting player within 512
  units. The emote goes to the hub with the leader's slot (`sync` in `POST /v1/emote`,
  relayed in the feed). Every client, the joiner's included, gives the follower the
  start time the leader's emote has *on that client* (`ActiveEmotes::sync`), so the two
  are in phase on every screen whatever each player's delay to the hub. Joining a
  follower follows its leader. The follower's music starts at the very sample the
  leader's voice plays (`Cue::Start`'s `align`, the mixer's `play_aligned`; the clock's
  `offset` when the leader's voice is gone or more than half a second off), and it plays
  on if the leader stops first. The mixer mixes the group's identical voices once, each
  ear as loud as the loudest of them (`MixerConfig::protected_channel`), so a group
  sounds like one song from the dancers, not a louder echo. (In 2026.1015.3 the
  follower's music stayed silent; in 2026.1015.4 each copy started from the clock, a few
  milliseconds off, and the copies added up.) A client that does not see the leader playing it, or a
  hub or client from before the change, plays it from the start as a plain emote.
- A received emote counts only when the game shows, in its slot, the name its claim
  was made under (the badges' rule). It then plays in `emotes::ActiveEmotes`: per slot,
  the emote, when it started and when it ends (its length, 3 seconds for an id the
  catalogue lacks, never for a loop), and it ends early when the player moves 24 units
  from where they were first seen dancing, or is missing from the snapshots for 2
  seconds (a moment out of them, in a crowded dance, is not leaving; since 20/10/2026,
  hub report #107), unless it is `moving`. Whatever the emote, it ends when its player
  mounts a vehicle (`m_iVehicleNum` set in their entity, or the own player's
  playerState; `ActiveEmotes::end_riding`, since 04/11/2026, hub report #247): the
  vehicle plays the rider's animations, and a `moving` one such as `cloud_float` fought
  them for the whole ride. The own player's client then sends the hub a `stop`, a
  received emote for a rider is not started, and `sjkemote` refuses one while riding. With Settings > Sound > Emotes: Attack ends emote
  (`cg_emoteAttackStop`, off by default), the player's own emote also ends when they
  attack or alt attack (`ActiveEmotes::end_on_attack`, called for each command sent):
  the server performs the attack whatever the emote shows, so it stops here and the
  client sends the hub a `stop`, as `sjkemote stop` does, which ends it for the others.
  Off, the dance goes on through the attacks (the meme way; on 19/10/2026 this was
  briefly always on). With Settings > Sound > Emotes: Typing keeps emote
  (`cg_emoteTypingKeep`, on; since 23/10/2026, Rollembones' idea), a player typing keeps
  their emote: while their chat, console or a menu is open (their talk balloon,
  `EF_TALK`, which their client's `BUTTON_TALK` sets on the server, so every SJK client
  sees it without the hub; for the own player, the open field itself), moving does not
  end it (being pushed, knocked back or carried) and neither does an attack; once they
  close it, where they stand then is the place they may not leave, so the next move of
  24 units or attack ends it (`ActiveEmotes::hold`, `emotes::typing_slots`). Typing
  never moved a player (the server clears the movement of a command with
  `BUTTON_TALK`, `PmoveSingle`), so what it changes is what happens to them meanwhile.
  The rule is the reader's: a client with it off ends a typist's emote as before. An
  alt-tabbed player sends `BUTTON_TALK` too, so they keep theirs the same way. A rig model plays an emote's `clip`
  (`emotes::playing`), and so does a stock model when the emote names its `rig`
  ([Emotes on stock models](rig-models.md#emotes-on-stock-models)); the `sound` plays
  on the player (`emotes::Cue`, channel 13,
  following another player, in both ears for the player's own, at `s_emoteVolume`
  apart from `s_volume`, live while it plays, silent in a duel with
  `s_muteEmotesInDuel`)
  and stops when the emote is cut short; the `torso` and `legs` animations are not
  played yet, and a received emote is written to the console (`* Sol: Wave`). Demos
  keep the emotes started on them.
- Away emote (since 19/10/2026, [away_emote.rs](../crates/sjk-viewer/src/away_emote.rs)):
  the player is away while the game's window is alt-tabbed out or minimised (Settings >
  Sound > Emotes: Away when alt-tabbed, `cg_awayAltTab`, on) or after
  `cg_awayIdle` seconds (Away after, 180; 0 never) without a key, mouse button, wheel
  or mouse move (the input `holocrons::activity` notes), on a remote server and not
  spectating. Idle time does not count while the player's own looping emote plays (a
  dance left running on purpose, whose music going quiet stopped it for everyone; since
  20/10/2026, hub report #107), unless they are away already. Going away, an emote the player plays goes quiet for everyone: the client
  posts `quiet` and each client stops its music (`ActiveEmotes::set_quiet`) while it
  keeps time, so `loud`, posted on the way back, plays it again from where it would be:
  in a synced group, at the very sample the leader's music plays, or another
  follower's when the leader is the one back or away too (`ActiveEmotes::in_step_with`;
  since 29/10/2026, hub report #195: a leader back from alt-tab came in from the clock
  alone, a few milliseconds off the followers, and the group sounded doubled until the
  loop came round).
  Alt-tabbing out or minimising always does this to an emote playing, whatever
  `cg_awayAltTab` says (Sol's rule, 23/10/2026, `away_emote::alt_tab_quiets`): the
  setting only decides whether alt-tabbing with no emote playing starts the away emote
  below (and counts as away for toys). The window's focus reaches the others only this
  way, through `quiet` and `loud` on the emotes' hub path; nothing else shares it.
  With no emote playing, the pick starts: on a JA+ server (its `gamename`, JoF
  included) the JA+ emote of `japlus.awayEmote` (JA+ away emote: the sit, kneel,
  hips, wait, surrender and Neo ones, which hold the player until they crouch), sent to
  the server; on the way back, while it still holds the player (`PM_FREEZE`), the
  commands crouch until it lets go, for a second at most (`away_emote::shape_command`;
  sending it again does not end it). Else (off a JA+ server, or on one with
  `japlus.awayEmote none`) the looping SJK emote of `cg_awayEmote` (Away emote, any
  looping emote that is not `local`), posted with `quiet` so no client plays its
  music, and stopped (`stop`) on the way back unless it already ended. The hub takes
  one emote every 2 seconds from a key, so a post waits for its turn
  (`player_identity::emote_ready`); one still waiting when the player comes back was
  never seen, and neither it nor its undoing is sent. A hub from before 19/10/2026
  refuses `quiet` in a post: the emote goes again with its sound. The settings list
  the installed emotes as they are when the page shows (`ValueKind::Live`). A JA+ emote
  that already holds the player when they go away (one they chose, `amsit5` or
  `ammeditate`) is left alone: neither away emote goes over it and no crouch follows,
  which got them up (hub report #196, `Away::turn`'s `held`).
- The quick wheel's editor lists the installed emotes under Emotes. An emote's `icon`
  is decoded when the catalogue loads and put into the UI atlas's emote cells
  (`emote_icons.rs`, 128 of them): a wheel choice that runs `sjkemote <id>` wears it,
  on a dark disc, in the ring and in the editor; without one it keeps the plain
  emote icon. Every world the menu moves to has a renderer of its own with an empty
  atlas, so the pictures go into each new one (`ShapeRenderer::id`); until
  18/10/2026 they reached only the first, and vanished after a map change.
- The emote picker (`emotes`, `emote_picker.rs`) is an SJK pop-up card over the game
  menu, on a server only: every installed emote as a tile with its picture, name and
  id, sorted by name, 7 by 4 on screen and scrolled by the wheel, the arrows or Page
  Up/Down. The search field has the typing from the start (the `text_entry` keys) and
  keeps the emotes whose name or id holds every typed word. A click on a tile, or
  Enter on the chosen one, closes the card and the game menu and queues
  `sjkemote <id>`; Escape clears the search, then closes.

## Pings

World pings ([client.md](client.md#world-pings)) ride the emotes' path: the pinger's
client sends the point and its scope to the hub (`Service::ping`, through the identity
worker, so a ping never waits for the feed's poll), the hub relays it in the feed of the
readers of that game server, and the feed thread queues it with the server it was read
for (`Service::take_pings` hands over only the viewer's server's).

Why the hub and not the game server (decided 13/10/2026): a hidden chat or `tell` line
would show as text to every player on another client, and to SJK players with it off;
it goes through each server mod's chat flood limits (JA+ and JoF mute or kick a fast
talker) and the server's logs; and a team chat line reaches stock-client teammates as
garbage. The hub reaches only SJK clients, on any server (base, JA+, JoF), with no
server change. The cost is latency: a reader waiting in its long poll gets a ping in
the hub's round trip, but the feed starts polls at most every half second
([client service](#client-service)), so a ping that comes just after another answer
waits up to that long (2 seconds before 27/10/2026). Pings need `cl_identity 1` and a live claim on the server (a
few seconds after joining).

The hub knows no teams, so the team rule is the readers' (`world_ping::Scope::sees`):
a modified client could read every ping of its server. Only a server that tells the hub
(or relays pings itself, an SJK dedicated server) could enforce it; nothing does yet.

## Toy events

A toy event is something one SJK player's toy did to another (since 16/10/2026): the
first is the [cardboard box](client.md#cardboard-box)'s `spotted`, a box lifted with
`+use`. It rides the pings' path: the sender's client sends the toy, the event and the
target's slot (`Service::toy_event`, through the identity worker), the hub relays it in
the feed of that game server's readers (`toys`), and the feed thread queues it with the
server it was read for (`Service::take_toy_events`). A reader applies it only when the
game shows, in the sender's slot, the name the claim was made under, and ignores its
own echo. The hub checks neither that the target holds a claim nor that they wear the
box: the readers decide, so a modified client could lift anyone's box, which takes off
a toy and nothing else. Without the hub's change (before 16/10/2026) the event is
refused and only the lifter's screen hides the box, for at most 15 seconds.

The second (since 17/10/2026, no hub change) is [Order 66](client.md#order-66)'s order:
toy `order66`, event `execute_<moment>`, the game server time (milliseconds) at which
every player with the toy on dies, 3 seconds after it was given. It concerns everyone
with the toy on, not one player: the hub wants a target other than the sender, so the
client names another player who has the toy on (else the next slot), and the readers
ignore it. A reader takes an order only on time (at most 0.5 seconds late, at most 10
ahead) and 30 seconds or more after the last one it took on that server; the hub's own
quota (one toy event a second, ten a minute per key) still applies. The client takes
the hub's toy events once a frame and hands them to every toy (`identity_frame.rs`).

The Monitor's Screen Saver (since 29/10/2026, no hub change,
[client.md](client.md#monitor)) is told the same way: toy `monitor`, event
`saver_<until>` (the game server time it ends, at most 31 seconds ahead of the reader's)
when it rises, and `saver_off` if the player dies before, to the player whose monitor
raised it (any other slot would do; the readers ignore the target).

## Toy relay

The toy relay (since 21/10/2026, Sol-Vulpes/SJK-hub, `PROTOCOL.md`, "Toy relay")
carries toy events that must reach one player at once, many times a
second, which the feed's toy events (one a second, read by a poll every few seconds)
cannot: the [Hax](client.md#hax)'s hacker steering the hacked player, the
[Ragdoll](client.md#ragdoll)'s falls, told to every SJK player on the server, the
[Drone](client.md#drone)'s place, told to each in turn, the
[Boombox](client.md#boombox)'s track, told to each player within reach, and its box on
the floor, told to every SJK player (since 03/11/2026), and the
[Monitor](client.md#monitor)'s throws, told to every SJK player, the nearest first (its
hits ride the Ragdoll's falls), and the [Banana peel](client.md#banana-peel)'s peels, told
the same way, and their slips (since 29/10/2026), and the [Prop gun](client.md#prop-gun)'s
guns held and shots fired (since 01/11/2026), and the
[status balloons](client.md#status-balloons)' reasons, told to every SJK player when they
change (since 08/11/2026). Two threads of
the identity service of their own ([relay.rs](../crates/sjk-identity/src/relay.rs),
`sjk-hub-relay-send` and `sjk-hub-relay-read`), each with its own HTTP client, read the
hub and the game server as the feed does:

- **Sending:** `Service::relay_toy_event` queues an event for the player in a slot,
  `POST /v1/relay` with a toy event's body (the event up to 64 characters, `-` allowed);
  the sending thread posts it at once (3 second timeout). A queued event marked as
  replacing (a control frame) gives way to the next one of the same toy and target
  while it waits, so a slow way to the hub sends the newest frame, not a backlog. The hub
  takes 40 a second per key.
- **Reading:** while the viewer listens (`Service::set_relay_listen`: on a game server,
  since 23/10/2026; before, while the Hax's toy was on), the reading thread keeps `GET /v1/relay?server=..&after=..&wait=20` open: the hub
  answers as soon as an event for the reader's own claimed slot comes. The first read
  (`after=0`) only learns the last id; another server or hub starts afresh, a failure
  waits 0.5 seconds, doubled up to 2 (under the 3 seconds the hub keeps an event). The events go to a queue the viewer takes once a
  frame (`Service::take_relay_events`, once for every toy, `identity_frame.rs`), with
  the server they were read for.

The hub keeps an event 3 seconds, in memory, and gives it only to its target. As with
toy events, a reader applies one only when the game shows, in the sender's slot, the
name the claim was made under, never from a player blocked here, and decides itself
what it may do.

## Encrypted whispers

Whispers between two SJK players on the same game server go through the hub end-to-end
encrypted (since 22/10/2026, Sol's request: on by default, `cl_e2eWhispers`), so only
the two of them can read them: not the game server, its admins or logs, and not the hub.
The hub's side is Sol-Vulpes/SJK-hub, `PROTOCOL.md`, "Whispers".

**Decisions (Sol, 22/10/2026).** When the hub cannot take an encrypted whisper, a card
asks before sending it unencrypted through the game server (never silently). Encrypted
whispers show in their own soft pink, apart from the plain whisper's lilac, with their
own sound. What players are told is what it is: end-to-end encrypted through the SJK
hub, only the two of them can read it; nothing more.

**The cryptography** ([whisper.rs](../crates/sjk-identity/src/whisper.rs)). Each message
is sealed to the recipient's identity key (Ed25519, turned into X25519 by its Montgomery
form): a fresh X25519 key per message, HKDF-SHA256 of the shared secret (salt: the fresh
key and the recipient's key) for a ChaCha20-Poly1305 key used once, the sender's key as
associated data. Inside, the sender signs with their identity key the recipient's key,
the game server, the time and the text, so the hub cannot write a whisper in someone's
name, move one to another player or server, or replay an old one (a reader refuses one
more than 10 minutes from its clock). New dependencies: `chacha20poly1305`, `hkdf`,
`curve25519-dalek` (already under `ed25519-dalek`).

**Who takes them.** While `cl_e2eWhispers` and `cl_identity` are on and the player is on
a game server, the toy relay's reader (the [Toy relay](#toy-relay)'s
`sjk-hub-relay-read`) keeps its long poll open with `whispers=1`, whether or not a toy
listens. The hub then gives the reading key in the player's presence as `whisper_key`,
for 45 seconds after each read. Presence is read every 5 seconds (15 before 27/10/2026),
so a player is known to take them a few seconds to 5 after joining. A client that only speaks the SJK chat
(another client built on the hub's protocol) never asks, so it has no `whisper_key` and
whispers to it go through the game server as before, without the card. Asking with
`whispers=1` promises to open box format 1 (`whisper.rs`); a client that cannot must
not ask, or the whispers sealed to it are lost.

**Who may start one (Sol, 25/10/2026).** Encrypted whispers are for verified players,
so nobody can advertise to players where no game server sees it: only a player the hub
vouches for (verified or staff) starts one. A player who is not answers one, to its
sender only, for 30 minutes after that sender's last whisper to them; their own answers
do not keep the window open. The hub enforces it (403 `whisper_not_verified`; its window
lives in memory, so a restart closes it), and the client follows the same rule
(`player_identity::vouched`, the answer window `ANSWER_WINDOW`, a minute shorter so an
answer never races its end): a whisper it may not send encrypted goes through the game
server as a whisper always did, without the card, and the composer's "Not private" note
says so. A refusal the client did not foresee (the hub restarted) opens the card.

**Servers that moderate whispers (Sol, 25/10/2026).** JoF asked to keep moderating
whispers on their servers (against advertising there), so on the server families of
`e2e_whispers::PLAIN_FAMILIES` (JoF, told by `sjk_mod::known_servers`) the client neither
takes nor sends encrypted whispers: it does not read with `whispers=1`, so the hub gives
no `whisper_key` there, and every whisper goes through the game server. The player's
setting for the family turns them back on (`jof.e2eWhispers`, off by default, a JoF tools
setting that holds whatever the mod's switch, [mods.md](mods.md)). Both players need it
on, since a player who does not take them there has no key to whisper to.

**Sending** ([e2e_whispers.rs](../crates/sjk-viewer/src/e2e_whispers.rs)). Every whisper,
from the composer, `tell` or `reply`, reaches `send_chat_command` after its
substitutions and colours. When the recipient's slot has a claim under the name the game
shows there with a `whisper_key`, and the player may whisper them encrypted (above), the
text after `tell <slot>` is handed to the identity
service (`Service::send_whisper`), and the game server gets nothing. The relay's sending
thread seals it (`whisper::seal`) and posts `POST /v1/whisper` before any toy frame. The
hub refuses it when the recipient no longer reads with that key (409 `not_listening`),
over 2 a second or 30 a minute (429), and keeps it 60 seconds in memory for the
recipient's next read. When the hub took it, the feed shows "To <name>" with the
recipient's marks as the SJK chat shows a sender's (SJK's emblem, the verified tick or
trust mark, their name emoji), ENCRYPTED and the text in pink, the console `To <name>
(encrypted): <text>`, and the encrypted whisper's sound plays (Sol, 22/10/2026: on the
sender's side too, under the private messages' sound settings, `cg_privateChatBeep` and
`cl_chatSoundPrivate`, [client.md](client.md#chat-sounds)). When it did not
(or no answer came within 10 seconds), the [Encrypted whisper
card](sjk-ui.md#encrypted-whisper) opens over the game menu and asks; Send unencrypted
sends the same `tell` to the game server. A whisper to a verified player whose key
changed goes the same way, before anything is sent.

**Receiving.** The reader opens each whisper with the player's key (`whisper::open`) on
its own thread; the viewer takes them once a frame (`Service::take_whispers`) and shows
one only when the game shows, in the sender's slot, the name the claim was made under,
not from the player's own slot and never from a player blocked here. It goes to the
feed in pink with the sender's name and ENCRYPTED, to the console as `<name>
(encrypted): <text>`, plays the encrypted whisper's sound (under the private messages'
sound settings), flashes the taskbar while the window is away (`cl_privateChatFlash`) and
becomes the whisper `reply` answers. One that does not open is thrown away with a
console line saying why.

**Keys remembered.** The hub hands out the keys, so a hub that lied could read what is
sealed to its own key. Each PC remembers the keys it has seen per player
(`whisper-keys.txt` beside `config.cfg`, `key_id key` lines, at most 8 per player; one
player's keys are those of each of their PCs, [linked keys](identity.md)), and the
console says when a whisper goes to or comes from a key not seen before for a known
player.

**Safety codes (optional, Sol's decision 22/10/2026).** The first whisper to someone
still trusts the key the hub gave. A player who wants proof compares a safety code
with the other outside the game (voice, Discord, in person): 60 digits in twelve
groups of five (`whisper::safety_code`, format v2 since 24/10/2026), the same on both
screens, and different if the hub handed out a key of its own. The code is two halves,
the lower key's fingerprint then the higher's, each 30 digits (about 100 bits) of 5200
rounds of SHA-512 over that key alone, as Signal's safety numbers are made. Each half
belongs to one key, so a hub that wants both screens to match with keys of its own must
make a key whose fingerprint equals a real key's, all 30 digits. The first format (v1,
22/10/2026: twelve digits, SHA-256 of both keys together, about 40 bits) did not hold:
the hub chooses both of the keys it hands out, so it needed only two codes that meet,
about a million keys a side, which a laptop found in 22 seconds (24/10/2026). A player
on an older SJK shows the old twelve digits, which never match the new code; the keys
already marked verified stay verified (the file keeps the key, not the code). The chat's player
card (resting the pointer on a name with the composer open) has a Safety code button
for a player who takes encrypted whispers, and `whisper_verify <player>` opens the same
[card](sjk-ui.md#encrypted-whisper). Mark verified keeps their key in
`whisper-verified.txt` (`key_id key` lines, never sent). Their whispers then say
VERIFIED instead of ENCRYPTED. Once their key is no longer the one checked, nothing
goes encrypted to them without the card asking (it offers the game server, as for a
refused whisper), and a whisper from the new key comes with a red console warning to
compare the code again. Remove the check forgets it.

**The composer** says "To <name> (encrypted)" in the pink when the whisper will go
encrypted (`ChatOverlay::set_encrypted_slots`, from presence and the answer windows, when
they change). When the whisper is not encrypted (the recipient does not take encrypted
whispers, the player may not start one with them, `cl_e2eWhispers` is off, or a JoF
server without `jof.e2eWhispers`), a red note under the field says so for as long as the composer
is on it: "Not private: the server logs this whisper and its owner can read it" (Sol,
22/10/2026; the same red pill as SJK chat's note on JoF servers, `chat/view/composer.rs`).

**Limits.** Only on the same game server, both players on SJK with the identity on. The
hub sees who whispers to whom, when, and how long the box is. A whisper not read
within 60 seconds, or held when the hub restarts, is lost without the sender knowing
(the hub had taken it). Staff cannot read whispers unless a recipient reports one.

**Reporting one (Sol, 25/10/2026).** The chat's player card has Report for a player whose
last encrypted whisper to us came in the last 23 hours on this server (beside Safety
code when both show; `ChatOverlay::set_reportable_slots`). It opens the game menu's Report
page for them ([identity.md](identity.md#player-reports)), and the report carries that
whisper: its text and time and the signature its sender made inside the box
(`whisper::open_signed`), which the hub checks against the sender's key, the reporter's
key and the server. So staff read a whisper only when its recipient sends it, and know it
the sender's: nobody can make one up in another's name. Only the last whisper from each
player is kept, in memory, never written to disk.

## Announcements and reward rounds

25/10/2026, `personal/announcements` (Sol's decisions the same day: only the creator
announces for now; rewards are claimed with a Claim button; a round's public list shows
names and reasons, and what each person got stays private to them). The hub's side is
the hub's `announcements` branch (`PROTOCOL.md`, "Announcements", "Reward rounds"); the
wire is the contract between the two.

**Announcements** are official messages from the SJK team, shown in the SJK chat only,
never in a game server's chat.

- The identity service reads `GET /v1/announcements` (the newest 10 not removed, at most
  30 days old) at the registration and every 30 minutes, and the feed relays new ones as
  they are made (`FeedNews`, told to the worker once per feed id); `Snapshot::announcements`
  keeps them newest first. A hub without the route answers 404: the service treats it as
  none and asks again in an hour, and the player is told nothing.
- `announcements_seen.txt` beside `identity.key` keeps the key id and the highest
  announcement number shown ([official/seen.rs](../crates/sjk-viewer/src/official/seen.rs),
  as `holocrons_seen.txt`). Announcements above it, at most the newest three, go to the end
  of the SJK chat oldest first (`Service::post_announcements`); with no file for the key
  only the newest shows, so a fresh install is not flooded. They go in only once the hub
  being read has answered (its backlog first), and only with the chat on (`cl_sjkChat`); an
  announcement is counted as shown once the chat holds it. One the feed relayed is already
  in the chat and is not added twice. An announcement put in this way takes the id of the
  chat's newest message, so the feed's next message still follows it; the game's feed
  tells these lines by their announcement number and shows each once.
- The line, in all three places the SJK chat shows (the game's chat feed, the docked
  chat, the chat page; [official/words.rs](../crates/sjk-viewer/src/official/words.rs)):
  SJK's logo where a player's tick goes, "SJK" in the SJK UI's accent gold, then the
  title in bold and a colon, the text's first line (cut with "..." past 110 characters)
  and "Read more" when the text says more. The game's chat draws the logo before the name
  as for every SJK sender and cannot draw the title in bold or be clicked, so its line
  ends with "Read more: /announcement" instead. The line is kept with the chat's last
  200, is not a message (no sound, no sender card, nothing to mute or ban) and is never
  sent to a game server.
- "Read more": a click on the line in the dock or on the chat page (Up and Down on the
  page pass over it), or the console's `announcement [number]` (no number: the newest),
  opens the announcement on the News panel ([sjk-ui.md](sjk-ui.md#news-panel)).
  The command closes the console first; the card frees the cursor, in a match too
  (Sol's report, 25/10/2026: the console stayed under the card, and closing it closed
  the card too).

**Reward rounds** are the SJK team's contributor rewards, posted by the hub's operator.

- The service reads `GET /v1/rewards` at the registration, every 30 minutes, soon after
  the feed tells of a round, and when the round list opens (`Service::refresh_rewards`,
  at most every 30 seconds); `Snapshot::rewards` holds the player's pending and claimed
  rewards and the newest rounds.
- A round's line, for everyone: the "from the SJK team" gift box (SJK Symbols U+E028) in
  the accent gold where the tick goes, "SJK", then "Contributor rewards #1: 7 players
  rewarded. See the list" ("1 player rewarded"); the game's chat starts it with the gift
  box and ends it with ": /rewards". A click (or `rewards`) opens the round list.
- A reward waiting for the player opens the claim window as the medal pop-up opens: on
  the main menu, or when the game menu opens in a match, never over play; in a match an
  unlock card says "Contributor reward waiting" / "Open the game menu to claim it" once
  per reward. Claim calls `POST /v1/rewards/claim`; the hub's answer brings the profile
  (its new gift holocrons then arrive through the holocron pop-up) and the rewards, and
  the window shows the next reward or closes. A refusal says why in the window
  ("This reward was claimed already.", "This reward is not waiting for you any more.",
  else the hub's words). Later puts the window away until the menus close and open again.
- `debug_announce [news|release|long]` and `debug_reward [list]` rehearse the lines (in
  the game's chat) and the windows offline: nothing is sent, nothing is written to
  `announcements_seen.txt`, a rehearsed Claim only closes the reward.

**The SJK team** writes announcements in game (the creator alone until 31/10/2026, then
every staff member, Sol's decision): the `announce` window (Kind News or Release with its
version, Title, Text; a live preview of the chat line; Ctrl+Enter or Send, then a
confirmation) sends `POST /v1/staff/announce` (`POST /v1/creator/announce` on a hub from
before), checked first with the hub's rules (`news::check`: a title of 1 to 80 characters
on one line, a text of 1 to 2000 characters on at most 30 lines, the chat's alphabet
without `^`, the backtick allowed for chips, well-formed markup, a release's version like
`2026.1025.1`). Anyone else is told "Only the SJK team can announce", and the hub refuses
them too.

**News panel markup** (31/10/2026, [sjk-ui.md](sjk-ui.md#news-panel),
[news_markup.rs](../crates/sjk-identity/src/news_markup.rs)): an announcement's text may
hold links into the client's UI, `[Label](setting:<cvar>)` and `[Label](open:<page>)`,
and lines `[[image:<id>]]` and `[[switch:<cvar>]]`. Only announcements are read this way,
never chat messages, so only the SJK team makes links. The chat line shows a link's label
and leaves pictures and switches out (`news_markup::plain`). In the `announce` window's
Text: Ctrl+L makes the cvar just typed a link to its setting, named as Settings names it,
or with none opens a picker of the places `ui_links.rs` knows; Ctrl+K makes the cvar just
typed its switch, on a line of its own; Ctrl+I opens the system's file dialog (on a thread
of its own) for a PNG or JPEG of at most 3 MB, uploads it (`PUT /v1/staff/news-image`,
`Service::upload_news_image`, the outcome in `Snapshot::news_image`) and puts its line at
the end. The hub re-encodes pictures (no metadata kept) at most 1600 x 900, names each by
a hash of the file, and lists each announcement's pictures with their sizes (`images`);
clients read them unsigned at `GET /v1/news-image/<id>` and keep them in `news/` beside
the settings. On a hub before the panel the pictures and the staff route are missing:
the announcement goes the creator's way and a picture upload fails with the hub's words.

Checked (25/10/2026): unit tests for the wire types (the spec's examples), the rules,
the request bodies, the feed's merging and its news, the posting of read announcements,
the service's readings, claims and announcements against a fake hub, the seen file, the
line words, the windows' keys, and the round list. Not checked: anything against the real
hub, or in a game.

## Vitals

Shared vitals ([client.md](client.md#sharing-and-estimates), 15/10/2026) take the
same route as pings, for the same reasons (only SJK clients, any server, no server
change), on a thread of their own: `Service::start_with_threads` starts
`sjk-hub-vitals` ([vitals.rs](../crates/sjk-identity/src/vitals.rs)) with an HTTP client
that gives up after 5 seconds. It reads the hub and the server from what the identity
worker tells the feed (identity on, registered, on a server) and what to send from
`Service::set_sharing`, which the HUD sets every frame. Each `POST /v1/vitals` sends the
player's flags and, while they share and are alive, their values with the snapshot's
server time; the answer lists the server's sharers (`Service::with_vitals`). It posts
four times a second (twice before 27/10/2026) while the player shares or somebody listed does, every 2 seconds
otherwise (to notice a sharer), at once when a flag changes, and backs off (2 to 30
seconds) when the hub fails; a 403 (no live claim yet) waits 2 seconds. The viewer
uses an entry only while its claimed name matches the name the game shows in its slot
and the list is under 6 seconds old (`player_identity::shared_vitals`), and never its
own slot; it draws shared or estimated bars only while that list also has the player's
own slot sharing (`player_identity::sharing_live`). The hub keeps entries 5 seconds in
memory, and answers a post that does not share without the values (who shares only).

## Privacy

With `cl_identity`, `cl_hubUrl` and `cl_sjkChat` on, the hub sees the player's key
and IP address on every feed request (which it counts for `online`), and each message
or emote they send with the name they wear. With `cl_sjkChat` off it still sees the
feed requests made on a game server (for the looks), with the server's address. A ping sends the hub the point, its scope and,
through the claim, the player's slot and name on that server; it is kept 10 seconds in
the hub's memory and reaches the SJK players reading that server. Shared vitals
(`cg_nameplate`, See & share nameplates; `cg_nameplateShare` before 17/10/2026) send the hub the player's health,
shield and Force four times a second and whether they see everyone's bars; the hub keeps
them 5 seconds in memory and gives them to every SJK player on that server. Without
either setting, the client still asks the hub every 2 seconds, on a game server, who
shares there (with its key and address, as any request). Messages are public to every SJK player,
kept in the hub's memory only (the last 200), and gone when the hub restarts; a
message staff delete stays in the staff log. Emotes are kept 10 seconds. The mute
flag is the only new thing on the hub's disk. Local mutes stay on this PC
(`chat-mutes.txt`, keys and names) and are never sent. With the proposed `people` list ([Who is online](#who-is-online)), every SJK
player reading the chat would see which keys read the feed in the last minute,
whether they are in a match (not which server), and when the others last read it, up
to a week back; the list is not deployed yet.

GIFs ([GIFs from GIPHY](#gifs-from-giphy)): with `cl_sjkChatGifs` on, a message linking
to a GIPHY GIF makes this PC fetch it from GIPHY (Giphy, Inc., a third-party service
with its own privacy policy), so GIPHY sees the player's IP address, the GIF's id and
when, as for any picture a browser loads from it. The hub is not involved and learns
nothing of it. Nothing else is sent: no cookie, key or name. GIFs stay in memory only.
`cl_sjkChatGifs 0` stops every request to GIPHY.

The GIF picker ([The GIF picker](#the-gif-picker)): a search sends GIPHY the words typed
(after 700 ms without a key, or Enter), the player's own key and, as any
request, the player's IP address; opening the Search tab with nothing typed asks for
GIPHY's trending GIFs. No name, SJK key or cookie is sent. Favourites stay on this PC
(`gif-favourites.txt`, GIPHY ids or approved-source HTTPS links) and are never sent; Seen in chat and Favourites
ask GIPHY only for the GIFs themselves.

Encrypted whispers ([Encrypted whispers](#encrypted-whispers)): with `cl_e2eWhispers` on,
on a game server, the hub learns that the player takes them (their key, in presence)
and, for each whisper, the sender, the recipient's slot and key, when, and the sealed
box's length, never its text, unless the recipient reports it (then the text, its time
and the sender's signature, with the report). It keeps a whisper 60 seconds in memory. The keys seen
for each player stay on this PC (`whisper-keys.txt`, and the checked ones in
`whisper-verified.txt`) and are never sent.

## Verification

08/10/2026, Ubuntu 24.04, Rust 1.99:

- Hub: unit tests for the rules and the feed (ids, the 200 kept, quotas, duplicates,
  batches of 50, a restart's `after`, emotes per server and their 10 seconds,
  deletions, the online count) and API tests run in memory, including the long poll
  under tokio's paused clock (answered when a message comes, empty after 25 seconds),
  emotes needing a claim, IPv6 servers, the feed's and the chat's own allowances (four
  readers behind one address, 40 reads a minute per key), staff delete and
  mute with the log, the operator's API and commands. `cargo test` and `cargo clippy
  --all-targets` pass.
- Client: unit tests for the rules (identical to the hub's), `for_display` and
  `name_for_display`, the feed
  thread against a scripted hub (order, repeats, 200 kept, deletions, a new hub, a
  restarted hub, turning the chat off, the 2 second gap, back-off), the worker
  (rules before sending, emotes needing a server, the feed only when registered with
  the chat on), the overlay's channel, the dock (fits under four servers and above the
  version at 1080p, 4K, 21:9, 4:3, 5:4, 1024x768; keys, typing, pointer), the page
  (every focus fits at 1080p, 4K, 4:3 and 21:9 in the families and Inter; keys,
  scrolling, mutes, staff only for staff), emotes (catalogue files, lengths, moving,
  trust), and the key's default and migration.
- End to end against a hub built and run on this PC (`hub_e2e`): a reader waiting at
  the hub received a message about 2 seconds after it was said, refusals came back
  with the hub's codes, an emote needed a claim and reached the server's reader, and
  the service's feed thread read its own message and stopped when the chat was turned
  off.
- The dock and the page were rendered off screen over a plain backdrop in the UI's
  families and looked at.
- Blocking a player: unit tests for the list and its file, matching slots by claim and
  by name, the card on a name in the feed, the dock and the page, the hidden lines,
  Kyle and the default saber, and the sound filter ([status.md](status.md#muting-a-player-from-a-name-in-chat));
  the `block`/`unblock` name matching and the page's list with Unblock
  ([status.md](status.md#blocking-a-player)). Not seen or heard in a game.

- The chat's sound (10/10/2026, Windows 11): unit tests for its making (0.74 seconds
  from a long talk sound, the quickened length and pitch, the highs kept, the fade
  reaching silence at 0.60 seconds, the echo taps on an impulse at 70 and 140 ms, the
  level and the clamp), for when it plays (only messages newer than the feed had seen,
  not the backlog, not the player's own, a muted player's or a drop, once a second) and
  the settings row. Made from the retail `talk.mp3` it matched the sound Sol chose,
  sample for sample: correlation 0.9993, RMS 0.03321 against 0.03319, the
  difference's RMS 0.0012, peak 0.527 against 0.527. Not heard in the client.

- GIFs (10/10/2026, Windows 11): unit tests for the links (every shape, bad ids,
  look-alike hosts such as `giphy.com.evil.example`, a user or port, `http` fetched over
  `https`), the address rebuilt and checked, the small rendition first and the original
  only after a 404, the 4 MiB cap (8 MiB since 18/10/2026), decoding (frames, delays, the 20 to 100 ms clamp,
  scaling to 480, 240 frames and the byte budget falling back to the first frame), the
  cache (asked once, uploaded once per frame change, a failure not asked again at once,
  eviction by count and by bytes, slots for GIFs on screen), muted senders and the switch
  fetching nothing, the settings row, and the page and the docks fitting their GIF at
  1080p and 4K. One real GIF fetched from GIPHY by an ignored test
  (`chat_gifs::fetch::tests::a_real_gif_from_giphy`): `200.gif` of `3o7TKSjRrfIPjeiVyM`,
  582,333 bytes in about 0.26 s, 200 by 200, 43 frames, 3.5 s long; a made-up id gave
  404. World shots over duel6 with test GIFs made in the test
  (`world_shot::chat_gifs_shots`, the page and both docks at 1080p and 4K) were looked
  at. Not seen in the game window or with a GIF sent through the hub.

Not verified: no client was started, so nothing was seen in a game or over the map;
nothing went through Cloudflare's tunnel or the deployed hub (which does not have the
routes yet); Windows; how the feed behaves with many players at once.

- JoF EJK's icon and lines said on the game server too (21/10/2026, Windows 11): unit
  tests for the copy's end and how it reads back, its `say` pieces with the tag, the
  beep check, holding and dropping, the player's own copy with its id, refused and
  unanswered lines, a copy for a match that ended, and a JoF EJK message's icon. End
  to end with JoF EJK clients on a local server and a local hub, before it moved into
  JoF tools ([status.md](status.md#sjk-chat-on-jof-servers-said-in-all-chat-too)).

## Emoji reactions

On the SJK chat page, select a public message and use one of the six emoji
buttons in **Chosen message**. A highlighted button is your reaction: activate
it again to remove it. The buttons show shared counts, and message metadata
shows a compact count summary. Pointer and keyboard/Tab use the same controls.
Blocked senders, official notices and reward notifications offer no reaction
controls. Messages can still be read on older hubs; an unavailable endpoint
reports that reaction support needs an updated hub instead of posting chat text.
Reaction errors show for five seconds, then the status line returns to the reader
count (or the current connection status); a later failure starts a new timeout.

The catalogue is `like` (thumbs up), `heart`, `laugh`, `wow`, `sad`, `fire`.
`POST /v1/chat/reaction` is signed and takes
`{"message":<id>,"emoji":"heart","active":true}`. Setting an existing state
again is a no-op. The hub uses the authenticated canonical person id; callers
cannot select another player. Removing a reaction changes only that person's
matching pair. The endpoint enforces chat mutes and applicable bans, a 30-change
per-minute person quota, and at most 128 person/emoji pairs per message.

A normal message optionally carries `reactions:[{"key_id":"..","emoji":"heart"}]`.
The feed optionally carries `reaction_updates`, complete replacement snapshots
`{"id":<feed sequence>,"message":<chat id>,"reactions":[..]}`; an empty list clears
that message's reactions. Unknown emoji ids, invalid person ids and duplicates
are dropped, with a bound of 128 pairs. State is shared between cloned display
snapshots; drawing counts and labels allocates no extra reaction lists or strings.
Fresh backlog reads refresh reactions on already-known messages, and an update
for an absent or deleted message cannot recreate it. Reactions are memory-only
and disappear with chat history on a hub restart.

Reactions use the SJK hub only. They are never appended to message text, sent as
fake messages, forwarded to a game server or charged against the ordinary message
quota. Sending a reaction has its own result, so a failure never offers Retry
for an unrelated message. No JKA protocol-26 fields or codecs change.

The [matching hub PR](https://github.com/Sol-Vulpes/SJK-hub-shared/pull/5) must be
merged and deployed before shared reactions work on the live service. This client
PR does not deploy it. Old clients ignore the added JSON fields. Live hub rollout
and released-client end-to-end behavior remain unverified; source/unit/API and
windowless UI checks cover the behavior reported in the PR description.
