//! The Saber Lab's sounds on the stage (`docs/client.md`, "Saber Lab"): the blade's
//! ignition and retraction as the tab lights it and puts it out, and the swings the
//! model's animations cue as a match's do ([`sjk_client::animation_events`]): the lab's
//! blade skin's own swing over the hilt's or the stock one, its own ignition and
//! retraction in place of the hilt's or the stock ones. Nothing plays while the tab mutes
//! them, and no swing while the blade is off. Here too the camera's way round the model
//! ([`crate::menu_backdrop::orbit`]), eased in as the tab shows and out as it is left.

use super::*;
use crate::GameAudio;
use sjk_audio::{ChannelId, SourceId};
use sjk_client::animation_events::{Cue, Cursor, Random};

/// The source the lab's sounds play from, their own (the interface cues' is the last).
const SOURCE: SourceId = SourceId(u32::MAX - 1);
/// The channel ignition and retraction share, so that one cuts the other short; swings
/// play on the free one.
const SWITCH_CHANNEL: ChannelId = ChannelId(1);
const FREE_CHANNEL: ChannelId = ChannelId(0);
/// The ignition SJK plays for a hilt without `soundOn`, and `WP_SaberSetDefaults`'
/// retraction (`bg_saberLoad.c:418`).
const STOCK_ON: &str = "sound/weapons/saber/saberon.mp3";
const STOCK_OFF: &str = "sound/weapons/saber/enemy_saber_off.wav";
/// The hum when the skin names none the packs hold (a hilt's own is not in its
/// definition here).
const STOCK_HUM: &str = "sound/weapons/saber/saberhum1.wav";
/// How loud the hum is heard: the camera is a match's third person away or nearer, and
/// the lab plays it in both ears.
const HUM_VOLUME: f32 = 0.5;
/// The most snaps heard a frame, however many bolts went.
const SNAPS_A_FRAME: u32 = 2;
/// The game's saber blocks, `{1-9}` of them (`WP_SaberDoClash`'s sounds), one at random
/// for the Clash button.
const BLOCK_PREFIX: &str = "sound/weapons/saber/saberblock";
/// The stock swings, `saberhup1` to `saberhup8` (`EV_SABER_ATTACK`).
const SWING_PREFIX: &str = "sound/weapons/saber/saberhup";

/// Seconds the camera takes to go round to where the tab looks from, and back.
const VIEW_EASE: f32 = 0.6;
/// How far over the model's origin the camera turns round: about its chest.
const VIEW_TARGET_ABOVE_ORIGIN: f32 = 8.0;
/// A standing player's eye over the origin (`DEFAULT_VIEWHEIGHT`), which a match's third
/// person camera follows.
const VIEW_HEIGHT: f32 = 36.0;

/// Where the lab's sounds are: the animations' cues already heard, whether the blade was
/// lit (`None` outside the tab, so coming to it plays no ignition), the attack last heard
/// and the swings' dice.
#[derive(Default)]
pub(super) struct Sounds {
    cursor: Cursor,
    lit: Option<bool>,
    /// When the attack last sounded began, so that each sounds once.
    attack: Option<Instant>,
    random: Random,
    /// The hand's bolt bursts let go since the last frame heard, and its emitter's
    /// sputter ending (`sabers.rs`), for a skin's crackle.
    snaps: u32,
    /// The Clash button struck the blades (`sabers.rs`): a block is heard.
    clashed: bool,
}

impl Sounds {
    /// Note `bursts` of the hand's bolts let go (`sabers.rs`), to snap with.
    pub(super) fn snap(&mut self, bursts: u32) {
        self.snaps = self.snaps.saturating_add(bursts);
    }

    /// Note that the Clash button struck the blades (`sabers.rs`), to sound a block.
    pub(super) fn clash(&mut self) {
        self.clashed = true;
    }
}

