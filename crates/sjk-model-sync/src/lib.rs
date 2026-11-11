//! Model sync's shared checks: what a player model pack sent to the Sith cloud may
//! contain ([docs/model-sync.md](../../../docs/model-sync.md)).
//!
//! The same [`check`] runs in three places: the client before it sends a pack, the
//! Sith cloud before it publishes one, and the client again before it loads a
//! download. A pack holds one model, `models/players/<name>/`, and at most its
//! `shaders/<name>.shader`; any rule broken refuses the whole pack.

#[cfg(feature = "client")]
pub mod client;
mod image;
pub mod pack;
mod shader;

use md5::{Digest, Md5};
use std::fmt;
use std::io::{Cursor, Read};

/// The size limits a pack must fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The PK3 file itself.
    pub pk3_bytes: u64,
    /// Every entry unpacked, together.
    pub unpacked_bytes: u64,
    /// How many times its packed size an entry may unpack to (zip bombs); entries
    /// under [`Limits::RATIO_FREE_BYTES`] are not held to it.
    pub max_ratio: u64,
    pub files: usize,
    /// A texture's width or height.
    pub texture_side: u32,
    /// Every texture as RGBA (width * height * 4), together.
    pub texture_rgba_bytes: u64,
    /// Every GLM's vertices over all its LODs, together.
    pub glm_vertices: usize,
    /// A `.skin` or `.shader` file.
    pub text_bytes: u64,
}

impl Limits {
    /// The limits the Sith cloud and SJK use (docs/model-sync.md, "Limits").
    pub const DEFAULT: Self = Self {
        pk3_bytes: 25 * 1024 * 1024,
        unpacked_bytes: 64 * 1024 * 1024,
        max_ratio: 40,
        files: 128,
        texture_side: 4096,
        texture_rgba_bytes: 256 * 1024 * 1024,
        glm_vertices: 200_000,
        text_bytes: 64 * 1024,
    };

    /// Entries this small are not held to [`Limits::max_ratio`]: text and blank
    /// textures compress far better than 40 to 1 without being a bomb.
    pub const RATIO_FREE_BYTES: u64 = 64 * 1024;
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The longest model name a pack may carry.
pub const MAX_NAME_LEN: usize = 32;

/// Why a pack was refused. The text is shown to the player who sent it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    TooLarge {
        what: &'static str,
        limit: u64,
    },
    NotAZip(String),
    Empty,
    BadPath(String),
    /// A file outside the model's folder and its shader file.
    Outside(String),
    /// Two models' folders in one pack.
    TwoModels(String, String),
    BadName(String),
    Duplicate(String),
    FileType(String),
    /// A file that does not parse as what its extension says.
    Broken {
        path: String,
        why: String,
    },
    /// A shader file defining a shader outside the model's folder.
    ForeignShader(String),
    NoModel,
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { what, limit } => write!(f, "{what} is over the limit ({limit})"),
            Self::NotAZip(why) => write!(f, "not a readable PK3: {why}"),
            Self::Empty => f.write_str("the pack is empty"),
            Self::BadPath(path) => write!(f, "bad file path: {path}"),
            Self::Outside(path) => write!(f, "{path} is not part of the model"),
            Self::TwoModels(a, b) => write!(f, "one model per pack ({a} and {b})"),
            Self::BadName(name) => write!(
                f,
                "model name {name:?} must be 1-{MAX_NAME_LEN} of a-z 0-9 _ -"
            ),
            Self::Duplicate(path) => write!(f, "{path} is in the pack twice"),
            Self::FileType(path) => write!(f, "{path}: this kind of file is not allowed"),
            Self::Broken { path, why } => write!(f, "{path}: {why}"),
            Self::ForeignShader(name) => {
                write!(f, "the shader file defines {name}, outside the model")
            }
            Self::NoModel => f.write_str("the pack has no .glm model"),
        }
    }
}

impl std::error::Error for Rejection {}

