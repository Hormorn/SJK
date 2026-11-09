//! 3D sound (`s_spatial`, off by default; Sol's feature after VisionJK's `v_spatial`,
//! Blackwolf's idea, SJK's own engine): the sounds of the level heard from the
//! player's head through measured head filters, quieter and duller behind walls and
//! shut doors and coming round corners, and the room round the player ringing with
//! its traced echo.
//!
//! The work is `sjk-acoustics`' on its own threads: this module hands it the level
//! and, each frame, a snapshot (the hearing point, the head's axes, the world sounds
//! heard lately, the doors and lifts where they are shown), and forwards what comes
//! back to the mixer: each sound's acoustics ([`sjk_audio::Mixer::set_source_acoustics`])
//! and the room's response ([`sjk_audio::Mixer::set_room_response`]). The snapshot is
//! filled in place: a frame allocates nothing once its lists have grown. Off, the
//! threads are stopped, nothing is sent and the mixer's output is sample for sample
//! the plain one.

use glam::Vec3;
use sjk_acoustics::{Acoustics, Mover, Output, RoomSettings, RoomStats};
use sjk_audio::{Attenuation, PlayRequest, SourceId};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The switch (Settings > Sound, 3D sound).
pub(crate) const CVAR: &str = "s_spatial";
/// Heard through the head (headphones, 1) or panned (speakers, 0).
pub(crate) const HEADPHONES_CVAR: &str = "s_spatialHeadphones";
/// How loud the room rings, 1 as traced (0 to 2; 0 traces no room).
pub(crate) const REVERB_CVAR: &str = "s_spatialReverb";
pub(crate) const DEFAULT_REVERB: f64 = 1.0;

/// The player's head, between crouching and standing eye height above the origin
/// (`CROUCH_VIEWHEIGHT`, `DEFAULT_VIEWHEIGHT` of `bg_public.h`).
const EYE_LOW: f32 = 12.0;
const EYE_HIGH: f32 = 36.0;
/// A camera further than this from the head is not following the player (a
/// spectator, the drone, the free camera): it is where they hear from.
const EYE_FAR: f32 = 160.0;

/// World sounds remembered for the acoustics.
pub(crate) const TRACKED: usize = 64;
/// A sound not heard again for this long is forgotten.
const FORGET_AFTER: Duration = Duration::from_secs(8);

/// Where the player hears from with `s_spatial` on: their head as high as the camera
/// (so in first person the eyes, in third person not the orbiting camera), else the
/// camera when it is far from them or there is no predicted player.
pub(crate) fn hearing_point(camera: Vec3, player_origin: Option<Vec3>) -> Vec3 {
    let Some(origin) = player_origin else {
        return camera;
    };
    let head = Vec3::new(
        origin.x,
        origin.y,
        camera.z.clamp(origin.z + EYE_LOW, origin.z + EYE_HIGH),
    );
    if head.distance_squared(camera) > EYE_FAR * EYE_FAR {
        camera
    } else {
        head
    }
}

#[derive(Clone, Copy)]
struct Tracked {
    source: SourceId,
    origin: [f32; 3],
    heard: Instant,
}

/// The world sounds sent lately and the running acoustics.
#[derive(Default)]
pub(crate) struct Tracker {
    pub(crate) enabled: bool,
    pub(crate) headphones: bool,
    pub(crate) reverb: f32,
    /// What the mixer was last told.
    mixer_enabled: bool,
    sent_headphones: Option<bool>,
    sent_reverb: Option<f32>,
    listener_source: Option<SourceId>,
    sources: Vec<Tracked>,
    acoustics: Option<Acoustics>,
    /// The last room traced, for the console.
    pub(crate) room: Option<RoomStats>,
}

impl Tracker {
    /// Note a command the audio output sends: world sounds and where they move.
    pub(crate) fn observe(&mut self, command: &crate::audio_output::AudioCommand, now: Instant) {
        use crate::audio_output::AudioCommand;
        if !self.enabled {
            return;
        }
        match command {
            AudioCommand::Play(_, request)
            | AudioCommand::PlayFrom(_, request, ..)
            | AudioCommand::PlayTrack(_, request, ..)
            | AudioCommand::SetLoop(_, request, _) => self.hear(request, now),
            AudioCommand::SourcePosition(source, origin) => {
                if let Some(tracked) = self.sources.iter_mut().find(|t| t.source == *source) {
                    tracked.origin = *origin;
                }
            }
            AudioCommand::ListenerSource(source) => self.listener_source = *source,
            AudioCommand::StopEffects | AudioCommand::StopAll => self.clear(),
            _ => {}
        }
    }

    /// Remember a world sound's source and place (not the player's own).
    pub(crate) fn hear(&mut self, request: &PlayRequest, now: Instant) {
        let Some(origin) = request.origin else {
            return;
        };
        if request.attenuation == Attenuation::None || Some(request.source) == self.listener_source
        {
            return;
        }
        if let Some(tracked) = self
            .sources
            .iter_mut()
            .find(|tracked| tracked.source == request.source)
        {
            tracked.origin = origin;
            tracked.heard = now;
            return;
        }
        let tracked = Tracked {
            source: request.source,
            origin,
            heard: now,
        };
        if self.sources.len() < TRACKED {
            if self.sources.capacity() == 0 {
                self.sources.reserve_exact(TRACKED);
            }
            self.sources.push(tracked);
        } else if let Some(oldest) = self.sources.iter_mut().min_by_key(|t| t.heard) {
            *oldest = tracked;
        }
    }

