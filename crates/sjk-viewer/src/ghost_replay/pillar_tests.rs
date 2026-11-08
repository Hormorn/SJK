//! The real `Pmove` on a synthetic course: a start platform, pillars over a pit and an
//! end platform. A scripted run jumps from pillar to pillar and is recorded; replays
//! must land on every pillar.

use super::*;
use sjk_client::pmove::{MovementCollision, MovementConfig, MovementTrace, Predictor};

/// Pillars' centres along +x and their half width; tops at z 0, the pit far below.
const PILLARS: [f32; 4] = [160.0, 310.0, 460.0, 610.0];
const HALF: f32 = 32.0;
const PIT: f32 = -40.0;

struct Course {
    boxes: Vec<([f32; 3], [f32; 3])>,
}

impl Course {
    fn new() -> Self {
        let mut boxes = vec![
            ([-300.0, -200.0, -64.0], [24.0, 200.0, 0.0]),
            ([676.0, -200.0, -64.0], [1100.0, 200.0, 0.0]),
            ([-2000.0, -2000.0, -600.0], [2000.0, 2000.0, -512.0]),
        ];
        for centre in PILLARS {
            boxes.push(([centre - HALF, -HALF, -512.0], [centre + HALF, HALF, 0.0]));
        }
        Self { boxes }
    }
}

impl MovementCollision for Course {
    /// A swept box against the course's boxes (each grown by the player's box).
    fn trace(
        &self,
        start: [f32; 3],
        mins: [f32; 3],
        maxs: [f32; 3],
        end: [f32; 3],
        _: u32,
    ) -> MovementTrace {
        let mut best = MovementTrace::miss(end);
        let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
        for (low, high) in &self.boxes {
            let low = [low[0] - maxs[0], low[1] - maxs[1], low[2] - maxs[2]];
            let high = [high[0] - mins[0], high[1] - mins[1], high[2] - mins[2]];
            let inside = (0..3).all(|i| start[i] > low[i] && start[i] < high[i]);
            if inside {
                return MovementTrace {
                    fraction: 0.0,
                    end_position: start,
                    start_solid: true,
                    all_solid: true,
                    entity_number: 1022,
                    ..MovementTrace::miss(start)
                };
            }
            let (mut enter, mut exit, mut normal) = (-1.0_f32, 1.0_f32, [0.0; 3]);
            let mut hit = true;
            for i in 0..3 {
                if delta[i].abs() < 1e-6 {
                    if start[i] < low[i] || start[i] > high[i] {
                        hit = false;
                        break;
                    }
                    continue;
                }
                let (near, far, side) = if delta[i] > 0.0 {
                    (low[i], high[i], -1.0)
                } else {
                    (high[i], low[i], 1.0)
                };
                let t_near = (near - start[i]) / delta[i];
                let t_far = (far - start[i]) / delta[i];
                if t_near > enter {
                    enter = t_near;
                    normal = [0.0; 3];
                    normal[i] = side;
                }
                exit = exit.min(t_far);
            }
            if !hit || enter > exit || enter < 0.0 || enter >= best.fraction {
                continue;
            }
            // Stop a little short of the surface, as the engine's traces do.
            let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
            let fraction = (enter - 0.03125 / length.max(1e-6)).max(0.0);
            best = MovementTrace {
                fraction,
                end_position: std::array::from_fn(|i| start[i] + delta[i] * fraction),
                plane_normal: normal,
                entity_number: 1022,
                ..MovementTrace::miss(end)
            };
        }
        best
    }
}

/// A player standing at `origin` whose last command ran at `command_time`.
fn runner(origin: [f32; 3], command_time: i32) -> Predictor {
    let mut player = PlayerState::default();
    player.set_command_time(command_time);
    player.stats[0] = 100;
    player.set_origin(origin);
    player.set_gravity(800);
    player.set_speed(250.0);
    player.set_base_speed(250);
    player.set_ground_entity_num(1022);
    Predictor::from_player_state(&player, MovementConfig::default())
}

/// Commands at 125 a second for `seconds`, from `first` stamp.
fn stamps(first: i32, count: usize) -> impl Iterator<Item = i32> {
    (0..count as i32).map(move |i| first + i * 8)
}

