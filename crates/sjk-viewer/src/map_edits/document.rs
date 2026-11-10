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
/// Longest shader or model path an edit names, and longest weather command.
const MAX_PATH: usize = 160;
/// Most world effect commands one weather edit runs.
pub(crate) const MAX_WEATHER: usize = 16;
/// The reach an added light may have, in units ([`Kind::Light`]).
pub(crate) const LIGHT_RADIUS: std::ops::RangeInclusive<f32> = 16.0..=2048.0;
/// How strong an added light may be: its colour is multiplied by this.
pub(crate) const LIGHT_INTENSITY: std::ops::RangeInclusive<f32> = 0.05..=8.0;
/// Farthest a map lamp may be moved from where the map has it, along each axis.
pub(crate) const LAMP_OFFSET: f32 = 4096.0;
/// The units a texel of a shown face may cover ([`Align::scale`], either sign: a
/// negative scale flips the texture).
pub(crate) const ALIGN_SCALE: std::ops::RangeInclusive<f32> = 0.01..=64.0;
/// Farthest a shown face's texture may be shifted (texels) or turned (degrees).
pub(crate) const ALIGN_SHIFT: f32 = 65_536.0;
pub(crate) const ALIGN_ROTATE: f32 = 3_600.0;
/// Radiant's texture scale for a new brush: a texel covers half a unit.
pub(crate) const RADIANT_SCALE: f32 = 0.5;

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
    /// check and to name it. `align` lays the texture (its fields sit beside the others,
    /// each left out at Radiant's default).
    ShowFace {
        brush: u32,
        side: u32,
        shader: String,
        to: String,
        #[serde(flatten)]
        align: Align,
    },
    /// The map's weather: world effect commands (`r_we`'s: `heavyrain`, `gustingwind`,
    /// `mist`, ...) run in place of the map's own, as `r_weatherForce` replaces them. One
    /// per scope; on a server, its own weather edit wins over the every-server one.
    Weather { commands: Vec<String> },
    /// A point light staff added: drawn as a dynamic light (the frame's point-light list,
    /// as `r_dynamiclight`'s are) at `origin`, of `color` (0 to 1 each) times
    /// `intensity`, reaching `radius` units. Every preset draws it; the baked lightmaps
    /// are not changed.
    Light {
        origin: [f32; 3],
        color: [f32; 3],
        radius: f32,
        #[serde(default = "unit_intensity")]
        intensity: f32,
    },
    /// One of the map's own real-time lamps (a fixture: every lamp patch of one BSP
    /// surface, of one surface of a prop's model, or one steady effect), turned off or
    /// moved by `offset`. `fixture` names it as [`crate::lamp_lights::Fixture`] writes
    /// it (`surface:1043`, `prop:12:3`, `effect:40`); `name` is its shader, model or
    /// effect, kept to name it. Only the real-time lighting (Experimental) draws lamps,
    /// and they are found when the map's world is built, so the edit applies from then.
    Lamp {
        fixture: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        off: bool,
        #[serde(default)]
        offset: [f32; 3],
    },
}

/// How a shown face's texture lies on it, as Radiant's surface inspector sets a brush
/// face's: shifted (`shift`, texels along the texture's width and height), scaled
/// (`scale`, units a texel covers along each; [`RADIANT_SCALE`] by default, a negative one
/// flips it) and turned (`rotate`, degrees). The projection is Radiant's axial one
/// (`faces::texture_coordinates`). Each field is left out of the JSON at its default, so a
/// face never aligned is written as before; a client from before alignment reads the
/// fields as unknown ones, ignores them and draws the face at Radiant's default.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Align {
    #[serde(default, skip_serializing_if = "is_zero_pair")]
    pub(crate) shift: [f32; 2],
    #[serde(default = "radiant_scale", skip_serializing_if = "is_radiant_scale")]
    pub(crate) scale: [f32; 2],
    #[serde(default, skip_serializing_if = "is_zero")]
    pub(crate) rotate: f32,
}

