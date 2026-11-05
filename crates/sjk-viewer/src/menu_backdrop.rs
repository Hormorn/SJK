//! Live-map backdrop behind the standalone client menus.
//!
//! The main menu sits over the real renderer instead of a flat gradient: the
//! camera parks at the map's `info_player_intermission` view (the vantage the
//! map author chose to show off the level) and drifts very slowly so the
//! screen never reads as a still image. Screens that ask for a different
//! shot (settings) fly there along a per-map authored route and back again.

mod flight;
pub(crate) mod pictures;
mod routes;
mod tour;

use super::GpuState;
use flight::Path;
use glam::Vec3;
pub(crate) use routes::Stage;
#[cfg(test)]
pub(crate) use routes::tour_for;
use sjk_bsp::Bsp;
use sjk_entity::parse_entity_lump;
use sjk_ui::{Easing, Tween};
use std::time::Instant;

/// Which backdrop shot a menu screen wants behind it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Shot {
    /// The intermission vantage behind the main menu.
    Main,
    /// The map's server-browser vantage (the big gate at the end of the
    /// corridor beside the ship on mp/ffa3).
    Browser,
    /// The map's settings vantage (the balcony behind the main vantage on mp/ffa3).
    Settings,
    /// The map's player stage (the tower balcony on mp/ffa3).
    Player,
    /// Where the thrown saber floats (between the tower walls on mp/ffa3),
    /// reached from the player stage.
    Saber,
    /// The Profile screen's Holocrons tab: a view over open ground, left clear for
    /// the 3D holocron, which floats before the camera wherever it is
    /// (`holocrons::stage`). A map without a route for it keeps the main vantage.
    Holocrons,
    /// The Collection's Saber Lab tab ([`crate::saber_lab`]): the player stage further
    /// off and to the left of the picture, the floating saber on its right. A map without
    /// a route for it takes the player's shot ([`Backdrop::flight_for`]).
    SaberLab,
}

/// Fraction of the flight over which the destination screen fades in. It
/// starts at departure so settings are usable while the camera is still
/// moving; the flight is scenery, not a wait.
const REVEAL_SPAN: f32 = 0.18;

/// Camera anchor for the menu backdrop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Vantage {
    pub(crate) origin: Vec3,
    pub(crate) yaw: f32,
    pitch: f32,
}

/// One drifted camera sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sample {
    pub(crate) origin: Vec3,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

/// One authored flight: from the main vantage, or chained onto the end of
/// another flight (`parent`).
struct Flight {
    shot: Shot,
    parent: Option<usize>,
    path: Path,
    millis: u32,
    stage: Option<Stage>,
    focus: Option<Vec3>,
}

/// Camera state behind the standalone menus: the main vantage, the routes
/// to the other shots, and where along the active one the camera is.
pub(crate) struct Backdrop {
    main: Vantage,
    flights: Vec<Flight>,
    /// Flight the camera is currently on (or parked at an end of).
    active: Option<usize>,
    /// 0 = parked where the active flight departs from (the main vantage or
    /// its parent's shot), 1 = parked on the active flight's shot.
    progress: Tween,
    last_progress: f32,
    /// The map's camera tour, when it has one: the main shot plays it, and
    /// the other shots are reached by a cut through dark instead of a flight
    /// (a tour's shots are all over the map, so no route starts from them).
    tour: Option<tour::Tour>,
    /// The cut under way on a toured map: the flight it leaves for (`None`
    /// back to the tour) and the backdrop time it began.
    cut: Option<(Option<usize>, u64)>,
    /// How dark the tour's fades and cuts make the world this frame.
    darkness: f32,
    /// Picture mode ([`pictures`]): pictures stand in for the map, which is empty.
    pictures: Option<pictures::Pictures>,
}

impl Backdrop {
    pub(crate) fn from_bsp(bsp: &Bsp) -> Option<Self> {
        let main = Vantage::from_bsp(bsp)?;
        Some(Self::for_map(main, worldspawn_message(bsp).as_deref()))
    }

    /// The backdrop of a picture set ([`crate::menu_pictures`]): the map whose
    /// worldspawn message is `message`, seen only through the pictures taken from
    /// `views`. Needs a main view or a tour.
    pub(crate) fn from_pictures(
        message: &str,
        views: Vec<(pictures::View, Vantage)>,
    ) -> Option<Self> {
        let main = views
            .iter()
            .find(|(view, _)| matches!(view, pictures::View::Main | pictures::View::Tour(0)))?
            .1;
        let mut backdrop = Self::for_map(main, Some(message));
        // Pictures play the tour; its glides and cuts do not.
        backdrop.tour = None;
        backdrop.pictures = Some(pictures::Pictures::new(views));
        Some(backdrop)
    }

