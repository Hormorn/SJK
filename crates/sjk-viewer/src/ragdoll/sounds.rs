//! The Ragdoll's knocks, as the sandbox games' ragdolls have them: every part of the
//! body that hits the world is heard on its own, louder the harder it hit, both as
//! flesh (the game's own body falls, slams, punches and kicks) and as what it hit (the
//! game's footstep of that material: stone, metal, wood, grass, snow...), and a part
//! dragged along the ground scrapes on it.

use super::Cue;
use super::body::{
    Body, HEAD, Knock, L_ELBOW, L_FOOT, L_HAND, L_KNEE, R_ELBOW, R_FOOT, R_HAND, R_KNEE,
};
use sjk_client::animation_events::footsteps;

/// The flesh of each part, a soft knock and a hard one.
const TRUNK_SOFT: [&str; 3] = [
    "sound/player/bodyfall_human1.mp3",
    "sound/player/bodyfall_human2.mp3",
    "sound/player/bodyfall_human3.mp3",
];
const TRUNK_HARD: [&str; 2] = [
    "sound/effects/body_slam1.mp3",
    "sound/effects/body_slam2.mp3",
];
const HEAD_HARD: [&str; 1] = ["sound/effects/thud.mp3"];
const LIMB_SOFT: [&str; 4] = [
    "sound/weapons/melee/punch1.mp3",
    "sound/weapons/melee/punch2.mp3",
    "sound/weapons/melee/punch3.mp3",
    "sound/weapons/melee/punch4.mp3",
];
const LIMB_HARD: [&str; 5] = [
    "sound/weapons/melee/kick1.mp3",
    "sound/weapons/melee/kick2.mp3",
    "sound/weapons/melee/kick3.mp3",
    "sound/weapons/melee/kick4.mp3",
    "sound/weapons/melee/kick5.mp3",
];
/// A knock this hard (units a second into the surface) is heard, from the trunk or
/// the head, and from a limb, which flails onto the ground more lightly; this hard, it
/// is a hard one.
const KNOCK_SPEED: f32 = 110.0;
const LIMB_KNOCK_SPEED: f32 = 150.0;
const HARD_SPEED: f32 = 420.0;
/// At this speed a knock is at its loudest.
const LOUDEST_SPEED: f32 = 700.0;
/// The quietest knock heard, and how loud what it hit is beside the flesh.
const QUIETEST: f32 = 0.3;
const SURFACE_SHARE: f32 = 0.7;
/// A limb sounds this much quieter than the trunk.
const LIMB_SHARE: f32 = 0.75;
/// A part is heard at most every this many game server milliseconds, and at most this
/// many parts of a body in a frame (the hardest).
const KNOCK_GAP_MS: i64 = 120;
const KNOCKS_A_FRAME: usize = 3;
/// A part sliding faster than this (units a second) scrapes, at most every this many
/// milliseconds for a body, and a run's footstep from this fast.
const SCRAPE_SPEED: f32 = 120.0;
const SCRAPE_GAP_MS: i64 = 220;
const SCRAPE_RUN_SPEED: f32 = 280.0;
/// A scrape's loudness: at its slowest, and how much more at [`LOUDEST_SPEED`].
const SCRAPE_QUIETEST: f32 = 0.2;
const SCRAPE_RISE: f32 = 0.4;

/// The parts heard on their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Trunk,
    Head,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

const PARTS: usize = 6;

impl Part {
    fn of(joint: usize) -> Self {
        match joint {
            HEAD => Self::Head,
            L_ELBOW | L_HAND => Self::LeftArm,
            R_ELBOW | R_HAND => Self::RightArm,
            L_KNEE | L_FOOT => Self::LeftLeg,
            R_KNEE | R_FOOT => Self::RightLeg,
            _ => Self::Trunk,
        }
    }

    fn limb(self) -> bool {
        !matches!(self, Self::Trunk | Self::Head)
    }

    /// The flesh a knock of this part sounds as.
    fn flesh(self, hard: bool) -> &'static [&'static str] {
        match (self, hard) {
            (Self::Trunk, false) => &TRUNK_SOFT,
            (Self::Trunk, true) => &TRUNK_HARD,
            (Self::Head, false) => &LIMB_SOFT,
            (Self::Head, true) => &HEAD_HARD,
            (_, false) => &LIMB_SOFT,
            (_, true) => &LIMB_HARD,
        }
    }
}

/// When each part of a body was last heard, and its scrape (`i64::MIN`: never, so the
/// gaps since are taken saturating).
#[derive(Clone, Copy, Debug)]
pub(super) struct Heard {
    knocked: [i64; PARTS],
    scraped: i64,
    /// Turns which of a set's sounds plays next.
    turn: usize,
}

impl Default for Heard {
    fn default() -> Self {
        Self {
            knocked: [i64::MIN; PARTS],
            scraped: i64::MIN,
            turn: 0,
        }
    }
}

/// How loud a knock or a scrape at `speed` is, from the quietest at `least`.
fn loudness(speed: f32, least: f32) -> f32 {
    ((speed - least) / (LOUDEST_SPEED - least)).clamp(0.0, 1.0)
}

/// What `surface` (a footstep family) sounds like, walked or run on.
fn surface_sound(surface: u8, run: bool, turn: usize) -> &'static str {
    let family = usize::from(surface).min(footsteps::PATHS.len() - 1);
    footsteps::PATHS[family][usize::from(run)][turn % 4]
}

