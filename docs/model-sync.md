# Model sync

Verified players share the player model they wear through the
[Sith cloud](sith-cloud.md), so others can see it without hunting for the file. Sol
asked for it on 11/11/2026, with Krovos (who runs the Sith cloud) on board.

**Status:** implemented, not verified in game or against a live Sith cloud. The
client's side is this repository's (the checks and the cloud calls in
[sjk-model-sync](../crates/sjk-model-sync/src/lib.rs), the client in
[model_sync.rs](../crates/sjk-viewer/src/model_sync.rs)); the hub's ticket and the
Sith cloud's `/models` are built in their own (private) repositories and go live when
they are deployed.

## Who can do what

- **Upload or update:** players with a JoF or Sith tick ([identity](identity.md)).
  Jawa, gold-only and unverified players cannot. Nobody owns a name: any of them may
  push a new version of an uploaded model. Abuse is punished afterwards (hub ban),
  which every version's record makes possible.
- **Names from the hosts' own packs** (the Sith cloud's curated PK3s) can be updated
  too (Sol, 11/11/2026): a verified player may upload a newer version of a model one of
  those packs holds. The pack itself is never changed; the synced version is saved as
  `zzz_sjk-model-<name>.pk3`, which loads after the hosts' `zz_` packs, so it is the one
  the game uses.
- **Download:** everyone. Nothing downloads unless the player asks.

Changes go live at once, with no review queue: the tick is the trust.

## Upload

1. Share or Update on the [Model sync page](#the-model-sync-page) (for JoF and Sith
   verified players), or `sharemodel [<name>]` in the console (the worn model without a
   name). The client finds the model wherever it is installed (a folder, or a large
   PK3 with other things in it), and packs only `models/players/<name>/` and that
   model's `shaders/<name>.shader` into a new PK3
   ([`pack`](../crates/sjk-model-sync/src/pack.rs)).
2. The client checks the pack itself (the same checks as the server), so the player
   learns what is wrong before sending.
3. The client asks the hub for an **upload ticket**: a short signed note (key, tick
   group, model name, content MD5, ten minutes valid), signed with the hub's ed25519
   key. The hub never sees the file.
4. The client sends the PK3 and the ticket straight to the Sith cloud. The server
   checks the signature against the hub's public key (its config), checks the pack
   again, and publishes it as the name's next version, also over a name the hosts'
   packs hold. If the same content (same MD5) is already there, it only links it and
   nothing is stored.

## What a pack may contain

The checks live in one crate, `sjk-model-sync`, used by the client before sending,
the Sith cloud before publishing, and the client again before loading a download, so
viewers are protected even if the server is wrong. One rule fails the whole pack.

- **One model per pack**, `models/players/<name>/...`, the name `a-z 0-9 _ -`, up to
  32 characters. Paths are lowercase, `/`-separated, with no `..`, no absolute path,
  no duplicate (in any case).
- **File types:** `.glm` (parsed by [sjk-model](../crates/sjk-model/src/lib.rs), so
  a broken or oversized mesh is refused), `.skin` (plain text, `surface,path` lines),
  `.tga`, `.jpg`, `.png` (the header is read: real image, size capped), and at most
  one `shaders/<name>.shader`, whose every shader name starts with
  `models/players/<name>/`, so it cannot redefine a saber, world or UI shader.
- **Nothing else:** no `.cfg`, `.menu`, `.qvm`, `.dll`, `.bsp`, `.sab`, sounds or
  other folders. Sounds may come later with their own checks.
- **Limits** (`Limits::DEFAULT`):

| Limit | Value |
|---|---|
| PK3 size | 25 MB |
| Unpacked size | 64 MB, and each entry at most 40 times its packed size |
| Files | 128 |
| Texture side | 4096 pixels |
| All textures unpacked (RGBA) | 256 MB |
| GLM vertices, all LODs | 200 000 |
| `.skin` and `.shader` text | 64 KB each |

The Sith cloud also limits uploads: 5 a day per player, one update an hour per name.

## Versions and staff

The Sith cloud keeps every version of a name with who sent it (the player's key, from
the ticket) and when. Staff can roll a name back to an earlier version in one press;
players who take updates get the old one back at their next check. The bad version's
sender is then dealt with through the hub (ban).

## The Model sync page

`modelsync` in the console, Settings > Mods > Model sync, or the Profile page's Share
my model opens it ([model_sync_page.rs](../crates/sjk-viewer/src/model_sync_page.rs),
drawn in the SJK UI's look in every menu style). It asks the Sith cloud for its list
when it opens and on Refresh, and lists the models this game has that are not the
game's own, then those shared on the cloud it lacks, the worn one first (marked
"Worn"), with a search over the names. Each row says where the model stands ("On the
cloud: v3", "In the Sith cloud's packs", "Not shared", "Downloaded v2") and offers:

- **Share** (a model nobody shared) or **Update** (one on the cloud, or in the hosts'
  packs): only to players with a JoF or Sith tick, or staff. It runs
  `sharemodel <name>`.
- **Download**: a version on the cloud this game lacks (not installed, an older one
  fetched, or only the hosts' pack's). It runs `getmodel <name>`.

The page's foot shows the last lines model sync said (the same as the console's) and
"Working..." while a job is on its way, and whether `cl_autoModels` is on. Keys: type
to search, Up/Down to choose a row, Tab, Left and Right between the buttons, Enter to
press, Esc to close; the mouse clicks and the wheel scrolls.