/// The stamps the live client makes at `fps` frames a second on a server that caps
/// commands (JoF's): at most one a frame on an accumulator a step apart, stamped with
/// the clock as the previous frame read it (`Pacing::OnePerFrame`,
/// `CommandSchedule::stock_stamps`).
fn one_per_frame_stamps(first: i32, fps: f64, count: usize) -> Vec<i32> {
    use sjk_client::command_rate::{COMMAND_MILLIS, CommandSchedule, Pacing};
    let mut schedule = CommandSchedule::default();
    let now = std::time::Instant::now();
    let mut stamps = Vec::with_capacity(count);
    let mut previous = first;
    let mut frame = 0_u32;
    while stamps.len() < count {
        let reading = first + (f64::from(frame) * 1000.0 / fps) as i32;
        let due = schedule.due(reading, true, now, COMMAND_MILLIS, Pacing::OnePerFrame);
        let due = schedule.stock_stamps(due, Pacing::OnePerFrame, COMMAND_MILLIS, previous, true);
        stamps.extend(
            due.as_slice()
                .iter()
                .copied()
                .filter(|&stamp| stamp > first - 8),
        );
        previous = reading;
        frame += 1;
    }
    stamps.truncate(count);
    stamps
}

/// The scripted perfect run: forward, and a jump (held 80 ms) whenever past a takeoff point
/// (a stock jump at full speed carries 148 units, so each lands near the next top's
/// centre and takes off again at once).
fn record_run(course: &Course) -> Run {
    record_run_at(course, stamps(8, 500), 0)
}

/// [`record_run`] with the given command stamps, standing still for the first `still`
/// commands (as a real recording begins).
fn record_run_at(
    course: &Course,
    command_stamps: impl IntoIterator<Item = i32>,
    still: usize,
) -> Run {
    let mut ghost = Ghost {
        mode: Mode::Recording(Recording {
            name: "pillars".into(),
            run: Run {
                map: "test".into(),
                ..Run::default()
            },
            first_stamp: None,
        }),
        ..Ghost::default()
    };
    let mut server = runner([-100.0, 0.0, 24.0], 0);
    let takeoffs = [8.0, 156.0, 306.0, 456.0, 606.0];
    // A player holds jump for a while; Pmove jumps once per press (`PMF_JUMP_HELD`).
    let mut held = 0;
    for (index, stamp) in command_stamps.into_iter().enumerate() {
        let pose = Pose::of(server.state());
        let x = pose.origin[0];
        let want_jump = pose.on_ground
            && takeoffs
                .iter()
                .any(|&takeoff| x > takeoff && x < takeoff + 40.0);
        if want_jump && held == 0 {
            held = 10;
        }
        let up = if held > 0 { 127 } else { 0 };
        held = (held - 1).max(0);
        let command = UserCommand {
            server_time: stamp,
            angles: std::array::from_fn(|i| 0_i32.wrapping_sub(pose.delta_angles[i])),
            forward_move: if index >= still && x < 760.0 { 127 } else { 0 },
            up_move: up,
            ..UserCommand::default()
        };
        server.predict_command(command, course);
        ghost.record(&command, &pose, Some(server.state()));
        assert!(server.state().origin[2] > PIT, "the script fell at x {x}");
    }
    let Mode::Recording(recording) = ghost.mode else {
        unreachable!()
    };
    let end = recording.run.frames.last().unwrap().origin;
    assert!(end[0] > 680.0, "the script never reached the end: {end:?}");
    recording.run
}

/// Replay `run` from `origin` on a server that loses the commands in `lost` (the
/// next command covers their time, as `ClientThink_real` does); returns the final
/// origin and the messages.
fn replay(course: &Course, run: &Run, origin: [f32; 3], lost: &[usize]) -> ([f32; 3], Vec<String>) {
    replay_bumped(course, run, origin, lost, None)
}

/// [`replay`] where, at command `bump.0`, something shoves the player sideways by
/// `bump.1` units a second (another player, a push).
fn replay_bumped(
    course: &Course,
    run: &Run,
    origin: [f32; 3],
    lost: &[usize],
    bump: Option<(usize, f32)>,
) -> ([f32; 3], Vec<String>) {
    replay_at(course, run, origin, lost, bump, stamps(50_000, 3_000))
}

