//! The client's side of model sync (feature `client`): the Sith cloud's `/models`
//! (docs/model-sync.md, "Wire"), and the record of what was fetched.
//!
//! Blocking calls, for a worker thread. Every download is checked with [`check`]
//! before it is written, whatever the server said of it.

use crate::{Checked, Limits, check, valid_name};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use sjk_jof_content::Directory;

/// The answer to an index or a small call, at most.
const ANSWER_LIMIT: u64 = 4 * 1024 * 1024;

/// The HTTP client, named to the Sith cloud as `SJK/<version> (model sync)`.
pub fn agent(version: &str) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .http_status_as_error(false)
        .user_agent(format!("SJK/{version} (model sync)"))
        .build()
        .into()
}

/// One uploaded model's current version, as the index lists it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub version: u32,
    pub size: u64,
    /// The PK3 file's MD5 (hex).
    pub md5: String,
    /// [`Checked::content_md5`].
    pub content_md5: String,
    /// When it was sent (Unix seconds).
    #[serde(default)]
    pub at: u64,
}

/// A name the hosts' own PK3s hold, taken from the Sith cloud card instead.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Pack {
    pub group: String,
    pub file: String,
}

/// `GET /models/index.json`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Index {
    #[serde(default)]
    pub models: BTreeMap<String, Entry>,
    #[serde(default)]
    pub packs: BTreeMap<String, Pack>,
}

/// What an upload became.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct Uploaded {
    pub version: u32,
    /// The current version already had this content: nothing was stored.
    #[serde(default)]
    pub linked: bool,
}

/// Why a call failed: the server's error code (and its reason for `rejected`), or a
/// connection's failure (`code` empty).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudError {
    pub code: String,
    pub why: String,
}

impl CloudError {
    fn failed(why: impl fmt::Display) -> Self {
        Self {
            code: String::new(),
            why: why.to_string(),
        }
    }
}

impl fmt::Display for CloudError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self.code.as_str() {
            "" => return f.write_str(&self.why),
            "rejected" if !self.why.is_empty() => return write!(f, "refused: {}", self.why),
            "bad_ticket" => "the upload ticket was refused; try again",
            "too_large" => "the model is too large",
            "rate_limited" => {
                "not now: this model was updated less than an hour ago, or you sent your 5 uploads of the day; try again later"
            }
            "not_found" => "the Sith cloud has no such model",
            other => return write!(f, "the Sith cloud said {other}"),
        };
        f.write_str(text)
    }
}

impl std::error::Error for CloudError {}

/// The Sith cloud's model sync, at the root its repository is served from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cloud {
    base: String,
}

impl Cloud {
    /// The cloud whose repository files are at `files` (`.../repository`).
    pub fn from_files_url(files: &str) -> Self {
        let files = files.trim().trim_end_matches('/');
        let base = files.strip_suffix("/repository").unwrap_or(files);
        Self {
            base: base.to_owned(),
        }
    }

    /// The cloud at `url` when the player set one (`sith.cloudUrl`), else the one the
    /// hub names for the `sith` collection.
    pub fn find(agent: &ureq::Agent, url: &str, hub: &str) -> Result<Self, CloudError> {
        if !url.trim().is_empty() {
            return Ok(Self::from_files_url(url));
        }
        let directory = Directory {
            hub: hub.to_owned(),
            collection: "sith",
            only: None,
        };
        let sources = sjk_jof_content::download::sources(agent, Some(&directory), &[])
            .map_err(CloudError::failed)?;
        sources
            .first()
            .map(|source| Self::from_files_url(source.files))
            .ok_or_else(|| CloudError::failed("the SJK hub names no Sith cloud"))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/models{path}", self.base)
    }

    /// The uploaded models and the hosts' names.
    pub fn index(&self, agent: &ureq::Agent) -> Result<Index, CloudError> {
        let response = agent
            .get(self.url("/index.json"))
            .call()
            .map_err(CloudError::failed)?;
        let bytes = answer(response, ANSWER_LIMIT)?;
        serde_json::from_slice(&bytes)
            .map_err(|_| CloudError::failed("the Sith cloud's model list is unreadable"))
    }

