//! The saved form of one map's edits: a JSON document per map and BSP checksum, kept in
//! `GameData/SJK/map_edits/` and on the hub (`/v1/map-edits`), which stores the `edits`
//! list as it is given.
//!
//! An edit applies to every server, or only to the one whose address it names
//! ([`Scope`]). Entity and surface numbers are the BSP's own, so a document applies only
//! to the map file whose checksum it carries.

use serde::{Deserialize, Serialize};

/// Most edits one map keeps, as the hub allows.
pub(crate) const MAX_EDITS: usize = 2048;
/// Longest shader or model path an edit names.
const MAX_PATH: usize = 160;

/// One map's edits.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct Document {
    /// The map, as `map_name` gives it (`mp/ffa3`), in lower case.
    pub(crate) map: String,
    /// The BSP's checksum (`CM_LoadMap`'s), the edits apply to that file only.
    pub(crate) checksum: i32,
    /// The hub's revision this copy is from; 0 when it never was on the hub.
    #[serde(default)]
    pub(crate) revision: u64,
    /// Changed on this PC since the hub's revision (staff's work not yet published).
    #[serde(default)]
    pub(crate) unpublished: bool,
    #[serde(default)]
    pub(crate) edits: Vec<Edit>,
}

/// One edit.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Edit {
    /// Unique within its document, kept when the document is saved again.
    pub(crate) id: u32,
    /// The server address (`ip:port`) the edit is for; every server when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) server: Option<String>,
    #[serde(flatten)]
    pub(crate) kind: Kind,
}

/// Where an edit applies.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Scope {
    #[default]
    AllServers,
    ThisServer,
}

/// What an edit does.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Kind {
    /// Do not draw the map's `misc_model_static` prop `entity` (its number in the entity
    /// lump, worldspawn 0); `model` is kept to check and to name it.
    HideProp { entity: u32, model: String },
    /// Do not draw BSP surface `surface`; `shader` is kept to check and to name it.
    HideSurface { surface: u32, shader: String },
    /// Draw `model` (an MD3 in the game data) at `origin`, turned by `angles` (pitch,
    /// yaw, roll in degrees) and scaled by `scale` on each axis.
    Place {
        model: String,
        origin: [f32; 3],
        #[serde(default)]
        angles: [f32; 3],
        #[serde(default = "unit_scale")]
        scale: [f32; 3],
    },
    /// Draw shader `to` wherever the map draws `from`.
    Remap { from: String, to: String },
    /// Draw side `side` of world brush `brush` (BSP numbers), a face the map draws
    /// nothing on (caulk, nodraw), with shader `to`; `shader` is the side's own, kept to
    /// check and to name it.
    ShowFace {
        brush: u32,
        side: u32,
        shader: String,
        to: String,
    },
}

fn unit_scale() -> [f32; 3] {
    [1.0; 3]
}

impl Edit {
    /// Whether the edit applies on `server` (`None` off a server, for every-server edits
    /// only).
    pub(crate) fn applies_on(&self, server: Option<&str>) -> bool {
        match (&self.server, server) {
            (None, _) => true,
            (Some(wanted), Some(server)) => wanted.eq_ignore_ascii_case(server),
            (Some(_), None) => false,
        }
    }

    /// A short line naming what the edit does, for the panel's list and its answers:
    /// `Hidden prop: tree01.md3`, `Placed: crate.md3`, `Remap: floor -> floor2`, with its
    /// server when it is for one.
    pub(crate) fn label(&self) -> String {
        let last = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
        let what = match &self.kind {
            Kind::HideProp { model, .. } => format!("Hidden prop: {}", last(model)),
            Kind::HideSurface { surface, shader } => {
                format!("Hidden surface: {} ({surface})", last(shader))
            }
            Kind::Place { model, scale, .. } => {
                if scale.iter().all(|axis| (axis - 1.0).abs() < 1e-3) {
                    format!("Placed: {}", last(model))
                } else {
                    format!("Placed: {} x{:.2}", last(model), scale[0])
                }
            }
            Kind::Remap { from, to } => format!("Remap: {} -> {}", last(from), last(to)),
            Kind::ShowFace { shader, to, .. } => {
                format!("Shown face: {} -> {}", last(shader), last(to))
            }
        };
        match &self.server {
            Some(server) => format!("{what}, {server} only"),
            None => what,
        }
    }

