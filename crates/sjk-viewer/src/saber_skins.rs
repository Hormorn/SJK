//! Blade skins: unlockable looks that replace a saber blade's colour (not its hilt).
//! Their ids, names and descriptions are the catalogue's ([`crate::unlockables`]); their
//! looks and sounds are data, blade-skin files ([`crate::blade_skin_file`]) in the packs
//! the SJK hub delivers ([`crate::sjk_packs`], `docs/unlockables.md`). The renderer is
//! generic: nothing here or in `saber.wgsl` knows any one skin.
//!
//! The loaded skins ([`LoadedSkins`], at most [`MAX_SKINS`]) are numbered in load order.
//! A skin is drawn as the saber material of its number after the retail pairs and the
//! neutral RGB pair ([`BladeColor::Skin`]), with its glow/core pair (the pack's images or
//! generated from the file's profiles) and its parameters in the skins' uniform array,
//! animated by time in `saber.wgsl`. It has its own trail and light colours and its own
//! sounds, named by the file.
//!
//! Who wears which skin is the per-client [`SaberSkins`] table on [`GpuState`]. Saber
//! submission (in the hand, thrown, first person) asks it for each entity's blades, and
//! the audio adapter gets its sound overrides from it every frame
//! ([`GpuState::sync_saber_skins`]). The local player's entry is its own skin, gated by
//! its hub profile ([`GpuState::local_saber_skin`]), in its own slot (the game state's,
//! also while it follows someone); every other client's is the look the hub relayed
//! for its slot (`GpuState::looks`), taken again only when it or the loaded skins
//! change. A skin whose pack is not loaded is the player's stock blade.

use crate::blade_skin_file::BladeSkinDef;
use crate::saber_rgb::BladeColor;
use crate::{GameAudio, GpuState};
use bytemuck::{Pod, Zeroable};
use sjk_vfs::VirtualFileSystem;
use std::sync::Arc;

/// Players a game server can hold (`MAX_CLIENTS`); entity ids 1..=32 are their bodies.
pub(crate) const MAX_CLIENTS: usize = sjk_client::SABER_SOUND_CLIENTS;
/// Most blade skins loaded at once: material slots and the storage array's length
/// (`MAX_SKINS` in `saber.wgsl`). 512 skins are 496 KiB, far inside a storage binding.
pub(crate) const MAX_SKINS: usize = 512;
/// A loaded skin's number (Creyon's Star Wars round, 1/11/2026: past 256 skins, so no longer a
/// byte).
pub(crate) type SkinIndex = u16;
/// Widest a skin's glow may reach past the stock capsule (`corona.reach`); the
/// instances' bounds allow for it ([`crate::saber::Instance::extent`]).
pub(crate) const MAX_GLOW_REACH: f32 = 2.0;
/// Largest glow or core image a pack may give, per side.
const IMAGE_MAX: u32 = 1_024;

/// A loaded skin's light flicker ([`crate::blade_skin_file::Flicker`]), copied for the
/// per-frame light.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LightFlicker {
    pub(crate) amount: f32,
    /// Rate, weight and phase of each wave; unused waves weigh nothing.
    pub(crate) waves: [[f32; 3]; 2],
    /// The skin's heartbeat (`pulse`), which the light beats with; none when absent.
    pub(crate) pulse: Option<Beat>,
}

/// A heartbeat ([`crate::blade_skin_file::effects::Pulse`]): the CPU's copy of
/// `saber.wgsl`'s `skin_beat`, which beats on the presentation clock alone (no seed), so
/// a blade and its light beat together.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Beat {
    pub(crate) rate: f32,
    pub(crate) amount: f32,
    pub(crate) second: f32,
    pub(crate) gap: f32,
    pub(crate) width: f32,
}

impl Beat {
    /// Where the first beat peaks in a cycle, seconds (`BEAT_FIRST` in `saber.wgsl`).
    pub(crate) const FIRST: f32 = 0.1;

    /// How much the beat brightens at `seconds`: `amount × (first + second × next)`, each
    /// a Gaussian `width` seconds wide.
    pub(crate) fn at(self, seconds: f32) -> f32 {
        let into = (seconds * self.rate).rem_euclid(1.0) / self.rate;
        let bump = |at: f32| {
            let d = (into - at) / self.width;
            (-d * d).exp()
        };
        self.amount * (bump(Self::FIRST) + self.second * bump(Self::FIRST + self.gap))
    }
}

impl LightFlicker {
    /// The light's brightness at `time_millis` for a hilt's `seed` (so blades of one
    /// hilt do not pulse together with another's): `1 − amount + amount × waves`.
    pub(crate) fn at(self, time_millis: i64, seed: u64) -> f32 {
        if self.amount == 0.0 && self.pulse.is_none() {
            return 1.0;
        }
        let t = (time_millis.rem_euclid(1_024_000)) as f32 * 0.001;
        let phase = (seed % 1_000) as f32 * 0.618;
        let wave = self
            .waves
            .iter()
            .map(|[rate, weight, offset]| (t * rate + phase * offset).sin() * weight)
            .sum::<f32>();
        let beat = self.pulse.map_or(0.0, |beat| beat.at(t));
        ((1.0 - self.amount) + self.amount * wave) * (1.0 + beat)
    }
}

/// What a blade wearing a loaded skin is drawn with: the skin's number (its material
/// and its parameters in `saber.wgsl`) and its trail, light and flicker. Small and
/// `Copy`, so blades carry it without the skin table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkinColor {
    /// The skin's number among the loaded skins, below [`MAX_SKINS`].
    pub(crate) index: SkinIndex,
    /// The blur trail's vertex colour.
    pub(crate) trail: [f32; 3],
    /// The dynamic light's colour (before `saber_lights`' gain).
    pub(crate) light: [f32; 3],
    pub(crate) flicker: LightFlicker,
    /// The skin's hue turning (`hue` in its file), for the light: turns a second, and the
    /// turn at the middle of a stock 40-unit blade. Zeros hold the light's colour.
    pub(crate) hue: [f32; 2],
    /// The skin's afterimages (`ghosts` in its file); none when absent.
    pub(crate) ghosts: Option<crate::saber_ghosts::GhostSpec>,
    /// The blossoms its swing sheds (its signature's `@petals`, or an extra's sheds,
    /// `blade_extras.rs`); none when absent or off.
    pub(crate) petals: Option<crate::saber_petals::PetalSpec>,
    /// Who wears it (team, name), for the skins drawn from that; the default with no
    /// wearer known.
    pub(crate) persona: crate::saber_persona::Persona,
    /// A chroma's base hue and the turn to its wearer's colour ([`Chroma`]); the default
    /// for the other skins.
    pub(crate) chroma: Chroma,
    /// The sections its wearer switched off ([`crate::blade_skin_file::extras::Section`]
    /// bits; `saber.wgsl` draws them as absent). The CPU's parts (afterimages, echo,
    /// glint, the light's beat and hue, and the emitter's smoke, ribbons, sparks, bolts,
    /// stars, crackle and burning) are already left out of this colour.
    pub(crate) off: u64,
    /// How far (units) the glow's quad reaches past the glow, for wisps and haze.
    pub(crate) room: f32,
    /// The glint at the tip, or the glints circling the blade below it, or the black
    /// holes in their places; none when absent or off.
    pub(crate) star: Option<Glint>,
    /// The echo beside the blade; none when absent or off.
    pub(crate) echo: Option<crate::blade_skin_file::extras::Echo>,
    /// Seconds its words float on after its blade has gone in
    /// ([`crate::blade_lab_designs::linger`]); 0 for most.
    pub(crate) linger: f32,
    /// Whether it answers each swing of a chain on its own
    /// ([`crate::blade_lab_designs::each_swing`]).
    pub(crate) each_swing: bool,
    /// Whether its motion's last lane carries how stirred the blade is
    /// ([`crate::blade_lab_designs::stirred`]).
    pub(crate) stirred: bool,
    /// How its swing trail is drawn: the blur's afterimage, comic speed lines (the Clone
    /// Wars blade, round 19), or its signature's `@trail` or an extra's (the Sun's plasma
    /// and flare, 26/10/2026; `saber_trail.wgsl` styles 3 to 6).
    pub(crate) trail_kind: crate::blade_extras::TrailKind,
    /// Whether its trail bends what is behind it, as heat does (an extra's, the Sun's
    /// Heat wake; `saber_trail.wgsl` `fragment_wake`).
    pub(crate) wake: bool,
    /// The emitter's crackling stars ([`crate::saber::Instance::flashes`]); none when
    /// absent.
    pub(crate) flash: Option<FlashSpec>,
    /// Whether its emitter is heard crackling (`crackle` in its file).
    pub(crate) crackle: bool,
    /// Its light crackling with those stars; none unless its file's `flash.light` asks.
    pub(crate) flash_light: Option<crate::saber_submission::lights::FlashLight>,
    /// The smoke from the emitter ([`crate::saber_smoke`]), and its ribbons
    /// ([`crate::saber_ribbons`]); each none when absent.
    pub(crate) smoke: Option<crate::saber_smoke::SmokeSpec>,
    pub(crate) ribbon: Option<crate::saber_ribbons::RibbonSpec>,
    /// The sparks and bolts from the emitter ([`crate::saber_sparks`]), each set its own
    /// emitter; none when absent.
    pub(crate) sparks: [Option<crate::saber_sparks::SparkSpec>; crate::saber_sparks::MOST_SETS],
}

/// A skin's emitter stars as its blades carry them ([`crate::blade_skin_file::smoke::Flash`]):
/// how many, where along the blade (a share), how far a star's quad reaches (units, grown,
/// or the heat burst's ring), and the seconds they surge once lit, sputter once put out and
/// the burst spreads (each 0: never).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FlashSpec {
    pub(crate) count: u8,
    pub(crate) along: f32,
    pub(crate) reach: f32,
    pub(crate) ignite: f32,
    pub(crate) die: f32,
    pub(crate) burst: f32,
}

/// Where a skin's glints (or black holes) are, for their quads
/// ([`crate::saber::Instance::stars`]): `saber.wgsl` moves them round their circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Glint {
    /// Its rays' length, or how far a hole's quad reaches (units).
    pub(crate) length: f32,
    /// How far below the tip (units) they circle (0 at the tip), and how many.
    pub(crate) at: f32,
    pub(crate) count: u8,
    /// The farthest (units) a glint's heart gets from that point on the axis: the circle's
    /// radius, its tilt included.
    pub(crate) reach: f32,
}

