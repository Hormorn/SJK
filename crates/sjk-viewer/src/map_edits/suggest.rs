//! Texture guesses (Sol, 10/11/2026: "improve the in game editor with textures
//! guesses"): the Suggested rows at the head of Draw another shader, for any selection
//! that draws a shader, each with why it is offered.
//!
//! - **Nearby**: what the drawn surfaces round the selection use ([`super::nearby`]'s
//!   vote): on caulk, nodraw and sky faces Fill's candidates; on drawn surfaces the same
//!   vote asked by the surfaces' own triangles, the surfaces themselves not voting.
//! - **Same family**: the shaders of the selection's own folder whose name starts as its
//!   own does ([`family`]): `wall`, `wall2`, `wall_trim`, `wall_dark`.
//! - **On this map**: the textures the map's own drawn surfaces use most (the vote's
//!   surfaces, so never a tool, sky, liquid or effect shader).
//!
//! Each list is made when the selection changes, never per frame: the vote's surfaces and
//! the map's use are kept per map with the brush cache (`web::caulk::Cache`).

use super::nearby::Asking;
use glam::Vec3;

/// Why a shader is suggested.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Reason {
    /// Its share of the nearby vote.
    Nearby(f32),
    /// Its name is of the selection's own family.
    Family,
    /// How many of the map's drawn surfaces use it.
    Map(u32),
}

impl Reason {
    /// The row's mark at its right: "62 % nearby", "same family", "140 on map".
    pub(crate) fn mark(self) -> String {
        match self {
            Self::Nearby(share) => format!("{:.0} % nearby", share * 100.0),
            Self::Family => "same family".to_owned(),
            Self::Map(count) => format!("{count} on map"),
        }
    }

    /// A tile's band: shorter.
    pub(crate) fn band(self) -> String {
        match self {
            Self::Nearby(share) => format!("{:.0} %", share * 100.0),
            Self::Family => "family".to_owned(),
            Self::Map(count) => format!("{count}x"),
        }
    }
}

/// The nearby vote's shaders with their shares, and the map's most used with their counts.
pub(crate) type Votes = (Vec<(String, f32)>, Vec<(String, u32)>);

/// How many Suggested rows a list starts with at most, and of each kind.
pub(crate) const MOST: usize = 10;
const MOST_NEARBY: usize = 6;
const MOST_FAMILY: usize = 4;
const MOST_MAP: usize = 3;
/// How many triangles of a selection of surfaces ask the vote.
const ASKING_TRIANGLES: usize = 64;

/// The words of a shader's last name part, letters only, lower case: `s_wall2b` is `s`,
/// `wall`, `b`; digits split words and are left out (they number variants).
pub(crate) fn words(shader: &str) -> Vec<String> {
    let base = shader.rsplit('/').next().unwrap_or(shader);
    base.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// The shaders of `shaders` (lower case, sorted) in `own`'s folder whose name starts with
/// the same words as `own`'s, the closest first: those sharing the most leading words,
/// then the nearest in length. One shared word must be three letters at least (`s_wall`
/// and `s_floor` share only `s`).
pub(crate) fn family(own: &str, shaders: &[String]) -> Vec<String> {
    let own = super::panel::without_image_extension(own).to_ascii_lowercase();
    let folder = &own[..own.rfind('/').map_or(0, |slash| slash + 1)];
    let mine = words(&own);
    if mine.is_empty() || folder.is_empty() {
        return Vec::new();
    }
    // The folder's shaders lie together in the sorted list.
    let start = shaders.partition_point(|shader| shader.as_str() < folder);
    let mut kin: Vec<(usize, usize, &String)> = shaders[start..]
        .iter()
        .take_while(|shader| shader.starts_with(folder))
        .filter(|shader| !shader[folder.len()..].contains('/') && **shader != own)
        .filter_map(|shader| {
            let theirs = words(shader);
            let shared = mine.iter().zip(&theirs).take_while(|(a, b)| a == b).count();
            (shared >= 2 || (shared == 1 && mine[0].len() >= 3))
                .then(|| (shared, shader.len().abs_diff(own.len()), shader))
        })
        .collect();
    kin.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(b.2)));
    kin.into_iter()
        .take(MOST_FAMILY)
        .map(|(_, _, shader)| shader.clone())
        .collect()
}

/// The Suggested rows: the nearby vote's first, then the family, then the map's most
/// used, each once, never `own` (the shader drawn there now), [`MOST`] at most. `listed`
/// keeps a shader the list offers (a remap may draw it).
pub(crate) fn merge(
    own: Option<&str>,
    nearby: &[(String, f32)],
    family: &[String],
    usage: &[(String, u32)],
    listed: impl Fn(&str) -> bool,
) -> Vec<(String, Reason)> {
    let own = own.map(|own| super::panel::without_image_extension(own).to_ascii_lowercase());
    let mut out: Vec<(String, Reason)> = Vec::new();
    let offer = |shader: &str, reason: Reason, out: &mut Vec<(String, Reason)>| {
        let shader = super::panel::without_image_extension(shader).to_ascii_lowercase();
        if out.len() >= MOST
            || own.as_deref() == Some(shader.as_str())
            || out.iter().any(|(known, _)| *known == shader)
            || !listed(&shader)
        {
            return false;
        }
        out.push((shader, reason));
        true
    };
    for (shader, share) in nearby.iter().take(MOST_NEARBY) {
        offer(shader, Reason::Nearby(*share), &mut out);
    }
    for shader in family {
        offer(shader, Reason::Family, &mut out);
    }
    let mut from_map = 0;
    for (shader, count) in usage {
        if from_map == MOST_MAP {
            break;
        }
        if offer(shader, Reason::Map(*count), &mut out) {
            from_map += 1;
        }
    }
    out
}

