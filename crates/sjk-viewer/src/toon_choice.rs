//! Toon models by choice (Sol's request, 08/11/2026): each player chooses toon shading
//! for their own model ([`CVAR`], off by default, on the Profile's Character tab and in
//! Settings), as some models look better with it and some without. The choice travels in
//! the player's hub look as toy [`TOY`] (`docs/identity.md`, "Unlocks and looks"), so
//! every SJK player draws that model the way its wearer chose. `r_toonModels` (on by
//! default) says whether this PC shows the choices at all: off, every model keeps stock
//! shading.
//!
//! A model drawn toon carries [`FLAG`] in its instance's `view_flags`, which
//! `stage_runtime.wgsl` turns into the toon bands: a player's body with its hats, capes
//! and what it holds (`actor_world_submission.rs`), and the menu's model of the local
//! player (`menu_stage`). Other models (NPCs, items, the view weapon) keep stock shading.

use crate::looks::Looks;
use crate::saber_skins::MAX_CLIENTS;

/// The own choice: the local player's model in toon shading, for everyone who shows it.
pub(crate) const CVAR: &str = "cg_toonModel";
/// The look's toy id (the hub knows no toys: any id of `a` to `z` and `_` goes).
pub(crate) const TOY: &str = "toon_model";
/// The instance `view_flags` bit of a toon model (`stage_runtime.wgsl`, `entity_vertex`).
pub(crate) const FLAG: u32 = 1 << 28;

const _: () = assert!(MAX_CLIENTS <= u32::BITS as usize);

/// The client slots, a bit each, whose players chose toon shading for their model.
pub(crate) fn slots(looks: &Looks) -> u32 {
    #[cfg(test)]
    if looks.shot_toon {
        return u32::MAX;
    }
    (0..MAX_CLIENTS)
        .filter(|slot| looks.toy(*slot, TOY))
        .fold(0, |bits, slot| bits | 1 << slot)
}

/// [`FLAG`] for entity `number` when it is a player (its number is its slot) in `slots`.
pub(crate) fn flag(slots: u32, number: u16) -> u32 {
    if usize::from(number) < MAX_CLIENTS && slots & (1 << number) != 0 {
        FLAG
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_players_who_chose_it_are_flagged() {
        let slots = 1 << 3 | 1 << 31;
        assert_eq!(flag(slots, 3), FLAG);
        assert_eq!(flag(slots, 31), FLAG);
        assert_eq!(flag(slots, 4), 0);
        // Entity 35 is no player, whatever the bits say.
        assert_eq!(flag(u32::MAX, 35), 0);
    }

    #[test]
    fn the_flag_is_free_in_the_instance_bits() {
        // Bits 0..3 views, 8..23 the animation index, 24..27 the render effects.
        for taken in [
            crate::disintegration::RF_DISINTEGRATE1,
            crate::disintegration::RF_DISINTEGRATE2,
            crate::demp2_shell::RF_VOLUMETRIC,
            crate::body_fade::DISPLAY_FILTER_ADD,
            crate::actor_instance::ActorInstance::WORLD,
            0xffff << 8 | 15,
        ] {
            assert_eq!(taken & FLAG, 0);
        }
        let shader = include_str!("stage_runtime.wgsl");
        assert!(shader.contains("(instance.view_flags & (1u << 28u))"));
        assert_eq!(FLAG, 1 << 28);
    }

    #[test]
    fn the_look_carries_the_choice() {
        assert!(crate::looks::TOYS.contains(&TOY));
    }
}
