//! Model sync in the client (`docs/model-sync.md`): JoF- and Sith-verified players
//! share the model they wear through the Sith cloud, and others fetch it.
//!
//! - `sharemodel` packs the model the player wears ([`sjk_model_sync::pack`]), asks
//!   the hub for an upload ticket and sends the pack to the Sith cloud.
//! - `getmodel <name>` fetches a synced model; `cl_autoModels` (off by default)
//!   fetches those of the players met, and new versions of fetched ones every hour.
//! - `modelrollback <name> <version>` (staff) makes an earlier version current.
//!
//! Every download is checked again here before it is written ([`sjk_model_sync::client`]),
//! and goes where the Sith cloud's files go ([`crate::sith_cloud::folder`]); it loads
//! from the next map. A model the game already has is never fetched. Network calls
//! run on a thread of their own; the frame only hands jobs over and reads results.

use crate::console::ViewerConsole;
use sjk_model_sync::Limits;
use sjk_model_sync::client::{Cloud, Index, Record};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Fetch the synced models of the players met, and keep fetched ones up to date.
pub(crate) const AUTO: &str = "cl_autoModels";
pub(crate) const GET: &str = "getmodel";
pub(crate) const SHARE: &str = "sharemodel";
pub(crate) const ROLLBACK: &str = "modelrollback";
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        GET,
        "Download a player model other players shared through the Sith cloud: getmodel <name>",
    ),
    (
        SHARE,
        "Share the model you wear through the Sith cloud, so others can see it (JoF and Sith verified players)",
    ),
    (
        ROLLBACK,
        "Make an earlier version of a shared model current again: modelrollback <name> <version> (staff)",
    ),
];

/// How long the hub may take to give a ticket.
const TICKET_WAIT: Duration = Duration::from_secs(30);
/// How often the players met are looked at, with `cl_autoModels` on.
const MET_EVERY: Duration = Duration::from_secs(20);
/// How often the fetched models are checked for new versions.
const UPDATE_EVERY: Duration = Duration::from_secs(3600);
/// How long an index is used before it is read again.
const INDEX_FOR: Duration = Duration::from_secs(300);

pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        AUTO,
        false,
        CvarFlags::ARCHIVE,
        "Download by themselves the player models other players shared through the Sith cloud, for the players you meet, and their new versions every hour; off by default (getmodel <name> fetches one)",
    ))?;
    Ok(())
}

/// Where a job talks to and writes.
#[derive(Clone)]
struct Place {
    url: String,
    hub: String,
    game_data: PathBuf,
    base: PathBuf,
}

impl Place {
    fn of(console: &ViewerConsole, game_data: &Path) -> Self {
        Self {
            url: console
                .text_value(crate::sith_cloud::URL)
                .unwrap_or("")
                .to_owned(),
            hub: console.text_value("cl_hubUrl").unwrap_or("").to_owned(),
            game_data: game_data.to_owned(),
            base: crate::sith_cloud::folder(game_data),
        }
    }
}

enum Job {
    /// Fetch these models, each with whether the game already has it (`quiet`: met
    /// players', said only when fetched, and one the game has only when it overrides
    /// a host pack's).
    Fetch(Vec<(String, bool)>, bool),
    /// Read the index again (the Model sync page).
    Index,
    /// Fetch what changed of the models fetched before.
    Update,
    Upload {
        name: String,
        pk3: Vec<u8>,
        ticket: String,
    },
    Rollback {
        name: String,
        ticket: String,
    },
}

/// What the network thread does: one job at a time, with the index it last read.
struct Worker {
    agent: ureq::Agent,
    index: Option<(Instant, String, Arc<Index>)>,
}

impl Worker {
    fn cloud(&self, place: &Place) -> Result<Cloud, String> {
        Cloud::find(&self.agent, &place.url, &place.hub).map_err(|error| error.to_string())
    }

