//! Blade Lab signature shaders: blade skins drawn by their own shader from Creyon's Blade
//! Lab (`docs/rendering.md`, "Blade Lab signature shaders").
//!
//! The lab (a three.js page) draws each blade with SJK's own skin shading ported to GLSL,
//! and lets a blade add a *signature*: `sig_glow`, `sig_core` and `sig_solid`, which take
//! what the skin's file drew and return the blade's own light, core and solid layer. The
//! signatures' GLSL lives in `blade_signatures/` beside its WGSL translation
//! (`scripts/blade_signatures.py`). A skin this table names is drawn by `saber.wgsl` as its
//! file says, each pass then handing its result to the signature (the `LAB_SIGNATURE`
//! design number); its pipelines are built from `saber.wgsl` with the stand-in signature
//! replaced by its own ([`module_source`]), once a blade wears it, off the render thread
//! (`saber_gpu.rs`). Until they are built the blade is drawn from its file alone.
//!
//! A signature skin whose file is in no pack is drawn from the lab's file built in here
//! ([`Signature::default_file`]), so the skins show (in the Collection's preview, and on
//! whoever is given one) before the hub's pack carries them.

use crate::blade_skin_file::BladeSkinDef;
use crate::blade_skin_file::extras::SkinOption;

/// What a signature draws round its blade, for its Collection swatch ([`Signature::swatch_hints`]):
/// each count is how many times it calls that piece (at most 4).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SwatchHints {
    pub(crate) smoke: u8,
    pub(crate) streams: u8,
    pub(crate) flakes: u8,
    pub(crate) sparks: u8,
    pub(crate) fall: u8,
    pub(crate) motes: u8,
    pub(crate) coils: u8,
    pub(crate) orbit: bool,
}

/// One signature shader: the skin it draws and its translated WGSL.
#[derive(Debug)]
pub(crate) struct Signature {
    pub(crate) id: &'static str,
    wgsl: &'static str,
    file: &'static str,
}

macro_rules! signature {
    ($name:literal) => {
        Signature {
            id: concat!("saber_", $name),
            wgsl: include_str!(concat!("blade_signatures/", $name, ".wgsl")),
            file: include_str!(concat!("blade_signatures/", $name, ".bladeskin")),
        }
    };
}

