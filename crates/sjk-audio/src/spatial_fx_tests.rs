//! The 3D sound in the mixer (`spatial_fx.rs`): off is the plain mix, a sound is
//! heard from its side, walls take its highs and glide, the room rings only from
//! sounds placed in the world, and nothing allocates.

use super::*;

const RATE: u32 = 44_100;
const BLOCK: usize = 256;
const NEAR: Attenuation = Attenuation::Linear {
    full_volume_distance: 256.0,
    falloff_per_unit: 0.0008,
};

fn mixer(voices: usize, loops: usize) -> Mixer {
    let mut mixer = Mixer::new(MixerConfig {
        voices,
        loops,
        sample_rate: RATE,
        loop_attenuation: NEAR,
        doppler: DopplerConfig::disabled(),
        protected_channels: &[],
    });
    // Quake axes: +X forward, +Y left, +Z up.
    mixer.listener([0.0; 3], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]);
    mixer
}

fn placed(source: u32, origin: [f32; 3]) -> PlayRequest {
    PlayRequest {
        origin: Some(origin),
        source: SourceId(source),
        channel: ChannelId(1),
        volume: 1.0,
        attenuation: NEAR,
    }
}

/// A tone at a quarter of the mixer's rate: in the high band.
fn high_tone(length: usize) -> Vec<f32> {
    (0..length).map(|i| [0.0, 0.5, 0.0, -0.5][i % 4]).collect()
}

