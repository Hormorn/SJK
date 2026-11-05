//! Codemp impact-event to retail-effect presentation mapping.
//!
//! Missile dispatch mirrors `CG_EntityEvent` in `codemp/cgame/cg_event.c`
//! lines 2888-2956 and `CG_MissileHitWall`/`CG_MissileHitPlayer` in
//! `codemp/cgame/cg_weapons.c` lines 1980-2200. Saber selection mirrors
//! `cg_event.c` lines 2147-2379. Disruptor trace shots mirror `cg_event.c`
//! lines 2442-2530 and the code-built lines of `fx_disruptor.c`. The shared
//! client adapter supplies exact event-toggle de-duplication and `ByteToDir`
//! normals.

use sjk_client::{LegacyImpactEvent, LegacyImpactKind, LegacyImpactTracker};
use sjk_protocol::Snapshot;

const MAX_SNAPSHOT_IMPACTS: usize = 1_024;
/// The largest plan is the concussion alt shot: its wall effect, one ring run, two
/// beam lines and the disruptor miss it borrows. Rings are a run, not one visual
/// each, so a plan stays a few hundred bytes.
const MAX_VISUALS_PER_IMPACT: usize = 6;
/// `WP_FireConcussionAlt`'s `shotRange` (`g_weapon.c`, `int shotRange = 16384`
/// in EternalJK's `WP_FireConcussionAlt`; 8192 in stock OpenJK servers, which never
/// send a longer shot). A longer vector from a modified server is clamped to it.
const CONCUSSION_ALT_RANGE: f32 = 16_384.0;
/// Distance between the rings of a concussion alt shot (`cg_event.c:2869`).
const CONCUSSION_RING_SPACING: f32 = 64.0;
/// One ring per 64 units of the longest shot; bounds the effects one event spawns.
const MAX_CONCUSSION_RINGS: usize = (CONCUSSION_ALT_RANGE / CONCUSSION_RING_SPACING) as usize;

/// One visual primitive selected by codemp for an impact event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Visual {
    Effect {
        name: &'static str,
        direction: [f32; 3],
    },
    /// Evenly spaced copies of one effect (the concussion alt shot's rings).
    EffectRun(EffectRun),
    /// `FX_AddLine` with linear size and alpha (`fx_disruptor.c`).
    Line(Line),
}

impl Visual {
    /// How many effects or lines this visual spawns.
    pub(crate) fn count(&self) -> usize {
        match self {
            Visual::EffectRun(run) => run.count,
            _ => 1,
        }
    }
}

/// `count` plays of one effect at `start + unit * (index * spacing)`, all facing
/// `direction`; never more than [`MAX_CONCUSSION_RINGS`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EffectRun {
    pub(crate) name: &'static str,
    pub(crate) start: [f32; 3],
    pub(crate) unit: [f32; 3],
    pub(crate) spacing: f32,
    pub(crate) count: usize,
    pub(crate) direction: [f32; 3],
}

impl EffectRun {
    /// Where the `index`th copy plays.
    pub(crate) fn position(&self, index: usize) -> [f32; 3] {
        let along = index as f32 * self.spacing;
        std::array::from_fn(|axis| self.start[axis] + self.unit[axis] * along)
    }
}

/// One `FX_AddLine` call: a beam of constant colour whose half-width and
/// alpha fade linearly over `lifetime_millis`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Line {
    pub(crate) start: [f32; 3],
    pub(crate) end: [f32; 3],
    pub(crate) size: [f32; 2],
    pub(crate) alpha: [f32; 2],
    pub(crate) color: [f32; 3],
    pub(crate) lifetime_millis: u32,
    pub(crate) shader: &'static str,
}

const WHITE: [f32; 3] = [1.0; 3];
/// `YELLER` in `fx_disruptor.c:73`.
const YELLER: [f32; 3] = [0.8, 0.7, 0.0];

/// Fixed-size result; effect selection itself never allocates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plan {
    visuals: [Option<Visual>; MAX_VISUALS_PER_IMPACT],
    len: usize,
}

