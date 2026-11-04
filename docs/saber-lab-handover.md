# Saber Lab handover

Written 03/11/2026 for Sol and his assistant, from Milamber's branch `feat/saber-lab`.
Milamber is handing the Saber Lab over for Sol to finish. This page covers:

- what is built;
- what is planned but not built;
- what must happen before it can merge;
- Milamber's own blade, which travels with the branch.

The behaviour itself is documented where it belongs:

- [client.md, Saber Lab](client.md#saber-lab): the lab, its tab, the editor and the builder;
- [unlockables.md, blade-skin files](unlockables.md#blade-skin-files): the format and its new sections;
- [status.md, Saber Lab](status.md#saber-lab): what was checked and how.

Delete this page once its items are done. Fold whatever is still true into those three pages.

## For an assistant picking this up

1. Read [AGENTS.md](../AGENTS.md), then the three pages above (client.md's section is long but complete).
2. The code is one commit on `main` (this page and the blade are a second): the local branch `wip/saber-lab`, 49 commits from 18/10 to 01/11/2026, squashed.
   - Two of those commits are left out. One paused AGENTS.md's testing rules on the local branch. The other was a 16 MiB main stack, which main's own 64 MiB fix made unneeded.
   - Milamber still has the full history locally.
3. Run it:
   1. Build the client.
   2. Copy [assets/saber-lab](../assets/saber-lab) to a folder outside the repository. The lab writes into the folder it opens (the editor's saves and `originals/`).
   3. Start with `SJK_SABER_LAB=<that folder>`, or type `saberlab <that folder>` in the console.
   4. Open Collection > Saber Lab, the last tab. It opens on the builder; `B` switches to the stage controls.
4. From 19/10/2026 the branch was built in a fast-iteration mode, at Milamber's request:
   - Effects were tuned by eye in the client.
   - No unit tests were written for new work, and the workspace checks were not run until this handover.
   - The test debt is listed under [Before merging](#before-merging).

## Milamber's blade: "Saber Lab: tuned mythic"

[assets/saber-lab/skins/blades/](../assets/saber-lab/skins/blades) holds three files:

- `saber_lab_tuned_mythic.bladeskin`
- `saber_lab_tuned_mythic.bladeextra`
- `saber_lab_tuned_mythic.bladeextra6`

**This is the saber Milamber intends for themselves.** It was tuned by eye, one effect at a time, over two weeks. Keep its look as it is: change its numbers only with Milamber.

**Shipping it.** How it ships is Sol's decision. The natural fit is a [Unique](unlockables.md#unique-shaders) blade granted to Milamber, like Thousand Cranes and Hearth. Until then:

- It exists only as a lab blade. A folder's `saber_lab_<name>.bladeskin` is the lab blade "Saber Lab: <name>" (`saber_lab::EXTRA_PREFIX`).
- It is worn on the lab's PC only. The hub never hears of a lab skin, so other players see the wearer's usual blade.

To ship it, it needs:

- a catalogue entry (id, number, name, tier, holder);
- its files in the hub's blade-skin packs;
- the blade-cost pass every new blade gets (below).

**What it is.** A red chroma blade (`"chroma": {"effects": true}`): it takes the player's saber colour, and its particles turn with it. It wears:

- **The blade file:** the core, corona, granulation, flares, shimmer, tongues, sputter, arcs and motes, a red trail and a flickering red light.
- **`.bladeextra`:** a heat haze.
- **`.bladeextra6`:**
  - ribbon wisps at ignition;
  - smoke;
  - ember sparks, which burst as it lights, as it goes out and on a clash;
  - red bolts;
  - unrest that reaches up a swing and climbs on a clash (arcs only);
  - a crackling flash that lights the room, surges at ignition, sputters out and sends a heat ring up the blade as it lights;
  - a crackle sound (buzz and snaps from retail sounds);
  - disintegration on a saber kill;
  - one Part, "Crackle" (flash, sparks, crackle), that the wearer can switch off.

**Tried and rejected.** Milamber tried these on this blade and turned them down. Don't suggest them again:

- `front` with `veins` (a white-hot surge climbing the blade);
- `pulse` (a heartbeat in the glow);
- `ghosts` (afterimages while swinging).

The motes are kept but are a cut candidate: Milamber was unsure.

**To fix before it ships.** Its `sounds` block names placeholder paths (`sound/test/blade/*.wav`) that no pack has. They came from the first test blade. Skin sounds add to the stock ones, so nothing breaks, but these should become real sounds or be removed.

## What is built

Each is detailed in [client.md](client.md#saber-lab) and [unlockables.md](unlockables.md#blade-skin-files).

- **The lab** (`saber_lab.rs`):
  - A folder of loose blade-skin files mounted over the hub's packs, and remounted when its stamp changes (it is looked at every half second).
  - A file's mistake is named on the Shaders tab, and the last good look is kept.
  - Lab skins count as owned on this PC only.
- **The Saber Lab tab** (`collection_saber_lab.rs`, `menu_stage/lab.rs`):
  - The player's model holds the blade and swings it in a chosen style and move, at a chosen speed. A copy floats beside it.
  - The camera can orbit the model or look from behind in third person.
  - The draw and put-away flourish.
  - Sounds: ignition, swings, the hum and the skin's crackle, with a mute.
  - Stage trails, and the stage's blades lighting the scene.
  - A Clash button (and `C`), and a preview of any blade.
- **The editor** (`blade_editor.rs`, `collection_saber_lab_editor.rs`):
  - Every section and field of a skin's seven files (the skin file and six additions files) as rows, with ranges taken from the format's own checks.
  - Changes show at once and are saved 0.7 s after the last one, in the files' own order (`ordered_json.rs`), with undo and redo.
- **New blade-skin sections and fields** (`blade_skin_file.rs`, `blade_skin_smoke.rs`, `saber.wgsl`):
  - **The eighth set, `.bladeextra6`:**
    - `smoke`: puffs of the game's particles (`saber_smoke.rs`);
    - `ribbon`: wispy strands (`saber_ribbons.rs`, `.wgsl`);
    - `sparks`: bursts, aimed along the blade (`saber_sparks.rs`);
    - `crackle`: a buzz loop and snaps per bolt burst;
    - `disintegrate`.
  - **`flash`:** `light`, `ignite`/`surge`/`grow`, `die` and `burst`.
  - **`unrest`:** `swing`, `haze` and `clash`/`clash_time`, read from Creyon's `BladeMotion`.
  - **`chroma`**, set by file.
  - **Parts:** every section can be one of a skin's Parts (the eighth set's are `Section` 32 to 39; the `options` mask is a `u64`).
- **In matches:**
  - A skin's crackle is heard (`saber_smoke::Heard`, `GameAudio::hear_sabers`).
  - A saber kill by a skin with `disintegrate` burns the body away for every SJK client (`disintegration.rs`, `actor_pose::track_disintegration`, driven by the kill feed; NPCs excluded).
  - The server's body copy is burnt before it shows.
- **The builder** (`blade_builder.rs`, `collection_saber_lab_builder.rs`). The tab's first view:
  - On the left, a library of 49 effect cards on six tabs.
  - In the middle, the blade's groups: the wearer's Parts, at most 6 groups of up to 4 effects. Drag effects in, out and between groups, or click a card.
  - Each change is checked as the game reads the files, and refused with the check's words when it would not read.
  - A chip opens that section's fields in place.
  - Undo and redo.
- **Revert** (01/11/2026):
  - Before the editor first writes a skin, it copies the skin's files into `<lab>/originals/<id>/`, and never overwrites that copy.
  - Revert puts them back as an undoable change.

## Not built yet

These are Milamber's builder plan, with the decisions already taken.

**Decided** (26/10/2026):

- Groups are the wearer's Parts, and the builder enforces the format's 6 × 4.
- The tab opens on the builder.
- A signature's own shader look is a locked card, outside the general system.

Planned:

1. **Start a blade:** from a blank blade (the plain shader, its effects in a basic group), or as a copy of any skin, to see how it is built.
   - Copying a signature skin needs code. `blade_signatures::of(id)` matches by skin id, so a lab copy loses the shader and its header's `@palette`/`@trail`/`@extras`.
   - A lab file would need a field naming the signature it wears.
   - `Signature::options` builds the switches from the header and overrides the file's `options`, so a copy would need its own.
   - Coverage, counted 26/10/2026: 168 skins, of which 35 are fully editable. Those are the plain files minus Clone Wars (`saber_comic`), which a hand-ported design draws.
   - Proposal: list signatures in the copy dialog, labelled "N of M editable".
2. **Hover previews:** the effect under the pointer shown on the stage's blade.
3. **Presets and new effect types:**
   - Presets are named, tuned copies of a section, kept in the lab folder.
   - New effect types are added in code over time. Each becomes a card in `blade_builder::EFFECTS`.
4. **Scrolling** when the groups overflow the middle column, and a **search** in the library.
5. **Known gap:** the lab stage's hum and buzz (`menu_stage/lab.rs`, `skins.crackle`) ignore a crackle Part switched off.

Ideas Milamber raised that were not started:

- unlocking a skin's card-gated extras in the lab;
- a different skin for the second side in the lab (it dresses both sides alike).

Milamber has a design canvas of the builder: six boards, the last one "How to use it now". It is at https://claude.ai/artifact/L8e5ctyNcHQQAurZttBWQB; ask Milamber to share it. Its board 5 is wrong: the Sun has six switches, not four.

## Before merging

These are owed by the rules in [AGENTS.md](../AGENTS.md):

1. **The startup stack check.** It has not been run on this branch:
   `cargo test --release -p sjk-viewer --bin sjk the_client_starts_in_a_quarter_of_the_main_stack -- --ignored`
   - On 01/11/2026, still linked with 8 MiB, this branch overflowed the main stack building the menu's map (with the lab off too), where its base did not. 10 MiB was enough.
   - It is likely to fail the quarter-stack check until whatever the branch adds to that path is found and cut. A guess, unmeasured: the stage and Collection state it grew, held by value in large frames.
2. **Blade cost.** The branch changes `saber.wgsl`, `saber_gpu.rs` and the `blade_skin_*` sections, so run `python scripts/blade_cost.py` on `main` and on the branch in one sitting, and put both tables in the pull request. It has not been run.
3. **Tests owed.** These were written without unit tests (fast-iteration mode):
   - the eighth set's later fields: `unrest.swing`/`haze`/`clash`, `flash.light`/`ignite`/`surge`/`grow`/`die`/`burst`, `sparks.clash`, `bolts.clash`;
   - `crackle`, in the lab and in matches;
   - `chroma` by file;
   - sections as Parts;
   - the builder: `blade_builder.rs`, `Editor::add_section`/`remove_section`/`set_options`/`write_options`;
   - the kept originals and Revert.

   Disintegration has the world shot `saber_kill_disintegrates`. It needs a local `sjk-server`, a bot, `JKA_DEDICATED`, `SJK_TEST_PACKS` and `JKA_GAME_DATA`.
4. **Not yet seen in a real match:**
   - the crackle heard in a match;
   - the fix that burns the server's body copy before it shows. Milamber had not confirmed it yet.
5. **Remove the temporary tracing.** The code marked `TRACE-DISINTEGRATE` (`SJK_TRACE_DISINTEGRATE`) is in `disintegration.rs` and `actor_world_submission.rs`.
6. **Decide on `SJK_APPEAR_OFFLINE`** ([hub-chat.md](hub-chat.md)). The branch added it so a tester's chat stays offline while they test. Keep it or drop it.
7. **Defaults** (Sol's rule). The new effects show only on skins whose files declare them; no catalogue skin does yet. Disintegration is seen by every SJK client when the killer's skin has it, so which skins get it is Sol's call.
8. **Screenshots** for the pull request, and the changelog line under "Unreleased", which Sol may rewrite or split.
9. **History.** The squash leaves one large commit. Split it if review wants smaller ones.

## Code map

| File | What it holds |
| --- | --- |
| `saber_lab.rs` | The lab's folder, mount, stamp and ownership; `saberlab`; the lab blades' ids |
| `collection_saber_lab.rs` | The tab: stage controls, keys, clicks |
| `collection_saber_lab_builder.rs` | The builder's panels, drag and drop, its own hit-testing (the canvas holds 96 widgets) |
| `collection_saber_lab_editor.rs` | The fields' rows (the editor's view) |
| `blade_builder.rs` | The effect catalogue (names, kinds, words, tooltips), `Shape`, `may_go`, `moved` |
| `blade_editor.rs` | `Editor`: the seven files as ordered JSON, undo and redo, save and follow, kept originals, Revert |
| `ordered_json.rs` | JSON that keeps its key order and writes back in it |
| `menu_stage/lab.rs` | The lab's sounds on the stage: ignition, retraction, swings, the hum and crackle |
| `menu_stage/sabers.rs` | `stage_skins()`, stage trails, the floating copy's still trail |
| `blade_skin_smoke.rs` | The eighth set's sections, read and checked |
| `blade_skin_shape.rs` | What the format holds, read from the format itself: the editor's sections and their starting values |
| `saber_smoke.rs`, `saber_ribbons.rs`/`.wgsl`, `saber_sparks.rs` | Smoke, ribbons, sparks and bolts drawn, plus the shaders they need in the effect atlas |
| `disintegration.rs`, `world_shot_saber_kill.rs` | The burn on a saber kill and its world shot |

## Things learned the hard way

- **The VFS lists files only one subfolder deep.** So the kept originals live at `<lab>/originals/<id>/`, outside `skins/blades`, and are never read as skins.
- **The lab folder overrides pack files of the same name.** For example, `saber_sun.bladeskin` tunes a copy of the Sun. Milamber's own lab folder holds old Sun and Void copies.
- **A signature's built-in file stands in for a broken one**, so a broken lab copy of a signature still draws.
- **Lanes are appended at the end of `SkinUniform`.** Tests that slice `lanes.len() - N` go stale each time a lane is added.
- **The lab's UI tokens are 3_000 to 3_499, and the lab tab's clicks are read before the other tabs'.** Main's nameplate styles and toy options took the lab's old numbers (1_380 and 1_390) while the branch was apart.
- **Blade-skin smoke, spark and bolt shaders are not in a map's effect atlas.** They join it through `ParticleAtlas::admitted`, which every rebuild takes in, on main's worker (`particle_atlas::Pending`). In a fresh world the smoke therefore shows a few frames late.
- **The menu stage is the local player.** Its blades wear `stage_skins()`, a pair: the Collection's previewed shader goes on the side being dressed, and the lab tab dresses both.
