# Reviewing and merging pull requests

How SJK reviews and merges someone's pull request. It is written for the maintainers
who may merge into `main` (Sol, Slash and Milamber since 03/11/2026, Lumaya since
05/11/2026 and Creyon since 11/11/2026; [Automation](sjk.md#automation)) and for the AI assistants that review for
them: an assistant asked to review or merge a pull request reads this page first and follows it
to the end. It applies [AGENTS.md](../AGENTS.md) and the
[pull request rules](development.md#branches-commits-and-pull-requests) to another
person's change; where those pages say more, they hold.

Two rules come before everything else on this page.

## Golden rule 1: base Jedi Academy stays base Jedi Academy

SJK is the stock game first. A pull request adds things on top of it as options: a
player who installs it and changes no setting must find the game looking, sounding and
playing as before ([Defaults](sjk.md#defaults), Sol's rule of 30/10/2026). Bug fixes
and SJK's own pages are not affected; everything else is.

- A new setting that changes how the game looks, sounds or plays compared with stock
  JKA is off by default, or at its stock value. Only Sol moves a default away from
  stock. A contributor asking for "on by default" is a question for Sol, not a merge:
  #292, server remaps on by default, was declined.
- An existing setting keeps its default and its meaning. Players' saved values are not
  reset towards a non-stock choice without Sol; when Sol agrees, the change carries an
  `= <cvar>` line in the changelog ([Changelog](sjk.md#changelog)).
- Movement, weapon and saber timing, animation and prediction stay OpenJK's `codemp`
  behaviour. A change there brings reference evidence (the OpenJK source, a capture, a
  side-by-side recording), never "it feels better", and holds at 8, 7, 4 and 3 ms
  command steps. Protocol 26 does not change; wire code brings byte fixtures
  ([AGENTS.md](../AGENTS.md#compatibility-and-architecture)). When players asked for
  the old behaviour back after a prediction change, it became a switch
  (`cg_predictLegacy`): a gameplay change that players may dislike ships with its way
  back.
- It behaves the same on stock servers and on every mod it does not target. Behaviour
  for one server, mod, map or community goes in a named table with a player override,
  never an inline check ([no hardcoded special cases](../AGENTS.md#compatibility-and-architecture)),
  and a mod's features stay behind its `mod_<id>` switch ([mods.md](mods.md)).
- Retail assets come first in every list, and community maps and models keep working.
- An existing look (a HUD, a menu, the classic pages, a scoreboard) is not redesigned in
  passing. A new style is an option beside the old one; replacing one is Sol's call
  (#351, a redesign of the SJK classic HUD, waited for Sol).
- Hats and capes stay free for everyone to wear: no unlocks on them.

Ask of every pull request: with this merged and no setting touched, what is different
on a stock server? Anything beyond a fixed bug needs a switch that is off by default,
or Sol's word.

## Golden rule 2: be very careful with the economy

The economy is everything a player earns, keeps, spends or trades: credits, kyber
shards, holocrons, kyber crystals, cards, saber shaders and finishers, tags, banners,
the shop, crafting and shattering, trading, mystery boxes and the box hunt, achievements
that pay, and anything new that gives or moves value ([cards.md](cards.md),
[holocrons.md](holocrons.md), [unlockables.md](unlockables.md)). Players have put
weeks into it and were promised compensation if odds change. A hole one player finds
is known to all within a day, and a revert does not take back what was gained through
it. So:

- **Maintainers may merge an economy change themselves** once its review meets every
  point of this list and its account (below) is written on the pull request. What
  they cannot do is the hub's part: only Sol deploys the hub and sets its switches,
  so a change that needs the hub is merged with a note saying so, and Sol is asked
  for the deploy.
- **The hub decides.** The client asks, shows and animates; the hub rolls, checks the
  limits and writes the ledger. A value computed on the client that decides what a
  player gets (an amount, odds, a winner, "they did it") is a hole. Anything the
  client reports (a position, a kill, time played, a catch) can be forged by a modified
  client, so whatever the hub pays on its word is capped.
- **No minting.** Name where every credit, shard or item comes from and where it goes.
  A game of chance gives back less than it takes on average: compute its return from
  the code and its tables, don't take it from the description.
- **Every gain has limits on the hub:** a cooldown and a daily cap, per key and per
  address (alts share an address). "Nobody will do that" is not a limit.
- **Value does not move between players** except through trading, which swaps items
  for items: credits are not tradeable and cards never trade. Anything else that lets
  value pass from one player to another (a poker table, a credit rain, a gift, a bet
  between players) is a transfer: verified players only, same-address refusals, caps
  on both giving and receiving.
- **Look for loops** between the new flow and every existing source and sink: bought
  or traded holocrons counted as found ones (#278's Echoes did, fixed at merge), an
  achievement that pays and can be cleared and earned again, buy then shatter then
  craft, a refund larger than the price.
- **Odds, prices and tables are pinned by tests.** A pull request that changes one
  says so in its title, and its review says who gains and who loses by it.
- **Rollout:** a new economy feature comes behind a hub switch, staff-only first, and
  is opened to everyone once staff have tried it; Sol flips the switch. The hub side is
  deployed before the client release that needs it, and older clients are refused or
  ignore it cleanly.
- **No real money**, ever, and nothing that looks like it.
- **Staff tools stay staff-only and quiet:** no line in the changelog, the credits or
  the release notes, and no command that grants anything outside staff tools.

The review of an economy change states, on the pull request: the source and the sink,
the expected and the largest gain per player per day, what an alt or a modified client
can get, the hub's limits, the switch and the rollout order. The casino (#334) and
credit rain (#335) reviews of 03/11/2026 are the model: they asked for daily net caps,
a cooldown, same-address refusals and verified-only access before either may merge.

## Before reviewing

- **Duplicates and claims.** Look for another open or draft pull request doing the same
  thing ([claim before you fix](../AGENTS.md#verification-and-scope)). If two overlap,
  say so on both and ask Sol which goes in.
- **Owned areas.** Blades (signatures, skins, the Blade Lab) and trading are Creyon's
  (Sol, 04/11/2026): a pull request there from someone else waits for Creyon's word.
- **Read everything first:** the description, the linked issue or hub report, every
  comment, and earlier reviews. A second review checks each earlier point, one by one.
- **Your own pull request** is reviewed by another maintainer or Sol, unless it is
  low risk: then you may merge it into `main` yourself, as Sol does with their own
  changes, still following every check and step on this page. Low risk means small,
  with no economy and no hub change, no new default and no change to an existing one,
  nothing in movement, prediction or the protocol, and nothing that redesigns or
  removes what players already have. When in doubt, it is not low risk.
- **Check it out apart** from any folder someone plays or works in, merged with the
  current `main`, since conflicts and semantic breaks (a field added on one side that
  the other side's new code does not set) only show there:

  ```sh
  git fetch origin main pull/<n>/head:pr-<n>
  git worktree add ../SJK-wt/pr-<n> -b review/pr-<n> origin/main
  cd ../SJK-wt/pr-<n>
  git merge --no-ff pr-<n> -m "Merge PR #<n> (<Author>: <title>)"
  ```

  Give each worktree its own Cargo target directory when building several at once.

## What to check

Read the whole diff and the code it connects to, not only the changed lines. Run the
checks yourself: a pull request's own account of its tests is a claim, not evidence.

- **The golden rules** above, first.
- **Scope:** one topic. Unrelated fixes, renames and formatting sweeps go in their own
  pull request.
- **Correctness:** the logic, the edge cases, what happens on reconnect, map change,
  server restart, an old hub or an old server. Stale state left behind after a
  disconnect is the most common miss (#348's burns after a reconnect).
- **Performance:** no new allocation, locking or quadratic work in per-frame paths;
  measured in a release build when it matters. Blade changes bring the
  `scripts/blade_cost.py` tables from `main` and the branch, run in one sitting
  ([AGENTS.md](../AGENTS.md#compatibility-and-architecture)).
- **The project rules:** an SJK UI version of every player-facing addition
  ([sjk-ui.md](sjk-ui.md#sjk-ui)), the text field rule (Ctrl+A, C, V and a held
  Backspace), no identity key id drawn anywhere in game, new work in focused modules
  rather than `main.rs`, setting names without an `sjk_` prefix
  ([Names](sjk.md#names)), dates `dd/mm/yyyy` and 24-hour times.
- **The hub:** a change that needs the hub has its matching pull request on the hub's
  shared copy, reviewed with it. Its merge note says "needs hub ... deployed first",
  and no release goes out before that deploy.
- **Content:** new art, models and sounds say who made them and under what terms.
  Nothing taken from another commercial game, and no code that reads or imports one
  ([AGENTS.md](../AGENTS.md#verification-and-scope)). No retail game data, personal
  paths, credentials or personal details anywhere, screenshots included.
- **Documentation** updated in the same change: the wiki page that owns the subject,
  [status.md](status.md) for verification and known limits, the Defaults table for a
  new default.
- **Changelog:** a player-visible change has its line under `## Unreleased` in player
  words, ending in its credit with the real author's name (`_(Slash, PR #350)_`,
  `_(Lumaya, after Blep's idea)_`). Contributors' assistants sometimes credit Sol or
  themselves: fix it. Staff-only work has no line.
- **Screenshots** for anything players see ([the rule](development.md#branches-commits-and-pull-requests)).
  Ask for them when missing.
- **The checks:** `cargo fmt --all --check`, `cargo build --locked --workspace`,
  `cargo test --locked --workspace`, `cargo clippy --locked --workspace --all-targets`,
  and before the merge the startup stack check
  ([AGENTS.md](../AGENTS.md#verification-and-scope)). A failure there blocks the merge;
  raising the stack reserve is not the fix.

## Ask, don't guess

When something is unclear, ask the author on the pull request instead of assuming an
answer. Questions to put whenever they apply:

- What does it fix or add, and who asked for it (a hub report, an idea, Sol)?
- How was it tested: which server and mod, which FPS, online or local? Screenshots or a
  short video?
- What does a player who changes nothing see? Which setting turns it on or off, and
  what is its default?
- Does anything change on stock servers or on other mods? What is the evidence
  against OpenJK?
- Does it need the hub? Which hub pull request, and in which order do they ship? What
  does an older client do?
- For the economy: the account in golden rule 2. Where does the value come from, what
  is the most a player gets in a day, what stops an alt or a modified client?
- What is the per-frame cost, and was it measured in a release build?
- What is not tested yet?

Write the review as a numbered list, each point with its file and line, marked as
blocking or as a suggestion. Say what is good in concrete terms when it is. Contributors
give their own time: be direct about problems and kind to the person.

Some choices are not the reviewer's to make. Ask Sol, with the options and your
recommendation, and wait for the answer:

- a default away from stock, or a new meaning for an existing setting;
- a redesign or removal of something players already have;
- a hub deploy or a hub switch (an economy feature opened to everyone, for one);
- unlocks or limits on cosmetics, and what is staff-only;
- a new dependency, a change to `.github/workflows`, the release or the licences;
- closing a pull request.

## Decide

- **Merge** when everything above holds and no question waits for Sol.
- **Fix at merge** what is small and mechanical: a missing changelog line or credit, a
  semantic conflict with `main`, a one-line bug. Make it a commit of its own on the
  merge branch and say on the pull request what changed and why. Never push to the
  contributor's branch without asking: some authors want to make every fix themselves
  (Creyon on trading, #197).
- **Ask for changes** when the fix is a design decision, larger than a few lines, or in
  code the author knows better.
- **Hold for Sol** anything in the list above.
- **Close** only with Sol's word, with a comment that says why (#356 was closed that way).

## Merging

1. Merge the pull request's commits as they are, in the merge branch from
   [Before reviewing](#before-reviewing): no squash and no rebase, so GitHub marks the
   pull request merged and the credits count it. One merge per pull request, named
   `Merge PR #<n> (<Author>: <title>)`. Several pull requests may go in one batch,
   each with its own merge.
2. Resolve conflicts by keeping both sides' intent; rebuild and test after each one.
3. Do the bookkeeping in the same merge ([Credits](sjk.md#credits)):
   - the changelog line, as above;
   - [credits.txt](../crates/sjk-viewer/assets/credits.txt): a `did:` line with the
     pull request's number and its count of merged pull requests on the author's card,
     and the same count in [CREDITS.md](../CREDITS.md). A first contribution gets a
     card and an entry in `PEOPLE` of [credits_history.py](../scripts/credits_history.py);
   - a [debug panel](sjk.md#debug-panel) entry saying how to test it, its area one
     of the list at the top of `debug_panel.txt`;
   - documentation the pull request missed.
4. Run all the checks of [What to check](#what-to-check) on the merged result,
   the startup stack check included.
5. Run `python scripts/credits_history.py` and amend the merge commit with the
   rewritten `credits_history.txt` (`git commit --amend`), so the merge carries its
   own history.
6. Push to `main` as a fast-forward: `git push origin HEAD:main`. `main` only moves
   forward; GitHub refuses a force-push. A mistake is fixed by a new commit, and a bad
   merge is undone with `git revert -m 1 <merge>`.
7. Check that the pull request shows Merged, and comment on it: the merge commit, any
   fix made at merge, what was and was not tested, and what still waits (a hub deploy,
   a follow-up).
8. Tell Sol when the merge needs the hub deployed or holds something back from the next
   release. Releases follow [Cutting a release](sjk.md#cutting-a-release) and are not
   part of a merge.
9. Remove the worktree (`git worktree remove`) once the merge is pushed.

## Report

Finish with a short account to whoever asked: what was merged (commits and pull
request numbers), what was fixed at merge, what was asked of each author, what waits
for Sol and why, and what nobody has tested in game yet. Merged and tested are not the
same thing: say which one each change is.
