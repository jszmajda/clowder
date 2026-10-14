//! The session's loop with its terminal taken away — `runtime::session`,
//! driven by a scripted input source and a screen that can refuse writes.
//!
//! @spec TUI-RUN-046, TUI-RUN-047, TUI-RUN-048, TUI-RUN-050, TUI-RUN-051,
//! TUI-RUN-057, TUI-RUN-059
//!
//! **The seam.** `terminal_lost.rs` runs the built game on a real
//! pseudo-terminal and asserts how a lost terminal *ends*: saved, unlocked,
//! gone with a quit's status. What the loop does on the way there — that it
//! asks for no further input, applies none of the events the frame had taken,
//! draws no further frame, keeps what a refused frame had already done, and
//! saves once — cannot be seen from outside a process. Those are asserted
//! here, against the loop itself with both of its questions answered by the
//! test: its input by a scripted source, its frames by a backend that records
//! every write and refuses them on cue.
//!
//! The surface these tests address, which the code stage adds to
//! `clowder_tui::runtime`:
//!
//! ```text
//! /// Where the loop takes its input from. In the game: the terminal
//! /// library's `event::poll` and `event::read`, and nothing else.
//! pub trait InputSource {
//!     /// Wait up to `timeout` for an event; `Ok(true)` when one is ready.
//!     fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
//!     /// Take the event a wait said was ready.
//!     fn read(&mut self) -> io::Result<crossterm::event::Event>;
//! }
//!
//! /// What startup found (today a private enum of the same shape).
//! pub enum Startup { NewGame(GameState), Loaded(Box<CatchUp>) }
//!
//! /// Everything between taking the terminal and giving it back: the wake
//! /// where the load owes one, then play, until a quit key, a recorded
//! /// termination (`terminated()` turning true), or a lost terminal — and
//! /// then the one shutdown save. Returns the parting lines, to be printed
//! /// once the player's own terminal is back.
//! pub fn session<B: Backend>(
//!     startup: Startup,
//!     save_path: PathBuf,
//!     terminal: &mut Terminal<B>,
//!     input: &mut impl InputSource,
//!     terminated: &dyn Fn() -> bool,
//! ) -> Vec<String>;
//! ```
//!
//! `session` reads the real clocks, as `run` does today, and replays a wake
//! at `WAKE_TICKS_PER_FRAME`; it takes no lock and touches no terminal but
//! the one it is handed. `run` keeps the lock, the load, the terminal guard,
//! the signal handlers, and the printing of what `session` returns — the
//! parts `terminal_lost.rs` and `loop_order.rs` observe from outside.
//!
//! **The scripted source is a faithful one.** It reads nothing when asked to
//! wait no time at all, as the input source the library is built with reads
//! nothing (TUI-RUN-059); it answers one cue per wait otherwise. A cue can
//! also record a termination signal or take the screen away as it answers, so
//! a test can say exactly which wait something happened during.
//!
//! **No test here can hang.** A loop that ignores its reason to stop would
//! otherwise run for ever; the source panics once the loop has asked a dozen
//! times past the end of its script, and says so.
//!
//! **Time is real here.** `session` reads the clocks itself, so how many live
//! ticks a scripted session runs is not chosen by the test. What a test needs
//! exactly is therefore read off things time does not move: the speed a key
//! set, the count of ticks a wake replayed, how many times the source was
//! asked. The one claim that needs an exact interval — that what a refused
//! frame ticked stands — is made at `App::step`, which takes its clock
//! reading as an argument.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use clowder_core::{begin_load, save_string, Advance, GameState, SetDown};
use clowder_tui::nav::{self, ViewState};
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{session, App, InputSource, Startup, TickAccumulator, WAKE_TICKS_PER_FRAME};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::{Backend, ClearType, TestBackend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// The world the loop runs in: one log, shared by its input and its screen
// ---------------------------------------------------------------------------

/// What the loop did, in the order it did it.
#[derive(Clone, Debug, PartialEq)]
enum Op {
    /// Asked the input source to wait this long.
    Wait(Duration),
    /// Took an event from the input source.
    Read,
    /// Wrote to the screen; `refused` when the screen was gone.
    Write { refused: bool },
}

/// How the source answers one wait.
#[derive(Clone, Debug)]
enum Answer {
    /// Nothing ready.
    Quiet,
    /// An event is ready, and the read that follows hands it over.
    Ready(Event),
    /// The wait itself fails.
    WaitFails(io::ErrorKind),
    /// An event is said to be ready, and the read of it fails.
    ReadFails(io::ErrorKind),
}

/// One wait's answer, and what else happens in the world during that wait.
#[derive(Clone, Debug)]
struct Cue {
    answer: Answer,
    signal: bool,
    screen_goes: bool,
}

fn quiet() -> Cue {
    Cue { answer: Answer::Quiet, signal: false, screen_goes: false }
}

fn ready(event: Event) -> Cue {
    Cue { answer: Answer::Ready(event), ..quiet() }
}

fn wait_fails(kind: io::ErrorKind) -> Cue {
    Cue { answer: Answer::WaitFails(kind), ..quiet() }
}

fn read_fails(kind: io::ErrorKind) -> Cue {
    Cue { answer: Answer::ReadFails(kind), ..quiet() }
}

impl Cue {
    /// A termination signal is recorded during this wait.
    fn and_a_signal(mut self) -> Cue {
        self.signal = true;
        self
    }

    /// The screen is gone from this wait on: every later write is refused.
    fn and_the_screen_goes(mut self) -> Cue {
        self.screen_goes = true;
        self
    }
}

/// How far past its reason to stop a loop may go before the source refuses to
/// play along. Small: while waking, every further wait costs a frame's replay.
const RUNAWAY: usize = 12;

#[derive(Default)]
struct World {
    log: Vec<Op>,
    script: VecDeque<Cue>,
    /// What the next read hands over.
    pending: Option<io::Result<Event>>,
    signalled: bool,
    screen_gone: bool,
    /// Where in the log a signal was recorded.
    signalled_at: Option<usize>,
    waits_of_no_time: usize,
    waits_past_the_script: usize,
}

struct Scripted(Rc<RefCell<World>>);

impl InputSource for Scripted {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        let mut w = self.0.borrow_mut();
        w.log.push(Op::Wait(timeout));
        if timeout.is_zero() {
            // The source the library is built with reads nothing in a wait of
            // no time: not what is ready, not what it has already decoded.
            w.waits_of_no_time += 1;
            assert!(
                w.waits_of_no_time <= RUNAWAY,
                "the loop keeps asking its input source to wait no time at all, and the source \
                 reads nothing in such a wait — so nothing that is ready is ever taken"
            );
            return Ok(false);
        }
        if w.pending.is_some() {
            // An event a wait announced and the loop has not yet taken is
            // still ready.
            return Ok(true);
        }
        let Some(cue) = w.script.pop_front() else {
            w.waits_past_the_script += 1;
            assert!(
                w.waits_past_the_script <= RUNAWAY,
                "the loop is still asking for input {RUNAWAY} waits after it was given its reason \
                 to stop"
            );
            return Ok(false);
        };
        if cue.signal {
            w.signalled = true;
            w.signalled_at = Some(w.log.len() - 1);
        }
        if cue.screen_goes {
            w.screen_gone = true;
        }
        match cue.answer {
            Answer::Quiet => Ok(false),
            Answer::Ready(event) => {
                w.pending = Some(Ok(event));
                Ok(true)
            }
            Answer::WaitFails(kind) => Err(kind.into()),
            Answer::ReadFails(kind) => {
                w.pending = Some(Err(kind.into()));
                Ok(true)
            }
        }
    }

    fn read(&mut self) -> io::Result<Event> {
        let mut w = self.0.borrow_mut();
        w.log.push(Op::Read);
        w.pending
            .take()
            .expect("the loop read an event no wait had said was ready — the real source would block here")
    }
}