    /// One line naming the edit, for `mapedit_list`.
    pub(crate) fn describe(&self) -> String {
        let what = match &self.kind {
            Kind::HideProp { entity, model } => format!("hide prop {entity} ({model})"),
            Kind::HideSurface { surface, shader } => {
                format!("hide surface {surface} ({shader})")
            }
            Kind::Place {
                model,
                origin,
                angles,
                scale,
            } => format!(
                "place {model} at {:.0} {:.0} {:.0}, yaw {:.0}{}",
                origin[0],
                origin[1],
                origin[2],
                angles[1],
                if scale.iter().all(|axis| (axis - 1.0).abs() < 1e-3) {
                    String::new()
                } else {
                    format!(", scale {:.2}", scale[0])
                }
            ),
            Kind::Remap { from, to } => format!("remap {from} -> {to}"),
            Kind::ShowFace {
                brush,
                side,
                shader,
                to,
            } => format!("show brush {brush} side {side} ({shader}) as {to}"),
        };
        let scope = self
            .server
            .as_ref()
            .map_or_else(|| "all servers".to_owned(), |server| server.clone());
        format!("{}: {what} [{scope}]", self.id)
    }
}

impl Document {
    pub(crate) fn new(map: &str, checksum: i32) -> Self {
        Self {
            map: map.to_ascii_lowercase(),
            checksum,
            ..Self::default()
        }
    }

    /// Read a document, refusing one that is not for `map` and `checksum` or that breaks
    /// the limits.
    pub(crate) fn parse(text: &str, map: &str, checksum: i32) -> Result<Self, String> {
        let document: Self = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if !document.map.eq_ignore_ascii_case(map) || document.checksum != checksum {
            return Err(format!(
                "edits for {} {}, not {map} {checksum}",
                document.map, document.checksum
            ));
        }
        document.check()?;
        Ok(document)
    }

    /// The limits every document keeps.
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.edits.len() > MAX_EDITS {
            return Err(format!("more than {MAX_EDITS} edits"));
        }
        for edit in &self.edits {
            let paths: &[&str] = match &edit.kind {
                Kind::HideProp { model, .. } => &[model],
                Kind::HideSurface { shader, .. } => &[shader],
                Kind::Place {
                    model,
                    origin,
                    angles,
                    scale,
                } => {
                    if origin
                        .iter()
                        .chain(angles)
                        .chain(scale)
                        .any(|value| !value.is_finite())
                        || scale.iter().any(|axis| !(0.01..=64.0).contains(axis))
                    {
                        return Err(format!("edit {}: bad placement", edit.id));
                    }
                    &[model]
                }
                Kind::Remap { from, to } => &[from, to],
                Kind::ShowFace { shader, to, .. } => &[shader, to],
            };
            if paths
                .iter()
                .any(|path| path.is_empty() || path.len() > MAX_PATH)
            {
                return Err(format!("edit {}: bad name", edit.id));
            }
        }
        Ok(())
    }

    /// Add `kind` for `server` (every server when `None`), returning its id. An edit that
    /// does the same thing for the same servers is replaced instead: hiding a prop twice
    /// keeps one edit, remapping a shader again keeps the newest target.
    pub(crate) fn add(&mut self, kind: Kind, server: Option<String>) -> u32 {
        if let Some(edit) = self
            .edits
            .iter_mut()
            .find(|edit| edit.server == server && same_subject(&edit.kind, &kind))
        {
            edit.kind = kind;
            self.unpublished = true;
            return edit.id;
        }
        let id = self.edits.iter().map(|edit| edit.id).max().unwrap_or(0) + 1;
        self.edits.push(Edit { id, server, kind });
        self.unpublished = true;
        id
    }

    /// Drop edit `id`, returning it.
    pub(crate) fn remove(&mut self, id: u32) -> Option<Edit> {
        let index = self.edits.iter().position(|edit| edit.id == id)?;
        self.unpublished = true;
        Some(self.edits.remove(index))
    }

    pub(crate) fn get_mut(&mut self, id: u32) -> Option<&mut Edit> {
        self.edits.iter_mut().find(|edit| edit.id == id)
    }

    /// The edits that apply on `server`.
    pub(crate) fn active<'a>(&'a self, server: Option<&'a str>) -> impl Iterator<Item = &'a Edit> {
        self.edits
            .iter()
            .filter(move |edit| edit.applies_on(server))
    }
}