    fn index(&mut self, cloud: &Cloud, place: &Place, fresh: bool) -> Result<Arc<Index>, String> {
        let key = format!("{}|{}", place.url, place.hub);
        if !fresh
            && let Some((at, known, index)) = &self.index
            && *known == key
            && at.elapsed() < INDEX_FOR
        {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(
            cloud
                .index(&self.agent)
                .map_err(|error| error.to_string())?,
        );
        self.index = Some((Instant::now(), key, Arc::clone(&index)));
        Ok(index)
    }

    fn run(&mut self, job: Job, place: &Place) -> Done {
        let lines = match self.try_run(job, place) {
            Ok(lines) => lines,
            Err(why) => vec![format!("^3Model sync: {why}.")],
        };
        Done {
            lines,
            index: self.index.as_ref().map(|(_, _, index)| Arc::clone(index)),
        }
    }

    fn try_run(&mut self, job: Job, place: &Place) -> Result<Vec<String>, String> {
        let cloud = self.cloud(place)?;
        match job {
            Job::Fetch(names, quiet) => {
                let index = self.index(&cloud, place, !quiet)?;
                Ok(names
                    .iter()
                    .filter(|(name, installed)| {
                        !(quiet && *installed && !index.packs.contains_key(name))
                    })
                    .filter_map(|(name, _)| self.fetch(&cloud, &index, place, name, quiet))
                    .collect())
            }
            Job::Index => {
                self.index(&cloud, place, true)?;
                Ok(Vec::new())
            }
            Job::Update => {
                let index = self.index(&cloud, place, true)?;
                let record = Record::load(&place.game_data);
                let names: Vec<String> = record
                    .models
                    .keys()
                    .filter(|name| {
                        index
                            .models
                            .get(*name)
                            .is_some_and(|entry| record.wants(name, entry))
                    })
                    .cloned()
                    .collect();
                Ok(names
                    .iter()
                    .filter_map(|name| self.fetch(&cloud, &index, place, name, true))
                    .collect())
            }
            Job::Upload { name, pk3, ticket } => {
                let uploaded = cloud
                    .upload(&self.agent, &name, &pk3, &ticket)
                    .map_err(|error| error.to_string())?;
                let _ = self.index(&cloud, place, true);
                Ok(vec![if uploaded.linked {
                    format!(
                        "Model sync: {name} is already on the Sith cloud as it is (version {}).",
                        uploaded.version
                    )
                } else {
                    format!(
                        "Model sync: {name} shared, version {}. Players with it download it from their next map.",
                        uploaded.version
                    )
                }])
            }
            Job::Rollback { name, ticket } => {
                let version = cloud
                    .rollback(&self.agent, &name, &ticket)
                    .map_err(|error| error.to_string())?;
                let _ = self.index(&cloud, place, true);
                Ok(vec![format!(
                    "Model sync: {name} is back to version {version}."
                )])
            }
        }
    }

    /// Fetches one model, or says why not (nothing in `quiet` but a success).
    fn fetch(
        &self,
        cloud: &Cloud,
        index: &Index,
        place: &Place,
        name: &str,
        quiet: bool,
    ) -> Option<String> {
        let loud = |text: String| (!quiet).then_some(text);
        let Some(entry) = index.models.get(name) else {
            if index.packs.contains_key(name) {
                return loud(format!(
                    "Model sync: nobody shared a newer {name}; the Sith cloud card (sith.downloads) has the hosts' pack."
                ));
            }
            return loud(format!(
                "Model sync: nobody shared {name} on the Sith cloud."
            ));
        };
        let mut record = Record::load(&place.game_data);
        if !record.wants(name, entry) {
            return loud(format!("Model sync: you have the latest {name}."));
        }
        let result = cloud
            .download(&self.agent, name, entry, &Limits::DEFAULT)
            .map_err(|error| error.to_string())
            .and_then(|(bytes, _)| {
                record
                    .install(&place.game_data, &place.base, name, entry, &bytes)
                    .map_err(|error| error.to_string())
            });
        Some(match result {
            Ok(_) => format!(
                "Model sync: {name} version {} downloaded; it shows from the next map.",
                entry.version
            ),
            Err(why) => format!("^3Model sync: {name}: {why}."),
        })
    }
}

/// A pack waiting for its ticket.
struct Waiting {
    name: String,
    pk3: Vec<u8>,
    /// The ticket outcome's serial when it was asked for.
    serial: u64,
    at: Instant,
    rollback: bool,
}

/// What the thread sends back after a job: console lines and the index it holds.
struct Done {
    lines: Vec<String>,
    index: Option<Arc<Index>>,
}

/// The jobs sent to the thread and what it sends back.
type Channels = (Sender<(Job, Place)>, Receiver<Done>);

/// Console lines the Model sync page shows, at most.
const RECENT: usize = 6;

struct State {
    jobs: Option<Channels>,
    waiting: Option<Waiting>,
    /// Models asked for this session, so a met player's is asked once.
    asked: BTreeSet<String>,
    /// Models the Inspect card offered this session.
    offered: BTreeSet<String>,
    met_at: Option<Instant>,
    update_at: Option<Instant>,
    /// The index the thread last read, for the Model sync page.
    index: Option<Arc<Index>>,
    /// The last lines said, newest last, for the page.
    recent: Vec<String>,
    /// Jobs sent and not answered yet.
    pending: usize,
}

static STATE: Mutex<State> = Mutex::new(State {
    jobs: None,
    waiting: None,
    asked: BTreeSet::new(),
    offered: BTreeSet::new(),
    met_at: None,
    update_at: None,
    index: None,
    recent: Vec::new(),
    pending: 0,
});

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|error| error.into_inner())
}

