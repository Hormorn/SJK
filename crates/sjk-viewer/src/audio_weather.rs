//! The rain and wind loops (`s_weatherSounds`, [`crate::weather::sound`]): played in both
//! ears with the frame's loops, at the levels the weather hears this frame.

use super::*;
use crate::weather::sound::{Levels, RAIN_LOOP, RAIN_VOLUME, WIND_LOOP, WIND_VOLUME};

/// The loops' sources: no entity's, so nothing mutes or moves them.
const RAIN_SOURCE: SourceId = SourceId(0x6300_0000);
const WIND_SOURCE: SourceId = SourceId(0x6300_0001);

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
            self.register_effect_paths(vfs, [RAIN_LOOP, WIND_LOOP]);
            self.weather_registered = Some(self.generation());
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