impl crate::GpuState {
    /// The nearby vote for the selected drawn surfaces (BSP numbers): what the surfaces
    /// round them use, asked by their own triangles ([`ASKING_TRIANGLES`] at most), the
    /// surfaces themselves not voting; and the map's most used shaders.
    pub(super) fn map_edit_surface_vote(&mut self, surfaces: &[usize]) -> Votes {
        let map = self.world_load_map.clone();
        let catalog = self.shaders.clone();
        let (_, drawn) = self
            .map_edits
            .web_cache
            .sides_and_drawn(&map, &self.bsp, Some(&catalog));
        let usage: Vec<(String, u32)> = drawn.usage().iter().take(16).cloned().collect();
        if surfaces.is_empty() {
            return (Vec::new(), usage);
        }
        let per_surface = ASKING_TRIANGLES.div_ceil(surfaces.len()).max(1);
        let triangles: Vec<([Vec3; 3], Vec3, f32)> = surfaces
            .iter()
            .flat_map(|&index| {
                super::nearby::surface_asking(
                    &super::nearby::surface_triangles(&self.bsp, index),
                    per_surface,
                )
            })
            .take(ASKING_TRIANGLES)
            .collect();
        let asking: Vec<Asking<'_>> = triangles
            .iter()
            .map(|(polygon, normal, distance)| Asking {
                polygon,
                normal: *normal,
                distance: *distance,
            })
            .collect();
        let nearby = drawn
            .vote_except(&asking, surfaces)
            .into_iter()
            .map(|candidate| (candidate.shader, candidate.share))
            .collect();
        (nearby, usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(names: &[&str]) -> Vec<String> {
        let mut list: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        list.sort();
        list
    }

    #[test]
    fn a_family_shares_the_leading_words_of_its_folder() {
        let shaders = list(&[
            "textures/a/wall",
            "textures/a/wall2",
            "textures/a/wall_trim",
            "textures/a/wall_dark",
            "textures/a/wallpaper",
            "textures/a/floor",
            "textures/a/sub/wall",
            "textures/b/wall",
            "textures/a/s_wall1",
            "textures/a/s_wall2",
            "textures/a/s_floor",
        ]);
        let kin = family("textures/a/wall", &shaders);
        assert_eq!(
            kin,
            [
                "textures/a/wall2",
                "textures/a/wall_dark",
                "textures/a/wall_trim"
            ],
            "the folder only, not wallpaper (another word), not sub-folders"
        );
        // two shared words beat one; `s` alone is too short to make a family
        let kin = family("textures/a/s_wall1.tga", &shaders);
        assert_eq!(kin, ["textures/a/s_wall2"]);
        assert!(family("floor", &shaders).is_empty());
        assert_eq!(words("textures/x/s_Wall2b"), ["s", "wall", "b"]);
    }

    #[test]
    fn suggestions_come_nearby_first_then_family_then_the_map_each_once() {
        let nearby = vec![
            ("textures/a/stone".to_owned(), 0.62),
            ("textures/a/OWN".to_owned(), 0.2),
            ("textures/a/trim".to_owned(), 0.18),
        ];
        let family = vec!["textures/a/own2".to_owned(), "textures/a/stone".to_owned()];
        let usage = vec![
            ("textures/a/stone".to_owned(), 140),
            ("textures/system/unlisted".to_owned(), 90),
            ("textures/a/floor".to_owned(), 60),
            ("textures/a/ceiling".to_owned(), 30),
            ("textures/a/pipe".to_owned(), 20),
            ("textures/a/rail".to_owned(), 10),
        ];
        let rows = merge(Some("textures/a/own.tga"), &nearby, &family, &usage, |s| {
            !s.contains("unlisted")
        });
        let names: Vec<&str> = rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "textures/a/stone",
                "textures/a/trim",
                "textures/a/own2",
                "textures/a/floor",
                "textures/a/ceiling",
                "textures/a/pipe",
            ],
            "own left out, each once, three from the map"
        );
        assert_eq!(rows[0].1, Reason::Nearby(0.62));
        assert_eq!(rows[2].1, Reason::Family);
        assert_eq!(rows[3].1, Reason::Map(60));
        assert_eq!(rows[0].1.mark(), "62 % nearby");
        assert_eq!(rows[3].1.mark(), "60 on map");
        assert_eq!(rows[3].1.band(), "60x");
    }
}