/// The signatures, mythicals first (Creyon's order, 19/10/2026). The last fourteen are his
/// Blade Lab designs reworked in full (19/10/2026: the Star Wars rework; the Bleeding Kyber in
/// round 9), which replace their hand-ported designs (`blade_lab_designs.rs`). Sol's Sun
/// (blade 001) became one on 20/10/2026, keeping its file's look under its own.
pub(crate) const SIGNATURES: [Signature; 263] = [
    signature!("sun"),
    signature!("abyssal_eye"),
    signature!("aurora_wyrm"),
    signature!("between_worlds"),
    signature!("big_bang"),
    signature!("carbonite"),
    signature!("dyson_sphere"),
    signature!("eclipse"),
    signature!("event_horizon"),
    signature!("exogorth"),
    signature!("hyperspace"),
    signature!("jedi_spirits"),
    signature!("kyber_heart"),
    signature!("nebula_song"),
    signature!("quasar"),
    signature!("singing_kyber"),
    signature!("sith_holocron"),
    signature!("star_dragon"),
    signature!("starforge"),
    signature!("superlaser"),
    signature!("uneti_tree"),
    signature!("witchfire"),
    signature!("world_shift"),
    signature!("wraithlight"),
    signature!("lightspeed_ram"),
    signature!("monarch"),
    signature!("obsidian"),
    signature!("quantum"),
    signature!("quicksilver"),
    signature!("reactor"),
    signature!("serpent"),
    signature!("peppermint"),
    signature!("bubble"),
    signature!("lavalamp"),
    signature!("sketch"),
    signature!("hourglass"),
    signature!("fiber"),
    signature!("phoenix"),
    signature!("bleeding"),
    // Round 11 and 12: the Blade Lab's legendaries, rares, uncommons and commons, ported Sol's
    // way and reworked to be Star Wars (tools/port.py in Creyon's blade lab).
    signature!("hoth_avalanche"),
    signature!("blood_moon"),
    signature!("force_echo"),
    signature!("hypermatter"),
    signature!("kessel_run"),
    signature!("unknown_regions"),
    signature!("krayt_pearl"),
    signature!("starkiller_frost"),
    signature!("royal_naboo"),
    signature!("bespin_storm"),
    signature!("scarif_tide"),
    signature!("assassin_droid"),
    signature!("aurodium"),
    signature!("kashyyyk_monsoon"),
    signature!("malachor_forge"),
    signature!("yavin_vines"),
    signature!("exegol_rift"),
    signature!("shattered_kyber"),
    signature!("twin_suns"),
    signature!("balance"),
    signature!("ord_mantell_storm"),
    signature!("mustafar_eruption"),
    signature!("hyperlane"),
    signature!("ahch_to_glow"),
    signature!("imperial_schematics"),
    signature!("jedi_texts"),
    signature!("star_map"),
    signature!("ilum_frost"),
    signature!("christophsis_crystal"),
    signature!("deflector_shield"),
    signature!("healed_kyber"),
    signature!("temple_mosaic"),
    signature!("dagobah_wisps"),
    signature!("nar_shaddaa_rain"),
    signature!("navigation_beacon"),
    signature!("puzzle_holocron"),
    signature!("dac_reef"),
    signature!("jakku_sandstorm"),
    signature!("moisture_farm"),
    signature!("tipoca_stormglass"),
    signature!("temple_sunbeam"),
    signature!("ion_coil"),
    signature!("sith_thorn"),
    signature!("holo_recording"),
    signature!("data_disc"),
    signature!("cargo_stack"),
    signature!("otoh_gunga"),
    signature!("victory_fireworks"),
    signature!("endor_ivy"),
    signature!("naboo_core"),
    signature!("gungan_waters"),
    signature!("cantina_lights"),
    signature!("imperial_searchlight"),
    signature!("flare_stick"),
    signature!("flimsiplast"),
    signature!("canto_bight"),
    signature!("sensor_sweep"),
    signature!("takodana_rain"),
    signature!("starfighter_canopy"),
    signature!("distress_beacon"),
    signature!("orto_plutonia"),
    signature!("rhen_var_frost"),
    signature!("seismic_charge"),
    signature!("thermal_fuse"),
    signature!("senate_glass"),
    signature!("holonet_feed"),
    signature!("holo_map"),
    signature!("cockpit_console"),
    signature!("code_cylinder"),
    signature!("cortosis_weave"),
    signature!("tatooine_dust"),
    signature!("trench_run"),
    signature!("ryloth_dawn"),
    signature!("power_coupling"),
    signature!("kyber_filament"),
    signature!("frosted_viewport"),
    signature!("holo_stylus"),
    signature!("sabacc_card"),
    signature!("geonosis_hive"),
    signature!("mygeeto_frost"),
    signature!("bogano_moss"),
    signature!("kef_bir_sea_glass"),
    signature!("jedha_lantern"),
    signature!("corellian_pearl"),
    signature!("targeting_display"),
    signature!("raxus_scrap"),
    signature!("pasaana_stone"),
    signature!("smoked_transparisteel"),
    signature!("batuu_dusk"),
    signature!("alderaan_velvet"),
    // 21/10/2026: Sol's Varykino Blossom, after Kushina's idea (blade 166).
    signature!("varykino_blossom"),
    // 25/10/2026: Sol's Thousand Cranes (blade 167), the first Unique blade.
    signature!("thousand_cranes"),
    // 25/10/2026: Sol's Hearth (blade 168), a Unique blade that grows as it is held.
    signature!("hearth"),
    // 29/10/2026: Creyon's Star Wars round (from blade 170).
    signature!("vulptex_crystalfire"),
    signature!("zeffo_thunderchorus"),
    signature!("moraband_erasure"),
    signature!("wellspring_triad"),
    signature!("chirrut_whills"),
    signature!("atollon_gathering"),
    signature!("wobani_projection"),
    signature!("iridonia_rite"),
    signature!("rishi_ocean_flow"),
    signature!("scipio_awakening"),
    signature!("nal_hutta_spicevine"),
    signature!("korriban_ragnos"),
    signature!("ossus_scroll"),
    signature!("kiros_echani"),
    signature!("toydaria_gale_streamers"),
    signature!("rattatak_crimson_surge"),
    signature!("hosnian_daybreak"),
    signature!("rakata_infinite"),
    signature!("tatooine_krayt_spine"),
    signature!("lahmu_sandveil"),
    signature!("dromund_kaas_vitiate"),
    signature!("utapau_whirlwind"),
    signature!("umbara_gloaming"),
    signature!("tython_jedaii"),
    signature!("crait_salt_flats"),
    signature!("sullust_magma"),
    signature!("kijimi_snowfall"),
    signature!("saleucami_turbolaser"),
    signature!("wayland_gravity_press"),
    signature!("onderon_blazing_strike"),
    signature!("dantooine_grassland"),
    signature!("bracca_rust_spark"),
    signature!("mandalore_mythosaur"),
    signature!("nevarro_ash"),
    signature!("vandor_whiteout"),
    signature!("bendu_equilibrium"),
    signature!("lehon_maelstrom"),
    signature!("zonama_starlight"),
    signature!("bastila_battle_meditation"),
    signature!("cantonica_ascension"),
    signature!("nightsister_ichor"),
    signature!("nihilus_hunger"),
    signature!("rodia_toxic_bloom"),
    signature!("glee_anselm_rockpools"),
    signature!("polis_massa_asteroid"),
    signature!("lotho_minor_lavaglass"),
    signature!("agamar_copper"),
    signature!("sorgan_lake"),
    signature!("iego_lightglow"),
    signature!("sluis_grey"),
    signature!("arkanis_teal"),
    signature!("peridea_convergence"),
    signature!("byss_force_storm"),
    signature!("lira_san_starglass"),
    signature!("mimban_marshlight"),
    signature!("zygerria_cleave"),
    signature!("jabiim_siege"),
    signature!("pantora_sunrise"),
    signature!("mon_cala_coral"),
    signature!("tund_sorcerers"),
    signature!("haruun_kal_wildheart"),
    signature!("kalevala_darksaber"),
    signature!("ruusan_thoughtbomb"),
    signature!("garel_corona"),
    signature!("trandosha_scorekeeper"),
    signature!("wroshyr_rootwood"),
    signature!("iktotch_foresight"),
    signature!("vardos_phrik"),
    signature!("kuat_shipwright"),
    signature!("kowak_silkweb"),
    signature!("nar_kreeta_lightspeed"),
    signature!("manaan_tidebreaker"),
    signature!("cato_neimoidia_gilded"),
    signature!("dxun_beastfang"),
    signature!("fondor_weldsparks"),
    signature!("plazir_lanterns"),
    signature!("coruscant_neon"),
    signature!("savareen_starfall"),
    signature!("hijarna_colours"),
    signature!("kef_bir_waves"),
    signature!("shili_thornfang"),
    signature!("florrum_brigand"),
    signature!("abafar_pallor"),
    signature!("teth_leafdrift"),
    signature!("malastare_podrace"),
    signature!("anaxes_clonefront"),
    signature!("lola_sayu_watchbeam"),
    signature!("quell_embers"),
    signature!("skako_pressure"),
    signature!("cerea_meadow"),
    signature!("corulag_slate"),
    signature!("bothawui_amber"),
    signature!("hapes_rose"),
    signature!("ithor_verdant"),
    signature!("mirial_saffron"),
    signature!("zeltros_violet"),
    signature!("socorro_ochre"),
    signature!("muunilinst_brass"),
    signature!("dorin_pewter"),
    // 26/10/2026: Creyon's Moon (blade 270, 169 until Sol's Mystery took that number), his own blade, the Sun's night-time sister.
    signature!("moon"),
    signature!("kathol_rift"),
    signature!("fondor_mass_driver"),
    signature!("exar_kun_temple"),
    signature!("naga_sadow_alchemy"),
    signature!("orbalisk_carapace"),
    signature!("ziost_silence"),
    signature!("nathema_deadworld"),
    signature!("amphistaff_serpent"),
    signature!("vergence_scatter"),
    signature!("voxyn_scream"),
    signature!("ysalamir_hush"),
    signature!("rakghoul_plague"),
    signature!("taris_undercity"),
    signature!("telos_restoration"),
    signature!("malevolence_ion_beam"),
    signature!("massassi_altar"),
    signature!("sith_tremor_sword"),
    signature!("eye_of_aldhani"),
    signature!("ferrix_foundry"),
    signature!("kenari_crash"),
    signature!("sarlacc_pit"),
    signature!("cloud_city_dusk"),
    signature!("ring_of_kafrene"),
    signature!("hypergate_drift"),
    signature!("bardotta_calm"),
    signature!("alzoc_ice"),
    signature!("fest_steppe"),
    signature!("ansion_plains"),
    signature!("sernpidal_dust"),
    signature!("angel"),
    // 6/11/2026: the Malachor Rose back, all its own (blade 300).
    signature!("malachor_rose"),
];

