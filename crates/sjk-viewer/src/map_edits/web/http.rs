//! The browser map editor's HTTP server: `127.0.0.1` on a free port, one short-lived
//! thread per connection, a minimal HTTP/1.1 (one request a connection, `Connection:
//! close`).
//!
//! Every request names the server as `127.0.0.1:<port>` or `localhost:<port>` in its
//! `Host` (a page of another site that rebinds a name of its own to 127.0.0.1 is
//! refused), and carries the random token: the page's address as `?t=<token>`, every
//! `api/` request as the `X-SJK-Token` header the page adds. A POST must come from the
//! page itself (`Origin`) as JSON. Files are read only, from the game data, by a path
//! [`sjk_vfs::VirtualPath`] accepts (no `..`, no absolute path).
//!
//! What the game knows (the edits, the selection) is asked of the game thread through
//! [`Ask`]s, which [`super`] answers once a frame; this thread never touches the world.

use serde::Deserialize;
use sjk_vfs::{VirtualFileSystem, VirtualPath};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Longest request line or header line, and most header lines.
const MAX_LINE: usize = 8 * 1024;
const MAX_HEADERS: usize = 64;
/// Largest POST body (an edit is a few hundred bytes).
const MAX_BODY: usize = 64 * 1024;
/// Connections served at once; more wait for the browser to try again (503).
const MAX_CONNECTIONS: usize = 24;
/// How long a request waits for the game thread's answer.
const GAME_WAIT: Duration = Duration::from_secs(15);
/// The header the page sends its token in.
pub(crate) const TOKEN_HEADER: &str = "x-sjk-token";

/// What the page asks of the game thread.
#[derive(Debug, PartialEq)]
pub(crate) enum GameRequest {
    /// The map, its edits, the selection and the view (`GET api/edits`).
    State,
    /// The models a map edit may place and the shaders a remap may draw (`GET api/lists`).
    Lists,
    /// An edit (`POST api/edit`).
    Edit(EditRequest),
    /// The shaders and models used lately (`GET api/recent`, [`super::recent`]).
    Recent,
    /// A change to them (`POST api/recent`).
    SetRecent(RecentRequest),
    /// The invisible brush faces as `r_showBrushes` sorts them, and the liquid volumes
    /// (`GET api/brushes`).
    Brushes,
    /// The faces joined to side `side` of brush `brush` that players see, growing by
    /// `grow` (`GET api/link?brush=&side=&grow=plane|surface`).
    Link {
        brush: u32,
        side: u32,
        grow: crate::map_edits::linked::Grow,
    },
    /// The shaders the drawn surfaces round some faces use (`POST api/suggest`).
    Suggest(SuggestRequest),
}

/// Most faces and surfaces one request names (a link takes 512 at most).
pub(crate) const MAX_MANY: usize = 1024;

/// Faces asking which shader surrounds them.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SuggestRequest {
    /// Brush and side of each face.
    pub(crate) faces: Vec<[u32; 2]>,
}

impl SuggestRequest {
    pub(crate) fn parse(body: &[u8]) -> Result<Self, String> {
        let request: Self = serde_json::from_slice(body).map_err(|error| error.to_string())?;
        if request.faces.is_empty() || request.faces.len() > MAX_MANY {
            return Err(format!("between 1 and {MAX_MANY} faces"));
        }
        Ok(request)
    }
}

/// `api/link`'s query: `brush`, `side` and `grow` (`plane`, the default, or `surface`).
pub(crate) fn link_request(request: &Request) -> Option<GameRequest> {
    let number = |name: &str| request.field(name)?.parse::<u32>().ok();
    let grow = match request.field("grow").unwrap_or("plane") {
        "plane" => crate::map_edits::linked::Grow::Plane,
        "surface" => crate::map_edits::linked::Grow::Surface,
        _ => return None,
    };
    Some(GameRequest::Link {
        brush: number("brush")?,
        side: number("side")?,
        grow,
    })
}

/// A change to the Recent rows the page asks for.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RecentRequest {
    /// "Remember recent textures" on or off (off forgets them).
    Remember { on: bool },
    /// Forget the lists (a struct variant, so a stray field is refused too).
    Clear {},
}

impl RecentRequest {
    pub(crate) fn parse(body: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(body).map_err(|error| error.to_string())
    }
}

/// Things other systems than map edits stand on the map, which the page places and
/// removes through their own code ([`super::things`]).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Thing {
    /// The player's own boombox on the floor (`boombox_place`).
    Boombox,
    /// A mystery box (`mysterybox_place`, through the hub).
    MysteryBox,
    /// A Toy Master toy (`toymaster_place`, through the hub).
    Toy,
}

/// The game thread's answer: JSON, or why not.
pub(crate) type Answer = Result<serde_json::Value, String>;

/// A request waiting for the game thread, with where its answer goes.
pub(crate) struct Ask {
    pub(crate) request: GameRequest,
    reply: SyncSender<Answer>,
}

impl Ask {
    pub(crate) fn answer(self, answer: Answer) {
        let _ = self.reply.send(answer);
    }
}

