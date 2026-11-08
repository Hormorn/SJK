//! JA+'s `jp_pushAll` (VisionJK `codemp/game/w_force.c:3146-3184` `JP_PushItem`, and
//! `codemp/game/g_items.c` where the item's first place is kept): a Force push or pull
//! flings the items it reaches, 650 units a second along the thrower's aim (towards the
//! thrower for a pull), under gravity; thirty seconds later the item is back where the map
//! put it (`RespawnItem` as its think). No weapon nor powerup moves; a flag moves only
//! with `jp_pushAll` 2 and only once dropped, and its thirty seconds end as a dropped
//! flag's do (`Team_DroppedFlagThink`: home). Not in a duel.
//!
//! The base game leaves items alone ([`crate::force_rules::ForceRules::push_items`] 0).
//!
//! SJK pushes only an item standing there: one taken and waiting for its respawn is left
//! alone, where the JA+ module would fling its hidden entity and put its respawn off to
//! thirty seconds. A flung item is back at its place at rest, where the module keeps the
//! trajectory it had (one still falling through the void stays lost).

use crate::items::{ITEMS, Kind, Pickup};

/// `JP_PushItem`'s speed.
const SPEED: f32 = 650.0;
/// `nextthink = level.time + 30000`.
const BACK_AFTER: i32 = 30_000;
/// `CONTENTS_TRIGGER`: an item standing there, to be taken.
const CONTENTS_TRIGGER: u32 = 0x400;
/// `TR_GRAVITY`.
const TR_GRAVITY: u32 = 6;
/// `EF_SEEKERDRONE`: `s.eFlags = (1 << 21)`, every other flag gone, as the module does.
const PUSHED_EFLAGS: u32 = 1 << 21;
/// The wire fields: `pos` and `apos` trajectories, `eFlags`, `groundEntityNum`.
const ES_POS_TIME: usize = 0;
const ES_POS_BASE: [usize; 3] = [2, 1, 4];
const ES_POS_DELTA: [usize; 3] = [6, 7, 10];
const ES_POS_TYPE: usize = 23;
const ES_APOS_TIME: usize = 34;
const ES_APOS_BASE: [usize; 3] = [5, 3, 33];
const ES_APOS_DELTA: [usize; 3] = [48, 44, 49];
const ES_APOS_TYPE: usize = 15;
const ES_EFLAGS: usize = 19;
const ES_GROUND: usize = 22;
/// What a fling changes and the way back restores: both trajectories, the flags, the
/// ground.
const FLUNG_FIELDS: [usize; 18] = [
    ES_POS_TYPE,
    ES_POS_TIME,
    ES_POS_BASE[0],
    ES_POS_BASE[1],
    ES_POS_BASE[2],
    ES_POS_DELTA[0],
    ES_POS_DELTA[1],
    ES_POS_DELTA[2],
    ES_APOS_TYPE,
    ES_APOS_TIME,
    ES_APOS_BASE[0],
    ES_APOS_BASE[1],
    ES_APOS_BASE[2],
    ES_APOS_DELTA[0],
    ES_APOS_DELTA[1],
    ES_APOS_DELTA[2],
    ES_EFLAGS,
    ES_GROUND,
];

/// Where a flung item goes back to (`jpSpawnOrigin`), and how it stood there: its
/// trajectories, flags and ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemHome {
    pub origin: [f32; 3],
    fields: [u32; FLUNG_FIELDS.len()],
}

/// Whether `pickup` is one a push or pull flings with `jp_pushAll` at `push_items`
/// (`JP_PushItem`'s tests), for a thrower in a duel or not.
pub fn pushable(pickup: &Pickup, push_items: u8, thrower_in_duel: bool) -> bool {
    if thrower_in_duel || push_items < 1 || pickup.contents != CONTENTS_TRIGGER {
        return false;
    }
    let row = &ITEMS[pickup.item];
    if row.kind == Kind::Weapon || row.kind == Kind::Powerup {
        return false;
    }
    let flag = matches!(row.classname, "team_CTF_redflag" | "team_CTF_blueflag");
    if flag {
        return push_items == 2 && pickup.dropped.is_some();
    }
    // Only a flag is flung once dropped: the base game drops no other item a push may
    // reach (a dropped weapon or powerup is never flung).
    pickup.dropped.is_none()
}

