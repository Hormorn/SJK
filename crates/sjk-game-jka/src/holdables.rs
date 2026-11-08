//! Holdable items: which one is used and whether it may be (OpenJK `codemp/game`
//! `bg_misc.c` `BG_IsItemSelectable`, `BG_CycleInven`; `bg_pmove.c` `PM_ItemUsable`;
//! `g_cmds.c` `G_ItemUsable`), and what using one does to its user (`g_items.c`
//! `ItemUse_MedPack`, `_MedPack_Big`, `_Binoculars`, `_Seeker`, `_Jetpack`, `_UseCloak`;
//! `NPC_AI_Jedi.c` `Jedi_Cloak`/`Jedi_Decloak`; `g_main.c`'s jetpack and cloak batteries).
//!
//! The use key's branch of `PM_Weapon` is [`crate::pmove_holdable`]; the seeker drone's
//! think is [`crate::seeker_drone`]. The shield, the sentry and the E-Web place entities
//! of their own; those deployable entities are not implemented yet.

use crate::items::{ITEMS, Kind};
use crate::pmove::{MovementCollision, MovementTrace};
use crate::{PM_JETPACK, PM_NORMAL};

/// `holdable_t`.
pub const HI_NONE: i32 = 0;
pub const HI_SEEKER: i32 = 1;
pub const HI_SHIELD: i32 = 2;
pub const HI_MEDPAC: i32 = 3;
pub const HI_MEDPAC_BIG: i32 = 4;
pub const HI_BINOCULARS: i32 = 5;
pub const HI_SENTRY_GUN: i32 = 6;
pub const HI_JETPACK: i32 = 7;
pub const HI_HEALTHDISP: i32 = 8;
pub const HI_AMMODISP: i32 = 9;
pub const HI_EWEB: i32 = 10;
pub const HI_CLOAK: i32 = 11;
pub const HI_NUM_HOLDABLE: i32 = 12;
/// `EV_USE_ITEM0`: `EV_USE_ITEM0 + tag` uses the item of that tag.
pub const EV_USE_ITEM0: u16 = 45;
/// `EV_ITEMUSEFAIL`, with an [`ItemUseFail`] as its parameter.
pub const EV_ITEMUSEFAIL: u16 = 61;
/// `PMF_USE_ITEM_HELD`: the use key has not been let go since it last did something.
pub const PMF_USE_ITEM_HELD: u16 = 1_024;
/// `EF_SEEKERDRONE`, `EF_DEAD`.
pub const EF_SEEKERDRONE: u32 = 0x0020_0000;
pub const EF_DEAD: u32 = 0x0000_0002;
/// `EF_JETPACK_ACTIVE`, `EF_JETPACK_FLAMING`.
pub const EF_JETPACK_ACTIVE: u32 = 1 << 11;
pub const EF_JETPACK_FLAMING: u32 = 1 << 30;
/// `MAX_MEDPACK_HEAL_AMOUNT`, `MAX_MEDPACK_BIG_HEAL_AMOUNT`.
pub const MEDPACK_HEAL: i32 = 25;
pub const MEDPACK_BIG_HEAL: i32 = 50;
/// `JETPACK_TOGGLE_TIME`, `CLOAK_TOGGLE_TIME`.
pub const TOGGLE_TIME: i32 = 1_000;
/// `JETPACK_DEFUEL_RATE`/`CLOAK_DEFUEL_RATE`, `JETPACK_REFUEL_RATE`/`CLOAK_REFUEL_RATE`.
pub const DEFUEL_RATE: i32 = 200;
pub const REFUEL_RATE: i32 = 150;
/// `Q3_INFINITE`: a powerup that lasts until taken off.
pub const Q3_INFINITE: i32 = 16_777_216;
/// `MASK_PLAYERSOLID`, `MASK_SHOT`, `MASK_SOLID`.
const MASK_PLAYERSOLID: u32 = 0x1111;
const MASK_SHOT: u32 = 0x1301;
const MASK_SOLID: u32 = 0x1001;

/// `itemUseFail_t`: why an item was refused, which the user is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemUseFail {
    SentryNoRoom = 1,
    SentryAlreadyPlaced = 2,
    ShieldNoRoom = 3,
    SeekerAlreadyDeployed = 4,
}

/// `bg_itemlist[index].giTag` of a holdable (0 for any other row).
pub fn tag_of(index: u32) -> i32 {
    ITEMS
        .get(index as usize)
        .filter(|item| item.kind == Kind::Holdable)
        .map_or(0, |item| item.tag)
}