/// The lab's helpers every signature may call (`lab.glsl`, translated).
const LAB: &str = include_str!("blade_signatures/lab.wgsl");
/// The stand-in signature in `saber.wgsl` that a signature replaces.
const BEGIN: &str = "// @signature begin\n";
const END: &str = "// @signature end\n";
/// The sections a signature's own parts ride on, part 0 to 5 (`SIG_PART_BITS` in
/// `saber.wgsl`): sections a signature never draws, so their bits are free.
const PART_SECTIONS: [&str; 6] = ["sputter", "glitch", "scan", "bands", "segments", "film"];

/// The section whose bit carries a signature's own part `part` (its `// @part` line, from 0).
pub(crate) fn part_section(part: u8) -> Option<crate::blade_skin_file::extras::Section> {
    crate::blade_skin_file::extras::Section::named(PART_SECTIONS.get(usize::from(part))?)
}

/// An option's id from its name: lower case letters, digits and `_`, at most 24.
fn slug(name: &str) -> String {
    let mut id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    id.truncate(24);
    id
}

/// The design number `saber.wgsl` draws a signature skin under (`LAB_SIGNATURE`).
pub(crate) const NUMBER: u32 = 100;

/// The signature of skin `id`, if it has one.
pub(crate) fn of(id: &str) -> Option<&'static Signature> {
    SIGNATURES.iter().find(|signature| signature.id == id)
}