    /// Forget every source (a new map, `soundstop`, switched off).
    pub(crate) fn clear(&mut self) {
        self.sources.clear();
    }

    /// Whether the mixer must be told the switch changed, and remember it was.
    pub(crate) fn switch_changed(&mut self) -> Option<bool> {
        (self.mixer_enabled != self.enabled).then(|| {
            self.mixer_enabled = self.enabled;
            if !self.enabled {
                self.clear();
                // Stops and joins the threads.
                self.acoustics = None;
                self.room = None;
                self.sent_headphones = None;
                self.sent_reverb = None;
            }
            self.enabled
        })
    }
}

/// What the frame's level is for the acoustics.
pub(crate) struct SpatialFrame<'a> {
    pub(crate) hearing: Vec3,
    /// The head's forward, right and up.
    pub(crate) axes: [Vec3; 3],
    /// The level, when in one.
    pub(crate) level: Option<&'a Arc<sjk_bsp::Bsp>>,
    pub(crate) movers: &'a mut dyn Iterator<Item = Mover>,
}

impl super::GameAudio {
    /// Hand this frame's 3D sound to the acoustics and what they found to the mixer.
    pub(crate) fn spatial_frame(&mut self, now: Instant, frame: SpatialFrame) {
        use crate::audio_output::AudioCommand;
        if let Some(enabled) = self.output.spatial.switch_changed() {
            self.output.send(AudioCommand::SpatialEffects(enabled));
            if !enabled {
                self.output.send(AudioCommand::RoomResponse(None));
            }
        }
        if !self.output.spatial.enabled {
            return;
        }
        let spatial = &mut self.output.spatial;
        let headphones = spatial.headphones;
        if spatial.sent_headphones != Some(headphones) {
            spatial.sent_headphones = Some(headphones);
            self.output
                .send(AudioCommand::SpatialHeadphones(headphones));
        }
        let spatial = &mut self.output.spatial;
        let reverb = spatial.reverb;
        if spatial.sent_reverb != Some(reverb) {
            spatial.sent_reverb = Some(reverb);
            self.output.send(AudioCommand::ReverbGain(reverb));
        }
        let spatial = &mut self.output.spatial;
        if spatial.acoustics.is_none() {
            spatial.acoustics = Acoustics::start();
        }
        let Tracker {
            acoustics, sources, ..
        } = spatial;
        let Some(acoustics) = acoustics.as_ref() else {
            return;
        };
        acoustics.set_level(frame.level);
        sources.retain(|tracked| now.saturating_duration_since(tracked.heard) <= FORGET_AFTER);
        acoustics.update(|snapshot| {
            snapshot.listener = frame.hearing;
            snapshot.axes = frame.axes;
            snapshot.sources.clear();
            snapshot.sources.extend(
                sources
                    .iter()
                    .map(|tracked| (tracked.source, Vec3::from_array(tracked.origin))),
            );
            snapshot.movers.clear();
            snapshot.movers.extend(&mut *frame.movers);
            snapshot.room = RoomSettings::default();
            snapshot.reverb = reverb > 0.0;
        });
        while let Some(output) = self
            .output
            .spatial
            .acoustics
            .as_ref()
            .and_then(Acoustics::poll)
        {
            match output {
                Output::Source(source, acoustics) => {
                    self.output
                        .send(AudioCommand::SourceAcoustics(source, acoustics));
                }
                Output::Room(response, stats) => {
                    self.output.spatial.room = response.is_some().then_some(stats);
                    self.output.send(AudioCommand::RoomResponse(response));
                }
                Output::Level {
                    triangles,
                    named,
                    millis,
                } => crate::log::progress(format_args!(
                    "3D sound: the level's walls are {triangles} triangles ({named} of a named \
                     material), ready in {millis} ms"
                )),
            }
        }
    }
}

impl crate::GpuState {
    /// Where the listener hears from this frame: the camera, or with `s_spatial` the
    /// player's head ([`hearing_point`]).
    pub(crate) fn audio_hearing_point(&self, spatial: bool) -> Vec3 {
        if !spatial {
            return self.camera_position;
        }
        let player = self
            .local_prediction
            .predicted_state()
            .filter(|_| !self.free_camera_active())
            .map(|state| Vec3::from_array(state.origin));
        hearing_point(self.camera_position, player)
    }

    /// The 3D sound's share of the frame ([`super::GameAudio::spatial_frame`]): in a
    /// map, its level and the doors and lifts where they are shown.
    pub(crate) fn spatial_audio_frame(
        &mut self,
        audio: &mut super::GameAudio,
        hearing: Vec3,
        axes: [Vec3; 3],
    ) {
        let in_map = self.live_session.is_some() || self.demo_session.is_some();
        let solids = self.local_prediction.presented_solids();
        let mut movers = solids
            .iter()
            .filter(|solid| solid.bounds.is_none() && solid.model > 0)
            .map(|solid| Mover {
                model: solid.model,
                origin: Vec3::from_array(solid.origin),
                axes: solid.rotated.then(|| solid.axes.map(Vec3::from_array)),
            });
        audio.spatial_frame(
            std::time::Instant::now(),
            SpatialFrame {
                hearing,
                axes,
                level: in_map.then_some(&self.bsp),
                movers: &mut movers,
            },
        );
    }
}

#[cfg(test)]
#[path = "audio_spatial_tests.rs"]
mod tests;