    #[cfg(test)]
    /// Every view a picture set of this map is taken from: each tour shot (the main
    /// vantage instead, without a tour) and the end of each flight.
    pub(crate) fn picture_views(&self) -> Vec<(pictures::View, Vantage)> {
        let mut views = Vec::new();
        if self.tour.is_none() {
            views.push((pictures::View::Main, self.main));
        }
        if let Some(tour) = &self.tour {
            views.extend(
                tour.shots().iter().enumerate().map(|(index, shot)| {
                    (pictures::View::Tour(index), pictures::tour_vantage(shot))
                }),
            );
        }
        views.extend(
            self.flights
                .iter()
                .map(|flight| (pictures::View::Shot(flight.shot), flight.path.destination())),
        );
        views
    }

    /// The picture on show (its index in the set), in picture mode.
    pub(crate) fn shown_picture(&self) -> Option<usize> {
        self.pictures.as_ref()?.shown()
    }

    fn for_map(main: Vantage, message: Option<&str>) -> Self {
        let routes = [
            Shot::Browser,
            Shot::Settings,
            Shot::Player,
            Shot::Saber,
            Shot::Holocrons,
            Shot::SaberLab,
        ]
        .into_iter()
        .filter_map(|shot| routes::route_for(message?, shot))
        .collect::<Vec<_>>();
        let mut backdrop = Self::new(main, &routes);
        if let Some(shots) = message.and_then(routes::tour_for) {
            // Each start shuffles the tour differently.
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |time| time.as_nanos() as u64);
            backdrop.tour = Some(tour::Tour::new(shots, seed));
        }
        backdrop
    }

    /// Routes chained onto another shot must follow that shot's route; a
    /// route whose parent is missing is dropped.
    fn new(main: Vantage, routes: &[&routes::Route]) -> Self {
        let mut flights: Vec<Flight> = Vec::with_capacity(routes.len());
        for route in routes {
            let parent = match route.from {
                Shot::Main => None,
                from => match flights.iter().position(|flight| flight.shot == from) {
                    Some(parent) => Some(parent),
                    None => continue,
                },
            };
            let start = parent.map_or(main, |parent| flights[parent].path.destination());
            flights.push(Flight {
                shot: route.shot,
                parent,
                path: Path::new(start, route),
                millis: route.millis,
                stage: route.stage,
                focus: route.focus.map(Vec3::from_array),
            });
        }
        Self {
            main,
            flights,
            active: None,
            progress: Tween::settled(0.0),
            last_progress: 0.0,
            tour: None,
            cut: None,
            darkness: 0.0,
            pictures: None,
        }
    }

    /// How dark the tour's fades and cuts make the world this frame (0 clear,
    /// 1 black); 0 on a map without a tour.
    pub(crate) fn darkness(&self) -> f32 {
        self.darkness
    }

    /// The camera on a toured map at backdrop time `millis`, `wanted` the
    /// flight whose shot the screen asks for (`None`: the tour). Moving
    /// between the tour and a shot fades to black, cuts, and fades back in.
    fn drive_tour(&mut self, wanted: Option<usize>, millis: u64) -> Sample {
        let half = tour::FADE_MILLIS / 2;
        if self.cut.is_none() && wanted != self.active {
            self.cut = Some((wanted, millis));
        }
        let mut darkness = 0.0_f32;
        if let Some((to, began)) = &mut self.cut {
            let elapsed = millis.saturating_sub(*began);
            if elapsed < half {
                // Still fading out: the screen may change its mind.
                *to = wanted;
                darkness = elapsed as f32 / half as f32;
            } else {
                let to = *to;
                if self.active != to {
                    self.active = to;
                    if to.is_none()
                        && let Some(tour) = &mut self.tour
                    {
                        tour.advance(millis);
                    }
                }
                darkness = 1.0 - (elapsed - half) as f32 / half as f32;
                if elapsed >= 2 * half {
                    self.cut = None;
                }
            }
        }
        let seconds = millis as f32 / 1_000.0;
        let sample = match (self.active, &mut self.tour) {
            (Some(index), _) => self.flights[index].path.destination().sample(seconds),
            (None, Some(tour)) => {
                let (sample, fade) = tour.sample(millis);
                darkness = darkness.max(fade);
                sample
            }
            (None, None) => self.main.sample(seconds),
        };
        self.darkness = darkness.clamp(0.0, 1.0);
        sample
    }

    fn flight_for(&self, shot: Shot) -> Option<usize> {
        self.flights
            .iter()
            .position(|flight| flight.shot == shot)
            // The Saber Lab stands the model on the player's stage where the map has no
            // shot of its own for it.
            .or_else(|| (shot == Shot::SaberLab).then(|| self.flight_for(Shot::Player))?)
    }

    /// Whether `ancestor` is `flight` itself or a flight it chains onto.
    fn descends_from(&self, flight: Option<usize>, ancestor: Option<usize>) -> bool {
        let mut current = flight;
        loop {
            if current == ancestor {
                return true;
            }
            match current {
                Some(index) => current = self.flights[index].parent,
                None => return false,
            }
        }
    }

    /// The flight chained directly onto `parent` on the way to `wanted`.
    fn child_toward(&self, parent: Option<usize>, wanted: Option<usize>) -> Option<usize> {
        let mut current = wanted?;
        while self.flights[current].parent != parent {
            current = self.flights[current].parent?;
        }
        Some(current)
    }

    /// Advance toward `shot` at backdrop time `millis` and return the camera.
    /// A destination off the current chain first flies back down the chain
    /// (to the main vantage at most), then departs along the new one; a
    /// chained shot is reached through its parent's shot.
    pub(crate) fn drive(&mut self, shot: Shot, millis: u64) -> Sample {
        let wanted = self.flight_for(shot);
        if let Some(pictures) = &mut self.pictures {
            let view = wanted.map_or(pictures::View::Main, |_| pictures::View::Shot(shot));
            return pictures
                .drive(view, millis)
                .unwrap_or_else(|| self.main.sample(0.0));
        }
        if self.tour.is_some() {
            return self.drive_tour(wanted, millis);
        }
        let progress = self.progress.sample(millis);
        let parked_start = self.progress.target() == 0.0 && progress <= 1e-4;
        let parked_end = self.progress.target() == 1.0 && progress >= 1.0 - 1e-4;
        // Parked at a flight's start: the camera is on the parent's shot.
        if parked_start && let Some(active) = self.active {
            self.active = self.flights[active].parent;
            self.progress = Tween::settled(1.0);
        }
        let target = if self.active.is_none() && wanted.is_none() {
            0.0
        } else if self.active == wanted {
            1.0
        } else if self.descends_from(wanted, self.active) {
            // On the way out along the chain: depart the next flight once
            // the camera has reached this one's end.
            let next = self.child_toward(self.active, wanted);
            if self.active.is_none() || parked_end {
                self.active = next;
                self.progress = Tween::settled(0.0);
                1.0
            } else {
                1.0
            }
        } else {
            0.0
        };
        let millis_full = self.active.map_or(0, |index| self.flights[index].millis);
        if self.progress.target() != target {
            let remaining = (target - self.progress.sample(millis)).abs();
            let duration = (millis_full as f32 * remaining) as u32;
            self.progress
                .retarget(target, millis, duration, Easing::SmoothStep);
        }
        let progress = self.progress.sample(millis);
        self.last_progress = progress;
        let seconds = millis as f32 / 1_000.0;
        let Some(flight) = self.active.map(|index| &self.flights[index]) else {
            return self.main.sample(seconds);
        };
        let pose = flight.path.sample(progress);
        let start = flight
            .parent
            .map_or(self.main, |parent| self.flights[parent].path.destination());
        let departure = start.drift(seconds);
        let arrival = flight.path.destination().drift(seconds);
        let drift = departure.blend(arrival, progress);
        Sample {
            origin: pose.origin + drift.origin,
            yaw: pose.yaw + drift.yaw,
            pitch: pose.pitch + drift.pitch,
        }
    }

    /// Opacity for the screen that belongs to `shot`: fully visible while the
    /// camera is parked there or heading there, fading in as soon as the
    /// flight departs and gone while the camera flies the other way. A
    /// screen stays visible while a flight chained onto its shot returns to
    /// it (the saber flying back to the player screen).
    pub(crate) fn reveal(&self, shot: Shot) -> f32 {
        // On a toured map every screen is up at once; the world cuts behind it.
        // Pictures cut too, so their screens are up at once as well.
        if self.tour.is_some() || self.pictures.is_some() {
            return 1.0;
        }
        let wanted = self.flight_for(shot);
        let target = match wanted {
            Some(index) if self.active == Some(index) => 1.0,
            None if shot == Shot::Main => 0.0,
            // A shot without a route on this map sits over the main vantage.
            None if self.active.is_none() => return 1.0,
            _ => {
                let returning = self.progress.target() == 0.0
                    && self
                        .active
                        .is_some_and(|active| self.flights[active].parent == wanted);
                return if returning { 1.0 } else { 0.0 };
            }
        };
        if self.progress.target() != target {
            return 0.0;
        }
        if target == 0.0
            && self
                .active
                .is_some_and(|active| self.flights[active].parent.is_some())
        {
            // Still on a chained flight: the main menu waits for the last leg.
            return 0.0;
        }
        let toward = (1.0 - (target - self.last_progress).abs()).clamp(0.0, 1.0);
        if toward >= 1.0 - 1e-4 {
            return 1.0;
        }
        (toward / REVEAL_SPAN).clamp(0.0, 1.0)
    }

    /// Where the player's model stands on this map. The stage is part of the
    /// scenery from every shot (the ffa3 balcony is in view from the main
    /// vantage), not only while the camera is on the player route.
    pub(crate) fn stage(&self) -> Option<Stage> {
        self.flights.iter().find_map(|flight| flight.stage)
    }

    /// Where `shot` presents floating objects on this map, and the camera's
    /// right along which several of them line up.
    pub(crate) fn focus(&self, shot: Shot) -> Option<Focus> {
        let flight = self.flights.iter().find(|flight| flight.shot == shot)?;
        let yaw = flight.path.destination().yaw;
        Some(Focus {
            point: flight.focus?,
            right: Vec3::new(yaw.sin(), -yaw.cos(), 0.0),
        })
    }
}

