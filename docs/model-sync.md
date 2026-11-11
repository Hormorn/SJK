# Model sync

Verified players share the player model they wear through the
[Sith cloud](sith-cloud.md), so others can see it without hunting for the file. Sol
asked for it on 11/11/2026, with Krovos (who runs the Sith cloud) on board.

**Status:** planned. Only the shared checks
([sjk-model-sync](../crates/sjk-model-sync/src/lib.rs)) are implemented; the upload
ticket, the server endpoint and the client's pages are not.

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

1. Profile > My model. The client finds the model the player wears, wherever it is
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

- `cl_autoModels` (0, Settings > Mods > Sith cloud): download, by themselves, the
  synced models of players you meet, and their new versions at the Sith cloud's
  hourly check. Off by default.
- Off: a player's Inspect card offers **Download this model** when their model is
  missing; `/getmodel <name>` does the same from the console.
- A download loads from the next map, as Sith cloud files do, and goes where they go
  (`base`, or `assets/S/base`).

## Still to build

1. The hub's upload ticket (`POST /v1/model-sync/ticket`, JoF and Sith ticks only).
2. The Sith cloud's upload, version and rollback endpoints (`sith-cloud`, the SJK
   Launcher's repository), and the hub's public key in its config. Krovos deploys it.
3. The client: Profile > My model, the Inspect button, `/getmodel`, `cl_autoModels`,
   the check before loading.

## Verification

`cargo test -p sjk-model-sync`: allowed and refused packs (paths, types, the shader's
names, images, GLM, limits, zip bombs) and the packer's output passing the checks.