impl Signature {
    /// A `// @key value` line of its header (`@room`, `@solid`, `@moments`, `@speed`, `@palette`).
    pub(crate) fn meta(&self, key: &str) -> Option<&'static str> {
        self.wgsl
            .lines()
            .take_while(|line| line.starts_with("//"))
            .find_map(|line| {
                line.strip_prefix("// @")?
                    .strip_prefix(key)?
                    .strip_prefix(' ')
                    .map(str::trim)
            })
    }

    /// Every `// @key value` line of its header, the values in order (`@extra`, which
    /// repeats).
    pub(crate) fn header_lines(&self, key: &'static str) -> impl Iterator<Item = &'static str> {
        self.wgsl
            .lines()
            .take_while(|line| line.starts_with("//"))
            .filter_map(move |line| {
                line.strip_prefix("// @")?
                    .strip_prefix(key)?
                    .strip_prefix(' ')
                    .map(str::trim)
            })
    }

    /// The trail every wearer of it gets (`@trail plasma`, `blade_extras.rs`), if not the
    /// blur in its trail colour.
    pub(crate) fn trail(&self) -> Option<crate::blade_extras::TrailKind> {
        serde_json::from_value(serde_json::Value::String(self.meta("trail")?.to_owned())).ok()
    }

    /// Its extras (`@extras {..}` and an `@extra {..}` line each, `blade_extras.rs`), if it
    /// has any; a line that does not parse is left out (a test checks none is).
    pub(crate) fn extras(&self) -> Option<crate::blade_extras::Extras> {
        let gate = serde_json::from_str(self.meta("extras")?).ok()?;
        Some(crate::blade_extras::Extras {
            skin: self.id,
            gate,
            list: self
                .header_lines("extra")
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect(),
        })
    }

    /// The blossoms its swing sheds (`@petals {..}`, `saber_petals.rs`), if it sheds any.
    pub(crate) fn petals(&self) -> Option<crate::saber_petals::PetalSpec> {
        serde_json::from_str(self.meta("petals")?).ok()
    }

    /// How far (units) its quad reaches past the glow for what it draws there (`@room`).
    pub(crate) fn room(&self) -> f32 {
        self.meta("room")
            .and_then(|room| room.parse().ok())
            .unwrap_or(0.0)
    }

    /// Whether its solid layer, once the blade is all out, stands past the tip uncut and closes
    /// itself there (`@solidpast 1`; Angel's cloud bank, whose top was cut flat just past the tip).
    pub(crate) fn solid_past(&self) -> bool {
        self.meta("solidpast") == Some("1")
    }

    /// How far its `@solidnear` solid layer's depth is pulled toward the camera (`@nearpull`),
    /// if not the usual 14 units.
    pub(crate) fn near_pull(&self) -> Option<f32> {
        self.meta("nearpull").and_then(|pull| pull.parse().ok())
    }

    /// Whether it draws a solid layer (`@solid 1`).
    pub(crate) fn solid(&self) -> bool {
        self.meta("solid") == Some("1")
    }

    /// Whether it draws its own lighting and clash (`@moments own`), so a mythical one goes
    /// without the shared awakening, breath and clash ring (`sig_mythic`) that would cover
    /// them (Sol's Sun: its sunrise and its eclipse).
    pub(crate) fn own_moments(&self) -> bool {
        self.meta("moments") == Some("own")
    }

    /// Whether its pieces read the view ray's part of the blade's hiding kept once per pixel
    /// (`lab.glsl` behindBlade, 31/10/2026); `@raycache off` works it out at every piece, as
    /// before, for World Shift, which drew at a third of its speed with it (RTX 5080, Vulkan).
    pub(crate) fn ray_cache(&self) -> bool {
        // The cost test times a GPU with and without the cache in one sitting
        #[cfg(test)]
        if std::env::var_os("SJK_RAYCACHE_OFF").is_some() {
            return false;
        }
        self.meta("raycache").is_none()
    }

    /// How fast its own time runs (`@speed`, 1 when it has none): Creyon's Star Wars round was
    /// written at about a third of the speeds of Sol's blades and ran in slow motion beside them
    /// in game (1/11/2026). Its cycles and moments run at this speed too; lighting, swings and
    /// clashes keep real time.
    pub(crate) fn speed(&self) -> f32 {
        self.meta("speed")
            .and_then(|speed| speed.parse().ok())
            .unwrap_or(1.0)
    }

    /// Its `design` lane in `saber.wgsl`'s `Skin`: the signature's number, no reach of its
    /// own, its room (in the designs' metres) and whether it has a solid layer.
    pub(crate) fn lane(&self) -> [f32; 4] {
        [
            NUMBER as f32,
            0.0,
            self.room() * crate::blade_lab_designs::METRE,
            if self.solid() { 1.0 } else { 0.0 },
        ]
    }

    /// `def` as the signature draws it: none of the file's sections but its afterimages,
    /// heat haze, wisps and embers, which the renderer draws round any blade as round Sol's
    /// (Creyon's round 9: the Starforge lacked the Sun's heat), and the options every
    /// signature has, which
    /// `saber.wgsl` reads from section bits a signature skin has no other use for
    /// (`sig_take`): `effects` (the arcs bit) keeps only the blade itself, `bursts` (the
    /// motes bit) stops swings and clashes setting anything off, `ignition` (the hue bit)
    /// skips the show when it is lit.
    pub(crate) fn shown(&self, def: &BladeSkinDef) -> BladeSkinDef {
        let mut shown = crate::blade_lab_designs::bare(def);
        // Sol's own pieces a signature wears as his blades do (round 11: his lightning and
        // prominences are arcs, his souls and sparks motes).
        shown.arcs = def.arcs.clone();
        shown.motes = def.motes.clone();
        shown.hue = def.hue;
        shown.ghosts = def.ghosts;
        shown.haze = def.haze;
        shown.wisps = def.wisps;
        shown.embers = def.embers;
        shown.echo = def.echo;
        shown.star = def.star;
        shown.orbit = def.orbit;
        shown.veins = def.veins;
        shown.options = self.options(&shown);
        shown
    }

    /// Its parts, each switched on its own in the Collection (Creyon's round 11): the
    /// signature's own (`// @part <name>` lines, in order, on the bits `SIG_PART_BITS` reads
    /// in `saber.wgsl`), then Sol's sections its file has (named by `// @<section> <name>`
    /// lines, or plainly), then its swing and clash bursts, at most
    /// [`MAX_OPTIONS`](crate::blade_skin_file::extras::MAX_OPTIONS).
    fn options(&self, def: &BladeSkinDef) -> Vec<SkinOption> {
        use crate::blade_skin_file::extras::{MAX_OPTIONS, Section};
        let option = |name: &str, section: &str| SkinOption {
            id: slug(name),
            name: name.to_owned(),
            sections: vec![section.to_owned()],
        };
        let mut options: Vec<SkinOption> = self
            .parts()
            .zip(PART_SECTIONS)
            .map(|(name, section)| option(name, section))
            .collect();
        let sections = [
            (Section::Arcs, "Arcs"),
            (Section::Motes, "Motes"),
            (Section::Ghosts, "Afterimages"),
            (Section::Wisps, "Smoke"),
            (Section::Echo, "Echo"),
            (Section::Star, "Glints"),
            (Section::Embers, "Embers"),
            (Section::Veins, "Veins"),
            (Section::Haze, "Heat haze"),
            (Section::Hue, "Colour turn"),
        ];
        for (section, plain) in sections {
            if def.has(section) {
                let name = self.meta(section.name()).unwrap_or(plain);
                // A section named like an option already listed is the same thing to the
                // wearer (#115: Holonet Feed's own flecks and its file's motes are both "Data
                // flecks", Power Coupling's sparks and embers both "Sparks"): one switch for both.
                match options
                    .iter_mut()
                    .find(|listed| listed.name.eq_ignore_ascii_case(name))
                {
                    Some(listed) => listed.sections.push(section.name().to_owned()),
                    None => options.push(option(name, section.name())),
                }
            }
        }
        options.push(option("Swing and clash bursts", "pulse"));
        options.truncate(MAX_OPTIONS);
        // Each on a bit of its own in the wearer's choices (`option_bit` hashes the id):
        // a name that lands on a taken bit gets a number.
        let mut taken = 0u64;
        for option in &mut options {
            let base = option.id.clone();
            let mut n = 2;
            while taken & crate::blade_skin_file::extras::option_bit(&option.id) != 0 {
                option.id = format!("{}_{n}", &base[..base.len().min(21)]);
                n += 1;
            }
            taken |= crate::blade_skin_file::extras::option_bit(&option.id);
        }
        options
    }

    /// The names of its own parts, from its `// @part` lines in order.
    pub(crate) fn parts(&self) -> impl Iterator<Item = &'static str> {
        self.wgsl
            .lines()
            .take_while(|line| line.starts_with("//"))
            .filter_map(|line| line.strip_prefix("// @part "))
            .map(str::trim)
    }

    /// Give `def` (a pack's file for this skin, or its own) the built-in file's sounds and
    /// every section drawn round or over the blade (heat haze, wisps, embers, echo, glints, veins,
    /// afterimages, arcs, motes, the hand-ported designs' figures and solids): the signature's
    /// own, which an older pack file of the skin (from before it was a signature) must not change.
    pub(crate) fn adopt(&self, def: &mut BladeSkinDef) {
        if let Ok(own) = crate::blade_skin_file::parse(self.id, self.file) {
            def.sounds = own.sounds;
            def.haze = own.haze;
            def.wisps = own.wisps;
            def.embers = own.embers;
            def.echo = own.echo;
            def.star = own.star;
            def.orbit = own.orbit;
            def.veins = own.veins;
            def.ghosts = own.ghosts;
            def.arcs = own.arcs;
            def.motes = own.motes;
            def.hue = own.hue;
            // Round 16 (Creyon: Death Star Plans ran through the hilt in third person): an older
            // pack's additions for the skin's hand-ported design (its pencil sketch, drawn from
            // inside the hilt) are not the signature's either; its own file decides them all.
            def.sputter = own.sputter;
            def.glitch = own.glitch;
            def.scan = own.scan;
            def.pulse = own.pulse;
            def.team = own.team;
            def.ambient = own.ambient;
            def.glyphs = own.glyphs;
            def.holes = own.holes;
            def.bands = own.bands;
            def.segments = own.segments;
            def.film = own.film;
            def.blobs = own.blobs;
            def.front = own.front;
            def.sketch = own.sketch;
            def.hourglass = own.hourglass;
            def.serpent = own.serpent;
            def.strands = own.strands;
            def.butterflies = own.butterflies;
            def.wings = own.wings;
            def.copies = own.copies;
            def.ink = own.ink;
            def.chrome = own.chrome;
        }
    }

    /// The lab's blade-skin file for it, for when no pack carries one.
    pub(crate) fn default_file(&self) -> &'static str {
        self.file
    }

    /// What kinds of effects it draws round its blade, read from its shader, for the
    /// Collection's swatch (Creyon's round 12: the swatches showed every signature as a plain
    /// blade): how often it calls each of the lab's effect pieces.
    pub(crate) fn swatch_hints(&self) -> SwatchHints {
        // The shells and `rx` pieces of Creyon's Star Wars round (1/11/2026, lab.glsl) count as the
        // effects they look like: its cards were drawn as plain blades.
        let count = |names: &[&str]| {
            names
                .iter()
                .map(|name| self.wgsl.matches(name).count())
                .sum::<usize>()
                .min(4) as u8
        };
        SwatchHints {
            smoke: count(&[
                "fxSmoke(",
                "shSheath(",
                "shSmokeA(",
                "shFire(",
                "rxMist(",
                "rxMistSpin(",
                "rxSmoke(",
            ]),
            streams: count(&[
                "fxStreams(",
                "shStrands(",
                "shCurtain(",
                "rxRibbon(",
                "rxTongue(",
                "rxHelix(",
                "rxAurora(",
            ]),
            flakes: count(&["fxFlakes(", "shFlakes(", "rxRocks("]),
            sparks: count(&[
                "fxSparks(",
                "shBolts(",
                "shVeins(",
                "shGlints(",
                "rxBolt(",
                "rxSparkShower(",
            ]),
            fall: count(&["fxFall(", "shStreaks("]),
            motes: count(&[
                "fxMotes(",
                "shMotes(",
                "rxStarCloud(",
                "rxBorn(",
                "rxComet(",
            ]),
            coils: count(&["fxCoils("]),
            orbit: self.wgsl.contains("fn sig_orbit(") || self.wgsl.contains("fn sig_orbit_solid("),
        }
    }

    /// The shader its pipelines are built from: `saber.wgsl` with its own signature.
    pub(crate) fn module_source(&self) -> String {
        let mut full = module_source(include_str!("saber.wgsl"), self.wgsl);
        // `@raycache fast`: the round's rays (`lab.glsl`), the hiding only where a piece can show
        if self.meta("raycache") == Some("fast") {
            full = full
                .replace("fn rayPt(", "fn rayPtKept(")
                .replace("fn raySeg(", "fn raySegKept(")
                .replace("fn rayPtFast(", "fn rayPt(")
                .replace("fn raySegFast(", "fn raySeg(")
                .replace("fn xDust(", "fn xDustKept(")
                .replace("fn xBeads(", "fn xBeadsKept(")
                .replace("fn xPuffs(", "fn xPuffsKept(")
                .replace("fn xDustFast(", "fn xDust(")
                .replace("fn xBeadsFast(", "fn xBeads(")
                .replace("fn xPuffsFast(", "fn xPuffs(")
                .replace("fn rxComet(", "fn rxCometKept(")
                .replace("fn rxCometFast(", "fn rxComet(")
                .replace("fn sig_vol(", "fn sig_vol_near(")
                .replace("fn sig_vol_solid(", "fn sig_vol_solid_near(");
            // Right against the eye a blade's air effects would cost the most and show the least
            // (Creyon's videos, 125 frames down to 46 at 3440x1440): they fade out and stop.
            full.push_str(
                "\nfn sig_vol(ro: vec3<f32>, rd: vec3<f32>, px: f32) -> vec3<f32> {\n    \
                 let e = length(vec3(ro.x, ro.y - clamp(ro.y, 0.0, uLength), ro.z));\n    if e < 3.5 {\n        return vec3(0.0);\n    }\n    \
                 return sig_vol_near(ro, rd, px) * smoothstep(3.5, 7.5, e);\n}\n\
                 fn sig_vol_solid(ro: vec3<f32>, rd: vec3<f32>, px: f32) -> vec4<f32> {\n    \
                 let e = length(vec3(ro.x, ro.y - clamp(ro.y, 0.0, uLength), ro.z));\n    if e < 3.5 {\n        return vec4(0.0);\n    }\n    \
                 let o = sig_vol_solid_near(ro, rd, px);\n    \
                 return vec4(o.rgb, o.a * smoothstep(3.5, 7.5, e));\n}\n",
            );
        }
        // The round's blades (`@speed`) always draw their built-in file: the stock features it has
        // no key for cannot run, so they need not be built (update 22, Creyon: load times).
        let mut source = if self.meta("speed").is_some() {
            slim(&lean(&full, self.default_file()))
        } else {
            slim(&full)
        };
        if !self.ray_cache() {
            // Every piece calls the hiding worked out whole, as before (`lab.glsl`); the kept
            // one is left unused.
            source = source
                .replace("fn behindBlade(", "fn behindBladeCached(")
                .replace("behindBladeFull(", "behindBlade(");
        }
        if let Some(pull) = self.near_pull() {
            source = source.replacen(
                "const SIG_NEAR_PULL: f32 = 14.0;",
                &format!("const SIG_NEAR_PULL: f32 = {pull:?};"),
                1,
            );
        }
        if self.solid_past() {
            source = source.replacen(
                "const SIG_SOLID_PAST_TIP: bool = false;",
                "const SIG_SOLID_PAST_TIP: bool = true;",
                1,
            );
        }
        if self.meta("speed").is_some() {
            source = source.replacen(
                "const SIG_SPEED: f32 = 1.0;",
                &format!("const SIG_SPEED: f32 = {:?};", self.speed()),
                1,
            );
            source = source.replacen(
                "const SIG_OWN_EFFECTS: bool = false;",
                "const SIG_OWN_EFFECTS: bool = true;",
                1,
            );
        }
        // A mythical blade's awakening, breath and clash ring (round 19, `sig_mythic`), and a
        // Unique one's (25/10/2026, Sol: Mythical's level of effects), unless it draws its own
        // (`@moments own`).
        if !self.own_moments()
            && crate::unlockables::find(self.id)
                .is_some_and(|unlockable| unlockable.tier >= crate::unlockables::Rarity::Mythical)
        {
            source.replacen(
                "const SIG_MYTHICAL: bool = false;",
                "const SIG_MYTHICAL: bool = true;",
                1,
            )
        } else {
            source
        }
    }
}

