//! The Prop gun (Sol's idea, 01/11/2026): a toy fake weapon. While it is on, the player
//! holds a gun that a pk3 defines instead of their real weapon, in first person on the
//! hand and in third person in the hand, and attack fires its bolt instead of reaching
//! the game server. A bolt that hits a player who has the toy on too knocks them down
//! as a [ragdoll](crate::ragdoll), as the Monitor does (always opt in).
//!
//! SJK ships no gun: each is `propguns/<id>.gun` in a pk3 or the game folder, a text
//! file naming its models and sounds ([`Gun::parse`]). Other SJK players see and hear a
//! gun only when they have the same file. What a player holds and fires travels through
//! the [toy relay](crate::player_identity::relay_toy_event): `hold_<id>`, `put` and
//! `fire_<number>_<moment>_<x>_<y>_<z>_<dx>_<dy>_<dz>_<id>` ([`Event`]). Every client
//! flies each bolt the same, in a straight line from its moment; the struck player's
//! client alone decides a hit, and the Ragdoll tells the fall. See `docs/client.md`,
//! "Prop gun", and the glue in `propgun_frame.rs`.

use glam::{Quat, Vec3};
use sjk_vfs::VirtualFileSystem;
use std::time::{Duration, Instant};

/// The switch: holding the gun, firing it and being hit by others' bolts.
pub(crate) const COMMAND: &str = "toy_propgun";
/// `propgun [<id>|next|prev]`: list the guns or choose one.
pub(crate) const PICK_COMMAND: &str = "propgun";
pub(crate) const PICK_HELP: &str = "propgun: list the prop guns; propgun <id>, next or prev chooses the one the Prop gun toy holds (guns are propguns/<id>.gun in pk3s)";
/// The toy's id in the relay.
pub(crate) const TOY: &str = "propgun";
/// `cg_toyPropGunShow` 0 hides other players' prop guns and bolts on this PC.
pub(crate) const SHOW_CVAR: &str = "cg_toyPropGunShow";
/// The chosen gun's id, archived.
pub(crate) const GUN_CVAR: &str = "cl_propGun";
/// Where the guns are, and their extension.
pub(crate) const FOLDER: &str = "propguns";
pub(crate) const EXTENSION: &str = ".gun";
/// The longest gun id.
pub(crate) const MAX_ID: usize = 24;
/// The mystery box's Ray Gun (SJK's own, bundled, 11/11/2026; `mystery_box.rs`): a gun
/// read from [`RAYGUN_FILE`], not `propguns/`, so it is never chosen as a prop gun. A box's
/// opener holds it [`PropGun::raygun_until`] whether the toy is on or not, and its bolts
/// knock down any SJK player in play, the toy on or not, once: a player it struck is
/// shielded from every Ray Gun for [`RAY_SHIELD_MS`].
pub(crate) const RAYGUN: &str = "raygun";
pub(crate) const RAYGUN_FILE: &str = "sjk/raygun.gun";
/// How long a player struck by a Ray Gun is shielded from them all (game server ms).
pub(crate) const RAY_SHIELD_MS: i64 = 30_000;
/// One relay event every this long (the relay takes 40 a second a player, shared).
pub(crate) const SEND_GAP: Duration = Duration::from_millis(30);
/// What a player holds is told again this often, and forgotten when not told this long.
pub(crate) const RETELL: Duration = Duration::from_secs(15);
pub(crate) const HOLD_EXPIRES: Duration = Duration::from_secs(40);
/// A shot heard this late (game server milliseconds) is not shown at all.
pub(crate) const LATE_MS: i64 = 1_500;
/// The numbers of a player's shots go round this many.
pub(crate) const NUMBERS: u8 = 100;
/// At most this many bolts fly at once (the oldest goes).
pub(crate) const MAX_BOLTS: usize = 64;
/// How near a body a bolt passes to strike it.
pub(crate) const REACH: f32 = 6.0;
/// A body struck is not struck again for this long (game server milliseconds).
pub(crate) const IMMUNE_MS: i64 = 250;
/// The lift every blow has, units a second upwards.
pub(crate) const LIFT: f32 = 220.0;
/// The bolt's way is tried against the body every this many units.
pub(crate) const SWEEP_PIECE: f32 = 4.0;

/// A model of a gun and how it sits: moved by `offset` (its own axes, X forward, Y left,
/// Z up), turned by `angles` (pitch, yaw, roll degrees) and grown by `scale`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) model: String,
    pub(crate) offset: Vec3,
    pub(crate) angles: Vec3,
    pub(crate) scale: f32,
}

impl Placed {
    fn new(model: String) -> Self {
        Self {
            model,
            offset: Vec3::ZERO,
            angles: Vec3::ZERO,
            scale: 1.0,
        }
    }