impl Default for Align {
    fn default() -> Self {
        Self {
            shift: [0.0; 2],
            scale: radiant_scale(),
            rotate: 0.0,
        }
    }
}

impl Align {
    /// Whether it keeps the limits: finite, the scale's size in [`ALIGN_SCALE`], the
    /// shift within [`ALIGN_SHIFT`] and the turn within [`ALIGN_ROTATE`].
    pub(crate) fn valid(&self) -> bool {
        self.shift
            .iter()
            .all(|value| value.is_finite() && value.abs() <= ALIGN_SHIFT)
            && self
                .scale
                .iter()
                .all(|value| value.is_finite() && ALIGN_SCALE.contains(&value.abs()))
            && self.rotate.is_finite()
            && self.rotate.abs() <= ALIGN_ROTATE
    }

    /// Short words for the panel and the answers: `shift 8 0, scale 0.50 0.50, turn 15`.
    pub(crate) fn describe(&self) -> String {
        format!(
            "shift {} {}, scale {:.2} {:.2}, turn {}",
            round(self.shift[0]),
            round(self.shift[1]),
            self.scale[0],
            self.scale[1],
            round(self.rotate)
        )
    }
}

/// A value to a tenth, without a needless `.0`.
fn round(value: f32) -> String {
    let tenths = (value * 10.0).round() / 10.0;
    if tenths.fract() == 0.0 {
        format!("{tenths:.0}")
    } else {
        format!("{tenths:.1}")
    }
}

fn radiant_scale() -> [f32; 2] {
    [RADIANT_SCALE; 2]
}

fn is_radiant_scale(scale: &[f32; 2]) -> bool {
    *scale == radiant_scale()
}

fn is_zero_pair(pair: &[f32; 2]) -> bool {
    *pair == [0.0; 2]
}

fn is_zero(value: &f32) -> bool {
    *value == 0.0
}

