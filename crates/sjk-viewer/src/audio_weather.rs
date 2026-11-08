//! The rain and wind loops (`s_weatherSounds`, [`crate::weather::sound`]): played in both
//! ears with the frame's loops, at the levels the weather hears this frame.

use super::*;
use crate::weather::lightning::{Cue, GUSTS, THUNDER_CLOSE, THUNDER_DISTANT};
use crate::weather::sound::{Levels, RAIN_LOOP, RAIN_VOLUME, WIND_LOOP, WIND_VOLUME};

/// The loops' sources: no entity's, so nothing mutes or moves them.
const RAIN_SOURCE: SourceId = SourceId(0x6300_0000);
const WIND_SOURCE: SourceId = SourceId(0x6300_0001);
/// Thunder and single gusts: one source, each sound on channel 0 (never replaced).
const CUE_SOURCE: SourceId = SourceId(0x6300_0002);

impl GameAudio {
    /// The rain and wind heard this frame; their loops are registered the first time
    /// they are heard in this audio generation, from `vfs`.
    pub(crate) fn set_weather(&mut self, vfs: Option<&VirtualFileSystem>, levels: Levels) {
        self.weather_levels = levels;
        let heard = levels.rain > 0.0 || levels.wind > 0.0;
        if self.weather_sounds
            && heard
            && self.weather_registered != Some(self.generation())
            && let Some(vfs) = vfs
        {
            self.register_effect_paths(
                vfs,
                [RAIN_LOOP, WIND_LOOP]
                    .into_iter()
                    .chain(THUNDER_CLOSE)
                    .chain(THUNDER_DISTANT)
                    .chain(GUSTS),
            );
            self.weather_registered = Some(self.generation());
        }
    }

    /// Play the weather's thunder and gusts (`cues`), in both ears, while
    /// `s_weatherSounds` is on; they are dropped otherwise.
    pub(crate) fn play_weather_cues(&mut self, cues: impl Iterator<Item = Cue>) {
        for cue in cues {
            if !self.weather_sounds || cue.volume <= 0.0 {
                continue;
            }
            let Some(handle) = self.find_handle(cue.path) else {
                continue;
            };
            self.output.send(AudioCommand::Play(
                handle,
                PlayRequest {
                    origin: None,
                    source: CUE_SOURCE,
                    channel: ChannelId(0),
                    volume: cue.volume,
                    attenuation: sjk_audio::Attenuation::None,
                },
            ));
        }
    }

    /// The rain and wind loops in this frame's set, while `s_weatherSounds` is on.
    pub(super) fn send_weather_loops(&mut self) {
        if !self.weather_sounds {
            return;
        }
        let Levels { rain, wind } = self.weather_levels;
        for (path, level, source) in [
            (RAIN_LOOP, rain * RAIN_VOLUME, RAIN_SOURCE),
            (WIND_LOOP, wind * WIND_VOLUME, WIND_SOURCE),
        ] {
            if level <= 0.0 {
                continue;
            }
            let Some(handle) = self.find_handle(path) else {
                continue;
            };
            self.output.send(AudioCommand::SetLoop(
                handle,
                PlayRequest {
                    origin: None,
                    source,
                    channel: ChannelId(0),
                    volume: level,
                    attenuation: sjk_audio::Attenuation::None,
                },
                [0.0; 3],
            ));
        }
    }
}
