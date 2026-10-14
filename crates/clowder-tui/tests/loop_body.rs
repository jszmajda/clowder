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
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("clowder-loop-{}-{name}-{n}", std::process::id()));
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
    // A pane-width terminal draws the persistent pane, where Space is a no-op
    // note rather than a toggle (TUI-NAV-088) — a poor tell for a drain bound.
    // 80x24 draws the narrow layout instead, where Space always flips the
    // call, but only once `prev_layout` says so: a first step sees `None` and
    // toggles nothing regardless of width, so an empty step establishes the
    // narrow layout on screen before the flood is the one that must bound it.
    let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut a = app(scratch("flood"));
    a.step(&mut term, still(), &[]).expect("the priming step must not fail");
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

/// A field never expires. Opened and then walked away from, it is still open —
/// with exactly what was typed in it — after the simulation has run on beneath
/// it for hours of game time and the wall clock has passed every timer the
/// shell keeps (an armed row's ~8s, a footer note's few seconds). Walking away
/// mid-type blocks nothing and loses nothing.
///
/// @spec TUI-NAV-060
#[test]
fn an_open_field_outlasts_every_timer_while_the_sim_runs_beneath_it() {
    use clowder_tui::nav::{CountPrompt, Overlay, Section};

    let fields: [(&str, ViewState); 2] = [
        (
            "a count prompt",
            ViewState {
                section: Some(Section::Meadow),
                prompt: Some(CountPrompt { digits: "12".to_string(), ..Default::default() }),
                ..ViewState::default()
            },
        ),
        (
            "the palette",
            ViewState {
                section: Some(Section::Meadow),
                overlay: Some(Overlay::Palette { query: "ab c".to_string(), selected: 0 }),
                ..ViewState::default()
            },
        ),
    ];

    let mut apps: Vec<(&str, App, Terminal<TestBackend>)> = fields
        .into_iter()
        .enumerate()
        .map(|(i, (what, view))| {
            let mut a = app(scratch(&format!("field-timer-{i}")));
            a.view = view;
            (what, a, Terminal::new(TestBackend::new(100, 30)).unwrap())
        })
        .collect();
    let before: Vec<u64> = apps.iter().map(|(_, a, _)| a.state.ticks).collect();

    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(9_000) {
        for (_, a, term) in apps.iter_mut() {
            a.step(term, elapsed(600.0), &[]).expect("a step with a field open must not fail");
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    for ((what, a, _), ticks_before) in apps.iter().zip(before) {
        assert!(
            a.state.ticks > ticks_before,
            "fixture: the simulation did not advance beneath {what}"
        );
        assert!(!a.quitting, "{what} was left open and the session ended");
    }
    assert_eq!(
        apps[0].1.view.prompt.as_ref().map(|p| p.digits.as_str()),
        Some("12"),
        "a count prompt left open closed or lost its digits on its own; a field never expires"
    );
    assert!(
        matches!(&apps[1].1.view.overlay, Some(Overlay::Palette { query, .. }) if query == "ab c"),
        "the palette left open closed or lost its query on its own ({:?}); a field never expires",
        apps[1].1.view.overlay
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-015 / TUI-NAV-076 — an arming across frames
//
// What ends an arming, and what leaves it running, includes things only the
// loop sees happen: the simulation advancing, the terminal changing size, and
// a row leaving its list because the state beneath it moved. These are
// asserted through `step`, the one place all three happen.
// ---------------------------------------------------------------------------

/// A meadow where the nap row is listed and a nap is affordable — the one
/// heavy row whose listing a test can switch off and on again.
fn nappable() -> GameState {
    let mut s = actionable();
    s.ideas.insert(clowder_core::IdeaId::DeepNapTheory);
    s.resources[clowder_core::ResourceId::Starshards.ix()] = 1.0e12;
    s
}

fn nap_key(state: &GameState) -> clowder_tui::nav::RowKey {
    use clowder_tui::nav::rows::{rows, RowKind};
    use clowder_tui::nav::Section;
    rows(state, Section::Dreams)
        .into_iter()
        .find(|r| {
            matches!(r.kind, RowKind::Act { command: clowder_core::Command::DeepNap(_), .. })
        })
        .map(|r| r.key)
        .expect("fixture: the nap row is listed")
}

fn still() -> Reading {
    Reading { monotonic: Duration::ZERO, wall: Duration::ZERO }
}

/// An app focused on the nap row, armed by a real Enter through the loop.
fn armed_on_the_nap(name: &str) -> (App, Terminal<TestBackend>) {
    use clowder_tui::nav::Section;
    let mut a = app(scratch(name));
    a.state = nappable();
    a.view.section = Some(Section::Dreams);
    a.view.focus[Section::Dreams.index()] = Some(nap_key(&a.state));
    let mut term = Terminal::new(TestBackend::new(120, 40)).unwrap();
    a.step(&mut term, still(), &[key(crossterm::event::KeyCode::Enter)]).unwrap();
    assert!(a.view.armed.is_some(), "fixture: Enter on the nap row armed it");
    (a, term)
}

/// The simulation keeps advancing while a row is armed, and advancing it
/// leaves the arming as it was — same row, same command, same expiry.
///
/// @spec TUI-NAV-015, TUI-NAV-076
#[test]
fn the_simulation_advancing_leaves_an_arming_running() {
    let (mut a, mut term) = armed_on_the_nap("armed-sim");
    let before = a.view.armed.clone();
    a.state.speed = 8.0;
    let ticks = a.state.ticks;

    a.step(&mut term, elapsed(5.0), &[]).unwrap();

    assert!(a.state.ticks > ticks, "the simulation stood still while a row was armed");
    assert_eq!(
        a.view.armed, before,
        "the simulation advancing changed or ended the arming; only the endings may"
    );
}

/// A change of layout leaves an arming running: however much of the summary
/// a terminal has room for, the row stays armed.
///
/// @spec TUI-NAV-076
#[test]
fn a_change_of_layout_leaves_an_arming_running() {
    use crossterm::event::Event;
    let (mut a, mut term) = armed_on_the_nap("armed-resize");
    let before = a.view.armed.clone();

    for (w, h) in [(80u16, 24u16), (99, 24), (150, 50), (100, 30)] {
        a.step(&mut term, still(), &[Event::Resize(w, h)]).unwrap();
        assert_eq!(a.view.armed, before, "a resize to {w}×{h} changed or ended the arming");
    }
}

/// An armed row that stops being listed ends its arming — navigation by other
/// means — and the arming does not come back with the row: the next Enter on
/// the row once it is listed again is a first press, which arms afresh, and
/// never a commit.
///
/// @spec TUI-NAV-076
#[test]
fn an_armed_row_that_stops_being_listed_ends_its_arming_for_good() {
    use crossterm::event::KeyCode;
    let (mut a, mut term) = armed_on_the_nap("armed-unlisted");
    let before = a.view.armed.clone().expect("fixture: armed");
    let shards = a.state.stock(clowder_core::ResourceId::Starshards);

    // The row leaves its list for a frame...
    a.state.ideas.remove(&clowder_core::IdeaId::DeepNapTheory);
    a.step(&mut term, still(), &[]).unwrap();
    // ...and comes back, still well inside the old arming's eight seconds.
    a.state.ideas.insert(clowder_core::IdeaId::DeepNapTheory);
    a.view.focus[clowder_tui::nav::Section::Dreams.index()] = Some(before.key.clone());
    std::thread::sleep(Duration::from_millis(300));
    assert!(Instant::now() < before.expires_at, "fixture: the old arming has not expired");

    a.step(&mut term, still(), &[key(KeyCode::Enter)]).unwrap();

    assert_eq!(
        a.state.stock(clowder_core::ResourceId::Starshards),
        shards,
        "the first Enter after the armed row left its list and came back committed the nap; \
         the row leaving ended that arming"
    );
    let now = a.view.armed.as_ref().expect("that Enter did not arm the row afresh");
    assert!(
        now.expires_at > before.expires_at,
        "the row came back still carrying the arming it had before it left its list"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-094 — the row-bound key, through the loop
//
// Whether a press leaves the narrow-mode inspector swap standing is a fact of
// the drawn layout, which only the loop holds; so is the core's answer to the
// command the press issues. Both are asserted through `step`.
// ---------------------------------------------------------------------------

fn roster_kitten(name: &str) -> clowder_core::Kitten {
    clowder_core::Kitten {
        name: name.to_string(),
        job: None,
        location: clowder_core::Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: clowder_core::KittenTrait::None,
        tinkering: None,
    }
}

/// At the narrow layout with the inspector swapped in for the list, `n` on a
/// kitten's row — every kitten's in turn — reaches the core as AppointElder
/// for that kitten, which the core accepts, and the swap, the cursor, the
/// focused section, and the report banner all stand as they did; the footer
/// carries no note (TUI-NAV-107), a refusal's least of all.
///
/// @spec TUI-NAV-094
#[test]
fn n_through_the_loop_appoints_each_kitten_and_leaves_the_narrow_swap_standing() {
    use clowder_tui::nav::rows::rows;
    use clowder_tui::nav::{PendingReport, RowKey, Section};
    use clowder_tui::render::layout::Body;
    use crossterm::event::KeyCode;

    let mut state = GameState::new(4242);
    state.kittens = ["Biscuit", "Clover", "Dumpling"].iter().map(|n| roster_kitten(n)).collect();
    state.roster_high_water = state.kittens.len();
    state.elder = None;
    let kittens: Vec<(usize, RowKey)> = rows(&state, Section::Clowder)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Kitten(ix, _) => Some((ix, r.key)),
            _ => None,
        })
        .collect();
    assert!(!kittens.is_empty(), "fixture: kitten rows are listed");

    for (ix, row_key) in kittens {
        let mut term = Terminal::new(TestBackend::new(80, 30)).unwrap();
        let mut a = app(scratch(&format!("row-bound-n-{ix}")));
        a.state = state.clone();
        a.view.section = Some(Section::Clowder);
        a.view.focus[Section::Clowder.index()] = Some(row_key.clone());
        a.view.inspector_wanted = true;
        a.view.reports = vec![PendingReport { entry: 0, dismissed: false }];
        a.step(&mut term, elapsed(0.0), &[]).expect("a first frame draws");
        assert!(
            matches!(a.prev_layout.map(|l| l.body), Some(Body::Narrow { .. })),
            "fixture: 80 columns lays out the narrow body, where the inspector call is a swap"
        );
        let before = a.view.clone();

        a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");

        let whr = format!("with the kitten row {row_key:?} focused in the narrow swap");
        assert_eq!(
            a.state.elder,
            Some(ix),
            "{whr}, `n` did not appoint kitten {ix} Elder; it issues AppointElder for the kitten \
             the row is about"
        );
        // An accepted appointment leaves no footer note (TUI-NAV-107) —
        // swept on its own below; here only that no note, a refusal's least
        // of all, stands after the press.
        assert_eq!(a.view.footer, None, "{whr}, the accepted appointment left a footer note");
        assert_eq!(a.view.section, before.section, "{whr}, `n` changed the focused section");
        assert_eq!(a.view.focus, before.focus, "{whr}, `n` moved the cursor");
        assert_eq!(a.view.reports, before.reports, "{whr}, `n` dismissed the report banner");
        assert!(a.view.inspector_wanted, "{whr}, `n` ended the narrow-mode inspector swap");
        assert!(
            matches!(a.prev_layout.map(|l| l.body), Some(Body::Narrow { .. })),
            "{whr}, `n` changed the layout"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-107 — how an appointment is answered
//
// Whether the core accepted the command a press issued, and whether it emitted
// an event, is known only where the command is applied, which is the loop; so
// is what the frame draws on its footer line and its ticker. All of it is
// asserted through `step`.
//
// An accepted press leaves no footer note: where the appointment changes the
// Elder, the ticker's line for its event is the one sentence on screen saying
// who the Elder now is; where the kitten named is already the Elder, the core
// emits nothing and nothing on screen changes.
//
// One clause has no reachable case: a press of `n` the core refuses. The key
// is live only on a listed kitten's row, a row is listed only for a kitten in
// the roster, and the core accepts AppointElder for every kitten in the
// roster — the press and its command meet the same state within one step —
// so no meadow makes the core refuse one. What a refusal leaves is the
// refusal the same row does reach, its `Enter`, which the tests below use as
// the note that stands on the footer line before a press.
// ---------------------------------------------------------------------------

/// The text of one row of the terminal, as the last frame drew it.
fn screen_row(term: &Terminal<TestBackend>, rect: ratatui::layout::Rect) -> String {
    let buf = term.backend().buffer();
    (rect.x..rect.right()).map(|x| buf.cell((x, rect.y)).map(|c| c.symbol()).unwrap_or(" ")).collect()
}

/// The whole screen, as the last frame drew it, one string per row.
fn screen_rows(term: &Terminal<TestBackend>) -> Vec<String> {
    let buf = term.backend().buffer();
    let area = buf.area;
    (area.y..area.bottom())
        .map(|y| (area.x..area.right()).map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" ")).collect::<String>())
        .collect()
}

/// How many journal entries record an appointment of a kitten of this name,
/// in the permanent entries and in the rolling window.
fn appointments_journalled(state: &GameState, name: &str) -> (usize, usize) {
    use clowder_core::{Event, JournalItem};
    let is_it = |e: &&clowder_core::JournalEntry| {
        matches!(&e.item, JournalItem::Happened(Event::ElderAppointed { name: n }) if n == name)
    };
    (state.journal.kept.iter().filter(is_it).count(), state.journal.recent.iter().filter(is_it).count())
}

/// A long name the roster's row must shorten in a narrow list, among short
/// ones; no name here is part of another.
const ROSTER_NAMES: [&str; 4] = ["Biscuit", "Clover", "Marmalade-of-the-Long-Whiskers", "Dumpling"];

fn small_roster() -> GameState {
    let mut state = GameState::new(4242);
    state.kittens = ROSTER_NAMES.iter().map(|n| roster_kitten(n)).collect();
    state.roster_high_water = state.kittens.len();
    state.elder = None;
    state
}

fn kitten_row_keys(state: &GameState) -> Vec<(usize, clowder_tui::nav::RowKey)> {
    use clowder_tui::nav::rows::rows;
    use clowder_tui::nav::{RowKey, Section};
    rows(state, Section::Clowder)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Kitten(ix, _) => Some((ix, r.key)),
            _ => None,
        })
        .collect()
}

/// The three layouts a kitten's row is pressed in: the narrow list, the
/// narrow-mode inspector swap, and the three-pane layout.
const LAYOUTS: [(&str, u16, u16, bool); 3] = [
    ("the narrow list", 80, 30, false),
    ("the narrow-mode inspector swap", 80, 30, true),
    ("the three-pane layout", 130, 40, false),
];

/// A session on the Clowder screen with the cursor on `row_key`, one frame
/// drawn.
fn session_on(
    state: &GameState,
    row_key: &clowder_tui::nav::RowKey,
    (w, h, swap): (u16, u16, bool),
    tag: &str,
) -> (App, Terminal<TestBackend>) {
    use clowder_tui::nav::Section;
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut a = app(scratch(tag));
    a.state = state.clone();
    a.view.section = Some(Section::Clowder);
    a.view.focus[Section::Clowder.index()] = Some(row_key.clone());
    a.view.inspector_wanted = swap;
    a.step(&mut term, elapsed(0.0), &[]).expect("a first frame draws");
    (a, term)
}

/// The footer line a frame draws for this state and view with no note on it:
/// a control session drawn fresh from the same state and view.
fn footer_line_without_a_note(a: &App, (w, h): (u16, u16)) -> String {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut control = app(scratch("appoint-control"));
    control.state = a.state.clone();
    control.view = a.view.clone();
    control.view.footer = None;
    control.step(&mut term, elapsed(0.0), &[]).expect("a control frame draws");
    screen_row(&term, control.prev_layout.expect("a control frame was drawn").hints)
}

/// A press of `n` that appoints — on every kitten's row, with nobody or
/// another kitten the Elder before it, in the narrow list, the narrow-mode
/// swap, and the three-pane layout — leaves no footer note: with the footer
/// line empty before the press it carries the hint bar after it, exactly as a
/// frame drawn with no note does, and with a refusal's note standing the note
/// stands as it was. The appointment is announced once, by the ticker's line
/// for its event: no other line on screen repeats that sentence, the footer
/// line beneath it included. The Journal keeps the event's entry in its
/// rolling window.
///
/// @spec TUI-NAV-107
#[test]
fn an_n_that_appoints_is_announced_once_by_the_ticker_and_leaves_no_footer_note() {
    use crossterm::event::KeyCode;

    let base = small_roster();
    let kittens = kitten_row_keys(&base);
    assert_eq!(kittens.len(), ROSTER_NAMES.len(), "fixture: every kitten's row is listed");

    let mut checked = 0u32;
    for (layout_name, w, h, swap) in LAYOUTS {
        for (ix, row_key) in &kittens {
            let ix = *ix;
            let name = base.kittens[ix].name.clone();
            let other = (ix + 1) % base.kittens.len();
            for (standing, elder_before) in [("nobody the Elder", None), ("another kitten the Elder", Some(other))] {
                for refusal_standing in [false, true] {
                    let whr = format!(
                        "in {layout_name}, with `n` pressed on kitten {ix}'s row, {standing} before the press{}",
                        if refusal_standing { " and a refusal's note on the footer line" } else { "" }
                    );
                    let mut state = base.clone();
                    state.elder = elder_before;
                    let tag = format!("appoint-once-{w}-{swap}-{ix}-{}-{refusal_standing}", elder_before.map_or(99, |e| e));
                    let (mut a, mut term) = session_on(&state, row_key, (w, h, swap), &tag);

                    if refusal_standing {
                        // This row's `Enter` is refused, and leaves its note.
                        a.step(&mut term, elapsed(0.0), &[key(KeyCode::Enter)]).expect("a step with Enter does not fail");
                        assert!(a.view.footer.is_some(), "fixture: {whr}, the row's Enter was refused with a note");
                        a.view.last_repeat = None;
                    } else {
                        assert_eq!(a.view.footer, None, "fixture: {whr}, the footer line carries no note");
                    }
                    let footer_before = a.view.footer.clone();
                    let kept_before = a.state.journal.kept.clone();

                    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");

                    assert_eq!(a.state.elder, Some(ix), "fixture: {whr}, the core accepted the appointment");
                    assert_eq!(
                        a.view.footer, footer_before,
                        "{whr}, the press changed the footer line's note; an accepted press leaves the \
                         footer line as it stood (TUI-NAV-107)"
                    );
                    let layout = a.prev_layout.expect("a frame was drawn");
                    if !refusal_standing {
                        assert_eq!(
                            screen_row(&term, layout.hints),
                            footer_line_without_a_note(&a, (w, h)),
                            "{whr}, the footer line is not the one a frame with no note draws (TUI-NAV-107)"
                        );
                    }

                    // Announced once: by the ticker's line, and by no other.
                    let ticker_line = screen_row(&term, layout.ticker);
                    let name_head: String = name.chars().take(12).collect();
                    let at = ticker_line.find(&name_head).unwrap_or_else(|| {
                        panic!("{whr}, the ticker does not carry the appointment's line (TUI-NAV-026): {ticker_line:?}")
                    });
                    let sentence = ticker_line[at..].trim_end().to_string();
                    assert!(
                        sentence.chars().count() > name_head.chars().count() + 3,
                        "fixture: {whr}, the ticker's line says more than the name: {sentence:?}"
                    );
                    let rows = screen_rows(&term);
                    for (y, row) in rows.iter().enumerate() {
                        if y as u16 == layout.ticker.y {
                            continue;
                        }
                        assert!(
                            !row.contains(&sentence),
                            "{whr}, screen row {y} repeats the ticker's sentence {sentence:?}; the \
                             appointment is announced once, by the ticker's line (TUI-NAV-107): {row:?}"
                        );
                    }

                    assert_eq!(a.state.journal.kept, kept_before, "{whr}, the appointment was kept permanently");
                    assert_eq!(
                        appointments_journalled(&a.state, &name),
                        (0, 1),
                        "{whr}, the Journal does not keep the appointment's one entry in its rolling window"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, (LAYOUTS.len() * ROSTER_NAMES.len() * 4) as u32, "fixture: every case was checked");
}

/// A press of `n` on the Elder's own row — every kitten's in turn as the
/// Elder, in every layout, once and held, with the footer line empty or
/// carrying a refusal's note — adds nothing to the screen: the frame after the
/// press is the frame before it, the footer line included, and the Elder and
/// the Journal are as they were.
///
/// @spec TUI-NAV-107
#[test]
fn n_on_the_elders_own_row_adds_nothing_to_the_screen() {
    use crossterm::event::KeyCode;

    let base = small_roster();
    let kittens = kitten_row_keys(&base);
    let mut checked = 0u32;
    for (layout_name, w, h, swap) in LAYOUTS {
        for (ix, row_key) in &kittens {
            let ix = *ix;
            for refusal_standing in [false, true] {
                let whr = format!(
                    "in {layout_name}, with `n` pressed on the Elder's own row, kitten {ix}'s{}",
                    if refusal_standing { ", a refusal's note on the footer line" } else { "" }
                );
                let mut state = base.clone();
                state.elder = Some(ix);
                let (mut a, mut term) = session_on(&state, row_key, (w, h, swap), &format!("appoint-own-{w}-{swap}-{ix}-{refusal_standing}"));
                if refusal_standing {
                    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Enter)]).expect("a step with Enter does not fail");
                    assert!(a.view.footer.is_some(), "fixture: {whr}, the row's Enter was refused with a note");
                }
                // The frame before the press, drawn with nothing pressed.
                a.view.last_repeat = None;
                a.step(&mut term, elapsed(0.0), &[]).expect("a frame draws");
                let screen_before = screen_rows(&term);
                let footer_before = a.view.footer.clone();
                let journal_before = a.state.journal.clone();

                for press in 1..=3 {
                    if press > 1 {
                        std::thread::sleep(Duration::from_millis(120));
                    }
                    a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
                    assert_eq!(a.state.elder, Some(ix), "{whr}, press {press} moved the Elder");
                    assert_eq!(a.state.journal, journal_before, "{whr}, press {press} wrote to the Journal");
                    assert_eq!(
                        a.view.footer, footer_before,
                        "{whr}, press {press} changed the footer line's note; a press that changes \
                         nothing adds nothing to the screen (TUI-NAV-107)"
                    );
                    let screen_after = screen_rows(&term);
                    for (y, (before, after)) in screen_before.iter().zip(&screen_after).enumerate() {
                        assert_eq!(
                            after, before,
                            "{whr}, press {press} changed screen row {y}; a press on the Elder's own \
                             row adds nothing to the screen (TUI-NAV-107)"
                        );
                    }
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, (LAYOUTS.len() * ROSTER_NAMES.len() * 2) as u32, "fixture: every case was checked");
}

/// A held `n` appoints at most once, on its first press, and an `n`
/// alternated between two kittens' rows appoints on every press: neither
/// leaves a footer note, and neither grows the Journal's permanent entries.
///
/// @spec TUI-NAV-107
#[test]
fn a_held_or_alternated_n_leaves_no_note_and_never_grows_the_permanent_journal() {
    use clowder_tui::nav::Section;
    use crossterm::event::KeyCode;

    let base = small_roster();
    let kittens = kitten_row_keys(&base);
    let (first, first_key) = kittens[0].clone();
    let (second, second_key) = kittens[1].clone();
    let name_of = |k: usize| base.kittens[k].name.clone();

    let (mut a, mut term) = session_on(&base, &first_key, (100, 30, false), "appoint-held");
    let kept = a.state.journal.kept.clone();

    // Held on one row: each press let through by the repeat gate.
    for press in 1..=5usize {
        std::thread::sleep(Duration::from_millis(120));
        a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
        assert_eq!(a.state.elder, Some(first), "fixture: press {press} left kitten {first} the Elder");
        assert_eq!(a.view.footer, None, "held press {press} of `n` left a footer note (TUI-NAV-107)");
        assert_eq!(
            appointments_journalled(&a.state, &name_of(first)),
            (0, 1),
            "after held press {press}, the Journal does not hold the one appointment, in its rolling window"
        );
    }

    // Alternated between two rows: every press appoints.
    for press in 1..=10usize {
        let (target, row_key) = if press % 2 == 1 { (second, &second_key) } else { (first, &first_key) };
        a.view.focus[Section::Clowder.index()] = Some(row_key.clone());
        std::thread::sleep(Duration::from_millis(120));
        a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
        assert_eq!(a.state.elder, Some(target), "fixture: alternating press {press} appointed kitten {target}");
        assert_eq!(a.view.footer, None, "alternating press {press} of `n` left a footer note (TUI-NAV-107)");
        assert_eq!(a.state.journal.kept, kept, "alternating press {press} grew the Journal's permanent entries");
    }
    let (kept_first, rolling_first) = appointments_journalled(&a.state, &name_of(first));
    let (kept_second, rolling_second) = appointments_journalled(&a.state, &name_of(second));
    assert_eq!((kept_first, kept_second), (0, 0), "an appointment was kept permanently");
    assert_eq!(rolling_first + rolling_second, 11, "the rolling window does not hold one entry per appointment made");
}

/// Every section's surfaces open, everything affordable, a long roster with
/// kittens at work and at rest: a meadow where the core accepts a command
/// from rows of every section.
fn everything_open() -> GameState {
    use clowder_core::{ArchId, ColonyId, ContentId, CraftId, IdeaId, JobId, ResourceId, RiteId, StructureId};

    let mut s = GameState::new(7);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect();
    let size = clowder_core::content::MEADOW_MOVE_MIN_KITTENS + 9;
    s.roster_high_water = size;
    let mut roster: Vec<clowder_core::Kitten> = (0..size)
        .map(|i| {
            let mut k = roster_kitten(&format!("kitten-{i}"));
            k.exp = 1.0e6;
            k
        })
        .collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
        k.job_seq = Some(1);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment].into_iter().collect();
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect();
    s.discovered = ColonyId::ALL.iter().copied().collect();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;
    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;
    s
}

/// No press of `n` the core accepts leaves a footer note, in a meadow with a
/// long roster of kittens at work, at rest and tinkering: every kitten's row
/// is pressed, the Elder's own and every other, each press in a session of
/// its own, and after each the footer line carries no note.
///
/// @spec TUI-NAV-107
#[test]
fn n_on_every_kittens_row_of_a_full_meadow_leaves_no_footer_note() {
    use crossterm::event::KeyCode;

    let base = everything_open();
    let kittens = kitten_row_keys(&base);
    assert!(kittens.len() > 10, "fixture: a long roster is listed");
    let mut own_row = false;
    for (ix, row_key) in &kittens {
        let ix = *ix;
        if base.elder == Some(ix) {
            own_row = true;
        }
        let (mut a, mut term) = session_on(&base, row_key, (100, 30, false), &format!("appoint-full-{ix}"));
        a.step(&mut term, elapsed(0.0), &[key(KeyCode::Char('n'))]).expect("a step with `n` does not fail");
        assert_eq!(a.state.elder, Some(ix), "fixture: the core accepted the appointment of kitten {ix}");
        assert_eq!(
            a.view.footer, None,
            "`n` on kitten {ix}'s row was accepted and left a footer note; an accepted press leaves \
             the footer line as it stood (TUI-NAV-107)"
        );
    }
    assert!(own_row, "fixture: the Elder's own row was pressed");
}
