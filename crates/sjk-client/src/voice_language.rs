//! `s_language`, the language of the voices, as retail reads it (`snd_mem.cpp`
//! `S_LoadSound_Actual`, separate from the text's `se_language`): a sound under
//! `chars` is looked up first in the language's folder (`chr_d`, `chr_f`, `chr_e`),
//! and a voice the language lacks stays English.
//!
//! One setting for every place that reads a sound: the viewer's sound cache and the
//! game's sound tables ([`crate::LegacySoundAdapter`]), which hold the players' pain,
//! death and taunt voices. Before hub report #298 only the first followed it, so
//! after `snd_restart` rebuilt the tables the players' voices were English again.

use std::sync::atomic::{AtomicU8, Ordering};

/// The cvar.
pub const CVAR: &str = "s_language";
/// The folders standing in for `chars`, by [`LANGUAGE`] less one.
const FOLDERS: [&str; 3] = ["chr_d", "chr_f", "chr_e"];
/// The voices' language: 0 the English `chars`, else one more than its folder's index.
static LANGUAGE: AtomicU8 = AtomicU8::new(0);

/// The number `language` stands for: retail's three names, and the English ones
/// players type; anything else is English, as in retail.
fn number(language: &str) -> u8 {
    [
        ("deutsch", 1),
        ("german", 1),
        ("francais", 2),
        ("french", 2),
        ("espanol", 3),
        ("spanish", 3),
    ]
    .into_iter()
    .find(|(name, _)| language.eq_ignore_ascii_case(name))
    .map_or(0, |(_, number)| number)
}

/// Follow `s_language`. Sounds already loaded stay until `snd_restart`, as in retail.
pub fn set(language: &str) {
    LANGUAGE.store(number(language), Ordering::Relaxed);
}

/// `path` in the voices' language (its `chars` swapped for the language's folder),
/// or `None` in English or for a sound that is no voice.
pub fn foreign(path: &str) -> Option<String> {
    foreign_in(path, LANGUAGE.load(Ordering::Relaxed))
}

fn foreign_in(path: &str, language: u8) -> Option<String> {
    let folder = FOLDERS.get(usize::from(language).checked_sub(1)?)?;
    let at = path.to_ascii_lowercase().find("chars")?;
    Some(format!("{}{folder}{}", &path[..at], &path[at + 5..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_voice_moves_to_its_languages_folder() {
        let path = "sound/chars/kyle/misc/pain25.mp3";
        assert_eq!(
            foreign_in(path, number("FRANCAIS")).as_deref(),
            Some("sound/chr_f/kyle/misc/pain25.mp3")
        );
        assert_eq!(
            foreign_in(path, number("german")).as_deref(),
            Some("sound/chr_d/kyle/misc/pain25.mp3")
        );
        assert_eq!(
            foreign_in(path, number("Espanol")).as_deref(),
            Some("sound/chr_e/kyle/misc/pain25.mp3")
        );
    }

    #[test]
    fn english_and_other_sounds_stay_where_they_are() {
        assert_eq!(number("english"), 0);
        assert_eq!(number("klingon"), 0);
        assert_eq!(foreign_in("sound/chars/kyle/a.mp3", 0), None);
        assert_eq!(foreign_in("sound/weapons/saber/on.mp3", 2), None);
    }

    /// Hub report #298: the game's sound tables take a voice from the language's
    /// folder, and an English one where the language has none. Only voices are
    /// moved, so tests reading other sounds meanwhile are not affected.
    #[test]
    fn the_sound_tables_read_the_voice_in_the_language() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "voices",
            [
                ("sound/chars/alora/misc/pain25.mp3", b"en".to_vec()),
                ("sound/chr_f/alora/misc/pain25.mp3", b"fr".to_vec()),
                ("sound/chars/alora/misc/death1.mp3", b"en".to_vec()),
            ],
        )
        .unwrap();
        let mut sounds = Vec::new();
        let mut heard = Vec::new();
        let mut register = |path: &str, bytes: &[u8]| {
            heard.push((path.to_owned(), bytes.to_vec()));
            Some(sjk_audio::SoundHandle(heard.len() as u32))
        };
        set("francais");
        crate::sound_events::intern_sound(
            &mut sounds,
            &vfs,
            "sound/chars/alora/misc/pain25.wav",
            &mut register,
        );
        crate::sound_events::intern_sound(
            &mut sounds,
            &vfs,
            "sound/chars/alora/misc/death1.wav",
            &mut register,
        );
        set("english");
        assert_eq!(heard[0].0, "sound/chr_f/alora/misc/pain25.mp3");
        assert_eq!(heard[0].1, b"fr");
        assert_eq!(heard[1].0, "sound/chars/alora/misc/death1.mp3");
    }
}