/// `BG_GetItemIndexByTag(tag, IT_HOLDABLE)`: the item list's row of the holdable `tag`
/// (0 for none).
pub fn index_of(tag: i32) -> u32 {
    ITEMS
        .iter()
        .position(|item| item.kind == Kind::Holdable && item.tag == tag)
        .map_or(0, |index| index as u32)
}

/// `BG_IsItemSelectable`: the dispensers and the jetpack are never the selected item.
pub fn selectable(tag: i32) -> bool {
    !matches!(tag, HI_HEALTHDISP | HI_AMMODISP | HI_JETPACK)
}

/// Whether using the item leaves it with its user (`PM_Weapon`'s list): binoculars,
/// jetpack, dispensers, cloak, E-Web.
pub fn kept_after_use(tag: i32) -> bool {
    matches!(
        tag,
        HI_BINOCULARS | HI_JETPACK | HI_HEALTHDISP | HI_AMMODISP | HI_CLOAK | HI_EWEB
    )
}

/// `BG_CycleInven(ps, direction)`: the next selectable item held after the selected one
/// (1) or before it (-1), going round once; the selection unchanged when none is.
pub fn cycle(items: u32, selected: u32, direction: i32) -> u32 {
    let original = tag_of(selected);
    let mut tag = original;
    let step = |tag: i32| -> i32 {
        let next = tag + direction;
        if next <= 0 {
            HI_NUM_HOLDABLE - 1
        } else if next >= HI_NUM_HOLDABLE {
            1
        } else {
            next
        }
    };
    tag = step(tag);
    // "if hit nothing then select nothing": the reference keeps the selection.
    for _ in 0..32 {
        if tag == original {
            break;
        }
        if items & (1 << tag) != 0 && selectable(tag) {
            return index_of(tag);
        }
        tag = step(tag);
    }
    selected
}

/// What `PM_ItemUsable` and `G_ItemUsable` read of the user.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Holder {
    pub client: u16,
    pub origin: [f32; 3],
    pub view_angles: [f32; 3],
    /// `stats[STAT_HEALTH]`, `stats[STAT_MAX_HEALTH]`.
    pub health: i32,
    pub max_health: i32,
    pub entity_flags: u32,
    pub movement_flags: u16,
    /// `m_iVehicleNum != 0`.
    pub riding: bool,
    pub duel_in_progress: bool,
    /// `fd.sentryDeployed`.
    pub sentry_deployed: bool,
}

/// `PM_ItemUsable(ps, tag)` (`in_pmove`) or `G_ItemUsable(ps, tag)`: `Ok` when the item
/// may be used; `Err(Some(fail))` when it is refused with an event, `Err(None)` quietly.
/// `trace` is the movement's or the server's, passing through the user.
pub fn usable(
    holder: &Holder,
    tag: i32,
    in_pmove: bool,
    world: &dyn MovementCollision,
) -> Result<(), Option<ItemUseFail>> {
    // `G_ItemUsable`'s own first gate: "dead players shouldn't use items".
    if !in_pmove && holder.health <= 0 {
        return Err(None);
    }
    if holder.riding || holder.movement_flags & PMF_USE_ITEM_HELD != 0 {
        return Err(None);
    }
    // Only the movement's refuses a private duel.
    if in_pmove && holder.duel_in_progress {
        return Err(None);
    }
    if !selectable(tag) {
        return Err(None);
    }
    let trace = |start: [f32; 3],
                 mins: [f32; 3],
                 maxs: [f32; 3],
                 end: [f32; 3],
                 mask: u32|
     -> MovementTrace { world.trace(start, mins, maxs, end, mask) };
    match tag {
        HI_MEDPAC | HI_MEDPAC_BIG => {
            let dead = holder.health <= 0 || (in_pmove && holder.entity_flags & EF_DEAD != 0);
            if holder.health >= holder.max_health || dead {
                Err(None)
            } else {
                Ok(())
            }
        }
        HI_SEEKER if holder.entity_flags & EF_SEEKERDRONE != 0 => {
            Err(Some(ItemUseFail::SeekerAlreadyDeployed))
        }
        HI_SENTRY_GUN => {
            if holder.sentry_deployed {
                return Err(Some(ItemUseFail::SentryAlreadyPlaced));
            }
            let forward = crate::pmove::flight::flight_axes([0.0, holder.view_angles[1], 0.0])
                .0
                .to_array();
            let ahead: [f32; 3] =
                std::array::from_fn(|axis| holder.origin[axis] + forward[axis] * 64.0);
            let test: [f32; 3] = std::array::from_fn(|axis| ahead[axis] + forward[axis] * 16.0);
            let hit = trace(
                holder.origin,
                [-8.0, -8.0, 0.0],
                [8.0, 8.0, 24.0],
                test,
                MASK_PLAYERSOLID,
            );
            if (hit.fraction != 1.0 && hit.entity_number != holder.client)
                || hit.start_solid
                || hit.all_solid
            {
                Err(Some(ItemUseFail::SentryNoRoom))
            } else {
                Ok(())
            }
        }
        HI_SHIELD => {
            let (mins, maxs) = ([-8.0, -8.0, 0.0], [8.0, 8.0, 8.0]);
            let mut forward = crate::pmove::flight::flight_axes(holder.view_angles)
                .0
                .to_array();
            forward[2] = 0.0;
            let dest: [f32; 3] =
                std::array::from_fn(|axis| holder.origin[axis] + 64.0 * forward[axis]);
            let hit = trace(holder.origin, mins, maxs, dest, MASK_SHOT);
            if hit.fraction > 0.9 && !hit.start_solid && !hit.all_solid {
                let pos = hit.end_position;
                let down = trace(
                    pos,
                    mins,
                    maxs,
                    [pos[0], pos[1], pos[2] - 4096.0],
                    MASK_SOLID,
                );
                if !down.start_solid && !down.all_solid {
                    return Ok(());
                }
            }
            Err(Some(ItemUseFail::ShieldNoRoom))
        }
        _ => Ok(()),
    }
}