/// A chroma skin ([`crate::unlockables::Unlockable::chroma`]) takes its wearer's saber
/// colour: every colour it draws is turned round the grey axis ([`turn_hue`]) from the
/// skin's own hue to the colour's, keeping brightness. A custom RGB colour paler than the
/// skin (Dante's idea, 06/11/2026: a blue chroma could not be worn white) also takes its
/// paleness: the colours are drawn toward grey ([`greyed`]) by [`Chroma::grey`], so white
/// or grey gives a white or grey blade at the skin's brightness and a pastel a paler one.
/// A stock colour, or an RGB one at least as strong as the skin's (or [`CHROMA_FULL`]),
/// keeps the skin's saturation, as before.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Chroma {
    /// The skin's own hue, in turns ([`hue_of`] of its corona's rim); `None` for a skin
    /// that is not a chroma.
    pub(crate) base: Option<f32>,
    /// The saturation ([`saturation_of`]) of the colour `base` was taken from.
    pub(crate) saturation: f32,
    /// Turns from the skin's hue to its wearer's colour; 0 draws the file's colours.
    pub(crate) turn: f32,
    /// How far its colours are drawn toward grey for a pale wearer's colour, 0 (none) to
    /// 1 (grey); [`greyed`].
    pub(crate) grey: f32,
    /// Whether its emitter's smoke, ribbons, sparks and bolts turn too (`chroma.effects`).
    pub(crate) effects: bool,
}

/// Below this saturation (`(max − min) / max` of a colour) a colour has no hue to take. Kept
/// small: a pastel (207, 190, 234 is 0.19) still has one, and a cliff higher up flipped a
/// chroma blade from violet to its own cyan between two slider steps (Kushina's report).
const CHROMA_GREY: f32 = 0.03;

/// A wearer's colour at least this saturated keeps a chroma's own saturation (the stock
/// colours are 0.8 to 0.9): below it, or below the skin's own when that is less, the
/// blade fades toward grey in step with the colour ([`Chroma::grey`]).
pub(crate) const CHROMA_FULL: f32 = 0.75;

/// `rgb`'s saturation, `(max - min) / max`; 0 for black.
pub(crate) fn saturation_of(rgb: [f32; 3]) -> f32 {
    let high = rgb[0].max(rgb[1]).max(rgb[2]);
    let low = rgb[0].min(rgb[1]).min(rgb[2]);
    if high <= 0.0 {
        0.0
    } else {
        (high - low) / high
    }
}

/// `rgb` drawn `grey` (0 to 1) of the way to the grey of its own brightness (the mean of
/// its channels): `saber.wgsl`'s `skin_greyed`. Linear, so it can be applied to a sum.
pub(crate) fn greyed(rgb: [f32; 3], grey: f32) -> [f32; 3] {
    kept_saturation(rgb, 1.0 - grey)
}

/// `rgb` with `keep` of its distance from the grey axis (1 unchanged, 0 grey, above 1
/// stronger: what undoes an earlier [`greyed`]).
fn kept_saturation(rgb: [f32; 3], keep: f32) -> [f32; 3] {
    let mean = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
    rgb.map(|channel| (mean + (channel - mean) * keep).max(0.0))
}

/// A chroma's turn and greying in one number, as the blade instance (`saber.wgsl`
/// decodes it in `blade_vertex`), the swatches and the petals carry it: the turn in
/// [0, 1) plus the greying in 255ths as the whole part. 0 draws the file's colours.
pub(crate) fn pack_chroma(turn: f32, grey: f32) -> f32 {
    let turn = turn.rem_euclid(1.0);
    let turn = if turn < 1.0 { turn } else { 0.0 };
    turn + (grey.clamp(0.0, 1.0) * 255.0).round()
}

/// `rgb` as a chroma packed `packed` ([`pack_chroma`]) draws it: turned, then greyed.
pub(crate) fn wear_chroma(rgb: [f32; 3], packed: f32) -> [f32; 3] {
    if packed == 0.0 {
        return rgb;
    }
    let grey = (packed.floor() / 255.0).min(1.0);
    let turned = turn_hue(rgb, packed.fract());
    if grey > 0.0 {
        greyed(turned, grey)
    } else {
        turned
    }
}

impl Chroma {
    /// Its turn and greying packed for the shader ([`pack_chroma`]).
    pub(crate) fn packed(self) -> f32 {
        pack_chroma(self.turn, self.grey)
    }
}

/// `rgb`'s hue in turns [0, 1): its angle round the grey axis from pure red, in the
/// direction [`turn_hue`] turns; `None` for a colour too grey to have one.
pub(crate) fn hue_of(rgb: [f32; 3]) -> Option<f32> {
    let high = rgb[0].max(rgb[1]).max(rgb[2]);
    let low = rgb[0].min(rgb[1]).min(rgb[2]);
    if high <= 0.0 || (high - low) / high < CHROMA_GREY {
        return None;
    }
    // Red's direction off the grey axis, and the axis crossed with it (turn_hue's sense).
    let x = (2.0 * rgb[0] - rgb[1] - rgb[2]) / 6.0_f32.sqrt();
    let y = (rgb[1] - rgb[2]) / 2.0_f32.sqrt();
    Some((y.atan2(x) / std::f32::consts::TAU).rem_euclid(1.0))
}

impl SkinColor {
    /// This skin on a blade whose stock colour is `stock`: a chroma turned to its hue,
    /// its light and trail with it; any other skin, or a grey `stock`, as it is.
    pub(crate) fn worn_with(self, stock: BladeColor) -> Self {
        let Some(base) = self.chroma.base else {
            return self;
        };
        let (rgb, custom) = match stock {
            BladeColor::Retail(color) => (color.blade_rgb(), false),
            BladeColor::Rgb(rgb) => (rgb, true),
            BladeColor::Skin(_) | BladeColor::Black => return self,
        };
        let rgb = rgb.map(|channel| f32::from(channel) / 255.0);
        let turn = hue_of(rgb).map_or(0.0, |hue| (hue - base).rem_euclid(1.0));
        // A custom colour paler than the skin (and than CHROMA_FULL) greys it in step.
        let full = self.chroma.saturation.min(CHROMA_FULL);
        let grey = if custom && full > 0.0 {
            (1.0 - saturation_of(rgb) / full).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let effects = self.chroma.effects;
        // The emitter's particles turn with it, when the skin says (`chroma.effects`): by
        // what is left of the turn and the greying, so wearing it again changes nothing
        // (a fully grey skin worn again in a colour stays grey: its hue is gone).
        let left = turn - self.chroma.turn;
        let keep = if self.chroma.grey < 1.0 {
            (1.0 - grey) / (1.0 - self.chroma.grey)
        } else {
            1.0
        };
        let turned = |rgb: [f32; 3]| {
            if !effects {
                return rgb;
            }
            let rgb = if left != 0.0 {
                turn_hue(rgb, left)
            } else {
                rgb
            };
            if keep != 1.0 {
                kept_saturation(rgb, keep)
            } else {
                rgb
            }
        };
        Self {
            chroma: Chroma {
                base: Some(base),
                saturation: self.chroma.saturation,
                turn,
                grey,
                effects,
            },
            smoke: self.smoke.map(|smoke| crate::saber_smoke::SmokeSpec {
                color: turned(smoke.color),
                ..smoke
            }),
            ribbon: self.ribbon.map(|ribbon| crate::saber_ribbons::RibbonSpec {
                color: turned(ribbon.color),
                ..ribbon
            }),
            sparks: self.sparks.map(|sparks| {
                sparks.map(|sparks| crate::saber_sparks::SparkSpec {
                    color: turned(sparks.color),
                    ..sparks
                })
            }),
            ..self
        }
    }

    /// The dynamic light's colour, a chroma's turned to its wearer's.
    pub(crate) fn light_color(&self) -> [f32; 3] {
        self.chromatic(self.light)
    }

    /// The blur trail's colour, a chroma's turned to its wearer's.
    pub(crate) fn trail_color(&self) -> [f32; 3] {
        self.chromatic(self.trail)
    }

    /// The blur trail's colour at animation time `seconds`, red through Rakata's 30-second surge.
    pub(crate) fn trail_color_surging(&self, seconds: f32) -> [f32; 3] {
        fn ease(a: f32, b: f32, x: f32) -> f32 {
            let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        }
        let t = seconds.rem_euclid(30.0);
        let surge = ease(18.0, 21.0, t) * (1.0 - ease(25.0, 29.0, t));
        let red = [1.0, 0.1, 0.08];
        self.chromatic(std::array::from_fn(|i| {
            self.trail[i] + (red[i] - self.trail[i]) * surge
        }))
    }

    fn chromatic(&self, rgb: [f32; 3]) -> [f32; 3] {
        wear_chroma(rgb, self.chroma.packed())
    }
}

impl SkinColor {
    /// What its emitter stars do `since` seconds after the blade lit, or `out` seconds
    /// after it was put out (each none otherwise): the surge, 1 the moment it lights and
    /// falling off over the flash's `ignite` seconds; the sputter, what is left of its
    /// `die` seconds; the heat burst, what is left of its `time`. Nothing without a flash.
    pub(crate) fn flash_moments(
        &self,
        since: Option<f32>,
        out: Option<f32>,
    ) -> crate::saber::Moments {
        let Some(flash) = self.flash else {
            return crate::saber::Moments::default();
        };
        let left = |at: Option<f32>, span: f32| {
            at.filter(|at| span > 0.0 && *at < span)
                .map_or(0.0, |at| 1.0 - at / span)
        };
        let surge = left(since, flash.ignite);
        crate::saber::Moments {
            surge: surge * surge,
            dying: left(out, flash.die),
            burst: left(since, flash.burst),
        }
    }

