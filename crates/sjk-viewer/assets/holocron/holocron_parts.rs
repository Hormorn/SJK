// The loot-box holocrons' and kyber crystals' parts and how they move (SJK; see
// README.md), included by crates/sjk-viewer/src/holocrons/stage.rs. Made by
// scripts/holocron_assets.py from holocron_forms.py's PARTS: edit that, not this.

/// Every form the stage draws, by the hub's tier id: the four holocrons in `TIERS`
/// order, then the four kyber crystals; each its light and its parts, the body first.
#[rustfmt::skip]
pub(crate) const FORMS: [Form; 8] = [
    Form { id: "uncommon", light: [0.55, 1.5, 0.6], parts: &[
        Part { model: "models/sjk/holocron_uncommon.md3", locked: "models/sjk/holocron_uncommon_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
    ] },
    Form { id: "rare", light: [0.5, 1.0, 1.75], parts: &[
        Part { model: "models/sjk/holocron_rare.md3", locked: "models/sjk/holocron_rare_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_rare_halo.md3", locked: "models/sjk/holocron_rare_halo_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.25, 1.3], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.3], tumble: [0.55, 0.0], precess: [0.0, 0.25] },
    ] },
    Form { id: "legendary", light: [1.25, 0.45, 1.75], parts: &[
        Part { model: "models/sjk/holocron_legendary.md3", locked: "models/sjk/holocron_legendary_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_legendary_cap.md3", locked: "models/sjk/holocron_legendary_cap_locked.md3", at: [0.0, 0.0, 3.2], bob: [0.35, 1.5], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.35], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_legendary_eye.md3", locked: "models/sjk/holocron_legendary_eye_locked.md3", at: [0.0, 0.0, 1.25], bob: [0.1, 1.5], orbit: [0.0, 0.0, 0.0], spin: [0.0, -1.2], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
    ] },
    Form { id: "mythical", light: [1.85, 1.3, 0.4], parts: &[
        Part { model: "models/sjk/holocron_mythical.md3", locked: "models/sjk/holocron_mythical_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -3.75], bob: [0.0, 0.0], orbit: [4.15, -0.562, 0.0], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -0.675], bob: [0.0, 0.0], orbit: [3.955, 0.716, 0.79], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 2.4], bob: [0.0, 0.0], orbit: [3.329, -0.63, 1.58], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -2.025], bob: [0.0, 0.0], orbit: [4.046, 0.649, 2.37], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 1.05], bob: [0.0, 0.0], orbit: [4.409, -0.698, 3.16], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -3.375], bob: [0.0, 0.0], orbit: [3.394, 0.581, 3.95], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -0.3], bob: [0.0, 0.0], orbit: [3.707, -0.735, 4.74], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 2.775], bob: [0.0, 0.0], orbit: [4.16, 0.611, 5.53], spin: [0.0, 2.2], tumble: [0.35, 0.0], precess: [0.0, 0.0] },
    ] },
    Form { id: "kyber_uncommon", light: [0.55, 1.5, 0.6], parts: &[
        Part { model: "models/sjk/holocron_kyber_uncommon.md3", locked: "models/sjk/holocron_kyber_uncommon_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_uncommon_crystal.md3", locked: "models/sjk/holocron_kyber_uncommon_crystal_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.5], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_uncommon_ring_a.md3", locked: "models/sjk/holocron_kyber_uncommon_ring_a_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [std::f32::consts::FRAC_PI_2, 0.3], precess: [0.0, 0.25] },
        Part { model: "models/sjk/holocron_kyber_uncommon_mote.md3", locked: "models/sjk/holocron_kyber_uncommon_mote_locked.md3", at: [0.0, 0.0, -3.5], bob: [0.0, 0.0], orbit: [3.5, 0.6, 0.0], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_uncommon_mote.md3", locked: "models/sjk/holocron_kyber_uncommon_mote_locked.md3", at: [0.0, 0.0, -0.91], bob: [0.0, 0.0], orbit: [3.837, 0.6, 0.63], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_uncommon_mote.md3", locked: "models/sjk/holocron_kyber_uncommon_mote_locked.md3", at: [0.0, 0.0, 1.68], bob: [0.0, 0.0], orbit: [3.864, 0.6, 1.26], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_uncommon_mote.md3", locked: "models/sjk/holocron_kyber_uncommon_mote_locked.md3", at: [0.0, 0.0, -2.73], bob: [0.0, 0.0], orbit: [3.556, 0.6, 1.89], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
    ] },
    Form { id: "kyber_rare", light: [0.5, 1.0, 1.75], parts: &[
        Part { model: "models/sjk/holocron_kyber_rare.md3", locked: "models/sjk/holocron_kyber_rare_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_crystal.md3", locked: "models/sjk/holocron_kyber_rare_crystal_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.5], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_ring_a.md3", locked: "models/sjk/holocron_kyber_rare_ring_a_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [std::f32::consts::FRAC_PI_2, 0.3], precess: [0.0, 0.25] },
        Part { model: "models/sjk/holocron_kyber_rare_mote.md3", locked: "models/sjk/holocron_kyber_rare_mote_locked.md3", at: [0.0, 0.0, -3.5], bob: [0.0, 0.0], orbit: [3.5, 0.6, 0.0], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_mote.md3", locked: "models/sjk/holocron_kyber_rare_mote_locked.md3", at: [0.0, 0.0, -0.91], bob: [0.0, 0.0], orbit: [3.837, 0.6, 0.63], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_mote.md3", locked: "models/sjk/holocron_kyber_rare_mote_locked.md3", at: [0.0, 0.0, 1.68], bob: [0.0, 0.0], orbit: [3.864, 0.6, 1.26], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_mote.md3", locked: "models/sjk/holocron_kyber_rare_mote_locked.md3", at: [0.0, 0.0, -2.73], bob: [0.0, 0.0], orbit: [3.556, 0.6, 1.89], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_mote.md3", locked: "models/sjk/holocron_kyber_rare_mote_locked.md3", at: [0.0, 0.0, -0.14], bob: [0.0, 0.0], orbit: [3.197, 0.6, 2.52], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_rare_mote.md3", locked: "models/sjk/holocron_kyber_rare_mote_locked.md3", at: [0.0, 0.0, 2.45], bob: [0.0, 0.0], orbit: [3.116, 0.6, 3.15], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
    ] },
    Form { id: "kyber_legendary", light: [1.25, 0.45, 1.75], parts: &[
        Part { model: "models/sjk/holocron_kyber_legendary.md3", locked: "models/sjk/holocron_kyber_legendary_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_crystal.md3", locked: "models/sjk/holocron_kyber_legendary_crystal_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.5], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_ring_a.md3", locked: "models/sjk/holocron_kyber_legendary_ring_a_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [std::f32::consts::FRAC_PI_2, 0.3], precess: [0.0, 0.25] },
        Part { model: "models/sjk/holocron_kyber_legendary_ring_b.md3", locked: "models/sjk/holocron_kyber_legendary_ring_b_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [1.0, -0.22], precess: [0.0, -0.15] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, -3.5], bob: [0.0, 0.0], orbit: [3.5, 0.6, 0.0], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, -0.91], bob: [0.0, 0.0], orbit: [3.837, 0.6, 0.63], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, 1.68], bob: [0.0, 0.0], orbit: [3.864, 0.6, 1.26], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, -2.73], bob: [0.0, 0.0], orbit: [3.556, 0.6, 1.89], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, -0.14], bob: [0.0, 0.0], orbit: [3.197, 0.6, 2.52], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, 2.45], bob: [0.0, 0.0], orbit: [3.116, 0.6, 3.15], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, -1.96], bob: [0.0, 0.0], orbit: [3.388, 0.6, 3.78], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_legendary_mote.md3", locked: "models/sjk/holocron_kyber_legendary_mote_locked.md3", at: [0.0, 0.0, 0.63], bob: [0.0, 0.0], orbit: [3.763, 0.6, 4.41], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
    ] },
    Form { id: "kyber_mythical", light: [1.85, 1.3, 0.4], parts: &[
        Part { model: "models/sjk/holocron_kyber_mythical.md3", locked: "models/sjk/holocron_kyber_mythical_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_crystal.md3", locked: "models/sjk/holocron_kyber_mythical_crystal_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.5], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_ring_a.md3", locked: "models/sjk/holocron_kyber_mythical_ring_a_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [std::f32::consts::FRAC_PI_2, 0.3], precess: [0.0, 0.25] },
        Part { model: "models/sjk/holocron_kyber_mythical_ring_b.md3", locked: "models/sjk/holocron_kyber_mythical_ring_b_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [1.0, -0.22], precess: [0.0, -0.15] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, -3.5], bob: [0.0, 0.0], orbit: [3.5, 0.6, 0.0], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, -0.91], bob: [0.0, 0.0], orbit: [3.837, 0.6, 0.63], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, 1.68], bob: [0.0, 0.0], orbit: [3.864, 0.6, 1.26], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, -2.73], bob: [0.0, 0.0], orbit: [3.556, 0.6, 1.89], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, -0.14], bob: [0.0, 0.0], orbit: [3.197, 0.6, 2.52], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, 2.45], bob: [0.0, 0.0], orbit: [3.116, 0.6, 3.15], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, -1.96], bob: [0.0, 0.0], orbit: [3.388, 0.6, 3.78], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, 0.63], bob: [0.0, 0.0], orbit: [3.763, 0.6, 4.41], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, 3.22], bob: [0.0, 0.0], orbit: [3.896, 0.6, 5.04], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_kyber_mythical_mote.md3", locked: "models/sjk/holocron_kyber_mythical_mote_locked.md3", at: [0.0, 0.0, -1.19], bob: [0.0, 0.0], orbit: [3.665, 0.6, 5.67], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
    ] },
];
