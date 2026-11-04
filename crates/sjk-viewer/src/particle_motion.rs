//! Allocation-free Raven `CParticle` motion and rotation state.
//!
//! `CParticle::UpdateOrigin` updates velocity before position
//! (`codemp/client/FxPrimitives.cpp:216-225`). Rotation follows
//! `CParticle::UpdateRotation` (`FxPrimitives.cpp:592-596`), including its
//! frame-time damping of angular velocity.

use super::*;

/// How fast a wandering particle's speed dies away, a share a second.
const WANDER_DRAG: f32 = 1.6;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Motion {
    origin: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
    rotation: f32,
    rotation_delta: f32,
    /// A wandering force (units a second², 0: none) pushing it about along smooth,
    /// tangled paths of its own (`wander_seed`), its own speed dying away against it: a
    /// blade skin's embers (`saber_sparks.rs`). The efx particles have none.
    wander: f32,
    wander_seed: f32,
    age: f32,

    updated_at: Instant,
}

impl Motion {
    pub(crate) fn new(
        origin: Vec3,
        velocity: Vec3,
        acceleration: Vec3,
        rotation: f32,
        rotation_delta: f32,
        starts_at: Instant,
    ) -> Self {
        Self {
            origin,
            velocity,
            acceleration,
            rotation,
            rotation_delta,
            wander: 0.0,
            wander_seed: 0.0,
            age: 0.0,

            updated_at: starts_at,
        }
    }

    /// This motion, pushed about by a wandering force of `amount` (units a second²) along a
    /// path of its own for `seed`.
    pub(crate) fn with_wander(mut self, amount: f32, seed: u32) -> Self {
        self.wander = amount;
        self.wander_seed = (seed % 10_007) as f32 * 0.618;
        self
    }

    /// The wandering force at its age: two waves on each axis, out of step, so the paths
    /// tangle without repeating soon.
    fn wander_force(&self) -> Vec3 {
        let (t, s) = (self.age, self.wander_seed);
        let wave = |a: f32, b: f32, p: f32| (t * a + p).sin() + 0.6 * (t * b + p * 1.7).sin();
        Vec3::new(
            wave(2.3, 5.1, s),
            wave(1.9, 4.4, s * 2.3 + 1.0),
            0.7 * wave(2.7, 6.3, s * 3.1 + 2.0),
        ) * self.wander
    }

    pub(crate) fn advance(&mut self, now: Instant) -> Sample {
        if let Some(step) = self.begin_step(now) {
            self.origin = step.predicted_origin;
        }
        self.sample()
    }

    pub(crate) fn begin_step(&mut self, now: Instant) -> Option<Step> {
        if now <= self.updated_at {
            return None;
        }
        let previous_origin = self.origin;
        let elapsed = now.duration_since(self.updated_at);
        let seconds = elapsed.as_secs_f32();
        let frame_millis = seconds * 1_000.0;
        self.velocity += self.acceleration * seconds;
        if self.wander > 0.0 {
            self.velocity += self.wander_force() * seconds;
            // The air holds it back: it darts where the force sends it, then slows.
            self.velocity *= (1.0 - WANDER_DRAG * seconds).max(0.0);
            self.age += seconds;
        }
        let predicted_origin = previous_origin + self.velocity * seconds;
        self.rotation += frame_millis * 0.01 * self.rotation_delta;
        self.rotation_delta *= 1.0 - frame_millis * 0.000_7;
        self.updated_at = now;
        Some(Step {
            previous_origin,
            predicted_origin,
            seconds,
        })
    }

    pub(crate) fn commit_origin(&mut self, origin: Vec3) {
        self.origin = origin;
    }

    pub(crate) fn add_acceleration_step(&mut self, seconds: f32) {
        self.velocity += self.acceleration * seconds;
    }

    pub(crate) fn reflect(&mut self, normal: Vec3, elasticity: f32) {
        self.velocity = (self.velocity - 2.0 * self.velocity.dot(normal) * normal) * elasticity;
    }

    pub(crate) fn stop_linear_motion(&mut self) {
        self.velocity = Vec3::ZERO;
        self.acceleration = Vec3::ZERO;
    }

    pub(crate) fn sample(&self) -> Sample {
        Sample {
            origin: self.origin,
            velocity: self.velocity,
            rotation_degrees: self.rotation,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Step {
    pub(crate) previous_origin: Vec3,
    pub(crate) predicted_origin: Vec3,
    pub(crate) seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sample {
    pub(crate) origin: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) rotation_degrees: f32,
}