/// A shot's presentation spot: the point it looks at and the horizontal
/// axis to the camera's right, so a row of objects lines up across the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Focus {
    pub(crate) point: Vec3,
    pub(crate) right: Vec3,
}

/// The map's worldspawn `message` (its display name), the key the authored
/// routes and props are filed under.
pub(crate) fn worldspawn_message(bsp: &Bsp) -> Option<String> {
    let entities = parse_entity_lump(bsp.entities()).ok()?;
    entities
        .iter()
        .find(|entity| entity.classname() == Some("worldspawn"))
        .and_then(|entity| entity.get("message"))
        .map(str::to_owned)
}

impl Vantage {
    /// A vantage at `origin`, turned `yaw` and `pitch` radians.
    pub(crate) fn new(origin: Vec3, yaw: f32, pitch: f32) -> Self {
        Self { origin, yaw, pitch }
    }

    /// The intermission view (looking at its `target` when it has one), else
    /// the same spawn the free camera starts from.
    pub(crate) fn from_bsp(bsp: &Bsp) -> Option<Self> {
        let entities = parse_entity_lump(bsp.entities()).ok()?;
        let intermission = entities
            .iter()
            .find(|entity| entity.classname() == Some("info_player_intermission"));
        if let Some(entity) = intermission
            && let Some(origin) = entity.vector("origin").ok().flatten()
        {
            let origin = Vec3::from_array(origin);
            let target = entity.get("target").and_then(|name| {
                entities
                    .iter()
                    .find(|candidate| candidate.get("targetname") == Some(name))
                    .and_then(|candidate| candidate.vector("origin").ok().flatten())
            });
            let (yaw, pitch) = match target {
                Some(target) => {
                    let direction = (Vec3::from_array(target) - origin).normalize_or(Vec3::X);
                    (direction.y.atan2(direction.x), direction.z.asin())
                }
                None => {
                    let angles = entity.vector("angles").ok().flatten().unwrap_or_default();
                    (angles[1].to_radians(), -angles[0].to_radians())
                }
            };
            return Some(Self { origin, yaw, pitch });
        }
        let (origin, yaw) = crate::assets::initial_camera(bsp).ok()?;
        Some(Self {
            origin: Vec3::from_array(origin),
            yaw,
            pitch: 0.0,
        })
    }

