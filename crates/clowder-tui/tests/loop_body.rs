//! The loop body — `runtime::App::step`.
//!
//! These were written after the implementation rather than before it, which is
//! the wrong order and is recorded here rather than hidden: the loop was hand
//! -written to unblock a launchable build. What that costs is that these tests
//! were shaped with the code in view, so they are deliberately written against
//! the spec text rather than against what the code happens to do.
//!
//! Two of the five specs cannot be honestly covered yet. TUI-RUN-015
//! (commands land before the frame's ticks) and TUI-RUN-016 (bounded drain,
//! coalesced resizes) both need input to arrive, and turning a key into a
//! command is `nav::keys::resolve` + `nav::update`, still `todo!()`. Driving
//! them would mean asserting against stubs. They are covered when navigation
//! lands, not before.
//!
//! @spec TUI-RUN-014, TUI-RUN-017, TUI-RUN-026

use clowder_tui::nav::ViewState;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator};

use clowder_core::GameState;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clowder-loop-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

fn app(save_path: PathBuf) -> App {
    App {
        state: GameState::new(4242),
        view: ViewState::default(),
        clock: TickAccumulator::default(),
        save_path,
        last_save: Instant::now(),
        prev_layout: None,
        quitting: false,
        save_failed: false,
    }
}

fn elapsed(secs: f64) -> Reading {
    let d = Duration::from_secs_f64(secs);
    Reading { monotonic: d, wall: d }
}

/// The property most likely to be quietly broken later, and the one the design
/// is most emphatic about: a terminal too small to draw does not stop the
/// clock. Time never depends on what is on screen.
/// @spec TUI-RUN-017
#[test]
fn the_simulation_advances_even_when_the_terminal_is_too_small_to_draw() {
    // 40x10 is well under the 80x24 floor, so the frame draws only the
    // too-small card — and must still tick.
    let mut term = Terminal::new(TestBackend::new(40, 10)).unwrap();
    let mut a = app(scratch("toosmall"));
    let before = a.state.ticks;

    a.step(&mut term, elapsed(5.0), &[]).expect("a step below the floor must not fail");

    assert!(
        a.state.ticks > before,
        "five seconds passed and the sim did not advance: a pane too small to draw \
         must never stop the clock"
    );
}

/// The same interval must buy the same ticks whatever the terminal size —
/// otherwise the game runs at a speed that depends on the window.
/// @spec TUI-RUN-017
#[test]
fn how_far_the_sim_advances_does_not_depend_on_the_terminal_size() {
    let mut tiny = Terminal::new(TestBackend::new(40, 10)).unwrap();
    let mut big = Terminal::new(TestBackend::new(140, 50)).unwrap();

    let mut a = app(scratch("tiny"));
    let mut b = app(scratch("big"));
    a.step(&mut tiny, elapsed(3.0), &[]).unwrap();
    b.step(&mut big, elapsed(3.0), &[]).unwrap();

    assert_eq!(a.state.ticks, b.state.ticks, "the window size is not a speed control");
}

/// A frame draws. Weak on its own, but it pins that render is reached at all —
/// the step returning Ok while never touching the terminal would satisfy every
/// other assertion here.
/// @spec TUI-RUN-014
#[test]
fn a_step_draws_a_frame() {
    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut a = app(scratch("draws"));
    a.step(&mut term, elapsed(0.5), &[]).unwrap();

    let painted = term.backend().buffer().content().iter().any(|c| c.symbol() != " ");
    assert!(painted, "a step left the terminal blank — render was never reached");
}

/// The layout must survive to the next frame, or the breakpoints' dead-band has
/// nothing to compare against and cannot damp anything.
/// @spec TUI-RUN-014
#[test]
fn a_step_keeps_the_layout_it_drew_for_the_next_frame() {
    let mut term = Terminal::new(TestBackend::new(110, 30)).unwrap();
    let mut a = app(scratch("layout"));
    assert!(a.prev_layout.is_none(), "fixture: nothing drawn yet");

    a.step(&mut term, elapsed(0.5), &[]).unwrap();
    assert!(
        a.prev_layout.is_some(),
        "the frame's layout was dropped, so the next frame cannot tell which side \
         of a breakpoint it is already on"
    );
}

/// Autosave is on a wall-clock cadence, so a step taken well inside the
/// interval must not write. The complement — that it *does* write once the
/// interval has passed — is covered by pushing `last_save` into the past.
/// @spec TUI-RUN-026
#[test]
fn autosave_waits_for_its_cadence_and_then_writes() {
    let path = scratch("cadence");
    let mut term = Terminal::new(TestBackend::new(90, 26)).unwrap();
    let mut a = app(path.clone());

    a.step(&mut term, elapsed(0.5), &[]).unwrap();
    assert!(!path.exists(), "a step moments after the last save must not write again");

    // Pretend the cadence has elapsed.
    a.last_save = Instant::now() - Duration::from_secs(60 * 60);
    a.step(&mut term, elapsed(0.5), &[]).unwrap();
    assert!(path.exists(), "once the cadence has passed, the step must save");
}

/// A save that cannot be written ends nothing: the session survives, and the
/// next cadence tries again. Killing a healthy session over a transient disk
/// problem destroys more than the failed write did.
/// @spec TUI-RUN-026, TUI-RUN-028
#[test]
fn a_failed_autosave_does_not_end_the_session() {
    let dir = std::env::temp_dir().join(format!("clowder-loop-{}-ro", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // A directory where the save file belongs: writing there always fails.
    let path = dir.join("save.json");
    std::fs::create_dir_all(&path).unwrap();

    let mut term = Terminal::new(TestBackend::new(90, 26)).unwrap();
    let mut a = app(path);
    a.last_save = Instant::now() - Duration::from_secs(60 * 60);

    let before = a.state.ticks;
    a.step(&mut term, elapsed(2.0), &[]).expect("a failed save must not fail the step");
    assert!(a.state.ticks > before, "the sim kept running through a failed save");
    assert!(a.save_failed, "the failure was noticed, so it can be said once and not again");
}
