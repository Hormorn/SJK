//! The E-Webs on this server ([`sjk_game_jka::eweb`]): the holdable a player sets down with
//! the use key (`ItemUse_UseEWeb`) and the single-player maps' own (`emplaced_eweb`, spawned
//! with the level), each an entity of the pool sent to the clients as a stock server sends
//! it — a Ghoul2 `eweb_model.glm` whose two bones the clients turn by the angles in its
//! state — and thinking every frame before the missiles (`EWebThink`): its gunner held on
//! the emplaced gun's weapon and kept behind it, its top turned after the gunner's view,
//! its bolts fired from the muzzle of the server's own posed copy of the model. It is solid
//! to everyone but its owner, struck by missiles, splash and the world's blows like the
//! emplaced guns, and blows up when shot to pieces or when its owner dies or leaves.

use super::*;
use sjk_game_jka::eweb::{self, Bones, Eweb, Kind, Taker};
use sjk_game_jka::map_turret_model::{TurretModel, TurretPose};
use std::sync::Arc;

/// `EFFECT_EXPLOSION_DETPACK`; `MOD_SUICIDE`, `MOD_UNKNOWN`.
const EFFECT_EXPLOSION_DETPACK: u32 = 6;
const MOD_SUICIDE: u32 = sjk_game_jka::means_of_death::MOD_SUICIDE;
const MOD_UNKNOWN: u32 = 0;
/// `STAT_HOLDABLE_ITEM`; `HI_EWEB`; `BUTTON_USE`; `PMF_DUCKED`.
const STAT_HOLDABLE_ITEM: usize = 1;
const HI_EWEB: i32 = sjk_game_jka::holdables::HI_EWEB;
const BUTTON_USE: u16 = 32;
const PMF_DUCKED: u16 = 1;
/// `ps.emplacedIndex`, `ps.weapon`.
const PS_EMPLACED_INDEX: usize = 112;
const PS_WEAPON: usize = 47;
/// `ES_SOLID`.
const ES_SOLID: usize = 26;
/// `WP_TURRET`'s item, registered with the first E-Web (`EWebPrecache`).
const WP_TURRET: u32 = 18;

/// One player's E-Web memory (`client->ewebIndex`, `ewebTime`, `ewebHealth`).
#[derive(Clone, Copy, Debug, Default)]
struct Owner {
    eweb: u16,
    time: i32,
    health: i32,
}

/// A gun of the level, its server copy of the model's pose, and whether the use key that
/// took it (a map gun) is still held.
pub(crate) struct Gun {
    pub(crate) id: EntityId,
    pub(crate) eweb: Eweb,
    pose: Option<TurretPose>,
    use_held: bool,
}

/// The level's E-Webs and what the players keep of theirs.
#[derive(Default)]
pub(crate) struct Ewebs {
    pub(crate) guns: Vec<Gun>,
    owners: Vec<Owner>,
    /// The model's files, read once a level (`None` inside: the files lack it).
    model: Option<Option<Arc<TurretModel>>>,
    /// The model's index and its bones', registered once a level.
    indices: Option<(u16, Bones)>,
}

impl Ewebs {
    fn owner(&mut self, client: usize) -> &mut Owner {
        if self.owners.len() <= client {
            self.owners.resize(client + 1, Owner::default());
        }
        &mut self.owners[client]
    }

    fn index(&self, number: u16) -> Option<usize> {
        self.guns.iter().position(|gun| gun.eweb.number == number)
    }