/// [`replay_bumped`] with the live client's command stamps.
fn replay_at(
    course: &Course,
    run: &Run,
    origin: [f32; 3],
    lost: &[usize],
    bump: Option<(usize, f32)>,
    live_stamps: impl IntoIterator<Item = i32>,
) -> ([f32; 3], Vec<String>) {
    let mut ghost = Ghost {
        mode: Mode::Approaching(Box::new(Approach {
            playback: Playback {
                name: "pillars".into(),
                run: run.clone(),
                first_stamp: None,
                warp: 0,
                cursor: 0,
                consumed: None,
                sent: None,
                commands_since_warp: 0,
                corrections: 0,
                worst_drift: 0.0,
            },
            first_stamp: None,
            settled: 0,
        })),
        ..Ghost::default()
    };
    let mut server = runner(origin, 50_000 - 8);
    let mut client = server.clone();
    let mut lowest = f32::INFINITY;
    let mut index = 0;
    // One live command a frame, as `Pacing::OnePerFrame` makes them; the replay
    // stamps its own (`Ghost::stamps`).
    'frames: for live in live_stamps {
        for &stamp in ghost.stamps(DueCommands::from_stamps([live])).as_slice() {
            // The client predicts every command; the server misses the lost ones, and
            // the client learns of it at once (a snapshot with no ping, the worst case).
            let pose = Pose::of(client.state());
            let Some(drive) = ghost.drive(
                &UserCommand {
                    server_time: stamp,
                    ..UserCommand::default()
                },
                &pose,
            ) else {
                break 'frames;
            };
            if !lost.contains(&index) {
                server.predict_command(drive.command, course);
            }
            if let Some((at, sideways)) = bump
                && at == index
            {
                server.state_mut().velocity[1] += sideways;
            }
            client = server.clone();
            if matches!(ghost.mode, Mode::Playing(_)) {
                lowest = lowest.min(server.state().origin[2]);
            }
            index += 1;
        }
    }
    let mut messages = ghost.take_messages();
    if lowest <= PIT {
        messages.push(format!("fell into the pit: lowest z {lowest}"));
    }
    (server.state().origin, messages)
}

/// How many commands the walk to the start sends from `origin` before the run's first.
fn walk_commands(course: &Course, run: &Run, origin: [f32; 3]) -> usize {
    let mut ghost = Ghost {
        mode: Mode::Approaching(Box::new(Approach {
            playback: Playback {
                name: "pillars".into(),
                run: run.clone(),
                first_stamp: None,
                warp: 0,
                cursor: 0,
                consumed: None,
                sent: None,
                commands_since_warp: 0,
                corrections: 0,
                worst_drift: 0.0,
            },
            first_stamp: None,
            settled: 0,
        })),
        ..Ghost::default()
    };
    let mut server = runner(origin, 50_000 - 8);
    for (count, stamp) in stamps(50_000, 3_000).enumerate() {
        let drive = ghost
            .drive(
                &UserCommand {
                    server_time: stamp,
                    ..UserCommand::default()
                },
                &Pose::of(server.state()),
            )
            .expect("walking");
        if matches!(ghost.mode, Mode::Playing(_)) {
            return count;
        }
        server.predict_command(drive.command, course);
    }
    panic!("never reached the start");
}

#[test]
fn a_recorded_pillar_run_replays_exactly_from_its_start() {
    let course = Course::new();
    let run = record_run(&course);
    let (end, messages) = replay(&course, &run, run.start.origin, &[]);
    let target = run.frames.last().unwrap().origin;
    assert!(
        steer::horizontal([target[0] - end[0], target[1] - end[1], 0.0]) < 0.01,
        "{end:?} vs {target:?}: {messages:?}"
    );
    assert!(
        messages.last().unwrap().contains("played to the end"),
        "{messages:?}"
    );
}

#[test]
fn the_replay_walks_to_the_start_and_lands_every_pillar_despite_lost_commands() {
    let course = Course::new();
    let run = record_run(&course);
    // Thirty units off the start; the server loses the command of the second takeoff
    // (the jump leaves 8 ms late) and one in the air after the third.
    let start = run.start.origin;
    let takeoffs: Vec<usize> = run
        .frames
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0].up == 0 && pair[1].up > 0)
        .map(|(index, _)| index + 1)
        .collect();
    assert_eq!(takeoffs.len(), 5, "five jumps");
    // The walk to the start takes some commands first: find how many.
    let walk = walk_commands(&course, &run, [start[0] - 20.0, start[1] + 22.0, start[2]]);
    let (end, messages) = replay(
        &course,
        &run,
        [start[0] - 20.0, start[1] + 22.0, start[2]],
        &[walk + takeoffs[1], walk + takeoffs[2] + 20],
    );
    let target = run.frames.last().unwrap().origin;
    assert!(
        steer::horizontal([target[0] - end[0], target[1] - end[1], 0.0]) < 8.0,
        "{end:?} vs {target:?}: {messages:?}"
    );
    assert!(
        messages.last().unwrap().contains("played to the end"),
        "{messages:?}"
    );
}

