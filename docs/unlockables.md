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
Kyber and sixteen rares, uncommons and commons); a signature's own effects keep their colours
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
fireworks keep their colours) chromas.

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

`disintegrate` (19/10/2026, the eighth set): the wearer's saber kills (a thrown saber's
too, `MOD_SABER`) burn away as a disruptor's do, `delay` seconds after the blow (optional,
0-3, default 0). The server knows nothing of it: every SJK client that sees the killer
wear the skin starts it from the kill feed (`GpuState::observe_saber_kills`,
`disintegration::Forced`), drawing the frozen pose, burn and smoke the disruptor's take
(`track_disintegration`), hides the victim after 1.5 s until they live again, and hides
the body the server copies of them as they respawn (`corpse_actors.rs`, its `ircg` body
command, which comes before the body is drawn; matching a body by where the player lay
took an older body of theirs lying near instead, 19/10/2026). The server and other clients keep the corpse. Only a killing blow does it.

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
it a second way besides holocrons. So 163 of the 168 shaders drop (the Varykino Blossom, 166, since 21/10/2026), all but Glitch, Hologram
and Runic (their medals) and the Thousand Cranes and the Hearth (Unique, given by hand,
25/10/2026); the test pins `droppable` exactly when no medal brings it and it is not Unique.

Duplicates show in the Collection's Shaders tab (14/10/2026): the rack's state line, the
grid card's state and the big card's pill read "Yours x2" (or "Worn x2"), the card's
owned line and the hover card say ", 2 held". The tab's headline still counts shaders
owned out of the catalogue the player sees ("5 of 23 owned"; an unheld Unique shader is
not counted), a duplicate counting once. The tab has a filter
since 14/10/2026 for a catalogue near 200: chips for All and each tier (Unique's only for
a player who holds one; T and Shift+T step through them), Owned (O) and a search field
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
  Shaders search matches an edition's name ("alpha"). Nothing drops by edition yet: later
  an edition can end its drops or limit its items, which will be the hub's table too.

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