    /// Sends `pk3` as model `name` with the hub's `ticket`.
    pub fn upload(
        &self,
        agent: &ureq::Agent,
        name: &str,
        pk3: &[u8],
        ticket: &str,
    ) -> Result<Uploaded, CloudError> {
        let response = agent
            .put(self.url(&format!("/{name}")))
            .config()
            .timeout_global(Some(Duration::from_secs(300)))
            .build()
            .header("Authorization", format!("SJK-Ticket {ticket}"))
            .header("Content-Type", "application/octet-stream")
            .send(pk3)
            .map_err(CloudError::failed)?;
        let bytes = answer(response, ANSWER_LIMIT)?;
        serde_json::from_slice(&bytes)
            .map_err(|_| CloudError::failed("the Sith cloud's answer is unreadable"))
    }

    /// Makes the version a staff rollback `ticket` names current again.
    pub fn rollback(
        &self,
        agent: &ureq::Agent,
        name: &str,
        ticket: &str,
    ) -> Result<u32, CloudError> {
        let response = agent
            .post(self.url(&format!("/{name}/rollback")))
            .header("Authorization", format!("SJK-Ticket {ticket}"))
            .send_empty()
            .map_err(CloudError::failed)?;
        let bytes = answer(response, ANSWER_LIMIT)?;
        serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| value["version"].as_u64())
            .and_then(|version| u32::try_from(version).ok())
            .ok_or_else(|| CloudError::failed("the Sith cloud's answer is unreadable"))
    }

    /// Fetches `entry` of model `name` and checks it: the file's MD5, every rule of
    /// [`check`], and that it is that model with that content.
    pub fn download(
        &self,
        agent: &ureq::Agent,
        name: &str,
        entry: &Entry,
        limits: &Limits,
    ) -> Result<(Vec<u8>, Checked), CloudError> {
        if !valid_name(name) {
            return Err(CloudError::failed(format!("{name:?} is not a model name")));
        }
        if entry.size > limits.pk3_bytes {
            return Err(CloudError::failed("the model is over the size limit"));
        }
        let response = agent
            .get(self.url(&format!("/{name}/{}.pk3", entry.version)))
            .config()
            .timeout_global(Some(Duration::from_secs(300)))
            .build()
            .call()
            .map_err(CloudError::failed)?;
        let bytes = answer(response, limits.pk3_bytes)?;
        verify(name, entry, bytes, limits)
    }
}

/// [`Cloud::download`]'s checks on bytes already fetched.
pub fn verify(
    name: &str,
    entry: &Entry,
    bytes: Vec<u8>,
    limits: &Limits,
) -> Result<(Vec<u8>, Checked), CloudError> {
    use md5::{Digest, Md5};
    let md5: String = Md5::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if !md5.eq_ignore_ascii_case(&entry.md5) {
        return Err(CloudError::failed("the download is not the file listed"));
    }
    let checked =
        check(&bytes, limits).map_err(|why| CloudError::failed(format!("refused: {why}")))?;
    if checked.name != name || !checked.content_md5.eq_ignore_ascii_case(&entry.content_md5) {
        return Err(CloudError::failed("the download is another model"));
    }
    Ok((bytes, checked))
}

/// The body of a response, or the server's error for a status of 400 and above.
fn answer(
    mut response: ureq::http::Response<ureq::Body>,
    limit: u64,
) -> Result<Vec<u8>, CloudError> {
    let status = response.status().as_u16();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(CloudError::failed)?;
    if status < 400 {
        return Ok(bytes);
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    let code = value["error"].as_str().map_or_else(
        || {
            if status == 404 {
                "not_found".to_owned()
            } else {
                format!("http {status}")
            }
        },
        str::to_owned,
    );
    Err(CloudError {
        code,
        why: value["why"].as_str().unwrap_or_default().to_owned(),
    })
}

/// The record of fetched models, beside the game's folders.
pub const RECORD: &str = "sjk-model-sync.json";

/// A fetched model's file name. It sorts after the hosts' own packs (`zz_*.pk3`), so
/// a newer version shared over one of theirs is the one the game loads.
pub fn file_name(name: &str) -> String {
    format!("zzz_sjk-model-{name}.pk3")
}

/// One fetched model.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Fetched {
    pub version: u32,
    pub md5: String,
    /// The file, relative to the game data folder.
    pub file: PathBuf,
}

/// `GameData/sjk-model-sync.json`: which models model sync fetched, so it replaces or
/// removes only its own files.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Record {
    #[serde(default)]
    pub models: BTreeMap<String, Fetched>,
}

impl Record {
    /// The record in `game_data`; empty when there is none or it is unreadable.
    pub fn load(game_data: &Path) -> Self {
        std::fs::read(game_data.join(RECORD))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, game_data: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        write_atomic(&game_data.join(RECORD), &bytes)
    }