/// The entry points a signature's own pipelines use (`saber_gpu.rs` `SignatureBuilder::build`).
const SIGNATURE_ENTRIES: [&str; 8] = [
    "vertex_main",
    "vertex_volume",
    "fragment_main",
    "fragment_main_deep",
    "fragment_volume",
    "vertex_solid",
    "fragment_solid",
    "fragment_solid_near",
];

/// A signature's module cut to what its pipelines run (Creyon's round 20: the blades' effects took
/// minutes to come in at launch): a signature skin is never a hand-ported design, so the designs'
/// glow and solid pass draw nothing in it, and every function its entry points do not reach is
/// left out, with the entry points it has no pipelines for. The driver then compiles far less.
/// The stock features a blade's file can switch on, as (file key, its functions): with no key in
/// the file, each function's body becomes a return of nothing, and what only it called goes too.
const LEAN_FEATURES: [(&str, &str); 12] = [
    ("serpent", "skin_serpent"),
    ("butterflies", "skin_butterflies"),
    ("strands", "skin_strands"),
    ("wings", "skin_wings"),
    ("blobs", "skin_blobs"),
    ("sketch", "skin_sketch"),
    ("glyphs", "skin_glyphs"),
    ("embers", "skin_embers"),
    ("arcs", "skin_arcs"),
    ("motes", "skin_motes"),
    ("veins", "skin_veins"),
    ("star", "skin_star"),
];