impl State {
    fn send(&mut self, job: Job, place: Place) {
        let (jobs, _) = self.jobs.get_or_insert_with(|| {
            let (jobs, incoming) = channel::<(Job, Place)>();
            let (answers, results) = channel();
            std::thread::Builder::new()
                .name("model-sync".into())
                .spawn(move || {
                    let mut worker = Worker {
                        agent: sjk_model_sync::client::agent(crate::build_info::VERSION),
                        index: None,
                    };
                    for (job, place) in incoming {
                        if answers.send(worker.run(job, &place)).is_err() {
                            break;
                        }
                    }
                })
                .expect("the model sync thread starts");
            (jobs, results)
        });
        if jobs.send((job, place)).is_ok() {
            self.pending += 1;
        }
    }

    fn say(&mut self, line: &str) {
        if self.recent.len() == RECENT {
            self.recent.remove(0);
        }
        self.recent.push(line.to_owned());
    }
}

/// `getmodel`, `sharemodel` and `modelrollback`; lines for the console.
pub(crate) fn command(
    name: &str,
    args: &[String],
    console: &ViewerConsole,
    game_data: &Path,
    vfs: Option<&sjk_vfs::VirtualFileSystem>,
    worn: Option<String>,
) -> Result<Vec<String>, String> {
    let place = Place::of(console, game_data);
    match name {
        GET => {
            let model = args
                .first()
                .map(|model| model.trim().to_ascii_lowercase())
                .filter(|model| sjk_model_sync::valid_name(model))
                .ok_or("usage: getmodel <name> (a-z 0-9 _ -)")?;
            let mut state = lock();
            state.asked.insert(model.clone());
            let has = installed(&model);
            state.send(Job::Fetch(vec![(model.clone(), has)], false), place);
            Ok(vec![format!(
                "Model sync: looking for {model} on the Sith cloud..."
            )])
        }
        SHARE => {
            let model = match args.first() {
                Some(model) => model.trim().to_ascii_lowercase(),
                None => {
                    worn.ok_or("you wear no model to share (sharemodel <name> shares another)")?
                }
            };
            if crate::installed_models::models()
                .is_some_and(|models| models.retail.contains(&model))
            {
                return Err(format!(
                    "{model} is one of the game's own models: everyone has it"
                ));
            }
            let vfs = vfs.ok_or("the game data is not loaded yet")?;
            let (pk3, checked) =
                sjk_model_sync::pack::pack(&model, files_of(vfs, &model), &Limits::DEFAULT)
                    .map_err(|why| format!("{model} cannot be shared: {why}"))?;
            if !crate::player_identity::ask_model_ticket(model.clone(), checked.content_md5) {
                return Err("the SJK hub is not connected".into());
            }
            let serial = crate::player_identity::model_ticket().map_or(0, |outcome| outcome.serial);
            lock().waiting = Some(Waiting {
                name: model.clone(),
                pk3,
                serial,
                at: Instant::now(),
                rollback: false,
            });
            Ok(vec![format!(
                "Model sync: {model} packed ({} files, {}); asking the SJK hub...",
                checked.files.len(),
                sjk_jof_content::human_size(checked.files.iter().map(|(_, size)| size).sum())
            )])
        }
        ROLLBACK => {
            if !crate::player_identity::is_staff() {
                return Err("only SJK staff can roll a model back".into());
            }
            let (Some(model), Some(version)) = (
                args.first().map(|model| model.to_ascii_lowercase()),
                args.get(1).and_then(|version| version.parse::<u32>().ok()),
            ) else {
                return Err("usage: modelrollback <name> <version>".into());
            };
            if !sjk_model_sync::valid_name(&model) || version == 0 {
                return Err("usage: modelrollback <name> <version>".into());
            }
            if !crate::player_identity::ask_model_rollback_ticket(model.clone(), version) {
                return Err("the SJK hub is not connected".into());
            }
            let serial = crate::player_identity::model_ticket().map_or(0, |outcome| outcome.serial);
            lock().waiting = Some(Waiting {
                name: model.clone(),
                pk3: Vec::new(),
                serial,
                at: Instant::now(),
                rollback: true,
            });
            Ok(vec![format!(
                "Model sync: rolling {model} back to version {version}..."
            )])
        }
        _ => Err(format!("unknown model sync command {name}")),
    }
}