    /// The guns as boxes in `passer`'s way: all but its own (`r.ownerNum`) and the one it
    /// works.
    pub(crate) fn obstacles(&self, passer: u16) -> impl Iterator<Item = BoxObstacle> + '_ {
        self.guns
            .iter()
            .filter(move |gun| {
                gun.eweb.owner() != Some(passer)
                    && gun.eweb.gunner != Some(passer)
                    && gun.eweb.number != passer
            })
            .map(|gun| BoxObstacle {
                entity: gun.eweb.number,
                origin: gun.eweb.origin,
                bounds: gun.eweb.bounds(),
                contents: gun.eweb.contents(),
                model: None,
            })
    }

    /// The guns a blast reaches, with their linked boxes.
    pub(crate) fn splash_targets(&self) -> impl Iterator<Item = SplashTarget> + '_ {
        self.guns.iter().map(|gun| {
            let (mins, maxs) = gun.eweb.bounds();
            let origin = gun.eweb.origin;
            SplashTarget {
                number: gun.eweb.number,
                bounds: (
                    std::array::from_fn(|axis| origin[axis] + mins[axis] - 1.0),
                    std::array::from_fn(|axis| origin[axis] + maxs[axis] + 1.0),
                ),
                origin,
                takes_damage: !gun.eweb.invulnerable,
            }
        })
    }
}

impl NativeGame {
    /// The model's files for the server's copy of it, read the first time a level asks.
    fn eweb_model(&mut self) -> Option<Arc<TurretModel>> {
        if let Some(model) = &self.ewebs.model {
            return model.clone();
        }
        let map = self.map.as_ref()?;
        let read = |path: &str| map.files.read(path).ok().flatten().map(|asset| asset.bytes);
        let path = String::from_utf8_lossy(eweb::MODEL).into_owned();
        let loaded = read(&path).and_then(|glm| {
            let skeleton = sjk_game_jka::vehicle_skeleton::VehicleModels::skeleton_name(&glm)?;
            let gla = read(&format!("{skeleton}.gla"))?;
            TurretModel::new(&glm, &gla).ok().map(Arc::new)
        });
        if loaded.is_none() {
            eprintln!("eweb: no server model {path}; its shots leave from its origin");
        }
        self.ewebs.model = Some(loaded.clone());
        loaded
    }

    /// `G_ModelIndex` of the model and `G_BoneIndex` of its two bones, the clients told.
    fn eweb_indices(&mut self) -> (u16, Bones) {
        if let Some(indices) = self.ewebs.indices {
            return indices;
        }
        let (models, told) = (&mut self.models, &mut self.told);
        let mut tell = |index, value: &[u8]| {
            told.push(Told::ConfigString {
                index,
                previous: Vec::new(),
                value: value.to_vec(),
            })
        };
        let model = models.index(eweb::MODEL, &mut tell);
        let bones = &mut self.map_effects.bones;
        let yaw = bones.index(eweb::YAW_BONE, &mut tell);
        let pitch = bones.index(eweb::PITCH_BONE, &mut tell);
        // `EWebPrecache`: the turret bolt's item, so the clients load what draws it.
        if let Some(item) = sjk_game_jka::dropped_items::item_for_weapon(WP_TURRET) {
            self.map_turrets.register_item(item);
        }
        self.ewebs.indices = Some((model, Bones { yaw, pitch }));
        self.register_turret_items();
        (model, Bones { yaw, pitch })
    }

    /// `G_EffectIndex(name)`, the clients told.
    fn eweb_effect(&mut self, name: &[u8]) -> u16 {
        let (effects, told) = (&mut self.map_effects.effects, &mut self.told);
        effects.index(name, &mut |index, value| {
            told.push(Told::ConfigString {
                index,
                previous: Vec::new(),
                value: value.to_vec(),
            })
        })
    }

    /// The level's guns: the ones before freed (their gunners let go), then every
    /// `emplaced_eweb` the map places.
    pub(in crate::bridge) fn spawn_ewebs(&mut self) {
        let level_time = self.last_frame_time;
        for gun in std::mem::take(&mut self.ewebs.guns) {
            self.pool.free(gun.id, level_time);
            if let Some(gunner) = gun.eweb.gunner {
                self.let_go_of_eweb(usize::from(gunner), &gun.eweb);
            }
        }
        self.ewebs.owners.clear();
        self.ewebs.model = None;
        self.ewebs.indices = None;
        let Some(map) = self.map.as_ref() else { return };
        let placed: Vec<_> = map
            .entities
            .iter()
            .filter(|entity| {
                entity
                    .classname()
                    .is_some_and(|name| name.eq_ignore_ascii_case("emplaced_eweb"))
            })
            .cloned()
            .collect();
        if placed.is_empty() {
            return;
        }
        let (model_index, bones) = self.eweb_indices();
        let model = self.eweb_model();
        for entity in &placed {
            let Some(id) = self.pool.spawn_entity(
                EntityState::zero(0, &sjk_protocol::LEGACY_ENTITY_FIELDS),
                level_time,
            ) else {
                break;
            };
            let eweb = eweb::from_map(entity, id.legacy_number(), model_index, bones);
            let pose = model.as_deref().map(TurretPose::new);
            self.ewebs.guns.push(Gun {
                id,
                eweb,
                pose,
                use_held: false,
            });
            self.publish_eweb(self.ewebs.guns.len() - 1);
        }
    }