    /// The light's colour at `time_millis`: the file's, turned with the hue of the
    /// blade's middle when the skin's hue turns.
    pub(crate) fn light_at(&self, time_millis: i64) -> [f32; 3] {
        let [rate, middle] = self.hue;
        if rate == 0.0 && middle == 0.0 {
            return self.light_color();
        }
        let t = (time_millis.rem_euclid(1_024_000)) as f32 * 0.001;
        greyed(
            turn_hue(self.light, t * rate + middle + self.chroma.turn),
            self.chroma.grey,
        )
    }
}

/// `rgb` turned `turns` round the grey axis, negative channels clipped: the CPU's copy of
/// `saber.wgsl`'s `skin_turn_hue` (a hue shift keeping brightness and saturation).
pub(crate) fn turn_hue(rgb: [f32; 3], turns: f32) -> [f32; 3] {
    let axis = 1.0 / 3.0_f32.sqrt();
    let (s, c) = (std::f32::consts::TAU * turns).sin_cos();
    let along_axis = axis * (rgb[0] + rgb[1] + rgb[2]) * axis * (1.0 - c);
    // axis × rgb, with every axis component equal.
    let cross = [
        axis * (rgb[2] - rgb[1]),
        axis * (rgb[0] - rgb[2]),
        axis * (rgb[1] - rgb[0]),
    ];
    std::array::from_fn(|i| (rgb[i] * c + cross[i] * s + along_axis).max(0.0))
}

/// One skin's parameters as `saber.wgsl`'s `Skin` holds them: 105 `vec4`s (1680 bytes, so
/// the [`MAX_SKINS`] array is 430080 bytes, a read-only storage binding). The layout is the shader's; see there for what each lane is. A section
/// the skin's file leaves out (arcs, motes, hue and the later ones) is zeros, which the
/// shader draws as nothing.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub(crate) struct SkinUniform {
    core_white: [f32; 4],
    core_fringe_cool: [f32; 4],
    core_fringe_hot: [f32; 4],
    core_fringe: [f32; 4],
    core_breathe: [f32; 4],
    rim_cool: [f32; 4],
    rim_hot: [f32; 4],
    inner: [f32; 4],
    inner_mix: [f32; 4],
    brightness: [f32; 4],
    swell: [f32; 4],
    grain_coarse: [f32; 4],
    grain_fine: [f32; 4],
    grain_mix: [f32; 4],
    flare_rate: [f32; 4],
    flare_shape: [f32; 4],
    flare_size: [f32; 4],
    shimmer_a: [f32; 4],
    shimmer_b: [f32; 4],
    tongue_a: [f32; 4],
    tongue_b: [f32; 4],
    arc_color: [f32; 4],
    arc_shape: [f32; 4],
    arc_strike: [f32; 4],
    arc_place: [f32; 4],
    arc_motion: [f32; 4],
    mote_color: [f32; 4],
    mote_field: [f32; 4],
    mote_motion: [f32; 4],
    mote_band: [f32; 4],
    hue: [f32; 4],
    sputter_edge: [f32; 4],
    sputter_cut: [f32; 4],
    glitch_a: [f32; 4],
    glitch_b: [f32; 4],
    scan_color: [f32; 4],
    scan_lines: [f32; 4],
    scan_edge: [f32; 4],
    scan_jitter: [f32; 4],
    pulse_a: [f32; 4],
    pulse_b: [f32; 4],
    ember_color: [f32; 4],
    ember_field: [f32; 4],
    ember_shape: [f32; 4],
    vein_color: [f32; 4],
    vein_shape: [f32; 4],
    team_red: [f32; 4],
    team_blue: [f32; 4],
    team_none: [f32; 4],
    ambient: [f32; 4],
    glyph_color: [f32; 4],
    glyph_shape: [f32; 4],
    star_color: [f32; 4],
    star_shape: [f32; 4],
    star_motion: [f32; 4],
    star_orbit: [f32; 4],
    hole_color: [f32; 4],
    hole_shape: [f32; 4],
    wisp_color: [f32; 4],
    wisp_shape: [f32; 4],
    wisp_field: [f32; 4],
    haze: [f32; 4],
    band_color: [f32; 4],
    band_shape: [f32; 4],
    band_mix: [f32; 4],
    segment_a: [f32; 4],
    segment_b: [f32; 4],
    film: [f32; 4],
    film_b: [f32; 4],
    blob_color: [f32; 4],
    blob_shape: [f32; 4],
    blob_more: [f32; 4],
    front_color: [f32; 4],
    front_shape: [f32; 4],
    front_edge: [f32; 4],
    sketch_color: [f32; 4],
    sketch_shape: [f32; 4],
    hour_color: [f32; 4],
    hour_shape: [f32; 4],
    serpent_color: [f32; 4],
    serpent_shape: [f32; 4],
    strand_shape: [f32; 4],
    strand_hue: [f32; 4],
    fly_color: [f32; 4],
    fly_shape: [f32; 4],
    fly_roam: [f32; 4],
    wing_color: [f32; 4],
    wing_shape: [f32; 4],
    copy_shape: [f32; 4],
    copy_more: [f32; 4],
    ink_color: [f32; 4],
    ink_shape: [f32; 4],
    ink_more: [f32; 4],
    chrome_color: [f32; 4],
    chrome_shape: [f32; 4],
    /// The Blade Lab design it is drawn as ([`crate::blade_lab_designs::Design::lane`]);
    /// zeros for every other skin.
    design: [f32; 4],
    /// The design's colour (the lab's), for the light at its emitter.
    design_color: [f32; 4],
    unrest: [f32; 4],
    unrest_lean: [f32; 4],
    flash_color: [f32; 4],
    flash_shape: [f32; 4],
    flash_motion: [f32; 4],
    flash_place: [f32; 4],
    flash_burst: [f32; 4],
    /// The arcs' climb at a clash (`unrest.clash_time`); zeros without.
    unrest_clash: [f32; 4],
}

impl SkinUniform {
    /// The parameters of `def`, as the shader reads them.
    pub(crate) fn of(def: &BladeSkinDef) -> Self {
        let rgb = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        let wave = |w: &crate::blade_skin_file::ShimmerWave| [w.amount, w.rate, w.along, w.seed];
        let octave =
            |index: usize| {
                def.granulation.octaves.get(index).copied().unwrap_or(
                    crate::blade_skin_file::Octave {
                        scale: 0.0,
                        speed: 0.0,
                        evolve: 0.0,
                        offset: 0.0,
                        weight: 0.0,
                    },
                )
            };
        let (coarse, fine) = (octave(0), octave(1));
        let shimmer = |index: usize| def.shimmer.get(index).map_or([0.0; 4], wave);
        let (c, r, f, t) = (&def.core, &def.corona, &def.flares, &def.tongues);
        Self {
            core_white: rgb(c.white, c.white_flare),
            core_fringe_cool: rgb(c.fringe_cool, c.fringe_heat.base),
            core_fringe_hot: rgb(c.fringe_hot, c.fringe_heat.grain),
            core_fringe: [
                c.fringe_brightness.base,
                c.fringe_brightness.grain,
                c.fringe_brightness.flare,
                c.tip,
            ],
            core_breathe: wave(&c.breathe),
            rim_cool: rgb(r.rim_cool, r.inner_width),
            rim_hot: rgb(r.rim_hot, r.rim_heat.grain),
            inner: rgb(r.inner, r.rim_heat.flare),
            inner_mix: [r.inner_mix.base, r.inner_mix.grain, r.inner_mix.flare, 0.0],
            brightness: [
                r.brightness.base,
                r.brightness.grain,
                r.brightness.flare,
                0.0,
            ],
            swell: [r.reach, r.swell.grain, r.swell.flare, 0.0],
            grain_coarse: [coarse.scale, coarse.speed, coarse.evolve, coarse.offset],
            grain_fine: [fine.scale, fine.speed, fine.evolve, fine.offset],
            grain_mix: [
                coarse.weight,
                fine.weight,
                def.granulation.low,
                def.granulation.high,
            ],
            flare_rate: [f.rate, f.rate_step, f.rate_seed, f.count as f32],
            flare_shape: [f.phase_seed, f.phase_step, f.threshold, f.overshoot],
            flare_size: [f.size, f.size_jitter, 0.0, 0.0],
            shimmer_a: shimmer(0),
            shimmer_b: shimmer(1),
            tongue_a: [t.along, t.offset, t.out, t.speed],
            tongue_b: [t.edge_low, t.edge_high, t.low, t.range],
            ..Self::default()
        }
        .with_arcs(def.arcs.as_ref())
        .with_motes(def.motes.as_ref())
        .with_hue(def.hue.as_ref())
        .with_effects(def.effects())
        .with_extras(def)
        .with_lab(def.lab())
        .with_figures(def.figures())
        .with_solids(def.solids())
        .with_unrest(def.unrest.as_ref())
        .with_flash(def.flash.as_ref())
    }

    /// Where the blade is unsettled ([`crate::blade_skin_file::smoke::Unrest`]): 1, reach,
    /// fade, far; zeros when absent (unsettled all along).
    fn with_unrest(mut self, unrest: Option<&crate::blade_skin_file::smoke::Unrest>) -> Self {
        if let Some(u) = unrest {
            self.unrest = [1.0, u.reach, u.fade, u.far];
            // The swing's and the clash's units, by the blade's motion (`saber.wgsl`).
            self.unrest_lean = [u.lean, u.swing, u.haze, u.clash];
            self.unrest_clash = [u.clash_time, 0.0, 0.0, 0.0];
        }
        self
    }

    /// The emitter's crackling star ([`crate::blade_skin_file::smoke::Flash`]); zeros when
    /// absent.
    fn with_flash(mut self, flash: Option<&crate::blade_skin_file::smoke::Flash>) -> Self {
        if let Some(f) = flash {
            self.flash_color = [f.color[0], f.color[1], f.color[2], f.brightness];
            self.flash_shape = [f.rays as f32, f.length, f.width, f.halo];
            self.flash_motion = [f.rate, f.chaos, f.jitter, f.lean];
            self.flash_place = [f.along, f.surge, f.grow, 0.0];
            if let Some(b) = &f.burst {
                self.flash_burst = [b.strength, b.reach, b.width, 0.0];
            }
        }
        self
    }

    /// The seventh set's lanes ([`crate::blade_skin_file::solids`]).
    fn with_solids(mut self, s: crate::blade_skin_file::solids::Solids<'_>) -> Self {
        let rgb = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        if let Some(i) = s.ink {
            self.ink_color = rgb(i.color, i.body);
            self.ink_shape = [i.outline, i.width, i.edge, i.point];
            self.ink_more = [i.gloss, i.halftone, 0.0, 0.0];
        }
        if let Some(c) = s.chrome {
            self.chrome_color = rgb(c.color, c.width);
            self.chrome_shape = [c.ripple, c.beads, c.speed, 0.0];
        }
        self
    }

