//! The browser editor's page, bundled at compile time: `page/index.html` is the
//! template, and each of its `<!--#include <file> -->` lines is replaced by that file
//! of `page/` (VisionJK's way of putting its page together, without its build step).
//!
//! `page/vjk/` holds files of VisionJK's web admin page (`codemp/game/webadmin/page/js`,
//! GPL-2.0, by Blackwolf, after the JKAdmin panel by Lixsl; taken at VisionJK
//! 50d7c18b), each headed with what SJK changed in it. `page/sjk/` is SJK's own: the
//! stubs for the panel parts left out, the edits, the panel, the camera and the frame.

use std::sync::OnceLock;

macro_rules! parts {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_str!(concat!("page/", $name)))),*]
    };
}

/// The template's files, by the name its include lines give.
const PARTS: &[(&str, &str)] = parts![
    "sjk/style.css",
    "sjk/head.js",
    "vjk/util.js",
    "sjk/stubs.js",
    "vjk/bsp.js",
    "vjk/gl.js",
    "vjk/map.js",
    "vjk/scripts-programs.js",
    "vjk/scripts-parse.js",
    "vjk/scripts-world.js",
    "vjk/scripts-models.js",
    "vjk/models.js",
    "vjk/world-ents.js",
    "vjk/flares.js",
    "vjk/pick.js",
    "vjk/camera.js",
    "vjk/editor-remaps.js",
    "sjk/edits.js",
    "sjk/panel.js",
    "sjk/brushes.js",
    "sjk/view.js",
    "sjk/gizmo.js",
    "sjk/tail.js",
];

const TEMPLATE: &str = include_str!("page/index.html");

/// The page, put together the first time it is asked for.
pub(crate) fn page() -> &'static str {
    static PAGE: OnceLock<String> = OnceLock::new();
    PAGE.get_or_init(|| build(TEMPLATE, PARTS).unwrap_or_else(|why| why))
}

/// `template` with every include line replaced by its part; an include of a part that
/// is not there is an error.
fn build(template: &str, parts: &[(&str, &str)]) -> Result<String, String> {
    let mut page =
        String::with_capacity(template.len() + parts.iter().map(|p| p.1.len()).sum::<usize>());
    for line in template.lines() {
        let include = line
            .trim()
            .strip_prefix("<!--#include ")
            .and_then(|rest| rest.strip_suffix("-->"))
            .map(str::trim);
        match include {
            Some(name) => {
                let (_, text) = parts
                    .iter()
                    .find(|(part, _)| *part == name)
                    .ok_or_else(|| format!("the page includes {name}, which is not bundled"))?;
                page.push_str(text);
            }
            None => page.push_str(line),
        }
        page.push('\n');
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_is_included_once_and_nothing_else() {
        let included: Vec<&str> = TEMPLATE
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("<!--#include ")
                    .and_then(|rest| rest.strip_suffix("-->"))
                    .map(str::trim)
            })
            .collect();
        let bundled: Vec<&str> = PARTS.iter().map(|(name, _)| *name).collect();
        let mut sorted_included = included.clone();
        sorted_included.sort_unstable();
        let mut sorted_bundled = bundled.clone();
        sorted_bundled.sort_unstable();
        assert_eq!(sorted_included, sorted_bundled);
        assert!(build(TEMPLATE, PARTS).is_ok());
        assert!(build("<!--#include missing.js -->", PARTS).is_err());
    }

    #[test]
    fn no_part_closes_the_element_it_sits_in() {
        for (name, text) in PARTS {
            let lower = text.to_ascii_lowercase();
            assert!(
                !lower.contains("</script") && !lower.contains("</style"),
                "{name}"
            );
        }
        let page = page();
        assert!(page.starts_with("<!doctype html>"));
        assert!(!page.contains("<!--#include"));
    }

    #[test]
    fn copied_files_say_where_they_come_from() {
        for (name, text) in PARTS {
            if name.starts_with("vjk/") {
                assert!(
                    text.contains("VisionJK"),
                    "{name} lacks its provenance note"
                );
            }
        }
    }
}