/// How far the camera has gone round to where the tab looks from (0 not at all, 1 all the
/// way) and behind the model, what it turns round, the model's eye and the way it faces,
/// and when it was last eased.
#[derive(Default)]
pub(super) struct Ease {
    share: f32,
    chase: f32,
    build: f32,
    target: Option<Vec3>,
    eye: Option<(Vec3, f32)>,
    at: Option<Instant>,
}

/// Where the Saber Lab's camera looks from ([`crate::menu_backdrop::orbit`]): round
/// `target` as `view` says, `share` of the way (eased); `chase` of the way to behind
/// the model, its eye `eye` and facing `facing` (radians), as a match's third person;
/// `build` of the way to the builder's framing, the model on the right of the picture
/// ([`crate::menu_backdrop::framed_yaw`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LabCamera {
    pub(crate) target: Vec3,
    pub(crate) view: crate::console::collection_panel::LabView,
    pub(crate) share: f32,
    pub(crate) chase: f32,
    pub(crate) build: f32,
    pub(crate) eye: Vec3,
    pub(crate) facing: f32,
}

/// The game's flourish as the tab lights or puts out the blade: whether it was lit, and
/// the move playing and how far into it (seconds at the tab's speed).
#[derive(Default)]
pub(super) struct Flourish {
    lit: Option<bool>,
    playing: Option<(&'static str, f32)>,
}

impl MenuStage {
    /// The flourish the model plays this frame, `step` seconds (at the tab's speed) after
    /// the last, while the moves wait: started as the blade is lit or put out, when the
    /// tab asks for it, until its animation ends.
    pub(super) fn lab_flourish(
        &mut self,
        held: crate::player_menu::SaberStyle,
        step: f32,
    ) -> Option<&'static str> {
        let Some(lab) = self.lab else {
            self.flourish = Flourish::default();
            return None;
        };
        let was = self.flourish.lit.replace(lab.lit);
        if was.is_some_and(|was| was != lab.lit) {
            self.flourish.playing = lab.flourish.then(|| {
                (
                    crate::player_menu::force_pose::lab_flourish(lab.lit, held),
                    0.0,
                )
            });
        }
        let (animation, played) = self.flourish.playing?;
        let length = self
            .actor
            .as_ref()
            .and_then(|actor| actor.preview.config.get(animation))
            .map_or(0.0, travel_seconds);
        if played >= length {
            self.flourish.playing = None;
            return None;
        }
        self.flourish.playing = Some((animation, played + step));
        Some(animation)
    }

    /// Ease the camera round to where the tab looks from while it shows, and back once it
    /// is left; the view and what it turns round are kept for the way back.
    pub(super) fn ease_lab_view(&mut self, now: Instant) {
        if let Some(lab) = self.lab {
            self.lab_view = lab.view;
        }
        if let Some(actor) = self.actor.as_ref().filter(|_| self.lab.is_some()) {
            self.lab_ease.target = Some(actor.origin + Vec3::Z * VIEW_TARGET_ABOVE_ORIGIN);
            self.lab_ease.eye = Some((actor.origin + Vec3::Z * VIEW_HEIGHT, actor.yaw));
        }
        let ease = &mut self.lab_ease;
        let step = ease.at.replace(now).map_or(0.0, |at| {
            now.saturating_duration_since(at).as_secs_f32() / VIEW_EASE
        });
        let toward = |share: f32, on: bool| {
            if on {
                (share + step).min(1.0)
            } else {
                (share - step).max(0.0)
            }
        };
        ease.share = toward(ease.share, self.lab.is_some());
        ease.chase = toward(ease.chase, self.lab.is_some_and(|lab| lab.chase));
        ease.build = toward(ease.build, self.lab.is_some_and(|lab| lab.build));
    }

    /// Where the camera looks from while the tab's view is on it or easing off it.
    pub(crate) fn lab_camera(&self) -> Option<LabCamera> {
        let ease = &self.lab_ease;
        let target = ease.target.filter(|_| ease.share > 0.0)?;
        let (eye, facing) = ease.eye?;
        let smooth = |share: f32| share * share * (3.0 - 2.0 * share);
        Some(LabCamera {
            target,
            view: self.lab_view,
            share: smooth(ease.share),
            chase: smooth(ease.chase),
            build: smooth(ease.build),
            eye,
            facing,
        })
    }
}