    /// The sixth set's lanes ([`crate::blade_skin_file::figures`]).
    fn with_figures(mut self, f: crate::blade_skin_file::figures::Figures<'_>) -> Self {
        let rgb = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        if let Some(s) = f.sketch {
            self.sketch_color = rgb(s.color, s.brightness);
            self.sketch_shape = [s.rate, s.wobble, s.hatch, s.glow];
        }
        if let Some(h) = f.hourglass {
            self.hour_color = rgb(h.color, h.brightness);
            self.hour_shape = [h.period, h.waist, h.glass, 0.0];
        }
        if let Some(s) = f.serpent {
            self.serpent_color = rgb(s.color, s.brightness);
            self.serpent_shape = [s.coils, s.amplitude, s.speed, s.thickness];
        }
        if let Some(s) = f.strands {
            self.strand_shape = [s.count as f32, s.spread, s.sway, s.brightness];
            self.strand_hue = [s.hue, s.saturation, 0.0, 0.0];
        }
        if let Some(b) = f.butterflies {
            self.fly_color = rgb(b.color, b.brightness);
            self.fly_shape = [b.count as f32, b.size, b.flap, b.flight];
            self.fly_roam = [b.roam, 0.0, 0.0, 0.0];
        }
        if let Some(w) = f.wings {
            self.wing_color = rgb(w.color, w.brightness);
            self.wing_shape = [w.span, w.rate, w.feathers as f32, 0.0];
        }
        if let Some(c) = f.copies {
            self.copy_shape = [c.count as f32, c.angle, c.fade, c.period];
            self.copy_more = [c.flicker, 0.0, 0.0, 0.0];
        }
        self
    }

    /// The fifth set's lanes ([`crate::blade_skin_file::lab`]).
    fn with_lab(mut self, lab: crate::blade_skin_file::lab::Lab<'_>) -> Self {
        let rgb = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        if let Some(b) = lab.bands {
            self.band_color = rgb(b.color, b.brightness);
            self.band_shape = [b.spacing, b.width, b.twist, b.speed];
            self.band_mix = [b.mix, b.pinch, 0.0, 0.0];
        }
        if let Some(s) = lab.segments {
            self.segment_a = [s.count as f32, s.rate, s.threshold, s.depth];
            self.segment_b = [s.gap, s.hum, 0.0, 0.0];
        }
        if let Some(f) = lab.film {
            self.film = [f.amount, f.scale, f.speed, f.saturation];
            self.film_b = [f.clear, 0.0, 0.0, 0.0];
        }
        if let Some(b) = lab.blobs {
            self.blob_color = rgb(b.color, b.brightness);
            self.blob_shape = [b.count as f32, b.size, b.speed, b.wobble];
            self.blob_more = [b.pools, 0.0, 0.0, 0.0];
        }
        if let Some(f) = lab.front {
            self.front_color = rgb(f.color, f.amount);
            self.front_shape = [f.period, f.duty, f.hold, f.reach];
            self.front_edge = [f.soft, f.jag, f.veins, 0.0];
        }
        self
    }

    /// The third set's lanes ([`crate::blade_skin_file::extras`]), and the room the glow's
    /// quad keeps past the glow for them (`swell.w`, units, before any option is off).
    fn with_extras(mut self, def: &BladeSkinDef) -> Self {
        let rgb = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        if let Some(s) = &def.star {
            self.star_color = rgb(s.color, s.brightness);
            self.star_shape = [s.rays as f32, s.length, s.width, s.halo];
            self.star_motion = [s.spin, s.twinkle, s.depth, 0.0];
        }
        if let Some(h) = &def.holes {
            self.hole_color = rgb(h.color, h.brightness);
            self.hole_shape = [h.size, h.lens, h.spin, h.reach()];
        }
        if let Some(o) = def
            .orbit
            .filter(|_| def.star.is_some() || def.holes.is_some())
        {
            self.star_motion[3] = o.count as f32;
            self.star_orbit = [o.at, o.radius, o.rate, o.tilt];
        }
        if let Some(w) = &def.wisps {
            self.wisp_color = rgb(w.color, w.brightness);
            self.wisp_shape = [w.rise, w.speed, w.scale, w.curl];
            self.wisp_field = [w.density, 0.0, 0.0, 0.0];
        }
        if let Some(h) = &def.haze {
            self.haze = [h.strength, h.scale, h.speed, h.reach];
        }
        self
    }

    /// The later sections' lanes ([`crate::blade_skin_file::effects`]).
    fn with_effects(mut self, e: crate::blade_skin_file::effects::Effects<'_>) -> Self {
        let rgb = |c: [f32; 3], w: f32| [c[0], c[1], c[2], w];
        if let Some(s) = e.sputter {
            self.sputter_edge = [s.ragged, s.scale, s.speed, 0.0];
            self.sputter_cut = [s.cut, s.rate, s.threshold, 0.0];
        }
        if let Some(g) = e.glitch {
            self.glitch_a = [g.split, g.blocks, g.rate, g.threshold];
            self.glitch_b = [g.shift, g.flash, 0.0, 0.0];
        }
        if let Some(s) = e.scan {
            self.scan_color = rgb(s.color, s.brightness);
            self.scan_lines = [s.lines, s.speed, s.depth, s.rings];
            self.scan_edge = [s.edge, s.width, s.hollow, 0.0];
            self.scan_jitter = [s.jitter, s.jitter_rate, 0.0, 0.0];
        }
        if let Some(p) = e.pulse {
            self.pulse_a = [p.rate, p.amount, p.second, p.gap];
            self.pulse_b = [p.width, p.swell, 0.0, 0.0];
        }
        if let Some(m) = e.embers {
            self.ember_color = rgb(m.color, m.brightness);
            self.ember_field = [m.density, m.cells, m.fall, m.life];
            self.ember_shape = [m.size, m.spread, 0.0, 0.0];
        }
        if let Some(v) = e.veins {
            self.vein_color = rgb(v.color, v.brightness);
            self.vein_shape = [v.scale, v.speed, v.width, v.core];
        }
        if let Some(t) = e.team {
            self.team_red = rgb(t.red, t.amount);
            self.team_blue = rgb(t.blue, 0.0);
            self.team_none = rgb(t.none, 0.0);
        }
        if let Some(a) = e.ambient {
            self.ambient = [a.amount, a.saturate, a.floor, 0.0];
        }
        if let Some(g) = e.glyphs {
            self.glyph_color = rgb(g.color, g.brightness);
            self.glyph_shape = [g.size, g.spacing, g.speed, g.width];
        }
        self
    }

    fn with_arcs(mut self, arcs: Option<&crate::blade_skin_file::Arcs>) -> Self {
        if let Some(a) = arcs {
            self.arc_color = [a.color[0], a.color[1], a.color[2], a.brightness];
            self.arc_shape = [a.width, a.halo, a.jag, a.kinks];
            self.arc_strike = [a.count as f32, a.rate, a.threshold, a.decay];
            self.arc_place = [a.reach, a.span.low, a.span.high, a.tip];
            self.arc_motion = [a.jitter, a.crawl, a.root, 0.0];
        }
        self
    }

    fn with_motes(mut self, motes: Option<&crate::blade_skin_file::Motes>) -> Self {
        if let Some(m) = motes {
            self.mote_color = [m.color[0], m.color[1], m.color[2], m.brightness];
            self.mote_field = [m.density, m.cells, m.rings, m.size];
            self.mote_motion = [m.drift.along, m.drift.out, m.twinkle, m.stretch];
            self.mote_band = [m.inner, m.outer, m.focus, 0.0];
        }
        self
    }

    fn with_hue(mut self, hue: Option<&crate::blade_skin_file::Hue>) -> Self {
        if let Some(h) = hue {
            self.hue = [h.rate, h.along, h.out, 0.0];
        }
        self
    }
}

/// A loaded blade skin.
#[derive(Clone, Debug)]
pub(crate) struct LoadedSkin {
    /// The catalogue's id of the unlock it is.
    pub(crate) id: &'static str,
    pub(crate) def: BladeSkinDef,
    /// Its Blade Lab design, if it has one, and `def` as the design draws it
    /// ([`crate::blade_lab_designs`]).
    pub(crate) design: Option<&'static crate::blade_lab_designs::Design>,
    designed: Option<Box<BladeSkinDef>>,
    /// Its Blade Lab signature shader, if it has one ([`crate::blade_signatures`]).
    pub(crate) signature: Option<&'static crate::blade_signatures::Signature>,
    /// Its glow and core images, RGBA.
    pub(crate) glow: image::RgbaImage,
    pub(crate) core: image::RgbaImage,
}

impl LoadedSkin {
    /// Its chroma, unturned: a chroma by the catalogue or by its own file (`chroma`, as a
    /// Saber Lab blade, which has no catalogue entry, is); the default for the others.
    fn chroma(&self) -> Chroma {
        let chroma = crate::unlockables::is_chroma(self.id) || self.def.chroma.is_some();
        let def = self.shown();
        // The design's colour is what the shader turns, else the corona's rim (or the
        // light, for a rim with no hue).
        let source = match self.design_shown() {
            Some(design) => Some(design.color),
            None => [def.corona.rim_cool, def.light.color]
                .into_iter()
                .find(|rgb| hue_of(*rgb).is_some()),
        };
        let base = source.filter(|_| chroma).and_then(hue_of);
        Chroma {
            base,
            saturation: source.filter(|_| base.is_some()).map_or(0.0, saturation_of),
            turn: 0.0,
            grey: 0.0,
            effects: chroma && self.def.chroma.is_none_or(|chroma| chroma.effects),
        }
    }

    /// Read `def` (the file of unlock `id`), taking its images from `packs`.
    pub(crate) fn new(
        id: &'static str,
        mut def: BladeSkinDef,
        packs: &VirtualFileSystem,
    ) -> Result<Self, String> {
        // A signature skin keeps its own sounds and heat from its built-in file, whatever an
        // older pack file of the same skin says (Creyon's round 9: the Sith Alchemy still
        // hissed with the snake design's sounds).
        if let Some(signature) = crate::blade_signatures::of(id) {
            signature.adopt(&mut def);
        }
        let image =
            |path: &Option<String>, size: [u32; 2], generated: fn(&BladeSkinDef) -> Vec<u8>| {
                match path {
                    Some(path) => pack_image(packs, path),
                    None => image::RgbaImage::from_raw(size[0], size[1], generated(&def))
                        .ok_or_else(|| "a generated image of the wrong size".to_owned()),
                }
            };
        let glow = image(&def.glow_image, crate::saber_rgb::glow_size(), |def| {
            crate::saber_rgb::generated_glow(def.glow_profile)
        })?;
        let core = image(&def.core_image, crate::saber_rgb::core_size(), |def| {
            crate::saber_rgb::generated_core(def.core_profile)
        })?;
        let design = crate::blade_lab_designs::of(id);
        let signature = crate::blade_signatures::of(id);
        Ok(Self {
            id,
            // A signature replaces a hand-ported design of the same skin.
            designed: match signature {
                Some(signature) => Some(Box::new(signature.shown(&def))),
                None => design.map(|design| Box::new(design.shown(&def))),
            },
            design: design.filter(|_| signature.is_none()),
            signature,
            def,
            glow,
            core,
        })
    }