impl Plan {
    fn new() -> Self {
        Self {
            visuals: [None; MAX_VISUALS_PER_IMPACT],
            len: 0,
        }
    }

    fn effect(mut self, name: &'static str, direction: [f32; 3]) -> Self {
        if self.len < self.visuals.len() {
            self.visuals[self.len] = Some(Visual::Effect { name, direction });
            self.len += 1;
        }
        self
    }

    fn run(mut self, run: EffectRun) -> Self {
        if self.len < self.visuals.len() {
            self.visuals[self.len] = Some(Visual::EffectRun(run));
            self.len += 1;
        }
        self
    }

    fn repeated(self, name: &'static str, direction: [f32; 3], count: usize) -> Self {
        (0..count).fold(self, |plan, _| plan.effect(name, direction))
    }

    fn line(mut self, line: Line) -> Self {
        if self.len < self.visuals.len() {
            self.visuals[self.len] = Some(Visual::Line(line));
            self.len += 1;
        }
        self
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = Visual> + '_ {
        self.visuals[..self.len].iter().flatten().copied()
    }
}

/// EternalJK's `cg_fleshSparks` (JoF EJK `cg_xcvar.h`, archived): how many of stock's
/// red sparks a saber hit on a body plays, 0 to 7 ([`saber_hit`]).
pub(crate) const FLESH_SPARKS_CVAR: &str = "cg_fleshSparks";
/// Every spark stock plays; the setting's default, and what a larger value means.
pub(crate) const FLESH_SPARKS_ALL: u8 = 7;

/// `cg_fleshSparks` as [`plan`] takes it: clamped to 0..=7, 7 when unset.
pub(crate) fn flesh_sparks(console: Option<&crate::console::ViewerConsole>) -> u8 {
    console
        .and_then(|console| console.integer_cvar(FLESH_SPARKS_CVAR))
        .map_or(FLESH_SPARKS_ALL, |level| {
            level.clamp(0, i64::from(FLESH_SPARKS_ALL)) as u8
        })
}

/// Fixed-capacity decoded impact pool reused for every received snapshot.
pub(crate) struct Pool {
    tracker: LegacyImpactTracker,
    events: Vec<LegacyImpactEvent>,
    /// `cg_fleshSparks`, sampled as each snapshot is observed.
    flesh_sparks: u8,
    /// Where blades clashed since the frame's sabers were last drawn: the snapshots'
    /// saber blocks, and the walls' first touches ([`Self::clash_at`]), for the blade
    /// skins that react (`saber_smoke::Sink::clashed`).
    clashes: Vec<[f32; 3]>,
}

/// The most clashes kept between two frames.
const MOST_CLASHES: usize = 32;

impl Pool {
    pub(crate) fn new() -> Self {
        Self {
            tracker: LegacyImpactTracker::new(),
            events: Vec::with_capacity(MAX_SNAPSHOT_IMPACTS),
            flesh_sparks: FLESH_SPARKS_ALL,
            clashes: Vec::with_capacity(MOST_CLASHES),
        }
    }

    /// Set the `cg_fleshSparks` level the next impacts are planned at.
    pub(crate) fn set_flesh_sparks(&mut self, level: u8) {
        self.flesh_sparks = level.min(FLESH_SPARKS_ALL);
    }

    /// The `cg_fleshSparks` level impacts are planned at.
    pub(crate) fn flesh_sparks(&self) -> u8 {
        self.flesh_sparks
    }

    pub(crate) fn observe(&mut self, snapshot: &Snapshot) -> usize {
        self.tracker.observe(snapshot, &mut self.events);
        for index in 0..self.events.len() {
            let event = self.events[index];
            if matches!(event.kind, LegacyImpactKind::SaberBlock) {
                self.clash_at(event.origin);
            }
        }
        self.events.len()
    }

    /// Note a clash at `point` (a blade struck a wall).
    pub(crate) fn clash_at(&mut self, point: [f32; 3]) {
        if self.clashes.len() < MOST_CLASHES {
            self.clashes.push(point);
        }
    }

