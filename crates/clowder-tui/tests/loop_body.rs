//! The loop body — `runtime::App::step`.
//!
//! These were written after the implementation rather than before it, which is
//! the wrong order and is recorded here rather than hidden: the loop was hand
//! -written to unblock a launchable build. What that costs is that these tests
//! were shaped with the code in view, so they are deliberately written against
//! the spec text rather than against what the code happens to do.
//!
//! `step` takes its events and its clock reading as arguments rather than
//! reaching for a terminal, which is what makes the frame's ordering and its
//! input bound observable at all: both are properties of one method that a
//! signature reaching for real input could only ever assert against itself.
//!
//! One clause has no seam even so — that a burst of resizes collapses to the
//! last. `Terminal::resize` never reaches the backend, and the next `draw`
//! re-derives the size from it, so the coalescing leaves nothing outside
//! `step` to observe. It is stated here rather than asserted falsely.
//!
//! @spec TUI-RUN-014, TUI-RUN-015, TUI-RUN-016, TUI-RUN-017, TUI-RUN-026

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

// ---------------------------------------------------------------------------
// The two properties that needed an input seam
//
// These were written last. Both specs sat marked implemented while the two
// files that name them said in prose that they could not be covered — one
// because `step` exposed no way to inject input, the other because turning a
// key into a command was still `todo!()`. Both blockers are gone: `step` takes
// its events and its clock reading as arguments, and navigation resolves. A
// spec whose only record of coverage is a note explaining the absence of
// coverage is not covered, whatever the suite's colour.
// ---------------------------------------------------------------------------

/// A meadow where the first row of the first section is something the player
/// can act on — the smallest state in which a keypress becomes a command.
fn actionable() -> GameState {
    let mut s = GameState::new(4242);
    for r in clowder_core::ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    s.revealed = clowder_core::StructureId::ALL
        .iter()
        .map(|b| clowder_core::ContentId::Structure(*b))
        .collect();
    s
}

fn key(c: crossterm::event::KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(crossterm::event::KeyEvent::from(c))
}

/// The frame's commands land before the frame's ticks, and the journal says so:
/// a command applied first is stamped with the tick the frame opened at, one
/// applied after the catch-up would carry the tick it closed at.
///
/// The gap is what makes this observable — a frame worth twenty ticks stamps
/// 0 one way and 20 the other. A frame worth none could not tell them apart.
/// The row has to be one whose command actually emits an event: a command that
/// journals nothing leaves the ticks' own entries as the only ones in the
/// window, and those say nothing about when the keypress landed.
/// @spec TUI-RUN-015
#[test]
fn a_frames_commands_are_applied_before_that_frames_ticks() {
    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut a = app(scratch("cmdorder"));
    a.state = actionable();
    let section = clowder_tui::nav::rows::revealed_sections(&a.state)[0];
    a.view.section = Some(section);

    // The first row that builds something: building emits, and emitting is
    // what puts a stamped entry in the journal.
    let target = clowder_tui::nav::rows::rows(&a.state, section)
        .into_iter()
        .find(|r| matches!(r.key, clowder_tui::nav::RowKey::Content(
            clowder_core::ContentId::Structure(_), _)))
        .expect("a meadow with every structure revealed offers at least one to build");
    a.view.focus[section.index()] = Some(target.key);

    let opened_at = a.state.ticks;
    a.step(&mut term, elapsed(5.0), &[key(crossterm::event::KeyCode::Enter)])
        .expect("a step carrying one keypress must not fail");

    assert!(
        a.state.ticks > opened_at,
        "the frame ran no ticks, so it cannot say whether the command preceded them"
    );
    // Both queues: an entry is filed by whether it is a milestone, which has
    // nothing to do with when it happened.
    let earliest = a
        .state
        .journal
        .recent
        .iter()
        .chain(a.state.journal.kept.iter())
        .map(|e| e.tick)
        .min()
        .expect("the keypress produced no journal entry — the command never applied");
    assert_eq!(
        earliest, opened_at,
        "the frame's first journalled event is stamped {} but the frame opened at {opened_at}: \
         the keypress waited out the whole elapsed interval instead of taking effect when it \
         was pressed",
        earliest
    );
}

/// A flood of input is cut off at the per-frame bound. Parity is the tell: an
/// odd number of toggles past the bound lands back where it started only if
/// the drain stopped counting.
///
/// The other half of this spec — that a burst of resizes collapses to the last
/// — has no seam to test through. `Terminal::resize` never reaches the backend,
/// and the next `draw` re-derives the size from the backend anyway, so nothing
/// the coalescing does survives to be observed from outside `step`.
/// @spec TUI-RUN-016
#[test]
fn a_flood_of_input_is_bounded_within_one_frame() {
    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut a = app(scratch("flood"));
    let initial = a.view.inspector_wanted;

    // One past the bound, so an unbounded drain ends on the opposite state.
    let flood: Vec<_> = (0..65).map(|_| key(crossterm::event::KeyCode::Char(' '))).collect();
    a.step(&mut term, elapsed(0.5), &flood).expect("a flooded step must not fail");

    assert_eq!(
        a.view.inspector_wanted, initial,
        "all 65 toggles were consumed in one frame — an unbounded drain lets input starve \
         the simulation and the renderer"
    );
}