    /// The turn of `angles`, as the game's `AnglesToAxis`.
    pub(crate) fn rotation(&self) -> Quat {
        Quat::from_array(sjk_client::legacy_angles_to_quaternion(
            self.angles.to_array(),
        ))
    }
}

/// One gun, as its `.gun` file says.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Gun {
    pub(crate) id: String,
    pub(crate) name: String,
    /// First person, on the hand.
    pub(crate) view: Placed,
    /// Third person, in the hand.
    pub(crate) world: Placed,
    /// The shot others hear at the shooter (one at random).
    pub(crate) fire: Vec<String>,
    /// The shooter's own shot, in both ears (else [`Self::fire`]).
    pub(crate) fire_own: Vec<String>,
    /// Where a bolt meets the world or a body.
    pub(crate) impact: Vec<String>,
    /// Played to the player when they take the gun out.
    pub(crate) raise: Option<String>,
    /// The bolt's model, drawn along its way (none: a light only).
    pub(crate) bolt_model: Option<String>,
    pub(crate) bolt_scale: f32,
    pub(crate) bolt_color: [f32; 3],
    pub(crate) bolt_light: f32,
    pub(crate) bolt_speed: f32,
    pub(crate) bolt_life_ms: i64,
    /// An effect (`.efx`) where the bolt meets the world.
    pub(crate) impact_effect: Option<String>,
    /// An effect where the bolt leaves, along its way, at each shot.
    pub(crate) fire_effect: Option<String>,
    /// An effect started along the bolt's way as it flies (a trail).
    pub(crate) bolt_effect: Option<String>,
    /// A sound looped at the bolt as it flies.
    pub(crate) bolt_sound: Option<String>,
    /// The view model's `tag_flash`, on its own axes (read from the model, not the file).
    pub(crate) view_muzzle: Option<Vec3>,
    /// Between two shots, milliseconds.
    pub(crate) refire_ms: i64,
    /// How hard a bolt pushes the body it strikes, units a second.
    pub(crate) push: f32,
}

/// Whether `id` may name a gun: 1 to [`MAX_ID`] of `a` to `z`, `0` to `9` and `_`.
pub(crate) fn valid_id(id: &str) -> bool {
    (1..=MAX_ID).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The words of one line: plain words or "quoted" ones, after a `//` nothing.
fn words(line: &str) -> Vec<String> {
    let line = line.split("//").next().unwrap_or("");
    let mut words = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            words.push(chars.by_ref().take_while(|&c| c != '"').collect());
        } else {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                word.push(c);
                chars.next();
            }
            words.push(word);
        }
    }
    words
}

fn number(values: &[String], index: usize, key: &str) -> Result<f32, String> {
    values
        .get(index)
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("{key} needs a number"))
}

fn vector(values: &[String], key: &str) -> Result<Vec3, String> {
    Ok(Vec3::new(
        number(values, 0, key)?,
        number(values, 1, key)?,
        number(values, 2, key)?,
    ))
}

fn path(values: &[String], key: &str) -> Result<String, String> {
    values
        .first()
        .filter(|value| !value.is_empty() && !value.contains(".."))
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| format!("{key} needs a path"))
}