    /// Where blades clashed since the sabers were last drawn.
    pub(crate) fn clashes(&self) -> &[[f32; 3]] {
        &self.clashes
    }

    /// Forget the clashes: the frame's sabers took them.
    pub(crate) fn clear_clashes(&mut self) {
        self.clashes.clear();
    }

    pub(crate) fn event(&self, index: usize) -> Option<LegacyImpactEvent> {
        self.events.get(index).copied()
    }
}

/// Select the exact stock EFX graph(s) used by codemp. A zero-length plan is
/// intentional where codemp has no wall visual (Flechette alt-fire). `flesh_sparks` is
/// `cg_fleshSparks` ([`saber_hit`]); [`FLESH_SPARKS_ALL`] plays what stock does.
pub(crate) fn plan(event: LegacyImpactEvent, flesh_sparks: u8) -> Plan {
    let direction = event.direction;
    match event.kind {
        LegacyImpactKind::SaberHit => saber_hit(event.event_parameter, flesh_sparks, direction),
        LegacyImpactKind::SaberBlock => Plan::new().effect(
            if event.event_parameter == 0 {
                "blaster/deflect"
            } else {
                "saber/saber_block"
            },
            direction,
        ),
        // This is a screen-space latch, not an EFX primitive. The exact
        // cg_draw.c envelope and BSP visibility trace live in
        // sjk-client::LegacySaberClashFlare.
        LegacyImpactKind::SaberClashFlare => Plan::new(),
        LegacyImpactKind::DisruptorMainShot => disruptor_main_shot(event),
        LegacyImpactKind::DisruptorSniperShot => disruptor_sniper_shot(event),
        // `FX_DisruptorAltMiss` also lays a 4 s bezier smoke trail
        // (`fx_disruptor.c:89-139`); bezier primitives are not rendered yet.
        LegacyImpactKind::DisruptorSniperMiss => Plan::new().effect(
            if event.weapon != 0 {
                "disruptor/wall_impact"
            } else {
                "disruptor/alt_miss"
            },
            direction,
        ),
        LegacyImpactKind::DisruptorHit => Plan::new().effect(
            if event.weapon != 0 {
                "disruptor/flesh_impact"
            } else {
                "disruptor/wall_impact"
            },
            direction,
        ),
        LegacyImpactKind::ConcussionAltShot => concussion_alt_shot(event, direction),
        LegacyImpactKind::MissileHitPlayer => missile_player(event, direction),
        LegacyImpactKind::MissileHitWall | LegacyImpactKind::MissileHitMetal => {
            missile_wall(event, direction)
        }
    }
}

/// `EV_SABER_HIT` (`cg_event.c:2147-2379`): `eventParm` 0 is a hit on something else,
/// 16 "something special happened", 2 a really big hit, 3 a moderate one, any other a
/// hit on a body. Stock plays six sparks, one, one and three; EternalJK's
/// `cg_fleshSparks` (JoF EJK `cg_event.c`, `cg_fleshSparks.integer > N` before each
/// `FX_PlayEffectID`) plays fewer on a body as it goes down from 7: the special hit's
/// five extra at 2 and over and its first at 1, the big hit's at 3, the moderate hit's
/// at 4, a body hit's three at 5, 6 and 7. A hit on something else is not a body hit
/// and keeps its cut. The special hit's sound, also below 2 in EJK, is
/// [`sjk_client::LegacySoundAdapter::set_flesh_sparks`]'s.
fn saber_hit(parameter: u8, flesh_sparks: u8, direction: [f32; 3]) -> Plan {
    let above = |threshold: u8| usize::from(flesh_sparks > threshold);
    match parameter {
        0 => Plan::new().effect("saber/saber_cut", direction),
        2 => Plan::new().repeated("saber/blood_sparks_50_mp", direction, above(2)),
        3 => Plan::new().repeated("saber/blood_sparks_25_mp", direction, above(3)),
        16 => Plan::new().repeated("saber/blood_sparks_mp", direction, 5 * above(1) + above(0)),
        _ => Plan::new().repeated(
            "saber/blood_sparks_mp",
            direction,
            above(4) + above(5) + above(6),
        ),
    }
}