    /// Writes a checked download of `name` into `base` (a folder inside `game_data`),
    /// replacing the file it fetched before, and records it.
    pub fn install(
        &mut self,
        game_data: &Path,
        base: &Path,
        name: &str,
        entry: &Entry,
        bytes: &[u8],
    ) -> std::io::Result<PathBuf> {
        let path = base.join(file_name(name));
        std::fs::create_dir_all(base)?;
        write_atomic(&path, bytes)?;
        let file = path
            .strip_prefix(game_data)
            .map(Path::to_owned)
            .unwrap_or_else(|_| path.clone());
        if let Some(old) = self.models.get(name)
            && game_data.join(&old.file) != path
        {
            let _ = std::fs::remove_file(game_data.join(&old.file));
        }
        self.models.insert(
            name.to_owned(),
            Fetched {
                version: entry.version,
                md5: entry.md5.to_ascii_lowercase(),
                file,
            },
        );
        self.save(game_data)?;
        Ok(path)
    }

    /// Whether `entry` is newer than what was fetched of `name` (or it changed).
    pub fn wants(&self, name: &str, entry: &Entry) -> bool {
        self.models
            .get(name)
            .is_none_or(|fetched| !fetched.md5.eq_ignore_ascii_case(&entry.md5))
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let partial = path.with_extension("partial");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cloud_is_the_repositorys_root() {
        let cloud = Cloud::from_files_url("http://example.test:8080/repository/");
        assert_eq!(
            cloud.url("/index.json"),
            "http://example.test:8080/models/index.json"
        );
        let cloud = Cloud::from_files_url("http://example.test:8080");
        assert_eq!(cloud.url("/x"), "http://example.test:8080/models/x");
    }

    #[test]
    fn errors_read_as_players_read_them() {
        let refused = CloudError {
            code: "rejected".into(),
            why: "a.dll: this kind of file is not allowed".into(),
        };
        assert_eq!(
            refused.to_string(),
            "refused: a.dll: this kind of file is not allowed"
        );
        let limited = CloudError {
            code: "rate_limited".into(),
            why: String::new(),
        };
        assert!(limited.to_string().contains("less than an hour ago"));
    }

    #[test]
    fn a_download_must_be_the_listed_model() {
        let (pk3, checked) =
            crate::pack::pack("solfox", crate::tests::good(), &Limits::DEFAULT).unwrap();
        use md5::{Digest, Md5};
        let md5: String = Md5::digest(&pk3)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let entry = Entry {
            version: 2,
            size: pk3.len() as u64,
            md5: md5.clone(),
            content_md5: checked.content_md5.clone(),
            at: 0,
        };
        assert!(verify("solfox", &entry, pk3.clone(), &Limits::DEFAULT).is_ok());
        assert!(verify("kyle", &entry, pk3.clone(), &Limits::DEFAULT).is_err());
        let wrong = Entry {
            md5: "0".repeat(32),
            ..entry.clone()
        };
        assert!(verify("solfox", &wrong, pk3.clone(), &Limits::DEFAULT).is_err());
        let other = Entry {
            content_md5: "0".repeat(32),
            ..entry.clone()
        };
        assert!(verify("solfox", &other, pk3, &Limits::DEFAULT).is_err());
    }

    #[test]
    fn the_record_replaces_only_its_own_file() {
        let game_data = tempfile::tempdir().unwrap();
        let base = game_data.path().join("base");
        let entry = Entry {
            version: 1,
            size: 3,
            md5: "AB".repeat(16),
            content_md5: String::new(),
            at: 0,
        };
        let mut record = Record::load(game_data.path());
        assert!(record.wants("solfox", &entry));
        let path = record
            .install(game_data.path(), &base, "solfox", &entry, b"pk3")
            .unwrap();
        assert_eq!(path, base.join("zzz_sjk-model-solfox.pk3"));
        let record = Record::load(game_data.path());
        assert!(!record.wants("solfox", &entry));
        assert_eq!(
            record.models["solfox"].file,
            Path::new("base").join("zzz_sjk-model-solfox.pk3")
        );
        let newer = Entry {
            version: 2,
            md5: "cd".repeat(16),
            ..entry
        };
        assert!(record.wants("solfox", &newer));
        // Moving to the assets folder removes the file it fetched in base.
        let mut record = record;
        let assets = game_data.path().join("assets").join("S").join("base");
        record
            .install(game_data.path(), &assets, "solfox", &newer, b"pk3 2")
            .unwrap();
        assert!(!path.exists());
        assert!(assets.join("zzz_sjk-model-solfox.pk3").exists());
    }
}