    /// Its design, while `cg_saberLabDesigns` draws the designs.
    pub(crate) fn design_shown(&self) -> Option<&'static crate::blade_lab_designs::Design> {
        self.design.filter(|_| crate::blade_lab_designs::enabled())
    }

    /// Its signature shader, while `cg_saberLabDesigns` draws the designs.
    pub(crate) fn signature_shown(&self) -> Option<&'static crate::blade_signatures::Signature> {
        self.signature
            .filter(|_| crate::blade_lab_designs::enabled())
    }

    /// Its file as drawn: as its design draws it while the designs are drawn.
    pub(crate) fn shown(&self) -> &BladeSkinDef {
        match &self.designed {
            Some(designed) if crate::blade_lab_designs::enabled() => designed,
            _ => &self.def,
        }
    }

    /// How a blade whose stock colour is `stock` changes its colours: a chroma's turn and
    /// greying to that colour ([`SkinColor::worn_with`]), packed ([`Chroma::packed`]); 0
    /// for any other skin.
    pub(crate) fn chroma_turn(&self, stock: BladeColor) -> f32 {
        self.color(0, 0).worn_with(stock).chroma.packed()
    }

    /// What its blades are drawn with, as the loaded skin numbered `index`, worn with
    /// the options in `off` switched from their default (its parts off, its extras on).
    fn color(&self, index: SkinIndex, off: crate::saber_skin_options::OptionsOff) -> SkinColor {
        use crate::blade_extras::TrailKind;
        use crate::blade_skin_file::extras::Section;
        // The extras are a signature's, drawn while the designs are.
        let extras = self.signature_shown().map_or_else(Default::default, |_| {
            crate::blade_extras::worn(self.id, off)
        });
        let def = self.shown();
        let off = def.off_mask(off);
        let on = |section: Section| off & section.bit() == 0;
        let flicker = &def.light.flicker;
        let mut waves = [[0.0; 3]; 2];
        for (slot, wave) in waves.iter_mut().zip(&flicker.waves) {
            *slot = [wave.rate, wave.weight, wave.phase];
        }
        SkinColor {
            index,
            trail: def.trail,
            light: def.light.color,
            flicker: LightFlicker {
                amount: flicker.amount,
                waves,
                pulse: self
                    .def
                    .pulse
                    .filter(|_| on(Section::Pulse))
                    .map(|pulse| Beat {
                        rate: pulse.rate,
                        amount: pulse.amount,
                        second: pulse.second,
                        gap: pulse.gap,
                        width: pulse.width,
                    }),
            },
            hue: self
                .def
                .hue
                .filter(|_| on(Section::Hue))
                .map_or([0.0; 2], |hue| [hue.rate, hue.along * 20.0]),
            ghosts: self
                .def
                .ghosts
                .filter(|_| on(Section::Ghosts))
                .map(|ghosts| crate::saber_ghosts::GhostSpec {
                    count: ghosts.count as u8,
                    spacing_millis: ghosts.spacing,
                    fade: ghosts.fade,
                    wearer: crate::blade_signatures::of(self.id).is_some(),
                }),
            petals: self
                .signature_shown()
                .and_then(|signature| signature.petals())
                .filter(|petals| {
                    crate::blade_signatures::part_section(petals.part)
                        .is_some_and(|section| on(section))
                })
                .or(extras.sheds),
            persona: crate::saber_persona::Persona::default(),
            linger: crate::blade_lab_designs::linger(self.id),
            each_swing: crate::blade_lab_designs::each_swing(self.id),
            stirred: crate::blade_lab_designs::stirred(self.id),
            // An extra's trail, else its signature's; the comic lines with the Clone Wars
            // design's comic panel (the sputter bit of its options), while the designs draw.
            trail_kind: extras
                .trail
                .or_else(|| {
                    self.signature_shown()
                        .and_then(|signature| signature.trail())
                })
                .unwrap_or(
                    if self
                        .design_shown()
                        .is_some_and(|design| crate::blade_lab_designs::comic_trail(design.id))
                        && on(Section::Sputter)
                    {
                        TrailKind::Comic
                    } else {
                        TrailKind::Blur
                    },
                ),
            wake: extras.wake,
            chroma: self.chroma(),
            off,
            // A design's quad reaches its glow's reach and its particles' room
            // (`saber.wgsl` `blade_vertex`).
            room: self
                .design_shown()
                .map_or(def.room(off), |design| {
                    design.reach.max(design.room) / crate::blade_lab_designs::METRE
                })
                .max(
                    self.signature_shown()
                        .map_or(0.0, |signature| signature.room()),
                ),
            star: self
                .def
                .star
                .filter(|_| on(Section::Star))
                .map(|star| star.length)
                .or_else(|| {
                    self.def
                        .holes
                        .filter(|_| on(Section::Holes))
                        .map(|holes| holes.reach())
                })
                .map(|length| match def.orbit {
                    Some(orbit) => Glint {
                        length,
                        at: orbit.at,
                        count: orbit.count as u8,
                        reach: orbit.radius / orbit.tilt.cos(),
                    },
                    None => Glint {
                        length,
                        at: 0.0,
                        count: 1,
                        reach: 0.0,
                    },
                }),
            echo: def.echo.filter(|_| on(Section::Echo)),
            flash: def.flash.filter(|_| on(Section::Flash)).map(|f| {
                let star = f.length * (1.0 + f.lean) * (1.0 + f.grow) + f.jitter;
                let ring = f.burst.map_or(0.0, |b| b.reach + b.width);
                FlashSpec {
                    count: f.count as u8,
                    along: f.along,
                    reach: star.max(ring),
                    ignite: f.ignite,
                    die: f.die,
                    burst: f.burst.map_or(0.0, |b| b.time),
                }
            }),
            crackle: def.crackle.is_some() && on(Section::Crackle),
            flash_light: def
                .flash
                .filter(|f| f.light > 0.0 && on(Section::Flash))
                .map(|f| crate::saber_submission::lights::FlashLight {
                    count: f.count as u8,
                    rays: f.rays as u8,
                    rate: f.rate,
                    chaos: f.chaos,
                    light: f.light,
                    surge: f.surge,
                }),
            smoke: def
                .smoke
                .as_ref()
                .filter(|_| on(Section::Smoke))
                .map(crate::saber_smoke::SmokeSpec::of),
            ribbon: self
                .def
                .ribbon
                .as_ref()
                .filter(|_| on(Section::Ribbon))
                .map(crate::saber_ribbons::RibbonSpec::of),
            sparks: crate::saber_sparks::SparkSpec::sets_of(
                &self.def,
                on(Section::Sparks),
                on(Section::Bolts),
            ),
        }
    }

    /// The options its file offers.
    pub(crate) fn options(&self) -> &[crate::blade_skin_file::extras::SkinOption] {
        &self.shown().options
    }
}

/// An image file `path` of the packs, decoded.
fn pack_image(packs: &VirtualFileSystem, path: &str) -> Result<image::RgbaImage, String> {
    let bytes = packs
        .read(path)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("{path} is not in the pack"))?
        .bytes;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("{path}: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || width > IMAGE_MAX || height > IMAGE_MAX {
        return Err(format!(
            "{path} is {width}×{height}; images are 1 to {IMAGE_MAX} a side"
        ));
    }
    Ok(image)
}

/// The blade skins loaded from the packs, numbered in id order, and the packs
/// themselves (their sounds are read from there).
#[derive(Clone, Default)]
pub(crate) struct LoadedSkins {
    skins: Vec<LoadedSkin>,
    /// Every pack, mounted in name order.
    packs: Arc<VirtualFileSystem>,
    /// Counts loads; 0 is the empty table before any.
    generation: u64,
    /// The one skin this table changed from the table of the generation given
    /// ([`Self::edited`]), so that only it is uploaded again.
    edit: Option<(u64, SkinIndex)>,
}

impl std::fmt::Debug for LoadedSkins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedSkins")
            .field("ids", &self.ids().collect::<Vec<_>>())
            .field("generation", &self.generation)
            .finish()
    }
}

impl LoadedSkins {
    /// The skins of the blade-skin files in `packs` whose ids are blade skins of the
    /// catalogue, at most [`MAX_SKINS`]; the others are named in the log.
    pub(crate) fn load(packs: VirtualFileSystem, generation: u64) -> Self {
        let mut skins = Vec::new();
        for (id, def) in crate::blade_skin_file::load(&packs) {
            let path = format!(
                "{}/{id}{}",
                crate::blade_skin_file::FOLDER,
                crate::blade_skin_file::EXTENSION
            );
            let Some(known) = crate::unlockables::blade_skin(&id) else {
                crate::log::progress(format_args!(
                    "blade skins: {path}: not a blade skin this client knows; left out"
                ));
                continue;
            };
            if skins.len() == MAX_SKINS {
                crate::log::progress(format_args!(
                    "blade skins: {path}: more than {MAX_SKINS} skins; left out"
                ));
                continue;
            }
            match LoadedSkin::new(known.id, def, &packs) {
                Ok(skin) => skins.push(skin),
                Err(why) => crate::log::progress(format_args!("blade skins: {path}: {why}")),
            }
        }
        // The signature skins no pack carries yet, from the lab's files built in.
        for signature in &crate::blade_signatures::SIGNATURES {
            if skins
                .iter()
                .any(|skin: &LoadedSkin| skin.id == signature.id)
                || skins.len() == MAX_SKINS
            {
                continue;
            }
            let Some(known) = crate::unlockables::blade_skin(signature.id) else {
                continue;
            };
            let loaded = crate::blade_skin_file::parse(known.id, signature.default_file())
                .and_then(|def| LoadedSkin::new(known.id, def, &packs));
            match loaded {
                Ok(skin) => skins.push(skin),
                Err(why) => crate::log::progress(format_args!(
                    "blade skins: {}'s built-in file: {why}",
                    known.id
                )),
            }
        }
        Self {
            skins,
            packs: Arc::new(packs),
            generation,
            edit: None,
        }
    }

