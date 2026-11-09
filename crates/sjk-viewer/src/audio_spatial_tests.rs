//! 3D sound's side in the client (`audio_spatial.rs`): the hearing point and the
//! world sounds handed to the acoustics.

use super::*;

const NEAR: Attenuation = Attenuation::Linear {
    full_volume_distance: 256.0,
    falloff_per_unit: 0.0008,
};

fn request(source: u32, origin: [f32; 3]) -> PlayRequest {
    PlayRequest {
        origin: Some(origin),
        source: SourceId(source),
        channel: sjk_audio::ChannelId(1),
        volume: 1.0,
        attenuation: NEAR,
    }
}

fn heard(tracker: &mut Tracker, now: Instant) -> Vec<(SourceId, Vec3)> {
    tracker
        .sources
        .retain(|tracked| now.saturating_duration_since(tracked.heard) <= FORGET_AFTER);
    tracker
        .sources
        .iter()
        .map(|tracked| (tracked.source, Vec3::from_array(tracked.origin)))
        .collect()
}

#[test]
fn the_hearing_point_is_the_head_unless_the_camera_is_far() {
    let origin = Vec3::new(100.0, 0.0, 0.0);
    // First person: the eyes.
    let eyes = Vec3::new(100.0, 0.0, 36.0);
    assert_eq!(hearing_point(eyes, Some(origin)), eyes);
    // Third person, the camera behind and above: the head, as high as allowed.
    let behind = Vec3::new(20.0, 0.0, 70.0);
    assert_eq!(
        hearing_point(behind, Some(origin)),
        Vec3::new(100.0, 0.0, 36.0)
    );
    // A camera far off (spectating, the drone) and no player: the camera.
    let far = Vec3::new(600.0, 0.0, 36.0);
    assert_eq!(hearing_point(far, Some(origin)), far);
    assert_eq!(hearing_point(behind, None), behind);
}

#[test]
fn the_players_own_and_listener_relative_sounds_are_left_out() {
    let mut tracker = Tracker {
        enabled: true,
        ..Tracker::default()
    };
    let now = Instant::now();
    tracker.observe(
        &crate::audio_output::AudioCommand::ListenerSource(Some(SourceId(4))),
        now,
    );
    tracker.hear(&request(4, [300.0, 0.0, 0.0]), now);
    tracker.hear(
        &PlayRequest {
            attenuation: Attenuation::None,
            ..request(5, [300.0, 0.0, 0.0])
        },
        now,
    );
    tracker.hear(&request(6, [10.0, 20.0, 30.0]), now);
    assert_eq!(
        heard(&mut tracker, now),
        vec![(SourceId(6), Vec3::new(10.0, 20.0, 30.0))]
    );
}

#[test]
fn a_moving_source_is_followed_and_old_ones_are_forgotten() {
    let mut tracker = Tracker {
        enabled: true,
        ..Tracker::default()
    };
    let now = Instant::now();
    tracker.hear(&request(1, [300.0, 0.0, 0.0]), now);
    tracker.observe(
        &crate::audio_output::AudioCommand::SourcePosition(SourceId(1), [-200.0, 0.0, 0.0]),
        now,
    );
    assert_eq!(
        heard(&mut tracker, now),
        vec![(SourceId(1), Vec3::new(-200.0, 0.0, 0.0))]
    );
    assert!(heard(&mut tracker, now + FORGET_AFTER * 2).is_empty());
}

#[test]
fn a_full_table_gives_up_the_sound_heard_longest_ago() {
    let mut tracker = Tracker {
        enabled: true,
        ..Tracker::default()
    };
    let start = Instant::now();
    for source in 0..TRACKED as u32 + 1 {
        let now = start + Duration::from_millis(u64::from(source));
        tracker.hear(&request(source, [0.0; 3]), now);
    }
    let sources = heard(&mut tracker, start + Duration::from_secs(1));
    assert_eq!(sources.len(), TRACKED);
    assert!(!sources.iter().any(|(source, _)| *source == SourceId(0)));
}

#[test]
fn switched_off_it_notes_nothing() {
    let mut tracker = Tracker::default();
    let now = Instant::now();
    tracker.observe(
        &crate::audio_output::AudioCommand::Play(
            sjk_audio::SoundHandle(0),
            request(1, [300.0, 0.0, 0.0]),
        ),
        now,
    );
    assert!(heard(&mut tracker, now).is_empty());
    assert_eq!(tracker.switch_changed(), None);
    tracker.enabled = true;
    assert_eq!(tracker.switch_changed(), Some(true));
    assert_eq!(tracker.switch_changed(), None);
}

/// Noting sounds takes no allocation once the table is made.
#[cfg(debug_assertions)]
#[test]
fn noting_sounds_allocates_nothing() {
    let mut tracker = Tracker {
        enabled: true,
        ..Tracker::default()
    };
    let now = Instant::now();
    tracker.hear(&request(0, [0.0; 3]), now);
    let before = assert_no_alloc::violation_count();
    assert_no_alloc::assert_no_alloc(|| {
        for round in 0..200_u32 {
            tracker.hear(&request(round % 90, [300.0, round as f32, 0.0]), now);
        }
    });
    assert_eq!(assert_no_alloc::violation_count(), before);
}