/// `MedPackGive(ent, amount)`: health up to the maximum, for a living user below it.
pub fn medpack_give(health: i32, max_health: i32, dead: bool, amount: i32) -> i32 {
    if health <= 0 || dead || health >= max_health {
        return health;
    }
    (health + amount).min(max_health)
}

/// `ItemUse_Binoculars`: with the weapon ready, the binoculars' zoom on (`zoomMode` 2,
/// unlocked, 40 degrees) or off (the time noted). Returns the new `zoomMode`,
/// `zoomLocked`, `zoomFov` and `zoomTime` as `(mode, Some((locked, fov)), time)`.
pub fn binoculars(zoom_mode: u8, weapon_ready: bool, level_time: i32) -> Option<Zoom> {
    if !weapon_ready {
        return None;
    }
    match zoom_mode {
        0 => Some(Zoom::On),
        2 => Some(Zoom::Off(level_time)),
        _ => None,
    }
}

/// What [`binoculars`] does to the zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom {
    /// `zoomMode` 2, `zoomLocked` off, `zoomFov` 40.
    On,
    /// `zoomMode` 0 and `zoomTime` this.
    Off(i32),
}

/// A user's jetpack and cloak, and their batteries: `jetPackOn`, `jetPackToggleTime`,
/// `jetPackDebReduce`, `jetPackDebRecharge`, `cloakToggleTime`, `cloakDebReduce`,
/// `cloakDebRecharge` of the client (the fuel itself is on the player state).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Packs {
    pub jetpack_on: bool,
    pub jetpack_toggle_time: i32,
    pub jetpack_reduce_at: i32,
    pub jetpack_recharge_at: i32,
    pub cloak_toggle_time: i32,
    pub cloak_reduce_at: i32,
    pub cloak_recharge_at: i32,
}

/// What a player's holdables keep on the server beyond its player state: the packs, and
/// the seeker drone's two clocks (`droneExistTime`, `droneFireTime`, which are not sent).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gear {
    pub packs: Packs,
    pub drone_exist: f32,
    pub drone_fire: f32,
}

/// What a toggle did, for the server to sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    Nothing,
    /// `sound/boba/JETON` on `CHAN_AUTO`.
    JetpackOn,
    JetpackOff,
    /// `sound/chars/shadowtrooper/cloak.wav` on `CHAN_ITEM`, `FL_NOTARGET` set.
    Cloaked,
    /// `sound/chars/shadowtrooper/decloak.wav`, `FL_NOTARGET` cleared.
    Decloaked,
}

/// The user as the jetpack and cloak read it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wearer {
    pub health: i32,
    pub dead: bool,
    pub jetpack_fuel: i32,
    pub cloak_fuel: i32,
    /// `powerups[PW_CLOAKED]`.
    pub cloaked: bool,
    /// `fd.forceGripBeingGripped >= level.time`.
    pub gripped: bool,
    pub falling_to_death: bool,
}