    /// The gun at `index` as the clients are sent it: its state into its slot, its box
    /// packed into `solid`, linked with its box.
    fn publish_eweb(&mut self, index: usize) {
        let gun = &self.ewebs.guns[index];
        let (id, bounds) = (gun.id, gun.eweb.bounds());
        let Some(slot) = self.pool.state_mut(id) else {
            return;
        };
        for field in 0..sjk_protocol::LEGACY_ENTITY_FIELDS.len() {
            slot.set_raw_field(field, gun.eweb.state.raw_field(field).unwrap_or(0));
        }
        let byte = |value: f32| (value as i32).clamp(1, 255) as u32;
        slot.set_raw_field(
            ES_SOLID,
            byte(bounds.1[2] + 32.0) << 16 | byte(-bounds.0[2]) << 8 | byte(bounds.1[0]),
        );
        self.pool.set_bounds(id, bounds);
    }

    /// `ItemUse_UseEWeb` (`g_items.c:2014-2048`): the E-Web put away when it is out, else
    /// set down in front of the player — not while busy, on another gun, or within a second
    /// of the last use.
    pub(in crate::bridge) fn use_eweb(&mut self, client: usize, level_time: i32) {
        let memo = *self.ewebs.owner(client);
        if memo.time > level_time {
            return;
        }
        let Some(peer) = self.peer(client) else {
            return;
        };
        if peer.state.weapon_time() > 0 || peer.state.force_hand_extend() != 0 {
            return;
        }
        let on = peer.state.emplaced_index();
        if on != 0 && memo.eweb == 0 {
            return;
        }
        if memo.eweb != 0 {
            if let Some(index) = self.ewebs.index(memo.eweb) {
                self.put_away_eweb(index, level_time);
            }
        } else {
            self.set_down_eweb(client, level_time);
        }
        self.ewebs.owner(client).time = level_time + eweb::USE_DEBOUNCE;
    }