/// The game's files of model `name`: its folder, and its own shader file.
fn files_of(vfs: &sjk_vfs::VirtualFileSystem, name: &str) -> Vec<(String, Vec<u8>)> {
    let folder = format!("models/players/{name}");
    let mut files = Vec::new();
    for extension in [".glm", ".skin", ".tga", ".jpg", ".png"] {
        for path in vfs.list_files(&folder, extension) {
            let path = format!("{folder}/{path}");
            if let Ok(Some(asset)) = vfs.read(&path) {
                files.push((path, asset.bytes));
            }
        }
    }
    let shader = sjk_model_sync::shader_file(name);
    if let Ok(Some(asset)) = vfs.read(&shader) {
        files.push((shader, asset.bytes));
    }
    files
}

/// The inspected player's `(name, model)`: when this game lacks the model, says once
/// how to fetch it (the Inspect card's "Download this model").
pub(crate) fn offer(
    console: &mut ViewerConsole,
    game_data: &Path,
    inspected: Option<(String, String)>,
) {
    let Some((name, model)) = inspected else {
        return;
    };
    if !sjk_model_sync::valid_name(&model)
        || installed(&model)
        || Record::load(game_data).models.contains_key(&model)
    {
        return;
    }
    let mut state = lock();
    if !state.offered.insert(model.clone()) {
        return;
    }
    drop(state);
    console.push_log(format!(
        "{name}^7 wears {model}, which you don't have: getmodel {model} downloads it if they shared it (or turn on {AUTO})."
    ));
}

fn installed(model: &str) -> bool {
    crate::installed_models::models()
        .is_some_and(|models| models.all.binary_search(&model.to_owned()).is_ok())
}

/// Each frame: hand a ticket that came to its upload, print what the thread finished,
/// and with `cl_autoModels` on, ask for the models of the players met and, hourly,
/// for new versions. A few comparisons while nothing is due.
pub(crate) fn update(
    console: &mut ViewerConsole,
    game_data: &Path,
    met: impl FnOnce() -> Vec<String>,
) {
    let now = Instant::now();
    let mut state = lock();
    let mut lines = Vec::new();
    let mut answers = Vec::new();
    if let Some((_, results)) = &state.jobs {
        while let Ok(done) = results.try_recv() {
            answers.push(done);
        }
    }
    for done in answers {
        state.pending = state.pending.saturating_sub(1);
        if done.index.is_some() {
            state.index = done.index;
        }
        lines.extend(done.lines);
    }
    if let Some(waiting) = &state.waiting {
        let outcome = crate::player_identity::model_ticket()
            .filter(|outcome| outcome.serial > waiting.serial);
        if let Some(outcome) = outcome {
            let waiting = state.waiting.take().expect("a pack waits");
            if outcome.sent {
                let place = Place::of(console, game_data);
                let job = if waiting.rollback {
                    Job::Rollback {
                        name: waiting.name,
                        ticket: outcome.message,
                    }
                } else {
                    Job::Upload {
                        name: waiting.name,
                        pk3: waiting.pk3,
                        ticket: outcome.message,
                    }
                };
                state.send(job, place);
            } else {
                lines.push(format!(
                    "^3Model sync: the SJK hub said no: {}.",
                    outcome.message
                ));
            }
        } else if now.duration_since(waiting.at) > TICKET_WAIT {
            state.waiting = None;
            lines.push("^3Model sync: the SJK hub did not answer; try again.".to_owned());
        }
    }
    if console.bool_cvar(AUTO) == Some(true) {
        if state
            .met_at
            .is_none_or(|at| now.duration_since(at) >= MET_EVERY)
        {
            state.met_at = Some(now);
            let record = Record::load(game_data);
            let wanted: Vec<(String, bool)> = met()
                .into_iter()
                .filter(|model| sjk_model_sync::valid_name(model))
                .filter(|model| !record.models.contains_key(model))
                .filter(|model| !state.asked.contains(model))
                .map(|model| {
                    let has = installed(&model);
                    (model, has)
                })
                .collect();
            if !wanted.is_empty() {
                state
                    .asked
                    .extend(wanted.iter().map(|(model, _)| model.clone()));
                state.send(Job::Fetch(wanted, true), Place::of(console, game_data));
            }
        }
        let first = state.update_at.is_none();
        if first
            || state
                .update_at
                .is_some_and(|at| now.duration_since(at) >= UPDATE_EVERY)
        {
            state.update_at = Some(now);
            if !first || !Record::load(game_data).models.is_empty() {
                state.send(Job::Update, Place::of(console, game_data));
            }
        }
    }
    for line in &lines {
        state.say(line);
    }
    drop(state);
    for line in lines {
        crate::log::progress(format_args!("{line}"));
        console.push_log(line);
    }
}

