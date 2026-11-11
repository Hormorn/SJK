# SJK project wiki

This directory is the canonical project wiki for contributors and AI assistants.
It lives beside the source so implementation and documentation can be reviewed
and changed together. Begin with the status page, then the architecture.

| Page | Contents |
| --- | --- |
| [Status and priorities](status.md) | Current scope, verification and open work |
| [Architecture](architecture.md) | Crate ownership and compatibility boundaries |
| [Development](development.md) | Build, validation and contribution workflow |
| [Reviewing and merging](reviewing.md) | How maintainers and their assistants review and merge pull requests: the golden rules (stock JKA first, economy care), own pull requests, the questions to ask, the merge steps |
| [Distributable packages](packages.md) | Windows/Linux ZIP layout, builds and startup checks |
| [Client](client.md) | Launch, configuration, content and common commands |
| [Dedicated server](server.md) | Hosting, configuration and local testing |
| [Rendering](rendering.md) | BSP rendering, lighting, UI and measurement |
| [Networking and gameplay](networking.md) | Protocol 26, prediction and server authority |
| [SJK conventions](sjk.md) | Branches, names, defaults, versions, releases, changelog, credits and debug panel |
| [Classic+ menus](classic-plus.md) | SJK's modernised classic pages: rules, layout and code recipe |
| [SJK UI](sjk-ui.md) | SJK's own menus (`ui_menuStyle sjk`): design, tokens, screens and plan |
| [Player identity](identity.md) | SJK's identity key, the hub, scoreboard badges, medals, clan tags: design, limits and privacy |
| [SJK chat and emotes](hub-chat.md) | The chat every SJK player shares through the hub, and the emotes path |
| [Holocron drops](holocrons.md) | Loot earned by playing: tiers, odds, caps, the active flag, the pop-up, chat lines, the staff tools and the list other screens read |
| [Cards and credits](cards.md) | Opening holocrons: sealed cards (rarity, finish, edition), credits and their sign, the ceremony, the Cards tab, odds; the first set and the shop planned |
| [Client mods](mods.md) | Mods kept out of the core client (`japlus.*`, `jof.*`): switches, names, what they may do |
| [JoF content](jof-content.md) | JoF's maps, skins, hilts and packs from JoF's repository (`jof.downloads`), opt in |
| [Sith cloud](sith-cloud.md) | The S community's maps, skins and extras from their own server, into the game's base (`sith.downloads`), opt in |
| [Model sync](model-sync.md) | JoF- and Sith-verified players share their models through the Sith cloud, everyone downloads them; the Model sync page (`modelsync`), checks in `sjk-model-sync` |
| [Rig models](rig-models.md) | glTF player models with their own skeleton, driven by the game's animations: manifest, bone maps, limits |
| [Mystery boxes](mystery-boxes.md) | Boxes SJK's creator places on maps, opened by players for credits: the hub's roll, the reach, the creator's commands |
| [WHEEL OF DOOM](wheel-of-doom.md) | The toy players spin for credits: the hub's spin, the sixteen outcomes and their effects on the spinner, the sounds, the model contract for Sol's wheel |
| [Quests](quests.md) | The Quests toy: the hub gives players taking part a quest (hug or kiss two others in a minute) for credits; how a target's client proves it |
| [The Toy Master](toy-master.md) | Secret toys SJK's creator hides on maps: what the hub sends, presses and shows, the "?" and the hidden achievements, the creator's commands |
| [Map edits](map-edits.md) | SJK's own map fixes drawn by SJK clients: hidden props and surfaces, placed models, remaps, per server; the staff editor, the browser editor and the hub copies |
| [Profile banners](profile-banners.md) | Collectible profile covers, Echo rewards, equip/unequip and compatibility |
| [Unlockables and looks](unlockables.md) | Blade skins and other unlockables, and looks every SJK player sees through the hub |
| [Saber Lab handover](saber-lab-handover.md) | The Saber Lab's state for Sol to finish (03/11/2026): built, planned, owed before merge, Milamber's blade |

[AGENTS.md](../AGENTS.md) defines the documentation maintenance rules. Update the
page that owns a fact rather than adding a second account elsewhere. New facts
should link to code or identify their verification; proposals must remain labeled
as planned until implemented and checked.

For installation and quick launch commands, see the [project README](../README.md).