impl GpuState {
    /// Play this frame's Saber Lab sounds at `now`, the stage's clock.
    pub(crate) fn play_saber_lab_sounds(&mut self, audio: &mut Option<GameAudio>, now: Instant) {
        let lab = self.menu_stage.lab;
        let (Some(lab), Some(actor)) = (lab, &mut self.menu_stage.actor) else {
            self.menu_stage.lab_sounds = Sounds::default();
            return;
        };
        let prefetch = actor.preview.event_sounds.take();
        let frames = [&actor.lower, &actor.upper].map(|track| {
            let clip = sjk_game_jka::legacy_animation_index(&track.sequence.name)?;
            let ((frame, _), _) = track.sample(now);
            Some((clip, frame as i32))
        });
        let sounds = &mut self.menu_stage.lab_sounds;
        let switched = sounds.lit.replace(lab.lit).filter(|was| *was != lab.lit);
        // An attack begun: a match plays its swing then (`EV_SABER_ATTACK`), not from the
        // animation's cues.
        let attacking = is_attack(&actor.upper.sequence.name);
        let attacked = attacking && sounds.attack != Some(actor.upper.started);
        if attacking {
            sounds.attack = Some(actor.upper.started);
        }
        let attack_variant = sounds.random.index(1 << 16);
        let snaps = std::mem::take(&mut sounds.snaps).min(SNAPS_A_FRAME);
        let clashed = std::mem::take(&mut sounds.clashed);
        let block_variant = sounds.random.index(9);
        let snap_variants = [sounds.random.index(1 << 16), sounds.random.index(1 << 16)];
        // With no match playing, the lab loops its hum (and buzz) itself.
        let looping = self.live_session.is_none() && self.demo_session.is_none();
        let audio = audio.as_mut().filter(|_| !lab.muted);
        let (Some(audio), Some(vfs)) = (audio, self.vfs.clone()) else {
            // Muted, the cues still pass, so that none plays late once heard again.
            let actor = self.menu_stage.actor.as_ref().expect("checked above");
            self.menu_stage.lab_sounds.cursor.advance(
                &actor.preview.events,
                &actor.preview.config,
                frames,
                |_, _| {},
            );
            return;
        };
        if let Some(prefetch) = prefetch {
            audio.absorb_animation_prefetch(&prefetch);
        }
        // The first saber's skin is heard (the lab's tab dresses both).
        let skin = self.stage_skins()[0].map(|skin| skin.index);
        let skins = std::sync::Arc::clone(&self.blade_skins);
        let skin_sounds = skin.and_then(|index| skins.sounds(index));
        let hilt = self.menu_stage.request.hilt(0).and_then(|name| {
            self.menu_stage
                .saber_definitions
                .as_ref()?
                .get(&name.to_ascii_lowercase())
        });
        if clashed {
            let block = format!("{BLOCK_PREFIX}{}.mp3", block_variant + 1);
            if audio.register_sound(&vfs, &block) {
                audio.play_local(&block, 1.0, SOURCE, FREE_CHANNEL);
            }
        }
        let crackle = skin.and_then(|index| skins.crackle(index));
        // The skin's crackle: a snap for the bolts let go, the buzz under the hum.
        if let Some(crackle) = crackle.filter(|crackle| !crackle.snaps.is_empty()) {
            for variant in snap_variants.iter().take(snaps as usize) {
                let path = &crackle.snaps[variant % crackle.snaps.len()];
                if audio.register_sound(&vfs, path) {
                    audio.play_local(path, crackle.snap_volume, SOURCE, FREE_CHANNEL);
                }
            }
        }
        if looping && lab.lit {
            let hum = [
                skin_sounds.map(|sounds| sounds.hum.as_str()),
                Some(STOCK_HUM),
            ]
            .into_iter()
            .flatten()
            .find(|path| !path.is_empty() && audio.register_sound(&vfs, path));
            if let Some(hum) = hum {
                audio.menu_loop(hum, HUM_VOLUME, SOURCE);
            }
            if let Some(crackle) = crackle.filter(|crackle| !crackle.buzz.is_empty())
                && audio.register_sound(&vfs, &crackle.buzz)
            {
                audio.menu_loop(&crackle.buzz, crackle.buzz_volume, SOURCE);
            }
        }
        // The first of `paths` there is plays (a skin may name a file no pack holds).
        let mut play = |paths: &[Option<&str>], channel: ChannelId| {
            let found = paths
                .iter()
                .flatten()
                .find(|path| !path.is_empty() && audio.register_sound(&vfs, path));
            if let Some(path) = found {
                audio.play_local(path, 1.0, SOURCE, channel);
            }
        };
        if let Some(was) = switched {
            let (own, hilt, stock) = if was {
                (
                    skin_sounds.map(|sounds| sounds.off.as_str()),
                    hilt.and_then(|hilt| hilt.sound_off.as_deref()),
                    STOCK_OFF,
                )
            } else {
                (
                    skin_sounds.map(|sounds| sounds.on.as_str()),
                    hilt.and_then(|hilt| hilt.sound_on.as_deref()),
                    STOCK_ON,
                )
            };
            play(&[own, hilt, Some(stock)], SWITCH_CHANNEL);
        }
        // The hilt's own swings, else the stock one; the skin's over it.
        let swing = |variant: usize, stock: &str| {
            hilt.filter(|hilt| hilt.sound_swing[0].is_some())
                .and_then(|hilt| hilt.sound_swing[variant % 3].clone())
                .unwrap_or_else(|| stock.to_owned())
        };
        let skin_swing = |variant| skin.and_then(|index| skins.swing(index, variant));
        if attacked && lab.lit {
            let stock = format!("{SWING_PREFIX}{}.mp3", attack_variant % 8 + 1);
            play(&[Some(&swing(attack_variant, &stock))], FREE_CHANNEL);
            play(&[skin_swing(attack_variant)], FREE_CHANNEL);
        }
        let actor = self.menu_stage.actor.as_ref().expect("checked above");
        self.menu_stage.lab_sounds.cursor.advance(
            &actor.preview.events,
            &actor.preview.config,
            frames,
            |cue, variant| {
                let Cue::Sound { paths, .. } = cue else {
                    return;
                };
                let Some(standard) = paths.get(variant % paths.len().max(1)) else {
                    return;
                };
                if !standard.starts_with(SWING_PREFIX) {
                    play(&[Some(standard)], FREE_CHANNEL);
                } else if lab.lit {
                    // A cued swing (the spins and specials), as a match plays it
                    // (`HiltCatalog::animation_sound`).
                    play(&[Some(&swing(variant, standard))], FREE_CHANNEL);
                    play(&[skin_swing(variant)], FREE_CHANNEL);
                }
            },
        );
    }
}

/// Whether `animation` is one of the attacks a match plays a swing for as it begins
/// (`BOTH_A1_T__B_` and the like, the `LS_A_*` moves' and the specials').
fn is_attack(animation: &str) -> bool {
    animation
        .strip_prefix("BOTH_A")
        .and_then(|rest| rest.strip_prefix(|c: char| c.is_ascii_digit()))
        .is_some_and(|rest| rest.starts_with('_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The attacks, a style's and the staff's and two sabers', sound as they begin;
    /// the starts, returns, stances and the back attack do not.
    #[test]
    fn the_attacks_are_the_swings_a_match_sounds() {
        for attack in [
            "BOTH_A1_T__B_",
            "BOTH_A3__L__R",
            "BOTH_A6_TL_BR",
            "BOTH_A7__L__R",
        ] {
            assert!(is_attack(attack), "{attack}");
        }
        for other in [
            "BOTH_ATTACK_BACK",
            "BOTH_S1_S1_T_",
            "BOTH_R1_B__S1",
            "BOTH_STAND2",
        ] {
            assert!(!is_attack(other), "{other}");
        }
    }
}
