# Sith cloud

The Sith cloud is the S community's own file server: their maps, skins and an
optional pack of unofficial SJK extras (emotes, skins and sounds). SJK chooses and
downloads from it on the same card as [JoF content](jof-content.md), with its own
settings (`sith.*`). Sol asked for it on 28/10/2026; the SJK Launcher's Sith cloud
tab reads the same server.

**The hosts' files, their responsibility.** The Sith cloud's hosts choose and publish
the files; SJK only downloads what a player picks. The card says so under its title.

**Off by default.** Nothing downloads until the player turns on `sith.cloud` and
presses Download on the card (`sith.downloads`, Settings > General or Advanced > Mods >
Sith cloud), or also turns on automatic updates. Optional groups (SJK Extras) are never
chosen by default.

## Where the files go

Straight into the game's `base`, where the SJK Launcher puts them, so the game,
EternalJK and SJK all load them; SJK reads `base` at every mount, so a finished
download loads from the next map. The job's root is `GameData` itself, so its record
of what it fetched, `GameData/sjk-jof-content.json`, is the one the launcher keeps:
both know which files came from the Sith cloud, and only those are ever removed
(a pick taken back, or a file the server no longer lists). Files a player put in
`base` by hand are never touched.

The SJK Launcher puts new files in `GameData/assets/S/base` by default, for a Sol JK
that reads the [assets folder](client.md) (its Settings can turn that off). SJK's card
follows: while that folder exists its files go there too, the record is the one beside
it (`assets/S/sjk-jof-content.json`), and a file already in `base` with the server's
MD5 is used as it is, never fetched twice. EternalJK does not read the assets folder.

## The server

The server (`sith-cloud`, kept in the SJK Launcher's repository) serves a folder of PK3s over HTTP: `/manifest.json` in JoF's manifest
format (groups from the folder's sub-folders, each file's size and MD5) and the files
under `/repository/<folder>/<name>`. [sjk-jof-content](../crates/sjk-jof-content/src/lib.rs)
reads it as it reads JoF's repository, so the checks, resumable downloads and the
plan are the same.

SJK holds no address for it: the SJK hub names the server (`GET /v1/content-sources`,
asked before each list, from `cl_hubUrl`), so SJK can move it or switch it off for
everyone (the hub's empty list) without a release. Without a hub the card says it cannot
ask where the server is. `sith.cloudUrl` sets another server by hand and skips the hub.

## Automatic updates

As JoF content's, by the same code ([jof-content.md](jof-content.md#automatic-updates)):
with `sith.cloud` and `sith.cloudAuto` on, the client checks 30 seconds after the start
and every hour, also during a match, fetches what changed in the groups marked Auto
(`sith.cloudAutoGroups`) and removes only the files it downloaded for an Auto group
that the server no longer lists. A run that changed something shows "Sith cloud
updated" in a small card; the files load from the next map. A failure (the hub says
the cloud is off, the server is down, a PK3 the running retail game holds in `base`)
is a log line, tried again the next hour.

The group each file was downloaded for is kept in the shared record
(`GameData/sjk-jof-content.json`, `group`). An SJK Launcher built on an older
`sjk-jof-content` that rewrites the record drops that note, so those files are then
left for the card's Download to clean, never removed by an automatic run.

## Settings

| Setting | Default | What it does |
|---|---|---|
| `sith.cloud` | 0 | Allow downloads from the Sith cloud |
| `sith.cloudGroups` | `default` | Groups kept: `default` (the server's recommended ones), `none`, or group ids |
| `sith.cloudFiles` | empty | Files picked one by one against their group (`+<id>`, `-<id>`), written by the card |
| `sith.cloudAuto` | 0 | Update automatically: at start and every hour (needs `sith.cloud`); Settings > General or Mods > Sith cloud auto-update |
| `sith.cloudAutoGroups` | `all` | Groups updated automatically: `all` (those you take files from), ids, or empty for none; the card's Auto ticks write it |
| `sith.cloudUrl` | empty | Another server's address; empty uses the one the SJK hub names |

The card is [jof_content_card.rs](../crates/sjk-viewer/src/jof_content_card.rs) with
the Sith cloud's words ([sith_cloud.rs](../crates/sjk-viewer/src/sith_cloud.rs)); like
JoF content it is built with the `mod-jof` feature, which brings `sjk-jof-content`.

## Verification

The tests listed in [jof-content.md](jof-content.md#verification) cover both
collections (the same code); `sith_cloud::tests` checks the settings' defaults,
automatic updates off. The card renders off screen with `SJK_CONTENT_CLOUD=sith`. Not
verified in game: an automatic run against the live Sith cloud.