/// An edit the page asks for: each one is the in-game editor's command
/// ([`EditRequest::command`]), so both stay one editor.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum EditRequest {
    /// Select what the ray from `eye` along `dir` meets, the page's view looking along
    /// `forward` (a placed model faces it).
    Select {
        eye: [f32; 3],
        dir: [f32; 3],
        forward: [f32; 3],
        /// The brush view's caulk or nodraw face the click met first (brush, side): it
        /// is selected unless a drawn surface stands in front of it.
        #[serde(default)]
        face: Option<[u32; 2]>,
    },
    /// Drop the selection (Esc in the page).
    Clear,
    /// `mapedit_hide [group]`
    Hide {
        #[serde(default)]
        group: bool,
    },
    /// `mapedit_place <model>` where the ray meets the map; a pasted copy brings its
    /// angles and its scale (`mapedit_place <model> <angles> <scale>`).
    Place {
        model: String,
        eye: [f32; 3],
        dir: [f32; 3],
        forward: [f32; 3],
        #[serde(default)]
        angles: Option<[f32; 3]>,
        #[serde(default)]
        scale: Option<[f32; 3]>,
    },
    /// Stand `thing` where the ray meets the map, facing the page's view: a boombox
    /// already down moves there; a toy is of `kind`; `server` for this server only,
    /// `escort` for a toy shown only where the creator is.
    PlaceThing {
        thing: Thing,
        eye: [f32; 3],
        dir: [f32; 3],
        forward: [f32; 3],
        #[serde(default)]
        kind: String,
        #[serde(default)]
        server: bool,
        #[serde(default)]
        escort: bool,
    },
    /// Take `thing` `id` away (the boombox: back on the shoulder, no id).
    RemoveThing {
        thing: Thing,
        #[serde(default)]
        id: i64,
    },
    /// A gizmo's drag in the page, put down: `mapedit_set` with each placed model's exact
    /// origin, angles and scale, one step to undo ([`crate::map_edits::gizmo`]).
    Set { edits: Vec<SetEdit> },
    /// The page's gizmo on the player's own boombox: stood again at `origin` (its floor
    /// under it), facing `yaw`. Boxes and toys have no move (the hub's), so only the
    /// boombox is taken.
    SetThing {
        thing: Thing,
        origin: [f32; 3],
        yaw: f32,
    },
    /// `mapedit_turn <degrees>`
    Turn { degrees: f32 },
    /// `mapedit_move <x> <y> <z>`
    Move { offset: [f32; 3] },
    /// `mapedit_scale <factor>`
    Scale { factor: f32 },
    /// `mapedit_remap <shader>`, `-` removes the remap.
    Remap { shader: String },
    /// `mapedit_sky <shader> [all]`: the selected sky (every sky of the map with `all`)
    /// swapped for sky shader `shader`; `-` puts the map's back.
    Sky {
        shader: String,
        #[serde(default)]
        all: bool,
    },
    /// `mapedit_scope all|server`
    Scope { scope: String },
    /// Make placed edit `id` the one Turn, Move and Scale act on.
    Current { id: u32 },
    /// `mapedit_delete <id>`
    Delete { id: u32 },
    /// `mapedit_undo`
    Undo,
    /// `mapedit_redo`
    Redo,
    /// `mapedit_publish`
    Publish,
    /// The page's multiple selection: draw caulk or nodraw faces (brush, side) with
    /// `shader` and remap the surfaces' shaders to it (`-` takes those edits away), one
    /// step to undo.
    ApplyMany {
        #[serde(default)]
        faces: Vec<[u32; 2]>,
        #[serde(default)]
        surfaces: Vec<u32>,
        shader: String,
    },
    /// The page's multiple selection hidden: the surfaces hidden, the faces drawn as
    /// nothing again, the placed models (by their edit's id) removed, one step to undo.
    HideMany {
        #[serde(default)]
        faces: Vec<[u32; 2]>,
        #[serde(default)]
        surfaces: Vec<u32>,
        #[serde(default)]
        placed: Vec<u32>,
    },
}

/// One placed model of [`EditRequest::Set`].
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetEdit {
    pub(crate) id: u32,
    pub(crate) origin: [f32; 3],
    pub(crate) angles: [f32; 3],
    pub(crate) scale: [f32; 3],
}

/// Longest model or shader name an edit takes (a document keeps 160 at most).
const MAX_NAME: usize = 160;

impl EditRequest {
    /// Read a POST body, refusing numbers that are not finite and names with spaces or
    /// control characters.
    pub(crate) fn parse(body: &[u8]) -> Result<Self, String> {
        let request: Self = serde_json::from_slice(body).map_err(|error| error.to_string())?;
        let finite = |values: &[f32]| values.iter().all(|value| value.is_finite());
        let ok = match &request {
            Self::Select {
                eye, dir, forward, ..
            } => finite(eye) && finite(dir) && finite(forward) && dir.iter().any(|v| *v != 0.0),
            Self::Place {
                model,
                eye,
                dir,
                forward,
                angles,
                scale,
            } => {
                name_ok(model)
                    && finite(eye)
                    && finite(dir)
                    && finite(forward)
                    && dir.iter().any(|v| *v != 0.0)
                    && angles.is_none_or(|angles| finite(&angles))
                    && scale.is_none_or(|scale| finite(&scale) && scale.iter().all(|v| *v > 0.0))
            }
            Self::PlaceThing {
                thing,
                eye,
                dir,
                forward,
                kind,
                ..
            } => {
                finite(eye)
                    && finite(dir)
                    && finite(forward)
                    && dir.iter().any(|v| *v != 0.0)
                    && match thing {
                        Thing::Toy => name_ok(kind),
                        _ => kind.is_empty(),
                    }
            }
            Self::RemoveThing { thing, id } => match thing {
                Thing::Boombox => *id == 0,
                _ => *id > 0,
            },
            Self::Set { edits } => {
                (1..=crate::map_edits::gizmo::MAX_SET).contains(&edits.len())
                    && edits.iter().all(|edit| {
                        finite(&edit.origin)
                            && finite(&edit.angles)
                            && finite(&edit.scale)
                            && edit.scale.iter().all(|v| *v > 0.0)
                    })
            }
            Self::SetThing { thing, origin, yaw } => {
                *thing == Thing::Boombox && finite(origin) && yaw.is_finite()
            }
            Self::Turn { degrees } => degrees.is_finite(),
            Self::Move { offset } => finite(offset),
            Self::Scale { factor } => factor.is_finite() && *factor > 0.0,
            Self::Remap { shader } | Self::Sky { shader, .. } => name_ok(shader),
            Self::Scope { scope } => scope == "all" || scope == "server",
            Self::ApplyMany {
                faces,
                surfaces,
                shader,
            } => many_ok(faces, surfaces) && name_ok(shader),
            Self::HideMany {
                faces,
                surfaces,
                placed,
            } => {
                placed.len() <= crate::map_edits::gizmo::MAX_SET
                    && (many_ok(faces, surfaces)
                        || (faces.len() + surfaces.len() == 0 && !placed.is_empty()))
            }
            _ => true,
        };
        if ok {
            Ok(request)
        } else {
            Err("bad values in the edit".into())
        }
    }