/// `ItemUse_Jetpack`: on or off, a second apart; never while dead, never switched on with
/// less than 5 fuel, nor while gripped or falling to death (`Jetpack_On`).
pub fn use_jetpack(packs: &mut Packs, wearer: &Wearer, level_time: i32) -> Toggled {
    if packs.jetpack_toggle_time >= level_time || wearer.health <= 0 || wearer.dead {
        return Toggled::Nothing;
    }
    if !packs.jetpack_on && wearer.jetpack_fuel < 5 {
        return Toggled::Nothing;
    }
    let toggled = if packs.jetpack_on {
        packs.jetpack_on = false;
        Toggled::JetpackOff
    } else if wearer.gripped || wearer.falling_to_death {
        Toggled::Nothing
    } else {
        packs.jetpack_on = true;
        Toggled::JetpackOn
    };
    packs.jetpack_toggle_time = level_time + TOGGLE_TIME;
    toggled
}

/// `TryUse`'s `tryJetPack` (`g_utils.c:2040-2048`), where the use key reaches when it
/// used nothing else, or straight away while the jetpack is on: whether to toggle a
/// `held` jetpack (`ItemUse_Jetpack`), as it does while `on` or while `airborne`
/// (`groundEntityNum == ENTITYNUM_NONE`).
pub fn try_jetpack(held: bool, on: bool, airborne: bool) -> bool {
    held && (on || airborne)
}

/// `Jetpack_Off` where stock calls it but for the use key: `player_die`, every tick of a
/// grip on the wearer (`ForceGrip`, `w_force.c:4185`, which also renews
/// `forceGripBeingGripped`) and the trigger a fall to death begins in (`g_trigger.c:2372`).
/// The server checks each frame; `Jetpack_On` refuses the same wearer, so the second
/// after a grip ends changes nothing.
pub fn jetpack_forced_off(packs: &mut Packs, wearer: &Wearer) {
    if wearer.health <= 0 || wearer.dead || wearer.gripped || wearer.falling_to_death {
        packs.jetpack_on = false;
    }
}

/// `ClientThink_real`'s jetpack link (`g_active.c:4007-4038`) for a player the rest of
/// the chain left in `movement_type` with `flags`: a switched-on jetpack flies one left
/// walking (`PM_JETPACK`, `EF_JETPACK_ACTIVE`); anything else lands one left flying
/// (`PM_NORMAL`) and puts out the jets (`killJetFlags`).
pub fn jetpack_movement(on: bool, movement_type: u8, flags: u32) -> (u8, u32) {
    if on && matches!(movement_type, PM_NORMAL | PM_JETPACK) {
        return (PM_JETPACK, flags | EF_JETPACK_ACTIVE);
    }
    let movement_type = if movement_type == PM_JETPACK {
        PM_NORMAL
    } else {
        movement_type
    };
    (
        movement_type,
        flags & !(EF_JETPACK_ACTIVE | EF_JETPACK_FLAMING),
    )
}

/// `ItemUse_UseCloak`: cloaked or uncloaked, a second apart; never while dead, never
/// cloaked with less than 5 in the battery.
pub fn use_cloak(packs: &mut Packs, wearer: &Wearer, level_time: i32) -> Toggled {
    if packs.cloak_toggle_time >= level_time || wearer.health <= 0 || wearer.dead {
        return Toggled::Nothing;
    }
    if !wearer.cloaked && wearer.cloak_fuel < 5 {
        return Toggled::Nothing;
    }
    packs.cloak_toggle_time = level_time + TOGGLE_TIME;
    if wearer.cloaked {
        Toggled::Decloaked
    } else {
        Toggled::Cloaked
    }
}

