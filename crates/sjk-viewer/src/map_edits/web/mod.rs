//! The map editor in a web browser (`mapedit_web`, SJK's creator only for now, the
//! hub's creator flag): the client serves a page on
//! 127.0.0.1 ([`http`]) that draws the loaded map in 3D with WebGL and edits its SJK
//! map edits. The page is VisionJK's web admin drawing, cut down to the map
//! ([`page`]); every edit it asks for is one of the in-game editor's commands, run here
//! on the game thread ([`crate::GpuState::map_edit_command`]), so it is drawn in the game
//! at once, saved, undone and published exactly as the in-game editor's are.
//!
//! The server runs while the editor is on: `mapedit off` (or the editor's Editor off)
//! stops it, and so does quitting. A map change keeps it; the page sees the new map in
//! its next look at `api/edits` and loads it.

mod caulk;
pub(crate) mod http;
pub(crate) mod page;
pub(crate) mod recent;
mod things;

pub(crate) use caulk::Cache;

use super::document::Kind;
use crate::GpuState;
use crate::world_notes::EditTarget;
use glam::Vec3;
use http::{Ask, EditRequest, GameRequest, RecentRequest};
use serde_json::{Value, json};

/// Requests answered in one frame at most; the rest wait for the next frame.
const REQUESTS_PER_FRAME: usize = 16;

impl GpuState {
    /// `mapedit_web`: turn the editor on, start the page's server if it is not running,
    /// and open the page in the default browser.
    pub(super) fn map_edit_web(&mut self) -> Result<Vec<String>, String> {
        // Sol's choice (03/11/2026): the creator only for now.
        if !crate::player_identity::is_creator() {
            return Err("The browser map editor is for SJK's creator only for now".into());
        }
        if !self.map_edits.editor.active {
            self.map_edit_command("mapedit", &["on".to_owned()])?;
        }
        if self.map_edits.web.is_none() {
            let server = http::Server::start(self.vfs.clone())
                .map_err(|error| format!("The browser editor could not start: {error}"))?;
            crate::log::progress(format_args!(
                "map edits: browser editor serving {}",
                server.address()
            ));
            self.map_edits.web = Some(server);
        }
        let Some(server) = &self.map_edits.web else {
            return Err("The browser editor is not running".into());
        };
        crate::update::open_local_page(&server.url());
        Ok(vec![
            format!(
                "Map editor opened in your web browser ({}, this PC only)",
                server.address()
            ),
            "  It stops with mapedit off; mapedit_web opens the page again".into(),
        ])
    }

    /// Once a frame: answer what the page asked.
    pub(super) fn map_edit_web_frame(&mut self) {
        let Some(server) = &self.map_edits.web else {
            return;
        };
        server.set_vfs(self.vfs.as_ref());
        for _ in 0..REQUESTS_PER_FRAME {
            let Some(ask) = self
                .map_edits
                .web
                .as_ref()
                .and_then(http::Server::next_request)
            else {
                return;
            };
            self.map_edit_web_answer(ask);
        }
    }

    fn map_edit_web_answer(&mut self, ask: Ask) {
        let answer = match &ask.request {
            _ if !crate::player_identity::is_creator() => {
                Err("The browser map editor is for SJK's creator only for now".into())
            }
            GameRequest::State => Ok(self.map_edit_web_state()),
            GameRequest::Lists => Ok(self.map_edit_web_lists()),
            GameRequest::Edit(edit) => {
                let edit = edit.clone();
                Ok(self.map_edit_web_edit(&edit))
            }
            GameRequest::Recent => Ok(self.map_edit_web_recent_json()),
            GameRequest::SetRecent(change) => {
                let recent = self.map_edit_web_recent();
                match change {
                    RecentRequest::Remember { on } => recent.set_remember(*on),
                    RecentRequest::Clear {} => recent.clear(),
                }
                self.map_edit_web_recent_save();
                Ok(self.map_edit_web_recent_json())
            }
            GameRequest::Brushes => {
                let map = self.world_load_map.clone();
                Ok(self.map_edits.web_cache.brushes(&map, &self.bsp).clone())
            }
            GameRequest::Link { brush, side, grow } => {
                let map = self.world_load_map.clone();
                let sides = self.map_edits.web_cache.sides(&map, &self.bsp);
                caulk::link_json(&self.bsp, sides, *brush, *side, *grow)
            }
            GameRequest::Suggest(suggest) => {
                let map = self.world_load_map.clone();
                let catalog = self.shaders.clone();
                let (sides, drawn) =
                    self.map_edits
                        .web_cache
                        .sides_and_drawn(&map, &self.bsp, Some(&catalog));
                Ok(caulk::suggest_json(sides, drawn, &suggest.faces))
            }
        };
        ask.answer(answer);
    }