/// A `TestBackend` that logs every write and refuses them once the screen is
/// gone, as a vanished terminal refuses them. Its size goes on answering, as
/// a vanished terminal's does.
struct Screen {
    inner: TestBackend,
    world: Rc<RefCell<World>>,
}

impl Screen {
    fn write(&self) -> io::Result<()> {
        let mut w = self.world.borrow_mut();
        let refused = w.screen_gone;
        w.log.push(Op::Write { refused });
        if refused {
            // EIO, which is what a write to a vanished terminal reports.
            Err(io::Error::from_raw_os_error(5))
        } else {
            Ok(())
        }
    }
}

impl Backend for Screen {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.write()?;
        self.inner.draw(content)
    }

    fn append_lines(&mut self, n: u16) -> io::Result<()> {
        self.write()?;
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.write()?;
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.write()?;
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.write()?;
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.write()?;
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.write()?;
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.write()?;
        self.inner.flush()
    }
}

// ---------------------------------------------------------------------------
// Running a session, and reading what it left
// ---------------------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("clowder-session-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// A save path no write can land on: a directory with something in it stands
/// in the save's place.
fn unwritable(name: &str) -> PathBuf {
    let path = scratch(name);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("in-the-way"), b"").unwrap();
    path
}

/// What a scripted session did and left.
struct Run {
    log: Vec<Op>,
    parting: Vec<String>,
    save_path: PathBuf,
    /// Cues the loop never asked for.
    unanswered: usize,
    /// Waits the loop asked for after its script had ended.
    past_the_script: usize,
    signalled_at: Option<usize>,
}

