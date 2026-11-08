//! The blade-skin editor (`docs/client.md`, "Saber Lab"): a skin of the Saber Lab's
//! folder ([`crate::saber_lab`]) opened as its files ([`FILES`]: the skin file and its
//! additions files), every section and field of them listed as rows, a number
//! with the range its check allows ([`crate::blade_skin_file::ranges`]), a colour with
//! its channels, a switch or a path. Each change is read as the game reads the files
//! (laid over each other, then checked) and, when it reads, put in place of the skin's
//! look at once ([`crate::sjk_packs::edit`]), the stage and the hand showing it the next
//! frame; when it does not, why is shown and the last good look stays. The files are
//! written back in their own order ([`crate::ordered_json`]) a moment after the last
//! change, so the lab does not mount the packs again for them. The sections are the
//! format's own, read from it ([`crate::blade_skin_shape`]): whatever the format can draw
//! may be added, as a shipped skin has it when one does (its values, in its file), else as
//! the format says it is made (its fields of their types, values within their ranges, in
//! the newest additions file). A section or a list's item may be taken out or copied, and
//! every change undone. Before the editor first writes a skin's files, it copies them as
//! they are into the lab folder's [`ORIGINALS`]: kept there from then on, they can always
//! be put back ([`Editor::revert`]).

use crate::blade_skin_file::extras::SkinOption;
use crate::blade_skin_file::{self, BladeSkinDef};
use crate::ordered_json::{Node, Step};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The skin's files, in the order they are laid over each other.
pub(crate) const FILES: [&str; blade_skin_file::FILES.len()] = blade_skin_file::FILES;
const FILE_COUNT: usize = FILES.len();
/// How long after the last change the files are written.
const SAVE_AFTER: Duration = Duration::from_millis(700);
/// Most changes kept to undo.
const UNDO: usize = 200;
/// The lab folder's folder where each skin's files are kept as they were before the
/// editor first wrote them, a folder of its id each (outside `skins/blades`, so the lab
/// does not read them as skins).
pub(crate) const ORIGINALS: &str = "originals";

type Files = [Option<Node>; FILE_COUNT];

/// A section a skin may add, as the first shipped skin holding it has it: its key, the
/// file it goes in, and its values.
#[derive(Clone, Debug)]
struct Template {
    key: String,
    file: usize,
    node: Node,
}

/// What a row of the editor shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    /// A number, the range its check allows when known, and whether it is whole.
    Number {
        value: f64,
        range: Option<(f32, f32)>,
        whole: bool,
    },
    Switch(bool),
    Text(String),
    /// A section, an object or a list: whether its rows show, and how many it holds.
    Group {
        open: bool,
        count: usize,
    },
    /// A colour: its channels, and whether their rows show.
    Colour {
        rgb: [f32; 3],
        open: bool,
    },
    /// A section the skin may add.
    Add,
    /// The heading over the sections to add.
    Heading,
}

/// One row: how deep it is, what it is called, the file and the path of what it shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub(crate) depth: usize,
    pub(crate) label: String,
    pub(crate) file: usize,
    pub(crate) path: Vec<Step>,
    pub(crate) value: Value,
}

impl Row {
    /// Whether the row is a section of the skin (a member of one of its files).
    pub(crate) fn section(&self) -> bool {
        self.path.len() == 1 && self.value != Value::Add
    }

    /// Whether the row is an item of a list.
    pub(crate) fn item(&self) -> bool {
        matches!(self.path.last(), Some(Step::Index(_)))
    }
}

/// How a changed skin is put in place of the shown one: [`crate::sjk_packs::edit`], or a
/// test's recorder.
type Publish = fn(&str, BladeSkinDef) -> Result<(), String>;

/// One skin of the lab's folder, being edited.
pub(crate) struct Editor {
    id: &'static str,
    /// The folder its files are in (the lab's `skins/blades`).
    folder: PathBuf,
    files: Files,
    /// The files as last written (or read).
    written: Files,
    templates: Vec<Template>,
    /// The sections the format has, in its order ([`crate::blade_skin_shape::sections`]).
    sections: Vec<String>,
    ranges: HashMap<String, (f32, f32)>,
    /// The groups whose rows show, by file and path.
    open: BTreeSet<(usize, Vec<Step>)>,
    undo: Vec<Files>,
    redo: Vec<Files>,
    /// When the files last changed and were not written since.
    changed: Option<Instant>,
    /// Why the files as they are do not read (the last good look stays), or could not be
    /// written.
    problem: Option<String>,
    /// The lab folder's stamp as the editor last saw it ([`crate::saber_lab::read_stamp`]).
    stamp: Option<u64>,
    publish: Publish,
    /// Where the skin's original files are kept ([`ORIGINALS`]; none in a test), the
    /// original (as kept, or until it is, the files as they are on disk) and whether it
    /// is kept.
    originals: Option<PathBuf>,
    original: Files,
    kept: bool,
}

impl Editor {
    /// Open skin `id` of the lab's folder, its templates and ranges from the packs.
    pub(crate) fn open(id: &'static str) -> Result<Self, String> {
        let lab = crate::saber_lab::folder().ok_or("the Saber Lab is off")?;
        let folder = lab.join(blade_skin_file::FOLDER);
        let skins = crate::sjk_packs::skins();
        let mut editor = Self::read(id, &folder, skins.packs(), crate::sjk_packs::edit)?;
        editor.stamp = crate::saber_lab::read_stamp();
        editor.originals = Some(lab.join(ORIGINALS).join(id));
        editor.find_original();
        Ok(editor)
    }