fn lean(source: &str, file: &str) -> String {
    let mut source = source.to_owned();
    for (key, name) in LEAN_FEATURES {
        if file.contains(&format!("\"{key}\"")) {
            continue;
        }
        let head = format!("fn {name}(");
        let Some(at) = source.find(&head) else {
            continue;
        };
        let Some(open) = source[at..].find('{').map(|n| at + n) else {
            continue;
        };
        if !source[at..open].contains("-> f32") {
            continue;
        }
        if let Some(end) = block_end(&source, open) {
            source.replace_range(open..end, "{\n    return 0.0;\n}");
        }
    }
    source
}

fn slim(source: &str) -> String {
    let mut source = source.to_owned();
    for (head, empty) in [
        (
            "fn lab_glow(input: VertexOutput, pixel: f32) -> vec3<f32> {",
            "return vec3(0.0);",
        ),
        (
            "fn lab_solid(input: VertexOutput, pixel: f32) -> vec4<f32> {",
            "return vec4(0.0);",
        ),
    ] {
        if let Some(at) = source.find(head)
            && let Some(end) = block_end(&source, at + head.len() - 1)
        {
            source.replace_range(at..end, &format!("{head}\n    {empty}\n}}"));
        }
    }
    let functions = top_functions(&source);
    let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    let mut reached = vec![false; functions.len()];
    let mut todo: Vec<usize> = (0..functions.len())
        .filter(|&i| SIGNATURE_ENTRIES.contains(&names[i]))
        .collect();
    for &i in &todo {
        reached[i] = true;
    }
    while let Some(i) = todo.pop() {
        let body = &source[functions[i].body.clone()];
        for word in words(body) {
            if let Some(j) = names.iter().position(|name| *name == word)
                && !reached[j]
            {
                reached[j] = true;
                todo.push(j);
            }
        }
    }
    let mut slim = String::with_capacity(source.len());
    let mut at = 0;
    for (function, kept) in functions.iter().zip(&reached) {
        if !kept {
            slim.push_str(&source[at..function.whole.start]);
            at = function.whole.end;
        }
    }
    slim.push_str(&source[at..]);
    slim
}

/// A top-level function of a module: its name, its whole text (with its attribute line) and its
/// body.
struct Function {
    name: String,
    whole: std::ops::Range<usize>,
    body: std::ops::Range<usize>,
}

/// The module's top-level functions, in order.
fn top_functions(source: &str) -> Vec<Function> {
    let mut functions = Vec::new();
    let mut line_start = 0;
    while line_start < source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |n| line_start + n);
        let line = &source[line_start..line_end];
        if let Some(rest) = line.strip_prefix("fn ")
            && let Some(name) = rest.split('(').next()
            && let Some(open) = source[line_start..].find('{').map(|n| line_start + n)
            && let Some(end) = block_end(source, open)
        {
            // Its attribute (`@vertex`, `@fragment`) on the line before goes with it.
            let mut start = line_start;
            let before = source[..line_start.saturating_sub(1)]
                .rfind('\n')
                .map_or(0, |n| n + 1);
            if line_start > 0 && source[before..line_start].trim_start().starts_with('@') {
                start = before;
            }
            let whole_end = if source[end..].starts_with('\n') {
                end + 1
            } else {
                end
            };
            functions.push(Function {
                name: name.trim().to_owned(),
                whole: start..whole_end,
                body: open..end,
            });
            line_start = whole_end;
            continue;
        }
        line_start = line_end + 1;
    }
    functions
}

/// Just past the brace closing the block opened at `open`, skipping line comments.
fn block_end(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The identifiers in `text`.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
}

/// `saber` with its stand-in signature replaced by the lab's helpers and `signature`.
fn module_source(saber: &str, signature: &str) -> String {
    let begin = saber.find(BEGIN).expect("saber.wgsl marks its signature");
    let end = saber[begin..]
        .find(END)
        .expect("saber.wgsl ends its signature")
        + begin;
    let mut source = String::with_capacity(saber.len() + LAB.len() + signature.len() + 2);
    source.push_str(&saber[..begin + BEGIN.len()]);
    source.push_str(LAB);
    source.push('\n');
    source.push_str(signature);
    // A signature without one of the three keeps the stand-in's.
    let stand_in = &saber[begin + BEGIN.len()..end];
    for function in stand_in.split("\n\n") {
        let name = function
            .trim_start()
            .strip_prefix("fn ")
            .and_then(|rest| rest.split('(').next());
        if let Some(name) = name
            && !signature.contains(&format!("fn {name}("))
            && !LAB.contains(&format!("fn {name}("))
        {
            source.push('\n');
            source.push_str(function.trim_end());
            source.push('\n');
        }
    }
    source.push_str(&saber[end..]);
    source
}

