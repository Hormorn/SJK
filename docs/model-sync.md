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
- **Names from the hosts' own packs** (the Sith cloud's curated PK3s) are linked,
  never overwritten: a model already there needs no upload, and an upload under its
  name is refused, so two PK3s never fight over the same files.
- **Download:** everyone. Nothing downloads unless the player asks.

Changes go live at once, with no review queue: the tick is the trust.

## Upload

1. Profile > Share my model (the Profile screen of the SJK UI, for JoF and Sith
   verified players), or `sharemodel` in the console. The client finds the model the player wears, wherever it is
   installed (a folder, or a large PK3 with other things in it), and packs only
   `models/players/<name>/` and that model's `shaders/<name>.shader` into a new PK3
   ([`pack`](../crates/sjk-model-sync/src/pack.rs)).
2. The client checks the pack itself (the same checks as the server), so the player
   learns what is wrong before sending.
3. The client asks the hub for an **upload ticket**: a short signed note (key, tick
   group, model name, content MD5, ten minutes valid), signed with the hub's ed25519
   key. The hub never sees the file.
4. The client sends the PK3 and the ticket straight to the Sith cloud. The server
   checks the signature against the hub's public key (its config), checks the pack
   again, and publishes it. If the same content (same MD5) is already there, it only
   links it and nothing is stored.

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

## Viewing

- `cl_autoModels` (0, Settings > Mods, "Shared player models"): download, by themselves, the
  synced models of players you meet, and their new versions at the Sith cloud's
  hourly check. Off by default.
- Off: inspecting a player (the Inspect key pins their card) whose model is missing
  says once in the console how to get it; `getmodel <name>` fetches it.
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
  PK3s already hold (those are taken from the Sith cloud card, never uploaded).
- `PUT /models/<name>`, header `Authorization: SJK-Ticket <ticket>`, the PK3 as the
  body (at most `Limits::DEFAULT.pk3_bytes`). Answers `{"version":<n>,"linked":<bool>}`,
  `linked` when the current version already had this content. Refused: 401
  `bad_ticket` (signature, time, act, name or MD5 not matching), 400 `rejected`
  with `"why"` (the [`Rejection`](../crates/sjk-model-sync/src/lib.rs) text), 409
  `host_pack`, 413 `too_large`, 429 `rate_limited` (5 uploads a day a key, one an hour
  a name). Errors are `{"error":"<code>"}`.
- `GET /models/<name>/<n>.pk3`: version `n`, `Range` as the repository's files.
- `POST /models/<name>/rollback`, a rollback ticket: makes version `version` current
  again; answers `{"version":<n>}`.

The host also has `sith-cloud versions <name>` and `sith-cloud rollback <name> <n>`.

### Client

A download is checked again (`sjk_model_sync::check`) and saved as
`sjk-model-<name>.pk3` in `base` (or `assets/S/base`); `GameData/sjk-model-sync.json`
records what it fetched (name, version, MD5), so only those files are ever replaced or
removed. A model the game already has by name, from anywhere else, is never fetched.

## Verification

`cargo test -p sjk-model-sync --features client`: allowed and refused packs (paths,
types, the shader's names, images, GLM, limits, zip bombs), the packer's output
passing the checks, a download checked against its listing, and the record replacing
only its own file. `sjk-identity`'s `model_tickets_come_back_with_a_serial` covers the
ticket request; `model_sync::tests` in the viewer the setting's default and the files
a share takes. Not verified: an upload and a download against a running Sith cloud
and hub, and the Profile button in game.

Known limits: a model's textures outside its own folder, or shaders in a file not
named `shaders/<name>.shader`, are not shared (viewers see those parts untextured or
plain); sounds are not shared; a download loads from the next map.