    /// The pitch, radians up.
    #[cfg(test)]
    pub(crate) fn pitch(self) -> f32 {
        self.pitch
    }

    /// Slow cinematic drift: a gentle yaw sweep, a slower pitch breath, and a
    /// few units of lateral float. Periods are long enough that no motion is
    /// perceptible frame to frame; only the parallax over seconds is.
    pub(crate) fn sample(self, seconds: f32) -> Sample {
        let drift = self.drift(seconds);
        Sample {
            origin: self.origin + drift.origin,
            yaw: self.yaw + drift.yaw,
            pitch: self.pitch + drift.pitch,
        }
    }

    /// The drift offset alone, so it can be blended across a flight.
    fn drift(self, seconds: f32) -> Sample {
        let right = Vec3::new(-self.yaw.sin(), self.yaw.cos(), 0.0);
        Sample {
            origin: right * 8.0 * (seconds * 0.09).sin()
                + Vec3::Z * 3.0 * (seconds * 0.13 + 0.7).sin(),
            yaw: 0.09 * (seconds * 0.11).sin(),
            pitch: 0.02 * (seconds * 0.07 + 1.3).sin(),
        }
    }
}

impl Sample {
    fn blend(self, other: Self, t: f32) -> Self {
        Self {
            origin: self.origin.lerp(other.origin, t),
            yaw: self.yaw + (other.yaw - self.yaw) * t,
            pitch: self.pitch + (other.pitch - self.pitch) * t,
        }
    }
}

