# Unlockables and looks

Unlockables are cosmetic things an SJK player owns at the hub (`cl_hubUrl`,
sjk.dfox.app): saber blade skins with their own look and sounds, the **Sun**,
**Storm**, **Void**, **Frost** and **Prism** blades. Their art is SJK's own and not in
this repository: the hub delivers it
to SJK clients in packs ([Packs](#packs)), and the client draws it with open, generic
code from data ([Blade-skin files](#blade-skin-files)). A player's *look* (the blade skin they wear and whether their
Illuminate holocron is lit) travels through the hub, so every SJK player on the
same game server sees it. Players on stock clients see nothing, and no game
server knows of any of it.

Status (08/10/2026): designed with Sol the same day and built on
`personal/saber-skins` (client, stacked on `personal/sjk-chat`) and
`feat/unlocks-looks` (Sol-Vulpes/SJK-hub, stacked on `feat/chat-emotes`). The
hub's side is not deployed, so nothing has been seen with real players yet. Each
section below says what is built and how it was verified.

09/10/2026, branch `feat/more-blade-skins` (stacked on `personal/blade-skins`, with the
hub's `feat/more-blade-skins`): a skinned blade's tip is round instead of square
([Blade skins](#blade-skins)); the generic shading gained lightning arcs, drifting motes
and a turning hue, and the catalogue the Storm, Void, Frost and Prism blades. Checked
by unit tests and a CPU copy of the shader's maths rendered to images; not yet seen on a
GPU or heard.

10/10/2026, branch `personal/saber-shaders` (with the hub's `feat/saber-shaders`): nine
more saber shaders (Unstable, Molten, Spectral, Glitch, Hologram, Runic, Chameleon,
Banner, Heartbeat) and the generic sections they use ([Blade-skin
files](#blade-skin-files), [rendering.md](rendering.md#saber-blade-skins)); up to 16
skins load at once. The Collection's rack and the Saber tab's blade row scroll, as
fifteen choices no longer fit. Seen in off-screen GPU renders on duel6 (world shot
`duel6_second_blades`); not in a live match, not heard.

## Decisions

| Question | Decision |
| --- | --- |
| Who sees a look | Every SJK player on the same game server, through the hub. |
| Who may wear an unlockable | Only a key the hub lists it for. The hub refuses to relay a look whose blade skin the key does not own (`not_unlocked`). |
| How one is unlocked | The hub operator or staff grant it by hand (Staff page, operator command), and since 10/10/2026 a medal brings one while held: Bug Hunter the Glitch, Tester the Hologram, Contributor the Runic blade ([Medal shaders](#medal-shaders)). Achievements granting them are an idea for later, with harder goals than today's. Since 17/10/2026 holocrons drop every shader but the medals' ([Holocron shader drops](#holocron-shader-drops)). Since 25/10/2026 the Unique ones come only by hand, Sol's thank-you gifts ([Unique shaders](#unique-shaders)). |
| How a look travels | Stored on the key's live claim, listed in presence for players who arrive later, and sent through the feed as a live event when it changes. |
| Illuminate | Free for everyone, a toy (`toy_illuminate`); its lit state is part of the look so others see the holocron. |
| Toys in the look | Since 16/10/2026 a look lists the toys out that others draw (`toys`, ids; the [cardboard box](client.md#cardboard-box)'s is `cardboard_box`, [Order 66](client.md#order-66)'s `order66`, the [Hax](client.md#hax)'s `hax`, `hax_away` when its trap holds while away, since 26/10/2026 the [Monitor](client.md#monitor)'s `monitor` while its switch is on, so other clients foresee its hits, and since 30/10/2026 the [Banana peel](client.md#banana-peel)'s `banana` while it is on), sent only when one is out, so older hubs see the look they know. The hub takes up to 32 toy ids since 30/10/2026 (8 before, and a hub from before refuses a look listing more, as `bad_look`). A client draws the toys it knows (`looks::TOYS`) and ignores the rest. Free for everyone, like Illuminate. |
| Feed with chat off | The feed reads while the player is on a game server whatever `cl_sjkChat` says (chat stays hidden); in the menus it reads only with chat on. |
| Choosing | The Collection's Shaders tab (SJK UI look; the Unlockables page until 10/10/2026) lists every blade skin, owned or locked (a Unique one only to its holders), previews the chosen one on the player's own saber and equips owned ones; the Profile screen's Saber tab offers the owned blade skins too (10/10/2026); `cg_saberSkin` holds the choice. Since 23/10/2026 the Saber tab's Sounds row chooses the saber sounds apart from the blade (`cg_saberSounds`: follow it, stock, or an owned shader's; [Saber sounds](#saber-sounds)). |
| Art (looks, sounds) | SJK's own, all rights reserved, not under the code's GPLv2: kept in the hub's private repository and served by the hub to registered SJK keys as packs. The code that draws it stays open and generic: no skin's values, sounds or pictures are in this repository or its history to come. |
| Delivery | The identity service downloads the hub's packs into `assets/` beside `identity.key` (at start once registered, then every 6 hours, only when a pack's SHA-256 changed); the viewer mounts every cached pack at start and a new one at once, below the game data, so a PK3 with the same paths replaces their files. A cached pack mounts with the identity off too. |
| Tiers | Every saber shader has a tier: Common (white, 14/10/2026: filler that holocrons of any tier may drop; there is no Common holocron), the holocrons' four (Uncommon, Rare, Legendary, Mythical), their names and colours (10/10/2026, Sol's tiers; [Tiers](#tiers)), and Unique (rose, 25/10/2026: hand-given, seen only by its holders; [Unique shaders](#unique-shaders)). |
| Format | A blade skin is a JSON file, `skins/blades/<unlock id>.bladeskin`, holding every parameter of the generic shading, its trail, light and sound paths. The format is documented here; the files are art. |

## Catalogue

Fixed, in the hub (`src/unlocks.rs`) and in the client
([unlockables.rs](../crates/sjk-viewer/src/unlockables.rs), the one client list of
public metadata: id, name, kind, a line on what it is and how to get it; the looks, the
blade skins, `saberskin`, the Collection and the Staff page all read it); a new
unlockable needs both, and a blade skin also its file in a pack. Ids never change
meaning.

Every blade has a number, as a card has in its set (`Unlockable::number`): given once in
the order blades came (the Sun is 001), never changed or reused, so notes and reviews
name a blade by it. The Collection shows it only while `debug_collectibles` is on (Sol,
21/10/2026: after the tier, and "Blade 042" on its hover card); its search finds a blade by number ("42", "042" or "#42") and `saberskin 42`
wears it. A new blade takes the next number and a line here (a test checks this table).

Tiers in this table are Sol's after his review of 20/10/2026, which moved 26 blades
(Clone Wars and Exegol Rift up to Mythical; Molten, Spectral, Reactor, Bleeding Kyber, Sith
Alchemy, Felucia Spores, Phoenix Squadron and eight of Creyon's mythicals down to Legendary;
Storm, Prism, Unstable, Chameleon and four of Creyon's legendaries down to Rare; Targeting
Display up to Uncommon), and after his adjustments of 21/10/2026, which moved 38 more
(048 053 054 055 056 062 064 066 069 075 to Legendary; 043 063 068 070 072 074 078 079 088
089 090 097 139 to Rare; 005 006 014 036 to Uncommon; 004 012 013 019 023 030 031 103 107
108 133 136 to Common; 11 mythicals left, 12 with the Varykino Blossom of 21/10/2026). The rounds' notes below keep the tiers the blades
came in.

Blades 170 to 299, Creyon's Star Wars round (PR #370, merged 06/11/2026), do not drop yet ("Coming to holocrons soon.", `COMING_TO_HOLOCRONS`): Sol tests them first, given by hand, then turns `droppable` on in the hub's catalogue and the client's together.

| No. | Id | Name | Tier | Smith | Drops |
| --- | --- | --- | --- | --- | --- |
| 001 | `saber_sun` | Sun blade | Mythical | Sol | yes |
| 002 | `saber_storm` | Storm blade | Rare | Sol | yes |
| 003 | `saber_void` | Void blade | Mythical | Sol | yes |
| 004 | `saber_frost` | Frost blade | Common | Sol | yes |
| 005 | `saber_prism` | Prism blade | Uncommon | Sol | yes |
| 006 | `saber_unstable` | Unstable blade | Uncommon | Sol | yes |
| 007 | `saber_molten` | Molten blade | Legendary | Sol | yes |
| 008 | `saber_spectral` | Spectral blade | Legendary | Sol | yes |
| 009 | `saber_glitch` | Glitch blade | Rare | Sol | no |
| 010 | `saber_hologram` | Hologram blade | Rare | Sol | no |
| 011 | `saber_runic` | Runic blade | Rare | Sol | no |
| 012 | `saber_chameleon` | Chameleon blade | Common | Sol | yes |
| 013 | `saber_banner` | Banner blade | Common | Sol | yes |
| 014 | `saber_heartbeat` | Heartbeat blade | Uncommon | Sol | yes |
| 015 | `saber_tidal` | Kamino blade | Common | Creyon | yes |
| 016 | `saber_circuit` | Droid Logic blade | Common | Creyon | yes |
| 017 | `saber_aurora` | Hoth Aurora blade | Common | Creyon | yes |
| 018 | `saber_echo` | Echo Base blade | Common | Creyon | yes |
| 019 | `saber_nebula` | Nebula blade | Common | Creyon | yes |
| 020 | `saber_cinder` | Mustafar Ember blade | Uncommon | Creyon | yes |
| 021 | `saber_venom` | Geonosis Acid blade | Uncommon | Creyon | yes |
| 022 | `saber_sakura` | Naboo Blossom blade | Uncommon | Creyon | yes |
| 023 | `saber_firefly` | Endor Lights blade | Common | Creyon | yes |
| 024 | `saber_slate` | Durasteel blade | Common | Creyon | yes |
| 025 | `saber_amber` | Tatooine Gold blade | Common | Creyon | yes |
| 026 | `saber_lilac` | Lothal Twilight blade | Common | Creyon | yes |
| 027 | `saber_mint` | Yavin Jungle blade | Common | Creyon | yes |
| 028 | `saber_coral` | Mon Cala Coral blade | Common | Creyon | yes |
| 029 | `saber_ivory` | Jedi Temple blade | Common | Creyon | yes |
| 030 | `saber_glacier` | Ilum blade | Common | Creyon | yes |
| 031 | `saber_dusk` | Binary Sunset blade | Common | Creyon | yes |
| 032 | `saber_ruby` | Sith Ruby blade | Uncommon | Creyon | yes |
| 033 | `saber_amethyst` | Kyber Amethyst blade | Uncommon | Creyon | yes |
| 034 | `saber_jade` | Dagobah Jade blade | Uncommon | Creyon | yes |
| 035 | `saber_moonlight` | Ahch-To Moonlight blade | Uncommon | Creyon | yes |
| 036 | `saber_neon` | Coruscant Neon blade | Uncommon | Creyon | yes |
| 037 | `saber_peppermint` | Kessel Spice blade | Uncommon | Creyon | yes |
| 038 | `saber_bubble` | Bacta blade | Rare | Creyon | yes |
| 039 | `saber_lavalamp` | Coaxium blade | Rare | Creyon | yes |
| 040 | `saber_reactor` | Reactor blade | Legendary | Creyon | yes |
| 041 | `saber_bleeding` | Bleeding Kyber blade | Legendary | Creyon | yes |
| 042 | `saber_sketch` | Death Star Plans blade | Rare | Creyon | yes |
| 043 | `saber_hourglass` | Beskar Forge blade | Rare | Creyon | yes |
| 044 | `saber_fiber` | Kyber Cluster blade | Legendary | Creyon | yes |
| 045 | `saber_serpent` | Sith Alchemy blade | Legendary | Creyon | yes |
| 046 | `saber_monarch` | Felucia Spores blade | Legendary | Creyon | yes |
| 047 | `saber_phoenix` | Phoenix Squadron blade | Legendary | Creyon | yes |
| 048 | `saber_quantum` | Mortis blade | Legendary | Creyon | yes |
| 049 | `saber_comic` | Clone Wars blade | Mythical | Creyon | yes |
| 050 | `saber_quicksilver` | Liquid Beskar blade | Legendary | Creyon | yes |
| 051 | `saber_obsidian` | Obsidian Edge blade | Mythical | Creyon | yes |
| 052 | `saber_abyssal_eye` | Abyssal Eye blade | Mythical | Creyon | yes |
| 053 | `saber_aurora_wyrm` | Ilum Aurora Storm blade | Legendary | Creyon | yes |
| 054 | `saber_balance` | Balance blade | Legendary | Creyon | yes |
| 055 | `saber_between_worlds` | Between Worlds blade | Legendary | Creyon | yes |
| 056 | `saber_big_bang` | Big Bang blade | Legendary | Creyon | yes |
| 057 | `saber_carbonite` | Carbonite blade | Mythical | Creyon | yes |
| 058 | `saber_dyson_sphere` | Dyson Sphere blade | Legendary | Creyon | yes |
| 059 | `saber_eclipse` | Eclipse blade | Mythical | Creyon | yes |
| 060 | `saber_event_horizon` | Event Horizon blade | Mythical | Creyon | yes |
| 061 | `saber_exogorth` | Exogorth blade | Mythical | Creyon | yes |
| 062 | `saber_hyperspace` | Hyperspace blade | Legendary | Creyon | yes |
| 063 | `saber_kyber_heart` | Kyber Heart blade | Rare | Creyon | yes |
| 064 | `saber_nebula_song` | Nebula Song blade | Legendary | Creyon | yes |
| 065 | `saber_quasar` | Quasar blade | Legendary | Creyon | yes |
| 066 | `saber_singing_kyber` | Singing Kyber blade | Legendary | Creyon | yes |
| 067 | `saber_sith_holocron` | Sith Holocron blade | Legendary | Creyon | yes |
| 068 | `saber_star_dragon` | Ashla Radiance blade | Rare | Creyon | yes |
| 069 | `saber_starforge` | Starforge blade | Legendary | Creyon | yes |
| 070 | `saber_superlaser` | Superlaser blade | Rare | Creyon | yes |
| 071 | `saber_uneti_tree` | Uneti Tree blade | Legendary | Creyon | yes |
| 072 | `saber_witchfire` | Witchfire blade | Rare | Creyon | yes |
| 073 | `saber_world_shift` | World Shift blade | Mythical | Creyon | yes |
| 074 | `saber_wraithlight` | Wraithlight blade | Rare | Creyon | yes |
| 075 | `saber_lightspeed_ram` | Holdo Maneuver blade | Legendary | Creyon | yes |
| 076 | `saber_hoth_avalanche` | Hoth Avalanche blade | Rare | Creyon | yes |
| 077 | `saber_blood_moon` | Dathomir Blood Moon blade | Legendary | Creyon | yes |
| 078 | `saber_force_echo` | Force Echo blade | Rare | Creyon | yes |
| 079 | `saber_hypermatter` | Hypermatter blade | Rare | Creyon | yes |
| 080 | `saber_kessel_run` | Kessel Run blade | Rare | Creyon | yes |
| 081 | `saber_unknown_regions` | Unknown Regions blade | Legendary | Creyon | yes |
| 082 | `saber_krayt_pearl` | Krayt Pearl blade | Rare | Creyon | yes |
| 083 | `saber_starkiller_frost` | Starkiller Frost blade | Legendary | Creyon | yes |
| 084 | `saber_royal_naboo` | Royal Naboo blade | Legendary | Creyon | yes |
| 085 | `saber_bespin_storm` | Bespin Storm blade | Legendary | Creyon | yes |
| 086 | `saber_scarif_tide` | Scarif Tide blade | Legendary | Creyon | yes |
| 087 | `saber_assassin_droid` | Assassin Droid blade | Rare | Creyon | yes |
| 088 | `saber_aurodium` | Aurodium blade | Rare | Creyon | yes |
| 089 | `saber_kashyyyk_monsoon` | Kashyyyk Monsoon blade | Rare | Creyon | yes |
| 090 | `saber_malachor_forge` | Malachor Forge blade | Rare | Creyon | yes |
| 091 | `saber_yavin_vines` | Yavin Vines blade | Legendary | Creyon | yes |
| 092 | `saber_exegol_rift` | Exegol Rift blade | Mythical | Creyon | yes |
| 093 | `saber_shattered_kyber` | Shattered Kyber blade | Legendary | Creyon | yes |
| 094 | `saber_twin_suns` | Twin Suns blade | Legendary | Creyon | yes |
| 095 | `saber_jedi_spirits` | Jedi Spirits blade | Legendary | Creyon | yes |
| 096 | `saber_ord_mantell_storm` | Ord Mantell Thunderhead blade | Legendary | Creyon | yes |
| 097 | `saber_mustafar_eruption` | Mustafar Eruption blade | Rare | Creyon | yes |
| 098 | `saber_hyperlane` | Hyperlane blade | Legendary | Creyon | yes |
| 099 | `saber_ahch_to_glow` | Ahch-To Glow blade | Rare | Creyon | yes |
| 100 | `saber_imperial_schematics` | Imperial Schematics blade | Rare | Creyon | yes |
| 101 | `saber_jedi_texts` | Jedi Texts blade | Rare | Creyon | yes |
| 102 | `saber_star_map` | Star Map blade | Rare | Creyon | yes |
| 103 | `saber_ilum_frost` | Ilum Frost blade | Common | Creyon | yes |
| 104 | `saber_christophsis_crystal` | Christophsis Crystal blade | Rare | Creyon | yes |
| 105 | `saber_deflector_shield` | Deflector Shield blade | Rare | Creyon | yes |
| 106 | `saber_healed_kyber` | Healed Kyber blade | Rare | Creyon | yes |
| 107 | `saber_temple_mosaic` | Temple Mosaic blade | Common | Creyon | yes |
| 108 | `saber_dagobah_wisps` | Dagobah Wisps blade | Common | Creyon | yes |
| 109 | `saber_nar_shaddaa_rain` | Nar Shaddaa Rain blade | Rare | Creyon | yes |
| 110 | `saber_navigation_beacon` | Navigation Beacon blade | Rare | Creyon | yes |
| 111 | `saber_puzzle_holocron` | Puzzle Holocron blade | Rare | Creyon | yes |
| 112 | `saber_dac_reef` | Dac Reef blade | Rare | Creyon | yes |
| 113 | `saber_jakku_sandstorm` | Jakku Sandstorm blade | Rare | Creyon | yes |
| 114 | `saber_moisture_farm` | Moisture Farm blade | Rare | Creyon | yes |
| 115 | `saber_tipoca_stormglass` | Tipoca Stormglass blade | Rare | Creyon | yes |
| 116 | `saber_temple_sunbeam` | Temple Sunbeam blade | Rare | Creyon | yes |
| 117 | `saber_ion_coil` | Ion Coil blade | Rare | Creyon | yes |
| 118 | `saber_sith_thorn` | Sith Thorn blade | Rare | Creyon | yes |
| 119 | `saber_holo_recording` | Holo-Recording blade | Rare | Creyon | yes |
| 120 | `saber_data_disc` | Data Disc blade | Rare | Creyon | yes |
| 121 | `saber_cargo_stack` | Cargo Stack blade | Uncommon | Creyon | yes |
| 122 | `saber_otoh_gunga` | Otoh Gunga blade | Uncommon | Creyon | yes |
| 123 | `saber_victory_fireworks` | Victory Fireworks blade | Uncommon | Creyon | yes |
| 124 | `saber_endor_ivy` | Endor Ivy blade | Uncommon | Creyon | yes |
| 125 | `saber_naboo_core` | Naboo Core blade | Uncommon | Creyon | yes |
| 126 | `saber_gungan_waters` | Gungan Waters blade | Uncommon | Creyon | yes |
| 127 | `saber_cantina_lights` | Cantina Lights blade | Uncommon | Creyon | yes |
| 128 | `saber_imperial_searchlight` | Imperial Searchlight blade | Uncommon | Creyon | yes |
| 129 | `saber_flare_stick` | Flare Stick blade | Uncommon | Creyon | yes |
| 130 | `saber_flimsiplast` | Flimsiplast blade | Uncommon | Creyon | yes |
| 131 | `saber_canto_bight` | Canto Bight blade | Uncommon | Creyon | yes |
| 132 | `saber_sensor_sweep` | Sensor Sweep blade | Uncommon | Creyon | yes |
| 133 | `saber_takodana_rain` | Takodana Rain blade | Common | Creyon | yes |
| 134 | `saber_starfighter_canopy` | Starfighter Canopy blade | Uncommon | Creyon | yes |
| 135 | `saber_distress_beacon` | Distress Beacon blade | Uncommon | Creyon | yes |
| 136 | `saber_orto_plutonia` | Orto Plutonia blade | Common | Creyon | yes |
| 137 | `saber_rhen_var_frost` | Rhen Var Frost blade | Uncommon | Creyon | yes |
| 138 | `saber_seismic_charge` | Seismic Charge blade | Uncommon | Creyon | yes |
| 139 | `saber_thermal_fuse` | Thermal Fuse blade | Rare | Creyon | yes |
| 140 | `saber_senate_glass` | Senate Glass blade | Uncommon | Creyon | yes |
| 141 | `saber_holonet_feed` | Holonet Feed blade | Uncommon | Creyon | yes |
| 142 | `saber_holo_map` | Holo-Map blade | Uncommon | Creyon | yes |
| 143 | `saber_cockpit_console` | Cockpit Console blade | Uncommon | Creyon | yes |
| 144 | `saber_code_cylinder` | Code Cylinder blade | Common | Creyon | yes |
| 145 | `saber_cortosis_weave` | Cortosis Weave blade | Common | Creyon | yes |
| 146 | `saber_tatooine_dust` | Tatooine Dust blade | Common | Creyon | yes |
| 147 | `saber_trench_run` | Trench Run blade | Common | Creyon | yes |
| 148 | `saber_ryloth_dawn` | Ryloth Dawn blade | Common | Creyon | yes |
| 149 | `saber_power_coupling` | Power Coupling blade | Common | Creyon | yes |
| 150 | `saber_kyber_filament` | Kyber Filament blade | Common | Creyon | yes |
| 151 | `saber_frosted_viewport` | Frosted Viewport blade | Common | Creyon | yes |
| 152 | `saber_holo_stylus` | Holo-Stylus blade | Common | Creyon | yes |
| 153 | `saber_sabacc_card` | Sabacc Card blade | Common | Creyon | yes |
| 154 | `saber_geonosis_hive` | Geonosis Hive blade | Common | Creyon | yes |
| 155 | `saber_mygeeto_frost` | Mygeeto Frost blade | Common | Creyon | yes |
| 156 | `saber_bogano_moss` | Bogano Moss blade | Common | Creyon | yes |
| 157 | `saber_kef_bir_sea_glass` | Kef Bir Sea Glass blade | Common | Creyon | yes |
| 158 | `saber_jedha_lantern` | Jedha Lantern blade | Common | Creyon | yes |
| 159 | `saber_corellian_pearl` | Corellian Pearl blade | Common | Creyon | yes |
| 160 | `saber_targeting_display` | Targeting Display blade | Uncommon | Creyon | yes |
| 161 | `saber_raxus_scrap` | Raxus Scrap blade | Common | Creyon | yes |
| 162 | `saber_pasaana_stone` | Pasaana Stone blade | Common | Creyon | yes |
| 163 | `saber_smoked_transparisteel` | Smoked Transparisteel blade | Common | Creyon | yes |
| 164 | `saber_batuu_dusk` | Batuu Dusk blade | Common | Creyon | yes |
| 165 | `saber_alderaan_velvet` | Alderaan Velvet blade | Common | Creyon | yes |
| 166 | `saber_varykino_blossom` | Varykino Blossom blade | Mythical | Sol | yes |
| 167 | `saber_thousand_cranes` | Thousand Cranes blade | Unique | Sol | no |
| 168 | `saber_hearth` | Hearth blade | Unique | Sol | no |
| 169 | `saber_mystery` | Mystery blade | Legendary | Sol | no |
| 170 | `saber_vulptex_crystalfire` | Vulptex Crystalfire blade | Mythical | Creyon | no |
| 171 | `saber_zeffo_thunderchorus` | Zeffo Thunderchorus blade | Mythical | Creyon | no |
| 172 | `saber_moraband_erasure` | Moraband Erasure blade | Mythical | Creyon | no |
| 173 | `saber_wellspring_triad` | Wellspring Triad blade | Mythical | Creyon | no |
| 174 | `saber_chirrut_whills` | Chirrut Whills blade | Rare | Creyon | no |
| 175 | `saber_atollon_gathering` | Atollon Gathering blade | Mythical | Creyon | no |
| 176 | `saber_wobani_projection` | Wobani Projection blade | Rare | Creyon | no |
| 177 | `saber_iridonia_rite` | Iridonia Rite blade | Legendary | Creyon | no |
| 178 | `saber_rishi_ocean_flow` | Rishi Ocean Flow blade | Legendary | Creyon | no |
| 179 | `saber_scipio_awakening` | Scipio Awakening blade | Legendary | Creyon | no |
| 180 | `saber_nal_hutta_spicevine` | Nal Hutta Spicevine blade | Rare | Creyon | no |
| 181 | `saber_korriban_ragnos` | Korriban Ragnos blade | Mythical | Creyon | no |
| 182 | `saber_ossus_scroll` | Ossus Scroll blade | Legendary | Creyon | no |
| 183 | `saber_kiros_echani` | Eshan Echani blade | Rare | Creyon | no |
| 184 | `saber_toydaria_gale_streamers` | Toydaria Gale Streamers blade | Rare | Creyon | no |
| 185 | `saber_rattatak_crimson_surge` | Rattatak Crimson Surge blade | Rare | Creyon | no |
| 186 | `saber_hosnian_daybreak` | Hosnian Daybreak blade | Common | Creyon | no |
| 187 | `saber_rakata_infinite` | Rakata Infinite blade | Legendary | Creyon | no |
| 188 | `saber_tatooine_krayt_spine` | Tatooine Krayt Spine blade | Mythical | Creyon | no |
| 189 | `saber_lahmu_sandveil` | Lah'mu Sandveil blade | Legendary | Creyon | no |
| 190 | `saber_dromund_kaas_vitiate` | Dromund Kaas Vitiate blade | Mythical | Creyon | no |
| 191 | `saber_utapau_whirlwind` | Utapau Whirlwind blade | Mythical | Creyon | no |
| 192 | `saber_umbara_gloaming` | Umbara Gloaming blade | Legendary | Creyon | no |
| 193 | `saber_tython_jedaii` | Tython Je'daii blade | Rare | Creyon | no |
| 194 | `saber_crait_salt_flats` | Crait Salt Flats blade | Rare | Creyon | no |
| 195 | `saber_sullust_magma` | Sullust Magma blade | Legendary | Creyon | no |
| 196 | `saber_kijimi_snowfall` | Kijimi Snowfall blade | Rare | Creyon | no |
| 197 | `saber_saleucami_turbolaser` | Saleucami Turbolaser blade | Legendary | Creyon | no |
| 198 | `saber_wayland_gravity_press` | Wayland Gravity Press blade | Mythical | Creyon | no |
| 199 | `saber_onderon_blazing_strike` | Onderon Blazing Strike blade | Uncommon | Creyon | no |
| 200 | `saber_dantooine_grassland` | Dantooine Grassland blade | Uncommon | Creyon | no |
| 201 | `saber_bracca_rust_spark` | Bracca Rust blade | Common | Creyon | no |
| 202 | `saber_mandalore_mythosaur` | Mandalore Beskar blade | Common | Creyon | no |
| 203 | `saber_nevarro_ash` | Nevarro Ash blade | Common | Creyon | no |
| 204 | `saber_vandor_whiteout` | Vandor Whiteout blade | Common | Creyon | no |
| 205 | `saber_bendu_equilibrium` | Bendu Equilibrium blade | Legendary | Creyon | no |
| 206 | `saber_lehon_maelstrom` | Lehon Maelstrom blade | Mythical | Creyon | no |
| 207 | `saber_zonama_starlight` | Zonama Starlight blade | Legendary | Creyon | no |
| 208 | `saber_bastila_battle_meditation` | Bastila Battle Meditation blade | Legendary | Creyon | no |
| 209 | `saber_cantonica_ascension` | Cantonica Champagne blade | Rare | Creyon | no |
| 210 | `saber_nightsister_ichor` | Nightsister Ichor blade | Legendary | Creyon | no |
| 211 | `saber_nihilus_hunger` | Nihilus Hunger blade | Mythical | Creyon | no |
| 212 | `saber_rodia_toxic_bloom` | Rodia Toxic Bloom blade | Legendary | Creyon | no |
| 213 | `saber_glee_anselm_rockpools` | Glee Anselm Rockpools blade | Rare | Creyon | no |
| 214 | `saber_polis_massa_asteroid` | Polis Massa Asteroid blade | Legendary | Creyon | no |
| 215 | `saber_lotho_minor_lavaglass` | Lotho Minor Lavaglass blade | Legendary | Creyon | no |
| 216 | `saber_agamar_copper` | Agamar Copper blade | Uncommon | Creyon | no |
| 217 | `saber_sorgan_lake` | Sorgan Lake blade | Uncommon | Creyon | no |
| 218 | `saber_iego_lightglow` | Iego Lightglow blade | Rare | Creyon | no |
| 219 | `saber_sluis_grey` | Sluis Grey blade | Common | Creyon | no |
| 220 | `saber_arkanis_teal` | Arkanis Teal blade | Uncommon | Creyon | no |
| 221 | `saber_peridea_convergence` | Peridea Convergence blade | Legendary | Creyon | no |
| 222 | `saber_byss_force_storm` | Byss Force Storm blade | Legendary | Creyon | no |
| 223 | `saber_lira_san_starglass` | Lira San Starglass blade | Legendary | Creyon | no |
| 224 | `saber_mimban_marshlight` | Mimban Marshlight blade | Rare | Creyon | no |
| 225 | `saber_zygerria_cleave` | Zygerria Whip blade | Rare | Creyon | no |
| 226 | `saber_jabiim_siege` | Jabiim Siege blade | Uncommon | Creyon | no |
| 227 | `saber_pantora_sunrise` | Pantora Sunrise blade | Common | Creyon | no |
| 228 | `saber_mon_cala_coral` | Mon Cala Coral blade | Common | Creyon | no |
| 229 | `saber_tund_sorcerers` | Tund Sorcerers blade | Legendary | Creyon | no |
| 230 | `saber_haruun_kal_wildheart` | Haruun Kal Wildheart blade | Legendary | Creyon | no |
| 231 | `saber_kalevala_darksaber` | Kalevala Darksaber blade | Legendary | Creyon | no |
| 232 | `saber_ruusan_thoughtbomb` | Ruusan Thoughtbomb blade | Legendary | Creyon | no |
| 233 | `saber_garel_corona` | Garel Corona blade | Uncommon | Creyon | no |
| 234 | `saber_trandosha_scorekeeper` | Trandosha Scorekeeper blade | Uncommon | Creyon | no |
| 235 | `saber_wroshyr_rootwood` | Wroshyr Rootwood blade | Legendary | Creyon | no |
| 236 | `saber_iktotch_foresight` | Iktotch Foresight blade | Legendary | Creyon | no |
| 237 | `saber_vardos_phrik` | Vardos Phrik blade | Legendary | Creyon | no |
| 238 | `saber_kuat_shipwright` | Kuat Shipwright blade | Rare | Creyon | no |
| 239 | `saber_kowak_silkweb` | Kowak Silkweb blade | Rare | Creyon | no |
| 240 | `saber_nar_kreeta_lightspeed` | Nar Kreeta Lightspeed blade | Rare | Creyon | no |
| 241 | `saber_manaan_tidebreaker` | Manaan Tidebreaker blade | Rare | Creyon | no |
| 242 | `saber_cato_neimoidia_gilded` | Cato Neimoidia Gilded blade | Rare | Creyon | no |
| 243 | `saber_dxun_beastfang` | Dxun Beastfang blade | Rare | Creyon | no |
| 244 | `saber_fondor_weldsparks` | Fondor Forge blade | Rare | Creyon | no |
| 245 | `saber_plazir_lanterns` | Plazir Lanterns blade | Uncommon | Creyon | no |
| 246 | `saber_coruscant_neon` | Coruscant Skylanes blade | Legendary | Creyon | no |
| 247 | `saber_savareen_starfall` | Savareen Starfall blade | Uncommon | Creyon | no |
| 248 | `saber_hijarna_colours` | Hijarna Opal blade | Uncommon | Creyon | no |
| 249 | `saber_kef_bir_waves` | Kef Bir Waves blade | Rare | Creyon | no |
| 250 | `saber_shili_thornfang` | Shili Thornfang blade | Rare | Creyon | no |
| 251 | `saber_florrum_brigand` | Florrum Brigand blade | Uncommon | Creyon | no |
| 252 | `saber_abafar_pallor` | Abafar Pallor blade | Uncommon | Creyon | no |
| 253 | `saber_teth_leafdrift` | Teth Leafdrift blade | Uncommon | Creyon | no |
| 254 | `saber_malastare_podrace` | Malastare Podrace blade | Common | Creyon | no |
| 255 | `saber_anaxes_clonefront` | Anaxes Clonefront blade | Rare | Creyon | no |
| 256 | `saber_lola_sayu_watchbeam` | Lola Sayu Watchbeam blade | Common | Creyon | no |
| 257 | `saber_quell_embers` | Quell Embers blade | Uncommon | Creyon | no |
| 258 | `saber_skako_pressure` | Skako Pressure blade | Uncommon | Creyon | no |
| 259 | `saber_cerea_meadow` | Cerea Meadow blade | Common | Creyon | no |
| 260 | `saber_corulag_slate` | Corulag Slate blade | Common | Creyon | no |
| 261 | `saber_bothawui_amber` | Bothawui Amber blade | Common | Creyon | no |
| 262 | `saber_hapes_rose` | Hapes Rose blade | Common | Creyon | no |
| 263 | `saber_ithor_verdant` | Ithor Verdant blade | Common | Creyon | no |
| 264 | `saber_mirial_saffron` | Mirial Saffron blade | Common | Creyon | no |
| 265 | `saber_zeltros_violet` | Zeltros Violet blade | Common | Creyon | no |
| 266 | `saber_socorro_ochre` | Socorro Ochre blade | Common | Creyon | no |
| 267 | `saber_muunilinst_brass` | Muunilinst Brass blade | Common | Creyon | no |
| 268 | `saber_dorin_pewter` | Dorin Pewter blade | Common | Creyon | no |
| 269 | `saber_moon` | Moon blade | Mythical | Creyon | no |
| 270 | `saber_kathol_rift` | Kathol Rift blade | Legendary | Creyon | no |
| 271 | `saber_fondor_mass_driver` | Fondor Mass Driver blade | Legendary | Creyon | no |
| 272 | `saber_exar_kun_temple` | Exar Kun Temple blade | Mythical | Creyon | no |
| 273 | `saber_naga_sadow_alchemy` | Naga Sadow Alchemy blade | Legendary | Creyon | no |
| 274 | `saber_orbalisk_carapace` | Orbalisk Carapace blade | Legendary | Creyon | no |
| 275 | `saber_ziost_silence` | Ziost Silence blade | Legendary | Creyon | no |
| 276 | `saber_nathema_deadworld` | Nathema Deadworld blade | Legendary | Creyon | no |
| 277 | `saber_amphistaff_serpent` | Amphistaff Serpent blade | Legendary | Creyon | no |
| 278 | `saber_vergence_scatter` | Vergence Scatter blade | Mythical | Creyon | no |
| 279 | `saber_voxyn_scream` | Voxyn Scream blade | Legendary | Creyon | no |
| 280 | `saber_ysalamir_hush` | Ysalamir Hush blade | Legendary | Creyon | no |
| 281 | `saber_rakghoul_plague` | Rakghoul Plague blade | Legendary | Creyon | no |
| 282 | `saber_taris_undercity` | Taris Undercity blade | Mythical | Creyon | no |
| 283 | `saber_telos_restoration` | Telos Restoration blade | Rare | Creyon | no |
| 284 | `saber_malevolence_ion_beam` | Malevolence Ion Beam blade | Rare | Creyon | no |
| 285 | `saber_massassi_altar` | Massassi Altar blade | Rare | Creyon | no |
| 286 | `saber_sith_tremor_sword` | Sith Tremor Sword blade | Legendary | Creyon | no |
| 287 | `saber_eye_of_aldhani` | Eye of Aldhani blade | Uncommon | Creyon | no |
| 288 | `saber_ferrix_foundry` | Ferrix Foundry blade | Rare | Creyon | no |
| 289 | `saber_kenari_crash` | Kenari Crash blade | Uncommon | Creyon | no |
| 290 | `saber_sarlacc_pit` | Sarlacc Pit blade | Uncommon | Creyon | no |
| 291 | `saber_cloud_city_dusk` | Cloud City Dusk blade | Uncommon | Creyon | no |
| 292 | `saber_ring_of_kafrene` | Ring of Kafrene blade | Uncommon | Creyon | no |
| 293 | `saber_hypergate_drift` | Hypergate Drift blade | Rare | Creyon | no |
| 294 | `saber_angel` | Angel blade | Mythical | Creyon | no |
| 295 | `saber_bardotta_calm` | Bardotta Calm blade | Common | Creyon | no |
| 296 | `saber_alzoc_ice` | Alzoc Ice blade | Common | Creyon | no |
| 297 | `saber_fest_steppe` | Fest Steppe blade | Common | Creyon | no |
| 298 | `saber_ansion_plains` | Ansion Plains blade | Common | Creyon | no |
| 299 | `saber_sernpidal_dust` | Sernpidal Dust blade | Common | Creyon | no |

A client ignores an id it does not know. Both catalogues' tests pin this exact list
(ids, names, order), so a change to one fails until the other follows.

What each looks and sounds like, in words (the values and the sounds themselves are
the hub's, described in its `scripts/blade_skin_sounds.py` and `saber_skin_sounds.py`):

- **Sun**: a white-gold core in an orange corona graded from red at its rim to gold
  inside, granules drifting along it, tall flame tongues licking out of its edge and
  bright flares running from the hilt to the tip; since 11/10/2026 prominences (thick,
  smooth gold loops slowly leaving the blade and falling back), rising sparks, and, for
  the clients that read the additions file, a four-ray glint turning at the tip and heat
  haze bending what is behind the blade ([Options](#options): each can be switched
  off); a warm flickering light; a solar roar. Since 11/10/2026 (Sol's request), for the
  clients that read the later additions file, the glint is two, a smaller companion
  opposite it, circling the blade 9 units below the tip (`orbit`: 4.5 units out, 0.45
  turns a second, tilted 0.35 rad; rays 6.5 long instead of 9); the part is "Orbiting
  glints", the same `glint` id, so a wearer's choice stays.
  Since 20/10/2026 (Sol: blade 001 should be the best) it is a signature shader
  ([rendering.md](rendering.md#blade-lab-signature-shaders), `blade_signatures/sun.glsl`)
  drawn over the same file: the star breathes slowly (about five seconds), dark sunspots
  drift along the core, coronal rain falls back down glowing loops beside the blade (beads
  of plasma sliding down each side, since 20/10/2026 in place of the straight streamers
  Sol did not like), four prominences rise off it, a solar wind of pale motes rides to
  the tip inside the widest warm aura of any blade (halved in brightness the same day, Sol:
  the bloom was too strong); it lights as a sunrise
  (deep red warming to white gold, a white edge climbing the blade, then a dawn flare at
  the emitter) and goes out as a sunset (cooling to deep red as it slides in); a swing
  swells the corona and the wind, and a clash is a total eclipse's diamond ring: a dark
  moon slides across the blade where it was hit, a thin corona ring flares round it and
  one blinding diamond of light breaks out at its edge before the moon slides off. Its light is the strongest of
  any blade (`[1.5, 0.85, 0.3]`, breathing with the star), and its sounds are deeper
  (48 Hz hum instead of 60, a longer ignition over a low rumble). Its parts: "Sunspots and
  corona" (new), then "Prominences", "Sparks", "Glint" and "Haze" (the old `prominences`,
  `sparks`, `glint` and `haze` ids: a wearer's choices stay) and the swing and clash bursts.
- **Moon blade** (2/11/2026, update 20 of Creyon's Star Wars round: the Sun is a blade of fire; its sister is cold polished silver with no core of light, updated in 21 at his wish: no moon texture, no eclipse on the blade. The moon bursts into pieces that roam the blade until they fuse again. Its six parts are Living moon, Moon glitter, Moonlight, Night sky, Shattered moon and Falling stars, and three extras make nine. The history of update 14: the Lunar
  Eclipse is gone, with its black core, purple moon, shatter and waves, and its Eclipse trail
  extra; the eclipse's lines below are its history. Update 15: its own **falling stars**, stars falling out of its night round the whole blade, spiralling in and bursting into stardust as they reach it (no other blade has them), and its sounds about 4 dB quieter. In the eclipse's place the **moon tide**, the sister of
  the Sun's coronal rain: where the moon passes, the blade swells toward it and drops of silver
  light lift off it toward the moon, hang and fall back glinting. Its glitter sheets went from
  nine to four, its bright stars from seven to four, its moondust from 0.6 to 0.35 a unit, its
  stream's spirals and grains were thinned (its frame rate: Creyon saw 60 a second). (26/10/2026, named to match the Sun blade, Creyon 28/10/2026; Creyon's own blade, the Sun's night-time sister, and a wolf's: Creyon's
  character is a wolf and his colour purple, his model's eyes): a signature shader
  ([rendering.md](rendering.md#blade-lab-signature-shaders), `blade_signatures/moon.glsl`)
  over its own file, and a chroma ([Chromas](#chromas)): its glow takes the wearer's saber
  colour (its own hue a pale lavender), and so does every purple thing it draws, the sheen,
  the stone's sheen, the stream and spirals, the claws, the
  moon's eclipse shadow and the swing's coloured moondust (Creyon's wish: "RGB", 26/10/2026),
  while the moon's lit face keeps its own colours (`uOwn`, less the shadow's share). The blade
  is a silver core in its glow, with a soft purple light breathing round both its ends, the tip
  and where it leaves the emitter (in depth, so the same from every side; lilac moonlit, deeper
  purple in the eclipse). As the eclipse's shadow creeps down it, its glow is drawn into it
  with the shadow, narrowing to a fine edge of moonlight round a pure black core, so in totality
  only the black blade is left (Creyon). An unseen body round the core (a capsule, `mo_stone`)
  hides the moon, glints, grains and stars behind it (Creyon could see through the moon). (For a
  round the blade was drawn as a moonstone crystal, six-sided and pointed, then round, black
  glass in the eclipse; Creyon wanted the solid tube gone.) Round the core, the moon's grey
  seas drift in it, a soft lilac crown round its tip, moonlight glitters on its glow as on water,
  and a **sheen** of billowing blue-violet light glides up and down the glow, flaring on
  a swing. (An enchantment of two winding strands and rising crescents, a moonflood and a wave of
  the moon's phases were tried on the glow and taken out: the effects fought each other.) Round it, from
  its file, on its "Moon glitter" switch, a purple **moon mist** rising and curling off it
  (`wisps`); it has no afterimages, its own effects took their place (Creyon's wish for
  something new). In true depth round the blade, on the same switch, two flows of glints, each
  glint a bright head with a soft tail of light, twinkling and now and then flashing a four-ray
  star. **Moonlit**, glistening glitter floods off the whole blade in every direction from the
  moment it lights, drifting slowly out (half a minute) along gently rising curves and fading
  into the distance, about 7 units out (14 at first, which reached into other players' faces), to a ragged reach that wanders so it never ends at a
  line; few and soft at rest and kept off the blade's glow so the blade shows (Creyon:
  a snowstorm hid the blade), many more on a swing or a clash, fading in rather than popping.
  **As the eclipse comes** the glitter glides into the blade (eased) as the shadow creeps down it:
  every glint slides in towards the blade from where it is, gathering, brightening a moment and
  vanishing into it, sparkling into stars as it goes, so in totality only the blade, the moon
  and the night sky are left (Creyon's wish); **as the moonlight returns** they burst back out
  sparkling, from the hilt up as the shadow draws back, in a flood that spreads to their places
  and flows on; above the tip they follow the tip, so nothing bursts out over the top (it did). It is one flow squeezed in towards the axis by the shadow at each height (a
  glint whose place is u out shows at u times the squeeze, so each pixel still looks up one
  cell; an earlier version had a second, purple flow drawn in during the eclipse). Their speeds
  and curves never change (they scroll with time, so a change would make them race, as it did). They are cells on nine sheets turned evenly
  round the blade's axis and turning slowly, and on five discs across it for a view from above
  or below along the blade (Creyon: from above nothing showed); a sheet or disc seen edge on
  fades and the others carry the picture; each cell is long along its glint's path, and all its
  light fades out before the cell's edges, so a tail or star ray is never cut (at most 14 looks
  a pixel, about 9 from most views; none of the sheets when the ray runs along the blade, where
  they are all edge on). Its room is 10 units (`@room 10`), so the quad's soft edge
  fade starts past them. The flat
  pictures round it (the night sky, haze) are placed where the view
  ray passes closest to the blade's axis, so they are the same from every side and from above.
  (Moonlit veils on its sides, curtains like an aurora, were taken out: they read as hard lines.
  Earlier versions had motes on shells round the blade, which read as rings with straight edges, then
  Sol's drifting motes from its file, which Creyon found cheap; both are gone.) Its glow is kept low so the effects show. Its own effect is the **living moon**: a detailed moon (1.6
  units) going round the upper blade on its own path, a tilted ellipse 2.6 to 3.4 units out that
  slowly wanders up and down the blade (`sig_orbit_solid`), its seas,
  craters at two sizes and one young crater's bright rays in relief, lit by the blade itself
  from its height, leaning toward the eye, so it waxes and wanes between full (behind the
  blade) and a half moon (in front of it), always turning the same face to the blade, as the
  real moon does to the Earth, and solid (only the blade's own core in front of it hides it). A soft **moonbeam** falls from it onto the blade below, and a
  **moon stream**: grains of purple moonlight pour from it along a curve that ends exactly
  where two **spirals** begin, and ride on round the blade on them, one down to the hilt and
  one up to the tip (2.3 units out where they start, tightening to under 1), the same 32 grains
  all the way, bright, with a soft glow along their path, purple as they leave the moon and lilac
  and white as they fade; as the eclipse comes the spirals and their glow are drawn into the
  blade with the glitter, winding in and fading as they go, and come back out with it after
  (Creyon) (the first spirals, bright thin ribbons, looked angular and were
  not joined to the stream)
  (skipped wherever the view ray passes far from it). Its great moment is the **Lunar
  Eclipse** (a 39 s cycle, the same for every blade):
  14 s moonlit; over 5 s the Earth's shadow slides across the moon, a wide grey penumbra ahead
  of the umbra, whose soft edge glows a little brighter and whose colour deepens from grey to a
  rich purple with the seas and craters still showing, while the shadow creeps down the blade
  from the tip with a long soft front and two lines of light, one silver and one purple, wind
  round it at the front, spinning; the blade's glow is drawn into it as the front passes,
  leaving a pure black core (`sig_solid`; nothing glows under it, so its bloom cannot grey it)
  with a fine edge of moonlight, and the glitter,
  spirals and stars are drawn into it too.
  In the last three seconds glowing purple cracks race over the moon, cell by cell, beating like
  a quickening heart, and as totality comes (with a purple wave down the blade) the
  **moonshatter**: the moon explodes in a flash of purple-white light with a turning sparkle,
  and 28 small jagged splinters of moon rock with glowing purple edges and 72 specks of glowing
  moondust blow out and are quickly carried round the blade (within about a second), each on its own slow path at its own height
  along the whole blade and a few round its tip, tumbling, twinkling and breathing; for the 14 s of totality they are
  the only light left round the black blade (Creyon: the explosion as the eclipse comes, staying
  exploded, longer timers, only the moon's purple pieces left). As the moonlight returns, over
  6 s, a silver wave runs back up the blade, the shadow draws back up it with the two lines,
  the pieces cool to silver as they are drawn home and fuse, a soft glow gathering where the
  moon will be, and it is whole again, glowing silver a moment, while the glitter, spirals and
  stars flood back out. (Earlier the cracks spread through totality and the moon burst only as it
  ended, its pieces back within seconds; a wolf's glowing eyes and a wolf constellation
  beside the blade were tried and taken out at Creyon's wish.) Switched off, the blade stays moonlit. The **night sky**
  round it, moving (Creyon: the night sky should move too): three fields of tiny twinkling stars rising slowly past the blade, the nearer ones faster so they move against each other (thinning out softly away from the blade, so
  it shows no edge), the faint band of the galaxy with its dark lanes, four bright stars with
  a turning four-ray sparkle wheeling slowly round the blade as they rise, shooting stars on two clocks and a faint drifting haze. A swing or
  the wearer's movement sends glittering moondust off into the world, floating up
  ([rendering.md](rendering.md#falling-blossoms); its shards of moon rock were taken out,
  Creyon not wanting stones falling out of the saber; lightning crawling on it was tried and
  taken out, Creyon wanting no thunder). It
  lights as nightfall (the stars come out from the hilt up as it grows, a soft silver flare
  rises off the tip and the moon glides out from it to its orbit) and goes out as dawn; a clash
  is a wolf's strike: three claw slashes of purple light rake across the blade where it lands,
  with a burst of moonlight and dust, a ring of moonlight sweeping out round the blade from
  where it struck and a burst of 26 tailed glints flying out from there in every direction, white
  turning lilac and purple as they slow and fade, while the glitter round the blade swells for a
  moment (Creyon: better clash effects) (`@moments own`, so it goes without the shared mythical
  moments). Its light is moonlight (`[1.0, 1.0, 1.08]`). Its parts: "Living moon" (the moon and
  its moonbeam and stream), "Moon glitter" (the glitter, the mist and the moondust
  its swings and movement shed), "Moonlight", "Night sky", "Shattered moon", "Falling stars"; the swing and clash
  bursts. The
  eclipse cycle is the same for every blade, so a staff's two halves and two sabers change
  together (it took each blade's seed, and they went out of step). Its own
  sounds (`sound/sjk/creyon/moon/`, synthesized, peaceful, Creyon having found the first ones
  irritating after a while, the next too heavy, then wanting them more beautiful and quieter,
  and on and off clearer: the hum a soft, slow pad moving between A minor 9 and F major 7, a
  seamless 6 s loop, quiet; on, a cascade of glimmer (soft high tones swelling in and out)
  climbing through the notes over a soft harp (Karplus-Strong plucks) as the pad swells in;
  off, the glimmer falling back down as the pad fades, ringing out to silence; swings, only a
  light breath of air and the calm glimmer after it, no low hum (they sounded heavy)) ship in the hub's
  pack; without them the stock saber is heard. Creyon's card (SJK Alpha 002) brings it while
  held, as Sol's brings the Sun (`unlockables::CARD_BLADES`).
- **Storm**: a white-blue core in a violet-to-electric-blue corona that crackles; a few
  thin, jagged white-blue lightning arcs at a time, struck again and again at random
  places, leaving the blade, bulging out to one side and coming back further along,
  crawling toward the tip and re-shaping as they fade; some leap from the tip into the
  air instead; white sparks spray from round the tip; a fast-flickering light; an
  electric buzz with sizzle and crackle.
- **Void**: a hollow blade: the inside of its glow is black (nothing is added there, so
  the world shows through), its rim violet, with a faint deep-violet line for a core
  and a dark round mouth at the tip; slow tendrils licking inward and rare slow magenta
  surges; pale star specks drift in from outside and fade as they reach it; a low,
  slowly beating drone. Since 11/10/2026 (Sol's request), for the clients that read the
  additions file, warped space (a slow haze bending what is behind the blade), a gravity
  pulse beating with the drone (0.5 a second) and dark-violet afterimages when it swings;
  and, for the clients that read the later additions file, two small black holes (the
  second smaller) circling the blade 10 units below the tip, 5 units out, 0.3 turns a
  second: black horizons that hide the blade behind them, a violet photon ring and
  swirl, and the scene round them bent into a ring. Each is a part
  ([Options](#options)).
- **Frost**: a pale ice-white core ending in a long pointed icicle tip, in an icy blue
  corona with a soft cold haze; faint crystalline veins by the core that re-form now
  and then; thin frost shards glinting as they trail toward the hilt and round the tip;
  a soft hum under shimmering bell tones and tiny ice tinkles.
- **Prism**: a white core whose fringe and corona run through the rainbow along the
  blade, the spectrum sliding slowly with time and the rim a different hue from the
  inside (refraction bands), sharp bright bands drifting along it and coloured
  sparkles; its light turns through the colours with the blade's middle; a chord in
  detuned voices under a slowly sweeping resonance.
- **Unstable**: a raw red blade whose glow's edge is frayed by fast noise and whose
  length now and then falters (cut short for an instant), sparks spitting off it, a
  hard-flickering light; a torn, stuttering buzz under dense crackle.
- **Molten**: a dim, dark-red core split by bright orange lava cracks (veins) running
  through the core and the inside of the glow, embers dripping off it and falling as
  gravity pulls them, whichever way the blade is held; a bubbling, sizzling rumble.
- **Spectral**: a pale, see-through blue-white blade with a wide soft glow and faint
  wisps; swinging, it leaves up to four fading afterimages of its glow where it was;
  since 11/10/2026 its outline wavers like smoke, souls (soft lights) drift slowly round
  it, its rim turns pale violet, its light is dimmer and colder and, for the clients
  that read the additions file, wisps of smoke rise off it (a sheet over a blade held
  level, a plume over the tip of one held upright) and an echo of it sways beside it,
  even held still ([Options](#options): each can be switched off); an airy pad under a
  whisper, with echoes.
- **Glitch**: a teal blade whose red and blue part from its green; on random draws
  blocks of it jump sideways and flash, now and then the whole blade; a gated buzz with
  digital blips.
- **Hologram**: a hollow blue projection drawn as a wireframe (two side lines and rings
  round the blade) with scan lines running along it, jittering sideways now and then;
  a clean tone, a projector whine and a scan tick.
- **Runic**: a dim gold core with glyphs scrolling toward the tip that spell its
  wearer's name (SJK's own stroke glyphs, one per letter); a chord of fifths with bell
  glints.
- **Chameleon**: a pale blade that takes the colour of the light where it is (the map's
  light grid, saturation raised), so it changes from place to place and map to map; a
  body whose resonance slowly morphs.
- **Banner**: its wearer's team colour in team games (red, blue), silver outside a team,
  rippling like a flag; a warm buzz and fluttering cloth.
- **Heartbeat**: a warm crimson blade that beats with a slow double heartbeat (lub-dub,
  once a second), brightening and swelling with each beat, its light beating with it;
  a drone with a heartbeat.
- **The nine plain ones** (14/10/2026, after Creyon's "Saber ideas" page; the hub's
  `scripts/common_skins.py` writes their files, with the stock saber's sounds): Common,
  a colour and one gentle movement: **Tidal** (a teal wave surging hilt to tip),
  **Circuit** (thin cyan, sharp data pulses), **Aurora** (green drifting through teal
  and violet along the blade), **Echo** (sky blue, a bright ring rippling out);
  Uncommon, a colour and particles: **Nebula** (indigo with twinkling stars),
  **Cinder** (coal orange shedding embers), **Venom** (acid green, dripping),
  **Sakura** (pink, petals drifting down), **Firefly** (warm green, blinking motes).
- **Twelve more plain ones** (14/10/2026, same script): Common **Slate** (steel grey),
  **Amber** (honey gold), **Lilac** (violet, a slow hue drift), **Mint** (pale green wave),
  **Coral** (salmon pulse), **Ivory** (cream); Uncommon **Glacier** (ice blue, frost
  glints), **Dusk** (orange into violet, turning), **Ruby** (red, sparks), **Amethyst**
  (purple, twinkling flecks), **Jade** (sea green, slow hue, specks), **Moonlight**
  (silver blue, drifting motes).
- **The Blade Lab's first six** (15/10/2026, after Creyon's "Blade Lab" page; the hub's
  `scripts/lab_skins.py` writes their files, their sounds `blade_skin_sounds_third.py`),
  drawn with the fifth set of sections ([Blade-skin files](#blade-skin-files)). Creyon
  ranked them a tier lower each; Sol found that too low, so each is one tier up from his.
  Uncommon **Neon** (a chroma: a glass tube in six sections, one stuttering out now and
  then, gas lines running up it; a mains hum) and **Peppermint** (red and white stripes
  winding up a candy blade, sugar glints; a rounded tone and bells); Rare **Soap Bubble**
  (a nearly clear blade in a soap film's sliding colours, bubbles drifting off; an airy
  hum and pops) and **Lava Lamp** (a violet column where wax rises in blobs that merge and
  split, pools at the hilt and tip; a warm drone and bubbling); Mythical **Reactor**
  (blue plasma squeezed by containment rings marching up it, overheating orange now and
  then, sparks; a deep throb) and **Bleeding Kyber** (a white blade where a red
  corruption climbs from the hilt through cracks and is pushed back, red drops falling;
  a choir over a dark undertone). Parts their wearer may switch off: Neon's stutter and
  gas lines, Peppermint's sugar, the Bubble's bubbles, the Reactor's rings, overheat,
  sparks and throb, the Kyber's corruption and drops.
- **The Blade Lab's next seven** (15/10/2026, same scripts), drawn with the sixth set
  ([Blade-skin files](#blade-skin-files)), again a tier above Creyon's: Rare **Sketch**
  (a blade drawn in pencil, outlines and hatching redrawn eight times a second; pencil
  scratches); Legendary **Hourglass** (pinched glass, glowing sand pouring from the upper
  chamber into the lower over eight seconds, then a flash; a sand hiss and ticks) and
  **Fiber Optic** (28 glass fibres fanning from the hilt, each ending in a slowly
  changing point of light; a glassy shimmer); Mythical **Serpent** (a golden serpent
  coiled round an emerald blade, slithering, passing in front and behind, its head by the
  tip opening its jaws, red eyes; a hiss), **Monarch** (an amber blade where butterflies
  rest, open their wings and fly off and back, pollen; fluttering), **Phoenix** (living
  flame, burning feathers falling, wings of fire unfolding from the hilt; roaring fire)
  and **Quantum** (two ghost copies a side at other angles drifting apart and collapsing
  back with a flash, interference fringes, a flickering cloud; a beating chord). Not
  built from Creyon's descriptions: anything that needs the swing (strands lagging,
  butterflies scattering, copies collapsing on a swing: the shader does not know the
  blade's motion, so they follow their own clocks) and the Phoenix's ash-to-fire
  ignition (it does not know when the blade was lit).
- **The Blade Lab's last three** (15/10/2026, same scripts), drawn with the seventh set,
  solid parts blended over the glow: Legendary **Comic** (a chroma: a thick black outline
  round its flat glow, the glow outside printed in halftone dots; little pops) and
  **Quicksilver** (a liquid mirror reflecting a bright sky over a dark ground, ripples
  running up it, beads rolling to the tip, silver drips, barely a glow; wet metallic
  wobbles); Mythical **Obsidian Edge** (a flat, pointed blade of black glass with a
  glossy streak sliding along it and white lightning crackling off its edges; a low
  growl and crackle). Not built: the Comic's speed lines and the words that burst out on
  ignite, swing and clash (the shader knows no swing or clash), and a true reflection of
  the map in the Quicksilver (it reflects a made-up surrounding in the light where the
  blade is).
- **Varykino Blossom** (21/10/2026, blade 166, Mythical, Kushina's idea; built by Sol's
  side as a signature, `blade_signatures/varykino_blossom.glsl`): the Naboo Blossom's
  Mythical sibling, after the spring gardens of Naboo's lake retreat. A soft blossom-pink
  blade in a locked palette (not a chroma, as Kushina asked), pale petals of light opening
  and fading in its glow, lavender and white mist curling off it, five-petal blossoms
  drifting down round it, a branch of dark cherry wood with a bud and one open flower
  growing from where the blade leaves the hilt, and a ring of petals on a clash. Swung, it
  sheds real blossoms that flutter to the ground behind it and settle there
  ([rendering.md](rendering.md#falling-blossoms)), where the Spectral leaves afterimages.
  Sol's addition the same day, after the Sakura Droplet of Sekiro (FromSoftware; studied
  from Sol's own copy of the game for its look only, nothing of the game's is in SJK): a
  pearl of nacre hangs at its tip, rose and mauve with growth rings, a rainbow sheen at its
  rim and a white heart, in a soft rose light (a clash bursts it into the petals and it
  grows back), and the blade has the same pearly sheen along its edges. Each of the five is
  a part its wearer may switch off. It borrows Royal Naboo's sounds
  (the skins pack is near its 16 MiB cap). Not built from her idea: the wooden hilt, which
  waits for custom hilts.
- **Thousand Cranes** (25/10/2026, blade 167, Sol's, the first [Unique](#unique-shaders) blade; a
  signature, `blade_signatures/thousand_cranes.glsl`): a thousand paper cranes (senbazuru), the old
  Japanese gift of thanks and good wishes, for a blade Sol gives by hand. A warm silk-white
  blade with the fine fibres of washi paper in its light (not a chroma), a red silk thread
  wound round it from the hilt to near the tip (flat, as Sol's pieces are: bright where it
  passes in front, faint behind the blade's light), and eight origami cranes in coloured
  paper (vermilion, indigo, sakura, jade, gold, sky, plum, cream) circling it in 3D as they
  rise up it, beating their wings and gliding by turns, each wearer's flock in its own order
  of colours (`uSeed`). A golden crane, the thousandth, hovers over the tip in a warm light.
  The cranes fold into being as the blade lights, spread wider on a swing, and a clash
  startles a ring of them out round the blade; a swing also releases real paper cranes that
  climb away from it ([rendering.md](rendering.md#falling-blossoms)). Five parts its wearer
  may switch off. Its crane is drawn from triangles (`tc_tri`, `tc_crane`): the far wing in
  shadow, the body with its crease, the neck and tail, the head folded down and the lit near
  wing with its fold, turned to fly the way it goes across the view and narrowed when it
  flies toward or away from it. Its own sounds, `sound/sjk/sol/thousand_cranes/` (soft and
  airy: a warm hum, paper-wing swings), are in the hub's skins pack. Kintsugi was the
  first idea, but the Healed Kyber (106) already is Creyon's Kintsugi; nothing in the
  catalogue had cranes.
- **Hearth** (25/10/2026, blade 168, Sol's, the second [Unique](#unique-shaders) blade; a
  signature, `blade_signatures/hearth.glsl`): a home fire. A warm amber blade of glowing
  coals, their hearts bright and the cracks between them dark red, each breathing on its
  own, with embers rising off it (its file's `embers`); it kindles up from the hilt as it
  lights and cools to a deep red as it goes out. Tongues of flame lick up both its edges
  (flat, as Sol's flames are), the air over it shimmers (its file's `haze`), and the fire
  is a blaze, taller and hotter, coals popping off it in a flash and a few sparks. A ring of
  seven hearthstones stands round its emitter in the firelight (in 3D, lit on the side
  toward the blade), ash drifts down off it, and its swings shed ash that settles on the
  floor ([rendering.md](rendering.md#falling-blossoms)). Its own sounds,
  `sound/sjk/sol/hearth/` (a warm hum with a log fire's crackle in it), are in the hub's
  skins pack. Not the Phoenix's fire: no plume or rebirth, a bed of coals and tongues of
  flame instead. It was first built to grow with the days its holder kept it (flames at a
  month, the blaze at six, the stones at a year); Sol dropped that the same day, the stages
  too alike to tell apart, so every holder has it whole (the growth code is on the branch
  `personal/hearth`, commit `c8e8bf26`).
- **Speed of the round** (1/11/2026, updates 17 and 18): the round's blades looked like slow motion beside Sol's (their time factors had a median of 0.4 against 1.1). A blade's motion now runs at its own `@speed` (`uTime`), tuned per blade; its cycles and moments keep real time (`uClock`), so a period named in a description is in real seconds. A woven bolt's writhing speed followed the storm's phase and raced, the bug update 16 fixed elsewhere; it is steady now. Update 18 also reworked Trayus (straight point, fine outline, scything arcs, spectral blades), Korriban (cursed flames as real fire), Zeffo (blue to silver), Rakata (six Star Forge suns, a visible god ki body, forked threads), Dromund Kaas (slimmer effects, a smooth run of colour), Bendu (a seam and two winding currents), Kalevala (beskar shards, black lightning) and Ruusan (a dawn haze). Update 19 replaced the bare look of the Mythicals with something each blade is: Zeffo a storm in glass with tomb stones, Trayus with orbiting scythes, Dromund Kaas with devoured worlds, Kalevala a true sword whose shards form a crown, Ruusan with its thought bomb, Tund with its mirage, Ossus with drifting pages, Atollon with gathered lights. Tund went to Legendary; Haruun Kal and Lira San to Rare; Atollon, Ossus and Utapau to Mythical. Update 20, after Creyon named Tund, Prism, Unstable, Neon, Abyssal, World Shift and the beskar blades as what he means: the blade itself must be the effect. Moon is rock, Zeffo a standing wave, Dromund Kaas flesh and veins, Ossus one brush stroke; Crait no longer has a red front climbing it, and Wobani's block of mist is gone. Update 21: `halo(d, w)` is 1 for every negative `d`, so a wave written `halo(y - front, w)` lit everything below its front and snapped back when it wrapped, the sudden resets Creyon saw; every round blade takes `abs()` there now. New blades in place of Ziost Dark Anvil and Nur Shadowflame: Tatooine Krayt Spine and Byss Force Storm. Nihilus (a wound in the blade, no mask at his wish), Zeffo, Vardos Phrik and Nightsister Ichor were rebuilt; Garel Corona went to Common and Trandosha Scorekeeper to Rare. Update 22: Lotho Minor Furnace and Vulptex Spiritfire became Lotho Minor Lavaglass and Vulptex Crystalfire; Hyperlane became a new blade named Fondor Mass Driver under its old unlock id `saber_hyperlane`, so those who own it keep it; Nihilus, Rodia and Polis Massa were rebuilt. rxRibbon stays on Moraband, Bendu, Lehon, Utapau and Tund; 35 other blades draw their paths with `xBeads`, `xDust`, `xComets` or `xPuffs`. Crooked lines writhe with `xZigT` instead of jumping to a new random path. Peridea and Zonama went to Legendary, Nightsister Ichor to Mythical with RGB, and the round's Uncommons have no RGB. Update 23, each Legendary a body of its own: Kuat a ship on the slip, Nar Kreeta racing bars, Wroshyr bark, Iktotch crossing ripples, Kowak a silk cocoon, Mimban bubbling mud. Update 24 (Creyon's last round of the list): seen along the blade, as its wearer does, short zigzags and lumps fold into steps that look like tears (Zeffo's "glitching thunder"), so what runs along a round blade now stays long and slow. Sixty-odd Commons, Uncommons, Rares and Legendaries that were a bright rod with dust became a material of their own (slate, pewter, amber, coral, silk, hide, kolto, a night sky): the blade itself is the effect, written from a short recipe. Ossus Scroll lost its black ink for gold, Utapau Whirlwind became a tornado with no bright strands, Nihilus Hunger lost its arms and eats the light round it, Crait Salt Flats became a chroma, Atollon Gathering a Legendary. Trail styles 28 to 33 are new (Bone, Hunger, Hex, Gilded, Gale, Salt) and style 23 is now Byss's violet storm. Every round blade's file now has the colours its header names, which is what the trail and the Collection icon read. The round's sounds (not in the repository) got a swing made of each blade's own material. Update 25 (Creyon: beside the Legendaries already in the client the new ones looked cheap and bare): a round blade is a bright saber again, with its material showing round a white heart and large effects round it that read from afar, as Reactor's or Big Bang's do; effects rise with the blade as it lights and sink as it goes out; tips are round, not pointed (the Moon keeps its point); nothing small sits in a blade (Kuat's lattice, Iktotch's beads, Vardos's discs, Polis Massa's rocks and Byss's wreckage went); Zeffo's sideways forks, which looked like cuts, went; Wellspring Triad and Kuat Shipwright became chromas. Update 26: `blade_place` takes a place's side from the ray and the axis, not the quad's `blade.x` (under a close perspective a slice of the blade drew mirrored; pixel-identical otherwise); Rakata's trail follows its surge; a round blade's body starts softly in the emitter.rs`, `pipeline_cache/` in the player's folder, one file a blade, Vulkan only) and those not yet there are compiled one by one in the background, so a blade compiles once. Update 27 (Creyon's list after testing): extras on Trayus (Energy aura), Chirrut (cloth, motes always faintly seen), Fondor Mass Driver, Wroshyr and Kalevala; Nihilus's dying worlds slower; the Moon's purple at its ends removed; effects start a little above the hilt. Update 28: Trayus Scythe's run is straight and only its tip curves; Wayland, Sullust, Iridonia, Atollon, Crait, Scipio and Wobani had the stock glow they lean on calmed (it wobbled quickly); the Moon's body takes a chroma's colour, and its shatter costs less (fewer pieces, and a ray skips those far from its height).
- **Creyon's Star Wars round** (29/10/2026, blades 170 to 269, `feat/blade-round-anime`): a
  hundred blades on Creyon's ideas, every one named for a place, a person or a piece of Star Wars
  lore that fits what it shows, its name sharing no word with another blade's (1/11/2026: the
  round began on ideas from anime fights; Creyon asked for every anime name and word to go).
  Their building blocks are the `rx` helpers at the end of
  [lab.glsl](../crates/sjk-viewer/src/blade_signatures/lab.glsl) (grid-free flow, life and cycle
  timing, ribbons, curtains, writhing bolts, star clouds, mist, rocks) and, since update 12, the
  `sh` shells after them (a pattern read where the view ray crosses a cylinder round the blade:
  the same from every side, a few look-ups a pixel instead of a loop of 3D pieces, no outline;
  [rendering.md](rendering.md) has them). Update 12 (1/11/2026, Creyon's test of update 11): the
  blades he liked keep their looks (their floating ribbons, smoke, rocks and motes are what make
  them); Trayus Scythe (on the energy blade in the video he sent) and Zeffo Thunderchorus (from 125
  to 30 frames a second up close) were redrawn on shells; Korriban Ragnos lost its spiked flames
  from the hilt and its heavy 3D bolts, Moraband Erasure its aurora curtains (a figure showed in
  them), Tund Sorcerers its black orbs, Mimban Marshlight its leaf-shaped flames, Zygerria
  Cleave its curved slashes, Ruusan Thoughtbomb and Garel Corona their flame tongues, each for
  sweeping ribbons and soft fire or smoke on shells; Kalevala Darksaber, Haruun Kal Vaapad (now
  Vaapad's violet), Polis Massa Asteroid and the new Legendaries got more round them; Bendu
  Equilibrium's ribbons drift slower; Dume Wolfspirit's tails and the lab's ribbons, tongues,
  helices, mists and star clouds cost far less with the same pictures (rendering.md: Utapau
  Whirlwind from 8 to 1.2 times the Sun's cost). Update 13 (1/11/2026, Creyon: the Mythicals did
  not look Mythical, every tier looked empty, effects felt like slow motion, some spun too fast and
  a line showed at the hilt): each blade keeps its look and gains a floating layer of its own
  round it (`sw_extra`, `tools/sw_extras.py`: ribbons, comets, star clouds, embers, lightning,
  flakes, glints, streaks; five or six pieces on a Mythical), the round's clock keeps real time
  (`tCalm`) while its slow spins were slowed where they are set, and every effect fades in over the
  blade's first units (`hiltFade`). Kitsune became Vulptex Spiritfire (a fox again, with a roar and
  a scream), Haruun Kal Vaapad became Haruun Kal Wildheart (no rocks), Moraband lost its cracks in
  space. Update 14 (1/11/2026, Creyon: the Mythicals looked alike, all with the same sweeping
  ribbons, and did not reach the game's own Mythicals): the floating layer left every Mythical,
  and each has a part of its own instead, drawn for it alone (Bendu's balanced stones, Lehon's
  stretched starlight, Zeffo's Lichtenberg flash, Moraband's unmade space, Korriban's bleeding
  crystal, Rakata's newborn stars, Ziost's molten runnels, Lah'mu's fused glass, Dromund Kaas's
  endless storm, Zonama's starseeds, Peridea's annihilation, Tund's threads of fate, Haruun Kal's
  fireflies, Kalevala's edge glints, Ruusan's kyber fragments, Trayus's severed slivers, Vulptex's
  spirit lanterns, Nur's black cinders; the solid ones on `xBall` and `xGem` in lab.glsl); Nur
  became black shadowfire with solid black flames; Trayus Scythe's point is straight and even;
  Bendu's steam and Lehon's swirl run at a calm speed and the round's `(tCalm()*0.8)` terms became
  `uTime*0.4`; Vulptex's tails are smaller and its glow tighter (its frame rate). Update 15
  (1/11/2026, Creyon: the effects should cover the whole blade and move round it, more dramatic on
  the Mythicals, on every tier): every blade of the round has a swarm of its own round its whole
  length (`xSwarm` in lab.glsl: motes, sparks, petals or leaves, stars, flakes, drops, bubbles or
  wisps on their own spiral paths, in its palette, chosen for its theme), the Mythicals two; Bendu
  has six stones; Nur's cinders (too like Obsidian Edge's shards) became a fire whirl; Trayus
  Scythe rests straight with the point of Creyon's picture, a torn black aura round it, and
  "Reshaping" and "Straight point" switch its change to the scimitar point (its wisps and threads,
  and its sparks, slivers and storm, now share a switch each: a skin offers six switches at most,
  `MAX_OPTIONS`, the parts riding on six bits); on the other Mythicals the swarm shares the switch
  of update 14's own part, and a blade with six parts of its own has no switch for its swing and
  clash bursts (they stay on). Update 16 (1/11/2026, Creyon's full review with ten videos): the
  swarms left every blade (the same small particles on all of them); effects that sped up in a
  phase (Lehon's opening, Bendu's moving, Utapau's tightening, Lah'mu's vortex, Dromund Kaas's
  heart) multiplied the whole clock by a changing speed, so they raced ahead by the blade's whole
  age as the phase rose: they now add the phase's time (`rxCycleInt`, `rxMistSpinP` in lab.glsl),
  and lightning rates no longer change with a moment; Zeffo is its first version again with
  lightning to its point and no rain; Dromund Kaas has new purple and white lightning over its whole
  length with a surge racing up it; Ziost's effects all come out of its heat (no sparks); Korriban's
  smoke shells gave way to sharper heat and strong Sith lightning; Rakata has the Star Forge's suns;
  Crait was redrawn away from Bleeding Kyber; Umbara reaches its point; Trayus's aura is a fine
  outline round its point too; the Moon has its full glow and a corona. Effects by tier
  (Creyon's counts): six or more for a Mythical, five or more for a Legendary, four for a Rare, three for a Uncommon, two for a Common. Most Mythicals and many others are chromas.
  - **Zeffo Thunderchorus** (Mythical, blade 171, `blade_signatures/zeffo_thunderchorus.glsl`): A thunderstorm held in a blade: blue-white plasma with lightning alive on its surface. Cold glints hang in the air round it, and great bolts strike in from the air under a flashing thunderhead. Every 20 seconds the great storm: the sky lights and the bolts come together.
  - **Vulptex Crystalfire** (Mythical, blade 170, `blade_signatures/vulptex_crystalfire.glsl`; Dume Wolfspirit until 1/11/2026): The vulptex, the crystal foxes of Crait, and the spirit fire in them: a long crystal shard in clean facets, fire moving inside it, veils of gold and rose foxfire rising round it and folding over each other. Every 16 seconds the howl: a surge climbs the blade and the foxfire stands tall.
  - **Moraband Erasure** (Mythical, blade 172, `blade_signatures/moraband_erasure.glsl`): The power to erase, as the old Sith Lords dreamed it on Moraband: deep violet with a void-black heart, its edges coming apart into violet dust, long ribbons of violet light round it. Holes of nothing open in the air beside it and close. Every so often a pulse of erasure climbs it and the light round it dims.
  - **Korriban Ragnos** (Mythical, blade 181, `blade_signatures/korriban_ragnos.glsl`; Korriban Black Flash until 1/11/2026): The tomb of Marka Ragnos on Korriban: obsidian black with a crimson rim and red cracks breathing through it, torn black-red flames up its edges, crimson lightning crawling over it and forked bolts leaping off it. Red light bleeds down its face and drips away. Every so often Ragnos wakes and lightning bursts out all round.
  - **Rakata Infinite** (Mythical, blade 187, `blade_signatures/rakata_infinite.glsl`; Rakata God Ki until 1/11/2026): The power of the Infinite Empire: deep sapphire with a white-hot heart in a towering crown of blue flame, blue lightning crawling up it. Three small suns of the Star Forge hang round it, each pouring itself into the blade in a stream of plasma until it is spent. Every 30 seconds it passes its limit and the flames turn crimson.
  - **Tatooine Krayt Spine** (Mythical, blade 188, `blade_signatures/tatooine_krayt_spine.glsl`): The spine of a krayt dragon, still alive: ivory vertebrae and thorns moving like a serpent, fire beating between the bones, embers climbing from its joints. A stream of golden sand as thick as a body winds up round it, in the haze of two small suns wheeling slowly round it. Every 18 seconds the roar.
  - **Lah'mu Sandveil** (Legendary, blade 189, `blade_signatures/lahmu_sandveil.glsl`; Lah'mu Dune Coffin until 1/11/2026): a blade of living sand that guards its bearer: golden sand packed hard by the Force, streams of sand spiralling up round it and sand trickling off its edges. Every so often the coffin closes: a whirling vortex of sand wraps the blade in a grinding shell that bursts outward in a cloud of grains. Lit, sand pours up out of the emitter; let go, it pours away. Parts: Packed sand, Sand streams, Falling sand, The coffin, Dust haze. Its trail: `@trail sand`. A chroma.
  - **Dromund Kaas Vitiate** (Mythical, blade 190, `blade_signatures/dromund_kaas_vitiate.glsl`; Dromund Kaas Ultra Ego until 1/11/2026): the hunger of a destroyer who ate worlds to live: a heavy blade of dark flesh and glowing magenta veins, each heartbeat travelling up it as a swell, its edges torn. Purple and white lightning crawls over it, and the air gives up its light: sparks kindle far out and are pulled in, stretching into streaks.
  - **Bendu Equilibrium** (Mythical, blade 205, `blade_signatures/bendu_equilibrium.glsl`; Bendu Autonomous Will until 1/11/2026): the body moving on its own, the Force in perfect balance: silver-white light so clear it seems hollow, a cool blue heart and a slow rainbow sheen, silver steam curling up off it. Every so often it moves on its own: its afterimages step out to either side and the steam blazes. Two currents, one pale and one faint, wind round it in balance. Parts: Silver body, Silver steam, Floating light, Haze shimmer, Moving on its own, Balanced stones. Its trail: `@trail equilibrium`. A chroma.
  - **Lehon Maelstrom** (Mythical, blade 206, `blade_signatures/lehon_maelstrom.glsl`): A tear in space bound into a blade that draws everything in: blue-black flowing inward in slow spirals with a bright cyan edge, motes of light spiralling in and vanishing into it. Every so often the maelstrom opens wide, the motes rush in and a flash bursts out. Far stars stretch into threads as they fall.
  - **Zonama Starlight** (Legendary, blade 207, `blade_signatures/zonama_starlight.glsl`): Zonama Sekot, the living world that fled through the stars: a dark blade holding its night sky, huge living filaments of warm gold growing up through it, and golden stars standing in two even columns either side of it, turning slowly with it.
  - **Peridea Convergence** (Legendary, blade 221, `blade_signatures/peridea_convergence.glsl`): the two halves of the Force meeting: red and blue streams spiral up from either side and meet in a deep violet where space bends, violet lightning crawls and every 24 seconds the convergence blazes and bursts. A white mote and a violet one spiral together and vanish in a flash. Parts: Split body, Twin streams, The imaginary, Bent space, Convergence, Annihilation. Its trail: `@trail convergence`.
  - **Byss Force Storm** (Mythical, blade 222, `blade_signatures/byss_force_storm.glsl`): The Emperor's Force storm over Byss: the blade is the funnel, a dark rope opening to a wide mouth, its walls one turning storm cloud lit from inside by blooms of lightning, three soft arms of the storm winding up and out, living lightning leaping from them, wind and wreckage round it, the eye burning at its crown; every 24 seconds the storm surges.
  - **Tund Sorcerers** (Legendary, blade 229, `blade_signatures/tund_sorcerers.glsl`; Tund Six Paths until 1/11/2026): the sorcery of the Sorcerers of Tund, masters of illusion: a pale gold blade that will not hold still to the eye, a slow wave bending its picture as it lies; a calm violet flame on its outline, ribbons of violet and gold, threads of fate strung round it. Every 26 seconds the sorcery gathers and it blazes gold.
  - **Haruun Kal Wildheart** (Legendary, blade 230, `blade_signatures/haruun_kal_wildheart.glsl`; Haruun Kal Vaapad until 1/11/2026): the wild heart of Haruun Kal's jungles, a power that cannot be held back: a blazing violet blade, a storm of violet energy rising round it, long violet ribbons sweeping round it, violet lightning lashing through it and leaping off it, motes of violet light round it and jungle leaves drifting past; every 20 seconds it erupts. Fireflies swarm round it, blinking. Parts: Raging body, Green storm, Lashing lightning, Rising rocks, Eruption, Fireflies. Its trail: `@trail vaapad`. A chroma.
  - **Kalevala Darksaber** (Mythical, blade 231, `blade_signatures/kalevala_darksaber.glsl`): The black blade of the Mandalorian kings: a flat black sword with a crackling white edge and a slow silver gleam over its polished face. Tall licks of white fire stream up both edges with black flame standing in them, black smoke and wind round it, black lightning. Every 22 seconds its edge blazes and light runs up it.
  - **Ruusan Thoughtbomb** (Legendary, blade 232, `blade_signatures/ruusan_thoughtbomb.glsl`; Ruusan Westflame until 1/11/2026): the light of the Jedi bound into the Force on the field of Ruusan: a silver-white blade mended with seams of old gold, a soft dawn haze and splinters of kyber round it. Every 24 seconds all its light drains from both ends into one point, leaving dark steel and gold; it bursts, the white runs back along it and a golden dawn breaks.
  - **Wellspring Triad** (Mythical, blade 173, `blade_signatures/wellspring_triad.glsl`; Taris Fusion Trinity until 1/11/2026): The source of the Force the Priestesses guard: three living streams, gold, silver and sea-green, twist round each other up a dark blade and flare white where they cross. Out in the air the same three streams turn in slow beads of light.
  - **Chirrut Whills** (Rare, blade 174, `blade_signatures/chirrut_whills.glsl`; Chandrila Silver Instinct until 1/11/2026): Chirrut Imwe saw nothing and knew everything. Each tap of his staff sends a wave of sound up the dark blade, and the motes in the air round it exist only while the wave passes: touched by it they light, then go dark again. Rain falls, seen only the same way.
  - **Atollon Gathering** (Mythical, blade 175, `blade_signatures/atollon_gathering.glsl`): the energy of everything round it, lent to one blade: great soft spheres of the sky's light drift in on long curves, each drawn out into a thread that pours into the blade. Sky-blue streams flow in beside them, and over a slow cycle its upper half fills, brightening, until it lets it all go in a soft wave.
  - **Ossus Scroll** (Mythical, blade 182, `blade_signatures/ossus_scroll.glsl`; Ossus Ink Brush until 1/11/2026): A blade painted in one breath of a brush loaded with gold: warm rice-paper with veins of gold leaf, great strokes of gold drawn round it up to its very point and drying, gold stars standing in two even columns in slow incense and leaf falling through them. As it lights, a brush of gold runs up with it.
  - **Utapau Whirlwind** (Mythical, blade 191, `blade_signatures/utapau_whirlwind.glsl`): The winds of Utapau's sinkholes wound into a blade: the body is a tornado, a rope at the hand opening to a funnel, its banded walls spinning, great arms of wind winding up round it, dust thrown off in slow clouds, thin lightning in the storm. Every 16 seconds the spiral tightens.
  - **Umbara Gloaming** (Legendary, blade 192, `blade_signatures/umbara_gloaming.glsl`; Umbara Shadow Bind until 1/11/2026): Umbara, the shadow world where no sun reaches: dark indigo fading to nothing, rows of bioluminescent lume spots waking along it, silent lightning far off inside it, a cold mist hanging round it from hilt to point.
  - **Tython Je'daii** (Rare, blade 193, `blade_signatures/tython_jedaii.glsl`; Tython Sage Mode until 1/11/2026): A standing stone from Tython where the Je'daii learned balance: moss-grown rock with green life forcing up through cracks in it, a bright moon and a dark moon crossing its face in turn, pollen rising round it.
  - **Crait Salt Flats** (Rare, blade 194, `blade_signatures/crait_salt_flats.glsl`): White salt over Crait's red earth, drifting along the blade, a burning furrow winding through it and scoring it open; red dust climbs off it in slow plumes and fine salt skims round it. Every 14 seconds a gust.
  - **Bastila Battle Meditation** (Legendary, blade 208, `blade_signatures/bastila_battle_meditation.glsl`): Bastila Shan's battle meditation: a deep blue stillness that breathes, one slow swell passing through it, while points of light stand all round it in perfect ranks and move as one: out on the breath, in on the breath. Nothing here is out of step.
  - **Cantonica Champagne** (Rare, blade 209, `blade_signatures/cantonica_ascension.glsl`; Vjun Golden Ascension until 1/11/2026): The casinos of Canto Bight: a blade of struck gold, pale champagne bubbles streaming up inside it, a sweep of white light passing over the metal as if under chandeliers. Round it the soft lights of the casino floor drift out of focus, long streams of gold silk wind up it, and pale beams of chandelier light turn slowly through the air.
  - **Nightsister Ichor** (Legendary, blade 210, `blade_signatures/nightsister_ichor.glsl`): magick of the Nightsisters of Dathomir, thick as sap: a stream of glowing green ichor over a black stem, marbled with pale light. Heavy drops form on its edge, run down, stretch and fall away; witch-mist clings to it from hilt to point and spirit lights rise through it. Every 20 seconds the magick wells up and slowly eases back.
  - **Nihilus Hunger** (Mythical, blade 211, `blade_signatures/nihilus_hunger.glsl`; Nihilus Morgul until 1/11/2026): Darth Nihilus, a wound in the Force that eats worlds: a hollow black blade with a thin red edge and red veins draining toward the hand. Long soft streams of stolen life bend in from all round it and sink into the blade, dark fire rises off it and a slow red mist is pulled in. When it feeds, everything is drawn in faster.
  - **Lira San Starglass** (Legendary, blade 223, `blade_signatures/lira_san_starglass.glsl`): The Shard-world of Lira San in dark glass: a night sky closed inside the blade, its stars crossing slowly from hilt to tip, each glinting as it turns.
  - **Mimban Marshlight** (Rare, blade 224, `blade_signatures/mimban_marshlight.glsl`; Mimban Blackflame until 1/11/2026): The mud-war of Mimban: wet black-green muck sliding slowly down the blade, sour marsh gas glowing where it breaks the surface, and soft wisps of light drifting round it, leading nowhere.
  - **Garel Corona** (Uncommon, blade 233, `blade_signatures/garel_corona.glsl`; Garel Cloakfire until 1/11/2026): the corona of Garel's sun caught round a blade: soft gold fire burning round it, long ribbons of sunlight sweeping round it, motes of light and rising heat. Parts: Sun corona, Corona fire, Chakra motes, Rising heat. A chroma.
  - **Trandosha Scorekeeper** (Uncommon, blade 234, `blade_signatures/trandosha_scorekeeper.glsl`; Trandosha Kaioken until 1/11/2026): Trandoshan scales running up the blade, overlapping and glinting, with a scored ritual count burning between them; a red mist hangs round it for the Scorekeeper.
  - **Wroshyr Rootwood** (Legendary, blade 235, `blade_signatures/wroshyr_rootwood.glsl`; Wroshyr Ent Grove until 1/11/2026): A living wroshyr tree of Kashyyyk: deep brown bark split by bright green sap, thick roots climbing round it and reaching out, broad leaves sprouting from them, spores drifting up through the green light.
  - **Iktotch Foresight** (Legendary, blade 236, `blade_signatures/iktotch_foresight.glsl`; Iktotch Infinity until 1/11/2026): The Iktotchi see what is coming: pale ghost blades stand round the real one, each a stroke yet to happen, sliding into place and fading, while light flows upward in the clear blue blade before its time. Every 20 seconds the vision opens and all of them stand at once.
  - **Vardos Phrik** (Legendary, blade 237, `blade_signatures/vardos_phrik.glsl`; Vardos Mithril until 1/11/2026): The phrik alloy of an electrostaff: dark blue-grey metal that takes a blow and holds it, a heavy current crawling over it in bright crackling lines, arcs snapping off into the air and winding back.
  - **Kuat Shipwright** (Rare, blade 238, `blade_signatures/kuat_shipwright.glsl`; Kuat Emperor Gilt until 1/11/2026): The great orbital drydocks of Kuat: a long smooth warship hull in gold-lit steel with lights running along its length, slim scaffold beams turning slowly round it, welders throwing sparks and small arcs from it. Every 24 seconds the drive test: the whole blade flares.
  - **Kowak Silkweb** (Rare, blade 239, `blade_signatures/kowak_silkweb.glsl`): The jungle-spider silk of Kowak wrapped round and round a blade: a woven skin with a slow sheen sliding over it and a green glow of venom showing through. Soft loose silk winds round it in the air, in a pale web-mist, dew glinting on it and fat green drops of venom running slowly down.
  - **Nar Kreeta Lightspeed** (Rare, blade 240, `blade_signatures/nar_kreeta_lightspeed.glsl`; Nar Kreeta Flashstep until 1/11/2026): The jump to lightspeed held in a blade: gold star-drive plasma rushing up it, the stars around it drawn out into long lines that run past, blue ahead and gold behind. Every 18 seconds the jump: all the lines stretch and flash and the blade blazes.
  - **Wobani Projection** (Rare, blade 176, `blade_signatures/wobani_projection.glsl`; Wobani Mist Double until 1/11/2026): A trick smuggled out of the Wobani labour camps: a teal blade with two thin projections of itself standing off it, swaying out of step and flickering in bands as they climb. Now and then a soft puff of white smoke bursts from it and the copies blaze; white smoke ribbons curl off it as it moves.
  - **Iridonia Rite** (Legendary, blade 177, `blade_signatures/iridonia_rite.glsl`; Iridonia Eight Gates until 1/11/2026): The rite of the Zabrak of Iridonia: a blade of living energy where eight gates open one by one, each a band of blazing light, the last at its very tip, while its colour climbs from jade through blue to crimson. Flames of that colour stand up its sides, power crackles through them and steam rises, thicker with every gate.
  - **Rishi Ocean Flow** (Legendary, blade 178, `blade_signatures/rishi_ocean_flow.glsl`): The storm seas of Rishi in a blade: deep blue water climbing it with light netted through it, great waves rearing out of it and curling back, bright spray thrown high and falling, a soft sea mist round it all.
  - **Eshan Echani** (Rare, blade 183, `blade_signatures/kiros_echani.glsl`; Kiros Gentle Fist until 1/11/2026): The white halls of the Echani on Eshan: polished marble veined in lilac, light moving through the veins as a duelist's guard moves, thin pale silk lifting round it.
  - **Toydaria Gale Streamers** (Rare, blade 184, `blade_signatures/toydaria_gale_streamers.glsl`): Toydaria's bog-wind: cloud racing fast up the blade with sun breaking through the gaps, thin pennants of colour snapping round it, grit in the air.
  - **Sullust Magma** (Legendary, blade 195, `blade_signatures/sullust_magma.glsl`; Sullust Lava Release until 1/11/2026): the molten rivers of Sullust shaped into a blade: dark crusted lava with magma flowing up through it, bubbles of magma swelling and bursting on it and drops of lava spattering off it. Parts: Lava flow, Magma bubbles, Spatter, Heat glow.
  - **Kijimi Snowfall** (Rare, blade 196, `blade_signatures/kijimi_snowfall.glsl`; Kijimi Snow Lotus until 1/11/2026): A snowy night on Kijimi: blue-black ice, frost feathering in from its edges and melting back, big soft flakes falling slowly all round it.
  - **Saleucami Turbolaser** (Legendary, blade 197, `blade_signatures/saleucami_turbolaser.glsl`; Saleucami Ki Cannon until 1/11/2026): A turbolaser's barrel: blue energy coiled round a hard white core, one heavy bolt after another climbing it, every shot lighting its whole length and leaving it as a streak.
  - **Wayland Gravity Press** (Mythical, blade 198, `blade_signatures/wayland_gravity_press.glsl`): The weight hidden under Mount Tantiss on Wayland: a heavy blade of liquid violet metal, its gold veins pressed slowly toward the hand. Torn stones lift off the ground and rise past it, light bends round it in long slow veils and gold dust sinks. Every so often the press comes down, and everything falls.
  - **Rodia Toxic Bloom** (Legendary, blade 212, `blade_signatures/rodia_toxic_bloom.glsl`): the swamps of Rodia in one blade: a thick poison-green stalk, ribbed and wet, purple veins climbing it. Pods swell out of it one after another, glowing from within, until they split and let go a cloud of yellow spores, then wither back to grow again. Spores drift off slowly, twinkling.
  - **Glee Anselm Rockpools** (Rare, blade 213, `blade_signatures/glee_anselm_rockpools.glsl`): A rock pool at Glee Anselm: sea stone wet with a sliding tide, sunlight netted over it, small bubbles rising from it, grains of sand turning in the water.
  - **Polis Massa Asteroid** (Legendary, blade 214, `blade_signatures/polis_massa_asteroid.glsl`): A blade of raw asteroid from the Polis Massa field: rough dark rock in one piece, cold blue light forcing through its cracks, a mist of ice crystals turning round it, and thin mining beams sweeping slowly past.
  - **Lotho Minor Lavaglass** (Legendary, blade 215, `blade_signatures/lotho_minor_lavaglass.glsl`): From Lotho Minor, the junk world whose fires never go out: a tall tube of dark glass holding molten rock. Soft blobs of melt rise slowly up it, stretch, join and part, cool at the top and sink back. Under an RGB its glass turns with the colours and every blob takes its own random colour.
  - **Zygerria Whip** (Rare, blade 225, `blade_signatures/zygerria_cleave.glsl`): A Zygerrian slaver's energy whip stiffened into a blade, its plaited coils showing, charge running along it and snapping off in short arcs.
  - **Manaan Tidebreaker** (Rare, blade 241, `blade_signatures/manaan_tidebreaker.glsl`): A blade of kolto from the deeps of Manaan: thick healing green-blue liquid with fat bubbles wobbling up inside it and out into the air, light from far above moving through it.
  - **Cato Neimoidia Gilded** (Rare, blade 242, `blade_signatures/cato_neimoidia_gilded.glsl`): A rose-gold blade worked with fine gold filigree that winds up it, a sheen sliding over the metal, thin gold leaf falling round it in slow turns.
  - **Dxun Beastfang** (Rare, blade 243, `blade_signatures/dxun_beastfang.glsl`): The hide of a Dxun beast pulled over a blade: orange fur barred with heavy black stripes, its flanks rising and falling, embers and warm jungle haze round it.
  - **Fondor Weldsparks** (Rare, blade 244, `blade_signatures/fondor_weldsparks.glsl`): Shipyard plate riveted edge to edge, three welding torches climbing its seams and throwing cascades of sparks that fall away past the hand.
  - **Plazir Lanterns** (Uncommon, blade 245, `blade_signatures/plazir_lanterns.glsl`): A summer evening on the terraces of Plazir-15: amber dusk in the blade, soft round lights swelling and climbing inside it, and more drifting up into the air around it and fading.
  - **Coruscant Skylanes** (Legendary, blade 246, `blade_signatures/coruscant_neon.glsl`): Coruscant at night in a blade: a dark tower lit from within by rushing lanes of magenta and cyan traffic, airspeeders streaking round it, the glow of the sign-lit level on its edges.
  - **Savareen Starfall** (Uncommon, blade 247, `blade_signatures/savareen_starfall.glsl`): The shore on Savareen at dusk: violet sky darkening up the blade with orange at its foot, and bright stars falling down past it, each with a long tail.
  - **Hijarna Opal** (Uncommon, blade 248, `blade_signatures/hijarna_colours.glsl`): The ruins on Hijarna: pale fortress stone, with opal fire moving under its surface in every colour, and fine dust sifting down its length.
  - **Kef Bir Waves** (Rare, blade 249, `blade_signatures/kef_bir_waves.glsl`): The wild sea round the Death Star wreck: dark waves climb the blade one behind another and break white along its edges, flinging spray that falls away.
  - **Shili Thornfang** (Rare, blade 250, `blade_signatures/shili_thornfang.glsl`): The montral horn of a Togruta: a long spiral of ivory narrowing to a sharp point, red hunter's markings running up it, grass and wind moving round it.
  - **Scipio Awakening** (Legendary, blade 179, `blade_signatures/scipio_awakening.glsl`): a warrior's first golden awakening on Scipio: a shimmering gold blade with a spiky aura of golden light standing up off its edges and golden dust rising off it. Parts: Spiky aura, Ki dust, Shimmer. A chroma.
  - **Rattatak Crimson Surge** (Rare, blade 185, `blade_signatures/rattatak_crimson_surge.glsl`): Rattatak's arenas: dark crimson with branching arteries, and each surge of the heartbeat runs up them all, a red breath coming off it.
  - **Onderon Blazing Strike** (Uncommon, blade 199, `blade_signatures/onderon_blazing_strike.glsl`): A blade on fire from within: slow rolling flame fills it, its edges licking outward, heat climbing off it.
  - **Dantooine Grassland** (Uncommon, blade 200, `blade_signatures/dantooine_grassland.glsl`): The plains of Dantooine: long grass bending in slow waves up the blade, patches of sun crossing it, seed fluff lifting away on the wind.
  - **Bracca Rust Spark** (Common, blade 201, `blade_signatures/bracca_rust_spark.glsl`): Hull plate off the Bracca scrap fields, flaking with rust; a cutter's torch crawls along it, throwing a fan of sparks.
  - **Agamar Copper** (Uncommon, blade 216, `blade_signatures/agamar_copper.glsl`): Weathered copper from Agamar's old wrecks, sea-green verdigris eating slowly along it, flakes of it falling away.
  - **Sorgan Lake** (Uncommon, blade 217, `blade_signatures/sorgan_lake.glsl`): The shallow krill ponds of Sorgan: still dark-green water, sun glints moving across it, and small blue-green lights hanging in it.
  - **Iego Lightglow** (Rare, blade 218, `blade_signatures/iego_lightglow.glsl`): The world of a thousand moons: violet glass with a soft milky glow moving through it, small warm lights circling it slowly.
  - **Jabiim Siege** (Uncommon, blade 226, `blade_signatures/jabiim_siege.glsl`): Jabiim, where it never stops raining: mud-gold metal streaked with running water, and rain falling straight down all round it.
  - **Florrum Brigand** (Uncommon, blade 251, `blade_signatures/florrum_brigand.glsl`): A pirate's blade off Florrum: iron daubed in orange with slanting black slashes, scratched and repainted, leaking soft dark exhaust.
  - **Abafar Pallor** (Uncommon, blade 252, `blade_signatures/abafar_pallor.glsl`; Abafar Bleach until 1/11/2026): The blank white world of Abafar: colourless stone, thin grey veins of rhydonium, and a pale haze with no shadow lying round it.
  - **Teth Leafdrift** (Uncommon, blade 253, `blade_signatures/teth_leafdrift.glsl`): The jungle of Teth: dark bark climbing the blade, shafts of canopy light sliding down it, and big leaves spinning slowly down past it.
  - **Malastare Podrace** (Common, blade 254, `blade_signatures/malastare_podrace.glsl`): A Malastare podracer laid along a blade: orange stripes rushing past at full throttle, heat shivering off it, blue thrust spitting out low down.
  - **Anaxes Clonefront** (Rare, blade 255, `blade_signatures/anaxes_clonefront.glsl`): White armour plate of the clone legions with a blue legion stripe down it and a marker light racing up it; blue tracer fire crosses round it.
  - **Lola Sayu Watchbeam** (Common, blade 256, `blade_signatures/lola_sayu_watchbeam.glsl`): The Citadel on Lola Sayu: dark iron, one red scanning bar travelling up it and down again, and one red searchlight turning round it.
  - **Quell Embers** (Uncommon, blade 257, `blade_signatures/quell_embers.glsl`): The bridge of the Quell after it fell: burnt metal with wide slow burns glowing through it, cinders falling past.
  - **Skako Pressure** (Uncommon, blade 258, `blade_signatures/skako_pressure.glsl`): A world under crushing pressure: bulges of squeezed light are forced up the blade from the hand, its width swelling and pinching as they pass, vapour leaking out.
  - **Nal Hutta Spicevine** (Rare, blade 180, `blade_signatures/nal_hutta_spicevine.glsl`; Nal Hutta Chakra Thread until 1/11/2026): A Hutt-world vine wound on itself, two dark stems twining up a sour green glow.
  - **Hosnian Daybreak** (Common, blade 186, `blade_signatures/hosnian_daybreak.glsl`): The sky at daybreak drawn up a blade: plum at the hand, rose in the middle, gold at the point, with the light rising through it and falling back.
  - **Mandalore Beskar** (Common, blade 202, `blade_signatures/mandalore_mythosaur.glsl`; Mandalore Ironsheen until 1/11/2026): Beaten Mandalorian beskar in long overlapping plates, hammer marks in its face and forge light moving slowly behind the seams.
  - **Nevarro Ash** (Common, blade 203, `blade_signatures/nevarro_ash.glsl`): A blade of cooled lava from Nevarro's black fields: charcoal crust, and hair-thin seams of fire crawling slowly through it.
  - **Vandor Whiteout** (Common, blade 204, `blade_signatures/vandor_whiteout.glsl`): A blizzard on Vandor packed into a blade: white snow streaming fast across it, blue shadows opening between the gusts.
  - **Sluis Grey** (Common, blade 219, `blade_signatures/sluis_grey.glsl`): The shipyard skies of Sluis Van: soft smoke-grey, with slow bright and dark cloud passing up it.
  - **Arkanis Teal** (Uncommon, blade 220, `blade_signatures/arkanis_teal.glsl`): A blade of still desert-lake water. Slow swells pass up its teal body and bend it a little, and their crests catch the light.
  - **Pantora Sunrise** (Common, blade 227, `blade_signatures/pantora_sunrise.glsl`): The frozen moon at first light: clear blue ice with a warm blush climbing it from the hand and fading back.
  - **Mon Cala Coral** (Common, blade 228, `blade_signatures/mon_cala_coral.glsl`): Soft coral from the reefs of Mon Cala, full of small dark pores, and a slow tide of light breathing through it.
  - **Cerea Meadow** (Common, blade 259, `blade_signatures/cerea_meadow.glsl`): A meadow on a stalk. Fine blades of grass stand up its length and lean together as the wind passes through them.
  - **Corulag Slate** (Common, blade 260, `blade_signatures/corulag_slate.glsl`): A blade of blue-grey slate, its leaves slanting along it, and between them cold light comes and goes in hairline seams.
  - **Bothawui Amber** (Common, blade 261, `blade_signatures/bothawui_amber.glsl`): A blade of old amber. Thick light creeps up it like honey, and dark specks hang caught inside.
  - **Hapes Rose** (Common, blade 262, `blade_signatures/hapes_rose.glsl`): A blade of rose silk. Long folds wind up it and drift apart and together, soft as cloth in a warm wind.
  - **Ithor Verdant** (Common, blade 263, `blade_signatures/ithor_verdant.glsl`): One long living leaf. A pale rib runs up its middle and veins fork off it to the edges, sap climbing through them.
  - **Mirial Saffron** (Common, blade 264, `blade_signatures/mirial_saffron.glsl`): Saffron-dyed cloth from Mirial wound tight round the blade in a soft slanted weave, one warmer thread drifting through it.
  - **Zeltros Violet** (Common, blade 265, `blade_signatures/zeltros_violet.glsl`): Zeltron violet, shot like silk: as the light moves over its round face it turns pink on one side and blue on the other.
  - **Socorro Ochre** (Common, blade 266, `blade_signatures/socorro_ochre.glsl`): A core cut through Socorro's desert: long bands of ochre and rust stacked up it, with sand drifting slowly over them.
  - **Muunilinst Brass** (Common, blade 267, `blade_signatures/muunilinst_brass.glsl`): Banker's brass from Muunilinst, long engraved lines running up it, with a band of polish rolling slowly along them.
  - **Dorin Pewter** (Common, blade 268, `blade_signatures/dorin_pewter.glsl`): A blade of brushed pewter, finely grained along its length, with one soft sheen sliding slowly up and down it.
  - **Kathol Rift** (Legendary, blade 270, `blade_signatures/kathol_rift.glsl`): The Kathol Rift, where the Aing-Tii walk through time: a blade twisted on itself, deep space on one face and liquid gold on the other, burning every colour where it turns. Its twist runs on, stops and runs backward. Ghosts of it hang beside it, comets of light chase round it, gold flame climbs it and motes fall upward.
  - **Fondor Mass Driver** (Legendary, blade 271, `blade_signatures/fondor_mass_driver.glsl`): The great guns of the Fondor shipyards: no beam but a gun, two steel rails with an open gap between them and capacitor rungs across it. Slugs of blue-white plasma are fired up the gap again and again, each lighting the rungs as it passes, thin arcs jumping across. Now and then a heavy round goes up and flashes from the muzzle.
  - **Exar Kun Temple** (Mythical, blade 272, `blade_signatures/exar_kun_temple.glsl`): The temple Exar Kun raised on Yavin 4 and bound his spirit to: a blade of black stone with burning lips and gold seams waking in slow waves. Six temple needles hang round it in the air striking it with lightning, ribbons of his spirit wind round it, and souls climb them with long tails.
  - **Naga Sadow Alchemy** (Legendary, blade 273, `blade_signatures/naga_sadow_alchemy.glsl`): Naga Sadow's forbidden alchemy at the boil: a blade of swallowing blood with bubbles climbing through it. Heavy ropes of blood are thrown out of its sides, arc through the air and fall back in, fumes roll off it and thick drops fall from it.
  - **Orbalisk Carapace** (Legendary, blade 274, `blade_signatures/orbalisk_carapace.glsl`): The orbalisks that armoured Darth Bane: dark shells packed over a burning body, the light running through the seams between them while each shell's eye of heat opens in turn. Lumps crawl along its outline, and lightning crawls over them as they feed.
  - **Ziost Silence** (Mythical, blade 275, `blade_signatures/ziost_silence.glsl`): Ziost as the Emperor eats it: pale stone split by black cracks, torn open in three wounds. Long rivers of life bend in to them from far off, an aurora of drained souls hangs round it and pieces of the world tear loose and spiral in. Then the silence: everything stops, and a cold shell is all that is left.
  - **Nathema Deadworld** (Mythical, blade 276, `blade_signatures/nathema_deadworld.glsl`): Nathema, where the Force was torn from every living thing: a charred black blade with fissures of dying fire. Great licks of dull flame leave it all along its length, black smoke rolls up it and a storm of embers and ash winds round it. Every so often a wave of death bursts all the way up.
  - **Amphistaff Serpent** (Legendary, blade 277, `blade_signatures/amphistaff_serpent.glsl`): A Yuuzhan Vong amphistaff, alive and awake: the whole blade hangs in slow serpent curves with scales, a pale belly and a swallow running up it. Venom drips from it, and when it strikes the curves whip fast and it spits a burst of venom.
  - **Vergence Scatter** (Mythical, blade 278, `blade_signatures/vergence_scatter.glsl`): A knot in the Force where the galaxy runs together: the blade is a window into deep space, with lips of blue and gold. Great clouds of nebula turn round it, long loops of light rise and land, far light winds in, and now and then it lets go in bursts of stars.
  - **Voxyn Scream** (Legendary, blade 279, `blade_signatures/voxyn_scream.glsl`): The voxyn's scream made visible: waves of sound chase up a violet body while soft waves of sound ripple outward through the air. Every eleven seconds it opens up and howls: the blade shudders and the sound reaches twice as far.
  - **Ysalamir Hush** (Mythical, blade 280, `blade_signatures/ysalamir_hush.glsl`): The forest of Myrkr, where the ysalamir silence the Force: a slim blade of living scales with two olbio branches winding round it, thick with bark, moss, hanging strands and glowing fronds that breathe. Rivers of fireflies circle it, a green aurora hangs in the air and a dark hush climbs the blade.
  - **Rakghoul Plague** (Legendary, blade 281, `blade_signatures/rakghoul_plague.glsl`): The rakghoul plague of Taris: sick yellow flesh full of black veins and raw sores. Boils swell slowly out of its outline until they burst in a spray of pus, spores sink off it and a low breath of plague hangs about the lower blade.
  - **Taris Undercity** (Mythical, blade 282, `blade_signatures/taris_undercity.glsl`): Taris at night, the city-world before Malak burned it: the blade is a black tower climbing in tiers, thousands of windows alight. Rivers of speeder traffic circle it in three dimensions, searchlights sweep through the smog and rain falls past. Then the Sith fleet fires: beams strike the tower, its lights die and fire bursts from it.
  - **Telos Restoration** (Rare, blade 283, `blade_signatures/telos_restoration.glsl`): The Telos restoration, life returned to a burned world: a clean white blade with green rising up it in a slow year. Living shoots climb round it opening leaves as they go, pollen drifts off them, then they turn gold, wither and shed, and spring starts again.
  - **Malevolence Ion Beam** (Rare, blade 284, `blade_signatures/malevolence_ion_beam.glsl`): The ion cannons of the Malevolence: a narrow channel with charges racing up it faster and faster, and plasma tendrils wandering off it to earth in the air. Every thirteen seconds the guns fire: the whole blade goes white, swells, and a storm of ions leaves the point.
  - **Massassi Altar** (Rare, blade 285, `blade_signatures/massassi_altar.glsl`): An altar of the Massassi on Yavin 4: rough stones stacked into a blade, each slowly shifting on the next, with fire burning in the gaps between them. Flames lick up from every gap, fire runs through old carved channels and dark smoke turns round it.
  - **Sith Tremor Sword** (Legendary, blade 286, `blade_signatures/sith_tremor_sword.glsl`): The Sith tremor sword, whose blade shakes to cut: a true sword of dark steel with a fuller and burning edges, its outline blurred by a hard fast tremble that gets worse as you swing. The air cracks round it, sparks shake loose and old rage rises round it.
  - **Eye of Aldhani** (Uncommon, blade 287, `blade_signatures/eye_of_aldhani.glsl`): The Eye of Aldhani, when the sky over the valley fills with falling light: a deep blue night in the blade with fixed stars in it, while long bright streaks come slanting down past it.
  - **Ferrix Foundry** (Rare, blade 288, `blade_signatures/ferrix_foundry.glsl`): A Ferrix foundry at the end of the shift: hammered steel with bands of forge heat sliding slowly up it. Weld sparks spit from wherever the heat has reached.
  - **Kenari Crash** (Uncommon, blade 289, `blade_signatures/kenari_crash.glsl`): The mining ship down in the Kenari forest: burning at the hand and cooling toward the point, where it is only charcoal. Embers climb from the fire end and go out along the way.
  - **Sarlacc Pit** (Uncommon, blade 290, `blade_signatures/sarlacc_pit.glsl`): The Great Pit of Carkoon: a ribbed pink throat swallowing downward into pale desert. Tendrils reach out of the low end of it and sand blows round the top.
  - **Cloud City Dusk** (Uncommon, blade 291, `blade_signatures/cloud_city_dusk.glsl`): Bespin at the end of the day: gold at the hand, rose along the middle and violet at the point, with bright cloud moving through the blade. A few soft clouds drift past it.
  - **Ring of Kafrene** (Uncommon, blade 292, `blade_signatures/ring_of_kafrene.glsl`): The mining outpost in the belt round Kafrene: rough teal rock with ore in it that catches light and flashes as it turns. A slow band of mining dust winds up round it.
  - **Hypergate Drift** (Rare, blade 293, `blade_signatures/hypergate_drift.glsl`): A hypergate seen from the side: light pulled into long lines that run the whole length of the blade in magenta and blue, with a few thin threads winding slowly round it.
  - **Bardotta Calm** (Common, blade 295, `blade_signatures/bardotta_calm.glsl`): The stillness of the Dagoyan mystics of Bardotta: a soft lilac blade that does nothing but breathe, the breath moving slowly up it, with a little air breathing round it.
  - **Alzoc Ice** (Common, blade 296, `blade_signatures/alzoc_ice.glsl`): The buried ice of Alzoc III: clear blue with fine white fractures all through it, its surface catching light in small faces of frost.
  - **Fest Steppe** (Common, blade 297, `blade_signatures/fest_steppe.glsl`): The windy steppes of Fest: dry gold stalks running up the blade, bending together as gusts pass through them. Seed heads are carried off into the air.
  - **Ansion Plains** (Common, blade 298, `blade_signatures/ansion_plains.glsl`): The open grasslands of Ansion: a green blade with broad patches of sun and cloud shadow moving over it, and pollen hanging in the air beside it.
  - **Sernpidal Dust** (Common, blade 299, `blade_signatures/sernpidal_dust.glsl`): Sernpidal under the falling moon: dust in the air pours down the blade from point to hand, and a thin red cloud hangs on round it.
  - **Angel** (Mythical, blade 294, `blade_signatures/angel.glsl`): off-white core, heavenly clouds that charge up over five seconds; once they have come a large golden halo above its point, a few falling feathers, small clouds coming away and a cloud-mist trail.
  <!-- star wars round end -->

## Hub protocol (additions to version 1)

Old clients ignore all of it, so it stays `/v1/`. The hub's `PROTOCOL.md` gains an
"Unlocks" and a "Looks" section with exactly this.

### Unlocks

- Profile gains `"unlocks":[{"id":"saber_sun","granted":<unix s>,"note":".."}]`, in
  the catalogue's order; `note` is 0 to 200 characters of plain text from the team,
  often empty. Absent from older hubs.
- A key holds each unlock at most once. Only the operator (`unlock <key_id> <id>
  [note]` and `relock <key_id> <id>` commands, and the operator API beside medals)
  and staff can grant or take one back:
  - `POST /v1/staff/unlock` (signed, staff) `{"key_id":"..","unlock":"..","note":".."}`
    answers the target's Profile. An unknown id is 400 `bad_unlock`, a key already
    holding it 409 `already_unlocked`, an unknown key 404 `not_found`.
  - `POST /v1/staff/relock` (signed, staff) `{"key_id":"..","unlock":".."}` answers the
    Profile; a key not holding it is 404 `not_unlocked`.
  - Both are written to the staff log and count against the staff limits.
- Taking a blade skin back clears it from that key's live looks at once (and sends a
  look event, below), so it disappears for everyone.
- Removing an identity removes its unlocks.

### Looks

A look is what a player wears that others draw:

    {"saber":"saber_sun","illuminate":true}

Since 23/10/2026 the client may send `"saber_sounds":".."`, the saber sounds its
wearer chose apart from the blade ([Saber sounds](#saber-sounds)): absent or `""` to follow
the blade, `"stock"`, or a blade-skin unlock id. The hub takes it since 24/10/2026; a
hub from before refuses the field with `bad_body`, and the client then sends the look
again without it.

Since 11/10/2026 a look may also carry `"saber_off":["glint","haze"]`, the blade skin's
options its wearer switched off ([Options](#options)): optional in a request (at most 8
distinct ids of 1 to 24 of `a`-`z`, `0`-`9`, `_`, else 400 `bad_look`; an older hub
refuses it with `bad_body`, so a client sends it only when it names one), stored with
the look and given back in presence and look events only when not empty.

Since 26/10/2026 a look may also carry `"saber_on":["flare_trail"]` and `"saber2_on"`,
each saber's extras switched on ([Extras](#extras)), on `saber_off`'s rules. The hub keeps
a saber's only while the key holds a revealed copy of the card that unlocks that skin's
extras in its finish, and drops them otherwise without refusing the look. A hub from
before refuses them with `bad_body`: the client sends each only when it is not empty, and
on a `bad_body` sends its looks without them for 10 minutes, or until the hub changes.

Since 21/10/2026 (hub pull request with SJK's, Creyon) a look may also carry `"saber2"`,
the second saber's blade skin for a player with two sabers (a blade-skin id the key holds
like `saber`, or `""` for the stock blade), and `"saber2_off"`, that skin's options
switched off, on `saber_off`'s rules. Without `saber2` both sabers wear `saber`, as
before; presence and look events carry it only when sent, and `saber2_off` only when not
empty. A hub from before refuses both with `bad_body`: the client sends `saber2` only
when the second saber wears another skin than the first, and on a `bad_body` sends its
looks without it for 10 minutes (`SABER2_RETRY`), or until the hub changes, then tries
again. Taking a skin back makes a second saber wearing it the
stock blade. The hub names the side it refuses (review of 22/10/2026): the second saber's
skin is 403 `saber2_not_unlocked` (not held) or 400 `bad_saber2` (not a blade skin), the
first's keeps `not_unlocked` and `bad_look`, and it checks the first before the second.
The client drops only the skin named and sends the rest of the look again
(`service::refused_skin`; a hub before the side was named answers `not_unlocked` for
either, read as the first's, or the second's when the first wears none).

`saber` is a blade-skin unlock id or `""` for the stock blade; `illuminate` is
whether the Illuminate holocron is lit. Both are required in a request.

- `POST /v1/look` (signed) `{"server":"ip:port","saber":"..","illuminate":false}`.
  Exactly those fields (400 `bad_body` otherwise). `server` as in claims (400
  `bad_server`). The key must hold a live claim on `server` (403 `not_on_server`);
  the hub takes the slot and claimed name from it. A `saber` that is not `""` must be
  a blade skin in the catalogue (400 `bad_look`) that the key holds (403
  `not_unlocked`). At most one look a second per key and 30 a minute (429
  `look_quota`; a refused one does not count). Looks share the chat and emotes
  per-address allowance. Answers `{"id":<n>}`, the feed id of its event.
- The look is kept on the claim. Renewing the claim for the same server, slot and
  name keeps it; a claim for another server, slot or name, a release or the claim's
  expiry drops it (a new claim starts with no look).
- Presence entries gain `"look":{"saber":"..","illuminate":false}` when the claim
  has one (absent otherwise).
- The feed gains `looks`, sharing the id sequence: the look events on the reader's
  `server` with an id above `after`, oldest first, kept for 60 seconds. Each:

      {"id":15,"at":<unix s>,"slot":3,"claimed_name":"^2Sol","key_id":"..",
       "saber":"saber_sun","illuminate":true}

  A batch with looks is not empty. `after` 0 or above the newest id (a new
  reader, or a hub that restarted) gives the server's looks of the last 60 seconds:
  looks are state, so a reader's first answer must not miss one, and applied oldest
  first they leave each slot with its latest look.
- A client applies a look, as emotes and badges, only when the name its game shows in
  `slot` matches `claimed_name`.

## Client

### Settings and commands

Built.

- `cg_saberSkin` (archived, default empty; one registration, the constant
  `unlockables::SABER_SKIN_CVAR`): the blade-skin unlock id the player wears. It
  applies only while the player's own hub profile lists that unlock; with no
  identity, no hub, the hub not answered yet or the unlock missing, the stock blade
  shows and nothing is sent. The looks read it twice a second and gate it
  (`Looks::own_saber_skin`); the renderer takes the gated skin from there
  (`GpuState::local_saber_skin`, [saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs)),
  for the hand, first person, a thrown saber, the sounds and the Character page's
  preview.
- `cg_saberSkin2` (archived, default empty, `looks::SECOND_CVAR`; 21/10/2026, Creyon):
  what the player's second saber wears when they hold two: empty for the first saber's
  skin, `stock` for the stock blade, or a blade-skin id, worn only while the profile lists
  it (else the stock blade, as for the first saber and as the hub makes a second saber whose
  skin is taken back: review of 22/10/2026, it followed the first here while the others saw
  it stock). The look sends it as `saber2` ([Looks](#looks)) and every other
  SJK player draws each saber with its own skin (`SaberSkins::get_both`,
  `saber_skins::skin_of_blade`, `saber_submission::submit`). With one saber whose blades
  point both ways, a staff, the second skin goes on its other half (the blades pointing more
  than 120 degrees away from the first, `saber_hilts::other_half`; Sol's request of
  21/10/2026), each half with its own light. A crossguard's side blades stand across the
  first and wear the first's skin with it, so a one-handed saber of three blades is no
  staff (review of 22/10/2026: counting blades took it for one); a thrown staff keeps the
  first skin, and the sounds stay the first skin's. `saberskin second <id>`,
  `saberskin second none` (the stock blade) and `saberskin second same` set it, and so does
  the Collection's Shaders tab through its target switch ([Shaders tab](#shaders-tab)).
- `saberskin` ([saber_skin_command.rs](../crates/sjk-viewer/src/saber_skin_command.rs))
  alone lists the blade skins the player may see (an unheld Unique one is neither listed
  nor taken by id or number, [Unique shaders](#unique-shaders)): id, name, owned (since
  when) or locked (how to get it), which one is worn or chosen, and why ownership is not
  known when it is not (identity off, no hub, waiting). `saberskin <id>` (case ignored, stored as the
  catalogue's id) or `saberskin none` sets `cg_saberSkin`; a locked skin is set all
  the same and the answer says it shows only once unlocked. An unknown id is refused.
  Tab completes the command's name; its argument has no completion (the console
  completes command and cvar names only).
- `saber_loadall` ([blade_load_all.rs](../crates/sjk-viewer/src/blade_load_all.rs); Sol,
  06/11/2026, to class the tiers of Creyon's new blades): builds every loaded blade's own
  shader now, after the blades worn and shown, instead of when a blade first draws, so the
  Collection's pictures and the preview show each blade's real look the moment it comes up
  (until its shader is built a signature blade draws plain, from its skin's file). Typed
  again it says how far the builds are ("Blade shaders: 120 of 262 built (48 s so far).",
  then "All 262 blade shaders are built (in 1 min 40 s)."). This PC and this run only;
  `debug_collectibles 1` shows every blade as held to go with it.
- `unlockables` opens (or closes) the Collection on its Shaders tab: in the SJK UI
  the Collection screen ([sjk-ui.md](sjk-ui.md#collection)).
- `debug_collectibles` (archived, default 0; 15/10/2026,
  [debug_collectibles.rs](../crates/sjk-viewer/src/debug_collectibles.rs)): for the SJK
  team, to try every collectible without granting it and taking it back at the hub.
  While it is 1 and the hub says the player's key is staff, the screens of what the
  player holds read a copy of their profile dressed with every shader, medal,
  achievement and card the client knows (each card in every finish): the Collection,
  the Profile and its card, the Saber tab's blade choice and `saberskin`; the blade they
  wear is drawn with any shader. What they really hold keeps its dates and counts; the
  added shaders and medals say "Debug collectibles: shown as yours on this PC only" on
  their cards, and the Shaders tab's lead line says the mode is on. Nothing reaches the
  hub: the look sent stays what the player holds (the hub refuses a skin they lack), so
  other players see their real blade; the new medal, achievement and holocron pop-ups
  and the Staff page read the real profile, so no pop-up fires and staff actions see
  the real grants. Holocrons and credits are not dressed (`debug_open` rehearses an
  opening). Checked by unit tests and the world shot `duel6_sjk_collection_debug`.

Verified by unit tests (the listing, the answers, the cvar set even when locked, the
gate); not yet used against the deployed hub.

### Sending

Built ([service.rs](../crates/sjk-identity/src/service.rs),
[looks_frame.rs](../crates/sjk-viewer/src/looks_frame.rs)). Twice a second the viewer
computes the own look: `cg_saberSkin` when the client knows that blade skin and the own
profile lists it (else `""`), and whether the local Illuminate is lit; it hands it to
`Service::set_look` only when it changed. The identity worker keeps the latest. After
its claim on a server is accepted, and whenever the look changes, it sends
`POST /v1/look`, at most one a second (changes in between are coalesced, the latest
wins); a new claim (another server, slot or name) sends it again, and renewing the
same claim does not. What the hub holds is known (none for a new claim or after a
release, else the last look it took) or unknown: at the worker's start (a client that
stopped without releasing may have left a live claim with a look), after a failed claim
(it may have lapsed, or still be live with the old look) and after a failed release.
A look equal to none (stock blade, holocron out) is not sent on a claim known to hold
none; while unknown, the next accepted claim gets the look whatever it is. A
`not_unlocked` (or `bad_look`) answer leaves that skin out, the look going on with
`saber:""` so Illuminate still syncs, until the profile's unlocks change. The own
profile is read every ten minutes, and sooner (at most once every 30 seconds) after a
`not_unlocked`, or when the feed relays a look of the player's own key, newer than the
last one sent, with another blade skin (staff took it back; `Service::take_looks`
hands it to the worker), so a relocked skin leaves the player's own blade within
seconds; too many
(429 `look_quota`) waits 1 second; the per-address `rate_limited` the chat and emotes
share, a refused signature (401) and failures wait 10 seconds and go again;
`not_on_server` asks for the claim again at once and the look goes with it;
another refusal (an older hub) is not repeated until the look or the claim changes.
Leaving the server sends nothing (the release drops the look).
`Snapshot::look_outcome` holds what became of the last one. Unit tests against a fake
hub, and end to end (`hub_e2e`) against the hub's unlocks work in progress run on this
PC: a look needs a claim, shows in presence and the feed, survives the claim's renewal
but not another slot, a skin the key lacks is refused until granted, and the service
wears its look (without a skin it lacks) and reads looks with the chat off.

### Receiving

Built ([feed.rs](../crates/sjk-identity/src/feed.rs),
[looks.rs](../crates/sjk-viewer/src/looks.rs)). The feed reads on a game server
whatever `cl_sjkChat` says, keeping no message with the chat off, and queues the
looks it gets (`Service::take_looks`, the newest 64). Another game server is read
from `after` 0, so its first answer brings that server's looks of the last minute.
Each queued look keeps the server it was read for and the reading's generation, which
changes with the hub, the server or the identity going off or on; the viewer is
handed only the looks read for the server it is on under the current generation, and
clears every hub look (reading the roster again) when the generation changes, so a
poll still under way at a change cannot bring an old look back. `GpuState::looks` keeps one look
per slot (64): the presence roster's when its revision changes (it replaces every hub
look) and feed events as they come (newer, so they win until the roster changes
again). A look counts only while the game shows the claimed name in its slot (the
badges' rule); the names are compared when a roster or an event comes and twice a
second, so a frame only reads a fixed table. Another server, leaving or a new feed
generation clears them.
An unknown skin id draws the stock blade. A player blocked on this PC
([hub-chat.md](hub-chat.md#blocking-a-player)) wears nothing while muted: the stock
blade without the skin's sounds, no holocron (`Looks::set_muted`, which never mutes
the own slot). The local player's own look comes from its
settings, not the hub.

What the renderer reads, on `GpuState::looks`:

- `saber_skin_id(client) -> Option<&'static str>`: the blade skin the player in that
  slot wears (a catalogue id), `None` for the stock blade; the local player's slot
  holds its own gated skin.
- `revision()`: changes whenever what a slot wears (or the own look) changes, so the
  blade skins' table is filled again only then.
- `illuminated(client) -> bool`: their holocron is lit.
- `own_saber_skin() -> Option<&'static str>`: the local player's gated skin, in a game
  or not (first person, the Character page's preview).

The player card ([client.md](client.md#player-card)) names the shader a player wears,
with its live swatch, by the same rule: the looks' skin for another slot, the own gated
skin for the local player's, and only when its pack is loaded, so the card never names
a shader their blade does not show here (10/10/2026). It names the second saber's (or a
staff's other half's) shader too, on a row of its own, when the player holds one and it
is not the first's (`second_skin(client)`, the local player's `own().second`; #224).

### Blade skins

Built (renderer and sounds); who wears which is the viewer's per-client table, filled
from the looks.

- A blade skin's look is its blade-skin file, loaded from the packs
  ([saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs) `LoadedSkins`, at most
  16): drawn as its own saber material after the neutral RGB pair, with its glow/core
  pair (the pack's images or generated from the file's profiles), coloured and animated
  in `saber.wgsl` from per-instance time, a per-blade seed and the file's parameters
  (core, corona gradient, granulation, flame tongues, shimmer, flares and, when the
  file has them, lightning arcs, drifting motes and a turning hue); the dynamic glow
  pass gets the same animation ([rendering.md](rendering.md#saber-blade-skins)). Its
  trail and light colours and the light's flicker (and hue, when it turns) are the
  file's. Retail and RGB blades are unchanged. A skin whose pack is not loaded (no hub
  yet, a client offline that never had it) is the stock blade, sounds too.
- The fourteen skins from Creyon's Blade Lab (Peppermint to Quantum) are drawn by the
  lab's own shaders while `cg_saberLabDesigns` is on (the default), not from their
  files' sections; their trail and light take the lab's colours, their sounds stay the
  files' ([rendering.md](rendering.md#blade-lab-designs)).
- Twenty-one mythicals from Creyon's Blade Lab collection (Abyssal Eye to Wraithlight,
  19/10/2026, all Star Wars or space: Fractal, Infinity Mirror and Kitsune were rebuilt as
  Hyperspace, Superlaser and Witchfire, Paradox, Time Fracture, Crystal Heart and
  Resonance renamed Balance, Between Worlds, Kyber Heart and Singing Kyber) are drawn by
  their lab signature shaders ([rendering.md](rendering.md#blade-lab-signature-shaders)),
  as are Monarch, Obsidian Edge, Quantum, Quicksilver, Reactor and Serpent in their
  reworked form. They were kept out of holocrons while they were tried, then released
  with every Blade Lab blade on 20/10/2026 (below). With no pack file, a built-in one
  draws them, so the Collection previews them.
- Twenty-three legendaries from the Blade Lab (round 11, 20/10/2026), each reworked to be
  Star Wars (Avalanche to Hoth Avalanche, Blood Moon to Dathomir Blood Moon, Chrono to Force
  Echo, Collider to Hypermatter, Comet to Kessel Run, Cosmic Ink to Unknown Regions,
  Dragonscale to Krayt Pearl, Frozen Lightning to Starkiller Frost, Gilded Cage to Royal Naboo,
  Hurricane to Bespin Storm, Koi Dragon to Scarif Tide, Mech Core to Assassin Droid, Midas to
  Aurodium, Monsoon to Kashyyyk Monsoon, Northern Forge to Malachor Forge, Overgrowth to Yavin
  Vines, Rift to Exegol Rift, Shatter to Shattered Kyber, Solar Flare to Twin Suns, Spirit
  Lanterns to Jedi Spirits, Thunderhead to Ord Mantell Thunderhead, Volcano to Mustafar
  Eruption, Wormhole to Hyperlane), are signatures too, less dramatic than the mythicals as a
  tier down and wearing Sol's own pieces from their files (afterimages, an echo, glints,
  smoke, embers, haze, veins). Released with the rest on 20/10/2026.
- Twenty-two rares from the Blade Lab (round 12, 19/10/2026), Star Wars reworks a tier below
  the legendaries: Bioluminescence to Ahch-To Glow, Blueprint to Imperial Schematics, Calligraphy
  to Jedi Texts, Constellation to Star Map, Frostbite to Ilum Frost, Geode to Christophsis
  Crystal, Hex Shield to Deflector Shield, Kintsugi to Healed Kyber, Mosaic to Temple Mosaic,
  Moth to Dagobah Wisps, Neon Rain to Nar Shaddaa Rain, Polaris to Navigation Beacon, Puzzle
  Cube to Puzzle Holocron, Reef to Dac Reef, Sandstorm to Jakku Sandstorm, Spider Silk to
  Moisture Farm, Stormglass to Tipoca Stormglass, Sunbeam to Temple Sunbeam, Tesla Coil to Ion
  Coil, Thorn to Sith Thorn, VHS to Holo-Recording and Vinyl to Data Disc; none has figures or
  anything on its tip. In the client's catalogue only, as the legendaries.
- Twenty-two uncommons and twenty-two commons from the Blade Lab (round 12), each a tier less
  special than the one above and every name and look made to fit Star Wars: Cargo Stack, Otoh
  Gunga, Victory Fireworks, Endor Ivy, Naboo Core, Gungan Waters, Cantina Lights, Imperial
  Searchlight, Flare Stick, Flimsiplast, Canto Bight, Sensor Sweep, Takodana Rain, Starfighter
  Canopy, Distress Beacon, Orto Plutonia, Rhen Var Frost, Seismic Charge, Thermal Fuse, Senate
  Glass, Holonet Feed, Holo-Map and Cockpit Console (uncommon); Code Cylinder, Cortosis Weave,
  Tatooine Dust, Trench Run, Ryloth Dawn, Power Coupling, Kyber Filament, Frosted Viewport,
  Holo-Stylus, Sabacc Card, Geonosis Hive, Mygeeto Frost, Bogano Moss, Kef Bir Sea Glass, Jedha
  Lantern, Corellian Pearl, Targeting Display, Raxus Scrap, Pasaana Stone, Smoked
  Transparisteel, Batuu Dusk and Alderaan Velvet (common). The Glowstick and Ink Brush stayed
  out. Star Dragon is now the Ashla Radiance blade (same id; the hub's name must follow).
- A skinned blade ends round (09/10/2026; Sol saw the Sun's tip square). The core line
  is a flat quad from behind the hilt to the tip, and the generated core's fringe is
  still about a fifth of its brightness at the quad's edges and end, so its end and
  corners showed as a bright square, plainest with a wide, bright fringe like the Sun's
  over a tip glow half as bright as the shaft's (stock blades draw the same flat line,
  but their white core is hidden in their glow). Past the tip, too, the corona's
  grading and its flame tongues were taken from the distance across only, so they ran
  on straight beyond the tip as a column, and a widening corona widened only sideways.
  Now, for skins only: over its last `core.tip` half-widths the core line narrows on a
  quarter circle to a rounded point and is cut (softly, over a pixel) at that edge;
  past the tip the corona widens about the tip as about the shaft, its distance out is
  measured from the tip, and the tongues run on round it. Along the shaft nothing
  changed. Pinned by a test of a CPU copy of the tip's maths (the shaft unchanged, the
  quarter circle, the corners cut, the glow round past the tip) that also checks the
  shader holds the same expressions.
- Shown for every client whose entry in the viewer's `SaberSkins` table is set: in the
  game, in first person, thrown, and on the menu stage (the Character page) for the
  local player. Once a frame (`GpuState::sync_saber_skins`) the table takes every
  other slot's skin from `Looks::saber_skin_id` when the looks' revision or the loaded
  skins changed (no allocation; 32 lookups only then), and the local player's own from
  the gated `Looks::own_saber_skin` in its slot (also with no session, for the
  preview). That slot is the game state's `client_num`, not the snapshot's player
  state's: following (spectating) someone, the state is theirs (`PMF_FOLLOW`), and they
  keep their own hub look, blades, thrown saber and sounds alike (`looks::ViewSlots`).
- Sounds, per player wearing it: ignition and switching off, the hum loop and three
  swings play over the stock ones, which keep playing below (each on a channel of its
  own, so the skin's does not cut them), at the paths the file names in its pack, unless
  the wearer chose other saber sounds ([Saber sounds](#saber-sounds)). With
  `s_saberSkinSounds 2` (Settings > Sound > Blade skin sounds: Only skin) they play in
  the stock ones' place, and with `0` (Off) they are not played: the stock ones alone. A sound
  the file names that is not in the game data (no pack carries it yet) is left out: the
  stock one plays there as for a player without a skin, in either mode, never silence
  (`sound_present` in `sjk-client`, the viewer's `has_sound` for animation swings). The viewer
  passes the table to the audio adapter every frame
  (`LegacySoundAdapter::set_saber_sound_overrides`, [client.md](client.md#blade-skins)),
  so other players' sounds follow their looks too; a pack arriving mid-session has its
  sounds registered at once. A thrown saber hums the skin's hum as well: over the saber
  entity's own `loopSound`, while its owner (`genericenemyindex`) wears a skin.
- Verified: unit tests against a made-up test skin (material slots, the uniform's
  layout and values, colours and flicker, the table following the looks, the gated own
  skin and the loaded skins, loading from a pack with images and sounds, a runtime
  mount, the sound overrides per client and registered mid-session, a thrown saber's
  hum), `saber.wgsl` validated by naga, and the `duel6_sun_blade` world shot with the
  hub's pack built locally (`SJK_TEST_PACKS`), compared with shots of the earlier
  built-in Sun. Not verified: the sounds by ear, a live match, another player's skin
  from a real hub, a pack downloaded from the deployed hub.

### Tiers

Built 10/10/2026 (Sol's request: give shaders qualities, as more come, plainer ones
too). Every blade skin has a tier (`Unlockable::tier`, `unlockables::Rarity` in
[unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)), one of the holocrons' four,
with the holocron tier's name (`Tier::label`) and colour
([holocrons.rs](../crates/sjk-viewer/src/holocrons.rs)) so a Rare shader and a Rare
holocron look alike, or Common (14/10/2026) or Unique (25/10/2026, rarer than Mythical
and no holocron's, [Unique shaders](#unique-shaders)). The table below is the first
placing; the catalogue's table above has today's:

| Tier | Shaders |
| --- | --- |
| Mythical | Sun, Void, Molten, Spectral, Reactor, Bleeding Kyber, Serpent, Monarch, Phoenix, Quantum, Obsidian Edge |
| Legendary | Storm, Unstable, Chameleon, Prism (provisional), Hourglass, Fiber Optic, Comic, Quicksilver |
| Rare | Frost, Hologram, Runic, Heartbeat, Glitch (provisional), Banner (provisional), Soap Bubble, Lava Lamp, Sketch |
| Common | Tidal, Circuit, Aurora, Echo, Slate, Amber, Lilac, Mint, Coral, Ivory |
| Uncommon | Nebula, Cinder, Venom, Sakura, Firefly, Glacier, Dusk, Ruby, Amethyst, Jade, Moonlight, Neon, Peppermint |

Sol placed eleven; Prism, Glitch and Banner are placed provisionally until Sol says
otherwise. The hub keeps the same tiers (`UnlockKind::tier`) for its loot table
([Holocron shader drops](#holocron-shader-drops)).

- Lists: the Collection's Shaders tab and the Saber tab's Blade row list the rarest
  first, then in catalogue order within a tier (`unlockables::blade_skins_by_tier`,
  kept to what the player may see by `Holdings::listed`), the stock blade first.
  Profiles, the hub and the Staff page keep the catalogue's order.
- Shown: the Collection's rack frames each swatch in its tier's colour (quieter when
  locked) and names the tier at the right of its state line; its cards carry a band of
  the tier's colour along the top, a frame and the tier's name; beside the model a pill
  in the tier's colour stands over the kind line; the Saber tab's swatches have a thin
  edge in it; the player card frames the worn shader's swatch in it and writes its name
  in the colour lifted toward white.
- Animated: every tier from Rare up has its rarity effects there
  ([Rarity effects](#rarity-effects)).

### Rarity effects

Built 10/10/2026 (Sol's request: small light animated touches that show off how rare a
thing is, for shaders, holocrons and what comes later).
[rarity_fx.rs](../crates/sjk-viewer/src/rarity_fx.rs) draws them for any tier, with the
UI's own shapes into the frame's draw list (no texture, no pipeline of its own), at most
30 commands an item, every one inside the item's frame, the same picture at the same
moment (no state, no randomness). Each tier up adds to the one under it:

| Tier | Effect |
| --- | --- |
| Uncommon | none |
| Rare | the frame's glow breathes: a soft pulse every 3 seconds |
| Legendary | the breath, and a slanted sheen sweeping left to right (1.4 seconds of every 4, each frame at its own moment so a page does not flash at once) |
| Mythical | the sheen, a bright spark running round the border with a short tail (a lap in 3.2 seconds), and on cards and tiles four faint motes rising inside |
| Unique | Mythical's, in Unique's rose (25/10/2026, Sol: "mythical level of effects"; `Rarity::effect_rank`) |

- Sizes: `Full` (the Collection's cards, the Holocrons tab's tier rows) has all of it;
  `Small` (the rack's swatch frames, the detail's tier pill, the Saber tab's swatches)
  leaves out the motes and has a shorter sheen; `Tiny` (the player card's shader row)
  only breathes, quietly.
- Locked: a shader not held (or a holocron tier not held) only breathes, at about a
  third of the strength, so what the player holds stands out.
- `ui_rarityEffects` (archived, default 1; Settings > Interface > Rarity effects) turns
  them off: the tier frames stay, still.
- The chosen card of the grid is marked apart from any tier: it lifts 5 pixels over a
  shadow and a white outline goes round it (until 10/10/2026 a gold border, which read
  like Mythical's gold frame).
- Not used: the holocron pop-up, which has its own ceremony in the tier's colour (rings,
  light, ticks), and the SJK chat's gem marks, too small at text size for a frame to
  show.
- Verified: unit tests (every shader's tier, the order, the holocrons' names and
  colours) and the world shots of the Collection (`duel6_sjk_collection`); not tried
  in the game.

### Chromas

Built 10/10/2026 (Sol's request: mark the shaders that can be coloured). A **chroma**
is a blade skin that takes its wearer's saber colour: `color1` for the first saber,
`color2` for the second, a retail colour or a custom RGB, as the stock blade would be
drawn. The flag is the client catalogue's (`Unlockable::chroma` in
[unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)); the hub and the packs are
unchanged. Chromas: Storm, Unstable, Spectral, Glitch, Hologram, Runic and Heartbeat
(my proposal to Sol). Sun, Frost, Void and Molten keep their themed colours; Prism,
Chameleon and Banner are about their colour. Creyon's chromas (his rounds 13 and 14) are
those where a colour makes sense: kyber, holograms, the Force, data and light (Droid Logic,
Echo Base, Amethyst, Neon, Bleeding Kyber, Kyber Cluster, Clone Wars, Between Worlds,
Eclipse, Hyperspace, Singing Kyber, Ashla Radiance, Holdo Maneuver, Abyssal Eye, Shattered
Kyber and sixteen rares, uncommons and commons; the Moon, 26/10/2026); a signature's own effects keep their colours
through `uKeep` and `uOwn`, so only its blade takes the wearer's, except Hyperspace's and the
Ashla Radiance's, whose effects take it with the blade (round 15, 20/10/2026). Round 15 also
took the chroma off Felucia Spores (it hid the spores' colours) and Force Echo, and swapped two
tiers: the Jedi Spirits blade is mythical, the Balance blade legendary. Sol's tier review of 20/10/2026 then moved Clone Wars and Exegol Rift
to mythical; Molten, Spectral, Reactor, Bleeding Kyber, Sith Alchemy, Felucia Spores, Phoenix
Squadron, Dyson Sphere, Quasar, Sith Holocron, Ashla Radiance, Superlaser, Uneti Tree, Witchfire
and Wraithlight to legendary; Storm, Prism, Unstable, Chameleon, Hoth Avalanche, Kessel Run,
Krayt Pearl and Assassin Droid to rare; and Targeting Display to uncommon: 22 mythicals. Only the
tier changed (label, colour, Collection order, holocron odds); the hub's catalogue must follow. Round 19
(21/10/2026) made Kyber Heart and Christophsis Crystal chromas (kyber and crystal take any colour);
no chroma was taken off. Round 20 made Hyperlane, Nebula Song, Deflector Shield, Quasar and Victory Fireworks (its
fireworks keep their colours) chromas. Creyon's Star Wars round (1/11/2026) made Event Horizon a chroma (he asked for it), and, on his second test of the round ("give mythicals most of them RGB, give more blades RGB"), 51 of the round's blades: Zeffo Thunderchorus, Moraband Erasure, Korriban Ragnos, Rakata Infinite, Ziost Dark Anvil, Lah'mu Sandveil, Dromund Kaas Vitiate, Bendu Equilibrium, Lehon Maelstrom, Zonama Starlight, Nur Shadowflame, Tund Sorcerers, Haruun Kal Wildheart, Ruusan Thoughtbomb, Chirrut Whills, Atollon Gathering, Utapau Whirlwind, Umbara Gloaming, Tython Je'daii, Bastila Battle Meditation, Cantonica Champagne, Lira San Starglass, Garel Corona, Iktotch Foresight, Kowak Silkweb, Nar Kreeta Lightspeed, Wobani Projection, Rishi Ocean Flow, Eshan Echani, Toydaria Gale Streamers, Kijimi Snowfall, Saleucami Turbolaser, Glee Anselm Rockpools, Manaan Tidebreaker, Coruscant Neon, Savareen Starfall, Hijarna Opal, Scipio Awakening, Rattatak Crimson Surge, Sorgan Lake, Iego Lightglow, Lola Sayu Watchbeam, Skako Pressure, Nal Hutta Spicevine, Sluis Grey, Arkanis Teal, Corulag Slate, Bothawui Amber, Ithor Verdant, Mirial Saffron, Zeltros Violet. Creyon's review of 5/11/2026 took it off where a blade's own colour is the blade: the God Ki (Trayus Scythe), Korriban Ragnos, the Moon (its colour should only reach its particles, which a chroma cannot do alone) and Angel among the Mythicals; Eshan Echani, Toydaria Gale Streamers and Cantonica Champagne among the Rares. On 6/11/2026 he asked for more Legendaries with RGB: all the round's Legendaries are chromas but Kalevala Darksaber, Wroshyr Rootwood, Orbalisk Carapace, Ysalamir Hush, Amphistaff Serpent, Ziost Silence and Nathema Deadworld, whose material is the blade. Commons and Uncommons are unchanged. Under a colour a round blade's palette now spreads (`sig_turned` in `saber.wgsl`): the body takes the wearer's colour, every other colour it was designed with keeps its side of the wheel a clear step off, up to a third of the wheel for one that was opposite (Peridea's red and blue had folded onto the body's), and its effects in the air take an analogous colour a sixth of the wheel to their side, so no effect runs into the body.

- **How it is drawn** ([saber_skins.rs](../crates/sjk-viewer/src/saber_skins.rs)
  `Chroma`, `SkinColor::worn_with`): the skin's own hue is its corona rim's
  (`hue_of`: the angle round the grey axis from red, in the sense `turn_hue` turns;
  the light's colour if the rim is grey). Worn on a saber, every colour the skin draws
  (glow, core, arcs, motes, veins, scan lines, glyphs, embers) is turned round the grey
  axis by the gap from that hue to the saber colour's, keeping brightness and
  saturation; the dynamic light and the blur trail turn with it. A grey or white
  colour (under 20 % saturation) has no hue to take: the skin is drawn as its file
  says. The turn rides on every blade instance (`Instance::chroma`, attribute 8), so
  `saber.wgsl` turns the finished glow and core once; other players' blades take their
  colours from their `CS_PLAYERS` string, so everyone sees a chroma in its wearer's
  colour. A pale skin (the Storm, the Hologram) stays pale, only tinted.
- **Team games**: with `cg_saberTeamColors` (on by default) a red or blue player's
  saber colour is their team's, and every skin stays worn with that colour as its base,
  so a chroma turns red or blue ([client.md](client.md#team-saber-colours)). Only
  `cg_ctfTeamSabers` in CTF and CTY draws the plain team blade without the skin.
- **The mark**: a small colour wheel (twelve hue segments round a white dot,
  `swatch::chroma_mark`) beside a chroma's name on the Collection's rack and before
  "Chroma saber shader: takes your saber colour" beside the model, in the corner of its
  swatch on the Saber tab's Blade row, and between its swatch and name on the player
  card. The swatches draw a chroma in the player's own colour (the card: in the
  player's first saber colour when it is a retail one).
- Verified: unit tests (hues and turns, grey and white kept, every retail colour and a
  custom one, each saber of a pair its own turn, the instance carrying it, the chroma
  list pinned, the mark inside its square, a turned swatch, the card with the wheel
  inside the screen at five sizes) and the world shot `duel6_chroma_blades` (every
  chroma in the six retail colours and white) with the hub's pack; not tried in the
  game.

### Packs

Built ([sjk_packs.rs](../crates/sjk-viewer/src/sjk_packs.rs); the download is the
identity service's, [identity.md](identity.md#asset-packs)). A pack is a PK3 of game
paths the hub serves (`GET /v1/assets`, `GET /v1/assets/<name>`): `sjk_skins` (the blade
skins' files and sounds), `sjk_cards` (the cards) and, since 16/10/2026, `sjk_solfox`
(Reyced's Solfox player model, which any player may wear; mounted below the game data like
the others, so it shows in the model list and on other SJK players with no code of its
own).

- The identity service keeps them in `assets/<name>.pk3` beside `identity.key`,
  checked against the hub's size and SHA-256 and written in one rename.
- At start the viewer mounts every cached pack (`sjk_packs::mount_at_start`), whatever
  `cl_identity` says: the content is SJK's own and already on the machine. Its
  blade-skin files load into `LoadedSkins` under a new generation; each frame
  `GpuState` compares one atomic counter and, when it changed, uploads the skins'
  parameters and pairs and registers their sounds.
- Twice a second the viewer compares the service's count of packs written; when it
  changes the folder is mounted again, so a pack that arrives mid-session shows (and
  sounds) at once, for the local player and everyone wearing it. What the last check
  did is written to the log.
- Every game file system built for a world mounts the cached packs below all game data,
  as the holocron's files are.

### Blade-skin files

`skins/blades/<unlock id>.bladeskin` in a pack, a JSON object
([blade_skin_file.rs](../crates/sjk-viewer/src/blade_skin_file.rs)); the Saber Lab
([client.md](client.md#saber-lab)) reads them from a folder and reloads them as they
are saved, to make one. Parsing is strict:
an unknown or missing field, a wrong type or a value outside its range refuses the file,
and the log names the file and the field. A file whose id is not a blade skin of the
client's catalogue is left out (a newer pack may hold skins an older client does not
know). `grain` below is the granulation in [0, 1], `flare` the flares' sum at a point.
`arcs`, `motes` and `hue` are optional sections (09/10/2026), and so are `sputter`,
`glitch`, `scan`, `pulse`, `ghosts`, `embers`, `veins`, `team`, `ambient` and `glyphs`
(10/10/2026, [blade_skin_effects.rs](../crates/sjk-viewer/src/blade_skin_effects.rs)): a
file without one draws none of it (its lanes of the uniform are zeros), so files written
before them draw as they did; when present, every field of the section is required.
`team` and `ambient` may not both be given.

The third set (11/10/2026, [blade_skin_extras.rs](../crates/sjk-viewer/src/blade_skin_extras.rs)):
`star`, `wisps`, `haze`, `echo` and `options`. A client refuses a whole file holding a
field it does not know, so these live in an **additions file** beside the skin's,
`skins/blades/<id>.bladeextra`, a JSON object whose fields are laid over the skin file's
before it is read (a field in both is the addition's). A client from before never opens
it and draws the skin file alone, so a pack can grow without blanking a skin for older
clients; an additions file without its skin file is no skin. The same rules hold: each
section optional, every field of it required.

The fourth set (11/10/2026): `orbit` and `holes`. A client of the third set refuses a field it does
not know in the additions file too, so it lives in a **later additions file**,
`skins/blades/<id>.bladeextra2`, laid over the skin file after the first one (a field in
it replaces the same field whole, so it may also give the `star` and `options` the newer
clients draw: the Sun's gives shorter rays and renames the part). A client of the third
set never opens it and draws the first additions file's glint at the tip. The same
rules hold. A later set goes in a file of the next number.

The fifth set (15/10/2026, for the Blade Lab's shaders,
[blade_skin_lab.rs](../crates/sjk-viewer/src/blade_skin_lab.rs)): `bands`, `segments`,
`film`, `blobs` and `front`, in the **third additions file**,
`skins/blades/<id>.bladeextra3`, laid over the skin file after the other two (and with
`options` naming its sections, which an older client could not check). A client of the
fourth set never opens it and draws the skin file alone, so the hub's files for those
skins put a fair plain blade in the skin file (the Bleeding Kyber's veins and drops, the
Reactor's sparks and throb, the Neon's glass edges as a hologram's side lines). The same
rules hold. Each section is drawn in the glow and the core alike: the bands' colour and
the front's recolour both, the segments dim both.

The sixth set (15/10/2026, the Blade Lab's figures,
[blade_skin_figures.rs](../crates/sjk-viewer/src/blade_skin_figures.rs)): `sketch`,
`hourglass`, `serpent`, `strands`, `butterflies`, `wings` and `copies`, in the **fourth
additions file**, `skins/blades/<id>.bladeextra4`, laid over the others (with the
`options` naming them). The same rules hold. The butterflies' flights and the wings'
span widen the glow's quad (its room, as the wisps' and haze's); the copies keep inside
it (their angle is a share of its width).

The seventh set (15/10/2026, the Blade Lab's solid blades,
[blade_skin_solids.rs](../crates/sjk-viewer/src/blade_skin_solids.rs)): `ink` and
`chrome`, in the **fifth additions file**, `skins/blades/<id>.bladeextra5`. Every other
section adds light; these are drawn after the blades, alpha-blended, so they can be
dark or opaque ([rendering.md](rendering.md#saber-blade-skins)). They take the last two
bits of the instance's 32-bit mask of parts switched off: a later section that can be
switched off needs a wider mask.

The eighth set (18/10/2026, the Saber Lab's red blade with a smoking emitter,
[blade_skin_smoke.rs](../crates/sjk-viewer/src/blade_skin_smoke.rs)): `smoke`, in the
**sixth additions file**, `skins/blades/<id>.bladeextra6`. It is not drawn by
`saber.wgsl` with the blade: each blade's emitter lets puffs of the game's particles go
into the air ([saber_smoke.rs](../crates/sjk-viewer/src/saber_smoke.rs)), so a swing
leaves them strung out behind it and a blade put out goes on smoking for `after`
seconds. The puffs of a frame are spread along the way the emitter came; a frame after a
gap of more than 250 ms lets none go for the gap. The smoke's shader is added to the
effect atlas (which holds the map's effects' shaders) when a loaded skin has it, once an
atlas. It cannot be switched off as an option: the mask has no bit left.

| `smoke` field | Meaning | Range |
| --- | --- | --- |
| `shader` | A game shader for the puff (`gfx/effects/jk_smoke5`: stock smoke with texture, alpha-blended; an additive shader fades its colour instead of its opacity, as its blend asks) | game path |
| `color`, `opacity` | The puff's tint, and how opaque it is when let go (it fades to nothing over its life) | 0-1, 0-1 |
| `rate`, `after_rate`, `after` | Puffs a second while lit; a second just after it is put out, falling to none over `after` seconds (small faint puffs, many a second, chain into a thin wisp) | 0-200, 0-200, 0-20 |
| `life` | Seconds a puff lasts (each ±25%) | 0.1-10 |
| `size`, `growth` | Units across when let go, and when gone | 0.2-32, 0.2-96 |
| `rise`, `spread` | Units a second up (negative: down), and whichever way at random | -50-100, 0-50 |
| `follow` | The share of the emitter's own speed a puff keeps (a swing drags it along) | 0-1 |
| `spin` | How fast a puff turns, either way at random (the game's rotation units) | 0-10 |
| `turn` | The most a puff is turned when let go, either way (180: any way) | 0-180 |
| `sway`, `sway_rate` | Units a second the stream sways to the side, and turns a second its way goes round: the chain of puffs curls | 0-50, 0-10 |
| `along` | Where on the blade it comes from: 0 the emitter, 1 the tip | 0-1 |

`ribbon`, in the same file, beside `smoke` or alone: smoke as strands
([saber_ribbons.rs](../crates/sjk-viewer/src/saber_ribbons.rs),
`saber_ribbons.wgsl`). The emitter lays points along its way, one strand through them,
drawn facing the camera: soft across, torn along by noise that rides with the points.
Each point is left in the air, rising, keeping a share of the emitter's speed (at most
40 units a second, dying away at `DRAG` 5 a second), the stream swaying as it is let go
and the point curling as it goes; it widens and fades as it ages. A swing stays smooth
(the user, 18/10/2026: "really sharp corners"): points are laid a unit apart however fast
the emitter goes, a frame behind on the centripetal Catmull-Rom spline through its last
four places; each keeps the emitter's speed where it was let go (the spline's way there,
the speed going smoothly from frame to frame) and is moved on to the frame it is laid in;
a pose held under 100 ms (the stage skins in steps) is no new place on the way; held
longer (the stance) the emitter smokes where it stands at its rate, the way laid out to it
first (at first a held place laid nothing at all, and the strand broke off after 250 ms:
no smoke standing, the user, 18/10/2026); the strand is drawn
through its points on the spline, each span in three pieces. A frame's move farther than
64 units (the model put elsewhere) breaks the strand. Put out, the strand is laid on for `after`
seconds, staying strong a while then thinning away (`1 − (t / after)²`). A gap (the blade
not drawn for 250 ms) ends a strand; one the emitter left lives out its points. The
pool holds at most 12 000 points in 256 strands. The strands are counted in the effect
layer's bounds: at first only the blades were, and a blade put out left its smoke
undrawn.

| `ribbon` field | Meaning | Range |
| --- | --- | --- |
| `color`, `opacity` | The strand's colour, and how opaque where it is let go (it fades as it ages) | 0-1, 0-1 |
| `rate`, `after` | Points a second (more: a smoother strand), and seconds it goes on once put out | 5-120, 0-20 |
| `life` | Seconds a point lasts: the strand's length | 0.2-10 |
| `width`, `growth` | Units across where it is let go, and where it is gone | 0.05-16, 0.05-48 |
| `rise`, `follow` | Units a second up; the share of the emitter's speed a point keeps | -50-100, 0-1 |
| `sway`, `sway_rate` | The stream's sway to the side as it is let go, units a second, and turns a second | 0-50, 0-10 |
| `curl` | Units a second a point curls as it goes | 0-50 |
| `detail`, `softness` | Noise cycles a unit tearing it along (0: smooth); the share of its half-width its edges fade over | 0-4, 0.05-1 |
| `idle`, `swinging`, `swing_speed` | How strong it smokes lit and held still, and lit and going `swing_speed` units a second or faster (the speed eased over a quarter of a second; in between, smoothly): `swinging` 0 smokes only standing, lighting and put out, 1 the whole time; at 0 the strand ends, and a new one starts when it slows | 0-1, 0-1, 10-1000 |
| `ignite` | Seconds it smokes at full strength once lit, swinging or not, thinning at their end (not when a blade first comes into view lit) | 0-10 |

`sparks`, in the same file: sparks crackling out of the emitter
([saber_sparks.rs](../crates/sjk-viewer/src/saber_sparks.rs)). Bursts come at random
moments (an exponential wait between them, drawn again when the rate rises, as at
ignition); a burst is one to `burst` of the game's particles shot out in a cone round
the world's up, the blade's way or between (`aim`), falling under gravity, each streaking behind it along the way it goes
now (`PrimitiveShape::Spark`: `CTail::CalcNewEndpoint`'s way; the efx tails keep the way
they were let go along, as SJK draws them), shrinking and fading as it dies. An additive
shader fades the colour, as the smoke's does.

| `sparks` field | Meaning | Range |
| --- | --- | --- |
| `shader` | A game shader for a spark (`gfx/misc/spark4`: a white-hot point in a warm halo, additive, taking the tint) | game path |
| `color`, `opacity` | Its tint (over 1 brightens it), and how strong when let go | 0-4, 0-1 |
| `rate` | Bursts a second while lit | 0-30 |
| `ignite`, `ignite_rate` | Seconds after lighting, and bursts a second then (when more than `rate`) | 0-10, 0-60 |
| `after`, `after_rate` | Seconds after putting out, and bursts a second just after, falling to none | 0-10, 0-60 |
| `burst` | Most sparks a burst (one to this many) | 1-16 |
| `speed`, `speed_jitter` | Units a second shot out, and the share each may be slower | 0-500, 0-1 |
| `spread`, `aim` | Degrees the cone opens round its axis; the axis: the world's up at 0, the way the blade points at 1, back out of the hilt at -1, turning between (up where the two would cancel) | 0-180, -1-1 |
| `gravity` | Units a second² they fall (negative: rise) | -800-2000 |
| `life`, `size`, `length` | About how long a spark lasts (seconds, ±40%), its width, and its streak's length (units) | 0.05-3, 0.05-8, 0-32 |
| `along` | Where on the blade they come from: 0 the emitter, 1 the tip | 0-1 |

`sparks` may be one set (an object) or a list of up to three, each its own emitter (a
welder's spray and embers drifting off, 19/10/2026); in a list a mistake is named
`sparks[1].speed` and so on. A set with `length` 0 lets go round points, not streaks:
embers. `bolts` (19/10/2026): jagged bolts of the game's electricity (`CElectricity`, as
an efx draws it, crackling into a new shape every frame) from the emitter to a point off
it, struck as sparks' bursts are: `shader` (`gfx/misc/redbolt`), `color`, `opacity`,
`rate`, `ignite`, `ignite_rate`, `after`, `after_rate`, `burst` (1-8), `reach` (units,
0.5-64) and `reach_jitter`, `spread` and `aim` as sparks', `width` (0.05-8), `chaos`
(0-2, how jagged), `branch` (forking), `life` (seconds, 0.02-2), `along`.

A spark set's optional `wander` (units a second², 0-400, default 0) pushes each spark
about with a force of its own, smooth and tangled (`Motion::with_wander`, slowed at 1.6 a
second): embers darting and swirling. `unrest` (19/10/2026, `reach`, `fade` in units,
`far` 0-1): the blade's unrest (granulation, pulled to a steady middle; flares; shimmer;
the frayed edge; the tongues' noise; the arcs) is whole for `reach` units from the
emitter, then calms over `fade` to `far` of itself (`skin_unrest` in `saber.wgsl`). The
arcs hug the blade as it calms (their bulge and zigzag scaled too, not only their light);
`lean` (default 0, -16 to 16) slants the tongues' spikes toward the tip, units along a
capsule radius out. `swing` (default 0, 0-64 units): a swinging blade's unrest reaches up
to `swing` units further, by the blade's swing as `saber_motion.rs` measures it for the
Blade Lab designs (0 to 1, its `motion.x`, folded into the unrest's reach in `skin_at`).
`haze` (default 0, 0-1): the heat haze calms with the unrest, 1 bending the scene only
where the blade is unsettled (`fragment_haze`). `clash` (default 0, 0-64 units) and
`clash_time` (default 0.6, 0.05-5 s): when the blade clashes its arcs' unrest alone (not
the edge, grain, flares or tongues) reaches that much further at once, settling as
(1 - t/clash_time)², from the seconds since the blade's last clash its motion carries
(`motion.y`, `saber_motion.rs`, which every clash below feeds with `BladeMotion::clash`).
A spark set's or the bolts' `clash` (default 0, 0-8) lets that many bursts go at once then
(`Sparks::kick`). A clash is a snapshot's saber block (`impacts::Pool::clashes`, by where
it struck: within a blade's radius and 6 units of its line), a blade's first touch of a
wall (`saber_contacts`, the next frame), the clash flares the motion watches itself, or
the Saber Lab's Clash button (C), which strikes the hand's and the floating copy's blades
and sounds a stock block. A jagged hotspot at the emitter is a spark set of streaks aimed back
into the hilt (`aim` -1, nearly still): a streak is drawn behind its spark, so they fan
out of the emitter up the blade.

`crackle` (19/10/2026, the eighth set): the emitter heard. `buzz` (a game path, or empty
for none) loops under the hum while the blade is lit at `buzz_volume` (0-2), and one of
`snaps` (0-8 game paths, at random) sounds at `snap_volume` (0-2) each time a burst of the
skin's `bolts` leaves the emitter (`saber_sparks` reports its bursts). Heard in the Saber
Lab (`menu_stage/lab.rs`, which also loops the skin's `hum` there, through
`GameAudio::menu_loop` with the menus' idle loop set) and in a match (19/10/2026): saber
submission notes the bolts' bursts and the lit emitters (`saber_smoke::Heard`),
`GameAudio::hear_sabers` plays the snaps where they went (a source of no entity's, a
muted player's left out) and loops the buzzes at the emitters in the next frame's set; the
sounds are decoded as the skins load (`audio_legacy.rs`). The last snap of a sputter
(`flash.die`) sounds in both.

`chroma` (20/10/2026, the eighth set): the skin is a chroma by its own file, as a
catalogue chroma is by its entry (a Saber Lab blade has no entry): it takes its wearer's
saber colour, `color1` or `color2`, the RGB ones too, its colours turned round to that hue
from its corona's rim's (`SkinColor::worn_with`). `effects` (optional, default true) turns
its emitter's smoke, ribbons, sparks and bolts with it (their colours turned where the
wearer's colour is taken), which a catalogue chroma's have none of.

`disintegrate` (19/10/2026 to 04/11/2026, the eighth set) is gone from the format: the
burn on a saber kill became the first finisher, worn with any blade
([Finishers](#finishers)).

`flash` (19/10/2026): a crackling star at the emitter, the Sun's glint gone wild, drawn by
`saber.wgsl` (`skin_flash`) on camera-facing quads of its own (`Instance::flashes`, colour
alpha `FLASH - n`, -16 and below: a glint's tests take them, the black holes' pass leaves
them out). Fields: `color`, `brightness` (0-10), `count` (1-4 at once), `rays` (2-12),
`length` (units, 0.2-24), `width` (a share of the length, 0.01-0.5), `halo` (the heart,
0-4), `rate` (shuffles a second, 0-60: each star's rays thrown to new angles and lengths,
its light to a new flicker, its place jumping), `chaos` (0 even, 1 wild), `jitter` (units
it jumps about its place, 0-8), `lean` (0-1: rays pointing up the blade longer, the ones
pointing back shorter), `along`, `light` (optional, 0-4, default 0: the blade's dynamic light crackles with the stars, `saber_lights.rs` `FlashLight` following `skin_flash`'s own hashes; at a shuffle's brightest they add that share of the light, 1 doubling it), `ignite` (optional, seconds 0-10, default 0), `surge` (0-8) and `grow` (0-2): for `ignite` seconds once the blade lights the stars are up to `surge` more bright and `grow` more big, falling off as (1 - t/ignite)², the light with them (`saber::Ignition` notes when a blade lit; a blade first seen lit is no ignition; the flash instance's red lane carries the share left, `Instance::with_moments`), `die`
(optional, seconds 0-5, default 0: once put out and drawn in all the way, which takes a
match's blade about a second, `Ignition::gone_for`, the stars sputter on without it,
now out, now flaring, out more often and dimmer as they die, the green lane; the Saber Lab
snaps once as it ends) and `burst` (optional: `strength` 0-4 units, `reach` 1-64, `time`
0.05-3 s, `width` 0.2-16: as the blade lights a ring of bent air spreads from the
emitter, drawn on the first star's quad in the haze pass, the blue lane). The Collection's stage lights the menu map and its model with its hands' blades as a match does (19/10/2026). `arcs.root` (optional, 0-1, default 0): the share of strikes
rooted at the emitter, running up the blade for their span, instead of anywhere along it.
| `along` | Where on the blade it comes from: 0 the emitter, 1 the tip | 0-1 |

| Field | Meaning | Range |
| --- | --- | --- |
| `version` | Format version | 1 |
| `glow_profile` | Generated glow: `width`, `peak`, `tail_width`, `tail_peak` (a Gaussian and an optional wider tail, in half-widths); default the neutral pair's | 0.05-1, 0-1, 0.05-2, 0-1 |
| `core_profile` | Generated core: `width` (hot centre, red), `fringe_width` (green); default the neutral pair's | 0.05-1, 0.05-1.5 |
| `glow_image`, `core_image` | Optional images in the pack (PNG, TGA, JPEG, at most 1024 a side) instead of the profiles | game paths |
| `core.white` | Hot centre colour; `white_flare` how much a flare brightens it | 0-4; 0-10 |
| `core.fringe_cool`, `fringe_hot` | Fringe colours, mixed by `fringe_heat` `base + grain × grain` | 0-4 |
| `core.fringe_brightness` | `base + grain × grain + flare × flare` | base 0-10 |
| `core.breathe` | The core's width breathing: `amount × sin(time × rate + along × along + seed × seed)` | amount 0-0.5 |
| `core.tip` | Optional: the rounded tip's length in core half-widths, over which the line narrows on a quarter circle to a point; default 1.5 | 0.5-8 |
| `corona.rim_cool`, `rim_hot`, `inner` | Rim colours (mixed by `rim_heat` `grain × grain + flare × flare`) and the inside colour | 0-4 |
| `corona.inner_width` | How far out the inside colour reaches, in capsule radii | 0.01-1 |
| `corona.inner_mix` | The inside colour's share: `inside² × (base + grain × grain) + flare × flare` | |
| `corona.brightness` | `base + grain × grain + flare × flare` | base 0-10 |
| `corona.reach` | Widest the glow gets, in stock capsules | 1-2 |
| `corona.swell` | How much `grain` and `flare` widen the glow | -2-2 |
| `granulation.octaves` | 1 or 2 octaves: noise at `(along × scale − time × speed, time × evolve + seed × offset)`, summed by `weight` | weight 0-1 |
| `granulation.low`, `high` | The sum's contrast (smoothstep) | 0-1, low < high |
| `flares` | `count` tracks; track n's rate `rate + rate_step × n + rate_seed × seed`, phase offsets `phase_seed`, `phase_step`; lit when its draw ≥ `threshold`; runs from `overshoot` units before the hilt to past the tip; `size + size_jitter × random` long | count 0-8, threshold 0-1 |
| `shimmer` | Up to 2 waves (as `breathe`) widening the corona | amount 0-0.5 |
| `tongues` | Noise at `(along × along + seed × offset, out × out − time × speed)`, from `edge_low` to `edge_high` capsule radii out, brightness `low + range × noise`; past the tip `out` is the distance from the tip and `along` runs on round it | edge 0-2, low < high |
| `arcs.count`, `color`, `brightness` | Lightning arcs at once, their colour and brightness | 0-4, 0-4, 0-10 |
| `arcs.width`, `halo` | A filament's half-width (units; one thinner than a pixel widens to it and dims) and its soft halo's share (four widths wide) | 0.02-1, 0-2 |
| `arcs.reach`, `jag`, `kinks` | How far an arc bulges out and how far its corners zigzag (capsule radii), and corners a unit along: piecewise-linear noise of two octaves | 0-2, 0-1, 0.05-4 |
| `arcs.span` | `low`, `high`: a strike's length, a share of the blade | 0.05-1, low ≤ high |
| `arcs.rate`, `threshold`, `jitter` | Strikes a second per arc, each at a new random place and side (a strike shows when its draw ≥ `threshold`), re-shaped `jitter` times a second | 0.05-60, 0-1, 0-120 |
| `arcs.tip` | The share of strikes that leap from just below the tip into the air (dying out on the way) instead of coming back to the blade | 0-1 |
| `arcs.crawl`, `decay` | Units a second a strike crawls along the blade (negative: hiltward); its light `(1 − age)^decay` over the strike (0 holds it) | -100-100, 0-8 |
| `motes.color`, `brightness` | Motes' colour and brightness | 0-4, 0-10 |
| `motes.density`, `cells`, `rings` | A field of cells, `cells` a unit along and `rings` a capsule radius out (each column staggered), `density` of them holding a mote | 0-1, 0.05-8, 0.5-16 |
| `motes.size`, `stretch` | A mote's half-width as a share of its cell, and how many times longer it is along the blade (`size × stretch` at most 0.5) | 0.02-0.5, 1-8 |
| `motes.drift` | `along` units a second toward the tip, `out` capsule radii a second outward (negative: inward); round the tip they flow round it | -100-100, -10-10 |
| `motes.twinkle` | About how many times a second each twinkles (0: steady) | 0-60 |
| `motes.inner`, `outer`, `focus` | Where (radii out) they appear and are gone; `focus` 1 shows them only toward and round the tip | 0-2, 0-2.5, 0-1 |
| `hue` | `rate` turns a second, `along` turns a unit along the blade, `out` turns a capsule radius out: the glow's and the core fringe's colours turned round the grey axis (brightness and saturation kept), and the light's with the blade's middle | -10-10, -1-1, -4-4 |
| `sputter.ragged`, `scale`, `speed` | How far noise frays the glow's edge (a share of its width), its cells a unit along and how fast it runs | 0-1, 0.05-8, -100-100 |
| `sputter.cut`, `rate`, `threshold` | The most of the blade a sputter cuts off (a share of its length); draws a second, a draw sputtering when its value is at least `threshold` | 0-0.5, 0-60, 0-1 |
| `glitch.split` | How far red and blue part from green, capsule radii (doubled in a glitch) | 0-1 |
| `glitch.blocks`, `rate`, `threshold` | A block's length (units); draws a second, a block glitching when its draw is at least `threshold` (the whole blade when a rarer draw passes halfway past it) | 0.5-40, 0.05-60, 0-1 |
| `glitch.shift`, `flash` | How far a glitched block jumps (radii) and how much it brightens | 0-1, 0-4 |
| `scan.color`, `brightness` | The hologram's wireframe colour and brightness | 0-4, 0-10 |
| `scan.lines`, `speed`, `depth` | Scan lines a unit along, how fast they run (units a second) and how deep they darken the glow and core | 0.05-8, -100-100, 0-1 |
| `scan.rings`, `edge`, `width`, `hollow` | Wireframe rings a unit along (0: none), where its side lines run and their half-width (radii), how much the inside is dimmed | 0-2, 0.1-1.5, 0.01-0.5, 0-1 |
| `scan.jitter`, `jitter_rate` | How far a jitter throws the projection sideways (radii) and draws a second (a quarter of them jitter) | 0-0.5, 0-60 |
| `pulse.rate`, `amount`, `second`, `gap`, `width`, `swell` | A heartbeat on the clock (no seed, so blades and their lights beat together): beats a second, how much a beat brightens, the second beat's share and seconds after the first (which peaks 0.1 s into a cycle), a beat's half-length (s) and how much it widens the glow | 0.1-4, 0-2, 0-1, 0.05-1, 0.01-0.5, 0-0.5 |
| `ghosts.count`, `spacing`, `fade` | Afterimages: the blade's pose kept every `spacing` ms, the last `count` drawn again as glow alone (the plain capsule in the corona's colours, without its noise, flares or parts), each `fade` of the one before; a pose the blade has not moved from is skipped, and so is one fainter than 0.06 | 1-4, 10-250, 0-1 |
| `embers.color`, `brightness` | The embers' colour and brightness | 0-4, 0-10 |
| `embers.density`, `cells`, `fall`, `life` | The share of drip places dripping, places a unit along, how fast they fall (units a second, accelerating) and how long one lasts (s) | 0-1, 0.05-4, 1-400, 0.1-4 |
| `embers.size`, `spread` | An ember's radius (units) and how much its way strays sideways | 0.05-2, 0-1 |
| `veins.color`, `brightness` | The veins' colour and brightness in the glow's inside | 0-4, 0-10 |
| `veins.scale`, `speed`, `width`, `core` | Ridged noise cells a unit, how fast they run along, a vein's half-width (share of a cell) and how bright they show in the core | 0.05-8, -100-100, 0.01-0.5, 0-10 |
| `team.amount`, `red`, `blue`, `none` | The share of its colour the glow and core fringe take from the wearer's team: red, blue, or none outside a team | 0-1, 0-4 each |
| `ambient.amount`, `saturate`, `floor` | The share of its colour taken from the light where the blade is (the grid's ambient and half its directed light at its hilt end, at full brightness), how much its saturation is raised and each channel's least | 0-1, 0-4, 0-1 |
| `glyphs.color`, `brightness` | The glyphs' colour and brightness | 0-4, 0-10 |
| `glyphs.size`, `spacing`, `speed`, `width` | A glyph's height along the blade and the gap after it (units), how fast they scroll toward the tip and a stroke's width (a share of a glyph): the wearer's name, a letter each, a cell left empty between repeats | 1-20, 0-10, -100-100, 0.02-0.3 |
| `star.color`, `brightness` | The tip's glint: its colour and brightness | 0-4, 0-10 |
| `star.rays`, `length`, `width` | Its rays, how far they reach from the tip (units, every other one 0.55 of that from four rays up) and how thick (a share of their length, tapering) | 2-8, 1-24, 0.01-0.3 |
| `star.spin`, `twinkle`, `depth`, `halo` | Turns a second; about how many times a second it twinkles and how deep; the round glow at its heart (a share of the rays' brightness) | -4-4, 0-20, 0-1, 0-2 |
| `orbit.at`, `count` | The glints circle the blade `at` units below the tip (0: round the tip), `count` of them evenly round the circle, every other one 0.7 the size; needs `star` | 0-40, 1-4 |
| `orbit.radius`, `rate`, `tilt` | How far out from the axis (units), turns a second (negative: the other way), and the circle's tilt (radians, the near side higher, so it shows as a ring); a glint behind the blade, near its axis, shows at 0.3 | 0-16, -4-4, 0-1 |
| `holes.color`, `brightness` | Black holes in the glints' places (one at the tip, or circling with `orbit`; not with `star`): the photon ring's and swirl's colour and brightness | 0-4, 0-10 |
| `holes.size`, `lens`, `spin` | The horizon's radius (units; every other hole 0.7 of it), how far past it the scene behind is drawn into a ring (horizon radii; the bending reaches `size × (2.5 + 2 × lens)`), and the swirl's turns a second | 0.2-4, 0-3, -4-4 |
| `wisps.color`, `brightness` | Smoke rising off the blade, whichever way it is held: its colour and brightness | 0-4, 0-10 |
| `wisps.rise`, `speed` | How high it rises before it is gone (units; the glow's quad reaches that much further) and how fast (units a second) | 1-16, 0-60 |
| `wisps.scale`, `curl`, `density` | Noise cells a unit, how much the smoke curls (the noise warped) and how much of it there is | 0.02-2, 0-2, 0-1 |
| `haze.strength`, `scale`, `speed`, `reach` | Heat haze bending the scene behind the blade: the most it bends (units), noise cells a unit, how fast it rises (units a second), how far past the glow it reaches (units) | 0-2, 0.05-4, 0-60, 0.5-16 |
| `bands.color`, `brightness` | Bands round the blade: their colour, and their brightness as a share of the blade's (below 1 darkens) | 0-4, 0-10 |
| `bands.spacing`, `width`, `twist`, `speed` | Units from one band to the next, the share of that a band covers, how far a band slants across (bands a capsule radius out: a barber's pole) and how fast they run toward the tip (units a second) | 0.2-40, 0.02-0.98, -4-4, -100-100 |
| `bands.mix`, `pinch` | How much a band takes its colour over the blade's, and how much it narrows the glow (rings squeezing it) | 0-1, 0-0.8 |
| `segments.count`, `gap` | A tube in sections, and the dark gap between two (units) | 2-12, 0-4 |
| `segments.rate`, `threshold`, `depth`, `hum` | Draws a second; a section whose draw is at least `threshold` drops to `1 − depth` of its light for that draw; a faint unsteadiness all along | 0-60, 0-1, 0-1, 0-1 |
| `film.amount`, `scale`, `speed`, `saturation`, `clear` | A soap film: how much the glow takes its colours, noise cells a unit, how fast they slide, how saturated, and how much the inside clears while the rim brightens | 0-1, 0.02-4, 0-10, 0-1, 0-1 |
| `blobs.color`, `brightness` | Wax blobs in the column: their colour and brightness | 0-4, 0-10 |
| `blobs.count`, `size`, `speed`, `wobble`, `pools` | How many rise (each at its own pace), their size (capsule radii), units a second, how far they sway, and the pools at the hilt and tip they leave and join (radii, 0: none); they merge where they touch | 1-8, 0.1-1.5, 0-40, 0-1, 0-2 |
| `front.color`, `amount` | A colour climbing the blade from the hilt and falling back; how much the blade under it takes it | 0-4, 0-1 |
| `front.period`, `duty`, `hold`, `reach` | Seconds a cycle, the share of it the front is out, the share of that it holds at its height, and that height (a share of the blade) | 0.5-60, 0.05-1, 0-0.9, 0.05-1 |
| `front.soft`, `jag`, `veins` | Its edge's width (units) and how much it wanders, and how much the skin's veins are kept to the part it covers (cracks it spreads through) | 0.1-20, 0-1, 0-1 |
| `sketch.color`, `brightness` | A pencil sketch's ink | 0-4, 0-10 |
| `sketch.rate`, `wobble`, `hatch`, `glow` | Redraws a second, how unsteady the outlines are (capsule radii), hatching lines a unit, the share of the blade's own glow and core kept | 1-30, 0-1, 0-4, 0-1 |
| `hourglass.color`, `brightness` | The sand | 0-4, 0-10 |
| `hourglass.period`, `waist`, `glass` | Seconds the sand takes to pour, the width left at the middle (a share), the glass rim's brightness (in the corona's hot rim colour); the core keeps a quarter | 2-60, 0.05-1, 0-4 |
| `serpent.color`, `brightness` | The serpent's body and head | 0-4, 0-10 |
| `serpent.coils`, `amplitude`, `speed`, `thickness` | Turns round the blade from hilt to tip, how far out (capsule radii), turns a second it slithers, its body's thickness (radii); in front of the blade it hides it | 1-12, 0.2-1.5, -4-4, 0.05-0.6 |
| `strands.count`, `spread`, `sway`, `brightness` | Glass fibres from the hilt, how far they fan at the tip (radii), how much they sway, their light; the core keeps a quarter | 2-32, 0-1.6, 0-1, 0-10 |
| `strands.hue`, `saturation` | Their tips' colours: hues from `hue` over a third of the wheel, slowly turning | 0-1, 0-1 |
| `butterflies.color`, `brightness` | Their wings | 0-4, 0-10 |
| `butterflies.count`, `size`, `flap`, `flight`, `roam` | How many rest on the blade, their size (units), wing beats a second in flight, the share of each one's cycle it flies, how far (units) | 1-8, 0.5-8, 0-20, 0-1, 0-16 |
| `wings.color`, `brightness`, `span`, `rate`, `feathers` | Wings of fire from near the hilt: colour (white-hot where they overlap), light, length (units), unfoldings a second, feathers a side | 0-4, 0-10, 2-16, 0-4, 3-9 |
| `copies.count`, `angle`, `fade`, `period`, `flicker` | Ghost copies a side, their angle (0.1: the outer copy reaches 0.6 of the glow's width at the tip), their light, seconds they drift apart before collapsing with a flash, how much they flicker | 1-2, 0.005-0.1, 0-1, 0.5-30, 0-1 |
| `ink.color`, `body` | A body over the blade: its colour (a chroma's takes the wearer's colour) and opacity | 0-1, 0-1 |
| `ink.outline`, `width`, `edge` | A black outline's opacity, the body's half-width (blade radii) and the outline's thickness (radii) | 0-1, 0.2-2, 0.02-0.6 |
| `ink.point`, `gloss`, `halftone` | The tip drawn to a point over its last units (0: round), a glossy streak sliding along the body, the glow outside the inside printed as dots, that many a unit (0: none); one of body, outline or halftone is needed | 0-20, 0-1, 0-4 |
| `chrome.color`, `width` | A liquid mirror: its tint and half-width (blade radii) | 0-1, 0.2-2 |
| `chrome.ripple`, `beads`, `speed` | How far ripples swell it, beads rolling up it at once, units a second | 0-0.5, 0-8, 0-40 |
| `echo.fade`, `sway`, `rate`, `lean` | One faint copy of the glow beside the blade, there even held still: its brightness, how far it sways (units, the tip more than the hilt), sways a second, how far it leans (radians) | 0-1, 0-6, 0.02-4, 0-0.3 |
| `options` | Up to 6 parts the wearer may switch off ([Options](#options)), each `{"id", "name", "sections"}`: `id` 1 to 24 of `a`-`z`, `0`-`9`, `_`; `name` 1 to 24 printable ASCII characters (the Collection shows it); `sections` 1 to 4 of the file's optional sections (`arcs`, `motes`, `hue`, `sputter`, `glitch`, `scan`, `pulse`, `ghosts`, `embers`, `veins`, `team`, `ambient`, `glyphs`, `star`, `wisps`, `haze`, `echo`, `holes`, `bands`, `segments`, `film`, `blobs`, `front`, `sketch`, `hourglass`, `serpent`, `strands`, `butterflies`, `wings`, `copies`, `ink`, `chrome`, and the eighth set's `smoke`, `ribbon`, `sparks`, `bolts`, `unrest`, `flash`, `crackle`, `disintegrate`), each in the file and in one option only. An option naming an eighth-set section belongs in the sixth additions file, which gives `options` anew: a client from before the eighth set reads the earlier file's and never sees it. Two ids of a skin may not share a bit of the look's 64 (FNV-1a of the id; the log names the pair to rename) | |
| `trail` | Blur trail colour | 0-1 |
| `light.color` | Dynamic light colour (stock gain) | 0-4 |
| `light.flicker` | `amount` and up to 2 `waves` (`rate`, `weight`, `phase` per hilt): brightness `1 − amount + amount × Σ weight × sin(t × rate + phase × hilt phase)` | amount 0-1 |
| `sounds` | `on`, `off`, `hum`, `swings` (three): game paths in the pack | 1-63 characters, relative, no `..` |

Other coefficients are any number from -1000 to 1000.

### Options

Built 11/10/2026 (Sol's request: some Mythical shaders get parts their wearer can
switch off; first the Sun and the Spectral). A skin's file names its parts (`options`,
[Blade-skin files](#blade-skin-files)); every part is on until its wearer switches it
off, and every other SJK player then draws the blade the same way.

| Skin | Parts (id: what it switches) |
| --- | --- |
| Sun | `prominences` Prominences (arcs), `sparks` Rising sparks (motes), `glint` Tip glint (star; "Orbiting glints" since the later additions file, its orbit going with the star), `haze` Heat haze (haze) |
| Void | `holes` Black holes (holes, later additions file only), `stars` Infalling stars (motes), `warp` Warped space (haze), `pulse` Gravity pulse (pulse), `afterimages` Afterimages (ghosts) |
| Spectral | `wisps` Wisps (wisps), `spirits` Souls (motes), `smoke` Smoky edge (sputter), `rim` Cold rim (hue), `echo` Echo (echo), `afterimages` Afterimages (ghosts) |
| Varykino Blossom | `blossom_branch` Blossom branch, `lavender_mist` Lavender mist, `drifting_petals` Drifting petals, `falling_blossoms` Falling blossoms (the petals its swing sheds, [rendering.md](rendering.md#falling-blossoms)), `sakura_droplet` Sakura droplet (the pearl at its tip), `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Thousand Cranes | `red_thread` Red thread, `paper_cranes` Paper cranes (the flock and the cranes a clash startles), `golden_crane` Golden crane, `washi_light` Washi light (the paper's fibres in its glow), `flying_cranes` Flying cranes (the paper cranes its swing releases), `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Hearth | `hearth_flames` Hearth flames, `blaze` Blaze, `hearthstones` Hearthstones (the stones, the drifting ash and the ash its swings shed), `embers` Embers (embers), `heat_shimmer` Heat shimmer (haze), `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Vulptex Crystalfire | `crystal_body` Crystal body, `foxfire` Foxfire, `embers` Embers, `inner_fire` Inner fire, `crystal_dust` Crystal dust, `the_howl` The howl |
| Zeffo Thunderchorus | `charged_plasma` Charged plasma, `thunderhead` Thunderhead, `sheet_lightning` Sheet lightning, `great_bolts` Great bolts, `struck_stars` Struck stars, `the_great_storm` The great storm |
| Moraband Erasure | `erased_space` Erased space, `burning_edges` Burning edges, `violet_ribbons` Violet ribbons, `void_smoke` Void smoke, `the_erasure` The erasure, `unmade_space` Unmade space, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Wellspring Triad | `three_streams` Three streams, `where_they_meet` Where they meet, `beads_of_life` Beads of life, `living_light` Living light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Chirrut Whills | `guardian_s_staff` Guardian's staff, `sound_through_it` Sound through it, `seen_by_sound` Seen by sound, `falling_rain` Falling rain, `the_force` The Force, `guardian_s_cloth` Guardian's cloth |
| Atollon Gathering | `inflowing_streams` Inflowing streams, `drifting_motes` Drifting motes, `gathering` Gathering, `release` Release, `inner_swirl` Inner swirl, `gathered_lights` Gathered lights |
| Wobani Projection | `smoke_puffs` Smoke puffs, `shadow_copy` Shadow copy, `smoke_ribbons` Smoke ribbons, `drifting_smoke` Drifting smoke, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Iridonia Rite | `gates` Gates, `rising_colour` Rising colour, `steam` Steam, `gate_throb` Gate throb, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Rishi Ocean Flow | `deep_water` Deep water, `great_waves` Great waves, `bright_spray` Bright spray, `sea_mist` Sea mist, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Scipio Awakening | `spiky_aura` Spiky aura, `ki_dust` Ki dust, `shimmer` Shimmer, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Nal Hutta Spicevine | `twining_vine` Twining vine, `spice_glow` Spice glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Korriban Ragnos | `obsidian_body` Obsidian body, `dark_heat` Dark heat, `black_lightning` Black lightning, `the_black_flash` The black flash, `cursed_flames` Cursed flames, `bleeding_crystal` Bleeding crystal, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ossus Scroll | `paper_glow` Paper glow, `gold_strokes` Gold strokes, `sepia_wash` Sepia wash, `falling_gold` Falling gold, `gold_splash` Gold splash, `incense` Incense |
| Eshan Echani | `veined_marble` Veined marble, `moving_light` Moving light, `silk_drift` Silk drift, `pale_sheen` Pale sheen, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Toydaria Gale Streamers | `racing_cloud` Racing cloud, `sun_gaps` Sun gaps, `bright_pennants` Bright pennants, `blown_grit` Blown grit, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Rattatak Crimson Surge | `arena_blood` Arena blood, `arteries` Arteries, `red_breath` Red breath, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Hosnian Daybreak | `the_dawn` The dawn, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Rakata Infinite | `god_ki_body` God ki body, `god_aura` God aura, `star_streams` Star streams, `lightning_threads` Lightning threads, `limit_surge` Limit surge, `star_forge_suns` Star Forge suns, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Tatooine Krayt Spine | `bone_spine` Bone spine, `thorns` Thorns, `pearl_fire` Pearl fire, `blowing_sand` Blowing sand, `twin_suns` Twin suns, `the_roar` The roar |
| Lah'mu Sandveil | `packed_sand` Packed sand, `sand_streams` Sand streams, `falling_sand` Falling sand, `the_coffin` The coffin, `dust_haze` Dust haze, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Dromund Kaas Vitiate | `beating_heart` Beating heart, `ego_aura` Ego aura, `destroyers_lightning` Destroyer's lightning, `lightning_surge` Lightning surge, `ego_rising` Ego rising, `drained_life` Drained life |
| Utapau Whirlwind | `spinning_walls` Spinning walls, `wind_streaks` Wind streaks, `caught_dust` Caught dust, `tightening` Tightening, `air_shimmer` Air shimmer, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Umbara Gloaming | `shadow_world` Shadow world, `lume_spots` Lume spots, `far_lightning` Far lightning, `dark_mist` Dark mist, `lost_lights` Lost lights, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Tython Je'daii | `temple_stone` Temple stone, `living_veins` Living veins, `ashla_and_bogan` Ashla and Bogan, `pollen` Pollen, `tython_s_pull` Tython's pull, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Crait Salt Flats | `salt_crust` Salt crust, `crimson_scrape` Crimson scrape, `salt_wind` Salt wind, `the_gust` The gust, `salt_glints` Salt glints, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Sullust Magma | `lava_flow` Lava flow, `magma_bubbles` Magma bubbles, `spatter` Spatter, `heat_glow` Heat glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kijimi Snowfall | `night_ice` Night ice, `frost_creep` Frost creep, `snowfall` Snowfall, `town_glow` Town glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Saleucami Turbolaser | `charging_barrel` Charging barrel, `bolts_up_the_barrel` Bolts up the barrel, `fired_bolts` Fired bolts, `muzzle_flash` Muzzle flash, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Wayland Gravity Press | `pressed_metal` Pressed metal, `rising_stones` Rising stones, `bent_light` Falling grit, `sinking_dust` Sinking dust, `the_press` The press, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Onderon Blazing Strike | `rolling_fire` Rolling fire, `licking_edge` Licking edge, `rising_heat` Rising heat, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Dantooine Grassland | `tall_grass` Tall grass, `sun_patches` Sun patches, `seed_fluff` Seed fluff, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Bracca Rust | `rusted_hull` Rusted hull, `cutting_torch` Cutting torch, `sparks` Sparks, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Mandalore Beskar | `forged_plates` Forged plates, `forge_light` Forge light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Nevarro Ash | `cooling_ash` Cooling ash, `living_seams` Living seams, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Vandor Whiteout | `blowing_snow` Blowing snow, `blue_shadow` Blue shadow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Bendu Equilibrium | `silver_body` Silver body, `silver_steam` Silver steam, `floating_light` Floating light, `haze_shimmer` Haze shimmer, `moving_on_its_own` Moving on its own, `two_currents` Two currents, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Lehon Maelstrom | `inward_body` Inward body, `swirling_space` Swirling space, `caught_motes` Caught motes, `drawn_mist` Drawn mist, `the_maelstrom` The maelstrom, `stretched_starlight` Stretched starlight, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Zonama Starlight | `night_of_zonama` Night of Zonama, `boras` Boras, `seed_partner_glow` Seed-partner glow, `wandering_stars` Wandering stars, `sekot_s_song` Sekot's song, `rising_sparks` Rising sparks |
| Bastila Battle Meditation | `still_deep` Still deep, `shared_breath` Shared breath, `formation` Formation, `unbroken_will` Unbroken will, `calm_spreading` Calm spreading, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Cantonica Champagne | `struck_gold` Struck gold, `rising_bubbles` Rising bubbles, `sweep_of_light` Sweep of light, `bubbles_and_glitter` Bubbles and glitter, `casino_glow` Casino glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Nightsister Ichor | `ghostly_mist` Ghostly mist, `falling_ichor` Falling ichor, `pale_heart` Pale heart, `spirit_breath` Spirit breath, `welling_magick` Welling magick, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Nihilus Hunger | `hollow_blade` Hollow blade, `draining_veins` Draining veins, `stolen_life` Stolen life, `dark_flame` Dark flame, `drawn_mist` Drawn mist, `the_hunger` The hunger, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Rodia Toxic Bloom | `living_stalk` Living stalk, `swelling_pods` Swelling pods, `spore_clouds` Spore clouds, `drifting_spores` Drifting spores |
| Glee Anselm Rockpools | `tide_on_stone` Tide on stone, `caustic_light` Caustic light, `rising_bubbles` Rising bubbles, `drifting_sand` Drifting sand, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Polis Massa Asteroid | `asteroid_rock` Asteroid rock, `ice_through_the_cracks` Ice through the cracks, `ice_mist` Ice mist, `mining_beams` Mining beams, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Lotho Minor Lavaglass | `dark_glass` Dark glass, `rising_melt` Rising melt, `hot_base` Hot base, `heat` Heat |
| Agamar Copper | `old_copper` Old copper, `verdigris` Verdigris, `falling_flakes` Falling flakes, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Sorgan Lake | `still_lake` Still lake, `moving_glints` Moving glints, `krill_glow` Krill glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Iego Lightglow | `moonlit_glass` Moonlit glass, `moving_glow` Moving glow, `angels__lights` Angels' lights, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Sluis Grey | `smoke_grey` Smoke grey, `passing_cloud` Passing cloud, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Arkanis Teal | `slow_swell` Slow swell, `crest_light` Crest light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Peridea Convergence | `split_body` Split body, `twin_streams` Twin streams, `the_imaginary` The imaginary, `bent_space` Bent space, `convergence` Convergence, `annihilation` Annihilation, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Byss Force Storm | `storm_funnel` Storm funnel, `emperor_s_lightning` Emperor's lightning, `storm_arms` Storm arms, `eye_of_the_storm` Eye of the storm, `the_wind` The wind, `storm_surge` Storm surge |
| Lira San Starglass | `dark_glass` Dark glass, `stars_inside` Stars inside, `glass_sparkle` Glass sparkle, `star_wind` Star wind, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Mimban Marshlight | `marsh_mud` Marsh mud, `swamp_gas` Swamp gas, `will_o__wisps` Will-o'-wisps, `slow_rot` Slow rot, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Zygerria Whip | `coiled_whip` Coiled whip, `charge_along_it` Charge along it, `crackle` Crackle, `dust_of_the_arena` Dust of the arena, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Jabiim Siege | `wet_mud_gold` Wet mud-gold, `running_water` Running water, `endless_rain` Endless rain, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Pantora Sunrise | `blue_ice` Blue ice, `first_sun` First sun, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Mon Cala Coral | `living_coral` Living coral, `soft_pulse` Soft pulse, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Tund Sorcerers | `gold_body` Gold body, `sorcery_ribbons` Sorcery ribbons, `violet_flame` Violet flame, `violet_motes` Violet motes, `the_gathering` The gathering, `threads_of_fate` Threads of fate, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Haruun Kal Wildheart | `raging_body` Raging body, `green_storm` Green storm, `lashing_lightning` Lashing lightning, `rising_rocks` Rising rocks, `eruption` Eruption, `fireflies` Fireflies, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kalevala Darksaber | `black_blade` Black blade, `crackling_edge` Crackling edge, `black_smoke_and_wind` Black smoke and wind, `black_banner` Black banner, `the_flare` The flare, `black_lightning` Black lightning |
| Ruusan Thoughtbomb | `reforged_body` Reforged body, `pale_flames` Pale flames, `dawn_light` Dawn light, `silver_sparks` Silver sparks, `the_dawn` The dawn, `kyber_fragments` Kyber fragments, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Garel Corona | `sun_corona` Sun corona, `corona_fire` Corona fire, `chakra_motes` Chakra motes, `rising_heat` Rising heat, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Trandosha Scorekeeper | `scaled_hide` Scaled hide, `ritual_score` Ritual score, `blood_mist` Blood mist, `clawed_spray` Clawed spray, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Wroshyr Rootwood | `wroshyr_bark` Wroshyr bark, `green_sap` Green sap, `climbing_roots` Climbing roots, `leaves` Leaves, `spores` Spores, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Iktotch Foresight | `clear_sight` Clear sight, `coming_strokes` Coming strokes, `what_will_be` What will be, `glass_air` Glass air, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Vardos Phrik | `phrik_alloy` Phrik alloy, `live_current` Live current, `crackle` Crackle, `stray_arcs` Stray arcs, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kuat Shipwright | `hull_steel` Hull steel, `running_lights` Running lights, `scaffold_beams` Scaffold beams, `welding` Welding, `drive_glow` Drive glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kowak Silkweb | `wrapped_silk` Wrapped silk, `green_venom` Green venom, `loose_threads` Loose threads, `dew` Dew, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Nar Kreeta Lightspeed | `star_drive_plasma` Star-drive plasma, `star_lines` Star lines, `the_jump` The jump, `blue_shift` Blue shift, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Manaan Tidebreaker | `kolto` Kolto, `rising_bubbles` Rising bubbles, `bubbles_round_it` Bubbles round it, `deep_light` Deep light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Cato Neimoidia Gilded | `gilded_filigree` Gilded filigree, `sliding_sheen` Sliding sheen, `falling_leaf` Falling leaf, `bridge_haze` Bridge haze, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Dxun Beastfang | `beast_s_hide` Beast's hide, `hide_breathing` Hide breathing, `rising_embers` Rising embers, `hunting_haze` Hunting haze, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Fondor Forge | `hull_plate` Hull plate, `live_welds` Live welds, `spark_showers` Spark showers, `shipyard_glow` Shipyard glow, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Plazir Lanterns | `warm_dusk` Warm dusk, `soft_lights_rising` Soft lights rising, `sky_lights` Sky lights, `evening_air` Evening air, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Coruscant Skylanes | `night_city` Night city, `traffic_lanes` Traffic lanes, `air_traffic` Air traffic, `neon_sign_light` Neon sign light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Savareen Starfall | `dusk_sky` Dusk sky, `coaxium_shimmer` Coaxium shimmer, `falling_stars` Falling stars, `sea_light` Sea light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Hijarna Opal | `ruined_stone` Ruined stone, `opal_fire` Opal fire, `stone_dust` Stone dust, `old_light` Old light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kef Bir Waves | `storm_sea` Storm sea, `breaking_foam` Breaking foam, `flung_spray` Flung spray, `wet_air` Wet air, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Shili Thornfang | `horn_spiral` Horn spiral, `red_markings` Red markings, `tall_grass` Tall grass, `hunting_wind` Hunting wind, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Florrum Brigand | `painted_iron` Painted iron, `pirate_slashes` Pirate slashes, `black_exhaust` Black exhaust, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Abafar Pallor | `blank_rock` Blank rock, `rhydonium_veins` Rhydonium veins, `void_haze` Void haze, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Teth Leafdrift | `jungle_bark` Jungle bark, `canopy_light` Canopy light, `falling_leaves` Falling leaves, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Malastare Podrace | `racing_stripes` Racing stripes, `engine_heat` Engine heat, `blue_thrust` Blue thrust, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Anaxes Clonefront | `armour_plate` Armour plate, `legion_stripe` Legion stripe, `tracers` Tracers, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Lola Sayu Watchbeam | `prison_iron` Prison iron, `scanning_bar` Scanning bar, `searchlight` Searchlight, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Quell Embers | `wreck_black` Wreck-black, `glowing_burn` Glowing burn, `falling_cinders` Falling cinders, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Skako Pressure | `crushing_bands` Crushing bands, `squeezed_light` Squeezed light, `leaking_vapour` Leaking vapour, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Cerea Meadow | `grass_in_the_wind` Grass in the wind, `sunlit_tips` Sunlit tips, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Corulag Slate | `split_slate` Split slate, `cold_seams` Cold seams, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Bothawui Amber | `honey_flow` Honey flow, `trapped_flecks` Trapped flecks, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Hapes Rose | `silk_folds` Silk folds, `fold_light` Fold light, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ithor Verdant | `living_leaf` Living leaf, `sap_in_the_veins` Sap in the veins, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Mirial Saffron | `woven_saffron` Woven saffron, `warm_thread` Warm thread, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Zeltros Violet | `shot_silk` Shot silk, `colour_turning` Colour turning, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Socorro Ochre | `desert_strata` Desert strata, `drifting_sand` Drifting sand, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Muunilinst Brass | `engraved_brass` Engraved brass, `rolling_polish` Rolling polish, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Dorin Pewter | `brushed_pewter` Brushed pewter, `moving_sheen` Moving sheen, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Moon blade | `moonlight` Moonlight, `moon_glitter` Moon glitter, `silver_tide` Silver tide, `falling_stars` Falling stars, `night_mist` Night mist, `lunar_eclipse` Lunar eclipse, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kathol Rift | `twisted_ribbon` Twisted ribbon, `space_and_gold` Space and gold, `echoes_of_time` Echoes of time, `flow_streams` Flow streams, `falling_upward` Falling upward, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Fondor Mass Driver | `steel_rails` Steel rails, `plasma_slugs` Plasma slugs, `charged_arcs` Charged arcs, `rungs` Rungs, `muzzle_flash` Muzzle flash, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Exar Kun Temple | `gold_seams` Gold seams, `temple_needles` Temple needles, `massassi_haze` Massassi haze, `spirit_fire` Spirit fire, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Naga Sadow Alchemy | `boiling_blood` Boiling blood, `thrown_ropes` Thrown ropes, `alchemic_fume` Alchemic fume, `falling_drops` Falling drops, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Orbalisk Carapace | `living_shells` Living shells, `feeding_lightning` Feeding lightning, `heat_motes` Heat motes, `parasite_breath` Parasite breath, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ziost Silence | `black_cracks` Black cracks, `rivers_of_life` Rivers of life, `falling_world` Falling world, `the_silence` The silence, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Nathema Deadworld | `charred_blade` Charred blade, `dead_fire` Dead fire, `ember_storm` Ember storm, `black_smoke` Black smoke, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Amphistaff Serpent blade | `chitin_plates` Chitin plates, `venom` Venom, `spit` Spit, `serpent_breath` Serpent breath, `living_coils` Living coils, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Vergence Scatter | `deep_space` Deep space, `nebula_clouds` Nebula clouds, `loops_of_light` Loops of light, `far_light` Far light, `wormhole` Wormhole, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Voxyn Scream | `sound_on_the_body` Sound on the body, `sound_in_the_air` Sound in the air, `howl` Howl, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ysalamir Hush | `scaled_hide` Scaled hide, `olbio_branches` Olbio branches, `firefly_rivers` Firefly rivers, `moss_aurora` Moss aurora, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Rakghoul Plague | `infected_flesh` Infected flesh, `bursting_boils` Bursting boils, `spores` Spores, `plague_breath` Plague breath, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Taris Undercity | `city_tower` City tower, `speeder_lanes` Speeder lanes, `searchlights` Searchlights, `smog_and_rain` Smog and rain, `the_bombardment` The bombardment, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Telos Restoration | `clean_light` Clean light, `climbing_shoots` Climbing shoots, `pollen` Pollen, `falling_leaves` Falling leaves, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Malevolence Ion Beam | `charging_channel` Charging channel, `plasma_tendrils` Plasma tendrils, `the_guns_fire` The guns fire, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Massassi Altar | `carved_stone` Carved stone, `altar_fires` Altar fires, `altar_smoke` Altar smoke, `sparks` Sparks, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Sith Tremor Sword | `trembling_steel` Trembling steel, `cracked_air` Cracked air, `shed_sparks` Shed sparks, `old_rage` Old rage, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Eye of Aldhani | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ferrix Foundry | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Kenari Crash | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Sarlacc Pit | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Cloud City Dusk | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ring of Kafrene | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Hypergate Drift | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Bardotta Calm | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Alzoc Ice | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Fest Steppe | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Ansion Plains | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Sernpidal Dust | `x` x, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |
| Angel | `heavenly_clouds` Heavenly clouds, `angel_feathers` Angel feathers, `golden_halo` Golden halo, `swing_and_clash_bursts` Swing and clash bursts (a signature's parts) |

- Kept in `cg_saberSkinOptions` (archived, default empty;
  [saber_skin_options.rs](../crates/sjk-viewer/src/saber_skin_options.rs)): one
  `<skin>.<option>` word for each part switched off, sorted, so every skin keeps its
  choice while another is worn (`saber_sun.haze saber_spectral.wisps`).
- Chosen on the Collection's Shaders tab ([sjk-ui.md](sjk-ui.md#collection)): under an
  owned skin's words, "Parts" and a tick for each, in two columns (three past four);
  a click or the number keys 1 to 6 switch one, on the model at once. In the console,
  `saberskin <id> parts` lists them with their state and `saberskin <id> <part> on|off`
  switches one, so every menu style has it.
- Sent with the look: the worn skin's parts switched off go as `saber_off` (at most 8
  ids, sorted), only when there are some (a hub from before refuses the field), and come
  back in presence and feed looks. In a frame a choice is 64 bits, an option id's bit
  its FNV-1a hash modulo 64 (`blade_skin_extras::option_bit`, distinct within a skin by
  the file check), so the looks stay `Copy` and nothing is allocated; a viewer whose
  pack lacks an option ignores it.
- Drawn: the wearer's choice and the skin's options give a mask of sections switched off
  (`SkinColor::off`, `BladeSkinDef::off_mask`), carried by each instance (attribute 9);
  `saber.wgsl` zeroes those sections' lanes as a file without them has
  ([rendering.md](rendering.md#saber-blade-skins)), and the CPU leaves out the parts it
  makes itself: afterimages, the echo, the glint, the light's beat and hue turning, the
  quad's room for wisps and haze, and of the eighth set the smoke, ribbons, sparks, bolts,
  the emitter's stars and their light, the crackle and the burning (`SkinColor` holds none
  of them). The mask has 64 bits, the instance two words (`saber.wgsl` `off.x`, `off.y`):
  the eighth set's sections take the second, of which the shader reads the unrest's alone.

Verified by unit tests (the cvar's words, the look carrying the worn skin's choice and
reading it back, a muted slot showing none, the option mask and room, the CPU's parts
left out, the instances' mask, the command, the file checks) and the world shots
`duel6_mythical_parts` (each skin with every part on beside every part off, close up
and upright, and the haze checked to bend the scene behind the Sun: about 87,000 pixels
change with it on, none of the glow's) and `duel6_sjk_collection` (the Parts grid of
the Sun and of the Spectral, one part off); not yet against the deployed hub or in a
live match.

### Extras

Built 26/10/2026 (Sol: a special trail for the Sun blade, the plasma by default, "the Solar
card unlocks all the other customizations", and likely the same for other blades later).
An extra is a part of a signature skin that is off until its wearer switches it on, and
that only a holder of a given card in a given finish may switch on
([blade_extras.rs](../crates/sjk-viewer/src/blade_extras.rs)).

| Skin | Every wearer | Extras (id: what it does) | Unlocked by |
| --- | --- | --- | --- |
| Sun | the plasma trail | `flare_trail` Flare trail (flames and loops licking past the tip), `swing_embers` Swing embers (sparks flung off the swing), `heat_wake` Heat wake (the air behind the trail shimmers) | a Solar copy of SJK Alpha 000 (Sol) |
| Moon blade | the moonlight trail | `moonshadow_trail` Moonshadow trail (the eclipse's dark band and diamond flash), `starfall_trail` Starfall trail (shooting stars streaking out of the swing), `moonlit_wake` Moonlit wake (the air behind the trail shimmers) | a Solar copy of SJK Alpha 002 (Creyon) |

- **Declared** by the signature's header: `// @trail plasma` (the trail every wearer gets),
  `// @extras {"card":..,"card_name":..,"finish":..}` (the card that unlocks them) and an
  `// @extra {"id":..,"name":..}` line each, with what it does: `"trail"` (another trail,
  [rendering.md](rendering.md#saber-trails)), `"sheds"` (a `@petals` spec without its part,
  [rendering.md](rendering.md#falling-blossoms)) or `"wake": true`. At most 6 extras;
  a test checks every line parses and that no extra's bit takes an option's or another
  extra's.
- **Kept** in `cg_saberSkinOptions` with the parts: a word there names a part switched off
  and an extra switched on, so a word always names a choice that differs from its default
  (`saber_skin_options.rs`). In a frame both ride on the same 64 bits.
- **Gated** in the client before the look is made (`blade_extras::gated`): the words for
  the extras of a skin whose card the own hub profile does not list in its finish are left
  out, so they are neither drawn for the player nor sent. The Collection lists the extras
  after the parts, each behind a padlock while locked, with a line saying what unlocks them;
  a locked one takes no click. `saberskin <id> parts` lists them too ("locked" until the
  card is held) and `saberskin <id> <extra> on` refuses while locked, saying why.
- **Sent** as `saber_on` (and `saber2_on` for a second saber's own skin), only when not
  empty: the hub keeps them only while the key holds the card in its finish and drops
  them otherwise ([identity.md](identity.md)). A viewer reads `saber_on` for extras only and
  `saber_off` for parts only (`saber_skin_options::look_bits`), so an extra named among the
  parts switched off, which the hub does not check, turns nothing on.

Verified by unit tests (the Sun's header, the words and lists, the look round trip and the
lists read for their own kind, the gate, the colour's trail and wake, the command's lock)
and the hub's tests (dropped without a Solar copy of the card, a Holo copy or a Solar copy
of another card, kept and relayed with one); not yet in game, nor against the deployed hub.

### Illuminate for others

Built ([illuminate.rs](../crates/sjk-viewer/src/illuminate.rs)). Every lit look puts a
holocron by that player's left shoulder, placed from their entity's interpolated
origin, eye height (from its box, so crouching lowers it) and view yaw, with the same
fade, bob, spin and trailing as one's own: the local holocron and one per other slot
step through the same code, each slot's bob a little out of step. The cube always
shows for another player while the game draws them; dead, hidden, cloaked or out of
the snapshot, it goes out where it was. Every cube shows, but only the four nearest
the camera add their warm light, so the frame's 32 lights stay for the weapons.
Following (spectating) another player, one's own holocron goes out and the followed
player's, if their look has it lit, floats by the view's eye as one's own does (their
entity is not in the snapshot), showing only its light in first person.
Unit tests and an off-screen shot on duel6 (two holocrons placed for remote players
without a session); not yet seen with real players, nor in follow mode.

### Shaders tab

Players read blade skins as saber **shaders** (10/10/2026, Sol's name, so body
shaders can follow): the player-facing words say "saber shader", the code and the hub
keep "blade skin" and the ids.

Built 08/10/2026 as the Unlockables page, the Profile screen's Collection tab in the
SJK UI; since 10/10/2026 the [Collection](sjk-ui.md#collection)'s Shaders tab
([collection_shaders.rs](../crates/sjk-viewer/src/collection_shaders.rs), its swatches
in [collection_swatch.rs](../crates/sjk-viewer/src/collection_swatch.rs)). An SJK page
in the SJK UI's look in every menu style; in the SJK UI the main page's and the
in-game menu's Collection open it, with the classic menus `unlockables` (and the
Profile page's See the collection) open it on its own:

- a wall of tiles under a heading to each tier, or a list whose column heads sort it
  (25/10/2026, the Collection revamp: [Collection](sjk-ui.md#collection) has the layout,
  keys and `ui_shaderList`): the stock blade (in the player's
  `color1`), then every blade skin the player may see (a Unique one only while they
  hold it, [Unique shaders](#unique-shaders)), the rarest [tier](#tiers) first, with a
  live swatch framed in its tier's colour. An owned skin's swatch shows the blade as the game draws
  it (Creyon's round 13, 19/10/2026: the drawings did not look like the blades): every
  skin a swatch shows is drawn live by the blade renderer, its own pipelines for a
  signature, into a cell of one picture (16 cells, two columns of eight, one pass,
  [blade_pictures.rs](../crates/sjk-viewer/src/blade_pictures.rs)), which the UI adds over
  the swatch's glass (`BLADE_PICTURES_TEXTURE`, clipped with the list). Since 25/10/2026
  (for weak PCs) a frame draws only the cells that need it: the hovered or chosen row or
  card's and a large swatch's every frame (`blade_pictures::focus`, from the Shaders tab,
  the Forge, the Saber tab and a holocron's shader), every cell while two or fewer are in
  use, a new one at once, and the others 30 times a second, each at its own moment so
  the frames share them, six of them a frame at most, the longest waiting first (a GPU
  too slow for 30 a second draws a few a frame in turn rather than all of them). The UI
  over them is still drawn every frame, so only those blades' animation steps: 30 times
  a second, about 26 for the grid's fourteen at 60 FPS, fewer on a slower frame. A cell
  drawn alone is cleared in its rectangle and the kept cells shielded by depth, so no
  glow adds into a kept picture. Every shader owned, `duel6_blade_pictures_cost` at
  1920x1080: on an RTX 5080 the Shaders grid's frame (fifteen pictures) went from 2.16,
  2.97 and 3.41 ms to 1.69, 2.11 and 2.18 (three alternating runs; the list, six, 1.76 to
  2.14 against 1.65 to 2.08); on the PC's integrated Radeon from 251 to 196 ms, where
  each picture costs about 7 to 10 ms and the Medals tab's frame is 69 (such cards
  usually have the menu pictures on, and the drawings). Until a skin has a cell (the
  first frame, more than 16 swatches at once) the swatch is the drawing:
  (drawn with the UI's shapes from its loaded file's
  colours, flicker and flares, moving: a breathing corona, flame loops and granules, a
  flare running hilt to tip, and the file's lightning arcs, motes and turning hue when
  it has them; grey and still under a padlock when locked; owned but with its pack not
  loaded yet, a neutral still blade; a chroma in the player's colour, with its colour
  wheel before its name), its name and its state (Worn, Yours, Locked) under it, its
  tier's name at the right of that line in its colour. The wall's tiles and the list's
  rows draw a blade as a thin strip (`swatch::strip`, a handful of shapes, so thirty
  tiles stay cheap): the live picture when owned, else a still blade in the skin's own
  colours, dimmed when not held (25/10/2026: a wall of locked shaders all grey told
  nothing apart), grey while its pack has not come. A click chooses a tile or a row and
  previews it (a double click wore it until 21/10/2026: Creyon found wearing on a click
  too easy to do by mistake, and the double click unclear), and Equip, Unequip or Enter
  wear it; resting the pointer never chooses one (31/10/2026; from 21/10 it chose one
  over its blade strip), only its hover card shows;
- until 25/10/2026 a rack of six rows of 106 and a grid of cards five by three
  (10/10/2026) stood there, `ui_shaderView` choosing (dropped from saved profiles since,
  so every player starts on the wall);
- the chosen saber on its own (17/10/2026, Sol's request: "see and turn the saber, we
  don't need to see the character"), over the clear right of the screen
  (`SABER_AREA`): the hilts the stage model would hold float before the backdrop
  camera, lying across the window, rolling slowly about their length, and the model is
  not drawn ([menu_stage/lone.rs](../crates/sjk-viewer/src/menu_stage/lone.rs)); over a
  match the page's live preview shows them the same way (lightsaber creation's
  showcase). A drag that starts on the saber turns it: across about the window's
  vertical, up and down about its length; let go, it keeps the drag's speed, slows to
  its own roll, and after 1.4 s its tip swings back to where it rests, a little towards
  the viewer ([collection_spin.rs](../crates/sjk-viewer/src/collection_spin.rs)). It
  stands as far off as makes the longest saber 600 frame pixels long, so a staff and a
  single saber fill the same room. A locked one shows too, as a preview this screen
  alone draws (`PreviewSkin`; nothing is set or sent), with a "Preview: not yours yet"
  pill; "Drag to turn" sits just over the card;
- under it, the shader's card (17/10/2026), framed and banded in its tier's colour and
  alive as its tier is (the rarity effects): the tier's mark and name, its edition
  with a star, its name and its state pill (a lightsaber Worn, a tick Yours, a padlock
  Locked; the name ends short of the pill, ellipsised), its facts as chips with
  [SJK Symbols](sjk-ui.md#symbols) icons (a chroma's colour wheel and "Takes your
  colour", the copies held, where it comes from: the medal that brings it, a holocron
  drop or an SJK team gift, and its saber smith), what it is in full (the card grows
  upward to hold its words, 25/10/2026), owned (a tick and `Yours
  since dd/mm/yyyy, from the SJK team` with the team's note, or `..., with your Bug
  Hunter medal`) or how to get it (a padlock), and for a locked shader a holocron may
  drop, where it drops: a chip for each holocron tier that may bring it, in that tier's
  colour, with the chance an opening of one does (the hub's odds split evenly among
  the droppable shaders of its tier, `cards::loot::shader_in`); then its parts and, at
  the card's foot, Equip, Unequip or Wear the stock blade, which set `cg_saberSkin`.
  While the player holds two sabers or a staff (`saber_skins::local_pair`, from the model
  the stage draws, or `saber2` naming a hilt), a target switch tops the saber's room: two
  pills, 1st saber and 2nd saber (1st half and Other half for a staff, told by its blades'
  directions as above), each with what it
  wears ("Same as the 1st" for none of its own), the one being dressed in gold. A click on
  a pill, or a click (not a drag) on the sabers themselves (the upper saber is the first;
  on a staff the half nearer the click: the stage keeps where it drew each half's middle on
  screen as the staff turns, `saber_skins::staff_half_at`, review of 22/10/2026), chooses it and moves the choice to the row that side wears, so the card and the sabers
  show what it has on; the pills are hit before the sabers' drag area under them. Each
  side is dressed on its own (Creyon, 21/10/2026): Equip puts the chosen shader on that side
  only and the second's Unequip leaves it the stock blade. While the second has no skin of
  its own (`cg_saberSkin2` empty) it follows the first, so dressing the first dresses both,
  as `saberskin` does; once the second was dressed, it keeps its own (review, 22/10/2026:
  a player who never used the switch no longer finds the second half left on the old skin
  and `cg_saberSkin2` written unasked). The first's skin chosen for the second is sent as
  no `saber2`, so `saber2` goes only when the two differ. The own look
  is read again as soon as `cg_saberSkin` or `cg_saberSkin2` changes
  (`Looks::own_settings_changed`), so the sabers show the new skin at once. The sabers show
  the shader on
  the target, the other keeping what it wears, the second laid below the first (a staff
  shows both halves on its one hilt), and the card's Equip and Unequip work on that side,
  for the second setting `cg_saberSkin2` (to the shader, or `stock` from the stock row and
  from Unequip); Enter does the same. A row the second wears
  says "2nd blade". Holding one saber of one blade, the room shows no switch and no row says
  "2nd blade"; a second skin chosen before waits, and a line says it shows with two sabers or
  a staff (chosen in the Profile's Saber tab, `saber1` and `saber2`).
  Beside the pills a "Both sabers" box ("Both halves" for a staff; Creyon, 21/10/2026)
  makes Equip dress both sides at once: both pills light, the sabers show the chosen shader
  on both, and Equip sets `cg_saberSkin` to it and `cg_saberSkin2` to empty (the second
  follows the first; `PanelAction::WearBoth`); with both wearing it, Unequip leaves both the
  stock blade ("Unequip both"), and the stock row reads "Stock on both". The box is kept
  for the session; choosing a side (a pill or a click on a saber) unticks it, a click on a lone saber does not (review of 22/10/2026). Its label is
  drawn in the face it is measured in, so it is never cut short ("Both sab...").
  Only a click chooses and previews a shader, on the wall, the list and the Forge's tiles;
  resting the pointer shows its hover card and nothing else (Creyon, 31/10/2026: going to
  Equip, the pointer crossed other blades and changed the choice; from 21/10 resting on a
  row's blade chose it).
  Once the hub's economy is known, the foot also leads to the Collection's Forge
  ([cards.md](cards.md#the-forge), 21/10/2026; Shatter and Craft stood here before):
  "Craft in the Forge" for a shader of the trade not held, with its cost, or "Break down
  in the Forge" for one of the player's own copies, with what a copy gives; a click shows
  the Forge on that section with the shader chosen.
  The rack's and the grid's state lines carry the same icons, and the grid's tier line
  its rarity mark. The classic menus have no stage: the area shows the swatch, larger;
- with the identity off, no hub or no answer yet, the tab says so and no skin is
  owned.

The Staff page lists every unlockable for the player found (the Unique ones too), with
Unlock (sending the note field's text) and Relock ([identity.md](identity.md#staff)), two to a row of 36
pixels (10/10/2026) so the whole catalogue (fourteen) fits above the keys, each with the
one button that applies.

Verified: unit tests (keys, pointer, the worn one chosen on opening, equip and unequip,
a locked one previewed but never equipped, every row and its words within the canvas
at 1080p, 4K, 4:3 and 21:9, the swatch moving and staying inside its frame, the Staff
page's Unlock and Relock; the grid's switches and V, its arrows in two directions, its
wheel, a click on a card, every card above the keys at the four shapes) and the
world shot `duel6_sjk_collection` (the Sun worn on the stage, the Storm previewed, the
grid with five owned, a 4:3 window and its grid, the in-game preview, the classic
menus; since 17/10/2026 the saber alone and `duel6-collection-shaders-turned`, turned by
a drag) with the hub's pack; unit tests for the lone saber's place in every window
shape and field, its turn, drag, fling and settle, and the card's room; not tried in
the game.

### Saber tab's blade choice

Built 10/10/2026 (Sol's request: choose an unlocked blade skin where the saber is
made), [player_menu/blade_skins.rs](../crates/sjk-viewer/src/player_menu/blade_skins.rs),
drawn by the Saber page ([sjk-ui.md](sjk-ui.md#character)). In the SJK UI the
Profile screen's Saber tab has a Blade row: the stock blade in the first blade's
colour, then every blade skin the player's own hub profile lists, in catalogue
order, each drawn as its Collection swatch shrunk (`swatch::small_blade`, the same
drawing scaled; the stock one `swatch::small_stock`). Locked skins are not listed
there, the Collection shows them; with the identity off, no hub, the hub not
answered yet or no skin owned, only the stock blade shows, with a line saying why.
What the player owns is read twice a second while the tab shows (and when the screen
opens), without copying the identity's state (`player_identity::with_snapshot`).

Picking a blade (a click, or Left and Right on the row) sets `cg_saberSkin` through
`unlockables::wear`, which the Collection's Equip and Unequip use too, by the same
rules (`Holdings::can_wear`, `Holdings::worn_blade_skin`, which the profile card
also reads); the stock blade sets it empty. A click on the blade already shown
chosen writes nothing, so a skin chosen before the hub answered stays chosen. The
stage model (or the live preview, opened from a game) wears the skin once the looks
read the setting again, within half a second, as after Equip. The classic menus are
unchanged.

Verified: unit tests (the stock blade and the owned skins offered in catalogue order,
locked and unknown ids left out, picking and stepping writing the setting, a pending
choice kept, the hint without skins or identity, a click on a swatch wearing it, the
page with every skin carrying every effect within the screen's canvas, Dual too) and
the world shot `duel6_sjk_saber_page` with the hub's pack (`SJK_TEST_PACKS`); not
tried in the game.

### Saber sounds

Built 23/10/2026, branch `personal/saber-sound-pick` (Nib's idea of 18/10/2026 and hub
report #109, approved by Sol on 23/10/2026): a player chooses the saber sounds they are
heard with apart from the shader they wear, and every SJK player hears that choice.

- `cg_saberSounds` (archived, default empty;
  [saber_sound_choice.rs](../crates/sjk-viewer/src/saber_sound_choice.rs)): empty follows
  the shader (the sounds come with the blade, as before), `stock` the stock saber sounds
  whatever the blade, or a blade skin's unlock id (`saber_void`) for that shader's ignition,
  switching off, hum and swings whatever the blade, the stock one included. A shader's
  sounds hold only while the own hub profile lists it, as for wearing it
  (`SoundChoice::own`); one not owned (or taken back) follows the blade, the setting kept.
- Chosen on the Profile screen's Saber tab, the Sounds row under the Blade row
  ([player_menu/saber_sounds.rs](../crates/sjk-viewer/src/player_menu/saber_sounds.rs),
  [sjk-ui.md](sjk-ui.md#character)): "Follow shader (<the blade worn>)", "Stock sounds",
  then the owned shaders, the rarest tier first as the Blade row lists them (the same
  holdings). Left and Right (or a click on the control's halves) write the setting;
  Preview, or Enter on the row, plays the choice's ignition and hum once, as the worn
  blade would be heard with it (a shader whose pack is not loaded plays the stock
  ones). An SJK UI page, drawn in its look in every menu style; the classic menus set the
  cvar from the console.
- Sent with the look as `saber_sounds`, only when not following (`Worn::to_look`), and
  read back from presence and feed looks (`Worn::of`; an unknown id, a newer client's
  shader, follows the blade). A look without the field, from an older client or hub,
  follows the blade: those players are heard as before.
- Heard: each slot's sound set is the blade skin `SoundChoice::sound_skin` gives for
  their blade and choice (`Looks::sound_skin_id`; the local player's own from its gated
  look), looked up among the loaded skins and kept beside the blades in `SaberSkins`
  (`follow_looks_and_sounds`, `set_local_with_sound`), which hands the audio adapter its
  per-client table (`sound_sets`) and the animation swings their set (`sound_of`). The
  rest is the existing path: `s_saberSkinSounds` (With normal, Only skin, Off) applies to
  the chosen set as to the blade's; a duel's stock blades (`cg_duelStockBlades`) and a
  player muted here are heard stock; a set whose pack is not loaded plays the stock
  sounds.
- Hub (deployed 24/10/2026): a hub from before refuses `saber_sounds` with 400
  `bad_body`; the client then sends the look again at once without it and leaves it out
  (`look_sounds_unsupported` in the identity service), so the blade, its parts and the
  toys still reach everyone, who hear the blade's sounds. The hub takes an optional
  `saber_sounds` string in `POST /v1/look` (`""`, `"stock"` or a blade-skin id of the
  Unlocks table, else 400 `bad_look`; an id the key does not hold is 403
  `sounds_not_unlocked`, which the client answers by sending the look without it until
  the profile's unlocks change), keeps it with the look, gives it back in presence and
  look events only when not empty (so older clients see the same JSON), and clears it
  from the live look, with an event, when the skin it names is taken back from the key.
  Older clients never send it.

Verified by unit tests: the resolution (follow, stock, another shader on a skinned or
the stock blade, an unknown id and a look without the field following the blade, a
muted slot stock), the own choice gated by ownership and sent and read back, the
per-slot table and the swings' set apart from the blade and stock in a duel, the Sounds
row (stepping and clicking write the setting, Preview and Enter ask for the right set,
Single and Dual within the canvas), the look's body, the feed's field, and the service
dropping the field for a hub that refuses it or a set the key lacks while still sending
the blade. World shot `duel6_sjk_saber_page` (the Sounds row with the Void's sounds under
the Storm blade). Not verified: heard in a game, the preview by ear, another player's
choice through a hub (the hub's side is tested in the hub's own suite).

## Finishers

Sol's decision, 04/11/2026: what a saber kill does to the body is a category of its own,
not part of a blade skin, so any blade can wear it and it can drop later. The client's
catalogue is `crate::finishers` (`finishers.rs`), the hub's the unlocks of kind `finisher`.

- **Disintegrate** (`finisher_disintegrate`, Mythical, by Milamber, from his Saber Lab
  blade): the victim of the wearer's saber kill (a thrown saber's too, `MOD_SABER`) burns
  away as a disruptor's kill does, at once. The server knows nothing of it: every SJK
  client that sees the killer wear it starts it from the kill feed
  (`GpuState::observe_saber_kills`, `disintegration::Forced`), drawing the frozen pose, burn
  and smoke the disruptor's take (`track_disintegration`), hides the victim after 1.5 s
  until they live again, and hides the body the server copies of them as they respawn
  (`corpse_actors.rs`, its `ircg` body command, which comes before the body is drawn). The
  server and other clients keep the corpse. Only a killing blow does it. A reconnect or a
  new timeline forgets the burns pending (`reset_live_presentation`).
- **Wearing:** one at a time, `cg_saberFinisher` (archived, empty for none), worn and sent
  only while the own profile lists it. Collection > Shaders has a Finishers pill beside
  Saber (and Body, to come): its page lists each finisher as a card, Equip or Unequip
  while held, how to get it otherwise.
- **The look:** the finisher goes to the hub as the look's `finisher` (hub PROTOCOL.md,
  Looks) and comes back to other players in presence and look events. A hub before
  04/11/2026 refuses the field (`bad_body`): the look goes again without it. A finisher the
  key does not hold (`finisher_not_unlocked`) is left out until the profile changes.
  Clients before ignore it.
- **Getting one:** not droppable yet. The hub grants it by hand (`unlock KEY
  finisher_disintegrate`, the panel or staff); Sol decides when finishers enter holocrons.

## Medal shaders

Built 10/10/2026, branch `personal/medal-blades` (with the hub's `feat/medal-blades`),
Sol's decision: Bug Hunter brings the Glitch blade, Tester the Hologram, Early
Contributor the Runic. The hub derives the unlock from the medal when it reads a profile
or a look (no grant row), lists it in the profile's `unlocks` with `medal` set to the
medal's id while no grant by hand covers it, and takes it back with the medal's last
award, clearing it off a live look with a look event. A grant by hand of the same blade
is separate and outlives the medal. The client ([wire.rs](../crates/sjk-identity/src/wire.rs)
`Unlock::medal`, absent from older hubs) wears it like any unlock (`Holdings::can_wear`
is unchanged); the Collection's Shaders tab says "Yours since dd/mm/yyyy, with your Bug
Hunter medal", its Medals tab adds "Comes with the Glitch blade, a saber shader." under
what the medal is for, and the three blades' "How to get it" names the medal
(`Medal::blade` in [medals.rs](../crates/sjk-viewer/src/medals.rs) pins the pairs, a
test checks the texts). Verified: unit tests (client and hub, the hub's covering award,
counted awards, revoke, a grant by hand surviving, relock and the look); the hub not
deployed when built, nothing tried in the game.

## Unique shaders

Built 25/10/2026 (Sol's decision), branch `personal/unique-tier`: a tier above Mythical,
**Unique** (rose, `#FF6B9A`; `Rarity::Unique` in
[unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)), for the personal thank-you
gifts Sol gives by hand to people who helped SJK. The first is the **Thousand Cranes**
blade (167), the second the **Hearth** (168). The hub names the tier `unique` (its side is
built apart).

- **Given by hand only:** granted at the hub like any grant by hand (the Staff page, the
  operator's command), with the note Sol writes. Never found, dropped, sold, traded or
  crafted: no holocron, kyber crystal, card or medal brings one (`droppable` is false,
  so `economy::in_trade` is too), the economy's tables have no Unique column
  (`shatter_yield` and `craft_cost` give 0) and no shop offers one. Its "How to get it"
  reads "A personal thank-you from Sol." (`unlockables::PERSONAL_THANKS`).
- **Seen only by its holders:** `Unlockable::hidden_unless_held` is true for a Unique
  one, and `Holdings::shows` lets it through only while the player's own profile lists
  it. Everything a player reads of the catalogue goes through it (`Holdings::listed`,
  `Holdings::listed_count`), so to anyone else it is never listed or counted: the
  Collection's Shaders tab (the rack and the grid, the search by name, tier word or
  number, the tier chips, "N of M owned", the Saber pill and the lead's "n of M
  shaders shown"), the Collection's Shaders count and tally, the hover cards, the
  Profile's "N of M shaders owned", the Saber tab's Blade and Sounds rows, and
  `saberskin` (not listed; its id or number answers "No blade skin"). These read each
  frame's holdings, so a gift shows at once and goes when taken back, with its chip and
  a filter on it. Its holder sees it first (the rarest), counted, with a Unique chip
  after Mythical's.
- **Still shown:** the Staff page lists every unlockable, so staff can grant and take it
  back; `debug_collectibles` shows it as held to staff on their own PC; and a player
  wearing one is seen wearing it like any blade (the look travels as usual), their
  player card naming it.
- **Its look:** the rarity mark U+E005 in SJK Symbols (Mythical's two frames round a
  solid four-point star, [sjk-ui.md](sjk-ui.md#symbols)) and Mythical's rarity effects
  in its rose ([Rarity effects](#rarity-effects)).

Verified by unit tests: the tier's name, colour, mark and effects; an unheld Unique blade
neither listed, counted, searched nor chipped on the Shaders tab, held one listed first,
counted and chipped, and its chip gone once taken back; the Saber tab, `saberskin` and
the economy. Not tried in the game; the hub's side is not part of this change.

## Holocron shader drops

Built 14/10/2026 (hub branch `loot-kinds`, merged into the hub's `main`, not deployed).
An unlock is held in a **count** now: a holocron may drop a shader the key already
holds, and the duplicate is another one (since 17/10/2026, prepared: shattered into kyber
shards, see below). A
profile's unlocks carry `count` ([wire.rs](../crates/sjk-identity/src/wire.rs)
`Unlock::count`, 1 from older hubs). Opening a holocron rolls its kind first: a card with
credits as before, or a shader with no credits and no card (`Opened::unlock`,
`Opened::card` is then `None`); the shader's tier is rolled next and a droppable shader of
it picked (the hub's `src/loot.rs` and `PROTOCOL.md` have the provisional odds). The client takes the new answer but has no ceremony for a shader yet:
an opening that held only shaders says "You found Tidal blade (2 held)." in the pop-up's
refusal line, and the Collection lists the shader; the crate-style opening comes next.
Opening is still staff-only. Checked by unit tests on both sides; not in the game.

**What drops** (Sol's rule, 17/10/2026; branch `personal/more-droppable` and hub
`feat/more-droppable`): every saber shader but those a medal or an achievement brings, so
today all but Glitch, Hologram and Runic. That makes 48 of the 51 droppable: Common 10,
Uncommon 13, Rare 6 (3 before), Legendary 8 (4 before) and Mythical 11 (7 before: Sun,
Void, Molten and Spectral join). Each catalogue entry keeps its `droppable` flag (the
client's `Unlockable::droppable`, the hub's `UnlockKind::droppable`), so another shader
could still be kept out; a test on each side pins the rule today (`droppable` exactly when
no medal brings it), and the droppable shaders' "How to get it" reads "Found in holocrons."

Every Blade Lab blade drops too since 20/10/2026 (Sol: no loot is handed out by the team
instead of found). The same day Sol's card, SJK Alpha 000, began to bring the Sun blade
for as long as the player holds a copy ([cards.md](cards.md#faces-and-the-first-set),
`grants`, `unlockables::CARD_BLADES`): for a few hours it was the only way, then Sol made
it a second way besides holocrons. So 293 of the 299 shaders drop (the Varykino Blossom, 166, since 21/10/2026; Creyon's anime round from 170, since 29/10/2026), all but Glitch, Hologram
and Runic (their medals), the Thousand Cranes and the Hearth (Unique, given by hand,
25/10/2026) and the Mystery blade (169, only from mystery boxes); the test pins `droppable` exactly when no medal brings it and it is not Unique.

Duplicates show in the Collection's Shaders tab (14/10/2026): the rack's state line, the
grid card's state and the big card's pill read "Yours x2" (or "Worn x2"), the card's
owned line and the hover card say ", 2 held". The tab's headline still counts shaders
owned out of the catalogue the player sees ("5 of 23 owned"; an unheld Unique shader is
not counted), a duplicate counting once. The tab has a filter
since 14/10/2026 for a catalogue near 200: chips for All and each tier (Unique's only for
a player who holds one; T and Shift+T step through them), Owned (O), the edition (E,
since 06/11/2026, [Alpha and Beta](#alpha-and-beta-the-collections)) and a search field
(/ or Ctrl+F; Enter or Esc leave it, Esc first clears it) that matches a name or a tier's word; the stock blade stays first,
the rack and the grid scroll as before and the lead line says "n of 23 shaders shown"
while anything is filtered. Row numbers are of the list shown, so the choice goes back
to the top when the filter changes.

**Kyber crystals** (21/10/2026, [holocrons.md](holocrons.md#kyber-crystals)): a crystal,
earned once per achievement, always opens to a droppable saber shader of a tier rolled by
the crystal's own odds (every tier with a small Mythical chance), the ones the person does
not hold first, with a pity that makes the tenth crystal in a row without a Legendary or
Mythical shader one of them. Its opening's strip shows only shaders.

## Shattering, crafting and the shop

Live since 21/10/2026 ([cards.md](cards.md#shards-and-the-shop) has the numbers and the
rules): a shader of the trade the player holds can be broken down into kyber shards, one
copy a request (the last copy too on a hub that says `shatter_last`, never a medal's or a
card's), or every duplicate at once; shards craft any shader of the trade the player does
not hold and buy the day's four offers. Only shaders of the *trade* take part: droppable
and brought by no medal (`economy::in_trade`, read from the catalogue's `droppable` and
`Medal::blade`, as the hub reads its own), so Glitch, Hologram, Runic and the Unique ones
([Unique shaders](#unique-shaders)) never do, and a shader made droppable joins by itself.
All of it is the Collection's Forge tab ([cards.md](cards.md#the-forge)); its requests are in
[economy.rs](../crates/sjk-viewer/src/economy.rs):

| Hook | What it gives |
| --- | --- |
| `in_trade(&Unlockable) -> bool` | Whether a shader can be broken down, crafted or sold at all |
| `shatter_yield(Rarity) -> u64`, `craft_cost(Rarity) -> u64` | The hub's tables (its copy until the hub sent them) |
| `may_act(&Snapshot) -> bool` | Whether the hub lets the player trade (as opening) |
| `shatter(id)`, `shatter_last(id)`, `shatter_all()`, `craft(id)`, `buy_holocron(tier)` | Ask the hub; false before the identity started |
| `outcome_words(&TradeOutcome, name: &str) -> String` | "Kamino blade broken down: +5 kyber shards.", or why not |

What the Forge lists counts the profile's `Unlock::count` when it has no `medal` and no
`card`; a shader not listed is not held. The kyber shard's mark to draw beside a count is
`cards::shard::draw` (or the symbol `symbols::SHARD` in text).

**Collection marks** (29/10/2026, [sjk-ui.md](sjk-ui.md#marks)): a player stars their
favourite shaders and locks the ones they want to keep; both are kept at the hub, which
refuses to break down or trade a locked one. `shader_marks::Marks` is what a page shows
(worn, favourite, locked, New), `sjk_identity::Service::set_mark` asks the hub, and the
New marks are kept on the PC (`shader-seen.txt`).

## Planned, not built

Achievements granting unlockables (with harder goals), holocron drops beyond shaders
(toys, body shaders, nameplates, medals), trading duplicates, other kinds of unlockable (holocron
skins, trails, emotes). Up to 256 blade skins load at once (the renderer's slots; the
skin array is a storage buffer of 253952 bytes since 14/10/2026, no longer a 16 KiB uniform,
so the limit is the material slots, not the lanes).

## Editions

14/10/2026 (Sol's request: a season system for every collectible, starting with an
Alpha tag). An **edition** is the stamp of when a collectible came: **Alpha Edition**
for the whole alpha, later ones as they open. [editions.rs](../crates/sjk-viewer/src/editions.rs)
holds the table (`id`, full name, short tag); a new edition is one line there.

- Holocron cards carry theirs per copy: the hub stamps the current edition
  (`cards::EDITION`, "alpha") at opening and the copy keeps it for good
  ([cards.md](cards.md)).
- Shaders (`Unlockable::edition`), medals (`Medal::edition`), achievements
  (`Kind::edition`) and the Illuminate toy take theirs from the client's catalogue
  entries, "alpha" for every one now. An item belongs to one edition; a duplicate keeps
  it with the item.
- Where it reads: the shader's big card (top right of its details), the medal's line
  under its name, and the hover card of each shader, medal, achievement and toy; the
  Shaders search matches an edition's name ("alpha", "beta").

### Alpha and Beta: the collections

06/11/2026 (Sol: "the blades we had before this merge will be the Alpha collection, this
one is the Beta collection"). The editions are the Collection's collections:

- **Alpha Edition**: every blade before Creyon's round (blades 1 to 169), and every
  medal, achievement, banner, tag, finisher and toy so far.
- **Beta Edition**: Creyon's round, PR #370, blades 170 to 299 (`edition:
  crate::editions::BETA` on each; `unlockables.rs` tests that exactly those 130 are
  Beta), and what comes after it.
- The Shaders tab filters by one: the toolbar's **Edition** chip, or E (Shift+E back),
  goes All, Alpha, Beta, All; the lit chip reads the edition shown ("Beta Edition"),
  and the tiers' counts (the strip and each heading's "N of M") follow it.

**Holocrons and kyber crystals carry an edition too**, stamped by the hub when one is
given (a play drop, a staff or operator gift, a purchase, a mystery box, a toy, an
achievement's crystal) and kept for good, through trades too. Opening one brings only
things of its edition or an older one: an Alpha holocron or crystal only Alpha things, a
Beta one Beta or Alpha: the hub picks its shaders, tags, banners and crystal shaders
among those `editions::within(item, holocron)` allows. Cards are not gated yet: every
card face is the SJK Alpha set. The Holocrons tab says each one's: the recent list puts it
before the date
("Beta · 06/11/2026 14:05"), a tier's hover card says the editions of those held ("Alpha
Edition", or "2 Alpha, 1 Beta"), and both loot tables say the rule. The wire field is
`edition` on each holocron ([holocrons.md](holocrons.md)); a hub before 06/11/2026 sends
none, which reads as Alpha. The shop and crafting are not gated by edition.

**The hub's switch.** The hub stamps Beta only while `--beta-min-client VERSION|off`
(`SJK_HUB_BETA_MIN_CLIENT`) names a release, off by default, and only for a person whose
newest client meets that release (SJK development builds pass, as for the other
minimum-client switches): an older client keeps getting Alpha ones, so nobody gets a blade their game
cannot show. Until it is set, every holocron and crystal is Alpha.

**Opening Beta** (when Sol's tests are done):

1. `droppable: true` on the 130 blades in the hub's catalogue and the client's
   `unlockables.rs` together, and both `COMING_TO_HOLOCRONS` constants go (they make
   "Coming to holocrons soon." and keep the blades out of drops, crafting, the shop and
   trades).
2. A client release with it.
3. The hub deployed with it, `SJK_HUB_BETA_MIN_CLIENT` set to that release.

## Saber smiths

14/10/2026 (Sol's request): every shader names its **saber smith**, the person who gave
its idea, even when it was made together (`Unlockable::smith`, the names `SOL` and
`CREYON` in [unlockables.rs](../crates/sjk-viewer/src/unlockables.rs)). Creyon's "Saber
ideas" page gave the 21 plain ones (Tidal to Firefly, Slate to Moonlight) and his
"Blade Lab" page the sixteen after them (Neon to Obsidian Edge, 15/10/2026); Sol gave the
fourteen before them. It reads "Saber smith: Creyon" at the right of the shader's big
card, beside its description, and on its hover card; the Shaders search matches it. A
new shader names its smith in its catalogue entry (a test lists Creyon's).

Card art for an "SJK Blades" card set (20/10/2026): the ignored world shot `blade_card_art`
([world_shot_blade_cards.rs](../crates/sjk-viewer/src/world_shot_blade_cards.rs)) draws
every droppable shader with the game's own saber renderer, lit across a 1020 x 850 frame
on a dark backdrop (the void beyond duel6, every signature shader built first), and writes
`cards/blade<NNN>.png` with its `import_cards.py` text file (number, tier, name, smith,
description) into the folder `SJK_BLADE_CARDS` names; `SJK_BLADE_CARD_SKINS` picks some.
The tier is the client's: check it against the hub's `src/unlocks.rs`, which decides drops.

A shader newer than the client (14/10/2026): when an opening brings a shader this client's
catalogue does not list, the ceremony still shows it, by the name the hub sent, in the
holocron's colour, with its blade if the hub's pack brought one, the crate spin ending on a
"?" tile; it reads "update SJK to see it in your Collection". The Collection lists it once
the client knows it. Since 03/11/2026 any prize the client does not know is shown the same
way, by its kind (Kushina, 30/10/2026: such a shader came out as a dark, empty blade card):
the kind is read from the hub's id (`saber_`, `tag_`, `banner_`, `toy_`; "prize" for
anything else, `opening::unknown_kind`), the strip's tile says "New tag", "New shader"...,
and without a blade from the pack the card shows a "?" in a ring and "Update SJK to see
it" instead of an empty swatch. A card of a rarity or finish the client does not know is
no longer left out: it shows as "A new card".

## Profile covers

[Profile banners](profile-banners.md) share the hub's counted unlock inventory but
have their own catalogue and Collection tab. They are identity equipment, not saber
skins or game-server looks; blade-only trade, craft and shatter actions exclude them.

- **Round 18 and 19** (4/11/2026): a new blade's effects in the air (`@raycache fast`; the blades already in the client draw as before) are drawn into a half-size layer in a view (`saber_volume.rs`, `SIG_SPLIT`, `fragment_volume`), scaled up with a Catmull-Rom filter; a body picture can be sampled with `skinBody`. A generated extra for every blade and generated "great moments" were tried and taken out again (Creyon: they made the blades alike): extras are to be made by hand for each blade (`gx*` in `lab.glsl` holds pieces for that). God Ki's Scythe form is gone. Hard distance switches in the fast rays and early cuts of effects that still shone are gone.
- **Round 20** (5/11/2026): the round lights and orbiting arcs that stood on most new blades are gone; `gxKin` in `lab.glsl` throws off what a blade's world would (grains, flecks, drops, threads, flashing leaf, smoke, bristles, panes, streaks), chosen per blade, with crystals kept for blades of crystal. Anything measured along a curve is continuous now (`bW`), because the nearest piece alone jumped where a curve folds in the view and drew a hard line in the air; mist marches only inside its own height; the effect layer fades by the ray's distance to the blade.
- **Merged** (PR #370, 06/11/2026): God Ki (blade 170 in the pull request) was left out: its body picture and sounds came from Dragon Ball material, and SJK ships only original blades and sounds (Sol). The blades after it moved down one, to 170 to 299. `skinBody` stays, with a plain white picture (`assets/blade_body/plain.png`) that no blade paints with now.