/// The sounds of the knocks and scrapes of `body`'s last steps, at game server `time`,
/// for the body of the player in `slot`.
pub(super) fn hear(heard: &mut Heard, body: &Body, time: i64, slot: u8, cues: &mut Vec<Cue>) {
    let points = body.points();
    // Each part's hardest knock.
    let mut hardest: [Option<Knock>; PARTS] = [None; PARTS];
    for knock in &body.knocks {
        let part = Part::of(knock.joint);
        let least = if part.limb() {
            LIMB_KNOCK_SPEED
        } else {
            KNOCK_SPEED
        };
        let slot = &mut hardest[part as usize];
        if knock.speed >= least && slot.is_none_or(|kept| knock.speed > kept.speed) {
            *slot = Some(*knock);
        }
    }
    let mut knocks: Vec<(Part, Knock)> = hardest
        .iter()
        .enumerate()
        .filter(|(index, _)| time.saturating_sub(heard.knocked[*index]) >= KNOCK_GAP_MS)
        .filter_map(|(index, knock)| knock.map(|knock| (part_at(index), knock)))
        .collect();
    knocks.sort_by(|a, b| b.1.speed.total_cmp(&a.1.speed));
    for (part, knock) in knocks.into_iter().take(KNOCKS_A_FRAME) {
        heard.knocked[part as usize] = time;
        heard.turn = heard.turn.wrapping_add(1);
        let hard = knock.speed >= HARD_SPEED;
        let least = if part.limb() {
            LIMB_KNOCK_SPEED
        } else {
            KNOCK_SPEED
        };
        let share = if part.limb() { LIMB_SHARE } else { 1.0 };
        let volume = (QUIETEST + (1.0 - QUIETEST) * loudness(knock.speed, least)) * share;
        let at = Some(points[knock.joint]);
        let flesh = part.flesh(hard);
        cues.push(Cue {
            path: flesh[heard.turn % flesh.len()],
            at,
            slot,
            volume,
        });
        cues.push(Cue {
            path: surface_sound(knock.surface, hard, heard.turn),
            at,
            slot,
            volume: volume * SURFACE_SHARE,
        });
    }
    // The fastest scrape.
    if time.saturating_sub(heard.scraped) >= SCRAPE_GAP_MS
        && let Some(scrape) = body
            .scrapes
            .iter()
            .filter(|scrape| scrape.speed >= SCRAPE_SPEED)
            .max_by(|a, b| a.speed.total_cmp(&b.speed))
    {
        heard.scraped = time;
        heard.turn = heard.turn.wrapping_add(1);
        cues.push(Cue {
            path: surface_sound(scrape.surface, scrape.speed >= SCRAPE_RUN_SPEED, heard.turn),
            at: Some(points[scrape.joint]),
            slot,
            volume: SCRAPE_QUIETEST + SCRAPE_RISE * loudness(scrape.speed, SCRAPE_SPEED),
        });
    }
}

fn part_at(index: usize) -> Part {
    [
        Part::Trunk,
        Part::Head,
        Part::LeftArm,
        Part::RightArm,
        Part::LeftLeg,
        Part::RightLeg,
    ][index]
}

/// Every sound a knock or a scrape may play, to register.
pub(super) fn paths() -> impl Iterator<Item = &'static str> {
    TRUNK_SOFT
        .into_iter()
        .chain(TRUNK_HARD)
        .chain(HEAD_HARD)
        .chain(LIMB_SOFT)
        .chain(LIMB_HARD)
        .chain(footsteps::PATHS.into_iter().flatten().flatten())
}

#[cfg(test)]
mod tests {
    use super::super::body::{JOINTS, PELVIS, Start};
    use super::*;

    #[test]
    fn a_body_never_heard_before_is_heard_on_its_first_knock() {
        let mut body = Body::new(&Start {
            moment: 1_000,
            origin: [0, 0, 24],
            velocity: [0, 0, 0],
            yaw: 0,
            scale: crate::ragdoll::STOCK_SCALE,
        });
        body.knocks.push(Knock {
            joint: PELVIS,
            speed: 300.0,
            surface: 0,
        });
        body.scrapes.push(Knock {
            joint: L_FOOT,
            speed: 200.0,
            surface: 0,
        });
        let mut heard = Heard::default();
        let mut cues = Vec::new();
        // A server's time well past zero: the gap since "never" must not wrap round.
        hear(&mut heard, &body, 5_000_000, 3, &mut cues);
        assert_eq!(cues.len(), 3, "flesh, surface and scrape");
        assert!(TRUNK_SOFT.contains(&cues[0].path));
        cues.clear();
        hear(&mut heard, &body, 5_000_050, 3, &mut cues);
        assert!(cues.is_empty(), "within the gaps");
    }

    #[test]
    fn every_joint_has_its_part_and_a_part_set_for_each_index() {
        for joint in 0..JOINTS {
            let part = Part::of(joint);
            assert_eq!(part_at(part as usize), part);
        }
        assert_eq!(Part::of(L_FOOT), Part::LeftLeg);
        assert_eq!(Part::of(HEAD), Part::Head);
    }

    #[test]
    fn harder_knocks_are_louder_and_every_family_has_a_sound() {
        assert!(loudness(200.0, KNOCK_SPEED) < loudness(500.0, KNOCK_SPEED));
        assert_eq!(loudness(5_000.0, KNOCK_SPEED), 1.0);
        for surface in 0..=20 {
            assert!(surface_sound(surface, true, 7).starts_with("sound/player/footsteps/"));
        }
    }
}