    /// The editor command and its arguments this edit runs; `None` for the page's own
    /// steps ([`Self::Select`], [`Self::Clear`], [`Self::Current`], the things, the
    /// multiple selection's; [`Self::Place`] selects first).
    pub(crate) fn command(&self) -> Option<(&'static str, Vec<String>)> {
        let number = |value: f32| format!("{value}");
        Some(match self {
            Self::Select { .. }
            | Self::Clear
            | Self::Current { .. }
            | Self::PlaceThing { .. }
            | Self::RemoveThing { .. }
            | Self::SetThing { .. }
            | Self::ApplyMany { .. }
            | Self::HideMany { .. } => return None,
            Self::Hide { group } => (
                "mapedit_hide",
                if *group { vec!["group".into()] } else { vec![] },
            ),
            Self::Place {
                model,
                angles,
                scale,
                ..
            } => {
                let mut args = vec![model.clone()];
                if angles.is_some() || scale.is_some() {
                    args.extend(angles.unwrap_or([0.0; 3]).map(number));
                    args.extend(scale.unwrap_or([1.0; 3]).map(number));
                }
                ("mapedit_place", args)
            }
            Self::Set { edits } => (
                crate::map_edits::gizmo::COMMAND,
                edits
                    .iter()
                    .flat_map(|edit| {
                        std::iter::once(edit.id.to_string()).chain(
                            edit.origin
                                .into_iter()
                                .chain(edit.angles)
                                .chain(edit.scale)
                                .map(number),
                        )
                    })
                    .collect(),
            ),
            Self::Turn { degrees } => ("mapedit_turn", vec![number(*degrees)]),
            Self::Move { offset } => ("mapedit_move", offset.iter().copied().map(number).collect()),
            Self::Scale { factor } => ("mapedit_scale", vec![number(*factor)]),
            Self::Remap { shader } => ("mapedit_remap", vec![shader.clone()]),
            Self::Sky { shader, all } => {
                let mut args = vec![shader.clone()];
                if *all {
                    args.push("all".into());
                }
                ("mapedit_sky", args)
            }
            Self::Scope { scope } => ("mapedit_scope", vec![scope.clone()]),
            Self::Delete { id } => ("mapedit_delete", vec![id.to_string()]),
            Self::Undo => ("mapedit_undo", vec![]),
            Self::Redo => ("mapedit_redo", vec![]),
            Self::Publish => ("mapedit_publish", vec![]),
        })
    }
}

/// A multiple selection: one face or surface at least, [`MAX_MANY`] at most.
fn many_ok(faces: &[[u32; 2]], surfaces: &[u32]) -> bool {
    let count = faces.len() + surfaces.len();
    (1..=MAX_MANY).contains(&count)
}

fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && !name.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// The game data the server reads, swapped by the game thread when it changes.
type SharedVfs = Arc<Mutex<Option<Arc<VirtualFileSystem>>>>;

/// The running server; dropping it stops it.
pub(crate) struct Server {
    port: u16,
    token: String,
    stop: Arc<AtomicBool>,
    vfs: SharedVfs,
    requests: Receiver<Ask>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// A random token: 32 hex digits from the system's generator.
fn new_token() -> std::io::Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl Server {
    /// Listen on 127.0.0.1 at a free port and serve `vfs`.
    pub(crate) fn start(vfs: Option<Arc<VirtualFileSystem>>) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let token = new_token()?;
        let stop = Arc::new(AtomicBool::new(false));
        let vfs: SharedVfs = Arc::new(Mutex::new(vfs));
        let (sender, requests) = std::sync::mpsc::channel();
        let context = Arc::new(Context {
            port,
            token: token.clone(),
            vfs: Arc::clone(&vfs),
            game: Mutex::new(sender),
            open: AtomicUsize::new(0),
        });
        let stopping = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("sjk-mapedit-web".into())
            .spawn(move || accept(&listener, &stopping, &context))?;
        Ok(Self {
            port,
            token,
            stop,
            vfs,
            requests,
            thread: Some(thread),
        })
    }

    /// The page's address, with its token.
    pub(crate) fn url(&self) -> String {
        format!("http://127.0.0.1:{}/?t={}", self.port, self.token)
    }

    /// The page's address without its token, to show.
    pub(crate) fn address(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

    /// Serve `vfs` from now on, when it is another game data.
    pub(crate) fn set_vfs(&self, vfs: Option<&Arc<VirtualFileSystem>>) {
        if let Ok(mut shared) = self.vfs.lock() {
            let same = match (shared.as_ref(), vfs) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            };
            if !same {
                *shared = vfs.cloned();
            }
        }
    }

    /// The next request for the game thread, if one waits.
    pub(crate) fn next_request(&self) -> Option<Ask> {
        self.requests.try_recv().ok()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accepting thread with a connection of our own.
        let _ = TcpStream::connect_timeout(
            &SocketAddr::from((Ipv4Addr::LOCALHOST, self.port)),
            Duration::from_millis(500),
        );
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// What every connection's thread shares.
struct Context {
    port: u16,
    token: String,
    vfs: SharedVfs,
    game: Mutex<Sender<Ask>>,
    open: AtomicUsize,
}

fn accept(listener: &TcpListener, stop: &AtomicBool, context: &Arc<Context>) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        if context.open.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            context.open.fetch_sub(1, Ordering::SeqCst);
            let _ = respond(&mut stream, &Response::text(503, "busy, try again"));
            continue;
        }
        let context = Arc::clone(context);
        let spawned = std::thread::Builder::new()
            .name("sjk-mapedit-web-request".into())
            .spawn(move || {
                serve(&mut stream, &context);
                context.open.fetch_sub(1, Ordering::SeqCst);
            });
        if spawned.is_err() {
            // The closure (and its count) is gone with the failed spawn.
        }
    }
}

fn serve(stream: &mut TcpStream, context: &Context) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let response = match read_request(&mut *stream) {
        Ok(request) => handle(&request, context),
        Err(status) => Response::text(status, "bad request"),
    };
    let _ = respond(stream, &response);
}