impl Gun {
    /// Read gun `id` from its file's `text`: one setting a line, its name then its
    /// values (`//` starts a comment). `view` and `world` are needed.
    pub(crate) fn parse(id: &str, text: &str) -> Result<Self, String> {
        if !valid_id(id) {
            return Err(format!(
                "{id}: a gun's name is 1 to {MAX_ID} of a to z, 0 to 9 and _"
            ));
        }
        let mut view: Option<Placed> = None;
        let mut world: Option<Placed> = None;
        let mut gun = Self {
            id: id.to_owned(),
            name: id.replace('_', " "),
            view: Placed::new(String::new()),
            world: Placed::new(String::new()),
            fire: Vec::new(),
            fire_own: Vec::new(),
            impact: Vec::new(),
            raise: None,
            bolt_model: None,
            bolt_scale: 1.0,
            bolt_color: [0.4, 1.0, 0.3],
            bolt_light: 120.0,
            bolt_speed: 1_800.0,
            bolt_life_ms: 3_000,
            impact_effect: None,
            fire_effect: None,
            bolt_effect: None,
            bolt_sound: None,
            view_muzzle: None,
            refire_ms: 350,
            push: 900.0,
        };
        for (line_number, line) in text.lines().enumerate() {
            let words = words(line);
            let Some((key, values)) = words.split_first() else {
                continue;
            };
            let key = key.to_ascii_lowercase();
            let failed = |why: String| format!("{id}.gun line {}: {why}", line_number + 1);
            let placed = |slot: &mut Option<Placed>, part: &str| -> Result<(), String> {
                let entry = slot.get_or_insert_with(|| Placed::new(String::new()));
                match part {
                    "" => entry.model = path(values, &key)?,
                    "offset" => entry.offset = vector(values, &key)?,
                    "angles" => entry.angles = vector(values, &key)?,
                    "scale" => entry.scale = number(values, 0, &key)?.clamp(0.05, 20.0),
                    _ => return Err(format!("unknown setting {key}")),
                }
                Ok(())
            };
            let result = match key.as_str() {
                "name" => {
                    gun.name = values.join(" ");
                    Ok(())
                }
                "view" | "viewoffset" | "viewangles" | "viewscale" => {
                    placed(&mut view, &key["view".len()..])
                }
                "world" | "worldoffset" | "worldangles" | "worldscale" => {
                    placed(&mut world, &key["world".len()..])
                }
                "fire" => path(values, &key).map(|p| gun.fire.push(p)),
                "fireown" => path(values, &key).map(|p| gun.fire_own.push(p)),
                "impact" => path(values, &key).map(|p| gun.impact.push(p)),
                "raise" => path(values, &key).map(|p| gun.raise = Some(p)),
                "boltmodel" => path(values, &key).map(|p| gun.bolt_model = Some(p)),
                "boltscale" => {
                    number(values, 0, &key).map(|n| gun.bolt_scale = n.clamp(0.05, 20.0))
                }
                "boltcolor" => vector(values, &key)
                    .map(|c| gun.bolt_color = c.clamp(Vec3::ZERO, Vec3::ONE).to_array()),
                "boltlight" => {
                    number(values, 0, &key).map(|n| gun.bolt_light = n.clamp(0.0, 400.0))
                }
                "boltspeed" => {
                    number(values, 0, &key).map(|n| gun.bolt_speed = n.clamp(200.0, 8_000.0))
                }
                "boltlife" => number(values, 0, &key)
                    .map(|n| gun.bolt_life_ms = (n as i64).clamp(100, 10_000)),
                "impacteffect" => path(values, &key).map(|p| gun.impact_effect = Some(p)),
                "fireeffect" => path(values, &key).map(|p| gun.fire_effect = Some(p)),
                "bolteffect" => path(values, &key).map(|p| gun.bolt_effect = Some(p)),
                "boltsound" => path(values, &key).map(|p| gun.bolt_sound = Some(p)),
                "refire" => {
                    number(values, 0, &key).map(|n| gun.refire_ms = (n as i64).clamp(80, 5_000))
                }
                "push" => number(values, 0, &key)
                    .map(|n| gun.push = n.clamp(0.0, crate::ragdoll::MAX_PUSH as f32)),
                _ => Err(format!("unknown setting {key}")),
            };
            result.map_err(failed)?;
        }
        gun.view = view
            .filter(|placed| !placed.model.is_empty())
            .ok_or_else(|| format!("{id}.gun names no view model"))?;
        gun.world = world
            .filter(|placed| !placed.model.is_empty())
            .ok_or_else(|| format!("{id}.gun names no world model"))?;
        Ok(gun)
    }

    /// Every sound the gun plays, to register once.
    pub(crate) fn sounds(&self) -> impl Iterator<Item = &str> {
        self.fire
            .iter()
            .chain(&self.fire_own)
            .chain(&self.impact)
            .chain(&self.raise)
            .chain(&self.bolt_sound)
            .map(String::as_str)
    }

