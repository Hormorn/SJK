# Working on SJK

SJK is a native Rust engine, client and dedicated server for Jedi Academy
multiplayer. Read [docs/status.md](docs/status.md),
[docs/architecture.md](docs/architecture.md) and the relevant page in the
[project wiki](docs/README.md) before changing code.

## Compatibility and architecture

- Preserve OpenJK multiplayer `codemp` movement, weapon/saber timing and
  animation behavior. Gameplay changes need reference evidence, not visual guesses.
  Cover integer command steps of 8/7/4/3 ms (125/142/250/333 FPS).
- Protocol 26 is fixed. Encode/decode changes in `sjk-protocol` or `sjk-network`
  need OpenJK-emitted or captured byte fixtures. Never infer wire correctness
  from a successful connection alone. Use `codemp`, not the single-player `code` tree.
- Keep legacy formats, identifiers and limits in compatibility adapters.
  Engine world ownership and presentation must remain independent of the wire.
- Community PK3 maps and models are compatibility inputs, just like retail assets.
  Modern rendering and UI must preserve gameplay and content semantics.
- Write small, named modules with rustdoc for public APIs. Keep new viewer work
  out of `crates/sjk-viewer/src/main.rs`; use focused modules instead.
- Every player-facing addition (screen, page, pop-up, setting, browser) ships
  with its SJK UI version in the same change; a classic+-only addition is not
  accepted, while an SJK UI-only one is (it then shows its SJK look in every menu
  style). See [the rule in sjk-ui.md](docs/sjk-ui.md#sjk-ui).
- No hardcoded special cases (Sol's rule, 12/10/2026). Behavior tied to a
  particular server, mod, client, map or community (its version string, gamename,
  address, name or a magic value) never lives as an inline check or a one-off
  function. It goes in a named, documented table, one entry per case with why and
  the evidence, read by generic code, and the player can override it with a cvar or
  setting. Supporting a new case means adding an entry, not code. Example:
  [server_caps.rs](crates/sjk-viewer/src/server_caps.rs) and `gameplay_fps_nocap`.
- The retail Jedi Academy's own assets ("OG") come first in every list of installed
  content a menu offers (models, species, saber hilts, and what a later list adds), under
  an "Jedi Academy" heading with the community's after it under "Community" (Sol's rule,
  12/10/2026). Retail is told by the archive (`assets0.pk3` to `assets3.pk3` of `base`,
  [retail_assets.rs](crates/sjk-client/src/retail_assets.rs)), never by an asset's name;
  a catalogue puts the retail ones first and says how many they are.
- Every text field takes Ctrl+A, Ctrl+C and Ctrl+V and repeats a held Backspace (Sol's
  rule, 12/10/2026), in every screen, pop-up and prompt, SJK UI and classic alike:
  Ctrl+A selects all the text (typing or a paste then replaces it, Backspace clears
  it), Ctrl+V (and Shift+Insert) pastes, Ctrl+C (and Ctrl+Insert) copies the field
  (never a password), and Backspace held deletes on, typed keys held repeat. A menu handler
  that drops `event.repeat` must let an open field's Backspace and typed keys through.
  A new field uses [text_entry.rs](crates/sjk-viewer/src/text_entry.rs) (one line) or
  [sjk_chat_field.rs](crates/sjk-viewer/src/sjk_chat_field.rs) (chat), not its own
  key handling.
- Every release carries a short player-facing recap in `CHANGELOG.md` (Sol's rule,
  12/10/2026): three to six `> Heading: sentence` lines under the release title, which
  the client's what's-new card shows after an update. Write it in the commit that
  renames "Unreleased" to the version; the tests reject a release without one. See
  [the rule in sjk.md](docs/sjk.md#changelog).
- When a change gives an on/off setting a new default or a new meaning, or adds one
  players should decide on, put a `= <cvar>: <sentence>` line under "Unreleased" in
  `CHANGELOG.md` in the same change, rather than a migration marker cvar: after the
  update the what's-new card opens on it with its switch. Add `reset`
  (`= <cvar> reset: ...`) when values saved under the old default or meaning must not
  carry over unchosen. The cvar must be an on/off Settings row (the tests check);
  other settings still migrate in `console_session.rs`. See
  [the rule in sjk.md](docs/sjk.md#changelog).
- Defaults are the stock base Jedi Academy experience (Sol's rule, 30/10/2026). A new
  setting that changes how the game looks, sounds or plays compared with stock JKA is
  off (or at its stock value) by default: players opt in. Do not turn an existing one
  on by default or reset players' saved values towards a non-stock choice on a
  contributor's request; that stays Sol's decision, recorded in the
  [Defaults table](docs/sjk.md#defaults). Example: #292, server remaps on by default,
  declined.
- Avoid new allocation, locking and quadratic work in per-frame paths. Measure
  performance-sensitive changes in release builds; 500+ FPS is a target, not a
  blanket claim about current performance.
- Blades are timed and kept as cheap as possible (Sol's rule, 31/10/2026). A change to
  a blade skin (a `.bladeskin` file, a signature under
  `crates/sjk-viewer/src/blade_signatures/`) runs `python scripts/blade_cost.py --ids
  <changed blades>`; a change to the shared blade shading (`saber.wgsl`, `saber_gpu.rs`,
  the `blade_skin_*` sections) runs `python scripts/blade_cost.py` (every Legendary
  and Mythical blade). Run it on `main` and on the branch in one sitting, as times move
  between runs, and put both tables in the pull request. A blade costing more than on
  `main` needs a reason and Sol's agreement. A new blade, or a new effect on one,
  is optimized until it looks the same for less (fewer steps, shorter loops,
  effects cut off where they no longer show) before review, not after. See
  [the tool](docs/rendering.md#blade-lab-signature-shaders).

## Verification and scope

- Follow [development.md](docs/development.md). Before handing off a change, run
  `cargo fmt --all --check`, `cargo build --locked --workspace`,
  `cargo test --locked --workspace` and
  `cargo clippy --locked --workspace --all-targets`. The build and the tests must
  give no compiler warning: CI fails on one ([CI](docs/development.md#continuous-integration)).
  Explain any check that could
  not run.
- Every merge into `main` first runs the startup check, which needs a GPU and the
  game data in `JKA_GAME_DATA`:
  `cargo test --release -p sjk-viewer --bin sjk the_client_starts_in_a_quarter_of_the_main_stack -- --ignored`.
  It builds the menu's world and draws its first frames in a quarter of the main
  thread's stack ([startup_stack.rs](crates/sjk-viewer/src/startup_stack.rs)); a
  failure (`STATUS_STACK_OVERFLOW`) blocks the merge until the stack use is found and
  cut. Raising the reserve again is not the fix.
- Passing Cargo tests currently does not establish gameplay parity: the repository
  has no bundled regression suite. Use focused external evidence for affected behavior.
  Small unit tests and fixtures that pin the changed behavior, such as the byte
  fixtures above, belong with the change; they never contain retail game data. New
  test infrastructure, harnesses or large reports need task authorization.
- Only exercise servers you control without human players. Do not automate chat
  messages. Use an unobtrusive name and stop only processes started for your check.
- Other games' assets belong to their owners (Sol's rule, 14/10/2026). Pulling models,
  textures or animations out of a commercial game (Fortnite, owned by Epic, for
  example) is against that game's licence terms. Fan extraction tools are tolerated,
  but a game-specific importer in a public game with downloadable releases invites a
  takedown of the repository. So SJK never ships, names or automates extraction from
  another game: no "import from <game>" button, no code that reads another game's
  archives, no guide that sends players there. It never uploads, hosts, relays or
  downloads such assets either: not in the repository, releases or hub, not between
  players, no screenshots of them in the project's channels. Players who want one do
  it themselves, on their own disk, with their own files; SJK's tools
  ([rig models](docs/rig-models.md) and their Blender add-on) take any glTF and say
  nothing of where it came from. Models whose authors allow sharing are not affected.
- Never commit retail game data, downloaded PK3s, credentials or personal paths.
  Put scratch evidence under `target/parity-reports/`, not `/tmp`, and remove it
  when finished. Keep a concise verification summary in the change description.
- Respect the requested scope and existing work. Do not discard unrelated changes,
  rewrite history or publish without authorization. A status page is context, not
  permission to start a new task.
- Claim before you fix (Sol's rule, 26/10/2026), so two people (or their assistants)
  never fix the same thing. Before starting on a bug, report or feature, look for an
  open pull request on this repository, drafts included, that covers it, and, for a
  player's bug report, for a claim on it in the hub (staff see and set claims there).
  If someone has it, leave it to them or comment on their pull request instead. Once
  you start, open a draft pull request early, one commit is enough, naming the report
  or issue it fixes, and claim the hub report if you can. A claim lapses after two
  days without being renewed; a draft pull request holds until it is closed.
- Follow the [branch, commit and pull request
  rules](docs/development.md#branches-commits-and-pull-requests): one topic per
  branch and pull request, never commit to `main`.
- Reviewing or merging someone's pull request follows
  [reviewing.md](docs/reviewing.md) from start to end, its two golden rules first:
  base Jedi Academy stays base Jedi Academy (everything added is optional on top),
  and the economy gets the strictest review (sources, limits, alts, loops). Only Sol
  deploys the hub. A maintainer merges their own pull request only when it is low risk.
- Attach screenshots to a pull request whose change players can see, whenever you
  can take them ([the rule](docs/development.md#branches-commits-and-pull-requests)):
  the project posts them in its Discord `#screenshots` channel.

## Documentation is part of the change

- When behavior, architecture, configuration or build instructions change, update
  the relevant wiki page in the same change and commit. Add new pages to the index.
- Update `docs/status.md` when verification, a known limitation or agreed priorities
  change. Record the revision, environment, evidence and limits of new verification.
- Distinguish implemented, verified and planned. Code presence is not proof of
  correctness; a smoke check is not full compatibility or performance certification.
- Keep durable decisions and current facts in the wiki. Do not append chat transcripts,
  session diaries, speculative completion claims or duplicate task histories.
- Documentation-only changes belong with their subject. Unchanged behavior does not
  require a ceremonial wiki update. Review affected links when moving code or docs.
- `AGENTS.md` is the shared instruction source; `CLAUDE.md` points here. Keep
  operational facts in the wiki so assistant-specific files cannot drift apart.

Finish with a short report of what changed, how it was verified and what remains
unverified. Commit and push only within the user's authorized scope.