/// Where a model stands, for the Model sync page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageRow {
    pub(crate) name: String,
    /// The version on the Sith cloud, when someone shared it.
    pub(crate) cloud: Option<u32>,
    /// The version this game fetched through model sync, if any.
    pub(crate) fetched: Option<u32>,
    /// One of the hosts' own packs holds it too (an upload is a newer version of it).
    pub(crate) host_pack: bool,
    /// The game has it (from anywhere), so it can be shared from here.
    pub(crate) installed: bool,
    /// The model the player wears.
    pub(crate) worn: bool,
}

/// What the Model sync page shows: the models this game has that are not the game's
/// own, then those shared on the Sith cloud it lacks; the worn model first, then by
/// name. `None` for the cloud's fields until the index has been read.
pub(crate) fn page_rows(game_data: &Path, worn: Option<&str>) -> Vec<PageRow> {
    let record = Record::load(game_data);
    let index = lock().index.clone();
    let installed_models = crate::installed_models::models();
    let mut names: BTreeSet<String> = installed_models
        .as_ref()
        .map(|models| {
            models
                .all
                .iter()
                .filter(|name| !models.retail.contains(*name))
                .filter(|name| sjk_model_sync::valid_name(name))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if let Some(index) = &index {
        names.extend(index.models.keys().cloned());
    }
    names.extend(record.models.keys().cloned());
    let mut rows: Vec<PageRow> = names
        .into_iter()
        .map(|name| PageRow {
            cloud: index
                .as_ref()
                .and_then(|index| index.models.get(&name))
                .map(|entry| entry.version),
            fetched: record.models.get(&name).map(|fetched| fetched.version),
            host_pack: index
                .as_ref()
                .is_some_and(|index| index.packs.contains_key(&name)),
            installed: installed(&name) || record.models.contains_key(&name),
            worn: worn == Some(name.as_str()),
            name,
        })
        .collect();
    rows.sort_by(|a, b| b.worn.cmp(&a.worn).then_with(|| a.name.cmp(&b.name)));
    rows
}

/// Ask the Sith cloud for its list again (the page asks when it opens).
pub(crate) fn refresh_index(console: &ViewerConsole, game_data: &Path) {
    lock().send(Job::Index, Place::of(console, game_data));
}

/// Whether the index has been read once.
pub(crate) fn has_index() -> bool {
    lock().index.is_some()
}

/// Whether a job or an upload's ticket is still on its way.
pub(crate) fn busy() -> bool {
    let state = lock();
    state.pending > 0 || state.waiting.is_some()
}

/// The last lines model sync said, newest last (colour codes kept).
pub(crate) fn recent_lines() -> Vec<String> {
    lock().recent.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_downloads_are_off_until_the_player_turns_them_on() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.bool_cvar(AUTO), Some(false));
    }

    #[test]
    fn the_shared_files_are_the_models_folder_and_its_shader() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "test",
            [
                ("models/players/solfox/model.glm", b"glm".to_vec()),
                ("models/players/solfox/body.tga", b"tga".to_vec()),
                ("models/players/solfox/sub/eyes.png", b"png".to_vec()),
                ("models/players/solfox/notes.txt", b"txt".to_vec()),
                ("models/players/kyle/model.glm", b"glm".to_vec()),
                ("shaders/solfox.shader", b"x".to_vec()),
                ("shaders/other.shader", b"x".to_vec()),
            ],
        )
        .unwrap();
        let mut paths: Vec<String> = files_of(&vfs, "solfox")
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            [
                "models/players/solfox/body.tga",
                "models/players/solfox/model.glm",
                "models/players/solfox/sub/eyes.png",
                "shaders/solfox.shader",
            ]
        );
    }
}