    /// This table with skin `id` read from `def` instead (the blade-skin editor's change,
    /// `crate::blade_editor`), as generation `generation`: its images made again only when
    /// what they come from changed.
    pub(crate) fn edited(
        &self,
        id: &str,
        def: BladeSkinDef,
        generation: u64,
    ) -> Result<Self, String> {
        let index = self
            .skins
            .iter()
            .position(|skin| skin.id == id)
            .ok_or_else(|| format!("{id} is not loaded"))?;
        let old = &self.skins[index];
        let same_images = (
            &old.def.glow_profile,
            &old.def.core_profile,
            &old.def.glow_image,
            &old.def.core_image,
        ) == (
            &def.glow_profile,
            &def.core_profile,
            &def.glow_image,
            &def.core_image,
        );
        let skin = if same_images {
            LoadedSkin { def, ..old.clone() }
        } else {
            LoadedSkin::new(old.id, def, &self.packs)?
        };
        let mut skins = self.skins.clone();
        skins[index] = skin;
        Ok(Self {
            skins,
            packs: Arc::clone(&self.packs),
            generation,
            edit: Some((self.generation, index as SkinIndex)),
        })
    }

    /// The skin [`Self::edited`] changed, and the generation of the table it changed.
    pub(crate) fn edit(&self) -> Option<(u64, SkinIndex)> {
        self.edit
    }

    /// Skins made in a test, with no pack behind them.
    #[cfg(test)]
    pub(crate) fn of(skins: Vec<LoadedSkin>, generation: u64) -> Self {
        Self {
            skins,
            packs: Arc::default(),
            generation,
            edit: None,
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Add `skin` (a look kept from an earlier load: the Saber Lab's last good one of a
    /// file that no longer parses) unless a skin of its id loaded, keeping the id order
    /// the numbers follow. Returns whether it was added.
    pub(crate) fn keep(&mut self, skin: LoadedSkin) -> bool {
        if self.skins.len() == MAX_SKINS || self.get(skin.id).is_some() {
            return false;
        }
        self.skins.push(skin);
        self.skins.sort_by(|a, b| a.id.cmp(b.id));
        true
    }

    /// The ids of the loaded skins, in their numbers' order.
    pub(crate) fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.skins.iter().map(|skin| skin.id)
    }

    /// The packs every loaded file came from.
    pub(crate) fn packs(&self) -> &Arc<VirtualFileSystem> {
        &self.packs
    }

    /// The loaded skin of unlock `id`, if its pack is loaded.
    pub(crate) fn get(&self, id: &str) -> Option<&LoadedSkin> {
        self.skins.iter().find(|skin| skin.id == id)
    }

    /// What the blades of a player wearing unlock `id` with every option on are drawn
    /// with; `None` (the stock blade) when no loaded skin is that unlock.
    #[cfg(test)]
    pub(crate) fn color_of(&self, id: &str) -> Option<SkinColor> {
        self.color_with(id, 0)
    }

    /// [`Self::color_of`], worn with the options in `off` switched off.
    pub(crate) fn color_with(
        &self,
        id: &str,
        off: crate::saber_skin_options::OptionsOff,
    ) -> Option<SkinColor> {
        self.skins
            .iter()
            .position(|skin| skin.id == id)
            .map(|index| self.skins[index].color(index as SkinIndex, off))
    }

    /// The number of the loaded skin of unlock `id` (its sound set's in the audio
    /// adapter's table), if its pack is loaded.
    pub(crate) fn index_of(&self, id: &str) -> Option<SkinIndex> {
        self.skins
            .iter()
            .position(|skin| skin.id == id)
            .map(|index| index as SkinIndex)
    }

    /// The loaded skins with their numbers.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (SkinIndex, &LoadedSkin)> {
        self.skins
            .iter()
            .enumerate()
            .map(|(index, skin)| (index as SkinIndex, skin))
    }

    /// The uniform array `saber.wgsl` reads, zero past the loaded skins.
    pub(crate) fn uniforms(&self) -> Vec<SkinUniform> {
        let mut uniforms = vec![SkinUniform::default(); MAX_SKINS];
        for (uniform, skin) in uniforms.iter_mut().zip(&self.skins) {
            *uniform = SkinUniform::of(skin.shown());
            if let Some(design) = skin.design_shown() {
                uniform.design = design.lane();
                let [r, g, b] = design.color;
                uniform.design_color = [r, g, b, 1.0];
            }
            if let Some(signature) = skin.signature_shown() {
                uniform.design = signature.lane();
                // The light's colour, and a chroma's own hue (turns; -1 for none), which
                // the shader turns to its wearer's.
                let [r, g, b] = skin.def.light.color;
                let hue = crate::unlockables::is_chroma(skin.id)
                    .then(|| {
                        hue_of(skin.def.corona.rim_cool).or_else(|| hue_of(skin.def.light.color))
                    })
                    .flatten();
                uniform.design_color = [r, g, b, hue.unwrap_or(-1.0)];
            }
        }
        uniforms
    }

    /// Whether the skin numbered `index` takes the colour of the light where it is
    /// (`ambient` in its file), so its instances want that light
    /// ([`crate::saber_persona::light_instances`]).
    pub(crate) fn takes_ambient(&self, index: SkinIndex) -> bool {
        self.skins
            .get(usize::from(index))
            .is_some_and(|skin| skin.shown().ambient.is_some())
    }

    /// The skins' sound sets in their numbers' order, for the audio adapter.
    pub(crate) fn sound_sets(&self) -> Vec<sjk_client::SaberSoundSet<'_>> {
        self.skins
            .iter()
            .map(|skin| {
                let sounds = &skin.def.sounds;
                sjk_client::SaberSoundSet {
                    on: &sounds.on,
                    off: &sounds.off,
                    hum: &sounds.hum,
                    swings: [&sounds.swings[0], &sounds.swings[1], &sounds.swings[2]],
                }
            })
            .collect()
    }

    /// The swing skin `index` plays for a stock swing cue's `variant`.
    /// Skin `index`'s sounds.
    pub(crate) fn sounds(&self, index: SkinIndex) -> Option<&crate::blade_skin_file::Sounds> {
        self.skins
            .get(usize::from(index))
            .map(|skin| &skin.def.sounds)
    }

    /// Every sound the skins' crackles name, for the audio to decode as they load.
    pub(crate) fn crackle_sounds(&self) -> Vec<&str> {
        self.skins
            .iter()
            .filter_map(|skin| skin.def.crackle.as_ref())
            .flat_map(|crackle| std::iter::once(&crackle.buzz).chain(&crackle.snaps))
            .filter(|path| !path.is_empty())
            .map(String::as_str)
            .collect()
    }

    /// Skin `index`'s crackle heard (`crackle` in its file); none without.
    pub(crate) fn crackle(
        &self,
        index: SkinIndex,
    ) -> Option<&crate::blade_skin_file::smoke::Crackle> {
        self.skins
            .get(usize::from(index))
            .and_then(|skin| skin.def.crackle.as_ref())
    }

    pub(crate) fn swing(&self, index: SkinIndex, variant: usize) -> Option<&str> {
        self.skins
            .get(usize::from(index))
            .map(|skin| skin.def.sounds.swings[variant % 3].as_str())
    }
}

/// Who wears which skin: one entry per client slot, and the local player's own choice
/// for the menus' preview (no client slot there).
#[derive(Clone, Debug, Default)]
pub(crate) struct SaberSkins {
    /// What each slot is drawn wearing: `others`, with the local player's own in its slot.
    clients: [Option<SkinColor>; MAX_CLIENTS],
    /// What each slot's second saber wears ([`Second`]), as `clients`.
    seconds: [Second; MAX_CLIENTS],
    /// What the hub's looks say each slot wears.
    others: [Option<SkinColor>; MAX_CLIENTS],
    /// What the hub's looks say each slot's second saber wears.
    other_seconds: [Second; MAX_CLIENTS],
    /// The local player's skin, also shown with no session (the Character preview).
    local: Option<SkinColor>,
    /// The local player's second saber's.
    local_second: Second,
    /// The local player's client slot, in a session.
    local_client: Option<usize>,
    /// Which skin's sound set each slot is heard with (its number): `sound_others`, with
    /// the local player's own in its slot. The blade's unless the player chose other
    /// saber sounds (`crate::saber_sound_choice`).
    sounds: [Option<SkinIndex>; MAX_CLIENTS],
    /// What the hub's looks say each slot is heard with.
    sound_others: [Option<SkinIndex>; MAX_CLIENTS],
    /// The local player's own sound set.
    sound_local: Option<SkinIndex>,
    /// Who sits in each slot (team, name), for the skins drawn from that.
    personas: [crate::saber_persona::Persona; MAX_CLIENTS],
    /// The local player's own, also with no session (its `name`).
    local_persona: crate::saber_persona::Persona,
    /// The looks' revision and the loaded skins' generation `others` was last taken at.
    taken_at: Option<(u64, u64)>,
    /// Whether every player's blade is the stock one this frame, in a duel
    /// ([`crate::duel_blades`]).
    plain: bool,
    /// Whose blades the player draws stock to save GPU time ([`crate::stock_blades`]).
    stock: crate::stock_blades::StockBlades,
    /// Blades the world shots add every frame, without a session (blade, unlock id or
    /// a stock colour, seed).
    #[cfg(test)]
    pub(crate) shot_blades: Vec<(crate::saber::Blade, ShotColor, u32)>,
    /// The time the world shots' blades are drawn at, instead of the frame's.
    #[cfg(test)]
    pub(crate) shot_seconds: Option<f64>,
    /// Afterimages the world shots add every frame (pose, unlock id, brightness).
    #[cfg(test)]
    pub(crate) shot_ghosts: Vec<(crate::saber::Blade, &'static str, f32)>,
    /// Who the world shots' skinned blades are worn by (team, name).
    #[cfg(test)]
    pub(crate) shot_persona: crate::saber_persona::Persona,
    /// Swings the world shots trail every frame: the poses a blade passed, oldest first,
    /// 8 ms apart, the last at the frame's time, and what it wears (`saber_trail.rs`).
    #[cfg(test)]
    pub(crate) shot_swings: Vec<(Vec<crate::saber::Blade>, ShotColor)>,
}

/// What a player's second saber wears: the first saber's skin (players with one skin, and
/// every look from before 21/10/2026), or its own (`None`: the stock blade).
// Kept by value in fixed per-slot arrays read every frame: boxing `Own` (clippy's advice)
// would allocate there, which the per-frame paths avoid.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Second {
    #[default]
    Same,
    Own(Option<SkinColor>),
}