/// `FX_DisruptorMainShot` (`fx_disruptor.c:35-44`).
fn disruptor_main_shot(event: LegacyImpactEvent) -> Plan {
    Plan::new().line(Line {
        start: event.start,
        end: event.origin,
        size: [0.1, 6.0],
        alpha: [1.0, 0.0],
        color: WHITE,
        lifetime_millis: 150,
        shader: "gfx/effects/redLine",
    })
}

/// `FX_DisruptorAltShot` (`fx_disruptor.c:63-81`): the sniper beam plus a
/// yellow core when the shot was fully charged.
fn disruptor_sniper_shot(event: LegacyImpactEvent) -> Plan {
    let plan = Plan::new().line(Line {
        start: event.start,
        end: event.origin,
        size: [0.1, 10.0],
        alpha: [1.0, 0.0],
        color: WHITE,
        lifetime_millis: 175,
        shader: "gfx/effects/redLine",
    });
    if !event.full_charge {
        return plan;
    }
    plan.line(Line {
        start: event.start,
        end: event.origin,
        size: [0.1, 7.0],
        alpha: [1.0, 0.0],
        color: YELLER,
        lifetime_millis: 150,
        shader: "gfx/misc/whiteline2",
    })
}

/// `EV_CONC_ALT_IMPACT` (`cg_event.c:2860-2881`): rings every 64 units along the
/// shot, the wall hit, `FX_ConcAltShot`'s two lines (`fx_bryarpistol.c:244-259`) to
/// the last ring, and the disruptor's alt miss at the end.
fn concussion_alt_shot(event: LegacyImpactEvent, direction: [f32; 3]) -> Plan {
    let [x, y, z] = event.shot;
    let length = (x * x + y * y + z * z).sqrt();
    // A non-finite vector from a modified server draws nothing; a longer one than
    // `WP_FireConcussionAlt` can fire is clamped to its range.
    if !length.is_finite()
        || !event
            .start
            .iter()
            .chain(&event.ring_direction)
            .all(|v| v.is_finite())
    {
        return Plan::new();
    }
    let distance = length.min(CONCUSSION_ALT_RANGE);
    let unit = if length > 0.0 {
        [x / length, y / length, z / length]
    } else {
        [0.0, 0.0, 0.0]
    };
    let rings = EffectRun {
        name: "concussion/alt_ring",
        start: event.start,
        unit,
        spacing: CONCUSSION_RING_SPACING,
        // `for (dist = 0; dist < shotDist; dist += 64)`; the divisor is a power of two.
        count: ((distance / CONCUSSION_RING_SPACING).ceil() as usize).min(MAX_CONCUSSION_RINGS),
        direction: event.ring_direction,
    };
    let last = rings.position(rings.count.saturating_sub(1));
    Plan::new()
        .effect("concussion/explosion", direction)
        .run(rings)
        .line(Line {
            start: event.start,
            end: last,
            size: [0.1, 10.0],
            alpha: [1.0, 0.0],
            color: WHITE,
            lifetime_millis: 175,
            shader: "gfx/effects/blueLine",
        })
        .line(Line {
            start: event.start,
            end: last,
            size: [0.1, 7.0],
            alpha: [1.0, 0.0],
            // `BRIGHT` (`fx_bryarpistol.c:242`).
            color: [0.75, 0.5, 1.0],
            lifetime_millis: 150,
            shader: "gfx/misc/whiteline2",
        })
        .effect("disruptor/alt_miss", direction)
}