struct Setup {
    startup: Startup,
    size: (u16, u16),
    /// The screen is gone before the session draws anything at all.
    screen_gone: bool,
    save_path: PathBuf,
}

fn play_out(setup: Setup, cues: Vec<Cue>) -> Run {
    let world = Rc::new(RefCell::new(World {
        script: cues.into(),
        screen_gone: setup.screen_gone,
        ..World::default()
    }));
    let backend = Screen { inner: TestBackend::new(setup.size.0, setup.size.1), world: world.clone() };
    let mut terminal = Terminal::new(backend).expect("a terminal over a TestBackend");
    let mut input = Scripted(world.clone());
    let flag = world.clone();
    let parting =
        session(setup.startup, setup.save_path.clone(), &mut terminal, &mut input, &move || flag.borrow().signalled);
    // Read the world before the terminal is let go: the drawing library
    // writes once more as its handle drops (it restores the cursor), and that
    // write is this harness's doing, not the session's.
    let w = world.borrow();
    Run {
        log: w.log.clone(),
        parting,
        save_path: setup.save_path,
        unanswered: w.script.len(),
        past_the_script: w.waits_past_the_script,
        signalled_at: w.signalled_at,
    }
}

impl Run {
    fn waits(&self) -> Vec<Duration> {
        self.log.iter().filter_map(|op| if let Op::Wait(t) = op { Some(*t) } else { None }).collect()
    }

    fn refused_writes(&self) -> usize {
        self.log.iter().filter(|op| matches!(op, Op::Write { refused: true })).count()
    }

    /// The loop's frames as the log shows them: each run of waits and reads
    /// between two stretches of writing is one frame's input. Returns, per
    /// frame, its waits and how many events it read.
    fn frames_of_input(&self) -> Vec<(Vec<Duration>, usize)> {
        let mut frames: Vec<(Vec<Duration>, usize)> = Vec::new();
        let mut open = false;
        for op in &self.log {
            match op {
                Op::Wait(t) => {
                    if !open {
                        frames.push((Vec::new(), 0));
                        open = true;
                    }
                    frames.last_mut().unwrap().0.push(*t);
                }
                Op::Read => frames.last_mut().expect("a read follows a wait").1 += 1,
                Op::Write { .. } => open = false,
            }
        }
        frames
    }

    /// The parting lines that name a failed save.
    fn save_failures(&self) -> Vec<&String> {
        self.parting.iter().filter(|l| l.to_lowercase().contains("save")).collect()
    }

    /// What the save the session left carries.
    fn saved(&self, what: &str) -> SetDown {
        let contents = std::fs::read_to_string(&self.save_path)
            .unwrap_or_else(|e| panic!("{what}: the session ended without leaving a save ({e})"));
        begin_load(&contents, 0).expect("the session's save loads").set_down()
    }

    /// The loop stopped where it was told to: it used the whole script and
    /// asked for nothing beyond it.
    fn expect_it_stopped_there(&self, what: &str) {
        assert_eq!(self.unanswered, 0, "{what}: fixture — the loop ended before the script's last cue");
        assert_eq!(
            self.past_the_script, 0,
            "{what}: the loop went on to wait for input again after it had its reason to stop"
        );
    }

    /// The input said the terminal was lost, and that was the last thing the
    /// loop did with its terminal: no wait, no read, no frame after it.
    fn expect_nothing_after_the_failed(&self, failed: &Op, what: &str) {
        self.expect_it_stopped_there(what);
        let last = self.log.last().expect("the loop did something");
        assert!(
            std::mem::discriminant(last) == std::mem::discriminant(failed),
            "{what}: after its input failed the loop went on to {last:?} — it must neither wait, \
             nor read, nor draw again over a terminal that is gone"
        );
        assert_eq!(self.refused_writes(), 0, "{what}: fixture — the screen was never taken away");
    }