    /// Every model the gun draws, to load with the map.
    pub(crate) fn models(&self) -> impl Iterator<Item = &str> {
        [
            Some(&self.view.model),
            Some(&self.world.model),
            self.bolt_model.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(String::as_str)
    }
}

/// Where `model` (an MD3) has its `tag_flash`, in its first frame.
fn muzzle_of(vfs: &VirtualFileSystem, model: &str) -> Option<Vec3> {
    let asset = vfs.read(model).ok().flatten()?;
    let md3 = sjk_model::Md3::parse(&asset.bytes).ok()?;
    md3.tags
        .first()?
        .iter()
        .find(|tag| tag.name.eq_ignore_ascii_case("tag_flash"))
        .map(|tag| Vec3::from_array(tag.origin))
}

/// The guns in the game's files, by id, and the problems with the others (one line
/// each).
pub(crate) fn catalogue(vfs: &VirtualFileSystem) -> (Vec<Gun>, Vec<String>) {
    let mut guns = Vec::new();
    let mut problems = Vec::new();
    let mut names = vfs.list_files(FOLDER, EXTENSION);
    names.sort();
    // The box's Ray Gun, read from its own place (a `propguns/raygun.gun` is skipped).
    if let Some(asset) = vfs.read(RAYGUN_FILE).ok().flatten() {
        match Gun::parse(RAYGUN, &String::from_utf8_lossy(&asset.bytes)) {
            Ok(gun) => guns.push(gun),
            Err(problem) => problems.push(problem),
        }
    }
    for name in names {
        let Some(id) = name.strip_suffix(EXTENSION).filter(|id| !id.contains('/')) else {
            continue;
        };
        let id = id.to_ascii_lowercase();
        if id == RAYGUN {
            continue;
        }
        let Some(asset) = vfs.read(&format!("{FOLDER}/{name}")).ok().flatten() else {
            continue;
        };
        match Gun::parse(&id, &String::from_utf8_lossy(&asset.bytes)).map(|mut gun| {
            gun.view_muzzle = muzzle_of(vfs, &gun.view.model);
            gun
        }) {
            Ok(gun) => guns.push(gun),
            Err(problem) => problems.push(problem),
        }
    }
    (guns, problems)
}

/// One shot as the relay carries it: the shooter's number for it, the game server time
/// it left the gun, where (whole units) and which way (thousandths).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shot {
    pub(crate) number: u8,
    pub(crate) moment: i32,
    pub(crate) from: [i32; 3],
    pub(crate) way: [i16; 3],
}

impl Shot {
    pub(crate) fn new(number: u8, moment: i32, from: Vec3, way: Vec3) -> Self {
        let way = way.normalize_or(Vec3::X);
        Self {
            number,
            moment,
            from: from.to_array().map(|c| c.round() as i32),
            way: way.to_array().map(|c| (c * 1000.0).round() as i16),
        }
    }

    pub(crate) fn origin(&self) -> Vec3 {
        Vec3::from_array(self.from.map(|c| c as f32))
    }

    pub(crate) fn direction(&self) -> Vec3 {
        Vec3::from_array(self.way.map(|c| f32::from(c) / 1000.0)).normalize_or(Vec3::X)
    }
}

/// What the relay carries for the toy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    /// The sender holds gun `id`.
    Hold(String),
    /// The sender put their gun away.
    Put,
    /// The sender fired gun `id`.
    Fire(Shot, String),
}

impl Event {
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Hold(id) => format!("hold_{id}"),
            Self::Put => "put".to_owned(),
            Self::Fire(shot, id) => {
                let [x, y, z] = shot.from;
                let [dx, dy, dz] = shot.way;
                format!(
                    "fire_{}_{}_{x}_{y}_{z}_{dx}_{dy}_{dz}_{id}",
                    shot.number, shot.moment
                )
            }
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        if text == "put" {
            return Some(Self::Put);
        }
        if let Some(id) = text.strip_prefix("hold_") {
            return valid_id(id).then(|| Self::Hold(id.to_owned()));
        }
        let rest = text.strip_prefix("fire_")?;
        let mut parts = rest.splitn(9, '_');
        let number: u8 = parts.next()?.parse().ok().filter(|n| *n < NUMBERS)?;
        let moment: i32 = parts.next()?.parse().ok()?;
        let mut from = [0_i32; 3];
        for c in &mut from {
            *c = parts
                .next()?
                .parse()
                .ok()
                .filter(|c: &i32| c.abs() <= 65_536)?;
        }
        let mut way = [0_i16; 3];
        for c in &mut way {
            *c = parts
                .next()?
                .parse()
                .ok()
                .filter(|c: &i16| c.abs() <= 1_000)?;
        }
        let id = parts.next()?;
        if !valid_id(id) || way == [0, 0, 0] {
            return None;
        }
        Some(Self::Fire(
            Shot {
                number,
                moment,
                from,
                way,
            },
            id.to_owned(),
        ))
    }
}

/// How a bolt's flight ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum End {
    /// It met the world at this time, there, facing the way `normal` says.
    Wall { time: i64, at: Vec3, normal: Vec3 },
    /// It struck a body (the local player's, or one a hit was heard for).
    Body { time: i64, at: Vec3 },
    /// It flew its life out.
    Spent,
}

/// One bolt in flight.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bolt {
    pub(crate) shooter: u8,
    /// Its gun's index in the catalogue.
    pub(crate) gun: usize,
    pub(crate) shot: Shot,
    pub(crate) speed: f32,
    pub(crate) life_ms: i64,
    /// How far its flight has been stepped (game server ms).
    pub(crate) stepped: i64,
    pub(crate) end: Option<End>,
    /// The SJK players told of it (the shooter's own bolts).
    pub(crate) told: u64,
    /// When its trail effect was last started (game server ms).
    pub(crate) trailed: i64,
}