/// What the local player holds that a second skin can go on ([`pair`]): nothing, two
/// sabers, or one saber with more than one blade (a staff's other half).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Pair {
    #[default]
    None,
    TwoSabers,
    Staff,
}

/// [`Pair`] as last seen on the local player's model (`menu_stage`), for the Collection.
static LOCAL_PAIR: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// What the local player's model held when last drawn.
pub(crate) fn local_pair() -> Pair {
    match LOCAL_PAIR.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Pair::TwoSabers,
        2 => Pair::Staff,
        _ => Pair::None,
    }
}

/// Where the Collection's lone staff draws the middle of its first half and of its other
/// half, in window pixels, each packed as two `f32` (`u64::MAX`: not drawn), so a click on
/// the staff goes to the half it lands on.
static STAFF_HALVES: [std::sync::atomic::AtomicU64; 2] = [
    std::sync::atomic::AtomicU64::new(u64::MAX),
    std::sync::atomic::AtomicU64::new(u64::MAX),
];

/// Keep where the lone staff's halves were drawn (`None`: no staff drawn whole).
pub(crate) fn set_staff_halves(halves: Option<[[f32; 2]; 2]>) {
    for (index, cell) in STAFF_HALVES.iter().enumerate() {
        let packed = halves.map_or(u64::MAX, |halves| {
            let [x, y] = halves[index];
            (u64::from(x.to_bits()) << 32) | u64::from(y.to_bits())
        });
        cell.store(packed, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Whether a click at `at` (window pixels) lands on the lone staff's other half (nearer its
/// middle than the first half's); `None` when no staff was drawn whole.
pub(crate) fn staff_half_at(at: [f32; 2]) -> Option<bool> {
    let [first, other] = STAFF_HALVES.each_ref().map(|cell| {
        let packed = cell.load(std::sync::atomic::Ordering::Relaxed);
        (packed != u64::MAX).then(|| {
            [
                f32::from_bits((packed >> 32) as u32),
                f32::from_bits(packed as u32),
            ]
        })
    });
    let distance = |point: [f32; 2]| (point[0] - at[0]).hypot(point[1] - at[1]);
    Some(distance(other?) < distance(first?))
}

/// Keep what the local player's model holds: on each hand, whether it holds a saber and
/// whether that saber has another half ([`crate::saber_hilts::other_half`]: a staff's
/// second blade, not a crossguard's side blades).
pub(crate) fn set_local_pair(held: [Option<bool>; 2]) {
    let pair = match held {
        [_, Some(_)] => 1,
        [Some(true), None] => 2,
        _ => 0,
    };
    LOCAL_PAIR.store(pair, std::sync::atomic::Ordering::Relaxed);
}

/// What the local player's two sabers (or a staff's halves) show on the Collection's
/// stage: what they wear (`worn`), with `shown` (the shader the page shows, `Some(None)` the
/// stock blade) on the sides being dressed (`dressed`: the first, the second; both with the
/// Collection's "Both" box ticked). A side not dressed keeps what it wears now, so the
/// pair looks as Equip will leave it.
pub(crate) fn previewed(
    worn: [Option<SkinColor>; 2],
    shown: Option<Option<SkinColor>>,
    dressed: [bool; 2],
) -> [Option<SkinColor>; 2] {
    match shown {
        None => worn,
        Some(shown) => [0, 1].map(|side| if dressed[side] { shown } else { worn[side] }),
    }
}

/// The skin a blade of saber `saber_index` wears, of `skins` (the first's and the
/// second's): the second saber wears the second, and with one saber, the blades of its other
/// half (`other_half`, [`crate::saber_hilts::other_half`]) do (a staff's, Sol's request of
/// 21/10/2026; a crossguard's side blades wear the first's with its main blade).
pub(crate) fn skin_of_blade(
    skins: [Option<SkinColor>; 2],
    two_sabers: bool,
    saber_index: usize,
    other_half: bool,
) -> Option<SkinColor> {
    if saber_index >= 1 || (!two_sabers && other_half) {
        skins[1]
    } else {
        skins[0]
    }
}

impl Second {
    /// The second saber's skin when the first wears `first`.
    pub(crate) fn resolve(self, first: Option<SkinColor>) -> Option<SkinColor> {
        match self {
            Self::Same => first,
            Self::Own(skin) => skin,
        }
    }
}

/// A world shot's blade colour: stock, or the loaded skin of an unlock id.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum ShotColor {
    Stock(BladeColor),
    Skin(&'static str),
    /// A skin on a saber of that stock colour (a chroma takes it).
    Worn(&'static str, BladeColor),
    /// A skin with these options switched from their default: parts off, extras on
    /// (`crate::saber_skin_options`).
    Parts(&'static str, crate::saber_skin_options::OptionsOff),
}

impl SaberSkins {
    /// Set what the hub says client slot `client` wears (`None` for the stock blade);
    /// out-of-range slots are ignored, and the local player's own slot keeps its skin.
    #[cfg(test)]
    pub(crate) fn set(&mut self, client: usize, skin: Option<SkinColor>) {
        if let Some(slot) = self.others.get_mut(client) {
            *slot = skin;
            self.sound_others[client] = skin.map(|skin| skin.index);
            self.merge();
        }
    }

    /// Take every other slot's skin from the looks (`skin_of` a client slot) when their
    /// `revision` or the loaded skins' `generation` is not the one last taken; no
    /// allocation, 32 lookups when it changed.
    /// Each slot is heard with its blade's sound set, and both sabers wear the first's.
    #[cfg(test)]
    pub(crate) fn follow_looks(
        &mut self,
        revision: u64,
        generation: u64,
        skin_of: impl Fn(usize) -> Option<SkinColor>,
    ) {
        self.follow_looks_each(revision, generation, |client| {
            let skin = skin_of(client);
            (skin, Second::Same, skin.map(|skin| skin.index))
        });
    }

    /// Take every other slot's skins and sound set from the looks when their `revision`
    /// or the loaded skins' `generation` is not the one last taken: `of` a client slot
    /// gives its first saber's skin, its second's, and its sound set (a loaded skin's
    /// number, `None` the stock sounds) taken apart from its blade. No allocation, 32
    /// lookups when it changed.
    pub(crate) fn follow_looks_each(
        &mut self,
        revision: u64,
        generation: u64,
        of: impl Fn(usize) -> (Option<SkinColor>, Second, Option<SkinIndex>),
    ) {
        if self.taken_at == Some((revision, generation)) {
            return;
        }
        self.taken_at = Some((revision, generation));
        for (client, ((skin, second), sound)) in self
            .others
            .iter_mut()
            .zip(self.other_seconds.iter_mut())
            .zip(self.sound_others.iter_mut())
            .enumerate()
        {
            (*skin, *second, *sound) = of(client);
        }
        self.merge();
    }

    /// Draw `others`, with the local player's own skins in its slot, and hear
    /// `sound_others` so.
    fn merge(&mut self) {
        self.clients = self.others;
        self.seconds = self.other_seconds;
        self.sounds = self.sound_others;
        if let Some(client) = self.local_client.filter(|client| *client < MAX_CLIENTS) {
            self.clients[client] = self.local;
            self.seconds[client] = self.local_second;
            self.sounds[client] = self.sound_local;
        }
    }

    /// The skin worn by the player whose body is entity `entity_id` (client + 1); none
    /// while their blade is drawn stock (a duel's [`Self::set_plain`], the player's
    /// [`Self::set_stock`]).
    pub(crate) fn get(&self, entity_id: u64) -> Option<SkinColor> {
        self.get_both(entity_id)[0]
    }

    /// The skins worn on the first and the second saber by the player whose body is entity
    /// `entity_id` (client + 1).
    pub(crate) fn get_both(&self, entity_id: u64) -> [Option<SkinColor>; 2] {
        let Some(client) = entity_id
            .checked_sub(1)
            .and_then(|client| usize::try_from(client).ok())
            .filter(|client| *client < MAX_CLIENTS)
        else {
            return [None, None];
        };
        let persona = if Some(client) == self.local_client {
            self.local_persona.in_team(self.personas[client].team)
        } else {
            self.personas[client]
        };
        // A duel's stock blades ([`Self::set_plain`]) and the player's own choice of
        // stock blades ([`Self::set_stock`]) are stock on both sabers.
        if self.plain || self.stock.plain(Some(client) == self.local_client) {
            return [None, None];
        }
        let wear = |skin: Option<SkinColor>| skin.map(|skin| SkinColor { persona, ..skin });
        let first = self.clients[client];
        [wear(first), wear(self.seconds[client].resolve(first))]
    }

    /// Draw every player's blade as the stock one in its own colour (in a duel,
    /// [`crate::duel_blades`]) or with the skin they wear.
    pub(crate) fn set_plain(&mut self, plain: bool) {
        self.plain = plain;
    }

    /// Draw the blades `stock` names as stock ones in their own colour, the others with
    /// their skin ([`crate::stock_blades`]). Only the world's blades: the menus' preview
    /// takes [`Self::local`], and the sounds are unchanged.
    pub(crate) fn set_stock(&mut self, stock: crate::stock_blades::StockBlades) {
        self.stock = stock;
    }

    /// Who sits in each client slot, and the local player's own name (`name`), for
    /// the skins drawn from them; taken twice a second ([`GpuState::update_looks`]).
    pub(crate) fn set_personas(
        &mut self,
        personas: [crate::saber_persona::Persona; MAX_CLIENTS],
        local: crate::saber_persona::Persona,
    ) {
        self.personas = personas;
        self.local_persona = local;
    }

    /// The local player's own persona, for the menus' preview.
    pub(crate) fn local_persona(&self) -> crate::saber_persona::Persona {
        self.local_persona
    }

    /// The colour entity `entity_id`'s blades are drawn with: its skin over `color`, a
    /// chroma in `color`'s hue.
    pub(crate) fn blade_color(&self, entity_id: u64, color: BladeColor) -> BladeColor {
        self.get(entity_id)
            .map_or(color, |skin| BladeColor::Skin(skin.worn_with(color)))
    }

    /// The local player's skins on its first and second saber (or a staff's halves), worn
    /// with its own persona.
    pub(crate) fn local_both(&self) -> [Option<SkinColor>; 2] {
        let first = self.local();
        [first, self.local_second_with(first)]
    }

    /// What the local player's second saber (or a staff's other half) wears when the first
    /// wears `first`: its own choice, else `first` (the Collection previews a shader on the
    /// first and shows the second as it is).
    pub(crate) fn local_second_with(&self, first: Option<SkinColor>) -> Option<SkinColor> {
        self.local_second.resolve(first).map(|skin| SkinColor {
            persona: self.local_persona,
            ..skin
        })
    }

    /// The local player's skin, worn with its own persona.
    pub(crate) fn local(&self) -> Option<SkinColor> {
        self.local.map(|skin| SkinColor {
            persona: self.local_persona,
            ..skin
        })
    }

    /// The local player wears `skin`, heard with its own sound set, and, in a session,
    /// sits in slot `client`.
    #[cfg(test)]
    pub(crate) fn set_local(&mut self, client: Option<usize>, skin: Option<SkinColor>) {
        self.set_local_each(client, skin, self.local_second, skin.map(|skin| skin.index));
    }

    /// The local player wears `skin`, and `second` on its second saber, heard with sound
    /// set `sound` (a loaded skin's number, `None` the stock sounds) whatever the blade,
    /// and, in a session, sits in slot `client`.
    pub(crate) fn set_local_each(
        &mut self,
        client: Option<usize>,
        skin: Option<SkinColor>,
        second: Second,
        sound: Option<SkinIndex>,
    ) {
        if (client, skin, second, sound)
            == (
                self.local_client,
                self.local,
                self.local_second,
                self.sound_local,
            )
        {
            return;
        }
        self.local = skin;
        self.local_second = second;
        self.sound_local = sound;
        self.local_client = client;
        self.merge();
    }

    /// Each client slot's sound set (a loaded skin's number) for the audio adapter:
    /// the blade's, or the one the player chose apart from it.
    pub(crate) fn sound_sets(&self) -> [Option<SkinIndex>; MAX_CLIENTS] {
        if self.plain {
            return [None; MAX_CLIENTS];
        }
        self.sounds
    }

    /// The sound set the player whose body is entity `entity_id` (client + 1) is heard
    /// with, for the animation's swing cues.
    pub(crate) fn sound_of(&self, entity_id: u64) -> Option<SkinIndex> {
        let client = usize::try_from(entity_id.checked_sub(1)?).ok()?;
        if self.plain {
            return None;
        }
        self.sounds.get(client).copied().flatten()
    }
}

impl GpuState {
    /// The skin the local player wears: `cg_saberSkin` only while the own hub profile
    /// lists that unlock (`Looks::own_saber_skin`, read twice a second) and its pack is
    /// loaded; with no identity, no hub, the unlock missing or no pack, the stock blade.
    pub(crate) fn local_saber_skin(&self) -> Option<SkinColor> {
        let own = self.looks.own();
        own.saber_skin
            .and_then(|id| self.blade_skins.color_with(id, own.options_off))
    }

    /// Once a frame, before the session's sabers are submitted: the loaded skins when a
    /// pack brought new ones (their parameters and pairs uploaded, their sounds
    /// registered again), every other client's entry from the hub's looks when they or
    /// the skins changed, the local player's from [`Self::local_saber_skin`] in its own
    /// slot (the game state's: following someone, the snapshot's player state is theirs,
    /// and they wear their own look), and every client's sound override to the audio
    /// adapter. No allocation unless skins were loaded: a few compares, and 32-entry
    /// copies on a change.
    pub(crate) fn sync_saber_skins(
        &mut self,
        game_audio: &mut Option<GameAudio>,
        presentation_time: i64,
    ) {
        // `cg_saberLabDesigns` changes what the loaded skins draw: upload them again.
        let designs = crate::blade_lab_designs::set_enabled(self.console.as_ref());
        if designs || crate::sjk_packs::generation() != self.blade_skins.generation() {
            let skins = crate::sjk_packs::skins();
            // An edit of one skin on the table drawn: that skin alone goes up again
            // (all of them when the designs were switched).
            let only = skins
                .edit()
                .filter(|(from, _)| !designs && *from == self.blade_skins.generation())
                .map(|(_, index)| index);
            self.blade_skins = skins;
            self.saber_gpu
                .upload_skins(&self.queue, &self.blade_skins, only);
        }
        let client = self
            .live_session
            .as_ref()
            .map(|session| (session.game_state(), session.latest_snapshot()))
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(|session| (session.game_state(), session.latest_snapshot()))
            })
            .and_then(|(game_state, snapshot)| {
                crate::looks::ViewSlots::of(game_state, &snapshot.player).own
            });
        let duel = self
            .live_session
            .as_ref()
            .map(|session| (session.game_state(), session.latest_snapshot()))
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(|session| (session.game_state(), session.latest_snapshot()))
            });
        self.saber_skins.set_plain(crate::duel_blades::plain(
            self.console.as_ref(),
            duel.map(|(game, _)| game),
            duel.map(|(_, snapshot)| &snapshot.player),
        ));
        let stock = crate::stock_blades::StockBlades::read(self.console.as_ref());
        self.saber_skins.set_stock(stock);
        self.admit_smoke_shaders();
        let (looks, skins) = (&self.looks, &self.blade_skins);
        let second = |second: crate::looks::SecondSkin, off| match second {
            crate::looks::SecondSkin::Same => Second::Same,
            crate::looks::SecondSkin::Own(id) => {
                Second::Own(id.and_then(|id| skins.color_with(id, off)))
            }
        };
        self.saber_skins
            .follow_looks_each(looks.revision(), skins.generation(), |client| {
                let skin = looks
                    .saber_skin_id(client)
                    .and_then(|id| skins.color_with(id, looks.options_off(client)));
                let sound = looks
                    .sound_skin_id(client)
                    .and_then(|id| skins.index_of(id));
                (
                    skin,
                    second(looks.second_skin(client), looks.options_off2(client)),
                    sound,
                )
            });
        let skin = self.local_saber_skin();
        let own = self.looks.own();
        let own_second = second(own.second, own.options_off2);
        let sound = own
            .sound_skin()
            .and_then(|id| self.blade_skins.index_of(id));
        self.saber_skins
            .set_local_each(client, skin, own_second, sound);
        if let Some(preview) = crate::saber_sound_choice::take_preview() {
            self.play_sound_preview(game_audio, preview);
        }
        if let Some(audio) = game_audio {
            audio.follow_blade_skins(&self.blade_skins);
            audio.set_saber_sound_overrides(&self.saber_skins.sound_sets());
        }
        #[cfg(test)]
        self.submit_shot_blades(presentation_time);
        #[cfg(not(test))]
        let _ = presentation_time;
    }

    /// The world shots' blades, lit, at their chosen time; a skin whose pack is not
    /// loaded is the stock blue.
    #[cfg(test)]
    fn submit_shot_blades(&mut self, presentation_time: i64) {
        let millis = self
            .saber_skins
            .shot_seconds
            .map_or(presentation_time, |seconds| (seconds * 1_000.0) as i64);
        for (blade, color, seed) in self.saber_skins.shot_blades.clone() {
            let persona = self.saber_skins.shot_persona;
            let color = match color {
                ShotColor::Stock(color) => color,
                ShotColor::Skin(id) => self
                    .blade_skins
                    .color_of(id)
                    .map_or(BladeColor::Retail(crate::saber::Color::Blue), |skin| {
                        BladeColor::Skin(SkinColor { persona, ..skin })
                    }),
                ShotColor::Worn(id, stock) => self.blade_skins.color_of(id).map_or(stock, |skin| {
                    BladeColor::Skin(SkinColor { persona, ..skin }.worn_with(stock))
                }),
                ShotColor::Parts(id, off) => self
                    .blade_skins
                    .color_with(id, off)
                    .map_or(BladeColor::Retail(crate::saber::Color::Blue), |skin| {
                        BladeColor::Skin(SkinColor { persona, ..skin })
                    }),
            };
            self.saber_instances.extend(
                crate::saber::Instance::pair(blade, color)
                    .map(|i| i.with_animation(millis as f64 * 0.001, seed)),
            );
            self.saber_instances.extend(crate::saber::skin_extras(
                blade,
                color,
                millis as f64 * 0.001,
                seed,
            ));
            let mut lit = [None; 8];
            lit[0] = Some(blade);
            crate::saber_submission::lights::append(
                &mut self.dynamic_lights,
                &lit,
                color,
                1,
                false,
                millis,
                u64::from(seed),
                seed,
                0.0,
            );
        }
        // Each swing's trail, laid again every frame ending at the frame's time.
        if !self.saber_skins.shot_swings.is_empty() {
            self.saber_trail_segments = Default::default();
        }
        for (poses, color) in self.saber_skins.shot_swings.clone() {
            let color = match color {
                ShotColor::Skin(id) => self.blade_skins.color_of(id),
                ShotColor::Parts(id, off) => self.blade_skins.color_with(id, off),
                _ => None,
            }
            .map_or(
                BladeColor::Retail(crate::saber::Color::Blue),
                BladeColor::Skin,
            );
            let mut trail = crate::saber_trail::Trail::default();
            let last = poses.len() as i64 - 1;
            for (index, blade) in poses.iter().enumerate() {
                let at = presentation_time - (last - index as i64) * 8;
                let base = glam::Vec3::from_array(blade.base);
                let tip = base + glam::Vec3::from_array(blade.direction) * blade.length;
                let edge = crate::saber_trail::Edge {
                    base: blade.base,
                    tip: tip.to_array(),
                };
                if let Some(quad) = trail.update(edge, at, 200, 0, color, true) {
                    self.saber_trail_segments.insert(quad, at);
                }
            }
        }
        let persona = self.saber_skins.shot_persona;
        for (blade, id, fade) in self.saber_skins.shot_ghosts.clone() {
            if let Some(skin) = self.blade_skins.color_of(id) {
                let color = BladeColor::Skin(SkinColor { persona, ..skin });
                self.saber_instances.push(
                    crate::saber::Instance::ghost(blade, color, fade)
                        .with_animation(millis as f64 * 0.001, 2),
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "saber_skins_tests.rs"]
pub(crate) mod tests;