/// Every signature's module, validated once (for the tests and the log).
#[cfg(test)]
fn modules() -> &'static Vec<(&'static str, String)> {
    use std::sync::OnceLock;
    static MODULES: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    MODULES.get_or_init(|| {
        SIGNATURES
            .iter()
            .map(|signature| (signature.id, signature.module_source()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::naga;

    /// 1/11/2026: `@speed` reaches the module's clock; a blade without it keeps real time.
    #[test]
    fn speed_sets_the_clock_and_sols_blades_keep_real_time() {
        let lehon = of("saber_lehon_maelstrom").expect("Lehon");
        assert_eq!(lehon.speed(), 4.5);
        assert!(
            lehon
                .module_source()
                .contains("const SIG_SPEED: f32 = 4.5;")
        );
        let sun = of("saber_sun").expect("the Sun");
        assert_eq!(sun.speed(), 1.0);
        assert!(sun.module_source().contains("const SIG_SPEED: f32 = 1.0;"));
        for signature in &SIGNATURES {
            let speed = signature.speed();
            assert!(speed.is_finite() && speed > 0.0, "{}", signature.id);
        }
    }

    /// 31/10/2026: every signature reads the ray's part of its hiding kept once per pixel but
    /// World Shift (`@raycache off`), whose module works it out at every piece.
    #[test]
    fn world_shift_moon_and_vulptex_go_without_the_ray_cache() {
        for signature in &SIGNATURES {
            let source = signature.module_source();
            // (1/11/2026, update 15: the Moon and Vulptex Spiritfire too; Creyon's RTX lost half its frames
            // on them though they cost less than Abyssal Eye on llvmpipe, World Shift's pattern)
            // (4/11/2026: and the round's blades, `@raycache fast`, after the same on his RTX)
            let off = signature.meta("raycache").is_some();
            assert_eq!(
                matches!(
                    signature.id,
                    "saber_world_shift" | "saber_moon" | "saber_vulptex_crystalfire"
                ) || signature.meta("speed").is_some() && off,
                off,
                "{}",
                signature.id
            );
            assert_eq!(signature.ray_cache(), !off, "{}", signature.id);
            // off: the kept one is defined but nothing calls it
            assert_eq!(
                source.matches("behindBladeCached(").count(),
                usize::from(off),
                "{}",
                signature.id
            );
            assert_eq!(
                source.contains("behindBladeFull("),
                !off,
                "{}",
                signature.id
            );
            assert!(
                source.contains("labRayPrepare(ray.o, ray.d)"),
                "{}",
                signature.id
            );
        }
    }

    /// Round 20: a signature's module keeps its pipelines' entry points and drops the designs'
    /// code and the entry points it has no pipelines for.
    #[test]
    fn a_signature_module_is_slim() {
        let source = of("saber_reactor").unwrap().module_source();
        for entry in SIGNATURE_ENTRIES {
            assert!(source.contains(&format!("fn {entry}(")), "{entry}");
        }
        for gone in [
            "fn fragment_haze(",
            "fn lab_comic_word(",
            "fn lab_quicksilver_body(",
        ] {
            assert!(!source.contains(gone), "{gone}");
        }
        assert!(source.len() < include_str!("saber.wgsl").len());
    }

    /// Round 16: the hub pack's additions for Death Star Plans (its hand-ported design's pencil
    /// sketch, drawn from inside the hilt) do not reach the signature.
    #[test]
    fn a_pack_s_older_figures_do_not_reach_a_signature() {
        let signature = of("saber_sketch").unwrap();
        let additions = r#"{"sketch": {"color": [0.93, 0.95, 1.0], "brightness": 2.2, "rate": 8.0, "wobble": 0.3, "hatch": 1.2, "glow": 0.25}}"#;
        let text =
            crate::blade_skin_file::extras::merged(signature.default_file(), additions).unwrap();
        let mut def = crate::blade_skin_file::parse(signature.id, &text).unwrap();
        assert!(def.sketch.is_some(), "the pack's sketch reads");
        signature.adopt(&mut def);
        assert!(def.sketch.is_none());
    }

    #[test]
    fn every_signature_is_a_blade_skin_of_the_catalogue_once() {
        for (index, signature) in SIGNATURES.iter().enumerate() {
            assert!(
                crate::unlockables::blade_skin(signature.id).is_some(),
                "{} is not a blade skin",
                signature.id
            );
            assert!(
                SIGNATURES[..index]
                    .iter()
                    .all(|other| other.id != signature.id),
                "{} twice",
                signature.id
            );
        }
    }

    #[test]
    fn every_signature_module_validates() {
        for (id, source) in modules() {
            let module = naga::front::wgsl::parse_str(source)
                .unwrap_or_else(|error| panic!("{id}: {}", error.emit_to_string(source)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::default(),
            )
            .validate(&module)
            .unwrap_or_else(|error| panic!("{id}: {}", error.emit_to_string(source)));
        }
    }

    /// The Moon blade (Creyon's, 26/10/2026): its five own parts, Sol's mist from its file on
    /// its "Moon glitter" switch (no motes, Creyon found them cheap; no afterimages, its own
    /// effects took their place), and the bursts: six options, none dropped.
    #[test]
    fn the_moon_blade_offers_its_parts_and_sols_pieces_on_one_switch() {
        let moon = of("saber_moon").unwrap();
        let def = crate::blade_skin_file::parse(moon.id, moon.default_file()).unwrap();
        let options = moon.options(&def);
        let names: Vec<_> = options.iter().map(|option| option.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Moonlight",
                "Moon glitter",
                "Silver tide",
                "Falling stars",
                "Night mist",
                "Lunar eclipse"
            ]
        );
        let glitter = &options[1].sections;
        assert!(glitter.iter().any(|s| s == "wisps"), "{glitter:?}");
        for gone in ["motes", "ghosts", "arcs"] {
            assert!(!glitter.iter().any(|s| s == gone), "{glitter:?}");
        }
        assert_eq!(moon.petals().map(|p| p.part), Some(1));
    }

    #[test]
    fn every_petals_line_parses_and_names_one_of_its_parts() {
        for signature in &SIGNATURES {
            if signature.meta("petals").is_none() {
                continue;
            }
            let petals = signature
                .petals()
                .unwrap_or_else(|| panic!("{}: @petals does not parse", signature.id));
            assert!(petals.valid(), "{}: {petals:?}", signature.id);
            assert!(
                usize::from(petals.part) < signature.parts().count(),
                "{}: @petals names part {} of {}",
                signature.id,
                petals.part,
                signature.parts().count()
            );
        }
        let blossom = of("saber_varykino_blossom").unwrap();
        assert_eq!(blossom.petals().map(|p| p.part), Some(3));
        assert_eq!(
            part_section(3),
            Some(crate::blade_skin_file::extras::Section::Bands)
        );
        // Its fourth part, "Falling blossoms", is the one its wearer switches them with.
        assert_eq!(blossom.parts().nth(3), Some("Falling blossoms"));
    }

    #[test]
    fn every_signature_has_its_room_and_its_own_three_passes() {
        for signature in &SIGNATURES {
            let room = signature.room();
            assert!(
                (0.0..=10.0).contains(&room),
                "{}: room {room}",
                signature.id
            );
            assert!(signature.wgsl.contains("fn sig_glow("), "{}", signature.id);
            assert_eq!(
                signature.solid(),
                signature.wgsl.contains("fn sig_solid(")
                    || signature.wgsl.contains("fn sig_vol_solid(")
                    || signature.wgsl.contains("fn sig_orbit_solid("),
                "{}: @solid and sig_solid disagree",
                signature.id
            );
        }
        let star_dragon = of("saber_star_dragon").unwrap();
        assert_eq!(star_dragon.room(), 10.0);
        assert_eq!(star_dragon.lane()[0], NUMBER as f32);
        assert!(
            include_str!("saber.wgsl").contains(&format!("const LAB_SIGNATURE: u32 = {NUMBER}u;"))
        );
    }

    #[test]
    fn a_module_keeps_the_stand_ins_the_signature_has_not() {
        let saber = include_str!("saber.wgsl");
        let source = module_source(
            saber,
            "fn sig_glow(g: vec3<f32>, b: vec2<f32>, r: f32, px: f32) -> vec3<f32> {\n    return g * 2.0;\n}\n",
        );
        assert_eq!(source.matches("fn sig_glow(").count(), 1);
        assert_eq!(source.matches("fn sig_core(").count(), 1);
        assert_eq!(source.matches("fn sig_solid(").count(), 1);
        assert!(source.contains("return g * 2.0;"));
        assert!(source.contains("fn h11("));
    }

    #[test]
    fn every_default_file_reads_as_a_blade_skin() {
        for signature in &SIGNATURES {
            crate::blade_skin_file::parse(signature.id, signature.default_file())
                .unwrap_or_else(|why| panic!("{}: {why}", signature.id));
        }
    }

    #[test]
    fn every_signature_offers_its_options_on_their_own_bits() {
        use crate::blade_skin_file::extras::{Section, option_bit};
        let signature = of("saber_wraithlight").unwrap();
        let def = crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
        let shown = signature.shown(&def);
        let ids: Vec<&str> = shown
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        // Its own part first, then Sol's sections its file has, then the bursts.
        assert_eq!(
            ids,
            [
                "mist_and_echoes",
                "souls",
                "afterimages",
                "spirit_mist",
                "echo",
                "swing_and_clash_bursts"
            ]
        );
        let bits: Vec<u64> = ids.iter().map(|id| option_bit(id)).collect();
        for (index, bit) in bits.iter().enumerate() {
            assert!(!bits[..index].contains(bit), "{} shares a bit", ids[index]);
        }
        // All off: the sections the shader reads them from.
        let all = bits.iter().fold(0, |all, bit| all | bit);
        let mask = shown.off_mask(all);
        for section in [
            Section::Sputter,
            Section::Motes,
            Section::Ghosts,
            Section::Wisps,
            Section::Echo,
            Section::Pulse,
        ] {
            assert_ne!(mask & section.bit(), 0, "{section:?}");
        }
        // Its own part 0 rides on the first part bit (`SIG_PART_BITS[0]` in saber.wgsl).
        assert_eq!(
            shown.off_mask(option_bit("mist_and_echoes")),
            Section::Sputter.bit()
        );
        // Every signature's options sit on bits of their own.
        for signature in &SIGNATURES {
            let def =
                crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
            let bits: Vec<u64> = signature
                .shown(&def)
                .options
                .iter()
                .map(|option| option_bit(&option.id))
                .collect();
            for (index, bit) in bits.iter().enumerate() {
                assert!(
                    !bits[..index].contains(bit),
                    "{}: options share a bit",
                    signature.id
                );
            }
        }
    }

    /// #115 (Rollembones): Holonet Feed listed "Data flecks" twice, its own part and its
    /// file's motes under the same name. No signature lists a name twice; a part and a
    /// section of one name are one switch for both.
    #[test]
    fn no_signature_lists_an_option_name_twice() {
        use crate::blade_skin_file::extras::{Section, option_bit};
        for signature in &SIGNATURES {
            let def =
                crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
            let names: Vec<String> = signature
                .shown(&def)
                .options
                .iter()
                .map(|option| option.name.to_lowercase())
                .collect();
            for (index, name) in names.iter().enumerate() {
                assert!(
                    !names[..index].contains(name),
                    "{}: {name:?} twice",
                    signature.id
                );
            }
        }
        let signature = of("saber_holonet_feed").unwrap();
        let def = crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
        let shown = signature.shown(&def);
        assert_eq!(
            shown.off_mask(option_bit("data_flecks")),
            Section::Glitch.bit() | Section::Motes.bit()
        );
    }

    /// The Sun's parts keep the ids they had before it was a signature (11/10/2026), so a
    /// wearer's choices stay.
    /// A mythical gets the shared awakening, breath and clash ring unless it draws its
    /// own moments: the Sun's sunrise and eclipse stay alone.
    #[test]
    fn a_mythical_with_its_own_moments_goes_without_the_shared_ones() {
        let on = "const SIG_MYTHICAL: bool = true;";
        let sun = of("saber_sun").unwrap();
        assert!(sun.own_moments());
        assert!(!sun.module_source().contains(on));
        let eye = of("saber_abyssal_eye").unwrap();
        assert!(!eye.own_moments());
        assert!(eye.module_source().contains(on));
    }

    #[test]
    fn the_sun_keeps_its_part_ids() {
        let signature = of("saber_sun").unwrap();
        let def = crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
        let ids: Vec<String> = signature
            .shown(&def)
            .options
            .into_iter()
            .map(|option| option.id)
            .collect();
        assert_eq!(
            ids,
            [
                "sunspots_and_corona",
                "prominences",
                "sparks",
                "glint",
                "haze",
                "swing_and_clash_bursts"
            ]
        );
    }

    /// The WGSL is the script's: none edited by hand, none left behind.
    #[test]
    fn every_signature_has_its_lab_source() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/blade_signatures");
        for signature in &SIGNATURES {
            let name = signature.id.trim_start_matches("saber_");
            let glsl = std::fs::read_to_string(folder.join(format!("{name}.glsl")))
                .unwrap_or_else(|why| panic!("{name}.glsl: {why}"));
            assert!(glsl.contains("sig_glow("), "{name}.glsl");
            assert!(signature.wgsl.starts_with(&format!(
                "// Generated by scripts/blade_signatures.py from {name}.glsl"
            )));
        }
        let wgsl = std::fs::read_dir(&folder)
            .unwrap()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(".wgsl") && name != "lab.wgsl")
            .count();
        assert_eq!(wgsl, SIGNATURES.len(), "a signature not in the table");
    }
}

#[cfg(test)]
#[path = "blade_signatures_gpu_test.rs"]
mod gpu_test;

#[cfg(test)]
#[path = "blade_views_gpu_test.rs"]
mod views_test;