impl Bolt {
    pub(crate) fn new(shooter: u8, gun: usize, shot: Shot, speed: f32, life_ms: i64) -> Self {
        Self {
            shooter,
            gun,
            shot,
            speed,
            life_ms,
            stepped: i64::from(shot.moment),
            end: None,
            told: 0,
            trailed: i64::MIN,
        }
    }

    /// Where it is at `time` along its line (before any end).
    pub(crate) fn at(&self, time: i64) -> Vec3 {
        let flown = (time - i64::from(self.shot.moment)).clamp(0, self.life_ms) as f32 / 1000.0;
        self.shot.origin() + self.shot.direction() * self.speed * flown
    }

    /// When it is done with: ended (shown a moment for its impact), or spent.
    pub(crate) fn last_time(&self) -> i64 {
        match self.end {
            Some(End::Wall { time, .. } | End::Body { time, .. }) => time,
            _ => i64::from(self.shot.moment) + self.life_ms,
        }
    }

    /// Whether it is still drawn at `time`.
    pub(crate) fn flying(&self, time: i64) -> bool {
        time >= i64::from(self.shot.moment) && time < self.last_time()
    }
}

/// The player's own body as a bolt may strike it this frame.
pub(crate) enum Target {
    /// Standing: their box and the points of a body standing there.
    Standing {
        low: Vec3,
        high: Vec3,
        points: [Vec3; crate::ragdoll::JOINTS],
    },
    /// Limp: the ragdoll's points.
    Limp([Vec3; crate::ragdoll::JOINTS]),
}

impl Target {
    fn nearest(points: &[Vec3; crate::ragdoll::JOINTS], centre: Vec3) -> u8 {
        (0..crate::ragdoll::JOINTS)
            .min_by(|a, b| {
                points[*a]
                    .distance_squared(centre)
                    .total_cmp(&points[*b].distance_squared(centre))
            })
            .unwrap_or(0) as u8
    }

    /// The joint a bolt at `centre` strikes, if it reaches the body.
    pub(crate) fn struck(&self, centre: Vec3) -> Option<u8> {
        match self {
            Self::Standing { low, high, points } => (centre.clamp(*low, *high).distance(centre)
                <= REACH)
                .then(|| Self::nearest(points, centre)),
            Self::Limp(points) => {
                let joint = Self::nearest(points, centre);
                (points[usize::from(joint)].distance(centre) <= REACH + 6.0).then_some(joint)
            }
        }
    }

    /// The first point of the way from `from` to `to` where a bolt strikes the body.
    pub(crate) fn struck_along(&self, from: Vec3, to: Vec3) -> Option<(Vec3, u8)> {
        let pieces = ((to - from).length() / SWEEP_PIECE).ceil().max(1.0) as u32;
        (1..=pieces).find_map(|piece| {
            let at = from.lerp(to, piece as f32 / pieces as f32);
            self.struck(at).map(|joint| (at, joint))
        })
    }
}

/// The blow a bolt going `direction` with gun push `push` gives.
pub(crate) fn blow(direction: Vec3, push: f32) -> [i32; 3] {
    (direction.normalize_or(Vec3::X) * push + Vec3::Z * LIFT)
        .clamp_length_max(crate::ragdoll::MAX_PUSH as f32)
        .to_array()
        .map(|c| c.round() as i32)
}

/// A sound to play this frame: at a point, or in both ears.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cue {
    pub(crate) path: String,
    pub(crate) at: Option<Vec3>,
    /// The player's own (heard even with others' guns hidden).
    pub(crate) mine: bool,
}

/// An effect to start this frame (an impact, a muzzle, a trail).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Splash {
    pub(crate) effect: String,
    pub(crate) at: Vec3,
    pub(crate) normal: Vec3,
}

/// Pick one of `paths` for shot `number` (the same on every client).
pub(crate) fn one_of(paths: &[String], number: u8) -> Option<&str> {
    (!paths.is_empty()).then(|| paths[usize::from(number) % paths.len()].as_str())
}