/// A parsed request.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Request {
    pub(crate) method: String,
    /// The path without its query, percent-decoded.
    pub(crate) path: String,
    /// The query's fields, percent-decoded.
    pub(crate) query: Vec<(String, String)>,
    /// Header names in lower case.
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl Request {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub(crate) fn field(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Read one request: its head, then a body of `Content-Length`. An error is the status
/// to answer.
pub(crate) fn read_request(stream: impl Read) -> Result<Request, u16> {
    let mut reader = BufReader::new(stream.take((MAX_LINE * (MAX_HEADERS + 1) + MAX_BODY) as u64));
    let mut line = String::new();
    let mut read_line = |line: &mut String| -> Result<(), u16> {
        line.clear();
        let mut bytes = Vec::new();
        reader
            .by_ref()
            .take(MAX_LINE as u64 + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|_| 400u16)?;
        if bytes.len() > MAX_LINE || !bytes.ends_with(b"\n") {
            return Err(if bytes.len() > MAX_LINE { 431 } else { 400 });
        }
        *line = String::from_utf8(bytes).map_err(|_| 400u16)?;
        Ok(())
    };
    read_line(&mut line)?;
    let mut parts = line.trim_end().split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(400);
    };
    if !version.starts_with("HTTP/1.") || !target.starts_with('/') {
        return Err(400);
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut request = Request {
        method: method.to_owned(),
        path: decode(path, false).ok_or(400u16)?,
        query: parse_query(query).ok_or(400u16)?,
        ..Request::default()
    };
    loop {
        read_line(&mut line)?;
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if request.headers.len() == MAX_HEADERS {
            return Err(431);
        }
        let (name, value) = header.split_once(':').ok_or(400u16)?;
        request
            .headers
            .push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    let length = match request.header("content-length") {
        Some(length) => length.parse::<usize>().map_err(|_| 400u16)?,
        None => 0,
    };
    if request.header("transfer-encoding").is_some() {
        return Err(411);
    }
    if length > MAX_BODY {
        return Err(413);
    }
    request.body = vec![0; length];
    reader.read_exact(&mut request.body).map_err(|_| 400u16)?;
    Ok(request)
}

/// Percent-decode `text` (`+` as a space in a query); `None` when it is not UTF-8 or a
/// `%` is not followed by two hex digits.
pub(crate) fn decode(text: &str, query: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = text.get(index + 1..index + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            }
            b'+' if query => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn parse_query(query: &str) -> Option<Vec<(String, String)>> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((decode(key, true)?, decode(value, true)?))
        })
        .collect()
}

/// Whether two texts are equal, in a time that does not depend on where they differ.
fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// Why a request is refused before it is served, or `None` when it may be.
pub(crate) fn refusal(request: &Request, port: u16, token: &str) -> Option<(u16, &'static str)> {
    let hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    let host_ok = request
        .header("host")
        .is_some_and(|host| hosts.iter().any(|known| known.eq_ignore_ascii_case(host)));
    if !host_ok {
        return Some((421, "wrong host"));
    }
    let given = if request.path.starts_with("/api/") {
        request.header(TOKEN_HEADER)
    } else {
        request.field("t")
    };
    if !given.is_some_and(|given| same_secret(given, token)) {
        return Some((403, "wrong or missing token"));
    }
    match request.method.as_str() {
        "GET" | "HEAD" => None,
        "POST" => {
            let origin_ok = request.header("origin").is_some_and(|origin| {
                hosts
                    .iter()
                    .any(|host| origin.eq_ignore_ascii_case(&format!("http://{host}")))
            });
            let json = request
                .header("content-type")
                .is_some_and(|kind| kind.to_ascii_lowercase().starts_with("application/json"));
            if !origin_ok {
                Some((403, "cross-origin request"))
            } else if !json {
                Some((415, "JSON only"))
            } else {
                None
            }
        }
        _ => Some((405, "method not allowed")),
    }
}

/// A game data path from a request: [`VirtualPath`]'s rules (relative, no `..`, no
/// drive), one leading slash forgiven as the game does.
pub(crate) fn game_path(name: &str) -> Option<VirtualPath> {
    VirtualPath::new(name.strip_prefix(['/', '\\']).unwrap_or(name)).ok()
}

/// An answer to send.
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) kind: &'static str,
    pub(crate) body: Vec<u8>,
}

impl Response {
    fn text(status: u16, text: &str) -> Self {
        Self {
            status,
            kind: "text/plain; charset=utf-8",
            body: text.as_bytes().to_vec(),
        }
    }