/// True while a client menu is up outside any live or demo session, or
/// while a join is under way in the menu world, i.e. the world on screen is
/// only the menu backdrop and no HUD belongs over it.
pub(crate) fn standalone_menu_visible(gpu: &GpuState) -> bool {
    let sessions = gpu.live_session.is_some() || gpu.demo_session.is_some();
    gpu.client_menu
        .as_ref()
        .is_some_and(|menu| menu_holds_view(menu, sessions, gpu.is_menu_world))
}

/// Whether the classic menu style covers this frame with an opaque screen,
/// so the world is not drawn: its main pages and the screens they open
/// outside a match, and the connect and loading screens (joins and server
/// map changes alike). Over a live match, the in-game menu and the screens
/// it opens leave the world visible, as retail's do. The SJK UI's main page
/// sits over the live map; the classic screens it opens cover it. Its
/// loading screen keeps the menu map under it until the destination's
/// levelshot covers it, and leaves out a server's world.
pub(crate) fn classic_hides_world(gpu: &GpuState) -> bool {
    // The Holocrons tab floats a 3D holocron in the world, which the page leaves clear
    // on the left, in every menu style.
    if holocrons_shown(gpu) {
        return false;
    }
    gpu.client_menu.as_ref().is_some_and(|menu| {
        if menu.sjk_loading_on_show() {
            return menu.sjk_loading_hides_world(gpu.is_menu_world);
        }
        !menu.sjk_screen()
            && menu.is_visible()
            && (standalone_menu_visible(gpu) || menu.is_loading_screen())
    })
}

/// No HUD belongs over the menu backdrop, nor under the loading card of a
/// map restart or map change on a server world.
pub(crate) fn hides_hud(gpu: &GpuState) -> bool {
    standalone_menu_visible(gpu)
        || gpu
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_connecting)
}

/// Whether `menu` owns the camera given that a live or demo session exists
/// (`sessions`) and whether the world on screen is the menu map
/// (`menu_world`). A join's session appears while the menu still reads
/// `Connecting` — the map load names the map a frame later — so the whole
/// connect phase counts, not only the map load: for that one frame the
/// live camera would otherwise render the joined player's position inside
/// the menu map. Only the menu world has the backdrop's camera routes: a
/// map restart or map change reloads behind the loading card on a server
/// world, whose game camera keeps the view — the backdrop's gate shot would
/// show that map from the menu's vantage until the reload lands.
pub(crate) fn menu_holds_view(
    menu: &crate::menu::ClientMenu,
    sessions: bool,
    menu_world: bool,
) -> bool {
    menu.is_visible() && (!sessions || (menu_world && menu.is_connecting()))
}

/// Whether the Profile screen's Holocrons page is open.
fn holocrons_shown(gpu: &GpuState) -> bool {
    gpu.console
        .as_ref()
        .is_some_and(|console| console.holocrons_stage().is_some())
}