    /// Open skin `id`'s files in `folder`, the sections a skin may add and the fields'
    /// ranges from every skin of `packs`.
    fn read(
        id: &'static str,
        folder: &Path,
        packs: &sjk_vfs::VirtualFileSystem,
        publish: Publish,
    ) -> Result<Self, String> {
        let files = read_files(folder, id)?;
        let mut templates: Vec<Template> = Vec::new();
        let mut ranges = HashMap::new();
        let shipped = packs.list_files(blade_skin_file::FOLDER, blade_skin_file::EXTENSION);
        for listed in shipped {
            let skin = listed.trim_end_matches(blade_skin_file::EXTENSION);
            let texts: Vec<Option<Node>> = FILES
                .iter()
                .map(|extension| {
                    let path = format!("{}/{skin}{extension}", blade_skin_file::FOLDER);
                    let asset = packs.read(&path).ok().flatten()?;
                    Node::parse(&String::from_utf8(asset.bytes).ok()?).ok()
                })
                .collect();
            for (file, node) in texts.iter().enumerate() {
                for (key, value) in node.iter().flat_map(Node::members) {
                    if !templates.iter().any(|template| template.key == *key) {
                        templates.push(Template {
                            key: key.clone(),
                            file,
                            node: value.clone(),
                        });
                    }
                }
            }
            let texts: Files = std::array::from_fn(|file| texts[file].clone());
            if let Some(merged) = merged(&texts) {
                learn(&mut ranges, skin, &merged);
            }
        }
        let mut sections = Vec::new();
        if let Some(merged) = merged(&files) {
            learn(&mut ranges, id, &merged);
            sections = crate::blade_skin_shape::sections(id, &merged);
        }
        // A skin the format cannot read yet: the sections the shipped skins hold.
        if sections.is_empty() {
            sections = templates
                .iter()
                .map(|template| template.key.clone())
                .collect();
        }
        sections.retain(|key| key != "version");
        let mut editor = Self {
            id,
            folder: folder.to_path_buf(),
            written: files.clone(),
            original: files.clone(),
            files,
            templates,
            sections,
            ranges,
            open: BTreeSet::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            changed: None,
            problem: None,
            stamp: None,
            publish,
            originals: None,
            kept: false,
        };
        editor.problem = editor.check().err();
        Ok(editor)
    }