/// `G_RunFrame`'s batteries for one client (`g_main.c:3246-3301`): a jetpack in use drains
/// a point every 200 ms (two while thrusting) and switches off empty; one off recharges a
/// point every 150 ms; the cloak likewise, decloaking empty, unless the server's cloak
/// lasts for ever (`cloak_drains` false: JA+'s `jp_cloakFuel` 0,
/// [`crate::force_rules::ForceRules::cloak_drains`]). Returns what switched off.
pub fn batteries(
    packs: &mut Packs,
    jetpack_fuel: &mut i32,
    cloak_fuel: &mut i32,
    cloaked: bool,
    thrusting: bool,
    level_time: i32,
    cloak_drains: bool,
) -> Toggled {
    let mut off = Toggled::Nothing;
    if packs.jetpack_on {
        if packs.jetpack_reduce_at < level_time {
            *jetpack_fuel -= if thrusting { 2 } else { 1 };
            if *jetpack_fuel <= 0 {
                *jetpack_fuel = 0;
                packs.jetpack_on = false;
                off = Toggled::JetpackOff;
            }
            packs.jetpack_reduce_at = level_time + DEFUEL_RATE;
        }
    } else if *jetpack_fuel < 100 && packs.jetpack_recharge_at < level_time {
        *jetpack_fuel += 1;
        packs.jetpack_recharge_at = level_time + REFUEL_RATE;
    }
    if cloaked {
        if cloak_drains && packs.cloak_reduce_at < level_time {
            *cloak_fuel -= 1;
            if *cloak_fuel <= 0 {
                *cloak_fuel = 0;
                off = Toggled::Decloaked;
            }
            packs.cloak_reduce_at = level_time + DEFUEL_RATE;
        }
    } else if *cloak_fuel < 100 && packs.cloak_recharge_at < level_time {
        *cloak_fuel += 1;
        packs.cloak_recharge_at = level_time + REFUEL_RATE;
    }
    off
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PM_FLOAT, PM_SPECTATOR};

    fn wearer() -> Wearer {
        Wearer {
            health: 100,
            dead: false,
            jetpack_fuel: 100,
            cloak_fuel: 100,
            cloaked: false,
            gripped: false,
            falling_to_death: false,
        }
    }

    #[test]
    fn the_use_key_toggles_a_held_jetpack_in_the_air_or_while_it_burns() {
        assert!(try_jetpack(true, false, true));
        assert!(try_jetpack(true, true, false));
        // On the ground with it off the use key reaches for nothing.
        assert!(!try_jetpack(true, false, false));
        assert!(!try_jetpack(false, false, true));
    }

    #[test]
    fn death_a_grip_or_a_fall_to_death_puts_the_jetpack_out() {
        let lit = Packs {
            jetpack_on: true,
            ..Packs::default()
        };
        let mut packs = lit;
        jetpack_forced_off(&mut packs, &wearer());
        assert!(packs.jetpack_on);
        // Off, a fit wearer lights it again; the others below cannot.
        packs.jetpack_on = false;
        assert_eq!(
            use_jetpack(&mut packs, &wearer(), 5_000),
            Toggled::JetpackOn
        );
        for wearer in [
            Wearer {
                health: 0,
                ..wearer()
            },
            Wearer {
                dead: true,
                ..wearer()
            },
            Wearer {
                gripped: true,
                ..wearer()
            },
            Wearer {
                falling_to_death: true,
                ..wearer()
            },
        ] {
            let mut packs = lit;
            jetpack_forced_off(&mut packs, &wearer);
            assert!(!packs.jetpack_on);
            // Nor does the use key light it again.
            assert_eq!(use_jetpack(&mut packs, &wearer, 5_000), Toggled::Nothing);
        }
    }

    #[test]
    fn a_switched_on_jetpack_flies_a_walking_player_and_lands_it_once_off() {
        // On: walking becomes flying, the jets lit.
        assert_eq!(
            jetpack_movement(true, PM_NORMAL, 0),
            (PM_JETPACK, EF_JETPACK_ACTIVE)
        );
        // Gripped (`PM_FLOAT`), spectating or dead (5): the chain's type stands, the
        // jets out.
        let lit = EF_JETPACK_ACTIVE | EF_JETPACK_FLAMING | 1;
        assert_eq!(jetpack_movement(true, PM_FLOAT, lit), (PM_FLOAT, 1));
        assert_eq!(jetpack_movement(true, PM_SPECTATOR, lit), (PM_SPECTATOR, 1));
        assert_eq!(jetpack_movement(true, 5, lit), (5, 1));
        // Off: flying lands, walking stays.
        assert_eq!(jetpack_movement(false, PM_JETPACK, lit), (PM_NORMAL, 1));
        assert_eq!(jetpack_movement(false, PM_NORMAL, 0), (PM_NORMAL, 0));
    }

    #[test]
    fn the_cloak_drains_unless_the_server_keeps_it_on() {
        let mut packs = Packs::default();
        let (mut jet, mut cloak) = (100, 50);
        assert_eq!(
            batteries(&mut packs, &mut jet, &mut cloak, true, false, 1_000, true),
            Toggled::Nothing
        );
        assert_eq!(cloak, 49);
        // JA+'s `jp_cloakFuel` 0: on for ever, the battery as it was.
        let mut packs = Packs::default();
        let mut cloak = 1;
        for level_time in (1_000..60_000).step_by(50) {
            assert_eq!(
                batteries(
                    &mut packs, &mut jet, &mut cloak, true, false, level_time, false
                ),
                Toggled::Nothing
            );
        }
        assert_eq!(cloak, 1);
        let mut packs = Packs::default();
        assert_eq!(
            batteries(&mut packs, &mut jet, &mut cloak, true, false, 1_000, true),
            Toggled::Decloaked
        );
    }
}