    /// `EWeb_Create` (`g_items.c:1886-2010`) for `client`, and the player on it.
    fn set_down_eweb(&mut self, client: usize, level_time: i32) {
        let Some(peer) = self.peer(client) else {
            return;
        };
        let (origin, view) = (peer.state.origin(), peer.state.view_angles());
        let (team, weapons) = (peer.session.team, peer.state.stats[STAT_WEAPONS]);
        self.gather_obstacles(client);
        let placed = {
            let obstacles = &self.obstacles;
            let mut trace = |start, mins, maxs, end| match self.map.as_ref() {
                Some(map) => WithPlayers {
                    world: WorldCollision {
                        bsp: &map.bsp,
                        scratch: &map.scratch,
                    },
                    players: obstacles,
                }
                .trace(start, mins, maxs, end, eweb::MASK_PLAYERSOLID),
                None => WithPlayers {
                    world: Void,
                    players: obstacles,
                }
                .trace(start, mins, maxs, end, eweb::MASK_PLAYERSOLID),
            };
            eweb::place(origin, view[1], &mut trace)
        };
        let Some(at) = placed else {
            // `G_Sound(spawner, CHAN_AUTO, "sound/interface/shieldcon_empty")`.
            let (sounds, told) = (&mut self.sounds, &mut self.told);
            let sound = sounds.index(b"sound/interface/shieldcon_empty", &mut |index, value| {
                told.push(Told::ConfigString {
                    index,
                    previous: Vec::new(),
                    value: value.to_vec(),
                })
            });
            let event = eweb::sound(origin, sound);
            let _ = self.pool.spawn_temporary(event.state(), level_time, None);
            return;
        };
        let (model_index, bones) = self.eweb_indices();
        let model = self.eweb_model();
        let _ = self.eweb_effect(eweb::MUZZLE_FLASH);
        let Some(id) = self.pool.spawn_entity(
            EntityState::zero(0, &sjk_protocol::LEGACY_ENTITY_FIELDS),
            level_time,
        ) else {
            return;
        };
        let number = id.legacy_number();
        let health = self.ewebs.owner(client).health;
        let made = eweb::portable(
            number,
            client as u16,
            team,
            at,
            view[1],
            health,
            weapons,
            model_index,
            bones,
            level_time,
        );
        self.ewebs.owner(client).health = made.health;
        self.ewebs.owner(client).eweb = number;
        let pose = model.as_deref().map(TurretPose::new);
        self.ewebs.guns.push(Gun {
            id,
            eweb: made,
            pose,
            use_held: false,
        });
        if let Some(peer) = self.peer_mut(client) {
            peer.state
                .set_raw_field(PS_EMPLACED_INDEX, u32::from(number));
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        self.publish_eweb(self.ewebs.guns.len() - 1);
    }

    /// `TryUse` reaching map gun `found`: taken (`eweb_use`) when it may be. Whether
    /// `found` is an E-Web.
    pub(in crate::bridge) fn use_map_eweb(
        &mut self,
        client: usize,
        found: u16,
        level_time: i32,
    ) -> bool {
        let Some(index) = self.ewebs.index(found) else {
            return false;
        };
        let Some(peer) = self.peer(client) else {
            return true;
        };
        let taker = Taker {
            view: peer.state.view_angles(),
            alive: peer.health > 0,
            ducked: peer.state.movement_flags() & PMF_DUCKED != 0,
            riding: peer.state.vehicle_entity_num() != 0,
        };
        let weapons = peer.state.stats[STAT_WEAPONS];
        if !eweb::can_take(&self.ewebs.guns[index].eweb, &taker, level_time) {
            return true;
        }
        let gun = &mut self.ewebs.guns[index];
        gun.eweb.gunner = Some(client as u16);
        gun.eweb.saved_weapons = weapons;
        gun.eweb.taken_at = level_time;
        gun.use_held = true;
        let number = gun.eweb.number;
        if let Some(peer) = self.peer_mut(client) {
            peer.state
                .set_raw_field(PS_EMPLACED_INDEX, u32::from(number));
            peer.state.set_raw_field(PS_WEAPON, eweb::WP_EMPLACED_GUN);
            peer.state.stats[STAT_WEAPONS] |= 1 << eweb::WP_EMPLACED_GUN;
            peer.movement = peer.movement.reseeded(&peer.state);
        }
        self.eweb_sound(index, b"sound/weapons/eweb/eweb_mount.mp3", level_time);
        self.publish_eweb(index);
        true
    }

    /// The use key pressed again by a map gun's gunner: it lets go. Whether it did.
    pub(in crate::bridge) fn leave_map_eweb_by_use(
        &mut self,
        client: usize,
        level_time: i32,
    ) -> bool {
        let Some(on) = self.peer(client).map(|peer| peer.state.emplaced_index()) else {
            return false;
        };
        let Some(index) = self.ewebs.index(on) else {
            return false;
        };
        let gun = &self.ewebs.guns[index];
        if !matches!(gun.eweb.kind, Kind::Map { .. })
            || gun.eweb.gunner != Some(client as u16)
            || gun.use_held
            || gun.eweb.taken_at + eweb::UNFOLD_TIME >= level_time
        {
            return false;
        }
        self.release_map_eweb(index, level_time);
        true
    }

    /// `G_Sound(gun, CHAN_AUTO, name)`.
    fn eweb_sound(&mut self, index: usize, name: &[u8], level_time: i32) {
        let origin = self.ewebs.guns[index].eweb.origin;
        let (sounds, told) = (&mut self.sounds, &mut self.told);
        let sound = sounds.index(name, &mut |index, value| {
            told.push(Told::ConfigString {
                index,
                previous: Vec::new(),
                value: value.to_vec(),
            })
        });
        let event = eweb::sound(origin, sound);
        let _ = self.pool.spawn_temporary(event.state(), level_time, None);
    }

    /// `EWebDisattach` (`g_items.c:1456-1471`): the holdable at `index` put away — its
    /// owner's weapons given back while it lives — and the gun freed.
    fn put_away_eweb(&mut self, index: usize, level_time: i32) {
        let gun = self.ewebs.guns.remove(index);
        self.pool.free(gun.id, level_time);
        if let Kind::Portable { owner } = gun.eweb.kind {
            let memo = self.ewebs.owner(usize::from(owner));
            memo.eweb = 0;
            memo.health = gun.eweb.health;
            self.let_go_of_eweb(usize::from(owner), &gun.eweb);
        }
    }

    /// A map gun's gunner let go of it (the single-player game's `ExitEmplacedWeapon`):
    /// the gun stays, its top as it was.
    fn release_map_eweb(&mut self, index: usize, level_time: i32) {
        let gun = &mut self.ewebs.guns[index];
        let Some(gunner) = gun.eweb.gunner.take() else {
            return;
        };
        gun.eweb.taken_at = level_time;
        if gun.eweb.firing {
            eweb::set_animation(&mut gun.eweb.state, 0, 1);
            gun.eweb.firing = false;
        }
        let eweb = gun.eweb.clone();
        self.let_go_of_eweb(usize::from(gunner), &eweb);
        if eweb.health > 0 {
            self.eweb_sound(index, b"sound/weapons/eweb/eweb_dismount.mp3", level_time);
        }
        self.publish_eweb(index);
    }

    /// The player off the gun: `emplacedIndex` cleared, its weapons back while it lives
    /// (none when it is dead), its weapon the best it holds.
    fn let_go_of_eweb(&mut self, client: usize, eweb: &Eweb) {
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        if peer.state.emplaced_index() == eweb.number {
            peer.state.set_raw_field(PS_EMPLACED_INDEX, 0);
        }
        peer.state.stats[STAT_WEAPONS] = if peer.health > 0 {
            eweb.saved_weapons
        } else {
            0
        };
        peer.movement = peer.movement.reseeded(&peer.state);
    }

    /// `G_Damage` on gun `number`. Whether `number` is an E-Web.
    pub(in crate::bridge) fn hurt_eweb(
        &mut self,
        number: u16,
        damage: i32,
        level_time: i32,
    ) -> bool {
        let Some(index) = self.ewebs.index(number) else {
            return false;
        };
        let died = eweb::damage(&mut self.ewebs.guns[index].eweb, damage);
        // `EWebPain`: the owner keeps its gun's health for the next one.
        let gun = &self.ewebs.guns[index].eweb;
        if let Some(owner) = gun.owner() {
            let health = gun.health;
            self.ewebs.owner(usize::from(owner)).health = health;
        }
        if died {
            self.eweb_dies(index, level_time);
        } else {
            self.publish_eweb(index);
        }
        true
    }

    /// `EWebDie` (`g_items.c:1486-1519`) and the map gun's `eweb_die`: the blast, the
    /// explosion, the gunner let go — the holdable's owner losing it for good — and the
    /// gun gone.
    fn eweb_dies(&mut self, index: usize, level_time: i32) {
        let gun = self.ewebs.guns.remove(index);
        self.pool.free(gun.id, level_time);
        let (damage, radius) = eweb::blast(&gun.eweb);
        let origin = gun.eweb.origin;
        match gun.eweb.kind {
            Kind::Portable { owner } => {
                let event = eweb::play_effect(EFFECT_EXPLOSION_DETPACK, origin, [1.0, 0.0, 0.0]);
                let _ = self.pool.spawn_temporary(event.state(), level_time, None);
                let client = usize::from(owner);
                let memo = self.ewebs.owner(client);
                memo.eweb = 0;
                // "make sure it resets next time we spawn one".
                memo.health = -1;
                self.let_go_of_eweb(client, &gun.eweb);
                if let Some(peer) = self.peer_mut(client) {
                    // "take it away from him, it is gone forever."
                    let held = peer.state.stats[STAT_HOLDABLE_ITEMS] & !(1 << HI_EWEB);
                    peer.state.stats[STAT_HOLDABLE_ITEMS] = held;
                    let selected = peer.state.stats[STAT_HOLDABLE_ITEM];
                    if selected == sjk_game_jka::holdables::index_of(HI_EWEB) as u32 {
                        let next = sjk_game_jka::holdables::cycle(held, 0, 1);
                        peer.state.stats[STAT_HOLDABLE_ITEM] = next;
                    }
                    peer.movement = peer.movement.reseeded(&peer.state);
                }
                self.gun_blast(
                    gun.eweb.number,
                    Some(owner),
                    sjk_game_jka::emplaced::Blast {
                        origin,
                        damage,
                        radius,
                    },
                    MOD_SUICIDE,
                    level_time,
                );
            }
            Kind::Map { .. } => {
                if let Some(gunner) = gun.eweb.gunner {
                    self.let_go_of_eweb(usize::from(gunner), &gun.eweb);
                }
                let effect = self.eweb_effect(eweb::MAP_EXPLOSION);
                let mut at = origin;
                at[2] += 20.0;
                let event = eweb::play_effect_id(effect, at, [0.0, 0.0, 1.0]);
                let _ = self.pool.spawn_temporary(event.state(), level_time, None);
                self.gun_blast(
                    gun.eweb.number,
                    gun.eweb.gunner,
                    sjk_game_jka::emplaced::Blast {
                        origin,
                        damage,
                        radius,
                    },
                    MOD_UNKNOWN,
                    level_time,
                );
            }
        }
    }

    /// `EWebThink` (`g_items.c:1806-1883`) for every gun, every frame.
    pub(in crate::bridge) fn run_ewebs(&mut self, level_time: i32) {
        let mut index = 0;
        while index < self.ewebs.guns.len() {
            let number = self.ewebs.guns[index].eweb.number;
            self.eweb_think(index, level_time);
            // A gun that died or was put away is gone from the list: its place is the next's.
            if self
                .ewebs
                .guns
                .get(index)
                .is_some_and(|gun| gun.eweb.number == number)
            {
                self.publish_eweb(index);
                index += 1;
            }
        }
    }

    fn eweb_think(&mut self, index: usize, level_time: i32) {
        let (number, kind, gunner) = {
            let gun = &self.ewebs.guns[index].eweb;
            (gun.number, gun.kind, gun.gunner)
        };
        let alive_on = |game: &Self, client: u16| {
            game.peer(usize::from(client))
                .filter(|peer| peer.begun && peer.health >= 1)
                .map(|peer| peer.state.emplaced_index())
        };
        match kind {
            Kind::Portable { owner } => {
                let mine = self.ewebs.owner(usize::from(owner)).eweb == number;
                match alive_on(self, owner) {
                    // "something happened to the owner, let's explode".
                    None => {
                        self.eweb_dies(index, level_time);
                        return;
                    }
                    _ if !mine => {
                        self.eweb_dies(index, level_time);
                        return;
                    }
                    // "just go back to the inventory then".
                    Some(on) if on != number => {
                        self.put_away_eweb(index, level_time);
                        return;
                    }
                    Some(_) => {}
                }
            }
            Kind::Map { .. } => {
                if let Some(client) = gunner
                    && alive_on(self, client) != Some(number)
                {
                    self.release_map_eweb(index, level_time);
                    return;
                }
            }
        }
        let Some(client) = self.ewebs.guns[index].eweb.gunner else {
            return;
        };
        let client = usize::from(client);
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        let view = peer.state.view_angles();
        let buttons = peer.last_command.buttons;
        peer.state.set_raw_field(PS_WEAPON, eweb::WP_EMPLACED_GUN);
        match kind {
            // `stats[STAT_WEAPONS] = WP_EMPLACED_GUN`: the reference's own assignment
            // (the number, not its bit), which the holdable's gunner holds while on it.
            Kind::Portable { .. } => peer.state.stats[STAT_WEAPONS] = eweb::WP_EMPLACED_GUN,
            Kind::Map { .. } => peer.state.stats[STAT_WEAPONS] |= 1 << eweb::WP_EMPLACED_GUN,
        }
        peer.movement = peer.movement.reseeded(&peer.state);
        if buttons & BUTTON_USE == 0 {
            self.ewebs.guns[index].use_held = false;
        }
        if self.ewebs.guns[index].eweb.ready_at >= level_time {
            return;
        }
        let (bones, model) = (self.eweb_indices().1, self.eweb_model());
        let clock = self.previous_frame_time;
        // `EWebUpdateBoneAngles`: the yaw first, the gunner placed by it, then the pitch.
        let gun = &mut self.ewebs.guns[index];
        let (yaw, pitch) = eweb::turn(&mut gun.eweb, view);
        eweb::set_bone_angles(&mut gun.eweb.state, bones.yaw, yaw);
        if let (Some(pose), Some(model)) = (gun.pose.as_mut(), model.as_deref()) {
            pose.set_bone_angles(model, "cannon_Yrot", yaw, level_time);
        }
        let place_bolt = gun.eweb.place_bolt();
        let place = self
            .eweb_bolt(index, model.as_deref(), place_bolt, clock)
            .map(|bolt| eweb::gunner_place(&self.ewebs.guns[index].eweb, bolt));
        if let Some(place) = place
            && !self.place_eweb_gunner(client, place)
        {
            // "can't move here.. stop using the thing I guess".
            match kind {
                Kind::Portable { .. } => self.put_away_eweb(index, level_time),
                Kind::Map { .. } => self.release_map_eweb(index, level_time),
            }
            return;
        }
        let gun = &mut self.ewebs.guns[index];
        eweb::set_bone_angles(&mut gun.eweb.state, bones.pitch, pitch);
        if let (Some(pose), Some(model)) = (gun.pose.as_mut(), model.as_deref()) {
            pose.set_bone_angles(model, "cannon_Xrot", pitch, level_time);
        }
        if buttons & BUTTON_ATTACK != 0 {
            if gun.eweb.next_fire < level_time {
                self.fire_eweb(index, client as u16, model.as_deref(), clock, level_time);
                let gun = &mut self.ewebs.guns[index].eweb;
                // "cheap firing anim".
                eweb::set_animation(&mut gun.state, 2, 4);
                gun.firing = true;
                gun.next_fire = level_time + eweb::FIRE_DEBOUNCE;
            }
        } else if gun.eweb.next_fire < level_time && gun.eweb.firing {
            // "reset the anim back to non-firing".
            eweb::set_animation(&mut gun.eweb.state, 0, 1);
            gun.eweb.firing = false;
        }
    }

    /// `G2API_GetBoltMatrix` of `bolt` on the gun's posed model at the Ghoul2 `clock`;
    /// the gun's origin, unturned, where the model or the bolt is missing.
    fn eweb_bolt(
        &mut self,
        index: usize,
        model: Option<&TurretModel>,
        bolt: &str,
        clock: i32,
    ) -> Option<[[f32; 4]; 3]> {
        let gun = &mut self.ewebs.guns[index];
        let (angles, origin) = (gun.eweb.model_angles(), gun.eweb.origin);
        let posed = model.zip(gun.pose.as_mut()).and_then(|(model, pose)| {
            pose.bolt_matrix(model, bolt, angles, origin, [0.0; 3], clock)
                .ok()
                .flatten()
        });
        posed.or_else(|| {
            model.is_none().then(|| {
                std::array::from_fn(|row| {
                    std::array::from_fn(|column| match column {
                        3 => origin[row],
                        _ if column == row => 1.0,
                        _ => 0.0,
                    })
                })
            })
        })
    }

    /// `EWebPositionUser`'s traces (`JP_Trace`, `MASK_PLAYERSOLID`, the player's box,
    /// passing it): the gunner moved onto `place` when it can get there and stand there
    /// (seven units down). Whether it could.
    fn place_eweb_gunner(&mut self, client: usize, place: [f32; 3]) -> bool {
        let Some(peer) = self.peer(client) else {
            return false;
        };
        let (origin, (mins, maxs)) = (peer.state.origin(), peer.movement.box_bounds());
        self.gather_obstacles(client);
        let obstacles = &self.obstacles;
        let trace = |start, end| match self.map.as_ref() {
            Some(map) => WithPlayers {
                world: WorldCollision {
                    bsp: &map.bsp,
                    scratch: &map.scratch,
                },
                players: obstacles,
            }
            .trace(start, mins, maxs, end, eweb::MASK_PLAYERSOLID),
            None => WithPlayers {
                world: Void,
                players: obstacles,
            }
            .trace(start, mins, maxs, end, eweb::MASK_PLAYERSOLID),
        };
        let across = trace(origin, place);
        if across.start_solid || across.all_solid || across.fraction != 1.0 {
            return false;
        }
        let mut down = place;
        down[2] -= 7.0;
        let ground = trace(place, down);
        if !ground.start_solid && !ground.all_solid {
            let end = ground.end_position;
            if let Some(peer) = self.peer_mut(client) {
                peer.state.set_origin(end);
                peer.movement = peer.movement.reseeded(&peer.state);
            }
        }
        true
    }

    /// `EWebFire` (`g_items.c:1656-1696`): the bolt from the muzzle, its flash.
    fn fire_eweb(
        &mut self,
        index: usize,
        gunner: u16,
        model: Option<&TurretModel>,
        clock: i32,
        level_time: i32,
    ) {
        // "Bad e-web bolt": nothing without the muzzle.
        let Some(muzzle) = self.eweb_bolt(index, model, eweb::MUZZLE_BOLT, clock) else {
            return;
        };
        let (missile, point, direction) =
            eweb::bolt(&self.ewebs.guns[index].eweb, gunner, muzzle, level_time);
        if let Some(id) = self.pool.spawn_entity(missile.state.clone(), level_time) {
            self.pool.set_bounds(id, missile.bounds);
            self.missiles.push((id, missile));
        }
        let flash = self.eweb_effect(eweb::MUZZLE_FLASH);
        let angles = sjk_game_jka::player_angle_math::vector_angles(direction);
        let event = eweb::play_effect_id(flash, point, angles);
        let _ = self.pool.spawn_temporary(event.state(), level_time, None);
    }
}

#[cfg(test)]
mod tests {
    use super::NativeGame;
    use crate::bridge::Identity;
    use crate::cvars::Cvars;