    /// The skin edited.
    pub(crate) fn id(&self) -> &'static str {
        self.id
    }

    /// Why the files as they are do not read or could not be written, if they do not.
    pub(crate) fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    /// Whether changes wait to be written.
    pub(crate) fn unsaved(&self) -> bool {
        self.changed.is_some()
    }

    /// Whether there is a change to undo, and one to redo.
    pub(crate) fn can_undo(&self) -> (bool, bool) {
        (!self.undo.is_empty(), !self.redo.is_empty())
    }

    /// Whether the files differ from the skin's original ([`ORIGINALS`]).
    pub(crate) fn revertable(&self) -> bool {
        self.files != self.original
    }

    /// Put the skin's original files back: a change like any other, undone with Undo.
    pub(crate) fn revert(&mut self) {
        let original = self.original.clone();
        self.change(true, |files| {
            *files = original;
            true
        });
    }

    /// The rows, in order: each file's members in their order (the skin file's `version`
    /// left out), a group's rows under it while it is open, then the sections the skin
    /// may add.
    pub(crate) fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for (file, node) in self.files.iter().enumerate() {
            for (key, value) in node.iter().flat_map(Node::members) {
                if file == 0 && key == "version" {
                    continue;
                }
                self.rows_of(&mut rows, file, vec![Step::Key(key.clone())], key, value, 0);
            }
        }
        // The format's sections the skin lacks, in the format's order.
        let adds: Vec<&String> = self.sections.iter().filter(|key| !self.has(key)).collect();
        if !adds.is_empty() {
            rows.push(Row {
                depth: 0,
                label: format!("Add a section ({})", adds.len()),
                file: 0,
                path: Vec::new(),
                value: Value::Heading,
            });
        }
        for key in adds {
            rows.push(Row {
                depth: 0,
                label: key.clone(),
                file: self.file_for(key),
                path: vec![Step::Key(key.clone())],
                value: Value::Add,
            });
        }
        rows
    }

    fn rows_of(
        &self,
        rows: &mut Vec<Row>,
        file: usize,
        path: Vec<Step>,
        label: &str,
        node: &Node,
        depth: usize,
    ) {
        let open = self.open.contains(&(file, path.clone()));
        let row = |value| Row {
            depth,
            label: label.to_owned(),
            file,
            path: path.clone(),
            value,
        };
        match node {
            Node::List(items) if colour(items).is_some() => {
                let rgb = colour(items).unwrap_or_default();
                rows.push(row(Value::Colour { rgb, open }));
                if open {
                    for (index, channel) in ["red", "green", "blue"].into_iter().enumerate() {
                        let mut path = path.clone();
                        path.push(Step::Index(index));
                        let name = format!("{} {channel}", range_name(&path[..path.len() - 1]));
                        rows.push(Row {
                            depth: depth + 1,
                            label: channel.to_owned(),
                            file,
                            path,
                            value: Value::Number {
                                value: f64::from(rgb[index]),
                                range: self.ranges.get(&name).copied(),
                                whole: false,
                            },
                        });
                    }
                }
            }
            Node::List(items) => {
                rows.push(row(Value::Group {
                    open,
                    count: items.len(),
                }));
                if open {
                    for (index, item) in items.iter().enumerate() {
                        let mut path = path.clone();
                        path.push(Step::Index(index));
                        let label = format!("#{}", index + 1);
                        self.rows_of(rows, file, path, &label, item, depth + 1);
                    }
                }
            }
            Node::Object(members) => {
                rows.push(row(Value::Group {
                    open,
                    count: members.len(),
                }));
                if open {
                    for (key, value) in members {
                        let mut path = path.clone();
                        path.push(Step::Key(key.clone()));
                        self.rows_of(rows, file, path, key, value, depth + 1);
                    }
                }
            }
            Node::Int(value) => rows.push(row(Value::Number {
                value: *value as f64,
                range: self.ranges.get(&range_name(&path)).copied(),
                whole: true,
            })),
            Node::Float(value) => rows.push(row(Value::Number {
                value: *value,
                range: self.ranges.get(&range_name(&path)).copied(),
                whole: false,
            })),
            Node::Bool(value) => rows.push(row(Value::Switch(*value))),
            Node::Text(text) => rows.push(row(Value::Text(text.clone()))),
            Node::Null => rows.push(row(Value::Text("null".to_owned()))),
        }
    }

    /// Where the sections to add start, among the rows.
    pub(crate) fn adds_row(rows: &[Row]) -> Option<usize> {
        rows.iter().position(|row| row.value == Value::Heading)
    }

    /// Whether a file of the skin holds section `key`.
    fn has(&self, key: &str) -> bool {
        self.files
            .iter()
            .flatten()
            .any(|node| node.get(key).is_some())
    }

    /// Open or close a group's or a colour's rows.
    pub(crate) fn toggle_open(&mut self, row: &Row) {
        let key = (row.file, row.path.clone());
        if !self.open.remove(&key) {
            self.open.insert(key);
        }
    }

    /// Open every section, or close them all.
    pub(crate) fn open_all(&mut self, open: bool) {
        self.open.clear();
        if open {
            for row in self.rows().into_iter().filter(Row::section) {
                self.open.insert((row.file, row.path));
            }
        }
    }

    /// The step a number of `row` moves by: whole numbers 1, a ranged one a hundredth of
    /// its range, another by its size; rounded to 1, 2 or 5 times a power of ten.
    pub(crate) fn step(row: &Row) -> f64 {
        match row.value {
            Value::Number { whole: true, .. } => 1.0,
            Value::Number {
                range: Some((low, high)),
                ..
            } => nice(f64::from(high - low) / 100.0),
            Value::Number { value, .. } => nice(value.abs().max(0.5) / 50.0),
            _ => 0.0,
        }
    }

    /// Move number `row` by `steps` of its step, within its range.
    pub(crate) fn nudge(&mut self, row: &Row, steps: f64, record: bool) {
        let Value::Number { value, .. } = row.value else {
            return;
        };
        self.set_number(row, value + Self::step(row) * steps, record);
    }

    /// Set number `row` to `share` of its range (0 its least, 1 its most).
    pub(crate) fn set_share(&mut self, row: &Row, share: f64, record: bool) {
        let Value::Number {
            range: Some((low, high)),
            ..
        } = row.value
        else {
            return;
        };
        let step = Self::step(row);
        let value = f64::from(low) + f64::from(high - low) * share.clamp(0.0, 1.0);
        self.set_number(row, (value / step).round() * step, record);
    }

    /// Set number `row` to `value`, within its range, whole when it is.
    pub(crate) fn set_number(&mut self, row: &Row, value: f64, record: bool) {
        let Value::Number { range, whole, .. } = row.value else {
            return;
        };
        let value = match range {
            Some((low, high)) => value.clamp(f64::from(low), f64::from(high)),
            None => value,
        };
        let node = if whole {
            Node::Int(value.round() as i64)
        } else {
            // Six decimals at most, so steps do not leave float dust in the file.
            Node::Float((value * 1e6).round() / 1e6)
        };
        self.change(record, |files| set(files, row, node));
    }

    /// Flip switch `row`.
    pub(crate) fn flip(&mut self, row: &Row) {
        if let Value::Switch(on) = row.value {
            self.change(true, |files| set(files, row, Node::Bool(!on)));
        }
    }

    /// Set text `row` to `text`.
    pub(crate) fn set_text(&mut self, row: &Row, text: &str) {
        if matches!(row.value, Value::Text(_)) {
            let node = Node::Text(text.to_owned());
            self.change(true, |files| set(files, row, node));
        }
    }

    /// Add section `row` as the shipped skin has it, its rows open.
    pub(crate) fn add(&mut self, row: &Row) {
        let key = row.label.clone();
        let file = self.file_for(&key);
        let Some(node) = self.template(&key) else {
            return;
        };
        self.change(true, |files| {
            files[file]
                .get_or_insert_with(|| Node::Object(Vec::new()))
                .insert(&key, node);
            true
        });
        self.open.insert((file, vec![Step::Key(key)]));
    }

    /// Section `key` as the first shipped skin holding it has it, else as the format
    /// makes it (its fields of their types, values within their ranges).
    fn template(&self, key: &str) -> Option<Node> {
        let shipped = self
            .templates
            .iter()
            .find(|template| template.key == key)
            .map(|template| template.node.clone());
        shipped.or_else(|| {
            let skin = merged(&self.files)?;
            Some(crate::blade_skin_shape::section(self.id, &skin, key))
        })
    }

    /// The sections the skin's files hold, laid over each other, in their order.
    pub(crate) fn present(&self) -> Vec<String> {
        merged(&self.files).map_or_else(Vec::new, |node| {
            node.members().iter().map(|(key, _)| key.clone()).collect()
        })
    }

    /// The skin's groups (its `options`, the newest file's), as its files give them.
    pub(crate) fn options(&self) -> Vec<SkinOption> {
        merged(&self.files)
            .and_then(|node| node.get("options").map(options_of))
            .unwrap_or_default()
    }

    /// Give the skin `options` for its groups ([`crate::blade_builder`]), or say why it
    /// would not read with them.
    pub(crate) fn set_options(&mut self, options: &[SkinOption]) -> Result<(), String> {
        self.try_change(|files| {
            write_options(files, options);
            true
        })
    }

    /// Add section `key` as [`Self::template`] has it, in the file a shipped skin keeps it
    /// in, the skin's groups then `options`; or say why it would not read.
    pub(crate) fn add_section(&mut self, key: &str, options: &[SkinOption]) -> Result<(), String> {
        let file = self.file_for(key);
        let node = self
            .template(key)
            .ok_or("the skin file does not read, so nothing can be added")?;
        let result = self.try_change(|files| {
            files[file]
                .get_or_insert_with(|| Node::Object(Vec::new()))
                .insert(key, node);
            write_options(files, options);
            true
        });
        if result.is_ok() {
            self.open.insert((file, vec![Step::Key(key.to_owned())]));
        }
        result
    }

    /// Take section `key` out of every file, the skin's groups then `options`; or say why
    /// it would not read without it.
    pub(crate) fn remove_section(
        &mut self,
        key: &str,
        options: &[SkinOption],
    ) -> Result<(), String> {
        self.try_change(|files| {
            let mut removed = false;
            for node in files.iter_mut().flatten() {
                removed |= node.remove(key).is_some();
            }
            write_options(files, options);
            removed
        })
    }

    /// Change the files with `edit` (which says whether it changed anything) when they
    /// still read after it, keeping what they were to undo; else leave them and say why.
    fn try_change(&mut self, edit: impl FnOnce(&mut Files) -> bool) -> Result<(), String> {
        let before = self.files.clone();
        if !edit(&mut self.files) || self.files == before {
            self.files = before;
            return Ok(());
        }
        if let Err(why) = self.check() {
            self.files = before;
            return Err(why);
        }
        self.undo.push(before);
        if self.undo.len() > UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.changed();
        Ok(())
    }

    /// The file section `key` goes in: the one a shipped skin keeps it in, else the
    /// newest additions file (a client older than the section never opens it).
    fn file_for(&self, key: &str) -> usize {
        self.templates
            .iter()
            .find(|template| template.key == key)
            .map_or(FILE_COUNT - 1, |template| template.file)
    }

    /// Take out section or list item `row`.
    pub(crate) fn remove(&mut self, row: &Row) {
        let Some((last, parent)) = row.path.split_last() else {
            return;
        };
        let last = last.clone();
        self.change(true, |files| {
            let Some(root) = files[row.file].as_mut() else {
                return false;
            };
            match (root.at_mut(parent), last) {
                (Some(node), Step::Key(key)) => node.remove(&key).is_some(),
                (Some(Node::List(items)), Step::Index(index)) if index < items.len() => {
                    items.remove(index);
                    true
                }
                _ => false,
            }
        });
    }

    /// Copy list item `row` after itself.
    pub(crate) fn duplicate(&mut self, row: &Row) {
        let Some((Step::Index(index), parent)) = row.path.split_last() else {
            return;
        };
        let index = *index;
        self.change(true, |files| {
            let Some(Node::List(items)) = files[row.file]
                .as_mut()
                .and_then(|root| root.at_mut(parent))
            else {
                return false;
            };
            let Some(item) = items.get(index).cloned() else {
                return false;
            };
            items.insert(index + 1, item);
            true
        });
    }

    /// Change the files with `edit` (which says whether it changed anything), keeping
    /// what they were to undo when `record`, then put the skin in place.
    fn change(&mut self, record: bool, edit: impl FnOnce(&mut Files) -> bool) {
        let before = self.files.clone();
        if !edit(&mut self.files) || self.files == before {
            self.files = before;
            return;
        }
        if record {
            self.undo.push(before);
            if self.undo.len() > UNDO {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.changed();
    }

    /// Set the number at `path` of the skin file (`corona.rim_cool.0`: a member, or a list's
    /// item by its number), as a row of it would, for a world shot.
    #[cfg(test)]
    pub(crate) fn set_for_shot(&mut self, path: &str, value: f64) {
        let path: Vec<Step> = path
            .split('.')
            .map(|part| match part.parse() {
                Ok(index) => Step::Index(index),
                Err(_) => Step::Key(part.to_owned()),
            })
            .collect();
        let row = Row {
            depth: 0,
            label: String::new(),
            file: 0,
            path,
            value: Value::Number {
                value,
                range: None,
                whole: false,
            },
        };
        self.set_number(&row, value, true);
    }

    /// Mark the start of a change made of many (a drag), to undo as one.
    pub(crate) fn begin(&mut self) {
        self.undo.push(self.files.clone());
        if self.undo.len() > UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Undo the last change.
    pub(crate) fn undo(&mut self) {
        if let Some(files) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.files, files));
            self.changed();
        }
    }

    /// Redo the last change undone.
    pub(crate) fn redo(&mut self) {
        if let Some(files) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.files, files));
            self.changed();
        }
    }

    /// The files changed: read them, put the skin in place when they read, and write
    /// them soon.
    fn changed(&mut self) {
        self.problem = match self.check() {
            Ok(def) => (self.publish)(self.id, def).err(),
            Err(why) => Some(why),
        };
        self.changed = Some(Instant::now());
    }

    /// The skin as the game would read the files, or why it would not.
    fn check(&self) -> Result<BladeSkinDef, String> {
        let merged = merged(&self.files).ok_or("the skin file is missing")?;
        blade_skin_file::parse(self.id, &merged.compact())
    }

    /// Each frame: write the files a moment after the last change; read them again when
    /// they changed on disk (another editor saved them) and nothing waits here.
    pub(crate) fn follow(&mut self, now: Instant) {
        if self
            .changed
            .is_some_and(|changed| now.saturating_duration_since(changed) >= SAVE_AFTER)
        {
            self.save();
            return;
        }
        let stamp = crate::saber_lab::read_stamp();
        if self.changed.is_none() && stamp != self.stamp {
            self.stamp = stamp;
            if let Ok(files) = read_files(&self.folder, self.id)
                && files != self.files
            {
                self.files = files.clone();
                self.written = files;
                self.undo.clear();
                self.redo.clear();
                self.problem = self.check().err();
                self.find_original();
            }
        }
    }

    /// The skin's original as [`ORIGINALS`] keeps it, when it does (another editor may have
    /// kept it); until one is kept, the files as they are.
    fn find_original(&mut self) {
        match self
            .originals
            .as_ref()
            .map(|kept| read_files(kept, self.id))
        {
            Some(Ok(original)) => {
                self.original = original;
                self.kept = true;
            }
            _ if !self.kept => self.original = self.files.clone(),
            _ => {}
        }
    }

    /// Before the files are first written over: copy them as they are on disk into
    /// [`ORIGINALS`], unless it holds the skin's already (an editor before kept it). The
    /// skin file goes last, so that a copy cut short is made again.
    fn keep_original(&mut self) -> Result<(), String> {
        let Some(kept) = self.originals.clone() else {
            return Ok(());
        };
        if self.kept || self.files == self.written {
            return Ok(());
        }
        let base = format!("{}{}", self.id, FILES[0]);
        if !kept.join(&base).exists() {
            std::fs::create_dir_all(&kept)
                .map_err(|error| format!("{}: {error}", kept.display()))?;
            for extension in FILES[1..].iter().chain([&FILES[0]]) {
                let name = format!("{}{extension}", self.id);
                let path = self.folder.join(&name);
                if path.exists() {
                    std::fs::copy(&path, kept.join(&name))
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                }
            }
        }
        self.original = read_files(&kept, self.id)?;
        self.kept = true;
        Ok(())
    }

    /// Write the files that changed now: the skin file and each additions file holding
    /// something; an additions file left empty is taken away. The first time, the files
    /// as they were are kept first ([`Self::keep_original`]); nothing is written when
    /// they cannot be.
    pub(crate) fn save(&mut self) {
        self.changed = None;
        if let Err(why) = self.keep_original() {
            self.problem = Some(format!(
                "could not keep a copy of its original files ({why}), so nothing is written"
            ));
            return;
        }
        let mut failed = None;
        for (file, extension) in FILES.iter().enumerate() {
            if self.files[file] == self.written[file] {
                continue;
            }
            let name = format!("{}{extension}", self.id);
            let path = self.folder.join(&name);
            // A file back as it was is the kept one itself, written its own way.
            let original = self
                .originals
                .as_ref()
                .map(|kept| kept.join(&name))
                .filter(|kept| {
                    self.kept && self.files[file] == self.original[file] && kept.exists()
                });
            let result = match (&self.files[file], original) {
                (Some(_), Some(original)) => std::fs::copy(original, &path).map(drop),
                (Some(node), None) if file == 0 || !node.members().is_empty() => {
                    std::fs::write(&path, node.pretty())
                }
                _ if path.exists() => std::fs::remove_file(&path),
                _ => Ok(()),
            };
            match result {
                Ok(()) => self.written[file] = self.files[file].clone(),
                Err(error) => failed = Some(format!("{}: {error}", path.display())),
            }
        }
        crate::saber_lab::wrote();
        self.stamp = crate::saber_lab::read_stamp();
        if let Some(why) = failed {
            self.problem = Some(format!("could not write {why}"));
        }
    }
}