/// A pack that passed every check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    /// The model's folder name, `models/players/<name>/`.
    pub name: String,
    /// Each file and its unpacked size, sorted by path.
    pub files: Vec<(String, u64)>,
    /// MD5 (hex) of the content, independent of how the PK3 was packed: the sorted
    /// paths with their sizes and bytes. Two packs of the same files share it, so
    /// the Sith cloud links an upload it already has instead of storing it again.
    pub content_md5: String,
}

/// The folder a model's files sit in.
pub fn model_folder(name: &str) -> String {
    format!("models/players/{name}/")
}

/// The one shader file a pack may carry.
pub fn shader_file(name: &str) -> String {
    format!("shaders/{name}.shader")
}

/// Whether `name` may be a synced model's name.
pub fn valid_name(name: &str) -> bool {
    (1..=MAX_NAME_LEN).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Checks a whole pack against `limits`.
pub fn check(pk3: &[u8], limits: &Limits) -> Result<Checked, Rejection> {
    if pk3.len() as u64 > limits.pk3_bytes {
        return Err(Rejection::TooLarge {
            what: "the PK3",
            limit: limits.pk3_bytes,
        });
    }
    let mut archive =
        zip::ZipArchive::new(Cursor::new(pk3)).map_err(|e| Rejection::NotAZip(e.to_string()))?;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut unpacked = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| Rejection::NotAZip(e.to_string()))?;
        let path = entry.name().to_owned();
        if entry.is_symlink() {
            return Err(Rejection::BadPath(path));
        }
        check_path(&path)?;
        if entry.is_dir() {
            continue;
        }
        if files.len() == limits.files {
            return Err(Rejection::TooLarge {
                what: "the number of files",
                limit: limits.files as u64,
            });
        }
        let declared = entry.size();
        let remaining = limits.unpacked_bytes - unpacked;
        if declared > remaining {
            return Err(Rejection::TooLarge {
                what: "the unpacked size",
                limit: limits.unpacked_bytes,
            });
        }
        if declared > Limits::RATIO_FREE_BYTES
            && declared > entry.compressed_size().saturating_mul(limits.max_ratio)
        {
            return Err(Rejection::Broken {
                path,
                why: format!("unpacks to more than {} times its size", limits.max_ratio),
            });
        }
        // Never trust the declared size: read at most what is left, plus one byte to
        // see an entry that lies.
        let mut bytes = Vec::with_capacity(declared as usize);
        (&mut entry)
            .take(remaining + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| Rejection::Broken {
                path: path.clone(),
                why: e.to_string(),
            })?;
        if bytes.len() as u64 != declared {
            return Err(Rejection::Broken {
                path,
                why: "its size is not the one the PK3 states".into(),
            });
        }
        unpacked += declared;
        files.push((path, bytes));
    }
    check_files(files, limits)
}

