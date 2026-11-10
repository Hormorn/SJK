//! The server end to end on a real socket, with a fake game thread: the page, the map,
//! an edit handed to the game, and refused requests.

use super::*;
use std::io::{Read, Write};

/// Send `head` (and `body`) to the server and read its whole answer: the status and
/// the body.
fn send(port: u16, head: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let split = answer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("a head");
    let status = std::str::from_utf8(&answer[9..12])
        .unwrap()
        .parse()
        .unwrap();
    (status, answer[split + 4..].to_vec())
}

fn get(port: u16, path: &str, token: Option<&str>, host: Option<&str>) -> (u16, Vec<u8>) {
    let host = host.map_or_else(|| format!("127.0.0.1:{port}"), str::to_owned);
    let token = token.map_or_else(String::new, |token| format!("X-SJK-Token: {token}\r\n"));
    send(
        port,
        &format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n{token}\r\n"),
        &[],
    )
}

/// Answer the game's requests as a game thread would, until the server is dropped.
fn fake_game(server: &Server, until: impl Fn() -> bool) {
    while !until() {
        match server.next_request() {
            Some(ask) => {
                let answer = match &ask.request {
                    GameRequest::State => Ok(serde_json::json!({ "map": "mp/test" })),
                    GameRequest::Lists => Ok(serde_json::json!({ "models": [] })),
                    GameRequest::Edit(edit) => Ok(serde_json::json!({
                        "ok": true,
                        "lines": [format!("{:?}", edit.command()), format!("{edit:?}")],
                    })),
                    GameRequest::Recent => Ok(serde_json::json!({ "remember": true })),
                    GameRequest::SetRecent(change) => {
                        Ok(serde_json::json!({ "change": format!("{change:?}") }))
                    }
                    GameRequest::Brushes => Ok(serde_json::json!({ "kinds": [], "faces": [] })),
                    GameRequest::Lamps => Ok(serde_json::json!({
                        "realtime": true,
                        "fixtures": [{ "id": "surface:12", "kind": "surface", "lamps": 3 }],
                    })),
                    GameRequest::Link { .. } | GameRequest::Suggest(_) => {
                        Ok(serde_json::json!({ "asked": format!("{:?}", ask.request) }))
                    }
                };
                ask.answer(answer);
            }
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
}

#[test]
fn the_page_the_map_and_an_edit_are_served_and_strangers_refused() {
    let mut vfs = VirtualFileSystem::new();
    vfs.mount_memory("test", [("maps/mp/test.bsp", &b"IBSP-bytes"[..])])
        .unwrap();
    let server = Server::start(Some(Arc::new(vfs))).unwrap();
    let token = server.token.clone();
    let port = server.port;
    assert!(server.url().ends_with(&format!("/?t={token}")));
    std::thread::scope(|scope| {
        let client = scope.spawn(|| {
        // The page, with its token in its address.
        let (status, page) = get(port, &format!("/?t={token}"), None, None);
        assert_eq!(status, 200);
        assert!(String::from_utf8(page).unwrap().contains("SJK map editor"));
        // The map's bytes from the game data.
        let (status, map) = get(port, "/api/map?name=mp/test", Some(&token), None);
        assert_eq!((status, map.as_slice()), (200, &b"IBSP-bytes"[..]));
        let (status, _) = get(port, "/api/map?name=mp/none", Some(&token), None);
        assert_eq!(status, 404);
        let (status, _) = get(port, "/api/file?name=../../x", Some(&token), None);
        assert_eq!(status, 400);
        // A question for the game thread.
        let (status, state) = get(port, "/api/edits", Some(&token), None);
        assert_eq!(status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&state).unwrap()["map"],
            "mp/test"
        );
        // An edit from the page.
        let body = br#"{"op":"turn","degrees":15}"#;
        let origin = format!("http://127.0.0.1:{}", port);
        let (status, answer) = send(
            port,
            &format!(
                "POST /api/edit HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-SJK-Token: {token}\r\n\
                 Origin: {origin}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                port,
                body.len()
            ),
            body,
        );
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("mapedit_turn"));
        // The Recent rows: read, and the switch from the page.
        let (status, recent) = get(port, "/api/recent", Some(&token), None);
        assert_eq!(status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&recent).unwrap()["remember"],
            true
        );
        let post = |path: &str, origin: &str, body: &[u8]| {
            send(
                port,
                &format!(
                    "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-SJK-Token: {token}\r\n\
                     Origin: {origin}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                ),
                body,
            )
        };
        let (status, answer) = post("/api/recent", &origin, br#"{"op":"remember","on":false}"#);
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("Remember { on: false }"));
        assert_eq!(post("/api/recent", &origin, br#"{"op":"forget_all"}"#).0, 422);
        assert_eq!(
            post("/api/recent", "http://evil.example", br#"{"op":"clear"}"#).0,
            403
        );
        assert_eq!(get(port, "/api/recent", None, None).0, 403);
        // A thing placed from the page reaches the game as its own request.
        let (status, answer) = post(
            "/api/edit",
            &origin,
            br#"{"op":"remove_thing","thing":"mystery_box","id":4}"#,
        );
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("None"));
        // Round 3: the brush view, a link, a suggestion and a multiple selection's fill.
        let (status, brushes) = get(port, "/api/brushes", Some(&token), None);
        assert_eq!(status, 200);
        assert!(String::from_utf8(brushes).unwrap().contains("kinds"));
        let (status, link) = get(
            port,
            "/api/link?brush=12&side=80&grow=surface",
            Some(&token),
            None,
        );
        assert_eq!(status, 200);
        let link = String::from_utf8(link).unwrap();
        assert!(link.contains("brush: 12") && link.contains("Surface"), "{link}");
        assert_eq!(get(port, "/api/link?brush=x", Some(&token), None).0, 422);
        assert_eq!(get(port, "/api/link?brush=1&side=2", None, None).0, 403);
        let (status, suggest) = post("/api/suggest", &origin, br#"{"faces":[[12,80],[13,86]]}"#);
        assert_eq!(status, 200);
        assert!(String::from_utf8(suggest).unwrap().contains("[12, 80]"));
        assert_eq!(post("/api/suggest", &origin, br#"{"faces":[]}"#).0, 422);
        assert_eq!(
            post(
                "/api/suggest",
                "http://evil.example",
                br#"{"faces":[[1,2]]}"#
            )
            .0,
            403
        );
        let (status, answer) = post(
            "/api/edit",
            &origin,
            br#"{"op":"apply_many","faces":[[12,80],[13,86]],"shader":"textures/a/b"}"#,
        );
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("ApplyMany"));
        assert_eq!(
            post("/api/edit", &origin, br#"{"op":"hide_many","faces":[]}"#).0,
            422
        );
        // The Lights layer: the map's lamp fixtures, a light added and set, a lamp
        // turned off, and a light out of its limits refused before the game sees it.
        let (status, lamps) = get(port, "/api/lamps", Some(&token), None);
        assert_eq!(status, 200);
        let lamps: serde_json::Value = serde_json::from_slice(&lamps).unwrap();
        assert_eq!(lamps["fixtures"][0]["id"], "surface:12");
        assert_eq!(get(port, "/api/lamps", None, None).0, 403);
        let (status, answer) = post(
            "/api/edit",
            &origin,
            br#"{"op":"add_light","eye":[0,0,64],"dir":[1,0,0],"forward":[1,0,0],"color":[1,0.8,0.6],"radius":320,"intensity":1}"#,
        );
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("mapedit_light"));
        let (status, answer) = post(
            "/api/edit",
            &origin,
            br#"{"op":"set_light","id":3,"origin":[8,0,64],"color":[1,1,1],"radius":200,"intensity":2}"#,
        );
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("mapedit_light_set"));
        let (status, answer) = post(
            "/api/edit",
            &origin,
            br#"{"op":"lamp","fixture":"surface:12","action":"off"}"#,
        );
        assert_eq!(status, 200);
        assert!(String::from_utf8(answer).unwrap().contains("mapedit_lamp"));
        assert_eq!(
            post(
                "/api/edit",
                &origin,
                br#"{"op":"set_light","id":3,"origin":[8,0,64],"color":[1,1,1],"radius":99999,"intensity":2}"#,
            )
            .0,
            422
        );
        assert_eq!(
            post(
                "/api/edit",
                "http://evil.example",
                br#"{"op":"lamp","fixture":"surface:12","action":"off"}"#,
            )
            .0,
            403
        );
        // Refused: no token, a wrong one, another host (DNS rebinding), a post from
        // another site.
        assert_eq!(get(port, "/", None, None).0, 403);
        assert_eq!(get(port, "/api/edits", None, None).0, 403);
        assert_eq!(get(port, "/api/edits", Some("nope"), None).0, 403);
        let rebound = format!("evil.example:{}", port);
        assert_eq!(
            get(port, &format!("/?t={token}"), None, Some(&rebound)).0,
            421
        );
        let (status, _) = send(
            port,
            &format!(
                "POST /api/edit HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-SJK-Token: {token}\r\n\
                 Origin: http://evil.example\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                port,
                body.len()
            ),
            body,
        );
        assert_eq!(status, 403);
        });
        fake_game(&server, || client.is_finished());
        client.join().unwrap();
    });
    drop(server);
    // Stopped: nothing answers on its port any more.
    assert!(TcpStream::connect((Ipv4Addr::LOCALHOST, port)).is_err());
}

/// For a look at the page in a browser without the game: serves the retail game data
/// (`SJK_MAPEDIT_WEB_GAMEDATA`, its `base` folder) as if the game were on
/// `SJK_MAPEDIT_WEB_MAP` (mp/duel6 by default) for `SJK_MAPEDIT_WEB_SECONDS`, printing
/// the page's address. Edits are refused, there is no game, but for the brush view's
/// picks, Esc and fills (printed as `SJK_MAPEDIT_WEB_EDIT=`).
#[test]
#[ignore = "serves a page for a manual look in a browser"]
fn serve_the_page_for_a_browser() {
    let base = std::env::var("SJK_MAPEDIT_WEB_GAMEDATA")
        .expect("SJK_MAPEDIT_WEB_GAMEDATA: the GameData folder");
    let map = std::env::var("SJK_MAPEDIT_WEB_MAP").unwrap_or_else(|_| "mp/duel6".into());
    let seconds: u64 = std::env::var("SJK_MAPEDIT_WEB_SECONDS")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .unwrap_or(120);
    let mut vfs = VirtualFileSystem::new();
    vfs.mount_pk3_directory(std::path::Path::new(&base).join("base"))
        .unwrap();
    // SJK's own models, as the game mounts them (the boombox, the mystery box).
    crate::boombox::mount(&mut vfs).unwrap();
    crate::mystery_box::mount(&mut vfs).unwrap();
    let vfs = Arc::new(vfs);
    let models = crate::map_edits::panel::placeable_models(&vfs);
    // Images of textures/ as shaders to remap to (the game adds its scripts' names).
    let shaders: Vec<String> = vfs
        .paths()
        .iter()
        .filter_map(|path| {
            let name = path.as_str();
            (name.starts_with("textures/") && name.ends_with(".jpg"))
                .then(|| name.trim_end_matches(".jpg").to_owned())
        })
        .collect();
    // With SJK_MAPEDIT_WEB_DEMO set, edits as the game would apply them: the first prop
    // hidden, a copy of its model placed beside it, the first 40 surfaces hidden, the
    // first surface's shader remapped.
    let demo = std::env::var_os("SJK_MAPEDIT_WEB_DEMO").is_some();
    let bsp = vfs
        .read(&format!("maps/{map}.bsp"))
        .unwrap()
        .map(|asset| sjk_bsp::Bsp::parse(&asset.bytes).unwrap());
    let (mut props, mut placed, mut remaps) = (vec![], vec![], vec![]);
    if let (true, Some(bsp)) = (demo, &bsp) {
        let lump = sjk_entity::parse_entity_lump(bsp.entities()).unwrap();
        if let Some((number, prop)) = lump
            .iter()
            .enumerate()
            .find(|(_, e)| e.get("classname") == Some("misc_model_static"))
        {
            props.push(number);
            let origin: Vec<f32> = prop
                .get("origin")
                .unwrap_or("0 0 0")
                .split_whitespace()
                .map(|v| v.parse().unwrap_or(0.0))
                .collect();
            placed.push(serde_json::json!({
                "id": 2, "model": prop.get("model"), "origin": [origin[0] + 96.0, origin[1], origin[2]],
                "angles": [0, 45, 0], "scale": [1.5, 1.5, 1.5],
            }));
        }
        let first = bsp.render().surfaces()[0].shader;
        remaps.push(serde_json::json!([
            bsp.shaders()[first].name_lossy(),
            "textures/yavin/bblock3"
        ]));
    }
    let surfaces: Vec<u32> = if demo { (0..40).collect() } else { vec![] };
    // With the demo, the view looks at the highest big floor left shown, with a placed copy,
    // the player's boombox and a mystery box on it; SJK_MAPEDIT_WEB_SELECT selects that
    // floor (`surface`), the copy (`placed`) or a caulk face over it (`face`).
    let select = std::env::var("SJK_MAPEDIT_WEB_SELECT").unwrap_or_default();
    let (mut selection, mut view) = (serde_json::Value::Null, serde_json::Value::Null);
    let mut things = serde_json::json!({ "list": [], "on_server": false, "toy_kinds": ["cat"] });
    // The Lights layer (08/11/2026): added lights and lamp edits kept here as the game
    // would, and three made-up lamp fixtures above the demo's floor.
    let (mut lights, mut lamp_edits, mut fixtures) = (
        Vec::<serde_json::Value>::new(),
        Vec::<serde_json::Value>::new(),
        Vec::<serde_json::Value>::new(),
    );
    let mut next_id = 10u32;
    if let (true, Some(bsp)) = (demo, &bsp) {
        let render = bsp.render();
        let floor = render
            .surfaces()
            .iter()
            .enumerate()
            .skip(40)
            .filter(|(_, surface)| {
                surface.kind == sjk_bsp::SurfaceKind::Planar
                    && !bsp.shaders()[surface.shader].name_lossy().contains("/sk")
            })
            .filter_map(|(index, surface)| {
                let corners = &render.vertices()[surface.vertices.clone()];
                if corners.first()?.normal[2] < 0.9 {
                    return None;
                }
                let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
                let mut middle = [0.0f32; 3];
                for corner in corners {
                    for axis in 0..2 {
                        low[axis] = low[axis].min(corner.position[axis]);
                        high[axis] = high[axis].max(corner.position[axis]);
                    }
                    for (sum, value) in middle.iter_mut().zip(corner.position) {
                        *sum += value / corners.len() as f32;
                    }
                }
                Some((index, (high[0] - low[0]) * (high[1] - low[1]), middle))
            })
            // the highest of the big ones, which the view from above sees
            .filter(|(_, area, _)| *area >= 128.0 * 128.0)
            .max_by(|a, b| a.2[2].total_cmp(&b.2[2]));
        if let Some((index, _, [x, y, z])) = floor {
            view = serde_json::json!({ "eye": [x - 300.0, y, z + 260.0], "yaw": 0, "pitch": 40 });
            for (k, (dx, colour)) in [
                (-96.0, [1.0, 0.6, 0.3]),
                (0.0, [0.4, 0.7, 1.0]),
                (96.0, [1.0, 1.0, 1.0]),
            ]
            .into_iter()
            .enumerate()
            {
                fixtures.push(serde_json::json!({
                    "id": format!("surface:{}", 100 + k), "kind": "surface",
                    "name": "textures/test/light_panel", "position": [x + dx, y + 48.0, z + 96.0],
                    "color": colour, "power": 2.0, "radius": 400.0, "lamps": 3 + k,
                }));
            }
            let model = placed
                .first()
                .map_or(serde_json::Value::Null, |p| p["model"].clone());
            placed.push(serde_json::json!({
                "id": 3, "model": model, "origin": [x, y - 80.0, z],
                "angles": [0, 0, 0], "scale": [1, 1, 1],
            }));
            let shader = bsp.shaders()[render.surfaces()[index].shader].name_lossy();
            let entities =
                sjk_entity::parse_entity_lump(bsp.entities()).map_or(0, |lump| lump.len());
            selection = match select.as_str() {
                "surface" => serde_json::json!({
                    "summary": format!("Surface {index}: {shader}"), "kind": "surface",
                    "hit": [x, y, z], "target": {
                        "surface": index, "shader": shader, "surface_kind": "planar",
                        "group": [index],
                    },
                }),
                "placed" => serde_json::json!({
                    "summary": "Placed model", "kind": "placed", "hit": [x, y - 80.0, z],
                    "target": { "entity": entities + 1, "model": model, "placed": 3, "shader": null },
                }),
                "face" => serde_json::json!({
                    "summary": "Caulk face", "kind": "face", "hit": [x, y, z + 48.0],
                    "target": {
                        "brush": 1, "side": 2, "shader": "textures/system/caulk",
                        "points": [
                            [x - 48.0, y - 48.0, z + 48.0], [x + 48.0, y - 48.0, z + 48.0],
                            [x + 48.0, y + 48.0, z + 48.0], [x - 48.0, y + 48.0, z + 48.0],
                        ],
                    },
                }),
                _ => serde_json::Value::Null,
            };
            let part = |model: &str, at: [f32; 3], scale: f32| serde_json::json!({ "model": model, "origin": at, "angles": [0, 180, 0], "scale": scale });
            things = serde_json::json!({
                "on_server": true, "toy_kinds": ["cat"],
                "list": [
                    {
                        "thing": "boombox", "id": 0, "label": "Your boombox", "mine": true,
                        "origin": [x + 64.0, y + 64.0, z],
                        "parts": [part(crate::boombox::MODEL, [x + 64.0, y + 64.0, z], crate::boombox::SCALE)],
                    },
                    {
                        "thing": "mystery_box", "id": 5, "label": "Mystery box 5", "mine": false,
                        "origin": [x + 64.0, y - 64.0, z],
                        "parts": [part(crate::mystery_box::MODEL, [x + 64.0, y - 64.0, z], 1.0)],
                    },
                ],
            });
        }
    }
    // SJK_MAPEDIT_WEB_VIEW ("x y z yaw pitch") puts the player's view there instead.
    if let Ok(given) = std::env::var("SJK_MAPEDIT_WEB_VIEW") {
        let v: Vec<f32> = given
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        if let [x, y, z, yaw, pitch] = v[..] {
            view = serde_json::json!({ "eye": [x, y, z], "yaw": yaw, "pitch": pitch });
        }
    }
    let recent = serde_json::json!({
        "remember": true,
        "shaders": shaders.iter().take(3).collect::<Vec<_>>(),
        "models": models.iter().take(2).collect::<Vec<_>>(),
    });
    // Round 3: the brush view, links and votes come from the real code on the map; a
    // click on a face of the brush view (or one the brush ray finds) selects it, Esc
    // clears, and every edit the page sends is printed (`SJK_MAPEDIT_WEB_EDIT=`).
    let catalog = Some(sjk_shader::ShaderCatalog::load_with_warnings(
        &vfs,
        |_, _| {},
    ));
    let brushes = bsp.as_ref().map(super::super::caulk::brushes_json);
    let sides = bsp.as_ref().map(crate::map_edits::linked::Sides::from_bsp);
    let drawn = bsp
        .as_ref()
        .map(|bsp| super::super::caulk::drawn_surfaces(bsp, catalog.as_ref()));
    // The sky (04/11/2026): the game data's skies for the page's list; a sky swap and a
    // cover drawn on a selected face are kept here as the game would draw them.
    let skies = catalog
        .as_ref()
        .map_or_else(Vec::new, |catalog| super::super::sky_list(&vfs, catalog));
    let mut shown_faces: Vec<serde_json::Value> = Vec::new();
    let server = Server::start(Some(vfs)).unwrap();
    println!("SJK_MAPEDIT_WEB_URL={}", server.url());
    let end = std::time::Instant::now() + Duration::from_secs(seconds);
    while std::time::Instant::now() < end {
        let Some(ask) = server.next_request() else {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        };
        let face_selection = |face: crate::map_edits::faces::Face, eye: [f32; 3]| {
            let hit = glam::Vec3::from_array(eye);
            serde_json::json!({
                "summary": format!("{} (brush {} side {})", face.shader, face.brush, face.side),
                "kind": "face", "hit": hit.to_array(),
                "target": {
                    "brush": face.brush, "side": face.side, "shader": face.shader,
                    "points": face.polygon, "sky": face.sky,
                },
            })
        };
        let answer = match &ask.request {
            GameRequest::Edit(edit) => {
                println!("SJK_MAPEDIT_WEB_EDIT={edit:?}");
                let picked = match (edit, &bsp) {
                    (EditRequest::Select { eye, dir, face, .. }, Some(bsp)) => {
                        let (at, along) = (
                            glam::Vec3::from_array(*eye),
                            glam::Vec3::from_array(*dir).normalize_or_zero(),
                        );
                        face.and_then(|[b, s]| crate::map_edits::faces::met(bsp, b, s, at, along))
                            .or_else(|| crate::map_edits::faces::pick(bsp, at, along, 8192.0))
                            .map(|face| {
                                let hit = at + along * face.distance;
                                face_selection(face, hit.to_array())
                            })
                    }
                    _ => None,
                };
                match edit {
                    EditRequest::Select { .. } => match picked {
                        Some(picked) => {
                            selection = picked;
                            Ok(serde_json::json!({ "ok": true, "lines": [selection["summary"]] }))
                        }
                        None => Err("no game: only caulk and nodraw faces are picked here".into()),
                    },
                    EditRequest::Clear => {
                        selection = serde_json::Value::Null;
                        Ok(serde_json::json!({ "ok": true, "lines": ["Nothing selected"] }))
                    }
                    // A gizmo's drag put down: the placed models where it says, as
                    // `mapedit_set` puts them.
                    EditRequest::Set { edits } => {
                        for set in edits {
                            if let Some(p) = placed.iter_mut().find(|p| p["id"] == set.id) {
                                p["origin"] = serde_json::json!(set.origin);
                                p["angles"] = serde_json::json!(set.angles);
                                p["scale"] = serde_json::json!(set.scale);
                            }
                        }
                        Ok(serde_json::json!({
                            "ok": true, "lines": [format!("(test) {} placed models set", edits.len())],
                        }))
                    }
                    EditRequest::SetThing { origin, yaw, .. } => {
                        if let Some(list) = things["list"].as_array_mut()
                            && let Some(boombox) = list.iter_mut().find(|t| t["thing"] == "boombox")
                        {
                            boombox["origin"] = serde_json::json!(origin);
                            boombox["parts"][0]["origin"] = serde_json::json!(origin);
                            boombox["parts"][0]["angles"] = serde_json::json!([0.0, yaw, 0.0]);
                        }
                        Ok(
                            serde_json::json!({ "ok": true, "lines": ["(test) boombox stood again"] }),
                        )
                    }
                    EditRequest::ApplyMany { faces, .. } => Ok(serde_json::json!({
                        "ok": true, "lines": [format!("(test) {} faces filled", faces.len())],
                    })),
                    EditRequest::AddLight {
                        eye,
                        dir,
                        color,
                        radius,
                        intensity,
                        ..
                    } => {
                        let at = glam::Vec3::from_array(*eye)
                            + glam::Vec3::from_array(*dir).normalize_or_zero() * 200.0;
                        next_id += 1;
                        lights.push(serde_json::json!({
                            "id": next_id, "origin": at.to_array(), "color": color,
                            "radius": radius, "intensity": intensity,
                        }));
                        Ok(
                            serde_json::json!({ "ok": true, "lines": [format!("{next_id}: light (test)")] }),
                        )
                    }
                    EditRequest::SetLight {
                        id,
                        origin,
                        color,
                        radius,
                        intensity,
                    } => match lights.iter_mut().find(|l| l["id"] == *id) {
                        Some(light) => {
                            *light = serde_json::json!({
                                "id": id, "origin": origin, "color": color,
                                "radius": radius, "intensity": intensity,
                            });
                            Ok(
                                serde_json::json!({ "ok": true, "lines": [format!("{id}: light set (test)")] }),
                            )
                        }
                        None => Err(format!("No light {id}")),
                    },
                    EditRequest::Lamp {
                        fixture,
                        action,
                        offset,
                    } => {
                        let old = lamp_edits
                            .iter()
                            .position(|e| e["edit"]["fixture"] == fixture.as_str());
                        let (mut off, mut moved) = old.map_or((false, [0.0f32; 3]), |at| {
                            let edit = &lamp_edits[at]["edit"];
                            let o = |i: usize| edit["offset"][i].as_f64().unwrap_or(0.0) as f32;
                            (edit["off"] == true, [o(0), o(1), o(2)])
                        });
                        match action {
                            super::LampAction::Off => off = true,
                            super::LampAction::On => off = false,
                            super::LampAction::Move => moved = *offset,
                            super::LampAction::Reset => (off, moved) = (false, [0.0; 3]),
                        }
                        if let Some(at) = old {
                            lamp_edits.remove(at);
                        }
                        if off || moved.iter().any(|v| *v != 0.0) {
                            next_id += 1;
                            lamp_edits.push(serde_json::json!({
                                "id": next_id, "label": format!("Lamp: {fixture}"), "describe": "", "active": true,
                                "edit": { "kind": "lamp", "fixture": fixture, "name": "", "off": off, "offset": moved },
                            }));
                        }
                        Ok(
                            serde_json::json!({ "ok": true, "lines": [format!("lamp {fixture} (test)")] }),
                        )
                    }
                    EditRequest::Delete { id } => {
                        lights.retain(|l| l["id"] != *id);
                        lamp_edits.retain(|e| e["id"] != *id);
                        Ok(
                            serde_json::json!({ "ok": true, "lines": [format!("Removed {id} (test)")] }),
                        )
                    }
                    EditRequest::Sky { shader, all } => {
                        let from: Vec<String> = match (&bsp, *all) {
                            (Some(bsp), true) => crate::map_edits::faces::map_skies(bsp),
                            _ => selection["target"]["shader"]
                                .as_str()
                                .map(|s| vec![s.to_owned()])
                                .unwrap_or_default(),
                        };
                        remaps.retain(|r| !from.iter().any(|f| r[0].as_str() == Some(f)));
                        if shader != "-" {
                            remaps.extend(from.iter().map(|f| serde_json::json!([f, shader])));
                        }
                        Ok(
                            serde_json::json!({ "ok": true, "lines": [format!("(test) {} skies -> {shader}", from.len())] }),
                        )
                    }
                    EditRequest::Remap { shader } if selection["kind"] == "face" => {
                        let target = &selection["target"];
                        let (brush, side) = (
                            target["brush"].as_u64().unwrap_or(0) as u32,
                            target["side"].as_u64().unwrap_or(0) as u32,
                        );
                        let own = target["shader"].as_str().unwrap_or("");
                        match bsp
                            .as_ref()
                            .and_then(|bsp| crate::map_edits::faces::polygon(bsp, brush, side, own))
                        {
                            Some((points, normal)) => {
                                let uv = crate::map_edits::faces::texture_coordinates(
                                    &points,
                                    normal,
                                    None,
                                    &crate::map_edits::document::Align::default(),
                                );
                                shown_faces.push(serde_json::json!({
                                    "id": shown_faces.len() + 1, "points": points,
                                    "normal": normal, "uv": uv, "shader": shader,
                                }));
                                Ok(
                                    serde_json::json!({ "ok": true, "lines": ["(test) face covered"] }),
                                )
                            }
                            None => Err("no such face".into()),
                        }
                    }
                    _ => Err("no game: this is the page's test server".into()),
                }
            }
            GameRequest::Brushes => brushes.clone().ok_or_else(|| "no map".into()),
            GameRequest::Lamps => Ok(serde_json::json!({
                "map": map, "realtime": true, "fixtures": fixtures,
                "applied": { "off": [], "moved": [] },
            })),
            GameRequest::Link { brush, side, grow } => match (&bsp, &sides) {
                (Some(bsp), Some(sides)) => {
                    super::super::caulk::link_json(bsp, sides, *brush, *side, *grow)
                }
                _ => Err("no map".into()),
            },
            GameRequest::Suggest(suggest) => match (&sides, &drawn) {
                (Some(sides), Some(drawn)) => Ok(super::super::caulk::suggest_json(
                    sides,
                    drawn,
                    &suggest.faces,
                )),
                _ => Err("no map".into()),
            },
            GameRequest::State => Ok(serde_json::json!({
                "map": map, "checksum": 1, "editor": true, "enabled": true, "scope": "all",
                "server": null, "revision": 0, "unpublished": false, "edits": lamp_edits, "current": null,
                "lights": lights, "lights_budget": crate::map_edits::lights::BUDGET,
                "undo": false, "redo": false, "hidden_props": props, "hidden_surfaces": surfaces,
                "placed": placed, "remaps": remaps, "faces": shown_faces, "selection": selection,
                "view": view, "things": things, "answer": null, "hub": null,
                "skies": bsp.as_ref().map(crate::map_edits::faces::map_skies),
            })),
            GameRequest::Lists => {
                Ok(serde_json::json!({ "models": models, "shaders": shaders, "skies": skies }))
            }

            GameRequest::SetRecent(_) => Err("no game: this is the page's test server".into()),
            GameRequest::Recent => Ok(recent.clone()),
        };
        ask.answer(answer);
    }
}