/// What each section draws, in a few words (`docs/unlockables.md`, "Blade-skin files"):
/// the editor's rows say it beside the section's name. Only words: the sections
/// themselves are the format's ([`crate::blade_skin_shape`]); a test says when the format
/// has one this does not word.
const SECTIONS: [(&str, &str); 55] = [
    ("glow_profile", "the glow's shape"),
    ("core_profile", "the core's shape"),
    ("glow_image", "a glow image"),
    ("core_image", "a core image"),
    ("core", "the hot core"),
    ("corona", "the glow round it"),
    ("granulation", "noise along it"),
    ("flares", "running flares"),
    ("shimmer", "the glow breathing"),
    ("tongues", "flame tongues"),
    ("arcs", "lightning arcs"),
    ("motes", "drifting motes"),
    ("hue", "a turning hue"),
    ("sputter", "a sputtering edge"),
    ("glitch", "glitching"),
    ("scan", "hologram lines"),
    ("pulse", "a heartbeat pulse"),
    ("ghosts", "afterimages"),
    ("embers", "falling embers"),
    ("veins", "glowing veins"),
    ("team", "the team's tint"),
    ("ambient", "the place's tint"),
    ("glyphs", "scrolling glyphs"),
    ("star", "a glint at the tip"),
    ("wisps", "rising wisps"),
    ("haze", "heat haze"),
    ("echo", "a swaying echo"),
    ("options", "switchable parts"),
    ("orbit", "glints circling"),
    ("holes", "black holes"),
    ("bands", "bands and rings"),
    ("segments", "stuttering segments"),
    ("film", "a soap film"),
    ("blobs", "rising wax blobs"),
    ("front", "a climbing colour"),
    ("sketch", "a pencil sketch"),
    ("hourglass", "an hourglass"),
    ("serpent", "a coiled serpent"),
    ("strands", "glass strands"),
    ("butterflies", "butterflies"),
    ("wings", "wings of fire"),
    ("copies", "ghost copies"),
    ("ink", "an ink outline"),
    ("chrome", "liquid chrome"),
    ("trail", "the trail's colour"),
    ("light", "the blade's light"),
    ("sounds", "its sounds"),
    ("smoke", "smoke from the emitter"),
    ("ribbon", "smoke as ribbons"),
    ("sparks", "sparks from the emitter"),
    ("bolts", "bolts from the emitter"),
    ("unrest", "wild at the emitter, calm up the blade"),
    ("flash", "a crackling star at the emitter"),
    ("crackle", "a buzz and snaps at the emitter"),
    ("chroma", "takes its wearer's saber colour"),
];