    /// The Recent rows' lists, read from their file the first time.
    fn map_edit_web_recent(&mut self) -> &mut recent::Recent {
        if self.map_edits.web_recent.is_none() {
            let loaded = self
                .map_edits_folder()
                .map(|folder| recent::Recent::load(&folder))
                .unwrap_or_default();
            self.map_edits.web_recent = Some(loaded);
        }
        self.map_edits
            .web_recent
            .get_or_insert_with(Default::default)
    }

    fn map_edit_web_recent_save(&mut self) {
        let (Some(folder), Some(recent)) = (self.map_edits_folder(), &self.map_edits.web_recent)
        else {
            return;
        };
        if let Err(error) = recent.save(&folder) {
            crate::log::progress(format_args!("map edits: recent list not saved: {error}"));
        }
    }

    fn map_edit_web_recent_json(&mut self) -> Value {
        serde_json::to_value(&*self.map_edit_web_recent()).unwrap_or(Value::Null)
    }

    /// After editor command `name` (with `args`) did its work: a remap's or a shown
    /// face's shader and a placed model go first in the Recent rows.
    pub(super) fn map_edit_web_remember(&mut self, name: &str, args: &[String]) {
        let used = match (name, args.first()) {
            ("mapedit_remap" | "mapedit_sky", Some(shader)) if shader != "-" => {
                Some((true, shader.clone()))
            }
            ("mapedit_place", _) => {
                let current = self.map_edits.editor.current();
                self.map_edits
                    .document
                    .as_ref()
                    .and_then(|document| document.edits.iter().find(|e| Some(e.id) == current))
                    .and_then(|edit| match &edit.kind {
                        Kind::Place { model, .. } => Some((false, model.clone())),
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some((shader, used)) = used else {
            return;
        };
        let recent = self.map_edit_web_recent();
        let changed = if shader {
            recent.used_shader(&used)
        } else {
            recent.used_model(&used)
        };
        if changed {
            self.map_edit_web_recent_save();
        }
    }

    /// An edit the page asked for, run as the in-game editor's command; its answer is
    /// the panel's too.
    fn map_edit_web_edit(&mut self, edit: &EditRequest) -> Value {
        // Checked on every edit: a running page stops editing when the flag goes.
        if !crate::player_identity::is_creator() {
            return json!({ "ok": false, "error": "The browser map editor is for SJK's creator only for now" });
        }
        if !self.map_edits.editor.active {
            return json!({ "ok": false, "error": "The map editor is off in the game" });
        }
        let result = match edit {
            EditRequest::Select {
                eye,
                dir,
                forward,
                face,
            } => self.map_edit_web_select(*eye, *dir, *forward, *face),
            EditRequest::Clear => {
                self.world_notes.cancel_selection();
                Ok(vec!["Nothing selected".into()])
            }
            EditRequest::ApplyMany {
                faces,
                surfaces,
                shader,
            } => self.map_edit_many(faces, surfaces, Some(shader)),
            EditRequest::HideMany { faces, surfaces } => self.map_edit_many(faces, surfaces, None),
            EditRequest::Current { id } => {
                let placed = self.map_edits.document.as_ref().is_some_and(|document| {
                    document
                        .edits
                        .iter()
                        .any(|e| e.id == *id && matches!(e.kind, Kind::Place { .. }))
                });
                if placed {
                    // A selected placed model would win over the chosen one.
                    self.world_notes.cancel_selection();
                    self.map_edits.editor.set_current(*id);
                    self.map_edits.panel.stale();
                    Ok(vec![format!(
                        "Edit {id} is the placed model the buttons move"
                    )])
                } else {
                    Err(format!("No placed model {id}"))
                }
            }
            EditRequest::Place {
                eye, dir, forward, ..
            } => self
                .map_edit_web_select(*eye, *dir, *forward, None)
                .and_then(|_| self.map_edit_web_command(edit)),
            EditRequest::PlaceThing {
                thing,
                eye,
                dir,
                forward,
                kind,
                server,
                escort,
            } => self.map_edit_web_place_thing(
                *thing,
                [*eye, *dir, *forward],
                kind,
                *server,
                *escort,
            ),
            EditRequest::RemoveThing { thing, id } => self.map_edit_web_remove_thing(*thing, *id),
            // Only the boombox (the request refuses the others): stood again where the
            // page's gizmo put it, on the floor under that.
            EditRequest::SetThing { origin, yaw, .. } => self
                .place_boombox_at(Vec3::from_array(*origin), yaw.rem_euclid(360.0))
                .map(|line| vec![line]),
            _ => self.map_edit_web_command(edit),
        };
        let reply = match &result {
            Ok(lines) => json!({ "ok": true, "lines": lines }),
            Err(error) => json!({ "ok": false, "error": error }),
        };
        self.map_edits.panel.answer(result);
        self.map_edits.panel.stale();
        reply
    }

    fn map_edit_web_command(&mut self, edit: &EditRequest) -> Result<Vec<String>, String> {
        let (name, args) = edit.command().ok_or("not an editor command")?;
        self.map_edit_command(name, &args)
    }

    /// Select what the page's ray meets, as a click on the in-game scene does; with
    /// `face` (the brush view's caulk or nodraw face the click met), that face, unless a
    /// drawn surface stands in front of it.
    fn map_edit_web_select(
        &mut self,
        eye: [f32; 3],
        dir: [f32; 3],
        forward: [f32; 3],
        face: Option<[u32; 2]>,
    ) -> Result<Vec<String>, String> {
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let (eye_at, along) = (
            Vec3::from_array(eye),
            Vec3::from_array(dir).normalize_or_zero(),
        );
        if let Some([brush, side]) = face
            && let Some(met) = super::faces::met(&self.bsp, brush, side, eye_at, along)
        {
            // Within a unit of a drawn surface (a caulk side behind a decal, a face edits
            // drew) the face wins, as the in-game pick's own margin has it.
            let hidden = self
                .decal_surfaces
                .ray_hit(eye_at, along, met.distance)
                .is_some_and(|hit| hit.distance + 1.0 < met.distance);
            if !hidden {
                let hit = eye_at + along * met.distance;
                let summary = self.world_notes.select_face_for_edit(
                    &map,
                    [eye_at, Vec3::from_array(forward), hit],
                    met,
                );
                return Ok(vec![summary]);
            }
        }
        let summary = self.world_notes.select_for_edit_ray(
            crate::world_notes::Scene {
                bsp: &self.bsp,
                surfaces: &self.decal_surfaces,
                scratch: &mut self.trace_scratch,
                vfs: self.vfs.as_deref(),
                map: &map,
                remaps: Vec::new(),
                markers: Vec::new(),
                entities: self.map_edits.pick_entities(&self.bsp),
            },
            Vec3::from_array(eye),
            Vec3::from_array(forward),
            Vec3::from_array(dir),
        );
        summary
            .map(|summary| vec![summary])
            .ok_or_else(|| "Nothing there".into())
    }

    /// `api/edits`: the map, what the edits draw now, the edits, the selection and the
    /// player's view.
    fn map_edit_web_state(&self) -> Value {
        let state = &self.map_edits;
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map);
        let server = self.map_edits_server();
        let enabled = self
            .console
            .as_ref()
            .is_none_or(|console| console.map_edits_on());
        let document = state.document.as_ref();
        let edits: Vec<Value> = document
            .map(|document| {
                document
                    .edits
                    .iter()
                    .rev()
                    .map(|edit| {
                        json!({
                            "id": edit.id,
                            "label": edit.label(),
                            "describe": edit.describe(),
                            "active": edit.applies_on(server.as_deref()),
                            "edit": edit,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let active = document
            .filter(|_| enabled)
            .into_iter()
            .flat_map(|document| document.active(server.as_deref()));
        let mut remaps = Vec::new();
        let mut faces = Vec::new();
        for edit in active {
            match &edit.kind {
                Kind::Remap { from, to } => remaps.push(json!([from, to])),
                Kind::ShowFace {
                    brush,
                    side,
                    shader,
                    to,
                } => {
                    if let Some((polygon, normal)) =
                        super::faces::polygon(&self.bsp, *brush, *side, shader)
                    {
                        let size = self
                            .vfs
                            .as_ref()
                            .and_then(|vfs| super::texture_size(vfs, &self.shaders, to));
                        let uv = super::faces::texture_coordinates(&polygon, normal, size);
                        faces.push(json!({
                            "id": edit.id, "points": polygon, "normal": normal,
                            "uv": uv, "shader": to,
                        }));
                    }
                }
                _ => {}
            }
        }
        let placed: Vec<Value> = state
            .placed
            .iter()
            .map(|placed| {
                json!({
                    "id": placed.id, "model": placed.model, "origin": placed.origin,
                    "angles": placed.angles, "scale": placed.scale,
                })
            })
            .collect();
        let selection = self.world_notes.waiting_selection().map(|selection| {
            let (hit, _) = selection.aim();
            let (kind, extra) = match selection.edit_target() {
                EditTarget::Model {
                    number,
                    model,
                    shader,
                    classname,
                } => {
                    let placed = sjk_entity::parse_entity_lump(self.bsp.entities())
                        .ok()
                        .and_then(|lump| state.placed_edit(number, lump.len()));
                    (
                        if placed.is_some() { "placed" } else { "model" },
                        json!({
                            "entity": number, "model": model, "placed": placed,
                            "shader": shader, "classname": classname,
                        }),
                    )
                }
                EditTarget::Surface {
                    index,
                    shader,
                    kind,
                } => {
                    // What Hide whole model takes, drawn fainter than the surface.
                    let group = self.map_edit_hide_group(index);
                    (
                        "surface",
                        json!({
                            "surface": index, "shader": shader, "surface_kind": kind,
                            "group": group,
                        }),
                    )
                }
                EditTarget::Face {
                    brush,
                    side,
                    shader,
                    sky,
                } => {
                    // The side's polygon, drawn over the hole it leaves (or over the sky).
                    let points = super::faces::polygon(&self.bsp, brush, side, &shader)
                        .map(|(points, _)| points);
                    (
                        "face",
                        json!({
                            "brush": brush, "side": side, "shader": shader, "points": points,
                            "sky": sky,
                        }),
                    )
                }
                EditTarget::Other => ("other", json!({})),
            };
            json!({ "summary": selection.summary(), "kind": kind, "hit": hit.to_array(), "target": extra })
        });
        let view = self.world_notes.view().map(|(eye, forward)| {
            json!({
                "eye": eye.to_array(),
                "yaw": forward.y.atan2(forward.x).to_degrees(),
                "pitch": -forward.z.clamp(-1.0, 1.0).asin().to_degrees(),
            })
        });
        let (undo, redo) = state.history.can();
        let answer = match (state.panel.status.is_empty(), state.panel.status_error) {
            (true, _) => Value::Null,
            (false, error) => json!({ "text": state.panel.status, "error": error }),
        };
        json!({
            "map": map,
            "checksum": state.map().map(|(_, checksum)| checksum),
            "editor": state.editor.active,
            "enabled": enabled,
            "scope": match state.editor.scope() {
                super::document::Scope::AllServers => "all",
                super::document::Scope::ThisServer => "server",
            },
            "server": server,
            "revision": document.map(|document| document.revision),
            "unpublished": document.is_some_and(|document| document.unpublished),
            "edits": edits,
            "current": state.editor.current(),
            "undo": undo,
            "redo": redo,
            "hidden_props": state.applied.props,
            "hidden_surfaces": state.applied.surfaces,
            "placed": placed,
            "remaps": remaps,
            "faces": faces,
            "skies": super::faces::map_skies(&self.bsp),
            "selection": selection,
            "view": view,
            "things": self.map_edit_web_things(),
            "answer": answer,
            "hub": state.hub_note.as_ref().and_then(|note| note.as_ref().err()),
        })
    }

    /// `api/lists`: the models to place, the shaders a remap may draw and the skies a sky
    /// swaps for, each with its picture.
    fn map_edit_web_lists(&self) -> Value {
        let Some(vfs) = self.vfs.as_deref() else {
            return json!({ "models": [], "shaders": [], "skies": [] });
        };
        json!({
            "models": super::panel::placeable_models(vfs),
            "shaders": super::panel::remap_shaders(vfs, &self.shaders),
            "skies": sky_list(vfs, &self.shaders),
        })
    }
}

/// The sky shaders of `catalog` for the page's sky list: each one's name and picture (the
/// image file of its box's front, `_ft`, else of its first stage; none without either).
pub(crate) fn sky_list(
    vfs: &sjk_vfs::VirtualFileSystem,
    catalog: &sjk_shader::ShaderCatalog,
) -> Vec<Value> {
    super::panel::sky_shaders(catalog)
        .into_iter()
        .map(|name| {
            let picture = sky_picture(vfs, catalog, &name);
            json!({ "name": name, "picture": picture })
        })
        .collect()
}

/// The picture of sky shader `name`: its box's front image (`<box>_ft`), else the image
/// its first stage draws.
fn sky_picture(
    vfs: &sjk_vfs::VirtualFileSystem,
    catalog: &sjk_shader::ShaderCatalog,
    name: &str,
) -> Option<String> {
    let definition = catalog.get(name)?;
    let front = definition
        .sky
        .as_ref()
        .and_then(|sky| sky.outer_box.as_deref())
        .and_then(|prefix| {
            catalog
                .resolve_stage_image(vfs, &format!("{prefix}_ft"))
                .ok()
                .flatten()
        });
    let stage = || {
        definition
            .stage_images
            .iter()
            .find_map(|image| catalog.resolve_stage_image(vfs, image).ok().flatten())
    };
    front.or_else(stage).map(|path| path.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sky list offers only shaders with `skyParms`, each pictured by its box's front
    /// image, else by its first stage's, and a sky is told from a texture.
    #[test]
    fn the_sky_list_offers_skies_with_their_pictures() {
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        let script = b"textures/skies/night\n{\nskyParms env/night 512 -\n}\n\
textures/skies/clouds\n{\nskyParms - 256 -\n{\nmap textures/skies/cloudlayer\ntcMod scroll 0.01 0\n}\n}\n\
textures/skies/fog\n{\nskyParms - 512 -\n}\n\
textures/a/wall\n{\n{\nmap textures/a/wall\n}\n}\n";
        vfs.mount_memory(
            "test",
            [
                ("shaders/sky.shader", script.to_vec()),
                ("env/night_ft.jpg", b"jpg".to_vec()),
                ("textures/skies/cloudlayer.tga", b"tga".to_vec()),
                ("textures/a/wall.jpg", b"jpg".to_vec()),
            ],
        )
        .unwrap();
        let shaders = sjk_shader::ShaderCatalog::load(&vfs).unwrap();
        let list = sky_list(&vfs, &shaders);
        let named: Vec<(&str, Option<&str>)> = list
            .iter()
            .map(|sky| (sky["name"].as_str().unwrap(), sky["picture"].as_str()))
            .collect();
        assert_eq!(
            named,
            [
                (
                    "textures/skies/clouds",
                    Some("textures/skies/cloudlayer.tga")
                ),
                ("textures/skies/fog", None),
                ("textures/skies/night", Some("env/night_ft.jpg")),
            ]
        );
        assert!(super::super::editor::is_sky_shader(
            &shaders,
            "TEXTURES/SKIES/NIGHT"
        ));
        assert!(!super::super::editor::is_sky_shader(
            &shaders,
            "textures/a/wall"
        ));
        assert!(!super::super::editor::is_sky_shader(
            &shaders,
            "textures/a/none"
        ));
    }
}