/// Fly or park the free camera on the current screen's shot while a
/// standalone menu is up.
pub(crate) fn drive(gpu: &mut GpuState, now: Instant) {
    if !standalone_menu_visible(gpu) || gpu.resident.map_change_pending {
        return;
    }
    let millis = now.duration_since(gpu.ui_epoch).as_millis() as u64;
    let holocrons = holocrons_shown(gpu);
    if let Some(menu) = gpu.client_menu.as_mut() {
        menu.set_holocron_shot(holocrons);
    }
    let Some(sample) = gpu
        .client_menu
        .as_mut()
        .and_then(|menu| menu.drive_backdrop(millis))
    else {
        return;
    };
    // The Saber Lab's camera goes round its model, or behind it as a match's third person
    // (`cg_thirdPersonRange`, `cg_thirdPersonVertOffset`, `cg_thirdPersonAngle`), short
    // of any wall in the way.
    let sample = match gpu.menu_stage.lab_camera() {
        Some(camera) => {
            let cvar = |name: &str, fallback: f32| {
                gpu.console
                    .as_ref()
                    .and_then(|console| console.float_cvar(name))
                    .map_or(fallback, |value| value as f32)
            };
            let chase = behind(
                camera.eye,
                camera.facing + cvar("cg_thirdPersonAngle", 0.0).to_radians(),
                cvar("cg_thirdPersonRange", 80.0),
                cvar("cg_thirdPersonVertOffset", 16.0),
            );
            let (base, target) = (
                blend(sample, chase.0, camera.chase),
                camera.target.lerp(chase.1, camera.chase),
            );
            let mut sample = orbit(base, target, camera.view, camera.share);
            let trace = gpu.bsp.trace_box(
                target.to_array(),
                sample.origin.to_array(),
                sjk_bsp::Aabb::POINT,
                CONTENTS_SOLID,
            );
            if !trace.start_solid && trace.fraction < 1.0 {
                let way = sample.origin - target;
                let reach = (way.length() * trace.fraction - WALL_CLEARANCE).max(0.0);
                sample.origin = target + way.normalize_or_zero() * reach;
            }
            // The builder's panels take the left: the camera turns until the model stands
            // on the right of the picture.
            if camera.build > 0.0 {
                let size = [
                    gpu.configuration.width as f32,
                    gpu.configuration.height as f32,
                ];
                let aspect = size[0] / size[1].max(1.0);
                let fov = crate::menu_stage::frame_fit_fov(gpu.field_of_view, aspect);
                let framed = framed_yaw(sample, target, BUILD_FRAME_X, fov, aspect);
                let turn = (framed - sample.yaw + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                sample.yaw += turn * camera.build;
            }
            sample
        }
        None => sample,
    };
    gpu.camera_position = sample.origin;
    gpu.camera_yaw = sample.yaw;
    gpu.camera_pitch = sample.pitch;
}

/// A match's third-person camera behind a player whose eye is at `eye`, facing `facing`
/// (radians), looking level (`CG_OffsetThirdPersonView`): `range` behind the point
/// `height` over the eye, which it looks at; and that point.
fn behind(eye: Vec3, facing: f32, range: f32, height: f32) -> (Sample, Vec3) {
    let target = eye + Vec3::Z * height;
    let forward = Vec3::new(facing.cos(), facing.sin(), 0.0);
    (
        Sample {
            origin: target - forward * range,
            yaw: facing,
            pitch: 0.0,
        },
        target,
    )
}

/// `share` of the way from `from` to `to`, turning the short way round.
fn blend(from: Sample, to: Sample, share: f32) -> Sample {
    let turn = (to.yaw - from.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    Sample {
        origin: from.origin.lerp(to.origin, share),
        yaw: from.yaw + turn * share,
        pitch: from.pitch + (to.pitch - from.pitch) * share,
    }
}

/// Q3 `CONTENTS_SOLID`: what the Saber Lab's camera stays short of.
const CONTENTS_SOLID: u32 = 1;
/// How far short of a wall it stays.
const WALL_CLEARANCE: f32 = 6.0;

/// Where across the Collection's frame (a share of its width) the Saber Lab's builder
/// shows its model: the middle of the room right of its panels.
const BUILD_FRAME_X: f32 = 0.78;

/// The yaw (radians) that shows `target`, seen from `sample`'s place, at `frame_x` of the
/// Collection's frame across, the view `fov` degrees high in a window `aspect` wide: the
/// frame is 16:9 at most, centred in a wider window.
pub(crate) fn framed_yaw(sample: Sample, target: Vec3, frame_x: f32, fov: f32, aspect: f32) -> f32 {
    let way = target - sample.origin;
    let toward = way.y.atan2(way.x);
    let frame_share = (16.0 / 9.0 / aspect.max(0.01)).min(1.0);
    let window_x = 0.5 + (frame_x - 0.5) * frame_share;
    let half_width = (fov.to_radians() * 0.5).tan() * aspect;
    // Turned left of the target by its angle off the middle, the target shows right of it.
    toward + ((2.0 * window_x - 1.0) * half_width).atan()
}

/// The Saber Lab's camera (`crate::console::collection_panel::LabView`): `sample`'s
/// turned round `target` by the view's turn, raised over it by its tilt and brought
/// nearer or taken back by its distance, `share` of the way. The camera's own turn and
/// tilt go with it, so `target` stays where the shot frames it (the model left of the
/// tab's choices).
pub(crate) fn orbit(
    sample: Sample,
    target: Vec3,
    view: crate::console::collection_panel::LabView,
    share: f32,
) -> Sample {
    let share = share.clamp(0.0, 1.0);
    // The short way round, so that the way back is too.
    let turn = if view.turn > std::f32::consts::PI {
        view.turn - std::f32::consts::TAU
    } else {
        view.turn
    } * share;
    let tilt = view.tilt * share;
    let distance = 1.0 + (view.distance - 1.0) * share;
    let yaw = sample.yaw + turn;
    // Raised over the target, the camera looks down on it as far: the whole view turns
    // about the camera's right.
    let right = Vec3::new(yaw.sin(), -yaw.cos(), 0.0);
    let rotation = glam::Quat::from_axis_angle(right, -tilt) * glam::Quat::from_rotation_z(turn);
    Sample {
        origin: target + rotation * (sample.origin - target) * distance,
        yaw,
        pitch: sample.pitch - tilt,
    }
}

/// Legacy `hud.wgsl` backdrop selector: 4 = console, 3 = a menu screen that
/// still relies on the shader-drawn backdrop, 0 = none.
pub(crate) fn shader_menu_state(gpu: &GpuState) -> f32 {
    if gpu
        .console
        .as_ref()
        .is_some_and(|console| console.is_open())
    {
        4.0
    } else if gpu
        .client_menu
        .as_ref()
        .is_some_and(|menu| menu.is_visible() && menu.draw_list().is_none())
    {
        3.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod orbit_tests {
    use super::*;
    use crate::console::collection_panel::LabView;

    /// Where `point` lies from a camera at `sample`: ahead, to the right and above.
    fn seen(sample: Sample, point: Vec3) -> Vec3 {
        let forward = Vec3::new(
            sample.yaw.cos() * sample.pitch.cos(),
            sample.yaw.sin() * sample.pitch.cos(),
            sample.pitch.sin(),
        );
        let right = Vec3::new(sample.yaw.sin(), -sample.yaw.cos(), 0.0);
        let up = right.cross(forward);
        let way = point - sample.origin;
        Vec3::new(way.dot(forward), way.dot(right), way.dot(up))
    }

    /// However the camera goes round, raised or brought nearer, the model stays where the
    /// shot frames it, only nearer or farther; raised, the camera is higher.
    #[test]
    fn the_model_stays_where_the_shot_frames_it() {
        let target = Vec3::new(-360.0, -360.0, 384.0);
        let shot = Sample {
            origin: Vec3::new(-428.0, -456.0, 398.0),
            yaw: 20_f32.to_radians(),
            pitch: -4_f32.to_radians(),
        };
        let before = seen(shot, target);
        for view in [
            LabView {
                turn: 1.0,
                tilt: 0.0,
                distance: 1.0,
            },
            LabView {
                turn: 4.0,
                tilt: 0.8,
                distance: 1.0,
            },
            LabView {
                turn: 2.5,
                tilt: -0.2,
                distance: 0.5,
            },
        ] {
            for share in [0.3, 1.0] {
                let moved = orbit(shot, target, view, share);
                let distance = 1.0 + (view.distance - 1.0) * share;
                let after = seen(moved, target);
                assert!(
                    (after - before * distance).length() < 1e-2,
                    "{view:?} {after} {before}"
                );
                if view.tilt > 0.0 {
                    assert!(moved.origin.z > shot.origin.z);
                }
            }
        }
        assert_eq!(orbit(shot, target, LabView::default(), 1.0), shot);
    }
}