/// What section `key` draws, in a few words.
pub(crate) fn about(key: &str) -> Option<&'static str> {
    SECTIONS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, about)| *about)
}

/// Copy skin `id`'s files from `packs` into the lab's folder, to edit them there: the lab
/// draws its copy in place of the pack's, on this PC only. Nothing is written over files
/// the folder already holds.
pub(crate) fn copy_into_lab(id: &str) -> Result<(), String> {
    let folder = crate::saber_lab::folder()
        .ok_or("the Saber Lab is off")?
        .join(blade_skin_file::FOLDER);
    let base = folder.join(format!("{id}{}", blade_skin_file::EXTENSION));
    if base.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let skins = crate::sjk_packs::skins();
    let mut copied = 0;
    for extension in FILES {
        let path = format!("{}/{id}{extension}", blade_skin_file::FOLDER);
        if let Ok(Some(asset)) = skins.packs().read(&path) {
            std::fs::write(folder.join(format!("{id}{extension}")), asset.bytes)
                .map_err(|error| error.to_string())?;
            copied += 1;
        }
    }
    if copied == 0 {
        return Err(format!("no pack holds {id}'s files"));
    }
    crate::saber_lab::reload();
    Ok(())
}

/// Whether the lab's folder holds skin `id`'s file, to edit.
pub(crate) fn in_lab(id: &str) -> bool {
    crate::saber_lab::folder().is_some_and(|folder| {
        folder
            .join(blade_skin_file::FOLDER)
            .join(format!("{id}{}", blade_skin_file::EXTENSION))
            .exists()
    })
}