/// Whether two edits act on the same thing (so the newer replaces the older).
fn same_subject(a: &Kind, b: &Kind) -> bool {
    match (a, b) {
        (Kind::HideProp { entity: a, .. }, Kind::HideProp { entity: b, .. }) => a == b,
        (Kind::HideSurface { surface: a, .. }, Kind::HideSurface { surface: b, .. }) => a == b,
        (Kind::Remap { from: a, .. }, Kind::Remap { from: b, .. }) => a.eq_ignore_ascii_case(b),
        (
            Kind::ShowFace {
                brush: a, side: c, ..
            },
            Kind::ShowFace {
                brush: b, side: d, ..
            },
        ) => a == b && c == d,
        _ => false,
    }
}

/// The file name of `map`'s document for `checksum`: `mp/ffa3` becomes
/// `mp_ffa3_-1234.json`.
pub(crate) fn file_name(map: &str, checksum: i32) -> String {
    // The identity worker writes the hub's copies under the same names.
    sjk_identity::map_edits::file_name(map, checksum)
}

/// The prefix every document of `map` starts with, whatever its checksum.
pub(crate) fn file_prefix(map: &str) -> String {
    let name = file_name(map, 0);
    name[..name.len() - "0.json".len()].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(x: f32) -> Kind {
        Kind::Place {
            model: "models/map_objects/imperial/crate_big.md3".into(),
            origin: [x, 0.0, 0.0],
            angles: [0.0, 90.0, 0.0],
            scale: [1.0; 3],
        }
    }

    #[test]
    fn edits_round_trip_in_the_hub_shape() {
        let mut document = Document::new("MP/FFA3", -42);
        document.add(
            Kind::HideProp {
                entity: 12,
                model: "models/map_objects/a.md3".into(),
            },
            None,
        );
        document.add(place(64.0), Some("1.2.3.4:29070".into()));
        document.add(
            Kind::Remap {
                from: "textures/a".into(),
                to: "textures/b".into(),
            },
            None,
        );
        let text = serde_json::to_string(&document).unwrap();
        assert!(text.contains(r#""kind":"hide_prop""#), "{text}");
        assert!(text.contains(r#""server":"1.2.3.4:29070""#), "{text}");
        let back = Document::parse(&text, "mp/ffa3", -42).unwrap();
        assert_eq!(back, document);
        assert!(back.unpublished);
        assert!(Document::parse(&text, "mp/ffa3", 7).is_err());
        assert!(Document::parse(&text, "mp/ffa2", -42).is_err());
    }

    #[test]
    fn a_scoped_edit_applies_on_its_server_only() {
        let mut document = Document::new("mp/ffa3", 1);
        let all = document.add(place(0.0), None);
        let one = document.add(place(1.0), Some("1.2.3.4:29070".into()));
        let ids = |server| {
            document
                .active(server)
                .map(|edit| edit.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Some("1.2.3.4:29070")), [all, one]);
        assert_eq!(ids(Some("5.6.7.8:29070")), [all]);
        assert_eq!(ids(None), [all]);
    }

    #[test]
    fn the_same_subject_is_edited_once_per_scope() {
        let mut document = Document::new("mp/ffa3", 1);
        let remap = |to: &str| Kind::Remap {
            from: "textures/a".into(),
            to: to.into(),
        };
        let first = document.add(remap("textures/b"), None);
        assert_eq!(document.add(remap("textures/c"), None), first);
        let scoped = document.add(remap("textures/d"), Some("1.2.3.4:1".into()));
        assert_ne!(scoped, first);
        assert_eq!(document.edits.len(), 2);
        assert_eq!(
            document.edits[0].kind,
            remap("textures/c"),
            "the newer target replaces the older"
        );
        // Placements are never merged.
        document.add(place(0.0), None);
        document.add(place(0.0), None);
        assert_eq!(document.edits.len(), 4);
        assert!(document.remove(first).is_some());
        assert!(document.remove(first).is_none());
    }

    #[test]
    fn labels_name_edits_shortly() {
        let mut document = Document::new("mp/ffa3", 1);
        let hide = document.add(
            Kind::HideProp {
                entity: 12,
                model: "models/map_objects/yavin/tree01.md3".into(),
            },
            None,
        );
        let placed = document.add(place(0.0), Some("1.2.3.4:29070".into()));
        let remap = document.add(
            Kind::Remap {
                from: "textures/a/floor".into(),
                to: "textures/b/floor2".into(),
            },
            None,
        );
        let label = |id: u32| {
            document
                .edits
                .iter()
                .find(|edit| edit.id == id)
                .unwrap()
                .label()
        };
        assert_eq!(label(hide), "Hidden prop: tree01.md3");
        assert_eq!(label(placed), "Placed: crate_big.md3, 1.2.3.4:29070 only");
        assert_eq!(label(remap), "Remap: floor -> floor2");
    }

    #[test]
    fn a_shown_face_is_one_edit_per_brush_side() {
        let mut document = Document::new("mp/ffa3", 1);
        let face = |side: u32, to: &str| Kind::ShowFace {
            brush: 40,
            side,
            shader: "textures/system/caulk".into(),
            to: to.into(),
        };
        let first = document.add(face(7, "textures/a/wall"), None);
        assert_eq!(document.add(face(7, "textures/a/wall2"), None), first);
        assert_ne!(document.add(face(8, "textures/a/wall"), None), first);
        assert_eq!(document.edits[0].kind, face(7, "textures/a/wall2"));
        assert_eq!(document.edits[0].label(), "Shown face: caulk -> wall2");
        let text = serde_json::to_string(&document).unwrap();
        assert!(text.contains(r#""kind":"show_face""#), "{text}");
        assert_eq!(Document::parse(&text, "mp/ffa3", 1).unwrap(), document);
    }

    /// A sky side covered with a texture is a shown face, and a sky swap a remap: kinds
    /// every client already reads, so a document with them parses everywhere (a client
    /// before sky faces checks the side against its own rule and skips that face only).
    #[test]
    fn sky_edits_are_shown_faces_and_remaps() {
        let text = r#"{"map":"mp/ffa3","checksum":1,"edits":[
            {"id":1,"kind":"show_face","brush":9,"side":54,"shader":"textures/skies/yavin","to":"textures/yavin/wall"},
            {"id":2,"kind":"remap","from":"textures/skies/yavin","to":"textures/skies/night"}]}"#;
        let document = Document::parse(text, "mp/ffa3", 1).unwrap();
        assert_eq!(document.edits[0].label(), "Shown face: yavin -> wall");
        assert_eq!(document.edits[1].label(), "Remap: yavin -> night");
        let back = serde_json::to_string(&document).unwrap();
        assert_eq!(Document::parse(&back, "mp/ffa3", 1).unwrap(), document);
    }

    #[test]
    fn broken_documents_are_refused() {
        let mut document = Document::new("mp/ffa3", 1);
        document.add(
            Kind::Place {
                model: "m.md3".into(),
                origin: [f32::NAN, 0.0, 0.0],
                angles: [0.0; 3],
                scale: [1.0; 3],
            },
            None,
        );
        assert!(document.check().is_err());
        let mut document = Document::new("mp/ffa3", 1);
        document.add(
            Kind::Remap {
                from: String::new(),
                to: "x".into(),
            },
            None,
        );
        assert!(document.check().is_err());
        assert!(Document::parse("{", "mp/ffa3", 1).is_err());
    }

    #[test]
    fn files_are_named_by_map_and_checksum() {
        assert_eq!(file_name("mp/FFA3", -1234), "mp_ffa3_-1234.json");
        assert_eq!(file_prefix("mp/ffa3"), "mp_ffa3_");
        assert!(file_name("mp/ffa3", 99).starts_with(&file_prefix("mp/ffa3")));
    }
}