fn unit_intensity() -> f32 {
    1.0
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
            Kind::Weather { commands } => format!("Weather: {}", commands.join(", ")),
            Kind::Light {
                color,
                radius,
                intensity,
                ..
            } => format!(
                "Light: #{} r{radius:.0}{}",
                hex(*color),
                if (intensity - 1.0).abs() < 1e-3 {
                    String::new()
                } else {
                    format!(" x{intensity:.2}")
                }
            ),
            Kind::Lamp {
                fixture,
                name,
                off,
                offset,
            } => {
                let what = if name.is_empty() {
                    fixture.clone()
                } else {
                    format!("{} ({fixture})", last(name))
                };
                match (*off, offset.iter().any(|axis| *axis != 0.0)) {
                    (true, _) => format!("Lamp off: {what}"),
                    (false, true) => format!("Lamp moved: {what}"),
                    (false, false) => format!("Lamp: {what}"),
                }
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
                align,
            } => format!(
                "show brush {brush} side {side} ({shader}) as {to}{}",
                if *align == Align::default() {
                    String::new()
                } else {
                    format!(", {}", align.describe())
                }
            ),
            Kind::Weather { commands } => format!("weather {}", commands.join(", ")),
            Kind::Light {
                origin,
                color,
                radius,
                intensity,
            } => format!(
                "light at {:.0} {:.0} {:.0}, colour {:.2} {:.2} {:.2}, radius {radius:.0}, \
                 intensity {intensity:.2}",
                origin[0], origin[1], origin[2], color[0], color[1], color[2]
            ),
            Kind::Lamp {
                fixture,
                name,
                off,
                offset,
            } => format!(
                "lamp {fixture} ({name}){}{}",
                if *off { " off" } else { "" },
                if offset.iter().any(|axis| *axis != 0.0) {
                    format!(", moved {:.0} {:.0} {:.0}", offset[0], offset[1], offset[2])
                } else {
                    String::new()
                }
            ),
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
                Kind::ShowFace {
                    shader, to, align, ..
                } => {
                    if !align.valid() {
                        return Err(format!("edit {}: bad texture alignment", edit.id));
                    }
                    &[shader, to]
                }
                Kind::Weather { commands } => {
                    check_weather(commands)
                        .map_err(|why| format!("edit {}: bad weather: {why}", edit.id))?;
                    &[]
                }
                Kind::Light {
                    origin,
                    color,
                    radius,
                    intensity,
                } => {
                    check_light(*origin, *color, *radius, *intensity)
                        .map_err(|why| format!("edit {}: bad light: {why}", edit.id))?;
                    &[]
                }
                Kind::Lamp {
                    fixture,
                    name,
                    offset,
                    ..
                } => {
                    if crate::lamp_lights::Fixture::parse(fixture).is_none()
                        || name.len() > MAX_PATH
                        || !offset
                            .iter()
                            .all(|axis| axis.is_finite() && axis.abs() <= LAMP_OFFSET)
                    {
                        return Err(format!("edit {}: bad lamp", edit.id));
                    }
                    &[]
                }
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
        let mut kind = kind;
        if let Some(edit) = self
            .edits
            .iter_mut()
            .find(|edit| edit.server == server && same_subject(&edit.kind, &kind))
        {
            // A face drawn with another shader keeps its texture's alignment, as a
            // brush face given another texture does in Radiant.
            if let (Kind::ShowFace { align: old, .. }, Kind::ShowFace { align: new, .. }) =
                (&edit.kind, &mut kind)
                && *new == Align::default()
            {
                *new = *old;
            }
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

    /// The weather commands that apply on `server`: its own weather edit's, else the
    /// every-server one's; none when the map keeps its own weather.
    pub(crate) fn weather<'a>(&'a self, server: Option<&'a str>) -> Option<&'a [String]> {
        let mut found: Option<&Edit> = None;
        for edit in self.active(server) {
            if matches!(edit.kind, Kind::Weather { .. })
                && (found.is_none() || edit.server.is_some())
            {
                found = Some(edit);
            }
        }
        found.map(|edit| match &edit.kind {
            Kind::Weather { commands } => commands.as_slice(),
            _ => &[],
        })
    }
}

/// The limits a weather edit's commands keep: 1 to [`MAX_WEATHER`] of them, each a
/// plausible world effect command ([`weather_command`]).
pub(crate) fn check_weather(commands: &[String]) -> Result<(), String> {
    if commands.is_empty() {
        return Err("no commands".into());
    }
    if commands.len() > MAX_WEATHER {
        return Err(format!("more than {MAX_WEATHER} commands"));
    }
    match commands.iter().find(|command| !weather_command(command)) {
        Some(command) => Err(format!("{command:?} is not a weather command")),
        None => Ok(()),
    }
}

/// The limits an added light keeps: a finite origin, each colour channel 0 to 1 (not all
/// 0), a radius in [`LIGHT_RADIUS`] and an intensity in [`LIGHT_INTENSITY`].
pub(crate) fn check_light(
    origin: [f32; 3],
    color: [f32; 3],
    radius: f32,
    intensity: f32,
) -> Result<(), String> {
    if !origin.iter().all(|axis| axis.is_finite()) {
        return Err("its origin is not a point".into());
    }
    if !color.iter().all(|channel| (0.0..=1.0).contains(channel))
        || color.iter().all(|channel| *channel == 0.0)
    {
        return Err("each colour channel is 0 to 1, not all 0".into());
    }
    if !LIGHT_RADIUS.contains(&radius) {
        return Err(format!(
            "the radius is {} to {} units",
            LIGHT_RADIUS.start(),
            LIGHT_RADIUS.end()
        ));
    }
    if !LIGHT_INTENSITY.contains(&intensity) {
        return Err(format!(
            "the intensity is {} to {}",
            LIGHT_INTENSITY.start(),
            LIGHT_INTENSITY.end()
        ));
    }
    Ok(())
}

/// A colour as six hex digits, `ffcc80`.
pub(crate) fn hex(color: [f32; 3]) -> String {
    color
        .iter()
        .map(|channel| format!("{:02x}", (channel.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect()
}

/// Whether `command` is a plausible world effect command to keep: 1 to [`MAX_PATH`]
/// printable ASCII characters, none of the console's separators. What it does is the
/// weather's to read (an unknown one is skipped there).
fn weather_command(command: &str) -> bool {
    (1..=MAX_PATH).contains(&command.len())
        && command
            .bytes()
            .all(|byte| (b' '..=b'~').contains(&byte) && !b";\"".contains(&byte))
        && !command.trim().is_empty()
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
        (Kind::Weather { .. }, Kind::Weather { .. }) => true,
        (Kind::Lamp { fixture: a, .. }, Kind::Lamp { fixture: b, .. }) => a == b,
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
            align: Align::default(),
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

    /// One weather edit per scope; on a server its own wins over the every-server one.
    #[test]
    fn weather_is_one_edit_per_scope_and_the_servers_own_wins() {
        let weather = |commands: &[&str]| Kind::Weather {
            commands: commands
                .iter()
                .map(|command| (*command).to_owned())
                .collect(),
        };
        let mut document = Document::new("t2_rogue", 1);
        let all = document.add(weather(&["rain"]), None);
        assert_eq!(document.add(weather(&["heavyrain", "mist"]), None), all);
        let here = Some("1.2.3.4:29070");
        assert_eq!(
            document.weather(here),
            Some(["heavyrain".to_owned(), "mist".to_owned()].as_slice())
        );
        document.add(weather(&["snow"]), here.map(str::to_owned));
        assert_eq!(document.weather(here), Some(["snow".to_owned()].as_slice()));
        assert_eq!(document.weather(Some("5.6.7.8:1")).map(<[_]>::len), Some(2));
        assert_eq!(document.weather(None).map(<[_]>::len), Some(2));
        assert_eq!(
            document.edits[0].label(),
            "Weather: heavyrain, mist",
            "the label lists the commands"
        );
        let text = serde_json::to_string(&document).unwrap();
        assert!(
            text.contains(r#""kind":"weather","commands":["heavyrain","mist"]"#),
            "{text}"
        );
        assert_eq!(Document::parse(&text, "t2_rogue", 1).unwrap(), document);
        assert!(Document::new("t2_rogue", 1).weather(here).is_none());
    }

    #[test]
    fn weather_without_commands_or_with_a_separator_is_refused() {
        for commands in [
            vec![],
            vec!["rain; quit".to_owned()],
            vec![" ".to_owned()],
            vec!["rain".to_owned(); MAX_WEATHER + 1],
        ] {
            let mut document = Document::new("t2_rogue", 1);
            document.add(Kind::Weather { commands }, None);
            assert!(document.check().is_err());
        }
        let mut document = Document::new("t2_rogue", 1);
        document.add(
            Kind::Weather {
                commands: vec!["constantwind ( -5000 0 0 )".into(), "rain".into()],
            },
            None,
        );
        assert!(document.check().is_ok());
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

    /// An added light round-trips in the hub's shape, is named by its colour and reach,
    /// and is never merged with another (two lights can stand side by side).
    #[test]
    fn added_lights_round_trip_and_keep_their_limits() {
        let light = |radius: f32| Kind::Light {
            origin: [10.0, -20.0, 64.0],
            color: [1.0, 0.8, 0.5],
            radius,
            intensity: 1.0,
        };
        let mut document = Document::new("mp/ffa3", 1);
        let a = document.add(light(300.0), None);
        let b = document.add(light(300.0), None);
        assert_ne!(a, b, "lights are never merged");
        assert_eq!(document.edits[0].label(), "Light: #ffcc80 r300");
        let text = serde_json::to_string(&document).unwrap();
        assert!(
            text.contains(r#""kind":"light","origin":[10.0,-20.0,64.0],"color":[1.0,0.8,0.5],"radius":300.0,"intensity":1.0"#),
            "{text}"
        );
        assert_eq!(Document::parse(&text, "mp/ffa3", 1).unwrap(), document);
        // Intensity defaults to 1 when left out.
        let short = r#"{"map":"mp/ffa3","checksum":1,"edits":[{"id":1,"kind":"light","origin":[0,0,0],"color":[1,1,1],"radius":200}]}"#;
        let parsed = Document::parse(short, "mp/ffa3", 1).unwrap();
        assert!(matches!(parsed.edits[0].kind, Kind::Light { intensity, .. } if intensity == 1.0));
        for bad in [
            Kind::Light {
                origin: [f32::INFINITY, 0.0, 0.0],
                color: [1.0; 3],
                radius: 200.0,
                intensity: 1.0,
            },
            Kind::Light {
                origin: [0.0; 3],
                color: [0.0; 3],
                radius: 200.0,
                intensity: 1.0,
            },
            Kind::Light {
                origin: [0.0; 3],
                color: [1.5, 1.0, 1.0],
                radius: 200.0,
                intensity: 1.0,
            },
            light(8.0),
            light(5000.0),
            Kind::Light {
                origin: [0.0; 3],
                color: [1.0; 3],
                radius: 200.0,
                intensity: 9.0,
            },
        ] {
            let mut document = Document::new("mp/ffa3", 1);
            document.add(bad.clone(), None);
            assert!(document.check().is_err(), "{bad:?}");
        }
    }

    /// A lamp edit is one per fixture and scope (turning a moved lamp off keeps one
    /// edit), names its fixture, and refuses a fixture it cannot read or a wild offset.
    #[test]
    fn lamp_edits_are_one_per_fixture() {
        let lamp = |fixture: &str, off: bool, offset: [f32; 3]| Kind::Lamp {
            fixture: fixture.into(),
            name: "textures/kejim/light_panel".into(),
            off,
            offset,
        };
        let mut document = Document::new("mp/ffa5", 1);
        let moved = document.add(lamp("surface:1043", false, [0.0, 0.0, 16.0]), None);
        assert_eq!(
            document.edits[0].label(),
            "Lamp moved: light_panel (surface:1043)"
        );
        assert_eq!(
            document.add(lamp("surface:1043", true, [0.0; 3]), None),
            moved
        );
        assert_eq!(document.edits.len(), 1);
        assert_eq!(
            document.edits[0].label(),
            "Lamp off: light_panel (surface:1043)"
        );
        document.add(lamp("prop:12:3", true, [0.0; 3]), None);
        document.add(lamp("effect:40", false, [8.0, 0.0, 0.0]), None);
        assert_eq!(document.edits.len(), 3);
        let text = serde_json::to_string(&document).unwrap();
        assert!(
            text.contains(r#""kind":"lamp","fixture":"surface:1043""#),
            "{text}"
        );
        assert_eq!(Document::parse(&text, "mp/ffa5", 1).unwrap(), document);
        for bad in [
            lamp("surface", true, [0.0; 3]),
            lamp("wall:3", true, [0.0; 3]),
            lamp("prop:12", true, [0.0; 3]),
            lamp("surface:1", false, [f32::NAN, 0.0, 0.0]),
            lamp("surface:1", false, [0.0, 0.0, LAMP_OFFSET * 2.0]),
        ] {
            let mut document = Document::new("mp/ffa5", 1);
            document.add(bad.clone(), None);
            assert!(document.check().is_err(), "{bad:?}");
        }
    }

    /// A shown face's alignment round-trips, is left out at Radiant's default (a document
    /// of faces never aligned reads as before), keeps its limits, and a client from before
    /// alignment, whose `show_face` knows no such fields, still reads the whole document
    /// (serde ignores unknown fields: no `deny_unknown_fields` on the edits).
    #[test]
    fn a_shown_faces_alignment_is_optional_and_old_clients_read_it() {
        let face = |align: Align| Kind::ShowFace {
            brush: 40,
            side: 7,
            shader: "textures/system/caulk".into(),
            to: "textures/a/wall".into(),
            align,
        };
        let mut document = Document::new("mp/ffa3", 1);
        document.add(face(Align::default()), None);
        let plain = serde_json::to_string(&document).unwrap();
        assert!(
            !plain.contains("shift") && !plain.contains("scale") && !plain.contains("rotate"),
            "{plain}"
        );
        let aligned = Align {
            shift: [8.0, -16.0],
            scale: [0.25, -0.5],
            rotate: 15.0,
        };
        document.add(face(aligned), None);
        assert_eq!(document.edits.len(), 1, "one edit per face");
        let text = serde_json::to_string(&document).unwrap();
        assert!(
            text.contains(r#""shift":[8.0,-16.0],"scale":[0.25,-0.5],"rotate":15.0"#),
            "{text}"
        );
        let back = Document::parse(&text, "mp/ffa3", 1).unwrap();
        assert_eq!(back, document);
        assert_eq!(
            document.edits[0].describe(),
            "1: show brush 40 side 7 (textures/system/caulk) as textures/a/wall, \
             shift 8 -16, scale 0.25 -0.50, turn 15 [all servers]"
        );
        // A field left out takes its default.
        let partial = r#"{"map":"mp/ffa3","checksum":1,"edits":[{"id":1,"kind":"show_face","brush":1,"side":2,"shader":"a/b","to":"c/d","rotate":90}]}"#;
        let parsed = Document::parse(partial, "mp/ffa3", 1).unwrap();
        assert!(matches!(
            parsed.edits[0].kind,
            Kind::ShowFace { align, .. } if align == Align { rotate: 90.0, ..Align::default() }
        ));
        // The limits.
        for bad in [
            Align {
                scale: [0.0, 0.5],
                ..Align::default()
            },
            Align {
                scale: [100.0, 0.5],
                ..Align::default()
            },
            Align {
                shift: [f32::NAN, 0.0],
                ..Align::default()
            },
            Align {
                rotate: 1e6,
                ..Align::default()
            },
        ] {
            let mut document = Document::new("mp/ffa3", 1);
            document.add(face(bad), None);
            assert!(document.check().is_err(), "{bad:?}");
        }
        // The edit as clients before alignment declared it (this file's shape on main
        // before 10/11/2026): the new fields are unknown to it and ignored.
        #[derive(Debug, Deserialize)]
        struct OldDocument {
            edits: Vec<OldEdit>,
        }
        #[derive(Debug, Deserialize)]
        struct OldEdit {
            #[serde(flatten)]
            kind: OldKind,
        }
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(tag = "kind", rename_all = "snake_case")]
        enum OldKind {
            ShowFace {
                brush: u32,
                side: u32,
                shader: String,
                to: String,
            },
            Remap {
                from: String,
                to: String,
            },
        }
        let mut both = document.clone();
        both.add(
            Kind::Remap {
                from: "textures/a".into(),
                to: "textures/b".into(),
            },
            None,
        );
        let text = serde_json::to_string(&both).unwrap();
        let old: OldDocument = serde_json::from_str(&text).expect("an old client reads it");
        assert_eq!(
            old.edits[0].kind,
            OldKind::ShowFace {
                brush: 40,
                side: 7,
                shader: "textures/system/caulk".into(),
                to: "textures/a/wall".into(),
            }
        );
        assert_eq!(
            old.edits[1].kind,
            OldKind::Remap {
                from: "textures/a".into(),
                to: "textures/b".into(),
            }
        );
    }

    /// A client from before a kind is added refuses the whole document (serde knows no
    /// such variant): the caveat the docs give for publishing new kinds.
    #[test]
    fn an_unknown_kind_refuses_the_whole_document() {
        let text = r#"{"map":"mp/ffa3","checksum":1,"edits":[
            {"id":1,"kind":"remap","from":"a/b","to":"c/d"},
            {"id":2,"kind":"sparkles","amount":3}]}"#;
        assert!(Document::parse(text, "mp/ffa3", 1).is_err());
    }

    #[test]
    fn files_are_named_by_map_and_checksum() {
        assert_eq!(file_name("mp/FFA3", -1234), "mp_ffa3_-1234.json");
        assert_eq!(file_prefix("mp/ffa3"), "mp_ffa3_");
        assert!(file_name("mp/ffa3", 99).starts_with(&file_prefix("mp/ffa3")));
    }
}