    fn json(value: &serde_json::Value) -> Self {
        Self {
            status: 200,
            kind: "application/json",
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        421 => "Misdirected Request",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    }
}

fn respond(stream: &mut impl Write, response: &Response) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\n\
         Referrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; \
         script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src 'self' blob: data:; \
         connect-src 'self'; frame-ancestors 'none'\r\n\r\n",
        response.status,
        reason(response.status),
        response.kind,
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

/// Serve `request`.
fn handle(request: &Request, context: &Context) -> Response {
    if let Some((status, why)) = refusal(request, context.port, &context.token) {
        return Response::text(status, why);
    }
    let vfs = context.vfs.lock().ok().and_then(|vfs| vfs.clone());
    match (request.method.as_str(), request.path.as_str()) {
        ("GET" | "HEAD", "/" | "/index.html") => Response {
            status: 200,
            kind: "text/html; charset=utf-8",
            body: super::page::page().as_bytes().to_vec(),
        },
        ("GET", "/api/map") => {
            let name = request.field("name").unwrap_or_default();
            file(vfs.as_deref(), &format!("maps/{name}.bsp"))
        }
        ("GET", "/api/file") => file(vfs.as_deref(), request.field("name").unwrap_or_default()),
        ("GET", "/api/shaders") => match vfs {
            Some(vfs) => Response::json(&serde_json::json!({ "files": shader_files(&vfs) })),
            None => Response::text(503, "no game data"),
        },
        ("GET", "/api/list") => match (vfs, request.field("kind")) {
            (Some(vfs), Some("dir")) => Response::json(&serde_json::json!({
                "names": list_dir(
                    &vfs,
                    request.field("dir").unwrap_or_default(),
                    request.field("ext").unwrap_or_default(),
                )
            })),
            (None, _) => Response::text(503, "no game data"),
            _ => Response::json(&serde_json::json!({ "names": [] })),
        },
        ("GET", "/api/edits") => ask_game(context, GameRequest::State),
        ("GET", "/api/lists") => ask_game(context, GameRequest::Lists),
        ("POST", "/api/edit") => match EditRequest::parse(&request.body) {
            Ok(edit) => ask_game(context, GameRequest::Edit(edit)),
            Err(why) => Response::text(422, &why),
        },
        ("GET", "/api/recent") => ask_game(context, GameRequest::Recent),
        ("POST", "/api/recent") => match RecentRequest::parse(&request.body) {
            Ok(change) => ask_game(context, GameRequest::SetRecent(change)),
            Err(why) => Response::text(422, &why),
        },
        ("GET", "/api/brushes") => ask_game(context, GameRequest::Brushes),
        ("GET", "/api/link") => match link_request(request) {
            Some(link) => ask_game(context, link),
            None => Response::text(422, "brush, side and grow (plane or surface)"),
        },
        ("POST", "/api/suggest") => match SuggestRequest::parse(&request.body) {
            Ok(suggest) => ask_game(context, GameRequest::Suggest(suggest)),
            Err(why) => Response::text(422, &why),
        },
        _ => Response::text(404, "not found"),
    }
}

/// Hand `request` to the game thread and wait for its answer.
fn ask_game(context: &Context, request: GameRequest) -> Response {
    let (reply, answer) = std::sync::mpsc::sync_channel(1);
    let sent = context
        .game
        .lock()
        .map(|game| game.send(Ask { request, reply }).is_ok())
        .unwrap_or(false);
    if !sent {
        return Response::text(503, "the game has stopped serving the editor");
    }
    match answer.recv_timeout(GAME_WAIT) {
        Ok(Ok(value)) => Response::json(&value),
        Ok(Err(why)) => Response::json(&serde_json::json!({ "ok": false, "error": why })),
        Err(_) => Response::text(504, "the game did not answer"),
    }
}

/// A file of the game data, by the content type of its extension.
fn file(vfs: Option<&VirtualFileSystem>, name: &str) -> Response {
    let Some(path) = game_path(name) else {
        return Response::text(400, "not a game path");
    };
    let Some(vfs) = vfs else {
        return Response::text(503, "no game data");
    };
    match vfs.read(path.as_str()) {
        Ok(Some(asset)) => Response {
            status: 200,
            kind: content_type(path.as_str()),
            body: asset.bytes,
        },
        _ => Response::text(404, "not in the game data"),
    }
}

fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "shader" | "oshader" | "txt" | "skin" | "cfg" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// The shader scripts in the order the catalog reads them (`sjk_shader`: the normal
/// ones, then the override files last to first), named below `shaders/`. The page lets
/// the last one win, as the catalog does.
pub(crate) fn shader_files(vfs: &VirtualFileSystem) -> Vec<String> {
    let paths = vfs.paths();
    let names = |extension: &'static str| {
        paths.iter().filter_map(move |path| {
            path.as_str()
                .strip_prefix("shaders/")
                .filter(|name| !name.contains('/') && name.ends_with(extension))
                .map(str::to_owned)
        })
    };
    let overrides: Vec<String> = names(".oshader").collect();
    names(".shader")
        .chain(overrides.into_iter().rev())
        .collect()
}

/// `api/list?kind=dir`: the folders (with a `/` at their end) and the files with one of
/// the `extensions` (`.jpg,.png`) of folder `dir`, each named from the root.
pub(crate) fn list_dir(vfs: &VirtualFileSystem, dir: &str, extensions: &str) -> Vec<String> {
    let Some(dir) = game_path(dir) else {
        return Vec::new();
    };
    let prefix = format!("{}/", dir.as_str());
    let extensions: Vec<String> = extensions
        .split(',')
        .map(|extension| extension.trim().to_ascii_lowercase())
        .filter(|extension| !extension.is_empty())
        .collect();
    let mut folders = BTreeSet::new();
    let mut files = BTreeSet::new();
    for path in vfs.paths() {
        let Some(rest) = path.as_str().strip_prefix(&prefix) else {
            continue;
        };
        match rest.split_once('/') {
            Some((folder, _)) => {
                folders.insert(format!("{prefix}{folder}/"));
            }
            None if extensions
                .iter()
                .any(|extension| rest.ends_with(extension.as_str())) =>
            {
                files.insert(path.as_str().to_owned());
            }
            None => {}
        }
    }
    folders.into_iter().chain(files).collect()
}

