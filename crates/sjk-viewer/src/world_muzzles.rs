//! Each player's world gun muzzle as last drawn, for the disruptor beams.
//!
//! `CG_EntityEvent` starts a disruptor beam fired by another player, or by
//! the local player seen in third person, at that player's world gun
//! `*flash` bolt (`CG_GetClientWeaponMuzzleBoltPoint`, `cg_weapons.c:1970-1990`,
//! called from `cg_event.c:2805-2860`) instead of the server's muzzle point,
//! which sits in front of the chest. SJK draws the world guns in
//! `actor_world_submission`, so the bolt each one was drawn with is kept here
//! for the next snapshot's events. A player not drawn on the last frame has no
//! point, and the beam keeps the server's muzzle, as stock does for a player
//! without a Ghoul2 gun.

/// `MAX_CLIENTS` (`codemp/qcommon/q_shared.h:890`).
const MAX_CLIENTS: usize = 32;

/// The world gun muzzle of every client drawn on the last frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Points([Option<[f32; 3]>; MAX_CLIENTS]);

impl Points {
    /// Forget the last frame's points; a frame records the guns it draws.
    pub(crate) fn clear(&mut self) {
        self.0 = [None; MAX_CLIENTS];
    }

    /// `client`'s gun was drawn with its muzzle at `point`.
    pub(crate) fn record(&mut self, client: u16, point: [f32; 3]) {
        if let Some(slot) = self.0.get_mut(usize::from(client)) {
            *slot = Some(point);
        }
    }

    /// Where `client`'s gun muzzle was drawn on the last frame.
    pub(crate) fn of(&self, client: u16) -> Option<[f32; 3]> {
        self.0.get(usize::from(client)).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawn_gun_is_kept_until_the_next_frame_clears_it() {
        let mut points = Points::default();
        points.record(3, [1.0, 2.0, 3.0]);
        assert_eq!(points.of(3), Some([1.0, 2.0, 3.0]));
        assert_eq!(points.of(4), None);
        points.clear();
        assert_eq!(points.of(3), None);
    }

    #[test]
    fn a_slot_past_the_clients_is_ignored() {
        let mut points = Points::default();
        points.record(32, [1.0; 3]);
        points.record(u16::MAX, [1.0; 3]);
        assert_eq!(points, Points::default());
        assert_eq!(points.of(32), None);
    }
}