/// Checks a pack's files, already unpacked: the rules [`check`] applies after
/// reading the PK3.
pub fn check_files(
    mut files: Vec<(String, Vec<u8>)>,
    limits: &Limits,
) -> Result<Checked, Rejection> {
    if files.is_empty() {
        return Err(Rejection::Empty);
    }
    for (path, _) in &files {
        check_path(path)?;
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for pair in files.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(Rejection::Duplicate(pair[0].0.clone()));
        }
    }
    let name = model_name(&files)?;
    let folder = model_folder(&name);
    let shader = shader_file(&name);
    let mut rgba = 0u64;
    let mut vertices = 0usize;
    let mut has_glm = false;
    for (path, bytes) in &files {
        let broken = |why: String| Rejection::Broken {
            path: path.clone(),
            why,
        };
        if *path == shader {
            check_text(path, bytes, limits)?;
            let text = std::str::from_utf8(bytes).map_err(|_| broken("not text".into()))?;
            for defined in shader::names(text).map_err(broken)? {
                if !defined.to_ascii_lowercase().starts_with(&folder) {
                    return Err(Rejection::ForeignShader(defined));
                }
            }
            continue;
        }
        if !path.starts_with(&folder) {
            return Err(Rejection::Outside(path.clone()));
        }
        match extension(path) {
            "glm" => {
                let glm = sjk_model::Glm::parse(bytes).map_err(|e| broken(e.to_string()))?;
                vertices += glm
                    .lods
                    .iter()
                    .flat_map(|lod| &lod.surfaces)
                    .map(|surface| surface.vertices.len())
                    .sum::<usize>();
                if vertices > limits.glm_vertices {
                    return Err(Rejection::TooLarge {
                        what: "the model's vertex count",
                        limit: limits.glm_vertices as u64,
                    });
                }
                has_glm = true;
            }
            "skin" => check_text(path, bytes, limits)?,
            kind @ ("tga" | "jpg" | "png") => {
                let (width, height) = image::size(kind, bytes).map_err(|why| broken(why.into()))?;
                if width.max(height) > limits.texture_side {
                    return Err(Rejection::TooLarge {
                        what: "a texture's side",
                        limit: u64::from(limits.texture_side),
                    });
                }
                rgba += u64::from(width) * u64::from(height) * 4;
                if rgba > limits.texture_rgba_bytes {
                    return Err(Rejection::TooLarge {
                        what: "the textures' memory",
                        limit: limits.texture_rgba_bytes,
                    });
                }
            }
            _ => return Err(Rejection::FileType(path.clone())),
        }
    }
    if !has_glm {
        return Err(Rejection::NoModel);
    }
    let mut md5 = Md5::new();
    for (path, bytes) in &files {
        md5.update(path.as_bytes());
        md5.update([0]);
        md5.update((bytes.len() as u64).to_le_bytes());
        md5.update(bytes);
    }
    Ok(Checked {
        name,
        files: files
            .iter()
            .map(|(path, bytes)| (path.clone(), bytes.len() as u64))
            .collect(),
        content_md5: md5
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    })
}

/// A path the game would read as written: lowercase ASCII, `/`-separated, relative,
/// no `.` or `..` part and no empty one (a trailing `/` marks a folder).
fn check_path(path: &str) -> Result<(), Rejection> {
    let bad = || Rejection::BadPath(path.to_owned());
    let allowed = |b: u8| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.' | b'/')
    };
    if path.is_empty() || path.len() > 160 || !path.bytes().all(allowed) {
        return Err(bad());
    }
    let parts = path.strip_suffix('/').unwrap_or(path);
    if parts
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(bad());
    }
    Ok(())
}

/// The one model the pack's `models/players/<name>/` files name.
fn model_name(files: &[(String, Vec<u8>)]) -> Result<String, Rejection> {
    let mut name: Option<&str> = None;
    for (path, _) in files {
        let Some(rest) = path.strip_prefix("models/players/") else {
            continue;
        };
        let Some((folder, _)) = rest.split_once('/') else {
            return Err(Rejection::Outside(path.clone()));
        };
        match name {
            None => name = Some(folder),
            Some(first) if first != folder => {
                return Err(Rejection::TwoModels(first.into(), folder.into()));
            }
            Some(_) => {}
        }
    }
    let name = name.ok_or(Rejection::NoModel)?;
    if !valid_name(name) {
        return Err(Rejection::BadName(name.into()));
    }
    Ok(name.to_owned())
}

fn check_text(path: &str, bytes: &[u8], limits: &Limits) -> Result<(), Rejection> {
    if bytes.len() as u64 > limits.text_bytes {
        return Err(Rejection::TooLarge {
            what: "a text file",
            limit: limits.text_bytes,
        });
    }
    if bytes
        .iter()
        .any(|&b| b == 0 || (b < 0x20 && !matches!(b, b'\t' | b'\r' | b'\n')))
    {
        return Err(Rejection::Broken {
            path: path.into(),
            why: "not plain text".into(),
        });
    }
    Ok(())
}

fn extension(path: &str) -> &str {
    path.rsplit_once('.').map_or("", |(_, extension)| extension)
}

#[cfg(test)]
mod tests;
