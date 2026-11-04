//! What other systems than map edits stand on the map, in the browser editor's World
//! things page (Sol, 03/11/2026: "also boombox placement, the toymaster cat and box"):
//! the player's own boombox on the floor and the other players' ([`crate::boombox`]),
//! the mystery boxes ([`crate::mystery_box`]) and the Toy Master's toys
//! ([`crate::toy_master`]).
//!
//! The page draws each with its models and places and removes them through those
//! modules' own code, as their console commands do, with the spot the page's ray meets
//! instead of the player's aim. The boombox moves by being placed again; a mystery box
//! and a toy are the hub's, which places and removes them but has no move, so the page
//! has none for them either.

use super::http::Thing;
use crate::GpuState;
use glam::Vec3;
use serde_json::{Value, json};

/// How far along the page's ray a thing may be placed.
const REACH: f32 = 8192.0;

/// One model of a thing as the game draws it at rest.
fn part(model: &str, origin: Vec3, angles: [f32; 3], scale: f32) -> Value {
    json!({ "model": model, "origin": origin.to_array(), "angles": angles, "scale": scale })
}

impl GpuState {
    /// The things on the map for `api/edits`, each with its parts to draw, and what
    /// placing them needs (a game server; the hub's toy kinds).
    pub(super) fn map_edit_web_things(&self) -> Value {
        let mut things = Vec::new();
        for (slot, rest, yaw) in self.boombox.standing() {
            let name = match slot {
                None => "Your boombox".to_owned(),
                Some(slot) => self
                    .live_session
                    .as_ref()
                    .and_then(|session| {
                        crate::player_identity::shown_name(session.game_state(), usize::from(slot))
                    })
                    .map_or_else(
                        || format!("Boombox of player {slot}"),
                        |name| format!("Boombox of {name}"),
                    ),
            };
            things.push(json!({
                "thing": "boombox",
                "id": 0,
                "label": name,
                "mine": slot.is_none(),
                "origin": rest.to_array(),
                "parts": [part(crate::boombox::MODEL, rest, [0.0, yaw, 0.0], crate::boombox::SCALE)],
            }));
        }
        for (id, origin, yaw) in self.mystery_boxes.standing() {
            things.push(json!({
                "thing": "mystery_box",
                "id": id,
                "label": format!("Mystery box {id}"),
                "mine": false,
                "origin": origin.to_array(),
                "parts": [
                    part(crate::mystery_box::MODEL, origin, [0.0, yaw, 0.0], 1.0),
                    part(
                        crate::mystery_box::LID_MODEL,
                        crate::mystery_box::lid_origin(origin, yaw),
                        [0.0, yaw, 0.0],
                        1.0,
                    ),
                ],
            }));
        }
        for toy in self.toy_master.hidden() {
            let escorted = if toy.escorted {
                ", shown only where you are"
            } else {
                ""
            };
            things.push(json!({
                "thing": "toy",
                "id": toy.id,
                "label": format!("Toy {}: {}{escorted}", toy.id, toy.kind.unwrap_or("?")),
                "mine": false,
                "origin": toy.origin.to_array(),
                "parts": [part(toy.model, toy.origin, [0.0, toy.yaw, 0.0], toy.scale)],
            }));
        }
        let on_server = self
            .live_session
            .as_ref()
            .is_some_and(|session| !session.is_local());
        json!({
            "list": things,
            "on_server": on_server,
            "toy_kinds": self.toy_master.kinds(),
        })
    }

    /// Stand `thing` where the page's ray from `eye` along `dir` meets the map, facing
    /// back along `forward` as the console commands face the player.
    pub(super) fn map_edit_web_place_thing(
        &mut self,
        thing: Thing,
        ray: [[f32; 3]; 3],
        kind: &str,
        one_server: bool,
        escort: bool,
    ) -> Result<Vec<String>, String> {
        let [eye, dir, forward] = ray.map(Vec3::from_array);
        let dir = dir.try_normalize().ok_or("No direction to place along")?;
        let hit = self
            .decal_surfaces
            .ray_hit(eye, dir, REACH)
            .ok_or("Click the floor where it should stand")?;
        let at = eye + dir * hit.distance;
        let yaw = (forward.y.atan2(forward.x).to_degrees() + 180.0).rem_euclid(360.0);
        match thing {
            Thing::Boombox => self.place_boombox_at(at, yaw).map(|line| vec![line]),
            Thing::MysteryBox | Thing::Toy => {
                let (who, what) = match thing {
                    Thing::Toy => ("Only the Toy Master hides toys", "toys"),
                    _ => ("Only SJK's creator places mystery boxes", "boxes"),
                };
                if !crate::player_identity::is_creator() {
                    return Err(who.into());
                }
                let server = self.map_edits_server().ok_or(format!(
                    "Join a game server first: {what} stand on a server's map"
                ))?;
                let server = one_server.then_some(server);
                Ok(vec![if thing == Thing::Toy {
                    self.toy_master_place_at(kind.to_owned(), at, yaw, server, escort)
                } else {
                    self.mystery_box_place_at(at, yaw, server)
                }])
            }
        }
    }

    /// Take `thing` `id` away: the hub's through their own commands, the boombox back
    /// on the shoulder.
    pub(super) fn map_edit_web_remove_thing(
        &mut self,
        thing: Thing,
        id: i64,
    ) -> Result<Vec<String>, String> {
        let id = id.to_string();
        match thing {
            Thing::Boombox => self.pick_up_boombox().map(|line| vec![line]),
            Thing::MysteryBox => self.mystery_box_command("mysterybox_remove", &[id]),
            Thing::Toy => self.toy_master_command("toymaster_remove", &[id]),
        }
    }
}