#[test]
fn a_sideways_shove_on_the_start_platform_is_steered_out_before_the_pillars() {
    let course = Course::new();
    let run = record_run(&course);
    let start = run.start.origin;
    let walk = walk_commands(&course, &run, start);
    // A shove of 60 units a second to the left a quarter of a second into the run,
    // while still running on the start platform. Unsteered, the run ends 46 units off
    // the line (measured), on the very edge of the last top; steered, the sideways
    // speed is gone before the first jump and only a small offset stays.
    let (end, messages) = replay_bumped(&course, &run, start, &[], Some((walk + 30, 60.0)));
    let target = run.frames.last().unwrap().origin;
    let off = steer::horizontal([target[0] - end[0], target[1] - end[1], 0.0]);
    assert!(off < 8.0, "{end:?} vs {target:?}: {messages:?}");
    assert!(
        messages.last().unwrap().contains("played to the end"),
        "{messages:?}"
    );
}

#[test]
fn a_run_cut_at_its_first_move_by_the_editor_replays_like_the_run_alone() {
    let course = Course::new();
    let run = record_run(&course);
    // The same run behind a second of standing with the console open (`ghost record`
    // typed), as a real recording begins.
    let mut idle = run.clone();
    let still = Frame {
        time: 0,
        view: run.start.view,
        forward: 0,
        right: 0,
        up: 0,
        buttons: sjk_game_jka::pmove_talk::BUTTON_TALK,
        weapon: run.start.weapon,
        force_selection: run.frames[0].force_selection,
        inventory_selection: run.frames[0].inventory_selection,
        generic_command: 0,
        origin: run.start.origin,
        velocity: run.start.velocity,
        ground: true,
        confirmed: true,
    };
    idle.frames = (0..125)
        .map(|step| Frame {
            time: step * 8,
            ..still
        })
        .chain(run.frames.iter().map(|frame| Frame {
            time: frame.time + 1_000,
            ..*frame
        }))
        .collect();
    let cut = edit::Cut::new(idle);
    let first = cut.first_move(0).expect("the run moves");
    assert_eq!(first, 125);
    let edited = cut.edited(first, cut.len() - 1, true);
    assert_eq!(edited.start, run.start);
    assert_eq!(edited.frames, run.frames);
    let (end, messages) = replay(&course, &edited, edited.start.origin, &[]);
    let target = run.frames.last().unwrap().origin;
    assert!(
        steer::horizontal([target[0] - end[0], target[1] - end[1], 0.0]) < 0.01,
        "{end:?} vs {target:?}: {messages:?}"
    );
}

/// The commands a replay sends are spaced as recorded, not as the live clock makes
/// them: on JoF's servers it makes one a frame (`Pacing::OnePerFrame`, since
/// 02/11/2026), so their spacing follows the frame rate, while the runs recorded before
/// that are on the 8 ms grid. Sent on the live stamps, a run took other durations
/// (0.4 to 3 units off over these four seconds), and the drift measured between two
/// different times made up to nine needless corrections.
#[test]
fn a_run_replays_exactly_whatever_the_frame_rate_it_was_recorded_and_replayed_at() {
    let course = Course::new();
    let recordings = [
        ("on the 8 ms grid", stamps(8, 520).collect::<Vec<_>>()),
        (
            "one a frame at 300 FPS",
            one_per_frame_stamps(8, 300.0, 520),
        ),
        (
            "one a frame at 144 FPS",
            one_per_frame_stamps(8, 144.0, 520),
        ),
    ];
    for (recorded, command_stamps) in recordings {
        let run = record_run_at(&course, command_stamps, 20);
        let target = run.frames.last().unwrap().origin;
        for fps in [350.0, 237.0, 144.0, 60.0] {
            let live = one_per_frame_stamps(50_000, fps, 3_000);
            let (end, messages) = replay_at(&course, &run, run.start.origin, &[], None, live);
            let off = steer::horizontal([target[0] - end[0], target[1] - end[1], 0.0]);
            assert!(
                off < 0.01,
                "recorded {recorded}, replayed at {fps} FPS: {off} units off: {messages:?}"
            );
            assert!(
                messages
                    .last()
                    .unwrap()
                    .contains("to the end (0 corrections"),
                "recorded {recorded}, replayed at {fps} FPS: {messages:?}"
            );
        }
        // Under 60 FPS the recorded times trail the clock too far: the commands passed
        // over merge into the live one, and the run still lands every pillar.
        let live = one_per_frame_stamps(50_000, 30.0, 3_000);
        let (_, messages) = replay_at(&course, &run, run.start.origin, &[], None, live);
        assert!(
            messages.last().unwrap().contains("played to the end"),
            "recorded {recorded}, replayed at 30 FPS: {messages:?}"
        );
    }
}
