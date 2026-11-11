//! Builds the PK3 a player uploads from the files of the model they wear, wherever
//! those were installed: only `models/players/<name>/` and `shaders/<name>.shader`
//! are kept, paths are lowercased, the shader file keeps only the model's own
//! shaders, and the result is checked before it is returned.

use crate::{Checked, Limits, Rejection, check, model_folder, shader_file, valid_name};
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

/// Packs model `name` from `files` (game paths with their bytes, in any case; the
/// files of other models or folders are left out) and checks the result.
pub fn pack(
    name: &str,
    files: impl IntoIterator<Item = (String, Vec<u8>)>,
    limits: &Limits,
) -> Result<(Vec<u8>, Checked), Rejection> {
    let name = name.to_ascii_lowercase();
    if !valid_name(&name) {
        return Err(Rejection::BadName(name));
    }
    let folder = model_folder(&name);
    let shader = shader_file(&name);
    let mut kept: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .map(|(path, bytes)| (path.replace('\\', "/").to_ascii_lowercase(), bytes))
        .filter(|(path, _)| path.starts_with(&folder) || *path == shader)
        .filter_map(|(path, bytes)| {
            if path != shader {
                return Some((path, bytes));
            }
            // A community shader file often defines its sibling models' shaders too
            // (`krovos.shader` holds `krovos2`'s): keep only this model's.
            let text = String::from_utf8(bytes).ok()?;
            let own = crate::shader::keep_own(&text, &folder).ok()?;
            (!own.trim().is_empty()).then(|| (path, own.into_bytes()))
        })
        .collect();
    kept.sort_by(|a, b| a.0.cmp(&b.0));
    // Same bytes, packed the same way, give the same PK3: fixed times, sorted names.
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::default());
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let failed = |e: &dyn std::fmt::Display| Rejection::NotAZip(e.to_string());
    for (path, bytes) in &kept {
        zip.start_file(path.as_str(), options)
            .map_err(|e| failed(&e))?;
        zip.write_all(bytes).map_err(|e| failed(&e))?;
    }
    let pk3 = zip.finish().map_err(|e| failed(&e))?.into_inner();
    let checked = check(&pk3, limits)?;
    Ok((pk3, checked))
}