/// Skin `id`'s files in `folder`, read; the skin file must be there and read.
fn read_files(folder: &Path, id: &str) -> Result<Files, String> {
    let mut files: Files = Default::default();
    for (file, extension) in FILES.iter().enumerate() {
        let path = folder.join(format!("{id}{extension}"));
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                files[file] =
                    Some(Node::parse(&text).map_err(|why| format!("{id}{extension}: {why}"))?);
            }
            Err(_) if file > 0 => {}
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Ok(files)
}

/// The files laid over each other, as the game reads them: each additions file's members
/// in place of the skin file's.
fn merged(files: &Files) -> Option<Node> {
    let mut base = files[0].clone()?;
    for node in files[1..].iter().flatten() {
        for (key, value) in node.members() {
            base.insert(key, value.clone());
        }
    }
    Some(base)
}

/// Note the ranges skin `id`'s checks give its fields.
fn learn(ranges: &mut HashMap<String, (f32, f32)>, id: &str, merged: &Node) {
    for (field, low, high) in blade_skin_file::ranges(id, &merged.compact()) {
        ranges.entry(field).or_insert((low, high));
    }
}

/// The name a field's check gives it: `motes.drift.along`, `granulation.octaves[0].scale`.
fn range_name(path: &[Step]) -> String {
    let mut name = String::new();
    for step in path {
        match step {
            Step::Key(key) => {
                if !name.is_empty() {
                    name.push('.');
                }
                name.push_str(key);
            }
            Step::Index(index) => name.push_str(&format!("[{index}]")),
        }
    }
    name
}

/// A list of three numbers: a colour.
fn colour(items: &[Node]) -> Option<[f32; 3]> {
    match items {
        [r, g, b] => Some([r.number()?, g.number()?, b.number()?].map(|channel| channel as f32)),
        _ => None,
    }
}

/// Set what `row` shows to `node`; whether there was something there.
fn set(files: &mut Files, row: &Row, node: Node) -> bool {
    match files[row.file]
        .as_mut()
        .and_then(|root| root.at_mut(&row.path))
    {
        Some(slot) => {
            *slot = node;
            true
        }
        None => false,
    }
}