    /// A frame was refused, once, and that was the last thing the loop did
    /// with its terminal: not drawn again, not skipped and carried on from.
    fn expect_one_refused_frame_and_nothing_after(&self, what: &str) {
        self.expect_it_stopped_there(what);
        assert_eq!(
            self.refused_writes(),
            1,
            "{what}: a refused frame is answered once, by shutting down — the loop wrote to the \
             vanished screen {} times",
            self.refused_writes()
        );
        assert_eq!(
            self.log.last(),
            Some(&Op::Write { refused: true }),
            "{what}: after its frame was refused the loop went on — it skipped the frame and kept \
             running"
        );
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const PLAY_SEED: u64 = 46_047;
const WAKE_SEED: u64 = 48_050;
const SAVED_AT: u64 = 1_000_000;
const WIDE: (u16, u16) = (100, 30);
/// Well under the floor: the too-small card is all that draws.
const TOO_SMALL: (u16, u16) = (20, 5);

fn key(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn quit_key() -> Event {
    key('q')
}

/// A key whose command changes something the save carries.
fn speed_key() -> Event {
    key('>')
}

/// An event neither play nor waking does anything with.
fn inert() -> Event {
    Event::FocusGained
}

fn play(save_path: PathBuf) -> Setup {
    Setup { startup: Startup::NewGame(GameState::new(PLAY_SEED)), size: WIDE, screen_gone: false, save_path }
}

/// A new game as the session opens it.
fn opened_app(save_path: PathBuf) -> App {
    let state = GameState::new(PLAY_SEED);
    let mut view = ViewState::default();
    nav::opening(&mut view, &state);
    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path,
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    }
}

fn still() -> Reading {
    Reading { monotonic: Duration::ZERO, wall: Duration::ZERO }
}

fn elapsed(secs: f64) -> Reading {
    let d = Duration::from_secs_f64(secs);
    Reading { monotonic: d, wall: d }
}

/// The speed a new game starts at, and the speed the speed key takes it to.
fn speeds() -> (f64, f64) {
    let mut app = opened_app(scratch("speeds"));
    let before = app.state.speed;
    let mut t = Terminal::new(TestBackend::new(WIDE.0, WIDE.1)).unwrap();
    app.step(&mut t, still(), &[speed_key()]).unwrap();
    assert_ne!(app.state.speed, before, "fixture: the speed key changes a new game's speed");
    (before, app.state.speed)
}

/// The frame budget a frame of play waits out.
fn frame_budget() -> Duration {
    opened_app(scratch("budget")).poll_timeout()
}

const BRIEFEST_AT_MOST: Duration = Duration::from_millis(1);

/// A save whose load owes many of waking's bounds: no scripted wake finishes.
fn away_save() -> String {
    save_string(&GameState::new(WAKE_SEED), SAVED_AT)
}

fn wake_now() -> u64 {
    SAVED_AT + 10 * WAKE_TICKS_PER_FRAME
}

fn waking(save_path: PathBuf) -> Setup {
    let catch_up = begin_load(&away_save(), wake_now()).expect("the fixture save loads");
    assert!(
        catch_up.total_ticks() >= 10 * WAKE_TICKS_PER_FRAME,
        "fixture: the wake is many bounds long, so no scripted session finishes it"
    );
    Setup { startup: Startup::Loaded(Box::new(catch_up)), size: WIDE, screen_gone: false, save_path }
}

/// Where a wake of the fixture save stands after replaying a number of
/// waking's bounds: the state, the ticks still to replay, the report so far.
struct Reached {
    state: String,
    ticks: u64,
    remaining: u64,
    report: String,
}

fn reached(bounds: usize) -> &'static Reached {
    static CACHE: [OnceLock<Reached>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    CACHE[bounds - 1].get_or_init(|| {
        let mut catch_up = begin_load(&away_save(), wake_now()).unwrap();
        for _ in 0..bounds {
            catch_up = match catch_up.advance(WAKE_TICKS_PER_FRAME) {
                Advance::Pending(next) => next,
                Advance::Finished(_) => panic!("fixture: the wake finished inside {bounds} bounds"),
            };
        }
        let set_down = catch_up.set_down();
        Reached {
            state: save_string(&set_down.state, 0),
            ticks: set_down.state.ticks,
            remaining: set_down.owed_ticks,
            report: format!("{:?}", set_down.report_so_far),
        }
    })
}

/// More live ticks than any of these sessions can bank in the real time it
/// spends waking, and fewer than one of waking's bounds.
const BANKED_AT_MOST: u64 = 10_000;

impl Run {
    /// The save is the wake set down having replayed exactly `bounds` of
    /// waking's bounds — what was replayed kept, nothing more replayed on the
    /// way out — as a shutdown taken while waking saves it (TUI-RUN-039): the
    /// state so far, owing the rest and whatever was banked, the report so
    /// far beside it.
    fn expect_the_wake_set_down_after(&self, bounds: usize, what: &str) {
        let saved = self.saved(what);
        let want = reached(bounds);
        assert_eq!(
            saved.state.ticks, want.ticks,
            "{what}: the save holds a wake that had replayed a different number of ticks than the \
             {bounds} bound(s) its frames replayed"
        );
        assert!(save_string(&saved.state, 0) == want.state, "{what}: the saved state is not the wake's state so far");
        assert!(
            saved.owed_ticks >= want.remaining && saved.owed_ticks <= want.remaining + BANKED_AT_MOST,
            "{what}: the save owes {} ticks; the wake had {} left to replay",
            saved.owed_ticks,
            want.remaining
        );
        assert!(
            format!("{:?}", saved.report_so_far) == want.report,
            "{what}: the save does not carry the wake's report so far"
        );
    }
}

/// Every cause an input failure can have, as far as the loop is concerned:
/// the library hands over nothing-ready and an interrupted wait as an empty
/// wait, so whatever error does arrive means the terminal is lost.
const CAUSES: [io::ErrorKind; 8] = [
    io::ErrorKind::Other,
    io::ErrorKind::UnexpectedEof,
    io::ErrorKind::BrokenPipe,
    io::ErrorKind::NotFound,
    io::ErrorKind::PermissionDenied,
    io::ErrorKind::Interrupted,
    io::ErrorKind::WouldBlock,
    io::ErrorKind::TimedOut,
];

// ---------------------------------------------------------------------------
// TUI-RUN-059 — never a wait of no time
// ---------------------------------------------------------------------------

/// A frame of play waits out the frame budget once, and every wait after that
/// first is the briefest: longer than none, no longer than a millisecond. So
/// a frame takes everything that is ready — ten events in one frame, and of a
/// hundred its bound and then the rest — rather than one event a frame.
/// @spec TUI-RUN-059, TUI-RUN-014, TUI-RUN-016
#[test]
fn a_frame_of_play_takes_all_the_input_that_is_ready_and_never_waits_no_time() {
    let mut cues = vec![quiet()];
    cues.extend((0..10).map(|_| ready(inert())));
    cues.push(quiet());
    cues.extend((0..100).map(|_| ready(inert())));
    cues.push(quiet());
    cues.push(ready(quit_key()));
    cues.push(quiet());
    let run = play_out(play(scratch("ready")), cues);
    run.expect_it_stopped_there("play");

    for (n, wait) in run.waits().iter().enumerate() {
        assert!(!wait.is_zero(), "wait {n} of the session asked the input source to wait no time at all");
    }
    let frames = run.frames_of_input();
    let budget = frame_budget();
    assert!(budget > BRIEFEST_AT_MOST, "fixture: the frame budget is longer than the briefest wait");
    for (n, (waits, _)) in frames.iter().enumerate() {
        assert_eq!(waits[0], budget, "frame {n}: a frame of play first waits out the frame budget");
        for later in &waits[1..] {
            assert!(
                *later > Duration::ZERO && *later <= BRIEFEST_AT_MOST,
                "frame {n}: a wait after the frame's first asked for {later:?} — it wants only what \
                 is already ready, so it asks for the briefest wait, never none and never long"
            );
        }
    }
    let reads: Vec<usize> = frames.iter().map(|(_, reads)| *reads).collect();
    assert_eq!(
        reads,
        vec![0, 10, 64, 36, 1],
        "events read per frame: a quiet frame, ten ready at once, a hundred ready at once (the \
         frame's bound, then the rest), and the quit key"
    );
}

/// Every wait of a waking frame is the briefest — never none, never the frame
/// budget — and so a waking game takes what is ready and answers its quit key.
/// @spec TUI-RUN-059, TUI-RUN-035
#[test]
fn every_wait_of_a_waking_frame_is_the_briefest_and_the_quit_key_is_heard() {
    let cues = vec![quiet(), ready(inert()), ready(inert()), quiet(), ready(quit_key()), quiet()];
    let run = play_out(waking(scratch("waking-quit")), cues);
    run.expect_it_stopped_there("waking");

    let waits = run.waits();
    assert_eq!(waits.len(), 6, "fixture: one wait per cue");
    for (n, wait) in waits.iter().enumerate() {
        assert!(
            *wait > Duration::ZERO && *wait <= BRIEFEST_AT_MOST,
            "waking wait {n} asked for {wait:?}: longer than none, so the source reads what is \
             ready, and no longer than a millisecond, so the replay runs at the machine's pace"
        );
    }
    let reads: Vec<usize> = run.frames_of_input().iter().map(|(_, reads)| *reads).collect();
    assert_eq!(reads, vec![0, 2, 1], "a waking frame takes all that is ready, the quit key included");
    assert_eq!(run.refused_writes(), 0);
    let saved = run.saved("q while waking");
    assert!(saved.owed_ticks > 0, "the quit key ended the wake, which saved still owing the rest");
}

// ---------------------------------------------------------------------------
// TUI-RUN-046 — the input says the terminal is lost, in play
// ---------------------------------------------------------------------------

/// The wait for input fails, or the read of an event does — for any cause at
/// all — in a frame that had already taken a key that would change the game.
/// The loop asks for nothing more, draws nothing more, and saves; the key it
/// had taken is not applied.
/// @spec TUI-RUN-046
#[test]
fn lost_input_in_play_ends_the_session_there_with_the_frames_events_unapplied() {
    let (starting_speed, _) = speeds();
    for cause in CAUSES {
        for (failure, last) in [(wait_fails(cause), Op::Wait(Duration::ZERO)), (read_fails(cause), Op::Read)] {
            let what = format!("{:?} in play", failure.answer);
            let cues = vec![quiet(), ready(speed_key()), ready(inert()), failure];
            let run = play_out(play(scratch("lost-input")), cues);

            run.expect_nothing_after_the_failed(&last, &what);
            let frames_drawn_after_the_key = run
                .log
                .iter()
                .skip_while(|op| !matches!(op, Op::Read))
                .filter(|op| matches!(op, Op::Write { .. }))
                .count();
            assert_eq!(frames_drawn_after_the_key, 0, "{what}: the frame that lost its input drew");

            let saved = run.saved(&what);
            assert_eq!(
                saved.state.speed, starting_speed,
                "{what}: a key the frame had taken before its input failed was applied — it acted \
                 on a meadow no one can see"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-047 — a frame that cannot be written
// ---------------------------------------------------------------------------

/// A frame of play that cannot be written ends the session: the refused frame
/// is neither drawn again nor skipped, and what it had applied before its
/// draw stands in the save.
/// @spec TUI-RUN-047
#[test]
fn a_refused_frame_in_play_ends_the_session_with_what_that_frame_applied() {
    let (_, sped_up) = speeds();

    let what = "a frame refused in play";
    let cues = vec![quiet(), ready(speed_key()).and_the_screen_goes(), quiet()];
    let run = play_out(play(scratch("refused")), cues);
    run.expect_one_refused_frame_and_nothing_after(what);
    assert_eq!(
        run.saved(what).state.speed,
        sped_up,
        "{what}: the key that frame applied before its draw does not stand in the save"
    );

    // The very first frame of a new game, with nothing ever drawn.
    let what = "a new game's first frame refused";
    let run = run_with_the_screen_gone(play(scratch("refused-first")), vec![quiet()]);
    run.expect_one_refused_frame_and_nothing_after(what);
    run.saved(what);
}

fn run_with_the_screen_gone(mut setup: Setup, cues: Vec<Cue>) -> Run {
    setup.screen_gone = true;
    play_out(setup, cues)
}

/// What a refused frame applied and ticked before its draw is exactly what an
/// accepted frame would have applied and ticked, and it is what the shutdown
/// save then carries — the ticks it ran in the state, the ticks it banked
/// beside it.
/// @spec TUI-RUN-047
#[test]
fn what_a_refused_frame_applied_and_ticked_stands_and_is_what_the_shutdown_saves() {
    let hour = elapsed(3_600.0);

    let mut accepted = opened_app(scratch("accepted"));
    let mut screen = Terminal::new(TestBackend::new(WIDE.0, WIDE.1)).unwrap();
    accepted.step(&mut screen, hour, &[speed_key()]).expect("a frame on a working screen");
    assert!(accepted.state.ticks > 0 && accepted.clock.backlog() > 0, "fixture: the frame ticked and banked");

    let world = Rc::new(RefCell::new(World { screen_gone: true, ..World::default() }));
    let gone = Screen { inner: TestBackend::new(WIDE.0, WIDE.1), world: world.clone() };
    let mut gone = Terminal::new(gone).unwrap();
    let path = scratch("refused-step");
    let mut refused = opened_app(path.clone());
    refused.step(&mut gone, hour, &[speed_key()]).expect_err("a frame on a vanished screen is refused");

    assert!(
        save_string(&refused.state, 0) == save_string(&accepted.state, 0),
        "a refused frame left a different state than an accepted one: what it applied and ticked \
         before its draw must stand"
    );
    assert_eq!(refused.clock, accepted.clock, "and the time it banked stands with it");

    refused.save().expect("the shutdown save");
    let saved = begin_load(&std::fs::read_to_string(&path).unwrap(), 0).unwrap().set_down();
    assert!(save_string(&saved.state, 0) == save_string(&accepted.state, 0), "the save is of the meadow as that frame left it");
    assert_eq!(saved.owed_ticks, accepted.clock.backlog(), "carrying the ticks that frame banked");
}

/// A terminal too small to draw is a working terminal, not a lost one: the
/// session runs through it, in play and while waking, until it is told to
/// stop.
/// @spec TUI-RUN-047, TUI-RUN-017
#[test]
fn a_terminal_too_small_to_draw_is_not_a_lost_terminal() {
    let mut setup = play(scratch("small-play"));
    setup.size = TOO_SMALL;
    let run_play = play_out(setup, vec![quiet(), quiet(), quiet(), quiet(), ready(quit_key()), quiet()]);
    run_play.expect_it_stopped_there("too small, in play");
    assert_eq!(run_play.refused_writes(), 0);
    assert_eq!(run_play.frames_of_input().len(), 5, "every frame ran, down to the one the quit key ended");
    assert!(run_play.parting.is_empty(), "nothing went wrong, so there is nothing to say: {:?}", run_play.parting);

    let mut setup = waking(scratch("small-waking"));
    setup.size = TOO_SMALL;
    let run_waking = play_out(setup, vec![quiet(), ready(quit_key()), quiet()]);
    run_waking.expect_it_stopped_there("too small, waking");
    assert_eq!(run_waking.refused_writes(), 0);
    let saved = run_waking.saved("too small, waking");
    assert!(
        saved.state.ticks >= reached(2).ticks,
        "the replay did not run on behind a terminal too small to draw: {} ticks replayed",
        saved.state.ticks
    );
}

// ---------------------------------------------------------------------------
// TUI-RUN-048 — the terminal lost while waking
// ---------------------------------------------------------------------------

/// The wait for input fails, or the read of an event does, while waking: the
/// loop asks for nothing more and draws nothing more, and the save is the
/// catch-up set down as it stood — the one bound the wake's first frame had
/// replayed kept, nothing more replayed on the way out.
/// @spec TUI-RUN-048
#[test]
fn lost_input_while_waking_sets_the_catch_up_down_where_it_stands() {
    let failures = [
        (wait_fails(io::ErrorKind::Other), Op::Wait(Duration::ZERO)),
        (wait_fails(io::ErrorKind::Interrupted), Op::Wait(Duration::ZERO)),
        (read_fails(io::ErrorKind::Other), Op::Read),
        (read_fails(io::ErrorKind::UnexpectedEof), Op::Read),
    ];
    for (failure, last) in failures {
        let what = format!("{:?} while waking", failure.answer);
        let run = play_out(waking(scratch("waking-lost-input")), vec![failure]);
        run.expect_nothing_after_the_failed(&last, &what);
        run.expect_the_wake_set_down_after(1, &what);
    }
}

/// A waking frame that cannot be written — the wake's first frame, or a later
/// one — ends the session, refused once and not drawn again, and the bound
/// that frame replayed before its draw is kept in the save.
/// @spec TUI-RUN-048, TUI-RUN-047
#[test]
fn a_waking_frame_that_cannot_be_written_keeps_what_it_had_replayed() {
    let what = "the wake's first frame refused";
    let run = run_with_the_screen_gone(waking(scratch("waking-refused-first")), vec![]);
    run.expect_one_refused_frame_and_nothing_after(what);
    assert_eq!(run.waits().len(), 0, "{what}: the loop went on to wait for input");
    run.expect_the_wake_set_down_after(1, what);

    let what = "a later waking frame refused";
    let run = play_out(waking(scratch("waking-refused-later")), vec![quiet().and_the_screen_goes()]);
    run.expect_one_refused_frame_and_nothing_after(what);
    run.expect_the_wake_set_down_after(2, what);
}

// ---------------------------------------------------------------------------
// TUI-RUN-050 — promptly
// ---------------------------------------------------------------------------

/// A signal recorded, or a terminal lost, during a frame's wait for input
/// begins the shutdown by the end of that frame: the loop never starts
/// another wait. And the wait it fell in is short — the frame budget at most
/// in play, the briefest wait while waking.
/// @spec TUI-RUN-050
#[test]
fn shutdown_begins_by_the_end_of_the_frame_whose_wait_it_happened_during() {
    let budget = frame_budget();

    // In play.
    let scenarios: [(&str, Vec<Cue>); 4] = [
        ("a signal in play", vec![quiet(), quiet(), quiet().and_a_signal()]),
        ("a signal in play, as an event arrives", vec![quiet(), ready(inert()).and_a_signal(), quiet()]),
        ("lost input in play", vec![quiet(), quiet(), wait_fails(io::ErrorKind::Other)]),
        ("the screen gone in play", vec![quiet(), quiet(), quiet().and_the_screen_goes()]),
    ];
    for (what, cues) in scenarios {
        let run = play_out(play(scratch("prompt-play")), cues);
        run.expect_it_stopped_there(what);
        for wait in run.waits() {
            assert!(wait <= budget, "{what}: a wait of {wait:?} outlasts the frame budget");
        }
        if let Some(at) = run.signalled_at {
            let frames_begun_after = run.log[at..]
                .windows(2)
                .filter(|pair| matches!(pair, [Op::Write { .. }, Op::Wait(_)]))
                .count();
            assert_eq!(frames_begun_after, 0, "{what}: another frame began its wait after the signal's frame ended");
        }
        run.saved(what);
    }

    // While waking.
    let scenarios: [(&str, Vec<Cue>); 3] = [
        ("a signal while waking", vec![quiet().and_a_signal()]),
        ("lost input while waking", vec![wait_fails(io::ErrorKind::Other)]),
        ("the screen gone while waking", vec![quiet().and_the_screen_goes()]),
    ];
    for (what, cues) in scenarios {
        let run = play_out(waking(scratch("prompt-waking")), cues);
        run.expect_it_stopped_there(what);
        for wait in run.waits() {
            assert!(
                wait > Duration::ZERO && wait <= BRIEFEST_AT_MOST,
                "{what}: a waking wait of {wait:?} is not the briefest wait"
            );
        }
        let saved = run.saved(what);
        assert!(
            saved.state.ticks <= reached(2).ticks,
            "{what}: the wake replayed on past the frame it should have stopped in ({} ticks)",
            saved.state.ticks
        );
        assert!(saved.owed_ticks > 0, "{what}: the wake was set down, not finished");
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-051 — a signal and a lost terminal are one shutdown
// ---------------------------------------------------------------------------

/// The ways a termination signal and the loss of the terminal can meet in one
/// session: in the same wait, or the signal recorded in the very frame whose
/// draw is then refused.
fn both_at_once() -> Vec<(&'static str, Vec<Cue>)> {
    vec![
        ("the wait fails as a signal is recorded", vec![wait_fails(io::ErrorKind::Other).and_a_signal()]),
        ("the read fails as a signal is recorded", vec![read_fails(io::ErrorKind::Other).and_a_signal()]),
        ("a signal is recorded and the frame is refused", vec![quiet().and_a_signal().and_the_screen_goes()]),
    ]
}

/// A termination signal and a lost terminal together are one shutdown, begun
/// by whichever the loop sees first: it stops there, and it saves once. The
/// count of saves is read off a save that cannot be written — each attempt
/// would name its failure in a parting line, and there is exactly one.
/// @spec TUI-RUN-051
#[test]
fn a_signal_and_a_lost_terminal_together_are_one_shutdown() {
    for (how, cues) in both_at_once() {
        // In play, a frame in before the two meet.
        let mut play_cues = vec![quiet()];
        play_cues.extend(cues.clone());

        let what = format!("{how}, in play");
        let saved = play_out(play(scratch("both-play")), play_cues.clone());
        saved.expect_it_stopped_there(&what);
        assert!(saved.refused_writes() <= 1, "{what}: the vanished screen was written to again");
        saved.saved(&what);

        let unsaved = play_out(play(unwritable("both-play-unwritable")), play_cues);
        unsaved.expect_it_stopped_there(&what);
        assert_eq!(unsaved.save_failures().len(), 1, "{what}: one shutdown saves once: {:?}", unsaved.parting);

        let what = format!("{how}, while waking");
        let saved = play_out(waking(scratch("both-waking")), cues.clone());
        saved.expect_it_stopped_there(&what);
        assert!(saved.refused_writes() <= 1, "{what}: the vanished screen was written to again");
        assert!(saved.saved(&what).owed_ticks > 0, "{what}: the wake was set down and saved");

        let unsaved = play_out(waking(unwritable("both-waking-unwritable")), cues);
        unsaved.expect_it_stopped_there(&what);
        assert_eq!(unsaved.save_failures().len(), 1, "{what}: one shutdown saves once: {:?}", unsaved.parting);
    }
}

// ---------------------------------------------------------------------------
// TUI-RUN-057 — a shutdown save that cannot be written
// ---------------------------------------------------------------------------

/// Whatever begins the shutdown — a quit key, a termination signal, lost
/// input, a refused frame — in play or while waking, a save that cannot be
/// written is tried once: the session ends all the same, and names the
/// failure in exactly one parting line.
/// @spec TUI-RUN-057
#[test]
fn a_shutdown_save_that_cannot_be_written_is_tried_once_whatever_began_the_shutdown() {
    let endings: [(&str, Vec<Cue>); 5] = [
        ("a quit key", vec![ready(quit_key()), quiet()]),
        ("a termination signal", vec![quiet().and_a_signal()]),
        ("a failed wait for input", vec![wait_fails(io::ErrorKind::Other)]),
        ("a failed read of an event", vec![read_fails(io::ErrorKind::Other)]),
        ("a refused frame", vec![quiet().and_the_screen_goes()]),
    ];
    for (ending, cues) in endings {
        let mut play_cues = vec![quiet()];
        play_cues.extend(cues.clone());
        for (phase, setup, cues) in [
            ("in play", play(unwritable("unwritable-play")), play_cues),
            ("while waking", waking(unwritable("unwritable-waking")), cues),
        ] {
            let what = format!("{ending}, {phase}, the save unwritable");
            let run = play_out(setup, cues);
            run.expect_it_stopped_there(&what);
            assert_eq!(
                run.save_failures().len(),
                1,
                "{what}: the failed save is named in exactly one parting line — none means it went \
                 unreported, more means it was tried again: {:?}",
                run.parting
            );
            assert!(
                run.save_path.is_dir(),
                "fixture: what stood in the save's place is still there, so the write did fail"
            );
        }
    }
}