/// The toy's state.
#[derive(Default)]
pub(crate) struct PropGun {
    pub(crate) on: bool,
    pub(crate) was_on: bool,
    /// The guns read from the files, and the files' generation they were read for.
    pub(crate) guns: Vec<Gun>,
    pub(crate) guns_for: Option<usize>,
    /// The chosen gun's id.
    pub(crate) chosen: Option<String>,
    /// Attack held now, and pressed since the last frame.
    pub(crate) held: bool,
    pub(crate) pressed: bool,
    /// The game server time of the player's last shot.
    pub(crate) last_shot: Option<i64>,
    pub(crate) next_number: u8,
    /// What others hold: their gun's id and when they last told it.
    pub(crate) holders: Vec<Option<(String, Instant)>>,
    /// The gun last told as held, to whom, and when it is told again.
    pub(crate) told_hold: Option<String>,
    pub(crate) hold_told: u64,
    pub(crate) retell_at: Option<Instant>,
    /// Events still to tell (a put), to the slots in the mask.
    pub(crate) outbox: Vec<(Event, u64)>,
    pub(crate) tell_at: Option<Instant>,
    pub(crate) bolts: Vec<Bolt>,
    pub(crate) cues: Vec<Cue>,
    pub(crate) splashes: Vec<Splash>,
    /// Where the first-person gun's muzzle was drawn this frame, if it was.
    pub(crate) view_muzzle: Option<Vec3>,
    /// This frame's flying bolts' loops: their source, sound and where.
    pub(crate) loops: Vec<(sjk_audio::SourceId, String, [f32; 3])>,
    /// When the player was last struck, and by which bolts (shooter, number).
    pub(crate) last_struck: Option<i64>,
    pub(crate) struck_by: Vec<(u8, u8)>,
    pub(crate) server: Option<std::net::SocketAddr>,
    pub(crate) server_id: Option<i32>,
    pub(crate) time: i64,
    pub(crate) duel: bool,
    pub(crate) unseen: bool,
    pub(crate) sounds_registered: Option<u64>,
    pub(crate) gun_cvar_read: bool,
    /// The box's Ray Gun is held until then ([`RAYGUN`]).
    pub(crate) raygun_until: Option<Instant>,
    /// The game server time the player's own Ray Gun shield ends ([`RAY_SHIELD_MS`]).
    pub(crate) ray_shield_until: Option<i64>,
    /// When the shield rose, for its glow.
    pub(crate) ray_shield_since: i64,
    /// Others' Ray Gun shields, by slot: when each rose and ends (game server time), from
    /// the hub's hit events.
    pub(crate) others_ray_shield: Vec<Option<(i64, i64)>>,
    /// For the world shots: every player holds the chosen gun.
    #[cfg(test)]
    pub(crate) shot_all: bool,
}

impl PropGun {
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// The chosen gun, if it is among the guns read (never the box's Ray Gun).
    pub(crate) fn gun(&self) -> Option<(usize, &Gun)> {
        let id = self.chosen.as_deref().filter(|id| *id != RAYGUN)?;
        self.guns.iter().enumerate().find(|(_, gun)| gun.id == id)
    }

    /// The gun the player holds now: the box's Ray Gun while it lasts, else with the toy
    /// on the gun chosen.
    pub(crate) fn holding(&self) -> Option<(usize, &Gun)> {
        if self.raygun_held() {
            return self
                .guns
                .iter()
                .enumerate()
                .find(|(_, gun)| gun.id == RAYGUN);
        }
        self.on.then(|| self.gun()).flatten()
    }

    /// The box's Ray Gun is in the player's hand now.
    pub(crate) fn raygun_held(&self) -> bool {
        self.raygun_until
            .is_some_and(|until| Instant::now() < until)
    }

    /// The prop guns a player may choose: every gun read but the box's Ray Gun.
    pub(crate) fn choosable(&self) -> impl Iterator<Item = &Gun> {
        self.guns.iter().filter(|gun| gun.id != RAYGUN)
    }

    /// Whether gun `gun` (an index) is the box's Ray Gun.
    pub(crate) fn is_raygun(&self, gun: usize) -> bool {
        self.guns.get(gun).is_some_and(|gun| gun.id == RAYGUN)
    }

    /// Whether the player's own Ray Gun shield holds at game server time `time`.
    pub(crate) fn ray_shielded(&self, time: i64) -> bool {
        self.ray_shield_until.is_some_and(|until| time < until)
    }

    pub(crate) fn gun_index(&self, id: &str) -> Option<usize> {
        self.guns.iter().position(|gun| gun.id == id)
    }

    /// The gun another player in `slot` holds (by index), if this PC has it.
    pub(crate) fn held_by(&self, slot: usize) -> Option<usize> {
        let (id, _) = self.holders.get(slot)?.as_ref()?;
        self.gun_index(id)
    }

    /// Read attack from the outgoing command and keep it from the server while a gun
    /// is held.
    pub(crate) fn shape_command(&mut self, command: &mut sjk_protocol::UserCommand) {
        let attack = command.buttons & FIRE_BUTTONS != 0;
        if self.holding().is_none() {
            self.held = false;
            return;
        }
        if attack && !self.held {
            self.pressed = true;
        }
        self.held = attack;
        command.buttons &= !FIRE_BUTTONS;
    }

    /// The preview command (the predicted look) without attack while a gun is held.
    pub(crate) fn shape_preview(&self, command: &mut sjk_protocol::UserCommand) {
        if self.holding().is_some() {
            command.buttons &= !FIRE_BUTTONS;
        }
    }

