//! The local player's choice of a blade skin's options (`docs/unlockables.md`, "Options"):
//! the parts of a skin its wearer switched off, every one on until then. They are kept in
//! [`CVAR`], one `<skin>.<option>` word for each part switched off, so a skin keeps its
//! choice while another is worn; the look sent to the hub carries the worn skin's
//! (`saber_off`), and every other SJK player draws the skin that way.
//!
//! A skin's extras (`blade_extras.rs`, since 26/10/2026) are the other way round: off until
//! switched on, so a word names an extra switched on. The look carries those as `saber_on`.
//! A word thus always names a choice that differs from its default.
//!
//! In a frame a choice is a set of 64 bits ([`OptionsOff`]), one per option id
//! ([`crate::blade_skin_file::extras::option_bit`]): a part's bit set is that part off, an
//! extra's set is that extra on. A look stays `Copy` and a skin's colour is found without
//! allocating.

use crate::blade_skin_file::extras::option_bit;

/// Archived: the blade skins' options switched off, `<skin id>.<option id>` words.
pub(crate) const CVAR: &str = "cg_saberSkinOptions";

/// A skin's options switched from their default, a bit per option id: its parts switched
/// off and its extras switched on; empty with every part on and every extra off.
pub(crate) type OptionsOff = u64;

/// The words of `setting` that name an option of `skin` switched off, the option ids.
pub(crate) fn off_ids<'a>(setting: &'a str, skin: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    setting.split_whitespace().filter_map(move |word| {
        let (named, option) = word.split_once('.')?;
        (named == skin && !option.is_empty()).then_some(option)
    })
}

/// The parts of `skin` switched off in `setting`, as the look sends them (`saber_off`):
/// sorted, each once, at most eight (the hub's limit).
pub(crate) fn off_list(setting: &str, skin: &str) -> Vec<String> {
    list(setting, skin, false)
}

/// The extras of `skin` switched on in `setting`, as the look sends them (`saber_on`), as
/// [`off_list`].
pub(crate) fn on_list(setting: &str, skin: &str) -> Vec<String> {
    list(setting, skin, true)
}

/// The words of `setting` for `skin`'s extras (`extras`) or its parts, sorted, each once,
/// at most eight.
fn list(setting: &str, skin: &str, extras: bool) -> Vec<String> {
    let mut ids: Vec<String> = off_ids(setting, skin)
        .filter(|id| crate::blade_extras::is_extra(skin, id) == extras)
        .map(str::to_owned)
        .collect();
    ids.sort();
    ids.dedup();
    ids.truncate(8);
    ids
}

/// The bits of the option ids `ids`.
pub(crate) fn bits<'a>(ids: impl IntoIterator<Item = &'a str>) -> OptionsOff {
    ids.into_iter().fold(0, |bits, id| bits | option_bit(id))
}

/// The options of `skin` switched from their default in `setting` (its parts off, its
/// extras on), as bits.
pub(crate) fn off_bits(setting: &str, skin: &str) -> OptionsOff {
    bits(off_ids(setting, skin))
}

/// The bits of what a look says of `skin`: its parts in `off` (`saber_off`) and its extras
/// in `on` (`saber_on`), each list read only for its own kind, so neither turns on
/// what the other is for (an extra named in `saber_off`, which the hub does not check).
pub(crate) fn look_bits<'a>(
    skin: &str,
    off: impl IntoIterator<Item = &'a str>,
    on: impl IntoIterator<Item = &'a str>,
) -> OptionsOff {
    bits(
        off.into_iter()
            .filter(|id| !crate::blade_extras::is_extra(skin, id))
            .chain(
                on.into_iter()
                    .filter(|id| crate::blade_extras::is_extra(skin, id)),
            ),
    )
}

/// Whether `skin`'s option `option` is on in `setting`: a part unless its word is there,
/// an extra only when it is.
pub(crate) fn is_on(setting: &str, skin: &str, option: &str) -> bool {
    off_ids(setting, skin).any(|id| id == option) == starts_off(skin, option)
}

/// Whether an option is off until switched on: an extra.
fn starts_off(skin: &str, option: &str) -> bool {
    crate::blade_extras::is_extra(skin, option)
}

/// `setting` with `skin`'s option `option` switched on or off: its word gone, or there
/// once (a part's while off, an extra's while on); the words sorted, so the same choice is
/// always the same text.
pub(crate) fn with(setting: &str, skin: &str, option: &str, on: bool) -> String {
    let word = format!("{skin}.{option}");
    let mut words: Vec<&str> = setting
        .split_whitespace()
        .filter(|other| *other != word)
        .collect();
    if on == starts_off(skin, option) {
        words.push(&word);
    }
    words.sort_unstable();
    words.dedup();
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_names_each_skins_parts_switched_off() {
        let setting = "saber_sun.haze saber_spectral.wisps saber_sun.glint junk .x y.";
        assert_eq!(off_list(setting, "saber_sun"), ["glint", "haze"]);
        assert_eq!(off_list(setting, "saber_spectral"), ["wisps"]);
        assert!(off_list(setting, "saber_void").is_empty());
        assert!(is_on(setting, "saber_sun", "prominences"));
        assert!(!is_on(setting, "saber_sun", "haze"));
        assert_eq!(
            off_bits(setting, "saber_sun"),
            option_bit("glint") | option_bit("haze")
        );
        assert_eq!(off_bits("", "saber_sun"), 0);
    }

    #[test]
    fn switching_an_option_keeps_one_sorted_word_for_it() {
        let off = with("", "saber_sun", "haze", false);
        assert_eq!(off, "saber_sun.haze");
        let both = with(&off, "saber_spectral", "wisps", false);
        assert_eq!(both, "saber_spectral.wisps saber_sun.haze");
        assert_eq!(with(&both, "saber_sun", "haze", false), both, "already off");
        assert_eq!(
            with(&both, "saber_sun", "haze", true),
            "saber_spectral.wisps"
        );
        assert_eq!(with("saber_sun.glint", "saber_sun", "glint", true), "");
    }

    #[test]
    fn an_extras_word_names_it_on_and_goes_in_its_own_list() {
        let on = with("saber_sun.haze", "saber_sun", "flare_trail", true);
        assert_eq!(on, "saber_sun.flare_trail saber_sun.haze");
        assert!(is_on(&on, "saber_sun", "flare_trail"));
        assert!(
            !is_on("", "saber_sun", "flare_trail"),
            "an extra is off by default"
        );
        assert!(!is_on(&on, "saber_sun", "haze"));
        assert_eq!(
            with(&on, "saber_sun", "flare_trail", false),
            "saber_sun.haze"
        );
        assert_eq!(off_list(&on, "saber_sun"), ["haze"]);
        assert_eq!(on_list(&on, "saber_sun"), ["flare_trail"]);
        assert_eq!(
            off_bits(&on, "saber_sun"),
            option_bit("haze") | option_bit("flare_trail")
        );
        // A look's lists, each read for its own kind only.
        assert_eq!(
            look_bits("saber_sun", ["haze", "heat_wake"], ["flare_trail", "glint"]),
            option_bit("haze") | option_bit("flare_trail")
        );
    }

    #[test]
    fn the_look_sends_at_most_eight_once_each() {
        let setting: String = (0..12)
            .map(|n| format!("saber_sun.o{n:02} saber_sun.o{n:02} "))
            .collect();
        let list = off_list(&setting, "saber_sun");
        assert_eq!(list.len(), 8);
        assert_eq!(list[0], "o00");
    }
}