/// Deterministic noise in [-1, 1].
fn noise(length: usize) -> Vec<f32> {
    let mut state = 0x1234_5678_u32;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

/// Voices, loops and music, rendered for `blocks` blocks.
fn scene(mixer: &mut Mixer, blocks: usize) -> Vec<f32> {
    let tone = mixer.sound_bank_mut().register_pcm(&noise(30_000), 22_050);
    let music = mixer.sound_bank_mut().register_pcm(&high_tone(9_000), RATE);
    mixer.start_background_track(music, music, 0.5);
    mixer.play(tone, placed(1, [400.0, 100.0, 0.0]));
    mixer.play(
        tone,
        PlayRequest {
            origin: None,
            attenuation: Attenuation::None,
            ..placed(2, [0.0; 3])
        },
    );
    let mut rendered = Vec::new();
    let mut out = vec![0.0; BLOCK];
    for block in 0..blocks {
        mixer.clear_loops();
        mixer.set_loop(SourceId(3), tone, [200.0, -300.0, 0.0], [0.0; 3]);
        mixer.set_loop(SourceId(4), tone, [-500.0, 0.0, 40.0], [0.0; 3]);
        mixer.commit_loops();
        if block == 7 {
            mixer.play(tone, placed(5, [900.0, 0.0, 0.0]));
        }
        mixer.render(&mut out);
        rendered.extend_from_slice(&out);
    }
    rendered
}

fn bits(samples: &[f32]) -> Vec<u32> {
    samples.iter().map(|sample| sample.to_bits()).collect()
}

fn walled() -> SourceAcoustics {
    SourceAcoustics {
        bands: [0.5, 0.1, 0.01],
        via: None,
    }
}

fn hall() -> Box<RoomResponse> {
    let ring: Vec<f32> = noise(RATE as usize)
        .iter()
        .enumerate()
        .map(|(index, sample)| sample * 0.05 * (-(index as f32) / 8_000.0).exp())
        .collect();
    Box::new(RoomResponse::from_impulses(&ring, &ring))
}

#[test]
fn switched_off_the_mix_is_bit_identical_whatever_was_measured() {
    let mut plain = mixer(16, 8);
    let expected = scene(&mut plain, 40);
    let mut measured = mixer(16, 8);
    measured.set_spatial_effects(true);
    measured.set_spatial_effects(false);
    for source in 1..=5 {
        measured.set_source_acoustics(SourceId(source), walled());
    }
    measured.set_room_response(Some(hall()));
    assert_eq!(bits(&scene(&mut measured, 40)), bits(&expected));
}

#[test]
fn on_for_speakers_in_the_open_with_a_dry_room_sounds_as_off() {
    let mut plain = mixer(16, 8);
    let expected = scene(&mut plain, 40);
    let mut open = mixer(16, 8);
    open.set_spatial_effects(true);
    open.set_spatial_headphones(false);
    let heard = scene(&mut open, 40);
    let worst = heard
        .iter()
        .zip(&expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(worst < 1.0e-5, "worst difference {worst}");
}

/// Each ear's energy of a noise burst from `origin`, through the head.
fn ears(origin: [f32; 3], acoustics: Option<SourceAcoustics>) -> [f32; 2] {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    if let Some(acoustics) = acoustics {
        mixer.set_source_acoustics(SourceId(1), acoustics);
    }
    let sound = mixer.sound_bank_mut().register_pcm(&noise(8_000), RATE);
    mixer.play(sound, placed(1, origin));
    let mut out = vec![0.0; BLOCK];
    let mut energy = [0.0_f32; 2];
    for _ in 0..40 {
        mixer.render(&mut out);
        for frame in out.chunks_exact(2) {
            energy[0] += frame[0] * frame[0];
            energy[1] += frame[1] * frame[1];
        }
    }
    energy
}

#[test]
fn a_sound_on_the_right_is_heard_in_the_right_ear() {
    // Quake's -Y is the listener's right.
    let [left, right] = ears([0.0, -200.0, 0.0], None);
    assert!(right > left * 4.0, "left {left}, right {right}");
    let [left, right] = ears([0.0, 200.0, 0.0], None);
    assert!(left > right * 4.0, "left {left}, right {right}");
    let [left, right] = ears([200.0, 0.0, 0.0], None);
    assert!(
        (left - right).abs() < 0.1 * left,
        "left {left}, right {right}"
    );
}

#[test]
fn a_wall_makes_a_sound_quieter() {
    let open = ears([200.0, 0.0, 0.0], None);
    let behind = ears([200.0, 0.0, 0.0], Some(walled()));
    assert!(
        behind[0] < open[0] * 0.5,
        "open {open:?}, behind {behind:?}"
    );
}

/// The loudest sample of each block of a placed high tone, walled from block 4.
fn walled_envelope() -> Vec<f32> {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    mixer.set_spatial_headphones(false);
    let tone = mixer
        .sound_bank_mut()
        .register_pcm(&high_tone(RATE as usize), RATE);
    mixer.play(tone, placed(1, [100.0, 0.0, 0.0]));
    let mut out = vec![0.0; BLOCK];
    (0..80)
        .map(|block| {
            if block == 4 {
                mixer.set_source_acoustics(SourceId(1), walled());
            }
            mixer.render(&mut out);
            out.iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
        })
        .collect()
}

#[test]
fn a_wall_glides_in_and_takes_the_highs() {
    let envelope = walled_envelope();
    let open = envelope[3];
    assert!(open > 0.2, "open level {open}");
    // No step: the first walled block still holds most of the sound...
    assert!(envelope[4] > open * 0.7, "{envelope:?}");
    for pair in envelope[4..40].windows(2) {
        assert!(pair[0] - pair[1] < open * 0.15, "{envelope:?}");
    }
    // ...and settled, a high tone is nearly gone.
    assert!(envelope[79] < open * 0.05, "settled {}", envelope[79]);
}

#[test]
fn a_voice_started_behind_a_wall_starts_muffled() {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    mixer.set_spatial_headphones(false);
    let tone = mixer
        .sound_bank_mut()
        .register_pcm(&high_tone(RATE as usize), RATE);
    mixer.set_source_acoustics(SourceId(1), walled());
    mixer.play(tone, placed(1, [100.0, 0.0, 0.0]));
    let mut out = vec![0.0; BLOCK];
    mixer.render(&mut out);
    let peak = out[64..]
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak < 0.05, "peak {peak}");
}

#[test]
fn listener_relative_sounds_are_left_as_they_are() {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    let tone = mixer
        .sound_bank_mut()
        .register_pcm(&high_tone(RATE as usize), RATE);
    mixer.set_source_acoustics(SourceId(1), walled());
    mixer.play(
        tone,
        PlayRequest {
            origin: None,
            attenuation: Attenuation::None,
            ..placed(1, [0.0; 3])
        },
    );
    let mut out = vec![0.0; BLOCK];
    mixer.render(&mut out);
    assert!((out[2] - 0.5).abs() < 1.0e-6, "{}", out[2]);
}

#[test]
fn a_loop_is_heard_through_the_head_too() {
    let mut mixer = mixer(1, 4);
    mixer.set_spatial_effects(true);
    let tone = mixer.sound_bank_mut().register_pcm(&noise(4_000), RATE);
    let mut out = vec![0.0; BLOCK];
    let mut energy = [0.0_f32; 2];
    for _ in 0..40 {
        mixer.clear_loops();
        mixer.set_loop(SourceId(7), tone, [0.0, 150.0, 0.0], [0.0; 3]);
        mixer.commit_loops();
        mixer.render(&mut out);
        for frame in out.chunks_exact(2) {
            energy[0] += frame[0] * frame[0];
            energy[1] += frame[1] * frame[1];
        }
    }
    assert!(energy[0] > energy[1] * 4.0, "{energy:?}");
}

/// The tail a short click leaves once it has ended, from `request`'s voice.
fn tail_after_click(request: PlayRequest) -> f32 {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    mixer.set_room_response(Some(hall()));
    let click = mixer.sound_bank_mut().register_pcm(&[0.8; 32], RATE);
    let mut out = vec![0.0; BLOCK];
    mixer.play(click, request);
    mixer.render(&mut out);
    (0..40)
        .map(|_| {
            mixer.render(&mut out);
            out.iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
        })
        .fold(0.0, f32::max)
}

#[test]
fn only_sounds_placed_in_the_world_ring_with_the_room() {
    let placed_tail = tail_after_click(placed(1, [100.0, 0.0, 0.0]));
    assert!(placed_tail > 1.0e-4, "placed tail {placed_tail}");
    // A menu cue, the announcer, the player's own listener-relative sounds: dry.
    let local = PlayRequest {
        origin: None,
        attenuation: Attenuation::None,
        ..placed(2, [0.0; 3])
    };
    assert_eq!(tail_after_click(local), 0.0);
    // Placed but global (no falloff): dry too.
    let global = PlayRequest {
        attenuation: Attenuation::None,
        ..placed(3, [100.0, 0.0, 0.0])
    };
    assert_eq!(tail_after_click(global), 0.0);
}

#[test]
fn a_replaced_room_comes_back_to_be_freed() {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    mixer.set_room_response(Some(hall()));
    mixer.set_room_response(Some(hall()));
    let mut out = vec![0.0; BLOCK];
    for _ in 0..40 {
        mixer.render(&mut out);
    }
    assert!(mixer.take_retired_response().is_some());
}

/// The counting allocator exists in debug test builds only.
#[cfg(debug_assertions)]
#[test]
fn the_3d_sound_allocates_nothing_while_mixing() {
    let mut mixer = mixer(16, 8);
    mixer.set_spatial_effects(true);
    let tone = mixer.sound_bank_mut().register_pcm(&noise(20_000), RATE);
    let mut rooms = [Some(hall()), Some(hall())];
    let mut retired = Vec::with_capacity(8);
    let mut out = vec![0.0; BLOCK];
    let before = assert_no_alloc::violation_count();
    assert_no_alloc::assert_no_alloc(|| {
        for block in 0..40_u32 {
            mixer.set_source_acoustics(
                SourceId(block % 70),
                SourceAcoustics {
                    bands: [1.0, (block % 3) as f32 / 2.0, 0.1],
                    via: (block % 2 == 0).then_some([50.0, 50.0, 0.0]),
                },
            );
            if block % 15 == 0
                && let Some(room) = rooms[(block / 15) as usize % 2].take()
            {
                mixer.set_room_response(Some(room));
            }
            mixer.play(tone, placed(block % 5, [300.0, 0.0, 0.0]));
            mixer.clear_loops();
            mixer.set_loop(SourceId(9), tone, [0.0, 300.0, 0.0], [0.0; 3]);
            mixer.commit_loops();
            mixer.render(&mut out);
            // Freeing is the caller's, outside the mix.
            if let Some(room) = mixer.take_retired_response() {
                retired.push(room);
            }
        }
    });
    assert_eq!(assert_no_alloc::violation_count(), before);
    assert!(out.iter().any(|sample| *sample != 0.0));
    drop(retired);
}

/// The mixer's cost with every voice and loop busy, the 3D sound off and on:
/// `cargo test --release -p sjk-audio spatial_mix_cost -- --ignored --nocapture`.
#[test]
#[ignore = "timing; run in release"]
fn spatial_mix_cost() {
    let run = |enabled: bool, voices: u32| {
        let mut mixer = mixer(128, 32);
        mixer.set_spatial_effects(enabled);
        // A hall: the longest response.
        let ring: Vec<f32> = noise(RATE as usize * 3)
            .iter()
            .map(|sample| sample * 0.01)
            .collect();
        mixer.set_room_response(Some(Box::new(RoomResponse::from_impulses(&ring, &ring))));
        let tone = mixer
            .sound_bank_mut()
            .register_pcm(&noise(RATE as usize * 12), RATE);
        let tones: Vec<_> = (0..32)
            .map(|_| {
                mixer
                    .sound_bank_mut()
                    .register_pcm(&noise(RATE as usize), RATE)
            })
            .collect();
        for voice in 0..voices {
            mixer.set_source_acoustics(SourceId(voice), walled());
            mixer.play(tone, placed(voice, [300.0 + voice as f32, 50.0, 0.0]));
        }
        let mut out = vec![0.0; BLOCK];
        let blocks = 4_000;
        let started = std::time::Instant::now();
        for block in 0..blocks {
            // The listener turns: every voice's filters change every block.
            let angle = block as f32 * 0.01;
            mixer.listener(
                [0.0; 3],
                [angle.cos(), angle.sin(), 0.0],
                [angle.sin(), -angle.cos(), 0.0],
                [0.0, 0.0, 1.0],
            );
            mixer.clear_loops();
            for (index, handle) in tones.iter().enumerate() {
                mixer.set_loop(
                    SourceId(1_000 + index as u32),
                    *handle,
                    [200.0, index as f32 * 10.0, 0.0],
                    [0.0; 3],
                );
            }
            mixer.commit_loops();
            mixer.render(&mut out);
            std::hint::black_box(&out);
        }
        started.elapsed().as_secs_f64() * 1.0e6 / f64::from(blocks)
    };
    let budget = (BLOCK / 2) as f64 * 1.0e6 / f64::from(RATE);
    for voices in [32, 128] {
        let (off, on) = (run(false, voices), run(true, voices));
        println!(
            "{voices} voices + 32 loops, {BLOCK}-sample block, turning, 3 s room: off {off:.1} us, \
             on {on:.1} us (budget {budget:.0} us)"
        );
    }
}

#[test]
fn a_long_render_with_the_3d_sound_is_mixed_in_parts() {
    let mut mixer = mixer(4, 0);
    mixer.set_spatial_effects(true);
    let tone = mixer.sound_bank_mut().register_pcm(&noise(20_000), RATE);
    mixer.play(tone, placed(1, [100.0, 0.0, 0.0]));
    let mut out = vec![0.0; spatial_fx::SEND_FRAMES * 2 * 2 + 6];
    mixer.render(&mut out);
    assert!(out[spatial_fx::SEND_FRAMES * 2 + 10] != 0.0);
}

#[test]
fn through_the_head_a_sound_is_about_as_loud_as_panned() {
    for origin in [
        [200.0, 0.0, 0.0],
        [0.0, -200.0, 0.0],
        [0.0, 200.0, 0.0],
        [-200.0, 0.0, 0.0],
        [0.0, 0.0, 200.0],
        [140.0, 140.0, -60.0],
    ] {
        let head = ears(origin, None);
        let mut plain = mixer(4, 0);
        let sound = plain.sound_bank_mut().register_pcm(&noise(8_000), RATE);
        plain.play(sound, placed(1, origin));
        let mut out = vec![0.0; BLOCK];
        let mut panned = 0.0_f32;
        for _ in 0..40 {
            plain.render(&mut out);
            panned += out.iter().map(|s| s * s).sum::<f32>();
        }
        let ratio = (head[0] + head[1]) / panned;
        println!("{origin:?}: {ratio}");
        assert!((0.4..2.0).contains(&ratio), "{origin:?}: {ratio}");
    }
}