    /// Everything of the server goes: bolts, holders, what was told.
    pub(crate) fn leave(&mut self) {
        self.bolts.clear();
        self.holders.clear();
        self.told_hold = None;
        self.hold_told = 0;
        self.retell_at = None;
        self.outbox.clear();
        self.struck_by.clear();
        self.last_struck = None;
        self.last_shot = None;
        self.raygun_until = None;
        self.ray_shield_until = None;
        self.others_ray_shield.clear();
    }

    /// Keep a bolt, the oldest going past [`MAX_BOLTS`].
    pub(crate) fn add_bolt(&mut self, bolt: Bolt) {
        if self.bolts.len() >= MAX_BOLTS {
            self.bolts.remove(0);
        }
        self.bolts.push(bolt);
    }

    /// Whether the shooter's shot `number` is already known.
    pub(crate) fn knows(&self, shooter: u8, shot: &Shot) -> bool {
        self.bolts
            .iter()
            .any(|bolt| bolt.shooter == shooter && bolt.shot == *shot)
    }
}

/// `BUTTON_ATTACK` and `BUTTON_ALT_ATTACK`.
pub(crate) const FIRE_BUTTONS: u16 = 1 | 128;

/// The way `pitch` and `yaw` (Quake degrees, positive pitch down) look.
pub(crate) fn looking(pitch: f32, yaw: f32) -> Vec3 {
    let (pitch, yaw) = (pitch.to_radians(), yaw.to_radians());
    Vec3::new(
        yaw.cos() * pitch.cos(),
        yaw.sin() * pitch.cos(),
        -pitch.sin(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUN: &str = r#"
// A test gun
name "Zapper"
view models/propguns/zapper/view.md3
viewOffset 1 2 3
viewScale 0.8
world models/propguns/zapper/world.md3
worldAngles 0 90 0
fire sound/propguns/zapper/fire.wav
fireOwn sound/propguns/zapper/fire_own.wav
impact sound/propguns/zapper/impact.wav
boltColor 0.2 1 0.3
boltSpeed 2000
fireEffect propguns/zapper/muzzle
boltEffect propguns/zapper/trail
boltSound sound/propguns/zapper/flight.wav
refire 300
"#;

    #[test]
    fn a_gun_file_reads_its_models_sounds_and_numbers() {
        let gun = Gun::parse("zapper", GUN).unwrap();
        assert_eq!(gun.name, "Zapper");
        assert_eq!(gun.view.model, "models/propguns/zapper/view.md3");
        assert_eq!(gun.view.offset, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(gun.view.scale, 0.8);
        assert_eq!(gun.fire_effect.as_deref(), Some("propguns/zapper/muzzle"));
        assert_eq!(gun.bolt_effect.as_deref(), Some("propguns/zapper/trail"));
        assert_eq!(
            gun.bolt_sound.as_deref(),
            Some("sound/propguns/zapper/flight.wav")
        );
        assert!(gun.sounds().any(|path| path.ends_with("flight.wav")));
        assert_eq!(gun.world.angles, Vec3::new(0.0, 90.0, 0.0));
        assert_eq!(gun.fire, ["sound/propguns/zapper/fire.wav"]);
        assert_eq!(gun.fire_own.len(), 1);
        assert_eq!(gun.bolt_speed, 2000.0);
        assert_eq!(gun.refire_ms, 300);
        assert_eq!(gun.sounds().count(), 4);
        assert_eq!(gun.models().count(), 2);
    }

    #[test]
    fn a_gun_file_without_models_or_with_a_bad_name_or_line_is_refused() {
        assert!(Gun::parse("zapper", "view a.md3").is_err());
        assert!(Gun::parse("Zap-Gun", GUN).is_err());
        let bad = format!("{GUN}\nboltSpeed fast");
        assert!(Gun::parse("zapper", &bad).unwrap_err().contains("line"));
        assert!(Gun::parse("zapper", &format!("{GUN}\nwhat 1")).is_err());
        assert!(Gun::parse("zapper", &format!("{GUN}\nfire ../x.wav")).is_err());
    }

    #[test]
    fn events_go_to_text_and_back() {
        let shot = Shot::new(
            7,
            123_456,
            Vec3::new(10.4, -20.0, 30.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        for event in [
            Event::Hold("zapper2".to_owned()),
            Event::Put,
            Event::Fire(shot, "zapper".to_owned()),
        ] {
            assert_eq!(
                Event::parse(&event.text()),
                Some(event.clone()),
                "{}",
                event.text()
            );
        }
        assert_eq!(
            Event::Fire(shot, "zapper".to_owned()).text(),
            "fire_7_123456_10_-20_30_0_1000_0_zapper"
        );
        assert_eq!(Event::parse("fire_100_1_0_0_0_1000_0_0_zapper"), None);
        assert_eq!(Event::parse("fire_1_1_0_0_0_0_0_0_zapper"), None);
        assert_eq!(Event::parse("hold_Ray"), None);
    }

    #[test]
    fn a_bolt_flies_in_a_line_for_its_life() {
        let shot = Shot::new(0, 1_000, Vec3::ZERO, Vec3::X);
        let bolt = Bolt::new(3, 0, shot, 2_000.0, 500);
        assert_eq!(bolt.at(1_000), Vec3::ZERO);
        assert!((bolt.at(1_250).x - 500.0).abs() < 0.01);
        assert!((bolt.at(9_000).x - 1_000.0).abs() < 0.01);
        assert!(bolt.flying(1_400) && !bolt.flying(1_500) && !bolt.flying(999));
    }

    #[test]
    fn a_bolt_strikes_a_standing_body_it_passes_near_and_not_one_it_misses() {
        let origin = Vec3::new(100.0, 0.0, 24.0);
        let target = Target::Standing {
            low: origin + Vec3::new(-15.0, -15.0, -24.0),
            high: origin + Vec3::new(15.0, 15.0, 40.0),
            points: crate::ragdoll::standing(origin, 0.0, crate::ragdoll::STOCK_SCALE),
        };
        let (at, joint) = target
            .struck_along(Vec3::new(0.0, 0.0, 40.0), Vec3::new(200.0, 0.0, 40.0))
            .unwrap();
        assert!(at.x < 90.0 && at.x > 70.0, "{at}");
        assert!(usize::from(joint) < crate::ragdoll::JOINTS);
        assert!(
            target
                .struck_along(Vec3::new(0.0, 60.0, 40.0), Vec3::new(200.0, 60.0, 40.0))
                .is_none()
        );
    }

    #[test]
    fn a_blow_goes_the_bolts_way_and_up_within_the_ragdolls_hardest() {
        let push = blow(Vec3::X, 900.0);
        assert_eq!(push, [900, 0, 220]);
        let hard = blow(Vec3::X, 9_000.0);
        let length = Vec3::from_array(hard.map(|c| c as f32)).length();
        assert!(length <= crate::ragdoll::MAX_PUSH as f32 + 1.0);
    }

    #[test]
    fn attack_is_kept_from_the_server_only_while_a_gun_is_held() {
        let mut toy = PropGun::default();
        toy.guns.push(Gun::parse("zapper", GUN).unwrap());
        toy.chosen = Some("zapper".to_owned());
        let mut command = sjk_protocol::UserCommand {
            buttons: 1 | 4,
            ..Default::default()
        };
        toy.shape_command(&mut command);
        assert_eq!(command.buttons, 1 | 4, "off: untouched");
        toy.on = true;
        toy.shape_command(&mut command);
        assert_eq!(command.buttons, 4);
        assert!(toy.pressed && toy.held);
    }

    #[test]
    fn the_box_ray_gun_is_held_while_it_lasts_whatever_the_switch_and_never_chosen() {
        let mut toy = PropGun::default();
        toy.guns.push(Gun::parse("zapper", GUN).unwrap());
        toy.guns.push(Gun::parse(RAYGUN, GUN).unwrap());
        toy.chosen = Some(RAYGUN.to_owned());
        assert!(toy.gun().is_none(), "the Ray Gun is no prop gun");
        assert_eq!(toy.choosable().count(), 1);
        assert!(toy.holding().is_none());
        toy.raygun_until = Some(Instant::now() + Duration::from_secs(30));
        let (index, gun) = toy.holding().expect("the Ray Gun");
        assert_eq!(gun.id, RAYGUN);
        assert!(toy.is_raygun(index) && !toy.is_raygun(0));
        let mut command = sjk_protocol::UserCommand {
            buttons: 1,
            ..Default::default()
        };
        toy.shape_command(&mut command);
        assert_eq!(command.buttons, 0, "attack fires the Ray Gun");
        toy.raygun_until = Some(Instant::now() - Duration::from_secs(1));
        assert!(toy.holding().is_none(), "gone at its end");
        toy.ray_shield_until = Some(1_000 + RAY_SHIELD_MS);
        assert!(toy.ray_shielded(1_000) && !toy.ray_shielded(1_000 + RAY_SHIELD_MS));
    }

    #[test]
    fn one_of_picks_the_same_sound_for_the_same_shot() {
        let paths = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(one_of(&paths, 4), Some("b"));
        assert_eq!(one_of(&[], 4), None);
    }
}
