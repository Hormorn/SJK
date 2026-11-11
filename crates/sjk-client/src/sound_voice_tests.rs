//! Synthetic adapter metadata checks; these do not exercise the mixer or movement.
use super::*;
use crate::LegacyVoiceLine;
use sjk_protocol::{LEGACY_ENTITY_FIELDS, PlayerState};

#[test]
fn voice_metadata_keeps_impacts_and_regular_sound_paths_out_of_holds() {
    let mut game = GameState::empty_local(0);
    game.replace_config_string(CS_SOUNDS + 1, b"*anger1.wav".to_vec())
        .unwrap();
    game.replace_config_string(CS_SOUNDS + 2, b"sound/regular.wav".to_vec())
        .unwrap();
    let mut vfs = VirtualFileSystem::new();
    vfs.mount_memory(
        "synthetic",
        [
            ("sound/chars/mp_generic_male/misc/anger1.wav", vec![1]),
            ("sound/chars/mp_generic_male/misc/jump1.wav", vec![1]),
            ("sound/chars/mp_generic_male/misc/land1.wav", vec![1]),
            ("sound/player/roll1.wav", vec![1]),
            ("sound/player/land1.wav", vec![1]),
            ("sound/regular.wav", vec![1]),
        ],
    )
    .unwrap();
    let mut adapter = LegacySoundAdapter::new(&game, &vfs, |_, _| Some(SoundHandle(1)));
    let snapshot = Snapshot {
        message_sequence: 1,
        reliable_acknowledge: 0,
        server_commands: vec![],
        server_time: 1000,
        delta_from: None,
        flags: 0,
        area_mask: vec![],
        player: PlayerState::zero(),
        vehicle_player: None,
        entities: vec![],
        consumed_bits: 0,
    };
    let mut entity = EntityState::zero(2, &LEGACY_ENTITY_FIELDS);
    entity.set_raw_field(32, 2);
    let subject = EventSubject::entity(&entity);
    for (event, parameter, expected) in [
        (76, 1, Some(LegacyVoiceLine::Taunt)),
        (79, 1, Some(LegacyVoiceLine::Taunt)),
        (76, 2, None),
        (16, 0, Some(LegacyVoiceLine::Jump)),
    ] {
        adapter.decisions.clear();
        adapter.resolve(event, parameter, subject, &snapshot, 1000);
        assert_eq!(adapter.decisions[0].voice_line, expected);
    }
    adapter.decisions.clear();
    adapter.resolve(17, 0, subject, &snapshot, 1000);
    assert_eq!(adapter.decisions.len(), 2);
    assert_eq!(adapter.decisions[0].voice_line, Some(LegacyVoiceLine::Roll));
    assert_eq!(adapter.decisions[1].voice_line, None);
    adapter.decisions.clear();
    adapter.resolve(11, 51, subject, &snapshot, 1000);
    assert!(
        adapter
            .decisions
            .iter()
            .any(|d| d.voice_line == Some(LegacyVoiceLine::Land))
    );
    assert!(adapter.decisions.iter().any(|d| d.voice_line.is_none()));
    // Live configstring edits replace the classification, not just the sound handle.
    game.replace_config_string(CS_SOUNDS + 1, b"sound/regular.wav".to_vec())
        .unwrap();
    adapter.refresh_sound_table(CS_SOUNDS + 1, &game, &vfs, |_, _| Some(SoundHandle(1)));
    adapter.decisions.clear();
    adapter.resolve(76, 1, subject, &snapshot, 1000);
    assert_eq!(adapter.decisions[0].voice_line, None);
}

#[test]
fn a_missing_model_speaks_with_the_players_own_gender() {
    // Hub #312: a female player whose model this client lacks (drawn as a female
    // stand-in with cg_defaultModelRandom) spoke with the generic male voice.
    let mut game = GameState::empty_local(0);
    game.replace_config_string(
        crate::player_identity::CS_PLAYERS,
        br"n\Her\t\0\model\not_here/default\ds\f".to_vec(),
    )
    .unwrap();
    game.replace_config_string(
        crate::player_identity::CS_PLAYERS + 1,
        br"n\Him\t\0\model\not_here/default".to_vec(),
    )
    .unwrap();
    let mut vfs = VirtualFileSystem::new();
    vfs.mount_memory(
        "synthetic",
        [
            ("sound/chars/mp_generic_female/misc/jump1.wav", vec![1]),
            ("sound/chars/mp_generic_male/misc/jump1.wav", vec![1]),
        ],
    )
    .unwrap();
    let mut paths = Vec::new();
    let mut intern = |path: &str| {
        paths.push(path.to_owned());
        u16::try_from(paths.len() - 1).unwrap()
    };
    let her = custom_set(&game, &vfs, 0, &mut intern).jump;
    let him = custom_set(&game, &vfs, 1, &mut intern).jump;
    let path = |index: Option<u16>| paths[usize::from(index.unwrap())].as_str();
    assert_eq!(path(her), "sound/chars/mp_generic_female/misc/jump1.wav");
    assert_eq!(path(him), "sound/chars/mp_generic_male/misc/jump1.wav");
}