## Viewing

- `cl_autoModels` (0, Settings > Mods, "Shared player models"): download, by themselves, the
  synced models of players you meet, and their new versions at the Sith cloud's
  hourly check. Off by default.
- Off: inspecting a player (the Inspect key pins their card) whose model is missing
  says once in the console how to get it; `getmodel <name>` or the Model sync page's
  Download fetches it.
- On, a met player's model the game already has is fetched only when it is a name of
  the hosts' packs with a shared version (the newer one wins over the pack).
- `modelrollback <name> <version>` (staff) asks the hub for a rollback ticket and
  makes that version current again.
- A download loads from the next map, as Sith cloud files do, and goes where they go
  (`base`, or `assets/S/base`).

## Wire

### Hub: upload ticket

`POST /v1/model-sync/ticket`, signed, `{"model":"<name>","md5":"<content md5>"}`
(the name as [`valid_name`](../crates/sjk-model-sync/src/lib.rs), the 32 hex digits of
`Checked::content_md5`). Answers `{"ticket":"<ticket>","expires":<unix seconds>}`.
Refused: 403 `not_registered`, 403 `not_allowed` (no JoF or Sith tick), 400
`bad_body` / `bad_model` / `bad_md5`, 429 `rate_limited` (10 tickets a day a person).

Staff: `POST /v1/staff/model-sync/rollback`, signed, `{"model":"<name>","version":<n>}`,
answers the same with a rollback ticket (staff log `model_rollback`).

A ticket is `<payload>.<signature>`, both base64url without padding: the payload is
JSON `{"v":1,"act":"upload"|"rollback","key":"<key id>","tick":"jof"|"sith"|"staff",
"model":"<name>","md5":"<content md5>"|null,"version":<n>|null,"exp":<unix seconds>}`
(ten minutes after it was made), the signature Ed25519 over the payload's bytes with
the hub's model sync key. The Sith cloud holds that key's public half.

### Sith cloud: `/models`

Its address is the Sith cloud's `files` address (from `GET /v1/content-sources`)
without its last `/repository`.

- `GET /models/index.json`: `{"models":{"<name>":{"version":<n>,"size":<bytes>,
  "md5":"<file md5>","content_md5":"..","at":<unix>}},"packs":{"<name>":{"group":"..",
  "file":".."}}}`: the uploaded models' current versions, and the names the hosts' own
  PK3s already hold (the Sith cloud card has those packs; an upload under such a name is
  a newer version over it).
- `PUT /models/<name>`, header `Authorization: SJK-Ticket <ticket>`, the PK3 as the
  body (at most `Limits::DEFAULT.pk3_bytes`). Answers `{"version":<n>,"linked":<bool>}`,
  `linked` when the current version already had this content. Refused: 401
  `bad_ticket` (signature, time, act, name or MD5 not matching), 400 `rejected`
  with `"why"` (the [`Rejection`](../crates/sjk-model-sync/src/lib.rs) text), 413
  `too_large`, 429 `rate_limited` (5 uploads a day a key, one an hour
  a name). Errors are `{"error":"<code>"}`.
- `GET /models/<name>/<n>.pk3`: version `n`, `Range` as the repository's files.
- `POST /models/<name>/rollback`, a rollback ticket: makes version `version` current
  again; answers `{"version":<n>}`.

The host also has `sith-cloud versions <name>` and `sith-cloud rollback <name> <n>`.

### Client

A download is checked again (`sjk_model_sync::check`) and saved as
`zzz_sjk-model-<name>.pk3` in `base` (or `assets/S/base`), after the hosts' `zz_`
packs, so a newer version of one of their models wins; `GameData/sjk-model-sync.json`
records what it fetched (name, version, MD5), so only those files are ever replaced or
removed. `getmodel` fetches what is asked; automatic downloads skip a model the game already
has, unless it is a hosts' pack name with a shared version.

## Verification

`cargo test -p sjk-model-sync --features client`: allowed and refused packs (paths,
types, the shader's names, images, GLM, limits, zip bombs), the packer's output
passing the checks, a download checked against its listing, and the record replacing
only its own file. `sjk-identity`'s `model_tickets_come_back_with_a_serial` covers the
ticket request; `model_sync::tests` in the viewer the setting's default and the files
a share takes; `model_sync_page::tests` the page's buttons for each state, its status
words, the search, keys and clicks, and that it fits windows from 1280x720 to 4K;
`model_sync_page_shot` (ignored, GPU and game data) renders it. Not verified: an
upload and a download against a running Sith cloud and hub, and the page in game.

Known limits: a model's textures outside its own folder, or shaders in a file not
named `shaders/<name>.shader`, are not shared (viewers see those parts untextured or
plain); sounds are not shared; a download loads from the next map.