    /// A mapless server with player 0 in it.
    fn game() -> NativeGame {
        let mut cvars = Cvars::new();
        cvars.set(b"g_userinfoValidate", b"0");
        cvars.register_server();
        let identity = Identity {
            hostname: b"test".to_vec(),
            mapname: b"mp/ffa3".to_vec(),
        };
        let mut game = NativeGame::with_cvars(identity, None, 4, 4, cvars).unwrap();
        game.connect_client(0, br"\name\P0", true).unwrap();
        game.begin(0, 0, None);
        game
    }

    #[test]
    fn the_give_cheat_gives_the_eweb_and_its_key_reaches_it() {
        let mut game = game();
        game.give_to(0, b"item_eweb_holdable", &[]);
        let held = game.peer(0).unwrap().state.stats[super::STAT_HOLDABLE_ITEMS];
        assert_ne!(held & (1 << super::HI_EWEB), 0, "held {held:#x}");
        let before = game.ewebs.owner(0).time;
        assert!(game.generic_holdable(0, 24, 5_000));
        // Reached `ItemUse_UseEWeb`: its debounce is set (no ground here, so no gun).
        assert_eq!(
            game.ewebs.owner(0).time,
            5_000 + sjk_game_jka::eweb::USE_DEBOUNCE,
            "{before}"
        );
        assert!(game.ewebs.guns.is_empty());
    }
}