fn missile_player(event: LegacyImpactEvent, direction: [f32; 3]) -> Plan {
    let name = match (event.weapon, event.alternate) {
        (4 | 16 | 18, _) => "bryar/flesh_impact",
        (5 | 17, _) => "blaster/flesh_impact",
        (6, _) => "disruptor/alt_hit",
        (7, _) => "bowcaster/explosion",
        (8, true) => "repeater/concussion",
        (8, false) => "repeater/flesh_impact",
        (9, true) => "demp2/altdetonate",
        (9, false) => "demp2/flesh_impact",
        (10, _) => "flechette/flesh_impact",
        (11, _) => "rocket/explosion",
        (15, _) => "concussion/explosion",
        (12, _) => {
            return Plan::new()
                .effect("thermal/explosion", direction)
                .effect("thermal/shockwave", [0.0, 0.0, 1.0]);
        }
        _ => return Plan::new(),
    };
    Plan::new().effect(name, direction)
}

fn missile_wall(event: LegacyImpactEvent, direction: [f32; 3]) -> Plan {
    let name = match (event.weapon, event.alternate) {
        (4 | 16, true) if event.charge >= 4 => "bryar/wall_impact3",
        (4 | 16, true) if event.charge >= 2 => "bryar/wall_impact2",
        (4 | 16 | 18, _) => "bryar/wall_impact",
        (5 | 17, _) => "blaster/wall_impact",
        (6, _) => "disruptor/alt_miss",
        (7, _) => "bowcaster/explosion",
        (8, true) => "repeater/concussion",
        (8, false) => "repeater/wall_impact",
        (9, true) => "demp2/altdetonate",
        (9, false) => "demp2/wall_impact",
        (10, true) => return Plan::new(),
        (10, false) => "flechette/wall_impact",
        (11, _) => "rocket/explosion",
        (15, _) => "concussion/explosion",
        (12, _) => {
            return Plan::new()
                .effect("thermal/explosion", direction)
                .effect("thermal/shockwave", [0.0, 0.0, 1.0]);
        }
        _ => return Plan::new(),
    };
    Plan::new().effect(name, direction)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every impact as stock plays it (`cg_fleshSparks 7`).
    fn plan(event: LegacyImpactEvent) -> Plan {
        super::plan(event, FLESH_SPARKS_ALL)
    }

    fn event(kind: LegacyImpactKind, weapon: u8) -> LegacyImpactEvent {
        LegacyImpactEvent {
            entity_number: 1,
            event: 84,
            kind,
            origin: [10.0, 20.0, 30.0],
            start: [0.0; 3],
            direction: [0.0, 0.0, 1.0],
            event_parameter: 0,
            weapon,
            alternate: false,
            charge: 0,
            full_charge: false,
            shot: [0.0; 3],
            ring_direction: [1.0, 0.0, 0.0],
            vehicle_weapon: 0,
        }
    }

    fn shot(shot: [f32; 3]) -> LegacyImpactEvent {
        LegacyImpactEvent {
            shot,
            alternate: true,
            ..event(LegacyImpactKind::ConcussionAltShot, 15)
        }
    }

    fn rings(plan: &Plan) -> usize {
        plan.iter()
            .map(|visual| match visual {
                Visual::EffectRun(run) if run.name == "concussion/alt_ring" => run.count,
                _ => 0,
            })
            .sum()
    }

    /// The rings, lines and effects of the shot the way a visual per ring would list
    /// them, `WP_FireConcussionAlt`'s `for (dist = 0; dist < shotDist; dist += 64)`.
    fn expected_rings(start: [f32; 3], shot: [f32; 3], range: f32) -> Vec<[f32; 3]> {
        let length = shot.iter().map(|v| v * v).sum::<f32>().sqrt();
        let unit = shot.map(|v| v / length);
        let distance = length.min(range);
        let mut spots = Vec::new();
        let mut along = 0.0_f32;
        while along < distance {
            spots.push(std::array::from_fn(|axis| start[axis] + unit[axis] * along));
            along += 64.0;
        }
        spots
    }

    /// `cg_fleshSparks` 0 to 7 gates each spark as JoF EJK's `EV_SABER_HIT` does, and 7
    /// plays exactly what stock (and SJK before the setting) plays.
    #[test]
    fn flesh_sparks_gate_each_saber_hit_on_a_body() {
        let hit = |parameter: u8, level: u8| {
            let mut event = event(LegacyImpactKind::SaberHit, 0);
            event.event_parameter = parameter;
            super::plan(event, level)
                .iter()
                .map(|visual| match visual {
                    Visual::Effect { name, .. } => name,
                    other => panic!("a saber hit plays effects only: {other:?}"),
                })
                .collect::<Vec<_>>()
        };
        // Per level 0..=7: the special hit (16), a really big hit (2), a moderate
        // hit (3) and a plain body hit (1).
        let expected: [(u8, &str, [usize; 8]); 4] = [
            (16, "saber/blood_sparks_mp", [0, 1, 6, 6, 6, 6, 6, 6]),
            (2, "saber/blood_sparks_50_mp", [0, 0, 0, 1, 1, 1, 1, 1]),
            (3, "saber/blood_sparks_25_mp", [0, 0, 0, 0, 1, 1, 1, 1]),
            (1, "saber/blood_sparks_mp", [0, 0, 0, 0, 0, 1, 2, 3]),
        ];
        for (parameter, name, counts) in expected {
            for (level, count) in counts.into_iter().enumerate() {
                let names = hit(parameter, level as u8);
                assert_eq!(names.len(), count, "eventParm {parameter} at {level}");
                assert!(names.iter().all(|effect| *effect == name), "{names:?}");
            }
        }
        // A hit on something else is no body hit: its cut stays at every level.
        for level in 0..=FLESH_SPARKS_ALL {
            assert_eq!(hit(0, level), ["saber/saber_cut"]);
        }
        // 7, the default, is stock's: six, one, one and three sparks.
        assert_eq!(hit(16, 7).len(), 6);
        assert_eq!(hit(1, 7).len(), 3);
        assert_eq!(hit(5, 7).len(), 3, "any other eventParm is a body hit");
    }

    #[test]
    fn flesh_sparks_read_clamped_and_default_to_every_spark() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(flesh_sparks(None), FLESH_SPARKS_ALL);
        assert_eq!(flesh_sparks(Some(&console)), FLESH_SPARKS_ALL);
        for (value, level) in [("0", 0), ("3", 3), ("7", 7), ("12", 7), ("-2", 0)] {
            assert!(console.set_cvar(FLESH_SPARKS_CVAR, value));
            assert_eq!(flesh_sparks(Some(&console)), level, "{value}");
        }
        let mut pool = Pool::new();
        assert_eq!(pool.flesh_sparks(), FLESH_SPARKS_ALL);
        pool.set_flesh_sparks(2);
        assert_eq!(pool.flesh_sparks(), 2);
    }

    #[test]
    fn plans_stay_small() {
        // The by-value builder moves a `Plan` per call; a ring per slot made it ~10 KB.
        assert!(std::mem::size_of::<Plan>() < 1_024);
    }

    #[test]
    fn ordinary_impacts_keep_their_visuals() {
        let mut flesh = event(LegacyImpactKind::MissileHitPlayer, 5);
        flesh.direction = [1.0, 0.0, 0.0];
        assert_eq!(
            plan(flesh).iter().collect::<Vec<_>>(),
            [Visual::Effect {
                name: "blaster/flesh_impact",
                direction: [1.0, 0.0, 0.0]
            }]
        );
        let thermal = plan(event(LegacyImpactKind::MissileHitWall, 12));
        assert_eq!(
            thermal.iter().collect::<Vec<_>>(),
            [
                Visual::Effect {
                    name: "thermal/explosion",
                    direction: [0.0, 0.0, 1.0]
                },
                Visual::Effect {
                    name: "thermal/shockwave",
                    direction: [0.0, 0.0, 1.0]
                },
            ]
        );
        let mut sniper = event(LegacyImpactKind::DisruptorSniperShot, 0);
        sniper.full_charge = true;
        let lines: Vec<_> = plan(sniper).iter().collect();
        assert_eq!(lines.len(), 2);
        assert!(matches!(
            lines[0],
            Visual::Line(Line {
                shader: "gfx/effects/redLine",
                end: [10.0, 20.0, 30.0],
                ..
            })
        ));
        let mut blood = event(LegacyImpactKind::SaberHit, 0);
        blood.event_parameter = 16;
        assert_eq!(plan(blood).iter().count(), 6);
        assert!(
            plan(event(LegacyImpactKind::SaberClashFlare, 0))
                .iter()
                .next()
                .is_none()
        );
    }

    #[test]
    fn rings_follow_the_shot_up_to_its_range() {
        assert_eq!(rings(&plan(shot([640.0, 0.0, 0.0]))), 10);
        assert_eq!(rings(&plan(shot([650.0, 0.0, 0.0]))), 11);
        assert_eq!(rings(&plan(shot([8_192.0, 0.0, 0.0]))), 128);
        assert_eq!(rings(&plan(shot([0.0, 0.0, 0.0]))), 0);
    }

    #[test]
    fn a_shot_of_the_reference_range_keeps_its_whole_effect() {
        // `shotRange` 16384 in EternalJK's `WP_FireConcussionAlt`.
        let long = plan(shot([0.0, 16_384.0, 0.0]));
        assert_eq!(rings(&long), MAX_CONCUSSION_RINGS);
        assert_eq!(MAX_CONCUSSION_RINGS, 256);
        let visuals: Vec<_> = long.iter().collect();
        assert_eq!(visuals.len(), 5);
        // Both lines end at the last ring (16384 - 64), `FX_ConcAltShot(origin2, spot)`.
        for visual in &visuals[2..4] {
            let Visual::Line(line) = visual else {
                panic!("{visual:?}")
            };
            assert_eq!(line.end, [0.0, 16_320.0, 0.0]);
        }
    }

    #[test]
    fn a_longer_shot_is_clamped_to_the_range() {
        let clamped = plan(shot([0.0, 1.0e9, 0.0]));
        assert_eq!(rings(&clamped), MAX_CONCUSSION_RINGS);
        let Visual::Line(line) = clamped.iter().nth(2).unwrap() else {
            panic!()
        };
        assert_eq!(line.end, [0.0, 16_320.0, 0.0]);
        assert_eq!(
            clamped.iter().collect::<Vec<_>>(),
            plan(shot([0.0, 16_384.0, 0.0])).iter().collect::<Vec<_>>()
        );
        assert_eq!(
            rings(&plan(shot([16_385.0, 0.0, 0.0]))),
            MAX_CONCUSSION_RINGS
        );
    }

    #[test]
    fn non_finite_shots_draw_nothing() {
        for bad in [
            [f32::INFINITY, 0.0, 0.0],
            [f32::NEG_INFINITY, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0],
            [0.0, f32::MAX, f32::MAX], // the squared length overflows to infinity
        ] {
            assert_eq!(plan(shot(bad)).iter().count(), 0, "{bad:?}");
        }
        let mut start = shot([640.0, 0.0, 0.0]);
        start.start = [f32::NAN, 0.0, 0.0];
        assert_eq!(plan(start).iter().count(), 0);
        let mut ring = shot([640.0, 0.0, 0.0]);
        ring.ring_direction = [0.0, f32::INFINITY, 0.0];
        assert_eq!(plan(ring).iter().count(), 0);
    }

    #[test]
    fn a_ring_run_lists_the_same_positions_as_a_visual_per_ring() {
        let mut event = shot([300.0, -400.0, 120.0]);
        event.start = [5.0, 6.0, 7.0];
        let visuals: Vec<_> = plan(event).iter().collect();
        let Visual::EffectRun(run) = visuals[1] else {
            panic!("{:?}", visuals[1])
        };
        let want = expected_rings(event.start, event.shot, CONCUSSION_ALT_RANGE);
        assert_eq!(run.count, want.len());
        for (index, spot) in want.iter().enumerate() {
            assert_eq!(run.position(index), *spot, "ring {index}");
        }
        assert_eq!(run.direction, event.ring_direction);
        assert_eq!(
            visuals.iter().map(Visual::count).sum::<usize>(),
            want.len() + 4
        );
    }
}