#[cfg(test)]
#[path = "http_e2e.rs"]
mod e2e;

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str) -> Result<Request, u16> {
        read_request(text.as_bytes())
    }

    #[test]
    fn a_request_is_read_with_its_query_headers_and_body() {
        let request = request(
            "POST /api/edit?name=textures%2Fa+b&x HTTP/1.1\r\nHost: 127.0.0.1:5\r\n\
             Content-Length: 4\r\nX-SJK-Token: abc\r\n\r\nbodyEXTRA",
        )
        .unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/edit");
        assert_eq!(request.field("name"), Some("textures/a b"));
        assert_eq!(request.field("x"), Some(""));
        assert_eq!(request.header("x-sjk-token"), Some("abc"));
        assert_eq!(request.body, b"body");
    }

    #[test]
    fn broken_or_oversized_requests_are_refused() {
        assert_eq!(request("GET / HTTP/1.1\r\n").unwrap_err(), 400);
        assert_eq!(request("GET /\r\n\r\n").unwrap_err(), 400);
        assert_eq!(request("GET http://x/ HTTP/1.1\r\n\r\n").unwrap_err(), 400);
        assert_eq!(request("GET /%zz HTTP/1.1\r\n\r\n").unwrap_err(), 400);
        assert_eq!(
            request("POST / HTTP/1.1\r\nContent-Length: 999999\r\n\r\n").unwrap_err(),
            413
        );
        assert_eq!(
            request("POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap_err(),
            411
        );
        let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(MAX_LINE));
        assert_eq!(request(&long).unwrap_err(), 431);
        let many = format!(
            "GET / HTTP/1.1\r\n{}\r\n",
            "A: b\r\n".repeat(MAX_HEADERS + 1)
        );
        assert_eq!(request(&many).unwrap_err(), 431);
    }

    fn with(method: &str, path: &str, headers: &[(&str, &str)]) -> Request {
        let (path, query) = path.split_once('?').unwrap_or((path, ""));
        Request {
            method: method.into(),
            path: path.into(),
            query: parse_query(query).unwrap(),
            headers: headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            body: Vec::new(),
        }
    }

    #[test]
    fn the_host_and_the_token_are_checked() {
        let token = "0123456789abcdef";
        let page = |host: &str, query: &str| {
            refusal(
                &with("GET", &format!("/{query}"), &[("host", host)]),
                80,
                token,
            )
        };
        assert_eq!(page("127.0.0.1:80", "?t=0123456789abcdef"), None);
        assert_eq!(page("LOCALHOST:80", "?t=0123456789abcdef"), None);
        // DNS rebinding: another name that leads here.
        assert_eq!(
            page("evil.example:80", "?t=0123456789abcdef").unwrap().0,
            421
        );
        assert_eq!(page("127.0.0.1:81", "?t=0123456789abcdef").unwrap().0, 421);
        assert_eq!(page("127.0.0.1:80", "").unwrap().0, 403);
        assert_eq!(page("127.0.0.1:80", "?t=0123456789abcdee").unwrap().0, 403);
        let api =
            |headers: &[(&str, &str)]| refusal(&with("GET", "/api/edits", headers), 80, token);
        assert_eq!(
            api(&[("host", "127.0.0.1:80"), (TOKEN_HEADER, token)]),
            None
        );
        // The page's query does not open the API.
        assert_eq!(
            refusal(
                &with(
                    "GET",
                    "/api/edits?t=0123456789abcdef",
                    &[("host", "127.0.0.1:80")]
                ),
                80,
                token
            )
            .unwrap()
            .0,
            403
        );
        assert_eq!(
            api(&[("host", "127.0.0.1:80"), (TOKEN_HEADER, "x")])
                .unwrap()
                .0,
            403
        );
    }

    #[test]
    fn a_post_must_come_from_the_page_as_json() {
        let token = "t0k3n";
        let post = |headers: &[(&str, &str)]| {
            let mut all = vec![("host", "127.0.0.1:9"), (TOKEN_HEADER, token)];
            all.extend_from_slice(headers);
            refusal(&with("POST", "/api/edit", &all), 9, token).map(|(status, _)| status)
        };
        assert_eq!(
            post(&[
                ("origin", "http://127.0.0.1:9"),
                ("content-type", "application/json")
            ]),
            None
        );
        assert_eq!(post(&[("content-type", "application/json")]), Some(403));
        assert_eq!(
            post(&[
                ("origin", "http://evil.example"),
                ("content-type", "application/json")
            ]),
            Some(403)
        );
        assert_eq!(
            post(&[
                ("origin", "http://127.0.0.1:9"),
                ("content-type", "text/plain")
            ]),
            Some(415)
        );
        assert_eq!(
            refusal(
                &with(
                    "DELETE",
                    "/api/edit",
                    &[("host", "127.0.0.1:9"), (TOKEN_HEADER, token)]
                ),
                9,
                token
            )
            .unwrap()
            .0,
            405
        );
    }

    #[test]
    fn game_paths_stay_in_the_game_data() {
        assert_eq!(
            game_path("Textures/A/B.jpg").unwrap().as_str(),
            "textures/a/b.jpg"
        );
        assert_eq!(game_path("/models/a.md3").unwrap().as_str(), "models/a.md3");
        assert!(game_path("../secret.txt").is_none());
        assert!(game_path("textures/../../x").is_none());
        assert!(game_path("C:/Windows/win.ini").is_none());
        assert!(game_path("//server/share").is_none());
        assert!(game_path("").is_none());
        assert!(game_path("a\0b").is_none());
    }

    #[test]
    fn edits_are_read_into_the_editors_commands() {
        let parse = |json: &str| EditRequest::parse(json.as_bytes());
        assert_eq!(
            parse(r#"{"op":"hide","group":true}"#).unwrap().command(),
            Some(("mapedit_hide", vec!["group".to_owned()]))
        );
        assert_eq!(
            parse(r#"{"op":"hide"}"#).unwrap().command(),
            Some(("mapedit_hide", vec![]))
        );
        assert_eq!(
            parse(r#"{"op":"move","offset":[1,-2.5,0]}"#)
                .unwrap()
                .command(),
            Some((
                "mapedit_move",
                vec!["1".to_owned(), "-2.5".to_owned(), "0".to_owned()]
            ))
        );
        assert_eq!(
            parse(r#"{"op":"remap","shader":"textures/a/b"}"#)
                .unwrap()
                .command(),
            Some(("mapedit_remap", vec!["textures/a/b".to_owned()]))
        );
        assert_eq!(
            parse(r#"{"op":"delete","id":7}"#).unwrap().command(),
            Some(("mapedit_delete", vec!["7".to_owned()]))
        );
        assert_eq!(
            parse(r#"{"op":"publish"}"#).unwrap().command(),
            Some(("mapedit_publish", vec![]))
        );
        let place = parse(
            r#"{"op":"place","model":"models/a.md3","eye":[0,0,0],"dir":[1,0,0],"forward":[1,0,0]}"#,
        )
        .unwrap();
        assert_eq!(
            place.command(),
            Some(("mapedit_place", vec!["models/a.md3".to_owned()]))
        );
        assert_eq!(
            parse(r#"{"op":"select","eye":[0,0,0],"dir":[0,0,1],"forward":[1,0,0]}"#)
                .unwrap()
                .command(),
            None
        );
        // Refused: unknown ops and fields, spaces in names, numbers out of range, a
        // ray without a direction.
        assert!(parse(r#"{"op":"exec","command":"quit"}"#).is_err());
        assert!(parse(r#"{"op":"hide","extra":1}"#).is_err());
        assert!(parse(r#"{"op":"remap","shader":"a; quit"}"#).is_err());
        assert!(parse(r#"{"op":"remap","shader":""}"#).is_err());
        assert_eq!(
            parse(r#"{"op":"sky","shader":"textures/skies/night"}"#)
                .unwrap()
                .command(),
            Some(("mapedit_sky", vec!["textures/skies/night".to_owned()]))
        );
        assert_eq!(
            parse(r#"{"op":"sky","shader":"-","all":true}"#)
                .unwrap()
                .command(),
            Some(("mapedit_sky", vec!["-".to_owned(), "all".to_owned()]))
        );
        assert!(parse(r#"{"op":"sky","shader":"a b"}"#).is_err());
        assert!(parse(r#"{"op":"sky","shader":"a","all":"yes"}"#).is_err());

        assert!(parse(r#"{"op":"scale","factor":0}"#).is_err());
        assert!(parse(r#"{"op":"scale","factor":-1}"#).is_err());
        assert!(parse(r#"{"op":"scope","scope":"everywhere"}"#).is_err());
        assert!(parse(r#"{"op":"select","eye":[0,0,0],"dir":[0,0,0],"forward":[1,0,0]}"#).is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn a_gizmo_drag_is_one_set_of_every_model() {
        let parse = |json: &str| EditRequest::parse(json.as_bytes());
        let set = parse(
            r#"{"op":"set","edits":[{"id":3,"origin":[1,2,3.5],"angles":[0,90,0],"scale":[1,1,2]},{"id":4,"origin":[0,0,0],"angles":[10,0,0],"scale":[1,1,1]}]}"#,
        )
        .unwrap();
        let (name, args) = set.command().unwrap();
        assert_eq!(name, "mapedit_set");
        assert_eq!(args.len(), 20);
        assert_eq!(&args[..4], ["3", "1", "2", "3.5"]);
        let sets = crate::map_edits::gizmo::parse_set(&args).unwrap();
        assert_eq!(sets[0].scale, Some([1.0, 1.0, 2.0]));
        assert_eq!(sets[1].angles, [10.0, 0.0, 0.0]);
        // Refused: none, a scale of 0, a missing scale, an unknown field.
        assert!(parse(r#"{"op":"set","edits":[]}"#).is_err());
        assert!(parse(r#"{"op":"set","edits":[{"id":3,"origin":[1,2,3],"angles":[0,0,0],"scale":[1,0,1]}]}"#).is_err());
        assert!(
            parse(r#"{"op":"set","edits":[{"id":3,"origin":[1,2,3],"angles":[0,0,0]}]}"#).is_err()
        );
        assert!(parse(r#"{"op":"set","edits":[{"id":3,"origin":[1,2,3],"angles":[0,0,0],"scale":[1,1,1],"x":1}]}"#).is_err());
        // The boombox stands again; a box or a toy has no move.
        assert_eq!(
            parse(r#"{"op":"set_thing","thing":"boombox","origin":[1,2,3],"yaw":90}"#).unwrap(),
            EditRequest::SetThing {
                thing: Thing::Boombox,
                origin: [1.0, 2.0, 3.0],
                yaw: 90.0
            }
        );
        assert!(
            parse(r#"{"op":"set_thing","thing":"mystery_box","origin":[1,2,3],"yaw":0}"#).is_err()
        );
        assert!(parse(r#"{"op":"set_thing","thing":"toy","origin":[1,2,3],"yaw":0}"#).is_err());
    }

    #[test]
    fn a_pasted_copy_brings_its_angles_and_scale() {
        let parse = |json: &str| EditRequest::parse(json.as_bytes());
        let ray = r#""eye":[0,0,0],"dir":[1,0,0],"forward":[1,0,0]"#;
        let place = |extra: &str| {
            parse(&format!(
                r#"{{"op":"place","model":"models/a.md3",{ray}{extra}}}"#
            ))
        };
        assert_eq!(
            place(r#","angles":[0,90,0],"scale":[2,2,1.5]"#)
                .unwrap()
                .command(),
            Some((
                "mapedit_place",
                ["models/a.md3", "0", "90", "0", "2", "2", "1.5"]
                    .map(str::to_owned)
                    .to_vec()
            ))
        );
        assert_eq!(
            place(r#","angles":[0,45,0]"#)
                .unwrap()
                .command()
                .unwrap()
                .1
                .len(),
            7
        );
        assert_eq!(place("").unwrap().command().unwrap().1.len(), 1);
        assert!(place(r#","scale":[0,1,1]"#).is_err());
        assert!(place(r#","angles":[1,2]"#).is_err());
    }

    #[test]
    fn things_are_placed_and_removed_by_their_own_requests() {
        let parse = |json: &str| EditRequest::parse(json.as_bytes());
        let ray = r#""eye":[0,0,0],"dir":[1,0,0],"forward":[1,0,0]"#;
        let thing = |fields: &str| parse(&format!(r#"{{"op":"place_thing",{fields},{ray}}}"#));
        let toy = thing(r#""thing":"toy","kind":"cat","server":true,"escort":true"#).unwrap();
        assert_eq!(
            toy,
            EditRequest::PlaceThing {
                thing: Thing::Toy,
                eye: [0.0; 3],
                dir: [1.0, 0.0, 0.0],
                forward: [1.0, 0.0, 0.0],
                kind: "cat".into(),
                server: true,
                escort: true,
            }
        );
        // Not an editor command: the game thread runs each thing's own code.
        assert_eq!(toy.command(), None);
        assert!(thing(r#""thing":"boombox""#).is_ok());
        assert!(thing(r#""thing":"mystery_box""#).is_ok());
        // A toy needs a kind without spaces; the others take none.
        assert!(thing(r#""thing":"toy""#).is_err());
        assert!(thing(r#""thing":"toy","kind":"a b""#).is_err());
        assert!(thing(r#""thing":"boombox","kind":"x""#).is_err());
        assert!(thing(r#""thing":"car""#).is_err());
        assert!(
            parse(
                r#"{"op":"place_thing","thing":"boombox","eye":[0,0,0],"dir":[0,0,0],"forward":[1,0,0]}"#
            )
            .is_err()
        );
        assert_eq!(
            parse(r#"{"op":"remove_thing","thing":"mystery_box","id":12}"#).unwrap(),
            EditRequest::RemoveThing {
                thing: Thing::MysteryBox,
                id: 12
            }
        );
        assert!(parse(r#"{"op":"remove_thing","thing":"boombox"}"#).is_ok());
        assert!(parse(r#"{"op":"remove_thing","thing":"toy"}"#).is_err());
        assert!(parse(r#"{"op":"remove_thing","thing":"boombox","id":3}"#).is_err());
    }

    #[test]
    fn the_multiple_selection_is_read_and_bounded() {
        let parse = |json: &str| EditRequest::parse(json.as_bytes());
        let apply = parse(
            r#"{"op":"apply_many","faces":[[3,17],[4,22]],"surfaces":[9],"shader":"textures/a/b"}"#,
        )
        .unwrap();
        assert_eq!(
            apply,
            EditRequest::ApplyMany {
                faces: vec![[3, 17], [4, 22]],
                surfaces: vec![9],
                shader: "textures/a/b".into(),
            }
        );
        // not a console command: the game thread runs it as one step
        assert_eq!(apply.command(), None);
        assert!(parse(r#"{"op":"apply_many","faces":[[1,2]],"shader":"-"}"#).is_ok());
        assert!(parse(r#"{"op":"hide_many","surfaces":[1,2,3]}"#).is_ok());
        // placed models: removed with the rest, or alone; no more than one set takes
        assert!(parse(r#"{"op":"hide_many","faces":[[1,2]],"placed":[4,5]}"#).is_ok());
        assert!(parse(r#"{"op":"hide_many","placed":[4,5]}"#).is_ok());
        let models: Vec<String> = (0..=crate::map_edits::gizmo::MAX_SET)
            .map(|i| i.to_string())
            .collect();
        assert!(
            parse(&format!(
                r#"{{"op":"hide_many","placed":[{}]}}"#,
                models.join(",")
            ))
            .is_err()
        );
        // refused: nothing named, too many, a shader with a space, a bad face
        assert!(parse(r#"{"op":"hide_many"}"#).is_err());
        assert!(parse(r#"{"op":"apply_many","faces":[[1,2]],"shader":"a b"}"#).is_err());
        assert!(parse(r#"{"op":"apply_many","faces":[[1]],"shader":"x/y"}"#).is_err());
        let many: Vec<String> = (0..=MAX_MANY).map(|i| i.to_string()).collect();
        assert!(
            parse(&format!(
                r#"{{"op":"hide_many","surfaces":[{}]}}"#,
                many.join(",")
            ))
            .is_err()
        );
        // a click on the brush view names its face; Esc clears
        let select = parse(
            r#"{"op":"select","eye":[0,0,0],"dir":[0,0,-1],"forward":[1,0,0],"face":[5,30]}"#,
        )
        .unwrap();
        assert!(matches!(
            select,
            EditRequest::Select {
                face: Some([5, 30]),
                ..
            }
        ));
        assert_eq!(parse(r#"{"op":"clear"}"#).unwrap(), EditRequest::Clear);
    }

    #[test]
    fn links_and_suggestions_are_read_from_their_requests() {
        let link = |query: &str| link_request(&with("GET", &format!("/api/link?{query}"), &[]));
        assert_eq!(
            link("brush=4&side=31&grow=surface"),
            Some(GameRequest::Link {
                brush: 4,
                side: 31,
                grow: crate::map_edits::linked::Grow::Surface,
            })
        );
        assert!(matches!(
            link("brush=4&side=31"),
            Some(GameRequest::Link {
                grow: crate::map_edits::linked::Grow::Plane,
                ..
            })
        ));
        assert_eq!(link("brush=4&side=31&grow=all"), None);
        assert_eq!(link("brush=-1&side=3"), None);
        assert_eq!(link("side=3"), None);
        assert_eq!(
            SuggestRequest::parse(br#"{"faces":[[1,2],[3,4]]}"#).unwrap(),
            SuggestRequest {
                faces: vec![[1, 2], [3, 4]]
            }
        );
        assert!(SuggestRequest::parse(br#"{"faces":[]}"#).is_err());
        assert!(SuggestRequest::parse(br#"{"faces":[[1,2]],"x":1}"#).is_err());
    }

    #[test]
    fn the_recent_rows_take_their_switch_and_a_clear() {
        assert_eq!(
            RecentRequest::parse(br#"{"op":"remember","on":false}"#).unwrap(),
            RecentRequest::Remember { on: false }
        );
        assert_eq!(
            RecentRequest::parse(br#"{"op":"clear"}"#).unwrap(),
            RecentRequest::Clear {}
        );
        assert!(RecentRequest::parse(br#"{"op":"remember"}"#).is_err());
        assert!(RecentRequest::parse(br#"{"op":"add","name":"x"}"#).is_err());
        assert!(RecentRequest::parse(br#"{"op":"clear","x":1}"#).is_err());
    }

    #[test]
    fn folders_and_shader_scripts_are_listed_from_the_game_data() {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "test",
            [
                ("textures/a/one.jpg", &b"x"[..]),
                ("textures/a/deep/two.tga", &b"x"[..]),
                ("textures/b.png", &b"x"[..]),
                ("textures/c.txt", &b"x"[..]),
                ("shaders/z.shader", &b"x"[..]),
                ("shaders/a.shader", &b"x"[..]),
                ("shaders/b.oshader", &b"x"[..]),
                ("shaders/a.oshader", &b"x"[..]),
            ],
        )
        .unwrap();
        assert_eq!(
            list_dir(&vfs, "textures", ".jpg,.png"),
            ["textures/a/", "textures/b.png"]
        );
        assert_eq!(
            list_dir(&vfs, "textures/a/", ".jpg"),
            ["textures/a/deep/", "textures/a/one.jpg"]
        );
        assert!(list_dir(&vfs, "../textures", ".jpg").is_empty());
        assert_eq!(
            shader_files(&vfs),
            ["a.shader", "z.shader", "b.oshader", "a.oshader"]
        );
    }
}