/// `JP_PushItem` on a [`pushable`] item: flung along `forward` (back along it for a
/// pull), its place remembered the first time, back there in thirty seconds.
pub fn push(pickup: &mut Pickup, forward: [f32; 3], pull: bool, level_time: i32) {
    let speed = if pull { -SPEED } else { SPEED };
    let state = &mut pickup.state;
    if pickup.home.is_none() && pickup.dropped.is_none() {
        pickup.home = Some(ItemHome {
            origin: pickup.origin,
            fields: FLUNG_FIELDS.map(|index| state.raw_field(index).unwrap_or(0)),
        });
    }
    match &mut pickup.dropped {
        // A dropped flag's think is `Team_DroppedFlagThink`: home once its time is up.
        Some(dropped) => dropped.free_at = level_time + BACK_AFTER,
        None => pickup.respawn_at = level_time + BACK_AFTER,
    }
    for (kind, time, base, delta) in [
        (ES_POS_TYPE, ES_POS_TIME, ES_POS_BASE, ES_POS_DELTA),
        (ES_APOS_TYPE, ES_APOS_TIME, ES_APOS_BASE, ES_APOS_DELTA),
    ] {
        state.set_raw_field(kind, TR_GRAVITY);
        state.set_raw_field(time, level_time as u32);
        for axis in 0..3 {
            state.set_raw_field(base[axis], pickup.origin[axis].to_bits());
            state.set_raw_field(delta[axis], (forward[axis] * speed).to_bits());
        }
    }
    state.set_raw_field(ES_EFLAGS, PUSHED_EFLAGS);
}

/// `RespawnItem` with `jp_pushAll` on, for a placed item that was flung: back at its
/// place, at rest as it stood. Nothing for an item never flung.
pub fn back_home(pickup: &mut Pickup) {
    let Some(home) = pickup.home.take() else {
        return;
    };
    pickup.origin = home.origin;
    // `RespawnItem` clears `EF_NODRAW` and `EF_ITEMPLACEHOLDER` after this.
    for (index, value) in FLUNG_FIELDS.into_iter().zip(home.fields) {
        pickup.state.set_raw_field(index, value);
    }
}

/// Whether `pickup` is a placed item in flight or flung, which `G_RunItem` moves
/// ([`crate::dropped_items::run`]).
pub fn flung(pickup: &Pickup) -> bool {
    pickup.dropped.is_none() && pickup.home.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TR_STATIONARY: u32 = 0;

    fn placed(classname: &str) -> Pickup {
        let item = ITEMS
            .iter()
            .position(|row| row.classname == classname)
            .unwrap();
        let mut pickup = crate::dropped_items::launch(item, [10.0, 20.0, 30.0], [0.0; 3], 0);
        pickup.dropped = None;
        pickup.state.set_raw_field(ES_POS_TYPE, TR_STATIONARY);
        pickup.state.set_raw_field(ES_GROUND, 0);
        pickup
    }

    #[test]
    fn the_base_game_and_a_duel_fling_nothing() {
        let medpac = placed("item_medpak_instant");
        assert!(!pushable(&medpac, 0, false));
        assert!(!pushable(&medpac, 1, true));
        assert!(pushable(&medpac, 1, false));
    }

    #[test]
    fn no_weapon_nor_powerup_moves_and_a_flag_only_dropped_at_2() {
        assert!(!pushable(&placed("weapon_blaster"), 2, false));
        assert!(!pushable(&placed("item_force_boon"), 2, false));
        assert!(pushable(&placed("ammo_blaster"), 1, false));
        assert!(pushable(&placed("item_shield"), 1, false));
        let flag = placed("team_CTF_redflag");
        assert!(!pushable(&flag, 1, false) && !pushable(&flag, 2, false));
        let mut dropped = flag.clone();
        dropped.dropped = Some(crate::dropped_items::Dropped {
            free_at: 30_000,
            count: 0,
            taken: false,
        });
        assert!(!pushable(&dropped, 1, false));
        assert!(pushable(&dropped, 2, false));
        // Taken and waiting: left alone.
        let mut taken = placed("ammo_blaster");
        taken.contents = 0;
        assert!(!pushable(&taken, 1, false));
    }

    #[test]
    fn a_flung_item_flies_and_comes_back_home() {
        let mut pickup = placed("ammo_blaster");
        let before = pickup.state.clone();
        push(&mut pickup, [1.0, 0.0, 0.0], false, 5_000);
        assert!(flung(&pickup));
        assert_eq!(pickup.respawn_at, 35_000);
        let read = |pickup: &Pickup, index| pickup.state.raw_field(index).unwrap();
        assert_eq!(read(&pickup, ES_POS_TYPE), TR_GRAVITY);
        assert_eq!(f32::from_bits(read(&pickup, ES_POS_DELTA[0])), 650.0);
        assert_eq!(read(&pickup, ES_EFLAGS), PUSHED_EFLAGS);
        // A pull comes back along the aim.
        push(&mut pickup, [1.0, 0.0, 0.0], true, 6_000);
        assert_eq!(f32::from_bits(read(&pickup, ES_POS_DELTA[0])), -650.0);
        pickup.origin = [300.0, 0.0, 0.0];
        back_home(&mut pickup);
        assert!(!flung(&pickup));
        assert_eq!(pickup.origin, [10.0, 20.0, 30.0]);
        assert_eq!(pickup.state, before);
    }
}