/// Put `options` in the files: in the newest file a client must read to know every
/// section they name (its set's, or the file holding it here, whichever is later; at
/// least the first additions file), and in no other, so an older client reads none it
/// could not check. None at all when `options` is empty.
fn write_options(files: &mut Files, options: &[SkinOption]) {
    for node in files.iter_mut().flatten() {
        node.remove("options");
    }
    if options.is_empty() {
        return;
    }
    let holding = |key: &str| {
        files
            .iter()
            .rposition(|node| node.as_ref().is_some_and(|node| node.get(key).is_some()))
            .unwrap_or(0)
    };
    let file = options
        .iter()
        .flat_map(|option| &option.sections)
        .map(|key| crate::blade_builder::set_file(key).max(holding(key)))
        .fold(crate::blade_builder::set_file("options"), usize::max)
        .min(FILE_COUNT - 1);
    let node = Node::List(
        options
            .iter()
            .map(|option| {
                Node::Object(vec![
                    ("id".to_owned(), Node::Text(option.id.clone())),
                    ("name".to_owned(), Node::Text(option.name.clone())),
                    (
                        "sections".to_owned(),
                        Node::List(
                            option
                                .sections
                                .iter()
                                .map(|key| Node::Text(key.clone()))
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect(),
    );
    files[file]
        .get_or_insert_with(|| Node::Object(Vec::new()))
        .insert("options", node);
}

/// The groups an `options` list gives, each as written (one that is not an object of an
/// id, a name and sections is left out: the check says why).
fn options_of(node: &Node) -> Vec<SkinOption> {
    let Node::List(items) = node else {
        return Vec::new();
    };
    let text = |node: Option<&Node>| match node {
        Some(Node::Text(text)) => Some(text.clone()),
        _ => None,
    };
    items
        .iter()
        .filter_map(|item| {
            let sections = match item.get("sections") {
                Some(Node::List(names)) => {
                    names.iter().filter_map(|name| text(Some(name))).collect()
                }
                _ => return None,
            };
            Some(SkinOption {
                id: text(item.get("id"))?,
                name: text(item.get("name"))?,
                sections,
            })
        })
        .collect()
}

/// `value` rounded to 1, 2 or 5 times a power of ten.
fn nice(value: f64) -> f64 {
    if !(value.is_finite() && value > 0.0) {
        return 0.01;
    }
    let power = 10f64.powf(value.log10().floor());
    let mantissa = value / power;
    let rounded = if mantissa < 1.5 {
        1.0
    } else if mantissa < 3.5 {
        2.0
    } else if mantissa < 7.5 {
        5.0
    } else {
        10.0
    };
    rounded * power
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blade_skin_file::tests::{ADDITIONS, SAMPLE};

    fn published(_: &str, _: BladeSkinDef) -> Result<(), String> {
        Ok(())
    }

    /// A folder holding the test sample and its additions as skin `saber_sun`, and packs
    /// holding the sample with its later sections as another.
    fn editor() -> (tempfile::TempDir, Editor) {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("saber_sun.bladeskin"), SAMPLE).unwrap();
        let mut packs = sjk_vfs::VirtualFileSystem::new();
        packs
            .mount_memory(
                "packs",
                [
                    (
                        "skins/blades/saber_storm.bladeskin",
                        SAMPLE.as_bytes().to_vec(),
                    ),
                    (
                        "skins/blades/saber_storm.bladeextra",
                        ADDITIONS.as_bytes().to_vec(),
                    ),
                ],
            )
            .unwrap();
        let editor = Editor::read("saber_sun", folder.path(), &packs, published).unwrap();
        (folder, editor)
    }

    fn row<'a>(rows: &'a [Row], label: &str) -> &'a Row {
        rows.iter()
            .find(|row| row.label == label)
            .unwrap_or_else(|| panic!("no row {label}"))
    }

    /// The sections show closed, their fields once open, a number with its check's
    /// range; a section of the shipped skin's additions is there to add.
    #[test]
    fn the_rows_are_the_files_sections_and_fields_with_their_ranges() {
        let (_folder, mut editor) = editor();
        assert!(editor.problem().is_none(), "{:?}", editor.problem());
        let rows = editor.rows();
        assert!(rows.iter().all(|row| row.label != "version"));
        let motes = row(&rows, "core");
        assert!(matches!(motes.value, Value::Group { open: false, .. }));
        editor.toggle_open(&row(&rows, "core").clone());
        let rows = editor.rows();
        let flare = row(&rows, "white_flare");
        assert_eq!(flare.depth, 1);
        assert!(matches!(
            flare.value,
            Value::Number {
                range: Some((0.0, 10.0)),
                whole: false,
                ..
            }
        ));
        let white = row(&rows, "white");
        assert!(matches!(white.value, Value::Colour { open: false, .. }));
        let star = row(&rows, "star");
        assert_eq!((star.value.clone(), star.file), (Value::Add, 1));
    }

    /// A nudge moves a number by its step within its range; a change that does not read
    /// says why; undo and redo walk the changes; the files are written in their order,
    /// an added section in its additions file.
    #[test]
    fn changes_check_undo_and_write_the_files() {
        let (folder, mut editor) = editor();
        editor.toggle_open(&row(&editor.rows(), "core").clone());
        let flare = row(&editor.rows(), "white_flare").clone();
        editor.nudge(&flare, 3.0, true);
        let moved = row(&editor.rows(), "white_flare").clone();
        assert!(matches!(moved.value, Value::Number { value, .. } if (value - 0.8).abs() < 1e-9));
        editor.set_number(&moved, 99.0, true);
        assert!(matches!(
            row(&editor.rows(), "white_flare").value,
            Value::Number { value, .. } if value == 10.0
        ));
        editor.undo();
        editor.undo();
        assert!(matches!(
            row(&editor.rows(), "white_flare").value,
            Value::Number { value, .. } if (value - 0.5).abs() < 1e-9
        ));
        editor.redo();
        assert_eq!(editor.can_undo(), (true, true));
        let star = row(&editor.rows(), "star").clone();
        editor.add(&star);
        assert!(editor.problem().is_none(), "{:?}", editor.problem());
        editor.save();
        let written = std::fs::read_to_string(folder.path().join("saber_sun.bladeskin")).unwrap();
        let keys: Vec<String> = Node::parse(&written)
            .unwrap()
            .members()
            .iter()
            .map(|(key, _)| key.clone())
            .collect();
        let sample: Vec<String> = Node::parse(SAMPLE)
            .unwrap()
            .members()
            .iter()
            .map(|(key, _)| key.clone())
            .collect();
        assert_eq!(keys, sample, "the skin file keeps its order");
        assert!(written.contains("\"white_flare\": 0.8"));
        let extra = std::fs::read_to_string(folder.path().join("saber_sun.bladeextra")).unwrap();
        assert!(Node::parse(&extra).unwrap().get("star").is_some());
        // Taken out again, the additions file goes with it.
        editor.remove(&row(&editor.rows(), "star").clone());
        editor.save();
        assert!(!folder.path().join("saber_sun.bladeextra").exists());
    }

    /// A section no shipped skin holds is offered, and added as the format makes it, in
    /// the newest additions file; every section the format has is worded.
    #[test]
    fn a_section_no_skin_holds_is_made_from_the_format() {
        let (_folder, mut editor) = editor();
        let rows = editor.rows();
        let arcs = row(&rows, "arcs").clone();
        assert_eq!(
            (arcs.value.clone(), arcs.file),
            (Value::Add, FILES.len() - 1)
        );
        editor.add(&arcs);
        assert!(editor.problem().is_none(), "{:?}", editor.problem());
        let rows = editor.rows();
        assert!(matches!(row(&rows, "span").value, Value::Group { .. }));
        for key in &editor.sections {
            assert!(about(key).is_some(), "{key} has no words in SECTIONS");
        }
    }

    /// A value out of what the checks allow, set by a path the ranges do not bound, is
    /// refused with the check's words and the last good look kept.
    #[test]
    fn a_change_that_does_not_read_says_why() {
        let (_folder, mut editor) = editor();
        let row = Row {
            depth: 0,
            label: "low".to_owned(),
            file: 0,
            path: vec![
                Step::Key("granulation".to_owned()),
                Step::Key("low".to_owned()),
            ],
            value: Value::Number {
                value: 0.0,
                range: None,
                whole: false,
            },
        };
        editor.set_number(&row, 5.0, true);
        assert!(
            editor
                .problem()
                .is_some_and(|why| why.contains("granulation.low"))
        );
        editor.undo();
        assert!(editor.problem().is_none());
    }

    #[test]
    fn steps_are_round() {
        assert_eq!(nice(0.1), 0.1);
        assert_eq!(nice(0.03), 0.02);
        assert_eq!(nice(0.06), 0.05);
        assert_eq!(nice(0.09), 0.1);
        assert_eq!(nice(20.0), 20.0);
    }
}

#[cfg(test)]
mod pack_probe {
    use super::*;

    /// Every section the format has is offered to add from the shipped skins (the black
    /// holes among them), under its heading, and said in a few words.
    #[test]
    #[ignore = "reads the packs SJK_TEST_PACKS names"]
    fn every_section_is_offered_from_the_packs() {
        let directory = std::env::var("SJK_TEST_PACKS").unwrap();
        let mut packs = sjk_vfs::VirtualFileSystem::new();
        for entry in std::fs::read_dir(directory).unwrap().flatten() {
            packs.mount_pk3(entry.path()).unwrap();
        }
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(
            folder.path().join("saber_sun.bladeskin"),
            crate::blade_skin_file::tests::SAMPLE,
        )
        .unwrap();
        fn published(_: &str, _: BladeSkinDef) -> Result<(), String> {
            Ok(())
        }
        let editor = Editor::read("saber_sun", folder.path(), &packs, published).unwrap();
        let adds: Vec<_> = editor
            .rows()
            .into_iter()
            .filter(|row| row.value == Value::Add)
            .map(|row| format!("{}@{}", row.label, row.file))
            .collect();
        println!("{adds:?}, {} ranges", editor.ranges.len());
        for (key, _) in SECTIONS {
            let skin_has = Node::parse(crate::blade_skin_file::tests::SAMPLE)
                .unwrap()
                .get(key)
                .is_some();
            let offered = adds.iter().any(|add| add.starts_with(&format!("{key}@")));
            // The sample holds the required sections and a few others; the images and
            // the sounds are not sections a shipped skin adds.
            assert!(
                skin_has || offered || matches!(key, "glow_image" | "core_image"),
                "{key} is not offered"
            );
        }
        let rows = editor.rows();
        let heading = Editor::adds_row(&rows).unwrap();
        assert!(
            rows[heading + 1..]
                .iter()
                .all(|row| row.value == Value::Add)
        );
        assert!(rows.iter().any(|row| row.label == "holes"));
    }
}
